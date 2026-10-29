//! Monster thinking: sight, sound propagation, movement, attacks, and the mobj state actions.

use crate::info::{self, Action, MOBJINFO};
use crate::m_fixed::{fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::m_random::p_random;
use crate::p_mobj::{
    self, MobjId, MF_AMBUSH, MF_CORPSE, MF_FLOAT, MF_INFLOAT, MF_JUSTATTACKED, MF_JUSTHIT,
    MF_SHADOW, MF_SHOOTABLE, MF_SKULLFLY, MF_SOLID,
};
use crate::p_tick::Think;
use crate::s_sound::{self, *};
use crate::tables::{finecosine, finesine, Angle, ANG180, ANG270, ANG90, ANGLETOFINESHIFT};
use crate::Doom;

pub const DI_EAST: i32 = 0;
pub const DI_NORTHEAST: i32 = 1;
pub const DI_NORTH: i32 = 2;
pub const DI_NORTHWEST: i32 = 3;
pub const DI_WEST: i32 = 4;
pub const DI_SOUTHWEST: i32 = 5;
pub const DI_SOUTH: i32 = 6;
pub const DI_SOUTHEAST: i32 = 7;
pub const DI_NODIR: i32 = 8;

pub const MELEERANGE: Fixed = 64 * FRACUNIT;
pub const MISSILERANGE: Fixed = 32 * 64 * FRACUNIT;
const FLOATSPEED: Fixed = 4 * FRACUNIT;
const MAXRADIUS: Fixed = 32 * FRACUNIT;
const MAPBLOCKSHIFT: i32 = FRACBITS + 7;
const SKULLSPEED: Fixed = 20 * FRACUNIT;
const TRACEANGLE: Angle = 0xc00_0000;
const FATSPREAD: Angle = ANG90 / 8;

const OPPOSITE: [i32; 9] = [
    DI_WEST,
    DI_SOUTHWEST,
    DI_SOUTH,
    DI_SOUTHEAST,
    DI_EAST,
    DI_NORTHEAST,
    DI_NORTH,
    DI_NORTHWEST,
    DI_NODIR,
];
const DIAGS: [i32; 4] = [DI_NORTHWEST, DI_NORTHEAST, DI_SOUTHWEST, DI_SOUTHEAST];
pub const XSPEED: [Fixed; 8] = [FRACUNIT, 47000, 0, -47000, -FRACUNIT, -47000, 0, 47000];
pub const YSPEED: [Fixed; 8] = [0, 47000, FRACUNIT, 47000, 0, -47000, -FRACUNIT, -47000];

use crate::p_doors::{VLD_BLAZEOPEN as VLD_BLAZE_OPEN, VLD_OPEN};
use crate::p_floor::{
    FLOOR_LOWERFLOORTOLOWEST as LOWER_FLOOR_TO_LOWEST, FLOOR_RAISETOTEXTURE as RAISE_TO_TEXTURE,
};

#[derive(Default)]
pub struct State {
    pub soundtarget: Option<MobjId>,
    pub corpsehit: Option<MobjId>,
    pub vileobj: Option<MobjId>,
    pub viletryx: Fixed,
    pub viletryy: Fixed,
    pub braintargets: Vec<MobjId>,
    pub braintargeton: i32,
    pub easy: i32,
}

fn rnd(d: &mut Doom) -> i32 {
    p_random(&mut d.m_random)
}

/// P_SubRandom: the first value minus the second value.
pub fn sub_random(d: &mut Doom) -> i32 {
    let r = rnd(d);
    r - rnd(d)
}

fn mo(d: &Doom, id: MobjId) -> &crate::p_mobj::Mobj {
    d.p_tick.mobj(id)
}

fn mo_mut(d: &mut Doom, id: MobjId) -> &mut crate::p_mobj::Mobj {
    d.p_tick.mobj_mut(id)
}

fn info_of(d: &Doom, id: MobjId) -> &'static info::MobjInfo {
    &MOBJINFO[mo(d, id).type_]
}

fn sector_of(d: &Doom, id: MobjId) -> usize {
    d.p_setup.subsectors[mo(d, id).subsector].sector
}

fn start_sound(d: &mut Doom, origin: Option<MobjId>, sfx: i32) {
    s_sound::start_sound(d, origin, sfx);
}

/// P_RecursiveSound: floods the sound through open two-sided lines.
fn recursive_sound(d: &mut Doom, sec: usize, soundblocks: i32) {
    let validcount = d.r_main.validcount;
    {
        let s = &d.p_setup.sectors[sec];
        if s.validcount == validcount && s.soundtraversed <= soundblocks + 1 {
            return;
        }
    }
    let soundtarget = d.p_enemy.soundtarget;
    {
        let s = &mut d.p_setup.sectors[sec];
        s.validcount = validcount;
        s.soundtraversed = soundblocks + 1;
        s.soundtarget = soundtarget;
    }
    let count = d.p_setup.sectors[sec].lines.len();
    for i in 0..count {
        let check = d.p_setup.sectors[sec].lines[i];
        let line = d.p_setup.lines[check];
        if line.flags & crate::p_setup::ML_TWOSIDED == 0 {
            continue;
        }
        crate::p_maputl::line_opening(d, check);
        if d.p_maputl.openrange <= 0 {
            continue;
        }
        let other = if d.p_setup.sides[line.sidenum[0] as usize].sector == sec {
            d.p_setup.sides[line.sidenum[1] as usize].sector
        } else {
            d.p_setup.sides[line.sidenum[0] as usize].sector
        };
        if line.flags & crate::p_setup::ML_SOUNDBLOCK != 0 {
            if soundblocks == 0 {
                recursive_sound(d, other, 1);
            }
        } else {
            recursive_sound(d, other, soundblocks);
        }
    }
}

/// P_NoiseAlert: wakes up the monsters that can hear the emitter.
pub fn noise_alert(d: &mut Doom, target: MobjId, emitter: MobjId) {
    d.p_enemy.soundtarget = Some(target);
    d.r_main.validcount += 1;
    let sec = sector_of(d, emitter);
    recursive_sound(d, sec, 0);
}

/// P_CheckMeleeRange.
pub fn check_melee_range(d: &mut Doom, actor: MobjId) -> bool {
    let Some(pl) = mo(d, actor).target else {
        return false;
    };
    let (a, p) = (mo(d, actor), mo(d, pl));
    let dist = crate::p_maputl::approx_distance(p.x.wrapping_sub(a.x), p.y.wrapping_sub(a.y));
    if dist >= MELEERANGE - 20 * FRACUNIT + MOBJINFO[p.type_].radius {
        return false;
    }
    crate::p_sight::check_sight(d, actor, pl)
}

/// P_CheckMissileRange.
pub fn check_missile_range(d: &mut Doom, actor: MobjId) -> bool {
    let target = mo(d, actor).target.unwrap();
    if !crate::p_sight::check_sight(d, actor, target) {
        return false;
    }
    if mo(d, actor).flags & MF_JUSTHIT != 0 {
        mo_mut(d, actor).flags &= !MF_JUSTHIT;
        return true;
    }
    if mo(d, actor).reactiontime != 0 {
        return false;
    }
    let (a, t) = (mo(d, actor), mo(d, target));
    let mut dist = crate::p_maputl::approx_distance(a.x.wrapping_sub(t.x), a.y.wrapping_sub(t.y))
        - 64 * FRACUNIT;
    let type_ = a.type_;
    if MOBJINFO[type_].meleestate == 0 {
        dist -= 128 * FRACUNIT;
    }
    dist >>= 16;
    if type_ == info::MT_VILE && dist > 14 * 64 {
        return false;
    }
    if type_ == info::MT_UNDEAD {
        if dist < 196 {
            return false;
        }
        dist >>= 1;
    }
    if type_ == info::MT_CYBORG || type_ == info::MT_SPIDER || type_ == info::MT_SKULL {
        dist >>= 1;
    }
    if dist > 200 {
        dist = 200;
    }
    if type_ == info::MT_CYBORG && dist > 160 {
        dist = 160;
    }
    rnd(d) >= dist
}

/// P_Move: moves the actor one step in its direction.
pub fn move_(d: &mut Doom, actor: MobjId) -> bool {
    let movedir = mo(d, actor).movedir;
    if movedir == DI_NODIR {
        return false;
    }
    if movedir as u32 >= 8 {
        crate::i_system::error(d, "Weird actor->movedir!");
    }
    let a = mo(d, actor);
    let speed = MOBJINFO[a.type_].speed;
    let tryx =
        a.x.wrapping_add(speed.wrapping_mul(XSPEED[movedir as usize]));
    let tryy =
        a.y.wrapping_add(speed.wrapping_mul(YSPEED[movedir as usize]));
    if !crate::p_map::try_move(d, actor, tryx, tryy) {
        if mo(d, actor).flags & MF_FLOAT != 0 && d.p_map.floatok {
            let tmfloorz = d.p_map.tmfloorz;
            let m = mo_mut(d, actor);
            if m.z < tmfloorz {
                m.z += FLOATSPEED;
            } else {
                m.z -= FLOATSPEED;
            }
            m.flags |= MF_INFLOAT;
            return true;
        }
        if d.p_map.numspechit == 0 {
            return false;
        }
        mo_mut(d, actor).movedir = DI_NODIR;
        let mut good = false;
        loop {
            let n = d.p_map.numspechit;
            d.p_map.numspechit -= 1;
            if n == 0 {
                break;
            }
            let ld = d.p_map.spechit[(n - 1) as usize];
            if crate::p_switch::use_special_line(d, actor, ld, 0) {
                good = true;
            }
        }
        return good;
    }
    let m = mo_mut(d, actor);
    m.flags &= !MF_INFLOAT;
    if m.flags & MF_FLOAT == 0 {
        m.z = m.floorz;
    }
    true
}

/// P_TryWalk.
fn try_walk(d: &mut Doom, actor: MobjId) -> bool {
    if !move_(d, actor) {
        return false;
    }
    let r = rnd(d) & 15;
    mo_mut(d, actor).movecount = r;
    true
}

fn try_dir(d: &mut Doom, actor: MobjId, dir: i32) -> bool {
    mo_mut(d, actor).movedir = dir;
    try_walk(d, actor)
}

/// P_NewChaseDir.
pub fn new_chase_dir(d: &mut Doom, actor: MobjId) {
    let Some(target) = mo(d, actor).target else {
        crate::i_system::error(d, "P_NewChaseDir: called with no target");
    };
    let olddir = mo(d, actor).movedir;
    let turnaround = OPPOSITE[olddir as usize];
    let deltax = mo(d, target).x.wrapping_sub(mo(d, actor).x);
    let deltay = mo(d, target).y.wrapping_sub(mo(d, actor).y);
    let mut dd = [0i32; 3];
    dd[1] = if deltax > 10 * FRACUNIT {
        DI_EAST
    } else if deltax < -10 * FRACUNIT {
        DI_WEST
    } else {
        DI_NODIR
    };
    dd[2] = if deltay < -10 * FRACUNIT {
        DI_SOUTH
    } else if deltay > 10 * FRACUNIT {
        DI_NORTH
    } else {
        DI_NODIR
    };
    if dd[1] != DI_NODIR && dd[2] != DI_NODIR {
        let dir = DIAGS[(((deltay < 0) as usize) << 1) + (deltax > 0) as usize];
        mo_mut(d, actor).movedir = dir;
        if dir != turnaround && try_walk(d, actor) {
            return;
        }
    }
    if rnd(d) > 200 || deltay.unsigned_abs() > deltax.unsigned_abs() {
        dd.swap(1, 2);
    }
    if dd[1] == turnaround {
        dd[1] = DI_NODIR;
    }
    if dd[2] == turnaround {
        dd[2] = DI_NODIR;
    }
    if dd[1] != DI_NODIR && try_dir(d, actor, dd[1]) {
        return;
    }
    if dd[2] != DI_NODIR && try_dir(d, actor, dd[2]) {
        return;
    }
    if olddir != DI_NODIR && try_dir(d, actor, olddir) {
        return;
    }
    if rnd(d) & 1 != 0 {
        for tdir in DI_EAST..=DI_SOUTHEAST {
            if tdir != turnaround && try_dir(d, actor, tdir) {
                return;
            }
        }
    } else {
        for tdir in (DI_EAST..=DI_SOUTHEAST).rev() {
            if tdir != turnaround && try_dir(d, actor, tdir) {
                return;
            }
        }
    }
    if turnaround != DI_NODIR && try_dir(d, actor, turnaround) {
        return;
    }
    mo_mut(d, actor).movedir = DI_NODIR;
}

/// P_LookForPlayers: returns true when the actor finds a visible player.
pub fn look_for_players(d: &mut Doom, actor: MobjId, allaround: bool) -> bool {
    let mut c = 0;
    let stop = (mo(d, actor).lastlook - 1) & 3;
    loop {
        let lastlook = mo(d, actor).lastlook;
        let skip = !d.g_game.playeringame[lastlook as usize];
        if !skip {
            let hit_limit = c == 2;
            c += 1;
            if hit_limit || lastlook == stop {
                return false;
            }
            let player = &d.g_game.players[lastlook as usize];
            if player.health > 0 {
                let pmo = player.mo.unwrap();
                if crate::p_sight::check_sight(d, actor, pmo) {
                    let mut ok = true;
                    if !allaround {
                        let (a, p) = (mo(d, actor), mo(d, pmo));
                        let an = crate::r_main::point_to_angle2(a.x, a.y, p.x, p.y)
                            .wrapping_sub(a.angle);
                        if an > ANG90 && an < ANG270 {
                            let dist = crate::p_maputl::approx_distance(
                                p.x.wrapping_sub(a.x),
                                p.y.wrapping_sub(a.y),
                            );
                            if dist > MELEERANGE {
                                ok = false;
                            }
                        }
                    }
                    if ok {
                        mo_mut(d, actor).target = Some(pmo);
                        return true;
                    }
                }
            }
        }
        let m = mo_mut(d, actor);
        m.lastlook = (m.lastlook + 1) & 3;
    }
}

/// A_KeenDie: opens the tagged door when the last Keen dies.
fn a_keen_die(d: &mut Doom, actor: MobjId) {
    a_fall(d, actor);
    let type_ = mo(d, actor).type_;
    for th in d.p_tick.ids() {
        if th == actor || d.p_tick.thinkers[th].func != Think::Mobj {
            continue;
        }
        let m = mo(d, th);
        if m.type_ == type_ && m.health > 0 {
            return;
        }
    }
    junk_line_special(d, 666, |d, l| {
        crate::p_doors::ev_do_door(d, l, VLD_OPEN);
    });
}

/// Runs a line special on a temporary line with only a tag, as the original does with a stack line.
fn junk_line_special(d: &mut Doom, tag: i32, f: impl FnOnce(&mut Doom, usize)) {
    let junk = crate::p_setup::Line {
        tag,
        ..Default::default()
    };
    d.p_setup.lines.push(junk);
    let idx = d.p_setup.lines.len() - 1;
    f(d, idx);
    d.p_setup.lines.truncate(idx);
}

/// A_Look: waits for a player or a sound.
pub fn a_look(d: &mut Doom, actor: MobjId) {
    mo_mut(d, actor).threshold = 0;
    let sec = sector_of(d, actor);
    let targ = d.p_setup.sectors[sec].soundtarget;
    let mut seeyou = false;
    if let Some(t) = targ {
        if mo(d, t).flags & MF_SHOOTABLE != 0 {
            mo_mut(d, actor).target = Some(t);
            if mo(d, actor).flags & MF_AMBUSH != 0 {
                seeyou = crate::p_sight::check_sight(d, actor, t);
            } else {
                seeyou = true;
            }
        }
    }
    if !seeyou && !look_for_players(d, actor, false) {
        return;
    }
    let inf = info_of(d, actor);
    if inf.seesound != 0 {
        let sound = match inf.seesound {
            SFX_POSIT1 | SFX_POSIT2 | SFX_POSIT3 => SFX_POSIT1 + rnd(d) % 3,
            SFX_BGSIT1 | SFX_BGSIT2 => SFX_BGSIT1 + rnd(d) % 2,
            s => s,
        };
        let type_ = mo(d, actor).type_;
        if type_ == info::MT_SPIDER || type_ == info::MT_CYBORG {
            start_sound(d, None, sound);
        } else {
            start_sound(d, Some(actor), sound);
        }
    }
    p_mobj::set_mobj_state(d, actor, inf.seestate as usize);
}

/// A_Chase: the main monster movement and attack decision.
pub fn a_chase(d: &mut Doom, actor: MobjId) {
    {
        let m = mo_mut(d, actor);
        if m.reactiontime != 0 {
            m.reactiontime -= 1;
        }
    }
    if mo(d, actor).threshold != 0 {
        let t = mo(d, actor).target;
        let dead = match t {
            None => true,
            Some(t) => mo(d, t).health <= 0,
        };
        let m = mo_mut(d, actor);
        if dead {
            m.threshold = 0;
        } else {
            m.threshold -= 1;
        }
    }
    {
        let m = mo_mut(d, actor);
        if m.movedir < 8 {
            m.angle &= 7 << 29;
            let delta = m.angle.wrapping_sub((m.movedir as u32) << 29) as i32;
            if delta > 0 {
                m.angle = m.angle.wrapping_sub(ANG90 / 2);
            } else if delta < 0 {
                m.angle = m.angle.wrapping_add(ANG90 / 2);
            }
        }
    }
    let inf = info_of(d, actor);
    let target = mo(d, actor).target;
    if target.is_none_or(|t| mo(d, t).flags & MF_SHOOTABLE == 0) {
        if look_for_players(d, actor, true) {
            return;
        }
        p_mobj::set_mobj_state(d, actor, inf.spawnstate as usize);
        return;
    }
    let target = target.unwrap();
    if mo(d, actor).flags & MF_JUSTATTACKED != 0 {
        mo_mut(d, actor).flags &= !MF_JUSTATTACKED;
        if d.g_game.gameskill != crate::g_game::SK_NIGHTMARE && !d.g_game.fastparm {
            new_chase_dir(d, actor);
        }
        return;
    }
    if inf.meleestate != 0 && check_melee_range(d, actor) {
        if inf.attacksound != 0 {
            start_sound(d, Some(actor), inf.attacksound);
        }
        p_mobj::set_mobj_state(d, actor, inf.meleestate as usize);
        return;
    }
    let mut nomissile = false;
    if inf.missilestate != 0 {
        if d.g_game.gameskill < crate::g_game::SK_NIGHTMARE
            && !d.g_game.fastparm
            && mo(d, actor).movecount != 0
        {
            nomissile = true;
        } else if !check_missile_range(d, actor) {
            nomissile = true;
        } else {
            p_mobj::set_mobj_state(d, actor, inf.missilestate as usize);
            mo_mut(d, actor).flags |= MF_JUSTATTACKED;
            return;
        }
    }
    let _ = nomissile;
    if d.g_game.netgame
        && mo(d, actor).threshold == 0
        && !crate::p_sight::check_sight(d, actor, target)
        && look_for_players(d, actor, true)
    {
        return;
    }
    mo_mut(d, actor).movecount -= 1;
    if mo(d, actor).movecount < 0 || !move_(d, actor) {
        new_chase_dir(d, actor);
    }
    if inf.activesound != 0 && rnd(d) < 3 {
        start_sound(d, Some(actor), inf.activesound);
    }
}

/// A_FaceTarget.
pub fn a_face_target(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    mo_mut(d, actor).flags &= !MF_AMBUSH;
    let (a, tm) = (mo(d, actor), mo(d, t));
    let angle = crate::r_main::point_to_angle2(a.x, a.y, tm.x, tm.y);
    let shadow = tm.flags & MF_SHADOW != 0;
    mo_mut(d, actor).angle = angle;
    if shadow {
        let r = sub_random(d);
        let m = mo_mut(d, actor);
        m.angle = m.angle.wrapping_add((r << 21) as u32);
    }
}

/// A_PosAttack.
fn a_pos_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    a_face_target(d, actor);
    let mut angle = mo(d, actor).angle;
    let slope = crate::p_map::aim_line_attack(d, actor, angle, MISSILERANGE);
    start_sound(d, Some(actor), SFX_PISTOL);
    angle = angle.wrapping_add((sub_random(d) << 20) as u32);
    let damage = ((rnd(d) % 5) + 1) * 3;
    crate::p_map::line_attack(d, actor, angle, MISSILERANGE, slope, damage);
}

/// A_SPosAttack.
fn a_spos_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    start_sound(d, Some(actor), SFX_SHOTGN);
    a_face_target(d, actor);
    let bangle = mo(d, actor).angle;
    let slope = crate::p_map::aim_line_attack(d, actor, bangle, MISSILERANGE);
    for _ in 0..3 {
        let angle = bangle.wrapping_add((sub_random(d) << 20) as u32);
        let damage = ((rnd(d) % 5) + 1) * 3;
        crate::p_map::line_attack(d, actor, angle, MISSILERANGE, slope, damage);
    }
}

/// A_CPosAttack.
fn a_cpos_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    start_sound(d, Some(actor), SFX_SHOTGN);
    a_face_target(d, actor);
    let bangle = mo(d, actor).angle;
    let slope = crate::p_map::aim_line_attack(d, actor, bangle, MISSILERANGE);
    let angle = bangle.wrapping_add((sub_random(d) << 20) as u32);
    let damage = ((rnd(d) % 5) + 1) * 3;
    crate::p_map::line_attack(d, actor, angle, MISSILERANGE, slope, damage);
}

fn target_lost(d: &mut Doom, actor: MobjId) -> bool {
    match mo(d, actor).target {
        None => true,
        Some(t) => mo(d, t).health <= 0 || !crate::p_sight::check_sight(d, actor, t),
    }
}

/// A_CPosRefire.
fn a_cpos_refire(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    if rnd(d) < 40 {
        return;
    }
    if target_lost(d, actor) {
        let s = info_of(d, actor).seestate as usize;
        p_mobj::set_mobj_state(d, actor, s);
    }
}

/// A_SpidRefire.
fn a_spid_refire(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    if rnd(d) < 10 {
        return;
    }
    if target_lost(d, actor) {
        let s = info_of(d, actor).seestate as usize;
        p_mobj::set_mobj_state(d, actor, s);
    }
}

fn missile_at_target(d: &mut Doom, actor: MobjId, type_: usize) -> Option<MobjId> {
    let t = mo(d, actor).target?;
    Some(p_mobj::spawn_missile(d, actor, t, type_))
}

/// A_BspiAttack.
fn a_bspi_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    a_face_target(d, actor);
    missile_at_target(d, actor, info::MT_ARACHPLAZ);
}

/// A_TroopAttack.
fn a_troop_attack(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    if check_melee_range(d, actor) {
        start_sound(d, Some(actor), SFX_CLAW);
        let damage = (rnd(d) % 8 + 1) * 3;
        crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), damage);
        return;
    }
    missile_at_target(d, actor, info::MT_TROOPSHOT);
}

/// A_SargAttack.
fn a_sarg_attack(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    if check_melee_range(d, actor) {
        let damage = ((rnd(d) % 10) + 1) * 4;
        crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), damage);
    }
}

/// A_HeadAttack.
fn a_head_attack(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    if check_melee_range(d, actor) {
        let damage = (rnd(d) % 6 + 1) * 10;
        crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), damage);
        return;
    }
    missile_at_target(d, actor, info::MT_HEADSHOT);
}

/// A_CyberAttack.
fn a_cyber_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    a_face_target(d, actor);
    missile_at_target(d, actor, info::MT_ROCKET);
}

/// A_BruisAttack.
fn a_bruis_attack(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    if check_melee_range(d, actor) {
        start_sound(d, Some(actor), SFX_CLAW);
        let damage = (rnd(d) % 8 + 1) * 10;
        crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), damage);
        return;
    }
    missile_at_target(d, actor, info::MT_BRUISERSHOT);
}

/// A_SkelMissile.
fn a_skel_missile(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    mo_mut(d, actor).z += 16 * FRACUNIT;
    let m = p_mobj::spawn_missile(d, actor, t, info::MT_TRACER);
    mo_mut(d, actor).z -= 16 * FRACUNIT;
    let mm = mo_mut(d, m);
    mm.x = mm.x.wrapping_add(mm.momx);
    mm.y = mm.y.wrapping_add(mm.momy);
    mm.tracer = Some(t);
}

/// A_Tracer: steers a homing missile.
fn a_tracer(d: &mut Doom, actor: MobjId) {
    if d.g_game.gametic & 3 != 0 {
        return;
    }
    let (x, y, z, momx, momy) = {
        let a = mo(d, actor);
        (a.x, a.y, a.z, a.momx, a.momy)
    };
    p_mobj::spawn_puff(d, x, y, z);
    let th = p_mobj::spawn_mobj(
        d,
        x.wrapping_sub(momx),
        y.wrapping_sub(momy),
        z,
        info::MT_SMOKE,
    );
    mo_mut(d, th).momz = FRACUNIT;
    let r = rnd(d) & 3;
    {
        let t = mo_mut(d, th);
        t.tics -= r;
        if t.tics < 1 {
            t.tics = 1;
        }
    }
    let Some(dest) = mo(d, actor).tracer else {
        return;
    };
    if mo(d, dest).health <= 0 {
        return;
    }
    let (dx, dy, dz) = {
        let m = mo(d, dest);
        (m.x, m.y, m.z)
    };
    let exact = crate::r_main::point_to_angle2(x, y, dx, dy);
    let speed = info_of(d, actor).speed;
    let a = mo_mut(d, actor);
    if exact != a.angle {
        if exact.wrapping_sub(a.angle) > 0x8000_0000 {
            a.angle = a.angle.wrapping_sub(TRACEANGLE);
            if exact.wrapping_sub(a.angle) < 0x8000_0000 {
                a.angle = exact;
            }
        } else {
            a.angle = a.angle.wrapping_add(TRACEANGLE);
            if exact.wrapping_sub(a.angle) > 0x8000_0000 {
                a.angle = exact;
            }
        }
    }
    let fine = (a.angle >> ANGLETOFINESHIFT) as usize;
    a.momx = fixed_mul(speed, finecosine(fine));
    a.momy = fixed_mul(speed, finesine(fine));
    let mut dist = crate::p_maputl::approx_distance(dx.wrapping_sub(x), dy.wrapping_sub(y));
    dist /= speed;
    if dist < 1 {
        dist = 1;
    }
    let a = mo_mut(d, actor);
    let slope = (dz + 40 * FRACUNIT - a.z) / dist;
    if slope < a.momz {
        a.momz -= FRACUNIT / 8;
    } else {
        a.momz += FRACUNIT / 8;
    }
}

/// A_SkelWhoosh.
fn a_skel_whoosh(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    a_face_target(d, actor);
    start_sound(d, Some(actor), SFX_SKESWG);
}

/// A_SkelFist.
fn a_skel_fist(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    if check_melee_range(d, actor) {
        let damage = ((rnd(d) % 10) + 1) * 6;
        start_sound(d, Some(actor), SFX_SKEPCH);
        crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), damage);
    }
}

/// PIT_VileCheck: finds a corpse that the vile can raise.
pub fn pit_vile_check(d: &mut Doom, thing: MobjId) -> bool {
    let t = mo(d, thing);
    if t.flags & MF_CORPSE == 0 {
        return true;
    }
    if t.tics != -1 {
        return true;
    }
    let inf = &MOBJINFO[t.type_];
    if inf.raisestate == info::S_NULL as i32 {
        return true;
    }
    let maxdist = inf.radius + MOBJINFO[info::MT_VILE].radius;
    if t.x.wrapping_sub(d.p_enemy.viletryx).wrapping_abs() > maxdist
        || t.y.wrapping_sub(d.p_enemy.viletryy).wrapping_abs() > maxdist
    {
        return true;
    }
    d.p_enemy.corpsehit = Some(thing);
    let (x, y) = {
        let c = mo_mut(d, thing);
        c.momx = 0;
        c.momy = 0;
        c.height <<= 2;
        (c.x, c.y)
    };
    let check = crate::p_map::check_position(d, thing, x, y);
    mo_mut(d, thing).height >>= 2;
    !check
}

/// A_VileChase: raises a nearby corpse or chases.
fn a_vile_chase(d: &mut Doom, actor: MobjId) {
    let movedir = mo(d, actor).movedir;
    if movedir != DI_NODIR {
        let (ax, ay, speed) = {
            let a = mo(d, actor);
            (a.x, a.y, MOBJINFO[a.type_].speed)
        };
        d.p_enemy.viletryx = ax.wrapping_add(speed.wrapping_mul(XSPEED[movedir as usize]));
        d.p_enemy.viletryy = ay.wrapping_add(speed.wrapping_mul(YSPEED[movedir as usize]));
        let (vx, vy) = (d.p_enemy.viletryx, d.p_enemy.viletryy);
        let (ox, oy) = (d.p_setup.bmaporgx, d.p_setup.bmaporgy);
        let xl = (vx - ox - MAXRADIUS * 2) >> MAPBLOCKSHIFT;
        let xh = (vx - ox + MAXRADIUS * 2) >> MAPBLOCKSHIFT;
        let yl = (vy - oy - MAXRADIUS * 2) >> MAPBLOCKSHIFT;
        let yh = (vy - oy + MAXRADIUS * 2) >> MAPBLOCKSHIFT;
        d.p_enemy.vileobj = Some(actor);
        for bx in xl..=xh {
            for by in yl..=yh {
                if !crate::p_maputl::block_things_iterator(d, bx, by, pit_vile_check) {
                    let corpsehit = d.p_enemy.corpsehit.unwrap();
                    let temp = mo(d, actor).target;
                    mo_mut(d, actor).target = Some(corpsehit);
                    a_face_target(d, actor);
                    mo_mut(d, actor).target = temp;
                    p_mobj::set_mobj_state(d, actor, info::S_VILE_HEAL1);
                    start_sound(d, Some(corpsehit), SFX_SLOP);
                    let inf = info_of(d, corpsehit);
                    p_mobj::set_mobj_state(d, corpsehit, inf.raisestate as usize);
                    let c = mo_mut(d, corpsehit);
                    c.height <<= 2;
                    c.flags = inf.flags;
                    c.health = inf.spawnhealth;
                    c.target = None;
                    return;
                }
            }
        }
    }
    a_chase(d, actor);
}

/// A_VileStart.
fn a_vile_start(d: &mut Doom, actor: MobjId) {
    start_sound(d, Some(actor), SFX_VILATK);
}

/// A_StartFire.
fn a_start_fire(d: &mut Doom, actor: MobjId) {
    start_sound(d, Some(actor), SFX_FLAMST);
    a_fire(d, actor);
}

/// A_FireCrackle.
fn a_fire_crackle(d: &mut Doom, actor: MobjId) {
    start_sound(d, Some(actor), SFX_FLAME);
    a_fire(d, actor);
}

/// A_Fire: keeps the flame in front of the victim.
fn a_fire(d: &mut Doom, actor: MobjId) {
    let Some(dest) = mo(d, actor).tracer else {
        return;
    };
    let Some(target) = mo(d, actor).target else {
        return;
    };
    if !crate::p_sight::check_sight(d, target, dest) {
        return;
    }
    let (dx, dy, dz, dangle) = {
        let m = mo(d, dest);
        (m.x, m.y, m.z, m.angle)
    };
    let an = (dangle >> ANGLETOFINESHIFT) as usize;
    crate::p_maputl::unset_thing_position(d, actor);
    {
        let a = mo_mut(d, actor);
        a.x = dx.wrapping_add(fixed_mul(24 * FRACUNIT, finecosine(an)));
        a.y = dy.wrapping_add(fixed_mul(24 * FRACUNIT, finesine(an)));
        a.z = dz;
    }
    crate::p_maputl::set_thing_position(d, actor);
}

/// A_VileTarget: spawns the fire on the target.
fn a_vile_target(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    let (tx, tz) = (mo(d, t).x, mo(d, t).z);
    // The original passes the target x coordinate twice.
    let fog = p_mobj::spawn_mobj(d, tx, tx, tz, info::MT_FIRE);
    mo_mut(d, actor).tracer = Some(fog);
    mo_mut(d, fog).target = Some(actor);
    mo_mut(d, fog).tracer = Some(t);
    a_fire(d, fog);
}

/// A_VileAttack.
fn a_vile_attack(d: &mut Doom, actor: MobjId) {
    let Some(t) = mo(d, actor).target else { return };
    a_face_target(d, actor);
    if !crate::p_sight::check_sight(d, actor, t) {
        return;
    }
    start_sound(d, Some(actor), SFX_BAREXP);
    crate::p_inter::damage_mobj(d, t, Some(actor), Some(actor), 20);
    let mass = info_of(d, t).mass;
    mo_mut(d, t).momz = 1000 * FRACUNIT / mass;
    let an = (mo(d, actor).angle >> ANGLETOFINESHIFT) as usize;
    let Some(fire) = mo(d, actor).tracer else {
        return;
    };
    let (tx, ty) = (mo(d, t).x, mo(d, t).y);
    {
        let f = mo_mut(d, fire);
        f.x = tx.wrapping_sub(fixed_mul(24 * FRACUNIT, finecosine(an)));
        f.y = ty.wrapping_sub(fixed_mul(24 * FRACUNIT, finesine(an)));
    }
    crate::p_map::radius_attack(d, fire, Some(actor), 70);
}

fn fat_missile(d: &mut Doom, actor: MobjId, target: MobjId, turn: Angle) {
    let m = p_mobj::spawn_missile(d, actor, target, info::MT_FATSHOT);
    let speed = info_of(d, m).speed;
    let mm = mo_mut(d, m);
    mm.angle = mm.angle.wrapping_add(turn);
    let an = (mm.angle >> ANGLETOFINESHIFT) as usize;
    mm.momx = fixed_mul(speed, finecosine(an));
    mm.momy = fixed_mul(speed, finesine(an));
}

/// A_FatRaise.
fn a_fat_raise(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    start_sound(d, Some(actor), SFX_MANATK);
}

/// A_FatAttack1.
fn a_fat_attack1(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    let m = mo_mut(d, actor);
    m.angle = m.angle.wrapping_add(FATSPREAD);
    let Some(t) = mo(d, actor).target else { return };
    p_mobj::spawn_missile(d, actor, t, info::MT_FATSHOT);
    fat_missile(d, actor, t, FATSPREAD);
}

/// A_FatAttack2.
fn a_fat_attack2(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    let m = mo_mut(d, actor);
    m.angle = m.angle.wrapping_sub(FATSPREAD);
    let Some(t) = mo(d, actor).target else { return };
    p_mobj::spawn_missile(d, actor, t, info::MT_FATSHOT);
    fat_missile(d, actor, t, (FATSPREAD * 2).wrapping_neg());
}

/// A_FatAttack3.
fn a_fat_attack3(d: &mut Doom, actor: MobjId) {
    a_face_target(d, actor);
    let Some(t) = mo(d, actor).target else { return };
    fat_missile(d, actor, t, (FATSPREAD / 2).wrapping_neg());
    fat_missile(d, actor, t, FATSPREAD / 2);
}

/// A_SkullAttack: the lost soul charges.
pub fn a_skull_attack(d: &mut Doom, actor: MobjId) {
    let Some(dest) = mo(d, actor).target else {
        return;
    };
    mo_mut(d, actor).flags |= MF_SKULLFLY;
    let snd = info_of(d, actor).attacksound;
    start_sound(d, Some(actor), snd);
    a_face_target(d, actor);
    let (dx, dy, dz, dh) = {
        let m = mo(d, dest);
        (m.x, m.y, m.z, m.height)
    };
    let a = mo_mut(d, actor);
    let an = (a.angle >> ANGLETOFINESHIFT) as usize;
    a.momx = fixed_mul(SKULLSPEED, finecosine(an));
    a.momy = fixed_mul(SKULLSPEED, finesine(an));
    let mut dist = crate::p_maputl::approx_distance(dx.wrapping_sub(a.x), dy.wrapping_sub(a.y));
    dist /= SKULLSPEED;
    if dist < 1 {
        dist = 1;
    }
    a.momz = (dz + (dh >> 1) - a.z) / dist;
}

/// A_PainShootSkull: spawns a lost soul and launches it.
fn pain_shoot_skull(d: &mut Doom, actor: MobjId, angle: Angle) {
    let mut count = 0;
    for th in d.p_tick.ids() {
        if d.p_tick.thinkers[th].func == Think::Mobj && mo(d, th).type_ == info::MT_SKULL {
            count += 1;
        }
    }
    if count > 20 {
        return;
    }
    let an = (angle >> ANGLETOFINESHIFT) as usize;
    let a = mo(d, actor);
    let prestep =
        4 * FRACUNIT + 3 * (MOBJINFO[a.type_].radius + MOBJINFO[info::MT_SKULL].radius) / 2;
    let x = a.x.wrapping_add(fixed_mul(prestep, finecosine(an)));
    let y = a.y.wrapping_add(fixed_mul(prestep, finesine(an)));
    let z = a.z + 8 * FRACUNIT;
    let newmobj = p_mobj::spawn_mobj(d, x, y, z, info::MT_SKULL);
    let (nx, ny) = (mo(d, newmobj).x, mo(d, newmobj).y);
    if !crate::p_map::try_move(d, newmobj, nx, ny) {
        crate::p_inter::damage_mobj(d, newmobj, Some(actor), Some(actor), 10000);
        return;
    }
    let t = mo(d, actor).target;
    mo_mut(d, newmobj).target = t;
    a_skull_attack(d, newmobj);
}

/// A_PainAttack.
fn a_pain_attack(d: &mut Doom, actor: MobjId) {
    if mo(d, actor).target.is_none() {
        return;
    }
    a_face_target(d, actor);
    let angle = mo(d, actor).angle;
    pain_shoot_skull(d, actor, angle);
}

/// A_PainDie.
fn a_pain_die(d: &mut Doom, actor: MobjId) {
    a_fall(d, actor);
    let angle = mo(d, actor).angle;
    pain_shoot_skull(d, actor, angle.wrapping_add(ANG90));
    pain_shoot_skull(d, actor, angle.wrapping_add(ANG180));
    pain_shoot_skull(d, actor, angle.wrapping_add(ANG270));
}

/// A_Scream.
fn a_scream(d: &mut Doom, actor: MobjId) {
    let sound = match info_of(d, actor).deathsound {
        0 => return,
        SFX_PODTH1 | SFX_PODTH2 | SFX_PODTH3 => SFX_PODTH1 + rnd(d) % 3,
        SFX_BGDTH1 | SFX_BGDTH2 => SFX_BGDTH1 + rnd(d) % 2,
        s => s,
    };
    let type_ = mo(d, actor).type_;
    if type_ == info::MT_SPIDER || type_ == info::MT_CYBORG {
        start_sound(d, None, sound);
    } else {
        start_sound(d, Some(actor), sound);
    }
}

/// A_XScream.
fn a_xscream(d: &mut Doom, actor: MobjId) {
    start_sound(d, Some(actor), SFX_SLOP);
}

/// A_Pain.
fn a_pain(d: &mut Doom, actor: MobjId) {
    let s = info_of(d, actor).painsound;
    if s != 0 {
        start_sound(d, Some(actor), s);
    }
}

/// A_Fall: the corpse stops blocking.
fn a_fall(d: &mut Doom, actor: MobjId) {
    mo_mut(d, actor).flags &= !MF_SOLID;
}

/// A_Explode.
fn a_explode(d: &mut Doom, thingy: MobjId) {
    let t = mo(d, thingy).target;
    crate::p_map::radius_attack(d, thingy, t, 128);
}

/// CheckBossEnd: decides whether the dead type ends the map (version dependent).
fn check_boss_end(d: &Doom, motype: usize) -> bool {
    let (episode, map) = (d.g_game.gameepisode, d.g_game.gamemap);
    if d.d_main.gameversion < crate::d_main::EXE_ULTIMATE {
        if map != 8 {
            return false;
        }
        return !(motype == info::MT_BRUISER && episode != 1);
    }
    match episode {
        1 => map == 8 && motype == info::MT_BRUISER,
        2 => map == 8 && motype == info::MT_CYBORG,
        3 => map == 8 && motype == info::MT_SPIDER,
        4 => (map == 6 && motype == info::MT_CYBORG) || (map == 8 && motype == info::MT_SPIDER),
        _ => map == 8,
    }
}

/// A_BossDeath: runs the map special when the last boss dies.
fn a_boss_death(d: &mut Doom, actor: MobjId) {
    let type_ = mo(d, actor).type_;
    let commercial = d.d_main.gamemode == crate::d_main::COMMERCIAL;
    if commercial {
        if d.g_game.gamemap != 7 {
            return;
        }
        if type_ != info::MT_FATSO && type_ != info::MT_BABY {
            return;
        }
    } else if !check_boss_end(d, type_) {
        return;
    }
    let alive = (0..crate::g_game::MAXPLAYERS)
        .any(|i| d.g_game.playeringame[i] && d.g_game.players[i].health > 0);
    if !alive {
        return;
    }
    for th in d.p_tick.ids() {
        if d.p_tick.thinkers[th].func != Think::Mobj || th == actor {
            continue;
        }
        let m = mo(d, th);
        if m.type_ == type_ && m.health > 0 {
            return;
        }
    }
    if commercial {
        if d.g_game.gamemap == 7 {
            if type_ == info::MT_FATSO {
                junk_line_special(d, 666, |d, l| {
                    crate::p_floor::ev_do_floor(d, l, LOWER_FLOOR_TO_LOWEST);
                });
                return;
            }
            if type_ == info::MT_BABY {
                junk_line_special(d, 667, |d, l| {
                    crate::p_floor::ev_do_floor(d, l, RAISE_TO_TEXTURE);
                });
                return;
            }
        }
    } else {
        match d.g_game.gameepisode {
            1 => {
                junk_line_special(d, 666, |d, l| {
                    crate::p_floor::ev_do_floor(d, l, LOWER_FLOOR_TO_LOWEST);
                });
                return;
            }
            4 => match d.g_game.gamemap {
                6 => {
                    junk_line_special(d, 666, |d, l| {
                        crate::p_doors::ev_do_door(d, l, VLD_BLAZE_OPEN);
                    });
                    return;
                }
                8 => {
                    junk_line_special(d, 666, |d, l| {
                        crate::p_floor::ev_do_floor(d, l, LOWER_FLOOR_TO_LOWEST);
                    });
                    return;
                }
                _ => {}
            },
            _ => {}
        }
    }
    crate::g_game::exit_level(d);
}

/// A_Hoof.
fn a_hoof(d: &mut Doom, mo_: MobjId) {
    start_sound(d, Some(mo_), SFX_HOOF);
    a_chase(d, mo_);
}

/// A_Metal.
fn a_metal(d: &mut Doom, mo_: MobjId) {
    start_sound(d, Some(mo_), SFX_METAL);
    a_chase(d, mo_);
}

/// A_BabyMetal.
fn a_baby_metal(d: &mut Doom, mo_: MobjId) {
    start_sound(d, Some(mo_), SFX_BSPWLK);
    a_chase(d, mo_);
}

/// A_BrainAwake: collects the spawn targets.
fn a_brain_awake(d: &mut Doom, _mo: MobjId) {
    d.p_enemy.braintargets.clear();
    d.p_enemy.braintargeton = 0;
    for th in d.p_tick.ids() {
        if d.p_tick.thinkers[th].func == Think::Mobj && mo(d, th).type_ == info::MT_BOSSTARGET {
            d.p_enemy.braintargets.push(th);
        }
    }
    start_sound(d, None, SFX_BOSSIT);
}

/// A_BrainPain.
fn a_brain_pain(d: &mut Doom, _mo: MobjId) {
    start_sound(d, None, SFX_BOSPN);
}

fn brain_rocket(d: &mut Doom, x: Fixed, y: Fixed) {
    let z = 128 + rnd(d) * 2 * FRACUNIT;
    let th = p_mobj::spawn_mobj(d, x, y, z, info::MT_ROCKET);
    let r = rnd(d);
    mo_mut(d, th).momz = r * 512;
    p_mobj::set_mobj_state(d, th, info::S_BRAINEXPLODE1);
    let r = rnd(d) & 7;
    let t = mo_mut(d, th);
    t.tics -= r;
    if t.tics < 1 {
        t.tics = 1;
    }
}

/// A_BrainScream.
fn a_brain_scream(d: &mut Doom, m: MobjId) {
    let (mx, my) = (mo(d, m).x, mo(d, m).y);
    let mut x = mx - 196 * FRACUNIT;
    while x < mx + 320 * FRACUNIT {
        brain_rocket(d, x, my - 320 * FRACUNIT);
        x += FRACUNIT * 8;
    }
    start_sound(d, None, SFX_BOSDTH);
}

/// A_BrainExplode.
fn a_brain_explode(d: &mut Doom, m: MobjId) {
    let x = mo(d, m).x.wrapping_add(sub_random(d) * 2048);
    let y = mo(d, m).y;
    brain_rocket(d, x, y);
}

/// A_BrainDie.
fn a_brain_die(d: &mut Doom, _mo: MobjId) {
    crate::g_game::exit_level(d);
}

/// A_BrainSpit: shoots a spawn cube at the next target.
fn a_brain_spit(d: &mut Doom, m: MobjId) {
    let old = d.p_enemy.easy;
    d.p_enemy.easy ^= 1;
    if d.g_game.gameskill <= crate::g_game::SK_EASY && old == 1 {
        return;
    }
    let n = d.p_enemy.braintargets.len() as i32;
    let on = d.p_enemy.braintargeton;
    let targ = d.p_enemy.braintargets[on as usize];
    d.p_enemy.braintargeton = if n == 0 { 0 } else { (on + 1) % n };
    let newmobj = p_mobj::spawn_missile(d, m, targ, info::MT_SPAWNSHOT);
    mo_mut(d, newmobj).target = Some(targ);
    let ty = mo(d, targ).y.wrapping_sub(mo(d, m).y);
    let nm = mo(d, newmobj);
    let momy = nm.momy;
    let tics = crate::g_game::state_tics(d, nm.state);
    let q = if momy == 0 { 0 } else { ty.wrapping_div(momy) };
    mo_mut(d, newmobj).reactiontime = if tics == 0 { 0 } else { q.wrapping_div(tics) };
    start_sound(d, None, SFX_BOSPIT);
}

/// A_SpawnSound.
fn a_spawn_sound(d: &mut Doom, m: MobjId) {
    start_sound(d, Some(m), SFX_BOSCUB);
    a_spawn_fly(d, m);
}

/// A_SpawnFly: the cube reaches its target and spawns a monster.
fn a_spawn_fly(d: &mut Doom, m: MobjId) {
    {
        let mm = mo_mut(d, m);
        mm.reactiontime -= 1;
        if mm.reactiontime != 0 {
            return;
        }
    }
    let targ = mo(d, m).target.unwrap();
    let (tx, ty, tz) = (mo(d, targ).x, mo(d, targ).y, mo(d, targ).z);
    let fog = p_mobj::spawn_mobj(d, tx, ty, tz, info::MT_SPAWNFIRE);
    start_sound(d, Some(fog), SFX_TELEPT);
    let r = rnd(d);
    let type_ = if r < 50 {
        info::MT_TROOP
    } else if r < 90 {
        info::MT_SERGEANT
    } else if r < 120 {
        info::MT_SHADOWS
    } else if r < 130 {
        info::MT_PAIN
    } else if r < 160 {
        info::MT_HEAD
    } else if r < 162 {
        info::MT_VILE
    } else if r < 172 {
        info::MT_UNDEAD
    } else if r < 192 {
        info::MT_BABY
    } else if r < 222 {
        info::MT_FATSO
    } else if r < 246 {
        info::MT_KNIGHT
    } else {
        info::MT_BRUISER
    };
    let newmobj = p_mobj::spawn_mobj(d, tx, ty, tz, type_);
    if look_for_players(d, newmobj, true) {
        let s = info_of(d, newmobj).seestate as usize;
        p_mobj::set_mobj_state(d, newmobj, s);
    }
    let (nx, ny) = (mo(d, newmobj).x, mo(d, newmobj).y);
    crate::p_map::teleport_move(d, newmobj, nx, ny);
    p_mobj::remove_mobj(d, m);
}

/// A_PlayerScream.
fn a_player_scream(d: &mut Doom, m: MobjId) {
    let mut sound = SFX_PLDETH;
    if d.d_main.gamemode == crate::d_main::COMMERCIAL && mo(d, m).health < -50 {
        sound = SFX_PDIEHI;
    }
    start_sound(d, Some(m), sound);
}

/// Calls the action of a mobj state. Weapon actions belong to the player sprites and do nothing here.
pub fn call_action(d: &mut Doom, action: Action, mobj: MobjId) {
    use Action::*;
    match action {
        Explode => a_explode(d, mobj),
        Pain => a_pain(d, mobj),
        PlayerScream => a_player_scream(d, mobj),
        Fall => a_fall(d, mobj),
        XScream => a_xscream(d, mobj),
        Look => a_look(d, mobj),
        Chase => a_chase(d, mobj),
        FaceTarget => a_face_target(d, mobj),
        PosAttack => a_pos_attack(d, mobj),
        Scream => a_scream(d, mobj),
        SPosAttack => a_spos_attack(d, mobj),
        VileChase => a_vile_chase(d, mobj),
        VileStart => a_vile_start(d, mobj),
        VileTarget => a_vile_target(d, mobj),
        VileAttack => a_vile_attack(d, mobj),
        StartFire => a_start_fire(d, mobj),
        Fire => a_fire(d, mobj),
        FireCrackle => a_fire_crackle(d, mobj),
        Tracer => a_tracer(d, mobj),
        SkelWhoosh => a_skel_whoosh(d, mobj),
        SkelFist => a_skel_fist(d, mobj),
        SkelMissile => a_skel_missile(d, mobj),
        FatRaise => a_fat_raise(d, mobj),
        FatAttack1 => a_fat_attack1(d, mobj),
        FatAttack2 => a_fat_attack2(d, mobj),
        FatAttack3 => a_fat_attack3(d, mobj),
        BossDeath => a_boss_death(d, mobj),
        CPosAttack => a_cpos_attack(d, mobj),
        CPosRefire => a_cpos_refire(d, mobj),
        TroopAttack => a_troop_attack(d, mobj),
        SargAttack => a_sarg_attack(d, mobj),
        HeadAttack => a_head_attack(d, mobj),
        BruisAttack => a_bruis_attack(d, mobj),
        SkullAttack => a_skull_attack(d, mobj),
        Metal => a_metal(d, mobj),
        SpidRefire => a_spid_refire(d, mobj),
        BabyMetal => a_baby_metal(d, mobj),
        BspiAttack => a_bspi_attack(d, mobj),
        Hoof => a_hoof(d, mobj),
        CyberAttack => a_cyber_attack(d, mobj),
        PainAttack => a_pain_attack(d, mobj),
        PainDie => a_pain_die(d, mobj),
        KeenDie => a_keen_die(d, mobj),
        BrainPain => a_brain_pain(d, mobj),
        BrainScream => a_brain_scream(d, mobj),
        BrainDie => a_brain_die(d, mobj),
        BrainAwake => a_brain_awake(d, mobj),
        BrainSpit => a_brain_spit(d, mobj),
        SpawnSound => a_spawn_sound(d, mobj),
        SpawnFly => a_spawn_fly(d, mobj),
        BrainExplode => a_brain_explode(d, mobj),
        BFGSpray => crate::p_pspr::a_bfg_spray(d, mobj),
        None | Light0 | WeaponReady | Lower | Raise | Punch | ReFire | FirePistol | Light1
        | FireShotgun | Light2 | FireShotgun2 | CheckReload | OpenShotgun2 | LoadShotgun2
        | CloseShotgun2 | FireCGun | GunFlash | FireMissile | Saw | FirePlasma | BFGsound
        | FireBFG => {}
    }
}

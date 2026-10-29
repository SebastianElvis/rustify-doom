//! Map objects: spawning, state changes, movement, and removal.

use crate::g_game::{CF_NOMOMENTUM, SK_NIGHTMARE};
use crate::info::{Action, MOBJINFO, STATES};
use crate::m_fixed::Fixed;
use crate::m_fixed::{fixed_mul, FRACUNIT};
use crate::m_random::p_random;
use crate::p_setup::MapThing;
use crate::p_tick::ThinkerId;
use crate::p_tick::{Think, ThinkerData};
use crate::tables::Angle;
use crate::tables::{finecosine, finesine, ANG45, ANGLETOFINESHIFT};
use crate::Doom;

pub type MobjId = ThinkerId;

pub const MF_SPECIAL: i32 = 1;
pub const MF_SOLID: i32 = 2;
pub const MF_SHOOTABLE: i32 = 4;
pub const MF_NOSECTOR: i32 = 8;
pub const MF_NOBLOCKMAP: i32 = 16;
pub const MF_AMBUSH: i32 = 32;
pub const MF_JUSTHIT: i32 = 64;
pub const MF_JUSTATTACKED: i32 = 128;
pub const MF_SPAWNCEILING: i32 = 256;
pub const MF_NOGRAVITY: i32 = 512;
pub const MF_DROPOFF: i32 = 0x400;
pub const MF_PICKUP: i32 = 0x800;
pub const MF_NOCLIP: i32 = 0x1000;
pub const MF_SLIDE: i32 = 0x2000;
pub const MF_FLOAT: i32 = 0x4000;
pub const MF_TELEPORT: i32 = 0x8000;
pub const MF_MISSILE: i32 = 0x10000;
pub const MF_DROPPED: i32 = 0x20000;
pub const MF_SHADOW: i32 = 0x40000;
pub const MF_NOBLOOD: i32 = 0x80000;
pub const MF_CORPSE: i32 = 0x100000;
pub const MF_INFLOAT: i32 = 0x200000;
pub const MF_COUNTKILL: i32 = 0x400000;
pub const MF_COUNTITEM: i32 = 0x800000;
pub const MF_SKULLFLY: i32 = 0x1000000;
pub const MF_NOTDMATCH: i32 = 0x2000000;
pub const MF_TRANSLATION: i32 = 0xc000000;
pub const MF_TRANSSHIFT: i32 = 26;

pub const ONFLOORZ: Fixed = i32::MIN;
pub const ONCEILINGZ: Fixed = i32::MAX;

#[derive(Clone, Debug)]
pub struct Mobj {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
    pub snext: Option<MobjId>,
    pub sprev: Option<MobjId>,
    pub angle: Angle,
    pub sprite: usize,
    pub frame: i32,
    pub bnext: Option<MobjId>,
    pub bprev: Option<MobjId>,
    pub subsector: usize,
    pub floorz: Fixed,
    pub ceilingz: Fixed,
    pub radius: Fixed,
    pub height: Fixed,
    pub momx: Fixed,
    pub momy: Fixed,
    pub momz: Fixed,
    pub validcount: i32,
    /// The mobjinfo index (mobjtype_t). `info` in the original points at MOBJINFO[type_].
    pub type_: usize,
    pub tics: i32,
    pub state: usize,
    pub flags: i32,
    pub health: i32,
    pub movedir: i32,
    pub movecount: i32,
    pub target: Option<MobjId>,
    pub reactiontime: i32,
    pub threshold: i32,
    /// The player index for a player body.
    pub player: Option<usize>,
    pub lastlook: i32,
    pub spawnpoint: MapThing,
    pub tracer: Option<MobjId>,
}

pub const ITEMQUESIZE: usize = 128;
pub const VIEWHEIGHT: Fixed = 41 * FRACUNIT;
pub const MTF_AMBUSH: i16 = 8;
const MAXMOVE: Fixed = 30 * FRACUNIT;
const STOPSPEED: Fixed = 0x1000;
const FRICTION: Fixed = 0xe800;
const GRAVITY: Fixed = FRACUNIT;
const FLOATSPEED: Fixed = 4 * FRACUNIT;
const MT_INV: usize = 56;
const MT_INS: usize = 58;

pub struct State {
    pub itemrespawnque: [MapThing; ITEMQUESIZE],
    pub itemrespawntime: [i32; ITEMQUESIZE],
    pub iquehead: usize,
    pub iquetail: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            itemrespawnque: [MapThing::default(); ITEMQUESIZE],
            itemrespawntime: [0; ITEMQUESIZE],
            iquehead: 0,
            iquetail: 0,
        }
    }
}

fn rnd(d: &mut Doom) -> i32 {
    p_random(&mut d.m_random)
}

fn sub_random(d: &mut Doom) -> i32 {
    let r = rnd(d);
    r - rnd(d)
}

fn mo(d: &Doom, id: MobjId) -> &Mobj {
    d.p_tick.mobj(id)
}

fn mo_mut(d: &mut Doom, id: MobjId) -> &mut Mobj {
    d.p_tick.mobj_mut(id)
}

fn removed(d: &Doom, id: MobjId) -> bool {
    d.p_tick.thinkers[id].func == Think::Removed
}

/// P_SetMobjState. Returns false when the object removes itself.
pub fn set_mobj_state(d: &mut Doom, mobj: MobjId, state: usize) -> bool {
    let mut state = state;
    loop {
        if state == crate::info::S_NULL {
            mo_mut(d, mobj).state = crate::info::S_NULL;
            remove_mobj(d, mobj);
            return false;
        }
        let st = &STATES[state];
        let tics = crate::g_game::state_tics(d, state);
        let m = mo_mut(d, mobj);
        m.state = state;
        m.tics = tics;
        m.sprite = st.sprite;
        m.frame = st.frame;
        if st.action != Action::None {
            crate::p_enemy::call_action(d, st.action, mobj);
        }
        state = st.nextstate;
        if mo(d, mobj).tics != 0 {
            return true;
        }
    }
}

/// P_ExplodeMissile.
pub fn explode_missile(d: &mut Doom, mobj: MobjId) {
    let m = mo_mut(d, mobj);
    m.momx = 0;
    m.momy = 0;
    m.momz = 0;
    let info = &MOBJINFO[m.type_];
    set_mobj_state(d, mobj, info.deathstate as usize);
    let r = rnd(d) & 3;
    let m = mo_mut(d, mobj);
    m.tics -= r;
    if m.tics < 1 {
        m.tics = 1;
    }
    m.flags &= !MF_MISSILE;
    if info.deathsound != 0 {
        crate::s_sound::start_sound(d, Some(mobj), info.deathsound);
    }
}

/// P_XYMovement.
fn xy_movement(d: &mut Doom, mobj: MobjId) {
    let m = mo_mut(d, mobj);
    if m.momx == 0 && m.momy == 0 {
        if m.flags & MF_SKULLFLY != 0 {
            m.flags &= !MF_SKULLFLY;
            m.momx = 0;
            m.momy = 0;
            m.momz = 0;
            let st = MOBJINFO[m.type_].spawnstate as usize;
            set_mobj_state(d, mobj, st);
        }
        return;
    }
    let player = m.player;
    m.momx = m.momx.clamp(-MAXMOVE, MAXMOVE);
    m.momy = m.momy.clamp(-MAXMOVE, MAXMOVE);
    let mut xmove = m.momx;
    let mut ymove = m.momy;
    loop {
        let m = mo(d, mobj);
        let (ptryx, ptryy);
        if xmove > MAXMOVE / 2 || ymove > MAXMOVE / 2 {
            ptryx = m.x.wrapping_add(xmove / 2);
            ptryy = m.y.wrapping_add(ymove / 2);
            xmove >>= 1;
            ymove >>= 1;
        } else {
            ptryx = m.x.wrapping_add(xmove);
            ptryy = m.y.wrapping_add(ymove);
            xmove = 0;
            ymove = 0;
        }
        if !crate::p_map::try_move(d, mobj, ptryx, ptryy) {
            let m = mo(d, mobj);
            if m.player.is_some() {
                crate::p_map::slide_move(d, mobj);
            } else if m.flags & MF_MISSILE != 0 {
                if let Some(cl) = d.p_map.ceilingline {
                    if let Some(bs) = d.p_setup.lines[cl].backsector {
                        if d.p_setup.sectors[bs].ceilingpic == d.r_sky.skyflatnum {
                            remove_mobj(d, mobj);
                            return;
                        }
                    }
                }
                explode_missile(d, mobj);
            } else {
                let m = mo_mut(d, mobj);
                m.momx = 0;
                m.momy = 0;
            }
        }
        if xmove == 0 && ymove == 0 {
            break;
        }
    }
    if let Some(p) = player {
        if d.g_game.players[p].cheats & CF_NOMOMENTUM != 0 {
            let m = mo_mut(d, mobj);
            m.momx = 0;
            m.momy = 0;
            return;
        }
    }
    let m = mo(d, mobj);
    if m.flags & (MF_MISSILE | MF_SKULLFLY) != 0 {
        return;
    }
    if m.z > m.floorz {
        return;
    }
    if m.flags & MF_CORPSE != 0
        && (m.momx > FRACUNIT / 4
            || m.momx < -FRACUNIT / 4
            || m.momy > FRACUNIT / 4
            || m.momy < -FRACUNIT / 4)
        && m.floorz != d.p_setup.sectors[d.p_setup.subsectors[m.subsector].sector].floorheight
    {
        return;
    }
    let still = match player {
        None => true,
        Some(p) => {
            let cmd = &d.g_game.players[p].cmd;
            cmd.forwardmove == 0 && cmd.sidemove == 0
        }
    };
    if m.momx > -STOPSPEED
        && m.momx < STOPSPEED
        && m.momy > -STOPSPEED
        && m.momy < STOPSPEED
        && still
    {
        if let Some(p) = player {
            if let Some(pmo) = d.g_game.players[p].mo {
                if mo(d, pmo).state.wrapping_sub(crate::info::S_PLAY_RUN1) < 4 {
                    set_mobj_state(d, pmo, crate::info::S_PLAY);
                }
            }
        }
        let m = mo_mut(d, mobj);
        m.momx = 0;
        m.momy = 0;
    } else {
        let m = mo_mut(d, mobj);
        m.momx = fixed_mul(m.momx, FRICTION);
        m.momy = fixed_mul(m.momy, FRICTION);
    }
}

/// P_ZMovement.
fn z_movement(d: &mut Doom, mobj: MobjId) {
    let m = mo(d, mobj);
    if let Some(p) = m.player {
        if m.z < m.floorz {
            let diff = m.floorz.wrapping_sub(m.z);
            let pl = &mut d.g_game.players[p];
            pl.viewheight -= diff;
            pl.deltaviewheight = (VIEWHEIGHT - pl.viewheight) >> 3;
        }
    }
    let m = mo_mut(d, mobj);
    m.z = m.z.wrapping_add(m.momz);
    if m.flags & MF_FLOAT != 0 {
        if let Some(t) = m.target {
            if m.flags & (MF_SKULLFLY | MF_INFLOAT) == 0 {
                let (mx, my, mz, mh) = (m.x, m.y, m.z, m.height);
                let tm = mo(d, t);
                let dist =
                    crate::p_maputl::approx_distance(mx.wrapping_sub(tm.x), my.wrapping_sub(tm.y));
                let delta = tm.z.wrapping_add(mh >> 1).wrapping_sub(mz);
                let m = mo_mut(d, mobj);
                if delta < 0 && dist < -(delta.wrapping_mul(3)) {
                    m.z -= FLOATSPEED;
                } else if delta > 0 && dist < delta.wrapping_mul(3) {
                    m.z += FLOATSPEED;
                }
            }
        }
    }
    let correct_lost_soul_bounce = d.d_main.gameversion >= crate::d_main::EXE_ULTIMATE;
    let m = mo(d, mobj);
    if m.z <= m.floorz {
        let m = mo_mut(d, mobj);
        if correct_lost_soul_bounce && m.flags & MF_SKULLFLY != 0 {
            m.momz = -m.momz;
        }
        if m.momz < 0 {
            if let Some(p) = m.player {
                if m.momz < -GRAVITY * 8 {
                    d.g_game.players[p].deltaviewheight = m.momz >> 3;
                    crate::s_sound::start_sound(d, Some(mobj), crate::s_sound::SFX_OOF);
                }
            }
            mo_mut(d, mobj).momz = 0;
        }
        let m = mo_mut(d, mobj);
        m.z = m.floorz;
        if !correct_lost_soul_bounce && m.flags & MF_SKULLFLY != 0 {
            m.momz = -m.momz;
        }
        if m.flags & MF_MISSILE != 0 && m.flags & MF_NOCLIP == 0 {
            explode_missile(d, mobj);
            return;
        }
    } else if m.flags & MF_NOGRAVITY == 0 {
        let m = mo_mut(d, mobj);
        if m.momz == 0 {
            m.momz = -GRAVITY * 2;
        } else {
            m.momz -= GRAVITY;
        }
    }
    let m = mo_mut(d, mobj);
    if m.z.wrapping_add(m.height) > m.ceilingz {
        if m.momz > 0 {
            m.momz = 0;
        }
        m.z = m.ceilingz - m.height;
        if m.flags & MF_SKULLFLY != 0 {
            m.momz = -m.momz;
        }
        if m.flags & MF_MISSILE != 0 && m.flags & MF_NOCLIP == 0 {
            explode_missile(d, mobj);
        }
    }
}

/// P_NightmareRespawn.
fn nightmare_respawn(d: &mut Doom, mobj: MobjId) {
    let sp = mo(d, mobj).spawnpoint;
    let x = (sp.x as i32) << 16;
    let y = (sp.y as i32) << 16;
    if !crate::p_map::check_position(d, mobj, x, y) {
        return;
    }
    let m = mo(d, mobj);
    let (mx, my, ty) = (m.x, m.y, m.type_);
    let fh = d.p_setup.sectors[d.p_setup.subsectors[m.subsector].sector].floorheight;
    let fog = spawn_mobj(d, mx, my, fh, crate::info::MT_TFOG);
    crate::s_sound::start_sound(d, Some(fog), crate::s_sound::SFX_TELEPT);
    let ss = crate::r_main::point_in_subsector(d, x, y);
    let fh = d.p_setup.sectors[d.p_setup.subsectors[ss].sector].floorheight;
    let fog = spawn_mobj(d, x, y, fh, crate::info::MT_TFOG);
    crate::s_sound::start_sound(d, Some(fog), crate::s_sound::SFX_TELEPT);
    let z = if MOBJINFO[ty].flags & MF_SPAWNCEILING != 0 {
        ONCEILINGZ
    } else {
        ONFLOORZ
    };
    let n = spawn_mobj(d, x, y, z, ty);
    let nm = mo_mut(d, n);
    nm.spawnpoint = sp;
    nm.angle = ANG45.wrapping_mul((sp.angle / 45) as i32 as u32);
    if sp.options & MTF_AMBUSH != 0 {
        nm.flags |= MF_AMBUSH;
    }
    nm.reactiontime = 18;
    remove_mobj(d, mobj);
}

/// P_MobjThinker.
pub fn mobj_thinker(d: &mut Doom, mobj: MobjId) {
    let m = mo(d, mobj);
    if m.momx != 0 || m.momy != 0 || m.flags & MF_SKULLFLY != 0 {
        xy_movement(d, mobj);
        if removed(d, mobj) {
            return;
        }
    }
    let m = mo(d, mobj);
    if m.z != m.floorz || m.momz != 0 {
        z_movement(d, mobj);
        if removed(d, mobj) {
            return;
        }
    }
    let m = mo_mut(d, mobj);
    if m.tics != -1 {
        m.tics -= 1;
        if m.tics == 0 {
            let next = STATES[m.state].nextstate;
            set_mobj_state(d, mobj, next);
        }
    } else {
        if m.flags & MF_COUNTKILL == 0 || !d.g_game.respawnmonsters {
            return;
        }
        let m = mo_mut(d, mobj);
        m.movecount += 1;
        if m.movecount < 12 * 35 {
            return;
        }
        if d.p_tick.leveltime & 31 != 0 {
            return;
        }
        if rnd(d) > 4 {
            return;
        }
        nightmare_respawn(d, mobj);
    }
}

/// P_SpawnMobj.
pub fn spawn_mobj(d: &mut Doom, x: Fixed, y: Fixed, z: Fixed, type_: usize) -> MobjId {
    let info = &MOBJINFO[type_];
    let reactiontime = if d.g_game.gameskill != SK_NIGHTMARE {
        info.reactiontime
    } else {
        0
    };
    let lastlook = rnd(d) % crate::g_game::MAXPLAYERS as i32;
    let st = &STATES[info.spawnstate as usize];
    let m = Mobj {
        x,
        y,
        z: 0,
        snext: None,
        sprev: None,
        angle: 0,
        sprite: st.sprite,
        frame: st.frame,
        bnext: None,
        bprev: None,
        subsector: 0,
        floorz: 0,
        ceilingz: 0,
        radius: info.radius,
        height: info.height,
        momx: 0,
        momy: 0,
        momz: 0,
        validcount: 0,
        type_,
        tics: crate::g_game::state_tics(d, info.spawnstate as usize),
        state: info.spawnstate as usize,
        flags: info.flags,
        health: info.spawnhealth,
        movedir: 0,
        movecount: 0,
        target: None,
        reactiontime,
        threshold: 0,
        player: None,
        lastlook,
        spawnpoint: MapThing::default(),
        tracer: None,
    };
    let id = d
        .p_tick
        .add_thinker(Think::Mobj, ThinkerData::Mobj(Box::new(m)));
    crate::p_maputl::set_thing_position(d, id);
    let sec = d.p_setup.subsectors[mo(d, id).subsector].sector;
    let (fh, ch) = (
        d.p_setup.sectors[sec].floorheight,
        d.p_setup.sectors[sec].ceilingheight,
    );
    let m = mo_mut(d, id);
    m.floorz = fh;
    m.ceilingz = ch;
    m.z = if z == ONFLOORZ {
        fh
    } else if z == ONCEILINGZ {
        ch - info.height
    } else {
        z
    };
    id
}

/// P_RemoveMobj.
pub fn remove_mobj(d: &mut Doom, mobj: MobjId) {
    let m = mo(d, mobj);
    if m.flags & MF_SPECIAL != 0
        && m.flags & MF_DROPPED == 0
        && m.type_ != MT_INV
        && m.type_ != MT_INS
    {
        let sp = m.spawnpoint;
        let s = &mut d.p_mobj;
        s.itemrespawnque[s.iquehead] = sp;
        s.itemrespawntime[s.iquehead] = d.p_tick.leveltime;
        s.iquehead = (s.iquehead + 1) & (ITEMQUESIZE - 1);
        if s.iquehead == s.iquetail {
            s.iquetail = (s.iquetail + 1) & (ITEMQUESIZE - 1);
        }
    }
    crate::p_maputl::unset_thing_position(d, mobj);
    crate::s_sound::stop_sound(d, mobj);
    d.p_tick.remove_thinker(mobj);
}

/// P_RespawnSpecials.
pub fn respawn_specials(d: &mut Doom) {
    if d.g_game.deathmatch != 2 {
        return;
    }
    let s = &d.p_mobj;
    if s.iquehead == s.iquetail {
        return;
    }
    if d.p_tick.leveltime - s.itemrespawntime[s.iquetail] < 30 * 35 {
        return;
    }
    let mthing = s.itemrespawnque[s.iquetail];
    let x = (mthing.x as i32) << 16;
    let y = (mthing.y as i32) << 16;
    let ss = crate::r_main::point_in_subsector(d, x, y);
    let fh = d.p_setup.sectors[d.p_setup.subsectors[ss].sector].floorheight;
    let fog = spawn_mobj(d, x, y, fh, crate::info::MT_IFOG);
    crate::s_sound::start_sound(d, Some(fog), crate::s_sound::SFX_ITMBK);
    let i = MOBJINFO
        .iter()
        .position(|m| m.doomednum == mthing.type_ as i32)
        .unwrap_or(crate::info::NUMMOBJTYPES);
    // The original reads past the table when no type matches.
    let i = i.min(crate::info::NUMMOBJTYPES - 1);
    let z = if MOBJINFO[i].flags & MF_SPAWNCEILING != 0 {
        ONCEILINGZ
    } else {
        ONFLOORZ
    };
    let n = spawn_mobj(d, x, y, z, i);
    let m = mo_mut(d, n);
    m.spawnpoint = mthing;
    m.angle = ANG45.wrapping_mul((mthing.angle / 45) as i32 as u32);
    d.p_mobj.iquetail = (d.p_mobj.iquetail + 1) & (ITEMQUESIZE - 1);
}

/// P_SpawnPlayer.
pub fn spawn_player(d: &mut Doom, mthing: &MapThing) {
    if mthing.type_ == 0 {
        return;
    }
    let pn = (mthing.type_ - 1) as usize;
    if !d.g_game.playeringame[pn] {
        return;
    }
    if d.g_game.players[pn].playerstate == crate::g_game::PST_REBORN {
        crate::g_game::player_reborn(d, pn);
    }
    let x = (mthing.x as i32) << 16;
    let y = (mthing.y as i32) << 16;
    let id = spawn_mobj(d, x, y, ONFLOORZ, crate::info::MT_PLAYER);
    let health = d.g_game.players[pn].health;
    let m = mo_mut(d, id);
    if mthing.type_ > 1 {
        m.flags |= (pn as i32) << MF_TRANSSHIFT;
    }
    m.angle = ANG45.wrapping_mul((mthing.angle / 45) as i32 as u32);
    m.player = Some(pn);
    m.health = health;
    let p = &mut d.g_game.players[pn];
    p.mo = Some(id);
    p.playerstate = crate::g_game::PST_LIVE;
    p.refire = 0;
    p.message = None;
    p.damagecount = 0;
    p.bonuscount = 0;
    p.extralight = 0;
    p.fixedcolormap = 0;
    p.viewheight = VIEWHEIGHT;
    crate::p_pspr::setup_psprites(d, pn);
    if d.g_game.deathmatch != 0 {
        d.g_game.players[pn].cards = [true; crate::d_items::NUMCARDS];
    }
    if pn == d.g_game.consoleplayer {
        crate::st_stuff::start(d);
        crate::hu_stuff::start(d);
    }
}

/// P_SpawnMapThing.
pub fn spawn_map_thing(d: &mut Doom, mthing: &MapThing) {
    if mthing.type_ == 11 {
        if d.p_setup.deathmatchstarts.len() < 10 {
            d.p_setup.deathmatchstarts.push(*mthing);
        }
        return;
    }
    if mthing.type_ <= 0 {
        return;
    }
    if mthing.type_ <= 4 {
        let i = (mthing.type_ - 1) as usize;
        d.p_setup.playerstarts[i] = *mthing;
        d.p_setup.playerstartsingame[i] = true;
        if d.g_game.deathmatch == 0 {
            spawn_player(d, mthing);
        }
        return;
    }
    if !d.g_game.netgame && mthing.options & 16 != 0 {
        return;
    }
    let bit = match d.g_game.gameskill {
        crate::g_game::SK_BABY => 1,
        SK_NIGHTMARE => 4,
        s => 1 << (s - 1),
    };
    if mthing.options as i32 & bit == 0 {
        return;
    }
    let Some(i) = MOBJINFO
        .iter()
        .position(|m| m.doomednum == mthing.type_ as i32)
    else {
        let msg = format!(
            "P_SpawnMapThing: Unknown type {} at ({}, {})",
            mthing.type_, mthing.x, mthing.y
        );
        crate::i_system::error(d, &msg);
    };
    let flags = MOBJINFO[i].flags;
    if d.g_game.deathmatch != 0 && flags & MF_NOTDMATCH != 0 {
        return;
    }
    if d.g_game.nomonsters && (i == crate::info::MT_SKULL || flags & MF_COUNTKILL != 0) {
        return;
    }
    let x = (mthing.x as i32) << 16;
    let y = (mthing.y as i32) << 16;
    let z = if flags & MF_SPAWNCEILING != 0 {
        ONCEILINGZ
    } else {
        ONFLOORZ
    };
    let id = spawn_mobj(d, x, y, z, i);
    mo_mut(d, id).spawnpoint = *mthing;
    if mo(d, id).tics > 0 {
        let r = rnd(d);
        let m = mo_mut(d, id);
        m.tics = 1 + r % m.tics;
    }
    let f = mo(d, id).flags;
    if f & MF_COUNTKILL != 0 {
        d.g_game.totalkills += 1;
    }
    if f & MF_COUNTITEM != 0 {
        d.g_game.totalitems += 1;
    }
    let m = mo_mut(d, id);
    m.angle = ANG45.wrapping_mul((mthing.angle / 45) as i32 as u32);
    if mthing.options & MTF_AMBUSH != 0 {
        m.flags |= MF_AMBUSH;
    }
}

/// P_SpawnPuff.
pub fn spawn_puff(d: &mut Doom, x: Fixed, y: Fixed, z: Fixed) {
    let z = z.wrapping_add(sub_random(d) << 10);
    let th = spawn_mobj(d, x, y, z, crate::info::MT_PUFF);
    mo_mut(d, th).momz = FRACUNIT;
    let r = rnd(d) & 3;
    let m = mo_mut(d, th);
    m.tics -= r;
    if m.tics < 1 {
        m.tics = 1;
    }
    if d.p_map.attackrange == crate::p_map::MELEERANGE {
        set_mobj_state(d, th, crate::info::S_PUFF3);
    }
}

/// P_SpawnBlood.
pub fn spawn_blood(d: &mut Doom, x: Fixed, y: Fixed, z: Fixed, damage: i32) {
    let z = z.wrapping_add(sub_random(d) << 10);
    let th = spawn_mobj(d, x, y, z, crate::info::MT_BLOOD);
    mo_mut(d, th).momz = FRACUNIT * 2;
    let r = rnd(d) & 3;
    let m = mo_mut(d, th);
    m.tics -= r;
    if m.tics < 1 {
        m.tics = 1;
    }
    if (9..=12).contains(&damage) {
        set_mobj_state(d, th, crate::info::S_BLOOD2);
    } else if damage < 9 {
        set_mobj_state(d, th, crate::info::S_BLOOD3);
    }
}

/// P_CheckMissileSpawn.
pub fn check_missile_spawn(d: &mut Doom, th: MobjId) {
    let r = rnd(d) & 3;
    let m = mo_mut(d, th);
    m.tics -= r;
    if m.tics < 1 {
        m.tics = 1;
    }
    m.x = m.x.wrapping_add(m.momx >> 1);
    m.y = m.y.wrapping_add(m.momy >> 1);
    m.z = m.z.wrapping_add(m.momz >> 1);
    let (x, y) = (m.x, m.y);
    if !crate::p_map::try_move(d, th, x, y) {
        explode_missile(d, th);
    }
}

/// P_SpawnMissile.
pub fn spawn_missile(d: &mut Doom, source: MobjId, dest: MobjId, type_: usize) -> MobjId {
    let s = mo(d, source);
    let (sx, sy, sz) = (s.x, s.y, s.z);
    let th = spawn_mobj(d, sx, sy, sz.wrapping_add(4 * 8 * FRACUNIT), type_);
    let info = &MOBJINFO[type_];
    if info.seesound != 0 {
        crate::s_sound::start_sound(d, Some(th), info.seesound);
    }
    mo_mut(d, th).target = Some(source);
    let t = mo(d, dest);
    let (dx, dy, dz, dflags) = (t.x, t.y, t.z, t.flags);
    let mut an = crate::r_main::point_to_angle2(sx, sy, dx, dy);
    if dflags & MF_SHADOW != 0 {
        an = an.wrapping_add((sub_random(d) << 20) as u32);
    }
    let speed = crate::g_game::mobj_speed(d, type_);
    let m = mo_mut(d, th);
    m.angle = an;
    let fa = (an >> ANGLETOFINESHIFT) as usize;
    m.momx = fixed_mul(speed, finecosine(fa));
    m.momy = fixed_mul(speed, finesine(fa));
    let mut dist = crate::p_maputl::approx_distance(dx.wrapping_sub(sx), dy.wrapping_sub(sy));
    dist /= speed;
    if dist < 1 {
        dist = 1;
    }
    m.momz = dz.wrapping_sub(sz) / dist;
    check_missile_spawn(d, th);
    th
}

/// P_SpawnPlayerMissile.
pub fn spawn_player_missile(d: &mut Doom, source: MobjId, type_: usize) {
    let sangle = mo(d, source).angle;
    let mut an = sangle;
    let mut slope = crate::p_map::aim_line_attack(d, source, an, 16 * 64 * FRACUNIT);
    if d.p_map.linetarget.is_none() {
        an = an.wrapping_add(1 << 26);
        slope = crate::p_map::aim_line_attack(d, source, an, 16 * 64 * FRACUNIT);
        if d.p_map.linetarget.is_none() {
            an = an.wrapping_sub(2 << 26);
            slope = crate::p_map::aim_line_attack(d, source, an, 16 * 64 * FRACUNIT);
        }
        if d.p_map.linetarget.is_none() {
            an = sangle;
            slope = 0;
        }
    }
    let s = mo(d, source);
    let (x, y, z) = (s.x, s.y, s.z.wrapping_add(4 * 8 * FRACUNIT));
    let th = spawn_mobj(d, x, y, z, type_);
    let info = &MOBJINFO[type_];
    if info.seesound != 0 {
        crate::s_sound::start_sound(d, Some(th), info.seesound);
    }
    let m = mo_mut(d, th);
    m.target = Some(source);
    m.angle = an;
    let fa = (an >> ANGLETOFINESHIFT) as usize;
    m.momx = fixed_mul(info.speed, finecosine(fa));
    m.momy = fixed_mul(info.speed, finesine(fa));
    m.momz = fixed_mul(info.speed, slope);
    check_missile_spawn(d, th);
}

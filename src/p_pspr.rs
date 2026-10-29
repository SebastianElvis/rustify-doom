//! Player weapon sprites and weapon actions.

use crate::d_items::*;
use crate::g_game::PST_DEAD;
use crate::info::{self, Action, STATES};
use crate::m_fixed::{fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::m_random::p_random;
use crate::p_enemy::{sub_random, MELEERANGE, MISSILERANGE};
use crate::p_mobj::{self, MobjId, MF_JUSTATTACKED};
use crate::s_sound::{self, *};
use crate::tables::{finecosine, finesine, Angle, ANG180, ANG90, FINEANGLES, FINEMASK};
use crate::Doom;

pub const PS_WEAPON: usize = 0;
pub const PS_FLASH: usize = 1;
pub const NUMPSPRITES: usize = 2;

const LOWERSPEED: Fixed = FRACUNIT * 6;
const RAISESPEED: Fixed = FRACUNIT * 6;
pub const WEAPONBOTTOM: Fixed = 128 * FRACUNIT;
pub const WEAPONTOP: Fixed = 32 * FRACUNIT;
const BFGCELLS: i32 = 40;
const S_SAW: usize = 67;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct PspDef {
    /// The state index, or None for an empty sprite.
    pub state: Option<usize>,
    pub tics: i32,
    pub sx: Fixed,
    pub sy: Fixed,
}

#[derive(Default)]
pub struct State {
    pub bulletslope: Fixed,
}

fn pmo(d: &Doom, player: usize) -> MobjId {
    d.g_game.players[player].mo.unwrap()
}

fn rnd(d: &mut Doom) -> i32 {
    p_random(&mut d.m_random)
}

/// P_SetPsprite: sets the sprite state and runs the zero-tic states and their actions.
pub fn set_psprite(d: &mut Doom, player: usize, position: usize, stnum: usize) {
    let mut stnum = stnum;
    loop {
        if stnum == 0 {
            d.g_game.players[player].psprites[position].state = None;
            break;
        }
        let st = &STATES[stnum];
        {
            let psp = &mut d.g_game.players[player].psprites[position];
            psp.state = Some(stnum);
            psp.tics = st.tics;
            if st.misc1 != 0 {
                psp.sx = st.misc1 << FRACBITS;
                psp.sy = st.misc2 << FRACBITS;
            }
        }
        if st.action != Action::None {
            call_weapon_action(d, st.action, player, position);
            if d.g_game.players[player].psprites[position].state.is_none() {
                break;
            }
        }
        let psp = d.g_game.players[player].psprites[position];
        stnum = STATES[psp.state.unwrap()].nextstate;
        if psp.tics != 0 {
            break;
        }
    }
}

/// P_BringUpWeapon: starts raising the pending weapon from the bottom.
pub fn bring_up_weapon(d: &mut Doom, player: usize) {
    let p = &mut d.g_game.players[player];
    if p.pendingweapon == WP_NOCHANGE {
        p.pendingweapon = p.readyweapon;
    }
    if p.pendingweapon == WP_CHAINSAW {
        let mo = pmo(d, player);
        s_sound::start_sound(d, Some(mo), SFX_SAWUP);
    }
    let p = &mut d.g_game.players[player];
    let newstate = WEAPONINFO[p.pendingweapon as usize].upstate;
    p.pendingweapon = WP_NOCHANGE;
    p.psprites[PS_WEAPON].sy = WEAPONBOTTOM;
    set_psprite(d, player, PS_WEAPON, newstate);
}

/// P_CheckAmmo: returns true when the ready weapon can fire; otherwise selects another weapon.
pub fn check_ammo(d: &mut Doom, player: usize) -> bool {
    let commercial = d.d_main.gamemode == crate::d_main::COMMERCIAL;
    let shareware = d.d_main.gamemode == crate::d_main::SHAREWARE;
    let p = &mut d.g_game.players[player];
    let ammo = WEAPONINFO[p.readyweapon as usize].ammo;
    let count = if p.readyweapon == WP_BFG {
        BFGCELLS
    } else if p.readyweapon == WP_SUPERSHOTGUN {
        2
    } else {
        1
    };
    if ammo == AM_NOAMMO || p.ammo[ammo as usize] >= count {
        return true;
    }
    let owned = |w: i32| p.weaponowned[w as usize];
    let am = |a: i32| p.ammo[a as usize];
    p.pendingweapon = if owned(WP_PLASMA) && am(AM_CELL) != 0 && !shareware {
        WP_PLASMA
    } else if owned(WP_SUPERSHOTGUN) && am(AM_SHELL) > 2 && commercial {
        WP_SUPERSHOTGUN
    } else if owned(WP_CHAINGUN) && am(AM_CLIP) != 0 {
        WP_CHAINGUN
    } else if owned(WP_SHOTGUN) && am(AM_SHELL) != 0 {
        WP_SHOTGUN
    } else if am(AM_CLIP) != 0 {
        WP_PISTOL
    } else if owned(WP_CHAINSAW) {
        WP_CHAINSAW
    } else if owned(WP_MISSILE) && am(AM_MISL) != 0 {
        WP_MISSILE
    } else if owned(WP_BFG) && am(AM_CELL) > 40 && !shareware {
        WP_BFG
    } else {
        WP_FIST
    };
    let down = WEAPONINFO[p.readyweapon as usize].downstate;
    set_psprite(d, player, PS_WEAPON, down);
    false
}

/// P_FireWeapon.
pub fn fire_weapon(d: &mut Doom, player: usize) {
    if !check_ammo(d, player) {
        return;
    }
    let mo = pmo(d, player);
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK1);
    let newstate = WEAPONINFO[d.g_game.players[player].readyweapon as usize].atkstate;
    set_psprite(d, player, PS_WEAPON, newstate);
    crate::p_enemy::noise_alert(d, mo, mo);
}

/// P_DropWeapon: the player died, so the weapon goes down.
pub fn drop_weapon(d: &mut Doom, player: usize) {
    let down = WEAPONINFO[d.g_game.players[player].readyweapon as usize].downstate;
    set_psprite(d, player, PS_WEAPON, down);
}

/// A_WeaponReady: fires, changes weapons, or bobs the weapon.
fn a_weapon_ready(d: &mut Doom, player: usize, psp: usize) {
    let mo = pmo(d, player);
    let st = d.p_tick.mobj(mo).state;
    if st == info::S_PLAY_ATK1 || st == info::S_PLAY_ATK2 {
        p_mobj::set_mobj_state(d, mo, info::S_PLAY);
    }
    if d.g_game.players[player].readyweapon == WP_CHAINSAW
        && d.g_game.players[player].psprites[psp].state == Some(S_SAW)
    {
        s_sound::start_sound(d, Some(mo), SFX_SAWIDL);
    }
    let p = &mut d.g_game.players[player];
    if p.pendingweapon != WP_NOCHANGE || p.health == 0 {
        let down = WEAPONINFO[p.readyweapon as usize].downstate;
        set_psprite(d, player, PS_WEAPON, down);
        return;
    }
    if p.cmd.buttons & crate::g_game::BT_ATTACK != 0 {
        if !p.attackdown || (p.readyweapon != WP_MISSILE && p.readyweapon != WP_BFG) {
            p.attackdown = true;
            fire_weapon(d, player);
            return;
        }
    } else {
        p.attackdown = false;
    }
    let leveltime = d.p_tick.leveltime;
    let p = &mut d.g_game.players[player];
    let mut angle = (128 * leveltime) as usize & FINEMASK;
    p.psprites[psp].sx = FRACUNIT + fixed_mul(p.bob, finecosine(angle));
    angle &= FINEANGLES / 2 - 1;
    p.psprites[psp].sy = WEAPONTOP + fixed_mul(p.bob, finesine(angle));
}

/// A_ReFire: fires again while the attack button stays down.
fn a_refire(d: &mut Doom, player: usize, _psp: usize) {
    let p = &mut d.g_game.players[player];
    if p.cmd.buttons & crate::g_game::BT_ATTACK != 0
        && p.pendingweapon == WP_NOCHANGE
        && p.health != 0
    {
        p.refire += 1;
        fire_weapon(d, player);
    } else {
        p.refire = 0;
        check_ammo(d, player);
    }
}

/// A_CheckReload.
fn a_check_reload(d: &mut Doom, player: usize, _psp: usize) {
    check_ammo(d, player);
}

/// A_Lower: lowers the weapon and then brings up the pending weapon.
fn a_lower(d: &mut Doom, player: usize, psp: usize) {
    let p = &mut d.g_game.players[player];
    p.psprites[psp].sy += LOWERSPEED;
    if p.psprites[psp].sy < WEAPONBOTTOM {
        return;
    }
    if p.playerstate == PST_DEAD {
        p.psprites[psp].sy = WEAPONBOTTOM;
        return;
    }
    if p.health == 0 {
        set_psprite(d, player, PS_WEAPON, info::S_NULL);
        return;
    }
    p.readyweapon = p.pendingweapon;
    bring_up_weapon(d, player);
}

/// A_Raise.
fn a_raise(d: &mut Doom, player: usize, psp: usize) {
    let p = &mut d.g_game.players[player];
    p.psprites[psp].sy -= RAISESPEED;
    if p.psprites[psp].sy > WEAPONTOP {
        return;
    }
    p.psprites[psp].sy = WEAPONTOP;
    let ready = WEAPONINFO[p.readyweapon as usize].readystate;
    set_psprite(d, player, PS_WEAPON, ready);
}

/// A_GunFlash.
fn a_gun_flash(d: &mut Doom, player: usize, _psp: usize) {
    let mo = pmo(d, player);
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK2);
    let flash = WEAPONINFO[d.g_game.players[player].readyweapon as usize].flashstate;
    set_psprite(d, player, PS_FLASH, flash);
}

fn point_to_linetarget(d: &Doom, mo: MobjId) -> Option<Angle> {
    let lt = d.p_map.linetarget?;
    let (m, t) = (d.p_tick.mobj(mo), d.p_tick.mobj(lt));
    Some(crate::r_main::point_to_angle2(m.x, m.y, t.x, t.y))
}

/// A_Punch.
fn a_punch(d: &mut Doom, player: usize, _psp: usize) {
    let mut damage = (rnd(d) % 10 + 1) << 1;
    if d.g_game.players[player].powers[PW_STRENGTH] != 0 {
        damage *= 10;
    }
    let mo = pmo(d, player);
    let angle = d
        .p_tick
        .mobj(mo)
        .angle
        .wrapping_add((sub_random(d) << 18) as u32);
    let slope = crate::p_map::aim_line_attack(d, mo, angle, MELEERANGE);
    crate::p_map::line_attack(d, mo, angle, MELEERANGE, slope, damage);
    if let Some(a) = point_to_linetarget(d, mo) {
        s_sound::start_sound(d, Some(mo), SFX_PUNCH);
        d.p_tick.mobj_mut(mo).angle = a;
    }
}

/// A_Saw.
fn a_saw(d: &mut Doom, player: usize, _psp: usize) {
    let damage = 2 * (rnd(d) % 10 + 1);
    let mo = pmo(d, player);
    let angle = d
        .p_tick
        .mobj(mo)
        .angle
        .wrapping_add((sub_random(d) << 18) as u32);
    let slope = crate::p_map::aim_line_attack(d, mo, angle, MELEERANGE + 1);
    crate::p_map::line_attack(d, mo, angle, MELEERANGE + 1, slope, damage);
    let Some(angle) = point_to_linetarget(d, mo) else {
        s_sound::start_sound(d, Some(mo), SFX_SAWFUL);
        return;
    };
    s_sound::start_sound(d, Some(mo), SFX_SAWHIT);
    let m = d.p_tick.mobj_mut(mo);
    if angle.wrapping_sub(m.angle) > ANG180 {
        if (angle.wrapping_sub(m.angle) as i32) < -((ANG90 / 20) as i32) {
            m.angle = angle.wrapping_add(ANG90 / 21);
        } else {
            m.angle = m.angle.wrapping_sub(ANG90 / 20);
        }
    } else if angle.wrapping_sub(m.angle) > ANG90 / 20 {
        m.angle = angle.wrapping_sub(ANG90 / 21);
    } else {
        m.angle = m.angle.wrapping_add(ANG90 / 20);
    }
    m.flags |= MF_JUSTATTACKED;
}

/// DecreaseAmmo: the no-ammo type writes into the maximum ammo array, as the original does.
fn decrease_ammo(d: &mut Doom, player: usize, ammonum: i32, amount: i32) {
    let p = &mut d.g_game.players[player];
    if (ammonum as usize) < NUMAMMO {
        p.ammo[ammonum as usize] -= amount;
    } else {
        p.maxammo[ammonum as usize - NUMAMMO] -= amount;
    }
}

fn ready_info(d: &Doom, player: usize) -> WeaponInfo {
    WEAPONINFO[d.g_game.players[player].readyweapon as usize]
}

/// A_FireMissile.
fn a_fire_missile(d: &mut Doom, player: usize, _psp: usize) {
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, 1);
    let mo = pmo(d, player);
    p_mobj::spawn_player_missile(d, mo, info::MT_ROCKET);
}

/// A_FireBFG.
fn a_fire_bfg(d: &mut Doom, player: usize, _psp: usize) {
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, BFGCELLS);
    let mo = pmo(d, player);
    p_mobj::spawn_player_missile(d, mo, info::MT_BFG);
}

/// A_FirePlasma.
fn a_fire_plasma(d: &mut Doom, player: usize, _psp: usize) {
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, 1);
    let r = (rnd(d) & 1) as usize;
    set_psprite(d, player, PS_FLASH, w.flashstate + r);
    let mo = pmo(d, player);
    p_mobj::spawn_player_missile(d, mo, info::MT_PLASMA);
}

/// P_BulletSlope: aims at the center, then to the right, then to the left.
fn bullet_slope(d: &mut Doom, mo: MobjId) {
    let mut an = d.p_tick.mobj(mo).angle;
    d.p_pspr.bulletslope = crate::p_map::aim_line_attack(d, mo, an, 16 * 64 * FRACUNIT);
    if d.p_map.linetarget.is_none() {
        an = an.wrapping_add(1 << 26);
        d.p_pspr.bulletslope = crate::p_map::aim_line_attack(d, mo, an, 16 * 64 * FRACUNIT);
        if d.p_map.linetarget.is_none() {
            an = an.wrapping_sub(2 << 26);
            d.p_pspr.bulletslope = crate::p_map::aim_line_attack(d, mo, an, 16 * 64 * FRACUNIT);
        }
    }
}

/// P_GunShot.
fn gun_shot(d: &mut Doom, mo: MobjId, accurate: bool) {
    let damage = 5 * (rnd(d) % 3 + 1);
    let mut angle = d.p_tick.mobj(mo).angle;
    if !accurate {
        angle = angle.wrapping_add((sub_random(d) << 18) as u32);
    }
    let slope = d.p_pspr.bulletslope;
    crate::p_map::line_attack(d, mo, angle, MISSILERANGE, slope, damage);
}

/// A_FirePistol.
fn a_fire_pistol(d: &mut Doom, player: usize, _psp: usize) {
    let mo = pmo(d, player);
    s_sound::start_sound(d, Some(mo), SFX_PISTOL);
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK2);
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, 1);
    set_psprite(d, player, PS_FLASH, w.flashstate);
    bullet_slope(d, mo);
    let accurate = d.g_game.players[player].refire == 0;
    gun_shot(d, mo, accurate);
}

/// A_FireShotgun.
fn a_fire_shotgun(d: &mut Doom, player: usize, _psp: usize) {
    let mo = pmo(d, player);
    s_sound::start_sound(d, Some(mo), SFX_SHOTGN);
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK2);
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, 1);
    set_psprite(d, player, PS_FLASH, w.flashstate);
    bullet_slope(d, mo);
    for _ in 0..7 {
        gun_shot(d, mo, false);
    }
}

/// A_FireShotgun2.
fn a_fire_shotgun2(d: &mut Doom, player: usize, _psp: usize) {
    let mo = pmo(d, player);
    s_sound::start_sound(d, Some(mo), SFX_DSHTGN);
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK2);
    let w = ready_info(d, player);
    decrease_ammo(d, player, w.ammo, 2);
    set_psprite(d, player, PS_FLASH, w.flashstate);
    bullet_slope(d, mo);
    for _ in 0..20 {
        let damage = 5 * (rnd(d) % 3 + 1);
        let angle = d
            .p_tick
            .mobj(mo)
            .angle
            .wrapping_add((sub_random(d) << 19) as u32);
        let slope = d.p_pspr.bulletslope.wrapping_add(sub_random(d) << 5);
        crate::p_map::line_attack(d, mo, angle, MISSILERANGE, slope, damage);
    }
}

/// A_FireCGun.
fn a_fire_cgun(d: &mut Doom, player: usize, psp: usize) {
    let mo = pmo(d, player);
    s_sound::start_sound(d, Some(mo), SFX_PISTOL);
    let w = ready_info(d, player);
    if d.g_game.players[player].ammo[w.ammo as usize] == 0 {
        return;
    }
    p_mobj::set_mobj_state(d, mo, info::S_PLAY_ATK2);
    decrease_ammo(d, player, w.ammo, 1);
    let st = d.g_game.players[player].psprites[psp].state.unwrap();
    set_psprite(
        d,
        player,
        PS_FLASH,
        (w.flashstate + st).wrapping_sub(info::S_CHAIN1),
    );
    bullet_slope(d, mo);
    let accurate = d.g_game.players[player].refire == 0;
    gun_shot(d, mo, accurate);
}

/// A_BFGSpray: the BFG ball hits the things in front of the shooter.
pub fn a_bfg_spray(d: &mut Doom, mo: MobjId) {
    let Some(source) = d.p_tick.mobj(mo).target else {
        return;
    };
    for i in 0..40u32 {
        let an = d
            .p_tick
            .mobj(mo)
            .angle
            .wrapping_sub(ANG90 / 2)
            .wrapping_add((ANG90 / 40).wrapping_mul(i));
        crate::p_map::aim_line_attack(d, source, an, 16 * 64 * FRACUNIT);
        let Some(lt) = d.p_map.linetarget else {
            continue;
        };
        let (x, y, z) = {
            let t = d.p_tick.mobj(lt);
            (t.x, t.y, t.z + (t.height >> 2))
        };
        p_mobj::spawn_mobj(d, x, y, z, info::MT_EXTRABFG);
        let mut damage = 0;
        for _ in 0..15 {
            damage += (rnd(d) & 7) + 1;
        }
        crate::p_inter::damage_mobj(d, lt, Some(source), Some(source), damage);
    }
}

fn sound_on_player(d: &mut Doom, player: usize, sfx: i32) {
    let mo = pmo(d, player);
    s_sound::start_sound(d, Some(mo), sfx);
}

/// Calls the action of a player sprite state.
fn call_weapon_action(d: &mut Doom, action: Action, player: usize, psp: usize) {
    use Action::*;
    match action {
        Light0 => d.g_game.players[player].extralight = 0,
        Light1 => d.g_game.players[player].extralight = 1,
        Light2 => d.g_game.players[player].extralight = 2,
        WeaponReady => a_weapon_ready(d, player, psp),
        Lower => a_lower(d, player, psp),
        Raise => a_raise(d, player, psp),
        Punch => a_punch(d, player, psp),
        ReFire => a_refire(d, player, psp),
        FirePistol => a_fire_pistol(d, player, psp),
        FireShotgun => a_fire_shotgun(d, player, psp),
        FireShotgun2 => a_fire_shotgun2(d, player, psp),
        CheckReload => a_check_reload(d, player, psp),
        OpenShotgun2 => sound_on_player(d, player, SFX_DBOPN),
        LoadShotgun2 => sound_on_player(d, player, SFX_DBLOAD),
        CloseShotgun2 => {
            sound_on_player(d, player, SFX_DBCLS);
            a_refire(d, player, psp);
        }
        FireCGun => a_fire_cgun(d, player, psp),
        GunFlash => a_gun_flash(d, player, psp),
        FireMissile => a_fire_missile(d, player, psp),
        Saw => a_saw(d, player, psp),
        FirePlasma => a_fire_plasma(d, player, psp),
        BFGsound => sound_on_player(d, player, SFX_BFG),
        FireBFG => a_fire_bfg(d, player, psp),
        _ => {}
    }
}

/// P_SetupPsprites: clears the sprites and raises the ready weapon.
pub fn setup_psprites(d: &mut Doom, player: usize) {
    let p = &mut d.g_game.players[player];
    for psp in p.psprites.iter_mut() {
        psp.state = None;
    }
    p.pendingweapon = p.readyweapon;
    bring_up_weapon(d, player);
}

/// P_MovePsprites: advances the sprite states by one tic.
pub fn move_psprites(d: &mut Doom, player: usize) {
    for i in 0..NUMPSPRITES {
        let psp = &mut d.g_game.players[player].psprites[i];
        if let Some(st) = psp.state {
            if psp.tics != -1 {
                psp.tics -= 1;
                if psp.tics == 0 {
                    set_psprite(d, player, i, STATES[st].nextstate);
                }
            }
        }
    }
    let p = &mut d.g_game.players[player];
    p.psprites[PS_FLASH].sx = p.psprites[PS_WEAPON].sx;
    p.psprites[PS_FLASH].sy = p.psprites[PS_WEAPON].sy;
}

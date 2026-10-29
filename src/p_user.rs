//! Player think: movement, view height and bob, death view, weapon change, and power counters.

use crate::d_items::*;
use crate::g_game::{
    BT_CHANGE, BT_SPECIAL, BT_USE, BT_WEAPONMASK, BT_WEAPONSHIFT, CF_NOCLIP, CF_NOMOMENTUM,
    PST_DEAD, PST_LIVE, PST_REBORN,
};
use crate::m_fixed::{fixed_mul, Fixed, FRACUNIT};
use crate::p_mobj::{MF_JUSTATTACKED, MF_NOCLIP, MF_SHADOW, VIEWHEIGHT};
use crate::tables::{
    finecosine, finesine, Angle, ANG180, ANG90, ANGLETOFINESHIFT, FINEANGLES, FINEMASK,
};
use crate::Doom;

const INVERSECOLORMAP: i32 = 32;
const MAXBOB: Fixed = 0x100000;
const ANG5: Angle = ANG90 / 18;

#[derive(Default)]
pub struct State {
    pub onground: bool,
}

fn pmo(d: &Doom, player: usize) -> usize {
    d.g_game.players[player].mo.expect("player mobj")
}

/// P_Thrust.
pub fn thrust(d: &mut Doom, player: usize, angle: Angle, mv: Fixed) {
    let fa = (angle >> ANGLETOFINESHIFT) as usize;
    let id = pmo(d, player);
    let m = d.p_tick.mobj_mut(id);
    m.momx = m.momx.wrapping_add(fixed_mul(mv, finecosine(fa)));
    m.momy = m.momy.wrapping_add(fixed_mul(mv, finesine(fa)));
}

/// P_CalcHeight: computes the view bob and the view height.
pub fn calc_height(d: &mut Doom, player: usize) {
    let leveltime = d.p_tick.leveltime;
    let onground = d.p_user.onground;
    let m = d.p_tick.mobj(pmo(d, player)).clone();
    let p = &mut d.g_game.players[player];
    p.bob = fixed_mul(m.momx, m.momx).wrapping_add(fixed_mul(m.momy, m.momy));
    p.bob >>= 2;
    if p.bob > MAXBOB {
        p.bob = MAXBOB;
    }
    if p.cheats & CF_NOMOMENTUM != 0 || !onground {
        p.viewz = m.z.wrapping_add(VIEWHEIGHT);
        if p.viewz > m.ceilingz - 4 * FRACUNIT {
            p.viewz = m.ceilingz - 4 * FRACUNIT;
        }
        p.viewz = m.z.wrapping_add(p.viewheight);
        return;
    }
    let angle = ((FINEANGLES / 20) as i32).wrapping_mul(leveltime) as usize & FINEMASK;
    let bob = fixed_mul(p.bob / 2, finesine(angle));
    if p.playerstate == PST_LIVE {
        p.viewheight += p.deltaviewheight;
        if p.viewheight > VIEWHEIGHT {
            p.viewheight = VIEWHEIGHT;
            p.deltaviewheight = 0;
        }
        if p.viewheight < VIEWHEIGHT / 2 {
            p.viewheight = VIEWHEIGHT / 2;
            if p.deltaviewheight <= 0 {
                p.deltaviewheight = 1;
            }
        }
        if p.deltaviewheight != 0 {
            p.deltaviewheight += FRACUNIT / 4;
            if p.deltaviewheight == 0 {
                p.deltaviewheight = 1;
            }
        }
    }
    p.viewz = m.z.wrapping_add(p.viewheight).wrapping_add(bob);
    if p.viewz > m.ceilingz - 4 * FRACUNIT {
        p.viewz = m.ceilingz - 4 * FRACUNIT;
    }
}

/// P_MovePlayer.
pub fn move_player(d: &mut Doom, player: usize) {
    let cmd = d.g_game.players[player].cmd;
    let id = pmo(d, player);
    let m = d.p_tick.mobj_mut(id);
    m.angle = m.angle.wrapping_add(((cmd.angleturn as i32) << 16) as u32);
    let angle = m.angle;
    d.p_user.onground = m.z <= m.floorz;
    if cmd.forwardmove != 0 && d.p_user.onground {
        thrust(d, player, angle, cmd.forwardmove as i32 * 2048);
    }
    if cmd.sidemove != 0 && d.p_user.onground {
        thrust(
            d,
            player,
            angle.wrapping_sub(ANG90),
            cmd.sidemove as i32 * 2048,
        );
    }
    if (cmd.forwardmove != 0 || cmd.sidemove != 0) && d.p_tick.mobj(id).state == crate::info::S_PLAY
    {
        crate::p_mobj::set_mobj_state(d, id, crate::info::S_PLAY_RUN1);
    }
}

/// P_DeathThink: lowers the view, turns to the killer, and waits for the use key.
pub fn death_think(d: &mut Doom, player: usize) {
    crate::p_pspr::move_psprites(d, player);
    let p = &mut d.g_game.players[player];
    if p.viewheight > 6 * FRACUNIT {
        p.viewheight -= FRACUNIT;
    }
    if p.viewheight < 6 * FRACUNIT {
        p.viewheight = 6 * FRACUNIT;
    }
    p.deltaviewheight = 0;
    let id = pmo(d, player);
    let m = d.p_tick.mobj(id);
    d.p_user.onground = m.z <= m.floorz;
    calc_height(d, player);
    let attacker = d.g_game.players[player].attacker;
    match attacker {
        Some(a) if a != id => {
            let (ax, ay) = (d.p_tick.mobj(a).x, d.p_tick.mobj(a).y);
            let m = d.p_tick.mobj(id);
            let angle = crate::r_main::point_to_angle2(m.x, m.y, ax, ay);
            let delta = angle.wrapping_sub(m.angle);
            if delta < ANG5 || delta > ANG5.wrapping_neg() {
                d.p_tick.mobj_mut(id).angle = angle;
                let p = &mut d.g_game.players[player];
                if p.damagecount != 0 {
                    p.damagecount -= 1;
                }
            } else if delta < ANG180 {
                let m = d.p_tick.mobj_mut(id);
                m.angle = m.angle.wrapping_add(ANG5);
            } else {
                let m = d.p_tick.mobj_mut(id);
                m.angle = m.angle.wrapping_sub(ANG5);
            }
        }
        _ => {
            let p = &mut d.g_game.players[player];
            if p.damagecount != 0 {
                p.damagecount -= 1;
            }
        }
    }
    let p = &mut d.g_game.players[player];
    if p.cmd.buttons & BT_USE != 0 {
        p.playerstate = PST_REBORN;
    }
}

/// P_PlayerThink.
pub fn player_think(d: &mut Doom, player: usize) {
    let id = pmo(d, player);
    let cheats = d.g_game.players[player].cheats;
    let m = d.p_tick.mobj_mut(id);
    if cheats & CF_NOCLIP != 0 {
        m.flags |= MF_NOCLIP;
    } else {
        m.flags &= !MF_NOCLIP;
    }
    if m.flags & MF_JUSTATTACKED != 0 {
        m.flags &= !MF_JUSTATTACKED;
        let cmd = &mut d.g_game.players[player].cmd;
        cmd.angleturn = 0;
        cmd.forwardmove = (0xc800 / 512) as i8;
        cmd.sidemove = 0;
    }
    if d.g_game.players[player].playerstate == PST_DEAD {
        death_think(d, player);
        return;
    }
    let m = d.p_tick.mobj_mut(id);
    if m.reactiontime != 0 {
        m.reactiontime -= 1;
    } else {
        move_player(d, player);
    }
    calc_height(d, player);
    let ss = d.p_tick.mobj(id).subsector;
    if d.p_setup.sectors[d.p_setup.subsectors[ss].sector].special != 0 {
        crate::p_spec::player_in_special_sector(d, player);
    }
    let commercial = d.d_main.gamemode == crate::d_main::COMMERCIAL;
    let shareware = d.d_main.gamemode == crate::d_main::SHAREWARE;
    let p = &mut d.g_game.players[player];
    if p.cmd.buttons & BT_SPECIAL != 0 {
        p.cmd.buttons = 0;
    }
    if p.cmd.buttons & BT_CHANGE != 0 {
        let mut newweapon = ((p.cmd.buttons & BT_WEAPONMASK) >> BT_WEAPONSHIFT) as i32;
        if newweapon == WP_FIST
            && p.weaponowned[WP_CHAINSAW as usize]
            && !(p.readyweapon == WP_CHAINSAW && p.powers[PW_STRENGTH] != 0)
        {
            newweapon = WP_CHAINSAW;
        }
        if commercial
            && newweapon == WP_SHOTGUN
            && p.weaponowned[WP_SUPERSHOTGUN as usize]
            && p.readyweapon != WP_SUPERSHOTGUN
        {
            newweapon = WP_SUPERSHOTGUN;
        }
        if p.weaponowned[newweapon as usize]
            && newweapon != p.readyweapon
            && ((newweapon != WP_PLASMA && newweapon != WP_BFG) || !shareware)
        {
            p.pendingweapon = newweapon;
        }
    }
    if p.cmd.buttons & BT_USE != 0 {
        if !p.usedown {
            crate::p_map::use_lines(d, player);
            d.g_game.players[player].usedown = true;
        }
    } else {
        p.usedown = false;
    }
    crate::p_pspr::move_psprites(d, player);
    let p = &mut d.g_game.players[player];
    if p.powers[PW_STRENGTH] != 0 {
        p.powers[PW_STRENGTH] += 1;
    }
    if p.powers[PW_INVULNERABILITY] != 0 {
        p.powers[PW_INVULNERABILITY] -= 1;
    }
    if p.powers[PW_INVISIBILITY] != 0 {
        p.powers[PW_INVISIBILITY] -= 1;
        if p.powers[PW_INVISIBILITY] == 0 {
            d.p_tick.mobj_mut(id).flags &= !MF_SHADOW;
        }
    }
    let p = &mut d.g_game.players[player];
    if p.powers[PW_INFRARED] != 0 {
        p.powers[PW_INFRARED] -= 1;
    }
    if p.powers[PW_IRONFEET] != 0 {
        p.powers[PW_IRONFEET] -= 1;
    }
    if p.damagecount != 0 {
        p.damagecount -= 1;
    }
    if p.bonuscount != 0 {
        p.bonuscount -= 1;
    }
    let inv = p.powers[PW_INVULNERABILITY];
    let ir = p.powers[PW_INFRARED];
    p.fixedcolormap = if inv != 0 {
        if inv > 4 * 32 || inv & 8 != 0 {
            INVERSECOLORMAP
        } else {
            0
        }
    } else if ir != 0 {
        if ir > 4 * 32 || ir & 8 != 0 {
            1
        } else {
            0
        }
    } else {
        0
    };
}

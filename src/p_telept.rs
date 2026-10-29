//! Teleporters.

use crate::info::{MT_TELEPORTMAN, MT_TFOG};
use crate::p_mobj::{MobjId, MF_MISSILE};
use crate::p_tick::Think;
use crate::s_sound::SFX_TELEPT;
use crate::tables::{finecosine, finesine, ANGLETOFINESHIFT};
use crate::Doom;

#[derive(Default)]
pub struct State {}

/// EV_Teleport: moves a thing to the teleport destination in the tagged sector.
pub fn ev_teleport(d: &mut Doom, line: usize, side: i32, thing: MobjId) -> bool {
    if d.p_tick.mobj(thing).flags & MF_MISSILE != 0 || side == 1 {
        return false;
    }
    let tag = d.p_setup.lines[line].tag;
    for i in 0..d.p_setup.numsectors {
        if d.p_setup.sectors[i].tag != tag {
            continue;
        }
        for th in d.p_tick.ids() {
            if d.p_tick.thinkers[th].func != Think::Mobj {
                continue;
            }
            let m = d.p_tick.mobj(th);
            if m.type_ != MT_TELEPORTMAN || d.p_setup.subsectors[m.subsector].sector != i {
                continue;
            }
            let (mx, my, mangle) = (m.x, m.y, m.angle);
            let (oldx, oldy, oldz) = {
                let t = d.p_tick.mobj(thing);
                (t.x, t.y, t.z)
            };
            if !crate::p_map::teleport_move(d, thing, mx, my) {
                return false;
            }
            if d.d_main.gameversion != crate::d_main::EXE_FINAL {
                let t = d.p_tick.mobj_mut(thing);
                t.z = t.floorz;
            }
            let (tz, player) = {
                let t = d.p_tick.mobj(thing);
                (t.z, t.player)
            };
            if let Some(p) = player {
                let pl = &mut d.g_game.players[p];
                pl.viewz = tz + pl.viewheight;
            }
            let fog = crate::p_mobj::spawn_mobj(d, oldx, oldy, oldz, MT_TFOG);
            crate::s_sound::start_sound(d, Some(fog), SFX_TELEPT);
            let an = (mangle >> ANGLETOFINESHIFT) as usize;
            let fog = crate::p_mobj::spawn_mobj(
                d,
                mx.wrapping_add(20 * finecosine(an)),
                my.wrapping_add(20 * finesine(an)),
                tz,
                MT_TFOG,
            );
            crate::s_sound::start_sound(d, Some(fog), SFX_TELEPT);
            let t = d.p_tick.mobj_mut(thing);
            if player.is_some() {
                t.reactiontime = 18;
            }
            t.angle = mangle;
            t.momx = 0;
            t.momy = 0;
            t.momz = 0;
            return true;
        }
    }
    false
}

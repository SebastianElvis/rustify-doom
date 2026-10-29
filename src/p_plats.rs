//! Platforms and lifts.

use crate::m_fixed::{Fixed, FRACUNIT};
use crate::m_random::p_random;
use crate::p_floor::{t_move_plane, RESULT_CRUSHED, RESULT_PASTDEST};
use crate::p_spec::*;
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::s_sound::{SFX_PSTART, SFX_PSTOP, SFX_STNMOV};
use crate::Doom;

pub const PLAT_UP: i32 = 0;
pub const PLAT_DOWN: i32 = 1;
pub const PLAT_WAITING: i32 = 2;
pub const PLAT_IN_STASIS: i32 = 3;

pub const PLAT_PERPETUALRAISE: i32 = 0;
pub const PLAT_DOWNWAITUPSTAY: i32 = 1;
pub const PLAT_RAISEANDCHANGE: i32 = 2;
pub const PLAT_RAISETONEARESTANDCHANGE: i32 = 3;
pub const PLAT_BLAZEDWUS: i32 = 4;
pub const MAXPLATS: usize = 30;
pub const PLATWAIT: i32 = 3;
pub const PLATSPEED: Fixed = FRACUNIT;

#[derive(Clone, Copy, Default, Debug)]
pub struct Plat {
    pub sector: usize,
    pub speed: Fixed,
    pub low: Fixed,
    pub high: Fixed,
    pub wait: i32,
    pub count: i32,
    pub status: i32,
    pub oldstatus: i32,
    pub crush: bool,
    pub tag: i32,
    pub type_: i32,
}

#[derive(Default)]
pub struct State {
    pub activeplats: [Option<ThinkerId>; MAXPLATS],
}

pub fn plat(d: &mut Doom, id: ThinkerId) -> &mut Plat {
    match &mut d.p_tick.thinkers[id].data {
        ThinkerData::Plat(x) => x,
        _ => panic!("thinker {id} is not a platform"),
    }
}

/// T_PlatRaise.
pub fn t_plat_raise(d: &mut Doom, id: ThinkerId) {
    let p = *plat(d, id);
    let sec = p.sector;
    match p.status {
        PLAT_UP => {
            let res = t_move_plane(d, sec, p.speed, p.high, p.crush as i32, 0, 1);
            if (p.type_ == PLAT_RAISEANDCHANGE || p.type_ == PLAT_RAISETONEARESTANDCHANGE)
                && d.p_tick.leveltime & 7 == 0
            {
                sector_sound(d, sec, SFX_STNMOV);
            }
            if res == RESULT_CRUSHED && !p.crush {
                let pm = plat(d, id);
                pm.count = pm.wait;
                pm.status = PLAT_DOWN;
                sector_sound(d, sec, SFX_PSTART);
            } else if res == RESULT_PASTDEST {
                let pm = plat(d, id);
                pm.count = pm.wait;
                pm.status = PLAT_WAITING;
                sector_sound(d, sec, SFX_PSTOP);
                if matches!(
                    p.type_,
                    PLAT_BLAZEDWUS
                        | PLAT_DOWNWAITUPSTAY
                        | PLAT_RAISEANDCHANGE
                        | PLAT_RAISETONEARESTANDCHANGE
                ) {
                    remove_active_plat(d, id);
                }
            }
        }
        PLAT_DOWN => {
            let res = t_move_plane(d, sec, p.speed, p.low, 0, 0, -1);
            if res == RESULT_PASTDEST {
                let pm = plat(d, id);
                pm.count = pm.wait;
                pm.status = PLAT_WAITING;
                sector_sound(d, sec, SFX_PSTOP);
            }
        }
        PLAT_WAITING => {
            plat(d, id).count -= 1;
            if p.count - 1 == 0 {
                let at_low = d.p_setup.sectors[sec].floorheight == p.low;
                plat(d, id).status = if at_low { PLAT_UP } else { PLAT_DOWN };
                sector_sound(d, sec, SFX_PSTART);
            }
        }
        _ => {}
    }
}

/// EV_DoPlat: starts a platform in each free sector with the tag of the line.
pub fn ev_do_plat(d: &mut Doom, line: usize, type_: i32, amount: i32) -> bool {
    let mut secnum = -1;
    let mut rtn = false;
    let tag = d.p_setup.lines[line].tag;
    if type_ == PLAT_PERPETUALRAISE {
        activate_in_stasis(d, tag);
    }
    loop {
        secnum = find_sector_from_line_tag(d, line, secnum);
        if secnum < 0 {
            break;
        }
        let sec = secnum as usize;
        if d.p_setup.sectors[sec].specialdata.is_some() {
            continue;
        }
        rtn = true;
        let id = d.p_tick.add_thinker(
            Think::PlatRaise,
            ThinkerData::Plat(Plat {
                type_,
                sector: sec,
                tag,
                ..Default::default()
            }),
        );
        d.p_setup.sectors[sec].specialdata = Some(id);
        let fh = d.p_setup.sectors[sec].floorheight;
        let mut p = *plat(d, id);
        let mut sound = SFX_PSTART;
        match type_ {
            PLAT_RAISETONEARESTANDCHANGE | PLAT_RAISEANDCHANGE => {
                p.speed = PLATSPEED / 2;
                let side0 = d.p_setup.lines[line].sidenum[0] as usize;
                let pic = d.p_setup.sectors[d.p_setup.sides[side0].sector].floorpic;
                d.p_setup.sectors[sec].floorpic = pic;
                if type_ == PLAT_RAISETONEARESTANDCHANGE {
                    p.high = find_next_highest_floor(d, sec, fh);
                    d.p_setup.sectors[sec].special = 0;
                } else {
                    p.high = fh + amount * FRACUNIT;
                }
                p.wait = 0;
                p.status = PLAT_UP;
                sound = SFX_STNMOV;
            }
            PLAT_DOWNWAITUPSTAY | PLAT_BLAZEDWUS => {
                p.speed = PLATSPEED * if type_ == PLAT_BLAZEDWUS { 8 } else { 4 };
                p.low = find_lowest_floor_surrounding(d, sec).min(fh);
                p.high = fh;
                p.wait = 35 * PLATWAIT;
                p.status = PLAT_DOWN;
            }
            PLAT_PERPETUALRAISE => {
                p.speed = PLATSPEED;
                p.low = find_lowest_floor_surrounding(d, sec).min(fh);
                p.high = find_highest_floor_surrounding(d, sec).max(fh);
                p.wait = 35 * PLATWAIT;
                p.status = p_random(&mut d.m_random) & 1;
            }
            _ => {}
        }
        *plat(d, id) = p;
        if (PLAT_PERPETUALRAISE..=PLAT_BLAZEDWUS).contains(&type_) {
            sector_sound(d, sec, sound);
        }
        add_active_plat(d, id);
    }
    rtn
}

/// P_ActivateInStasis: restarts the stopped platforms with a tag.
pub fn activate_in_stasis(d: &mut Doom, tag: i32) {
    for i in 0..MAXPLATS {
        let Some(id) = d.p_plats.activeplats[i] else {
            continue;
        };
        let p = plat(d, id);
        if p.tag == tag && p.status == PLAT_IN_STASIS {
            p.status = p.oldstatus;
            d.p_tick.thinkers[id].func = Think::PlatRaise;
        }
    }
}

/// EV_StopPlat: stops the moving platforms with the tag of the line.
pub fn ev_stop_plat(d: &mut Doom, line: usize) {
    let tag = d.p_setup.lines[line].tag;
    for j in 0..MAXPLATS {
        let Some(id) = d.p_plats.activeplats[j] else {
            continue;
        };
        let p = plat(d, id);
        if p.status != PLAT_IN_STASIS && p.tag == tag {
            p.oldstatus = p.status;
            p.status = PLAT_IN_STASIS;
            d.p_tick.thinkers[id].func = Think::Null;
        }
    }
}

/// P_AddActivePlat.
pub fn add_active_plat(d: &mut Doom, id: ThinkerId) {
    match d.p_plats.activeplats.iter_mut().find(|s| s.is_none()) {
        Some(slot) => *slot = Some(id),
        None => crate::i_system::error(d, "P_AddActivePlat: no more plats!"),
    }
}

/// P_RemoveActivePlat.
pub fn remove_active_plat(d: &mut Doom, id: ThinkerId) {
    match d.p_plats.activeplats.iter().position(|s| *s == Some(id)) {
        Some(i) => {
            let sec = plat(d, id).sector;
            d.p_setup.sectors[sec].specialdata = None;
            d.p_tick.remove_thinker(id);
            d.p_plats.activeplats[i] = None;
        }
        None => crate::i_system::error(d, "P_RemoveActivePlat: can't find plat!"),
    }
}

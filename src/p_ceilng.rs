//! Moving ceilings and crushers.

use crate::m_fixed::{Fixed, FRACUNIT};
use crate::p_floor::{t_move_plane, RESULT_CRUSHED, RESULT_PASTDEST};
use crate::p_spec::{find_highest_ceiling_surrounding, find_sector_from_line_tag, sector_sound};
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::s_sound::{SFX_PSTOP, SFX_STNMOV};
use crate::Doom;

pub const CEILING_LOWERTOFLOOR: i32 = 0;
pub const CEILING_RAISETOHIGHEST: i32 = 1;
pub const CEILING_LOWERANDCRUSH: i32 = 2;
pub const CEILING_CRUSHANDRAISE: i32 = 3;
pub const CEILING_FASTCRUSHANDRAISE: i32 = 4;
pub const CEILING_SILENTCRUSHANDRAISE: i32 = 5;
pub const MAXCEILINGS: usize = 30;
pub const CEILSPEED: Fixed = FRACUNIT;

#[derive(Clone, Copy, Default, Debug)]
pub struct Ceiling {
    pub type_: i32,
    pub sector: usize,
    pub bottomheight: Fixed,
    pub topheight: Fixed,
    pub speed: Fixed,
    pub crush: bool,
    pub direction: i32,
    pub tag: i32,
    pub olddirection: i32,
}

#[derive(Default)]
pub struct State {
    pub activeceilings: [Option<ThinkerId>; MAXCEILINGS],
}

pub fn ceiling(d: &mut Doom, id: ThinkerId) -> &mut Ceiling {
    match &mut d.p_tick.thinkers[id].data {
        ThinkerData::Ceiling(x) => x,
        _ => panic!("thinker {id} is not a ceiling"),
    }
}

/// T_MoveCeiling.
pub fn t_move_ceiling(d: &mut Doom, id: ThinkerId) {
    let c = *ceiling(d, id);
    let sound_tic = d.p_tick.leveltime & 7 == 0 && c.type_ != CEILING_SILENTCRUSHANDRAISE;
    match c.direction {
        1 => {
            let res = t_move_plane(d, c.sector, c.speed, c.topheight, 0, 1, c.direction);
            if sound_tic {
                sector_sound(d, c.sector, SFX_STNMOV);
            }
            if res == RESULT_PASTDEST {
                match c.type_ {
                    CEILING_RAISETOHIGHEST => remove_active_ceiling(d, id),
                    CEILING_SILENTCRUSHANDRAISE
                    | CEILING_FASTCRUSHANDRAISE
                    | CEILING_CRUSHANDRAISE => {
                        if c.type_ == CEILING_SILENTCRUSHANDRAISE {
                            sector_sound(d, c.sector, SFX_PSTOP);
                        }
                        ceiling(d, id).direction = -1;
                    }
                    _ => {}
                }
            }
        }
        -1 => {
            let res = t_move_plane(
                d,
                c.sector,
                c.speed,
                c.bottomheight,
                c.crush as i32,
                1,
                c.direction,
            );
            if sound_tic {
                sector_sound(d, c.sector, SFX_STNMOV);
            }
            if res == RESULT_PASTDEST {
                match c.type_ {
                    CEILING_SILENTCRUSHANDRAISE
                    | CEILING_CRUSHANDRAISE
                    | CEILING_FASTCRUSHANDRAISE => {
                        if c.type_ == CEILING_SILENTCRUSHANDRAISE {
                            sector_sound(d, c.sector, SFX_PSTOP);
                        }
                        let cm = ceiling(d, id);
                        if c.type_ != CEILING_FASTCRUSHANDRAISE {
                            cm.speed = CEILSPEED;
                        }
                        cm.direction = 1;
                    }
                    CEILING_LOWERANDCRUSH | CEILING_LOWERTOFLOOR => remove_active_ceiling(d, id),
                    _ => {}
                }
            } else if res == RESULT_CRUSHED
                && matches!(
                    c.type_,
                    CEILING_SILENTCRUSHANDRAISE | CEILING_CRUSHANDRAISE | CEILING_LOWERANDCRUSH
                )
            {
                ceiling(d, id).speed = CEILSPEED / 8;
            }
        }
        _ => {}
    }
}

/// EV_DoCeiling: starts a ceiling mover in each free sector with the tag of the line.
pub fn ev_do_ceiling(d: &mut Doom, line: usize, type_: i32) -> bool {
    let mut secnum = -1;
    let mut rtn = false;
    if matches!(
        type_,
        CEILING_FASTCRUSHANDRAISE | CEILING_SILENTCRUSHANDRAISE | CEILING_CRUSHANDRAISE
    ) {
        activate_in_stasis_ceiling(d, line);
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
        let (fh, ch, tag) = {
            let s = &d.p_setup.sectors[sec];
            (s.floorheight, s.ceilingheight, s.tag)
        };
        let mut c = Ceiling {
            sector: sec,
            ..Default::default()
        };
        match type_ {
            CEILING_FASTCRUSHANDRAISE => {
                c.crush = true;
                c.topheight = ch;
                c.bottomheight = fh + 8 * FRACUNIT;
                c.direction = -1;
                c.speed = CEILSPEED * 2;
            }
            CEILING_SILENTCRUSHANDRAISE
            | CEILING_CRUSHANDRAISE
            | CEILING_LOWERANDCRUSH
            | CEILING_LOWERTOFLOOR => {
                if type_ == CEILING_SILENTCRUSHANDRAISE || type_ == CEILING_CRUSHANDRAISE {
                    c.crush = true;
                    c.topheight = ch;
                }
                c.bottomheight = fh;
                if type_ != CEILING_LOWERTOFLOOR {
                    c.bottomheight += 8 * FRACUNIT;
                }
                c.direction = -1;
                c.speed = CEILSPEED;
            }
            CEILING_RAISETOHIGHEST => {
                c.topheight = find_highest_ceiling_surrounding(d, sec);
                c.direction = 1;
                c.speed = CEILSPEED;
            }
            _ => {}
        }
        c.tag = tag;
        c.type_ = type_;
        let id = d
            .p_tick
            .add_thinker(Think::MoveCeiling, ThinkerData::Ceiling(c));
        d.p_setup.sectors[sec].specialdata = Some(id);
        add_active_ceiling(d, id);
    }
    rtn
}

/// P_AddActiveCeiling. A full list keeps the ceiling out of the list without an error.
pub fn add_active_ceiling(d: &mut Doom, id: ThinkerId) {
    if let Some(slot) = d.p_ceilng.activeceilings.iter_mut().find(|s| s.is_none()) {
        *slot = Some(id);
    }
}

/// P_RemoveActiveCeiling.
pub fn remove_active_ceiling(d: &mut Doom, id: ThinkerId) {
    if let Some(i) = d
        .p_ceilng
        .activeceilings
        .iter()
        .position(|s| *s == Some(id))
    {
        let sec = ceiling(d, id).sector;
        d.p_setup.sectors[sec].specialdata = None;
        d.p_tick.remove_thinker(id);
        d.p_ceilng.activeceilings[i] = None;
    }
}

/// P_ActivateInStasisCeiling.
pub fn activate_in_stasis_ceiling(d: &mut Doom, line: usize) {
    let tag = d.p_setup.lines[line].tag;
    for i in 0..MAXCEILINGS {
        let Some(id) = d.p_ceilng.activeceilings[i] else {
            continue;
        };
        let c = ceiling(d, id);
        if c.tag == tag && c.direction == 0 {
            c.direction = c.olddirection;
            d.p_tick.thinkers[id].func = Think::MoveCeiling;
        }
    }
}

/// EV_CeilingCrushStop: puts the moving ceilings with the tag of the line in stasis.
pub fn ev_ceiling_crush_stop(d: &mut Doom, line: usize) -> bool {
    let tag = d.p_setup.lines[line].tag;
    let mut rtn = false;
    for i in 0..MAXCEILINGS {
        let Some(id) = d.p_ceilng.activeceilings[i] else {
            continue;
        };
        let c = ceiling(d, id);
        if c.tag == tag && c.direction != 0 {
            c.olddirection = c.direction;
            c.direction = 0;
            d.p_tick.thinkers[id].func = Think::Null;
            rtn = true;
        }
    }
    rtn
}

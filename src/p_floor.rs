//! Moving floors, stairs, and the plane movement helper.

use crate::m_fixed::{Fixed, FRACUNIT};
use crate::p_spec::*;
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::s_sound::{SFX_PSTOP, SFX_STNMOV};
use crate::Doom;

pub const FLOOR_LOWERFLOOR: i32 = 0;
pub const FLOOR_LOWERFLOORTOLOWEST: i32 = 1;
pub const FLOOR_TURBOLOWER: i32 = 2;
pub const FLOOR_RAISEFLOOR: i32 = 3;
pub const FLOOR_RAISEFLOORTONEAREST: i32 = 4;
pub const FLOOR_RAISETOTEXTURE: i32 = 5;
pub const FLOOR_LOWERANDCHANGE: i32 = 6;
pub const FLOOR_RAISEFLOOR24: i32 = 7;
pub const FLOOR_RAISEFLOOR24ANDCHANGE: i32 = 8;
pub const FLOOR_RAISEFLOORCRUSH: i32 = 9;
pub const FLOOR_RAISEFLOORTURBO: i32 = 10;
pub const FLOOR_DONUTRAISE: i32 = 11;
pub const FLOOR_RAISEFLOOR512: i32 = 12;

pub const STAIRS_BUILD8: i32 = 0;
pub const STAIRS_TURBO16: i32 = 1;

pub const RESULT_OK: i32 = 0;
pub const RESULT_CRUSHED: i32 = 1;
pub const RESULT_PASTDEST: i32 = 2;

pub const FLOORSPEED: Fixed = FRACUNIT;
/// The stair builder stores this value in the crush field. It is true for P_ChangeSector but not equal to 1.
const STAIRS_UNINITIALIZED_CRUSH_FIELD_VALUE: i32 = 10;

#[derive(Clone, Copy, Default, Debug)]
pub struct FloorMove {
    pub type_: i32,
    /// The crush flag. It is an int: the stair builder stores 10.
    pub crush: i32,
    pub sector: usize,
    pub direction: i32,
    pub newspecial: i32,
    pub texture: i32,
    pub floordestheight: Fixed,
    pub speed: Fixed,
}

#[derive(Default)]
pub struct State {}

/// T_MovePlane: moves a floor (0) or a ceiling (1) one step and checks the things in the sector.
pub fn t_move_plane(
    d: &mut Doom,
    sector: usize,
    speed: Fixed,
    dest: Fixed,
    crush: i32,
    floor_or_ceiling: i32,
    direction: i32,
) -> i32 {
    let crunch = crush != 0;
    let is_floor = floor_or_ceiling == 0;
    let get = |d: &Doom| {
        if is_floor {
            d.p_setup.sectors[sector].floorheight
        } else {
            d.p_setup.sectors[sector].ceilingheight
        }
    };
    let set = |d: &mut Doom, v: Fixed| {
        if is_floor {
            d.p_setup.sectors[sector].floorheight = v
        } else {
            d.p_setup.sectors[sector].ceilingheight = v
        }
    };
    if direction != -1 && direction != 1 {
        return RESULT_OK;
    }
    let lastpos = get(d);
    let past = if direction == -1 {
        lastpos.wrapping_sub(speed) < dest
    } else {
        lastpos.wrapping_add(speed) > dest
    };
    if past {
        set(d, dest);
        if crate::p_map::change_sector(d, sector, crunch) {
            set(d, lastpos);
            crate::p_map::change_sector(d, sector, crunch);
        }
        return RESULT_PASTDEST;
    }
    set(
        d,
        if direction == -1 {
            lastpos.wrapping_sub(speed)
        } else {
            lastpos.wrapping_add(speed)
        },
    );
    let flag = crate::p_map::change_sector(d, sector, crunch);
    // A rising ceiling never reports a crush.
    if !flag || (!is_floor && direction == 1) {
        return RESULT_OK;
    }
    // A lowering floor always moves back. Other planes move back unless crush equals 1.
    if (is_floor && direction == -1) || crush != 1 {
        set(d, lastpos);
        crate::p_map::change_sector(d, sector, crunch);
    }
    RESULT_CRUSHED
}

fn floor(d: &mut Doom, id: ThinkerId) -> &mut FloorMove {
    match &mut d.p_tick.thinkers[id].data {
        ThinkerData::Floor(x) => x,
        _ => panic!("thinker {id} is not a floor"),
    }
}

/// Adds a floor thinker and links it to its sector.
fn add_floor(d: &mut Doom, f: FloorMove) -> ThinkerId {
    let id = d
        .p_tick
        .add_thinker(Think::MoveFloor, ThinkerData::Floor(f));
    d.p_setup.sectors[f.sector].specialdata = Some(id);
    id
}

/// T_MoveFloor.
pub fn t_move_floor(d: &mut Doom, id: ThinkerId) {
    let f = *floor(d, id);
    let res = t_move_plane(
        d,
        f.sector,
        f.speed,
        f.floordestheight,
        f.crush,
        0,
        f.direction,
    );
    if d.p_tick.leveltime & 7 == 0 {
        sector_sound(d, f.sector, SFX_STNMOV);
    }
    if res == RESULT_PASTDEST {
        let s = &mut d.p_setup.sectors[f.sector];
        s.specialdata = None;
        if (f.direction == 1 && f.type_ == FLOOR_DONUTRAISE)
            || (f.direction == -1 && f.type_ == FLOOR_LOWERANDCHANGE)
        {
            s.special = f.newspecial;
            s.floorpic = f.texture as i16 as i32;
        }
        d.p_tick.remove_thinker(id);
        sector_sound(d, f.sector, SFX_PSTOP);
    }
}

/// EV_DoFloor: starts a floor mover in each free sector with the tag of the line.
pub fn ev_do_floor(d: &mut Doom, line: usize, floortype: i32) -> bool {
    let mut secnum = -1;
    let mut rtn = false;
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
        let mut f = FloorMove {
            type_: floortype,
            sector: sec,
            speed: FLOORSPEED,
            ..Default::default()
        };
        let (fh, ch) = (
            d.p_setup.sectors[sec].floorheight,
            d.p_setup.sectors[sec].ceilingheight,
        );
        match floortype {
            FLOOR_LOWERFLOOR => {
                f.direction = -1;
                f.floordestheight = find_highest_floor_surrounding(d, sec);
            }
            FLOOR_LOWERFLOORTOLOWEST => {
                f.direction = -1;
                f.floordestheight = find_lowest_floor_surrounding(d, sec);
            }
            FLOOR_TURBOLOWER => {
                f.direction = -1;
                f.speed = FLOORSPEED * 4;
                f.floordestheight = find_highest_floor_surrounding(d, sec);
                if d.d_main.gameversion <= crate::d_main::EXE_DOOM_1_2 || f.floordestheight != fh {
                    f.floordestheight += 8 * FRACUNIT;
                }
            }
            FLOOR_RAISEFLOORCRUSH | FLOOR_RAISEFLOOR => {
                if floortype == FLOOR_RAISEFLOORCRUSH {
                    f.crush = 1;
                }
                f.direction = 1;
                f.floordestheight = find_lowest_ceiling_surrounding(d, sec).min(ch);
                if floortype == FLOOR_RAISEFLOORCRUSH {
                    f.floordestheight -= 8 * FRACUNIT;
                }
            }
            FLOOR_RAISEFLOORTURBO | FLOOR_RAISEFLOORTONEAREST => {
                f.direction = 1;
                if floortype == FLOOR_RAISEFLOORTURBO {
                    f.speed = FLOORSPEED * 4;
                }
                f.floordestheight = find_next_highest_floor(d, sec, fh);
            }
            FLOOR_RAISEFLOOR24 | FLOOR_RAISEFLOOR512 | FLOOR_RAISEFLOOR24ANDCHANGE => {
                f.direction = 1;
                f.floordestheight = fh
                    + if floortype == FLOOR_RAISEFLOOR512 {
                        512
                    } else {
                        24
                    } * FRACUNIT;
                if floortype == FLOOR_RAISEFLOOR24ANDCHANGE {
                    let front = d.p_setup.lines[line]
                        .frontsector
                        .expect("line without a front sector");
                    let (pic, special) = (
                        d.p_setup.sectors[front].floorpic,
                        d.p_setup.sectors[front].special,
                    );
                    d.p_setup.sectors[sec].floorpic = pic;
                    d.p_setup.sectors[sec].special = special;
                }
            }
            FLOOR_RAISETOTEXTURE => {
                let mut minsize = i32::MAX;
                f.direction = 1;
                for i in 0..d.p_setup.sectors[sec].lines.len() {
                    if two_sided(d, sec, i) {
                        for side in 0..2 {
                            let t = d.p_setup.sides[get_side(d, sec, i, side)].bottomtexture;
                            if t >= 0 {
                                minsize = minsize.min(d.r_data.textureheight[t as usize]);
                            }
                        }
                    }
                }
                f.floordestheight = fh.wrapping_add(minsize);
            }
            FLOOR_LOWERANDCHANGE => {
                f.direction = -1;
                f.floordestheight = find_lowest_floor_surrounding(d, sec);
                f.texture = d.p_setup.sectors[sec].floorpic;
                for i in 0..d.p_setup.sectors[sec].lines.len() {
                    if !two_sided(d, sec, i) {
                        continue;
                    }
                    let side = if d.p_setup.sides[get_side(d, sec, i, 0)].sector == sec {
                        1
                    } else {
                        0
                    };
                    let o = get_sector(d, sec, i, side);
                    if d.p_setup.sectors[o].floorheight == f.floordestheight {
                        f.texture = d.p_setup.sectors[o].floorpic;
                        f.newspecial = d.p_setup.sectors[o].special;
                        break;
                    }
                }
            }
            _ => {}
        }
        add_floor(d, f);
    }
    rtn
}

/// EV_BuildStairs: raises a chain of sectors with the same floor flat as steps.
pub fn ev_build_stairs(d: &mut Doom, line: usize, type_: i32) -> bool {
    let mut secnum = -1;
    let mut rtn = false;
    let (mut speed, mut stairsize) = (0, 0);
    loop {
        secnum = find_sector_from_line_tag(d, line, secnum);
        if secnum < 0 {
            break;
        }
        let mut sec = secnum as usize;
        if d.p_setup.sectors[sec].specialdata.is_some() {
            continue;
        }
        rtn = true;
        match type_ {
            STAIRS_BUILD8 => {
                speed = FLOORSPEED / 4;
                stairsize = 8 * FRACUNIT;
            }
            STAIRS_TURBO16 => {
                speed = FLOORSPEED * 4;
                stairsize = 16 * FRACUNIT;
            }
            _ => {}
        }
        let mut height = d.p_setup.sectors[sec].floorheight + stairsize;
        let step = |sector, height| FloorMove {
            direction: 1,
            sector,
            speed,
            floordestheight: height,
            type_: FLOOR_LOWERFLOOR,
            crush: STAIRS_UNINITIALIZED_CRUSH_FIELD_VALUE,
            ..Default::default()
        };
        add_floor(d, step(sec, height));
        let texture = d.p_setup.sectors[sec].floorpic;
        loop {
            let mut ok = false;
            for i in 0..d.p_setup.sectors[sec].lines.len() {
                let l = d.p_setup.lines[d.p_setup.sectors[sec].lines[i]];
                if l.flags & crate::p_setup::ML_TWOSIDED == 0 {
                    continue;
                }
                if l.frontsector != Some(secnum as usize) {
                    continue;
                }
                let tsec = l.backsector.expect("two-sided line without a back sector");
                if d.p_setup.sectors[tsec].floorpic != texture {
                    continue;
                }
                height += stairsize;
                if d.p_setup.sectors[tsec].specialdata.is_some() {
                    continue;
                }
                sec = tsec;
                secnum = tsec as i32;
                add_floor(d, step(sec, height));
                ok = true;
                break;
            }
            if !ok {
                break;
            }
        }
    }
    rtn
}

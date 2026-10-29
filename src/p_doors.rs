//! Vertical doors.

use crate::d_items::*;
use crate::m_fixed::{Fixed, FRACUNIT};
use crate::p_floor::{t_move_plane, RESULT_CRUSHED, RESULT_PASTDEST};
use crate::p_mobj::MobjId;
use crate::p_spec::{find_lowest_ceiling_surrounding, find_sector_from_line_tag, sector_sound};
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::s_sound::*;
use crate::Doom;

pub const VLD_NORMAL: i32 = 0;
pub const VLD_CLOSE30THENOPEN: i32 = 1;
pub const VLD_CLOSE: i32 = 2;
pub const VLD_OPEN: i32 = 3;
pub const VLD_RAISEIN5MINS: i32 = 4;
pub const VLD_BLAZERAISE: i32 = 5;
pub const VLD_BLAZEOPEN: i32 = 6;
pub const VLD_BLAZECLOSE: i32 = 7;

pub const VDOORSPEED: Fixed = FRACUNIT * 2;
pub const VDOORWAIT: i32 = 150;

#[derive(Clone, Copy, Default, Debug)]
pub struct VlDoor {
    pub type_: i32,
    pub sector: usize,
    pub topheight: Fixed,
    pub speed: Fixed,
    pub direction: i32,
    pub topwait: i32,
    pub topcountdown: i32,
}

#[derive(Default)]
pub struct State {}

fn door(d: &mut Doom, id: ThinkerId) -> &mut VlDoor {
    match &mut d.p_tick.thinkers[id].data {
        ThinkerData::Door(x) => x,
        _ => panic!("thinker {id} is not a door"),
    }
}

/// Adds a door thinker and links it to its sector.
fn add_door(d: &mut Doom, v: VlDoor) -> ThinkerId {
    let id = d
        .p_tick
        .add_thinker(Think::VerticalDoor, ThinkerData::Door(v));
    d.p_setup.sectors[v.sector].specialdata = Some(id);
    id
}

/// Ends a door: clears the sector link and removes the thinker.
fn finish(d: &mut Doom, id: ThinkerId, sector: usize) {
    d.p_setup.sectors[sector].specialdata = None;
    d.p_tick.remove_thinker(id);
}

/// T_VerticalDoor.
pub fn t_vertical_door(d: &mut Doom, id: ThinkerId) {
    let v = *door(d, id);
    let sec = v.sector;
    match v.direction {
        0 => {
            door(d, id).topcountdown -= 1;
            if v.topcountdown - 1 == 0 {
                match v.type_ {
                    VLD_BLAZERAISE => {
                        door(d, id).direction = -1;
                        sector_sound(d, sec, SFX_BDCLS);
                    }
                    VLD_NORMAL => {
                        door(d, id).direction = -1;
                        sector_sound(d, sec, SFX_DORCLS);
                    }
                    VLD_CLOSE30THENOPEN => {
                        door(d, id).direction = 1;
                        sector_sound(d, sec, SFX_DOROPN);
                    }
                    _ => {}
                }
            }
        }
        2 => {
            door(d, id).topcountdown -= 1;
            if v.topcountdown - 1 == 0 && v.type_ == VLD_RAISEIN5MINS {
                let dr = door(d, id);
                dr.direction = 1;
                dr.type_ = VLD_NORMAL;
                sector_sound(d, sec, SFX_DOROPN);
            }
        }
        -1 => {
            let floor = d.p_setup.sectors[sec].floorheight;
            let res = t_move_plane(d, sec, v.speed, floor, 0, 1, v.direction);
            if res == RESULT_PASTDEST {
                match v.type_ {
                    VLD_BLAZERAISE | VLD_BLAZECLOSE => {
                        finish(d, id, sec);
                        sector_sound(d, sec, SFX_BDCLS);
                    }
                    VLD_NORMAL | VLD_CLOSE => finish(d, id, sec),
                    VLD_CLOSE30THENOPEN => {
                        let dr = door(d, id);
                        dr.direction = 0;
                        dr.topcountdown = 35 * 30;
                    }
                    _ => {}
                }
            } else if res == RESULT_CRUSHED && !matches!(v.type_, VLD_BLAZECLOSE | VLD_CLOSE) {
                door(d, id).direction = 1;
                sector_sound(d, sec, SFX_DOROPN);
            }
        }
        1 => {
            let res = t_move_plane(d, sec, v.speed, v.topheight, 0, 1, v.direction);
            if res == RESULT_PASTDEST {
                match v.type_ {
                    VLD_BLAZERAISE | VLD_NORMAL => {
                        let dr = door(d, id);
                        dr.direction = 0;
                        dr.topcountdown = dr.topwait;
                    }
                    VLD_CLOSE30THENOPEN | VLD_BLAZEOPEN | VLD_OPEN => finish(d, id, sec),
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// EV_DoLockedDoor: opens a locked door when the player has the key.
pub fn ev_do_locked_door(d: &mut Doom, line: usize, type_: i32, thing: MobjId) -> bool {
    let Some(p) = d.p_tick.mobj(thing).player else {
        return false;
    };
    let need = match d.p_setup.lines[line].special {
        99 | 133 => Some((
            IT_BLUECARD,
            IT_BLUESKULL,
            "You need a blue key to activate this object",
        )),
        134 | 135 => Some((
            IT_REDCARD,
            IT_REDSKULL,
            "You need a red key to activate this object",
        )),
        136 | 137 => Some((
            IT_YELLOWCARD,
            IT_YELLOWSKULL,
            "You need a yellow key to activate this object",
        )),
        _ => None,
    };
    if let Some((card, skull, msg)) = need {
        let pl = &mut d.g_game.players[p];
        if !pl.cards[card] && !pl.cards[skull] {
            pl.message = Some(msg.to_string());
            start_sound(d, None, SFX_OOF);
            return false;
        }
    }
    ev_do_door(d, line, type_)
}

/// EV_DoDoor: starts a door in each free sector with the tag of the line.
pub fn ev_do_door(d: &mut Doom, line: usize, type_: i32) -> bool {
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
        let mut v = VlDoor {
            sector: sec,
            type_,
            topwait: VDOORWAIT,
            speed: VDOORSPEED,
            ..Default::default()
        };
        let ceil = d.p_setup.sectors[sec].ceilingheight;
        let sound = match type_ {
            VLD_BLAZECLOSE => {
                v.topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
                v.direction = -1;
                v.speed = VDOORSPEED * 4;
                Some(SFX_BDCLS)
            }
            VLD_CLOSE => {
                v.topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
                v.direction = -1;
                Some(SFX_DORCLS)
            }
            VLD_CLOSE30THENOPEN => {
                v.topheight = ceil;
                v.direction = -1;
                Some(SFX_DORCLS)
            }
            VLD_BLAZERAISE | VLD_BLAZEOPEN => {
                v.direction = 1;
                v.topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
                v.speed = VDOORSPEED * 4;
                (v.topheight != ceil).then_some(SFX_BDOPN)
            }
            VLD_NORMAL | VLD_OPEN => {
                v.direction = 1;
                v.topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
                (v.topheight != ceil).then_some(SFX_DOROPN)
            }
            _ => None,
        };
        add_door(d, v);
        if let Some(s) = sound {
            sector_sound(d, sec, s);
        }
    }
    rtn
}

/// The 32-bit field at the offset of vldoor_t.direction in another thinker type of the arm64 original.
fn alias_direction(data: &mut ThinkerData) -> Option<&mut i32> {
    match data {
        ThinkerData::Door(x) => Some(&mut x.direction),
        ThinkerData::Plat(x) => Some(&mut x.count),
        ThinkerData::Ceiling(x) => Some(&mut x.speed),
        ThinkerData::Floor(x) => Some(&mut x.texture),
        _ => None,
    }
}

/// EV_VerticalDoor: opens or closes the door behind a used line.
pub fn ev_vertical_door(d: &mut Doom, line: usize, thing: MobjId) {
    let player = d.p_tick.mobj(thing).player;
    let special = d.p_setup.lines[line].special;
    let need = match special {
        26 | 32 => Some((
            IT_BLUECARD,
            IT_BLUESKULL,
            "You need a blue key to open this door",
        )),
        27 | 34 => Some((
            IT_YELLOWCARD,
            IT_YELLOWSKULL,
            "You need a yellow key to open this door",
        )),
        28 | 33 => Some((
            IT_REDCARD,
            IT_REDSKULL,
            "You need a red key to open this door",
        )),
        _ => None,
    };
    if let Some((card, skull, msg)) = need {
        let Some(p) = player else { return };
        let pl = &mut d.g_game.players[p];
        if !pl.cards[card] && !pl.cards[skull] {
            pl.message = Some(msg.to_string());
            start_sound(d, None, SFX_OOF);
            return;
        }
    }
    let side1 = d.p_setup.lines[line].sidenum[1];
    let sec = d.p_setup.sides[side1 as usize].sector;
    if let Some(id) = d.p_setup.sectors[sec].specialdata {
        if matches!(special, 1 | 26 | 27 | 28 | 117) {
            let func = d.p_tick.thinkers[id].func;
            let data = &mut d.p_tick.thinkers[id].data;
            let dir = alias_direction(data).map(|x| *x).unwrap_or(0);
            if dir == -1 {
                if let Some(x) = alias_direction(data) {
                    *x = 1;
                }
                return;
            }
            if player.is_none() {
                return;
            }
            if let (Think::PlatRaise, ThinkerData::Plat(p)) = (func, &mut *data) {
                p.wait = -1;
                return;
            }
            if func != Think::VerticalDoor {
                eprintln!("EV_VerticalDoor: Tried to close something that wasn't a door.");
            }
            if let Some(x) = alias_direction(data) {
                *x = -1;
            }
            return;
        }
    }
    let sfx = if matches!(special, 117 | 118) {
        SFX_BDOPN
    } else {
        SFX_DOROPN
    };
    sector_sound(d, sec, sfx);
    let mut v = VlDoor {
        sector: sec,
        direction: 1,
        speed: VDOORSPEED,
        topwait: VDOORWAIT,
        ..Default::default()
    };
    match special {
        1 | 26 | 27 | 28 => v.type_ = VLD_NORMAL,
        31..=34 => {
            v.type_ = VLD_OPEN;
            d.p_setup.lines[line].special = 0;
        }
        117 => {
            v.type_ = VLD_BLAZERAISE;
            v.speed = VDOORSPEED * 4;
        }
        118 => {
            v.type_ = VLD_BLAZEOPEN;
            d.p_setup.lines[line].special = 0;
            v.speed = VDOORSPEED * 4;
        }
        _ => {}
    }
    v.topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
    add_door(d, v);
}

/// P_SpawnDoorCloseIn30.
pub fn spawn_door_close_in30(d: &mut Doom, sec: usize) {
    d.p_setup.sectors[sec].special = 0;
    add_door(
        d,
        VlDoor {
            sector: sec,
            direction: 0,
            type_: VLD_NORMAL,
            speed: VDOORSPEED,
            topcountdown: 30 * 35,
            ..Default::default()
        },
    );
}

/// P_SpawnDoorRaiseIn5Mins.
pub fn spawn_door_raise_in5_mins(d: &mut Doom, sec: usize, _secnum: i32) {
    d.p_setup.sectors[sec].special = 0;
    let topheight = find_lowest_ceiling_surrounding(d, sec) - 4 * FRACUNIT;
    add_door(
        d,
        VlDoor {
            sector: sec,
            direction: 2,
            type_: VLD_RAISEIN5MINS,
            speed: VDOORSPEED,
            topheight,
            topwait: VDOORWAIT,
            topcountdown: 5 * 60 * 35,
        },
    );
}

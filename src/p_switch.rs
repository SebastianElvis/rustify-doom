//! Switch textures, buttons, and the use-line dispatch.

use crate::p_ceilng::*;
use crate::p_doors::*;
use crate::p_floor::*;
use crate::p_mobj::MobjId;
use crate::p_plats::*;
use crate::s_sound::{SFX_SWTCHN, SFX_SWTCHX};
use crate::Doom;

pub const MAXBUTTONS: usize = 16;
pub const BUTTONTIME: i32 = 35;

pub const TOP: i32 = 0;
pub const MIDDLE: i32 = 1;
pub const BOTTOM: i32 = 2;

/// The alphSwitchList table of the original: (name1, name2, episode).
const ALPH_SWITCH_LIST: [(&str, &str, i32); 40] = [
    ("SW1BRCOM", "SW2BRCOM", 1),
    ("SW1BRN1", "SW2BRN1", 1),
    ("SW1BRN2", "SW2BRN2", 1),
    ("SW1BRNGN", "SW2BRNGN", 1),
    ("SW1BROWN", "SW2BROWN", 1),
    ("SW1COMM", "SW2COMM", 1),
    ("SW1COMP", "SW2COMP", 1),
    ("SW1DIRT", "SW2DIRT", 1),
    ("SW1EXIT", "SW2EXIT", 1),
    ("SW1GRAY", "SW2GRAY", 1),
    ("SW1GRAY1", "SW2GRAY1", 1),
    ("SW1METAL", "SW2METAL", 1),
    ("SW1PIPE", "SW2PIPE", 1),
    ("SW1SLAD", "SW2SLAD", 1),
    ("SW1STARG", "SW2STARG", 1),
    ("SW1STON1", "SW2STON1", 1),
    ("SW1STON2", "SW2STON2", 1),
    ("SW1STONE", "SW2STONE", 1),
    ("SW1STRTN", "SW2STRTN", 1),
    ("SW1BLUE", "SW2BLUE", 2),
    ("SW1CMT", "SW2CMT", 2),
    ("SW1GARG", "SW2GARG", 2),
    ("SW1GSTON", "SW2GSTON", 2),
    ("SW1HOT", "SW2HOT", 2),
    ("SW1LION", "SW2LION", 2),
    ("SW1SATYR", "SW2SATYR", 2),
    ("SW1SKIN", "SW2SKIN", 2),
    ("SW1VINE", "SW2VINE", 2),
    ("SW1WOOD", "SW2WOOD", 2),
    ("SW1PANEL", "SW2PANEL", 3),
    ("SW1ROCK", "SW2ROCK", 3),
    ("SW1MET2", "SW2MET2", 3),
    ("SW1WDMET", "SW2WDMET", 3),
    ("SW1BRIK", "SW2BRIK", 3),
    ("SW1MOD1", "SW2MOD1", 3),
    ("SW1ZIM", "SW2ZIM", 3),
    ("SW1STON6", "SW2STON6", 3),
    ("SW1TEK", "SW2TEK", 3),
    ("SW1MARB", "SW2MARB", 3),
    ("SW1SKULL", "SW2SKULL", 3),
];

/// A pressed button that changes its texture back when the timer ends.
#[derive(Clone, Copy, Default, Debug)]
pub struct Button {
    pub line: Option<usize>,
    pub where_: i32,
    pub btexture: i32,
    pub btimer: i32,
    /// The sector whose sound origin the button uses (soundorg).
    pub soundorg: Option<usize>,
}

#[derive(Default)]
pub struct State {
    /// Texture pairs: the switch-off texture at an even index, the switch-on texture after it.
    pub switchlist: Vec<i32>,
    pub numswitches: usize,
    pub buttonlist: [Button; MAXBUTTONS],
}

/// P_InitSwitchList.
pub fn init_switch_list(d: &mut Doom) {
    use crate::d_main::{COMMERCIAL, REGISTERED, RETAIL};
    let mode = d.d_main.gamemode;
    let episode = if mode == REGISTERED || mode == RETAIL {
        2
    } else if mode == COMMERCIAL {
        3
    } else {
        1
    };
    let mut list = Vec::new();
    for &(n1, n2, ep) in ALPH_SWITCH_LIST.iter() {
        if ep <= episode {
            list.push(crate::r_data::texture_num_for_name(d, n1));
            list.push(crate::r_data::texture_num_for_name(d, n2));
        }
    }
    d.p_switch.numswitches = list.len() / 2;
    list.push(-1);
    d.p_switch.switchlist = list;
}

/// P_StartButton: starts a button timer for a line.
pub fn start_button(d: &mut Doom, line: usize, w: i32, texture: i32, time: i32) {
    let bl = &mut d.p_switch.buttonlist;
    if bl.iter().any(|b| b.btimer != 0 && b.line == Some(line)) {
        return;
    }
    if let Some(b) = bl.iter_mut().find(|b| b.btimer == 0) {
        *b = Button {
            line: Some(line),
            where_: w,
            btexture: texture,
            btimer: time,
            soundorg: d.p_setup.lines[line].frontsector,
        };
        return;
    }
    crate::i_system::error(d, "P_StartButton: no button slots left!");
}

/// S_StartSound with the origin of the first button, as the original does.
fn first_button_sound(d: &mut Doom, sfx: i32) {
    match d.p_switch.buttonlist[0].soundorg {
        Some(s) => crate::p_spec::sector_sound(d, s, sfx),
        None => crate::s_sound::start_sound(d, None, sfx),
    }
}

/// P_ChangeSwitchTexture: flips the switch texture of the front side and starts a button when the switch is reusable.
pub fn change_switch_texture(d: &mut Doom, line: usize, use_again: bool) {
    if !use_again {
        d.p_setup.lines[line].special = 0;
    }
    let side = d.p_setup.lines[line].sidenum[0] as usize;
    let s = d.p_setup.sides[side];
    let sound = if d.p_setup.lines[line].special == 11 {
        SFX_SWTCHX
    } else {
        SFX_SWTCHN
    };
    for i in 0..d.p_switch.numswitches * 2 {
        let sw = d.p_switch.switchlist[i];
        let other = d.p_switch.switchlist[i ^ 1];
        let w = if sw == s.toptexture {
            TOP
        } else if sw == s.midtexture {
            MIDDLE
        } else if sw == s.bottomtexture {
            BOTTOM
        } else {
            continue;
        };
        first_button_sound(d, sound);
        let sd = &mut d.p_setup.sides[side];
        match w {
            TOP => sd.toptexture = other,
            MIDDLE => sd.midtexture = other,
            _ => sd.bottomtexture = other,
        }
        if use_again {
            start_button(d, line, w, sw, BUTTONTIME);
        }
        return;
    }
}

/// The button part of P_UpdateSpecials: restores switch textures when their timers end.
pub fn update_buttons(d: &mut Doom) {
    for i in 0..MAXBUTTONS {
        let b = d.p_switch.buttonlist[i];
        if b.btimer == 0 {
            continue;
        }
        d.p_switch.buttonlist[i].btimer -= 1;
        if b.btimer - 1 != 0 {
            continue;
        }
        let side = d.p_setup.lines[b.line.unwrap()].sidenum[0] as usize;
        let sd = &mut d.p_setup.sides[side];
        match b.where_ {
            TOP => sd.toptexture = b.btexture,
            MIDDLE => sd.midtexture = b.btexture,
            BOTTOM => sd.bottomtexture = b.btexture,
            _ => {}
        }
        match b.soundorg {
            Some(s) => crate::p_spec::sector_sound(d, s, SFX_SWTCHN),
            None => crate::s_sound::start_sound(d, None, SFX_SWTCHN),
        }
        d.p_switch.buttonlist[i] = Button::default();
    }
}

/// P_UseSpecialLine: runs the use trigger of a line. It returns false when the line cannot be used.
pub fn use_special_line(d: &mut Doom, thing: MobjId, line: usize, side: i32) -> bool {
    let special = d.p_setup.lines[line].special;
    if side != 0 && special != 124 {
        return false;
    }
    if d.p_tick.mobj(thing).player.is_none() {
        if d.p_setup.lines[line].flags & crate::p_setup::ML_SECRET != 0 {
            return false;
        }
        if !matches!(special, 1 | 32 | 33 | 34) {
            return false;
        }
    }
    let sw = |d: &mut Doom, ok: bool| {
        if ok {
            change_switch_texture(d, line, false);
        }
    };
    let bt = |d: &mut Doom, ok: bool| {
        if ok {
            change_switch_texture(d, line, true);
        }
    };
    match special {
        // Manual doors.
        1 | 26 | 27 | 28 | 31 | 32 | 33 | 34 | 117 | 118 => ev_vertical_door(d, line, thing),
        // Switches.
        7 => {
            let r = ev_build_stairs(d, line, STAIRS_BUILD8);
            sw(d, r)
        }
        9 => {
            let r = crate::p_spec::ev_do_donut(d, line);
            sw(d, r)
        }
        11 => {
            change_switch_texture(d, line, false);
            crate::g_game::exit_level(d);
        }
        14 => {
            let r = ev_do_plat(d, line, PLAT_RAISEANDCHANGE, 32);
            sw(d, r)
        }
        15 => {
            let r = ev_do_plat(d, line, PLAT_RAISEANDCHANGE, 24);
            sw(d, r)
        }
        18 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORTONEAREST);
            sw(d, r)
        }
        20 => {
            let r = ev_do_plat(d, line, PLAT_RAISETONEARESTANDCHANGE, 0);
            sw(d, r)
        }
        21 => {
            let r = ev_do_plat(d, line, PLAT_DOWNWAITUPSTAY, 0);
            sw(d, r)
        }
        23 => {
            let r = ev_do_floor(d, line, FLOOR_LOWERFLOORTOLOWEST);
            sw(d, r)
        }
        29 => {
            let r = ev_do_door(d, line, VLD_NORMAL);
            sw(d, r)
        }
        41 => {
            let r = ev_do_ceiling(d, line, CEILING_LOWERTOFLOOR);
            sw(d, r)
        }
        71 => {
            let r = ev_do_floor(d, line, FLOOR_TURBOLOWER);
            sw(d, r)
        }
        49 => {
            let r = ev_do_ceiling(d, line, CEILING_CRUSHANDRAISE);
            sw(d, r)
        }
        50 => {
            let r = ev_do_door(d, line, VLD_CLOSE);
            sw(d, r)
        }
        51 => {
            change_switch_texture(d, line, false);
            crate::g_game::secret_exit_level(d);
        }
        55 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORCRUSH);
            sw(d, r)
        }
        101 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOOR);
            sw(d, r)
        }
        102 => {
            let r = ev_do_floor(d, line, FLOOR_LOWERFLOOR);
            sw(d, r)
        }
        103 => {
            let r = ev_do_door(d, line, VLD_OPEN);
            sw(d, r)
        }
        111 => {
            let r = ev_do_door(d, line, VLD_BLAZERAISE);
            sw(d, r)
        }
        112 => {
            let r = ev_do_door(d, line, VLD_BLAZEOPEN);
            sw(d, r)
        }
        113 => {
            let r = ev_do_door(d, line, VLD_BLAZECLOSE);
            sw(d, r)
        }
        122 => {
            let r = ev_do_plat(d, line, PLAT_BLAZEDWUS, 0);
            sw(d, r)
        }
        127 => {
            let r = ev_build_stairs(d, line, STAIRS_TURBO16);
            sw(d, r)
        }
        131 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORTURBO);
            sw(d, r)
        }
        133 | 135 | 137 => {
            let r = ev_do_locked_door(d, line, VLD_BLAZEOPEN, thing);
            sw(d, r)
        }
        140 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOOR512);
            sw(d, r)
        }
        // Buttons.
        42 => {
            let r = ev_do_door(d, line, VLD_CLOSE);
            bt(d, r)
        }
        43 => {
            let r = ev_do_ceiling(d, line, CEILING_LOWERTOFLOOR);
            bt(d, r)
        }
        45 => {
            let r = ev_do_floor(d, line, FLOOR_LOWERFLOOR);
            bt(d, r)
        }
        60 => {
            let r = ev_do_floor(d, line, FLOOR_LOWERFLOORTOLOWEST);
            bt(d, r)
        }
        61 => {
            let r = ev_do_door(d, line, VLD_OPEN);
            bt(d, r)
        }
        62 => {
            let r = ev_do_plat(d, line, PLAT_DOWNWAITUPSTAY, 1);
            bt(d, r)
        }
        63 => {
            let r = ev_do_door(d, line, VLD_NORMAL);
            bt(d, r)
        }
        64 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOOR);
            bt(d, r)
        }
        66 => {
            let r = ev_do_plat(d, line, PLAT_RAISEANDCHANGE, 24);
            bt(d, r)
        }
        67 => {
            let r = ev_do_plat(d, line, PLAT_RAISEANDCHANGE, 32);
            bt(d, r)
        }
        65 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORCRUSH);
            bt(d, r)
        }
        68 => {
            let r = ev_do_plat(d, line, PLAT_RAISETONEARESTANDCHANGE, 0);
            bt(d, r)
        }
        69 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORTONEAREST);
            bt(d, r)
        }
        70 => {
            let r = ev_do_floor(d, line, FLOOR_TURBOLOWER);
            bt(d, r)
        }
        114 => {
            let r = ev_do_door(d, line, VLD_BLAZERAISE);
            bt(d, r)
        }
        115 => {
            let r = ev_do_door(d, line, VLD_BLAZEOPEN);
            bt(d, r)
        }
        116 => {
            let r = ev_do_door(d, line, VLD_BLAZECLOSE);
            bt(d, r)
        }
        123 => {
            let r = ev_do_plat(d, line, PLAT_BLAZEDWUS, 0);
            bt(d, r)
        }
        132 => {
            let r = ev_do_floor(d, line, FLOOR_RAISEFLOORTURBO);
            bt(d, r)
        }
        99 | 134 | 136 => {
            let r = ev_do_locked_door(d, line, VLD_BLAZEOPEN, thing);
            bt(d, r)
        }
        138 => {
            crate::p_lights::ev_light_turn_on(d, line, 255);
            change_switch_texture(d, line, true);
        }
        139 => {
            crate::p_lights::ev_light_turn_on(d, line, 35);
            change_switch_texture(d, line, true);
        }
        _ => {}
    }
    true
}

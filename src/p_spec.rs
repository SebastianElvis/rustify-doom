//! Sector and line specials: animated flats and textures, sector helpers, line triggers, and donuts.

use crate::m_fixed::{Fixed, FRACUNIT};
use crate::m_random::p_random;
use crate::p_ceilng::*;
use crate::p_doors::*;
use crate::p_floor::*;
use crate::p_mobj::MobjId;
use crate::p_plats::*;
use crate::p_setup::ML_TWOSIDED;
use crate::p_tick::{Think, ThinkerData};
use crate::Doom;

pub const MAXANIMS: usize = 32;
pub const MAXLINEANIMS: usize = 64;
const DONUT_FLOORHEIGHT_DEFAULT: i32 = 0;
const DONUT_FLOORPIC_DEFAULT: i32 = 0x16;

#[derive(Clone, Copy, Default, Debug)]
pub struct Anim {
    pub istexture: bool,
    pub picnum: i32,
    pub basepic: i32,
    pub numpics: i32,
    pub speed: i32,
}

/// The animdefs table of the original: (istexture, endname, startname, speed).
const ANIMDEFS: [(bool, &str, &str, i32); 22] = [
    (false, "NUKAGE3", "NUKAGE1", 8),
    (false, "FWATER4", "FWATER1", 8),
    (false, "SWATER4", "SWATER1", 8),
    (false, "LAVA4", "LAVA1", 8),
    (false, "BLOOD3", "BLOOD1", 8),
    (false, "RROCK08", "RROCK05", 8),
    (false, "SLIME04", "SLIME01", 8),
    (false, "SLIME08", "SLIME05", 8),
    (false, "SLIME12", "SLIME09", 8),
    (true, "BLODGR4", "BLODGR1", 8),
    (true, "SLADRIP3", "SLADRIP1", 8),
    (true, "BLODRIP4", "BLODRIP1", 8),
    (true, "FIREWALL", "FIREWALA", 8),
    (true, "GSTFONT3", "GSTFONT1", 8),
    (true, "FIRELAVA", "FIRELAV3", 8),
    (true, "FIREMAG3", "FIREMAG1", 8),
    (true, "FIREBLU2", "FIREBLU1", 8),
    (true, "ROCKRED3", "ROCKRED1", 8),
    (true, "BFALL4", "BFALL1", 8),
    (true, "SFALL4", "SFALL1", 8),
    (true, "WFALL4", "WFALL1", 8),
    (true, "DBRAIN4", "DBRAIN1", 8),
];

#[derive(Default)]
pub struct State {
    pub anims: Vec<Anim>,
    pub level_timer: bool,
    pub level_time_count: i32,
    /// The -timer minute limit for deathmatch (timelimit). The startup code sets it.
    pub timelimit: i32,
    pub linespeciallist: Vec<usize>,
    /// DonutOverrun keeps its values after the first overrun.
    pub donut_done: bool,
    pub tmp_s3_floorheight: i32,
    pub tmp_s3_floorpic: i32,
}

/// S_StartSound with the sound origin of a sector.
pub fn sector_sound(d: &mut Doom, sector: usize, sfx: i32) {
    crate::s_sound::start_sector_sound(d, sector, sfx);
}

/// P_InitPicAnims.
pub fn init_pic_anims(d: &mut Doom) {
    d.p_spec.anims.clear();
    for &(istexture, endname, startname, speed) in ANIMDEFS.iter() {
        let (picnum, basepic);
        if istexture {
            if crate::r_data::check_texture_num_for_name(d, startname) == -1 {
                continue;
            }
            picnum = crate::r_data::texture_num_for_name(d, endname);
            basepic = crate::r_data::texture_num_for_name(d, startname);
        } else {
            if crate::w_wad::check_num_for_name(&d.w_wad, startname) == -1 {
                continue;
            }
            picnum = crate::r_data::flat_num_for_name(d, endname);
            basepic = crate::r_data::flat_num_for_name(d, startname);
        }
        let numpics = picnum - basepic + 1;
        if numpics < 2 {
            crate::i_system::error(
                d,
                &format!(
                    "P_InitPicAnims: bad cycle from {} to {}",
                    startname, endname
                ),
            );
        }
        d.p_spec.anims.push(Anim {
            istexture,
            picnum,
            basepic,
            numpics,
            speed,
        });
    }
}

/// getSide: the side `side` of line `line` of sector `sec`.
pub fn get_side(d: &Doom, sec: usize, line: usize, side: usize) -> usize {
    let l = d.p_setup.sectors[sec].lines[line];
    d.p_setup.lines[l].sidenum[side] as usize
}

/// getSector: the sector of side `side` of line `line` of sector `sec`.
pub fn get_sector(d: &Doom, sec: usize, line: usize, side: usize) -> usize {
    d.p_setup.sides[get_side(d, sec, line, side)].sector
}

/// twoSided.
pub fn two_sided(d: &Doom, sec: usize, line: usize) -> bool {
    let l = d.p_setup.sectors[sec].lines[line];
    d.p_setup.lines[l].flags & ML_TWOSIDED != 0
}

/// getNextSector: the sector on the other side of a two-sided line.
pub fn get_next_sector(d: &Doom, line: usize, sec: usize) -> Option<usize> {
    let l = &d.p_setup.lines[line];
    if l.flags & ML_TWOSIDED == 0 {
        return None;
    }
    if l.frontsector == Some(sec) {
        return l.backsector;
    }
    l.frontsector
}

/// Applies `f` to each neighbour sector of `sec` in line order.
fn for_each_neighbour(d: &Doom, sec: usize, mut f: impl FnMut(usize)) {
    for &l in &d.p_setup.sectors[sec].lines {
        if let Some(o) = get_next_sector(d, l, sec) {
            f(o);
        }
    }
}

/// P_FindLowestFloorSurrounding.
pub fn find_lowest_floor_surrounding(d: &Doom, sec: usize) -> Fixed {
    let mut floor = d.p_setup.sectors[sec].floorheight;
    for_each_neighbour(d, sec, |o| {
        floor = floor.min(d.p_setup.sectors[o].floorheight)
    });
    floor
}

/// P_FindHighestFloorSurrounding.
pub fn find_highest_floor_surrounding(d: &Doom, sec: usize) -> Fixed {
    let mut floor = -500 * FRACUNIT;
    for_each_neighbour(d, sec, |o| {
        floor = floor.max(d.p_setup.sectors[o].floorheight)
    });
    floor
}

/// P_FindNextHighestFloor. It emulates the 20-entry stack array of the original.
pub fn find_next_highest_floor(d: &mut Doom, sec: usize, currentheight: Fixed) -> Fixed {
    const MAX_ADJOINING_SECTORS: usize = 20;
    let mut height = currentheight;
    let mut list: Vec<Fixed> = Vec::new();
    for i in 0..d.p_setup.sectors[sec].lines.len() {
        let l = d.p_setup.sectors[sec].lines[i];
        let Some(o) = get_next_sector(d, l, sec) else {
            continue;
        };
        let fh = d.p_setup.sectors[o].floorheight;
        if fh > height {
            if list.len() == MAX_ADJOINING_SECTORS + 1 {
                height = fh;
            } else if list.len() == MAX_ADJOINING_SECTORS + 2 {
                crate::i_system::error(
                    d,
                    "Sector with more than 22 adjoining sectors. Vanilla will crash here",
                );
            }
            list.push(fh);
        }
    }
    list.iter().copied().min().unwrap_or(currentheight)
}

/// P_FindLowestCeilingSurrounding.
pub fn find_lowest_ceiling_surrounding(d: &Doom, sec: usize) -> Fixed {
    let mut height = i32::MAX;
    for_each_neighbour(d, sec, |o| {
        height = height.min(d.p_setup.sectors[o].ceilingheight)
    });
    height
}

/// P_FindHighestCeilingSurrounding.
pub fn find_highest_ceiling_surrounding(d: &Doom, sec: usize) -> Fixed {
    let mut height = 0;
    for_each_neighbour(d, sec, |o| {
        height = height.max(d.p_setup.sectors[o].ceilingheight)
    });
    height
}

/// P_FindSectorFromLineTag: the next sector after `start` with the tag of the line, or -1.
pub fn find_sector_from_line_tag(d: &Doom, line: usize, start: i32) -> i32 {
    let tag = d.p_setup.lines[line].tag;
    let n = d.p_setup.numsectors as i32;
    ((start + 1)..n)
        .find(|&i| d.p_setup.sectors[i as usize].tag == tag)
        .unwrap_or(-1)
}

/// P_FindMinSurroundingLight.
pub fn find_min_surrounding_light(d: &Doom, sector: usize, max: i32) -> i32 {
    let mut min = max;
    for_each_neighbour(d, sector, |o| {
        min = min.min(d.p_setup.sectors[o].lightlevel)
    });
    min
}

/// P_CrossSpecialLine: runs the walk-over trigger of a line.
pub fn cross_special_line(d: &mut Doom, linenum: usize, side: i32, thing: MobjId) {
    let line = linenum;
    let (is_player, ttype) = {
        let m = d.p_tick.mobj(thing);
        (m.player.is_some(), m.type_)
    };
    use crate::info::*;
    if !is_player {
        if matches!(
            ttype,
            MT_ROCKET | MT_PLASMA | MT_BFG | MT_TROOPSHOT | MT_HEADSHOT | MT_BRUISERSHOT
        ) {
            return;
        }
        if !matches!(
            d.p_setup.lines[line].special,
            39 | 97 | 125 | 126 | 4 | 10 | 88
        ) {
            return;
        }
    }
    let special = d.p_setup.lines[line].special;
    let once = |d: &mut Doom| d.p_setup.lines[line].special = 0;
    match special {
        // Triggers.
        2 => {
            ev_do_door(d, line, VLD_OPEN);
            once(d);
        }
        3 => {
            ev_do_door(d, line, VLD_CLOSE);
            once(d);
        }
        4 => {
            ev_do_door(d, line, VLD_NORMAL);
            once(d);
        }
        5 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR);
            once(d);
        }
        6 => {
            ev_do_ceiling(d, line, CEILING_FASTCRUSHANDRAISE);
            once(d);
        }
        8 => {
            ev_build_stairs(d, line, STAIRS_BUILD8);
            once(d);
        }
        10 => {
            ev_do_plat(d, line, PLAT_DOWNWAITUPSTAY, 0);
            once(d);
        }
        12 => {
            crate::p_lights::ev_light_turn_on(d, line, 0);
            once(d);
        }
        13 => {
            crate::p_lights::ev_light_turn_on(d, line, 255);
            once(d);
        }
        16 => {
            ev_do_door(d, line, VLD_CLOSE30THENOPEN);
            once(d);
        }
        17 => {
            crate::p_lights::ev_start_light_strobing(d, line);
            once(d);
        }
        19 => {
            ev_do_floor(d, line, FLOOR_LOWERFLOOR);
            once(d);
        }
        22 => {
            ev_do_plat(d, line, PLAT_RAISETONEARESTANDCHANGE, 0);
            once(d);
        }
        25 => {
            ev_do_ceiling(d, line, CEILING_CRUSHANDRAISE);
            once(d);
        }
        30 => {
            ev_do_floor(d, line, FLOOR_RAISETOTEXTURE);
            once(d);
        }
        35 => {
            crate::p_lights::ev_light_turn_on(d, line, 35);
            once(d);
        }
        36 => {
            ev_do_floor(d, line, FLOOR_TURBOLOWER);
            once(d);
        }
        37 => {
            ev_do_floor(d, line, FLOOR_LOWERANDCHANGE);
            once(d);
        }
        38 => {
            ev_do_floor(d, line, FLOOR_LOWERFLOORTOLOWEST);
            once(d);
        }
        39 => {
            crate::p_telept::ev_teleport(d, line, side, thing);
            once(d);
        }
        40 => {
            ev_do_ceiling(d, line, CEILING_RAISETOHIGHEST);
            ev_do_floor(d, line, FLOOR_LOWERFLOORTOLOWEST);
            once(d);
        }
        44 => {
            ev_do_ceiling(d, line, CEILING_LOWERANDCRUSH);
            once(d);
        }
        52 => crate::g_game::exit_level(d),
        53 => {
            ev_do_plat(d, line, PLAT_PERPETUALRAISE, 0);
            once(d);
        }
        54 => {
            ev_stop_plat(d, line);
            once(d);
        }
        56 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORCRUSH);
            once(d);
        }
        57 => {
            ev_ceiling_crush_stop(d, line);
            once(d);
        }
        58 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR24);
            once(d);
        }
        59 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR24ANDCHANGE);
            once(d);
        }
        104 => {
            crate::p_lights::ev_turn_tag_lights_off(d, line);
            once(d);
        }
        108 => {
            ev_do_door(d, line, VLD_BLAZERAISE);
            once(d);
        }
        109 => {
            ev_do_door(d, line, VLD_BLAZEOPEN);
            once(d);
        }
        100 => {
            ev_build_stairs(d, line, STAIRS_TURBO16);
            once(d);
        }
        110 => {
            ev_do_door(d, line, VLD_BLAZECLOSE);
            once(d);
        }
        119 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORTONEAREST);
            once(d);
        }
        121 => {
            ev_do_plat(d, line, PLAT_BLAZEDWUS, 0);
            once(d);
        }
        124 => crate::g_game::secret_exit_level(d),
        125 => {
            if !is_player {
                crate::p_telept::ev_teleport(d, line, side, thing);
                once(d);
            }
        }
        130 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORTURBO);
            once(d);
        }
        141 => {
            ev_do_ceiling(d, line, CEILING_SILENTCRUSHANDRAISE);
            once(d);
        }
        // Retriggers.
        72 => {
            ev_do_ceiling(d, line, CEILING_LOWERANDCRUSH);
        }
        73 => {
            ev_do_ceiling(d, line, CEILING_CRUSHANDRAISE);
        }
        74 => {
            ev_ceiling_crush_stop(d, line);
        }
        75 => {
            ev_do_door(d, line, VLD_CLOSE);
        }
        76 => {
            ev_do_door(d, line, VLD_CLOSE30THENOPEN);
        }
        77 => {
            ev_do_ceiling(d, line, CEILING_FASTCRUSHANDRAISE);
        }
        79 => crate::p_lights::ev_light_turn_on(d, line, 35),
        80 => crate::p_lights::ev_light_turn_on(d, line, 0),
        81 => crate::p_lights::ev_light_turn_on(d, line, 255),
        82 => {
            ev_do_floor(d, line, FLOOR_LOWERFLOORTOLOWEST);
        }
        83 => {
            ev_do_floor(d, line, FLOOR_LOWERFLOOR);
        }
        84 => {
            ev_do_floor(d, line, FLOOR_LOWERANDCHANGE);
        }
        86 => {
            ev_do_door(d, line, VLD_OPEN);
        }
        87 => {
            ev_do_plat(d, line, PLAT_PERPETUALRAISE, 0);
        }
        88 => {
            ev_do_plat(d, line, PLAT_DOWNWAITUPSTAY, 0);
        }
        89 => ev_stop_plat(d, line),
        90 => {
            ev_do_door(d, line, VLD_NORMAL);
        }
        91 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR);
        }
        92 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR24);
        }
        93 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR24ANDCHANGE);
        }
        94 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORCRUSH);
        }
        95 => {
            ev_do_plat(d, line, PLAT_RAISETONEARESTANDCHANGE, 0);
        }
        96 => {
            ev_do_floor(d, line, FLOOR_RAISETOTEXTURE);
        }
        97 => {
            crate::p_telept::ev_teleport(d, line, side, thing);
        }
        98 => {
            ev_do_floor(d, line, FLOOR_TURBOLOWER);
        }
        105 => {
            ev_do_door(d, line, VLD_BLAZERAISE);
        }
        106 => {
            ev_do_door(d, line, VLD_BLAZEOPEN);
        }
        107 => {
            ev_do_door(d, line, VLD_BLAZECLOSE);
        }
        120 => {
            ev_do_plat(d, line, PLAT_BLAZEDWUS, 0);
        }
        126 => {
            if !is_player {
                crate::p_telept::ev_teleport(d, line, side, thing);
            }
        }
        128 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORTONEAREST);
        }
        129 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOORTURBO);
        }
        _ => {}
    }
}

/// P_ShootSpecialLine: runs the gunshot trigger of a line.
pub fn shoot_special_line(d: &mut Doom, thing: MobjId, line: usize) {
    let special = d.p_setup.lines[line].special;
    if d.p_tick.mobj(thing).player.is_none() && special != 46 {
        return;
    }
    match special {
        24 => {
            ev_do_floor(d, line, FLOOR_RAISEFLOOR);
            crate::p_switch::change_switch_texture(d, line, false);
        }
        46 => {
            ev_do_door(d, line, VLD_OPEN);
            crate::p_switch::change_switch_texture(d, line, true);
        }
        47 => {
            ev_do_plat(d, line, PLAT_RAISETONEARESTANDCHANGE, 0);
            crate::p_switch::change_switch_texture(d, line, false);
        }
        _ => {}
    }
}

/// P_PlayerInSpecialSector: applies the damage, secret, and exit effects of the floor under a player.
pub fn player_in_special_sector(d: &mut Doom, player: usize) {
    use crate::d_items::PW_IRONFEET;
    let mo = d.g_game.players[player]
        .mo
        .expect("player without a map object");
    let (z, ss) = {
        let m = d.p_tick.mobj(mo);
        (m.z, m.subsector)
    };
    let sector = d.p_setup.subsectors[ss].sector;
    if z != d.p_setup.sectors[sector].floorheight {
        return;
    }
    let leveltime = d.p_tick.leveltime;
    let ironfeet = d.g_game.players[player].powers[PW_IRONFEET] != 0;
    let special = d.p_setup.sectors[sector].special;
    match special {
        5 => {
            if !ironfeet && leveltime & 0x1f == 0 {
                crate::p_inter::damage_mobj(d, mo, None, None, 10);
            }
        }
        7 => {
            if !ironfeet && leveltime & 0x1f == 0 {
                crate::p_inter::damage_mobj(d, mo, None, None, 5);
            }
        }
        16 | 4 => {
            if (!ironfeet || p_random(&mut d.m_random) < 5) && leveltime & 0x1f == 0 {
                crate::p_inter::damage_mobj(d, mo, None, None, 20);
            }
        }
        9 => {
            d.g_game.players[player].secretcount += 1;
            d.p_setup.sectors[sector].special = 0;
        }
        11 => {
            d.g_game.players[player].cheats &= !crate::g_game::CF_GODMODE;
            if leveltime & 0x1f == 0 {
                crate::p_inter::damage_mobj(d, mo, None, None, 20);
            }
            if d.g_game.players[player].health <= 10 {
                crate::g_game::exit_level(d);
            }
        }
        _ => crate::i_system::error(
            d,
            &format!("P_PlayerInSpecialSector: unknown special {}", special),
        ),
    }
}

/// P_UpdateSpecials: the level timer, the animations, the scrolling walls, and the button timers.
pub fn update_specials(d: &mut Doom) {
    if d.p_spec.level_timer {
        d.p_spec.level_time_count -= 1;
        if d.p_spec.level_time_count == 0 {
            crate::g_game::exit_level(d);
        }
    }
    let leveltime = d.p_tick.leveltime;
    for a in d.p_spec.anims.iter() {
        for i in a.basepic..a.basepic + a.numpics {
            let pic = a.basepic + (leveltime / a.speed + i) % a.numpics;
            if a.istexture {
                d.r_data.texturetranslation[i as usize] = pic;
            } else {
                d.r_data.flattranslation[i as usize] = pic;
            }
        }
    }
    for &l in &d.p_spec.linespeciallist {
        if d.p_setup.lines[l].special == 48 {
            let s = d.p_setup.lines[l].sidenum[0] as usize;
            d.p_setup.sides[s].textureoffset += FRACUNIT;
        }
    }
    crate::p_switch::update_buttons(d);
}

/// P_SpawnSpecials: starts the sector effects and collects the scrolling walls after a level load.
pub fn spawn_specials(d: &mut Doom) {
    if d.p_spec.timelimit > 0 && d.g_game.deathmatch != 0 {
        d.p_spec.level_timer = true;
        d.p_spec.level_time_count = d.p_spec.timelimit * 60 * 35;
    } else {
        d.p_spec.level_timer = false;
    }
    use crate::p_lights::*;
    for s in 0..d.p_setup.numsectors {
        match d.p_setup.sectors[s].special {
            1 => spawn_light_flash(d, s),
            2 => spawn_strobe_flash(d, s, FASTDARK, false),
            3 => spawn_strobe_flash(d, s, SLOWDARK, false),
            4 => {
                spawn_strobe_flash(d, s, FASTDARK, false);
                d.p_setup.sectors[s].special = 4;
            }
            8 => spawn_glowing_light(d, s),
            9 => d.g_game.totalsecret += 1,
            10 => spawn_door_close_in30(d, s),
            12 => spawn_strobe_flash(d, s, SLOWDARK, true),
            13 => spawn_strobe_flash(d, s, FASTDARK, true),
            14 => spawn_door_raise_in5_mins(d, s, s as i32),
            17 => spawn_fire_flicker(d, s),
            _ => {}
        }
    }
    d.p_spec.linespeciallist.clear();
    for i in 0..d.p_setup.lines.len() {
        if d.p_setup.lines[i].special == 48 {
            if d.p_spec.linespeciallist.len() >= MAXLINEANIMS {
                crate::i_system::error(
                    d,
                    "Too many scrolling wall linedefs! (Vanilla limit is 64)",
                );
            }
            d.p_spec.linespeciallist.push(i);
        }
    }
    d.p_ceilng.activeceilings = [None; MAXCEILINGS];
    d.p_plats.activeplats = [None; MAXPLATS];
    d.p_switch.buttonlist = Default::default();
}

/// DonutOverrun: returns the floor height and the floor picture that the original reads from address 0.
fn donut_overrun(d: &mut Doom) -> (Fixed, i32) {
    if !d.p_spec.donut_done {
        d.p_spec.donut_done = true;
        d.p_spec.tmp_s3_floorheight = DONUT_FLOORHEIGHT_DEFAULT;
        d.p_spec.tmp_s3_floorpic = DONUT_FLOORPIC_DEFAULT;
        let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-donut", 2);
        if p > 0 {
            let a = d.m_argv.myargv[p + 1].clone();
            let b = d.m_argv.myargv[p + 2].clone();
            if let Some(v) = crate::m_misc::str_to_int(&a) {
                d.p_spec.tmp_s3_floorheight = v;
            }
            if let Some(v) = crate::m_misc::str_to_int(&b) {
                d.p_spec.tmp_s3_floorpic = v;
            }
            if d.p_spec.tmp_s3_floorpic >= d.r_data.numflats {
                eprintln!(
                    "DonutOverrun: The second parameter for \"-donut\" switch should be greater than 0 and less than number of flats ({}). Using default value ({}) instead. ",
                    d.r_data.numflats, DONUT_FLOORPIC_DEFAULT
                );
                d.p_spec.tmp_s3_floorpic = DONUT_FLOORPIC_DEFAULT;
            }
        }
    }
    (
        d.p_spec.tmp_s3_floorheight,
        d.p_spec.tmp_s3_floorpic as i16 as i32,
    )
}

/// EV_DoDonut: lowers the donut hole and raises the ring to the outer floor.
pub fn ev_do_donut(d: &mut Doom, line: usize) -> bool {
    let mut secnum = -1;
    let mut rtn = false;
    loop {
        secnum = find_sector_from_line_tag(d, line, secnum);
        if secnum < 0 {
            break;
        }
        let s1 = secnum as usize;
        if d.p_setup.sectors[s1].specialdata.is_some() {
            continue;
        }
        rtn = true;
        let l0 = d.p_setup.sectors[s1].lines[0];
        let Some(s2) = get_next_sector(d, l0, s1) else {
            eprintln!("EV_DoDonut: linedef had no second sidedef! Unexpected behavior may occur in Vanilla Doom. ");
            break;
        };
        for i in 0..d.p_setup.sectors[s2].lines.len() {
            let l = d.p_setup.sectors[s2].lines[i];
            let s3 = d.p_setup.lines[l].backsector;
            if s3 == Some(s1) {
                continue;
            }
            let (s3_floorheight, s3_floorpic) = match s3 {
                None => {
                    eprintln!("EV_DoDonut: WARNING: emulating buffer overrun due to NULL back sector. Unexpected behavior may occur in Vanilla Doom.");
                    donut_overrun(d)
                }
                Some(s3) => (
                    d.p_setup.sectors[s3].floorheight,
                    d.p_setup.sectors[s3].floorpic,
                ),
            };
            let rise = FloorMove {
                type_: FLOOR_DONUTRAISE,
                crush: 0,
                direction: 1,
                sector: s2,
                speed: FRACUNIT / 2,
                texture: s3_floorpic,
                newspecial: 0,
                floordestheight: s3_floorheight,
            };
            let id = d
                .p_tick
                .add_thinker(Think::MoveFloor, ThinkerData::Floor(rise));
            d.p_setup.sectors[s2].specialdata = Some(id);
            let lower = FloorMove {
                type_: FLOOR_LOWERFLOOR,
                crush: 0,
                direction: -1,
                sector: s1,
                speed: FRACUNIT / 2,
                floordestheight: s3_floorheight,
                ..Default::default()
            };
            let id = d
                .p_tick
                .add_thinker(Think::MoveFloor, ThinkerData::Floor(lower));
            d.p_setup.sectors[s1].specialdata = Some(id);
            break;
        }
    }
    rtn
}

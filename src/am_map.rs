//! The automap: map view, follow mode, grid, marks, zoom, pan, and the map cheat.

use crate::d_main::{Event, EV_KEYDOWN, EV_KEYUP, EXE_DOOM_1_8};
use crate::g_game::MAXPLAYERS;
use crate::i_video::{KEY_DOWNARROW, KEY_LEFTARROW, KEY_RIGHTARROW, KEY_TAB, KEY_UPARROW};
use crate::m_cheat::{check_cheat, CheatSeq};
use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::p_setup::{ML_DONTDRAW, ML_MAPPED, ML_SECRET};
use crate::tables::{finecosine, ANGLETOFINESHIFT, FINESINE};
use crate::Doom;

const AM_MSGHEADER: i32 = ((b'a' as i32) << 24) + ((b'm' as i32) << 16);
const AM_MSGENTERED: i32 = AM_MSGHEADER | ((b'e' as i32) << 8);
const AM_MSGEXITED: i32 = AM_MSGHEADER | ((b'x' as i32) << 8);

const KEY_MAP_NORTH: i32 = KEY_UPARROW;
const KEY_MAP_SOUTH: i32 = KEY_DOWNARROW;
const KEY_MAP_EAST: i32 = KEY_RIGHTARROW;
const KEY_MAP_WEST: i32 = KEY_LEFTARROW;
const KEY_MAP_ZOOMIN: i32 = b'=' as i32;
const KEY_MAP_ZOOMOUT: i32 = b'-' as i32;
const KEY_MAP_TOGGLE: i32 = KEY_TAB;
const KEY_MAP_MAXZOOM: i32 = b'0' as i32;
const KEY_MAP_FOLLOW: i32 = b'f' as i32;
const KEY_MAP_GRID: i32 = b'g' as i32;
const KEY_MAP_MARK: i32 = b'm' as i32;
const KEY_MAP_CLEARMARK: i32 = b'c' as i32;

const REDS: i32 = 256 - 5 * 16;
const REDRANGE: i32 = 16;
const GREENS: i32 = 7 * 16;
const GRAYS: i32 = 6 * 16;
const GRAYSRANGE: i32 = 16;
const BROWNS: i32 = 4 * 16;
const YELLOWS: i32 = 256 - 32 + 7;
const WHITE: i32 = 256 - 47;
const BACKGROUND: i32 = 0;
const WALLCOLORS: i32 = REDS;
const WALLRANGE: i32 = REDRANGE;
const TSWALLCOLORS: i32 = GRAYS;
const FDWALLCOLORS: i32 = BROWNS;
const CDWALLCOLORS: i32 = YELLOWS;
const THINGCOLORS: i32 = GREENS;
const SECRETWALLCOLORS: i32 = WALLCOLORS;
const GRIDCOLORS: i32 = GRAYS + GRAYSRANGE / 2;
const XHAIRCOLORS: i32 = GRAYS;

const AM_NUMMARKPOINTS: usize = 10;
const INITSCALEMTOF: Fixed = (0.2 * FRACUNIT as f64) as Fixed;
const F_PANINC: i32 = 4;
const M_ZOOMIN: Fixed = (1.02 * FRACUNIT as f64) as Fixed;
const M_ZOOMOUT: Fixed = (FRACUNIT as f64 / 1.02) as Fixed;
const PLAYERRADIUS: Fixed = 16 * FRACUNIT;
const MAPBLOCKUNITS: Fixed = 128;
const PW_INVISIBILITY: usize = 2;
const PW_ALLMAP: usize = 4;

type MLine = ((Fixed, Fixed), (Fixed, Fixed));

const R: Fixed = (8 * PLAYERRADIUS) / 7;
const PLAYER_ARROW: [MLine; 7] = [
    ((-R + R / 8, 0), (R, 0)),
    ((R, 0), (R - R / 2, R / 4)),
    ((R, 0), (R - R / 2, -R / 4)),
    ((-R + R / 8, 0), (-R - R / 8, R / 4)),
    ((-R + R / 8, 0), (-R - R / 8, -R / 4)),
    ((-R + 3 * R / 8, 0), (-R + R / 8, R / 4)),
    ((-R + 3 * R / 8, 0), (-R + R / 8, -R / 4)),
];
const CHEAT_PLAYER_ARROW: [MLine; 16] = [
    ((-R + R / 8, 0), (R, 0)),
    ((R, 0), (R - R / 2, R / 6)),
    ((R, 0), (R - R / 2, -R / 6)),
    ((-R + R / 8, 0), (-R - R / 8, R / 6)),
    ((-R + R / 8, 0), (-R - R / 8, -R / 6)),
    ((-R + 3 * R / 8, 0), (-R + R / 8, R / 6)),
    ((-R + 3 * R / 8, 0), (-R + R / 8, -R / 6)),
    ((-R / 2, 0), (-R / 2, -R / 6)),
    ((-R / 2, -R / 6), (-R / 2 + R / 6, -R / 6)),
    ((-R / 2 + R / 6, -R / 6), (-R / 2 + R / 6, R / 4)),
    ((-R / 6, 0), (-R / 6, -R / 6)),
    ((-R / 6, -R / 6), (0, -R / 6)),
    ((0, -R / 6), (0, R / 4)),
    ((R / 6, R / 4), (R / 6, -R / 7)),
    ((R / 6, -R / 7), (R / 6 + R / 32, -R / 7 - R / 32)),
    ((R / 6 + R / 32, -R / 7 - R / 32), (R / 6 + R / 10, -R / 7)),
];
const THINTRIANGLE_GUY: [MLine; 3] = [
    ((-32768, -45875), (FRACUNIT, 0)),
    ((FRACUNIT, 0), (-32768, 45875)),
    ((-32768, 45875), (-32768, -45875)),
];

/// The automap globals.
pub struct State {
    pub automapactive: bool,
    cheating: i32,
    grid: bool,
    f_x: i32,
    f_y: i32,
    f_w: i32,
    f_h: i32,
    lightlev: i32,
    amclock: i32,
    m_paninc: (Fixed, Fixed),
    mtof_zoommul: Fixed,
    ftom_zoommul: Fixed,
    m_x: Fixed,
    m_y: Fixed,
    m_x2: Fixed,
    m_y2: Fixed,
    m_w: Fixed,
    m_h: Fixed,
    min_x: Fixed,
    min_y: Fixed,
    max_x: Fixed,
    max_y: Fixed,
    min_scale_mtof: Fixed,
    max_scale_mtof: Fixed,
    old_m_w: Fixed,
    old_m_h: Fixed,
    old_m_x: Fixed,
    old_m_y: Fixed,
    f_oldloc: (Fixed, Fixed),
    scale_mtof: Fixed,
    scale_ftom: Fixed,
    plr: usize,
    marknums: [usize; 10],
    markpoints: [(Fixed, Fixed); AM_NUMMARKPOINTS],
    markpointnum: usize,
    followplayer: bool,
    cheat_amap: CheatSeq,
    stopped: bool,
    bigstate: bool,
    lastlevel: i32,
    lastepisode: i32,
    fuck: i32,
}

impl Default for State {
    fn default() -> Self {
        State {
            automapactive: false,
            cheating: 0,
            grid: false,
            f_x: 0,
            f_y: 0,
            f_w: 0,
            f_h: 0,
            lightlev: 0,
            amclock: 0,
            m_paninc: (0, 0),
            mtof_zoommul: 0,
            ftom_zoommul: 0,
            m_x: 0,
            m_y: 0,
            m_x2: 0,
            m_y2: 0,
            m_w: 0,
            m_h: 0,
            min_x: 0,
            min_y: 0,
            max_x: 0,
            max_y: 0,
            min_scale_mtof: 0,
            max_scale_mtof: 0,
            old_m_w: 0,
            old_m_h: 0,
            old_m_x: 0,
            old_m_y: 0,
            f_oldloc: (0, 0),
            scale_mtof: INITSCALEMTOF,
            scale_ftom: 0,
            plr: 0,
            marknums: [0; 10],
            markpoints: [(0, 0); AM_NUMMARKPOINTS],
            markpointnum: 0,
            followplayer: true,
            cheat_amap: CheatSeq::new("iddt", 0),
            stopped: true,
            bigstate: false,
            lastlevel: -1,
            lastepisode: -1,
            fuck: 0,
        }
    }
}

impl State {
    fn ftom(&self, x: i32) -> Fixed {
        fixed_mul(x << FRACBITS, self.scale_ftom)
    }
    fn mtof(&self, x: Fixed) -> i32 {
        fixed_mul(x, self.scale_mtof) >> FRACBITS
    }
    fn cxmtof(&self, x: Fixed) -> i32 {
        self.f_x + self.mtof(x.wrapping_sub(self.m_x))
    }
    fn cymtof(&self, y: Fixed) -> i32 {
        self.f_y + (self.f_h - self.mtof(y.wrapping_sub(self.m_y)))
    }

    /// AM_activateNewScale.
    fn activate_new_scale(&mut self) {
        self.m_x += self.m_w / 2;
        self.m_y += self.m_h / 2;
        self.m_w = self.ftom(self.f_w);
        self.m_h = self.ftom(self.f_h);
        self.m_x -= self.m_w / 2;
        self.m_y -= self.m_h / 2;
        self.m_x2 = self.m_x + self.m_w;
        self.m_y2 = self.m_y + self.m_h;
    }

    /// AM_saveScaleAndLoc.
    fn save_scale_and_loc(&mut self) {
        self.old_m_x = self.m_x;
        self.old_m_y = self.m_y;
        self.old_m_w = self.m_w;
        self.old_m_h = self.m_h;
    }

    /// AM_addMark.
    fn add_mark(&mut self) {
        self.markpoints[self.markpointnum] = (self.m_x + self.m_w / 2, self.m_y + self.m_h / 2);
        self.markpointnum = (self.markpointnum + 1) % AM_NUMMARKPOINTS;
    }

    /// AM_changeWindowLoc.
    fn change_window_loc(&mut self) {
        if self.m_paninc.0 != 0 || self.m_paninc.1 != 0 {
            self.followplayer = false;
            self.f_oldloc.0 = i32::MAX;
        }
        self.m_x += self.m_paninc.0;
        self.m_y += self.m_paninc.1;
        if self.m_x + self.m_w / 2 > self.max_x {
            self.m_x = self.max_x - self.m_w / 2;
        } else if self.m_x + self.m_w / 2 < self.min_x {
            self.m_x = self.min_x - self.m_w / 2;
        }
        if self.m_y + self.m_h / 2 > self.max_y {
            self.m_y = self.max_y - self.m_h / 2;
        } else if self.m_y + self.m_h / 2 < self.min_y {
            self.m_y = self.min_y - self.m_h / 2;
        }
        self.m_x2 = self.m_x + self.m_w;
        self.m_y2 = self.m_y + self.m_h;
    }

    /// AM_clearMarks.
    fn clear_marks(&mut self) {
        for m in self.markpoints.iter_mut() {
            m.0 = -1;
        }
        self.markpointnum = 0;
    }

    /// AM_minOutWindowScale.
    fn min_out_window_scale(&mut self) {
        self.scale_mtof = self.min_scale_mtof;
        self.scale_ftom = fixed_div(FRACUNIT, self.scale_mtof);
        self.activate_new_scale();
    }

    /// AM_maxOutWindowScale.
    fn max_out_window_scale(&mut self) {
        self.scale_mtof = self.max_scale_mtof;
        self.scale_ftom = fixed_div(FRACUNIT, self.scale_mtof);
        self.activate_new_scale();
    }

    /// AM_changeWindowScale.
    fn change_window_scale(&mut self) {
        self.scale_mtof = fixed_mul(self.scale_mtof, self.mtof_zoommul);
        self.scale_ftom = fixed_div(FRACUNIT, self.scale_mtof);
        if self.scale_mtof < self.min_scale_mtof {
            self.min_out_window_scale();
        } else if self.scale_mtof > self.max_scale_mtof {
            self.max_out_window_scale();
        } else {
            self.activate_new_scale();
        }
    }
}

fn plr_mo(d: &Doom, p: usize) -> &crate::p_mobj::Mobj {
    d.p_tick
        .mobj(d.g_game.players[p].mo.expect("player without a map object"))
}

/// AM_restoreScaleAndLoc.
fn restore_scale_and_loc(d: &mut Doom) {
    let (px, py) = {
        let mo = plr_mo(d, d.am_map.plr);
        (mo.x, mo.y)
    };
    let a = &mut d.am_map;
    a.m_w = a.old_m_w;
    a.m_h = a.old_m_h;
    if !a.followplayer {
        a.m_x = a.old_m_x;
        a.m_y = a.old_m_y;
    } else {
        a.m_x = px - a.m_w / 2;
        a.m_y = py - a.m_h / 2;
    }
    a.m_x2 = a.m_x + a.m_w;
    a.m_y2 = a.m_y + a.m_h;
    a.scale_mtof = fixed_div(a.f_w << FRACBITS, a.m_w);
    a.scale_ftom = fixed_div(FRACUNIT, a.scale_mtof);
}

/// AM_findMinMaxBoundaries.
fn find_min_max_boundaries(d: &mut Doom) {
    let a = &mut d.am_map;
    a.min_x = i32::MAX;
    a.min_y = i32::MAX;
    a.max_x = -i32::MAX;
    a.max_y = -i32::MAX;
    for v in d.p_setup.vertexes.iter() {
        if v.x < a.min_x {
            a.min_x = v.x;
        } else if v.x > a.max_x {
            a.max_x = v.x;
        }
        if v.y < a.min_y {
            a.min_y = v.y;
        } else if v.y > a.max_y {
            a.max_y = v.y;
        }
    }
    let max_w = a.max_x.wrapping_sub(a.min_x);
    let max_h = a.max_y.wrapping_sub(a.min_y);
    let ma = fixed_div(a.f_w << FRACBITS, max_w);
    let mb = fixed_div(a.f_h << FRACBITS, max_h);
    a.min_scale_mtof = if ma < mb { ma } else { mb };
    a.max_scale_mtof = fixed_div(a.f_h << FRACBITS, 2 * PLAYERRADIUS);
}

/// AM_initVariables.
fn init_variables(d: &mut Doom) {
    d.am_map.automapactive = true;
    let a = &mut d.am_map;
    a.f_oldloc.0 = i32::MAX;
    a.amclock = 0;
    a.lightlev = 0;
    a.m_paninc = (0, 0);
    a.ftom_zoommul = FRACUNIT;
    a.mtof_zoommul = FRACUNIT;
    a.m_w = a.ftom(a.f_w);
    a.m_h = a.ftom(a.f_h);
    let cp = d.g_game.consoleplayer;
    a.plr = if d.g_game.playeringame[cp] {
        cp
    } else {
        (0..MAXPLAYERS)
            .find(|&p| d.g_game.playeringame[p])
            .unwrap_or(0)
    };
    let (px, py) = {
        let mo = plr_mo(d, d.am_map.plr);
        (mo.x, mo.y)
    };
    let a = &mut d.am_map;
    a.m_x = px - a.m_w / 2;
    a.m_y = py - a.m_h / 2;
    a.change_window_loc();
    a.save_scale_and_loc();
    let st_notify = Event {
        type_: EV_KEYUP,
        data1: AM_MSGENTERED,
        data2: 0,
        data3: 0,
        data4: 0,
    };
    crate::st_stuff::responder(d, &st_notify);
}

/// AM_loadPics.
fn load_pics(d: &mut Doom) {
    for i in 0..10 {
        d.am_map.marknums[i] = crate::w_wad::get_num_for_name(d, &format!("AMMNUM{}", i));
    }
}

/// AM_LevelInit.
fn level_init(d: &mut Doom) {
    let a = &mut d.am_map;
    a.f_x = 0;
    a.f_y = 0;
    a.f_w = 320;
    a.f_h = 200 - 32;
    a.clear_marks();
    find_min_max_boundaries(d);
    let a = &mut d.am_map;
    a.scale_mtof = fixed_div(a.min_scale_mtof, (0.7 * FRACUNIT as f64) as i32);
    if a.scale_mtof > a.max_scale_mtof {
        a.scale_mtof = a.min_scale_mtof;
    }
    a.scale_ftom = fixed_div(FRACUNIT, a.scale_mtof);
}

/// AM_Stop.
pub fn stop(d: &mut Doom) {
    d.am_map.automapactive = false;
    // The original event has its fields in the wrong order: a key-down event with data2 = AM_MSGEXITED.
    let st_notify = Event {
        type_: 0,
        data1: EV_KEYUP,
        data2: AM_MSGEXITED,
        data3: 0,
        data4: 0,
    };
    crate::st_stuff::responder(d, &st_notify);
    d.am_map.stopped = true;
}

/// AM_Start.
pub fn start(d: &mut Doom) {
    if !d.am_map.stopped {
        stop(d);
    }
    d.am_map.stopped = false;
    if d.am_map.lastlevel != d.g_game.gamemap || d.am_map.lastepisode != d.g_game.gameepisode {
        level_init(d);
        d.am_map.lastlevel = d.g_game.gamemap;
        d.am_map.lastepisode = d.g_game.gameepisode;
    }
    init_variables(d);
    load_pics(d);
}

fn set_message(d: &mut Doom, msg: String) {
    let p = d.am_map.plr;
    d.g_game.players[p].message = Some(msg);
}

/// AM_Responder.
pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    let mut rc = false;
    if !d.am_map.automapactive {
        if ev.type_ == EV_KEYDOWN && ev.data1 == KEY_MAP_TOGGLE {
            start(d);
            d.g_game.viewactive = false;
            rc = true;
        }
    } else if ev.type_ == EV_KEYDOWN {
        rc = true;
        let key = ev.data1;
        let a = &mut d.am_map;
        let pan = a.ftom(F_PANINC);
        if key == KEY_MAP_EAST {
            if !a.followplayer {
                a.m_paninc.0 = pan
            } else {
                rc = false
            }
        } else if key == KEY_MAP_WEST {
            if !a.followplayer {
                a.m_paninc.0 = -pan
            } else {
                rc = false
            }
        } else if key == KEY_MAP_NORTH {
            if !a.followplayer {
                a.m_paninc.1 = pan
            } else {
                rc = false
            }
        } else if key == KEY_MAP_SOUTH {
            if !a.followplayer {
                a.m_paninc.1 = -pan
            } else {
                rc = false
            }
        } else if key == KEY_MAP_ZOOMOUT {
            a.mtof_zoommul = M_ZOOMOUT;
            a.ftom_zoommul = M_ZOOMIN;
        } else if key == KEY_MAP_ZOOMIN {
            a.mtof_zoommul = M_ZOOMIN;
            a.ftom_zoommul = M_ZOOMOUT;
        } else if key == KEY_MAP_TOGGLE {
            a.bigstate = false;
            d.g_game.viewactive = true;
            stop(d);
        } else if key == KEY_MAP_MAXZOOM {
            a.bigstate = !a.bigstate;
            if a.bigstate {
                a.save_scale_and_loc();
                a.min_out_window_scale();
            } else {
                restore_scale_and_loc(d);
            }
        } else if key == KEY_MAP_FOLLOW {
            a.followplayer = !a.followplayer;
            a.f_oldloc.0 = i32::MAX;
            let m = if a.followplayer {
                "Follow Mode ON"
            } else {
                "Follow Mode OFF"
            };
            set_message(d, m.to_string());
        } else if key == KEY_MAP_GRID {
            a.grid = !a.grid;
            let m = if a.grid { "Grid ON" } else { "Grid OFF" };
            set_message(d, m.to_string());
        } else if key == KEY_MAP_MARK {
            let m = format!("{} {}", "Marked Spot", a.markpointnum);
            a.add_mark();
            set_message(d, m);
        } else if key == KEY_MAP_CLEARMARK {
            a.clear_marks();
            set_message(d, "All Marks Cleared".to_string());
        } else {
            rc = false;
        }
        if (d.g_game.deathmatch == 0 || d.d_main.gameversion <= EXE_DOOM_1_8)
            && check_cheat(&mut d.am_map.cheat_amap, ev.data2 as u8)
        {
            rc = false;
            d.am_map.cheating = (d.am_map.cheating + 1) % 3;
        }
    } else if ev.type_ == EV_KEYUP {
        rc = false;
        let key = ev.data1;
        let a = &mut d.am_map;
        if key == KEY_MAP_EAST || key == KEY_MAP_WEST {
            if !a.followplayer {
                a.m_paninc.0 = 0;
            }
        } else if key == KEY_MAP_NORTH || key == KEY_MAP_SOUTH {
            if !a.followplayer {
                a.m_paninc.1 = 0;
            }
        } else if key == KEY_MAP_ZOOMOUT || key == KEY_MAP_ZOOMIN {
            a.mtof_zoommul = FRACUNIT;
            a.ftom_zoommul = FRACUNIT;
        }
    }
    rc
}

/// AM_doFollowPlayer.
fn do_follow_player(d: &mut Doom) {
    let (px, py) = {
        let mo = plr_mo(d, d.am_map.plr);
        (mo.x, mo.y)
    };
    let a = &mut d.am_map;
    if a.f_oldloc.0 != px || a.f_oldloc.1 != py {
        a.m_x = a.ftom(a.mtof(px)) - a.m_w / 2;
        a.m_y = a.ftom(a.mtof(py)) - a.m_h / 2;
        a.m_x2 = a.m_x + a.m_w;
        a.m_y2 = a.m_y + a.m_h;
        a.f_oldloc = (px, py);
    }
}

/// AM_Ticker.
pub fn ticker(d: &mut Doom) {
    if !d.am_map.automapactive {
        return;
    }
    d.am_map.amclock += 1;
    if d.am_map.followplayer {
        do_follow_player(d);
    }
    let a = &mut d.am_map;
    if a.ftom_zoommul != FRACUNIT {
        a.change_window_scale();
    }
    if a.m_paninc.0 != 0 || a.m_paninc.1 != 0 {
        a.change_window_loc();
    }
}

/// AM_clipMline: clips a map line to the window. Returns the frame line when a part is visible.
fn clip_mline(a: &State, ml: &MLine) -> Option<((i32, i32), (i32, i32))> {
    const LEFT: i32 = 1;
    const RIGHT: i32 = 2;
    const BOTTOM: i32 = 4;
    const TOP: i32 = 8;
    let outcode = |x: i32, y: i32| {
        let mut oc = 0;
        if y < 0 {
            oc |= TOP;
        } else if y >= a.f_h {
            oc |= BOTTOM;
        }
        if x < 0 {
            oc |= LEFT;
        } else if x >= a.f_w {
            oc |= RIGHT;
        }
        oc
    };
    let (ma, mb) = *ml;
    let mut oc1 = 0;
    let mut oc2 = 0;
    if ma.1 > a.m_y2 {
        oc1 = TOP;
    } else if ma.1 < a.m_y {
        oc1 = BOTTOM;
    }
    if mb.1 > a.m_y2 {
        oc2 = TOP;
    } else if mb.1 < a.m_y {
        oc2 = BOTTOM;
    }
    if oc1 & oc2 != 0 {
        return None;
    }
    if ma.0 < a.m_x {
        oc1 |= LEFT;
    } else if ma.0 > a.m_x2 {
        oc1 |= RIGHT;
    }
    if mb.0 < a.m_x {
        oc2 |= LEFT;
    } else if mb.0 > a.m_x2 {
        oc2 |= RIGHT;
    }
    if oc1 & oc2 != 0 {
        return None;
    }
    let mut fa = (a.cxmtof(ma.0), a.cymtof(ma.1));
    let mut fb = (a.cxmtof(mb.0), a.cymtof(mb.1));
    oc1 = outcode(fa.0, fa.1);
    oc2 = outcode(fb.0, fb.1);
    if oc1 & oc2 != 0 {
        return None;
    }
    while oc1 | oc2 != 0 {
        let outside = if oc1 != 0 { oc1 } else { oc2 };
        let tmp = if outside & TOP != 0 {
            let dy = fa.1 - fb.1;
            let dx = fb.0 - fa.0;
            (fa.0 + (dx.wrapping_mul(fa.1)) / dy, 0)
        } else if outside & BOTTOM != 0 {
            let dy = fa.1 - fb.1;
            let dx = fb.0 - fa.0;
            (fa.0 + (dx.wrapping_mul(fa.1 - a.f_h)) / dy, a.f_h - 1)
        } else if outside & RIGHT != 0 {
            let dy = fb.1 - fa.1;
            let dx = fb.0 - fa.0;
            (a.f_w - 1, fa.1 + (dy.wrapping_mul(a.f_w - 1 - fa.0)) / dx)
        } else if outside & LEFT != 0 {
            let dy = fb.1 - fa.1;
            let dx = fb.0 - fa.0;
            (0, fa.1 + (dy.wrapping_mul(-fa.0)) / dx)
        } else {
            (0, 0)
        };
        if outside == oc1 {
            fa = tmp;
            oc1 = outcode(fa.0, fa.1);
        } else {
            fb = tmp;
            oc2 = outcode(fb.0, fb.1);
        }
        if oc1 & oc2 != 0 {
            return None;
        }
    }
    Some((fa, fb))
}

/// AM_drawFline: Bresenham line in frame coordinates.
fn draw_fline(d: &mut Doom, fl: ((i32, i32), (i32, i32)), color: i32) {
    let a = &mut d.am_map;
    let ((ax0, ay0), (bx, by)) = fl;
    if ax0 < 0
        || ax0 >= a.f_w
        || ay0 < 0
        || ay0 >= a.f_h
        || bx < 0
        || bx >= a.f_w
        || by < 0
        || by >= a.f_h
    {
        eprint!("fuck {} \r", a.fuck);
        a.fuck += 1;
        return;
    }
    let fw = a.f_w as usize;
    let fb = &mut d.i_video.video_buffer;
    let dx = bx - ax0;
    let ax = 2 * dx.abs();
    let sx = if dx < 0 { -1 } else { 1 };
    let dy = by - ay0;
    let ay = 2 * dy.abs();
    let sy = if dy < 0 { -1 } else { 1 };
    let (mut x, mut y) = (ax0, ay0);
    if ax > ay {
        let mut dd = ay - ax / 2;
        loop {
            fb[y as usize * fw + x as usize] = color as u8;
            if x == bx {
                return;
            }
            if dd >= 0 {
                y += sy;
                dd -= ax;
            }
            x += sx;
            dd += ay;
        }
    } else {
        let mut dd = ax - ay / 2;
        loop {
            fb[y as usize * fw + x as usize] = color as u8;
            if y == by {
                return;
            }
            if dd >= 0 {
                x += sx;
                dd -= ay;
            }
            y += sy;
            dd += ax;
        }
    }
}

/// AM_drawMline.
fn draw_mline(d: &mut Doom, ml: &MLine, color: i32) {
    if let Some(fl) = clip_mline(&d.am_map, ml) {
        draw_fline(d, fl, color);
    }
}

/// AM_drawGrid: draws the blockmap grid lines.
fn draw_grid(d: &mut Doom, color: i32) {
    let unit = MAPBLOCKUNITS << FRACBITS;
    let (m_x, m_y, m_w, m_h) = (d.am_map.m_x, d.am_map.m_y, d.am_map.m_w, d.am_map.m_h);
    let (orgx, orgy) = (d.p_setup.bmaporgx, d.p_setup.bmaporgy);
    let mut start = m_x;
    if (start - orgx) % unit != 0 {
        start += unit - ((start - orgx) % unit);
    }
    let end = m_x + m_w;
    let mut x = start;
    while x < end {
        draw_mline(d, &((x, m_y), (x, m_y + m_h)), color);
        x += unit;
    }
    let mut start = m_y;
    if (start - orgy) % unit != 0 {
        start += unit - ((start - orgy) % unit);
    }
    let end = m_y + m_h;
    let mut y = start;
    while y < end {
        draw_mline(d, &((m_x, y), (m_x + m_w, y)), color);
        y += unit;
    }
}

/// AM_drawWalls: draws the seen lines with the colors of their kind.
fn draw_walls(d: &mut Doom) {
    let cheating = d.am_map.cheating;
    let lightlev = d.am_map.lightlev;
    let allmap = d.g_game.players[d.am_map.plr].powers[PW_ALLMAP] != 0;
    for i in 0..d.p_setup.lines.len() {
        let line = &d.p_setup.lines[i];
        let v1 = &d.p_setup.vertexes[line.v1];
        let v2 = &d.p_setup.vertexes[line.v2];
        let l = ((v1.x, v1.y), (v2.x, v2.y));
        let flags = line.flags;
        let color = if cheating != 0 || flags & ML_MAPPED != 0 {
            if flags & ML_DONTDRAW != 0 && cheating == 0 {
                continue;
            }
            match (line.backsector, line.frontsector) {
                (None, _) => Some(WALLCOLORS + lightlev),
                (Some(back), front) => {
                    let back = &d.p_setup.sectors[back];
                    let front = &d.p_setup.sectors[front.unwrap_or(0)];
                    if line.special == 39 {
                        Some(WALLCOLORS + WALLRANGE / 2)
                    } else if flags & ML_SECRET != 0 {
                        Some(if cheating != 0 {
                            SECRETWALLCOLORS + lightlev
                        } else {
                            WALLCOLORS + lightlev
                        })
                    } else if back.floorheight != front.floorheight {
                        Some(FDWALLCOLORS + lightlev)
                    } else if back.ceilingheight != front.ceilingheight {
                        Some(CDWALLCOLORS + lightlev)
                    } else if cheating != 0 {
                        Some(TSWALLCOLORS + lightlev)
                    } else {
                        None
                    }
                }
            }
        } else if allmap && flags & ML_DONTDRAW == 0 {
            Some(GRAYS + 3)
        } else {
            None
        };
        if let Some(c) = color {
            draw_mline(d, &l, c);
        }
    }
}

/// AM_rotate.
fn rotate(x: Fixed, y: Fixed, a: u32) -> (Fixed, Fixed) {
    let i = (a >> ANGLETOFINESHIFT) as usize;
    let tmpx = fixed_mul(x, finecosine(i)).wrapping_sub(fixed_mul(y, FINESINE[i]));
    let ny = fixed_mul(x, FINESINE[i]).wrapping_add(fixed_mul(y, finecosine(i)));
    (tmpx, ny)
}

/// AM_drawLineCharacter.
fn draw_line_character(
    d: &mut Doom,
    lineguy: &[MLine],
    scale: Fixed,
    angle: u32,
    color: i32,
    x: Fixed,
    y: Fixed,
) {
    let tf = |p: (Fixed, Fixed)| {
        let mut p = p;
        if scale != 0 {
            p = (fixed_mul(scale, p.0), fixed_mul(scale, p.1));
        }
        if angle != 0 {
            p = rotate(p.0, p.1, angle);
        }
        (p.0.wrapping_add(x), p.1.wrapping_add(y))
    };
    for l in lineguy {
        let ml = (tf(l.0), tf(l.1));
        draw_mline(d, &ml, color);
    }
}

/// AM_drawPlayers.
fn draw_players(d: &mut Doom) {
    const THEIR_COLORS: [i32; 4] = [GREENS, GRAYS, BROWNS, REDS];
    let plr = d.am_map.plr;
    if !d.g_game.netgame {
        let (angle, x, y) = {
            let mo = plr_mo(d, plr);
            (mo.angle, mo.x, mo.y)
        };
        if d.am_map.cheating != 0 {
            draw_line_character(d, &CHEAT_PLAYER_ARROW, 0, angle, WHITE, x, y);
        } else {
            draw_line_character(d, &PLAYER_ARROW, 0, angle, WHITE, x, y);
        }
        return;
    }
    for i in 0..MAXPLAYERS {
        if (d.g_game.deathmatch != 0 && !d.g_game.singledemo) && i != plr {
            continue;
        }
        if !d.g_game.playeringame[i] {
            continue;
        }
        let color = if d.g_game.players[i].powers[PW_INVISIBILITY] != 0 {
            246
        } else {
            THEIR_COLORS[i]
        };
        let (angle, x, y) = {
            let mo = plr_mo(d, i);
            (mo.angle, mo.x, mo.y)
        };
        draw_line_character(d, &PLAYER_ARROW, 0, angle, color, x, y);
    }
}

/// AM_drawThings: draws every thing as a small triangle.
fn draw_things(d: &mut Doom, colors: i32) {
    let lightlev = d.am_map.lightlev;
    for i in 0..d.p_setup.numsectors {
        let mut t = d.p_setup.sectors[i].thinglist;
        while let Some(id) = t {
            let (angle, x, y, next) = {
                let mo = d.p_tick.mobj(id);
                (mo.angle, mo.x, mo.y, mo.snext)
            };
            draw_line_character(
                d,
                &THINTRIANGLE_GUY,
                16 << FRACBITS,
                angle,
                colors + lightlev,
                x,
                y,
            );
            t = next;
        }
    }
}

/// AM_drawMarks.
fn draw_marks(d: &mut Doom) {
    for i in 0..AM_NUMMARKPOINTS {
        let a = &d.am_map;
        let (mx, my) = a.markpoints[i];
        if mx == -1 {
            continue;
        }
        let (w, h) = (5, 6);
        let fx = a.cxmtof(mx);
        let fy = a.cymtof(my);
        if fx >= a.f_x && fx <= a.f_w - w && fy >= a.f_y && fy <= a.f_h - h {
            let p = a.marknums[i];
            crate::v_video::draw_patch(d, fx, fy, p);
        }
    }
}

/// AM_Drawer.
pub fn drawer(d: &mut Doom) {
    if !d.am_map.automapactive {
        return;
    }
    let n = (d.am_map.f_w * d.am_map.f_h) as usize;
    d.i_video.video_buffer[..n].fill(BACKGROUND as u8);
    if d.am_map.grid {
        draw_grid(d, GRIDCOLORS);
    }
    draw_walls(d);
    draw_players(d);
    if d.am_map.cheating == 2 {
        draw_things(d, THINGCOLORS);
    }
    let (f_w, f_h) = (d.am_map.f_w, d.am_map.f_h);
    d.i_video.video_buffer[((f_w * (f_h + 1)) / 2) as usize] = XHAIRCOLORS as u8;
    draw_marks(d);
    let (f_x, f_y) = (d.am_map.f_x, d.am_map.f_y);
    crate::v_video::mark_rect(d, f_x, f_y, f_w, f_h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrow_constants_match_fixed_point_values() {
        assert_eq!(R, 1198372);
        assert_eq!(INITSCALEMTOF, 0x3333);
        assert_eq!(M_ZOOMIN, 66846);
        assert_eq!(M_ZOOMOUT, 64250);
    }

    #[test]
    fn exit_notify_keeps_original_field_order() {
        assert_eq!(AM_MSGENTERED, 0x616d_6500);
        assert_eq!(AM_MSGEXITED, 0x616d_7800);
    }
}

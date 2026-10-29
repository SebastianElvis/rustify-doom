//! Status bar widgets: numbers, percentages, multiple-state icons, and binary icons.
//! The caller passes the current value and the `on` flag of each widget.

use crate::hu_lib::patch_header;
use crate::Doom;

pub const ST_Y: i32 = 168;

#[derive(Default)]
pub struct State {
    pub sttminus: Option<usize>,
}

#[derive(Clone, Default)]
pub struct StNumber {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub oldnum: i32,
    pub p: Vec<usize>,
}

#[derive(Clone, Default)]
pub struct StPercent {
    pub n: StNumber,
    pub p: usize,
}

#[derive(Clone, Default)]
pub struct StMultIcon {
    pub x: i32,
    pub y: i32,
    pub oldinum: i32,
    pub p: Vec<usize>,
}

#[derive(Clone, Default)]
pub struct StBinIcon {
    pub x: i32,
    pub y: i32,
    pub oldval: bool,
    pub p: usize,
}

/// STlib_init.
pub fn init(d: &mut Doom) {
    let n = crate::w_wad::check_num_for_name(&d.w_wad, "STTMINUS");
    d.st_lib.sttminus = if n >= 0 { Some(n as usize) } else { None };
}

/// Copies a rectangle of the status bar background to the screen.
fn restore_background(d: &mut Doom, x: i32, srcy: i32, w: i32, h: i32, desty: i32) {
    let backing = std::mem::take(&mut d.st_stuff.st_backing_screen);
    crate::v_video::copy_rect(d, x, srcy, &backing, w, h, x, desty);
    d.st_stuff.st_backing_screen = backing;
}

pub fn init_num(x: i32, y: i32, pl: &[usize], width: i32) -> StNumber {
    StNumber {
        x,
        y,
        width,
        oldnum: 0,
        p: pl.to_vec(),
    }
}

fn draw_num(d: &mut Doom, n: &mut StNumber, value: i32) {
    let mut numdigits = n.width;
    let mut num = value;
    let (w, h, _, _) = patch_header(d, n.p[0]);
    n.oldnum = value;
    let neg = num < 0;
    if neg {
        if numdigits == 2 && num < -9 {
            num = -9;
        } else if numdigits == 3 && num < -99 {
            num = -99;
        }
        num = -num;
    }
    let mut x = n.x - numdigits * w;
    if n.y - ST_Y < 0 {
        crate::i_system::error(d, "drawNum: n->y - ST_Y < 0");
    }
    restore_background(d, x, n.y - ST_Y, w * numdigits, h, n.y);
    if num == 1994 {
        return;
    }
    x = n.x;
    if num == 0 {
        crate::v_video::draw_patch(d, x - w, n.y, n.p[0]);
    }
    while num != 0 && numdigits != 0 {
        numdigits -= 1;
        x -= w;
        crate::v_video::draw_patch(d, x, n.y, n.p[(num % 10) as usize]);
        num /= 10;
    }
    if neg {
        if let Some(m) = d.st_lib.sttminus {
            crate::v_video::draw_patch(d, x - 8, n.y, m);
        }
    }
}

pub fn update_num(d: &mut Doom, n: &mut StNumber, value: i32, on: bool) {
    if on {
        draw_num(d, n, value);
    }
}

pub fn init_percent(x: i32, y: i32, pl: &[usize], percent: usize) -> StPercent {
    StPercent {
        n: init_num(x, y, pl, 3),
        p: percent,
    }
}

pub fn update_percent(d: &mut Doom, per: &mut StPercent, value: i32, on: bool, refresh: bool) {
    if refresh && on {
        crate::v_video::draw_patch(d, per.n.x, per.n.y, per.p);
    }
    update_num(d, &mut per.n, value, on);
}

pub fn init_mult_icon(x: i32, y: i32, il: &[usize]) -> StMultIcon {
    StMultIcon {
        x,
        y,
        oldinum: -1,
        p: il.to_vec(),
    }
}

pub fn update_mult_icon(d: &mut Doom, mi: &mut StMultIcon, inum: i32, on: bool, refresh: bool) {
    if on && (mi.oldinum != inum || refresh) && inum != -1 {
        if mi.oldinum != -1 {
            let (w, h, lo, to) = patch_header(d, mi.p[mi.oldinum as usize]);
            let x = mi.x - lo;
            let y = mi.y - to;
            if y - ST_Y < 0 {
                crate::i_system::error(d, "updateMultIcon: y - ST_Y < 0");
            }
            restore_background(d, x, y - ST_Y, w, h, y);
        }
        crate::v_video::draw_patch(d, mi.x, mi.y, mi.p[inum as usize]);
        mi.oldinum = inum;
    }
}

pub fn init_bin_icon(x: i32, y: i32, i: usize) -> StBinIcon {
    StBinIcon {
        x,
        y,
        oldval: false,
        p: i,
    }
}

pub fn update_bin_icon(d: &mut Doom, bi: &mut StBinIcon, val: bool, on: bool, refresh: bool) {
    if on && (bi.oldval != val || refresh) {
        let (w, h, lo, to) = patch_header(d, bi.p);
        let x = bi.x - lo;
        let y = bi.y - to;
        if y - ST_Y < 0 {
            crate::i_system::error(d, "updateBinIcon: y - ST_Y < 0");
        }
        if val {
            crate::v_video::draw_patch(d, bi.x, bi.y, bi.p);
        } else {
            restore_background(d, x, y - ST_Y, w, h, y);
        }
        bi.oldval = val;
    }
}

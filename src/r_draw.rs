//! Column and span drawers, the view window layout, the translation tables, and the view border.

use crate::i_system;
use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::m_fixed::{Fixed, FRACBITS};
use crate::r_data::ColSrc;
use crate::w_wad;
use crate::Doom;

pub const SBARHEIGHT: i32 = 32;
const FUZZTABLE: usize = 50;

/// The fuzz offsets of the original: '+' is one row down, '-' is one row up.
const FUZZOFFSET: &[u8; FUZZTABLE] = b"+-+-++-++-+++-+++----+--++++-+-++--++----++++-++-+";

/// The column and span drawers of the original. The low variants draw two pixels for each column.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ColFunc {
    #[default]
    Column,
    ColumnLow,
    Fuzz,
    FuzzLow,
    Translated,
    TranslatedLow,
}

pub struct State {
    pub viewwidth: i32,
    pub scaledviewwidth: i32,
    pub viewheight: i32,
    pub viewwindowx: i32,
    pub viewwindowy: i32,
    /// The screen offset of each view row.
    pub ylookup: Vec<usize>,
    /// The screen offset of each view column.
    pub columnofs: Vec<usize>,
    /// The colormap offset in `r_data.colormaps`.
    pub dc_colormap: usize,
    pub dc_x: i32,
    pub dc_yl: i32,
    pub dc_yh: i32,
    pub dc_iscale: Fixed,
    pub dc_texturemid: Fixed,
    pub dc_source: ColSrc,
    /// The table offset in `translationtables`.
    pub dc_translation: usize,
    pub ds_y: i32,
    pub ds_x1: i32,
    pub ds_x2: i32,
    pub ds_colormap: usize,
    pub ds_xfrac: Fixed,
    pub ds_yfrac: Fixed,
    pub ds_xstep: Fixed,
    pub ds_ystep: Fixed,
    /// The flat lump number.
    pub ds_source: usize,
    pub translationtables: Vec<u8>,
    pub fuzzpos: usize,
    /// The saved border picture (320x168 used), or None for a full-width view.
    pub background_buffer: Option<Vec<u8>>,
    /// The zone offset of background_buffer in the zone model.
    pub zbackground: usize,
    pub colfunc: ColFunc,
    pub basecolfunc: ColFunc,
    pub fuzzcolfunc: ColFunc,
    pub transcolfunc: ColFunc,
    pub spanlow: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            viewwidth: 0,
            scaledviewwidth: 0,
            viewheight: 0,
            viewwindowx: 0,
            viewwindowy: 0,
            ylookup: vec![0; SCREENHEIGHT],
            columnofs: vec![0; SCREENWIDTH],
            dc_colormap: 0,
            dc_x: 0,
            dc_yl: 0,
            dc_yh: 0,
            dc_iscale: 0,
            dc_texturemid: 0,
            dc_source: ColSrc::Lump(0, 0),
            dc_translation: 0,
            ds_y: 0,
            ds_x1: 0,
            ds_x2: 0,
            ds_colormap: 0,
            ds_xfrac: 0,
            ds_yfrac: 0,
            ds_xstep: 0,
            ds_ystep: 0,
            ds_source: 0,
            translationtables: Vec::new(),
            fuzzpos: 0,
            background_buffer: None,
            zbackground: 0,
            colfunc: ColFunc::Column,
            basecolfunc: ColFunc::Column,
            fuzzcolfunc: ColFunc::Fuzz,
            transcolfunc: ColFunc::Translated,
            spanlow: false,
        }
    }
}

/// Returns the bytes of a column source and the start index of the column in them.
/// With the zone model, the bytes are the zone, so reads past the column end return the next zone bytes as in the original.
pub fn source<'a>(
    z: &'a crate::z_zone::State,
    w: &'a w_wad::State,
    r: &'a crate::r_data::State,
    s: ColSrc,
) -> (&'a [u8], usize) {
    let (b, p, o) = match s {
        ColSrc::Lump(l, o) => (
            w_wad::lump(w, l),
            z.lumpcache.get(l).copied().unwrap_or(0),
            o,
        ),
        ColSrc::Composite(t, o) => (
            &r.texturecomposite[t][..],
            crate::r_data::composite_ofs(z, r, t),
            o,
        ),
    };
    if p != 0 {
        (&z.mem, p.wrapping_add(o))
    } else {
        (b, o.min(b.len()))
    }
}

/// Calls the current column drawer (colfunc).
pub fn colfunc(d: &mut Doom) {
    match d.r_draw.colfunc {
        ColFunc::Column => draw_column(d, false),
        ColFunc::ColumnLow => draw_column(d, true),
        ColFunc::Fuzz => draw_fuzz_column(d, false),
        ColFunc::FuzzLow => draw_fuzz_column(d, true),
        ColFunc::Translated => draw_translated_column(d, false),
        ColFunc::TranslatedLow => draw_translated_column(d, true),
    }
}

/// R_DrawColumn and R_DrawColumnLow.
fn draw_column(d: &mut Doom, low: bool) {
    let s = &d.r_draw;
    let count = s.dc_yh - s.dc_yl;
    if count < 0 {
        return;
    }
    if s.dc_yh >= SCREENHEIGHT as i32 || s.dc_yl < 0 || s.dc_x as u32 >= SCREENWIDTH as u32 {
        let msg = format!("R_DrawColumn: {} to {} at {}", s.dc_yl, s.dc_yh, s.dc_x);
        i_system::error(d, &msg);
    }
    let x = if low {
        (s.dc_x << 1) as usize
    } else {
        s.dc_x as usize
    };
    let mut dest = s.ylookup[s.dc_yl as usize] + s.columnofs[x];
    let dest2 = if low {
        s.ylookup[s.dc_yl as usize] + s.columnofs[x + 1]
    } else {
        0
    };
    let fracstep = s.dc_iscale;
    let mut frac = s
        .dc_texturemid
        .wrapping_add((s.dc_yl - d.r_main.centery).wrapping_mul(fracstep));
    let (src, at) = source(&d.z_zone, &d.w_wad, &d.r_data, s.dc_source);
    let cm = &d.r_data.colormaps[s.dc_colormap..];
    let screen = &mut d.i_video.video_buffer;
    let delta = dest2.wrapping_sub(dest);
    for _ in 0..=count {
        let c = cm[src
            .get(at + ((frac >> FRACBITS) & 127) as usize)
            .copied()
            .unwrap_or(0) as usize];
        screen[dest] = c;
        if low {
            screen[dest.wrapping_add(delta)] = c;
        }
        dest += SCREENWIDTH;
        frac = frac.wrapping_add(fracstep);
    }
}

/// R_DrawFuzzColumn and R_DrawFuzzColumnLow: darkens the pixels around the column.
fn draw_fuzz_column(d: &mut Doom, low: bool) {
    let viewheight = d.r_draw.viewheight;
    let s = &mut d.r_draw;
    if s.dc_yl == 0 {
        s.dc_yl = 1;
    }
    if s.dc_yh == viewheight - 1 {
        s.dc_yh = viewheight - 2;
    }
    let count = s.dc_yh - s.dc_yl;
    if count < 0 {
        return;
    }
    let x = if low { s.dc_x << 1 } else { s.dc_x };
    if s.dc_yh >= SCREENHEIGHT as i32 || s.dc_yl < 0 || x as u32 >= SCREENWIDTH as u32 {
        let msg = format!("R_DrawFuzzColumn: {} to {} at {}", s.dc_yl, s.dc_yh, s.dc_x);
        i_system::error(d, &msg);
    }
    let x = x as usize;
    let mut dest = s.ylookup[s.dc_yl as usize] + s.columnofs[x];
    let mut dest2 = if low {
        s.ylookup[s.dc_yl as usize] + s.columnofs[x + 1]
    } else {
        0
    };
    let cm = &d.r_data.colormaps[6 * 256..];
    let screen = &mut d.i_video.video_buffer;
    let mut pos = s.fuzzpos;
    for _ in 0..=count {
        let up = FUZZOFFSET[pos] == b'-';
        let at = |p: usize| if up { p - SCREENWIDTH } else { p + SCREENWIDTH };
        screen[dest] = cm[screen[at(dest)] as usize];
        if low {
            screen[dest2] = cm[screen[at(dest2)] as usize];
            dest2 += SCREENWIDTH;
        }
        pos += 1;
        if pos == FUZZTABLE {
            pos = 0;
        }
        dest += SCREENWIDTH;
    }
    s.fuzzpos = pos;
}

/// R_DrawTranslatedColumn and R_DrawTranslatedColumnLow: remaps the green ramp for player colors.
fn draw_translated_column(d: &mut Doom, low: bool) {
    let s = &d.r_draw;
    let count = s.dc_yh - s.dc_yl;
    if count < 0 {
        return;
    }
    let x = if low { s.dc_x << 1 } else { s.dc_x };
    if s.dc_yh >= SCREENHEIGHT as i32 || s.dc_yl < 0 || x as u32 >= SCREENWIDTH as u32 {
        let msg = format!("R_DrawColumn: {} to {} at {}", s.dc_yl, s.dc_yh, s.dc_x);
        i_system::error(d, &msg);
    }
    let x = x as usize;
    let mut dest = s.ylookup[s.dc_yl as usize] + s.columnofs[x];
    let dest2 = if low {
        s.ylookup[s.dc_yl as usize] + s.columnofs[x + 1]
    } else {
        0
    };
    let delta = dest2.wrapping_sub(dest);
    let fracstep = s.dc_iscale;
    let mut frac = s
        .dc_texturemid
        .wrapping_add((s.dc_yl - d.r_main.centery).wrapping_mul(fracstep));
    let (src, at) = source(&d.z_zone, &d.w_wad, &d.r_data, s.dc_source);
    let cm = &d.r_data.colormaps[s.dc_colormap..];
    let tr = &s.translationtables[s.dc_translation..];
    let screen = &mut d.i_video.video_buffer;
    for _ in 0..=count {
        // The original reads dc_source[frac >> FRACBITS] without a mask.
        let p = src
            .get(at.wrapping_add((frac >> FRACBITS) as usize))
            .copied()
            .unwrap_or(0);
        let c = cm[tr[p as usize] as usize];
        screen[dest] = c;
        if low {
            screen[dest.wrapping_add(delta)] = c;
        }
        dest += SCREENWIDTH;
        frac = frac.wrapping_add(fracstep);
    }
}

/// R_InitTranslationTables: maps the green ramp to gray, brown, and red.
pub fn init_translation_tables(d: &mut Doom) {
    let mut t = vec![0u8; 256 * 3];
    for i in 0..256 {
        if (0x70..=0x7f).contains(&i) {
            t[i] = 0x60 + (i as u8 & 0xf);
            t[i + 256] = 0x40 + (i as u8 & 0xf);
            t[i + 512] = 0x20 + (i as u8 & 0xf);
        } else {
            t[i] = i as u8;
            t[i + 256] = i as u8;
            t[i + 512] = i as u8;
        }
    }
    d.z_zone.malloc_copy(&t, crate::z_zone::PU_STATIC, 0);
    d.r_draw.translationtables = t;
}

/// R_DrawSpan and R_DrawSpanLow: draws one horizontal row of a flat.
pub fn spanfunc(d: &mut Doom) {
    let s = &mut d.r_draw;
    if s.ds_x2 < s.ds_x1
        || s.ds_x1 < 0
        || s.ds_x2 >= SCREENWIDTH as i32
        || s.ds_y as u32 > SCREENHEIGHT as u32
    {
        let msg = format!("R_DrawSpan: {} to {} at {}", s.ds_x1, s.ds_x2, s.ds_y);
        i_system::error(d, &msg);
    }
    let count = s.ds_x2 - s.ds_x1;
    let low = s.spanlow;
    if low {
        s.ds_x1 <<= 1;
        s.ds_x2 <<= 1;
    }
    let mut position: u32 =
        (((s.ds_xfrac as u32) << 10) & 0xffff_0000) | (((s.ds_yfrac >> 6) as u32) & 0xffff);
    let step: u32 =
        (((s.ds_xstep as u32) << 10) & 0xffff_0000) | (((s.ds_ystep >> 6) as u32) & 0xffff);
    let mut dest = s.ylookup[s.ds_y as usize] + s.columnofs[s.ds_x1 as usize];
    let src = w_wad::lump(&d.w_wad, s.ds_source);
    let cm = &d.r_data.colormaps[s.ds_colormap..];
    let screen = &mut d.i_video.video_buffer;
    for _ in 0..=count {
        let spot = (((position >> 4) & 0x0fc0) | (position >> 26)) as usize;
        let c = cm[src.get(spot).copied().unwrap_or(0) as usize];
        screen[dest] = c;
        dest += 1;
        if low {
            screen[dest] = c;
            dest += 1;
        }
        position = position.wrapping_add(step);
    }
}

/// R_InitBuffer: places the view window and builds the row and column offsets.
pub fn init_buffer(d: &mut Doom, width: i32, height: i32) {
    let s = &mut d.r_draw;
    s.viewwindowx = (SCREENWIDTH as i32 - width) >> 1;
    for i in 0..width as usize {
        s.columnofs[i] = s.viewwindowx as usize + i;
    }
    s.viewwindowy = if width == SCREENWIDTH as i32 {
        0
    } else {
        (SCREENHEIGHT as i32 - SBARHEIGHT - height) >> 1
    };
    for i in 0..height as usize {
        s.ylookup[i] = (i + s.viewwindowy as usize) * SCREENWIDTH;
    }
}

/// R_FillBackScreen: draws the border flat and the bevel patches into the background buffer.
pub fn fill_back_screen(d: &mut Doom) {
    if d.r_draw.scaledviewwidth == SCREENWIDTH as i32 {
        d.r_draw.background_buffer = None;
        d.z_zone.free(std::mem::take(&mut d.r_draw.zbackground));
        return;
    }
    if d.r_draw.zbackground == 0 {
        d.r_draw.zbackground = d.z_zone.malloc(
            (SCREENWIDTH * (SCREENHEIGHT - SBARHEIGHT as usize)) as i32,
            crate::z_zone::PU_STATIC,
            0,
        );
    }
    let name = if d.d_main.gamemode == crate::d_main::COMMERCIAL {
        "GRNROCK"
    } else {
        "FLOOR7_2"
    };
    let lump = w_wad::get_num_for_name(d, name);
    d.z_zone.cache_lump(&d.w_wad, lump, crate::z_zone::PU_CACHE);
    let mut buf = d.r_draw.background_buffer.take().unwrap_or_default();
    buf.resize(SCREENWIDTH * SCREENHEIGHT, 0);
    {
        let src = w_wad::lump(&d.w_wad, lump);
        for y in 0..SCREENHEIGHT - SBARHEIGHT as usize {
            let row = &src[((y & 63) << 6).min(src.len())..];
            for x in 0..SCREENWIDTH {
                buf[y * SCREENWIDTH + x] = row.get(x & 63).copied().unwrap_or(0);
            }
        }
    }
    // V_UseBuffer: the patches go to the background buffer.
    std::mem::swap(&mut d.i_video.video_buffer, &mut buf);
    let (vx, vy, w, h) = (
        d.r_draw.viewwindowx,
        d.r_draw.viewwindowy,
        d.r_draw.scaledviewwidth,
        d.r_draw.viewheight,
    );
    let patch = |d: &mut Doom, n: &str| w_wad::get_num_for_name(d, n);
    let p = patch(d, "brdr_t");
    for x in (0..w).step_by(8) {
        crate::v_video::draw_patch(d, vx + x, vy - 8, p);
    }
    let p = patch(d, "brdr_b");
    for x in (0..w).step_by(8) {
        crate::v_video::draw_patch(d, vx + x, vy + h, p);
    }
    let p = patch(d, "brdr_l");
    for y in (0..h).step_by(8) {
        crate::v_video::draw_patch(d, vx - 8, vy + y, p);
    }
    let p = patch(d, "brdr_r");
    for y in (0..h).step_by(8) {
        crate::v_video::draw_patch(d, vx + w, vy + y, p);
    }
    let p = patch(d, "brdr_tl");
    crate::v_video::draw_patch(d, vx - 8, vy - 8, p);
    let p = patch(d, "brdr_tr");
    crate::v_video::draw_patch(d, vx + w, vy - 8, p);
    let p = patch(d, "brdr_bl");
    crate::v_video::draw_patch(d, vx - 8, vy + h, p);
    let p = patch(d, "brdr_br");
    crate::v_video::draw_patch(d, vx + w, vy + h, p);
    // V_RestoreBuffer.
    std::mem::swap(&mut d.i_video.video_buffer, &mut buf);
    d.r_draw.background_buffer = Some(buf);
}

/// R_VideoErase: copies a part of the background buffer to the screen.
pub fn video_erase(d: &mut Doom, ofs: usize, count: usize) {
    if let Some(bg) = &d.r_draw.background_buffer {
        d.i_video.video_buffer[ofs..ofs + count].copy_from_slice(&bg[ofs..ofs + count]);
    }
}

/// R_DrawViewBorder: restores the border around a reduced view.
pub fn draw_view_border(d: &mut Doom) {
    let (w, vh) = (
        d.r_draw.scaledviewwidth as usize,
        d.r_draw.viewheight as usize,
    );
    if w == SCREENWIDTH {
        return;
    }
    let top = ((SCREENHEIGHT - SBARHEIGHT as usize) - vh) / 2;
    let mut side = (SCREENWIDTH - w) / 2;
    video_erase(d, 0, top * SCREENWIDTH + side);
    let ofs = (vh + top) * SCREENWIDTH - side;
    video_erase(d, ofs, top * SCREENWIDTH + side);
    let mut ofs = top * SCREENWIDTH + SCREENWIDTH - side;
    side <<= 1;
    for _ in 1..vh {
        video_erase(d, ofs, side);
        ofs += SCREENWIDTH;
    }
    crate::v_video::mark_rect(
        d,
        0,
        0,
        SCREENWIDTH as i32,
        SCREENHEIGHT as i32 - SBARHEIGHT,
    );
}

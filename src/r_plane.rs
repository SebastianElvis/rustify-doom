//! Visplanes: floor and ceiling areas, their spans, and the sky.

use crate::i_system;
use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::m_fixed::{fixed_div, fixed_mul, Fixed};
use crate::r_main::{LIGHTLEVELS, LIGHTSEGSHIFT, LIGHTZSHIFT, MAXLIGHTZ};
use crate::r_sky::ANGLETOSKYSHIFT;
use crate::tables::{finecosine, finesine, ANG90, ANGLETOFINESHIFT};
use crate::Doom;

pub const MAXVISPLANES: usize = 128;
pub const MAXOPENINGS: usize = SCREENWIDTH * 64;

/// A visplane (visplane_t). `top` and `bottom` have one pad entry on each side: index x + 1 holds column x.
#[derive(Clone)]
pub struct Visplane {
    pub height: Fixed,
    pub picnum: i32,
    pub lightlevel: i32,
    pub minx: i32,
    pub maxx: i32,
    pub top: [u8; SCREENWIDTH + 2],
    pub bottom: [u8; SCREENWIDTH + 2],
}

impl Default for Visplane {
    fn default() -> Self {
        Visplane {
            height: 0,
            picnum: 0,
            lightlevel: 0,
            minx: 0,
            maxx: 0,
            top: [0; SCREENWIDTH + 2],
            bottom: [0; SCREENWIDTH + 2],
        }
    }
}

pub struct State {
    pub visplanes: Vec<Visplane>,
    pub lastvisplane: usize,
    pub floorplane: Option<usize>,
    pub ceilingplane: Option<usize>,
    /// The clip and sprite opening store (openings). It has spare room past MAXOPENINGS.
    pub openings: Vec<i16>,
    pub lastopening: usize,
    pub floorclip: [i16; SCREENWIDTH],
    pub ceilingclip: [i16; SCREENWIDTH],
    pub spanstart: [i32; SCREENHEIGHT],
    /// The light table row (index into r_main.zlight) of the current plane.
    pub planezlight: usize,
    pub planeheight: Fixed,
    pub yslope: [Fixed; SCREENHEIGHT],
    pub distscale: [Fixed; SCREENWIDTH],
    pub basexscale: Fixed,
    pub baseyscale: Fixed,
    pub cachedheight: [Fixed; SCREENHEIGHT],
    pub cacheddistance: [Fixed; SCREENHEIGHT],
    pub cachedxstep: [Fixed; SCREENHEIGHT],
    pub cachedystep: [Fixed; SCREENHEIGHT],
}

impl Default for State {
    fn default() -> Self {
        State {
            visplanes: vec![Visplane::default(); MAXVISPLANES],
            lastvisplane: 0,
            floorplane: None,
            ceilingplane: None,
            openings: vec![0; MAXOPENINGS * 2],
            lastopening: 0,
            floorclip: [0; SCREENWIDTH],
            ceilingclip: [0; SCREENWIDTH],
            spanstart: [0; SCREENHEIGHT],
            planezlight: 0,
            planeheight: 0,
            yslope: [0; SCREENHEIGHT],
            distscale: [0; SCREENWIDTH],
            basexscale: 0,
            baseyscale: 0,
            cachedheight: [0; SCREENHEIGHT],
            cacheddistance: [0; SCREENHEIGHT],
            cachedxstep: [0; SCREENHEIGHT],
            cachedystep: [0; SCREENHEIGHT],
        }
    }
}

/// Grows the opening store when the original would write past its end.
pub fn reserve_openings(p: &mut State, n: usize) {
    if p.lastopening + n + crate::i_video::SCREENWIDTH > p.openings.len() {
        let len = p.lastopening + n + 2 * crate::i_video::SCREENWIDTH;
        p.openings.resize(len, 0);
    }
}

/// R_InitPlanes. The original has nothing to do here.
pub fn init_planes(_d: &mut Doom) {}

/// R_MapPlane: computes the texture steps of one span and draws it.
fn map_plane(d: &mut Doom, y: i32, x1: i32, x2: i32) {
    if x2 < x1 || x1 < 0 || x2 >= d.r_draw.viewwidth || y > d.r_draw.viewheight {
        i_system::error(d, &format!("R_MapPlane: {}, {} at {}", x1, x2, y));
    }
    let p = &mut d.r_plane;
    let yu = y as usize;
    let (distance, xstep, ystep);
    if p.planeheight != p.cachedheight[yu] {
        p.cachedheight[yu] = p.planeheight;
        distance = fixed_mul(p.planeheight, p.yslope[yu]);
        p.cacheddistance[yu] = distance;
        xstep = fixed_mul(distance, p.basexscale);
        p.cachedxstep[yu] = xstep;
        ystep = fixed_mul(distance, p.baseyscale);
        p.cachedystep[yu] = ystep;
    } else {
        distance = p.cacheddistance[yu];
        xstep = p.cachedxstep[yu];
        ystep = p.cachedystep[yu];
    }
    let m = &d.r_main;
    let length = fixed_mul(distance, p.distscale[x1 as usize]);
    let angle =
        (m.viewangle.wrapping_add(m.xtoviewangle[x1 as usize]) >> ANGLETOFINESHIFT) as usize;
    let s = &mut d.r_draw;
    s.ds_xstep = xstep;
    s.ds_ystep = ystep;
    s.ds_xfrac = m.viewx.wrapping_add(fixed_mul(finecosine(angle), length));
    s.ds_yfrac = m
        .viewy
        .wrapping_neg()
        .wrapping_sub(fixed_mul(finesine(angle), length));
    s.ds_colormap = match m.fixedcolormap {
        Some(cm) => cm,
        None => {
            let index = ((distance >> LIGHTZSHIFT) as usize).min(MAXLIGHTZ - 1);
            m.zlight[p.planezlight][index]
        }
    };
    s.ds_y = y;
    s.ds_x1 = x1;
    s.ds_x2 = x2;
    crate::r_draw::spanfunc(d);
}

/// R_ClearPlanes: empties the visplanes and the clip arrays at the start of a frame.
pub fn clear_planes(d: &mut Doom) {
    let viewwidth = d.r_draw.viewwidth as usize;
    let viewheight = d.r_draw.viewheight as i16;
    let p = &mut d.r_plane;
    for i in 0..viewwidth {
        p.floorclip[i] = viewheight;
        p.ceilingclip[i] = -1;
    }
    p.lastvisplane = 0;
    p.lastopening = 0;
    p.cachedheight = [0; SCREENHEIGHT];
    let angle = (d.r_main.viewangle.wrapping_sub(ANG90) >> ANGLETOFINESHIFT) as usize;
    p.basexscale = fixed_div(finecosine(angle), d.r_main.centerxfrac);
    p.baseyscale = fixed_div(finesine(angle), d.r_main.centerxfrac).wrapping_neg();
}

/// R_FindPlane: returns a visplane with these properties, and creates it when needed.
pub fn find_plane(d: &mut Doom, mut height: Fixed, picnum: i32, mut lightlevel: i32) -> usize {
    if picnum == d.r_sky.skyflatnum {
        height = 0;
        lightlevel = 0;
    }
    let p = &mut d.r_plane;
    for i in 0..p.lastvisplane {
        let c = &p.visplanes[i];
        if height == c.height && picnum == c.picnum && lightlevel == c.lightlevel {
            return i;
        }
    }
    if p.lastvisplane == MAXVISPLANES {
        i_system::error(d, "R_FindPlane: no more visplanes");
    }
    let i = p.lastvisplane;
    p.lastvisplane += 1;
    if i == p.visplanes.len() {
        p.visplanes.push(Visplane::default());
    }
    let c = &mut p.visplanes[i];
    c.height = height;
    c.picnum = picnum;
    c.lightlevel = lightlevel;
    c.minx = SCREENWIDTH as i32;
    c.maxx = -1;
    c.top = [0xff; SCREENWIDTH + 2];
    i
}

/// R_CheckPlane: extends a visplane over a column range, or splits it into a new visplane.
pub fn check_plane(d: &mut Doom, pl: usize, start: i32, stop: i32) -> usize {
    let p = &mut d.r_plane;
    let (minx, maxx) = (p.visplanes[pl].minx, p.visplanes[pl].maxx);
    let (intrl, unionl) = if start < minx {
        (minx, start)
    } else {
        (start, minx)
    };
    let (intrh, unionh) = if stop > maxx {
        (maxx, stop)
    } else {
        (stop, maxx)
    };
    let mut x = intrl;
    while x <= intrh {
        if p.visplanes[pl].top[(x + 1) as usize] != 0xff {
            break;
        }
        x += 1;
    }
    if x > intrh {
        p.visplanes[pl].minx = unionl;
        p.visplanes[pl].maxx = unionh;
        return pl;
    }
    // The original has no limit check here. R_DrawPlanes reports the overflow later.
    let n = p.lastvisplane;
    if n == p.visplanes.len() {
        p.visplanes.push(Visplane::default());
    }
    let (h, pic, light) = (
        p.visplanes[pl].height,
        p.visplanes[pl].picnum,
        p.visplanes[pl].lightlevel,
    );
    let c = &mut p.visplanes[n];
    c.height = h;
    c.picnum = pic;
    c.lightlevel = light;
    p.lastvisplane += 1;
    c.minx = start;
    c.maxx = stop;
    c.top = [0xff; SCREENWIDTH + 2];
    n
}

/// R_MakeSpans: closes the spans that end at column x and opens the spans that start there.
fn make_spans(d: &mut Doom, x: i32, mut t1: i32, mut b1: i32, mut t2: i32, mut b2: i32) {
    while t1 < t2 && t1 <= b1 {
        let s = d.r_plane.spanstart[t1 as usize];
        map_plane(d, t1, s, x - 1);
        t1 += 1;
    }
    while b1 > b2 && b1 >= t1 {
        let s = d.r_plane.spanstart[b1 as usize];
        map_plane(d, b1, s, x - 1);
        b1 -= 1;
    }
    while t2 < t1 && t2 <= b2 {
        d.r_plane.spanstart[t2 as usize] = x;
        t2 += 1;
    }
    while b2 > b1 && b2 >= t2 {
        d.r_plane.spanstart[b2 as usize] = x;
        b2 -= 1;
    }
}

/// R_DrawPlanes: draws all visplanes of the frame.
pub fn draw_planes(d: &mut Doom) {
    let nds = d.r_bsp.ds_p;
    if nds > crate::r_bsp::MAXDRAWSEGS {
        i_system::error(d, &format!("R_DrawPlanes: drawsegs overflow ({})", nds));
    }
    let nvp = d.r_plane.lastvisplane;
    if nvp > MAXVISPLANES {
        i_system::error(d, &format!("R_DrawPlanes: visplane overflow ({})", nvp));
    }
    let nop = d.r_plane.lastopening;
    if nop > MAXOPENINGS {
        i_system::error(d, &format!("R_DrawPlanes: opening overflow ({})", nop));
    }
    for i in 0..nvp {
        let (minx, maxx, picnum, height, lightlevel) = {
            let pl = &d.r_plane.visplanes[i];
            (pl.minx, pl.maxx, pl.picnum, pl.height, pl.lightlevel)
        };
        if minx > maxx {
            continue;
        }
        if picnum == d.r_sky.skyflatnum {
            let s = &mut d.r_draw;
            s.dc_iscale = d.r_things.pspriteiscale >> d.r_main.detailshift;
            // The sky always uses colormap 0, so invulnerability does not change it.
            s.dc_colormap = 0;
            s.dc_texturemid = d.r_sky.skytexturemid;
            for x in minx..=maxx {
                let (yl, yh) = (
                    d.r_plane.visplanes[i].top[(x + 1) as usize] as i32,
                    d.r_plane.visplanes[i].bottom[(x + 1) as usize] as i32,
                );
                d.r_draw.dc_yl = yl;
                d.r_draw.dc_yh = yh;
                if yl <= yh {
                    let angle = d
                        .r_main
                        .viewangle
                        .wrapping_add(d.r_main.xtoviewangle[x as usize])
                        >> ANGLETOSKYSHIFT;
                    d.r_draw.dc_x = x;
                    let sky = d.r_sky.skytexture as usize;
                    d.r_draw.dc_source = crate::r_data::get_column(d, sky, angle as i32);
                    crate::r_draw::colfunc(d);
                }
            }
            continue;
        }
        let lumpnum = d.r_data.firstflat + d.r_data.flattranslation[picnum as usize];
        d.r_draw.ds_source = lumpnum as usize;
        let zp = d
            .z_zone
            .cache_lump(&d.w_wad, lumpnum as usize, crate::z_zone::PU_STATIC);
        d.r_plane.planeheight = height.wrapping_sub(d.r_main.viewz).wrapping_abs();
        let light =
            ((lightlevel >> LIGHTSEGSHIFT) + d.r_main.extralight).clamp(0, LIGHTLEVELS as i32 - 1);
        d.r_plane.planezlight = light as usize;
        {
            let pl = &mut d.r_plane.visplanes[i];
            pl.top[(maxx + 2) as usize] = 0xff;
            pl.top[minx as usize] = 0xff;
        }
        let stop = maxx + 1;
        for x in minx..=stop {
            let pl = &d.r_plane.visplanes[i];
            let (t1, b1, t2, b2) = (
                pl.top[x as usize] as i32,
                pl.bottom[x as usize] as i32,
                pl.top[(x + 1) as usize] as i32,
                pl.bottom[(x + 1) as usize] as i32,
            );
            make_spans(d, x, t1, b1, t2, b2);
        }
        d.z_zone.change_tag(zp, crate::z_zone::PU_CACHE);
    }
}

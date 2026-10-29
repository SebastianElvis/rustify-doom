//! Wall segments: wall ranges, plane marks, wall columns, and masked middle textures.

use crate::i_system;
use crate::m_fixed::{fixed_mul, Fixed, FRACBITS};
use crate::p_setup::{ML_DONTPEGBOTTOM, ML_DONTPEGTOP, ML_MAPPED};
use crate::r_bsp::{Clip, MAXDRAWSEGS, SIL_BOTH, SIL_BOTTOM, SIL_TOP};
use crate::r_data::ColSrc;
use crate::r_main::{LIGHTLEVELS, LIGHTSCALESHIFT, LIGHTSEGSHIFT, MAXLIGHTSCALE};
use crate::tables::{finesine, Angle, ANG180, ANG90, ANGLETOFINESHIFT, FINETANGENT};
use crate::Doom;

const HEIGHTBITS: i32 = 12;
const HEIGHTUNIT: i32 = 1 << HEIGHTBITS;
const SHRT_MAX: i16 = i16::MAX;

pub struct State {
    pub segtextured: bool,
    pub markfloor: bool,
    pub markceiling: bool,
    pub maskedtexture: bool,
    pub toptexture: i32,
    pub bottomtexture: i32,
    pub midtexture: i32,
    pub rw_normalangle: Angle,
    pub rw_angle1: Angle,
    pub rw_x: i32,
    pub rw_stopx: i32,
    pub rw_centerangle: Angle,
    pub rw_offset: Fixed,
    pub rw_distance: Fixed,
    pub rw_scale: Fixed,
    pub rw_scalestep: Fixed,
    pub rw_midtexturemid: Fixed,
    pub rw_toptexturemid: Fixed,
    pub rw_bottomtexturemid: Fixed,
    pub worldtop: i32,
    pub worldbottom: i32,
    pub worldhigh: i32,
    pub worldlow: i32,
    pub pixhigh: Fixed,
    pub pixlow: Fixed,
    pub pixhighstep: Fixed,
    pub pixlowstep: Fixed,
    pub topfrac: Fixed,
    pub topstep: Fixed,
    pub bottomfrac: Fixed,
    pub bottomstep: Fixed,
    /// The colormap offsets of the current wall light level.
    pub walllights: [usize; MAXLIGHTSCALE],
    /// A base offset into `r_plane.openings`.
    pub maskedtexturecol: isize,
}

impl Default for State {
    fn default() -> Self {
        State {
            segtextured: false,
            markfloor: false,
            markceiling: false,
            maskedtexture: false,
            toptexture: 0,
            bottomtexture: 0,
            midtexture: 0,
            rw_normalangle: 0,
            rw_angle1: 0,
            rw_x: 0,
            rw_stopx: 0,
            rw_centerangle: 0,
            rw_offset: 0,
            rw_distance: 0,
            rw_scale: 0,
            rw_scalestep: 0,
            rw_midtexturemid: 0,
            rw_toptexturemid: 0,
            rw_bottomtexturemid: 0,
            worldtop: 0,
            worldbottom: 0,
            worldhigh: 0,
            worldlow: 0,
            pixhigh: 0,
            pixlow: 0,
            pixhighstep: 0,
            pixlowstep: 0,
            topfrac: 0,
            topstep: 0,
            bottomfrac: 0,
            bottomstep: 0,
            walllights: [0; MAXLIGHTSCALE],
            maskedtexturecol: 0,
        }
    }
}

/// Selects the wall light table for a sector light level and the seg direction.
fn wall_lights(d: &Doom, lightlevel: i32, seg: usize) -> [usize; MAXLIGHTSCALE] {
    let s = &d.p_setup.segs[seg];
    let (v1, v2) = (d.p_setup.vertexes[s.v1], d.p_setup.vertexes[s.v2]);
    let mut lightnum = (lightlevel >> LIGHTSEGSHIFT) + d.r_main.extralight;
    if v1.y == v2.y {
        lightnum -= 1;
    } else if v1.x == v2.x {
        lightnum += 1;
    }
    d.r_main.scalelight[lightnum.clamp(0, LIGHTLEVELS as i32 - 1) as usize]
}

/// R_RenderMaskedSegRange: draws the masked middle texture of a drawseg over a column range.
pub fn render_masked_seg_range(d: &mut Doom, ds: usize, x1: i32, x2: i32) {
    let dseg = d.r_bsp.drawsegs[ds];
    let seg = d.p_setup.segs[dseg.curline];
    let fs = d.p_setup.sectors[seg.frontsector].clone();
    let bs = d.p_setup.sectors[seg.backsector.unwrap_or(seg.frontsector)].clone();
    let side = d.p_setup.sides[seg.sidedef];
    let texnum = d.r_data.texturetranslation[side.midtexture as usize] as usize;
    d.r_segs.walllights = wall_lights(d, fs.lightlevel, dseg.curline);
    let base = dseg.maskedtexturecol.unwrap_or(0);
    d.r_segs.maskedtexturecol = base;
    d.r_segs.rw_scalestep = dseg.scalestep;
    d.r_things.spryscale = dseg
        .scale1
        .wrapping_add((x1 - dseg.x1).wrapping_mul(dseg.scalestep));
    d.r_things.mfloorclip = dseg.sprbottomclip;
    d.r_things.mceilingclip = dseg.sprtopclip;
    let viewz = d.r_main.viewz;
    let mut mid = if d.p_setup.lines[seg.linedef].flags & ML_DONTPEGBOTTOM != 0 {
        let h = fs.floorheight.max(bs.floorheight);
        h.wrapping_add(d.r_data.textureheight[texnum])
            .wrapping_sub(viewz)
    } else {
        fs.ceilingheight.min(bs.ceilingheight).wrapping_sub(viewz)
    };
    mid = mid.wrapping_add(side.rowoffset);
    d.r_draw.dc_texturemid = mid;
    if let Some(cm) = d.r_main.fixedcolormap {
        d.r_draw.dc_colormap = cm;
    }
    for x in x1..=x2 {
        d.r_draw.dc_x = x;
        let at = (base + x as isize) as usize;
        let col = d.r_plane.openings[at];
        if col != SHRT_MAX {
            let spryscale = d.r_things.spryscale;
            if d.r_main.fixedcolormap.is_none() {
                let index = ((spryscale >> LIGHTSCALESHIFT) as usize).min(MAXLIGHTSCALE - 1);
                d.r_draw.dc_colormap = d.r_segs.walllights[index];
            }
            d.r_things.sprtopscreen = d
                .r_main
                .centeryfrac
                .wrapping_sub(fixed_mul(d.r_draw.dc_texturemid, spryscale));
            d.r_draw.dc_iscale = (0xffff_ffffu32 / spryscale as u32) as i32;
            let src = match crate::r_data::get_column(d, texnum, col as i32) {
                ColSrc::Lump(l, o) => ColSrc::Lump(l, o.wrapping_sub(3)),
                ColSrc::Composite(t, o) => ColSrc::Composite(t, o.wrapping_sub(3)),
            };
            crate::r_things::draw_masked_column(d, src);
            d.r_plane.openings[at] = SHRT_MAX;
        }
        d.r_things.spryscale = d.r_things.spryscale.wrapping_add(d.r_segs.rw_scalestep);
    }
}

/// R_RenderSegLoop: draws the wall columns of the current range and marks the planes.
fn render_seg_loop(d: &mut Doom) {
    let viewheight = d.r_draw.viewheight as i16;
    while d.r_segs.rw_x < d.r_segs.rw_stopx {
        let x = d.r_segs.rw_x as usize;
        let s = &d.r_segs;
        let ceilclip = d.r_plane.ceilingclip[x] as i32;
        let floorclip = d.r_plane.floorclip[x] as i32;
        let mut yl = (s.topfrac.wrapping_add(HEIGHTUNIT - 1)) >> HEIGHTBITS;
        if yl < ceilclip + 1 {
            yl = ceilclip + 1;
        }
        if s.markceiling {
            let top = ceilclip + 1;
            let mut bottom = yl - 1;
            if bottom >= floorclip {
                bottom = floorclip - 1;
            }
            if top <= bottom {
                let pl = d.r_plane.ceilingplane.unwrap_or(0);
                d.r_plane.visplanes[pl].top[x + 1] = top as u8;
                d.r_plane.visplanes[pl].bottom[x + 1] = bottom as u8;
            }
        }
        let mut yh = s.bottomfrac >> HEIGHTBITS;
        if yh >= floorclip {
            yh = floorclip - 1;
        }
        if s.markfloor {
            let mut top = yh + 1;
            let bottom = floorclip - 1;
            if top <= ceilclip {
                top = ceilclip + 1;
            }
            if top <= bottom {
                let pl = d.r_plane.floorplane.unwrap_or(0);
                d.r_plane.visplanes[pl].top[x + 1] = top as u8;
                d.r_plane.visplanes[pl].bottom[x + 1] = bottom as u8;
            }
        }
        let mut texturecolumn = 0;
        if s.segtextured {
            let angle = (s.rw_centerangle.wrapping_add(d.r_main.xtoviewangle[x])
                >> ANGLETOFINESHIFT) as usize;
            // The original data puts finesine right after finetangent, so an index past 4095 reads finesine.
            let tan = if angle < 4096 {
                FINETANGENT[angle]
            } else {
                crate::tables::finesine(angle - 4096)
            };
            texturecolumn = s.rw_offset.wrapping_sub(fixed_mul(tan, s.rw_distance)) >> FRACBITS;
            let index = ((s.rw_scale >> LIGHTSCALESHIFT) as usize).min(MAXLIGHTSCALE - 1);
            d.r_draw.dc_colormap = s.walllights[index];
            d.r_draw.dc_x = x as i32;
            d.r_draw.dc_iscale = (0xffff_ffffu32 / s.rw_scale as u32) as i32;
        }
        let s = &d.r_segs;
        if s.midtexture != 0 {
            d.r_draw.dc_yl = yl;
            d.r_draw.dc_yh = yh;
            d.r_draw.dc_texturemid = s.rw_midtexturemid;
            let tex = s.midtexture as usize;
            d.r_draw.dc_source = crate::r_data::get_column(d, tex, texturecolumn);
            crate::r_draw::colfunc(d);
            d.r_plane.ceilingclip[x] = viewheight;
            d.r_plane.floorclip[x] = -1;
        } else {
            if s.toptexture != 0 {
                let mut mid = s.pixhigh >> HEIGHTBITS;
                d.r_segs.pixhigh = s.pixhigh.wrapping_add(s.pixhighstep);
                if mid >= floorclip {
                    mid = floorclip - 1;
                }
                if mid >= yl {
                    d.r_draw.dc_yl = yl;
                    d.r_draw.dc_yh = mid;
                    d.r_draw.dc_texturemid = d.r_segs.rw_toptexturemid;
                    let tex = d.r_segs.toptexture as usize;
                    d.r_draw.dc_source = crate::r_data::get_column(d, tex, texturecolumn);
                    crate::r_draw::colfunc(d);
                    d.r_plane.ceilingclip[x] = mid as i16;
                } else {
                    d.r_plane.ceilingclip[x] = (yl - 1) as i16;
                }
            } else if s.markceiling {
                d.r_plane.ceilingclip[x] = (yl - 1) as i16;
            }
            let s = &d.r_segs;
            if s.bottomtexture != 0 {
                let mut mid = (s.pixlow.wrapping_add(HEIGHTUNIT - 1)) >> HEIGHTBITS;
                d.r_segs.pixlow = s.pixlow.wrapping_add(s.pixlowstep);
                let cc = d.r_plane.ceilingclip[x] as i32;
                if mid <= cc {
                    mid = cc + 1;
                }
                if mid <= yh {
                    d.r_draw.dc_yl = mid;
                    d.r_draw.dc_yh = yh;
                    d.r_draw.dc_texturemid = d.r_segs.rw_bottomtexturemid;
                    let tex = d.r_segs.bottomtexture as usize;
                    d.r_draw.dc_source = crate::r_data::get_column(d, tex, texturecolumn);
                    crate::r_draw::colfunc(d);
                    d.r_plane.floorclip[x] = mid as i16;
                } else {
                    d.r_plane.floorclip[x] = (yh + 1) as i16;
                }
            } else if s.markfloor {
                d.r_plane.floorclip[x] = (yh + 1) as i16;
            }
            if d.r_segs.maskedtexture {
                let at = (d.r_segs.maskedtexturecol + x as isize) as usize;
                d.r_plane.openings[at] = texturecolumn as i16;
            }
        }
        let s = &mut d.r_segs;
        s.rw_scale = s.rw_scale.wrapping_add(s.rw_scalestep);
        s.topfrac = s.topfrac.wrapping_add(s.topstep);
        s.bottomfrac = s.bottomfrac.wrapping_add(s.bottomstep);
        s.rw_x += 1;
    }
}

/// R_StoreWallRange: draws a wall range of the current seg and records a drawseg.
pub fn store_wall_range(d: &mut Doom, start: i32, stop: i32) {
    if d.r_bsp.ds_p == MAXDRAWSEGS {
        return;
    }
    if start >= d.r_draw.viewwidth || start > stop {
        i_system::error(d, &format!("Bad R_RenderWallRange: {} to {}", start, stop));
    }
    let curline = d.r_bsp.curline;
    let seg = d.p_setup.segs[curline];
    let sidedef = d.p_setup.sides[seg.sidedef];
    d.r_bsp.sidedef = seg.sidedef;
    d.r_bsp.linedef = seg.linedef;
    d.p_setup.lines[seg.linedef].flags |= ML_MAPPED;
    let lineflags = d.p_setup.lines[seg.linedef].flags;
    let fs = d.p_setup.sectors[d.r_bsp.frontsector].clone();
    let bs = d.r_bsp.backsector.map(|b| d.p_setup.sectors[b].clone());
    let skyflatnum = d.r_sky.skyflatnum;
    let viewz = d.r_main.viewz;
    let viewangle = d.r_main.viewangle;

    d.r_segs.rw_normalangle = seg.angle.wrapping_add(ANG90);
    let mut offsetangle =
        (d.r_segs.rw_normalangle.wrapping_sub(d.r_segs.rw_angle1) as i32).wrapping_abs() as u32;
    if offsetangle > ANG90 {
        offsetangle = ANG90;
    }
    let distangle = ANG90 - offsetangle;
    let v1 = d.p_setup.vertexes[seg.v1];
    let hyp = crate::r_main::point_to_dist(d, v1.x, v1.y);
    let sineval = finesine((distangle >> ANGLETOFINESHIFT) as usize);
    d.r_segs.rw_distance = fixed_mul(hyp, sineval);

    let dsi = d.r_bsp.ds_p;
    d.r_segs.rw_x = start;
    d.r_bsp.drawsegs[dsi].x1 = start;
    d.r_bsp.drawsegs[dsi].x2 = stop;
    d.r_bsp.drawsegs[dsi].curline = curline;
    d.r_segs.rw_stopx = stop + 1;

    let scale1 = crate::r_main::scale_from_global_angle(
        d,
        viewangle.wrapping_add(d.r_main.xtoviewangle[start as usize]),
    );
    d.r_segs.rw_scale = scale1;
    d.r_bsp.drawsegs[dsi].scale1 = scale1;
    if stop > start {
        let scale2 = crate::r_main::scale_from_global_angle(
            d,
            viewangle.wrapping_add(d.r_main.xtoviewangle[stop as usize]),
        );
        d.r_bsp.drawsegs[dsi].scale2 = scale2;
        d.r_segs.rw_scalestep = scale2.wrapping_sub(scale1) / (stop - start);
        d.r_bsp.drawsegs[dsi].scalestep = d.r_segs.rw_scalestep;
    } else {
        d.r_bsp.drawsegs[dsi].scale2 = scale1;
    }

    let s = &mut d.r_segs;
    s.worldtop = fs.ceilingheight.wrapping_sub(viewz);
    s.worldbottom = fs.floorheight.wrapping_sub(viewz);
    s.midtexture = 0;
    s.toptexture = 0;
    s.bottomtexture = 0;
    s.maskedtexture = false;
    let ds = &mut d.r_bsp.drawsegs[dsi];
    ds.maskedtexturecol = None;
    let tt = &d.r_data.texturetranslation;
    let th = &d.r_data.textureheight;

    match &bs {
        None => {
            s.midtexture = tt[sidedef.midtexture as usize];
            s.markfloor = true;
            s.markceiling = true;
            s.rw_midtexturemid = if lineflags & ML_DONTPEGBOTTOM != 0 {
                fs.floorheight
                    .wrapping_add(th[sidedef.midtexture as usize])
                    .wrapping_sub(viewz)
            } else {
                s.worldtop
            };
            s.rw_midtexturemid = s.rw_midtexturemid.wrapping_add(sidedef.rowoffset);
            ds.silhouette = SIL_BOTH;
            ds.sprtopclip = Clip::ScreenHeight;
            ds.sprbottomclip = Clip::NegOne;
            ds.bsilheight = i32::MAX;
            ds.tsilheight = i32::MIN;
        }
        Some(bs) => {
            ds.sprtopclip = Clip::Null;
            ds.sprbottomclip = Clip::Null;
            ds.silhouette = 0;
            if fs.floorheight > bs.floorheight {
                ds.silhouette = SIL_BOTTOM;
                ds.bsilheight = fs.floorheight;
            } else if bs.floorheight > viewz {
                ds.silhouette = SIL_BOTTOM;
                ds.bsilheight = i32::MAX;
            }
            if fs.ceilingheight < bs.ceilingheight {
                ds.silhouette |= SIL_TOP;
                ds.tsilheight = fs.ceilingheight;
            } else if bs.ceilingheight < viewz {
                ds.silhouette |= SIL_TOP;
                ds.tsilheight = i32::MIN;
            }
            if bs.ceilingheight <= fs.floorheight {
                ds.sprbottomclip = Clip::NegOne;
                ds.bsilheight = i32::MAX;
                ds.silhouette |= SIL_BOTTOM;
            }
            if bs.floorheight >= fs.ceilingheight {
                ds.sprtopclip = Clip::ScreenHeight;
                ds.tsilheight = i32::MIN;
                ds.silhouette |= SIL_TOP;
            }
            s.worldhigh = bs.ceilingheight.wrapping_sub(viewz);
            s.worldlow = bs.floorheight.wrapping_sub(viewz);
            if fs.ceilingpic == skyflatnum && bs.ceilingpic == skyflatnum {
                s.worldtop = s.worldhigh;
            }
            s.markfloor = s.worldlow != s.worldbottom
                || bs.floorpic != fs.floorpic
                || bs.lightlevel != fs.lightlevel;
            s.markceiling = s.worldhigh != s.worldtop
                || bs.ceilingpic != fs.ceilingpic
                || bs.lightlevel != fs.lightlevel;
            if bs.ceilingheight <= fs.floorheight || bs.floorheight >= fs.ceilingheight {
                s.markceiling = true;
                s.markfloor = true;
            }
            if s.worldhigh < s.worldtop {
                s.toptexture = tt[sidedef.toptexture as usize];
                s.rw_toptexturemid = if lineflags & ML_DONTPEGTOP != 0 {
                    s.worldtop
                } else {
                    bs.ceilingheight
                        .wrapping_add(th[sidedef.toptexture as usize])
                        .wrapping_sub(viewz)
                };
            }
            if s.worldlow > s.worldbottom {
                s.bottomtexture = tt[sidedef.bottomtexture as usize];
                s.rw_bottomtexturemid = if lineflags & ML_DONTPEGBOTTOM != 0 {
                    s.worldtop
                } else {
                    s.worldlow
                };
            }
            s.rw_toptexturemid = s.rw_toptexturemid.wrapping_add(sidedef.rowoffset);
            s.rw_bottomtexturemid = s.rw_bottomtexturemid.wrapping_add(sidedef.rowoffset);
            if sidedef.midtexture != 0 {
                s.maskedtexture = true;
                let base = d.r_plane.lastopening as isize - s.rw_x as isize;
                s.maskedtexturecol = base;
                ds.maskedtexturecol = Some(base);
                d.r_plane.lastopening =
                    (d.r_plane.lastopening as isize + (s.rw_stopx - s.rw_x) as isize) as usize;
                crate::r_plane::reserve_openings(&mut d.r_plane, 0);
            }
        }
    }

    s.segtextured =
        s.midtexture != 0 || s.toptexture != 0 || s.bottomtexture != 0 || s.maskedtexture;
    if s.segtextured {
        let diff = s.rw_normalangle.wrapping_sub(s.rw_angle1);
        let mut offsetangle = diff;
        if offsetangle > ANG180 {
            offsetangle = offsetangle.wrapping_neg();
        }
        if offsetangle > ANG90 {
            offsetangle = ANG90;
        }
        let sineval = finesine((offsetangle >> ANGLETOFINESHIFT) as usize);
        s.rw_offset = fixed_mul(hyp, sineval);
        if diff < ANG180 {
            s.rw_offset = s.rw_offset.wrapping_neg();
        }
        s.rw_offset = s
            .rw_offset
            .wrapping_add(sidedef.textureoffset)
            .wrapping_add(seg.offset);
        s.rw_centerangle = ANG90.wrapping_add(viewangle).wrapping_sub(s.rw_normalangle);
        if d.r_main.fixedcolormap.is_none() {
            let wl = wall_lights(d, fs.lightlevel, curline);
            d.r_segs.walllights = wl;
        }
    }
    let s = &mut d.r_segs;
    if fs.floorheight >= viewz {
        s.markfloor = false;
    }
    if fs.ceilingheight <= viewz && fs.ceilingpic != skyflatnum {
        s.markceiling = false;
    }
    let centeryfrac = d.r_main.centeryfrac;
    s.worldtop >>= 4;
    s.worldbottom >>= 4;
    s.topstep = fixed_mul(s.rw_scalestep, s.worldtop).wrapping_neg();
    s.topfrac = (centeryfrac >> 4).wrapping_sub(fixed_mul(s.worldtop, s.rw_scale));
    s.bottomstep = fixed_mul(s.rw_scalestep, s.worldbottom).wrapping_neg();
    s.bottomfrac = (centeryfrac >> 4).wrapping_sub(fixed_mul(s.worldbottom, s.rw_scale));
    if bs.is_some() {
        s.worldhigh >>= 4;
        s.worldlow >>= 4;
        if s.worldhigh < s.worldtop {
            s.pixhigh = (centeryfrac >> 4).wrapping_sub(fixed_mul(s.worldhigh, s.rw_scale));
            s.pixhighstep = fixed_mul(s.rw_scalestep, s.worldhigh).wrapping_neg();
        }
        if s.worldlow > s.worldbottom {
            s.pixlow = (centeryfrac >> 4).wrapping_sub(fixed_mul(s.worldlow, s.rw_scale));
            s.pixlowstep = fixed_mul(s.rw_scalestep, s.worldlow).wrapping_neg();
        }
    }
    let (rw_x, rw_stopx) = (s.rw_x, s.rw_stopx);
    if s.markceiling {
        let pl = d.r_plane.ceilingplane.unwrap_or(0);
        d.r_plane.ceilingplane = Some(crate::r_plane::check_plane(d, pl, rw_x, rw_stopx - 1));
    }
    if d.r_segs.markfloor {
        let pl = d.r_plane.floorplane.unwrap_or(0);
        d.r_plane.floorplane = Some(crate::r_plane::check_plane(d, pl, rw_x, rw_stopx - 1));
    }
    render_seg_loop(d);

    let maskedtexture = d.r_segs.maskedtexture;
    let n = (rw_stopx - start) as usize;
    let ds = d.r_bsp.drawsegs[dsi];
    if ((ds.silhouette & SIL_TOP) != 0 || maskedtexture) && ds.sprtopclip == Clip::Null {
        let lo = d.r_plane.lastopening;
        let p = &mut d.r_plane;
        crate::r_plane::reserve_openings(p, n);
        p.openings[lo..lo + n].copy_from_slice(&p.ceilingclip[start as usize..start as usize + n]);
        d.r_bsp.drawsegs[dsi].sprtopclip = Clip::Openings(lo as isize - start as isize);
        d.r_plane.lastopening += n;
    }
    if ((ds.silhouette & SIL_BOTTOM) != 0 || maskedtexture) && ds.sprbottomclip == Clip::Null {
        let lo = d.r_plane.lastopening;
        let p = &mut d.r_plane;
        crate::r_plane::reserve_openings(p, n);
        p.openings[lo..lo + n].copy_from_slice(&p.floorclip[start as usize..start as usize + n]);
        d.r_bsp.drawsegs[dsi].sprbottomclip = Clip::Openings(lo as isize - start as isize);
        d.r_plane.lastopening += n;
    }
    let ds = &mut d.r_bsp.drawsegs[dsi];
    if maskedtexture && ds.silhouette & SIL_TOP == 0 {
        ds.silhouette |= SIL_TOP;
        ds.tsilheight = i32::MIN;
    }
    if maskedtexture && ds.silhouette & SIL_BOTTOM == 0 {
        ds.silhouette |= SIL_BOTTOM;
        ds.bsilheight = i32::MAX;
    }
    d.r_bsp.ds_p += 1;
}

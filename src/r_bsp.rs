//! BSP traversal: node order, bounding box checks, and the solid wall clip list.

use crate::i_system;
use crate::m_fixed::Fixed;
use crate::p_setup::{BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP, NF_SUBSECTOR};
use crate::r_main::point_to_angle;
use crate::tables::{Angle, ANG180, ANG90, ANGLETOFINESHIFT};
use crate::Doom;

pub const MAXDRAWSEGS: usize = 256;
const MAXSEGS: usize = crate::i_video::SCREENWIDTH / 2 + 1;

pub const SIL_NONE: i32 = 0;
pub const SIL_BOTTOM: i32 = 1;
pub const SIL_TOP: i32 = 2;
pub const SIL_BOTH: i32 = 3;

/// A reference to a clip array of the original (a short pointer).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Clip {
    #[default]
    Null,
    /// A base offset into `r_plane.openings`. Column x is at base + x.
    Openings(isize),
    NegOne,
    ScreenHeight,
    /// The sprite clip arrays of R_DrawSprite.
    SprBottom,
    SprTop,
}

/// A drawn wall range (drawseg_t).
#[derive(Clone, Copy, Default, Debug)]
pub struct Drawseg {
    pub curline: usize,
    pub x1: i32,
    pub x2: i32,
    pub scale1: Fixed,
    pub scale2: Fixed,
    pub scalestep: Fixed,
    pub silhouette: i32,
    pub bsilheight: Fixed,
    pub tsilheight: Fixed,
    pub sprtopclip: Clip,
    pub sprbottomclip: Clip,
    /// A base offset into `r_plane.openings`, or None.
    pub maskedtexturecol: Option<isize>,
}

#[derive(Clone, Copy, Default)]
pub struct ClipRange {
    pub first: i32,
    pub last: i32,
}

pub struct State {
    pub curline: usize,
    pub sidedef: usize,
    pub linedef: usize,
    pub frontsector: usize,
    pub backsector: Option<usize>,
    pub drawsegs: Vec<Drawseg>,
    pub ds_p: usize,
    pub solidsegs: Vec<ClipRange>,
    pub newend: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            curline: 0,
            sidedef: 0,
            linedef: 0,
            frontsector: 0,
            backsector: None,
            drawsegs: vec![Drawseg::default(); MAXDRAWSEGS],
            ds_p: 0,
            solidsegs: vec![ClipRange::default(); MAXSEGS * 2 + 2],
            newend: 0,
        }
    }
}

/// Returns one value of a clip array.
pub fn clip_at(d: &Doom, c: Clip, x: i32) -> i16 {
    match c {
        Clip::Null => 0,
        Clip::Openings(base) => d.r_plane.openings[(base + x as isize) as usize],
        Clip::NegOne => d.r_things.negonearray[x as usize],
        Clip::ScreenHeight => d.r_things.screenheightarray[x as usize],
        Clip::SprBottom => d.r_things.clipbot[x as usize],
        Clip::SprTop => d.r_things.cliptop[x as usize],
    }
}

/// R_ClearDrawSegs.
pub fn clear_draw_segs(d: &mut Doom) {
    d.r_bsp.ds_p = 0;
}

/// R_ClipSolidWallSegment: draws the visible parts of a solid wall and adds it to the clip list.
fn clip_solid_wall_segment(d: &mut Doom, first: i32, last: i32) {
    let mut start = 0;
    while d.r_bsp.solidsegs[start].last < first - 1 {
        start += 1;
    }
    if first < d.r_bsp.solidsegs[start].first {
        if last < d.r_bsp.solidsegs[start].first - 1 {
            crate::r_segs::store_wall_range(d, first, last);
            let b = &mut d.r_bsp;
            let mut next = b.newend;
            b.newend += 1;
            if b.newend >= b.solidsegs.len() {
                b.solidsegs.push(ClipRange::default());
            }
            while next != start {
                b.solidsegs[next] = b.solidsegs[next - 1];
                next -= 1;
            }
            b.solidsegs[next] = ClipRange { first, last };
            return;
        }
        let f = d.r_bsp.solidsegs[start].first;
        crate::r_segs::store_wall_range(d, first, f - 1);
        d.r_bsp.solidsegs[start].first = first;
    }
    if last <= d.r_bsp.solidsegs[start].last {
        return;
    }
    let mut next = start;
    let mut crunch = false;
    while last >= d.r_bsp.solidsegs[next + 1].first - 1 {
        let (a, b) = (
            d.r_bsp.solidsegs[next].last + 1,
            d.r_bsp.solidsegs[next + 1].first - 1,
        );
        crate::r_segs::store_wall_range(d, a, b);
        next += 1;
        if last <= d.r_bsp.solidsegs[next].last {
            d.r_bsp.solidsegs[start].last = d.r_bsp.solidsegs[next].last;
            crunch = true;
            break;
        }
    }
    if !crunch {
        let a = d.r_bsp.solidsegs[next].last + 1;
        crate::r_segs::store_wall_range(d, a, last);
        d.r_bsp.solidsegs[start].last = last;
    }
    // Remove start+1 to next from the clip list, because start now covers their area.
    if next == start {
        return;
    }
    let b = &mut d.r_bsp;
    loop {
        let cur = next;
        next += 1;
        if cur == b.newend {
            break;
        }
        start += 1;
        b.solidsegs[start] = b.solidsegs[next.min(b.solidsegs.len() - 1)];
    }
    b.newend = start + 1;
}

/// R_ClipPassWallSegment: draws the visible parts of a see-through wall. The clip list does not change.
fn clip_pass_wall_segment(d: &mut Doom, first: i32, last: i32) {
    let mut start = 0;
    while d.r_bsp.solidsegs[start].last < first - 1 {
        start += 1;
    }
    if first < d.r_bsp.solidsegs[start].first {
        if last < d.r_bsp.solidsegs[start].first - 1 {
            crate::r_segs::store_wall_range(d, first, last);
            return;
        }
        let f = d.r_bsp.solidsegs[start].first;
        crate::r_segs::store_wall_range(d, first, f - 1);
    }
    if last <= d.r_bsp.solidsegs[start].last {
        return;
    }
    while last >= d.r_bsp.solidsegs[start + 1].first - 1 {
        let (a, b) = (
            d.r_bsp.solidsegs[start].last + 1,
            d.r_bsp.solidsegs[start + 1].first - 1,
        );
        crate::r_segs::store_wall_range(d, a, b);
        start += 1;
        if last <= d.r_bsp.solidsegs[start].last {
            return;
        }
    }
    let a = d.r_bsp.solidsegs[start].last + 1;
    crate::r_segs::store_wall_range(d, a, last);
}

/// R_ClearClipSegs: the clip list starts with the two sentinel ranges outside the view.
pub fn clear_clip_segs(d: &mut Doom) {
    let vw = d.r_draw.viewwidth;
    let b = &mut d.r_bsp;
    b.solidsegs[0] = ClipRange {
        first: -0x7fff_ffff,
        last: -1,
    };
    b.solidsegs[1] = ClipRange {
        first: vw,
        last: 0x7fff_ffff,
    };
    b.newend = 2;
}

/// Clips an angle pair to the view. Returns None when the range is outside the view.
fn clip_angles(
    clipangle: Angle,
    mut angle1: Angle,
    mut angle2: Angle,
    span: Angle,
) -> Option<(Angle, Angle)> {
    let mut tspan = angle1.wrapping_add(clipangle);
    if tspan > clipangle.wrapping_mul(2) {
        tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
        if tspan >= span {
            return None;
        }
        angle1 = clipangle;
    }
    tspan = clipangle.wrapping_sub(angle2);
    if tspan > clipangle.wrapping_mul(2) {
        tspan = tspan.wrapping_sub(clipangle.wrapping_mul(2));
        if tspan >= span {
            return None;
        }
        angle2 = clipangle.wrapping_neg();
    }
    Some((angle1, angle2))
}

/// R_AddLine: clips a seg to the view and sends it to the wall clipper.
fn add_line(d: &mut Doom, line: usize) {
    d.r_bsp.curline = line;
    let seg = d.p_setup.segs[line];
    let v1 = d.p_setup.vertexes[seg.v1];
    let v2 = d.p_setup.vertexes[seg.v2];
    let angle1 = point_to_angle(d, v1.x, v1.y);
    let angle2 = point_to_angle(d, v2.x, v2.y);
    let span = angle1.wrapping_sub(angle2);
    if span >= ANG180 {
        return;
    }
    d.r_segs.rw_angle1 = angle1;
    let viewangle = d.r_main.viewangle;
    let Some((a1, a2)) = clip_angles(
        d.r_main.clipangle,
        angle1.wrapping_sub(viewangle),
        angle2.wrapping_sub(viewangle),
        span,
    ) else {
        return;
    };
    let a1 = (a1.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize;
    let a2 = (a2.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize;
    let x1 = d.r_main.viewangletox[a1];
    let x2 = d.r_main.viewangletox[a2];
    if x1 == x2 {
        return;
    }
    d.r_bsp.backsector = seg.backsector;
    let Some(back) = seg.backsector else {
        clip_solid_wall_segment(d, x1, x2 - 1);
        return;
    };
    let fs = &d.p_setup.sectors[d.r_bsp.frontsector];
    let bs = &d.p_setup.sectors[back];
    if bs.ceilingheight <= fs.floorheight || bs.floorheight >= fs.ceilingheight {
        clip_solid_wall_segment(d, x1, x2 - 1);
        return;
    }
    let window = bs.ceilingheight != fs.ceilingheight || bs.floorheight != fs.floorheight;
    let empty = bs.ceilingpic == fs.ceilingpic
        && bs.floorpic == fs.floorpic
        && bs.lightlevel == fs.lightlevel
        && d.p_setup.sides[seg.sidedef].midtexture == 0;
    if !window && empty {
        return;
    }
    clip_pass_wall_segment(d, x1, x2 - 1);
}

const CHECKCOORD: [[usize; 4]; 12] = [
    [3, 0, 2, 1],
    [3, 0, 2, 0],
    [3, 1, 2, 0],
    [0, 0, 0, 0],
    [2, 0, 2, 1],
    [0, 0, 0, 0],
    [3, 1, 3, 0],
    [0, 0, 0, 0],
    [2, 0, 3, 1],
    [2, 1, 3, 1],
    [2, 1, 3, 0],
    [0, 0, 0, 0],
];

/// R_CheckBBox: returns true when some part of the box can be visible.
fn check_bbox(d: &Doom, bspcoord: &[Fixed; 4]) -> bool {
    let (viewx, viewy) = (d.r_main.viewx, d.r_main.viewy);
    let boxx = if viewx <= bspcoord[BOXLEFT] {
        0
    } else if viewx < bspcoord[BOXRIGHT] {
        1
    } else {
        2
    };
    let boxy = if viewy >= bspcoord[BOXTOP] {
        0
    } else if viewy > bspcoord[BOXBOTTOM] {
        1
    } else {
        2
    };
    let boxpos = (boxy << 2) + boxx;
    if boxpos == 5 {
        return true;
    }
    let c = CHECKCOORD[boxpos];
    let (x1, y1, x2, y2) = (
        bspcoord[c[0]],
        bspcoord[c[1]],
        bspcoord[c[2]],
        bspcoord[c[3]],
    );
    let viewangle = d.r_main.viewangle;
    let angle1 = point_to_angle(d, x1, y1).wrapping_sub(viewangle);
    let angle2 = point_to_angle(d, x2, y2).wrapping_sub(viewangle);
    let span = angle1.wrapping_sub(angle2);
    if span >= ANG180 {
        return true;
    }
    let Some((a1, a2)) = clip_angles(d.r_main.clipangle, angle1, angle2, span) else {
        return false;
    };
    let a1 = (a1.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize;
    let a2 = (a2.wrapping_add(ANG90) >> ANGLETOFINESHIFT) as usize;
    let sx1 = d.r_main.viewangletox[a1];
    let mut sx2 = d.r_main.viewangletox[a2];
    if sx1 == sx2 {
        return false;
    }
    sx2 -= 1;
    let mut start = 0;
    while d.r_bsp.solidsegs[start].last < sx2 {
        start += 1;
    }
    !(sx1 >= d.r_bsp.solidsegs[start].first && sx2 <= d.r_bsp.solidsegs[start].last)
}

/// R_Subsector: finds the planes, adds the sprites, and adds the segs of one subsector.
fn subsector(d: &mut Doom, num: usize) {
    let numss = d.p_setup.subsectors.len();
    if num >= numss {
        i_system::error(
            d,
            &format!("R_Subsector: ss {} with numss = {}", num, numss),
        );
    }
    d.r_main.sscount += 1;
    let sub = d.p_setup.subsectors[num];
    let fs = sub.sector;
    d.r_bsp.frontsector = fs;
    let sec = &d.p_setup.sectors[fs];
    let (fh, fpic, ch, cpic, light) = (
        sec.floorheight,
        sec.floorpic,
        sec.ceilingheight,
        sec.ceilingpic,
        sec.lightlevel,
    );
    let viewz = d.r_main.viewz;
    d.r_plane.floorplane = if fh < viewz {
        Some(crate::r_plane::find_plane(d, fh, fpic, light))
    } else {
        None
    };
    d.r_plane.ceilingplane = if ch > viewz || cpic == d.r_sky.skyflatnum {
        Some(crate::r_plane::find_plane(d, ch, cpic, light))
    } else {
        None
    };
    crate::r_things::add_sprites(d, fs);
    for line in sub.firstline..sub.firstline + sub.numlines {
        add_line(d, line);
    }
}

/// R_RenderBSPNode: walks the BSP tree front to back.
pub fn render_bsp_node(d: &mut Doom, bspnum: i32) {
    if bspnum & NF_SUBSECTOR as i32 != 0 {
        if bspnum == -1 {
            subsector(d, 0);
        } else {
            subsector(d, (bspnum & !(NF_SUBSECTOR as i32)) as usize);
        }
        return;
    }
    let node = d.p_setup.nodes[bspnum as usize];
    let side = crate::r_main::point_on_side(d.r_main.viewx, d.r_main.viewy, &node) as usize;
    render_bsp_node(d, node.children[side] as i32);
    if check_bbox(d, &node.bbox[side ^ 1]) {
        render_bsp_node(d, node.children[side ^ 1] as i32);
    }
}

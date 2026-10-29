//! Map utilities: distance, line sides, line openings, thing links, block iterators, and path traversal.

use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::p_mobj::{MobjId, MF_NOBLOCKMAP, MF_NOSECTOR};
use crate::p_setup::{SlopeType, BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP, MAPBLOCKSHIFT};
use crate::Doom;

pub const PT_ADDLINES: i32 = 1;
pub const PT_ADDTHINGS: i32 = 2;
pub const PT_EARLYOUT: i32 = 4;
const MAPBLOCKSIZE: Fixed = 128 * FRACUNIT;
const MAPBTOFRAC: i32 = MAPBLOCKSHIFT - FRACBITS;
const MAXINTERCEPTS_ORIGINAL: usize = 128;

#[derive(Clone, Copy, Default, Debug)]
pub struct DivLine {
    pub x: Fixed,
    pub y: Fixed,
    pub dx: Fixed,
    pub dy: Fixed,
}

/// One path crossing. `d` is a line index when `isaline` is true, else a map object id.
#[derive(Clone, Copy, Default, Debug)]
pub struct Intercept {
    pub frac: Fixed,
    pub isaline: bool,
    pub d: usize,
}

#[derive(Default)]
pub struct State {
    pub opentop: Fixed,
    pub openbottom: Fixed,
    pub openrange: Fixed,
    pub lowfloor: Fixed,
    pub intercepts: Vec<Intercept>,
    pub trace: DivLine,
    pub earlyout: bool,
}

/// P_AproxDistance.
pub fn approx_distance(dx: Fixed, dy: Fixed) -> Fixed {
    let dx = dx.wrapping_abs();
    let dy = dy.wrapping_abs();
    if dx < dy {
        dx.wrapping_add(dy).wrapping_sub(dx >> 1)
    } else {
        dx.wrapping_add(dy).wrapping_sub(dy >> 1)
    }
}

/// P_PointOnLineSide.
pub fn point_on_line_side(d: &Doom, x: Fixed, y: Fixed, line: usize) -> i32 {
    let l = &d.p_setup.lines[line];
    let v1 = d.p_setup.vertexes[l.v1];
    if l.dx == 0 {
        return if x <= v1.x {
            (l.dy > 0) as i32
        } else {
            (l.dy < 0) as i32
        };
    }
    if l.dy == 0 {
        return if y <= v1.y {
            (l.dx < 0) as i32
        } else {
            (l.dx > 0) as i32
        };
    }
    let dx = x.wrapping_sub(v1.x);
    let dy = y.wrapping_sub(v1.y);
    let left = fixed_mul(l.dy >> FRACBITS, dx);
    let right = fixed_mul(dy, l.dx >> FRACBITS);
    if right < left {
        0
    } else {
        1
    }
}

/// P_BoxOnLineSide. Returns 0 or 1 for one side, or -1 when the box crosses the line.
pub fn box_on_line_side(d: &Doom, tmbox: &[Fixed; 4], line: usize) -> i32 {
    let l = &d.p_setup.lines[line];
    let v1 = d.p_setup.vertexes[l.v1];
    let (mut p, mut p2);
    match l.slopetype {
        SlopeType::Horizontal => {
            p = (tmbox[BOXTOP] > v1.y) as i32;
            p2 = (tmbox[BOXBOTTOM] > v1.y) as i32;
            if l.dx < 0 {
                p ^= 1;
                p2 ^= 1;
            }
        }
        SlopeType::Vertical => {
            p = (tmbox[BOXRIGHT] < v1.x) as i32;
            p2 = (tmbox[BOXLEFT] < v1.x) as i32;
            if l.dy < 0 {
                p ^= 1;
                p2 ^= 1;
            }
        }
        SlopeType::Positive => {
            p = point_on_line_side(d, tmbox[BOXLEFT], tmbox[BOXTOP], line);
            p2 = point_on_line_side(d, tmbox[BOXRIGHT], tmbox[BOXBOTTOM], line);
        }
        SlopeType::Negative => {
            p = point_on_line_side(d, tmbox[BOXRIGHT], tmbox[BOXTOP], line);
            p2 = point_on_line_side(d, tmbox[BOXLEFT], tmbox[BOXBOTTOM], line);
        }
    }
    if p == p2 {
        p
    } else {
        -1
    }
}

/// P_PointOnDivlineSide.
pub fn point_on_divline_side(x: Fixed, y: Fixed, line: &DivLine) -> i32 {
    if line.dx == 0 {
        return if x <= line.x {
            (line.dy > 0) as i32
        } else {
            (line.dy < 0) as i32
        };
    }
    if line.dy == 0 {
        return if y <= line.y {
            (line.dx < 0) as i32
        } else {
            (line.dx > 0) as i32
        };
    }
    let dx = x.wrapping_sub(line.x);
    let dy = y.wrapping_sub(line.y);
    if (line.dy ^ line.dx ^ dx ^ dy) < 0 {
        return if (line.dy ^ dx) < 0 { 1 } else { 0 };
    }
    let left = fixed_mul(line.dy >> 8, dx >> 8);
    let right = fixed_mul(dy >> 8, line.dx >> 8);
    if right < left {
        0
    } else {
        1
    }
}

/// P_MakeDivline.
pub fn make_divline(d: &Doom, line: usize) -> DivLine {
    let l = &d.p_setup.lines[line];
    let v1 = d.p_setup.vertexes[l.v1];
    DivLine {
        x: v1.x,
        y: v1.y,
        dx: l.dx,
        dy: l.dy,
    }
}

/// P_InterceptVector.
pub fn intercept_vector(v2: &DivLine, v1: &DivLine) -> Fixed {
    let den = fixed_mul(v1.dy >> 8, v2.dx).wrapping_sub(fixed_mul(v1.dx >> 8, v2.dy));
    if den == 0 {
        return 0;
    }
    let num = fixed_mul(v1.x.wrapping_sub(v2.x) >> 8, v1.dy)
        .wrapping_add(fixed_mul(v2.y.wrapping_sub(v1.y) >> 8, v1.dx));
    fixed_div(num, den)
}

/// P_LineOpening. A one-sided line sets only `openrange`.
pub fn line_opening(d: &mut Doom, line: usize) {
    let l = &d.p_setup.lines[line];
    if l.sidenum[1] == -1 {
        d.p_maputl.openrange = 0;
        return;
    }
    let (Some(f), Some(b)) = (l.frontsector, l.backsector) else {
        d.p_maputl.openrange = 0;
        return;
    };
    let front = &d.p_setup.sectors[f];
    let back = &d.p_setup.sectors[b];
    let s = &mut d.p_maputl;
    s.opentop = front.ceilingheight.min(back.ceilingheight);
    if front.floorheight > back.floorheight {
        s.openbottom = front.floorheight;
        s.lowfloor = back.floorheight;
    } else {
        s.openbottom = back.floorheight;
        s.lowfloor = front.floorheight;
    }
    s.openrange = s.opentop.wrapping_sub(s.openbottom);
}

fn block_of(d: &Doom, x: Fixed, y: Fixed) -> Option<usize> {
    let s = &d.p_setup;
    let bx = x.wrapping_sub(s.bmaporgx) >> MAPBLOCKSHIFT;
    let by = y.wrapping_sub(s.bmaporgy) >> MAPBLOCKSHIFT;
    if bx >= 0 && bx < s.bmapwidth && by >= 0 && by < s.bmapheight {
        Some((by * s.bmapwidth + bx) as usize)
    } else {
        None
    }
}

/// P_UnsetThingPosition. The object keeps its own links, as in the original.
pub fn unset_thing_position(d: &mut Doom, thing: MobjId) {
    let m = d.p_tick.mobj(thing);
    let (flags, snext, sprev, bnext, bprev, ss, x, y) = (
        m.flags,
        m.snext,
        m.sprev,
        m.bnext,
        m.bprev,
        m.subsector,
        m.x,
        m.y,
    );
    if flags & MF_NOSECTOR == 0 {
        if let Some(n) = snext {
            d.p_tick.mobj_mut(n).sprev = sprev;
        }
        if let Some(p) = sprev {
            d.p_tick.mobj_mut(p).snext = snext;
        } else {
            let sec = d.p_setup.subsectors[ss].sector;
            d.p_setup.sectors[sec].thinglist = snext;
        }
    }
    if flags & MF_NOBLOCKMAP == 0 {
        if let Some(n) = bnext {
            d.p_tick.mobj_mut(n).bprev = bprev;
        }
        if let Some(p) = bprev {
            d.p_tick.mobj_mut(p).bnext = bnext;
        } else if let Some(b) = block_of(d, x, y) {
            d.p_setup.blocklinks[b] = bnext;
        }
    }
}

/// P_SetThingPosition.
pub fn set_thing_position(d: &mut Doom, thing: MobjId) {
    let (x, y, flags) = {
        let m = d.p_tick.mobj(thing);
        (m.x, m.y, m.flags)
    };
    let ss = crate::r_main::point_in_subsector(d, x, y);
    d.p_tick.mobj_mut(thing).subsector = ss;
    if flags & MF_NOSECTOR == 0 {
        let sec = d.p_setup.subsectors[ss].sector;
        let head = d.p_setup.sectors[sec].thinglist;
        let m = d.p_tick.mobj_mut(thing);
        m.sprev = None;
        m.snext = head;
        if let Some(h) = head {
            d.p_tick.mobj_mut(h).sprev = Some(thing);
        }
        d.p_setup.sectors[sec].thinglist = Some(thing);
    }
    if flags & MF_NOBLOCKMAP == 0 {
        if let Some(b) = block_of(d, x, y) {
            let head = d.p_setup.blocklinks[b];
            let m = d.p_tick.mobj_mut(thing);
            m.bprev = None;
            m.bnext = head;
            if let Some(h) = head {
                d.p_tick.mobj_mut(h).bprev = Some(thing);
            }
            d.p_setup.blocklinks[b] = Some(thing);
        } else {
            let m = d.p_tick.mobj_mut(thing);
            m.bnext = None;
            m.bprev = None;
        }
    }
}

/// P_BlockLinesIterator. Each line runs once per validcount.
pub fn block_lines_iterator(
    d: &mut Doom,
    x: i32,
    y: i32,
    func: fn(&mut Doom, usize) -> bool,
) -> bool {
    let s = &d.p_setup;
    if x < 0 || y < 0 || x >= s.bmapwidth || y >= s.bmapheight {
        return true;
    }
    let mut list = s.blockmaplump[s.blockmap + (y * s.bmapwidth + x) as usize] as u16 as usize;
    loop {
        let entry = d.p_setup.blockmaplump[list];
        if entry == -1 {
            return true;
        }
        let ld = entry as u16 as usize;
        list += 1;
        if d.p_setup.lines[ld].validcount == d.r_main.validcount {
            continue;
        }
        d.p_setup.lines[ld].validcount = d.r_main.validcount;
        if !func(d, ld) {
            return false;
        }
    }
}

/// P_BlockThingsIterator. The next link is read after the call, as in the original.
pub fn block_things_iterator(
    d: &mut Doom,
    x: i32,
    y: i32,
    func: fn(&mut Doom, MobjId) -> bool,
) -> bool {
    let s = &d.p_setup;
    if x < 0 || y < 0 || x >= s.bmapwidth || y >= s.bmapheight {
        return true;
    }
    let mut cur = s.blocklinks[(y * s.bmapwidth + x) as usize];
    while let Some(m) = cur {
        if !func(d, m) {
            return false;
        }
        cur = d.p_tick.mobj(m).bnext;
    }
    true
}

/// InterceptsMemoryOverrun: writes a value into the globals that follow the original intercepts array.
fn intercepts_memory_overrun(d: &mut Doom, location: usize, value: i32) {
    // (length, target) in the original memory order after intercepts[128].
    const LAYOUT: [usize; 22] = [
        4, 4, 4, 4, 4, 4, 4, 4, 120, 8, 4, 4, 4, 4, 40, 4, 4, 4, 4, 4, 4, 4,
    ];
    let mut offset = 0;
    for (i, &len) in LAYOUT.iter().enumerate() {
        if offset + len > location {
            let rel = location - offset;
            match i {
                3 => d.p_maputl.lowfloor = value,
                4 => d.p_maputl.openbottom = value,
                5 => d.p_maputl.opentop = value,
                6 => d.p_maputl.openrange = value,
                10 => d.p_pspr.bulletslope = value,
                14 => {
                    // playerstarts is an array of 16-bit fields: five per map thing.
                    let index = rel / 2;
                    for (k, v) in [(index, value as i16), (index + 1, (value >> 16) as i16)] {
                        if k < 20 {
                            let t = &mut d.p_setup.playerstarts[k / 5];
                            match k % 5 {
                                0 => t.x = v,
                                1 => t.y = v,
                                2 => t.angle = v,
                                3 => t.type_ = v,
                                _ => t.options = v,
                            }
                        }
                    }
                }
                16 => d.p_setup.bmapwidth = value,
                18 => d.p_setup.bmaporgx = value,
                19 => d.p_setup.bmaporgy = value,
                21 => d.p_setup.bmapheight = value,
                _ => {}
            }
            return;
        }
        offset += len;
    }
}

/// InterceptsOverrun: emulates the original writes past 128 intercepts.
fn intercepts_overrun(d: &mut Doom, num: usize, ic: Intercept) {
    if num <= MAXINTERCEPTS_ORIGINAL {
        return;
    }
    let location = (num - MAXINTERCEPTS_ORIGINAL - 1) * 12;
    intercepts_memory_overrun(d, location, ic.frac);
    intercepts_memory_overrun(d, location + 4, ic.isaline as i32);
    // The original writes a pointer here; this build writes the index.
    intercepts_memory_overrun(d, location + 8, ic.d as i32);
}

fn add_intercept(d: &mut Doom, ic: Intercept) {
    let n = d.p_maputl.intercepts.len();
    d.p_maputl.intercepts.push(ic);
    intercepts_overrun(d, n, ic);
}

/// PIT_AddLineIntercepts.
pub fn pit_add_line_intercepts(d: &mut Doom, ld: usize) -> bool {
    let trace = d.p_maputl.trace;
    let (s1, s2);
    if trace.dx > FRACUNIT * 16
        || trace.dy > FRACUNIT * 16
        || trace.dx < -FRACUNIT * 16
        || trace.dy < -FRACUNIT * 16
    {
        let l = &d.p_setup.lines[ld];
        let (v1, v2) = (d.p_setup.vertexes[l.v1], d.p_setup.vertexes[l.v2]);
        s1 = point_on_divline_side(v1.x, v1.y, &trace);
        s2 = point_on_divline_side(v2.x, v2.y, &trace);
    } else {
        s1 = point_on_line_side(d, trace.x, trace.y, ld);
        s2 = point_on_line_side(
            d,
            trace.x.wrapping_add(trace.dx),
            trace.y.wrapping_add(trace.dy),
            ld,
        );
    }
    if s1 == s2 {
        return true;
    }
    let dl = make_divline(d, ld);
    let frac = intercept_vector(&trace, &dl);
    if frac < 0 {
        return true;
    }
    if d.p_maputl.earlyout && frac < FRACUNIT && d.p_setup.lines[ld].backsector.is_none() {
        return false;
    }
    add_intercept(
        d,
        Intercept {
            frac,
            isaline: true,
            d: ld,
        },
    );
    true
}

/// PIT_AddThingIntercepts.
pub fn pit_add_thing_intercepts(d: &mut Doom, thing: MobjId) -> bool {
    let trace = d.p_maputl.trace;
    let m = d.p_tick.mobj(thing);
    let tracepositive = (trace.dx ^ trace.dy) > 0;
    let (x1, y1, x2, y2) = if tracepositive {
        (
            m.x.wrapping_sub(m.radius),
            m.y.wrapping_add(m.radius),
            m.x.wrapping_add(m.radius),
            m.y.wrapping_sub(m.radius),
        )
    } else {
        (
            m.x.wrapping_sub(m.radius),
            m.y.wrapping_sub(m.radius),
            m.x.wrapping_add(m.radius),
            m.y.wrapping_add(m.radius),
        )
    };
    let s1 = point_on_divline_side(x1, y1, &trace);
    let s2 = point_on_divline_side(x2, y2, &trace);
    if s1 == s2 {
        return true;
    }
    let dl = DivLine {
        x: x1,
        y: y1,
        dx: x2.wrapping_sub(x1),
        dy: y2.wrapping_sub(y1),
    };
    let frac = intercept_vector(&trace, &dl);
    if frac < 0 {
        return true;
    }
    add_intercept(
        d,
        Intercept {
            frac,
            isaline: false,
            d: thing,
        },
    );
    true
}

/// P_TraverseIntercepts: calls `func` for each intercept in order of distance up to `maxfrac`.
pub fn traverse_intercepts(
    d: &mut Doom,
    func: fn(&mut Doom, &Intercept) -> bool,
    maxfrac: Fixed,
) -> bool {
    let count = d.p_maputl.intercepts.len();
    let mut idx = 0;
    for _ in 0..count {
        let mut dist = i32::MAX;
        for (i, ic) in d.p_maputl.intercepts.iter().enumerate() {
            if ic.frac < dist {
                dist = ic.frac;
                idx = i;
            }
        }
        if dist > maxfrac {
            return true;
        }
        let ic = d.p_maputl.intercepts[idx];
        if !func(d, &ic) {
            return false;
        }
        d.p_maputl.intercepts[idx].frac = i32::MAX;
    }
    true
}

/// P_PathTraverse: walks the blocks from (x1, y1) to (x2, y2), collects intercepts, and runs `trav` on them.
pub fn path_traverse(
    d: &mut Doom,
    x1: Fixed,
    y1: Fixed,
    x2: Fixed,
    y2: Fixed,
    flags: i32,
    trav: fn(&mut Doom, &Intercept) -> bool,
) -> bool {
    let (mut x1, mut y1, mut x2, mut y2) = (x1, y1, x2, y2);
    d.p_maputl.earlyout = flags & PT_EARLYOUT != 0;
    d.r_main.validcount += 1;
    d.p_maputl.intercepts.clear();
    let (orgx, orgy) = (d.p_setup.bmaporgx, d.p_setup.bmaporgy);
    if x1.wrapping_sub(orgx) & (MAPBLOCKSIZE - 1) == 0 {
        x1 = x1.wrapping_add(FRACUNIT);
    }
    if y1.wrapping_sub(orgy) & (MAPBLOCKSIZE - 1) == 0 {
        y1 = y1.wrapping_add(FRACUNIT);
    }
    d.p_maputl.trace = DivLine {
        x: x1,
        y: y1,
        dx: x2.wrapping_sub(x1),
        dy: y2.wrapping_sub(y1),
    };
    x1 = x1.wrapping_sub(orgx);
    y1 = y1.wrapping_sub(orgy);
    let xt1 = x1 >> MAPBLOCKSHIFT;
    let yt1 = y1 >> MAPBLOCKSHIFT;
    x2 = x2.wrapping_sub(orgx);
    y2 = y2.wrapping_sub(orgy);
    let xt2 = x2 >> MAPBLOCKSHIFT;
    let yt2 = y2 >> MAPBLOCKSHIFT;

    let (mapxstep, mut partial, ystep);
    if xt2 > xt1 {
        mapxstep = 1;
        partial = FRACUNIT - ((x1 >> MAPBTOFRAC) & (FRACUNIT - 1));
        ystep = fixed_div(y2.wrapping_sub(y1), x2.wrapping_sub(x1).wrapping_abs());
    } else if xt2 < xt1 {
        mapxstep = -1;
        partial = (x1 >> MAPBTOFRAC) & (FRACUNIT - 1);
        ystep = fixed_div(y2.wrapping_sub(y1), x2.wrapping_sub(x1).wrapping_abs());
    } else {
        mapxstep = 0;
        partial = FRACUNIT;
        ystep = 256 * FRACUNIT;
    }
    let mut yintercept = (y1 >> MAPBTOFRAC).wrapping_add(fixed_mul(partial, ystep));

    let (mapystep, xstep);
    if yt2 > yt1 {
        mapystep = 1;
        partial = FRACUNIT - ((y1 >> MAPBTOFRAC) & (FRACUNIT - 1));
        xstep = fixed_div(x2.wrapping_sub(x1), y2.wrapping_sub(y1).wrapping_abs());
    } else if yt2 < yt1 {
        mapystep = -1;
        partial = (y1 >> MAPBTOFRAC) & (FRACUNIT - 1);
        xstep = fixed_div(x2.wrapping_sub(x1), y2.wrapping_sub(y1).wrapping_abs());
    } else {
        mapystep = 0;
        partial = FRACUNIT;
        xstep = 256 * FRACUNIT;
    }
    let mut xintercept = (x1 >> MAPBTOFRAC).wrapping_add(fixed_mul(partial, xstep));

    let mut mapx = xt1;
    let mut mapy = yt1;
    for _ in 0..64 {
        if flags & PT_ADDLINES != 0 && !block_lines_iterator(d, mapx, mapy, pit_add_line_intercepts)
        {
            return false;
        }
        if flags & PT_ADDTHINGS != 0
            && !block_things_iterator(d, mapx, mapy, pit_add_thing_intercepts)
        {
            return false;
        }
        if mapx == xt2 && mapy == yt2 {
            break;
        }
        if (yintercept >> FRACBITS) == mapy {
            yintercept = yintercept.wrapping_add(ystep);
            mapx += mapxstep;
        } else if (xintercept >> FRACBITS) == mapx {
            xintercept = xintercept.wrapping_add(xstep);
            mapy += mapystep;
        }
    }
    traverse_intercepts(d, trav, FRACUNIT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_distance_matches_the_octagon_formula() {
        assert_eq!(
            approx_distance(3 * FRACUNIT, 4 * FRACUNIT),
            3 * FRACUNIT + 4 * FRACUNIT - ((3 * FRACUNIT) >> 1)
        );
        assert_eq!(approx_distance(-8, 2), 8 + 2 - 1);
    }

    #[test]
    fn divline_side_uses_sign_bits() {
        let l = DivLine {
            x: 0,
            y: 0,
            dx: FRACUNIT,
            dy: FRACUNIT,
        };
        assert_eq!(point_on_divline_side(-FRACUNIT, FRACUNIT, &l), 1);
        assert_eq!(point_on_divline_side(FRACUNIT, -FRACUNIT, &l), 0);
    }
}

//! Line of sight: the reject table and a BSP walk from the looker to the target.

use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS};
use crate::p_maputl::DivLine;
use crate::p_mobj::MobjId;
use crate::p_setup::{ML_TWOSIDED, NF_SUBSECTOR};
use crate::Doom;

#[derive(Default)]
pub struct State {
    pub sightzstart: Fixed,
    /// The slopes to the top and the bottom of the target. P_AimLineAttack also uses them.
    pub topslope: Fixed,
    pub bottomslope: Fixed,
    pub strace: DivLine,
    pub t2x: Fixed,
    pub t2y: Fixed,
    pub sightcounts: [i32; 2],
}

/// P_DivlineSide. Returns 0 (front), 1 (back), or 2 (on the line).
/// The horizontal case compares x with node.y, as the original does.
fn divline_side(x: Fixed, y: Fixed, node: &DivLine) -> i32 {
    if node.dx == 0 {
        if x == node.x {
            return 2;
        }
        return if x <= node.x {
            (node.dy > 0) as i32
        } else {
            (node.dy < 0) as i32
        };
    }
    if node.dy == 0 {
        if x == node.y {
            return 2;
        }
        return if y <= node.y {
            (node.dx < 0) as i32
        } else {
            (node.dx > 0) as i32
        };
    }
    let dx = x.wrapping_sub(node.x);
    let dy = y.wrapping_sub(node.y);
    let left = (node.dy >> FRACBITS).wrapping_mul(dx >> FRACBITS);
    let right = (dy >> FRACBITS).wrapping_mul(node.dx >> FRACBITS);
    if right < left {
        0
    } else if left == right {
        2
    } else {
        1
    }
}

/// P_InterceptVector2.
fn intercept_vector2(v2: &DivLine, v1: &DivLine) -> Fixed {
    let den = fixed_mul(v1.dy >> 8, v2.dx).wrapping_sub(fixed_mul(v1.dx >> 8, v2.dy));
    if den == 0 {
        return 0;
    }
    let num = fixed_mul(v1.x.wrapping_sub(v2.x) >> 8, v1.dy)
        .wrapping_add(fixed_mul(v2.y.wrapping_sub(v1.y) >> 8, v1.dx));
    fixed_div(num, den)
}

/// P_CrossSubsector: returns false when a line in the subsector blocks the sight line.
fn cross_subsector(d: &mut Doom, num: usize) -> bool {
    let numss = d.p_setup.subsectors.len();
    if num >= numss {
        let msg = format!("P_CrossSubsector: ss {} with numss = {}", num, numss);
        crate::i_system::error(d, &msg);
    }
    let sub = d.p_setup.subsectors[num];
    let validcount = d.r_main.validcount;
    let strace = d.p_sight.strace;
    for segi in sub.firstline..sub.firstline + sub.numlines {
        let seg = d.p_setup.segs[segi];
        let line = &mut d.p_setup.lines[seg.linedef];
        if line.validcount == validcount {
            continue;
        }
        line.validcount = validcount;
        let line = *line;
        let v1 = d.p_setup.vertexes[line.v1];
        let v2 = d.p_setup.vertexes[line.v2];
        let s1 = divline_side(v1.x, v1.y, &strace);
        let s2 = divline_side(v2.x, v2.y, &strace);
        if s1 == s2 {
            continue;
        }
        let divl = DivLine {
            x: v1.x,
            y: v1.y,
            dx: v2.x.wrapping_sub(v1.x),
            dy: v2.y.wrapping_sub(v1.y),
        };
        let s1 = divline_side(strace.x, strace.y, &divl);
        let s2 = divline_side(d.p_sight.t2x, d.p_sight.t2y, &divl);
        if s1 == s2 {
            continue;
        }
        if line.backsector.is_none() {
            return false;
        }
        if line.flags & ML_TWOSIDED == 0 {
            return false;
        }
        let Some(back) = seg.backsector else {
            return false;
        };
        let front = &d.p_setup.sectors[seg.frontsector];
        let back = &d.p_setup.sectors[back];
        if front.floorheight == back.floorheight && front.ceilingheight == back.ceilingheight {
            continue;
        }
        let opentop = front.ceilingheight.min(back.ceilingheight);
        let openbottom = front.floorheight.max(back.floorheight);
        if openbottom >= opentop {
            return false;
        }
        let frac = intercept_vector2(&strace, &divl);
        let s = &mut d.p_sight;
        if front.floorheight != back.floorheight {
            let slope = fixed_div(openbottom.wrapping_sub(s.sightzstart), frac);
            if slope > s.bottomslope {
                s.bottomslope = slope;
            }
        }
        if front.ceilingheight != back.ceilingheight {
            let slope = fixed_div(opentop.wrapping_sub(s.sightzstart), frac);
            if slope < s.topslope {
                s.topslope = slope;
            }
        }
        if s.topslope <= s.bottomslope {
            return false;
        }
    }
    true
}

/// P_CrossBSPNode: returns true when the sight line crosses the node subtree.
fn cross_bsp_node(d: &mut Doom, bspnum: i32) -> bool {
    if bspnum & NF_SUBSECTOR as i32 != 0 {
        if bspnum == -1 {
            return cross_subsector(d, 0);
        }
        return cross_subsector(d, (bspnum & !(NF_SUBSECTOR as i32)) as usize);
    }
    let bsp = d.p_setup.nodes[bspnum as usize];
    let dl = DivLine {
        x: bsp.x,
        y: bsp.y,
        dx: bsp.dx,
        dy: bsp.dy,
    };
    let strace = d.p_sight.strace;
    let mut side = divline_side(strace.x, strace.y, &dl);
    if side == 2 {
        side = 0;
    }
    if !cross_bsp_node(d, bsp.children[side as usize] as i32) {
        return false;
    }
    if side == divline_side(d.p_sight.t2x, d.p_sight.t2y, &dl) {
        return true;
    }
    cross_bsp_node(d, bsp.children[(side ^ 1) as usize] as i32)
}

/// P_CheckSight: returns true when `t1` can see `t2`.
pub fn check_sight(d: &mut Doom, t1: MobjId, t2: MobjId) -> bool {
    let a = d.p_tick.mobj(t1).clone();
    let b = d.p_tick.mobj(t2).clone();
    let s1 = d.p_setup.subsectors[a.subsector].sector;
    let s2 = d.p_setup.subsectors[b.subsector].sector;
    let pnum = s1 * d.p_setup.numsectors + s2;
    let bytenum = pnum >> 3;
    let bitnum = 1u8 << (pnum & 7);
    if d.p_setup.rejectmatrix.get(bytenum).copied().unwrap_or(0) & bitnum != 0 {
        d.p_sight.sightcounts[0] += 1;
        return false;
    }
    d.p_sight.sightcounts[1] += 1;
    d.r_main.validcount += 1;
    let s = &mut d.p_sight;
    s.sightzstart = a.z.wrapping_add(a.height).wrapping_sub(a.height >> 2);
    s.topslope = b.z.wrapping_add(b.height).wrapping_sub(s.sightzstart);
    s.bottomslope = b.z.wrapping_sub(s.sightzstart);
    s.strace = DivLine {
        x: a.x,
        y: a.y,
        dx: b.x.wrapping_sub(a.x),
        dy: b.y.wrapping_sub(a.y),
    };
    s.t2x = b.x;
    s.t2y = b.y;
    let root = d.p_setup.nodes.len() as i32 - 1;
    cross_bsp_node(d, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divline_side_keeps_the_horizontal_quirk() {
        let node = DivLine {
            x: 0,
            y: 5,
            dx: 10,
            dy: 0,
        };
        // x equals node.y, so the original reports "on the line".
        assert_eq!(divline_side(5, 100, &node), 2);
        assert_eq!(divline_side(6, 100, &node), 1);
    }
}

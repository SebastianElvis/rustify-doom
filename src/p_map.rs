//! Movement and collision: position checks, moves, slides, aiming, shooting, use, radius damage, and sector changes.

use crate::info::MOBJINFO;
use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::m_random::p_random;
use crate::p_maputl::{
    approx_distance, block_lines_iterator, block_things_iterator, box_on_line_side, line_opening,
    path_traverse, point_on_line_side, set_thing_position, unset_thing_position, Intercept,
    PT_ADDLINES, PT_ADDTHINGS,
};
use crate::p_mobj::*;
use crate::p_setup::{
    SlopeType, BOXBOTTOM, BOXLEFT, BOXRIGHT, BOXTOP, MAPBLOCKSHIFT, MAXRADIUS, ML_BLOCKING,
    ML_BLOCKMONSTERS, ML_TWOSIDED,
};
use crate::tables::{finecosine, finesine, Angle, ANG180, ANGLETOFINESHIFT};
use crate::Doom;

pub const MELEERANGE: Fixed = 64 * FRACUNIT;
pub const MISSILERANGE: Fixed = 32 * 64 * FRACUNIT;
pub const USERANGE: Fixed = 64 * FRACUNIT;
const MAXSPECIALCROSS_ORIGINAL: i32 = 8;
const DEFAULT_SPECHIT_MAGIC: u32 = 0x01C09C98;

#[derive(Default)]
pub struct State {
    pub tmbbox: [Fixed; 4],
    pub tmthing: Option<MobjId>,
    pub tmflags: i32,
    pub tmx: Fixed,
    pub tmy: Fixed,
    pub floatok: bool,
    pub tmfloorz: Fixed,
    pub tmceilingz: Fixed,
    pub tmdropoffz: Fixed,
    pub ceilingline: Option<usize>,
    pub spechit: Vec<usize>,
    pub numspechit: i32,
    pub spechit_baseaddr: u32,
    pub bestslidefrac: Fixed,
    pub secondslidefrac: Fixed,
    pub bestslideline: Option<usize>,
    pub secondslideline: Option<usize>,
    pub slidemo: Option<MobjId>,
    pub tmxmove: Fixed,
    pub tmymove: Fixed,
    pub linetarget: Option<MobjId>,
    pub shootthing: Option<MobjId>,
    pub shootz: Fixed,
    pub la_damage: i32,
    pub attackrange: Fixed,
    pub aimslope: Fixed,
    pub usething: Option<MobjId>,
    pub bombsource: Option<MobjId>,
    pub bombspot: Option<MobjId>,
    pub bombdamage: i32,
    /// Booleans in the original; the spechit overrun can write any value.
    pub crushchange: i32,
    pub nofit: i32,
}

fn mo(d: &Doom, id: MobjId) -> &Mobj {
    d.p_tick.mobj(id)
}

fn mo_mut(d: &mut Doom, id: MobjId) -> &mut Mobj {
    d.p_tick.mobj_mut(id)
}

fn sub_random(d: &mut Doom) -> i32 {
    let r = p_random(&mut d.m_random);
    r - p_random(&mut d.m_random)
}

fn tmthing(d: &Doom) -> MobjId {
    d.p_map.tmthing.expect("tmthing")
}

/// SpechitOverrun: emulates the original writes past 8 special lines.
fn spechit_overrun(d: &mut Doom, ld: usize) {
    if d.p_map.spechit_baseaddr == 0 {
        let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-spechit", 1);
        d.p_map.spechit_baseaddr = if p > 0 {
            crate::m_misc::str_to_int(&d.m_argv.myargv[p + 1]).unwrap_or(0) as u32
        } else {
            DEFAULT_SPECHIT_MAGIC
        };
    }
    let addr = d
        .p_map
        .spechit_baseaddr
        .wrapping_add((ld as u32).wrapping_mul(0x3E)) as i32;
    let n = d.p_map.numspechit;
    match n {
        9..=12 => d.p_map.tmbbox[(n - 9) as usize] = addr,
        13 => d.p_map.crushchange = addr,
        14 => d.p_map.nofit = addr,
        _ => eprintln!(
            "SpechitOverrun: Warning: unable to emulatean overrun where numspechit={}",
            n
        ),
    }
}

/// PIT_StompThing.
pub fn pit_stomp_thing(d: &mut Doom, thing: MobjId) -> bool {
    let t = mo(d, thing);
    if t.flags & MF_SHOOTABLE == 0 {
        return true;
    }
    let tm = tmthing(d);
    let tmo = mo(d, tm);
    let blockdist = t.radius.wrapping_add(tmo.radius);
    if t.x.wrapping_sub(d.p_map.tmx).wrapping_abs() >= blockdist
        || t.y.wrapping_sub(d.p_map.tmy).wrapping_abs() >= blockdist
    {
        return true;
    }
    if thing == tm {
        return true;
    }
    if tmo.player.is_none() && d.g_game.gamemap != 30 {
        return false;
    }
    crate::p_inter::damage_mobj(d, thing, Some(tm), Some(tm), 10000);
    true
}

/// Sets the globals shared by P_TeleportMove and P_CheckPosition.
fn start_check(d: &mut Doom, thing: MobjId, x: Fixed, y: Fixed) {
    let (flags, radius) = (mo(d, thing).flags, mo(d, thing).radius);
    let ss = crate::r_main::point_in_subsector(d, x, y);
    let sec = &d.p_setup.sectors[d.p_setup.subsectors[ss].sector];
    let (fh, ch) = (sec.floorheight, sec.ceilingheight);
    let s = &mut d.p_map;
    s.tmthing = Some(thing);
    s.tmflags = flags;
    s.tmx = x;
    s.tmy = y;
    s.tmbbox[BOXTOP] = y.wrapping_add(radius);
    s.tmbbox[BOXBOTTOM] = y.wrapping_sub(radius);
    s.tmbbox[BOXRIGHT] = x.wrapping_add(radius);
    s.tmbbox[BOXLEFT] = x.wrapping_sub(radius);
    s.ceilingline = None;
    s.tmfloorz = fh;
    s.tmdropoffz = fh;
    s.tmceilingz = ch;
    d.r_main.validcount += 1;
    d.p_map.numspechit = 0;
}

/// Returns the block range of the current box, extended by `ext`.
fn block_range(d: &Doom, ext: Fixed) -> (i32, i32, i32, i32) {
    let (b, s) = (&d.p_map.tmbbox, &d.p_setup);
    (
        b[BOXLEFT].wrapping_sub(s.bmaporgx).wrapping_sub(ext) >> MAPBLOCKSHIFT,
        b[BOXRIGHT].wrapping_sub(s.bmaporgx).wrapping_add(ext) >> MAPBLOCKSHIFT,
        b[BOXBOTTOM].wrapping_sub(s.bmaporgy).wrapping_sub(ext) >> MAPBLOCKSHIFT,
        b[BOXTOP].wrapping_sub(s.bmaporgy).wrapping_add(ext) >> MAPBLOCKSHIFT,
    )
}

/// P_TeleportMove.
pub fn teleport_move(d: &mut Doom, thing: MobjId, x: Fixed, y: Fixed) -> bool {
    start_check(d, thing, x, y);
    let (xl, xh, yl, yh) = block_range(d, MAXRADIUS);
    for bx in xl..=xh {
        for by in yl..=yh {
            if !block_things_iterator(d, bx, by, pit_stomp_thing) {
                return false;
            }
        }
    }
    unset_thing_position(d, thing);
    let (fz, cz) = (d.p_map.tmfloorz, d.p_map.tmceilingz);
    let m = mo_mut(d, thing);
    m.floorz = fz;
    m.ceilingz = cz;
    m.x = x;
    m.y = y;
    set_thing_position(d, thing);
    true
}

/// PIT_CheckLine.
pub fn pit_check_line(d: &mut Doom, ld: usize) -> bool {
    let l = d.p_setup.lines[ld];
    let b = d.p_map.tmbbox;
    if b[BOXRIGHT] <= l.bbox[BOXLEFT]
        || b[BOXLEFT] >= l.bbox[BOXRIGHT]
        || b[BOXTOP] <= l.bbox[BOXBOTTOM]
        || b[BOXBOTTOM] >= l.bbox[BOXTOP]
    {
        return true;
    }
    if box_on_line_side(d, &b, ld) != -1 {
        return true;
    }
    if l.backsector.is_none() {
        return false;
    }
    let tm = mo(d, tmthing(d));
    if tm.flags & MF_MISSILE == 0 {
        if l.flags & ML_BLOCKING != 0 {
            return false;
        }
        if tm.player.is_none() && l.flags & ML_BLOCKMONSTERS != 0 {
            return false;
        }
    }
    line_opening(d, ld);
    let o = &d.p_maputl;
    let s = &mut d.p_map;
    if o.opentop < s.tmceilingz {
        s.tmceilingz = o.opentop;
        s.ceilingline = Some(ld);
    }
    if o.openbottom > s.tmfloorz {
        s.tmfloorz = o.openbottom;
    }
    if o.lowfloor < s.tmdropoffz {
        s.tmdropoffz = o.lowfloor;
    }
    if l.special != 0 {
        let n = s.numspechit.max(0) as usize;
        if s.spechit.len() <= n {
            s.spechit.resize(n + 1, 0);
        }
        s.spechit[n] = ld;
        s.numspechit += 1;
        if s.numspechit > MAXSPECIALCROSS_ORIGINAL {
            spechit_overrun(d, ld);
        }
    }
    true
}

/// PIT_CheckThing.
pub fn pit_check_thing(d: &mut Doom, thing: MobjId) -> bool {
    let t = mo(d, thing).clone();
    if t.flags & (MF_SOLID | MF_SPECIAL | MF_SHOOTABLE) == 0 {
        return true;
    }
    let tm = tmthing(d);
    let tmo = mo(d, tm);
    let blockdist = t.radius.wrapping_add(tmo.radius);
    if t.x.wrapping_sub(d.p_map.tmx).wrapping_abs() >= blockdist
        || t.y.wrapping_sub(d.p_map.tmy).wrapping_abs() >= blockdist
    {
        return true;
    }
    if thing == tm {
        return true;
    }
    if tmo.flags & MF_SKULLFLY != 0 {
        let damage = (p_random(&mut d.m_random) % 8 + 1) * MOBJINFO[mo(d, tm).type_].damage;
        crate::p_inter::damage_mobj(d, thing, Some(tm), Some(tm), damage);
        let m = mo_mut(d, tm);
        m.flags &= !MF_SKULLFLY;
        m.momx = 0;
        m.momy = 0;
        m.momz = 0;
        let st = MOBJINFO[m.type_].spawnstate as usize;
        set_mobj_state(d, tm, st);
        return false;
    }
    if tmo.flags & MF_MISSILE != 0 {
        if tmo.z > t.z.wrapping_add(t.height) {
            return true;
        }
        if tmo.z.wrapping_add(tmo.height) < t.z {
            return true;
        }
        if let Some(src) = tmo.target {
            let st = mo(d, src).type_;
            use crate::info::{MT_BRUISER, MT_KNIGHT, MT_PLAYER};
            if st == t.type_
                || (st == MT_KNIGHT && t.type_ == MT_BRUISER)
                || (st == MT_BRUISER && t.type_ == MT_KNIGHT)
            {
                if thing == src {
                    return true;
                }
                if t.type_ != MT_PLAYER {
                    return false;
                }
            }
        }
        if t.flags & MF_SHOOTABLE == 0 {
            return t.flags & MF_SOLID == 0;
        }
        let target = tmo.target;
        let damage = (p_random(&mut d.m_random) % 8 + 1) * MOBJINFO[mo(d, tm).type_].damage;
        crate::p_inter::damage_mobj(d, thing, Some(tm), target, damage);
        return false;
    }
    if t.flags & MF_SPECIAL != 0 {
        let solid = t.flags & MF_SOLID != 0;
        if d.p_map.tmflags & MF_PICKUP != 0 {
            crate::p_inter::touch_special_thing(d, thing, tm);
        }
        return !solid;
    }
    t.flags & MF_SOLID == 0
}

/// P_CheckPosition: tests a move to (x, y) and sets tmfloorz, tmceilingz, tmdropoffz, and the special lines.
pub fn check_position(d: &mut Doom, thing: MobjId, x: Fixed, y: Fixed) -> bool {
    start_check(d, thing, x, y);
    if d.p_map.tmflags & MF_NOCLIP != 0 {
        return true;
    }
    let (xl, xh, yl, yh) = block_range(d, MAXRADIUS);
    for bx in xl..=xh {
        for by in yl..=yh {
            if !block_things_iterator(d, bx, by, pit_check_thing) {
                return false;
            }
        }
    }
    let (xl, xh, yl, yh) = block_range(d, 0);
    for bx in xl..=xh {
        for by in yl..=yh {
            if !block_lines_iterator(d, bx, by, pit_check_line) {
                return false;
            }
        }
    }
    true
}

/// P_TryMove.
pub fn try_move(d: &mut Doom, thing: MobjId, x: Fixed, y: Fixed) -> bool {
    d.p_map.floatok = false;
    if !check_position(d, thing, x, y) {
        return false;
    }
    let m = mo(d, thing).clone();
    if m.flags & MF_NOCLIP == 0 {
        if d.p_map.tmceilingz.wrapping_sub(d.p_map.tmfloorz) < m.height {
            return false;
        }
        d.p_map.floatok = true;
        let s = &d.p_map;
        if m.flags & MF_TELEPORT == 0 && s.tmceilingz.wrapping_sub(m.z) < m.height {
            return false;
        }
        if m.flags & MF_TELEPORT == 0 && s.tmfloorz.wrapping_sub(m.z) > 24 * FRACUNIT {
            return false;
        }
        if m.flags & (MF_DROPOFF | MF_FLOAT) == 0
            && s.tmfloorz.wrapping_sub(s.tmdropoffz) > 24 * FRACUNIT
        {
            return false;
        }
    }
    unset_thing_position(d, thing);
    let (fz, cz) = (d.p_map.tmfloorz, d.p_map.tmceilingz);
    let m = mo_mut(d, thing);
    let (oldx, oldy) = (m.x, m.y);
    m.floorz = fz;
    m.ceilingz = cz;
    m.x = x;
    m.y = y;
    set_thing_position(d, thing);
    if mo(d, thing).flags & (MF_TELEPORT | MF_NOCLIP) == 0 {
        loop {
            let n = d.p_map.numspechit;
            d.p_map.numspechit -= 1;
            if n == 0 {
                break;
            }
            let ld = d.p_map.spechit[(n - 1) as usize];
            let (tx, ty) = (mo(d, thing).x, mo(d, thing).y);
            let side = point_on_line_side(d, tx, ty, ld);
            let oldside = point_on_line_side(d, oldx, oldy, ld);
            if side != oldside && d.p_setup.lines[ld].special != 0 {
                crate::p_spec::cross_special_line(d, ld, oldside, thing);
            }
        }
    }
    true
}

/// P_ThingHeightClip.
fn thing_height_clip(d: &mut Doom, thing: MobjId) -> bool {
    let m = mo(d, thing);
    let onfloor = m.z == m.floorz;
    let (x, y) = (m.x, m.y);
    check_position(d, thing, x, y);
    let (fz, cz) = (d.p_map.tmfloorz, d.p_map.tmceilingz);
    let m = mo_mut(d, thing);
    m.floorz = fz;
    m.ceilingz = cz;
    if onfloor {
        m.z = m.floorz;
    } else if m.z.wrapping_add(m.height) > m.ceilingz {
        m.z = m.ceilingz - m.height;
    }
    m.ceilingz.wrapping_sub(m.floorz) >= m.height
}

/// P_HitSlideLine: turns the move to run along the line.
fn hit_slide_line(d: &mut Doom, ld: usize) {
    let l = d.p_setup.lines[ld];
    if l.slopetype == SlopeType::Horizontal {
        d.p_map.tmymove = 0;
        return;
    }
    if l.slopetype == SlopeType::Vertical {
        d.p_map.tmxmove = 0;
        return;
    }
    let sm = mo(d, d.p_map.slidemo.expect("slidemo"));
    let side = point_on_line_side(d, sm.x, sm.y, ld);
    let mut lineangle = crate::r_main::point_to_angle2(0, 0, l.dx, l.dy);
    if side == 1 {
        lineangle = lineangle.wrapping_add(ANG180);
    }
    let (tx, ty) = (d.p_map.tmxmove, d.p_map.tmymove);
    let moveangle = crate::r_main::point_to_angle2(0, 0, tx, ty);
    let mut deltaangle = moveangle.wrapping_sub(lineangle);
    if deltaangle > ANG180 {
        deltaangle = deltaangle.wrapping_add(ANG180);
    }
    let la = (lineangle >> ANGLETOFINESHIFT) as usize;
    let da = (deltaangle >> ANGLETOFINESHIFT) as usize;
    let movelen = approx_distance(tx, ty);
    let newlen = fixed_mul(movelen, finecosine(da));
    d.p_map.tmxmove = fixed_mul(newlen, finecosine(la));
    d.p_map.tmymove = fixed_mul(newlen, finesine(la));
}

/// PTR_SlideTraverse.
pub fn ptr_slide_traverse(d: &mut Doom, ic: &Intercept) -> bool {
    if !ic.isaline {
        crate::i_system::error(d, "PTR_SlideTraverse: not a line?");
    }
    let li = ic.d;
    let sm = mo(d, d.p_map.slidemo.expect("slidemo"));
    let (sx, sy, sz, sh) = (sm.x, sm.y, sm.z, sm.height);
    let blocking = if d.p_setup.lines[li].flags & ML_TWOSIDED == 0 {
        if point_on_line_side(d, sx, sy, li) != 0 {
            return true;
        }
        true
    } else {
        line_opening(d, li);
        let o = &d.p_maputl;
        o.openrange < sh
            || o.opentop.wrapping_sub(sz) < sh
            || o.openbottom.wrapping_sub(sz) > 24 * FRACUNIT
    };
    if !blocking {
        return true;
    }
    let s = &mut d.p_map;
    if ic.frac < s.bestslidefrac {
        s.secondslidefrac = s.bestslidefrac;
        s.secondslideline = s.bestslideline;
        s.bestslidefrac = ic.frac;
        s.bestslideline = Some(li);
    }
    false
}

/// The stair step at the end of P_SlideMove.
fn stairstep(d: &mut Doom, m: MobjId) {
    let o = mo(d, m);
    let (x, y, mx, my) = (o.x, o.y, o.momx, o.momy);
    if !try_move(d, m, x, y.wrapping_add(my)) {
        try_move(d, m, x.wrapping_add(mx), y);
    }
}

/// P_SlideMove: moves a blocked player along the nearest blocking line.
pub fn slide_move(d: &mut Doom, mobj: MobjId) {
    d.p_map.slidemo = Some(mobj);
    let mut hitcount = 0;
    loop {
        hitcount += 1;
        if hitcount == 3 {
            stairstep(d, mobj);
            return;
        }
        let m = mo(d, mobj);
        let (leadx, trailx) = if m.momx > 0 {
            (m.x.wrapping_add(m.radius), m.x.wrapping_sub(m.radius))
        } else {
            (m.x.wrapping_sub(m.radius), m.x.wrapping_add(m.radius))
        };
        let (leady, traily) = if m.momy > 0 {
            (m.y.wrapping_add(m.radius), m.y.wrapping_sub(m.radius))
        } else {
            (m.y.wrapping_sub(m.radius), m.y.wrapping_add(m.radius))
        };
        let (mx, my) = (m.momx, m.momy);
        d.p_map.bestslidefrac = FRACUNIT + 1;
        path_traverse(
            d,
            leadx,
            leady,
            leadx.wrapping_add(mx),
            leady.wrapping_add(my),
            PT_ADDLINES,
            ptr_slide_traverse,
        );
        path_traverse(
            d,
            trailx,
            leady,
            trailx.wrapping_add(mx),
            leady.wrapping_add(my),
            PT_ADDLINES,
            ptr_slide_traverse,
        );
        path_traverse(
            d,
            leadx,
            traily,
            leadx.wrapping_add(mx),
            traily.wrapping_add(my),
            PT_ADDLINES,
            ptr_slide_traverse,
        );
        if d.p_map.bestslidefrac == FRACUNIT + 1 {
            stairstep(d, mobj);
            return;
        }
        d.p_map.bestslidefrac -= 0x800;
        let bsf = d.p_map.bestslidefrac;
        if bsf > 0 {
            let m = mo(d, mobj);
            let newx = fixed_mul(m.momx, bsf);
            let newy = fixed_mul(m.momy, bsf);
            let (x, y) = (m.x, m.y);
            if !try_move(d, mobj, x.wrapping_add(newx), y.wrapping_add(newy)) {
                stairstep(d, mobj);
                return;
            }
        }
        let mut bsf = FRACUNIT - (d.p_map.bestslidefrac + 0x800);
        if bsf > FRACUNIT {
            bsf = FRACUNIT;
        }
        d.p_map.bestslidefrac = bsf;
        if bsf <= 0 {
            return;
        }
        let (mx, my) = (mo(d, mobj).momx, mo(d, mobj).momy);
        d.p_map.tmxmove = fixed_mul(mx, bsf);
        d.p_map.tmymove = fixed_mul(my, bsf);
        let line = d.p_map.bestslideline.expect("bestslideline");
        hit_slide_line(d, line);
        let (tx, ty) = (d.p_map.tmxmove, d.p_map.tmymove);
        let m = mo_mut(d, mobj);
        m.momx = tx;
        m.momy = ty;
        let (x, y) = (m.x, m.y);
        if try_move(d, mobj, x.wrapping_add(tx), y.wrapping_add(ty)) {
            return;
        }
    }
}

/// PTR_AimTraverse.
pub fn ptr_aim_traverse(d: &mut Doom, ic: &Intercept) -> bool {
    if ic.isaline {
        let li = d.p_setup.lines[ic.d];
        if li.flags & ML_TWOSIDED == 0 {
            return false;
        }
        line_opening(d, ic.d);
        let (ob, ot) = (d.p_maputl.openbottom, d.p_maputl.opentop);
        if ob >= ot {
            return false;
        }
        let dist = fixed_mul(d.p_map.attackrange, ic.frac);
        let shootz = d.p_map.shootz;
        let front = &d.p_setup.sectors[li.frontsector.unwrap_or(0)];
        let back = li.backsector.map(|b| &d.p_setup.sectors[b]);
        if back.is_none_or(|b| front.floorheight != b.floorheight) {
            let slope = fixed_div(ob.wrapping_sub(shootz), dist);
            if slope > d.p_sight.bottomslope {
                d.p_sight.bottomslope = slope;
            }
        }
        if back.is_none_or(|b| front.ceilingheight != b.ceilingheight) {
            let slope = fixed_div(ot.wrapping_sub(shootz), dist);
            if slope < d.p_sight.topslope {
                d.p_sight.topslope = slope;
            }
        }
        return d.p_sight.topslope > d.p_sight.bottomslope;
    }
    let th = ic.d;
    if Some(th) == d.p_map.shootthing {
        return true;
    }
    let t = mo(d, th);
    if t.flags & MF_SHOOTABLE == 0 {
        return true;
    }
    let shootz = d.p_map.shootz;
    let dist = fixed_mul(d.p_map.attackrange, ic.frac);
    let mut thingtopslope = fixed_div(t.z.wrapping_add(t.height).wrapping_sub(shootz), dist);
    if thingtopslope < d.p_sight.bottomslope {
        return true;
    }
    let mut thingbottomslope = fixed_div(t.z.wrapping_sub(shootz), dist);
    if thingbottomslope > d.p_sight.topslope {
        return true;
    }
    if thingtopslope > d.p_sight.topslope {
        thingtopslope = d.p_sight.topslope;
    }
    if thingbottomslope < d.p_sight.bottomslope {
        thingbottomslope = d.p_sight.bottomslope;
    }
    d.p_map.aimslope = thingtopslope.wrapping_add(thingbottomslope) / 2;
    d.p_map.linetarget = Some(th);
    false
}

/// PTR_ShootTraverse.
pub fn ptr_shoot_traverse(d: &mut Doom, ic: &Intercept) -> bool {
    let shootz = d.p_map.shootz;
    let aimslope = d.p_map.aimslope;
    let attackrange = d.p_map.attackrange;
    let trace = d.p_maputl.trace;
    if ic.isaline {
        let li = d.p_setup.lines[ic.d];
        if li.special != 0 {
            let st = d.p_map.shootthing.expect("shootthing");
            crate::p_spec::shoot_special_line(d, st, ic.d);
        }
        if li.flags & ML_TWOSIDED != 0 {
            line_opening(d, ic.d);
            let (ob, ot) = (d.p_maputl.openbottom, d.p_maputl.opentop);
            let dist = fixed_mul(attackrange, ic.frac);
            let front = &d.p_setup.sectors[li.frontsector.unwrap_or(0)];
            let back = li.backsector.map(|b| &d.p_setup.sectors[b]);
            let hit = (back.is_none_or(|b| front.floorheight != b.floorheight)
                && fixed_div(ob.wrapping_sub(shootz), dist) > aimslope)
                || (back.is_none_or(|b| front.ceilingheight != b.ceilingheight)
                    && fixed_div(ot.wrapping_sub(shootz), dist) < aimslope);
            if !hit {
                return true;
            }
        }
        let frac = ic.frac.wrapping_sub(fixed_div(4 * FRACUNIT, attackrange));
        let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
        let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
        let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
        let front = &d.p_setup.sectors[li.frontsector.unwrap_or(0)];
        if front.ceilingpic == d.r_sky.skyflatnum {
            if z > front.ceilingheight {
                return false;
            }
            if let Some(b) = li.backsector {
                if d.p_setup.sectors[b].ceilingpic == d.r_sky.skyflatnum {
                    return false;
                }
            }
        }
        spawn_puff(d, x, y, z);
        return false;
    }
    let th = ic.d;
    if Some(th) == d.p_map.shootthing {
        return true;
    }
    let t = mo(d, th);
    if t.flags & MF_SHOOTABLE == 0 {
        return true;
    }
    let dist = fixed_mul(attackrange, ic.frac);
    let thingtopslope = fixed_div(t.z.wrapping_add(t.height).wrapping_sub(shootz), dist);
    if thingtopslope < aimslope {
        return true;
    }
    let thingbottomslope = fixed_div(t.z.wrapping_sub(shootz), dist);
    if thingbottomslope > aimslope {
        return true;
    }
    let noblood = t.flags & MF_NOBLOOD != 0;
    let frac = ic.frac.wrapping_sub(fixed_div(10 * FRACUNIT, attackrange));
    let x = trace.x.wrapping_add(fixed_mul(trace.dx, frac));
    let y = trace.y.wrapping_add(fixed_mul(trace.dy, frac));
    let z = shootz.wrapping_add(fixed_mul(aimslope, fixed_mul(frac, attackrange)));
    let damage = d.p_map.la_damage;
    if noblood {
        spawn_puff(d, x, y, z);
    } else {
        spawn_blood(d, x, y, z, damage);
    }
    if damage != 0 {
        let st = d.p_map.shootthing;
        crate::p_inter::damage_mobj(d, th, st, st, damage);
    }
    false
}

fn attack_setup(
    d: &mut Doom,
    t1: MobjId,
    angle: Angle,
    distance: Fixed,
) -> (Fixed, Fixed, Fixed, Fixed) {
    let fa = (angle >> ANGLETOFINESHIFT) as usize;
    d.p_map.shootthing = Some(t1);
    let m = mo(d, t1);
    let x2 =
        m.x.wrapping_add((distance >> FRACBITS).wrapping_mul(finecosine(fa)));
    let y2 =
        m.y.wrapping_add((distance >> FRACBITS).wrapping_mul(finesine(fa)));
    let (x1, y1) = (m.x, m.y);
    d.p_map.shootz = m.z.wrapping_add(m.height >> 1).wrapping_add(8 * FRACUNIT);
    d.p_map.attackrange = distance;
    (x1, y1, x2, y2)
}

/// P_AimLineAttack: returns the slope to the first shootable thing, or 0.
pub fn aim_line_attack(d: &mut Doom, t1: MobjId, angle: Angle, distance: Fixed) -> Fixed {
    let (x1, y1, x2, y2) = attack_setup(d, t1, angle, distance);
    d.p_sight.topslope = 100 * FRACUNIT / 160;
    d.p_sight.bottomslope = -100 * FRACUNIT / 160;
    d.p_map.linetarget = None;
    path_traverse(
        d,
        x1,
        y1,
        x2,
        y2,
        PT_ADDLINES | PT_ADDTHINGS,
        ptr_aim_traverse,
    );
    if d.p_map.linetarget.is_some() {
        d.p_map.aimslope
    } else {
        0
    }
}

/// P_LineAttack.
pub fn line_attack(
    d: &mut Doom,
    t1: MobjId,
    angle: Angle,
    distance: Fixed,
    slope: Fixed,
    damage: i32,
) {
    d.p_map.la_damage = damage;
    let (x1, y1, x2, y2) = attack_setup(d, t1, angle, distance);
    d.p_map.aimslope = slope;
    path_traverse(
        d,
        x1,
        y1,
        x2,
        y2,
        PT_ADDLINES | PT_ADDTHINGS,
        ptr_shoot_traverse,
    );
}

/// PTR_UseTraverse.
pub fn ptr_use_traverse(d: &mut Doom, ic: &Intercept) -> bool {
    let us = d.p_map.usething.expect("usething");
    if d.p_setup.lines[ic.d].special == 0 {
        line_opening(d, ic.d);
        if d.p_maputl.openrange <= 0 {
            crate::s_sound::start_sound(d, Some(us), crate::s_sound::SFX_NOWAY);
            return false;
        }
        return true;
    }
    let (x, y) = (mo(d, us).x, mo(d, us).y);
    let side = if point_on_line_side(d, x, y, ic.d) == 1 {
        1
    } else {
        0
    };
    crate::p_switch::use_special_line(d, us, ic.d, side);
    false
}

/// P_UseLines.
pub fn use_lines(d: &mut Doom, player: usize) {
    let pmo = d.g_game.players[player].mo.expect("player mobj");
    d.p_map.usething = Some(pmo);
    let m = mo(d, pmo);
    let fa = (m.angle >> ANGLETOFINESHIFT) as usize;
    let (x1, y1) = (m.x, m.y);
    let x2 = x1.wrapping_add((USERANGE >> FRACBITS).wrapping_mul(finecosine(fa)));
    let y2 = y1.wrapping_add((USERANGE >> FRACBITS).wrapping_mul(finesine(fa)));
    path_traverse(d, x1, y1, x2, y2, PT_ADDLINES, ptr_use_traverse);
}

/// PIT_RadiusAttack.
pub fn pit_radius_attack(d: &mut Doom, thing: MobjId) -> bool {
    let t = mo(d, thing);
    if t.flags & MF_SHOOTABLE == 0 {
        return true;
    }
    if t.type_ == crate::info::MT_CYBORG || t.type_ == crate::info::MT_SPIDER {
        return true;
    }
    let spot = d.p_map.bombspot.expect("bombspot");
    let s = mo(d, spot);
    let dx = t.x.wrapping_sub(s.x).wrapping_abs();
    let dy = t.y.wrapping_sub(s.y).wrapping_abs();
    let mut dist = if dx > dy { dx } else { dy };
    dist = dist.wrapping_sub(t.radius) >> FRACBITS;
    if dist < 0 {
        dist = 0;
    }
    let bombdamage = d.p_map.bombdamage;
    if dist >= bombdamage {
        return true;
    }
    if crate::p_sight::check_sight(d, thing, spot) {
        let src = d.p_map.bombsource;
        crate::p_inter::damage_mobj(d, thing, Some(spot), src, bombdamage - dist);
    }
    true
}

/// P_RadiusAttack. The source can be absent.
pub fn radius_attack(d: &mut Doom, spot: MobjId, source: Option<MobjId>, damage: i32) {
    let dist = damage.wrapping_add(MAXRADIUS) << FRACBITS;
    let s = mo(d, spot);
    let (orgx, orgy) = (d.p_setup.bmaporgx, d.p_setup.bmaporgy);
    let yh = s.y.wrapping_add(dist).wrapping_sub(orgy) >> MAPBLOCKSHIFT;
    let yl = s.y.wrapping_sub(dist).wrapping_sub(orgy) >> MAPBLOCKSHIFT;
    let xh = s.x.wrapping_add(dist).wrapping_sub(orgx) >> MAPBLOCKSHIFT;
    let xl = s.x.wrapping_sub(dist).wrapping_sub(orgx) >> MAPBLOCKSHIFT;
    d.p_map.bombspot = Some(spot);
    d.p_map.bombsource = source;
    d.p_map.bombdamage = damage;
    for y in yl..=yh {
        for x in xl..=xh {
            block_things_iterator(d, x, y, pit_radius_attack);
        }
    }
}

/// PIT_ChangeSector.
pub fn pit_change_sector(d: &mut Doom, thing: MobjId) -> bool {
    if thing_height_clip(d, thing) {
        return true;
    }
    let t = mo(d, thing);
    if t.health <= 0 {
        set_mobj_state(d, thing, crate::info::S_GIBS);
        let m = mo_mut(d, thing);
        m.flags &= !MF_SOLID;
        m.height = 0;
        m.radius = 0;
        return true;
    }
    if t.flags & MF_DROPPED != 0 {
        remove_mobj(d, thing);
        return true;
    }
    if t.flags & MF_SHOOTABLE == 0 {
        return true;
    }
    d.p_map.nofit = 1;
    if d.p_map.crushchange != 0 && d.p_tick.leveltime & 3 == 0 {
        crate::p_inter::damage_mobj(d, thing, None, None, 10);
        let t = mo(d, thing);
        let (x, y, z) = (t.x, t.y, t.z.wrapping_add(t.height / 2));
        let b = spawn_mobj(d, x, y, z, crate::info::MT_BLOOD);
        let mx = sub_random(d) << 12;
        let my = sub_random(d) << 12;
        let m = mo_mut(d, b);
        m.momx = mx;
        m.momy = my;
    }
    true
}

/// P_ChangeSector: rechecks the things near a moving sector. Returns true when something does not fit.
pub fn change_sector(d: &mut Doom, sector: usize, crunch: bool) -> bool {
    d.p_map.nofit = 0;
    d.p_map.crushchange = crunch as i32;
    let bb = d.p_setup.sectors[sector].blockbox;
    for x in bb[BOXLEFT]..=bb[BOXRIGHT] {
        for y in bb[BOXBOTTOM]..=bb[BOXTOP] {
            block_things_iterator(d, x, y, pit_change_sector);
        }
    }
    d.p_map.nofit != 0
}

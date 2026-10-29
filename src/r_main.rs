//! Refresh control: view setup, view size, light tables, angle helpers, and the player view.

use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::p_setup::{Node, NF_SUBSECTOR};
use crate::r_draw::ColFunc;
use crate::tables::{
    finecosine, finesine, slope_div, Angle, ANG180, ANG270, ANG90, ANGLETOFINESHIFT, DBITS,
    FINEANGLES, FINETANGENT, TANTOANGLE,
};
use crate::Doom;

pub const FIELDOFVIEW: usize = 2048;
pub const NUMCOLORMAPS: i32 = 32;
pub const LIGHTLEVELS: usize = 16;
pub const LIGHTSEGSHIFT: i32 = 4;
pub const MAXLIGHTSCALE: usize = 48;
pub const LIGHTSCALESHIFT: i32 = 12;
pub const MAXLIGHTZ: usize = 128;
pub const LIGHTZSHIFT: i32 = 20;
const DISTMAP: i32 = 2;

pub struct State {
    pub viewangleoffset: i32,
    /// The shared visit counter of the original. The play code also uses it.
    pub validcount: i32,
    /// The colormap offset for invulnerability or light amplification, or None.
    pub fixedcolormap: Option<usize>,
    pub centerx: i32,
    pub centery: i32,
    pub centerxfrac: Fixed,
    pub centeryfrac: Fixed,
    pub projection: Fixed,
    pub framecount: i32,
    pub sscount: i32,
    pub linecount: i32,
    pub loopcount: i32,
    pub viewx: Fixed,
    pub viewy: Fixed,
    pub viewz: Fixed,
    pub viewangle: Angle,
    pub viewcos: Fixed,
    pub viewsin: Fixed,
    pub viewplayer: usize,
    pub detailshift: i32,
    pub clipangle: Angle,
    pub viewangletox: Vec<i32>,
    pub xtoviewangle: Vec<Angle>,
    /// Colormap offsets for each light level and wall scale.
    pub scalelight: [[usize; MAXLIGHTSCALE]; LIGHTLEVELS],
    pub scalelightfixed: [usize; MAXLIGHTSCALE],
    /// Colormap offsets for each light level and plane distance.
    pub zlight: [[usize; MAXLIGHTZ]; LIGHTLEVELS],
    pub extralight: i32,
    pub setsizeneeded: bool,
    pub setblocks: i32,
    pub setdetail: i32,
}

impl Default for State {
    fn default() -> Self {
        State {
            viewangleoffset: 0,
            validcount: 1,
            fixedcolormap: None,
            centerx: 0,
            centery: 0,
            centerxfrac: 0,
            centeryfrac: 0,
            projection: 0,
            framecount: 0,
            sscount: 0,
            linecount: 0,
            loopcount: 0,
            viewx: 0,
            viewy: 0,
            viewz: 0,
            viewangle: 0,
            viewcos: 0,
            viewsin: 0,
            viewplayer: 0,
            detailshift: 0,
            clipangle: 0,
            viewangletox: vec![0; FINEANGLES / 2],
            xtoviewangle: vec![0; SCREENWIDTH + 1],
            scalelight: [[0; MAXLIGHTSCALE]; LIGHTLEVELS],
            scalelightfixed: [0; MAXLIGHTSCALE],
            zlight: [[0; MAXLIGHTZ]; LIGHTLEVELS],
            extralight: 0,
            setsizeneeded: false,
            setblocks: 0,
            setdetail: 0,
        }
    }
}

/// R_PointOnSide: returns 0 for the front side of a node line and 1 for the back side.
pub fn point_on_side(x: Fixed, y: Fixed, node: &Node) -> i32 {
    if node.dx == 0 {
        if x <= node.x {
            return (node.dy > 0) as i32;
        }
        return (node.dy < 0) as i32;
    }
    if node.dy == 0 {
        if y <= node.y {
            return (node.dx < 0) as i32;
        }
        return (node.dx > 0) as i32;
    }
    let dx = x.wrapping_sub(node.x);
    let dy = y.wrapping_sub(node.y);
    if (node.dy ^ node.dx ^ dx ^ dy) < 0 {
        return ((node.dy ^ dx) < 0) as i32;
    }
    let left = fixed_mul(node.dy >> FRACBITS, dx);
    let right = fixed_mul(dy, node.dx >> FRACBITS);
    if right < left {
        0
    } else {
        1
    }
}

/// R_PointOnSegSide: the same test against a seg.
pub fn point_on_seg_side(d: &Doom, x: Fixed, y: Fixed, seg: usize) -> i32 {
    let s = &d.p_setup.segs[seg];
    let v1 = d.p_setup.vertexes[s.v1];
    let v2 = d.p_setup.vertexes[s.v2];
    let node = Node {
        x: v1.x,
        y: v1.y,
        dx: v2.x.wrapping_sub(v1.x),
        dy: v2.y.wrapping_sub(v1.y),
        ..Default::default()
    };
    point_on_side(x, y, &node)
}

/// The octant arithmetic of R_PointToAngle for a vector from the view point.
fn vector_to_angle(x: Fixed, y: Fixed) -> Angle {
    if x == 0 && y == 0 {
        return 0;
    }
    let t = |n: i32, d: i32| TANTOANGLE[slope_div(n as u32, d as u32) as usize];
    if x >= 0 {
        if y >= 0 {
            if x > y {
                t(y, x)
            } else {
                (ANG90 - 1).wrapping_sub(t(x, y))
            }
        } else {
            let y = y.wrapping_neg();
            if x > y {
                t(y, x).wrapping_neg()
            } else {
                ANG270.wrapping_add(t(x, y))
            }
        }
    } else {
        let x = x.wrapping_neg();
        if y >= 0 {
            if x > y {
                (ANG180 - 1).wrapping_sub(t(y, x))
            } else {
                ANG90.wrapping_add(t(x, y))
            }
        } else {
            let y = y.wrapping_neg();
            if x > y {
                ANG180.wrapping_add(t(y, x))
            } else {
                (ANG270 - 1).wrapping_sub(t(x, y))
            }
        }
    }
}

/// R_PointToAngle: the angle from the view point to a point.
pub fn point_to_angle(d: &Doom, x: Fixed, y: Fixed) -> Angle {
    vector_to_angle(
        x.wrapping_sub(d.r_main.viewx),
        y.wrapping_sub(d.r_main.viewy),
    )
}

/// R_PointToAngle2: the angle from the first point to the second point.
/// The original also stores the first point as the view point; the next frame setup overwrites it.
pub fn point_to_angle2(x1: Fixed, y1: Fixed, x2: Fixed, y2: Fixed) -> Angle {
    vector_to_angle(x2.wrapping_sub(x1), y2.wrapping_sub(y1))
}

/// R_PointToDist: the distance from the view point to a point.
pub fn point_to_dist(d: &Doom, x: Fixed, y: Fixed) -> Fixed {
    let mut dx = x.wrapping_sub(d.r_main.viewx).wrapping_abs();
    let mut dy = y.wrapping_sub(d.r_main.viewy).wrapping_abs();
    if dy > dx {
        std::mem::swap(&mut dx, &mut dy);
    }
    let angle = if dx != 0 {
        (TANTOANGLE[(fixed_div(dy, dx) >> DBITS) as usize].wrapping_add(ANG90) >> ANGLETOFINESHIFT)
            as usize
    } else {
        0
    };
    fixed_div(dx, finesine(angle))
}

/// R_ScaleFromGlobalAngle: the wall scale at a view angle for the current seg.
pub fn scale_from_global_angle(d: &Doom, visangle: Angle) -> Fixed {
    let m = &d.r_main;
    let anglea = ANG90.wrapping_add(visangle.wrapping_sub(m.viewangle));
    let angleb = ANG90.wrapping_add(visangle.wrapping_sub(d.r_segs.rw_normalangle));
    let sinea = finesine((anglea >> ANGLETOFINESHIFT) as usize);
    let sineb = finesine((angleb >> ANGLETOFINESHIFT) as usize);
    let num = fixed_mul(m.projection, sineb) << m.detailshift;
    let den = fixed_mul(d.r_segs.rw_distance, sinea);
    if den > num >> 16 {
        let scale = fixed_div(num, den);
        if scale > 64 * FRACUNIT {
            64 * FRACUNIT
        } else if scale < 256 {
            256
        } else {
            scale
        }
    } else {
        64 * FRACUNIT
    }
}

/// R_PointInSubsector: walks the node tree to the subsector that holds a point.
pub fn point_in_subsector(d: &Doom, x: Fixed, y: Fixed) -> usize {
    let nodes = &d.p_setup.nodes;
    if nodes.is_empty() {
        return 0;
    }
    let mut nodenum = (nodes.len() - 1) as u16;
    while nodenum & NF_SUBSECTOR == 0 {
        let node = &nodes[nodenum as usize];
        let side = point_on_side(x, y, node);
        nodenum = node.children[side as usize];
    }
    (nodenum & !NF_SUBSECTOR) as usize
}

/// R_InitTextureMapping: builds the angle-to-column and column-to-angle tables.
fn init_texture_mapping(d: &mut Doom) {
    let viewwidth = d.r_draw.viewwidth;
    let m = &mut d.r_main;
    let focallength = fixed_div(m.centerxfrac, FINETANGENT[FINEANGLES / 4 + FIELDOFVIEW / 2]);
    for i in 0..FINEANGLES / 2 {
        let tan = FINETANGENT[i];
        let t = if tan > FRACUNIT * 2 {
            -1
        } else if tan < -FRACUNIT * 2 {
            viewwidth + 1
        } else {
            let t = fixed_mul(tan, focallength);
            let t = (m.centerxfrac.wrapping_sub(t).wrapping_add(FRACUNIT - 1)) >> FRACBITS;
            t.clamp(-1, viewwidth + 1)
        };
        m.viewangletox[i] = t;
    }
    for x in 0..=viewwidth {
        let mut i = 0;
        while m.viewangletox[i] > x {
            i += 1;
        }
        m.xtoviewangle[x as usize] = ((i as u32) << ANGLETOFINESHIFT).wrapping_sub(ANG90);
    }
    for i in 0..FINEANGLES / 2 {
        if m.viewangletox[i] == -1 {
            m.viewangletox[i] = 0;
        } else if m.viewangletox[i] == viewwidth + 1 {
            m.viewangletox[i] = viewwidth;
        }
    }
    m.clipangle = m.xtoviewangle[0];
}

/// R_InitLightTables: builds the plane light tables.
fn init_light_tables(d: &mut Doom) {
    for i in 0..LIGHTLEVELS {
        let startmap =
            ((LIGHTLEVELS as i32 - 1 - i as i32) * 2) * NUMCOLORMAPS / LIGHTLEVELS as i32;
        for j in 0..MAXLIGHTZ {
            let mut scale = fixed_div(
                (SCREENWIDTH as i32 / 2) * FRACUNIT,
                ((j as i32) + 1) << LIGHTZSHIFT,
            );
            scale >>= LIGHTSCALESHIFT;
            let level = (startmap - scale / DISTMAP).clamp(0, NUMCOLORMAPS - 1);
            d.r_main.zlight[i][j] = (level * 256) as usize;
        }
    }
}

/// R_SetViewSize: requests a new view size for the next frame.
pub fn set_view_size(d: &mut Doom, blocks: i32, detail: i32) {
    d.r_main.setsizeneeded = true;
    d.r_main.setblocks = blocks;
    d.r_main.setdetail = detail;
}

/// R_ExecuteSetViewSize: applies the requested view size and detail.
pub fn execute_set_view_size(d: &mut Doom) {
    d.r_main.setsizeneeded = false;
    let setblocks = d.r_main.setblocks;
    let (scaledviewwidth, viewheight) = if setblocks == 11 {
        (SCREENWIDTH as i32, SCREENHEIGHT as i32)
    } else {
        (setblocks * 32, (setblocks * 168 / 10) & !7)
    };
    let detailshift = d.r_main.setdetail;
    let viewwidth = scaledviewwidth >> detailshift;
    d.r_draw.scaledviewwidth = scaledviewwidth;
    d.r_draw.viewheight = viewheight;
    d.r_draw.viewwidth = viewwidth;
    let m = &mut d.r_main;
    m.detailshift = detailshift;
    m.centery = viewheight / 2;
    m.centerx = viewwidth / 2;
    m.centerxfrac = m.centerx << FRACBITS;
    m.centeryfrac = m.centery << FRACBITS;
    m.projection = m.centerxfrac;
    let s = &mut d.r_draw;
    if detailshift == 0 {
        s.colfunc = ColFunc::Column;
        s.basecolfunc = ColFunc::Column;
        s.fuzzcolfunc = ColFunc::Fuzz;
        s.transcolfunc = ColFunc::Translated;
        s.spanlow = false;
    } else {
        s.colfunc = ColFunc::ColumnLow;
        s.basecolfunc = ColFunc::ColumnLow;
        s.fuzzcolfunc = ColFunc::FuzzLow;
        s.transcolfunc = ColFunc::TranslatedLow;
        s.spanlow = true;
    }
    crate::r_draw::init_buffer(d, scaledviewwidth, viewheight);
    init_texture_mapping(d);

    let t = &mut d.r_things;
    t.pspritescale = FRACUNIT * viewwidth / SCREENWIDTH as i32;
    t.pspriteiscale = FRACUNIT * SCREENWIDTH as i32 / viewwidth;
    for i in 0..viewwidth as usize {
        t.screenheightarray[i] = viewheight as i16;
    }
    let p = &mut d.r_plane;
    for i in 0..viewheight {
        let dy = (((i - viewheight / 2) << FRACBITS) + FRACUNIT / 2).abs();
        p.yslope[i as usize] = fixed_div((viewwidth << detailshift) / 2 * FRACUNIT, dy);
    }
    for i in 0..viewwidth as usize {
        let cosadj = finecosine((d.r_main.xtoviewangle[i] >> ANGLETOFINESHIFT) as usize).abs();
        p.distscale[i] = fixed_div(FRACUNIT, cosadj);
    }
    for i in 0..LIGHTLEVELS {
        let startmap =
            ((LIGHTLEVELS as i32 - 1 - i as i32) * 2) * NUMCOLORMAPS / LIGHTLEVELS as i32;
        for j in 0..MAXLIGHTSCALE {
            let level =
                startmap - (j as i32) * SCREENWIDTH as i32 / (viewwidth << detailshift) / DISTMAP;
            d.r_main.scalelight[i][j] = (level.clamp(0, NUMCOLORMAPS - 1) * 256) as usize;
        }
    }
}

/// R_Init: loads the refresh data and builds the tables. It prints progress dots.
pub fn init(d: &mut Doom) {
    crate::r_data::init_data(d);
    print!(".");
    // R_InitPointToAngle: the tables are static.
    print!(".");
    // R_InitTables: the tables are static.
    print!(".");
    let (blocks, detail) = crate::m_menu::view_settings(d);
    set_view_size(d, blocks, detail);
    crate::r_plane::init_planes(d);
    print!(".");
    init_light_tables(d);
    print!(".");
    crate::r_sky::init_sky_map(d);
    crate::r_draw::init_translation_tables(d);
    print!(".");
    d.r_main.framecount = 0;
}

/// Returns viewwindowx, viewwindowy, viewwidth, and viewheight for the heads-up erase.
pub fn view_window(d: &Doom) -> (i32, i32, i32, i32) {
    let s = &d.r_draw;
    (s.viewwindowx, s.viewwindowy, s.viewwidth, s.viewheight)
}

/// R_SetupFrame: copies the player view into the view globals.
fn setup_frame(d: &mut Doom, player: usize) {
    let p = &d.g_game.players[player];
    let mo = d.p_tick.mobj(p.mo.expect("the view player has no body"));
    let m = &mut d.r_main;
    m.viewplayer = player;
    m.viewx = mo.x;
    m.viewy = mo.y;
    m.viewangle = mo.angle.wrapping_add(m.viewangleoffset as u32);
    m.extralight = p.extralight;
    m.viewz = p.viewz;
    m.viewsin = finesine((m.viewangle >> ANGLETOFINESHIFT) as usize);
    m.viewcos = finecosine((m.viewangle >> ANGLETOFINESHIFT) as usize);
    m.sscount = 0;
    if p.fixedcolormap != 0 {
        let cm = (p.fixedcolormap * 256) as usize;
        m.fixedcolormap = Some(cm);
        m.scalelightfixed = [cm; MAXLIGHTSCALE];
        d.r_segs.walllights = m.scalelightfixed;
    } else {
        m.fixedcolormap = None;
    }
    m.framecount += 1;
    m.validcount += 1;
}

/// R_RenderPlayerView: draws the 3D view of a player.
pub fn render_player_view(d: &mut Doom, player: usize) {
    setup_frame(d, player);
    crate::r_bsp::clear_clip_segs(d);
    crate::r_bsp::clear_draw_segs(d);
    crate::r_plane::clear_planes(d);
    crate::r_things::clear_sprites(d);
    crate::d_loop::net_update(d);
    let numnodes = d.p_setup.nodes.len() as i32;
    crate::r_bsp::render_bsp_node(d, numnodes - 1);
    crate::d_loop::net_update(d);
    crate::r_plane::draw_planes(d);
    crate::d_loop::net_update(d);
    crate::r_things::draw_masked(d);
    crate::d_loop::net_update(d);
}

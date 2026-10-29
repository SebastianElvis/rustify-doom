//! Level loading: map lumps, the blockmap, the reject matrix, and the sector line lists.

use crate::m_fixed::Fixed;
use crate::m_fixed::{fixed_div, FRACBITS};
use crate::p_mobj::MobjId;
use crate::p_tick::ThinkerId;
use crate::tables::Angle;
use crate::Doom;

pub const BOXTOP: usize = 0;
pub const BOXBOTTOM: usize = 1;
pub const BOXLEFT: usize = 2;
pub const BOXRIGHT: usize = 3;

pub const ML_BLOCKING: i32 = 1;
pub const ML_BLOCKMONSTERS: i32 = 2;
pub const ML_TWOSIDED: i32 = 4;
pub const ML_DONTPEGTOP: i32 = 8;
pub const ML_DONTPEGBOTTOM: i32 = 16;
pub const ML_SECRET: i32 = 32;
pub const ML_SOUNDBLOCK: i32 = 64;
pub const ML_DONTDRAW: i32 = 128;
pub const ML_MAPPED: i32 = 256;

pub const NF_SUBSECTOR: u16 = 0x8000;

#[derive(Clone, Copy, Default, Debug)]
pub struct Vertex {
    pub x: Fixed,
    pub y: Fixed,
}

#[derive(Clone, Default, Debug)]
pub struct Sector {
    pub floorheight: Fixed,
    pub ceilingheight: Fixed,
    pub floorpic: i32,
    pub ceilingpic: i32,
    pub lightlevel: i32,
    pub special: i32,
    pub tag: i32,
    pub soundtraversed: i32,
    pub soundtarget: Option<MobjId>,
    pub blockbox: [i32; 4],
    /// The sound origin (center of the bounding box).
    pub soundorg_x: Fixed,
    pub soundorg_y: Fixed,
    pub validcount: i32,
    pub thinglist: Option<MobjId>,
    pub specialdata: Option<ThinkerId>,
    /// Indexes into `Level::lines`.
    pub lines: Vec<usize>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Side {
    pub textureoffset: Fixed,
    pub rowoffset: Fixed,
    pub toptexture: i32,
    pub bottomtexture: i32,
    pub midtexture: i32,
    pub sector: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SlopeType {
    #[default]
    Horizontal,
    Vertical,
    Positive,
    Negative,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Line {
    pub v1: usize,
    pub v2: usize,
    pub dx: Fixed,
    pub dy: Fixed,
    pub flags: i32,
    pub special: i32,
    pub tag: i32,
    /// Side numbers; -1 marks a missing side.
    pub sidenum: [i32; 2],
    pub bbox: [Fixed; 4],
    pub slopetype: SlopeType,
    pub frontsector: Option<usize>,
    pub backsector: Option<usize>,
    pub validcount: i32,
    pub specialdata: Option<ThinkerId>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Seg {
    pub v1: usize,
    pub v2: usize,
    pub offset: Fixed,
    pub angle: Angle,
    pub sidedef: usize,
    pub linedef: usize,
    pub frontsector: usize,
    pub backsector: Option<usize>,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Subsector {
    pub sector: usize,
    pub numlines: usize,
    pub firstline: usize,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Node {
    pub x: Fixed,
    pub y: Fixed,
    pub dx: Fixed,
    pub dy: Fixed,
    pub bbox: [[Fixed; 4]; 2],
    pub children: [u16; 2],
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct MapThing {
    pub x: i16,
    pub y: i16,
    pub angle: i16,
    pub type_: i16,
    pub options: i16,
}

#[derive(Default)]
pub struct State {
    pub vertexes: Vec<Vertex>,
    pub segs: Vec<Seg>,
    /// The map sectors, then the sector at the null address when a seg needs it.
    pub sectors: Vec<Sector>,
    pub numsectors: usize,
    pub subsectors: Vec<Subsector>,
    pub nodes: Vec<Node>,
    pub lines: Vec<Line>,
    pub sides: Vec<Side>,
    /// The BLOCKMAP lump as 16-bit words, header included.
    pub blockmaplump: Vec<i16>,
    /// The offset of the block offsets inside `blockmaplump` (4 after the header).
    pub blockmap: usize,
    pub bmapwidth: i32,
    pub bmapheight: i32,
    pub bmaporgx: Fixed,
    pub bmaporgy: Fixed,
    pub blocklinks: Vec<Option<MobjId>>,
    pub rejectmatrix: Vec<u8>,
    pub deathmatchstarts: Vec<MapThing>,
    pub playerstarts: [MapThing; 4],
    pub playerstartsingame: [bool; 4],
    pub totallines: usize,
}

pub const MAPBLOCKSHIFT: i32 = FRACBITS + 7;
pub const MAXRADIUS: Fixed = 32 << FRACBITS;

fn rd16(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}

fn rdu16(b: &[u8], at: usize) -> usize {
    u16::from_le_bytes([b[at], b[at + 1]]) as usize
}

/// Returns the 8-byte lump field as a name, cut at the first zero byte.
fn name8(b: &[u8]) -> String {
    let end = b
        .iter()
        .take(8)
        .position(|&c| c == 0)
        .unwrap_or(8.min(b.len()));
    String::from_utf8_lossy(&b[..end]).into_owned()
}

fn lump_data(d: &Doom, lump: usize) -> Vec<u8> {
    crate::w_wad::lump(&d.w_wad, lump).to_vec()
}

/// P_LoadVertexes.
fn load_vertexes(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    d.p_setup.vertexes = data
        .chunks_exact(4)
        .map(|c| Vertex {
            x: (rd16(c, 0) as i32) << FRACBITS,
            y: (rd16(c, 2) as i32) << FRACBITS,
        })
        .collect();
}

/// P_LoadSegs.
fn load_segs(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let mut segs = Vec::with_capacity(data.len() / 12);
    for c in data.chunks_exact(12) {
        let linedef = rdu16(c, 6);
        let side = rd16(c, 8) as usize & 1;
        let ldef = d.p_setup.lines[linedef];
        let sidedef = ldef.sidenum[side] as usize;
        let backsector = if ldef.flags & ML_TWOSIDED != 0 {
            let other = ldef.sidenum[side ^ 1];
            if other < 0 || other as usize >= d.p_setup.sides.len() {
                Some(sector_at_null_address(d))
            } else {
                Some(d.p_setup.sides[other as usize].sector)
            }
        } else {
            None
        };
        segs.push(Seg {
            v1: rdu16(c, 0),
            v2: rdu16(c, 2),
            angle: ((rd16(c, 4) as i32) << FRACBITS) as u32,
            offset: (rd16(c, 10) as i32) << FRACBITS,
            sidedef,
            linedef,
            frontsector: d.p_setup.sides[sidedef].sector,
            backsector,
        });
    }
    d.p_setup.segs = segs;
}

/// GetSectorAtNullAddress: a zero sector with the DOS memory values as heights.
fn sector_at_null_address(d: &mut Doom) -> usize {
    let n = d.p_setup.numsectors;
    if d.p_setup.sectors.len() == n {
        let floorheight = crate::i_system::get_memory_value(d, 0);
        let ceilingheight = crate::i_system::get_memory_value(d, 4);
        d.p_setup.sectors.push(Sector {
            floorheight,
            ceilingheight,
            ..Default::default()
        });
    }
    n
}

/// P_LoadSubsectors.
fn load_subsectors(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    d.p_setup.subsectors = data
        .chunks_exact(4)
        .map(|c| Subsector {
            sector: 0,
            numlines: rdu16(c, 0),
            firstline: rdu16(c, 2),
        })
        .collect();
}

/// P_LoadSectors.
fn load_sectors(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let mut sectors = Vec::with_capacity(data.len() / 26);
    for c in data.chunks_exact(26) {
        let floorpic = crate::r_data::flat_num_for_name(d, &name8(&c[4..12]));
        let ceilingpic = crate::r_data::flat_num_for_name(d, &name8(&c[12..20]));
        sectors.push(Sector {
            floorheight: (rd16(c, 0) as i32) << FRACBITS,
            ceilingheight: (rd16(c, 2) as i32) << FRACBITS,
            floorpic,
            ceilingpic,
            lightlevel: rd16(c, 20) as i32,
            special: rd16(c, 22) as i32,
            tag: rd16(c, 24) as i32,
            ..Default::default()
        });
    }
    d.p_setup.numsectors = sectors.len();
    d.p_setup.sectors = sectors;
}

/// P_LoadNodes.
fn load_nodes(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let f = |c: &[u8], at: usize| (rd16(c, at) as i32) << FRACBITS;
    d.p_setup.nodes = data
        .chunks_exact(28)
        .map(|c| {
            let mut bbox = [[0; 4]; 2];
            for (j, b) in bbox.iter_mut().enumerate() {
                for (k, v) in b.iter_mut().enumerate() {
                    *v = f(c, 8 + j * 8 + k * 2);
                }
            }
            Node {
                x: f(c, 0),
                y: f(c, 2),
                dx: f(c, 4),
                dy: f(c, 6),
                bbox,
                children: [rdu16(c, 24) as u16, rdu16(c, 26) as u16],
            }
        })
        .collect();
}

/// P_LoadThings. The first Doom II monster in a non-commercial game stops the load.
fn load_things(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    for c in data.chunks_exact(10) {
        let mt = MapThing {
            x: rd16(c, 0),
            y: rd16(c, 2),
            angle: rd16(c, 4),
            type_: rd16(c, 6),
            options: rd16(c, 8),
        };
        if d.d_main.gamemode != crate::d_main::COMMERCIAL
            && matches!(mt.type_, 68 | 64 | 88 | 89 | 69 | 67 | 71 | 65 | 66 | 84)
        {
            break;
        }
        crate::p_mobj::spawn_map_thing(d, &mt);
    }
}

/// P_LoadLineDefs.
fn load_line_defs(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let mut lines = Vec::with_capacity(data.len() / 14);
    for c in data.chunks_exact(14) {
        let (v1, v2) = (rdu16(c, 0), rdu16(c, 2));
        let (p1, p2) = (d.p_setup.vertexes[v1], d.p_setup.vertexes[v2]);
        let dx = p2.x.wrapping_sub(p1.x);
        let dy = p2.y.wrapping_sub(p1.y);
        let slopetype = if dx == 0 {
            SlopeType::Vertical
        } else if dy == 0 {
            SlopeType::Horizontal
        } else if fixed_div(dy, dx) > 0 {
            SlopeType::Positive
        } else {
            SlopeType::Negative
        };
        let mut bbox = [0; 4];
        if p1.x < p2.x {
            bbox[BOXLEFT] = p1.x;
            bbox[BOXRIGHT] = p2.x;
        } else {
            bbox[BOXLEFT] = p2.x;
            bbox[BOXRIGHT] = p1.x;
        }
        if p1.y < p2.y {
            bbox[BOXBOTTOM] = p1.y;
            bbox[BOXTOP] = p2.y;
        } else {
            bbox[BOXBOTTOM] = p2.y;
            bbox[BOXTOP] = p1.y;
        }
        let sidenum = [rd16(c, 10) as i32, rd16(c, 12) as i32];
        let sec = |n: i32| {
            if n != -1 {
                Some(d.p_setup.sides[n as usize].sector)
            } else {
                None
            }
        };
        lines.push(Line {
            v1,
            v2,
            dx,
            dy,
            flags: rd16(c, 4) as i32,
            special: rd16(c, 6) as i32,
            tag: rd16(c, 8) as i32,
            sidenum,
            bbox,
            slopetype,
            frontsector: sec(sidenum[0]),
            backsector: sec(sidenum[1]),
            validcount: 0,
            specialdata: None,
        });
    }
    d.p_setup.lines = lines;
}

/// P_LoadSideDefs.
fn load_side_defs(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let mut sides = Vec::with_capacity(data.len() / 30);
    for c in data.chunks_exact(30) {
        let toptexture = crate::r_data::texture_num_for_name(d, &name8(&c[4..12]));
        let bottomtexture = crate::r_data::texture_num_for_name(d, &name8(&c[12..20]));
        let midtexture = crate::r_data::texture_num_for_name(d, &name8(&c[20..28]));
        sides.push(Side {
            textureoffset: (rd16(c, 0) as i32) << FRACBITS,
            rowoffset: (rd16(c, 2) as i32) << FRACBITS,
            toptexture,
            bottomtexture,
            midtexture,
            sector: rdu16(c, 28),
        });
    }
    d.p_setup.sides = sides;
}

/// P_LoadBlockMap.
fn load_block_map(d: &mut Doom, lump: usize) {
    let data = lump_data(d, lump);
    let s = &mut d.p_setup;
    s.blockmaplump = data.chunks_exact(2).map(|c| rd16(c, 0)).collect();
    s.blockmap = 4;
    s.bmaporgx = (s.blockmaplump[0] as i32) << FRACBITS;
    s.bmaporgy = (s.blockmaplump[1] as i32) << FRACBITS;
    s.bmapwidth = s.blockmaplump[2] as i32;
    s.bmapheight = s.blockmaplump[3] as i32;
    s.blocklinks = vec![None; (s.bmapwidth * s.bmapheight).max(0) as usize];
}

/// M_ClearBox and M_AddToBox. The original updates only one side of each axis per point.
fn add_to_box(b: &mut [Fixed; 4], x: Fixed, y: Fixed) {
    if x < b[BOXLEFT] {
        b[BOXLEFT] = x;
    } else if x > b[BOXRIGHT] {
        b[BOXRIGHT] = x;
    }
    if y < b[BOXBOTTOM] {
        b[BOXBOTTOM] = y;
    } else if y > b[BOXTOP] {
        b[BOXTOP] = y;
    }
}

/// P_GroupLines: builds the sector line lists, the sector bounding boxes, and the sound origins.
fn group_lines(d: &mut Doom) {
    let s = &mut d.p_setup;
    for i in 0..s.subsectors.len() {
        let seg = s.segs[s.subsectors[i].firstline];
        s.subsectors[i].sector = s.sides[seg.sidedef].sector;
    }
    let mut total = 0;
    for sec in s.sectors[..s.numsectors].iter_mut() {
        sec.lines.clear();
    }
    for (i, li) in s.lines.iter().enumerate() {
        total += 1;
        if let Some(f) = li.frontsector {
            s.sectors[f].lines.push(i);
        }
        if let Some(b) = li.backsector {
            if Some(b) != li.frontsector {
                s.sectors[b].lines.push(i);
                total += 1;
            }
        }
    }
    s.totallines = total;
    for i in 0..s.numsectors {
        let mut bbox = [i32::MIN, i32::MAX, i32::MAX, i32::MIN];
        for &l in &s.sectors[i].lines {
            let (v1, v2) = (s.vertexes[s.lines[l].v1], s.vertexes[s.lines[l].v2]);
            add_to_box(&mut bbox, v1.x, v1.y);
            add_to_box(&mut bbox, v2.x, v2.y);
        }
        let sec = &mut s.sectors[i];
        sec.soundorg_x = bbox[BOXRIGHT].wrapping_add(bbox[BOXLEFT]) / 2;
        sec.soundorg_y = bbox[BOXTOP].wrapping_add(bbox[BOXBOTTOM]) / 2;
        let block = (bbox[BOXTOP]
            .wrapping_sub(s.bmaporgy)
            .wrapping_add(MAXRADIUS))
            >> MAPBLOCKSHIFT;
        sec.blockbox[BOXTOP] = if block >= s.bmapheight {
            s.bmapheight - 1
        } else {
            block
        };
        let block = (bbox[BOXBOTTOM]
            .wrapping_sub(s.bmaporgy)
            .wrapping_sub(MAXRADIUS))
            >> MAPBLOCKSHIFT;
        sec.blockbox[BOXBOTTOM] = block.max(0);
        let block = (bbox[BOXRIGHT]
            .wrapping_sub(s.bmaporgx)
            .wrapping_add(MAXRADIUS))
            >> MAPBLOCKSHIFT;
        sec.blockbox[BOXRIGHT] = if block >= s.bmapwidth {
            s.bmapwidth - 1
        } else {
            block
        };
        let block = (bbox[BOXLEFT]
            .wrapping_sub(s.bmaporgx)
            .wrapping_sub(MAXRADIUS))
            >> MAPBLOCKSHIFT;
        sec.blockbox[BOXLEFT] = block.max(0);
    }
}

/// PadRejectArray: fills a short REJECT lump with the bytes that follow it in the original memory.
fn pad_reject_array(d: &Doom, array: &mut [u8]) {
    let len = array.len();
    let rejectpad: [u32; 4] = [
        ((d.p_setup.totallines as u32 * 4 + 3) & !3) + 24,
        0,
        50,
        0x1d4a11,
    ];
    for i in 0..len.min(16) {
        array[i] = (rejectpad[i / 4] >> ((i % 4) * 8)) as u8;
    }
    if len > 16 {
        eprintln!(
            "PadRejectArray: REJECT lump too short to pad! ({} > {})",
            len, 16
        );
        let padvalue = if crate::m_argv::check_parm(&d.m_argv, "-reject_pad_with_ff") != 0 {
            0xff
        } else {
            0
        };
        for b in &mut array[16..] {
            *b = padvalue;
        }
    }
}

/// P_LoadReject.
fn load_reject(d: &mut Doom, lump: usize) {
    let n = d.p_setup.numsectors;
    let minlength = (n * n).div_ceil(8);
    let mut data = lump_data(d, lump);
    let lumplen = data.len();
    if lumplen < minlength {
        data.resize(minlength, 0);
        let mut tail = data.split_off(lumplen);
        pad_reject_array(d, &mut tail);
        data.extend_from_slice(&tail);
    }
    d.p_setup.rejectmatrix = data;
}

/// P_SetupLevel.
pub fn setup_level(d: &mut Doom, episode: i32, map: i32, _playermask: i32, _skill: i32) {
    let g = &mut d.g_game;
    g.totalkills = 0;
    g.totalitems = 0;
    g.totalsecret = 0;
    g.wminfo.maxfrags = 0;
    g.wminfo.partime = 180;
    for p in g.players.iter_mut() {
        p.killcount = 0;
        p.secretcount = 0;
        p.itemcount = 0;
    }
    let cp = g.consoleplayer;
    g.players[cp].viewz = 1;

    crate::s_sound::start(d);
    d.p_tick.init_thinkers();

    let lumpname = if d.d_main.gamemode == crate::d_main::COMMERCIAL {
        if map < 10 {
            format!("map0{}", map)
        } else {
            format!("map{}", map)
        }
    } else {
        let e = (b'0' as i32 + episode) as u8 as char;
        let m = (b'0' as i32 + map) as u8 as char;
        format!("E{}M{}", e, m)
    };
    let lumpnum = crate::w_wad::get_num_for_name(d, &lumpname);
    d.p_tick.leveltime = 0;

    load_block_map(d, lumpnum + 10);
    load_vertexes(d, lumpnum + 4);
    load_sectors(d, lumpnum + 8);
    load_side_defs(d, lumpnum + 3);
    load_line_defs(d, lumpnum + 2);
    load_subsectors(d, lumpnum + 6);
    load_nodes(d, lumpnum + 7);
    load_segs(d, lumpnum + 5);
    group_lines(d);
    load_reject(d, lumpnum + 9);

    d.g_game.bodyqueslot = 0;
    d.p_setup.deathmatchstarts.clear();
    load_things(d, lumpnum + 1);

    if d.g_game.deathmatch != 0 {
        for i in 0..crate::g_game::MAXPLAYERS {
            if d.g_game.playeringame[i] {
                d.g_game.players[i].mo = None;
                crate::g_game::death_match_spawn_player(d, i);
            }
        }
    }

    d.p_mobj.iquehead = 0;
    d.p_mobj.iquetail = 0;

    crate::p_spec::spawn_specials(d);

    if d.g_game.precache {
        crate::r_data::precache_level(d);
    }
}

/// P_Init.
pub fn init(d: &mut Doom) {
    crate::p_switch::init_switch_list(d);
    crate::p_spec::init_pic_anims(d);
    crate::r_things::init_sprites(d);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_to_box_keeps_the_else_if_quirk() {
        let mut b = [i32::MIN, i32::MAX, i32::MAX, i32::MIN];
        add_to_box(&mut b, 5, 7);
        assert_eq!(b, [i32::MIN, 7, 5, i32::MIN]);
        add_to_box(&mut b, 9, 9);
        assert_eq!(b, [9, 7, 5, 9]);
    }
}

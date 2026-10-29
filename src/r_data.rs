//! Refresh data: wall textures, flats, sprite lumps, and colormaps.

use crate::i_system;
use crate::m_fixed::{Fixed, FRACBITS};
use crate::p_tick::{ThinkerData, ZoneEvent};
use crate::w_wad;
use crate::z_zone::{PU_CACHE, PU_LEVEL, PU_LEVSPEC, PU_STATIC};
use crate::Doom;

/// A model address for owners on the original stack. Its *user = NULL write has no effect.
const STACK_USER: u64 = 0x1_6fdf_f000;

/// One patch placement inside a composite texture (texpatch_t).
#[derive(Clone, Copy, Default)]
pub struct TexPatch {
    pub originx: i32,
    pub originy: i32,
    /// The lump number of the patch.
    pub patch: i32,
}

/// A wall texture definition (texture_t).
#[derive(Clone, Default)]
pub struct Texture {
    pub name: [u8; 8],
    pub width: i16,
    pub height: i16,
    pub patches: Vec<TexPatch>,
}

/// The source of a column of texture data: a patch lump or a composite texture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColSrc {
    /// The lump number and the byte offset in the lump.
    Lump(usize, usize),
    /// The texture number and the byte offset in its composite block.
    Composite(usize, usize),
}

#[derive(Default)]
pub struct State {
    pub firstflat: i32,
    pub lastflat: i32,
    pub numflats: i32,
    pub firstpatch: i32,
    pub lastpatch: i32,
    pub numpatches: i32,
    pub firstspritelump: i32,
    pub lastspritelump: i32,
    pub numspritelumps: i32,
    pub numtextures: i32,
    pub textures: Vec<Texture>,
    pub texturewidthmask: Vec<i32>,
    /// The texture heights in fixed point, for the texture position.
    pub textureheight: Vec<Fixed>,
    pub texturecompositesize: Vec<i32>,
    pub texturecolumnlump: Vec<Vec<i16>>,
    pub texturecolumnofs: Vec<Vec<u16>>,
    /// The composite blocks. An empty block is not generated yet.
    pub texturecomposite: Vec<Vec<u8>>,
    /// The zone offset of the texturecomposite pointer array, or 0 without the zone model.
    pub zcomposite: usize,
    /// The animation translation for flats (one extra entry, as in the original).
    pub flattranslation: Vec<i32>,
    /// The animation translation for wall textures (one extra entry, as in the original).
    pub texturetranslation: Vec<i32>,
    /// Sprite widths and offsets in fixed point, indexed by sprite lump.
    pub spritewidth: Vec<Fixed>,
    pub spriteoffset: Vec<Fixed>,
    pub spritetopoffset: Vec<Fixed>,
    /// The COLORMAP lump: 34 light tables of 256 bytes.
    pub colormaps: Vec<u8>,
}

fn rd_i16(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}

fn rd_i32(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// Returns the name bytes up to the first NUL as text.
pub fn name_str(name: &[u8]) -> String {
    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
    String::from_utf8_lossy(&name[..end]).into_owned()
}

fn upper8(name: &[u8]) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, &c) in name.iter().take(8).enumerate() {
        if c == 0 {
            break;
        }
        out[i] = c.to_ascii_uppercase();
    }
    out
}

/// R_DrawColumnInCache: copies the posts of a patch column into a composite block.
fn draw_column_in_cache(
    patch: &[u8],
    mut col: usize,
    cache: &mut [u8],
    originy: i32,
    cacheheight: i32,
) {
    while col < patch.len() && patch[col] != 0xff {
        let topdelta = patch[col] as i32;
        let length = patch.get(col + 1).copied().unwrap_or(0) as i32;
        let source = col + 3;
        let mut count = length;
        let mut position = originy + topdelta;
        if position < 0 {
            // The original copies from the post start even when the post starts above the texture.
            count += position;
            position = 0;
        }
        if position + count > cacheheight {
            count = cacheheight - position;
        }
        if count > 0 {
            for i in 0..count as usize {
                cache[position as usize + i] = patch.get(source + i).copied().unwrap_or(0);
            }
        }
        col += length as usize + 4;
    }
}

/// Returns the zone offset of a composite block, or 0 when the zone has no such block.
pub fn composite_ofs(z: &crate::z_zone::State, r: &State, t: usize) -> usize {
    if z.mem.is_empty() || r.zcomposite == 0 {
        return 0;
    }
    z.ptr_at(r.zcomposite + 8 * t)
}

/// R_GenerateComposite: builds the composite block of a multi-patch texture.
/// The original does not clear the block, so the gaps between the posts keep stale zone bytes.
fn generate_composite(d: &mut Doom, texnum: usize) {
    let tex = d.r_data.textures[texnum].clone();
    let size = d.r_data.texturecompositesize[texnum];
    let zp = if d.r_data.zcomposite != 0 {
        let owner = d.z_zone.addr(d.r_data.zcomposite + 8 * texnum);
        d.z_zone.malloc(size, PU_STATIC, owner)
    } else {
        0
    };
    let mut block = if zp != 0 {
        d.z_zone.mem[zp..zp + size as usize].to_vec()
    } else {
        vec![0u8; size.max(1) as usize]
    };
    for p in &tex.patches {
        d.z_zone.cache_lump(&d.w_wad, p.patch as usize, PU_CACHE);
        let realpatch = w_wad::lump(&d.w_wad, p.patch as usize);
        let x1 = p.originx;
        let mut x2 = x1 + rd_i16(realpatch, 0) as i32;
        let mut x = x1.max(0);
        if x2 > tex.width as i32 {
            x2 = tex.width as i32;
        }
        while x < x2 {
            if d.r_data.texturecolumnlump[texnum][x as usize] < 0 {
                let ofs = rd_i32(realpatch, 8 + 4 * (x - x1) as usize) as usize;
                let at = d.r_data.texturecolumnofs[texnum][x as usize] as usize;
                draw_column_in_cache(
                    realpatch,
                    ofs,
                    &mut block[at..],
                    p.originy,
                    tex.height as i32,
                );
            }
            x += 1;
        }
    }
    if zp != 0 {
        d.z_zone.mem[zp..zp + block.len()].copy_from_slice(&block);
        d.z_zone.change_tag(zp, PU_CACHE);
    }
    d.r_data.texturecomposite[texnum] = block;
}

/// R_GenerateLookup: finds the patch of each column and sizes the composite block.
fn generate_lookup(d: &mut Doom, texnum: usize) {
    let tex = d.r_data.textures[texnum].clone();
    let width = tex.width.max(0) as usize;
    d.r_data.texturecomposite[texnum] = Vec::new();
    d.r_data.texturecompositesize[texnum] = 0;
    if d.r_data.zcomposite != 0 {
        let at = d.r_data.zcomposite + 8 * texnum;
        d.z_zone.mem[at..at + 8].fill(0);
    }
    let mut patchcount = vec![0u8; width];
    let zcount = d.z_zone.malloc(tex.width as i32, PU_STATIC, STACK_USER);
    for p in &tex.patches {
        d.z_zone.cache_lump(&d.w_wad, p.patch as usize, PU_CACHE);
        let realpatch = w_wad::lump(&d.w_wad, p.patch as usize);
        let x1 = p.originx;
        let mut x2 = x1 + rd_i16(realpatch, 0) as i32;
        let mut x = x1.max(0);
        if x2 > width as i32 {
            x2 = width as i32;
        }
        while x < x2 {
            patchcount[x as usize] = patchcount[x as usize].wrapping_add(1);
            d.r_data.texturecolumnlump[texnum][x as usize] = p.patch as i16;
            d.r_data.texturecolumnofs[texnum][x as usize] =
                (rd_i32(realpatch, 8 + 4 * (x - x1) as usize) + 3) as u16;
            x += 1;
        }
    }
    if zcount != 0 {
        d.z_zone.mem[zcount..zcount + width].copy_from_slice(&patchcount);
    }
    for x in 0..width {
        if patchcount[x] == 0 {
            // The original returns without Z_Free, so the block stays allocated.
            println!(
                "R_GenerateLookup: column without a patch ({})",
                name_str(&tex.name)
            );
            return;
        }
        if patchcount[x] > 1 {
            d.r_data.texturecolumnlump[texnum][x] = -1;
            d.r_data.texturecolumnofs[texnum][x] = d.r_data.texturecompositesize[texnum] as u16;
            if d.r_data.texturecompositesize[texnum] > 0x10000 - tex.height as i32 {
                i_system::error(d, &format!("R_GenerateLookup: texture {} is >64k", texnum));
            }
            d.r_data.texturecompositesize[texnum] += tex.height as i32;
        }
    }
    d.z_zone.free(zcount);
}

/// R_GetColumn: returns the source of one texture column.
pub fn get_column(d: &mut Doom, tex: usize, col: i32) -> ColSrc {
    let col = (col & d.r_data.texturewidthmask[tex]) as usize;
    let lump = d.r_data.texturecolumnlump[tex][col];
    let ofs = d.r_data.texturecolumnofs[tex][col] as usize;
    if lump > 0 {
        d.z_zone.cache_lump(&d.w_wad, lump as usize, PU_CACHE);
        return ColSrc::Lump(lump as usize, ofs);
    }
    let purged = d.r_data.zcomposite != 0
        && !d.z_zone.mem.is_empty()
        && composite_ofs(&d.z_zone, &d.r_data, tex) == 0;
    if d.r_data.texturecomposite[tex].is_empty() || purged {
        generate_composite(d, tex);
    }
    ColSrc::Composite(tex, ofs)
}

/// R_InitTextures: reads PNAMES, TEXTURE1, and TEXTURE2.
fn init_textures(d: &mut Doom) {
    let pnames = w_wad::get_num_for_name(d, "PNAMES");
    let names = w_wad::lump(&d.w_wad, pnames).to_vec();
    let nummappatches = rd_i32(&names, 0).max(0) as usize;
    d.z_zone.cache_lump(&d.w_wad, pnames, PU_STATIC);
    let zlookup = d.z_zone.malloc(4 * nummappatches as i32, PU_STATIC, 0);
    let patchlookup: Vec<i32> = (0..nummappatches)
        .map(|i| w_wad::check_num_for_name(&d.w_wad, &name_str(&names[4 + 8 * i..12 + 8 * i])))
        .collect();
    d.z_zone.release_lump(pnames);

    let tex1lump = w_wad::get_num_for_name(d, "TEXTURE1");
    d.z_zone.cache_lump(&d.w_wad, tex1lump, PU_STATIC);
    let maptex1 = w_wad::lump(&d.w_wad, tex1lump).to_vec();
    let numtextures1 = rd_i32(&maptex1, 0);
    let maxoff1 = w_wad::lump_length(&d.w_wad, tex1lump) as i32;
    let (maptex2, numtextures2, maxoff2) = if w_wad::check_num_for_name(&d.w_wad, "TEXTURE2") != -1
    {
        let l = w_wad::get_num_for_name(d, "TEXTURE2");
        d.z_zone.cache_lump(&d.w_wad, l, PU_STATIC);
        let m = w_wad::lump(&d.w_wad, l).to_vec();
        let n = rd_i32(&m, 0);
        (m, n, w_wad::lump_length(&d.w_wad, l) as i32)
    } else {
        (Vec::new(), 0, 0)
    };
    let numtextures = (numtextures1 + numtextures2).max(0) as usize;
    for (i, size) in [8, 8, 8, 8, 4, 4, 4].into_iter().enumerate() {
        let p = d.z_zone.malloc(size * numtextures as i32, PU_STATIC, 0);
        if i == 3 {
            d.r_data.zcomposite = p;
        }
    }
    let r = &mut d.r_data;
    r.numtextures = numtextures as i32;
    r.textures = Vec::with_capacity(numtextures);
    r.texturecolumnlump = Vec::with_capacity(numtextures);
    r.texturecolumnofs = Vec::with_capacity(numtextures);
    r.texturecomposite = vec![Vec::new(); numtextures];
    r.texturecompositesize = vec![0; numtextures];
    r.texturewidthmask = Vec::with_capacity(numtextures);
    r.textureheight = Vec::with_capacity(numtextures);

    // The original looks these up for its progress box. The lookups can fail with an error.
    w_wad::get_num_for_name(d, "S_START");
    w_wad::get_num_for_name(d, "S_END");

    let mut maptex = &maptex1;
    let mut maxoff = maxoff1;
    let mut dir = 4usize;
    for i in 0..numtextures {
        if i & 63 == 0 {
            print!(".");
        }
        if i as i32 == numtextures1 {
            maptex = &maptex2;
            maxoff = maxoff2;
            dir = 4;
        }
        let offset = rd_i32(maptex, dir);
        dir += 4;
        if offset > maxoff {
            i_system::error(d, "R_InitTextures: bad texture directory");
        }
        let m = offset as usize;
        let mut tex = Texture {
            name: [0; 8],
            width: rd_i16(maptex, m + 12),
            height: rd_i16(maptex, m + 14),
            patches: Vec::new(),
        };
        tex.name.copy_from_slice(&maptex[m..m + 8]);
        let patchcount = rd_i16(maptex, m + 20).max(0) as usize;
        for j in 0..patchcount {
            let p = m + 22 + 10 * j;
            let idx = rd_i16(maptex, p + 4) as usize;
            let patch = patchlookup.get(idx).copied().unwrap_or(-1);
            tex.patches.push(TexPatch {
                originx: rd_i16(maptex, p) as i32,
                originy: rd_i16(maptex, p + 2) as i32,
                patch,
            });
            if patch == -1 {
                i_system::error(
                    d,
                    &format!(
                        "R_InitTextures: Missing patch in texture {}",
                        name_str(&tex.name)
                    ),
                );
            }
        }
        let width = tex.width.max(0) as usize;
        d.z_zone
            .malloc(32 + 8 * tex.patches.len() as i32, PU_STATIC, 0);
        d.z_zone.malloc(2 * tex.width as i32, PU_STATIC, 0);
        d.z_zone.malloc(2 * tex.width as i32, PU_STATIC, 0);
        let mut j = 1;
        while j * 2 <= tex.width as i32 {
            j <<= 1;
        }
        let r = &mut d.r_data;
        r.texturecolumnlump.push(vec![0; width]);
        r.texturecolumnofs.push(vec![0; width]);
        r.texturewidthmask.push(j - 1);
        r.textureheight.push((tex.height as i32) << FRACBITS);
        r.textures.push(tex);
    }
    d.z_zone.free(zlookup);
    d.z_zone.release_lump(tex1lump);
    if !maptex2.is_empty() {
        let l = w_wad::get_num_for_name(d, "TEXTURE2");
        d.z_zone.release_lump(l);
    }
    for i in 0..numtextures {
        generate_lookup(d, i);
    }
    d.z_zone.malloc(4 * numtextures as i32 + 4, PU_STATIC, 0);
    d.z_zone.malloc(8 * numtextures as i32, PU_STATIC, 0);
    d.r_data.texturetranslation = (0..=numtextures as i32).collect();
}

/// R_InitFlats: finds the flats between F_START and F_END.
fn init_flats(d: &mut Doom) {
    let firstflat = w_wad::get_num_for_name(d, "F_START") as i32 + 1;
    let lastflat = w_wad::get_num_for_name(d, "F_END") as i32 - 1;
    let r = &mut d.r_data;
    r.firstflat = firstflat;
    r.lastflat = lastflat;
    r.numflats = lastflat - firstflat + 1;
    r.flattranslation = (0..=r.numflats.max(0)).collect();
    d.z_zone
        .malloc(4 * (lastflat - firstflat) + 8, PU_STATIC, 0);
}

/// R_InitSpriteLumps: reads the size and offsets of each sprite lump.
fn init_sprite_lumps(d: &mut Doom) {
    let first = w_wad::get_num_for_name(d, "S_START") as i32 + 1;
    let last = w_wad::get_num_for_name(d, "S_END") as i32 - 1;
    let r = &mut d.r_data;
    r.firstspritelump = first;
    r.lastspritelump = last;
    r.numspritelumps = last - first + 1;
    let n = r.numspritelumps.max(0) as usize;
    r.spritewidth = vec![0; n];
    r.spriteoffset = vec![0; n];
    r.spritetopoffset = vec![0; n];
    for _ in 0..3 {
        d.z_zone.malloc(4 * (last - first + 1), PU_STATIC, 0);
    }
    for i in 0..n {
        if i & 63 == 0 {
            print!(".");
        }
        d.z_zone.cache_lump(&d.w_wad, first as usize + i, PU_CACHE);
        let patch = w_wad::lump(&d.w_wad, first as usize + i);
        d.r_data.spritewidth[i] = (rd_i16(patch, 0) as i32) << FRACBITS;
        d.r_data.spriteoffset[i] = (rd_i16(patch, 4) as i32) << FRACBITS;
        d.r_data.spritetopoffset[i] = (rd_i16(patch, 6) as i32) << FRACBITS;
    }
}

/// R_InitColormaps: loads the COLORMAP lump.
fn init_colormaps(d: &mut Doom) {
    let lump = w_wad::get_num_for_name(d, "COLORMAP");
    d.z_zone.cache_lump(&d.w_wad, lump, PU_STATIC);
    let mut c = w_wad::lump(&d.w_wad, lump).to_vec();
    // Keep 34 full tables even when the lump is short.
    if c.len() < 34 * 256 {
        c.resize(34 * 256, 0);
    }
    d.r_data.colormaps = c;
}

/// R_InitData: loads all refresh data. It prints progress dots.
pub fn init_data(d: &mut Doom) {
    init_textures(d);
    print!(".");
    init_flats(d);
    print!(".");
    init_sprite_lumps(d);
    print!(".");
    init_colormaps(d);
}

/// R_FlatNumForName: returns the flat number of a lump name, or stops with an error.
pub fn flat_num_for_name(d: &mut Doom, name: &str) -> i32 {
    let i = w_wad::check_num_for_name(&d.w_wad, name);
    if i == -1 {
        let short: String = name.chars().take(8).collect();
        i_system::error(d, &format!("R_FlatNumForName: {} not found", short));
    }
    i - d.r_data.firstflat
}

/// R_CheckTextureNumForName: returns the texture number, 0 for "-", or -1.
pub fn check_texture_num_for_name(d: &Doom, name: &str) -> i32 {
    if name.as_bytes().first() == Some(&b'-') {
        return 0;
    }
    let key = upper8(name.as_bytes());
    for (i, t) in d.r_data.textures.iter().enumerate() {
        if upper8(&t.name) == key {
            return i as i32;
        }
    }
    -1
}

/// R_TextureNumForName: returns the texture number, or stops with an error.
pub fn texture_num_for_name(d: &mut Doom, name: &str) -> i32 {
    let i = check_texture_num_for_name(d, name);
    if i == -1 {
        i_system::error(d, &format!("R_TextureNumForName: {} not found", name));
    }
    i
}

/// Replays the zone calls of G_DoPlayDemo, P_SetupLevel, and R_PrecacheLevel.
/// `events` are the thinker events of the setup, before its first tic.
fn setup_level_zone(d: &mut Doom, events: &[ZoneEvent]) {
    if d.g_game.demoplayback {
        let l = w_wad::get_num_for_name(d, &d.g_game.defdemoname.clone());
        d.z_zone.cache_lump(&d.w_wad, l, PU_STATIC);
        d.z_zone.demo_lump = l;
    }
    sync_music(d);
    d.z_zone.free_tags(PU_LEVEL, PU_LEVSPEC);
    d.z_zone.thinkers.clear();
    let (e, m) = (d.g_game.gameepisode, d.g_game.gamemap);
    let name = if d.d_main.gamemode == crate::d_main::COMMERCIAL {
        format!("map{m:02}")
    } else {
        format!("E{e}M{m}")
    };
    let base = w_wad::get_num_for_name(d, &name);
    let z = &mut d.z_zone;
    let mut at = [0; 9];
    z.malloc_copy(w_wad::lump(&d.w_wad, base + 10), PU_LEVEL, 0);
    let n = (d.p_setup.bmapwidth * d.p_setup.bmapheight * 8).max(0) as usize;
    z.malloc_copy(&vec![0; n], PU_LEVEL, 0);
    for (k, unit, size, zero) in [
        (4, 4, 8, false),
        (8, 26, 128, true),
        (3, 30, 24, true),
        (2, 14, 88, true),
        (6, 4, 16, false),
        (7, 28, 52, false),
        (5, 12, 56, true),
    ] {
        let n = w_wad::lump_length(&d.w_wad, base + k) / unit * size;
        at[k] = if zero {
            z.malloc_copy(&vec![0; n], PU_LEVEL, 0)
        } else {
            z.malloc(n as i32, PU_LEVEL, 0)
        };
        z.cache_lump(&d.w_wad, base + k, PU_STATIC);
        z.release_lump(base + k);
    }
    z.subsectors = at[6];
    // P_LoadSegs fills each seg_t. Wall columns read past a short texture into these bytes.
    for (i, g) in d.p_setup.segs.iter().enumerate().filter(|_| at[5] != 0) {
        let side = &d.p_setup.sides[g.sidedef];
        let sector = |s: usize| z.addr(at[8] + 128 * s);
        let mut b = [0u8; 56];
        for (o, v) in [
            (0, z.addr(at[4] + 8 * g.v1)),
            (8, z.addr(at[4] + 8 * g.v2)),
            (0x10, g.offset as u32 as u64 | (g.angle as u64) << 32),
            (0x18, z.addr(at[3] + 24 * g.sidedef)),
            (0x20, z.addr(at[2] + 88 * g.linedef)),
            (0x28, sector(side.sector)),
            (0x30, g.backsector.map_or(0, sector)),
        ] {
            b[o..o + 8].copy_from_slice(&v.to_le_bytes());
        }
        z.mem[at[5] + 56 * i..][..56].copy_from_slice(&b);
    }
    z.malloc(8 * d.p_setup.totallines as i32, PU_LEVEL, 0);
    let ns = d.p_setup.numsectors;
    if (ns * ns).div_ceil(8) > w_wad::lump_length(&d.w_wad, base + 9) {
        z.malloc_copy(&d.p_setup.rejectmatrix, PU_LEVEL, STACK_USER);
    } else {
        z.cache_lump(&d.w_wad, base + 9, PU_LEVEL);
    }
    z.cache_lump(&d.w_wad, base + 1, PU_STATIC);
    let player = d.g_game.players[d.g_game.consoleplayer].mo;
    let (playpal, minus) = (
        w_wad::check_num_for_name(&d.w_wad, "PLAYPAL"),
        w_wad::check_num_for_name(&d.w_wad, "STTMINUS"),
    );
    let mut things = true;
    let mut spr = vec![0u8; d.r_things.numsprites as usize];
    for e in events {
        let ZoneEvent::Add(id) = *e else { continue };
        if let ThinkerData::Mobj(mo) = &d.p_tick.thinkers[id].data {
            spr[mo.sprite] = 1;
        } else if things {
            // P_LoadThings releases THINGS before P_SpawnSpecials.
            z.release_lump(base + 1);
            things = false;
        }
        thinker_zone(z, &d.p_tick.thinkers, *e);
        if player == Some(id) {
            // ST_Start: ST_Stop sets the palette after the first start; STlib_init caches STTMINUS.
            if z.st_started {
                z.cache_lump(&d.w_wad, playpal as usize, PU_CACHE);
            }
            z.cache_lump(&d.w_wad, minus as usize, PU_STATIC);
            z.st_started = true;
        }
    }
    if things {
        z.release_lump(base + 1);
    }
    if !std::mem::take(&mut z.precache) || d.g_game.demoplayback {
        return;
    }
    let r = &d.r_data;
    let mut lumps: Vec<Vec<usize>> = Vec::new();
    let mut flat = vec![0u8; r.numflats.max(0) as usize];
    for s in &d.p_setup.sectors[..d.p_setup.numsectors] {
        flat[s.floorpic as usize] = 1;
        flat[s.ceilingpic as usize] = 1;
    }
    lumps.push(
        flat.iter()
            .enumerate()
            .filter(|f| *f.1 != 0)
            .map(|f| r.firstflat as usize + f.0)
            .collect(),
    );
    let mut tex = vec![0u8; r.numtextures as usize];
    for s in &d.p_setup.sides {
        for t in [s.toptexture, s.midtexture, s.bottomtexture] {
            tex[t as usize] = 1;
        }
    }
    tex[d.r_sky.skytexture as usize] = 1;
    lumps.push(
        tex.iter()
            .enumerate()
            .filter(|t| *t.1 != 0)
            .flat_map(|t| r.textures[t.0].patches.iter().map(|p| p.patch as usize))
            .collect(),
    );
    let first = r.firstspritelump as usize;
    let sprites = &d.r_things.sprites;
    lumps.push(
        spr.iter()
            .enumerate()
            .filter(|s| *s.1 != 0)
            .flat_map(|s| {
                sprites[s.0]
                    .spriteframes
                    .iter()
                    .flat_map(|f| f.lump.map(|l| first + l as usize))
            })
            .collect(),
    );
    for (present, list) in [flat, tex, spr].iter().zip(lumps) {
        let p = d.z_zone.malloc_copy(present, PU_STATIC, 0);
        for l in list {
            d.z_zone.cache_lump(&d.w_wad, l, PU_CACHE);
        }
        d.z_zone.free(p);
    }
}

/// Allocates or frees the zone block of one thinker event.
fn thinker_zone(z: &mut crate::z_zone::State, thinkers: &[crate::p_tick::Thinker], e: ZoneEvent) {
    match e {
        ZoneEvent::Add(id) => {
            let (size, tag) = match thinkers[id].data {
                ThinkerData::Mobj(_) => (224, PU_LEVEL),
                ThinkerData::Glow(_) | ThinkerData::FireFlicker(_) => (48, PU_LEVSPEC),
                ThinkerData::LightFlash(_) | ThinkerData::Strobe(_) => (56, PU_LEVSPEC),
                ThinkerData::Door(_) | ThinkerData::Floor(_) => (64, PU_LEVSPEC),
                _ => (72, PU_LEVSPEC),
            };
            if z.thinkers.len() <= id {
                z.thinkers.resize(id + 1, 0);
            }
            z.thinkers[id] = z.malloc(size, tag, 0);
        }
        ZoneEvent::Free(id) => {
            mobj_image(z, thinkers, id);
            let p = z.thinkers.get(id).copied().unwrap_or(0);
            z.free(p);
        }
        _ => {}
    }
}

/// Writes the mobj_t bytes of a thinker into its zone block, so that over-reads and freed blocks show them.
/// Static pointers use the unslid addresses of thinkercap, P_MobjThinker, mobjinfo, states, and players.
fn mobj_image(z: &mut crate::z_zone::State, thinkers: &[crate::p_tick::Thinker], id: usize) {
    let (p, ThinkerData::Mobj(m)) = (z.thinkers.get(id).copied().unwrap_or(0), &thinkers[id].data)
    else {
        return;
    };
    if p == 0 {
        return;
    }
    let at = |i: usize| z.thinkers.get(i).map_or(0, |&o| z.addr(o));
    let link = |i: usize| if i == 0 { 0x1_0007_dc98 } else { at(i) };
    let mo = |o: Option<usize>| o.map_or(0, at);
    let func = match thinkers[id].func {
        crate::p_tick::Think::Removed => u64::MAX,
        _ => 0x1_0003_07b4,
    };
    let mut b = [0u8; 224];
    let mut put = |o: usize, v: &[u8]| b[o..o + v.len()].copy_from_slice(v);
    let sp = m.spawnpoint;
    for (o, v) in [
        (0, link(thinkers[id].prev)),
        (8, link(thinkers[id].next)),
        (0x10, func),
        (0x28, mo(m.snext)),
        (0x30, mo(m.sprev)),
        (0x48, mo(m.bnext)),
        (0x50, mo(m.bprev)),
        (0x58, z.addr(z.subsectors + 16 * m.subsector)),
        (0x88, 0x1_0006_a0e4 + 92 * m.type_ as u64),
        (
            0x98,
            if m.state == 0 {
                0
            } else {
                0x1_0006_09c8 + 40 * m.state as u64
            },
        ),
        (0xb0, mo(m.target)),
        (0xc0, m.player.map_or(0, |i| 0x1_0007_9478 + 328 * i as u64)),
        (0xd8, mo(m.tracer)),
    ] {
        put(o, &v.to_le_bytes());
    }
    for (o, v) in [
        (0x18, m.x),
        (0x1c, m.y),
        (0x20, m.z),
        (0x38, m.angle as i32),
        (0x3c, m.sprite as i32),
        (0x40, m.frame),
        (0x60, m.floorz),
        (0x64, m.ceilingz),
        (0x68, m.radius),
        (0x6c, m.height),
        (0x70, m.momx),
        (0x74, m.momy),
        (0x78, m.momz),
        (0x7c, m.validcount),
        (0x80, m.type_ as i32),
        (0x90, m.tics),
        (0xa0, m.flags),
        (0xa4, m.health),
        (0xa8, m.movedir),
        (0xac, m.movecount),
        (0xb8, m.reactiontime),
        (0xbc, m.threshold),
        (0xc8, m.lastlook),
    ] {
        put(o, &v.to_le_bytes());
    }
    for (i, v) in [sp.x, sp.y, sp.angle, sp.type_, sp.options]
        .iter()
        .enumerate()
    {
        put(0xcc + 2 * i, &v.to_le_bytes());
    }
    z.mem[p..p + 224].copy_from_slice(&b);
}

/// S_ChangeMusic: releases the old music lump and caches the new one.
fn sync_music(d: &mut Doom) {
    let (mus, l) = (d.s_sound.mus_playing as usize, d.s_sound.mus_lump);
    if mus != d.z_zone.music && mus != 0 {
        let old = d.z_zone.music_lump;
        d.z_zone.release_lump(old);
        d.z_zone.cache_lump(&d.w_wad, l, PU_STATIC);
        (d.z_zone.music, d.z_zone.music_lump) = (mus, l);
    }
}

/// Replays the zone calls that other modules made since the last frame, in their order:
/// the demo end, the thinker blocks, the level setups, and the music.
pub fn zone_sync(d: &mut Doom) {
    if d.z_zone.demo_lump != 0 && !d.g_game.demoplayback {
        let l = std::mem::take(&mut d.z_zone.demo_lump);
        d.z_zone.release_lump(l);
    }
    let log = std::mem::take(&mut d.p_tick.zone_log);
    let mut i = 0;
    while i < log.len() {
        if log[i] == ZoneEvent::Init {
            let end = log[i + 1..]
                .iter()
                .position(|e| *e == ZoneEvent::Tic)
                .map_or(log.len(), |p| i + 1 + p);
            setup_level_zone(d, &log[i + 1..end]);
            i = end;
        } else {
            thinker_zone(&mut d.z_zone, &d.p_tick.thinkers, log[i]);
            i += 1;
        }
    }
    let mut id = d.p_tick.thinkers[0].next;
    while id != 0 {
        mobj_image(&mut d.z_zone, &d.p_tick.thinkers, id);
        id = d.p_tick.thinkers[id].next;
    }
    sync_music(d);
    crate::z_zone::check(d);
}

/// R_PrecacheLevel. The original loads the lumps of the level into the zone.
/// The Rust build keeps all lumps in memory, so only the zone model sees the effect. zone_sync replays it.
pub fn precache_level(d: &mut Doom) {
    d.z_zone.precache = true;
}

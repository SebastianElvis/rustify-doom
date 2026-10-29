//! Sprites: sprite definitions, projection, sorting, clipping, and the player weapon sprites.

use crate::d_items::PW_INVISIBILITY;
use crate::i_system;
use crate::i_video::SCREENWIDTH;
use crate::info::{NUMSPRITES, SPRNAMES, STATES};
use crate::m_fixed::{fixed_div, fixed_mul, Fixed, FRACBITS, FRACUNIT};
use crate::p_mobj::{MobjId, MF_SHADOW, MF_TRANSLATION, MF_TRANSSHIFT};
use crate::p_pspr::{PspDef, NUMPSPRITES};
use crate::r_bsp::{clip_at, Clip, SIL_BOTTOM, SIL_TOP};
use crate::r_data::ColSrc;
use crate::r_main::{LIGHTLEVELS, LIGHTSCALESHIFT, LIGHTSEGSHIFT, MAXLIGHTSCALE};
use crate::tables::{Angle, ANG45};
use crate::w_wad;
use crate::Doom;

pub const MAXVISSPRITES: usize = 128;
pub const FF_FULLBRIGHT: i32 = 0x8000;
pub const FF_FRAMEMASK: i32 = 0x7fff;
const MINZ: Fixed = FRACUNIT * 4;
const BASEYCENTER: i32 = 100;

/// One frame of a sprite (spriteframe_t). `rotate` is -1 until a lump sets it.
#[derive(Clone, Copy, Debug)]
pub struct SpriteFrame {
    pub rotate: i32,
    pub lump: [i16; 8],
    pub flip: [u8; 8],
}

impl Default for SpriteFrame {
    fn default() -> Self {
        SpriteFrame {
            rotate: -1,
            lump: [-1; 8],
            flip: [0xff; 8],
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct SpriteDef {
    pub numframes: i32,
    pub spriteframes: Vec<SpriteFrame>,
}

/// A visible sprite of the current frame (vissprite_t).
#[derive(Clone, Copy, Default, Debug)]
pub struct Vissprite {
    pub x1: i32,
    pub x2: i32,
    pub gx: Fixed,
    pub gy: Fixed,
    pub gz: Fixed,
    pub gzt: Fixed,
    pub startfrac: Fixed,
    pub scale: Fixed,
    pub xiscale: Fixed,
    pub texturemid: Fixed,
    pub patch: i32,
    /// The colormap offset, or None for the shadow (fuzz) draw.
    pub colormap: Option<usize>,
    pub mobjflags: i32,
}

pub struct State {
    pub sprites: Vec<SpriteDef>,
    pub numsprites: i32,
    pub negonearray: [i16; SCREENWIDTH],
    pub screenheightarray: [i16; SCREENWIDTH],
    pub clipbot: [i16; SCREENWIDTH],
    pub cliptop: [i16; SCREENWIDTH],
    pub mfloorclip: Clip,
    pub mceilingclip: Clip,
    pub spryscale: Fixed,
    pub sprtopscreen: Fixed,
    pub pspritescale: Fixed,
    pub pspriteiscale: Fixed,
    pub spritelights: [usize; MAXLIGHTSCALE],
    /// The vissprites. The extra last entry is the overflow sprite of the original.
    pub vissprites: Vec<Vissprite>,
    pub vissprite_p: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            sprites: Vec::new(),
            numsprites: 0,
            negonearray: [0; SCREENWIDTH],
            screenheightarray: [0; SCREENWIDTH],
            clipbot: [0; SCREENWIDTH],
            cliptop: [0; SCREENWIDTH],
            mfloorclip: Clip::Null,
            mceilingclip: Clip::Null,
            spryscale: 0,
            sprtopscreen: 0,
            pspritescale: 0,
            pspriteiscale: 0,
            spritelights: [0; MAXLIGHTSCALE],
            vissprites: vec![Vissprite::default(); MAXVISSPRITES + 1],
            vissprite_p: 0,
        }
    }
}

/// The working state of R_InitSpriteDefs.
struct SpriteInit {
    sprtemp: [SpriteFrame; 29],
    maxframe: i32,
    name: &'static str,
}

/// R_InstallSpriteLump: records one sprite lump for a frame and a rotation.
fn install_sprite_lump(
    d: &mut Doom,
    w: &mut SpriteInit,
    lump: i32,
    frame: u32,
    rotation: u32,
    flipped: bool,
) {
    if frame >= 29 || rotation > 8 {
        i_system::error(
            d,
            &format!("R_InstallSpriteLump: Bad frame characters in lump {}", lump),
        );
    }
    if frame as i32 > w.maxframe {
        w.maxframe = frame as i32;
    }
    let first = d.r_data.firstspritelump;
    let letter = (b'A' + frame as u8) as char;
    let f = &mut w.sprtemp[frame as usize];
    if rotation == 0 {
        if f.rotate == 0 {
            i_system::error(
                d,
                &format!(
                    "R_InitSprites: Sprite {} frame {} has multip rot=0 lump",
                    w.name, letter
                ),
            );
        }
        if f.rotate == 1 {
            i_system::error(
                d,
                &format!(
                    "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                    w.name, letter
                ),
            );
        }
        f.rotate = 0;
        for r in 0..8 {
            f.lump[r] = (lump - first) as i16;
            f.flip[r] = flipped as u8;
        }
        return;
    }
    if f.rotate == 0 {
        i_system::error(
            d,
            &format!(
                "R_InitSprites: Sprite {} frame {} has rotations and a rot=0 lump",
                w.name, letter
            ),
        );
    }
    f.rotate = 1;
    let r = (rotation - 1) as usize;
    if f.lump[r] != -1 {
        let msg = format!(
            "R_InitSprites: Sprite {} : {} : {} has two lumps mapped to it",
            w.name,
            letter,
            (b'1' + r as u8) as char
        );
        i_system::error(d, &msg);
    }
    f.lump[r] = (lump - first) as i16;
    f.flip[r] = flipped as u8;
}

/// R_InitSpriteDefs: builds the frame and rotation tables of each sprite name.
fn init_sprite_defs(d: &mut Doom) {
    d.r_things.numsprites = NUMSPRITES as i32;
    d.r_things.sprites = vec![SpriteDef::default(); NUMSPRITES];
    d.z_zone
        .malloc(16 * NUMSPRITES as i32, crate::z_zone::PU_STATIC, 0);
    let start = d.r_data.firstspritelump - 1;
    let end = d.r_data.lastspritelump + 1;
    for (i, &name) in SPRNAMES.iter().enumerate() {
        let mut w = SpriteInit {
            sprtemp: [SpriteFrame::default(); 29],
            maxframe: -1,
            name,
        };
        let key = name.as_bytes();
        for l in start + 1..end {
            let lname = d.w_wad.lumps[l as usize].name;
            if !lname[..4].eq_ignore_ascii_case(&key[..4]) {
                continue;
            }
            let frame = (lname[4] as i8 as i32 - b'A' as i32) as u32;
            let rotation = (lname[5] as i8 as i32 - b'0' as i32) as u32;
            let patched = if d.d_main.modifiedgame {
                w_wad::get_num_for_name(d, &w_wad::lump_name(&d.w_wad, l as usize)) as i32
            } else {
                l
            };
            install_sprite_lump(d, &mut w, patched, frame, rotation, false);
            if lname[6] != 0 {
                let frame = (lname[6] as i8 as i32 - b'A' as i32) as u32;
                let rotation = (lname[7] as i8 as i32 - b'0' as i32) as u32;
                install_sprite_lump(d, &mut w, l, frame, rotation, true);
            }
        }
        if w.maxframe == -1 {
            d.r_things.sprites[i].numframes = 0;
            continue;
        }
        w.maxframe += 1;
        for frame in 0..w.maxframe as usize {
            let letter = (b'A' + frame as u8) as char;
            match w.sprtemp[frame].rotate {
                -1 => i_system::error(
                    d,
                    &format!(
                        "R_InitSprites: No patches found for {} frame {}",
                        name, letter
                    ),
                ),
                1 => {
                    if w.sprtemp[frame].lump.contains(&-1) {
                        i_system::error(
                            d,
                            &format!(
                                "R_InitSprites: Sprite {} frame {} is missing rotations",
                                name, letter
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        d.z_zone
            .malloc(28 * w.maxframe, crate::z_zone::PU_STATIC, 0);
        d.r_things.sprites[i] = SpriteDef {
            numframes: w.maxframe,
            spriteframes: w.sprtemp[..w.maxframe as usize].to_vec(),
        };
    }
}

/// R_InitSprites: fills the -1 clip array and builds the sprite definitions. P_Init calls it.
pub fn init_sprites(d: &mut Doom) {
    d.r_things.negonearray = [-1; SCREENWIDTH];
    init_sprite_defs(d);
}

/// R_ClearSprites.
pub fn clear_sprites(d: &mut Doom) {
    d.r_things.vissprite_p = 0;
}

/// R_NewVisSprite: returns the next vissprite slot, or the overflow slot when all are in use.
fn new_vis_sprite(d: &mut Doom) -> usize {
    if d.r_things.vissprite_p == MAXVISSPRITES {
        return MAXVISSPRITES;
    }
    d.r_things.vissprite_p += 1;
    d.r_things.vissprite_p - 1
}

fn add_ofs(s: ColSrc, n: usize) -> ColSrc {
    match s {
        ColSrc::Lump(l, o) => ColSrc::Lump(l, o.wrapping_add(n)),
        ColSrc::Composite(t, o) => ColSrc::Composite(t, o.wrapping_add(n)),
    }
}

/// R_DrawMaskedColumn: draws the posts of a patch column inside the clip arrays.
pub fn draw_masked_column(d: &mut Doom, column: ColSrc) {
    let basetexturemid = d.r_draw.dc_texturemid;
    let x = d.r_draw.dc_x;
    let fclip = clip_at(d, d.r_things.mfloorclip, x) as i32;
    let cclip = clip_at(d, d.r_things.mceilingclip, x) as i32;
    let mut ofs = 0usize;
    loop {
        let (topdelta, length) = {
            let (b, at) = crate::r_draw::source(&d.z_zone, &d.w_wad, &d.r_data, column);
            (
                b.get(at + ofs).copied().unwrap_or(0xff),
                b.get(at + ofs + 1).copied().unwrap_or(0),
            )
        };
        if topdelta == 0xff {
            break;
        }
        let t = &d.r_things;
        let topscreen = t
            .sprtopscreen
            .wrapping_add(t.spryscale.wrapping_mul(topdelta as i32));
        let bottomscreen = topscreen.wrapping_add(t.spryscale.wrapping_mul(length as i32));
        let mut yl = topscreen.wrapping_add(FRACUNIT - 1) >> FRACBITS;
        let mut yh = bottomscreen.wrapping_sub(1) >> FRACBITS;
        if yh >= fclip {
            yh = fclip - 1;
        }
        if yl <= cclip {
            yl = cclip + 1;
        }
        d.r_draw.dc_yl = yl;
        d.r_draw.dc_yh = yh;
        if yl <= yh {
            d.r_draw.dc_source = add_ofs(column, ofs + 3);
            d.r_draw.dc_texturemid = basetexturemid.wrapping_sub((topdelta as i32) << FRACBITS);
            crate::r_draw::colfunc(d);
        }
        ofs += length as usize + 4;
    }
    d.r_draw.dc_texturemid = basetexturemid;
}

/// R_DrawVisSprite: draws a vissprite column by column.
fn draw_vis_sprite(d: &mut Doom, vis: &Vissprite) {
    let lump = (vis.patch + d.r_data.firstspritelump) as usize;
    d.z_zone.cache_lump(&d.w_wad, lump, crate::z_zone::PU_CACHE);
    let patch_width = {
        let p = w_wad::lump(&d.w_wad, lump);
        i16::from_le_bytes([p[0], p[1]]) as i32
    };
    match vis.colormap {
        None => d.r_draw.colfunc = d.r_draw.fuzzcolfunc,
        Some(cm) => {
            d.r_draw.dc_colormap = cm;
            if vis.mobjflags & MF_TRANSLATION != 0 {
                d.r_draw.colfunc = d.r_draw.transcolfunc;
                d.r_draw.dc_translation =
                    (((vis.mobjflags & MF_TRANSLATION) >> (MF_TRANSSHIFT - 8)) - 256) as usize;
            }
        }
    }
    d.r_draw.dc_iscale = vis.xiscale.wrapping_abs() >> d.r_main.detailshift;
    d.r_draw.dc_texturemid = vis.texturemid;
    let mut frac = vis.startfrac;
    d.r_things.spryscale = vis.scale;
    d.r_things.sprtopscreen = d
        .r_main
        .centeryfrac
        .wrapping_sub(fixed_mul(vis.texturemid, vis.scale));
    for x in vis.x1..=vis.x2 {
        d.r_draw.dc_x = x;
        let texturecolumn = frac >> FRACBITS;
        if texturecolumn < 0 || texturecolumn >= patch_width {
            i_system::error(d, "R_DrawSpriteRange: bad texturecolumn");
        }
        let ofs = {
            let p = w_wad::lump(&d.w_wad, lump);
            let at = 8 + 4 * texturecolumn as usize;
            i32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]]) as usize
        };
        draw_masked_column(d, ColSrc::Lump(lump, ofs));
        frac = frac.wrapping_add(vis.xiscale);
    }
    d.r_draw.colfunc = d.r_draw.basecolfunc;
}

/// Returns the sprite frame of a sprite and frame number, or stops with the original error.
fn sprite_frame(d: &mut Doom, sprite: usize, frame: i32) -> SpriteFrame {
    if sprite as u32 >= d.r_things.numsprites as u32 {
        i_system::error(
            d,
            &format!("R_ProjectSprite: invalid sprite number {} ", sprite),
        );
    }
    if (frame & FF_FRAMEMASK) >= d.r_things.sprites[sprite].numframes {
        i_system::error(
            d,
            &format!(
                "R_ProjectSprite: invalid sprite frame {} : {} ",
                sprite, frame
            ),
        );
    }
    d.r_things.sprites[sprite].spriteframes[(frame & FF_FRAMEMASK) as usize]
}

/// R_ProjectSprite: transforms a thing into a vissprite when it can be visible.
fn project_sprite(d: &mut Doom, thing: MobjId) {
    let (tx0, ty0, tz0, tangle, tsprite, tframe, tflags) = {
        let t = d.p_tick.mobj(thing);
        (t.x, t.y, t.z, t.angle, t.sprite, t.frame, t.flags)
    };
    let m = &d.r_main;
    let tr_x = tx0.wrapping_sub(m.viewx);
    let tr_y = ty0.wrapping_sub(m.viewy);
    let gxt = fixed_mul(tr_x, m.viewcos);
    let gyt = fixed_mul(tr_y, m.viewsin).wrapping_neg();
    let tz = gxt.wrapping_sub(gyt);
    if tz < MINZ {
        return;
    }
    let xscale = fixed_div(m.projection, tz);
    let gxt = fixed_mul(tr_x, m.viewsin).wrapping_neg();
    let gyt = fixed_mul(tr_y, m.viewcos);
    let mut tx = gyt.wrapping_add(gxt).wrapping_neg();
    if tx.wrapping_abs() > tz << 2 {
        return;
    }
    let sprframe = sprite_frame(d, tsprite, tframe);
    let (lump, flip) = if sprframe.rotate != 0 {
        let ang: Angle = crate::r_main::point_to_angle(d, tx0, ty0);
        let rot = (ang
            .wrapping_sub(tangle)
            .wrapping_add((ANG45 / 2).wrapping_mul(9))
            >> 29) as usize;
        (sprframe.lump[rot] as i32, sprframe.flip[rot] != 0)
    } else {
        (sprframe.lump[0] as i32, sprframe.flip[0] != 0)
    };
    let lu = lump as usize;
    let m = &d.r_main;
    let viewwidth = d.r_draw.viewwidth;
    tx = tx.wrapping_sub(d.r_data.spriteoffset[lu]);
    let x1 = m.centerxfrac.wrapping_add(fixed_mul(tx, xscale)) >> FRACBITS;
    if x1 > viewwidth {
        return;
    }
    tx = tx.wrapping_add(d.r_data.spritewidth[lu]);
    let x2 = (m.centerxfrac.wrapping_add(fixed_mul(tx, xscale)) >> FRACBITS) - 1;
    if x2 < 0 {
        return;
    }
    let detailshift = m.detailshift;
    let viewz = m.viewz;
    let fixedcolormap = m.fixedcolormap;
    let mut vis = Vissprite {
        mobjflags: tflags,
        scale: xscale << detailshift,
        gx: tx0,
        gy: ty0,
        gz: tz0,
        gzt: tz0.wrapping_add(d.r_data.spritetopoffset[lu]),
        ..Default::default()
    };
    vis.texturemid = vis.gzt.wrapping_sub(viewz);
    vis.x1 = x1.max(0);
    vis.x2 = if x2 >= viewwidth { viewwidth - 1 } else { x2 };
    let iscale = fixed_div(FRACUNIT, xscale);
    if flip {
        vis.startfrac = d.r_data.spritewidth[lu] - 1;
        vis.xiscale = iscale.wrapping_neg();
    } else {
        vis.startfrac = 0;
        vis.xiscale = iscale;
    }
    if vis.x1 > x1 {
        vis.startfrac = vis
            .startfrac
            .wrapping_add(vis.xiscale.wrapping_mul(vis.x1 - x1));
    }
    vis.patch = lump;
    vis.colormap = if tflags & MF_SHADOW != 0 {
        None
    } else if let Some(cm) = fixedcolormap {
        Some(cm)
    } else if tframe & FF_FULLBRIGHT != 0 {
        Some(0)
    } else {
        let index = ((xscale >> (LIGHTSCALESHIFT - detailshift)) as usize).min(MAXLIGHTSCALE - 1);
        Some(d.r_things.spritelights[index])
    };
    let slot = new_vis_sprite(d);
    d.r_things.vissprites[slot] = vis;
}

fn sprite_lights(d: &Doom, lightlevel: i32) -> [usize; MAXLIGHTSCALE] {
    let lightnum = (lightlevel >> LIGHTSEGSHIFT) + d.r_main.extralight;
    d.r_main.scalelight[lightnum.clamp(0, LIGHTLEVELS as i32 - 1) as usize]
}

/// R_AddSprites: projects the things of a sector once per frame.
pub fn add_sprites(d: &mut Doom, sec: usize) {
    if d.p_setup.sectors[sec].validcount == d.r_main.validcount {
        return;
    }
    d.p_setup.sectors[sec].validcount = d.r_main.validcount;
    d.r_things.spritelights = sprite_lights(d, d.p_setup.sectors[sec].lightlevel);
    let mut thing = d.p_setup.sectors[sec].thinglist;
    while let Some(t) = thing {
        project_sprite(d, t);
        thing = d.p_tick.mobj(t).snext;
    }
}

/// R_DrawPSprite: draws one player weapon sprite.
fn draw_psprite(d: &mut Doom, psp: &PspDef) {
    let st = &STATES[psp.state.unwrap_or(0)];
    let sprframe = sprite_frame(d, st.sprite, st.frame);
    let lump = sprframe.lump[0] as i32;
    let flip = sprframe.flip[0] != 0;
    let lu = lump as usize;
    let m = &d.r_main;
    let t = &d.r_things;
    let viewwidth = d.r_draw.viewwidth;
    let mut tx = psp.sx.wrapping_sub((SCREENWIDTH as i32 / 2) * FRACUNIT);
    tx = tx.wrapping_sub(d.r_data.spriteoffset[lu]);
    let x1 = m.centerxfrac.wrapping_add(fixed_mul(tx, t.pspritescale)) >> FRACBITS;
    if x1 > viewwidth {
        return;
    }
    tx = tx.wrapping_add(d.r_data.spritewidth[lu]);
    let x2 = (m.centerxfrac.wrapping_add(fixed_mul(tx, t.pspritescale)) >> FRACBITS) - 1;
    if x2 < 0 {
        return;
    }
    let mut vis = Vissprite {
        mobjflags: 0,
        texturemid: (BASEYCENTER << FRACBITS) + FRACUNIT / 2
            - psp.sy.wrapping_sub(d.r_data.spritetopoffset[lu]),
        x1: x1.max(0),
        x2: if x2 >= viewwidth { viewwidth - 1 } else { x2 },
        scale: t.pspritescale << m.detailshift,
        ..Default::default()
    };
    if flip {
        vis.xiscale = t.pspriteiscale.wrapping_neg();
        vis.startfrac = d.r_data.spritewidth[lu] - 1;
    } else {
        vis.xiscale = t.pspriteiscale;
        vis.startfrac = 0;
    }
    if vis.x1 > x1 {
        vis.startfrac = vis
            .startfrac
            .wrapping_add(vis.xiscale.wrapping_mul(vis.x1 - x1));
    }
    vis.patch = lump;
    let invis = d.g_game.players[m.viewplayer].powers[PW_INVISIBILITY];
    vis.colormap = if invis > 4 * 32 || invis & 8 != 0 {
        None
    } else if let Some(cm) = m.fixedcolormap {
        Some(cm)
    } else if st.frame & FF_FULLBRIGHT != 0 {
        Some(0)
    } else {
        Some(t.spritelights[MAXLIGHTSCALE - 1])
    };
    draw_vis_sprite(d, &vis);
}

/// R_DrawPlayerSprites: draws the weapon and the muzzle flash of the view player.
fn draw_player_sprites(d: &mut Doom) {
    let p = d.r_main.viewplayer;
    let mo = d.g_game.players[p].mo.expect("the view player has no body");
    let ss = d.p_tick.mobj(mo).subsector;
    let sec = d.p_setup.subsectors[ss].sector;
    d.r_things.spritelights = sprite_lights(d, d.p_setup.sectors[sec].lightlevel);
    d.r_things.mfloorclip = Clip::ScreenHeight;
    d.r_things.mceilingclip = Clip::NegOne;
    for i in 0..NUMPSPRITES {
        let psp = d.g_game.players[p].psprites[i];
        if psp.state.is_some() {
            draw_psprite(d, &psp);
        }
    }
}

/// R_SortVisSprites: returns the vissprite order by increasing scale. Equal scales keep their order.
fn sort_vis_sprites(d: &Doom) -> Vec<usize> {
    let n = d.r_things.vissprite_p;
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| d.r_things.vissprites[i].scale);
    order
}

/// R_DrawSprite: clips a vissprite against the drawsegs in front of it and draws it.
fn draw_sprite(d: &mut Doom, spr: &Vissprite) {
    for x in spr.x1..=spr.x2 {
        d.r_things.clipbot[x as usize] = -2;
        d.r_things.cliptop[x as usize] = -2;
    }
    let mut dsi = d.r_bsp.ds_p;
    while dsi > 0 {
        dsi -= 1;
        let ds = d.r_bsp.drawsegs[dsi];
        if ds.x1 > spr.x2 || ds.x2 < spr.x1 || (ds.silhouette == 0 && ds.maskedtexturecol.is_none())
        {
            continue;
        }
        let r1 = if ds.x1 < spr.x1 { spr.x1 } else { ds.x1 };
        let r2 = if ds.x2 > spr.x2 { spr.x2 } else { ds.x2 };
        let (lowscale, scale) = if ds.scale1 > ds.scale2 {
            (ds.scale2, ds.scale1)
        } else {
            (ds.scale1, ds.scale2)
        };
        if scale < spr.scale
            || (lowscale < spr.scale
                && crate::r_main::point_on_seg_side(d, spr.gx, spr.gy, ds.curline) == 0)
        {
            if ds.maskedtexturecol.is_some() {
                crate::r_segs::render_masked_seg_range(d, dsi, r1, r2);
            }
            continue;
        }
        let mut silhouette = ds.silhouette;
        if spr.gz >= ds.bsilheight {
            silhouette &= !SIL_BOTTOM;
        }
        if spr.gzt <= ds.tsilheight {
            silhouette &= !SIL_TOP;
        }
        for x in r1..=r2 {
            let xu = x as usize;
            if silhouette & SIL_BOTTOM != 0 && d.r_things.clipbot[xu] == -2 {
                d.r_things.clipbot[xu] = clip_at(d, ds.sprbottomclip, x);
            }
            if silhouette & SIL_TOP != 0 && d.r_things.cliptop[xu] == -2 {
                d.r_things.cliptop[xu] = clip_at(d, ds.sprtopclip, x);
            }
        }
    }
    let viewheight = d.r_draw.viewheight as i16;
    for x in spr.x1..=spr.x2 {
        let xu = x as usize;
        if d.r_things.clipbot[xu] == -2 {
            d.r_things.clipbot[xu] = viewheight;
        }
        if d.r_things.cliptop[xu] == -2 {
            d.r_things.cliptop[xu] = -1;
        }
    }
    d.r_things.mfloorclip = Clip::SprBottom;
    d.r_things.mceilingclip = Clip::SprTop;
    draw_vis_sprite(d, spr);
}

/// R_DrawMasked: draws the sprites back to front, the remaining masked textures, and the weapon.
pub fn draw_masked(d: &mut Doom) {
    let order = sort_vis_sprites(d);
    for i in order {
        let spr = d.r_things.vissprites[i];
        draw_sprite(d, &spr);
    }
    let mut dsi = d.r_bsp.ds_p;
    while dsi > 0 {
        dsi -= 1;
        let ds = d.r_bsp.drawsegs[dsi];
        if ds.maskedtexturecol.is_some() {
            crate::r_segs::render_masked_seg_range(d, dsi, ds.x1, ds.x2);
        }
    }
    if d.r_main.viewangleoffset == 0 {
        draw_player_sprites(d);
    }
}

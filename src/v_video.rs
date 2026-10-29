//! Screen drawing: patches, blocks, rectangle copies, and the dirty box.
//! The drawing target is `d.i_video.video_buffer`. A caller swaps another buffer in to draw there.

use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::Doom;

const W: i32 = SCREENWIDTH as i32;
const H: i32 = SCREENHEIGHT as i32;

#[derive(Default)]
pub struct State {
    /// The dirty box: top, bottom, left, right (M_AddToBox order).
    pub dirtybox: [i32; 4],
}

fn rd16(b: &[u8], at: usize) -> i32 {
    i16::from_le_bytes([b[at], b[at + 1]]) as i32
}

/// Returns width, height, leftoffset, and topoffset of a patch lump.
fn header(p: &[u8]) -> (i32, i32, i32, i32) {
    (rd16(p, 0), rd16(p, 2), rd16(p, 4), rd16(p, 6))
}

/// Draws the posts of one patch column at screen offset `desttop`.
fn draw_column(screen: &mut [u8], p: &[u8], col: i32, desttop: usize) {
    let at = 8 + 4 * col as usize;
    let mut c = i32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]]) as usize;
    while p[c] != 0xff {
        let topdelta = p[c] as usize;
        let length = p[c + 1] as usize;
        let mut dest = desttop + topdelta * SCREENWIDTH;
        for &b in &p[c + 3..c + 3 + length] {
            screen[dest] = b;
            dest += SCREENWIDTH;
        }
        c += length + 4;
    }
}

fn draw_patch_common(d: &mut Doom, x: i32, y: i32, patch: usize, flipped: bool) {
    d.z_zone.touch_lump(&d.w_wad, patch);
    let p = crate::w_wad::lump(&d.w_wad, patch);
    let (w, h, left, top) = header(p);
    let (x, y) = (x - left, y - top);
    if x < 0 || x + w > W || y < 0 || y + h > H {
        let msg = if flipped {
            "Bad V_DrawPatchFlipped".to_string()
        } else {
            format!("Bad V_DrawPatch x={x} y={y} patch.width={w} patch.height={h} topoffset={top} leftoffset={left}")
        };
        crate::i_system::error(d, &msg);
    }
    mark_rect(d, x, y, w, h);
    let p = crate::w_wad::lump(&d.w_wad, patch);
    let screen = &mut d.i_video.video_buffer;
    let desttop = (y * W + x) as usize;
    for col in 0..w {
        let src = if flipped { w - 1 - col } else { col };
        draw_column(screen, p, src, desttop + col as usize);
    }
}

/// V_DrawPatch.
pub fn draw_patch(d: &mut Doom, x: i32, y: i32, patch: usize) {
    draw_patch_common(d, x, y, patch, false);
}

/// V_DrawPatchFlipped.
pub fn draw_patch_flipped(d: &mut Doom, x: i32, y: i32, patch: usize) {
    draw_patch_common(d, x, y, patch, true);
}

/// V_DrawPatchDirect. The original calls V_DrawPatch.
pub fn draw_patch_direct(d: &mut Doom, x: i32, y: i32, patch: usize) {
    draw_patch(d, x, y, patch);
}

/// V_DrawBlock. The original has no range check here.
pub fn draw_block(d: &mut Doom, x: i32, y: i32, width: i32, height: i32, src: &[u8]) {
    mark_rect(d, x, y, width, height);
    let (w, mut dest) = (width as usize, (y * W + x) as usize);
    for row in src.chunks(w).take(height as usize) {
        d.i_video.video_buffer[dest..dest + w].copy_from_slice(row);
        dest += SCREENWIDTH;
    }
}

/// V_CopyRect: copies a rectangle of a 320-wide source screen to the drawing target.
#[allow(clippy::too_many_arguments)]
pub fn copy_rect(
    d: &mut Doom,
    srcx: i32,
    srcy: i32,
    source: &[u8],
    width: i32,
    height: i32,
    destx: i32,
    desty: i32,
) {
    if srcx < 0
        || srcx + width > W
        || srcy < 0
        || srcy + height > H
        || destx < 0
        || destx + width > W
        || desty < 0
        || desty + height > H
    {
        crate::i_system::error(d, "Bad V_CopyRect");
    }
    mark_rect(d, destx, desty, width, height);
    let w = width as usize;
    for row in 0..height {
        let s = ((srcy + row) * W + srcx) as usize;
        let t = ((desty + row) * W + destx) as usize;
        d.i_video.video_buffer[t..t + w].copy_from_slice(&source[s..s + w]);
    }
}

/// V_MarkRect: adds the rectangle to the dirty box.
pub fn mark_rect(d: &mut Doom, x: i32, y: i32, width: i32, height: i32) {
    let b = &mut d.v_video.dirtybox;
    for (px, py) in [(x, y), (x + width - 1, y + height - 1)] {
        if px < b[2] {
            b[2] = px;
        } else if px > b[3] {
            b[3] = px;
        }
        if py < b[1] {
            b[1] = py;
        } else if py > b[0] {
            b[0] = py;
        }
    }
}

/// V_DrawRawScreen.
pub fn draw_raw_screen(d: &mut Doom, raw: &[u8]) {
    d.i_video.video_buffer[..SCREENWIDTH * SCREENHEIGHT]
        .copy_from_slice(&raw[..SCREENWIDTH * SCREENHEIGHT]);
}

/// V_ScreenShot: writes the screen as the first free DOOMnn.pcx file.
pub fn screen_shot(d: &mut Doom, prefix: &str) {
    let Some(name) = (0..=99)
        .map(|i| format!("{prefix}{i:02}.pcx"))
        .find(|n| !crate::m_misc::file_exists(n))
    else {
        crate::i_system::error(d, "V_ScreenShot: Couldn't create a PCX");
    };
    let lump = crate::w_wad::get_num_for_name(d, "PLAYPAL");
    let palette = &crate::w_wad::lump(&d.w_wad, lump)[..768];
    // WritePCXfile: a 128-byte header, the packed pixels, the marker 0x0c, and the palette.
    let mut pcx = vec![0x0a, 5, 1, 8, 0, 0, 0, 0];
    for v in [W - 1, H - 1, W, H] {
        pcx.extend_from_slice(&(v as u16).to_le_bytes());
    }
    pcx.extend_from_slice(&[0; 49]);
    pcx.push(1);
    pcx.extend_from_slice(&(W as u16).to_le_bytes());
    pcx.extend_from_slice(&2u16.to_le_bytes());
    pcx.extend_from_slice(&[0; 58]);
    for &b in &d.i_video.video_buffer {
        if b & 0xc0 == 0xc0 {
            pcx.push(0xc1);
        }
        pcx.push(b);
    }
    pcx.push(0x0c);
    pcx.extend_from_slice(palette);
    crate::m_misc::write_file(&name, &pcx);
}

/// V_DrawMouseSpeedBox: takes the six widget colors. The mouse is off, so no box appears.
pub fn draw_mouse_speed_box(_d: &mut Doom, _speed: i32) {
    for _ in 0..6 {
        // I_GetPaletteIndex of this build only reports the call.
        println!("I_GetPaletteIndex");
    }
}

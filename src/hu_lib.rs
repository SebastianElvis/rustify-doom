//! Text line, scrolling text, and input text widgets of the heads-up display.

use crate::i_video::{KEY_BACKSPACE, KEY_ENTER, SCREENWIDTH};
use crate::Doom;

pub const HU_MAXLINES: usize = 4;
pub const HU_MAXLINELENGTH: usize = 80;

#[derive(Default)]
pub struct State {}

/// Reads the width, height, left offset, and top offset of a patch lump.
pub fn patch_header(d: &Doom, lump: usize) -> (i32, i32, i32, i32) {
    let p = crate::w_wad::lump(&d.w_wad, lump);
    let s = |i: usize| i16::from_le_bytes([p[i], p[i + 1]]) as i32;
    (s(0), s(2), s(4), s(6))
}

pub fn patch_width(d: &Doom, lump: usize) -> i32 {
    patch_header(d, lump).0
}

pub fn patch_height(d: &Doom, lump: usize) -> i32 {
    patch_header(d, lump).1
}

/// hu_textline_t. The font holds patch lump numbers.
#[derive(Clone, Default)]
pub struct TextLine {
    pub x: i32,
    pub y: i32,
    pub f: Vec<usize>,
    pub sc: i32,
    pub l: Vec<u8>,
    pub needsupdate: i32,
}

/// hu_stext_t. The `on` flag stays with the caller.
#[derive(Clone, Default)]
pub struct SText {
    pub l: [TextLine; HU_MAXLINES],
    pub h: usize,
    pub cl: usize,
    pub laston: bool,
}

/// hu_itext_t. The `on` flag stays with the caller.
#[derive(Clone, Default)]
pub struct IText {
    pub l: TextLine,
    pub lm: usize,
    pub laston: bool,
}

pub fn clear_text_line(t: &mut TextLine) {
    t.l.clear();
    t.needsupdate = 1;
}

pub fn init_text_line(t: &mut TextLine, x: i32, y: i32, f: &[usize], sc: i32) {
    t.x = x;
    t.y = y;
    t.f = f.to_vec();
    t.sc = sc;
    clear_text_line(t);
}

pub fn add_char_to_text_line(t: &mut TextLine, ch: u8) -> bool {
    if t.l.len() == HU_MAXLINELENGTH {
        false
    } else {
        t.l.push(ch);
        t.needsupdate = 4;
        true
    }
}

pub fn del_char_from_text_line(t: &mut TextLine) -> bool {
    if t.l.is_empty() {
        false
    } else {
        t.l.pop();
        t.needsupdate = 4;
        true
    }
}

pub fn draw_text_line(d: &mut Doom, l: &TextLine, drawcursor: bool) {
    let mut x = l.x;
    for &ch in &l.l {
        let c = ch.to_ascii_uppercase() as i32;
        if c != b' ' as i32 && c >= l.sc && c <= b'_' as i32 {
            let p = l.f[(c - l.sc) as usize];
            let w = patch_width(d, p);
            if x + w > SCREENWIDTH as i32 {
                break;
            }
            crate::v_video::draw_patch_direct(d, x, l.y, p);
            x += w;
        } else {
            x += 4;
            if x >= SCREENWIDTH as i32 {
                break;
            }
        }
    }
    if drawcursor {
        let p = l.f[(b'_' as i32 - l.sc) as usize];
        if x + patch_width(d, p) <= SCREENWIDTH as i32 {
            crate::v_video::draw_patch_direct(d, x, l.y, p);
        }
    }
}

/// HUlib_eraseTextLine. It erases only a reduced view outside the automap.
pub fn erase_text_line(d: &mut Doom, l: &mut TextLine) {
    let (wx, wy, vw, vh) = crate::r_main::view_window(d);
    if !d.am_map.automapactive && wx != 0 && l.needsupdate != 0 {
        let lh = patch_height(d, l.f[0]) + 1;
        let sw = SCREENWIDTH as i32;
        let mut y = l.y;
        let mut yoffset = y * sw;
        while y < l.y + lh {
            if y < wy || y >= wy + vh {
                crate::r_draw::video_erase(d, yoffset as usize, sw as usize);
            } else {
                crate::r_draw::video_erase(d, yoffset as usize, wx as usize);
                crate::r_draw::video_erase(d, (yoffset + wx + vw) as usize, wx as usize);
            }
            y += 1;
            yoffset += sw;
        }
    }
    if l.needsupdate != 0 {
        l.needsupdate -= 1;
    }
}

pub fn init_s_text(
    d: &Doom,
    s: &mut SText,
    x: i32,
    y: i32,
    h: usize,
    font: &[usize],
    startchar: i32,
) {
    s.h = h;
    s.laston = true;
    s.cl = 0;
    let fh = patch_height(d, font[0]);
    for i in 0..h {
        init_text_line(&mut s.l[i], x, y - i as i32 * (fh + 1), font, startchar);
    }
}

pub fn add_line_to_s_text(s: &mut SText) {
    s.cl += 1;
    if s.cl == s.h {
        s.cl = 0;
    }
    clear_text_line(&mut s.l[s.cl]);
    for i in 0..s.h {
        s.l[i].needsupdate = 4;
    }
}

pub fn add_message_to_s_text(s: &mut SText, prefix: Option<&str>, msg: &str) {
    add_line_to_s_text(s);
    let cl = s.cl;
    if let Some(p) = prefix {
        for &c in p.as_bytes() {
            add_char_to_text_line(&mut s.l[cl], c);
        }
    }
    for &c in msg.as_bytes() {
        add_char_to_text_line(&mut s.l[cl], c);
    }
}

pub fn draw_s_text(d: &mut Doom, s: &SText, on: bool) {
    if !on {
        return;
    }
    for i in 0..s.h {
        let mut idx = s.cl as i32 - i as i32;
        if idx < 0 {
            idx += s.h as i32;
        }
        draw_text_line(d, &s.l[idx as usize], false);
    }
}

pub fn erase_s_text(d: &mut Doom, s: &mut SText, on: bool) {
    for i in 0..s.h {
        if s.laston && !on {
            s.l[i].needsupdate = 4;
        }
        erase_text_line(d, &mut s.l[i]);
    }
    s.laston = on;
}

pub fn init_i_text(it: &mut IText, x: i32, y: i32, font: &[usize], startchar: i32) {
    it.lm = 0;
    it.laston = true;
    init_text_line(&mut it.l, x, y, font, startchar);
}

pub fn del_char_from_i_text(it: &mut IText) {
    if it.l.l.len() != it.lm {
        del_char_from_text_line(&mut it.l);
    }
}

pub fn erase_line_from_i_text(it: &mut IText) {
    while it.lm != it.l.l.len() {
        del_char_from_text_line(&mut it.l);
    }
}

pub fn reset_i_text(it: &mut IText) {
    it.lm = 0;
    clear_text_line(&mut it.l);
}

pub fn add_prefix_to_i_text(it: &mut IText, s: &str) {
    for &c in s.as_bytes() {
        add_char_to_text_line(&mut it.l, c);
    }
    it.lm = it.l.l.len();
}

/// HUlib_keyInIText. It returns true when the widget eats the key.
pub fn key_in_i_text(it: &mut IText, ch: u8) -> bool {
    let ch = ch.to_ascii_uppercase();
    if (b' '..=b'_').contains(&ch) {
        add_char_to_text_line(&mut it.l, ch);
    } else if ch as i32 == KEY_BACKSPACE {
        del_char_from_i_text(it);
    } else if ch as i32 != KEY_ENTER {
        return false;
    }
    true
}

pub fn draw_i_text(d: &mut Doom, it: &IText, on: bool) {
    if !on {
        return;
    }
    draw_text_line(d, &it.l, true);
}

pub fn erase_i_text(d: &mut Doom, it: &mut IText, on: bool) {
    if it.laston && !on {
        it.l.needsupdate = 4;
    }
    erase_text_line(d, &mut it.l);
    it.laston = on;
}

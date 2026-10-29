//! The SDL platform layer: window, frame presentation, key queue, clock, and palette conversion.

use std::ffi::{c_char, c_int, c_void, CString};

pub const SCREENWIDTH: usize = 320;
pub const SCREENHEIGHT: usize = 200;
pub const RESX: usize = 640;
pub const RESY: usize = 400;

// Doom key codes after the platform translation.
pub const KEY_RIGHTARROW: i32 = 0xae;
pub const KEY_LEFTARROW: i32 = 0xac;
pub const KEY_UPARROW: i32 = 0xad;
pub const KEY_DOWNARROW: i32 = 0xaf;
pub const KEY_STRAFE_L: i32 = 0xa0;
pub const KEY_STRAFE_R: i32 = 0xa1;
pub const KEY_USE: i32 = 0xa2;
pub const KEY_FIRE: i32 = 0xa3;
pub const KEY_ESCAPE: i32 = 27;
pub const KEY_ENTER: i32 = 13;
pub const KEY_TAB: i32 = 9;
pub const KEY_F1: i32 = 0x80 + 0x3b;
pub const KEY_F2: i32 = 0x80 + 0x3c;
pub const KEY_F3: i32 = 0x80 + 0x3d;
pub const KEY_F4: i32 = 0x80 + 0x3e;
pub const KEY_F5: i32 = 0x80 + 0x3f;
pub const KEY_F6: i32 = 0x80 + 0x40;
pub const KEY_F7: i32 = 0x80 + 0x41;
pub const KEY_F8: i32 = 0x80 + 0x42;
pub const KEY_F9: i32 = 0x80 + 0x43;
pub const KEY_F10: i32 = 0x80 + 0x44;
pub const KEY_F11: i32 = 0x80 + 0x57;
pub const KEY_F12: i32 = 0x80 + 0x58;
pub const KEY_BACKSPACE: i32 = 0x7f;
pub const KEY_PAUSE: i32 = 0xff;
pub const KEY_EQUALS: i32 = 0x3d;
pub const KEY_MINUS: i32 = 0x2d;
pub const KEY_RSHIFT: i32 = 0x80 + 0x36;
pub const KEY_RCTRL: i32 = 0x80 + 0x1d;
pub const KEY_RALT: i32 = 0x80 + 0x38;

#[repr(C)]
struct SdlEvent {
    kind: u32,
    timestamp: u32,
    window_id: u32,
    state: u8,
    repeat: u8,
    padding: [u8; 2],
    scancode: i32,
    sym: i32,
    rest: [u8; 36],
}

extern "C" {
    static __stdoutp: *mut c_void;
    fn fwrite(ptr: *const c_void, size: usize, n: usize, stream: *mut c_void) -> usize;
    fn raise(sig: c_int) -> c_int;
    fn signal(sig: c_int, handler: usize) -> usize;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut c_void;
    fn SDL_CreateRenderer(window: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
    fn SDL_CreateTexture(
        renderer: *mut c_void,
        format: u32,
        access: c_int,
        w: c_int,
        h: c_int,
    ) -> *mut c_void;
    fn SDL_UpdateTexture(
        texture: *mut c_void,
        rect: *const c_void,
        pixels: *const c_void,
        pitch: c_int,
    ) -> c_int;
    fn SDL_RenderClear(renderer: *mut c_void) -> c_int;
    fn SDL_RenderCopy(
        renderer: *mut c_void,
        texture: *mut c_void,
        src: *const c_void,
        dst: *const c_void,
    ) -> c_int;
    fn SDL_RenderPresent(renderer: *mut c_void);
    fn SDL_PollEvent(event: *mut SdlEvent) -> c_int;
    fn SDL_GetTicks() -> u32;
    fn SDL_Delay(ms: u32);
    fn SDL_SetWindowTitle(window: *mut c_void, title: *const c_char);
}

pub struct State {
    window: *mut c_void,
    renderer: *mut c_void,
    texture: *mut c_void,
    /// The 640x400 32-bit frame buffer that the SDL texture receives.
    pub screen_buffer: Vec<u8>,
    /// The -gfxmode bits per pixel and the -scaling factor.
    pub bpp: usize,
    pub scaling: i32,
    /// The 320x200 palette-index screen (screens[0]).
    pub video_buffer: Vec<u8>,
    pub colors: [u32; 256],
    pub usegamma: i32,
    key_queue: [u16; 16],
    key_write: usize,
    key_read: usize,
    shiftdown: i32,
}

impl Default for State {
    fn default() -> Self {
        State {
            window: std::ptr::null_mut(),
            renderer: std::ptr::null_mut(),
            texture: std::ptr::null_mut(),
            screen_buffer: vec![0; RESX * RESY * 4],
            bpp: 32,
            scaling: 2,
            video_buffer: vec![0; SCREENWIDTH * SCREENHEIGHT],
            colors: [0; 256],
            usegamma: 0,
            key_queue: [0; 16],
            key_write: 0,
            key_read: 0,
            shiftdown: 0,
        }
    }
}

/// Creates the window, the renderer, and the streaming texture in the original order.
pub fn dg_init(s: &mut State) {
    let title = CString::new("DOOM").unwrap();
    // SAFETY: SDL receives valid C strings and its own handles. SDL owns the returned objects.
    unsafe {
        s.window = SDL_CreateWindow(
            title.as_ptr(),
            0x1fff_0000,
            0x1fff_0000,
            RESX as c_int,
            RESY as c_int,
            4,
        );
        s.renderer = SDL_CreateRenderer(s.window, -1, 2);
        SDL_RenderClear(s.renderer);
        SDL_RenderPresent(s.renderer);
        s.texture = SDL_CreateTexture(s.renderer, 0x1616_1804, 2, RESX as c_int, RESY as c_int);
    }
}

/// Translates an SDL key symbol into a Doom key code with the original table.
fn convert_key(sym: i32) -> i32 {
    match sym as u32 {
        0x4000_003b..=0x4000_0043 => 188 + (sym as u32 - 0x4000_003b) as i32,
        0x4000_0044 => 215,
        0x4000_004f => KEY_RIGHTARROW,
        0x4000_0050 => KEY_LEFTARROW,
        0x4000_0051 => KEY_DOWNARROW,
        0x4000_0052 => KEY_UPARROW,
        0x4000_00e0 | 0x4000_00e4 => KEY_FIRE,
        0x4000_00e1 | 0x4000_00e5 => KEY_RSHIFT,
        0x4000_00e2 | 0x4000_00e6 => KEY_RALT,
        13 | 27 | 45 => sym,
        43 | 61 => KEY_EQUALS,
        32 => KEY_USE,
        // The original calls tolower() and keeps the low byte. F1 and F12 become ':' and 'E' this way.
        other => {
            let c = other as i32;
            let lowered = if (b'A' as i32..=b'Z' as i32).contains(&c) {
                c + 32
            } else {
                c
            };
            lowered & 0xff
        }
    }
}

fn handle_key_input(s: &mut State) {
    let mut e = SdlEvent {
        kind: 0,
        timestamp: 0,
        window_id: 0,
        state: 0,
        repeat: 0,
        padding: [0; 2],
        scancode: 0,
        sym: 0,
        rest: [0; 36],
    };
    // SAFETY: the event buffer is larger than SDL_Event (56 bytes) and lives for the call.
    while unsafe { SDL_PollEvent(&mut e) } != 0 {
        let pressed: u16 = match e.kind {
            0x300 => 1,
            0x301 => 0,
            0x100 => {
                println!("Quit requested");
                std::process::exit(1);
            }
            _ => continue,
        };
        let key = convert_key(e.sym) as u16 & 0xff;
        s.key_queue[s.key_write] = (pressed << 8) | key;
        s.key_write = (s.key_write + 1) & 15;
    }
}

/// Presents the frame and then drains SDL events into the key queue, as the original does.
pub fn dg_draw_frame(s: &mut State) {
    // SAFETY: the frame buffer has RESX * RESY 32-bit pixels and the pitch matches its rows.
    unsafe {
        SDL_UpdateTexture(
            s.texture,
            std::ptr::null(),
            s.screen_buffer.as_ptr() as *const c_void,
            (RESX * 4) as c_int,
        );
        SDL_RenderClear(s.renderer);
        SDL_RenderCopy(s.renderer, s.texture, std::ptr::null(), std::ptr::null());
        SDL_RenderPresent(s.renderer);
    }
    handle_key_input(s);
}

/// Returns the next queued key as (pressed, key).
pub fn dg_get_key(s: &mut State) -> Option<(bool, i32)> {
    if s.key_read == s.key_write {
        return None;
    }
    let v = s.key_queue[s.key_read];
    s.key_read = (s.key_read + 1) & 15;
    Some((v >> 8 != 0, (v & 0xff) as i32))
}

pub fn dg_get_ticks_ms() -> u32 {
    // SAFETY: SDL_GetTicks has no preconditions.
    unsafe { SDL_GetTicks() }
}

pub fn dg_sleep_ms(ms: u32) {
    // SAFETY: SDL_Delay has no preconditions.
    unsafe { SDL_Delay(ms) }
}

pub fn dg_set_window_title(s: &State, title: &str) {
    let t = CString::new(title).unwrap_or_default();
    // SAFETY: the window handle comes from SDL_CreateWindow and the title is a valid C string.
    unsafe { SDL_SetWindowTitle(s.window, t.as_ptr()) }
}

/// Converts a 768-byte PLAYPAL palette through the current gamma table.
pub fn set_palette(s: &mut State, palette: &[u8]) {
    let gamma = &crate::tables::GAMMATABLE[s.usegamma as usize];
    for i in 0..256 {
        let r = gamma[palette[3 * i] as usize] as u32;
        let g = gamma[palette[3 * i + 1] as usize] as u32;
        let b = gamma[palette[3 * i + 2] as usize] as u32;
        s.colors[i] = (r << 16) | (g << 8) | b;
    }
}

/// Scales the 320x200 screen into the frame buffer in the -gfxmode layout and presents it.
/// A scale below 1 writes nothing. The 16 bpp rows fill only the top half of the 32-bit texture.
pub fn finish_update(s: &mut State) {
    let (f, n) = (s.scaling.max(0) as usize, s.bpp / 8);
    let pad = (RESX - SCREENWIDTH * f) * s.bpp;
    let mut o = 0;
    for row in s.video_buffer.chunks(SCREENWIDTH) {
        for _ in 0..f {
            o += pad / 16;
            for &p in row {
                let c = s.colors[p as usize];
                let c = if n == 2 {
                    (c >> 8) & 0xf800 | (c >> 5) & 0x7e0 | (c >> 3) & 0x1f
                } else {
                    c
                };
                for _ in 0..f {
                    s.screen_buffer[o..o + n].copy_from_slice(&c.to_le_bytes()[..n]);
                    o += n;
                }
            }
            o += pad / 8 - pad / 16;
        }
    }
    dg_draw_frame(s);
}

/// I_InitGraphics: checks -gfxmode and -scaling and prints the framebuffer report.
pub fn init_graphics(d: &mut crate::Doom) {
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-gfxmode", 1);
    let mode = if p > 0 {
        d.m_argv.myargv[p + 1].clone()
    } else {
        "rgba8888".to_string()
    };
    // bpp, then the red, green, blue, and alpha lengths and offsets.
    let (bpp, len, off): (usize, _, _) = match mode.as_str() {
        "rgba8888" => (32, [8, 8, 8, 8], [16, 8, 0, 24]),
        "rgb565" => (16, [5, 6, 5, 0], [0, 5, 11, 16]),
        _ => crate::i_system::error(d, &format!("Unknown gfxmode value: {mode}\n")),
    };
    println!("I_InitGraphics: framebuffer: x_res: {RESX}, y_res: {RESY}, x_virtual: {RESX}, y_virtual: {RESY}, bpp: {bpp}");
    println!(
        "I_InitGraphics: framebuffer: RGBA: {}{}{}{}, red_off: {}, green_off: {}, blue_off: {}, transp_off: {}",
        len[0], len[1], len[2], len[3], off[0], off[1], off[2], off[3]
    );
    d.i_video.bpp = bpp;
    println!("I_InitGraphics: DOOM screen size: w x h: {SCREENWIDTH} x {SCREENHEIGHT}");
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-scaling", 1);
    if p > 0 {
        let f = crate::z_zone::atoi(&d.m_argv.myargv[p + 1]);
        d.i_video.scaling = f;
        println!("I_InitGraphics: Scaling factor: {f}");
        if f * SCREENWIDTH as i32 > RESX as i32 || f * SCREENHEIGHT as i32 > RESY as i32 {
            // The original scales past the end of the frame buffer and ends with SIGBUS.
            crash(SIGBUS);
        }
    } else {
        println!(
            "I_InitGraphics: Auto-scaling factor: {}",
            (RESX / SCREENWIDTH).min(RESY / SCREENHEIGHT)
        );
    }
}

pub const SIGBUS: i32 = 10;
pub const SIGSEGV: i32 = 11;

/// Ends the process with a signal, as the original does where it reads or writes invalid memory.
/// The C library stdout buffer is not flushed, so its content is lost as in the original.
pub fn crash(sig: i32) -> ! {
    // SAFETY: signal and raise have no memory preconditions. SIG_DFL (0) makes the signal end the process.
    unsafe {
        signal(sig, 0);
        raise(sig);
    }
    std::process::abort()
}

/// Restores the default SIGPIPE action. The Rust runtime ignores SIGPIPE, but the original dies by it.
pub fn default_sigpipe() {
    // SAFETY: signal has no memory preconditions. SIG_DFL is 0, and SIGPIPE is 13.
    unsafe { signal(13, 0) };
}

/// Writes to the C library stdout stream. The C library buffers it as in the original (ADR 0008).
pub fn print(args: std::fmt::Arguments) {
    let s = std::fmt::format(args);
    // SAFETY: the pointer and the length describe a live buffer. __stdoutp is the C library stdout stream.
    unsafe { fwrite(s.as_ptr().cast(), 1, s.len(), __stdoutp) };
}

/// I_StartTic: posts the queued key events (I_GetEvent of the doomgeneric input layer).
pub fn start_tic(d: &mut crate::Doom) {
    use crate::d_main::{post_event, Event, EV_KEYDOWN, EV_KEYUP};
    while let Some((pressed, key)) = dg_get_key(&mut d.i_video) {
        if key == KEY_RSHIFT {
            d.i_video.shiftdown += if pressed { 1 } else { -1 };
        }
        if pressed {
            let typed = if d.i_video.shiftdown <= 0 {
                key
            } else if key & 0x80 != 0 {
                0
            } else {
                SHIFTXFORM[(key & 0x7f) as usize] as i32
            };
            if key != 0 {
                post_event(
                    d,
                    Event {
                        type_: EV_KEYDOWN,
                        data1: key,
                        data2: typed,
                        ..Default::default()
                    },
                );
            }
        } else {
            // The doomgeneric input layer stops reading the queue after a key release.
            if key != 0 {
                post_event(
                    d,
                    Event {
                        type_: EV_KEYUP,
                        data1: key,
                        ..Default::default()
                    },
                );
            }
            return;
        }
    }
}

/// The shifted character table of the original input layer.
const SHIFTXFORM: [u8; 128] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 34, 40, 41, 42, 43, 60, 95, 62, 63, 41, 33,
    64, 35, 36, 37, 94, 38, 42, 40, 58, 58, 60, 43, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
    74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 33, 93, 34, 95, 39, 65,
    66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
    90, 123, 124, 125, 126, 127,
];

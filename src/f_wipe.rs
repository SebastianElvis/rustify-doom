//! The melt screen wipe between game states.

use crate::m_random::m_random;
use crate::Doom;

/// The wipe globals: the start and end screens and the column offsets.
#[derive(Default)]
pub struct State {
    go: bool,
    wipe_scr_start: Vec<u8>,
    wipe_scr_end: Vec<u8>,
    y: Vec<i32>,
    /// The zone offsets of wipe_scr_start, wipe_scr_end, and y in the zone model.
    zone: [usize; 3],
}

/// wipe_shittyColMajorXform with its zone block: the original transposes into a temporary block.
fn xform(d: &mut Doom, end: bool, zone: usize) {
    let a = if end {
        &mut d.f_wipe.wipe_scr_end
    } else {
        &mut d.f_wipe.wipe_scr_start
    };
    shitty_col_major_xform(a, 160, 200);
    let p = d
        .z_zone
        .malloc_copy(&a[..64000], crate::z_zone::PU_STATIC, 0);
    if zone != 0 {
        d.z_zone.mem[zone..zone + 64000].copy_from_slice(&a[..64000]);
    }
    d.z_zone.free(p);
}

/// wipe_shittyColMajorXform: transposes a screen of 16-bit pairs into column order.
fn shitty_col_major_xform(array: &mut [u8], width: usize, height: usize) {
    let mut dest = vec![0u8; width * height * 2];
    for y in 0..height {
        for x in 0..width {
            let s = 2 * (y * width + x);
            let d = 2 * (x * height + y);
            dest[d] = array[s];
            dest[d + 1] = array[s + 1];
        }
    }
    array[..dest.len()].copy_from_slice(&dest);
}

/// wipe_initMelt.
fn init_melt(d: &mut Doom, width: i32, height: i32) {
    let n = (width * height) as usize;
    d.i_video.video_buffer[..n].copy_from_slice(&d.f_wipe.wipe_scr_start[..n]);
    let [zs, ze, _] = d.f_wipe.zone;
    xform(d, false, zs);
    xform(d, true, ze);
    d.f_wipe.zone[2] = d.z_zone.malloc(4 * width, crate::z_zone::PU_STATIC, 0);
    let mut y = vec![0i32; width as usize];
    y[0] = -(m_random(&mut d.m_random) % 16);
    for i in 1..width as usize {
        let r = (m_random(&mut d.m_random) % 3) - 1;
        y[i] = y[i - 1] + r;
        if y[i] > 0 {
            y[i] = 0;
        } else if y[i] == -16 {
            y[i] = -15;
        }
    }
    d.f_wipe.y = y;
}

/// wipe_doMelt: moves each 2-pixel column down by its speed for each tic.
fn do_melt(d: &mut Doom, width: i32, height: i32, mut ticks: i32) -> bool {
    let width = (width / 2) as usize;
    let mut done = true;
    let w = &mut d.f_wipe;
    let scr = &mut d.i_video.video_buffer;
    let copy = |scr: &mut [u8], src: &[u8], s: usize, row: usize, i: usize, n: i32| {
        for j in 0..n as usize {
            let si = 2 * (s + j);
            let di = 2 * ((row + j) * width + i);
            scr[di] = src[si];
            scr[di + 1] = src[si + 1];
        }
    };
    while ticks > 0 {
        ticks -= 1;
        for i in 0..width {
            if w.y[i] < 0 {
                w.y[i] += 1;
                done = false;
            } else if w.y[i] < height {
                let mut dy = if w.y[i] < 16 { w.y[i] + 1 } else { 8 };
                if w.y[i] + dy >= height {
                    dy = height - w.y[i];
                }
                let yi = w.y[i] as usize;
                copy(scr, &w.wipe_scr_end, i * height as usize + yi, yi, i, dy);
                w.y[i] += dy;
                let yi = w.y[i] as usize;
                copy(
                    scr,
                    &w.wipe_scr_start,
                    i * height as usize,
                    yi,
                    i,
                    height - w.y[i],
                );
                done = false;
            }
        }
    }
    done
}

/// wipe_exitMelt.
fn exit_melt(d: &mut Doom) {
    let [zs, ze, zy] = std::mem::take(&mut d.f_wipe.zone);
    if zy != 0 {
        let y: Vec<u8> = d.f_wipe.y.iter().flat_map(|v| v.to_le_bytes()).collect();
        d.z_zone.mem[zy..zy + y.len()].copy_from_slice(&y);
    }
    for p in [zy, zs, ze] {
        d.z_zone.free(p);
    }
    d.f_wipe.y = Vec::new();
    d.f_wipe.wipe_scr_start = Vec::new();
    d.f_wipe.wipe_scr_end = Vec::new();
}

/// wipe_StartScreen: keeps a copy of the current screen.
pub fn start_screen(d: &mut Doom, _x: i32, _y: i32, _width: i32, _height: i32) {
    d.f_wipe.wipe_scr_start = d.i_video.video_buffer.clone();
    d.f_wipe.zone[0] = d
        .z_zone
        .malloc_copy(&d.i_video.video_buffer, crate::z_zone::PU_STATIC, 0);
}

/// wipe_EndScreen: keeps a copy of the new screen and restores the start screen.
pub fn end_screen(d: &mut Doom, x: i32, y: i32, width: i32, height: i32) {
    d.f_wipe.wipe_scr_end = d.i_video.video_buffer.clone();
    d.f_wipe.zone[1] = d
        .z_zone
        .malloc_copy(&d.i_video.video_buffer, crate::z_zone::PU_STATIC, 0);
    let start = std::mem::take(&mut d.f_wipe.wipe_scr_start);
    crate::v_video::draw_block(d, x, y, width, height, &start);
    d.f_wipe.wipe_scr_start = start;
}

/// wipe_ScreenWipe: runs the melt for the given tics. Returns true when the wipe is complete.
/// The binary contains only the melt wipe (wipe_ColorXForm is absent), so `wipeno` has no effect.
pub fn screen_wipe(
    d: &mut Doom,
    _wipeno: i32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    ticks: i32,
) -> bool {
    if !d.f_wipe.go {
        d.f_wipe.go = true;
        init_melt(d, width, height);
    }
    crate::v_video::mark_rect(d, x, y, width, height);
    if do_melt(d, width, height, ticks) {
        d.f_wipe.go = false;
        exit_melt(d);
    }
    !d.f_wipe.go
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn col_major_xform_transposes_pairs() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        shitty_col_major_xform(&mut a, 3, 2);
        assert_eq!(a, vec![1, 2, 7, 8, 3, 4, 9, 10, 5, 6, 11, 12]);
    }

    #[test]
    fn melt_offsets_stay_in_range() {
        let mut d = Box::<Doom>::default();
        d.f_wipe.wipe_scr_start = vec![1; 64000];
        d.f_wipe.wipe_scr_end = vec![2; 64000];
        init_melt(&mut d, 320, 200);
        assert_eq!(d.f_wipe.y.len(), 320);
        assert!(d.f_wipe.y.iter().all(|&v| (-15..=0).contains(&v)));
        assert_eq!(d.m_random.rndindex, 320 & 0xff);
        let mut n = 0;
        while !do_melt(&mut d, 320, 200, 1) {
            n += 1;
        }
        assert!(d.i_video.video_buffer.iter().all(|&p| p == 2));
        assert!(n > 20 && n < 50);
    }
}

//! Angle constants and the trigonometric lookup tables.

include!("data/tables.in");

pub type Angle = u32;
pub const FINEANGLES: usize = 8192;
pub const FINEMASK: usize = FINEANGLES - 1;
pub const ANGLETOFINESHIFT: u32 = 19;
pub const ANG45: Angle = 0x2000_0000;
pub const ANG90: Angle = 0x4000_0000;
pub const ANG180: Angle = 0x8000_0000;
pub const ANG270: Angle = 0xc000_0000;
pub const ANG_MAX: Angle = 0xffff_ffff;
pub const SLOPERANGE: i32 = 2048;
pub const SLOPEBITS: i32 = 11;
pub const DBITS: i32 = 16 - SLOPEBITS;

/// The cosine table starts a quarter turn into the sine table.
pub fn finecosine(i: usize) -> i32 {
    FINESINE[i + FINEANGLES / 4]
}

pub fn finesine(i: usize) -> i32 {
    FINESINE[i]
}

pub fn slope_div(num: u32, den: u32) -> i32 {
    if den < 512 {
        return SLOPERANGE;
    }
    let ans = (num << 3) / (den >> 8);
    if ans <= SLOPERANGE as u32 {
        ans as i32
    } else {
        SLOPERANGE
    }
}

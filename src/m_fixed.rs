//! Fixed-point arithmetic (16.16).

pub type Fixed = i32;
pub const FRACBITS: i32 = 16;
pub const FRACUNIT: Fixed = 1 << FRACBITS;

pub fn fixed_mul(a: Fixed, b: Fixed) -> Fixed {
    ((a as i64 * b as i64) >> FRACBITS) as Fixed
}

pub fn fixed_div(a: Fixed, b: Fixed) -> Fixed {
    if (a.unsigned_abs() >> 14) >= b.unsigned_abs() {
        if (a ^ b) < 0 {
            i32::MIN
        } else {
            i32::MAX
        }
    } else {
        (((a as i64) << FRACBITS) / b as i64) as Fixed
    }
}

//! The sky texture and its fixed vertical position.

use crate::m_fixed::{Fixed, FRACUNIT};
use crate::Doom;

/// The flat name that marks sky ceilings.
pub const SKYFLATNAME: &str = "F_SKY1";
/// The sky angle shift: 22 maps 1024 columns over a full turn.
pub const ANGLETOSKYSHIFT: u32 = 22;

#[derive(Default)]
pub struct State {
    /// The flat number of F_SKY1. G_DoLoadLevel sets it.
    pub skyflatnum: i32,
    /// The texture number of the sky. The game sets it for each level.
    pub skytexture: i32,
    pub skytexturemid: Fixed,
}

/// R_InitSkyMap.
pub fn init_sky_map(d: &mut Doom) {
    d.r_sky.skytexturemid = 100 * FRACUNIT;
}

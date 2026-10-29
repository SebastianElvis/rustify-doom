//! Level statistics for -statdump. The original links empty StatCopy and StatDump bodies.

#[allow(unused_imports)]
use crate::g_game::WbStart;
use crate::Doom;

#[derive(Default)]
pub struct State {}

/// StatCopy. The original body is empty.
pub fn copy(_d: &mut Doom, _stats: &WbStart) {}

/// StatDump. The original body is empty, so no statistics file appears.
pub fn dump(_d: &mut Doom) {}

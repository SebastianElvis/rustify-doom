//! The fixed random number table with separate game and menu indexes.

use crate::tables::RNDTABLE;

#[derive(Default)]
pub struct State {
    pub rndindex: usize,
    pub prndindex: usize,
}

/// The game simulation uses this generator. Demos depend on its sequence.
pub fn p_random(s: &mut State) -> i32 {
    s.prndindex = (s.prndindex + 1) & 0xff;
    RNDTABLE[s.prndindex] as i32
}

/// The menu, the wipe, and other display effects use this generator.
pub fn m_random(s: &mut State) -> i32 {
    s.rndindex = (s.rndindex + 1) & 0xff;
    RNDTABLE[s.rndindex] as i32
}

pub fn clear_random(s: &mut State) {
    s.rndindex = 0;
    s.prndindex = 0;
}

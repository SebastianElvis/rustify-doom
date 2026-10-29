//! Light effects: flicker, flash, strobe, and glow.

use crate::m_random::p_random;
use crate::p_spec::{find_min_surrounding_light, find_sector_from_line_tag, get_next_sector};
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::Doom;

#[derive(Clone, Copy, Default, Debug)]
pub struct FireFlicker {
    pub sector: usize,
    pub count: i32,
    pub maxlight: i32,
    pub minlight: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct LightFlash {
    pub sector: usize,
    pub count: i32,
    pub maxlight: i32,
    pub minlight: i32,
    pub maxtime: i32,
    pub mintime: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Strobe {
    pub sector: usize,
    pub count: i32,
    pub minlight: i32,
    pub maxlight: i32,
    pub darktime: i32,
    pub brighttime: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Glow {
    pub sector: usize,
    pub minlight: i32,
    pub maxlight: i32,
    pub direction: i32,
}

pub const GLOWSPEED: i32 = 8;
pub const STROBEBRIGHT: i32 = 5;
pub const FASTDARK: i32 = 15;
pub const SLOWDARK: i32 = 35;

#[derive(Default)]
pub struct State {}

fn light(d: &mut Doom, sector: usize) -> &mut i32 {
    &mut d.p_setup.sectors[sector].lightlevel
}

/// T_FireFlicker.
pub fn t_fire_flicker(d: &mut Doom, id: ThinkerId) {
    let ThinkerData::FireFlicker(mut f) = d.p_tick.thinkers[id].data else {
        return;
    };
    f.count -= 1;
    if f.count == 0 {
        let amount = (p_random(&mut d.m_random) & 3) * 16;
        let l = light(d, f.sector);
        *l = if *l - amount < f.minlight {
            f.minlight
        } else {
            f.maxlight - amount
        };
        f.count = 4;
    }
    d.p_tick.thinkers[id].data = ThinkerData::FireFlicker(f);
}

/// P_SpawnFireFlicker.
pub fn spawn_fire_flicker(d: &mut Doom, sector: usize) {
    d.p_setup.sectors[sector].special = 0;
    let ll = d.p_setup.sectors[sector].lightlevel;
    let f = FireFlicker {
        sector,
        maxlight: ll,
        minlight: find_min_surrounding_light(d, sector, ll) + 16,
        count: 4,
    };
    d.p_tick
        .add_thinker(Think::FireFlicker, ThinkerData::FireFlicker(f));
}

/// T_LightFlash.
pub fn t_light_flash(d: &mut Doom, id: ThinkerId) {
    let ThinkerData::LightFlash(mut f) = d.p_tick.thinkers[id].data else {
        return;
    };
    f.count -= 1;
    if f.count == 0 {
        if *light(d, f.sector) == f.maxlight {
            *light(d, f.sector) = f.minlight;
            f.count = (p_random(&mut d.m_random) & f.mintime) + 1;
        } else {
            *light(d, f.sector) = f.maxlight;
            f.count = (p_random(&mut d.m_random) & f.maxtime) + 1;
        }
    }
    d.p_tick.thinkers[id].data = ThinkerData::LightFlash(f);
}

/// P_SpawnLightFlash.
pub fn spawn_light_flash(d: &mut Doom, sector: usize) {
    d.p_setup.sectors[sector].special = 0;
    let ll = d.p_setup.sectors[sector].lightlevel;
    let mut f = LightFlash {
        sector,
        maxlight: ll,
        minlight: find_min_surrounding_light(d, sector, ll),
        maxtime: 64,
        mintime: 7,
        count: 0,
    };
    f.count = (p_random(&mut d.m_random) & f.maxtime) + 1;
    d.p_tick
        .add_thinker(Think::LightFlash, ThinkerData::LightFlash(f));
}

/// T_StrobeFlash.
pub fn t_strobe_flash(d: &mut Doom, id: ThinkerId) {
    let ThinkerData::Strobe(mut f) = d.p_tick.thinkers[id].data else {
        return;
    };
    f.count -= 1;
    if f.count == 0 {
        if *light(d, f.sector) == f.minlight {
            *light(d, f.sector) = f.maxlight;
            f.count = f.brighttime;
        } else {
            *light(d, f.sector) = f.minlight;
            f.count = f.darktime;
        }
    }
    d.p_tick.thinkers[id].data = ThinkerData::Strobe(f);
}

/// P_SpawnStrobeFlash.
pub fn spawn_strobe_flash(d: &mut Doom, sector: usize, fast_or_slow: i32, in_sync: bool) {
    let ll = d.p_setup.sectors[sector].lightlevel;
    let mut f = Strobe {
        sector,
        darktime: fast_or_slow,
        brighttime: STROBEBRIGHT,
        maxlight: ll,
        minlight: find_min_surrounding_light(d, sector, ll),
        count: 1,
    };
    if f.minlight == f.maxlight {
        f.minlight = 0;
    }
    d.p_setup.sectors[sector].special = 0;
    if !in_sync {
        f.count = (p_random(&mut d.m_random) & 7) + 1;
    }
    d.p_tick
        .add_thinker(Think::StrobeFlash, ThinkerData::Strobe(f));
}

/// EV_StartLightStrobing.
pub fn ev_start_light_strobing(d: &mut Doom, line: usize) {
    let mut secnum = -1;
    loop {
        secnum = find_sector_from_line_tag(d, line, secnum);
        if secnum < 0 {
            break;
        }
        if d.p_setup.sectors[secnum as usize].specialdata.is_some() {
            continue;
        }
        spawn_strobe_flash(d, secnum as usize, SLOWDARK, false);
    }
}

/// EV_TurnTagLightsOff: sets each tagged sector to the darkest neighbour light.
pub fn ev_turn_tag_lights_off(d: &mut Doom, line: usize) {
    let tag = d.p_setup.lines[line].tag;
    for j in 0..d.p_setup.numsectors {
        if d.p_setup.sectors[j].tag == tag {
            let min = find_min_surrounding_light(d, j, d.p_setup.sectors[j].lightlevel);
            d.p_setup.sectors[j].lightlevel = min;
        }
    }
}

/// EV_LightTurnOn: sets each tagged sector to `bright`, or to the brightest neighbour light when `bright` is 0.
/// The original keeps the found value for the next tagged sectors.
pub fn ev_light_turn_on(d: &mut Doom, line: usize, mut bright: i32) {
    let tag = d.p_setup.lines[line].tag;
    for i in 0..d.p_setup.numsectors {
        if d.p_setup.sectors[i].tag != tag {
            continue;
        }
        if bright == 0 {
            for &l in &d.p_setup.sectors[i].lines {
                if let Some(o) = get_next_sector(d, l, i) {
                    bright = bright.max(d.p_setup.sectors[o].lightlevel);
                }
            }
        }
        d.p_setup.sectors[i].lightlevel = bright;
    }
}

/// T_Glow.
pub fn t_glow(d: &mut Doom, id: ThinkerId) {
    let ThinkerData::Glow(mut g) = d.p_tick.thinkers[id].data else {
        return;
    };
    let l = &mut d.p_setup.sectors[g.sector].lightlevel;
    match g.direction {
        -1 => {
            *l -= GLOWSPEED;
            if *l <= g.minlight {
                *l += GLOWSPEED;
                g.direction = 1;
            }
        }
        1 => {
            *l += GLOWSPEED;
            if *l >= g.maxlight {
                *l -= GLOWSPEED;
                g.direction = -1;
            }
        }
        _ => {}
    }
    d.p_tick.thinkers[id].data = ThinkerData::Glow(g);
}

/// P_SpawnGlowingLight.
pub fn spawn_glowing_light(d: &mut Doom, sector: usize) {
    let ll = d.p_setup.sectors[sector].lightlevel;
    let g = Glow {
        sector,
        minlight: find_min_surrounding_light(d, sector, ll),
        maxlight: ll,
        direction: -1,
    };
    d.p_tick.add_thinker(Think::Glow, ThinkerData::Glow(g));
    d.p_setup.sectors[sector].special = 0;
}

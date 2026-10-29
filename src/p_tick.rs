//! The thinker list and the per-tic world update.
//!
//! The original keeps a circular doubly linked list of heterogeneous thinkers.
//! This module keeps the list in an arena. Index 0 is the list head (thinkercap).
//! Removed thinkers keep their data, so stale references read the last values, as freed zone blocks do.

use crate::p_ceilng::Ceiling;
use crate::p_doors::VlDoor;
use crate::p_floor::FloorMove;
use crate::p_lights::{FireFlicker, Glow, LightFlash, Strobe};
use crate::p_mobj::Mobj;
use crate::p_plats::Plat;
#[allow(unused_imports)]
use crate::Doom;

pub type ThinkerId = usize;

/// The thinker function. `Null` is a stopped thinker; `Removed` waits for unlinking.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Think {
    Null,
    Removed,
    Mobj,
    MoveCeiling,
    VerticalDoor,
    MoveFloor,
    PlatRaise,
    LightFlash,
    StrobeFlash,
    Glow,
    FireFlicker,
}

pub enum ThinkerData {
    None,
    Mobj(Box<Mobj>),
    Ceiling(Ceiling),
    Door(VlDoor),
    Floor(FloorMove),
    Plat(Plat),
    LightFlash(LightFlash),
    Strobe(Strobe),
    Glow(Glow),
    FireFlicker(FireFlicker),
}

pub struct Thinker {
    pub prev: ThinkerId,
    pub next: ThinkerId,
    pub func: Think,
    pub data: ThinkerData,
}

/// A thinker event for the zone model (ADR 0006): the original allocates and frees a zone block for each thinker.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ZoneEvent {
    Init,
    Add(ThinkerId),
    Free(ThinkerId),
    Tic,
}

pub struct State {
    pub thinkers: Vec<Thinker>,
    pub leveltime: i32,
    pub zone_log: Vec<ZoneEvent>,
}

impl Default for State {
    fn default() -> Self {
        State {
            thinkers: vec![Thinker {
                prev: 0,
                next: 0,
                func: Think::Null,
                data: ThinkerData::None,
            }],
            leveltime: 0,
            zone_log: Vec::new(),
        }
    }
}

impl State {
    /// Empties the list (P_InitThinkers). All earlier ids become invalid.
    pub fn init_thinkers(&mut self) {
        self.thinkers.truncate(1);
        self.zone_log.push(ZoneEvent::Init);
        self.thinkers[0].prev = 0;
        self.thinkers[0].next = 0;
    }

    /// Appends a thinker at the tail (P_AddThinker) and returns its id.
    pub fn add_thinker(&mut self, func: Think, data: ThinkerData) -> ThinkerId {
        let id = self.thinkers.len();
        let tail = self.thinkers[0].prev;
        self.thinkers.push(Thinker {
            prev: tail,
            next: 0,
            func,
            data,
        });
        self.thinkers[tail].next = id;
        self.thinkers[0].prev = id;
        self.zone_log.push(ZoneEvent::Add(id));
        id
    }

    /// Marks a thinker for removal (P_RemoveThinker). P_RunThinkers unlinks it later.
    pub fn remove_thinker(&mut self, id: ThinkerId) {
        self.thinkers[id].func = Think::Removed;
    }

    pub fn unlink(&mut self, id: ThinkerId) {
        let (prev, next) = (self.thinkers[id].prev, self.thinkers[id].next);
        self.thinkers[next].prev = prev;
        self.thinkers[prev].next = next;
        self.zone_log.push(ZoneEvent::Free(id));
    }

    pub fn mobj(&self, id: ThinkerId) -> &Mobj {
        match &self.thinkers[id].data {
            ThinkerData::Mobj(m) => m,
            _ => panic!("thinker {id} is not a map object"),
        }
    }

    pub fn mobj_mut(&mut self, id: ThinkerId) -> &mut Mobj {
        match &mut self.thinkers[id].data {
            ThinkerData::Mobj(m) => m,
            _ => panic!("thinker {id} is not a map object"),
        }
    }

    pub fn is_mobj(&self, id: ThinkerId) -> bool {
        matches!(self.thinkers[id].data, ThinkerData::Mobj(_))
    }

    /// Returns the ids of the linked thinkers in list order.
    pub fn ids(&self) -> Vec<ThinkerId> {
        let mut out = Vec::new();
        let mut cur = self.thinkers[0].next;
        while cur != 0 {
            out.push(cur);
            cur = self.thinkers[cur].next;
        }
        out
    }
}

/// P_RunThinkers: runs each linked thinker once and unlinks removed thinkers.
/// The next link is read after the thinker runs, so new thinkers at the tail also run.
pub fn run_thinkers(d: &mut Doom) {
    let mut cur = d.p_tick.thinkers[0].next;
    while cur != 0 {
        let func = d.p_tick.thinkers[cur].func;
        if func == Think::Removed {
            let next = d.p_tick.thinkers[cur].next;
            d.p_tick.unlink(cur);
            cur = next;
            continue;
        }
        match func {
            Think::Null | Think::Removed => {}
            Think::Mobj => crate::p_mobj::mobj_thinker(d, cur),
            Think::MoveCeiling => crate::p_ceilng::t_move_ceiling(d, cur),
            Think::VerticalDoor => crate::p_doors::t_vertical_door(d, cur),
            Think::MoveFloor => crate::p_floor::t_move_floor(d, cur),
            Think::PlatRaise => crate::p_plats::t_plat_raise(d, cur),
            Think::LightFlash => crate::p_lights::t_light_flash(d, cur),
            Think::StrobeFlash => crate::p_lights::t_strobe_flash(d, cur),
            Think::Glow => crate::p_lights::t_glow(d, cur),
            Think::FireFlicker => crate::p_lights::t_fire_flicker(d, cur),
        }
        cur = d.p_tick.thinkers[cur].next;
    }
}

/// P_Ticker: runs one game tic of the world.
pub fn ticker(d: &mut Doom) {
    if d.g_game.paused {
        return;
    }
    // The world pauses while the menu is open, after at least one tic has run.
    if !d.g_game.netgame
        && d.m_menu.menuactive
        && !d.g_game.demoplayback
        && d.g_game.players[d.g_game.consoleplayer].viewz != 1
    {
        return;
    }
    d.p_tick.zone_log.push(ZoneEvent::Tic);
    for i in 0..crate::g_game::MAXPLAYERS {
        if d.g_game.playeringame[i] {
            crate::p_user::player_think(d, i);
        }
    }
    run_thinkers(d);
    crate::p_spec::update_specials(d);
    crate::p_mobj::respawn_specials(d);
    d.p_tick.leveltime += 1;
}

//! The tic loop: ticcmd building and game tic execution for the single local node.

use crate::g_game::Ticcmd;
use crate::Doom;

pub const BACKUPTICS: usize = 128;

#[derive(Clone, Copy, Default)]
pub struct TiccmdSet {
    pub cmds: [Ticcmd; 8],
    pub ingame: [bool; 8],
}

pub struct State {
    pub ticdata: Vec<TiccmdSet>,
    pub maketic: i32,
    pub lasttime: i32,
    pub skiptics: i32,
    pub oldentertics: i32,
    /// Set by G_TimeDemo: run one tic for each frame.
    pub singletics: bool,
    /// The old synchronisation rule. D_CheckNetGame sets it for the local game.
    pub old_sync: bool,
    /// The commands of the tic that G_Ticker runs (netcmds).
    pub netcmds: [Ticcmd; 8],
}

impl Default for State {
    fn default() -> Self {
        State {
            ticdata: vec![TiccmdSet::default(); BACKUPTICS],
            maketic: 0,
            lasttime: 0,
            skiptics: 0,
            oldentertics: 0,
            singletics: false,
            old_sync: false,
            netcmds: [Ticcmd::default(); 8],
        }
    }
}

fn slot(tic: i32) -> usize {
    tic.rem_euclid(BACKUPTICS as i32) as usize
}

/// BuildNewTic: reads input and builds one ticcmd. Returns false when the buffer is full enough.
fn build_new_tic(d: &mut Doom) -> bool {
    let gameticdiv = d.g_game.gametic;
    crate::i_video::start_tic(d);
    crate::d_main::process_events(d);
    crate::m_menu::ticker(d);
    let ahead = d.d_loop.maketic - gameticdiv;
    if if d.d_loop.old_sync {
        ahead >= 5
    } else {
        ahead > 2
    } {
        return false;
    }
    let mut cmd = Ticcmd::default();
    let maketic = d.d_loop.maketic;
    crate::g_game::build_ticcmd(d, &mut cmd, maketic);
    let set = &mut d.d_loop.ticdata[slot(maketic)];
    set.cmds[0] = cmd;
    set.ingame[0] = true;
    d.d_loop.maketic += 1;
    true
}

/// NetUpdate: builds the ticcmds for the tics that passed since the last call.
pub fn net_update(d: &mut Doom) {
    if d.d_loop.singletics {
        return;
    }
    let nowtime = get_time();
    let mut newtics = nowtime - d.d_loop.lasttime;
    d.d_loop.lasttime = nowtime;
    if d.d_loop.skiptics <= newtics {
        newtics -= d.d_loop.skiptics;
        d.d_loop.skiptics = 0;
    } else {
        d.d_loop.skiptics -= newtics;
        newtics = 0;
    }
    for _ in 0..newtics {
        if !build_new_tic(d) {
            break;
        }
    }
}

/// TryRunTics: runs the available tics, or waits up to one tic for new ones.
pub fn try_run_tics(d: &mut Doom) {
    let entertic = get_time();
    let realtics = entertic - d.d_loop.oldentertics;
    d.d_loop.oldentertics = entertic;
    if d.d_loop.singletics {
        build_new_tic(d);
    } else {
        net_update(d);
    }
    let mut lowtic = d.d_loop.maketic;
    let availabletics = lowtic - d.g_game.gametic;
    let mut counts = if d.d_loop.old_sync {
        let c = if realtics < availabletics - 1 {
            realtics + 1
        } else if realtics < availabletics {
            realtics
        } else {
            availabletics
        };
        c.max(1)
    } else {
        availabletics
    };
    counts = counts.max(1);
    while lowtic < d.g_game.gametic + counts {
        net_update(d);
        lowtic = d.d_loop.maketic;
        if lowtic < d.g_game.gametic {
            crate::i_system::error(d, "TryRunTics: lowtic < gametic");
        }
        if get_time() > entertic {
            return;
        }
        crate::i_system::sleep(1);
    }
    while counts > 0 {
        counts -= 1;
        let s = slot(d.g_game.gametic);
        let set = &mut d.d_loop.ticdata[s];
        set.ingame[1..].iter_mut().for_each(|b| *b = false);
        if d.g_game.gametic > lowtic {
            crate::i_system::error(d, "gametic>lowtic");
        }
        let set = d.d_loop.ticdata[s];
        run_tic(d, &set);
        d.g_game.gametic += 1;
        // TicdupSquash.
        for c in d.d_loop.ticdata[s].cmds.iter_mut() {
            c.chatchar = 0;
            if c.buttons & crate::g_game::BT_SPECIAL != 0 {
                c.buttons = 0;
            }
        }
        net_update(d);
    }
}

/// RunTic: hands the commands to the game and runs one game tic.
fn run_tic(d: &mut Doom, set: &TiccmdSet) {
    d.d_loop.netcmds = set.cmds;
    if d.d_main.advancedemo {
        crate::d_main::do_advance_demo(d);
    }
    crate::g_game::ticker(d);
}

/// I_GetTime.
pub fn get_time() -> i32 {
    crate::i_system::get_time()
}

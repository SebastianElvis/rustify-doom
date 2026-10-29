//! Game control: game state, player commands, level start and exit, demos, and save and load requests.

use crate::d_items::{NUMAMMO, NUMCARDS, NUMPOWERS, NUMWEAPONS};
#[allow(unused_imports)]
use crate::d_main::Event;
use crate::m_fixed::Fixed;
use crate::p_mobj::MobjId;
use crate::p_pspr::{PspDef, NUMPSPRITES};
use crate::p_setup::MapThing;
#[allow(unused_imports)]
use crate::Doom;

pub const MAXPLAYERS: usize = 4;

pub const GS_FORCE_WIPE: i32 = -1;
pub const GS_LEVEL: i32 = 0;
pub const GS_INTERMISSION: i32 = 1;
pub const GS_FINALE: i32 = 2;
pub const GS_DEMOSCREEN: i32 = 3;

pub const GA_NOTHING: i32 = 0;
pub const GA_LOADLEVEL: i32 = 1;
pub const GA_NEWGAME: i32 = 2;
pub const GA_LOADGAME: i32 = 3;
pub const GA_SAVEGAME: i32 = 4;
pub const GA_PLAYDEMO: i32 = 5;
pub const GA_COMPLETED: i32 = 6;
pub const GA_VICTORY: i32 = 7;
pub const GA_WORLDDONE: i32 = 8;
pub const GA_SCREENSHOT: i32 = 9;

pub const SK_NOITEMS: i32 = -1;
pub const SK_BABY: i32 = 0;
pub const SK_EASY: i32 = 1;
pub const SK_MEDIUM: i32 = 2;
pub const SK_HARD: i32 = 3;
pub const SK_NIGHTMARE: i32 = 4;

pub const PST_LIVE: i32 = 0;
pub const PST_DEAD: i32 = 1;
pub const PST_REBORN: i32 = 2;

pub const CF_NOCLIP: i32 = 1;
pub const CF_GODMODE: i32 = 2;
pub const CF_NOMOMENTUM: i32 = 4;

pub const BT_ATTACK: u8 = 1;
pub const BT_USE: u8 = 2;
pub const BT_SPECIAL: u8 = 128;
pub const BT_SPECIALMASK: u8 = 3;
pub const BT_CHANGE: u8 = 4;
pub const BT_WEAPONMASK: u8 = 8 + 16 + 32;
pub const BT_WEAPONSHIFT: u8 = 3;
pub const BTS_PAUSE: u8 = 1;
pub const BTS_SAVEGAME: u8 = 2;
pub const BTS_SAVEMASK: u8 = 4 + 8 + 16;
pub const BTS_SAVESHIFT: u8 = 2;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Ticcmd {
    pub forwardmove: i8,
    pub sidemove: i8,
    pub angleturn: i16,
    pub chatchar: u8,
    pub buttons: u8,
    pub consistancy: i16,
}

#[derive(Clone, Default, Debug)]
pub struct Player {
    pub mo: Option<MobjId>,
    pub playerstate: i32,
    pub cmd: Ticcmd,
    pub viewz: Fixed,
    pub viewheight: Fixed,
    pub deltaviewheight: Fixed,
    pub bob: Fixed,
    pub health: i32,
    pub armorpoints: i32,
    pub armortype: i32,
    pub powers: [i32; NUMPOWERS],
    pub cards: [bool; NUMCARDS],
    pub backpack: bool,
    pub frags: [i32; MAXPLAYERS],
    pub readyweapon: i32,
    pub pendingweapon: i32,
    pub weaponowned: [bool; NUMWEAPONS],
    pub ammo: [i32; NUMAMMO],
    pub maxammo: [i32; NUMAMMO],
    pub attackdown: bool,
    pub usedown: bool,
    pub cheats: i32,
    pub refire: i32,
    pub killcount: i32,
    pub itemcount: i32,
    pub secretcount: i32,
    pub message: Option<String>,
    pub damagecount: i32,
    pub bonuscount: i32,
    pub attacker: Option<MobjId>,
    pub extralight: i32,
    pub fixedcolormap: i32,
    pub colormap: i32,
    pub psprites: [PspDef; NUMPSPRITES],
    pub didsecret: bool,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct WbPlayer {
    pub in_: bool,
    pub skills: i32,
    pub sitems: i32,
    pub ssecret: i32,
    pub stime: i32,
    pub frags: [i32; MAXPLAYERS],
    pub score: i32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct WbStart {
    pub epsd: i32,
    pub didsecret: bool,
    pub last: i32,
    pub next: i32,
    pub maxkills: i32,
    pub maxitems: i32,
    pub maxsecret: i32,
    pub maxfrags: i32,
    pub partime: i32,
    pub pnum: i32,
    pub plyr: [WbPlayer; MAXPLAYERS],
}

pub const BACKUPTICS: usize = 128;
pub const NUMKEYS: usize = 256;
pub const BODYQUESIZE: usize = 32;
const TURBOTHRESHOLD: i8 = 0x32;
const SLOWTURNTICS: i32 = 6;
const DEMOMARKER: u8 = 0x80;
const DOOM_191_VERSION: u8 = 111;
const SAVEGAMESIZE: usize = 0x2c000;
const ANGLETURN: [i32; 3] = [640, 1280, 320];
const S_SARG_RUN1: usize = 477;
const S_SARG_PAIN2: usize = 489;
const FAST_MISSILES: [usize; 3] = [
    crate::info::MT_BRUISERSHOT,
    crate::info::MT_HEADSHOT,
    crate::info::MT_TROOPSHOT,
];

/// The tics of a state, with the -fast and Nightmare changes of G_InitNew.
pub fn state_tics(d: &Doom, state: usize) -> i32 {
    if (S_SARG_RUN1..=S_SARG_PAIN2).contains(&state) {
        d.g_game.sarg_tics[state - S_SARG_RUN1]
    } else {
        crate::info::STATES[state].tics
    }
}

/// The speed of a thing type, with the -fast and Nightmare changes of G_InitNew.
pub fn mobj_speed(d: &Doom, type_: usize) -> Fixed {
    match FAST_MISSILES.iter().position(|&t| t == type_) {
        Some(i) => d.g_game.missile_speeds[i],
        None => crate::info::MOBJINFO[type_].speed,
    }
}

// The default key bindings of the original data (no configuration file is read).
const KEY_RIGHT: usize = 174;
const KEY_LEFT: usize = 172;
const KEY_UP: usize = 173;
const KEY_DOWN: usize = 175;
const KEY_STRAFELEFT: usize = 160;
const KEY_STRAFERIGHT: usize = 161;
const KEY_FIRE: usize = 163;
const KEY_USE: usize = 162;
const KEY_STRAFE: usize = 184;
const KEY_SPEED: usize = 182;
const KEY_PAUSE: i32 = 255;
const KEY_DEMO_QUIT: usize = b'q' as usize;
const KEY_SPY: i32 = 216;
const KEY_WEAPONS: [usize; 8] = [49, 50, 51, 52, 53, 54, 55, 56];

/// The par times of Doom 1 episodes 1 to 3 (seconds).
const PARS: [[i32; 10]; 4] = [
    [0; 10],
    [0, 30, 75, 120, 90, 165, 180, 180, 30, 165],
    [0, 90, 90, 90, 120, 90, 360, 240, 30, 170],
    [0, 90, 45, 90, 150, 90, 90, 165, 30, 135],
];
/// The par times of Doom 2 maps (seconds).
const CPARS: [i32; 32] = [
    30, 90, 120, 120, 90, 150, 120, 120, 270, 90, 210, 150, 150, 150, 210, 150, 420, 150, 210, 150,
    240, 150, 180, 150, 150, 300, 330, 420, 300, 180, 120, 30,
];
const PLAYER_NAMES: [&str; 4] = ["Green: ", "Indigo: ", "Brown: ", "Red: "];

/// The game control globals.
pub struct State {
    pub gamestate: i32,
    pub gameaction: i32,
    pub gameskill: i32,
    pub gameepisode: i32,
    pub gamemap: i32,
    pub players: [Player; MAXPLAYERS],
    pub playeringame: [bool; MAXPLAYERS],
    pub consoleplayer: usize,
    pub displayplayer: usize,
    pub gametic: i32,
    pub totalkills: i32,
    pub totalitems: i32,
    pub totalsecret: i32,
    pub wminfo: WbStart,
    pub paused: bool,
    pub usergame: bool,
    pub viewactive: bool,
    pub deathmatch: i32,
    pub netgame: bool,
    pub nomonsters: bool,
    pub respawnparm: bool,
    pub fastparm: bool,
    pub demoplayback: bool,
    pub demorecording: bool,
    pub timingdemo: bool,
    pub singledemo: bool,
    pub precache: bool,
    pub levelstarttic: i32,
    pub bodyqueslot: usize,
    pub respawnmonsters: bool,
    /// The raw values of -nomonsters, -respawn, and -fast (their argument positions) for the demo header.
    pub parm_values: [i32; 3],
    pub savegamedir: String,
    pub lowres_turn: bool,
    pub longtics: bool,
    pub netdemo: bool,
    pub demobuffer: Vec<u8>,
    /// The demo position. `DEMO_P_NULL` stands for the NULL pointer before the first demo.
    pub demo_p: usize,
    pub demoend: usize,
    pub demoname: String,
    pub defdemoname: String,
    pub starttime: i32,
    pub sendpause: bool,
    pub sendsave: bool,
    pub savegameslot: i32,
    pub savedescription: String,
    pub savename: String,
    pub secretexit: bool,
    pub d_skill: i32,
    pub d_episode: i32,
    pub d_map: i32,
    pub gamekeydown: [bool; NUMKEYS],
    pub forwardmove: [i32; 2],
    pub sidemove: [i32; 2],
    turnheld: i32,
    lowres_carry: i16,
    consistancy: [[u8; BACKUPTICS]; MAXPLAYERS],
    turbodetected: [bool; MAXPLAYERS],
    oldgamestate: i32,
    bodyque: [Option<MobjId>; BODYQUESIZE],
    /// The tics of S_SARG_RUN1..=S_SARG_PAIN2 and the speeds of the three fast missiles. G_InitNew changes them.
    sarg_tics: [i32; 13],
    missile_speeds: [Fixed; 3],
}

impl Default for State {
    fn default() -> Self {
        State {
            gamestate: 0,
            gameaction: 0,
            gameskill: 0,
            gameepisode: 0,
            gamemap: 0,
            players: Default::default(),
            playeringame: [false; MAXPLAYERS],
            consoleplayer: 0,
            displayplayer: 0,
            gametic: 0,
            totalkills: 0,
            totalitems: 0,
            totalsecret: 0,
            wminfo: WbStart::default(),
            paused: false,
            usergame: false,
            viewactive: false,
            deathmatch: 0,
            netgame: false,
            nomonsters: false,
            respawnparm: false,
            fastparm: false,
            demoplayback: false,
            demorecording: false,
            timingdemo: false,
            singledemo: false,
            precache: true,
            levelstarttic: 0,
            bodyqueslot: 0,
            respawnmonsters: false,
            parm_values: [0; 3],
            savegamedir: String::new(),
            lowres_turn: false,
            longtics: false,
            netdemo: false,
            demobuffer: Vec::new(),
            demo_p: DEMO_P_NULL,
            demoend: 0,
            demoname: String::new(),
            defdemoname: String::new(),
            starttime: 0,
            sendpause: false,
            sendsave: false,
            savegameslot: 0,
            savedescription: String::new(),
            savename: String::new(),
            secretexit: false,
            d_skill: 0,
            d_episode: 0,
            d_map: 0,
            gamekeydown: [false; NUMKEYS],
            forwardmove: [0x19, 0x32],
            sidemove: [0x18, 0x28],
            turnheld: 0,
            lowres_carry: 0,
            consistancy: [[0; BACKUPTICS]; MAXPLAYERS],
            turbodetected: [false; MAXPLAYERS],
            oldgamestate: 0,
            bodyque: [None; BODYQUESIZE],
            sarg_tics: std::array::from_fn(|i| crate::info::STATES[S_SARG_RUN1 + i].tics),
            missile_speeds: FAST_MISSILES.map(|t| crate::info::MOBJINFO[t].speed),
        }
    }
}

pub const DEMO_P_NULL: usize = usize::MAX;

impl State {
    /// Writes one demo byte. The original writes past the end of a small demo buffer.
    fn poke(&mut self, i: usize, b: u8) {
        if i >= self.demobuffer.len() {
            self.demobuffer.resize(i + 1, 0);
        }
        self.demobuffer[i] = b;
    }

    /// Sets nomonsters, respawnparm, and fastparm from their raw values.
    pub fn set_parms(&mut self, nomonsters: i32, respawn: i32, fast: i32) {
        self.parm_values = [nomonsters, respawn, fast];
        self.nomonsters = nomonsters != 0;
        self.respawnparm = respawn != 0;
        self.fastparm = fast != 0;
    }

    /// Scales the movement speeds by the -turbo percentage.
    pub fn set_turbo(&mut self, scale: i32) {
        for i in 0..2 {
            self.forwardmove[i] = self.forwardmove[i] * scale / 100;
            self.sidemove[i] = self.sidemove[i] * scale / 100;
        }
    }
}

/// G_Responder.
pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    use crate::d_main::{EV_JOYSTICK, EV_KEYDOWN, EV_KEYUP, EV_MOUSE};
    let g = &mut d.g_game;
    if g.gamestate == GS_LEVEL
        && ev.type_ == EV_KEYDOWN
        && ev.data1 == KEY_SPY
        && (g.singledemo || g.deathmatch == 0)
    {
        loop {
            g.displayplayer += 1;
            if g.displayplayer == MAXPLAYERS {
                g.displayplayer = 0;
            }
            if g.playeringame[g.displayplayer] || g.displayplayer == g.consoleplayer {
                break;
            }
        }
        return true;
    }
    if g.gameaction == GA_NOTHING
        && !g.singledemo
        && (g.demoplayback || g.gamestate == GS_DEMOSCREEN)
    {
        if ev.type_ == EV_KEYDOWN
            || ((ev.type_ == EV_MOUSE || ev.type_ == EV_JOYSTICK) && ev.data1 != 0)
        {
            crate::m_menu::start_control_panel(d);
            return true;
        }
        return false;
    }
    if g.gamestate == GS_LEVEL
        && (crate::hu_stuff::responder(d, ev)
            || crate::st_stuff::responder(d, ev)
            || crate::am_map::responder(d, ev))
    {
        return true;
    }
    if d.g_game.gamestate == GS_FINALE && crate::f_finale::responder(d, ev) {
        return true;
    }
    let g = &mut d.g_game;
    match ev.type_ {
        EV_KEYDOWN => {
            if ev.data1 == KEY_PAUSE {
                g.sendpause = true;
            } else if (0..NUMKEYS as i32).contains(&ev.data1) {
                g.gamekeydown[ev.data1 as usize] = true;
            }
            true
        }
        EV_KEYUP => {
            if (0..NUMKEYS as i32).contains(&ev.data1) {
                g.gamekeydown[ev.data1 as usize] = false;
            }
            false
        }
        EV_MOUSE | EV_JOYSTICK => true,
        _ => false,
    }
}

/// G_BuildTiccmd: makes the ticcmd of the console player from the held keys.
pub fn build_ticcmd(d: &mut Doom, cmd: &mut Ticcmd, maketic: i32) {
    *cmd = Ticcmd::default();
    let cp = d.g_game.consoleplayer;
    cmd.consistancy =
        d.g_game.consistancy[cp][maketic.rem_euclid(BACKUPTICS as i32) as usize] as i16;
    let g = &mut d.g_game;
    let k = g.gamekeydown;
    let strafe = k[KEY_STRAFE];
    let speed = k[KEY_SPEED] as usize;
    let (mut forward, mut side) = (0i32, 0i32);
    if k[KEY_RIGHT] || k[KEY_LEFT] {
        g.turnheld += 1;
    } else {
        g.turnheld = 0;
    }
    let tspeed = if g.turnheld < SLOWTURNTICS { 2 } else { speed };
    let mut angleturn = 0i32;
    if strafe {
        if k[KEY_RIGHT] {
            side += g.sidemove[speed];
        }
        if k[KEY_LEFT] {
            side -= g.sidemove[speed];
        }
    } else {
        if k[KEY_RIGHT] {
            angleturn -= ANGLETURN[tspeed];
        }
        if k[KEY_LEFT] {
            angleturn += ANGLETURN[tspeed];
        }
    }
    if k[KEY_UP] {
        forward += g.forwardmove[speed];
    }
    if k[KEY_DOWN] {
        forward -= g.forwardmove[speed];
    }
    if k[KEY_STRAFELEFT] {
        side -= g.sidemove[speed];
    }
    if k[KEY_STRAFERIGHT] {
        side += g.sidemove[speed];
    }
    cmd.chatchar = crate::hu_stuff::dequeue_chat_char(d);
    let g = &mut d.g_game;
    if k[KEY_FIRE] {
        cmd.buttons |= BT_ATTACK;
    }
    if k[KEY_USE] {
        cmd.buttons |= BT_USE;
    }
    if let Some(i) = KEY_WEAPONS.iter().position(|&w| k[w]) {
        cmd.buttons |= BT_CHANGE | ((i as u8) << BT_WEAPONSHIFT);
    }
    let maxplmove = g.forwardmove[1];
    forward = forward.clamp(-maxplmove, maxplmove);
    side = side.clamp(-maxplmove, maxplmove);
    cmd.forwardmove = forward as i8;
    cmd.sidemove = side as i8;
    cmd.angleturn = angleturn as i16;
    if g.sendpause {
        g.sendpause = false;
        cmd.buttons = BT_SPECIAL | BTS_PAUSE;
    }
    if g.sendsave {
        g.sendsave = false;
        cmd.buttons = BT_SPECIAL | BTS_SAVEGAME | ((g.savegameslot as u8) << BTS_SAVESHIFT);
    }
    if g.lowres_turn {
        let desired = cmd.angleturn.wrapping_add(g.lowres_carry);
        cmd.angleturn = (desired.wrapping_add(128) as u16 & 0xff00) as i16;
        g.lowres_carry = desired.wrapping_sub(cmd.angleturn);
    }
}

/// G_DoLoadLevel.
fn do_load_level(d: &mut Doom) {
    d.r_sky.skyflatnum = crate::r_data::flat_num_for_name(d, crate::r_sky::SKYFLATNAME);
    use crate::d_main::{COMMERCIAL, EXE_CHEX, EXE_FINAL2};
    if d.d_main.gamemode == COMMERCIAL
        && (d.d_main.gameversion == EXE_FINAL2 || d.d_main.gameversion == EXE_CHEX)
    {
        let m = d.g_game.gamemap;
        let name = if m < 12 {
            "SKY1"
        } else if m < 21 {
            "SKY2"
        } else {
            "SKY3"
        };
        d.r_sky.skytexture = crate::r_data::texture_num_for_name(d, name);
    }
    let g = &mut d.g_game;
    g.levelstarttic = g.gametic;
    if d.d_main.wipegamestate == GS_LEVEL {
        d.d_main.wipegamestate = -1;
    }
    g.gamestate = GS_LEVEL;
    for i in 0..MAXPLAYERS {
        g.turbodetected[i] = false;
        if g.playeringame[i] && g.players[i].playerstate == PST_DEAD {
            g.players[i].playerstate = PST_REBORN;
        }
        g.players[i].frags = [0; MAXPLAYERS];
    }
    let (e, m, sk) = (g.gameepisode, g.gamemap, g.gameskill);
    crate::p_setup::setup_level(d, e, m, 0, sk);
    let g = &mut d.g_game;
    g.displayplayer = g.consoleplayer;
    g.gameaction = GA_NOTHING;
    g.gamekeydown = [false; NUMKEYS];
    g.sendpause = false;
    g.sendsave = false;
    g.paused = false;
    if d.m_menu.testcontrols {
        g.players[g.consoleplayer].message = Some("Press escape to quit.".into());
    }
}

/// G_ReadDemoTiccmd.
fn read_demo_ticcmd(d: &mut Doom, cmd: &mut Ticcmd) {
    // The original reads past a truncated demo into the next zone blocks.
    let z = &d.z_zone;
    let base = z.lumpcache.get(z.demo_lump).copied().unwrap_or(0);
    let g = &mut d.g_game;
    let at = |g: &State, i: usize| match g.demobuffer.get(i) {
        Some(&b) => b,
        None if base != 0 => z.mem.get(base + i).copied().unwrap_or(DEMOMARKER),
        None => DEMOMARKER,
    };
    if at(g, g.demo_p) == DEMOMARKER {
        check_demo_status(d);
        return;
    }
    let p = g.demo_p;
    cmd.forwardmove = at(g, p) as i8;
    cmd.sidemove = at(g, p + 1) as i8;
    if g.longtics {
        cmd.angleturn = (at(g, p + 2) as u16 | ((at(g, p + 3) as u16) << 8)) as i16;
        g.demo_p += 1;
    } else {
        cmd.angleturn = ((at(g, p + 2) as u16) << 8) as i16;
    }
    cmd.buttons = at(g, g.demo_p + 3);
    g.demo_p += 4;
}

/// G_WriteDemoTiccmd.
fn write_demo_ticcmd(d: &mut Doom, cmd: &mut Ticcmd) {
    if d.g_game.gamekeydown[KEY_DEMO_QUIT] {
        check_demo_status(d);
    }
    let g = &mut d.g_game;
    let mut p = g.demo_p;
    let mut put = |g: &mut State, b: u8| {
        g.poke(p, b);
        p += 1;
    };
    put(g, cmd.forwardmove as u8);
    put(g, cmd.sidemove as u8);
    if g.longtics {
        put(g, cmd.angleturn as u8);
        put(g, (cmd.angleturn >> 8) as u8);
    } else {
        put(g, (cmd.angleturn >> 8) as u8);
    }
    put(g, cmd.buttons);
    // During playback demo_p points into the demo lump. The zone holds the lump above the
    // recording buffer, so the end test is true.
    if g.demo_p + 16 > g.demoend || g.demoplayback {
        check_demo_status(d);
        return;
    }
    read_demo_ticcmd(d, cmd);
}

/// G_Ticker: runs the pending game actions and one tic of the current game state.
pub fn ticker(d: &mut Doom) {
    for i in 0..MAXPLAYERS {
        if d.g_game.playeringame[i] && d.g_game.players[i].playerstate == PST_REBORN {
            do_reborn(d, i);
        }
    }
    while d.g_game.gameaction != GA_NOTHING {
        match d.g_game.gameaction {
            GA_LOADLEVEL => do_load_level(d),
            GA_NEWGAME => do_new_game(d),
            GA_LOADGAME => do_load_game(d),
            GA_SAVEGAME => do_save_game(d),
            GA_PLAYDEMO => do_play_demo(d),
            GA_COMPLETED => do_completed(d),
            GA_VICTORY => crate::f_finale::start_finale(d),
            GA_WORLDDONE => do_world_done(d),
            GA_SCREENSHOT => {
                crate::v_video::screen_shot(d, "DOOM");
                let cp = d.g_game.consoleplayer;
                d.g_game.players[cp].message = Some("screen shot".into());
                d.g_game.gameaction = GA_NOTHING;
            }
            _ => {}
        }
    }
    let buf = d.g_game.gametic.rem_euclid(BACKUPTICS as i32) as usize;
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        let mut cmd = d.d_loop.netcmds[i];
        if d.g_game.demoplayback {
            read_demo_ticcmd(d, &mut cmd);
        }
        if d.g_game.demorecording {
            write_demo_ticcmd(d, &mut cmd);
        }
        let g = &mut d.g_game;
        g.players[i].cmd = cmd;
        if cmd.forwardmove > TURBOTHRESHOLD {
            g.turbodetected[i] = true;
        }
        if g.gametic & 31 == 0
            && ((g.gametic >> 5) as usize % MAXPLAYERS) == i
            && g.turbodetected[i]
        {
            let cp = g.consoleplayer;
            g.players[cp].message = Some(format!("{} is turbo!", PLAYER_NAMES[i]));
            g.turbodetected[i] = false;
        }
        if g.netgame && !g.netdemo {
            if g.gametic > BACKUPTICS as i32 && g.consistancy[i][buf] as i16 != cmd.consistancy {
                let msg = format!(
                    "consistency failure ({} should be {})",
                    cmd.consistancy, g.consistancy[i][buf]
                );
                crate::i_system::error(d, &msg);
            }
            let v = match d.g_game.players[i].mo {
                Some(mo) => d.p_tick.mobj(mo).x as u8,
                None => d.m_random.rndindex as u8,
            };
            d.g_game.consistancy[i][buf] = v;
        }
    }
    for i in 0..MAXPLAYERS {
        let g = &mut d.g_game;
        if !g.playeringame[i] || g.players[i].cmd.buttons & BT_SPECIAL == 0 {
            continue;
        }
        let b = g.players[i].cmd.buttons;
        match b & BT_SPECIALMASK {
            BTS_PAUSE => {
                g.paused = !g.paused;
                if g.paused {
                    crate::s_sound::pause_sound(d);
                } else {
                    crate::s_sound::resume_sound(d);
                }
            }
            BTS_SAVEGAME => {
                if g.savedescription.is_empty() {
                    g.savedescription = "NET GAME".into();
                }
                g.savegameslot = ((b & BTS_SAVEMASK) >> BTS_SAVESHIFT) as i32;
                g.gameaction = GA_SAVEGAME;
            }
            _ => {}
        }
    }
    // WI_End only frees the intermission graphics.
    d.g_game.oldgamestate = d.g_game.gamestate;
    match d.g_game.gamestate {
        GS_LEVEL => {
            crate::p_tick::ticker(d);
            crate::st_stuff::ticker(d);
            crate::am_map::ticker(d);
            crate::hu_stuff::ticker(d);
        }
        GS_INTERMISSION => crate::wi_stuff::ticker(d),
        GS_FINALE => crate::f_finale::ticker(d),
        GS_DEMOSCREEN => crate::d_main::page_ticker(d),
        _ => {}
    }
}

/// G_PlayerFinishLevel.
fn player_finish_level(d: &mut Doom, player: usize) {
    let p = &mut d.g_game.players[player];
    p.powers = [0; NUMPOWERS];
    p.cards = [false; NUMCARDS];
    p.extralight = 0;
    p.fixedcolormap = 0;
    p.damagecount = 0;
    p.bonuscount = 0;
    if let Some(mo) = p.mo {
        d.p_tick.mobj_mut(mo).flags &= !crate::p_mobj::MF_SHADOW;
    }
}

/// G_PlayerReborn: resets a player and keeps the frags and the level counts.
pub fn player_reborn(d: &mut Doom, player: usize) {
    let old = std::mem::take(&mut d.g_game.players[player]);
    let p = &mut d.g_game.players[player];
    p.frags = old.frags;
    p.killcount = old.killcount;
    p.itemcount = old.itemcount;
    p.secretcount = old.secretcount;
    p.usedown = true;
    p.attackdown = true;
    p.playerstate = PST_LIVE;
    p.health = 100;
    p.readyweapon = crate::d_items::WP_PISTOL;
    p.pendingweapon = crate::d_items::WP_PISTOL;
    p.weaponowned[crate::d_items::WP_FIST as usize] = true;
    p.weaponowned[crate::d_items::WP_PISTOL as usize] = true;
    p.ammo[crate::d_items::AM_CLIP as usize] = 50;
    p.maxammo = crate::p_inter::MAXAMMO;
}

/// G_CheckSpot: true when the player can spawn at the map thing. It spawns the teleport fog.
fn check_spot(d: &mut Doom, playernum: usize, mthing: &MapThing) -> bool {
    let (x, y) = ((mthing.x as i32) << 16, (mthing.y as i32) << 16);
    let Some(body) = d.g_game.players[playernum].mo else {
        for i in 0..playernum {
            if let Some(mo) = d.g_game.players[i].mo {
                let m = d.p_tick.mobj(mo);
                if m.x == x && m.y == y {
                    return false;
                }
            }
        }
        return true;
    };
    if !crate::p_map::check_position(d, body, x, y) {
        return false;
    }
    let g = &mut d.g_game;
    if g.bodyqueslot >= BODYQUESIZE {
        if let Some(old) = g.bodyque[g.bodyqueslot % BODYQUESIZE] {
            crate::p_mobj::remove_mobj(d, old);
        }
    }
    let g = &mut d.g_game;
    g.bodyque[g.bodyqueslot % BODYQUESIZE] = Some(body);
    g.bodyqueslot += 1;
    let ss = crate::r_main::point_in_subsector(d, x, y);
    let an = 1024 * (mthing.angle as i32 / 45);
    use crate::tables::{finecosine, finesine, FINETANGENT};
    let (xa, ya) = match an {
        4096 => (FINETANGENT[2048], FINETANGENT[0]),
        5120 => (FINETANGENT[3072], FINETANGENT[1024]),
        6144 => (finesine(0), FINETANGENT[2048]),
        7168 => (finesine(1024), FINETANGENT[3072]),
        0 | 1024 | 2048 | 3072 => (finecosine(an as usize), finesine(an as usize)),
        _ => crate::i_system::error(d, &format!("G_CheckSpot: unexpected angle {an}\n")),
    };
    let sector = d.p_setup.subsectors[ss].sector;
    let z = d.p_setup.sectors[sector].floorheight;
    let mo = crate::p_mobj::spawn_mobj(
        d,
        x.wrapping_add(20 * xa),
        y.wrapping_add(20 * ya),
        z,
        crate::info::MT_TFOG,
    );
    let cp = d.g_game.consoleplayer;
    if d.g_game.players[cp].viewz != 1 {
        crate::s_sound::start_sound(d, Some(mo), crate::s_sound::SFX_TELEPT);
    }
    true
}

/// G_DeathMatchSpawnPlayer.
pub fn death_match_spawn_player(d: &mut Doom, playernum: usize) {
    let selections = d.p_setup.deathmatchstarts.len() as i32;
    if selections < 4 {
        crate::i_system::error(
            d,
            &format!("Only {selections} deathmatch spots, 4 required"),
        );
    }
    for _ in 0..20 {
        let i = (crate::m_random::p_random(&mut d.m_random) % selections) as usize;
        let spot = d.p_setup.deathmatchstarts[i];
        if check_spot(d, playernum, &spot) {
            d.p_setup.deathmatchstarts[i].type_ = playernum as i16 + 1;
            let spot = d.p_setup.deathmatchstarts[i];
            crate::p_mobj::spawn_player(d, &spot);
            return;
        }
    }
    let spot = d.p_setup.playerstarts[playernum];
    crate::p_mobj::spawn_player(d, &spot);
}

/// G_DoReborn.
fn do_reborn(d: &mut Doom, playernum: usize) {
    if !d.g_game.netgame {
        d.g_game.gameaction = GA_LOADLEVEL;
        return;
    }
    if let Some(mo) = d.g_game.players[playernum].mo {
        d.p_tick.mobj_mut(mo).player = None;
    }
    if d.g_game.deathmatch != 0 {
        death_match_spawn_player(d, playernum);
        return;
    }
    let start = d.p_setup.playerstarts[playernum];
    if check_spot(d, playernum, &start) {
        crate::p_mobj::spawn_player(d, &start);
        return;
    }
    for i in 0..MAXPLAYERS {
        let s = d.p_setup.playerstarts[i];
        if check_spot(d, playernum, &s) {
            d.p_setup.playerstarts[i].type_ = playernum as i16 + 1;
            let s = d.p_setup.playerstarts[i];
            crate::p_mobj::spawn_player(d, &s);
            d.p_setup.playerstarts[i].type_ = i as i16 + 1;
            return;
        }
    }
    crate::p_mobj::spawn_player(d, &start);
}

/// G_ScreenShot.
pub fn screen_shot(d: &mut Doom) {
    d.g_game.gameaction = GA_SCREENSHOT;
}

/// G_ExitLevel.
pub fn exit_level(d: &mut Doom) {
    d.g_game.secretexit = false;
    d.g_game.gameaction = GA_COMPLETED;
}

/// G_SecretExitLevel.
pub fn secret_exit_level(d: &mut Doom) {
    let no_map31 = d.d_main.gamemode == crate::d_main::COMMERCIAL
        && crate::w_wad::check_num_for_name(&d.w_wad, "map31") < 0;
    d.g_game.secretexit = !no_map31;
    d.g_game.gameaction = GA_COMPLETED;
}

/// G_DoCompleted: fills the intermission data and starts the intermission.
fn do_completed(d: &mut Doom) {
    d.g_game.gameaction = GA_NOTHING;
    for i in 0..MAXPLAYERS {
        if d.g_game.playeringame[i] {
            player_finish_level(d, i);
        }
    }
    if d.am_map.automapactive {
        crate::am_map::stop(d);
    }
    let commercial = d.d_main.gamemode == crate::d_main::COMMERCIAL;
    let g = &mut d.g_game;
    if !commercial {
        if d.d_main.gameversion == crate::d_main::EXE_CHEX {
            if g.gamemap == 5 {
                g.gameaction = GA_VICTORY;
                return;
            }
        } else {
            match g.gamemap {
                8 => {
                    g.gameaction = GA_VICTORY;
                    return;
                }
                9 => g.players.iter_mut().for_each(|p| p.didsecret = true),
                _ => {}
            }
        }
    }
    let w = &mut g.wminfo;
    w.didsecret = g.players[g.consoleplayer].didsecret;
    w.epsd = g.gameepisode - 1;
    w.last = g.gamemap - 1;
    if commercial {
        if g.secretexit {
            match g.gamemap {
                15 => w.next = 30,
                31 => w.next = 31,
                _ => {}
            }
        } else {
            w.next = match g.gamemap {
                31 | 32 => 15,
                m => m,
            };
        }
    } else if g.secretexit {
        w.next = 8;
    } else if g.gamemap == 9 {
        match g.gameepisode {
            1 => w.next = 3,
            2 => w.next = 5,
            3 => w.next = 6,
            4 => w.next = 2,
            _ => {}
        }
    } else {
        w.next = g.gamemap;
    }
    w.maxkills = g.totalkills;
    w.maxitems = g.totalitems;
    w.maxsecret = g.totalsecret;
    w.maxfrags = 0;
    let (e, m) = (g.gameepisode, g.gamemap);
    w.partime = if commercial {
        if m == 33 {
            // map33 reads its par time from the bytes after the cpars array (the "GAMMALVL0" string).
            35 * i32::from_le_bytes(*b"GAMM")
        } else {
            35 * CPARS[(m - 1) as usize]
        }
    } else if e < 4 {
        35 * PARS[e as usize][m as usize]
    } else {
        35 * CPARS[m as usize]
    };
    w.pnum = g.consoleplayer as i32;
    let leveltime = d.p_tick.leveltime;
    for i in 0..MAXPLAYERS {
        let p = &g.players[i];
        w.plyr[i] = WbPlayer {
            in_: g.playeringame[i],
            skills: p.killcount,
            sitems: p.itemcount,
            ssecret: p.secretcount,
            stime: leveltime,
            frags: p.frags,
            score: w.plyr[i].score,
        };
    }
    g.gamestate = GS_INTERMISSION;
    g.viewactive = false;
    d.am_map.automapactive = false;
    let wminfo = d.g_game.wminfo;
    crate::statdump::copy(d, &wminfo);
    crate::wi_stuff::start(d, &wminfo);
}

/// G_WorldDone.
pub fn world_done(d: &mut Doom) {
    let g = &mut d.g_game;
    g.gameaction = GA_WORLDDONE;
    if g.secretexit {
        g.players[g.consoleplayer].didsecret = true;
    }
    if d.d_main.gamemode == crate::d_main::COMMERCIAL {
        match g.gamemap {
            15 | 31 if g.secretexit => crate::f_finale::start_finale(d),
            6 | 11 | 20 | 30 => crate::f_finale::start_finale(d),
            _ => {}
        }
    }
}

/// G_DoWorldDone.
fn do_world_done(d: &mut Doom) {
    let g = &mut d.g_game;
    g.gamestate = GS_LEVEL;
    g.gamemap = g.wminfo.next + 1;
    do_load_level(d);
    d.g_game.gameaction = GA_NOTHING;
    d.g_game.viewactive = true;
}

/// P_SaveGameFile.
pub fn save_game_name(d: &Doom, slot: i32) -> String {
    format!("{}doomsav{}.dsg", d.g_game.savegamedir, slot)
}

/// G_LoadGame.
pub fn load_game(d: &mut Doom, name: &str) {
    d.g_game.savename = name.to_string();
    d.g_game.gameaction = GA_LOADGAME;
}

/// G_DoLoadGame.
fn do_load_game(d: &mut Doom) {
    d.g_game.gameaction = GA_NOTHING;
    let name = d.g_game.savename.clone();
    if !d.p_saveg.open_read(&name) {
        // The original reads from a NULL stream here and crashes (REQ-086).
        crate::i_video::crash(crate::i_video::SIGSEGV);
    }
    if !crate::p_saveg::read_save_game_header(d) {
        return;
    }
    let savedleveltime = d.p_tick.leveltime;
    let g = &d.g_game;
    let (sk, e, m) = (g.gameskill, g.gameepisode, g.gamemap);
    init_new(d, sk, e, m);
    d.p_tick.leveltime = savedleveltime;
    crate::p_saveg::unarchive_players(d);
    crate::p_saveg::unarchive_world(d);
    crate::p_saveg::unarchive_thinkers(d);
    crate::p_saveg::unarchive_specials(d);
    if !crate::p_saveg::read_save_game_eof(d) {
        crate::i_system::error(d, "Bad savegame");
    }
    if d.r_main.setsizeneeded {
        crate::r_main::execute_set_view_size(d);
    }
    crate::r_draw::fill_back_screen(d);
}

/// G_SaveGame: requests a save at the next tic.
pub fn save_game(d: &mut Doom, slot: i32, description: &str) {
    d.g_game.savegameslot = slot;
    d.g_game.savedescription = description.to_string();
    d.g_game.sendsave = true;
}

/// G_DoSaveGame.
fn do_save_game(d: &mut Doom) {
    let temp = format!("{}temp.dsg", d.g_game.savegamedir);
    let file = save_game_name(d, d.g_game.savegameslot);
    d.p_saveg.open_write();
    let desc = d.g_game.savedescription.clone();
    crate::p_saveg::write_save_game_header(d, &desc);
    crate::p_saveg::archive_players(d);
    crate::p_saveg::archive_world(d);
    crate::p_saveg::archive_thinkers(d);
    crate::p_saveg::archive_specials(d);
    crate::p_saveg::write_save_game_eof(d);
    if d.p_saveg.save_stream.len() > SAVEGAMESIZE {
        crate::i_system::error(d, "Savegame buffer overrun");
    }
    if !crate::m_misc::write_file(&temp, &d.p_saveg.save_stream) {
        // The original inlines M_TempFile: M_StringJoin("/tmp", "/", "recovery.dsg").
        let recovery = "/tmp/recovery.dsg";
        let msg = if crate::m_misc::write_file(recovery, &d.p_saveg.save_stream) {
            format!("Failed to open savegame file '{temp}' for writing.\nBut your game has been saved to '{recovery}' for recovery.")
        } else {
            format!("Failed to open either '{temp}' or '{recovery}' to write savegame.")
        };
        crate::i_system::error(d, &msg);
    }
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::rename(&temp, &file);
    let g = &mut d.g_game;
    g.gameaction = GA_NOTHING;
    g.savedescription.clear();
    g.players[g.consoleplayer].message = Some("game saved.".into());
    crate::r_draw::fill_back_screen(d);
}

/// G_DeferedInitNew.
pub fn defered_init_new(d: &mut Doom, skill: i32, episode: i32, map: i32) {
    let g = &mut d.g_game;
    g.d_skill = skill;
    g.d_episode = episode;
    g.d_map = map;
    g.gameaction = GA_NEWGAME;
}

/// G_DoNewGame.
fn do_new_game(d: &mut Doom) {
    let g = &mut d.g_game;
    g.demoplayback = false;
    g.netdemo = false;
    g.netgame = false;
    g.deathmatch = 0;
    g.playeringame[1..].iter_mut().for_each(|b| *b = false);
    g.set_parms(0, 0, 0);
    g.consoleplayer = 0;
    let (sk, e, m) = (g.d_skill, g.d_episode, g.d_map);
    init_new(d, sk, e, m);
    d.g_game.gameaction = GA_NOTHING;
}

/// G_InitNew.
pub fn init_new(d: &mut Doom, skill: i32, episode: i32, map: i32) {
    use crate::d_main::{COMMERCIAL, EXE_ULTIMATE, SHAREWARE};
    if d.g_game.paused {
        d.g_game.paused = false;
        crate::s_sound::resume_sound(d);
    }
    let skill = skill.min(SK_NIGHTMARE);
    let mut episode = episode;
    if d.d_main.gameversion >= EXE_ULTIMATE {
        if episode == 0 {
            episode = 4;
        }
    } else {
        episode = episode.clamp(1, 3);
    }
    if episode > 1 && d.d_main.gamemode == SHAREWARE {
        episode = 1;
    }
    let mut map = map.max(1);
    if map > 9 && d.d_main.gamemode != COMMERCIAL {
        map = 9;
    }
    crate::m_random::clear_random(&mut d.m_random);
    let g = &mut d.g_game;
    g.respawnmonsters = skill == SK_NIGHTMARE || g.respawnparm;
    if g.fastparm || (skill == SK_NIGHTMARE && g.gameskill != SK_NIGHTMARE) {
        g.sarg_tics.iter_mut().for_each(|t| *t >>= 1);
        g.missile_speeds = [20 << 16; 3];
    } else if skill != SK_NIGHTMARE && g.gameskill == SK_NIGHTMARE {
        g.sarg_tics.iter_mut().for_each(|t| *t <<= 1);
        g.missile_speeds = [15 << 16, 10 << 16, 10 << 16];
    }
    for p in g.players.iter_mut() {
        p.playerstate = PST_REBORN;
    }
    g.usergame = true;
    g.paused = false;
    g.demoplayback = false;
    d.am_map.automapactive = false;
    g.viewactive = true;
    g.gameepisode = episode;
    g.gamemap = map;
    g.gameskill = skill;
    let name = if d.d_main.gamemode == COMMERCIAL {
        if map < 12 {
            "SKY1"
        } else if map < 21 {
            "SKY2"
        } else {
            "SKY3"
        }
    } else {
        match episode {
            2 => "SKY2",
            3 => "SKY3",
            4 => "SKY4",
            _ => "SKY1",
        }
    };
    d.r_sky.skytexture = crate::r_data::texture_num_for_name(d, name);
    do_load_level(d);
}

/// G_RecordDemo.
pub fn record_demo(d: &mut Doom, name: &str) {
    let g = &mut d.g_game;
    g.usergame = false;
    g.demoname = format!("{name}.lmp");
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-maxdemo", 1);
    let maxsize = if p > 0 {
        crate::z_zone::atoi(&d.m_argv.myargv[p + 1]).wrapping_mul(1024)
    } else {
        0x20000
    };
    let g = &mut d.g_game;
    g.demobuffer = vec![0; maxsize.max(0) as usize];
    g.demoend = maxsize.max(0) as usize;
    g.demorecording = true;
}

/// G_VanillaVersionCode. Doom 1.2 has no code; the callers that need it report the error.
pub fn vanilla_version_code(d: &Doom) -> i32 {
    use crate::d_main::*;
    match d.d_main.gameversion {
        EXE_DOOM_1_666 => 106,
        EXE_DOOM_1_7 => 107,
        EXE_DOOM_1_8 => 108,
        _ => 109,
    }
}

fn checked_version_code(d: &mut Doom) -> i32 {
    if d.d_main.gameversion == crate::d_main::EXE_DOOM_1_2 {
        crate::i_system::error(d, "Doom 1.2 does not have a version code!");
    }
    vanilla_version_code(d)
}

/// G_BeginRecording: writes the 13-byte demo header.
pub fn begin_recording(d: &mut Doom) {
    let longtics = crate::m_argv::check_parm(&d.m_argv, "-longtics") > 0;
    let version = if longtics {
        DOOM_191_VERSION
    } else {
        checked_version_code(d) as u8
    };
    let g = &mut d.g_game;
    g.longtics = longtics;
    g.lowres_turn = !longtics;
    let mut header = vec![
        version,
        g.gameskill as u8,
        g.gameepisode as u8,
        g.gamemap as u8,
        g.deathmatch as u8,
        g.parm_values[1] as u8,
        g.parm_values[2] as u8,
        g.parm_values[0] as u8,
        g.consoleplayer as u8,
    ];
    header.extend(g.playeringame.iter().map(|&b| b as u8));
    for (i, b) in header.into_iter().enumerate() {
        g.poke(i, b);
    }
    g.demo_p = 13;
}

/// G_DeferedPlayDemo.
pub fn defered_play_demo(d: &mut Doom, name: &str) {
    d.g_game.defdemoname = name.to_string();
    d.g_game.gameaction = GA_PLAYDEMO;
}

/// DemoVersionDescription.
fn demo_version_description(version: i32) -> String {
    match version {
        104 => "v1.4".into(),
        105 => "v1.5".into(),
        106 => "v1.6/v1.666".into(),
        107 => "v1.7/v1.7a".into(),
        108 => "v1.8".into(),
        109 => "v1.9".into(),
        0..=4 => "v1.0/v1.1/v1.2".into(),
        v => format!("{}.{} (unknown)", v / 100, v % 100),
    }
}

/// G_DoPlayDemo: reads the demo header and starts the demo game.
fn do_play_demo(d: &mut Doom) {
    let name = d.g_game.defdemoname.clone();
    let lump = crate::w_wad::get_num_for_name(d, &name);
    d.g_game.gameaction = GA_NOTHING;
    let buf = crate::w_wad::lump(&d.w_wad, lump).to_vec();
    let at = |i: usize| buf.get(i).copied().unwrap_or(0);
    let demoversion = at(0) as i32;
    let code = checked_version_code(d);
    if demoversion == code {
        d.g_game.longtics = false;
    } else if demoversion == DOOM_191_VERSION as i32 {
        d.g_game.longtics = true;
    } else {
        print!(
            "Demo is from a different game version!\n(read {}, should be {})\n\n*** You may need to upgrade your version of Doom to v1.9. ***\n    See: https://www.doomworld.com/classicdoom/info/patches.php\n    This appears to be {}.",
            demoversion,
            code,
            demo_version_description(demoversion)
        );
    }
    let (skill, episode, map) = (at(1) as i32, at(2) as i32, at(3) as i32);
    let g = &mut d.g_game;
    g.deathmatch = at(4) as i32;
    g.set_parms(at(7) as i32, at(5) as i32, at(6) as i32);
    g.consoleplayer = at(8) as usize;
    for i in 0..MAXPLAYERS {
        g.playeringame[i] = at(9 + i) != 0;
    }
    g.demobuffer = buf;
    g.demo_p = 13;
    let a = &d.m_argv;
    if g.playeringame[1]
        || crate::m_argv::check_parm(a, "-solo-net") > 0
        || crate::m_argv::check_parm(a, "-netdemo") > 0
    {
        g.netgame = true;
        g.netdemo = true;
    }
    g.precache = false;
    init_new(d, skill, episode, map);
    let g = &mut d.g_game;
    g.precache = true;
    g.starttime = crate::i_system::get_time();
    g.usergame = false;
    g.demoplayback = true;
}

/// G_TimeDemo.
pub fn time_demo(d: &mut Doom, name: &str) {
    d.d_main.nodrawers = crate::m_argv::check_parm(&d.m_argv, "-nodraw") > 0;
    d.g_game.timingdemo = true;
    d.d_loop.singletics = true;
    d.g_game.defdemoname = name.to_string();
    d.g_game.gameaction = GA_PLAYDEMO;
}

/// G_CheckDemoStatus: ends a timed demo, a played demo, or a recording.
pub fn check_demo_status(d: &mut Doom) -> bool {
    let g = &mut d.g_game;
    if g.timingdemo {
        let realtics = crate::i_system::get_time() - g.starttime;
        let fps = g.gametic as f32 * 35.0 / realtics as f32;
        g.timingdemo = false;
        g.demoplayback = false;
        let fps = if fps.is_nan() {
            "nan".to_string()
        } else {
            format!("{:.6}", fps as f64)
        };
        let msg = format!(
            "timed {} gametics in {} realtics ({} fps)",
            g.gametic, realtics, fps
        );
        crate::i_system::error(d, &msg);
    }
    if g.demoplayback {
        // W_ReleaseLumpName(defdemoname).
        let name = g.defdemoname.clone();
        crate::w_wad::get_num_for_name(d, &name);
        let g = &mut d.g_game;
        g.demoplayback = false;
        g.netdemo = false;
        g.netgame = false;
        g.deathmatch = 0;
        g.playeringame[1..].iter_mut().for_each(|b| *b = false);
        g.set_parms(0, 0, 0);
        g.consoleplayer = 0;
        if g.singledemo {
            // I_Quit of this build runs the exit functions; D_Endoom ends the process.
            crate::i_system::run_exit_funcs(d, false);
        } else {
            crate::d_main::advance_demo(d);
        }
        return true;
    }
    if g.demorecording {
        let p = g.demo_p;
        if p == DEMO_P_NULL {
            crate::i_video::crash(crate::i_video::SIGSEGV);
        }
        g.poke(p, DEMOMARKER);
        crate::m_misc::write_file(&g.demoname, &g.demobuffer[..p + 1]);
        g.demorecording = false;
        let msg = format!("Demo {} recorded", g.demoname);
        crate::i_system::error(d, &msg);
    }
    false
}

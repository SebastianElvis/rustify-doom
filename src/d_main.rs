//! Startup, the main loop, the display, and the attract sequence.

use crate::g_game::{GS_DEMOSCREEN, GS_FINALE, GS_INTERMISSION, GS_LEVEL};
use crate::i_system::{at_exit, ExitFunc};
use crate::m_argv::{check_parm, check_parm_with_args};
use crate::w_wad::check_num_for_name;
#[allow(unused_imports)]
use crate::z_zone::PU_STATIC;
use crate::Doom;

pub const SHAREWARE: i32 = 0;
pub const REGISTERED: i32 = 1;
pub const COMMERCIAL: i32 = 2;
pub const RETAIL: i32 = 3;
pub const INDETERMINED: i32 = 4;

pub const MISSION_DOOM: i32 = 0;
pub const MISSION_DOOM2: i32 = 1;
pub const MISSION_PACK_TNT: i32 = 2;
pub const MISSION_PACK_PLUT: i32 = 3;
pub const MISSION_PACK_CHEX: i32 = 4;
pub const MISSION_PACK_HACX: i32 = 5;
pub const MISSION_NONE: i32 = 6;

pub const EXE_DOOM_1_2: i32 = 0;
pub const EXE_DOOM_1_666: i32 = 1;
pub const EXE_DOOM_1_7: i32 = 2;
pub const EXE_DOOM_1_8: i32 = 3;
pub const EXE_DOOM_1_9: i32 = 4;
pub const EXE_HACX: i32 = 5;
pub const EXE_ULTIMATE: i32 = 6;
pub const EXE_FINAL: i32 = 7;
pub const EXE_FINAL2: i32 = 8;
pub const EXE_CHEX: i32 = 9;

pub const EV_KEYDOWN: i32 = 0;
pub const EV_KEYUP: i32 = 1;
pub const EV_MOUSE: i32 = 2;
pub const EV_JOYSTICK: i32 = 3;
pub const EV_QUIT: i32 = 4;

pub const VARIANT_VANILLA: i32 = 0;
pub const VARIANT_FREEDOOM: i32 = 1;
pub const VARIANT_FREEDM: i32 = 2;
pub const VARIANT_BFG: i32 = 3;

const MAXEVENTS: usize = 64;

/// The supported -gameversion values: description, command-line name, and version.
const GAMEVERSIONS: [(&str, &str, i32); 9] = [
    ("Doom 1.666", "1.666", EXE_DOOM_1_666),
    ("Doom 1.7/1.7a", "1.7", EXE_DOOM_1_7),
    ("Doom 1.8", "1.8", EXE_DOOM_1_8),
    ("Doom 1.9", "1.9", EXE_DOOM_1_9),
    ("Hacx", "hacx", EXE_HACX),
    ("Ultimate Doom", "ultimate", EXE_ULTIMATE),
    ("Final Doom", "final", EXE_FINAL),
    ("Final Doom (alt)", "final2", EXE_FINAL2),
    ("Chex Quest", "chex", EXE_CHEX),
];

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Event {
    pub type_: i32,
    pub data1: i32,
    pub data2: i32,
    pub data3: i32,
    pub data4: i32,
}

/// The startup and main-loop globals.
#[derive(Default)]
pub struct State {
    pub gamemode: i32,
    pub gamemission: i32,
    pub gameversion: i32,
    pub gamevariant: i32,
    pub gamedescription: String,
    pub devparm: bool,
    pub modifiedgame: bool,
    pub nodrawers: bool,
    pub advancedemo: bool,
    pub demosequence: i32,
    pub pagetic: i32,
    pub pagename: String,
    pub wipegamestate: i32,
    /// The palette at the last zone model PLAYPAL cache.
    pub zpal: Vec<u32>,
    pub events: Vec<Event>,
    pub eventhead: usize,
    pub eventtail: usize,
    pub storedemo: bool,
    pub main_loop_started: bool,
    pub screenvisible: bool,
    pub autostart: bool,
    pub startskill: i32,
    pub startepisode: i32,
    pub startmap: i32,
    pub startloadgame: i32,
    pub timelimit: i32,
    pub demolumpname: String,
    // D_Display statics.
    viewactivestate: bool,
    menuactivestate: bool,
    inhelpscreensstate: bool,
    fullscreen: bool,
    oldgamestate: i32,
    borderdrawcount: i32,
}

/// I_PrintBanner: centers the text at column 35.
fn print_banner(msg: &str) {
    let spaces = 35 - (msg.len() as i32 / 2);
    println!("{}{}", " ".repeat(spaces.max(0) as usize), msg);
}

/// I_PrintDivider.
fn print_divider() {
    println!("{}", "=".repeat(75));
}

/// D_PostEvent.
pub fn post_event(d: &mut Doom, ev: Event) {
    let s = &mut d.d_main;
    if s.events.len() < MAXEVENTS {
        s.events.resize(MAXEVENTS, Event::default());
    }
    s.events[s.eventhead] = ev;
    s.eventhead = (s.eventhead + 1) % MAXEVENTS;
}

/// D_PopEvent.
fn pop_event(d: &mut Doom) -> Option<Event> {
    let s = &mut d.d_main;
    if s.eventtail == s.eventhead {
        return None;
    }
    let ev = s.events[s.eventtail];
    s.eventtail = (s.eventtail + 1) % MAXEVENTS;
    Some(ev)
}

/// D_ProcessEvents: the menu sees each event first, then the game.
pub fn process_events(d: &mut Doom) {
    if d.d_main.storedemo {
        return;
    }
    while let Some(ev) = pop_event(d) {
        if crate::m_menu::responder(d, &ev) {
            continue;
        }
        crate::g_game::responder(d, &ev);
    }
}

/// D_Display: draws the current game state, then runs the screen melt when the state changed.
pub fn display(d: &mut Doom) {
    use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
    if d.d_main.nodrawers {
        return;
    }
    crate::r_data::zone_sync(d);
    let mut redrawsbar = false;
    if d.r_main.setsizeneeded {
        crate::r_main::execute_set_view_size(d);
        d.d_main.oldgamestate = -1;
        d.d_main.borderdrawcount = 3;
    }
    let gamestate = d.g_game.gamestate;
    let gametic = d.g_game.gametic;
    let wipe = gamestate != d.d_main.wipegamestate;
    if wipe {
        crate::f_wipe::start_screen(d, 0, 0, SCREENWIDTH as i32, SCREENHEIGHT as i32);
    }
    if gamestate == GS_LEVEL && gametic != 0 {
        crate::hu_stuff::erase(d);
    }
    match gamestate {
        GS_LEVEL if gametic != 0 => {
            if d.am_map.automapactive {
                crate::am_map::drawer(d);
            }
            let viewheight = d.r_draw.viewheight;
            if wipe || (viewheight != 200 && d.d_main.fullscreen) {
                redrawsbar = true;
            }
            if d.d_main.inhelpscreensstate && !d.m_menu.inhelpscreens {
                redrawsbar = true;
            }
            crate::st_stuff::drawer(d, viewheight == 200, redrawsbar);
            // ST_doPaletteStuff caches PLAYPAL with PU_CACHE for each palette change.
            if d.i_video.colors[..] != d.d_main.zpal[..] {
                d.d_main.zpal = d.i_video.colors.to_vec();
                let lump = crate::w_wad::get_num_for_name(d, "PLAYPAL");
                d.z_zone.cache_lump(&d.w_wad, lump, crate::z_zone::PU_CACHE);
            }
            d.d_main.fullscreen = viewheight == 200;
        }
        GS_INTERMISSION => crate::wi_stuff::drawer(d),
        GS_FINALE => crate::f_finale::drawer(d),
        GS_DEMOSCREEN => page_drawer(d),
        _ => {}
    }
    if gamestate == GS_LEVEL && !d.am_map.automapactive && gametic != 0 {
        let p = d.g_game.displayplayer;
        crate::r_main::render_player_view(d, p);
    }
    if gamestate == GS_LEVEL && gametic != 0 {
        crate::hu_stuff::drawer(d);
    }
    if gamestate != d.d_main.oldgamestate && gamestate != GS_LEVEL {
        let lump = crate::w_wad::get_num_for_name(d, "PLAYPAL");
        d.z_zone.cache_lump(&d.w_wad, lump, crate::z_zone::PU_CACHE);
        let pal = crate::w_wad::lump(&d.w_wad, lump)[..768].to_vec();
        crate::i_video::set_palette(&mut d.i_video, &pal);
        d.d_main.zpal = d.i_video.colors.to_vec();
    }
    if gamestate == GS_LEVEL && d.d_main.oldgamestate != GS_LEVEL {
        d.d_main.viewactivestate = false;
        crate::r_draw::fill_back_screen(d);
    }
    if gamestate == GS_LEVEL && !d.am_map.automapactive && d.r_draw.scaledviewwidth != 320 {
        if d.m_menu.menuactive || d.d_main.menuactivestate || !d.d_main.viewactivestate {
            d.d_main.borderdrawcount = 3;
        }
        if d.d_main.borderdrawcount != 0 {
            crate::r_draw::draw_view_border(d);
            d.d_main.borderdrawcount -= 1;
        }
    }
    if d.m_menu.testcontrols {
        crate::v_video::draw_mouse_speed_box(d, 0);
    }
    d.d_main.menuactivestate = d.m_menu.menuactive;
    d.d_main.viewactivestate = d.g_game.viewactive;
    d.d_main.inhelpscreensstate = d.m_menu.inhelpscreens;
    d.d_main.oldgamestate = gamestate;
    d.d_main.wipegamestate = gamestate;
    if d.g_game.paused {
        let y = if d.am_map.automapactive {
            4
        } else {
            d.r_draw.viewwindowy + 4
        };
        let x = d.r_draw.viewwindowx + (d.r_draw.scaledviewwidth - 68) / 2;
        let lump = crate::w_wad::get_num_for_name(d, "M_PAUSE");
        crate::v_video::draw_patch_direct(d, x, y, lump);
    }
    crate::m_menu::drawer(d);
    crate::d_loop::net_update(d);
    if !wipe {
        crate::i_video::finish_update(&mut d.i_video);
        return;
    }
    crate::f_wipe::end_screen(d, 0, 0, SCREENWIDTH as i32, SCREENHEIGHT as i32);
    let mut wipestart = crate::i_system::get_time() - 1;
    loop {
        let (mut nowtime, mut tics);
        loop {
            nowtime = crate::i_system::get_time();
            tics = nowtime - wipestart;
            crate::i_system::sleep(1);
            if tics > 0 {
                break;
            }
        }
        wipestart = nowtime;
        let done =
            crate::f_wipe::screen_wipe(d, 0, 0, 0, SCREENWIDTH as i32, SCREENHEIGHT as i32, tics);
        crate::m_menu::drawer(d);
        crate::i_video::finish_update(&mut d.i_video);
        if done {
            break;
        }
    }
}

/// D_PageTicker.
pub fn page_ticker(d: &mut Doom) {
    d.d_main.pagetic -= 1;
    if d.d_main.pagetic < 0 {
        advance_demo(d);
    }
}

/// D_PageDrawer.
pub fn page_drawer(d: &mut Doom) {
    let name = d.d_main.pagename.clone();
    let lump = crate::w_wad::get_num_for_name(d, &name);
    crate::v_video::draw_patch(d, 0, 0, lump);
}

/// D_AdvanceDemo: requests the next page or demo of the attract sequence.
pub fn advance_demo(d: &mut Doom) {
    d.d_main.advancedemo = true;
}

/// D_DoAdvanceDemo.
pub fn do_advance_demo(d: &mut Doom) {
    let cp = d.g_game.consoleplayer;
    d.g_game.players[cp].playerstate = crate::g_game::PST_LIVE;
    d.d_main.advancedemo = false;
    d.g_game.usergame = false;
    d.g_game.paused = false;
    d.g_game.gameaction = crate::g_game::GA_NOTHING;
    let s = &mut d.d_main;
    let commercial = s.gamemode == COMMERCIAL;
    s.demosequence = if s.gameversion == EXE_ULTIMATE || s.gameversion == EXE_FINAL {
        (s.demosequence + 1) % 7
    } else {
        (s.demosequence + 1) % 6
    };
    match s.demosequence {
        0 => {
            s.pagetic = if commercial { 35 * 11 } else { 170 };
            d.g_game.gamestate = GS_DEMOSCREEN;
            s.pagename = "TITLEPIC".into();
            crate::s_sound::start_music(
                d,
                if commercial {
                    crate::s_sound::MUS_DM2TTL
                } else {
                    crate::s_sound::MUS_INTRO
                },
            );
        }
        1 => crate::g_game::defered_play_demo(d, "demo1"),
        2 => {
            s.pagetic = 200;
            d.g_game.gamestate = GS_DEMOSCREEN;
            s.pagename = "CREDIT".into();
        }
        3 => crate::g_game::defered_play_demo(d, "demo2"),
        4 => {
            d.g_game.gamestate = GS_DEMOSCREEN;
            if commercial {
                s.pagetic = 35 * 11;
                s.pagename = "TITLEPIC".into();
                crate::s_sound::start_music(d, crate::s_sound::MUS_DM2TTL);
            } else {
                s.pagetic = 200;
                s.pagename = if s.gameversion >= EXE_ULTIMATE {
                    "CREDIT"
                } else {
                    "HELP2"
                }
                .into();
            }
        }
        5 => crate::g_game::defered_play_demo(d, "demo3"),
        6 => crate::g_game::defered_play_demo(d, "demo4"),
        _ => {}
    }
    let s = &d.d_main;
    if s.gamevariant == VARIANT_BFG
        && s.pagename.eq_ignore_ascii_case("TITLEPIC")
        && check_num_for_name(&d.w_wad, "titlepic") < 0
    {
        d.d_main.pagename = "INTERPIC".into();
    }
}

/// D_StartTitle.
pub fn start_title(d: &mut Doom) {
    d.g_game.gameaction = crate::g_game::GA_NOTHING;
    d.d_main.demosequence = -1;
    advance_demo(d);
}

/// D_Endoom: exits with status 0 once the main loop has started.
pub fn endoom(d: &mut Doom) {
    if !d.d_main.main_loop_started || check_parm(&d.m_argv, "-testcontrols") > 0 {
        return;
    }
    crate::w_wad::get_num_for_name(d, "ENDOOM");
    std::process::exit(0);
}

/// D_AddFile.
fn add_file(d: &mut Doom, name: &str) -> bool {
    println!(" adding {name}");
    crate::w_wad::add_file(d, name)
}

fn arg(d: &Doom, i: usize) -> String {
    d.m_argv.myargv[i].clone()
}

fn first_char(d: &Doom, i: usize) -> i32 {
    d.m_argv.myargv[i]
        .bytes()
        .next()
        .map_or(0, |b| b as i8 as i32)
}

/// D_IdentifyVersion.
fn identify_version(d: &mut Doom) {
    if d.d_main.gamemission == MISSION_NONE {
        for l in d.w_wad.lumps.iter() {
            if &l.name[..6] == b"MAP01\0" {
                d.d_main.gamemission = MISSION_DOOM2;
                break;
            } else if &l.name[..5] == b"E1M1\0" {
                d.d_main.gamemission = MISSION_DOOM;
                break;
            }
        }
        if d.d_main.gamemission == MISSION_NONE {
            crate::i_system::error(d, "Unknown or invalid IWAD file.");
        }
    }
    let m = d.d_main.gamemission;
    if m == MISSION_DOOM || m == MISSION_PACK_CHEX {
        d.d_main.gamemode = if check_num_for_name(&d.w_wad, "E4M1") > 0 {
            RETAIL
        } else if check_num_for_name(&d.w_wad, "E3M1") > 0 {
            REGISTERED
        } else {
            SHAREWARE
        };
    } else {
        d.d_main.gamemode = COMMERCIAL;
        let p = check_parm_with_args(&d.m_argv, "-pack", 1);
        if p > 0 {
            let name = arg(d, p + 1);
            match name.to_ascii_lowercase().as_str() {
                "doom2" => d.d_main.gamemission = MISSION_DOOM2,
                "tnt" => d.d_main.gamemission = MISSION_PACK_TNT,
                "plutonia" => d.d_main.gamemission = MISSION_PACK_PLUT,
                _ => {
                    println!("Valid mission packs are:");
                    for n in ["doom2", "tnt", "plutonia"] {
                        println!("\t{n}");
                    }
                    crate::i_system::error(d, &format!("Unknown mission pack name: {name}"));
                }
            }
        }
    }
}

/// InitGameVersion.
fn init_game_version(d: &mut Doom) {
    let p = check_parm_with_args(&d.m_argv, "-gameversion", 1);
    let s = &d.d_main;
    if p > 0 {
        let v = arg(d, p + 1);
        if let Some(&(_, _, version)) = GAMEVERSIONS.iter().find(|g| g.1 == v) {
            d.d_main.gameversion = version;
        } else {
            println!("Supported game versions:");
            for (desc, cmd, _) in GAMEVERSIONS {
                println!("\t{cmd} ({desc})");
            }
            crate::i_system::error(d, &format!("Unknown game version '{v}'"));
        }
    } else if s.gamemission == MISSION_PACK_CHEX {
        d.d_main.gameversion = EXE_CHEX;
    } else if s.gamemission == MISSION_PACK_HACX {
        d.d_main.gameversion = EXE_HACX;
    } else if s.gamemode == SHAREWARE
        || s.gamemode == REGISTERED
        || (s.gamemode == COMMERCIAL && s.gamemission == MISSION_DOOM2)
    {
        d.d_main.gameversion = EXE_DOOM_1_9;
        for i in 1..=3 {
            let n = check_num_for_name(&d.w_wad, &format!("demo{i}"));
            if n > 0 {
                let v = match crate::w_wad::lump(&d.w_wad, n as usize)
                    .first()
                    .copied()
                    .unwrap_or(255)
                {
                    0..=4 => EXE_DOOM_1_2,
                    106 => EXE_DOOM_1_666,
                    107 => EXE_DOOM_1_7,
                    108 => EXE_DOOM_1_8,
                    109 => EXE_DOOM_1_9,
                    _ => continue,
                };
                d.d_main.gameversion = v;
                break;
            }
        }
    } else if s.gamemode == RETAIL {
        d.d_main.gameversion = EXE_ULTIMATE;
    } else if s.gamemode == COMMERCIAL {
        d.d_main.gameversion = EXE_FINAL;
    }
    let s = &mut d.d_main;
    if s.gameversion <= EXE_DOOM_1_2 && d.g_game.deathmatch == 2 {
        d.g_game.deathmatch = 1;
    }
    if s.gameversion < EXE_ULTIMATE && s.gamemode == RETAIL {
        s.gamemode = REGISTERED;
    }
    if s.gameversion < EXE_FINAL
        && s.gamemode == COMMERCIAL
        && (s.gamemission == MISSION_PACK_TNT || s.gamemission == MISSION_PACK_PLUT)
    {
        s.gamemission = MISSION_DOOM2;
    }
}

/// D_SetGameDescription.
fn set_game_description(d: &mut Doom) {
    let s = &d.d_main;
    let logical = crate::hu_stuff::logical_gamemission(d);
    let desc = if logical == MISSION_DOOM {
        if s.gamevariant == VARIANT_FREEDOOM {
            "Freedoom: Phase 1"
        } else if s.gamemode == RETAIL {
            "The Ultimate DOOM"
        } else if s.gamemode == REGISTERED {
            "DOOM Registered"
        } else if s.gamemode == SHAREWARE {
            "DOOM Shareware"
        } else {
            "Unknown"
        }
    } else if s.gamevariant == VARIANT_FREEDM {
        "FreeDM"
    } else if s.gamevariant == VARIANT_FREEDOOM {
        "Freedoom: Phase 2"
    } else if logical == MISSION_DOOM2 {
        "DOOM 2: Hell on Earth"
    } else if logical == MISSION_PACK_PLUT {
        "DOOM 2: Plutonia Experiment"
    } else if logical == MISSION_PACK_TNT {
        "DOOM 2: TNT - Evilution"
    } else {
        "Unknown"
    };
    d.d_main.gamedescription = desc.to_string();
}

/// D_CheckNetGame with the single-node settings.
fn check_net_game(d: &mut Doom) {
    if d.g_game.netgame {
        d.d_main.autostart = true;
    }
    let record = check_parm(&d.m_argv, "-record") > 0;
    let longtics = check_parm(&d.m_argv, "-longtics") > 0;
    d.g_game.lowres_turn = record && !longtics;
    d.d_loop.old_sync = true;
    d.g_game.consoleplayer = 0;
    if d.g_game.lowres_turn {
        println!("NOTE: Turning resolution is reduced; this is probably because there is a client recording a Vanilla demo.");
    }
    d.g_game.playeringame = [true, false, false, false];
    d.p_spec.timelimit = d.d_main.timelimit;
    let s = &d.d_main;
    println!(
        "startskill {}  deathmatch: {}  startmap: {}  startepisode: {}",
        s.startskill, d.g_game.deathmatch, s.startmap, s.startepisode
    );
    println!(
        "player {} of {} ({} nodes)",
        d.g_game.consoleplayer + 1,
        1,
        1
    );
    let t = s.timelimit;
    if t > 0 && d.g_game.deathmatch != 0 {
        if t == 20 && check_parm(&d.m_argv, "-avg") > 0 {
            println!("Austin Virtual Gaming: Levels will end after 20 minutes");
        } else {
            println!(
                "Levels will end after {} minute{}.",
                t,
                if t > 1 { "s" } else { "" }
            );
        }
    }
}

/// D_DoomMain: the startup sequence. It ends with the first main-loop iteration.
pub fn doom_main(d: &mut Doom) {
    d.d_main.oldgamestate = -1;
    d.d_main.wipegamestate = GS_DEMOSCREEN;
    d.g_game.precache = true;
    at_exit(d, ExitFunc::Endoom, false);
    print_banner("Doom Generic 0.1");
    println!("Z_Init: Init zone memory allocation daemon. ");
    crate::z_zone::init(d);
    let a = &d.m_argv;
    let (nomonsters, respawn, fast) = (
        check_parm(a, "-nomonsters"),
        check_parm(a, "-respawn"),
        check_parm(a, "-fast"),
    );
    d.g_game
        .set_parms(nomonsters as i32, respawn as i32, fast as i32);
    d.d_main.devparm = check_parm(a, "-devparm") > 0;
    if check_parm(a, "-deathmatch") > 0 {
        d.g_game.deathmatch = 1;
    }
    if check_parm(&d.m_argv, "-altdeath") > 0 {
        d.g_game.deathmatch = 2;
    }
    if d.d_main.devparm {
        println!("Development mode ON.");
    }
    crate::m_config::set_config_dir(d);
    let p = check_parm(&d.m_argv, "-turbo");
    if p > 0 {
        let mut scale = 200;
        if p < d.m_argv.myargv.len() - 1 {
            scale = crate::z_zone::atoi(&arg(d, p + 1));
        }
        scale = scale.clamp(10, 400);
        println!("turbo scale: {scale}%");
        d.g_game.set_turbo(scale);
    }
    println!("V_Init: allocate screens.");
    println!("M_LoadDefaults: Load system defaults.");
    crate::m_config::load_defaults(d);
    at_exit(d, ExitFunc::SaveDefaults, false);
    let Some((iwadfile, mission)) = crate::d_iwad::find_iwad(d) else {
        crate::i_system::error(
            d,
            "Game mode indeterminate.  No IWAD file was found.  Try\nspecifying one with the '-iwad' command line parameter.\n",
        );
    };
    d.d_main.gamemission = mission;
    d.d_main.modifiedgame = false;
    println!("W_Init: Init WADfiles.");
    add_file(d, &iwadfile);
    for (lump, game) in [
        ("IMPXA1", "Heretic"),
        ("ETTNA1", "Hexen"),
        ("AGRDA1", "Strife"),
    ] {
        if check_num_for_name(&d.w_wad, lump) >= 0 {
            let bin = game.to_ascii_lowercase();
            crate::i_system::error(
                d,
                &format!("\nYou are trying to use a {game} IWAD file with the doomgenericdoom binary.\nThis isn't going to work.\nYou probably want to use the doomgeneric{bin} binary."),
            );
        }
    }
    identify_version(d);
    init_game_version(d);
    if check_num_for_name(&d.w_wad, "FREEDOOM") >= 0 {
        d.d_main.gamevariant = if check_num_for_name(&d.w_wad, "FREEDM") >= 0 {
            VARIANT_FREEDM
        } else {
            VARIANT_FREEDOOM
        };
    } else if check_num_for_name(&d.w_wad, "DMENUPIC") >= 0 {
        d.d_main.gamevariant = VARIANT_BFG;
    }
    if d.d_main.gamevariant == VARIANT_BFG {
        println!("BFG Edition: Using workarounds as needed.");
    }
    // W_ParseCommandLine.
    let p = check_parm_with_args(&d.m_argv, "-file", 1);
    if p > 0 {
        d.d_main.modifiedgame = true;
        let mut i = p + 1;
        while i < d.m_argv.myargv.len() && !d.m_argv.myargv[i].starts_with('-') {
            let f = crate::d_iwad::try_find_wad_by_name(&arg(d, i));
            println!(" adding {f}");
            crate::w_wad::add_file(d, &f);
            i += 1;
        }
    }
    let mut p = check_parm_with_args(&d.m_argv, "-playdemo", 1);
    if p == 0 {
        p = check_parm_with_args(&d.m_argv, "-timedemo", 1);
    }
    if p > 0 {
        let name = arg(d, p + 1);
        let file = if name.ends_with(".lmp") {
            name.clone()
        } else {
            format!("{name}.lmp")
        };
        d.d_main.demolumpname = if add_file(d, &file) {
            let n = d.w_wad.lumps.len() - 1;
            crate::w_wad::lump_name(&d.w_wad, n)
        } else {
            name
        };
        println!("Playing demo {file}.");
    }
    // W_GenerateHashTable.
    let n = d.w_wad.lumps.len() as i32;
    d.z_zone.malloc(8 * n, PU_STATIC, 0);
    at_exit(d, ExitFunc::CheckDemoStatus, true);
    set_game_description(d);
    d.g_game.savegamedir = crate::m_config::get_save_game_dir(d);
    if d.d_main.modifiedgame {
        if d.d_main.gamemode == SHAREWARE {
            crate::i_system::error(
                d,
                "\nYou cannot -file with the shareware version. Register!",
            );
        }
        if d.d_main.gamemode == REGISTERED {
            let names = (2..=3).flat_map(|e| (1..=9).map(move |m| format!("e{e}m{m}")));
            let others = ["dphoof", "bfgga0", "heada1", "cybra1", "spida1d1"].map(String::from);
            for n in names.chain(others) {
                if check_num_for_name(&d.w_wad, &n) < 0 {
                    crate::i_system::error(d, "\nThis is not the registered version.");
                }
            }
        }
    }
    if check_num_for_name(&d.w_wad, "SS_START") >= 0 || check_num_for_name(&d.w_wad, "FF_END") >= 0
    {
        print_divider();
        println!(" WARNING: The loaded WAD file contains modified sprites or\n floor textures.  You may want to use the '-merge' command\n line option instead of '-file'.");
    }
    print_divider();
    print_banner(&d.d_main.gamedescription.clone());
    print_divider();
    println!(" Doom Generic is free software, covered by the GNU General Public\n License.  There is NO warranty; not even for MERCHANTABILITY or FITNESS\n FOR A PARTICULAR PURPOSE. You are welcome to change and distribute\n copies under certain conditions. See the source for more information.");
    print_divider();
    if d.d_main.gamevariant == VARIANT_FREEDOOM {
        println!(" WARNING: You are playing using one of the Freedoom IWAD\n files, which might not work in this port. See this page\n for more information on how to play using Freedoom:\n   http://www.chocolate-doom.org/wiki/index.php/Freedoom");
        print_divider();
    }
    println!("I_Init: Setting up machine state.");
    if check_parm(&d.m_argv, "-left") > 0 {
        d.r_main.viewangleoffset = 0x4000_0000;
    }
    if check_parm(&d.m_argv, "-right") > 0 {
        d.r_main.viewangleoffset = -0x4000_0000;
    }
    at_exit(d, ExitFunc::QuitNetGame, true);
    d.g_game.netgame = check_parm(&d.m_argv, "-solo-net") > 0;
    let s = &mut d.d_main;
    s.startskill = crate::g_game::SK_MEDIUM;
    s.startepisode = 1;
    s.startmap = 1;
    s.autostart = false;
    let p = check_parm_with_args(&d.m_argv, "-skill", 1);
    if p > 0 {
        d.d_main.startskill = first_char(d, p + 1) - '1' as i32;
        d.d_main.autostart = true;
    }
    let p = check_parm_with_args(&d.m_argv, "-episode", 1);
    if p > 0 {
        d.d_main.startepisode = first_char(d, p + 1) - '0' as i32;
        d.d_main.startmap = 1;
        d.d_main.autostart = true;
    }
    d.d_main.timelimit = 0;
    let p = check_parm_with_args(&d.m_argv, "-timer", 1);
    if p > 0 {
        d.d_main.timelimit = crate::z_zone::atoi(&arg(d, p + 1));
    }
    if check_parm(&d.m_argv, "-avg") > 0 {
        d.d_main.timelimit = 20;
    }
    let p = check_parm_with_args(&d.m_argv, "-warp", 1);
    if p > 0 {
        if d.d_main.gamemode == COMMERCIAL {
            d.d_main.startmap = crate::z_zone::atoi(&arg(d, p + 1));
        } else {
            d.d_main.startepisode = first_char(d, p + 1) - '0' as i32;
            d.d_main.startmap = if p + 2 < d.m_argv.myargv.len() {
                first_char(d, p + 2) - '0' as i32
            } else {
                1
            };
        }
        d.d_main.autostart = true;
    }
    if check_parm(&d.m_argv, "-testcontrols") > 0 {
        d.d_main.startepisode = 1;
        d.d_main.startmap = 1;
        d.d_main.autostart = true;
        d.m_menu.testcontrols = true;
    }
    let p = check_parm_with_args(&d.m_argv, "-loadgame", 1);
    d.d_main.startloadgame = if p > 0 {
        crate::z_zone::atoi(&arg(d, p + 1))
    } else {
        -1
    };
    println!("M_Init: Init miscellaneous info.");
    crate::m_menu::init(d);
    print!("R_Init: Init DOOM refresh daemon - ");
    crate::r_main::init(d);
    println!("\nP_Init: Init Playloop state.");
    crate::p_setup::init(d);
    println!("S_Init: Setting up sound.");
    let (sv, mv) = (d.m_menu.sfx_volume * 8, d.m_menu.music_volume * 8);
    crate::s_sound::init(d, sv, mv);
    // S_Init allocates 8 sound channels of 24 bytes.
    d.z_zone.malloc(8 * 24, PU_STATIC, 0);
    println!("D_CheckNetGame: Checking network game status.");
    check_net_game(d);
    let v = d.d_main.gameversion;
    if let Some((desc, _, _)) = GAMEVERSIONS.iter().find(|g| g.2 == v) {
        println!("Emulating the behavior of the '{desc}' executable.");
    }
    println!("HU_Init: Setting up heads up display.");
    crate::hu_stuff::init(d);
    zone_cache(d, (33..=95).map(|i| format!("STCFN{i:03}")));
    println!("ST_Init: Init status bar.");
    crate::st_stuff::init(d);
    zone_cache(d, st_lump_names(d.g_game.consoleplayer));
    d.z_zone.malloc(320 * 32, PU_STATIC, 0);
    if d.d_main.gamemode == COMMERCIAL && check_num_for_name(&d.w_wad, "map01") < 0 {
        d.d_main.storedemo = true;
    }
    if check_parm_with_args(&d.m_argv, "-statdump", 1) > 0 {
        at_exit(d, ExitFunc::StatDump, true);
        println!("External statistics registered.");
    }
    let p = check_parm_with_args(&d.m_argv, "-record", 1);
    if p > 0 {
        crate::g_game::record_demo(d, &arg(d, p + 1));
        let name = format!("{}\0", d.g_game.demoname);
        d.z_zone.malloc_copy(name.as_bytes(), PU_STATIC, 0);
        let n = d.g_game.demoend as i32;
        d.z_zone.malloc(n, PU_STATIC, 0);
        d.d_main.autostart = true;
    }
    let name = d.d_main.demolumpname.clone();
    if check_parm_with_args(&d.m_argv, "-playdemo", 1) > 0 {
        d.g_game.singledemo = true;
        crate::g_game::defered_play_demo(d, &name);
    } else if check_parm_with_args(&d.m_argv, "-timedemo", 1) > 0 {
        crate::g_game::time_demo(d, &name);
    } else {
        if d.d_main.startloadgame >= 0 {
            let file = crate::g_game::save_game_name(d, d.d_main.startloadgame);
            crate::g_game::load_game(d, &file);
        }
        if d.g_game.gameaction != crate::g_game::GA_LOADGAME {
            if d.d_main.autostart || d.g_game.netgame {
                let s = &d.d_main;
                let (sk, e, m) = (s.startskill, s.startepisode, s.startmap);
                crate::g_game::init_new(d, sk, e, m);
            } else {
                start_title(d);
            }
        }
    }
    doom_loop(d);
    // defdemoname points to a D_DoomMain stack buffer. Later calls overwrite it with an empty string.
    if d.g_game.defdemoname == name {
        d.g_game.defdemoname.clear();
    }
}

/// Caches lumps by name in the zone model, as W_CacheLumpName with PU_STATIC does.
fn zone_cache(d: &mut Doom, names: impl IntoIterator<Item = String>) {
    for name in names {
        let lump = crate::w_wad::get_num_for_name(d, &name);
        d.z_zone.cache_lump(&d.w_wad, lump, PU_STATIC);
    }
}

/// The status bar graphics in ST_loadGraphics order.
fn st_lump_names(consoleplayer: usize) -> Vec<String> {
    let mut v: Vec<String> = (0..10)
        .flat_map(|i| [format!("STTNUM{i}"), format!("STYSNUM{i}")])
        .collect();
    v.push("STTPRCNT".into());
    v.extend((0..6).map(|i| format!("STKEYS{i}")));
    v.push("STARMS".into());
    v.extend((2..8).map(|i| format!("STGNUM{i}")));
    v.push(format!("STFB{consoleplayer}"));
    v.push("STBAR".into());
    for i in 0..5 {
        v.extend((0..3).map(|j| format!("STFST{i}{j}")));
        v.extend([
            format!("STFTR{i}0"),
            format!("STFTL{i}0"),
            format!("STFOUCH{i}"),
            format!("STFEVL{i}"),
            format!("STFKILL{i}"),
        ]);
    }
    v.extend(["STFGOD0".into(), "STFDEAD0".into()]);
    v
}

/// D_DoomLoop: starts the graphics and runs the first main-loop iteration.
fn doom_loop(d: &mut Doom) {
    if d.d_main.gamevariant == VARIANT_BFG
        && (d.g_game.demorecording
            || d.g_game.gameaction == crate::g_game::GA_PLAYDEMO
            || d.g_game.netgame)
    {
        println!(" WARNING: You are playing using one of the Doom Classic\n IWAD files shipped with the Doom 3: BFG Edition. These are\n known to be incompatible with the regular IWAD files and\n may cause demos and network games to get out of sync.");
    }
    if d.g_game.demorecording {
        crate::g_game::begin_recording(d);
    }
    d.d_main.main_loop_started = true;
    // The original allocates the startup level before the first tics.
    crate::r_data::zone_sync(d);
    crate::d_loop::try_run_tics(d);
    let title = d.d_main.gamedescription.clone();
    crate::i_video::dg_set_window_title(&d.i_video, &title);
    crate::i_video::init_graphics(d);
    // I_VideoBuffer starts with the stale zone bytes at its address.
    crate::r_data::zone_sync(d);
    let p = d.z_zone.malloc(64000, PU_STATIC, 0);
    crate::z_zone::check(d);
    if p != 0 {
        d.i_video
            .video_buffer
            .copy_from_slice(&d.z_zone.mem[p..p + 64000]);
    }
    d.d_main.screenvisible = true;
    crate::r_main::execute_set_view_size(d);
    d.d_loop.lasttime = crate::i_system::get_time();
    if d.m_menu.testcontrols {
        d.d_main.wipegamestate = d.g_game.gamestate;
    }
    doom_loop_tick(d);
}

/// doomgeneric_Tick: one main-loop iteration.
pub fn doom_loop_tick(d: &mut Doom) {
    crate::d_loop::try_run_tics(d);
    let listener = d.g_game.players[d.g_game.consoleplayer].mo;
    crate::s_sound::update_sounds(d, listener);
    if d.d_main.screenvisible {
        display(d);
    }
}

//! Menu screens, prompts, savegame slot names, quick save and load, and option settings.

use crate::d_main::{
    Event, COMMERCIAL, EV_JOYSTICK, EV_KEYDOWN, EV_MOUSE, EV_QUIT, EXE_CHEX, MISSION_DOOM,
    REGISTERED, RETAIL, SHAREWARE,
};
use crate::i_video::*;
use crate::s_sound::*;
use crate::Doom;

const SAVESTRINGSIZE: usize = 24;
const LINEHEIGHT: i32 = 16;
const SKULLXOFF: i32 = -32;
const EMPTYSTRING: &[u8] = b"empty slot";

// The key bindings. The original reads no configuration value, so the defaults are fixed.
const KEY_MENU_ACTIVATE: i32 = KEY_ESCAPE;
const KEY_MENU_UP: i32 = KEY_UPARROW;
const KEY_MENU_DOWN: i32 = KEY_DOWNARROW;
const KEY_MENU_LEFT: i32 = KEY_LEFTARROW;
const KEY_MENU_RIGHT: i32 = KEY_RIGHTARROW;
const KEY_MENU_BACK: i32 = KEY_BACKSPACE;
const KEY_MENU_FORWARD: i32 = KEY_ENTER;
const KEY_MENU_CONFIRM: i32 = b'y' as i32;
const KEY_MENU_ABORT: i32 = b'n' as i32;
const KEY_MENU_HELP: i32 = KEY_F1;
const KEY_MENU_SAVE: i32 = KEY_F2;
const KEY_MENU_LOAD: i32 = KEY_F3;
const KEY_MENU_VOLUME: i32 = KEY_F4;
const KEY_MENU_DETAIL: i32 = KEY_F5;
const KEY_MENU_QSAVE: i32 = KEY_F6;
const KEY_MENU_ENDGAME: i32 = KEY_F7;
const KEY_MENU_MESSAGES: i32 = KEY_F8;
const KEY_MENU_QLOAD: i32 = KEY_F9;
const KEY_MENU_QUIT: i32 = KEY_F10;
const KEY_MENU_GAMMA: i32 = KEY_F11;
const KEY_MENU_INCSCREEN: i32 = KEY_EQUALS;
const KEY_MENU_DECSCREEN: i32 = KEY_MINUS;
const KEY_MENU_SCREENSHOT: i32 = 0;
const JOYBMENU: i32 = -1;

const NEWGAME: &str = "you can't start a new game\nwhile in a network game.\n\npress a key.";
const LOADNET: &str = "you can't do load while in a net game!\n\npress a key.";
const SAVEDEAD: &str = "you can't save if you aren't playing!\n\npress a key.";
const QSPROMPT: (&str, &str) = ("quicksave over your game named\n\n'", "'?\n\npress y or n.");
const QLPROMPT: (&str, &str) = (
    "do you want to quickload the game named\n\n'",
    "'?\n\npress y or n.",
);
const QLOADNET: &str = "you can't quickload during a netgame!\n\npress a key.";
const QSAVESPOT: &str = "you haven't picked a quicksave slot yet!\n\npress a key.";
const SWSTRING: &str = "this is the shareware version of doom.\n\nyou need to order the entire trilogy.\n\npress a key.";
const NIGHTMARE: &str =
    "are you sure? this skill level\nisn't even remotely fair.\n\npress y or n.";
const NETEND: &str = "you can't end a netgame!\n\npress a key.";
const ENDGAME: &str = "are you sure you want to end the game?\n\npress y or n.";
const DOSY: &str = "(press y to quit to dos.)";
const GAMMAMSG: [&str; 5] = [
    "Gamma correction OFF",
    "Gamma correction level 1",
    "Gamma correction level 2",
    "Gamma correction level 3",
    "Gamma correction level 4",
];
const DOOM1_ENDMSG: [&str; 8] = [
    "are you sure you want to\nquit this great game?",
    "please don't leave, there's more\ndemons to toast!",
    "let's beat it -- this is turning\ninto a bloodbath!",
    "i wouldn't leave if i were you.\ndos is much worse.",
    "you're trying to say you like dos\nbetter than me, right?",
    "don't leave yet -- there's a\ndemon around that corner!",
    "ya know, next time you come in here\ni'm gonna toast ya.",
    "go ahead and leave. see if i care.",
];
const DOOM2_ENDMSG: [&str; 8] = [
    "are you sure you want to\nquit this great game?",
    "you want to quit?\nthen, thou hast lost an eighth!",
    "don't go now, there's a \ndimensional shambler waiting\nat the dos prompt!",
    "get outta here and go back\nto your boring programs.",
    "if i were your boss, i'd \n deathmatch ya in a minute!",
    "look, bud. you leave now\nand you forfeit your body count!",
    "just leave. when you come\nback, i'll be waiting with a bat.",
    "you're lucky i don't smack\nyou for thinking about leaving.",
];
const QUITSOUNDS: [i32; 8] = [
    SFX_PLDETH, SFX_DMPAIN, SFX_POPAIN, SFX_SLOP, SFX_TELEPT, SFX_POSIT1, SFX_POSIT3, SFX_SGTATK,
];
const QUITSOUNDS2: [i32; 8] = [
    SFX_VILACT, SFX_GETPOW, SFX_BOSCUB, SFX_SLOP, SFX_SKESWG, SFX_KNTDTH, SFX_BSPACT, SFX_SGTATK,
];
const SKULL_NAME: [&str; 2] = ["M_SKULL1", "M_SKULL2"];
const DETAIL_NAMES: [&str; 2] = ["M_GDHIGH", "M_GDLOW"];
const MSG_NAMES: [&str; 2] = ["M_MSGOFF", "M_MSGON"];

/// The menu_t values of the original data section.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuId {
    Main,
    Episode,
    NewGame,
    Options,
    ReadThis1,
    ReadThis2,
    Sound,
    Load,
    Save,
}

/// The item routines of the menu tables.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    NewGame,
    Options,
    LoadGame,
    SaveGame,
    ReadThis,
    QuitDoom,
    Episode,
    ChooseSkill,
    EndGame,
    ChangeMessages,
    ChangeDetail,
    SizeDisplay,
    ChangeSensitivity,
    Sound,
    ReadThis2,
    FinishReadThis,
    SfxVol,
    MusicVol,
    LoadSelect,
    SaveSelect,
}

/// The message routines of M_StartMessage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MsgRoutine {
    QuitResponse,
    VerifyNightmare,
    EndGameResponse,
    QuickSaveResponse,
    QuickLoadResponse,
}

#[derive(Clone, Copy)]
struct MenuItem {
    status: i16,
    name: &'static str,
    routine: Option<Action>,
    alpha_key: u8,
}

#[derive(Clone)]
struct Menu {
    numitems: i16,
    prev: Option<MenuId>,
    items: Vec<MenuItem>,
    x: i16,
    y: i16,
    last_on: i16,
}

const fn it(status: i16, name: &'static str, routine: Action, alpha_key: u8) -> MenuItem {
    MenuItem {
        status,
        name,
        routine: Some(routine),
        alpha_key,
    }
}

const GAP: MenuItem = MenuItem {
    status: -1,
    name: "",
    routine: None,
    alpha_key: 0,
};

fn menu(prev: Option<MenuId>, items: Vec<MenuItem>, x: i16, y: i16, last_on: i16) -> Menu {
    Menu {
        numitems: items.len() as i16,
        prev,
        items,
        x,
        y,
        last_on,
    }
}

fn default_menus() -> Vec<Menu> {
    use Action as A;
    use MenuId as M;
    let slots = |a: Action| (b'1'..=b'6').map(|k| it(1, "", a, k)).collect::<Vec<_>>();
    vec![
        menu(
            None,
            vec![
                it(1, "M_NGAME", A::NewGame, b'n'),
                it(1, "M_OPTION", A::Options, b'o'),
                it(1, "M_LOADG", A::LoadGame, b'l'),
                it(1, "M_SAVEG", A::SaveGame, b's'),
                it(1, "M_RDTHIS", A::ReadThis, b'r'),
                it(1, "M_QUITG", A::QuitDoom, b'q'),
            ],
            97,
            64,
            0,
        ),
        menu(
            Some(M::Main),
            vec![
                it(1, "M_EPI1", A::Episode, b'k'),
                it(1, "M_EPI2", A::Episode, b't'),
                it(1, "M_EPI3", A::Episode, b'i'),
                it(1, "M_EPI4", A::Episode, b't'),
            ],
            48,
            63,
            0,
        ),
        menu(
            Some(M::Episode),
            vec![
                it(1, "M_JKILL", A::ChooseSkill, b'i'),
                it(1, "M_ROUGH", A::ChooseSkill, b'h'),
                it(1, "M_HURT", A::ChooseSkill, b'h'),
                it(1, "M_ULTRA", A::ChooseSkill, b'u'),
                it(1, "M_NMARE", A::ChooseSkill, b'n'),
            ],
            48,
            63,
            2,
        ),
        menu(
            Some(M::Main),
            vec![
                it(1, "M_ENDGAM", A::EndGame, b'e'),
                it(1, "M_MESSG", A::ChangeMessages, b'm'),
                it(1, "M_DETAIL", A::ChangeDetail, b'g'),
                it(2, "M_SCRNSZ", A::SizeDisplay, b's'),
                GAP,
                it(2, "M_MSENS", A::ChangeSensitivity, b'm'),
                GAP,
                it(1, "M_SVOL", A::Sound, b's'),
            ],
            60,
            37,
            0,
        ),
        menu(Some(M::Main), vec![it(1, "", A::ReadThis2, 0)], 280, 185, 0),
        menu(
            Some(M::ReadThis1),
            vec![it(1, "", A::FinishReadThis, 0)],
            330,
            175,
            0,
        ),
        menu(
            Some(M::Options),
            vec![
                it(2, "M_SFXVOL", A::SfxVol, b's'),
                GAP,
                it(2, "M_MUSVOL", A::MusicVol, b'm'),
                GAP,
            ],
            80,
            64,
            0,
        ),
        menu(Some(M::Main), slots(A::LoadSelect), 80, 54, 0),
        menu(Some(M::Main), slots(A::SaveSelect), 80, 54, 0),
    ]
}

/// The globals of m_menu. The option values live here because the menu owns their defaults.
pub struct State {
    pub menuactive: bool,
    pub show_messages: i32,
    pub inhelpscreens: bool,
    pub screenblocks: i32,
    pub detail_level: i32,
    pub mouse_sensitivity: i32,
    pub sfx_volume: i32,
    pub music_volume: i32,
    /// The -testcontrols flag. D_DoomMain sets it.
    pub testcontrols: bool,
    pub screen_size: i32,
    epi: i32,
    menus: Vec<Menu>,
    current: MenuId,
    item_on: i16,
    which_skull: i16,
    skull_anim_counter: i16,
    message_to_print: bool,
    message_string: Vec<u8>,
    message_last_menu_active: bool,
    message_needs_input: bool,
    message_routine: Option<MsgRoutine>,
    savegamestrings: [[u8; SAVESTRINGSIZE]; 6],
    save_string_enter: bool,
    save_slot: i32,
    save_char_index: i32,
    save_old_string: [u8; SAVESTRINGSIZE],
    quick_save_slot: i32,
    joywait: i32,
    mousewait: i32,
    mousey: i32,
    lasty: i32,
    mousex: i32,
    lastx: i32,
}

impl Default for State {
    fn default() -> Self {
        State {
            menuactive: false,
            show_messages: 1,
            inhelpscreens: false,
            screenblocks: 10,
            detail_level: 0,
            mouse_sensitivity: 5,
            sfx_volume: 8,
            music_volume: 8,
            testcontrols: false,
            screen_size: 0,
            epi: 0,
            menus: default_menus(),
            current: MenuId::Main,
            item_on: 0,
            which_skull: 0,
            skull_anim_counter: 0,
            message_to_print: false,
            message_string: Vec::new(),
            message_last_menu_active: false,
            message_needs_input: false,
            message_routine: None,
            savegamestrings: [[0; SAVESTRINGSIZE]; 6],
            save_string_enter: false,
            save_slot: 0,
            save_char_index: 0,
            save_old_string: [0; SAVESTRINGSIZE],
            quick_save_slot: 0,
            joywait: 0,
            mousewait: 0,
            mousey: 0,
            lasty: 0,
            mousex: 0,
            lastx: 0,
        }
    }
}

/// Returns the bytes before the first NUL.
fn cstr(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

/// M_StringCopy: copies at most dst.len() - 1 bytes and pads with NUL.
fn copy_str(dst: &mut [u8], src: &[u8]) {
    let src = cstr(src);
    let n = src.len().min(dst.len() - 1);
    dst[..n].copy_from_slice(&src[..n]);
    dst[n..].fill(0);
}

/// Returns the heads-up font index of a character, or None for a space.
fn font_index(ch: u8) -> Option<usize> {
    let c = ch.to_ascii_uppercase() as i32 - b'!' as i32;
    (0..63).contains(&c).then_some(c as usize)
}

fn current(d: &Doom) -> &Menu {
    &d.m_menu.menus[d.m_menu.current as usize]
}

fn menu_mut(d: &mut Doom, m: MenuId) -> &mut Menu {
    &mut d.m_menu.menus[m as usize]
}

fn sound(d: &mut Doom, sfx: i32) {
    crate::s_sound::start_sound(d, None, sfx);
}

fn player_message(d: &mut Doom, msg: &str) {
    let cp = d.g_game.consoleplayer;
    d.g_game.players[cp].message = Some(msg.to_string());
}

fn draw_named(d: &mut Doom, x: i32, y: i32, name: &str) {
    let lump = crate::w_wad::get_num_for_name(d, name);
    crate::v_video::draw_patch_direct(d, x, y, lump);
}

fn font_width(d: &Doom, c: usize) -> i32 {
    crate::hu_lib::patch_width(d, d.hu_stuff.hu_font[c])
}

/// M_StringWidth.
pub fn string_width(d: &Doom, s: &[u8]) -> i32 {
    cstr(s)
        .iter()
        .map(|&ch| font_index(ch).map_or(4, |c| font_width(d, c)))
        .sum()
}

/// M_StringHeight.
pub fn string_height(d: &Doom, s: &[u8]) -> i32 {
    let height = crate::hu_lib::patch_height(d, d.hu_stuff.hu_font[0]);
    height + cstr(s).iter().filter(|&&c| c == b'\n').count() as i32 * height
}

/// M_WriteText.
pub fn write_text(d: &mut Doom, x: i32, y: i32, s: &[u8]) {
    let (mut cx, mut cy) = (x, y);
    for &ch in cstr(s) {
        if ch == b'\n' {
            cx = x;
            cy += 12;
            continue;
        }
        let Some(c) = font_index(ch) else {
            cx += 4;
            continue;
        };
        let w = font_width(d, c);
        if cx + w > 320 {
            break;
        }
        let lump = d.hu_stuff.hu_font[c];
        crate::v_video::draw_patch_direct(d, cx, cy, lump);
        cx += w;
    }
}

/// Splits a message into the lines that M_Drawer draws. Each line keeps at most 79 bytes.
fn message_lines(msg: &[u8]) -> Vec<Vec<u8>> {
    let msg = cstr(msg);
    let mut lines = Vec::new();
    let mut start = 0;
    while start < msg.len() {
        let rest = &msg[start..];
        let mut line = rest[..rest.len().min(79)].to_vec();
        if let Some(i) = rest.iter().position(|&c| c == b'\n') {
            line.truncate(i);
            start += i + 1;
        } else {
            start += line.len();
        }
        lines.push(line);
    }
    lines
}

fn setup_next_menu(d: &mut Doom, m: MenuId) {
    d.m_menu.current = m;
    d.m_menu.item_on = d.m_menu.menus[m as usize].last_on;
}

fn clear_menus(d: &mut Doom) {
    d.m_menu.menuactive = false;
}

fn start_message(d: &mut Doom, string: &str, routine: Option<MsgRoutine>, input: bool) {
    let s = &mut d.m_menu;
    s.message_last_menu_active = s.menuactive;
    s.message_to_print = true;
    s.message_string = string.as_bytes().to_vec();
    s.message_routine = routine;
    s.message_needs_input = input;
    s.menuactive = true;
}

/// M_StartControlPanel.
pub fn start_control_panel(d: &mut Doom) {
    if d.m_menu.menuactive {
        return;
    }
    d.m_menu.menuactive = true;
    setup_next_menu(d, MenuId::Main);
}

/// screenblocks and detailLevel for R_Init.
pub fn view_settings(d: &Doom) -> (i32, i32) {
    (d.m_menu.screenblocks, d.m_menu.detail_level)
}

/// M_Init.
pub fn init(d: &mut Doom) {
    let s = &mut d.m_menu;
    s.current = MenuId::Main;
    s.menuactive = false;
    s.item_on = s.menus[MenuId::Main as usize].last_on;
    s.which_skull = 0;
    s.skull_anim_counter = 10;
    s.screen_size = s.screenblocks - 3;
    s.message_to_print = false;
    s.message_string.clear();
    s.message_last_menu_active = false;
    s.quick_save_slot = -1;
    if d.d_main.gamemode == COMMERCIAL {
        let main = &mut s.menus[MenuId::Main as usize];
        main.items[4] = main.items[5];
        main.numitems -= 1;
        main.y += 8;
        s.menus[MenuId::NewGame as usize].prev = Some(MenuId::Main);
    }
    if d.d_main.gameversion <= 5 {
        s.menus[MenuId::Episode as usize].numitems -= 1;
    }
}

/// M_Ticker.
pub fn ticker(d: &mut Doom) {
    let s = &mut d.m_menu;
    s.skull_anim_counter -= 1;
    if s.skull_anim_counter <= 0 {
        s.which_skull ^= 1;
        s.skull_anim_counter = 8;
    }
}

/// M_Drawer.
pub fn drawer(d: &mut Doom) {
    d.m_menu.inhelpscreens = false;
    if d.m_menu.message_to_print {
        let msg = d.m_menu.message_string.clone();
        let height = crate::hu_lib::patch_height(d, d.hu_stuff.hu_font[0]);
        let mut y = 100 - string_height(d, &msg) / 2;
        for line in message_lines(&msg) {
            let x = 160 - string_width(d, &line) / 2;
            write_text(d, x, y, &line);
            y += height;
        }
        return;
    }
    if !d.m_menu.menuactive {
        return;
    }
    draw_routine(d, d.m_menu.current);
    let m = current(d).clone();
    let (x, mut y) = (m.x as i32, m.y as i32);
    for item in &m.items[..m.numitems as usize] {
        if !item.name.is_empty() {
            draw_named(d, x, y, item.name);
        }
        y += LINEHEIGHT;
    }
    let skull = SKULL_NAME[d.m_menu.which_skull as usize];
    draw_named(
        d,
        x + SKULLXOFF,
        m.y as i32 - 5 + d.m_menu.item_on as i32 * LINEHEIGHT,
        skull,
    );
}

fn draw_routine(d: &mut Doom, m: MenuId) {
    match m {
        MenuId::Main => draw_named(d, 94, 2, "M_DOOM"),
        MenuId::Episode => draw_named(d, 54, 38, "M_EPISOD"),
        MenuId::NewGame => {
            draw_named(d, 96, 14, "M_NEWG");
            draw_named(d, 54, 38, "M_SKILL");
        }
        MenuId::Options => draw_options(d),
        MenuId::ReadThis1 => draw_read_this1(d),
        MenuId::ReadThis2 => {
            d.m_menu.inhelpscreens = true;
            draw_named(d, 0, 0, "HELP1");
        }
        MenuId::Sound => {
            draw_named(d, 60, 38, "M_SVOL");
            let (x, y) = (
                d.m_menu.menus[MenuId::Sound as usize].x as i32,
                d.m_menu.menus[MenuId::Sound as usize].y as i32,
            );
            let (sfx, music) = (d.m_menu.sfx_volume, d.m_menu.music_volume);
            draw_thermo(d, x, y + LINEHEIGHT, 16, sfx);
            draw_thermo(d, x, y + 3 * LINEHEIGHT, 16, music);
        }
        MenuId::Load => draw_load(d, "M_LOADG"),
        MenuId::Save => {
            draw_load(d, "M_SAVEG");
            if d.m_menu.save_string_enter {
                let slot = d.m_menu.save_slot as usize;
                let s = d.m_menu.savegamestrings[slot];
                let i = string_width(d, &s);
                let l = &d.m_menu.menus[MenuId::Load as usize];
                let (x, y) = (l.x as i32, l.y as i32);
                write_text(d, x + i, y + LINEHEIGHT * slot as i32, b"_");
            }
        }
    }
}

fn draw_options(d: &mut Doom) {
    draw_named(d, 108, 15, "M_OPTTTL");
    let o = &d.m_menu.menus[MenuId::Options as usize];
    let (x, y) = (o.x as i32, o.y as i32);
    let detail = DETAIL_NAMES[d.m_menu.detail_level as usize];
    draw_named(d, x + 175, y + 2 * LINEHEIGHT, detail);
    let msg = MSG_NAMES[d.m_menu.show_messages as usize];
    draw_named(d, x + 120, y + LINEHEIGHT, msg);
    let (sens, size) = (d.m_menu.mouse_sensitivity, d.m_menu.screen_size);
    draw_thermo(d, x, y + 6 * LINEHEIGHT, 10, sens);
    draw_thermo(d, x, y + 4 * LINEHEIGHT, 9, size);
}

fn draw_read_this1(d: &mut Doom) {
    d.m_menu.inhelpscreens = true;
    let commercial = d.d_main.gamemode == COMMERCIAL;
    let (lump, x, y) = match d.d_main.gameversion {
        1..=5 if commercial => ("HELP", 330, 165),
        1..=5 => ("HELP2", 280, 185),
        7 | 8 => ("HELP", 330, 175),
        6 | 9 => ("HELP1", 330, 175),
        _ => crate::i_system::error(d, "Unhandled game version"),
    };
    draw_named(d, 0, 0, lump);
    let m = menu_mut(d, MenuId::ReadThis1);
    m.x = x;
    m.y = y;
}

fn draw_load(d: &mut Doom, title: &str) {
    draw_named(d, 72, 28, title);
    let l = &d.m_menu.menus[MenuId::Load as usize];
    let (x, y) = (l.x as i32, l.y as i32);
    for i in 0..6 {
        draw_save_load_border(d, x, y + LINEHEIGHT * i);
        let s = d.m_menu.savegamestrings[i as usize];
        write_text(d, x, y + LINEHEIGHT * i, &s);
    }
}

fn draw_save_load_border(d: &mut Doom, x: i32, y: i32) {
    draw_named(d, x - 8, y + 7, "M_LSLEFT");
    for i in 0..24 {
        draw_named(d, x + 8 * i, y + 7, "M_LSCNTR");
    }
    draw_named(d, x + 192, y + 7, "M_LSRGHT");
}

fn draw_thermo(d: &mut Doom, x: i32, y: i32, therm_width: i32, therm_dot: i32) {
    draw_named(d, x, y, "M_THERML");
    let mut xx = x + 8;
    for _ in 0..therm_width {
        draw_named(d, xx, y, "M_THERMM");
        xx += 8;
    }
    draw_named(d, xx, y, "M_THERMR");
    draw_named(d, x + 8 + therm_dot * 8, y, "M_THERMO");
}

/// M_ReadSaveStrings: reads the 24-byte description of each savegame file.
fn read_save_strings(d: &mut Doom) {
    use std::io::Read;
    for i in 0..6 {
        let name = crate::g_game::save_game_name(d, i as i32);
        let status = match std::fs::File::open(&name) {
            Ok(mut f) => {
                let buf = &mut d.m_menu.savegamestrings[i];
                let mut n = 0;
                while n < SAVESTRINGSIZE {
                    match f.read(&mut buf[n..]) {
                        Ok(0) | Err(_) => break,
                        Ok(k) => n += k,
                    }
                }
                1
            }
            Err(_) => {
                copy_str(&mut d.m_menu.savegamestrings[i], EMPTYSTRING);
                0
            }
        };
        menu_mut(d, MenuId::Load).items[i].status = status;
    }
}

fn call(d: &mut Doom, a: Action, choice: i32) {
    match a {
        Action::NewGame => new_game(d),
        Action::Options => setup_next_menu(d, MenuId::Options),
        Action::LoadGame => {
            if d.g_game.netgame {
                start_message(d, LOADNET, None, false);
                return;
            }
            setup_next_menu(d, MenuId::Load);
            read_save_strings(d);
        }
        Action::SaveGame => save_game(d),
        Action::ReadThis => setup_next_menu(d, MenuId::ReadThis1),
        Action::QuitDoom => quit_doom(d),
        Action::Episode => episode(d, choice),
        Action::ChooseSkill => {
            if choice == crate::g_game::SK_NIGHTMARE {
                start_message(d, NIGHTMARE, Some(MsgRoutine::VerifyNightmare), true);
                return;
            }
            let epi = d.m_menu.epi;
            crate::g_game::defered_init_new(d, choice, epi + 1, 1);
            clear_menus(d);
        }
        Action::EndGame => end_game(d),
        Action::ChangeMessages => change_messages(d),
        Action::ChangeDetail => change_detail(d),
        Action::SizeDisplay => size_display(d, choice),
        Action::ChangeSensitivity => {
            let s = &mut d.m_menu.mouse_sensitivity;
            match choice {
                0 if *s != 0 => *s -= 1,
                1 if *s < 9 => *s += 1,
                _ => {}
            }
        }
        Action::Sound => setup_next_menu(d, MenuId::Sound),
        Action::ReadThis2 => {
            let next = if d.d_main.gameversion < 5 && d.d_main.gamemode != COMMERCIAL {
                MenuId::ReadThis2
            } else {
                MenuId::Main
            };
            setup_next_menu(d, next);
        }
        Action::FinishReadThis => setup_next_menu(d, MenuId::Main),
        Action::SfxVol => {
            let v = step_volume(&mut d.m_menu.sfx_volume, choice);
            crate::s_sound::set_sfx_volume(d, v * 8);
        }
        Action::MusicVol => {
            let v = step_volume(&mut d.m_menu.music_volume, choice);
            crate::s_sound::set_music_volume(d, v * 8);
        }
        Action::LoadSelect => load_select(d, choice),
        Action::SaveSelect => save_select(d, choice),
    }
}

fn step_volume(v: &mut i32, choice: i32) -> i32 {
    match choice {
        0 if *v != 0 => *v -= 1,
        1 if *v < 15 => *v += 1,
        _ => {}
    }
    *v
}

fn new_game(d: &mut Doom) {
    if d.g_game.netgame && !d.g_game.demoplayback {
        start_message(d, NEWGAME, None, false);
        return;
    }
    let next = if d.d_main.gamemode == COMMERCIAL || d.d_main.gameversion == EXE_CHEX {
        MenuId::NewGame
    } else {
        MenuId::Episode
    };
    setup_next_menu(d, next);
}

fn episode(d: &mut Doom, mut choice: i32) {
    if d.d_main.gamemode == SHAREWARE && choice != 0 {
        start_message(d, SWSTRING, None, false);
        setup_next_menu(d, MenuId::ReadThis1);
        return;
    }
    if choice >= 3 && d.d_main.gamemode == REGISTERED {
        eprintln!("M_Episode: 4th episode requires UltimateDOOM");
        choice = 0;
    }
    d.m_menu.epi = choice;
    setup_next_menu(d, MenuId::NewGame);
}

fn save_game(d: &mut Doom) {
    if !d.g_game.usergame {
        start_message(d, SAVEDEAD, None, false);
        return;
    }
    if d.g_game.gamestate != crate::g_game::GS_LEVEL {
        return;
    }
    setup_next_menu(d, MenuId::Save);
    read_save_strings(d);
}

fn quit_doom(d: &mut Doom) {
    let table = if crate::hu_stuff::logical_gamemission(d) == MISSION_DOOM {
        &DOOM1_ENDMSG
    } else {
        &DOOM2_ENDMSG
    };
    let msg = format!(
        "{}\n\n{}",
        table[d.g_game.gametic.rem_euclid(8) as usize],
        DOSY
    );
    start_message(d, &msg, Some(MsgRoutine::QuitResponse), true);
}

fn end_game(d: &mut Doom) {
    if !d.g_game.usergame {
        sound(d, SFX_OOF);
        return;
    }
    if d.g_game.netgame {
        start_message(d, NETEND, None, false);
        return;
    }
    start_message(d, ENDGAME, Some(MsgRoutine::EndGameResponse), true);
}

fn change_messages(d: &mut Doom) {
    let old = d.m_menu.show_messages;
    d.m_menu.show_messages = 1 - old;
    player_message(
        d,
        if old == 1 {
            "Messages OFF"
        } else {
            "Messages ON"
        },
    );
    d.hu_stuff.message_dontfuckwithme = true;
}

fn change_detail(d: &mut Doom) {
    let old = d.m_menu.detail_level;
    d.m_menu.detail_level = 1 - old;
    let (blocks, detail) = view_settings(d);
    crate::r_main::set_view_size(d, blocks, detail);
    player_message(
        d,
        if old == 1 {
            "High detail"
        } else {
            "Low detail"
        },
    );
}

fn size_display(d: &mut Doom, choice: i32) {
    let s = &mut d.m_menu;
    match choice {
        0 if s.screen_size > 0 => {
            s.screenblocks -= 1;
            s.screen_size -= 1;
        }
        1 if s.screen_size < 8 => {
            s.screenblocks += 1;
            s.screen_size += 1;
        }
        _ => {}
    }
    let (blocks, detail) = view_settings(d);
    crate::r_main::set_view_size(d, blocks, detail);
}

fn load_select(d: &mut Doom, choice: i32) {
    let name = crate::g_game::save_game_name(d, choice);
    crate::g_game::load_game(d, &name);
    clear_menus(d);
}

fn save_select(d: &mut Doom, choice: i32) {
    let s = &mut d.m_menu;
    s.save_string_enter = true;
    s.save_slot = choice;
    let slot = s.savegamestrings[choice as usize];
    copy_str(&mut s.save_old_string, &slot);
    if cstr(&slot) == EMPTYSTRING {
        s.savegamestrings[choice as usize][0] = 0;
    }
    s.save_char_index = cstr(&s.savegamestrings[choice as usize]).len() as i32;
}

/// M_DoSave.
fn do_save(d: &mut Doom, slot: i32) {
    let desc = String::from_utf8_lossy(cstr(&d.m_menu.savegamestrings[slot as usize])).into_owned();
    crate::g_game::save_game(d, slot, &desc);
    clear_menus(d);
    if d.m_menu.quick_save_slot == -2 {
        d.m_menu.quick_save_slot = slot;
    }
}

fn slot_prompt(d: &Doom, prompt: (&str, &str)) -> String {
    let name = String::from_utf8_lossy(cstr(
        &d.m_menu.savegamestrings[d.m_menu.quick_save_slot as usize],
    ))
    .into_owned();
    let mut s = format!("{}{}{}", prompt.0, name, prompt.1);
    s.truncate(79);
    s
}

fn quick_save(d: &mut Doom) {
    if !d.g_game.usergame {
        sound(d, SFX_OOF);
        return;
    }
    if d.g_game.gamestate != crate::g_game::GS_LEVEL {
        return;
    }
    if d.m_menu.quick_save_slot < 0 {
        start_control_panel(d);
        read_save_strings(d);
        setup_next_menu(d, MenuId::Save);
        d.m_menu.quick_save_slot = -2;
        return;
    }
    let msg = slot_prompt(d, QSPROMPT);
    start_message(d, &msg, Some(MsgRoutine::QuickSaveResponse), true);
}

fn quick_load(d: &mut Doom) {
    if d.g_game.netgame {
        start_message(d, QLOADNET, None, false);
    } else if d.m_menu.quick_save_slot < 0 {
        start_message(d, QSAVESPOT, None, false);
    } else {
        let msg = slot_prompt(d, QLPROMPT);
        start_message(d, &msg, Some(MsgRoutine::QuickLoadResponse), true);
    }
}

fn call_message(d: &mut Doom, r: MsgRoutine, key: i32) {
    if key != KEY_MENU_CONFIRM {
        return;
    }
    match r {
        MsgRoutine::QuitResponse => {
            if !d.g_game.netgame {
                let table = if d.d_main.gamemode == COMMERCIAL {
                    &QUITSOUNDS2
                } else {
                    &QUITSOUNDS
                };
                let sfx = table[((d.g_game.gametic >> 2) & 7) as usize];
                sound(d, sfx);
            }
            crate::i_system::quit(d);
        }
        MsgRoutine::VerifyNightmare => {
            let epi = d.m_menu.epi;
            crate::g_game::defered_init_new(d, crate::g_game::SK_NIGHTMARE, epi + 1, 1);
            clear_menus(d);
        }
        MsgRoutine::EndGameResponse => {
            let on = d.m_menu.item_on;
            menu_mut(d, d.m_menu.current).last_on = on;
            clear_menus(d);
            // D_StartTitle.
            d.g_game.gameaction = crate::g_game::GA_NOTHING;
            d.d_main.demosequence = -1;
            d.d_main.advancedemo = true;
        }
        MsgRoutine::QuickSaveResponse => {
            let slot = d.m_menu.quick_save_slot;
            do_save(d, slot);
            sound(d, SFX_SWTCHX);
        }
        MsgRoutine::QuickLoadResponse => {
            let slot = d.m_menu.quick_save_slot;
            load_select(d, slot);
            sound(d, SFX_SWTCHX);
        }
    }
}

/// Reads the joystick and mouse menu navigation. It returns the menu key or -1.
fn pointer_key(d: &mut Doom, ev: &Event) -> i32 {
    use crate::d_loop::get_time;
    let mut key = -1;
    let s = &mut d.m_menu;
    if ev.type_ == EV_JOYSTICK && s.joywait < get_time() {
        if ev.data3 < 0 {
            key = KEY_MENU_UP;
            s.joywait = get_time() + 5;
        } else if ev.data3 > 0 {
            key = KEY_MENU_DOWN;
            s.joywait = get_time() + 5;
        }
        if ev.data2 < 0 {
            key = KEY_MENU_LEFT;
            s.joywait = get_time() + 2;
        } else if ev.data2 > 0 {
            key = KEY_MENU_RIGHT;
            s.joywait = get_time() + 2;
        }
        if ev.data1 & 1 != 0 {
            key = KEY_MENU_FORWARD;
            s.joywait = get_time() + 5;
        }
        if ev.data1 & 2 != 0 {
            key = KEY_MENU_BACK;
            s.joywait = get_time() + 5;
        }
        if JOYBMENU >= 0 && ev.data1 & (1 << JOYBMENU) != 0 {
            key = KEY_MENU_ACTIVATE;
            s.joywait = get_time() + 5;
        }
    } else if ev.type_ == EV_MOUSE && s.mousewait < get_time() {
        s.mousey += ev.data3;
        if s.mousey < s.lasty - 30 {
            key = KEY_MENU_DOWN;
            s.mousewait = get_time() + 5;
            s.lasty -= 30;
            s.mousey = s.lasty;
        } else if s.mousey > s.lasty + 30 {
            key = KEY_MENU_UP;
            s.mousewait = get_time() + 5;
            s.lasty += 30;
            s.mousey = s.lasty;
        }
        s.mousex += ev.data2;
        if s.mousex < s.lastx - 30 {
            key = KEY_MENU_LEFT;
            s.mousewait = get_time() + 5;
            s.lastx -= 30;
            s.mousex = s.lastx;
        } else if s.mousex > s.lastx + 30 {
            key = KEY_MENU_RIGHT;
            s.mousewait = get_time() + 5;
            s.lastx += 30;
            s.mousex = s.lastx;
        }
        if ev.data1 & 1 != 0 {
            key = KEY_MENU_FORWARD;
            s.mousewait = get_time() + 15;
        }
        if ev.data1 & 2 != 0 {
            key = KEY_MENU_BACK;
            s.mousewait = get_time() + 15;
        }
    }
    key
}

/// M_Responder.
pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    if d.m_menu.testcontrols {
        if ev.type_ == EV_QUIT
            || (ev.type_ == EV_KEYDOWN
                && (ev.data1 == KEY_MENU_ACTIVATE || ev.data1 == KEY_MENU_QUIT))
        {
            crate::i_system::quit(d);
            #[allow(unreachable_code)]
            return true;
        }
        return false;
    }
    if ev.type_ == EV_QUIT {
        if d.m_menu.menuactive
            && d.m_menu.message_to_print
            && d.m_menu.message_routine == Some(MsgRoutine::QuitResponse)
        {
            call_message(d, MsgRoutine::QuitResponse, KEY_MENU_CONFIRM);
            return true;
        }
        sound(d, SFX_SWTCHN);
        quit_doom(d);
        return true;
    }
    let (key, ch) = if ev.type_ == EV_KEYDOWN {
        (ev.data1, ev.data2)
    } else {
        (pointer_key(d, ev), 0)
    };
    if key == -1 {
        return false;
    }

    if d.m_menu.save_string_enter {
        save_string_key(d, key);
        return true;
    }

    if d.m_menu.message_to_print {
        if d.m_menu.message_needs_input
            && ![b' ' as i32, KEY_ESCAPE, KEY_MENU_CONFIRM, KEY_MENU_ABORT].contains(&key)
        {
            return false;
        }
        d.m_menu.menuactive = d.m_menu.message_last_menu_active;
        d.m_menu.message_to_print = false;
        if let Some(r) = d.m_menu.message_routine {
            call_message(d, r, key);
        }
        d.m_menu.menuactive = false;
        sound(d, SFX_SWTCHX);
        return true;
    }

    if (d.d_main.devparm && key == KEY_MENU_HELP) || (key != 0 && key == KEY_MENU_SCREENSHOT) {
        crate::g_game::screen_shot(d);
        return true;
    }

    if !d.m_menu.menuactive {
        return inactive_key(d, key);
    }
    active_key(d, key, ch)
}

fn save_string_key(d: &mut Doom, key: i32) {
    let s = &mut d.m_menu;
    let slot = s.save_slot as usize;
    match key {
        KEY_BACKSPACE => {
            if s.save_char_index > 0 {
                s.save_char_index -= 1;
                s.savegamestrings[slot][s.save_char_index as usize] = 0;
            }
        }
        KEY_ESCAPE => {
            s.save_string_enter = false;
            let old = s.save_old_string;
            copy_str(&mut s.savegamestrings[slot], &old);
        }
        KEY_ENTER => {
            s.save_string_enter = false;
            if s.savegamestrings[slot][0] != 0 {
                do_save(d, slot as i32);
            }
        }
        _ => {
            let ch = if (b'a' as i32..=b'z' as i32).contains(&key) {
                key - 32
            } else {
                key
            };
            if !(32..=95).contains(&ch) || s.save_char_index > 22 {
                return;
            }
            let text = s.savegamestrings[slot];
            if string_width(d, &text) > 175 {
                return;
            }
            let s = &mut d.m_menu;
            let i = s.save_char_index as usize;
            s.savegamestrings[slot][i] = ch as u8;
            s.savegamestrings[slot][i + 1] = 0;
            s.save_char_index += 1;
        }
    }
}

/// Handles the function keys while no menu is open.
fn inactive_key(d: &mut Doom, key: i32) -> bool {
    match key {
        KEY_MENU_DECSCREEN | KEY_MENU_INCSCREEN => {
            if d.am_map.automapactive || d.hu_stuff.chat_on {
                return false;
            }
            size_display(d, (key == KEY_MENU_INCSCREEN) as i32);
            sound(d, SFX_STNMOV);
        }
        KEY_MENU_HELP => {
            start_control_panel(d);
            d.m_menu.current = if d.d_main.gamemode == RETAIL {
                MenuId::ReadThis2
            } else {
                MenuId::ReadThis1
            };
            d.m_menu.item_on = 0;
            sound(d, SFX_SWTCHN);
        }
        KEY_MENU_SAVE | KEY_MENU_LOAD => {
            start_control_panel(d);
            sound(d, SFX_SWTCHN);
            call(
                d,
                if key == KEY_MENU_SAVE {
                    Action::SaveGame
                } else {
                    Action::LoadGame
                },
                0,
            );
        }
        KEY_MENU_VOLUME => {
            start_control_panel(d);
            d.m_menu.current = MenuId::Sound;
            d.m_menu.item_on = 0;
            sound(d, SFX_SWTCHN);
        }
        KEY_MENU_DETAIL => {
            change_detail(d);
            sound(d, SFX_SWTCHN);
        }
        KEY_MENU_QSAVE => {
            sound(d, SFX_SWTCHN);
            quick_save(d);
        }
        KEY_MENU_ENDGAME => {
            sound(d, SFX_SWTCHN);
            end_game(d);
        }
        KEY_MENU_MESSAGES => {
            change_messages(d);
            sound(d, SFX_SWTCHN);
        }
        KEY_MENU_QLOAD => {
            sound(d, SFX_SWTCHN);
            quick_load(d);
        }
        KEY_MENU_QUIT => {
            sound(d, SFX_SWTCHN);
            quit_doom(d);
        }
        KEY_MENU_GAMMA => {
            let g = &mut d.i_video.usegamma;
            *g = if *g > 3 { 0 } else { *g + 1 };
            player_message(d, GAMMAMSG[d.i_video.usegamma as usize]);
            let lump = crate::w_wad::get_num_for_name(d, "PLAYPAL");
            let pal = crate::w_wad::lump(&d.w_wad, lump).to_vec();
            crate::i_video::set_palette(&mut d.i_video, &pal);
        }
        KEY_MENU_ACTIVATE => {
            start_control_panel(d);
            sound(d, SFX_SWTCHN);
        }
        _ => return false,
    }
    true
}

/// Handles the keys while a menu is open.
fn active_key(d: &mut Doom, key: i32, ch: i32) -> bool {
    let m = current(d).clone();
    let on = d.m_menu.item_on;
    let item = m.items[on as usize];
    match key {
        KEY_MENU_DOWN | KEY_MENU_UP => loop {
            let s = &mut d.m_menu;
            s.item_on = match key {
                KEY_MENU_DOWN if s.item_on + 1 > m.numitems - 1 => 0,
                KEY_MENU_DOWN => s.item_on + 1,
                _ if s.item_on == 0 => m.numitems - 1,
                _ => s.item_on - 1,
            };
            sound(d, SFX_PSTOP);
            if m.items[d.m_menu.item_on as usize].status != -1 {
                break;
            }
        },
        KEY_MENU_LEFT | KEY_MENU_RIGHT => {
            if let (Some(r), 2) = (item.routine, item.status) {
                sound(d, SFX_STNMOV);
                call(d, r, (key == KEY_MENU_RIGHT) as i32);
            }
        }
        KEY_MENU_FORWARD => {
            if let Some(r) = item.routine.filter(|_| item.status != 0) {
                menu_mut(d, d.m_menu.current).last_on = on;
                if item.status == 2 {
                    call(d, r, 1);
                    sound(d, SFX_STNMOV);
                } else {
                    call(d, r, on as i32);
                    sound(d, SFX_PISTOL);
                }
            }
        }
        KEY_MENU_ACTIVATE => {
            menu_mut(d, d.m_menu.current).last_on = on;
            clear_menus(d);
            sound(d, SFX_SWTCHX);
        }
        KEY_MENU_BACK => {
            menu_mut(d, d.m_menu.current).last_on = on;
            if let Some(prev) = m.prev {
                setup_next_menu(d, prev);
                sound(d, SFX_SWTCHN);
            }
        }
        _ => {
            if ch == 0 && ![KEY_PAUSE, 0xba, 0xc5, 0xc6].contains(&key) {
                return false;
            }
            let hit = |i: &i16| m.items[*i as usize].alpha_key as i8 as i32 == ch;
            let Some(i) = (on + 1..m.numitems)
                .find(hit)
                .or_else(|| (0..=on).find(hit))
            else {
                return false;
            };
            d.m_menu.item_on = i;
            sound(d, SFX_PSTOP);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_lines_split_on_newlines() {
        let lines = message_lines(b"are you sure?\n\npress y or n.");
        assert_eq!(
            lines,
            vec![
                b"are you sure?".to_vec(),
                Vec::new(),
                b"press y or n.".to_vec()
            ]
        );
        let long = vec![b'a'; 100];
        assert_eq!(
            message_lines(&long)
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![79, 21]
        );
    }

    #[test]
    fn copy_str_truncates_and_pads() {
        let mut dst = [b'x'; 4];
        copy_str(&mut dst, b"abcdef");
        assert_eq!(&dst, b"abc\0");
        copy_str(&mut dst, b"a\0zz");
        assert_eq!(&dst, b"a\0\0\0");
    }

    #[test]
    fn font_index_maps_hu_font() {
        assert_eq!(font_index(b'!'), Some(0));
        assert_eq!(font_index(b'a'), font_index(b'A'));
        assert_eq!(font_index(b'_'), Some(62));
        assert_eq!(font_index(b' '), None);
        assert_eq!(font_index(b'{'), None);
    }

    #[test]
    fn default_menus_match_the_data_section() {
        let m = default_menus();
        assert_eq!(m[MenuId::Options as usize].numitems, 8);
        assert_eq!(m[MenuId::NewGame as usize].last_on, 2);
        assert_eq!(
            (
                m[MenuId::ReadThis2 as usize].x,
                m[MenuId::ReadThis2 as usize].y
            ),
            (330, 175)
        );
        assert_eq!(m[MenuId::Save as usize].items[5].alpha_key, b'6');
    }
}

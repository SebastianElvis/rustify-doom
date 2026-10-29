//! Heads-up display: the message line, the automap level title, and the chat input.

use crate::d_main::{Event, EV_KEYDOWN};
use crate::g_game::MAXPLAYERS;
use crate::hu_lib::{self, IText, SText, TextLine};
use crate::i_video::{KEY_ENTER, KEY_ESCAPE, KEY_RALT, KEY_RSHIFT};
use crate::Doom;

pub const HU_FONTSTART: i32 = b'!' as i32;
pub const HU_FONTEND: i32 = b'_' as i32;
pub const HU_FONTSIZE: usize = (HU_FONTEND - HU_FONTSTART + 1) as usize;
pub const HU_BROADCAST: u8 = 5;
pub const HU_MSGX: i32 = 0;
pub const HU_MSGY: i32 = 0;
pub const HU_MSGHEIGHT: usize = 1;
pub const HU_MSGTIMEOUT: i32 = 4 * 35;
const QUEUESIZE: usize = 128;

pub const MAPNAMES: [&str; 45] = [
    "E1M1: Hangar",
    "E1M2: Nuclear Plant",
    "E1M3: Toxin Refinery",
    "E1M4: Command Control",
    "E1M5: Phobos Lab",
    "E1M6: Central Processing",
    "E1M7: Computer Station",
    "E1M8: Phobos Anomaly",
    "E1M9: Military Base",
    "E2M1: Deimos Anomaly",
    "E2M2: Containment Area",
    "E2M3: Refinery",
    "E2M4: Deimos Lab",
    "E2M5: Command Center",
    "E2M6: Halls of the Damned",
    "E2M7: Spawning Vats",
    "E2M8: Tower of Babel",
    "E2M9: Fortress of Mystery",
    "E3M1: Hell Keep",
    "E3M2: Slough of Despair",
    "E3M3: Pandemonium",
    "E3M4: House of Pain",
    "E3M5: Unholy Cathedral",
    "E3M6: Mt. Erebus",
    "E3M7: Limbo",
    "E3M8: Dis",
    "E3M9: Warrens",
    "E4M1: Hell Beneath",
    "E4M2: Perfect Hatred",
    "E4M3: Sever The Wicked",
    "E4M4: Unruly Evil",
    "E4M5: They Will Repent",
    "E4M6: Against Thee Wickedly",
    "E4M7: And Hell Followed",
    "E4M8: Unto The Cruel",
    "E4M9: Fear",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
    "NEWLEVEL",
];

/// Doom II, Plutonia, and TNT level names (32 each), in this order.
pub const MAPNAMES_COMMERCIAL: [&str; 96] = [
    "level 1: entryway",
    "level 2: underhalls",
    "level 3: the gantlet",
    "level 4: the focus",
    "level 5: the waste tunnels",
    "level 6: the crusher",
    "level 7: dead simple",
    "level 8: tricks and traps",
    "level 9: the pit",
    "level 10: refueling base",
    "level 11: 'o' of destruction!",
    "level 12: the factory",
    "level 13: downtown",
    "level 14: the inmost dens",
    "level 15: industrial zone",
    "level 16: suburbs",
    "level 17: tenements",
    "level 18: the courtyard",
    "level 19: the citadel",
    "level 20: gotcha!",
    "level 21: nirvana",
    "level 22: the catacombs",
    "level 23: barrels o' fun",
    "level 24: the chasm",
    "level 25: bloodfalls",
    "level 26: the abandoned mines",
    "level 27: monster condo",
    "level 28: the spirit world",
    "level 29: the living end",
    "level 30: icon of sin",
    "level 31: wolfenstein",
    "level 32: grosse",
    "level 1: congo",
    "level 2: well of souls",
    "level 3: aztec",
    "level 4: caged",
    "level 5: ghost town",
    "level 6: baron's lair",
    "level 7: caughtyard",
    "level 8: realm",
    "level 9: abattoire",
    "level 10: onslaught",
    "level 11: hunted",
    "level 12: speed",
    "level 13: the crypt",
    "level 14: genesis",
    "level 15: the twilight",
    "level 16: the omen",
    "level 17: compound",
    "level 18: neurosphere",
    "level 19: nme",
    "level 20: the death domain",
    "level 21: slayer",
    "level 22: impossible mission",
    "level 23: tombstone",
    "level 24: the final frontier",
    "level 25: the temple of darkness",
    "level 26: bunker",
    "level 27: anti-christ",
    "level 28: the sewers",
    "level 29: odyssey of noises",
    "level 30: the gateway of hell",
    "level 31: cyberden",
    "level 32: go 2 it",
    "level 1: system control",
    "level 2: human bbq",
    "level 3: power control",
    "level 4: wormhole",
    "level 5: hanger",
    "level 6: open season",
    "level 7: prison",
    "level 8: metal",
    "level 9: stronghold",
    "level 10: redemption",
    "level 11: storage facility",
    "level 12: crater",
    "level 13: nukage processing",
    "level 14: steel works",
    "level 15: dead zone",
    "level 16: deepest reaches",
    "level 17: processing area",
    "level 18: mill",
    "level 19: shipping/respawning",
    "level 20: central processing",
    "level 21: administration center",
    "level 22: habitat",
    "level 23: lunar mining project",
    "level 24: quarry",
    "level 25: baron's den",
    "level 26: ballistyx",
    "level 27: mount pain",
    "level 28: heck",
    "level 29: river styx",
    "level 30: last call",
    "level 31: pharaoh",
    "level 32: caribbean",
];

const PLAYER_NAMES: [&str; 4] = ["Green: ", "Indigo: ", "Brown: ", "Red: "];
const KEY_MULTI_MSG: i32 = b't' as i32;
const KEY_MULTI_MSGPLAYER: [i32; 4] = [b'g' as i32, b'i' as i32, b'b' as i32, b'r' as i32];

pub struct State {
    pub hu_font: Vec<usize>,
    pub chat_macros: [String; 10],
    w_title: TextLine,
    w_chat: IText,
    w_inputbuffer: [IText; MAXPLAYERS],
    w_message: SText,
    pub chat_on: bool,
    pub message_on: bool,
    pub message_dontfuckwithme: bool,
    message_nottobefuckedwith: bool,
    headsupactive: bool,
    message_counter: i32,
    chat_dest: [u8; MAXPLAYERS],
    chatchars: [u8; QUEUESIZE],
    head: usize,
    tail: usize,
    altdown: bool,
    num_nobrainers: i32,
    /// plr: the console player at HU_Start.
    plr: usize,
}

impl Default for State {
    fn default() -> Self {
        State {
            hu_font: Vec::new(),
            chat_macros: [
                "No",
                "I'm ready to kick butt!",
                "I'm OK.",
                "I'm not looking too good!",
                "Help!",
                "You suck!",
                "Next time, scumbag...",
                "Come here!",
                "I'll take care of it.",
                "Yes",
            ]
            .map(String::from),
            w_title: TextLine::default(),
            w_chat: IText::default(),
            w_inputbuffer: Default::default(),
            w_message: SText::default(),
            chat_on: false,
            message_on: false,
            message_dontfuckwithme: false,
            message_nottobefuckedwith: false,
            headsupactive: false,
            message_counter: 0,
            chat_dest: [0; MAXPLAYERS],
            chatchars: [0; QUEUESIZE],
            head: 0,
            tail: 0,
            altdown: false,
            num_nobrainers: 0,
            plr: 0,
        }
    }
}

/// The game mission with Chex Quest as Doom and HACX as Doom II.
pub fn logical_gamemission(d: &Doom) -> i32 {
    use crate::d_main::*;
    match d.d_main.gamemission {
        MISSION_PACK_CHEX => MISSION_DOOM,
        MISSION_PACK_HACX => MISSION_DOOM2,
        m => m,
    }
}

/// HU_Init: loads the heads-up font STCFN033 to STCFN095.
pub fn init(d: &mut Doom) {
    let mut font = Vec::with_capacity(HU_FONTSIZE);
    for j in HU_FONTSTART..=HU_FONTEND {
        font.push(crate::w_wad::get_num_for_name(d, &format!("STCFN{:03}", j)));
    }
    d.hu_stuff.hu_font = font;
}

fn stop(d: &mut Doom) {
    d.hu_stuff.headsupactive = false;
}

pub fn start(d: &mut Doom) {
    use crate::d_main::*;
    if d.hu_stuff.headsupactive {
        stop(d);
    }
    let h = &mut d.hu_stuff;
    h.plr = d.g_game.consoleplayer;
    h.message_on = false;
    h.message_dontfuckwithme = false;
    h.message_nottobefuckedwith = false;
    h.chat_on = false;
    let font = h.hu_font.clone();
    let mut msg = SText::default();
    hu_lib::init_s_text(
        d,
        &mut msg,
        HU_MSGX,
        HU_MSGY,
        HU_MSGHEIGHT,
        &font,
        HU_FONTSTART,
    );
    let fh = hu_lib::patch_height(d, font[0]);
    let mut title = TextLine::default();
    hu_lib::init_text_line(&mut title, 0, 167 - fh, &font, HU_FONTSTART);
    let (ep, map) = (d.g_game.gameepisode, d.g_game.gamemap);
    let s: &str = match logical_gamemission(d) {
        MISSION_DOOM => MAPNAMES[((ep - 1) * 9 + map - 1) as usize],
        MISSION_DOOM2 => {
            if d.d_main.gameversion <= EXE_DOOM_1_9 && map >= 33 {
                ""
            } else {
                MAPNAMES_COMMERCIAL[(map - 1) as usize]
            }
        }
        MISSION_PACK_PLUT => MAPNAMES_COMMERCIAL[(map - 1 + 32) as usize],
        MISSION_PACK_TNT => MAPNAMES_COMMERCIAL[(map - 1 + 64) as usize],
        _ => "Unknown level",
    };
    let s = if logical_gamemission(d) == MISSION_DOOM && d.d_main.gameversion == EXE_CHEX {
        MAPNAMES[(map - 1) as usize]
    } else {
        s
    };
    for &c in s.as_bytes() {
        hu_lib::add_char_to_text_line(&mut title, c);
    }
    let mut chat = IText::default();
    hu_lib::init_i_text(
        &mut chat,
        HU_MSGX,
        HU_MSGY + HU_MSGHEIGHT as i32 * (fh + 1),
        &font,
        HU_FONTSTART,
    );
    let h = &mut d.hu_stuff;
    h.w_message = msg;
    h.w_title = title;
    h.w_chat = chat;
    for i in 0..MAXPLAYERS {
        hu_lib::init_i_text(&mut h.w_inputbuffer[i], 0, 0, &[], 0);
    }
    h.headsupactive = true;
}

pub fn drawer(d: &mut Doom) {
    let msg = d.hu_stuff.w_message.clone();
    let on = d.hu_stuff.message_on;
    hu_lib::draw_s_text(d, &msg, on);
    let chat = d.hu_stuff.w_chat.clone();
    let on = d.hu_stuff.chat_on;
    hu_lib::draw_i_text(d, &chat, on);
    if d.am_map.automapactive {
        let title = d.hu_stuff.w_title.clone();
        hu_lib::draw_text_line(d, &title, false);
    }
}

pub fn erase(d: &mut Doom) {
    let mut msg = std::mem::take(&mut d.hu_stuff.w_message);
    let on = d.hu_stuff.message_on;
    hu_lib::erase_s_text(d, &mut msg, on);
    d.hu_stuff.w_message = msg;
    let mut chat = std::mem::take(&mut d.hu_stuff.w_chat);
    let on = d.hu_stuff.chat_on;
    hu_lib::erase_i_text(d, &mut chat, on);
    d.hu_stuff.w_chat = chat;
    let mut title = std::mem::take(&mut d.hu_stuff.w_title);
    hu_lib::erase_text_line(d, &mut title);
    d.hu_stuff.w_title = title;
}

pub fn ticker(d: &mut Doom) {
    let h = &mut d.hu_stuff;
    if h.message_counter != 0 {
        h.message_counter -= 1;
        if h.message_counter == 0 {
            h.message_on = false;
            h.message_nottobefuckedwith = false;
        }
    }
    let cp = d.g_game.consoleplayer;
    if d.m_menu.show_messages != 0 || d.hu_stuff.message_dontfuckwithme {
        let h = &mut d.hu_stuff;
        let has = d.g_game.players[h.plr].message.is_some();
        if (has && !h.message_nottobefuckedwith) || (has && h.message_dontfuckwithme) {
            let m = d.g_game.players[h.plr].message.take().unwrap();
            hu_lib::add_message_to_s_text(&mut h.w_message, None, &m);
            h.message_on = true;
            h.message_counter = HU_MSGTIMEOUT;
            h.message_nottobefuckedwith = h.message_dontfuckwithme;
            h.message_dontfuckwithme = false;
        }
    }
    if d.g_game.netgame {
        for i in 0..MAXPLAYERS {
            if !d.g_game.playeringame[i] {
                continue;
            }
            let c = d.g_game.players[i].cmd.chatchar;
            if i != cp && c != 0 {
                if (c as i8) <= HU_BROADCAST as i8 {
                    d.hu_stuff.chat_dest[i] = c;
                } else {
                    let rc = hu_lib::key_in_i_text(&mut d.hu_stuff.w_inputbuffer[i], c);
                    if rc && c as i32 == KEY_ENTER {
                        let h = &mut d.hu_stuff;
                        if !h.w_inputbuffer[i].l.l.is_empty()
                            && (h.chat_dest[i] as usize == cp + 1 || h.chat_dest[i] == HU_BROADCAST)
                        {
                            let text =
                                String::from_utf8_lossy(&h.w_inputbuffer[i].l.l).into_owned();
                            hu_lib::add_message_to_s_text(
                                &mut h.w_message,
                                Some(PLAYER_NAMES[i]),
                                &text,
                            );
                            h.message_nottobefuckedwith = true;
                            h.message_on = true;
                            h.message_counter = HU_MSGTIMEOUT;
                            let sfx = if d.d_main.gamemode == crate::d_main::COMMERCIAL {
                                crate::s_sound::SFX_RADIO
                            } else {
                                crate::s_sound::SFX_TINK
                            };
                            crate::s_sound::start_sound(d, None, sfx);
                        }
                        hu_lib::reset_i_text(&mut d.hu_stuff.w_inputbuffer[i]);
                    }
                }
                d.g_game.players[i].cmd.chatchar = 0;
            }
        }
    }
}

fn queue_chat_char(d: &mut Doom, c: u8) {
    let h = &mut d.hu_stuff;
    if ((h.head + 1) & (QUEUESIZE - 1)) == h.tail {
        d.g_game.players[h.plr].message = Some("[Message unsent]".to_string());
    } else {
        h.chatchars[h.head] = c;
        h.head = (h.head + 1) & (QUEUESIZE - 1);
    }
}

/// HU_dequeueChatChar. G_BuildTiccmd calls it.
pub fn dequeue_chat_char(d: &mut Doom) -> u8 {
    let h = &mut d.hu_stuff;
    if h.head != h.tail {
        let c = h.chatchars[h.tail];
        h.tail = (h.tail + 1) & (QUEUESIZE - 1);
        c
    } else {
        0
    }
}

pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    let mut eatkey = false;
    let numplayers = d.g_game.playeringame.iter().filter(|&&p| p).count();
    let cp = d.g_game.consoleplayer;
    if ev.data1 == KEY_RSHIFT {
        return false;
    } else if ev.data1 == KEY_RALT {
        d.hu_stuff.altdown = ev.type_ == EV_KEYDOWN;
        return false;
    }
    if ev.type_ != EV_KEYDOWN {
        return false;
    }
    if !d.hu_stuff.chat_on {
        if ev.data1 == KEY_ENTER {
            d.hu_stuff.message_on = true;
            d.hu_stuff.message_counter = HU_MSGTIMEOUT;
            eatkey = true;
        } else if d.g_game.netgame && ev.data2 == KEY_MULTI_MSG {
            eatkey = true;
            d.hu_stuff.chat_on = true;
            hu_lib::reset_i_text(&mut d.hu_stuff.w_chat);
            queue_chat_char(d, HU_BROADCAST);
        } else if d.g_game.netgame && numplayers > 2 {
            for i in 0..MAXPLAYERS {
                if ev.data2 == KEY_MULTI_MSGPLAYER[i] {
                    if d.g_game.playeringame[i] && i != cp {
                        eatkey = true;
                        d.hu_stuff.chat_on = true;
                        hu_lib::reset_i_text(&mut d.hu_stuff.w_chat);
                        queue_chat_char(d, i as u8 + 1);
                        break;
                    } else if i == cp {
                        d.hu_stuff.num_nobrainers += 1;
                        let n = d.hu_stuff.num_nobrainers;
                        let m = if n < 3 {
                            "You mumble to yourself"
                        } else if n < 6 {
                            "Who's there?"
                        } else if n < 9 {
                            "You scare yourself"
                        } else if n < 32 {
                            "You start to rave"
                        } else {
                            "You've lost it..."
                        };
                        d.g_game.players[d.hu_stuff.plr].message = Some(m.to_string());
                    }
                }
            }
        }
    } else if d.hu_stuff.altdown {
        let c = (ev.data1 - b'0' as i32) as u8;
        if c > 9 {
            return false;
        }
        let macromessage = d.hu_stuff.chat_macros[c as usize].clone();
        queue_chat_char(d, KEY_ENTER as u8);
        for &b in macromessage.as_bytes() {
            queue_chat_char(d, b);
        }
        queue_chat_char(d, KEY_ENTER as u8);
        d.hu_stuff.chat_on = false;
        let mut last = macromessage;
        last.truncate(hu_lib::HU_MAXLINELENGTH);
        d.g_game.players[d.hu_stuff.plr].message = Some(last);
        eatkey = true;
    } else {
        let c = ev.data2 as u8;
        eatkey = hu_lib::key_in_i_text(&mut d.hu_stuff.w_chat, c);
        if eatkey {
            queue_chat_char(d, c);
        }
        if c as i32 == KEY_ENTER {
            d.hu_stuff.chat_on = false;
            if !d.hu_stuff.w_chat.l.l.is_empty() {
                let last = String::from_utf8_lossy(&d.hu_stuff.w_chat.l.l).into_owned();
                d.g_game.players[d.hu_stuff.plr].message = Some(last);
            }
        } else if c as i32 == KEY_ESCAPE {
            d.hu_stuff.chat_on = false;
        }
    }
    eatkey
}

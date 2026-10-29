//! Status bar: widgets, face animation, palette effects, and the cheat responder.

use crate::d_items::{
    NUMAMMO, NUMCARDS, NUMWEAPONS, PW_INVULNERABILITY, PW_IRONFEET, PW_STRENGTH, WEAPONINFO,
};
use crate::d_main::{Event, EV_KEYDOWN, EV_KEYUP};
use crate::g_game::{CF_GODMODE, CF_NOCLIP, MAXPLAYERS};
use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::m_cheat::{check_cheat, get_param, CheatSeq};
use crate::st_lib::{self, StBinIcon, StMultIcon, StNumber, StPercent};
use crate::tables::{ANG180, ANG45};
use crate::Doom;

pub const ST_HEIGHT: i32 = 32;
pub const ST_WIDTH: i32 = SCREENWIDTH as i32;
pub const ST_Y: i32 = SCREENHEIGHT as i32 - ST_HEIGHT;
const ST_X: i32 = 0;
const ST_FX: i32 = 143;

const ST_NUMPAINFACES: i32 = 5;
const ST_NUMSTRAIGHTFACES: i32 = 3;
const ST_FACESTRIDE: i32 = 8;
const ST_NUMFACES: usize = 42;
const ST_TURNOFFSET: i32 = ST_NUMSTRAIGHTFACES;
const ST_OUCHOFFSET: i32 = ST_TURNOFFSET + 2;
const ST_EVILGRINOFFSET: i32 = ST_OUCHOFFSET + 1;
const ST_RAMPAGEOFFSET: i32 = ST_EVILGRINOFFSET + 1;
const ST_GODFACE: i32 = ST_NUMPAINFACES * ST_FACESTRIDE;
const ST_DEADFACE: i32 = ST_GODFACE + 1;
const ST_EVILGRINCOUNT: i32 = 2 * 35;
const ST_STRAIGHTFACECOUNT: i32 = 35 / 2;
const ST_TURNCOUNT: i32 = 35;
const ST_RAMPAGEDELAY: i32 = 2 * 35;
const ST_MUCHPAIN: i32 = 20;

const STARTREDPALS: i32 = 1;
const NUMREDPALS: i32 = 8;
const STARTBONUSPALS: i32 = 9;
const NUMBONUSPALS: i32 = 4;
const RADIATIONPAL: i32 = 13;

/// AM_MSGENTERED: the automap posts it as a key-up event.
pub const AM_MSGENTERED: i32 = ((b'a' as i32) << 24) | ((b'm' as i32) << 16) | ((b'e' as i32) << 8);

/// The ammunition source of the ready-weapon number.
/// 0..3 read `ammo`, 4..7 read `maxammo`, and -1 shows nothing (1994).
type AmmoSrc = i32;

pub struct State {
    pub st_backing_screen: Vec<u8>,
    plyr: usize,
    st_stopped: bool,
    st_firsttime: bool,
    st_clock: i32,
    st_msgcounter: i32,
    st_statusbaron: bool,
    st_chat: bool,
    st_oldchat: bool,
    st_notdeathmatch: bool,
    st_armson: bool,
    st_fragson: bool,
    lu_palette: usize,
    sbar: usize,
    tallnum: Vec<usize>,
    tallpercent: usize,
    shortnum: Vec<usize>,
    keys: Vec<usize>,
    faces: Vec<usize>,
    faceback: usize,
    armsbg: usize,
    arms: Vec<[usize; 2]>,
    w_ready: StNumber,
    w_ready_src: AmmoSrc,
    w_frags: StNumber,
    w_health: StPercent,
    w_armsbg: StBinIcon,
    w_arms: Vec<StMultIcon>,
    w_faces: StMultIcon,
    w_keyboxes: Vec<StMultIcon>,
    w_armor: StPercent,
    w_ammo: Vec<StNumber>,
    w_maxammo: Vec<StNumber>,
    st_fragscount: i32,
    st_oldhealth: i32,
    oldweaponsowned: [bool; NUMWEAPONS],
    st_facecount: i32,
    st_faceindex: i32,
    keyboxes: [i32; 3],
    st_randomnumber: i32,
    st_palette: i32,
    lastattackdown: i32,
    priority: i32,
    lastcalc: i32,
    oldhealth: i32,
    cheat_mus: CheatSeq,
    cheat_god: CheatSeq,
    cheat_ammo: CheatSeq,
    cheat_ammonokey: CheatSeq,
    cheat_noclip: CheatSeq,
    cheat_commercial_noclip: CheatSeq,
    cheat_powerup: [CheatSeq; 7],
    cheat_choppers: CheatSeq,
    cheat_clev: CheatSeq,
    cheat_mypos: CheatSeq,
}

impl Default for State {
    fn default() -> Self {
        State {
            st_backing_screen: vec![0; SCREENWIDTH * SCREENHEIGHT],
            plyr: 0,
            st_stopped: true,
            st_firsttime: false,
            st_clock: 0,
            st_msgcounter: 0,
            st_statusbaron: false,
            st_chat: false,
            st_oldchat: false,
            st_notdeathmatch: false,
            st_armson: false,
            st_fragson: false,
            lu_palette: 0,
            sbar: 0,
            tallnum: Vec::new(),
            tallpercent: 0,
            shortnum: Vec::new(),
            keys: Vec::new(),
            faces: Vec::new(),
            faceback: 0,
            armsbg: 0,
            arms: Vec::new(),
            w_ready: StNumber::default(),
            w_ready_src: 0,
            w_frags: StNumber::default(),
            w_health: StPercent::default(),
            w_armsbg: StBinIcon::default(),
            w_arms: Vec::new(),
            w_faces: StMultIcon::default(),
            w_keyboxes: Vec::new(),
            w_armor: StPercent::default(),
            w_ammo: Vec::new(),
            w_maxammo: Vec::new(),
            st_fragscount: 0,
            st_oldhealth: -1,
            oldweaponsowned: [false; NUMWEAPONS],
            st_facecount: 0,
            st_faceindex: 0,
            keyboxes: [-1; 3],
            st_randomnumber: 0,
            st_palette: 0,
            lastattackdown: -1,
            priority: 0,
            lastcalc: 0,
            oldhealth: -1,
            cheat_mus: CheatSeq::new("idmus", 2),
            cheat_god: CheatSeq::new("iddqd", 0),
            cheat_ammo: CheatSeq::new("idkfa", 0),
            cheat_ammonokey: CheatSeq::new("idfa", 0),
            cheat_noclip: CheatSeq::new("idspispopd", 0),
            cheat_commercial_noclip: CheatSeq::new("idclip", 0),
            cheat_powerup: [
                CheatSeq::new("idbeholdv", 0),
                CheatSeq::new("idbeholds", 0),
                CheatSeq::new("idbeholdi", 0),
                CheatSeq::new("idbeholdr", 0),
                CheatSeq::new("idbeholda", 0),
                CheatSeq::new("idbeholdl", 0),
                CheatSeq::new("idbehold", 0),
            ],
            cheat_choppers: CheatSeq::new("idchoppers", 0),
            cheat_clev: CheatSeq::new("idclev", 2),
            cheat_mypos: CheatSeq::new("idmypos", 0),
        }
    }
}

fn ammo_value(d: &Doom, src: AmmoSrc) -> i32 {
    let p = &d.g_game.players[d.st_stuff.plyr];
    match src {
        0..=3 => p.ammo[src as usize],
        4..=7 => p.maxammo[(src - 4) as usize],
        _ => 1994,
    }
}

/// The ammo slot of a weapon. The original indexes past `ammo` for am_noammo and reads maxammo[1].
fn ready_src(ammo: i32) -> AmmoSrc {
    if (ammo as usize) < NUMAMMO {
        ammo
    } else {
        ammo - NUMAMMO as i32 + 4
    }
}

pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    if ev.type_ == EV_KEYUP
        && (ev.data1 as u32 & 0xffff_0000) == (AM_MSGENTERED as u32 & 0xffff_0000)
    {
        if ev.data1 == AM_MSGENTERED {
            d.st_stuff.st_firsttime = true;
        }
        return false;
    }
    if ev.type_ != EV_KEYDOWN {
        return false;
    }
    let key = ev.data2 as u8;
    let p = d.st_stuff.plyr;
    if !d.g_game.netgame && d.g_game.gameskill != crate::g_game::SK_NIGHTMARE {
        let st = &mut d.st_stuff;
        if check_cheat(&mut st.cheat_god, key) {
            let pl = &mut d.g_game.players[p];
            pl.cheats ^= CF_GODMODE;
            if pl.cheats & CF_GODMODE != 0 {
                if let Some(mo) = pl.mo {
                    d.p_tick.mobj_mut(mo).health = 100;
                }
                let pl = &mut d.g_game.players[p];
                pl.health = 100;
                pl.message = Some("Degreelessness Mode On".into());
            } else {
                pl.message = Some("Degreelessness Mode Off".into());
            }
        } else if check_cheat(&mut st.cheat_ammonokey, key) {
            let pl = &mut d.g_game.players[p];
            pl.armorpoints = 200;
            pl.armortype = 2;
            pl.weaponowned = [true; NUMWEAPONS];
            pl.ammo = pl.maxammo;
            pl.message = Some("Ammo (no keys) Added".into());
        } else if check_cheat(&mut st.cheat_ammo, key) {
            let pl = &mut d.g_game.players[p];
            pl.armorpoints = 200;
            pl.armortype = 2;
            pl.weaponowned = [true; NUMWEAPONS];
            pl.ammo = pl.maxammo;
            pl.cards = [true; NUMCARDS];
            pl.message = Some("Very Happy Ammo Added".into());
        } else if check_cheat(&mut st.cheat_mus, key) {
            d.g_game.players[p].message = Some("Music Change".into());
            let buf = get_param(&d.st_stuff.cheat_mus);
            let (b0, b1) = (buf[0] as i8 as i32, buf[1] as i8 as i32);
            if d.d_main.gamemode == crate::d_main::COMMERCIAL
                || d.d_main.gameversion < crate::d_main::EXE_ULTIMATE
            {
                let n = (b0 - b'0' as i32) * 10 + b1 - b'0' as i32;
                if n > 35 {
                    d.g_game.players[p].message = Some("IMPOSSIBLE SELECTION".into());
                } else {
                    // mus_runnin is 33.
                    crate::s_sound::change_music(d, 33 + n - 1, true);
                }
            } else {
                let n = (b0 - b'1' as i32) * 9 + b1 - b'1' as i32;
                if n > 31 {
                    d.g_game.players[p].message = Some("IMPOSSIBLE SELECTION".into());
                } else {
                    // mus_e1m1 is 1.
                    crate::s_sound::change_music(d, 1 + n, true);
                }
            }
        } else {
            let doom = crate::hu_stuff::logical_gamemission(d) == crate::d_main::MISSION_DOOM;
            let st = &mut d.st_stuff;
            let hit = if doom {
                check_cheat(&mut st.cheat_noclip, key)
            } else {
                check_cheat(&mut st.cheat_commercial_noclip, key)
            };
            if hit {
                let pl = &mut d.g_game.players[p];
                pl.cheats ^= CF_NOCLIP;
                pl.message = Some(
                    if pl.cheats & CF_NOCLIP != 0 {
                        "No Clipping Mode ON"
                    } else {
                        "No Clipping Mode OFF"
                    }
                    .into(),
                );
            }
        }
        for i in 0..6 {
            if check_cheat(&mut d.st_stuff.cheat_powerup[i], key) {
                if d.g_game.players[p].powers[i] == 0 {
                    crate::p_inter::give_power(d, p, i);
                } else if i != PW_STRENGTH {
                    d.g_game.players[p].powers[i] = 1;
                } else {
                    d.g_game.players[p].powers[i] = 0;
                }
                d.g_game.players[p].message = Some("Power-up Toggled".into());
            }
        }
        let st = &mut d.st_stuff;
        if check_cheat(&mut st.cheat_powerup[6], key) {
            d.g_game.players[p].message =
                Some("inVuln, Str, Inviso, Rad, Allmap, or Lite-amp".into());
        } else if check_cheat(&mut st.cheat_choppers, key) {
            let pl = &mut d.g_game.players[p];
            pl.weaponowned[crate::d_items::WP_CHAINSAW as usize] = true;
            pl.powers[PW_INVULNERABILITY] = 1;
            pl.message = Some("... doesn't suck - GM".into());
        } else if check_cheat(&mut st.cheat_mypos, key) {
            let cp = d.g_game.consoleplayer;
            let mo = d.p_tick.mobj(d.g_game.players[cp].mo.unwrap());
            let mut s = format!(
                "ang=0x{:x};x,y=(0x{:x},0x{:x})",
                mo.angle, mo.x as u32, mo.y as u32
            );
            s.truncate(51);
            d.g_game.players[p].message = Some(s);
        }
    }
    if !d.g_game.netgame && check_cheat(&mut d.st_stuff.cheat_clev, key) {
        let buf = get_param(&d.st_stuff.cheat_clev);
        let (b0, b1) = (buf[0] as i8 as i32, buf[1] as i8 as i32);
        use crate::d_main::*;
        let gm = d.d_main.gamemode;
        let (mut epsd, map) = if gm == COMMERCIAL {
            (1, (b0 - b'0' as i32) * 10 + b1 - b'0' as i32)
        } else {
            (b0 - b'0' as i32, b1 - b'0' as i32)
        };
        if d.d_main.gameversion == EXE_CHEX {
            epsd = 1;
        }
        if epsd < 1 || map < 1 {
            return false;
        }
        if gm == RETAIL && (epsd > 4 || map > 9) {
            return false;
        }
        if gm == REGISTERED && (epsd > 3 || map > 9) {
            return false;
        }
        if gm == SHAREWARE && (epsd > 1 || map > 9) {
            return false;
        }
        if gm == COMMERCIAL && (epsd > 1 || map > 40) {
            return false;
        }
        d.g_game.players[p].message = Some("Changing Level...".into());
        let skill = d.g_game.gameskill;
        crate::g_game::defered_init_new(d, skill, epsd, map);
    }
    false
}

fn calc_pain_offset(d: &mut Doom) -> i32 {
    let h = d.g_game.players[d.st_stuff.plyr].health;
    let health = if h > 100 { 100 } else { h };
    let st = &mut d.st_stuff;
    if health != st.oldhealth {
        st.lastcalc = ST_FACESTRIDE * (((100 - health) * ST_NUMPAINFACES) / 101);
        st.oldhealth = health;
    }
    st.lastcalc
}

fn update_face_widget(d: &mut Doom) {
    let p = d.st_stuff.plyr;
    if d.st_stuff.priority < 10 && d.g_game.players[p].health == 0 {
        d.st_stuff.priority = 9;
        d.st_stuff.st_faceindex = ST_DEADFACE;
        d.st_stuff.st_facecount = 1;
    }
    if d.st_stuff.priority < 9 && d.g_game.players[p].bonuscount != 0 {
        let mut doevilgrin = false;
        for i in 0..NUMWEAPONS {
            if d.st_stuff.oldweaponsowned[i] != d.g_game.players[p].weaponowned[i] {
                doevilgrin = true;
                d.st_stuff.oldweaponsowned[i] = d.g_game.players[p].weaponowned[i];
            }
        }
        if doevilgrin {
            d.st_stuff.priority = 8;
            d.st_stuff.st_facecount = ST_EVILGRINCOUNT;
            d.st_stuff.st_faceindex = calc_pain_offset(d) + ST_EVILGRINOFFSET;
        }
    }
    if d.st_stuff.priority < 8 {
        let pl = &d.g_game.players[p];
        if pl.damagecount != 0 && pl.attacker.is_some() && pl.attacker != pl.mo {
            d.st_stuff.priority = 7;
            if pl.health - d.st_stuff.st_oldhealth > ST_MUCHPAIN {
                d.st_stuff.st_facecount = ST_TURNCOUNT;
                d.st_stuff.st_faceindex = calc_pain_offset(d) + ST_OUCHOFFSET;
            } else {
                let mo = d.p_tick.mobj(pl.mo.unwrap());
                let at = d.p_tick.mobj(pl.attacker.unwrap());
                let badguyangle = crate::r_main::point_to_angle2(mo.x, mo.y, at.x, at.y);
                let (diffang, i) = if badguyangle > mo.angle {
                    let diff = badguyangle.wrapping_sub(mo.angle);
                    (diff, diff > ANG180)
                } else {
                    let diff = mo.angle.wrapping_sub(badguyangle);
                    (diff, diff <= ANG180)
                };
                d.st_stuff.st_facecount = ST_TURNCOUNT;
                let mut idx = calc_pain_offset(d);
                if diffang < ANG45 {
                    idx += ST_RAMPAGEOFFSET;
                } else if i {
                    idx += ST_TURNOFFSET;
                } else {
                    idx += ST_TURNOFFSET + 1;
                }
                d.st_stuff.st_faceindex = idx;
            }
        }
    }
    if d.st_stuff.priority < 7 && d.g_game.players[p].damagecount != 0 {
        if d.g_game.players[p].health - d.st_stuff.st_oldhealth > ST_MUCHPAIN {
            d.st_stuff.priority = 7;
            d.st_stuff.st_facecount = ST_TURNCOUNT;
            d.st_stuff.st_faceindex = calc_pain_offset(d) + ST_OUCHOFFSET;
        } else {
            d.st_stuff.priority = 6;
            d.st_stuff.st_facecount = ST_TURNCOUNT;
            d.st_stuff.st_faceindex = calc_pain_offset(d) + ST_RAMPAGEOFFSET;
        }
    }
    if d.st_stuff.priority < 6 {
        if d.g_game.players[p].attackdown {
            if d.st_stuff.lastattackdown == -1 {
                d.st_stuff.lastattackdown = ST_RAMPAGEDELAY;
            } else {
                d.st_stuff.lastattackdown -= 1;
                if d.st_stuff.lastattackdown == 0 {
                    d.st_stuff.priority = 5;
                    d.st_stuff.st_faceindex = calc_pain_offset(d) + ST_RAMPAGEOFFSET;
                    d.st_stuff.st_facecount = 1;
                    d.st_stuff.lastattackdown = 1;
                }
            }
        } else {
            d.st_stuff.lastattackdown = -1;
        }
    }
    if d.st_stuff.priority < 5 {
        let pl = &d.g_game.players[p];
        if pl.cheats & CF_GODMODE != 0 || pl.powers[PW_INVULNERABILITY] != 0 {
            d.st_stuff.priority = 4;
            d.st_stuff.st_faceindex = ST_GODFACE;
            d.st_stuff.st_facecount = 1;
        }
    }
    if d.st_stuff.st_facecount == 0 {
        d.st_stuff.st_faceindex = calc_pain_offset(d) + (d.st_stuff.st_randomnumber % 3);
        d.st_stuff.st_facecount = ST_STRAIGHTFACECOUNT;
        d.st_stuff.priority = 0;
    }
    d.st_stuff.st_facecount -= 1;
}

fn update_widgets(d: &mut Doom) {
    let p = d.st_stuff.plyr;
    let pl = &d.g_game.players[p];
    let ammo = WEAPONINFO[pl.readyweapon as usize].ammo;
    d.st_stuff.w_ready_src = if ammo == crate::d_items::AM_NOAMMO {
        -1
    } else {
        ammo
    };
    for i in 0..3 {
        let mut k = if pl.cards[i] { i as i32 } else { -1 };
        if pl.cards[i + 3] {
            k = i as i32 + 3;
        }
        d.st_stuff.keyboxes[i] = k;
    }
    update_face_widget(d);
    let st = &mut d.st_stuff;
    st.st_notdeathmatch = d.g_game.deathmatch == 0;
    st.st_armson = st.st_statusbaron && d.g_game.deathmatch == 0;
    st.st_fragson = d.g_game.deathmatch != 0 && st.st_statusbaron;
    st.st_fragscount = 0;
    let pl = &d.g_game.players[p];
    for i in 0..MAXPLAYERS {
        if i != d.g_game.consoleplayer {
            st.st_fragscount += pl.frags[i];
        } else {
            st.st_fragscount -= pl.frags[i];
        }
    }
    st.st_msgcounter -= 1;
    if st.st_msgcounter == 0 {
        st.st_chat = st.st_oldchat;
    }
}

pub fn ticker(d: &mut Doom) {
    d.st_stuff.st_clock += 1;
    d.st_stuff.st_randomnumber = crate::m_random::m_random(&mut d.m_random);
    update_widgets(d);
    d.st_stuff.st_oldhealth = d.g_game.players[d.st_stuff.plyr].health;
}

fn set_palette_num(d: &mut Doom, palette: i32) {
    let lump = crate::w_wad::lump(&d.w_wad, d.st_stuff.lu_palette);
    crate::i_video::set_palette(&mut d.i_video, &lump[palette as usize * 768..]);
}

fn do_palette_stuff(d: &mut Doom) {
    let pl = &d.g_game.players[d.st_stuff.plyr];
    let mut cnt = pl.damagecount;
    if pl.powers[PW_STRENGTH] != 0 {
        let bzc = 12 - (pl.powers[PW_STRENGTH] >> 6);
        if bzc > cnt {
            cnt = bzc;
        }
    }
    let mut palette = if cnt != 0 {
        ((cnt + 7) >> 3).min(NUMREDPALS - 1) + STARTREDPALS
    } else if pl.bonuscount != 0 {
        ((pl.bonuscount + 7) >> 3).min(NUMBONUSPALS - 1) + STARTBONUSPALS
    } else if pl.powers[PW_IRONFEET] > 4 * 32 || pl.powers[PW_IRONFEET] & 8 != 0 {
        RADIATIONPAL
    } else {
        0
    };
    if d.d_main.gameversion == crate::d_main::EXE_CHEX
        && (STARTREDPALS..STARTREDPALS + NUMREDPALS).contains(&palette)
    {
        palette = RADIATIONPAL;
    }
    if palette != d.st_stuff.st_palette {
        d.st_stuff.st_palette = palette;
        set_palette_num(d, palette);
    }
}

/// Runs `f` with `buf` as the drawing screen (V_UseBuffer and V_RestoreBuffer).
fn with_buffer(d: &mut Doom, buf: &mut Vec<u8>, f: impl FnOnce(&mut Doom)) {
    std::mem::swap(&mut d.i_video.video_buffer, buf);
    f(d);
    std::mem::swap(&mut d.i_video.video_buffer, buf);
}

fn refresh_background(d: &mut Doom) {
    if d.st_stuff.st_statusbaron {
        let (sbar, faceback, net) = (d.st_stuff.sbar, d.st_stuff.faceback, d.g_game.netgame);
        let mut backing = std::mem::take(&mut d.st_stuff.st_backing_screen);
        with_buffer(d, &mut backing, |d| {
            crate::v_video::draw_patch(d, ST_X, 0, sbar);
            if net {
                crate::v_video::draw_patch(d, ST_FX, 0, faceback);
            }
        });
        crate::v_video::copy_rect(d, ST_X, 0, &backing, ST_WIDTH, ST_HEIGHT, ST_X, ST_Y);
        d.st_stuff.st_backing_screen = backing;
    }
}

fn draw_widgets(d: &mut Doom, refresh: bool) {
    let st = &mut d.st_stuff;
    st.st_armson = st.st_statusbaron && d.g_game.deathmatch == 0;
    st.st_fragson = d.g_game.deathmatch != 0 && st.st_statusbaron;
    let on = st.st_statusbaron;
    let p = st.plyr;

    let mut w = std::mem::take(&mut d.st_stuff.w_ready);
    let v = ammo_value(d, d.st_stuff.w_ready_src);
    st_lib::update_num(d, &mut w, v, on);
    d.st_stuff.w_ready = w;
    for i in 0..4 {
        let mut w = std::mem::take(&mut d.st_stuff.w_ammo[i]);
        let v = d.g_game.players[p].ammo[i];
        st_lib::update_num(d, &mut w, v, on);
        d.st_stuff.w_ammo[i] = w;
        let mut w = std::mem::take(&mut d.st_stuff.w_maxammo[i]);
        let v = d.g_game.players[p].maxammo[i];
        st_lib::update_num(d, &mut w, v, on);
        d.st_stuff.w_maxammo[i] = w;
    }
    let mut w = std::mem::take(&mut d.st_stuff.w_health);
    let v = d.g_game.players[p].health;
    st_lib::update_percent(d, &mut w, v, on, refresh);
    d.st_stuff.w_health = w;
    let mut w = std::mem::take(&mut d.st_stuff.w_armor);
    let v = d.g_game.players[p].armorpoints;
    st_lib::update_percent(d, &mut w, v, on, refresh);
    d.st_stuff.w_armor = w;

    let mut w = std::mem::take(&mut d.st_stuff.w_armsbg);
    let v = d.st_stuff.st_notdeathmatch;
    st_lib::update_bin_icon(d, &mut w, v, on, refresh);
    d.st_stuff.w_armsbg = w;
    for i in 0..6 {
        let mut w = std::mem::take(&mut d.st_stuff.w_arms[i]);
        let v = d.g_game.players[p].weaponowned[i + 1] as i32;
        let armson = d.st_stuff.st_armson;
        st_lib::update_mult_icon(d, &mut w, v, armson, refresh);
        d.st_stuff.w_arms[i] = w;
    }
    let mut w = std::mem::take(&mut d.st_stuff.w_faces);
    let v = d.st_stuff.st_faceindex;
    st_lib::update_mult_icon(d, &mut w, v, on, refresh);
    d.st_stuff.w_faces = w;
    for i in 0..3 {
        let mut w = std::mem::take(&mut d.st_stuff.w_keyboxes[i]);
        let v = d.st_stuff.keyboxes[i];
        st_lib::update_mult_icon(d, &mut w, v, on, refresh);
        d.st_stuff.w_keyboxes[i] = w;
    }
    let mut w = std::mem::take(&mut d.st_stuff.w_frags);
    let (v, fon) = (d.st_stuff.st_fragscount, d.st_stuff.st_fragson);
    st_lib::update_num(d, &mut w, v, fon);
    d.st_stuff.w_frags = w;
}

fn do_refresh(d: &mut Doom) {
    d.st_stuff.st_firsttime = false;
    refresh_background(d);
    draw_widgets(d, true);
}

pub fn drawer(d: &mut Doom, fullscreen: bool, refresh: bool) {
    d.st_stuff.st_statusbaron = !fullscreen || d.am_map.automapactive;
    d.st_stuff.st_firsttime = d.st_stuff.st_firsttime || refresh;
    do_palette_stuff(d);
    if d.st_stuff.st_firsttime {
        do_refresh(d);
    } else {
        draw_widgets(d, false);
    }
}

fn load_graphics(d: &mut Doom) {
    let n = |d: &mut Doom, name: String| crate::w_wad::get_num_for_name(d, &name);
    let mut tallnum = Vec::new();
    let mut shortnum = Vec::new();
    for i in 0..10 {
        tallnum.push(n(d, format!("STTNUM{i}")));
        shortnum.push(n(d, format!("STYSNUM{i}")));
    }
    let tallpercent = n(d, "STTPRCNT".into());
    let keys: Vec<usize> = (0..NUMCARDS).map(|i| n(d, format!("STKEYS{i}"))).collect();
    let armsbg = n(d, "STARMS".into());
    let mut arms = Vec::new();
    for i in 0..6 {
        arms.push([n(d, format!("STGNUM{}", i + 2)), shortnum[i + 2]]);
    }
    let faceback = n(d, format!("STFB{}", d.g_game.consoleplayer));
    let sbar = n(d, "STBAR".into());
    let mut faces = Vec::with_capacity(ST_NUMFACES);
    for i in 0..ST_NUMPAINFACES {
        for j in 0..ST_NUMSTRAIGHTFACES {
            faces.push(n(d, format!("STFST{i}{j}")));
        }
        faces.push(n(d, format!("STFTR{i}0")));
        faces.push(n(d, format!("STFTL{i}0")));
        faces.push(n(d, format!("STFOUCH{i}")));
        faces.push(n(d, format!("STFEVL{i}")));
        faces.push(n(d, format!("STFKILL{i}")));
    }
    faces.push(n(d, "STFGOD0".into()));
    faces.push(n(d, "STFDEAD0".into()));
    let st = &mut d.st_stuff;
    st.tallnum = tallnum;
    st.shortnum = shortnum;
    st.tallpercent = tallpercent;
    st.keys = keys;
    st.armsbg = armsbg;
    st.arms = arms;
    st.faceback = faceback;
    st.sbar = sbar;
    st.faces = faces;
}

pub fn init(d: &mut Doom) {
    d.st_stuff.lu_palette = crate::w_wad::get_num_for_name(d, "PLAYPAL");
    load_graphics(d);
}

fn init_data(d: &mut Doom) {
    let st = &mut d.st_stuff;
    st.st_firsttime = true;
    st.plyr = d.g_game.consoleplayer;
    st.st_clock = 0;
    st.st_statusbaron = true;
    st.st_oldchat = false;
    st.st_chat = false;
    st.st_faceindex = 0;
    st.st_palette = -1;
    st.st_oldhealth = -1;
    st.oldweaponsowned = d.g_game.players[st.plyr].weaponowned;
    st.keyboxes = [-1; 3];
    st_lib::init(d);
}

fn create_widgets(d: &mut Doom) {
    let p = d.st_stuff.plyr;
    let ready = d.g_game.players[p].readyweapon;
    let st = &mut d.st_stuff;
    st.w_ready = st_lib::init_num(44, 171, &st.tallnum, 3);
    st.w_ready_src = ready_src(WEAPONINFO[ready as usize].ammo);
    st.w_health = st_lib::init_percent(90, 171, &st.tallnum, st.tallpercent);
    st.w_armsbg = st_lib::init_bin_icon(104, 168, st.armsbg);
    st.w_arms = (0..6)
        .map(|i| {
            st_lib::init_mult_icon(111 + (i % 3) * 12, 172 + (i / 3) * 10, &st.arms[i as usize])
        })
        .collect();
    st.w_frags = st_lib::init_num(138, 171, &st.tallnum, 2);
    st.w_faces = st_lib::init_mult_icon(143, 168, &st.faces);
    st.w_armor = st_lib::init_percent(221, 171, &st.tallnum, st.tallpercent);
    st.w_keyboxes = [171, 181, 191]
        .iter()
        .map(|&y| st_lib::init_mult_icon(239, y, &st.keys))
        .collect();
    st.w_ammo = [173, 179, 191, 185]
        .iter()
        .map(|&y| st_lib::init_num(288, y, &st.shortnum, 3))
        .collect();
    st.w_maxammo = [173, 179, 191, 185]
        .iter()
        .map(|&y| st_lib::init_num(314, y, &st.shortnum, 3))
        .collect();
}

fn stop(d: &mut Doom) {
    if d.st_stuff.st_stopped {
        return;
    }
    set_palette_num(d, 0);
    d.st_stuff.st_stopped = true;
}

pub fn start(d: &mut Doom) {
    if !d.st_stuff.st_stopped {
        stop(d);
    }
    init_data(d);
    create_widgets(d);
    d.st_stuff.st_stopped = false;
}

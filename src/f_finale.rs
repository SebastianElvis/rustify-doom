//! The finale: episode end text, art screens, the bunny scroll, and the Doom II cast.

use crate::d_main::{
    Event, COMMERCIAL, EV_KEYDOWN, EXE_CHEX, MISSION_DOOM, MISSION_DOOM2, MISSION_PACK_PLUT,
    MISSION_PACK_TNT, RETAIL,
};
use crate::g_game::{GA_NOTHING, GA_WORLDDONE, GS_FINALE, MAXPLAYERS};
use crate::hu_lib::patch_header;
use crate::hu_stuff::{logical_gamemission, HU_FONTSIZE, HU_FONTSTART};
use crate::i_video::{SCREENHEIGHT, SCREENWIDTH};
use crate::info::{
    MOBJINFO, MT_BABY, MT_BRUISER, MT_CHAINGUY, MT_CYBORG, MT_FATSO, MT_HEAD, MT_KNIGHT, MT_PAIN,
    MT_PLAYER, MT_POSSESSED, MT_SERGEANT, MT_SHOTGUY, MT_SKULL, MT_SPIDER, MT_TROOP, MT_UNDEAD,
    MT_VILE, STATES, S_NULL, S_PLAY_ATK1,
};
use crate::s_sound::{self, *};
use crate::v_video::{draw_patch, draw_patch_flipped};
use crate::Doom;

const MUS_BUNNY: i32 = 30;
const MUS_VICTOR: i32 = 31;
const MUS_EVIL: i32 = 63;
const MUS_READ_M: i32 = 65;

const TEXTSPEED: i32 = 3;
const TEXTWAIT: i32 = 250;

const F_STAGE_TEXT: i32 = 0;
const F_STAGE_ARTSCREEN: i32 = 1;
const F_STAGE_CAST: i32 = 2;

/// textscreen_t: mission, episode, level, background flat, text.
const TEXTSCREENS: [(i32, i32, i32, &str, &str); 22] = [
    (MISSION_DOOM, 1, 8, "FLOOR4_8", E1TEXT),
    (MISSION_DOOM, 2, 8, "SFLR6_1", E2TEXT),
    (MISSION_DOOM, 3, 8, "MFLR8_4", E3TEXT),
    (MISSION_DOOM, 4, 8, "MFLR8_3", E4TEXT),
    (MISSION_DOOM2, 1, 6, "SLIME16", C1TEXT),
    (MISSION_DOOM2, 1, 11, "RROCK14", C2TEXT),
    (MISSION_DOOM2, 1, 20, "RROCK07", C3TEXT),
    (MISSION_DOOM2, 1, 30, "RROCK17", C4TEXT),
    (MISSION_DOOM2, 1, 15, "RROCK13", C5TEXT),
    (MISSION_DOOM2, 1, 31, "RROCK19", C6TEXT),
    (MISSION_PACK_TNT, 1, 6, "SLIME16", T1TEXT),
    (MISSION_PACK_TNT, 1, 11, "RROCK14", T2TEXT),
    (MISSION_PACK_TNT, 1, 20, "RROCK07", T3TEXT),
    (MISSION_PACK_TNT, 1, 30, "RROCK17", T4TEXT),
    (MISSION_PACK_TNT, 1, 15, "RROCK13", T5TEXT),
    (MISSION_PACK_TNT, 1, 31, "RROCK19", T6TEXT),
    (MISSION_PACK_PLUT, 1, 6, "SLIME16", P1TEXT),
    (MISSION_PACK_PLUT, 1, 11, "RROCK14", P2TEXT),
    (MISSION_PACK_PLUT, 1, 20, "RROCK07", P3TEXT),
    (MISSION_PACK_PLUT, 1, 30, "RROCK17", P4TEXT),
    (MISSION_PACK_PLUT, 1, 15, "RROCK13", P5TEXT),
    (MISSION_PACK_PLUT, 1, 31, "RROCK19", P6TEXT),
];

/// castinfo_t.
const CASTORDER: [(&str, usize); 17] = [
    ("ZOMBIEMAN", MT_POSSESSED),
    ("SHOTGUN GUY", MT_SHOTGUY),
    ("HEAVY WEAPON DUDE", MT_CHAINGUY),
    ("IMP", MT_TROOP),
    ("DEMON", MT_SERGEANT),
    ("LOST SOUL", MT_SKULL),
    ("CACODEMON", MT_HEAD),
    ("HELL KNIGHT", MT_KNIGHT),
    ("BARON OF HELL", MT_BRUISER),
    ("ARACHNOTRON", MT_BABY),
    ("PAIN ELEMENTAL", MT_PAIN),
    ("REVENANT", MT_UNDEAD),
    ("MANCUBUS", MT_FATSO),
    ("ARCH-VILE", MT_VILE),
    ("THE SPIDER MASTERMIND", MT_SPIDER),
    ("THE CYBERDEMON", MT_CYBORG),
    ("OUR HERO", MT_PLAYER),
];

/// The finale globals.
#[derive(Default)]
pub struct State {
    finalestage: i32,
    finalecount: u32,
    finaletext: &'static str,
    finaleflat: &'static str,
    castnum: usize,
    casttics: i32,
    caststate: usize,
    castdeath: bool,
    castframes: i32,
    castonmelee: i32,
    castattacking: bool,
    laststage: i32,
}

/// F_StartFinale.
pub fn start_finale(d: &mut Doom) {
    d.g_game.gameaction = GA_NOTHING;
    d.g_game.gamestate = GS_FINALE;
    d.g_game.viewactive = false;
    d.am_map.automapactive = false;
    let mission = logical_gamemission(d);
    let mus = if mission == MISSION_DOOM {
        MUS_VICTOR
    } else {
        MUS_READ_M
    };
    s_sound::change_music(d, mus, true);
    for &(m, episode, level, flat, text) in TEXTSCREENS.iter() {
        let level = if d.d_main.gameversion == EXE_CHEX && m == MISSION_DOOM {
            5
        } else {
            level
        };
        if mission == m
            && (mission != MISSION_DOOM || d.g_game.gameepisode == episode)
            && d.g_game.gamemap == level
        {
            d.f_finale.finaletext = text;
            d.f_finale.finaleflat = flat;
        }
    }
    d.f_finale.finalestage = F_STAGE_TEXT;
    d.f_finale.finalecount = 0;
}

/// F_Responder.
pub fn responder(d: &mut Doom, ev: &Event) -> bool {
    if d.f_finale.finalestage == F_STAGE_CAST {
        return cast_responder(d, ev);
    }
    false
}

/// F_Ticker.
pub fn ticker(d: &mut Doom) {
    if d.d_main.gamemode == COMMERCIAL && d.f_finale.finalecount > 50 {
        let pressed = (0..MAXPLAYERS).any(|i| d.g_game.players[i].cmd.buttons != 0);
        if pressed {
            if d.g_game.gamemap == 30 {
                start_cast(d);
            } else {
                d.g_game.gameaction = GA_WORLDDONE;
            }
        }
    }
    d.f_finale.finalecount = d.f_finale.finalecount.wrapping_add(1);
    if d.f_finale.finalestage == F_STAGE_CAST {
        cast_ticker(d);
        return;
    }
    if d.d_main.gamemode == COMMERCIAL {
        return;
    }
    let f = &mut d.f_finale;
    if f.finalestage == F_STAGE_TEXT
        && f.finalecount as usize > f.finaletext.len() * TEXTSPEED as usize + TEXTWAIT as usize
    {
        f.finalecount = 0;
        f.finalestage = F_STAGE_ARTSCREEN;
        d.d_main.wipegamestate = -1;
        if d.g_game.gameepisode == 3 {
            s_sound::start_music(d, MUS_BUNNY);
        }
    }
}

/// Returns the heads-up font index of a character, or None for a character without a glyph.
fn font_index(c: u8) -> Option<usize> {
    let c = c.to_ascii_uppercase() as i32 - HU_FONTSTART;
    // The original also accepts HU_FONTSIZE, one past the font; that glyph does not exist.
    if c < 0 || c >= HU_FONTSIZE as i32 {
        None
    } else {
        Some(c as usize)
    }
}

/// F_TextWrite: tiles the flat and types the text one character every 3 tics.
fn text_write(d: &mut Doom) {
    let flat = crate::w_wad::get_num_for_name(d, d.f_finale.finaleflat);
    {
        let src = crate::w_wad::lump(&d.w_wad, flat);
        let dest = &mut d.i_video.video_buffer;
        for y in 0..SCREENHEIGHT {
            for x in 0..SCREENWIDTH {
                dest[y * SCREENWIDTH + x] = src[((y & 63) << 6) + (x & 63)];
            }
        }
    }
    crate::v_video::mark_rect(d, 0, 0, SCREENWIDTH as i32, SCREENHEIGHT as i32);
    let mut cx = 10;
    let mut cy = 10;
    let text = d.f_finale.finaletext.as_bytes();
    let mut count = (d.f_finale.finalecount as i32 - 10) / TEXTSPEED;
    if count < 0 {
        count = 0;
    }
    let mut i = 0;
    while count > 0 {
        count -= 1;
        let Some(&c) = text.get(i) else { break };
        i += 1;
        if c == b'\n' {
            cx = 10;
            cy += 11;
            continue;
        }
        let Some(fi) = font_index(c) else {
            cx += 4;
            continue;
        };
        let p = d.hu_stuff.hu_font[fi];
        let w = patch_header(d, p).0;
        if cx + w > SCREENWIDTH as i32 {
            break;
        }
        draw_patch(d, cx, cy, p);
        cx += w;
    }
}

fn mobjinfo(d: &Doom) -> &'static crate::info::MobjInfo {
    &MOBJINFO[CASTORDER[d.f_finale.castnum].1]
}

/// F_StartCast.
fn start_cast(d: &mut Doom) {
    d.d_main.wipegamestate = -1;
    let f = &mut d.f_finale;
    f.castnum = 0;
    f.caststate = MOBJINFO[CASTORDER[0].1].seestate as usize;
    f.casttics = STATES[f.caststate].tics;
    f.castdeath = false;
    f.finalestage = F_STAGE_CAST;
    f.castframes = 0;
    f.castonmelee = 0;
    f.castattacking = false;
    s_sound::change_music(d, MUS_EVIL, true);
}

/// The attack sound of a cast state ("sound hacks").
fn cast_sound(st: usize) -> i32 {
    match st {
        S_PLAY_ATK1 => SFX_DSHTGN,
        185 => SFX_PISTOL,             // S_POSS_ATK2
        218 => SFX_SHOTGN,             // S_SPOS_ATK2
        256 => SFX_VILATK,             // S_VILE_ATK2
        336 => SFX_SKESWG,             // S_SKEL_FIST2
        338 => SFX_SKEPCH,             // S_SKEL_FIST4
        340 => SFX_SKEATK,             // S_SKEL_MISS2
        377 | 380 | 383 => SFX_FIRSHT, // S_FATT_ATK2, 5, 8
        417..=419 => SFX_SHOTGN,       // S_CPOS_ATK2 to 4
        454 => SFX_CLAW,               // S_TROO_ATK3
        486 => SFX_SGTATK,             // S_SARG_ATK2
        538 | 567 | 505 => SFX_FIRSHT, // S_BOSS_ATK2, S_BOS2_ATK2, S_HEAD_ATK2
        590 => SFX_SKLATK,             // S_SKULL_ATK2
        616 | 617 => SFX_SHOTGN,       // S_SPID_ATK2, 3
        648 => SFX_PLASMA,             // S_BSPI_ATK2
        685 | 687 | 689 => SFX_RLAUNC, // S_CYBER_ATK2, 4, 6
        710 => SFX_SKLATK,             // S_PAIN_ATK3
        _ => 0,
    }
}

/// F_CastTicker.
fn cast_ticker(d: &mut Doom) {
    d.f_finale.casttics -= 1;
    if d.f_finale.casttics > 0 {
        return;
    }
    let cur = STATES[d.f_finale.caststate];
    let mut stopattack = false;
    if cur.tics == -1 || cur.nextstate == S_NULL {
        let f = &mut d.f_finale;
        f.castnum += 1;
        f.castdeath = false;
        if f.castnum == CASTORDER.len() {
            f.castnum = 0;
        }
        let info = mobjinfo(d);
        if info.seesound != 0 {
            s_sound::start_sound(d, None, info.seesound);
        }
        d.f_finale.caststate = info.seestate as usize;
        d.f_finale.castframes = 0;
    } else if d.f_finale.caststate == S_PLAY_ATK1 {
        stopattack = true;
    } else {
        let st = cur.nextstate;
        d.f_finale.caststate = st;
        d.f_finale.castframes += 1;
        let sfx = cast_sound(st);
        if sfx != 0 {
            s_sound::start_sound(d, None, sfx);
        }
    }
    let info = mobjinfo(d);
    let f = &mut d.f_finale;
    if !stopattack {
        if f.castframes == 12 {
            f.castattacking = true;
            f.caststate = if f.castonmelee != 0 {
                info.meleestate
            } else {
                info.missilestate
            } as usize;
            f.castonmelee ^= 1;
            if f.caststate == S_NULL {
                f.caststate = if f.castonmelee != 0 {
                    info.meleestate
                } else {
                    info.missilestate
                } as usize;
            }
        }
        if f.castattacking && (f.castframes == 24 || f.caststate == info.seestate as usize) {
            stopattack = true;
        }
    }
    if stopattack {
        f.castattacking = false;
        f.castframes = 0;
        f.caststate = info.seestate as usize;
    }
    f.casttics = STATES[f.caststate].tics;
    if f.casttics == -1 {
        f.casttics = 15;
    }
}

/// F_CastResponder: a key press kills the current cast monster.
fn cast_responder(d: &mut Doom, ev: &Event) -> bool {
    if ev.type_ != EV_KEYDOWN {
        return false;
    }
    if d.f_finale.castdeath {
        return true;
    }
    let info = mobjinfo(d);
    let f = &mut d.f_finale;
    f.castdeath = true;
    f.caststate = info.deathstate as usize;
    f.casttics = STATES[f.caststate].tics;
    f.castframes = 0;
    f.castattacking = false;
    if info.deathsound != 0 {
        s_sound::start_sound(d, None, info.deathsound);
    }
    true
}

/// F_CastPrint: draws a centered name at y 180.
fn cast_print(d: &mut Doom, text: &str) {
    let mut width = 0;
    for &c in text.as_bytes() {
        width += match font_index(c) {
            Some(fi) => patch_header(d, d.hu_stuff.hu_font[fi]).0,
            None => 4,
        };
    }
    let mut cx = 160 - width / 2;
    for &c in text.as_bytes() {
        let Some(fi) = font_index(c) else {
            cx += 4;
            continue;
        };
        let p = d.hu_stuff.hu_font[fi];
        let w = patch_header(d, p).0;
        draw_patch(d, cx, 180, p);
        cx += w;
    }
}

/// F_CastDrawer.
fn cast_drawer(d: &mut Doom) {
    let bg = crate::w_wad::get_num_for_name(d, "BOSSBACK");
    draw_patch(d, 0, 0, bg);
    cast_print(d, CASTORDER[d.f_finale.castnum].0);
    let st = STATES[d.f_finale.caststate];
    let frame = d.r_things.sprites[st.sprite].spriteframes
        [(st.frame & crate::r_things::FF_FRAMEMASK) as usize];
    let patch = (frame.lump[0] as i32 + d.r_data.firstspritelump) as usize;
    if frame.flip[0] != 0 {
        draw_patch_flipped(d, 160, 170, patch);
    } else {
        draw_patch(d, 160, 170, patch);
    }
}

/// F_DrawPatchCol: copies one patch column to screen column x.
fn draw_patch_col(d: &mut Doom, x: usize, patch: usize, col: usize) {
    let p = crate::w_wad::lump(&d.w_wad, patch);
    let at = 8 + 4 * col;
    let mut c = u32::from_le_bytes([p[at], p[at + 1], p[at + 2], p[at + 3]]) as usize;
    let dest = &mut d.i_video.video_buffer;
    while p[c] != 0xff {
        let top = p[c] as usize;
        let len = p[c + 1] as usize;
        for j in 0..len {
            dest[(top + j) * SCREENWIDTH + x] = p[c + 3 + j];
        }
        c += len + 4;
    }
}

/// F_BunnyScroll: scrolls from PFUB2 to PFUB1 and then types "THE END".
fn bunny_scroll(d: &mut Doom) {
    let p1 = crate::w_wad::get_num_for_name(d, "PFUB2");
    let p2 = crate::w_wad::get_num_for_name(d, "PFUB1");
    crate::v_video::mark_rect(d, 0, 0, SCREENWIDTH as i32, SCREENHEIGHT as i32);
    let finalecount = d.f_finale.finalecount;
    let scrolled = (320 - (finalecount as i32 - 230) / 2).clamp(0, 320) as usize;
    for x in 0..SCREENWIDTH {
        if x + scrolled < 320 {
            draw_patch_col(d, x, p1, x + scrolled);
        } else {
            draw_patch_col(d, x, p2, x + scrolled - 320);
        }
    }
    if finalecount < 1130 {
        return;
    }
    let (x, y) = (
        (SCREENWIDTH as i32 - 13 * 8) / 2,
        (SCREENHEIGHT as i32 - 8 * 8) / 2,
    );
    if finalecount < 1180 {
        let e = crate::w_wad::get_num_for_name(d, "END0");
        draw_patch(d, x, y, e);
        d.f_finale.laststage = 0;
        return;
    }
    let stage = (((finalecount - 1180) / 5) as i32).min(6);
    if stage > d.f_finale.laststage {
        s_sound::start_sound(d, None, SFX_PISTOL);
        d.f_finale.laststage = stage;
    }
    let e = crate::w_wad::get_num_for_name(d, &format!("END{}", stage));
    draw_patch(d, x, y, e);
}

/// F_ArtScreenDrawer.
fn art_screen_drawer(d: &mut Doom) {
    if d.g_game.gameepisode == 3 {
        bunny_scroll(d);
        return;
    }
    let name = match d.g_game.gameepisode {
        1 => {
            if d.d_main.gamemode == RETAIL {
                "CREDIT"
            } else {
                "HELP2"
            }
        }
        2 => "VICTORY2",
        4 => "ENDPIC",
        _ => return,
    };
    let l = crate::w_wad::get_num_for_name(d, name);
    draw_patch(d, 0, 0, l);
}

/// F_Drawer.
pub fn drawer(d: &mut Doom) {
    match d.f_finale.finalestage {
        F_STAGE_CAST => cast_drawer(d),
        F_STAGE_TEXT => text_write(d),
        _ => art_screen_drawer(d),
    }
}

const E1TEXT: &str = "Once you beat the big badasses and\n\
clean out the moon base you're supposed\n\
to win, aren't you? Aren't you? Where's\n\
your fat reward and ticket home? What\n\
the hell is this? It's not supposed to\n\
end this way!\n\
\n\
It stinks like rotten meat, but looks\n\
like the lost Deimos base.  Looks like\n\
you're stuck on The Shores of Hell.\n\
The only way out is through.\n\
\n\
To continue the DOOM experience, play\n\
The Shores of Hell and its amazing\n\
sequel, Inferno!\n\
";
const E2TEXT: &str = "You've done it! The hideous cyber-\n\
demon lord that ruled the lost Deimos\n\
moon base has been slain and you\n\
are triumphant! But ... where are\n\
you? You clamber to the edge of the\n\
moon and look down to see the awful\n\
truth.\n\
\n\
Deimos floats above Hell itself!\n\
You've never heard of anyone escaping\n\
from Hell, but you'll make the bastards\n\
sorry they ever heard of you! Quickly,\n\
you rappel down to  the surface of\n\
Hell.\n\
\n\
Now, it's on to the final chapter of\n\
DOOM! -- Inferno.";
const E3TEXT: &str = "The loathsome spiderdemon that\n\
masterminded the invasion of the moon\n\
bases and caused so much death has had\n\
its ass kicked for all time.\n\
\n\
A hidden doorway opens and you enter.\n\
You've proven too tough for Hell to\n\
contain, and now Hell at last plays\n\
fair -- for you emerge from the door\n\
to see the green fields of Earth!\n\
Home at last.\n\
\n\
You wonder what's been happening on\n\
Earth while you were battling evil\n\
unleashed. It's good that no Hell-\n\
spawn could have come through that\n\
door with you ...";
const E4TEXT: &str = "the spider mastermind must have sent forth\n\
its legions of hellspawn before your\n\
final confrontation with that terrible\n\
beast from hell.  but you stepped forward\n\
and brought forth eternal damnation and\n\
suffering upon the horde as a true hero\n\
would in the face of something so evil.\n\
\n\
besides, someone was gonna pay for what\n\
happened to daisy, your pet rabbit.\n\
\n\
but now, you see spread before you more\n\
potential pain and gibbitude as a nation\n\
of demons run amok among our cities.\n\
\n\
next stop, hell on earth!";
const C1TEXT: &str = "YOU HAVE ENTERED DEEPLY INTO THE INFESTED\n\
STARPORT. BUT SOMETHING IS WRONG. THE\n\
MONSTERS HAVE BROUGHT THEIR OWN REALITY\n\
WITH THEM, AND THE STARPORT'S TECHNOLOGY\n\
IS BEING SUBVERTED BY THEIR PRESENCE.\n\
\n\
AHEAD, YOU SEE AN OUTPOST OF HELL, A\n\
FORTIFIED ZONE. IF YOU CAN GET PAST IT,\n\
YOU CAN PENETRATE INTO THE HAUNTED HEART\n\
OF THE STARBASE AND FIND THE CONTROLLING\n\
SWITCH WHICH HOLDS EARTH'S POPULATION\n\
HOSTAGE.";
const C2TEXT: &str = "YOU HAVE WON! YOUR VICTORY HAS ENABLED\n\
HUMANKIND TO EVACUATE EARTH AND ESCAPE\n\
THE NIGHTMARE.  NOW YOU ARE THE ONLY\n\
HUMAN LEFT ON THE FACE OF THE PLANET.\n\
CANNIBAL MUTATIONS, CARNIVOROUS ALIENS,\n\
AND EVIL SPIRITS ARE YOUR ONLY NEIGHBORS.\n\
YOU SIT BACK AND WAIT FOR DEATH, CONTENT\n\
THAT YOU HAVE SAVED YOUR SPECIES.\n\
\n\
BUT THEN, EARTH CONTROL BEAMS DOWN A\n\
MESSAGE FROM SPACE: \"SENSORS HAVE LOCATED\n\
THE SOURCE OF THE ALIEN INVASION. IF YOU\n\
GO THERE, YOU MAY BE ABLE TO BLOCK THEIR\n\
ENTRY.  THE ALIEN BASE IS IN THE HEART OF\n\
YOUR OWN HOME CITY, NOT FAR FROM THE\n\
STARPORT.\" SLOWLY AND PAINFULLY YOU GET\n\
UP AND RETURN TO THE FRAY.";
const C3TEXT: &str = "YOU ARE AT THE CORRUPT HEART OF THE CITY,\n\
SURROUNDED BY THE CORPSES OF YOUR ENEMIES.\n\
YOU SEE NO WAY TO DESTROY THE CREATURES'\n\
ENTRYWAY ON THIS SIDE, SO YOU CLENCH YOUR\n\
TEETH AND PLUNGE THROUGH IT.\n\
\n\
THERE MUST BE A WAY TO CLOSE IT ON THE\n\
OTHER SIDE. WHAT DO YOU CARE IF YOU'VE\n\
GOT TO GO THROUGH HELL TO GET TO IT?";
const C4TEXT: &str = "THE HORRENDOUS VISAGE OF THE BIGGEST\n\
DEMON YOU'VE EVER SEEN CRUMBLES BEFORE\n\
YOU, AFTER YOU PUMP YOUR ROCKETS INTO\n\
HIS EXPOSED BRAIN. THE MONSTER SHRIVELS\n\
UP AND DIES, ITS THRASHING LIMBS\n\
DEVASTATING UNTOLD MILES OF HELL'S\n\
SURFACE.\n\
\n\
YOU'VE DONE IT. THE INVASION IS OVER.\n\
EARTH IS SAVED. HELL IS A WRECK. YOU\n\
WONDER WHERE BAD FOLKS WILL GO WHEN THEY\n\
DIE, NOW. WIPING THE SWEAT FROM YOUR\n\
FOREHEAD YOU BEGIN THE LONG TREK BACK\n\
HOME. REBUILDING EARTH OUGHT TO BE A\n\
LOT MORE FUN THAN RUINING IT WAS.\n\
";
const C5TEXT: &str = "CONGRATULATIONS, YOU'VE FOUND THE SECRET\n\
LEVEL! LOOKS LIKE IT'S BEEN BUILT BY\n\
HUMANS, RATHER THAN DEMONS. YOU WONDER\n\
WHO THE INMATES OF THIS CORNER OF HELL\n\
WILL BE.";
const C6TEXT: &str = "CONGRATULATIONS, YOU'VE FOUND THE\n\
SUPER SECRET LEVEL!  YOU'D BETTER\n\
BLAZE THROUGH THIS ONE!\n\
";
const T1TEXT: &str = "You've fought your way out of the infested\n\
experimental labs.   It seems that UAC has\n\
once again gulped it down.  With their\n\
high turnover, it must be hard for poor\n\
old UAC to buy corporate health insurance\n\
nowadays..\n\
\n\
Ahead lies the military complex, now\n\
swarming with diseased horrors hot to get\n\
their teeth into you. With luck, the\n\
complex still has some warlike ordnance\n\
laying around.";
const T2TEXT: &str = "You hear the grinding of heavy machinery\n\
ahead.  You sure hope they're not stamping\n\
out new hellspawn, but you're ready to\n\
ream out a whole herd if you have to.\n\
They might be planning a blood feast, but\n\
you feel about as mean as two thousand\n\
maniacs packed into one mad killer.\n\
\n\
You don't plan to go down easy.";
const T3TEXT: &str = "The vista opening ahead looks real damn\n\
familiar. Smells familiar, too -- like\n\
fried excrement. You didn't like this\n\
place before, and you sure as hell ain't\n\
planning to like it now. The more you\n\
brood on it, the madder you get.\n\
Hefting your gun, an evil grin trickles\n\
onto your face. Time to take some names.";
const T4TEXT: &str = "Suddenly, all is silent, from one horizon\n\
to the other. The agonizing echo of Hell\n\
fades away, the nightmare sky turns to\n\
blue, the heaps of monster corpses start \n\
to evaporate along with the evil stench \n\
that filled the air. Jeeze, maybe you've\n\
done it. Have you really won?\n\
\n\
Something rumbles in the distance.\n\
A blue light begins to glow inside the\n\
ruined skull of the demon-spitter.";
const T5TEXT: &str = "What now? Looks totally different. Kind\n\
of like King Tut's condo. Well,\n\
whatever's here can't be any worse\n\
than usual. Can it?  Or maybe it's best\n\
to let sleeping gods lie..";
const T6TEXT: &str = "Time for a vacation. You've burst the\n\
bowels of hell and by golly you're ready\n\
for a break. You mutter to yourself,\n\
Maybe someone else can kick Hell's ass\n\
next time around. Ahead lies a quiet town,\n\
with peaceful flowing water, quaint\n\
buildings, and presumably no Hellspawn.\n\
\n\
As you step off the transport, you hear\n\
the stomp of a cyberdemon's iron shoe.";
const P1TEXT: &str = "You gloat over the steaming carcass of the\n\
Guardian.  With its death, you've wrested\n\
the Accelerator from the stinking claws\n\
of Hell.  You relax and glance around the\n\
room.  Damn!  There was supposed to be at\n\
least one working prototype, but you can't\n\
see it. The demons must have taken it.\n\
\n\
You must find the prototype, or all your\n\
struggles will have been wasted. Keep\n\
moving, keep fighting, keep killing.\n\
Oh yes, keep living, too.";
const P2TEXT: &str = "Even the deadly Arch-Vile labyrinth could\n\
not stop you, and you've gotten to the\n\
prototype Accelerator which is soon\n\
efficiently and permanently deactivated.\n\
\n\
You're good at that kind of thing.";
const P3TEXT: &str = "You've bashed and battered your way into\n\
the heart of the devil-hive.  Time for a\n\
Search-and-Destroy mission, aimed at the\n\
Gatekeeper, whose foul offspring is\n\
cascading to Earth.  Yeah, he's bad. But\n\
you know who's worse!\n\
\n\
Grinning evilly, you check your gear, and\n\
get ready to give the bastard a little Hell\n\
of your own making!";
const P4TEXT: &str = "The Gatekeeper's evil face is splattered\n\
all over the place.  As its tattered corpse\n\
collapses, an inverted Gate forms and\n\
sucks down the shards of the last\n\
prototype Accelerator, not to mention the\n\
few remaining demons.  You're done. Hell\n\
has gone back to pounding bad dead folks \n\
instead of good live ones.  Remember to\n\
tell your grandkids to put a rocket\n\
launcher in your coffin. If you go to Hell\n\
when you die, you'll need it for some\n\
final cleaning-up ...";
const P5TEXT: &str = "You've found the second-hardest level we\n\
got. Hope you have a saved game a level or\n\
two previous.  If not, be prepared to die\n\
aplenty. For master marines only.";
const P6TEXT: &str = "Betcha wondered just what WAS the hardest\n\
level we had ready for ya?  Now you know.\n\
No one gets out alive.";

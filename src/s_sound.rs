//! Sound requests. The original links no audio output API (EV-002).
//! The module keeps the sound effect numbers and the request entry points.

#[allow(unused_imports)]
use crate::p_mobj::MobjId;
#[allow(unused_imports)]
use crate::Doom;
pub const SFX_NONE: i32 = 0;
pub const SFX_PISTOL: i32 = 1;
pub const SFX_SHOTGN: i32 = 2;
pub const SFX_SGCOCK: i32 = 3;
pub const SFX_DSHTGN: i32 = 4;
pub const SFX_DBOPN: i32 = 5;
pub const SFX_DBCLS: i32 = 6;
pub const SFX_DBLOAD: i32 = 7;
pub const SFX_PLASMA: i32 = 8;
pub const SFX_BFG: i32 = 9;
pub const SFX_SAWUP: i32 = 10;
pub const SFX_SAWIDL: i32 = 11;
pub const SFX_SAWFUL: i32 = 12;
pub const SFX_SAWHIT: i32 = 13;
pub const SFX_RLAUNC: i32 = 14;
pub const SFX_RXPLOD: i32 = 15;
pub const SFX_FIRSHT: i32 = 16;
pub const SFX_FIRXPL: i32 = 17;
pub const SFX_PSTART: i32 = 18;
pub const SFX_PSTOP: i32 = 19;
pub const SFX_DOROPN: i32 = 20;
pub const SFX_DORCLS: i32 = 21;
pub const SFX_STNMOV: i32 = 22;
pub const SFX_SWTCHN: i32 = 23;
pub const SFX_SWTCHX: i32 = 24;
pub const SFX_PLPAIN: i32 = 25;
pub const SFX_DMPAIN: i32 = 26;
pub const SFX_POPAIN: i32 = 27;
pub const SFX_VIPAIN: i32 = 28;
pub const SFX_MNPAIN: i32 = 29;
pub const SFX_PEPAIN: i32 = 30;
pub const SFX_SLOP: i32 = 31;
pub const SFX_ITEMUP: i32 = 32;
pub const SFX_WPNUP: i32 = 33;
pub const SFX_OOF: i32 = 34;
pub const SFX_TELEPT: i32 = 35;
pub const SFX_POSIT1: i32 = 36;
pub const SFX_POSIT2: i32 = 37;
pub const SFX_POSIT3: i32 = 38;
pub const SFX_BGSIT1: i32 = 39;
pub const SFX_BGSIT2: i32 = 40;
pub const SFX_SGTSIT: i32 = 41;
pub const SFX_CACSIT: i32 = 42;
pub const SFX_BRSSIT: i32 = 43;
pub const SFX_CYBSIT: i32 = 44;
pub const SFX_SPISIT: i32 = 45;
pub const SFX_BSPSIT: i32 = 46;
pub const SFX_KNTSIT: i32 = 47;
pub const SFX_VILSIT: i32 = 48;
pub const SFX_MANSIT: i32 = 49;
pub const SFX_PESIT: i32 = 50;
pub const SFX_SKLATK: i32 = 51;
pub const SFX_SGTATK: i32 = 52;
pub const SFX_SKEPCH: i32 = 53;
pub const SFX_VILATK: i32 = 54;
pub const SFX_CLAW: i32 = 55;
pub const SFX_SKESWG: i32 = 56;
pub const SFX_PLDETH: i32 = 57;
pub const SFX_PDIEHI: i32 = 58;
pub const SFX_PODTH1: i32 = 59;
pub const SFX_PODTH2: i32 = 60;
pub const SFX_PODTH3: i32 = 61;
pub const SFX_BGDTH1: i32 = 62;
pub const SFX_BGDTH2: i32 = 63;
pub const SFX_SGTDTH: i32 = 64;
pub const SFX_CACDTH: i32 = 65;
pub const SFX_SKLDTH: i32 = 66;
pub const SFX_BRSDTH: i32 = 67;
pub const SFX_CYBDTH: i32 = 68;
pub const SFX_SPIDTH: i32 = 69;
pub const SFX_BSPDTH: i32 = 70;
pub const SFX_VILDTH: i32 = 71;
pub const SFX_KNTDTH: i32 = 72;
pub const SFX_PEDTH: i32 = 73;
pub const SFX_SKEDTH: i32 = 74;
pub const SFX_POSACT: i32 = 75;
pub const SFX_BGACT: i32 = 76;
pub const SFX_DMACT: i32 = 77;
pub const SFX_BSPACT: i32 = 78;
pub const SFX_BSPWLK: i32 = 79;
pub const SFX_VILACT: i32 = 80;
pub const SFX_NOWAY: i32 = 81;
pub const SFX_BAREXP: i32 = 82;
pub const SFX_PUNCH: i32 = 83;
pub const SFX_HOOF: i32 = 84;
pub const SFX_METAL: i32 = 85;
pub const SFX_CHGUN: i32 = 86;
pub const SFX_TINK: i32 = 87;
pub const SFX_BDOPN: i32 = 88;
pub const SFX_BDCLS: i32 = 89;
pub const SFX_ITMBK: i32 = 90;
pub const SFX_FLAME: i32 = 91;
pub const SFX_FLAMST: i32 = 92;
pub const SFX_GETPOW: i32 = 93;
pub const SFX_BOSPIT: i32 = 94;
pub const SFX_BOSCUB: i32 = 95;
pub const SFX_BOSSIT: i32 = 96;
pub const SFX_BOSPN: i32 = 97;
pub const SFX_BOSDTH: i32 = 98;
pub const SFX_MANATK: i32 = 99;
pub const SFX_MANDTH: i32 = 100;
pub const SFX_SSSIT: i32 = 101;
pub const SFX_SSDTH: i32 = 102;
pub const SFX_KEENPN: i32 = 103;
pub const SFX_KEENDT: i32 = 104;
pub const SFX_SKEACT: i32 = 105;
pub const SFX_SKESIT: i32 = 106;
pub const SFX_SKEATK: i32 = 107;
pub const SFX_RADIO: i32 = 108;
pub const NUMSFX: usize = 109;

pub const MUS_E1M1: i32 = 1;
pub const MUS_INTRO: i32 = 29;
pub const MUS_RUNNIN: i32 = 33;
pub const MUS_DM2TTL: i32 = 66;
pub const NUMMUSIC: i32 = 68;

/// The music of Episode 4 maps (spmus in the original).
const SPMUS: [i32; 9] = [22, 20, 21, 5, 16, 13, 15, 14, 9];

/// The sound globals. The output device is silent, so no channel state is observable.
#[derive(Default)]
pub struct State {
    pub snd_sfx_volume: i32,
    pub snd_music_volume: i32,
    pub mus_paused: bool,
    pub mus_playing: i32,
    /// The lump of the playing music.
    pub mus_lump: usize,
}

/// S_Init.
pub fn init(d: &mut Doom, sfx_volume: i32, music_volume: i32) {
    set_sfx_volume(d, sfx_volume);
    set_music_volume(d, music_volume);
    d.s_sound.mus_paused = false;
    crate::i_system::at_exit(d, crate::i_system::ExitFunc::ShutdownSound, true);
}

/// S_Start: starts the music of the level.
pub fn start(d: &mut Doom) {
    d.s_sound.mus_paused = false;
    let (e, m) = (d.g_game.gameepisode, d.g_game.gamemap);
    let mnum = if d.d_main.gamemode == crate::d_main::COMMERCIAL {
        MUS_RUNNIN + m - 1
    } else if e < 4 {
        MUS_E1M1 + (e - 1) * 9 + m - 1
    } else {
        SPMUS[(m - 1) as usize]
    };
    change_music(d, mnum, true);
}

/// S_StartSound. The original takes no random number here. Silent channels have no visible effect.
pub fn start_sound(d: &mut Doom, _origin: Option<MobjId>, sfx: i32) {
    if !(1..=NUMSFX as i32).contains(&sfx) {
        crate::i_system::error(d, &format!("Bad sfx #: {sfx}"));
    }
}

/// S_StopSound.
pub fn stop_sound(_d: &mut Doom, _origin: MobjId) {}

/// S_StartMusic.
pub fn start_music(d: &mut Doom, music: i32) {
    change_music(d, music, false);
}

/// S_ChangeMusic. The zone model replays the W_CacheLumpNum call of the lump.
pub fn change_music(d: &mut Doom, music: i32, _looping: bool) {
    use crate::d_main::{COMMERCIAL, RETAIL};
    let retail = matches!(d.d_main.gamemode, COMMERCIAL | RETAIL);
    let music = if music == MUS_INTRO && retail {
        32
    } else {
        music
    };
    if music <= 0 || music >= NUMMUSIC {
        crate::i_system::error(d, &format!("Bad music number {music}"));
    }
    if music != d.s_sound.mus_playing {
        let name = format!("d_{}", MUSIC_NAMES[music as usize]);
        d.s_sound.mus_lump = crate::w_wad::get_num_for_name(d, &name);
        d.s_sound.mus_playing = music;
    }
}

/// The lump name suffixes of S_music.
const MUSIC_NAMES: [&str; NUMMUSIC as usize] = [
    "", "e1m1", "e1m2", "e1m3", "e1m4", "e1m5", "e1m6", "e1m7", "e1m8", "e1m9", "e2m1", "e2m2",
    "e2m3", "e2m4", "e2m5", "e2m6", "e2m7", "e2m8", "e2m9", "e3m1", "e3m2", "e3m3", "e3m4", "e3m5",
    "e3m6", "e3m7", "e3m8", "e3m9", "inter", "intro", "bunny", "victor", "introa", "runnin",
    "stalks", "countd", "betwee", "doom", "the_da", "shawn", "ddtblu", "in_cit", "dead", "stlks2",
    "theda2", "doom2", "ddtbl2", "runni2", "dead2", "stlks3", "romero", "shawn2", "messag",
    "count2", "ddtbl3", "ampie", "theda3", "adrian", "messg2", "romer2", "tense", "shawn3",
    "openin", "evil", "ultima", "read_m", "dm2ttl", "dm2int",
];

/// S_PauseSound.
pub fn pause_sound(d: &mut Doom) {
    d.s_sound.mus_paused = true;
}

/// S_ResumeSound.
pub fn resume_sound(d: &mut Doom) {
    d.s_sound.mus_paused = false;
}

/// S_UpdateSounds. Silent channels end at once.
pub fn update_sounds(_d: &mut Doom, _listener: Option<MobjId>) {}

/// S_SetMusicVolume.
pub fn set_music_volume(d: &mut Doom, volume: i32) {
    if !(0..=127).contains(&volume) {
        crate::i_system::error(d, &format!("Attempt to set music volume at {volume}"));
    }
    d.s_sound.snd_music_volume = volume;
}

/// S_SetSfxVolume.
pub fn set_sfx_volume(d: &mut Doom, volume: i32) {
    if !(0..=127).contains(&volume) {
        crate::i_system::error(d, &format!("Attempt to set sfx volume at {volume}"));
    }
    d.s_sound.snd_sfx_volume = volume;
}

/// S_StartSound with the sound origin of a sector (sector_t.soundorg).
pub fn start_sector_sound(d: &mut Doom, _sector: usize, sfx: i32) {
    start_sound(d, None, sfx);
}

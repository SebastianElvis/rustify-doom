//! Savegame byte serialization of players, world, thinkers, and specials.
//!
//! The layout is the vanilla layout of the arm64 original. Pointer fields hold 32 bits.
//! The original stores process addresses there. They vary between runs (rule V-1), and the load path ignores
//! them except for the zero test of a stopped ceiling or platform. The Rust build stores
//! a nonzero value derived from the thinker index for each non-null pointer.

use crate::g_game::{Player, MAXPLAYERS};
use crate::m_fixed::FRACBITS;
use crate::p_ceilng::Ceiling;
use crate::p_doors::VlDoor;
use crate::p_floor::FloorMove;
use crate::p_lights::{Glow, LightFlash, Strobe};
use crate::p_mobj::Mobj;
use crate::p_plats::Plat;
use crate::p_setup::MapThing;
use crate::p_tick::{Think, ThinkerData, ThinkerId};
use crate::Doom;

pub const SAVESTRINGSIZE: usize = 24;
pub const VERSIONSIZE: usize = 16;
const SAVEGAME_EOF: u8 = 0x1d;

const TC_END: u8 = 0;
const TC_MOBJ: u8 = 1;

const TC_CEILING: u8 = 0;
const TC_DOOR: u8 = 1;
const TC_FLOOR: u8 = 2;
const TC_PLAT: u8 = 3;
const TC_FLASH: u8 = 4;
const TC_STROBE: u8 = 5;
const TC_GLOW: u8 = 6;
const TC_ENDSPECIALS: u8 = 7;

/// The save stream (save_stream). A read stream holds the file bytes. A write stream collects the bytes for the caller.
#[derive(Default)]
pub struct State {
    pub save_stream: Vec<u8>,
    /// The stream position (ftell).
    pub pos: usize,
    pub savegame_error: bool,
}

impl State {
    /// Opens a savegame file for reading (fopen "rb"). It returns false when the file cannot be read.
    pub fn open_read(&mut self, path: &str) -> bool {
        match std::fs::read(path) {
            Ok(b) => {
                self.save_stream = b;
                self.pos = 0;
                self.savegame_error = false;
                true
            }
            Err(_) => false,
        }
    }

    /// Starts an empty write stream. The caller writes `save_stream` to the file.
    pub fn open_write(&mut self) {
        self.save_stream.clear();
        self.pos = 0;
        self.savegame_error = false;
    }

    fn read8(&mut self) -> u8 {
        match self.save_stream.get(self.pos) {
            Some(&b) => {
                self.pos += 1;
                b
            }
            None => {
                if !self.savegame_error {
                    eprintln!("saveg_read8: Unexpected end of file while reading save game");
                    self.savegame_error = true;
                }
                0xff
            }
        }
    }

    fn write8(&mut self, v: u8) {
        self.save_stream.push(v);
        self.pos += 1;
    }

    fn read16(&mut self) -> i32 {
        let lo = self.read8() as u16;
        let hi = self.read8() as u16;
        (lo | hi << 8) as i16 as i32
    }

    fn write16(&mut self, v: i32) {
        self.write8(v as u8);
        self.write8((v >> 8) as u8);
    }

    fn read32(&mut self) -> i32 {
        let mut r = 0u32;
        for i in 0..4 {
            r |= (self.read8() as u32) << (8 * i);
        }
        r as i32
    }

    fn write32(&mut self, v: i32) {
        for i in 0..4 {
            self.write8((v >> (8 * i)) as u8);
        }
    }

    fn read_pad(&mut self) {
        for _ in 0..(4 - (self.pos & 3)) & 3 {
            self.read8();
        }
    }

    fn write_pad(&mut self) {
        for _ in 0..(4 - (self.pos & 3)) & 3 {
            self.write8(0);
        }
    }
}

/// The stored value of a non-null pointer to thinker or object `id`.
fn ptr(id: Option<usize>) -> i32 {
    id.map_or(0, |i| 0x0100_0000 + (i as i32) * 0x100)
}

/// Writes the thinker_t header: the list links and the function.
fn write_thinker(d: &mut Doom, id: ThinkerId) {
    let t = &d.p_tick.thinkers[id];
    let (prev, next) = (ptr(Some(t.prev)), ptr(Some(t.next)));
    let func = if t.func == Think::Null {
        0
    } else {
        0x0010_0000 + t.func as i32
    };
    let s = &mut d.p_saveg;
    s.write32(prev);
    s.write32(next);
    s.write32(func);
}

/// Reads the thinker_t header and returns true when the stored function is not null.
fn read_thinker(s: &mut State) -> bool {
    s.read32();
    s.read32();
    s.read32() != 0
}

fn write_mapthing(s: &mut State, m: &MapThing) {
    for v in [m.x, m.y, m.angle, m.type_, m.options] {
        s.write16(v as i32);
    }
}

fn read_mapthing(s: &mut State) -> MapThing {
    let mut v = [0i16; 5];
    for x in v.iter_mut() {
        *x = s.read16() as i16;
    }
    MapThing {
        x: v[0],
        y: v[1],
        angle: v[2],
        type_: v[3],
        options: v[4],
    }
}

fn write_mobj(d: &mut Doom, id: ThinkerId) {
    write_thinker(d, id);
    let m = d.p_tick.mobj(id).clone();
    let s = &mut d.p_saveg;
    for v in [
        m.x,
        m.y,
        m.z,
        ptr(m.snext),
        ptr(m.sprev),
        m.angle as i32,
        m.sprite as i32,
        m.frame,
    ] {
        s.write32(v);
    }
    for v in [
        ptr(m.bnext),
        ptr(m.bprev),
        ptr(Some(m.subsector)),
        m.floorz,
        m.ceilingz,
        m.radius,
        m.height,
    ] {
        s.write32(v);
    }
    for v in [
        m.momx,
        m.momy,
        m.momz,
        m.validcount,
        m.type_ as i32,
        ptr(Some(m.type_)),
        m.tics,
        m.state as i32,
    ] {
        s.write32(v);
    }
    for v in [
        m.flags,
        m.health,
        m.movedir,
        m.movecount,
        ptr(m.target),
        m.reactiontime,
        m.threshold,
    ] {
        s.write32(v);
    }
    s.write32(m.player.map_or(0, |p| p as i32 + 1));
    s.write32(m.lastlook);
    write_mapthing(s, &m.spawnpoint);
    s.write32(ptr(m.tracer));
}

fn read_mobj(s: &mut State) -> Mobj {
    read_thinker(s);
    let mut r = || s.read32();
    let (x, y, z) = (r(), r(), r());
    r();
    r();
    let (angle, sprite, frame) = (r() as u32, r() as usize, r());
    r();
    r();
    r();
    let (floorz, ceilingz, radius, height, momx, momy, momz, validcount) =
        (r(), r(), r(), r(), r(), r(), r(), r());
    let type_ = r() as usize;
    r();
    let (tics, state, flags, health, movedir, movecount) = (r(), r() as usize, r(), r(), r(), r());
    r();
    let (reactiontime, threshold, pl, lastlook) = (r(), r(), r(), r());
    let spawnpoint = read_mapthing(s);
    s.read32();
    Mobj {
        x,
        y,
        z,
        snext: None,
        sprev: None,
        angle,
        sprite,
        frame,
        bnext: None,
        bprev: None,
        subsector: 0,
        floorz,
        ceilingz,
        radius,
        height,
        momx,
        momy,
        momz,
        validcount,
        type_,
        tics,
        state,
        flags,
        health,
        movedir,
        movecount,
        target: None,
        reactiontime,
        threshold,
        player: (pl > 0).then(|| (pl - 1) as usize),
        lastlook,
        spawnpoint,
        tracer: None,
    }
}

fn write_player(d: &mut Doom, p: usize) {
    let pl = d.g_game.players[p].clone();
    let s = &mut d.p_saveg;
    s.write32(ptr(pl.mo));
    s.write32(pl.playerstate);
    s.write8(pl.cmd.forwardmove as u8);
    s.write8(pl.cmd.sidemove as u8);
    s.write16(pl.cmd.angleturn as i32);
    s.write16(pl.cmd.consistancy as i32);
    s.write8(pl.cmd.chatchar);
    s.write8(pl.cmd.buttons);
    for v in [
        pl.viewz,
        pl.viewheight,
        pl.deltaviewheight,
        pl.bob,
        pl.health,
        pl.armorpoints,
        pl.armortype,
    ] {
        s.write32(v);
    }
    pl.powers.iter().for_each(|&v| s.write32(v));
    pl.cards.iter().for_each(|&v| s.write32(v as i32));
    s.write32(pl.backpack as i32);
    pl.frags.iter().for_each(|&v| s.write32(v));
    s.write32(pl.readyweapon);
    s.write32(pl.pendingweapon);
    pl.weaponowned.iter().for_each(|&v| s.write32(v as i32));
    pl.ammo.iter().for_each(|&v| s.write32(v));
    pl.maxammo.iter().for_each(|&v| s.write32(v));
    for v in [
        pl.attackdown as i32,
        pl.usedown as i32,
        pl.cheats,
        pl.refire,
        pl.killcount,
        pl.itemcount,
        pl.secretcount,
    ] {
        s.write32(v);
    }
    s.write32(if pl.message.is_some() {
        ptr(Some(p))
    } else {
        0
    });
    s.write32(pl.damagecount);
    s.write32(pl.bonuscount);
    s.write32(ptr(pl.attacker));
    s.write32(pl.extralight);
    s.write32(pl.fixedcolormap);
    s.write32(pl.colormap);
    for ps in pl.psprites.iter() {
        s.write32(ps.state.map_or(0, |x| x as i32));
        s.write32(ps.tics);
        s.write32(ps.sx);
        s.write32(ps.sy);
    }
    s.write32(pl.didsecret as i32);
}

fn read_player(s: &mut State, pl: &mut Player) {
    s.read32();
    pl.playerstate = s.read32();
    pl.cmd.forwardmove = s.read8() as i8;
    pl.cmd.sidemove = s.read8() as i8;
    pl.cmd.angleturn = s.read16() as i16;
    pl.cmd.consistancy = s.read16() as i16;
    pl.cmd.chatchar = s.read8();
    pl.cmd.buttons = s.read8();
    for v in [
        &mut pl.viewz,
        &mut pl.viewheight,
        &mut pl.deltaviewheight,
        &mut pl.bob,
        &mut pl.health,
        &mut pl.armorpoints,
        &mut pl.armortype,
    ] {
        *v = s.read32();
    }
    pl.powers.iter_mut().for_each(|v| *v = s.read32());
    pl.cards.iter_mut().for_each(|v| *v = s.read32() != 0);
    pl.backpack = s.read32() != 0;
    pl.frags.iter_mut().for_each(|v| *v = s.read32());
    pl.readyweapon = s.read32();
    pl.pendingweapon = s.read32();
    pl.weaponowned.iter_mut().for_each(|v| *v = s.read32() != 0);
    pl.ammo.iter_mut().for_each(|v| *v = s.read32());
    pl.maxammo.iter_mut().for_each(|v| *v = s.read32());
    pl.attackdown = s.read32() != 0;
    pl.usedown = s.read32() != 0;
    for v in [
        &mut pl.cheats,
        &mut pl.refire,
        &mut pl.killcount,
        &mut pl.itemcount,
        &mut pl.secretcount,
    ] {
        *v = s.read32();
    }
    s.read32();
    pl.damagecount = s.read32();
    pl.bonuscount = s.read32();
    s.read32();
    pl.extralight = s.read32();
    pl.fixedcolormap = s.read32();
    pl.colormap = s.read32();
    for ps in pl.psprites.iter_mut() {
        let st = s.read32();
        ps.state = (st > 0).then_some(st as usize);
        ps.tics = s.read32();
        ps.sx = s.read32();
        ps.sy = s.read32();
    }
    pl.didsecret = s.read32() != 0;
}

/// P_WriteSaveGameHeader.
pub fn write_save_game_header(d: &mut Doom, description: &str) {
    let version = format!("version {}", crate::g_game::vanilla_version_code(d));
    let s = &mut d.p_saveg;
    let desc = description.as_bytes();
    desc.iter().for_each(|&b| s.write8(b));
    (desc.len()..SAVESTRINGSIZE).for_each(|_| s.write8(0));
    let mut name = [0u8; VERSIONSIZE];
    let n = version.len().min(VERSIONSIZE - 1);
    name[..n].copy_from_slice(&version.as_bytes()[..n]);
    name.iter().for_each(|&b| s.write8(b));
    let g = &d.g_game;
    s.write8(g.gameskill as u8);
    s.write8(g.gameepisode as u8);
    s.write8(g.gamemap as u8);
    g.playeringame.iter().for_each(|&b| s.write8(b as u8));
    let lt = d.p_tick.leveltime;
    s.write8((lt >> 16) as u8);
    s.write8((lt >> 8) as u8);
    s.write8(lt as u8);
}

/// P_ReadSaveGameHeader. It returns false when the version string differs.
pub fn read_save_game_header(d: &mut Doom) -> bool {
    let s = &mut d.p_saveg;
    (0..SAVESTRINGSIZE).for_each(|_| {
        s.read8();
    });
    let read: Vec<u8> = (0..VERSIONSIZE).map(|_| s.read8()).collect();
    let read = &read[..read.iter().position(|&b| b == 0).unwrap_or(VERSIONSIZE)];
    let version = format!("version {}", crate::g_game::vanilla_version_code(d));
    if read != version.as_bytes() {
        return false;
    }
    let s = &mut d.p_saveg;
    d.g_game.gameskill = s.read8() as i32;
    d.g_game.gameepisode = s.read8() as i32;
    d.g_game.gamemap = s.read8() as i32;
    for i in 0..MAXPLAYERS {
        d.g_game.playeringame[i] = s.read8() != 0;
    }
    let (a, b, c) = (s.read8() as i32, s.read8() as i32, s.read8() as i32);
    d.p_tick.leveltime = (a << 16) + (b << 8) + c;
    true
}

/// P_ReadSaveGameEOF.
pub fn read_save_game_eof(d: &mut Doom) -> bool {
    d.p_saveg.read8() == SAVEGAME_EOF
}

/// P_WriteSaveGameEOF.
pub fn write_save_game_eof(d: &mut Doom) {
    d.p_saveg.write8(SAVEGAME_EOF);
}

/// P_ArchivePlayers.
pub fn archive_players(d: &mut Doom) {
    for i in 0..MAXPLAYERS {
        if d.g_game.playeringame[i] {
            d.p_saveg.write_pad();
            write_player(d, i);
        }
    }
}

/// P_UnArchivePlayers.
pub fn unarchive_players(d: &mut Doom) {
    // The zone model replays the level setup while its thinker ids are valid.
    crate::r_data::zone_sync(d);
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        d.p_saveg.read_pad();
        read_player(&mut d.p_saveg, &mut d.g_game.players[i]);
        let pl = &mut d.g_game.players[i];
        pl.mo = None;
        pl.message = None;
        pl.attacker = None;
    }
}

/// P_ArchiveWorld.
pub fn archive_world(d: &mut Doom) {
    let s = &mut d.p_saveg;
    for sec in &d.p_setup.sectors[..d.p_setup.numsectors] {
        for v in [
            sec.floorheight >> FRACBITS,
            sec.ceilingheight >> FRACBITS,
            sec.floorpic,
            sec.ceilingpic,
            sec.lightlevel,
            sec.special,
            sec.tag,
        ] {
            s.write16(v);
        }
    }
    for li in d.p_setup.lines.iter() {
        s.write16(li.flags);
        s.write16(li.special);
        s.write16(li.tag);
        for j in 0..2 {
            if li.sidenum[j] == -1 {
                continue;
            }
            let si = &d.p_setup.sides[li.sidenum[j] as usize];
            for v in [
                si.textureoffset >> FRACBITS,
                si.rowoffset >> FRACBITS,
                si.toptexture,
                si.bottomtexture,
                si.midtexture,
            ] {
                s.write16(v);
            }
        }
    }
}

/// P_UnArchiveWorld.
pub fn unarchive_world(d: &mut Doom) {
    let s = &mut d.p_saveg;
    for sec in &mut d.p_setup.sectors[..d.p_setup.numsectors] {
        sec.floorheight = s.read16() << FRACBITS;
        sec.ceilingheight = s.read16() << FRACBITS;
        sec.floorpic = s.read16();
        sec.ceilingpic = s.read16();
        sec.lightlevel = s.read16();
        sec.special = s.read16();
        sec.tag = s.read16();
        sec.specialdata = None;
        sec.soundtarget = None;
    }
    let st = &mut d.p_setup;
    for li in st.lines.iter_mut() {
        li.flags = s.read16();
        li.special = s.read16();
        li.tag = s.read16();
        for j in 0..2 {
            if li.sidenum[j] == -1 {
                continue;
            }
            let si = &mut st.sides[li.sidenum[j] as usize];
            si.textureoffset = s.read16() << FRACBITS;
            si.rowoffset = s.read16() << FRACBITS;
            si.toptexture = s.read16();
            si.bottomtexture = s.read16();
            si.midtexture = s.read16();
        }
    }
}

/// P_ArchiveThinkers.
pub fn archive_thinkers(d: &mut Doom) {
    for id in d.p_tick.ids() {
        if d.p_tick.thinkers[id].func == Think::Mobj {
            d.p_saveg.write8(TC_MOBJ);
            d.p_saveg.write_pad();
            write_mobj(d, id);
        }
    }
    d.p_saveg.write8(TC_END);
}

/// P_UnArchiveThinkers.
pub fn unarchive_thinkers(d: &mut Doom) {
    for id in d.p_tick.ids() {
        if d.p_tick.thinkers[id].func == Think::Mobj {
            crate::p_mobj::remove_mobj(d, id);
        } else {
            d.p_tick.zone_log.push(crate::p_tick::ZoneEvent::Free(id));
        }
    }
    // The original frees the other thinkers and leaks the mobj blocks. It sets up no level here.
    crate::r_data::zone_sync(d);
    d.p_tick.init_thinkers();
    d.p_tick.zone_log.pop();
    loop {
        let tclass = d.p_saveg.read8();
        match tclass {
            TC_END => return,
            TC_MOBJ => {
                d.p_saveg.read_pad();
                let m = read_mobj(&mut d.p_saveg);
                let player = m.player;
                let id = d
                    .p_tick
                    .add_thinker(Think::Mobj, ThinkerData::Mobj(Box::new(m)));
                if let Some(p) = player {
                    d.g_game.players[p].mo = Some(id);
                }
                crate::p_maputl::set_thing_position(d, id);
                let sec = d.p_setup.subsectors[d.p_tick.mobj(id).subsector].sector;
                let (f, c) = (
                    d.p_setup.sectors[sec].floorheight,
                    d.p_setup.sectors[sec].ceilingheight,
                );
                let m = d.p_tick.mobj_mut(id);
                m.floorz = f;
                m.ceilingz = c;
            }
            _ => crate::i_system::error(d, &format!("Unknown tclass {} in savegame", tclass)),
        }
    }
}

fn write_special(d: &mut Doom, tc: u8, id: ThinkerId) {
    d.p_saveg.write8(tc);
    d.p_saveg.write_pad();
    write_thinker(d, id);
    let s = &mut d.p_saveg;
    match d.p_tick.thinkers[id].data {
        ThinkerData::Ceiling(c) => {
            for v in [
                c.type_,
                c.sector as i32,
                c.bottomheight,
                c.topheight,
                c.speed,
                c.crush as i32,
                c.direction,
                c.tag,
                c.olddirection,
            ] {
                s.write32(v);
            }
        }
        ThinkerData::Door(v) => {
            for x in [
                v.type_,
                v.sector as i32,
                v.topheight,
                v.speed,
                v.direction,
                v.topwait,
                v.topcountdown,
            ] {
                s.write32(x);
            }
        }
        ThinkerData::Floor(f) => {
            for x in [f.type_, f.crush, f.sector as i32, f.direction, f.newspecial] {
                s.write32(x);
            }
            s.write16(f.texture);
            s.write32(f.floordestheight);
            s.write32(f.speed);
        }
        ThinkerData::Plat(p) => {
            for x in [
                p.sector as i32,
                p.speed,
                p.low,
                p.high,
                p.wait,
                p.count,
                p.status,
                p.oldstatus,
                p.crush as i32,
                p.tag,
                p.type_,
            ] {
                s.write32(x);
            }
        }
        ThinkerData::LightFlash(f) => {
            for x in [
                f.sector as i32,
                f.count,
                f.maxlight,
                f.minlight,
                f.maxtime,
                f.mintime,
            ] {
                s.write32(x);
            }
        }
        ThinkerData::Strobe(f) => {
            for x in [
                f.sector as i32,
                f.count,
                f.minlight,
                f.maxlight,
                f.darktime,
                f.brighttime,
            ] {
                s.write32(x);
            }
        }
        ThinkerData::Glow(g) => {
            for x in [g.sector as i32, g.minlight, g.maxlight, g.direction] {
                s.write32(x);
            }
        }
        _ => {}
    }
}

/// P_ArchiveSpecials. A stopped ceiling is saved only when it is in the active list; a stopped platform is lost.
pub fn archive_specials(d: &mut Doom) {
    for id in d.p_tick.ids() {
        let tc = match d.p_tick.thinkers[id].func {
            Think::Null => {
                if d.p_ceilng.activeceilings.contains(&Some(id)) {
                    write_special(d, TC_CEILING, id);
                }
                continue;
            }
            Think::MoveCeiling => TC_CEILING,
            Think::VerticalDoor => TC_DOOR,
            Think::MoveFloor => TC_FLOOR,
            Think::PlatRaise => TC_PLAT,
            Think::LightFlash => TC_FLASH,
            Think::StrobeFlash => TC_STROBE,
            Think::Glow => TC_GLOW,
            _ => continue,
        };
        write_special(d, tc, id);
    }
    d.p_saveg.write8(TC_ENDSPECIALS);
}

/// P_UnArchiveSpecials.
pub fn unarchive_specials(d: &mut Doom) {
    loop {
        let tclass = d.p_saveg.read8();
        if tclass == TC_ENDSPECIALS {
            return;
        }
        if tclass > TC_GLOW {
            crate::i_system::error(
                d,
                &format!("P_UnarchiveSpecials:Unknown tclass {} in savegame", tclass),
            );
        }
        let s = &mut d.p_saveg;
        s.read_pad();
        let active = read_thinker(s);
        let mut r = || s.read32();
        let (func, data, sector) = match tclass {
            TC_CEILING => {
                let c = Ceiling {
                    type_: r(),
                    sector: r() as usize,
                    bottomheight: r(),
                    topheight: r(),
                    speed: r(),
                    crush: r() != 0,
                    direction: r(),
                    tag: r(),
                    olddirection: r(),
                };
                (
                    if active {
                        Think::MoveCeiling
                    } else {
                        Think::Null
                    },
                    ThinkerData::Ceiling(c),
                    Some(c.sector),
                )
            }
            TC_DOOR => {
                let v = VlDoor {
                    type_: r(),
                    sector: r() as usize,
                    topheight: r(),
                    speed: r(),
                    direction: r(),
                    topwait: r(),
                    topcountdown: r(),
                };
                (Think::VerticalDoor, ThinkerData::Door(v), Some(v.sector))
            }
            TC_FLOOR => {
                let (type_, crush, sector, direction, newspecial) =
                    (r(), r(), r() as usize, r(), r());
                let texture = s.read16();
                let f = FloorMove {
                    type_,
                    crush,
                    sector,
                    direction,
                    newspecial,
                    texture,
                    floordestheight: s.read32(),
                    speed: s.read32(),
                };
                (Think::MoveFloor, ThinkerData::Floor(f), Some(sector))
            }
            TC_PLAT => {
                let p = Plat {
                    sector: r() as usize,
                    speed: r(),
                    low: r(),
                    high: r(),
                    wait: r(),
                    count: r(),
                    status: r(),
                    oldstatus: r(),
                    crush: r() != 0,
                    tag: r(),
                    type_: r(),
                };
                (
                    if active {
                        Think::PlatRaise
                    } else {
                        Think::Null
                    },
                    ThinkerData::Plat(p),
                    Some(p.sector),
                )
            }
            TC_FLASH => {
                let f = LightFlash {
                    sector: r() as usize,
                    count: r(),
                    maxlight: r(),
                    minlight: r(),
                    maxtime: r(),
                    mintime: r(),
                };
                (Think::LightFlash, ThinkerData::LightFlash(f), None)
            }
            TC_STROBE => {
                let f = Strobe {
                    sector: r() as usize,
                    count: r(),
                    minlight: r(),
                    maxlight: r(),
                    darktime: r(),
                    brighttime: r(),
                };
                (Think::StrobeFlash, ThinkerData::Strobe(f), None)
            }
            _ => {
                let g = Glow {
                    sector: r() as usize,
                    minlight: r(),
                    maxlight: r(),
                    direction: r(),
                };
                (Think::Glow, ThinkerData::Glow(g), None)
            }
        };
        let id = d.p_tick.add_thinker(func, data);
        if let Some(sec) = sector {
            d.p_setup.sectors[sec].specialdata = Some(id);
        }
        match tclass {
            TC_CEILING => crate::p_ceilng::add_active_ceiling(d, id),
            TC_PLAT => crate::p_plats::add_active_plat(d, id),
            _ => {}
        }
    }
}

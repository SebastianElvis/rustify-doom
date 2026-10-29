//! Interactions: item pickups, damage, and death.

use crate::d_items::*;
use crate::g_game::{CF_GODMODE, PST_DEAD, SK_BABY, SK_NIGHTMARE};
use crate::info::{self, MOBJINFO};
use crate::m_fixed::{fixed_mul, FRACUNIT};
use crate::m_random::p_random;
use crate::p_mobj::{
    self, MobjId, MF_CORPSE, MF_COUNTITEM, MF_COUNTKILL, MF_DROPOFF, MF_DROPPED, MF_FLOAT,
    MF_JUSTHIT, MF_NOCLIP, MF_NOGRAVITY, MF_SHADOW, MF_SHOOTABLE, MF_SKULLFLY, MF_SOLID, ONFLOORZ,
};
use crate::s_sound::{self, SFX_GETPOW, SFX_ITEMUP, SFX_WPNUP};
use crate::tables::{finecosine, finesine, ANG180, ANGLETOFINESHIFT};
use crate::Doom;

pub const BONUSADD: i32 = 6;
pub const MAXHEALTH: i32 = 100;
const BASETHRESHOLD: i32 = 100;
pub const MAXAMMO: [i32; NUMAMMO] = [200, 50, 300, 50];
pub const CLIPAMMO: [i32; NUMAMMO] = [10, 4, 20, 1];

#[derive(Default)]
pub struct State {}

/// P_GiveAmmo: `num` is the number of clips; zero means half a clip.
pub fn give_ammo(d: &mut Doom, player: usize, ammo: i32, num: i32) -> bool {
    if ammo == AM_NOAMMO {
        return false;
    }
    if ammo as u32 >= NUMAMMO as u32 {
        crate::i_system::error(d, &format!("P_GiveAmmo: bad type {}", ammo));
    }
    let skill = d.g_game.gameskill;
    let p = &mut d.g_game.players[player];
    let a = ammo as usize;
    if p.ammo[a] == p.maxammo[a] {
        return false;
    }
    let mut num = if num != 0 {
        num * CLIPAMMO[a]
    } else {
        CLIPAMMO[a] / 2
    };
    if skill == SK_BABY || skill == SK_NIGHTMARE {
        num <<= 1;
    }
    let oldammo = p.ammo[a];
    p.ammo[a] += num;
    if p.ammo[a] > p.maxammo[a] {
        p.ammo[a] = p.maxammo[a];
    }
    if oldammo != 0 {
        return true;
    }
    match ammo {
        AM_CLIP => {
            if p.readyweapon == WP_FIST {
                p.pendingweapon = if p.weaponowned[WP_CHAINGUN as usize] {
                    WP_CHAINGUN
                } else {
                    WP_PISTOL
                };
            }
        }
        AM_SHELL => {
            if (p.readyweapon == WP_FIST || p.readyweapon == WP_PISTOL)
                && p.weaponowned[WP_SHOTGUN as usize]
            {
                p.pendingweapon = WP_SHOTGUN;
            }
        }
        AM_CELL => {
            if (p.readyweapon == WP_FIST || p.readyweapon == WP_PISTOL)
                && p.weaponowned[WP_PLASMA as usize]
            {
                p.pendingweapon = WP_PLASMA;
            }
        }
        AM_MISL => {
            if p.readyweapon == WP_FIST && p.weaponowned[WP_MISSILE as usize] {
                p.pendingweapon = WP_MISSILE;
            }
        }
        _ => {}
    }
    true
}

/// P_GiveWeapon: `dropped` gives one clip instead of two.
pub fn give_weapon(d: &mut Doom, player: usize, weapon: i32, dropped: bool) -> bool {
    let w = weapon as usize;
    let ammo = WEAPONINFO[w].ammo;
    if d.g_game.netgame && d.g_game.deathmatch != 2 && !dropped {
        if d.g_game.players[player].weaponowned[w] {
            return false;
        }
        let p = &mut d.g_game.players[player];
        p.bonuscount += BONUSADD;
        p.weaponowned[w] = true;
        let clips = if d.g_game.deathmatch != 0 { 5 } else { 2 };
        give_ammo(d, player, ammo, clips);
        d.g_game.players[player].pendingweapon = weapon;
        if player == d.g_game.consoleplayer {
            s_sound::start_sound(d, None, SFX_WPNUP);
        }
        return false;
    }
    let gaveammo = if ammo != AM_NOAMMO {
        give_ammo(d, player, ammo, if dropped { 1 } else { 2 })
    } else {
        false
    };
    let p = &mut d.g_game.players[player];
    let gaveweapon = if p.weaponowned[w] {
        false
    } else {
        p.weaponowned[w] = true;
        p.pendingweapon = weapon;
        true
    };
    gaveweapon || gaveammo
}

/// P_GiveBody: returns false when the health is already full.
pub fn give_body(d: &mut Doom, player: usize, num: i32) -> bool {
    let p = &mut d.g_game.players[player];
    if p.health >= MAXHEALTH {
        return false;
    }
    p.health += num;
    if p.health > MAXHEALTH {
        p.health = MAXHEALTH;
    }
    let (h, mo) = (p.health, p.mo.unwrap());
    d.p_tick.mobj_mut(mo).health = h;
    true
}

/// P_GiveArmor: armor type 1 is green, type 2 is blue.
pub fn give_armor(d: &mut Doom, player: usize, armortype: i32) -> bool {
    let p = &mut d.g_game.players[player];
    let hits = armortype * 100;
    if p.armorpoints >= hits {
        return false;
    }
    p.armortype = armortype;
    p.armorpoints = hits;
    true
}

/// P_GiveCard.
pub fn give_card(d: &mut Doom, player: usize, card: usize) {
    let p = &mut d.g_game.players[player];
    if p.cards[card] {
        return;
    }
    p.bonuscount = BONUSADD;
    p.cards[card] = true;
}

/// P_GivePower.
pub fn give_power(d: &mut Doom, player: usize, power: usize) -> bool {
    let p = &mut d.g_game.players[player];
    match power {
        PW_INVULNERABILITY => p.powers[power] = INVULNTICS,
        PW_INVISIBILITY => {
            p.powers[power] = INVISTICS;
            let mo = p.mo.unwrap();
            d.p_tick.mobj_mut(mo).flags |= MF_SHADOW;
        }
        PW_INFRARED => p.powers[power] = INFRATICS,
        PW_IRONFEET => p.powers[power] = IRONTICS,
        PW_STRENGTH => {
            give_body(d, player, 100);
            d.g_game.players[player].powers[power] = 1;
        }
        _ => {
            if p.powers[power] != 0 {
                return false;
            }
            p.powers[power] = 1;
        }
    }
    true
}

/// P_TouchSpecialThing: the toucher picks up the special item.
pub fn touch_special_thing(d: &mut Doom, special: MobjId, toucher: MobjId) {
    let (sz, sprite, sflags) = {
        let s = d.p_tick.mobj(special);
        (s.z, s.sprite, s.flags)
    };
    let (tz, theight, thealth, tplayer) = {
        let t = d.p_tick.mobj(toucher);
        (t.z, t.height, t.health, t.player)
    };
    let delta = sz.wrapping_sub(tz);
    if delta > theight || delta < -8 * FRACUNIT {
        return;
    }
    let mut sound = SFX_ITEMUP;
    if thealth <= 0 {
        return;
    }
    let pl = tplayer.unwrap();
    let netgame = d.g_game.netgame;
    let msg: &'static str;
    macro_rules! p {
        () => {
            d.g_game.players[pl]
        };
    }
    macro_rules! card {
        ($card:expr, $text:expr) => {{
            if !p!().cards[$card] {
                p!().message = Some($text.to_string());
            }
            give_card(d, pl, $card);
            if netgame {
                return;
            }
            msg = "";
        }};
    }
    macro_rules! power {
        ($pw:expr, $text:expr) => {{
            if !give_power(d, pl, $pw) {
                return;
            }
            msg = $text;
            sound = SFX_GETPOW;
        }};
    }
    macro_rules! ammo {
        ($am:expr, $num:expr, $text:expr) => {{
            if !give_ammo(d, pl, $am, $num) {
                return;
            }
            msg = $text;
        }};
    }
    macro_rules! weapon {
        ($wp:expr, $dropped:expr, $text:expr) => {{
            if !give_weapon(d, pl, $wp, $dropped) {
                return;
            }
            msg = $text;
            sound = SFX_WPNUP;
        }};
    }
    let dropped = sflags & MF_DROPPED != 0;
    match sprite {
        info::SPR_ARM1 => {
            if !give_armor(d, pl, 1) {
                return;
            }
            msg = "Picked up the armor.";
        }
        info::SPR_ARM2 => {
            if !give_armor(d, pl, 2) {
                return;
            }
            msg = "Picked up the MegaArmor!";
        }
        info::SPR_BON1 => {
            let p = &mut p!();
            p.health += 1;
            if p.health > 200 {
                p.health = 200;
            }
            let (h, mo) = (p.health, p.mo.unwrap());
            d.p_tick.mobj_mut(mo).health = h;
            msg = "Picked up a health bonus.";
        }
        info::SPR_BON2 => {
            let p = &mut p!();
            p.armorpoints += 1;
            if p.armorpoints > 200 {
                p.armorpoints = 200;
            }
            if p.armortype == 0 {
                p.armortype = 1;
            }
            msg = "Picked up an armor bonus.";
        }
        info::SPR_SOUL => {
            let p = &mut p!();
            p.health += 100;
            if p.health > 200 {
                p.health = 200;
            }
            let (h, mo) = (p.health, p.mo.unwrap());
            d.p_tick.mobj_mut(mo).health = h;
            msg = "Supercharge!";
            sound = SFX_GETPOW;
        }
        info::SPR_MEGA => {
            if d.d_main.gamemode != crate::d_main::COMMERCIAL {
                return;
            }
            let p = &mut p!();
            p.health = 200;
            let mo = p.mo.unwrap();
            d.p_tick.mobj_mut(mo).health = 200;
            give_armor(d, pl, 2);
            msg = "MegaSphere!";
            sound = SFX_GETPOW;
        }
        info::SPR_BKEY => card!(IT_BLUECARD, "Picked up a blue keycard."),
        info::SPR_YKEY => card!(IT_YELLOWCARD, "Picked up a yellow keycard."),
        info::SPR_RKEY => card!(IT_REDCARD, "Picked up a red keycard."),
        info::SPR_BSKU => card!(IT_BLUESKULL, "Picked up a blue skull key."),
        info::SPR_YSKU => card!(IT_YELLOWSKULL, "Picked up a yellow skull key."),
        info::SPR_RSKU => card!(IT_REDSKULL, "Picked up a red skull key."),
        info::SPR_STIM => {
            if !give_body(d, pl, 10) {
                return;
            }
            msg = "Picked up a stimpack.";
        }
        info::SPR_MEDI => {
            if !give_body(d, pl, 25) {
                return;
            }
            // The check runs after the pickup, so the first message shows only for a negative start health.
            msg = if p!().health < 25 {
                "Picked up a medikit that you REALLY need!"
            } else {
                "Picked up a medikit."
            };
        }
        info::SPR_PINV => power!(PW_INVULNERABILITY, "Invulnerability!"),
        info::SPR_PSTR => {
            power!(PW_STRENGTH, "Berserk!");
            if p!().readyweapon != WP_FIST {
                p!().pendingweapon = WP_FIST;
            }
        }
        info::SPR_PINS => power!(PW_INVISIBILITY, "Partial Invisibility"),
        info::SPR_SUIT => power!(PW_IRONFEET, "Radiation Shielding Suit"),
        info::SPR_PMAP => power!(PW_ALLMAP, "Computer Area Map"),
        info::SPR_PVIS => power!(PW_INFRARED, "Light Amplification Visor"),
        info::SPR_CLIP => ammo!(AM_CLIP, if dropped { 0 } else { 1 }, "Picked up a clip."),
        info::SPR_AMMO => ammo!(AM_CLIP, 5, "Picked up a box of bullets."),
        info::SPR_ROCK => ammo!(AM_MISL, 1, "Picked up a rocket."),
        info::SPR_BROK => ammo!(AM_MISL, 5, "Picked up a box of rockets."),
        info::SPR_CELL => ammo!(AM_CELL, 1, "Picked up an energy cell."),
        info::SPR_CELP => ammo!(AM_CELL, 5, "Picked up an energy cell pack."),
        info::SPR_SHEL => ammo!(AM_SHELL, 1, "Picked up 4 shotgun shells."),
        info::SPR_SBOX => ammo!(AM_SHELL, 5, "Picked up a box of shotgun shells."),
        info::SPR_BPAK => {
            if !p!().backpack {
                for i in 0..NUMAMMO {
                    p!().maxammo[i] *= 2;
                }
                p!().backpack = true;
            }
            for i in 0..NUMAMMO {
                give_ammo(d, pl, i as i32, 1);
            }
            msg = "Picked up a backpack full of ammo!";
        }
        info::SPR_BFUG => weapon!(WP_BFG, false, "You got the BFG9000!  Oh, yes."),
        info::SPR_MGUN => weapon!(WP_CHAINGUN, dropped, "You got the chaingun!"),
        info::SPR_CSAW => weapon!(WP_CHAINSAW, false, "A chainsaw!  Find some meat!"),
        info::SPR_LAUN => weapon!(WP_MISSILE, false, "You got the rocket launcher!"),
        info::SPR_PLAS => weapon!(WP_PLASMA, false, "You got the plasma gun!"),
        info::SPR_SHOT => weapon!(WP_SHOTGUN, dropped, "You got the shotgun!"),
        info::SPR_SGN2 => weapon!(WP_SUPERSHOTGUN, dropped, "You got the super shotgun!"),
        _ => crate::i_system::error(d, "P_SpecialThing: Unknown gettable thing"),
    }
    if !msg.is_empty() {
        p!().message = Some(msg.to_string());
    }
    if sflags & MF_COUNTITEM != 0 {
        p!().itemcount += 1;
    }
    p_mobj::remove_mobj(d, special);
    p!().bonuscount += BONUSADD;
    if pl == d.g_game.consoleplayer {
        s_sound::start_sound(d, None, sound);
    }
}

/// P_KillMobj.
pub fn kill_mobj(d: &mut Doom, source: Option<MobjId>, target: MobjId) {
    let (tflags, tplayer, ttype) = {
        let t = d.p_tick.mobj_mut(target);
        t.flags &= !(MF_SHOOTABLE | MF_FLOAT | MF_SKULLFLY);
        if t.type_ != info::MT_SKULL {
            t.flags &= !MF_NOGRAVITY;
        }
        t.flags |= MF_CORPSE | MF_DROPOFF;
        t.height >>= 2;
        (t.flags, t.player, t.type_)
    };
    let splayer = source.and_then(|s| d.p_tick.mobj(s).player);
    if let Some(sp) = splayer {
        if tflags & MF_COUNTKILL != 0 {
            d.g_game.players[sp].killcount += 1;
        }
        if let Some(tp) = tplayer {
            d.g_game.players[sp].frags[tp] += 1;
        }
    } else if !d.g_game.netgame && tflags & MF_COUNTKILL != 0 {
        d.g_game.players[0].killcount += 1;
    }
    if let Some(tp) = tplayer {
        if source.is_none() {
            d.g_game.players[tp].frags[tp] += 1;
        }
        d.p_tick.mobj_mut(target).flags &= !MF_SOLID;
        d.g_game.players[tp].playerstate = PST_DEAD;
        crate::p_pspr::drop_weapon(d, tp);
        if tp == d.g_game.consoleplayer && d.am_map.automapactive {
            crate::am_map::stop(d);
        }
    }
    let inf = &MOBJINFO[ttype];
    let health = d.p_tick.mobj(target).health;
    if health < -inf.spawnhealth && inf.xdeathstate != 0 {
        p_mobj::set_mobj_state(d, target, inf.xdeathstate as usize);
    } else {
        p_mobj::set_mobj_state(d, target, inf.deathstate as usize);
    }
    let r = p_random(&mut d.m_random) & 3;
    {
        let t = d.p_tick.mobj_mut(target);
        t.tics -= r;
        if t.tics < 1 {
            t.tics = 1;
        }
    }
    if d.d_main.gameversion == crate::d_main::EXE_CHEX {
        return;
    }
    let item = match ttype {
        info::MT_WOLFSS | info::MT_POSSESSED => info::MT_CLIP,
        info::MT_SHOTGUY => info::MT_SHOTGUN,
        info::MT_CHAINGUY => info::MT_CHAINGUN,
        _ => return,
    };
    let (x, y) = {
        let t = d.p_tick.mobj(target);
        (t.x, t.y)
    };
    let mo = p_mobj::spawn_mobj(d, x, y, ONFLOORZ, item);
    d.p_tick.mobj_mut(mo).flags |= MF_DROPPED;
}

/// P_DamageMobj: the inflictor hurts the target; the source is the attacker.
pub fn damage_mobj(
    d: &mut Doom,
    target: MobjId,
    inflictor: Option<MobjId>,
    source: Option<MobjId>,
    damage: i32,
) {
    let mut damage = damage;
    let (flags, health, tplayer, ttype) = {
        let t = d.p_tick.mobj(target);
        (t.flags, t.health, t.player, t.type_)
    };
    if flags & MF_SHOOTABLE == 0 || health <= 0 {
        return;
    }
    if flags & MF_SKULLFLY != 0 {
        let t = d.p_tick.mobj_mut(target);
        t.momx = 0;
        t.momy = 0;
        t.momz = 0;
    }
    if tplayer.is_some() && d.g_game.gameskill == SK_BABY {
        damage >>= 1;
    }
    let inf = &MOBJINFO[ttype];
    if let Some(inflictor) = inflictor {
        let chainsaw = source
            .and_then(|s| d.p_tick.mobj(s).player)
            .is_some_and(|sp| d.g_game.players[sp].readyweapon == WP_CHAINSAW);
        if flags & MF_NOCLIP == 0 && !chainsaw {
            let (ix, iy, iz) = {
                let i = d.p_tick.mobj(inflictor);
                (i.x, i.y, i.z)
            };
            let (tx, ty, tz) = {
                let t = d.p_tick.mobj(target);
                (t.x, t.y, t.z)
            };
            let mut ang = crate::r_main::point_to_angle2(ix, iy, tx, ty);
            let mut thrust = damage.wrapping_mul((FRACUNIT >> 3) * 100) / inf.mass;
            if damage < 40
                && damage > health
                && tz.wrapping_sub(iz) > 64 * FRACUNIT
                && p_random(&mut d.m_random) & 1 != 0
            {
                ang = ang.wrapping_add(ANG180);
                thrust = thrust.wrapping_mul(4);
            }
            let fine = (ang >> ANGLETOFINESHIFT) as usize;
            let t = d.p_tick.mobj_mut(target);
            t.momx = t.momx.wrapping_add(fixed_mul(thrust, finecosine(fine)));
            t.momy = t.momy.wrapping_add(fixed_mul(thrust, finesine(fine)));
        }
    }
    if let Some(pl) = tplayer {
        let sec = d.p_setup.subsectors[d.p_tick.mobj(target).subsector].sector;
        if d.p_setup.sectors[sec].special == 11 && damage >= health {
            damage = health - 1;
        }
        let p = &mut d.g_game.players[pl];
        if damage < 1000 && (p.cheats & CF_GODMODE != 0 || p.powers[PW_INVULNERABILITY] != 0) {
            return;
        }
        if p.armortype != 0 {
            let mut saved = if p.armortype == 1 {
                damage / 3
            } else {
                damage / 2
            };
            if p.armorpoints <= saved {
                saved = p.armorpoints;
                p.armortype = 0;
            }
            p.armorpoints -= saved;
            damage -= saved;
        }
        p.health -= damage;
        if p.health < 0 {
            p.health = 0;
        }
        p.attacker = source;
        p.damagecount += damage;
        if p.damagecount > 100 {
            p.damagecount = 100;
        }
    }
    let newhealth = health - damage;
    d.p_tick.mobj_mut(target).health = newhealth;
    if newhealth <= 0 {
        kill_mobj(d, source, target);
        return;
    }
    if p_random(&mut d.m_random) < inf.painchance && flags & MF_SKULLFLY == 0 {
        d.p_tick.mobj_mut(target).flags = flags | MF_JUSTHIT;
        p_mobj::set_mobj_state(d, target, inf.painstate as usize);
    }
    d.p_tick.mobj_mut(target).reactiontime = 0;
    let threshold = d.p_tick.mobj(target).threshold;
    if let Some(src) = source {
        if (threshold == 0 || ttype == info::MT_VILE)
            && src != target
            && d.p_tick.mobj(src).type_ != info::MT_VILE
        {
            let t = d.p_tick.mobj_mut(target);
            t.target = Some(src);
            t.threshold = BASETHRESHOLD;
            if t.state == inf.spawnstate as usize && inf.seestate != info::S_NULL as i32 {
                p_mobj::set_mobj_state(d, target, inf.seestate as usize);
            }
        }
    }
}

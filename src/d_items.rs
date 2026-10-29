//! Weapon, ammunition, key card, and power definitions.

pub const WP_FIST: i32 = 0;
pub const WP_PISTOL: i32 = 1;
pub const WP_SHOTGUN: i32 = 2;
pub const WP_CHAINGUN: i32 = 3;
pub const WP_MISSILE: i32 = 4;
pub const WP_PLASMA: i32 = 5;
pub const WP_BFG: i32 = 6;
pub const WP_CHAINSAW: i32 = 7;
pub const WP_SUPERSHOTGUN: i32 = 8;
pub const NUMWEAPONS: usize = 9;
pub const WP_NOCHANGE: i32 = 10;

pub const AM_CLIP: i32 = 0;
pub const AM_SHELL: i32 = 1;
pub const AM_CELL: i32 = 2;
pub const AM_MISL: i32 = 3;
pub const NUMAMMO: usize = 4;
pub const AM_NOAMMO: i32 = 5;

pub const IT_BLUECARD: usize = 0;
pub const IT_YELLOWCARD: usize = 1;
pub const IT_REDCARD: usize = 2;
pub const IT_BLUESKULL: usize = 3;
pub const IT_YELLOWSKULL: usize = 4;
pub const IT_REDSKULL: usize = 5;
pub const NUMCARDS: usize = 6;

pub const PW_INVULNERABILITY: usize = 0;
pub const PW_STRENGTH: usize = 1;
pub const PW_INVISIBILITY: usize = 2;
pub const PW_IRONFEET: usize = 3;
pub const PW_ALLMAP: usize = 4;
pub const PW_INFRARED: usize = 5;
pub const NUMPOWERS: usize = 6;

pub const INVULNTICS: i32 = 30 * 35;
pub const INVISTICS: i32 = 60 * 35;
pub const INFRATICS: i32 = 120 * 35;
pub const IRONTICS: i32 = 60 * 35;

#[derive(Clone, Copy, Debug)]
pub struct WeaponInfo {
    pub ammo: i32,
    pub upstate: usize,
    pub downstate: usize,
    pub readystate: usize,
    pub atkstate: usize,
    pub flashstate: usize,
}

include!("data/weaponinfo.in");

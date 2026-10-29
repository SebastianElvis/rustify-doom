//! IWAD search: the -iwad option and the fixed list of IWAD names in the current directory.

use crate::d_main::{
    MISSION_DOOM, MISSION_DOOM2, MISSION_NONE, MISSION_PACK_CHEX, MISSION_PACK_HACX,
    MISSION_PACK_PLUT, MISSION_PACK_TNT,
};
use crate::m_misc::file_exists;
use crate::Doom;

#[derive(Default)]
pub struct State {}

/// The IWAD names in search order with their missions.
const IWADS: [(&str, i32); 10] = [
    ("doom2.wad", MISSION_DOOM2),
    ("plutonia.wad", MISSION_PACK_PLUT),
    ("tnt.wad", MISSION_PACK_TNT),
    ("doom.wad", MISSION_DOOM),
    ("doom1.wad", MISSION_DOOM),
    ("chex.wad", MISSION_PACK_CHEX),
    ("hacx.wad", MISSION_PACK_HACX),
    ("freedm.wad", MISSION_DOOM2),
    ("freedoom2.wad", MISSION_DOOM2),
    ("freedoom1.wad", MISSION_DOOM),
];

/// The IWAD directory list. The original holds only ".".
const DIR: &str = ".";

/// D_FindWADByName: the name itself when it exists, else the name inside the IWAD directory.
pub fn find_wad_by_name(name: &str) -> Option<String> {
    if file_exists(name) {
        return Some(name.to_string());
    }
    let joined = format!("{DIR}/{name}");
    file_exists(&joined).then_some(joined)
}

/// D_TryFindWADByName: the found path, or the name unchanged.
pub fn try_find_wad_by_name(name: &str) -> String {
    find_wad_by_name(name).unwrap_or_else(|| name.to_string())
}

/// IdentifyIWADByName: the mission of a known IWAD base name, else MISSION_NONE.
fn identify_iwad_by_name(path: &str) -> i32 {
    let base = path.rsplit('/').next().unwrap_or(path);
    IWADS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(base))
        .map_or(MISSION_NONE, |&(_, m)| m)
}

/// D_FindIWAD: returns the IWAD path and its mission.
pub fn find_iwad(d: &mut Doom) -> Option<(String, i32)> {
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-iwad", 1);
    if p > 0 {
        let name = d.m_argv.myargv[p + 1].clone();
        let Some(path) = find_wad_by_name(&name) else {
            crate::i_system::error(d, &format!("IWAD file '{name}' not found!"));
        };
        let mission = identify_iwad_by_name(&path);
        return Some((path, mission));
    }
    println!("-iwad not specified, trying a few iwad names");
    for &(name, mission) in IWADS.iter() {
        let path = if DIR == "." {
            name.to_string()
        } else {
            format!("{DIR}/{name}")
        };
        println!("Trying IWAD file:{path}");
        if file_exists(&path) {
            return Some((path, mission));
        }
    }
    None
}

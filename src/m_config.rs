//! Configuration files. The original build reports the file names but neither reads nor writes them (EV-002).

use crate::Doom;

#[derive(Default)]
pub struct State {
    pub configdir: String,
    pub default_main_config: String,
    pub default_extra_config: String,
}

/// M_LoadDefaults: reports the main and the extra configuration file names.
pub fn load_defaults(d: &mut Doom) {
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-config", 1);
    if p > 0 {
        d.m_config.default_main_config = d.m_argv.myargv[p + 1].clone();
        println!("\tdefault file: {}", d.m_config.default_main_config);
    } else {
        d.m_config.default_main_config = format!("{}{}", d.m_config.configdir, "default.cfg");
    }
    println!("saving config in {}", d.m_config.default_main_config);
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-extraconfig", 1);
    if p > 0 {
        d.m_config.default_extra_config = d.m_argv.myargv[p + 1].clone();
        println!(
            "        extra configuration file: {}",
            d.m_config.default_extra_config
        );
    } else {
        d.m_config.default_extra_config =
            format!("{}{}", d.m_config.configdir, "doomgenericdoom.cfg");
    }
}

/// M_SaveDefaults. The original body writes nothing.
pub fn save_defaults(_d: &mut Doom) {}

/// M_SetConfigDir: the configuration directory is ".".
pub fn set_config_dir(d: &mut Doom) {
    d.m_config.configdir = ".".to_string();
    println!("Using {} for configuration and saves", d.m_config.configdir);
    let _ = std::fs::create_dir(&d.m_config.configdir);
}

/// M_GetSaveGameDir: creates and reports the savegame directory.
pub fn get_save_game_dir(d: &mut Doom) -> String {
    let dir = format!("{}/{}", d.m_config.configdir, ".savegame/");
    let _ = std::fs::create_dir(&dir);
    println!("Using {dir} for savegames");
    dir
}

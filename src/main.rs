//! The doomgeneric entry: argument capture, platform start, engine start, and the endless tick loop.
// The modules keep the full constant inventory of the original, also where no code reads it.
#![allow(dead_code)]
// The code keeps the control flow of the original functions, also where clippy prefers another form.
#![allow(
    clippy::needless_range_loop,
    clippy::if_same_then_else,
    clippy::manual_clamp
)]
#![allow(
    clippy::unnecessary_unwrap,
    clippy::while_let_loop,
    clippy::nonminimal_bool
)]

/// Every print site writes to the C library stdout stream (ADR 0008).
macro_rules! print {
    ($($t:tt)*) => { $crate::i_video::print(format_args!($($t)*)) };
}
macro_rules! println {
    ($($t:tt)*) => { $crate::i_video::print(format_args!("{}\n", format_args!($($t)*))) };
}

mod am_map;
mod d_items;
mod d_iwad;
mod d_loop;
mod d_main;
mod f_finale;
mod f_wipe;
mod g_game;
mod hu_lib;
mod hu_stuff;
mod i_system;
mod i_video;
mod info;
mod m_argv;
mod m_cheat;
mod m_config;
mod m_fixed;
mod m_menu;
mod m_misc;
mod m_random;
mod p_ceilng;
mod p_doors;
mod p_enemy;
mod p_floor;
mod p_inter;
mod p_lights;
mod p_map;
mod p_maputl;
mod p_mobj;
mod p_plats;
mod p_pspr;
mod p_saveg;
mod p_setup;
mod p_sight;
mod p_spec;
mod p_switch;
mod p_telept;
mod p_tick;
mod p_user;
mod r_bsp;
mod r_data;
mod r_draw;
mod r_main;
mod r_plane;
mod r_segs;
mod r_sky;
mod r_things;
mod s_sound;
mod sha1;
mod st_lib;
mod st_stuff;
mod statdump;
mod tables;
mod v_video;
mod w_wad;
mod wi_stuff;
mod z_zone;

/// All engine state. Each field holds the globals of the module with the same name.
#[derive(Default)]
pub struct Doom {
    pub am_map: am_map::State,
    pub d_iwad: d_iwad::State,
    pub d_main: d_main::State,
    pub d_loop: d_loop::State,
    pub f_finale: f_finale::State,
    pub f_wipe: f_wipe::State,
    pub g_game: g_game::State,
    pub hu_lib: hu_lib::State,
    pub hu_stuff: hu_stuff::State,
    pub i_system: i_system::State,
    pub i_video: i_video::State,
    pub m_argv: m_argv::State,
    pub m_cheat: m_cheat::State,
    pub m_config: m_config::State,
    pub m_menu: m_menu::State,
    pub m_misc: m_misc::State,
    pub m_random: m_random::State,
    pub p_ceilng: p_ceilng::State,
    pub p_doors: p_doors::State,
    pub p_enemy: p_enemy::State,
    pub p_floor: p_floor::State,
    pub p_inter: p_inter::State,
    pub p_lights: p_lights::State,
    pub p_map: p_map::State,
    pub p_maputl: p_maputl::State,
    pub p_mobj: p_mobj::State,
    pub p_plats: p_plats::State,
    pub p_pspr: p_pspr::State,
    pub p_saveg: p_saveg::State,
    pub p_setup: p_setup::State,
    pub p_sight: p_sight::State,
    pub p_spec: p_spec::State,
    pub p_switch: p_switch::State,
    pub p_telept: p_telept::State,
    pub p_tick: p_tick::State,
    pub p_user: p_user::State,
    pub r_bsp: r_bsp::State,
    pub r_data: r_data::State,
    pub r_draw: r_draw::State,
    pub r_main: r_main::State,
    pub r_plane: r_plane::State,
    pub r_segs: r_segs::State,
    pub r_sky: r_sky::State,
    pub r_things: r_things::State,
    pub s_sound: s_sound::State,
    pub sha1: sha1::State,
    pub st_lib: st_lib::State,
    pub st_stuff: st_stuff::State,
    pub statdump: statdump::State,
    pub v_video: v_video::State,
    pub w_wad: w_wad::State,
    pub wi_stuff: wi_stuff::State,
    pub z_zone: z_zone::State,
}

/// doomgeneric_Create: keeps the arguments, opens the platform window, and runs the engine startup.
fn create(d: &mut Doom, args: Vec<String>) {
    d.m_argv.myargv = args;
    i_video::dg_init(&mut d.i_video);
    d_main::doom_main(d);
}

/// doomgeneric_Tick: runs one main-loop iteration.
fn tick(d: &mut Doom) {
    d_main::doom_loop_tick(d);
}

fn main() {
    // A failed check in the original is a memory fault, so a Rust panic ends with SIGSEGV.
    // RUST_BACKTRACE keeps the Rust report for debugging.
    if std::env::var_os("RUST_BACKTRACE").is_none() {
        std::panic::set_hook(Box::new(|_| i_video::crash(i_video::SIGSEGV)));
    }
    i_video::default_sigpipe();
    let mut d = Box::<Doom>::default();
    create(&mut d, std::env::args().collect());
    loop {
        tick(&mut d);
    }
}

//! System services: fatal errors, exit functions, the tic clock, and sleep.

use crate::Doom;
use std::io::Write;
use std::sync::atomic::{AtomicU32, Ordering};

/// The functions that I_AtExit registers in the original.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExitFunc {
    Endoom,
    SaveDefaults,
    CheckDemoStatus,
    QuitNetGame,
    ShutdownSound,
    StatDump,
}

#[derive(Default)]
pub struct State {
    /// The exit function list. The last registered function runs first.
    pub exit_funcs: Vec<(ExitFunc, bool)>,
    already_quitting: bool,
    /// The message of the first I_Error call.
    message: String,
    /// The DOS memory dump of I_GetMemoryValue after its first call.
    mem_dump: Option<[u8; 10]>,
}

/// I_AtExit.
pub fn at_exit(d: &mut Doom, func: ExitFunc, run_on_error: bool) {
    d.i_system.exit_funcs.push((func, run_on_error));
}

fn run_exit_func(d: &mut Doom, f: ExitFunc) {
    match f {
        ExitFunc::Endoom => crate::d_main::endoom(d),
        ExitFunc::CheckDemoStatus => {
            crate::g_game::check_demo_status(d);
        }
        ExitFunc::StatDump => crate::statdump::dump(d),
        ExitFunc::SaveDefaults => crate::m_config::save_defaults(d),
        ExitFunc::QuitNetGame | ExitFunc::ShutdownSound => {}
    }
}

/// Runs the exit functions from the last registered one. With `on_error`, only the error-safe ones run.
pub fn run_exit_funcs(d: &mut Doom, on_error: bool) {
    let funcs = d.i_system.exit_funcs.clone();
    for &(f, safe) in funcs.iter().rev() {
        if safe || !on_error {
            run_exit_func(d, f);
        }
    }
}

fn system(cmd: &str) -> bool {
    std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// The recursive I_Error calls before the stack overflow (G_CheckDemoStatus cycle, fz-playdemo-warp).
const RECURSION_LIMIT: usize = 10671;

/// I_Error: reports the message, runs the error-safe exit functions, and exits with status 255.
pub fn error(d: &mut Doom, message: &str) -> ! {
    const WARNING: &str = "Warning: recursive call to I_Error detected.\n";
    if d.i_system.already_quitting {
        if message == d.i_system.message {
            // The same error repeats through the exit functions until the original
            // overflows its stack in the print after warning number RECURSION_LIMIT.
            let cycle = format!("{message}\n\n{WARNING}");
            eprint!("{WARNING}{}", cycle.repeat(RECURSION_LIMIT - 1));
            crate::i_video::crash(crate::i_video::SIGSEGV);
        }
        eprint!("{WARNING}");
    } else {
        d.i_system.already_quitting = true;
        d.i_system.message = message.to_string();
    }
    eprint!("{message}\n\n");
    let _ = std::io::stderr().flush();
    run_exit_funcs(d, true);
    if !crate::m_argv::parm_exists(&d.m_argv, "-nogui")
        && system("/usr/bin/zenity --help >/dev/null 2>&1")
    {
        let mut quoted = String::from("\"");
        for c in message.chars() {
            if "\"$`\\!".contains(c) {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        system(&format!("/usr/bin/zenity --error --text={quoted}"));
    }
    std::process::exit(255)
}

/// I_Quit: runs every exit function and returns. D_Endoom ends the process with status 0, except under -testcontrols.
pub fn quit(d: &mut Doom) {
    run_exit_funcs(d, false);
}

/// The dos622, dos71, and dosbox memory dumps.
const DOS_MEM: [[u8; 10]; 3] = [
    [0x57, 0x92, 0x19, 0, 0xf4, 0x06, 0x70, 0, 0x16, 0],
    [0x9e, 0x0f, 0xc9, 0, 0x65, 0x04, 0x70, 0, 0x16, 0],
    [0, 0, 0, 0xf1, 0, 0, 0, 0, 0x07, 0],
];

/// I_GetMemoryValue for a 4-byte value. The original has no "else" after the dos622 test,
/// so "-setmem dos622" selects the custom dump. An unparsed value keeps the last value (0 at first).
pub fn get_memory_value(d: &mut Doom, offset: usize) -> i32 {
    if d.i_system.mem_dump.is_none() {
        let a = &d.m_argv.myargv;
        let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-setmem", 1);
        let arg = a
            .get(p + 1)
            .filter(|_| p > 0)
            .map(|s| s.to_ascii_lowercase());
        let dump = match arg.as_deref() {
            None => DOS_MEM[0],
            Some("dos71") => DOS_MEM[1],
            Some("dosbox") => DOS_MEM[2],
            _ => {
                let (mut dump, mut val) = ([0u8; 10], 0);
                for i in 0..5 {
                    match a.get(p + 1 + i) {
                        Some(s) if !s.starts_with('-') => {
                            val = crate::m_misc::str_to_int(s).unwrap_or(val);
                            dump[2 * i] = val as u8;
                        }
                        _ => break,
                    }
                }
                dump
            }
        };
        d.i_system.mem_dump = Some(dump);
    }
    let m = d.i_system.mem_dump.unwrap_or_default();
    i32::from_le_bytes(m[offset..offset + 4].try_into().unwrap())
}

/// I_Sleep.
pub fn sleep(ms: u32) {
    crate::i_video::dg_sleep_ms(ms);
}

static BASETIME: AtomicU32 = AtomicU32::new(0);

/// I_GetTime: tics since the first call, at 35 tics per second.
pub fn get_time() -> i32 {
    let ticks = crate::i_video::dg_get_ticks_ms();
    let mut base = BASETIME.load(Ordering::Relaxed);
    if base == 0 {
        BASETIME.store(ticks, Ordering::Relaxed);
        base = ticks;
    }
    (ticks.wrapping_sub(base).wrapping_mul(35) / 1000) as i32
}

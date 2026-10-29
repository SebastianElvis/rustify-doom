//! Command-line argument access.

#[derive(Default)]
pub struct State {
    pub myargv: Vec<String>,
}

/// Returns the position of `check` in the argument list, or 0 when absent. The match ignores case.
pub fn check_parm(s: &State, check: &str) -> usize {
    check_parm_with_args(s, check, 0)
}

/// Returns the position of `check` only when at least `num_args` arguments follow it.
pub fn check_parm_with_args(s: &State, check: &str, num_args: usize) -> usize {
    let argc = s.myargv.len();
    for i in 1..argc.saturating_sub(num_args) {
        if s.myargv[i].eq_ignore_ascii_case(check) {
            return i;
        }
    }
    0
}

pub fn parm_exists(s: &State, check: &str) -> bool {
    check_parm(s, check) != 0
}

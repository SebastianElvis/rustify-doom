//! Cheat key sequence matching.

#[derive(Default)]
pub struct State {}

/// cheatseq_t.
#[derive(Clone, Default)]
pub struct CheatSeq {
    pub sequence: &'static [u8],
    pub parameter_chars: usize,
    pub chars_read: usize,
    pub param_chars_read: usize,
    pub parameter_buf: [u8; 5],
}

impl CheatSeq {
    pub const fn new(sequence: &'static str, parameter_chars: usize) -> Self {
        CheatSeq {
            sequence: sequence.as_bytes(),
            parameter_chars,
            chars_read: 0,
            param_chars_read: 0,
            parameter_buf: [0; 5],
        }
    }
}

/// cht_CheckCheat: feeds one key and returns true when the sequence and its parameters are complete.
pub fn check_cheat(cht: &mut CheatSeq, key: u8) -> bool {
    if cht.chars_read < cht.sequence.len() {
        if key == cht.sequence[cht.chars_read] {
            cht.chars_read += 1;
        } else {
            cht.chars_read = 0;
        }
        cht.param_chars_read = 0;
    } else if cht.param_chars_read < cht.parameter_chars {
        cht.parameter_buf[cht.param_chars_read] = key;
        cht.param_chars_read += 1;
    }
    if cht.chars_read >= cht.sequence.len() && cht.param_chars_read >= cht.parameter_chars {
        cht.chars_read = 0;
        cht.param_chars_read = 0;
        return true;
    }
    false
}

/// cht_GetParam.
pub fn get_param(cht: &CheatSeq) -> Vec<u8> {
    cht.parameter_buf[..cht.parameter_chars].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_sequence_with_parameters() {
        let mut c = CheatSeq::new("idclev", 2);
        let mut hit = false;
        for &k in b"xidclev13" {
            hit = check_cheat(&mut c, k);
        }
        assert!(hit);
        assert_eq!(get_param(&c), b"13");
    }
}

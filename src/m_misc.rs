//! Miscellaneous helpers: integer parse, file write, and file existence.

#[derive(Default)]
pub struct State {}

/// Parses an optional sign and digits of `radix` at the start of `s`, as sscanf does.
fn scan(s: &str, radix: u32) -> Option<i32> {
    let (neg, digits) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let end = digits
        .find(|c: char| !c.is_digit(radix))
        .unwrap_or(digits.len());
    if end == 0 {
        return None;
    }
    let v = digits[..end].chars().fold(0u32, |a, c| {
        a.wrapping_mul(radix)
            .wrapping_add(c.to_digit(radix).unwrap_or(0))
    });
    Some(if neg {
        v.wrapping_neg() as i32
    } else {
        v as i32
    })
}

/// M_StrToInt: tries " 0x%x", " 0X%x", " 0%o", and " %d" in this order.
pub fn str_to_int(s: &str) -> Option<i32> {
    let t = s.trim_start();
    t.strip_prefix("0x")
        .and_then(|r| scan(r, 16))
        .or_else(|| t.strip_prefix("0X").and_then(|r| scan(r, 16)))
        .or_else(|| t.strip_prefix('0').and_then(|r| scan(r, 8)))
        .or_else(|| scan(t, 10))
}

/// M_WriteFile. Returns false when the file cannot be written.
pub fn write_file(name: &str, data: &[u8]) -> bool {
    std::fs::write(name, data).is_ok()
}

/// M_FileExists: true when fopen succeeds or the path is a directory.
pub fn file_exists(name: &str) -> bool {
    std::fs::File::open(name).is_ok() || std::path::Path::new(name).is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_to_int_follows_sscanf_order() {
        assert_eq!(str_to_int("0x1234"), Some(0x1234));
        assert_eq!(str_to_int(" 017"), Some(15));
        assert_eq!(str_to_int("0"), Some(0));
        assert_eq!(str_to_int("-5"), Some(-5));
        assert_eq!(str_to_int("abc"), None);
    }
}

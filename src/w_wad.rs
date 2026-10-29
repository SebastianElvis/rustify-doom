//! WAD file loading and the lump directory.

#[allow(unused_imports)]
use crate::Doom;
use std::fs;

pub struct LumpInfo {
    pub name: [u8; 8],
    pub file: usize,
    pub position: usize,
    pub size: usize,
}

#[derive(Default)]
pub struct State {
    pub files: Vec<Vec<u8>>,
    pub lumps: Vec<LumpInfo>,
    pub filenames: Vec<String>,
}

fn upper8(name: &[u8]) -> [u8; 8] {
    let mut out = [0u8; 8];
    for (i, &c) in name.iter().take(8).enumerate() {
        if c == 0 {
            break;
        }
        out[i] = c.to_ascii_uppercase();
    }
    out
}

fn read_i32(data: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// W_AddFile: adds a WAD or a single lump file. Returns false when the file cannot be opened.
pub fn add_file(d: &mut Doom, filename: &str) -> bool {
    let data = match fs::read(filename) {
        Ok(v) => v,
        // fopen opens a directory on macOS, and fread reads nothing.
        Err(_) if std::path::Path::new(filename).is_dir() => Vec::new(),
        Err(_) => {
            println!(" couldn't open {filename}");
            return false;
        }
    };
    // W_OpenFile keeps a 32-byte file record in the zone.
    d.z_zone.malloc(32, crate::z_zone::PU_STATIC, 0);
    let is_wad = filename.len() >= 3
        && filename.is_char_boundary(filename.len() - 3)
        && filename[filename.len() - 3..].eq_ignore_ascii_case("wad");
    // A short fread leaves the rest of the header and the directory unread; this model uses zeros.
    let padded = |at: usize, len: usize| {
        let mut v = vec![0u8; len];
        let m = data.len().saturating_sub(at).min(len);
        v[..m].copy_from_slice(&data[at.min(data.len())..at.min(data.len()) + m]);
        v
    };
    let dir = if !is_wad {
        // M_ExtractFileBase: the base name up to the first dot, at most 8 characters.
        let base = filename.rsplit('/').next().unwrap_or(filename);
        let stem = base.split('.').next().unwrap_or(base);
        if stem.len() > 8 {
            println!(
                "Warning: Truncated '{}' lump name to '{}'.",
                base,
                stem[..8].to_ascii_uppercase()
            );
        }
        let mut v = [0u8; 16].to_vec();
        v[4..8].copy_from_slice(&(data.len() as i32).to_le_bytes());
        v[8..].copy_from_slice(&upper8(stem.as_bytes()));
        v
    } else {
        let hdr = padded(0, 12);
        if &hdr[0..4] != b"IWAD" && &hdr[0..4] != b"PWAD" {
            crate::i_system::error(
                d,
                &format!("Wad file {filename} doesn't have IWAD or PWAD id\n"),
            );
        }
        padded(read_i32(&hdr, 8) as usize, 16 * read_i32(&hdr, 4) as usize)
    };
    let s = &mut d.w_wad;
    let file = s.files.len();
    for e in dir.chunks(16) {
        s.lumps.push(LumpInfo {
            name: upper8(&e[8..16]),
            file,
            position: if is_wad { read_i32(e, 0) as usize } else { 0 },
            size: read_i32(e, 4) as usize,
        });
    }
    s.files.push(data);
    s.filenames.push(filename.to_string());
    let p = d.z_zone.malloc_copy(&dir, crate::z_zone::PU_STATIC, 0);
    d.z_zone.free(p);
    true
}

/// Returns the last lump with this name, or -1.
pub fn check_num_for_name(s: &State, name: &str) -> i32 {
    let key = upper8(name.as_bytes());
    for i in (0..s.lumps.len()).rev() {
        if s.lumps[i].name == key {
            return i as i32;
        }
    }
    -1
}

pub fn lump_name(s: &State, lump: usize) -> String {
    let n = &s.lumps[lump].name;
    let end = n.iter().position(|&c| c == 0).unwrap_or(8);
    String::from_utf8_lossy(&n[..end]).into_owned()
}

pub fn lump_length(s: &State, lump: usize) -> usize {
    s.lumps[lump].size
}

pub fn lump(s: &State, lump: usize) -> &[u8] {
    let l = &s.lumps[lump];
    let data = &s.files[l.file];
    let end = (l.position + l.size).min(data.len());
    &data[l.position.min(end)..end]
}

/// W_GetNumForName.
pub fn get_num_for_name(d: &mut Doom, name: &str) -> usize {
    let i = check_num_for_name(&d.w_wad, name);
    if i < 0 {
        crate::i_system::error(d, &format!("W_GetNumForName: {name} not found!"));
    }
    i as usize
}

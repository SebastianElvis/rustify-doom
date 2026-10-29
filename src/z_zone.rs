//! Zone memory. Rust containers own the game data (ADR 0005).
//! A byte model of the zone keeps the block layout and the stale bytes that the original reads (ADR 0006).

use crate::Doom;

pub const ZONE_SIZE: usize = 0x600000;
pub const PU_STATIC: i32 = 1;
pub const PU_LEVEL: i32 = 5;
pub const PU_LEVSPEC: i32 = 6;
pub const PU_CACHE: i32 = 8;
const PU_FREE: i32 = 4;
const ZONEID: i32 = 0x1d4a11;
/// The model addresses of the zone and of the lump directory. Only the low bytes are stable in the original.
const ZBASE: u64 = 0x5_02c0_0000;
const LBASE: u64 = 0x6_0010_0000;

#[derive(Default)]
pub struct State {
    pub size: usize,
    zone: Vec<u8>,
    /// The zone bytes. Offsets replace the original pointers. The model stops after a failed allocation.
    pub mem: Vec<u8>,
    /// lumpinfo[].cache: the data offset of each cached lump, or 0.
    pub lumpcache: Vec<usize>,
    /// The music number and lump that S_ChangeMusic keeps cached.
    pub music: usize,
    pub music_lump: usize,
    /// True after the first ST_Start.
    pub st_started: bool,
    /// R_PrecacheLevel ran in the last level setup.
    pub precache: bool,
    /// The zone offset of each thinker block by thinker id.
    pub thinkers: Vec<usize>,
    /// The demo lump that G_DoPlayDemo keeps with PU_STATIC.
    pub demo_lump: usize,
    /// The zone offset of the subsectors array of the level.
    pub subsectors: usize,
    /// The block size of a failed allocation, or 0.
    failed: i32,
}

impl State {
    fn r32(&self, o: usize) -> i32 {
        i32::from_le_bytes(self.mem[o..o + 4].try_into().unwrap())
    }
    fn w32(&mut self, o: usize, v: i32) {
        self.mem[o..o + 4].copy_from_slice(&v.to_le_bytes());
    }
    fn wu(&mut self, o: usize, v: u64) {
        self.mem[o..o + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn rp(&self, o: usize) -> usize {
        (u64::from_le_bytes(self.mem[o..o + 8].try_into().unwrap()) - ZBASE) as usize
    }
    fn wp(&mut self, o: usize, p: usize) {
        self.wu(o, ZBASE + p as u64);
    }

    /// Z_Init on fresh zero memory.
    fn z_init(&mut self) {
        self.mem = vec![0; self.size];
        let n = self.size as i32;
        self.w32(0, n);
        self.w32(0x38, n - 0x38);
        self.wp(0x20, 0x38);
        self.wp(0x28, 0x38);
        self.wu(0x10, ZBASE);
        self.w32(0x18, PU_STATIC);
        self.wp(0x30, 0x38);
        self.w32(0x48, PU_FREE);
        self.wp(0x50, 8);
        self.wp(0x58, 8);
    }

    /// Clears the owner pointer (*user = NULL).
    fn clear_user(&mut self, user: u64) {
        if (ZBASE..ZBASE + self.size as u64).contains(&user) {
            self.wu((user - ZBASE) as usize, 0);
        } else if user >= LBASE + 0x18 && (user - LBASE - 0x18).is_multiple_of(40) {
            if let Some(c) = self
                .lumpcache
                .get_mut(((user - LBASE - 0x18) / 40) as usize)
            {
                *c = 0;
            }
        }
    }

    /// Z_Free on a block header. Returns the merged block.
    fn free_block(&mut self, mut b: usize) -> usize {
        let user = u64::from_le_bytes(self.mem[b + 8..b + 16].try_into().unwrap());
        if self.r32(b + 0x10) != PU_FREE && user != 0 {
            self.clear_user(user);
        }
        self.wu(b + 8, 0);
        self.wu(b + 0x10, PU_FREE as u64);
        for prev_side in [true, false] {
            let (keep, gone) = if prev_side {
                (self.rp(b + 0x20), b)
            } else {
                (b, self.rp(b + 0x18))
            };
            if self.r32(if prev_side { keep } else { gone } + 0x10) == PU_FREE {
                let n = self.r32(keep) + self.r32(gone);
                self.w32(keep, n);
                let next = self.rp(gone + 0x18);
                self.wp(keep + 0x18, next);
                self.wp(next + 0x20, keep);
                if self.rp(0x30) == gone {
                    self.wp(0x30, keep);
                }
                b = keep;
            }
        }
        b
    }

    /// Z_Malloc. Returns the data offset, or 0 when the model has no zone.
    pub fn malloc(&mut self, size: i32, tag: i32, user: u64) -> usize {
        if self.mem.is_empty() {
            return 0;
        }
        let size = ((size + 7) & !7) + 0x28;
        let mut base = self.rp(0x30);
        if self.r32(self.rp(base + 0x20) + 0x10) == PU_FREE {
            base = self.rp(base + 0x20);
        }
        let mut rover = base;
        let start = self.rp(base + 0x20);
        loop {
            if rover == start {
                // Z_Malloc: failed on allocation. The model stops, and check() reports the error.
                self.mem = Vec::new();
                self.failed = size;
                return 0;
            }
            let t = self.r32(rover + 0x10);
            if t == PU_FREE {
                rover = self.rp(rover + 0x18);
            } else if t < 7 {
                rover = self.rp(rover + 0x18);
                base = rover;
            } else {
                base = self.rp(base + 0x20);
                self.free_block(rover);
                base = self.rp(base + 0x18);
                rover = self.rp(base + 0x18);
            }
            if self.r32(base + 0x10) == PU_FREE && self.r32(base) >= size {
                break;
            }
        }
        let extra = self.r32(base) - size;
        if extra > 64 {
            let nb = base + size as usize;
            let next = self.rp(base + 0x18);
            self.w32(nb, extra);
            self.w32(nb + 0x10, PU_FREE);
            self.wu(nb + 8, 0);
            self.wp(nb + 0x18, next);
            self.wp(nb + 0x20, base);
            self.wp(next + 0x20, nb);
            self.wp(base + 0x18, nb);
            self.w32(base, size);
        }
        self.wu(base + 8, user);
        self.w32(base + 0x10, tag);
        if user != 0 {
            self.wp_user(user, base + 0x28);
        }
        let next = self.rp(base + 0x18);
        self.wp(0x30, next);
        self.w32(base + 0x14, ZONEID);
        base + 0x28
    }

    /// Stores the new block address in its owner (*user = ptr).
    fn wp_user(&mut self, user: u64, ptr: usize) {
        if (ZBASE..ZBASE + self.size as u64).contains(&user) {
            self.wp((user - ZBASE) as usize, ptr);
        } else if user >= LBASE + 0x18 && (user - LBASE - 0x18).is_multiple_of(40) {
            if let Some(c) = self
                .lumpcache
                .get_mut(((user - LBASE - 0x18) / 40) as usize)
            {
                *c = ptr;
            }
        }
    }

    /// Z_Free.
    pub fn free(&mut self, ptr: usize) {
        if !self.mem.is_empty() && ptr != 0 {
            self.free_block(ptr - 0x28);
        }
    }

    /// Z_ChangeTag.
    pub fn change_tag(&mut self, ptr: usize, tag: i32) {
        if !self.mem.is_empty() && ptr != 0 {
            self.w32(ptr - 0x28 + 0x10, tag);
        }
    }

    /// Z_FreeTags.
    pub fn free_tags(&mut self, lo: i32, hi: i32) {
        if self.mem.is_empty() {
            return;
        }
        let mut b = self.rp(0x20);
        while b != 8 {
            let next = self.rp(b + 0x18);
            let t = self.r32(b + 0x10);
            if t >= lo && t <= hi {
                self.free_block(b);
            }
            b = next;
        }
    }

    /// W_CacheLumpNum: returns the data offset of the lump in the zone.
    pub fn cache_lump(&mut self, w: &crate::w_wad::State, lump: usize, tag: i32) -> usize {
        if self.lumpcache.len() < w.lumps.len() {
            self.lumpcache.resize(w.lumps.len(), 0);
        }
        let c = self.lumpcache[lump];
        if c != 0 {
            self.change_tag(c, tag);
            return c;
        }
        let data = crate::w_wad::lump(w, lump);
        let p = self.malloc(data.len() as i32, tag, LBASE + 40 * lump as u64 + 0x18);
        if p != 0 {
            self.mem[p..p + data.len()].copy_from_slice(data);
        }
        p
    }

    /// W_CacheLumpName with PU_CACHE for a drawn patch. A cached lump keeps its tag,
    /// because the original draws its static patches through stored pointers.
    pub fn touch_lump(&mut self, w: &crate::w_wad::State, lump: usize) {
        if self.lumpcache.get(lump).copied().unwrap_or(0) == 0 {
            self.cache_lump(w, lump, PU_CACHE);
        }
    }

    /// W_ReleaseLumpNum.
    pub fn release_lump(&mut self, lump: usize) {
        let c = self.lumpcache.get(lump).copied().unwrap_or(0);
        self.change_tag(c, PU_CACHE);
    }

    /// Z_Malloc followed by a copy of the given bytes.
    pub fn malloc_copy(&mut self, data: &[u8], tag: i32, user: u64) -> usize {
        let p = self.malloc(data.len() as i32, tag, user);
        if p != 0 {
            self.mem[p..p + data.len()].copy_from_slice(data);
        }
        p
    }

    /// Reads a zone pointer at an offset. Returns its zone offset, or 0 for NULL.
    pub fn ptr_at(&self, o: usize) -> usize {
        let v = u64::from_le_bytes(self.mem[o..o + 8].try_into().unwrap());
        if v == 0 {
            0
        } else {
            (v - ZBASE) as usize
        }
    }

    /// The model address of a zone offset, for owner pointers that live in zone memory.
    pub fn addr(&self, o: usize) -> u64 {
        ZBASE + o as u64
    }
}

/// Z_Init with I_ZoneBase: reserves the zone and reports its address and size.
pub fn init(d: &mut Doom) {
    let p = crate::m_argv::check_parm_with_args(&d.m_argv, "-mb", 1);
    let size = if p > 0 {
        (atoi(&d.m_argv.myargv[p + 1]) as usize).wrapping_mul(1024 * 1024)
    } else {
        ZONE_SIZE
    };
    let z = &mut d.z_zone;
    z.size = size;
    z.zone = Vec::with_capacity(16);
    if (0x60..=0x4000_0000).contains(&size) {
        z.z_init();
    }
    println!(
        "zone memory: {:p}, {:x} allocated for zone",
        z.zone.as_ptr(),
        size as u32
    );
}

/// The I_Error of a failed Z_Malloc. The model replays allocations late, so callers check after each replay.
pub fn check(d: &mut Doom) {
    let n = std::mem::take(&mut d.z_zone.failed);
    if n != 0 {
        crate::i_system::error(d, &format!("Z_Malloc: failed on allocation of {n} bytes"));
    }
}

/// atoi: parses an optional sign and leading decimal digits.
pub fn atoi(s: &str) -> i32 {
    let t = s.trim_start();
    let (neg, r) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let v = r.bytes().take_while(u8::is_ascii_digit).fold(0i32, |a, c| {
        a.wrapping_mul(10).wrapping_add((c - b'0') as i32)
    });
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

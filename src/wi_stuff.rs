//! The intermission screen: level statistics, deathmatch and netgame tables, and the episode map.

use crate::d_main::{COMMERCIAL, RETAIL};
use crate::g_game::{WbPlayer, WbStart, BT_ATTACK, BT_USE, MAXPLAYERS};
use crate::hu_lib::patch_header;
use crate::m_random::m_random;
use crate::s_sound::{self, SFX_BAREXP, SFX_PISTOL, SFX_PLDETH, SFX_SGCOCK, SFX_SLOP};
use crate::v_video::draw_patch;
use crate::Doom;

const MUS_INTER: i32 = 28;
const MUS_DM2INT: i32 = 67;

const NUMMAPS: usize = 9;
const NUMCMAPS: usize = 32;
const TICRATE: i32 = 35;
const ORIGWIDTH: i32 = 320;
const SP_STATSX: i32 = 50;
const SP_STATSY: i32 = 50;
const SP_TIMEX: i32 = 16;
const SP_TIMEY: i32 = 200 - 32;
const SHOWNEXTLOCDELAY: i32 = 4;
const WI_TITLEY: i32 = 2;
const WI_SPACINGY: i32 = 33;
const NG_STATSY: i32 = 50;
const NG_SPACINGX: i32 = 64;
const DM_MATRIXX: i32 = 42;
const DM_MATRIXY: i32 = 68;
const DM_SPACINGX: i32 = 40;
const DM_TOTALSX: i32 = 269;
const DM_KILLERSX: i32 = 10;
const DM_KILLERSY: i32 = 100;
const DM_VICTIMSX: i32 = 5;
const DM_VICTIMSY: i32 = 50;

const NO_STATE: i32 = -1;
const STAT_COUNT: i32 = 0;
const SHOW_NEXT_LOC: i32 = 1;

const ANIM_ALWAYS: i32 = 0;
const ANIM_RANDOM: i32 = 1;
const ANIM_LEVEL: i32 = 2;

/// The map node positions of the "you are here" and splat marks for episodes 1 to 3.
const LNODES: [[(i32, i32); NUMMAPS]; 3] = [
    [
        (185, 164),
        (148, 143),
        (69, 122),
        (209, 102),
        (116, 89),
        (166, 55),
        (71, 56),
        (135, 29),
        (71, 24),
    ],
    [
        (254, 25),
        (97, 50),
        (188, 64),
        (128, 78),
        (214, 92),
        (133, 130),
        (208, 136),
        (148, 140),
        (235, 158),
    ],
    [
        (156, 168),
        (48, 154),
        (174, 95),
        (265, 75),
        (130, 48),
        (279, 23),
        (198, 48),
        (140, 25),
        (281, 136),
    ],
];

/// anim_t: type, period, nanims, x, y, data1.
type AnimDef = (i32, i32, i32, i32, i32, i32);
const EPSD0: [AnimDef; 10] = [
    (0, 11, 3, 224, 104, 0),
    (0, 11, 3, 184, 160, 0),
    (0, 11, 3, 112, 136, 0),
    (0, 11, 3, 72, 112, 0),
    (0, 11, 3, 88, 96, 0),
    (0, 11, 3, 64, 48, 0),
    (0, 11, 3, 192, 40, 0),
    (0, 11, 3, 136, 16, 0),
    (0, 11, 3, 80, 16, 0),
    (0, 11, 3, 64, 24, 0),
];
const EPSD1: [AnimDef; 9] = [
    (2, 11, 1, 128, 136, 1),
    (2, 11, 1, 128, 136, 2),
    (2, 11, 1, 128, 136, 3),
    (2, 11, 1, 128, 136, 4),
    (2, 11, 1, 128, 136, 5),
    (2, 11, 1, 128, 136, 6),
    (2, 11, 1, 128, 136, 7),
    (2, 11, 3, 192, 144, 8),
    (2, 11, 1, 128, 136, 8),
];
const EPSD2: [AnimDef; 6] = [
    (0, 11, 3, 104, 168, 0),
    (0, 11, 3, 40, 136, 0),
    (0, 11, 3, 160, 96, 0),
    (0, 11, 3, 104, 80, 0),
    (0, 11, 3, 120, 32, 0),
    (0, 8, 3, 40, 0, 0),
];

fn anim_defs(epsd: i32) -> &'static [AnimDef] {
    match epsd {
        0 => &EPSD0,
        1 => &EPSD1,
        _ => &EPSD2,
    }
}

/// The mutable part of anim_t.
#[derive(Clone, Copy, Default)]
struct Anim {
    p: [usize; 3],
    nexttic: i32,
    ctr: i32,
}

/// The intermission globals.
#[derive(Default)]
pub struct State {
    wbs: WbStart,
    plrs: [WbPlayer; MAXPLAYERS],
    me: usize,
    state: i32,
    acceleratestage: i32,
    bcnt: i32,
    cnt: i32,
    cnt_kills: [i32; MAXPLAYERS],
    cnt_items: [i32; MAXPLAYERS],
    cnt_secret: [i32; MAXPLAYERS],
    cnt_frags: [i32; MAXPLAYERS],
    cnt_time: i32,
    cnt_par: i32,
    cnt_pause: i32,
    sp_state: i32,
    ng_state: i32,
    dm_state: i32,
    dm_frags: [[i32; MAXPLAYERS]; MAXPLAYERS],
    dm_totals: [i32; MAXPLAYERS],
    dofrags: i32,
    snl_pointeron: bool,
    anims: Vec<Anim>,
    // Patches as lump numbers.
    background: usize,
    lnames: Vec<usize>,
    yah: [Option<usize>; 2],
    splat: [Option<usize>; 2],
    wiminus: usize,
    num: [usize; 10],
    percent: usize,
    finished: usize,
    entering: usize,
    kills: usize,
    secret: usize,
    sp_secret: usize,
    items: usize,
    frags: usize,
    colon: usize,
    timepatch: usize,
    sucks: usize,
    par: usize,
    killers: usize,
    victims: usize,
    total: usize,
    star: usize,
    bstar: usize,
    p: [usize; MAXPLAYERS],
    bp: [usize; MAXPLAYERS],
}

fn width(d: &Doom, lump: usize) -> i32 {
    patch_header(d, lump).0
}

fn height(d: &Doom, lump: usize) -> i32 {
    patch_header(d, lump).1
}

fn lump(d: &mut Doom, name: &str) -> usize {
    crate::w_wad::get_num_for_name(d, name)
}

fn is_commercial(d: &Doom) -> bool {
    d.d_main.gamemode == COMMERCIAL
}

/// WI_slamBackground.
fn slam_background(d: &mut Doom) {
    let bg = d.wi_stuff.background;
    draw_patch(d, 0, 0, bg);
}

/// WI_drawLF: draws the finished level name and "Finished".
fn draw_lf(d: &mut Doom) {
    let mut y = WI_TITLEY;
    let ln = d.wi_stuff.lnames[d.wi_stuff.wbs.last as usize];
    draw_patch(d, (ORIGWIDTH - width(d, ln)) / 2, y, ln);
    y += (5 * height(d, ln)) / 4;
    let f = d.wi_stuff.finished;
    draw_patch(d, (ORIGWIDTH - width(d, f)) / 2, y, f);
}

/// WI_drawEL: draws "Entering" and the next level name.
fn draw_el(d: &mut Doom) {
    let mut y = WI_TITLEY;
    let e = d.wi_stuff.entering;
    draw_patch(d, (ORIGWIDTH - width(d, e)) / 2, y, e);
    let ln = d.wi_stuff.lnames[d.wi_stuff.wbs.next as usize];
    y += (5 * height(d, ln)) / 4;
    draw_patch(d, (ORIGWIDTH - width(d, ln)) / 2, y, ln);
}

/// WI_drawOnLnode: draws the first patch that fits on the screen at a map node.
fn draw_on_lnode(d: &mut Doom, n: usize, c: [Option<usize>; 2]) {
    let (x, y) = LNODES[d.wi_stuff.wbs.epsd as usize][n];
    let mut i = 0;
    let mut fits = false;
    loop {
        let Some(p) = c[i] else { break };
        let (w, h, lo, to) = patch_header(d, p);
        let left = x - lo;
        let top = y - to;
        if left >= 0 && left + w < 320 && top >= 0 && top + h < 200 {
            fits = true;
            break;
        }
        i += 1;
        if i == 2 {
            break;
        }
    }
    if fits && i < 2 {
        draw_patch(d, x, y, c[i].unwrap());
    } else {
        print!("Could not place patch on level {}", n + 1);
    }
}

/// WI_initAnimatedBack.
fn init_animated_back(d: &mut Doom) {
    let epsd = d.wi_stuff.wbs.epsd;
    if is_commercial(d) || epsd > 2 {
        return;
    }
    let bcnt = d.wi_stuff.bcnt;
    for (i, def) in anim_defs(epsd).iter().enumerate() {
        d.wi_stuff.anims[i].ctr = -1;
        let nexttic = match def.0 {
            ANIM_ALWAYS => bcnt + 1 + (m_random(&mut d.m_random) % def.1),
            ANIM_RANDOM => bcnt + 1 + (m_random(&mut d.m_random) % def.5),
            _ => bcnt + 1,
        };
        d.wi_stuff.anims[i].nexttic = nexttic;
    }
}

/// WI_updateAnimatedBack.
fn update_animated_back(d: &mut Doom) {
    let epsd = d.wi_stuff.wbs.epsd;
    if is_commercial(d) || epsd > 2 {
        return;
    }
    let w = &mut d.wi_stuff;
    for (i, def) in anim_defs(epsd).iter().enumerate() {
        let a = &mut w.anims[i];
        if w.bcnt != a.nexttic {
            continue;
        }
        match def.0 {
            ANIM_ALWAYS => {
                a.ctr += 1;
                if a.ctr >= def.2 {
                    a.ctr = 0;
                }
                a.nexttic = w.bcnt + def.1;
            }
            ANIM_RANDOM => {
                a.ctr += 1;
                if a.ctr == def.2 {
                    a.ctr = -1;
                    a.nexttic = w.bcnt + m_random(&mut d.m_random) % def.5;
                } else {
                    a.nexttic = w.bcnt + def.1;
                }
            }
            ANIM_LEVEL => {
                if !(w.state == STAT_COUNT && i == 7) && w.wbs.next == def.5 {
                    a.ctr += 1;
                    if a.ctr == def.2 {
                        a.ctr -= 1;
                    }
                    a.nexttic = w.bcnt + def.1;
                }
            }
            _ => {}
        }
    }
}

/// WI_drawAnimatedBack.
fn draw_animated_back(d: &mut Doom) {
    let epsd = d.wi_stuff.wbs.epsd;
    if is_commercial(d) || epsd > 2 {
        return;
    }
    for (i, def) in anim_defs(epsd).iter().enumerate() {
        let a = d.wi_stuff.anims[i];
        if a.ctr >= 0 {
            draw_patch(d, def.3, def.4, a.p[a.ctr as usize]);
        }
    }
}

/// WI_drawNum: draws a right-aligned number and returns the new left x.
fn draw_num(d: &mut Doom, mut x: i32, y: i32, mut n: i32, mut digits: i32) -> i32 {
    let fontwidth = width(d, d.wi_stuff.num[0]);
    if digits < 0 {
        if n == 0 {
            digits = 1;
        } else {
            digits = 0;
            let mut temp = n;
            while temp != 0 {
                temp /= 10;
                digits += 1;
            }
        }
    }
    let neg = n < 0;
    if neg {
        n = n.wrapping_neg();
    }
    if n == 1994 {
        return 0;
    }
    while digits > 0 {
        digits -= 1;
        x -= fontwidth;
        let p = d.wi_stuff.num[(n % 10) as usize];
        draw_patch(d, x, y, p);
        n /= 10;
    }
    if neg {
        x -= 8;
        let m = d.wi_stuff.wiminus;
        draw_patch(d, x, y, m);
    }
    x
}

/// WI_drawPercent.
fn draw_percent(d: &mut Doom, x: i32, y: i32, p: i32) {
    if p < 0 {
        return;
    }
    let pc = d.wi_stuff.percent;
    draw_patch(d, x, y, pc);
    draw_num(d, x, y, p, -1);
}

/// WI_drawTime: draws a time in seconds as minutes and seconds, or "sucks".
fn draw_time(d: &mut Doom, mut x: i32, y: i32, t: i32) {
    if t < 0 {
        return;
    }
    if t <= 61 * 59 {
        let colon = d.wi_stuff.colon;
        let mut div = 1;
        loop {
            let n = (t / div) % 60;
            x = draw_num(d, x, y, n, 2) - width(d, colon);
            div *= 60;
            if div == 60 || t / div != 0 {
                draw_patch(d, x, y, colon);
            }
            if t / div == 0 {
                break;
            }
        }
    } else {
        let s = d.wi_stuff.sucks;
        draw_patch(d, x - width(d, s), y, s);
    }
}

/// WI_initNoState.
fn init_no_state(d: &mut Doom) {
    let w = &mut d.wi_stuff;
    w.state = NO_STATE;
    w.acceleratestage = 0;
    w.cnt = 10;
}

/// WI_updateNoState.
fn update_no_state(d: &mut Doom) {
    update_animated_back(d);
    d.wi_stuff.cnt -= 1;
    if d.wi_stuff.cnt == 0 {
        crate::g_game::world_done(d);
    }
}

/// WI_initShowNextLoc.
fn init_show_next_loc(d: &mut Doom) {
    let w = &mut d.wi_stuff;
    w.state = SHOW_NEXT_LOC;
    w.acceleratestage = 0;
    w.cnt = SHOWNEXTLOCDELAY * TICRATE;
    init_animated_back(d);
}

/// WI_updateShowNextLoc.
fn update_show_next_loc(d: &mut Doom) {
    update_animated_back(d);
    let w = &mut d.wi_stuff;
    w.cnt -= 1;
    if w.cnt == 0 || w.acceleratestage != 0 {
        init_no_state(d);
    } else {
        w.snl_pointeron = (w.cnt & 31) < 20;
    }
}

/// WI_drawShowNextLoc.
fn draw_show_next_loc(d: &mut Doom) {
    slam_background(d);
    draw_animated_back(d);
    let wbs = d.wi_stuff.wbs;
    if !is_commercial(d) {
        if wbs.epsd > 2 {
            draw_el(d);
            return;
        }
        let last = if wbs.last == 8 {
            wbs.next - 1
        } else {
            wbs.last
        };
        let splat = d.wi_stuff.splat;
        let mut i = 0;
        while i <= last {
            draw_on_lnode(d, i as usize, splat);
            i += 1;
        }
        if wbs.didsecret {
            draw_on_lnode(d, 8, splat);
        }
        if d.wi_stuff.snl_pointeron {
            let yah = d.wi_stuff.yah;
            draw_on_lnode(d, wbs.next as usize, yah);
        }
    }
    if !is_commercial(d) || wbs.next != 30 {
        draw_el(d);
    }
}

/// WI_drawNoState.
fn draw_no_state(d: &mut Doom) {
    d.wi_stuff.snl_pointeron = true;
    draw_show_next_loc(d);
}

/// WI_fragSum.
fn frag_sum(d: &Doom, playernum: usize) -> i32 {
    let mut frags = 0;
    for i in 0..MAXPLAYERS {
        if d.g_game.playeringame[i] && i != playernum {
            frags += d.wi_stuff.plrs[playernum].frags[i];
        }
    }
    frags - d.wi_stuff.plrs[playernum].frags[playernum]
}

/// WI_initDeathmatchStats.
fn init_deathmatch_stats(d: &mut Doom) {
    let ingame = d.g_game.playeringame;
    let w = &mut d.wi_stuff;
    w.state = STAT_COUNT;
    w.acceleratestage = 0;
    w.dm_state = 1;
    w.cnt_pause = TICRATE;
    for i in 0..MAXPLAYERS {
        if ingame[i] {
            for j in 0..MAXPLAYERS {
                if ingame[j] {
                    w.dm_frags[i][j] = 0;
                }
            }
            w.dm_totals[i] = 0;
        }
    }
    init_animated_back(d);
}

fn start_sound(d: &mut Doom, sfx: i32) {
    s_sound::start_sound(d, None, sfx);
}

/// WI_updateDeathmatchStats.
fn update_deathmatch_stats(d: &mut Doom) {
    update_animated_back(d);
    let ingame = d.g_game.playeringame;
    if d.wi_stuff.acceleratestage != 0 && d.wi_stuff.dm_state != 4 {
        d.wi_stuff.acceleratestage = 0;
        for i in 0..MAXPLAYERS {
            if ingame[i] {
                for j in 0..MAXPLAYERS {
                    if ingame[j] {
                        d.wi_stuff.dm_frags[i][j] = d.wi_stuff.plrs[i].frags[j];
                    }
                }
                d.wi_stuff.dm_totals[i] = frag_sum(d, i);
            }
        }
        start_sound(d, SFX_BAREXP);
        d.wi_stuff.dm_state = 4;
    }
    let st = d.wi_stuff.dm_state;
    if st == 2 {
        if d.wi_stuff.bcnt & 3 == 0 {
            start_sound(d, SFX_PISTOL);
        }
        let mut stillticking = false;
        for i in 0..MAXPLAYERS {
            if !ingame[i] {
                continue;
            }
            for j in 0..MAXPLAYERS {
                let target = d.wi_stuff.plrs[i].frags[j];
                let f = &mut d.wi_stuff.dm_frags[i][j];
                if ingame[j] && *f != target {
                    if target < 0 {
                        *f -= 1;
                    } else {
                        *f += 1;
                    }
                    *f = (*f).clamp(-99, 99);
                    stillticking = true;
                }
            }
            d.wi_stuff.dm_totals[i] = frag_sum(d, i).clamp(-99, 99);
        }
        if !stillticking {
            start_sound(d, SFX_BAREXP);
            d.wi_stuff.dm_state += 1;
        }
    } else if st == 4 {
        if d.wi_stuff.acceleratestage != 0 {
            start_sound(d, SFX_SLOP);
            if is_commercial(d) {
                init_no_state(d);
            } else {
                init_show_next_loc(d);
            }
        }
    } else if st & 1 != 0 {
        d.wi_stuff.cnt_pause -= 1;
        if d.wi_stuff.cnt_pause == 0 {
            d.wi_stuff.dm_state += 1;
            d.wi_stuff.cnt_pause = TICRATE;
        }
    }
}

/// WI_drawDeathmatchStats.
fn draw_deathmatch_stats(d: &mut Doom) {
    slam_background(d);
    draw_animated_back(d);
    draw_lf(d);
    let w = &d.wi_stuff;
    let (total, killers, victims, star, bstar, p) =
        (w.total, w.killers, w.victims, w.star, w.bstar, w.p);
    draw_patch(
        d,
        DM_TOTALSX - width(d, total) / 2,
        DM_MATRIXY - WI_SPACINGY + 10,
        total,
    );
    draw_patch(d, DM_KILLERSX, DM_KILLERSY, killers);
    draw_patch(d, DM_VICTIMSX, DM_VICTIMSY, victims);
    let mut x = DM_MATRIXX + DM_SPACINGX;
    let mut y = DM_MATRIXY;
    let me = d.wi_stuff.me;
    for i in 0..MAXPLAYERS {
        if d.g_game.playeringame[i] {
            let pw = width(d, p[i]) / 2;
            draw_patch(d, x - pw, DM_MATRIXY - WI_SPACINGY, p[i]);
            draw_patch(d, DM_MATRIXX - pw, y, p[i]);
            if i == me {
                draw_patch(d, x - pw, DM_MATRIXY - WI_SPACINGY, bstar);
                draw_patch(d, DM_MATRIXX - pw, y, star);
            }
        }
        x += DM_SPACINGX;
        y += WI_SPACINGY;
    }
    let mut y = DM_MATRIXY + 10;
    let w = width(d, d.wi_stuff.num[0]);
    for i in 0..MAXPLAYERS {
        let mut x = DM_MATRIXX + DM_SPACINGX;
        if d.g_game.playeringame[i] {
            for j in 0..MAXPLAYERS {
                if d.g_game.playeringame[j] {
                    let f = d.wi_stuff.dm_frags[i][j];
                    draw_num(d, x + w, y, f, 2);
                }
                x += DM_SPACINGX;
            }
            let t = d.wi_stuff.dm_totals[i];
            draw_num(d, DM_TOTALSX + w, y, t, 2);
        }
        y += WI_SPACINGY;
    }
}

/// WI_initNetgameStats.
fn init_netgame_stats(d: &mut Doom) {
    let w = &mut d.wi_stuff;
    w.state = STAT_COUNT;
    w.acceleratestage = 0;
    w.ng_state = 1;
    w.cnt_pause = TICRATE;
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        let w = &mut d.wi_stuff;
        w.cnt_kills[i] = 0;
        w.cnt_items[i] = 0;
        w.cnt_secret[i] = 0;
        w.cnt_frags[i] = 0;
        let fs = frag_sum(d, i);
        d.wi_stuff.dofrags += fs;
    }
    d.wi_stuff.dofrags = (d.wi_stuff.dofrags != 0) as i32;
    init_animated_back(d);
}

/// Counts one netgame column toward its final percent. Returns true while any value still counts.
fn count_column(d: &mut Doom, which: usize) -> bool {
    let mut stillticking = false;
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        let w = &mut d.wi_stuff;
        let (v, cnt) = match which {
            0 => (
                (w.plrs[i].skills * 100) / w.wbs.maxkills,
                &mut w.cnt_kills[i],
            ),
            1 => (
                (w.plrs[i].sitems * 100) / w.wbs.maxitems,
                &mut w.cnt_items[i],
            ),
            _ => (
                (w.plrs[i].ssecret * 100) / w.wbs.maxsecret,
                &mut w.cnt_secret[i],
            ),
        };
        *cnt += 2;
        if *cnt >= v {
            *cnt = v;
        } else {
            stillticking = true;
        }
    }
    stillticking
}

/// WI_updateNetgameStats.
fn update_netgame_stats(d: &mut Doom) {
    update_animated_back(d);
    if d.wi_stuff.acceleratestage != 0 && d.wi_stuff.ng_state != 10 {
        d.wi_stuff.acceleratestage = 0;
        for i in 0..MAXPLAYERS {
            if !d.g_game.playeringame[i] {
                continue;
            }
            let w = &mut d.wi_stuff;
            w.cnt_kills[i] = (w.plrs[i].skills * 100) / w.wbs.maxkills;
            w.cnt_items[i] = (w.plrs[i].sitems * 100) / w.wbs.maxitems;
            w.cnt_secret[i] = (w.plrs[i].ssecret * 100) / w.wbs.maxsecret;
            if w.dofrags != 0 {
                d.wi_stuff.cnt_frags[i] = frag_sum(d, i);
            }
        }
        start_sound(d, SFX_BAREXP);
        d.wi_stuff.ng_state = 10;
    }
    let st = d.wi_stuff.ng_state;
    if st == 2 || st == 4 || st == 6 {
        if d.wi_stuff.bcnt & 3 == 0 {
            start_sound(d, SFX_PISTOL);
        }
        if !count_column(d, (st / 2 - 1) as usize) {
            start_sound(d, SFX_BAREXP);
            if st == 6 {
                d.wi_stuff.ng_state += 1 + 2 * (d.wi_stuff.dofrags == 0) as i32;
            } else {
                d.wi_stuff.ng_state += 1;
            }
        }
    } else if st == 8 {
        if d.wi_stuff.bcnt & 3 == 0 {
            start_sound(d, SFX_PISTOL);
        }
        let mut stillticking = false;
        for i in 0..MAXPLAYERS {
            if !d.g_game.playeringame[i] {
                continue;
            }
            d.wi_stuff.cnt_frags[i] += 1;
            let fsum = frag_sum(d, i);
            if d.wi_stuff.cnt_frags[i] >= fsum {
                d.wi_stuff.cnt_frags[i] = fsum;
            } else {
                stillticking = true;
            }
        }
        if !stillticking {
            start_sound(d, SFX_PLDETH);
            d.wi_stuff.ng_state += 1;
        }
    } else if st == 10 {
        if d.wi_stuff.acceleratestage != 0 {
            start_sound(d, SFX_SGCOCK);
            if is_commercial(d) {
                init_no_state(d);
            } else {
                init_show_next_loc(d);
            }
        }
    } else if st & 1 != 0 {
        d.wi_stuff.cnt_pause -= 1;
        if d.wi_stuff.cnt_pause == 0 {
            d.wi_stuff.ng_state += 1;
            d.wi_stuff.cnt_pause = TICRATE;
        }
    }
}

/// WI_drawNetgameStats.
fn draw_netgame_stats(d: &mut Doom) {
    let w = &d.wi_stuff;
    let (percent, kills, items, secret, frags, star, p, dofrags) = (
        w.percent, w.kills, w.items, w.secret, w.frags, w.star, w.p, w.dofrags,
    );
    let pwidth = width(d, percent);
    let ng_statsx = 32 + width(d, star) / 2 + 32 * (dofrags == 0) as i32;
    slam_background(d);
    draw_animated_back(d);
    draw_lf(d);
    draw_patch(
        d,
        ng_statsx + NG_SPACINGX - width(d, kills),
        NG_STATSY,
        kills,
    );
    draw_patch(
        d,
        ng_statsx + 2 * NG_SPACINGX - width(d, items),
        NG_STATSY,
        items,
    );
    draw_patch(
        d,
        ng_statsx + 3 * NG_SPACINGX - width(d, secret),
        NG_STATSY,
        secret,
    );
    if dofrags != 0 {
        draw_patch(
            d,
            ng_statsx + 4 * NG_SPACINGX - width(d, frags),
            NG_STATSY,
            frags,
        );
    }
    let mut y = NG_STATSY + height(d, kills);
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        let mut x = ng_statsx;
        draw_patch(d, x - width(d, p[i]), y, p[i]);
        if i == d.wi_stuff.me {
            draw_patch(d, x - width(d, p[i]), y, star);
        }
        x += NG_SPACINGX;
        let (k, it, s, f) = (
            d.wi_stuff.cnt_kills[i],
            d.wi_stuff.cnt_items[i],
            d.wi_stuff.cnt_secret[i],
            d.wi_stuff.cnt_frags[i],
        );
        draw_percent(d, x - pwidth, y + 10, k);
        x += NG_SPACINGX;
        draw_percent(d, x - pwidth, y + 10, it);
        x += NG_SPACINGX;
        draw_percent(d, x - pwidth, y + 10, s);
        x += NG_SPACINGX;
        if dofrags != 0 {
            draw_num(d, x, y + 10, f, -1);
        }
        y += WI_SPACINGY;
    }
}

/// WI_initStats.
fn init_stats(d: &mut Doom) {
    let w = &mut d.wi_stuff;
    w.state = STAT_COUNT;
    w.acceleratestage = 0;
    w.sp_state = 1;
    w.cnt_kills[0] = -1;
    w.cnt_items[0] = -1;
    w.cnt_secret[0] = -1;
    w.cnt_time = -1;
    w.cnt_par = -1;
    w.cnt_pause = TICRATE;
    init_animated_back(d);
}

/// WI_updateStats.
fn update_stats(d: &mut Doom) {
    update_animated_back(d);
    let me = d.wi_stuff.me;
    let pl = d.wi_stuff.plrs[me];
    let wbs = d.wi_stuff.wbs;
    let kills = (pl.skills * 100) / wbs.maxkills;
    let items = (pl.sitems * 100) / wbs.maxitems;
    let secret = (pl.ssecret * 100) / wbs.maxsecret;
    let time = pl.stime / TICRATE;
    let par = wbs.partime / TICRATE;
    if d.wi_stuff.acceleratestage != 0 && d.wi_stuff.sp_state != 10 {
        let w = &mut d.wi_stuff;
        w.acceleratestage = 0;
        w.cnt_kills[0] = kills;
        w.cnt_items[0] = items;
        w.cnt_secret[0] = secret;
        w.cnt_time = time;
        w.cnt_par = par;
        start_sound(d, SFX_BAREXP);
        d.wi_stuff.sp_state = 10;
    }
    let st = d.wi_stuff.sp_state;
    if st == 2 || st == 4 || st == 6 {
        let w = &mut d.wi_stuff;
        let (cnt, target) = match st {
            2 => (&mut w.cnt_kills[0], kills),
            4 => (&mut w.cnt_items[0], items),
            _ => (&mut w.cnt_secret[0], secret),
        };
        *cnt += 2;
        let reached = *cnt >= target;
        if reached {
            *cnt = target;
        }
        if w.bcnt & 3 == 0 {
            start_sound(d, SFX_PISTOL);
        }
        if reached {
            start_sound(d, SFX_BAREXP);
            d.wi_stuff.sp_state += 1;
        }
    } else if st == 8 {
        if d.wi_stuff.bcnt & 3 == 0 {
            start_sound(d, SFX_PISTOL);
        }
        let w = &mut d.wi_stuff;
        w.cnt_time += 3;
        if w.cnt_time >= time {
            w.cnt_time = time;
        }
        w.cnt_par += 3;
        if w.cnt_par >= par {
            w.cnt_par = par;
            if w.cnt_time >= time {
                start_sound(d, SFX_BAREXP);
                d.wi_stuff.sp_state += 1;
            }
        }
    } else if st == 10 {
        if d.wi_stuff.acceleratestage != 0 {
            start_sound(d, SFX_SGCOCK);
            if is_commercial(d) {
                init_no_state(d);
            } else {
                init_show_next_loc(d);
            }
        }
    } else if st & 1 != 0 {
        let w = &mut d.wi_stuff;
        w.cnt_pause -= 1;
        if w.cnt_pause == 0 {
            w.sp_state += 1;
            w.cnt_pause = TICRATE;
        }
    }
}

/// WI_drawStats.
fn draw_stats(d: &mut Doom) {
    let lh = (3 * height(d, d.wi_stuff.num[0])) / 2;
    slam_background(d);
    draw_animated_back(d);
    draw_lf(d);
    let w = &d.wi_stuff;
    let (kills, items, sp_secret, timepatch, par) =
        (w.kills, w.items, w.sp_secret, w.timepatch, w.par);
    let (ck, ci, cs, ct, cp, epsd) = (
        w.cnt_kills[0],
        w.cnt_items[0],
        w.cnt_secret[0],
        w.cnt_time,
        w.cnt_par,
        w.wbs.epsd,
    );
    draw_patch(d, SP_STATSX, SP_STATSY, kills);
    draw_percent(d, ORIGWIDTH - SP_STATSX, SP_STATSY, ck);
    draw_patch(d, SP_STATSX, SP_STATSY + lh, items);
    draw_percent(d, ORIGWIDTH - SP_STATSX, SP_STATSY + lh, ci);
    draw_patch(d, SP_STATSX, SP_STATSY + 2 * lh, sp_secret);
    draw_percent(d, ORIGWIDTH - SP_STATSX, SP_STATSY + 2 * lh, cs);
    draw_patch(d, SP_TIMEX, SP_TIMEY, timepatch);
    draw_time(d, ORIGWIDTH / 2 - SP_TIMEX, SP_TIMEY, ct);
    if epsd < 3 {
        draw_patch(d, ORIGWIDTH / 2 + SP_TIMEX, SP_TIMEY, par);
        draw_time(d, ORIGWIDTH - SP_TIMEX, SP_TIMEY, cp);
    }
}

/// WI_checkForAccelerate: an attack or use press skips the current count.
fn check_for_accelerate(d: &mut Doom) {
    for i in 0..MAXPLAYERS {
        if !d.g_game.playeringame[i] {
            continue;
        }
        let player = &mut d.g_game.players[i];
        if player.cmd.buttons & BT_ATTACK != 0 {
            if !player.attackdown {
                d.wi_stuff.acceleratestage = 1;
            }
            player.attackdown = true;
        } else {
            player.attackdown = false;
        }
        if player.cmd.buttons & BT_USE != 0 {
            if !player.usedown {
                d.wi_stuff.acceleratestage = 1;
            }
            player.usedown = true;
        } else {
            player.usedown = false;
        }
    }
}

/// WI_Ticker.
pub fn ticker(d: &mut Doom) {
    d.wi_stuff.bcnt += 1;
    if d.wi_stuff.bcnt == 1 {
        let mus = if is_commercial(d) {
            MUS_DM2INT
        } else {
            MUS_INTER
        };
        s_sound::change_music(d, mus, true);
    }
    check_for_accelerate(d);
    match d.wi_stuff.state {
        STAT_COUNT => {
            if d.g_game.deathmatch != 0 {
                update_deathmatch_stats(d);
            } else if d.g_game.netgame {
                update_netgame_stats(d);
            } else {
                update_stats(d);
            }
        }
        SHOW_NEXT_LOC => update_show_next_loc(d),
        NO_STATE => update_no_state(d),
        _ => {}
    }
}

/// WI_loadData: looks up every intermission patch.
fn load_data(d: &mut Doom) {
    let epsd = d.wi_stuff.wbs.epsd;
    let commercial = is_commercial(d);
    let mut lnames = Vec::new();
    if commercial {
        for i in 0..NUMCMAPS {
            lnames.push(lump(d, &format!("CWILV{:02}", i)));
        }
    } else {
        for i in 0..NUMMAPS {
            lnames.push(lump(d, &format!("WILV{}{}", epsd, i)));
        }
        d.wi_stuff.yah = [Some(lump(d, "WIURH0")), Some(lump(d, "WIURH1"))];
        d.wi_stuff.splat = [Some(lump(d, "WISPLAT")), None];
        if epsd < 3 {
            let defs = anim_defs(epsd);
            let mut anims = vec![Anim::default(); defs.len()];
            for (j, def) in defs.iter().enumerate() {
                for i in 0..def.2 as usize {
                    if epsd != 1 || j != 8 {
                        anims[j].p[i] = lump(d, &format!("WIA{}{:02}{:02}", epsd, j, i));
                    } else {
                        anims[j].p[i] = anims[4].p[i];
                    }
                }
            }
            d.wi_stuff.anims = anims;
        }
    }
    d.wi_stuff.lnames = lnames;
    d.wi_stuff.wiminus = lump(d, "WIMINUS");
    for i in 0..10 {
        d.wi_stuff.num[i] = lump(d, &format!("WINUM{}", i));
    }
    d.wi_stuff.percent = lump(d, "WIPCNT");
    d.wi_stuff.finished = lump(d, "WIF");
    d.wi_stuff.entering = lump(d, "WIENTER");
    d.wi_stuff.kills = lump(d, "WIOSTK");
    d.wi_stuff.secret = lump(d, "WIOSTS");
    d.wi_stuff.sp_secret = lump(d, "WISCRT2");
    d.wi_stuff.items = lump(d, "WIOSTI");
    d.wi_stuff.frags = lump(d, "WIFRGS");
    d.wi_stuff.colon = lump(d, "WICOLON");
    d.wi_stuff.timepatch = lump(d, "WITIME");
    d.wi_stuff.sucks = lump(d, "WISUCKS");
    d.wi_stuff.par = lump(d, "WIPAR");
    d.wi_stuff.killers = lump(d, "WIKILRS");
    d.wi_stuff.victims = lump(d, "WIVCTMS");
    d.wi_stuff.total = lump(d, "WIMSTT");
    d.wi_stuff.star = lump(d, "STFST01");
    d.wi_stuff.bstar = lump(d, "STFDEAD0");
    for i in 0..MAXPLAYERS {
        d.wi_stuff.p[i] = lump(d, &format!("STPB{}", i));
        d.wi_stuff.bp[i] = lump(d, &format!("WIBP{}", i + 1));
    }
    let name = if commercial || (d.d_main.gamemode == RETAIL && epsd == 3) {
        "INTERPIC".to_string()
    } else {
        format!("WIMAP{}", epsd)
    };
    d.wi_stuff.background = lump(d, &name);
}

/// WI_Drawer.
pub fn drawer(d: &mut Doom) {
    match d.wi_stuff.state {
        STAT_COUNT => {
            if d.g_game.deathmatch != 0 {
                draw_deathmatch_stats(d);
            } else if d.g_game.netgame {
                draw_netgame_stats(d);
            } else {
                draw_stats(d);
            }
        }
        SHOW_NEXT_LOC => draw_show_next_loc(d),
        NO_STATE => draw_no_state(d),
        _ => {}
    }
}

/// WI_initVariables.
fn init_variables(d: &mut Doom, wbstartstruct: &WbStart) {
    let mut wbs = *wbstartstruct;
    if wbs.maxkills == 0 {
        wbs.maxkills = 1;
    }
    if wbs.maxitems == 0 {
        wbs.maxitems = 1;
    }
    if wbs.maxsecret == 0 {
        wbs.maxsecret = 1;
    }
    if d.d_main.gamemode != RETAIL && wbs.epsd > 2 {
        wbs.epsd -= 3;
    }
    let w = &mut d.wi_stuff;
    w.acceleratestage = 0;
    w.cnt = 0;
    w.bcnt = 0;
    w.me = wbs.pnum as usize;
    w.plrs = wbs.plyr;
    w.wbs = wbs;
    // The original changes the caller's wbstartstruct_t through its pointer.
    d.g_game.wminfo.maxkills = wbs.maxkills;
    d.g_game.wminfo.maxitems = wbs.maxitems;
    d.g_game.wminfo.maxsecret = wbs.maxsecret;
    d.g_game.wminfo.epsd = wbs.epsd;
}

/// WI_Start.
pub fn start(d: &mut Doom, wbstartstruct: &WbStart) {
    init_variables(d, wbstartstruct);
    load_data(d);
    if d.g_game.deathmatch != 0 {
        init_deathmatch_stats(d);
    } else if d.g_game.netgame {
        init_netgame_stats(d);
    } else {
        init_stats(d);
    }
}

/// WI_Responder: the intermission takes no events directly.
#[allow(dead_code)]
pub fn responder(_d: &mut Doom, _ev: &crate::d_main::Event) -> bool {
    false
}

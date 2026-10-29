# doom program: faithful Rust design

This design implements the full product for target SHA-256 `2d0cfd7af1290d434d700eec470e7a0df5d31ed3e4e904ae6b748f5bdc6e0da9`.
It follows arc42 sections. The ledger `recovered/model.yaml` holds the records.

## Solution strategy

The original execution model is one process with one thread. One endless loop runs game tics from the SDL millisecond clock at 35 tics per second, renders one 320x200 palette frame, scales it two times into a 640x400 32-bit buffer, presents it through SDL2, and then drains SDL key events into a 16-entry queue.
The Rust execution model is the same single-threaded blocking loop. The Rust code calls the same 11 SDL2 functions in the same order (ADR 0003).
The design preserves the tic timing rule `ticks * 35 / 1000`, the order of the P_Random and M_Random calls, the thinker order, the state and thing tables, and every diagnostic text.

## Building blocks

| Original component | Rust module | Responsibility and boundary | Preserved state and interfaces |
| --- | --- | --- | --- |
| C-010 | `src/i_video.rs` | SDL platform layer: frame conversion and present, palette with gamma, input event poll, quit event. | The globals of i_video in `Doom::i_video` where the module keeps state. |
| C-011 | `src/i_system.rs` | Fatal error report with zenity dialog and exit; DOS memory emulation; 35 Hz tic clock. | The globals of i_system in `Doom::i_system` where the module keeps state. |
| C-012 | `src/d_main.rs` | Startup sequence, event dispatch to responders, per-frame display, attract page and demo sequence, exit screen. | The globals of d_main in `Doom::d_main` where the module keeps state. |
| C-013 | `src/d_iwad.rs` | IWAD search over a fixed name list and -iwad path check. | The globals of d_iwad in `Doom::d_iwad` where the module keeps state. |
| C-014 | `src/d_loop.rs` | Tic loop: build ticcmds, run game tics, wait on the clock; single local node. | The globals of d_loop in `Doom::d_loop` where the module keeps state. |
| C-015 | `src/g_game.rs` | Game state and actions: ticcmd build, level load and exit, new game, demo record and playback, save and load control. | The globals of g_game in `Doom::g_game` where the module keeps state. |
| C-016 | `src/m_menu.rs` | Menu screens, prompts, savegame slot names, quick save and load, option settings. | The globals of m_menu in `Doom::m_menu` where the module keeps state. |
| C-017 | `src/m_config.rs` | Configuration variable table and lookup by name. | The globals of m_config in `Doom::m_config` where the module keeps state. |
| C-018 | `src/m_argv.rs` | Command-line option lookup. | The globals of m_argv in `Doom::m_argv` where the module keeps state. |
| C-019 | `src/m_misc.rs` | String join, formatted print, integer parse, and whole-file write helpers. | The globals of m_misc in `Doom::m_misc` where the module keeps state. |
| C-020 | `src/m_random.rs` | Deterministic random table and indexes; reads are inlined into 61 functions. | The globals of m_random in `Doom::m_random` where the module keeps state. |
| C-021 | `src/m_fixed.rs` | 16.16 fixed-point multiply and divide. | The globals of m_fixed in `Doom::m_fixed` where the module keeps state. |
| C-022 | `src/tables.rs` | Fine sine, tangent, and arctangent tables. | The globals of tables in `Doom::tables` where the module keeps state. |
| C-023 | `src/m_cheat.rs` | Cheat key sequence match. | The globals of m_cheat in `Doom::m_cheat` where the module keeps state. |
| C-024 | `src/w_wad.rs` | WAD file open, lump directory, name lookup, lump read and cache; stdio file backend. | The globals of w_wad in `Doom::w_wad` where the module keeps state. |
| C-025 | `src/z_zone.rs` | Zone memory allocator with tags, owners, and purge. | The globals of z_zone in `Doom::z_zone` where the module keeps state. |
| C-026 | `src/v_video.rs` | Screen buffers, patch draw, rectangle copy, screenshot. | The globals of v_video in `Doom::v_video` where the module keeps state. |
| C-027 | `src/f_wipe.rs` | Screen melt transition between game states. | The globals of f_wipe in `Doom::f_wipe` where the module keeps state. |
| C-028 | `src/f_finale.rs` | Episode end text, art screens, and bunny scroll. | The globals of f_finale in `Doom::f_finale` where the module keeps state. |
| C-029 | `src/wi_stuff.rs` | Intermission statistics screen. | The globals of wi_stuff in `Doom::wi_stuff` where the module keeps state. |
| C-030 | `src/hu_stuff.rs` | Heads-up messages, map title, chat input. | The globals of hu_stuff in `Doom::hu_stuff` where the module keeps state. |
| C-031 | `src/hu_lib.rs` | Text line and widget primitives for the heads-up display. | The globals of hu_lib in `Doom::hu_lib` where the module keeps state. |
| C-032 | `src/st_stuff.rs` | Status bar state, face, cheat responder, drawer. | The globals of st_stuff in `Doom::st_stuff` where the module keeps state. |
| C-033 | `src/st_lib.rs` | Status bar number and icon widgets. | The globals of st_lib in `Doom::st_lib` where the module keeps state. |
| C-034 | `src/am_map.rs` | Automap view, marks, grid, follow mode. | The globals of am_map in `Doom::am_map` where the module keeps state. |
| C-035 | `src/s_sound.rs` | Sound and music control; output device is silent in this build. | The globals of s_sound in `Doom::s_sound` where the module keeps state. |
| C-036 | `src/info.rs` | State, sprite name, and thing type tables. | The globals of info in `Doom::info` where the module keeps state. |
| C-037 | `src/d_items.rs` | Weapon table: ammo type and state indexes. | The globals of d_items in `Doom::d_items` where the module keeps state. |
| C-038 | `src/r_main.rs` | View setup, angle and subsector helpers, render entry. | The globals of r_main in `Doom::r_main` where the module keeps state. |
| C-039 | `src/r_bsp.rs` | BSP traversal, subsector and line clipping. | The globals of r_bsp in `Doom::r_bsp` where the module keeps state. |
| C-040 | `src/r_segs.rs` | Wall segment ranges and column setup. | The globals of r_segs in `Doom::r_segs` where the module keeps state. |
| C-041 | `src/r_plane.rs` | Visplanes and floor and ceiling spans. | The globals of r_plane in `Doom::r_plane` where the module keeps state. |
| C-042 | `src/r_things.rs` | Sprite projection, sort, masked draw, player weapon sprites. | The globals of r_things in `Doom::r_things` where the module keeps state. |
| C-043 | `src/r_draw.rs` | Column, fuzz, translated, and span drawers; view border. | The globals of r_draw in `Doom::r_draw` where the module keeps state. |
| C-044 | `src/r_data.rs` | Texture, flat, sprite lump and colormap setup; composite columns. | The globals of r_data in `Doom::r_data` where the module keeps state. |
| C-045 | `src/r_sky.rs` | Sky texture choice and sky column mapping. | The globals of r_sky in `Doom::r_sky` where the module keeps state. |
| C-046 | `src/p_setup.rs` | Map lump load and level object spawn. | The globals of p_setup in `Doom::p_setup` where the module keeps state. |
| C-047 | `src/p_tick.rs` | Thinker list and per-tic world update. | The globals of p_tick in `Doom::p_tick` where the module keeps state. |
| C-048 | `src/p_mobj.rs` | Map object spawn, state change, movement, missiles. | The globals of p_mobj in `Doom::p_mobj` where the module keeps state. |
| C-049 | `src/p_enemy.rs` | Monster AI and monster action functions. | The globals of p_enemy in `Doom::p_enemy` where the module keeps state. |
| C-050 | `src/p_inter.rs` | Pickups, damage, death, ammo and weapon grants. | The globals of p_inter in `Doom::p_inter` where the module keeps state. |
| C-051 | `src/p_map.rs` | Movement checks, slide, use, attack, radius damage. | The globals of p_map in `Doom::p_map` where the module keeps state. |
| C-052 | `src/p_maputl.rs` | Blockmap iteration, path traverse, line geometry. | The globals of p_maputl in `Doom::p_maputl` where the module keeps state. |
| C-053 | `src/p_sight.rs` | Line-of-sight checks. | The globals of p_sight in `Doom::p_sight` where the module keeps state. |
| C-054 | `src/p_spec.rs` | Sector and line specials, animations, donut, crossing triggers. | The globals of p_spec in `Doom::p_spec` where the module keeps state. |
| C-055 | `src/p_switch.rs` | Switch textures, buttons, use-line dispatch. | The globals of p_switch in `Doom::p_switch` where the module keeps state. |
| C-056 | `src/p_doors.rs` | Door movers and key door checks. | The globals of p_doors in `Doom::p_doors` where the module keeps state. |
| C-057 | `src/p_floor.rs` | Floor movers and stairs. | The globals of p_floor in `Doom::p_floor` where the module keeps state. |
| C-058 | `src/p_ceilng.rs` | Ceiling movers and crushers. | The globals of p_ceilng in `Doom::p_ceilng` where the module keeps state. |
| C-059 | `src/p_plats.rs` | Platform lifts. | The globals of p_plats in `Doom::p_plats` where the module keeps state. |
| C-060 | `src/p_lights.rs` | Light flicker, flash, strobe, glow. | The globals of p_lights in `Doom::p_lights` where the module keeps state. |
| C-061 | `src/p_telept.rs` | Teleport lines. | The globals of p_telept in `Doom::p_telept` where the module keeps state. |
| C-062 | `src/p_user.rs` | Player think, movement, view bob, sector damage. | The globals of p_user in `Doom::p_user` where the module keeps state. |
| C-063 | `src/p_pspr.rs` | Player weapon sprites and weapon action functions. | The globals of p_pspr in `Doom::p_pspr` where the module keeps state. |
| C-064 | `src/p_saveg.rs` | Savegame byte serialization of players, world, thinkers, and specials. | The globals of p_saveg in `Doom::p_saveg` where the module keeps state. |
| C-065 | `src/statdump.rs` | Level statistics copy for -statdump. | The globals of statdump in `Doom::statdump` where the module keeps state. |
| C-066 | `src/main.rs` | Process entry: frame buffer, SDL window, renderer, texture; endless frame loop. | The globals of doomgeneric entry in `Doom::main` where the module keeps state. |
| C-068 | `src/sha1.rs` | SHA-1 digest used during startup. | The globals of sha1 in `Doom::sha1` where the module keeps state. |

`main.rs` also declares the module tree and the `Doom` state value (ADR 0001). Generated data stays in `src/data/*.in` files that `include!` loads (ADR 0004).

## Runtime

| Slice | Kind and risk | Requirements | Scenario that proves the slice |
| --- | --- | --- | --- |
| SLICE-001 | skeleton, high | 44 requirements, REQ-001 to REQ-112 | `startup-explicit-iwad` |
| SLICE-002 | slice, medium | 43 requirements, REQ-093 to REQ-255 | `menu-navigation` |
| SLICE-003 | slice, high | 54 requirements, REQ-010 to REQ-322 | `warp-e1m1` |
| SLICE-004 | slice, high | 29 requirements, REQ-056 to REQ-203 | `timedemo-demo1` |
| SLICE-005 | slice, medium | 44 requirements, REQ-099 to REQ-324 | `cheats-god-ammo` |
| SLICE-006 | slice, medium | 35 requirements, REQ-018 to REQ-335 | `exit-intermission` |

The skeleton runs the startup log, the window, the title page, and the exit paths through every container.
Later slices add the menus, the level view and simulation, the demos, the status bar and automap, and the level exit screens and savegames.

## Cross-cutting concepts

The error model keeps the original I_Error path: the message goes to standard error and the status is 255. Signal crashes of the original stay signal crashes (ADR 0008).
The I/O boundaries are SDL2 in `i_video.rs`, files in `w_wad.rs`, `m_misc.rs`, `m_config.rs`, `g_game.rs`, and `p_saveg.rs`, and the clock in `i_video.rs` and `d_loop.rs`.
The data formats are the WAD format, the demo format (13-byte header, 4 bytes per player per tic, 0x80 end marker), and the savegame format (EV-A022, EV-P011).

The quality budgets are: a timedemo of 7117 tics finishes in less than 300 seconds under the replay harness; memory stays below 512 MiB; no `unsafe` code exists outside the SDL calls in `i_video.rs`.
These budgets are user and harness constraints. They are not observations of the original.

## Decisions

| ADR | Decision | Status |
| --- | --- | --- |
| [0001](adr/0001-engine-state-aggregate.md) | Keep module globals in one engine state value | accepted |
| [0002](adr/0002-thinker-arena.md) | Keep thinkers in an index arena | accepted |
| [0003](adr/0003-sdl-ffi.md) | Call the same SDL2 library through a minimal foreign interface | accepted |
| [0004](adr/0004-generated-tables.md) | Generate the data tables from the original binary | accepted |
| [0005](adr/0005-zone-memory.md) | Replace zone memory with Rust ownership | accepted |
| [0006](adr/0006-uninitialized-memory.md) | Do not reproduce reads of uninitialized memory | accepted |
| [0007](adr/0007-containers-outside-ledger.md) | Keep the containers outside the component ledger | accepted |
| [0008](adr/0008-standard-stream-buffering.md) | Buffer standard output as the C library does | accepted |

## Risks and gaps

The open gaps are listed in `recovered/roadmap.md`. GAP-D01, GAP-D02, and GAP-D03 come from ADRs 0002, 0005, and 0006.
No slice is blocked. The demo slice SLICE-004 has the highest risk, because one wrong random call breaks demo synchronization.

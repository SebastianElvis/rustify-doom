# 0006. Reproduce reads of stale zone memory with a zone byte model

- Status: accepted
- Date: 2026-10-29
- Items: C-025, C-027, C-029, C-064

## Context

The original melts the first screen of a level from I_VideoBuffer. I_InitGraphics allocates this buffer with `Z_Malloc(64000, PU_STATIC)` and does not clear it.
The buffer holds the stale bytes of earlier zone blocks: patches from R_GenerateLookup, level lumps, and free block headers.
The column drawers also read past the end of a patch or a composite texture column. These reads return the bytes of the next zone block.
R_GenerateComposite does not clear its block, so the gaps between posts keep stale bytes.
Two original runs give the same bytes, except the high bytes of zone pointers. The zone base changes in each run, but its low 22 bits stay 0x0.
Some intermission numbers and the savegame bytes also vary between two original runs (EV-007, rule V-1).

## Decision

`z_zone.rs` keeps a byte model of the 6 MiB zone. The model copies Z_Init, Z_Malloc with purge, Z_Free, Z_ChangeTag, Z_FreeTags, and W_CacheLumpNum from the binary. A pointer in the model is a fixed base plus the zone offset.
The modules call the model in the original order. `r_data::zone_sync` replays the zone calls of the level setup, the music, the demo lump, and the thinker blocks before each frame.
The video buffer starts with the model bytes. The column sources read the model bytes, so reads past a column end return the next zone bytes.
The Rust containers still own the game data (ADR 0005). The replay harness does not assert frames and files that vary between two original runs.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Zone byte model and rule V-1 | Yes for stable bytes. Pointer high bytes stay unasserted. | `warp-e1m1`, `timedemo-demo1`, `attract-loop`, `exit-intermission` |
| Zero bytes | No. The first melt and the over-read pixels differ in frames that are stable between original runs. | Session 2 report |

## Consequences

The first melt and the over-read pixels match the original when the model follows the original zone calls.
Thinker blocks are replayed per frame, with frees before allocations. The original order inside one tic can differ, so a rare block can move.
The model does not follow the zone calls of the intermission, the finale, the automap, and savegames. After these screens, an over-read pixel can differ.
A pixel that shows a zone pointer high byte varies between original runs. GAP-D03 records these frames.

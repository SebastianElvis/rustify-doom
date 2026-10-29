# Compatibility

This file gives the measured parity of the Rust build with the original binary.
The Rust build does not have full parity yet.
Finite tests cannot prove equivalence for every input.

## Original

- Binary: doomgeneric with the SDL2 front end, SHA-256 `2d0cfd7af1290d434d700eec470e7a0df5d31ed3e4e904ae6b748f5bdc6e0da9`.
- Game data: `freedoom1.wad` from Freedoom 0.13.0.
- Host: macOS on Apple silicon.

## Independent scorer

The [Rustify Doom scorer](https://github.com/SebastianElvis/rustify/tree/main/eval/open/doom) measured this build.
The scorer cases were not an input to the Rustify agent.

| Measure | Result |
| --- | --- |
| Correctness gate | Passed |
| Case pass rate | 0.82 |
| Feature parity score | 0.707 |
| Feature coverage | 0.576 |
| Milestone credit | 0.333 |
| Strict pass | No |

| Report | Passed | Failed |
| --- | --- | --- |
| `replay-core` | 12 | 0 |
| `replay-extended` | 50 | 3 |
| `replay-complete` | 8 | 0 |
| `generate-demos` | 45 | 15 |
| `upstream-demo1` | 2 | 0 |
| `upstream-demo2` | 1 | 1 |
| `upstream-demo3` | 1 | 1 |
| `upstream-demo4` | 2 | 0 |

19 of 20 failed cases differ in the written files. Each of these cases writes savegames.
The other failed case, `memory/mb-1`, differs in its steps.
The scorer cannot capture frames on its host.
The renderer, the menus, the status bar, the automap, the cheats, the controls, the window, and the screenshot therefore need manual checks.

## Agent validation

The Rustify agent ran its own checks on the final build.

- 234 of 234 recorded scenarios pass. Each scenario compares the text output, the exit status, the written files, and a hash of each frame.
- 455 of 479 recovered requirements pass. No requirement fails. 24 requirements stay open, because an open gap names them.
- The parity gate reports `incomplete`, with 406 open gaps.
- Format, `clippy -D warnings`, and 14 unit tests pass.
- `unsafe` code occurs only in `src/i_video.rs`, for the SDL calls, the C library streams, and the signals.

## Open gaps

- The savegame layout after the header.
- Edge cases of `-spechit`, `-donut`, `-reject_pad_with_ff`, `-netdemo`, `-left`, and `-right`.
- The level end at the `-timer` limit, other game versions, and long-tics demos.
- One rule for each monster, weapon, and sector special.
- The pause graphic, the attract page time, the quit text rule, and the power-up visuals.
- Speed: a timed run of demo 1 takes 12.4 s, against 8.7 s for the original.
- Memory: the peak is about 60 MB, against 27 MB for the original.
- A new fuzz search on the final build.

## Method

Rustify built this game from the stripped binary, the WAD, and the observed behavior.
The agent did not read the source code, the tests, or the documentation of the original.
The first agent session stopped after 70 minutes at an evaluation process limit.
A second session resumed the same project and finished it.

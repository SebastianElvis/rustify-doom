# rustify-doom

This repository holds a Rust reconstruction of Doom.
[Rustify](https://github.com/SebastianElvis/rustify) makes it from a stripped doomgeneric binary.

## Status

The reconstruction is in progress.
The Rustify Doom track defines the reference and the scorer: [`eval/open/doom`](https://github.com/SebastianElvis/rustify/tree/main/eval/open/doom).

## Reference

- Engine: doomgeneric commit `e257a09527062bfb986b6e5e0780839b6cbd5b71` with its SDL2 front end.
- Game data: Freedoom: Phase 1, version 0.13.0.
- Build: the Rustify hardened profile with `-O3`, LTO, and `strip`. The binary has no sound.

## Layout

| Folder | Contents |
| --- | --- |
| `recovered/` | The traceability ledger, the specifications, the architecture, the evidence, and the declarative tests. |
| `implementation/` | The Rust project, its tests, the Rust design, and the decision records. |
| `verification/` | The check results, the comparisons with the original, and the parity gate result. |

## Run

The repository does not include game data.
Download `freedoom1.wad` from [Freedoom](https://freedoom.github.io/).
Give its path with `-iwad`.

## License

This project uses the GNU General Public License, version 2 or later. See [LICENSE](LICENSE).
The doomgeneric engine uses the same license.
Freedoom uses the BSD-3-Clause license.

DOOM is a registered trademark of id Software LLC, a ZeniMax Media company.
This project has no affiliation with id Software, ZeniMax Media, or Microsoft.

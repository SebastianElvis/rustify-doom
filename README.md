# rustify-doom

This repository holds a Rust reconstruction of Doom.
[Rustify](https://github.com/SebastianElvis/rustify) makes it from a stripped doomgeneric binary.

## Status

The game runs and matches the original in every recorded scenario.
It does not have full parity yet. [COMPATIBILITY.md](COMPATIBILITY.md) gives the measured result and the open gaps.
The Rustify Doom track defines the reference and the scorer: [`eval/open/doom`](https://github.com/SebastianElvis/rustify/tree/main/eval/open/doom).

## Reference

- Engine: doomgeneric commit `e257a09527062bfb986b6e5e0780839b6cbd5b71` with its SDL2 front end.
- Game data: Freedoom: Phase 1, version 0.13.0.
- Build: the Rustify hardened profile with `-O3`, LTO, and `strip`. The binary has no sound.

## Layout

| Path | Contents |
| --- | --- |
| `Cargo.toml`, `build.rs`, `src/` | The Rust game and its unit tests. |
| `docs/` | The Rust design and the decision records. |
| `COMPATIBILITY.md` | The measured parity with the original binary and the open gaps. |

The repository does not include the Rustify working files.
Rustify keeps the recovered specifications, the evidence, and the raw comparison reports in its own project folder.
The design and the decision records use requirement IDs (`REQ-`), component IDs (`C-`), and gap IDs (`GAP-`) from that folder.

## Build and run

The build needs macOS on Apple silicon, Rust, and the Homebrew `sdl2-compat` package.
The original binary links the same library, so `build.rs` uses the Homebrew path.

```sh
brew install sdl2-compat
cargo build --release
cargo test
```

The repository does not include game data.
Download `freedoom1.wad` from the [Freedoom 0.13.0 release](https://github.com/freedoom/freedoom/releases/tag/v0.13.0).
Give its path with `-iwad`:

```sh
target/release/doom -iwad freedoom1.wad
```

## License

This project uses the GNU General Public License, version 2 or later. See [LICENSE](LICENSE).
The doomgeneric engine uses the same license.
Freedoom uses the BSD-3-Clause license.

DOOM is a registered trademark of id Software LLC, a ZeniMax Media company.
This project has no affiliation with id Software, ZeniMax Media, or Microsoft.

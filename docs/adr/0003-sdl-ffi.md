# 0003. Call the same SDL2 library through a minimal foreign interface

- Status: accepted
- Date: 2026-10-28
- Items: C-002, C-010

## Context

The original imports 11 SDL2 functions from `/opt/homebrew/opt/sdl2-compat/lib/libSDL2-2.0.0.dylib` (EV-002, EV-003).
SDL2 is an open generic platform library, not a closed feature library of the product.
The replay harness interposes the same SDL functions on both programs.

## Decision

The Rust implementation declares the 11 SDL2 functions in `i_video.rs` and links the same library. It uses no Rust crates.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Direct declarations of the same SDL2 calls | Yes. The window, the texture format, the frame call order, and the event drain order stay the same. | All replay scenarios |
| The `sdl2` crate | Partly. It adds initialization calls that the original does not make. | None |

## Consequences

The harness observes the same calls on both programs.
The build needs the Homebrew SDL2 compatibility library at the same path.

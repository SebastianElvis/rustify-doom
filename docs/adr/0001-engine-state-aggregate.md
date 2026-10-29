# 0001. Keep module globals in one engine state value

- Status: accepted
- Date: 2026-10-28
- Items: C-066, all components

## Context

The original keeps the state of each component in global variables (EV-001 diagnostics name the components).
The components call each other directly and share these globals.
Safe Rust does not permit unsynchronized mutable globals without `unsafe`.

## Decision

The Rust implementation keeps one `Doom` value. Each field holds the globals of the module with the same name, and each function receives `&mut Doom`.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| One state value with one field for each module | Yes. The call order and the shared state stay the same. | Replay scenarios in `recovered/scenarios.yaml` |
| `static mut` globals | Yes, but each access needs `unsafe`. | None |
| Separate objects with message passing | No. It changes the call order. | None |

## Consequences

The module boundaries and the call order stay the same.
The state ownership moves from link-time globals to one heap value. This change has no observable effect.

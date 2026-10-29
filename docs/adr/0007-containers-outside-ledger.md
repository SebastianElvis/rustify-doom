# 0007. Keep the containers outside the component ledger

- Status: accepted
- Date: 2026-10-28
- Items: C-001, C-002, C-003, C-004, C-005

## Context

The architecture shard lists five containers: the executable, SDL2, the WAD files, the configuration files, and the savegame and demo files.
A correspondence maps one original component to one Rust source file.
The external containers have no Rust source file. The executable is the whole crate.

## Decision

The ledger lists only the components inside the executable. `recovered/architecture.md` keeps the containers in the context and container views.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Containers in the document only | Yes. The Rust crate uses the same external library and files. | Replay scenarios |
| Containers in the ledger without a correspondence | Yes, but the reflexion check reports permanent unmapped items. | None |

## Consequences

The reflexion check compares the internal components only. The external interfaces stay tested through the replay scenarios.

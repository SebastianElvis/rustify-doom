# 0004. Generate the data tables from the original binary

- Status: accepted
- Date: 2026-10-28
- Items: C-022, C-036, C-037, C-020

## Context

The original contains the sine, tangent, arctangent, gamma, and random tables, the state table, the thing table, the sprite names, and the weapon table (EV-005, EV-006).
Demo playback depends on each value.

## Decision

`verification/harness/gen_rust.py` generates `info.rs`, `data/tables.in`, and `data/weaponinfo.in` from the extracted YAML evidence.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Generate from the extracted bytes | Yes. Each value equals the original value. | `timedemo-demo1` to `timedemo-demo4` |
| Compute the tables at startup | No. Floating-point rounding can change values. | None |

## Consequences

The tables equal the original bytes. The action names are labels that the state context assigns. The action behavior comes from the Rust modules.

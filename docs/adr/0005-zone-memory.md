# 0005. Replace zone memory with Rust ownership

- Status: accepted
- Date: 2026-10-28
- Items: C-025, C-011

## Context

The original requests a 6 MiB zone (`zone memory: %p, %x allocated for zone`, 0x600000 in EV-007) and allocates game memory from it.
Rust containers own memory safely. The zone address changes in each original run.

## Decision

The Rust implementation prints the same zone report. Rust containers own the game data.
A zone byte model in `z_zone.rs` mirrors the original allocation order and block bytes (ADR 0006). The model reproduces stale-memory reads, but the game data does not live in it.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Rust containers and the same report | Partly. The visible report stays the same after the rule V-2 normalization. Zone exhaustion errors cannot occur. | `startup-explicit-iwad` |
| A zone byte model next to the Rust containers | Partly. Stale-memory reads match. Pointer high bytes and exhaustion errors stay unreproduced. | ADR 0006 |
| A byte-exact zone allocator that owns the game data | Partly. It needs unsafe raw memory and still cannot reproduce addresses. | None |

## Consequences

The model does not raise the original `Z_Malloc` exhaustion error. GAP-D02 records this loss.

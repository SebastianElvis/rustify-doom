# 0002. Keep thinkers in an index arena

- Status: accepted
- Date: 2026-10-28
- Items: C-047, C-048, C-064

## Context

The original links thinkers (map objects, doors, floors, ceilings, platforms, and lights) in one circular list of pointers.
The original frees a removed thinker during the next thinker pass. Other objects can still point at the freed block.
The savegame stores the thinker list in list order.

## Decision

The Rust implementation stores thinkers in a vector with index links. Index 0 is the list head. A removed thinker keeps its last data and its index stays unused until the next level.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| Index arena without reuse during a level | Partly. The list order and the update order stay the same. A stale reference reads the last values of the removed object. The original can read reused zone memory. | Demo replay scenarios |
| Reference-counted pointers | Partly. It changes the lifetime of removed objects. | None |

## Consequences

The update order, the spawn order, and the savegame order stay the same.
A stale reference to a reused zone block in the original can give a different value. GAP-D01 records this loss.

# 0008. Buffer standard output as the C library does

- Status: accepted
- Date: 2026-10-28
- Items: REQ-024, REQ-086, REQ-087, GAP-P10

## Context

The original writes its log with C standard I/O. A pipe makes standard output fully buffered.
When the original ends with SIGSEGV or SIGBUS, the buffered output is lost (EV-P001).
Rust standard output is line buffered.

## Decision

The Rust implementation writes standard output with `fwrite` to the C library `stdout` stream. The C library selects the buffer mode and size as in the original. The C library `exit` flushes the stream. The Rust implementation raises the same signal without a flush where the original crashes.
The `print!` and `println!` macros in `main.rs` route every print site to `i_video::print`. The `fwrite`, `signal`, and `raise` declarations stay in `i_video.rs`, the only module with `unsafe` code.

## Options

| Option | Preserves the original? | Evidence or test |
| --- | --- | --- |
| The C library `stdout` stream and the same signal at the crash points | Yes. The C library applies its own buffer rules. | `prod-loadgame-missing`, `prod-loadgame-bad`, `prod-scaling-3` |
| A Rust buffer that copies the C library rules | Partly. The harness flushes the C library stream before `_exit`, and a Rust buffer misses that flush. | `prod-testcontrols` |
| Line buffering | No. Output appears that the original loses. | The same scenarios |

## Consequences

The status and the lost output match. The crash cause is not memory corruption in the Rust implementation; the Rust code raises the signal at the recovered crash point.

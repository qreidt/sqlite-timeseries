# Spikes

[← Contents](../README.md)

Phase 0 ([Phasing](../13-phasing.md)) runs one throwaway experiment per [risk](../15-risks.md) before any storage code is written. Each spike answers one question and records a verdict. Its code is never reused directly.

## How to add a spike

1. Copy [`_template/`](_template/README.md) to `NN-short-name/` (for example `05-binding-crate/`).
2. If it needs code, run `cargo init --lib` (or `cargo init`) inside that folder, then add an empty `[workspace]` table to its `Cargo.toml`. That keeps it out of the root workspace, so throwaway code never affects the main build. The root `Cargo.toml` also excludes `docs/spikes`.
3. Fill in the README as you go, and update the table below and the matching decision record when it is settled.

## Index

| # | Risk | Status | Folder |
| --- | --- | --- | --- |
| 1 | `CREATE TABLE` from inside the virtual table's write method | Not started | — |
| 2 | `DROP TABLE` while a SELECT is running | Not started | — |
| 3 | Per-row cost of reading through the virtual table | Not started | — |
| 4 | Thousands of chunk tables slow connection open | Not started | — |
| 5 | Binding crate covers writable vtabs, window functions, vtab extras | Not started | — |
| 6 | `run_jobs()` commit and savepoint behaviour inside a statement | Not started | — |
| 7 | Bucket filter on real-time aggregates reaches the raw scan | Not started | — |
| 8 | Write-buffer flush in `xSync` with savepoints and other vtabs | Not started | — |
| 9 | Read-time merge of buffer and small blocks vs. B-tree | Not started | — |
| 10 | Block-file write protocol under crash, power loss, full disk | Not started | — |
| 11 | Users back up only the `.db` file | Not started | — |

Risks 1 and 2 gate Phase 2. Spike 5 settles [decision 0003](../decisions/0003-binding-crate.md).

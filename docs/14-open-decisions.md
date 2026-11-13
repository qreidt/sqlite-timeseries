# Open decisions

[← Contents](README.md)

Fifteen choices are still open. Each has a proposed default so work can start, and the first five should be settled before Phase 2.

| # | Decision | Options | Proposed default |
| --- | --- | --- | --- |
| 1 | Time column storage | INTEGER epoch only; INTEGER plus normalized ISO TEXT | INTEGER required, TEXT later |
| 2 | Unit for bare INTEGER input to standalone functions | seconds; milliseconds; required argument | Seconds, with a connection setting |
| 3 | Binding crate | rusqlite plus raw FFI; sqlite3_ext; sqlite-loadable | rusqlite plus raw FFI, pending spike |
| 4 | Main read path | virtual table only; virtual table plus generated `UNION ALL` view | Decide from the read-overhead spike |
| 5 | How refresh restricts the raw scan | rewrite the stored query with a SQL parser; connection-scoped range filter on the virtual table | Range filter (no parser dependency) |
| 6 | Chunk tables as shadow tables | yes, read-only to untrusted SQL in defensive mode; no, plain tables | Yes, if naming fits `xShadowName` |
| 7 | Omitted columns on insert | apply template DEFAULT when the value arrives NULL; no DEFAULT support | Decide after checking what the virtual table can see |
| 8 | Trust flags on the hypertable module | `SQLITE_VTAB_INNOCUOUS`; none | Innocuous, so real-time aggregate views work with `trusted_schema=OFF` |
| 9 | UPDATE and DELETE on compressed rows | reject; decompress affected batches | Reject in v1 |
| 10 | `run_jobs()` call shape | SQL function only; also a C/Rust API callable outside any statement | Both |
| 11 | Extension and library name | `timeseries`; other | `timeseries` (entry point `sqlite3_timeseries_init`) |
| 12 | License | MIT; Apache-2.0; dual | MIT / Apache-2.0 dual |
| 13 | Block directory name | file name minus extension (`metrics.db` to `metrics/`); file name plus a suffix (`metrics.db.ts/`) | Minus extension, refusing a folder without the marker |
| 14 | Block file granularity | one file per block; per run; per sealed chunk | Per run, compacted toward one file per chunk |
| 15 | Grace period before deleting retired files | fixed; until the next checkpoint; configurable | Configurable, default 1 hour |

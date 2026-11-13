# Testing and verification

[← Contents](README.md)

The main test is differential: run the same random workload against a hypertable and a plain table, and require identical results. Everything else protects the file format and the crash behaviour.

| ID | Requirement | Priority |
| --- | --- | --- |
| TS-01 | Property-based differential tests: random rows, then random queries (time ranges, filters, `IN`, `ORDER BY` with `LIMIT`, aggregates, joins) on a hypertable and a plain table. Results must match, including across chunk boundaries and compressed chunks. | Must |
| TS-02 | Encoding round-trip tests over edge values: NaN, ±infinity, −0.0, int64 extremes, mixed storage classes, long NULL runs, empty batches. | Must |
| TS-03 | Fuzzing of every decoder and of the interval parser. No panic, no allocation beyond a set limit. | Must |
| TS-04 | Crash tests that kill the process during chunk creation, compression, decompression and aggregate refresh, then reopen and run `PRAGMA integrity_check` plus a catalog consistency check (`ts_check()`). | Must |
| TS-05 | Multi-process tests in WAL mode: concurrent inserts and `run_jobs()` calls, checking no lost rows and no job run twice. | Must |
| TS-06 | Golden-output tests for `time_bucket`, `first`, `last`, `locf` and `interpolate`, generated once from TimescaleDB on a fixed dataset. | Must |
| TS-07 | The full suite runs in WAL and rollback-journal modes, on the minimum and latest SQLite versions. | Must |
| TS-08 | Runs with `trusted_schema=OFF` and `SQLITE_DBCONFIG_DEFENSIVE` on. | Should |
| TS-09 | A benchmark suite tracks the NF-04 to NF-07 targets and is published per release. | Should |

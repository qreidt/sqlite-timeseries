# References

[← Contents](README.md)

## SQLite

- [Implementation limits](https://sqlite.org/limits.html): compound SELECT terms, schema parsing on open, `SQLITE_LIMIT_SCHEMA`, attached database limit.
- [Release 3.38.0](https://www.sqlite.org/releaselog/3_38_0.html): `sqlite3_vtab_in`, `sqlite3_vtab_rhs_value`, LIMIT and OFFSET constraints.
- [UPSERT](https://www3.sqlite.org/lang_UPSERT.html) and [RETURNING](https://www2.sqlite.org/lang_returning.html): virtual table limitations.
- [sqlite3_unlock_notify](https://docs.w3cub.com/sqlite/c3ref/unlock_notify): the DROP TABLE exception.
- [vtab5.test](https://www.sqlite.org/src/artifact/26bc7a0a52c5c2bc): no triggers, indexes or ALTER on virtual tables.
- [DBSTAT virtual table](https://www2.sqlite.org/dbstat.html) and [loadable extensions](https://sqlite.org/loadext.html).

## TimescaleDB

- [time_bucket()](https://www.tigerdata.com/docs/reference/timescaledb/hyperfunctions/time-series-utilities/time_bucket): default origins, integer buckets, time zones.
- [Compression methods](https://www.tigerdata.com/docs/learn/columnar-storage/compression-methods): delta, delta-of-delta, simple-8b, RLE, XOR, dictionary.
- [Segmenting and ordering](https://www.tigerdata.com/docs/use-timescale/latest/hypercore/secondary-indexes): 1,000-row batches.

## Rust

- [rusqlite README via libsqlite3-sys](https://docs.rs/crate/libsqlite3-sys/latest) and [rusqlite feature list](https://docs.rs/crate/rusqlite/latest/features).
- [sqlite-loadable-rs](https://www.github.com/asg017/sqlite-loadable-rs).
- [sqlite3_ext](https://docs.rs/crate/sqlite3_ext/latest).

## Storage engine techniques

- [FTS3 xSync flush check-in](https://www.sqlite.org/src/info/af55ca5fc6778cb6) and [FTS5 incremental merge check-in](https://sqlite.org/src/info/556671444c03e3af): LSM-style buffering and merging inside SQLite.
- [lsm_vtab.c](https://www.sqlite.org/src/artifact/529255dc70428900): the bundled LSM engine and its separate file.
- [ATTACH DATABASE](https://sqlite.org/lang_attach.html): per-file atomicity in WAL mode.
- [TimescaleDB enable_chunk_skipping()](https://docs.timescale.com/api/latest/hypertable/enable_chunk_skipping) and [CREATE TABLE sparse indexes](https://www.tigerdata.com/docs/reference/timescaledb/hypertables/create_table): per-chunk ranges, bloom and minmax indexes, direct compress.

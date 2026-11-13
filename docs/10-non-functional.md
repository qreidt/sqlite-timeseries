# Non-functional requirements

[← Contents](README.md)

The overriding rule is correctness: a query on a hypertable returns exactly what the same query returns on a plain table with the same rows. Speed targets below are *proposed* and should be confirmed by the first benchmarks.

| ID | Area | Requirement | Priority |
| --- | --- | --- | --- |
| NF-01 | Correctness | For every supported query shape, hypertable results equal plain-table results with the same rows. | Must |
| NF-02 | Atomicity | Catalog changes happen in the same transaction as the data change. A crash at any point leaves catalog, chunks and compressed data consistent, in both WAL and rollback-journal modes; block files rely on the write protocol in [Block files](08-block-files.md). | Must |
| NF-03 | Cache coherence | Per-connection caches (chunk lists, prepared statements) are dropped on rollback and savepoint rollback, and refreshed when the catalog generation counter changes (another connection created or dropped a chunk). | Must |
| NF-04 | Insert speed | Batched inserts in one transaction reach at least 70% of plain-table throughput (*proposed*). | Should |
| NF-05 | Query speed | A one-day query on a one-year hypertable with daily chunks touches one chunk. Per-row read overhead is at most 2× a direct chunk scan (*proposed*). | Must / Should |
| NF-06 | Retention speed | `drop_chunks` cost depends on the number of chunks dropped, not the number of rows. | Must |
| NF-07 | Scale | Supports at least 10,000 chunks per database (*proposed*), with connection open time measured and documented, plus guidance on choosing `chunk_interval`. | Should |
| NF-08 | Compatibility | Requires SQLite 3.38.0 or later. At load time the extension checks `sqlite3_libversion_number()` and fails with a clear message on older versions. | Must |
| NF-09 | Concurrency | Works with several connections and processes on one file. Hypertables inside an ATTACHed database are a Should. | Must |
| NF-10 | Security | Functions that write or run DDL (`create_hypertable`, `drop_chunks`, `run_jobs` and similar) are registered `SQLITE_DIRECTONLY`, so a hostile schema cannot call them from a trigger or view. | Must |
| NF-11 | Robustness | No Rust panic crosses the FFI boundary: every callback is wrapped, and a panic becomes an SQLite error with a message. No unsafe global mutable state. Safe under SQLite's serialized threading mode. | Must |
| NF-12 | Errors | Messages name the function and the cause. Routing violations use `SQLITE_CONSTRAINT`; bad blobs use `SQLITE_CORRUPT`. | Must |
| NF-13 | Upgrades | The catalog has a schema version and is migrated forward by an explicit `ts_upgrade()` or on first write. A database written by a newer extension is opened read-only with a clear message. | Must |
| NF-14 | Portability | A database file and its block directory move together between operating systems and CPU architectures without conversion. | Must |
| NF-15 | Diagnostics | Optional logging through `sqlite3_log`, off by default. | Should |

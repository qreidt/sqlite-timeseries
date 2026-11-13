# Policies, jobs and introspection

[← Contents](README.md)

Policies are rows in a job table, and nothing runs until the host app calls `run_jobs()`. Every job must therefore be idempotent, bounded in how long it holds the write lock, and safe when several processes call `run_jobs()` at once.

## Policies and the job runner

| ID | Requirement | Priority |
| --- | --- | --- |
| JB-01 | `add_retention_policy(table, drop_after [, schedule])`, `add_compression_policy(table, compress_after [, schedule])` and `add_refresh_policy(cagg, start_offset, end_offset [, schedule])` create jobs. Matching `remove_*` functions and `alter_job(id, ...)` exist. | Must |
| JB-02 | The job table stores id, kind, target, config (JSON text), schedule interval, next start, enabled flag, last start and finish, last status, last error and consecutive failure count. | Must |
| JB-03 | `run_jobs([max_jobs [, time_budget_ms]])` runs due jobs in `next_start` order, stops at the budget, and returns one row or JSON entry per job run. | Must |
| JB-04 | Each job's changes are atomic (savepoint per job). A failing job records its error and does not undo jobs that already succeeded in the same call. | Must |
| JB-05 | If the caller already has a transaction open, `run_jobs()` work joins it, and the docs say so. Otherwise its work commits when the calling statement ends. | Must |
| JB-06 | Concurrent callers never run the same job twice: a job is claimed with a lease (owner, expiry) written in its own transaction. | Must |
| JB-07 | A job that is several intervals overdue runs once, then schedules its next start from now, so missed runs do not pile up. | Must |
| JB-08 | The current time comes from the host clock, with an override for tests (for example `ts_config('now', ...)`). | Must |
| JB-09 | Example host loops for Rust, Python and Node show how to call `run_jobs()` on a timer. | Should |
| JB-10 | Opt-in "piggyback" mode that checks for due jobs after every N inserts. | Could |

## Introspection

| ID | Requirement | Priority |
| --- | --- | --- |
| JB-11 | `timeseries_hypertables`: name, time column, unit, chunk interval, chunk count, estimated rows, total bytes. | Must |
| JB-12 | `timeseries_chunks`: hypertable, chunk, range start and end, status, estimated rows, bytes, compressed bytes. | Must |
| JB-13 | `timeseries_compression_stats`: bytes before and after, and ratio, per chunk and per column. | Should |
| JB-14 | `timeseries_continuous_aggregates`: name, source, bucket width, watermark, pending invalidated ranges. | Must |
| JB-15 | `timeseries_jobs`: job, kind, target, schedule, next start, last status, last error. | Must |
| JB-16 | Byte sizes come from `dbstat` when it is compiled in. Otherwise size columns are NULL and the views still work. | Must |
| JB-17 | `ts_version()` and `ts_catalog_version()` report the extension and catalog versions. | Must |
| JB-18 | `ts_explain_chunks(sql)` lists the chunks a query would scan, to debug pruning. | Should |

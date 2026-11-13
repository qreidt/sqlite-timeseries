# Query functions

[← Contents](README.md)

Query functions work on any table, hypertable or not, and must match TimescaleDB results for the same inputs. They ship first because they have no storage dependencies.

| ID | Requirement | Priority |
| --- | --- | --- |
| QF-01 | `interval(text)` parses the accepted interval forms and returns a canonical text form. Unparseable input raises an error naming the bad token. | Must |
| QF-02 | `interval_us(text)` returns integer microseconds for fixed-width intervals and errors for month-based ones. | Should |
| QF-03 | `time_bucket(width, ts [, origin])` floors `ts` to the start of its bucket, using floor (not truncation) for times before the origin. | Must |
| QF-04 | Default origin matches TimescaleDB: 2000-01-03 UTC (a Monday) for sub-month widths, 2000-01-01 for month widths ([TimescaleDB docs](https://www.tigerdata.com/docs/reference/timescaledb/hyperfunctions/time-series-utilities/time_bucket)). | Must |
| QF-05 | `time_bucket` with integer width and integer `ts` does pure integer bucketing. | Must |
| QF-06 | Month and year widths use calendar arithmetic. | Should |
| QF-07 | An optional time zone argument buckets by local calendar days, handling DST transitions. | Could |
| QF-08 | Pure functions are registered `SQLITE_DETERMINISTIC` and `SQLITE_INNOCUOUS`, so they work in expression indexes, generated columns and views with `trusted_schema=OFF`. | Must |
| QF-09 | `time_bucket_gapfill(width, start, finish)` is a table-valued function that emits one row per bucket in `[start, finish)`. Users LEFT JOIN their aggregate to it. Both bounds are required. | Must |
| QF-10 | `time_bucket_gapfill` refuses ranges above a row limit (proposed 1,000,000 buckets) to stop runaway queries. | Must |
| QF-11 | `locf(value [, prev [, treat_null_as_missing]])` is a window function that carries the last non-NULL value forward in window order. `prev` seeds the value before the first row. | Must |
| QF-12 | `interpolate(value, ts [, prev, next])` is a window function that fills NULLs by linear interpolation between the nearest non-NULL neighbours, weighted by `ts`. The required window frame is documented, and the function errors when called outside a window. | Must |
| QF-13 | `first(value, ts)` and `last(value, ts)` return the value at the earliest and latest `ts`. Rows with NULL `ts` are ignored. Ties keep the first row seen, and that order is documented as unspecified. | Must |
| QF-14 | `first` and `last` accept any storage class for `value` and compare `ts` with SQLite's normal ordering rules. | Must |
| QF-15 | `first` and `last` also work as window functions with moving frames. | Could |

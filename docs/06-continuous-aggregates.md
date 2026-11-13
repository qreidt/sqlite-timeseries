# Continuous aggregates

[← Contents](README.md)

A continuous aggregate is an ordinary materialization table plus a view, kept current by recomputing only the buckets that writes have touched. Because refresh always recomputes whole buckets, any aggregate SQLite supports can be used, with no need for mergeable partial states.

| ID | Requirement | Priority |
| --- | --- | --- |
| CA-01 | `create_continuous_aggregate(name, query [, options])` accepts a query over one hypertable with `time_bucket(<fixed width>, <time column>)` in GROUP BY, optional extra group columns and any aggregates. Joins, window functions and subqueries in FROM are rejected with a clear error in v1. | Must |
| CA-02 | Materialized rows live in an ordinary table keyed by (bucket, group columns). The name the user queries is a view. | Must |
| CA-03 | Each write through the hypertable whose time falls below the aggregate's watermark records an invalidated time range, coalesced per transaction, in the invalidation log. | Must |
| CA-04 | `refresh_continuous_aggregate(name, start, end)` recomputes only the buckets in `[start, end)` that intersect invalidated ranges or lie above the watermark. It deletes and reinserts those buckets atomically and then advances the watermark. | Must |
| CA-05 | During refresh, the raw scan is restricted by a time range on the source column, so chunk pruning applies. The mechanism is an open decision (SQL rewrite versus a connection-scoped range filter on the virtual table). | Must |
| CA-06 | Real-time mode (on by default) makes the view return materialized rows below the watermark plus a live aggregate of raw rows at or above it. `materialized_only` turns it off. | Must |
| CA-07 | A filter on the bucket column of the view reaches the raw scan as a time constraint, so real-time queries over recent data stay pruned. | Should |
| CA-08 | `drop_chunks` on the source does not invalidate or delete materialized buckets (same as LC-03). | Must |
| CA-09 | If a schema change breaks the stored query, refresh fails and the error is recorded on the job. It never silently writes partial results. | Must |
| CA-10 | `drop_continuous_aggregate(name)` removes the view, materialization table, invalidation entries and refresh jobs. | Must |
| CA-11 | Month-width buckets with time zones in continuous aggregates. | Could |
| CA-12 | Continuous aggregates built on other continuous aggregates. | Could |

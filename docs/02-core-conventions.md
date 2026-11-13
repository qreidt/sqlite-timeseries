# Core conventions

[← Contents](README.md)

Every feature depends on three shared decisions: how time is stored, how intervals are expressed, and where metadata lives. They must be settled before any feature work starts.

## Time values

- A hypertable's time column is stored as an INTEGER epoch in a unit declared at creation: `s`, `ms`, `us` or `ns`. This is the Must path.
- TEXT ISO-8601 time columns are a Should. If supported, the extension normalizes them to one UTC format on insert so that text comparison matches time order.
- Internally all chunk boundaries, bucket edges and watermarks are signed 64-bit integers in the hypertable's unit.
- Standalone functions (`time_bucket` and friends, used outside a hypertable) accept INTEGER epochs and ISO-8601 TEXT. INTEGER input without a known unit is read as seconds, matching SQLite's `unixepoch()`. A connection-level setting can change that default.
- Functions return the same storage class they receive: INTEGER in, INTEGER out; TEXT in, TEXT out.
- Time zone handling defaults to UTC. Named time zones are a Should, behind a build feature, because the time zone database adds binary size.

## Intervals

- An interval has three parts, like PostgreSQL: months, days and sub-day microseconds. Month and day parts are calendar-aware; the sub-day part is fixed width.
- Accepted text forms: PostgreSQL style (`'15 minutes'`, `'1 hour 30 minutes'`, `'2 days'`, `'1 month'`, with common abbreviations) and ISO-8601 durations (`'PT15M'`).
- An INTEGER where an interval is expected means raw units of the time column. Integer width with integer time is pure integer arithmetic, with no unit involved.
- Chunk intervals must be fixed width (no months) so chunk ranges never depend on a calendar.

## Catalog and naming

- All metadata lives in ordinary tables inside the same database file (or attached schema) as the hypertable, prefixed `_ts_`: hypertables, dimensions, chunks, chunk indexes, compression settings, continuous aggregates, invalidation log, jobs, job runs, and a meta table holding the catalog version.
- The meta table also holds a catalog generation counter, bumped on every catalog change, so connections can detect stale caches.
- Chunk tables are named `<hypertable>_chunk_<id>` and compressed chunks `<hypertable>_cchunk_<id>`. Final naming depends on whether chunks are registered as shadow tables (see [Open decisions](14-open-decisions.md)).
- User-facing function names follow TimescaleDB. Because `first`, `last` and `time_bucket` may collide with other extensions, each also gets a `ts_`-prefixed alias.

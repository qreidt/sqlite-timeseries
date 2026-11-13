# SQLite constraints that shape the design

[← Contents](README.md)

Most of the hard requirements exist because SQLite is an embedded library, not a server, and because a hypertable has to be a virtual table. These facts are fixed inputs to the design.

| SQLite fact | What it forces on the extension |
| --- | --- |
| No background process or scheduler. | Policies only run when the host app calls `run_jobs()`. Every job must be safe to run late, twice in a row, or from several processes. |
| No native timestamp or interval type; values are INTEGER, REAL, TEXT, BLOB or NULL. | The extension defines its own canonical time and interval representation (see [Core conventions](02-core-conventions.md)). |
| Virtual tables cannot have triggers, cannot be indexed and cannot be altered ([vtab5.test](https://www.sqlite.org/src/artifact/26bc7a0a52c5c2bc)). | Indexes and schema changes go through extension functions. Write tracking for continuous aggregates happens inside the virtual table's write path, not in triggers. |
| UPSERT does not work on virtual tables ([UPSERT docs](https://www3.sqlite.org/lang_UPSERT.html)). | `INSERT ... ON CONFLICT DO UPDATE` on a hypertable is unsupported and must be documented, with a workaround. |
| RETURNING is not available on UPDATE and DELETE against virtual tables ([RETURNING docs](https://www2.sqlite.org/lang_returning.html)). | Only `INSERT ... RETURNING` can be promised on hypertables. |
| DROP TABLE and DROP INDEX return `SQLITE_LOCKED` while any SELECT on the same connection is still running ([unlock_notify docs](https://docs.w3cub.com/sqlite/c3ref/unlock_notify)). | Dropping or compressing chunks from inside a SQL function call can fail. Chunk removal needs a two-phase design (see [Risks](15-risks.md)). |
| The planner does not push aggregates or joins into a virtual table; every row crosses the virtual table interface. | Read overhead per row must be measured. A generated `UNION ALL` view is a candidate alternative read path. |
| A compound SELECT is limited to 500 terms by default ([limits](https://sqlite.org/limits.html)). | A generated `UNION ALL` view cannot list more than 500 chunks without a custom build. |
| Each connection parses the whole schema on open, so startup time and memory grow with it. Every chunk adds a table plus its indexes. | Chunk count affects connection open time and memory, and since 3.54.0 a host can cap schema objects with `SQLITE_LIMIT_SCHEMA`. The extension must give chunk-sizing guidance and a supported maximum. |
| A virtual table only opens on connections that loaded its module. | Every connection that touches a hypertable must load the extension. Uncompressed chunk tables stay readable without it. |
| `dbstat` exists only when SQLite is built with `SQLITE_ENABLE_DBSTAT_VTAB` ([dbstat](https://www2.sqlite.org/dbstat.html)). | Size reporting needs a fallback when `dbstat` is missing. |
| `IN`, `LIMIT` and `OFFSET` handling inside `xBestIndex` (`sqlite3_vtab_in`, `sqlite3_vtab_rhs_value`) arrived in 3.38.0 ([release notes](https://www.sqlite.org/releaselog/3_38_0.html)). | Minimum supported SQLite version is 3.38.0. |
| One writer per database file at a time. | Insert batching and job work must keep write transactions short. |

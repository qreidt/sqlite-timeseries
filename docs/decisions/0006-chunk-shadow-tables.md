# 0006. Chunk tables as shadow tables

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-145](https://linear.app/cqr/issue/CQR-145)

## Context

Chunk tables are ordinary tables that users could write to directly, bypassing routing and catalog statistics.

## Options

- Register chunks as shadow tables, read-only to untrusted SQL in defensive mode.
- Keep them as plain tables guarded only by CHECK constraints.

## Decision

Shadow tables, if the chunk naming fits `xShadowName`.

## Consequences

Chunk names must follow `<hypertable>_<suffix>`. Direct writes fail when `SQLITE_DBCONFIG_DEFENSIVE` is on.

## Links

- Requirements: HT-05, NF-10
- [Open decisions](../14-open-decisions.md)

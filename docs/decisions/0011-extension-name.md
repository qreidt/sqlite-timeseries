# 0011. Extension and library name

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-136](https://linear.app/cqr/issue/CQR-136)

## Context

SQLite derives the default entry point from the library file name.

## Options

- `timeseries`.
- Another name.

## Decision

`timeseries`, giving `libtimeseries.so` and the entry point `sqlite3_timeseries_init`.

## Consequences

Function aliases use the `ts_` prefix.

## Links

- Requirements: BD-02
- [Open decisions](../14-open-decisions.md)

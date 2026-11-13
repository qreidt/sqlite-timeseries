# 0005. How refresh restricts the raw scan

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-135](https://linear.app/cqr/issue/CQR-135)

## Context

Continuous aggregate refresh must scan only the invalidated time range, but the stored query groups by `time_bucket(...)`, which is not a constraint SQLite passes to the virtual table.

## Options

- Rewrite the stored query with a SQL parser.
- A connection-scoped range filter on the virtual table, set during refresh.

## Decision

Connection-scoped range filter, which needs no parser dependency. Spike 7 checks real-time view pushdown on the same path.

## Consequences

The range filter must be cleared on every exit path, including errors.

## Links

- Requirements: CA-05, CA-07, risk 7
- [Open decisions](../14-open-decisions.md)

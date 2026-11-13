# 0010. `run_jobs()` call shape

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-124](https://linear.app/cqr/issue/CQR-124)

## Context

Work done from inside a SQL statement commits only when that statement ends, and dropping tables there can fail with `SQLITE_LOCKED`.

## Options

- SQL function only.
- SQL function plus a C/Rust API callable outside any statement.

## Decision

Both, pending spike 6.

## Consequences

Host examples show the API path as the recommended one if spike 6 confirms the problems.

## Links

- Requirements: JB-03, JB-05, risk 2, risk 6
- [Open decisions](../14-open-decisions.md)

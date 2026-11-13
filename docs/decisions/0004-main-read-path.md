# 0004. Main read path

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-134](https://linear.app/cqr/issue/CQR-134)

## Context

Every row returned through a virtual table crosses its interface, and SQLite never pushes aggregates into it, so large scans may be slow.

## Options

- Virtual table only.
- Virtual table plus a generated `UNION ALL` view over chunks (limited to 500 terms by default).

## Decision

Decide from the read-overhead measurements in spike 3.

## Consequences

If the view wins, heavy analytic queries use it and the 500-chunk limit must be documented.

## Links

- Requirements: HT-26, NF-05, risk 3
- [Open decisions](../14-open-decisions.md)

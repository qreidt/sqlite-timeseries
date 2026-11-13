# 0008. Trust flags on the hypertable module

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-154](https://linear.app/cqr/issue/CQR-154)

## Context

With `trusted_schema=OFF`, views can only use virtual tables marked innocuous. Real-time continuous aggregate views read the hypertable.

## Options

- Mark the module `SQLITE_VTAB_INNOCUOUS`.
- No trust flag.

## Decision

Innocuous, so real-time aggregate views keep working with `trusted_schema=OFF`.

## Consequences

Functions that write or run DDL stay `SQLITE_DIRECTONLY` regardless.

## Links

- Requirements: NF-10, CA-06
- [Open decisions](../14-open-decisions.md)

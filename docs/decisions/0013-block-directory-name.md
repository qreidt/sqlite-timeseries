# 0013. Block directory name

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-171](https://linear.app/cqr/issue/CQR-171)

## Context

Sealed blocks live in a directory beside the database. A folder cannot share the database file's exact name.

## Options

- File name minus extension (`metrics.db` to `metrics/`).
- File name plus a suffix (`metrics.db.ts/`).

## Decision

File name minus extension, refusing any folder without the `.timeseries` marker.

## Consequences

A database file with no extension has no free name and must be refused or use the suffix form.

## Links

- Requirements: BF-01, BF-02
- [Open decisions](../14-open-decisions.md)

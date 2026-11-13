# 0015. Grace period before deleting retired files

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-174](https://linear.app/cqr/issue/CQR-174)

## Context

Readers in other processes may still be reading an older snapshot that references a retired file.

## Options

- Fixed period.
- Until the next WAL checkpoint.
- Configurable period.

## Decision

Configurable, default 1 hour.

## Consequences

Disk space from compaction and retention is freed up to an hour later.

## Links

- Requirements: BF-08
- [Open decisions](../14-open-decisions.md)

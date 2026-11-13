# 0009. UPDATE and DELETE on compressed rows

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-173](https://linear.app/cqr/issue/CQR-173)

## Context

Changing rows inside immutable compressed blocks means decompressing, tombstoning or refusing.

## Options

- Reject with a clear error.
- Decompress the affected batches.
- Tombstones applied at read time (SE-10).

## Decision

Reject in v1.

## Consequences

Users must decompress a chunk before changing its rows until tombstones exist.

## Links

- Requirements: LC-14, SE-10
- [Open decisions](../14-open-decisions.md)

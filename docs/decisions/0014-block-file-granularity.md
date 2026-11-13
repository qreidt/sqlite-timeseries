# 0014. Block file granularity

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-177](https://linear.app/cqr/issue/CQR-177)

## Context

More files mean cheaper rewrites but more file handles and directory entries.

## Options

- One file per block.
- One file per run.
- One file per sealed chunk.

## Decision

One file per run, compacted toward one file per chunk.

## Consequences

File count stays close to chunk count after compaction.

## Links

- Requirements: BF-15, SE-09
- [Open decisions](../14-open-decisions.md)

# 0001. Time column storage

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-131](https://linear.app/cqr/issue/CQR-131)

## Context

A hypertable's time column needs one canonical storage form so chunk boundaries, pruning and bucketing compare correctly. SQLite has no timestamp type.

## Options

- INTEGER epoch only, in a unit declared at creation (`s`, `ms`, `us`, `ns`).
- INTEGER epoch plus normalized ISO-8601 TEXT columns.

## Decision

INTEGER epoch is required in v1. Normalized ISO-8601 TEXT is a later addition.

## Consequences

All chunk boundaries, bucket edges and watermarks are signed 64-bit integers. TEXT support, if added, must normalize to one UTC format on insert.

## Links

- Requirements: Core conventions, HT-01, HT-04
- [Open decisions](../14-open-decisions.md)

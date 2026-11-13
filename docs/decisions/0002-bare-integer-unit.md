# 0002. Unit for bare INTEGER input to standalone functions

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-132](https://linear.app/cqr/issue/CQR-132)

## Context

Standalone functions such as `time_bucket` receive INTEGER timestamps with no hypertable to say what unit they are in.

## Options

- Seconds.
- Milliseconds.
- A required unit argument.

## Decision

Seconds, matching SQLite's `unixepoch()`, with a connection-level setting to change the default.

## Consequences

Callers using milliseconds must set the connection default or pass integer widths in their own unit.

## Links

- Requirements: Core conventions, QF-03, QF-05
- [Open decisions](../14-open-decisions.md)

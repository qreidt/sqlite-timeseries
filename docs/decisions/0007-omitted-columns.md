# 0007. Omitted columns on insert

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-146](https://linear.app/cqr/issue/CQR-146)

## Context

A virtual table's write method may not be able to tell an omitted column from an explicit NULL, so template DEFAULT values may not apply as users expect.

## Options

- Apply the template DEFAULT when the value arrives NULL.
- Do not support DEFAULT on hypertables.

## Decision

Decide after checking what the virtual table can see during insert.

## Consequences

Whichever is chosen must be documented next to HT-15.

## Links

- Requirements: HT-15
- [Open decisions](../14-open-decisions.md)

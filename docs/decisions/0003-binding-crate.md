# 0003. Binding crate

- Status: Proposed
- Date: 2026-11-13
- Settled by: [CQR-133](https://linear.app/cqr/issue/CQR-133)

## Context

The extension needs writable virtual tables, window functions, `sqlite3_vtab_in`, `sqlite3_vtab_rhs_value`, `xShadowName` and savepoint hooks from Rust. rusqlite's README says its virtual table support is read-only.

## Options

- rusqlite plus raw FFI through `libsqlite3-sys` for the gaps.
- sqlite3_ext.
- sqlite-loadable (beta, unstable).

## Decision

rusqlite plus raw FFI behind one internal module, pending spike 5.

## Consequences

One internal FFI module owns every unsafe call the wrapper does not cover.

## Links

- Requirements: BD-01, risk 5
- [Open decisions](../14-open-decisions.md)

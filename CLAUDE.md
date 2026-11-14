# CLAUDE.md

SQLite loadable extension in Rust that adds TimescaleDB-style time-series storage. **Scaffold stage:** the entry point loads and registers nothing.

## Working with the user

The user is new to Rust. Explain Rust-specific concepts and tooling briefly when they come up, prefer simple and idiomatic code over clever code, and ask before making structural choices (ask several rounds of questions if needed).

## Layout

- `Cargo.toml`: root workspace with shared lints and profiles. Members: `crates/timeseries`.
- `crates/timeseries/`: the only crate (cdylib + rlib).
  - `src/lib.rs`: feature checks and `register(&Connection)`, where all SQL functions and modules get registered.
  - `src/ffi/`: the **only** place allowed to use `unsafe` (BD-01). It holds the entry point `sqlite3_timeseries_init`.
- `docs/`: requirements (numbered files), `decisions/` (ADRs, never rewritten, superseded instead), `spikes/` (Phase 0 experiments, each its own standalone Cargo project, excluded from the workspace).
- `benchmark/`: a separate Cargo project (`benchmark/harness` has its own `[workspace]`). It is not part of the root workspace.

## Commands

- Build the loadable .so: `cargo build`
- Static build: `cargo build --no-default-features --features static`
- Load smoke test: `sqlite3 :memory: ".load target/debug/libtimeseries" "select 1;"`
- Before committing, run `cargo fmt --check` and `cargo clippy --all-targets`, for both feature sets.

## Conventions

- Features `loadable` (default) and `static` are mutually exclusive (rusqlite's `loadable_extension` and `bundled` can't combine). A `compile_error!` enforces this.
- Binding: rusqlite 0.41 + libsqlite3-sys (decision 0003, pending spike 5). Keep libsqlite3-sys on the version rusqlite uses.
- No panic may cross into SQLite (NF-11). Avoid `unwrap()`, and use `// SAFETY:` comments on every unsafe block.
- Requirement IDs (QF-, HT-, LC-, CA-, SE-, BF-, JB-, NF-, BD-, TS-) are cited in comments where code implements them.
- User-facing SQL names follow TimescaleDB, with `ts_` aliases. Catalog tables are prefixed `_ts_`.
- Licensing is undecided (decision 0012). Don't add license files or `license` fields yet.
- Work is tracked in Linear (team CQR, project "SQLite Time-Series Extension").

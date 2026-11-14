//! `timeseries`: a SQLite extension that brings TimescaleDB-style time-series
//! storage (hypertables, retention, compression, continuous aggregates) to SQLite.
//!
//! The requirements live in `docs/` at the repository root; start with
//! `docs/README.md`. Nothing is implemented yet: this crate is a scaffold.
//!
//! # Build modes
//!
//! Exactly one of these Cargo features must be enabled:
//!
//! - `loadable` (default): builds `libtimeseries.so` (or `.dylib` / `.dll`), which
//!   SQLite loads at run time with `.load` or `sqlite3_load_extension`. The entry
//!   point is `sqlite3_timeseries_init` (BD-02).
//! - `static`: links the extension and a bundled SQLite into a Rust or C
//!   application, which calls [`register`] on each connection (BD-03).

#[cfg(all(feature = "loadable", feature = "static"))]
compile_error!(
    "features `loadable` and `static` are mutually exclusive; \
     use `--no-default-features --features static` for the static build"
);

#[cfg(not(any(feature = "loadable", feature = "static")))]
compile_error!("enable exactly one of the features `loadable` (default) or `static`");

mod ffi;

/// Registers the extension's SQL functions and virtual table modules on `conn`.
///
/// It does nothing yet. This is where the SQLite version check (NF-08) and
/// every function and module registration will go.
pub fn register(_conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    Ok(())
}

//! Raw FFI boundary with SQLite.
//!
//! This is the **only** module allowed to contain `unsafe` code (BD-01). Anything
//! the rusqlite wrapper does not expose (writable virtual tables, `sqlite3_vtab_in`,
//! `xShadowName`, savepoint hooks, ...) is called through `libsqlite3-sys` here and
//! wrapped in a safe API for the rest of the crate.
//!
//! Rules for code in this module:
//! - Every `unsafe {}` block has a `// SAFETY:` comment.
//! - No Rust panic may cross into SQLite (NF-11): callbacks catch panics and turn
//!   them into SQLite errors.

#[cfg(feature = "loadable")]
use std::ffi::{c_char, c_int};

#[cfg(feature = "loadable")]
use rusqlite::{Connection, ffi};

/// Entry point SQLite calls when the library is loaded (BD-02).
///
/// SQLite derives this name from the file name `libtimeseries.so`, so it must not change
/// unless the library is renamed (decision 0011).
///
/// # Safety
///
/// Only SQLite may call this, with a valid connection handle and API routines
/// pointer, as described in <https://sqlite.org/loadext.html>.
#[cfg(feature = "loadable")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sqlite3_timeseries_init(
    db: *mut ffi::sqlite3,
    pz_err_msg: *mut *mut c_char,
    p_api: *mut ffi::sqlite3_api_routines,
) -> c_int {
    // SAFETY: SQLite passes a valid `db`, `pz_err_msg` and `p_api` to the entry point,
    // and `init` only registers features, as `extension_init2` requires.
    unsafe { Connection::extension_init2(db, pz_err_msg, p_api, init) }
}

/// Runs once per connection that loads the extension.
///
/// It returns `Ok(false)`, so the extension is not loaded permanently. It applies only to the
/// connection that loaded it.
#[cfg(feature = "loadable")]
fn init(conn: Connection) -> rusqlite::Result<bool> {
    crate::register(&conn)?;
    Ok(false)
}

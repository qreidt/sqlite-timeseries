# Rust build, packaging and distribution

[← Contents](README.md)

rusqlite is the likely base, with raw FFI through `libsqlite3-sys` for anything its wrappers do not expose. A spike must confirm it, because rusqlite's README says its virtual table support is read-only, and a hypertable needs a writable one.

| Crate | Status (as of Oct 2026) | Fit for this project |
| --- | --- | --- |
| [rusqlite](https://docs.rs/crate/libsqlite3-sys/latest) | Actively maintained; 0.40.2 bundles SQLite 3.53.2. The `loadable_extension` feature exists since 0.30.0 ([sqlite-loadable-rs README](https://www.github.com/asg017/sqlite-loadable-rs)). | Mature `functions` and `window` support ([feature list](https://docs.rs/crate/rusqlite/latest/features)). The README says `vtab` currently supports only read-only virtual tables, so the hypertable write path may need raw FFI. |
| [sqlite-loadable](https://www.github.com/asg017/sqlite-loadable-rs) | 0.0.5. The author labels it beta and unstable, is unsure it works in serialized threading mode, and lists window functions as not yet supported. | Purpose-built for loadable extensions and has a writable virtual table API (marked as likely to change), but the stability caveats are a poor match for a storage engine. |
| [sqlite3_ext](https://docs.rs/crate/sqlite3_ext/latest) | Pre-1.0. Claims all virtual table methods and SQLite features up to 3.38.5. | Worth testing if rusqlite cannot cover writes, `sqlite3_vtab_in`, savepoint hooks or shadow tables. |

## Requirements

| ID | Requirement | Priority |
| --- | --- | --- |
| BD-01 | Pick the binding crate after the spike in [Risks](15-risks.md). Whatever is missing from the wrapper (writable virtual tables, window functions, `sqlite3_vtab_in`, `sqlite3_vtab_rhs_value`, `xShadowName`, savepoints) goes through raw FFI behind one internal module. | Must |
| BD-02 | Build a `cdylib` whose file name yields the intended entry point. SQLite drops a leading `lib` and keeps only the lowercase ASCII letters before the first dot ([loadext docs](https://sqlite.org/loadext.html)), so `libtimeseries.so` maps to `sqlite3_timeseries_init`. The entry point is exported under that exact name. | Must |
| BD-03 | A `static` feature lets Rust and C apps link the extension and register it per connection or through `sqlite3_auto_extension`. | Must |
| BD-04 | Prebuilt binaries for Linux x86_64 and aarch64 (glibc and musl), macOS x86_64 and arm64, and Windows x86_64. | Must |
| BD-05 | Packages for the main ecosystems (crates.io, PyPI, npm) that ship the binary and a `load()` helper. | Should |
| BD-06 | Optional pieces sit behind Cargo features: time zone database, zstd codec, static linking. The default build stays small. | Must |
| BD-07 | The docs explain that extension loading must be enabled by the host (for example `enable_load_extension` in Python) and that some platform SQLite builds disable it, with static linking or a bundled SQLite as the fallback. | Must |
| BD-08 | CI tests against the minimum SQLite version (3.38.x) and the latest release. | Must |
| BD-09 | A WebAssembly build for browser SQLite. | Could |

# timeseries

A SQLite loadable extension, written in Rust, that brings TimescaleDB-style time-series storage to SQLite: automatic time partitioning (hypertables), cheap retention, columnar compression and continuous aggregates.

**Status: scaffold only.** The extension builds and loads but does nothing yet. The requirements are in [`docs/`](docs/README.md), and work starts with the Phase 0 [spikes](docs/spikes/README.md).

## Repository layout

| Path | What it is |
| --- | --- |
| [`crates/timeseries/`](crates/timeseries/) | The extension crate (builds `libtimeseries.so` / `.dylib` / `.dll`) |
| [`docs/`](docs/README.md) | Requirements, decision records and spikes |
| [`benchmark/`](benchmark/README.md) | TSBS-based benchmark against plain SQLite and TimescaleDB (a separate Cargo project) |

## Build

Install Rust with [rustup](https://rustup.rs). The repository's `rust-toolchain.toml` selects the stable toolchain automatically.

```bash
cargo build              # debug build: target/debug/libtimeseries.so
```

```bash
cargo build --release    # optimized build: target/release/libtimeseries.so
```

```bash
cargo build --no-default-features --features static   # static build (BD-03), for linking into an app
```

## Load it into SQLite

```bash
sqlite3 :memory: ".load target/debug/libtimeseries" "select sqlite_version();"
```

SQLite derives the entry point `sqlite3_timeseries_init` from the file name. The host must allow extension loading: in Python, call `conn.enable_load_extension(True)` first. Some platform SQLite builds disable loading entirely, and in that case use the static build (BD-07). SQLite 3.38.0 or later is required (NF-08).

## Development

```bash
cargo fmt                       # format code
```

```bash
cargo clippy --all-targets      # lint
```

```bash
cargo test                      # run tests
```

//! `tsbench`: benchmark harness for the SQLite time-series extension.
//!
//! Subcommands:
//! - `convert`              TSBS output -> CSV + Parquet + manifest.json (engine-neutral dataset)
//! - `load`                 load the CSV dataset into a SQLite engine and measure ingest
//! - `size`                 measure on-disk size of a SQLite engine's database
//! - `compress`             run an engine's compression SQL, then measure size again
//! - `query`                run the TSBS devops query set against a SQLite engine
//! - `list-queries`         print the TSBS query types that fit a dataset
//! - `prepare-timescaledb`  write TimescaleDB schema/compression/size SQL for a dataset
//! - `report`               combine all result files into a markdown report

mod compress;
mod convert;
mod engine;
mod load;
mod manifest;
mod queries;
mod query;
mod report;
mod results;
mod rng;
mod size;
mod timefmt;
mod timescaledb;
mod tsbs;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tsbench",
    version,
    about = "Time-series benchmark harness (TSBS data, SQLite engines)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Convert TSBS `timescaledb`-format output into CSV + Parquet files and a manifest.
    Convert(convert::Args),
    /// Load a converted dataset into a SQLite engine and measure ingest speed.
    Load(load::Args),
    /// Measure the on-disk size of a SQLite engine's database.
    Size(size::Args),
    /// Run an engine's compression SQL, then measure size again.
    Compress(compress::Args),
    /// Run the TSBS devops query set against a SQLite engine.
    Query(query::Args),
    /// Print the TSBS query types that fit a dataset (one per line).
    ListQueries(queries::ListArgs),
    /// Write TimescaleDB schema, compression and size SQL for a dataset.
    PrepareTimescaledb(timescaledb::Args),
    /// Combine result files into a markdown report.
    Report(report::Args),
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Convert(a) => convert::run(a),
        Cmd::Load(a) => load::run(a),
        Cmd::Size(a) => size::run(a),
        Cmd::Compress(a) => compress::run(a),
        Cmd::Query(a) => query::run(a),
        Cmd::ListQueries(a) => queries::list(a),
        Cmd::PrepareTimescaledb(a) => timescaledb::run(a),
        Cmd::Report(a) => report::run(a),
    }
}

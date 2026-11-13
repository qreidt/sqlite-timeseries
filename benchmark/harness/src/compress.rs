//! `tsbench compress`: run the engine's `compress` SQL for every measurement table, then measure
//! size again (phase `compressed`). Plain SQLite engines have no compression step.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

use crate::engine::{self, Engine, EngineArgs};
use crate::manifest::Manifest;
use crate::{results, size};

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub engine: EngineArgs,
    #[arg(long)]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub db: PathBuf,
    #[arg(long)]
    pub results_dir: Option<PathBuf>,
    /// Exit successfully (doing nothing) when the engine has no `compress` SQL.
    #[arg(long)]
    pub if_supported: bool,
}

#[derive(Serialize)]
struct CompressResult {
    kind: &'static str,
    engine: String,
    dataset: String,
    created_at: String,
    seconds: f64,
    rows: u64,
    rows_per_sec: f64,
}

pub fn run(a: Args) -> Result<()> {
    let m = Manifest::load(&a.data_dir)?;
    let eng = Engine::from_args(&a.engine)?;
    if eng.compress.trim().is_empty() {
        if a.if_supported {
            eprintln!("engine {} has no compression step; skipping", eng.name);
            return Ok(());
        }
        bail!(
            "engine {} has no `compress` SQL, so there is nothing to compress",
            eng.name
        );
    }
    let conn = eng.open(&a.db)?;
    let t0 = Instant::now();
    for t in &m.tables {
        conn.execute_batch(&eng.render(&eng.compress, Some(t)))
            .with_context(|| format!("compress failed for {}", t.name))?;
    }
    engine::checkpoint(&conn)?;
    let seconds = t0.elapsed().as_secs_f64();
    let rows = m.total_rows();
    let res = CompressResult {
        kind: "compress",
        engine: eng.name.clone(),
        dataset: m.dataset.clone(),
        created_at: results::now_rfc3339(),
        seconds,
        rows,
        rows_per_sec: rows as f64 / seconds,
    };
    eprintln!(
        "compressed in {seconds:.2}s ({:.0} rows/s)",
        res.rows_per_sec
    );
    results::write(
        a.results_dir.as_deref(),
        &format!("{}.compress.json", eng.name),
        &res,
    )?;

    let sz = size::measure(&conn, &eng, &a.db, &m, "compressed")?;
    size::print(&sz);
    results::write(
        a.results_dir.as_deref(),
        &format!("{}.size-compressed.json", eng.name),
        &sz,
    )?;
    Ok(())
}

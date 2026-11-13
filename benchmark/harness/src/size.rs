//! `tsbench size`: on-disk size of a SQLite engine's database.
//!
//! `total_bytes` is what the dataset costs on disk: the database file, any WAL left after a
//! checkpoint, and (for block-file engines) every file in the block directory. When SQLite has
//! `dbstat`, the result also breaks the database file down by measurement table (including its
//! indexes and chunk tables), tags, the extension's `_ts_` catalog and free pages.

use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::engine::{self, Engine, EngineArgs};
use crate::manifest::Manifest;
use crate::results;

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub engine: EngineArgs,
    #[arg(long)]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub db: PathBuf,
    /// Label for this measurement: raw (after load) or compressed (after `compress`).
    #[arg(long, default_value = "raw")]
    pub phase: String,
    #[arg(long)]
    pub results_dir: Option<PathBuf>,
}

#[derive(Serialize)]
pub struct SizeResult {
    pub kind: &'static str,
    pub engine: String,
    pub dataset: String,
    pub phase: String,
    pub created_at: String,
    pub total_bytes: u64,
    pub db_file_bytes: u64,
    pub wal_bytes: u64,
    pub block_dir_bytes: u64,
    pub block_files: u64,
    pub free_bytes: u64,
    pub page_size: u64,
    /// Bytes per object group, from dbstat (empty when dbstat is not compiled in).
    pub objects: BTreeMap<String, u64>,
    pub rows: u64,
    pub metrics: u64,
    pub bytes_per_row: f64,
    pub bytes_per_metric: f64,
}

pub fn run(a: Args) -> Result<()> {
    let m = Manifest::load(&a.data_dir)?;
    let eng = Engine::from_args(&a.engine)?;
    let conn = eng.open(&a.db)?;
    let res = measure(&conn, &eng, &a.db, &m, &a.phase)?;
    print(&res);
    results::write(
        a.results_dir.as_deref(),
        &format!("{}.size-{}.json", eng.name, a.phase),
        &res,
    )?;
    Ok(())
}

pub fn print(r: &SizeResult) {
    eprintln!(
        "{} [{}]: {} total ({:.2} bytes/metric, {:.2} bytes/row)",
        r.engine,
        r.phase,
        crate::convert::human_bytes(r.total_bytes),
        r.bytes_per_metric,
        r.bytes_per_row
    );
    for (k, v) in &r.objects {
        eprintln!("    {k:<16} {}", crate::convert::human_bytes(*v));
    }
}

pub fn measure(
    conn: &Connection,
    eng: &Engine,
    db: &Path,
    m: &Manifest,
    phase: &str,
) -> Result<SizeResult> {
    engine::checkpoint(conn)?;
    let page_size = conn.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))? as u64;
    let freelist = conn.query_row("PRAGMA freelist_count", [], |r| r.get::<_, i64>(0))? as u64;
    let objects = dbstat_groups(conn, m).unwrap_or_default();

    let db_file_bytes = std::fs::metadata(db).map(|x| x.len()).unwrap_or(0);
    let wal_bytes = std::fs::metadata(format!("{}-wal", db.display()))
        .map(|x| x.len())
        .unwrap_or(0);
    let (block_dir_bytes, block_files) = if eng.block_dir {
        dir_size(&Engine::block_dir_for(db))
    } else {
        (0, 0)
    };
    let total_bytes = db_file_bytes + wal_bytes + block_dir_bytes;
    let rows = m.total_rows();
    let metrics = m.total_metrics();
    Ok(SizeResult {
        kind: "size",
        engine: eng.name.clone(),
        dataset: m.dataset.clone(),
        phase: phase.to_string(),
        created_at: results::now_rfc3339(),
        total_bytes,
        db_file_bytes,
        wal_bytes,
        block_dir_bytes,
        block_files,
        free_bytes: freelist * page_size,
        page_size,
        objects,
        rows,
        metrics,
        bytes_per_row: total_bytes as f64 / rows.max(1) as f64,
        bytes_per_metric: total_bytes as f64 / metrics.max(1) as f64,
    })
}

/// Group dbstat page usage: measurement tables (with their indexes and `<table>_*` chunk tables),
/// `tags`, the extension catalog (`_ts_*`), and everything else.
fn dbstat_groups(conn: &Connection, m: &Manifest) -> Result<BTreeMap<String, u64>> {
    let mut owner: BTreeMap<String, String> = BTreeMap::new();
    {
        let mut st = conn.prepare("SELECT name, tbl_name FROM sqlite_schema")?;
        let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (name, tbl) = row?;
            owner.insert(name, tbl);
        }
    }
    let mut groups: BTreeMap<String, u64> = BTreeMap::new();
    let mut st = conn.prepare("SELECT name, SUM(pgsize) FROM dbstat GROUP BY name")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    for row in rows {
        let (name, bytes) = row?;
        let tbl = owner.get(&name).cloned().unwrap_or_else(|| name.clone());
        let group = if let Some(t) = m
            .tables
            .iter()
            .find(|t| tbl == t.name || tbl.starts_with(&format!("{}_", t.name)))
        {
            t.name.clone()
        } else if tbl == "tags" {
            "tags".into()
        } else if tbl.starts_with("_ts_") {
            "catalog".into()
        } else if tbl.starts_with("sqlite_") {
            "sqlite_internal".into()
        } else {
            "other".into()
        };
        *groups.entry(group).or_default() += bytes as u64;
    }
    Ok(groups)
}

fn dir_size(dir: &Path) -> (u64, u64) {
    let mut bytes = 0;
    let mut files = 0;
    let Ok(rd) = std::fs::read_dir(dir) else {
        return (0, 0);
    };
    for e in rd.flatten() {
        let Ok(md) = e.metadata() else { continue };
        if md.is_dir() {
            let (b, f) = dir_size(&e.path());
            bytes += b;
            files += f;
        } else {
            bytes += md.len();
            files += 1;
        }
    }
    (bytes, files)
}

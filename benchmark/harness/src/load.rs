//! `tsbench load`: load a converted dataset into a SQLite engine and measure ingest speed.
//!
//! One thread reads and parses the CSV; the main thread inserts with a prepared statement,
//! one transaction per batch (TSBS's default batch is 10,000 rows). The reported time covers the
//! measurement tables only, from the first batch to the last commit; the tags table is loaded
//! first and timed separately. `parser_wait_seconds` shows how long the writer waited for the
//! parser: if it is a large share of the total, the parser, not SQLite, was the bottleneck.

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::mpsc::sync_channel;
use std::time::{Duration, Instant};

use crate::engine::{self, Engine, EngineArgs, TimeUnit};
use crate::manifest::{Manifest, TableInfo};
use crate::results::{self, Host};
use crate::timefmt;

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub engine: EngineArgs,
    /// Dataset directory written by `tsbench convert`.
    #[arg(long)]
    pub data_dir: PathBuf,
    /// SQLite database file to create (an existing one is replaced).
    #[arg(long)]
    pub db: PathBuf,
    /// Rows per transaction.
    #[arg(long, default_value_t = 10_000)]
    pub batch_size: usize,
    /// Load only these measurement tables (default: all).
    #[arg(long, value_delimiter = ',')]
    pub tables: Vec<String>,
    /// Directory for the result JSON file.
    #[arg(long)]
    pub results_dir: Option<PathBuf>,
}

#[derive(Serialize)]
struct TableResult {
    table: String,
    rows: u64,
    metrics: u64,
    seconds: f64,
    rows_per_sec: f64,
    metrics_per_sec: f64,
    parser_wait_seconds: f64,
}

#[derive(Serialize)]
struct LoadResult {
    kind: &'static str,
    engine: String,
    dataset: String,
    created_at: String,
    rows: u64,
    metrics: u64,
    seconds: f64,
    rows_per_sec: f64,
    metrics_per_sec: f64,
    workers: u32,
    batch_size: usize,
    tags_rows: u64,
    tags_seconds: f64,
    post_load_seconds: f64,
    parser_wait_seconds: f64,
    tables: Vec<TableResult>,
    time_unit: &'static str,
    pragmas: Vec<String>,
    extension: Option<String>,
    sqlite_version: String,
    harness: String,
    host: Host,
}

struct Batch {
    times: Vec<i64>,
    tags: Vec<i64>,
    values: Vec<Option<f64>>,
}

impl Batch {
    fn with_capacity(rows: usize, nfields: usize) -> Self {
        Batch {
            times: Vec::with_capacity(rows),
            tags: Vec::with_capacity(rows),
            values: Vec::with_capacity(rows * nfields),
        }
    }
}

pub fn run(a: Args) -> Result<()> {
    if a.batch_size == 0 {
        bail!("--batch-size must be at least 1");
    }
    let m = Manifest::load(&a.data_dir)?;
    let eng = Engine::from_args(&a.engine)?;
    remove_db(&a.db, eng.block_dir)?;
    if let Some(parent) = a.db.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = eng.open(&a.db)?;
    eprintln!(
        "engine {} ({}) -> {}",
        eng.name,
        if eng.description.is_empty() {
            "no description"
        } else {
            &eng.description
        },
        a.db.display()
    );

    let t0 = Instant::now();
    load_tags(&mut conn, &m, &a.data_dir)?;
    let tags_seconds = t0.elapsed().as_secs_f64();

    let selected: Vec<&TableInfo> = m
        .tables
        .iter()
        .filter(|t| a.tables.is_empty() || a.tables.contains(&t.name))
        .collect();
    if selected.is_empty() {
        bail!(
            "no tables selected (dataset has: {})",
            m.tables
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let mut table_results = Vec::new();
    for t in &selected {
        conn.execute_batch(&eng.render(&eng.create_table, Some(t)))
            .with_context(|| format!("create_table failed for {}", t.name))?;
        if !eng.setup.trim().is_empty() {
            conn.execute_batch(&eng.render(&eng.setup, Some(t))).with_context(|| {
                let hint = if eng.extension.is_none() { " (this engine loads no extension: set `extension` in the engine file or pass --extension)" } else { "" };
                format!("setup failed for {}{hint}", t.name)
            })?;
        }
        table_results.push(load_table(&mut conn, &eng, t, &a.data_dir, a.batch_size)?);
    }

    let t1 = Instant::now();
    if !eng.post_load.trim().is_empty() {
        for t in &selected {
            conn.execute_batch(&eng.render(&eng.post_load, Some(t)))
                .context("post_load failed")?;
        }
    }
    engine::checkpoint(&conn)?;
    let post_load_seconds = t1.elapsed().as_secs_f64();

    let rows: u64 = table_results.iter().map(|r| r.rows).sum();
    let metrics: u64 = table_results.iter().map(|r| r.metrics).sum();
    let seconds: f64 = table_results.iter().map(|r| r.seconds).sum();
    let wait: f64 = table_results.iter().map(|r| r.parser_wait_seconds).sum();
    let res = LoadResult {
        kind: "load",
        engine: eng.name.clone(),
        dataset: m.dataset.clone(),
        created_at: results::now_rfc3339(),
        rows,
        metrics,
        seconds,
        rows_per_sec: rows as f64 / seconds,
        metrics_per_sec: metrics as f64 / seconds,
        workers: 1,
        batch_size: a.batch_size,
        tags_rows: m.tags.rows,
        tags_seconds,
        post_load_seconds,
        parser_wait_seconds: wait,
        tables: table_results,
        time_unit: eng.unit.name(),
        pragmas: eng.pragmas.clone(),
        extension: eng.extension.as_ref().map(|p| p.display().to_string()),
        sqlite_version: engine::sqlite_version(&conn),
        harness: format!("tsbench {}", env!("CARGO_PKG_VERSION")),
        host: results::host(),
    };
    eprintln!(
        "loaded {} rows ({} metrics) in {:.2}s: {:.0} rows/s, {:.0} metrics/s (parser wait {:.2}s, post-load {:.2}s)",
        res.rows, res.metrics, res.seconds, res.rows_per_sec, res.metrics_per_sec, wait, post_load_seconds
    );
    results::write(
        a.results_dir.as_deref(),
        &format!("{}.load.json", eng.name),
        &res,
    )?;
    Ok(())
}

/// Delete the database, its WAL/SHM/journal files and, for block-file engines, its block directory.
fn remove_db(db: &Path, block_dir: bool) -> Result<()> {
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let p = PathBuf::from(format!("{}{}", db.display(), suffix));
        if p.exists() {
            std::fs::remove_file(&p).with_context(|| format!("cannot remove {}", p.display()))?;
        }
    }
    if block_dir {
        let dir = Engine::block_dir_for(db);
        if dir.exists() {
            // Only delete a directory the extension created (it holds the `.timeseries` marker, BF-02).
            if !dir.join(".timeseries").exists() {
                bail!("{} exists but has no .timeseries marker; remove it yourself if it is safe to delete", dir.display());
            }
            std::fs::remove_dir_all(&dir)
                .with_context(|| format!("cannot remove {}", dir.display()))?;
        }
    }
    Ok(())
}

fn sql_type(tsbs_type: &str) -> &'static str {
    match tsbs_type {
        "float32" | "float64" => "REAL",
        "int32" | "int64" => "INTEGER",
        _ => "TEXT",
    }
}

/// The tags table mirrors TSBS's TimescaleDB schema: `id` primary key, one column per tag,
/// a unique index over all tags and an index on the first tag (hostname).
fn load_tags(conn: &mut Connection, m: &Manifest, data_dir: &Path) -> Result<()> {
    let cols = &m.tags.columns;
    let defs: Vec<String> = cols
        .iter()
        .map(|c| format!("{} {}", c.name, sql_type(&c.ty)))
        .collect();
    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    conn.execute_batch(&format!(
        "CREATE TABLE tags (id INTEGER PRIMARY KEY, {defs});\nCREATE UNIQUE INDEX tags_uniq ON tags ({all});\nCREATE INDEX tags_{first} ON tags ({first});",
        defs = defs.join(", "),
        all = names.join(", "),
        first = names[0],
    ))?;
    let placeholders = vec!["?"; cols.len() + 1].join(", ");
    let sql = format!(
        "INSERT INTO tags (id, {}) VALUES ({placeholders})",
        names.join(", ")
    );
    let reader = BufReader::new(
        File::open(data_dir.join(&m.tags.csv))
            .with_context(|| format!("cannot open {}", m.tags.csv))?,
    );
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(&sql)?;
        for (i, line) in reader.lines().enumerate().skip(1) {
            let line = line?;
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != cols.len() + 1 {
                bail!(
                    "tags.csv line {}: expected {} columns, got {}",
                    i + 1,
                    cols.len() + 1,
                    parts.len()
                );
            }
            stmt.raw_bind_parameter(1, parts[0].parse::<i64>()?)?;
            for (j, (v, c)) in parts[1..].iter().zip(cols).enumerate() {
                let idx = j + 2;
                if v.is_empty() {
                    stmt.raw_bind_parameter(idx, rusqlite::types::Null)?;
                } else if sql_type(&c.ty) == "REAL" {
                    stmt.raw_bind_parameter(idx, v.parse::<f64>()?)?;
                } else if sql_type(&c.ty) == "INTEGER" {
                    stmt.raw_bind_parameter(idx, v.parse::<i64>()?)?;
                } else {
                    stmt.raw_bind_parameter(idx, *v)?;
                }
            }
            stmt.raw_execute()?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn load_table(
    conn: &mut Connection,
    eng: &Engine,
    t: &TableInfo,
    data_dir: &Path,
    batch_size: usize,
) -> Result<TableResult> {
    let nfields = t.fields.len();
    let path = data_dir.join(&t.csv);
    let unit = eng.unit;
    let (tx, rx) = sync_channel::<Batch>(4);
    let reader = std::thread::spawn(move || parse_csv(&path, nfields, unit, batch_size, tx));

    let placeholders = vec!["?"; nfields + 2].join(", ");
    let sql = if nfields > 0 {
        format!(
            "INSERT INTO {} (time, tags_id, {}) VALUES ({placeholders})",
            t.name,
            t.fields.join(", ")
        )
    } else {
        format!("INSERT INTO {} (time, tags_id) VALUES (?, ?)", t.name)
    };

    let mut rows: u64 = 0;
    let mut wait = Duration::ZERO;
    let mut last_report = Instant::now();
    let start = Instant::now();
    loop {
        let w0 = Instant::now();
        let Ok(batch) = rx.recv() else { break };
        wait += w0.elapsed();
        let txn = conn.transaction()?;
        {
            let mut stmt = txn
                .prepare_cached(&sql)
                .with_context(|| format!("cannot prepare {sql}"))?;
            for i in 0..batch.times.len() {
                stmt.raw_bind_parameter(1, batch.times[i])?;
                stmt.raw_bind_parameter(2, batch.tags[i])?;
                for f in 0..nfields {
                    stmt.raw_bind_parameter(f + 3, batch.values[i * nfields + f])?;
                }
                stmt.raw_execute()
                    .with_context(|| format!("insert into {} failed", t.name))?;
            }
        }
        txn.commit()?;
        rows += batch.times.len() as u64;
        if last_report.elapsed().as_secs() >= 5 {
            last_report = Instant::now();
            let secs = start.elapsed().as_secs_f64();
            eprintln!(
                "  {}: {:>12} / {} rows  ({:.0} rows/s)",
                t.name,
                rows,
                t.rows,
                rows as f64 / secs
            );
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let parsed = reader
        .join()
        .map_err(|_| anyhow::anyhow!("CSV reader thread panicked"))??;
    if parsed != rows {
        bail!("{}: parsed {parsed} rows but inserted {rows}", t.name);
    }
    if rows != t.rows {
        eprintln!(
            "warning: {} has {rows} rows, manifest says {}",
            t.name, t.rows
        );
    }
    let metrics = rows * nfields as u64;
    Ok(TableResult {
        table: t.name.clone(),
        rows,
        metrics,
        seconds,
        rows_per_sec: rows as f64 / seconds,
        metrics_per_sec: metrics as f64 / seconds,
        parser_wait_seconds: wait.as_secs_f64(),
    })
}

fn parse_csv(
    path: &Path,
    nfields: usize,
    unit: TimeUnit,
    batch_size: usize,
    tx: std::sync::mpsc::SyncSender<Batch>,
) -> Result<u64> {
    let file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut r = BufReader::with_capacity(4 << 20, file);
    let mut line = String::new();
    r.read_line(&mut line)?; // header
    let mut batch = Batch::with_capacity(batch_size, nfields);
    let mut rows: u64 = 0;
    let mut line_no: u64 = 1;
    loop {
        line.clear();
        if r.read_line(&mut line)? == 0 {
            break;
        }
        line_no += 1;
        let l = line.trim_end_matches(['\n', '\r']);
        if l.is_empty() {
            continue;
        }
        let mut it = l.split(',');
        let ts = it.next().unwrap_or("");
        let us =
            timefmt::parse_ts_us(ts).with_context(|| format!("{}:{line_no}", path.display()))?;
        let tags_id: i64 = it
            .next()
            .unwrap_or("")
            .parse()
            .with_context(|| format!("{}:{line_no}: bad tags_id", path.display()))?;
        batch.times.push(unit.us_to_unit(us));
        batch.tags.push(tags_id);
        let mut n = 0;
        for v in it {
            n += 1;
            batch.values.push(if v.is_empty() {
                None
            } else {
                Some(
                    v.parse::<f64>().with_context(|| {
                        format!("{}:{line_no}: bad value {v:?}", path.display())
                    })?,
                )
            });
        }
        if n != nfields {
            bail!(
                "{}:{line_no}: expected {nfields} values, got {n}",
                path.display()
            );
        }
        rows += 1;
        if batch.times.len() >= batch_size {
            let full = std::mem::replace(&mut batch, Batch::with_capacity(batch_size, nfields));
            if tx.send(full).is_err() {
                return Ok(rows); // writer stopped (it reports its own error)
            }
        }
    }
    if !batch.times.is_empty() {
        let _ = tx.send(batch);
    }
    Ok(rows)
}

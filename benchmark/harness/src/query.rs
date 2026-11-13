//! `tsbench query`: run the TSBS devops query set against a SQLite engine.
//!
//! Like `tsbs_run_queries_*` with one worker: queries run one at a time, and each latency covers
//! preparing the SQL text and reading every row and column of the result.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use crate::engine::{self, Engine, EngineArgs};
use crate::manifest::Manifest;
use crate::queries::{self, Ctx, QueryType};
use crate::results::{self, Host};
use crate::rng::{seed_for, Rng};

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub engine: EngineArgs,
    #[arg(long)]
    pub data_dir: PathBuf,
    #[arg(long)]
    pub db: PathBuf,
    /// Queries per type.
    #[arg(long, default_value_t = 100)]
    pub queries: usize,
    /// Query types to run, comma-separated (default: every type that fits the dataset).
    #[arg(long, value_delimiter = ',')]
    pub types: Vec<String>,
    /// Seed for hosts and time windows.
    #[arg(long, default_value_t = 123)]
    pub seed: u64,
    /// Extra queries per type run first and not measured.
    #[arg(long, default_value_t = 0)]
    pub warmup: usize,
    /// Label stored with the results, e.g. `compressed` (the result file becomes `<engine>-<label>.query.json`).
    #[arg(long)]
    pub label: Option<String>,
    /// Write every generated SQL statement to this file.
    #[arg(long)]
    pub dump_sql: Option<PathBuf>,
    #[arg(long)]
    pub results_dir: Option<PathBuf>,
}

#[derive(Serialize)]
struct TypeStats {
    query_type: String,
    count: usize,
    min_ms: f64,
    q50_ms: f64,
    mean_ms: f64,
    q95_ms: f64,
    q99_ms: f64,
    max_ms: f64,
    avg_rows: f64,
}

#[derive(Serialize)]
struct QueryResult {
    kind: &'static str,
    engine: String,
    dataset: String,
    created_at: String,
    seed: u64,
    queries_per_type: usize,
    warmup: usize,
    types: Vec<TypeStats>,
    time_unit: &'static str,
    pragmas: Vec<String>,
    extension: Option<String>,
    sqlite_version: String,
    harness: String,
    host: Host,
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len()) - 1;
    sorted[idx]
}

pub fn run(a: Args) -> Result<()> {
    let m = Manifest::load(&a.data_dir)?;
    let eng = Engine::from_args(&a.engine)?;
    let conn = eng.open(&a.db)?;
    let (ok, skipped) = queries::applicable(&m)?;
    let selected: Vec<QueryType> = if a.types.is_empty() {
        for (qt, why) in &skipped {
            eprintln!("skipping {}: {why}", qt.name);
        }
        ok
    } else {
        let mut v = Vec::new();
        for name in &a.types {
            match ok.iter().find(|q| q.name == name) {
                Some(q) => v.push(*q),
                None => match skipped.iter().find(|(q, _)| q.name == name) {
                    Some((_, why)) => eprintln!("skipping {name}: {why}"),
                    None => bail!("unknown query type {name:?} (see `tsbench list-queries`)"),
                },
            }
        }
        v
    };

    let (start_us, end_us) = m.query_range_us()?;
    let ctx = Ctx {
        engine: &eng,
        start_us,
        end_us,
        scale: m.scale(),
    };
    let engine_name = match &a.label {
        Some(l) => format!("{}-{l}", eng.name),
        None => eng.name.clone(),
    };
    let mut dump = match &a.dump_sql {
        Some(p) => Some(std::io::BufWriter::new(
            std::fs::File::create(p).with_context(|| format!("cannot create {}", p.display()))?,
        )),
        None => None,
    };

    eprintln!(
        "{:<24} {:>6} {:>10} {:>10} {:>10} {:>10} {:>10}",
        "query type", "n", "q50 ms", "mean ms", "q95 ms", "max ms", "rows"
    );
    let mut stats = Vec::new();
    for qt in &selected {
        let mut rng = Rng::new(seed_for(a.seed, qt.name));
        let sqls: Vec<String> = (0..a.warmup + a.queries)
            .map(|_| queries::generate(qt, &ctx, &mut rng))
            .collect();
        if let Some(w) = dump.as_mut() {
            for s in &sqls[a.warmup..] {
                writeln!(w, "-- {}\n{s};", qt.name)?;
            }
        }
        let mut lat = Vec::with_capacity(a.queries);
        let mut rows_total: u64 = 0;
        for (i, sql) in sqls.iter().enumerate() {
            let t0 = Instant::now();
            let mut stmt = conn
                .prepare(sql)
                .with_context(|| format!("{}: cannot prepare: {sql}", qt.name))?;
            let ncol = stmt.column_count();
            let mut rows = stmt.raw_query();
            let mut n: u64 = 0;
            while let Some(row) = rows.next()? {
                for c in 0..ncol {
                    std::hint::black_box(row.get_ref(c)?);
                }
                n += 1;
            }
            drop(rows);
            drop(stmt);
            let ms = t0.elapsed().as_secs_f64() * 1e3;
            if i >= a.warmup {
                lat.push(ms);
                rows_total += n;
            }
        }
        let mut sorted = lat.clone();
        sorted.sort_by(|x, y| x.total_cmp(y));
        let s = TypeStats {
            query_type: qt.name.to_string(),
            count: lat.len(),
            min_ms: sorted.first().copied().unwrap_or(0.0),
            q50_ms: quantile(&sorted, 0.50),
            mean_ms: lat.iter().sum::<f64>() / lat.len().max(1) as f64,
            q95_ms: quantile(&sorted, 0.95),
            q99_ms: quantile(&sorted, 0.99),
            max_ms: sorted.last().copied().unwrap_or(0.0),
            avg_rows: rows_total as f64 / lat.len().max(1) as f64,
        };
        eprintln!(
            "{:<24} {:>6} {:>10.3} {:>10.3} {:>10.3} {:>10.3} {:>10.1}",
            s.query_type, s.count, s.q50_ms, s.mean_ms, s.q95_ms, s.max_ms, s.avg_rows
        );
        stats.push(s);
    }
    if let Some(mut w) = dump {
        w.flush()?;
    }

    let res = QueryResult {
        kind: "query",
        engine: engine_name.clone(),
        dataset: m.dataset.clone(),
        created_at: results::now_rfc3339(),
        seed: a.seed,
        queries_per_type: a.queries,
        warmup: a.warmup,
        types: stats,
        time_unit: eng.unit.name(),
        pragmas: eng.pragmas.clone(),
        extension: eng.extension.as_ref().map(|p| p.display().to_string()),
        sqlite_version: engine::sqlite_version(&conn),
        harness: format!("tsbench {}", env!("CARGO_PKG_VERSION")),
        host: results::host(),
    };
    results::write(
        a.results_dir.as_deref(),
        &format!("{engine_name}.query.json"),
        &res,
    )?;
    Ok(())
}

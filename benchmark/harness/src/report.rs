//! `tsbench report`: combine every result file in a results directory into one markdown report.

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use crate::convert::human_bytes;
use crate::manifest::Manifest;
use crate::queries;
use crate::timefmt;

#[derive(clap::Args)]
pub struct Args {
    /// Results directory for one dataset (it should contain a copy of manifest.json).
    #[arg(long)]
    pub results_dir: PathBuf,
    /// Where to write the report (default: <results-dir>/report.md). It is also printed.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

struct Load {
    engine: String,
    rows: f64,
    seconds: f64,
    rows_per_sec: f64,
    metrics_per_sec: f64,
    workers: f64,
    batch: f64,
}

struct Size {
    engine: String,
    phase: String,
    bytes: f64,
    per_metric: Option<f64>,
}

#[derive(Clone, Copy, Default)]
struct Lat {
    q50: f64,
    q95: f64,
}

fn num(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(f64::NAN)
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("?").to_string()
}

fn thousands(x: f64) -> String {
    if !x.is_finite() {
        return "–".into();
    }
    let s = format!("{:.0}", x);
    let (sign, digits) = s.strip_prefix('-').map(|d| ("-", d)).unwrap_or(("", &s));
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

fn ms(x: f64) -> String {
    if !x.is_finite() {
        "–".into()
    } else if x >= 100.0 {
        format!("{x:.0}")
    } else if x >= 1.0 {
        format!("{x:.2}")
    } else {
        format!("{x:.3}")
    }
}

pub fn run(a: Args) -> Result<()> {
    let manifest = Manifest::load(&a.results_dir).ok();
    let mut loads = Vec::new();
    let mut sizes = Vec::new();
    let mut compress: Vec<(String, f64)> = Vec::new();
    // query type -> engine -> latency
    let mut lat: BTreeMap<String, BTreeMap<String, Lat>> = BTreeMap::new();
    let mut engines_seen: Vec<String> = Vec::new();
    let note_engine = |e: &str, list: &mut Vec<String>| {
        if !list.iter().any(|x| x == e) {
            list.push(e.to_string());
        }
    };

    let mut files: Vec<PathBuf> = std::fs::read_dir(&a.results_dir)
        .with_context(|| format!("cannot read {}", a.results_dir.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().is_some_and(|x| x == "json")
                && p.file_name().is_some_and(|n| n != "manifest.json")
        })
        .collect();
    files.sort();

    for p in &files {
        let fname = p.file_name().unwrap().to_string_lossy().into_owned();
        let Ok(text_) = std::fs::read_to_string(p) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<Value>(&text_) else {
            eprintln!("skipping {fname}: not valid JSON");
            continue;
        };
        if let Some(totals) = v.get("Totals") {
            // TSBS query result: <engine>.query-<type>.tsbs.json
            let Some(stem) = fname.strip_suffix(".tsbs.json") else {
                continue;
            };
            let Some((engine, qtype)) = stem.split_once(".query-") else {
                continue;
            };
            if let Some(qs) = totals
                .get("overallQuantiles")
                .and_then(Value::as_object)
                .and_then(|o| o.values().next())
            {
                lat.entry(qtype.to_string()).or_default().insert(
                    engine.to_string(),
                    Lat {
                        q50: num(qs, "q50"),
                        q95: num(qs, "q95"),
                    },
                );
                note_engine(engine, &mut engines_seen);
            }
            continue;
        }
        match v.get("kind").and_then(Value::as_str) {
            Some("load") => loads.push(Load {
                engine: text(&v, "engine"),
                rows: num(&v, "rows"),
                seconds: num(&v, "seconds"),
                rows_per_sec: num(&v, "rows_per_sec"),
                metrics_per_sec: num(&v, "metrics_per_sec"),
                workers: num(&v, "workers"),
                batch: num(&v, "batch_size"),
            }),
            Some("size") => sizes.push(Size {
                engine: text(&v, "engine"),
                phase: text(&v, "phase"),
                bytes: num(&v, "total_bytes"),
                per_metric: v.get("bytes_per_metric").and_then(Value::as_f64),
            }),
            Some("compress") => compress.push((text(&v, "engine"), num(&v, "seconds"))),
            Some("query") => {
                let engine = text(&v, "engine");
                note_engine(&engine, &mut engines_seen);
                for t in v
                    .get("types")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    lat.entry(text(t, "query_type")).or_default().insert(
                        engine.clone(),
                        Lat {
                            q50: num(t, "q50_ms"),
                            q95: num(t, "q95_ms"),
                        },
                    );
                }
            }
            _ => {}
        }
    }

    let total_metrics = manifest.as_ref().map(|m| m.total_metrics() as f64);
    let mut r = String::new();
    let title = manifest
        .as_ref()
        .map(|m| m.dataset.clone())
        .unwrap_or_else(|| a.results_dir.display().to_string());
    writeln!(r, "# Benchmark report: {title}\n")?;

    if let Some(m) = &manifest {
        let (s, e) = m.query_range_us().unwrap_or((0, 0));
        writeln!(r, "| Dataset | |\n|---|---|")?;
        writeln!(
            r,
            "| Use case | {} |",
            m.source.use_case.as_deref().unwrap_or("?")
        )?;
        writeln!(r, "| Hosts (scale) | {} |", thousands(m.scale() as f64))?;
        writeln!(
            r,
            "| Time range | {} to {} |",
            timefmt::ts_string(s),
            timefmt::ts_string(e)
        )?;
        writeln!(
            r,
            "| Interval | {} |",
            m.source.log_interval.as_deref().unwrap_or("?")
        )?;
        writeln!(r, "| Rows | {} |", thousands(m.total_rows() as f64))?;
        writeln!(
            r,
            "| Metrics (rows × fields) | {} |",
            thousands(m.total_metrics() as f64)
        )?;
        writeln!(
            r,
            "| Seed | {} |\n",
            m.source
                .seed
                .map(|s| s.to_string())
                .unwrap_or_else(|| "?".into())
        )?;
    }

    if !loads.is_empty() {
        loads.sort_by(|x, y| y.rows_per_sec.total_cmp(&x.rows_per_sec));
        writeln!(r, "## Ingest\n")?;
        writeln!(r, "| Engine | Rows/s | Metrics/s | Time (s) | Rows | Workers | Batch |\n|---|--:|--:|--:|--:|--:|--:|")?;
        for l in &loads {
            writeln!(
                r,
                "| {} | {} | {} | {:.2} | {} | {} | {} |",
                l.engine,
                thousands(l.rows_per_sec),
                thousands(l.metrics_per_sec),
                l.seconds,
                thousands(l.rows),
                thousands(l.workers),
                thousands(l.batch)
            )?;
        }
        writeln!(r)?;
    }

    if !sizes.is_empty() || manifest.is_some() {
        writeln!(r, "## Storage\n")?;
        writeln!(r, "Bytes per metric = total size ÷ (rows × fields). The 16 B row is an uncompressed 8-byte timestamp plus an 8-byte value per metric, the baseline the Gorilla paper uses.\n")?;
        writeln!(
            r,
            "| Engine | Phase | Size | Bytes/metric | Ratio vs 16 B |\n|---|---|--:|--:|--:|"
        )?;
        let row = |r: &mut String,
                   name: &str,
                   phase: &str,
                   bytes: f64,
                   per: Option<f64>|
         -> std::fmt::Result {
            let per = per.or_else(|| total_metrics.map(|t| bytes / t));
            match per {
                Some(p) => writeln!(
                    r,
                    "| {name} | {phase} | {} | {p:.2} | {:.1}× |",
                    human_bytes(bytes as u64),
                    16.0 / p
                ),
                None => writeln!(
                    r,
                    "| {name} | {phase} | {} | – | – |",
                    human_bytes(bytes as u64)
                ),
            }
        };
        if let Some(m) = &manifest {
            let t = m.total_metrics() as f64;
            row(&mut r, "baseline: 16 B/metric", "–", t * 16.0, Some(16.0))?;
            let csv: u64 = m.tags.csv_bytes + m.tables.iter().map(|x| x.csv_bytes).sum::<u64>();
            row(&mut r, "baseline: CSV", "–", csv as f64, None)?;
            if let Some(pq) = m
                .tables
                .iter()
                .map(|x| x.parquet_bytes)
                .sum::<Option<u64>>()
            {
                row(
                    &mut r,
                    "baseline: Parquet (zstd)",
                    "–",
                    (pq + m.tags.parquet_bytes.unwrap_or(0)) as f64,
                    None,
                )?;
            }
        }
        sizes.sort_by(|x, y| {
            x.engine
                .cmp(&y.engine)
                .then(x.phase.cmp(&y.phase).reverse())
        });
        for s in &sizes {
            row(&mut r, &s.engine, &s.phase, s.bytes, s.per_metric)?;
        }
        writeln!(r)?;
    }

    if !compress.is_empty() {
        writeln!(r, "## Compression time\n\n| Engine | Seconds |\n|---|--:|")?;
        for (e, s) in &compress {
            writeln!(r, "| {e} | {s:.2} |")?;
        }
        writeln!(r)?;
    }

    if !lat.is_empty() {
        let order: Vec<String> = queries::catalog()
            .iter()
            .map(|q| q.name.to_string())
            .collect();
        let mut types: Vec<String> = lat.keys().cloned().collect();
        types.sort_by_key(|t| order.iter().position(|o| o == t).unwrap_or(usize::MAX));
        let engines: Vec<String> = {
            let present: BTreeSet<&String> = lat.values().flat_map(|m| m.keys()).collect();
            let mut v: Vec<String> = engines_seen
                .iter()
                .filter(|e| present.contains(e))
                .cloned()
                .collect();
            // Each engine next to its "-compressed" run: sqlite, sqlite-clustered, timescaledb, timescaledb-compressed, ...
            v.sort_by_key(|e| match e.strip_suffix("-compressed") {
                Some(base) => (base.to_string(), 1),
                None => (e.clone(), 0),
            });
            v
        };
        for (label, pick) in [("median", 0), ("p95", 1)] {
            writeln!(r, "## Query latency, {label} (ms)\n")?;
            writeln!(r, "| Query | {} |", engines.join(" | "))?;
            writeln!(r, "|---|{}", "--:|".repeat(engines.len()))?;
            for t in &types {
                let cells: Vec<String> = engines
                    .iter()
                    .map(|e| {
                        lat[t]
                            .get(e)
                            .map(|l| ms(if pick == 0 { l.q50 } else { l.q95 }))
                            .unwrap_or_else(|| "–".into())
                    })
                    .collect();
                writeln!(r, "| {t} | {} |", cells.join(" | "))?;
            }
            writeln!(r)?;
        }
        writeln!(r, "TimescaleDB latencies come from TSBS's runner and include the client-server round trip; SQLite runs in-process.\n")?;
    }

    let out = a.out.unwrap_or_else(|| a.results_dir.join("report.md"));
    std::fs::write(&out, &r).with_context(|| format!("cannot write {}", out.display()))?;
    println!("{r}");
    eprintln!("wrote {}", out.display());
    Ok(())
}

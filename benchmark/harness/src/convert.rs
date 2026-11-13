//! `tsbench convert`: TSBS output -> engine-neutral CSV (+ Parquet) files and `manifest.json`.
//!
//! Output, for a dataset with measurement `cpu`:
//!
//! - `tags.csv`      `id,hostname,region,...`        one row per tag set (host/device)
//! - `cpu.csv`       `time,tags_id,usage_user,...`   time as RFC 3339 UTC, microsecond precision
//! - `tags.parquet`, `cpu.parquet`                   same tables, zstd-compressed (unless --no-parquet)
//! - `manifest.json`                                 schema, row counts, time range, TSBS parameters
//!
//! Field values are copied verbatim from TSBS, so the CSV holds exactly what TSBS generated.
//! Timestamps are truncated from nanoseconds to microseconds, PostgreSQL's precision; the manifest
//! counts how many rows that affected (zero for the standard TSBS use cases).

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::manifest::{Column, Manifest, Source, TableInfo, TagsInfo};
use crate::{timefmt, tsbs};

#[derive(clap::Args)]
pub struct Args {
    /// TSBS output in `timescaledb` format (plain or gzip), or `-` for stdin.
    #[arg(long)]
    pub input: String,
    /// Directory to write the CSV/Parquet files and manifest.json into.
    #[arg(long)]
    pub out_dir: PathBuf,
    /// Dataset name recorded in the manifest (default: the output directory's name).
    #[arg(long)]
    pub dataset: Option<String>,
    /// Skip writing Parquet files.
    #[arg(long)]
    pub no_parquet: bool,
    /// TSBS parameters to record in the manifest (used later to generate matching queries).
    #[arg(long)]
    pub use_case: Option<String>,
    #[arg(long)]
    pub scale: Option<u64>,
    #[arg(long)]
    pub seed: Option<i64>,
    #[arg(long)]
    pub timestamp_start: Option<String>,
    #[arg(long)]
    pub timestamp_end: Option<String>,
    #[arg(long)]
    pub log_interval: Option<String>,
}

struct TableOut {
    info: TableInfo,
    csv: BufWriter<File>,
    nfields: usize,
    last_time: i64,
    #[cfg(feature = "parquet")]
    pq: Option<pq::TableWriter>,
}

pub fn run(a: Args) -> Result<()> {
    let started = Instant::now();
    std::fs::create_dir_all(&a.out_dir)
        .with_context(|| format!("cannot create {}", a.out_dir.display()))?;
    let dataset = a.dataset.clone().unwrap_or_else(|| {
        a.out_dir
            .canonicalize()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "dataset".into())
    });
    for ts in [&a.timestamp_start, &a.timestamp_end].into_iter().flatten() {
        timefmt::parse_ts_us(ts)
            .with_context(|| format!("bad --timestamp-start/--timestamp-end {ts:?}"))?;
    }
    let write_parquet = !a.no_parquet && cfg!(feature = "parquet");
    if !a.no_parquet && !cfg!(feature = "parquet") {
        eprintln!("note: built without the `parquet` feature; writing CSV only");
    }

    let mut input = tsbs::open_input(&a.input)?;
    let header = tsbs::read_header(&mut *input)?;
    eprintln!(
        "header: tags [{}], measurements [{}]",
        header.tag_names.join(", "),
        header
            .measurements
            .iter()
            .map(|m| format!("{} ({} fields)", m.name, m.fields.len()))
            .collect::<Vec<_>>()
            .join(", ")
    );

    let mut tables: Vec<TableOut> = Vec::new();
    let mut table_idx: HashMap<String, usize> = HashMap::new();
    for m in &header.measurements {
        let csv_name = format!("{}.csv", m.name);
        let mut csv = BufWriter::with_capacity(1 << 20, File::create(a.out_dir.join(&csv_name))?);
        writeln!(csv, "time,tags_id,{}", m.fields.join(","))?;
        #[cfg(feature = "parquet")]
        let pq = if write_parquet {
            Some(pq::TableWriter::new(
                &a.out_dir.join(format!("{}.parquet", m.name)),
                &m.fields,
            )?)
        } else {
            None
        };
        table_idx.insert(m.name.clone(), tables.len());
        tables.push(TableOut {
            info: TableInfo {
                name: m.name.clone(),
                fields: m.fields.clone(),
                rows: 0,
                metrics: 0,
                null_values: 0,
                time_min_us: i64::MAX,
                time_max_us: i64::MIN,
                out_of_order_rows: 0,
                truncated_timestamps: 0,
                csv: csv_name,
                csv_bytes: 0,
                parquet: write_parquet.then(|| format!("{}.parquet", m.name)),
                parquet_bytes: None,
            },
            csv,
            nfields: m.fields.len(),
            last_time: i64::MIN,
            #[cfg(feature = "parquet")]
            pq,
        });
    }

    // Tag sets: full `k=v,...` string -> id (1-based, in order of first appearance).
    let mut tag_ids: HashMap<String, i64> = HashMap::new();
    let mut tag_rows: Vec<Vec<Option<String>>> = Vec::new();
    let tag_pos: HashMap<&str, usize> = header
        .tag_names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    let mut current_tag: Option<i64> = None;
    let mut line = String::new();
    let mut ts_buf = String::with_capacity(32);
    let mut values: Vec<Option<f64>> = Vec::new();
    let mut line_no: u64 = 0;
    let mut total_rows: u64 = 0;
    let mut last_report = Instant::now();

    loop {
        line.clear();
        if input.read_line(&mut line)? == 0 {
            break;
        }
        line_no += 1;
        let l = line.trim_end_matches(['\n', '\r']);
        if l.is_empty() {
            continue;
        }
        if l == "tags" || l.starts_with("tags,") {
            let rest = l
                .strip_prefix("tags")
                .unwrap()
                .strip_prefix(',')
                .unwrap_or("");
            let id = match tag_ids.get(rest) {
                Some(&id) => id,
                None => {
                    let mut row = vec![None; header.tag_names.len()];
                    if !rest.is_empty() {
                        for kv in rest.split(',') {
                            let (k, v) = kv
                                .split_once('=')
                                .with_context(|| format!("line {line_no}: bad tag {kv:?}"))?;
                            let i = *tag_pos.get(k).with_context(|| {
                                format!("line {line_no}: tag {k:?} is not in the header")
                            })?;
                            row[i] = (!v.is_empty()).then(|| v.to_string());
                        }
                    }
                    tag_rows.push(row);
                    let id = tag_rows.len() as i64;
                    tag_ids.insert(rest.to_string(), id);
                    id
                }
            };
            current_tag = Some(id);
            continue;
        }

        let (name, rest) = l
            .split_once(',')
            .with_context(|| format!("line {line_no}: expected `measurement,timestamp,...`"))?;
        let ti = *table_idx.get(name).with_context(|| {
            format!("line {line_no}: measurement {name:?} is not in the header")
        })?;
        let t = &mut tables[ti];
        let (ts_str, vals) = rest.split_once(',').unwrap_or((rest, ""));
        let ns: i64 = ts_str
            .parse()
            .with_context(|| format!("line {line_no}: bad timestamp {ts_str:?}"))?;
        let us = ns.div_euclid(1000);
        if ns.rem_euclid(1000) != 0 {
            t.info.truncated_timestamps += 1;
        }
        let tags_id = current_tag
            .with_context(|| format!("line {line_no}: data line before any tags line"))?;

        let mut n = 0usize;
        let mut nulls = 0u64;
        values.clear();
        if t.nfields > 0 {
            for v in vals.split(',') {
                n += 1;
                if v.is_empty() {
                    nulls += 1;
                    values.push(None);
                } else if write_parquet {
                    values
                        .push(Some(parse_value(v).with_context(|| {
                            format!("line {line_no}: bad value {v:?}")
                        })?));
                }
            }
        }
        if n != t.nfields {
            bail!(
                "line {line_no}: {name} has {n} values, header says {}",
                t.nfields
            );
        }

        t.info.rows += 1;
        t.info.null_values += nulls;
        t.info.time_min_us = t.info.time_min_us.min(us);
        t.info.time_max_us = t.info.time_max_us.max(us);
        if us < t.last_time {
            t.info.out_of_order_rows += 1;
        }
        t.last_time = us;

        ts_buf.clear();
        timefmt::format_ts_us(us, &mut ts_buf);
        t.csv.write_all(ts_buf.as_bytes())?;
        write!(t.csv, ",{tags_id}")?;
        if t.nfields > 0 {
            t.csv.write_all(b",")?;
            t.csv.write_all(vals.as_bytes())?;
        }
        t.csv.write_all(b"\n")?;

        #[cfg(feature = "parquet")]
        if let Some(pq) = t.pq.as_mut() {
            pq.push(us, tags_id, &values)?;
        }

        total_rows += 1;
        if total_rows.is_multiple_of(1_000_000) && last_report.elapsed().as_secs() >= 5 {
            last_report = Instant::now();
            let secs = started.elapsed().as_secs_f64();
            eprintln!(
                "  {:>12} rows  ({:.0} rows/s)",
                total_rows,
                total_rows as f64 / secs
            );
        }
    }

    // Finish measurement tables.
    let mut infos = Vec::new();
    for mut t in tables {
        t.csv.flush()?;
        drop(t.csv);
        t.info.metrics = t.info.rows * t.nfields as u64;
        if t.info.rows == 0 {
            t.info.time_min_us = 0;
            t.info.time_max_us = 0;
        }
        t.info.csv_bytes = file_len(&a.out_dir.join(&t.info.csv));
        #[cfg(feature = "parquet")]
        if let Some(pq) = t.pq.take() {
            pq.finish()?;
        }
        t.info.parquet_bytes = t
            .info
            .parquet
            .as_ref()
            .map(|p| file_len(&a.out_dir.join(p)));
        infos.push(t.info);
    }

    // Tags table.
    let tags_csv = "tags.csv".to_string();
    {
        let mut w = BufWriter::new(File::create(a.out_dir.join(&tags_csv))?);
        writeln!(w, "id,{}", header.tag_names.join(","))?;
        for (i, row) in tag_rows.iter().enumerate() {
            write!(w, "{}", i + 1)?;
            for v in row {
                w.write_all(b",")?;
                if let Some(v) = v {
                    w.write_all(v.as_bytes())?;
                }
            }
            w.write_all(b"\n")?;
        }
        w.flush()?;
    }
    let columns: Vec<Column> = header
        .tag_names
        .iter()
        .zip(&header.tag_types)
        .map(|(n, t)| Column {
            name: n.clone(),
            ty: t.clone(),
        })
        .collect();
    let mut tags = TagsInfo {
        columns,
        rows: tag_rows.len() as u64,
        csv_bytes: file_len(&a.out_dir.join(&tags_csv)),
        csv: tags_csv,
        parquet: None,
        parquet_bytes: None,
    };
    #[cfg(feature = "parquet")]
    if write_parquet {
        let p = "tags.parquet".to_string();
        pq::write_tags(&a.out_dir.join(&p), &tags.columns, &tag_rows)?;
        tags.parquet_bytes = Some(file_len(&a.out_dir.join(&p)));
        tags.parquet = Some(p);
    }

    let manifest = Manifest {
        format_version: 1,
        dataset,
        created_by: format!("tsbench {}", env!("CARGO_PKG_VERSION")),
        source: Source {
            generator: Some("tsbs (timescaledb format)".into()),
            use_case: a.use_case,
            scale: a.scale,
            seed: a.seed,
            timestamp_start: a.timestamp_start,
            timestamp_end: a.timestamp_end,
            log_interval: a.log_interval,
        },
        time_precision: "us".into(),
        tags,
        tables: infos,
    };
    manifest.save(&a.out_dir)?;

    let secs = started.elapsed().as_secs_f64();
    eprintln!(
        "converted {} rows ({} metrics, {} tag sets) in {:.1}s -> {}",
        manifest.total_rows(),
        manifest.total_metrics(),
        manifest.tags.rows,
        secs,
        a.out_dir.display()
    );
    for t in &manifest.tables {
        let pq = t
            .parquet_bytes
            .map(|b| format!(", parquet {}", human_bytes(b)))
            .unwrap_or_default();
        eprintln!(
            "  {:<10} rows {:>12}  csv {}{}  out-of-order {}  nulls {}  truncated-ts {}",
            t.name,
            t.rows,
            human_bytes(t.csv_bytes),
            pq,
            t.out_of_order_rows,
            t.null_values,
            t.truncated_timestamps
        );
    }
    Ok(())
}

fn parse_value(v: &str) -> Result<f64> {
    match v {
        "true" => Ok(1.0),
        "false" => Ok(0.0),
        _ => Ok(v.parse::<f64>()?),
    }
}

fn file_len(p: &Path) -> u64 {
    std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
}

pub fn human_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{b} B")
    } else {
        format!("{v:.2} {}", UNITS[u])
    }
}

#[cfg(feature = "parquet")]
mod pq {
    use anyhow::{bail, Result};
    use arrow_array::builder::{
        Float32Builder, Float64Builder, Int32Builder, Int64Builder, StringBuilder,
    };
    use arrow_array::{ArrayRef, Float64Array, Int64Array, RecordBatch, TimestampMicrosecondArray};
    use arrow_schema::{DataType, Field, Schema, SchemaRef, TimeUnit};
    use parquet::arrow::ArrowWriter;
    use parquet::basic::{Compression, ZstdLevel};
    use parquet::file::properties::WriterProperties;
    use std::fs::File;
    use std::path::Path;
    use std::sync::Arc;

    use crate::manifest::Column;

    const BATCH_ROWS: usize = 64 * 1024;

    fn props() -> Result<WriterProperties> {
        Ok(WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::try_new(3)?))
            .build())
    }

    pub struct TableWriter {
        writer: ArrowWriter<File>,
        schema: SchemaRef,
        times: Vec<i64>,
        tags: Vec<i64>,
        cols: Vec<Vec<Option<f64>>>,
    }

    impl TableWriter {
        pub fn new(path: &Path, fields: &[String]) -> Result<Self> {
            let mut fs = vec![
                Field::new(
                    "time",
                    DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
                    false,
                ),
                Field::new("tags_id", DataType::Int64, false),
            ];
            fs.extend(
                fields
                    .iter()
                    .map(|f| Field::new(f, DataType::Float64, true)),
            );
            let schema: SchemaRef = Arc::new(Schema::new(fs));
            let writer = ArrowWriter::try_new(File::create(path)?, schema.clone(), Some(props()?))?;
            Ok(TableWriter {
                writer,
                schema,
                times: Vec::with_capacity(BATCH_ROWS),
                tags: Vec::with_capacity(BATCH_ROWS),
                cols: vec![Vec::with_capacity(BATCH_ROWS); fields.len()],
            })
        }

        pub fn push(&mut self, us: i64, tags_id: i64, values: &[Option<f64>]) -> Result<()> {
            if values.len() != self.cols.len() {
                bail!(
                    "parquet writer: expected {} values, got {}",
                    self.cols.len(),
                    values.len()
                );
            }
            self.times.push(us);
            self.tags.push(tags_id);
            for (c, v) in self.cols.iter_mut().zip(values) {
                c.push(*v);
            }
            if self.times.len() >= BATCH_ROWS {
                self.flush()?;
            }
            Ok(())
        }

        fn flush(&mut self) -> Result<()> {
            if self.times.is_empty() {
                return Ok(());
            }
            let mut arrays: Vec<ArrayRef> = vec![
                Arc::new(
                    TimestampMicrosecondArray::from(std::mem::take(&mut self.times))
                        .with_timezone("UTC"),
                ),
                Arc::new(Int64Array::from(std::mem::take(&mut self.tags))),
            ];
            for c in self.cols.iter_mut() {
                arrays.push(Arc::new(Float64Array::from(std::mem::take(c))));
            }
            let batch = RecordBatch::try_new(self.schema.clone(), arrays)?;
            self.writer.write(&batch)?;
            Ok(())
        }

        pub fn finish(mut self) -> Result<()> {
            self.flush()?;
            self.writer.close()?;
            Ok(())
        }
    }

    pub fn write_tags(path: &Path, columns: &[Column], rows: &[Vec<Option<String>>]) -> Result<()> {
        let mut fields = vec![Field::new("id", DataType::Int64, false)];
        let mut arrays: Vec<ArrayRef> = vec![Arc::new(Int64Array::from_iter_values(
            1..=rows.len() as i64,
        ))];
        for (i, c) in columns.iter().enumerate() {
            let vals = rows.iter().map(|r| r[i].as_deref());
            let (dt, arr): (DataType, ArrayRef) = match c.ty.as_str() {
                "float32" => {
                    let mut b = Float32Builder::new();
                    for v in vals {
                        b.append_option(v.map(str::parse::<f32>).transpose()?);
                    }
                    (DataType::Float32, Arc::new(b.finish()))
                }
                "float64" => {
                    let mut b = Float64Builder::new();
                    for v in vals {
                        b.append_option(v.map(str::parse::<f64>).transpose()?);
                    }
                    (DataType::Float64, Arc::new(b.finish()))
                }
                "int32" => {
                    let mut b = Int32Builder::new();
                    for v in vals {
                        b.append_option(v.map(str::parse::<i32>).transpose()?);
                    }
                    (DataType::Int32, Arc::new(b.finish()))
                }
                "int64" => {
                    let mut b = Int64Builder::new();
                    for v in vals {
                        b.append_option(v.map(str::parse::<i64>).transpose()?);
                    }
                    (DataType::Int64, Arc::new(b.finish()))
                }
                _ => {
                    let mut b = StringBuilder::new();
                    for v in vals {
                        b.append_option(v);
                    }
                    (DataType::Utf8, Arc::new(b.finish()))
                }
            };
            fields.push(Field::new(&c.name, dt, true));
            arrays.push(arr);
        }
        let schema: SchemaRef = Arc::new(Schema::new(fields));
        let mut w = ArrowWriter::try_new(File::create(path)?, schema.clone(), Some(props()?))?;
        w.write(&RecordBatch::try_new(schema, arrays)?)?;
        w.close()?;
        Ok(())
    }
}

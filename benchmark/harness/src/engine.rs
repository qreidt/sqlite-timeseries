//! SQLite engine definitions, read from `engines/*.toml`.
//!
//! An engine is a plain SQLite schema or the time-series extension, described entirely by SQL
//! templates, so trying a new layout (or the extension's API as it evolves) means editing a TOML
//! file, not this code. Placeholders available in every template:
//!
//! | placeholder              | example                                    |
//! |--------------------------|--------------------------------------------|
//! | `{table}`                | `cpu`                                      |
//! | `{field_columns}`        | `usage_user REAL, usage_system REAL, ...`  |
//! | `{fields}`               | `usage_user, usage_system, ...`            |
//! | `{time_unit}`            | `us`                                       |
//! | `{chunk_interval}`       | `43200000000` (integer, in `time_unit`)    |
//! | `{chunk_interval_text}`  | `12 hours`                                 |
//!
//! `bucket` additionally gets `{time}` (the time column) and `{width}` (integer, in `time_unit`).

use anyhow::{bail, Context, Result};
use rusqlite::Connection;
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::manifest::TableInfo;
use crate::timefmt;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EngineFile {
    name: Option<String>,
    description: Option<String>,
    #[serde(default)]
    extension: String,
    #[serde(default)]
    extension_entry_point: String,
    time_unit: Option<String>,
    chunk_interval: Option<String>,
    #[serde(default)]
    pragmas: Vec<String>,
    create_table: String,
    #[serde(default)]
    setup: String,
    #[serde(default)]
    post_load: String,
    #[serde(default)]
    compress: String,
    bucket: Option<String>,
    #[serde(default)]
    block_dir: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeUnit {
    S,
    Ms,
    Us,
    Ns,
}

impl TimeUnit {
    fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "s" => TimeUnit::S,
            "ms" => TimeUnit::Ms,
            "us" => TimeUnit::Us,
            "ns" => TimeUnit::Ns,
            _ => bail!("time_unit must be one of s, ms, us, ns (got {s:?})"),
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            TimeUnit::S => "s",
            TimeUnit::Ms => "ms",
            TimeUnit::Us => "us",
            TimeUnit::Ns => "ns",
        }
    }

    /// Convert microseconds to this unit (floor division for coarser units).
    pub fn us_to_unit(self, us: i64) -> i64 {
        match self {
            TimeUnit::S => us.div_euclid(1_000_000),
            TimeUnit::Ms => us.div_euclid(1_000),
            TimeUnit::Us => us,
            TimeUnit::Ns => us * 1_000,
        }
    }
}

pub struct Engine {
    pub name: String,
    pub description: String,
    pub extension: Option<PathBuf>,
    pub extension_entry_point: Option<String>,
    pub unit: TimeUnit,
    pub chunk_interval_us: i64,
    pub pragmas: Vec<String>,
    pub create_table: String,
    pub setup: String,
    pub post_load: String,
    pub compress: String,
    pub bucket: String,
    pub block_dir: bool,
}

/// Command-line options shared by every subcommand that opens a SQLite engine.
#[derive(clap::Args, Clone)]
pub struct EngineArgs {
    /// Engine definition (TOML), e.g. engines/sqlite.toml.
    #[arg(long)]
    pub engine: PathBuf,
    /// Override the engine's `extension` path (the loadable extension to test).
    #[arg(long)]
    pub extension: Option<PathBuf>,
    /// Override the engine name used in result files.
    #[arg(long)]
    pub name: Option<String>,
}

impl Engine {
    pub fn from_args(a: &EngineArgs) -> Result<Engine> {
        Engine::load(&a.engine, a.extension.clone(), a.name.clone())
    }

    pub fn load(path: &Path, extension: Option<PathBuf>, name: Option<String>) -> Result<Engine> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read engine file {}", path.display()))?;
        let f: EngineFile = toml::from_str(&text)
            .with_context(|| format!("invalid engine file {}", path.display()))?;
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "sqlite".into());
        let extension = extension.or_else(|| {
            (!f.extension.trim().is_empty()).then(|| PathBuf::from(f.extension.trim()))
        });
        let chunk = f.chunk_interval.as_deref().unwrap_or("12h");
        Ok(Engine {
            name: name.or(f.name).unwrap_or(stem),
            description: f.description.unwrap_or_default(),
            extension,
            extension_entry_point: (!f.extension_entry_point.trim().is_empty())
                .then(|| f.extension_entry_point.trim().to_string()),
            unit: TimeUnit::parse(f.time_unit.as_deref().unwrap_or("us"))?,
            chunk_interval_us: timefmt::parse_duration_us(chunk)
                .with_context(|| format!("bad chunk_interval {chunk:?}"))?,
            pragmas: f.pragmas,
            create_table: f.create_table,
            setup: f.setup,
            post_load: f.post_load,
            compress: f.compress,
            bucket: f
                .bucket
                .unwrap_or_else(|| "(({time}) / {width}) * {width}".into()),
            block_dir: f.block_dir,
        })
    }

    /// Open a connection: load the extension (if any), then apply the pragmas.
    pub fn open(&self, db: &Path) -> Result<Connection> {
        let conn = Connection::open(db).with_context(|| format!("cannot open {}", db.display()))?;
        if let Some(ext) = &self.extension {
            // SAFETY: loading a native extension runs its code in this process. The path comes
            // from the engine file or the command line, which the person running the benchmark controls.
            unsafe {
                conn.load_extension_enable()?;
                let r = conn.load_extension(ext, self.extension_entry_point.as_deref());
                conn.load_extension_disable()?;
                r.with_context(|| format!("cannot load extension {}", ext.display()))?;
            }
        }
        for p in &self.pragmas {
            conn.execute_batch(&format!("PRAGMA {p};"))
                .with_context(|| format!("PRAGMA {p} failed"))?;
        }
        Ok(conn)
    }

    pub fn render(&self, template: &str, table: Option<&TableInfo>) -> String {
        let mut s = template
            .replace("{time_unit}", self.unit.name())
            .replace(
                "{chunk_interval_text}",
                &timefmt::pg_interval(self.chunk_interval_us),
            )
            .replace(
                "{chunk_interval}",
                &self.unit.us_to_unit(self.chunk_interval_us).to_string(),
            );
        if let Some(t) = table {
            let field_columns: Vec<String> = t.fields.iter().map(|f| format!("{f} REAL")).collect();
            s = s
                .replace("{table}", &t.name)
                .replace("{field_columns}", &field_columns.join(", "))
                .replace("{fields}", &t.fields.join(", "));
        }
        s
    }

    /// SQL expression that floors `time_col` to a bucket of `width_us` microseconds.
    pub fn bucket_expr(&self, time_col: &str, width_us: i64) -> String {
        self.bucket
            .replace("{time}", time_col)
            .replace("{width}", &self.unit.us_to_unit(width_us).to_string())
    }

    /// The block directory next to `metrics.db` is `metrics/` (see docs/08-block-files.md, BF-01).
    pub fn block_dir_for(db: &Path) -> PathBuf {
        db.with_extension("")
    }
}

/// `PRAGMA wal_checkpoint(TRUNCATE)`, so the main file holds everything before sizes are read.
pub fn checkpoint(conn: &Connection) -> Result<()> {
    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .or_else(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Ok(()),
            e => Err(e),
        })?;
    Ok(())
}

pub fn sqlite_version(conn: &Connection) -> String {
    conn.query_row("SELECT sqlite_version()", [], |r| r.get::<_, String>(0))
        .unwrap_or_default()
}

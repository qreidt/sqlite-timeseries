//! `manifest.json`: describes a converted dataset, so every engine loads and queries the same thing.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::timefmt;

pub const FILE: &str = "manifest.json";

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Source {
    pub generator: Option<String>,
    pub use_case: Option<String>,
    pub scale: Option<u64>,
    pub seed: Option<i64>,
    pub timestamp_start: Option<String>,
    pub timestamp_end: Option<String>,
    pub log_interval: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Column {
    pub name: String,
    /// TSBS tag type: string, float32, float64, int32, int64.
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TagsInfo {
    pub columns: Vec<Column>,
    pub rows: u64,
    pub csv: String,
    pub csv_bytes: u64,
    pub parquet: Option<String>,
    pub parquet_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TableInfo {
    pub name: String,
    pub fields: Vec<String>,
    pub rows: u64,
    /// rows x fields, the way TSBS counts metrics.
    pub metrics: u64,
    pub null_values: u64,
    pub time_min_us: i64,
    pub time_max_us: i64,
    /// Rows whose time is earlier than the previous row of the same table.
    pub out_of_order_rows: u64,
    /// Rows whose nanosecond timestamp was not a whole microsecond (truncated on conversion).
    pub truncated_timestamps: u64,
    pub csv: String,
    pub csv_bytes: u64,
    pub parquet: Option<String>,
    pub parquet_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Manifest {
    pub format_version: u32,
    pub dataset: String,
    pub created_by: String,
    pub source: Source,
    /// Precision of the `time` column in the CSV and Parquet files.
    pub time_precision: String,
    pub tags: TagsInfo,
    pub tables: Vec<TableInfo>,
}

impl Manifest {
    pub fn load(dir: &Path) -> Result<Manifest> {
        let p = dir.join(FILE);
        let text = std::fs::read_to_string(&p).with_context(|| {
            format!("cannot read {} (run `tsbench convert` first)", p.display())
        })?;
        serde_json::from_str(&text).with_context(|| format!("cannot parse {}", p.display()))
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let p = dir.join(FILE);
        std::fs::write(&p, serde_json::to_string_pretty(self)? + "\n")
            .with_context(|| format!("cannot write {}", p.display()))
    }

    pub fn total_rows(&self) -> u64 {
        self.tables.iter().map(|t| t.rows).sum()
    }

    pub fn total_metrics(&self) -> u64 {
        self.tables.iter().map(|t| t.metrics).sum()
    }

    pub fn table(&self, name: &str) -> Option<&TableInfo> {
        self.tables.iter().find(|t| t.name == name)
    }

    /// Number of hosts/devices: the TSBS scale if recorded, else the number of tag sets.
    pub fn scale(&self) -> u64 {
        self.source.scale.unwrap_or(self.tags.rows)
    }

    /// Time range queries are drawn from: the TSBS generation range if recorded, else the data's range.
    pub fn query_range_us(&self) -> Result<(i64, i64)> {
        let min = self.tables.iter().map(|t| t.time_min_us).min().unwrap_or(0);
        let max = self.tables.iter().map(|t| t.time_max_us).max().unwrap_or(0);
        let start = match &self.source.timestamp_start {
            Some(s) => timefmt::parse_ts_us(s)?,
            None => min,
        };
        let end = match &self.source.timestamp_end {
            Some(s) => timefmt::parse_ts_us(s)?,
            None => max + 1,
        };
        Ok((start, end))
    }
}

//! Result files. Every run writes one JSON file per step into the results directory:
//!
//! - `<engine>.load.json`             kind = "load"
//! - `<engine>.size-<phase>.json`     kind = "size"   (phase: raw | compressed)
//! - `<engine>.compress.json`         kind = "compress"
//! - `<engine>.query.json`            kind = "query"
//!
//! The TimescaleDB scripts write the same shapes (load, size) plus TSBS's own query result files
//! (`<engine>.query-<type>.tsbs.json`). `tsbench report` reads all of them.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct Host {
    pub os: &'static str,
    pub arch: &'static str,
    pub cpus: usize,
}

pub fn host() -> Host {
    Host {
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        cpus: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(0),
    }
}

pub fn now_rfc3339() -> String {
    let us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0);
    crate::timefmt::ts_string(us - us % 1_000_000)
}

/// Write `value` as pretty JSON to `<dir>/<file>` (when a results directory was given).
pub fn write<T: Serialize>(dir: Option<&Path>, file: &str, value: &T) -> Result<Option<PathBuf>> {
    let Some(dir) = dir else { return Ok(None) };
    std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let p = dir.join(file);
    std::fs::write(&p, serde_json::to_string_pretty(value)? + "\n")
        .with_context(|| format!("cannot write {}", p.display()))?;
    eprintln!("wrote {}", p.display());
    Ok(Some(p))
}

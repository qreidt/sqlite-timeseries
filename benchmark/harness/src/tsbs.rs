//! Reader for TSBS output in the `timescaledb` format.
//!
//! Layout (see TSBS `internal/inputs/generator_data.go` and `pkg/targets/timescaledb/serializer.go`):
//!
//! ```text
//! tags,hostname string,region string,...      <- tag names and types
//! cpu,usage_user,usage_system,...             <- one line per measurement: name, field names
//!                                             <- blank line ends the header
//! tags,hostname=host_0,region=eu-west-1,...   <- tag set of the next point
//! cpu,1451606400000000000,58,2,24,...         <- measurement, timestamp (ns), field values
//! ```
//!
//! Empty values mean NULL.

use anyhow::{bail, Context, Result};
use flate2::read::MultiGzDecoder;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

pub struct Header {
    pub tag_names: Vec<String>,
    pub tag_types: Vec<String>,
    pub measurements: Vec<Measurement>,
}

pub struct Measurement {
    pub name: String,
    pub fields: Vec<String>,
}

/// Open a file (or `-` for stdin), transparently decompressing gzip.
pub fn open_input(path: &str) -> Result<Box<dyn BufRead + Send>> {
    let raw: Box<dyn Read + Send> = if path == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(Path::new(path)).with_context(|| format!("cannot open {path}"))?)
    };
    let mut br = BufReader::with_capacity(1 << 20, raw);
    let is_gzip = {
        let head = br.fill_buf()?;
        head.len() >= 2 && head[0] == 0x1f && head[1] == 0x8b
    };
    if is_gzip {
        Ok(Box::new(BufReader::with_capacity(
            1 << 20,
            MultiGzDecoder::new(br),
        )))
    } else {
        Ok(Box::new(br))
    }
}

pub fn read_header(r: &mut dyn BufRead) -> Result<Header> {
    let mut line = String::new();
    if r.read_line(&mut line)? == 0 {
        bail!("input is empty: expected a TSBS header");
    }
    let first = line.trim_end();
    let mut parts = first.split(',');
    if parts.next() != Some("tags") {
        bail!("not TSBS `timescaledb` format: first line should start with `tags,` (got {first:?}). Generate data with --format=timescaledb");
    }
    let mut tag_names = Vec::new();
    let mut tag_types = Vec::new();
    for p in parts {
        let (n, t) = p
            .split_once(' ')
            .with_context(|| format!("bad tag header entry {p:?}"))?;
        tag_names.push(n.to_string());
        tag_types.push(t.to_string());
    }
    let mut measurements = Vec::new();
    loop {
        line.clear();
        if r.read_line(&mut line)? == 0 {
            bail!("header ended without a blank line");
        }
        let l = line.trim_end();
        if l.is_empty() {
            break;
        }
        let mut cols = l.split(',');
        let name = cols.next().unwrap().to_string();
        let fields: Vec<String> = cols.filter(|c| !c.is_empty()).map(str::to_string).collect();
        measurements.push(Measurement { name, fields });
    }
    if measurements.is_empty() {
        bail!("header lists no measurements");
    }
    Ok(Header {
        tag_names,
        tag_types,
        measurements,
    })
}

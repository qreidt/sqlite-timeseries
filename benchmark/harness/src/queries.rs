//! The TSBS devops query set, written for SQLite.
//!
//! Each query type mirrors TSBS's TimescaleDB generator
//! (`cmd/tsbs_generate_queries/databases/timescaledb/devops.go`): same tables, filters, windows,
//! buckets and aggregates, with three SQLite adaptations:
//!
//! - time literals are integers in the engine's time unit instead of timestamptz strings;
//! - `time_bucket(...)` becomes the engine's `bucket` expression (plain SQLite: integer arithmetic);
//! - `lastpoint` uses a correlated `max(time)` instead of PostgreSQL's `DISTINCT ON` + `LATERAL`.
//!
//! Parameters follow TSBS: random distinct hosts `host_<n>` with n < scale, and a random window of
//! the type's length inside the dataset's time range. They come from a seeded PRNG, so every
//! engine gets identical queries. TSBS's own PRNG is Go's, so TimescaleDB's TSBS query files draw
//! different (but identically distributed) hosts and windows.

use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::engine::Engine;
use crate::manifest::Manifest;
use crate::rng::Rng;
use crate::timefmt::US_PER_SEC;

pub const CPU_METRICS: [&str; 10] = [
    "usage_user",
    "usage_system",
    "usage_idle",
    "usage_nice",
    "usage_iowait",
    "usage_irq",
    "usage_softirq",
    "usage_steal",
    "usage_guest",
    "usage_guest_nice",
];

const HOUR: i64 = 3_600 * US_PER_SEC;
const MINUTE: i64 = 60 * US_PER_SEC;

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// max of `metrics` CPU metrics per minute, for `hosts` random hosts, over a random window.
    SingleGroupby { metrics: usize, hosts: u64 },
    /// max of all CPU metrics per hour, for `hosts` random hosts, over a random window.
    MaxAll { hosts: u64 },
    /// avg of `metrics` CPU metrics per hour and host, all hosts, random 12h window.
    DoubleGroupby { metrics: usize },
    /// max usage_user per minute, last 5 minutes before a random end time.
    GroupbyOrderbyLimit,
    /// rows with usage_user > 90 in a random 12h window, for `hosts` hosts (0 = all).
    HighCpu { hosts: u64 },
    /// latest row per host.
    Lastpoint,
}

#[derive(Clone, Copy, Debug)]
pub struct QueryType {
    pub name: &'static str,
    pub kind: Kind,
    pub window_us: i64,
}

impl QueryType {
    fn min_hosts(&self) -> u64 {
        match self.kind {
            Kind::SingleGroupby { hosts, .. }
            | Kind::MaxAll { hosts }
            | Kind::HighCpu { hosts } => hosts,
            _ => 1,
        }
    }
}

/// The TSBS devops use-case matrix (`cmd/tsbs_generate_queries/main.go`).
pub fn catalog() -> Vec<QueryType> {
    use Kind::*;
    let q = |name, kind, window_us| QueryType {
        name,
        kind,
        window_us,
    };
    vec![
        q(
            "single-groupby-1-1-1",
            SingleGroupby {
                metrics: 1,
                hosts: 1,
            },
            HOUR,
        ),
        q(
            "single-groupby-1-1-12",
            SingleGroupby {
                metrics: 1,
                hosts: 1,
            },
            12 * HOUR,
        ),
        q(
            "single-groupby-1-8-1",
            SingleGroupby {
                metrics: 1,
                hosts: 8,
            },
            HOUR,
        ),
        q(
            "single-groupby-5-1-1",
            SingleGroupby {
                metrics: 5,
                hosts: 1,
            },
            HOUR,
        ),
        q(
            "single-groupby-5-1-12",
            SingleGroupby {
                metrics: 5,
                hosts: 1,
            },
            12 * HOUR,
        ),
        q(
            "single-groupby-5-8-1",
            SingleGroupby {
                metrics: 5,
                hosts: 8,
            },
            HOUR,
        ),
        q("cpu-max-all-1", MaxAll { hosts: 1 }, 8 * HOUR),
        q("cpu-max-all-8", MaxAll { hosts: 8 }, 8 * HOUR),
        q("cpu-max-all-32-24", MaxAll { hosts: 32 }, 24 * HOUR),
        q("double-groupby-1", DoubleGroupby { metrics: 1 }, 12 * HOUR),
        q("double-groupby-5", DoubleGroupby { metrics: 5 }, 12 * HOUR),
        q(
            "double-groupby-all",
            DoubleGroupby { metrics: 10 },
            12 * HOUR,
        ),
        q("groupby-orderby-limit", GroupbyOrderbyLimit, HOUR),
        q("high-cpu-all", HighCpu { hosts: 0 }, 12 * HOUR),
        q("high-cpu-1", HighCpu { hosts: 1 }, 12 * HOUR),
        q("lastpoint", Lastpoint, 0),
    ]
}

/// Query types that fit the dataset, plus the reasons the others were skipped.
/// Query types that fit, and the skipped ones with the reason.
pub type Applicable = (Vec<QueryType>, Vec<(QueryType, String)>);

pub fn applicable(m: &Manifest) -> Result<Applicable> {
    let Some(cpu) = m.table("cpu") else {
        bail!("the devops query set needs a `cpu` table (use case cpu-only or devops); this dataset has: {}", m.tables.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(", "))
    };
    if let Some(missing) = CPU_METRICS
        .iter()
        .find(|f| !cpu.fields.iter().any(|x| x == *f))
    {
        bail!("cpu table is missing field {missing}");
    }
    if !m.tags.columns.iter().any(|c| c.name == "hostname") {
        bail!("tags have no hostname column");
    }
    let (start, end) = m.query_range_us()?;
    let range = end - start;
    let scale = m.scale();
    let mut ok = Vec::new();
    let mut skipped = Vec::new();
    for qt in catalog() {
        if qt.window_us > 0 && range <= qt.window_us {
            skipped.push((
                qt,
                format!("needs more than {}h of data", qt.window_us / HOUR),
            ));
        } else if qt.min_hosts() > scale {
            skipped.push((
                qt,
                format!("needs at least {} hosts (scale is {scale})", qt.min_hosts()),
            ));
        } else {
            ok.push(qt);
        }
    }
    Ok((ok, skipped))
}

pub struct Ctx<'a> {
    pub engine: &'a Engine,
    pub start_us: i64,
    pub end_us: i64,
    pub scale: u64,
}

impl Ctx<'_> {
    fn lit(&self, us: i64) -> String {
        self.engine.unit.us_to_unit(us).to_string()
    }

    fn window(&self, rng: &mut Rng, len: i64) -> (i64, i64) {
        let s = self.start_us + rng.below((self.end_us - len - self.start_us) as u64) as i64;
        (s, s + len)
    }

    fn hosts_filter(&self, rng: &mut Rng, n: u64) -> String {
        let names: Vec<String> = rng
            .sample_distinct(n, self.scale)
            .iter()
            .map(|i| format!("'host_{i}'"))
            .collect();
        format!(
            "tags_id IN (SELECT id FROM tags WHERE hostname IN ({}))",
            names.join(",")
        )
    }
}

pub fn generate(qt: &QueryType, c: &Ctx, rng: &mut Rng) -> String {
    let minute = c.engine.bucket_expr("time", MINUTE);
    let hour = c.engine.bucket_expr("time", HOUR);
    match qt.kind {
        Kind::SingleGroupby { metrics, hosts } => {
            let (s, e) = c.window(rng, qt.window_us);
            let hosts = c.hosts_filter(rng, hosts);
            let sel: Vec<String> = CPU_METRICS[..metrics].iter().map(|m| format!("max({m}) AS max_{m}")).collect();
            format!(
                "SELECT {minute} AS minute, {} FROM cpu WHERE {hosts} AND time >= {} AND time < {} GROUP BY minute ORDER BY minute ASC",
                sel.join(", "),
                c.lit(s),
                c.lit(e)
            )
        }
        Kind::MaxAll { hosts } => {
            let (s, e) = c.window(rng, qt.window_us);
            let hosts = c.hosts_filter(rng, hosts);
            let sel: Vec<String> = CPU_METRICS.iter().map(|m| format!("max({m}) AS max_{m}")).collect();
            format!(
                "SELECT {hour} AS hour, {} FROM cpu WHERE {hosts} AND time >= {} AND time < {} GROUP BY hour ORDER BY hour",
                sel.join(", "),
                c.lit(s),
                c.lit(e)
            )
        }
        Kind::DoubleGroupby { metrics } => {
            let (s, e) = c.window(rng, qt.window_us);
            let avg: Vec<String> = CPU_METRICS[..metrics].iter().map(|m| format!("avg({m}) AS mean_{m}")).collect();
            let means: Vec<String> = CPU_METRICS[..metrics].iter().map(|m| format!("mean_{m}")).collect();
            format!(
                "WITH cpu_avg AS (SELECT {hour} AS hour, tags_id, {} FROM cpu WHERE time >= {} AND time < {} GROUP BY 1, 2) \
                 SELECT hour, tags.hostname, {} FROM cpu_avg JOIN tags ON cpu_avg.tags_id = tags.id ORDER BY hour, tags.hostname",
                avg.join(", "),
                c.lit(s),
                c.lit(e),
                means.join(", ")
            )
        }
        Kind::GroupbyOrderbyLimit => {
            let (_, e) = c.window(rng, qt.window_us);
            format!("SELECT {minute} AS minute, max(usage_user) FROM cpu WHERE time < {} GROUP BY minute ORDER BY minute DESC LIMIT 5", c.lit(e))
        }
        Kind::HighCpu { hosts } => {
            let (s, e) = c.window(rng, qt.window_us);
            let filter = if hosts == 0 { String::new() } else { format!(" AND {}", c.hosts_filter(rng, hosts)) };
            format!("SELECT * FROM cpu WHERE usage_user > 90.0 AND time >= {} AND time < {}{filter}", c.lit(s), c.lit(e))
        }
        Kind::Lastpoint => "SELECT t.*, c.* FROM tags t JOIN cpu c ON c.tags_id = t.id AND c.time = (SELECT max(time) FROM cpu WHERE tags_id = t.id) ORDER BY t.hostname".to_string(),
    }
}

#[derive(clap::Args)]
pub struct ListArgs {
    #[arg(long)]
    pub data_dir: PathBuf,
}

/// Print applicable query types to stdout (one per line); skipped ones go to stderr.
pub fn list(a: ListArgs) -> Result<()> {
    let m = Manifest::load(&a.data_dir)?;
    let (ok, skipped) = applicable(&m)?;
    for qt in ok {
        println!("{}", qt.name);
    }
    for (qt, why) in skipped {
        eprintln!("skipping {}: {why}", qt.name);
    }
    Ok(())
}

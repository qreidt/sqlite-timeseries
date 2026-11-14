# SQLite Time-Series Extension — Requirements

Oct 16, 2026 · @Caio · exported Oct 27, 2026 from the [shared doc](https://claude.ai/artifact/FS2Dmnn8bdnQ9gfdThMWcW)

Requirements for a SQLite loadable extension, written in Rust, that brings TimescaleDB-style time-series storage to SQLite. There is no code yet: these files define what must be built.

## Contents

| # | Document | What it covers |
| --- | --- | --- |
| 1 | [SQLite constraints](01-sqlite-constraints.md) | SQLite facts that shape the design |
| 2 | [Core conventions](02-core-conventions.md) | Time values, intervals, catalog and naming |
| 3 | [Query functions](03-query-functions.md) | `interval`, `time_bucket`, gap filling, `locf`, `interpolate`, `first`, `last` (QF) |
| 4 | [Hypertables](04-hypertables.md) | Registration, routing, pruning, indexes (HT) |
| 5 | [Data lifecycle](05-data-lifecycle.md) | Retention and compression (LC) |
| 6 | [Continuous aggregates](06-continuous-aggregates.md) | Materialized rollups and real-time reads (CA) |
| 7 | [Storage engine techniques](07-storage-engine.md) | Append-only mode, LSM layer, block metadata, bloom filters, downsampling (SE) |
| 8 | [Block files](08-block-files.md) | Sealed blocks stored as files beside the database (BF) |
| 9 | [Policies, jobs and introspection](09-policies-jobs-introspection.md) | Job runner and introspection views (JB) |
| 10 | [Non-functional requirements](10-non-functional.md) | Correctness, atomicity, speed, security (NF) |
| 11 | [Rust build, packaging and distribution](11-rust-build-packaging.md) | Binding crate and build targets (BD) |
| 12 | [Testing and verification](12-testing.md) | Differential, fuzz and crash tests (TS) |
| 13 | [Phasing](13-phasing.md) | Build order and gates |
| 14 | [Open decisions](14-open-decisions.md) | Choices still open, with proposed defaults |
| 15 | [Risks to spike first](15-risks.md) | Assumptions to test before building |
| — | [Decision records](decisions/README.md) | One record per decision, with the Linear ticket that settles it |
| — | [Spikes](spikes/README.md) | Phase 0 experiments, one folder per risk, with results and verdicts |
| — | [References](references.md) | Sources behind the facts in these documents |

Work is tracked in the Linear project [SQLite Time-Series Extension](https://linear.app/cqr/project/sqlite-time-series-extension-1c6ca96e21da).

## Purpose, scope and non-goals

The extension must give a plain SQLite database TimescaleDB-style *storage*: automatic time partitioning, cheap retention, columnar compression and continuous aggregates. Query helpers alone are not the goal, because existing projects (tslite, sqlean-time) already cover them.

**Who it is for.** Developers who embed SQLite in an app, an edge device or a local analytics tool and need to keep months of append-heavy metrics in one file without hand-rolling partition tables.

**In scope for v1**

- Query functions: `interval`, `time_bucket`, gap filling, `locf`, `interpolate`, `first`, `last`.
- Hypertables: registration, chunk catalog, insert routing, automatic chunk creation, chunk pruning, per-chunk indexes.
- Data lifecycle: `drop_chunks`, chunk compression with transparent reads.
- Continuous aggregates with incremental and real-time reads.
- Policies stored in the database and executed by an app-driven `run_jobs()`.
- Introspection views.
- Storage engine layer: append-only mode, sealed blocks stored as files beside the database (statistics stay inside it), a write buffer, compaction and downsampling tiers.

**Non-goals for v1**

- Syntax compatibility with TimescaleDB or PostgreSQL. Features are exposed as SQL functions and virtual tables, not new DDL.
- Background threads inside the extension. The host app decides when jobs run.
- Distributed or multi-file hypertables.
- Continuous aggregates defined on other continuous aggregates.
- Approximate statistics (percentile sketches, HyperLogLog) and other hyperfunctions.
- Full `ALTER TABLE` parity on hypertables.

## How to read the requirements

Each requirement has an ID (`QF`, `HT`, `LC`, `CA`, `SE`, `BF`, `JB`, `NF`, `BD`, `TS`) and a priority: **Must** for v1, **Should** for v1 if time allows, **Could** for later. Numbers marked *proposed* are targets to confirm, not measurements.

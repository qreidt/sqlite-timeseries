# Phasing

[← Contents](README.md)

Build in six phases, starting with throwaway spikes, because two of the [risks](15-risks.md) decide whether the virtual-table design works at all.

**Spikes settle the design before any storage code is written.** Each phase closes with its own tests.

```mermaid
flowchart TD
    p0["0 · Spikes<br/>Run the eleven risk spikes and settle open decisions 1 to 5.<br/>Phase 1 can run alongside, since it needs no storage."]
    gA{"Gate: risks 1 and 2 pass,<br/>or their fallbacks are chosen"}
    p1["1 · Query functions<br/>interval, time_bucket, gapfill, locf, interpolate, first, last<br/>Golden-output tests against TimescaleDB (TS-06)"]
    p2["2 · Hypertables<br/>Chunks, routing, pruning, sealing, append-only mode, drop_chunks<br/>Differential tests (TS-01) and crash tests (TS-04)"]
    gB{"Gate: differential and crash tests<br/>green on every SQLite version in CI"}
    p3["3 · Jobs and introspection<br/>Policies, run_jobs(), retention policy, introspection views"]
    p4["4 · Continuous aggregates<br/>Invalidation log, incremental refresh, real-time views<br/>Downsampling tiers built on refresh and retention policies"]
    p5["5 · Blocks and compression<br/>Sealed blocks: encodings, zone maps, bloom filters, compaction<br/>Write buffer too; last, because the block format is permanent"]
    gC{"Release gate: blob format v1 frozen,<br/>decoders fuzzed clean"}
    p0 --> gA --> p1 --> p2 --> gB --> p3 --> p4 --> p5 --> gC
```

Continuous aggregates come before compression: they deliver the bigger query win, and compression's blob format cannot change once a release ships it.

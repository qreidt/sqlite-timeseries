#!/usr/bin/env bash
# One-time setup: build the tools image (TSBS + timescaledb-parallel-copy), pull TimescaleDB and
# build the tsbench harness.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

command -v docker >/dev/null || die "Docker is required (Docker Desktop with WSL integration on Windows)"
docker compose version >/dev/null 2>&1 || die "Docker Compose v2 is required (the 'docker compose' command)"
command -v cargo >/dev/null || die "Rust is required: https://rustup.rs"

log "Building the tools image (TSBS + timescaledb-parallel-copy)"
compose --profile tools build tools

log "Pulling $TIMESCALE_IMAGE"
compose pull timescaledb

log "Building tsbench (Rust, release mode)"
(cd "$BENCH_DIR/harness" && cargo build --release)
[[ -x "$TSBENCH" ]] || die "tsbench was built, but not at $TSBENCH. If CARGO_TARGET_DIR is set, set TSBENCH to the binary's path"

log "Setup complete. Next: bash scripts/run-all.sh small"

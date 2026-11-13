#!/usr/bin/env bash
# Benchmark TimescaleDB in Docker: load the same CSV files, size, TSBS queries, compress, size
# again, TSBS queries on compressed data. The work happens in docker/tools/timescaledb-bench.sh.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tsbench
require_dataset
ensure_results_dir
print_config

log "Writing TimescaleDB SQL for this dataset"
"$TSBENCH" prepare-timescaledb --data-dir "$DATA_DIR" --chunk-interval "$TSDB_CHUNK_INTERVAL"

log "Starting TimescaleDB ($TIMESCALE_IMAGE)"
compose up -d --wait timescaledb

export DATASET USE_CASE SCALE TS_START TS_END QUERIES QUERY_WARMUP QUERY_SEED QUERY_TYPES \
  TSDB_LOAD_WORKERS TSDB_BATCH_SIZE TSDB_DIRECT_COMPRESS TIMESCALE_IMAGE
status=0
tools -e DATASET -e USE_CASE -e SCALE -e TS_START -e TS_END -e QUERIES -e QUERY_WARMUP -e QUERY_SEED -e QUERY_TYPES \
  -e TSDB_LOAD_WORKERS -e TSDB_BATCH_SIZE -e TSDB_DIRECT_COMPRESS -e TIMESCALE_IMAGE -e HOME=/tmp \
  tools bash /bench/timescaledb-bench.sh || status=$?

if [[ "$TSDB_KEEP_RUNNING" != true ]]; then
  log "Stopping TimescaleDB (set TSDB_KEEP_RUNNING=true to keep it running)"
  compose stop timescaledb
fi
[[ $status -eq 0 ]] || die "TimescaleDB benchmark failed (exit $status)"
log "TimescaleDB results in $RESULTS_DIR"

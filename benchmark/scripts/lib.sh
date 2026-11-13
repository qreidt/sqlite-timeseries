#!/usr/bin/env bash
# Shared configuration and helpers, sourced by the other scripts.
# shellcheck disable=SC2034  # variables defined here are used by the scripts that source it
set -euo pipefail

BENCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

CONFIG_VARS=(
  PRESET USE_CASE SCALE TS_START TS_END LOG_INTERVAL SEED WRITE_PARQUET
  QUERIES QUERY_WARMUP QUERY_SEED QUERY_TYPES
  SQLITE_ENGINES SQLITE_BATCH_SIZE EXTENSION_PATH
  TIMESCALE_IMAGE TSDB_LOAD_WORKERS TSDB_BATCH_SIZE TSDB_CHUNK_INTERVAL TSDB_DIRECT_COMPRESS
  TSDB_KEEP_RUNNING TSDB_PORT TS_TUNE_MEMORY TS_TUNE_NUM_CPUS POSTGRES_PASSWORD
  DATA_ROOT DB_ROOT RESULTS_ROOT TSBENCH
)

log() { printf '\n==> %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

# Remember values set in the environment; they win over the config files and presets.
_overridden=()
for _v in "${CONFIG_VARS[@]}"; do
  if [[ -n "${!_v+x}" ]]; then
    printf -v "_env_${_v}" '%s' "${!_v}"
    _overridden+=("$_v")
  fi
done

# shellcheck source=../config.env
source "$BENCH_DIR/config.env"
if [[ -f "$BENCH_DIR/config.local.env" ]]; then
  # shellcheck disable=SC1091
  source "$BENCH_DIR/config.local.env"
fi

if [[ -n "${_env_PRESET+x}" ]]; then PRESET="$_env_PRESET"; fi
case "${PRESET:-}" in
  "") ;;
  small)       SCALE=100;    TS_START=2016-01-01T00:00:00Z; TS_END=2016-01-02T00:00:00Z ;;  # 864K rows
  main)        SCALE=4000;   TS_START=2016-01-01T00:00:00Z; TS_END=2016-01-04T00:00:00Z ;;  # 103.7M rows
  cardinality) SCALE=100000; TS_START=2016-01-01T00:00:00Z; TS_END=2016-01-01T01:30:00Z ;;  # 54M rows, 100K series
  *) die "unknown preset '$PRESET' (use small, main or cardinality)" ;;
esac

for _v in ${_overridden[@]+"${_overridden[@]}"}; do
  _src="_env_${_v}"
  printf -v "$_v" '%s' "${!_src}"
done

abs_path() { case "$1" in /*) printf '%s' "$1" ;; *) printf '%s' "$BENCH_DIR/$1" ;; esac; }

DATASET="${USE_CASE}_s${SCALE}_${TS_START//[-:]/}_${TS_END//[-:]/}_${LOG_INTERVAL}_seed${SEED}"
DATA_ROOT_ABS="$(abs_path "$DATA_ROOT")"
DB_ROOT_ABS="$(abs_path "$DB_ROOT")"
RESULTS_ROOT_ABS="$(abs_path "$RESULTS_ROOT")"
DATA_DIR="$DATA_ROOT_ABS/$DATASET"
DB_DIR="$DB_ROOT_ABS/$DATASET"
RESULTS_DIR="$RESULTS_ROOT_ABS/$DATASET"
TSBENCH="${TSBENCH:-$BENCH_DIR/harness/target/release/tsbench}"

# Read by docker-compose.yml.
export DATA_ROOT_ABS RESULTS_ROOT_ABS TIMESCALE_IMAGE TSDB_PORT TS_TUNE_MEMORY TS_TUNE_NUM_CPUS POSTGRES_PASSWORD

compose() { docker compose -f "$BENCH_DIR/docker-compose.yml" "$@"; }

# Run a command in the tools container (TSBS, timescaledb-parallel-copy, psql) as the current user,
# so files it writes into data/ and results/ stay yours.
tools() {
  mkdir -p "$DATA_ROOT_ABS" "$RESULTS_ROOT_ABS"
  compose --profile tools run --rm -T --user "$(id -u):$(id -g)" "$@"
}

require_tsbench() {
  [[ -x "$TSBENCH" ]] || die "tsbench is not built ($TSBENCH). Run: bash scripts/setup.sh"
}

require_dataset() {
  [[ -f "$DATA_DIR/manifest.json" ]] || die "dataset $DATASET not found in $DATA_ROOT_ABS. Run: bash scripts/generate.sh"
}

# Results for a dataset live next to a copy of its manifest, which `tsbench report` reads.
ensure_results_dir() {
  mkdir -p "$RESULTS_DIR"
  cp "$DATA_DIR/manifest.json" "$RESULTS_DIR/manifest.json"
}

print_config() {
  log "Dataset $DATASET"
  printf '    use case %s, %s hosts, %s to %s every %s, seed %s\n' "$USE_CASE" "$SCALE" "$TS_START" "$TS_END" "$LOG_INTERVAL" "$SEED" >&2
  local p
  for p in "$DATA_ROOT_ABS" "$DB_ROOT_ABS"; do
    if [[ "$p" == /mnt/[a-z]/* ]]; then
      printf '    warning: %s is on a Windows drive; WSL reaches it through a slow file-sharing layer,\n' "$p" >&2
      printf '    which skews SQLite and Docker numbers. Set DATA_ROOT and DB_ROOT to a Linux path (see README).\n' >&2
    fi
  done
}

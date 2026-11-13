#!/usr/bin/env bash
# Benchmark SQLite engines: load, size, queries; for engines with a compression step also
# compress, size again and query the compressed data.
#
# Usage: bash scripts/bench-sqlite.sh [engine ...]   (default: SQLITE_ENGINES from config.env)
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tsbench
require_dataset
ensure_results_dir
print_config

if [[ $# -gt 0 ]]; then
  engines=("$@")
else
  read -r -a engines <<< "$SQLITE_ENGINES"
fi

has_queries=false
if [[ -n "$("$TSBENCH" list-queries --data-dir "$DATA_DIR" 2>/dev/null)" ]]; then
  has_queries=true
else
  log "No TSBS query types fit this dataset; skipping queries"
fi

query_args=(--queries "$QUERIES" --warmup "$QUERY_WARMUP" --seed "$QUERY_SEED")
read -r -a selected_types <<< "$QUERY_TYPES"
if [[ ${#selected_types[@]} -gt 0 ]]; then query_args+=(--types "$(IFS=,; echo "${selected_types[*]}")"); fi

mkdir -p "$DB_DIR"
for e in "${engines[@]}"; do
  conf="$BENCH_DIR/engines/$e.toml"
  [[ -f "$conf" ]] || die "no engine file $conf"
  db="$DB_DIR/$e.db"
  common=(--engine "$conf" --data-dir "$DATA_DIR" --db "$db" --results-dir "$RESULTS_DIR")
  # Only engines that declare an `extension` key get EXTENSION_PATH.
  if grep -Eq '^[[:space:]]*extension[[:space:]]*=' "$conf"; then
    [[ -n "$EXTENSION_PATH" ]] || die "engine $e loads an extension: set EXTENSION_PATH (config.env or environment)"
    common+=(--extension "$(abs_path "$EXTENSION_PATH")")
  fi
  rm -f "$RESULTS_DIR/$e".*.json "$RESULTS_DIR/$e"-compressed.*.json

  log "[$e] load"
  "$TSBENCH" load "${common[@]}" --batch-size "$SQLITE_BATCH_SIZE"

  log "[$e] size"
  "$TSBENCH" size "${common[@]}" --phase raw

  if $has_queries; then
    log "[$e] queries ($QUERIES per type)"
    "$TSBENCH" query "${common[@]}" "${query_args[@]}"
  fi

  log "[$e] compress"
  "$TSBENCH" compress "${common[@]}" --if-supported
  if [[ -f "$RESULTS_DIR/$e.compress.json" ]] && $has_queries; then
    log "[$e] queries on compressed data"
    "$TSBENCH" query "${common[@]}" "${query_args[@]}" --label compressed
  fi
done

log "SQLite results in $RESULTS_DIR"

#!/usr/bin/env bash
# Runs inside the tools container; started by scripts/bench-timescaledb.sh.
#
#  1. load the dataset's CSV files with timescaledb-parallel-copy (once per TSDB_LOAD_WORKERS value)
#  2. ANALYZE, then measure size
#  3. run the TSBS query set
#  4. convert every chunk to the columnstore, measure size again
#  5. run the TSBS query set on the compressed data
set -euo pipefail

: "${DATASET:?}" "${USE_CASE:?}" "${SCALE:?}" "${TS_START:?}" "${TS_END:?}"
: "${QUERIES:?}" "${QUERY_WARMUP:?}" "${QUERY_SEED:?}"
: "${TSDB_LOAD_WORKERS:?}" "${TSDB_BATCH_SIZE:?}" "${TSDB_DIRECT_COMPRESS:?}"
: "${PGHOST:?}" "${PGUSER:?}" "${PGPASSWORD:?}"

D="/data/$DATASET"          # dataset: CSV files + manifest.json
T="$D/timescaledb"          # SQL written by `tsbench prepare-timescaledb`
Q="$D/tsbs-queries"         # TSBS query files (generated once, reused)
R="/results/$DATASET"
DB=benchmark
CONN="host=$PGHOST user=$PGUSER password=$PGPASSWORD dbname=$DB sslmode=disable"

log() { printf '\n--> %s\n' "$*" >&2; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }
now() { date +%s.%N; }
elapsed() { awk -v a="$1" -v b="$2" 'BEGIN { printf "%.3f", b - a }'; }
ratio() { awk -v a="$1" -v b="$2" 'BEGIN { printf "%.1f", (b > 0 ? a / b : 0) }'; }
psql_admin() { psql -X -q -v ON_ERROR_STOP=1 -d postgres "$@"; }
psql_db() { psql -X -q -v ON_ERROR_STOP=1 -d "$DB" "$@"; }

for f in schema.sql compress.sql size.sql tables.tsv query-types.txt; do
  [[ -f "$T/$f" ]] || die "missing $T/$f (run tsbench prepare-timescaledb)"
done
mkdir -p "$R" "$Q"
rm -f "$R"/timescaledb*.json

for _ in $(seq 1 60); do pg_isready -q -d postgres && break; sleep 1; done
pg_isready -q -d postgres || die "TimescaleDB is not accepting connections"

pcopy_flags=()
[[ "$TSDB_DIRECT_COMPRESS" == true ]] || pcopy_flags+=(--disable-direct-compress)

load() {
  local workers="$1" engine="timescaledb-w$1"
  log "Load with $workers worker(s)"
  psql_admin -c "DROP DATABASE IF EXISTS $DB" -c "CREATE DATABASE $DB"
  psql_db -f "$T/schema.sql" >/dev/null

  local t0 t1 tags_secs secs rows=0 metrics=0
  t0=$(now)
  psql_db -c "\\copy tags($(head -n1 "$D/tags.csv")) FROM '$D/tags.csv' WITH (FORMAT csv, HEADER true)"
  psql_db -c "SELECT setval(pg_get_serial_sequence('tags', 'id'), (SELECT max(id) FROM tags))" >/dev/null
  t1=$(now)
  tags_secs=$(elapsed "$t0" "$t1")

  t0=$(now)
  while IFS=$'\t' read -r name csv nrows nmetrics; do
    timescaledb-parallel-copy --connection "$CONN" --table "$name" --file "$D/$csv" --skip-header \
      --columns "$(head -n1 "$D/$csv")" --copy-options "CSV" \
      --workers "$workers" --batch-size "$TSDB_BATCH_SIZE" --reporting-period 30s --verbose \
      ${pcopy_flags[@]+"${pcopy_flags[@]}"} < /dev/null
    rows=$((rows + nrows))
    metrics=$((metrics + nmetrics))
  done < "$T/tables.tsv"
  t1=$(now)
  secs=$(elapsed "$t0" "$t1")

  # Every row must be there, or the numbers mean nothing.
  while IFS=$'\t' read -r name _ nrows _; do
    local got
    got=$(psql_db -At -c "SELECT count(*) FROM $name" < /dev/null)
    [[ "$got" == "$nrows" ]] || die "$name has $got rows after loading, expected $nrows"
  done < "$T/tables.tsv"

  t0=$(now)
  psql_db -c "ANALYZE"
  t1=$(now)
  local post_secs
  post_secs=$(elapsed "$t0" "$t1")

  printf '{"kind":"load","engine":"%s","dataset":"%s","rows":%s,"metrics":%s,"seconds":%s,"rows_per_sec":%s,"metrics_per_sec":%s,"workers":%s,"batch_size":%s,"tags_seconds":%s,"post_load_seconds":%s,"direct_compress":%s,"image":"%s","tsbs_commit":"%s","parallel_copy":"%s"}\n' \
    "$engine" "$DATASET" "$rows" "$metrics" "$secs" "$(ratio "$rows" "$secs")" "$(ratio "$metrics" "$secs")" \
    "$workers" "$TSDB_BATCH_SIZE" "$tags_secs" "$post_secs" "$TSDB_DIRECT_COMPRESS" "${TIMESCALE_IMAGE:-}" "${TSBS_COMMIT:-}" \
    "$(timescaledb-parallel-copy --version 2>&1 | head -n1 | tr -d '"')" > "$R/$engine.load.json"
  log "Loaded $rows rows in ${secs}s ($(ratio "$rows" "$secs") rows/s)"
}

size() {
  log "Size ($1)"
  psql_db -At -v engine=timescaledb -v phase="$1" -f "$T/size.sql" > "$R/timescaledb.size-$1.json"
  cat "$R/timescaledb.size-$1.json"
}

# Query types to run: every type that fits the dataset, or the QUERY_TYPES subset of them.
query_types() {
  local qt
  if [[ -z "${QUERY_TYPES:-}" ]]; then
    cat "$T/query-types.txt"
    return
  fi
  for qt in $QUERY_TYPES; do
    if grep -qx "$qt" "$T/query-types.txt"; then echo "$qt"; else echo "skipping $qt: does not fit this dataset" >&2; fi
  done
}

run_queries() {
  local engine="$1" qt f
  while read -r qt; do
    [[ -n "$qt" ]] || continue
    f="$Q/timescaledb_${qt}_n${QUERIES}_w${QUERY_WARMUP}_seed${QUERY_SEED}.gz"
    if [[ ! -s "$f" ]]; then
      tsbs_generate_queries --format=timescaledb --use-case="$USE_CASE" --seed="$QUERY_SEED" --scale="$SCALE" \
        --timestamp-start="$TS_START" --timestamp-end="$TS_END" \
        --queries=$((QUERIES + QUERY_WARMUP)) --query-type="$qt" < /dev/null | gzip > "$f.tmp"
      mv "$f.tmp" "$f"
    fi
    log "[$engine] $qt"
    gunzip -c "$f" | tsbs_run_queries_timescaledb --hosts="$PGHOST" --user="$PGUSER" --pass="$PGPASSWORD" \
      --postgres="sslmode=disable" --db-name="$DB" --workers=1 --burn-in="$QUERY_WARMUP" --print-interval=0 \
      --results-file="$R/$engine.query-$qt.tsbs.json"
  done < <(query_types)
}

for w in $TSDB_LOAD_WORKERS; do
  load "$w"
done

psql_db -At -c "SELECT json_build_object('kind', 'meta', 'engine', 'timescaledb', 'settings',
  (SELECT json_object_agg(name, setting) FROM pg_settings WHERE name IN
    ('shared_buffers', 'effective_cache_size', 'work_mem', 'maintenance_work_mem', 'max_worker_processes',
     'max_parallel_workers', 'max_parallel_workers_per_gather', 'synchronous_commit', 'wal_level', 'max_wal_size')),
  'server_version', current_setting('server_version'),
  'timescaledb_version', (SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'))" > "$R/timescaledb.meta.json"

size raw
if [[ -s "$T/query-types.txt" ]]; then run_queries timescaledb; fi

log "Compress (convert every chunk to the columnstore)"
t0=$(now)
psql_db -f "$T/compress.sql" >/dev/null
t1=$(now)
secs=$(elapsed "$t0" "$t1")
rows=$(awk -F'\t' '{ s += $3 } END { print s + 0 }' "$T/tables.tsv")
printf '{"kind":"compress","engine":"timescaledb","dataset":"%s","seconds":%s,"rows":%s,"rows_per_sec":%s}\n' \
  "$DATASET" "$secs" "$rows" "$(ratio "$rows" "$secs")" > "$R/timescaledb.compress.json"
log "Compressed in ${secs}s"

size compressed
if [[ -s "$T/query-types.txt" ]]; then run_queries timescaledb-compressed; fi

log "Done"

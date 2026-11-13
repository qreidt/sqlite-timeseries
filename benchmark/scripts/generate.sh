#!/usr/bin/env bash
# Generate the dataset with TSBS, then convert it to CSV + Parquet + manifest.json.
# Both steps are skipped when their output exists; FORCE=1 redoes them.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tsbench
print_config
mkdir -p "$DATA_DIR"

RAW="$DATA_DIR/tsbs-data.gz"
if [[ -s "$RAW" && "${FORCE:-0}" != 1 ]]; then
  log "Reusing TSBS output $RAW"
else
  log "Generating data with TSBS"
  # shellcheck disable=SC2016  # expanded inside the container
  tools tools sh -c 'tsbs_generate_data --format=timescaledb --use-case="$1" --seed="$2" --scale="$3" \
      --timestamp-start="$4" --timestamp-end="$5" --log-interval="$6" | gzip -1 > "$7.tmp" && mv "$7.tmp" "$7"' \
    sh "$USE_CASE" "$SEED" "$SCALE" "$TS_START" "$TS_END" "$LOG_INTERVAL" "/data/$DATASET/tsbs-data.gz"
fi

if [[ -f "$DATA_DIR/manifest.json" && "${FORCE:-0}" != 1 ]]; then
  log "Reusing converted dataset in $DATA_DIR"
else
  log "Converting to CSV$([[ "$WRITE_PARQUET" == true ]] && printf ' + Parquet')"
  extra=()
  [[ "$WRITE_PARQUET" == true ]] || extra+=(--no-parquet)
  "$TSBENCH" convert --input "$RAW" --out-dir "$DATA_DIR" --dataset "$DATASET" \
    --use-case "$USE_CASE" --scale "$SCALE" --seed "$SEED" \
    --timestamp-start "$TS_START" --timestamp-end "$TS_END" --log-interval "$LOG_INTERVAL" \
    ${extra[@]+"${extra[@]}"}
fi

log "Dataset ready: $DATA_DIR"

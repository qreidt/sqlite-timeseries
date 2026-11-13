#!/usr/bin/env bash
# Full run: generate data, benchmark the SQLite engines and TimescaleDB, write the report.
#
# Usage: bash scripts/run-all.sh [small | main | cardinality]
#   small        100 hosts x 1 day       (864K rows, a few minutes)
#   main         4,000 hosts x 3 days    (103.7M rows, the size most published TSBS results use)
#   cardinality  100,000 hosts x 90 min  (54M rows; only query types with 1-hour windows fit)
set -euo pipefail
if [[ $# -gt 0 ]]; then export PRESET="$1"; fi
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "$here/generate.sh"
bash "$here/bench-sqlite.sh"
bash "$here/bench-timescaledb.sh"
bash "$here/report.sh"

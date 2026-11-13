#!/usr/bin/env bash
# Combine every result for the current dataset into results/<dataset>/report.md.
source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"
require_tsbench
[[ -d "$RESULTS_DIR" ]] || die "no results for $DATASET yet"
"$TSBENCH" report --results-dir "$RESULTS_DIR"

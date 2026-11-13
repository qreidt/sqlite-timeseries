# Decision records

[← Contents](../README.md)

One file per architectural decision. A record starts as *Proposed*, becomes *Accepted* when its Linear ticket closes, and is never rewritten afterwards: a later change gets a new record marked *Supersedes NNNN*, and the old one is marked *Superseded by NNNN*.

| # | Decision | Status | Settled by |
| --- | --- | --- | --- |
| [0001](0001-time-column-storage.md) | Time column storage | Proposed | [CQR-131](https://linear.app/cqr/issue/CQR-131) |
| [0002](0002-bare-integer-unit.md) | Unit for bare INTEGER input to standalone functions | Proposed | [CQR-132](https://linear.app/cqr/issue/CQR-132) |
| [0003](0003-binding-crate.md) | Binding crate | Proposed | [CQR-133](https://linear.app/cqr/issue/CQR-133) |
| [0004](0004-main-read-path.md) | Main read path | Proposed | [CQR-134](https://linear.app/cqr/issue/CQR-134) |
| [0005](0005-refresh-scan-restriction.md) | How refresh restricts the raw scan | Proposed | [CQR-135](https://linear.app/cqr/issue/CQR-135) |
| [0006](0006-chunk-shadow-tables.md) | Chunk tables as shadow tables | Proposed | [CQR-145](https://linear.app/cqr/issue/CQR-145) |
| [0007](0007-omitted-columns.md) | Omitted columns on insert | Proposed | [CQR-146](https://linear.app/cqr/issue/CQR-146) |
| [0008](0008-vtab-trust-flags.md) | Trust flags on the hypertable module | Proposed | [CQR-154](https://linear.app/cqr/issue/CQR-154) |
| [0009](0009-compressed-row-changes.md) | UPDATE and DELETE on compressed rows | Proposed | [CQR-173](https://linear.app/cqr/issue/CQR-173) |
| [0010](0010-run-jobs-call-shape.md) | `run_jobs()` call shape | Proposed | [CQR-124](https://linear.app/cqr/issue/CQR-124) |
| [0011](0011-extension-name.md) | Extension and library name | Proposed | [CQR-136](https://linear.app/cqr/issue/CQR-136) |
| [0012](0012-license.md) | License | Proposed | [CQR-136](https://linear.app/cqr/issue/CQR-136) |
| [0013](0013-block-directory-name.md) | Block directory name | Proposed | [CQR-171](https://linear.app/cqr/issue/CQR-171) |
| [0014](0014-block-file-granularity.md) | Block file granularity | Proposed | [CQR-177](https://linear.app/cqr/issue/CQR-177) |
| [0015](0015-retired-file-grace-period.md) | Grace period before deleting retired files | Proposed | [CQR-174](https://linear.app/cqr/issue/CQR-174) |

## Template

```markdown
# NNNN. Title

- Status: Proposed | Accepted | Superseded by NNNN
- Date: YYYY-MM-DD
- Settled by: Linear ticket

## Context
## Options
## Decision
## Consequences
## Links
```

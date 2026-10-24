STATUS: IN PROGRESS

# R4-05 (cloud): one reasons dictionary with its gate and route, and the kept alert kinds

Branch `cloud/db-R4-05-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R4-05.md`.

## Prerequisites

- IA-16: merged (`Merge IA-16: failure kinds and causes ...` on `origin/feat/db-fix`).
- R3-21: FILES_ONLY link (merge order on `crates/pull/src/alerts.rs` KINDS), not merged; does not block the start.
- IA-12a: dropped (D-6900), not a prerequisite.

## Log

- 2026-10-24 (IST morning): GO seen on the base branch, spec read, branch cut. Reading the code sources.
  NEXT: build `crates/pull/src/reasons.rs` and its gate.

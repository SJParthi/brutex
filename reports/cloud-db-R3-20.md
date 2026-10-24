STATUS: IN PROGRESS

# R3-20 (cloud): month recovery after a power cut

Branch `cloud/db-R3-20-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R3-20.md`.

Prerequisites, all merged at `e851da1ca` (`git log origin/feat/db-fix --first-parent --merges`):
IA-16, R3-16, R3-36, SC-4.

## Progress log

### 2026-10-24 09:16 IST — start

- GO present, HOLD absent, spec and CONTEXT read. Branch cut from `e851da1ca`.
- Read: `CLAUDE.md`, README, CONTEXT, R3-20.md; `store::file` (doors, `validated`,
  `read_header`), `store::repair` (`publish`, `repair_month_with`), `store::generation`
  (`promote`), `crates/store/REPAIR.md`, `pull::ingest` (`already_served`, the successor door),
  `pull::gdfl_import_journal` (the `incomplete` grammar), `pull::gdfl_import_day::admit_rule`.
- NEXT: write `crates/store/src/recover.rs` (plan + door) and its file.rs hook, then tests.

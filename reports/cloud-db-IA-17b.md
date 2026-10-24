STATUS: IN PROGRESS

# IA-17b: D-4875 remainder: the once-per-month gather and the CASH .auc writer

Cloud session, branch `cloud/db-IA-17b-20261024`, cut from `origin/feat/db-fix` at `e851da1c`
(Merge DATA-2), which is also the newest `origin/feat/db-fix` at the start.

## Prerequisites (README "Before you build")

`git log origin/feat/db-fix --first-parent --merges --format=%s | grep -E '^Merge (IA-17|WP-12d|IA-16):'`
printed all three: `Merge IA-16: ...`, `Merge IA-17: ...`, `Merge WP-12d: ...`. Not blocked.

## Log

### 24 Oct 2026, about 11:00 IST: start

- Waited for `GO` and the spec (five 9-minute polls); both arrived with `83b4d4e6` on the base branch.
- Read `CLAUDE.md`, `README.md`, `CONTEXT.md` and `IA-17b.md` in full.
- Spec section 8 item 1 checked at the head: the back-fill still repairs one file at a time
  (`Filing::through`, `Door::BackFill`, `ingest::repair_rows`) and no caller passes a `before_promote`
  step, so `nothing_to_do` is false.
- Baseline: `cargo test -p pull --locked --lib -- gdfl_import::tests::backfill gdfl_import::tests::sealed`
  gave `17 passed; 0 failed`.

NEXT: the `ingest.rs` seam (a door that judges and gathers, and a gathered repair that takes the
step), then the gather in `gdfl_import_filing.rs` and the deferred day close in `gdfl_import_day.rs`.

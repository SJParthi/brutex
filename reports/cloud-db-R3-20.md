STATUS: HELD

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

### 2026-10-24 09:33 IST — save point, HELD

- Done: `crates/store/src/recover.rs` (the door: `plan`, `recover`, the per-thread cut scope
  `within`/`scoped`, the record file `bar-recoveries-v1/…/<yyyy-mm>.recover-v1`, `RecoverError`);
  `crates/store/src/file.rs` (`read_header` applies the recovery cut for the read door on the
  door's own thread only; `open_existing_at` is `pub(crate)`); `crates/store/src/lib.rs`
  (`pub mod recover`). `crates/store/src/recover_tests.rs` is a placeholder.
- `cargo build -p store`: ok. `cargo clippy -p store --lib --locked -- -D warnings`: clean.
  No test run yet (the tests are the next step).
- Commit `88e6d466` (WIP R3-20: store recovery door), pushed.
- HOLD found on `origin/cloud/db-base-20261024` ("HOLD 2026-10-24 09:16 IST", usage window
  near its limit, expected lift soon after 12:30 IST). Waiting as README step 1 says.
- NEXT: when HOLD is gone, write `crates/store/src/recover_tests.rs` (the every-synced-write walk,
  FM-062's test, the extreme cases), then the cli verb.

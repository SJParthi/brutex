STATUS: HELD

# R4-09b — the store's backup verb and its restore drill (AUT-06), cloud build

Branch `cloud/db-R4-09b-20261024`, cut from `origin/feat/db-fix` `e851da1c` (Merge DATA-2).
Prerequisite `R4-09a`: merged (`Merge R4-09a:` is in `git log origin/feat/db-fix --first-parent --merges`).
Decision block: D-7175..D-7184 (D-7137 is LEDGER-1023's; no second one is written).

## Done so far

- `crates/cli/src/backup.rs` (new): `cli backup --to <dir> [--plan] [--mib-per-s N]` and
  `cli backup-drill --from <dir> [--months N]`. It compiles, and `cargo clippy -p cli --locked -- -D warnings`
  is clean on the library. The tests are not written yet: `backup_tests.rs` is still a stub.
- `crates/cli/src/lib.rs`: `mod backup;`, two dispatch arms, `COMMANDS` 60 -> 62, two `USAGE` entries.
- `crates/cli/src/store_write.rs` (outside ownership, minimal): `"backup"` and `"backup-drill"` in
  `WRITERS` (19 -> 21, sorted); `writes_the_store("backup", ..)` is false only with `--plan`; five
  rows added to `a_plan_or_a_dry_run_is_not_a_write_and_its_applying_form_is`.

## Left

- `backup_tests.rs`: the spec's ten tests, failing first, plus at least 8 extreme cases (2 combinations,
  1 out-of-the-box); the per-file step's p50/p99/max at n >= 200.
- `crates/api/src/disk_route.rs` read side (receipt words, target present or missing, last drill,
  staleness by name); the web band (`disk.js`, `CopyAlert.svelte`, `DiskTile.svelte`, `disk.test.js`).
- docs/36 §4, docs/04 rows, docs/05 D-7175.., docs/06 one-time-pass row.
- Gates, then the final report.

## Log

### 2026-10-24 (IST) — start

- GO present, HOLD absent on `origin/cloud/db-base-20261024` (`83b4d4e6 GO: cloud parts may build (24 Oct)`).
- Read README.md, CONTEXT.md and R4-09b.md in full. Read `disk_route.rs` (`last_backup`, `measured`),
  `store_write.rs`, `scrub_blocks.rs` (the month reader lock), `store::generation`, `store::repair`
  (revision ordinals), `pull::gdfl_import` (journal `run` lines, `Origin`) and `web/src/lib/disk.js`.
- Commit `f5281c06` (`WIP R4-09b: report opened (IN PROGRESS)`), pushed.

### 2026-10-24 (IST) — save point 1, then HOLD

- Wrote `crates/cli/src/backup.rs` and registered the verbs (files above).
- `cargo build -p cli --locked`: `Finished`. `cargo clippy -p cli --locked -- -D warnings`: `Finished`, no warning.
- Commit `5d78f00d` (`WIP R4-09b: cli backup and backup-drill verbs`), pushed.
- `HOLD` found on `origin/cloud/db-base-20261024` at this save point: stopped here, waiting for GO.
- NEXT: write `crates/cli/src/backup_tests.rs` (the ten named tests and the extreme cases), run
  `cargo test -p cli --locked backup`, then the api read side.

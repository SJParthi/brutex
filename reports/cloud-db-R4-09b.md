STATUS: IN PROGRESS

# R4-09b — the store's backup verb and its restore drill (AUT-06), cloud build

Branch `cloud/db-R4-09b-20261024`, cut from `origin/feat/db-fix` `e851da1c` (Merge DATA-2).
Prerequisite `R4-09a`: merged (`Merge R4-09a:` is in `git log origin/feat/db-fix --first-parent --merges`).
Decision block: D-7175..D-7184 (D-7137 is LEDGER-1023's; no second one is written).

## Log

### 2026-10-24 (IST) — start

- GO present, HOLD absent on `origin/cloud/db-base-20261024` (`83b4d4e6 GO: cloud parts may build (24 Oct)`).
- Read README.md, CONTEXT.md and R4-09b.md whole; read `disk_route.rs` (`last_backup`, `measured`),
  `store_write.rs`, `scrub_blocks.rs` (the month reader lock), `store::generation`, `store::repair`
  (revision ordinals), `pull::gdfl_import` (journal `run` lines, `Origin`), `web/src/lib/disk.js`.
- NEXT: write `crates/cli/src/backup.rs` and its tests; register the two verbs.

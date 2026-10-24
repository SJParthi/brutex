STATUS: HELD

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

### 24 Oct 2026, save point: the code of the gather, the deferred close and the `.auc` step (HELD)

Commit `cecb5583` (pushed). `cargo check -p pull --locked --tests` clean; `cargo fmt --all` applied.
No test run yet on this code. `HOLD` found on the base branch at this save point
(`5dbfdf3f`, "shared 5-hour usage window near its limit"), so the session stops here and waits.

What is written (not yet proved by a test run):

- `crates/pull/src/ingest.rs` (outside ownership, one seam): `gather_rows` (the back-fill door's
  judgement, `judge_back_fill`, split out of `back_fill_or_append` with its behaviour unchanged, and
  nothing published: `Filed::Gathered { appended }`), `repair_gathered` (the `repairing` line, then
  `repair_after` with a `before_promote` step through `store::repair::repair_month_with`; every
  other caller passes `None`), `rows_mode` hands back the stamps a gathered repair added.
- `crates/pull/src/gdfl_import_filing.rs`: `Gather`, `GatheredFile`, `Offer`, `Deferred`,
  `DeferredClose`, `repair_gathered_months` (one repair per (instrument, month), the CASH step,
  the run's end on a full disk / lost store / unwritable store with the gathered days left begun),
  `gathered_reason` (one file keeps the lone back-fill's words exactly), the audited event
  "gathered month repair finished". The back-fill pass now gathers instead of repairing per file.
- `crates/pull/src/gdfl_import_day.rs`: `Ctx.gather`; a gathered day's close is deferred
  (`defer_day`), the gather is flushed when the run leaves the month (top of `day_pass`), at the
  run's end and before a stop (`stop_after_gather`); `close_deferred` writes each waiting day's
  close in day order with its own counts (`rebased`); report day lists stay in day order.
- `crates/pull/src/gdfl_cm_auction.rs`: `path` resolves the served generation; `append` refuses a
  month served from a revision; `file_or_carry` / `DayPrints` / `Carried` carry an older day's or a
  revision-served month's prints; `write_revision` is the step.
- `crates/pull/src/gdfl_import.rs`: `FileFacts.auction`; `cm_read` calls `file_or_carry`.
- Outside ownership, one line each: `gdfl_ia16_tests.rs` (a `FileFacts` literal),
  `gdfl_scope_tests.rs` (a `Ctx` literal).

NEXT: after `GO` with no `HOLD`: run `cargo test -p pull --locked --lib` and fix what the gather
moves in the existing tests; then the new test file (IA-RP-03..05, FM-092, FM-098, the `.auc`
hazards, at least 8 extreme cases), bench C-RP-02, the ledgers, the gates.

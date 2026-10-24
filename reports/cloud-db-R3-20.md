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

### 2026-10-24 12:46 IST — HOLD lifted; store tests

- HOLD removed on the base branch (55dfb044, "Lift HOLD"); GO present. Resumed.
- `crates/store/src/recover_tests.rs`: 18 tests, including
  `a_month_cut_at_every_synced_write_recovers_to_its_last_good_commit` (every subset of a day's
  writes, three scenarios) and FM-062's `a_power_cut_inside_a_batch_is_named_and_repaired`.
- The walk found a real case the first rule refused: block 1 torn while block 2 of the same batch
  landed. `NotTheTail` now refuses only a failed block below the tail block of the oldest surviving
  commit (inside a durable history) with a verified block after it.
- `cargo test -p store --lib --locked recover`: `test result: ok. 18 passed; 0 failed`.
- NEXT: the whole store suite, then the cli verb.

### 2026-10-24 13:12 IST — the cli verb

- `crates/cli/src/month_recover_command.rs` (`cli month-recover … --plan|--yes`) and its tests
  `crates/cli/src/month_recover_command_tests.rs`: 7 tests, including
  `the_days_dropped_by_recovery_are_read_again` and `a_power_cut_during_a_compaction_plans_and_resumes`.
- Store side: `Record::reread_named` and `store::recover::note_reread` (a cli run cut off between
  the recovery and its journal lines writes them on the next run; 19 store tests pass).
- Touches outside ownership: `crates/cli/src/lib.rs` (module, usage, dispatch arm, `COMMANDS` 61),
  `crates/cli/src/store_write.rs` (`WRITERS` 20, `--plan` is the read-only form, test rows).
- The journal is read with `pull::gdfl_run::JournalReader` (writes nothing: `done_whole_days`
  closes a torn tail when it loads, so it is not used by the plan).
- `cargo test -p cli --lib --locked month_recover`: `test result: ok. 7 passed; 0 failed`.
- NEXT: the store-step row, the bench, the D-7093 and L-IA-40 texts, then the full gates.

### 2026-10-24 13:12 IST — HELD again (times above corrected to the commit times)

- HOLD found again on the base branch at this save point. Commit `a6ab3775` pushed. Waiting.
- NEXT (unchanged): the store-step row, the bench, the D-7093 and L-IA-40 texts, the full gates.

### 2026-10-24 18:26 IST — HOLD lifted; merge, store step, bench

- HOLD lifted (base `f046635b`, "Lift HOLD after the 17:30 reset"). Resumed.
- Merged the newest `origin/feat/db-fix` (`7b397bbe`, Merge CLOUD-MUTCLI; also R4-27, IA-4,
  CLOUD-MUTSTORE): conflicts only in `crates/cli/src/lib.rs` (module list, `COMMANDS` 63), both
  sides kept.
- `config/store-steps.toml`: package R3-20 (merged = false), leftovers RA-280 and FM-062, step
  S-52 (`cli month-recover … --plan` / `--yes`). Gate 30 (`.github/gates_store_steps.rs check`):
  no refusal of S-52; the four refusals left (packages IA-1, IA-4, R4-27, R3-17 merged with
  `merged = false`) are the unmodified base's own (BASE-REDS-1023), shown on a worktree of
  `7b397bbe`.
- `crates/store/benches/ratio/recover.rs`: C-RC-01 (recovery flat at 1x/10x/100x months stored,
  p50/p99/max n = 20) and C-RC-02 (flat per kept bar at N and 4N); hooked into
  `crates/store/benches/ratio.rs` (no new `ratio(` call in that file, so gate 14's store pin of 21
  is unchanged).
- `cargo clippy -p store --all-targets --locked -- -D warnings`: clean.
- NEXT: run the bench, the D-7093 and L-IA-40 texts, then the full gates.

### 2026-10-24 18:27 IST — HELD (HOLD of 18:16 IST)

- HOLD found at this save point ("HOLD 2026-10-24 18:16 IST", lift at the latest after 22:30 IST;
  no GO-R3-20). Commit `ec816448` and the `cargo fmt` order of the cli module list pushed.
- NEXT: when HOLD is gone, run `cargo bench -p store --bench ratio` (C-RC-01, C-RC-02), write the
  D-7093 and L-IA-40 texts, then the full gates (fmt, clippy, workspace tests, deny, CI shell gates,
  llvm-cov branch coverage and cargo-mutants on the touched modules).

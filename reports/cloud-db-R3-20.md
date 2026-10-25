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

### 2026-10-24 19:51 IST — GO-R3-20; bench and ledgers

- `GO-R3-20` on the base branch (57078c40, 19:47 IST): this part builds while HOLD stands for others.
- `cargo bench -p store --bench ratio --locked`: `all ratios within the ceiling`, EXIT 0. New rows:
  C-RC-01 recover a torn month (22,500 kept bars, 100 torn) beside 1, 10, 100 months: p50 49.2 /
  46.2 / 48.2 ms, p99 64.5 / 55.7 / 60.6 ms, max 73.5 / 57.8 / 71.4 ms, n = 20 each; min-of-5
  ratio 0.991x (10x) and 1.039x (100x), ok. C-RC-02 a kept bar: 1,894,661 ps at 22,500 bars,
  1,503,131 ps at 90,000 bars, ratio 0.793x, ok. Method: `Instant` around one `recover` call, a
  fresh copy of the torn month put back outside the timer, invented fixtures on the cloud runner's
  disk (not the operator's drive).
- Docs written for the Mac to land (outside ownership): `docs/05-decisions.md` D-7093,
  `docs/06-limits.md` L-IA-40 and L-R320-01..05, `docs/02-store-format.md` §44 (the recovery
  record), `docs/04-invariants.md` R320-01..10.
- Ledger gates (`.github/gates_ledger.rs`): `invariant-ids` OK, 3209 rows unique;
  `decision-numbers` output identical to the base's except one more heading (its one refusal,
  D-6247 heading two entries, is the base's own, gate 27b, BASE-REDS-1023).
- NEXT: full gates (fmt, clippy workspace, workspace tests, deny, CI shell gates), then coverage
  and mutants on the touched modules.

### 2026-10-24 20:59 IST — gates run; HELD (every GO-<NAME> removed)

- `cargo fmt --all --check`: clean. `cargo clippy --workspace --all-targets --locked -- -D warnings`:
  clean (EXIT 0).
- `cargo deny check`: `advisories FAILED, bans ok, licenses ok, sources ok`, on RUSTSEC-2026-0386
  (tokio `AsyncFd::new` unsound). The unmodified base `7b397bbe` fails the same advisory, same id;
  this branch does not touch `Cargo.toml` or `Cargo.lock`. NEEDS THE MAC (a lock bump is not
  this part's).
- CI shell gates, every `Gate N` run block of `.github/workflows/ci.yml` run on this branch and on
  a worktree of the base `7b397bbe` (with `GITHUB_ENV` and `RUNNER_TEMP` emulated): the same gates
  are red on both, 1c, 1d, 10, 11, 12, 19, 23, 27b, 30 (the CONTEXT 4.10 list, BASE-REDS-1023's),
  13a and 13b (exit 127: `gates-jobs`, built by a non-gate step, not emulated); every other gate run
  is green on both. Within each red gate the branch's refusals equal the base's after two fixes made
  here: gate 11 refused one more `.contains` in `crates/cli/src/store_write.rs` (raised 7 -> 8 in
  `ALLOW_MEMBER`, `.github/gates_ledger.rs`, reason in `docs/06-limits.md`) and the slot sort in
  `recover.rs` (replaced by one comparison of the at most two slots); gate 12 refused the bench
  module's cost claim naming no proof (it now names `crates/store/benches/ratio.rs` and the test).
  Not run here: 1e, 6d, 20, 31 (a no-front-end build, the web tests, coverage, macOS), and 13c, 6c,
  5, 3 on the base (stopped: the disk filled).
- The disk allowance filled once (the base worktree's own `target/`, 12 GB, from gate 13c's build);
  deleted (my scratch build output only), 12 GB free.
- Workspace tests (`cargo test --workspace --locked --no-fail-fast`): still running at this save
  point; the api lib suite had 9 failures (root-owned permission checks, slow-reader timing under
  load, and `a_full_calendar_pool_refuses_the_census_and_the_audit`, whose child sat at 0% CPU for
  25 min and was stopped by me). None touches this part's files; to be compared with the base.
- NOT YET TESTED: the slot-order edit in `recover.rs` (one comparison in place of the sort) came after
  the test runs above; the store suite runs again first.
- NEXT: when HOLD is gone: the store suite, read the finished workspace run, rerun every failure on this branch and on
  the base worktree, then llvm-cov (lines and regions, stable; branch coverage needs a nightly this
  container lacks) and cargo-mutants on recover.rs, file.rs's diff and month_recover_command.rs.

### 2026-10-25 13:12 IST — new session (25 Oct account); HOLD lifted; merge

- Started from the branch tip `580ef522`. HOLD of 13:0x IST lifted at `1dcc33e9` ("Lift HOLD: all
  nine cloud parts build"); GO present.
- `cargo test -p store --locked` (lib): every `recover::tests` test passes (19) after the slot-order
  edit, so that edit is now tested. The lib run's only failures are 3 `settings::tests` that need a
  non-root user (`a_directory_that_stops_being_readable_...` says "this host reads a 000 directory
  (root?)"); this container runs as uid 0. Not this part's files.
- Merged the newest `origin/feat/db-fix` (`666ddf7f`, Merge DATA-1w; also R3-13): conflicts only in
  `docs/04-invariants.md`, `docs/05-decisions.md`, `docs/06-limits.md` (both sides appended a
  section), both kept. Merge commit `716cd5d1`.
- NEXT: fmt, clippy, the workspace tests on the merged tree, deny, then llvm-cov and cargo-mutants.

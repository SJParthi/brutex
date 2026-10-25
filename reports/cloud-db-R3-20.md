STATUS: HELD

# R3-20 (cloud): month recovery after a power cut

Branch `cloud/db-R3-20-20261024`. Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R3-20.md`.
Prerequisites IA-16, R3-16, R3-36 (and SC-4) are merged on `feat/db-fix`. Newest `origin/feat/db-fix`
merged: `cc0a3a0d` (Merge R4-28), merge commit `f922ccd3` (25 Oct, 23:0x IST).

HELD at 02:09 IST, 26 Oct: `HOLD` on the base branch (`4dce2ee1`, "HOLD cloud parts without GO until the
04:00 IST reset"), no `GO-R3-20`. Code and tests are pushed at `b3fc883c`.

## Done

- `crates/store/src/recover.rs` — the door: `plan` (read-only), `recover` (writes a NEW revision of the
  kept bars, promotes it, appends a 256-byte record), `note_reread`, `read_record`, `record_path`,
  `RecoverError` (every refusal by word), `Recovered::how`; the per-thread cut scope (`within`/`scoped`).
- `crates/store/src/file.rs` — `read_header` applies the door's cut on the door's own thread only, for
  the exact bytes it was planned against; `open_existing_at` is `pub(crate)`.
- `crates/store/src/recover_tests.rs` — 30 tests (list below).
- `crates/cli/src/month_recover_command.rs` — `cli month-recover VENDOR EX/SEG/SYM[/CONTRACT] RUNG
  YYYY-MM --plan|--yes`: plan, recovery, read-again lines in the GDFL import journal (reason
  `store_damaged`), end record in `audit/<vendor>.recoveries`, the note; 11 tests in
  `month_recover_command_tests.rs`.
- Bench rows C-RC-01 / C-RC-02 (`crates/store/benches/ratio/recover.rs`), store step S-52, the D-7093
  entry, L-IA-40 and L-R320-01..05, store format §44, invariants R320-01..10 (texts for the Mac to land).
- 25 Oct: three workspace-test refusals of this part fixed (sweep-wiring list, S-52's writers-stopped
  prerequisite wording, a D-4911 citation in a crate with no docs/05 heading); coverage work (below).

## Left

1. Store mutants, second run on `b3fc883c` (after the first run's 22 survivors were each killed by a
   test or their code removed): shard 0/2 running at the HOLD (48 of 116 tested, 0 missed so far);
   shard 1/2 next. Command:
   `CARGO_PROFILE_DEV_OPT_LEVEL=1 cargo mutants --in-diff <diff of recover.rs and file.rs vs feat/db-fix> -p store -j 4 -C --lib --shard K/2 -- -- recover`.
2. cli mutants: `cargo mutants --in-diff <diff of month_recover_command.rs> -p cli -C --lib -- -- month_recover` (41 mutants; not run yet).
3. Coverage once more after the mutant fixes (the 25 Oct 23:xx run is in the Gates table).
4. Merge the newest `origin/feat/db-fix` again before the final gates; fmt, clippy, the touched suites.

## Gates (commands and results)

| gate | command | result |
|---|---|---|
| fmt | `cargo fmt --all --check` | clean (25 Oct, after the last commit) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | clean, EXIT 0 (25 Oct, after the R4-18 merge); `-p store -p cli` clean after the last commit |
| tests | `cargo test --workspace --locked --no-fail-fast` | 33 failures on the merged tree; 3 were this part's and are fixed (each rerun green: `sweep_wiring_tests` 22 pass, `decision_citations` 4 pass, `rehearsal` 16 pass). The other 30 are the container's: tests that `chmod` a file or directory and expect a refusal (this container runs as uid 0, which ignores modes: settings, sink, census link_tests, gdfl_dropbox, gdfl_import ia16, alerts, scrub_blocks, watchdog, zerodha_verify, quarantine index-publish, pullrun journal extremes), slow-reader timing tests (head_deadline_body_tests, feeds_route window, detail admission), `gdfl_cm::fixtures_are_built_not_pasted` (names DATA-1w's `tick_second_check_tests.rs`), and two tests that hang at 0% CPU (`gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`, `cloud_mutants_bars::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`; stopped by me). The hangs, the api timing failures and the pull permission failures were reproduced on a worktree of the unmodified base `bf9b1c8f` (same result); none touches this part's files (`crates/api` is not in the diff). |
| store suite | `cargo test -p store --lib --locked recover` | `ok. 34 passed; 0 failed` (26 Oct, `b3fc883c`) |
| cli suite | `cargo test -p cli --lib --locked month_recover` | `ok. 11 passed; 0 failed` |
| deny | `cargo deny check` | `advisories FAILED, bans ok, licenses ok, sources ok`: RUSTSEC-2026-0386 (tokio 1.53.1), a BASE item (BASE-DENY-1024 bumps tokio; this branch never touches Cargo.lock) |
| gate 30 | `rustc --edition=2024 -D warnings .github/gates_store_steps.rs` then `check config/store-steps.toml --merges … --writers crates/cli/src/store_write.rs --decisions docs/05-decisions.md --owed config/owed-runs.toml` | no refusal names S-52 or R3-20; the 5 refusals left are the base's own `merged = false` flags (IA-1, IA-4, R4-18, R4-27, R3-17) |
| CI shell gates | every `Gate N` block of ci.yml (24 Oct run) | red only where the base is red (1c, 1d, 10, 11, 12, 19, 23, 27b, 30; 13a/13b need a non-gate build step); within each the branch's refusals equal the base's (24 Oct log) |
| coverage | `cargo +nightly llvm-cov --branch --no-report -p store --lib -- recover`, then `-p cli --lib -- month_recover store_write`, then `cargo +nightly llvm-cov report --branch` (25 Oct 23:xx, at `f922ccd3`) | recover.rs: branches 104/104 (100%), every line executed in the line view (`--text`); month_recover_command.rs: branches 30/30 (100%), every line executed; file.rs diff: every line and both arms of each new branch executed. The summary table counts 925/931 and 396/399 lines: zero-count copies of closures in the other test binary on lines the line view shows executed. store_write.rs: every line and branch of this branch's hunks covered; its 6 unreached lines and 3 branches are base test code outside the diff |
| mutants | store: `cargo mutants --in-diff … -p store -j 4 -C --lib -- -- recover` | first run at `d6afa13e`: 243 mutants, 175 caught, 46 unviable, 22 missed; every one of the 22 is now caught by a named test or its code removed as changing nothing (commit `b3fc883c`); second run in progress (Left 1). cli: not run yet (Left 2) |

## Measurements (p50 / p99 / max, n, method)

- C-RC-01, recover a torn month (22,500 kept bars, 100 torn) with 1x / 10x / 100x months stored:
  p50 49.2 / 46.2 / 48.2 ms, p99 64.5 / 55.7 / 60.6 ms, max 73.5 / 57.8 / 71.4 ms, n = 20 each;
  min-of-5 ratio 0.991x (10x) and 1.039x (100x) against the 3.0x ceiling. Method: `Instant` around one
  `recover` call, a fresh copy of the torn month put back outside the timer; invented fixtures on the
  cloud runner's disk (24 Oct, `cargo bench -p store --bench ratio --locked`, EXIT 0).
- C-RC-02, one kept bar: 1,894,661 ps at 22,500 bars, 1,503,131 ps at 90,000 bars, ratio 0.793x.
- The door is NOT O(1) and says so (L-IA-40): O(the month's bytes) per run, one attended run per month,
  never a read path. The months stored beside it do not enter it (C-RC-01 flat). The full one-second
  NIFTY month on the operator's drive is NOT MEASURED (NEEDS THE MAC 7).

## Extreme cases (each a test; * = a combination; † = out of the box)

1. every subset of a day's writes torn (records per block, length, each checksum entry, the slot) —
   `a_month_cut_at_every_synced_write_recovers_to_its_last_good_commit`
2. FM-062: a power cut inside a batch — `a_power_cut_inside_a_batch_is_named_and_repaired`
3. empty: a month born in the open batch — `a_month_that_held_nothing_proven_is_refused_by_name_and_left_as_it_was`
4. one record — `a_one_bar_month_recovers_its_one_bar`
5. * maximum (44,640 bars, the row ceiling) and a door cut off mid-way — `a_full_month_cut_while_its_door_was_cut_off_resumes_at_the_row_ceiling`
6. month and year edges (00:00 on the 1st, 23:59 on the 31st, December) — `the_month_and_year_edges_name_the_first_and_last_days`
7. * two commits torn in one batch at a year end — `two_commits_torn_in_one_batch_at_a_year_end_keep_the_whole_blocks`
8. crash after the promotion, before the record — `a_door_cut_off_after_its_promotion_writes_the_record_it_owes`, and at the verb `a_door_cut_off_before_its_record_is_planned_and_resumed_by_name`
9. a writer holding the month — `a_writer_holding_the_month_refuses_the_door_and_nothing_is_written`
10. torn and garbage record files — `a_torn_record_file_is_stepped_over_and_garbage_is_refused`
11. damage inside the history (not a tail) — `damage_inside_the_months_history_is_not_a_torn_tail`
12. † a cut never applies to other bytes, another thread or another file — `a_cut_never_applies_to_other_bytes_another_thread_or_another_file`
13. † a misdirected write: one header slot written into the other's place — `a_slot_written_into_the_others_place_is_not_a_torn_tail`
14. * all-zero header region; an unproven first block — `a_month_with_no_header_left_or_no_proven_block_is_refused_by_name`
15. links (record file, month lock, bar file) and a missing bar file beside its lock — `links_and_a_missing_bar_file_are_refused_by_name`, `a_linked_record_directory_is_refused_before_anything_is_published`
16. full disk's cousin: record directory / revision root cannot be made — `a_record_directory_that_cannot_be_made_stops_the_door_before_it_publishes`, `a_revision_root_that_cannot_be_made_is_the_publications_refusal`
17. * two doors at once (the other finishes between plan and record) — `a_door_that_finished_first_is_the_answer_and_nothing_is_written_twice`
18. maximum ordinals: one held, and all 1,024 held — `held_ordinals_are_stepped_over_and_the_last_refuses_at_the_limit`
19. a full record file (64 records) — `a_full_record_file_refuses_one_more_record`
20. a later revision and a foreign cut's revision — `a_later_or_a_foreign_cut_revision_is_served_as_it_is`; a stranger's record — `a_record_of_another_recovery_is_not_this_ones_answer`
21. * a power cut during a census compaction — `a_power_cut_during_a_compaction_plans_and_resumes`
22. the importer's re-read of done days — `the_days_dropped_by_recovery_are_read_again`; a torn journal tail — `a_re_read_not_named_is_written_by_the_next_run_and_only_once`; a live GDFL run — `a_live_run_or_an_unreadable_journal_is_named`; a segment no import kind files — `a_segment_no_import_kind_files_names_its_days_for_a_re_read_by_hand`; an end record that cannot be written — `an_end_record_that_cannot_be_written_fails_by_name_and_is_written_next_run`
23. * a claim past the file's end with no commit that checks out — `a_claim_past_the_files_end_with_no_good_commit_keeps_the_whole_blocks_it_holds`; a lost write into the shared block 2 — `a_lost_write_into_the_shared_block_keeps_the_blocks_before_it`; the newest commit in the second slot — `the_newest_commit_is_tried_first_from_either_slot`; every directory made synced and the record's clock — `every_directory_made_is_synced_and_the_record_carries_its_time`

## Files outside ownership (kept minimal)

`crates/cli/src/lib.rs` (module, usage, dispatch, `COMMANDS` 64), `crates/cli/src/store_write.rs`
(`WRITERS` 21, `--plan` read-only, test rows), `crates/cli/src/sweep_wiring_tests.rs` (month-recover is a
store verb), `crates/store/src/lib.rs` (`pub mod recover`), `crates/store/benches/ratio.rs` and
`crates/store/benches/ratio/recover.rs`, `config/store-steps.toml` (package R3-20, step S-52),
`.github/gates_ledger.rs` (gate 11 `ALLOW_MEMBER` store_write.rs 7 -> 8), `docs/02-store-format.md` §44,
`docs/04-invariants.md` R320-01..10, `docs/05-decisions.md` D-7093, `docs/06-limits.md` L-IA-40 and
L-R320-01..05.

## D-numbers

D-7093 (assigned). No `D-TBD-R3-20-n` was needed.

## NEEDS THE MAC

Never against the real store or `:8080`; `W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`.

1. Build: `export CARGO_TARGET_DIR=$W/tgt/pool-2; mkdir $W/tgt/pool-2.lock; (cd $W/wt/P-R3-20 && git ls-files -z | xargs -0 touch && cargo build --release -p api -p cli); mkdir -p $W/tgt/bin-R3-20/release && cp $W/tgt/pool-2/release/{api,cli} $W/tgt/bin-R3-20/release/; rmdir $W/tgt/pool-2.lock`.
2. A private APFS-clone copy (`cp -c`) with a real `bars/` directory: a GDFL one-second NIFTY spot month
   (first), a NIFTY option month, a BANKNIFTY month, an F&O stock month, an illiquid strike month, plus
   `imports/`, `manifest/`, `bar-generations-v1/`, `bar-revisions-v1/`. Expected: `find <copy> -type l`
   prints nothing; `stat -f %i` of each month differs from the real store's.
3. Tear each copied month as a power cut does (the shape of the cli tests' `Store::tear(false)`). No
   cli seam ships; the cloud tests tear in-memory images (`recover_tests.rs` `torn`). On the copy: keep
   `B=<month>.bin` and `C=<month>.crc` aside as `B0`, `C0`; append one more day to the month (the
   release cli's import, on the copy only); then put back the old records with the NEW header region:
   `cp B0 T && dd if=<month>.bin of=T bs=32768 count=1 conv=notrunc && cp T <month>.bin && cp C0 <month>.crc`
   (old length and records, the newest slot claiming the appended day). Expected:
   `BRUTEX_STORE=<copy> cli month-recover gdfl NSE/INDEX/NIFTY 1s <yyyy-mm> --plan` prints
   `refused   … n_valid claims more records than the file holds` and `cut       the commit of
   generation N: keeps K bars` (K = the bars before the append), and the shadow GET of that month
   answers `month_damaged` 500 naming the file.
4. `BRUTEX_STORE=<copy> $W/tgt/bin-R3-20/release/cli month-recover gdfl NSE/INDEX/NIFTY 1s <yyyy-mm> --plan`
   then `--yes`. Expected: exit 0; `shasum -a 256` of the original `.bin` and `.crc` unchanged; the
   receipt names the cut (generation, kept bars, last stamp), the dropped bars and IST days;
   `audit/gdfl.recoveries` gains one `recovered month=…` line; `logs/cli/audit.ndjson` gains the
   `cli.month_recover.lifecycle` `month recovered` event; a second `--yes` prints `Already recovered;
   nothing was written.`
5. Resume the importer on the copy over the dropped days. Expected: each day the journal showed done
   gets one `incomplete … reason=store_damaged` line from step 4 and is read again; the month then equals
   a clean import of the same days bar for bar; no day loops `incomplete reason=store_damaged`.
6. Serve the copy on 8426 (`stop.sh R3-20; serve.sh R3-20 8426 $W/tgt/bin-R3-20 $W/wt/P-R3-20`, store
   root the copy); `lsof -nP -iTCP:8426 -sTCP:LISTEN -t` equals the pid file. Expected:
   `curl -s 'http://127.0.0.1:8426/audit.json?feed=gdfl'` lists the recovery; the month's bars route
   answers 200 with every bar up to the cut; the store-step list shows S-52. GETs only.
7. Time `cli month-recover … --yes` on the full one-second NIFTY month, n = 20, a fresh clone each,
   `/usr/bin/time -l` (wall, peak RSS): p50/p99/max; again with 10x and 100x months stored beside it
   (ratio <= 3.0x). Write the figures into L-IA-40. Also `cargo bench -p store --bench ratio` (C-RC-01,
   C-RC-02: `all ratios within the ceiling`).
8. Gate 30 on the merged tree: no refusal names S-52 (set `merged = true` for R3-20 at the merge).
9. The real store untouched: `imports/gdfl.journal`, `audit/` and the five months keep length and mtime.
10. No write on the real store: a refused real month is recovered only as step S-52, on the operator's
    yes, after a rehearsal on a copy.
11. Coverage and mutants on the Mac's toolchain if the cloud's runs are incomplete at merge-back:
    `cargo llvm-cov --branch -p store --lib -- recover` and `-p cli --lib -- month_recover` (expected 100%
    lines and branches of recover.rs, month_recover_command.rs and file.rs's diff);
    `cargo mutants --in-diff <diff of this branch> -p store -p cli` (expected: no surviving mutant).
12. `cargo deny check` after BASE-DENY-1024 lands (expected green; this branch does not touch Cargo.lock).

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

### 2026-10-25 13:30–21:05 IST — merge, gates, coverage; HELD

- Merged `origin/feat/db-fix` `bf9b1c8f` (Merge R4-18): conflicts in store `lib.rs`, the ratio bench,
  `store_write.rs` and `docs/02-store-format.md`, both sides kept (feat/db-fix first); `WRITERS` 21,
  `COMMANDS` 64.
- Installed cargo-deny, cargo-llvm-cov, cargo-mutants and a nightly toolchain (branch coverage).
- Workspace tests, deny, gate 30: results in the Gates table. Three refusals of this part fixed.
- Coverage: refactors and 16 new tests (store 19 -> 30, cli 7 -> 11); see Done and Left.
- HOLD at 20:40 IST (`17d214c2`), no GO-R3-20: pushed `faf0ed8c`, waiting.
- NEXT: when HOLD is gone (or GO-R3-20 appears): re-measure coverage, run cargo-mutants on the diff,
  merge the newest feat/db-fix, final gates.

### 2026-10-25 23:03 – 26 Oct 02:09 IST — merge, coverage, mutants; HELD

- HOLD lifted at 23:03 (`7136d5d5`). Merged `origin/feat/db-fix` `cc0a3a0d` (Merge R4-28), clean.
- Coverage (nightly `--branch`): recover.rs and month_recover_command.rs 100% of branches, every line
  executed; file.rs's diff fully covered.
- Store mutants, first run (`d6afa13e`): 243 tested, 175 caught, 46 unviable, 22 missed. Each of the
  22 killed by a new test or its code removed (`b3fc883c`); store suite 34 tests.
- HOLD at 02:07 (`4dce2ee1`, until the 04:00 reset), no GO-R3-20: pushed, waiting. The second store
  mutants run (shard 0/2) keeps running in the background meanwhile.
- NEXT: when GO: read shard 0's result, run shard 1/2, then the cli mutants, coverage again, merge
  the newest feat/db-fix, final gates.

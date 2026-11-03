STATUS: DONE

# report-BASE-MSUM-BACKFILL-MS1

Session BASE-MSUM-BACKFILL-MS1 of the 3 Nov 2026 cloud wave. Token `BASE-MSUM-BACKFILL-MS1-20261103-1`
(`OWNER-BASE-MSUM-BACKFILL-MS1.md`). Branch `cloud/msum-ms1-20261103`, START
`8bc44c1eb94167868bf256ca0c345972383464ea` (`4d7f446f7a81` plus the kit). **Code tip
`501651b3b6b56ff30b7635ee9bce7e4d6d8b75f1`** (`git rev-list -1 HEAD -- . ':(exclude)reports'`), pushed. Host: Linux
x86_64, root, 4 cores, 15 GB, `rustc 1.97.1 (8bab26f4f 2026-07-14)` for the gates, the pinned nightly for Route B.

The code-only half of BASE-MSUM-BACKFILL is done: the WIP of `4d7f446f7a81` compiles, its tests and mine pass apart
from proved host reds and one test red by design, findings (1), (2) and (3) are each proved by a test shown red under a
hand change, fmt and clippy `-D warnings` are clean, and Route B covers every line and branch of the D2 set but one (U)
arm. Everything left is the Mac's half (NEEDS THE MAC), which includes the store-steps rows that turn the by-design red
test green.

## What was done

1. **Kit checked.** All 26 kit files match `MANIFEST.md`'s sha256.
2. **START did not compile; fixed** (9189ac1d): `store` lib test lacked `StoreError` in `month_summary_tests.rs`;
   `pull` lib test lacked `summary_owed_by_len` in `upkeep_tests.rs`.
3. **Failures fixed** (9189ac1d): finding (3) changed what the walk lists, so three existing tests were wrong for the
   new rule and two WIP tests expected the wrong skip counts:
   - `pull::upkeep::tests::the_walk_lists_every_month_and_what_it_lacks`: its three 10-bar months now lack only `.tix`
     and `.srt`, and `sum_small == 3` is asserted.
   - `pull::upkeep::tests::a_sidecar_name_held_by_anything_is_not_lacking`: its month now owes all three (1,100 bars,
     the writer's sidecars removed), so its original contrast (a held name is not lacking, a free `.sum` is) stands.
   - `cli::upkeep_command::tests::a_whole_feed_gets_its_sort_index_in_one_run` (DB-N14): `.sum 6 of 7` and the
     `SMALL:` line, its 40-bar month counted apart.
   - `a_clock_stepped_back_gives_a_resume_its_own_id` and
     `a_refused_month_then_a_resume_at_another_worker_count_builds_only_it`: a resume re-walks, so a month that already
     has its `.sum` is in its plan only under `--all`; both resumes now ask `--all`, so their skip counts (1 and 2)
     prove the resume read its chain (without it, the skip count is 0 either way).
4. **Self-check of every hunk (the Mac prompt's step 5b), and what it found and fixed:**
   - (S1, 844d4cba) the one-door `flock` opened the bar file a second time; that open could fail only by a race, an
     error path no test reaches. The lock is now taken on the handle the door already opened.
   - (S2, 844d4cba) `store.sum`'s start line logged `lacking = plan.len()`, which under `--all` counts months that
     have a `.sum`. It now logs `items` and `lacking` apart.
   - (S3, 844d4cba) `summary_owed_by_len` assumes the bar geometry (32,768-byte header, 56-byte rows); a test now pins
     it to every bar layout in `Layout::KNOWN`, so a bar version with another geometry fails before a big month is
     counted small.
   - (S4, b9d24074) **the door the summary and the sort index share (`open_held`) did not refuse a month filed under
     another name (DB-B65)**, though the reader's, the writer's, the index-rebuild and the audited doors all do: a
     backfill would write a `.sum` beside a month every reader refuses. It now calls `refuse_misfiled` before
     `validated`. This also changes the sort door (`cli upkeep run srt` refuses such a month by name instead of
     building its `.srt`): D-7434, for the Mac to confirm at landing.
   - (S5, b9d24074) `msum_builder::build`'s read-back arms (a read-back refused after the door; a reader refusing the
     summary the door kept or wrote) were reachable only by a writer or a repair landing between the door and the
     read-back. `build` now hands the door's answer and a read-back closure to `checked`, and a new module drives them.
5. **Coverage gaps closed by tests** (340cd34c, b9d24074, ceb28bb9, 501651b3): store tests for every arm of the door;
   compound assertion conditions split; two once-only callback branches removed; the store-steps test rewritten to list
   every owed part at once and to return its answer, so all of it runs red or green.

## Files (all under the brief's list; `files_outside_ownership` is empty)

Changed over `f01d7a0e4` at the code tip: `crates/store/src/month_summary_writer.rs`,
`crates/store/src/sort_index_writer.rs`, `crates/store/src/month_summary_tests.rs`,
`crates/store/src/file_fault_tests.rs`, `crates/pull/src/upkeep.rs`, `crates/pull/src/upkeep_tests.rs`,
`crates/cli/src/upkeep_command.rs`, `crates/cli/src/upkeep_command_tests.rs`, `crates/cli/src/msum_builder.rs`, and
one new test file declared in `msum_builder.rs`: `crates/cli/src/msum_builder_read_back_tests.rs`. No `lib.rs`,
`Cargo.*`, `docs/`, `.github/` or `config/` file was touched.

Reports (this folder): this file, `fix-BASE-MSUM-BACKFILL-MS1.json.md`, `OWNER-BASE-MSUM-BACKFILL-MS1.md`,
`cov-BASE-MSUM-BACKFILL-MS1.txt.md` (every Route B figure), `deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-set.txt.md` (the
D2 set), `deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-fu.txt.md` (the one (U) item), `red-BASE-MSUM-BACKFILL-MS1.txt.md`
(every red run: the hand changes and the failure lines).

## The findings of the progress file

| finding | status | proof (each red under its hand change, `red-BASE-MSUM-BACKFILL-MS1.txt.md`) |
|---|---|---|
| (1) the door held the month EXCLUSIVE: a reader refused the build, and a reader waited it out past 200 ms (503 on `/gaps.json`) | fixed: `Hold::Shared` + the bar file's own `flock` (D-7431) | `store::month_summary::tests::the_door_refuses_a_writer_and_never_a_reader` and `cli::upkeep_command::tests::a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name`, both red with `Hold::Exclusive` (batch A: `Locked { path: ".../2031-05.lock" }` for the reader's month); `a_second_door_is_refused_by_the_bar_files_own_lock` red without the bar-file lock |
| (2) the door renamed with no sync | fixed: `Durable::Synced` in the rebuild door only (D-7432) | `store::file::file_fault_tests::a_refused_sync_leaves_the_name_free`, red with the sync removed (batch A: `unwrap_err()` on `Ok(Built(..))`) |
| (3) a small month was listed as lacking a `.sum` forever, so a plan never read 0 | fixed: `summary_owed_by_len` + `Walked::sum_small` (D-7433) | `pull::upkeep::tests::the_walk_counts_a_month_too_short_to_owe_a_summary_apart` and `cli::upkeep_command::tests::a_plan_names_the_lacking_months_and_none_after_the_step`, both red with the walk's small-month block disabled (batch B: the 1,023-bar month listed `[DayIndex, SortIndex, MonthSummary]`; the plan's `.sum 4`) |
| (4) gate 30 is red at `feat/db-fix` (nine `merged = false` packages, R3-17 among them), so the store-steps rows must not name R3-17 as a prerequisite beside figures | **open: the Mac's half** (the rows are the Mac's) | not proved here; D-7437 carries the progress file's plan |

## Tests (new tests by name; every one shown red under a hand change)

- store (8): `month_summary::tests::the_door_refuses_a_writer_and_never_a_reader`,
  `...::a_second_door_is_refused_by_the_bar_files_own_lock`,
  `...::the_door_names_an_empty_and_a_small_month_and_writes_nothing`,
  `...::a_name_held_by_a_directory_is_refused_before_a_byte_is_written`,
  `...::a_staging_name_the_host_refuses_is_the_doors_refusal` (runs where the mode bits bind: uid 65534 under root),
  `...::a_month_filed_under_another_name_is_refused_by_the_door`,
  `file::file_fault_tests::a_full_disk_leaves_no_partial_summary_under_its_name`,
  `file::file_fault_tests::a_refused_sync_leaves_the_name_free`.
- pull (4): `upkeep::tests::the_length_of_a_bar_file_says_when_no_summary_is_owed`,
  `...::a_set_of_lacks_without_a_kind_keeps_the_rest`,
  `...::the_walk_counts_a_month_too_short_to_owe_a_summary_apart`, `...::the_length_rule_holds_for_every_bar_layout`.
- cli (15): `upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed` (red by design, below),
  `...::a_plan_names_the_lacking_months_and_none_after_the_step`,
  `...::a_writer_commit_leaves_no_month_without_its_summary`,
  `...::a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name`,
  `...::an_all_run_rebuilds_a_spoiled_summary_and_keeps_a_true_one`,
  `...::an_all_run_builds_a_version_1_summary_whole_as_version_2`,
  `...::an_empty_and_a_zero_length_month_are_told_apart`,
  `...::a_sealed_month_beside_its_raw_form_is_summarised_and_left_as_it_was`,
  `...::a_month_repaired_mid_run_keeps_its_own_summary`,
  `...::an_import_during_a_run_stops_it_and_a_resume_finishes_the_same_bytes`,
  `...::a_refused_month_then_a_resume_at_another_worker_count_builds_only_it`,
  `...::a_clock_stepped_back_gives_a_resume_its_own_id`,
  `...::a_run_says_on_store_sum_what_it_was_given_and_left`,
  `msum_builder::read_back_tests::a_read_back_refused_after_the_door_is_said_and_never_clean`,
  `msum_builder::read_back_tests::a_summary_the_reader_refuses_after_the_door_is_not_summarised`.
- **Red under a hand change:** four batches at `ceb28bb9`, 29 checks, 29 red, each test sensitive to one change of its
  batch and its failure line naming what that change broke; every hand change reverted and the tree checked clean; none
  committed. The store-steps test is red at the tip itself. Record: `red-BASE-MSUM-BACKFILL-MS1.txt.md`.
- **Extreme cases** (the spec's list): a month under 1,024 bars (store, pull, cli); a month a writer holds (store, cli);
  a sealed `.sbar` month (cli); a version 1 `.sum` (cli); a spoiled and a zero-byte `.sum` (cli); an empty month file
  and a zero-length `.bin` (store, cli); a month repaired mid-run (cli); disk full mid-write (store); combinations: an
  import starting during the run then a stop and resume, a refused month then a resume at another worker count (cli);
  out of the box: the clock stepped back during a run (cli); and beyond the list: a month filed under another name, a
  `.sum` or staging name held by a directory, a staging name the host refuses, a read-back refused or disagreeing after
  the door, a bar layout of another geometry.

**Red by design:** `upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed` returns `Err("24
part(s) of the msum store steps owed, listed above")`: no row of `config/store-steps.toml` runs `cli upkeep run msum
gdfl NSE/INDEX --workers 8` or `cli upkeep run msum zerodha --workers 8` yet. It is the spec's test-first test; it
turns green when the Mac writes the rows from the rehearsal (D-7437). The commands it reads are the Mac's to set.

## Gates (stable `rustc 1.97.1`, at the code tip `501651b3`, one run after the last edit)

One run, 18:40:48-19:17:09 IST, after the last code edit (log lines quoted as printed):

- `cargo fmt --all -- --check`: clean (rc 0).
- `cargo clippy -p store -p pull -p cli --all-targets --locked -- -D warnings`: clean (rc 0, no lint). cli's build script
  printed its own note that an untracked report file leaves the build unstamped; that is a build-script warning, not a
  lint.
- `cargo test -p store --lib --locked`: 433 passed, 5 failed, 2 ignored. The 5 are the root reds below; the same
  binary as uid 65534: **438 passed, 0 failed**, 2 ignored.
- `cargo test -p pull --lib --locked upkeep`: **39 passed**, 0 failed.
- `cargo test -p cli --lib --locked -- upkeep msum` (cargo takes one TESTNAME, so the brief's two filters go after
  `--`): 46 passed, 1 failed, 1 ignored. The failure is the red-by-design store-steps test; the ignored one is
  `upkeep_command::tests::live_run_timings` (a timing, printed, not a gate).
- Beyond the brief, each whole lib once: `cargo test -p pull --lib --locked`: 2,173 passed, 9 failed (the 9 root reds),
  48 ignored; `cargo test -p cli --lib --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store`
  (the Mac prompt's own skip): 2,431 passed, 4 failed (the 3 root reds and the red-by-design test), 12 ignored, 1
  filtered out, 937 s.
- The root reds on these final binaries: pull 9 of 9 and cli 3 of 3 fail as root and pass as uid 65534.
- web: not applicable (`web/` unchanged).

## Coverage: Route B, recipe v4 step 4

Pins checked by the script: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` (`nightly-2026-08-08`), `llvm-cov` and
`llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly`. Reading **narrow** (no `Merge BASE-COV-*` on `f01d7a0e4`).
Set: `deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-set.txt.md` (152 coverage items over the 10 files). Commands (the kit's
`d3-route-b-cloud.sh` through a wrapper that sets `D3_*` for this host and `umask 000`, so the uid-65534 child of the
permission test can write its profile):

    d3-route-b-cloud.sh /home/user/brutex /home/user/deep <P new> <L new> <set> store
    d3-route-b-cloud.sh /home/user/brutex /home/user/deep <P new> <L new> <set> pull -- upkeep
    d3-route-b-cloud.sh /home/user/brutex /home/user/deep <P new> <L new> <set> cli -- upkeep msum

The test filters are the brief's own gates for pull and cli (recipe step 4 allows a filter the part's gates use).
store and pull were measured at `ceb28bb9` (store 17:55-18:21 IST, 25 min, 948 passed, 5 failed: the root reds below;
pull 18:23, 39 passed); `git diff --stat ceb28bb9 501651b3` names only `crates/cli/src/upkeep_command_tests.rs`, so
their sources and set lines are byte-identical at the code tip. cli was measured at `501651b3` (18:37-18:40 IST, 46
passed, 1 failed: the red-by-design test, its profile counted).

| file | lines | branches |
|---|---|---|
| crates/store/src/month_summary_writer.rs | 63/63 | 8/8 |
| crates/store/src/sort_index_writer.rs | 57/57 | 6/6 |
| crates/store/src/month_summary_tests.rs | 200/200 | 0/0 |
| crates/store/src/file_fault_tests.rs | 47/47 | 0/0 |
| crates/pull/src/upkeep.rs | 80/80 | 16/16 |
| crates/pull/src/upkeep_tests.rs | 310/310 | 0/0 |
| crates/cli/src/upkeep_command.rs | 198/198 | 36/36 |
| crates/cli/src/upkeep_command_tests.rs | 552/552 | 2/2 |
| crates/cli/src/msum_builder.rs | 50/54 | 0/0 |
| crates/cli/src/msum_builder_read_back_tests.rs | 35/35 | 0/0 |
| **total** | **1,592/1,596** | **68/68** |

The 4 lines are one (U) item, `crates/cli/src/msum_builder.rs:184-187`, the wildcard arm of a `#[non_exhaustive]`
`store::file::Summarized` whose four variants are each matched above it: proof in
`deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-fu.txt.md`. No GAP and no (F) item. Per item figures:
`cov-BASE-MSUM-BACKFILL-MS1.txt.md`. These are the builder's figures on Linux x86_64; the Mac re-measures by its Route B
at landing. Mutants: deferred (operator 30 Oct); the set file carries the mutants lines for the background pass.

## Host reds (root only; each passes as uid 65534 on the same binary)

- store (5): `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
  `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
  `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`.
- pull (9): `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`,
  `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`,
  `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`,
  `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
  `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`,
  `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
  `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`,
  `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`,
  `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named` (the nine of CONTEXT.md).
- cli (3): `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
  `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`,
  `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`.

Proof for each: the test fails as root and passes as `nobody` with the same binary and command (`setpriv --reuid=65534
--regid=65534 --clear-groups`); the whole store lib binary as `nobody`: 438 passed, 0 failed at the code tip (434
passed, 0 failed at 9189ac1d). Two store
catalog tests failed in my first run only because my `TMPDIR` was not writable by their uid-65534 child; with it
`1777` they pass as root.

## Benches

None run. No request path changed: the added costs are per month of a backfill run (one `lstat` per month with no
`.sum` on the walk; one `fsync` per summary the door writes; one `.prv` open and 64-byte read per month at the door) and
one pass over the plan per run (`msum_given`). Their measured rates are the rehearsal's (months a second, bytes, the
limiting stage): the Mac's half. `/gaps.json` p50/p99/max before and after: the Mac's shadow step 5.

## Decision drafts (MS1's block D-7430..D-7439; for the Mac's docs entries, not written to docs here)

- **D-7430 — BASE-MSUM-BACKFILL, the msum store half (this package's entry).** Every held month of
  1,024 bars or more keeps a `.sum` describing its committed header, so `/gaps.json` reads no bar:
  the code half is at this branch's code tip (the door's hold, its sync, its misfiled refusal, the
  walk's small-month count, the `store.sum` run lines, the read-back seam: D-7431..D-7436). The live
  run is the operator's, after the Mac's rehearsal (1x/10x/100x months at 1, 4 and 8 workers on the
  shadows) and the store-steps rows built from it (D-7437). Nothing here writes the live store.
- **D-7431 — The summary door holds the month SHARED; two doors are kept apart by the bar file's
  own `flock` (finding (1)).** `BarFile::month_summary_existing` opens through `open_held(..,
  Hold::Shared)` (`Flock::try_lock_shared_patiently`, one attempt where no patience is set, as in the
  CLI): a writer holding the month is `StoreError::Locked` naming the month's `.lock` and nothing is
  written; no writer can commit while the rows are read; a reader is neither refused nor made to wait.
  The exclusive hold it had (the sort index's) refused a month while any reader held it and made a
  reader wait out the build past the 200 ms a page waits: a `/gaps.json` asked during a backfill would
  answer 503. Two doors over one month (the staging name has one writer at a time) are kept apart by an
  exclusive non-blocking `flock` taken on the bar file handle the door already opened (no second open,
  so no race-only refusal path); the second door is `Locked` naming the bar file. A kept summary takes
  no such lock. Proof: `store::month_summary::tests::the_door_refuses_a_writer_and_never_a_reader`,
  `...::a_second_door_is_refused_by_the_bar_files_own_lock`,
  `cli::upkeep_command::tests::a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name`.
- **D-7432 — The rebuild door syncs before its rename; the writer's per-commit step does not (finding
  (2)).** `write_image(.., Durable::Synced)` syncs the staged file before it takes the `.sum` name:
  a backfill closes a month in its journal once, so a power cut must not leave a torn file under the
  name of a month it called done. A refused sync, a full disk, a `.sum` or staging name held by a
  directory, and a staging name the host will not create each leave no name behind. The writer's
  per-commit summary stays unsynced (`Durable::Unsynced`): its next open repairs what a cut leaves.
  Proof: `store::file::file_fault_tests::a_refused_sync_leaves_the_name_free`,
  `...::a_full_disk_leaves_no_partial_summary_under_its_name`,
  `store::month_summary::tests::a_name_held_by_a_directory_is_refused_before_a_byte_is_written`,
  `...::a_staging_name_the_host_refuses_is_the_doors_refusal`.
- **D-7433 — A month too short to owe a `.sum` is counted apart, by its bar file's length, for `.sum`
  only (finding (3)).** `pull::upkeep::months` drops the `.sum` lack of a month whose bar file length
  proves fewer than `INDEX_MIN_ROWS` rows (`summary_owed_by_len`: header plus whole rows; the length is
  every row written, committed or not, so it never reads short of the committed rows) and counts it in
  `Walked::sum_small`; a zero-length file or a torn header proves nothing and stays owed, so a run meets
  it and refuses it by name. `.tix` and `.srt` are unchanged (a run still names a small month `small`).
  `cli upkeep plan` prints a `SMALL:` line, a run's items say how many owe none, and `--all` takes the
  small months (each answers `small`). So the plan's months lacking a `.sum` read zero once the step is
  done. Cost: one more `lstat` per month with no `.sum`, on the walk only (plan and run, never a request
  path). The rule's geometry (32,768-byte header, 56-byte rows) is pinned to every bar layout this build
  reads. Proof: `pull::upkeep::tests::the_walk_counts_a_month_too_short_to_owe_a_summary_apart`,
  `...::the_length_of_a_bar_file_says_when_no_summary_is_owed`,
  `...::the_length_rule_holds_for_every_bar_layout`,
  `cli::upkeep_command::tests::a_plan_names_the_lacking_months_and_none_after_the_step`,
  `...::an_empty_and_a_zero_length_month_are_told_apart`.
- **D-7434 — The shared door refuses a month filed under another name (DB-B65), under either hold.**
  `open_held` now calls `refuse_misfiled` before `validated`, as the reader's, the writer's, the
  index-rebuild and the audited doors already do. Before, a backfill wrote a `.sum` (and the sort step a
  `.srt`) beside a month every reader refuses, then reported the read-back's refusal. This changes the
  sort index door too (`cli upkeep run srt` now refuses such a month by name instead of building it).
  Proof: `store::month_summary::tests::a_month_filed_under_another_name_is_refused_by_the_door`.
- **D-7435 — Every `msum` run says on `store.sum` what it was given and what it left.** One INFO line at
  its start (`vendor`, `items` = the plan, `lacking` = months in the plan with no `.sum`, `scope` = the
  run's items sentence) and one at its end (`vendor`, `run`, `how`, `built`, `clean`, `left`; WARN when
  the run is not whole), so the months lacking a summary are searched on `/logs` without a walk. One
  pass over the plan, once a run (gate 17's per-run granularity). Proof:
  `cli::upkeep_command::tests::a_run_says_on_store_sum_what_it_was_given_and_left`.
- **D-7436 — The read side has the last word on what the door did.** `msum_builder::checked(door,
  read_back)`: a read-back refused after the door, or one that refuses the summary the door kept or
  wrote (a writer or a repair landing between the two), is named and never counted clean; a small
  month's read-back is asked only for its rows. Proof:
  `cli::msum_builder::read_back_tests::a_read_back_refused_after_the_door_is_said_and_never_clean`,
  `...::a_summary_the_reader_refuses_after_the_door_is_not_summarised`.
- **D-7437 — (the Mac's half; draft only) The store-steps rows.** One row per feed, `state =
  "pending"`, the command on the worker count the rehearsal measured (the test reads `cli upkeep run
  msum gdfl NSE/INDEX --workers 8` and `cli upkeep run msum zerodha --workers 8`), its check reading `cli
  upkeep plan <feed>` and `cli upkeep status msum <feed>` and `/gaps.json`, a prerequisite of no import
  or pull running, `rehearsed_ms`, `rehearsed_bytes`, `rehearsed_on`, `rehearsed_result`, `disk`.
  Finding (4): gate 30 is red at `feat/db-fix` on nine `merged = false` packages (R3-17 among them;
  BASE-REDS-1023's), so these rows must not name R3-17 as a prerequisite beside figures: the progress
  file's plan is prerequisites `["R3-06", "operator: <the yes>"]`, a new `[[package]] R3-06 merged =
  true`, and `closes` two new leftovers, one per feed. Not proved here: the rows are the Mac's.
- D-7438, D-7439: unused (append-only; an unused id stays unused).

## Invariant drafts (for docs/04, the Mac's)

- A summary door over a held month refuses a writer by name and never a reader, and never makes a reader wait
  (`the_door_refuses_a_writer_and_never_a_reader`, `a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name`).
- Two summary doors over one month: the second is refused by name and writes nothing
  (`a_second_door_is_refused_by_the_bar_files_own_lock`).
- A `.sum` the rebuild door writes is synced before its name appears, and a refused write (full disk, refused sync, a
  name held by a directory, a staging name the host refuses) leaves no name behind
  (`a_refused_sync_leaves_the_name_free`, `a_full_disk_leaves_no_partial_summary_under_its_name`,
  `a_name_held_by_a_directory_is_refused_before_a_byte_is_written`, `a_staging_name_the_host_refuses_is_the_doors_refusal`).
- A month filed under another name gets no sidecar from the shared door
  (`a_month_filed_under_another_name_is_refused_by_the_door`).
- A plan's count of months lacking a `.sum` leaves out months too short to owe one, counts them apart, and reads zero
  after the step (`the_walk_counts_a_month_too_short_to_owe_a_summary_apart`,
  `a_plan_names_the_lacking_months_and_none_after_the_step`); the length rule holds for every bar layout
  (`the_length_rule_holds_for_every_bar_layout`).
- A month the step finishes is clean only when a reader, asked after the door, finds its summary
  (`msum_builder::read_back_tests::*`).
- Every `msum` run says on `store.sum` what it was given and what it left
  (`a_run_says_on_store_sum_what_it_was_given_and_left`).

## Limits drafts (for docs/06, the Mac's)

- `pull::upkeep::months`: O(directory entries under the scope) plus one more `lstat` for each month with no `.sum`
  (D-7433); plan and run only, never a request path.
- `msum_given`: one pass over the plan, once a run.
- The backfill's rate, bytes and the live run's time (a labelled extrapolation from the rehearsal): NOT MEASURED here;
  the Mac's.

## NEEDS THE MAC

1. **Release build** of api and cli: `cargo build --release -p api -p cli`.
2. **The rehearsal** on the Mac's shadows (`shadow/MSUM-g1`, `shadow/MSUM-z1`, every `.sum` left out): `cli upkeep run
   msum gdfl` (NSE/INDEX, NSE/CASH, NSE/FNO scopes) and the zerodha twin at 1x/10x/100x months and 1, 4, 8 workers:
   months a second, bytes, the limiting stage, byte-identical `.sum` across worker counts, one stop and resume, and
   `/gaps.json` asked during the run never a 503, a torn read or a changed answer (the shared hold, D-7431, is what makes
   that possible: re-check it there).
3. **The store-steps rows and their figures**, one per feed, with **finding (4)**: no R3-17 prerequisite beside figures
   while gate 30 is red; the progress file's plan is in D-7437. Then
   `the_msum_store_half_is_a_pending_step_for_each_feed` turns green (adjust its two commands if the rows run others).
4. **Docs** from the rehearsal: docs/04 (the invariant drafts above), docs/05 (D-7430..D-7437 below), docs/06 (the
   limits drafts, with the measured rates and bytes and the live time as a labelled extrapolation).
5. **Full gates** on the merged tree (workspace fmt, clippy, tests, the CI shell gates; gate 30 is BASE-REDS-1023's).
6. **Shadow step 5:** `/gaps.json` p50/p99/max at n = 200 over 1, 12 and 96 months before and after, 1 and 16 clients (0
   refused), one month at 705, 116k and 517k bars, flat within the ratio gate after the step; with it the package's
   original findings O1L-04, O1L-08, o1s-16, o1r-N1, AL-10, AA-07 (still open: they close on the live run).
7. **macOS rows**: the store's macOS-only lines (`crates/store/src/file.rs:7271`, `:10466`,
   `crates/store/src/open_flags.rs:33-122`) by the Mac's Route B; and the whole set re-measured there (these figures
   are Linux x86_64).
8. **Non-root, 14-core runs** of the suites: the 17 root reds above are expected green there.
9. **D-7434's sort-door change**: confirm that `cli upkeep run srt` refusing a misfiled month by name is wanted (it
   matches every other door; before, it built a `.srt` no reader reads).
10. **The D1 landing.**

## Not done

Nothing of the brief's code-only half. Not in it and not done: everything under NEEDS THE MAC; mutants (deferred,
operator 30 Oct). No store, shadow, server or real vendor data was touched; no fixture holds a real vendor row.

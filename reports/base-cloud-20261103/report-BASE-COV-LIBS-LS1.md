STATUS: BLOCKED QUESTION

# BASE-COV-LIBS-LS1 (crate `store`): continuation of 5 Nov 2026

TOKEN `BASE-COV-LIBS-LS1-20261105-2` (a new account; the 3 Nov token `-20261103-1` is retired).
Branch `cloud/cov-libs-ls1-20261103`, continued from its tip `6d18036ec12a`. Code tip
`4b3ee558a38fbc65f6c0dfcf59893cff750568c0` (`git rev-list -1 HEAD -- . ':(exclude)reports'`).

## QUESTION (for the main session)

Every production row of `store` that an input, a file, a folder, a variable or a direct call
reaches now has an asserting test, and every other one is either a (U) with its proof (81 rows)
or one of **61 rows reached only by a host refusal or a race** (`fu-LS1.txt.md`, section
"PRODUCTION ROWS ONLY A HOST REFUSAL OR A RACE REACHES", each with its call and why): a refused
`fsync`, `write`, `fstat`, shared or patient `flock`, rename or read on a file the code has just
opened and checked, or a file or directory that must change between two system calls. No seam
reaches those calls: `file.rs`'s seam covers its own `fault()` and positional reads and writes,
`flock.rs`'s covers `Flock::try_lock` and the unlock only. Beside them, **144 test-code rows**
are a test's own defensive arms: timeouts waiting for a FIFO peer, skips for a host without git,
macOS-only arms, a check module's failure-collection lines, fault-mock guards.

1. For the 61 production rows, which do you want?
   (A) LS1 adds `#[cfg(test)]` seams at those calls in `store`'s production files, in
   `file.rs`'s pattern (a per-thread, one-shot refusal a test arms; no production behaviour
   changes; D-6017's argument that the lift that ships is the lift every test runs), with a
   decision entry (D-8561, below) and a test through each seam;
   (B) LSA records them as limits in `docs/06-limits.md` and they stay owed to the Mac's check
   as host-only rows;
   (C) seams for the host refusals (28 rows), limits for the races (33 rows).
   LS1 recommends (C): a race needs a second actor between two calls, and a seam that fakes it
   tests the seam, not the code.
2. For the 144 test-code rows: may they be (F) under recipe step 4's second clause (a test's or
   a check module's own failing or bounding branch), or must each test be restructured so the
   arm does not exist?

## What this continuation did

- Re-installed the pinned nightly (`rustc -V`: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`;
  `llvm-cov`/`llvm-profdata`: `LLVM version 23.1.0-rust-1.99.0-nightly`) and checked the kit
  against `MANIFEST.md` (`d3-route-b-cloud.sh` f032b525..., `d3-covset.mjs` ecef3bdc...,
  `d3-route-b.sh` f3d70ac2..., `replay-minus-reports.sh` f223185d...: all match).
- Closed the report's NEXT: `quarantine.rs:2708`, the `file.rs` time-index arms, the overlap
  diagnosis past the last record (`file.rs:4451-4452`), `sealed.rs:1504`; `sealed.rs:2352` is a
  (U) (pure arithmetic, proof in the list).
- Then went through **every** remaining production row of the 3 Nov list. A row a direct call
  reaches is never (U), so each such row got a direct-call test against the function's own
  contract (the admission or a door refuses those inputs first on every public path). Rows the
  3 Nov list called (U) but a direct call reaches (`file_sealed.rs:460-461`, `agree`) got tests.
- Two test seams were already in the crate and are now used for the rows they reach: `file.rs`'s
  host-fault seam (read, measure) and `flock.rs`'s lock and unlock refusals.
- Process-wide statics (the one telemetry sink a process may install, the lock registry) are
  met in a **fresh child process** of the test binary that runs the one test alone (the
  `where_permission_binds` mechanism without the uid change), so a test can install its own
  sink and read back what a function said, and see a registry no other test touched.
- No production line changed. The only edits to production files are `#[cfg(test)]` module
  declarations and, in `file.rs`'s `#[cfg(test)]` test module, the seam's `arm_host_fault`,
  `disarm_host_fault` and `host_fault_armed` made crate-visible and `directory_flushes` visible to
  `file`'s own test children.

## Tests added (36 new, 1 corrected), each red under a hand change, then restored

| commit | module (declared in) | tests |
|---|---|---|
| `32002500` | `quarantine_tests.rs` (`quarantine.rs`) | `two_runs_killed_after_their_promotions_are_both_resumed_and_counted` |
| `2781c032`, `12b16ec1`, `4b3ee558` | `file_index_cover_tests.rs` (new, `file.rs`) | `a_daily_month_reaching_the_threshold_with_two_bars_in_one_day_commits_unindexed`, `the_threshold_step_refuses_an_off_grid_or_outside_bar_by_its_batch_offset`, `the_index_step_refuses_an_off_grid_or_outside_bar_by_its_batch_offset`, `an_index_failure_is_refused_naming_the_index_and_its_reason`, `a_handle_that_keeps_no_index_has_no_index_step_to_take`, `a_month_with_no_ready_index_takes_no_entries`, `an_empty_month_has_no_verified_suffix`, `a_bar_past_the_last_record_but_inside_the_claimed_range_is_not_held`, `a_seal_past_every_offset_is_refused_by_the_offset_and_writes_nothing`, `a_block_the_counter_does_not_commit_is_refused_before_it_is_read`, `a_month_at_its_last_generation_answers_an_offer_as_its_append_refuses`, `every_refused_read_of_an_offer_refuses_it_by_the_bars_name`, `a_relative_store_root_confirms_every_directory_up_to_the_root`, `a_level_that_turns_up_before_its_creation_is_taken_as_made` |
| `38ee3db3` | `sealed_fold_cover_tests.rs` (new, `sealed.rs`) | `a_fold_whose_table_slot_has_no_width_stops_at_its_last_row` |
| `9780c55d` | `checksum_audit_cover_tests.rs` (new, `checksum_audit.rs`) | `the_audit_refuses_a_header_or_geometry_it_cannot_hold_before_reading` |
| `9780c55d` | `flock_cover_tests.rs` (new, `flock.rs`) | `a_reader_that_outlives_a_patient_writers_turn_keeps_its_entry`, `one_side_leaving_keeps_an_entry_the_other_still_holds`, `a_hold_sentence_a_formatter_refuses_ends_at_the_refusal`, `a_note_that_cannot_be_cleared_is_said_once_per_process`, `a_turn_ended_before_any_lock_was_recorded_makes_no_registry` |
| `9780c55d` | `generation_door_cover_tests.rs` (`generation.rs`) | `a_linked_pointer_refuses_the_read`, `a_promotion_the_pointers_path_refuses_is_refused_by_that_step` |
| `9d118a6a` | `provenance_cover_tests.rs` (`provenance.rs`) | `a_sealed_form_whose_head_does_not_decode_refuses_the_read`, `a_landing_whose_sidecar_lock_the_host_refuses_is_refused_by_the_lock`, `a_device_the_host_will_not_measure_refuses_the_deferred_flush` |
| `9d118a6a` | `month_summary_decode_cover_tests.rs` (`month_summary.rs`) | `a_bit_past_a_bitmaps_end_is_dropped_never_wrapped` |
| `d081db4e` | `quarantine_tests.rs` (`quarantine.rs`) | `a_release_the_host_refuses_names_the_lock_and_keeps_the_line`, `a_directory_at_the_lock_or_the_index_name_refuses_the_act_before_any_line`, `a_slot_of_the_same_hash_and_another_month_is_passed_over`, `a_resume_whose_dropped_line_the_ledger_refuses_is_still_owed` |
| `12b16ec1` | `file_sealed_cover_tests.rs` (`file_sealed.rs`) | `a_sealed_audit_of_another_symbol_or_timeframe_disagrees_with_the_raw_month` |
| `4b3ee558` | `repair_door_cover_tests.rs` (`repair.rs`) | `a_refused_source_release_after_a_drop_revision_says_it_may_be_visible`, `a_restate_beside_a_drop_that_offers_none_of_its_stamps_holds_nothing_back`, `a_restate_that_restated_nothing_says_nothing` |
| `4b3ee558` | `settings_door_cover_tests.rs` (`settings.rs`) | corrected: `a_bare_number_under_the_floor_is_told_of_percent_only_where_it_could_be_one` |

The suite grew by exactly these 36: 1,064 results passed at the final run against 1,028 at the 3 Nov final.

- Red checks: per batch, one hand change per test in the production file it covers (a refusal
  mapped to another error or ignored, a guard loosened, an early return made an error or a
  panic, a reason word, an emit added), applied together from copies; every new test FAILED;
  each file restored from its copy and `git diff` checked empty.
- Hand changes worth naming: `sealed.rs:1504` `<` to `<=` (the zero-width fold pushes two rows);
  `quarantine.rs:2708` keeping only the last intent's bars (`Resumed { bars: 2 }`, not 3);
  `file.rs` `raw_answer`'s terminal arm turned into a `Follows` answer; `flock.rs` `note_unnoted`'s
  once-guard removed (two `store.flock` lines in the child's own sink); `repair.rs`'s
  `note_restated` emitting for a month with nothing new.
- **A flaw in this shard's own 3 Nov test, fixed** (`4b3ee558`): the floor test's `"95MB"` case
  was refused as an unknown unit ("is not a floor") before the percent clause, so its
  `!contains("percent")` passed without reaching the clause it claimed. It now asserts that
  `1GB` is the floor itself and that `MB` is refused as no unit; `settings.rs:688:30 False` is a
  (U) (every unit scales by at least the floor).
- **A misfiling in the 3 Nov lists, corrected**: `verdict.rs:2278`, `:2312`, `:2478` are
  production lines (the test modules start at `verdict.rs:4178`); the 3 Nov classifier filed them
  under test code. Each is now a (U) with its proof.

## Gates (code tip `4b3ee558`, stable `rustc 1.97.1`, this host)

- `cargo fmt --all --check`: exit 0.
- `cargo clippy -p store --all-targets --locked -- -D warnings`: exit 0 (run after the last code
  change).
- `cargo test -p store --locked --no-fail-fast` (as root, `TMPDIR` a mode-1777 tmpfs): exit 0, 1,064 passed, 0 failed, 8 ignored over 43 result lines (the same counts as the final Route B run on the nightly).
- Mutants: deferred (operator 30 Oct). Benches: none run; no production line changed.

## Coverage: Route B (`store`, whole crate)

The kit's `d3-route-b-cloud.sh` unchanged (`D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_CARGO` set to the
pinned nightly's own tools, `D3_DIR` to the extracted `d3-covset.mjs`, `D3_W=/home/user`), tree
`/home/user/ls1-deep` (a detached worktree at the measured tip), `K=/home/user/brutex/target`,
a new empty profile folder per run, run under `umask 000` so the profile folder takes the
profiles of the `nobody` re-runs and of the fresh-process children. Set file: `reading
whole:store` plus `coverage <file> whole` for every tracked `.rs` file of the crate.

| run | tip | wall (IST) | tests | store lines | store branch sides |
|---|---|---|---|---|---|
| 3 Nov final | `fc9b489f0e3c` | 17:06-17:34 (3 Nov) | 1,028 / 0 / 8 | 51,094/52,629 (1,535 uncovered) | 3,351/3,916 (565 uncovered) |
| progress | `38ee3db3d2fc` | 08:15:51-08:33:56 | 1,038 / 0 / 8 | 51,474/52,972 (1,498 uncovered) | 3,364/3,924 (560 uncovered) |
| **final** | **`4b3ee558a38f`** | **08:47:41-09:05:56 (tests 17 min)** | **1,064 / 0 / 8** | **52,295/53,771 (1,476 uncovered)** | **3,405/3,964 (559 uncovered)** |

- Every run: NOT READ (exit 3) the four `benches/` files (`[[bench]] test = false`, so no object
  holds them; L-BCL-1) and `src/lib.rs` (module declarations only). The final run read 112 files.
- **Production rows** (non-test code): 3 Nov final 239 (169 lines + 70 sides, 44 of them (U));
  progress 197; **final 142 (94 lines + 48 sides): 81 (U) with proofs, 61 host/race, 0 GAP.**
- Test code at the final run: 1,108 rows of `#[ignore]`d tests and the helpers only they call,
  641 (F) by pattern, 144 test-code GAPs. (The totals count the new tests' own failing arms, so
  they move less than the production rows do.)
- `settings.rs:1367-1376` (two racing savers' hard link) stays nondeterministic, as on 3 Nov.
- Lists (this folder): `gaps-LS1.txt.md` (the final run's log lines, the per-file table with row
  counts by class, every uncovered row tagged), `fu-LS1.txt.md` (the 81 (U) items with proofs, the
  61 host/race rows with the call and what is needed, the ignored-test counts by file, the 144
  test-code GAPs, the 641 (F) items by pattern). The 3 Nov proofs are carried for every row still
  uncovered; the two that tests now cover (`file_sealed.rs:460-461`) are gone from them.

## Decision drafts (for LSA; ids from my block only)

- **D-8560** (draft, replaces the 3 Nov draft): BASE-COV-LIBS-LS1, crate `store` (D2 part A, D6
  shard LS1). Route B (the kit's `d3-route-b-cloud.sh`, pinned `rustc 1.99.0-nightly (1a98b1e13
  2026-08-07)` with its own LLVM 23.1.0 tools) on a Linux x86_64 root host. Before (START plus
  the seven root re-runs, tip `c7ccb0f7f7e2`): lines 50,033/51,607, branch sides 3,291/3,868.
  After (tip `4b3ee558a38f`): lines 52,295/53,771, branch sides 3,405/3,964; production rows 225
  lines + 104 sides down to 94 + 48, every one a (U) with its proof (81) or a host refusal or race
  no seam reaches (61). NOT READ: `benches/` (test = false) and `src/lib.rs` (no executable line).
  Lists: `reports/base-cloud-20261103/gaps-LS1.txt.md` and `fu-LS1.txt.md`. Seven permission
  tests that failed only as root re-run their unchanged bodies as `nobody` through
  `crate::support::where_permission_binds` (the D-0995 pattern); tests that need a process-wide
  static fresh (the telemetry sink, the lock registry) run in a fresh child of the test binary.
  Under `cfg(test)` only, `file.rs`'s host-fault seam is crate-visible so another module's test
  arms it. The bar is not met for `store` until the question above is answered; the macOS-only
  lines and a Route B on the Mac's pin are the Mac's.
- **D-8561**: kept for the answer to the question (the seams, if (A) or (C) is chosen).
- docs/04: no new invariant (the tests pin documented behaviour); docs/06: none of mine beyond
  L-BCL-1 and L-BCL-3, unless the answer is (B) or (C).

## NEEDS THE MAC

1. Route B of `store` on the Mac's pin (aarch64-apple-darwin): the macOS-only lines (the brief
   names `src/file.rs:7271`, `:10466`, `src/open_flags.rs:33-122`) are not compiled here and are
   owed to that run, never claimed; the (U) proofs in `fu-LS1.txt.md` are for that run's check.
2. A non-root, 14-core run of the `store` suite (here: root, 4 cores; every root-sensitive test
   re-runs as `nobody` and passes both ways by design).
3. The `#[ignore]`d tests (L-BCL-3 and the measurements): `tests/sealed_codec_samples.rs`,
   `tests/sealed_samples.rs`, `tests/sealed_write_door.rs`, the private-root run of
   `tests/sealed_door.rs`, `tests/tix_latency.rs`, `file::sealed_door_tests::read_rows_at_timing`
   and the full-volume test `file::tests::the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name`.
4. The (F) classification (by pattern, spot-checked) and the test-code GAPs (question 2).
5. The D1 landing; mutants (deferred, operator 30 Oct).

## NOT DONE

- The 61 host/race production rows and the 144 test-code rows: the question above.
- Nothing else of this shard is open.

---

## 3 Nov 2026 (token `BASE-COV-LIBS-LS1-20261103-1`), kept as that session wrote it

Its figures, lists and NEXT are superseded by the sections above; its line 1 was `STATUS: IN PROGRESS`.


TOKEN `BASE-COV-LIBS-LS1-20261103-1`; branch `cloud/cov-libs-ls1-20261103`, cut from
`origin/cloud/base-kit-libs-20261103` (`eeec369be7dc`, START `bd04fb299d23`). Code tip
`fc9b489f0e3ca9207a3f1fe045d913adfa1429fa` (`git rev-list -1 HEAD -- . ':(exclude)reports'`).

**Outcome.** The `store` suite compiles and is green on this host (1,028 passed, 0 failed, 8
ignored); fmt and clippy `-D warnings` are clean. Route B on the pinned nightly measured the
whole crate before and after: uncovered production rows fell from 225 lines and 104 sides to
169 lines and 70 sides, with 36 new tests (each red under a hand change) and no production line
changed. **The bar (100% lines and branches) is not met**: 195 production rows are neither
covered nor proved (U), 995 rows are bodies of `#[ignore]`d tests, 213 test-code rows are GAPs.
Line 1 is IN PROGRESS because some of those production rows are reachable and mine to close
(NEXT, below); most of the rest need a host fault, a race, or the Mac.

### Host

Linux x86_64, root, 4 cores, 15 GiB memory, 29 GiB free disk. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (the repository's `rust-toolchain.toml` pin). Pinned nightly installed
as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
2026-08-07)`; `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
`TMPDIR=/dev/shm/ls1tmp` (tmpfs, mode 1777, so a child test re-run as `nobody` can write).
Kit files checked against `MANIFEST.md` before use: `d3-route-b-cloud.sh` f032b525...,
`d3-covset.mjs` ecef3bdc..., `d3-route-b.sh` f3d70ac2..., `replay-minus-reports.sh`
f223185d... (all match). Scratch tools of mine (node only, never committed): a row classifier
(F? by pattern / GAP) and the two list writers.

### Step 1: `cargo test -p store --lib --locked --no-fail-fast`

- On START (as root, `TMPDIR` mode 0755): compiles with no warning; 466 passed, 9 failed,
  2 ignored. Two of the nine already re-run themselves as `nobody` through
  `crate::support::where_permission_binds` and failed only because the child could not write
  `TMPDIR` (my host setup); with `TMPDIR` mode 1777 they pass.
- The other seven fail only as root. Proof: each passes alone as uid 65534 with
  `setpriv --reuid=65534 --regid=65534 --clear-groups <store lib test binary> --exact <name>`
  (`test result: ok. 1 passed`):
  - attempt 2-4 tests: `catalog::cover_tests::a_bars_root_that_cannot_be_listed_is_refused_not_empty`,
    `quarantine::cover_tests::an_index_the_host_will_not_open_is_refused_with_the_hosts_kind`;
  - base tests (present at `f01d7a0e4`):
    `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
    `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
    `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
    `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
    `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`.
- Fix (test code only, the crate's own D-0995 pattern, commit `c7ccb0f7`): each of the seven
  calls `crate::support::where_permission_binds("<harness name>", <name>_body)`, which re-runs
  the unchanged body as `nobody` when the process is root and in the same uid otherwise (the Mac
  and CI run it as before). After: 475 passed, 0 failed, 2 ignored, as root. No host red is left.

### Step 2: Route B (`store`, whole crate)

Script: the kit's `d3-route-b-cloud.sh` unchanged, with `D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_CARGO`
set to the pinned nightly's own tools and `D3_DIR` to the extracted `d3-covset.mjs`; tree
`/home/user/ls1-deep` (a detached worktree checked out at each measured tip),
`K=/home/user/brutex/target` (build in `target/cov`), a new empty profile folder per run, set file
`reading whole:store` plus `coverage <file> whole` for every tracked `.rs` file of the crate.

| run | tip | wall (IST) | tests | store lines | store branch sides |
|---|---|---|---|---|---|
| 1 (not counted) | `c7ccb0f7f7e2` | 15:16:44-15:46:37 | 992 / 0 / 8 | — | — |
| baseline | `c7ccb0f7f7e2` | 15:47:42-16:15:48 (tests 26 min) | 992 / 0 / 8 | 50,033/51,607 (1,574 uncovered) | 3,291/3,868 (577 uncovered) |
| progress | `4cc3016a1aa0` | 16:27:35-16:56:21 (tests 26 min) | 1,020 / 0 / 8 | 50,882/52,410 (1,528 uncovered) | 3,339/3,912 (573 uncovered) |
| **final** | **`fc9b489f0e3c`** | 17:06:02-17:34:14 (tests 25 min) | 1,028 / 0 / 8 | **51,094/52,629 (1,535 uncovered)** | **3,351/3,916 (565 uncovered)** |

- Run 1 is not counted: its profile folder sat under a mode-700 parent, so every child re-run
  as `nobody` wrote no profile. The tree and profiles moved under `/home/user` (mode 755).
- Every run: NOT READ (exit 3) the four `benches/` files (`[[bench]] test = false`, so `cargo
  test` builds none and no object holds them; L-BCL-1) and `src/lib.rs` (module declarations
  only, no executable line). Totals count the files read (99 at baseline, 108 final).
- The totals include the new test files' own failing assert arms (F), so they move less than the
  production rows do. Production rows (non-test code, by the classifier): baseline 225 lines +
  104 sides; final 169 lines + 70 sides, of which 44 are (U) with proofs.
- Per file, production (final): `month_summary_writer.rs` 165/165 lines, 34/34 sides (from
  146/165, 25/34); `month_summary.rs` 682/683, 189/194 (from 677/683, 182/194); `provenance.rs`
  756/765, 150/152 (from 742/765, 144/152); `repair.rs` 727/741, 128/132 (from 715/741,
  122/132); `generation.rs` 315/325, 59/60 (from 313/325, 58/60); `file_sealed.rs` 255/271, 46/58
  (from 255/271, 42/58; its remaining rows are (U)); `file.rs` 4,564/4,662, 362/382 (from
  4,558/4,662); `quarantine.rs` 1,460/1,483, 185/188; `settings.rs` 724/743, 144/150.
- `settings.rs` 1367-1374 and side 1369:21 True were covered in the baseline run and not in the
  final one: they are the hard-link collision of two writers, reached only when the base suite's
  racing-writers test interleaves that way (`a_stale_or_racing_change_is_refused_and_one_racer_lands`,
  `under_a_real_successor_race_no_landed_save_is_answered_refused`). Nondeterministic base
  coverage, not a change of mine; a deterministic test needs a seam between the listing and the
  link.
- Lists (this folder): `gaps-LS1.txt.md` (the final run's log lines, the per-file table and every
  uncovered line and side with its first-pass class), `fu-LS1.txt.md` (44 (U) items each with its
  proof from the code, 653 (F) rows by pattern, and every remaining row by category: 195
  production GAPs, 995 ignored-test rows, 213 test-code GAPs).

### Step 3: tests added (36 new, 7 converted), each red under a hand change, then restored

| commit | module (declared in) | tests |
|---|---|---|
| `5ecb6180` | `file_cover_tests.rs` (`file.rs`) | `every_sealed_offer_has_its_own_word_and_the_sentence_names_the_conflict`, `a_sink_that_refuses_the_sealed_sentence_ends_it_at_the_refused_write` |
| `5ecb6180` | `file_sealed_cover_tests.rs` (`file_sealed.rs`) | `each_header_fact_the_two_forms_disagree_on_refuses_the_audit_by_name` |
| `5ecb6180`, `fc9b489f` | `month_summary_writer_cover_tests.rs` (`month_summary_writer.rs`) | `a_kept_summary_of_another_symbol_is_built_whole_never_extended`, `a_stale_summary_that_is_no_prefix_of_these_bars_is_built_whole`, `the_existing_month_door_answers_an_empty_and_a_small_month_without_writing`, `a_name_held_by_a_directory_is_left_and_the_summary_write_refused`, `a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind` |
| `70c2bc05` | `provenance_cover_tests.rs` (`provenance.rs`) | `a_sidecar_of_another_month_of_the_series_is_named_by_its_month`, `a_sidecar_name_that_holds_no_regular_file_says_nothing`, `a_served_generation_that_cannot_be_resolved_refuses_the_read`, `a_sealed_form_that_cannot_be_opened_refuses_the_read`, `a_sealed_month_alone_carries_its_records_to_the_reader`, `a_sidecar_the_host_will_not_open_is_refused_by_its_open`, `a_landing_inside_a_durable_batch_leaves_its_flushes_to_the_batch`, `a_batch_sealed_under_a_writer_leaves_the_flush_to_the_writer` |
| `9caf8155` | `repair_door_cover_tests.rs` (`repair.rs`) | `a_month_served_from_the_last_revision_has_none_left_for_any_door`, `a_correction_whose_next_revision_another_request_holds_steps_over_it`, `the_drop_door_refuses_a_month_with_an_indexed_sibling`, `a_correction_that_only_adds_bars_publishes_them_in_order`, `an_empty_month_needs_no_basis_witness`, `a_revision_published_from_another_source_or_other_bars_is_never_reused`, `a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host` |
| `4cc3016a` | `settings_door_cover_tests.rs` (`settings.rs`) | `a_bare_number_under_the_floor_is_told_of_percent_only_where_it_could_be_one`, `a_settings_directory_a_save_cannot_list_refuses_the_save_as_io`, `a_generation_the_host_will_not_open_is_a_fault_in_the_hosts_words`, `a_store_root_a_landed_save_cannot_open_still_lands_the_save` |
| `4cc3016a` | `quarantine_tests.rs` (`quarantine.rs`) | `a_ledger_holding_two_series_answers_each_only_its_own_dropped_stamps` |
| `a3953208` | `month_summary_decode_cover_tests.rs` (`month_summary.rs`) | `a_day_entry_whose_seconds_and_bar_count_disagree_is_refused_each_way`, `a_section_with_a_reserved_version_byte_set_is_refused_as_its_version`, `a_payload_past_the_largest_month_or_cut_inside_its_days_is_refused`, `a_summary_that_counts_decreasing_stamps_counts_no_padded_minute`, `a_kept_summary_the_host_will_not_open_is_unreadable_in_its_words` |
| `1f93151d` | `generation_door_cover_tests.rs` (`generation.rs`) | `a_pointer_that_holds_its_last_record_refuses_the_next_promotion`, `a_non_directory_inside_the_pointer_tree_refuses_the_read` |
| `c160302c` | `staging_cover_tests.rs` (`staging.rs`) | `a_leftover_the_host_will_not_let_the_door_remove_is_refused_by_the_removal` |

- Every test that needs the mode bits to bind runs its body through `where_permission_binds`
  (8 of the new ones, and the 7 converted), so it is green as root here and as an ordinary user
  on the Mac.
- Red checks: per batch, one hand change per test (a word, a dropped `?`, a removed guard or
  clause, a skip arm made a return, a count, an action name), applied together from copies;
  every new test FAILED; each file restored from its copy and the diff checked. The stale-summary
  test was checked again alone after its fix (`fc9b489f`): red on exactly its fold-count case.
- A flaw I found and fixed in my own test (`fc9b489f`): the stale-summary cases left `n_valid` 0
  (the writer stamps it from the header), so all six stopped at "no rows" and the test passed
  without reaching the clauses it names. Each case now carries its row count; the final run shows
  every clause's false side covered.
- One test written and dropped before commit: a "counter-only" cache-miss case for
  `read_verified`; a hand change showed it never reached that side, because every append clears
  the cache in `seal_committed` before the counter moves. That side is (U) in `fu-LS1.txt.md`.

### Gates (at the code tip `fc9b489f`, stable 1.97.1, this host)

- `cargo fmt --all --check`: exit 0.
- `cargo clippy -p store --all-targets --locked -- -D warnings`: exit 0 (two findings in my tests
  at `c160302c`, a name too like `root` and two arms that name their one variant, fixed in
  `5d236b04`).
- `cargo test -p store --locked --no-fail-fast`: exit 0, 1,028 passed, 0 failed, 8 ignored over
  43 result lines, 4 min 4 s wall.
- Coverage: the final Route B row above. Mutants: deferred (operator 30 Oct); my diff adds no
  production function (only `#[cfg(test)]` module declarations), so its D2 mutants set is test
  code only.
- Benches: none run; no hot path changed (no production line changed).

### Files changed (all under `crates/store/src/`, 1,893 lines added, none removed)

New: `file_cover_tests.rs`, `file_sealed_cover_tests.rs`, `generation_door_cover_tests.rs`,
`month_summary_decode_cover_tests.rs`, `month_summary_writer_cover_tests.rs`,
`provenance_cover_tests.rs`, `repair_door_cover_tests.rs`, `settings_door_cover_tests.rs`,
`staging_cover_tests.rs`. Module declarations appended to: `file.rs`, `file_sealed.rs`,
`generation.rs`, `month_summary.rs`, `month_summary_writer.rs`, `provenance.rs`, `repair.rs`,
`settings.rs`, `staging.rs`. Tests converted or added in: `catalog_cover_tests.rs`,
`quarantine_cover_tests.rs`, `quarantine_tests.rs`, `settings_tests.rs`. Nothing outside my shard.

### Decision drafts (for LSA; ids from my block only)

- **D-8560** (draft): BASE-COV-LIBS-LS1, crate `store` (D2 part A, D6 shard LS1). Route B (the
  kit's `d3-route-b-cloud.sh`, pinned `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)` with its own
  LLVM 23.1.0 tools) on a Linux x86_64 root host. Before (START plus the seven root re-runs, tip
  `c7ccb0f7f7e2`): lines 50,033/51,607, branch sides 3,291/3,868. After (tip `fc9b489f0e3c`):
  lines 51,094/52,629, branch sides 3,351/3,916; production rows 225+104 down to 169+70 (44 of
  them (U) with proofs). NOT READ: `benches/` (test = false) and `src/lib.rs` (no executable
  line). Lists: `reports/base-cloud-20261103/gaps-LS1.txt.md` and `fu-LS1.txt.md`. Seven
  permission tests that failed only as root re-run their unchanged bodies as `nobody` through
  `crate::support::where_permission_binds` (the D-0995 pattern). The bar is not met for `store`:
  the remaining rows are listed by category in `fu-LS1.txt.md`; the macOS-only lines and a
  Route B on the Mac's pin are the Mac's.
- **D-8561**: unused. No production line changed and no defect was found.
- docs/04: no new invariant (the tests pin documented behaviour); docs/06: none of mine beyond
  L-BCL-1 (benches) and L-BCL-3 (ignored real-data tests), which already exist.

### NEEDS THE MAC

1. Route B of `store` on the Mac's pin (aarch64-apple-darwin). The macOS-only lines (the brief
   names `src/file.rs:7271`, `:10466`, `src/open_flags.rs:33-122`) are not compiled on Linux, so
   they are absent from my figures; they are owed to that run, never claimed here. The (U) proofs
   in `fu-LS1.txt.md` are for that run's check.
2. A non-root, 14-core run of the `store` suite (mine: root, 4 cores; every root-sensitive test
   re-runs as `nobody` and passes both ways by design).
3. The `#[ignore]`d tests (L-BCL-3 and the measurements): `tests/sealed_codec_samples.rs`,
   `tests/sealed_samples.rs`, `tests/sealed_write_door.rs`, the private-root run of
   `tests/sealed_door.rs` (real months); `tests/tix_latency.rs` (2) and
   `file::sealed_door_tests::read_rows_at_timing` (release-mode timings); the full-volume test
   `file::tests::the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name`
   (`BRUTEX_FULL_VOLUME`, L-PREC1-01; on Linux a small tmpfs serves, which root can mount).
4. The D1 landing; mutants (deferred, operator 30 Oct).

### NOT DONE (NEXT for a continuation of this shard)

- Reachable production rows still without a test (from `fu-LS1.txt.md` PRODUCTION GAPS):
  `quarantine.rs:2708` (two killed runs' intents owed for one month), the `file.rs` time-index
  paths reached only through a torn `.tix` mid-append (3881-3891, 3994-4013, 4926, 4934-4937), the
  `file.rs` overlap diagnosis with a header claiming a later last stamp than its last record
  (4451-4452), `sealed.rs:1504` (a time table filled by the month's last slot), `sealed.rs:2352`.
- Host-fault rows (a failed fsync, write, flock or fstat, e.g. the `)?;` lines after them) and
  race rows (`file_sealed.rs:425`, `quarantine.rs:1187-1191`, `quarantine.rs:1439`,
  `settings.rs:885-888`, `settings.rs:1320-1324`, `file.rs:5334`): no portable test drives them
  without a seam; each needs a decision (a test seam in production code, or a recorded limit).
- Test-code GAPs (213): `git`-missing skip paths, fault-mock guard sides, check modules'
  failure-collection lines (`tests/cited_commits.rs`, `tests/docs.rs`; (F) by recipe step 4's
  second clause if the Mac agrees), helpers no test calls.
- The (F) rows are classified by pattern (failing side of an assert macro, a `panic!` or
  `unreachable!` arm, inside test code) and were spot-checked, not read one by one.

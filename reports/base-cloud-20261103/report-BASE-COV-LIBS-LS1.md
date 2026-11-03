STATUS: IN PROGRESS

# BASE-COV-LIBS-LS1 (crate `store`), 3 Nov 2026 cloud wave

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

## Host

Linux x86_64, root, 4 cores, 15 GiB memory, 29 GiB free disk. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (the repository's `rust-toolchain.toml` pin). Pinned nightly installed
as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
2026-08-07)`; `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
`TMPDIR=/dev/shm/ls1tmp` (tmpfs, mode 1777, so a child test re-run as `nobody` can write).
Kit files checked against `MANIFEST.md` before use: `d3-route-b-cloud.sh` f032b525...,
`d3-covset.mjs` ecef3bdc..., `d3-route-b.sh` f3d70ac2..., `replay-minus-reports.sh`
f223185d... (all match). Scratch tools of mine (node only, never committed): a row classifier
(F? by pattern / GAP) and the two list writers.

## Step 1: `cargo test -p store --lib --locked --no-fail-fast`

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

## Step 2: Route B (`store`, whole crate)

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

## Step 3: tests added (36 new, 7 converted), each red under a hand change, then restored

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

## Gates (at the code tip `fc9b489f`, stable 1.97.1, this host)

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

## Files changed (all under `crates/store/src/`, 1,893 lines added, none removed)

New: `file_cover_tests.rs`, `file_sealed_cover_tests.rs`, `generation_door_cover_tests.rs`,
`month_summary_decode_cover_tests.rs`, `month_summary_writer_cover_tests.rs`,
`provenance_cover_tests.rs`, `repair_door_cover_tests.rs`, `settings_door_cover_tests.rs`,
`staging_cover_tests.rs`. Module declarations appended to: `file.rs`, `file_sealed.rs`,
`generation.rs`, `month_summary.rs`, `month_summary_writer.rs`, `provenance.rs`, `repair.rs`,
`settings.rs`, `staging.rs`. Tests converted or added in: `catalog_cover_tests.rs`,
`quarantine_cover_tests.rs`, `quarantine_tests.rs`, `settings_tests.rs`. Nothing outside my shard.

## Decision drafts (for LSA; ids from my block only)

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

## NEEDS THE MAC

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

## NOT DONE (NEXT for a continuation of this shard)

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

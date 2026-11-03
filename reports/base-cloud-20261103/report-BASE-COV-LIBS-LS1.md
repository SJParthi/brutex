STATUS: IN PROGRESS

# BASE-COV-LIBS-LS1 (crate `store`), 3 Nov 2026 cloud wave

TOKEN `BASE-COV-LIBS-LS1-20261103-1`; branch `cloud/cov-libs-ls1-20261103`, cut from
`origin/cloud/base-kit-libs-20261103` (`eeec369be7dc`, START `bd04fb299d23`).

## Host

Linux x86_64, root, 4 cores, 15 GiB memory, 29 GiB free disk. Stable `rustc 1.97.1
(8bab26f4f 2026-07-14)` (the repository's `rust-toolchain.toml` pin). Pinned nightly installed
as `nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13
2026-08-07)`; `llvm-cov` and `llvm-profdata` print `LLVM version 23.1.0-rust-1.99.0-nightly`.
`TMPDIR=/dev/shm/ls1tmp` (tmpfs, mode 1777 so a child test re-run as `nobody` can write).
Kit scripts checked against `MANIFEST.md`: `d3-route-b-cloud.sh` f032b525..., `d3-covset.mjs`
ecef3bdc..., `d3-route-b.sh` f3d70ac2..., `replay-minus-reports.sh` f223185d... (all match).

## Step 1: `cargo test -p store --lib --locked --no-fail-fast`

- On START (as root, `TMPDIR` mode 0755): compiles with no warning; 466 passed, 9 failed,
  2 ignored. Two of the nine re-run themselves as `nobody` through
  `crate::support::where_permission_binds` and failed only because the child could not write
  `TMPDIR` (my host setup); with `TMPDIR` mode 1777 they pass.
- The other seven fail only as root. Proof: each passes run alone as uid 65534 with
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
- Fix (test code only, the crate's own D-0995 pattern): each of the seven now calls
  `crate::support::where_permission_binds("<harness name>", <name>_body)`, which re-runs the
  unchanged body as `nobody` when the process is root and in-process-uid otherwise (the Mac and
  CI run it as before). After: 475 passed, 0 failed, 2 ignored (as root).

## Step 2: Route B baseline (`store`, whole crate)

- Script: the kit's `d3-route-b-cloud.sh` unchanged, with `D3_NIGHTLY`, `D3_LLVM_BIN`, `D3_CARGO`
  set to the pinned nightly's own tools and `D3_DIR` to the extracted `d3-covset.mjs`; tree
  `/home/user/ls1-deep` (a detached worktree at the tip), `K=/home/user/brutex/target` (so the
  build is `target/cov`), a new empty profile folder per run.
- First run (tip `c7ccb0f7f7e2`, 15:16:44-15:46:37 IST): `cargo test -p store` 992 passed, 0
  failed, 8 ignored. NOT COUNTED: its profile folder sat under a mode-700 parent, so every child
  re-run as `nobody` (D-0995) wrote no profile and those tests' bodies read as uncovered. Moved
  the tree and profiles under `/home/user` (mode 755) and ran again.
- Baseline run (same tip, 15:47:42-16:15:48 IST, tests 26 min): 992 passed, 0 failed, 8
  ignored; 69 profiles. **store lines 50,033/51,607 (1,574 uncovered), branches 3,291/3,868 (577
  uncovered sides), 99 files read.** NOT READ (exit 3): the four `benches/` files (`[[bench]]
  test = false`, so `cargo test` builds none: no object holds them) and `src/lib.rs` (module
  declarations only, no executable line). About 950 of the uncovered lines are the bodies of the
  `#[ignore]`d real-data and timing tests (`tests/sealed_codec_samples.rs`,
  `tests/sealed_samples.rs`, `tests/sealed_write_door.rs`, `tests/tix_latency.rs`, the ignored
  test of `tests/sealed_door.rs`): owed to the operator (L-BCL-3), never run by `cargo test`.
- `gaps-LS1.txt.md` (this folder) holds that run's log lines, the per-file table and every
  uncovered line and side with a first-pass class (F? or GAP).

## Step 3: closing gaps (tests added so far, each red under a hand change, then restored)

| commit | module (declared in) | tests |
|---|---|---|
| `5ecb6180` | `file_cover_tests.rs` (`file.rs`) | `every_sealed_offer_has_its_own_word_and_the_sentence_names_the_conflict`, `a_sink_that_refuses_the_sealed_sentence_ends_it_at_the_refused_write` |
| `5ecb6180` | `file_sealed_cover_tests.rs` (`file_sealed.rs`) | `each_header_fact_the_two_forms_disagree_on_refuses_the_audit_by_name` |
| `5ecb6180` | `month_summary_writer_cover_tests.rs` (`month_summary_writer.rs`) | `a_kept_summary_of_another_symbol_is_built_whole_never_extended`, `a_stale_summary_that_is_no_prefix_of_these_bars_is_built_whole`, `the_existing_month_door_answers_an_empty_and_a_small_month_without_writing`, `a_name_held_by_a_directory_is_left_and_the_summary_write_refused`, `a_month_directory_the_host_will_not_write_refuses_the_staging_by_its_kind` |
| `70c2bc05` | `provenance_cover_tests.rs` (`provenance.rs`) | `a_sidecar_of_another_month_of_the_series_is_named_by_its_month`, `a_sidecar_name_that_holds_no_regular_file_says_nothing`, `a_served_generation_that_cannot_be_resolved_refuses_the_read`, `a_sealed_form_that_cannot_be_opened_refuses_the_read`, `a_sealed_month_alone_carries_its_records_to_the_reader`, `a_sidecar_the_host_will_not_open_is_refused_by_its_open`, `a_landing_inside_a_durable_batch_leaves_its_flushes_to_the_batch`, `a_batch_sealed_under_a_writer_leaves_the_flush_to_the_writer` |
| `9caf8155` | `repair_door_cover_tests.rs` (`repair.rs`) | `a_month_served_from_the_last_revision_has_none_left_for_any_door`, `a_correction_whose_next_revision_another_request_holds_steps_over_it`, `the_drop_door_refuses_a_month_with_an_indexed_sibling`, `a_correction_that_only_adds_bars_publishes_them_in_order`, `an_empty_month_needs_no_basis_witness`, `a_revision_published_from_another_source_or_other_bars_is_never_reused`, `a_revision_the_host_will_not_let_the_door_create_is_refused_by_the_host` |

Red checks: each batch's hand changes (one per test: a word, a dropped `?`, a removed guard, a
skip arm made a return, a count) were applied together; every new test FAILED; the files were
restored from copies and the diff checked.

## NEXT

Settings, quarantine, flock, generation, safe_open, verdict, month_summary, file.rs remainder;
test-file GAPs; `fu-LS1.txt.md` with the (U) proofs; final Route B at the last code tip.

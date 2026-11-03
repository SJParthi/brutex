STATUS: IN PROGRESS

# report-BASE-MSUM-BACKFILL-MS1 (interim)

Session BASE-MSUM-BACKFILL-MS1, token BASE-MSUM-BACKFILL-MS1-20261103-1, branch
`cloud/msum-ms1-20261103`, START `8bc44c1eb94167868bf256ca0c345972383464ea`
(4d7f446f7a81 plus the kit). Host: Linux x86_64, root, 4 cores, 15 GB.

Interim: written so the work survives a cut. The final report replaces it.

## Done so far

- The kit checked against `MANIFEST.md`: all 26 files match their sha256.
- START did not compile (`store` lib test: `StoreError` not imported in
  `month_summary_tests.rs`; `pull` lib test: `summary_owed_by_len` not
  imported in `upkeep_tests.rs`). Fixed in 9189ac1d.
- Tests now green on this host apart from proved host reds and one red by
  design (see below): store lib, pull upkeep, cli upkeep + msum.
- Self-check changes (844d4cba): the door's one-door `flock` is taken on the
  bar handle the door already opened (a second `open_read` could fail only by
  a race); `store.sum`'s start line logs `items` and `lacking` apart (under
  `--all` the plan holds months that have a `.sum`); a pull test pins the
  length rule's geometry to every bar layout.

## Red by design

`upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed`
fails with `no step runs \`cli upkeep run msum gdfl NSE/INDEX --workers 8\``:
the store-steps rows are the Mac's (rehearsal figures).

## Host reds (root), each passing as `nobody`

- store (5): `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
  `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
  `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`.
  The whole store lib binary as `nobody`: 434 passed, 0 failed, 2 ignored.
- pull (9): the nine of CONTEXT.md (alerts, gdfl_dropbox x2, gdfl_import ia16,
  gdfl_run ia2, census_store x2, zerodha_verify x2).
- cli (3): `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
  `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`,
  `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`.

## Next

Route B coverage of the D2 set (store, pull `upkeep`, cli `upkeep msum`);
close gaps; each new test shown red under a hand change; clippy; final gates;
the final report and `fix-BASE-MSUM-BACKFILL-MS1.json.md`.

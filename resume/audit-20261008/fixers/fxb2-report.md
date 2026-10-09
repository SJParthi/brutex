# fxb2 report — observability fixes (worktree /home/claude/wt-fx-b2, branch audit/fx-b2, base de932e5b)

| item | verdict | decision | test (invariant) | commit |
|---|---|---|---|---|
| sobs-5 pullrun coordinator silent | FIXED | D-4452 | `pullrun::a_press_logs_its_legs_and_verdict_under_one_run_and_nothing_else`, `a_press_that_dies_is_named_on_the_log`, `only_a_clean_press_finishes_at_info` (FXB2-09, -10) | 31c6fac1 |
| sobs-6 autopilot stop/panic/halt/stall | FIXED | D-4453 | `autopilot::a_stall_and_a_halt_are_logged_once_each_with_feed_and_month`, `an_empty_universe_logs_its_halt_once_and_not_once_a_pass`, `a_backfill_task_that_dies_is_named_and_its_status_halted`, `only_the_first_clock_wait_is_logged` (FXB2-11..14) | 31c6fac1 |
| sobs-7 F&O contract refusals | FIXED | D-4449 | `an_fno_landing_keeps_five_refusal_reasons_and_counts_all_six` (FXB2-05) | 31c6fac1 |
| sobs-8 recovery BLOCKED reason | FIXED | D-4448 | `every_refused_recovery_names_its_stage_and_reason_in_the_log`, `a_recovery_task_that_dies_is_named_with_its_stage` (FXB2-04) | 31c6fac1 |
| sobs-9 sweep-route stamp/budget refusals | FIXED | D-4447 | `an_unstamped_build_is_logged_on_every_launch_route`, `a_usable_server_budget_is_refused_before_the_slot_and_writes_nothing` (FXB2-03) | 31c6fac1 |
| sobs-10 /verify unlogged, no page | FIXED | D-4454 | `every_scrub_writes_its_verdict_to_the_log_once`; `web/tests/store-scrub.test.js` (FXB2-15, -16) | 31c6fac1, 36091640 |
| sobs-12 audit.rs dir sync | FIXED | D-4446 | `the_first_record_syncs_its_directory_and_the_root_and_later_ones_do_not`, `a_bare_journal_name_syncs_the_working_directory` (FXB2-02) | ceb5b0c5 |
| sobs-14 run attribution | FIXED | D-4451 | `telemetry::scope::a_run_filter_returns_only_the_runs_own_events`, `detail::admitted_work_keeps_the_callers_log_run` (FXB2-07, -08) | 27c39bb8, 31c6fac1 |
| sobs-15 unreadable lock note | FIXED | D-4445 | `an_unreadable_serve_lock_stamp_is_named_not_called_unstamped` (FXB2-01) | 31c6fac1 |
| L1-R3-2 broker_run early refusals | FIXED | D-4450 | `a_run_refused_before_it_starts_is_logged_with_its_reason` (FXB2-06) | 31c6fac1 |
| L1-R3-5 network retries | FIXED | D-4455 | `every_retry_on_the_bars_ladder_is_one_event_with_its_reason` + retry count in the rolling F&O test (FXB2-17) | 31c6fac1 |
| sobs-18 research-ledger endpoints | NOT-DONE (dropped by coordinator for usage pacing) | — | — | — |

Docs: fae2225e (D-4445..D-4455 at the end of 05-decisions; FXB2-01..17 in 04-invariants); 06-limits budget bullet closed by D-4447 in 31c6fac1.

sobs-18, what remains: the ~25 cli research ledgers expose commit and render functions, not paged structured readers. Closing it needs cli reader APIs, `api` `/ledgers.json` plus per-ledger paged routes, and a web listing.

Mutants (cargo-mutants not run, per the rules): every changed function has tests that assert level (both arms), target, message, fields and counts, plus transition truth tables (`publish_into_halt`, `finished_level`). Known gap: the one-line `clock_wait_published` call inside `fly` has no clock seam, so no test drives it; the helper itself is tested.

## Checks (final tree fae2225e)
- cargo fmt --all --check: clean
- cargo test -p telemetry --locked: 137+2+2 passed, 0 failed
- api lib tests (api-d29a1719c3818ec8, built from the committed tree): 1545 passed, 0 failed, 2 ignored. The four permission tests that failed in an earlier contended run passed here.
- cargo clippy -p api -p telemetry --all-targets -D warnings: NOT COMPLETED (stopped by the coordinator's usage pause). The test build emitted no warnings.
- web: `npm run check` 0 errors / 0 warnings; `node --test web/tests/*.test.js` 881 pass, 0 fail, 3 cancelled (the pre-existing ask.test.js cancellations)
- Gate W1: "The bundle corresponds to the source."
- Gates 11, 12, 15, 17, 19, 23: OK
- docs gates helper (10, 10b, 27, 27b): OK

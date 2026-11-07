# fix-BASE-COV-API-AS3.json.md

The FIX object of session BASE-COV-API-AS3 (shard G3, repair r3), as continuation -5 left it. Its `head` is the
code tip; this file is written in a later commit that touches only `reports/`.

```json
{
  "name": "BASE-COV-API-AS3",
  "branch": "cloud/cov-api-as3-20261105",
  "head": "4a86f4085b6ec5d0303b5e4de5a0a99801c868fe",
  "committed": true,
  "findings": [
    {
      "id": "G3 GAP rows of base-cov-api-gaps-r2.txt.md (shard G3, whole:api)",
      "status": "partial",
      "how": "Route B of record on 4a86f408: G3's 56 files (the 48 SHARDS.md gives G3 and 8 new test files) lines 63260/63962 (702 uncovered), branches 4726/5124 (398 sides). (F) 235 lines 312 sides; (U) 97 lines 52 sides, each with its proof; (H) 44 lines 2 sides; GAP 326 lines 32 sides: #[ignore]d tests 86 lines (L-BCA-3, the Mac), L-BCA-5 193/15 (booleanevidencejson 184/13: the production launch writes its artifact but ran past 13 minutes at opt-level 3; selectionv6json 9/2: the public cli::run ledger-v6 verb, not tried), L-BCA-6 18/7 (mastersrun), the host clock 18/4 (autopilot), races 4/4, a failing medium 3/1, reachable and not done 4/1 (indexstopqualificationjson 235 and 237, indexstopcandlesjson 117 True/118/120), test code 0. Continuation -5 closed bars_export 974 and indexstopqualificationjson 236.",
      "evidence": "reports/base-cloud-20261103/gaps-AS3.txt.md; reports/base-cloud-20261103/fu-AS3.txt.md; Route B log /var/tmp/as3/routeb5.log (this host)"
    },
    {
      "id": "continuation -4's NEXT 1-3 (reachable production rows)",
      "status": "partial",
      "how": "bars_export.rs 974 by an_end_read_that_cannot_open_its_month_refuses_the_export_by_name (a header zeroed in place with its stamp kept, so only the descending end read opens the damaged month); indexstopqualificationjson.rs 236 by a_forty_session_training_window_is_measured_whole_and_ranked_rejected (a second production launch: April and May training, every value measured, every setting Rejected). Not reached: 235 (a permissive policy file still fails worst_reward_risk, the operator's fixed 3:1 rule, and worst_trade_loss on the generated minutes), 237 (a missing minute is refused by the calendar receipt before any metric), indexstopcandlesjson 117-120 (only a hand re-sealed source context reaches a page over 8 MiB).",
      "evidence": "commits 8b311fdc, 042384a3, 4a86f408; red-AS3.txt.md run red5; gaps-AS3.txt.md rows for 235, 237 and 117-120"
    },
    {
      "id": "verify r3 item (3): bars_series.rs a_summary_that_does_not_describe_its_file_is_refused_field_by_field (9 / 1 / 0)",
      "status": "fixed",
      "how": "11 singles / 3 combinations / 2 out of the box, each asserted and named on its Extremes: line",
      "evidence": "api::bars_series::tests::a_summary_that_does_not_describe_its_file_is_refused_field_by_field"
    },
    {
      "id": "verify r3 item (3): feeds_route.rs small-size timing wrapper (none declared)",
      "status": "fixed",
      "how": "the_held_span_bench_runs_at_a_small_size runs the bench's own path at 20/30/40, 2/15/19 and 40/30/20 keys and names 8 singles, 2 combinations and 1 out of the box on its own Extremes: line, each asserted (red under a hand change in red5); the_held_span_answers_at_every_edge_of_its_census beside it adds 6 / 2 / 3",
      "evidence": "api::server::feeds_route::tests::the_held_span_bench_runs_at_a_small_size; api::server::feeds_route::tests::the_held_span_answers_at_every_edge_of_its_census"
    },
    {
      "id": "frontierjson.rs and mastersrun.rs kept as verify r3 found them",
      "status": "fixed",
      "how": "every_edge_of_a_page_answers_or_refuses_by_name 10 / 2 / 2 and the refresh child's 9 configurations / 2 / 1, unchanged",
      "evidence": "api::frontierjson::tests::every_edge_of_a_page_answers_or_refuses_by_name; api::mastersrun::tests::a_refresh_with_no_credential_lands_the_public_masters_and_says_it_is_not_whole"
    },
    {
      "id": "each new test red under a hand change",
      "status": "fixed",
      "how": "all 111 new tests: 93 in red run 1, 4 in red run 2 (with the new asserts of 2 widened tests), 1 in red run 3, 11 in the -3 session's red3/red4, 2 in red5 (with the new asserts of the autopilot tick test and the feeds_route wrapper); every hand change in a local commit never pushed",
      "evidence": "reports/base-cloud-20261103/red-AS3.txt.md; the -3 session report at 44a0a656"
    },
    {
      "id": "G3's test code had GAP rows",
      "status": "fixed",
      "how": "none at the tip: the rows the new code added are (F) (two sides of a new assert) or (U) (the new launch test's unstamped arm)",
      "evidence": "gaps-AS3.txt.md: test code 0 lines 0 sides"
    },
    {
      "id": "(U) rows each with a proof from the code",
      "status": "fixed",
      "how": "149 rows (97 lines, 52 sides), each with its proof beside it; none rests on hard to reach, needs a fault or never happens",
      "evidence": "reports/base-cloud-20261103/fu-AS3.txt.md"
    },
    {
      "id": "root host reds in G3 files (3)",
      "status": "needs_operator",
      "how": "each fails as root in both runs of this round and passes as uid 65534 with this round's instrumented binary (api-bbc8861d417950e8); the rows only their later lines reach are (H), owed to the Mac",
      "evidence": "server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted; server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker; server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault"
    },
    {
      "id": "late red: server::cloud_mutant_tests_c::an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused",
      "status": "fixed",
      "how": "Cause found: the test read the newest NOT STARTED record of a journal read newest first (a page of at most 200 records) while autopilot::fly kept ticking dry Zerodha months with no wait (DRY_ROUNDS 2) up to Groww's fixed 2020-01 floor, where Groww wins the tie; a host fast enough to run the 120 dry ticks between two polls handed the test Groww's record. The test now reads the journal's oldest page and asserts the first tick's window is Zerodha's floor month (2015-01). Production code unchanged. Red before: 2 of 2 alone at :2492 with Groww's record; green after: 3 of 3 alone and in both full runs of this round; its new assert red under a hand change (red5).",
      "evidence": "commit 32fbc1ab; red-AS3.txt.md run red5; Route B log /var/tmp/as3/routeb5.log and stable log /var/tmp/as3/gate-test5.log (this host)"
    },
    {
      "id": "flaky: logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code",
      "status": "fixed",
      "how": "Cause found: the refusal and the 200 noise lines after it shared one millisecond on a tmpfs, so the ring's oldest kept line carried the refusal's stamp and audited_from < ring_from failed (logs.rs:10249). A 2 ms sleep after the refusal moves the clock first. 5 of 12 red alone before, 20 of 20 green after.",
      "evidence": "commit 32fbc1ab; red-AS3.txt.md (run red5 note)"
    },
    {
      "id": "Extremes: lines on every new test",
      "status": "fixed",
      "how": "The four new tests that lacked one (bars_series x2, the indexstoplaunch preview door, the saved qualification probe) name what they assert; every new test of this continuation has one.",
      "evidence": "commit 33fe1754"
    },
    {
      "id": "decision drafts D-8664 and D-8665, six limit drafts for docs/06",
      "status": "needs_operator",
      "how": "drafted in the report (D-8664 now records continuation -5's two test fixes; the L-BCA-5 draft names the Boolean launch and the hand re-seal finding); ASA writes the docs entries",
      "evidence": "reports/base-cloud-20261103/report-BASE-COV-API-AS3.md"
    },
    {
      "id": "NEEDS THE MAC",
      "status": "needs_operator",
      "how": "Route B whole:api on its pin over the merged tree; the ignored real-data tests (L-BCA-3); the (H) rows; macOS rows and the non-root 14-core tests; L-BCA-4; the G2 test this host skips; shadow D rows and attacks on real data; the D1 landing",
      "evidence": "report NEEDS THE MAC"
    },
    {
      "id": "mutants on touched modules",
      "status": "deferred (operator 30 Oct)",
      "how": "not run; each new test shown red under a hand change instead",
      "evidence": "red-AS3.txt.md"
    }
  ],
  "files_changed": [
    "crates/api/src/autopilot.rs",
    "crates/api/src/bars_export.rs",
    "crates/api/src/bars_route.rs",
    "crates/api/src/bars_series.rs",
    "crates/api/src/bars_window_route_tests.rs",
    "crates/api/src/booleanevidencejson_tests.rs",
    "crates/api/src/census_request_tests.rs",
    "crates/api/src/census_rollup.rs",
    "crates/api/src/census_rollup_cover_tests.rs",
    "crates/api/src/dayflags.rs",
    "crates/api/src/dayflags_cover_tests.rs",
    "crates/api/src/dayflags_filter_tests.rs",
    "crates/api/src/feeds_route.rs",
    "crates/api/src/frontierjson.rs",
    "crates/api/src/gaps_route.rs",
    "crates/api/src/http_admission_tests.rs",
    "crates/api/src/imports.rs",
    "crates/api/src/indexstopcandlesjson.rs",
    "crates/api/src/indexstopjson.rs",
    "crates/api/src/indexstopjson_cover_tests.rs",
    "crates/api/src/indexstoplaunch_tests.rs",
    "crates/api/src/indexstopqualificationjson.rs",
    "crates/api/src/indexstopqualificationjson_saved_tests.rs",
    "crates/api/src/ingest.rs",
    "crates/api/src/ingest_cover_tests.rs",
    "crates/api/src/ingest_refusals.rs",
    "crates/api/src/ladder.rs",
    "crates/api/src/logs.rs",
    "crates/api/src/logs_cover_tests.rs",
    "crates/api/src/logs_units_tests.rs",
    "crates/api/src/mastersrun.rs",
    "crates/api/src/month_change.rs",
    "crates/api/src/nse_identity.rs",
    "crates/api/src/ops_safety_tests.rs",
    "crates/api/src/optiongreeks.rs",
    "crates/api/src/optiongreeks_cover_tests.rs",
    "crates/api/src/owed_tests.rs",
    "crates/api/src/pool_table_tests.rs",
    "crates/api/src/serve_edge_tests.rs",
    "crates/api/src/server_cloud_mutant_tests_b.rs",
    "crates/api/src/server_cloud_mutant_tests_c.rs",
    "crates/api/src/server_cloud_mutant_tests_d.rs",
    "crates/api/src/strict_sweep_tests.rs",
    "crates/api/src/sweepevidence.rs",
    "crates/api/src/sweeprun.rs",
    "crates/api/src/sweeprun_cover_tests.rs",
    "crates/api/src/sweeprun_status_cache_tests.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 at 4a86f408",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 at 4a86f408 (stable 1.97.1)",
    "tests": "cargo test -p api --locked --no-fail-fast -- --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held rc 101 at 4a86f408 (stable 1.97.1, a stamped build of the committed tree): 3189 passed, 13 failed, 16 ignored (9 result lines; lib 3105 / 13 / 16, every other target and the doc tests green); the 13 are host reds (report): G3's three root reds and ten of other shards",
    "route_b_tests": "the same tests under Route B (nightly, instrumented, opt-level 0) at 4a86f408: 3189 passed, 13 failed (the same host reds), 16 ignored",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "no bench run this round: no p50 or p99 to give",
    "D3 coverage api Route B: 26 min (15:38-16:04 IST 7 Nov; build 3 min on a cold target, tests 7 min, readout 16 min), this host"
  ],
  "coverage": "Route B (the kit's d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly 1a98b1e13 2026-08-07, llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly), reading whole:api, set = the kit's deep-BASE-COV-API-6ea46e59c074-set.txt plus one coverage line for each of the 8 files the branch added, a clean worktree at 4a86f408, tests run with --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (a G2 test that hangs on a 4-core host): whole api lines 179147/183008, branches 13021/15386 (190 files read); G3 lines 63260/63962 (702 uncovered), branches 4726/5124 (398 uncovered sides), classified in gaps-AS3.txt.md and fu-AS3.txt.md. NOT PROVED: the 22 benches and lib.rs only (L-BCA-2). This host's figures, the builder's: the Mac re-measures.",
  "mutants": "deferred (operator 30 Oct): not run; each new test shown red under a hand change instead (red-AS3.txt.md)",
  "store_writes_needed": [],
  "notes": "STATUS: IN PROGRESS. The bar is not met for G3: rows still G3's to reach are indexstopqualificationjson 235 and 237 (rank's Admitted and Refused arms; this round's launches reach Rejected only), indexstopcandlesjson 117 True/118/120 (a hand re-sealed source context), booleanevidencejson 184/13 (the production Boolean launch writes its artifact but ran past 13 minutes at opt-level 3) and selectionv6json 9/2 (the public cli::run ledger-v6 verb, not tried): report NEXT 1-4. Every other uncovered G3 row is (F), (U) with its proof, (H) or owed outside (L-BCA-3, L-BCA-6, the host clock, races, a failing medium). Continuation -5 found and fixed both reds of 6 Nov (each the test's own, neither a host red) and put an Extremes: line on every new test. No production behaviour changed: outside test modules and test files the diff from START adds only #[cfg(test)] module declarations, doc comments and the test-only indexstopcandlesjson::render_at. Follow-up for the operator (not changed: production behaviour): the same damaged month is refused 400 bad_query by bars_export's end read and 500 export_not_whole by its first page. Host: a fresh cloud container, 4 cores, root; TMPDIR and the profile folder 1777 so the uid 65534 children write their profiles. Slips: the -3 session's (a shell command named the Py-language's interpreter, twice, and once in its helper b's shell); continuation -5 once (a discarded interpreter version check at the head of a scratch-tree edit command, which also used perl for that one-line edit); nothing written to any tracked file by either.",
  "held": false
}
```

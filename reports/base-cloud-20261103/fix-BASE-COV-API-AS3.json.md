# fix-BASE-COV-API-AS3.json.md

The FIX object of session BASE-COV-API-AS3 (shard G3, repair r3). Its `head` is the code tip; this file is written
in a later commit that touches only `reports/`.

```json
{
  "name": "BASE-COV-API-AS3",
  "branch": "cloud/cov-api-as3-20261105",
  "head": "26003a036453c0c9012f32d7e5215d756ddb7166",
  "committed": true,
  "findings": [
    {
      "id": "G3 GAP rows of base-cov-api-gaps-r2.txt.md (shard G3, whole:api)",
      "status": "partial",
      "how": "Route B of record on 26003a03: G3's 56 files (the 48 SHARDS.md gives G3 and 8 new test files) lines 63091/63794 (703 uncovered), branches 4719/5114 (395 sides). (F) 235 lines 310 sides; (U) 96 lines 51 sides, each with its proof; (H) 44 lines 2 sides; GAP 328 lines 32 sides: #[ignore]d tests 86 lines (L-BCA-3, the Mac), L-BCA-5 193/15 (booleanevidencejson 184/13, selectionv6json 9/2: a production-launch probe as the saved qualification's may reach them, not redone after the -3 session's was lost), L-BCA-6 18/7 (mastersrun), the host clock 18/4 (autopilot), races 4/4, a failing medium 3/1, reachable and not done 6/1 (indexstopqualificationjson 235-237, indexstopcandlesjson 117 True/118/120, bars_export 974), test code 0.",
      "evidence": "reports/base-cloud-20261103/gaps-AS3.txt.md; reports/base-cloud-20261103/fu-AS3.txt.md; Route B log /var/tmp/as3/routeb-final3.log (this host)"
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
      "how": "the_held_span_bench_runs_at_a_small_size names its 3 sizes (20, 30, 40 keys) and its extremes as those of the_held_span_answers_at_every_edge_of_its_census, which runs the same path where its answer turns: 6 / 2 / 3",
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
      "how": "all 109 new tests: 93 in red run 1, 4 in red run 2 (with the new asserts of 2 widened tests), 1 in red run 3, 11 in the -3 session's red3/red4; every hand change in a local commit never pushed",
      "evidence": "reports/base-cloud-20261103/red-AS3.txt.md; the -3 session report at 44a0a656"
    },
    {
      "id": "G3's test code had GAP rows",
      "status": "fixed",
      "how": "none at the tip: month_get's retry driven in a child that holds the store-read pool, the pool_table wait through the shared eventually, the probe-halt test asserting an empty store root; a multi-line let-else and a matches! no-match arm read as (F) by recipe step 4",
      "evidence": "gaps-AS3.txt.md: test code 0 lines 0 sides"
    },
    {
      "id": "(U) rows each with a proof from the code",
      "status": "fixed",
      "how": "147 rows (96 lines, 51 sides), each with its proof beside it; none rests on hard to reach, needs a fault or never happens",
      "evidence": "reports/base-cloud-20261103/fu-AS3.txt.md"
    },
    {
      "id": "root host reds in G3 files (3)",
      "status": "needs_operator",
      "how": "each fails as root and passes as uid 65534 with the same instrumented binary (api-7b94fb89521800a1); the rows only their later lines reach are (H), owed to the Mac",
      "evidence": "server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted; server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker; server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault"
    },
    {
      "id": "late red: server::cloud_mutant_tests_c::an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused",
      "status": "open",
      "how": "passed in all four Route B runs of 6 Nov up to 11:27 IST and fails since about 11:55 IST on both builds, alone too: the first NOT STARTED record is Groww's 2020-01 tick where the test expects the Zerodha month; its production path is unchanged on the branch; cause not found (report NEXT 0)",
      "evidence": "stable gate run /var/tmp/as3/test-final.log; reruns of the instrumented binary of record"
    },
    {
      "id": "flaky: logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code",
      "status": "open",
      "how": "unchanged since START; failed in the stable full run and 1 of 3 runs alone (the audit history reaches back 0 day(s))",
      "evidence": "/var/tmp/as3/test-final.log"
    },
    {
      "id": "decision drafts D-8664 and D-8665, six limit drafts for docs/06",
      "status": "needs_operator",
      "how": "drafted in the report; ASA writes the docs entries",
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
      "status": "open",
      "how": "not run this round; each new test shown red under a hand change instead",
      "evidence": "red-AS3.txt.md"
    },
    {
      "id": "Extremes: lines on every new test",
      "status": "open",
      "how": "105 of 109 carry one; bars_series a_journal_that_cannot_be_read_moves_no_index_life and an_instant_before_the_month_and_a_unit_past_the_span_answer_nothing, indexstoplaunch_tests the_configuration_preview_door_answers_json_when_nothing_is_configured and the saved qualification probe do not",
      "evidence": "report NEXT 5"
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
    "fmt": "cargo fmt --all --check rc 0 at 26003a03",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 at 26003a03 (stable 1.97.1)",
    "tests": "cargo test -p api --locked --no-fail-fast -- --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held rc 101 at 26003a03 (stable 1.97.1): 3185 passed, 15 failed, 16 ignored (9 result lines; lib 3101 / 15 / 16, every other target and the doc tests green); the 15: 13 host reds and 2 late reds (report)",
    "route_b_tests": "the same tests under Route B (nightly, instrumented) at 26003a03: 3186 passed, 14 failed (host reds: 13 lib, 1 integration), 16 ignored",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "no bench run this round: no p50 or p99 to give",
    "D3 coverage api Route B: 29 min (11:18-11:47 IST 6 Nov; build 2 min, tests 6 min, readout 20 min), this host"
  ],
  "coverage": "Route B (the kit's d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly 1a98b1e13 2026-08-07, llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly), reading whole:api, set = the kit's deep-BASE-COV-API-6ea46e59c074-set.txt plus one coverage line for each of the 8 files the branch added, tree 26003a03, tests run with --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (a G2 test that hangs on a 4-core host): whole api lines 178980/182840, branches 13015/15376 (190 files read); G3 lines 63091/63794 (703 uncovered), branches 4719/5114 (395 uncovered sides), classified in gaps-AS3.txt.md and fu-AS3.txt.md. NOT PROVED: the 22 benches and lib.rs only (L-BCA-2). This host's figures, the builder's: the Mac re-measures.",
  "mutants": "not run this round; each new test shown red under a hand change (red-AS3.txt.md)",
  "store_writes_needed": [],
  "notes": "STATUS: IN PROGRESS. The bar is not met: 7 reachable production rows not driven; booleanevidencejson and selectionv6json (L-BCA-5 names them) reachable by a production-launch probe not redone; one late red whose cause is not found; mutants not run; 4 new tests without an Extremes: line. No production behaviour changed: outside test modules and test files the diff adds only #[cfg(test)] module declarations, doc comments and the test-only indexstopcandlesjson::render_at. Host: a fresh cloud container, 4 cores, root; TMPDIR and the profile folder 1777 so the uid 65534 children write their profiles. Slips: the -3 session's (a shell command named the Py-language's interpreter, twice, and once in its helper b's shell; nothing written to any file); none in this continuation.",
  "held": false
}
```

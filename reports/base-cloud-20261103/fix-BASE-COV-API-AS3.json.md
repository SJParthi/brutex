# fix-BASE-COV-API-AS3.json.md

The FIX object of session BASE-COV-API-AS3 (shard G3, repair r3), as continuation -6 left it. Its `head` is the
code tip; this file is written in a later commit that touches only `reports/`.

```json
{
  "name": "BASE-COV-API-AS3",
  "branch": "cloud/cov-api-as3-20261105",
  "head": "c40fac07b7ab47af3f243f65158b430ab0cf4550",
  "committed": true,
  "findings": [
    {
      "id": "G3 GAP rows of base-cov-api-gaps-r2.txt.md (shard G3, whole:api)",
      "status": "partial",
      "how": "Route B of record on c40fac07: G3's 58 files (the 48 SHARDS.md gives G3 and 10 new test files) lines 64037/64569 (532 uncovered), branches 4756/5150 (394 sides). (F) 234 lines 314 sides; (U) 117 lines 59 sides, each with its proof; (H) 44 lines 2 sides; GAP 137 lines 19 sides: #[ignore]d tests 86 lines (L-BCA-3, the Mac), L-BCA-6 18/7 (mastersrun), the host clock 18/4 (autopilot), races 4/4, a failing medium 3/1, macOS only 2/1 (selectionv6json 175 False, 178, 180), reachable and not done 6/2 (indexstopqualificationjson 237; indexstopcandlesjson 117 True, 118, 120; booleanevidencejson 209 False, 212, 214, 467); L-BCA-5 none; test code none. Against continuation -5's record (98908e77): 186 rows closed, 168 lines 9 sides of them in booleanevidencejson.rs and 7 lines 1 side in selectionv6json.rs.",
      "evidence": "reports/base-cloud-20261103/gaps-AS3.txt.md; reports/base-cloud-20261103/fu-AS3.txt.md; Route B log /var/tmp/as3/routeb8.log (this host)"
    },
    {
      "id": "continuation -4's NEXT 1-3 (reachable production rows)",
      "status": "partial",
      "how": "bars_export.rs 974 by an_end_read_that_cannot_open_its_month_refuses_the_export_by_name; indexstopqualificationjson.rs 236 by a_forty_session_training_window_is_measured_whole_and_ranked_rejected; 235 by a_setting_that_passes_every_check_is_ranked_admitted (generated months that rise through every session, the 09:16 minute dipping past the stop on seven sessions of both windows, an admission policy at its floors: the long setting of the bullish bar passes every check and is ranked Admitted). Not reached: 237 (three routes ruled out by measurement: a missing minute is refused by the calendar receipt; a day whose readings straddle is a zero day, not unmeasured; an unmeasured calendar day exists only outside the measured window), indexstopcandlesjson 117-120 (only a hand re-sealed source context reaches a page over 8 MiB).",
      "evidence": "commits 8b311fdc, 042384a3, 4a86f408, f0c9b20f; red-AS3.txt.md runs red5 and red6; gaps-AS3.txt.md rows for 237 and 117-120"
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
      "how": "all 115 new tests: 93 in red run 1, 4 in red run 2 (with the new asserts of 2 widened tests), 1 in red run 3, 11 in the -3 session's red3/red4, 2 in red5, 1 in red6 (with the Rejected launch test again), 1 in red7 and 2 in red8 (continuation -6: the Selection V6 and Boolean evidence launch tests, two hand changes at once, stamped tree 08408911 over 32c1ac68, 0 passed; 2 failed, each red in its child at the assert its change aimed for). The held-reader repeat asks c40fac07 added drive the admission and qualification currency checks; a broken check reopens cold and answers the same page, so no hand change turns them red.",
      "evidence": "reports/base-cloud-20261103/red-AS3.txt.md; the -3 session report at 44a0a656"
    },
    {
      "id": "G3's test code had GAP rows",
      "status": "fixed",
      "how": "none at the tip. Route B on 32c1ac68 left logs.rs 10917 False, 10923-10925 and 10929 uncovered: the follower test's wait ran its not-yet side only on a host slower than the follower; c40fac07 waits through repeat_with/take_while/find and no row of it is uncovered. The rows the new launch tests added are (U) (their unstamped arms) or (F) (two sides of a new check).",
      "evidence": "gaps-AS3.txt.md: test code 0 lines 0 sides"
    },
    {
      "id": "(U) rows each with a proof from the code",
      "status": "fixed",
      "how": "176 rows (117 lines, 59 sides), each with its proof beside it; none rests on hard to reach, needs a fault or never happens. New this round: 13 lines 3 sides of booleanevidencejson.rs (a put into an object literal; the reconciled verdict indexstopqualificationjson.rs 317's proof covers; the page kinds Asked::parse admits; the row-for-row linkage Admission::open proves) and the two launch tests' unstamped arms",
      "evidence": "reports/base-cloud-20261103/fu-AS3.txt.md"
    },
    {
      "id": "root host reds in G3 files (3)",
      "status": "needs_operator",
      "how": "each fails as root in every run of this round (two stable, two Route B) and passes as uid 65534 with each Route B run's instrumented binary (api-bbc8861d417950e8, built at 32c1ac68 and at c40fac07); the rows only their later lines reach are (H), owed to the Mac",
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
      "id": "decision drafts D-8664 and D-8665, seven limit drafts for docs/06",
      "status": "needs_operator",
      "how": "drafted in the report: D-8664 now counts 115 tests and records continuation -6's follower wait; the L-BCA-5 draft is narrowed to indexstopcandlesjson 117 True/118/120 and carries the measured cost of the Boolean chain's currency checks at 1min; a seventh draft names selectionv6json's macOS-only 503; ASA writes the docs entries",
      "evidence": "reports/base-cloud-20261103/report-BASE-COV-API-AS3.md"
    },
    {
      "id": "NEEDS THE MAC",
      "status": "needs_operator",
      "how": "Route B whole:api on its pin over the merged tree; the ignored real-data tests (L-BCA-3); the (H) rows; macOS rows and the non-root 14-core tests; L-BCA-4; the G2 test this host skips; selectionv6json 175 False/178/180 (a child with a BRUTEX_STORE past about 155 KB of control characters: macOS only, Linux's MAX_ARG_STRLEN keeps the page under 8 MiB); shadow D rows and attacks on real data; the D1 landing",
      "evidence": "report NEEDS THE MAC"
    },
    {
      "id": "mutants on touched modules",
      "status": "deferred (operator 30 Oct)",
      "how": "not run; each new test shown red under a hand change instead",
      "evidence": "red-AS3.txt.md"
    },
    {
      "id": "a coverage row that moved between runs: bars_series.rs 387 True and 388 (Weighed memo re-put)",
      "status": "fixed",
      "how": "covered at 4a86f408 and not at f0c9b20f with the same code: only two readers racing to fill one month's time-of-day index reached the re-put of a held key; a_key_put_again_replaces_its_value_and_its_weight drives it directly (red7), covered at 98908e77",
      "evidence": "commit 98908e77; Route B logs /var/tmp/as3/routeb6.log and routeb7.log (this host)"
    },
    {
      "id": "continuation -5's NEXT 4: selectionv6json.rs saved arm (9 lines 2 sides)",
      "status": "fixed",
      "how": "a_ledger_v6_block_is_served_saved_and_every_damage_to_it_is_refused_by_name: the public cli::run ledger-v6 verb over generated NIFTY and BANKNIFTY minutes and days commits the 1min rung's sealed Selection V6 block (both families naturally extinct at 999,999 ppm) and refuses the run at 2min, whose files the store does not hold; /selection-v6.json serves the block saved beside seven absent rungs, a rerun reuses it byte for byte, and a doubled block, a torn tail, the block under 5min and a link in its place are refused by name. 7 lines 1 side closed; left 2 lines 1 side, macOS only (the over-ceiling 503)",
      "evidence": "commit 32c1ac68; api::selectionv6json::saved_tests::a_ledger_v6_block_is_served_saved_and_every_damage_to_it_is_refused_by_name; red-AS3.txt.md red8"
    },
    {
      "id": "continuation -5's NEXT 3: booleanevidencejson.rs saved renders (184 lines 13 sides)",
      "status": "partial",
      "how": "saved_boolean_evidence_is_served_by_every_reader_that_reads_it: the qualified Boolean search launch over a 60min rung folded from the generated minutes (at 1min one batch saved a 550 MB candidate body that every currency check rehashes; at 60min 13 MB and about 20 s at opt-level 3), April-May training, June-July later; every page kind of the statistics, admission and qualification readers and the search page's overview and rung detail. 168 lines 9 sides closed, 13 lines 3 sides (U) with proofs; left 3 lines 1 side reachable and not done (the page past 8 MiB: 209 False, 212, 214; a Romano-Wolf family measured whole: 467)",
      "evidence": "commits 32c1ac68, c40fac07; api::booleanevidencejson::saved_tests::saved_boolean_evidence_is_served_by_every_reader_that_reads_it; red-AS3.txt.md red8"
    },
    {
      "id": "continuation -5's NEXT 1: indexstopqualificationjson.rs 237 (rank's Refused arm)",
      "status": "partial",
      "how": "not reached; two more ways measured and not landed: a training minute with its prices zero is refused by the exact-minute ORB/GapFib overlay before any metric (CorruptMinute PriceNotPositive); the 2021-02-24 outage session is withheld (CHARTER_NON_REGULAR_IST_DAYS), all four settings Unmeasured. NEXT names a coarse rung, a window past the calendar's LAST_DAY, or a (U) proof",
      "evidence": "report: What continuation -6 did, NEXT 1; gaps-AS3.txt.md row for 237"
    },
    {
      "id": "continuation -5's NEXT 2: indexstopcandlesjson.rs 117 True, 118, 120",
      "status": "partial",
      "how": "not done in continuation -6: the hand re-seal of a launch's source context its gaps rows spell out",
      "evidence": "gaps-AS3.txt.md rows for 117-120; report NEXT 2"
    },
    {
      "id": "follow-up for the operator: ledger-v6's refusal of a rung with no stored files",
      "status": "needs_operator",
      "how": "cli (not G3's; production behaviour, not changed): the 2min refusal reads only 'refused: No such file or directory (os error 2)', naming neither the rung nor the file, though run_route's doc says every refusal names the rung and the stage",
      "evidence": "the first ledger-v6 run of selectionv6json_saved_tests (report: What continuation -6 did)"
    }
  ],
  "files_changed": [
    "crates/api/src/autopilot.rs",
    "crates/api/src/bars_export.rs",
    "crates/api/src/bars_route.rs",
    "crates/api/src/bars_series.rs",
    "crates/api/src/bars_window_route_tests.rs",
    "crates/api/src/booleanevidencejson.rs",
    "crates/api/src/booleanevidencejson_saved_tests.rs",
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
    "crates/api/src/selectionv6json.rs",
    "crates/api/src/selectionv6json_saved_tests.rs",
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
    "fmt": "cargo fmt --all --check rc 0 at c40fac07",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 at c40fac07 (stable 1.97.1)",
    "tests": "cargo test -p api --locked --no-fail-fast -- --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held rc 101 at c40fac07 (stable 1.97.1, a stamped build of the committed tree): 3193 passed, 13 failed, 16 ignored (9 result lines; lib 3109 / 13 / 16, every other target and the doc tests green); the 13 are host reds (report): G3's three root reds and ten of other shards; the same at 32c1ac68",
    "route_b_tests": "the same tests under Route B (nightly, instrumented, opt-level 0) of record at c40fac07: 3193 passed, 13 failed (the same 13), 16 ignored; at 32c1ac68, its test phase beside a stable build, 16 failed (the 13, pool_load and two timing tests of other shards, all green in the three runs after)",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "no bench run this round: no p50 or p99 to give",
    "the two new launch tests, one run each on this host at opt-level 3: the Selection V6 test 3 s, the Boolean evidence test 22 s",
    "D3 coverage api Route B of record: 32 min (19:28-20:00 IST 9 Nov; build 3 min, tests 9 min, readout 19 min), this host"
  ],
  "coverage": "Route B (the kit's d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly 1a98b1e13 2026-08-07, llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly), reading whole:api, set = the kit's deep-BASE-COV-API-6ea46e59c074-set.txt plus one coverage line for each of the 10 files the branch added (215), a clean worktree at c40fac07, tests run with --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (a G2 test that hangs on a 4-core host): whole api lines 180196/183615, branches 13073/15412 (190 files read); G3 lines 64037/64569 (532 uncovered), branches 4756/5150 (394 uncovered sides), classified in gaps-AS3.txt.md and fu-AS3.txt.md. NOT PROVED: the 22 benches and lib.rs only (L-BCA-2). This host's figures, the builder's: the Mac re-measures.",
  "mutants": "deferred (operator 30 Oct): not run; each new test shown red under a hand change instead (red-AS3.txt.md)",
  "store_writes_needed": [],
  "notes": "STATUS: IN PROGRESS. The bar is not met for G3: rows still G3's to reach are indexstopqualificationjson 237 (rank's Refused arm; five ways measured over two rounds, none reaches it), indexstopcandlesjson 117 True/118/120 (a hand re-sealed source context), booleanevidencejson 209 False/212/214 (a page past 8 MiB) and 467 (a Romano-Wolf family measured whole): report NEXT 1-4. selectionv6json 175 False/178/180 is macOS only (NEEDS THE MAC). Every other uncovered G3 row is (F), (U) with its proof, (H) or owed outside (L-BCA-3, L-BCA-6, the host clock, races, a failing medium). Continuation -6 reached booleanevidencejson.rs and selectionv6json.rs through production launches over generated stores (the qualified Boolean search at 60min; the public ledger-v6 verb), so L-BCA-5 holds no G3 row; and it made the logs follower wait independent of the host's speed. No production behaviour changed: outside test modules and test files the diff from START adds only #[cfg(test)] module declarations, doc comments and the test-only indexstopcandlesjson::render_at. Follow-ups for the operator (not changed: production behaviour): the same damaged month is refused 400 bad_query by bars_export's end read and 500 export_not_whole by its first page; ledger-v6's refusal of a rung with no stored files names neither the rung nor the file. Host: a fresh cloud container, 4 cores, root; TMPDIR and the profile folder 1777 so the uid 65534 children write their profiles. Slips: the -3 session's (a shell command named the Py-language's interpreter, twice, and once in its helper b's shell); continuation -5 once (a discarded interpreter version check at the head of a scratch-tree edit command, which also used perl for that one-line edit); continuation -6 once (a discarded perl version check); stray default .profraw files of continuation -6's --list probe made one stable build unstamped, removed and rebuilt; nothing written to any tracked file by any of them.",
  "held": false
}
```

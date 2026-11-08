# fix-BASE-COV-API-AS2.json.md

The launcher's FIX report of shard BASE-COV-API-AS2 (repair round 3, G2), one json block.

```json
{
  "name": "BASE-COV-API-AS2",
  "branch": "cloud/cov-api-as2-20261105",
  "head": "edd467b09037937bf001bd4ba8d5e0f4415b647f",
  "committed": true,
  "findings": [
    {
      "id": "verify r3 blocking 1: G2's GAP rows (the whole-crate bar, D2 part A)",
      "status": "partial",
      "how": "Session -6 (8 Nov, token BASE-COV-API-AS2-20261105-6) on G2's files only: four new tests, each with an Extremes: line of at least 8 numbered cases (2 or more combined, 1 or more out of the box), each shown red under a hand change (red rounds 10 and 11: 0 passed, 3 failed; 0 passed, 1 failed, each at its own assertion): calendar_of.rs a_minute_bar_stamped_inside_the_minute_before_it_neither_extends_nor_splits_its_run (10: 7/2/1) and a_daily_bar_before_the_epoch_is_filed_under_no_month_and_the_months_after_it_are_withheld (11: 8/2/1), over bar files written legally and restamped in place with their blocks sealed again (store::block::seal over Layout::covered_byte_range), reaching 834:35 T and 551:32 F; server.rs a_serve_on_a_named_port_opens_its_own_address_and_says_what_the_handler_answered (11: 7/3/1), seven serves in each of two children whose PATH is one scratch folder, reaching 21052 (the port probed below every host's ephemeral range, so no other test's :0 can take it); and a_serve_on_a_host_that_starts_no_thread_says_each_warm_up_it_could_not_start (9: 6/2/1), census_cache's no-thread child shape through run_in_over, reaching 21067:24 T, 21068, 21073:24 T and 21074. server.rs 6632:12 T and 6633 were read and named a race-only row (no file mode refuses the re-offered member failure without refusing the close; the write seam is single-shot and G1's). Measured on this host at edd467b0: G2 (58 files) lines 70899/71878, branches 4779/5514: (F) 176 lines 557 sides, (U) 123 lines 90 sides, GAP 680 lines 88 sides (production 519/69, of which 389/28 carry an L-BCA-5 or L-BCA-6 tag and 50/20 are L-BCA-5's by D-8663's draft, the other 80/21 each named by kind in the report; test code 51/7, all head_deadline_body_tests.rs' behind the five host reds; inside #[ignore]d tests 110/12, L-BCA-3). Against 1fbb5ad6: production 524/75 to 519/69 (this session's tests closed 3 lines and 4 sides; the wall clock's finished_day_only pair swapped sides with the date). Earlier sessions: see the report (session -5 took production from 950/269 at b3d5321c to 524/75). The bar is not met; every row left is listed and named, and only items for the Mac, the assembly or the operator are left.",
      "evidence": "reports/base-cloud-20261103/gaps-AS2.txt.md, fu-AS2.txt.md (Route B on edd467b0); report-BASE-COV-API-AS2.md (Session -6: its tests, red rounds 10 and 11, the final measurement, Left and whose)"
    },
    {
      "id": "verify r3 blocking 2: sources_tests.rs:2113 print site refused by gate 23",
      "status": "fixed",
      "how": "026e8be7 (session -1): the crowd child proves itself by a child-ran file its parent reads; no print site is left. Gate 23's step run by hand again on 8e94e0b8 and edd467b0 (outputs equal line for line, and equal to session -5's) and on f01d7a0e4, each with its own scanner and runtime tool built as the job builds them: the outputs differ in one line (993 against 997 tracked source files) and name no sources_tests.rs site and no site of this shard's; session -6's tests add no print site (the serve-opening and no-thread parents read the children's streams).",
      "evidence": "report-BASE-COV-API-AS2.md, Gate 23 by hand on 8e94e0b8, and on edd467b0 under the no-thread section"
    },
    {
      "id": "verify r3 blocking 3: the 8-extremes rule in G2's named files",
      "status": "fixed",
      "how": "sources_tests.rs 13 (10/2/1); research_doors_tests.rs 12 (8/2/2); dayflags_rules_tests.rs day band 10 (7/2/1), cold share page 11 (8/2/1), profilers' pages 11 (8/2/1); disk_route.rs 10 (7/2/1) and three tests of 11 (8/2/1); server.rs pull runtime 11 (8/2/1), the futures route 9 (6/2/1), the spawning arm 10 (7/2/1), /pull/run 10 (7/2/1), the serve-opening test 11 (7/3/1), the no-thread serve 9 (6/2/1); calendar_of.rs's two new tests 10 (7/2/1) and 11 (8/2/1); every test sessions -5 and -6 added has its own Extremes: line.",
      "evidence": "report-BASE-COV-API-AS2.md, The 8-extremes counts and Session -5's tables"
    },
    {
      "id": "provenance.rs: bars_on_no_journal_day counted bars on a journal day whose lookup failed",
      "status": "fixed",
      "how": "day_json's reconstruction answers null for bars_on_no_journal_day whenever bar_lookup_errors is non-zero (it was n_valid minus the bars held on journal days, counting a failed day's bars as on none). Pinned by a_day_whose_bars_will_not_read_is_a_lookup_error_named_by_its_day (red on the old code, red round 4).",
      "evidence": "commit 0d054c1a; report Session -5"
    },
    {
      "id": "calendar_record.rs: a calendar withheld because its months would not read said the feed held no bar",
      "status": "fixed",
      "how": "record::answer's withheld reason names the months that would not read ('holds no readable <rung> bar ... N month(s) could not be read (...)') instead of '<feed> holds no <rung> bar', which read a read failure as an empty store and was kept. Pinned by a_month_that_opens_and_will_not_be_looked_up_is_named_and_the_rest_is_answered case (8).",
      "evidence": "commit 41d85f8b; report Session -5"
    },
    {
      "id": "F&O walk: a scripted credential's loopback base does not reach the walk's URLs (test hazard)",
      "status": "needs_operator",
      "how": "fno_walk builds the F&O URLs from Wire::spec (the descriptor's) and chain::month from lookup_of(feed); Credentials::Scripted replaces only the source's spec, and the walk reads Broker only to render. A test past discovery would reach the vendor's real host; this session's tests drive fno_futures through a loopback Wire instead. server.rs 11993 stays a GAP of L-BCA-6's kind. The operator chooses the seam.",
      "evidence": "commit a5a1b057; report Session -5, the finding on the futures route; D-8663 draft"
    },
    {
      "id": "HOST RED: five server::head_deadline_body_tests slow-reader tests on this Linux host",
      "status": "needs_mac",
      "how": "Cut at the stall deadline after 3.9-5.5 MB here; the same five fail the same way on START with the same command, and each alone with one thread (session -4's proof); red again in session -5's runs. Green on the Mac at verify r3. Their lines past the cut are owed to the Mac's Route B. Red again, the same five and no other G2 test, in session -6's run on edd467b0.",
      "evidence": "report-BASE-COV-API-AS2.md, Host reds"
    },
    {
      "id": "G3's gaps_route a_full_calendar_pool_refuses_the_census_and_the_audit hangs on a 4-core host",
      "status": "needs_owner",
      "how": "Not this shard's file (G3, AS3): skipped by name in this shard's Route B runs.",
      "evidence": "report-BASE-COV-API-AS2.md, Session -3"
    },
    {
      "id": "G3's server_cloud_mutant_tests_c an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused red in the 1fbb5ad6 run",
      "status": "needs_owner",
      "how": "Not this shard's file (G3, AS3). Red once (server_cloud_mutant_tests_c.rs:2466, the tick's note read 'BANKNIFTY: the broker did not answer with a window' where 'no instrument in the universe could be reached' was expected) in the 1fbb5ad6 run, whose lib code is 619a9b4e's byte for byte; it passed in the 518b2355, 2dc1534d and 619a9b4e runs. What the tick met changed with the hour or the load, not with G2's code. It passed in session -6's run on edd467b0.",
      "evidence": "report-BASE-COV-API-AS2.md, Measurement on 1fbb5ad6"
    },
    {
      "id": "quarantine_route.rs: the press's busy refusal named the store-read pool's bound; its doc claimed a Retry-After",
      "status": "fixed",
      "how": "b3d5321c (session -4): busy() takes the pool's bound; the doc says what is sent. Sending Retry-After is the operator's call.",
      "evidence": "report-BASE-COV-API-AS2.md, Batch 5"
    },
    {
      "id": "calendar_of.rs: withhold_unproved withheld nothing for a calendar span that begins before the epoch",
      "status": "fixed",
      "how": "29b4b636: withhold_unproved (D-1443) returned before withholding anything when the calendar's first day was before the epoch (u32::try_from of a negative day failed), so one daily bar stamped 1969, which only a file written outside the store's writer holds, left every month after it Closed: decades of holidays no daily file had read. The walk now starts at the epoch's month (calendar.first_day().max(0)); the days before the epoch, which no Day or YearMonth names, are left as session -5's test pins a span wholly before the epoch. Pinned by a_daily_bar_before_the_epoch_is_filed_under_no_month_and_the_months_after_it_are_withheld, red on the old code at its own case (3) (left Closed, right Unmeasured). Draft invariant row in D-8663 (7). Whether such a bar should instead refuse its whole month is the operator's call.",
      "evidence": "report-BASE-COV-API-AS2.md, Session -6, NEXT (2); red round 10"
    }
  ],
  "files_changed": [
    "crates/api/src/bars_sort.rs",
    "crates/api/src/bars_sort_tests.rs",
    "crates/api/src/bars_unknown_series_tests.rs",
    "crates/api/src/boolean_observation_budget.rs",
    "crates/api/src/boolean_search_budget.rs",
    "crates/api/src/booleancampaignjson_tests.rs",
    "crates/api/src/booleanjson_tests.rs",
    "crates/api/src/booleanlaunch_tests.rs",
    "crates/api/src/calendar_of.rs",
    "crates/api/src/calendar_record.rs",
    "crates/api/src/calendar_record_tests.rs",
    "crates/api/src/calendar_route.rs",
    "crates/api/src/calendar_route_tests.rs",
    "crates/api/src/census_cache.rs",
    "crates/api/src/census_tail_tests.rs",
    "crates/api/src/constituents.rs",
    "crates/api/src/dayflags_rules_tests.rs",
    "crates/api/src/disk_route.rs",
    "crates/api/src/disk_route_seal_tests.rs",
    "crates/api/src/fault_log.rs",
    "crates/api/src/fno_boundary_tests.rs",
    "crates/api/src/head_deadline_body_tests.rs",
    "crates/api/src/indexstopvixjson.rs",
    "crates/api/src/instruments_route.rs",
    "crates/api/src/livejson.rs",
    "crates/api/src/master.rs",
    "crates/api/src/operation_audit_tests.rs",
    "crates/api/src/provenance.rs",
    "crates/api/src/provenance_tests.rs",
    "crates/api/src/quarantine_route.rs",
    "crates/api/src/quarantine_route_tests.rs",
    "crates/api/src/recovery_control.rs",
    "crates/api/src/render.rs",
    "crates/api/src/research_doors_tests.rs",
    "crates/api/src/scrub_route.rs",
    "crates/api/src/server.rs",
    "crates/api/src/server_cloud_mutant_tests_e.rs",
    "crates/api/src/sources_day_truth_tests.rs",
    "crates/api/src/sources_tests.rs",
    "crates/api/src/speed_tests.rs",
    "crates/api/src/store_wire.rs",
    "crates/api/src/topjson.rs",
    "crates/api/src/trades.rs",
    "crates/api/src/verify.rs",
    "crates/api/tests/cloud_mutants_bars.rs",
    "crates/api/tests/logs_r3_24.rs",
    "crates/api/tests/store_step_forms.rs",
    "reports/base-cloud-20261103/CLOUD.md",
    "reports/base-cloud-20261103/CONTEXT.md",
    "reports/base-cloud-20261103/DECISIONS.md",
    "reports/base-cloud-20261103/MANIFEST.md",
    "reports/base-cloud-20261103/OWNER-BASE-COV-API-AS2.md",
    "reports/base-cloud-20261103/RULES-MAC-COV-API.md",
    "reports/base-cloud-20261103/SHARDS.md",
    "reports/base-cloud-20261103/base-cov-api-fu-read-overrides.txt.md",
    "reports/base-cloud-20261103/base-cov-api-fu.txt.md",
    "reports/base-cloud-20261103/base-cov-api-gaps-r2.txt.md",
    "reports/base-cloud-20261103/d3-covset.mjs.md",
    "reports/base-cloud-20261103/d3-recipe.txt.md",
    "reports/base-cloud-20261103/d3-route-b-cloud.sh.md",
    "reports/base-cloud-20261103/d3-route-b.sh.md",
    "reports/base-cloud-20261103/deep-BASE-COV-API-6ea46e59c074-set.txt.md",
    "reports/base-cloud-20261103/fix-BASE-COV-API-AS2.json.md",
    "reports/base-cloud-20261103/fix-BASE-COV-API-progress.md",
    "reports/base-cloud-20261103/fix-BASE-COV-API.json.md",
    "reports/base-cloud-20261103/fu-AS2.txt.md",
    "reports/base-cloud-20261103/gaps-AS2.txt.md",
    "reports/base-cloud-20261103/repair-BASE-COV-API-progress.md",
    "reports/base-cloud-20261103/replay-minus-reports.sh.md",
    "reports/base-cloud-20261103/report-BASE-COV-API-AS2.md",
    "reports/base-cloud-20261103/verify-BASE-COV-API-r3.json.md",
    "reports/base-cloud-20261103/verify-BASE-COV-API-r3.progress.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 (toolchain 1.97.1, the repository's pin) at edd467b0",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (1.97.1) at edd467b0",
    "tests": "Route B's instrumented cargo test -p api --locked --no-fail-fast at edd467b0, G3's 4-core hang skipped by name: passed 3201, failed 14, ignored 16 (lib 3117 passed, 13 failed, 16 ignored, 1 filtered out). The 14 reds are exactly the baseline's: eight lib tests of other shards' files (mode bits root defeats, a 4-core pool), tests/pool_load.rs (L-BCA-4) and G2's five head-deadline host reds proved on START. Every test this shard added or changed passed.",
    "gate23": "run by hand on edd467b0 (output equal to 8e94e0b8's and to session -5's runs line for line) and on f01d7a0e4: red on both (BASE-REDS-1023); one-line diff, the tracked-file count (997 against 993); no sources_tests.rs site, no site of this shard's",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": "none: this shard changed no bench and no timed route; gate 8 and every p99 claim are the Mac's",
  "coverage": "Route B whole:api (d3-route-b-cloud.sh, the kit's set file of 205 coverage files) on edd467b0, the code tip, on this Linux host, pins rustc 1.99.0-nightly (1a98b1e13 2026-08-07) and LLVM 23.1.0-rust-1.99.0-nightly: whole api lines 180819/184989, branches 12970/15502; G2 lines 70899/71878, branches 4779/5514: (F) 176 lines 557 sides, (U) 123 lines 90 sides, GAP 680 lines 88 sides (production 519/69, test code 51/7, inside #[ignore]d tests 110/12). The bar (100% of lines and branches past (F) and (U)) is not met for G2. NOT PROVED: the 22 crates/api/benches files and lib.rs (L-BCA-2). macOS-only lines are the Mac's and never claimed here. A readout of 8e94e0b8 was discarded (its object was replaced by another build mid-readout; see the report) and no figure of it is claimed.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Shard G2 of BASE-COV-API repair round 3, continuation -6 (8 Nov 2026, token BASE-COV-API-AS2-20261105-6). STATUS: BLOCKED OUTSIDE: only items for the Mac, the assembly or the operator are left (the report's 'Left, and whose'). Decision drafts D-8662 and D-8663 are in the report (session -6 adds items (7)-(10) to D-8663, among them the withhold_unproved fix and its draft invariant row); ASA writes the entries. Operator items: the saved-research projections stay L-BCA-5's; server.rs read_credential, credentialed_source and universe_resolve stay L-BCA-6's; speed::stage joins L-BCA-7's; the F&O walk's seam; a census warm row's took_ms is the kept read's; speed.rs's signal arms; two overflow observations in bars_sort.rs on inputs no request makes; emitted.rs (G1's) still lists a line now reached; whether a pre-epoch daily bar should refuse its whole month; whether the named race, clock, host, build and real-host rows stay named limits. ASA: the two indexstopvix tests belong in G1's indexstopvix_projection_tests.rs; D-1202's note. NEEDS THE MAC: Route B whole:api on its pin, the five head-deadline host reds, L-BCA-3's ignored tests, L-BCA-4's load-bound tests, non-root permission runs, gate 23 on its CI shell gates, the D1 landing, provenance.rs 1576-1581 (a Linux-only input), the other side of finished_day_only (the wall clock: this Sunday's run took 6855:68 T), and the platform items of session -6's two serve tests (a folder in the handler's place answered EACCES; a 1 EiB stack refused). Nothing was deleted that this session did not create; the scratch worktrees /home/user/rb6 and the f01d7a0e4 one are this session's.",
  "held": false
}
```

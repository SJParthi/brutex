# fix-BASE-COV-API-AS2.json.md

The launcher's FIX report of shard BASE-COV-API-AS2 (repair round 3, G2), one json block.

```json
{
  "name": "BASE-COV-API-AS2",
  "branch": "cloud/cov-api-as2-20261105",
  "head": "b0b3a034cbee945e99d426918546cba074e69e77",
  "committed": true,
  "findings": [
    {
      "id": "verify r3 blocking 1: G2's GAP rows (the whole-crate bar, D2 part A)",
      "status": "partial",
      "how": "Session -5 (7 Nov) on G2's files only: 77 new tests (test attributes added between b3d5321c and b0b3a034; each with an Extremes: line of at least 8 cases, 2 combined, 1 out of the box) and one case added to each of four existing server.rs tests, every one of the 81 shown red under a hand change (red rounds 4-6: 67, 4 and 10); test-code rows put in their (F) or branch-free forms (D-8662), no check weakened; rows proved (U) with their proofs from the code (fu-AS2.txt). Measured on this host at b0b3a034: G2 (58 files) lines 70235/71278, branches 4757/5510: (F) 173 lines 545 sides, (U) 123 lines 90 sides, GAP 747 lines 118 sides (production 541/78, of which 389/28 carry an L-BCA-5 or L-BCA-6 tag and 50/20 are L-BCA-5's by D-8663's draft, the other 102/30 each named by kind in the report: host faults, races, the wall clock, the real vendor host, start-up states, a hand-crafted bar file; test code 96/28: the five host reds' 51/7, run_serves_until_the_signal_and_exits_zero 29/9 (drafted as four children, not applied), 16/12 in session -5's own new tests (NEXT); inside #[ignore]d tests 110/12, L-BCA-3). Against b3d5321c: production 950/269 to 541/78, test code 118/74 to 96/28, (U) 56/35 to 123/90. The bar is not met; every row left is listed.",
      "evidence": "reports/base-cloud-20261103/gaps-AS2.txt.md, fu-AS2.txt.md; report-BASE-COV-API-AS2.md (Session -5)"
    },
    {
      "id": "verify r3 blocking 2: sources_tests.rs:2113 print site refused by gate 23",
      "status": "fixed",
      "how": "026e8be7 (session -1): the crowd child proves itself by a child-ran file its parent reads; no print site is left. Gate 23's step run by hand again on b0b3a034 and on f01d7a0e4, each with its own scanner and runtime tool built as the job builds them: the outputs differ in one line (993 against 997 tracked source files); the print surface is the same 56 lines and names no sources_tests.rs site.",
      "evidence": "report-BASE-COV-API-AS2.md, Gate 23 by hand (session -5)"
    },
    {
      "id": "verify r3 blocking 3: the 8-extremes rule in G2's named files",
      "status": "fixed",
      "how": "sources_tests.rs 13 (10/2/1); research_doors_tests.rs 12 (8/2/2); dayflags_rules_tests.rs day band 10 (7/2/1), cold share page 11 (8/2/1), profilers' pages 11 (8/2/1); disk_route.rs 10 (7/2/1) and three tests of 11 (8/2/1); server.rs pull runtime 11 (8/2/1), the futures route 9 (6/2/1), the spawning arm 10 (7/2/1), /pull/run 10 (7/2/1); every test session -5 added has its own Extremes: line.",
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
      "how": "Cut at the stall deadline after 3.9-5.5 MB here; the same five fail the same way on START with the same command, and each alone with one thread (session -4's proof); red again in session -5's runs. Green on the Mac at verify r3. Their lines past the cut are owed to the Mac's Route B.",
      "evidence": "report-BASE-COV-API-AS2.md, Host reds"
    },
    {
      "id": "G3's gaps_route a_full_calendar_pool_refuses_the_census_and_the_audit hangs on a 4-core host",
      "status": "needs_owner",
      "how": "Not this shard's file (G3, AS3): skipped by name in this shard's Route B runs.",
      "evidence": "report-BASE-COV-API-AS2.md, Session -3"
    },
    {
      "id": "quarantine_route.rs: the press's busy refusal named the store-read pool's bound; its doc claimed a Retry-After",
      "status": "fixed",
      "how": "b3d5321c (session -4): busy() takes the pool's bound; the doc says what is sent. Sending Retry-After is the operator's call.",
      "evidence": "report-BASE-COV-API-AS2.md, Batch 5"
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
    "fmt": "cargo fmt --all --check rc 0 (toolchain 1.97.1, the repository's pin) at b0b3a034",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (1.97.1) at b0b3a034",
    "tests": "Route B's instrumented cargo test -p api --locked --no-fail-fast at b0b3a034, G3's 4-core hang skipped by name: passed 3195, failed 15, ignored 16 (lib 3111 passed, 14 failed, 16 ignored, 1 filtered out). The reds: eight lib tests of other shards' files (mode bits root defeats, a 4-core pool), tests/pool_load.rs (L-BCA-4), G2's five head-deadline host reds proved on START, and G1's stop-bound test under the run's load (6.58 s against 6 s, L-BCA-4). Every test this shard added or changed passed.",
    "gate23": "run by hand on b0b3a034 and on f01d7a0e4: red on both (BASE-REDS-1023); one-line diff, the tracked-file count (997 against 993); no sources_tests.rs site, no site of this shard's",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": "none: this shard changed no bench and no timed route; gate 8 and every p99 claim are the Mac's",
  "coverage": "Route B whole:api (d3-route-b-cloud.sh, the kit's set file of 205 coverage files) on b0b3a034, the code tip, on this Linux host, pins rustc 1.99.0-nightly (1a98b1e13 2026-08-07) and LLVM 23.1.0-rust-1.99.0-nightly: whole api lines 180141/184389, branches 12948/15498; G2 lines 70235/71278, branches 4757/5510: (F) 173 lines 545 sides, (U) 123 lines 90 sides, GAP 747 lines 118 sides (production 541/78, test code 96/28, inside #[ignore]d tests 110/12). The bar (100% of lines and branches past (F) and (U)) is not met for G2. NOT PROVED: the 22 crates/api/benches files and lib.rs (L-BCA-2). macOS-only lines are the Mac's and never claimed here.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Shard G2 of BASE-COV-API repair round 3, continuation -5 (7 Nov 2026, token BASE-COV-API-AS2-20261105-5). Decision drafts D-8662 and D-8663 are in the report (session -5 adds five items to D-8663); ASA writes the entries. Operator items: the saved-research projections stay L-BCA-5's; server.rs read_credential, credentialed_source and universe_resolve stay L-BCA-6's; speed::stage joins L-BCA-7's; the F&O walk's seam (finding above); a census warm row's took_ms is the kept read's; speed.rs's signal arms need a signal this crate cannot raise; two overflow observations in bars_sort.rs on inputs no request makes; emitted.rs (G1's) still lists a line now reached; two indexstopvix tests belong in G1's indexstopvix_projection_tests.rs. NEEDS THE MAC: Route B whole:api on its pin, the five head-deadline host reds, L-BCA-3's ignored tests, L-BCA-4's load-bound tests, non-root permission runs, gate 23 on its CI shell gates, the D1 landing, provenance.rs 1576-1581 (a Linux-only input), the other side of finished_day_only (the wall clock). NEXT: the 16 lines and 12 sides of test code session -5's own new tests left, run_serves 29/9 by the drafted four children, server.rs 6632-6633 and the serve start-up lines, calendar_of.rs 551:32 F and 827:35 T; then Route B whole:api on the code tip again, the lists and this json.",
  "held": false
}
```

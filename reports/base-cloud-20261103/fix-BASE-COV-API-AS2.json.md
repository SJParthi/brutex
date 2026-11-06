# fix-BASE-COV-API-AS2.json.md

The launcher's FIX report of shard BASE-COV-API-AS2 (repair round 3, G2), one json block.

```json
{
  "name": "BASE-COV-API-AS2",
  "branch": "cloud/cov-api-as2-20261105",
  "head": "f1a06fe8ca670b1e4a9ec93819bafabeb2a9e05e",
  "committed": true,
  "findings": [
    {
      "id": "verify r3 blocking 1: G2's GAP rows (the whole-crate bar, D2 part A)",
      "status": "partial",
      "how": "Tests and restructures in G2's files only: 13 new tests, each with an Extremes: line of at least 8 cases (2 combined, 1 out of the box), test-code rows put in their (F) or branch-free forms, and 33 lines and 18 sides proved (U) from the code. Measured on this host at c7da9149: G2's GAP production 1,071/348 at the session's start (tip 725e11d5) to 1,002/300, test code 172/120 to 130/88. The bar is not met; what is left is listed row by row.",
      "evidence": "reports/base-cloud-20261103/gaps-AS2.txt.md, fu-AS2.txt.md; report-BASE-COV-API-AS2.md (baseline, interim measurement, tests by name)"
    },
    {
      "id": "verify r3 blocking 2: sources_tests.rs:2113 print site refused by gate 23",
      "status": "fixed",
      "how": "026e8be7 (session -1): the crowd child proves itself by a child-ran file its parent reads; no print site is left. Gate 23's step run by hand on this tree and on f01d7a0e4, each with its own source_scan and gates_runtime built as the job builds them: the outputs differ in one line (993 against 997 tracked source files); the print surface is the same 39 lines and names no sources_tests.rs site.",
      "evidence": "report-BASE-COV-API-AS2.md, Gate 23 by hand"
    },
    {
      "id": "verify r3 blocking 3: the 8-extremes rule in G2's named files",
      "status": "fixed",
      "how": "sources_tests.rs 13 (10/2/1); research_doors_tests.rs 12 (8/2/2); dayflags_rules_tests.rs day band 10 (7/2/1), cold share page 11 (8/2/1), profilers' pages 11 (8/2/1); disk_route.rs 10 (7/2/1); server.rs pull runtime 11 (8/2/1), each read from its Extremes: line (026e8be7, 39046efa, f13f4f57); every test this shard added has its own line.",
      "evidence": "report-BASE-COV-API-AS2.md, The 8-extremes counts"
    },
    {
      "id": "HOST RED: five server::head_deadline_body_tests slow-reader tests on this Linux host",
      "status": "needs_mac",
      "how": "Cut at the stall deadline after 3.9-5.5 MB here; the same five fail the same way on START with the same command, and each alone with one thread, so not load: this kernel's TCP buffers (tcp_wmem max 4 MiB, tcp_rmem max 32 MiB). Green on the Mac at verify r3. Their lines past the cut are owed to the Mac's Route B.",
      "evidence": "report-BASE-COV-API-AS2.md, Host reds"
    },
    {
      "id": "tests/cloud_mutants_bars.rs store-read pool test hung on a 4-core host",
      "status": "fixed",
      "how": "59d66729 (session -3): the holders park on a gate the test opens, and the test asserts this host's slot count, not a barrier of MAX + 1.",
      "evidence": "commit 59d66729"
    },
    {
      "id": "G3's gaps_route a_full_calendar_pool_refuses_the_census_and_the_audit hangs on a 4-core host",
      "status": "needs_owner",
      "how": "Not this shard's file (G3, AS3): skipped by name in this shard's Route B runs; the fix is the gate of tests/cloud_mutants_bars.rs.",
      "evidence": "report-BASE-COV-API-AS2.md, Session -3"
    }
  ],
  "files_changed": [
    "crates/api/src/bars_sort.rs",
    "crates/api/src/bars_sort_tests.rs",
    "crates/api/src/bars_unknown_series_tests.rs",
    "crates/api/src/boolean_observation_budget.rs",
    "crates/api/src/boolean_search_budget.rs",
    "crates/api/src/booleanjson_tests.rs",
    "crates/api/src/booleanlaunch_tests.rs",
    "crates/api/src/calendar_of.rs",
    "crates/api/src/calendar_record_tests.rs",
    "crates/api/src/calendar_route_tests.rs",
    "crates/api/src/census_tail_tests.rs",
    "crates/api/src/constituents.rs",
    "crates/api/src/dayflags_rules_tests.rs",
    "crates/api/src/disk_route.rs",
    "crates/api/src/fault_log.rs",
    "crates/api/src/fno_boundary_tests.rs",
    "crates/api/src/head_deadline_body_tests.rs",
    "crates/api/src/instruments_route.rs",
    "crates/api/src/livejson.rs",
    "crates/api/src/master.rs",
    "crates/api/src/operation_audit_tests.rs",
    "crates/api/src/provenance_tests.rs",
    "crates/api/src/quarantine_route_tests.rs",
    "crates/api/src/recovery_control.rs",
    "crates/api/src/render.rs",
    "crates/api/src/research_doors_tests.rs",
    "crates/api/src/scrub_route.rs",
    "crates/api/src/server.rs",
    "crates/api/src/server_cloud_mutant_tests_e.rs",
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
    "reports/base-cloud-20261103/fix-BASE-COV-API-progress.md",
    "reports/base-cloud-20261103/fix-BASE-COV-API.json.md",
    "reports/base-cloud-20261103/repair-BASE-COV-API-progress.md",
    "reports/base-cloud-20261103/replay-minus-reports.sh.md",
    "reports/base-cloud-20261103/report-BASE-COV-API-AS2.md",
    "reports/base-cloud-20261103/verify-BASE-COV-API-r3.json.md",
    "reports/base-cloud-20261103/verify-BASE-COV-API-r3.progress.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 (toolchain 1.97.1, the repository's pin) at f1a06fe8",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (1.97.1) at f1a06fe8",
    "tests": "Route B's instrumented cargo test -p api --locked --no-fail-fast at c7da9149, G3's 4-core hang skipped by name: passed 3110, failed 14, ignored 16; the 14 are nine tests of other shards' files (mode bits root defeats, a 4-core pool, tests/pool_load.rs, L-BCA-4) and G2's five head-deadline host reds proved on START. The tests f1a06fe8 adds or changes ran instrumented and filtered, green but for those host reds.",
    "gate23": "run by hand on this tree and on f01d7a0e4: red on both (BASE-REDS-1023); one-line diff, the tracked-file count; no sources_tests.rs site",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": "none: this shard changed no bench and no timed route; gate 8 and every p99 claim are the Mac's",
  "coverage": "Route B whole:api (d3-route-b-cloud.sh, the kit's set file of 205 coverage files) on c7da9149 on this Linux host, pins rustc 1.99.0-nightly (1a98b1e13 2026-08-07) and LLVM 23.1.0-rust-1.99.0-nightly: whole api lines 173618/178248, branches 12686/15364; G2 lines 63723/65137, branches 4499/5376: (F) 139 lines 459 sides, (U) 33 lines 18 sides, GAP 1242 lines 400 sides (production 1002/300, test code 130/88, inside #[ignore]d tests 110/12). THE BAR IS NOT MET. NOT PROVED: the 22 crates/api/benches/** files and crates/api/src/lib.rs (L-BCA-2). Lists: reports/base-cloud-20261103/gaps-AS2.txt.md and fu-AS2.txt.md. The code tip f1a06fe8 adds batch 3 after the measured tree, measured by filtered instrumented runs only.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Shard G2 of BASE-COV-API repair round 3, continuation -4 (6 Nov 2026). Decision drafts D-8662 (how G2's test-code rows are closed; G2's lists) and D-8663 (host reds and operator items) are in the report; ASA writes the entries. Operator items: the saved-research projections (indexstoprankingjson.rs, booleanqualification_projection.rs, past Reader::open in expressionsearchjson.rs, the launched search's observer in booleanlaunch.rs) stay L-BCA-5's; server.rs read_credential, credentialed_source and universe_resolve stay L-BCA-6's; speed::stage is a public function nothing calls and joins L-BCA-7's. NEEDS THE MAC: Route B whole:api on its pin (macOS-only lines are owed there), the five head-deadline host reds, the ignored real-data tests (L-BCA-3), the load-bound tests (L-BCA-4), non-root permission runs, gate 23 on its CI shell gates, the D1 landing. One shell command began with a no-op call to the interpreter the operator's rule forbids by name; it changed no file (report, A slip). NEXT: red round 2, more G2 production rows, the final Route B on the final tip.",
  "held": false
}
```

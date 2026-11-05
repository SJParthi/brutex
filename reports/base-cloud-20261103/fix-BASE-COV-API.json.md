# fix-BASE-COV-API.json.md

Source: `state/fix/round2/fix-BASE-COV-API.json` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````json
{
  "name": "BASE-COV-API",
  "branch": "fix/db-BASE-COV-API",
  "worktree": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-BASE-COV-API",
  "committed": true,
  "head": "6ea46e59c07488eeccff388407976041852aae2c",
  "held": false,
  "nothing_to_do": false,
  "coverage": "Route B (D3 recipe v4, d3-route-b.sh; rustc 1.99.0-nightly 1a98b1e13 2026-08-07, llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly), reading whole:api, set state/fix/round2/deep-BASE-COV-API-8091e72f7d85-set.txt (205 coverage files, 298 mutants lines), tree 8091e72f7d85 (its .rs files are the final tip's; the tip adds docs only), deep dir tgt/deep-10, 11:34-12:12 IST 3 Nov. TOTAL lines 171,626/176,239 (4,613 uncovered), branches 12,637/15,420 (2,783 uncovered sides), 182 files read; NOT PROVED: the 22 crates/api/benches/** files (no object holds them, L-BCA-2); (U) lib.rs. At check r2 (129c6da42a74): 4,853 lines and 2,848 sides. Strict classification, every (F) row read by hand: (F) 427 lines 1,053 sides (state/fix/round2/base-cov-api-fu.txt; 20 rows moved to GAP by reading, state/fix/round2/base-cov-api-fu-read-overrides.txt); GAP 4,186 lines 1,730 sides, production 3,514 lines 1,353 sides, inside #[ignore]d tests 233 lines 12 sides (state/fix/round2/base-cov-api-gaps-r2.txt, per file lines a/b and branches c/d, every row with its text). THE BAR IS NOT MET.",
  "mutants": "deferred (operator 30 Oct)",
  "findings": [
    {
      "id": "R2-BLOCK-1 whole-crate bar (D2 part A, recipe v4, whole:api)",
      "status": "partial",
      "how": "Repair 2 closed 240 lines and 65 sides net (4,853/2,848 -> 4,613/2,783 uncovered) while its own 663 new test lines are all covered; production GAP rows 3,724/1,381 -> 3,514/1,353. Still far over the ~2,000-line mark (D6 not decided). D-7347 records it and says Merge BASE-COV-API: must not happen on these figures.",
      "evidence": "scratch/deep-BASE-COV-API-logs/8091e72f7d85-routeb-r2.log; state/fix/round2/base-cov-api-gaps-r2.txt"
    },
    {
      "id": "R2-BLOCK-1b the 22 bench files NOT PROVED (L-BCA-2)",
      "status": "needs_operator",
      "how": "Unchanged: test = false keeps them out of every object cargo test reports. L-BCA-2 names the two ways to measure them (Route C after BASE-BENCH-1028, or a #[path] mount); the choice is the operator's.",
      "evidence": "Route B log: the 22 benches + lib.rs are the only NOT PROVED lines"
    },
    {
      "id": "R2-BLOCK-2 the part's own test code had GAPs it could close",
      "status": "fixed",
      "how": "served_routes_tests's door loops are assert!/assert_eq! (failing arms now (F)); sweeprun_accepted_tests reads the attempt as a string or fails (the u64 fallback that would hide a change of the answer is gone) and asserts the slot holds the press; basis_route_tests counts notes through a branchless helper with its own test (BCA-24).",
      "evidence": "Route B: served_routes_tests.rs 277/277 lines 4/4 branches; sweeprun_accepted_tests.rs 308/308 and 10/10; basis_route_tests.rs 653/656 (the 3 left are (F) message lines) and 4/4"
    },
    {
      "id": "R2-BLOCK-3 the (F)/(U) list marked rows that are not failing arms",
      "status": "fixed",
      "how": "Rebuilt from this round's Route B by the strict rule (the failing side of an assert-family macro or panic whose statement ran; || operands that do not fail by themselves, closures of .any/.find/.filter, and true sides are GAP; ignored-test bodies are GAP marked ignored, never (F)); then every (F) row read by hand and 20 more moved to GAP with their reasons. sweeprun_accepted_tests.rs:45 no longer exists. D-7346 item 6 supersedes D-7340 item 3 and D-7345 item 2.",
      "evidence": "state/fix/round2/base-cov-api-fu.txt (427 lines, 1,053 sides, (U) lib.rs); state/fix/round2/base-cov-api-fu-read-overrides.txt; progress file entry of 12:1x IST"
    },
    {
      "id": "R2-BLOCK-4a no new test shown failing first",
      "status": "fixed",
      "how": "50 hand behaviour changes in two scratch trees (never committed), one per new test of rounds 1 and 2: first tree 44 of 45 red at the assertion its change breaks; the 45th (a_receipt_nobody_saved...) red under a second change in the second tree with the 5 tests added after the first.",
      "evidence": "state/fix/round2/repair-BASE-COV-API-r2-red.diff + -r2-red-tests.log + -r2-red-summary.txt ('RED SUMMARY: 44 of 45'); -r2-red2.diff + -r2-red2-tests.log ('0 passed; 6 failed')"
    },
    {
      "id": "R2-BLOCK-4b 8 extreme cases per tested file",
      "status": "fixed",
      "how": "frontierjson every_edge_of_a_page_answers_or_refuses_by_name (10 edges, 2 combined, 2 out of the box); mastersrun refresh child under 9 HOME/transport configurations (2 combined; out of the box a credential 'file' that is a folder) plus its status/refresh doors with no usable HOME; sweeprun presses: attempt edges at both ends of u64, seven 400s, three doors pressed at once while an attempt is in flight (409, slot untouched).",
      "evidence": "api::frontierjson::tests::every_edge_of_a_page_answers_or_refuses_by_name; api::mastersrun::tests::a_refresh_with_no_credential_lands_the_public_masters_and_says_it_is_not_whole; api::sweeprun::accepted_tests::every_press_the_routes_accept_runs_to_its_end_and_reads_back (BCA-12 to BCA-15)"
    },
    {
      "id": "R2-FOLLOWUP OpsRead race: callers without apart_from_slot_owners",
      "status": "fixed",
      "how": "the_disk_timing (timed_against_the_child), the basis handler and router tests take the guard; every crowd this repair added runs alone in a child process.",
      "evidence": "disk_route.rs timed_against_the_child; basis_route_tests.rs; D-7346 item 3"
    },
    {
      "id": "R2-FOLLOWUP BCA-07 checked only the list pool",
      "status": "fixed",
      "how": "The lost pass ends by reading /pools.json: all nine pools running 0 and waiting 0; four more doors in the table (folder, provenance, calendar, backtest audit).",
      "evidence": "api::served_routes_tests::every_pooled_door_names_a_full_pool_and_a_lost_task; BCA-07 widened"
    },
    {
      "id": "R2-FOLLOWUP doc comment named 'a symbol list past its bound' with no row",
      "status": "fixed",
      "how": "The row is there: 121 symbols -> 400 'at most'.",
      "evidence": "api::served_routes_tests::a_door_refuses_what_it_cannot_read_through_the_router; shadow :9722 /ingest/refusals.json 121 symbols -> 400 'one request reads at most 120'"
    },
    {
      "id": "R2-FOLLOWUP L-BCA-4 should name census_cache warm_tests::a_census_change_never_blocks_a_request",
      "status": "fixed",
      "how": "L-BCA-4 names four load-bound tests now.",
      "evidence": "docs/06-limits.md L-BCA-4"
    },
    {
      "id": "R2-FOLLOWUP D-7340 item 3's old count",
      "status": "fixed",
      "how": "D-7346 item 6 says the counts of D-7340 item 3 and D-7345 item 2 are superseded by D-7347's.",
      "evidence": "docs/05-decisions.md D-7346, D-7347"
    },
    {
      "id": "R2-NEW-1 small-size dayflags timings asserted no answer",
      "status": "fixed",
      "how": "day_band_timed and cold_share_pages_timed now assert the page is the whole span, one record a day (BCA-20); shown red with a page that drops a day.",
      "evidence": "api::server::dayflags::rules::tests::the_day_band_timing_runs_at_a_small_size and two others; red round 1"
    },
    {
      "id": "R2-NEW-2 a non-deterministic test of this repair (two-exchange facets)",
      "status": "fixed",
      "how": "Passed alone, failed in the full run (last_close 4321 for 4323): its April row carried March's stamps, so the newest close was a map's order. April's row is stamped in April.",
      "evidence": "gates at a44f249f9 (failed) -> chain and final gates (passed); D-7346"
    },
    {
      "id": "R2-NEW-3 a test premise that a test build carries no stamp",
      "status": "fixed",
      "how": "A build from a clean tree is stamped (cli's build script); the press-door test now asks a body the halves cannot read (400 whatever the stamp) and keeps the crowded pass.",
      "evidence": "api::sweeprun::accepted_tests::each_press_door_answers_through_its_pool_and_says_when_it_is_full"
    },
    {
      "id": "R2-NEW-4 six public functions nothing calls (dead code)",
      "status": "needs_operator",
      "how": "Reported, not removed (L-BCA-7): detail::run_scan, ingest::discovery_ask, settings_route::snapshot_off_worker, census::Since::read_back, census::Manifests::partitions, census::Manifests::closes.",
      "evidence": "git grep over crates/: no caller; docs/06-limits.md L-BCA-7"
    },
    {
      "id": "R1 BLOCK-3/4 gaps_route not_owed_json arms; BLOCK-5 /basis.json row; head_deadline follow-up; BCA-07..BCA-10",
      "status": "fixed",
      "how": "Repair 1's fixes stand; each test shown red by hand this round (red round 1).",
      "evidence": "repair-BASE-COV-API-r2-red-tests.log; shadow :9722 /basis.json?symbol=SPLITCO&symbol=SPLITCO -> 400 text/plain 'more than once'"
    },
    {
      "id": "R1 D-7344 hand-run timings",
      "status": "partial",
      "how": "The small-size wrappers run and now assert their answers; the 16 #[ignore]d wrappers and the 4 real-store bodies stay uncovered (L-BCA-3, needs an operator decision).",
      "evidence": "233 lines and 12 sides marked ignored in base-cov-api-gaps-r2.txt"
    },
    {
      "id": "D3 mutants",
      "status": "fixed",
      "how": "deferred (operator 30 Oct): nothing ran; the set file keeps its 298 mutants lines and the line is appended to d3-mutants-deferred.txt (2026-11-03 11:34:19 IST).",
      "evidence": "state/fix/round2/deep-BASE-COV-API-8091e72f7d85-set.txt; state/fix/round2/d3-mutants-deferred.txt"
    }
  ],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 at 6453ea643 (tree of the tip 6ea46e59c; tgt/pool-3, lock held, every tracked file touched first)",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (56 s) at 6453ea643",
    "tests": "cargo test -p api --locked --no-fail-fast rc 0 in 707 s at 6453ea643: 3,087 passed, 0 failed, 16 ignored (9 result lines). Docs tests at 6453ea643: core lint/graph/findings/decision_citations/cost_invariants 40 passed; store docs/cited_commits 16 passed; cli limits_doc_drift 5 passed. Route B's instrumented run at 8091e72f7: 3,087 passed, 0 failed, 16 ignored. Not run: cargo test --workspace, cargo deny (CI shell gates red on feat/db-fix are BASE-REDS-1023's).",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "D3 coverage api route B: 38 min (11:34-12:12 IST; build 2 min, tests 8 min, readout 27 min); deep dir taken at once (deep-10, a slot holder's)",
    "shadow :9722 GET /backtest/run.json?attempt=1: p50 4.60 ms, p99 8.82 ms, max 25.25 ms, n 220 (serial curl time_total, final release 6453ea643)",
    "shadow :9722 GET /pools.json: p50 0.40 ms, p99 0.57 ms, max 0.71 ms, n 220 (same method)",
    "no production route or step changed in this repair: every change is test code or docs"
  ],
  "files_changed": [
    "crates/api/src/bars_series.rs",
    "crates/api/src/basis_route_tests.rs",
    "crates/api/src/dayflags_rules_tests.rs",
    "crates/api/src/detail.rs",
    "crates/api/src/disk_route.rs",
    "crates/api/src/emitted.rs",
    "crates/api/src/facets_tests.rs",
    "crates/api/src/frontierjson.rs",
    "crates/api/src/isolated.rs",
    "crates/api/src/logs_units_tests.rs",
    "crates/api/src/mastersrun.rs",
    "crates/api/src/served_routes_tests.rs",
    "crates/api/src/sources_tests.rs",
    "crates/api/src/store_route.rs",
    "crates/api/src/sweeprun_accepted_tests.rs",
    "docs/04-invariants.md",
    "docs/05-decisions.md",
    "docs/06-limits.md"
  ],
  "files_outside_ownership": [],
  "store_writes_needed": [],
  "notes": "DRAIN CHECK printed OPEN. Reading whole:api (SET OVERRIDE). d3.mutants = deferred (operator 30 Oct); mutants_proved false; survivors 0; timeouts 0; shards deferred (operator 30 Oct); not_proved 22 coverage files (crates/api/benches/**). THE BAR IS NOT MET: 4,613 lines and 2,783 sides uncovered, 4,186 GAP lines and 1,730 GAP sides, far over the ~2,000-line mark (the operator may split the part; D6 not decided). Blocking 2, 3 and 4 of check r2 and its follow-ups are closed; blocking 1 is partial. No production line changed in this repair; every change is test code or docs. D-block: D-7346 and D-7347 used now (D-7342..D-7345 by repair 1), D-7348 and D-7349 unused. feat/db-fix did not move (f01d7a0e4): nothing merged.\n\nSET FILE INCIDENT: the shared scratchpad/mkset.mjs (round 1's set script) had been rewritten for crates/pull by BASE-COV-PULL's agent at 10:22; my Route B job first wrote a 'reading whole:pull' set. Seen during the build; the set was rewritten from my own copy (scratchpad/bca-r2/mkset-api.mjs: whole:api, 205 coverage lines, 298 mutants lines identical to the first write) before the readout, the only step that reads it. The wrong one is kept as scratchpad/bca-r2/set-written-wrong-pull.txt.\n\nNOT DONE (a later round or the operator): (a) the 4,186 GAP lines and 1,730 GAP sides, row by row in state/fix/round2/base-cov-api-gaps-r2.txt, most in recovery.rs (assess, qualify_lifecycle, retry_day: a site with Zerodha masters and source bars), server.rs (vendor pull paths, L-BCA-6), dayflags.rs, the research projections (L-BCA-5); (b) L-BCA-2 benches, L-BCA-3 ignored tests, L-BCA-5 research fixtures, L-BCA-6 a host seam, L-BCA-7 six dead public functions: operator decisions; (c) L-BCA-4: audit::lock_wait_tests::eight_concurrent_appenders_lose_no_record failed once more under load at a44f249f9 (2 of 800 refused) and passed at the final tip; still needs a form the load cannot decide.\n\nPROCESS: one cargo slot session (low slot 2, got after 370 s) held 08:59-12:38 IST, 14 jobs; pool-3.lock held 09:04-12:38 and released; deep-10 held 11:34-12:12 and released. I stopped two of my own gate jobs (clippy refused this repair's own test code: Value indexing, a complex type, long functions, needless borrows; each fixed). Scratch left in place: scratch/bca-r2-red and scratch/bca-r2-red2 (red trees with hand changes, uncommitted), scratch/deep-BASE-COV-API (at 8091e72f7), scratch/deep-BASE-COV-API-prof/8091e72f7d85-10-B-r2, scratch/deep-BASE-COV-API-logs/8091e72f7d85-*, shadow/BASE-COV-API, tgt/bin-BASE-COV-API/release/api, scratchpad/bca-r2/*. One deletion of my own: a 0-byte marker scratchpad/bca-r2/logs/lock-mine that my job script made (rm -f when the session ended). Pushed: nothing. Main session: Route B measures api at 97.38% lines (171,626/176,239), a different instrument from CI's D-0677 floor."
}
`````

# verify-BASE-COV-API-r3.json.md

Source: `state/fix/round2/verify-BASE-COV-API-r3.json` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````json
{
  "name": "BASE-COV-API",
  "verdict": "fail",
  "head": "6ea46e59c07488eeccff388407976041852aae2c",
  "held": false,
  "needs_operator": false,
  "tip_moved": false,
  "blocking": [
    {
      "what": "The whole-crate coverage bar (operator decision D2 part A, D3 recipe v4, reading whole:api) is NOT met on the tip by the part's own record: D-7347 (docs/05-decisions.md, added by this part) states lines 171,626/176,239 (4,613 uncovered) and branches 12,637/15,420 (2,783 uncovered sides) at tree 8091e72f7 (same .rs files as the tip), of which 4,186 lines and 1,730 sides are GAP (3,514 and 1,353 in production code, most in recovery.rs, server.rs, dayflags.rs and the research projections), and says itself that 'Merge BASE-COV-API:' must not happen on these figures. DONE for this part is 'the bar holds on your final tip'. This check could not re-measure it (Route B never got a cargo slot, see coverage), so its own figures are NOT PROVED here; the shortfall is the part's own statement.",
      "where": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/base-cov-api-gaps-r2.txt (every GAP row, file:line and text; e.g. crates/api/src/recovery.rs, crates/api/src/server.rs, crates/api/src/dayflags.rs) and docs/05-decisions.md D-7347 on fix/db-BASE-COV-API",
      "reproduce": "W=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925; DD=$W/state/decisions-20261027; take a deep dir with $DD/d3-deep-take.sh, then $DD/d3-route-b.sh $W/scratch/deep-BASE-COV-API <K> <new profile folder> <new log folder> /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt api (tree at 6ea46e59c07488eeccff388407976041852aae2c); every UNCOVERED LINE / UNCOVERED BRANCH that is not an (F) failing arm of an assert/panic in test code or a proved (U) is a GAP",
      "expected": "every line and branch side of crates/api (src/, tests/, benches/) covered, except (F)/(U) items listed with proof (recipe v4 step 4); or the operator splits the part (D6, not decided)"
    },
    {
      "what": "A CI refusal the tip shows and feat/db-fix does not (D1: BLOCKING): gate 23 ('stderr is not a log', clause 'the print surface is not the declared one') refuses a NEW undeclared print site this part added: crates/api/src/sources_tests.rs:2113 `println!(\"SOURCES-CROWD 429 on three doors\");` (commit 1c1394759, repair 2). On the tip gate 23 lists '> api/src/sources_tests.rs:println!:1' among the undeclared sites (39 lines); on feat/db-fix the list has 38 lines and no sources_tests entry. Every other red gate (27b 19 10 11 12 29, and the rest of 23) differs between tip and base only in counters, test order and thread ids.",
      "where": "crates/api/src/sources_tests.rs:2113 (fix/db-BASE-COV-API); gate 23 in .github/workflows/ci.yml",
      "reproduce": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/ci-shell-gates.sh <a clean tree at 6ea46e59c07488eeccff388407976041852aae2c> --jobs language-purity --only \"23\" -> GATE 23 rc=1 with '> api/src/sources_tests.rs:println!:1' after 'REFUSED  the print surface is not the declared one.'; the same command on a tree at feat/db-fix (f01d7a0e4) has no sources_tests line (this check: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-API-logs-r3/ci-shell-tip/logs/language-purity/019-23.log vs /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-API-logs-r3/ci-shell-base/logs/language-purity/019-23.log)",
      "expected": "no print site gate 23 does not declare: the crowd child proves itself the way the other children of this part do (a proof file the parent reads, e.g. served_routes_tests' child-ran file), or the site is declared in gate 23's list with its own decision"
    },
    {
      "what": "Spec step (2) obligation not met (the operator's standing rule, 'unconditional'): at least 8 extreme cases for EVERY file the part tests, 2 of them combinations of extremes at once and 1 out of the box. The fixer's 'R2-BLOCK-4b fixed' holds only for the three files check r2 named (frontierjson, mastersrun, the sweeprun presses). By the builder's own 'Extremes:' doc lines the other files it tested fall short: detail.rs a_pool_made_at_run_time_is_the_pool_a_static_is (3 singles, 0 combined, 0 out of the box); store_route.rs a_quarantined_months_row_names_it_and_every_other_row_says_none (3/0/0); facets_tests.rs an_underlying_held_on_two_exchanges_is_refused_until_one_is_named (4 + 1 combined + 1 out of the box); sources_tests.rs a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name (5 + 1 combined + 0); bars_series.rs a_summary_that_does_not_describe_its_file_is_refused_field_by_field (9 + 1 combined + 0); research_doors_tests.rs (8 + 1 combined + 2: one combination short); emitted.rs BCA-19 (none declared); and the small-size timing wrappers in audit.rs, audit_lock_wait_measure.rs, dayflags_rules_tests.rs, disk_route.rs, feeds_route.rs, import_journal.rs, pullrun_restart_tests.rs and server.rs (none).",
      "where": "crates/api/src/detail.rs:1986 (doc of a_pool_made_at_run_time_is_the_pool_a_static_is), crates/api/src/store_route.rs (a_quarantined_months_row_names_it_and_every_other_row_says_none), crates/api/src/facets_tests.rs:1643, crates/api/src/sources_tests.rs:2098, crates/api/src/bars_series.rs (a_summary_that_does_not_describe_its_file_is_refused_field_by_field), crates/api/src/research_doors_tests.rs, crates/api/src/emitted.rs:1569",
      "reproduce": "git -C wt/P-BASE-COV-API diff feat/db-fix...fix/db-BASE-COV-API -- <file> | grep -E '^\\+\\s*///' and read each new test's 'Extremes:' list (and its body)",
      "expected": "for each tested file: >= 8 extreme cases, >= 2 of them combinations of extremes at once, >= 1 out of the box, each asserted"
    }
  ],
  "reproduced": [
    {
      "id": "R2-BLOCK-2 own test code GAPs closed",
      "ok": false,
      "evidence": "NOT PROVED here: the Route B figures it cites (served_routes_tests.rs 277/277 and 4/4, sweeprun_accepted_tests.rs 308/308 and 10/10, basis_route_tests.rs 653/656 and 4/4) were not re-measured (no cargo slot for Route B in 3 h 18 min). The code changes it names ARE present: served_routes_tests' door loops are assert_eq!(full.status, 429, ..) / assert!(lost.status >= 500, ..); sweeprun_accepted_tests attempt_of reads parsed['attempt'].as_str().expect(..) (no u64 fallback); basis_route_tests counts notes through corporate_notes with its own test the_note_counter_counts_only_corporate_actions; all of them passed in this check's cargo test -p api."
    },
    {
      "id": "R2-BLOCK-3 (F)/(U) list rebuilt",
      "ok": true,
      "evidence": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/base-cov-api-fu.txt: no sweeprun_accepted_tests.rs:45 row; no (F) row inside the ignored test dayflags.rs the_real_days_are_labelled_and_the_answer_is_timed (lines 9741-9849; its rows are GAP marked ignored in base-cov-api-gaps-r2.txt); the 20 rows moved to GAP are in base-cov-api-fu-read-overrides.txt with reasons; D-7346 item 6 supersedes D-7340 item 3 and D-7345 item 2. The 'which ran' annotations could not be re-checked against a profile of this check."
    },
    {
      "id": "R2-BLOCK-4a every new test shown red by hand",
      "ok": true,
      "evidence": "from the evidence files, not re-executed: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/repair-BASE-COV-API-r2-red-summary.txt 'RED SUMMARY: 44 of 45 new tests fail under their hand changes' (GREEN(!) a_receipt_nobody_saved_is_refused_by_name_on_every_door), /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/repair-BASE-COV-API-r2-red2-tests.log 'test result: FAILED. 0 passed; 6 failed'; red.diff 65 hunk/file lines, red2.diff 12"
    },
    {
      "id": "R2-BLOCK-4b eight extremes per tested file",
      "ok": false,
      "evidence": "holds for the three files check r2 named: frontierjson every_edge_of_a_page_answers_or_refuses_by_name (10 edges, 2 combined, 2 out of the box), mastersrun a_refresh_with_no_credential_... (9 HOME/transport configurations, 2 combined, 1 out of the box: a credential folder), sweeprun presses (attempt edges, seven 400s, three doors at once in flight, the field twice); NOT for the other files the part tested (blocking 3)"
    },
    {
      "id": "R2-FOLLOWUP OpsRead race",
      "ok": true,
      "evidence": "disk_route.rs timed_against_the_child takes crate::detail::apart_from_slot_owners_blocking(); basis_route_tests handler/crowd/router tests take apart_from_slot_owners().await; the crowds of sweeprun/sources/served_routes run in child processes (crate::isolated)"
    },
    {
      "id": "R2-FOLLOWUP BCA-07 reads every pool",
      "ok": true,
      "evidence": "served_routes_tests full_pools_and_lost_tasks ends with GET /pools.json: rows == Named::ALL.len(), each running 0 and waiting 0; on this check's shadow :9820 /pools.json after the D rows and after the attacks: 9 pools running 0, waiting 0"
    },
    {
      "id": "R2-FOLLOWUP 'a symbol list past its bound' row",
      "ok": true,
      "evidence": "row present (past_the_bound, 121 symbols, 400 \"at most\"); shadow :9820 GET /ingest/refusals.json?feed=zerodha&symbols=S0..S120 (121) -> 400 application/json {\"error\":\"121 symbols named; one request reads at most 120. ...\"}; 120 symbols -> 200 with \"bounds\":{\"days_per_month\":4,\"symbols\":120"
    },
    {
      "id": "R2-FOLLOWUP L-BCA-4 names census_cache warm test",
      "ok": true,
      "evidence": "docs/06-limits.md L-BCA-4 names server::census_cache::warm_tests::a_census_change_never_blocks_a_request among four load-bound tests (and it failed in this check's gate run at load ~150)"
    },
    {
      "id": "R2-FOLLOWUP D-7340 item 3's old count",
      "ok": true,
      "evidence": "docs/05 D-7346 item 6: the counts of D-7340 item 3 (1,857 provisional) and D-7345 item 2 are superseded by D-7347's"
    },
    {
      "id": "R2-NEW-1 small-size dayflags timings assert the page",
      "ok": true,
      "evidence": "dayflags_rules_tests.rs day_band_timed asserts 92 days for every cold copy and warm page; profiled_pages asserts 92 days per answer; the three small-size tests passed in this check's cargo test -p api"
    },
    {
      "id": "R2-NEW-2 two-exchange facets test deterministic",
      "ok": true,
      "evidence": "facets_tests.rs: April's row stamped 5*86_400 s after March's (APRIL'S OWN STAMPS); an_underlying_held_on_two_exchanges_is_refused_until_one_is_named passed in this check's full run"
    },
    {
      "id": "R2-NEW-3 press-door test independent of the stamp",
      "ok": true,
      "evidence": "press_doors asks a body the halves cannot read (\"{}\") -> 400 with \"accepted\":false, then the crowded pass -> 503 \"admission is unavailable\" / \"no engine command was dispatched\"; passed in this check's run (a build from a clean tree, so stamped)"
    },
    {
      "id": "R1 BLOCK-5 /basis.json repeated field",
      "ok": true,
      "evidence": "shadow :9820 (own release, stamp 6ea46e59c) GET /basis.json?symbol=SPLITCO&symbol=SPLITCO -> 400 text/plain \"REFUSED — the query names \\\"symbol\\\" more than once ... Nothing was read or run.\"; repeated day -> 400 text/plain naming \"day\""
    },
    {
      "id": "R1 BLOCK-3/4 not_owed_json arms",
      "ok": true,
      "evidence": "shadow :9820 GET /gaps.json?feed=zerodha&exchange=NSE&segment=CASH&symbol=JIOFIN&timeframe=1min&month=2023-08 -> 200 \"not_owed\":[{\"code\":\"before_listing\",\"date\":\"2023-08-21\",\"days\":13,...,\"first\":\"2023-08-01\",\"last\":\"2023-08-18\",\"master\":{\"file\":\"NSE_CM_security_02112026.csv.gz\",\"sha256\":\"07193e3d...\"},\"minutes\":4875,\"reason\":\"before-listing\"}] (real data, the arm with a master); NIFTY 2024-01 -> \"not_owed\":[]; unit test a_not_owed_row_states_what_its_answer_has_and_only_that passed"
    },
    {
      "id": "R1 BCA-13 attempt edges on the real server",
      "ok": true,
      "evidence": "shadow :9820 /backtest/run.json?attempt=18446744073709551615 -> 200 \"requested_attempt\":\"18446744073709551615\",\"status\":\"unknown\"; attempt=1 -> 200 unknown; attempt=0, 01, 18446744073709551616, 184467440737095516150, attempt=, when=1 -> 400 \"exactly one canonical positive u64\"; attempt=1&attempt=1 -> 400 text/plain naming \"attempt\""
    },
    {
      "id": "R1 BCA-15 /masters and /masters/status.json",
      "ok": true,
      "evidence": "shadow :9820 GET /masters -> 200 text/html with <nav class=\"top\">; /masters/status.json -> 200 one row per source starting dhan_scrip.csv"
    },
    {
      "id": "D3 mutants deferred",
      "ok": true,
      "evidence": "this check appended its own line to /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/d3-mutants-deferred.txt ('2026-11-03 14:46:04 IST BASE-COV-API 6ea46e59c07488eeccff388407976041852aae2c /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt base-reds'); set file keeps 274 mutants lines"
    }
  ],
  "gates": {
    "fmt": "pass: cargo fmt --all --check rc 0 (15:26:37 IST; tgt/pool-3, lock held once, every tracked file touched at 15:15:42)",
    "clippy": "pass: cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (15:29:17 IST, Finished in 2m 40s)",
    "tests": "FAILED / NOT PROVED here: cargo test -p api --locked --no-fail-fast rc 101 (15:49:40 IST, 1,223 s): 3,086 passed, 1 failed, 16 ignored (9 result lines). Failed: server::census_cache::warm_tests::a_census_change_never_blocks_a_request (crates/api/src/census_warm_tests.rs:425 'a request waited: p50 526.917µs, p99 15.220375ms, max 58.0495ms', bound max < 50 ms, machine load ~150 on 14 cores); that test is not in this part's diff and the part's own L-BCA-4 names it load-bound; proved only by the D1 landing gate on the merged tree. Docs tests: cargo test -p core --test decision_citations --test findings --test graph --test lint --test cost_invariants 40 passed; -p store --test docs --test cited_commits 16 passed; -p cli --test limits_doc_drift --test pool_and_escape_docs 16 passed (all rc 0). D1 pre-check (CI shell gates, language-purity subset 0 1 1b 1f 10 10b 11 12 13 15 17 19 23 25 27 27b 28 29): RED 23 27b 19 10 11 12 29 on tip AND base; gate 23 has one refusal only on the tip (blocking 2); the other six: NOT PROVED here: red on feat/db-fix too (owner BASE-REDS-1023); proved only by the D1 landing gate on the merged tree. Not run: cargo test --workspace, cargo deny.",
    "web_build": "not applicable (web/ unchanged by the part)",
    "web_test": "not applicable (web/ unchanged by the part)"
  },
  "p50_p99": [
    "shadow :9820 (own release 6ea46e59c), n 220 sequential GETs each, curl time_total, new connection each, load ~70 on 14 cores while this check's gates ran:",
    "/basis.json?symbol=RELIANCE: p50 0.66 ms, p99 1.81 ms, max 3.66 ms (200 x220)",
    "/basis.json?symbol=RELIANCE&day=2025-03-04: p50 0.67 ms, p99 1.67 ms, max 2.17 ms (200 x220)",
    "/gaps.json JIOFIN 2023-08 (not-owed before_listing row with a master): p50 1.64 ms, p99 2.57 ms, max 3.56 ms (200 x220)",
    "/masters: p50 0.60 ms, p99 1.34 ms, max 1.66 ms; /masters/status.json: p50 0.65 ms, p99 1.60 ms, max 1.84 ms",
    "/backtest/run.json?attempt=1: p50 5.07 ms, p99 11.19 ms, max 30.94 ms (fixer: 4.60 / 8.82 / 25.25 ms, load not stated)",
    "/pools.json: p50 0.66 ms, p99 1.38 ms, max 1.62 ms (fixer: 0.40 / 0.57 / 0.71 ms)",
    "/ingest/refusals.json 121 symbols (400): p50 0.67 ms, p99 1.50 ms, max 3.81 ms; /basis.json repeated field (400): p50 0.58 ms, p99 1.30 ms, max 1.87 ms",
    "D3 coverage api route B: not run (queued 14:46-18:04 IST, 3 h 18 min, no slot); release build 10 min 40 s; cargo test -p api 1,223 s"
  ],
  "attacks": [
    "E1 COMBINATION (20 concurrent: 5 served /basis.json day + 5 repeated-field + 5 not-owed /gaps.json JIOFIN + 5 /backtest/run.json?attempt=u64::MAX): basis 200 x5, repeat 400 x5, gaps 200 x5 each byte-identical to the serial answer, attempt 200 x4 and 503 x1 \"invocation audit busy: another operation holds the journal lock; no work was queued\" (named, never a completion; pre-existing journal-lock behaviour); /pools.json after: 9 pools running 0 waiting 0",
    "E2 COMBINATION (crowd: 48 concurrent served /basis.json lookups + 8 repeated-field at once): 48 x 200 (the 4-slot ops pool with 32 waiting admitted all), 8 x 400 text/plain naming \"day\", 0 x 5xx; pools back to 0",
    "E3 huge: an 8,000-byte symbol -> 400 \"is not a stored share name\"; a 70,000-byte query -> 414 URI Too Long with content-length 0 (hyper's head bound, no words: pre-existing)",
    "E4 wrong case / garbage: symbol=reliance -> 400 \"not a stored share name (A-Z, 0-9, & - _, at most 64 bytes)\"; segment=cash on /gaps.json -> 400 \"byte 0x63 in the wrong case\"; symbol=REL%00IANCE -> 400 named; attempt=-1, %2B1, 1e3, %31 -> 400",
    "E5 day boundaries on /basis.json: 2024-02-29 200, 2023-02-29 400, 2025-12-31 200, 2026-01-01 200, 1970-01-01 200, 9999-12-31 200 (outside: \"the basis-run table is not served\"), 0000-01-01 400 \"the calendar holds 1970-01-01 to 9999-12-31\", 2025-3-4 400",
    "E6 month/year boundaries of a not-owed answer (JIOFIN listed 2023-08-21): 2023-07 not_owed 7,875 min one before_listing row 2023-07-03..2023-07-31 with its master; 2023-08 4,875 min 2023-08-01..2023-08-18; 2023-09, 2023-12, 2024-01 none",
    "E7 a span with a month before the listing (from=2023-07&to=2023-09) -> 400 \"\\\"\\\" is not a YYYY-MM month for \\\"month\\\"\" (the route reads one month; refused by name)",
    "E8 OUT OF THE BOX: two GETs pipelined in one write on one connection (/masters then /basis.json, the premise D-7343's test now relies on): both 200 in order, the page and the answer each whole",
    "E9 special sessions (no not-owed rows): NIFTY 2024-03 (Sat 2 Mar) expected 6,855 held 6,855; 2024-05 (Sat 18 May) 7,980/7,980; 2023-11 (Muhurat) 7,560 expected, 7,500 held, 60 lost: a data matter of the store, not this part",
    "E10 OUT OF THE BOX: Host: evil.example on /basis.json -> 403 naming the rebound check (D-0687), nothing dispatched",
    "E-PROBE write path (throwaway test in scratch/verify-BASE-COV-API-r3, never committed): refresh_over over an invented transport into an owned temp folder: run one 502 complete false missing zerodha (no credential), run two byte-identical (idempotent), run three SIGKILLed after two masters were rewritten one row longer while the third was asked: a reader sees every file whole (newline-terminated, no row of another width), the two new, the rest as before, no temporary left, no answer written; run four lands run two's bytes again; real masters dir: 0 files newer than 15:50 (never touched)"
  ],
  "followups": [
    "server::census_cache::warm_tests::a_census_change_never_blocks_a_request asserts a wall-clock max < 50 ms over 1,000 requests and failed at load ~150 (58.05 ms): needs a form the load cannot decide (owner: the census-cache warm tests, last touched by GF-1 b2acb33ba; named in this part's L-BCA-4)",
    "crates/api/src/detail.rs:1983-1985: the doc comment of a_pool_admits_its_bound_and_a_dropped_permit_frees_its_slot (W1-api2-11, D-1443) now heads the new a_pool_made_at_run_time_is_the_pool_a_static_is, and the old test has none (BASE-COV-API)",
    "crates/api/src/sweeprun_accepted_tests.rs in_a_child keys its proof file by the test name's LENGTH (sweep-crowd-<len>): two callers of equal length in one process would share one proof (today 91 and 82: latent) (BASE-COV-API)",
    "the 22 crates/api/benches/** files are NOT READ by any route (test = false; L-BCA-2): an operator decision (Route C after BASE-BENCH-1028, or a #[path] mount); lib.rs holds only mod items (no region)",
    "six public functions nothing calls (L-BCA-7: detail::run_scan, ingest::discovery_ask, settings_route::snapshot_off_worker, census::Since::read_back, census::Manifests::partitions, census::Manifests::closes): operator decision",
    "/backtest/run.json?attempt=<durable range> under concurrency answers 429/503 \"invocation audit busy\" instead of waiting for the journal lock (named; pre-existing; owner of the operation audit)",
    "a 70,000-byte request line is a 414 with an empty body (hyper head bound; pre-existing; owner of the server request bound)",
    "CI coverage floor D-0677: nothing measured here to raise it"
  ],
  "coverage": "NOT PROVED by this check. D3 Route B (recipe v4, d3-route-b.sh) never started: its runner (/private/tmp/claude-501/-Volumes-WD-BLACK-brutex/70a1a8d5-2e3a-40fe-8ee9-b2c576fe37af/scratchpad/vbca3/routeb-runner.sh, one cargo slot, wide tier '*BASE-COV-API*') waited in bin/cargo's queue 14:46-18:04 IST (3 h 18 min) with all 8 slots held by sessions of 45 min to 3 h 10 min; by the prompt's 20-minute rule it was treated as stuck and stopped (my own waiter; no slot, deep dir or profile was ever taken). Set written: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt (reading whole:api; 205 coverage files, every .rs of crates/api whole; 274 mutants lines, the D2 set of the diff). The part's own figures at the same .rs tree 8091e72f7 (D-7347): lines 171,626/176,239, branches 12,637/15,420, GAP 4,186 lines / 1,730 sides, (F) 427 lines / 1,053 sides, NOT reproduced here. The part's (F)/(U) list: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/base-cov-api-fu.txt; its per-file table and every GAP row: /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/base-cov-api-gaps-r2.txt.",
  "mutants": "deferred (operator 30 Oct)",
  "d3": {
    "step": "check",
    "tip": "6ea46e59c07488eeccff388407976041852aae2c",
    "reading": "whole:api (SET OVERRIDE: coverage = every crates/api source file whole, 205; mutants = the D2 set of the diff, 274 lines, deferred). Coverage NOT measured by this check (no cargo slot in 3 h 18 min); gaps below counts only what this check measured (none), not_proved counts every coverage file.",
    "set_file": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt",
    "set_size": 205,
    "coverage_proved": false,
    "mutants_proved": false,
    "gaps": 0,
    "survivors": 0,
    "timeouts": 0,
    "not_proved": 205,
    "not_applicable": false,
    "empty_set_evidence": "",
    "shards": "deferred (operator 30 Oct)",
    "mutants": "deferred (operator 30 Oct)"
  },
  "notes": "DRAIN CHECK printed OPEN. No earlier r3 progress file: fresh attempt; tip 6ea46e59c07488eeccff388407976041852aae2c (unchanged at 18:04), feat/db-fix f01d7a0e4 (unchanged; nothing merged after the part's cut). Reading whole:api. VERDICT fail on three blocking items: (1) the bar is not met by the part's own D-7347 record (not re-measured here); (2) a new gate-23 refusal (sources_tests.rs:2113 println!) that feat/db-fix does not show (D1); (3) the 8-extremes rule unmet for files tested in repairs 1-2. The cargo test -p api gate failed on one load-bound test the part did not touch: NOT PROVED here. REVIEW (B): production hunks are four and keep their answers: basis_route::answer built as a serde_json::Map (serde_json 1.0.151 features arbitrary_precision only, no preserve_order: BTreeMap keys, same bytes); gaps_route::not_owed_json writes the own-evidence pairs into the row (same keys and values; real BeforeListing row verified on the shadow); mastersrun::refresh_work -> refresh_over extraction (REFRESH FIFO lock still taken in refresh_work before it; no new path to a host); detail.rs cfg(test)-only lost_work seam inside the admitted task (absent from release builds; set only inside a child process). No loop a request can reach was added; no store write path changed (the refresh write path is pull::masters::land/replace_locked, unchanged: tmp + sync_all + rename; probed above). Docs are append-only (230 insertions, 0 deletions); D-7340..D-7347 used from block D-7340..D-7349 (D-7348, D-7349 unused); every test named in BCA-01..BCA-24 exists once. Fixtures are invented (SPLITCO, NOSUCHCO, TWOHOME, QSTORE, RELIANCE only in a synthetic master row with an invented FinInstrmId); no vendor row copied. The new mastersrun tests reach no host: every configuration lacks a usable credential, and refresh_work is called only where no masters directory can be found. ORDER: per the operator's full-speed parallel instruction the release build and the step-C gates ran in ONE pool-3 hold (15:15-16:05 IST, one cargo slot, slot 3 after a 26-min wait), in parallel with D and E (served from the copied release, not from the pool), on the same fixed tip. An INTERIM server of the fixer's binary (stamp 6453ea643, same .rs) was used only to prepare the D/E scripts; every D row, latency and attack was then run on this check's own release build (stamp 6ea46e59c). D rows on own build: 22 of 22 ok true (/private/tmp/claude-501/-Volumes-WD-BLACK-brutex/70a1a8d5-2e3a-40fe-8ee9-b2c576fe37af/scratchpad/vbca3/drows-own.txt). Latencies at load ~70 are in p50_p99. Attacks in /private/tmp/claude-501/-Volumes-WD-BLACK-brutex/70a1a8d5-2e3a-40fe-8ee9-b2c576fe37af/scratchpad/vbca3/attacks-own.txt. LEFT IN PLACE (nothing deleted): worktrees scratch/verify-BASE-COV-API-r3 (tip + the uncommitted probe in crates/api/src/mastersrun.rs), scratch/verify-BASE-COV-API-r3ci (tip, clean), scratch/verify-BASE-COV-API-r3base (feat/db-fix); scratch/deep-BASE-COV-API (checked out at the tip, clean, lock released); logs /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/verify-BASE-COV-API-logs-r3 (session.log, s-release.log, g-*.log, probe-refresh.log, ci-shell-tip/base and their targets); shadow/BASE-COV-API-v (server stopped after this report); tgt/bin-BASE-COV-API-v/release (api, cli of the tip; r2's moved aside as *.r2-129c6da42) and tgt/bin-BASE-COV-API-v/interim-r3; probe output /var/folders/bk/hwtgtk4n18lcqxnc4kjj4fs40000gn/T/brutex-55874-probe-r3-refresh; scratchpad /private/tmp/claude-501/-Volumes-WD-BLACK-brutex/70a1a8d5-2e3a-40fe-8ee9-b2c576fe37af/scratchpad/vbca3 (scripts, outputs); stray temp files /tmp/vbca3-t-*.txt and /tmp/vbca3-b-*.txt (gate-23 normalised comparisons); progress /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/verify-BASE-COV-API-r3.progress.md; set file /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-API-6ea46e59c074-set.txt; deferral line appended to /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/d3-mutants-deferred.txt. Pushed nothing; :8080, wt/APP, tgt/app* and the real store untouched; GET only on :9820."
}
`````

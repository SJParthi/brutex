# fix-BASE-COV-API-AS1.json.md

Session BASE-COV-API-AS1, shard G1 of BASE-COV-API repair round 3. One fenced block below holds the FIX object.

```json
{
  "name": "BASE-COV-API-AS1",
  "branch": "cloud/cov-api-as1-20261105",
  "head": "b85ecf4deef037dfcf26ecd0979c35dc05203d08",
  "committed": true,
  "findings": [
    {
      "id": "R3-ITEM-3 eight extremes for each test G1's brief names (D-8660)",
      "status": "fixed",
      "how": "detail.rs 8/2/1; store_route.rs 10/2/1; facets_tests.rs (with facets.rs) 10/3/1; emitted.rs (BCA-19) 8/2/1; audit.rs 8/2/1; audit_lock_wait_measure.rs 8/2/1; import_journal.rs end-line 8/2/1 and field-count 9/2/1; pullrun_restart_tests.rs open-runs 8/2/1 and walks 8/2/1 (singles / at once / out of the box, each asserted and named on the Extremes: line). Small-size timing wrappers call the full-size entry at its smallest size first, then assert every extreme through a helper that returns what it measured. sweeprun_accepted_tests.rs press tests untouched (verify r3: met).",
      "evidence": "commits c4969e7a and 87e75385; red: ten hand behaviour changes in a scratch tree, ten tests red at their new assertions (test result: FAILED. 0 passed; 10 failed); report section Blocking item (3)"
    },
    {
      "id": "R3-ITEM-1 G1's GAP rows (D-8661)",
      "status": "partial",
      "how": "First session: 33 gap-closing tests over 11 new cover modules and existing test modules. Continuation (token -2, test code only, 5295f342 and b85ecf4d): 17 new tests (feed_checks query rules, calendar halts and owed years, environment-named check folders in children, tick CSV shapes; census StoreFilter, partition at the single name, read-back refusals, every refusal of the tail read, continues_file/same_read, mode-bit refusals run as uid 65534, Room::capped_reason; a two-hop alias; Votes::keep at its bound; a parked answer's bound; folder_json end to end; the emit scanner's helpers), test-only failing forms in candidatejson, backtest, bars and isolated, and 41 (U) rows with proofs (25 lines, 16 sides). Route B whole:api on b85ecf4d, G1's 87 files (SHARDS.md's 76 and the 11 test files AS1 added): lines 58,420/59,479 (1,059 uncovered), branches 4,178/4,966 (788 uncovered sides): GAP production 702 lines 277 sides (821/366 at 3f324ee8), GAP test 86/86 (116/112), GAP ignored 37/0, HOST 52/10, (F) 133/387, (U) 49/28. The bar is not met.",
      "evidence": "reports/base-cloud-20261103/gaps-AS1.txt.md and fu-AS1.txt.md (every row with its text and class); the Route B log of 5 Nov 20:45:57-21:20:04 IST in the session scratchpad (logs/rb1.log, logs/rb1/)"
    },
    {
      "id": "R3-RED every new test shown red under a hand change",
      "status": "fixed",
      "how": "33 production hand changes (messages, comparisons, returned values; no injected panics) applied at once in a scratch tree at 3f324ee8: all 33 gap-closing tests failed at the assertion each change targets (test result: FAILED. 0 passed; 33 failed; the sink test's child 0 passed; 1 failed); reverted (git status empty, HEAD 3f324ee8): test result: ok. 33 passed; 0 failed. Continuation: one set of 17 hand behaviour changes (two of them to the test helpers the new tests prove), built with Route B's instrumented build, turned all 17 new tests red at their targeted assertions (lib 0 passed 16 failed, the scanner's binary 0 passed 1 failed); reverted by git checkout, git status empty.",
      "evidence": "report table \"Red, every new test\" (test, file:line of the hand change, the change, FAILED)"
    },
    {
      "id": "R3-HOST the final runs' reds are this root 4-core host's",
      "status": "needs_mac",
      "how": "Continuation: Route B 13 reds, each among the first session's proven host reds (failing the same way at START, rb0-start); the stable run 14 lib reds: the same twelve, the stable-only autopilot red (5 of 5 at START), and logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code (G3's logs.rs), a millisecond collision: 6 of 20 red alone with the same binary, each with audited_from == ring_from; it passed under Route B and in the first session's stable run. Owed to the Mac (14 cores, non-root).",
      "evidence": "report section Host reds; logs rb0-start and rb3-3f324ee8; logs/flake-start-fail-*.log"
    },
    {
      "id": "R3-DECISIONS D-8660 and D-8661 have no ledger entry yet",
      "status": "needs_assembly",
      "how": "cargo test -p core --test decision_citations --locked names exactly D-8660 first cited at crates/api/src/audit.rs:5372 and D-8661 first cited at crates/api/src/aliases.rs:497; ASA writes both entries from the report's drafts (D-8661 gains items 5-9 for the continuation).",
      "evidence": "report section Decision drafts; every_cited_decision_heads_an_entry_or_is_named_pending output"
    },
    {
      "id": "R3-LISTS the gap and (F)/(U) lists read the 11 test files AS1 added",
      "status": "fixed",
      "how": "The first session's lists read SHARDS.md's 76 files only, so the failing arms of its 11 new *_cover_tests.rs files were never listed; the continuation's lists read all 87 (38 rows (F), three match-arm panics GAP test).",
      "evidence": "gaps-AS1.txt.md per-file table (87 files); fu-AS1.txt.md"
    }
  ],
  "files_changed": [
    "crates/api/src/aliases.rs",
    "crates/api/src/audit.rs",
    "crates/api/src/audit_json.rs",
    "crates/api/src/audit_json_cover_tests.rs",
    "crates/api/src/audit_lock_wait_measure.rs",
    "crates/api/src/backtest.rs",
    "crates/api/src/bars.rs",
    "crates/api/src/calendar_build.rs",
    "crates/api/src/candidatejson.rs",
    "crates/api/src/candidatejson_cover_tests.rs",
    "crates/api/src/census.rs",
    "crates/api/src/census_cover_tests.rs",
    "crates/api/src/chain.rs",
    "crates/api/src/chain_cover_tests.rs",
    "crates/api/src/credential_law_tests.rs",
    "crates/api/src/detail.rs",
    "crates/api/src/detail_cover_tests.rs",
    "crates/api/src/emitted.rs",
    "crates/api/src/facets_tests.rs",
    "crates/api/src/feed_checks_files.rs",
    "crates/api/src/feed_checks_files_cover_tests.rs",
    "crates/api/src/feed_checks_tests.rs",
    "crates/api/src/folder.rs",
    "crates/api/src/import_journal.rs",
    "crates/api/src/isolated.rs",
    "crates/api/src/merge.rs",
    "crates/api/src/merge_cover_tests.rs",
    "crates/api/src/pullrun.rs",
    "crates/api/src/pullrun_cover_tests.rs",
    "crates/api/src/pullrun_restart_tests.rs",
    "crates/api/src/recovery.rs",
    "crates/api/src/recovery_cover_tests.rs",
    "crates/api/src/recovery_journal.rs",
    "crates/api/src/recovery_journal_cover_tests.rs",
    "crates/api/src/repair_route.rs",
    "crates/api/src/repair_route_cover_tests.rs",
    "crates/api/src/settings_route_tests.rs",
    "crates/api/src/store_route.rs",
    "crates/api/tests/state_stream_emits.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 at b85ecf4d",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (stable 1.97.1), the run after the last code change, tree of b85ecf4d",
    "tests": "cargo test -p api --locked --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (stable 1.97.1) at b85ecf4d: lib 3,040 passed, 14 failed, 16 ignored, 1 filtered out; bin unit 4, binary 4, cloud_mutants_bars 58 (1 filtered out), logs_r3_24 6, pool_load 5, state_stream_emits 4, store_step_forms 3, doc 1, all passed. Every red a host red with its proof in the report (13 of the first session's list; one G3 timing test, 6 of 20 red alone with the same binary).",
    "tests_route_b": "the same suite instrumented under Route B at b85ecf4d: 3,126 passed, 13 failed (host reds, each among the first session's proven list), 16 ignored",
    "decision_citations": "cargo test -p core --test decision_citations --locked: red until ASA lands D-8660 and D-8661 (D-8660 first cited at crates/api/src/audit.rs:5372, D-8661 at crates/api/src/aliases.rs:497); nothing else",
    "gate_23_print_sites": "every changed file holds the same count of print, println, eprint, eprintln and dbg sites as at START; the new files hold none; the continuation added none (0 print, println, eprint, eprintln or dbg sites in its diff; its children prove they ran by a file their parent reads)"
  },
  "p50_p99": [
    "D3 coverage api route B on 3f324ee8: 29 min (15:46:25-16:14:54 IST; build 2 min, tests 6 min, readout 19 min), this cloud container (Linux x86_64, 4 cores, root)",
    "no bench run: crates/api/benches/ is ASA's; the small-size timing tests' own lines below are smoke checks of the measurement paths, not bench figures",
    "smoke, not bench: one run each of the six small-size timing tests, <lib test binary> --exact <test> --nocapture --test-threads=1, stable 1.97.1 dev profile (optimized + debuginfo), this 4-core container as root, 11:05-11:12 UTC 5 Nov",
    "audit::tests::the_append_timing_runs_at_a_small_size, appends_timed at its smallest size: n 3, p50 6 us, p99 6 us, max 48 us",
    "audit::lock_wait_measure::the_lock_wait_measurement_runs_at_a_small_size, open_run at 1/2/8 writers: n 8 each, p50 25/79/463 us, p99 348/194/875 us, max 348/194/875 us (0 refused)",
    "server::import_journal::tests::the_end_line_timing_runs_at_a_small_size, end_lines_folded_beside(3): n 3, p50 2.662 us, p99 21.787 us, max 21.787 us",
    "server::import_journal::tests::the_field_count_timing_runs_at_a_small_size, 8 fields: n 2, p50 23.806 us, p99 23.806 us, max 23.806 us; 4,096 fields: n 2, p50 3.793 ms",
    "pullrun::restart_tests::the_open_runs_timing_runs_at_a_small_size, restart_scan 65 records 64 open: n 2, p50 244 us, p99 244 us, max 244 us; status_json 64 open: n 2, p50 277 us",
    "pullrun::restart_tests::the_walks_worst_case_timing_runs_at_a_small_size, restart_scan 864 run records every close a guess: n 2, p50 2,904 us, p99 2,904 us, max 2,904 us",
    "the continuation ran no bench and its tests time nothing; the first session's figures stand"
  ],
  "coverage": "Route B (kit d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly; RUSTFLAGS -C instrument-coverage -Z coverage-options=branch, opt-level 0, llvm-profdata merge -sparse), reading whole:api on code tip b85ecf4d, set = the kit's 205 coverage files plus the 11 test files AS1 added (216), 5 Nov 2026 20:45:57-21:20:04 IST. Whole api: 193 files read, NOT PROVED 23 (the 22 benches and lib.rs, no object holds them, as the kit's run); lines 175,544/179,982 (4,438 uncovered), branches 12,903/15,560 (2,657 uncovered sides). G1's 87 files: lines 58,420/59,479 (1,059 uncovered), branches 4,178/4,966 (788 uncovered sides); the 76 of SHARDS.md alone 55,990/57,038 (1,048) and 4,134/4,892 (758), against 1,187 and 857 at 3f324ee8. Classified: GAP production 702 lines 277 sides, GAP test 86/86, GAP ignored 37/0, HOST 52/10, (F) 133/387, (U) 49/28. The bar (100% of api) is not met.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "STATUS IN PROGRESS: G1 GAP rows remain that a cloud session can close. Commit messages corrected in the report: c4969e7a says store_route 11 singles (it is 10); 3f324ee8 says pullrun.rs (7) rewritten match arms (it is 8, 15 in all). The edit tree checked START out once, to prove a host red, and returned to 3f324ee8; nothing of it was committed. CONTINUATION (token BASE-COV-API-AS1-20261105-2, 5 Nov 2026): took the branch at 3b58556d (owner file bf435421), test code only, code tip b85ecf4d; Route B whole:api, the stable gates, the red proof and both lists are of b85ecf4d. A slip, said plainly: one shell command of the continuation carried a stray version query of the interpreter the brief bans; it printed nothing, no file holds it and nothing ran through it. Scratch left in place (never in the repository): /home/user/as1-deep and /home/user/as1-stable (build trees), /dev/shm profile folders, the session scratchpad.",
  "held": false
}
```

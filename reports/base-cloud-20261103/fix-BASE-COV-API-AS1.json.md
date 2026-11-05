# fix-BASE-COV-API-AS1.json.md

Session BASE-COV-API-AS1, shard G1 of BASE-COV-API repair round 3. One fenced block below holds the FIX object.

```json
{
  "name": "BASE-COV-API-AS1",
  "branch": "cloud/cov-api-as1-20261105",
  "head": "5295f342cc8def6f6071a295319e241b9a54cb0b",
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
      "how": "Against START's kit rows for G1 (1,155 production GAP lines, 491 sides; 166 test-code lines, 123 sides): production GAP 821 lines and 366 sides remain (310 lines and 113 sides covered by new tests, 24 lines and 12 sides proved (U)); test-code GAP 116 lines and 112 sides remain. 33 new tests in 11 cover modules declared at the end of their production files (no production line moved) and 5 beside existing tests; test-only rewrites: never-called closures as one-line unreachable!, 15 other => panic! match arms as let-else failing arms, why_not_known narrowed to the guessed answer every caller passes, one assertion naming the one spelling it meets. Open: recovery.rs assess path (no fixture writes a Zerodha-mapped master with an index row), L-BCA-5 files, census.rs, facets.rs, settings_route.rs, feed_checks.rs, bars.rs and the rest in gaps-AS1.txt.md.",
      "evidence": "Route B on 3f324ee8 (log logs/rb3.log in the session scratchpad); reports/base-cloud-20261103/gaps-AS1.txt.md and fu-AS1.txt.md"
    },
    {
      "id": "R3-RED every new test shown red under a hand change",
      "status": "partial",
      "how": "33 production hand changes (messages, comparisons, returned values; no injected panics) applied at once in a scratch tree at 3f324ee8: all 33 gap-closing tests failed at the assertion each change targets (test result: FAILED. 0 passed; 33 failed; the sink test's child 0 passed; 1 failed); reverted (git status empty, HEAD 3f324ee8): test result: ok. 33 passed; 0 failed. CONTINUATION (token -2): the tests this continuation adds are not yet shown red; they are listed in the report and are shown red before line 1 says DONE.",
      "evidence": "report table \"Red, every new test\" (test, file:line of the hand change, the change, FAILED)"
    },
    {
      "id": "R3-HOST the final run's 15 reds are this root 4-core host's",
      "status": "needs_mac",
      "how": "14 panic at the same statement as in START's Route B on this host (rb0-start; three moved only by test code added above them); audit::lock_wait_tests::the_holder_is_named_through_every_in_process_hand_off was not red in START's single run here but failed 1 of 30 alone at START with the same assertion and 1 of 10 alone at the tip (a scheduling-sensitive sampler over code unchanged since START). Two tests skipped by name as 4-core hangs (G2's and G3's).",
      "evidence": "report section Host reds; logs rb0-start and rb3-3f324ee8; logs/flake-start-fail-*.log"
    },
    {
      "id": "R3-DECISIONS D-8660 and D-8661 have no ledger entry yet",
      "status": "needs_assembly",
      "how": "Drafts in the report; ASA writes docs/05-decisions.md. Until then cargo test -p core --test decision_citations is red, naming exactly D-8660 (first cited at crates/api/src/audit.rs:5372) and D-8661 (first cited at crates/api/src/audit_json_cover_tests.rs:2).",
      "evidence": "report section Decision drafts; every_cited_decision_heads_an_entry_or_is_named_pending output"
    }
  ],
  "files_changed": [
    "crates/api/src/aliases.rs",
    "crates/api/src/audit.rs",
    "crates/api/src/audit_json.rs",
    "crates/api/src/audit_json_cover_tests.rs",
    "crates/api/src/audit_lock_wait_measure.rs",
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
    "crates/api/src/store_route.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 at 3f324ee8",
    "clippy": "cargo clippy -p api --all-targets --locked -- -D warnings rc 0 (stable 1.97.1, the run after the last code change, tree of 3f324ee8)",
    "tests": "cargo test -p api --locked -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held (stable 1.97.1) at 3f324ee8: lib 3,026 passed, 13 failed, 16 ignored, 1 filtered out; every red a host red with proof (12 also red at START here, one red only in the stable build and 5 of 5 red at START with the same stable toolchain). Cargo stopped at the lib, so each other test binary of the same build was run directly with the same skips: bin 4, binary 4, cloud_mutants_bars 58 (+1 filtered), logs_r3_24 6, pool_load 5, state_stream_emits 3, store_step_forms 3, all ok; doc tests (cargo test -p api --locked --doc) 1 passed",
    "tests_route_b": "the same suite instrumented in Route B on 3f324ee8: 3,108 passed, 15 failed (host reds, proofs in the report), 16 ignored; a_full_calendar_pool_refuses_the_census_and_the_audit and spot_json_is_refused_by_name_when_every_store_read_slot_is_held skipped by name (4-core hangs)",
    "decision_citations": "cargo test -p core --test decision_citations --locked: red until ASA lands D-8660 and D-8661 (nothing else named)",
    "gate_23_print_sites": "every changed file holds the same count of print, println, eprint, eprintln and dbg sites as at START; the new files hold none"
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
    "pullrun::restart_tests::the_walks_worst_case_timing_runs_at_a_small_size, restart_scan 864 run records every close a guess: n 2, p50 2,904 us, p99 2,904 us, max 2,904 us"
  ],
  "coverage": "Route B (kit d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly; RUSTFLAGS -C instrument-coverage -Z coverage-options=branch, opt-level 0, llvm-profdata merge -sparse), reading whole:api on code tip 3f324ee8, set = the kit's 205 coverage files plus this session's 11 new test files (216). Whole api: 193 files read, NOT PROVED 23 (the 22 benches and lib.rs, no object holds them, as the kit's run); lines 174,120/178,686 (4,566 uncovered), branches 12,801/15,552 (2,751 uncovered sides). G1's 76 files: lines 55,261/56,448 (1,187 uncovered), branches 4,033/4,890 (857 uncovered sides); START on this host 1,561 lines and 970 sides uncovered. Classified: GAP production 821 lines 366 sides; GAP test 116 lines 112 sides; GAP ignored 37 lines; HOST 64 lines 10 sides (covered on the Mac's run of START, reached here only by host-red or skipped tests); (F) 125 lines 357 sides; (U) 24 lines 12 sides with proofs. THE BAR IS NOT MET.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "STATUS IN PROGRESS: G1 GAP rows remain that a cloud session can close. Commit messages corrected in the report: c4969e7a says store_route 11 singles (it is 10); 3f324ee8 says pullrun.rs (7) rewritten match arms (it is 8, 15 in all). The edit tree checked START out once, to prove a host red, and returned to 3f324ee8; nothing of it was committed. CONTINUATION (token BASE-COV-API-AS1-20261105-2, 5 Nov 2026): the code tip moved to 5295f342cc8def6f6071a295319e241b9a54cb0b (test code only: feed_checks_tests.rs, census_cover_tests.rs, aliases.rs, calendar_build.rs, isolated.rs). Interim figures from this host's instrumented runs of the new tests are in the report's continuation section; the Route B whole:api figures above are still those of 3f324ee8 until the continuation measures its own tip.",
  "held": false
}
```

# fix-BASE-COV-LIBS-LS2.json.md

The FIX record of session BASE-COV-LIBS-LS2 (shard LS2, crate runner), written by its continuation (TOKEN BASE-COV-LIBS-LS2-20261105-2). One fenced block below holds the json object.

```json
{
  "name": "BASE-COV-LIBS-LS2",
  "branch": "cloud/cov-libs-ls2-20261103",
  "head": "473f62e05b5b7220ad3c267a2a8e066a51c1188f",
  "committed": true,
  "findings": [
    {
      "id": "LS2-STEP1",
      "status": "fixed",
      "how": "cargo test -p runner --lib --locked --no-fail-fast did not compile at START: attempt 4 signal_candle_stop_tests.rs:888 called Evaluation::alignment(), which does not exist; it now reads fixture.prepared().alignment(). Commit 4f49dbf4.",
      "evidence": "second run: test result: ok. 800 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 68.52s (stable rustc 1.97.1)"
    },
    {
      "id": "LS2-COVERAGE",
      "status": "partial",
      "how": "123 new tests in crates/runner over the shard (99 by the first session, 24 by this continuation), every one shown red under a hand change of the rule it pins; test-code fallbacks that never ran made loud; branch-free equivalents in test code where no input reaches the other side; a written proof from the code for each row proved unreachable. Production lines untouched; crates/runner/src/lib.rs untouched.",
      "evidence": "Route B (pinned nightly rustc 1.99.0-nightly 1a98b1e13 2026-08-07, LLVM 23.1.0) on Linux x86_64: START 4f49dbf43e79 lines 46145/47136 branches 3349/4218; the first session's tip 2b998e4b5f6e lines 50775/51110 branches 3711/4170; code tip 473f62e05b5b lines 51836/52061 branches 3666/4014 (225 lines and 348 branch sides uncovered, 573 rows: 274 (F), 258 (U), 36 [FAULT], 5 [HASH], 0 [UNPROVEN], 0 [open]); test runner rc=0, tests passed 980 failed 0 ignored 0."
    },
    {
      "id": "LS2-LISTED",
      "status": "operator",
      "how": "Of the 299 uncovered rows that are not (F): 258 are (U) with a proof from the code; 36 need an allocator refusal or an address-space-sized input and 5 a blake3 collision. Decision draft D-8563 asks the operator to rule on the 41.",
      "evidence": "reports/base-cloud-20261103/gaps-LS2.txt.md (every row tagged) and fu-LS2.txt.md ((F) rows and (U) proofs)"
    },
    {
      "id": "LS2-OPEN",
      "status": "fixed",
      "how": "The 251 rows the first session left open (68 in production code, 183 in test code) and its 24 [UNPROVEN] rows are each closed in this continuation by a test that drives the input or by a proof from the code; no row is [open] or [UNPROVEN] at the code tip.",
      "evidence": "reports/base-cloud-20261103/gaps-LS2.txt.md at 473f62e05b5b: [open] 0, [UNPROVEN] 0"
    },
    {
      "id": "F-LS2-1",
      "status": "open",
      "how": "A finding in production code, not changed (no id of this shard covers a fix): grid::entry_fills prices a Long entry against its own bar under Anchor::PrintedExtreme (fills_at(entry_bar, entry_bar, ..), so the sell leg reads the ENTRY bar's low), while trade::round_trip sells at the EXIT bar's low. An entry bar whose low is below one tick (5 paisa) gives the level-less walk a priced trade and the exit grid a refused path in every cell: the legacy walk ranks no candidate of such a fold, and the replay door refuses with \"pricing-refusal branch carried no refusal fact\", a reason that misnames the cause. Not reachable at a real NSE price; a loud refusal, not a wrong number.",
      "evidence": "tests a_candidate_whose_every_cell_prices_no_trade_is_not_ranked (validate_cover_tests.rs) and a_replay_entering_on_a_bar_below_one_tick_is_refused_by_its_path_classifier (exit_grid_policy.rs)"
    }
  ],
  "files_changed": [
    "crates/runner/src/admission.rs",
    "crates/runner/src/audit.rs",
    "crates/runner/src/bootstrap.rs",
    "crates/runner/src/bootstrap_family_pass_tests.rs",
    "crates/runner/src/bootstrap_zero_v2_tests.rs",
    "crates/runner/src/excursion.rs",
    "crates/runner/src/exit_grid_policy.rs",
    "crates/runner/src/expression.rs",
    "crates/runner/src/expression_execution_tests.rs",
    "crates/runner/src/expression_oos_tests.rs",
    "crates/runner/src/expression_validation_tests.rs",
    "crates/runner/src/grid.rs",
    "crates/runner/src/grid_cover_tests.rs",
    "crates/runner/src/identity.rs",
    "crates/runner/src/outcome.rs",
    "crates/runner/src/pbo.rs",
    "crates/runner/src/portfolio.rs",
    "crates/runner/src/rank.rs",
    "crates/runner/src/report.rs",
    "crates/runner/src/report_cover_tests.rs",
    "crates/runner/src/resample.rs",
    "crates/runner/src/research_admission_projection.rs",
    "crates/runner/src/research_exit_grid_tests.rs",
    "crates/runner/src/signal_candle_stop_tests.rs",
    "crates/runner/src/significance_cover_tests.rs",
    "crates/runner/src/split.rs",
    "crates/runner/src/synthetic.rs",
    "crates/runner/src/topn.rs",
    "crates/runner/src/trade.rs",
    "crates/runner/src/validate.rs",
    "crates/runner/src/validate_cover_tests.rs",
    "crates/runner/tests/auto_probe_found_nothing.rs",
    "crates/runner/tests/auto_probe_reporter_refusal.rs",
    "crates/runner/tests/checkpointed_rank_refusals.rs",
    "crates/runner/tests/d_0694_record.rs",
    "crates/runner/tests/end_to_end.rs",
    "crates/runner/tests/expression_pricing.rs",
    "crates/runner/tests/foreign_slice_deadline_readiness.rs",
    "crates/runner/tests/hole_after_exit.rs",
    "crates/runner/tests/population_door_halt.rs",
    "crates/runner/tests/research_family_readiness.rs",
    "crates/runner/tests/unwired_validation_record.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS2.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-2b998e4b5f6e-set.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-473f62e05b5b-set.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-b8a346d26dce-set.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-e81f0aa6f411-set.txt.md",
    "reports/base-cloud-20261103/fix-BASE-COV-LIBS-LS2.json.md",
    "reports/base-cloud-20261103/fu-LS2.txt.md",
    "reports/base-cloud-20261103/gaps-LS2.txt.md",
    "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS2.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check: exit 0 at 473f62e05b5b (stable rustc 1.97.1)",
    "clippy": "cargo clippy -p runner --all-targets --locked -- -D warnings: exit 0 at 473f62e05b5b",
    "test": "cargo test -p runner --locked --no-fail-fast: exit 0 in 5 min 44 s, 19 result lines, 980 passed, 0 failed, 0 ignored at 473f62e05b5b (the instrumented Route B run at the same tip: tests passed 980 failed 0 ignored 0)",
    "forbidden_language_scan": "cargo test -p vocab --locked --test no_py_language: test result: ok. 9 passed; 0 failed, over the tracked tree with these reports staged",
    "route_b": "d3-route-b-cloud.sh at 473f62e05b5b: build rc 0 (34 artifacts, 10 reused, 18 test objects), test runner rc=0 (88 min), 19 profraw files merged, 74 of 75 set files read; NOT PROVED only crates/runner/benches/ratio.rs (no test object, operator item L-BCL-1); exit 3",
    "deny": "cargo deny check: not run (no Cargo.toml, Cargo.lock or dependency change in this shard)"
  },
  "p50_p99": "none: every change is test code or a test-only declaration, so no route or operation gained work and no bench is owed or claimed",
  "coverage": {
    "tool": "Route B: d3-route-b-cloud.sh, rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly, -C instrument-coverage -Z coverage-options=branch, opt-level 0, Linux x86_64 cloud host",
    "before": {
      "commit": "4f49dbf43e79",
      "lines": "46145/47136",
      "branches": "3349/4218"
    },
    "interim": {
      "commit": "2b998e4b5f6e",
      "lines": "50775/51110",
      "branches": "3711/4170"
    },
    "after": {
      "commit": "473f62e05b5b",
      "lines": "51836/52061",
      "branches": "3666/4014"
    },
    "bar_met": false,
    "lists": [
      "reports/base-cloud-20261103/gaps-LS2.txt.md",
      "reports/base-cloud-20261103/fu-LS2.txt.md"
    ],
    "macos": "owed to the Mac: runner has no cfg(target_os), cfg(unix) or pointer-width code, so no macOS-only line is expected, and none is claimed"
  },
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": [
    "STATUS: BLOCKED OUTSIDE: no row is [open] or [UNPROVEN] at the code tip. What remains is outside this shard's files: the operator's ruling on the 36 [FAULT] and 5 [HASH] rows (decision draft D-8563, which LSA writes into docs/), and the Mac's items.",
    "NEEDS THE MAC: Route B on its pin, macOS rows, non-root 14-core tests, ignored real-data tests (L-BCL-3: runner has none), the D1 landing, and the deferred mutants pass over the set file deep-BASE-COV-LIBS-LS2-473f62e05b5b-set.txt.md.",
    "Decision drafts in the report: D-8562 (the coverage figures and lists) and D-8563 (the allocator and hash-collision rows, and finding F-LS2-1, for the operator).",
    "crates/runner/src/lib.rs (LSA) is untouched: its paths are tested from crates/runner/tests/; red checks edited a local copy and restored it byte for byte, never committed.",
    "Line numbers: the first session's (U) proofs name lines as at 2b998e4b5f6e; at 473f62e05b5b they hold in every file but exit_grid_policy.rs, where lines 679-5862 are one higher (fu-LS2.txt.md says so at its head, and each moved [U] row in gaps-LS2.txt.md names its earlier line).",
    "This continuation also measured b8a346d26dce by Route B (lines 51834/52060, branches 3666/4014, 14 rows open). 13 of the 14 were (F); one was a test's ? on a line of its own, removed in 473f62e0. The lists are from the Route B at 473f62e05b5b.",
    "The extreme-case count per file (at least 8, 2 combined, 1 out of the box) was not audited per file.",
    "Process slips are recorded in the report. The first session's: a commit pushed before clippy passed, three commit messages with a wrong figure or count, a wrong (U) claim, two transient shell commands that named or ran the forbidden language. This continuation's: a Route B start stopped by a background time limit I had not raised, a red-check hand change that first matched a production line, and a test rewrite that left a ? on a line of its own, which cost one more Route B."
  ],
  "held": false
}
```

# fix-BASE-COV-LIBS-LS2.json.md

The FIX record of session BASE-COV-LIBS-LS2 (shard LS2, crate runner). One fenced block below holds the json object.

```json
{
  "name": "BASE-COV-LIBS-LS2",
  "branch": "cloud/cov-libs-ls2-20261103",
  "head": "2b998e4b5f6e4cc77ab8bd32de00ba8a4d165f0b",
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
      "how": "99 new tests in crates/runner (every one shown red under a hand change of the rule it pins), test-code fallbacks that never ran made loud, and a written proof for each row proved unreachable; production lines untouched, crates/runner/src/lib.rs untouched.",
      "evidence": "Route B (pinned nightly rustc 1.99.0-nightly 1a98b1e13 2026-08-07, LLVM 23.1.0) on Linux x86_64: START 4f49dbf43e79 lines 46145/47136 branches 3349/4218; code tip 2b998e4b5f6e lines 50775/51110 branches 3711/4170 (335 lines and 459 branch sides uncovered); test runner rc=0, tests passed 956 failed 0 ignored 0."
    },
    {
      "id": "LS2-LISTED",
      "status": "operator",
      "how": "Of the 540 uncovered rows that are not (F): 224 are (U) with a proof from the code, 36 need an allocator refusal or an address-space-sized input, 5 need a blake3 collision, 24 are believed unreachable with no proof claimed. Decision draft D-8563 asks the operator to rule on these.",
      "evidence": "reports/base-cloud-20261103/gaps-LS2.txt.md (every row tagged) and fu-LS2.txt.md ((F) rows and (U) proofs)"
    },
    {
      "id": "LS2-OPEN",
      "status": "open",
      "how": "251 uncovered rows remain open: 68 in production code and 183 in test code or read by hand. They are reachable or not yet classified; no test drives them in this session.",
      "evidence": "reports/base-cloud-20261103/gaps-LS2.txt.md, rows tagged [open]"
    }
  ],
  "files_changed": [
    "crates/runner/src/admission.rs",
    "crates/runner/src/audit.rs",
    "crates/runner/src/bootstrap.rs",
    "crates/runner/src/bootstrap_zero_v2_tests.rs",
    "crates/runner/src/excursion.rs",
    "crates/runner/src/exit_grid_policy.rs",
    "crates/runner/src/expression.rs",
    "crates/runner/src/expression_execution_tests.rs",
    "crates/runner/src/expression_oos_tests.rs",
    "crates/runner/src/expression_validation_tests.rs",
    "crates/runner/src/grid.rs",
    "crates/runner/src/grid_cover_tests.rs",
    "crates/runner/src/outcome.rs",
    "crates/runner/src/pbo.rs",
    "crates/runner/src/rank.rs",
    "crates/runner/src/report.rs",
    "crates/runner/src/report_cover_tests.rs",
    "crates/runner/src/resample.rs",
    "crates/runner/src/research_admission_projection.rs",
    "crates/runner/src/research_exit_grid_tests.rs",
    "crates/runner/src/signal_candle_stop_tests.rs",
    "crates/runner/src/significance_cover_tests.rs",
    "crates/runner/src/topn.rs",
    "crates/runner/src/trade.rs",
    "crates/runner/src/validate.rs",
    "crates/runner/src/validate_cover_tests.rs",
    "crates/runner/tests/auto_probe_reporter_refusal.rs",
    "crates/runner/tests/checkpointed_rank_refusals.rs",
    "crates/runner/tests/expression_pricing.rs",
    "crates/runner/tests/foreign_slice_deadline_readiness.rs",
    "crates/runner/tests/population_door_halt.rs",
    "crates/runner/tests/research_family_readiness.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS2.md",
    "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS2.md",
    "reports/base-cloud-20261103/fix-BASE-COV-LIBS-LS2.json.md",
    "reports/base-cloud-20261103/gaps-LS2.txt.md",
    "reports/base-cloud-20261103/fu-LS2.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-e81f0aa6f411-set.txt.md",
    "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS2-2b998e4b5f6e-set.txt.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check: exit 0 at 2b998e4b5f6e (stable rustc 1.97.1)",
    "clippy": "cargo clippy -p runner --all-targets --locked -- -D warnings: exit 0 at 2b998e4b5f6e",
    "test": "cargo test -p runner --locked --no-fail-fast: exit 0 in 5 min, 18 result lines, 956 passed, 0 failed, 0 ignored (lib 896 passed in 125.40s)",
    "forbidden_language_scan": "cargo test -p vocab --locked --test no_py_language: 9 passed, over the tracked tree with these reports staged",
    "route_b": "d3-route-b-cloud.sh at 2b998e4b5f6e: build rc 0, 17 test objects, test runner rc=0 (89 min), 73 of 74 set files read; NOT PROVED only crates/runner/benches/ratio.rs (no test object, operator item L-BCL-1); exit 3",
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
      "commit": "e81f0aa6f411",
      "lines": "49901/50358",
      "branches": "3709/4232"
    },
    "after": {
      "commit": "2b998e4b5f6e",
      "lines": "50775/51110",
      "branches": "3711/4170"
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
    "STATUS: IN PROGRESS, because open rows remain; the bar (100% lines and branches by Route B) is not met for runner.",
    "NEEDS THE MAC: Route B on its pin, macOS rows, non-root 14-core tests, ignored real-data tests (L-BCL-3: runner has none), the D1 landing, and the deferred mutants pass over the set file.",
    "Decision drafts in the report: D-8562 (the coverage figures and lists) and D-8563 (allocator, hash-collision and unproven rows, for the operator).",
    "crates/runner/src/lib.rs (LSA) is untouched: its paths are tested from crates/runner/tests/; red checks edited it locally and restored it byte for byte, never committed.",
    "The extreme-case count per file (at least 8, 2 combined, 1 out of the box) was not audited in this session.",
    "Process slips are recorded in the report: a commit pushed before clippy passed (fixed by the next commit), three commit messages with a wrong figure or count (corrected in the report), a wrong (U) claim (corrected, test added), and two transient shell commands that named or ran the forbidden language (no file, nothing tracked)."
  ],
  "held": false
}
```

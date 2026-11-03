# fix-BASE-COV-LIBS-LS3 (FIX shape)

```json
{
 "name": "BASE-COV-LIBS-LS3",
 "branch": "cloud/cov-libs-ls3-20261103",
 "head": "dc3632704fdbc9a33f40145d2e6b23a8fbe719d0",
 "committed": true,
 "findings": [
  {
   "id": "BCL-LS3-lake",
   "status": "fixed",
   "how": "reader.rs, page.rs and schema.rs refusals reached by tests that drive their input (negative row count, chunk past the file end and at it, null spot, the six wrong-type reads, a position past the chunk, non-timestamp annotations, spec at run time); the real-lake scan, walk and census proved on synthetic trees; two production restructures (D-8564 draft). Left: (F) failing sides, one (U) with its proof, and the #[ignore]d real-lake tests, which are the Mac's (L-BCL-3).",
   "evidence": "cargo test -p lake --locked --no-fail-fast EXIT 0; clippy -D warnings EXIT 0; 18 of 18 red checks RED; Route B figures in coverage; report-BASE-COV-LIBS-LS3.md step 2"
  },
  {
   "id": "BCL-LS3-indicators",
   "status": "fixed",
   "how": "every production guard reached by a test or restructured (D-8565 draft): exact-minute bridge, crossing walk, VWAP refusal mapping; white-box states for not-live sessions, overflowing windows and legs, unseeded and unwarmed evaluators; zero-period averages; a close breaking both swings checked against the oracle; dead oracle clauses in the readiness tests asserted. Left: (F) failing sides and one (U) with its proof.",
   "evidence": "cargo test -p indicators --locked --no-fail-fast EXIT 0; clippy EXIT 0; 24 of 24 red checks RED; Route B figures in coverage; report step 3"
  },
  {
   "id": "BCL-LS3-vocab",
   "status": "fixed",
   "how": "production guards restructured (D-8565 draft) and the renderer bound, pivot pair rules, probe walk and prefix references proved (step 4); the workspace gates' own readers tested at every edge no planted file took, the name gate's no-git paths reached through a named git program, the lock walk filed beside its seen flags, the search paused, the anti-monotonicity trials made to hit (step 5). Left: (F) failing sides and one (U) with its proof.",
   "evidence": "cargo test -p vocab --locked --no-fail-fast EXIT 0; clippy EXIT 0; 8 of 8 red checks RED (step 4) and 36 of 40 RED (step 5), the 4 green each equivalent on every input, named in the report; Route B figures in coverage"
  }
 ],
 "files_changed": [
  "crates/indicators/src/anchored.rs",
  "crates/indicators/src/column.rs",
  "crates/indicators/src/column_cover_tests.rs",
  "crates/indicators/src/evaluator.rs",
  "crates/indicators/src/evaluator_cover_tests.rs",
  "crates/indicators/src/gap.rs",
  "crates/indicators/src/gap_cover_tests.rs",
  "crates/indicators/src/lib.rs",
  "crates/indicators/src/orb.rs",
  "crates/indicators/src/orb_cover_tests.rs",
  "crates/indicators/src/session.rs",
  "crates/indicators/src/session_cover_tests.rs",
  "crates/indicators/src/trend.rs",
  "crates/indicators/src/trend_cover_tests.rs",
  "crates/indicators/src/vwap.rs",
  "crates/indicators/src/vwap_cover_tests.rs",
  "crates/indicators/tests/crossing_known_readiness.rs",
  "crates/indicators/tests/current_day_fib_known_readiness.rs",
  "crates/indicators/tests/exact_minute_orb_gap_readiness.rs",
  "crates/indicators/tests/fib_known_readiness.rs",
  "crates/indicators/tests/fib_rung_rounding.rs",
  "crates/indicators/tests/gap_known_readiness.rs",
  "crates/indicators/tests/module_doc_counts.rs",
  "crates/indicators/tests/orb_known_readiness.rs",
  "crates/indicators/tests/session_known_readiness.rs",
  "crates/indicators/tests/trend_known_readiness.rs",
  "crates/indicators/tests/withheld_fold.rs",
  "crates/lake/src/page.rs",
  "crates/lake/src/reader.rs",
  "crates/lake/src/schema.rs",
  "crates/lake/src/schema_cover_tests.rs",
  "crates/lake/tests/refusals.rs",
  "crates/lake/tests/synthetic.rs",
  "crates/vocab/src/expression.rs",
  "crates/vocab/src/expression_search.rs",
  "crates/vocab/src/implication.rs",
  "crates/vocab/src/table.rs",
  "crates/vocab/tests/expression_search.rs",
  "crates/vocab/tests/mask.rs",
  "crates/vocab/tests/no_float.rs",
  "crates/vocab/tests/no_py_language.rs",
  "crates/vocab/tests/no_runtime_spawn.rs",
  "crates/vocab/tests/workspace_is_rust.rs"
 ],
 "files_outside_ownership": [],
 "gates": {
  "fmt": "pass (cargo fmt --all --check EXIT 0, at the head)",
  "clippy": "pass (cargo clippy -p lake -p indicators -p vocab --all-targets --locked -- -D warnings EXIT 0, at the head)",
  "tests": "pass (cargo test -p lake -p indicators -p vocab --locked --no-fail-fast EXIT 0 at the head: 868 passed, 0 failed, 6 ignored, the real-lake tests, L-BCL-3; no host red)",
  "web_build": "not applicable",
  "web_test": "not applicable"
 },
 "p50_p99": "no bench run: no hot path changed (each production change is one lookup or pattern per step, O(1) as before); benches/ratio.rs of the three crates are the Mac's (gate 8, L-BCL-1)",
 "coverage": "Route B (pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07) as nightly-2026-08-08; -C instrument-coverage -Z coverage-options=branch, opt-level 0, llvm-profdata merge -sparse, llvm-cov with branches; the kit's d3-route-b-cloud.sh) on the head dc3632704fdb, set = every .rs file of lake, indicators and vocab whole (79 files, SET OVERRIDE): indicators lines 16909/16914, branches 1892/2078; lake lines 5678/6135, branches 305/364; vocab lines 8185/8205, branches 1240/1298 (at START 29121/29904 lines and 3315/3838 branch sides for the three). Every one of the 785 uncovered rows is classified: GAP 0, (F) 428, (U) 4 with their proofs, NEEDS THE MAC 353 (the #[ignore]d real-lake tests, L-BCL-3). The route printed NOT PROVED for four files only: the three benches/ratio.rs ([[bench]] test = false, cargo test builds none: L-BCL-1, the Mac's) and crates/lake/src/lib.rs (nine mod lines and no function: no object holds a line of it). Lists: reports/base-cloud-20261103/gaps-LS3.txt.md and fu-LS3.txt.md.",
 "mutants": "deferred (operator 30 Oct)",
 "store_writes_needed": [],
 "notes": "Route B on this host with the pinned nightly; the Mac re-measures. gaps-LS3.txt.md and fu-LS3.txt.md hold every uncovered row with its class. Decision drafts D-8564 (lake) and D-8565 (indicators, vocab) are in the report for LSA. NEEDS THE MAC: Route B on its pin, the #[ignore]d real-lake tests (L-BCL-3), benches (L-BCL-1), macOS rows, the D1 landing. files_changed lists the 43 code files between START and head, all in the shard; the session files under reports/base-cloud-20261103 (OWNER-BASE-COV-LIBS-LS3.md, report-BASE-COV-LIBS-LS3.md, fix-BASE-COV-LIBS-LS3.json.md, gaps-LS3.txt.md, fu-LS3.txt.md) are the ones CLOUD.md items 5 and 10 give every session. Red checks: lake 18 of 18, indicators 24 of 24, vocab 8 of 8 and 37 of 41 RED, the 4 green each equivalent on every input (report step 5).",
 "held": false
}
```

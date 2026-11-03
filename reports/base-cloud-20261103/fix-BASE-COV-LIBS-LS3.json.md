# fix-BASE-COV-LIBS-LS3 (FIX shape)

```json
{
 "name": "BASE-COV-LIBS-LS3",
 "branch": "cloud/cov-libs-ls3-20261103",
 "head": "8275882edcf1932a4f2a9cf43136036442453be4",
 "committed": true,
 "findings": [
  {
   "id": "BCL-LS3-lake",
   "status": "fixed",
   "how": "reader.rs, page.rs and schema.rs refusals reached by tests that drive their input (negative row count, chunk past the file end and at it, null spot, the six wrong-type reads, a position past the chunk, non-timestamp annotations, spec at run time); the real-lake scan, walk and census proved on synthetic trees; two production restructures (D-8564 draft).",
   "evidence": "cargo test -p lake --locked --no-fail-fast EXIT 0; clippy -D warnings EXIT 0; 18 of 18 red checks RED; report-BASE-COV-LIBS-LS3.md step 2"
  },
  {
   "id": "BCL-LS3-indicators",
   "status": "fixed",
   "how": "every production guard reached by a test or restructured (D-8565 draft): exact-minute bridge, crossing walk, VWAP refusal mapping; white-box states for not-live sessions, overflowing windows and legs, unseeded and unwarmed evaluators; zero-period averages; a close breaking both swings checked against the oracle; dead oracle clauses in the readiness tests asserted.",
   "evidence": "cargo test -p indicators --locked --no-fail-fast EXIT 0; clippy EXIT 0; 24 of 24 red checks RED; Route B indicators lines 16909/16914, the 5 left (F); one (U)"
  },
  {
   "id": "BCL-LS3-vocab",
   "status": "partly fixed",
   "how": "production guards restructured (D-8565 draft) and the renderer bound, pivot pair rules, probe walk and prefix references proved; the test-code gaps of the workspace scanners (tests/no_runtime_spawn.rs, tests/no_py_language.rs, tests/workspace_is_rust.rs) are the remaining GAP rows, listed in gaps-LS3.txt.md.",
   "evidence": "cargo test -p vocab --locked --no-fail-fast EXIT 0; clippy EXIT 0; 8 of 8 red checks RED"
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
  "crates/vocab/tests/no_float.rs"
 ],
 "files_outside_ownership": [],
 "gates": {
  "fmt": "pass (cargo fmt --all --check EXIT 0)",
  "clippy": "pass (cargo clippy -p lake -p indicators -p vocab --all-targets --locked -- -D warnings, each EXIT 0)",
  "tests": "pass (cargo test -p lake -p indicators -p vocab --locked --no-fail-fast EXIT 0; 6 ignored real-lake tests, L-BCL-3)",
  "web_build": "not applicable",
  "web_test": "not applicable"
 },
 "p50_p99": "no bench run: no hot path changed (each production change is one lookup or pattern per step, O(1) as before); benches/ratio.rs of the three crates are the Mac's (gate 8, L-BCL-1)",
 "coverage": "Route B at 68d588fc34f6 (interim; code since then is test-only for vocab): indicators lines 16909/16914 branches 1892/2078, lake 5678/6135 and 305/364, vocab 7725/7819 and 1131/1280; uncovered rows 950: (F) 415, (U) 4, NEEDS THE MAC 353, GAP 178 (vocab workspace scanners); benches and lake/src/lib.rs NOT READ by the route",
 "mutants": "deferred (operator 30 Oct)",
 "store_writes_needed": [],
 "notes": "Route B on this host with the pinned nightly; the Mac re-measures. gaps-LS3.txt.md and fu-LS3.txt.md hold every uncovered row with its class. Decision drafts D-8564 (lake) and D-8565 (indicators, vocab) are in the report for LSA. NEEDS THE MAC: Route B on its pin, the #[ignore]d real-lake tests (L-BCL-3), benches (L-BCL-1), macOS rows, the D1 landing. Interim: the vocab scanner gaps are being closed.",
 "held": false
}
```

# fix-BASE-COV-LIBS-LS4.json.md

The FIX-shape report of session BASE-COV-LIBS-LS4. Its one fenced block is the json object; `head` is the code tip, `git rev-list -1 HEAD -- . ':(exclude)reports'`.

```json
{
 "name": "BASE-COV-LIBS-LS4",
 "branch": "cloud/cov-libs-ls4-20261103",
 "head": "bcbc6524bdb29351f5d211aed11e4f7b76795b7b",
 "committed": true,
 "held": false,
 "status": "BLOCKED OUTSIDE: every row left is LSA's (engine lib.rs), the operator's (L-BCL-3, global_absent by design) or the Mac's (its pin)",
 "reading": "whole:core,costs,engine,greeks,telemetry (SET OVERRIDE; shard LS4 of D6)",
 "findings": [
  {
   "id": "LS4-01",
   "status": "blocked_outside",
   "how": "Attempt 4's indented #[cfg(test)] arm in engine reserved (D-7329, lib.rs:208) cut two source-shape tests' shipping region at line 208; patch 1 splits on the first #[cfg(test)] line at the margin, as a sibling test already does (draft D-8566)",
   "evidence": "red as root and as nobody: tests::the_sweep_counts_support_against_the_fixed_width_column (lib.rs:4719 left 0 right 2), tests::the_walk_has_exactly_three_early_exits_and_each_is_a_budget (lib.rs:4445); with patch 1 (sha256 c4cbc10e...) cargo test -p engine --locked green, clippy -D warnings clean, rustfmt clean; patch text in report-BASE-COV-LIBS-LS4.md"
  },
  {
   "id": "LS4-02",
   "status": "fixed",
   "how": "engine/src/refusal_cover_tests.rs: the refused-reservation counter equalled the loop index (clippy explicit_counter_loop under -D warnings)",
   "evidence": "commit d76b11b7; cargo clippy -p engine --all-targets --locked -- -D warnings RC 0"
  },
  {
   "id": "LS4-03",
   "status": "fixed",
   "how": "telemetry/src/sink_recovery_tests.rs: Duration::from_secs(60) -> from_mins(1), assert!(a != b) -> assert_ne!; the FIFO guard and the code-0 any() made exact",
   "evidence": "commit 5a4eab93; clippy -p telemetry -D warnings RC 0; red under a wrong ENXIO"
  },
  {
   "id": "LS4-04",
   "status": "fixed",
   "how": "engine/tests/adversarial.rs one(nth) used vocab set_exact(..).unwrap_or(m): position 7 (a near_ bit) gave the empty mask, so the shape \"eight bars, one bit each, all different\" held a bar with no bit; one() now sets the bit by position",
   "evidence": "commit e0790fc2; every_one_bit_fixture_holds_one_live_bit (every live position) red under the old fallback"
  },
  {
   "id": "LS4-05",
   "status": "fixed",
   "how": "Two checks ran on nothing: findings.rs every_refuted_finding_carries_its_reason (no REFUTED row) and lint.rs no_float_in_price's per-line float check (no float in OTHERS); each is now a function of its input, also checked on input that breaks it",
   "evidence": "commits 9cb09048, 7bdf142d; red under a five-byte reason accepted, f32 unseen, a four-line window"
  },
  {
   "id": "LS4-06",
   "status": "fixed",
   "how": "Production rows reached by their inputs: core alias by_fno/by_store_key/from_old/to_new own-field checks (a skewed table built in a child module), costs stack.rs:589 (100% STT card built in a child module of schedule), engine lib.rs:191 (JoinProbe refused pair)",
   "evidence": "alias::cover_tests (red under each check removed), schedule::cover_tests (red under a saturating sum), tests/join_probe_screen.rs (red under the verdict inverted); Route B bcbc6524bdb2 shows them covered"
  },
  {
   "id": "LS4-07",
   "status": "fixed",
   "how": "Test-code GAP rows closed in core (cost_invariants, decision_citations, findings, graph, lint), costs (dhan_calculator const fn and refusal cell, fa73061 and lot_table timing helpers), engine (keep heap oracle, column reference, production_callers, resume_readiness), telemetry (sink reaped, sink_recovery_tests): scratch trees, child processes, synthetic text; silent skips refuse by name",
   "evidence": "27 new tests named in the report, each red under a hand change (commit messages); Route B 294/193 -> 190/145 uncovered lines/sides"
  },
  {
   "id": "LS4-08",
   "status": "proved_u",
   "how": "costs stack.rs:720 (Exact::sum of the calculator total cannot overflow: page_lines panics on a brokerage past i128::MAX/18 first) and stack.rs:907 (to_rupee narrowing never fails for an i64)",
   "evidence": "proofs in fu-LS4.txt.md; rounding_to_the_rupee_holds_at_both_ends_of_the_type"
  },
  {
   "id": "LS4-09",
   "status": "host_red",
   "how": "Four telemetry sink tests chmod a directory and expect a refusal root bypasses (the base's telemetry 4)",
   "evidence": "each fails as root and passes as nobody: setpriv --reuid=65534 --regid=65534 --clear-groups <binary> --exact <test>; the whole nobody run passes them"
  },
  {
   "id": "LS4-10",
   "status": "blocked_outside",
   "how": "engine lib.rs test code (LSA's file): windows(2) let-else and an expect after assert!(is_ok), an oracle subset check the fixture never fails, a handed-batch filter that never filtered; patch 2 (on top of patch 1) clears all 6 rows",
   "evidence": "Route B of engine with both patches applied: the 6 rows gone, lib.rs lines 2240/2243 branches 126/148; nobody run 1119 passed 0 failed 3 ignored; patch text in the report (sha256 1b62c7b0...)"
  },
  {
   "id": "LS4-11",
   "status": "blocked_outside",
   "how": "Judgment rows in engine lib.rs test code: 3014 (distinctness asserted over an always-empty level) and 5906-5907 (an observer rejection arm that runs only on a production defect)",
   "evidence": "gaps-LS4.txt.md, with suggested changes"
  },
  {
   "id": "LS4-12",
   "status": "open_operator",
   "how": "telemetry/tests/global_absent.rs:106-107: emit_if! arguments the test asserts never run; uncoverable by design",
   "evidence": "gaps-LS4.txt.md"
  },
  {
   "id": "LS4-13",
   "status": "open_operator",
   "how": "L-BCL-3: 75 lines inside three #[ignore]d tests (costs fa73061 and lot table timing, telemetry 2 GiB audit run); their helpers are now checked where the gate runs",
   "evidence": "gaps-LS4.txt.md"
  },
  {
   "id": "LS4-14",
   "status": "blocked_outside",
   "how": "Gate 20 counts and docs/06 section 54 are LSA's from CI's cargo llvm-cov workspace instrument; Route B crate-scoped telemetry uncovered lines: sink.rs 19 (5 in the ignored test), lib.rs 0, tail.rs 0, record.rs 1, level.rs 1",
   "evidence": "gaps-LS4.txt.md PER FILE"
  }
 ],
 "files_changed": [
  "crates/core/src/alias.rs",
  "crates/core/src/alias_cover_tests.rs",
  "crates/core/tests/cost_invariants.rs",
  "crates/core/tests/decision_citations.rs",
  "crates/core/tests/findings.rs",
  "crates/core/tests/graph.rs",
  "crates/core/tests/lint.rs",
  "crates/costs/src/lot_table_tests.rs",
  "crates/costs/src/schedule.rs",
  "crates/costs/src/schedule_cover_tests.rs",
  "crates/costs/src/stack.rs",
  "crates/costs/src/stack_cover_tests.rs",
  "crates/costs/tests/dhan_calculator.rs",
  "crates/costs/tests/fa73061.rs",
  "crates/engine/src/column.rs",
  "crates/engine/src/keep.rs",
  "crates/engine/src/refusal_cover_tests.rs",
  "crates/engine/tests/adversarial.rs",
  "crates/engine/tests/join_probe_screen.rs",
  "crates/engine/tests/production_callers.rs",
  "crates/engine/tests/resume_readiness.rs",
  "crates/telemetry/src/sink.rs",
  "crates/telemetry/src/sink_recovery_tests.rs",
  "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS4.md",
  "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md",
  "reports/base-cloud-20261103/fu-LS4.txt.md",
  "reports/base-cloud-20261103/gaps-LS4.txt.md",
  "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS4.md"
 ],
 "files_outside_ownership": [],
 "gates": {
  "fmt": "pass (cargo fmt --all --check RC 0 at the code tip)",
  "clippy": "pass (cargo clippy -p core -p costs -p engine -p greeks -p telemetry --all-targets --locked -- -D warnings RC 0)",
  "tests_root": "fail 6: passed 1113 failed 6 ignored 3 (2 engine lib.rs reds for LSA patch 1; 4 proved root-only host reds)",
  "tests_nobody": "fail 2: passed 1117 failed 2 ignored 3 (the 2 engine lib.rs reds)",
  "tests_with_lsa_patches": "pass: nobody run passed 1119 failed 0 ignored 3; clippy -p engine -D warnings RC 0",
  "web_build": "not applicable",
  "web_test": "not applicable"
 },
 "p50_p99": [
  "none owed: no production code changed (every hunk in a production file is inside #[cfg(test)]); no bench run",
  "D3 coverage core,costs,engine,greeks,telemetry route B: 0.9 min (real 0m51.847s; build 11 s, tests about 40 s), queue 0 min"
 ],
 "coverage": {
  "route": "B: rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0-rust-1.99.0-nightly, x86_64-unknown-linux-gnu, test binaries as uid 65534 (builder figure; the Mac re-measures)",
  "before_d76b11b79ae9": "core 56/73, costs 171/22, engine 35/50, greeks 3/3, telemetry 29/45; all 294 lines / 193 sides uncovered",
  "after_bcbc6524bdb2": "core 28/38, costs 102/22, engine 30/40, greeks 3/3, telemetry 27/42; all 190 lines / 145 sides uncovered; lines 36050/36240, branches 2341/2486, 81 files read",
  "not_proved": "9 files: 6 benches/ (L-BCL-1), core/costs/greeks src/lib.rs with no executable line (L-BCL-2)",
  "fu_file": "reports/base-cloud-20261103/fu-LS4.txt.md (227 (F), 2 (U))",
  "gaps_file": "reports/base-cloud-20261103/gaps-LS4.txt.md (106 GAP rows: 75 L-BCL-3, 25 cleared by LSA patches, 4 LSA judgment, 2 by design)",
  "set_file": "reports/base-cloud-20261103/deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md (90 coverage files whole, 171 mutants lines)",
  "bar_met": false
 },
 "mutants": "deferred (operator 30 Oct)",
 "store_writes_needed": [],
 "decision_drafts": [
  "D-8566 (engine lib.rs shipping-region split; patch 2 if taken)",
  "D-8567 (LS4 figures, (F)/(U) list, test-code restructures, the adversarial fixture defect; D-7250 narrow reading does not end here)"
 ],
 "notes": [
  "The brief's -p brutex-core names no package: the package is core (lib target brutex_core); -p core used.",
  "Host: root, 4 cores, 15 GB, 28 GB free disk at the end; TMPDIR /dev/shm/ls4-tmp and the target tmp sticky world-writable, GIT_CONFIG_GLOBAL naming this checkout safe.directory, so the nobody runs read git.",
  "NEEDS THE MAC: Route B on its pin, macOS rows (none in this shard), non-root 14-core run, L-BCL-3, the D1 landing, the d3-mutants-deferred.txt line.",
  "Mutants deferred (operator 30 Oct); the set file carries the D2 mutants lines of f01d7a0e4...bcbc6524bdb2 in these crates."
 ]
}
```

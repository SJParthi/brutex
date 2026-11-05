# result-PSRB-BOLL (token PSRB-BOLL-20261103-1)

```json
{
  "name": "PSRB-BOLL",
  "branch": "cloud/psrb-boll-20261103",
  "head": "19f410d2bd2c851c77aaed9974d647bb5df93487",
  "committed": true,
  "tests": {
    "new": 23,
    "names": "every test PARTS.md 4.5 names, in crates/indicators/src/bollinger_tests.rs (fixtures BOLL-1 and BOLL-2 exactly)",
    "indicators": "cargo test -p indicators --locked: 22 binaries, 527 passed, 0 failed, 0 ignored (START 504)",
    "core": "cargo test -p core --locked --test cost_invariants --test decision_citations: 5 and 4 passed",
    "shown_red": "23 of 23, under 27 hand changes of bollinger.rs, each restored byte for byte"
  },
  "gates": {
    "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "fmt": "pass: cargo fmt --all --check",
    "clippy": "pass: cargo clippy -p indicators --all-targets --locked -- -D warnings",
    "test_indicators": "pass",
    "test_core": "pass",
    "host_reds": [],
    "also_green": "cargo test -p core --locked (261 passed); vocab no_py_language, workspace_is_rust, stale_claims, no_float (9, 18, 10, 3 passed)",
    "ci_gates_run_here": "gates 14 and 27 green; gates 10, 11, 12 and 27b red on START and on the head with the same refusals, none naming a PSRB-BOLL file, row or id"
  },
  "coverage": {
    "route": "B",
    "rustc": "rustc 1.99.0-nightly (1a98b1e13 2026-08-07)",
    "llvm": "LLVM version 23.1.0-rust-1.99.0-nightly",
    "reading": "narrow",
    "measured_at": "19f410d2bd2c851c77aaed9974d647bb5df93487",
    "files": {
      "crates/indicators/src/bollinger.rs": "lines 229/229, branches 40/40",
      "crates/indicators/src/bollinger_tests.rs": "lines 700/700, branches 18/18",
      "crates/indicators/benches/ratio/psrb_boll.rs": "lines 57/60, branches 0/0",
      "crates/indicators/src/lib.rs line 64": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs lines 71, 72": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs main 758-821": "lines 24/26, branches 1/2"
    },
    "f_items": [
      "crates/indicators/benches/ratio/psrb_boll.rs:89: Err(why) => {",
      "crates/indicators/benches/ratio/psrb_boll.rs:90: println!(\"{id} {size}: the step was refused ({why:?})   FAILED\");",
      "crates/indicators/benches/ratio/psrb_boll.rs:91: return false;",
      "crates/indicators/benches/ratio.rs:816:8 branch True: if verdicts.contains(&false) {",
      "crates/indicators/benches/ratio.rs:817: println!(\"A RATIO BREACHED ITS CEILING — see the lines marked BREACH.\");",
      "crates/indicators/benches/ratio.rs:818: std::process::exit(1);"
    ],
    "gaps": 0,
    "u_items": 0,
    "not_proved": 0,
    "bench_half": "the bench target built in the bench profile under the same RUSTFLAGS and target dir, run once (rc 0, every row ok), its profile merged on its own and read for the two bench files by d3-covset.mjs"
  },
  "benches": {
    "advisory": true,
    "machine": "Linux 6.18 x86_64, 4 vCPU Intel Xeon @ 2.80GHz, rustc 1.97.1",
    "command": "cargo bench -p indicators --bench ratio --locked",
    "runs": 1,
    "C-PSRB-BOLL-01": "best-round p99 after 2,000, 20,000, 200,000 candles at n 20: 46, 46, 46 ns; 1.000x; budget 2,000 ns; ok",
    "C-PSRB-BOLL-02": "best-round p99 at n 20, 200, 2,000 after 20,000 candles: 45, 44, 39 ns; no size above the first (1.000x at most); budget 2,000 ns; ok",
    "every_row": "all ratios within the ceiling, exit 0"
  },
  "decisions": ["D-8590", "D-8591", "D-8592", "D-8593", "D-8594", "D-8595"],
  "needs_mac": [
    "check 1 on the Mac's pin: Route B on the D2 set against START, the bench wrapper and main by the instrumented bench run",
    "gate 8: cargo bench -p indicators --bench ratio with both C-PSRB-BOLL rows",
    "gates 10, 11, 12 and 14; gate 14's cover-table row for C-PSRB-BOLL-01 and -02 is the main session's at the family assembly",
    "macOS rows: none expected (no platform code)",
    "the anchors on the then-current feat/db-fix",
    "mutants: deferred (operator, 30 Oct); the set file in the report carries the mutants lines"
  ],
  "held": false
}
```

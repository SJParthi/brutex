# result-PSRB-RSI (token PSRB-RSI-20261103-1)

```json
{
  "name": "PSRB-RSI",
  "branch": "cloud/psrb-rsi-20261103",
  "head": "947dbd3ad97854784ce3e895318b478a8a51354b",
  "committed": true,
  "tests": {
    "new": 27,
    "names": "every test PARTS.md 3.5 names, in crates/indicators/src/rsi_tests.rs",
    "indicators": "cargo test -p indicators --locked: 22 binaries, 531 passed, 0 failed, 0 ignored (START 504)",
    "core": "cargo test -p core --locked --test cost_invariants --test decision_citations: 5 and 4 passed",
    "shown_red": "27 of 27, under 24 hand changes of rsi.rs, the file restored byte for byte"
  },
  "gates": {
    "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "fmt": "pass: cargo fmt --all --check",
    "clippy": "pass: cargo clippy -p indicators --all-targets --locked -- -D warnings",
    "test_indicators": "pass",
    "test_core": "pass",
    "host_reds": [],
    "ci_gates_run_here": "gate 27 green; gates 10, 11, 12 and 27b red on START and on the head with the same refusals, none naming a PSRB-RSI file or id"
  },
  "coverage": {
    "route": "B",
    "rustc": "rustc 1.99.0-nightly (1a98b1e13 2026-08-07)",
    "llvm": "LLVM version 23.1.0-rust-1.99.0-nightly",
    "reading": "narrow",
    "measured_at": "947dbd3ad97854784ce3e895318b478a8a51354b",
    "files": {
      "crates/indicators/src/rsi.rs": "lines 153/153, branches 22/22",
      "crates/indicators/src/rsi_tests.rs": "lines 653/653, branches 26/26",
      "crates/indicators/benches/ratio/psrb_rsi.rs": "lines 70/70, branches 0/0",
      "crates/indicators/src/lib.rs line 71": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs lines 69, 70": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs main 758-821": "lines 24/26, branches 1/2"
    },
    "f_items": [
      "crates/indicators/benches/ratio.rs:816:8 branch True: if verdicts.contains(&false) {",
      "crates/indicators/benches/ratio.rs:817: println!(\"A RATIO BREACHED ITS CEILING — see the lines marked BREACH.\");",
      "crates/indicators/benches/ratio.rs:818: std::process::exit(1);"
    ],
    "gaps": 0,
    "u_items": 0,
    "not_proved": 0,
    "bench_half": "the bench target built in the test profile under the same instrumentation, run once, its profile merged with the 21 test profiles"
  },
  "benches": {
    "advisory": true,
    "machine": "Linux x86_64 KVM guest, 4 vCPU Intel Xeon @ 2.30GHz, rustc 1.97.1",
    "command": "cargo bench -p indicators --bench ratio --locked",
    "runs": 2,
    "C-PSRB-RSI-01": "best-round p99 after 2,000, 20,000, 200,000 candles: 40, 40, 40 ns (run 1) and 40, 40, 40 ns (run 2); 1.000x; budget 2,000 ns; ok",
    "C-PSRB-RSI-02": "best-round p99 at periods 14, 140, 1,400: 40, 41, 42 ns (run 1, 1.050x) and 40, 40, 40 ns (run 2, 1.000x); budget 2,000 ns; ok",
    "every_row": "all ratios within the ceiling, exit 0, both runs"
  },
  "decisions": ["D-8580", "D-8581", "D-8582", "D-8583", "D-8584"],
  "needs_mac": [
    "check 1 on the Mac's pin: Route B on the D2 set against START, the bench wrapper and main by the instrumented bench run",
    "gate 8: cargo bench -p indicators --bench ratio with both C-PSRB-RSI rows",
    "gates 10, 11, 12 and 14; gate 14's cover-table row for C-PSRB-RSI-01 and -02 is the main session's at the family assembly",
    "macOS rows: none expected (no platform code)",
    "the anchors on the then-current feat/db-fix",
    "mutants: deferred (operator, 30 Oct); the set file in the report carries the mutants lines"
  ],
  "held": false
}
```

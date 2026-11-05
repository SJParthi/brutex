# result-PSRB-CCI (token PSRB-CCI-20261105-1)

```json
{
  "name": "PSRB-CCI",
  "branch": "cloud/psrb-cci-20261105",
  "head": "bd65eb6f7d8eb2a1af9dc75b3cf3b380d1df70c1",
  "committed": true,
  "tests": {
    "new": 25,
    "names": "every test PARTS-WILLR-CCI.md 6.5 names, in crates/indicators/src/cci_tests.rs",
    "indicators": "cargo test -p indicators --locked: 22 binaries, 529 passed, 0 failed, 0 ignored (START 504)",
    "core": "cargo test -p core --locked --test cost_invariants --test decision_citations: 5 and 4 passed",
    "shown_red": "25 of 25, under 27 hand changes of cci.rs, the file restored byte for byte"
  },
  "gates": {
    "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "fmt": "pass: cargo fmt --all --check",
    "clippy": "pass: cargo clippy -p indicators --all-targets --locked -- -D warnings",
    "test_indicators": "pass",
    "test_core": "pass",
    "host_reds": [],
    "start_baseline": "cargo test -p indicators --locked on START: 22 binaries, 504 passed, 0 failed",
    "ci_gates_run_here": "gates 0, 10b, 14, 25 and 27 green; gates 10, 11, 12 and 27b red on START and on the head with the same refusals, none naming a PSRB-CCI file or id"
  },
  "coverage": {
    "route": "B",
    "rustc": "rustc 1.99.0-nightly (1a98b1e13 2026-08-07)",
    "llvm": "LLVM version 23.1.0-rust-1.99.0-nightly",
    "reading": "narrow",
    "measured_at": "bd65eb6f7d8eb2a1af9dc75b3cf3b380d1df70c1",
    "files": {
      "crates/indicators/src/cci.rs": "lines 184/184, branches 22/22",
      "crates/indicators/src/cci_tests.rs": "lines 766/766, branches 14/14",
      "crates/indicators/benches/ratio/psrb_cci.rs": "lines 86/86, branches 0/0",
      "crates/indicators/src/lib.rs line 80": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs lines 70, 71": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs main 764-833": "lines 24/26, branches 1/2"
    },
    "f_items": [
      "crates/indicators/benches/ratio.rs:828:8 branch True: if verdicts.contains(&false) {",
      "crates/indicators/benches/ratio.rs:829: println!(\"A RATIO BREACHED ITS CEILING — see the lines marked BREACH.\");",
      "crates/indicators/benches/ratio.rs:830: std::process::exit(1);"
    ],
    "gaps": 0,
    "u_items": 0,
    "not_proved": 0,
    "bench_half": "the bench target built in the test profile under the same instrumentation, run once, its profile merged with the 21 test profiles"
  },
  "benches": {
    "advisory": true,
    "machine": "Linux x86_64, 4 vCPU Intel Xeon @ 2.10GHz, rustc 1.97.1",
    "command": "cargo bench -p indicators --bench ratio --locked",
    "runs": 2,
    "C-PSRB-CCI-01": "best-round p99 after 2,000, 20,000, 200,000 candles at period 20: 196, 188, 205 ns (run 1, 1.045x) and 188, 185, 189 ns (run 2, 1.005x); budget 2,000 ns; ok",
    "C-PSRB-CCI-02": "fold alone, best-round p99 at periods 20, 200, 2,000: 37, 34, 38 ns (run 1, 1.027x) and 35, 37, 34 ns (run 2, 1.057x); budget 2,000 ns; ok",
    "reading_line": "unjudged, the step with the reading at periods 20, 200, 2,000: 188, 809, 7,225 ns (run 1) and 211, 789, 4,981 ns (run 2)",
    "every_row": "all ratios within the ceiling, exit 0, both runs"
  },
  "decisions": ["D-8650", "D-8651", "D-8652", "D-8653", "D-8654", "D-8655"],
  "needs_mac": [
    "check 1 on the Mac's pin: Route B on the D2 set against START = PSRB-SEAM-2, the bench wrapper and main by the instrumented bench run",
    "gate 8: cargo bench -p indicators --bench ratio with both C-PSRB-CCI rows and the unjudged reading line printed whole",
    "gates 10, 11, 12 and 14; gate 14's cover-table row for C-PSRB-CCI-01 and -02 is the main session's at the family assembly",
    "macOS rows: none expected (no platform code)",
    "XW/anchors2-check.sh re-run against the then-current first-wave tips",
    "the anchors on the then-current feat/db-fix",
    "mutants: deferred (operator, 30 Oct); the set file in the report carries the mutants lines, and the report names one equivalent mutant (the D > 0 guard of above and below, written as 6.3 states it)",
    "for the operator: D-8653, reading() O(period) accepted by default, reversible before PSRB-STRAT's first run"
  ],
  "held": false
}
```

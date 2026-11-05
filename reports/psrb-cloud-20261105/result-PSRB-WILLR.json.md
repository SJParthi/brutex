# result-PSRB-WILLR (token PSRB-WILLR-20261105-1)

```json
{
  "name": "PSRB-WILLR",
  "branch": "cloud/psrb-willr-20261105",
  "head": "a910ba9272a726c20ac7de2643ca4ad298834a14",
  "committed": true,
  "tests": {
    "new": 28,
    "names": "every test PARTS-WILLR-CCI.md 5.5 names, in crates/indicators/src/williams_r_tests.rs",
    "indicators": "cargo test -p indicators --locked: 532 passed, 0 failed, 0 ignored (START 504)",
    "core": "cargo test -p core --locked --test cost_invariants --test decision_citations: 5 and 4 passed; cargo test -p core --locked: 261 passed",
    "shown_red": "28 of 28, under 30 hand changes of williams_r.rs at the code tip, every change compiled, the file restored byte for byte"
  },
  "gates": {
    "toolchain": "rustc 1.97.1 (8bab26f4f 2026-07-14)",
    "fmt": "pass: cargo fmt --all --check",
    "clippy": "pass: cargo clippy -p indicators --all-targets --locked -- -D warnings",
    "test_indicators": "pass",
    "test_core": "pass",
    "host_reds": [],
    "ci_gates_run_here": "gates 10, 11 and 12 red on START and on the head with the same refusal lines (gate 10: 2 rows, gate 11: 29 entries, gate 12: 85 blocks), none naming a PSRB-WILLR file or id; this part adds one gate-12 claim block, which names crates/indicators/benches/ratio.rs and says UNVERIFIED"
  },
  "coverage": {
    "route": "B",
    "rustc": "rustc 1.99.0-nightly (1a98b1e13 2026-08-07)",
    "llvm": "LLVM version 23.1.0-rust-1.99.0-nightly",
    "reading": "narrow",
    "measured_at": "a910ba9272a726c20ac7de2643ca4ad298834a14",
    "files": {
      "crates/indicators/src/williams_r.rs": "lines 195/195, branches 38/38",
      "crates/indicators/src/williams_r_tests.rs": "lines 711/711, branches 6/6",
      "crates/indicators/benches/ratio/psrb_willr.rs": "lines 70/70, branches 0/0",
      "crates/indicators/src/lib.rs line 78": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs lines 68, 69": "lines 0/0, branches 0/0 (no region)",
      "crates/indicators/benches/ratio.rs main 764-836": "lines 27/29, branches 1/2"
    },
    "set": "lines 1003/1005, branches 45/46",
    "f_items": [
      "crates/indicators/benches/ratio.rs:831:8 branch True: if verdicts.contains(&false) {",
      "crates/indicators/benches/ratio.rs:832: println!(\"A RATIO BREACHED ITS CEILING — see the lines marked BREACH.\");",
      "crates/indicators/benches/ratio.rs:833: std::process::exit(1);"
    ],
    "gaps": 0,
    "u_items": 0,
    "not_proved": 0,
    "bench_half": "the bench target built in the test profile under the same instrumentation, run once (exit 0), its profile merged with the 21 test profiles"
  },
  "benches": {
    "advisory": true,
    "machine": "Linux x86_64 KVM guest, 4 vCPU Intel Xeon @ 2.80GHz, rustc 1.97.1",
    "command": "cargo bench -p indicators --bench ratio --locked",
    "runs": 3,
    "C-PSRB-WILLR-01": "best-round p99 after 2,000, 20,000, 200,000 candles: 96, 83, 81 ns (run 1, 0.843x); 79, 81, 190 ns (run 2, 2.405x); 80, 78, 101 ns (run 3, 1.262x); budget 2,000 ns; ok in all three",
    "C-PSRB-WILLR-02": "best-round p99 at periods 14, 140, 1,400: 81, 82, 83 ns (run 1, 1.024x); 117, 180, 180 ns (run 2, 1.538x); 93, 85, 108 ns (run 3, 1.161x); budget 2,000 ns; ok in all three",
    "every_row": "all ratios within the ceiling, exit 0, all three runs",
    "noise": "run 2 rose in every round of the affected sizes and run 3 read the machine floor at 1,104 ps against 589 ps: a noisy host; L-PSRB-WILLR-03 records all three runs"
  },
  "decisions": ["D-8640", "D-8641", "D-8642", "D-8643", "D-8644"],
  "needs_mac": [
    "check 1 on the Mac's pin: Route B on the D2 set against START, the bench wrapper and main by the instrumented bench run",
    "gate 8: cargo bench -p indicators --bench ratio with both C-PSRB-WILLR rows",
    "gates 10, 11, 12 and 14; gate 14's cover-table row for C-PSRB-WILLR-01 and -02 is the main session's at the family assembly",
    "macOS rows: none expected (no platform code)",
    "XW/anchors2-check.sh re-run against the then-current first-wave tips, and the anchors on the then-current feat/db-fix",
    "accept or send back two departures from the letter of PARTS-WILLR-CCI.md: no separate R > 0 conjunct in above and below (redundant: U = R = 0 there; D-8641), and the hashed bench series instead of a stride (D-8644)"
  ],
  "held": false
}
```

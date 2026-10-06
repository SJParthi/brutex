# RESULT — cli-b (branch `pr74/g18-cli-b`, head 27b7770, pushed)

Base: origin/final/all-fixes 969493e. It is still the newest, already an ancestor of the head, so no merge was needed.
Decisions: D-2020..D-2034. Invariants: G18-cli-b-01..27.

## Validation at 27b7770
- `cargo fmt --check`: clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0.
- cli lib tests as uid 65534 (`setpriv --reuid=65534 --regid=65534 --clear-groups <cli lib test bin> --test-threads=3`): **1972 passed, 0 failed, 1 ignored**.
- Static gates: every `jobs.language-purity.steps` run step, gate 1e skipped: all EXIT 0.
- Not run: `cargo test --workspace` (only cli touched), `cargo deny`, coverage. p50/p99/max of the restructures: **UNVERIFIED**. Each restructure keeps the same bound (same reads and comparisons per operation); none was benchmarked.

## Mutation evidence
cargo-mutants 26.2.0, targeted with `--re`, copy mode (never `--in-place`), nextest, `--timeout 300`.
- About 120 mutant runs in total: every listed survivor, every listed timeout, and each restructured line.
- **0 timeout.** Unviable, all expected (5):
  - `readonly_file::regular` -> `Ok(Default::default())`: File has no Default.
  - `ordered::turn` -> `Default::default()`: Turn has no Default.
  - Two let-chain `&&` -> `||` in observations: does not compile.
  - The earlier one in the first partial run.
- Misses and how they were closed:
  - (a) `stored.rs` `|| last` -> `&&` (which reads `A||B||(C&&D)`). Fixed in fd7e3a2 with a past-the-dated-close test; the rerun caught it.
  - (b) `expected_third < final_window.from` with `<` changed to `<=` and to `==`. Each built into the cli lib and run against the FULL suite as non-root: `le` and `eq` both 1972 passed, 0 failed, so the survivors were real. No measured session has a final window under 3 minutes (shortest 60), so the mutants were unreachable. Restructured to `!final_window.holds(expected_third)` (D-2034). The rerun of that block: **7/7 caught**.

| item | fixed? | commit | evidence |
|---|---|---|---|
| live.rs:229-231 clears_bar (6) | yes, test | abda922 | caught (live 229 true/false, 230 >,>=,==,<, 231 `&&`) |
| ordered.rs:96 `<` -> `<=` | yes, restructure + test | abda922 | caught |
| ordered.rs timeouts (ready false, `||` x2, `-` -> `+`/`/`, update -> (), Finished::drop -> (), delete `!` in turn) | yes, bounded tests | abda922, d85fa9d | all caught; 0 timeout |
| pool.rs:992 forced `>` -> `>=` | yes, restructure (equivalent) | abda922 | mutant no longer exists |
| pool.rs:1307 `>` -> `<` | yes, test | abda922 | caught |
| pool_oos.rs 457/500/514/551/653/682/715/797 (19) | yes, tests | 39a0d82 | all caught |
| population.rs 3375/3376/3390/3391 | yes, restructure + test | 4f7a6e0, 9f4914d | original mutants no longer exist; new `!=` in the population module |
| admission_v4 2730/2739x2/2740/2744 + timeout 2745 | yes, test + bounded range | 4f7a6e0, d85fa9d | caught; loop mutants caught |
| finalization_v4 2454/2463x2/2464/2468 + timeout 2469 | yes, test + bounded range | 4f7a6e0, d85fa9d | caught |
| finalization_v3 2455 | yes, test | 4f7a6e0 | caught |
| observations_v1 2178, 2410x3, 2725, 3678x3 | yes, tests + restructure | 4f7a6e0 | caught / unviable (let-chain) |
| statistics_v2 2234, 3701x2, 5549; statistics_v3 2888 | yes, tests | 4f7a6e0 | caught |
| selection_v6 318x2, 325, 405, 434, 436, 451 | yes, tests + restructure (451) | a6a50f0 | caught |
| selection_v6_read 223, 326x3, 329, 330, 477x3 | yes, tests | a6a50f0 | caught |
| readonly_file 83 | yes, restructure (cfg-dead body) | a6a50f0 | mutant unviable |
| result_set 617, 698x3 | yes, test + generic reader | a6a50f0 | caught; no timeout |
| search_checkpoint 332x3 | yes, test | a6a50f0 | caught |
| stored 3589, 3590 | yes, tests + restructure | a6a50f0, fd7e3a2, 27b7770 | caught; `<` boundary pair restructured, 7/7 |
| clippy doc_markdown (2 test docs) | yes | ce21e17 | clippy EXIT 0 |

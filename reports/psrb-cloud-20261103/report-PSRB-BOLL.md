STATUS: DONE

# PSRB-BOLL: cloud build report (Thu 5 Nov 2026)

Bollinger Bands for the PSRB strategy family (Pivot R1, Supertrend, RSI, Bollinger; NSE F&O stocks, cash, intraday),
built to PARTS.md section 4 as `indicators::bollinger`. Session token `PSRB-BOLL-20261103-1`. Nothing here is a
backtest result. Only items for the Mac are left (section 9).

## 1. Branch, START, commits

- Branch `cloud/psrb-boll-20261103`, cut from `origin/cloud/psrb-kit-20261103` at `56f3043d`, pushed after every step;
  never merged, rebased or force-pushed; no pull request; no other branch pushed. Before each push the branch was
  fetched and its OWNER file read: it held `PSRB-BOLL-20261103-1` every time, and no push was refused.
- START = `35234cead4009299a93fcd79a3d1fffd773dcc16` (PSRB-SEAM on `1ad6e24ef8e0`) = `origin/cloud/psrb-kit-20261103~1`.
- **Code tip = `19f410d2bd2c851c77aaed9974d647bb5df93487`** (`git rev-list -1 HEAD -- . ':(exclude)reports'`).
- The part's commits (`git log --oneline 56f3043d..`): `2b99f553` OWNER file; `8c77e2f9` the module, its tests, the
  bench wrapper, the `lib.rs` line and the two `ratio.rs` insertions; `5edb813d` this report, IN PROGRESS; `ea0b26a2`
  the four ledger blocks; `c567f9ad` two `&&` assertions in a test split into four plain ones (Route B had found their
  unrun branch sides); `19f410d2` L-PSRB-BOLL-04 carries its measured figures (reports/ does not land); then reports
  only. Every message ends with the `Co-Authored-By` line.
- Between `8c77e2f9` and `ea0b26a2` the tree cited D-8590 to D-8595 and printed the two bench ids before the ledgers
  held them, so `decision_citations` and `cost_invariants` would read them as missing there (said in `8c77e2f9`'s
  message). Every commit from `ea0b26a2` on is green on both.

## 2. Kit check (CLOUD.md "What you have", MANIFEST.md)

- `git diff --name-only START origin/cloud/psrb-kit-20261103` lists only the 13 files of `reports/psrb-cloud-20261103/`.
- All 12 MANIFEST.md rows match: `sha256sum CLOUD.md PARTS.md` (whole files) and, for the ten fenced files, the
  manifest's own `awk` extraction then `sha256sum`: `19dff357…`, `1a96fc9b…`, `9ed425e3…`, `f3d70ac2…`,
  `f032b525…`, `ecef3bdc…`, `c78e72b8…`, `e4818033…`, `d03d3ceb…`, `a99a83ad…`, `3c18cb3c…`, `2ed69004…`, each equal
  to its row. MANIFEST.md itself hashes to `85b873eb…`, the value the kit commit's message gives.
- Read whole: CLAUDE.md, CLOUD.md, MANIFEST.md, PARTS.md sections 0, 1, 2, 4 and 7, and the parts of section 3 that
  section 4 points to (the 3.1 statement form, 3.2's helpers, 3.5's bench tests, 3.6's L-PSRB-RSI-02, 3.7).
  `d3-recipe.txt`, `d3-route-b-cloud.sh` and `d3-covset.mjs` read and used as extracted (hashes above).

## 3. What was built

Files (all in PARTS.md 4.1 and 2.2; `git diff -U0 START 19f410d2 -- . ':(exclude)reports'` shows every hunk as a pure
insertion, `@@ -a,0 +b,c @@`, and no deleted line):

| file | change | where |
|---|---|---|
| `crates/indicators/src/bollinger.rs` | new, 692 lines | — |
| `crates/indicators/src/bollinger_tests.rs` | new, 1,046 lines, 23 tests | — |
| `crates/indicators/benches/ratio/psrb_boll.rs` | new, 132 lines | — |
| `crates/indicators/src/lib.rs` | +1 `pub mod bollinger;` | after base :63 `pub mod anchored;` |
| `crates/indicators/benches/ratio.rs` | +2 `#[path = "ratio/psrb_boll.rs"]`, `mod psrb_boll;`; +5 one statement re-binding `verdicts` with `.chain([psrb_boll::row(floor, &["C-PSRB-BOLL-01", "C-PSRB-BOLL-02"])])` (rustfmt's layout) | after :70 `// slot bench-mod PSRB-BOLL`; after :783 `    // slot bench-row PSRB-BOLL` |
| `docs/04-invariants.md` | +19 | after :9015, above `## GDFL readers: series codes, …` |
| `docs/05-decisions.md` | +157 | after :86591, above `### D-7310 — G3: …` |
| `docs/06-limits.md` | +13 | after :22023, above `## One not-owed authority (R4-19; D-7315)` |
| `docs/07-plan.md` | +6 | after :1057, above `## Grammar search limits recorded but not changed — OPEN (D-0751 to D-0754)` |

**The module.** `MAX_PERIOD` 4,096, `MAX_CLOSE` `1 << 32`, `MAX_WIDTH_MILLI` 10,000, `WIDTH_DEN` 1,000; `Refusal`
(`PeriodZero`, `PeriodAboveMax { period }`, `CloseOutOfRange { close }`, `WidthAboveMax { milli }`, `RingOutOfStep`,
`Corrupt(crate::Corrupt)`); `Close` (`new`, `paisa`, const); `Width` (`from_milli`, `milli`, const); `Bollinger` (`new`
allocates the ring once at full length, zeros; `period` and `warm` const; `fold`; `window`); `Window` (`period`,
`last`, `sum`, `sum_sq` const; `variance_numerator`, `basis_floor`, `above_upper`, `below_lower`, `upper_floor`,
`lower_ceil`, `z_milli`, `above_upper_sample`, `below_lower_sample`); `StepCost`, `summarize`, `flat_within`,
`measure_step` (n 20), `measure_period` (history 20,000). `#[cfg(test)] #[path = "bollinger_tests.rs"] mod tests;` at
the end.

- `fold`: `check_evaluable` (as `Corrupt`), then `Close::new` (above `MAX_CLOSE` refused), then ONE `get_mut(next)`
  whose `None` arm is `RingOutOfStep`; only then `replace`, `sum += c - old`, `sum_sq += c^2 - old^2`,
  `filled = min(filled + 1, period)`, `next + 1` or 0, `last = close`. `window()` copies the sums and `last` and never
  reads the ring; `new` starts `last` at `Close(1)`, which no reading shows.
- Band tests by squares in `i128`: `D > 0 && (1000 D)^2 > z_milli^2 V` and the mirror; the sample reading
  `(1000 D)^2 (n - 1) > z_milli^2 n V`, `None` at n = 1. `upper_floor`, `lower_ceil` and `z_milli` through
  `crate::vwap::isqrt_i128` (read, never edited) exactly as 4.3 writes them. Narrowing: `try_from(..).ok()` for
  `z_milli`, `unwrap_or(MAX_CLOSE)` for `basis_floor`, `unwrap_or(±6 * MAX_CLOSE)` for the band values (a private
  `BAND_EDGE`, whose doc says why no value inside the domain reaches it).
- Bounds: five `const _: () = assert!(...)` beside the arithmetic, each a `checked_mul` chain (`|D| <= 2^44`,
  `V <= 2^86`, `(1000 D)^2 n <= 2^120`, `z_milli^2 n V <= 2^125`, `10^6 D^2 <= 2^108`), over private `i128` edges tied
  to the public constants by a sixth.
- No `as`, no float, no indexing, no `unwrap`/`expect`/`panic!`, no sort, no `.contains(&` and no `assert!` in fn scope
  in production code; no telemetry, no print (gate 17). The module's docs say "the published definition … UNVERIFIED
  against `TradingView`'s own output" (2.4), and every doc block making a cost claim names
  `crates/indicators/benches/ratio.rs`.

**Where it differs from the letter of 4.2** (private or additive only; no public name, argument or return type
differs): (1) `Bollinger` has one more private field, `period: u16`, because `period()` is a `const fn` and reading the
ring's `usize` length back needs `u16::try_from` (not const) or an `as` (refused); (2) `StepCost` also derives `Default`;
(3) private helpers `Window::{n, reach, spread, beyond_sample}`, `square`, `nearest_rank`, `judge`, the synthetic `Walk`
(invented candles, xorshift, ±4 paisa a minute from Rs 25,000) and `measure(period, history, samples, rounds, base)`,
which both public measuring functions call; its `base` lets a test make the walk's candles refuse, so the two `?` arms
inside the measurement are driven by input. No `const` was dropped from 4.2's signatures.

**The bench wrapper** `benches/ratio/psrb_boll.rs`: `pub(crate) fn row(floor: u128, ids: &[&str]) -> bool` runs
`after_any_history` (C-PSRB-BOLL-01: n 20 after 2,000, 20,000, 200,000 candles) and `at_any_period` (C-PSRB-BOLL-02:
n 20, 200, 2,000 after 20,000), 11 rounds of 20,000 steps each, each step timed alone; it sorts each round, reads ranks
with `summarize`, prints per size the best round's p50, p99 and max with n and the median and worst rounds' p99, each
line led by its id, and judges the best rounds with `flat_within(…, 3_000, 2_000)`: p99 within the harness's
`CEILING_PERMILLE` (tied to it by a const assertion) of the first size's, and at most 2 µs. The verdict label is a
lookup, so no passing path depends on speed. The floor is printed as context (the best p99 in floors).

## 4. Gates (pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`, this container, final tip `19f410d2`)

| gate | command | result |
|---|---|---|
| fmt | `cargo fmt --all --check` | clean, exit 0 |
| clippy | `cargo clippy -p indicators --all-targets --locked -- -D warnings` | clean, exit 0 (lib, tests, bench) |
| indicators | `cargo test -p indicators --locked` | 22 binaries, 527 passed, 0 failed, 0 ignored (START: 504; these 23 added) |
| core | `cargo test -p core --locked --test cost_invariants --test decision_citations` | 5 passed; 4 passed |

Also run, beyond the bar: `cargo test -p core --locked` whole, 11 binaries, 261 passed, 0 failed;
`cargo test -p vocab --locked --test no_py_language --test workspace_is_rust --test stale_claims --test no_float`,
9, 18, 10 and 3 passed. The CI gates that read what this part adds were run with the workflow's own commands and tools
built by `rustc --edition=2024 -D warnings .github/<tool>.rs` on the pinned stable, on START and on the tip:

| gate | START | tip | what differs |
|---|---|---|---|
| 10 (`invariant_paths.rs` + `source_scan fns`/`modules`/`inline-mods` + `gates_ledger invariant-tests`) | rc 1: R427-11, STACK1028-03; HA-02, AG05-13 missing | rc 1, the same four | 5,093 table rows read against 5,083 (this part's 8 rows, header, separator); no refusal names a PSRB-BOLL row. A probe with one cited name misspelled was refused both as an undeclared proof and as a path reference, then restored byte for byte |
| 11 (`source_scan prod-files` + `gates_ledger banned-constructs`) | rc 1, 29 REFUSED lines | rc 1, the identical 29 | `bollinger.rs` joins the production closure (525 files against 524), `bollinger_tests.rs` does not; no line names it |
| 12 (`gates_bounds gate12-tokens` / `invariant_paths --resolve` / `gate12`) | rc 1 | rc 1, identical refusal list | 799 claim blocks against 797: `flat_within`'s doc proved by the bench file it names, the module doc (which also says UNVERIFIED, as 2.4 requires) counted under UNVERIFIED |
| 14 (`gates_bounds gate14` with the workflow's cover table) | rc 0 | rc 0 | `crates/indicators` covered, 19 cost claims against 17; its measurement-point pin of 6 still holds (no `ratio(` added) |
| 27 (`gates_ledger invariant-ids`) | OK, 3,479 ids | OK, 3,487 ids, all unique | — |
| 27b (`gates_ledger decision-numbers`) | rc 1: D-6247 heads two entries | rc 1, the same | 1,784 headings against 1,778 |

**Host reds: none.** On START (detached worktree, same commands) `cargo test -p indicators --locked` passed 504 of 504
and the two core tests 5 and 4; every red above is the base's own, the same on START.

**Each new test shown red under a hand change.** 27 hand changes, one at a time, of `bollinger.rs` at `c567f9ad`
(whose `.rs` files are the tip's: `git diff --stat c567f9ad 19f410d2` touches only `docs/06-limits.md`), each a literal
substitution checked to match exactly once, then `cargo test -p indicators --locked --lib bollinger`, then
`git checkout -- crates/indicators/src/bollinger.rs` and a clean `git status`. Every one built, every one turned at
least one test red, and every one of the 23 tests was red under at least one:

| id | hand change | red |
|---|---|---|
| H01 | period ceiling off by one (`> MAX_PERIOD + 1`) | T01 T22 |
| H02 | the zero-period guard never fires | T01 T22 |
| H03 | close ceiling off by one (`> MAX_CLOSE + 1`) | T02 |
| H04 | close floor off by one (`< 0`) | T02 |
| H05 | width ceiling refuses its own edge (`>=`) | T03 T10 T16 T17 T18 |
| H06 | the write position advances before the candle is checked (four lines moved above `check_evaluable`) | T02 T05 T19 |
| H07 | `RingOutOfStep` silently wraps and folds instead of refusing | T19 |
| H08 | warm one close early (`filled + 1 >= period`) | T01 T06 T16 T20 |
| H09 | the sum forgets the close leaving (`sum += new`) | T11 T14 T16 T20 |
| H10 | the sum of squares forgets it (`sum_sq += new * new`) | T11 T14 T16 T20 |
| H11 | the ring wraps one slot late (`next > len`) | T05 T06 T11 T14 T16 T19 T20 T21 T22 |
| H12 | `window()` reports `Close(1)` as its last close | T02 T06 T07 T20 |
| H13 | `V` off by one (`+ 1`) | T04 T07 T09 T10 T11 T13 T16 T17 T18 |
| H14 | a close ON the upper band counts as above (`>=`) | T08 T12 T13 T16 T17 T18 |
| H15 | the lower band's floor instead of its ceiling | T04 T08 T09 T15 T17 T18 |
| H16 | the upper band's ceiling instead of its floor | T04 T08 T09 T15 T17 T18 |
| H17 | negative z-score truncated toward zero (`-root`) | T09 T17 |
| H18 | the exact-root test of a negative z-score inverted (`!=`) | T07 T09 T17 T18 |
| H19 | no `V = 0` guard in `z_milli` (`variance < 0`) | T10 T11 T17 |
| H20 | `above_upper_sample` answers at n = 1 (`period == 0`) | T11 T16 |
| H21 | the sample test uses `n` for `n - 1` (population) | T12 T16 |
| H22 | `fold` reads the open, not the close | T14 |
| H23 | the ring slot taken from the timestamp's minute | T02 T14 T19 |
| H24 | `measure_step` passes rounds and samples swapped | T22 |
| H25 | `flat_within` accepts a zero first p99 (`base >= 0`) | T23 |
| H26 | nearest rank one too high (`div_ceil(100) + 1`) | T23 |
| H27 | `fold` reserves ring capacity (`reserve(len)`) | T21 |

T01 `a_zero_period_and_one_past_the_ceiling_are_refused`, T02 `a_close_outside_its_range_is_refused_and_changes_nothing`,
T03 `a_width_past_its_ceiling_is_refused`, T04 `a_zero_width_band_is_the_basis`, T05
`a_corrupt_candle_is_refused_and_changes_nothing`, T06 `no_window_before_period_closes_and_one_at_the_period_th`, T07
`the_basis_and_sigma_match_fixture_boll_1`, T08 `a_close_exactly_on_the_band_is_not_beyond_it`, T09
`a_steady_trend_sits_one_point_six_four_seven_sigma_above_its_mean`, T10
`a_flat_window_has_zero_sigma_and_the_close_on_both_bands`, T11 `a_window_of_one_close_has_zero_sigma_and_no_sample_sigma`,
T12 `sample_sigma_is_wider_by_the_root_of_n_over_n_minus_one`, T13 `an_all_up_and_an_all_down_series`, T14
`a_gap_enters_the_window_and_leaves_it_after_period_closes`, T15
`the_square_test_and_the_floor_band_agree_on_every_integer_close`, T16 `the_ring_equals_a_brute_force_window`, T17
`parity_with_the_published_definition`, T18 `the_extremes_never_overflow`, T19
`a_ring_out_of_step_is_refused_by_name_and_changes_nothing`, T20 `the_previous_window_reads_the_current_candle_excluded`,
T21 `the_ring_never_grows_after_new`, T22 `the_bench_measures_both_rows_at_tiny_sizes`, T23
`a_cost_is_read_by_rank_and_judged_flat_or_not`. What each pins: `part-PSRB-BOLL.md`.

## 5. Coverage: Route B, measured here on the pinned nightly

Toolchain: `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview`;
`rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, and its own `llvm-cov` and `llvm-profdata`
print `LLVM version 23.1.0-rust-1.99.0-nightly` (no `RUSTC_BOOTSTRAP`). One `CARGO_TARGET_DIR` per toolchain; `TMPDIR`
on a tmpfs. Reading **narrow**: no `Merge BASE-COV-` commit is in the tip's history.

The D2 set against START (`git diff --name-only --diff-filter=AMR START 19f410d2 -- '*.rs'`, three files added), written
before measuring, with its mutants lines for the background pass:

```
reading narrow
coverage crates/indicators/src/bollinger.rs whole
coverage crates/indicators/src/bollinger_tests.rs whole
coverage crates/indicators/benches/ratio/psrb_boll.rs whole
coverage crates/indicators/src/lib.rs line 64
coverage crates/indicators/benches/ratio.rs line 71
coverage crates/indicators/benches/ratio.rs line 72
coverage crates/indicators/benches/ratio.rs main 758-821
mutants crates/indicators/src/bollinger.rs whole
mutants crates/indicators/src/bollinger_tests.rs whole
mutants crates/indicators/benches/ratio/psrb_boll.rs whole
mutants crates/indicators/src/lib.rs line 64
mutants crates/indicators/benches/ratio.rs line 71
mutants crates/indicators/benches/ratio.rs line 72
mutants crates/indicators/benches/ratio.rs main 758-821
```

**The tests half**, in a detached worktree at `19f410d2`:
`D3_W=<scratch> D3_DIR=<kit extract> D3_NIGHTLY=~/.rustup/toolchains/nightly-2026-08-08-x86_64-unknown-linux-gnu
D3_LLVM_BIN=$D3_NIGHTLY/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=$D3_NIGHTLY/bin/cargo bash d3-route-b-cloud.sh
<tree> <K> <new P> <L> <set file> indicators`: one instrumented build (`RUSTFLAGS='-C instrument-coverage -Z
coverage-options=branch'`, `CARGO_PROFILE_DEV_OPT_LEVEL=0`; 23 artifacts, 0 reused, 21 objects from the build's own
report), `cargo test -p indicators --locked --no-fail-fast` 08:59:04 to 09:03:10 IST, 4 min, 527 passed, 0 failed,
21 profiles, `llvm-profdata merge -sparse`, `llvm-cov show --show-branches=count --show-line-counts-or-regions`, read by
`d3-covset.mjs`. The script exits 3 only because the two bench files are in no test object (the bench target has
`test = false`): its own "COVERAGE NOT READ" lines for them, measured next.

**The bench half** (CLOUD.md item 6): in the same tree, same `RUSTFLAGS` and target dir, `cargo bench -p indicators
--bench ratio --locked --no-run --message-format=json` (the workspace's bench profile; 11 s), the bench executable run
once with `LLVM_PROFILE_FILE` in a new folder (rc 0, 153 s, every row ok, so `main`'s passing path ran), its one profile
merged and shown for the two bench files with the same flags and read by the same `d3-covset.mjs` and set file.

| file and item | lines | branches | uncovered |
|---|---|---|---|
| `crates/indicators/src/bollinger.rs` whole | **229/229** | **40/40** | none |
| `crates/indicators/src/bollinger_tests.rs` whole | **700/700** | **18/18** | none |
| `crates/indicators/src/lib.rs` line 64 | 0/0 | 0/0 | no region (`pub mod bollinger;`) |
| `crates/indicators/benches/ratio/psrb_boll.rs` whole | 57/60 | 0/0 | 89-91, (F) |
| `crates/indicators/benches/ratio.rs` lines 71, 72 | 0/0 | 0/0 | no region (`#[path]`, `mod`) |
| `crates/indicators/benches/ratio.rs` `main` 758-821 | 24/26 | 1/2 | 816 True, 817, 818, (F) |

**(F) items, the only uncovered lines and sides, each a self-checking bench's own failing arm (recipe step 4):**

- `crates/indicators/benches/ratio/psrb_boll.rs:89` `Err(why) => {`, `:90`
  `println!("{id} {size}: the step was refused ({why:?})   FAILED");`, `:91` `return false;`: the row's own failing arm,
  reached only when a measuring function refuses, which fails the row. Its fixed sizes cannot make the walk refuse;
  the library arms behind it are driven in `the_bench_measures_both_rows_at_tiny_sizes`.
- `crates/indicators/benches/ratio.rs:816:8` branch True `if verdicts.contains(&false) {`, `:817`
  `println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");`, `:818` `std::process::exit(1);`: the
  harness's breach exit (base code, in the set because this part adds a statement to `main`), reached only when a row
  fails.

GAPs 0, (U) items 0, and nothing NOT PROVED: the tests half's two NOT READ lines are the two bench files, which the
bench half measured and read. The first measurement, at `ea0b26a2`, found `bollinger_tests.rs` at 698/698 lines
and 22/26 branches: the four unrun sides were `&&` written inside two `assert!`s; `c567f9ad` splits each into two
assertions, and the measurements at `c567f9ad` and at the tip give the figures above. Mutants: deferred (operator,
30 Oct); nothing of recipe step 5 ran.

## 6. Benches (advisory; the Mac's gate 8 decides)

Machine: this container, Linux 6.18 x86_64, 4 vCPU Intel Xeon @ 2.80 GHz, 1 thread per core. Command:
`cargo bench -p indicators --bench ratio --locked` on the pinned stable, the machine otherwise idle, at `c567f9ad` (the
tip's `.rs` files). Printed lines, whole:

```
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 589 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                          1103467 ps = 1873.458 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in          1103762 ps ->  1099907 ps   ratio 0.996x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in         1103762 ps ->  1108181 ps   ratio 1.004x  ok
  C-I-02 step: a wandering candle -> a motionless one         1103760 ps ->   732666 ps   ratio 0.663x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle  1103760 ps ->  1046992 ps   ratio 0.948x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  8732 ps over   2 iteration(s) = 4366 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          26405 ps over   5 iteration(s) = 5281 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             26380 ps over   5 iteration(s) = 5276 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         32027 ps over   6 iteration(s) = 5337 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover         1040976 ps ->   901729 ps   ratio 0.866x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1817348 ps ->  1945256 ps   ratio 1.070x  ok
  the stable floor is 599 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1770898 ps = 2956.424 floors, budget 5200   ok
C-PSRB-BOLL-01 after 2000 candles, n 20: best round p50 36 ns, p99 46 ns, max 3502 ns (n 20000); p99 of the median round 46 ns, of the worst 49 ns; the best p99 is 78.098 floors
C-PSRB-BOLL-01 after 20000 candles, n 20: best round p50 35 ns, p99 46 ns, max 48920 ns (n 20000); p99 of the median round 46 ns, of the worst 46 ns; the best p99 is 78.098 floors
C-PSRB-BOLL-01 after 200000 candles, n 20: best round p50 35 ns, p99 46 ns, max 44849 ns (n 20000); p99 of the median round 46 ns, of the worst 54 ns; the best p99 is 78.098 floors
C-PSRB-BOLL-01 the best rounds' p99 across the history sizes: within 3.000x of the first and at most 2000 ns   ok
C-PSRB-BOLL-02 n 20, after 20000 candles: best round p50 35 ns, p99 45 ns, max 78 ns (n 20000); p99 of the median round 46 ns, of the worst 46 ns; the best p99 is 76.400 floors
C-PSRB-BOLL-02 n 200, after 20000 candles: best round p50 35 ns, p99 44 ns, max 18609 ns (n 20000); p99 of the median round 45 ns, of the worst 45 ns; the best p99 is 74.702 floors
C-PSRB-BOLL-02 n 2000, after 20000 candles: best round p50 35 ns, p99 39 ns, max 54689 ns (n 20000); p99 of the median round 44 ns, of the worst 48 ns; the best p99 is 66.213 floors
C-PSRB-BOLL-02 the best rounds' p99 across the period sizes: within 3.000x of the first and at most 2000 ns   ok
all ratios within the ceiling
```

Exit 0. Each sample is one step plus two clock reads, so the figures are upper bounds on a step. The rare maxima (up to
55 µs) are single samples; the judged p99 is the best round's. The instrumented coverage run of section 5 is a bench run
too, and its PSRB lines were (coverage counters in every step, so not a cost figure; every row ok there as well):

```
C-PSRB-BOLL-01 after 2000 candles, n 20: best round p50 173 ns, p99 180 ns, max 25844 ns (n 20000); p99 of the median round 194 ns, of the worst 240 ns; the best p99 is 15.286 floors
C-PSRB-BOLL-01 after 20000 candles, n 20: best round p50 183 ns, p99 190 ns, max 29571 ns (n 20000); p99 of the median round 198 ns, of the worst 248 ns; the best p99 is 16.135 floors
C-PSRB-BOLL-01 after 200000 candles, n 20: best round p50 183 ns, p99 190 ns, max 24670 ns (n 20000); p99 of the median round 190 ns, of the worst 248 ns; the best p99 is 16.135 floors
C-PSRB-BOLL-01 the best rounds' p99 across the history sizes: within 3.000x of the first and at most 2000 ns   ok
C-PSRB-BOLL-02 n 20, after 20000 candles: best round p50 183 ns, p99 190 ns, max 48324 ns (n 20000); p99 of the median round 202 ns, of the worst 252 ns; the best p99 is 16.135 floors
C-PSRB-BOLL-02 n 200, after 20000 candles: best round p50 183 ns, p99 189 ns, max 45291 ns (n 20000); p99 of the median round 195 ns, of the worst 243 ns; the best p99 is 16.050 floors
C-PSRB-BOLL-02 n 2000, after 20000 candles: best round p50 173 ns, p99 177 ns, max 25287 ns (n 20000); p99 of the median round 181 ns, of the worst 199 ns; the best p99 is 15.031 floors
C-PSRB-BOLL-02 the best rounds' p99 across the period sizes: within 3.000x of the first and at most 2000 ns   ok
all ratios within the ceiling
```

L-PSRB-BOLL-04 records the stable run's figures as advisory, with the machine and the command.

## 7. Decisions drafted (this part's block only)

`docs/05-decisions.md`, above D-7310: **D-8590** the module and its reading (the published form, UNVERIFIED against
`TradingView`'s output; the current candle included; a runtime period of 1 to 4,096 in a ring allocated once; no
vocabulary position; streaming; timestamps not read); **D-8591** exact band tests by squares in `i128` and the
const-asserted domain, with why each edge (the Rs 4.29 crore reading is the design's and UNVERIFIED: no charter source
records the highest NSE price, and a close above it is refused either way); **D-8592** population sigma the reading
(report A marks the divisor as secondary sources only, so UNVERIFIED), sample sigma labelled, `None` at n = 1;
**D-8593** band values and the z-score exact via `isqrt_i128`, for records and seeds, never a decision; **D-8594**
refusals change nothing, `RingOutOfStep` named and unreachable; **D-8595** the two benches, sizes, ceiling, 2 µs budget
(the 400 ns target an ESTIMATE), the helpers copied per part, gate 14's row the main session's. D-8596 to D-8599 unused.
Before writing, `grep -nE '^#{2,4} +D-85[89][0-9]'` and `'^#{2,4} +D-860[0-9]'` on docs/05 found no heading. The code
cites only D-8590 to D-8595. docs/04: PSRB-BOLL-I01 to I06 and C-PSRB-BOLL-01/-02, proofs in gate 10's path form;
docs/06: L-PSRB-BOLL-01 to -04; docs/07: one row. No commit hash appears in any ledger block.

## 8. Not done, open, UNVERIFIED

- Nothing of the part is left undone. Mutants are deferred by the operator's decision; the set file carries the lines.
- UNVERIFIED, stated where it binds: parity is with the published formulas, not with `TradingView`'s output (D-8590);
  the population divisor rests on secondary sources per report A (D-8592); that no NSE share trades above Rs 4.29 crore
  (D-8591, L-PSRB-BOLL-01). Report A itself was not readable here; its readings are taken from PARTS.md section 4.
- The bench figures are one cloud run (advisory); the budget and target stay ESTIMATES until the Mac's gate 8.
- **A rule slip, disclosed.** One shell command of this session, while copying the part file into place, began with a
  stray version probe of the interpreter CLOUD.md rule 12 forbids (its output discarded). Nothing used it, it wrote
  nothing, and no tracked file names or depends on it; every tool used for the work was Rust, git, the shell, `jq`,
  `awk`, `perl` one-line substitutions and `node` (the kit's own `d3-covset.mjs`).

## 9. NEEDS THE MAC (PARTS.md 4.7, as 3.7 with the C-PSRB-BOLL rows)

1. Check 1 on the Mac's pin: Route B on the D2 set of section 5 against the seam commit; the bench wrapper and `main`
   by one instrumented run of the bench target.
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-BOLL rows.
3. Gates 10, 11, 12 and 14 on the Mac (section 4: here each adds no refusal); **gate 14's cover-table row for
   C-PSRB-BOLL-01 and -02 is the main session's at the family assembly** (PARTS.md 7.6).
4. macOS rows: none expected (no platform code; the only clock is `Instant` in the measuring functions).
5. The anchors on the then-current feat/db-fix (each insertion directly under its slot or directly above its named
   heading; nothing else in a shared file changed).
6. Mutants: the background pass, on the set file above.

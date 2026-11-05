STATUS: DONE

# PSRB-CCI cloud report (token PSRB-CCI-20261105-1)

Branch `cloud/psrb-cci-20261105`, cut from `origin/cloud/psrb-kit-20261105` (`fdc091ba69c9`).
START = `git rev-parse origin/cloud/psrb-kit-20261105~1` = `deaa0f5c0606464f2ec79dc9e94de03792ece326` (PSRB-SEAM-2,
parent PSRB-SEAM `35234cead400`, its parent `1ad6e24ef8e0`). Code tip (`git rev-list -1 HEAD -- . ':(exclude)reports'`)
= `bd65eb6f7d8eb2a1af9dc75b3cf3b380d1df70c1`. Only items for the Mac are left (NEEDS THE MAC, at the end). The short form
for the Mac's check is `part-PSRB-CCI.md`; the machine-read summary is `result-PSRB-CCI.json.md`.

Commits on the branch, oldest first: `e867c897` the OWNER file; `cfd8a3c8` this report IN PROGRESS with the kit check
and the START baseline; `c8e89ddc` the code (module, tests, bench wrapper, the `lib.rs` line, the two `ratio.rs`
insertions); `bd65eb6f` the four ledger blocks; then this report, the part file and the result file (`reports/` only).
Every push was preceded by `git fetch origin cloud/psrb-cci-20261105`, whose OWNER file held this token each time (the
first push created the branch); no push was refused, nothing was forced, merged or rebased, and no pull request was
opened.

## Kit check

- `git diff --name-only $START origin/cloud/psrb-kit-20261105` lists only the 13 files under
  `reports/psrb-cloud-20261105/`.
- Every MANIFEST row matches (each fenced file extracted with MANIFEST.md's own `awk` line): `CLOUD.md` `004a7f9b...`,
  `PARTS.md` `edcfdb27...`, `PARTS-WILLR-CCI.md` `0c502de2...` (whole files); `d3-recipe.txt` `9ed425e3...`,
  `d3-route-b.sh` `f3d70ac2...`, `d3-route-b-cloud.sh` `f032b525...`, `d3-covset.mjs` `ecef3bdc...`, `wr-cci.mjs`
  `ea3d831f...`, `pops.mjs` `261a5697...`, `cpr-wr6.mjs` `0daa7c91...`, `repair1.mjs` `a2f7bd48...`, `repair3.mjs`
  `d85ca768...` (fenced). `sha256sum MANIFEST.md` = `2ad1450d...`.
- `wr-cci.mjs run` (node, from its extracted copy) prints every CCI fixture figure of PARTS-WILLR-CCI.md 6.5 (CCI-1's
  four warm rows, CCI-2 and CCI-2m exactly +/-200, CCI-3 at 700/3, the period-6 ceiling 200,000, the period-3 window of
  one typical price, period 1 and period 2) and its random cross-check, 8,631 warm readings, 0 disagreements;
  `repair3.mjs`'s CCI block gives the `u16::MAX` ceiling (S 196,608, dev 196,602, D 393,204, `milli` 2,184,500,000).
  The Rust tests pin the same figures independently (below).
- Decision ids: `grep -n -E '^#{2,4} +D-86(4|5)[0-9]' docs/05-decisions.md` printed nothing at START (exit 1): no clash
  with D-8650..D-8659. No `D-865x` appeared anywhere outside `reports/` at START.
- The four ledger anchors held at START: docs/04 :8671, docs/05 :87133, docs/06 :21624 and docs/07 :929 are the headings
  PARTS-WILLR-CCI.md 3.4 names, each unique in its file and with a blank line above it.

## START baseline (before the first code commit)

The code tree at the OWNER commit equals START outside `reports/` (`git diff --stat $START HEAD -- . ':(exclude)reports'`
printed nothing). In a detached worktree at START, on the pinned stable `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
`cargo test -p indicators --locked`: 22 test binaries, 504 passed, 0 failed, 0 ignored, exit 0 (5 Nov 2026, ~13:50 UTC).
This host has no indicators red.

## Toolchains and machine

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)` (rust-toolchain.toml's pin).
- Coverage nightly: `rustup toolchain install nightly-2026-08-08 --profile minimal --component llvm-tools-preview`;
  `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its own `llvm-cov --version` and
  `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`. No `RUSTC_BOOTSTRAP`.
- Machine: Linux 6.18.44 x86_64, 4 vCPU `Intel(R) Xeon(R) Processor @ 2.10GHz`, 15 GiB memory, running as root;
  TMPDIR on a tmpfs (`/dev/shm`); one CARGO_TARGET_DIR per toolchain (stable and the hand-change runs in scratch target
  dirs of their own, the nightly in the scratch deep dir).

## What was built (PARTS-WILLR-CCI.md 6.1-6.6, nothing else)

`indicators::cci`: the Commodity Channel Index on the typical price, in the published form
`CCI = (TP - SMA) / (0.015 MD)` with `TP = hlc3`, `SMA = ta.sma(hlc3, n)` and `MD` the mean absolute deviation from
that same SMA over the same `n` candles, the current candle included (UNVERIFIED against `TradingView`'s own output, as
the module says). In thirds and n-multiples every quantity is an integer: `T = H + L + C`, `S` the window's sum of `T`,
`dev = n T - S`, `D = sum |n T_i - S|`, and `CCI = 200 n dev / (3 D)`.

- `Cci` keeps `T` in a `VecDeque<i128>` built by `VecDeque::with_capacity(period)` in `new`; when full, `fold` pops the
  oldest value BEFORE it pushes, reading the pop's `Option` with `unwrap_or(0)` (never a `match` or `if let`; the
  doc comment says why the default never applies), so the ring never holds more than `period` values and a ring built
  by `new` never reallocates. The running sum `S` and the current `T` (`last`) are kept, so `fold` is two `i128`
  additions, at most one pop and one push, and `reading()` reads the ring once, only to sum `D`.
- `Reading` copies `period`, `dev` and `D`; `above`/`below` compare `200_000 n dev` with `3 m D` in `i128` (a tie is
  neither, both false when `D == 0`); `milli()` is `(200_000 n dev).div_euclid(3 D)` narrowed by `i64::try_from(..).ok()`,
  `None` when `D == 0`; `flat()` is `D == 0`.
- `Level` holds thousandths of a point (`i32`), `MAX_LEVEL_MILLI` = 10,000,000; `from_milli` refuses beyond either edge
  with `LevelOutOfRange { milli }`.
- Every bound of 6.3 is a `const _: () = assert!(...)` taken with `checked_mul`: `T < 2^65`, `S` and `n T < 2^81`,
  `D < 2^97`, `200_000 n |dev| < 2^115`, `3 |m| D < 2^123`.
- Refusals: `PeriodZero`, `LevelOutOfRange { milli }`, `Corrupt(why)`; `fold` checks the candle before it moves anything.
- The measuring half of the bench rows lives in the module (`measure_step` for C-PSRB-CCI-01, `measure_period` for
  C-PSRB-CCI-02 timing `fold` ALONE, `measure_reading_period` for the unjudged reading line, `StepCost`, `summarize`,
  `flat_within`); the wrapper sorts, prints and judges.

### Files and functions

| file | lines | what |
|---|---|---|
| `crates/indicators/src/cci.rs` | 581 (new) | `MAX_LEVEL_MILLI`; `Refusal`; `Level::{from_milli, milli}`; `Cci::{new, period, fold, warm, reading}`; `Reading::{period, deviation, total_deviation, flat, milli, above, below}`; `StepCost`, `summarize`, `flat_within`, `measure_step`, `measure_period`, `measure_reading_period`; private `MILLI_SCALE`, `PRICE_LIMIT`, `TYPICAL_LIMIT`, `PERIOD_LIMIT`, `SUM_LIMIT`, `TOTAL_LIMIT`, `SCALE_LIMIT`, `LEVEL_EDGE` and eight const assertions, `Reading::{scaled, threshold}`, `nearest_rank`, `STEP_PERIOD`, `PERIOD_HISTORY`, `STEP_ABOVE`, `STEP_BELOW`, `WALK_BASE`, `WALK_REACH`, `MINUTE_MICROS`, `measure` (the one body behind the three measuring functions), `fold_only`, `read_and_judge`, `Walk`; the test module mounted last as `#[cfg(test)] #[path = "cci_tests.rs"] mod tests;` |
| `crates/indicators/src/cci_tests.rs` | 1,091 (new) | the 25 tests; helpers `candle`, `level`, `cci`, `folded`, `states`, `stream`, `last_reading`, `repeated`, `cci_2`, `cci_2m`, `cci_3`, `typical`, `brute` (oracle 1, loops of its own), `seen`, `Ratio` and `gcd` (exact rationals), `published_parts` and `published` (oracle 2, the published definition), `Draw`, `spread`, `draw_series`; constants `MINUTE`, `CCI_1`, `CCI_1_T`, `CCI_1_WARM`, `BASE` |
| `crates/indicators/benches/ratio/psrb_cci.rs` | 157 (new) | `pub(crate) fn row(floor, ids)`, `one_step_however_long_the_history` (C-PSRB-CCI-01), `one_fold_whatever_the_period` (C-PSRB-CCI-02), `the_reading_across_periods` (the unjudged line), `best_round`, `verdict`; `SAMPLES` 20,000, `ROUNDS` 9, `BUDGET_NS` 2,000, `HISTORIES`, `PERIODS` |
| `crates/indicators/src/lib.rs` | +1 | `pub mod cci;` directly under `// slot PSRB-CCI (cci)` (new line 80) |
| `crates/indicators/benches/ratio.rs` | +7 | `#[path = "ratio/psrb_cci.rs"]` and `mod psrb_cci;` directly under `// slot bench-mod PSRB-CCI` (new lines 70-71); under `    // slot bench-row PSRB-CCI`, `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_cci::row(floor, &["C-PSRB-CCI-01", "C-PSRB-CCI-02"])]).collect();` as rustfmt wraps it (new lines 790-794) |
| `docs/04-invariants.md` | +21 | `## The Commodity Channel Index for the PSRB family (PSRB-CCI; D-8650 to D-8655)`: PSRB-CCI-I01..I06, C-PSRB-CCI-01, -02 |
| `docs/05-decisions.md` | +195 | D-8650..D-8655 |
| `docs/06-limits.md` | +13 | `## The Commodity Channel Index for the PSRB family: limits (PSRB-CCI; D-8650 to D-8655)`: L-PSRB-CCI-01..04 |
| `docs/07-plan.md` | +6 | `## PSRB-CCI: the Commodity Channel Index — 2026-11-05 (D-8650 to D-8655)`, one row |

`git diff --stat $START bd65eb6f -- . ':(exclude)reports'`: 9 files, 2,072 insertions, 0 deletions. Each shared-file
hunk is an insertion only (`@@ -a,0 +b,n @@`) directly after the anchor PARTS-WILLR-CCI.md 3.4 names: `lib.rs` after
:79, `ratio.rs` after :69 and :787 (the two PSRB-CCI slots), docs/04 after :8670, docs/05 after :87132, docs/06 after
:21623, docs/07 after :928. No `trend.rs`, `vwap.rs`, `daily.rs`, `evaluator.rs`, first-wave file, `tests/` file,
`Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `AGENTS.md`, `docs/00-charter.md`, `.github/` or PSRB-WILLR line changed.

### Against PARTS-WILLR-CCI.md 6.2 (signatures)

Every name, argument and return type is as shown, and every `const fn` shown is `const` (none dropped):
`Level::from_milli`, `Level::milli`, `Cci::period`, `Reading::{period, deviation, total_deviation, flat}`. Added beyond
the shown text: `Level` derives `Clone, Copy, Debug, PartialEq, Eq` (6.2 says `Copy`); `StepCost` derives
`Clone, Copy, Debug, Default, PartialEq, Eq` (5.2 shows only its fields); the private items listed above. A layout
probe of the same field types with rustc 1.97.1 on x86_64 reads `Cci` 80 bytes, `Reading` 48 bytes, `VecDeque<i128>`
32 bytes, `i128` align 16; `the_state_is_a_fixed_size` asserts `Cci` at most 96 bytes (6.5's ESTIMATE) and `Reading` at
most 48. The same probe shows `VecDeque::with_capacity(n)` reserving exactly `n` at n = 1, 20, 1,000 and 65,535, and a
clone of a two-value deque reserving 2: the design's reading that a clone reserves only its length holds on the pinned
std. The module doc says both halves of PARTS-WILLR-CCI.md 2's sentence: a ring built by `new` never reallocates; a
clone reserves only its length; the measured path never clones.

### Gate 12 and the word "flat" in this part's doc blocks

No doc block in the three new files says "flat window" or "flat state": they say "a window of one typical price" or
link [`Reading::flat`] (scrubbed by gate 12 as `::flat`), and `flat()` and `flat_within` are not claims. The two cost
claims (`O(1)`) are the module doc (it names `C-PSRB-CCI-01`, `C-PSRB-CCI-02` and `crates/indicators/benches/ratio.rs`,
and carries UNVERIFIED for the definition) and `Cci::fold`'s doc (the two ids and `crates/indicators/benches/ratio.rs`).
Gate 12 run here as CI runs it counts exactly these two new claim blocks, both proved (below).

## Tests by name (25, `crates/indicators/src/cci_tests.rs`; every name of PARTS-WILLR-CCI.md 6.5)

1. `a_zero_period_is_refused_and_every_other_period_is_accepted` (0 refused; 1, 20, 65,535 accepted, empty, not warm,
   no reading, ring capacity at least `period`).
2. `a_level_past_ten_thousand_points_is_refused_at_both_edges` (10,000,000, -10,000,000, 0, +/-200,000 read back;
   10,000,001, -10,000,001, `i32::MAX`, `i32::MIN` refused naming the value).
3. `the_first_reading_appears_after_exactly_period_candles` (periods 1, 2, 3, 20, 65,535: none after `period - 1`, one
   after `period`, equal to the brute force).
4. `cci_matches_fixture_cci_1` (T at every candle; S, dev, D, `milli` at candles 3-6 = 1,291/85/196/115,646,
   1,340/116/304/101,754, 1,381/87/324/71,604, 1,394/-118/272/-115,687; above +100 at 3 and 4 only, below -100 at 6 only).
5. `a_tie_at_plus_two_hundred_is_neither_above_nor_below` (CCI-2: S 2,149, deviations -49 x5, 35, 210, dev 210, D 490,
   `milli` 200,000, neither at 200,000, above 199,999, below 200,001; CCI-2m: S 2,051, dev -210, D 490, -200,000, neither
   at -200,000, below -199,999, above -200,001).
6. `a_floored_milli_equal_to_the_level_is_not_a_tie` (period 7: T 3,370,001, S 21,490,001, dev 2,100,006, D 4,900,010,
   `milli` 200,000, the products 2,940,008,400,000 and 2,940,006,000,000: above true, below false).
7. `the_confirmed_level_is_three_mean_deviations` (300 random series at periods 1-40, more than 10,000 warm readings:
   `above(200,000)` is `n dev > 3 D` and is `TP - SMA > 3 MD` in exact rationals; the mirror at -200; both levels met
   and missed).
8. `the_reading_never_passes_one_hundred_n_over_three` (CCI-3: S 2,101, dev 6, D 12, 233,333, above 200,000; 1,000
   random series at periods 1-12: `2 |dev| <= D`, equality with `dev != 0` exactly when the current candle is the only
   one strictly on its side, with `dev == 0` exactly when `D == 0`, both cases met; at 65,535: S 196,608, dev 196,602,
   D 393,204 = 2 dev, `milli` 2,184,500,000 = 100,000 n / 3, above `i32::MAX`).
9. `a_period_of_six_or_less_never_reads_above_two_hundred` (period 6: dev 5, D 10, 200,000, not above; the shape at
   periods 2-6 reads 66,666, 100,000, 133,333, 166,666, 200,000; 1,000 random series at periods 1-6, more than 10,000
   readings, none above +200).
10. `a_flat_window_has_no_value_and_compares_false` (period 3 over (101, 99, 100), (102, 98, 100), (100, 100, 100): ring
    300, 300, 300, D 0, dev 0, `flat()`, `milli` None, every comparison false at -10,000,000, -200,000, 0, 200,000,
    10,000,000).
11. `a_period_of_one_is_always_flat` (2,000 random candles at period 1: every reading has D 0, dev 0, no `milli`).
12. `a_period_of_two_reads_plus_or_minus_sixty_six_point_six` (66,666, then -66,667, then no value; 5,000 random candles
    at period 2: every reading with a value is exactly +200/3 or -200/3).
13. `milli_is_the_floor_of_the_exact_ratio` (500 random series, more than 10,000 readings with a value:
    `milli 3D <= 200,000 n dev < (milli + 1) 3D`).
14. `the_ring_equals_a_brute_force_window` (BRUTE FORCE: 1,000 random series at periods 1-40, every candle, more than
    50,000 compared: S while filling, and S, dev, D once full, against loops of their own).
15. `parity_with_the_published_definition` (PARITY: CCI-1, CCI-2, CCI-2m, CCI-3 and 400 random series, more than 10,000
    readings: no value exactly when MD is 0, `milli` the floor of `1000 CCI`, every comparison at +/-200, +/-100 and 0,
    against `(TP - SMA) / (0.015 MD)` in exact rationals written apart from the module).
16. `the_largest_prices_and_period_never_overflow` (65,535 candles alternating (1, 1, 1) and `i64::MAX` x3: S, dev, D
    in closed form, `milli` `(-6,553,500,000).div_euclid(98,304)`, the const bounds restated at run time, the levels
    +/-10,000 points; one more fold: `6,553,500,000.div_euclid(98,304)`; period 2 of the same: 66,666, -66,667, 66,666).
17. `a_price_gap_enters_the_window_and_leaves_it_after_period_candles` (periods 3 and 20: S, dev, D equal the brute force
    at every step; the 10% jump's T in the window for exactly `period` candles from its own fold).
18. `a_time_gap_reads_as_contiguous_candles` (stamps a minute apart, a day apart, all equal, receding, and an open at the
    low with volume 0 and an open interest: equal states and equal readings, 187 of them).
19. `no_reading_depends_on_a_later_candle` (periods 1, 5, 20, cuts at 0, 10, 37, 60, 100: states to the cut equal,
    readings to the cut equal, states after it changed).
20. `a_corrupt_candle_is_refused_and_changes_nothing` (HighBelowLow, RangeOverflows, PriceOutsideRange twice,
    NegativeVolume, PriceNotPositive, at an empty, filling, full and long-running window of period 10; the whole state
    compared).
21. `the_ring_never_grows_after_new` (periods 1, 20, 1,000: 100,000 folds each, length at most `period` at every fold and
    `period` at the end, capacity equal to what `new` reserved at every fold).
22. `the_state_is_a_fixed_size` (`Cci` at most 96 bytes, `Reading` at most 48, `Level` 4).
23. `the_previous_reading_judges_with_the_current_candle_excluded` (period 14: the kept copy is unchanged by the fold and
    equals the brute force of the window that excludes the current candle).
24. `the_bench_measures_every_row_at_tiny_sizes` (the three measuring functions warm and cold, rounds of the right
    shape; period 0 refused; a walk from 0 paisa refusing in the history and in a timed step; the walk's candles
    evaluable and 8 paisa wide).
25. `a_cost_is_read_by_rank_and_judged_flat_or_not` (ranks at 1, 100 and 1,000 samples; within the ceiling and budget at
    both edges; a zero first p99, a zero later p99, past the ceiling, past the budget and nothing measured all fail).

## Gates (pinned stable 1.97.1, at the code tip `bd65eb6f`, 14:37:40-14:38:09 UTC, 5 Nov 2026)

| command | result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy -p indicators --all-targets --locked -- -D warnings` | exit 0 (run again after touching `lib.rs` and `ratio.rs`, so `indicators` was checked afresh) |
| `cargo test -p indicators --locked` | exit 0: 22 binaries, 529 passed, 0 failed, 0 ignored (START 504 + the 25 new) |
| `cargo test -p core --locked --test cost_invariants --test decision_citations` | exit 0: `cost_invariants` 5 passed, `decision_citations` 4 passed |

No host red: every command above is green here, so none needed the START or non-root proof.

### CI gates 0, 10, 10b, 11, 12, 14, 25, 27 and 27b, run here as CI runs them (beyond the bar; for the Mac)

Each step's `run:` body was taken from `.github/workflows/ci.yml` as it stands and run with `bash` in the clean checkout
at `bd65eb6f`, and again in the clean START worktree (the tools are built by `rustc` from `.github/*.rs`, as CI builds
them). Gates 0, 10b, 14, 25 and 27 pass on both. Gates 10, 11, 12 and 27b fail on START and on the tip with the SAME
refusals (a `diff` of the two outputs, test-binary lines aside, shows only walked counts), none naming a PSRB-CCI file
or id: gate 10 refuses rows R427-11 and STACK1028-03 ("a ✓ row names no test"); gate 11 refuses files in `crates/api`,
`crates/cli`, `crates/pull` and `crates/store` (rules 1, 3, 4, 6 and 7; `cci` appears nowhere in its output); gate 12
refuses 18 blocks in `crates/store`; gate 27b refuses D-6247 (two headings). This part only moves the counts: gate 11
walks 525 production files (524 at START); gate 12 reads 799 claim blocks (797), the two new ones proved (one names the
tracked bench file, the module doc also says UNVERIFIED); gate 14 reads indicators' cost claims as 19 (17), covered;
gate 27b reads 1,784 headings (1,778), no new duplicate; gate 10 accepts the eight new rows.

## Each new test shown red under a hand change (CLOUD.md rule 6)

A node script (`red.mjs`, this session's, not tracked; sha256 `3579596c...`) applied 27 hand changes to
`crates/indicators/src/cci.rs` one at a time in a worktree of this session's own at `bd65eb6f`, ran
`cargo test -p indicators --locked --lib cci:: -- --test-threads=1` on the pinned stable for each, recorded the failed
tests and wrote the file back; it printed `RESTORED byte for byte`, `TESTS 25, shown red 25; changes 27, build failures
0`, and `git status --porcelain` of that worktree was empty afterwards.

| # | hand change of `cci.rs` | tests red |
|---|---|---|
| 1 | `Cci::new` accepts period 0 | `a_zero_period_is_refused_and_every_other_period_is_accepted`, `the_bench_measures_every_row_at_tiny_sizes` |
| 2 | `Level::from_milli` refuses +10,000 points itself (`>=`) | `a_level_past_ten_thousand_points_is_refused_at_both_edges`, `a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_is_always_flat`, `the_largest_prices_and_period_never_overflow` |
| 3 | the same at -10,000 points (`<=`) | the same four |
| 4 | the ring keeps one value too many | 13, among them `the_ring_equals_a_brute_force_window`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `the_ring_never_grows_after_new`, `milli_is_the_floor_of_the_exact_ratio` |
| 5 | the pop comes after the push (every value unchanged) | `the_ring_never_grows_after_new` only |
| 6 | warm one candle early | 12, among them `the_first_reading_appears_after_exactly_period_candles`, `a_zero_period_is_refused_and_every_other_period_is_accepted` |
| 7 | `dev` with its sign flipped | 13, among them `cci_matches_fixture_cci_1`, `a_tie_at_plus_two_hundred_is_neither_above_nor_below`, `the_reading_never_passes_one_hundred_n_over_three` |
| 8 | `dev` taken with `n + 1` | 15, among them `a_period_of_one_is_always_flat`, `a_flat_window_has_no_value_and_compares_false` |
| 9 | `D` without the absolute value | 14, among them `parity_with_the_published_definition`, `milli_is_the_floor_of_the_exact_ratio` |
| 10 | `D` skips the oldest value | 13, among them `the_ring_equals_a_brute_force_window`, `the_previous_reading_judges_with_the_current_candle_excluded` |
| 11 | `flat()` also when `dev == 0` | `parity_with_the_published_definition` |
| 12 | `milli()` reads 0 on a window of one typical price | `a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_is_always_flat`, `a_period_of_two_reads_plus_or_minus_sixty_six_point_six`, `parity_with_the_published_definition` |
| 13 | `milli()` truncates toward zero (`/`) | `milli_is_the_floor_of_the_exact_ratio`, `cci_matches_fixture_cci_1`, `a_period_of_two_reads_plus_or_minus_sixty_six_point_six`, `parity_with_the_published_definition`, `the_largest_prices_and_period_never_overflow` |
| 14 | `above` counts a tie (`>=`) | `a_tie_at_plus_two_hundred_is_neither_above_nor_below`, `a_period_of_six_or_less_never_reads_above_two_hundred`, `the_confirmed_level_is_three_mean_deviations`, `parity_with_the_published_definition` |
| 15 | `above` compares the floored `milli` | `a_floored_milli_equal_to_the_level_is_not_a_tie`, `a_period_of_two_reads_plus_or_minus_sixty_six_point_six` |
| 16 | `below` counts a tie (`<=`) | `a_tie_at_plus_two_hundred_is_neither_above_nor_below`, `the_confirmed_level_is_three_mean_deviations`, `parity_with_the_published_definition` |
| 17 | the level scaled by `2 D`, not `3 D` | 6, among them `the_confirmed_level_is_three_mean_deviations`, `a_period_of_six_or_less_never_reads_above_two_hundred` |
| 18 | a refused candle still replaces the last `T` (the check after the move) | `a_corrupt_candle_is_refused_and_changes_nothing` |
| 19 | `T` reads the timestamp (one paisa for a negative stamp) | `a_time_gap_reads_as_contiguous_candles` |
| 20 | `T` reads the open (`+ open - close`) | `a_time_gap_reads_as_contiguous_candles` |
| 21 | `T` summed in `i64`, wrapping | `the_largest_prices_and_period_never_overflow` |
| 22 | the fold reads a counter later folds share (a static) | 17, among them `no_reading_depends_on_a_later_candle` |
| 23 | the ring reserved with no capacity (`VecDeque::new()`) | `a_zero_period_is_refused_and_every_other_period_is_accepted`, `the_ring_never_grows_after_new` |
| 24 | a `[i128; 4]` field added to `Cci` | `the_state_is_a_fixed_size` |
| 25 | a round times one sample too many | `the_bench_measures_every_row_at_tiny_sizes` |
| 26 | p99 read at rank 98 | `a_cost_is_read_by_rank_and_judged_flat_or_not` |
| 27 | a zero p99 passes the verdict | `a_cost_is_read_by_rank_and_judged_flat_or_not` |

Hand change 22 stands in for look-ahead: a streaming `fold` is never given a later candle, so the change that lets a
state at `t` depend on anything after `t` is shared state that later folds update; the test turns red on it.

## Coverage: Route B on the pinned nightly

**Set** (recipe step 2 against START; reading `narrow`: START's first-parent history holds no "Merge BASE-COV-" commit;
written before measuring, sha256 `f2a18366...`; `fn main` of `ratio.rs` spans 764-833 at the tip):

```
reading narrow
coverage crates/indicators/src/cci.rs whole
coverage crates/indicators/src/cci_tests.rs whole
coverage crates/indicators/benches/ratio/psrb_cci.rs whole
coverage crates/indicators/src/lib.rs line 80
coverage crates/indicators/benches/ratio.rs line 70
coverage crates/indicators/benches/ratio.rs line 71
coverage crates/indicators/benches/ratio.rs main 764-833
mutants crates/indicators/src/cci.rs whole
mutants crates/indicators/src/cci_tests.rs whole
mutants crates/indicators/benches/ratio/psrb_cci.rs whole
mutants crates/indicators/src/lib.rs line 80
mutants crates/indicators/benches/ratio.rs line 70
mutants crates/indicators/benches/ratio.rs line 71
mutants crates/indicators/benches/ratio.rs main 764-833
```

**Commands.** In a worktree of this session's own detached at `bd65eb6f` (clean), deep dir and profile folder new:
`D3_W=<scratch> D3_DIR=<scratch>/kit D3_NIGHTLY=~/.rustup/toolchains/nightly-2026-08-08-x86_64-unknown-linux-gnu
D3_LLVM_BIN=$D3_NIGHTLY/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=$D3_NIGHTLY/bin/cargo bash d3-route-b-cloud.sh
<tree> <K> <P> <L> <set file> indicators` (the kit's script, unmodified), then `d3-bench-cov.sh <tree> <K> <P> <L2>
<set file> indicators ratio` (below), same pins and environment. Route B: pins as above; build 19:51:00-19:51:05 IST,
23 artifacts, 0 reused, 21 objects; `cargo test -p indicators --locked --no-fail-fast` 19:51:05-19:55:32 IST, 529
passed, 0 failed, 21 profiles. Its exit was 3 only because the two bench files are in no test object (`benches/ratio.rs`
is `test = false`): "COVERAGE NOT READ ... NOT PROVED" for those two, by design, as for the first wave. The bench step
built the bench target in the same test profile (opt-level 0), ran it once 19:55:47-20:01:12 IST (exit 0, every row
ok), merged its profile with the 21 test profiles (22) and read every set file again with the tests' objects and the
bench executable: exit 0, every set file read.

**Figures** (`d3-covset.mjs`, the merged reading at `bd65eb6f`):

| file | item | lines | branches |
|---|---|---|---|
| `crates/indicators/src/cci.rs` | whole | **184/184** | **22/22** |
| `crates/indicators/src/cci_tests.rs` | whole | **766/766** | **14/14** |
| `crates/indicators/benches/ratio/psrb_cci.rs` | whole | **86/86** | 0/0 |
| `crates/indicators/src/lib.rs` | line 80 | 0/0 (no region: `pub mod cci;`) | 0/0 |
| `crates/indicators/benches/ratio.rs` | lines 70, 71 | 0/0, 0/0 (no region: `#[path]`, `mod`) | 0/0 |
| `crates/indicators/benches/ratio.rs` | `main` 764-833 | 24/26 | 1/2 |
| the set | | 1,060/1,062 | 37/38 |

The tests-only reading of Route B gives the same figures for `cci.rs`, `cci_tests.rs` and `lib.rs`. The 22 branch sides
of `cci.rs` are its eleven branch points, each taken both ways: the two edges of `Level::from_milli`, `period == 0`, the
full ring in `fold`, `!warm()` in `reading`, `flat()` in `milli`, the `D > 0` guard of `above` and of `below`, the
`let`-`else` of `flat_within` and the two `&&` points of its verdict (`?` is not instrumented as a branch).

**(F) items, the only uncovered lines and sides** (recipe step 4; no GAP, no (U)):

- `crates/indicators/benches/ratio.rs:828:8` branch True, `if verdicts.contains(&false) {`; `:829`
  `println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");`; `:830` `std::process::exit(1);` — `main`'s
  own failing arm, which a run reaches only when one of the file's own rows fails (CC-20's kind). These are base lines
  of `main`, in the set only because `main` holds this part's statement. The wrapper has no such arm: a refusal is
  printed on every size's line (`refusal None`) and fails the verdict through a zero p99, so every line of
  `psrb_cci.rs` runs in a passing run.

The bench half of Route B, `d3-bench-cov.sh` (this session's, not a kit file, sha256 `99e30289...`; the form the
PSRB-RSI cloud session printed in its report on 5 Nov 2026, written again here):

```bash
#!/bin/bash
# d3-bench-cov.sh (PSRB-CCI cloud session, 5 Nov 2026; shell, jq and node only, no Py-language): the bench half of
# Route B for a part whose D2 set holds bench files (PARTS.md 2.4: "The bench wrapper and `main` are measured by running
# the bench target once under the same instrumentation and merging its profile"). Not a kit file: written in this
# session after the form the PSRB-RSI cloud session used on 5 Nov 2026 (its report-PSRB-RSI.md prints its script), and
# run after the kit's d3-route-b-cloud.sh on the same tree, deep dir and profile folder:
#   1. the same pins (rustc -V, llvm-cov and llvm-profdata --version) and the same environment
#      (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', CARGO_PROFILE_DEV_OPT_LEVEL=0,
#      CARGO_TARGET_DIR=<K>/cov, RUSTC and RUSTDOC the nightly's);
#   2. `cargo test -p <pkg> --locked --bench <bench> --no-run --message-format=json` builds the bench target in the test
#      profile (opt-level 0, the profile the tests ran in); its executable is the one THIS build reports;
#   3. that executable runs ONCE with LLVM_PROFILE_FILE in a NEW folder <P>/bench-<bench>;
#   4. llvm-profdata merge -sparse of the tests' profiles (<P>/<pkg>/profraw.list) and the bench's;
#   5. llvm-cov show of every set file of the package with the tests' objects and the bench executable, read by
#      d3-covset.mjs.
# Deletes nothing. Exit 0: the bench ran (whatever its verdict) and every set file was read; 3: NOT PROVED.
# Usage: d3-bench-cov.sh <S> <K> <P of the Route B run> <L new log folder> <set file> <pkg> <bench>
set -o pipefail
N=${D3_NIGHTLY:?}; T=${D3_LLVM_BIN:?}; CARGO_Q=${D3_CARGO:?}; D=${D3_DIR:?}
PIN_RUSTC='rustc 1.99.0-nightly (1a98b1e13 2026-08-07)'; PIN_LLVM='LLVM version 23.1.0-rust-1.99.0-nightly'
S=$1; K=$2; P=$3; L=$4; SET=$5; C=$6; B=$7
[ -n "$B" ] || { echo "usage: d3-bench-cov.sh <S> <K> <P> <L> <set file> <pkg> <bench>"; exit 2; }
ts() { TZ=Asia/Kolkata date '+%F %T IST'; }
np=0; notp() { echo "BENCH-COV NOT PROVED: $*"; np=1; }
[ -s "$P/$C/profraw.list" ] || { echo "BENCH-COV REFUSED: $P/$C/profraw.list is missing (run d3-route-b-cloud.sh first)"; exit 2; }
[ -s "$P/objects.txt" ] || { echo "BENCH-COV REFUSED: $P/objects.txt is missing"; exit 2; }
PB=$P/bench-$B
if [ -e "$PB" ] && [ "$(ls -A "$PB" | wc -l | tr -d ' ')" != 0 ]; then echo "BENCH-COV REFUSED: $PB is not empty"; exit 2; fi
mkdir -p "$PB" "$L" || exit 2
echo "BENCH-COV START $(ts) tree $S tip $(git -C "$S" rev-parse HEAD) package $C bench $B"
rv=$("$N/bin/rustc" -V 2>&1); cv=$("$T/llvm-cov" --version 2>&1 | grep 'LLVM version'); pv=$("$T/llvm-profdata" --version 2>&1 | grep 'LLVM version')
echo "versions: $rv | llvm-cov ${cv# } | llvm-profdata ${pv# }"
[ "$rv" = "$PIN_RUSTC" ] || { echo "BENCH-COV ENVIRONMENT: rustc is '$rv', pin '$PIN_RUSTC'"; exit 3; }
case "$cv" in *"$PIN_LLVM"*) ;; *) echo "BENCH-COV ENVIRONMENT: llvm-cov '$cv'"; exit 3;; esac
case "$pv" in *"$PIN_LLVM"*) ;; *) echo "BENCH-COV ENVIRONMENT: llvm-profdata '$pv'"; exit 3;; esac
export PATH=$N/bin:$PATH RUSTC=$N/bin/rustc RUSTDOC=$N/bin/rustdoc
export CARGO_TARGET_DIR=$K/cov RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' CARGO_PROFILE_DEV_OPT_LEVEL=0
cd "$S" || exit 2
t0=$(date +%s); echo "== bench build START $(ts): cargo test -p $C --locked --bench $B --no-run --message-format=json"
mkdir -p "$P/bench-build-$B" || exit 2
LLVM_PROFILE_FILE=$P/bench-build-$B/%p-%m.profraw "$CARGO_Q" test -p "$C" --locked --bench "$B" --no-run --message-format=json > "$L/bench-build.json" 2> "$L/bench-build.err"; rc=$?
echo "== bench build rc=$rc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min)"
[ $rc = 0 ] || { tail -5 "$L/bench-build.err"; echo "BENCH-COV NOT PROVED: the instrumented bench build failed"; exit 3; }
EXE=$(jq -r --arg b "$B" 'select(.executable != null and .target.name == $b and (.target.kind | index("bench"))) | .executable' "$L/bench-build.json" | sort -u)
[ "$(printf '%s\n' "$EXE" | grep -c .)" = 1 ] || { echo "BENCH-COV NOT PROVED: the build reported '$EXE' as the bench executable"; exit 3; }
echo "bench executable: $EXE (from this build's own report)"
t0=$(date +%s); echo "== bench run START $(ts): LLVM_PROFILE_FILE=$PB/%p-%m.profraw $EXE"
LLVM_PROFILE_FILE=$PB/%p-%m.profraw "$EXE" > "$L/bench-run.log" 2>&1; brc=$?
echo "== bench run rc=$brc END $(ts) ($(( ($(date +%s) - t0) / 60 )) min) (rc 1 is a breach under instrumentation: a verdict, not a coverage matter)"
find "$PB" -name '*.profraw' > "$PB/profraw.list"; nb=$(wc -l < "$PB/profraw.list" | tr -d ' '); echo "bench profraw files: $nb"
[ "$nb" -gt 0 ] || { echo "BENCH-COV NOT PROVED: the bench wrote no profile"; exit 3; }
cat "$P/$C/profraw.list" "$PB/profraw.list" > "$PB/all.list"
"$T/llvm-profdata" merge -sparse -f "$PB/all.list" -o "$PB/merged.profdata" > "$L/merge-all.log" 2>&1 || { echo "BENCH-COV NOT PROVED: merge failed ($L/merge-all.log)"; exit 3; }
echo "merged $(wc -l < "$PB/all.list" | tr -d ' ') profiles (tests and bench) into $PB/merged.profdata"
OBJ=("$(head -1 "$P/objects.txt")"); while IFS= read -r o; do OBJ+=("--object=$o"); done < <(tail -n +2 "$P/objects.txt"); OBJ+=("--object=$EXE")
files=$(awk -v c="crates/$C/" '$1 == "coverage" && index($2, c) == 1 {print $2}' "$SET" | sort -u)
for f in $files; do
  b=$(printf '%s' "$f" | tr '/' '_')
  "$T/llvm-cov" show --show-branches=count --show-line-counts-or-regions --path-equivalence="$S/crates/$C/src/..,$S/crates/$C" --instr-profile="$PB/merged.profdata" "${OBJ[@]}" "$S/$f" > "$L/show-all-$b.txt" 2> "$L/show-all-$b.err"
  node "$D/d3-covset.mjs" "$SET" "$f" "$L/show-all-$b.txt" | tee "$L/figures-all-$b.txt"; [ "${PIPESTATUS[0]}" = 0 ] || notp "$f: not read ($L/show-all-$b.err)"
done
if [ $np = 0 ]; then echo "BENCH-COV DONE $(ts): every set file of $C read with the tests' and the bench's profiles"; exit 0; fi
echo "BENCH-COV DONE $(ts) WITH NOT PROVED lines above"; exit 3
```

(The block above is the run copy byte for byte: `sha256sum` of it, extracted, prints `99e30289...`.)

## Benches (advisory; the Mac's gate 8 run decides)

Machine as above (4 vCPU Intel Xeon @ 2.10 GHz, rustc 1.97.1), command `cargo bench -p indicators --bench ratio --locked`
(the bench profile: release, `lto = "fat"`), the machine otherwise idle. Each PSRB-CCI sample holds two clock reads, so
its figures are upper bounds on the step. Both runs exit 0, every row of the file within its ceiling and budget.

Run 1, 14:16:35-14:16:59 UTC, HEAD `c8e89ddc` (its `.rs` files are the code tip's: `git diff --stat c8e89ddc bd65eb6f
-- '*.rs'` is empty), load average 0.46. Its figures are the ones `docs/06-limits.md` L-PSRB-CCI-04 records. Printed
lines, whole:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 680 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           882084 ps = 1297.182 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           866014 ps ->   866388 ps   ratio 1.000x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          866014 ps ->   867328 ps   ratio 1.001x  ok
  C-I-02 step: a wandering candle -> a motionless one          880204 ps ->   618428 ps   ratio 0.702x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   880204 ps ->   833314 ps   ratio 0.946x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  8564 ps over   2 iteration(s) = 4282 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          29207 ps over   5 iteration(s) = 5841 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             27110 ps over   5 iteration(s) = 5422 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         35571 ps over   6 iteration(s) = 5928 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          879885 ps ->   775109 ps   ratio 0.880x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1474229 ps ->  1631597 ps   ratio 1.106x  ok
  the stable floor is 623 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1517548 ps = 2435.871 floors, budget 5200   ok
  PSRB-CCI: 2 id(s) for its 2 rows; the machine's floor is 680 ps; budget 2000 ns a step
C-PSRB-CCI-01 history 2000, period 20: best of 9 rounds p50 127 ns, p99 196 ns, max 18557 ns, n 20000; median round p99 222 ns; worst round p99 233 ns; refusal None
C-PSRB-CCI-01 history 20000, period 20: best of 9 rounds p50 125 ns, p99 188 ns, max 17255 ns, n 20000; median round p99 209 ns; worst round p99 222 ns; refusal None
C-PSRB-CCI-01 history 200000, period 20: best of 9 rounds p50 127 ns, p99 205 ns, max 21789 ns, n 20000; median round p99 220 ns; worst round p99 238 ns; refusal None
C-PSRB-CCI-01: best-round p99 against the first size 1.000x -> 0.959x -> 1.045x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-CCI-02 fold alone, period 20, history 20000: best of 9 rounds p50 35 ns, p99 37 ns, max 11349 ns, n 20000; median round p99 37 ns; worst round p99 49 ns; refusal None
C-PSRB-CCI-02 fold alone, period 200, history 20000: best of 9 rounds p50 32 ns, p99 34 ns, max 10007 ns, n 20000; median round p99 42 ns; worst round p99 49 ns; refusal None
C-PSRB-CCI-02 fold alone, period 2000, history 20000: best of 9 rounds p50 35 ns, p99 38 ns, max 195 ns, n 20000; median round p99 49 ns; worst round p99 51 ns; refusal None
C-PSRB-CCI-02: best-round p99 against the first size 1.000x -> 0.918x -> 1.027x; ceiling 3.000x, budget 2000 ns   ok
  (unjudged) reading at period 20, history 20000: best of 9 rounds p50 126 ns, p99 188 ns, max 13831 ns, n 20000; median round p99 212 ns; worst round p99 244 ns; refusal None
  (unjudged) reading at period 200, history 20000: best of 9 rounds p50 491 ns, p99 809 ns, max 54797 ns, n 20000; median round p99 944 ns; worst round p99 1231 ns; refusal None
  (unjudged) reading at period 2000, history 20000: best of 9 rounds p50 4096 ns, p99 7225 ns, max 359478 ns, n 20000; median round p99 10822 ns; worst round p99 12105 ns; refusal None
  reading at periods 20, 200, 2000: O(period) by design, L-PSRB-CCI-02, not judged flat: best-round p99 188 ns, 809 ns, 7225 ns
all ratios within the ceiling
```

Run 2, 14:38:14-14:38:36 UTC, HEAD `bd65eb6f` (the code tip), load average 1.20 (after the gate runs). Printed lines,
whole:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 526 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                           858638 ps = 1632.391 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in           854917 ps ->   903187 ps   ratio 1.056x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in          854917 ps ->   863009 ps   ratio 1.009x  ok
  C-I-02 step: a wandering candle -> a motionless one          865187 ps ->   583457 ps   ratio 0.674x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle   865187 ps ->   829884 ps   ratio 0.959x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  8585 ps over   2 iteration(s) = 4292 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          25900 ps over   5 iteration(s) = 5180 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             25892 ps over   5 iteration(s) = 5178 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         31953 ps over   6 iteration(s) = 5325 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover          879474 ps ->   714717 ps   ratio 0.812x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1456790 ps ->  1591408 ps   ratio 1.092x  ok
  the stable floor is 574 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1494825 ps = 2604.224 floors, budget 5200   ok
  PSRB-CCI: 2 id(s) for its 2 rows; the machine's floor is 526 ps; budget 2000 ns a step
C-PSRB-CCI-01 history 2000, period 20: best of 9 rounds p50 125 ns, p99 188 ns, max 14695 ns, n 20000; median round p99 213 ns; worst round p99 221 ns; refusal None
C-PSRB-CCI-01 history 20000, period 20: best of 9 rounds p50 125 ns, p99 185 ns, max 14214 ns, n 20000; median round p99 189 ns; worst round p99 219 ns; refusal None
C-PSRB-CCI-01 history 200000, period 20: best of 9 rounds p50 126 ns, p99 189 ns, max 15559 ns, n 20000; median round p99 204 ns; worst round p99 233 ns; refusal None
C-PSRB-CCI-01: best-round p99 against the first size 1.000x -> 0.984x -> 1.005x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-CCI-02 fold alone, period 20, history 20000: best of 9 rounds p50 32 ns, p99 35 ns, max 50 ns, n 20000; median round p99 38 ns; worst round p99 38 ns; refusal None
C-PSRB-CCI-02 fold alone, period 200, history 20000: best of 9 rounds p50 35 ns, p99 37 ns, max 10679 ns, n 20000; median round p99 38 ns; worst round p99 38 ns; refusal None
C-PSRB-CCI-02 fold alone, period 2000, history 20000: best of 9 rounds p50 32 ns, p99 34 ns, max 10150 ns, n 20000; median round p99 48 ns; worst round p99 51 ns; refusal None
C-PSRB-CCI-02: best-round p99 against the first size 1.000x -> 1.057x -> 0.971x; ceiling 3.000x, budget 2000 ns   ok
  (unjudged) reading at period 20, history 20000: best of 9 rounds p50 139 ns, p99 211 ns, max 17431 ns, n 20000; median round p99 224 ns; worst round p99 241 ns; refusal None
  (unjudged) reading at period 200, history 20000: best of 9 rounds p50 476 ns, p99 789 ns, max 27324 ns, n 20000; median round p99 858 ns; worst round p99 913 ns; refusal None
  (unjudged) reading at period 2000, history 20000: best of 9 rounds p50 3712 ns, p99 4981 ns, max 35510 ns, n 20000; median round p99 6443 ns; worst round p99 7089 ns; refusal None
  reading at periods 20, 200, 2000: O(period) by design, L-PSRB-CCI-02, not judged flat: best-round p99 211 ns, 789 ns, 4981 ns
all ratios within the ceiling
```

The coverage step's run of the same target at `bd65eb6f` (instrumented, opt-level 0, 19:55:47-20:01:12 IST, exit 0) is
a coverage run, not a cost figure; its PSRB-CCI lines, for completeness: C-PSRB-CCI-01 best-round p99 818, 740, 767 ns
(ok); C-PSRB-CCI-02 108, 118, 117 ns (ok); the reading line 743, 6,542, 55,445 ns.

What the figures say, plainly: the step at the confirmed period (fold, a 20-value reading and two comparisons) is about
190 ns at p99 here whatever the history; the fold alone is about 35 ns at every period; and a reading grows with the
period, about 0.2, 0.8 and 5 to 7 µs at periods 20, 200 and 2,000, which is L-PSRB-CCI-02's O(period), accepted by
D-8653 and on record, unjudged.

## Decisions drafted (docs/05, this part's block only): D-8650 to D-8655; D-8656 to D-8659 unused

- D-8650: CCI as `indicators::cci`, the published form on `hlc3` with the mean deviation from the same SMA, the current
  candle included; runtime period 1..=65,535; the first reading after `period` candles; no vocabulary position; carried
  across sessions.
- D-8651: thirds and n-multiples, the identity `CCI = 200 n dev / (3 D)` with its one-line derivation, exact comparisons,
  `milli` the floor for records, `Level` in thousandths with the chosen +/-10,000-point domain, the const-asserted
  bounds.
- D-8652: the window of one typical price (`D == 0`) a named state with no value and every comparison false
  (`TradingView`'s `na`, UNVERIFIED); the ceiling `|CCI| <= 100 n / 3` with its proof, and what follows (period 6 or
  less never above +200, so PSRB-STRAT's searched periods for a +/-200 level start at 7; period 1 always `D = 0`;
  period 2 +/-66.666...; `milli` an `i64`).
- D-8653: `fold` O(1), `reading()` O(period) by design, the one non-O(1) reading of the PSRB parts (L-PSRB-CCI-02); the
  order-statistic tree named and not built; accepted (the design's decision, default accept) and to be reported to the
  operator, who may reverse it; C-PSRB-CCI-02 judges `fold` alone and the unjudged line prints the reading's slope.
- D-8654: refusals and their order; `unwrap_or(0)` where the ring's `Option` cannot be `None`; no timestamp, open,
  volume or open interest read; no look-ahead; the ring allocated once in `new` (a clone reserves only its length; the
  measured path never clones).
- D-8655: C-PSRB-CCI-01 and -02, their sizes, 9 rounds of 20,000, the 3.0x ceiling and 2 µs budget, the unjudged
  reading line, the helpers copied per part, gate 14's cover-table row left to the main session.

Ledger rows: PSRB-CCI-I01 to I06, C-PSRB-CCI-01 and -02 (docs/04, each proof cell in gate 10's path-qualified form);
L-PSRB-CCI-01 to 04 (docs/06, L-PSRB-CCI-04 with run 1's figures); one docs/07 row.

## Anything not done, and notes for the Mac and the operator

- **Mutants: deferred** (operator, 30 Oct; CLOUD.md rule 6). The set file above carries the mutants lines for the
  background pass. One item that pass will meet, said now: `Reading::above` and `Reading::below` read
  `D > 0 && 200_000 n dev > 3 m D` (and `<`), the form PARTS-WILLR-CCI.md 6.3 states. When `D == 0`, `dev` is 0 too
  (`D >= 2 |dev|`), so both products are 0 and the bare comparison is false as well: the guard is redundant in value,
  and a mutant that widens it to `D >= 0` is equivalent (no test can kill it, in `above` and in `below`). Removing the
  guard is a one-word change with the same values everywhere (the flat test and the parity test pin them); it was kept
  here because 6.3 writes it and the brief says to follow the formulas to the letter. The same holds for PSRB-WILLR's
  `R > 0` guard (5.3) by the same argument (`0 <= U <= R`). The main session or the operator decides.
- **D-8653 goes to the operator**: `reading()` is O(period) (about 0.2 µs at period 20 here, 5-7 µs at 2,000, advisory),
  accepted by the design's default; the operator may reverse it before PSRB-STRAT's first run.
- The definition is the published one, UNVERIFIED against `TradingView`'s own help page and output (PARTS-WILLR-CCI.md
  8); so is `na` on a window of one typical price. PSRB-LAW's charter rows should add the CCI page.
- Gates 10, 11, 12 and 27b are red on START with the same refusals as on this tip (section "CI gates ... run here");
  they are not this part's and nothing of this part appears in them.
- No real candle has passed through the module (no store, no vendor data, as CLOUD.md rule 2 says); every series in the
  tests and the bench is invented.

## NEEDS THE MAC (PARTS-WILLR-CCI.md 6.7, as 5.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against START = PSRB-SEAM-2 (`deaa0f5c0606`), with
   `benches/ratio.rs` `main` and `benches/ratio/psrb_cci.rs` measured by the instrumented bench run (this session's set
   file, commands and figures are above; the cloud figure is the builder's).
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-CCI rows and the unjudged reading line printed
   whole (the cloud runs are advisory).
3. Gates 10, 11, 12 and 14. Gate 14's cover-table row for `C-PSRB-CCI-01` and `C-PSRB-CCI-02` is the main session's at
   the family assembly (ten C-PSRB ids in all, PARTS-WILLR-CCI.md 3.6). Run here as CI runs them, gates 10, 11, 12 and
   27b are red on START and on the tip with the same refusals, none naming a PSRB-CCI file or id; gates 0, 10b, 14, 25
   and 27 are green.
4. macOS rows: none expected (no platform code, no file system, no clock but `Instant` in the measuring functions).
5. `XW/anchors2-check.sh` re-run against the then-current first-wave tips (PARTS-WILLR-CCI.md 7.3).
6. The anchors on the then-current `feat/db-fix` (the `lib.rs` slot, the two bench slots, the four ledger headings).
7. Mutants are deferred (operator, 30 Oct): the set file's mutants lines are above for the background pass, with the
   equivalent-mutant note on the `D > 0` guard.

STATUS: DONE

# PSRB-WILLR: Williams %R in exact integers (`indicators::williams_r`)

Session PSRB-WILLR of the 5 Nov 2026 PSRB second wave, TOKEN `PSRB-WILLR-20261105-1`, branch
`cloud/psrb-willr-20261105` cut from `origin/cloud/psrb-kit-20261105` at `fdc091ba69c9b632e4cd80388ff8b6d5ee681682`.
**Code tip** (`git rev-list -1 HEAD -- . ':(exclude)reports'`): `a910ba9272a726c20ac7de2643ca4ad298834a14`.
`part-PSRB-WILLR.md` is what the Mac's check reads first; `result-PSRB-WILLR.json.md` is the machine-readable record.

Commits, in order (every push preceded by `git fetch origin cloud/psrb-willr-20261105`, whose OWNER file held this
session's token each time; no push was refused; nothing was forced, merged or rebased; no pull request):

| commit | what |
|---|---|
| `d7621461` | `reports/psrb-cloud-20261105/OWNER-PSRB-WILLR.md` holding the TOKEN (CLOUD.md rule 4) |
| `bc47c963` | this report, line 1 `STATUS: IN PROGRESS`: the kit check and the START baseline |
| `237a6952` | the code: `williams_r.rs`, `williams_r_tests.rs`, `benches/ratio/psrb_willr.rs`, the `lib.rs` line, the two `ratio.rs` insertions |
| `e836b47c` | the four ledger blocks at the PARTS-WILLR-CCI.md 3.4 anchors |
| `a910ba92` | L-PSRB-WILLR-03 cites all three cloud bench runs (`docs/06` only; `crates/` byte-identical to `e836b47c`) |
| (this commit) | `report-PSRB-WILLR.md` line 1 `STATUS: DONE`, `part-PSRB-WILLR.md`, `result-PSRB-WILLR.json.md` (`reports/` only) |

## Kit check

- START = `git rev-parse origin/cloud/psrb-kit-20261105~1` = `deaa0f5c0606464f2ec79dc9e94de03792ece326` (PSRB-SEAM-2),
  whose parent is PSRB-SEAM `35234cead4009299a93fcd79a3d1fffd773dcc16`, whose parent is
  `1ad6e24ef8e05e8836e511873e8352071dd98fa0`: the chain the brief names.
- `git diff --name-only $START origin/cloud/psrb-kit-20261105` lists only the 13 files of
  `reports/psrb-cloud-20261105/`.
- Every MANIFEST row checked: `CLOUD.md`, `PARTS.md` and `PARTS-WILLR-CCI.md` hashed whole, the nine fenced files
  extracted with MANIFEST's own `awk` line and hashed: all 12 sha256 equal the MANIFEST column.
- Decision-id clash grep on docs/05 (PARTS-WILLR-CCI.md 2.1): `grep -nE '^#{2,4} +D-864[0-9]'` and
  `grep -nE '^#{2,4} +D-865[0-9]'` printed nothing at START.
- The four ledger anchors of 3.4 sit at the stated lines at START: docs/04 :9038, docs/05 :86873, docs/06 :22007,
  docs/07 :1084.

## START baseline (before the first code commit)

The tree at the OWNER commit held START's code exactly (`git diff --stat $START HEAD -- . ':(exclude)reports'` printed
nothing), so this run is START's: `CARGO_TARGET_DIR=/home/user/tgt/stable TMPDIR=/dev/shm/tmp-willr cargo test -p
indicators --locked` on `rustc 1.97.1 (8bab26f4f 2026-07-14)`: EXIT 0, 504 passed, 0 failed, 0 ignored. No host red
at START, and none since: every command below is green here, so none needed the START or non-root proof.

## Toolchains and machine

- Stable: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, the repository's pin
  (`rust-toolchain.toml`).
- Coverage nightly: installed here by `rustup toolchain install nightly-2026-08-08 --profile minimal --component
  llvm-tools-preview --no-self-update`; `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; its own
  `llvm-cov --version` and `llvm-profdata --version` print `LLVM version 23.1.0-rust-1.99.0-nightly`. No
  `RUSTC_BOOTSTRAP`.
- Machine: Linux 6.18.44 x86_64 KVM guest, 4 vCPU `Intel(R) Xeon(R) Processor @ 2.80GHz`, 15 GiB memory; TMPDIR on
  tmpfs (`/dev/shm/tmp-willr`); one `CARGO_TARGET_DIR` per toolchain and purpose (`/home/user/tgt/stable`,
  `/home/user/tgt/stable-red` for the hand changes, `/home/user/tgt/deep-1/cov` for Route B).

## What was built (PARTS-WILLR-CCI.md 5)

`indicators::williams_r`: Williams %R over the last `period` candles, the current one included, in exact integers.
`%R = -100 (HH - C) / (HH - LL)`, in `-100..=0`; a window whose highest high equals its lowest low (a window of one
price) has no value: `Reading::flat()` is true, `ppm()` is `None`, every comparison and crossing is false.

- **State** (`WilliamsR`, `Clone, Debug, PartialEq, Eq`, not `Copy`): `period: u16`; `highs` and `lows`,
  `VecDeque<(u64, i64)>` of (candle number, price), each `VecDeque::with_capacity(period)` in `new`; `next: u64` (the
  wrapping candle number); `seen: u32` (the saturating warm count); `close: i64`. `size_of::<WilliamsR>() <= 128` is
  const-asserted.
- **`fold`**: `check_evaluable` refused first (`Refusal::Corrupt`), then per deque one `if` that expires the front when
  `s.wrapping_sub(t) >= n`, then the `while ...back().is_some_and(...)` back-pops and one push, then
  `next = s.wrapping_add(1)`, `seen = seen.saturating_add(1)`, `close = c.close`.
- **`reading`**: `None` until `warm()` (`seen >= u32::from(period)`); else the fronts read with `map_or(close, ...)`
  (a doc comment says why the default never applies: the fold that made the state warm pushed onto both deques, and a
  fold that pops pushes before it returns).
- **`Reading`** (`Copy`): `period`, `highest`, `lowest`, `close`, `flat`; `ppm()` =
  `i32::try_from((-10^6 U).div_euclid(R)).ok()`; `above(l)` = `10^6 U < -l R`, `below(l)` = `10^6 U > -l R`, in
  `i128`; `crossed_above(previous, l)` = same period, previous not of one price, previous not above, self above
  (`ta.crossover`); `crossed_below` the mirror (`ta.crossunder`).
- **Bounds**, const-asserted with `checked_mul`: `SPAN_LIMIT = 2^63` (`U, R < 2^63`), `UNIT_LIMIT = 2^20` (`10^6` and
  every `|l|`), `SPAN_LIMIT x UNIT_LIMIT = PRODUCT_LIMIT = 2^83`, and twice that still fits.
- **Refusals** (5.4): `PeriodZero` (`new(0)`), `LevelOutOfRange { ppm }` (`Level::from_ppm` below -1,000,000 or above
  0), `Corrupt(why)`; each leaves the state unchanged.
- **Bench half** in the module: `StepCost`, `summarize`, `flat_within`, `measure_step` (period 14), `measure_period`
  (history 20,000), the private `measure`, `bench_candle` and `hashed`; the wrapper `benches/ratio/psrb_willr.rs` sorts,
  prints and judges (`row`, `one_step_however_long_the_history`, `one_step_whatever_the_period`, `best_round`,
  `verdict`).
- **Gate-12 wording** (PARTS-WILLR-CCI.md 2): no doc block says "flat window" or "flat state"; they say "a window of one
  price" or link [`Reading::flat`]. The one cost claim, the module doc's "amortized O(1)", sits in the block that names
  `C-PSRB-WILLR-01`, `C-PSRB-WILLR-02` and `crates/indicators/benches/ratio.rs`; the module doc also says a deque never
  passes `period`, a state built by `new` never reallocates, and "a clone reserves only its length; the measured path
  never clones".

### Files and functions

| file | lines | items |
|---|---|---|
| `crates/indicators/src/williams_r.rs` | new, 576 | `MIN_LEVEL_PPM`, `MAX_LEVEL_PPM`, `Refusal`, `Level` (`from_ppm`, `ppm`), `WilliamsR` (`new`, `period`, `fold`, `warm`, `reading`), `Reading` (`period`, `highest`, `lowest`, `close`, `flat`, `under`, `range`, `ppm`, `above`, `below`, `crossed_above`, `crossed_below`), `StepCost`, `summarize`, `nearest_rank`, `flat_within`, `measure_step`, `measure_period`, `measure`, `bench_candle`, `hashed`; consts `PPM`, `SPAN_LIMIT`, `UNIT_LIMIT`, `PRODUCT_LIMIT`, `STEP_PERIOD`, `PERIOD_HISTORY`, `BENCH_LEVEL` |
| `crates/indicators/src/williams_r_tests.rs` | new, 1,076 | the 28 tests below and their helpers (`Xorshift`, `candle`, `willr`, `level`, `fold_all`, `readings`, `last`, `scan`, `published`, `published_sides`, `random_bars`, `staircase`, `expiring`, `Pops`, `staircase_pops`, `assert_window`) |
| `crates/indicators/benches/ratio/psrb_willr.rs` | new, 135 | `row`, `one_step_however_long_the_history`, `one_step_whatever_the_period`, `best_round`, `verdict` |
| `crates/indicators/src/lib.rs` | +1 (78) | `pub mod williams_r;` under `// slot PSRB-WILLR (williams_r)` |
| `crates/indicators/benches/ratio.rs` | +2 (68-69), +8 (788-795) | the bench module under `// slot bench-mod PSRB-WILLR`; the row statement under `    // slot bench-row PSRB-WILLR` |
| `docs/04-invariants.md` | +13 | PSRB-WILLR-I01..I06, C-PSRB-WILLR-01, C-PSRB-WILLR-02 |
| `docs/05-decisions.md` | +216 | D-8640..D-8644 |
| `docs/06-limits.md` | +9 | L-PSRB-WILLR-01..04 |
| `docs/07-plan.md` | +7 | one row, the block starting with a blank line |

### Where this part departs from the letter of PARTS-WILLR-CCI.md, and why (for the Mac to accept or send back)

1. **No separate `R > 0` conjunct in `above` and `below`.** 5.3 writes `R > 0 && 10^6 x U < (-l) x R`. With
   `0 <= U <= R`, `R = 0` forces `U = 0`, so both products are 0 and the strict comparison is already false: the
   conjunct changes no result at any reachable state, and written it would be a branch no test can tell from its
   absence (a survivor for the deferred mutants pass). The window of one price still compares false
   (`a_flat_window_has_no_value_and_compares_false`); D-8641 records it. `!previous.flat()` in the crossings is not
   redundant and stays.
2. **The bench's candles are hashed, not strided** (D-8644). A stride of 7,919 through 4,001 prices is a falling
   sawtooth (7,919 is -83 modulo 4,001) whose jumps empty the highs deque about once in 48 candles, inside the p99
   (derived, not measured). The first probe of this part's own tiny-size bench test on that series failed its
   assertion that the lows deque held more than one entry after 9,999 candles; on the hashed series each deque reaches
   9 entries at period 14 and pops on 5,278 and 5,420 of 10,000 folds.
3. **rustfmt wraps the row statement into 8 lines, not 5.** The statement is exactly 5.1's;
   `psrb_willr::row(floor, &["C-PSRB-WILLR-01", "C-PSRB-WILLR-02"])` is 62 characters, past rustfmt's default
   60-character call width (no `rustfmt.toml` in the tree), where RSI's was 57. Longest added line 52 characters.
4. `StepCost` also derives `Default`, as PSRB-RSI's does (the wrapper's `unwrap_or_default`).

## Tests (`crates/indicators/src/williams_r_tests.rs`, every name of 5.5)

1. `a_zero_period_is_refused_and_every_other_period_is_accepted` (0 refused; 1, 14, 65,535 accepted, empty, each
   deque's capacity at least `period`).
2. `a_level_outside_minus_one_hundred_to_zero_is_refused_at_both_edges` (accepts -1,000,000, 0, -850,000, -150,000 and
   reads each back; refuses 1, `i32::MAX`, -1,000,001, `i32::MIN` naming the value).
3. `the_first_reading_appears_after_exactly_period_candles` (periods 1, 2, 3, 14, 65,535; the window is the scan's).
4. `williams_r_matches_fixture_wr_1` (every column of the table at every candle: HH, LL, U, R, `ppm`, the exact %R as a
   fraction, above -85, below -15, both crossings; candle 2 has no previous reading).
5. `a_tie_at_the_level_is_neither_above_nor_below` (WR-1 candle 5: -850,000 neither; -850,001 above; -849,999 below).
6. `a_floored_ppm_equal_to_the_level_is_not_a_tie` (period 1, (2,000,001, 1,000,000, 1,150,001): U 850,000,
   R 1,000,001, `ppm` -850,000, above -850,000 and not below; the two products restated).
7. `a_flat_window_has_no_value_and_compares_false` (WR-2: `flat()`, `ppm` None, every comparison false at -1,000,000,
   -850,000, -150,000 and 0; the fourth candle reads 0; the fifth U 2, R 2, -1,000,000, below -150,000).
8. `a_period_of_one_reads_the_candle_alone` (-500,000, None, 0, -1,000,000).
9. `the_crossings_follow_tradingviews_crossover` (WR-1 above -85 at 3 and 6 only, below -15 at 4 only; WR-2 never from
   its reading of one price at four levels, below -15 at its fifth; other-period previous readings false both ways;
   random series at seven levels against the crossing on the exact rationals, more than 100 crossings each way).
10. `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen` (WR-1 candle 4, -421,053).
11. `every_reading_lies_between_minus_one_hundred_and_zero` (200 random series; more than 10,000 readings with a value;
    more than 100 closes on the high read 0 and more than 100 on the low read -1,000,000).
12. `ppm_is_the_floor_of_the_exact_ratio` (`ppm R <= -10^6 U < (ppm + 1) R` on more than 10,000 readings).
13. `the_deques_equal_a_brute_force_window` (1,000 random series, periods 1..=40: every reading equals the scanned
    window's and every comparison at -850,000, -150,000, -500,000 equals the published one; more than 50,000 warm
    readings).
14. `parity_with_the_published_definition` (302 series, both fixtures among them: `flat()`, `ppm` the rational's
    floor, and both comparisons at seven levels, one random, equal to `100 (C - HH) / (HH - LL)` as `num / den`; more
    than 8,000 warm readings).
15. `the_pops_stay_bounded_at_the_ninety_ninth_percentile` (the 5.3 staircase, periods 2,000 and 14, `next` from 0 and
    from `u64::MAX - 5`; back-pops white-box `len_before + 1 - len_after - expired`; pins every 5.5 figure and 5.3's
    p99 and removal figures; removals at most `period` on every fold; exactly `period` at every jump after the first).
16. `at_most_one_entry_expires_per_candle` (before each fold at most one entry of a deque is `period` old; after it every
    front younger than `period`, numbers rising, highs falling and lows rising strictly; random series and staircases,
    `next` from 0 and from `u64::MAX - 20`).
17. `the_deques_never_grow_past_their_period_or_reallocate` (100,000 folds at periods 1, 14, 1,000: lengths at most
    `period`, capacities unchanged, each deque reaching exactly `period`).
18. `the_candle_counter_wraps_without_changing_a_reading` (40 candles from `next = u64::MAX - 5` and from 0, periods
    1, 3, 7, 14: equal readings).
19. `the_warm_counter_saturates_and_stays_warm` (`seen = u32::MAX - 1`, two folds: `u32::MAX`, warm, readings equal to a
    copy with its true count, 22).
20. `the_largest_prices_and_period_never_overflow` (H `i64::MAX`, L 1, C 1 / `i64::MAX` / `i64::MAX - 1` at periods 1
    and 65,535: -1,000,000, 0, -1; the const bounds restated at runtime).
21. `a_price_gap_enters_the_window_and_leaves_it_after_period_candles` (a 10% higher high is HH for exactly `period`
    candles at periods 1, 3, 14; equal to the scan throughout).
22. `a_time_gap_reads_as_contiguous_candles` (a minute, a day, 7 minutes and 13 microseconds, receding: equal states).
23. `no_reading_depends_on_a_later_candle` (bars after t redrawn at six points t, periods 1, 3, 14).
24. `a_corrupt_candle_is_refused_and_changes_nothing` (five `Corrupt` kinds, `PriceNotPositive` among them, on fresh,
    one-candle, short-of-warm, warm and wrapped states; the whole state compared).
25. `the_state_is_a_fixed_size_and_allocates_only_in_new` (`size_of::<WilliamsR>() <= 128`, `Reading` at most 32 bytes
    and `Copy`; capacities unchanged through 10,000 folds at periods 1, 14, 200).
26. `the_previous_reading_judges_with_the_current_candle_excluded` (WR-1: the reading kept at candle 4 keeps window
    2..=4 after candle 5 folds; candle 5's close judged against it is -89.47..., below -85, where its own reading is a
    tie).
27. `the_bench_measures_both_rows_at_tiny_sizes` (shapes of `measure_step` and `measure_period`, `PeriodZero`, an empty
    measurement; 10,000 bench candles valid; both deques reach more than 3 entries and pop on more than 4,000 folds).
28. `a_cost_is_read_by_rank_and_judged_flat_or_not` (`summarize` by nearest rank; `flat_within` passes, breaches the
    ceiling, breaches the budget, fails on a zero first or later p99, fails on nothing).

The test file holds no `&&` or `||` operator and no one-sided `if` in its own code (its one `while` takes both sides), so its own branches read covered (6/6).

## Gates (stable 1.97.1, at the code tip `a910ba92`)

| command | result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy -p indicators --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test -p indicators --locked` | exit 0: 532 passed, 0 failed, 0 ignored (START's 504 and this part's 28) |
| `cargo test -p core --locked --test cost_invariants --test decision_citations` | exit 0: 5 passed (`cost_invariants`) and 4 passed (`decision_citations`) |
| `cargo test -p core --locked` (beyond the bar: `core`'s other tests read the ledgers too; run at `e836b47c`, whose docs/04 and docs/05 the tip holds) | exit 0: 261 passed |

Gates 10, 11 and 12 (CI only; NEEDS THE MAC), run here with the repository's own `.github` tools built by
`rustc --edition=2024 -D warnings` on 1.97.1 and the step bodies of `.github/workflows/ci.yml` copied into a scratch
script, at `e836b47c` (whose `crates/` and docs/04 the tip holds) and on a worktree at START:

- **Gate 10** refuses 2 rows at both, R427-11 (:8873) and STACK1028-03 (:9103 at START, :9116 at the tip): base rows,
  not this part's. The eight PSRB-WILLR rows are read and refused for nothing.
- **Gate 11** refuses 29 file-and-rule entries at both, the same lines (`api`, `cli`, `pull` and `store` files); no
  `indicators` file is refused, and `williams_r.rs` appears in no rule's hits.
- **Gate 12** refuses 85 blocks at both, the same lines, none under `crates/indicators/`. This part adds one claim
  block (797 to 798), the module doc, counted under its UNVERIFIED pass (424 to 425; the gate checks UNVERIFIED first),
  and the block names `crates/indicators/benches/ratio.rs` as well.

So gates 10, 11 and 12 are red at this base for reasons outside this part, and this part adds no refusal to any of them.

## Each new test shown red under a hand change (CLOUD.md rule 6)

A node script (`red.mjs`, scratch, not tracked; sha256 `b5ea654a...`) applied 30 hand changes to `crates/indicators/src/williams_r.rs`
one at a time in a worktree of this session's own detached at the code tip `a910ba92` (clean), ran `cargo test -p
indicators --locked --lib williams_r::` on the pinned stable for each (change 29 with its own test filter and
`--test-threads=1`, change 23 with `--test-threads=1`), recorded the failed tests, and restored the file. It printed
`RESTORED byte for byte` (sha256 before and after equal), `git status --porcelain` of the worktree was empty afterwards,
every change compiled, and `TESTS 28, shown red 28`.

| # | hand change of `williams_r.rs` | tests red |
|---|---|---|
| 1 | `WilliamsR::new` accepts period 0 | `a_zero_period_is_refused_and_every_other_period_is_accepted`, `the_bench_measures_both_rows_at_tiny_sizes` |
| 2 | `Level::from_ppm` accepts 1 (`> MAX_LEVEL_PPM + 1`) | `a_level_outside_minus_one_hundred_to_zero_is_refused_at_both_edges` |
| 3 | warm one candle early (`seen + 1 >= period`) | 8: `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `a_zero_period_is_refused_and_every_other_period_is_accepted`, `parity_with_the_published_definition`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_first_reading_appears_after_exactly_period_candles`, `the_warm_counter_saturates_and_stays_warm`, `williams_r_matches_fixture_wr_1` |
| 4 | both deques expire one candle late (`> period`) | 10: `a_period_of_one_reads_the_candle_alone`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `at_most_one_entry_expires_per_candle`, `parity_with_the_published_definition`, `the_candle_counter_wraps_without_changing_a_reading`, `the_deques_equal_a_brute_force_window`, `the_crossings_follow_tradingviews_crossover`, `the_deques_never_grow_past_their_period_or_reallocate`, `the_state_is_a_fixed_size_and_allocates_only_in_new`, `williams_r_matches_fixture_wr_1` |
| 5 | `above` counts a tie (`<=`) | 8: `a_flat_window_has_no_value_and_compares_false`, `a_tie_at_the_level_is_neither_above_nor_below`, `parity_with_the_published_definition`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_largest_prices_and_period_never_overflow`, `the_previous_reading_judges_with_the_current_candle_excluded`, `williams_r_matches_fixture_wr_1` |
| 6 | `above` decides on the floored `ppm` | `a_floored_ppm_equal_to_the_level_is_not_a_tie` |
| 7 | `below` counts a tie (`>=`) | 7: `a_flat_window_has_no_value_and_compares_false`, `a_tie_at_the_level_is_neither_above_nor_below`, `parity_with_the_published_definition`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_largest_prices_and_period_never_overflow`, `the_previous_reading_judges_with_the_current_candle_excluded` |
| 8 | `crossed_above` ignores a previous reading of one price | `the_crossings_follow_tradingviews_crossover` |
| 9 | `crossed_below` ignores the period | `the_crossings_follow_tradingviews_crossover` |
| 10 | `below` made exclusive of above -85 | 6: `parity_with_the_published_definition`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_largest_prices_and_period_never_overflow`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `williams_r_matches_fixture_wr_1` |
| 11 | `ppm` loses its sign | 9: `a_floored_ppm_equal_to_the_level_is_not_a_tie`, `a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_reads_the_candle_alone`, `every_reading_lies_between_minus_one_hundred_and_zero`, `parity_with_the_published_definition`, `ppm_is_the_floor_of_the_exact_ratio`, `the_largest_prices_and_period_never_overflow`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `williams_r_matches_fixture_wr_1` |
| 12 | `ppm` truncates toward zero (`/`) | 6: `a_floored_ppm_equal_to_the_level_is_not_a_tie`, `ppm_is_the_floor_of_the_exact_ratio`, `parity_with_the_published_definition`, `the_largest_prices_and_period_never_overflow`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `williams_r_matches_fixture_wr_1` |
| 13 | `ppm` divides by `R + 1` | 8: `a_floored_ppm_equal_to_the_level_is_not_a_tie`, `a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_reads_the_candle_alone`, `every_reading_lies_between_minus_one_hundred_and_zero`, `parity_with_the_published_definition`, `ppm_is_the_floor_of_the_exact_ratio`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `williams_r_matches_fixture_wr_1` |
| 14 | `lows` back-pops the wrong way (`<=`) | 15: `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `a_flat_window_has_no_value_and_compares_false`, `a_tie_at_the_level_is_neither_above_nor_below`, `at_most_one_entry_expires_per_candle`, `parity_with_the_published_definition`, `ppm_is_the_floor_of_the_exact_ratio`, `the_candle_counter_wraps_without_changing_a_reading`, `every_reading_lies_between_minus_one_hundred_and_zero`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_first_reading_appears_after_exactly_period_candles`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `the_previous_reading_judges_with_the_current_candle_excluded`, `williams_r_matches_fixture_wr_1`, `the_pops_stay_bounded_at_the_ninety_ninth_percentile` |
| 15 | `highs` never expires | 9: `a_period_of_one_reads_the_candle_alone`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `at_most_one_entry_expires_per_candle`, `parity_with_the_published_definition`, `the_candle_counter_wraps_without_changing_a_reading`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_deques_never_grow_past_their_period_or_reallocate`, `the_state_is_a_fixed_size_and_allocates_only_in_new` |
| 16 | `highs` keeps equal highs (`<`) | `at_most_one_entry_expires_per_candle`, `the_candle_counter_wraps_without_changing_a_reading` |
| 17 | `lows` never expires | 12: `a_period_of_one_reads_the_candle_alone`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `at_most_one_entry_expires_per_candle`, `parity_with_the_published_definition`, `the_crossings_follow_tradingviews_crossover`, `the_candle_counter_wraps_without_changing_a_reading`, `the_deques_equal_a_brute_force_window`, `the_deques_never_grow_past_their_period_or_reallocate`, `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`, `the_previous_reading_judges_with_the_current_candle_excluded`, `the_state_is_a_fixed_size_and_allocates_only_in_new`, `williams_r_matches_fixture_wr_1` |
| 18 | ages read with `saturating_sub`, not wrapping | `at_most_one_entry_expires_per_candle`, `the_candle_counter_wraps_without_changing_a_reading` |
| 19 | the warm count wraps (`wrapping_add`) | `the_warm_counter_saturates_and_stays_warm` |
| 20 | `ppm` computed in a wrapping `i64` | `the_largest_prices_and_period_never_overflow` |
| 21 | `highs` expires one candle late (`> period`, highs only) | 9: `a_period_of_one_reads_the_candle_alone`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`, `at_most_one_entry_expires_per_candle`, `parity_with_the_published_definition`, `the_candle_counter_wraps_without_changing_a_reading`, `the_crossings_follow_tradingviews_crossover`, `the_deques_never_grow_past_their_period_or_reallocate`, `the_deques_equal_a_brute_force_window`, `the_state_is_a_fixed_size_and_allocates_only_in_new` |
| 22 | the fold reads the timestamp (a receding stamp skips a number) | `a_time_gap_reads_as_contiguous_candles` |
| 23 | the fold reads a counter shared with later folds (a static; `--test-threads=1`) | 13: `a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_reads_the_candle_alone`, `a_tie_at_the_level_is_neither_above_nor_below`, `a_time_gap_reads_as_contiguous_candles`, `every_reading_lies_between_minus_one_hundred_and_zero`, `parity_with_the_published_definition`, `ppm_is_the_floor_of_the_exact_ratio`, `the_crossings_follow_tradingviews_crossover`, `the_deques_equal_a_brute_force_window`, `the_first_reading_appears_after_exactly_period_candles`, `the_largest_prices_and_period_never_overflow`, `the_warm_counter_saturates_and_stays_warm`, `williams_r_matches_fixture_wr_1` |
| 24 | a refused candle still advances the candle number | `a_corrupt_candle_is_refused_and_changes_nothing` |
| 25 | `#[repr(align(256))]` on `WilliamsR`, its const bound relaxed with it | `the_state_is_a_fixed_size_and_allocates_only_in_new` |
| 26 | the deques are not reserved in `new` (`VecDeque::new()`) | `a_zero_period_is_refused_and_every_other_period_is_accepted`, `the_deques_never_grow_past_their_period_or_reallocate`, `the_state_is_a_fixed_size_and_allocates_only_in_new` |
| 27 | a round times one sample too many (`0..=samples`) | `the_bench_measures_both_rows_at_tiny_sizes` |
| 28 | p99 read at rank 999 | `a_cost_is_read_by_rank_and_judged_flat_or_not` |
| 29 | the reading takes its highest high from a static every fold raises (`fetch_max`): later candles reach it; that test alone, `--test-threads=1` | `no_reading_depends_on_a_later_candle` |
| 30 | a zero p99 passes the verdict | `a_cost_is_read_by_rank_and_judged_flat_or_not` |

Changes 23 and 29 stand in for look-ahead: a streaming `fold` is never given a later candle, so the only change
that lets a reading at t depend on anything after t is state that later folds update. Change 23 (a shared counter
added to the close) does not turn `no_reading_depends_on_a_later_candle` red: its series are 60 bars each, so the
counter's parity lines up between the runs it compares. Change 29 (the reading's highest high raised by a static
every fold raises, so the first run's later candles reach the second run's reading at t) turns it red.

## Coverage: Route B on the pinned nightly

**Set** (recipe step 2 against START; reading `narrow`: START's first-parent history holds no "Merge BASE-COV-" commit,
`git log --first-parent $START | grep -c 'Merge BASE-COV-'` prints 0; set file sha256 `e191c571...`). The touched
`.rs` files against START are the three new files (whole), `lib.rs` (one added line outside a function) and
`benches/ratio.rs` (two added lines outside a function, and eight inside `fn main`, so `main` whole, 764-836):

```
reading narrow
coverage crates/indicators/src/williams_r.rs whole
coverage crates/indicators/src/williams_r_tests.rs whole
coverage crates/indicators/benches/ratio/psrb_willr.rs whole
coverage crates/indicators/src/lib.rs line 78
coverage crates/indicators/benches/ratio.rs line 68
coverage crates/indicators/benches/ratio.rs line 69
coverage crates/indicators/benches/ratio.rs main 764-836
mutants crates/indicators/src/williams_r.rs whole
mutants crates/indicators/src/williams_r_tests.rs whole
mutants crates/indicators/benches/ratio/psrb_willr.rs whole
mutants crates/indicators/src/lib.rs line 78
mutants crates/indicators/benches/ratio.rs line 68
mutants crates/indicators/benches/ratio.rs line 69
mutants crates/indicators/benches/ratio.rs main 764-836
```

**Commands.** In a worktree of this session's own detached at the code tip `a910ba92` (`git status --porcelain`
empty), deep dir `/home/user/tgt/deep-1` and profile folder new:
`D3_W=<scratch> D3_DIR=<scratch>/kit D3_NIGHTLY=~/.rustup/toolchains/nightly-2026-08-08-x86_64-unknown-linux-gnu
D3_LLVM_BIN=$D3_NIGHTLY/lib/rustlib/x86_64-unknown-linux-gnu/bin D3_CARGO=$D3_NIGHTLY/bin/cargo bash
d3-route-b-cloud.sh <tree> <K> <P> <L> <set file> indicators`, the kit's script unmodified (sha256 `f032b525...`, the
MANIFEST row), reading with the kit's `d3-covset.mjs` (sha256 `ecef3bdc...`); then `d3-bench-cov.sh <tree> <K> <P>
<L2> <set file> indicators ratio` with the same pins and environment. `d3-bench-cov.sh` is the bench half of Route B
that the PSRB-RSI session wrote (PARTS.md 2.4: "The bench wrapper and `main` are measured by running the bench target
once under the same instrumentation and merging its profile"), taken byte for byte from its report on
`cloud/psrb-rsi-20261103` (`reports/psrb-cloud-20261103/report-PSRB-RSI.md`), sha256 `31ddc848...`, the prefix that
report states; it is not a kit file.

Route B, 5 Nov 2026 IST: pins as above; instrumented build 20:11:00-20:11:07, 23 artifacts, 21 objects;
`cargo test -p indicators` 20:11:07-20:15:58, 532 passed, 0 failed, 21 profiles. Its exit was 3 only because the two
bench files are in no test object (`benches/ratio.rs` is `test = false`): "COVERAGE NOT READ ... NOT PROVED" for those
two, by design, as for PSRB-RSI. The bench half built the bench target in the same test profile (opt-level 0), ran it
once 20:16:00-20:22:24 (exit 0), merged its profile with the 21 test profiles (22) and read every set file again with
the tests' objects and the bench executable: exit 0, every set file read. The same two steps at `e836b47c`, whose
`crates/` the tip holds byte for byte, read the same figures (19:54-20:08 IST).

**Figures** (`d3-covset.mjs`, the merged reading at `a910ba92`):

| file | item | lines | branches |
|---|---|---|---|
| `crates/indicators/src/williams_r.rs` | whole | 195/195 | 38/38 |
| `crates/indicators/src/williams_r_tests.rs` | whole | 711/711 | 6/6 |
| `crates/indicators/benches/ratio/psrb_willr.rs` | whole | 70/70 | 0/0 |
| `crates/indicators/src/lib.rs` | line 78 | 0/0 (no region) | 0/0 |
| `crates/indicators/benches/ratio.rs` | line 68, line 69 | 0/0, 0/0 (no region) | 0/0 |
| `crates/indicators/benches/ratio.rs` | `main` 764-836 | 27/29 | 1/2 |
| the set | | 1,003/1,005 | 45/46 |

The tests-only reading of Route B gives the same figures for `williams_r.rs`, `williams_r_tests.rs` and `lib.rs`.

**(F) items, the only uncovered lines and side** (recipe step 4; no GAP, no (U)):

- `crates/indicators/benches/ratio.rs:831:8` branch True, `if verdicts.contains(&false) {`; `:832`
  `println!("A RATIO BREACHED ITS CEILING — see the lines marked BREACH.");`; `:833` `std::process::exit(1);`: `main`'s
  own failing arm, which a run reaches only when one of the file's own rows fails (CC-20's kind). These are base lines
  of `main`, in the set only because `main` holds this part's statement; PSRB-RSI's report lists the same three.

So every line and branch of the D2 set is covered except `main`'s own failing arm, an (F) item.

## Benches (advisory; the Mac's gate 8 run decides)

Machine as above (KVM guest, 4 vCPU Xeon @ 2.80 GHz, rustc 1.97.1), command `cargo bench -p indicators --bench ratio
--locked` (the bench profile), `CARGO_TARGET_DIR=/home/user/tgt/stable`, nothing else running. Each PSRB-WILLR sample
holds two clock reads, so its figures are upper bounds on the step. All three runs exit 0, every row of the file within
its ceiling and budget. The runs measured this part's `crates/` as committed at `237a6952` (run 1, HEAD `237a6952`) and
`e836b47c` (runs 2 and 3, HEAD and code tip `e836b47c`); `a910ba92` holds the same `crates/`.

| run | UTC | C-PSRB-WILLR-01 best-round p99 (2,000 / 20,000 / 200,000) | C-PSRB-WILLR-02 best-round p99 (14 / 140 / 1,400) |
|---|---|---|---|
| 1 | 14:15:13-14:15:46 | 96, 83, 81 ns: 1.000x, 0.864x, 0.843x | 81, 82, 83 ns: 1.000x, 1.012x, 1.024x |
| 2 | 14:38:52-14:39:14 | 79, 81, 190 ns: 1.000x, 1.025x, 2.405x | 117, 180, 180 ns: 1.000x, 1.538x, 1.538x |
| 3 | 14:39:38-14:40:02 | 80, 78, 101 ns: 1.000x, 0.975x, 1.262x | 93, 85, 108 ns: 1.000x, 0.913x, 1.161x |

Run 2's rise came in every round of those sizes, p50 with p99 (C-PSRB-WILLR-01 at 200,000: p50 67 ns, median and
worst rounds' p99 200 and 210 ns), and run 3 read the machine's floor at 1,104 ps against 589 ps in runs 1 and 2: the
host was noisy. Run 3's `/proc/stat` steal column rose by 27 ticks over its 24 s (3,585 to 3,612; not sampled in runs 1
and 2). Nothing in the state grows with the history (two deques of at most 14 entries at period 14, allocated once), so
no mechanism here ties the step to the history; run 2's 2.405x is the nearest any reading came to the 3.0x ceiling, and
L-PSRB-WILLR-03 says so.

Run 1, 2026-11-05 14:15:13 UTC to 2026-11-05 14:15:46 UTC, HEAD `237a6952`, load average: 0.28, 0.54, 0.33:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 589 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                          1095252 ps = 1859.511 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in          1086674 ps ->  1084976 ps   ratio 0.998x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in         1086674 ps ->  1112900 ps   ratio 1.024x  ok
  C-I-02 step: a wandering candle -> a motionless one         1160810 ps ->   726365 ps   ratio 0.625x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle  1160810 ps ->  1054789 ps   ratio 0.908x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  9563 ps over   2 iteration(s) = 4781 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          26263 ps over   5 iteration(s) = 5252 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             26206 ps over   5 iteration(s) = 5241 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         31779 ps over   6 iteration(s) = 5296 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover         1073062 ps ->   893678 ps   ratio 0.832x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1879152 ps ->  1983938 ps   ratio 1.055x  ok
  the stable floor is 1196 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        3087062 ps = 2581.155 floors, budget 5200   ok
  PSRB-WILLR: 2 id(s) for its 2 rows; the machine's floor is 589 ps; budget 2000 ns a step
C-PSRB-WILLR-01 history 2000, period 14: best of 9 rounds p50 52 ns, p99 96 ns, max 638691 ns, n 20000; median round p99 118 ns; worst round p99 141 ns; refusal None
C-PSRB-WILLR-01 history 20000, period 14: best of 9 rounds p50 48 ns, p99 83 ns, max 22168 ns, n 20000; median round p99 106 ns; worst round p99 168 ns; refusal None
C-PSRB-WILLR-01 history 200000, period 14: best of 9 rounds p50 48 ns, p99 81 ns, max 16838 ns, n 20000; median round p99 91 ns; worst round p99 175 ns; refusal None
C-PSRB-WILLR-01: best-round p99 against the first size 1.000x -> 0.864x -> 0.843x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-WILLR-02 period 14, history 20000: best of 9 rounds p50 48 ns, p99 81 ns, max 40563 ns, n 20000; median round p99 111 ns; worst round p99 171 ns; refusal None
C-PSRB-WILLR-02 period 140, history 20000: best of 9 rounds p50 47 ns, p99 82 ns, max 38485 ns, n 20000; median round p99 84 ns; worst round p99 95 ns; refusal None
C-PSRB-WILLR-02 period 1400, history 20000: best of 9 rounds p50 47 ns, p99 83 ns, max 17352 ns, n 20000; median round p99 104 ns; worst round p99 176 ns; refusal None
C-PSRB-WILLR-02: best-round p99 against the first size 1.000x -> 1.012x -> 1.024x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

Run 2, 2026-11-05 14:38:52 UTC to 2026-11-05 14:39:14 UTC, HEAD `e836b47c`, load average: 0.85, 1.40, 1.50:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 589 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                          1087658 ps = 1846.617 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in          1095925 ps ->  1131191 ps   ratio 1.032x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in         1095925 ps ->  1090702 ps   ratio 0.995x  ok
  C-I-02 step: a wandering candle -> a motionless one         1087268 ps ->   723150 ps   ratio 0.665x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle  1087268 ps ->  1079165 ps   ratio 0.992x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                  9595 ps over   2 iteration(s) = 4797 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          26489 ps over   5 iteration(s) = 5297 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             26963 ps over   5 iteration(s) = 5392 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         42374 ps over   6 iteration(s) = 7062 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover         2055079 ps ->   894729 ps   ratio 0.435x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           1801236 ps ->  1933162 ps   ratio 1.073x  ok
  the stable floor is 1106 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        1843611 ps = 1666.917 floors, budget 5200   ok
  PSRB-WILLR: 2 id(s) for its 2 rows; the machine's floor is 589 ps; budget 2000 ns a step
C-PSRB-WILLR-01 history 2000, period 14: best of 9 rounds p50 47 ns, p99 79 ns, max 14836 ns, n 20000; median round p99 82 ns; worst round p99 89 ns; refusal None
C-PSRB-WILLR-01 history 20000, period 14: best of 9 rounds p50 48 ns, p99 81 ns, max 21905 ns, n 20000; median round p99 87 ns; worst round p99 134 ns; refusal None
C-PSRB-WILLR-01 history 200000, period 14: best of 9 rounds p50 67 ns, p99 190 ns, max 74658 ns, n 20000; median round p99 200 ns; worst round p99 210 ns; refusal None
C-PSRB-WILLR-01: best-round p99 against the first size 1.000x -> 1.025x -> 2.405x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-WILLR-02 period 14, history 20000: best of 9 rounds p50 68 ns, p99 117 ns, max 31734 ns, n 20000; median round p99 186 ns; worst round p99 315 ns; refusal None
C-PSRB-WILLR-02 period 140, history 20000: best of 9 rounds p50 73 ns, p99 180 ns, max 36518 ns, n 20000; median round p99 196 ns; worst round p99 307 ns; refusal None
C-PSRB-WILLR-02 period 1400, history 20000: best of 9 rounds p50 73 ns, p99 180 ns, max 82360 ns, n 20000; median round p99 187 ns; worst round p99 318 ns; refusal None
C-PSRB-WILLR-02: best-round p99 against the first size 1.000x -> 1.538x -> 1.538x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

Run 3, 2026-11-05 14:39:38 UTC to 2026-11-05 14:40:02 UTC, HEAD `e836b47c`, load average: 0.64, 1.28, 1.45:

```text
indicators — gate 8, ceiling 3000 permille
  evaluator is 1776 bytes, emitting 328 positions
  the machine's floor is 1104 ps — one black-boxed wrapping_add
  C-I-05 step: one bar, every family                          1103016 ps = 999.108 floors, budget 2000   ok
  C-I-01 warm-up depth (context)                             5 -> 5 -> 5 sessions folded
  C-I-01 step: 1,000 candles in -> 50,000 candles in          1921689 ps ->  1797336 ps   ratio 0.935x  ok
  C-I-01 step: 1,000 candles in -> 200,000 candles in         1921689 ps ->  1078543 ps   ratio 0.561x  ok
  C-I-02 step: a wandering candle -> a motionless one         1225667 ps ->   712357 ps   ratio 0.581x  ok (cheaper)
  C-I-02 step: a wandering candle -> a decisive trend candle  1225667 ps ->  1030376 ps   ratio 0.840x  ok (cheaper)
  C-I-03 iterations at 1, i64::MAX, 10^30, i128::MAX         2 -> 5 -> 5 -> 6 of 16
  C-I-03 cost at v = 1 (context, NOT a ceiling)                 14900 ps over   2 iteration(s) = 7450 ps each
  C-I-03 cost at v = i64::MAX (context, NOT a ceiling)          31486 ps over   5 iteration(s) = 6297 ps each
  C-I-03 cost at v = 10^30 (context, NOT a ceiling)             31556 ps over   5 iteration(s) = 6311 ps each
  C-I-03 cost at v = i128::MAX (context, NOT a ceiling)         36802 ps over   6 iteration(s) = 6133 ps each
  C-I-04 rollovers performed (context)                       5 sessions crossed
  C-I-04 step: mid-session candle -> session rollover         1864168 ps ->  1559616 ps   ratio 0.836x  ok
  C-I-06 Column::build: 20,000 bars -> 200,000 bars           3132438 ps ->  2120322 ps   ratio 0.676x  ok
  the stable floor is 1212 ps — 2000000 adds, minimum of 24 trials
  C-I-07 step_known: one bar, every family, known mask        2825113 ps = 2330.951 floors, budget 5200   ok
  PSRB-WILLR: 2 id(s) for its 2 rows; the machine's floor is 1104 ps; budget 2000 ns a step
C-PSRB-WILLR-01 history 2000, period 14: best of 9 rounds p50 47 ns, p99 80 ns, max 21575 ns, n 20000; median round p99 89 ns; worst round p99 121 ns; refusal None
C-PSRB-WILLR-01 history 20000, period 14: best of 9 rounds p50 47 ns, p99 78 ns, max 17223 ns, n 20000; median round p99 79 ns; worst round p99 86 ns; refusal None
C-PSRB-WILLR-01 history 200000, period 14: best of 9 rounds p50 53 ns, p99 101 ns, max 18581 ns, n 20000; median round p99 168 ns; worst round p99 179 ns; refusal None
C-PSRB-WILLR-01: best-round p99 against the first size 1.000x -> 0.975x -> 1.262x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-WILLR-02 period 14, history 20000: best of 9 rounds p50 49 ns, p99 93 ns, max 40000 ns, n 20000; median round p99 132 ns; worst round p99 176 ns; refusal None
C-PSRB-WILLR-02 period 140, history 20000: best of 9 rounds p50 47 ns, p99 85 ns, max 47630 ns, n 20000; median round p99 126 ns; worst round p99 171 ns; refusal None
C-PSRB-WILLR-02 period 1400, history 20000: best of 9 rounds p50 50 ns, p99 108 ns, max 21079 ns, n 20000; median round p99 153 ns; worst round p99 183 ns; refusal None
C-PSRB-WILLR-02: best-round p99 against the first size 1.000x -> 0.913x -> 1.161x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```


The instrumented run of the bench half at `a910ba92` (opt-level 0, coverage profile, 20:16:00-20:22:24 IST, exit 0) is
a coverage run, not a cost figure; its PSRB-WILLR lines, for completeness:

```text
  PSRB-WILLR: 2 id(s) for its 2 rows; the machine's floor is 28167 ps; budget 2000 ns a step
C-PSRB-WILLR-01 history 2000, period 14: best of 9 rounds p50 411 ns, p99 577 ns, max 35670 ns, n 20000; median round p99 685 ns; worst round p99 838 ns; refusal None
C-PSRB-WILLR-01 history 20000, period 14: best of 9 rounds p50 630 ns, p99 818 ns, max 38065 ns, n 20000; median round p99 822 ns; worst round p99 831 ns; refusal None
C-PSRB-WILLR-01 history 200000, period 14: best of 9 rounds p50 392 ns, p99 515 ns, max 24847 ns, n 20000; median round p99 544 ns; worst round p99 728 ns; refusal None
C-PSRB-WILLR-01: best-round p99 against the first size 1.000x -> 1.417x -> 0.892x; ceiling 3.000x, budget 2000 ns   ok
C-PSRB-WILLR-02 period 14, history 20000: best of 9 rounds p50 414 ns, p99 588 ns, max 29087 ns, n 20000; median round p99 602 ns; worst round p99 787 ns; refusal None
C-PSRB-WILLR-02 period 140, history 20000: best of 9 rounds p50 411 ns, p99 565 ns, max 29637 ns, n 20000; median round p99 584 ns; worst round p99 642 ns; refusal None
C-PSRB-WILLR-02 period 1400, history 20000: best of 9 rounds p50 413 ns, p99 605 ns, max 33356 ns, n 20000; median round p99 630 ns; worst round p99 874 ns; refusal None
C-PSRB-WILLR-02: best-round p99 against the first size 1.000x -> 0.960x -> 1.028x; ceiling 3.000x, budget 2000 ns   ok
all ratios within the ceiling
```

## Decisions drafted (docs/05, this part's block only)

The five entry headings, as written (each `### D-864n — PSRB-WILLR: ... — 2026-11-05`):

- **D-8640**: Williams %R is `indicators::williams_r`, a reading with no vocabulary position, the published form over
  the last `period` candles, the first reading after `period` candles.
- **D-8641**: levels in ppm of the 0..-100 scale; exact comparisons `10^6 (HH - C)` against `-level (HH - LL)` in
  `i128`, a tie neither; `ppm` the floor, for records only; bounds const-asserted; a window of one price has no value
  (it records the absent `R > 0` conjunct).
- **D-8642**: the crossings in `TradingView`'s `ta.crossover` / `ta.crossunder` form, false next to a window of one price
  or across periods; the previous reading is the caller's copy.
- **D-8643**: two monotonic deques, expiry first, then back-pops and one push; allocated once in `new`; a wrapping
  `u64` candle number and a saturating `u32` warm count; refusals before any move; timestamps not read; no look-ahead.
- **D-8644**: `C-PSRB-WILLR-01` and `C-PSRB-WILLR-02`, their sizes, ceiling and 2 µs budget, the bench's hashed series;
  the helpers copied per part.

Each has **Decision**, **Rejected**, **Why** and **Proof** paragraphs in the PSRB-RSI form. D-8645..D-8649 are unused.
Every `D-nnnn` this part cites under `crates/` is one of D-8640..D-8644 (`decision_citations` green).

## Not done, open, or not proven here

- **Mutants: deferred** (operator decision, 30 Oct 2026); the set file above carries the mutants lines for the
  background pass. Point 1 of the departures section removes one equivalent mutant before it exists.
- **Gates 8, 10, 11, 12 and 14 on the Mac.** Gates 10, 11 and 12 were run here (above): red at this base for reasons
  outside this part, the same lines at START and at the tip. Gate 14's cover-table row for the two new ids is the main
  session's at the family assembly.
- **`TradingView` parity is UNVERIFIED** (L-PSRB-WILLR-04): neither their page nor their output was read; the `na` of a
  window of one price is Pine's division by zero as PARTS-WILLR-CCI.md section 8 recalls it. No real candle has passed
  through the module.
- **No `XW/anchors2-check.sh` run here**: the kit carries no copy; the hunks are pure insertions at the 3.4 anchor
  lines (listed in `part-PSRB-WILLR.md`).
- The cloud bench figures are advisory and noisy (three runs above); the Mac's gate 8 run decides.

## NEEDS THE MAC (PARTS-WILLR-CCI.md 5.7)

1. Check 1 on the Mac's pin: Route B on the D2 set above against START = PSRB-SEAM-2, the bench wrapper and `fn main`
   of `benches/ratio.rs` by the instrumented bench run.
2. Gate 8: `cargo bench -p indicators --bench ratio` with both `C-PSRB-WILLR` rows.
3. Gates 10, 11, 12 and 14 (gate 14's cover-table row for `C-PSRB-WILLR-01` and `-02`: the main session's at the
   family assembly).
4. macOS rows: none expected (no platform code).
5. `XW/anchors2-check.sh` re-run against the then-current first-wave tips, and the anchors on the then-current
   feat/db-fix.
6. Accept or send back the two departures from the letter of 5.3 and 5.6 (the absent `R > 0` conjunct; the hashed
   bench series).

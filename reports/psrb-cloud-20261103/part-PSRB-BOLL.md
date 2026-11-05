# part-PSRB-BOLL: Bollinger Bands for the PSRB family (what the Mac's check reads first)

PARTS.md section 4, built Thu 5 Nov 2026 by a cloud session. Branch `cloud/psrb-boll-20261103`, TOKEN
`PSRB-BOLL-20261103-1`, cut from `origin/cloud/psrb-kit-20261103` (`56f3043d`). START =
`35234cead4009299a93fcd79a3d1fffd773dcc16` (PSRB-SEAM on `1ad6e24ef8e0`). Code tip
(`git rev-list -1 HEAD -- . ':(exclude)reports'`): `19f410d2bd2c851c77aaed9974d647bb5df93487`. The full record, with
every command and figure, is `report-PSRB-BOLL.md`.

**At the tip:** fmt, clippy `-D warnings`, `cargo test -p indicators --locked` (527 passed, 0 failed) and the two core
tests (5 and 4 passed) green on the pinned stable; Route B on the pinned nightly: `bollinger.rs` 229/229 lines and
40/40 branches, `bollinger_tests.rs` 700/700 and 18/18, `psrb_boll.rs` 57/60 and `fn main` 24/26 lines, 1/2 branches,
the unrun ones only the two self-check failing arms, (F); each of the 23 tests red under a hand change; both bench rows
ok on this container (advisory).

## Files

Every file is in PARTS.md 4.1 and 2.2; `git diff --name-only START <tip> -- . ':(exclude)reports'` lists exactly these
nine, and `git diff -U0` shows every hunk as a pure insertion (`@@ -a,0 +b,c @@`), no line deleted or changed.

| file | change | anchor (base line, "insert after") |
|---|---|---|
| `crates/indicators/src/bollinger.rs` | new, 692 lines | — |
| `crates/indicators/src/bollinger_tests.rs` | new, 1,046 lines, 23 tests | — |
| `crates/indicators/benches/ratio/psrb_boll.rs` | new, 132 lines | — |
| `crates/indicators/src/lib.rs` | +1 `pub mod bollinger;` | under `pub mod anchored;` (:63) |
| `crates/indicators/benches/ratio.rs` | +2 (`#[path = "ratio/psrb_boll.rs"]`, `mod psrb_boll;`) and +5 (one re-binding statement, rustfmt's layout) | under `// slot bench-mod PSRB-BOLL` (:70) and `    // slot bench-row PSRB-BOLL` (:783) |
| `docs/04-invariants.md` | +19, one `##` section | above `## GDFL readers: …` (:9015) |
| `docs/05-decisions.md` | +157, `### D-8590` to `### D-8595` | above `### D-7310 — G3: …` (:86591) |
| `docs/06-limits.md` | +13, one `##` section | above `## One not-owed authority (R4-19; D-7315)` (:22023) |
| `docs/07-plan.md` | +6, one `##` section, one row | above `## Grammar search limits recorded but not changed — OPEN (D-0751 to D-0754)` (:1057) |

## API (`indicators::bollinger`), as PARTS.md 4.2 gives it

```rust
pub const MAX_PERIOD: u16 = 4_096;
pub const MAX_CLOSE: i64 = 1 << 32;
pub const MAX_WIDTH_MILLI: u32 = 10_000;
pub const WIDTH_DEN: i128 = 1_000;
pub enum Refusal { PeriodZero, PeriodAboveMax { period: u16 }, CloseOutOfRange { close: i64 },
                   WidthAboveMax { milli: u32 }, RingOutOfStep, Corrupt(crate::Corrupt) }  // Clone, Copy, Debug, PartialEq, Eq
pub struct Close(i64);   // Clone, Copy, Debug, PartialEq, Eq
impl Close { pub const fn new(paisa: i64) -> Result<Self, Refusal>; pub const fn paisa(self) -> i64; }
pub struct Width(u32);   // Clone, Copy, Debug, PartialEq, Eq
impl Width { pub const fn from_milli(milli: u32) -> Result<Self, Refusal>; pub const fn milli(self) -> u32; }
pub struct Bollinger { ring: Vec<i64>, next: usize, filled: u16, period: u16, sum: i128, sum_sq: i128, last: Close }
                         // Clone, Debug, PartialEq, Eq
impl Bollinger {
    pub fn new(period: u16) -> Result<Self, Refusal>;
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub const fn warm(&self) -> bool;
    pub fn window(&self) -> Option<Window>;
}
pub struct Window { period: u16, sum: i128, sum_sq: i128, last: Close }   // Clone, Copy, Debug, PartialEq, Eq
impl Window {
    pub const fn period(&self) -> u16;  pub const fn last(&self) -> Close;
    pub const fn sum(&self) -> i128;    pub const fn sum_sq(&self) -> i128;
    pub fn variance_numerator(&self) -> i128;
    pub fn basis_floor(&self) -> i64;
    pub fn above_upper(&self, close: Close, width: Width) -> bool;
    pub fn below_lower(&self, close: Close, width: Width) -> bool;
    pub fn upper_floor(&self, width: Width) -> i64;
    pub fn lower_ceil(&self, width: Width) -> i64;
    pub fn z_milli(&self, close: Close) -> Option<i64>;
    pub fn above_upper_sample(&self, close: Close, width: Width) -> Option<bool>;
    pub fn below_lower_sample(&self, close: Close, width: Width) -> Option<bool>;
}
pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }  // + Default
pub fn summarize(sorted: &[u64]) -> StepCost;
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool;
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;   // n 20
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>; // history 20,000
```

**Where it differs from the letter of 4.2, and why** (each is private or additive; no public name, argument or return
type differs):

1. `Bollinger` holds one more private field, `period: u16`. `period()` is asked as a `const fn` returning `u16`, and the
   ring's length is a `usize`: reading it back would need `u16::try_from` (not callable in a `const fn`) or an `as`
   (refused, 2.4). The field is set once in `new`.
2. `StepCost` also derives `Default` (an empty round reads all zeros; the wrapper's `unwrap_or_default`).
3. Private helpers: `Window::{n, reach, spread, beyond_sample}`, `square`, `nearest_rank`, `judge`, the synthetic
   `Walk`, and `measure(period, history, samples, rounds, base)`, which both public measuring functions call; its `base`
   lets a test make the walk's candles refuse, so the two `?` arms of the fold refusals inside the measurement are
   driven by input, not left unrun.
4. The domain bounds are const-asserted with `checked_mul` chains written as `if let` chains (edition 2024), over three
   private `i128` constants tied to the public ones by a fourth assertion, because a widening `From` cannot run in a
   constant and the module writes no `as`.

## Tests (`crates/indicators/src/bollinger_tests.rs`, the module's `tests` child; every test of PARTS.md 4.5 by name)

| test | what it pins | row |
|---|---|---|
| `a_zero_period_and_one_past_the_ceiling_are_refused` | 0, 4,097 and `u16::MAX` refused by name; 1 and 4,096 accepted with a ring of exactly that length | I05 |
| `a_close_outside_its_range_is_refused_and_changes_nothing` | `Close::new` at both edges (1, `MAX_CLOSE` read back; 0, -1, `i64::MIN`, `MAX_CLOSE + 1`, `i64::MAX` refused naming the value); through `fold`, `MAX_CLOSE` folds, `MAX_CLOSE + 1` and `i64::MAX` refuse, 0 and -1 are the candle's `PriceNotPositive`, whole state compared | I05 |
| `a_width_past_its_ceiling_is_refused` | 0, 1, 2,000, 10,000 accepted and read back; 10,001 and `u32::MAX` refused naming the value | I05 |
| `a_zero_width_band_is_the_basis` | BOLL-1 at width 0 (501 above, 500 neither, 499 below; both values 500); BOLL-1 with 901 (basis 500.125: floor 500, ceiling 501); the sample readings agree at width 0 | I02 |
| `a_corrupt_candle_is_refused_and_changes_nothing` | each `check_evaluable` kind (`HighBelowLow`, `RangeOverflows`, `PriceOutsideRange` twice, `NegativeVolume`, `PriceNotPositive`), whole state compared | I05 |
| `no_window_before_period_closes_and_one_at_the_period_th` | periods 1, 2, 3, 20, 4,096: none after `n - 1` folds, one at the `n`-th holding exactly those closes | I01 |
| `the_basis_and_sigma_match_fixture_boll_1` | S 4,000, Q 2,320,000, V 2,560,000, basis 500, `isqrt(V)` 1,600 so sigma 200; D 3,200; z-scores 2.000, 1.000, 0, -1.000, -2.000 | I03 |
| `a_close_exactly_on_the_band_is_not_beyond_it` | BOLL-1 at z 1.999, 2.000, 2.001 (above, ON, not; floors 899/900/900, ceilings 101/100/100), and the mirrored fixture for the lower side | I02 |
| `a_steady_trend_sits_one_point_six_four_seven_sigma_above_its_mean` | BOLL-2 S 2,190, Q 240,470, V 13,300, D 190, `190^2 * 21 = 13,300 * 57`; above at 1.647 (floor 118), not at 1.648 (119), z 1,647; falling: below at 1.647 (ceiling 101), not at 1.648 (100), z -1,648 | I03 |
| `a_flat_window_has_zero_sigma_and_the_close_on_both_bands` | 20 equal closes: V 0, no z-score, neither side at widths 0, 1, 2,000, 10,000 (population and sample), both values the basis; any other close beyond its side | I03 |
| `a_window_of_one_close_has_zero_sigma_and_no_sample_sigma` | n = 1: each fold replaces the window, V 0, no z-score, no sample reading, both values the close | I03 |
| `sample_sigma_is_wider_by_the_root_of_n_over_n_minus_one` | BOLL-1 at z 2.000: 901 and 927 above the population band only, 928 above both, the mirror below (99, 73, 72); 900 above the sample band at z 1.870, not at 1.871 | I03 |
| `an_all_up_and_an_all_down_series` | n consecutive closes at every n in 2..=60, nine widths: beyond iff `z_milli^2 (n + 1) < 3 (n - 1) 10^6`; the ties n 2 at 1.000 and n 7 at 1.500 by name | I03 |
| `a_gap_enters_the_window_and_leaves_it_after_period_closes` | a 10% jump in a period-5 window: sums equal a brute force at every candle, the jump gone exactly 5 closes later, the final window equal to a fresh one of the last five; stamps a minute apart, a day apart or all equal, with every other field changed, fold to equal states | I01, I06 |
| `the_square_test_and_the_floor_band_agree_on_every_integer_close` | 2,000 random windows, widths 0..=10,000: every close within 3 of each band value | I02 |
| `the_ring_equals_a_brute_force_window` | BRUTE FORCE, periods 1..=64, random walks with jumps and repeats: window existence, sums, and every comparison (population and sample, 7 closes, 6 widths) against the published definition over the last `n` closes | I04, I06 |
| `parity_with_the_published_definition` | the definition transcribed apart (deviations from the mean, never `nQ - S^2`): basis floor brackets `S / n`, `n V` = the definition's spread, every band test, both band values as the last close not beyond, the z-score as the floor; fixtures, mirrors, equal closes, 300 random windows; the slow path's four sign cases by name | I04 |
| `the_extremes_never_overflow` | 4,096 closes alternating 1 and `MAX_CLOSE`, z 10.000: sigma `(2^32 - 1) / 2`, each extreme exactly one sigma out (z ±1,000; above at 0.999, not at 1.000; sample likewise), basis 2,147,483,648, band values 23,622,320,123 and -19,327,352,826, inside the fallback edge; the const bounds restated with `checked_mul` | I05 |
| `a_ring_out_of_step_is_refused_by_name_and_changes_nothing` | WHITE-BOX: `next = period` is refused `RingOutOfStep`, whole state unchanged; through 50 folds `next` stays inside the ring | I05 |
| `the_previous_window_reads_the_current_candle_excluded` | a kept `Window` equals the published definition over the 20 closes before the candle and judges its close; the current window holds the candle | I01, I06 |
| `the_ring_never_grows_after_new` | length and capacity equal after 100,000 folds, at 20 and 4,096 | I05 |
| `the_bench_measures_both_rows_at_tiny_sizes` | the measuring functions' shapes, their period refusals, a walk refusing before and inside the timed rounds, no rounds, an unwarmed window timed; the walk within 4 paisa a minute | C-PSRB-BOLL-01, -02 |
| `a_cost_is_read_by_rank_and_judged_flat_or_not` | nearest ranks of 0, 1, 100, 200 samples; `flat_within` refuses no costs, a zero first p99, 3.01x and 2,001 ns, passes exactly 3.000x and 2,000 ns | C-PSRB-BOLL-01, -02 |

## Decisions (docs/05, this part's block only)

D-8590 the module and its reading (published form, current candle included, ring allocated once, no vocabulary
position, streaming, timestamps not read); D-8591 exact comparisons by squares and the const-asserted domain; D-8592
population sigma the reading, sample sigma labelled, `None` at n = 1; D-8593 band values and z-score exact via
`isqrt_i128`, for records and seeds, never a decision; D-8594 refusals change nothing, `RingOutOfStep` named and
unreachable; D-8595 the two benches, sizes, ceiling, 2 µs budget, helpers copied per part. D-8596 to D-8599 unused.
Before writing, `grep -nE '^#{2,4} +D-85[89][0-9]'` and `'^#{2,4} +D-860[0-9]'` on docs/05 found no heading.

## NEEDS THE MAC (PARTS.md 4.7, as 3.7 with the C-PSRB-BOLL rows)

1. **Check 1 on the Mac's pin**: Route B on the D2 set against the seam commit (the set file is in the report, section
   5), and the bench wrapper and `fn main` of `benches/ratio.rs` by one instrumented run of the bench target.
2. **Gate 8**: `cargo bench -p indicators --bench ratio` with both C-PSRB-BOLL rows; the figures here are advisory
   (report section 6), and the 2 µs budget and the 400 ns target are ESTIMATES until that run.
3. **Gates 10, 11, 12 and 14** on the Mac. Run here with the workflow's own commands and tools built by the pinned
   stable `rustc` (report section 4): each is red on START already, and this part adds no refusal to any of them.
   **Gate 14's cover-table row for C-PSRB-BOLL-01 and -02 is the main session's** at the family assembly (PARTS.md
   7.6); the row's measurement-point pin stays 6, since this part adds no `ratio(` call.
4. **macOS rows**: none expected (no platform code, no file system, no clock but `Instant` in the measuring functions).
5. **The anchors on the then-current feat/db-fix**: each insertion sits directly under its own slot or directly above
   its named heading, and nothing else in a shared file changed.
6. **Mutants**: deferred (operator, 30 Oct). The set file carries the mutants lines for the background pass.

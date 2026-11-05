# part-PSRB-CCI: what the Mac's check reads first

Session token `PSRB-CCI-20261105-1`. Branch `cloud/psrb-cci-20261105`, cut from `origin/cloud/psrb-kit-20261105`
(`fdc091ba69c9`). START (PSRB-SEAM-2) `deaa0f5c0606464f2ec79dc9e94de03792ece326`. Code tip
(`git rev-list -1 HEAD -- . ':(exclude)reports'`) `bd65eb6f7d8eb2a1af9dc75b3cf3b380d1df70c1`. The whole report is
`report-PSRB-CCI.md`; the machine-read summary is `result-PSRB-CCI.json.md`.

## Files (PARTS-WILLR-CCI.md 6.1 and 3.4), and nothing else outside `reports/`

`git diff --stat $START bd65eb6f -- . ':(exclude)reports'`: 9 files, 2,072 insertions, 0 deletions.

| file | change |
|---|---|
| `crates/indicators/src/cci.rs` | new, 581 lines |
| `crates/indicators/src/cci_tests.rs` | new, 1,091 lines; mounted at the end of `cci.rs` as `#[cfg(test)] #[path = "cci_tests.rs"] mod tests;` |
| `crates/indicators/benches/ratio/psrb_cci.rs` | new, 157 lines |
| `crates/indicators/src/lib.rs` | +1: `pub mod cci;` directly under `// slot PSRB-CCI (cci)` (new line 80) |
| `crates/indicators/benches/ratio.rs` | +2 directly under `// slot bench-mod PSRB-CCI` (`#[path = "ratio/psrb_cci.rs"]`, `mod psrb_cci;`, new lines 70-71); +5 directly under `    // slot bench-row PSRB-CCI`: `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_cci::row(floor, &["C-PSRB-CCI-01", "C-PSRB-CCI-02"])]).collect();` as rustfmt wraps it (new lines 790-794) |
| `docs/04-invariants.md` | +21 after line 8670, directly above `## Corporate actions, basis runs and the day universe (R4-26, D-6892)` |
| `docs/05-decisions.md` | +195 after line 87132, directly above `### D-7196 — R4-26 repair 2: a basis step is crossed back by the ratio's LEVELS, ...` |
| `docs/06-limits.md` | +13 after line 21623, directly above `## Corporate actions, basis runs and the day universe (R4-26, D-6892)` |
| `docs/07-plan.md` | +6 after line 928, directly above `## Cash-stock research checkpoint — 2026-09-05` |

Every hunk is an insertion only (`git diff -U0` shows `@@ -a,0 +b,n @@` for each), at the anchor PARTS-WILLR-CCI.md 3.4
names. No PSRB-WILLR slot or line, no first-wave file and no forbidden file was touched.

## API (`indicators::cci`), as PARTS-WILLR-CCI.md 6.2

- `pub const MAX_LEVEL_MILLI: i32 = 10_000_000` (+/-10,000 CCI points in thousandths; +200 is 200,000).
- `pub enum Refusal { PeriodZero, LevelOutOfRange { milli: i32 }, Corrupt(crate::Corrupt) }` (Clone, Copy, Debug,
  PartialEq, Eq).
- `pub struct Level(i32)` (Clone, Copy, Debug, PartialEq, Eq); `pub const fn from_milli(i32) -> Result<Level, Refusal>`,
  `pub const fn milli(self) -> i32`.
- `pub struct Cci { ring: VecDeque<i128>, sum: i128, last: i128, period: u16 }` (Clone, Debug, PartialEq, Eq; the ring
  of `T = H + L + C` reserved for `period` values in `new`, popped before the push when full with `unwrap_or(0)`);
  `pub fn new(u16) -> Result<Cci, Refusal>` (`PeriodZero` for 0 only), `pub const fn period(&self) -> u16`,
  `pub fn fold(&mut self, &Candle) -> Result<(), Refusal>` (O(1)), `pub fn warm(&self) -> bool`,
  `pub fn reading(&self) -> Option<Reading>` (O(period) by design: one pass sums `D`).
- `pub struct Reading { period: u16, deviation: i128, total_deviation: i128 }` (Clone, Copy, Debug, PartialEq, Eq);
  `pub const fn period`, `deviation` (`n T - S`), `total_deviation` (`D = sum |n T_i - S|`), `flat` (`D == 0`);
  `pub fn milli(&self) -> Option<i64>` (`floor(200_000 n dev / (3 D))`, `None` when `D == 0`), `above(Level)`,
  `below(Level)` (`D > 0` and `200_000 n dev` against `3 m D` in `i128`; a tie is neither).
- `pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }`;
  `pub fn summarize(&[u64]) -> StepCost`; `pub fn flat_within(&[StepCost], u64, u64) -> bool`;
  `pub fn measure_step(history: u64, samples: u64, rounds: u64)` (period 20, the C-PSRB-CCI-01 step),
  `pub fn measure_period(period: u16, samples: u64, rounds: u64)` (history 20,000, `fold` ALONE, C-PSRB-CCI-02) and
  `pub fn measure_reading_period(period: u16, samples: u64, rounds: u64)` (history 20,000, the step with the reading,
  for the unjudged line), each `-> Result<Vec<Vec<u64>>, Refusal>`.

Every `const fn` 6.2 shows is `const`; none was dropped. Every bound of 6.3 is a `const _: () = assert!(...)` taken
with `checked_mul` (`T < 2^65`, `S` and `n T < 2^81`, `D < 2^97`, `200_000 n |dev| < 2^115`, `3 |m| D < 2^123`).
`size_of::<Cci>()` is 80 bytes and `size_of::<Reading>()` 48 on x86_64 (a layout probe of the same field types with
rustc 1.97.1); `the_state_is_a_fixed_size` asserts at most 96 and 48. The module doc says that a ring built by `new`
never reallocates, that a clone reserves only its length, and that the measured path never clones.

## Tests (25, all in `crates/indicators/src/cci_tests.rs`, every name PARTS-WILLR-CCI.md 6.5 gives)

`a_zero_period_is_refused_and_every_other_period_is_accepted`,
`a_level_past_ten_thousand_points_is_refused_at_both_edges`, `the_first_reading_appears_after_exactly_period_candles`,
`cci_matches_fixture_cci_1`, `a_tie_at_plus_two_hundred_is_neither_above_nor_below`,
`a_floored_milli_equal_to_the_level_is_not_a_tie`, `the_confirmed_level_is_three_mean_deviations`,
`the_reading_never_passes_one_hundred_n_over_three`, `a_period_of_six_or_less_never_reads_above_two_hundred`,
`a_flat_window_has_no_value_and_compares_false`, `a_period_of_one_is_always_flat`,
`a_period_of_two_reads_plus_or_minus_sixty_six_point_six`, `milli_is_the_floor_of_the_exact_ratio`,
`the_ring_equals_a_brute_force_window`, `parity_with_the_published_definition`,
`the_largest_prices_and_period_never_overflow`, `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`,
`a_time_gap_reads_as_contiguous_candles`, `no_reading_depends_on_a_later_candle`,
`a_corrupt_candle_is_refused_and_changes_nothing`, `the_ring_never_grows_after_new`, `the_state_is_a_fixed_size`,
`the_previous_reading_judges_with_the_current_candle_excluded`, `the_bench_measures_every_row_at_tiny_sizes`,
`a_cost_is_read_by_rank_and_judged_flat_or_not`.

Fixtures CCI-1 (row for row), CCI-2, CCI-2m, CCI-3, the period-6 ceiling, the floored-`milli` fixture of 6.5 and the
ceiling at `u16::MAX` (S 196,608, dev 196,602, D 393,204, `milli` 2,184,500,000, above `i32::MAX`) are pinned exactly.
Each of the 25 was shown red under a hand change of `cci.rs` (27 changes) and the file restored byte for byte (the
table is in the report).

## Gates, coverage, benches (figures in the report)

- Pinned stable 1.97.1 at `bd65eb6f`: `cargo fmt --all --check`, `cargo clippy -p indicators --all-targets --locked --
  -D warnings`, `cargo test -p indicators --locked` (529 passed: START's 504 + 25) and `cargo test -p core --locked
  --test cost_invariants --test decision_citations` (5 and 4 passed): all exit 0. No host red.
- Route B on `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, reading narrow: `cci.rs` 184/184 lines, 22/22 branches;
  `cci_tests.rs` 766/766, 14/14; `psrb_cci.rs` 86/86, 0/0; `lib.rs` line 80 and `ratio.rs` lines 70-71 no region;
  `ratio.rs` `main` 764-833 24/26, 1/2, its breach arm (828 True, 829, 830) the only (F) items. GAPs 0.
- Benches (advisory, two runs here): C-PSRB-CCI-01 best-round p99 196/188/205 ns and 188/185/189 ns; C-PSRB-CCI-02
  37/34/38 ns and 35/37/34 ns; both ok. The unjudged reading line: 188, 809, 7,225 ns and 211, 789, 4,981 ns at periods
  20, 200, 2,000.

## Decisions (docs/05): D-8650 to D-8655; D-8656 to D-8659 unused

D-8650 module, published form, period, warm-up, no vocabulary position; D-8651 thirds and n-multiples, the identity,
exact comparisons, `milli`, the level domain, the const bounds; D-8652 the window of one typical price and the ceiling
`|CCI| <= 100 n / 3` (PSRB-STRAT's searched periods for +/-200 start at 7); D-8653 `fold` O(1), `reading()` O(period)
by design, the order-statistic tree not built (for the operator); D-8654 refusals, `unwrap_or(0)`, nothing but H, L, C
read, the ring allocated once; D-8655 the two bench rows, the unjudged reading line, the copied helpers. Rows:
PSRB-CCI-I01 to I06, C-PSRB-CCI-01 and -02 (docs/04); L-PSRB-CCI-01 to 04 (docs/06); one docs/07 row.

## NEEDS THE MAC (PARTS-WILLR-CCI.md 6.7, as 5.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against START = PSRB-SEAM-2, with `benches/ratio.rs` `main` and
   `benches/ratio/psrb_cci.rs` measured by the instrumented bench run (this session's set file, commands and figures
   are in the report; the cloud figure is the builder's).
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-CCI rows and the unjudged reading line printed
   whole (the cloud runs are advisory).
3. Gates 10, 11, 12 and 14. Gate 14's cover-table row for `C-PSRB-CCI-01` and `C-PSRB-CCI-02` is the main session's at
   the family assembly. Run here as CI runs them, gates 10, 11, 12 and 27b are red on START and on the tip with the same
   refusals, none naming a PSRB-CCI file or id; gates 0, 10b, 14, 25 and 27 are green.
4. macOS rows: none expected (no platform code, no file system, no clock but `Instant` in the measuring functions).
5. `XW/anchors2-check.sh` re-run against the then-current first-wave tips.
6. The anchors on the then-current `feat/db-fix` (the `lib.rs` slot, the two bench slots, the four ledger headings).
7. Mutants are deferred (operator, 30 Oct). The report names one equivalent mutant the background pass will meet: the
   `D > 0` guard of `above` and `below`, written as 6.3 states it, is redundant in value (`D == 0` forces `dev == 0`).
8. For the operator: D-8653, `reading()` O(period) accepted by default (L-PSRB-CCI-02), reversible before PSRB-STRAT's
   first run.

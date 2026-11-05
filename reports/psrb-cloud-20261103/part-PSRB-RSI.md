# part-PSRB-RSI: what the Mac's check reads first

Session token `PSRB-RSI-20261103-1`. Branch `cloud/psrb-rsi-20261103`, cut from `origin/cloud/psrb-kit-20261103`
(`56f3043dd4ca`). START (PSRB-SEAM) `35234cead4009299a93fcd79a3d1fffd773dcc16`. Code tip
(`git rev-list -1 HEAD -- . ':(exclude)reports'`) `947dbd3ad97854784ce3e895318b478a8a51354b`. The whole report is
`report-PSRB-RSI.md`; the machine-read summary is `result-PSRB-RSI.json.md`.

## Files (PARTS.md 3.1 and 2.2), and nothing else outside `reports/`

`git diff --stat $START 947dbd3a -- . ':(exclude)reports'`: 9 files, 1,776 insertions, 0 deletions.

| file | change |
|---|---|
| `crates/indicators/src/rsi.rs` | new, 497 lines |
| `crates/indicators/src/rsi_tests.rs` | new, 942 lines; mounted at the end of `rsi.rs` as `#[cfg(test)] #[path = "rsi_tests.rs"] mod tests;` |
| `crates/indicators/benches/ratio/psrb_rsi.rs` | new, 132 lines |
| `crates/indicators/src/lib.rs` | +1: `pub mod rsi;` directly after `pub mod pattern;` (new line 71) |
| `crates/indicators/benches/ratio.rs` | +2 directly under `// slot bench-mod PSRB-RSI` (`#[path = "ratio/psrb_rsi.rs"]`, `mod psrb_rsi;`, new lines 69-70); +5 directly under `    // slot bench-row PSRB-RSI`: `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_rsi::row(floor, &["C-PSRB-RSI-01", "C-PSRB-RSI-02"])]).collect();` as rustfmt wraps it (new lines 784-788) |
| `docs/04-invariants.md` | +13 after line 8950, directly above `## The quarantine (R4-18, D-7290)` |
| `docs/05-decisions.md` | +170 after line 84675, directly above `### D-7272 — AUT-10 decision 2 (DB-B44): ...` |
| `docs/06-limits.md` | +8 after line 21954, directly above `## The quarantine (R4-18, D-7290)` |
| `docs/07-plan.md` | +6 after line 982, directly above `## Historical sweep completion follow-through — 2026-09-06 (D-0524)` |

Every hunk is an insertion only (`git diff -U0` shows `@@ -a,0 +b,n @@` for each), at the anchor PARTS.md 2.2 names.

## API (`indicators::rsi`), as PARTS.md 3.2

- `pub const MAX_LEVEL_PPM: u32 = 1_000_000`, `pub const FAITHFUL_FACTOR: i128 = 1_000`.
- `pub enum Refusal { PeriodZero, LevelAboveMax { ppm: u32 }, Corrupt(crate::Corrupt) }` (Clone, Copy, Debug, PartialEq, Eq).
- `pub struct Level(u32)`; `pub const fn from_ppm(u32) -> Result<Level, Refusal>`, `pub const fn ppm(self) -> u32`.
- `pub struct Rsi { period: u16, previous: Option<i64>, changes: u32, gain: i128, loss: i128 }` (Clone, Copy, Debug,
  PartialEq, Eq; the two accumulators hold the seed sums until the seed, the averages after); `pub fn new(u16)`,
  `pub const fn period(&self)`, `pub fn fold(&mut self, &Candle) -> Result<(), Refusal>`, `pub fn warm(&self)`,
  `pub fn reading(&self) -> Option<Reading>`.
- `pub struct Reading { period: u16, avg_gain: i128, avg_loss: i128 }` (Clone, Copy, Debug, PartialEq, Eq);
  `pub const fn period`, `avg_gain_scaled`, `avg_loss_scaled`, `flat`; `pub fn faithful`, `ppm`, `above(Level)`,
  `below(Level)`.
- `pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }`;
  `pub fn summarize(&[u64]) -> StepCost`; `pub fn flat_within(&[StepCost], u64, u64) -> bool`;
  `pub fn measure_step(history: u64, samples: u64, rounds: u64)` (period 14) and
  `pub fn measure_period(period: u16, samples: u64, rounds: u64)` (history 20,000), each
  `-> Result<Vec<Vec<u64>>, Refusal>`.

Every `const fn` PARTS.md 3.2 shows is `const`; none was dropped. Beyond the shown signatures: `StepCost` derives
`Clone, Copy, Debug, Default, PartialEq, Eq` (3.2 shows only its fields); private items `PPM`, `MOVE_LIMIT`,
`UNIT_LIMIT`, `PERIOD_LIMIT`, `GAIN_LIMIT` (the const-asserted bounds), `STEP_PERIOD`, `PERIOD_HISTORY`, `nearest_rank`,
`measure` (the one body behind both measuring functions) and `bench_candle`. Every bound of 3.3 is a
`const _: () = assert!(...)` taken with `checked_mul`. `size_of::<Rsi>()` is 64 bytes and `size_of::<Reading>()` 48 on
x86_64 (a layout probe of the same field types with rustc 1.97.1); the build asserts at most 96.

## Tests (27, all in `crates/indicators/src/rsi_tests.rs`, every name PARTS.md 3.5 gives)

`a_zero_period_is_refused_and_every_other_period_is_accepted`, `a_level_above_one_hundred_is_refused_at_the_edge`,
`the_first_close_folds_no_change_and_reads_nothing`, `the_reading_appears_after_exactly_period_changes`,
`the_largest_period_smooths_after_its_seed`, `the_seed_is_the_floored_mean_of_the_first_period_moves`,
`wilder_smoothing_matches_fixture_rsi_1`, `a_flat_series_reads_one_hundred_and_says_it_is_flat`,
`an_all_up_series_reads_one_hundred`, `an_all_down_series_reads_zero`, `equal_moves_at_the_seed_read_fifty`,
`a_period_of_one_reads_the_last_change_alone`, `a_tie_at_the_level_is_neither_above_nor_below`,
`a_floor_ppm_equal_to_the_level_is_not_a_tie`, `ppm_is_the_floor_of_the_exact_ratio`,
`a_price_gap_is_one_move_and_nothing_more`, `a_time_gap_reads_as_contiguous_candles`,
`no_reading_depends_on_a_later_candle`, `the_stream_equals_a_from_scratch_recomputation`,
`the_fixed_point_stays_within_period_units_of_exact_wilder`,
`a_long_unchanged_run_departs_only_where_the_ratio_bound_allows`, `the_faithful_edge_is_one_thousand_units_per_period`,
`the_largest_moves_and_period_never_overflow`, `a_corrupt_candle_is_refused_and_changes_nothing`,
`the_state_is_a_fixed_size_and_never_allocates`, `the_bench_measures_both_rows_at_tiny_sizes`,
`a_cost_is_read_by_rank_and_judged_flat_or_not`.

Fixture RSI-1's table is pinned row for row; every long-run figure of 3.5 is pinned (exact floors 428,572, 547,023,
593,601; unfaithful first after 11, 28, 70; departures after 20 (G 0, L 1), 58 (G 1, L 0), 159 (G 5, L 0); both
averages 0 first after 21, 59, 164; the period-2 run near 100: 19 (G 9,536, L 0, faithful), 22, 33). Each of the 27 was
shown red under a hand change of `rsi.rs` and the file restored byte for byte (the table is in the report).

## Decisions (docs/05): D-8580 to D-8584; D-8585 to D-8589 unused

D-8580 module, period, warm-up and the `u32` counter; D-8581 units, floors, the two bounds, `faithful()`, exact
comparisons, `ppm`, const bounds; D-8582 both averages zero read 100 with `flat()`, and the long-run departures;
D-8583 refusals, closes only, no look-ahead; D-8584 the two bench rows and the copied helpers. Rows: PSRB-RSI-I01 to
I06, C-PSRB-RSI-01 and -02 (docs/04); L-PSRB-RSI-01 to 03 (docs/06); one docs/07 row.

## NEEDS THE MAC (PARTS.md 3.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against START, with `benches/ratio.rs` `main` and
   `benches/ratio/psrb_rsi.rs` measured by the instrumented bench run (this session's figures and how they were taken
   are in the report; the cloud figure is the builder's).
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-RSI rows (the cloud run is advisory).
3. Gates 10, 11, 12 and 14. Gate 14's cover-table row for `C-PSRB-RSI-01` and `C-PSRB-RSI-02` is the main session's at
   the family assembly (as for F1). Run here as CI runs them, gates 10, 11, 12 and 27b are red on START and on the tip
   with the same refusals, none naming a PSRB-RSI file or id (report, "Gates 10, 11, 12, 27, 27b"); gate 27 is green.
4. macOS rows: none expected (no platform code, no file system, no clock but `Instant` in the measuring functions).
5. The anchors on the then-current `feat/db-fix` (lib.rs, the two bench slots, the four ledger headings).
6. Mutants are deferred (operator, 30 Oct): the set file's mutants lines are in the report for the background pass.

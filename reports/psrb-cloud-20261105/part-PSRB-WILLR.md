# part-PSRB-WILLR: what the Mac's check reads first

Part PSRB-WILLR of the PSRB second wave (PARTS-WILLR-CCI.md 5): Williams %R in exact integers as
`indicators::williams_r`. Branch `cloud/psrb-willr-20261105`, START = PSRB-SEAM-2
`deaa0f5c0606464f2ec79dc9e94de03792ece326`. The code tip, the gates, coverage and benches are in
`report-PSRB-WILLR.md`; the machine-readable record is `result-PSRB-WILLR.json.md`.

## Files (PARTS-WILLR-CCI.md 5.1 and 3.4; nothing else in the tree)

| file | change |
|---|---|
| `crates/indicators/src/williams_r.rs` | new: the module |
| `crates/indicators/src/williams_r_tests.rs` | new: its 28 tests, `#[cfg(test)] #[path = "williams_r_tests.rs"] mod tests;` |
| `crates/indicators/benches/ratio/psrb_willr.rs` | new: the bench wrapper, `pub(crate) fn row(floor: u128, ids: &[&str]) -> bool` |
| `crates/indicators/src/lib.rs` | +1: `pub mod williams_r;` directly under `// slot PSRB-WILLR (williams_r)` (line 78) |
| `crates/indicators/benches/ratio.rs` | +2 under `// slot bench-mod PSRB-WILLR` (`#[path = "ratio/psrb_willr.rs"]` / `mod psrb_willr;`, lines 68-69); +8 under `    // slot bench-row PSRB-WILLR`: the row statement of 5.1, which rustfmt wraps into 8 lines (lines 788-795), not the 5 of 5.1's estimate, because `psrb_willr::row(...)` is 62 characters, past rustfmt's 60-character call width (RSI's was 57) |
| `docs/04-invariants.md` | +13 directly above `## One not-owed authority: NSE's withdrawal, before listing, after delisting, unread (R4-19; D-7315)`: `## Williams %R for the PSRB family (PSRB-WILLR; D-8640 to D-8644)`, rows PSRB-WILLR-I01..I06, C-PSRB-WILLR-01, C-PSRB-WILLR-02 |
| `docs/05-decisions.md` | +216 directly above `### D-7315 — R4-19: one not-owed authority — ...`: D-8640..D-8644 |
| `docs/06-limits.md` | +9 directly above `## GDFL readers: series codes, stem hand-overs, header shapes, named bad rows (R4-22; D-7310 to D-7314)`: L-PSRB-WILLR-01..04 |
| `docs/07-plan.md` | +7 directly above `## GDFL one-second fills: the capital-market reader and its archive check — 2026-10-01 (D-0808, D-0812)`, the block starting with a blank line (3.4): one row |

Every hunk is a pure insertion at the anchor 3.4 names (`git diff -U0 START`: lib.rs `@@ -77,0 +78 @@`, ratio.rs
`@@ -67,0 +68,2 @@` and `@@ -785,0 +788,8 @@`, docs/04 `@@ -9037,0 +9038,13 @@`, docs/05 `@@ -86872,0 +86873,216 @@`,
docs/06 `@@ -22006,0 +22007,9 @@`, docs/07 `@@ -1083,0 +1084,7 @@`). No line of `trend.rs`, `vwap.rs`, `daily.rs`,
`evaluator.rs`, a first-wave file, `crates/indicators/tests/`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`, `.github/` or
PSRB-CCI's slots changed.

## API (PARTS-WILLR-CCI.md 5.2, as built)

```rust
pub const MIN_LEVEL_PPM: i32 = -1_000_000;
pub const MAX_LEVEL_PPM: i32 = 0;
pub enum Refusal { PeriodZero, LevelOutOfRange { ppm: i32 }, Corrupt(crate::Corrupt) }  // Clone, Copy, Debug, PartialEq, Eq
pub struct Level(i32);                                                                // Clone, Copy, Debug, PartialEq, Eq
impl Level { pub const fn from_ppm(ppm: i32) -> Result<Self, Refusal>; pub const fn ppm(self) -> i32; }
pub struct WilliamsR { period: u16, highs: VecDeque<(u64, i64)>, lows: VecDeque<(u64, i64)>, next: u64, seen: u32, close: i64 }
                                                                                      // Clone, Debug, PartialEq, Eq
impl WilliamsR {
    pub fn new(period: u16) -> Result<Self, Refusal>;
    pub const fn period(&self) -> u16;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub fn warm(&self) -> bool;
    pub fn reading(&self) -> Option<Reading>;
}
pub struct Reading { period: u16, highest: i64, lowest: i64, close: i64 }              // Clone, Copy, Debug, PartialEq, Eq
impl Reading {
    pub const fn period(&self) -> u16;
    pub const fn highest(&self) -> i64;
    pub const fn lowest(&self) -> i64;
    pub const fn close(&self) -> i64;
    pub const fn flat(&self) -> bool;
    pub fn ppm(&self) -> Option<i32>;
    pub fn above(&self, level: Level) -> bool;
    pub fn below(&self, level: Level) -> bool;
    pub fn crossed_above(&self, previous: &Reading, level: Level) -> bool;
    pub fn crossed_below(&self, previous: &Reading, level: Level) -> bool;
}
pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }  // + Default
pub fn summarize(sorted: &[u64]) -> StepCost;
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool;
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;   // period 14
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>; // history 20,000
```

Names, types and `const fn` exactly as 5.2; `warm`, `ppm`, `above`, `below` and the crossings are not `const` (they widen
with `u32::from` / `i128::from`), as 5.2 says. Two points the Mac should read against 5.3:

1. **`above` and `below` carry no separate `R > 0` conjunct.** 5.3 writes `R > 0 && 10^6 U < (-l) R`. With `0 <= U <= R`,
   `R = 0` forces `U = 0`, both products are 0 and the strict comparison is already false, so the conjunct changes no
   result at any reachable state; written, it would be a branch whose removal no test can see (an equivalent mutant
   the deferred mutants pass would file as a survivor). The window of one price still compares false, pinned by
   `a_flat_window_has_no_value_and_compares_false`; D-8641 records the choice. The crossings keep `!previous.flat()`,
   which is not redundant (WR-2's crossing from a reading of one price).
2. **The bench's candles are a hashed series, not a stride.** `bench_candle` picks the close among 4,001 prices 13 paisa
   apart and the high and low offsets by `SplitMix64`'s finalizer of the minute and the field. A stride of 7,919 through
   4,001 prices (the first wave's form) is a falling sawtooth (7,919 is -83 modulo 4,001) whose jumps empty the highs
   deque about once in 48 candles, inside the p99; D-8644 records why the hashed series is the typical step and the
   staircase is L-PSRB-WILLR-01's.

## Tests (`crates/indicators/src/williams_r_tests.rs`; PARTS-WILLR-CCI.md 5.5, every name)

1. `a_zero_period_is_refused_and_every_other_period_is_accepted`
2. `a_level_outside_minus_one_hundred_to_zero_is_refused_at_both_edges`
3. `the_first_reading_appears_after_exactly_period_candles`
4. `williams_r_matches_fixture_wr_1`
5. `a_tie_at_the_level_is_neither_above_nor_below`
6. `a_floored_ppm_equal_to_the_level_is_not_a_tie`
7. `a_flat_window_has_no_value_and_compares_false`
8. `a_period_of_one_reads_the_candle_alone`
9. `the_crossings_follow_tradingviews_crossover`
10. `the_long_and_short_levels_overlap_between_minus_eighty_five_and_minus_fifteen`
11. `every_reading_lies_between_minus_one_hundred_and_zero`
12. `ppm_is_the_floor_of_the_exact_ratio`
13. `the_deques_equal_a_brute_force_window`
14. `parity_with_the_published_definition`
15. `the_pops_stay_bounded_at_the_ninety_ninth_percentile`
16. `at_most_one_entry_expires_per_candle`
17. `the_deques_never_grow_past_their_period_or_reallocate`
18. `the_candle_counter_wraps_without_changing_a_reading`
19. `the_warm_counter_saturates_and_stays_warm`
20. `the_largest_prices_and_period_never_overflow`
21. `a_price_gap_enters_the_window_and_leaves_it_after_period_candles`
22. `a_time_gap_reads_as_contiguous_candles`
23. `no_reading_depends_on_a_later_candle`
24. `a_corrupt_candle_is_refused_and_changes_nothing`
25. `the_state_is_a_fixed_size_and_allocates_only_in_new`
26. `the_previous_reading_judges_with_the_current_candle_excluded`
27. `the_bench_measures_both_rows_at_tiny_sizes`
28. `a_cost_is_read_by_rank_and_judged_flat_or_not`

Fixtures WR-1, WR-2, the 5.5 floored-tie candle, the period-one candles and the 5.3 staircase are in the file exactly as
5.3 and 5.5 state them; the pop test pins every back-pop figure of 5.5 (and 5.3's p99 and removal figures) at both
counter starts.

## Decisions (docs/05, this part's block only)

- **D-8640**: Williams %R as `indicators::williams_r`, no vocabulary position; the published form over the last `period`
  candles, the current one included; runtime `u16` period; the first reading after `period` candles; carried across
  sessions.
- **D-8641**: levels in ppm of the 0..-100 scale; exact comparisons `10^6 (HH - C)` against `-level (HH - LL)` in `i128`,
  a tie neither; `ppm` the floor, for records only; bounds const-asserted; a window of one price has no value; no
  separate `R > 0` conjunct (point 1 above); the two default levels overlap.
- **D-8642**: the crossings in `ta.crossover` / `ta.crossunder` form, false next to a window of one price or across
  periods; the previous reading is the caller's copy.
- **D-8643**: two monotonic deques allocated once, expiry first, then back-pops and one push; a clone reserves only its
  length and the measured path never clones; the wrapping `u64` candle number and the saturating `u32` warm count;
  `map_or` where `front()` cannot be `None`; refusals before any move; timestamps not read; no look-ahead.
- **D-8644**: `C-PSRB-WILLR-01` and `-02`, their sizes, ceiling and 2 µs budget, the hashed bench series (point 2 above);
  the helpers copied per part.

D-8645..D-8649 are unused (spare).

## NEEDS THE MAC (PARTS-WILLR-CCI.md 5.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against START = PSRB-SEAM-2 (the set is in the report), the bench
   wrapper and `fn main` of `benches/ratio.rs` by the instrumented bench run.
2. Gate 8: `cargo bench -p indicators --bench ratio` with both `C-PSRB-WILLR` rows (the cloud figure is advisory).
3. Gates 10, 11, 12 and 14 on the Mac. Run here with the repository's own `.github` tools: gates 10, 11 and 12 refuse
   at this base with the same refusal lines at START and at the tip (none names a file of this part); this part adds one
   gate-12 claim block, its module doc, which names `crates/indicators/benches/ratio.rs` and says UNVERIFIED. Gate 14's
   cover-table row for `C-PSRB-WILLR-01` and `-02` is the main session's at the family assembly.
4. macOS rows: none expected (no platform code).
5. `XW/anchors2-check.sh` re-run against the then-current first-wave tips, and the anchors on the then-current
   feat/db-fix.
6. The `R > 0` conjunct and the hashed bench series (the two points under API): the main session's to accept or send
   back.

# part-PSRB-STV.md: what the Mac's check reads first

Part **PSRB-STV** (`state/strategy-psrb-20261103/PARTS.md` section 5), TOKEN `PSRB-STV-20261103-1`, branch
`cloud/psrb-stv-20261103`. START (PSRB-SEAM) `35234cead4009299a93fcd79a3d1fffd773dcc16`; FROM (kit)
`56f3043dd4ca8f77520eda8ce0f1f0d69b42d915`; code tip `c3c39a44bc23d6c08c806140f00d17444da9e490`. Full detail, figures
and logs: `report-PSRB-STV.md`. Machine-read summary: `result-PSRB-STV.json.md`.

## Files (the Files list of PARTS.md 5.1 and the anchors of 2.2; nothing else changes outside `reports/`)

- New: `crates/indicators/src/dual_band_supertrend.rs`, `crates/indicators/src/dual_band_supertrend_tests.rs`,
  `crates/indicators/benches/ratio/psrb_stv.rs`.
- `crates/indicators/src/lib.rs`: one line, `pub mod dual_band_supertrend;` at :66, directly under `pub mod daily;`.
- `crates/indicators/benches/ratio.rs`: `#[path = "ratio/psrb_stv.rs"]` and `mod psrb_stv;` at :73-74 directly under
  `// slot bench-mod PSRB-STV`; one statement at :788-792 directly under `    // slot bench-row PSRB-STV`:
  `let verdicts: Vec<bool> = verdicts.iter().copied().chain([psrb_stv::row(floor, &["C-PSRB-STV-01",
  "C-PSRB-STV-02"])]).collect();` as rustfmt wraps it. No `ratio(` call added (gate 14's count stays 6).
- Ledgers, each a pure insertion directly above its anchor: docs/04 :9062-9085 (above `## /db reads only what its
  view shows (R3-12; D-7111)`), docs/05 :86993-87132 (above `### D-7111 — /db reads only what its view shows: ...`),
  docs/06 :22067-22081 (above `## /db reads only what its view shows: what R3-12 leaves bounded, unmeasured or open
  (R3-12, D-7111)`), docs/07 :1110-1115 (above `## GDFL import at one second — 2026-10-05 (D-2802 to D-2806)`).
- Untouched, checked with `git diff --exit-code START HEAD`: `crates/indicators/src/trend.rs`, `evaluator.rs`,
  `vwap.rs`, `daily.rs`, `crates/indicators/tests/`, `crates/vocab`, `Cargo.toml`, `Cargo.lock`, `CLAUDE.md`,
  `AGENTS.md`, `docs/00-charter.md`, `.github/`.

## API (`indicators::dual_band_supertrend`, PARTS.md 5.2)

```rust
pub const MAX_FACTOR_MILLI: u32 = 100_000;
pub enum Refusal { PeriodZero, FactorOutOfRange { milli: u32 }, Corrupt(crate::Corrupt) }    // Clone, Copy, Debug, PartialEq, Eq
pub struct DualBandSuperTrend { /* atr: trend::Atr, factor_milli: i128, previous_close: Option<i64>, upper: i128,
                                   lower: i128, trend: trend::Trend, seen_atr: bool, reading: Option<Reading> */ }
                                   // Clone, Copy, Debug, PartialEq, Eq; 224 bytes, asserted <= 256
impl DualBandSuperTrend {
    pub fn new(atr_period: u16, factor_milli: u32) -> Result<Self, Refusal>;
    pub fn fold(&mut self, candle: &crate::Candle) -> Result<(), Refusal>;
    pub fn reading(&self) -> Option<Reading>;
    pub fn atr(&self) -> Option<i64>;
}
pub struct Reading { pub trend: trend::Trend, pub line: i128, pub upper: i128, pub lower: i128, pub settled: bool }
pub struct StepCost { pub p50_ns: u64, pub p99_ns: u64, pub max_ns: u64, pub samples: usize }    // also Default
pub fn summarize(sorted: &[u64]) -> StepCost;
pub fn flat_within(costs: &[StepCost], ceiling_permille: u64, budget_ns: u64) -> bool;
pub fn measure_step(history: u64, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;   // period 10, factor 3,000
pub fn measure_period(period: u16, samples: u64, rounds: u64) -> Result<Vec<Vec<u64>>, Refusal>;  // after 20,000 candles
```

## Tests (23, `crates/indicators/src/dual_band_supertrend_tests.rs`, the names of PARTS.md 5.5)

`a_zero_period_and_a_factor_outside_its_range_are_refused`, `nothing_is_read_before_the_atr_has_its_period`,
`the_first_atr_candle_is_down_on_the_upper_band_and_unsettled`, `period_one_reads_the_published_zero_band_first`,
`both_bands_ratchet_on_every_candle_and_reset_as_published`, `a_turn_needs_a_close_beyond_the_current_band`,
`fixture_st_1_the_seed_differs`, `fixture_st_2_a_flip_down_keeps_the_ratcheted_upper_band`,
`fixture_st_3_a_flip_up_keeps_the_ratcheted_lower_band_and_turns_down_first`,
`fixture_st_4_a_flat_series_reads_up_in_the_latch_and_down_here`,
`fixture_st_5_a_wide_candle_turns_the_two_band_form_first`, `fixture_st_9_equal_bands_do_not_turn_an_up_candle`,
`a_gap_through_either_band_turns_on_the_gap_candle`, `an_all_up_and_an_all_down_series`,
`the_stream_equals_a_from_scratch_transcription_of_the_published_rules`, `the_atr_is_the_engine_atr`,
`no_reading_depends_on_a_later_candle`, `a_time_gap_reads_as_contiguous_candles`, `the_extremes_never_overflow`,
`a_corrupt_candle_is_refused_and_changes_nothing`, `the_state_is_a_fixed_size`,
`the_bench_measures_both_rows_at_tiny_sizes`, `a_cost_is_read_by_rank_and_judged_flat_or_not`.

Each fixture test asserts the two-band form, the latch and the engine ATR on every candle. Every test was shown red
under at least one of 25 hand changes (report section 7).

## Figures (pinned stable 1.97.1 unless named)

- `cargo fmt --all --check` clean; `cargo clippy -p indicators --all-targets --locked -- -D warnings` exit 0;
  `cargo test -p indicators --locked` 527 passed, 0 failed (START: 504); `cargo test -p core --locked --test
  cost_invariants --test decision_citations` 9 passed.
- Route B, `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, with the bench target run once under the same
  instrumentation and merged: `dual_band_supertrend.rs` lines 148/148, branches 28/28; `dual_band_supertrend_tests.rs`
  775/775, 54/54; `benches/ratio/psrb_stv.rs` 45/48, 3/6; `benches/ratio.rs` `main` 24/26, 1/2; the three added lines
  outside functions 0/0. Every miss is a self-checking bench's own failing arm, (F), listed in report section 6.
- Gates 10, 11 and 12, run locally as CI runs them, refuse the same things at START and at the tip (all in other
  crates); this part adds no refusal.
- Benches (advisory), two release runs: C-PSRB-STV-01 best-round p99 62, 64, 64 ns and 81, 82, 84 ns; C-PSRB-STV-02
  63, 65, 65 ns and 82, 81, 82 ns; ceiling 3.0x and budget 2,000 ns, every row of both runs `ok`.

## Decisions (docs/05, this part's block; D-8606 to D-8609 spare)

D-8600 the module, beside the untouched latch; D-8601 the ATR reused and the latch's band arithmetic, bands in `i128`;
D-8602 the previous trend picks the band judged, the script's zero prior bands and first-candle direction kept,
LITERAL-PINE named; D-8603 the domain and refusals, timestamps not read, no look-ahead; D-8604 `settled`, PSRB-STRAT to
read settled readings only; D-8605 the two benches and the per-part helpers. Rows: PSRB-STV-I01 to I06, C-PSRB-STV-01
and -02 (docs/04); L-PSRB-STV-01 to 04 (docs/06); one docs/07 row.

## NEEDS THE MAC (PARTS.md 5.7, as 3.7)

1. Check 1 on the Mac's pin: Route B on the D2 set against the seam commit; the bench wrapper and `main` by one
   instrumented bench run merged in.
2. Gate 8: `cargo bench -p indicators --bench ratio` with both C-PSRB-STV rows.
3. Gates 10, 11, 12 and 14; gate 14's cover-table row for `C-PSRB-STV-01` and `C-PSRB-STV-02` is the main session's at
   the family assembly.
4. macOS rows: none expected; `the_state_is_a_fixed_size` pins 224 bytes, measured on x86_64 only.
5. The anchors on the then-current feat/db-fix.
6. `git diff --exit-code <seam> <tip> -- crates/indicators/src/trend.rs crates/indicators/src/evaluator.rs
   crates/vocab`: empty here.

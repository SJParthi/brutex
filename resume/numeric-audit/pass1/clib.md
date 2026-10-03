# SLICE clib — crates/cli/src/lib.rs, numeric/look-ahead/O(1) pass 1

Commit 331b05c (origin/final/all-fixes). Production code is lines 1..20359; `mod tests` begins at 20368.

## Verdict

lib.rs is mostly glue code with very little float in it. Workspace lints (`float_arithmetic`, `cast_possible_truncation`, `cast_sign_loss` = deny) and `overflow-checks = true` in release hold for every site I checked. I found **two new low-severity rounding defects**. Both are support floors that are rounded down twice, or rounded before a multiply, so the floor comes out lower than the figure its own doc says it computes. I also found one item that is already reported in this pass (run1-3, `bar_milli` truncation). I re-confirmed it but do not count it as new.

Probe: `$SCRATCH/probes/clib` (path deps on `crates/runner` and `crates/cli`). Every output below is verbatim from `cargo run` of that probe.

## Findings

### clib-1 (low): the statistical support floor loses one trade in its ppm round trip, so `min_hits` is `needed - 1` and the floor is never actually applied

- **Where:** `crates/cli/src/lib.rs:11391` (`statistical_floor_ppm`) and `crates/cli/src/lib.rs:8358-8360` (`min_hits_for`). Reached through `statistical_support_floor` (lib.rs:11351), `screen_range` (lib.rs:13502-13506) and `elite_descend` (lib.rs:14208, then 11149).
- **Code:**
  ```rust
  needed.saturating_mul(1_000_000).saturating_div(bars).max(1)   // 11391: ppm, floored
  ...
  let hits = (bars as u64).saturating_mul(support_ppm) / 1_000_000; // 8359: hits, floored again
  ```
- **Why it is wrong:** The floor is documented as the answer to *"below how many round trips can the stated win rate no longer clear its own confidence bound? Under that count no combination can pass"* (lib.rs:11336-11340). `needed` is that count. The value goes to ppm by a floor division and comes back to hits by a second floor division. Unless `needed·1e6` is an exact multiple of `bars`, the result is `needed - 1`. So the extinction threshold sits one hit below the count the doc says no combination can go under. The doc at lib.rs:11358-11359 even records the symptom ("`trades_needed_for` answered 29 -- which after the ppm round trip is the `min_hits: 28`") but treats it as context, not as a defect. It is still live. The effect is that the search descends one hit deeper than the statistic justifies (more candidates, no extra admissible answers). It is not a wrong verdict, which is why this is low.
- **Repro (probe ran):** `Rules::elite(1,1).with_win_rate(sizing_rate_bp())`, `trades_needed_for(.., 5_000)`, the real `cli::statistical_support_floor`, then the `min_hits_for` formula verbatim:
  ```
  bars=1671 needed=47 floor_ppm=28126 min_hits=46
  bars=6300 needed=47 floor_ppm=7460 min_hits=46
  bars=75600 needed=47 floor_ppm=621 min_hits=46
  bars=617921 needed=47 floor_ppm=76 min_hits=46
  bars=618296 needed=47 floor_ppm=76 min_hits=46
  ```
  Expected `min_hits = 47` (the count the doc names). Actual: 46 on every bar count tried. The fix is a ceiling division in `statistical_floor_ppm`, or passing hits directly. Either one changes `support_ppm` and therefore run identity, so it needs a decision entry.

### clib-2 (low): `Cadence::trades_over` floors weeks before multiplying by the cadence, contradicting its constant's "divides out once, at the end" and lowering the cadence support floor by up to `n-1` trades

- **Where:** `crates/cli/src/lib.rs:13812-13820`, consumed by `cadence_floor_ppm_of` (lib.rs:13878-13887, public).
- **Code:**
  ```rust
  let weeks = months.saturating_mul(WEEKS_PER_MONTH_CENTI) / 100;   // floored FIRST
  match self {
      Self::PerWeek(n) => weeks.saturating_mul(n).max(1),
      Self::PerYear(n) => (weeks.saturating_mul(n) / WEEKS_PER_YEAR).max(1),
  ```
- **Why it is wrong:** The doc on `WEEKS_PER_MONTH_CENTI` (lib.rs:13745-13752) says: *"Scaled by a hundred it stays exact through the multiply and divides out once, at the end, where a single rounding is visible."* The code divides by 100 before the multiply by `n`, so the fractional week (up to 0.99) is dropped and then multiplied. The PerYear arm's own "MULTIPLY BEFORE DIVIDING" comment is half true for the same reason. The cadence floor is the support threshold an operator states as "N trades a week", and it comes out lower than stated. On short spans the gap reaches 7%.
- **Repro (probe ran):** the real `cli::cadence_floor_ppm_of`, compared against the single-rounding order the doc promises (`months*435*n/100`, then `*1e6/bars`):
  ```
  bars=6300 months=1 PerWeek(10): got_ppm=6349 single-rounding_ppm=6825
  bars=6300 months=1 PerWeek(5): got_ppm=3174 single-rounding_ppm=3333
  bars=510300 months=81 PerWeek(3): got_ppm=2069 single-rounding_ppm=2071
  bars=75600 months=12 PerYear(52): got_ppm=687 single-rounding_ppm=687
  ```
  Expected 6825 ppm for 10/week over one month (43 trades). Actual 6349 (40 trades). `PerWeek(1)` is unaffected (n=1), which is why the doc's 81-month/352-trade example still holds. Changing the order changes `support_ppm` and so run identity for `n>1`. The variant doc says PerWeek is "bit-identical" to history, so the fix is either a decision entry or a corrected constant doc. One of the two must give.

### Already reported this pass (not new): truncated `bar_milli` vs rounded `t_milli`

`lib.rs:17894` `let bar_milli = (bar * 1_000.0) as i64;` is compared in `api::livejson:229` against `Edge::t_milli()` (rounded, `runner/src/outcome.rs:1574`). Re-confirmed with the real `bonferroni_t` and `Edge::t_milli`, with `t = bar - 0.0004`:
```
n=1 bar=1.959964 bar_milli=1959 t=1.959564 t_milli=1960 report_clears=false live_clears=true
n=12393 bar=4.609595 bar_milli=4609 t=4.609195 t_milli=4609 report_clears=false live_clears=true
n=54895691 bar=6.124302 bar_milli=6124 t=6.123902 t_milli=6124 report_clears=false live_clears=true
disagreements: 7
```
This is the same defect as run1-3 (also reproduced in xcut). It is listed here only for cross-reference.

## Checked and clean

- All 20 `as` casts in production: `i128→i64` per-trade (7108, bounded by the numerator), `usize→u64`/`u32→usize` (8359, 8541, 10454-10506, 12050-12149, 23016), `usize→u128` (10303). All are widening or provably in range on 64-bit.
- `quality_block` reward ratio (7169) uses an i128 intermediate. `rupees`/`as_percent`/`grouped`/`ppm_as_percent`/`hundredths_of` use `unsigned_abs` (no `i64::MIN` panic) and are render-only truncation. `bp_as_percent` (13076) would drop the sign for -1..-99 bp, but every caller passes non-negative rule fields (knob parse refuses negatives; derived floors are ≥0). Unreachable, so not reported.
- `ppm_to_points_at` (1572): truncating to integer paisa before half-away rounding cannot cross the 50-paisa boundary (floor(x) ≥ 50 iff x ≥ 50). Correct on both signs. `points_to_ppm_at` (1544) floors by design, with a saturating multiply.
- Every division site is guarded: `checked_div`/`saturating_div`/`div_ceil`/explicit zero tests at 4243, 4543, 4884, 4913, 4997, 5057-5059, 5464, 10304, 10863, 10916, 11391, 13189, 13357, 13893, 16512, 16801, 18370. No division by zero is reachable.
- `base_win_rate_bp` (10839) and `hold_return_over_drawdown_bp` (10894): checked arithmetic and `None` on empty/flat input. A ratio that rounds to zero is floored at 1 (documented).
- Percentile helpers (`range_percentile`, `window_range_percentile`, `grid_step_ppm` median) place one index with `select_nth_unstable`, and empty input is refused first. Already in docs/06-limits (D-1116).
- Float use in production is confined to `side_of_evidence` (NaN reads as Long, and a NaN mean cannot clear the bar), `{:.2}` rendering of thresholds (deterministic `Display`), and the `bar_milli` cast above. No f64 is digested or stored as a statistic in this file.
- `cap_within_budget`: u128 throughout, quantised to a power of two. The wall-clock dependence is already stated in its doc.
- `rungs_within_cell_budget`: `whole_machine_ceiling()/1024/threads` cancels the core count exactly for the default ceiling (9,362 cells at 1 and 14 cores), so the grid width is machine-independent there.
- Release `overflow-checks = true`. Raw `+`/`*` sites (1582, 7169, 9718, 10966, 12149, 13893) are bounded by their operands.
- Hot paths: `dropped.contains` (898, 1040) is bounded by `ATTEMPTS = 64`. The `EVERY_RUNG.iter().find` sites run once per request over a fixed table. The tier cascade's repeated full re-screens are documented (D-0289, D-1190). Per-slice `reference_price`/percentile recomputation per `screen` call is O(bars) per tier, not per candidate. Every bar loop (`session_index`, `validate_one_minute_execution`, `signal_spacing_minutes`, `span_checks`) is a single pass.
- Look-ahead: lib.rs builds the legacy in-sample screen's exit-ladder geometry (stop rungs, step, `reference_price` midpoint) from whole-span statistics. That is the grid of hypotheses an in-sample screen searches, not a per-bar decision. The validated V6 path is governed by EG-01 (training distribution only, no OOS derivation), which lives in `runner::exit_grid_policy`, outside this slice. Not reported.

# run2: numeric / look-ahead / O(1) audit of `runner::grid` and `runner::exit_grid_policy`

Commit 331b05c (origin/final/all-fixes). Audit only; nothing under /home/claude/brutex was modified.
Probes: `$SCRATCH/probes/run2/` (`src/main.rs`, `src/bin/oprule.rs`), built with `CARGO_TARGET_DIR=$SCRATCH/target`.

## Verdict

The fill model holds up. Entry is at the execution bar's open (best case) or its printed adverse extreme (worst case). A level exit is filled at its level, or at the open on a gap. Stops and trails slip to the bar's adverse extreme in the pessimistic reading. When a stop and a target fall in the same bar, the pessimistic reading takes the minimum over the orders that could actually have filled. Money is i64 paisa throughout, and an envelope check refuses any input big enough to saturate a sum.

I found two new defects, both low severity:

1. **run2-1.** A stop level and its trigger test disagree by one paisa. A bar that prints the stop price exactly does not trigger the stop.
2. **run2-2.** The V1 `OperatorRule` selector has no `wins > 0` guard. The grid's own `by_reward_to_risk` selector has one.

hunt-runner-1 (a fixed target first touched on the horizon time-exit bar still beats the time exit) is still in the code (`ExitChoices::of`, grid.rs:5100-5145). It is known, so it is not repeated here.

## Findings

### run2-1 · low · crates/runner/src/grid.rs:5219-5283 (`paisa_of`, `level_price`) against crates/runner/src/excursion.rs:1185, 1462 (crossing test)

```rust
fn paisa_of(ppm: Ppm, price: i64) -> i64 {
    let scaled = i128::from(ppm).saturating_mul(i128::from(price)) / 1_000_000;   // floor
...
fn level_price(level: Level, anchor: i64, side: Side) -> i64 {
    let distance = paisa_of(level.ppm, anchor);
    ... (Resting::Stop, Side::Long) | (Resting::Target, Side::Short) => anchor.saturating_sub(distance)
```
The crossing test is `ppm_of(move, entry) >= rung`, where `ppm_of` floors (`... * PPM_ONE / entry`). It fires only when `move >= ceil(r*e/1e6)`. The grid then places and fills the resting order at `e - floor(r*e/1e6)`.

**Why this is wrong:** when `r*e` is not a multiple of 1e6 (almost always the case), a bar whose low equals the stop price exactly prints the price the engine says the order rests at. The engine still does not fire the stop. A stop-market order triggers on a touch, so the position is held past a stop that had already triggered.

The doc comment on `level_fill` proves only one direction: the level is always inside the extreme when the stop fires. It never covers the other direction. The same floor/ceil gap applies to short stops and to trailing stops, since `BarMoves::retreat` uses `ppm_of` and the fill is `anchor - paisa_of(ppm, entry)`. For targets the gap is on the conservative side (a touch does not fill a limit order).

The effect is a 1-paisa band per level. The bias is optimistic whenever the path recovers after the untriggered touch.

**Repro, PROBE RAN** (`probes/run2/src/main.rs`): `synthetic::sessions(8)`, `ConditionMask::default()`, `Horizon::bars(15)`, long. Take the first baseline trade and pick a stop rung below every low on its path. Set the low of the bar at offset 5 to the grid's stop level, then to one paisa below it, and run `grid::per_trade` with `Chosen{stop:Some(0)}`:
```
baseline first trade: entry_bar=1877 exit_bar=1892 entry_open=2502006 min low on path=2501886
stop rung r=87 ppm; r*e/1e6=217.674522; grid stop level = e - floor = 2501789
bar 1882 (offset 5) low == level (touches the stop price exactly): open=2502021 high=2502081 low=2501789 close=2501946
  trade entry_bar=1877 exit_bar=1892 worst=-75 best=45 | cell stopped=0 timed_out=68
bar 1882 (offset 5) low == level - 1: open=2502021 high=2502081 low=2501788 close=2501946
  trade entry_bar=1877 exit_bar=1882 worst=-278 best=-217 | cell stopped=1 timed_out=68
```
- **Expected:** when the low equals 2,501,789, the stop fires on bar 1882, because that is the price the engine itself fills at (`best = 2501789 - 2502006 = -217`).
- **Actual:** the stop does not fire. The trade runs to the time exit and books −75 instead of −278 in the pessimistic reading.

**Fix direction:** place the level at `e - ceil(r*e/1e6)`, or make the trigger test `move >= floor(r*e/1e6)`, so the order price and the trigger agree.

### run2-2 · low · crates/runner/src/exit_grid_policy.rs:2460-2466 (`OperatorRule` selector), with grid.rs:592 (`reward_to_risk_bp`)

```rust
ExitGridSelectorV1::OperatorRule => admitted.max_by_key(|cell| {
    (cell.reward_to_risk_bp(), cell.pessimistic, crate::grid::merit(cell))
}),
```
`reward_to_risk_bp` returns `i64::MAX` whenever `worst_trade >= 0`, and that includes a cell with zero wins whose trades were all exactly flat. Admission (`resolved_grid_view.rs:405-430`) requires a stop, a target and `trades > 0`, but it never requires `wins > 0`.

grid.rs documents this exact trap at `Grid::by_reward_to_risk` (grid.rs:1057-1077: *"a cell firing twice with two winners would otherwise outrank every cell that has actually been tested"*). That selector guards with `c.wins > 0 && c.trades >= min_trades` and breaks ties on `trades`. The V1 selector has neither guard nor the tie-break. So:
- a winless all-flat cell ranks at `i64::MAX`, above every cell that has a loss;
- among cells that never lost, the tie-break ignores sample size.

Within one combination the cells share a candidate set, so very different trade counts are bounded by exclusivity, and an all-flat winless cell needs every pessimistic P&L to be exactly 0. That is why the severity is low. Still, the selected cell's `reward_to_risk_bp` comes out as `i64::MAX`, which is not a measured ratio.

**Repro, PROBE RAN** (`probes/run2/src/bin/oprule.rs`): the probe resolves a real `ExitGridPolicyV1` (OperatorRule, NIFTY, `synthetic::sessions(2)`) through `resolve_attested`, then calls the public `admits` and `reward_to_risk_bp` on crafted cells at an admitted stop/target pair. It applies `select`'s key exactly as quoted above. `select` itself was not called, because `EvaluatedExitGridV1.grid` is private.
```
resolved stops=[24] targets=[44, 54] pairs=2
A 1 trade, 1 win +5: admits=true reward_to_risk_bp=9223372036854775807 key=(9223372036854775807, 5, ..)
B 400 trades, min_win 3000 / worst -1000: admits=true reward_to_risk_bp=300 key=(300, 800000, ..)
C 50 trades, 0 wins, all flat: admits=true reward_to_risk_bp=9223372036854775807 key=(9223372036854775807, 0, ..)
OperatorRule key (select: max_by_key((reward_to_risk_bp, pessimistic, merit))) over [B, A] picks trades=1 pessimistic=5
grid::Grid::by_reward_to_risk(min_trades=30) over the same two picks trades=Some(400)
```
- **Expected:** the operator's 1:3 rule is answered by a tested cell, and a winless cell (C) is never admitted as "best risk-reward".
- **Actual:** both A and C outrank B.

## Checked and clean

- **Entry fills (`entry_fills`, grid.rs:1707).** Worst = printed high for a long and low for a short, via `costs::fill` `PrintedExtreme`. Best = open. Both come from the execution bar, which is after the signal (that is trade.rs's concern; the grid reads only `path.entry_bar`). A bracket violation refuses with a 0 sentinel.
- **Market exit fills (`exit_fill`).** Best = open, worst = printed adverse extreme. The legs swap correctly between long and short.
- **Same bar touches both stop and target (`read_trip`, `ExitChoices::of`).** A gap at the open owns the exit. Of the stop and the pre-bar trail, only the nearer is reachable. A raised-peak trail is unreachable when a fixed target is selected. Pessimistic = minimum over reachable orders, then re-priced at `entry_pess` with stop slippage. `ordered()` enforces pessimistic <= optimistic. This is the pessimistic resolution.
- **Stop slippage (`stop_slippage`).** It is applied to gapped stops too, and to trails. Targets are not slipped, which is correct for limit orders.
- **Trail pricing.** Give-back is measured on the entry-open basis, matching the crossing basis. The pre-bar peak is the pessimistic anchor. Arming takes effect on the next bar.
- **Holding period.** `bars_held += pess_off`: a time exit at horizon H counts H, and an exit on the entry bar counts 0. This is consistent with execution-bar offsets.
- **Exclusivity.** `blocks_without_pricing` refuses a next entry at or before the previous exit bar, and only a hole at or before the exit un-prices a path (D-1183 holds in the code).
- **Overflow.** `money_envelope_fits`: the 4·A·P bound covers the pessimistic/optimistic totals (≤2A per trade), `fill_cost` (≤4A), and drawdown (≤4A·P). `paisa_of`/`peak` use an i128 intermediate. The V1 `validate_envelope_extremes` checks money, excursion and level-price bounds. The `exit_grid_policy` resolution arithmetic is fully checked: percentile ranks, ratio cross-products, cell count, coordinate width, bitmap slots.
- **Percentiles (`resolve_axis_with_forced`).** Nearest-rank `ceil(n·p)−1` on a sorted sample. A zero rung refuses. The forced stop is inserted in one pass.
- **`encode_observed`.** Open > 0 is validated, and floor/ceiling are exact integer operations.
- **Training-only resolution.** `observed_ranges` reads only the training series and skips bars at or after 15:10 IST. No OOS data enters it.
- **Wilson `assurance_bp`.** The only float. It is a statistic over counts, clamped before the cast and floored. `trades_needed_for` is O(ceiling), but it runs once per CLI run, not in a per-bar or per-candidate loop.
- **`win_rate_bp`, `profit_factor_bp`, `avg_*`, `edge_ratio`, `return_over_drawdown`, `guaranteed_floor` (i128).** Division-by-zero guards are present and saturation is explicit.
- **`mean_excursions`.** The ceiling on `all_mae` is the safe direction for a denominator.
- **Per-candidate cost.** `one_variant` makes O(1) indexed reads into `Crossings` per candidate per cell. Two items are not O(1), and both are documented as known: `evaluate_timed` pass-two `peak_*` scans (2·trades·span, no variants factor; the grid.rs:1765-1830 doc and docs/06-limits) and `money_envelope_fits` (O(Σ span), o1eng2 row).
- **Selectors (`select`, `best`, `by_reward_to_risk`).** `max_by_key` on integer keys only, with no float or NaN in any ranking key. Ties resolve to the last maximum, which is deterministic.
- **Known items, re-checked and unchanged.** hunt-runner-1 (target on the horizon bar) is still present. errpaths-3 (`unwrap_or_default` on an invalid caller stop ladder, grid.rs:2131) is still present. Neither is re-reported.

# Numeric audit, pass 9: trade simulation correctness

Head: `1f4de71` (origin/final/all-fixes-zero), read-only checkout `/home/claude/wt/zero3`.
Scope: `crates/runner` (trade.rs, excursion.rs, grid.rs, exit_grid_policy.rs, research_exit_grid.rs, signal_candle_stop.rs, closed.rs) and `crates/costs/src/fill.rs`.
Method: source reading only. All three findings are low severity, so per the shared rules no cargo test was run. Each repro below is a hand trace through the quoted code.

## Verdict

The entry, hold and exit model holds up on every scenario that decides the pessimistic (ranked) figure. There is no look-ahead, no overnight hold, no exit invented on a cut slice or a holiday, no price that the bar did not print in the pessimistic reading, and long and short are mirrored.

Three new low findings:

1. **p9num-1:** on the time-exit or square-off bar, a stop or trail touched only after that bar's open still owns the exit. That bar's time exit fills at the open, so the position was already flat. The optimistic reading and the exit counters are wrong. The pessimistic reading is unaffected.
2. **p9num-2:** the 15:10 square-off is priced at the 15:09 bar's OPEN in trade.rs and grid.rs, but at its CLOSE in signal_candle_stop.rs and outcome.rs. The same policy gets two prices one minute apart.
3. **p9num-3:** grid.rs prices a same-bar `fills_at(bar, bar)` that also checks the bar's low against the tick, on a leg that never uses the low. A bar trade.rs accepted is then silently mispriced: a 0 entry for a long, and a flat pessimistic exit for a short.

Known items run2-1 and the hunt-runner-1 optimistic half are not re-reported.

## Table

| scenario | code path | behaviour | expected | verdict |
|---|---|---|---|---|
| Entry timing: signal at bar N close | trade.rs:957-1012 `step` (Signal: +1, Fill: 0), `immediate` check `signal.ts + step == entry.ts` | Entry is bar N+1 (or the reprojected minute stamped at N's close). Best is its open; worst is its printed high (long) or low (short). The column bit at N is folded from bars 0..N only (Column::build streams). | Next-bar fill, no look-ahead | CLEAN |
| Entry must be the same IST day as the signal | trade.rs:1103-1112 `same_session` | A signal on the last bar of a day is `too_late`, not filled next morning | No overnight entry | CLEAN |
| Entry at or after the last fillable minute | trade.rs:1013-1030; SessionBounds `fillable` (open+step <= 15:10); exit <= entry is refused (:1185) | The last entry is 15:08, with a one-bar hold to 15:09 | Refuse entries that cannot exit same day | CLEAN |
| Horizon in time, not in bar index | trade.rs:1150-1156, `horizon_bar` :1472-1492 (exact-timestamp lookup) | A missing deadline minute gives block-only occupancy, never the "last bar before the gap" | No retroactive or stretched hold | CLEAN |
| Minute missing inside a trade | trade.rs:1192 `path_accepts` (refused prefix plus broken-step prefix); grid `blocks_without_pricing` :4259-4286; signal_candle_stop :1224 `expected` | The path is not priced, and it blocks to the time exit. No price is read across the hole. | The trade never sees a price it never had | CLEAN |
| Holding across days or a holiday | trade.rs `actual_square_off` :1494-1522 (same IST day, deadline 15:10); `forced.real` needs a unique accepted 15:09 | Every hold ends the same day, and an overrun with no 15:09 row is dropped | Intraday only | CLEAN |
| Last bar of a month or file (cut slice) | trade.rs:1157-1184 (`forced.real == false` gives occupancy only, `too_late`) | A cut tail is refused, not stamped `forced` | No invented square-off | CLEAN |
| Forced exit price, 15:10 policy | trade.rs:1154-1156 to round_trip :1346 (`Anchor::Open` on the 15:09 bar); grid `exit_fill` :1773-1797; signal_candle_stop :1272 (`bar.close`) | trade.rs and grid.rs exit at the 15:09 OPEN (best) or LOW (worst). signal_candle_stop and outcome exit at the 15:09 CLOSE. | One price for one policy | **p9num-2** |
| Stop and target in one bar | excursion.rs:1180-1206 (`ambiguous`), grid.rs `read_trip` :4053-4180, `ordered` :5280 | Gap-at-open first. Only the nearer adverse level is reachable. The pessimistic reading is the minimum over reachable orders, re-charged at `entry_pess`. Counted in `ambiguous_bars`. | Pessimistic order applied and labelled | CLEAN |
| Order touched only on the time-exit bar | grid.rs `ExitChoices::of` :5168-5214 | Stop or trail: `timed = None`, exit counted `stopped`, optimistic = stop level. Target: D-1541 keeps the target as an optimistic choice. | Time exit at the open owns the bar unless the open itself crossed a level | **p9num-1** |
| Gap through stop at the open | grid.rs `level_fill` :5388-5409 (`rested_at_open && opened_through` gives the open, gapped); `stop_slippage` :5478 gives the bar's adverse extreme in the pessimistic reading | Optimistic fills at the open. Pessimistic fills at the low (long) or high (short). Not at the stop level. | Fill at the open or worse | CLEAN |
| Stop exactly equal to high or low | grid.rs `paisa_of` :5288 (floor) and excursion.rs `ppm_of` :1484 (floor) | A touch at the exact level does not fire | Fire on touch | NOT FIXED (run2-1, known) |
| Same, signal-candle stop | signal_candle_stop.rs:1231 `bar.low <= stop` | Inclusive | Fire on touch | CLEAN |
| Trailing stop on the bar that raises the peak | excursion.rs `BarMoves::of` (retreat from the pre-bar peak), `cross_rungs`, `trail_ambiguous`; grid `Trailing::ended` | Fires only if the pre-bar peak retreat reaches the rung. The raised-peak anchor is priced as the optimistic alternative. It is unreachable when a target is selected. | No same-bar favourable-first assumption | CLEAN |
| Trailing take-profit armed on the target bar | excursion.rs:1214-1217 (arming after `cross_armed_rows`) | Arms for the next bar only | No same-bar arm-and-fire | CLEAN |
| Long and short symmetry | `BarMoves::of`, `peak_ppm_at`, `adverse_ppm_ceil_at`, `level_price`, `stop_slippage`, `entry_fills`, `exit_fill`, `fills_at` (`Direction::Short` gives buy = exit.high, sell = entry.low) | Every pair is mirrored. MAE for a short is high − entry, and MFE is entry − low. | Mirrored | CLEAN |
| Tick rounding of levels | `paisa_of` floor on the `anchor ± distance` level; no snapping | Levels are integer paisa. The CLAUDE.md §7 grid is 2 dp (1 paisa), so a level is on-grid. Snapping happens only at the store write boundary. | Half-up at the write boundary only | CLEAN (the floor-vs-trigger part is run2-1) |
| ppm stop on a tiny price | `paisa_of(500 ppm, 1000 paisa) = 0` puts the level at the entry. The trigger needs a move of at least 1 paisa. | The stop books 0 optimistic and slips to the low in the pessimistic reading | Distance 0 is not hidden | CLEAN (run2-1 class, not new) |
| Overflow in P&L | trade.rs `pnl` :1396 (both legs >= TICK, so `sell - buy` cannot overflow); grid `money_envelope_fits` :2414 refuses 4·A·P > i64::MAX; signal_candle_stop `checked_sub` | Per-unit P&L. Sums are refused rather than saturated. | No silent clamp | CLEAN |
| Same-bar fill legs on a sub-tick low | grid.rs `entry_fills` :1733-1759, `exit_fill` :1773-1797 vs trade.rs `round_trip` :1344-1352 | trade.rs prices the trip. The grid's entry falls back to 0 (long) and its pessimistic exit falls back to flat (short). | Same refusal set in both halves, or price the leg actually used | **p9num-3** |
| One position at a time | trade.rs:1003 (`entry <= open_until` is blocked); grid :4271 | An exit minute is never a re-entry minute | Strict | CLEAN |
| Refused or duplicated bar on the path | trade.rs `facts.accepts`; excursion `admit_located` (first_refused); grid D-1183 | A hole at or before the exit un-prices the trade. A later hole does not. | Causal | CLEAN |
| closed.rs | itemset closure, not trade logic | n/a | n/a | out of theme |

## New findings

### p9num-1 (low): an order first touched inside the time-exit (or square-off) bar still owns the exit, although the time exit filled at that bar's open

`crates/runner/src/grid.rs:5175-5212` (`ExitChoices::of`):

```rust
let stop_fired = stop_at != NEVER && stop_at <= chosen;
...
let no_order = !stop_fired && !target_fired && trail_before.is_none();
let deadline_target =
    on_time_exit_bar && target_fired && !stop_fired && trail_before.is_none();
...
    timed: (no_order || deadline_target).then_some(Ended::Time),
```

The crossings path is inclusive of the time-exit bar (`crossings_checked(bars, path.entry_bar, path.exit_bar, ..)`, grid.rs:2328). D-1541 states the governing fact: "that bar's time exit fills from its OPEN, the deadline price". The open is the bar's first print. So a level that is not already crossed AT the open can only be touched after the position was closed.

The code handles this only for a lone target, and only in the pessimistic reading. For a stop or a pre-bar trail first touched inside that bar, `timed` is `None` and the trade is attributed to the stop:

- `count_exit` increments `stopped` (or the trail counter) instead of `timed_out`.
- The optimistic reading is the stop level, not the open.
- `uncertainty` and `bracket` shrink.

The pessimistic figure is unchanged, because both routes price the bar's low.

The target half is the mirror case. D-1541 deliberately kept "the optimistic reading may still book the target", and that books a limit fill after the position was already flat.

Repro (not run). Long, h = 2, stop rung 1,000 ppm, entry open 100,000:

| offset | open | high | low |
|---|---|---|---|
| 0 | 100,000 | 100,050 | 99,950 |
| 1 | 100,000 | 100,050 | 99,950 |
| 2 (time exit) | 100,020 | 100,030 | 99,800 |

- At offset 2, `mae = ppm_of(200, 100000) = 2000 >= 1000`, so `stop_at = 2 = span` and `ExitChoices = {stop}`.
- `read_trip`: the level is 99,900, inside the bar, and the open is not below it, so the optimistic reading is 99,900 − 100,000 = −100. The trade is counted `stopped`.
- Correct: a time exit at the open gives an optimistic reading of +20, with `timed_out` counted.
- Pessimistic: both routes give 99,800 − 100,050 = −250.

Fix: when `on_time_exit_bar`, keep a level attribution only if it gapped at the open (`level_fill(..).1` with `rested_at_open`). Otherwise use `Ended::Time` alone. This also replaces D-1541's optimistic-target allowance, and needs a decision entry.

### p9num-2 (low): the 15:10 square-off has two prices, the 15:09 open in trade.rs and grid.rs and the 15:09 close in signal_candle_stop.rs and outcome.rs

`crates/runner/src/outcome.rs:60-62` says the 15:09 bar "is the only stored OHLCV record that can price this deadline", and :77-78 says it is "the only opening minute whose one-minute close prices the fixed forced exit". `signal_candle_stop.rs:1272` does that:

```rust
(ExitReason::Forced1510, bar.close, bar.close, false)
```

`trade.rs:1154-1155` sets `exit = forced.bar` (the 15:09 row). `round_trip` (:1346) then prices it with `Anchor::Open`, which is the 15:09 OPEN, a print at 15:09:00. The worst reading is the 15:09 low. `grid::exit_fill` (:1773-1781) does the same.

So in the walk and the grid, the "15:10 policy deadline" (trade.rs:20-23) is actually a 15:09:00 exit. A horizon whose deadline is exactly 15:10 is priced identically to one whose deadline is 15:09. The two execution models of the same product give different P&L for the same forced trade.

The best reading is also not an upper bound for the 15:10 price. For a long, the true deadline price is the close, which can sit above the open.

The pessimistic reading stays conservative, because the low is at or below the close for a long and the high is at or above it for a short. That is why this is low severity.

Repro (not run). Long entered 15:00 with h = 15, so the deadline 15:15 is past 15:09 and `forced.real` holds. The 15:09 bar is O 100, H 110, L 99, C 109:

- trade.rs reports best exit 100 and worst exit 99.
- signal_candle_stop (and outcome) exit at 109.

Fix: choose one convention and state it in both modules, with a decision entry. Either price the forced leg at the 15:09 close in trade.rs and grid.rs (best = worst = close, as signal_candle_stop does), or document in trade.rs and outcome.rs that the square-off is executed at 15:09:00.

### p9num-3 (low): grid.rs re-prices the entry and exit legs with a same-bar `fills_at(bar, bar)` that also tick-checks the unused low, and falls back silently where trade.rs priced the trade

The grid code:

```rust
// grid.rs:1746-1753 (entry_fills)
let Ok(fills) = costs::fill::fills_at(fill_bar, fill_bar, direction_of(side),
    costs::fill::Anchor::PrintedExtreme) else {
    return (0, open);
};
// grid.rs:4960-4961 (realised, time exit)
let exit = exit_fill(bars, entry.saturating_add(offset), side, pessimistic)
    .unwrap_or(fills.charged);
```

`fills_at(.., PrintedExtreme)` refuses when the sell leg is below `TICK` (fill.rs, `sell.raw() < TICK.raw()`). For the same bar on both legs, the sell leg is always `bar.low`. That is wrong in two places:

- **Long entry.** The grid wants the buy, which is the high. `trade::round_trip` checks only `entry.high` and `exit.low`, so it accepts an entry bar with a positive low of 1-4 paisa. `Candle::check_evaluable` only requires `low > 0`. The grid then gets `entry_pess = 0` on a `priceable` candidate:
  - The pessimistic reading is charged at entry 0, then clamped to the optimistic one by `ordered`.
  - `worst_mae` and `adverse` come out 0, because `ppm_ceil_of(.., entry <= 0)` returns 0.
  - `fill_cost` can go negative, which breaks the "non-negative by construction" claim (grid.rs:191-193).
- **Short time exit.** The grid wants the buy, which is the high, but checks the exit bar's low. `round_trip` never reads the exit low for a short. The pessimistic exit falls back to `fills.charged`, so the trip is booked flat (or clamped to the optimistic reading) instead of entry low − exit high.

Both are fallbacks that hide a failure (CLAUDE.md §4). They also contradict a prior pass's note that these fallbacks are unreachable because refused bars are excluded upstream: these bars are not refused upstream.

Repro (not run). Long, entry bar O 1,000, H 1,000, L 3, exit bar O = H = L = 900:

- trade.rs: worst = 900 − 1,000 = −100.
- grid baseline cell: pess = min(900 − 0, opt = −100) = −100 by luck of the clamp, but `worst_mae = 0` against a true 997,000 ppm.

Short, entry O = H = L = 1,000, exit bar O 1,100, H 1,100, L 3:

- trade.rs: worst = 1,000 − 1,100 = −100.
- grid time-exit pessimistic: 0, clamped to −100. With an exit open of 950 instead, the optimistic reading is +50 and the pessimistic reading is booked 0 against a true −100.

Fix: take the needed leg directly from the validated `Bar` (`high` for a buy, `low` for a sell, each checked against `TICK`). Or mark the candidate `block_only` when either leg refuses, so the grid and trade.rs share one refusal set.

## Known items re-checked at 1f4de71

| id | state | evidence |
|---|---|---|
| run2-1 | NOT FIXED | grid.rs:5288 `paisa_of` floors; excursion.rs:1484 `ppm_of` floors |
| hunt-runner-1 | FIXED for the pessimistic reading (D-1541) | grid.rs:5205-5212. The optimistic half is folded into p9num-1. |
| p3floor-2 | FIXED (still holds) | grid.rs:4510 `adverse_ppm_ceil_at` |

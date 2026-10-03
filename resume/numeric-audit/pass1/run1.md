# run1: runner PnL and return arithmetic (outcome.rs, grid.rs Cell aggregates)

Commit 331b05c (origin/final/all-fixes). Audit only.

**Verdict.** `outcome.rs` holds up: no float in a price, no overflow in the sums, no NaN reaching a ranking, and the short side is read correctly. I found three new defects, all low or medium, in the per-variant aggregates `grid::Cell` and in the milli-scaled `t` that `outcome` hands to `cli`. All three come from one root: a flat trade is counted as a loser, or a rounded statistic is compared against a bar that was cut down to an integer instead of rounded. `outcome::Edge::payoff_bp` already fixed the flat-counting defect for the edge ("COUNTED, NOT SUBTRACTED"). `Cell` was never given the same fix.

Probe: `$SCRATCH/probes/run1` (path dependency on `crates/runner`), built and run with `CARGO_TARGET_DIR=$SCRATCH/target`. Verbatim output:

```
avg_loss with 9 flats = -10 ; without flats = -100
avg_win/|avg_loss| with flats = 50 ; without = 5
guaranteed_floor with flats = -500 ; without = 400
trials=1000 bar=4.0556270 bar_milli=4055 t=4.0545500 t<bar=true t_milli=4055 clears_bar(t_milli>=bar_milli)=true
trials=61000000 bar=6.1410705 bar_milli=6141 t=6.1405500 t<bar=true t_milli=6141 clears_bar(t_milli>=bar_milli)=true
trials=123456789 bar=6.2520884 bar_milli=6252 t=6.2515500 t<bar=true t_milli=6252 clears_bar(t_milli>=bar_milli)=true
```

## Findings

### run1-1 (medium): `Cell::guaranteed_floor` charges every flat trade the worst loss, so the floor drops below the worst arrangement it claims to be
- `crates/runner/src/grid.rs:639-643`
  ```rust
  let losses = self.trades.saturating_sub(self.wins);
  let worst_loss = self.worst_trade.saturating_neg().max(0);
  let gain = i128::from(self.wins).saturating_mul(i128::from(self.min_win));
  let pain = i128::from(losses).saturating_mul(i128::from(worst_loss));
  ```
- `wins` counts only `pess > 0` (`grid.rs:4442`). `tally_trade` books `pess == 0` on the loser branch (`grid.rs:4551-4627`; the test at `grid.rs:6247` states "A FLAT TRADE IS A LOSER"). That makes `losses` equal to losers plus flats, and each flat is charged `worst_loss`.
- The doc (`grid.rs:600-631`) says the floor is "every winner its smallest and every loser its largest" and "the total that could not have been undercut given the same wins, losses and extremes". A flat trade earned 0 and cannot be re-ordered into a loss, so the true worst arrangement is `wins*min_win - real_losers*worst_loss`. The figure returned is lower than that and is not reachable.
- Effect: `Grid::best_clearing` and `Grid::best_under` (`grid.rs:1138`, `:1165`) pick the variant by `guaranteed_floor` first. `bound::Bound::admits` (`bound.rs:289`) refuses with `FloorShort` when the floor is below `min_floor`. A variant with many flat exits, such as time exits on 1-minute bars, is therefore ranked down or refused for trades that lost nothing.
- Repro (probe above): `Cell{trades:11, wins:1, min_win:500, worst_trade:-100}` with 1 loser and 9 flats gives `guaranteed_floor = -500`. The true worst arrangement is 500 - 100 = +400, which is also what the same cell without the flats gives. With `min_floor = 0` the bound refuses a variant whose true floor is +400.
- How often `pess == 0` occurs on real data was not measured (INFERRED). The pessimistic fill brackets with a tick on each leg, so zero needs the bar prices to line up.

### run1-2 (low): `Cell::avg_loss` divides by losers plus flats, so it reports a smaller loss than the real average (the defect `Edge::payoff_bp` already fixed)
- `crates/runner/src/grid.rs:557-562`
  ```rust
  let losers = self.trades.saturating_sub(self.wins);
  if losers == 0 { return 0; }
  self.gross_loss.saturating_div(losers.cast_signed())
  ```
- The doc reads "Mean loss of the LOSING trades". `gross_loss` adds 0 for each flat but the denominator still counts it, so each flat shrinks the mean loss. `outcome.rs:1325-1334` documents this exact defect and fixed it for the edge payoff ("inflated this ratio by `(losses + flats) / losses`"). The copy in `Cell` was not fixed.
- Where it ends up: the "avg losing trade" line of `audit::strategy_report` (`audit.rs:851`), `cli::frontier` (`frontier.rs:3074`) and `/frontier.json` `avg_loss`, `population_admission_writer.rs:1311` `average_loss` (stored in `TopMetricsV1`), and `institutional_evidence.rs:321`.
- Repro (probe): 1 loser of -100 plus 9 flats gives `avg_loss = -10`, where the real figure is -100. Average win over average loss reads 50x instead of 5x.
- Because of the "flat is a loser" convention (decisions near `docs/05-decisions.md:28386`, invariant BT-22), you could argue `losing_trades` and the loser count are consistent by definition. Even so, the mean is not a mean loss once zeros are averaged in, and the edge-level code rejects this convention in so many words.

### run1-3 (low): the milli-scaled `t` is rounded but the bar it is compared to is truncated, so `clears_bar` can be true for `|t|` below the bar
- `crates/runner/src/outcome.rs:1574`: `let out = scaled.round() as i64;` (`t_milli` rounds half away from zero)
- `crates/cli/src/lib.rs:17894`: `let bar_milli = (bar * 1_000.0) as i64;` (truncates toward zero)
- `crates/api/src/livejson.rs:229`: `row.t_milli.saturating_abs() >= bar_milli` is published as `clears_bar`
- Any `|t|` in `[floor(bar*1000) - 0.5, bar*1000) / 1000` is shown as clearing the Bonferroni bar. That window can be up to about 1.5 thousandths wide. The comment at `lib.rs:17884-17890` argues "a unit in the last place of a threshold does not move a verdict that is 3.42 against 6.19". That is true for those two numbers and false at the boundary, which is the only place `clears_bar` matters.
- Repro (probe, formula copied verbatim): trials=61,000,000 gives bar=6.1410705 and bar_milli=6141. t=6.14055 < bar gives t_milli=6141, so `clears_bar` is true. Expected: false.
- Fix direction: compare at full precision in the crate that owns the floats, or use ceil for the bar.

## Checked and clean
- `outcome::forward_over` (`:769-935`): the return is close(exit) minus close(entry) on positive paisa, saturating. The exit is the exact-timestamp deadline or a proved square-off. Excursions run over `[i+1, exit]` from the entry close. There is no entry on a forced-exit bar and a refused path is marked rather than dropped. Neither the decision nor the entry reads bar N+1; the cadence look-ahead (`out[i]=out[i+1]`) is already documented (D-1410).
- `outcome::edge` (`:1892-2258`): Welford mean and M2 in f64 from i64 moves (exact below 2^53, with a dedicated test above it). `Sides` sums are i128 saturating. The `min_win`/`min_loss` zero sentinels are sound because only strictly signed values enter. `x.unsigned_abs()` is safe at `i64::MIN`. `n<2`, `m2<=0` and the all-in-one-window case each return t=0, and `newey_west_t` maps a NaN, infinite or non-positive SE to 0.
- `Edge::payoff_bp`, `path_ratio_bp`, `worst_reward_risk_bp`: the side comes from `mean_paisa < 0.0` (-0.0 counts as long, which is consistent). Division by zero is caught by guards and returns `i64::MAX` or 0. NaN is caught by `is_finite`. f64-to-i64 casts are clamped below 2^63: the 9.2233720368547750e18 literal rounds down to 2^63-1024, so the cast cannot overflow.
- `milli()`: NaN becomes 0 (known, attacksweep.md), and ±9e18 saturates.
- `grid::Cell::profit_factor_bp`, `reward_to_risk_bp`, `return_over_drawdown`, `edge_ratio`, `avg_win`, `avg_bars_held`, `assurance_bp` (Wilson bound, clamped before the cast): zero denominators are guarded, and `saturating_mul(100)` cannot wrap. The `i64::MAX` "never lost" sentinel is demoted by `cli::ranked`.
- `grid::paisa_of` and `peak`: i128 products, and `entry <= 0` is guarded before the divide. The floor in the fill and the ceil in the crossing differ by under 1 paisa, as already documented.
- `excursion::ppm_of`: i128 product, and non-positive entry or move returns 0.
- `trade::pnl`: `sell - buy` with legs assigned by `costs::fill`, which is correct for shorts with no second sign flip.
- `report::paisa` (rounding a mean for display), `report/audit::permille` (integer tenths, `whole==0` guarded).
- `rank::Scored`/`ByAsymmetry` use `total_cmp` with finite-first ordering, so a NaN `t` is ordered and never panics.

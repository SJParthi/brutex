# Pass 2 — slice p2idx

Commit 5140aca (origin/final/all-fixes-zero). Audit only. Nothing in the repo was edited. No probe was built: the one finding below runs through a helper that p2bool-1 has already probed, and its only new content is the call site. That is why it is labelled INFERRED.

Scope covered: cli index_stop.rs, index_stop_qualification.rs, index_stop_qualification_metrics.rs, index_stop_qualification_numeric.rs, index_stop_search.rs, index_stop_source_context.rs, index_stop_vix.rs, expression_pricing.rs, frontier.rs (the non-codec parts: Row::of, derived, verdict, append_all, block read), and stability.rs.

## Verdict

One new low finding. It is an extra call site of the p2bool-1 floor-vs-ceiling class that neither p2bool nor p2inst listed. It matters because the fix cannot go into the shared helper. One known item (D-0743) was confirmed still open at a site in this slice. No look-ahead and no in-sample/out-of-sample leakage were found. No per-candidate scan was found that could be replaced by an O(1) structure.

## Findings

### p2idx-1 — low — crates/cli/src/index_stop_qualification_metrics.rs:142, 174-185 (helper population_base_evidence_v2.rs:1570-1591)

```rust
let rate = |part, total, name| measured_rate(part, total, name).map_err(display);
...
win_rate_ppm: rate(m.wins, m.trades, "native win rate")?,                 // min-gated
ambiguous_fill_rate_ppm: rate(t.ambiguous, m.trades, "native fill bound rate")?,  // max-gated
gap_affected_rate_ppm: rate(m.stop_gaps, m.trades, "native stop gap rate")?,      // max-gated
session_concentration_ppm: rate(row.periods().iter().map(|p| p.trades).max().unwrap_or(0), m.trades, ..)?, // max-gated
largest_trade_profit_share_ppm: rate(t.max_win, t.gross_win, "native largest profit share")?, // max-gated
losing_trade_rate_ppm: rate(losses, m.trades, "native losing trade rate")?,  // max-gated (run3-1 class)
```

- **What happens.** `measured_rate` calls `rate_ppm`, which floors (`scaled.checked_div(denominator)`). These values go through `metrics::base`, then `numeric::project`, then `AdmissionPolicyV1::evaluate_research_projection`. That path ends at `check_max_u64`, which tests `value > ceiling` (runner/src/admission.rs:2891). The single-stop (index-stop) qualification therefore lets a share whose exact value is just above `max_largest_trade_profit_share_ppm` pass. This is the same class as p2bool-1.
- **The rounding.** `largest_trade_profit_share_ppm` divides one paisa sum by another, so the leak occurs at round ceilings. The three trade-count rates leak at non-round ceilings.
- **Why it is listed separately.**
  - p2bool-1 names population_base_evidence_v2.rs:586-594 and p2inst-1 names institutional_evidence.rs. Neither names this site.
  - p2bool-1's fix direction is to project with `div_ceil`. That cannot be done inside `measured_rate`: this same closure also produces the **min-gated** `win_rate_ppm` on line 162, and a ceiling there would open the opposite leak.
  - So the fix has to be made at each call site, and this file needs its own.
- **Repro (INFERRED, arithmetic of the helper that p2bool-1 probed, `$SCRATCH/probes/p2bool`).** Take a single-stop row with gross_win = 2,000,000 paisa and max_win = 400,001 paisa, under policy `max_largest_trade_profit_share_ppm = 200_000`.
  - Exact share: 200,000.5 ppm.
  - `rate_ppm` returns 200,000. Since 200000 > 200000 is false, the LargestTradeProfitShare reason is not failed.
  - Expected: failed bit 65536. Actual: not failed. This is identical to p2bool-1's probe line `best=400001 gross=2000000 share_ppm=200000 exact>ceiling(200000)=true ... failed: ReasonBits(0)`.
- **Fix direction.** Use `div_ceil` for the max-gated rates in this file, or apply a D-0743-style floor-hidden refusal. Either changes the qualification evidence bytes, so it needs a decision entry.

## Known, code matches documented status (not new)

- **PBO is projected by floor into a max-gated field.** At index_stop_qualification_numeric.rs:608, `values.pbo_ppm = ObservedU64V1::Measured(probability(bottom, contributing)?.ppm())`. `ppm()` floors (admission.rs:3007-3010), and `check_max_u64` tests `>`. 06-limits §D-0743 (line 10944) says: "The cli V1 builders that fill the same fields with `.ppm()` are not changed by D-0743." So this is the documented open item. Note the inconsistency inside this one file: the four p-values in this same `project` use `upper()` (`div_ceil`, :667-670), which is the safe direction, and only PBO floors. The leak needs bottom/contributing to fall in (c, c+1e-6]. Contributing is at most the split count (6,435 for 16 segments), so whether it occurs depends on the ceiling (INFERRED).
- **Flat trades are counted as losses** in `trade_facts`: losing streak, gross_loss/average_loss denominator, `losses = trades - wins`. This is the locked conservative rule (docs/05-decisions.md around line 28386: "only `pess > 0` is a win"). Population_base_evidence_v2 does the same. Not a finding.
- **VIX entry/exit-minute full-candle annotation** (index_stop_vix.rs `POLICY`) reads the full minute candle at the entry stamp. It is reference only and never enters ranking, identity or admission. It is already noted in pass1/ind2.

## Checked and clean

- **Training/later (OOS) separation.**
  - index_stop_search.rs:548-550 refuses `later.0 <= training.1`. numeric::validate_family refuses `first.last() >= after_first.first()`.
  - The later source is loaded from `training.0` (index_stop_search.rs:596-605). That is indicator warm-up: the catalog is evaluated only over `month_days(request.later)` (:450-459), and `daily()` concatenates the before and after periods without overlap.
  - CSCV/PBO uses the training family only. The bootstrap (RW/White/SPA), folds and base metrics use the later family only. Execution and calendar completeness also check `before`.
- **p-values.** `upper()` uses a u128 `numerator*1e6` and then `div_ceil` (no overflow for u64 inputs). `hypothesis_decision` uses the exact fraction. The policy ceiling is min'd with the allocation alpha (`with_search_probability_ceiling`).
- **CSCV.** `segment_sums` falls back to the per-period path when a row's absolute sum exceeds i64::MAX, so the result is exact. `split` refuses an observation outside both masks. Every money addition is checked.
- **Folds.** Widths `days/count + (index < days%count)` partition the span exactly. The final `at != len` refuses unassigned periods. All additions are checked.
- **Ratios and returns.** `ratio_observed` and `return_drawdown` floor, and they feed min-gated fields (conservative). drawdown == 0 with profit > 0 gives u64::MAX, which is consistent with the population builders. `worst_reward_risk_ppm` with no losses is Unmeasured (zero denominator), not a fake value.
- **Overflow.** `trade_facts`, `independent_sessions` and `weakest` use checked arithmetic. `weakest` returns Unmeasured when there are no trades.
- **stability.rs.**
  - `year_month` is Hinnant's algorithm, and `bucket` partitions IST time in monotone order.
  - `positive_share_bp` floors. It is not wired into any gate (grep: no caller outside stability.rs). The `min_weakest_bp` rule uses other code.
  - `at` is one O(T) pass, and a key going backwards opens a new bucket.
- **frontier.rs.**
  - `verdict` uses `>=` against floored runner Cell projections, which is conservative. Cell internals belong to the runner slices.
  - An unpriced row is reported as unchecked, not as passed. `stop_unchecked` and `protective_exits_unchecked` are carried explicitly.
  - `append_all` makes one O(rows) identity check. The block read is O(block) and is documented in 06-limits §108.
- **expression_pricing.rs.** `Plan::parse` refuses a non-positive step or rung count, `rungs*step` overflow, and grids over 100,000 cells. Knob overrides are validated and never defaulted. `words()` casts are lossless for the validated ranges.
- **index_stop.rs.** `prepare_column` converts the rung to seconds with try_from. `validate_catalog` deduplicates with one pre-sized HashSet insert per program (expected O(1)). `publish_vix` bounds records as `min(records, bytes/96)`.
- **index_stop_source_context.rs.** `window(trade, before, after)` is a trade-viewer page around the entry bar. It is display only, not a decision input, and its arithmetic is checked (`checked_sub` / `checked_add`, page bound).
- **index_stop_vix.rs.** Capture sorts the queries once (O(N log N) per catalog) and loads each month once. A trade whose entry and exit fall in different civil months is refused, not silently mixed.
- **Hot paths.** Every per-candidate walk in this slice (trade_facts, weakest across 7 grains, folds, returns, independent_sessions) is linear in that candidate's own trades or periods and is needed to produce the statistic. There is no repeated scan of other candidates' data.

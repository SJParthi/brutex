# p2expr: numeric audit pass 2 (runner expression*, signal_candle_stop, resample, align, closed, research_exit_grid, replay_mask)

Target: origin/final/all-fixes-zero @ 5140aca. Read-only. No probe crate built: nothing suspicious survived reading, so there was nothing to probe.

## Verdict

NO NEW FINDINGS.

## Findings

None.

## Checked and clean

- **signal_candle_stop.rs, stop level and trigger (the run2-1 class).** The stop is the signal bar's printed `low` (long) or `high` (short), an integer paisa price with no ppm conversion and no floor (:1131-1134). The trigger is `bar.low <= stop` / `bar.high >= stop` (:1230-1233). Level and trigger use the same integer, so the 1-paisa floor/ceil gap found in grid.rs cannot happen here. The gap-invalid check `entry.open <= stop` (long) / `>= stop` (short) (:1184-1187) refuses an entry that is already at the stop. Fills go through `grid::printed_stop_fills_v1` with `rested_at_open = index != entry`, so the entry minute is never treated as a gap.
- **signal_candle_stop.rs, look-ahead.** The entry is the aligned minute stamped exactly at the signal close (`align::onto_execution`, same IST day), filled at that minute's open. The stop is known when the signal bar closes. On a stop minute the favourable excursion credits only the open (:1237-1244). The forced exit is the 15:09 minute's close, and a missing 15:09 minute leads to `ClosingMinute` or `PathRefused`, never to a nearest-minute substitute. `TooLate` is classified the same way whether the entry minute exists or not (:1161 vs :1170). The `WhileOpen` occupancy runs through the exit minute's open, so a new entry on that minute is blocked.
- **signal_candle_stop.rs, arithmetic.** Every sum, PnL and excursion uses checked i64/u64 arithmetic. `ppm()` widens to i128 and floors, but its only caller is `statistical_row`, which has no production caller (grep). The drawdown peak starts at 0. `worst_trade` starts from the first trade. Trade, Period and Metrics `decode` re-validate the PnL against entry/stop orientation, holding minutes on both the clock and the index, and the count hierarchy.
- **signal_candle_stop.rs, cost per operation.** Each signal does O(1) work, plus a walk over the minutes it holds. That walk is a trade's real duration and cannot be shortened. The source checks look only at neighbours. `evaluate_days` walks the whole column and skips rows outside the window. Each row is O(1), so that costs a constant factor, not a complexity class.
- **expression_oos.rs.** The later slice must start strictly after the training day (`first > training_last` and `day(first) > day(training_last)`). The anchor seal binds the program, resolution, column, spec and horizon. Levels and the selected coordinate come from training only. `summary.hits == grid.signals` is cross-checked. Support-session counting is O(1) per row.
- **expression_validation.rs and expression_validation_codec.rs.** Windows must be contiguous, must cover the request exactly and must start after training. `index_folds` is one forward pass. In `partition`, a trade must have its entry and exit in the same fold and on the same day, and trades must not overlap. Fold totals are checked against the exact later cell. `day()` is an overflow-free equivalent of `(ts+IST).div_euclid(DAY)`. The codec checks the size with checked_mul/add and refuses padding.
- **expression_execution.rs and expression.rs.** Counts are checked. Source order is strictly increasing. Unknown is never turned into a hit.
- **resample.rs.** Buckets are anchored at 09:15 and keyed by clock. A bucket is stamped at its start and its close is the last minute it contains, so no later minute enters a bucket. Volume uses `checked_add`. `u32::MAX` minutes in micros is checked at compile time. Partial buckets cannot be entered early: entry needs the minute exactly at `ts+len`, which matches ind2's pass-1 reading.
- **align.rs.** One monotone cursor, O(1) amortised per signal. Exact timestamp and same IST day are required. `saturating_add` on the deadline can at worst produce a miss, never a wrong match.
- **closed.rs.** One hash probe per (superset, bit). `closure_complete` follows `Sweep::halted`. Closure is measured against the enumerated lattice, which leaves out pairs that `vocab::implication` pruned. That is a semantic choice and is not numeric, so it is not raised here.
- **research_exit_grid.rs.** Levels come from training bars only. The digest binds the training first and last timestamps, so the OOS boundary in expression_oos cannot be moved.
- **replay_mask.rs.** A fixed six-word check against the live mask. The `bit as u16` cast is bounded by 383.

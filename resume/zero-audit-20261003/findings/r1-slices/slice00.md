# slice00 — 3 findings

Scope: crates/runner/src/{admission,align,audit,bootstrap,bootstrap_family_pass,bootstrap_zero_v2,bound,closed,excursion}.rs (production code).

## F1 [medium] Audit tells the operator a MEAN winner MAE is "the tightest stop that keeps every winner"
- where: crates/runner/src/audit.rs:751-755 (strategy report row "mean MAE, winners only" / note "the tightest stop that keeps every winner") and audit.rs:1076-1085 (SHARPEST line: "That adverse figure is the tightest stop that would not have killed a winner.")
- what: `Cell::winner_mae` is an arithmetic mean: grid.rs:4688 `cell.winner_mae = adverse_won / n` with `n = wins`. The stop that keeps every winner is the MAXIMUM winner MAE. Setting a stop at the mean cuts every winner whose MAE is above the mean, which is typically about half of them. The row's own label even says "mean", but its note and the SHARPEST sentence present the number as a stop level, and that is the sentence an operator acts on. (grid.rs:260-261 repeats the claim in the field rustdoc. That file is outside this slice, but the same fix applies there.)
- evidence: trace. Take two winning trades with MAE 0 ppm and 100 ppm. `adverse_won = 100` and `wins = 2`, so `winner_mae = 50`. The report prints "Winners went 50 ppm against ... That adverse figure is the tightest stop that would not have killed a winner." A 50 ppm stop kills the second winner (MAE 100 >= 50). No field in `Cell` holds the winner maximum: `worst_mae` covers all trades, not only winners.
- fix: Reword both notes. For example: "mean adverse excursion of winning trades; NOT a stop level -- about half of winners went further". If the stop statement is wanted, add a `winner_max_mae` running maximum in grid.rs and quote that instead.

## F2 [low] Strategy report prints the `i64::MAX` no-drawdown sentinel as a measured ratio
- where: crates/runner/src/audit.rs:867-871 (`hundredths(cell.return_over_drawdown())`)
- what: `Cell::return_over_drawdown` (grid.rs:739-747) returns `i64::MAX` when `pessimistic > 0` and `max_drawdown <= 0`. Every variant whose trades all won reaches that state, so it happens with real data. The grid table handles the case through `ret_dd` (audit.rs:649-655), whose doc says printing the raw sentinel "reads as a MEASURED RATIO of nine quintillion, which is not a thing that happened". `strategy_report` skips that guard and renders `hundredths(i64::MAX)` = `92233720368547758.07` under "return over drawdown / profit per unit of pain". The cell reaches this path because `render_selected` passes the selected cell, or `best()`, straight to `strategy_report`.
- evidence: trace. `hundredths(9223372036854775807)` formats as `m/100 = 92233720368547758` and `m%100 = 07`, giving "92233720368547758.07". A temporary probe test (Cell with trades=3, wins=3, pessimistic=30000, max_drawdown=0, calling `runner::audit::strategy_report`) was written to confirm this. It never ran: it waited on the shared build-dir lock until it was stopped, so F2 rests on the trace above.
- fix: In `strategy_report`, use the same check as `ret_dd`: `if v == i64::MAX { "no drawdown".to_owned() } else { hundredths(v) }`.

## F3 [low] Exit-grid legend labels ret/DD (and mfe/allMAE) as ppm; both are hundredths
- where: crates/runner/src/audit.rs:629-633 (the legend printed under the grid columns: "the MAE/MFE columns and ret/DD are ppm")
- what: `return_over_drawdown` is `pessimistic * 100 / max_drawdown`, documented as "in hundredths" (grid.rs:716, 746). `edge_ratio` (the `mfe/allMAE` column) is `winner_mfe * 100 / all_mae`, also hundredths (grid.rs:703-708). `grid_row` prints both raw (audit.rs:679, 684). A ret/DD of 2.50 therefore prints as `250` under a legend that says ppm, which reads as 0.025%: off by a factor of 10^4. The strategy report renders the same quantity correctly as `2.50` (audit.rs:869), so one audit shows the same number in two different units. The same legend also calls the column "entry cost" while the heading says "fill cost".
- evidence: trace. Take pessimistic=25000 and max_drawdown=10000. `return_over_drawdown()` = 250, `grid_row` prints `ret_dd(c)` = "250", and the legend says ret/DD is in ppm.
- fix: Change the legend to: "the MAE/MFE columns are ppm; ret/DD and mfe/allMAE are ratios in hundredths (250 = 2.50)". Rename "entry cost" to "fill cost".

## What was checked and found sound
- align.rs: a forward-only merge cursor, an exact close-instant match, and a same-IST-day guard. No look-ahead and no off-by-one found.
- bound.rs: clause order is consistent; the sign split in `Hundredths` is correct for `i64::MIN`.
- bootstrap.rs: the stationary bootstrap, the `(B+1)` p-values and the RW exact-rule bar selection (`select_nth` at `draws - admissible`). The rejection-set equivalence to the adjusted p-values holds by monotonicity of the suffix bars.
- bootstrap_family_pass.rs: the claim that its p-values match the three separate procedures bit for bit. The expressions, fold order, RNG stream and refusal order all match.
- bootstrap_zero_v2.rs: the zero-row identity, the evaluate/evaluate_with_family_tests equivalence, and the refusal order.
- closed.rs: only the halted-sweep issues, already known (c4a-6/7, W3-runner1-3).
- excursion.rs: the quantile selection is equivalent to sorting; the pre-bar trailing anchor, the arming-at-end-of-bar rule, the `ppm_of` floor semantics and the clock/acceptance gate.
- admission.rs: the V1 check table (44 bits matching `KNOWN`), the reconciliations, the verdict status precedence, and the V1 decoders' re-validation. The probability floor-projection gap is closed by `floor_hidden_ceiling_v2`. The rate floor gap is known (GAP15-17).

## Known, still present
- closed.rs:162-168 `redundant_count` still ignores `sweep.halted` (W3-runner1-3); only `closed()` sets `closure_complete`.

Probe note: crates/runner/tests/zz_probe_slice00.rs was written for F2 and deleted without running (the build lock was held the whole time). Every finding rests on the code trace given.

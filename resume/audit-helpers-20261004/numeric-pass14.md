# Numeric pass 14 (num14): metrics that rank and describe results

Checkout: /home/claude/wt/zero3 at 1f4de71 (read-only). IDs p14num-1 upward. This pass does not repeat pass 8 (OOS, multiple testing) or pass 9 (trade simulation).

Counts: 3 new findings (1 medium, 2 low). 8 verification rows.

## Metric inventory (code, definition, verdict)

| metric | code | standard definition | verdict |
|---|---|---|---|
| win rate (cell) | runner grid.rs:394-405 `win_rate_bp` = floor(wins*10000/trades), 0 at 0 trades | wins / trades (Pardo, *Evaluation and Optimization of Trading Strategies*, 2008) | Formula correct. The floor is conservative for the `>=` gate. The web shows it as `(bp/100).toFixed(1)`, which hides the 1 bp floor-versus-round gap with the trade fold (+page.svelte:1496 `Math.round`). Cosmetic. |
| Wilson lower bound | grid.rs:463-515, frontier-analytics.js:76-86, population_statistics_v2.rs:5049, institutional_evidence.rs:2073 | Wilson (1927) score interval, z = 1.959964 | Every copy has the same operation order. **Ran:** a scan of every wins/trades pair with trades <= 3000 (4.5M pairs) found 0 mismatches between the trunc-bp projection and the floor-ppm/100 projection that institutional_evidence.rs:2096 reconciles. |
| profit factor | grid.rs:538-544 (`i64::MAX` when gross_loss = 0, including 0 trades); admission `ratio_observed` (population_base_evidence_v2.rs:1627) gives Unmeasured on a zero loss; web +page.svelte:1497 gives null | gross profit / gross loss | Formula correct. The flats edge case is p14num-3. |
| avg win / avg loss / payoff | grid.rs:546-578; outcome.rs:1472-1560 `payoff_bp` (side-aware, true loser count, n<2 gives 0) | mean win / mean loss | `Edge::payoff_bp` is correct. `Cell::avg_loss` counts flats in its denominator: this is run1-2, see verification. |
| min-win / max-loss | grid.rs:633-640 `reward_to_risk_bp`; outcome.rs:1392-1420 `worst_reward_risk_bp` | operator's own rule | Correct. The never-lost sentinel is `i64::MAX`. cli `ranked()` (lib.rs:12746) demotes it to last; the web treats it differently (p14num-2). |
| max drawdown | grid.rs:4714-4727 `accrue_risk`, reconciled at grid.rs:3060-3093; web trade-analytics.js:100 | peak-to-trough of closed-trade equity starting at 0 (Magdon-Ismail & Atiya 2004 uses the equity curve) | Engine, reconciler and web all start the peak at 0, so an initial loss counts. Absolute paisa, closed-trade only. Intra-trade excursion is `worst_mae`, a separate field. |
| drawdown duration | none | — | Not computed anywhere: grep for duration, underwater or recovery found nothing. |
| Sharpe / Sortino / CAGR / annualisation | none | — | Not present in runner, cli, api or web. The `sharpest()` selector (grid.rs:1051) uses `edge_ratio` (winner MFE / all MAE), not Sharpe. No bars-per-year or days-per-year constant exists, so there is no annualisation to check. |
| t (Newey-West, Bartlett on bar distance) | outcome.rs:2142-2440 | Newey & West (1987) | Edge cases are guarded: n<2 gives 0, m2 = 0 gives 0, and one window holding the whole sample gives 0. Earlier passes covered these. |
| tail measures | outcome.rs:1571 `largest_gain_paisa` (side-aware), Cell `best_trade`, admission `largest_trade_profit_share_ppm` = ceil(best_trade/gross_win) (population_base_evidence_v2.rs:589) | — | Correct. The share is of gross wins, not net, and it rounds up for its max gate. |
| leaderboard order | runner rank.rs:121-131 (`|t|` finite-first, then mask), ByPayoff/ByPath/ByAsymmetry rank.rs:453-575; cli money_key lib.rs:12756; web `rankRows` +page.svelte:886-980 | — | Every Rust order is total and ends on mask bytes. Grid selectors use `max_by_key`, so the last of equal keys wins over a fixed cell order. That is deterministic. The web sort breaks ties on `rank`. |

## New findings

### p14num-1 (medium): the live panel's "won %" is the share of UP moves, so every sell row shows its losing share as its win rate

- **Where:**
  - web/src/routes/backtest/+page.svelte:7292 (lead keystat "Winning trades") and :7381 (each live row: "% won").
  - The data comes from api/src/livejson.rs:225 (`edge_wins` = `row.wins`) and cli/src/frontier.rs:692 (`wins: scored.edge.wins`).
  - The source count is runner/src/outcome.rs:1765-1770: `if x > 0 { self.wins = ...`.
- **Code:** `{lead?.n > 0 ? ((lead.edge_wins / lead.n) * 100).toFixed(2) : '0.00'}%` and `{n > 0 ? Math.round((wins / n) * 100) : 0}% won`.
- **Why it is wrong:**
  - `Edge::wins` counts strictly positive forward moves on either side.
  - A live row's direction is the evidence side: `side_of_evidence`, mean < 0 means short (frontier.rs:723, lib.rs:5750). For a sell row, the positive moves are the trades it lost.
  - The engine's own test `the_perfect_short_and_the_perfect_long_are_worth_the_same` (outcome.rs:2779) builds the ideal short with `wins == 0`. The panel would print that short as "0% won", directly under "sell".
  - The same panel already flips the side for the move (`Math.abs(mean_milli_paisa)`), and `payoff_bp` reads the side too (outcome.rs:1517-1524). So the win rate is the one side-blind figure on a side-aware panel.
  - The win rate is shown on the operator's live ranking view, which is the first place a candidate is judged.
- **Repro (ran):** a node script uses the two template expressions on `/live.json`-shaped rows:
  - `{direction:'short', n:50, edge_wins:0}` (the outcome.rs:2779 short) prints `sell Winning trades 0.00% / row: 0% won`.
  - The outcome.rs:2747 "excellent short" (90 of 100 moves down) prints `10% won`. The true figure is 90%.
- **Fix:**
  - Choose one of these:
    - Serve a side-aware count from `Row::of`: `edge.losses` when mean < 0.
    - Or serve both `edge.wins` and `edge.losses` and have the page pick by `row.direction`.
  - Then name the field for what it counts.
  - Add a web test with a short row.

### p14num-2 (low): the web leaderboard scores an unmeasurable ratio as the midpoint, so a never-won row beats a winning row on "less losing ratio", and the board disagrees with the server's own never-lost rule

- **Where:**
  - web/src/routes/backtest/+page.svelte:812: `const lossRatio = (r) => (r.gross_win > 0 ? -r.gross_loss / r.gross_win : null);`
  - :936-940: `norm` returns `0.5` for `null`.
  - :966 and :970: the two terms that use it.
  - api/src/frontierjson.rs:423 and :546-552: `measurable()` sends `null` for `reward_to_risk_bp == i64::MAX`.
- **Why it is wrong:**
  - A row with losses and no wins has an unbounded loss ratio, which is the worst possible. `norm(null)` gives it 0.5, so it ranks above every measured row in the lower half.
  - A row that never lost has `reward_to_risk_bp` null, and it also gets 0.5.
  - The server's rule for that same sentinel is the opposite. `cli::ranked` (lib.rs:12716-12748) says *"A cell that never lost is UNTESTED, not best"* and sorts it LAST in all four server ranking keys. The web places it in the middle, so the web re-rank and the server `rank` it breaks ties on follow different policies for one fact.
  - The code comment's justification ("neither evidence for the row nor against it") holds for a never-lost row. It does not hold for a never-won row, whose ratio is known to be the worst possible.
- **Repro (ran):** a node script uses lines 781, 812 and 886-980 verbatim.
  - With only the "less losing ratio" weight set: `lossRatio 0.2 score=1`, `never won (gross_win 0, gross_loss -1000) score=0.5`, `lossRatio 0.9 score=0`.
  - With only "higher win:loss ratio" set: `rr 3.00x score=1`, `unbeaten (rr=null) score=0.5`, `rr 1.00x score=0`.
- **Fix:**
  - `lossRatio`: return `Infinity`-as-worst for `gross_win == 0 && gross_loss < 0`. In practice, give that term 0 rather than 0.5. Keep `null` only when both sums are zero.
  - `rewardRisk` null: give 0, which matches `cli::ranked`'s "ranks last". Or state on the page that it differs from the server order.
  - Add a decision entry, because the server rule exists as a decision.

### p14num-3 (low): one cell's losing-trade count has two definitions. The strategy report prints "losing trades N" beside "PROFIT FACTOR: no losing trade", and the backtest page prints the count both ways for the same run

- **Where:**
  - runner/src/audit.rs:900 `let losers = cell.trades.saturating_sub(cell.wins);` (printed at :943).
  - The profit factor line at :925-932 prints `"no losing trade"` when `profit_factor_bp() == i64::MAX`, which grid.rs:538-544 returns whenever `gross_loss == 0`.
  - On the web, +page.svelte:1466-1477 counts `losses` only for `net < 0` and `breakevens` separately (donut, :9775-9781).
  - :1846 `losses: trades - worstWins` (table, :10125) and frontier rows' `losses` (cli/src/frontier.rs:3232 `trades - cell_wins`, shown at :8407 and :10363) both include flats.
- **Why it is wrong:**
  - A trade with exactly zero pessimistic P&L is not a win (`pess > 0`, grid.rs:4515 and :4609). It adds nothing to `gross_loss`. It is counted as a loser by `trades - wins`.
  - Example: 3 winners and 2 flat trades. The strategy report prints `losing trades 2` and `PROFIT FACTOR no losing trade`, two lines that contradict each other.
  - On /backtest, the donut says 0 losers and 2 breakevens, while the run table and the frontier row say 2 losers.
  - On 1-minute index bars a zero move is common. The `Edge::payoff_bp` comment at outcome.rs:1512-1516 says so.
  - This is the same root as run1-2 and run1-1, which are OPERATOR-pending. The new symptoms are the self-contradicting report lines and the three counts on one page, not the averages.
- **Repro:** not run. The derivation from source is direct: losers = 5 - 3 = 2, gross_loss = 0, pf = `i64::MAX`.
- **Fix:** the run1-2 fix covers this. Fold a true loser count (`pess < 0`) into `tally_trade`'s else-branch, then use it for audit.rs:900, frontier `losses`, +page.svelte:1846 and `avg_loss`. Until then, print "flat" as its own line in the strategy report.

## Checked and clean (no finding)

- Wilson projections agree across the 4 copies (ran, see the table).
- The web frontier validator (frontier-analytics.js:279-298) recomputes win rate, R:R, return/DD, avg win and avg loss with the same integer floors as Rust. It refuses the payload on any mismatch, so the frontier figures cannot silently disagree between api and web.
- Ranking determinism: `Scored::cmp` is a total order (finite-first, then `total_cmp`, then mask words). The lens wrappers chain into it. `top.sort_unstable_by` is sound on a strict total order. The cli `pool.rs:721` sort is stable with explicit directions.
- `milli()` (outcome.rs:1695) maps NaN to 0. `mean_paisa` cannot be NaN, and t's NaN routes are guarded upstream.
- Earlier passes reported the §7 storage rounding of t, payoff and mean in frontier.bin as P9-01 (OPERATOR). Not re-reported.
- The live `clears_bar` rule is checked under xcut-1 and run1-3 below.

## Verification of related earlier findings

| ID | status | evidence at 1f4de71 |
|---|---|---|
| xcut-1 | FIXED | api/src/livejson.rs:244-245 `row.n >= cli::live::MIN_JUDGEABLE_OBSERVATIONS && row.t_milli.saturating_abs() > bar_milli` (D-1991) |
| run1-3 | FIXED | cli/src/lib.rs:18307-18313 `bar_milli` is `scaled.ceil()`, and livejson.rs:245 uses a strict `>` (CE-7, D-1769) |
| run1-1 | NOT FIXED (OPERATOR in triage) | runner/src/grid.rs:652-658 `guaranteed_floor` still uses `losses = trades - wins` |
| run1-2 | NOT FIXED (OPERATOR in triage) | grid.rs:557-563 `avg_loss` and :572-578 `avg_loss_magnitude_ceil` still divide by `trades - wins` |
| p4num-2 | NOT FIXED | the admission `average_loss_paisa` still follows the run1-2 denominator: boolean_admission_v1.rs:206-209 `losses = cell.trades.checked_sub(cell.wins)` is used at :250-255 |
| p5num-2 | NOT FIXED | cli/src/lib.rs:9261 `let losses = cell.trades.saturating_sub(cell.wins);` in `avg_payoff_holds` |
| p5num-4 | NOT FIXED | runner/src/audit.rs:966-969 `hundredths(cell.return_over_drawdown())`. `hundredths` (:994-1003) has no `i64::MAX` branch, so it prints 92233720368547758.07 |
| p5num-5 | NOT FIXED | api/src/frontierjson.rs:412 `row.payoff_bp` is written raw, not through `measurable()` |

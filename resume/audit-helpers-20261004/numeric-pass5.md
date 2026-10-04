# Numeric / look-ahead / O(1) audit, pass 5

**Verdict (head 1f4de71, final/all-fixes-zero, checkout /home/claude/wt/zero3):** 5 new findings: 1 medium and 4 low. The medium one is in the cli screen's calendar gate (`min_weakest_bp`). The gate runs on only the first `rules.top` rows. Every row past them keeps `admitted = true` without being judged. Those rows are counted as "satisfy every rule", can be selected as the run's admitted subject, and turn on `YOUR RULES: MET`. Every O(1) path that CLAUDE.md §3 rule 4 names still has exactly one live implementation.

Audit only. No repository was edited and no cargo was run, because no finding is high. Everything below was read from source at 1f4de71. Prior IDs are not re-reported (pst-*, clib-*, grk-*, run*, p2*, p3floor-*, p4num-1/2, and the D-1632 frontier verdict omission).

## Table 1: denominator and classification, by metric

Win = pessimistic P&L > 0 everywhere in the engine (grid.rs:4609, index_stop_qualification_metrics.rs:25, institutional_evidence.rs:~1630, signal_candle_stop.rs:1457, index_stop_store.rs:651). A flat trade (P&L exactly 0) is therefore a non-win, and it falls into the "loss" side of every builder below except the web display.

| metric | builder file:line | numerator | denominator | flat handling | gate direction | verdict |
|---|---|---|---|---|---|---|
| win rate | runner grid.rs:394; pbe_v2 :560; boolean_admission_v1 :219; index_stop_qualification_metrics :163; institutional_evidence :1422; web +page.svelte:1494 | wins | trades | in denominator | min | conservative. All five agree. |
| losing trade rate | pbe_v2 :609; boolean :245; index-stop :208; institutional :1439; runner admission.rs:2889 `losing_rate_ceil` | trades - wins | trades | counted as a loss | max | conservative (a flat raises it) |
| losing trades | same four builders | trades - wins | - | counted | max | conservative |
| average win | the four builders; Cell::avg_win grid.rs:548 | gross_win | wins (floor) | excluded | min | conservative. Unmeasured at wins = 0. |
| average loss | the four builders (div_ceil); Cell::avg_loss grid.rs:557 | abs(gross_loss) | trades - wins | **in denominator** | max | PERMISSIVE. Known as p4num-2 / run1-2, NOT FIXED. The web (+page.svelte:1497) divides by true losers, so web and engine disagree. |
| average payoff (avg win / avg loss) | cli lib.rs:9254 `Rules::avg_payoff_holds` | gross_win × losses | abs(gross_loss) × wins | **flats in `losses`** | min (`min_avg_rr_bp`, default 150) | **PERMISSIVE, new: p5num-2.** runner `Edge::payoff_bp` (outcome.rs:1508-1516) counts real losers, so the two builders disagree. |
| profit factor | the four builders `ratio_observed`; Cell grid.rs:538; pool.rs:173; web :1494 | gross_win | abs(gross_loss) (floor) | adds 0 | min | conservative. A zero denominator gives Unmeasured (admission), i64::MAX shown as "no losing trade"/"inf" (Cell), NEVER_LOST demoted (pool), or null (web). All consistent. |
| worst reward/risk | the four builders; Cell grid.rs:607; pool.rs:189 | min_win | abs(worst_trade) | an all-flat loss side makes worst = 0 | min | Admission: worst = 0 gives Unmeasured (fine). `Cell::clears` guards wins > 0 (grid.rs:683). **pool.rs `meets` has no wins guard: p5num-3.** |
| return / drawdown | pbe_v2 :1643 `return_drawdown`; Cell grid.rs:754 | pessimistic | max drawdown (floor) | - | min | conservative. Profit <= 0 gives 0, DD = 0 gives MAX. **The renderer prints the MAX sentinel as a number: p5num-4.** |
| max drawdown | index_stop_store :502; grid :4304; institutional :1578; pbe_v2 :1008; signal_candle_stop :1069 | peak - running on closed-trade pessimistic equity, peak starting at 0 | - | - | max | All five agree. Closed-trade equity only, with no intra-trade MTM, which is the stated definition. |
| losing streak | grid.rs:4678; index-stop :70; institutional :~1650 | run of non-wins | - | a flat extends it (grid.rs:4680, stated) | max | conservative |
| winning streak | same | run of wins | - | a flat breaks it | min | conservative |
| ambiguous fill rate | grid families: Cell.ambiguous_bars (grid.rs:4401/4425, ceiling); index-stop :80 `opt != pess` | ambiguous trades | trades (ceiling) | - | max | Definitions differ by family. The index-stop version counts any fill bracket, so it is stricter. Not permissive. |
| gap-affected rate | Cell.gapped, counted once in `ordered` grid.rs:5280 (level and trail); index-stop m.stop_gaps | gapped trades | trades (ceiling) | - | max | fine |
| session concentration | institutional :1690-1745; pbe_v2 :578; index-stop :172 (periods are IST days, index_stop_store :625) | max trades in one IST day | trades (ceiling) | counted | max | All three agree. |
| largest trade profit share | institutional :1720; pbe_v2 :588; index-stop :178 | best trade | gross_win (ceiling) | - | max | Agree. Unmeasured at gross_win = 0. |
| Wilson lower bound | institutional :1424; boolean `wilson_ppm` | - | - | flat counts as a non-win | min | floor, conservative |
| PBO | boolean_admission_v1 :312 | bottom-half splits | contributing splits (ceiling) | - | max | Unmeasured when 0 splits contribute. |
| White / SPA / RW / FWER p | boolean_admission_v1 :282-325 `ceiling_ppm` | - | - | - | max | fine (D-1990) |
| profitable OOS folds | validate.rs:3390, :3913, :4016 | folds with OOS pessimistic > 0 | decided folds | a flat fold is not profitable | min | conservative |
| weakest calendar share | cli lib.rs:12050 `weakest_bp`, gate :12619 | positive periods per grain, then the minimum | periods | a flat period is not positive | min (`min_weakest_bp`) | **Gate applied to the first `top` rows only: p5num-1.** |
| edge payoff (ranking) | runner outcome.rs:1472 | mean gain | mean give-back (real losses) | excluded (fixed) | rank only | Correct, but serialized raw: p5num-5. |

## Table 2: refused or unmeasured statistics, by renderer

| renderer | statistic | refused / unmeasured state | rendered as | verdict |
|---|---|---|---|---|
| runner audit.rs:1374 `bootstrap` | Reality Check, SPA | `None` | REFUSED | ok |
| runner audit.rs:1401 (fed by cli lib.rs:18530-18537 `.len()`) | Romano-Wolf named | empty Vec (block refusal) | "named 0" | p4num-1, NOT FIXED |
| runner audit.rs:1300 `overfitting` | legacy PBO | `probability() == None` | NOT MEASURED | ok |
| runner audit.rs:1207 `walk_forward` | folds | refused / empty | REFUSED / "-" | ok |
| runner report.rs:346, :402 | significance threshold | effective < 2 | "-" while the verdict uses `bonferroni_t` | p2run-1, NOT FIXED |
| runner audit.rs:748 `ret_dd` (grid table) | return/DD | i64::MAX | "no DD" | ok |
| runner audit.rs:932 `strategy_report` | profit factor | i64::MAX | "no losing trade" | ok |
| runner audit.rs:968 `strategy_report` | return over drawdown | i64::MAX | `92233720368547758.07` | **p5num-4** |
| runner audit.rs:946-947 `strategy_report` | average win / average loss | wins = 0 / losers = 0 | `0.00` | **p5num-4** (secondary) |
| cli lib.rs:13017 `screen_table` | PF | MAX | "inf" | ok |
| cli lib.rs:12633, :12642 | "N of M satisfy every rule", `admitted_any` | calendar rule not evaluated past `top` | counted as passed | **p5num-1** |
| cli pool.rs:1126 `ratio_cell` | tail | NEVER_LOST on an all-flat, zero-win candidate | "never lost", meets = true | **p5num-3** |
| api booleanadmission_projection.rs:39-54 | every Observed/Completeness/Decision value | Unmeasured / Refused | `{"state":..,"value":null}` | ok |
| api frontierjson.rs:423-424 `measurable` | reward/risk, return/DD | MAX | null | ok |
| api frontierjson.rs:412, livejson.rs:220 | `payoff_bp` | n < 2 gives 0 (a refusal); no give-back gives i64::MAX | raw `0` / `9223372036854775807` | **p5num-5** |
| api frontierjson.rs:240 | admitted | no rows (rules = None) | 0 | ok (true zero) |
| web BooleanEvidence.svelte:72, boolean-evidence.js:47 | Romano-Wolf per row | availability not measured | label, `romano === null` enforced | ok |
| web +page.svelte:1494-1500 | PF, average win/loss, win-loss ratio | no denominator | null | ok |

## O(1) primitives of CLAUDE.md §3 rule 4 at 1f4de71

`git diff 5140aca 1f4de71` over engine, vocab, store and indicators changes no live O(1) primitive:
- engine is docs plus a stricter source test (column.rs:480+).
- vocab/table.rs moves the const name index into `const fn name_index`, which is still built at compile time.
- store/file.rs changes are append-path diagnostics only.

Each primitive still has one live implementation:

| primitive | live implementation | state |
|---|---|---|
| mask evaluation | engine column.rs:105 `support`: one `fold`, one `row.hits` per row. The test now bans loops, filter, count and get in the body. | ok |
| duplicate rejection, k = 1 | engine lib.rs:142 `primitives::offer` → `HashSet<u32>::insert` (:1507) | ok |
| duplicate rejection, k >= 2 | none; the join is injective | ok |
| result append | lib.rs:149 `primitives::append` → `out.push` into `out`, reserved for the batch at lib.rs:2371. The tracked CLAUDE.md text (:152-158) matches; the session's older copy still describes the old lane-local `kept` design, and D-1440 already fixed that text. | ok |
| retention dedup | `Retention::offer` keep.rs:375 (`HashSet` plus a log-cap heap, declared o1engine-20) | ok, declared |

No second scanning path was added.

## Verification of related prior items at 1f4de71

| id | state | evidence |
|---|---|---|
| p4num-1 | NOT FIXED | cli lib.rs:18530-18537 still `.len()` |
| p4num-2 / run1-2 | NOT FIXED | boolean_admission_v1.rs:206-209 and :252-256; grid.rs:557-578 still divide by `trades - wins` |
| run2-2 | NOT FIXED | exit_grid_policy.rs:2462 unchanged |
| p2run-1 | NOT FIXED | report.rs:346-354, :402-410 |
| D-1632 (frontier verdict omits average payoff and fill headroom) | declared, open | docs/05-decisions.md:55486 |

## New findings

### p5num-1 (medium): the calendar gate judges only the printed rows, and un-judged rows are counted, selected and bannered as passing

**Where**
- cli/src/lib.rs:12619-12625, the gate
- :12633, the passed count
- :12642, `admitted_any`
- :10999, `final_selection`
- :11800-11804, the `YOUR RULES: MET` banner
- :12953, the measurement band

**Code**
```rust
for row in rows.iter_mut().take(rules.top) {            // gate: top rows only
    if let Some(ref c) = row.consistency {
        row.steady = c.weakest_bp() >= rules.min_weakest_bp;
        if !row.steady { row.admitted = false; } } }
let passed = rows.iter().filter(|r| r.admitted).count();   // ALL rows
...
rows.iter().find(|row| row.admitted && row.cell.trades > 0)  // final_selection, ALL rows
let admitted_any = rows.iter().any(|row| row.admitted);
```

**Why it is wrong.** Rule 5 of the banner ("at least {} of periods must close POSITIVE at EVERY grain") is applied only to `rows[..top]`. `measure_top` measures `measured_band(top)` = 8 × top rows (:12953). Its doc (:12950) says this is "so the calendar gate below can demote a measured row and the one that replaces it is measured too". The gate never looks past `top`, though.

The calendar sort (:12583-12610) puts admitted rows first and then orders them by `weakest_bp` descending. So suppose more than `top` rows pass the cell rules and the best of them fails `min_weakest_bp`:
- every printed admitted row is demoted;
- row `top + 1` is still `admitted`. It is either measured with an even lower weakest share, or unmeasured and sorted last.

`final_selection` then returns that row as the admitted subject. `passed` counts it under "satisfy every rule", and `screen_cascade` prints `YOUR RULES: MET`. The descent reads that banner from the text (:11786-11788), so the outcome is a refused calendar rule rendered as a pass. That is the §4 ban, and it is the permissive direction for a min-gated share. In a cascade tier with relaxed rules, many rows pass the cell rules, so the strictest tier "met" is inflated in the same way.

**Repro.** Not run, traced from source. Use `top = 10` and `min_weakest_bp = 5000`, with 11 or more rows admitted by `Rules::admits`, all of whose weakest grain is below 50%:
- Expected: 0 passed and `YOUR RULES: UNMET`.
- Actual: `passed = admitted - 10`, `admitted_any = true`, and the selected row is rank 11, which fails rule 5.

**Fix.** Apply the gate over the whole measured band. Treat every admitted row the gate did not evaluate as not passing whenever `min_weakest_bp > 0`: `admitted = false` with reason `steady-unmeasured`, or exclude it from `passed`, `admitted_any` and `final_selection`'s first arm. Add a test with `top + 1` admitted, inconsistent rows. Add a decision entry, because the run's selected subject can change.

### p5num-2 (low): `Rules::avg_payoff_holds` counts flat trades as losers, so `min_avg_rr_bp` is permissive

**Where.** cli/src/lib.rs:9254-9271, called from `Rules::admits` :9130.

**Code**
```rust
let losses = cell.trades.saturating_sub(cell.wins);
...
cell.gross_win.saturating_mul(losses).saturating_mul(100)
    >= lost.saturating_mul(wins).saturating_mul(required_bp)
```

**Why it is wrong.** The rule is "average win / average LOSS >= r". A flat trade adds 0 to `lost` and 1 to `losses`, which shrinks the implied mean loss. That is the permissive side for a minimum. This is a different gate from p4num-2 (`average_loss_paisa`, admission) and from run1-2 (the `Cell::avg_loss` display). The runner's own `Edge::payoff_bp` was corrected for exactly this (outcome.rs:1508-1514, "COUNTED, NOT SUBTRACTED"), so the two payoff builders disagree. The web stat (+page.svelte:1499) also uses true losers.

**Repro.** Not run, arithmetic only:
- Inputs: 4 trades, 1 win (+100), 1 loss (-100), 2 flats; `min_avg_rr_bp` = 200.
- LHS = 100 × 3 × 100 = 30,000. RHS = 100 × 1 × 200 = 20,000. The rule passes.
- True ratio: 100 / 100 = 1.00, which should fail.

**Fix.** Use the count of P&L < 0 trades. `Cell` has no loser count, so add one, folded in `tally_trade`'s else-branch when `pess < 0`, and use it here, in `avg_loss`/`avg_loss_magnitude_ceil`, and in the admission builders. That fixes run1-2 and p4num-2 at the same time. Add a decision entry.

### p5num-3 (low): the pooled tail rule passes a candidate that never won

**Where.** cli/src/pool.rs:189-201 (`tail_bp`, `meets`), :962-977 (fold), :1126 (`ratio_cell`).

**Code**
```rust
fn tail_bp(&self) -> i128 { if self.worst == 0 { return NEVER_LOST; } ... }
fn meets(&self, rule_bp: i64) -> bool { self.fired > 0 && self.tail_bp() >= i128::from(rule_bp) }
```

**Why it is wrong.** Suppose a candidate's every pooled trade is flat (P&L exactly 0). Then `worst == 0`, `wins == 0` and `min_win` is reset to 0 (:976). The tail is NEVER_LOST, so `meets` is true and the row sorts first (`key` leads with `meets`). It prints `wins 0, tail never lost`.

The single-instrument rule `Cell::clears` (grid.rs:683-688) carries an explicit `wins > 0` clause for this case, and its doc explains why ("no winners is never what an operator means by a cleared bound"). The pooled table claims to hold "the same number so the two tables agree" (:114-117), but it drops that clause. The case is rare, since it needs exactly 0 paisa on every trade, but it is reachable: `shown_cell` can return a non-admitted cell (lib.rs:12153), and pool.rs:917 keeps any cell with trades > 0.

**Repro.** Not run. Fold one cell with `trades = 2, wins = 0, worst_trade = 0, min_win = 0, gross_* = 0`. Expected: `meets` is false. Actual: true, with tail shown as "never lost".

**Fix.** `self.fired > 0 && self.wins > 0 && ...`, and render the tail as "-" when `wins == 0`. Add a test beside `refusals_and_unfired_candidates_are_never_folded_in`.

### p5num-4 (low): STRATEGY REPORT prints the return/drawdown sentinel as a measured ratio

**Where.** runner/src/audit.rs:966-969, with secondary rows at :946-947. Called from audit.rs:459 (`audit`, `audit-stored`).

**Code**
```rust
("return over drawdown", hundredths(cell.return_over_drawdown()), "profit per unit of pain"),
("avg winning trade", money(cell.avg_win()), ""), ("avg losing trade", money(cell.avg_loss()), ""),
```

**Why it is wrong.** `Cell::return_over_drawdown` returns `i64::MAX` when the variant made money and never gave any back (grid.rs:758-760). Any all-winner selected variant does this. The same file already knows this. `ret_dd` (audit.rs:740-753) maps the value to "no DD" and says that printed raw it "reads as a MEASURED RATIO of nine quintillion, which is not a thing that happened". `strategy_report` is the second renderer of the same metric, and it prints `92233720368547758.07`.

As a secondary defect, `avg_win()` and `avg_loss()` return 0 when there are no winners or no losers, and those rows print `0.00` as if a mean had been measured. PF on the same report already says "no losing trade".

**Repro.** Not run. Call `strategy_report` with a cell where `trades = 3`, `wins = 3`, `pessimistic > 0` and `max_drawdown = 0`. The row reads `92233720368547758.07`.

**Fix.** Use the same mapping as `ret_dd` ("no DD"). Render `-` with the note "no winning/losing trade" when `wins == 0` or `trades == wins`. Add a test.

### p5num-5 (low): `payoff_bp` goes on the wire raw, so a refusal reads 0 and the unbounded case breaks the combinations panel

**Where.** api/src/frontierjson.rs:412 and api/src/livejson.rs:220, with values from runner outcome.rs:1526-1534 via cli/frontier.rs:691. The consumer is web/src/lib/frontier-analytics.js:9 and :196-199, which the backtest page uses at +page.svelte:710-717.

**Code**
```rust
if self.n < 2 || gains == 0 { return 0; }          // refusal
if backs == 0 || back_sum <= 0.0 { return i64::MAX; }  // unbounded
... row.payoff_bp,   // frontierjson.rs:412, emitted raw
```

**Why it is wrong.** The same response already runs `reward_to_risk_bp` and `return_over_drawdown` through `measurable()` (frontierjson.rs:546), which turns MAX into null. `payoff_bp` gets no such treatment, and that causes two problems:
1. **The unbounded case.** A ranked row whose forward moves never went against it (n >= 2) is emitted as `9223372036854775807`. `JSON.parse` turns that into 9223372036854775808. `validateFrontierPayload` then requires `Number.isSafeInteger(row.payoff_bp)` and refuses the whole frontier ("rows[i].payoff_bp is not an exact integer"). The run's combinations panel fails for an honest value.
2. **The refusal case.** n < 2 is a refusal, but it is served as a measured `0`.

**Repro.** Not run. Inferred from the source and from JS number semantics (2^63 - 1 is above 2^53).

**Fix.**
- Serve `payoff_bp` through `measurable()`, and map the n < 2 refusal to null as well. That needs `Edge` to expose the refusal as `Option`.
- Move `payoff_bp` to `NULLABLE_ROW_INTEGERS` in frontier-analytics.js.
- Make the same change in /live.json.

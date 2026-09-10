# NIFTY and BANKNIFTY: what the Backtest sweep must show

This workflow researches real stored OHLCV for one selected index at a time.
It starts only from **Backtest → Run sweep**. Opening the page, viewing a
saved result, changing a timeframe or retrying a status read cannot start work.
The current implementation is a release candidate until its verification and
source-admission evidence has passed. This document is not launch clearance.

## Your rule and the evidence used to check it

| Requirement | What the engine checks | What you see |
|---|---|---|
| NIFTY or BANKNIFTY | Exact canonical index, feed, historical span and selected timeframes | Submitted scope and a separate row for every selected timeframe |
| Both trading directions | Every accepted expression is evaluated long and short | Direction beside each setting |
| One defined stop | Completed signal candle low for long; high for short | Original signal, entry and stop for each saved trade |
| Intraday exit | Risk stop or the unique accepted one-minute close reaching 15:10 IST | Exit reason and recorded minute evidence |
| Both fill readings | Pessimistic and optimistic prices on the same recorded trade path | Both results; acceptance uses the pessimistic reading |
| At least 60% winning days | Winning days divided by all eligible trading days | Winning days, eligible days and percentage |
| At least three wins, at most two losses in a full week | Every complete five-session Monday–Friday week | Complete weeks passed and failed, with individual days available |
| Never three losing trading days in succession | Count across weekends, holidays, year boundaries and the training/later split | Longest losing-day sequence and failures |
| Institutional checks | Existing complete research policy plus the independent day/week assessment | Institutional verdict, day/week verdict and whether both passed |
| Save progress | Exact program order, source identity, policy, completed batch and result pins | Acknowledged counts and the last complete saved batch |
| Inspect the outcome | Bounded settings, trade, day and week pages | One selected timeframe comparison with detailed evidence |
| Compare saved batches | Authenticate the acknowledged search prefix and every included family before ranking | Per-timeframe ranks across that exact saved scope, with passed and failed outcomes available separately |
| Inspect original candles | Read the immutable source companion named by the saved catalog | Original condition names, build, feed, stop, entry and exit candles; existing tables remain available if the companion is absent |
| Inspect VIX context | Read the original same-feed reference companion for the exact saved trade | Separate entry-minute and exit-minute OHLCV, or the saved absence/unavailability reason |

The capital and rupee examples are not fixed execution parameters. NIFTY and
BANKNIFTY output is in **index points per underlying unit**. It is not the
profit of a futures lot, an option contract or an account. Costs are excluded.

## Difficult cases remain visible

| Case | Required result |
|---|---|
| Flat or no-trade day | It does not count as a winning day. Under V1 only a winning day resets a losing-day sequence. |
| Holiday or partial week | Show it separately; do not pretend it was a complete five-session week. No complete weeks means the weekly rule is unmeasured. |
| Missing expected trading session | Refuse the affected evidence; do not insert an invented zero-return day. |
| The month before training is absent | Require the same feed's original minute and daily context for that month. Display its date before launch; do not silently shorten the selected span or manufacture earlier candles. |
| Missing or duplicated 15:09 close | Do not fabricate a nearby 15:10 exit. |
| Next opening price is already beyond the signal stop | Skip that invalid entry; do not create a free gap trade. |
| Index spot VWAP is unavailable | Preserve unknown eligibility, including inside OR/NOT expressions. |
| Multiple trades in one day | Allow sequential trades, at most one open position per setting; assess the sum for the day. |
| New batch is still running or refuses | Retain the last acknowledged saved batch with its original number and pins. |
| Status request times out | Show uncertainty and inspect the same attempt. Do not submit another sweep automatically. |
| Saved evidence is changed or belongs to another declaration | Refuse it before presenting a qualification. |
| Work, memory or evidence budget is insufficient | Report the exact admission failure; do not report an incomplete family as a finished search. |
| No setting passes | Show zero qualifying settings and the reasons. Do not replace them with profitable-looking failures. |
| A newer cumulative read fails | Keep the previous verified comparison under its original checkpoint and show the new read failure. |
| A monthly source file or today's vocabulary changes | Use the original archived source and names, or refuse the chart. Never attach current candles to an old trade. |
| Original-source inspection exceeds its memory limit | Refuse the complete publication or read; do not silently trim candidate observations or candle windows. |
| Original VIX is absent or its month was unreadable | Distinguish a missing exact minute from an unavailable original month. Keep the saved reason and never substitute current VIX or an invented zero candle. |

The cumulative comparison is ordered by **later pessimistic index points**.
It includes all acknowledged nonempty batches for the chosen timeframe inside
the verified checkpoint. The default filter shows only settings that passed
both institutional and day/week requirements. **All outcomes** retains the
failures and their reasons. A result's details open its exact saved batch and
setting, even when that setting is beyond the first page.

Trade charts show original **one-minute execution candles**, including the
saved entry and exit candle. The strategy's signal timeframe remains separately
labelled. A candle does not reveal the order of every price within that minute;
the chart preserves the recorded pessimistic/optimistic readings rather than
inventing ticks or an exact intraminute exit time. Legacy results without an
original-source companion keep their existing evidence tables and explicitly
refuse original-source charts.

“Show candles and VIX” opens two independent views for the same saved trade.
VIX is reference context and does not enter the strategy, results or ranking.
Its full entry-minute candle becomes known after the minute completes, so it is
not an instantaneous value available at the entry open. The exit annotation
also covers a printed minute; a risk stop's precise intraminute time remains
unknown. Later corrections to current VIX files do not rewrite these original
saved annotations.

## Parallel execution and time

Selected timeframes can execute concurrently within the configured CPU and
memory admission. Source preparation, historical replay and statistical work
still take time. If a completed batch contains P expressions and R timeframes,
it contains **2 × P × R settings**: each expression is long and short. The two
fill readings are retained within each setting, not counted as two additional
strategies. All eight timeframes therefore contain 16 × P settings per complete
batch. There is no exit-grid multiplier in this workflow.

An invocation's work allowance is a pause/resume limit. It is not a statement
that every possible Boolean expression has been tested. The full grammar is
too large to exhaust in a practical run; D-0580 records an exact conjunction
lower bound. A finish-time estimate needs a finite declared population and
measured throughput on the admitted history. No full-search duration, O(1)
total runtime or constant disk latency is claimed.

The original-source producer and viewer use the same observation limit. Before
pricing, the producer admits the archive and the minimum complete declared
batch. Before publication, it admits the actual complete catalog together with
source reconstruction. The first check does not promise that every later
catalog will fit. An oversized completed family is refused as a whole. The
memory accounting is a conservative allocation model, not a measured RSS or
operating-system guarantee.

The first cumulative read verifies the retained checkpoint chain and complete
candidate families and sorts them before paging. That work grows with saved
history. A cached page holds publication locks and checks generations for all
contributing artifacts, including batches outside the displayed page. Sorting
and checking those references grows with the retained scope; the bounded page
lookup does not make the complete read O(1). Newer journal entries can still be
appended while an older exact prefix is inspected. A new checkpoint requires a
new complete admission. Insufficient current replay or memory bounds produce a
visible refusal, while exact individual batch comparisons remain available.

## Interpreting a passed result

Passing means the exact saved setting passed the declared historical checks.
Training, later comparison and their combined calendar span are assessed
separately. The proposed chronological month split is visible and editable
before launch. Later comparison participates in research selection and is not
claimed to be a final untouched holdout.

These checks can reject weak or inconsistent historical results. They cannot
guarantee future weekly wins, sustained account profits, or membership of the
strongest 0.1% of an unexhausted strategy population.

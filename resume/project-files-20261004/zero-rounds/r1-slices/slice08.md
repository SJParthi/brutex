# slice08 (indicators: anchored, column, daily, evaluator, fib, gap, lib, orb, pattern, session) — 4 findings

## F1 [low] `pat_separating_lines_bull/bear` (208/209) fire on the wrong colour pair and never on the classical shape
- where: crates/indicators/src/pattern.rs:598-603
- what: 208 requires `bar1.bullish() && bar0.bullish() && bar0.open == bar1.open`, and 209 is its mirror with both bars bearish. In the classical definition the module claims to follow (pattern.rs:12-20, "the widely-used classical conventions"), separating lines are two bars of OPPOSITE colour with the same open: bearish then bullish for the bull form, bullish then bearish for the bear form. So the bit is set on two same-colour bars, which is the "matching opens" shape the test at pattern.rs:1014 describes. It is never set on the shape its vocabulary name gives. This is the same class as known finding hunt-indicators-3 (228/230/213/221), but that finding does not list these two bits.
- evidence: a temporary probe through the public `Patterns::step` API (since deleted):
  ```
  classical bull separating (bear,bull,same open): 208=false 209=false
  classical bear separating (bull,bear,same open): 208=false 209=false
  two bullish same open: 208=true 215(side_by_side)=false
  ```
- fix: 208 `bar1.bearish() && bar0.bullish() && bar0.open == bar1.open`; 209 `bar1.bullish() && bar0.bearish() && bar0.open == bar1.open`. If the current shape is intended, rename the vocabulary rows instead. That is an append-only vocab change (retire and re-add) plus a D-entry, because a bit's meaning may not change in place.

## F2 [low] A reprojected column's `Census` cannot reconcile, while `Census::reconciles` says false is always a defect in the module
- where: crates/indicators/src/column.rs:950 (`census.offered = onto_len as u64;`), against column.rs:261-266 ("False is a defect in this module, never in the data")
- what: `reproject_with` keeps the signal series' `warming`/`swept`/refusal counts but overwrites `offered` with the execution series length. The comment directly above it (column.rs:937, "THE CENSUS IS THE SIGNAL SERIES', AND IS LEFT ALONE") says the opposite of what the next line does. Whenever the execution slice length differs from the signal slice length, the census `Column::census()` returns has `offered != warming + swept + refused`, so `reconciles()` is false. That is the normal case (1-minute execution under a coarser signal). A consumer that applies the documented check to a Fill column, for example `runner::complete`, which requires `census.reconciles()`, would read the run as defective. No production caller does that today (sweeps get signal columns), so this is a latent contract contradiction rather than a live wrong result.
- evidence: probe through the public API, 40 sessions x 375 bars, then `reproject_checked` with an identity `onto`:
  ```
  signal census Census { offered: 15000, warming: 1876, swept: 13124, ... } reconciles=true
  same-length projection reconciles=true
  execution one bar longer: census Census { offered: 15001, warming: 1876, swept: 13124, ... } reconciles=false
  ```
- fix: keep the signal census untouched (`offered` included) and expose the execution length separately (it is already `acceptance_census().offered` on the checked path). Alternatively, document on `reconciles` that it applies only to `Sourced::Signal` columns and correct the comment at :937.

## F3 [low] `Corrupt::PriceNotPositive` rustdoc says all four prices are tested; the code tests `low` alone
- where: crates/indicators/src/lib.rs:384-388 (doc) vs lib.rs:1775-1780 (`check_evaluable`: `if self.low <= 0`), and evaluator.rs `stepped` (`if bar.low <= 0`)
- what: the variant doc says *"All four fields are tested, not `low` alone, for the reason `ohlc_is_sane` gives for the same choice: this predicate must not depend on another clause of itself being true, or a later edit to the ordering guards silently widens what a price may be."* The implementation does exactly what that sentence forbids. It tests `low` only and relies on the containment clauses of `check()`; the comment at lib.rs:1775-1777 says so. Today the behaviour is equivalent, because containment runs first. The doc states a robustness property the code does not have, and the hazard it names is real: dropping or reordering a containment clause would let `open`/`close <= 0` through.
- evidence: trace. `check_evaluable` = `check()?` then `low <= 0` only. Remove the `close < low` clause from `check` and a bar with `low=1, close=0` is accepted.
- fix: test all four fields (`open|high|low|close <= 0`), which is four comparisons, or rewrite the variant doc to state the actual dependency on containment.

## F4 [low] `GapReferenceSource::SignalSeriesLastThreeIntradayBars` documents the pre-D-1441 gap rule
- where: crates/indicators/src/anchored.rs:270-275 and :448-451
- what: the variant and its doc ("signal stream's last three bars versus today's first three") describe the three-BAR fold. gap.rs:57-62 records that D-1441 replaced it with 3-minute clock buckets ("Until D-1441 the fold counted three BARS and never read the timestamp"). The public accessor `AnchoredEvaluator::gap_reference_source()` therefore names a rule the evaluator no longer applies. On rungs of 3 minutes and above, one bar is the candle, not three. Only a test reads it today.
- evidence: trace. `GapFib::fold` (gap.rs) keys on `candle_bucket(ts)`, not on a bar count, while anchored.rs:450 returns `SignalSeriesLastThreeIntradayBars`.
- fix: add a variant such as `SignalSeriesThreeMinuteBuckets` (or rename with a D-entry) and update the doc.

## What was checked and found sound
- No look-ahead: every module computes bits before folding the current bar. CurDayFib, GapFib and Orb close windows before measuring; Session folds the current bar's own extremes, which are known at its close; the anchored join installs only references with `day < signal_day`; the exact-minute overlay maps the closing minute only, clamped to the caller's session close.
- Commit-on-success: `Evaluator::stepped` works by value, `AnchoredEvaluator` works on a copy, and `AnchoredColumn::build_required` works on a copy.
- Truth/known coherence: checked for curday, gap, orb, daily, fib, session, patterns and crossings, including the zero-range pattern early return. Every multi-bar pattern needs a directional or doji bar0, so "known false" is correct.
- Tolerance bands match each `set_near` call (pivot on CprWidth, everything else on SessionRange).
- The nine charter IST day numbers decode to the dates and weekdays their comments name.
- Integer-only arithmetic in i128 with `div_euclid`; no float.

Known, still present: none beyond the recorded items (hunt-indicators-2/3, errpaths-4, F-777A8F, F-EBD1B5, R9-csr-cx-4).

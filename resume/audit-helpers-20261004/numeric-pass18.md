# Numeric audit, pass 18: leakage between in-sample and out-of-sample data

Head: `/home/claude/wt/zero3` @ **1f4de71** (origin/final/all-fixes-zero). Audit only. Nothing in the checkout was edited.
One throwaway integration test ran in a scratch worktree (`scratch-num18`), which was then removed.
Scope: every place the code splits discovery from validation.

- `runner::split`: `purged_folds`, `anchored_folds` and `rolling_folds`.
- `runner::validate`: `walk_forward_core`, the V2/V3/V4 anchored doors and `walk_forward_exact_grid_v4`.
- `exit_grid_policy`: resolve and OOS replay.
- `expression_oos` and `expression_validation` (fixed-training later folds).
- Boolean search, OOS, admission and qualification, with its plan and the consistency records.
- Index-stop search, launch, qualification and the ranking page.
- Step 3, the post-training OOS cohort, Global Replay V4 (`ledger_v6`) and Selection V6 / Execution V4.
- The CSCV layout in `population_observations_v1`.
- `cli` `audit_bars` / `both_shapes`, with the `Sweeper::auto` support derivation.

Not re-reported: p8num-1 (pass 8), p17num-1 (alpha restarts).

## Verdict

**1 new finding (p18num-1, low, latent).** Every split on a production path is causally clean, and the
reason is the same each time: the training half is a physical slice, or a timestamp-clipped prefix.
Index arithmetic alone does not keep the halves apart. Only `split::purged_folds` relies on index
arithmetic alone, and its purge and embargo are one bar too short for this engine's trades.
A throwaway test showed one bar sitting in both halves. `purged_folds` has no production caller.

## Checklist results

| # | Question | Result | Evidence |
|---|---|---|---|
| 1 | Can a bar, day or trade sit in both halves? | No, on every production path. Yes, in `purged_folds` (p18num-1). | Walk-forward: the training execution prefix is `MonotonicExecutionPrefix::before(exec, test_open)` (validate.rs walk_forward_core and :2519). The training signal slice is `bars[fold.train.0]`. With one series, `trade_train = train`, so a hold past the slice end is `too_late` (trade.rs RULE 1c). The throwaway test also walked each anchored training slice: no trade exits at or past `boundary`. Month and day splits: `later.from <= to` refuses in boolean_oos_v1.rs:200, boolean_oos_command.rs:57, boolean_search_command.rs:71, boolean_qualified_command.rs:55, index_stop_launch.rs:46 and ledger_v6.rs:676. Every trade is intraday (forced 15:10), so a month or day boundary cannot be crossed by a position. expression_oos.rs:268-271 also requires a strictly later IST day. |
| 2 | Does OOS warm-up read IS bars (allowed)? Does any IS statistic read OOS bars (leakage)? | Warm-up reads IS bars, which is allowed: OOS columns are built from bar 0, or from an "original-first" source, and training rows are blanked by `restricted`. No IS outcome statistic reads OOS bars. One support-threshold input does (note A). | Exit-grid rungs are re-resolved per fold on the training prefix (validate.rs:2536-2537). The legacy door derives `Levels` from training walks only. The side comes from the training-window edge (`direction_from_training_edge`). Daily references must be "strictly before" each bar (lib.rs:2589-2613). V4 checks that every fold column is a prefix of the full column (`training_signal_cursor.authenticate`). |
| 3 | Was the OOS set used to choose anything before validation? | Not before its own validation. After validation it is used, and docs say so. | The admission gates `min_profitable_oos_folds` and `min_oos_pessimistic_return_paisa` consume walk-forward test folds inside the training span. Global Replay then validates on strictly later months. The index-stop ranking page orders by later pessimistic paisa (`indexstoprankingjson.rs` "later_pessimistic_descending…") after RW/White/SPA qualification on that same later window. Covered by docs/06-limits.md:8795 ("Fixed-training later windows are not a separate terminal untouched holdout"). |
| 4 | Is repeated validation on the same OOS set counted? | Within one declaration, yes: index-stop countable spending, `alpha/(8(b+1)(b+2))`, and the boolean plan's eight units. Across studies, no, and this is documented. | docs/06-limits.md:8795-8796: "cross-study data reuse is not globally budgeted". boolean_qualification_plan.rs:7-8: "Separate plans do not claim a shared grammar-wide budget". Global Replay V4 has no reuse counter. Known, so not re-reported. |
| 5 | Is there purging or an embargo at the boundary? | Walk-forward: a purge of `h` signal bars, plus a hard timestamp clip. Month/day splits: none, and none is needed, because labels are intraday and features look only backwards. CSCV periods are whole accepted IST sessions (population_observations_v1.rs `derive_layout`), so no label crosses a block. | EMA200 and the five-session ladder only look back, so they are warm-up and not leakage. An embargo matters only when a test label reaches a training bar. That happens only in `purged_folds` (p18num-1). |

## New finding

### p18num-1 (low, latent): `purged_folds` purges and embargoes `h` bars, but a trade from bar `i` occupies bars `i+1 ..= i+1+h`, so one bar lands in both halves

- **Where.** `crates/runner/src/split.rs:85` (`purged_folds`), :110 and :114:
  ```rust
  // PURGE BEFORE. A training bar at `i` has an outcome window reaching
  // `i + h`, so it must end before the test range begins ...
  let left_end = test_start.saturating_sub(h);
  ...
  let right_start = test_end.saturating_add(h).min(bars);
  ```
  The module's own leak test checks the same window: split.rs:403, `let window_end = i.saturating_add(15);`.
  The anchored and rolling shapes use the same arithmetic at :227 and :289.
- **Why it is wrong.** `[i, i+h]` is the window of the `outcome::forward` label. A trade is different.
  `trade::walk` on a signal-sourced column fills on the next bar (trade.rs:986, `signal.checked_add(step)` with
  `step = 1`). Its deadline is `ts(entry) + h·step`, so on contiguous bars it exits at `i+1+h`.
  - The last left-training signal `test_start-h-1` therefore exits on `test_start`, the first test bar.
  - The last test signal `test_end-1` exits on `right_start`, the first post-embargo training bar.

  The doc comment ("drop any training bar whose outcome window overlaps the test range") and the test
  `no_training_bar_can_see_into_its_test_window` both claim this cannot happen. They prove it only for
  the forward label, not for the trades the engine prices.
  On production paths the same arithmetic does no harm, for two reasons. The anchored and rolling folds
  are consumed through physical slices (`bars.get(fold.train.0)`), and the training execution prefix
  stops before `test_open` (docs/04-invariants.md UE-03). `purged_folds` is `pub`, has no caller outside
  its tests, and is the function the module offers for CSCV/PBO. A first caller would inherit a one-bar
  leak that the existing test cannot catch.
- **Repro (ran).** A throwaway integration test `crates/runner/tests/zz_num18_purge.rs` in a scratch
  worktree at 1f4de71. Setup: `synthetic::sessions(8)` (3,000 bars), `Column::build`, the all-firing mask
  `ConditionMask::default()`, `trade::walk` Long, h ∈ {2,3,5,15}, folds 2..=9. Results:
  - 16 trades crossed a `purged_folds` boundary.
  - `h=2 folds=5 train=(0..2398, ..) test=2400..3000`: trade signal 2397 exits at bar **2400 = test.start**.
  - `h=2 folds=3 test=1000..2000, right train 2002..3000`: test signal 1999 exits at bar **2002 = right_start**.
  - `h=3 folds=3 train 0..1997 test 2000..`: signal 1996 exits at 2000.
  - The same index arithmetic in `anchored_folds` was also crossed by full-series trades (h=2, n=2/5; h=3, n=8; h=15, n=8).
  - The production-style slice walk (`walk(&bars[fold.train.0], ..)`) never exited at or past `boundary`; that was asserted and passed.
  Worktree and target directory removed afterwards.
- **Minimal fix.** Purge and embargo `h + 1` bars: `left_end = test_start.saturating_sub(h + 1)` and
  `right_start = test_end.saturating_add(h + 1)`. Better, take the reach as an argument that the caller
  derives from its own entry convention: `+1` for `Sourced::Signal`, `+0` for `Sourced::Fill`.
  Change the module test to `window_end = i + 1 + h`, and add a test that walks real trades as above.
  Apply the same `+1` to the documented purge width of `anchored_folds` and `rolling_folds`, or change
  their docs to say that slicing, not the purge, is what separates the halves.

## Notes (not findings)

- **A. Whole-span support threshold.** When no support is named, `audit_bars` takes `min_hits` from
  `Sweeper::auto` over the full-span column (cli/src/lib.rs:13842 onwards). Every fold then rescales it
  (`fold_ladder`, validate.rs:1786). The test windows' condition frequencies therefore help set the
  training search depth. Only frequencies of feature bits enter. No price, outcome or label does, so no
  return information passes. Record it if a future change makes `auto` read outcomes.
- **B. Minute-only overlap check.** `ResolvedExitGridV1` OOS replay checks only
  `first_test_stamp <= training_last_ts_micros` (exit_grid_policy.rs:2819), not a later IST day, unlike
  expression_oos.rs:268. This is by design: walk-forward folds share a session across the purge. Every
  month-level caller (Global Replay via ledger_v6.rs:676 and stored_post_training_oos.rs:253) cannot reach
  the same session, because loads begin on day 1 of a strictly later month.
- **C. Combined-span consistency.** `boolean_qualification_v1.rs:288-340` evaluates training+later
  sessions jointly. `Record::outcome` (index_consistency_store.rs:80) requires all three periods to pass,
  which is conservative. The joint span does not let in-sample days rescue an OOS failure.
- **D. Pass 8's rows for walk-forward, expression OOS and qualification split** were re-derived and
  still hold at 1f4de71.

## Verification tally

No earlier fix IDs were in scope for this theme. Pass 8's "no leakage" rows were re-checked as follows.

| ID / row | Status | Evidence |
|---|---|---|
| p8 walk-forward / purge row | FIXED (holds) | validate.rs walk_forward_core: training prefix `before(test_open)`; `restricted` blanks rows < test.start; per-fold grid in V4 (:2536) |
| p8 expression OOS row | FIXED (holds) | expression_oos.rs:268-271, minute and IST-day check |
| p8 qualification split row | FIXED (holds) | index_stop_launch.rs:46; index_stop.rs:521-525, measurement window inside original-first history |

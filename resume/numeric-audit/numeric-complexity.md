# Numeric precision, rounding, look-ahead and O(1) audit

Audit of `SJParthi/brutex` branch `final/all-fixes` @ 331b05c (read only, no repo edits).
Angle: numeric precision, rounding, look-ahead bias in backtest logic, hot paths that are not O(1).
Per-slice raw reports: `zero-rounds/numeric/pass<N>/<slice>.md`. This file holds the merged, de-duplicated findings.

## Pass 1 (20 agents, 2026-10-03) — all 20 slices reported

Clean slices (no new findings): eng1, eng2 (brute-force oracle, 0 mismatches), sel, swp, ind2 (future-perturbation probe, 0 mismatches).
Full evidence and repro for each id is in `numeric/pass1/<slice>.md`.

### New findings

| id | sev | where | what | probe |
|---|---|---|---|---|
| pst-1 | medium | cli/src/population_statistics_v2.rs:3673-3682 | `build_raw_candidates` filters whole C·P and C·S vectors per candidate, O(C²); production path. C=2000 took 4.33s. Strided walk would be linear. Not in 06-limits. | ran |
| grk-1 | medium | api/src/server.rs:13368-13370, pull/src/pricing.rs:760-764 | Vendor IV unit unchecked: percent IV below 10 (e.g. 9.79) accepted as decimal 979% vol and stored as vendor figure. Charter §4b sample is percent. | ran |
| run1-1 | medium | runner/src/grid.rs:639-643 | `Cell::guaranteed_floor` charges flat (zero) trades as worst loss; floor drives variant selection and bound refusal. 1 win 500, 1 loss -100, 9 flats → -500 vs true +400. | ran |
| xcut-1 | medium | api/src/livejson.rs:229 | `/live.json` `clears_bar` skips report.rs:604-636 refusals (n<30, mismatched>0); web counts such rows as proved. n=5 t=9 → clears_bar=true. | ran |
| run1-2 | low | runner/src/grid.rs:557-562 | `Cell::avg_loss` divides by losers+flats (Edge::payoff_bp already fixed this). | ran |
| run1-3 | low | runner/src/outcome.rs:1574, cli/src/lib.rs:17894, api/src/livejson.rs:229 | t rounded to milli vs threshold truncated → clears_bar true just below the bar. Fix with xcut-1 (carry verdict from runner). | ran |
| run2-1 | low | runner/src/grid.rs:5219-5283 vs excursion.rs:1185,1462 | Stop placed at floor level but trigger needs ceil move: a bar printing exactly the stop does not stop out (1 paisa gap, longs/shorts/trailing). | ran |
| run2-2 | low | runner/src/exit_grid_policy.rs:2460-2466 | `OperatorRule` selector ranks `reward_to_risk_bp`=i64::MAX for worst_trade>=0 without wins/trade-count guard (grid.rs:1057-1077 guards it). | ran |
| run3-1 | low | runner/src/admission.rs:1684, 1024-1031 | Floored `losing_trade_rate_ppm` lands on ceiling when exact rate is just above (1/3 vs 333333). Same class: cli population_base_evidence_v2.rs:593 largest_trade_profit_share_ppm (unprobed). | ran |
| pst-2 | low | cli/src/population_observations_v1.rs:1366-1393 | Every CSCV split re-walks every period; segment sums give same result ~83x faster. | ran |
| pst-3 | low | cli/src/population_statistics_v2.rs:4981 | `cscv_placement` strict `>` excludes median tie from overfit; Bailey λ≤0 reading is UNVERIFIED (no charter source). Needs decision. | ran |
| grk-2 | low | pull/src/tenor.rs:251-255 (callers server.rs:11907/11923, 13341/13376) | Tenor measured from bar open stamp but premium is bar close: one bar too long; 15:39 bar priced with 60s left instead of AlreadyExpired. apis-1 is the same defect (adds: daily cadence gap, inferred). | ran |
| STO-1 | low | store/src/format.rs:1007-1009 | `Overlay::is_sane` always true: negative spot / iv commit. | ran |
| STO-2 | low | pull/src/rolling.rs:633, 841-847 | Unreadable IV cell silently filed as null (hidden failure, §4); negative IV accepted, rounds half-away. | ran |
| pul-1 | low | pull/src/fold.rs:229-237, 312-323 | Fold grid drifts off 09:15 when width does not divide 86400; `complete_minutes` accepts the short pre-open bar. Public API only; docs at :699-704 claim 7s/90s correct. | ran |
| core-1 | low | core/src/instrument.rs:391-395 | Doc says contract names sort by strike; strike is unpadded text (₹5,000 > ₹15,000). | ran |
| apir-1 | low | api/src/render.rs:2354-2366 | `page_peak` shades one feed using all-vendor max. | ran |
| apir-2 | low | api/src/render.rs:1376-1391 | `share_bar` gives rounding remainder to last reason even at count 0. | ran |
| rep-1 | info | cli/src/global_replay.rs:750-768, global_replay_v2.rs:3480-3498 | `find_offered_stream` rescans offered streams per decision, up to 200² per minute; docs claim ≤200 heads. O(1) possible via slot index. | inferred |
| pst-4 | info | cli/src/population_statistics_v2.rs:4996-5005, v3.rs:3225 | `wilson_lower(0,n)` ≈ 2e-19 not 0 (rounds to 0 ppm, no decision change). | ran |
| apir-3 | info | api/src/render.rs:517-522 | Negative sub-rupee strike loses sign (unreachable today). | ran |
| apis-2 | info | api/src/server.rs:13338 | Lost `\` line continuation: 18-space gap in refusal text. | read |
| clib-1 | low | cli/src/lib.rs:11391, 8358-8360 | Statistical support floor floored twice (trades→ppm→hits): min_hits = needed-1. needed=47 → 46. Changes run identity: needs decision entry. | ran |
| clib-2 | low | cli/src/lib.rs:13812-13820 | `Cadence::trades_over` floors weeks before multiplying; doc says divide once at end. 10/wk, 1 month: 40 vs 43 trades. Needs decision entry (docs contradict). | ran |
| ind1-1 | low | indicators/src/vwap.rs:615-618 (floors :309, :393) | VWAP bands built as floor(vwap) ± m·floor(σ): edge off by up to 1+m paisa, flips bits 146-152, 190-197 (~0.17% of answers on random walk). Repro in ind1.md. | ran |
| ind1-2 | low | indicators/src/trend.rs:228 + :1017; vwap.rs:309 vs :518-525; session.rs:476-482 | Level floored to paisa then strict `<`: close equal to floored fractional level reports neither side; 'below' under-reported (bits 1/3, 53/144, 67). EMA20 at 19,999 on flat 20,000 sets 'above' forever. | ran |

### Known items confirmed STILL OPEN at 331b05c (user wants every finding fixed)

| id | where | status |
|---|---|---|
| hunt-costs-1 (medium) | costs/src/trip.rs:1222-1226 | STT rate taken from entry day for long's exit sell leg; probe re-confirmed (₹120 vs ₹180). 06-limits §27 "only over-charges" is wrong. |
| hunt-runner-1 | runner ExitChoices::of | target first touched on horizon bar still beats time exit |
| errpaths-3 | runner/src/grid.rs:2131 | invalid stop ladder silently becomes empty (`unwrap_or_default`) |
| hunt-pull-1 | pull F&O gap audit | still uses 375-minute session |
| hunt-store-1 | store bar door | accepts overlay/greeks geometries |
| gaps-7 | core universe | survivorship: today's F&O list applied to history (ST-02) |

### Plan (throttled 2026-10-03 13:14 UTC at user's request: 5-hour usage 56%)

1. Pass 1 done (26 new, 6 still-open known). No new agents until after 18:00 UTC.
2. Hand the table above to the zero-findings thread to fix.
3. Pass 2 at 3 agents at a time, focused on areas pass 1 did not cover deeply: cli lib.rs remainder, population_base_evidence_v2 (run3-1 sibling), runner report.rs/topn.rs/bound.rs/excursion.rs/trade.rs, api livejson.rs + web/ number paths, cli verify/boolean/candidate modules. Then re-check the pass-1 areas that had findings.
4. Repeat until a pass finds nothing new.

## Pass 2 (resumed 2026-10-03 18:06 UTC, max 2 agents at a time; weekly usage 82%)

Target: origin/final/all-fixes-zero @ 5140aca. Raw reports: `numeric/pass2/<slice>.md`.
- Wave 1 (running): p2run = runner report/topn/bound/excursion/trade + api livejson; p2bool = cli boolean_statistics*/oos/qualification/admission + population_base_evidence_v2 + boolean_search_sizing.
- Wave 2 (queued): cli lib.rs lines 20359..end + candidate_trades; web/ number paths + remaining cli boolean_* readers/codecs.
- Wave 3 (queued): p2stat runner bootstrap/pbo/significance/rank/split/allocation (running); p2expr runner expression*/signal_candle_stop/resample/align/closed/research_exit_grid; p2idx cli index_stop_*/expression_pricing/frontier/stability; p2misc cli pool/results/trades/minute_gaps/vix_reference/columns/sweep_evidence/live/v6_statistics_adapter.
- Re-check of pass-1 areas waits until the fix thread pushes (no new commits on final/all-fixes-zero as of 18:20 UTC).
- Stop rule: weekly usage 93% → save state to GitHub; 98% → stop.

### Pass 2 results (8 agents, 2 at a time, 18:06–18:50 UTC) — 7 new + 2 documented-only items worth fixing

Clean: p2clib (lib.rs has no production code past 20358; candidate_trades), p2stat (bootstrap/pbo/significance/rank/split/allocation), p2expr (expression*/signal_candle_stop/resample/align/closed).

| id | sev | where | what | probe |
|---|---|---|---|---|
| p2bool-1 | low | cli/src/population_base_evidence_v2.rs:586-594 (`rate_ppm` :1582-1591) | `largest_trade_profit_share_ppm` floored then `value > ceiling`: 400001/2000000 = 200000.5 ppm stored 200000, passes a 200000 max (real `AdmissionPolicyV1` → failed=0). Same pattern: session_concentration, ambiguous_fill, gap_affected in this file. Fix with `div_ceil` for max-gated values; decision entry needed. | ran |
| p2inst-1 | low | cli/src/institutional_evidence.rs:2009-2018 (`rate_ppm`), fields :1722, :1731, :1437-1438 | Same floor-vs-max leak in `reconcile_trade_rows`, reaching institutional evidence and Boolean admission (`boolean_admission_v1::base_values` :193). Case A 200000.5→200000 passes; case B 1/3 sessions 333333.33→333333 passes. Contradicts the file's own doc at :2079-2089. | ran |
| p2idx-1 | low | cli/src/index_stop_qualification_metrics.rs:142, 174-185 | Same leak on the four max-gated rates for single-stop qualification. Cannot fix in the shared closure: it also produces min-gated `win_rate_ppm` (:162). | inferred (same helper as p2bool-1) |
| p2run-1 | low | runner/src/report.rs:383-411 (+329-352) vs :556, :605 | With 1 trial, SIGNIFICANCE prints "threshold -" while FINDINGS says "bar 1.96" and marks rows clears. | ran (branches) |
| p2misc-1 | low | cli/src/pool.rs:41-42, 928, 971-972; docs/06-limits.md §171 line 8495 | `dd>=` (max single-instrument drawdown) is labelled a lower bound on pooled drawdown and used as 2nd sort key, but bounds nothing: A −100,−100 and B +150 between → reported 200, true 100. | ran |
| p2misc-2 | low | cli/src/minute_gaps.rs:206-223 | `days_with_interior_gaps` never withholds a day missing 09:15 or 15:29; the missing close then makes the exact-minute join refuse the whole span (`MissingClosingMinute`, inferred end-to-end). Fix bumps `MINUTE_GAP_POLICY`. | ran |
| p2bool-2 | info | cli/src/boolean_statistics_numeric_v1.rs:181-212 | `split_scores` re-walks every period per CSCV split (sibling of pst-2); declared and capped. | inferred |

Documented-only but worth fixing (user wants nothing left "documented only"):
- **D-0742 bootstrap block length never checked against period count** (cli callers refuse only 0). p2stat probe: 400 periods, 3 pure-noise strategies, 999 draws, 40 families — block ≥ 40000 gives 36/40 passing p<0.05 on White, SPA and Romano-Wolf; block 10 gives 2/40. Fix: refuse block > periods.
- **D-0743 PBO floor** at cli/src/index_stop_qualification_numeric.rs:608 (`pbo_ppm = ….ppm()` into a max-gated field) while the four p-values in the same function round up (:667-670). Same class in `boolean_admission_reader.rs:223-246` and `boolean_admission_v1::apply_statistics` :291-321.

Next: pass 3 (2 agents): workspace-wide sweep for every floor-then-max-gated projection (the class found 5 times), and the cli/api files no pass has covered yet.

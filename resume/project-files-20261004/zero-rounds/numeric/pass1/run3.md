# run3: runner/src/validate.rs and runner/src/admission.rs (numeric, look-ahead, O(1))

**Verdict.** One new low-severity finding. A probe that ran confirmed it. Walk-forward leakage, fold arithmetic, the exact-fraction probabilities and the Wilson projection all held up when checked adversarially. Neither file has a Sharpe, t-statistic or bootstrap of its own: those live in outcome.rs, bootstrap.rs and significance.rs, which other slices cover. The statistical surface here is the exact `numerator/(draws+1)` p-values, the Wilson lower bound, and the ppm projections that feed the fixed V1 policy.

## Findings

### run3-1 (low): the canonical floor projection lets a max-gated losing-trade rate pass a ceiling its exact fraction exceeds

- Projection, admission.rs:1684 (`validate_count_rate`):
  `let Ok(projected) = u64::try_from(u128::from(count) * u128::from(PPM) / u128::from(total))`
  followed by `if projected != rate { ... RateCountMismatch }`. This makes the floor the only `losing_trade_rate_ppm` the evidence constructor accepts. The call is at admission.rs:1655.
- Gate, admission.rs:1024-1031 (`check_max_u64(e.losing_trade_rate_ppm, p.max_losing_trade_rate_ppm, AdmissionReasonV1::LosingTradeRate, ..)`). It fails only when `value > ceiling`.
- Why it is wrong: if `losses/trades` sits just above `ceiling/1e6`, the floor lands on the ceiling and the gate passes. This is the GAP5-49 defect class (D-0743), which was fixed only for the five probabilities in `floor_hidden_ceiling_v2` (admission.rs:4175-4202: PBO, FWER, SPA, White, candidate RW). The losing rate is different from the other rate fields in one way: its exact numerator and denominator (`losing_trades`, `trades`) are in the same evidence record. So the runner has everything it needs to decide this exactly, and it still decides on the floor. The V2/V3 projection doors do not refuse it either, because `floor_hidden_ceiling_v2` never looks at this field.
- Repro (probe `$SCRATCH/probes/run3`, depends on `runner` only). The policy is all-permissive except `max_losing_trade_rate_ppm`. The evidence carries `trades`, `winning_trades = trades - losses`, `losing_trades` and the canonical floor rate. Verbatim output:
```
ceiling=333333 trades=3 losses=1 exact_ppm=333333.333333 floor=333333
  ceil value 333334 accepted? Err(RateCountMismatch)
  LosingTradeRate failed? false  status=Unmeasured
ceiling=142857 trades=7 losses=1 exact_ppm=142857.142857 floor=142857
  ceil value 142858 accepted? Err(RateCountMismatch)
  LosingTradeRate failed? false  status=Unmeasured
ceiling=285714 trades=7 losses=2 exact_ppm=285714.285714 floor=285714
  ceil value 285715 accepted? Err(RateCountMismatch)
  LosingTradeRate failed? false  status=Unmeasured
ceiling=90909 trades=11 losses=1 exact_ppm=90909.090909 floor=90909
  ceil value 90910 accepted? Err(RateCountMismatch)
  LosingTradeRate failed? false  status=Unmeasured
```
  Expected: `LosingTradeRate` fails, because 1/3 > 333,333/1e6. Actual: it does not fail. Rounding up (`ceil`) is refused as `RateCountMismatch`, so no caller can supply a conservative value. (`status=Unmeasured` comes from the probe leaving the other fields unmeasured; the gate bit itself is what is being tested.)
- Scope and size: the overshoot is under 1 ppm. With round ceilings such as 300,000 it needs more than about 1e5 trades. With ceilings that are not exact in ppm (1/3, 1/7, 2/7, 1/11 as above) it happens at single-digit trade counts.
- Related, outside this slice and not probed: the same floor-on-max problem applies to `largest_trade_profit_share_ppm`. Its denominator is gross winning paisa, so the case is easy to reach. `cli/population_base_evidence_v2.rs:593-598` computes it as `rate_ppm(best_trade, gross_win)`, a floor. For example, best = 2,000,001 and gross = 10,000,001 paisa gives 200,000.08 ppm, which floors to 200,000 and passes a 200,000 ceiling (INFERRED arithmetic). `ambiguous_fill_rate_ppm` and `gap_affected_rate_ppm` are only exposed above about 1e6 trades.
- Fix direction (not applied): evaluate max-gated rates as `count * PPM <= total * ceiling` from the measured counts, as `rejects_at_ppm` already does. Alternatively, refuse floor-hidden values the way D-0743 does.

## Checked and clean

- **Walk-forward IS/OOS separation** (validate.rs:4350-5127, the V4 path 2447-2903):
  - The mask, side (`direction_from_training_edge` over `forward_over(trade_train..)`), exit cell and ladders all come from training only.
  - The training execution prefix is `before(test_open)`, which is strict.
  - The OOS column is built from bars `..test.end` with bits cleared before `test.start`, and its execution path runs through `last close + horizon`.
  - V4 re-resolves rungs on the fold's training prefix (`resolve_attested(train_series)`) and uses `project_oos_fold`, which physically removes pre-OOS sources.
  - Folds in split.rs: anchored and rolling test windows are contiguous and disjoint, with an `h`-bar purge. No test window overlaps its own training range. A trade cannot straddle into the test window, because the training trade path ends before `test_open`.
- **`scale_min_hits`**: a ratio of lengths, ceiling division in u128, saturating, floored at 1. It is not fitted to OOS data.
- **Selection ties**: strict `>` over i64 in collected order, which is deterministic. Reconciliation uses `max()` + `position`, consistent with the first strict maximum.
- **Aggregates**: `aggregate_oos_paisa` uses `checked_add` with a named refusal in V2, V3 and V4. Fold counts saturate or are checked.
- **No floats in validate.rs** other than `direction_from_training_edge(f64)`. Its input is a Welford mean seeded at 0.0 over i64 paisa, so it cannot be NaN.
- **Wilson** (admission.rs:3370-3385): `trades == 0` maps to 0. `wins.min(trades)` gives p <= 1, so the sqrt argument is >= 0. The formula is bit-identical to cli population_statistics_v2/v3. The ppm floor is only min-gated, so it errs conservative. `wins > trades` is refused before the call (3240, 3547, and research_admission_projection.rs:110).
- **Exact probabilities**: the numerator must be > 0, and the denominator must equal `draws + 1` (checked_add). `ppm()` floors with numerator <= denominator. `rejects_at_ppm` uses exact u128 cross-multiplication. PBO counts reconcile with the CSCV split count. Romano-Wolf ordering is checked.
- **Count-rate reconciliation**: u128, the zero denominator refused (`RateWithZeroDenominator`), overflow refused. Min-gated rates (win rate, Wilson) floor in the conservative direction.
- **O(1)**: per-fold `SliceFacts` and `forward` are hoisted out of the candidate loops. `first_live` is one scan per fold. `MonotonicExecutionPrefix` is O(E + F) in total. The argmax re-checks are O(retained) per fold, already documented in docs/06-limits.md (11398, 13786-13790).
- **Known items not repeated**: errpaths-9 (`fold_rungs` env fallback), gaps-3 (the admission and V3 doors are unreachable from production), rolling-fold cold start (validate.rs:1909-1924), GAP5-49 / D-0743.

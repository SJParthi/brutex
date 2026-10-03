# Pass 2 — slice p2bool

Commit 5140aca (origin/final/all-fixes-zero). Audit only. Nothing in the repo was edited.

## Verdict

One new confirmed finding (low). It is the probe pass 1 asked for on its run3-1 note, and it **confirms** the suspected defect at population_base_evidence_v2.rs:593. One info-level complexity note, a sibling of pst-2. The p-value floor in `boolean_admission_v1` is already documented (06-limits, D-0743), so it is listed under known items, not as a finding.

## Findings

### p2bool-1 — low — crates/cli/src/population_base_evidence_v2.rs:586-594 (helper :1582-1591)

```rust
largest_trade_profit_share_ppm: if a.gross_win > 0 {
    ObservedU64V1::Measured(rate_ppm(
        best_trade,
        gross_win,
        "largest trade profit share",
    )?)
```
`rate_ppm` floors (`scaled.checked_div(denominator)`). The admission policy then compares it to a **ceiling** with
`check_max_u64`, which tests `value > ceiling` (runner/src/admission.rs:2891). A share just above the ceiling floors onto it and passes.
Both terms are paisa sums, so the denominator is large, and an exact share of up to ceiling + 1 ppm − ε passes for **round** ceilings such as 200000.

Same class, same file and same helper, all floored and all max-gated:
- `session_concentration_ppm` :577-582
- `ambiguous_fill_rate_ppm` :571-575
- `gap_affected_rate_ppm` :576

Their denominators are trade counts, so they leak only at non-round ceilings or above 1e6 trades (INFERRED from the arithmetic). The Boolean admission path (`boolean_admission_v1::base_values` → `institutional_evidence::reconcile_trade_rows`, which floors at institutional_evidence.rs:2009) fills the same two fields the same way (INFERRED, not probed; outside this slice). `losing_trade_rate_ppm` :603 is the known run3-1 and is not repeated here.

Fix direction: project max-gated rates with `div_ceil`, as `boolean_qualification_v1::upper_ppm` (:740-745) already does for p-values. Alternatively, refuse floor-hidden values as D-0743 did for probabilities. This changes stored evidence bytes, so it needs a decision entry.

**Probe (ran)**: `$SCRATCH/probes/p2bool`. It is the cli `rate_ppm` copied verbatim, fed through the real `runner::admission::AdmissionPolicyV1::evaluate_research_projection` with `max_largest_trade_profit_share_ppm = 200_000`. Failed bit 65536 = LargestTradeProfitShare. Verbatim output:
```
best=400000 gross=2000000 share_ppm=200000 exact>ceiling(200000)=false status=Unmeasured verdict=AdmissionVerdictV1 { reasons: ReasonBits(17587891084864), failed: ReasonBits(0), unmeasured: ReasonBits(17587891084864), refused: ReasonBits(0), status: Unmeasured }
best=400001 gross=2000000 share_ppm=200000 exact>ceiling(200000)=true status=Unmeasured verdict=AdmissionVerdictV1 { reasons: ReasonBits(17587891084864), failed: ReasonBits(0), unmeasured: ReasonBits(17587891084864), refused: ReasonBits(0), status: Unmeasured }
best=400002 gross=2000000 share_ppm=200001 exact>ceiling(200000)=true status=Unmeasured verdict=AdmissionVerdictV1 { reasons: ReasonBits(17587891150400), failed: ReasonBits(65536), unmeasured: ReasonBits(17587891084864), refused: ReasonBits(0), status: Unmeasured }
best=1 gross=4 share_ppm=250000 exact>ceiling(200000)=true status=Unmeasured verdict=AdmissionVerdictV1 { reasons: ReasonBits(17587891150400), failed: ReasonBits(65536), unmeasured: ReasonBits(17587891084864), refused: ReasonBits(0), status: Unmeasured }
```
- Expected: a best trade of 400001 paisa out of a gross win of 2000000 is an exact share of 200000.5 ppm, which is above the 200000 ceiling, so the reason is failed.
- Actual: the reason is not failed (failed=0).

(Overall status is Unmeasured only because the probe leaves the statistics fields unmeasured. The failed bit for this reason is what is under test.)

### p2bool-2 — info — crates/cli/src/boolean_statistics_numeric_v1.rs:181-212 (`split_scores`, called per split per candidate at :167)

For each of the C(S, S/2) CSCV splits, every candidate's full period row is re-walked: the cost is C·P·splits. The bound is declared and gated (`split_work = observations * splits`, boolean_statistics_v1.rs:309-311, refused above `bounds.split_work`), so this is not a hidden cost. However, per-candidate segment sums (one O(P) pass) would make each split O(S) per candidate, i.e. C·(P + S·splits). This is the same pattern as pass-1 pst-2 (population_observations_v1.rs), in a file pst-2 did not name. INFERRED speed-up, not timed.

## Known, code matches documented status (not new)

- `boolean_admission_v1::apply_statistics` (:291-321) projects the White, SPA, FWER, RW and PBO probabilities with `AdmissionExactProbabilityV2::ppm()` (floor), and these are then max-gated. 06-limits §D-0743 states: "The cli V1 builders that fill the same fields with `.ppm()` are not changed by D-0743." The leak needs a denominator above 1e6/(granularity) for round ceilings: bootstrap draws+1 > 50000 for a 50000 ceiling, so PBO (≤12870 folds) cannot leak at round ceilings. The exact `hypothesis_decision` path is unaffected.
- Losing-trade-rate floor (run3-1).
- `cscv_placement` tie handling (pst-3).

## Checked and clean

- boolean_statistics_wire_v1 / boolean_qualification_v1: the RW observed statistic is stored as raw `f64::to_bits()` (not rounded). p-values travel as exact numerator/denominator.
- boolean_qualification_v1 `upper_ppm` uses `div_ceil` for max-gated p-values (correct direction). Wilson is rechecked through the canonical bits (`wilson_ppm`).
- boolean_statistics_numeric_v1 `project_row`: checked adds, and conservation against the priced cell (pessimistic, trades, wins). Period/day alignment is refused on mismatch. Splits must partition the periods, or the run refuses.
- population_base_evidence_v2: min-gated ratios (win rate, worst reward/risk, profit factor, return/drawdown) are floored, which is conservative for a floor gate. `return_drawdown` with profit ≤ 0 → 0, and with dd = 0 → u64::MAX (no division by zero). Averages use `checked_div` → Unmeasured at zero. The aggregator uses checked i64 adds, refuses overlapping or out-of-order rows, and refuses worst > best.
- boolean_oos_v1: later months must be strictly after the training `to` (:200, `later.from <= context.to` refused). Programs and anchors come from the committed training family and are crosswire-checked (:437-452). No in-sample/out-of-sample leakage found. Qualification keeps the training PBO and labels later folds as fixed-training checks, not retraining.
- boolean_search_sizing: the decimal 2^n−1 builder is exact (u8 digit ≤ 19), with capacity checked against a 384-bit mask. There are no floats.
- boolean_admission_v1: losses use `checked_sub`, and `positive()` refuses a negative gross win or min win.

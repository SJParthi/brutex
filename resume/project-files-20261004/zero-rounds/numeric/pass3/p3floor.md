# p3floor: workspace sweep for floor-then-max (and ceil-then-min) gate leaks

Target: origin/final/all-fixes-zero @ 5140aca. Audit only. Probe: `$SCRATCH/probes/p3floor` (path deps on runner + indicators), ran.

## Verdict

2 NEW leaks, both probed. Every other gated field I traced is SAFE or already known (tables below).

## Findings

### p3floor-1 (low): `average_loss_paisa` is floored, then gated against `max_average_loss_paisa`

The mean loss is integer division (`u64 /`, or `i64 /` which truncates toward zero, so the magnitude rounds down). The result goes into the max-gated `average_loss_paisa`. At four production sites:

- `cli/src/population_base_evidence_v2.rs:610-612`: `average_loss_paisa: gross_loss.checked_div(a.losses)`
- `cli/src/boolean_admission_v1.rs:250-252`: `average_loss_paisa: gross_loss.checked_div(losses)`
- `cli/src/index_stop_qualification_metrics.rs:210-213`: `average_loss_paisa: t.gross_loss.checked_div(losses)`
- `cli/src/institutional_evidence.rs:1412`: `ObservedU64V1::Measured(cell.avg_loss().unsigned_abs())`, where `runner/src/grid.rs:562` is `self.gross_loss.saturating_div(losers.cast_signed())`

The gate is `runner/src/admission.rs:1064-1066` (`check_max_u64(e.average_loss_paisa, p.max_average_loss_paisa, ..)`), which fails only when `value > ceiling` (:2883-2893). The ceiling is operator-settable: `cli/src/ledger_all.rs:476` `r.gate("max_average_loss_paisa", None)`. The cell metric `TopMetricsV1.average_loss` (`population_admission_writer.rs:1311`) is reconciled against the same evidence field (:1580s), so Population V1 carries the same value.

Repro (ran). Cell: 3 trades, 1 win, gross_loss = -301 paisa. The exact mean loss is 150.5 paisa. Policy: max 150.
```
== p3floor-1 average_loss_paisa floor vs max_average_loss_paisa ==
exact mean loss = 150.5  Cell::avg_loss()=-150  -> institutional=150  base_v2/boolean/index_stop=150
max_average_loss_paisa=150: AverageLoss failed? false
control avg=151 (ceil): AverageLoss failed? true
```
Expected: AverageLoss failed (150.5 > 150). Actual: it passes.

Fix: use `div_ceil` for this max-gated mean. `average_win_paisa` is min-gated, so its floor is correct. Changes stored evidence bytes, so it needs a decision entry. Not in numeric-complexity.md, 06-limits or 11-findings. p2idx.md mentions average_loss only for the flat-counts-as-loss rule.

### p3floor-2 (low): `Cell::worst_mae` is a floored ppm, then gated by `worst_mae <= max_mae_ppm`

`runner/src/excursion.rs:1466` (`ppm_of`): `i128::from(move_paisa).saturating_mul(i128::from(PPM_ONE)) / i128::from(entry)` floors. It is reached through `Crossings::adverse_ppm_at` (:751) and `grid.rs:4399`: `let went_against = c.cross.adverse_ppm_at(pess_off, c.entry_pess, side);`, then `cell.worst_mae = went_against` (:4454).

Gates:
- `cli/src/lib.rs:8911` `Rules::admits`: `(self.max_mae_ppm == 0 || cell.worst_mae <= self.max_mae_ppm)`
- `cli/src/lib.rs:13045` screen reason: `rules.max_mae_ppm > 0 && cell.worst_mae > rules.max_mae_ppm`
- `cli/src/frontier.rs` verdict (doc :3134)

`max_mae_ppm` is typed directly through `BRUTEX_MAX_MAE_PPM` (lib.rs:9144) or the api sweeprun `max_mae_ppm` field (api/src/sweeprun.rs:687).

Repro (ran, through the real `excursion::crossings` + `adverse_ppm_at`). Entry 2,000,001 paisa; a long's low is 20,001 paisa below entry. The gate effect is the quoted one-line comparison, evaluated in the probe:
```
== p3floor-2 worst_mae floor vs Rules::max_mae_ppm ==
exact adverse ppm = 10000.495000  adverse_ppm_at = 10000
Rules::admits clause `cell.worst_mae <= max_mae_ppm` with max_mae_ppm=10000: true
screen reason clause `worst_mae > max_mae_ppm` (MAE refusal): false
```
Expected: MAE refusal (10000.495 > 10000). Actual: admitted.

Ways to fix:
- Compare exactly: `move * PPM > max * entry`.
- Or store a ceiled MAE. `ppm_of` also feeds rung crossing (run2-1 territory), so a ceil there is a separate decision. Splitting the MAE projection from the crossing projection is the narrow fix.

This is distinct from run2-1 (stop trigger vs stop level).

## Known, re-confirmed at 5140aca (not re-reported)

run3-1, p2bool-1, p2inst-1, p2idx-1, run1-3, D-0743 (cli V1 builders), clib-1, clib-2.

Additional call sites of the known p2bool-1 helper: `boolean_admission_v1::base_values` uses `population_base_evidence_v2::measured_rate` for `ambiguous_fill_rate_ppm`, `gap_affected_rate_ppm` and `losing_trade_rate_ppm` (around :229-243). The same floor reaches the max gates there. A helper-level fix covers it. Listed so the fix thread does not miss the caller. clib-1's location (`lib.rs:8358-8360`, `min_hits_for`) also covers a typed `SUPPORT_PPM` that does not divide evenly. Example: 1000 bars at 1500 ppm floors to 1 hit.

## Classified SAFE (next pass can skip)

| field / gate | producer | why safe |
|---|---|---|
| admission min-gated `win_rate_ppm`, `worst_reward_risk_ppm`, `profit_factor_ppm`, `return_drawdown_ppm`, `average_win_paisa` (pop_base_v2, boolean_admission_v1, index_stop_metrics, institutional) | floor `rate_ppm`/`ratio_observed`/`/` | floor vs minimum is conservative |
| `wilson_win_rate_ppm` | runner admission.rs:3383, research_admission_projection.rs:109, institutional_evidence.rs:2053, population_statistics_v2.rs:2767, v3.rs:986, population_admission_writer `assurance_bp*100` | floor vs minimum |
| `max_mae_paisa`, `drawdown_paisa`, `worst_trade_loss_paisa`, `losing_trades`, `consecutive_*_streak`, `pbo_unrankable_folds`, `weakest_period_return_paisa`, `pessimistic_profit_paisa`, `oos_pessimistic_return_paisa` | exact integers | no projection |
| institutional `fwer_p_value_ppm`, `romano_wolf_p_value_ppm` | `institutional_statistics.rs:782/789` `ceiling_ppm` | ceil vs max (stored floor at :460 is identity-only, reconciled at :206) |
| institutional `white/spa_p_value_ppm` | `BootstrapFraction::ppm` institutional_evidence.rs:2104 | ceil (D-0930) |
| boolean_qualification_v1 p-values :771-781, boolean_search_projection.rs:102 | `upper_ppm` div_ceil | ceil vs max |
| index_stop_qualification_numeric four p-values :667-670 | div_ceil | ceil |
| runner AdmissionEvidenceV2/V3 statistics `.ppm()` floors (admission.rs:4093-4111, 4436-4454) | floor | guarded by `floor_hidden_ceiling_v2/v3` (:4175, used :4767, :4826, :4857) |
| `rejects_at_ppm` (admission.rs:3012, bootstrap.rs:995, institutional_statistics.rs:794, institutional_evidence.rs:2109) | exact cross-multiply | exact |
| `search/family_allocation_v1::minimum_draws` + `require_draws` | `div_ceil(...) - 1`, `draws < minimum` | exact inverse of 1/(D+1) <= t |
| `bootstrap::Verdict::clears` `p_value < 0.05` | correctly rounded f64 k/d | equality only at d > ~1e16 |
| runner `Cell::win_rate_bp`, `reward_to_risk_bp`, `return_over_drawdown`, `assurance_bp` (grid.rs) vs `Rules::admits`, `Bound::verdict`, `Cell::clears`, frontier verdict | floor | floor vs minimum |
| `Rules::fills_hold` (lib.rs:8999), `avg_payoff_holds` (:9043) | exact cross-multiply | exact |
| `Consistency::weakest_bp` / `stability::positive_share_bp` vs `min_weakest_bp` | floor | floor vs minimum |
| pool.rs `tail_bp`, `profit_factor_bp`, `meets` | floor | floor vs minimum |
| exit-grid ratio bitmap (resolved_grid_view.rs:366, exit_grid_policy.rs:839) | `target*100` vs `stop*bound` cross-multiply | exact |
| exit-grid `ambiguous_bars`/`gapped` limits (resolved_grid_view.rs:416-421) | counts | exact |
| `index_consistency` weekly wins/losses (:855-856) | counts | exact |
| `breakeven_rr_bp` (lib.rs:10962) | not called in non-test build | n/a |
| Population V1 `TopMetricsV1` `losing_rate_ppm`, `loss_ratio_ppm`, `average_loss` in selection V1-V6 | floor | ranking keys/encoding only, no threshold. The admission side is p3floor-1/run3-1 |
| outcome.rs `payoff_bp`/`path_ratio_bp`/`worst_reward_risk_bp` | float truncation | ranking only (rank.rs) |
| api constituents `CEILING_PERMILLE` | test-only | n/a |

Not re-swept (covered by earlier passes): indicators/vocab tolerance bands (ind1/ind2), costs.

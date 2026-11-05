# gaps-LS2.txt.md: runner coverage gaps by Route B

Shard BASE-COV-LIBS-LS2 (crate runner). FINAL: these lists are from the final Route B at code tip 473f62e05b5b (11:41-13:11 IST, 5 Nov), which measures every code commit of this shard.
One fenced block below holds the list.

`````text
Route B (d3-route-b-cloud.sh, pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0-rust-1.99.0-nightly) on tip 473f62e05b5b7220ad3c267a2a8e066a51c1188f, set reading whole:runner.
Log: /tmp/claude-0/-home-user-brutex/6abe04da-0adb-571f-8e68-25c7af4be0c4/scratchpad/logs/473f62e05b5b-routeb.log (ROUTE-B DONE 2026-11-05 13:11:10 IST; tests passed 980 failed 0 ignored 0)
TOTAL runner: lines 51836/52061 (uncovered 225), branches 3666/4014 (uncovered 348), files read 74; NOT READ: crates/runner/benches/ratio.rs ([[bench]] test = false: no test object; L-BCL-1).
Rows: 573 uncovered lines and branch sides = 274 (F) (in fu-LS2.txt.md) + 299 others below.
Of the 299: [U] 258 (proof in fu-LS2.txt.md), [FAULT] 36 (allocator or address-space bound, operator ruling), [HASH] 5 (a blake3 collision only, operator ruling), [UNPROVEN] 0 (believed unreachable, no proof claimed), [open] 0 (0 production, 0 test code).
Tags: rows on lines unchanged since 2b998e4b5f6e (followed through the diff where a line moved) keep the earlier lists' tags; this session's proofs and (F) decisions are applied by row; a row the earlier lists never saw is (F) only by the recipe's step-4 rule, read from the source, else [open]. Line numbers here are as at 473f62e05b5b; a [U] row whose proof names it by its line at 2b998e4b5f6e says so (only exit_grid_policy.rs lines moved under a proof; see the head of fu-LS2.txt.md).

PER FILE (lines a/b, branches c/d)
  crates/runner/src/admission.rs lines 6265/6270 branches 305/310  GAPS
  crates/runner/src/admission_reader_cover_tests.rs lines 445/445 branches 6/8  GAPS
  crates/runner/src/admission_statistics_cover_tests.rs lines 235/235 branches 0/0
  crates/runner/src/align.rs lines 147/147 branches 8/8
  crates/runner/src/audit.rs lines 2119/2119 branches 141/154  GAPS
  crates/runner/src/bootstrap.rs lines 1789/1810 branches 157/178  GAPS
  crates/runner/src/bootstrap_family_pass.rs lines 269/269 branches 35/36  GAPS
  crates/runner/src/bootstrap_family_pass_tests.rs lines 387/387 branches 6/8  GAPS
  crates/runner/src/bootstrap_zero_v2.rs lines 323/324 branches 48/50  GAPS
  crates/runner/src/bootstrap_zero_v2_tests.rs lines 426/426 branches 20/20
  crates/runner/src/bound.rs lines 228/228 branches 14/14
  crates/runner/src/closed.rs lines 146/146 branches 10/10
  crates/runner/src/excursion.rs lines 1130/1136 branches 112/120  GAPS
  crates/runner/src/excursion_clock_tests.rs lines 89/89 branches 8/8
  crates/runner/src/exit_grid_policy.rs lines 5840/5874 branches 304/336  GAPS
  crates/runner/src/exit_grid_policy_cover_tests.rs lines 90/90 branches 0/0
  crates/runner/src/expression.rs lines 212/212 branches 6/6
  crates/runner/src/expression_execution.rs lines 334/335 branches 29/30  GAPS
  crates/runner/src/expression_execution_tests.rs lines 504/504 branches 0/0
  crates/runner/src/expression_oos.rs lines 250/252 branches 19/20  GAPS
  crates/runner/src/expression_oos_tests.rs lines 400/400 branches 6/6
  crates/runner/src/expression_validation.rs lines 401/404 branches 81/90  GAPS
  crates/runner/src/expression_validation_codec.rs lines 194/194 branches 24/24
  crates/runner/src/expression_validation_tests.rs lines 696/696 branches 6/6
  crates/runner/src/family_allocation_v1.rs lines 112/112 branches 16/16
  crates/runner/src/family_allocation_v1_tests.rs lines 71/71 branches 0/0
  crates/runner/src/grid.rs lines 5181/5227 branches 347/364  GAPS
  crates/runner/src/grid_cover_tests.rs lines 726/726 branches 4/6  GAPS
  crates/runner/src/identity.rs lines 655/655 branches 20/20
  crates/runner/src/lib.rs lines 956/957 branches 65/66  GAPS
  crates/runner/src/outcome.rs lines 2321/2335 branches 267/304  GAPS
  crates/runner/src/pbo.rs lines 265/265 branches 34/34
  crates/runner/src/portfolio.rs lines 750/750 branches 60/62  GAPS
  crates/runner/src/rank.rs lines 707/709 branches 14/14  GAPS
  crates/runner/src/replay_mask.rs lines 136/136 branches 14/14
  crates/runner/src/report.rs lines 996/996 branches 62/66  GAPS
  crates/runner/src/report_cover_tests.rs lines 194/194 branches 8/12  GAPS
  crates/runner/src/resample.rs lines 417/417 branches 22/26  GAPS
  crates/runner/src/research_admission_projection.rs lines 139/139 branches 2/2
  crates/runner/src/research_admission_projection_codec.rs lines 27/27 branches 6/6
  crates/runner/src/research_exit_grid.rs lines 149/149 branches 6/6
  crates/runner/src/research_exit_grid_tests.rs lines 44/44 branches 0/0
  crates/runner/src/research_family.rs lines 163/163 branches 14/14
  crates/runner/src/resolved_grid_view.rs lines 337/352 branches 74/74  GAPS
  crates/runner/src/search_allocation_v1.rs lines 101/101 branches 12/12
  crates/runner/src/search_allocation_v1_tests.rs lines 124/124 branches 0/0
  crates/runner/src/signal_candle_stop.rs lines 1198/1202 branches 236/248  GAPS
  crates/runner/src/signal_candle_stop_tests.rs lines 1185/1186 branches 30/32  GAPS
  crates/runner/src/significance.rs lines 507/510 branches 72/76  GAPS
  crates/runner/src/significance_cover_tests.rs lines 42/42 branches 2/4  GAPS
  crates/runner/src/split.rs lines 235/236 branches 36/38  GAPS
  crates/runner/src/synthetic.rs lines 149/149 branches 8/14  GAPS
  crates/runner/src/topn.rs lines 820/820 branches 69/74  GAPS
  crates/runner/src/trade.rs lines 1808/1825 branches 184/222  GAPS
  crates/runner/src/trade_clock_tests.rs lines 137/137 branches 2/4  GAPS
  crates/runner/src/validate.rs lines 4292/4332 branches 511/570  GAPS
  crates/runner/src/validate_cover_tests.rs lines 1555/1560 branches 23/34  GAPS
  crates/runner/tests/auto_probe_found_nothing.rs lines 71/71 branches 0/0
  crates/runner/tests/auto_probe_reporter_refusal.rs lines 47/47 branches 8/8
  crates/runner/tests/auto_reporting.rs lines 77/77 branches 0/0
  crates/runner/tests/base_cov_admission_words.rs lines 115/115 branches 7/8  GAPS
  crates/runner/tests/checkpointed_rank_refusals.rs lines 43/43 branches 0/0
  crates/runner/tests/d_0694_record.rs lines 166/166 branches 10/10
  crates/runner/tests/end_to_end.rs lines 240/240 branches 2/4  GAPS
  crates/runner/tests/expression_pricing.rs lines 158/158 branches 0/0
  crates/runner/tests/fib_rung_sweep.rs lines 119/119 branches 10/10
  crates/runner/tests/fixed_deadline_readiness.rs lines 211/211 branches 30/54  GAPS
  crates/runner/tests/foreign_slice_deadline_readiness.rs lines 267/267 branches 12/22  GAPS
  crates/runner/tests/hole_after_exit.rs lines 108/109 branches 4/4  GAPS
  crates/runner/tests/join_answer_is_unchanged.rs lines 66/67 branches 0/0  GAPS
  crates/runner/tests/population_door_halt.rs lines 33/33 branches 0/0
  crates/runner/tests/prefix_only_cadence.rs lines 124/124 branches 4/4
  crates/runner/tests/research_family_readiness.rs lines 445/445 branches 2/2
  crates/runner/tests/unwired_validation_record.rs lines 128/129 branches 12/12  GAPS

EVERY UNCOVERED LINE AND BRANCH SIDE THAT IS NOT (F), TAGGED
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3483  [production]  ExitGridErrorV1::BufferAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3484  [production]  buffer: name,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3485  [production]  elements: bars.len(),
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3486  [production]  }
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3487  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3698  [production]  elements: reserve,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3699  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3748  [production]  elements: length,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3749  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3386  [production]  requested_paths,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3387  [production]  },
[FAULT] GAP LINE   crates/runner/src/grid.rs:3388  [production]  )?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3439  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/grid.rs:3440  [production]  requested_paths,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3441  [production]  }
[FAULT] GAP LINE   crates/runner/src/grid.rs:3442  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3445  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/grid.rs:3446  [production]  requested_paths: 1,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3447  [production]  }
[FAULT] GAP LINE   crates/runner/src/grid.rs:3448  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:126  [production]  ExitGridErrorV1::BufferAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:127  [production]  buffer: "exact expression ladder",
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:128  [production]  elements: values.len(),
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:129  [production]  }
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:130  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:222  [production]  )?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:225  [production]  )?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:228  [production]  )?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:231  [production]  ExitGridErrorV1::BufferAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:232  [production]  buffer: "coordinate row offsets",
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:233  [production]  elements: row_count,
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:234  [production]  }
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:235  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:253  [production]  )?)
[FAULT] GAP LINE   crates/runner/src/resolved_grid_view.rs:276  [production]  )?;
[FAULT] GAP LINE   crates/runner/src/signal_candle_stop.rs:939  [production]  )?;
[HASH] GAP LINE   crates/runner/src/validate.rs:3071  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
[HASH] GAP LINE   crates/runner/src/validate.rs:3072  [production]  "Long/Short policy or resolution identities are indistinguishable",
[HASH] GAP LINE   crates/runner/src/validate.rs:3073  [production]  ));
[HASH] GAP BRANCH crates/runner/src/validate.rs:3070:8 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
[HASH] GAP BRANCH crates/runner/src/validate.rs:3070:57 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
[U] GAP BRANCH crates/runner/src/admission.rs:2194:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:5056:36 True  [test code]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:5228:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:5771:45 True  [test code]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:6121:45 True  [production]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
[U] GAP LINE   crates/runner/src/bootstrap.rs:766  [production]  )?,
[U] GAP LINE   crates/runner/src/bootstrap.rs:803  [production]  return None;
[U] GAP LINE   crates/runner/src/bootstrap.rs:851  [production]  )?,
[U] GAP LINE   crates/runner/src/bootstrap.rs:873  [production]  return None;
[U] GAP LINE   crates/runner/src/bootstrap.rs:893  [production]  return None;
[U] GAP LINE   crates/runner/src/bootstrap.rs:901  [production]  return None;
[U] GAP LINE   crates/runner/src/bootstrap.rs:1348  [production]  return None;
[U] GAP LINE   crates/runner/src/bootstrap.rs:1362  [production]  _ => left.cmp(right),
[U] GAP LINE   crates/runner/src/bootstrap.rs:1547  [production]  _ => left.cmp(right),
[U] GAP LINE   crates/runner/src/bootstrap.rs:1566  [production]  continue;
[U] GAP LINE   crates/runner/src/bootstrap.rs:1569  [production]  continue;
[U] GAP LINE   crates/runner/src/bootstrap.rs:1601  [production]  break;
[U] GAP LINE   crates/runner/src/bootstrap.rs:1622  [production]  }
[U] GAP BRANCH crates/runner/src/bootstrap.rs:628:17 False  [production]  let Some(own) = stats.get(s) else { continue };
[U] GAP BRANCH crates/runner/src/bootstrap.rs:727:8 True  [production]  if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:802:8 True  [production]  if !observed.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:868:8 True  [production]  if stats.iter().any(|summary| {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:869:9 True  [production]  !summary.mean.is_finite()
[U] GAP BRANCH crates/runner/src/bootstrap.rs:870:16 True  [production]  || !summary.standard_error.is_finite()
[U] GAP BRANCH crates/runner/src/bootstrap.rs:892:8 True  [production]  if matched_or_exceeded > draws || !observed.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:892:39 True  [production]  if matched_or_exceeded > draws || !observed.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:900:8 True  [production]  if !p_value.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1347:8 True  [production]  if observed.iter().any(|value| !value.is_finite()) {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1565:13 False  [production]  let Some(&strategy) = order.get(rank) else {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1568:13 False  [production]  let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1588:16 False  [production]  if let Some(slot) = bars.get_mut(rank) {
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1599:17 False  [production]  let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))
[U] GAP BRANCH crates/runner/src/bootstrap.rs:1620:16 False  [production]  if let Some(this_round) = out.get_mut(first..) {
[U] GAP BRANCH crates/runner/src/bootstrap_family_pass.rs:461:20 True  [production]  || !stat.standard_error.is_finite()
[U] GAP LINE   crates/runner/src/bootstrap_zero_v2.rs:192  [production]  return Err(Refusal::Numerical);
[U] GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:187:45 True  [production]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:191:12 True  [production]  if !observed.is_finite() {
[U] GAP LINE   crates/runner/src/excursion.rs:1405  [production]  break;
[U] GAP BRANCH crates/runner/src/excursion.rs:234:24 False  [production]  if let Some(prefix) = observed.get_mut(..bound) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1221:20 False  [production]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1232:20 False  [production]  if let Some(slot) = out.favourable.get_mut(target_cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1253:20 False  [production]  if let Some(slot) = armed_from.get_mut(arm) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1397:13 False  [production]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
[U] GAP BRANCH crates/runner/src/excursion.rs:1472:16 False  [production]  if let Some(slot) = crossed_at.get_mut(*cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1479:16 False  [production]  if let Some(slot) = peak.get_mut(*cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1482:16 False  [production]  if let Some(slot) = peak_raised.get_mut(*cursor) {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:615  [production]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1133  [production; its proof names line 1132, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1134  [production; its proof names line 1133, as at 2b998e4b5f6e]  needed: pair_count,
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1135  [production; its proof names line 1134, as at 2b998e4b5f6e]  max: self.ratios.max_pairs(),
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1136  [production; its proof names line 1135, as at 2b998e4b5f6e]  });
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1173  [test code; its proof names line 1172, as at 2b998e4b5f6e]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2133  [production; its proof names line 2132, as at 2b998e4b5f6e]  return false;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2458  [test code; its proof names line 2457, as at 2b998e4b5f6e]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2677  [production; its proof names line 2676, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2702  [production; its proof names line 2701, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::SelectionDigestMismatch);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2877  [production; its proof names line 2876, as at 2b998e4b5f6e]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3414  [production; its proof names line 3413, as at 2b998e4b5f6e]  return true;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3417  [production; its proof names line 3416, as at 2b998e4b5f6e]  return true;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3669  [production; its proof names line 3668, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4157  [production]  return Err(ExitGridErrorV1::ColumnShapeMismatch {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4158  [production]  bits: column.bits().len(),
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4159  [production]  sources: column.sources().len(),
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4160  [production]  });
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4174  [production]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4175  [production]  previous: previous.unwrap_or(source),
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4176  [production]  current: source,
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:4177  [production]  });
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:605:12 False  [production]  && core::ptr::eq(window, evaluated_execution_1m)
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [production]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:1132:12 True  [production; its proof names line 1131, as at 2b998e4b5f6e]  if pair_count > self.ratios.max_pairs() {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2132:25 False  [production; its proof names line 2131, as at 2b998e4b5f6e]  let Some(next) = pricing_refused.checked_add(1) else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2676:12 True  [production; its proof names line 2675, as at 2b998e4b5f6e]  if !self.chosen_is_in_bounds(coordinate) {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2680:12 True  [production; its proof names line 2679, as at 2b998e4b5f6e]  if selected.resolution_digest != self.digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2681:16 True  [production; its proof names line 2680, as at 2b998e4b5f6e]  || selected.run_id != evaluated.run_id
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2682:16 True  [production; its proof names line 2681, as at 2b998e4b5f6e]  || selected.mask != evaluated.mask
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2683:16 True  [production; its proof names line 2682, as at 2b998e4b5f6e]  || selected.horizon != evaluated.horizon
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2684:16 True  [production; its proof names line 2683, as at 2b998e4b5f6e]  || selected.column_digest != evaluated.column_digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2685:16 True  [production; its proof names line 2684, as at 2b998e4b5f6e]  || selected.evaluation_spec != evaluated.evaluation_spec
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2686:16 True  [production; its proof names line 2685, as at 2b998e4b5f6e]  || selected.side != evaluated.side
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2687:16 True  [production; its proof names line 2686, as at 2b998e4b5f6e]  || selected.coordinate != coordinate
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2688:16 True  [production; its proof names line 2687, as at 2b998e4b5f6e]  || selected.training_cell != *cell
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2689:16 True  [production; its proof names line 2688, as at 2b998e4b5f6e]  || selected.selection_digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3413:13 False  [production; its proof names line 3412, as at 2b998e4b5f6e]  let Some(left) = pair.first() else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3416:13 False  [production; its proof names line 3415, as at 2b998e4b5f6e]  let Some(right) = pair.last() else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3591:12 True  [production; its proof names line 3590, as at 2b998e4b5f6e]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3591:45 True  [production; its proof names line 3590, as at 2b998e4b5f6e]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3668:8 True  [production; its proof names line 3667, as at 2b998e4b5f6e]  if out.is_empty() {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4139:12 True  [production]  || !census.reconciles()
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4140:12 True  [production]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4141:12 True  [production]  || census.refused() != 0
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4156:8 True  [production]  if column.bits().len() != column.sources().len() {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4173:12 True  [production]  if previous.is_some_and(|before| source <= before) {
[U] GAP LINE   crates/runner/src/expression_execution.rs:358  [production]  return Err("expression exact truth summary differs from grid signals".to_owned());
[U] GAP BRANCH crates/runner/src/expression_execution.rs:357:12 True  [production]  if summary.hits != grid.signals {
[U] GAP LINE   crates/runner/src/expression_oos.rs:299  [production]  )?;
[U] GAP LINE   crates/runner/src/expression_oos.rs:303  [production]  return Err("later expression truth/grid signals differ".into());
[U] GAP BRANCH crates/runner/src/expression_oos.rs:302:12 True  [production]  if summary.hits != grid.signals {
[U] GAP LINE   crates/runner/src/expression_validation.rs:325  [production]  )?;
[U] GAP LINE   crates/runner/src/expression_validation.rs:349  [production]  return Err("fixed-training fold totals disagree with exact later cell".into());
[U] GAP LINE   crates/runner/src/expression_validation.rs:523  [production]  return Err("fixed-training trade crosses source/fold/session boundaries".into());
[U] GAP BRANCH crates/runner/src/expression_validation.rs:345:12 True  [production]  if totals.trades != cell.trades
[U] GAP BRANCH crates/runner/src/expression_validation.rs:346:16 True  [production]  || totals.wins != cell.wins
[U] GAP BRANCH crates/runner/src/expression_validation.rs:347:16 True  [production]  || totals.return_paisa != cell.pessimistic
[U] GAP BRANCH crates/runner/src/expression_validation.rs:516:12 True  [production]  if first != last
[U] GAP BRANCH crates/runner/src/expression_validation.rs:517:16 True  [production]  || trade.entry_micros != entry.ts_micros
[U] GAP BRANCH crates/runner/src/expression_validation.rs:518:16 True  [production]  || trade.exit_micros != exit.ts_micros
[U] GAP BRANCH crates/runner/src/expression_validation.rs:519:16 True  [production]  || day(entry.ts_micros) != day(exit.ts_micros)
[U] GAP BRANCH crates/runner/src/expression_validation.rs:520:16 True  [production]  || trade.exit_bar < trade.entry_bar
[U] GAP BRANCH crates/runner/src/expression_validation.rs:521:16 True  [production]  || previous.is_some_and(|index| trade.entry_bar <= index)
[U] GAP LINE   crates/runner/src/grid.rs:2095  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:2673  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:2700  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")
[U] GAP LINE   crates/runner/src/grid.rs:2701  [production]  })?;
[U] GAP LINE   crates/runner/src/grid.rs:2722  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")
[U] GAP LINE   crates/runner/src/grid.rs:2723  [production]  })?;
[U] GAP LINE   crates/runner/src/grid.rs:3088  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:3376  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3377  [production]  "candidate count does not fit the V1 receipt",
[U] GAP LINE   crates/runner/src/grid.rs:3378  [production]  )
[U] GAP LINE   crates/runner/src/grid.rs:3379  [production]  })?;
[U] GAP LINE   crates/runner/src/grid.rs:3433  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3434  [production]  "candidate count does not fit the V1 receipt",
[U] GAP LINE   crates/runner/src/grid.rs:3435  [production]  )
[U] GAP LINE   crates/runner/src/grid.rs:3436  [production]  })?;
[U] GAP LINE   crates/runner/src/grid.rs:3478  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:3621  [production]  return Err(
[U] GAP LINE   crates/runner/src/grid.rs:3622  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3623  [production]  "one candidate produced impossible multi-trade quality counts",
[U] GAP LINE   crates/runner/src/grid.rs:3624  [production]  ),
[U] GAP LINE   crates/runner/src/grid.rs:3625  [production]  );
[U] GAP LINE   crates/runner/src/grid.rs:3631  [production]  )?;
[U] GAP BRANCH crates/runner/src/grid.rs:1733:17 True  [production]  let at = if at > last { last } else { at };
[U] GAP BRANCH crates/runner/src/grid.rs:3613:32 True  [production]  if measured.trades != 1 || one_row.len() != 1 {
[U] GAP BRANCH crates/runner/src/grid.rs:3620:8 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
[U] GAP BRANCH crates/runner/src/grid.rs:3620:39 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
[U] GAP BRANCH crates/runner/src/grid.rs:3637:8 True  [production]  if row.signal_bar != candidate.signal
[U] GAP BRANCH crates/runner/src/grid.rs:3638:12 True  [production]  || row.entry_bar != candidate.entry
[U] GAP BRANCH crates/runner/src/grid.rs:3640:12 True  [production]  || row.exit_micros != stamp
[U] GAP BRANCH crates/runner/src/grid.rs:3641:12 True  [production]  || row.exit_bar < candidate.entry
[U] GAP BRANCH crates/runner/src/grid.rs:4748:12 False  [production]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {
[U] GAP LINE   crates/runner/src/outcome.rs:963  [production]  lanes.absent(Unpriced::Refused);
[U] GAP LINE   crates/runner/src/outcome.rs:964  [production]  continue;
[U] GAP LINE   crates/runner/src/outcome.rs:1055  [production]  lanes.absent(Unpriced::Refused);
[U] GAP LINE   crates/runner/src/outcome.rs:1056  [production]  continue;
[U] GAP BRANCH crates/runner/src/outcome.rs:812:19 False  [production]  while let Some(below) = levels.last() {
[U] GAP BRANCH crates/runner/src/outcome.rs:962:13 False  [production]  let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {
[U] GAP BRANCH crates/runner/src/outcome.rs:1050:13 False  [production]  let Some((later, now)) = bars
[U] GAP BRANCH crates/runner/src/outcome.rs:1170:28 False  [production]  if let Some(Reverse(moved)) = upper.pop() {
[U] GAP BRANCH crates/runner/src/outcome.rs:1174:28 False  [production]  && let Some(moved) = lower.pop()
[U] GAP BRANCH crates/runner/src/outcome.rs:1193:20 False  [production]  && let Some(slot) = out.get_mut(index)
[U] GAP BRANCH crates/runner/src/outcome.rs:2086:24 False  [production]  if let Some(freed) = self.slab.get_mut(at) {
[U] GAP BRANCH crates/runner/src/outcome.rs:2091:24 False  [production]  if let Some(staying) = self.slab.get_mut(at) {
[U] GAP BRANCH crates/runner/src/outcome.rs:2098:16 False  [production]  if let Some(head) = self.wheel.get_mut(slot) {
[U] GAP BRANCH crates/runner/src/outcome.rs:2116:16 False  [production]  if let Some(head) = self.wheel.get_mut(slot) {
[U] GAP BRANCH crates/runner/src/outcome.rs:2274:23 True  [production]  if !paired || !forward.covers(source) {
[U] GAP LINE   crates/runner/src/signal_candle_stop.rs:896  [production]  return Err(Error::Source);
[U] GAP LINE   crates/runner/src/signal_candle_stop.rs:1289  [production]  return Ok(Err(Reason::ClosingMinute));
[U] GAP LINE   crates/runner/src/signal_candle_stop.rs:1505  [production]  )?;
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:895:12 True  [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:895:51 True  [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1281:20 True  [production]  if !self
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1287:41 False  [production]  .is_some_and(|exit| exit.real && exit.bar == index)
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1345:12 True  [production]  || source.series.feed().is_empty()
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1346:12 True  [production]  || source.series.commit().is_empty()
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1347:12 True  [production]  || source.series.calendar_digest() == [0; 32]
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1349:12 True  [production]  || column.bits().len() != column.known().len()
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1350:12 True  [production]  || column.len() != column.sources().len()
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1352:12 True  [production]  || column.evaluation_spec_token().is_none()
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1358:12 True  [production]  || column
[U] GAP BRANCH crates/runner/src/signal_candle_stop.rs:1367:12 True  [production]  if !column.accepts(index)
[U] GAP BRANCH crates/runner/src/topn.rs:373:16 False  [production]  if let Some(slot) = bytes.get_mut(..16) {
[U] GAP BRANCH crates/runner/src/topn.rs:376:16 False  [production]  if let Some(slot) = bytes.get_mut(16..20) {
[U] GAP BRANCH crates/runner/src/topn.rs:379:16 False  [production]  if let Some(slot) = bytes.get_mut(20..24) {
[U] GAP BRANCH crates/runner/src/topn.rs:382:16 False  [production]  if let Some(slot) = bytes.get_mut(24..28) {
[U] GAP BRANCH crates/runner/src/topn.rs:389:20 False  [production]  if let Some(slot) = bytes.get_mut(start..end) {
[U] GAP LINE   crates/runner/src/trade.rs:680  [production]  return false;
[U] GAP LINE   crates/runner/src/trade.rs:683  [production]  return false;
[U] GAP LINE   crates/runner/src/trade.rs:812  [production]  return Err("expression pricing column alignment mismatch".to_owned());
[U] GAP LINE   crates/runner/src/trade.rs:1046  [production]  let blocked = open_until.is_some_and(|until| signal <= until);
[U] GAP LINE   crates/runner/src/trade.rs:1047  [production]  if blocked {
[U] GAP LINE   crates/runner/src/trade.rs:1048  [production]  out.while_open = out.while_open.saturating_add(1);
[U] GAP LINE   crates/runner/src/trade.rs:1049  [production]  } else {
[U] GAP LINE   crates/runner/src/trade.rs:1050  [production]  out.too_late = out.too_late.saturating_add(1);
[U] GAP LINE   crates/runner/src/trade.rs:1051  [production]  }
[U] GAP LINE   crates/runner/src/trade.rs:1052  [production]  continue;
[U] GAP LINE   crates/runner/src/trade.rs:1160  [production]  out.while_open = out.while_open.saturating_add(1);
[U] GAP LINE   crates/runner/src/trade.rs:1353  [production]  return None;
[U] GAP BRANCH crates/runner/src/trade.rs:679:13 False  [production]  let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {
[U] GAP BRANCH crates/runner/src/trade.rs:682:13 False  [production]  let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {
[U] GAP BRANCH crates/runner/src/trade.rs:729:16 False  [production]  if let [before, after] = pair
[U] GAP BRANCH crates/runner/src/trade.rs:810:8 True  [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
[U] GAP BRANCH crates/runner/src/trade.rs:810:55 True  [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
[U] GAP BRANCH crates/runner/src/trade.rs:1045:13 False  [production]  let Some(entry) = signal.checked_add(step) else {
[U] GAP BRANCH crates/runner/src/trade.rs:1047:16 True  [production]  if blocked {
[U] GAP BRANCH crates/runner/src/trade.rs:1047:16 False  [production]  if blocked {
[U] GAP BRANCH crates/runner/src/trade.rs:1159:16 True  [production]  if blocked {
[U] GAP BRANCH crates/runner/src/trade.rs:1352:8 True  [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
[U] GAP BRANCH crates/runner/src/trade.rs:1352:37 True  [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
[U] GAP LINE   crates/runner/src/validate.rs:2229  [production]  )?;
[U] GAP LINE   crates/runner/src/validate.rs:2286  [production]  )?;
[U] GAP LINE   crates/runner/src/validate.rs:2393  [production]  return Err(AnchoredSearchValidationRefusalV4::Empty);
[U] GAP LINE   crates/runner/src/validate.rs:2600  [production]  )?;
[U] GAP LINE   crates/runner/src/validate.rs:2746  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
[U] GAP LINE   crates/runner/src/validate.rs:2747  [production]  "evaluated cell population differs from its resolution",
[U] GAP LINE   crates/runner/src/validate.rs:2748  [production]  ));
[U] GAP LINE   crates/runner/src/validate.rs:2788  [production]  )?;
[U] GAP LINE   crates/runner/src/validate.rs:2809  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
[U] GAP LINE   crates/runner/src/validate.rs:2810  [production]  "mask-by-side population is incomplete",
[U] GAP LINE   crates/runner/src/validate.rs:2811  [production]  ));
[U] GAP LINE   crates/runner/src/validate.rs:2863  [production]  )?;
[U] GAP LINE   crates/runner/src/validate.rs:2952  [production]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
[U] GAP LINE   crates/runner/src/validate.rs:3104  [production]  overflowed = true;
[U] GAP LINE   crates/runner/src/validate.rs:3105  [production]  return false;
[U] GAP LINE   crates/runner/src/validate.rs:3111  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
[U] GAP LINE   crates/runner/src/validate.rs:3112  [production]  "canonical coordinate order is incomplete",
[U] GAP LINE   crates/runner/src/validate.rs:3113  [production]  ));
[U] GAP LINE   crates/runner/src/validate.rs:3605  [production]  return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());
[U] GAP LINE   crates/runner/src/validate.rs:3626  [production]  return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());
[U] GAP LINE   crates/runner/src/validate.rs:3772  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
[U] GAP LINE   crates/runner/src/validate.rs:4124  [production]  return false;
[U] GAP LINE   crates/runner/src/validate.rs:4516  [production]  continue;
[U] GAP LINE   crates/runner/src/validate.rs:4972  [production]  out.refused = Some(
[U] GAP LINE   crates/runner/src/validate.rs:4973  [production]  "a walk-forward fold selected a candidate without its training side"
[U] GAP LINE   crates/runner/src/validate.rs:4974  [production]  .to_owned(),
[U] GAP LINE   crates/runner/src/validate.rs:4975  [production]  );
[U] GAP LINE   crates/runner/src/validate.rs:4976  [production]  return out;
[U] GAP LINE   crates/runner/src/validate.rs:5092  [production]  0
[U] GAP BRANCH crates/runner/src/validate.rs:2392:8 True  [production]  if folds.is_empty() || folds.len() != validated.folds.len() {
[U] GAP BRANCH crates/runner/src/validate.rs:2392:28 True  [production]  if folds.is_empty() || folds.len() != validated.folds.len() {
[U] GAP BRANCH crates/runner/src/validate.rs:2458:12 True  [production]  || column.bits().len() != column.sources().len()
[U] GAP BRANCH crates/runner/src/validate.rs:2460:12 True  [production]  || census.swept != stable_u64_v4(column.bits().len())?
[U] GAP BRANCH crates/runner/src/validate.rs:2461:12 True  [production]  || !census.reconciles()
[U] GAP BRANCH crates/runner/src/validate.rs:2462:12 True  [production]  || !column.acceptance_covers(signal.len())
[U] GAP BRANCH crates/runner/src/validate.rs:2463:12 True  [production]  || column.acceptance_census() != census
[U] GAP BRANCH crates/runner/src/validate.rs:2464:12 True  [production]  || column.evaluation_spec_token().is_none()
[U] GAP BRANCH crates/runner/src/validate.rs:2465:12 True  [production]  || column.collided() != 0
[U] GAP BRANCH crates/runner/src/validate.rs:2466:12 True  [production]  || column.first_swept() != column.sources().first().copied()
[U] GAP BRANCH crates/runner/src/validate.rs:2467:12 True  [production]  || column
[U] GAP BRANCH crates/runner/src/validate.rs:2472:12 True  [production]  || !column
[U] GAP BRANCH crates/runner/src/validate.rs:2574:12 True  [production]  if fold.train.0.start != 0
[U] GAP BRANCH crates/runner/src/validate.rs:2575:16 True  [production]  || !fold.train.1.is_empty()
[U] GAP BRANCH crates/runner/src/validate.rs:2576:16 True  [production]  || fold.train.0.is_empty()
[U] GAP BRANCH crates/runner/src/validate.rs:2745:16 True  [production]  if evaluated_cells != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:2808:12 True  [production]  if population_cells != expected_population {
[U] GAP BRANCH crates/runner/src/validate.rs:2951:12 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
[U] GAP BRANCH crates/runner/src/validate.rs:2951:55 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
[U] GAP BRANCH crates/runner/src/validate.rs:3103:17 False  [production]  let Some(next) = ordinal.checked_add(1) else {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:12 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:26 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:39 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:3604:8 True  [production]  if alignment.onto.len() != signal_column.sources().len() {
[U] GAP BRANCH crates/runner/src/validate.rs:3625:8 True  [production]  if dropped != expected_dropped {
[U] GAP BRANCH crates/runner/src/validate.rs:3771:13 False  [production]  let Some(outcome) = oos.get(ordinal) else {
[U] GAP BRANCH crates/runner/src/validate.rs:4123:9 False  [production]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {
[U] GAP BRANCH crates/runner/src/validate.rs:4515:13 False  [production]  let Some(train) = bars.get(fold.train.0.clone()) else {
[U] GAP BRANCH crates/runner/src/validate.rs:4971:21 False  [production]  let Some(chosen_side) = chosen_side else {
[U] GAP BRANCH crates/runner/src/validate.rs:5091:24 True  [production]  if *mask == ConditionMask::ZERO {

OPERATOR ITEMS (the [FAULT] and [HASH] rows above, as written; no row is [UNPROVEN])
GAP-FAULT crates/runner/src/signal_candle_stop.rs:939 (`)?;` of `periods_before = days_before(..)` in Prepared::new). Only an allocator refusal reaches it: line 935 has just run days_before over the same source span and succeeded, so the span cannot overflow; the count is at most periods.len() and the day at most last_day + 1 (validated to be at most i64::MAX / DAY), so neither checked add fails; the remaining error is `try_reserve_exact(span).map_err(|_| Error::Allocation)`, a fault no input or call can produce on demand. The recipe allows no (U) for a fault, so this row stays a GAP for the operator's ruling (no test can drive it without a failing allocator, and the workspace denies unsafe_code, which a test allocator would need).
GAP-FAULT crates/runner/src/grid.rs:3386-3388, 3439-3442, 3445-3448 (the `ReplayEvidenceAllocationRefused` closures of `try_reserve_exact` in replay_universe_v1 and replay_universe_from_candidates_v1): the requested sizes are an occupancy table's length and 1, both already held in memory, so only an allocator refusal reaches them.
GAP-FAULT crates/runner/src/exit_grid_policy.rs:3483-3487 (the BufferAllocationRefused closure of `values.try_reserve_exact(bars.len())` for the observed stops, targets and trails), crates/runner/src/resolved_grid_view.rs:126-130 (`copy.try_reserve_exact(values.len())` of an exact expression ladder) and 231-235 (`offsets.try_reserve_exact(row_count)` of the coordinate row offsets): each reservation is sized from data already held in memory (the bars, a ladder, the (S + 1)(T + 1) row table), so only an allocator refusal reaches the closure; no input or call produces one on demand, and a test allocator would need unsafe code, which the workspace denies.
GAP-HASH crates/runner/src/validate.rs:3070:8 True, 3070:57 True, 3071-3073 (`long.policy_digest() == short.policy_digest() || long.digest() == short.digest()` in authenticate_grid_pair_v4): reached only after 3045 (long is a Long policy, short a Short one) and 3051 (each policy_digest equals its policy's digest and each digest equals digest_resolved of its fields). The policy digest hashes the side, and the resolution digest hashes the policy digest, so equal digests need two different preimages with one blake3 output. No input or tampering short of a hash collision reaches it; it is not a code-level (U), so it is listed for the operator's ruling with the fault rows.
GAP-FAULT crates/runner/src/resolved_grid_view.rs:222, 225, 228 (the checked_add/checked_mul `)?` of coordinate_row_offsets), 253 and 276 (the same in canonical_coordinate_ordinal): an overflow needs the stop and target ladders to hold about 2^32 rungs each, so that (S + 1)(T + 1) passes usize::MAX; each ladder is a Vec<Ppm> resolved from a rung plan of as many percentiles, i.e. tens of GiB per vector. Like the allocator rows, no test can build such an input; listed for the operator's ruling with them.
GAP-FAULT crates/runner/src/exit_grid_policy.rs:3698-3699, 3748-3749 (the BufferAllocationRefused closures of `out.try_reserve_exact(reserve)` in admitted_ratio_pairs and `bitmap.try_reserve_exact(length)` in ratio_bitmap): each size is at most the stop count times the target count of ladders already resolved and held in memory (the pairs reservation is also capped by the policy's max_pairs), so only an allocator refusal reaches the closure. A rung plan of tens of thousands of percentiles per axis asks for gigabytes, which one host's allocator grants and another's refuses depending on its memory and overcommit policy, so no portable test drives it; listed for the operator's ruling with the other allocator rows.
`````

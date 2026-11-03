# gaps-LS2.txt.md: runner coverage gaps by Route B

Shard BASE-COV-LIBS-LS2 (crate runner). FINAL: these lists are from the final Route B at code tip 2b998e4b (21:10-22:40 IST), which measures every code commit of this shard.
One fenced block below holds the list.

`````text
Route B (d3-route-b-cloud.sh, pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0-rust-1.99.0-nightly) on tip 2b998e4b5f6e4cc77ab8bd32de00ba8a4d165f0b, set reading whole:runner.
Log: /tmp/claude-0/-home-user-brutex/b3671350-4320-59ae-92c2-fdf03932a703/scratchpad/logs/2b998e4b5f6e-routeb.log (ROUTE-B DONE 2026-11-03 22:40:40 IST; tests passed 956 failed 0 ignored 0)
TOTAL runner: lines 50775/51110 (uncovered 335), branches 3711/4170 (uncovered 459), files read 73; NOT READ: crates/runner/benches/ratio.rs ([[bench]] test = false: no test object; L-BCL-1).
Rows: 794 uncovered lines and branch sides = 254 (F) (in fu-LS2.txt.md) + 540 others below.
Of the 540: [U] 224 (proof in fu-LS2.txt.md), [FAULT] 36 (allocator or address-space bound, operator ruling), [HASH] 5 (a blake3 collision only, operator ruling), [UNPROVEN] 24 (believed unreachable, no proof claimed), [open] 251 (68 production, 183 test code or hand-read; not closed in this session).

PER FILE (lines a/b, branches c/d)
  crates/runner/src/admission_reader_cover_tests.rs lines 445/445 branches 6/8  GAPS
  crates/runner/src/admission_statistics_cover_tests.rs lines 235/235 branches 0/0
  crates/runner/src/admission.rs lines 6247/6259 branches 305/310  GAPS
  crates/runner/src/align.rs lines 147/147 branches 8/8
  crates/runner/src/audit.rs lines 2118/2121 branches 146/170  GAPS
  crates/runner/src/bootstrap_family_pass_tests.rs lines 387/387 branches 7/10  GAPS
  crates/runner/src/bootstrap_family_pass.rs lines 269/269 branches 35/36  GAPS
  crates/runner/src/bootstrap_zero_v2_tests.rs lines 426/426 branches 20/20
  crates/runner/src/bootstrap_zero_v2.rs lines 323/324 branches 48/50  GAPS
  crates/runner/src/bootstrap.rs lines 1790/1814 branches 160/186  GAPS
  crates/runner/src/bound.rs lines 228/228 branches 14/14
  crates/runner/src/closed.rs lines 146/146 branches 10/10
  crates/runner/src/excursion_clock_tests.rs lines 89/89 branches 8/8
  crates/runner/src/excursion.rs lines 1130/1138 branches 110/120  GAPS
  crates/runner/src/exit_grid_policy_cover_tests.rs lines 90/90 branches 0/0
  crates/runner/src/exit_grid_policy.rs lines 5738/5777 branches 323/366  GAPS
  crates/runner/src/expression_execution_tests.rs lines 465/468 branches 1/2  GAPS
  crates/runner/src/expression_execution.rs lines 333/335 branches 29/30  GAPS
  crates/runner/src/expression_oos_tests.rs lines 400/400 branches 6/6
  crates/runner/src/expression_oos.rs lines 250/252 branches 19/20  GAPS
  crates/runner/src/expression_validation_codec.rs lines 194/194 branches 24/24
  crates/runner/src/expression_validation_tests.rs lines 672/677 branches 9/12  GAPS
  crates/runner/src/expression_validation.rs lines 401/404 branches 81/90  GAPS
  crates/runner/src/expression.rs lines 212/212 branches 6/6
  crates/runner/src/family_allocation_v1_tests.rs lines 71/71 branches 0/0
  crates/runner/src/family_allocation_v1.rs lines 112/112 branches 16/16
  crates/runner/src/grid_cover_tests.rs lines 442/442 branches 2/4  GAPS
  crates/runner/src/grid.rs lines 5091/5177 branches 343/382  GAPS
  crates/runner/src/identity.rs lines 657/657 branches 21/22  GAPS
  crates/runner/src/lib.rs lines 956/957 branches 64/66  GAPS
  crates/runner/src/outcome.rs lines 2320/2334 branches 271/314  GAPS
  crates/runner/src/pbo.rs lines 265/265 branches 34/34
  crates/runner/src/portfolio.rs lines 750/750 branches 61/64  GAPS
  crates/runner/src/rank.rs lines 707/709 branches 14/14  GAPS
  crates/runner/src/replay_mask.rs lines 136/136 branches 14/14
  crates/runner/src/report_cover_tests.rs lines 194/194 branches 8/12  GAPS
  crates/runner/src/report.rs lines 996/996 branches 62/66  GAPS
  crates/runner/src/resample.rs lines 414/414 branches 23/28  GAPS
  crates/runner/src/research_admission_projection_codec.rs lines 27/27 branches 6/6
  crates/runner/src/research_admission_projection.rs lines 139/139 branches 2/2
  crates/runner/src/research_exit_grid_tests.rs lines 44/44 branches 0/0
  crates/runner/src/research_exit_grid.rs lines 149/149 branches 6/6
  crates/runner/src/research_family.rs lines 163/163 branches 14/14
  crates/runner/src/resolved_grid_view.rs lines 337/352 branches 74/74  GAPS
  crates/runner/src/search_allocation_v1_tests.rs lines 124/124 branches 0/0
  crates/runner/src/search_allocation_v1.rs lines 101/101 branches 12/12
  crates/runner/src/signal_candle_stop_tests.rs lines 1185/1186 branches 31/34  GAPS
  crates/runner/src/signal_candle_stop.rs lines 1198/1202 branches 236/248  GAPS
  crates/runner/src/significance_cover_tests.rs lines 42/42 branches 2/4  GAPS
  crates/runner/src/significance.rs lines 507/510 branches 72/76  GAPS
  crates/runner/src/split.rs lines 234/235 branches 37/42  GAPS
  crates/runner/src/synthetic.rs lines 149/149 branches 9/16  GAPS
  crates/runner/src/topn.rs lines 820/820 branches 69/74  GAPS
  crates/runner/src/trade_clock_tests.rs lines 137/137 branches 2/4  GAPS
  crates/runner/src/trade.rs lines 1808/1829 branches 195/244  GAPS
  crates/runner/src/validate_cover_tests.rs lines 1234/1240 branches 10/12  GAPS
  crates/runner/src/validate.rs lines 4259/4329 branches 513/592  GAPS
  crates/runner/tests/auto_probe_reporter_refusal.rs lines 47/47 branches 8/8
  crates/runner/tests/auto_reporting.rs lines 77/77 branches 0/0
  crates/runner/tests/base_cov_admission_words.rs lines 115/115 branches 7/8  GAPS
  crates/runner/tests/checkpointed_rank_refusals.rs lines 43/43 branches 0/0
  crates/runner/tests/d_0694_record.rs lines 143/143 branches 9/10  GAPS
  crates/runner/tests/end_to_end.rs lines 226/227 branches 4/10  GAPS
  crates/runner/tests/expression_pricing.rs lines 157/158 branches 9/14  GAPS
  crates/runner/tests/fib_rung_sweep.rs lines 119/119 branches 10/10
  crates/runner/tests/fixed_deadline_readiness.rs lines 211/211 branches 30/54  GAPS
  crates/runner/tests/foreign_slice_deadline_readiness.rs lines 264/266 branches 13/28  GAPS
  crates/runner/tests/hole_after_exit.rs lines 105/106 branches 7/8  GAPS
  crates/runner/tests/join_answer_is_unchanged.rs lines 66/67 branches 0/0  GAPS
  crates/runner/tests/population_door_halt.rs lines 33/33 branches 0/0
  crates/runner/tests/prefix_only_cadence.rs lines 124/124 branches 4/4
  crates/runner/tests/research_family_readiness.rs lines 445/446 branches 2/2  GAPS
  crates/runner/tests/unwired_validation_record.rs lines 97/99 branches 10/12  GAPS

EVERY UNCOVERED LINE AND BRANCH SIDE THAT IS NOT (F), TAGGED
[open] GAP LINE   crates/runner/src/admission.rs:7110  [test code]  AdmissionReasonV1::Trades => values.trades = ObservedU64V1::Refused,
[open] GAP LINE   crates/runner/src/admission.rs:7160  [test code]  AdmissionReasonV1::LosingTrades => {
[open] GAP LINE   crates/runner/src/admission.rs:7161  [test code]  values.losing_trades = ObservedU64V1::Refused;
[open] GAP LINE   crates/runner/src/admission.rs:7162  [test code]  }
[open] GAP LINE   crates/runner/src/admission.rs:7166  [test code]  AdmissionReasonV1::WinningTrades => {
[open] GAP LINE   crates/runner/src/admission.rs:7167  [test code]  values.winning_trades = ObservedU64V1::Refused;
[open] GAP LINE   crates/runner/src/admission.rs:7168  [test code]  }
[U] GAP BRANCH crates/runner/src/admission.rs:2194:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[open] GAP BRANCH crates/runner/src/admission.rs:5056:36 True  [test code branch]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:5228:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:5771:45 True  [test code branch]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
[U] GAP BRANCH crates/runner/src/admission.rs:6121:45 True  [production]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
[open] GAP LINE   crates/runner/src/audit.rs:2644  [test code]  || (what != "the sentence"
[open] GAP LINE   crates/runner/src/audit.rs:2645  [test code]  && token == "V6"
[open] GAP LINE   crates/runner/src/audit.rs:2646  [test code]  && text.contains("Selection V6")),
[open] GAP? BRANCH crates/runner/src/audit.rs:2358:44 False  [assert! condition branch, side False (inside a closure)]  line.is_empty() || line.starts_with("  "),
[open] GAP BRANCH crates/runner/src/audit.rs:2465:34 False  [test code branch]  .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()));
[open] GAP? BRANCH crates/runner/src/audit.rs:2467:29 False  [assert! condition branch, side False (inside a closure)]  decision || token == "V6",
[open] GAP? BRANCH crates/runner/src/audit.rs:2620:40 False  [assert! condition branch, side False (inside a closure)]  line.is_empty() || line.starts_with("  "),
[open] GAP? BRANCH crates/runner/src/audit.rs:2643:42 False  [assert! condition branch, side False (inside a closure)]  .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
[open] GAP? BRANCH crates/runner/src/audit.rs:2644:29 True  [assert! condition branch, side True (inside a closure)]  || (what != "the sentence"
[open] GAP? BRANCH crates/runner/src/audit.rs:2644:29 False  [assert! condition branch, side False (inside a closure)]  || (what != "the sentence"
[open] GAP? BRANCH crates/runner/src/audit.rs:2645:32 True  [assert! condition branch, side True (inside a closure)]  && token == "V6"
[open] GAP? BRANCH crates/runner/src/audit.rs:2645:32 False  [assert! condition branch, side False (inside a closure)]  && token == "V6"
[open] GAP? BRANCH crates/runner/src/audit.rs:2646:32 True  [assert! condition branch, side True (inside a closure)]  && text.contains("Selection V6")),
[open] GAP? BRANCH crates/runner/src/audit.rs:2646:32 False  [assert! condition branch, side False (inside a closure)]  && text.contains("Selection V6")),
[open] GAP? BRANCH crates/runner/src/audit.rs:2974:13 True  [assert! condition branch, side True]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
[open] GAP? BRANCH crates/runner/src/audit.rs:2974:62 False  [assert! condition branch, side False (inside a closure)]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
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
[open] GAP LINE   crates/runner/src/bootstrap.rs:3641  [test code]  {
[open] GAP LINE   crates/runner/src/bootstrap.rs:3642  [test code]  disagreements.push((seed, shift, alpha, strategy));
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
[open] GAP? BRANCH crates/runner/src/bootstrap.rs:3401:46 True  [assert! condition branch, side True (inside a closure)]  r.round == expected_round || r.round == expected_round + 1,
[open] GAP? BRANCH crates/runner/src/bootstrap.rs:3401:46 False  [assert! condition branch, side False (inside a closure)]  r.round == expected_round || r.round == expected_round + 1,
[open] GAP BRANCH crates/runner/src/bootstrap.rs:3639:28 True  [test code branch]  if legacy.iter().any(|row| row.strategy == strategy) != exact
[open] GAP BRANCH crates/runner/src/bootstrap.rs:3640:32 True  [test code branch]  || receipt.is_rejected(strategy) != Some(exact)
[U] GAP BRANCH crates/runner/src/bootstrap_family_pass.rs:461:20 True  [production]  || !stat.standard_error.is_finite()
[open] GAP? BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:194:13 False  [assert! condition branch, side False (inside a closure)]  strict > 0 && strict < draws
[U] GAP LINE   crates/runner/src/bootstrap_zero_v2.rs:192  [production]  return Err(Refusal::Numerical);
[U] GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:187:45 True  [production]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
[U] GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:191:12 True  [production]  if !observed.is_finite() {
[U] GAP LINE   crates/runner/src/excursion.rs:1405  [production]  break;
[open] GAP LINE   crates/runner/src/excursion.rs:1664  [test code]  _ => true,
[open] GAP LINE   crates/runner/src/excursion.rs:2534  [test code]  return None;
[U] GAP BRANCH crates/runner/src/excursion.rs:234:24 False  [production]  if let Some(prefix) = observed.get_mut(..bound) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1221:20 False  [production]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1232:20 False  [production]  if let Some(slot) = out.favourable.get_mut(target_cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1253:20 False  [production]  if let Some(slot) = armed_from.get_mut(arm) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1397:13 False  [production]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
[U] GAP BRANCH crates/runner/src/excursion.rs:1472:16 False  [production]  if let Some(slot) = crossed_at.get_mut(*cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1479:16 False  [production]  if let Some(slot) = peak.get_mut(*cursor) {
[U] GAP BRANCH crates/runner/src/excursion.rs:1482:16 False  [production]  if let Some(slot) = peak_raised.get_mut(*cursor) {
[open] GAP BRANCH crates/runner/src/excursion.rs:2533:12 True  [test code branch]  if observed.is_empty() || count == 0 {
[open] GAP BRANCH crates/runner/src/excursion.rs:2533:35 True  [test code branch]  if observed.is_empty() || count == 0 {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:615  [production]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1132  [production]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1133  [production]  needed: pair_count,
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1134  [production]  max: self.ratios.max_pairs(),
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1135  [production]  });
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:1172  [test code]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2132  [production]  return false;
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:2398  [production]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2457  [test code]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2676  [production]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2701  [production]  return Err(ExitGridErrorV1::SelectionDigestMismatch);
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:2876  [production]  )?;
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:2924  [production]  )?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3413  [production]  return true;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3416  [production]  return true;
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3482  [production]  ExitGridErrorV1::BufferAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3483  [production]  buffer: name,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3484  [production]  elements: bars.len(),
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3485  [production]  }
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3486  [production]  })?;
[U] GAP LINE   crates/runner/src/exit_grid_policy.rs:3668  [production]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3697  [production]  elements: reserve,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3698  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3747  [production]  elements: length,
[FAULT] GAP LINE   crates/runner/src/exit_grid_policy.rs:3748  [production]  })?;
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:3789  [production]  )?)
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4156  [production]  return Err(ExitGridErrorV1::ColumnShapeMismatch {
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4157  [production]  bits: column.bits().len(),
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4158  [production]  sources: column.sources().len(),
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4159  [production]  });
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4173  [production]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4174  [production]  previous: previous.unwrap_or(source),
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4175  [production]  current: source,
[UNPROVEN] GAP LINE   crates/runner/src/exit_grid_policy.rs:4176  [production]  });
[open] GAP LINE   crates/runner/src/exit_grid_policy.rs:7230  [test code]  )?;
[open] GAP LINE   crates/runner/src/exit_grid_policy.rs:7241  [test code]  )?,
[open] GAP LINE   crates/runner/src/exit_grid_policy.rs:7645  [test code]  }
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:605:12 False  [production]  && core::ptr::eq(window, evaluated_execution_1m)
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [production]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:676:16 False  [test code branch]  if let Some(last) = forged.last_mut() {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:1131:12 True  [production]  if pair_count > self.ratios.max_pairs() {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2131:25 False  [production]  let Some(next) = pricing_refused.checked_add(1) else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2675:12 True  [production]  if !self.chosen_is_in_bounds(coordinate) {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2679:12 True  [production]  if selected.resolution_digest != self.digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2680:16 True  [production]  || selected.run_id != evaluated.run_id
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2681:16 True  [production]  || selected.mask != evaluated.mask
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2682:16 True  [production]  || selected.horizon != evaluated.horizon
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2683:16 True  [production]  || selected.column_digest != evaluated.column_digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2684:16 True  [production]  || selected.evaluation_spec != evaluated.evaluation_spec
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2685:16 True  [production]  || selected.side != evaluated.side
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2686:16 True  [production]  || selected.coordinate != coordinate
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2687:16 True  [production]  || selected.training_cell != *cell
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:2688:16 True  [production]  || selected.selection_digest
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3412:13 False  [production]  let Some(left) = pair.first() else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3415:13 False  [production]  let Some(right) = pair.last() else {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3590:12 True  [production]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3590:45 True  [production]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
[U] GAP BRANCH crates/runner/src/exit_grid_policy.rs:3667:8 True  [production]  if out.is_empty() {
[UNPROVEN] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4138:12 True  [production]  || !census.reconciles()
[UNPROVEN] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4139:12 True  [production]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
[UNPROVEN] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4140:12 True  [production]  || census.refused() != 0
[UNPROVEN] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4155:8 True  [production]  if column.bits().len() != column.sources().len() {
[UNPROVEN] GAP BRANCH crates/runner/src/exit_grid_policy.rs:4172:12 True  [production]  if previous.is_some_and(|before| source <= before) {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:5863:16 False  [test code branch]  if let Some(bar) = gapped.get_mut(3) {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6197:49 False  [test code branch]  .find(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6204:26 False  [test code branch]  .find(|cell| cell.stop.is_some() && cell.target.is_none() && cell.ttp.is_none())
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6211:28 False  [test code branch]  .filter(|cell| cell.stop.is_some() && cell.target.is_some() && cell.ttp.is_none())
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6211:51 False  [test code branch]  .filter(|cell| cell.stop.is_some() && cell.target.is_some() && cell.ttp.is_none())
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6345:51 False  [test code branch]  .filter(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:6426:24 False  [test code branch]  && cell.target.is_some()
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:7596:16 False  [test code branch]  if let Some(slot) = cells.get_mut(1) {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:7640:16 False  [test code branch]  if let Some(rejected_coordinate) = rejected_coordinate {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:7661:16 False  [test code branch]  if let (Some(first), Some(second)) = (first_coordinate, duplicated.grid.cells.get_mut(1)) {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:7679:16 False  [test code branch]  if let Some(Some(offset)) = wrong_offset.coordinate_row_offsets.first_mut() {
[open] GAP BRANCH crates/runner/src/exit_grid_policy.rs:7820:13 False  [test code branch]  let crate::grid::ReplayPathV1::Priceable(price) = &mut candidate.path else {
[UNPROVEN] GAP LINE   crates/runner/src/expression_execution.rs:336  [production]  )?;
[U] GAP LINE   crates/runner/src/expression_execution.rs:358  [production]  return Err("expression exact truth summary differs from grid signals".to_owned());
[U] GAP BRANCH crates/runner/src/expression_execution.rs:357:12 True  [production]  if summary.hits != grid.signals {
[open] GAP LINE   crates/runner/src/expression_execution_tests.rs:218  [test code]  return Err("pair".to_owned());
[open] GAP LINE   crates/runner/src/expression_execution_tests.rs:361  [test code]  )?;
[open] GAP LINE   crates/runner/src/expression_execution_tests.rs:410  [test code]  )?;
[open] GAP BRANCH crates/runner/src/expression_execution_tests.rs:217:13 False  [test code branch]  let [left, right] = pair else {
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
[open] GAP LINE   crates/runner/src/expression_validation_tests.rs:141  [test code]  }
[open] GAP LINE   crates/runner/src/expression_validation_tests.rs:425  [test code]  }
[open] GAP LINE   crates/runner/src/expression_validation_tests.rs:689  [test code]  )?;
[open] GAP LINE   crates/runner/src/expression_validation_tests.rs:698  [test code]  )?;
[open] GAP LINE   crates/runner/src/expression_validation_tests.rs:723  [test code]  }
[open] GAP BRANCH crates/runner/src/expression_validation_tests.rs:138:16 False  [test code branch]  if let Some(selected) = disposition.selected() {
[open] GAP BRANCH crates/runner/src/expression_validation_tests.rs:404:20 False  [test code branch]  if let Some((selected, wrong)) = disposition.selected().zip(foreign.selected()) {
[open] GAP BRANCH crates/runner/src/expression_validation_tests.rs:720:16 False  [test code branch]  if let Some(selected) = disposition.selected() {
[open] GAP LINE   crates/runner/src/grid.rs:2095  [production]  )?;
[open] GAP LINE   crates/runner/src/grid.rs:2384  [production]  return envelope_refused(timed);
[U] GAP LINE   crates/runner/src/grid.rs:2673  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:2700  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")
[U] GAP LINE   crates/runner/src/grid.rs:2701  [production]  })?;
[open] GAP LINE   crates/runner/src/grid.rs:2703  [production]  return Err(
[open] GAP LINE   crates/runner/src/grid.rs:2704  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayCellCountMismatch {
[open] GAP LINE   crates/runner/src/grid.rs:2705  [production]  expected: resolved.cell_count(),
[open] GAP LINE   crates/runner/src/grid.rs:2706  [production]  actual,
[open] GAP LINE   crates/runner/src/grid.rs:2707  [production]  },
[open] GAP LINE   crates/runner/src/grid.rs:2708  [production]  );
[U] GAP LINE   crates/runner/src/grid.rs:2722  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")
[U] GAP LINE   crates/runner/src/grid.rs:2723  [production]  })?;
[open] GAP LINE   crates/runner/src/grid.rs:3088  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:3376  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3377  [production]  "candidate count does not fit the V1 receipt",
[U] GAP LINE   crates/runner/src/grid.rs:3378  [production]  )
[U] GAP LINE   crates/runner/src/grid.rs:3379  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3386  [production]  requested_paths,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3387  [production]  },
[FAULT] GAP LINE   crates/runner/src/grid.rs:3388  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:3433  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3434  [production]  "candidate count does not fit the V1 receipt",
[U] GAP LINE   crates/runner/src/grid.rs:3435  [production]  )
[U] GAP LINE   crates/runner/src/grid.rs:3436  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3439  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/grid.rs:3440  [production]  requested_paths,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3441  [production]  }
[FAULT] GAP LINE   crates/runner/src/grid.rs:3442  [production]  })?;
[FAULT] GAP LINE   crates/runner/src/grid.rs:3445  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
[FAULT] GAP LINE   crates/runner/src/grid.rs:3446  [production]  requested_paths: 1,
[FAULT] GAP LINE   crates/runner/src/grid.rs:3447  [production]  }
[FAULT] GAP LINE   crates/runner/src/grid.rs:3448  [production]  })?;
[U] GAP LINE   crates/runner/src/grid.rs:3478  [production]  )?;
[U] GAP LINE   crates/runner/src/grid.rs:3621  [production]  return Err(
[U] GAP LINE   crates/runner/src/grid.rs:3622  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
[U] GAP LINE   crates/runner/src/grid.rs:3623  [production]  "one candidate produced impossible multi-trade quality counts",
[U] GAP LINE   crates/runner/src/grid.rs:3624  [production]  ),
[U] GAP LINE   crates/runner/src/grid.rs:3625  [production]  );
[U] GAP LINE   crates/runner/src/grid.rs:3631  [production]  )?;
[UNPROVEN] GAP LINE   crates/runner/src/grid.rs:3636  [production]  )?;
[open] GAP LINE   crates/runner/src/grid.rs:4921  [production]  return Priced {
[open] GAP LINE   crates/runner/src/grid.rs:4922  [production]  paisa: 0,
[open] GAP LINE   crates/runner/src/grid.rs:4923  [production]  gapped: false,
[open] GAP LINE   crates/runner/src/grid.rs:4924  [production]  };
[open] GAP LINE   crates/runner/src/grid.rs:5003  [production]  return Priced {
[open] GAP LINE   crates/runner/src/grid.rs:5004  [production]  paisa: 0,
[open] GAP LINE   crates/runner/src/grid.rs:5005  [production]  gapped: false,
[open] GAP LINE   crates/runner/src/grid.rs:5006  [production]  };
[open] GAP LINE   crates/runner/src/grid.rs:5145  [production]  None => 0,
[open] GAP LINE   crates/runner/src/grid.rs:5266  [production]  core::cmp::Ordering::Greater => live,
[open] GAP LINE   crates/runner/src/grid.rs:5316  [production]  None => None,
[open] GAP LINE   crates/runner/src/grid.rs:5323  [production]  None => None,
[open] GAP LINE   crates/runner/src/grid.rs:5485  [production]  }
[open] GAP LINE   crates/runner/src/grid.rs:5486  [production]  (bar.open, true)
[open] GAP LINE   crates/runner/src/grid.rs:6310  [test code]  assert_eq!(
[open] GAP LINE   crates/runner/src/grid.rs:6311  [test code]  super::materialize_cell(&bars, &column, &mask, horizon, side, &grid, cell),
[open] GAP LINE   crates/runner/src/grid.rs:6312  [test code]  super::materialize_cell_over(
[open] GAP LINE   crates/runner/src/grid.rs:6313  [test code]  &bars, &column, &mask, horizon, side, &grid, cell, &facts
[open] GAP LINE   crates/runner/src/grid.rs:6316  [test code]  let ladders = crate::excursion::Ladders {
[open] GAP LINE   crates/runner/src/grid.rs:6317  [test code]  stops: &grid.stops,
[open] GAP LINE   crates/runner/src/grid.rs:6318  [test code]  targets: &grid.targets,
[open] GAP LINE   crates/runner/src/grid.rs:6319  [test code]  trails: &grid.trails,
[open] GAP LINE   crates/runner/src/grid.rs:6320  [test code]  };
[open] GAP LINE   crates/runner/src/grid.rs:6321  [test code]  let chosen = super::Chosen::from_cell(cell);
[open] GAP LINE   crates/runner/src/grid.rs:6322  [test code]  assert_eq!(
[open] GAP LINE   crates/runner/src/grid.rs:6323  [test code]  super::per_trade(&bars, &column, &mask, horizon, side, ladders, chosen),
[open] GAP LINE   crates/runner/src/grid.rs:6324  [test code]  super::per_trade_over(
[open] GAP LINE   crates/runner/src/grid.rs:6325  [test code]  &bars, &column, &mask, horizon, side, ladders, chosen, &facts
[open] GAP LINE   crates/runner/src/grid.rs:8058  [test code]  }
[open] GAP LINE   crates/runner/src/grid.rs:8067  [test code]  }
[open] GAP LINE   crates/runner/src/grid.rs:8075  [test code]  }
[open] GAP LINE   crates/runner/src/grid.rs:10132  [test code]  }
[open] GAP LINE   crates/runner/src/grid.rs:10686  [test code]  (Some(t), Some(ttp)) => ttp.arm < t,
[U] GAP BRANCH crates/runner/src/grid.rs:1733:17 True  [production]  let at = if at > last { last } else { at };
[open] GAP BRANCH crates/runner/src/grid.rs:2383:8 True  [production]  if !money_envelope_fits(bars, &timed.occupancy, facts.acceptance()) {
[open] GAP BRANCH crates/runner/src/grid.rs:2702:8 True  [production]  if actual != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/grid.rs:3613:32 True  [production]  if measured.trades != 1 || one_row.len() != 1 {
[open] GAP BRANCH crates/runner/src/grid.rs:3620:8 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
[open] GAP BRANCH crates/runner/src/grid.rs:3620:39 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3637:8 True  [production]  if row.signal_bar != candidate.signal
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3638:12 True  [production]  || row.entry_bar != candidate.entry
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3640:12 True  [production]  || row.exit_micros != stamp
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3641:12 True  [production]  || row.exit_bar < candidate.entry
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3642:12 True  [production]  || row.exit_bar > candidate.time_exit
[UNPROVEN] GAP BRANCH crates/runner/src/grid.rs:3643:12 True  [production]  || stamp < entry_micros
[open] GAP BRANCH crates/runner/src/grid.rs:4748:12 False  [production]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {
[open] GAP BRANCH crates/runner/src/grid.rs:4915:17 False  [production]  let Some(bar) = bars.get(entry.saturating_add(offset)) else {
[open] GAP BRANCH crates/runner/src/grid.rs:4990:14 False  [production]  ) if anchor > 0 => {
[open] GAP BRANCH crates/runner/src/grid.rs:5002:17 False  [production]  let Some(bar) = bars.get(entry.saturating_add(offset)) else {
[open] GAP BRANCH crates/runner/src/grid.rs:5483:8 False  [production]  if resting >= bar.low && resting <= bar.high {
[open] GAP BRANCH crates/runner/src/grid.rs:5483:30 False  [production]  if resting >= bar.low && resting <= bar.high {
[open] GAP BRANCH crates/runner/src/grid.rs:7867:25 False  [test code branch]  .find(|row| row.exit_bar > row.entry_bar && row.exit_bar + 2 < row.entry_bar + 30)
[open] GAP BRANCH crates/runner/src/grid.rs:8051:16 False  [test code branch]  if c.pessimistic < 0 {
[open] GAP BRANCH crates/runner/src/grid.rs:8062:16 False  [test code branch]  if c.trades > 0 {
[open] GAP? BRANCH crates/runner/src/grid.rs:8064:43 True  [assert! condition branch, side True (inside a closure)]  c.worst_trade <= 0 || c.pessimistic > 0,
[open] GAP? BRANCH crates/runner/src/grid.rs:8064:43 False  [assert! condition branch, side False (inside a closure)]  c.worst_trade <= 0 || c.pessimistic > 0,
[open] GAP BRANCH crates/runner/src/grid.rs:8069:16 False  [test code branch]  if c.pessimistic <= 0 {
[open] GAP? BRANCH crates/runner/src/grid.rs:10077:43 True  [assert! condition branch, side True (inside a closure)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
[open] GAP? BRANCH crates/runner/src/grid.rs:10077:43 False  [assert! condition branch, side False (inside a closure)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
[open] GAP BRANCH crates/runner/src/grid.rs:10126:12 False  [test code branch]  if !g.cells.is_empty() {
[open] GAP? BRANCH crates/runner/src/grid.rs:10486:36 False  [assert! condition branch, side False (inside a closure)]  .all(|row| victim < row.entry_bar || victim > row.exit_bar),
[open] GAP? BRANCH crates/runner/src/grid.rs:10907:63 False  [assert! condition branch, side False (inside a closure)]  .all(|pair| matches!(pair, [first, second] if second.entry_bar > first.exit_bar)),
[open] GAP? BRANCH crates/runner/src/identity.rs:1462:26 False  [assert! condition branch, side False (inside a closure)]  .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
[open] GAP BRANCH crates/runner/src/lib.rs:1105:24 False  [production]  if found {
[open] GAP? BRANCH crates/runner/src/lib.rs:2032:33 False  [assert! condition branch, side False (inside a closure)]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
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
[open] GAP? BRANCH crates/runner/src/outcome.rs:2626:41 False  [assert! condition branch, side False (inside a closure)]  assert!((0..bars.len()).all(|i| refused.at(i).is_none() && refused.was_refused(i)));
[open] GAP BRANCH crates/runner/src/outcome.rs:4874:65 False  [test code branch]  .filter(|w| matches!(w, [a, b] if a.2 != b.2 && b.0 - a.0 < horizon as usize))
[open] GAP? BRANCH crates/runner/src/outcome.rs:4918:48 False  [assert! condition branch, side False (inside a closure)]  .all(|w| matches!(w, [a, b] if a.2 == b.2)),
[open] GAP BRANCH crates/runner/src/outcome.rs:5138:16 False  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
[open] GAP BRANCH crates/runner/src/outcome.rs:5138:48 True  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
[open] GAP BRANCH crates/runner/src/outcome.rs:5138:48 False  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
[open] GAP? BRANCH crates/runner/src/portfolio.rs:785:20 False  [assert! condition branch, side False (inside a closure)]  if admitted == StrategyDigest::new([1; 32])
[open] GAP BRANCH crates/runner/src/resample.rs:561:16 False  [test code branch]  if let Some(b) = bars.get_mut(1) {
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
[U] GAP LINE   crates/runner/src/signal_candle_stop.rs:896  [production]  return Err(Error::Source);
[FAULT] GAP LINE   crates/runner/src/signal_candle_stop.rs:939  [production]  )?;
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
[open] GAP? BRANCH crates/runner/src/signal_candle_stop_tests.rs:808:39 False  [assert! condition branch, side False (inside a closure)]  .all(|period| period.day >= first && period.day <= last)
[open] GAP? BRANCH crates/runner/src/split.rs:513:60 True  [assert! condition branch, side True (inside a closure)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
[open] GAP? BRANCH crates/runner/src/split.rs:513:60 False  [assert! condition branch, side False (inside a closure)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
[open] GAP BRANCH crates/runner/src/synthetic.rs:334:27 True  [test code branch]  let (lo, hi) = if up < down { (up, down) } else { (down, up) };
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
[open] GAP LINE   crates/runner/src/trade.rs:2042  [test code]  return false;
[open] GAP LINE   crates/runner/src/trade.rs:2134  [test code]  assert_eq!(
[open] GAP LINE   crates/runner/src/trade.rs:2135  [test code]  bars.get(fill).map(|bar| bar.ts_micros),
[open] GAP LINE   crates/runner/src/trade.rs:2136  [test code]  bars.get(source)
[open] GAP LINE   crates/runner/src/trade.rs:2137  [test code]  .and_then(|bar| bar.ts_micros.checked_add(60_000_000)),
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
[open] GAP BRANCH crates/runner/src/trade.rs:1878:17 False  [test code branch]  let [previous, next] = pair else {
[open] GAP BRANCH crates/runner/src/trade.rs:2041:21 False  [test code branch]  let [before, after] = pair else {
[open] GAP BRANCH crates/runner/src/trade.rs:2133:20 True  [test code branch]  if let Some(fill) = *target {
[open] GAP? BRANCH crates/runner/src/trade.rs:2205:52 True  [assert! condition branch, side True (inside a closure)]  trade.entry_bar > previous_exit || n == 0,
[open] GAP? BRANCH crates/runner/src/trade.rs:2205:52 False  [assert! condition branch, side False (inside a closure)]  trade.entry_bar > previous_exit || n == 0,
[open] GAP BRANCH crates/runner/src/trade.rs:2640:38 True  [test code branch]  if path.priceable || !holed {
[open] GAP? BRANCH crates/runner/src/trade.rs:2819:42 False  [assert! condition branch, side False (inside a closure)]  .all(|trade| victim < trade.entry_bar || victim > trade.exit_bar),
[open] GAP? BRANCH crates/runner/src/trade.rs:2825:33 False  [assert! condition branch, side False (inside a closure)]  !held.priceable
[open] GAP? BRANCH crates/runner/src/trade.rs:2826:40 False  [assert! condition branch, side False (inside a closure)]  && held.entry_bar <= victim
[open] GAP? BRANCH crates/runner/src/trade.rs:3031:17 True  [assert! condition branch, side True]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
[open] GAP? BRANCH crates/runner/src/trade.rs:3031:34 False  [assert! condition branch, side False (inside a closure)]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
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
[open] GAP LINE   crates/runner/src/validate.rs:2904  [production]  )?;
[open] GAP LINE   crates/runner/src/validate.rs:2910  [production]  )?;
[open] GAP LINE   crates/runner/src/validate.rs:2949  [production]  }
[open] GAP LINE   crates/runner/src/validate.rs:2952  [production]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
[HASH] GAP LINE   crates/runner/src/validate.rs:3071  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
[HASH] GAP LINE   crates/runner/src/validate.rs:3072  [production]  "Long/Short policy or resolution identities are indistinguishable",
[HASH] GAP LINE   crates/runner/src/validate.rs:3073  [production]  ));
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
[open] GAP LINE   crates/runner/src/validate.rs:4634  [production]  out.refused =
[open] GAP LINE   crates/runner/src/validate.rs:4635  [production]  Some("walk-forward training execution boundaries are not monotonic".to_owned());
[open] GAP LINE   crates/runner/src/validate.rs:4636  [production]  return out;
[open] GAP LINE   crates/runner/src/validate.rs:4851  [production]  return None;
[open] GAP LINE   crates/runner/src/validate.rs:4972  [production]  out.refused = Some(
[open] GAP LINE   crates/runner/src/validate.rs:4973  [production]  "a walk-forward fold selected a candidate without its training side"
[open] GAP LINE   crates/runner/src/validate.rs:4974  [production]  .to_owned(),
[open] GAP LINE   crates/runner/src/validate.rs:4975  [production]  );
[open] GAP LINE   crates/runner/src/validate.rs:4976  [production]  return out;
[open] GAP LINE   crates/runner/src/validate.rs:4999  [production]  out.refused = Some(
[open] GAP LINE   crates/runner/src/validate.rs:5000  [production]  "walk-forward OOS execution boundaries are not monotonic".to_owned(),
[open] GAP LINE   crates/runner/src/validate.rs:5001  [production]  );
[open] GAP LINE   crates/runner/src/validate.rs:5002  [production]  return out;
[open] GAP LINE   crates/runner/src/validate.rs:5016  [production]  Err(why) => {
[open] GAP LINE   crates/runner/src/validate.rs:5017  [production]  out.refused = Some(why);
[open] GAP LINE   crates/runner/src/validate.rs:5018  [production]  return out;
[open] GAP LINE   crates/runner/src/validate.rs:5092  [production]  0
[open] GAP LINE   crates/runner/src/validate.rs:5204  [production]  return out;
[open] GAP LINE   crates/runner/src/validate.rs:5585  [test code]  builder_was_called = true;
[open] GAP LINE   crates/runner/src/validate.rs:5586  [test code]  Err("the zero-rung door must refuse before building".to_owned())
[open] GAP LINE   crates/runner/src/validate.rs:5587  [test code]  };
[open] GAP LINE   crates/runner/src/validate.rs:5723  [test code]  &|_| {},
[open] GAP LINE   crates/runner/src/validate.rs:5756  [test code]  &|_| {},
[open] GAP LINE   crates/runner/src/validate.rs:6016  [test code]  &|_| {
[open] GAP LINE   crates/runner/src/validate.rs:6017  [test code]  silent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
[open] GAP LINE   crates/runner/src/validate.rs:6018  [test code]  },
[open] GAP LINE   crates/runner/src/validate.rs:6591  [test code]  None => f.out_of_sample.worst_case_positive(),
[open] GAP LINE   crates/runner/src/validate.rs:6622  [test code]  assert!(
[open] GAP LINE   crates/runner/src/validate.rs:6623  [test code]  f.chosen_exit.is_some(),
[open] GAP LINE   crates/runner/src/validate.rs:6654  [test code]  continue;
[open] GAP LINE   crates/runner/src/validate.rs:6952  [test code]  continue;
[open] GAP LINE   crates/runner/src/validate.rs:6955  [test code]  continue;
[open] GAP LINE   crates/runner/src/validate.rs:7374  [test code]  mask.without_bit(383)
[open] GAP LINE   crates/runner/src/validate.rs:7392  [test code]  Direction::Short => Direction::Long,
[open] GAP LINE   crates/runner/src/validate.rs:7572  [test code]  matches!(
[open] GAP LINE   crates/runner/src/validate.rs:7587  [test code]  Direction::Short => Direction::Long,
[open] GAP LINE   crates/runner/src/validate.rs:7590  [test code]  matches!(
[open] GAP LINE   crates/runner/src/validate.rs:7692  [test code]  }
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
[open] GAP BRANCH crates/runner/src/validate.rs:2621:44 True  [production]  if swept.sweep.halted.is_some() || !swept.is_complete() {
[U] GAP BRANCH crates/runner/src/validate.rs:2745:16 True  [production]  if evaluated_cells != resolved.cell_count() {
[open] GAP BRANCH crates/runner/src/validate.rs:2795:24 False  [production]  if let Some(selected) = pair.selected {
[U] GAP BRANCH crates/runner/src/validate.rs:2808:12 True  [production]  if population_cells != expected_population {
[open] GAP BRANCH crates/runner/src/validate.rs:2839:12 False  [production]  if !pending.is_empty() {
[open] GAP BRANCH crates/runner/src/validate.rs:2951:12 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
[open] GAP BRANCH crates/runner/src/validate.rs:2951:55 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
[HASH] GAP BRANCH crates/runner/src/validate.rs:3070:8 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
[HASH] GAP BRANCH crates/runner/src/validate.rs:3070:57 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
[U] GAP BRANCH crates/runner/src/validate.rs:3103:17 False  [production]  let Some(next) = ordinal.checked_add(1) else {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:12 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:26 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[U] GAP BRANCH crates/runner/src/validate.rs:3110:39 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
[open] GAP BRANCH crates/runner/src/validate.rs:3529:12 True  [production]  if self.end > execution.len()
[U] GAP BRANCH crates/runner/src/validate.rs:3604:8 True  [production]  if alignment.onto.len() != signal_column.sources().len() {
[U] GAP BRANCH crates/runner/src/validate.rs:3625:8 True  [production]  if dropped != expected_dropped {
[U] GAP BRANCH crates/runner/src/validate.rs:3771:13 False  [production]  let Some(outcome) = oos.get(ordinal) else {
[U] GAP BRANCH crates/runner/src/validate.rs:4123:9 False  [production]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {
[U] GAP BRANCH crates/runner/src/validate.rs:4515:13 False  [production]  let Some(train) = bars.get(fold.train.0.clone()) else {
[open] GAP BRANCH crates/runner/src/validate.rs:4633:17 False  [production]  let Some(prefix) = training_execution_cursor.before(exec.bars, test_open) else {
[open] GAP BRANCH crates/runner/src/validate.rs:4850:20 True  [production]  if cell.trades == 0 {
[open] GAP BRANCH crates/runner/src/validate.rs:4971:21 False  [production]  let Some(chosen_side) = chosen_side else {
[open] GAP BRANCH crates/runner/src/validate.rs:4996:25 False  [production]  let Some(prefix) =
[open] GAP BRANCH crates/runner/src/validate.rs:5091:24 True  [production]  if *mask == ConditionMask::ZERO {
[open] GAP BRANCH crates/runner/src/validate.rs:5202:16 True  [production]  && !capture.observe_fold_and_retain(&fold, &visible, &scored, &oos_exact, best_ordinal)
[open] GAP BRANCH crates/runner/src/validate.rs:6620:16 True  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
[open] GAP BRANCH crates/runner/src/validate.rs:6620:57 True  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
[open] GAP BRANCH crates/runner/src/validate.rs:6620:57 False  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
[open] GAP BRANCH crates/runner/src/validate.rs:6653:16 True  [test code branch]  if f.out_of_sample.trades == 0 {
[open] GAP BRANCH crates/runner/src/validate.rs:6662:16 False  [test code branch]  if f.out_of_sample_exit != Some(f.out_of_sample.worst) {
[open] GAP BRANCH crates/runner/src/validate.rs:6951:21 False  [test code branch]  let Some(cell) = g.sharpest().or_else(|| g.best()) else {
[open] GAP BRANCH crates/runner/src/validate.rs:6954:20 True  [test code branch]  if cell.trades == 0 {
[open] GAP BRANCH crates/runner/src/validate.rs:7315:44 False  [test code branch]  .find(|(other, value)| *other != ordinal && *value != expected)
[open] GAP BRANCH crates/runner/src/validate.rs:7373:34 True  [test code branch]  visible.chosen = Some(if high_bit_set {
[open] GAP BRANCH crates/runner/src/validate.rs:7414:24 True  [test code branch]  exit.stop = if exit.stop.is_some() { None } else { Some(0) };
[open] GAP? BRANCH crates/runner/src/validate.rs:7675:17 False  [assert! condition branch, side False (inside a closure)]  candidate.oos_cell_digest.is_some() && candidate.out_of_sample_pessimistic.is_some()
[open] GAP BRANCH crates/runner/src/validate.rs:7683:20 False  [test code branch]  if let Some(choice) = proof.choice {
[open] GAP LINE   crates/runner/src/validate_cover_tests.rs:1459  [test code]  called.set(true);
[open] GAP LINE   crates/runner/src/validate_cover_tests.rs:1460  [test code]  Ok(built(slice))
[open] GAP LINE   crates/runner/src/validate_cover_tests.rs:1461  [test code]  };
[open] GAP LINE   crates/runner/src/validate_cover_tests.rs:1896  [test code]  &|_| {},
[open] GAP LINE   crates/runner/src/validate_cover_tests.rs:1962  [test code]  &|_| {},
[open] GAP? BRANCH crates/runner/tests/base_cov_admission_words.rs:66:69 False  [assert! condition branch, side False (inside a closure)]  name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
[open] GAP BRANCH crates/runner/tests/d_0694_record.rs:179:12 False  [test code branch]  if is_source
[open] GAP LINE   crates/runner/tests/end_to_end.rs:487  [test code]  Some(halt) => format!("HALTED k={} {:?}", halt.k, halt.breach),
[open] GAP? BRANCH crates/runner/tests/end_to_end.rs:115:39 True  [assert! condition branch, side True (inside a closure)]  exits.baseline().is_some() || exits.cells.is_empty(),
[open] GAP? BRANCH crates/runner/tests/end_to_end.rs:115:39 False  [assert! condition branch, side False (inside a closure)]  exits.baseline().is_some() || exits.cells.is_empty(),
[open] GAP? BRANCH crates/runner/tests/end_to_end.rs:383:13 False  [assert! condition branch, side False (inside a closure)]  !rest.starts_with("  NOT SUPPLIED") && rest.contains("variants evaluated")
[open] GAP LINE   crates/runner/tests/expression_pricing.rs:164  [test code]  }
[open] GAP BRANCH crates/runner/tests/expression_pricing.rs:108:31 False  [test code branch]  let either = (known.get(0) && truth.get(0)) || (known.get(369) && truth.get(369));
[open] GAP BRANCH crates/runner/tests/expression_pricing.rs:108:65 False  [test code branch]  let either = (known.get(0) && truth.get(0)) || (known.get(369) && truth.get(369));
[open] GAP BRANCH crates/runner/tests/expression_pricing.rs:111:29 False  [test code branch]  "!0" => known.get(0) && !truth.get(0),
[open] GAP BRANCH crates/runner/tests/expression_pricing.rs:112:36 False  [test code branch]  _ => either && known.get(63) && !truth.get(63),
[open] GAP BRANCH crates/runner/tests/expression_pricing.rs:134:20 False  [test code branch]  if let Some(cell) = priced.best() {
[open] GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:256  [test code]  assert!(
[open] GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:257  [test code]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
[open] GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:257:21 True  [assert! condition branch, side True]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
[open] GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:257:36 True  [assert! condition branch, side True (inside a closure)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
[open] GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:257:36 False  [assert! condition branch, side False (inside a closure)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
[open] GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:314:58 False  [assert! condition branch, side False (inside a closure)]  assert!(timed.eligible.iter().any(|item| item.entry_bar < corrupt_index && corrupt_index < item.exit_bar),
[open] GAP BRANCH crates/runner/tests/hole_after_exit.rs:100:13 False  [test code branch]  row.entry_bar % BARS_PER_SESSION < 300
[open] GAP LINE   crates/runner/tests/research_family_readiness.rs:438  [test code]  matches!(
[open] GAP LINE   crates/runner/tests/unwired_validation_record.rs:48  [test code]  }
[open] GAP BRANCH crates/runner/tests/unwired_validation_record.rs:37:16 False  [test code branch]  if path.file_name().is_some_and(|name| name != "tests") {
[open] GAP BRANCH crates/runner/tests/unwired_validation_record.rs:40:19 False  [test code branch]  } else if path.extension().is_some_and(|ext| ext == "rs") {

OPERATOR ITEMS (the [FAULT], [HASH] and [UNPROVEN] rows above, as written)
GAP-FAULT crates/runner/src/signal_candle_stop.rs:939 (`)?;` of `periods_before = days_before(..)` in Prepared::new). Only an allocator refusal reaches it: line 935 has just run days_before over the same source span and succeeded, so the span cannot overflow; the count is at most periods.len() and the day at most last_day + 1 (validated to be at most i64::MAX / DAY), so neither checked add fails; the remaining error is `try_reserve_exact(span).map_err(|_| Error::Allocation)`, a fault no input or call can produce on demand. The recipe allows no (U) for a fault, so this row stays a GAP for the operator's ruling (no test can drive it without a failing allocator, and the workspace denies unsafe_code, which a test allocator would need).
GAP-FAULT crates/runner/src/signal_candle_stop.rs:939 (periods_before `days_before(..)?` in Prepared::new): only an allocator refusal of `try_reserve_exact(span)` reaches it (line 935's identical span just succeeded; the counts cannot overflow).
GAP-FAULT crates/runner/src/grid.rs:3386-3388, 3439-3442, 3445-3448 (the `ReplayEvidenceAllocationRefused` closures of `try_reserve_exact` in replay_universe_v1 and replay_universe_from_candidates_v1): the requested sizes are an occupancy table's length and 1, both already held in memory, so only an allocator refusal reaches them.
GAP-FAULT crates/runner/src/exit_grid_policy.rs:3482-3486 (the BufferAllocationRefused closure of `values.try_reserve_exact(bars.len())` for the observed stops, targets and trails), crates/runner/src/resolved_grid_view.rs:126-130 (`copy.try_reserve_exact(values.len())` of an exact expression ladder) and 231-235 (`offsets.try_reserve_exact(row_count)` of the coordinate row offsets): each reservation is sized from data already held in memory (the bars, a ladder, the (S + 1)(T + 1) row table), so only an allocator refusal reaches the closure; no input or call produces one on demand, and a test allocator would need unsafe code, which the workspace denies.
GAP-HASH crates/runner/src/validate.rs:3070:8 True, 3070:57 True, 3071-3073 (`long.policy_digest() == short.policy_digest() || long.digest() == short.digest()` in authenticate_grid_pair_v4): reached only after 3045 (long is a Long policy, short a Short one) and 3051 (each policy_digest equals its policy's digest and each digest equals digest_resolved of its fields). The policy digest hashes the side, and the resolution digest hashes the policy digest, so equal digests need two different preimages with one blake3 output. No input or tampering short of a hash collision reaches it; it is not a code-level (U), so it is listed for the operator's ruling with the fault rows.
GAP-UNPROVEN crates/runner/src/grid.rs:3636 and the operands of 3637-3643 (the replayed row against its candidate in priceable_candidate_path_v1): believed unreachable (row_of copies the candidate's signal and entry and stamps entry and exit from the same bars, exit = entry + an offset bounded by the candidate's time exit), but the bound of every candidate's time_exit inside the slice was not traced to its builder in this session, so no (U) is claimed.
GAP-FAULT crates/runner/src/resolved_grid_view.rs:222, 225, 228 (the checked_add/checked_mul `)?` of coordinate_row_offsets), 253 and 276 (the same in canonical_coordinate_ordinal): an overflow needs the stop and target ladders to hold about 2^32 rungs each, so that (S + 1)(T + 1) passes usize::MAX; each ladder is a Vec<Ppm> resolved from a rung plan of as many percentiles, i.e. tens of GiB per vector. Like the allocator rows, no test can build such an input; listed for the operator's ruling with them.
GAP-UNPROVEN crates/runner/src/expression_execution.rs:336 (the `)?` of evaluate_resolved_expression_policy_v1 in evaluate_expression_with_attested): believed reachable only through an allocator refusal (ladders' and reserve_policy_cells_v1's try_reserve_exact), with the integrity, ladder and cell-count refusals ruled out by require_runtime_integrity two statements earlier and by the enumeration that defines cell_count; the ladder refusal (Ladder::new of an empty level list) was not traced to every resolution in this session, so no (U) is claimed.
GAP-FAULT crates/runner/src/exit_grid_policy.rs:3697-3698, 3747-3748 (the BufferAllocationRefused closures of `out.try_reserve_exact(reserve)` in admitted_ratio_pairs and `bitmap.try_reserve_exact(length)` in ratio_bitmap): each size is at most the stop count times the target count of ladders already resolved and held in memory (the pairs reservation is also capped by the policy's max_pairs), so only an allocator refusal reaches the closure. A rung plan of tens of thousands of percentiles per axis asks for gigabytes, which one host's allocator grants and another's refuses depending on its memory and overcommit policy, so no portable test drives it; listed for the operator's ruling with the other allocator rows.
GAP-UNPROVEN crates/runner/src/exit_grid_policy.rs:2398 (the `)?` of crate::grid::evaluate_resolved_policy_v1 in evaluate_with_attested): believed reachable only through an allocator refusal (reserve_policy_cells_v1's try_reserve_exact). Its other refusals are an invalid resolved ladder, which needs a resolution with an empty or unordered level list that resolve_levels never builds, and ReplayCellCountMismatch, which needs the coordinate enumerator to disagree with checked_policy_cell_count; the test checked_cell_count_matches_every_small_ratio_filtered_enumeration measures that agreement on small grids, but it was not proved for every grid in this session, so no (U) is claimed.
GAP-UNPROVEN crates/runner/src/exit_grid_policy.rs:2924 (the `)?` of crate::grid::replay_universe_v1 in replay_selected_universe_with): its refusals are the replay allocator closures (grid.rs 3386-3388, 3439-3442, 3445-3448, listed as allocator rows), the priceable-path reconciliation (grid.rs 3636-3643, listed as unproven) and the consistency checks of refused_candidate_path_v1; believed unreachable after the door's own checks, not proved in this session.
GAP-UNPROVEN crates/runner/src/exit_grid_policy.rs:3789 (the `?` of `target_setting.checked_mul(armed_trails)` in coordinate_row_width): believed unreachable on a 64-bit target: the callers pass a target setting no larger than the resolution's target count, and the resolution's cell count (checked_policy_cell_count, u128 arithmetic, refused above u64::MAX) already contains that count times the same armed-trail number; the callers' bound on the target setting was not traced in this session.
GAP-UNPROVEN crates/runner/src/exit_grid_policy.rs:4138:12 True, 4139:12 True, 4140:12 True, 4155:8 True, 4156-4159, 4172:12 True, 4173-4176 (require_complete_acceptance's census operands, ColumnShapeMismatch and ColumnSourcesNotIncreasing in validate_column_sources): each needs an indicators::Column whose private fields break its own invariants (an acceptance census that does not reconcile with an all-true, full-length acceptance; bits and sources of different lengths; sources not strictly increasing). Column's fields are private to indicators, so no runner test can build one; the constructors read in this session (build, reproject) keep the invariants, but the others (build_withholding, reproject_checked, try_clone_exact, clear_before, replace_gapfib, replace_orb_and_gapfib) were not traced, so no (U) is claimed.
`````

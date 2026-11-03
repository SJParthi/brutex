# gaps-LS2.txt.md: runner coverage gaps by Route B

Shard BASE-COV-LIBS-LS2 (crate runner). BEFORE figures: measured at START (4f49dbf43e79 = bd04fb299d23 + kit + OWNER + the one-line test fix), before this shard's new tests; refreshed at the final tip when its Route B is read.
One fenced block below holds the list.

`````text
Route B (d3-route-b-cloud.sh, pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0-rust-1.99.0-nightly) on tip 4f49dbf43e791ba431566f9e86eba46db7a95a4b, set reading whole:runner.
Log: /tmp/claude-0/-home-user-brutex/b3671350-4320-59ae-92c2-fdf03932a703/scratchpad/logs/4f49dbf43e79-routeb.log (this host; not kept)
TOTAL runner: lines 46145/47136 (uncovered 991), branches 3349/4218 (uncovered 869), files read 69; NOT READ: crates/runner/benches/ratio.rs ([[bench]] test = false: no test object; L-BCL-1).
Rows: 1860 uncovered lines and branch sides = 224 (F) (in fu-LS2.txt.md) + 1636 others below (GAP, GAP? = read by hand, or (U) where fu-LS2.txt.md proves it).

PER FILE (lines a/b, branches c/d)
  crates/runner/src/admission_reader_cover_tests.rs lines 445/445 branches 6/8  GAPS
  crates/runner/src/admission_statistics_cover_tests.rs lines 235/235 branches 0/0
  crates/runner/src/admission.rs lines 6147/6177 branches 303/310  GAPS
  crates/runner/src/align.rs lines 147/147 branches 8/8
  crates/runner/src/audit.rs lines 1982/2002 branches 127/162  GAPS
  crates/runner/src/bootstrap_family_pass_tests.rs lines 387/387 branches 7/10  GAPS
  crates/runner/src/bootstrap_family_pass.rs lines 269/269 branches 35/36  GAPS
  crates/runner/src/bootstrap_zero_v2_tests.rs lines 419/419 branches 20/20
  crates/runner/src/bootstrap_zero_v2.rs lines 323/324 branches 47/50  GAPS
  crates/runner/src/bootstrap.rs lines 1766/1792 branches 158/188  GAPS
  crates/runner/src/bound.rs lines 228/228 branches 14/14
  crates/runner/src/closed.rs lines 146/146 branches 10/10
  crates/runner/src/excursion_clock_tests.rs lines 89/89 branches 8/8
  crates/runner/src/excursion.rs lines 1098/1109 branches 105/120  GAPS
  crates/runner/src/exit_grid_policy_cover_tests.rs lines 90/90 branches 0/0
  crates/runner/src/exit_grid_policy.rs lines 4726/4973 branches 312/430  GAPS
  crates/runner/src/expression_execution_tests.rs lines 261/266 branches 1/2  GAPS
  crates/runner/src/expression_execution.rs lines 277/335 branches 21/30  GAPS
  crates/runner/src/expression_oos_tests.rs lines 394/403 branches 6/6  GAPS
  crates/runner/src/expression_oos.rs lines 250/252 branches 19/20  GAPS
  crates/runner/src/expression_validation_codec.rs lines 194/194 branches 24/24
  crates/runner/src/expression_validation_tests.rs lines 521/535 branches 8/10  GAPS
  crates/runner/src/expression_validation.rs lines 397/404 branches 69/90  GAPS
  crates/runner/src/expression.rs lines 204/207 branches 5/6  GAPS
  crates/runner/src/family_allocation_v1_tests.rs lines 71/71 branches 0/0
  crates/runner/src/family_allocation_v1.rs lines 112/112 branches 16/16
  crates/runner/src/grid_cover_tests.rs lines 105/105 branches 2/4  GAPS
  crates/runner/src/grid.rs lines 4998/5179 branches 330/390  GAPS
  crates/runner/src/identity.rs lines 657/657 branches 21/22  GAPS
  crates/runner/src/lib.rs lines 952/957 branches 56/66  GAPS
  crates/runner/src/outcome.rs lines 2228/2249 branches 265/326  GAPS
  crates/runner/src/pbo.rs lines 252/252 branches 33/34  GAPS
  crates/runner/src/portfolio.rs lines 750/750 branches 61/64  GAPS
  crates/runner/src/rank.rs lines 707/709 branches 14/14  GAPS
  crates/runner/src/replay_mask.rs lines 136/136 branches 14/14
  crates/runner/src/report_cover_tests.rs lines 160/160 branches 6/8  GAPS
  crates/runner/src/report.rs lines 993/999 branches 64/74  GAPS
  crates/runner/src/resample.rs lines 405/405 branches 26/34  GAPS
  crates/runner/src/research_admission_projection_codec.rs lines 27/27 branches 6/6
  crates/runner/src/research_admission_projection.rs lines 56/65 branches 2/2  GAPS
  crates/runner/src/research_exit_grid_tests.rs lines 41/42 branches 0/0  GAPS
  crates/runner/src/research_exit_grid.rs lines 146/149 branches 6/6  GAPS
  crates/runner/src/research_family.rs lines 163/163 branches 14/14
  crates/runner/src/resolved_grid_view.rs lines 327/352 branches 60/74  GAPS
  crates/runner/src/search_allocation_v1_tests.rs lines 124/124 branches 0/0
  crates/runner/src/search_allocation_v1.rs lines 101/101 branches 12/12
  crates/runner/src/signal_candle_stop_tests.rs lines 739/740 branches 29/30  GAPS
  crates/runner/src/signal_candle_stop.rs lines 1189/1202 branches 189/248  GAPS
  crates/runner/src/significance_cover_tests.rs lines 31/31 branches 0/0
  crates/runner/src/significance.rs lines 506/510 branches 72/76  GAPS
  crates/runner/src/split.rs lines 234/235 branches 37/42  GAPS
  crates/runner/src/synthetic.rs lines 149/149 branches 9/16  GAPS
  crates/runner/src/topn.rs lines 821/821 branches 70/76  GAPS
  crates/runner/src/trade_clock_tests.rs lines 137/137 branches 2/4  GAPS
  crates/runner/src/trade.rs lines 1574/1612 branches 169/224  GAPS
  crates/runner/src/validate.rs lines 4118/4335 branches 345/598  GAPS
  crates/runner/tests/auto_reporting.rs lines 77/77 branches 0/0
  crates/runner/tests/base_cov_admission_words.rs lines 115/115 branches 7/8  GAPS
  crates/runner/tests/d_0694_record.rs lines 143/143 branches 9/10  GAPS
  crates/runner/tests/end_to_end.rs lines 226/227 branches 4/10  GAPS
  crates/runner/tests/expression_pricing.rs lines 149/160 branches 9/14  GAPS
  crates/runner/tests/fib_rung_sweep.rs lines 119/119 branches 10/10
  crates/runner/tests/fixed_deadline_readiness.rs lines 211/211 branches 30/54  GAPS
  crates/runner/tests/foreign_slice_deadline_readiness.rs lines 268/276 branches 13/28  GAPS
  crates/runner/tests/hole_after_exit.rs lines 105/106 branches 7/8  GAPS
  crates/runner/tests/join_answer_is_unchanged.rs lines 66/67 branches 0/0  GAPS
  crates/runner/tests/prefix_only_cadence.rs lines 124/124 branches 4/4
  crates/runner/tests/research_family_readiness.rs lines 441/448 branches 3/4  GAPS
  crates/runner/tests/unwired_validation_record.rs lines 97/99 branches 10/12  GAPS

EVERY UNCOVERED LINE AND BRANCH SIDE THAT IS NOT (F)
GAP LINE   crates/runner/src/admission.rs:5683  [test code]  AdmissionCanonicalRefusalV2::Statistics(AdmissionStatisticsRefusalV2::Probability {
GAP LINE   crates/runner/src/admission.rs:5684  [test code]  field,
GAP LINE   crates/runner/src/admission.rs:5685  [test code]  kind,
GAP LINE   crates/runner/src/admission.rs:5686  [test code]  })
GAP LINE   crates/runner/src/admission.rs:5687  [test code]  })
GAP LINE   crates/runner/src/admission.rs:5719  [test code]  )?,
GAP LINE   crates/runner/src/admission.rs:5724  [test code]  )?,
GAP LINE   crates/runner/src/admission.rs:5728  [test code]  )?,
GAP LINE   crates/runner/src/admission.rs:5772  [test code]  return Err(AdmissionCanonicalRefusalV2::AuthorityProjectionMismatch);
GAP LINE   crates/runner/src/admission.rs:6040  [production]  AdmissionCanonicalRefusalV3::Statistics(AdmissionStatisticsRefusalV3::Probability {
GAP LINE   crates/runner/src/admission.rs:6041  [production]  field,
GAP LINE   crates/runner/src/admission.rs:6042  [production]  kind,
GAP LINE   crates/runner/src/admission.rs:6043  [production]  })
GAP LINE   crates/runner/src/admission.rs:6044  [production]  })
GAP LINE   crates/runner/src/admission.rs:6071  [production]  )?,
GAP LINE   crates/runner/src/admission.rs:6076  [production]  )?,
GAP LINE   crates/runner/src/admission.rs:6080  [production]  )?,
GAP LINE   crates/runner/src/admission.rs:6122  [production]  return Err(AdmissionCanonicalRefusalV3::AuthorityProjectionMismatch);
GAP LINE   crates/runner/src/admission.rs:7110  [test code]  AdmissionReasonV1::Trades => values.trades = ObservedU64V1::Refused,
GAP LINE   crates/runner/src/admission.rs:7160  [test code]  AdmissionReasonV1::LosingTrades => {
GAP LINE   crates/runner/src/admission.rs:7161  [test code]  values.losing_trades = ObservedU64V1::Refused;
GAP LINE   crates/runner/src/admission.rs:7162  [test code]  }
GAP LINE   crates/runner/src/admission.rs:7166  [test code]  AdmissionReasonV1::WinningTrades => {
GAP LINE   crates/runner/src/admission.rs:7167  [test code]  values.winning_trades = ObservedU64V1::Refused;
GAP LINE   crates/runner/src/admission.rs:7168  [test code]  }
GAP BRANCH crates/runner/src/admission.rs:2194:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
GAP BRANCH crates/runner/src/admission.rs:5056:36 True  [test code branch]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
GAP BRANCH crates/runner/src/admission.rs:5228:36 True  [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
GAP BRANCH crates/runner/src/admission.rs:5771:8 True  [test code branch]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
GAP BRANCH crates/runner/src/admission.rs:5771:45 True  [test code branch]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
GAP BRANCH crates/runner/src/admission.rs:6121:8 True  [production]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
GAP BRANCH crates/runner/src/admission.rs:6121:45 True  [production]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
GAP LINE   crates/runner/src/audit.rs:302  [production]  continue;
GAP LINE   crates/runner/src/audit.rs:584  [production]  }
GAP LINE   crates/runner/src/audit.rs:773  [production]  row(
GAP LINE   crates/runner/src/audit.rs:774  [production]  out,
GAP LINE   crates/runner/src/audit.rs:775  [production]  "PATHS UNPRICED, refused bar",
GAP LINE   crates/runner/src/audit.rs:776  [production]  &g.refused_paths.to_string(),
GAP LINE   crates/runner/src/audit.rs:777  [production]  "a bar between entry and exit failed the engine's own bar check, so \
GAP LINE   crates/runner/src/audit.rs:778  [production]  the round trip could not be priced. It STILL HELD THE POSITION and \
GAP LINE   crates/runner/src/audit.rs:779  [production]  blocked the next signal, so the sample below is smaller by these \
GAP LINE   crates/runner/src/audit.rs:780  [production]  and the sequence is unchanged. A tighter exit can lose a later \
GAP LINE   crates/runner/src/audit.rs:781  [production]  signal to one of these that a looser exit hid.",
GAP LINE   crates/runner/src/audit.rs:782  [production]  );
GAP LINE   crates/runner/src/audit.rs:935  [production]  grouped.push(',');
GAP LINE   crates/runner/src/audit.rs:1375  [production]  row(out, "folds", "REFUSED", "no fold was run, and this is why:");
GAP LINE   crates/runner/src/audit.rs:1376  [production]  let _ = writeln!(out, "    {why}");
GAP LINE   crates/runner/src/audit.rs:1377  [production]  let _ = writeln!(out);
GAP LINE   crates/runner/src/audit.rs:1378  [production]  return;
GAP LINE   crates/runner/src/audit.rs:2644  [test code]  || (what != "the sentence"
GAP LINE   crates/runner/src/audit.rs:2645  [test code]  && token == "V6"
GAP LINE   crates/runner/src/audit.rs:2646  [test code]  && text.contains("Selection V6")),
GAP BRANCH crates/runner/src/audit.rs:301:12 True  [production]  if prior.close <= 0 {
GAP BRANCH crates/runner/src/audit.rs:338:19 True  [production]  let sign = if paisa < 0 { "-" } else { "" };
GAP BRANCH crates/runner/src/audit.rs:378:19 False  [production]  let sign = if found.ppm < 0 { "-" } else { "+" };
GAP BRANCH crates/runner/src/audit.rs:574:20 False  [production]  if let Some(chosen) = selected.or_else(|| x.best()) {
GAP BRANCH crates/runner/src/audit.rs:674:50 False  [production]  if c.stop.is_none() && c.target.is_none() && c.tsl.is_none() && c.ttp.is_none() {
GAP BRANCH crates/runner/src/audit.rs:772:8 True  [production]  if g.refused_paths > 0 {
GAP BRANCH crates/runner/src/audit.rs:934:22 True  [production]  if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
GAP BRANCH crates/runner/src/audit.rs:934:42 True  [production]  if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
GAP BRANCH crates/runner/src/audit.rs:934:60 True  [production]  if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
GAP BRANCH crates/runner/src/audit.rs:934:60 False  [production]  if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
GAP BRANCH crates/runner/src/audit.rs:1166:12 True  [production]  if negative { "-" } else { "" },
GAP BRANCH crates/runner/src/audit.rs:1178:12 True  [production]  if negative { "-" } else { "" },
GAP BRANCH crates/runner/src/audit.rs:1299:35 False  [production]  (Some(a), Some(b)) if key(b) < key(a) => (Some(b), Some(a)),
GAP BRANCH crates/runner/src/audit.rs:1374:12 True  [production]  if let Some(why) = v.refused.as_deref() {
GAP? BRANCH crates/runner/src/audit.rs:2358:44 False  [assert! condition branch, side False (inside a closure)]  line.is_empty() || line.starts_with("  "),
GAP BRANCH crates/runner/src/audit.rs:2465:34 False  [test code branch]  .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()));
GAP? BRANCH crates/runner/src/audit.rs:2467:29 False  [assert! condition branch, side False (inside a closure)]  decision || token == "V6",
GAP? BRANCH crates/runner/src/audit.rs:2620:40 False  [assert! condition branch, side False (inside a closure)]  line.is_empty() || line.starts_with("  "),
GAP? BRANCH crates/runner/src/audit.rs:2643:42 False  [assert! condition branch, side False (inside a closure)]  .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
GAP? BRANCH crates/runner/src/audit.rs:2644:29 True  [assert! condition branch, side True (inside a closure)]  || (what != "the sentence"
GAP? BRANCH crates/runner/src/audit.rs:2644:29 False  [assert! condition branch, side False (inside a closure)]  || (what != "the sentence"
GAP? BRANCH crates/runner/src/audit.rs:2645:32 True  [assert! condition branch, side True (inside a closure)]  && token == "V6"
GAP? BRANCH crates/runner/src/audit.rs:2645:32 False  [assert! condition branch, side False (inside a closure)]  && token == "V6"
GAP? BRANCH crates/runner/src/audit.rs:2646:32 True  [assert! condition branch, side True (inside a closure)]  && text.contains("Selection V6")),
GAP? BRANCH crates/runner/src/audit.rs:2646:32 False  [assert! condition branch, side False (inside a closure)]  && text.contains("Selection V6")),
GAP? BRANCH crates/runner/src/audit.rs:2974:13 True  [assert! condition branch, side True]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
GAP? BRANCH crates/runner/src/audit.rs:2974:62 False  [assert! condition branch, side False (inside a closure)]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
GAP LINE   crates/runner/src/bootstrap.rs:112  [production]  return 0;
GAP LINE   crates/runner/src/bootstrap.rs:766  [production]  )?,
GAP LINE   crates/runner/src/bootstrap.rs:803  [production]  return None;
GAP LINE   crates/runner/src/bootstrap.rs:851  [production]  )?,
GAP LINE   crates/runner/src/bootstrap.rs:873  [production]  return None;
GAP LINE   crates/runner/src/bootstrap.rs:893  [production]  return None;
GAP LINE   crates/runner/src/bootstrap.rs:901  [production]  return None;
GAP LINE   crates/runner/src/bootstrap.rs:1348  [production]  return None;
GAP LINE   crates/runner/src/bootstrap.rs:1362  [production]  _ => left.cmp(right),
GAP LINE   crates/runner/src/bootstrap.rs:1547  [production]  _ => left.cmp(right),
GAP LINE   crates/runner/src/bootstrap.rs:1566  [production]  continue;
GAP LINE   crates/runner/src/bootstrap.rs:1569  [production]  continue;
GAP LINE   crates/runner/src/bootstrap.rs:1601  [production]  break;
GAP LINE   crates/runner/src/bootstrap.rs:1622  [production]  }
GAP LINE   crates/runner/src/bootstrap.rs:1676  [production]  return Performance::default();
GAP LINE   crates/runner/src/bootstrap.rs:3638  [test code]  {
GAP LINE   crates/runner/src/bootstrap.rs:3639  [test code]  disagreements.push((seed, shift, alpha, strategy));
GAP BRANCH crates/runner/src/bootstrap.rs:111:12 True  [production]  if n == 0 {
GAP BRANCH crates/runner/src/bootstrap.rs:628:17 False  [production]  let Some(own) = stats.get(s) else { continue };
GAP BRANCH crates/runner/src/bootstrap.rs:727:8 True  [production]  if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {
GAP BRANCH crates/runner/src/bootstrap.rs:802:8 True  [production]  if !observed.is_finite() {
GAP BRANCH crates/runner/src/bootstrap.rs:868:8 True  [production]  if stats.iter().any(|summary| {
GAP BRANCH crates/runner/src/bootstrap.rs:869:9 True  [production]  !summary.mean.is_finite()
GAP BRANCH crates/runner/src/bootstrap.rs:870:16 True  [production]  || !summary.standard_error.is_finite()
GAP BRANCH crates/runner/src/bootstrap.rs:892:8 True  [production]  if matched_or_exceeded > draws || !observed.is_finite() {
GAP BRANCH crates/runner/src/bootstrap.rs:892:39 True  [production]  if matched_or_exceeded > draws || !observed.is_finite() {
GAP BRANCH crates/runner/src/bootstrap.rs:900:8 True  [production]  if !p_value.is_finite() {
GAP BRANCH crates/runner/src/bootstrap.rs:1347:8 True  [production]  if observed.iter().any(|value| !value.is_finite()) {
GAP BRANCH crates/runner/src/bootstrap.rs:1521:33 True  [production]  if alpha_ppm > 1_000_000 || draws == 0 {
GAP BRANCH crates/runner/src/bootstrap.rs:1565:13 False  [production]  let Some(&strategy) = order.get(rank) else {
GAP BRANCH crates/runner/src/bootstrap.rs:1568:13 False  [production]  let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {
GAP BRANCH crates/runner/src/bootstrap.rs:1588:16 False  [production]  if let Some(slot) = bars.get_mut(rank) {
GAP BRANCH crates/runner/src/bootstrap.rs:1599:17 False  [production]  let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))
GAP BRANCH crates/runner/src/bootstrap.rs:1620:16 False  [production]  if let Some(this_round) = out.get_mut(first..) {
GAP BRANCH crates/runner/src/bootstrap.rs:1675:8 True  [production]  if n == 0 {
GAP BRANCH crates/runner/src/bootstrap.rs:2691:16 False  [test code branch]  if let Some(first) = series.first_mut() {
GAP? BRANCH crates/runner/src/bootstrap.rs:3398:46 True  [assert! condition branch, side True (inside a closure)]  r.round == expected_round || r.round == expected_round + 1,
GAP? BRANCH crates/runner/src/bootstrap.rs:3398:46 False  [assert! condition branch, side False (inside a closure)]  r.round == expected_round || r.round == expected_round + 1,
GAP BRANCH crates/runner/src/bootstrap.rs:3636:28 True  [test code branch]  if legacy.iter().any(|row| row.strategy == strategy) != exact
GAP BRANCH crates/runner/src/bootstrap.rs:3637:32 True  [test code branch]  || receipt.is_rejected(strategy) != Some(exact)
GAP BRANCH crates/runner/src/bootstrap_family_pass.rs:461:20 True  [production]  || !stat.standard_error.is_finite()
GAP? BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:194:13 False  [assert! condition branch, side False (inside a closure)]  strict > 0 && strict < draws
GAP LINE   crates/runner/src/bootstrap_zero_v2.rs:192  [production]  return Err(Refusal::Numerical);
GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:166:8 True  [production]  if work > u128::from(bounds.max_work) || bytes > u128::from(bounds.max_bytes) {
GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:187:45 True  [production]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
GAP BRANCH crates/runner/src/bootstrap_zero_v2.rs:191:12 True  [production]  if !observed.is_finite() {
GAP LINE   crates/runner/src/excursion.rs:169  [production]  return None;
GAP LINE   crates/runner/src/excursion.rs:780  [production]  return 0;
GAP LINE   crates/runner/src/excursion.rs:810  [production]  return 0;
GAP LINE   crates/runner/src/excursion.rs:1405  [production]  break;
GAP LINE   crates/runner/src/excursion.rs:1664  [test code]  _ => true,
GAP LINE   crates/runner/src/excursion.rs:2534  [test code]  return None;
GAP BRANCH crates/runner/src/excursion.rs:168:12 True  [production]  if step <= 0 || count == 0 {
GAP BRANCH crates/runner/src/excursion.rs:168:25 True  [production]  if step <= 0 || count == 0 {
GAP BRANCH crates/runner/src/excursion.rs:214:35 True  [production]  if observed.is_empty() || count == 0 {
GAP BRANCH crates/runner/src/excursion.rs:234:24 False  [production]  if let Some(prefix) = observed.get_mut(..bound) {
GAP BRANCH crates/runner/src/excursion.rs:779:13 False  [production]  let (Some(&low), Some(&high)) = (self.low_run.get(at), self.high_run.get(at)) else {
GAP BRANCH crates/runner/src/excursion.rs:809:13 False  [production]  let (Some(&low), Some(&high)) = (self.low_run.get(at), self.high_run.get(at)) else {
GAP BRANCH crates/runner/src/excursion.rs:1221:20 False  [production]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {
GAP BRANCH crates/runner/src/excursion.rs:1232:20 False  [production]  if let Some(slot) = out.favourable.get_mut(target_cursor) {
GAP BRANCH crates/runner/src/excursion.rs:1253:20 False  [production]  if let Some(slot) = armed_from.get_mut(arm) {
GAP BRANCH crates/runner/src/excursion.rs:1397:13 False  [production]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
GAP BRANCH crates/runner/src/excursion.rs:1472:16 False  [production]  if let Some(slot) = crossed_at.get_mut(*cursor) {
GAP BRANCH crates/runner/src/excursion.rs:1479:16 False  [production]  if let Some(slot) = peak.get_mut(*cursor) {
GAP BRANCH crates/runner/src/excursion.rs:1482:16 False  [production]  if let Some(slot) = peak_raised.get_mut(*cursor) {
GAP BRANCH crates/runner/src/excursion.rs:2533:12 True  [test code branch]  if observed.is_empty() || count == 0 {
GAP BRANCH crates/runner/src/excursion.rs:2533:35 True  [test code branch]  if observed.is_empty() || count == 0 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:462  [production]  return Err(ExitGridErrorV1::MissingRunIdentity("feed"));
GAP LINE   crates/runner/src/exit_grid_policy.rs:465  [production]  return Err(ExitGridErrorV1::MissingRunIdentity("commit"));
GAP LINE   crates/runner/src/exit_grid_policy.rs:492  [production]  pub const fn mask(self) -> ConditionMask {
GAP LINE   crates/runner/src/exit_grid_policy.rs:493  [production]  self.mask
GAP LINE   crates/runner/src/exit_grid_policy.rs:494  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:532  [production]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:615  [production]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
GAP LINE   crates/runner/src/exit_grid_policy.rs:1117  [production]  return Err(ExitGridErrorV1::ResolvedRungLimitExceeded {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1118  [production]  stop: stops.len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:1119  [production]  target: targets.len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:1120  [production]  trail: trails.len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:1121  [production]  max: self.rungs.max_levels_per_axis(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:1122  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:1132  [production]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1133  [production]  needed: pair_count,
GAP LINE   crates/runner/src/exit_grid_policy.rs:1134  [production]  max: self.ratios.max_pairs(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:1135  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:1172  [test code]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:1454  [production]  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1455  [production]  f.debug_struct("AttestedTrainingV1")
GAP LINE   crates/runner/src/exit_grid_policy.rs:1456  [production]  .field("resolution_digest", &self.resolution_digest)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1457  [production]  .field("bars", &self.bars)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1458  [production]  .field("column", &self.column)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1459  [production]  .field("horizon", &self.horizon)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1460  [production]  .field("column_digest", &self.column_digest)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1461  [production]  .field("evaluation_spec", &self.evaluation_spec)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1462  [production]  .finish_non_exhaustive()
GAP LINE   crates/runner/src/exit_grid_policy.rs:1463  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:1489  [production]  pub const fn resolution_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1490  [production]  self.resolution_digest
GAP LINE   crates/runner/src/exit_grid_policy.rs:1491  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:1519  [production]  pub const fn column_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1520  [production]  self.column_digest
GAP LINE   crates/runner/src/exit_grid_policy.rs:1521  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:1548  [production]  pub const fn resolution_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1549  [production]  self.resolution_digest
GAP LINE   crates/runner/src/exit_grid_policy.rs:1550  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:1576  [production]  pub fn is_evaluation(&self, evaluated: &EvaluatedExitGridV1) -> bool {
GAP LINE   crates/runner/src/exit_grid_policy.rs:1577  [production]  core::ptr::eq(self.evaluated, evaluated)
GAP LINE   crates/runner/src/exit_grid_policy.rs:1578  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:2046  [production]  pub const fn run_id(&self) -> RunId {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2047  [production]  self.run_id
GAP LINE   crates/runner/src/exit_grid_policy.rs:2048  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:2052  [production]  pub const fn selected_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2053  [production]  self.selected_digest
GAP LINE   crates/runner/src/exit_grid_policy.rs:2054  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:2075  [production]  pub const fn local_cell(&self) -> Option<&Cell> {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2076  [production]  self.local_cell.as_ref()
GAP LINE   crates/runner/src/exit_grid_policy.rs:2077  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:2081  [production]  pub const fn digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2082  [production]  self.digest
GAP LINE   crates/runner/src/exit_grid_policy.rs:2083  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:2111  [production]  return false;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2131  [production]  let Some(next) = pricing_refused.checked_add(1) else {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2132  [production]  return false;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2134  [production]  pricing_refused = next;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2398  [production]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2457  [test code]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2504  [production]  ExitGridSelectorV1::EdgeThenPessimistic => admitted.max_by_key(|cell| {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2505  [production]  (
GAP LINE   crates/runner/src/exit_grid_policy.rs:2506  [production]  cell.edge_ratio(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2507  [production]  cell.pessimistic,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2508  [production]  crate::grid::merit(cell),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2509  [production]  )
GAP LINE   crates/runner/src/exit_grid_policy.rs:2510  [production]  }),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2518  [production]  ExitGridSelectorV1::OperatorRule => admitted.max_by_key(|cell| {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2519  [production]  (
GAP LINE   crates/runner/src/exit_grid_policy.rs:2520  [production]  cell.reward_to_risk_bp(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2521  [production]  cell.pessimistic,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2522  [production]  crate::grid::merit(cell),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2523  [production]  )
GAP LINE   crates/runner/src/exit_grid_policy.rs:2524  [production]  }),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2676  [production]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
GAP LINE   crates/runner/src/exit_grid_policy.rs:2701  [production]  return Err(ExitGridErrorV1::SelectionDigestMismatch);
GAP LINE   crates/runner/src/exit_grid_policy.rs:2769  [production]  return Err(ExitGridErrorV1::RefusedExecutionPaths {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2770  [production]  paths: universe.pricing_refused_paths,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2771  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:2777  [production]  return Err(ExitGridErrorV1::ReplayQualityLimitExceeded {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2778  [production]  ambiguous_bars: value.ambiguous_bars,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2779  [production]  max_ambiguous_bars: self.policy.max_ambiguous_bars(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2780  [production]  gap_fills: value.gapped,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2781  [production]  max_gap_fills: self.policy.max_gap_fills(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:2782  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:2867  [production]  return Err(ExitGridErrorV1::RunIdentityMismatch("mask"));
GAP LINE   crates/runner/src/exit_grid_policy.rs:2876  [production]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2878  [production]  return Err(ExitGridErrorV1::OosOverlapsTraining {
GAP LINE   crates/runner/src/exit_grid_policy.rs:2879  [production]  training_last: self.training_last_ts_micros,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2880  [production]  first_oos: first_test_stamp,
GAP LINE   crates/runner/src/exit_grid_policy.rs:2881  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:2884  [production]  return Err(ExitGridErrorV1::EvaluationSpecMismatch);
GAP LINE   crates/runner/src/exit_grid_policy.rs:2890  [production]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
GAP LINE   crates/runner/src/exit_grid_policy.rs:2895  [production]  ) && selected.coordinate.stop != self.forced_stop_index
GAP LINE   crates/runner/src/exit_grid_policy.rs:2897  [production]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
GAP LINE   crates/runner/src/exit_grid_policy.rs:2906  [production]  return Err(ExitGridErrorV1::RunIdentityMismatch(
GAP LINE   crates/runner/src/exit_grid_policy.rs:2907  [production]  "hoisted OOS slice facts do not cover the OOS series",
GAP LINE   crates/runner/src/exit_grid_policy.rs:2908  [production]  ));
GAP LINE   crates/runner/src/exit_grid_policy.rs:2924  [production]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:2982  [production]  crate::excursion::Side::Short => CostDirection::Short,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3405  [production]  return Err(ExitGridErrorV1::PercentileLimitExceeded {
GAP LINE   crates/runner/src/exit_grid_policy.rs:3406  [production]  axis,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3407  [production]  supplied: values.len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:3408  [production]  max,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3409  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:3413  [production]  return true;
GAP LINE   crates/runner/src/exit_grid_policy.rs:3416  [production]  return true;
GAP LINE   crates/runner/src/exit_grid_policy.rs:3482  [production]  ExitGridErrorV1::BufferAllocationRefused {
GAP LINE   crates/runner/src/exit_grid_policy.rs:3483  [production]  buffer: name,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3484  [production]  elements: bars.len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:3485  [production]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:3486  [production]  })?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:3546  [production]  RangeResolutionV1::PpmFloor => floor,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3594  [production]  return Err(ExitGridErrorV1::InvalidOpenInterest {
GAP LINE   crates/runner/src/exit_grid_policy.rs:3595  [production]  index,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3596  [production]  value: bar.open_interest,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3597  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:3668  [production]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
GAP LINE   crates/runner/src/exit_grid_policy.rs:3697  [production]  elements: reserve,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3698  [production]  })?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:3747  [production]  elements: length,
GAP LINE   crates/runner/src/exit_grid_policy.rs:3748  [production]  })?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:3789  [production]  )?)
GAP LINE   crates/runner/src/exit_grid_policy.rs:4028  [production]  crate::grid::ReplayPathV1::BlockOnly => h.update(&[2]),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4029  [production]  crate::grid::ReplayPathV1::CrossingRefused => h.update(&[3]),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4030  [production]  crate::grid::ReplayPathV1::BlockOnlyAndCrossingRefused => h.update(&[4]),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4052  [production]  CostDirection::Short => 2,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4098  [production]  None => h.update(&[0]),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4105  [production]  None => h.update(&[0]),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4127  [production]  return Err(ExitGridErrorV1::IncompleteExecutionAcceptance {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4128  [production]  bars,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4129  [production]  refused_or_missing: u64::try_from(bars).unwrap_or(u64::MAX),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4130  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:4156  [production]  return Err(ExitGridErrorV1::ColumnShapeMismatch {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4157  [production]  bits: column.bits().len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4158  [production]  sources: column.sources().len(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4159  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:4167  [production]  return Err(ExitGridErrorV1::ColumnSourceBeforeOos {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4168  [production]  source,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4169  [production]  first_oos: first_permitted,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4170  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:4173  [production]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4174  [production]  previous: previous.unwrap_or(source),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4175  [production]  current: source,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4176  [production]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:4232  [production]  return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
GAP LINE   crates/runner/src/exit_grid_policy.rs:4233  [production]  "aggregate paisa accumulator",
GAP LINE   crates/runner/src/exit_grid_policy.rs:4234  [production]  ));
GAP LINE   crates/runner/src/exit_grid_policy.rs:4247  [production]  return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
GAP LINE   crates/runner/src/exit_grid_policy.rs:4248  [production]  "excursion accumulator",
GAP LINE   crates/runner/src/exit_grid_policy.rs:4249  [production]  ));
GAP LINE   crates/runner/src/exit_grid_policy.rs:4271  [production]  return Err(ExitGridErrorV1::ArithmeticEnvelopeExceeded(
GAP LINE   crates/runner/src/exit_grid_policy.rs:4272  [production]  "resolved level price",
GAP LINE   crates/runner/src/exit_grid_policy.rs:4273  [production]  ));
GAP LINE   crates/runner/src/exit_grid_policy.rs:4606  [test code]  fn unreachable_policy() -> ExitGridPolicyV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4607  [test code]  ExitGridPolicyV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4608  [test code]  execution_resolution: ExecutionResolutionV1::OneMinuteOhlcv,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4609  [test code]  range_resolution: RangeResolutionV1::PpmCeiling,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4610  [test code]  side: crate::excursion::Side::Long,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4611  [test code]  rungs: unreachable_policy_rungs(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4612  [test code]  ratios: RatioLimitsV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4613  [test code]  min_hundredths: 100,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4614  [test code]  max_hundredths: 100,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4615  [test code]  max_pairs: 1,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4616  [test code]  },
GAP LINE   crates/runner/src/exit_grid_policy.rs:4617  [test code]  max_cells: 1,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4618  [test code]  selector: ExitGridSelectorV1::PessimisticTotal,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4619  [test code]  cost_model_id: printed_ohlcv_cost_model_id_v2(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4620  [test code]  forced_stop: ForcedStopV1::Disabled,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4621  [test code]  max_ambiguous_bars: 0,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4622  [test code]  max_gap_fills: 0,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4623  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:4624  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:4664  [test code]  return Column::default();
GAP LINE   crates/runner/src/exit_grid_policy.rs:4719  [test code]  fn unsupported_key() -> InstrumentKey {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4720  [test code]  InstrumentKey {
GAP LINE   crates/runner/src/exit_grid_policy.rs:4721  [test code]  exchange: Exchange::Nse,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4722  [test code]  segment: Segment::Cash,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4723  [test code]  underlying: symbol("X"),
GAP LINE   crates/runner/src/exit_grid_policy.rs:4724  [test code]  kind: Kind::Equity,
GAP LINE   crates/runner/src/exit_grid_policy.rs:4725  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:4726  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:4730  [test code]  return symbol("X");
GAP LINE   crates/runner/src/exit_grid_policy.rs:5021  [test code]  fn unreachable_expiry() -> Expiry {
GAP LINE   crates/runner/src/exit_grid_policy.rs:5022  [test code]  match Expiry::new(2026, 1, 1) {
GAP LINE   crates/runner/src/exit_grid_policy.rs:5023  [test code]  Ok(value) => value,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5024  [test code]  Err(_) => unreachable_expiry(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:5026  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:5115  [test code]  fn unreachable_resolved() -> ResolvedExitGridV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:5116  [test code]  let policy = unreachable_policy();
GAP LINE   crates/runner/src/exit_grid_policy.rs:5117  [test code]  let instrument = nifty();
GAP LINE   crates/runner/src/exit_grid_policy.rs:5118  [test code]  let first = REGULAR_OPEN_IST_MINUTE
GAP LINE   crates/runner/src/exit_grid_policy.rs:5119  [test code]  .saturating_mul(ONE_MINUTE_MICROS)
GAP LINE   crates/runner/src/exit_grid_policy.rs:5120  [test code]  .saturating_sub(indicators::IST_OFFSET_MICROS);
GAP LINE   crates/runner/src/exit_grid_policy.rs:5121  [test code]  ResolvedExitGridV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:5122  [test code]  instrument,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5123  [test code]  family: InstrumentFamilyV1::Nifty,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5124  [test code]  feed_digest: hash(b"test-feed"),
GAP LINE   crates/runner/src/exit_grid_policy.rs:5125  [test code]  commit_digest: hash(b"test-commit"),
GAP LINE   crates/runner/src/exit_grid_policy.rs:5126  [test code]  calendar_digest: [0xA5; 32],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5127  [test code]  policy_digest: policy.digest(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:5128  [test code]  training_digest: [1; 32],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5129  [test code]  training_bars: 1,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5130  [test code]  training_first_ts_micros: first,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5131  [test code]  training_last_ts_micros: first,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5132  [test code]  stop_levels_ppm: vec![1],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5133  [test code]  target_levels_ppm: vec![1],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5134  [test code]  trail_levels_ppm: vec![1],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5135  [test code]  ratio_pairs: vec![RatioPairV1 {
GAP LINE   crates/runner/src/exit_grid_policy.rs:5136  [test code]  stop_index: 0,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5137  [test code]  target_index: 0,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5138  [test code]  }],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5139  [test code]  ratio_bitmap: vec![true],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5140  [test code]  forced_stop_index: None,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5141  [test code]  cell_count: 1,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5142  [test code]  digest: [2; 32],
GAP LINE   crates/runner/src/exit_grid_policy.rs:5143  [test code]  policy,
GAP LINE   crates/runner/src/exit_grid_policy.rs:5144  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:5145  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:5252  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:5261  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:5968  [test code]  continue;
GAP LINE   crates/runner/src/exit_grid_policy.rs:6013  [test code]  stops: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:6014  [test code]  targets: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:6015  [test code]  trails: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:6016  [test code]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:6047  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:6053  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:6559  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7154  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7163  [test code]  )?;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7174  [test code]  )?,
GAP LINE   crates/runner/src/exit_grid_policy.rs:7231  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7236  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7246  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7259  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7354  [test code]  assert_eq!(
GAP LINE   crates/runner/src/exit_grid_policy.rs:7355  [test code]  resolved.replay_selected(oos, &execution_column, &selected, run),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7356  [test code]  Err(ExitGridErrorV1::RefusedExecutionPaths {
GAP LINE   crates/runner/src/exit_grid_policy.rs:7357  [test code]  paths: universe.pricing_refused_paths(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7358  [test code]  }),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7519  [test code]  stops: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7520  [test code]  targets: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7521  [test code]  trails: Ladder::new(vec![1]).unwrap_or_default(),
GAP LINE   crates/runner/src/exit_grid_policy.rs:7522  [test code]  });
GAP LINE   crates/runner/src/exit_grid_policy.rs:7569  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7582  [test code]  return;
GAP LINE   crates/runner/src/exit_grid_policy.rs:7597  [test code]  }
GAP LINE   crates/runner/src/exit_grid_policy.rs:7631  [test code]  return;
GAP BRANCH crates/runner/src/exit_grid_policy.rs:146:30 True  [production]  if numerator == 0 || denominator == 0 || numerator > denominator {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:146:50 True  [production]  if numerator == 0 || denominator == 0 || numerator > denominator {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:461:12 True  [production]  if run.feed.is_empty() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:464:12 True  [production]  if run.commit.is_empty() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:602:16 False  [production]  && let Some(window) = (offset / size)
GAP BRANCH crates/runner/src/exit_grid_policy.rs:605:12 False  [production]  && core::ptr::eq(window, evaluated_execution_1m)
GAP BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [production]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:676:16 False  [test code branch]  if let Some(last) = forged.last_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:1113:12 True  [production]  if stops.len() > self.rungs.max_levels_per_axis()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:1114:16 True  [production]  || targets.len() > self.rungs.max_levels_per_axis()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:1115:16 True  [production]  || trails.len() > self.rungs.max_levels_per_axis()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:1131:12 True  [production]  if pair_count > self.ratios.max_pairs() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:1292:16 True  [production]  || instrument.kind != Kind::Index
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2105:16 True  [production]  if candidate.occupied_through_bar < candidate.entry_bar
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2106:20 True  [production]  || candidate.occupied_through_micros < candidate.entry_micros
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2107:20 True  [production]  || prior.is_some_and(|(bar, stamp)| {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2108:21 True  [production]  candidate.entry_bar <= bar || candidate.entry_micros <= stamp
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2117:24 True  [production]  if price.ambiguous_bars > 1
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2118:28 True  [production]  || price.gap_fills > 1
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2119:28 True  [production]  || row.signal_bar != candidate.signal_bar
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2120:28 True  [production]  || row.entry_bar != candidate.entry_bar
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2121:28 True  [production]  || row.exit_bar != candidate.occupied_through_bar
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2122:28 True  [production]  || row.entry_micros != candidate.entry_micros
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2131:25 True  [production]  let Some(next) = pricing_refused.checked_add(1) else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2131:25 False  [production]  let Some(next) = pricing_refused.checked_add(1) else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2327:9 False  [production]  self.policy_digest == self.policy.digest() && self.digest == digest_resolved(self)
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2675:12 True  [production]  if !self.chosen_is_in_bounds(coordinate) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2679:12 True  [production]  if selected.resolution_digest != self.digest
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2680:16 True  [production]  || selected.run_id != evaluated.run_id
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2681:16 True  [production]  || selected.mask != evaluated.mask
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2682:16 True  [production]  || selected.horizon != evaluated.horizon
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2683:16 True  [production]  || selected.column_digest != evaluated.column_digest
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2684:16 True  [production]  || selected.evaluation_spec != evaluated.evaluation_spec
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2685:16 True  [production]  || selected.side != evaluated.side
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2686:16 True  [production]  || selected.coordinate != coordinate
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2687:16 True  [production]  || selected.training_cell != *cell
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2688:16 True  [production]  || selected.selection_digest
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2768:12 True  [production]  if universe.pricing_refused_paths != 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2774:17 True  [production]  && (value.ambiguous_bars > self.policy.max_ambiguous_bars()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2775:20 True  [production]  || value.gapped > self.policy.max_gap_fills())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2844:12 True  [production]  if selected.resolution_digest != self.digest
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2866:12 True  [production]  if oos_run.mask != selected.mask {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2877:12 True  [production]  if first_test_stamp <= self.training_last_ts_micros {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2883:12 True  [production]  if column.evaluation_spec_token() != Some(selected.evaluation_spec) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2889:12 True  [production]  if !self.chosen_is_in_bounds(selected.coordinate) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2895:14 True  [production]  ) && selected.coordinate.stop != self.forced_stop_index
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2895:14 False  [production]  ) && selected.coordinate.stop != self.forced_stop_index
GAP BRANCH crates/runner/src/exit_grid_policy.rs:2905:16 True  [production]  if !facts.covers(bars) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3028:58 True  [production]  if evaluated.resolution_digest != self.digest || evaluated.side != self.side() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3404:8 True  [production]  if values.len() > max {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3412:13 False  [production]  let Some(left) = pair.first() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3415:13 False  [production]  let Some(right) = pair.last() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3590:12 True  [production]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3590:45 True  [production]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3593:12 True  [production]  if bar.open_interest != indicators::OI_NULL && bar.open_interest < 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3593:56 True  [production]  if bar.open_interest != indicators::OI_NULL && bar.open_interest < 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3593:56 False  [production]  if bar.open_interest != indicators::OI_NULL && bar.open_interest < 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:3667:8 True  [production]  if out.is_empty() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4126:9 False  [production]  let Some(accepted) = column.acceptance() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4138:12 True  [production]  || !census.reconciles()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4139:12 True  [production]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4140:12 True  [production]  || census.refused() != 0
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4155:8 True  [production]  if column.bits().len() != column.sources().len() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4166:12 True  [production]  if source < first_permitted {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4172:12 True  [production]  if previous.is_some_and(|before| source <= before) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4231:8 True  [production]  if money_bound > i128::from(i64::MAX) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4246:8 True  [production]  if excursion_bound > i128::from(i64::MAX) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4268:8 True  [production]  if distance > i128::from(i64::MAX)
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4269:12 True  [production]  || i128::from(max_price).checked_add(distance) > Some(i128::from(i64::MAX))
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4663:13 False  [test code branch]  let Ok(widths) = Widths::pinned() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:4729:13 False  [test code branch]  let Ok(value) = Symbol::new(text) else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5208:16 False  [test code branch]  if let Some(bar) = changed.get_mut(50) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5251:13 False  [test code branch]  let Some(sealed) = sealed.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5260:13 False  [test code branch]  let Some(series) = series.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5655:16 False  [test code branch]  if let Some(level) = value.stop_levels_ppm.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5660:16 False  [test code branch]  if let Some(level) = value.target_levels_ppm.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5665:16 False  [test code branch]  if let Some(level) = value.trail_levels_ppm.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5670:16 False  [test code branch]  if let Some(pair) = value.ratio_pairs.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5675:16 False  [test code branch]  if let Some(admitted) = value.ratio_bitmap.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5768:16 False  [test code branch]  if let Some(second) = duplicate.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5776:16 False  [test code branch]  if let Some(second) = backward.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5784:16 False  [test code branch]  if let Some(second) = off_grid.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5797:16 False  [test code branch]  if let Some(bar) = gapped.get_mut(2) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5800:16 False  [test code branch]  if let Some(bar) = gapped.get_mut(3) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5850:16 False  [test code branch]  if let Some(bar) = corrupt.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5859:16 False  [test code branch]  if let Some(bar) = zero_open.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:5967:25 False  [test code branch]  let Some(subsets) = subsets else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6046:13 False  [test code branch]  let Some(mut evaluated) = evaluated.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6052:13 False  [test code branch]  let Some(validated) = validated.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6137:49 False  [test code branch]  .find(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6144:26 False  [test code branch]  .find(|cell| cell.stop.is_some() && cell.target.is_none() && cell.ttp.is_none())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6151:28 False  [test code branch]  .filter(|cell| cell.stop.is_some() && cell.target.is_some() && cell.ttp.is_none())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6151:51 False  [test code branch]  .filter(|cell| cell.stop.is_some() && cell.target.is_some() && cell.ttp.is_none())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6285:51 False  [test code branch]  .filter(|cell| cell.stop.is_none() && cell.target.is_some() && cell.ttp.is_none())
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6366:24 False  [test code branch]  && cell.target.is_some()
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6558:13 False  [test code branch]  let Some(evaluated) = evaluated_result.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6569:16 False  [test code branch]  if let Some(bar) = changed.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6902:16 False  [test code branch]  if let Some(bar) = changed.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:6996:16 False  [test code branch]  if let Some(bar) = changed.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7073:16 False  [test code branch]  if let Some(bar) = changed.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7153:13 False  [test code branch]  let Some(selected) = selected else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7186:16 False  [test code branch]  if let Some(bar) = off_minute.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7196:16 False  [test code branch]  if let Some(bar) = duplicate.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7205:16 False  [test code branch]  if let Some(bar) = corrupt.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7214:16 False  [test code branch]  if let Some(bar) = whole_minute_gap.get_mut(2) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7230:13 False  [test code branch]  let Some(series) = series.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7235:13 False  [test code branch]  let Some(oos) = oos.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7245:13 False  [test code branch]  let Some(run) = run.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7258:13 False  [test code branch]  let Some(changed_spec) = changed_spec else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7353:12 True  [test code branch]  if universe.pricing_refused_paths() > 0 {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7539:16 False  [test code branch]  if let Some(slot) = cells.first_mut() {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7544:16 False  [test code branch]  if let Some(slot) = cells.get_mut(1) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7568:13 False  [test code branch]  let Some(mut evaluated) = evaluated.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7581:13 False  [test code branch]  let Some(validated) = validated.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7592:16 False  [test code branch]  if let Some(rejected_coordinate) = rejected_coordinate {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7613:16 False  [test code branch]  if let (Some(first), Some(second)) = (first_coordinate, duplicated.grid.cells.get_mut(1)) {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7630:13 False  [test code branch]  let Some(mut wrong_offset) = wrong_offset.ok() else {
GAP BRANCH crates/runner/src/exit_grid_policy.rs:7633:16 False  [test code branch]  if let Some(Some(offset)) = wrong_offset.coordinate_row_offsets.first_mut() {
GAP LINE   crates/runner/src/expression.rs:292  [test code]  calls += 1;
GAP LINE   crates/runner/src/expression.rs:293  [test code]  Ok::<_, ()>(())
GAP LINE   crates/runner/src/expression.rs:294  [test code]  };
GAP BRANCH crates/runner/src/expression.rs:98:38 True  [production]  if truth.len() != known.len() || truth.len() != sources.len() {
GAP LINE   crates/runner/src/expression_execution.rs:53  [production]  pub fn new_with_daily_reference(
GAP LINE   crates/runner/src/expression_execution.rs:54  [production]  run: &Run<'_>,
GAP LINE   crates/runner/src/expression_execution.rs:55  [production]  program: &Expression,
GAP LINE   crates/runner/src/expression_execution.rs:56  [production]  signal: &[Candle],
GAP LINE   crates/runner/src/expression_execution.rs:57  [production]  minute_context: &[Candle],
GAP LINE   crates/runner/src/expression_execution.rs:58  [production]  execution: &[Candle],
GAP LINE   crates/runner/src/expression_execution.rs:59  [production]  reference: DailyReferenceBinding<'_>,
GAP LINE   crates/runner/src/expression_execution.rs:60  [production]  ) -> Result<Self, String> {
GAP LINE   crates/runner/src/expression_execution.rs:61  [production]  let source = ExecutionRunV1::new_with_daily_reference(
GAP LINE   crates/runner/src/expression_execution.rs:62  [production]  run,
GAP LINE   crates/runner/src/expression_execution.rs:63  [production]  signal,
GAP LINE   crates/runner/src/expression_execution.rs:64  [production]  minute_context,
GAP LINE   crates/runner/src/expression_execution.rs:65  [production]  execution,
GAP LINE   crates/runner/src/expression_execution.rs:66  [production]  reference,
GAP LINE   crates/runner/src/expression_execution.rs:68  [production]  .map_err(display)?;
GAP LINE   crates/runner/src/expression_execution.rs:69  [production]  Self::seal(run, program, source)
GAP LINE   crates/runner/src/expression_execution.rs:70  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:79  [production]  pub fn with_digests(
GAP LINE   crates/runner/src/expression_execution.rs:80  [production]  run: &Run<'_>,
GAP LINE   crates/runner/src/expression_execution.rs:81  [production]  program: &Expression,
GAP LINE   crates/runner/src/expression_execution.rs:82  [production]  digests: &crate::exit_grid_policy::ExecutionDigestsV1,
GAP LINE   crates/runner/src/expression_execution.rs:83  [production]  ) -> Result<Self, String> {
GAP LINE   crates/runner/src/expression_execution.rs:84  [production]  let source = ExecutionRunV1::with_digests(run, digests).map_err(display)?;
GAP LINE   crates/runner/src/expression_execution.rs:85  [production]  Self::seal(run, program, source)
GAP LINE   crates/runner/src/expression_execution.rs:86  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:115  [production]  pub const fn execution_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/expression_execution.rs:116  [production]  self.source.execution_digest()
GAP LINE   crates/runner/src/expression_execution.rs:117  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:149  [production]  pub const fn resolution_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/expression_execution.rs:150  [production]  self.resolution
GAP LINE   crates/runner/src/expression_execution.rs:151  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:161  [production]  pub const fn horizon(&self) -> Horizon {
GAP LINE   crates/runner/src/expression_execution.rs:162  [production]  self.horizon
GAP LINE   crates/runner/src/expression_execution.rs:163  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:167  [production]  pub const fn side(&self) -> crate::excursion::Side {
GAP LINE   crates/runner/src/expression_execution.rs:168  [production]  self.side
GAP LINE   crates/runner/src/expression_execution.rs:169  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:173  [production]  pub const fn column_digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/expression_execution.rs:174  [production]  self.column_digest
GAP LINE   crates/runner/src/expression_execution.rs:175  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:212  [production]  pub const fn evaluation(&self) -> &EvaluatedExpressionGridV1 {
GAP LINE   crates/runner/src/expression_execution.rs:213  [production]  self.evaluated
GAP LINE   crates/runner/src/expression_execution.rs:214  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:249  [production]  pub const fn coordinate(&self) -> Chosen {
GAP LINE   crates/runner/src/expression_execution.rs:250  [production]  self.coordinate
GAP LINE   crates/runner/src/expression_execution.rs:251  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:255  [production]  pub const fn digest(&self) -> [u8; 32] {
GAP LINE   crates/runner/src/expression_execution.rs:256  [production]  self.identity
GAP LINE   crates/runner/src/expression_execution.rs:257  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:293  [production]  pub const fn training_cell(&self) -> &Cell {
GAP LINE   crates/runner/src/expression_execution.rs:294  [production]  &self.cell
GAP LINE   crates/runner/src/expression_execution.rs:295  [production]  }
GAP LINE   crates/runner/src/expression_execution.rs:336  [production]  )?;
GAP LINE   crates/runner/src/expression_execution.rs:358  [production]  return Err("expression exact truth summary differs from grid signals".to_owned());
GAP LINE   crates/runner/src/expression_execution.rs:419  [production]  return Err("expression coordinate ordinal changed".to_owned());
GAP LINE   crates/runner/src/expression_execution.rs:467  [production]  return Err(
GAP LINE   crates/runner/src/expression_execution.rs:468  [production]  "expression replay does not match authenticated training inputs".to_owned(),
GAP LINE   crates/runner/src/expression_execution.rs:469  [production]  );
GAP BRANCH crates/runner/src/expression_execution.rs:357:12 True  [production]  if summary.hits != grid.signals {
GAP BRANCH crates/runner/src/expression_execution.rs:383:51 True  [production]  if evaluated.resolution != self.digest || evaluated.side != self.side() {
GAP BRANCH crates/runner/src/expression_execution.rs:404:12 True  [production]  if validated.evaluated.resolution != self.digest
GAP BRANCH crates/runner/src/expression_execution.rs:418:12 True  [production]  if Chosen::from_cell(cell) != coordinate {
GAP BRANCH crates/runner/src/expression_execution.rs:461:12 True  [production]  if evaluation.resolution != self.digest
GAP BRANCH crates/runner/src/expression_execution.rs:462:16 True  [production]  || attested.resolution_digest != self.digest
GAP BRANCH crates/runner/src/expression_execution.rs:463:16 True  [production]  || attested.column_digest != evaluation.column_digest
GAP BRANCH crates/runner/src/expression_execution.rs:464:16 True  [production]  || attested.horizon != evaluation.horizon
GAP BRANCH crates/runner/src/expression_execution.rs:465:16 True  [production]  || attested.evaluation_spec != evaluation.evaluation_spec
GAP LINE   crates/runner/src/expression_execution_tests.rs:155  [test code]  )?;
GAP LINE   crates/runner/src/expression_execution_tests.rs:197  [test code]  )?;
GAP LINE   crates/runner/src/expression_execution_tests.rs:212  [test code]  return Err("pair".to_owned());
GAP LINE   crates/runner/src/expression_execution_tests.rs:224  [test code]  )?;
GAP LINE   crates/runner/src/expression_execution_tests.rs:291  [test code]  )?;
GAP BRANCH crates/runner/src/expression_execution_tests.rs:211:13 False  [test code branch]  let [left, right] = pair else {
GAP LINE   crates/runner/src/expression_oos.rs:299  [production]  )?;
GAP LINE   crates/runner/src/expression_oos.rs:303  [production]  return Err("later expression truth/grid signals differ".into());
GAP BRANCH crates/runner/src/expression_oos.rs:302:12 True  [production]  if summary.hits != grid.signals {
GAP LINE   crates/runner/src/expression_oos_tests.rs:97  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:106  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:182  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:271  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:280  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:317  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:354  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:435  [test code]  )?;
GAP LINE   crates/runner/src/expression_oos_tests.rs:445  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation.rs:325  [production]  )?;
GAP LINE   crates/runner/src/expression_validation.rs:349  [production]  return Err("fixed-training fold totals disagree with exact later cell".into());
GAP LINE   crates/runner/src/expression_validation.rs:523  [production]  return Err("fixed-training trade crosses source/fold/session boundaries".into());
GAP LINE   crates/runner/src/expression_validation.rs:584  [production]  )?;
GAP LINE   crates/runner/src/expression_validation.rs:621  [production]  fn display(why: impl std::fmt::Display) -> String {
GAP LINE   crates/runner/src/expression_validation.rs:622  [production]  why.to_string()
GAP LINE   crates/runner/src/expression_validation.rs:623  [production]  }
GAP BRANCH crates/runner/src/expression_validation.rs:76:48 True  [production]  if anchor.digest() != anchor.seal() || resolved.digest() != anchor.resolution_digest() {
GAP BRANCH crates/runner/src/expression_validation.rs:168:16 True  [production]  || !std::ptr::eq(map.column, later.column)
GAP BRANCH crates/runner/src/expression_validation.rs:183:43 True  [production]  if later.anchor != self.anchor || later.side != self.side {
GAP BRANCH crates/runner/src/expression_validation.rs:270:12 False  [production]  if column.accepts(index) && last_accepted != Some(actual) {
GAP BRANCH crates/runner/src/expression_validation.rs:303:16 True  [production]  || selected.run_id() != anchor.run_id()
GAP BRANCH crates/runner/src/expression_validation.rs:304:16 True  [production]  || selected.resolution != anchor.resolution
GAP BRANCH crates/runner/src/expression_validation.rs:305:16 True  [production]  || selected.evaluation != anchor.evaluation
GAP BRANCH crates/runner/src/expression_validation.rs:345:12 True  [production]  if totals.trades != cell.trades
GAP BRANCH crates/runner/src/expression_validation.rs:346:16 True  [production]  || totals.wins != cell.wins
GAP BRANCH crates/runner/src/expression_validation.rs:347:16 True  [production]  || totals.return_paisa != cell.pessimistic
GAP BRANCH crates/runner/src/expression_validation.rs:516:12 True  [production]  if first != last
GAP BRANCH crates/runner/src/expression_validation.rs:517:16 True  [production]  || trade.entry_micros != entry.ts_micros
GAP BRANCH crates/runner/src/expression_validation.rs:518:16 True  [production]  || trade.exit_micros != exit.ts_micros
GAP BRANCH crates/runner/src/expression_validation.rs:519:16 True  [production]  || day(entry.ts_micros) != day(exit.ts_micros)
GAP BRANCH crates/runner/src/expression_validation.rs:520:16 True  [production]  || trade.exit_bar < trade.entry_bar
GAP BRANCH crates/runner/src/expression_validation.rs:521:16 True  [production]  || previous.is_some_and(|index| trade.entry_bar <= index)
GAP BRANCH crates/runner/src/expression_validation.rs:559:16 True  [production]  || window.first > window.last
GAP BRANCH crates/runner/src/expression_validation.rs:587:12 True  [production]  if fold.sessions == 0
GAP BRANCH crates/runner/src/expression_validation.rs:590:16 True  [production]  || fold.wins > fold.trades
GAP BRANCH crates/runner/src/expression_validation.rs:592:35 True  [production]  || (fold.wins == 0 && fold.return_paisa > 0)
GAP BRANCH crates/runner/src/expression_validation.rs:593:64 True  [production]  || (fold.wins == fold.trades && fold.trades > 0 && fold.return_paisa <= 0)
GAP LINE   crates/runner/src/expression_validation_tests.rs:117  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:125  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:136  [test code]  }
GAP LINE   crates/runner/src/expression_validation_tests.rs:267  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:296  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:359  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:365  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:373  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:379  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:414  [test code]  }
GAP LINE   crates/runner/src/expression_validation_tests.rs:492  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:501  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:610  [test code]  )?;
GAP LINE   crates/runner/src/expression_validation_tests.rs:618  [test code]  )?;
GAP BRANCH crates/runner/src/expression_validation_tests.rs:133:16 False  [test code branch]  if let Some(selected) = disposition.selected() {
GAP BRANCH crates/runner/src/expression_validation_tests.rs:393:20 False  [test code branch]  if let Some((selected, wrong)) = disposition.selected().zip(foreign.selected()) {
GAP LINE   crates/runner/src/grid.rs:1966  [production]  pub fn evaluate_with(
GAP LINE   crates/runner/src/grid.rs:1967  [production]  bars: &[Candle],
GAP LINE   crates/runner/src/grid.rs:1968  [production]  column: &Column,
GAP LINE   crates/runner/src/grid.rs:1969  [production]  mask: &ConditionMask,
GAP LINE   crates/runner/src/grid.rs:1970  [production]  horizon: Horizon,
GAP LINE   crates/runner/src/grid.rs:1971  [production]  side: Side,
GAP LINE   crates/runner/src/grid.rs:1972  [production]  levels: Levels<'_>,
GAP LINE   crates/runner/src/grid.rs:1973  [production]  _exits: Option<&[Option<crate::trade::SquareOff>]>,
GAP LINE   crates/runner/src/grid.rs:1974  [production]  ) -> Grid {
GAP LINE   crates/runner/src/grid.rs:1975  [production]  let facts = crate::trade::SliceFacts::of(bars, column);
GAP LINE   crates/runner/src/grid.rs:1976  [production]  evaluate_over(bars, column, mask, horizon, side, levels, &facts)
GAP LINE   crates/runner/src/grid.rs:1977  [production]  }
GAP LINE   crates/runner/src/grid.rs:2095  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:2225  [production]  Ladder::from_excursions(&mut observed.to_vec(), rungs).unwrap_or_default()
GAP LINE   crates/runner/src/grid.rs:2226  [production]  }),
GAP LINE   crates/runner/src/grid.rs:2234  [production]  Some(_) => merged(&ladder_of(&adverse), forced),
GAP LINE   crates/runner/src/grid.rs:2384  [production]  return envelope_refused(timed);
GAP LINE   crates/runner/src/grid.rs:2673  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:2700  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")
GAP LINE   crates/runner/src/grid.rs:2701  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:2703  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:2704  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayCellCountMismatch {
GAP LINE   crates/runner/src/grid.rs:2705  [production]  expected: resolved.cell_count(),
GAP LINE   crates/runner/src/grid.rs:2706  [production]  actual,
GAP LINE   crates/runner/src/grid.rs:2707  [production]  },
GAP LINE   crates/runner/src/grid.rs:2708  [production]  );
GAP LINE   crates/runner/src/grid.rs:2722  [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")
GAP LINE   crates/runner/src/grid.rs:2723  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:2878  [production]  return Ok(Vec::new());
GAP LINE   crates/runner/src/grid.rs:3088  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3203  [production]  pub const fn row(&self) -> TradeRow {
GAP LINE   crates/runner/src/grid.rs:3204  [production]  self.row
GAP LINE   crates/runner/src/grid.rs:3205  [production]  }
GAP LINE   crates/runner/src/grid.rs:3209  [production]  pub const fn ambiguous_bars(&self) -> u64 {
GAP LINE   crates/runner/src/grid.rs:3210  [production]  self.ambiguous_bars
GAP LINE   crates/runner/src/grid.rs:3211  [production]  }
GAP LINE   crates/runner/src/grid.rs:3215  [production]  pub const fn gap_fills(&self) -> u64 {
GAP LINE   crates/runner/src/grid.rs:3216  [production]  self.gap_fills
GAP LINE   crates/runner/src/grid.rs:3217  [production]  }
GAP LINE   crates/runner/src/grid.rs:3269  [production]  pub const fn signal_bar(&self) -> usize {
GAP LINE   crates/runner/src/grid.rs:3270  [production]  self.signal_bar
GAP LINE   crates/runner/src/grid.rs:3271  [production]  }
GAP LINE   crates/runner/src/grid.rs:3287  [production]  pub const fn signal_micros(&self) -> i64 {
GAP LINE   crates/runner/src/grid.rs:3288  [production]  self.signal_micros
GAP LINE   crates/runner/src/grid.rs:3289  [production]  }
GAP LINE   crates/runner/src/grid.rs:3376  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3377  [production]  "candidate count does not fit the V1 receipt",
GAP LINE   crates/runner/src/grid.rs:3378  [production]  )
GAP LINE   crates/runner/src/grid.rs:3379  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:3386  [production]  requested_paths,
GAP LINE   crates/runner/src/grid.rs:3387  [production]  },
GAP LINE   crates/runner/src/grid.rs:3388  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3433  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3434  [production]  "candidate count does not fit the V1 receipt",
GAP LINE   crates/runner/src/grid.rs:3435  [production]  )
GAP LINE   crates/runner/src/grid.rs:3436  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:3439  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
GAP LINE   crates/runner/src/grid.rs:3440  [production]  requested_paths,
GAP LINE   crates/runner/src/grid.rs:3441  [production]  }
GAP LINE   crates/runner/src/grid.rs:3442  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:3445  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceAllocationRefused {
GAP LINE   crates/runner/src/grid.rs:3446  [production]  requested_paths: 1,
GAP LINE   crates/runner/src/grid.rs:3447  [production]  }
GAP LINE   crates/runner/src/grid.rs:3448  [production]  })?;
GAP LINE   crates/runner/src/grid.rs:3457  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3458  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3459  [production]  "candidate entries are duplicated or out of order",
GAP LINE   crates/runner/src/grid.rs:3460  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3461  [production]  );
GAP LINE   crates/runner/src/grid.rs:3473  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3478  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3507  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3512  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3546  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3563  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3565  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3566  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3567  [production]  "block-only occupancy ends before its entry",
GAP LINE   crates/runner/src/grid.rs:3568  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3569  [production]  );
GAP LINE   crates/runner/src/grid.rs:3581  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3582  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3583  [production]  "pricing-refusal branch carried no refusal fact",
GAP LINE   crates/runner/src/grid.rs:3584  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3585  [production]  );
GAP LINE   crates/runner/src/grid.rs:3614  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3615  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3616  [production]  "a priceable candidate did not replay to exactly one row",
GAP LINE   crates/runner/src/grid.rs:3617  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3618  [production]  );
GAP LINE   crates/runner/src/grid.rs:3621  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3622  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3623  [production]  "one candidate produced impossible multi-trade quality counts",
GAP LINE   crates/runner/src/grid.rs:3624  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3625  [production]  );
GAP LINE   crates/runner/src/grid.rs:3631  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3636  [production]  )?;
GAP LINE   crates/runner/src/grid.rs:3645  [production]  return Err(
GAP LINE   crates/runner/src/grid.rs:3646  [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
GAP LINE   crates/runner/src/grid.rs:3647  [production]  "chosen candidate row does not reconcile with its occupancy path",
GAP LINE   crates/runner/src/grid.rs:3648  [production]  ),
GAP LINE   crates/runner/src/grid.rs:3649  [production]  );
GAP LINE   crates/runner/src/grid.rs:3711  [production]  pub fn with_levels_over(
GAP LINE   crates/runner/src/grid.rs:3712  [production]  bars: &[Candle],
GAP LINE   crates/runner/src/grid.rs:3713  [production]  column: &Column,
GAP LINE   crates/runner/src/grid.rs:3714  [production]  mask: &ConditionMask,
GAP LINE   crates/runner/src/grid.rs:3715  [production]  horizon: Horizon,
GAP LINE   crates/runner/src/grid.rs:3716  [production]  side: Side,
GAP LINE   crates/runner/src/grid.rs:3717  [production]  ladders: Ladders<'_>,
GAP LINE   crates/runner/src/grid.rs:3718  [production]  variant: Chosen,
GAP LINE   crates/runner/src/grid.rs:3719  [production]  facts: &crate::trade::SliceFacts,
GAP LINE   crates/runner/src/grid.rs:3720  [production]  ) -> Option<Cell> {
GAP LINE   crates/runner/src/grid.rs:3721  [production]  levelled_over(
GAP LINE   crates/runner/src/grid.rs:3722  [production]  bars, column, mask, horizon, side, ladders, variant, None, facts,
GAP LINE   crates/runner/src/grid.rs:3723  [production]  )
GAP LINE   crates/runner/src/grid.rs:3724  [production]  .cell
GAP LINE   crates/runner/src/grid.rs:3725  [production]  }
GAP LINE   crates/runner/src/grid.rs:4921  [production]  return Priced {
GAP LINE   crates/runner/src/grid.rs:4922  [production]  paisa: 0,
GAP LINE   crates/runner/src/grid.rs:4923  [production]  gapped: false,
GAP LINE   crates/runner/src/grid.rs:4924  [production]  };
GAP LINE   crates/runner/src/grid.rs:5003  [production]  return Priced {
GAP LINE   crates/runner/src/grid.rs:5004  [production]  paisa: 0,
GAP LINE   crates/runner/src/grid.rs:5005  [production]  gapped: false,
GAP LINE   crates/runner/src/grid.rs:5006  [production]  };
GAP LINE   crates/runner/src/grid.rs:5145  [production]  None => 0,
GAP LINE   crates/runner/src/grid.rs:5266  [production]  core::cmp::Ordering::Greater => live,
GAP LINE   crates/runner/src/grid.rs:5316  [production]  None => None,
GAP LINE   crates/runner/src/grid.rs:5323  [production]  None => None,
GAP LINE   crates/runner/src/grid.rs:5485  [production]  }
GAP LINE   crates/runner/src/grid.rs:5486  [production]  (bar.open, true)
GAP LINE   crates/runner/src/grid.rs:6310  [test code]  assert_eq!(
GAP LINE   crates/runner/src/grid.rs:6311  [test code]  super::materialize_cell(&bars, &column, &mask, horizon, side, &grid, cell),
GAP LINE   crates/runner/src/grid.rs:6312  [test code]  super::materialize_cell_over(
GAP LINE   crates/runner/src/grid.rs:6313  [test code]  &bars, &column, &mask, horizon, side, &grid, cell, &facts
GAP LINE   crates/runner/src/grid.rs:6316  [test code]  let ladders = crate::excursion::Ladders {
GAP LINE   crates/runner/src/grid.rs:6317  [test code]  stops: &grid.stops,
GAP LINE   crates/runner/src/grid.rs:6318  [test code]  targets: &grid.targets,
GAP LINE   crates/runner/src/grid.rs:6319  [test code]  trails: &grid.trails,
GAP LINE   crates/runner/src/grid.rs:6320  [test code]  };
GAP LINE   crates/runner/src/grid.rs:6321  [test code]  let chosen = super::Chosen::from_cell(cell);
GAP LINE   crates/runner/src/grid.rs:6322  [test code]  assert_eq!(
GAP LINE   crates/runner/src/grid.rs:6323  [test code]  super::per_trade(&bars, &column, &mask, horizon, side, ladders, chosen),
GAP LINE   crates/runner/src/grid.rs:6324  [test code]  super::per_trade_over(
GAP LINE   crates/runner/src/grid.rs:6325  [test code]  &bars, &column, &mask, horizon, side, ladders, chosen, &facts
GAP LINE   crates/runner/src/grid.rs:7276  [test code]  continue;
GAP LINE   crates/runner/src/grid.rs:8060  [test code]  }
GAP LINE   crates/runner/src/grid.rs:8069  [test code]  }
GAP LINE   crates/runner/src/grid.rs:8077  [test code]  }
GAP LINE   crates/runner/src/grid.rs:10134  [test code]  }
GAP LINE   crates/runner/src/grid.rs:10690  [test code]  (Some(t), Some(ttp)) => ttp.arm < t,
GAP LINE   crates/runner/src/grid.rs:11010  [production]  }
GAP LINE   crates/runner/src/grid.rs:11013  [production]  let s1 = stops.saturating_add(1).max(1);
GAP LINE   crates/runner/src/grid.rs:11014  [production]  let r_term = trails.saturating_add(1).saturating_mul(trails).max(2) / 2;
GAP LINE   crates/runner/src/grid.rs:11015  [production]  let denom = s1.saturating_mul(r_term).max(1);
GAP LINE   crates/runner/src/grid.rs:11016  [production]  let want = (BUDGET / denom).max(1);
GAP LINE   crates/runner/src/grid.rs:11025  [production]  let mut keep = 2_usize;
GAP LINE   crates/runner/src/grid.rs:11026  [production]  while keep < have {
GAP LINE   crates/runner/src/grid.rs:11027  [production]  let next = keep.saturating_add(1);
GAP LINE   crates/runner/src/grid.rs:11028  [production]  if next.saturating_mul(next.saturating_add(1)) / 2 > want {
GAP LINE   crates/runner/src/grid.rs:11029  [production]  break;
GAP LINE   crates/runner/src/grid.rs:11030  [production]  }
GAP LINE   crates/runner/src/grid.rs:11031  [production]  keep = next;
GAP LINE   crates/runner/src/grid.rs:11033  [production]  if keep >= have {
GAP LINE   crates/runner/src/grid.rs:11034  [production]  return targets;
GAP LINE   crates/runner/src/grid.rs:11035  [production]  }
GAP LINE   crates/runner/src/grid.rs:11036  [production]  let stride = have.div_ceil(keep).max(1);
GAP LINE   crates/runner/src/grid.rs:11037  [production]  let thinned: Vec<Ppm> = targets.rungs().iter().step_by(stride).copied().collect();
GAP LINE   crates/runner/src/grid.rs:11038  [production]  Ladder::new(thinned).unwrap_or(targets)
GAP BRANCH crates/runner/src/grid.rs:688:9 False  [production]  self.trades > 0
GAP BRANCH crates/runner/src/grid.rs:1733:17 True  [production]  let at = if at > last { last } else { at };
GAP BRANCH crates/runner/src/grid.rs:2383:8 True  [production]  if !money_envelope_fits(bars, &timed.occupancy, facts.acceptance()) {
GAP BRANCH crates/runner/src/grid.rs:2702:8 True  [production]  if actual != resolved.cell_count() {
GAP BRANCH crates/runner/src/grid.rs:2877:9 False  [production]  let Some(selected) = grid.best() else {
GAP BRANCH crates/runner/src/grid.rs:3454:12 True  [production]  if previous_entry
GAP BRANCH crates/runner/src/grid.rs:3455:41 True  [production]  .is_some_and(|(bar, stamp)| candidate.entry <= bar || entry_micros <= stamp)
GAP BRANCH crates/runner/src/grid.rs:3564:8 True  [production]  if candidate.time_exit < candidate.entry || stamp < entry_micros {
GAP BRANCH crates/runner/src/grid.rs:3564:49 True  [production]  if candidate.time_exit < candidate.entry || stamp < entry_micros {
GAP BRANCH crates/runner/src/grid.rs:3613:8 True  [production]  if measured.trades != 1 || one_row.len() != 1 {
GAP BRANCH crates/runner/src/grid.rs:3613:32 True  [production]  if measured.trades != 1 || one_row.len() != 1 {
GAP BRANCH crates/runner/src/grid.rs:3620:8 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
GAP BRANCH crates/runner/src/grid.rs:3620:39 True  [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
GAP BRANCH crates/runner/src/grid.rs:3637:8 True  [production]  if row.signal_bar != candidate.signal
GAP BRANCH crates/runner/src/grid.rs:3638:12 True  [production]  || row.entry_bar != candidate.entry
GAP BRANCH crates/runner/src/grid.rs:3639:12 True  [production]  || row.entry_micros != entry_micros
GAP BRANCH crates/runner/src/grid.rs:3640:12 True  [production]  || row.exit_micros != stamp
GAP BRANCH crates/runner/src/grid.rs:3641:12 True  [production]  || row.exit_bar < candidate.entry
GAP BRANCH crates/runner/src/grid.rs:3642:12 True  [production]  || row.exit_bar > candidate.time_exit
GAP BRANCH crates/runner/src/grid.rs:3643:12 True  [production]  || stamp < entry_micros
GAP BRANCH crates/runner/src/grid.rs:4748:12 False  [production]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {
GAP BRANCH crates/runner/src/grid.rs:4915:17 False  [production]  let Some(bar) = bars.get(entry.saturating_add(offset)) else {
GAP BRANCH crates/runner/src/grid.rs:4990:14 False  [production]  ) if anchor > 0 => {
GAP BRANCH crates/runner/src/grid.rs:5002:17 False  [production]  let Some(bar) = bars.get(entry.saturating_add(offset)) else {
GAP BRANCH crates/runner/src/grid.rs:5483:8 False  [production]  if resting >= bar.low && resting <= bar.high {
GAP BRANCH crates/runner/src/grid.rs:5483:30 False  [production]  if resting >= bar.low && resting <= bar.high {
GAP BRANCH crates/runner/src/grid.rs:7275:17 False  [test code branch]  let Some(slot) = holed.get_mut(hole) else {
GAP BRANCH crates/runner/src/grid.rs:7869:25 False  [test code branch]  .find(|row| row.exit_bar > row.entry_bar && row.exit_bar + 2 < row.entry_bar + 30)
GAP BRANCH crates/runner/src/grid.rs:8053:16 False  [test code branch]  if c.pessimistic < 0 {
GAP BRANCH crates/runner/src/grid.rs:8064:16 False  [test code branch]  if c.trades > 0 {
GAP? BRANCH crates/runner/src/grid.rs:8066:43 True  [assert! condition branch, side True (inside a closure)]  c.worst_trade <= 0 || c.pessimistic > 0,
GAP? BRANCH crates/runner/src/grid.rs:8066:43 False  [assert! condition branch, side False (inside a closure)]  c.worst_trade <= 0 || c.pessimistic > 0,
GAP BRANCH crates/runner/src/grid.rs:8071:16 False  [test code branch]  if c.pessimistic <= 0 {
GAP BRANCH crates/runner/src/grid.rs:8596:16 False  [test code branch]  if let Some(bar) = bars.get_mut(victim) {
GAP? BRANCH crates/runner/src/grid.rs:10079:43 True  [assert! condition branch, side True (inside a closure)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
GAP? BRANCH crates/runner/src/grid.rs:10079:43 False  [assert! condition branch, side False (inside a closure)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
GAP BRANCH crates/runner/src/grid.rs:10128:12 False  [test code branch]  if !g.cells.is_empty() {
GAP BRANCH crates/runner/src/grid.rs:10434:28 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP BRANCH crates/runner/src/grid.rs:10441:28 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP? BRANCH crates/runner/src/grid.rs:10490:36 False  [assert! condition branch, side False (inside a closure)]  .all(|row| victim < row.entry_bar || victim > row.exit_bar),
GAP? BRANCH crates/runner/src/grid.rs:10911:63 False  [assert! condition branch, side False (inside a closure)]  .all(|pair| matches!(pair, [first, second] if second.entry_bar > first.exit_bar)),
GAP BRANCH crates/runner/src/grid.rs:11008:8 False  [production]  if have <= 2 || variants(stops, have, trails) <= BUDGET {
GAP BRANCH crates/runner/src/grid.rs:11008:21 True  [production]  if have <= 2 || variants(stops, have, trails) <= BUDGET {
GAP BRANCH crates/runner/src/grid.rs:11008:21 False  [production]  if have <= 2 || variants(stops, have, trails) <= BUDGET {
GAP BRANCH crates/runner/src/grid.rs:11026:11 True  [production]  while keep < have {
GAP BRANCH crates/runner/src/grid.rs:11026:11 False  [production]  while keep < have {
GAP BRANCH crates/runner/src/grid.rs:11028:12 True  [production]  if next.saturating_mul(next.saturating_add(1)) / 2 > want {
GAP BRANCH crates/runner/src/grid.rs:11028:12 False  [production]  if next.saturating_mul(next.saturating_add(1)) / 2 > want {
GAP BRANCH crates/runner/src/grid.rs:11033:8 True  [production]  if keep >= have {
GAP BRANCH crates/runner/src/grid.rs:11033:8 False  [production]  if keep >= have {
GAP? BRANCH crates/runner/src/identity.rs:1462:26 False  [assert! condition branch, side False (inside a closure)]  .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
GAP LINE   crates/runner/src/lib.rs:473  [production]  closure_complete = false;
GAP LINE   crates/runner/src/lib.rs:474  [production]  (std::collections::HashSet::with_capacity(0), false)
GAP LINE   crates/runner/src/lib.rs:480  [production]  ClosureVerdict::Unknown
GAP LINE   crates/runner/src/lib.rs:1154  [production]  return Err(why);
GAP BRANCH crates/runner/src/lib.rs:470:50 False  [production]  let (redundant_here, known) = if let Some(next) = next {
GAP BRANCH crates/runner/src/lib.rs:479:34 True  [production]  let closure = if !known {
GAP BRANCH crates/runner/src/lib.rs:582:12 True  [production]  || sweep.bars != column.census().swept
GAP BRANCH crates/runner/src/lib.rs:623:9 False  [production]  u64::from(level.k) == u64::try_from(offset).unwrap_or(u64::MAX).saturating_add(1)
GAP BRANCH crates/runner/src/lib.rs:626:5 False  [production]  sequence
GAP BRANCH crates/runner/src/lib.rs:698:9 False  [production]  self.outcome.is_complete() && self.considered == self.redundant.saturating_add(self.closed)
GAP BRANCH crates/runner/src/lib.rs:1105:24 False  [production]  if found {
GAP BRANCH crates/runner/src/lib.rs:1144:16 False  [production]  if failure.borrow().is_none() {
GAP BRANCH crates/runner/src/lib.rs:1153:16 True  [production]  if let Some(why) = failure.into_inner() {
GAP? BRANCH crates/runner/src/lib.rs:2032:33 False  [assert! condition branch, side False (inside a closure)]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
GAP LINE   crates/runner/src/outcome.rs:313  [production]  (Some(a), Some(b)) => a.day == b.day,
GAP LINE   crates/runner/src/outcome.rs:963  [production]  lanes.absent(Unpriced::Refused);
GAP LINE   crates/runner/src/outcome.rs:964  [production]  continue;
GAP LINE   crates/runner/src/outcome.rs:1055  [production]  lanes.absent(Unpriced::Refused);
GAP LINE   crates/runner/src/outcome.rs:1056  [production]  continue;
GAP LINE   crates/runner/src/outcome.rs:1435  [production]  return 0;
GAP LINE   crates/runner/src/outcome.rs:1438  [production]  return i64::MAX;
GAP LINE   crates/runner/src/outcome.rs:1571  [production]  return 0;
GAP LINE   crates/runner/src/outcome.rs:1575  [production]  return i64::MAX;
GAP LINE   crates/runner/src/outcome.rs:1648  [production]  return 0;
GAP LINE   crates/runner/src/outcome.rs:1658  [production]  return i64::MAX;
GAP BRANCH crates/runner/src/outcome.rs:812:19 False  [production]  while let Some(below) = levels.last() {
GAP BRANCH crates/runner/src/outcome.rs:962:13 False  [production]  let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {
GAP BRANCH crates/runner/src/outcome.rs:1050:13 False  [production]  let Some((later, now)) = bars
GAP BRANCH crates/runner/src/outcome.rs:1170:28 False  [production]  if let Some(Reverse(moved)) = upper.pop() {
GAP BRANCH crates/runner/src/outcome.rs:1174:28 False  [production]  && let Some(moved) = lower.pop()
GAP BRANCH crates/runner/src/outcome.rs:1193:20 False  [production]  && let Some(slot) = out.get_mut(index)
GAP BRANCH crates/runner/src/outcome.rs:1434:12 True  [production]  if !ratio.is_finite() || ratio <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1434:34 True  [production]  if !ratio.is_finite() || ratio <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1437:12 True  [production]  if ratio >= 9_223_372_036_854_775_000.0 {
GAP BRANCH crates/runner/src/outcome.rs:1559:26 True  [production]  if backs == 0 || back_sum <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1570:14 False  [production]  if !(mean_gain.is_finite() && mean_back.is_finite()) || mean_back <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1570:39 False  [production]  if !(mean_gain.is_finite() && mean_back.is_finite()) || mean_back <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1570:65 True  [production]  if !(mean_gain.is_finite() && mean_back.is_finite()) || mean_back <= 0.0 {
GAP BRANCH crates/runner/src/outcome.rs:1574:12 True  [production]  if !ratio.is_finite() || ratio >= UNBOUNDED {
GAP BRANCH crates/runner/src/outcome.rs:1574:34 True  [production]  if !ratio.is_finite() || ratio >= UNBOUNDED {
GAP BRANCH crates/runner/src/outcome.rs:1647:12 True  [production]  if self.n == 0 {
GAP BRANCH crates/runner/src/outcome.rs:1657:12 True  [production]  if !ratio.is_finite() || ratio >= UNBOUNDED {
GAP BRANCH crates/runner/src/outcome.rs:1657:34 True  [production]  if !ratio.is_finite() || ratio >= UNBOUNDED {
GAP BRANCH crates/runner/src/outcome.rs:2086:24 False  [production]  if let Some(freed) = self.slab.get_mut(at) {
GAP BRANCH crates/runner/src/outcome.rs:2091:24 False  [production]  if let Some(staying) = self.slab.get_mut(at) {
GAP BRANCH crates/runner/src/outcome.rs:2098:16 False  [production]  if let Some(head) = self.wheel.get_mut(slot) {
GAP BRANCH crates/runner/src/outcome.rs:2116:16 False  [production]  if let Some(head) = self.wheel.get_mut(slot) {
GAP BRANCH crates/runner/src/outcome.rs:2274:23 True  [production]  if !paired || !forward.covers(source) {
GAP? BRANCH crates/runner/src/outcome.rs:2626:41 False  [assert! condition branch, side False (inside a closure)]  assert!((0..bars.len()).all(|i| refused.at(i).is_none() && refused.was_refused(i)));
GAP BRANCH crates/runner/src/outcome.rs:4123:16 False  [test code branch]  if let Some(b) = bars.get_mut(2_000) {
GAP BRANCH crates/runner/src/outcome.rs:4299:16 False  [test code branch]  if let Some(slot) = healthy.get_mut(2) {
GAP BRANCH crates/runner/src/outcome.rs:4348:16 False  [test code branch]  if let Some(slot) = dirty.get_mut(3) {
GAP BRANCH crates/runner/src/outcome.rs:4430:16 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP BRANCH crates/runner/src/outcome.rs:4447:16 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP BRANCH crates/runner/src/outcome.rs:4859:65 False  [test code branch]  .filter(|w| matches!(w, [a, b] if a.2 != b.2 && b.0 - a.0 < horizon as usize))
GAP? BRANCH crates/runner/src/outcome.rs:4903:48 False  [assert! condition branch, side False (inside a closure)]  .all(|w| matches!(w, [a, b] if a.2 == b.2)),
GAP BRANCH crates/runner/src/outcome.rs:5123:16 False  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
GAP BRANCH crates/runner/src/outcome.rs:5123:48 True  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
GAP BRANCH crates/runner/src/outcome.rs:5123:48 False  [test code branch]  if i32::try_from(down).is_err() || i32::try_from(up).is_err() {
GAP BRANCH crates/runner/src/outcome.rs:5420:16 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP BRANCH crates/runner/src/pbo.rs:83:25 True  [production]  if last == 0 || self.winner_rank_twice > last {
GAP? BRANCH crates/runner/src/portfolio.rs:785:20 False  [assert! condition branch, side False (inside a closure)]  if admitted == StrategyDigest::new([1; 32])
GAP LINE   crates/runner/src/report.rs:157  [production]  Why::NotLive => "retired, void, or outside the table",
GAP LINE   crates/runner/src/report.rs:917  [production]  engine::Breach::Workers => "WORKERS",
GAP LINE   crates/runner/src/report.rs:1148  [test code]  || text.contains("partitions nothing")
GAP LINE   crates/runner/src/report.rs:1149  [test code]  || text.contains("retired, void, or outside the table"),
GAP LINE   crates/runner/src/report.rs:1157  [test code]  assert!(
GAP LINE   crates/runner/src/report.rs:1158  [test code]  text.contains("not measured"),
GAP? BRANCH crates/runner/src/report.rs:1148:20 True  [assert! condition branch, side True (inside a closure)]  || text.contains("partitions nothing")
GAP? BRANCH crates/runner/src/report.rs:1148:20 False  [assert! condition branch, side False (inside a closure)]  || text.contains("partitions nothing")
GAP? BRANCH crates/runner/src/report.rs:1149:20 True  [assert! condition branch, side True (inside a closure)]  || text.contains("retired, void, or outside the table"),
GAP? BRANCH crates/runner/src/report.rs:1149:20 False  [assert! condition branch, side False (inside a closure)]  || text.contains("retired, void, or outside the table"),
GAP BRANCH crates/runner/src/report.rs:1156:12 True  [test code branch]  if out.sweep.excluded.iter().any(|e| e.support.is_none()) {
GAP BRANCH crates/runner/src/resample.rs:555:16 False  [test code branch]  if let Some(b) = bars.first_mut() {
GAP BRANCH crates/runner/src/resample.rs:558:16 False  [test code branch]  if let Some(b) = bars.get_mut(1) {
GAP BRANCH crates/runner/src/resample.rs:561:16 False  [test code branch]  if let Some(b) = bars.get_mut(2) {
GAP BRANCH crates/runner/src/resample.rs:573:16 False  [test code branch]  if let Some(b) = tail.get_mut(2) {
GAP LINE   crates/runner/src/research_admission_projection.rs:29  [production]  pub fn with_search_probability_ceiling(mut self, maximum_ppm: u64) -> Self {
GAP LINE   crates/runner/src/research_admission_projection.rs:30  [production]  self.values.max_fwer_p_value_ppm = self.values.max_fwer_p_value_ppm.min(maximum_ppm);
GAP LINE   crates/runner/src/research_admission_projection.rs:31  [production]  self.values.max_spa_p_value_ppm = self.values.max_spa_p_value_ppm.min(maximum_ppm);
GAP LINE   crates/runner/src/research_admission_projection.rs:32  [production]  self.values.max_white_reality_p_value_ppm =
GAP LINE   crates/runner/src/research_admission_projection.rs:33  [production]  self.values.max_white_reality_p_value_ppm.min(maximum_ppm);
GAP LINE   crates/runner/src/research_admission_projection.rs:34  [production]  self.values.max_romano_wolf_p_value_ppm =
GAP LINE   crates/runner/src/research_admission_projection.rs:35  [production]  self.values.max_romano_wolf_p_value_ppm.min(maximum_ppm);
GAP LINE   crates/runner/src/research_admission_projection.rs:36  [production]  self
GAP LINE   crates/runner/src/research_admission_projection.rs:37  [production]  }
GAP LINE   crates/runner/src/research_exit_grid.rs:90  [production]  pub const fn instrument(&self) -> InstrumentKey {
GAP LINE   crates/runner/src/research_exit_grid.rs:91  [production]  self.instrument
GAP LINE   crates/runner/src/research_exit_grid.rs:92  [production]  }
GAP LINE   crates/runner/src/research_exit_grid_tests.rs:30  [test code]  )?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:126  [production]  ExitGridErrorV1::BufferAllocationRefused {
GAP LINE   crates/runner/src/resolved_grid_view.rs:127  [production]  buffer: "exact expression ladder",
GAP LINE   crates/runner/src/resolved_grid_view.rs:128  [production]  elements: values.len(),
GAP LINE   crates/runner/src/resolved_grid_view.rs:129  [production]  }
GAP LINE   crates/runner/src/resolved_grid_view.rs:130  [production]  })?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:195  [production]  return Err(ExitGridErrorV1::RefusedExecutionPaths {
GAP LINE   crates/runner/src/resolved_grid_view.rs:196  [production]  paths: grid.refused_paths,
GAP LINE   crates/runner/src/resolved_grid_view.rs:197  [production]  });
GAP LINE   crates/runner/src/resolved_grid_view.rs:206  [production]  return Err(ExitGridErrorV1::SeriesIdentityMismatch("instrument"));
GAP LINE   crates/runner/src/resolved_grid_view.rs:222  [production]  )?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:225  [production]  )?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:228  [production]  )?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:231  [production]  ExitGridErrorV1::BufferAllocationRefused {
GAP LINE   crates/runner/src/resolved_grid_view.rs:232  [production]  buffer: "coordinate row offsets",
GAP LINE   crates/runner/src/resolved_grid_view.rs:233  [production]  elements: row_count,
GAP LINE   crates/runner/src/resolved_grid_view.rs:234  [production]  }
GAP LINE   crates/runner/src/resolved_grid_view.rs:235  [production]  })?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:253  [production]  )?)
GAP LINE   crates/runner/src/resolved_grid_view.rs:260  [production]  return Err(ExitGridErrorV1::ReplayCellCountMismatch {
GAP LINE   crates/runner/src/resolved_grid_view.rs:261  [production]  expected: self.cell_count,
GAP LINE   crates/runner/src/resolved_grid_view.rs:262  [production]  actual,
GAP LINE   crates/runner/src/resolved_grid_view.rs:263  [production]  });
GAP LINE   crates/runner/src/resolved_grid_view.rs:276  [production]  )?;
GAP LINE   crates/runner/src/resolved_grid_view.rs:357  [production]  return false;
GAP LINE   crates/runner/src/resolved_grid_view.rs:376  [production]  return false;
GAP BRANCH crates/runner/src/resolved_grid_view.rs:155:12 True  [production]  if count != self.training_bars || crate::identity::data_digest(bars) != self.training_digest
GAP BRANCH crates/runner/src/resolved_grid_view.rs:178:16 True  [production]  || grid.targets.rungs() != self.target_levels_ppm()
GAP BRANCH crates/runner/src/resolved_grid_view.rs:179:16 True  [production]  || grid.trails.rungs() != self.trail_levels_ppm()
GAP BRANCH crates/runner/src/resolved_grid_view.rs:194:12 True  [production]  if grid.refused_paths != 0 {
GAP BRANCH crates/runner/src/resolved_grid_view.rs:205:12 True  [production]  if *series.instrument() != self.instrument {
GAP BRANCH crates/runner/src/resolved_grid_view.rs:259:12 True  [production]  if actual != self.cell_count {
GAP BRANCH crates/runner/src/resolved_grid_view.rs:351:32 True  [production]  if !visit(Chosen {
GAP BRANCH crates/runner/src/resolved_grid_view.rs:375:12 True  [production]  if !self.chosen_axes_are_in_bounds(chosen) {
GAP BRANCH crates/runner/src/resolved_grid_view.rs:388:16 True  [production]  || chosen
GAP BRANCH crates/runner/src/resolved_grid_view.rs:391:16 True  [production]  || chosen.tsl.is_some_and(|i| i >= self.trail_levels_ppm.len())
GAP BRANCH crates/runner/src/resolved_grid_view.rs:397:20 True  [production]  || ttp.trail >= self.trail_levels_ppm.len()
GAP BRANCH crates/runner/src/resolved_grid_view.rs:398:20 True  [production]  || chosen.target.is_some_and(|target| ttp.arm >= target)
GAP BRANCH crates/runner/src/resolved_grid_view.rs:399:20 True  [production]  || chosen.tsl.is_some_and(|tsl| ttp.trail >= tsl))
GAP BRANCH crates/runner/src/resolved_grid_view.rs:425:14 False  [production]  ) && cell.stop != self.forced_stop_index
GAP LINE   crates/runner/src/signal_candle_stop.rs:397  [production]  std::cmp::Ordering::Equal => return Err(Error::Encoding),
GAP LINE   crates/runner/src/signal_candle_stop.rs:524  [production]  return Err(Error::Encoding);
GAP LINE   crates/runner/src/signal_candle_stop.rs:620  [production]  return Err(Error::Encoding);
GAP LINE   crates/runner/src/signal_candle_stop.rs:768  [production]  return Err(Error::Encoding);
GAP LINE   crates/runner/src/signal_candle_stop.rs:896  [production]  return Err(Error::Source);
GAP LINE   crates/runner/src/signal_candle_stop.rs:939  [production]  )?;
GAP LINE   crates/runner/src/signal_candle_stop.rs:987  [production]  return Err(Error::Scope);
GAP LINE   crates/runner/src/signal_candle_stop.rs:1289  [production]  return Ok(Err(Reason::ClosingMinute));
GAP LINE   crates/runner/src/signal_candle_stop.rs:1373  [production]  return Err(Error::Source);
GAP LINE   crates/runner/src/signal_candle_stop.rs:1383  [production]  return Err(Error::Source);
GAP LINE   crates/runner/src/signal_candle_stop.rs:1403  [production]  return Err(Error::Source);
GAP LINE   crates/runner/src/signal_candle_stop.rs:1505  [production]  )?;
GAP LINE   crates/runner/src/signal_candle_stop.rs:1668  [production]  _ => Err(Error::Scope),
GAP BRANCH crates/runner/src/signal_candle_stop.rs:361:16 True  [production]  || !same_day
GAP BRANCH crates/runner/src/signal_candle_stop.rs:376:20 False  [production]  if row.exit_from_micros == row.exit_bar_micros && row.exit_until_micros == end => {}
GAP BRANCH crates/runner/src/signal_candle_stop.rs:376:67 False  [production]  if row.exit_from_micros == row.exit_bar_micros && row.exit_until_micros == end => {}
GAP BRANCH crates/runner/src/signal_candle_stop.rs:380:24 False  [production]  && end
GAP BRANCH crates/runner/src/signal_candle_stop.rs:384:24 False  [production]  && row.optimistic_exit_paisa == row.pessimistic_exit_paisa
GAP BRANCH crates/runner/src/signal_candle_stop.rs:385:24 False  [production]  && !row.gapped => {}
GAP BRANCH crates/runner/src/signal_candle_stop.rs:400:55 True  [production]  || row.exit_reason == ExitReason::Stop && row.optimistic_paisa >= 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:401:16 True  [production]  || row.entry_micros.rem_euclid(MINUTE) != 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:402:16 True  [production]  || row.exit_bar_micros.rem_euclid(MINUTE) != 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:403:16 True  [production]  || row.exit_bar_micros > forced_open(row.entry_micros)?
GAP BRANCH crates/runner/src/signal_candle_stop.rs:511:12 True  [production]  if version != 1
GAP BRANCH crates/runner/src/signal_candle_stop.rs:512:16 True  [production]  || row.signal_micros >= row.signal_close_micros
GAP BRANCH crates/runner/src/signal_candle_stop.rs:513:16 True  [production]  || row.stop_paisa <= 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:514:16 True  [production]  || row
GAP BRANCH crates/runner/src/signal_candle_stop.rs:517:16 True  [production]  || row
GAP BRANCH crates/runner/src/signal_candle_stop.rs:520:16 True  [production]  || (row.reason == Reason::None) != row.trade_index.is_some()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:522:21 True  [production]  && (row.entry_bar.is_none() || row.occupied_through_micros.is_none())
GAP BRANCH crates/runner/src/signal_candle_stop.rs:522:48 True  [production]  && (row.entry_bar.is_none() || row.occupied_through_micros.is_none())
GAP BRANCH crates/runner/src/signal_candle_stop.rs:609:12 True  [production]  if close > 1
GAP BRANCH crates/runner/src/signal_candle_stop.rs:610:16 True  [production]  || row.offered_bars == 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:611:16 True  [production]  || row.accepted_bars > row.offered_bars
GAP BRANCH crates/runner/src/signal_candle_stop.rs:612:16 True  [production]  || row.wins > row.trades
GAP BRANCH crates/runner/src/signal_candle_stop.rs:613:16 True  [production]  || row
GAP BRANCH crates/runner/src/signal_candle_stop.rs:617:16 True  [production]  || row.pessimistic_paisa > row.optimistic_paisa
GAP BRANCH crates/runner/src/signal_candle_stop.rs:618:36 True  [production]  || row.trades == 0 && (row.optimistic_paisa != 0 || row.pessimistic_paisa != 0)
GAP BRANCH crates/runner/src/signal_candle_stop.rs:618:65 True  [production]  || row.trades == 0 && (row.optimistic_paisa != 0 || row.pessimistic_paisa != 0)
GAP BRANCH crates/runner/src/signal_candle_stop.rs:748:12 True  [production]  if wins > trades
GAP BRANCH crates/runner/src/signal_candle_stop.rs:749:16 True  [production]  || stopped.checked_add(forced) != Some(trades)
GAP BRANCH crates/runner/src/signal_candle_stop.rs:750:16 True  [production]  || stop_gaps > stopped
GAP BRANCH crates/runner/src/signal_candle_stop.rs:751:16 True  [production]  || row.optimistic_paisa < row.pessimistic_paisa
GAP BRANCH crates/runner/src/signal_candle_stop.rs:752:16 True  [production]  || row.drawdown_paisa < 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:753:16 True  [production]  || row.adverse_paisa < 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:754:16 True  [production]  || row.favourable_paisa < 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:755:16 True  [production]  || holding_minutes < trades
GAP BRANCH crates/runner/src/signal_candle_stop.rs:757:20 True  [production]  && [
GAP BRANCH crates/runner/src/signal_candle_stop.rs:895:12 True  [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
GAP BRANCH crates/runner/src/signal_candle_stop.rs:895:51 True  [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
GAP BRANCH crates/runner/src/signal_candle_stop.rs:986:12 True  [production]  if direction == Direction::Undirected {
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1281:20 True  [production]  if !self
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1287:41 False  [production]  .is_some_and(|exit| exit.real && exit.bar == index)
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1340:8 True  [production]  if signals.is_empty()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1341:12 True  [production]  || bars.is_empty()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1345:12 True  [production]  || source.series.feed().is_empty()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1346:12 True  [production]  || source.series.commit().is_empty()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1347:12 True  [production]  || source.series.calendar_digest() == [0; 32]
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1348:12 True  [production]  || column.sourced() != Sourced::Signal
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1349:12 True  [production]  || column.bits().len() != column.known().len()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1350:12 True  [production]  || column.len() != column.sources().len()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1352:12 True  [production]  || column.evaluation_spec_token().is_none()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1353:12 True  [production]  || signals.windows(2).any(|pair| {
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1358:12 True  [production]  || column
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1367:12 True  [production]  if !column.accepts(index)
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1368:16 True  [production]  || row.ts_micros.rem_euclid(MINUTE) != 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1369:16 True  [production]  || row.ts_micros.checked_add(length).is_none()
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1370:16 True  [production]  || row.low <= 0
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1371:16 True  [production]  || row.high < row.low
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1377:12 True  [production]  if offered.windows(2).any(|pair| {
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1381:15 True  [production]  }) || offered.iter().any(|bar| ist_day(bar.ts_micros).is_err())
GAP BRANCH crates/runner/src/signal_candle_stop.rs:1402:8 True  [production]  if source.minute_context.get(start..end) != Some(bars) {
GAP? BRANCH crates/runner/src/signal_candle_stop_tests.rs:808:39 False  [assert! condition branch, side False (inside a closure)]  .all(|period| period.day >= first && period.day <= last)
GAP LINE   crates/runner/src/significance.rs:479  [production]  f64::NAN
GAP? BRANCH crates/runner/src/split.rs:513:60 True  [assert! condition branch, side True (inside a closure)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
GAP? BRANCH crates/runner/src/split.rs:513:60 False  [assert! condition branch, side False (inside a closure)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
GAP BRANCH crates/runner/src/synthetic.rs:334:27 True  [test code branch]  let (lo, hi) = if up < down { (up, down) } else { (down, up) };
GAP BRANCH crates/runner/src/topn.rs:373:16 False  [production]  if let Some(slot) = bytes.get_mut(..16) {
GAP BRANCH crates/runner/src/topn.rs:376:16 False  [production]  if let Some(slot) = bytes.get_mut(16..20) {
GAP BRANCH crates/runner/src/topn.rs:379:16 False  [production]  if let Some(slot) = bytes.get_mut(20..24) {
GAP BRANCH crates/runner/src/topn.rs:382:16 False  [production]  if let Some(slot) = bytes.get_mut(24..28) {
GAP BRANCH crates/runner/src/topn.rs:389:20 False  [production]  if let Some(slot) = bytes.get_mut(start..end) {
GAP BRANCH crates/runner/src/topn.rs:834:16 False  [test code branch]  if let Some(slot) = words.get_mut(word) {
GAP LINE   crates/runner/src/trade.rs:652  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:671  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:674  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:677  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:680  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:683  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:812  [production]  return Err("expression pricing column alignment mismatch".to_owned());
GAP LINE   crates/runner/src/trade.rs:1046  [production]  let blocked = open_until.is_some_and(|until| signal <= until);
GAP LINE   crates/runner/src/trade.rs:1047  [production]  if blocked {
GAP LINE   crates/runner/src/trade.rs:1048  [production]  out.while_open = out.while_open.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1049  [production]  } else {
GAP LINE   crates/runner/src/trade.rs:1050  [production]  out.too_late = out.too_late.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1051  [production]  }
GAP LINE   crates/runner/src/trade.rs:1052  [production]  continue;
GAP LINE   crates/runner/src/trade.rs:1108  [production]  out.while_open = out.while_open.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1159  [production]  if blocked {
GAP LINE   crates/runner/src/trade.rs:1160  [production]  out.while_open = out.while_open.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1161  [production]  } else {
GAP LINE   crates/runner/src/trade.rs:1162  [production]  out.too_late = out.too_late.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1163  [production]  }
GAP LINE   crates/runner/src/trade.rs:1164  [production]  continue;
GAP LINE   crates/runner/src/trade.rs:1287  [production]  out.occupancy.push(held(signal, entry, exit, false, facts));
GAP LINE   crates/runner/src/trade.rs:1288  [production]  if !blocked {
GAP LINE   crates/runner/src/trade.rs:1289  [production]  open_until = Some(exit);
GAP LINE   crates/runner/src/trade.rs:1290  [production]  }
GAP LINE   crates/runner/src/trade.rs:1293  [production]  out.while_open = out.while_open.saturating_add(1);
GAP LINE   crates/runner/src/trade.rs:1353  [production]  return None;
GAP LINE   crates/runner/src/trade.rs:1444  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:1447  [production]  return false;
GAP LINE   crates/runner/src/trade.rs:2042  [test code]  return false;
GAP LINE   crates/runner/src/trade.rs:2134  [test code]  assert_eq!(
GAP LINE   crates/runner/src/trade.rs:2135  [test code]  bars.get(fill).map(|bar| bar.ts_micros),
GAP LINE   crates/runner/src/trade.rs:2136  [test code]  bars.get(source)
GAP LINE   crates/runner/src/trade.rs:2137  [test code]  .and_then(|bar| bar.ts_micros.checked_add(60_000_000)),
GAP BRANCH crates/runner/src/trade.rs:528:29 False  [production]  *accepted && bar.ts_micros.rem_euclid(EXECUTION_MINUTE_MICROS) == 0
GAP BRANCH crates/runner/src/trade.rs:651:12 True  [production]  if from > to {
GAP BRANCH crates/runner/src/trade.rs:670:12 True  [production]  if from > to {
GAP BRANCH crates/runner/src/trade.rs:673:13 False  [production]  let Some(before) = self.refused_prefix.get(from).copied() else {
GAP BRANCH crates/runner/src/trade.rs:676:13 False  [production]  let Some(after) = self.refused_prefix.get(to.saturating_add(1)).copied() else {
GAP BRANCH crates/runner/src/trade.rs:679:13 False  [production]  let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {
GAP BRANCH crates/runner/src/trade.rs:682:13 False  [production]  let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {
GAP BRANCH crates/runner/src/trade.rs:729:16 False  [production]  if let [before, after] = pair
GAP BRANCH crates/runner/src/trade.rs:810:8 True  [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
GAP BRANCH crates/runner/src/trade.rs:810:55 True  [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
GAP BRANCH crates/runner/src/trade.rs:1045:13 False  [production]  let Some(entry) = signal.checked_add(step) else {
GAP BRANCH crates/runner/src/trade.rs:1047:16 True  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1047:16 False  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1087:21 False  [production]  step_micros > 0
GAP BRANCH crates/runner/src/trade.rs:1107:16 True  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1158:12 True  [production]  if !same_session {
GAP BRANCH crates/runner/src/trade.rs:1159:16 True  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1159:16 False  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1286:16 True  [production]  if entry_is_priceable(bars, entry, direction, facts) {
GAP BRANCH crates/runner/src/trade.rs:1288:20 True  [production]  if !blocked {
GAP BRANCH crates/runner/src/trade.rs:1288:20 False  [production]  if !blocked {
GAP BRANCH crates/runner/src/trade.rs:1292:16 True  [production]  if blocked {
GAP BRANCH crates/runner/src/trade.rs:1352:8 True  [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
GAP BRANCH crates/runner/src/trade.rs:1352:37 True  [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
GAP BRANCH crates/runner/src/trade.rs:1443:9 False  [production]  let Some(bar) = bars.get(entry) else {
GAP BRANCH crates/runner/src/trade.rs:1446:8 True  [production]  if !facts.accepts(entry) {
GAP BRANCH crates/runner/src/trade.rs:1452:5 False  [production]  fills_at(fill_bar, fill_bar, direction, Anchor::Open).is_ok()
GAP BRANCH crates/runner/src/trade.rs:1878:17 False  [test code branch]  let [previous, next] = pair else {
GAP BRANCH crates/runner/src/trade.rs:2041:21 False  [test code branch]  let [before, after] = pair else {
GAP BRANCH crates/runner/src/trade.rs:2133:20 True  [test code branch]  if let Some(fill) = *target {
GAP? BRANCH crates/runner/src/trade.rs:2205:52 True  [assert! condition branch, side True (inside a closure)]  trade.entry_bar > previous_exit || n == 0,
GAP? BRANCH crates/runner/src/trade.rs:2205:52 False  [assert! condition branch, side False (inside a closure)]  trade.entry_bar > previous_exit || n == 0,
GAP BRANCH crates/runner/src/trade.rs:2640:38 True  [test code branch]  if path.priceable || !holed {
GAP BRANCH crates/runner/src/trade.rs:2775:32 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP BRANCH crates/runner/src/trade.rs:2781:32 False  [test code branch]  if let Some(bar) = dirty.get_mut(victim) {
GAP? BRANCH crates/runner/src/trade.rs:2813:42 False  [assert! condition branch, side False (inside a closure)]  .all(|trade| victim < trade.entry_bar || victim > trade.exit_bar),
GAP? BRANCH crates/runner/src/trade.rs:2819:33 False  [assert! condition branch, side False (inside a closure)]  !held.priceable
GAP? BRANCH crates/runner/src/trade.rs:2820:40 False  [assert! condition branch, side False (inside a closure)]  && held.entry_bar <= victim
GAP? BRANCH crates/runner/src/trade.rs:3025:17 True  [assert! condition branch, side True]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
GAP? BRANCH crates/runner/src/trade.rs:3025:34 False  [assert! condition branch, side False (inside a closure)]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
GAP LINE   crates/runner/src/validate.rs:788  [production]  fn from(value: AnchoredAdmissionValidationRefusalV2) -> Self {
GAP LINE   crates/runner/src/validate.rs:789  [production]  match value {
GAP LINE   crates/runner/src/validate.rs:790  [production]  AnchoredAdmissionValidationRefusalV2::UpstreamRefused(why) => {
GAP LINE   crates/runner/src/validate.rs:791  [production]  Self::UpstreamRefused(why)
GAP LINE   crates/runner/src/validate.rs:793  [production]  AnchoredAdmissionValidationRefusalV2::Empty => Self::Empty,
GAP LINE   crates/runner/src/validate.rs:794  [production]  AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs => Self::ZeroResolvedRungs,
GAP LINE   crates/runner/src/validate.rs:795  [production]  AnchoredAdmissionValidationRefusalV2::StableWidth => Self::StableWidth,
GAP LINE   crates/runner/src/validate.rs:796  [production]  AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold } => {
GAP LINE   crates/runner/src/validate.rs:797  [production]  Self::CandidateFamilyMismatch { fold }
GAP LINE   crates/runner/src/validate.rs:799  [production]  AnchoredAdmissionValidationRefusalV2::IncompleteChosenOos { fold } => {
GAP LINE   crates/runner/src/validate.rs:800  [production]  Self::IncompleteChosenOos { fold }
GAP LINE   crates/runner/src/validate.rs:802  [production]  AnchoredAdmissionValidationRefusalV2::ExitRungMismatch { fold } => {
GAP LINE   crates/runner/src/validate.rs:803  [production]  Self::ExitRungMismatch { fold }
GAP LINE   crates/runner/src/validate.rs:805  [production]  AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold } => {
GAP LINE   crates/runner/src/validate.rs:806  [production]  Self::ProvenanceMismatch { fold }
GAP LINE   crates/runner/src/validate.rs:809  [production]  Self::AggregateOosOverflow
GAP LINE   crates/runner/src/validate.rs:811  [production]  AnchoredAdmissionValidationRefusalV2::SealMismatch => Self::SealMismatch,
GAP LINE   crates/runner/src/validate.rs:813  [production]  }
GAP LINE   crates/runner/src/validate.rs:1074  [production]  fn from(value: ExitGridErrorV1) -> Self {
GAP LINE   crates/runner/src/validate.rs:1075  [production]  Self::ExitGrid(value)
GAP LINE   crates/runner/src/validate.rs:1076  [production]  }
GAP LINE   crates/runner/src/validate.rs:1104  [production]  pub const fn validated(&self) -> &Validated {
GAP LINE   crates/runner/src/validate.rs:1105  [production]  &self.validated
GAP LINE   crates/runner/src/validate.rs:1106  [production]  }
GAP LINE   crates/runner/src/validate.rs:1157  [production]  pub const fn digest(self) -> [u8; 32] {
GAP LINE   crates/runner/src/validate.rs:1158  [production]  self.0
GAP LINE   crates/runner/src/validate.rs:1159  [production]  }
GAP LINE   crates/runner/src/validate.rs:1322  [production]  pub const fn grid_identity(self) -> AnchoredSearchGridIdentityV4 {
GAP LINE   crates/runner/src/validate.rs:1323  [production]  self.grid_identity
GAP LINE   crates/runner/src/validate.rs:1324  [production]  }
GAP LINE   crates/runner/src/validate.rs:1605  [production]  Err(refusal) => {
GAP LINE   crates/runner/src/validate.rs:1606  [production]  capture.refusal = Some(refusal);
GAP LINE   crates/runner/src/validate.rs:1607  [production]  false
GAP LINE   crates/runner/src/validate.rs:1614  [production]  Err(refusal) => {
GAP LINE   crates/runner/src/validate.rs:1615  [production]  capture.refusal = Some(refusal);
GAP LINE   crates/runner/src/validate.rs:1616  [production]  false
GAP LINE   crates/runner/src/validate.rs:2026  [production]  Err(why) => {
GAP LINE   crates/runner/src/validate.rs:2027  [production]  return Validated {
GAP LINE   crates/runner/src/validate.rs:2028  [production]  refused: Some(why),
GAP LINE   crates/runner/src/validate.rs:2029  [production]  ..Validated::default()
GAP LINE   crates/runner/src/validate.rs:2030  [production]  };
GAP LINE   crates/runner/src/validate.rs:2229  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2277  [production]  return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
GAP LINE   crates/runner/src/validate.rs:2286  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2350  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:2351  [production]  "execution is not a non-empty exact one-minute path",
GAP LINE   crates/runner/src/validate.rs:2352  [production]  ));
GAP LINE   crates/runner/src/validate.rs:2393  [production]  return Err(AnchoredSearchValidationRefusalV4::Empty);
GAP LINE   crates/runner/src/validate.rs:2477  [production]  return Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
GAP LINE   crates/runner/src/validate.rs:2478  [production]  "the prepared signal column does not exactly cover its ordered signal series",
GAP LINE   crates/runner/src/validate.rs:2479  [production]  ));
GAP LINE   crates/runner/src/validate.rs:2503  [production]  return Err(AnchoredSearchValidationRefusalV4::SignalSourceMismatch(
GAP LINE   crates/runner/src/validate.rs:2504  [production]  "prepared signal-prefix boundaries are not monotonic",
GAP LINE   crates/runner/src/validate.rs:2505  [production]  ));
GAP LINE   crates/runner/src/validate.rs:2529  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2563  [production]  return Err(AnchoredSearchValidationRefusalV4::Empty);
GAP LINE   crates/runner/src/validate.rs:2579  [production]  return Err(AnchoredSearchValidationRefusalV4::DiscontiguousTraining { fold: index });
GAP LINE   crates/runner/src/validate.rs:2600  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2622  [production]  return Err(AnchoredSearchValidationRefusalV4::IncompleteSearch { fold: index });
GAP LINE   crates/runner/src/validate.rs:2746  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:2747  [production]  "evaluated cell population differs from its resolution",
GAP LINE   crates/runner/src/validate.rs:2748  [production]  ));
GAP LINE   crates/runner/src/validate.rs:2788  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2809  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:2810  [production]  "mask-by-side population is incomplete",
GAP LINE   crates/runner/src/validate.rs:2811  [production]  ));
GAP LINE   crates/runner/src/validate.rs:2863  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2904  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2910  [production]  )?;
GAP LINE   crates/runner/src/validate.rs:2949  [production]  }
GAP LINE   crates/runner/src/validate.rs:2952  [production]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
GAP LINE   crates/runner/src/validate.rs:3011  [production]  120_000_000 => Some("2min"),
GAP LINE   crates/runner/src/validate.rs:3012  [production]  180_000_000 => Some("3min"),
GAP LINE   crates/runner/src/validate.rs:3013  [production]  300_000_000 => Some("5min"),
GAP LINE   crates/runner/src/validate.rs:3014  [production]  600_000_000 => Some("10min"),
GAP LINE   crates/runner/src/validate.rs:3015  [production]  900_000_000 => Some("15min"),
GAP LINE   crates/runner/src/validate.rs:3016  [production]  1_800_000_000 => Some("30min"),
GAP LINE   crates/runner/src/validate.rs:3017  [production]  3_600_000_000 => Some("60min"),
GAP LINE   crates/runner/src/validate.rs:3018  [production]  _ => None,
GAP LINE   crates/runner/src/validate.rs:3052  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:3053  [production]  "one resolution or policy digest is torn",
GAP LINE   crates/runner/src/validate.rs:3054  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3071  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:3072  [production]  "Long/Short policy or resolution identities are indistinguishable",
GAP LINE   crates/runner/src/validate.rs:3073  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3078  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:3079  [production]  "retained resolution does not exactly match the attested execution series",
GAP LINE   crates/runner/src/validate.rs:3080  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3104  [production]  overflowed = true;
GAP LINE   crates/runner/src/validate.rs:3105  [production]  return false;
GAP LINE   crates/runner/src/validate.rs:3111  [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
GAP LINE   crates/runner/src/validate.rs:3112  [production]  "canonical coordinate order is incomplete",
GAP LINE   crates/runner/src/validate.rs:3113  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3257  [production]  hasher.update(&[0]);
GAP LINE   crates/runner/src/validate.rs:3258  [production]  return;
GAP LINE   crates/runner/src/validate.rs:3352  [production]  if !proof.candidates.is_empty()
GAP LINE   crates/runner/src/validate.rs:3353  [production]  || visible.chosen.is_some()
GAP LINE   crates/runner/src/validate.rs:3354  [production]  || visible.chosen_side.is_some()
GAP LINE   crates/runner/src/validate.rs:3355  [production]  || visible.chosen_exit.is_some()
GAP LINE   crates/runner/src/validate.rs:3356  [production]  || visible.chosen_exit_total.is_some()
GAP LINE   crates/runner/src/validate.rs:3357  [production]  || visible.out_of_sample_exit.is_some()
GAP LINE   crates/runner/src/validate.rs:3359  [production]  return Err(AnchoredSearchValidationRefusalV4::ProvenanceMismatch { fold: expected });
GAP LINE   crates/runner/src/validate.rs:3360  [production]  }
GAP LINE   crates/runner/src/validate.rs:3361  [production]  return Ok((0, 0, 0));
GAP LINE   crates/runner/src/validate.rs:3566  [production]  "a walk-forward fold could not align its signal rows to exact one-minute closes".to_owned()
GAP LINE   crates/runner/src/validate.rs:3567  [production]  })?;
GAP LINE   crates/runner/src/validate.rs:3572  [production]  "a walk-forward fold produced an execution alignment that was not parallel to its signal column"
GAP LINE   crates/runner/src/validate.rs:3573  [production]  .to_owned()
GAP LINE   crates/runner/src/validate.rs:3574  [production]  })
GAP LINE   crates/runner/src/validate.rs:3593  [production]  return Err("an OOS projection boundary is outside its supplied series".to_owned());
GAP LINE   crates/runner/src/validate.rs:3602  [production]  "an OOS fold could not align its warmed signal rows to exact one-minute closes".to_owned()
GAP LINE   crates/runner/src/validate.rs:3603  [production]  })?;
GAP LINE   crates/runner/src/validate.rs:3605  [production]  return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());
GAP LINE   crates/runner/src/validate.rs:3623  [production]  "an OOS execution alignment was not parallel to its warmed signal column".to_owned()
GAP LINE   crates/runner/src/validate.rs:3624  [production]  })?;
GAP LINE   crates/runner/src/validate.rs:3626  [production]  return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());
GAP LINE   crates/runner/src/validate.rs:3633  [production]  return Err("an OOS projection retained a source outside its exact OOS window".to_owned());
GAP LINE   crates/runner/src/validate.rs:3666  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ZeroResolvedRungs);
GAP LINE   crates/runner/src/validate.rs:3714  [production]  return Err(refusal);
GAP LINE   crates/runner/src/validate.rs:3717  [production]  return Err(AnchoredAdmissionValidationRefusalV2::UpstreamRefused(
GAP LINE   crates/runner/src/validate.rs:3718  [production]  why.clone(),
GAP LINE   crates/runner/src/validate.rs:3719  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3722  [production]  return Err(AnchoredAdmissionValidationRefusalV2::Empty);
GAP LINE   crates/runner/src/validate.rs:3725  [production]  return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
GAP LINE   crates/runner/src/validate.rs:3769  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:3772  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:3788  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:3798  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:3832  [production]  return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
GAP LINE   crates/runner/src/validate.rs:3869  [production]  )?);
GAP LINE   crates/runner/src/validate.rs:3878  [production]  return Err(refusal);
GAP LINE   crates/runner/src/validate.rs:3881  [production]  return Err(AnchoredSearchValidationRefusalV3::UpstreamRefused(
GAP LINE   crates/runner/src/validate.rs:3882  [production]  why.clone(),
GAP LINE   crates/runner/src/validate.rs:3883  [production]  ));
GAP LINE   crates/runner/src/validate.rs:3886  [production]  return Err(AnchoredSearchValidationRefusalV3::Empty);
GAP LINE   crates/runner/src/validate.rs:3889  [production]  return Err(AnchoredSearchValidationRefusalV3::SealMismatch);
GAP LINE   crates/runner/src/validate.rs:3921  [production]  return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
GAP LINE   crates/runner/src/validate.rs:3924  [production]  return Err(AnchoredAdmissionValidationRefusalV2::SealMismatch);
GAP LINE   crates/runner/src/validate.rs:3953  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
GAP LINE   crates/runner/src/validate.rs:3954  [production]  fold: expected,
GAP LINE   crates/runner/src/validate.rs:3955  [production]  });
GAP LINE   crates/runner/src/validate.rs:3996  [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch {
GAP LINE   crates/runner/src/validate.rs:3997  [production]  fold: expected,
GAP LINE   crates/runner/src/validate.rs:3998  [production]  });
GAP LINE   crates/runner/src/validate.rs:4021  [production]  return Err(AnchoredSearchValidationRefusalV3::ZeroResolvedRungs);
GAP LINE   crates/runner/src/validate.rs:4058  [production]  return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch { fold: expected });
GAP LINE   crates/runner/src/validate.rs:4081  [production]  return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch {
GAP LINE   crates/runner/src/validate.rs:4082  [production]  fold: expected,
GAP LINE   crates/runner/src/validate.rs:4083  [production]  });
GAP LINE   crates/runner/src/validate.rs:4099  [production]  return Err(AnchoredSearchValidationRefusalV3::ProvenanceMismatch {
GAP LINE   crates/runner/src/validate.rs:4100  [production]  fold: expected,
GAP LINE   crates/runner/src/validate.rs:4101  [production]  });
GAP LINE   crates/runner/src/validate.rs:4124  [production]  return false;
GAP LINE   crates/runner/src/validate.rs:4180  [production]  rung(&pick.targets, Some(ttp.arm))?,
GAP LINE   crates/runner/src/validate.rs:4181  [production]  rung(&pick.trails, Some(ttp.trail))?,
GAP LINE   crates/runner/src/validate.rs:4183  [production]  })?;
GAP LINE   crates/runner/src/validate.rs:4315  [production]  return Err(AnchoredAdmissionValidationRefusalV2::CandidateFamilyMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:4339  [production]  return Err(AnchoredSearchValidationRefusalV3::CandidateFamilyMismatch { fold });
GAP LINE   crates/runner/src/validate.rs:4485  [production]  return Validated {
GAP LINE   crates/runner/src/validate.rs:4486  [production]  folds: Vec::new(),
GAP LINE   crates/runner/src/validate.rs:4487  [production]  refused: Some(
GAP LINE   crates/runner/src/validate.rs:4488  [production]  "the signal timeframe has no positive duration, so its exact close cannot be mapped"
GAP LINE   crates/runner/src/validate.rs:4489  [production]  .to_owned(),
GAP LINE   crates/runner/src/validate.rs:4490  [production]  ),
GAP LINE   crates/runner/src/validate.rs:4491  [production]  };
GAP LINE   crates/runner/src/validate.rs:4516  [production]  continue;
GAP LINE   crates/runner/src/validate.rs:4630  [production]  continue;
GAP LINE   crates/runner/src/validate.rs:4634  [production]  out.refused =
GAP LINE   crates/runner/src/validate.rs:4635  [production]  Some("walk-forward training execution boundaries are not monotonic".to_owned());
GAP LINE   crates/runner/src/validate.rs:4636  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:4650  [production]  Err(why) => {
GAP LINE   crates/runner/src/validate.rs:4651  [production]  out.refused = Some(why);
GAP LINE   crates/runner/src/validate.rs:4652  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:4851  [production]  return None;
GAP LINE   crates/runner/src/validate.rs:4972  [production]  out.refused = Some(
GAP LINE   crates/runner/src/validate.rs:4973  [production]  "a walk-forward fold selected a candidate without its training side"
GAP LINE   crates/runner/src/validate.rs:4974  [production]  .to_owned(),
GAP LINE   crates/runner/src/validate.rs:4975  [production]  );
GAP LINE   crates/runner/src/validate.rs:4976  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:4981  [production]  Err(why) => {
GAP LINE   crates/runner/src/validate.rs:4982  [production]  out.refused = Some(why);
GAP LINE   crates/runner/src/validate.rs:4983  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:4999  [production]  out.refused = Some(
GAP LINE   crates/runner/src/validate.rs:5000  [production]  "walk-forward OOS execution boundaries are not monotonic".to_owned(),
GAP LINE   crates/runner/src/validate.rs:5001  [production]  );
GAP LINE   crates/runner/src/validate.rs:5002  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:5016  [production]  Err(why) => {
GAP LINE   crates/runner/src/validate.rs:5017  [production]  out.refused = Some(why);
GAP LINE   crates/runner/src/validate.rs:5018  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:5092  [production]  0
GAP LINE   crates/runner/src/validate.rs:5204  [production]  return out;
GAP LINE   crates/runner/src/validate.rs:5585  [test code]  builder_was_called = true;
GAP LINE   crates/runner/src/validate.rs:5586  [test code]  Err("the zero-rung door must refuse before building".to_owned())
GAP LINE   crates/runner/src/validate.rs:5587  [test code]  };
GAP LINE   crates/runner/src/validate.rs:5723  [test code]  &|_| {},
GAP LINE   crates/runner/src/validate.rs:5756  [test code]  &|_| {},
GAP LINE   crates/runner/src/validate.rs:6016  [test code]  &|_| {
GAP LINE   crates/runner/src/validate.rs:6017  [test code]  silent.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
GAP LINE   crates/runner/src/validate.rs:6018  [test code]  },
GAP LINE   crates/runner/src/validate.rs:6249  [test code]  continue;
GAP LINE   crates/runner/src/validate.rs:6265  [test code]  continue;
GAP LINE   crates/runner/src/validate.rs:6420  [test code]  }
GAP LINE   crates/runner/src/validate.rs:6599  [test code]  None => f.out_of_sample.worst_case_positive(),
GAP LINE   crates/runner/src/validate.rs:6630  [test code]  assert!(
GAP LINE   crates/runner/src/validate.rs:6631  [test code]  f.chosen_exit.is_some(),
GAP LINE   crates/runner/src/validate.rs:6662  [test code]  continue;
GAP LINE   crates/runner/src/validate.rs:6960  [test code]  continue;
GAP LINE   crates/runner/src/validate.rs:6963  [test code]  continue;
GAP LINE   crates/runner/src/validate.rs:7382  [test code]  mask.without_bit(383)
GAP LINE   crates/runner/src/validate.rs:7400  [test code]  Direction::Short => Direction::Long,
GAP LINE   crates/runner/src/validate.rs:7580  [test code]  matches!(
GAP LINE   crates/runner/src/validate.rs:7595  [test code]  Direction::Short => Direction::Long,
GAP LINE   crates/runner/src/validate.rs:7598  [test code]  matches!(
GAP LINE   crates/runner/src/validate.rs:7700  [test code]  }
GAP BRANCH crates/runner/src/validate.rs:2276:8 True  [production]  if rungs == 0 {
GAP BRANCH crates/runner/src/validate.rs:2349:8 True  [production]  if !is_one_minute_path(execution.bars()) {
GAP BRANCH crates/runner/src/validate.rs:2392:8 True  [production]  if folds.is_empty() || folds.len() != validated.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:2392:28 True  [production]  if folds.is_empty() || folds.len() != validated.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:2451:8 True  [production]  if signal.is_empty()
GAP BRANCH crates/runner/src/validate.rs:2452:12 True  [production]  || !signal.windows(2).all(|pair| {
GAP BRANCH crates/runner/src/validate.rs:2457:12 True  [production]  || column.sourced() != indicators::column::Sourced::Signal
GAP BRANCH crates/runner/src/validate.rs:2458:12 True  [production]  || column.bits().len() != column.sources().len()
GAP BRANCH crates/runner/src/validate.rs:2459:12 True  [production]  || census.offered != signal_bars
GAP BRANCH crates/runner/src/validate.rs:2460:12 True  [production]  || census.swept != stable_u64_v4(column.bits().len())?
GAP BRANCH crates/runner/src/validate.rs:2461:12 True  [production]  || !census.reconciles()
GAP BRANCH crates/runner/src/validate.rs:2462:12 True  [production]  || !column.acceptance_covers(signal.len())
GAP BRANCH crates/runner/src/validate.rs:2463:12 True  [production]  || column.acceptance_census() != census
GAP BRANCH crates/runner/src/validate.rs:2464:12 True  [production]  || column.evaluation_spec_token().is_none()
GAP BRANCH crates/runner/src/validate.rs:2465:12 True  [production]  || column.collided() != 0
GAP BRANCH crates/runner/src/validate.rs:2466:12 True  [production]  || column.first_swept() != column.sources().first().copied()
GAP BRANCH crates/runner/src/validate.rs:2467:12 True  [production]  || column
GAP BRANCH crates/runner/src/validate.rs:2472:12 True  [production]  || !column
GAP BRANCH crates/runner/src/validate.rs:2498:12 True  [production]  if self.end > full.sources().len()
GAP BRANCH crates/runner/src/validate.rs:2499:16 True  [production]  || self
GAP BRANCH crates/runner/src/validate.rs:2530:12 True  [production]  if prepared.sources() != expected_sources
GAP BRANCH crates/runner/src/validate.rs:2532:16 True  [production]  || prepared.evaluation_spec_token() != full.evaluation_spec_token()
GAP BRANCH crates/runner/src/validate.rs:2533:16 True  [production]  || prepared.acceptance().as_deref() != Some(expected_acceptance)
GAP BRANCH crates/runner/src/validate.rs:2562:8 True  [production]  if split_windows.is_empty() {
GAP BRANCH crates/runner/src/validate.rs:2574:12 True  [production]  if fold.train.0.start != 0
GAP BRANCH crates/runner/src/validate.rs:2575:16 True  [production]  || !fold.train.1.is_empty()
GAP BRANCH crates/runner/src/validate.rs:2576:16 True  [production]  || fold.train.0.is_empty()
GAP BRANCH crates/runner/src/validate.rs:2577:16 True  [production]  || fold.test.is_empty()
GAP BRANCH crates/runner/src/validate.rs:2621:12 True  [production]  if swept.sweep.halted.is_some() || !swept.is_complete() {
GAP BRANCH crates/runner/src/validate.rs:2621:44 True  [production]  if swept.sweep.halted.is_some() || !swept.is_complete() {
GAP BRANCH crates/runner/src/validate.rs:2745:16 True  [production]  if evaluated_cells != resolved.cell_count() {
GAP BRANCH crates/runner/src/validate.rs:2795:24 False  [production]  if let Some(selected) = pair.selected {
GAP BRANCH crates/runner/src/validate.rs:2808:12 True  [production]  if population_cells != expected_population {
GAP BRANCH crates/runner/src/validate.rs:2839:12 False  [production]  if !pending.is_empty() {
GAP BRANCH crates/runner/src/validate.rs:2951:12 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
GAP BRANCH crates/runner/src/validate.rs:2951:55 True  [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
GAP BRANCH crates/runner/src/validate.rs:3045:55 True  [production]  if long.side() != crate::excursion::Side::Long || short.side() != crate::excursion::Side::Short
GAP BRANCH crates/runner/src/validate.rs:3051:8 True  [production]  if !long.digest_is_valid() || !short.digest_is_valid() {
GAP BRANCH crates/runner/src/validate.rs:3051:35 True  [production]  if !long.digest_is_valid() || !short.digest_is_valid() {
GAP BRANCH crates/runner/src/validate.rs:3056:8 True  [production]  if long.instrument() != short.instrument()
GAP BRANCH crates/runner/src/validate.rs:3057:12 True  [production]  || long.instrument() != *series.instrument()
GAP BRANCH crates/runner/src/validate.rs:3058:12 True  [production]  || long.feed_digest() != short.feed_digest()
GAP BRANCH crates/runner/src/validate.rs:3059:12 True  [production]  || long.commit_digest() != short.commit_digest()
GAP BRANCH crates/runner/src/validate.rs:3060:12 True  [production]  || long.calendar_digest() != short.calendar_digest()
GAP BRANCH crates/runner/src/validate.rs:3062:12 True  [production]  || long.training_bars() != short.training_bars()
GAP BRANCH crates/runner/src/validate.rs:3063:12 True  [production]  || long.training_first_ts_micros() != short.training_first_ts_micros()
GAP BRANCH crates/runner/src/validate.rs:3064:12 True  [production]  || long.training_last_ts_micros() != short.training_last_ts_micros()
GAP BRANCH crates/runner/src/validate.rs:3070:8 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
GAP BRANCH crates/runner/src/validate.rs:3070:57 True  [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
GAP BRANCH crates/runner/src/validate.rs:3077:8 True  [production]  if &fresh_long != long || &fresh_short != short {
GAP BRANCH crates/runner/src/validate.rs:3077:31 True  [production]  if &fresh_long != long || &fresh_short != short {
GAP BRANCH crates/runner/src/validate.rs:3103:17 False  [production]  let Some(next) = ordinal.checked_add(1) else {
GAP BRANCH crates/runner/src/validate.rs:3110:12 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
GAP BRANCH crates/runner/src/validate.rs:3110:26 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
GAP BRANCH crates/runner/src/validate.rs:3110:39 True  [production]  if overflowed || !complete || ordinal != resolved.cell_count() {
GAP BRANCH crates/runner/src/validate.rs:3256:9 False  [production]  let Some(choice) = choice else {
GAP BRANCH crates/runner/src/validate.rs:3303:5 False  [production]  choice.mask == candidate.mask
GAP BRANCH crates/runner/src/validate.rs:3304:12 False  [production]  && choice.side == candidate.side
GAP BRANCH crates/runner/src/validate.rs:3305:12 False  [production]  && choice.coordinate == candidate.coordinate
GAP BRANCH crates/runner/src/validate.rs:3306:12 False  [production]  && choice.resolution_digest == candidate.resolution_digest
GAP BRANCH crates/runner/src/validate.rs:3307:12 False  [production]  && choice.evaluation_digest == candidate.evaluation_digest
GAP BRANCH crates/runner/src/validate.rs:3308:12 False  [production]  && choice.selection_digest == candidate.selection_digest
GAP BRANCH crates/runner/src/validate.rs:3309:12 False  [production]  && choice.training_run_id == candidate.training_run_id
GAP BRANCH crates/runner/src/validate.rs:3310:12 False  [production]  && choice.replay_digest == candidate.replay_digest
GAP BRANCH crates/runner/src/validate.rs:3311:12 False  [production]  && choice.oos_run_id == candidate.oos_run_id
GAP BRANCH crates/runner/src/validate.rs:3312:12 False  [production]  && choice.training_cell_digest == candidate.training_cell_digest
GAP BRANCH crates/runner/src/validate.rs:3313:12 False  [production]  && candidate.oos_cell_digest == Some(choice.oos_cell_digest)
GAP BRANCH crates/runner/src/validate.rs:3314:12 False  [production]  && choice.in_sample_pessimistic == candidate.in_sample_pessimistic
GAP BRANCH crates/runner/src/validate.rs:3327:5 False  [production]  u64::try_from(proof.candidates.len()).is_ok_and(|count| proof.selected_candidates == count)
GAP BRANCH crates/runner/src/validate.rs:3328:12 False  [production]  && proof.candidates.len() == visible.in_sample_all.len()
GAP BRANCH crates/runner/src/validate.rs:3329:12 False  [production]  && proof.candidates.len() == visible.out_of_sample_all.len()
GAP BRANCH crates/runner/src/validate.rs:3330:12 False  [production]  && proof.in_sample == visible.in_sample
GAP BRANCH crates/runner/src/validate.rs:3331:12 False  [production]  && proof.out_of_sample == visible.out_of_sample
GAP BRANCH crates/runner/src/validate.rs:3339:25 False  [production]  visible.in_sample_all.get(ordinal).copied()
GAP BRANCH crates/runner/src/validate.rs:3351:9 False  [production]  let Some(choice) = proof.choice else {
GAP BRANCH crates/runner/src/validate.rs:3352:12 True  [production]  if !proof.candidates.is_empty()
GAP BRANCH crates/runner/src/validate.rs:3352:12 False  [production]  if !proof.candidates.is_empty()
GAP BRANCH crates/runner/src/validate.rs:3353:16 True  [production]  || visible.chosen.is_some()
GAP BRANCH crates/runner/src/validate.rs:3353:16 False  [production]  || visible.chosen.is_some()
GAP BRANCH crates/runner/src/validate.rs:3354:16 True  [production]  || visible.chosen_side.is_some()
GAP BRANCH crates/runner/src/validate.rs:3354:16 False  [production]  || visible.chosen_side.is_some()
GAP BRANCH crates/runner/src/validate.rs:3355:16 True  [production]  || visible.chosen_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3355:16 False  [production]  || visible.chosen_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3356:16 True  [production]  || visible.chosen_exit_total.is_some()
GAP BRANCH crates/runner/src/validate.rs:3356:16 False  [production]  || visible.chosen_exit_total.is_some()
GAP BRANCH crates/runner/src/validate.rs:3357:16 True  [production]  || visible.out_of_sample_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3357:16 False  [production]  || visible.out_of_sample_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3381:8 True  [production]  if !choice_matches_candidate_v4(&choice, candidate)
GAP BRANCH crates/runner/src/validate.rs:3382:12 True  [production]  || visible.chosen != Some(choice.mask)
GAP BRANCH crates/runner/src/validate.rs:3383:12 True  [production]  || visible.chosen_side != Some(choice.side)
GAP BRANCH crates/runner/src/validate.rs:3384:12 True  [production]  || visible.chosen_exit != Some(choice.coordinate)
GAP BRANCH crates/runner/src/validate.rs:3386:12 True  [production]  || visible.out_of_sample_exit != Some(choice.out_of_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:3387:12 True  [production]  || best_ordinal != Some(ordinal)
GAP BRANCH crates/runner/src/validate.rs:3404:8 True  [production]  if proof.index != expected_index
GAP BRANCH crates/runner/src/validate.rs:3405:12 True  [production]  || visible.index != expected
GAP BRANCH crates/runner/src/validate.rs:3406:12 True  [production]  || proof.train_start != 0
GAP BRANCH crates/runner/src/validate.rs:3407:12 True  [production]  || proof.train_end.saturating_sub(proof.train_start) != stable_u64_v4(visible.train_bars)?
GAP BRANCH crates/runner/src/validate.rs:3408:12 True  [production]  || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v4(visible.test_bars)?
GAP BRANCH crates/runner/src/validate.rs:3409:12 True  [production]  || proof.purged != stable_u64_v4(visible.purged)?
GAP BRANCH crates/runner/src/validate.rs:3410:12 True  [production]  || proof.considered_masks != visible.considered
GAP BRANCH crates/runner/src/validate.rs:3411:12 True  [production]  || proof.considered_masks != visible.priced
GAP BRANCH crates/runner/src/validate.rs:3412:12 True  [production]  || visible.halted.is_some()
GAP BRANCH crates/runner/src/validate.rs:3432:8 True  [production]  if value.policy.semantic_order != CANDIDATE_SEMANTIC_ORDER_V4
GAP BRANCH crates/runner/src/validate.rs:3433:12 True  [production]  || value.policy.full_long_policy_digest == value.policy.full_short_policy_digest
GAP BRANCH crates/runner/src/validate.rs:3434:12 True  [production]  || value.policy.full_long_resolution_digest == value.policy.full_short_resolution_digest
GAP BRANCH crates/runner/src/validate.rs:3435:12 True  [production]  || value.validated.refused.is_some()
GAP BRANCH crates/runner/src/validate.rs:3436:12 True  [production]  || value.folds.is_empty()
GAP BRANCH crates/runner/src/validate.rs:3437:12 True  [production]  || value.folds.len() != value.validated.folds.len()
GAP BRANCH crates/runner/src/validate.rs:3529:12 True  [production]  if self.end > execution.len()
GAP BRANCH crates/runner/src/validate.rs:3592:8 True  [production]  if first_oos_signal > signal.len() || first_oos_execution > execution.len() {
GAP BRANCH crates/runner/src/validate.rs:3592:43 True  [production]  if first_oos_signal > signal.len() || first_oos_execution > execution.len() {
GAP BRANCH crates/runner/src/validate.rs:3604:8 True  [production]  if alignment.onto.len() != signal_column.sources().len() {
GAP BRANCH crates/runner/src/validate.rs:3625:8 True  [production]  if dropped != expected_dropped {
GAP BRANCH crates/runner/src/validate.rs:3628:8 True  [production]  if projected
GAP BRANCH crates/runner/src/validate.rs:3631:24 True  [production]  .any(|&source| source < first_oos_execution || source >= execution.len())
GAP BRANCH crates/runner/src/validate.rs:3665:12 True  [production]  if resolved_rungs == 0 {
GAP BRANCH crates/runner/src/validate.rs:3713:16 True  [production]  if let Some(refusal) = self.refusal.clone() {
GAP BRANCH crates/runner/src/validate.rs:3716:16 True  [production]  if let Some(why) = validated.refused.as_ref() {
GAP BRANCH crates/runner/src/validate.rs:3721:12 True  [production]  if validated.folds.is_empty() {
GAP BRANCH crates/runner/src/validate.rs:3724:12 True  [production]  if validated.folds.len() != self.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:3757:8 True  [production]  if visible.halted.is_some()
GAP BRANCH crates/runner/src/validate.rs:3758:12 True  [production]  || visible.considered != visible.priced
GAP BRANCH crates/runner/src/validate.rs:3760:12 True  [production]  || scored.len() != oos.len()
GAP BRANCH crates/runner/src/validate.rs:3761:12 True  [production]  || visible.in_sample_all.len() != scored.len()
GAP BRANCH crates/runner/src/validate.rs:3762:12 True  [production]  || visible.out_of_sample_all.len() != scored.len()
GAP BRANCH crates/runner/src/validate.rs:3768:13 False  [production]  let Some(candidate) = scored.get(ordinal) else {
GAP BRANCH crates/runner/src/validate.rs:3771:13 False  [production]  let Some(outcome) = oos.get(ordinal) else {
GAP BRANCH crates/runner/src/validate.rs:3787:12 True  [production]  if !choice_matches_visible_v2(&choice, visible, scored) {
GAP BRANCH crates/runner/src/validate.rs:3792:12 True  [production]  if visible.chosen.is_some()
GAP BRANCH crates/runner/src/validate.rs:3793:16 True  [production]  || visible.chosen_side.is_some()
GAP BRANCH crates/runner/src/validate.rs:3794:16 True  [production]  || visible.chosen_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3795:16 True  [production]  || visible.chosen_exit_total.is_some()
GAP BRANCH crates/runner/src/validate.rs:3796:16 True  [production]  || visible.out_of_sample_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3831:12 True  [production]  if resolved_rungs == 0 {
GAP BRANCH crates/runner/src/validate.rs:3877:16 True  [production]  if let Some(refusal) = self.refusal {
GAP BRANCH crates/runner/src/validate.rs:3880:16 True  [production]  if let Some(why) = validated.refused.as_ref() {
GAP BRANCH crates/runner/src/validate.rs:3885:12 True  [production]  if validated.folds.is_empty() {
GAP BRANCH crates/runner/src/validate.rs:3888:12 True  [production]  if validated.folds.len() != self.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:3920:8 True  [production]  if value.validated.refused.is_some() {
GAP BRANCH crates/runner/src/validate.rs:3923:8 True  [production]  if value.folds.is_empty() || value.folds.len() != value.validated.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:3923:34 True  [production]  if value.folds.is_empty() || value.folds.len() != value.validated.folds.len() {
GAP BRANCH crates/runner/src/validate.rs:3941:12 True  [production]  if proof.index != stable_u64_v2(expected)?
GAP BRANCH crates/runner/src/validate.rs:3942:16 True  [production]  || visible.index != expected
GAP BRANCH crates/runner/src/validate.rs:3943:16 True  [production]  || proof.train_end.saturating_sub(proof.train_start)
GAP BRANCH crates/runner/src/validate.rs:3945:16 True  [production]  || proof.train_second_end != proof.train_second_start
GAP BRANCH crates/runner/src/validate.rs:3946:16 True  [production]  || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v2(visible.test_bars)?
GAP BRANCH crates/runner/src/validate.rs:3947:16 True  [production]  || proof.purged != stable_u64_v2(visible.purged)?
GAP BRANCH crates/runner/src/validate.rs:3948:16 True  [production]  || proof.considered != visible.considered
GAP BRANCH crates/runner/src/validate.rs:3949:16 True  [production]  || proof.priced != visible.priced
GAP BRANCH crates/runner/src/validate.rs:3950:16 True  [production]  || proof.in_sample != visible.in_sample
GAP BRANCH crates/runner/src/validate.rs:3951:16 True  [production]  || proof.out_of_sample != visible.out_of_sample
GAP BRANCH crates/runner/src/validate.rs:3965:24 True  [production]  || visible.chosen_exit_total != Some(choice.in_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:3967:24 True  [production]  || visible.in_sample_all.get(ordinal).copied()
GAP BRANCH crates/runner/src/validate.rs:3969:24 True  [production]  || visible_oos != Some(choice.out_of_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:3970:24 True  [production]  || visible
GAP BRANCH crates/runner/src/validate.rs:3990:20 True  [production]  if visible.chosen.is_some()
GAP BRANCH crates/runner/src/validate.rs:3991:24 True  [production]  || visible.chosen_side.is_some()
GAP BRANCH crates/runner/src/validate.rs:3992:24 True  [production]  || visible.chosen_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:3993:24 True  [production]  || visible.chosen_exit_total.is_some()
GAP BRANCH crates/runner/src/validate.rs:3994:24 True  [production]  || visible.out_of_sample_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:4020:8 True  [production]  if value.policy.resolved_rungs == 0 {
GAP BRANCH crates/runner/src/validate.rs:4024:12 True  [production]  || value.policy.execution_mode != 1
GAP BRANCH crates/runner/src/validate.rs:4025:12 True  [production]  || value.validated.refused.is_some()
GAP BRANCH crates/runner/src/validate.rs:4026:12 True  [production]  || value.folds.is_empty()
GAP BRANCH crates/runner/src/validate.rs:4027:12 True  [production]  || value.folds.len() != value.validated.folds.len()
GAP BRANCH crates/runner/src/validate.rs:4032:12 True  [production]  || hash_validation_family_v3(&value.folds) != value.validation_family_digest
GAP BRANCH crates/runner/src/validate.rs:4046:12 True  [production]  if proof.index != stable_u64_v2(expected)?
GAP BRANCH crates/runner/src/validate.rs:4047:16 True  [production]  || visible.index != expected
GAP BRANCH crates/runner/src/validate.rs:4048:16 True  [production]  || proof.train_end.saturating_sub(proof.train_start)
GAP BRANCH crates/runner/src/validate.rs:4050:16 True  [production]  || proof.train_second_end != proof.train_second_start
GAP BRANCH crates/runner/src/validate.rs:4051:16 True  [production]  || proof.test_end.saturating_sub(proof.test_start) != stable_u64_v2(visible.test_bars)?
GAP BRANCH crates/runner/src/validate.rs:4052:16 True  [production]  || proof.purged != stable_u64_v2(visible.purged)?
GAP BRANCH crates/runner/src/validate.rs:4053:16 True  [production]  || proof.considered != visible.considered
GAP BRANCH crates/runner/src/validate.rs:4054:16 True  [production]  || proof.priced != visible.priced
GAP BRANCH crates/runner/src/validate.rs:4055:16 True  [production]  || proof.in_sample != visible.in_sample
GAP BRANCH crates/runner/src/validate.rs:4056:16 True  [production]  || proof.out_of_sample != visible.out_of_sample
GAP BRANCH crates/runner/src/validate.rs:4064:20 True  [production]  if visible.chosen != Some(choice.mask)
GAP BRANCH crates/runner/src/validate.rs:4065:24 True  [production]  || visible.chosen_side != Some(choice.side)
GAP BRANCH crates/runner/src/validate.rs:4066:24 True  [production]  || visible.chosen_exit != Some(choice.exit)
GAP BRANCH crates/runner/src/validate.rs:4067:24 True  [production]  || visible.chosen_exit_total != Some(choice.in_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:4068:24 True  [production]  || visible.out_of_sample_exit != Some(choice.out_of_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:4069:24 True  [production]  || visible.in_sample_all.get(ordinal).copied()
GAP BRANCH crates/runner/src/validate.rs:4071:24 True  [production]  || visible.out_of_sample_all.get(ordinal).copied()
GAP BRANCH crates/runner/src/validate.rs:4073:24 True  [production]  || visible
GAP BRANCH crates/runner/src/validate.rs:4093:20 True  [production]  if visible.chosen.is_some()
GAP BRANCH crates/runner/src/validate.rs:4094:24 True  [production]  || visible.chosen_side.is_some()
GAP BRANCH crates/runner/src/validate.rs:4095:24 True  [production]  || visible.chosen_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:4096:24 True  [production]  || visible.chosen_exit_total.is_some()
GAP BRANCH crates/runner/src/validate.rs:4097:24 True  [production]  || visible.out_of_sample_exit.is_some()
GAP BRANCH crates/runner/src/validate.rs:4123:9 False  [production]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {
GAP BRANCH crates/runner/src/validate.rs:4126:5 False  [production]  visible.chosen == Some(choice.mask)
GAP BRANCH crates/runner/src/validate.rs:4127:12 False  [production]  && visible.chosen_side == Some(choice.side)
GAP BRANCH crates/runner/src/validate.rs:4128:12 False  [production]  && visible.chosen_exit == Some(choice.exit)
GAP BRANCH crates/runner/src/validate.rs:4129:12 False  [production]  && visible.chosen_exit_total == Some(choice.in_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:4130:12 False  [production]  && visible.out_of_sample_exit == Some(choice.out_of_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:4131:12 False  [production]  && visible.in_sample_all.get(ordinal).copied() == Some(choice.in_sample_pessimistic)
GAP BRANCH crates/runner/src/validate.rs:4132:12 False  [production]  && scored.get(ordinal).is_some_and(|candidate| {
GAP BRANCH crates/runner/src/validate.rs:4133:13 False  [production]  candidate.mask == choice.mask
GAP BRANCH crates/runner/src/validate.rs:4134:20 False  [production]  && candidate.side == choice.side
GAP BRANCH crates/runner/src/validate.rs:4135:20 False  [production]  && candidate.pick.rungs == choice.exit
GAP BRANCH crates/runner/src/validate.rs:4314:8 True  [production]  if scored.len() != oos.len() {
GAP BRANCH crates/runner/src/validate.rs:4338:8 True  [production]  if scored.len() != oos.len() {
GAP BRANCH crates/runner/src/validate.rs:4484:12 True  [production]  if exec.signal_length_micros <= 0 {
GAP BRANCH crates/runner/src/validate.rs:4515:13 False  [production]  let Some(train) = bars.get(fold.train.0.clone()) else {
GAP BRANCH crates/runner/src/validate.rs:4629:13 False  [production]  let Some(test_open) = bars.get(fold.test.start).map(|bar| bar.ts_micros) else {
GAP BRANCH crates/runner/src/validate.rs:4633:17 False  [production]  let Some(prefix) = training_execution_cursor.before(exec.bars, test_open) else {
GAP BRANCH crates/runner/src/validate.rs:4850:20 True  [production]  if cell.trades == 0 {
GAP BRANCH crates/runner/src/validate.rs:4971:21 False  [production]  let Some(chosen_side) = chosen_side else {
GAP BRANCH crates/runner/src/validate.rs:4996:25 False  [production]  let Some(prefix) =
GAP BRANCH crates/runner/src/validate.rs:5091:24 True  [production]  if *mask == ConditionMask::ZERO {
GAP BRANCH crates/runner/src/validate.rs:5202:16 True  [production]  && !capture.observe_fold_and_retain(&fold, &visible, &scored, &oos_exact, best_ordinal)
GAP BRANCH crates/runner/src/validate.rs:6248:17 False  [test code branch]  let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
GAP BRANCH crates/runner/src/validate.rs:6264:17 False  [test code branch]  let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
GAP BRANCH crates/runner/src/validate.rs:6415:20 False  [test code branch]  if let [a, b] = pair {
GAP BRANCH crates/runner/src/validate.rs:6628:16 True  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
GAP BRANCH crates/runner/src/validate.rs:6628:57 True  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
GAP BRANCH crates/runner/src/validate.rs:6628:57 False  [test code branch]  if f.out_of_sample.worst_case_positive() && f.out_of_sample_exit.is_some_and(|t| t <= 0)
GAP BRANCH crates/runner/src/validate.rs:6661:16 True  [test code branch]  if f.out_of_sample.trades == 0 {
GAP BRANCH crates/runner/src/validate.rs:6670:16 False  [test code branch]  if f.out_of_sample_exit != Some(f.out_of_sample.worst) {
GAP BRANCH crates/runner/src/validate.rs:6959:21 False  [test code branch]  let Some(cell) = g.sharpest().or_else(|| g.best()) else {
GAP BRANCH crates/runner/src/validate.rs:6962:20 True  [test code branch]  if cell.trades == 0 {
GAP BRANCH crates/runner/src/validate.rs:7323:44 False  [test code branch]  .find(|(other, value)| *other != ordinal && *value != expected)
GAP BRANCH crates/runner/src/validate.rs:7381:34 True  [test code branch]  visible.chosen = Some(if high_bit_set {
GAP BRANCH crates/runner/src/validate.rs:7422:24 True  [test code branch]  exit.stop = if exit.stop.is_some() { None } else { Some(0) };
GAP? BRANCH crates/runner/src/validate.rs:7683:17 False  [assert! condition branch, side False (inside a closure)]  candidate.oos_cell_digest.is_some() && candidate.out_of_sample_pessimistic.is_some()
GAP BRANCH crates/runner/src/validate.rs:7691:20 False  [test code branch]  if let Some(choice) = proof.choice {
GAP? BRANCH crates/runner/tests/base_cov_admission_words.rs:66:69 False  [assert! condition branch, side False (inside a closure)]  name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
GAP BRANCH crates/runner/tests/d_0694_record.rs:179:12 False  [test code branch]  if is_source
GAP LINE   crates/runner/tests/end_to_end.rs:487  [test code]  Some(halt) => format!("HALTED k={} {:?}", halt.k, halt.breach),
GAP? BRANCH crates/runner/tests/end_to_end.rs:115:39 True  [assert! condition branch, side True (inside a closure)]  exits.baseline().is_some() || exits.cells.is_empty(),
GAP? BRANCH crates/runner/tests/end_to_end.rs:115:39 False  [assert! condition branch, side False (inside a closure)]  exits.baseline().is_some() || exits.cells.is_empty(),
GAP? BRANCH crates/runner/tests/end_to_end.rs:383:13 False  [assert! condition branch, side False (inside a closure)]  !rest.starts_with("  NOT SUPPLIED") && rest.contains("variants evaluated")
GAP LINE   crates/runner/tests/expression_pricing.rs:31  [test code]  fn debug(why: impl std::fmt::Debug) -> String {
GAP LINE   crates/runner/tests/expression_pricing.rs:32  [test code]  format!("{why:?}")
GAP LINE   crates/runner/tests/expression_pricing.rs:33  [test code]  }
GAP LINE   crates/runner/tests/expression_pricing.rs:55  [test code]  )?,
GAP LINE   crates/runner/tests/expression_pricing.rs:66  [test code]  )?;
GAP LINE   crates/runner/tests/expression_pricing.rs:80  [test code]  )?,
GAP LINE   crates/runner/tests/expression_pricing.rs:83  [test code]  )?
GAP LINE   crates/runner/tests/expression_pricing.rs:125  [test code]  )?;
GAP LINE   crates/runner/tests/expression_pricing.rs:136  [test code]  )?;
GAP LINE   crates/runner/tests/expression_pricing.rs:153  [test code]  }
GAP LINE   crates/runner/tests/expression_pricing.rs:186  [test code]  )?;
GAP BRANCH crates/runner/tests/expression_pricing.rs:105:31 False  [test code branch]  let either = (known.get(0) && truth.get(0)) || (known.get(369) && truth.get(369));
GAP BRANCH crates/runner/tests/expression_pricing.rs:105:65 False  [test code branch]  let either = (known.get(0) && truth.get(0)) || (known.get(369) && truth.get(369));
GAP BRANCH crates/runner/tests/expression_pricing.rs:108:29 False  [test code branch]  "!0" => known.get(0) && !truth.get(0),
GAP BRANCH crates/runner/tests/expression_pricing.rs:109:36 False  [test code branch]  _ => either && known.get(63) && !truth.get(63),
GAP BRANCH crates/runner/tests/expression_pricing.rs:127:20 False  [test code branch]  if let Some(cell) = priced.best() {
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:111  [test code]  )?;
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:136  [test code]  )?;
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:176  [test code]  )?;
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:201  [test code]  )?;
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:251  [test code]  )?;
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:259  [test code]  assert!(
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:260  [test code]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
GAP LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:336  [test code]  )?;
GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:21 True  [assert! condition branch, side True]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:36 True  [assert! condition branch, side True (inside a closure)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:36 False  [assert! condition branch, side False (inside a closure)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
GAP? BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:317:58 False  [assert! condition branch, side False (inside a closure)]  assert!(timed.eligible.iter().any(|item| item.entry_bar < corrupt_index && corrupt_index < item.exit_bar),
GAP BRANCH crates/runner/tests/hole_after_exit.rs:100:13 False  [test code branch]  row.entry_bar % BARS_PER_SESSION < 300
GAP LINE   crates/runner/tests/research_family_readiness.rs:431  [test code]  )?;
GAP LINE   crates/runner/tests/research_family_readiness.rs:433  [test code]  matches!(
GAP LINE   crates/runner/tests/research_family_readiness.rs:473  [test code]  )?;
GAP LINE   crates/runner/tests/research_family_readiness.rs:485  [test code]  )?;
GAP LINE   crates/runner/tests/research_family_readiness.rs:506  [test code]  indicators::vwap::Availability::Absent
GAP LINE   crates/runner/tests/research_family_readiness.rs:525  [test code]  )?;
GAP LINE   crates/runner/tests/research_family_readiness.rs:533  [test code]  )?;
GAP BRANCH crates/runner/tests/research_family_readiness.rs:503:27 False  [test code branch]  let availability = if family.is_cash() {
GAP LINE   crates/runner/tests/unwired_validation_record.rs:48  [test code]  }
GAP BRANCH crates/runner/tests/unwired_validation_record.rs:37:16 False  [test code branch]  if path.file_name().is_some_and(|name| name != "tests") {
GAP BRANCH crates/runner/tests/unwired_validation_record.rs:40:19 False  [test code branch]  } else if path.extension().is_some_and(|ext| ext == "rs") {
`````

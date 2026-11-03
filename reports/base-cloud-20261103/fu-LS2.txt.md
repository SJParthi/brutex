# fu-LS2.txt.md: runner (F) and (U) items

Shard BASE-COV-LIBS-LS2 (crate runner). BEFORE figures: measured at START (4f49dbf43e79 = bd04fb299d23 + kit + OWNER + the one-line test fix), before this shard's new tests; refreshed at the final tip when its Route B is read.
One fenced block below holds the list.

`````text
(F) and (U) items of runner on tip 4f49dbf43e791ba431566f9e86eba46db7a95a4b (recipe v4 step 4). (F) rows are classified by rule (scratchpad classify.mjs: the failing arm of assert!/assert_eq!/assert_ne!/panic!/unreachable! inside #[cfg(test)] code or tests/, the message argument of such an assert, a let-else or if whose block is only a panic, and the False side of an assert! condition operand that makes it fail); (U) rows were each read and proved by hand.

(F) ITEMS: 224
(F) LINE   crates/runner/src/admission.rs:7423  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7429  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7435  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7466  [assert_eq! message argument]  left.name(),
(F) LINE   crates/runner/src/admission.rs:7467  [assert_eq! message argument]  right.name()
(F) BRANCH crates/runner/src/admission_reader_cover_tests.rs:285:70 False  [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F) BRANCH crates/runner/src/admission_reader_cover_tests.rs:289:70 False  [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F) BRANCH crates/runner/src/audit.rs:2224:13 False  [assert! condition branch (failing side)]  ranked
(F) BRANCH crates/runner/src/audit.rs:2227:20 False  [assert! condition branch (failing side)]  && ranked.ends_with("what selection ranks on, gross of every charge"),
(F) BRANCH crates/runner/src/audit.rs:2641:21 False  [assert! condition branch (failing side)]  token
(F) BRANCH crates/runner/src/audit.rs:2788:13 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:2788:54 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:3067:17 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3067:35 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3067:54 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) LINE   crates/runner/src/bootstrap.rs:2108  [assert! message argument]  v.calibration()
(F) LINE   crates/runner/src/bootstrap.rs:2188  [assert! message argument]  (n as f64).sqrt()
(F) LINE   crates/runner/src/bootstrap.rs:2719  [assert! message argument]  draw.runs.len()
(F) LINE   crates/runner/src/bootstrap.rs:3408  [assert! message argument]  rounds + 1,
(F) LINE   crates/runner/src/bootstrap.rs:3409  [assert! message argument]  rejected.len(),
(F) LINE   crates/runner/src/bootstrap.rs:3410  [assert! message argument]  set.len()
(F) LINE   crates/runner/src/bootstrap.rs:3517  [assert! message argument]  rc.map(|v| v.p_value)
(F) LINE   crates/runner/src/bootstrap.rs:3523  [assert! message argument]  v.map(|v| (v.p_value, v.clears()))
(F) LINE   crates/runner/src/bootstrap.rs:3650  [assert! message argument]  disagreements.len()
(F) BRANCH crates/runner/src/bootstrap.rs:2435:13 False  [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),
(F) BRANCH crates/runner/src/bootstrap.rs:2435:36 False  [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),
(F) BRANCH crates/runner/src/bootstrap.rs:2652:17 False  [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F) BRANCH crates/runner/src/bootstrap.rs:2652:29 False  [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F) BRANCH crates/runner/src/bootstrap.rs:3398:17 False  [assert! condition branch (failing side)]  r.round == expected_round || r.round == expected_round + 1,
(F) BRANCH crates/runner/src/bootstrap.rs:3406:13 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap.rs:3406:40 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:13 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:28 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) LINE   crates/runner/src/excursion.rs:1685  [assert! message argument]  tightest(8),
(F) LINE   crates/runner/src/excursion.rs:1686  [assert! message argument]  tightest(4)
(F) LINE   crates/runner/src/excursion.rs:1691  [assert! message argument]  tightest(4),
(F) LINE   crates/runner/src/excursion.rs:1692  [assert! message argument]  tightest(2)
(F) LINE   crates/runner/src/excursion.rs:2584  [assert_eq! message argument]  sample.len()
(F) LINE   crates/runner/src/grid.rs:7081  [assert! message argument]  stopped.edge_ratio(),
(F) LINE   crates/runner/src/grid.rs:7082  [assert! message argument]  stopless.edge_ratio()
(F) LINE   crates/runner/src/grid.rs:8344  [assert! message argument]  c.uncertainty()
(F) LINE   crates/runner/src/grid.rs:9778  [assert! message argument]  a.reward_to_risk_bp()
(F) LINE   crates/runner/src/grid.rs:10246  [assert! message argument]  sniper.assurance_bp(),
(F) LINE   crates/runner/src/grid.rs:10247  [assert! message argument]  grinder.assurance_bp()
(F) LINE   crates/runner/src/grid.rs:10253  [assert! message argument]  grinder.assurance_bp(),
(F) LINE   crates/runner/src/grid.rs:10254  [assert! message argument]  fluke.assurance_bp()
(F) LINE   crates/runner/src/grid.rs:10293  [assert! message argument]  cell.win_rate_bp()
(F) LINE   crates/runner/src/grid.rs:10327  [assert! message argument]  trades - 1
(F) LINE   crates/runner/src/grid.rs:10964  [assert! message argument]  t.eligible.len(),
(F) LINE   crates/runner/src/grid.rs:10965  [assert! message argument]  t.trades.len()
(F) BRANCH crates/runner/src/grid.rs:6254:13 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6254:35 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:13 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:36 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:7977:13 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:7977:39 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:8066:21 False  [assert! condition branch (failing side)]  c.worst_trade <= 0 || c.pessimistic > 0,
(F) BRANCH crates/runner/src/grid.rs:8158:17 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid.rs:8158:40 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid.rs:10079:17 False  [assert! condition branch (failing side)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
(F) BRANCH crates/runner/src/grid_cover_tests.rs:56:13 False  [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F) BRANCH crates/runner/src/grid_cover_tests.rs:56:27 False  [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F) LINE   crates/runner/src/lib.rs:2028  [assert_eq! message argument]  reads.len()
(F) LINE   crates/runner/src/outcome.rs:3152  [assert! message argument]  column.bits().len()
(F) LINE   crates/runner/src/outcome.rs:3242  [assert! message argument]  minute_of(i)
(F) LINE   crates/runner/src/outcome.rs:3317  [assert! message argument]  bars.len()
(F) LINE   crates/runner/src/outcome.rs:3382  [assert_eq! message argument]  forced_minute - 1
(F) LINE   crates/runner/src/outcome.rs:3412  [assert_eq! message argument]  forced_minute - 1,
(F) LINE   crates/runner/src/outcome.rs:3413  [assert_eq! message argument]  forced_minute - 2
(F) LINE   crates/runner/src/outcome.rs:4615  [assert_eq! message argument]  i + 1
(F) LINE   crates/runner/src/outcome.rs:5277  [assert! message argument]  want[0]
(F) LINE   crates/runner/src/outcome.rs:5282  [assert! message argument]  want[1]
(F) LINE   crates/runner/src/outcome.rs:5287  [assert! message argument]  want[2]
(F) BRANCH crates/runner/src/outcome.rs:3665:13 False  [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),
(F) BRANCH crates/runner/src/outcome.rs:3665:36 False  [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),
(F) BRANCH crates/runner/src/outcome.rs:3764:13 False  [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F) BRANCH crates/runner/src/outcome.rs:3764:40 False  [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F) BRANCH crates/runner/src/outcome.rs:4143:13 False  [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),
(F) BRANCH crates/runner/src/outcome.rs:4143:27 False  [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),
(F) BRANCH crates/runner/src/outcome.rs:4571:13 False  [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F) BRANCH crates/runner/src/outcome.rs:4571:39 False  [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F) BRANCH crates/runner/src/outcome.rs:4883:17 False  [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F) BRANCH crates/runner/src/outcome.rs:4883:36 False  [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F) BRANCH crates/runner/src/outcome.rs:4920:17 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:4920:30 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:4920:50 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:5304:17 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5304:34 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5304:51 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5429:17 False  [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F) BRANCH crates/runner/src/outcome.rs:5429:46 False  [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F) BRANCH crates/runner/src/outcome.rs:5456:13 False  [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),
(F) BRANCH crates/runner/src/outcome.rs:5456:37 False  [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),
(F) BRANCH crates/runner/src/outcome.rs:5459:17 False  [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F) BRANCH crates/runner/src/outcome.rs:5459:47 False  [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F) BRANCH crates/runner/src/outcome.rs:5515:13 False  [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F) BRANCH crates/runner/src/outcome.rs:5515:39 False  [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F) BRANCH crates/runner/src/outcome.rs:5518:17 False  [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F) BRANCH crates/runner/src/outcome.rs:5518:42 False  [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F) BRANCH crates/runner/src/portfolio.rs:916:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([7; 32])
(F) BRANCH crates/runner/src/portfolio.rs:1018:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([3; 32])
(F) LINE   crates/runner/src/rank.rs:1264  [assert! message argument]  grinder.payoff_bp(),
(F) LINE   crates/runner/src/rank.rs:1265  [assert! message argument]  grinder.worst_reward_risk_bp()
(F) BRANCH crates/runner/src/report.rs:1147:13 False  [assert! condition branch (failing side)]  text.contains("never true on any loaded bar")
(F) BRANCH crates/runner/src/report.rs:1509:13 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1509:27 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1522:13 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/report.rs:1522:29 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/report_cover_tests.rs:192:13 False  [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F) BRANCH crates/runner/src/report_cover_tests.rs:192:45 False  [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F) BRANCH crates/runner/src/resample.rs:869:21 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:869:41 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:870:21 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) BRANCH crates/runner/src/resample.rs:870:42 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) LINE   crates/runner/src/signal_candle_stop_tests.rs:74  [unreachable! (failing arm)]  _ => unreachable!(),
(F) LINE   crates/runner/src/significance.rs:945  [assert! message argument]  bonferroni_t(trials)
(F) LINE   crates/runner/src/significance.rs:1341  [assert! message argument]  (approx - exact).abs()
(F) LINE   crates/runner/src/significance.rs:1357  [assert! message argument]  bonferroni_t(316)
(F) BRANCH crates/runner/src/significance.rs:1323:21 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1323:42 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1385:13 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) BRANCH crates/runner/src/significance.rs:1385:26 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) LINE   crates/runner/src/split.rs:385  [assert! message argument]  bar + horizon,
(F) BRANCH crates/runner/src/split.rs:513:21 False  [assert! condition branch (failing side)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
(F) BRANCH crates/runner/src/split.rs:573:21 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/split.rs:573:45 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/synthetic.rs:130:25 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:130:45 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:25 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:46 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:330:17 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) BRANCH crates/runner/src/synthetic.rs:330:27 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) LINE   crates/runner/src/trade.rs:1879  [unreachable! (failing arm)]  unreachable!("windows(2) always yields exactly two trades")
(F) LINE   crates/runner/src/trade.rs:2238  [assert! message argument]  t.count(),
(F) LINE   crates/runner/src/trade.rs:2382  [assert! message argument]  ist_minute_of(exit_ts)
(F) LINE   crates/runner/src/trade.rs:3131  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/trade.rs:1692:25 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:1692:48 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2086:21 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2086:48 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2205:17 False  [assert! condition branch (failing side)]  trade.entry_bar > previous_exit || n == 0,
(F) BRANCH crates/runner/src/trade.rs:2594:17 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2594:32 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2708:13 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:2708:32 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:3205:13 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3205:34 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3229:17 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3229:44 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3235:13 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade.rs:3235:42 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:13 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:43 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) LINE   crates/runner/src/validate.rs:6791  [assert_eq! message argument]  f.considered.saturating_sub(f.priced)
(F) LINE   crates/runner/src/validate.rs:7935  [assert! message argument]  body.len()
(F) LINE   crates/runner/src/validate.rs:7994  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/validate.rs:6850:13 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6850:29 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6850:50 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:7348:19 False  [assert! condition branch (failing side)]  }) if refused_fold == fold
(F) BRANCH crates/runner/src/validate.rs:7821:20 False  [assert! condition branch (failing side)]  if fold == fold_index
(F) BRANCH crates/runner/src/validate.rs:7851:24 False  [assert! condition branch (failing side)]  if fold == fold_index
(F) BRANCH crates/runner/src/validate.rs:7914:17 False  [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F) BRANCH crates/runner/src/validate.rs:7914:53 False  [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F) BRANCH crates/runner/src/validate.rs:7915:17 False  [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F) BRANCH crates/runner/src/validate.rs:7915:54 False  [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F) BRANCH crates/runner/src/validate.rs:8002:13 False  [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F) BRANCH crates/runner/src/validate.rs:8002:45 False  [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F) BRANCH crates/runner/src/validate.rs:8025:13 False  [assert! condition branch (failing side)]  !inner.contains("ExecutionRunV1::new(")
(F) BRANCH crates/runner/src/validate.rs:8026:20 False  [assert! condition branch (failing side)]  && !inner.contains("ExecutionDigestsV1::of(")
(F) BRANCH crates/runner/src/validate.rs:8027:20 False  [assert! condition branch (failing side)]  && inner.contains("ExecutionRunV1::with_digests(&run, &train_digests)"),
(F) BRANCH crates/runner/src/validate.rs:8053:17 False  [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F) BRANCH crates/runner/src/validate.rs:8053:60 False  [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F) BRANCH crates/runner/src/validate.rs:8055:17 False  [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F) BRANCH crates/runner/src/validate.rs:8055:56 False  [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F) BRANCH crates/runner/src/validate.rs:8091:13 False  [assert! condition branch (failing side)]  !inner.contains("data_digest_with_execution(")
(F) BRANCH crates/runner/src/validate.rs:8092:20 False  [assert! condition branch (failing side)]  && !inner.contains("ExecutionRunV1::new("),
(F) BRANCH crates/runner/src/validate.rs:8127:13 False  [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F) BRANCH crates/runner/src/validate.rs:8127:58 False  [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F) BRANCH crates/runner/src/validate.rs:8131:13 False  [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F) BRANCH crates/runner/src/validate.rs:8131:46 False  [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F) BRANCH crates/runner/src/validate.rs:8135:13 False  [assert! condition branch (failing side)]  !inner.contains("attest_training(")
(F) BRANCH crates/runner/src/validate.rs:8136:20 False  [assert! condition branch (failing side)]  && !inner.contains("evaluate_training_grid_attested("),
(F) BRANCH crates/runner/tests/end_to_end.rs:115:9 False  [assert! condition branch (failing side)]  exits.baseline().is_some() || exits.cells.is_empty(),
(F) BRANCH crates/runner/tests/end_to_end.rs:210:9 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
(F) BRANCH crates/runner/tests/end_to_end.rs:210:25 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:17 False  [assert! condition branch (failing side)]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:40 False  [assert! condition branch (failing side)]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:17 False  [assert! condition branch (failing side)]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:41 False  [assert! condition branch (failing side)]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:17 False  [assert! condition branch (failing side)]  trades.count() > 0 && trades.reconciles(),
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:39 False  [assert! condition branch (failing side)]  trades.count() > 0 && trades.reconciles(),
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:13 False  [assert! condition branch (failing side)]  !column.is_empty() && column.census().refused() == 0,
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:35 False  [assert! condition branch (failing side)]  !column.is_empty() && column.census().refused() == 0,
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:25 False  [assert! condition branch (failing side)]  assert!(trades.signals > 0 && trades.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:47 False  [assert! condition branch (failing side)]  assert!(trades.signals > 0 && trades.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:202:21 False  [assert! condition branch (failing side)]  trades.trades.is_empty()
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:203:28 False  [assert! condition branch (failing side)]  && trades.eligible.is_empty()
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:204:28 False  [assert! condition branch (failing side)]  && trades.occupancy.is_empty()
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:13 False  [assert! condition branch (failing side)]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:42 False  [assert! condition branch (failing side)]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:13 False  [assert! condition branch (failing side)]  assert!(trades.count() > 0 && trades.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:35 False  [assert! condition branch (failing side)]  assert!(trades.count() > 0 && trades.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:17 False  [assert! condition branch (failing side)]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:41 False  [assert! condition branch (failing side)]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:21 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:44 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:291:17 False  [assert! condition branch (failing side)]  refused.trades.is_empty()
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:292:24 False  [assert! condition branch (failing side)]  && refused.eligible.is_empty()
(F) BRANCH crates/runner/tests/fixed_deadline_readiness.rs:293:24 False  [assert! condition branch (failing side)]  && refused.occupancy.is_empty()
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:85:21 False  [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:85:44 False  [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:113:21 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:113:41 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:118:21 False  [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:118:49 False  [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:178:25 False  [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:178:45 False  [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:253:21 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:253:41 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:21 False  [assert! condition branch (failing side)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
(F) LINE   crates/runner/tests/hole_after_exit.rs:124  [assert! message argument]  probes.len()
(F) LINE   crates/runner/tests/join_answer_is_unchanged.rs:140  [assert_eq! message argument]  level.frequent.len(),
(F) LINE   crates/runner/tests/unwired_validation_record.rs:129  [assert! message argument]  path.display()

(U) ITEMS, each with the proof from the code:
(U) crates/runner/src/signal_candle_stop.rs:895:12 True, 895:51 True, 896 (`if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE { return Err(Error::Source); }` in Prepared::new). Proof: `facts` is SliceFacts::of(bars, &execution_column) and `execution_column` is the Some of `signal_column.reproject_checked(&aligned.onto, bars)` (signal_candle_stop.rs:889-892). reproject_checked (indicators column.rs:1000-1007) builds `accepted` with acceptance_of(execution = bars), which pushes exactly one verdict per bar (column.rs:314-336), and reproject_with returns it with `acceptance_known: true` and `sourced: Sourced::Fill` (column.rs:1094-1117). SliceFacts::of (trade.rs:511-) keeps column.acceptance() when its length is bars.len(), so covers(bars) is true; a Fill column gets Cadence::Fixed(EXECUTION_MINUTE_MICROS = 60_000_000), so step_micros() == MINUTE. Both operands are false for every value that reaches the line.
(U) crates/runner/src/signal_candle_stop.rs:1281:20 True, 1287:41 False, 1289 (`if !self.facts.exits().get(entry)...is_some_and(|exit| exit.real && exit.bar == index) { return Ok(Err(Reason::ClosingMinute)); }` in Prepared::walk). Proof: walk reaches a bar with ts == forced (the entry day's 15:09 open) only through `if bar.ts_micros != expected || !self.accepts(index) { return Ok(Err(Reason::PathRefused)) }` (signal_candle_stop.rs:1238-1240), so that bar is accepted by the evaluator, on the whole minute (SliceFacts::accepts) and not a duplicate (Prepared::accepts excludes any row with an adjacent equal stamp; validate_source refuses decreasing stamps, so equal stamps are adjacent). It is therefore the day's only raw 15:09 record and it is accepted with step 60_000_000, which is exactly the `proved` rule of SessionBounds::with_steps (outcome.rs:175-217), so day_ended is true; and its reverse traversal sets every same-day accepted bar's last_fill_bar to the latest fillable bar, which is that 15:09 bar (a later same-day bar ends after the 15:10 deadline and is not fillable, outcome.rs:262-271). forced_exits_with_steps (trade.rs:1641-1657) then gives exits()[entry] = Some(SquareOff { bar: index, real: true }), so the guard is true and `exit.bar == index` holds.
(U) crates/runner/src/signal_candle_stop.rs:1505 (`)?;` of `period.unavailable_minutes = add(...)` in period_geometry). Proof: `previous` is reset to None at each new IST day (line 1482), so the gaps added within one period sum to at most (last stamp - first stamp of that day) / MINUTE - 1 < 1440, and the other increments are one per bar; unavailable_minutes is therefore below bars.len() + 1440 and the u64 add cannot overflow. (The try_from on the same statement cannot fail either: distance > MINUTE there, so distance / MINUTE - 1 >= 0.)
(U) crates/runner/src/signal_candle_stop.rs:1345:12 True, 1346:12 True, 1347:12 True (`source.series.feed().is_empty()`, `commit().is_empty()`, `calendar_digest() == [0; 32]` in validate_source). Proof: ExecutionSeriesV1's fields are private to exit_grid_policy and its only constructor is ExecutionSeriesV1::new (exit_grid_policy.rs:209-231; one impl block, one `Self { .. }`; no struct literal elsewhere: `grep -n 'ExecutionSeriesV1 {' crates/` finds none outside the declaration), which refuses an empty feed, an empty commit and an all-zero calendar digest; Clone/Copy copy a value it built. No ExecutionSeriesV1 with those values exists.
(U) crates/runner/src/signal_candle_stop.rs:1349:12 True, 1350:12 True (`column.bits().len() != column.known().len()`, `column.len() != column.sources().len()`). Proof: Column's fields are private to indicators::column; every constructor keeps bits, known and source parallel: build_withholding_from pushes the three together (column.rs:705-716), reproject_with pushes the three together (column.rs:1077-1081), try_clone_exact copies each (column.rs:765-791), Default leaves all three empty, and clear_before / replace_gapfib / replace_orb_and_gapfib rewrite entries in place (replace_* refuse a different length, column.rs:894-895). len() is bits.len().
(U) crates/runner/src/signal_candle_stop.rs:1352:12 True (`column.evaluation_spec_token().is_none()`). Proof: the operand before it (`!column.acceptance_covers(signals.len())`) is true unless acceptance_known is true (column.rs:1137-1139); acceptance_known is true only from build_withholding_from, which stores evaluator.replay_spec(), Some for both evaluators (column.rs:193-195, 206-213), and from reproject_checked, which returns None unless self.spec is Some (column.rs:1005) and copies it; try_clone_exact copies both fields. So acceptance_known implies spec is Some.
(U) crates/runner/src/signal_candle_stop.rs:1358:12 True (`column.sources().windows(2).any(|pair| pair.first() >= pair.get(1))`). Proof: it is reached only when `column.sourced() == Sourced::Signal` (line 1348 refuses otherwise); a Signal column's sources come only from build_withholding_from, which pushes the kept index `index - withheld` of an enumerate walk that skips withheld bars, so it strictly increases (column.rs:688-716), and try_clone_exact, clear_before and the replace_* methods keep source unchanged. Only reproject_with (always Fill) writes other sources.
(U) crates/runner/src/signal_candle_stop.rs:1367:12 True (`!column.accepts(index)` for a row's source index). Proof: build_withholding_from pushes `source.push(index)` only in the `Ok(..) if warm` arm, right after `accepted.push(true)` for that same kept index (column.rs:706-716), so accepted[source] is true for every row of a Signal column; clear_before, try_clone_exact and replace_* never change `accepted`, and a Fill column is refused at line 1348 first.
GAP-FAULT crates/runner/src/signal_candle_stop.rs:939 (`)?;` of `periods_before = days_before(..)` in Prepared::new). Only an allocator refusal reaches it: line 935 has just run days_before over the same source span and succeeded, so the span cannot overflow; the count is at most periods.len() and the day at most last_day + 1 (validated to be at most i64::MAX / DAY), so neither checked add fails; the remaining error is `try_reserve_exact(span).map_err(|_| Error::Allocation)`, a fault no input or call can produce on demand. The recipe allows no (U) for a fault, so this row stays a GAP for the operator's ruling (no test can drive it without a failing allocator, and the workspace denies unsafe_code, which a test allocator would need).
(U) crates/runner/src/topn.rs:373:16 False, 376:16 False, 379:16 False, 382:16 False, 389:20 False (the `if let Some(slot) = bytes.get_mut(..)` arms of RankingPolicyV1::canonical_bytes). Proof: `bytes` is `[0_u8; RANKING_POLICY_CANONICAL_LEN_V1]` with the constant 76 (topn.rs:47); the four header ranges are the constants ..16, 16..20, 20..24, 24..28, and the weight loop runs over `self.weights.values()`, a `[u32; CRITERIA]` with CRITERIA = 11 (topn.rs:53, 107), so its ranges are 32 + 4i .. 36 + 4i for i in 0..11, the last ending at 76. Every range lies inside the array, so get_mut is Some on every call.
(U) crates/runner/src/bootstrap_zero_v2.rs:187:45 True, 191:12 True, 192 (`!summary.standard_error.is_finite()`, `!observed.is_finite()` in audit_sources), and crates/runner/src/bootstrap_family_pass.rs:461:20 True (`!stat.standard_error.is_finite()`), all on bootstrap::summarise of an i64 series. Proof: summarise (bootstrap.rs:1673-1695) is reached with n >= 2 periods; every |x as f64| <= 9.3e18, so mean is finite and each (x - mean)^2 <= 3.5e38, and the sum over n <= 2^64 terms is below 6.4e57: the variance and standard_error = sqrt(variance / n) are finite and non-negative, never NaN. These lines are reached only when standard_error > 0 (the first operand refuses <= 0). Then observed = mean / standard_error (studentized, bootstrap.rs:1725): a nonzero deviation of integer data from its f64 mean is at least about 5.4e-20 in magnitude (the mean is a nonzero integer sum over n <= 2^64, or an exact zero, and a difference of two doubles of magnitude >= 1 is a multiple of 2^-52), so variance >= 2.9e-39 / (n - 1), standard_error >= 5.4e-20 / n >= 2.9e-39, and |observed| <= 9.3e18 / 2.9e-39 < 3.3e57, finite.
(U) crates/runner/src/validate.rs:3771:13 False, 3772 (`let Some(outcome) = oos.get(ordinal) else { return Err(ProvenanceMismatch) }` in capture_anchored_fold). Proof: the function refuses at its first `if` unless `scored.len() == oos.len()` (line 3760), and the line before refuses unless `scored.get(ordinal)` is Some (3768), so ordinal < scored.len() == oos.len() and oos.get(ordinal) is Some. True for a direct call as well: the order of the two checks is fixed in the body.
(U) crates/runner/src/validate.rs:4123:9 False, 4124 (`let Ok(ordinal) = usize::try_from(choice.ordinal) else { return false }` in choice_matches_visible_v2), and every `usize::try_from(choice.ordinal).map_err(|_| ..StableWidth)?` of the V2/V3/V4 reconciliations that the figures show. Proof: choice.ordinal is a u64 and usize is 64 bits wide on aarch64-apple-darwin (and on this x86_64 host), so the conversion is total there; the arm exists for a 32-bit target, which the bar does not measure.
(U) crates/runner/src/exit_grid_policy.rs:2876 (`.ok_or(ExitGridErrorV1::InvalidOosBoundary { .. })?;` after `bars.get(first_oos)` in replay_selected_universe_with). Proof: OosExecutionSeriesV1's fields are private and its only constructor, OosExecutionSeriesV1::new (exit_grid_policy.rs:716-724), refuses `first_oos >= series.bars.len()`; `bars` here is that same series' bars (`oos.series().bars()`), so bars.get(first_oos) is always Some.
(U) crates/runner/src/exit_grid_policy.rs:2131:25 False (the `else { return false }` of `let Some(next) = pricing_refused.checked_add(1)` in evidence_reconciles). Proof: pricing_refused starts at 0 and gains 1 per candidate visited, so it never exceeds candidates.len(), which is below u64::MAX for any vector that fits in memory; checked_add cannot overflow.
`````

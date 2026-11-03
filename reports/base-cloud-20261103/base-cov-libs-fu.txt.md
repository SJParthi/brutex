# base-cov-libs-fu.txt.md

Source: `state/fix/round2/base-cov-libs-fu.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
BASE-COV-LIBS (F)/(U) list, attempt 1, measured on tip f01d7a0e430e (Route B, state/fix/round2/base-cov-libs-gaps-a1.txt). PROVISIONAL: the (F) rows are classified by rule and not yet each read by hand; the (U) rows were each read and proved by hand. Named in docs/05-decisions.md D-7320.

(F) ITEMS: 1572 (by rule: panic! (failing arm) 285; assert! condition branch (failing side) 849; assert! message argument 234; let-else in test 107; assert_eq! message argument 87; unreachable! (failing arm) 3; assert_eq! condition branch (failing side) 4; assert_ne! message argument 3)
Rule (recipe v4 step 4): the failing arm of assert!/assert_eq!/assert_ne!/panic!/unreachable! inside a #[test] body or a
#[cfg(test)] helper only tests call. Classified by brace-matching the enclosing macro inside test code (classify.mjs);
a row here that is NOT such an arm is a GAP, and a later attempt reviewing it moves it to the GAP list.

(F) LINE   crates/core/src/alias.rs:1241  [panic! (failing arm)]  other => panic!("{asked}: {other:?}"),
(F) LINE   crates/core/src/alias.rs:1285  [panic! (failing arm)]  other => panic!("{asked}: {other:?}"),
(F) BRANCH crates/core/src/alias.rs:1784:58 False  [assert! condition branch (failing side)]  assert!(matches!(got.via, Via::Index(row) if row.store_key == key));
(F) BRANCH crates/core/src/alias.rs:1793:61 False  [assert! condition branch (failing side)]  assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));
(F) BRANCH crates/core/src/alias.rs:1832:36 False  [assert! condition branch (failing side)]  assert!(hit || !name.is_empty());
(F) BRANCH crates/core/src/instrument.rs:779:17 False  [assert! condition branch (failing side)]  assert!(a < b && b < c);
(F) BRANCH crates/core/src/instrument.rs:779:26 False  [assert! condition branch (failing side)]  assert!(a < b && b < c);
(F) BRANCH crates/core/src/instrument.rs:1698:17 False  [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F) BRANCH crates/core/src/instrument.rs:1698:55 False  [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F) LINE   crates/core/src/universe.rs:5628  [assert! message argument]  u.bits()
(F) BRANCH crates/core/src/universe.rs:4755:17 False  [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F) BRANCH crates/core/src/universe.rs:4755:38 False  [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F) BRANCH crates/core/src/universe.rs:4755:60 False  [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F) BRANCH crates/core/src/universe.rs:4767:17 False  [assert! condition branch (failing side)]  assert!(tight.contains("X") && tight.contains("Y"));
(F) BRANCH crates/core/src/universe.rs:4767:40 False  [assert! condition branch (failing side)]  assert!(tight.contains("X") && tight.contains("Y"));
(F) BRANCH crates/core/src/universe.rs:4801:17 False  [assert! condition branch (failing side)]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:4801:42 False  [assert! condition branch (failing side)]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:4802:17 False  [assert! condition branch (failing side)]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:4802:48 False  [assert! condition branch (failing side)]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:4803:17 False  [assert! condition branch (failing side)]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:4803:48 False  [assert! condition branch (failing side)]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F) BRANCH crates/core/src/universe.rs:5622:17 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_50)
(F) BRANCH crates/core/src/universe.rs:5623:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_100)
(F) BRANCH crates/core/src/universe.rs:5624:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_200)
(F) BRANCH crates/core/src/universe.rs:5625:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_500)
(F) BRANCH crates/core/src/universe.rs:5626:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5634:17 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5634:52 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5786:17 False  [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F) BRANCH crates/core/src/universe.rs:5786:46 False  [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F) BRANCH crates/core/tests/banned_lists.rs:113:9 False  [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F) BRANCH crates/core/tests/banned_lists.rs:113:33 False  [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F) BRANCH crates/core/tests/claim_reader.rs:125:13 False  [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F) BRANCH crates/core/tests/claim_reader.rs:125:33 False  [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F) LINE   crates/core/tests/cost_invariants.rs:88  [panic! (failing arm)]  panic!(
(F) LINE   crates/core/tests/cost_invariants.rs:90  [panic! (failing arm)]  path.display()
(F) LINE   crates/core/tests/cost_invariants.rs:104  [panic! (failing arm)]  panic!(
(F) LINE   crates/core/tests/cost_invariants.rs:106  [panic! (failing arm)]  crates_dir.display()
(F) LINE   crates/core/tests/cost_invariants.rs:128  [assert! message argument]  crates_dir.display()
(F) LINE   crates/core/tests/cost_invariants.rs:353  [assert! message argument]  rows.len()
(F) LINE   crates/core/tests/cost_invariants.rs:360  [assert! message argument]  printed.len(),
(F) LINE   crates/core/tests/cost_invariants.rs:361  [assert! message argument]  benches.len()
(F) BRANCH crates/core/tests/cost_invariants.rs:116:13 False  [let-else in test]  let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
(F) BRANCH crates/core/tests/cost_invariants.rs:153:13 False  [let-else in test]  let Ok(files) = std::fs::read_dir(entry.path().join("benches")) else {
(F) BRANCH crates/core/tests/cost_invariants.rs:228:13 False  [let-else in test]  let Some(id) = rest.split(" |").next().map(str::trim) else {
(F) LINE   crates/core/tests/decision_citations.rs:243  [assert! message argument]  path.display()
(F) LINE   crates/core/tests/decision_citations.rs:303  [assert! message argument]  cited.len()
(F) LINE   crates/core/tests/decision_citations.rs:316  [assert! message argument]  missing.len()
(F) LINE   crates/core/tests/decision_citations.rs:412  [assert! message argument]  citations(&none[0].1)
(F) LINE   crates/core/tests/decision_citations.rs:536  [panic! (failing arm)]  panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
(F) LINE   crates/core/tests/decision_citations.rs:539  [panic! (failing arm)]  || panic!("{file} is not under a crate's src/"),
(F) LINE   crates/core/tests/findings.rs:93  [assert! message argument]  cells.len()
(F) LINE   crates/core/tests/findings.rs:295  [assert! message argument]  rows.len()
(F) LINE   crates/core/tests/findings.rs:335  [assert_eq! message argument]  counted + rest,
(F) LINE   crates/core/tests/findings.rs:336  [assert_eq! message argument]  rows.len()
(F) LINE   crates/core/tests/findings.rs:361  [assert_eq! message argument]  rows.len()
(F) LINE   crates/core/tests/findings.rs:449  [assert! message argument]  reason.len() >= 20,
(F) BRANCH crates/core/tests/findings.rs:366:13 False  [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,
(F) BRANCH crates/core/tests/findings.rs:366:33 False  [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,
(F) BRANCH crates/core/tests/findings.rs:613:13 False  [let-else in test]  let Ok(out) = out else {
(F) BRANCH crates/core/tests/findings.rs:713:13 False  [let-else in test]  let Ok(out) = out else {
(F) BRANCH crates/core/tests/findings.rs:801:13 False  [let-else in test]  let Ok(out) = out else {
(F) BRANCH crates/core/tests/graph.rs:528:33 False  [assert! condition branch (failing side)]  !deps.is_empty() || claim.to_lowercase().contains("nothing"),
(F) BRANCH crates/core/tests/graph.rs:563:44 False  [assert! condition branch (failing side)]  !name.contains('*') && !name.contains('/') && !name.is_empty(),
(F) LINE   crates/core/tests/lint.rs:139  [assert! message argument]  &next_fn[..next_fn.find('(').unwrap_or(0)],
(F) LINE   crates/core/tests/lint.rs:204  [assert! message argument]  reaches,
(F) LINE   crates/core/tests/lint.rs:209  [assert! message argument]  i + 1,
(F) LINE   crates/core/tests/zerodha_index.rs:60  [panic! (failing arm)]  panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
(F) BRANCH crates/core/tests/zerodha_index.rs:59:13 False  [let-else in test]  let Decoded::Keep(listing) = decoded else {
(F) BRANCH crates/costs/src/day.rs:897:25 False  [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F) BRANCH crates/costs/src/day.rs:897:57 False  [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F) LINE   crates/costs/src/fill.rs:1010  [assert! message argument]  best.buy().raw(),
(F) LINE   crates/costs/src/fill.rs:1011  [assert! message argument]  worst.buy().raw()
(F) LINE   crates/costs/src/fill.rs:1017  [assert! message argument]  best.sell().raw(),
(F) LINE   crates/costs/src/fill.rs:1018  [assert! message argument]  worst.sell().raw()
(F) LINE   crates/costs/src/ledger.rs:1334  [assert! message argument]  LINE_NAMES[at],
(F) LINE   crates/costs/src/ledger.rs:1335  [assert! message argument]  summed[at],
(F) LINE   crates/costs/src/ledger.rs:1336  [assert! message argument]  closed[at]
(F) LINE   crates/costs/src/lot.rs:1503  [panic! (failing arm)]  panic!("{subject} {on:?}: {:?}", got.basis);
(F) LINE   crates/costs/src/lot.rs:1595  [panic! (failing arm)]  panic!("{subject} {on:?}: an unread revision answered a lot");
(F) LINE   crates/costs/src/lot.rs:1619  [assert! message argument]  matches!(
(F) BRANCH crates/costs/src/lot.rs:1502:21 False  [let-else in test]  let LotBasis::ContractRule(citation) = got.basis else {
(F) BRANCH crates/costs/src/lot.rs:1559:25 False  [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F) BRANCH crates/costs/src/lot.rs:1559:57 False  [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F) BRANCH crates/costs/src/lot.rs:1594:17 False  [let-else in test]  let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
(F) LINE   crates/costs/src/lot_table_tests.rs:146  [panic! (failing arm)]  other => panic!("expected the unverified row, got {other:?}"),
(F) LINE   crates/costs/src/lot_table_tests.rs:156  [panic! (failing arm)]  other => panic!("expected before-first-row, got {other:?}"),
(F) LINE   crates/costs/src/lot_table_tests.rs:368  [assert! message argument]  l99 <= s99.saturating_mul(10).max(2_000),
(F) BRANCH crates/costs/src/lot_table_tests.rs:112:52 False  [assert! condition branch (failing side)]  assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
(F) BRANCH crates/costs/src/lot_table_tests.rs:117:25 False  [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F) BRANCH crates/costs/src/lot_table_tests.rs:117:56 False  [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F) BRANCH crates/costs/src/lot_table_tests.rs:187:56 False  [assert! condition branch (failing side)]  Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
(F) BRANCH crates/costs/src/money.rs:383:17 False  [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);
(F) BRANCH crates/costs/src/money.rs:383:31 False  [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);
(F) LINE   crates/costs/src/schedule.rs:1213  [assert! message argument]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1280  [assert! message argument]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1288  [assert! message argument]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1634  [panic! (failing arm)]  panic!("an options card carries option taxes");
(F) LINE   crates/costs/src/schedule.rs:1643  [panic! (failing arm)]  (card, trip) => panic!(
(F) BRANCH crates/costs/src/schedule.rs:1244:75 False  [assert! condition branch (failing side)]  matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
(F) BRANCH crates/costs/src/schedule.rs:1633:29 False  [let-else in test]  let Taxes::Options { stt_sell, .. } = card.taxes() else {
(F) LINE   crates/costs/src/scope.rs:249  [assert! message argument]  lines[arm]
(F) BRANCH crates/costs/src/stack.rs:2681:17 False  [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F) BRANCH crates/costs/src/stack.rs:2681:43 False  [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F) LINE   crates/costs/tests/adversarial.rs:109  [assert! message argument]  c.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:110  [assert! message argument]  f.raw()
(F) LINE   crates/costs/tests/adversarial.rs:116  [assert! message argument]  c.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:117  [assert! message argument]  f.raw()
(F) LINE   crates/costs/tests/adversarial.rs:128  [assert! message argument]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:135  [assert! message argument]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:143  [assert_eq! message argument]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:261  [assert! message argument]  n.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:262  [assert! message argument]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:302  [assert! message argument]  out.map(Paisa::raw)
(F) BRANCH crates/costs/tests/adversarial.rs:123:21 False  [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F) BRANCH crates/costs/tests/adversarial.rs:123:44 False  [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F) BRANCH crates/costs/tests/adversarial.rs:195:21 False  [assert! condition branch (failing side)]  ceil.is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:196:28 False  [assert! condition branch (failing side)]  && floor.is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:197:28 False  [assert! condition branch (failing side)]  && levy_ceiling(notional, rate).is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:198:28 False  [assert! condition branch (failing side)]  && statutory_levy(notional, rate).is_ok(),
(F) LINE   crates/costs/tests/dhan_calculator.rs:666  [assert! message argument]  case.page[7]
(F) LINE   crates/costs/tests/dhan_calculator.rs:926  [assert_eq! message argument]  case.name, dated_lines[at].0
(F) LINE   crates/costs/tests/fa73061.rs:603  [assert_eq! message argument]  RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),
(F) BRANCH crates/engine/src/keep.rs:825:17 False  [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F) BRANCH crates/engine/src/keep.rs:825:39 False  [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F) BRANCH crates/engine/src/keep.rs:832:13 False  [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F) BRANCH crates/engine/src/keep.rs:832:35 False  [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F) LINE   crates/engine/src/lib.rs:3270  [assert! message argument]  first.frequent.capacity(),
(F) LINE   crates/engine/src/lib.rs:3271  [assert! message argument]  repeated.len()
(F) BRANCH crates/engine/src/lib.rs:2845:13 False  [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,
(F) BRANCH crates/engine/src/lib.rs:2845:36 False  [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,
(F) BRANCH crates/engine/src/lib.rs:3002:48 True  [assert! condition branch (failing side)]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
(F) BRANCH crates/engine/src/lib.rs:3002:48 False  [assert! condition branch (failing side)]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
(F) BRANCH crates/engine/src/lib.rs:3210:17 False  [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3210:45 False  [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3232:17 False  [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F) BRANCH crates/engine/src/lib.rs:3232:46 False  [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F) BRANCH crates/engine/src/lib.rs:3287:17 False  [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3287:35 False  [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3289:13 False  [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,
(F) BRANCH crates/engine/src/lib.rs:3289:29 False  [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,
(F) BRANCH crates/engine/src/lib.rs:3303:17 False  [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());
(F) BRANCH crates/engine/src/lib.rs:3303:45 False  [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());
(F) BRANCH crates/engine/src/lib.rs:3607:13 False  [assert! condition branch (failing side)]  text.contains("excluded 3 0 AlwaysFalse")
(F) BRANCH crates/engine/src/lib.rs:3608:20 False  [assert! condition branch (failing side)]  && text.contains("excluded 9 8 AlwaysTrue")
(F) BRANCH crates/engine/src/lib.rs:3609:20 False  [assert! condition branch (failing side)]  && text.contains("excluded 240 unmeasured NotLive"),
(F) BRANCH crates/engine/src/lib.rs:3929:13 False  [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),
(F) BRANCH crates/engine/src/lib.rs:3929:34 False  [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),
(F) BRANCH crates/engine/src/lib.rs:5229:17 False  [let-else in test]  let (Some(&(_, a0, p0)), Some(&(_, a1, p1))) = (pair.first(), pair.get(1)) else {
(F) BRANCH crates/engine/src/lib.rs:5560:21 False  [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F) BRANCH crates/engine/src/lib.rs:5560:34 False  [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F) LINE   crates/engine/tests/production_callers.rs:198  [assert! message argument]  file.path.display(),
(F) LINE   crates/engine/tests/production_callers.rs:199  [assert! message argument]  at + 1
(F) BRANCH crates/engine/tests/production_callers.rs:47:13 False  [let-else in test]  let Ok(entries) = std::fs::read_dir(dir) else {
(F) BRANCH crates/engine/tests/production_callers.rs:138:9 False  [assert! condition branch (failing side)]  engine > 1 && others > 1,
(F) BRANCH crates/engine/tests/production_callers.rs:138:23 False  [assert! condition branch (failing side)]  engine > 1 && others > 1,
(F) LINE   crates/engine/tests/resume_readiness.rs:76  [assert! message argument]  bytes.get(56..192).expect("header")
(F) LINE   crates/engine/tests/resume_readiness.rs:264  [assert! message argument]  ()
(F) LINE   crates/engine/tests/resume_readiness.rs:276  [assert! message argument]  &mut |_| Ok(())
(F) LINE   crates/engine/tests/resume_readiness.rs:288  [assert! message argument]  &mut |_| Ok(())
(F) BRANCH crates/engine/tests/resume_readiness.rs:96:67 False  [assert! condition branch (failing side)]  matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
(F) BRANCH crates/engine/tests/resume_readiness.rs:592:24 False  [assert! condition branch (failing side)]  .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
(F) BRANCH crates/engine/tests/resume_readiness.rs:814:65 False  [assert! condition branch (failing side)]  assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
(F) BRANCH crates/engine/tests/resume_readiness.rs:981:13 False  [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F) BRANCH crates/engine/tests/resume_readiness.rs:981:37 False  [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F) BRANCH crates/engine/tests/sweep_readiness_oracle.rs:159:48 False  [assert! condition branch (failing side)]  .all(|w| matches!(w, [x, y] if x.mask.words() < y.mask.words())))
(F) LINE   crates/greeks/src/solver.rs:1079  [assert! message argument]  BISECTION_STEPS - 1
(F) LINE   crates/greeks/src/solver.rs:1135  [assert! message argument]  (high - low) / ulp(low)
(F) BRANCH crates/greeks/src/solver.rs:1133:21 False  [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),
(F) BRANCH crates/greeks/src/solver.rs:1133:36 False  [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),
(F) BRANCH crates/greeks/src/solver.rs:1140:48 False  [assert! condition branch (failing side)]  found.volatility == low || found.volatility == high,
(F) LINE   crates/greeks/tests/vendor_anchor.rs:524  [assert! message argument]  rate_disagreement(root)
(F) BRANCH crates/indicators/src/anchored.rs:2177:28 False  [assert! condition branch (failing side)]  if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
(F) BRANCH crates/indicators/src/anchored.rs:2731:17 False  [assert! condition branch (failing side)]  accepted > 500 && refused > 500,
(F) BRANCH crates/indicators/src/anchored.rs:2731:35 False  [assert! condition branch (failing side)]  accepted > 500 && refused > 500,
(F) BRANCH crates/indicators/src/column.rs:2599:13 False  [assert! condition branch (failing side)]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F) BRANCH crates/indicators/src/column.rs:2599:50 False  [assert! condition branch (failing side)]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F) BRANCH crates/indicators/src/daily.rs:869:17 False  [assert! condition branch (failing side)]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F) BRANCH crates/indicators/src/daily.rs:869:31 False  [assert! condition branch (failing side)]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F) BRANCH crates/indicators/src/daily.rs:1114:17 False  [assert! condition branch (failing side)]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) BRANCH crates/indicators/src/daily.rs:1114:30 False  [assert! condition branch (failing side)]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) BRANCH crates/indicators/src/daily.rs:1114:43 False  [assert! condition branch (failing side)]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) BRANCH crates/indicators/src/daily.rs:1115:17 False  [assert! condition branch (failing side)]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F) BRANCH crates/indicators/src/daily.rs:1115:31 False  [assert! condition branch (failing side)]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F) BRANCH crates/indicators/src/daily.rs:1116:17 False  [assert! condition branch (failing side)]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F) BRANCH crates/indicators/src/daily.rs:1116:30 False  [assert! condition branch (failing side)]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F) BRANCH crates/indicators/src/daily.rs:1137:17 False  [assert! condition branch (failing side)]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1137:30 False  [assert! condition branch (failing side)]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1137:43 False  [assert! condition branch (failing side)]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1138:17 False  [assert! condition branch (failing side)]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1138:30 False  [assert! condition branch (failing side)]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1138:43 False  [assert! condition branch (failing side)]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1139:17 False  [assert! condition branch (failing side)]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1139:31 False  [assert! condition branch (failing side)]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1140:17 False  [assert! condition branch (failing side)]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1140:31 False  [assert! condition branch (failing side)]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F) BRANCH crates/indicators/src/daily.rs:1141:17 False  [assert! condition branch (failing side)]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F) BRANCH crates/indicators/src/daily.rs:1141:30 False  [assert! condition branch (failing side)]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F) BRANCH crates/indicators/src/daily.rs:1554:13 False  [assert! condition branch (failing side)]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F) BRANCH crates/indicators/src/daily.rs:1554:44 False  [assert! condition branch (failing side)]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F) BRANCH crates/indicators/src/daily.rs:1573:13 False  [assert! condition branch (failing side)]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F) BRANCH crates/indicators/src/daily.rs:1573:47 False  [assert! condition branch (failing side)]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F) BRANCH crates/indicators/src/evaluator.rs:1586:78 False  [assert! condition branch (failing side)]  Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected
(F) BRANCH crates/indicators/src/evaluator.rs:1888:17 False  [assert! condition branch (failing side)]  assert!(truth_only.warmed_up() && full.warmed_up());
(F) BRANCH crates/indicators/src/evaluator.rs:1888:43 False  [assert! condition branch (failing side)]  assert!(truth_only.warmed_up() && full.warmed_up());
(F) BRANCH crates/indicators/src/evaluator.rs:1979:17 False  [assert! condition branch (failing side)]  !first.get(above) && !first.get(below),
(F) BRANCH crates/indicators/src/evaluator.rs:1979:38 False  [assert! condition branch (failing side)]  !first.get(above) && !first.get(below),
(F) BRANCH crates/indicators/src/evaluator.rs:3561:13 False  [assert! condition branch (failing side)]  !first.get(276) && !first.get(277),
(F) BRANCH crates/indicators/src/evaluator.rs:3561:32 False  [assert! condition branch (failing side)]  !first.get(276) && !first.get(277),
(F) BRANCH crates/indicators/src/evaluator.rs:3585:13 False  [assert! condition branch (failing side)]  !flat.get(276) && !flat.get(277),
(F) BRANCH crates/indicators/src/evaluator.rs:3585:31 False  [assert! condition branch (failing side)]  !flat.get(276) && !flat.get(277),
(F) BRANCH crates/indicators/src/evaluator.rs:3645:36 True  [assert! condition branch (failing side)]  !(mask.get(278) && mask.get(279)),
(F) BRANCH crates/indicators/src/fib.rs:584:21 False  [assert! condition branch (failing side)]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F) BRANCH crates/indicators/src/fib.rs:584:31 False  [assert! condition branch (failing side)]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F) BRANCH crates/indicators/src/gap.rs:1491:13 False  [assert! condition branch (failing side)]  fired > 0 && legs > 0,
(F) BRANCH crates/indicators/src/gap.rs:1491:26 False  [assert! condition branch (failing side)]  fired > 0 && legs > 0,
(F) BRANCH crates/indicators/src/lib.rs:883:17 True  [assert! condition branch (failing side)]  assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
(F) BRANCH crates/indicators/src/lib.rs:883:36 False  [assert! condition branch (failing side)]  assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
(F) BRANCH crates/indicators/src/orb.rs:555:13 False  [assert! condition branch (failing side)]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F) BRANCH crates/indicators/src/orb.rs:555:40 False  [assert! condition branch (failing side)]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F) BRANCH crates/indicators/src/orb.rs:559:13 False  [assert! condition branch (failing side)]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F) BRANCH crates/indicators/src/orb.rs:559:40 False  [assert! condition branch (failing side)]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F) BRANCH crates/indicators/src/orb.rs:610:17 False  [assert! condition branch (failing side)]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) BRANCH crates/indicators/src/orb.rs:610:40 False  [assert! condition branch (failing side)]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) BRANCH crates/indicators/src/orb.rs:610:63 False  [assert! condition branch (failing side)]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) BRANCH crates/indicators/src/pattern.rs:1558:17 False  [assert! condition branch (failing side)]  assert!(shape.bullish() && !shape.bearish());
(F) BRANCH crates/indicators/src/pattern.rs:1558:36 False  [assert! condition branch (failing side)]  assert!(shape.bullish() && !shape.bearish());
(F) BRANCH crates/indicators/src/pattern.rs:1562:17 False  [assert! condition branch (failing side)]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F) BRANCH crates/indicators/src/pattern.rs:1562:44 False  [assert! condition branch (failing side)]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F) BRANCH crates/indicators/src/pattern.rs:1597:13 False  [assert! condition branch (failing side)]  m2.get(153) && m2.get(155),
(F) BRANCH crates/indicators/src/pattern.rs:1597:28 False  [assert! condition branch (failing side)]  m2.get(153) && m2.get(155),
(F) BRANCH crates/indicators/src/pattern.rs:1711:41 True  [assert! condition branch (failing side)]  !(mask.get(bull) && mask.get(bear)),
(F) BRANCH crates/indicators/src/session.rs:586:17 False  [assert! condition branch (failing side)]  assert!(truth.get(66) && !truth.get(67));
(F) BRANCH crates/indicators/src/session.rs:586:34 False  [assert! condition branch (failing side)]  assert!(truth.get(66) && !truth.get(67));
(F) BRANCH crates/indicators/src/session.rs:587:17 False  [assert! condition branch (failing side)]  assert!(known.get(66) && known.get(67));
(F) BRANCH crates/indicators/src/session.rs:587:34 False  [assert! condition branch (failing side)]  assert!(known.get(66) && known.get(67));
(F) BRANCH crates/indicators/src/session.rs:589:13 False  [assert! condition branch (failing side)]  !truth.get(68) && !known.get(68),
(F) BRANCH crates/indicators/src/session.rs:589:31 False  [assert! condition branch (failing side)]  !truth.get(68) && !known.get(68),
(F) BRANCH crates/indicators/src/session.rs:655:13 False  [assert! condition branch (failing side)]  !bits.get(42) && !bits.get(43),
(F) BRANCH crates/indicators/src/session.rs:655:30 False  [assert! condition branch (failing side)]  !bits.get(42) && !bits.get(43),
(F) BRANCH crates/indicators/src/session.rs:680:13 False  [assert! condition branch (failing side)]  !edge.get(44) && edge.get(45),
(F) BRANCH crates/indicators/src/session.rs:680:30 False  [assert! condition branch (failing side)]  !edge.get(44) && edge.get(45),
(F) BRANCH crates/indicators/src/session.rs:788:17 False  [assert! condition branch (failing side)]  assert!(up.get(30) && !up.get(31));
(F) BRANCH crates/indicators/src/session.rs:788:31 False  [assert! condition branch (failing side)]  assert!(up.get(30) && !up.get(31));
(F) BRANCH crates/indicators/src/session.rs:790:17 False  [assert! condition branch (failing side)]  assert!(down.get(31) && !down.get(30));
(F) BRANCH crates/indicators/src/session.rs:790:33 False  [assert! condition branch (failing side)]  assert!(down.get(31) && !down.get(30));
(F) BRANCH crates/indicators/src/session.rs:793:13 False  [assert! condition branch (failing side)]  !flat.get(30) && !flat.get(31),
(F) BRANCH crates/indicators/src/session.rs:793:30 False  [assert! condition branch (failing side)]  !flat.get(30) && !flat.get(31),
(F) BRANCH crates/indicators/src/session.rs:838:17 False  [assert! condition branch (failing side)]  assert!(!mask.get(37) && !mask.get(38));
(F) BRANCH crates/indicators/src/session.rs:838:34 False  [assert! condition branch (failing side)]  assert!(!mask.get(37) && !mask.get(38));
(F) BRANCH crates/indicators/src/session.rs:858:17 False  [assert! condition branch (failing side)]  !mask.get(30) && !mask.get(31),
(F) BRANCH crates/indicators/src/session.rs:858:34 False  [assert! condition branch (failing side)]  !mask.get(30) && !mask.get(31),
(F) BRANCH crates/indicators/src/session.rs:922:13 False  [assert! condition branch (failing side)]  !flat.get(30) && !flat.get(31),
(F) BRANCH crates/indicators/src/session.rs:922:30 False  [assert! condition branch (failing side)]  !flat.get(30) && !flat.get(31),
(F) BRANCH crates/indicators/src/session.rs:931:17 False  [assert! condition branch (failing side)]  assert!(!mask.get(37) && !mask.get(38));
(F) BRANCH crates/indicators/src/session.rs:931:34 False  [assert! condition branch (failing side)]  assert!(!mask.get(37) && !mask.get(38));
(F) BRANCH crates/indicators/src/session.rs:950:13 False  [assert! condition branch (failing side)]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) BRANCH crates/indicators/src/session.rs:950:30 False  [assert! condition branch (failing side)]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) BRANCH crates/indicators/src/session.rs:950:47 False  [assert! condition branch (failing side)]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) BRANCH crates/indicators/src/session.rs:966:17 False  [assert! condition branch (failing side)]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F) BRANCH crates/indicators/src/session.rs:966:33 False  [assert! condition branch (failing side)]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F) BRANCH crates/indicators/src/session.rs:968:17 False  [assert! condition branch (failing side)]  assert!(mask.get(66) && !mask.get(67));
(F) BRANCH crates/indicators/src/session.rs:968:33 False  [assert! condition branch (failing side)]  assert!(mask.get(66) && !mask.get(67));
(F) BRANCH crates/indicators/src/session.rs:972:17 False  [assert! condition branch (failing side)]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F) BRANCH crates/indicators/src/session.rs:972:34 False  [assert! condition branch (failing side)]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F) BRANCH crates/indicators/src/session.rs:974:13 False  [assert! condition branch (failing side)]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) BRANCH crates/indicators/src/session.rs:974:30 False  [assert! condition branch (failing side)]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) BRANCH crates/indicators/src/session.rs:974:47 False  [assert! condition branch (failing side)]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) BRANCH crates/indicators/src/session.rs:989:17 False  [assert! condition branch (failing side)]  assert!(mask.get(50) && !mask.get(51));
(F) BRANCH crates/indicators/src/session.rs:989:33 False  [assert! condition branch (failing side)]  assert!(mask.get(50) && !mask.get(51));
(F) BRANCH crates/indicators/src/session.rs:993:17 False  [assert! condition branch (failing side)]  assert!(mask.get(51) && !mask.get(50));
(F) BRANCH crates/indicators/src/session.rs:993:33 False  [assert! condition branch (failing side)]  assert!(mask.get(51) && !mask.get(50));
(F) BRANCH crates/indicators/src/session.rs:1283:13 False  [assert! condition branch (failing side)]  !mask.get(66) && !mask.get(67),
(F) BRANCH crates/indicators/src/session.rs:1283:30 False  [assert! condition branch (failing side)]  !mask.get(66) && !mask.get(67),
(F) BRANCH crates/indicators/src/trend.rs:1292:13 False  [assert! condition branch (failing side)]  !mask.get(0) && !mask.get(1),
(F) BRANCH crates/indicators/src/trend.rs:1292:29 False  [assert! condition branch (failing side)]  !mask.get(0) && !mask.get(1),
(F) BRANCH crates/indicators/src/trend.rs:1296:13 False  [assert! condition branch (failing side)]  !mask.get(2) && !mask.get(3),
(F) BRANCH crates/indicators/src/trend.rs:1296:29 False  [assert! condition branch (failing side)]  !mask.get(2) && !mask.get(3),
(F) BRANCH crates/indicators/src/trend.rs:1300:13 False  [assert! condition branch (failing side)]  !mask.get(4) && !mask.get(5),
(F) BRANCH crates/indicators/src/trend.rs:1300:29 False  [assert! condition branch (failing side)]  !mask.get(4) && !mask.get(5),
(F) BRANCH crates/indicators/src/trend.rs:2296:74 False  [assert! condition branch (failing side)]  matches!((previous, now), (Some(before), Some(after)) if after <= before),
(F) BRANCH crates/indicators/src/vwap.rs:776:17 False  [assert! condition branch (failing side)]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F) BRANCH crates/indicators/src/vwap.rs:776:34 False  [assert! condition branch (failing side)]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F) BRANCH crates/indicators/src/vwap.rs:777:17 False  [assert! condition branch (failing side)]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F) BRANCH crates/indicators/src/vwap.rs:777:34 False  [assert! condition branch (failing side)]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F) BRANCH crates/indicators/src/vwap.rs:889:17 False  [assert! condition branch (failing side)]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F) BRANCH crates/indicators/src/vwap.rs:889:44 False  [assert! condition branch (failing side)]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F) BRANCH crates/indicators/src/vwap.rs:1235:21 False  [assert! condition branch (failing side)]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F) BRANCH crates/indicators/src/vwap.rs:1235:52 False  [assert! condition branch (failing side)]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F) BRANCH crates/indicators/tests/crossing_known_readiness.rs:84:32 True  [assert! condition branch (failing side)]  !(above && below),
(F) BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:212:13 False  [assert! condition branch (failing side)]  false_count > 0 && true_count > 0,
(F) BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:212:32 False  [assert! condition branch (failing side)]  false_count > 0 && true_count > 0,
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:13 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:26 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:40 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:13 False  [assert! condition branch (failing side)]  !truth.get(bit) && !known.get(bit),
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:32 False  [assert! condition branch (failing side)]  !truth.get(bit) && !known.get(bit),
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:17 False  [assert! condition branch (failing side)]  !truth.get(bit) && !known.get(bit),
(F) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:36 False  [assert! condition branch (failing side)]  !truth.get(bit) && !known.get(bit),
(F) BRANCH crates/indicators/tests/fib_known_readiness.rs:174:13 False  [assert! condition branch (failing side)]  !truth.get(retired) && !known.get(retired),
(F) BRANCH crates/indicators/tests/fib_known_readiness.rs:174:36 False  [assert! condition branch (failing side)]  !truth.get(retired) && !known.get(retired),
(F) BRANCH crates/indicators/tests/fib_known_readiness.rs:229:13 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0,
(F) BRANCH crates/indicators/tests/fib_known_readiness.rs:229:26 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0,
(F) BRANCH crates/indicators/tests/gap_known_readiness.rs:216:21 False  [assert! condition branch (failing side)]  assert!(false_count > 0 && true_count > 0);
(F) BRANCH crates/indicators/tests/gap_known_readiness.rs:216:40 False  [assert! condition branch (failing side)]  assert!(false_count > 0 && true_count > 0);
(F) BRANCH crates/indicators/tests/gap_known_readiness.rs:256:17 False  [assert! condition branch (failing side)]  assert!(false_count > 0 && true_count > 0);
(F) BRANCH crates/indicators/tests/gap_known_readiness.rs:256:36 False  [assert! condition branch (failing side)]  assert!(false_count > 0 && true_count > 0);
(F) LINE   crates/indicators/tests/module_doc_counts.rs:271  [assert_eq! message argument]  positions.len()
(F) LINE   crates/indicators/tests/module_doc_counts.rs:321  [assert_eq! message argument]  positions.len()
(F) LINE   crates/indicators/tests/module_doc_counts.rs:332  [assert_eq! message argument]  emitted.len()
(F) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:13 False  [assert! condition branch (failing side)]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:32 False  [assert! condition branch (failing side)]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:50 False  [assert! condition branch (failing side)]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) BRANCH crates/indicators/tests/session_from_outside.rs:200:9 False  [assert! condition branch (failing side)]  mask.get(46) && !mask.get(47),
(F) BRANCH crates/indicators/tests/session_from_outside.rs:200:25 False  [assert! condition branch (failing side)]  mask.get(46) && !mask.get(47),
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:360:13 False  [assert! condition branch (failing side)]  !truth.get(position) && !known.get(position),
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:360:37 False  [assert! condition branch (failing side)]  !truth.get(position) && !known.get(position),
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:364:13 False  [assert! condition branch (failing side)]  assert!(known.get(48) && known.get(49));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:364:30 False  [assert! condition branch (failing side)]  assert!(known.get(48) && known.get(49));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:365:13 False  [assert! condition branch (failing side)]  assert!(!truth.get(48) && !truth.get(49));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:365:31 False  [assert! condition branch (failing side)]  assert!(!truth.get(48) && !truth.get(49));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:452:13 False  [assert! condition branch (failing side)]  assert!(known.get(33) && !truth.get(33));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:452:30 False  [assert! condition branch (failing side)]  assert!(known.get(33) && !truth.get(33));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:453:13 False  [assert! condition branch (failing side)]  assert!(!known.get(37) && !known.get(66));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:453:31 False  [assert! condition branch (failing side)]  assert!(!known.get(37) && !known.get(66));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:504:13 False  [assert! condition branch (failing side)]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:504:30 False  [assert! condition branch (failing side)]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) BRANCH crates/indicators/tests/session_known_readiness.rs:504:47 False  [assert! condition branch (failing side)]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:9 False  [assert! condition branch (failing side)]  hammers > 0 && hammers < cases,
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:24 False  [assert! condition branch (failing side)]  hammers > 0 && hammers < cases,
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:13 False  [assert! condition branch (failing side)]  assert!(bullish > 0 && bearish > 0);
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:28 False  [assert! condition branch (failing side)]  assert!(bullish > 0 && bearish > 0);
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:17 False  [assert! condition branch (failing side)]  assert!(!truth.get(30) && !truth.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:35 False  [assert! condition branch (failing side)]  assert!(!truth.get(30) && !truth.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:17 False  [assert! condition branch (failing side)]  assert!(known.get(30) && known.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:34 False  [assert! condition branch (failing side)]  assert!(known.get(30) && known.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:17 False  [assert! condition branch (failing side)]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:34 False  [assert! condition branch (failing side)]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:51 False  [assert! condition branch (failing side)]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:68 False  [assert! condition branch (failing side)]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:9 False  [assert! condition branch (failing side)]  !next_day.get(157) && !next_day.get(176),
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:31 False  [assert! condition branch (failing side)]  !next_day.get(157) && !next_day.get(176),
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:9 False  [assert! condition branch (failing side)]  !next_day.get(276) && !next_day.get(277),
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:31 False  [assert! condition branch (failing side)]  !next_day.get(276) && !next_day.get(277),
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:17 False  [assert! condition branch (failing side)]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:47 False  [assert! condition branch (failing side)]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:21 False  [assert! condition branch (failing side)]  assert!(known.get(30) && known.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:38 False  [assert! condition branch (failing side)]  assert!(known.get(30) && known.get(31));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:17 False  [assert! condition branch (failing side)]  assert!(!truth.get(13) && !truth.get(16));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:35 False  [assert! condition branch (failing side)]  assert!(!truth.get(13) && !truth.get(16));
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:13 False  [assert! condition branch (failing side)]  known.get(13) && known.get(16),
(F) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:30 False  [assert! condition branch (failing side)]  known.get(13) && known.get(16),
(F) BRANCH crates/indicators/tests/trend_known_readiness.rs:177:45 False  [assert! condition branch (failing side)]  assert!(!truth.get(position) || known.get(position));
(F) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:13 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:26 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:40 False  [assert! condition branch (failing side)]  trues > 0 && falses > 0 && unknowns > 0,
(F) BRANCH crates/indicators/tests/withheld_fold.rs:147:13 False  [let-else in test]  let Some(old) = spliced_at(source) else {
(F) LINE   crates/lake/src/bar.rs:237  [panic! (failing arm)]  other => panic!("expected NotRepresentable, got {other:?}"),
(F) LINE   crates/lake/src/bar.rs:247  [panic! (failing arm)]  other => panic!("expected NotRepresentable, got {other:?}"),
(F) LINE   crates/lake/src/contract.rs:414  [panic! (failing arm)]  ContractKind::Future => panic!("a CE is not a future"),
(F) LINE   crates/lake/src/contract.rs:565  [panic! (failing arm)]  other => panic!("expected TooLong, got {other:?}"),
(F) LINE   crates/lake/src/contract.rs:573  [assert! message argument]  name.len()
(F) LINE   crates/lake/src/contract.rs:616  [panic! (failing arm)]  ContractKind::Future => panic!("not a future"),
(F) LINE   crates/lake/src/contract.rs:689  [panic! (failing arm)]  other => panic!("{padded} must be refused, got {other:?}"),
(F) LINE   crates/lake/src/contract.rs:723  [panic! (failing arm)]  other => panic!("{signed} must be refused, got {other:?}"),
(F) LINE   crates/lake/src/contract.rs:755  [panic! (failing arm)]  other => panic!("{name} must be refused, got {other:?}"),
(F) LINE   crates/lake/src/footer_tests.rs:33  [panic! (failing arm)]  panic!("expected the walk to admit, got {e:?}");
(F) LINE   crates/lake/src/footer_tests.rs:40  [panic! (failing arm)]  other => panic!("expected FooterUnreadable, got {other:?}"),
(F) LINE   crates/lake/src/footer_tests.rs:69  [panic! (failing arm)]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F) LINE   crates/lake/src/footer_tests.rs:80  [panic! (failing arm)]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F) LINE   crates/lake/src/page.rs:878  [panic! (failing arm)]  other => panic!("expected a data page, got {other:?}"),
(F) LINE   crates/lake/src/page.rs:986  [assert! message argument]  frame.len(),
(F) LINE   crates/lake/src/page.rs:987  [assert! message argument]  plain.len()
(F) LINE   crates/lake/src/page.rs:1034  [panic! (failing arm)]  other => panic!("expected a data page, got {other:?}"),
(F) LINE   crates/lake/src/page.rs:1102  [assert! message argument]  frame.len()
(F) BRANCH crates/lake/src/page.rs:631:13 False  [assert! condition branch (failing side)]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F) BRANCH crates/lake/src/page.rs:631:54 False  [assert! condition branch (failing side)]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F) BRANCH crates/lake/src/page.rs:640:13 False  [assert! condition branch (failing side)]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F) BRANCH crates/lake/src/page.rs:640:53 False  [assert! condition branch (failing side)]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F) BRANCH crates/lake/src/page.rs:681:13 False  [assert! condition branch (failing side)]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F) BRANCH crates/lake/src/page.rs:681:65 False  [assert! condition branch (failing side)]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F) BRANCH crates/lake/src/page.rs:741:13 False  [assert! condition branch (failing side)]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F) BRANCH crates/lake/src/page.rs:741:51 False  [assert! condition branch (failing side)]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F) BRANCH crates/lake/src/page.rs:1147:13 False  [assert! condition branch (failing side)]  text.contains("zstd frame refused") && text.contains("134217729"),
(F) BRANCH crates/lake/src/page.rs:1147:52 False  [assert! condition branch (failing side)]  text.contains("zstd frame refused") && text.contains("134217729"),
(F) LINE   crates/lake/src/reader.rs:899  [panic! (failing arm)]  other => panic!("expected TooLarge, got {:?}", other.err()),
(F) LINE   crates/lake/src/reader.rs:911  [assert! message argument]  matches!(
(F) LINE   crates/lake/src/reader.rs:918  [assert! message argument]  matches!(
(F) LINE   crates/lake/src/reader.rs:952  [panic! (failing arm)]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F) LINE   crates/lake/src/reader.rs:965  [panic! (failing arm)]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F) LINE   crates/lake/src/reader.rs:978  [panic! (failing arm)]  other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
(F) LINE   crates/lake/src/reader.rs:1052  [panic! (failing arm)]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1098  [panic! (failing arm)]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1115  [panic! (failing arm)]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1145  [panic! (failing arm)]  other => panic!("expected UnexpectedNull, got {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1280  [panic! (failing arm)]  _ => panic!("the cash layout is INT64 and DOUBLE only"),
(F) LINE   crates/lake/src/reader.rs:1387  [panic! (failing arm)]  other => panic!("expected ImpossibleLength, got {:?}", other.err()),
(F) LINE   crates/lake/src/reader.rs:1421  [panic! (failing arm)]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1422  [panic! (failing arm)]  Ok(batch) => panic!(
(F) LINE   crates/lake/src/reader.rs:1424  [panic! (failing arm)]  batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()
(F) LINE   crates/lake/src/reader.rs:1478  [panic! (failing arm)]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1479  [panic! (failing arm)]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F) LINE   crates/lake/src/reader.rs:1529  [panic! (failing arm)]  other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1553  [panic! (failing arm)]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1554  [panic! (failing arm)]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F) LINE   crates/lake/src/reader.rs:1625  [panic! (failing arm)]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1626  [panic! (failing arm)]  Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),
(F) LINE   crates/lake/src/reader.rs:1726  [panic! (failing arm)]  other => panic!("the padded group was not refused as short: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1757  [panic! (failing arm)]  panic!("the cut and its copy sum to the file count")
(F) LINE   crates/lake/src/reader.rs:1800  [panic! (failing arm)]  other => panic!("group {group} under {claim}: {other:?}"),
(F) LINE   crates/lake/src/reader.rs:1883  [panic! (failing arm)]  crate::error::ColumnType::Other => panic!("no lake column is Other"),
(F) LINE   crates/lake/src/reader.rs:1924  [panic! (failing arm)]  _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),
(F) LINE   crates/lake/tests/events.rs:159  [panic! (failing arm)]  _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
(F) LINE   crates/lake/tests/events.rs:366  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/events.rs:401  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/real_lake.rs:75  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/real_lake.rs:114  [assert_eq! message argument]  g.gamma.to_bits(),
(F) LINE   crates/lake/tests/real_lake.rs:115  [assert_eq! message argument]  0.000_171_426_804_295_494_02_f64.to_bits()
(F) LINE   crates/lake/tests/real_lake.rs:121  [assert_eq! message argument]  g.t_years_used.to_bits(),
(F) LINE   crates/lake/tests/real_lake.rs:122  [assert_eq! message argument]  0.033_280_060_882_800_61_f64.to_bits()
(F) LINE   crates/lake/tests/real_lake.rs:134  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/real_lake.rs:182  [panic! (failing arm)]  .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));
(F) LINE   crates/lake/tests/real_lake.rs:184  [assert_eq! message argument]  parsed.to_string(),
(F) LINE   crates/lake/tests/real_lake.rs:224  [panic! (failing arm)]  .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));
(F) LINE   crates/lake/tests/real_lake.rs:228  [panic! (failing arm)]  .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));
(F) BRANCH crates/lake/tests/real_lake.rs:74:9 True  [let-else in test]  let Some(path) = lake_path(SAMPLE_FNO) else {
(F) BRANCH crates/lake/tests/real_lake.rs:74:9 False  [let-else in test]  let Some(path) = lake_path(SAMPLE_FNO) else {
(F) BRANCH crates/lake/tests/real_lake.rs:133:9 True  [let-else in test]  let Some(path) = lake_path(SAMPLE_CASH) else {
(F) BRANCH crates/lake/tests/real_lake.rs:133:9 False  [let-else in test]  let Some(path) = lake_path(SAMPLE_CASH) else {
(F) BRANCH crates/lake/tests/real_lake.rs:168:9 True  [let-else in test]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
(F) BRANCH crates/lake/tests/real_lake.rs:168:9 False  [let-else in test]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
(F) BRANCH crates/lake/tests/real_lake.rs:200:9 True  [let-else in test]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
(F) BRANCH crates/lake/tests/real_lake.rs:200:9 False  [let-else in test]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
(F) BRANCH crates/lake/tests/real_lake.rs:214:13 True  [let-else in test]  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
(F) BRANCH crates/lake/tests/real_lake.rs:214:13 False  [let-else in test]  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
(F) BRANCH crates/lake/tests/real_lake.rs:218:17 True  [let-else in test]  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
(F) BRANCH crates/lake/tests/real_lake.rs:218:17 False  [let-else in test]  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
(F) LINE   crates/lake/tests/real_lake_regression.rs:166  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/real_lake_regression.rs:172  [assert! message argument]  root.exists(),
(F) LINE   crates/lake/tests/real_lake_regression.rs:223  [assert! message argument]  refused.is_empty(),
(F) LINE   crates/lake/tests/real_lake_regression.rs:225  [assert! message argument]  refused.join("\n  ")
(F) LINE   crates/lake/tests/real_lake_regression.rs:228  [assert! message argument]  files >= MINIMUM_FILES,
(F) LINE   crates/lake/tests/real_lake_regression.rs:232  [assert! message argument]  fno > 0 && cash > 0,
(F) BRANCH crates/lake/tests/real_lake_regression.rs:132:9 True  [let-else in test]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:132:9 False  [let-else in test]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:138:13 True  [let-else in test]  let Ok(months) = std::fs::read_dir(&y) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:138:13 False  [let-else in test]  let Ok(months) = std::fs::read_dir(&y) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:150:9 True  [let-else in test]  let Ok(entries) = std::fs::read_dir(root) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:150:9 False  [let-else in test]  let Ok(entries) = std::fs::read_dir(root) else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:165:9 True  [let-else in test]  let Some(home) = std::env::var_os("HOME") else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:165:9 False  [let-else in test]  let Some(home) = std::env::var_os("HOME") else {
(F) BRANCH crates/lake/tests/real_lake_regression.rs:232:9 True  [assert! condition branch (failing side)]  fno > 0 && cash > 0,
(F) BRANCH crates/lake/tests/real_lake_regression.rs:232:9 False  [assert! condition branch (failing side)]  fno > 0 && cash > 0,
(F) BRANCH crates/lake/tests/real_lake_regression.rs:232:20 True  [assert! condition branch (failing side)]  fno > 0 && cash > 0,
(F) BRANCH crates/lake/tests/real_lake_regression.rs:232:20 False  [assert! condition branch (failing side)]  fno > 0 && cash > 0,
(F) LINE   crates/lake/tests/refusals.rs:176  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:234  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:318  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/refusals.rs:326  [panic! (failing arm)]  batch.len()
(F) LINE   crates/lake/tests/refusals.rs:454  [assert! message argument]  fabricated.join("\n  ")
(F) LINE   crates/lake/tests/refusals.rs:556  [panic! (failing arm)]  _ => panic!("one INT64 column"),
(F) LINE   crates/lake/tests/refusals.rs:680  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:701  [panic! (failing arm)]  Err(other) => panic!(
(F) LINE   crates/lake/tests/refusals.rs:707  [panic! (failing arm)]  Err(e) => panic!(
(F) LINE   crates/lake/tests/refusals.rs:717  [assert_eq! message argument]  vec![Some(7), Some(8), Some(9)],
(F) LINE   crates/lake/tests/refusals.rs:800  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:819  [panic! (failing arm)]  other => panic!("a {what} leaf must be refused by name, got {other:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1012  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/refusals.rs:1018  [assert! message argument]  root.exists(),
(F) LINE   crates/lake/tests/refusals.rs:1075  [assert! message argument]  scan.files >= 200,
(F) LINE   crates/lake/tests/refusals.rs:1080  [assert! message argument]  scan.years.len() >= 3,
(F) LINE   crates/lake/tests/refusals.rs:1086  [assert! message argument]  scan.declared_short.is_empty(),
(F) LINE   crates/lake/tests/refusals.rs:1170  [panic! (failing arm)]  .unwrap_or_else(|e| panic!("real lake directory {name} was refused: {e}"));
(F) LINE   crates/lake/tests/refusals.rs:1172  [assert_eq! message argument]  parsed.to_string(),
(F) LINE   crates/lake/tests/refusals.rs:1205  [panic! (failing arm)]  panic!(
(F) LINE   crates/lake/tests/refusals.rs:1215  [panic! (failing arm)]  other.to_string() == padded,
(F) LINE   crates/lake/tests/refusals.rs:1216  [panic! (failing arm)]  other == good,
(F) LINE   crates/lake/tests/refusals.rs:1217  [panic! (failing arm)]  other.key() == good.key()
(F) LINE   crates/lake/tests/refusals.rs:1345  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:1487  [panic! (failing arm)]  _ => panic!("the cash layout has no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:1516  [panic! (failing arm)]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1517  [panic! (failing arm)]  Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
(F) LINE   crates/lake/tests/refusals.rs:1595  [panic! (failing arm)]  _ => panic!("this fixture writes no other physical type"),
(F) LINE   crates/lake/tests/refusals.rs:1626  [panic! (failing arm)]  Err(other) => panic!("refused as the wrong thing: {other:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1627  [panic! (failing arm)]  Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
(F) LINE   crates/lake/tests/refusals.rs:1645  [panic! (failing arm)]  Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1646  [panic! (failing arm)]  Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
(F) LINE   crates/lake/tests/refusals.rs:1696  [panic! (failing arm)]  Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1697  [panic! (failing arm)]  Ok(_) => panic!("an INT96 timestamp opened"),
(F) LINE   crates/lake/tests/refusals.rs:1747  [panic! (failing arm)]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F) LINE   crates/lake/tests/refusals.rs:1748  [panic! (failing arm)]  Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
(F) BRANCH crates/lake/tests/refusals.rs:404:13 False  [let-else in test]  let Some(len) = exact_page_prefix(&sound, OI, pages) else {
(F) BRANCH crates/lake/tests/refusals.rs:981:9 True  [let-else in test]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
(F) BRANCH crates/lake/tests/refusals.rs:981:9 False  [let-else in test]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
(F) BRANCH crates/lake/tests/refusals.rs:986:13 True  [let-else in test]  let Ok(months) = std::fs::read_dir(y.path()) else {
(F) BRANCH crates/lake/tests/refusals.rs:986:13 False  [let-else in test]  let Ok(months) = std::fs::read_dir(y.path()) else {
(F) BRANCH crates/lake/tests/refusals.rs:997:9 True  [let-else in test]  let Ok(entries) = std::fs::read_dir(root) else {
(F) BRANCH crates/lake/tests/refusals.rs:997:9 False  [let-else in test]  let Ok(entries) = std::fs::read_dir(root) else {
(F) BRANCH crates/lake/tests/refusals.rs:1011:9 True  [let-else in test]  let Some(home) = std::env::var_os("HOME") else {
(F) BRANCH crates/lake/tests/refusals.rs:1011:9 False  [let-else in test]  let Some(home) = std::env::var_os("HOME") else {
(F) BRANCH crates/lake/tests/refusals.rs:1084:13 True  [assert! condition branch (failing side)]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
(F) BRANCH crates/lake/tests/refusals.rs:1084:13 False  [assert! condition branch (failing side)]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
(F) BRANCH crates/lake/tests/refusals.rs:1084:29 True  [assert! condition branch (failing side)]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
(F) BRANCH crates/lake/tests/refusals.rs:1084:29 False  [assert! condition branch (failing side)]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
(F) BRANCH crates/lake/tests/refusals.rs:1140:9 False  [let-else in test]  let Some(home) = std::env::var_os("HOME") else {
(F) BRANCH crates/lake/tests/refusals.rs:1265:9 False  [assert! condition branch (failing side)]  text.contains("400") && text.contains("2480"),
(F) BRANCH crates/lake/tests/refusals.rs:1265:33 False  [assert! condition branch (failing side)]  text.contains("400") && text.contains("2480"),
(F) BRANCH crates/lake/tests/refusals.rs:1394:71 False  [assert! condition branch (failing side)]  Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
(F) LINE   crates/lake/tests/synthetic.rs:136  [panic! (failing arm)]  _ => panic!("writer type does not match the column"),
(F) LINE   crates/lake/tests/synthetic.rs:345  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:347  [panic! (failing arm)]  other = other.err()
(F) LINE   crates/lake/tests/synthetic.rs:395  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:397  [panic! (failing arm)]  other = other.err()
(F) LINE   crates/lake/tests/synthetic.rs:445  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:447  [panic! (failing arm)]  other = other.err()
(F) LINE   crates/lake/tests/synthetic.rs:465  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:467  [panic! (failing arm)]  other = other.err()
(F) LINE   crates/lake/tests/synthetic.rs:618  [panic! (failing arm)]  other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
(F) LINE   crates/lake/tests/synthetic.rs:631  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:633  [panic! (failing arm)]  other = other.err()
(F) LINE   crates/lake/tests/synthetic.rs:654  [assert! message argument]  matches!(
(F) LINE   crates/lake/tests/synthetic.rs:772  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:774  [panic! (failing arm)]  other.map(|b| b.len())
(F) LINE   crates/lake/tests/synthetic.rs:812  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:814  [panic! (failing arm)]  other.map(|b| b.len())
(F) LINE   crates/lake/tests/synthetic.rs:944  [panic! (failing arm)]  other => panic!("page type {} is not one this writer emits", other.0),
(F) LINE   crates/lake/tests/synthetic.rs:1148  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:1150  [panic! (failing arm)]  other.map(|groups| groups.len())
(F) LINE   crates/lake/tests/synthetic.rs:1215  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/lake/tests/synthetic.rs:1217  [panic! (failing arm)]  other.map(|b| b.len())
(F) LINE   crates/runner/src/admission.rs:7423  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7429  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7435  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7466  [assert_eq! message argument]  left.name(),
(F) LINE   crates/runner/src/admission.rs:7467  [assert_eq! message argument]  right.name()
(F) BRANCH crates/runner/src/admission.rs:5500:13 False  [let-else in test]  let Some(end) = self.offset.checked_add(N) else {
(F) BRANCH crates/runner/src/admission.rs:5507:13 False  [let-else in test]  let Some(source) = self.bytes.get(self.offset..end) else {
(F) LINE   crates/runner/src/audit.rs:2644  [assert! message argument]  || (what != "the sentence"
(F) LINE   crates/runner/src/audit.rs:2645  [assert! message argument]  && token == "V6"
(F) LINE   crates/runner/src/audit.rs:2646  [assert! message argument]  && text.contains("Selection V6")),
(F) BRANCH crates/runner/src/audit.rs:2224:13 False  [assert! condition branch (failing side)]  ranked
(F) BRANCH crates/runner/src/audit.rs:2227:20 False  [assert! condition branch (failing side)]  && ranked.ends_with("what selection ranks on, gross of every charge"),
(F) BRANCH crates/runner/src/audit.rs:2358:44 False  [assert! condition branch (failing side)]  line.is_empty() || line.starts_with("  "),
(F) BRANCH crates/runner/src/audit.rs:2467:29 False  [assert! condition branch (failing side)]  decision || token == "V6",
(F) BRANCH crates/runner/src/audit.rs:2620:40 False  [assert! condition branch (failing side)]  line.is_empty() || line.starts_with("  "),
(F) BRANCH crates/runner/src/audit.rs:2641:21 False  [assert! condition branch (failing side)]  token
(F) BRANCH crates/runner/src/audit.rs:2643:42 False  [assert! condition branch (failing side)]  .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
(F) BRANCH crates/runner/src/audit.rs:2644:29 True  [assert! condition branch (failing side)]  || (what != "the sentence"
(F) BRANCH crates/runner/src/audit.rs:2644:29 False  [assert! condition branch (failing side)]  || (what != "the sentence"
(F) BRANCH crates/runner/src/audit.rs:2645:32 True  [assert! condition branch (failing side)]  && token == "V6"
(F) BRANCH crates/runner/src/audit.rs:2645:32 False  [assert! condition branch (failing side)]  && token == "V6"
(F) BRANCH crates/runner/src/audit.rs:2646:32 True  [assert! condition branch (failing side)]  && text.contains("Selection V6")),
(F) BRANCH crates/runner/src/audit.rs:2646:32 False  [assert! condition branch (failing side)]  && text.contains("Selection V6")),
(F) BRANCH crates/runner/src/audit.rs:2788:13 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:2788:54 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:2974:13 True  [assert! condition branch (failing side)]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
(F) BRANCH crates/runner/src/audit.rs:2974:62 False  [assert! condition branch (failing side)]  out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
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
(F) BRANCH crates/runner/src/bootstrap.rs:3398:46 True  [assert! condition branch (failing side)]  r.round == expected_round || r.round == expected_round + 1,
(F) BRANCH crates/runner/src/bootstrap.rs:3398:46 False  [assert! condition branch (failing side)]  r.round == expected_round || r.round == expected_round + 1,
(F) BRANCH crates/runner/src/bootstrap.rs:3406:13 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap.rs:3406:40 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:13 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:28 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:194:13 False  [assert! condition branch (failing side)]  strict > 0 && strict < draws
(F) LINE   crates/runner/src/excursion.rs:1664  [assert! message argument]  _ => true,
(F) LINE   crates/runner/src/excursion.rs:1685  [assert! message argument]  tightest(8),
(F) LINE   crates/runner/src/excursion.rs:1686  [assert! message argument]  tightest(4)
(F) LINE   crates/runner/src/excursion.rs:1691  [assert! message argument]  tightest(4),
(F) LINE   crates/runner/src/excursion.rs:1692  [assert! message argument]  tightest(2)
(F) LINE   crates/runner/src/excursion.rs:2584  [assert_eq! message argument]  sample.len()
(F) LINE   crates/runner/src/exit_grid_policy.rs:7355  [assert_eq! message argument]  resolved.replay_selected(oos, &execution_column, &selected, run),
(F) LINE   crates/runner/src/exit_grid_policy.rs:7356  [assert_eq! message argument]  Err(ExitGridErrorV1::RefusedExecutionPaths {
(F) LINE   crates/runner/src/exit_grid_policy.rs:7357  [assert_eq! message argument]  paths: universe.pricing_refused_paths(),
(F) LINE   crates/runner/src/exit_grid_policy.rs:7358  [assert_eq! message argument]  }),
(F) BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [let-else in test]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:4663:13 False  [let-else in test]  let Ok(widths) = Widths::pinned() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:4729:13 False  [let-else in test]  let Ok(value) = Symbol::new(text) else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:5251:13 False  [let-else in test]  let Some(sealed) = sealed.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:5260:13 False  [let-else in test]  let Some(series) = series.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:5967:25 False  [let-else in test]  let Some(subsets) = subsets else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:6046:13 False  [let-else in test]  let Some(mut evaluated) = evaluated.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:6052:13 False  [let-else in test]  let Some(validated) = validated.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:6558:13 False  [let-else in test]  let Some(evaluated) = evaluated_result.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7153:13 False  [let-else in test]  let Some(selected) = selected else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7230:13 False  [let-else in test]  let Some(series) = series.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7235:13 False  [let-else in test]  let Some(oos) = oos.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7245:13 False  [let-else in test]  let Some(run) = run.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7258:13 False  [let-else in test]  let Some(changed_spec) = changed_spec else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7568:13 False  [let-else in test]  let Some(mut evaluated) = evaluated.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7581:13 False  [let-else in test]  let Some(validated) = validated.ok() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7630:13 False  [let-else in test]  let Some(mut wrong_offset) = wrong_offset.ok() else {
(F) BRANCH crates/runner/src/expression_execution_tests.rs:211:13 False  [let-else in test]  let [left, right] = pair else {
(F) LINE   crates/runner/src/grid.rs:6311  [assert_eq! message argument]  super::materialize_cell(&bars, &column, &mask, horizon, side, &grid, cell),
(F) LINE   crates/runner/src/grid.rs:6312  [assert_eq! message argument]  super::materialize_cell_over(
(F) LINE   crates/runner/src/grid.rs:6313  [assert_eq! message argument]  &bars, &column, &mask, horizon, side, &grid, cell, &facts
(F) LINE   crates/runner/src/grid.rs:6323  [assert_eq! message argument]  super::per_trade(&bars, &column, &mask, horizon, side, ladders, chosen),
(F) LINE   crates/runner/src/grid.rs:6324  [assert_eq! message argument]  super::per_trade_over(
(F) LINE   crates/runner/src/grid.rs:6325  [assert_eq! message argument]  &bars, &column, &mask, horizon, side, ladders, chosen, &facts
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
(F) LINE   crates/runner/src/grid.rs:10690  [assert! message argument]  (Some(t), Some(ttp)) => ttp.arm < t,
(F) LINE   crates/runner/src/grid.rs:10964  [assert! message argument]  t.eligible.len(),
(F) LINE   crates/runner/src/grid.rs:10965  [assert! message argument]  t.trades.len()
(F) BRANCH crates/runner/src/grid.rs:6254:13 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6254:35 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:13 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:36 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:7275:17 False  [let-else in test]  let Some(slot) = holed.get_mut(hole) else {
(F) BRANCH crates/runner/src/grid.rs:7977:13 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:7977:39 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:8066:21 False  [assert! condition branch (failing side)]  c.worst_trade <= 0 || c.pessimistic > 0,
(F) BRANCH crates/runner/src/grid.rs:8066:43 True  [assert! condition branch (failing side)]  c.worst_trade <= 0 || c.pessimistic > 0,
(F) BRANCH crates/runner/src/grid.rs:8066:43 False  [assert! condition branch (failing side)]  c.worst_trade <= 0 || c.pessimistic > 0,
(F) BRANCH crates/runner/src/grid.rs:8158:17 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid.rs:8158:40 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid.rs:10079:17 False  [assert! condition branch (failing side)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
(F) BRANCH crates/runner/src/grid.rs:10079:43 True  [assert! condition branch (failing side)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
(F) BRANCH crates/runner/src/grid.rs:10079:43 False  [assert! condition branch (failing side)]  assert!(empty.cells.is_empty() || empty.baseline().is_some());
(F) BRANCH crates/runner/src/grid.rs:10490:36 False  [assert! condition branch (failing side)]  .all(|row| victim < row.entry_bar || victim > row.exit_bar),
(F) BRANCH crates/runner/src/grid.rs:10911:63 False  [assert! condition branch (failing side)]  .all(|pair| matches!(pair, [first, second] if second.entry_bar > first.exit_bar)),
(F) BRANCH crates/runner/src/identity.rs:1462:26 False  [assert! condition branch (failing side)]  .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
(F) LINE   crates/runner/src/lib.rs:2028  [assert_eq! message argument]  reads.len()
(F) BRANCH crates/runner/src/lib.rs:2032:33 False  [assert! condition branch (failing side)]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
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
(F) BRANCH crates/runner/src/outcome.rs:2626:41 False  [assert! condition branch (failing side)]  assert!((0..bars.len()).all(|i| refused.at(i).is_none() && refused.was_refused(i)));
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
(F) BRANCH crates/runner/src/outcome.rs:4903:48 False  [assert! condition branch (failing side)]  .all(|w| matches!(w, [a, b] if a.2 == b.2)),
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
(F) BRANCH crates/runner/src/portfolio.rs:785:20 False  [assert! condition branch (failing side)]  if admitted == StrategyDigest::new([1; 32])
(F) BRANCH crates/runner/src/portfolio.rs:916:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([7; 32])
(F) BRANCH crates/runner/src/portfolio.rs:1018:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([3; 32])
(F) LINE   crates/runner/src/rank.rs:1264  [assert! message argument]  grinder.payoff_bp(),
(F) LINE   crates/runner/src/rank.rs:1265  [assert! message argument]  grinder.worst_reward_risk_bp()
(F) LINE   crates/runner/src/report.rs:1148  [assert! message argument]  || text.contains("partitions nothing")
(F) LINE   crates/runner/src/report.rs:1149  [assert! message argument]  || text.contains("retired, void, or outside the table"),
(F) LINE   crates/runner/src/report.rs:1158  [assert! message argument]  text.contains("not measured"),
(F) BRANCH crates/runner/src/report.rs:1147:13 False  [assert! condition branch (failing side)]  text.contains("never true on any loaded bar")
(F) BRANCH crates/runner/src/report.rs:1148:20 True  [assert! condition branch (failing side)]  || text.contains("partitions nothing")
(F) BRANCH crates/runner/src/report.rs:1148:20 False  [assert! condition branch (failing side)]  || text.contains("partitions nothing")
(F) BRANCH crates/runner/src/report.rs:1149:20 True  [assert! condition branch (failing side)]  || text.contains("retired, void, or outside the table"),
(F) BRANCH crates/runner/src/report.rs:1149:20 False  [assert! condition branch (failing side)]  || text.contains("retired, void, or outside the table"),
(F) BRANCH crates/runner/src/report.rs:1509:13 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1509:27 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1522:13 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/report.rs:1522:29 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/resample.rs:869:21 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:869:41 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:870:21 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) BRANCH crates/runner/src/resample.rs:870:42 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) LINE   crates/runner/src/signal_candle_stop_tests.rs:74  [unreachable! (failing arm)]  _ => unreachable!(),
(F) BRANCH crates/runner/src/signal_candle_stop_tests.rs:808:39 False  [assert! condition branch (failing side)]  .all(|period| period.day >= first && period.day <= last)
(F) LINE   crates/runner/src/significance.rs:945  [assert! message argument]  bonferroni_t(trials)
(F) LINE   crates/runner/src/significance.rs:1341  [assert! message argument]  (approx - exact).abs()
(F) LINE   crates/runner/src/significance.rs:1357  [assert! message argument]  bonferroni_t(316)
(F) BRANCH crates/runner/src/significance.rs:1323:21 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1323:42 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1385:13 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) BRANCH crates/runner/src/significance.rs:1385:26 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) LINE   crates/runner/src/split.rs:385  [assert! message argument]  bar + horizon,
(F) BRANCH crates/runner/src/split.rs:513:21 False  [assert! condition branch (failing side)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
(F) BRANCH crates/runner/src/split.rs:513:60 True  [assert! condition branch (failing side)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
(F) BRANCH crates/runner/src/split.rs:513:60 False  [assert! condition branch (failing side)]  i.saturating_add(15) < f.test.start || f.test.is_empty(),
(F) BRANCH crates/runner/src/split.rs:573:21 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/split.rs:573:45 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/synthetic.rs:130:25 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:130:45 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:25 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:46 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:330:17 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) BRANCH crates/runner/src/synthetic.rs:330:27 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) LINE   crates/runner/src/trade.rs:1879  [unreachable! (failing arm)]  unreachable!("windows(2) always yields exactly two trades")
(F) LINE   crates/runner/src/trade.rs:2135  [assert_eq! message argument]  bars.get(fill).map(|bar| bar.ts_micros),
(F) LINE   crates/runner/src/trade.rs:2136  [assert_eq! message argument]  bars.get(source)
(F) LINE   crates/runner/src/trade.rs:2137  [assert_eq! message argument]  .and_then(|bar| bar.ts_micros.checked_add(60_000_000)),
(F) LINE   crates/runner/src/trade.rs:2238  [assert! message argument]  t.count(),
(F) LINE   crates/runner/src/trade.rs:2382  [assert! message argument]  ist_minute_of(exit_ts)
(F) LINE   crates/runner/src/trade.rs:3131  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/trade.rs:1692:25 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:1692:48 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:1878:17 False  [let-else in test]  let [previous, next] = pair else {
(F) BRANCH crates/runner/src/trade.rs:2041:21 False  [let-else in test]  let [before, after] = pair else {
(F) BRANCH crates/runner/src/trade.rs:2086:21 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2086:48 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2205:17 False  [assert! condition branch (failing side)]  trade.entry_bar > previous_exit || n == 0,
(F) BRANCH crates/runner/src/trade.rs:2205:52 True  [assert! condition branch (failing side)]  trade.entry_bar > previous_exit || n == 0,
(F) BRANCH crates/runner/src/trade.rs:2205:52 False  [assert! condition branch (failing side)]  trade.entry_bar > previous_exit || n == 0,
(F) BRANCH crates/runner/src/trade.rs:2594:17 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2594:32 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2708:13 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:2708:32 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:2813:42 False  [assert! condition branch (failing side)]  .all(|trade| victim < trade.entry_bar || victim > trade.exit_bar),
(F) BRANCH crates/runner/src/trade.rs:2819:33 False  [assert! condition branch (failing side)]  !held.priceable
(F) BRANCH crates/runner/src/trade.rs:2820:40 False  [assert! condition branch (failing side)]  && held.entry_bar <= victim
(F) BRANCH crates/runner/src/trade.rs:3025:17 True  [assert! condition branch (failing side)]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
(F) BRANCH crates/runner/src/trade.rs:3025:34 False  [assert! condition branch (failing side)]  !trade.forced || forced_price_exists(&bars, trade.exit_bar),
(F) BRANCH crates/runner/src/trade.rs:3205:13 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3205:34 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3229:17 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3229:44 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3235:13 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade.rs:3235:42 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:13 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:43 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) LINE   crates/runner/src/validate.rs:6631  [assert! message argument]  f.chosen_exit.is_some(),
(F) LINE   crates/runner/src/validate.rs:6791  [assert_eq! message argument]  f.considered.saturating_sub(f.priced)
(F) LINE   crates/runner/src/validate.rs:7580  [assert! message argument]  matches!(
(F) LINE   crates/runner/src/validate.rs:7598  [assert! message argument]  matches!(
(F) LINE   crates/runner/src/validate.rs:7935  [assert! message argument]  body.len()
(F) LINE   crates/runner/src/validate.rs:7994  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/validate.rs:6248:17 False  [let-else in test]  let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
(F) BRANCH crates/runner/src/validate.rs:6264:17 False  [let-else in test]  let (Some(a), Some(b)) = (pair.first(), pair.get(1)) else {
(F) BRANCH crates/runner/src/validate.rs:6850:13 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6850:29 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6850:50 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6959:21 False  [let-else in test]  let Some(cell) = g.sharpest().or_else(|| g.best()) else {
(F) BRANCH crates/runner/src/validate.rs:7348:19 False  [assert! condition branch (failing side)]  }) if refused_fold == fold
(F) BRANCH crates/runner/src/validate.rs:7683:17 False  [assert! condition branch (failing side)]  candidate.oos_cell_digest.is_some() && candidate.out_of_sample_pessimistic.is_some()
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
(F) BRANCH crates/runner/tests/end_to_end.rs:115:39 True  [assert! condition branch (failing side)]  exits.baseline().is_some() || exits.cells.is_empty(),
(F) BRANCH crates/runner/tests/end_to_end.rs:115:39 False  [assert! condition branch (failing side)]  exits.baseline().is_some() || exits.cells.is_empty(),
(F) BRANCH crates/runner/tests/end_to_end.rs:210:9 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
(F) BRANCH crates/runner/tests/end_to_end.rs:210:25 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
(F) BRANCH crates/runner/tests/end_to_end.rs:383:13 False  [assert! condition branch (failing side)]  !rest.starts_with("  NOT SUPPLIED") && rest.contains("variants evaluated")
(F) LINE   crates/runner/tests/expression_pricing.rs:55  [assert_eq! message argument]  )?,
(F) LINE   crates/runner/tests/expression_pricing.rs:80  [assert_eq! message argument]  )?,
(F) LINE   crates/runner/tests/expression_pricing.rs:83  [assert_eq! message argument]  )?
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
(F) LINE   crates/runner/tests/foreign_slice_deadline_readiness.rs:260  [assert! message argument]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
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
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:21 True  [assert! condition branch (failing side)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:21 False  [assert! condition branch (failing side)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:36 True  [assert! condition branch (failing side)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:260:36 False  [assert! condition branch (failing side)]  item.forced || elapsed(&bars, item)? == 3 * MINUTE,
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:317:58 False  [assert! condition branch (failing side)]  assert!(timed.eligible.iter().any(|item| item.entry_bar < corrupt_index && corrupt_index < item.exit_bar),
(F) LINE   crates/runner/tests/hole_after_exit.rs:124  [assert! message argument]  probes.len()
(F) LINE   crates/runner/tests/join_answer_is_unchanged.rs:140  [assert_eq! message argument]  level.frequent.len(),
(F) LINE   crates/runner/tests/research_family_readiness.rs:433  [assert! message argument]  matches!(
(F) LINE   crates/runner/tests/unwired_validation_record.rs:129  [assert! message argument]  path.display()
(F) LINE   crates/store/src/catalog_tests.rs:268  [panic! (failing arm)]  panic!("a dangling `bars` link is refused");
(F) LINE   crates/store/src/catalog_tests.rs:282  [panic! (failing arm)]  panic!("an unsearchable root is refused, not empty: {locked:?}");
(F) LINE   crates/store/src/catalog_tests.rs:460  [assert! message argument]  path.display()
(F) BRANCH crates/store/src/catalog_tests.rs:267:9 False  [let-else in test]  let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
(F) BRANCH crates/store/src/catalog_tests.rs:281:9 False  [let-else in test]  let Err(CatalogError::BarsUnreadable { because }) = locked else {
(F) BRANCH crates/store/src/catalog_tests.rs:314:13 False  [assert! condition branch (failing side)]  assert!(stack.is_empty() && out.held.is_empty());
(F) BRANCH crates/store/src/catalog_tests.rs:314:33 False  [assert! condition branch (failing side)]  assert!(stack.is_empty() && out.held.is_empty());
(F) BRANCH crates/store/src/catalog_tests.rs:320:13 False  [assert! condition branch (failing side)]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F) BRANCH crates/store/src/catalog_tests.rs:320:32 False  [assert! condition branch (failing side)]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F) BRANCH crates/store/src/catalog_tests.rs:320:50 False  [assert! condition branch (failing side)]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F) BRANCH crates/store/src/catalog_tests.rs:336:13 False  [assert! condition branch (failing side)]  assert!(stack.is_empty() && out.held.is_empty());
(F) BRANCH crates/store/src/catalog_tests.rs:336:33 False  [assert! condition branch (failing side)]  assert!(stack.is_empty() && out.held.is_empty());
(F) LINE   crates/store/src/file.rs:6716  [assert! message argument]  parent.display()
(F) LINE   crates/store/src/file.rs:6920  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:6940  [assert_eq! message argument]  level.display()
(F) LINE   crates/store/src/file.rs:7199  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:7253  [panic! (failing arm)]  other => panic!("the flush was refused by path: {other:?}"),
(F) LINE   crates/store/src/file.rs:7573  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:7840  [assert_eq! message argument]  StoreError::DiskFull {
(F) LINE   crates/store/src/file.rs:7841  [assert_eq! message argument]  path: at.clone(),
(F) LINE   crates/store/src/file.rs:7842  [assert_eq! message argument]  action: Action::Write,
(F) LINE   crates/store/src/file.rs:7843  [assert_eq! message argument]  },
(F) LINE   crates/store/src/file.rs:8337  [assert! message argument]  reads.get()
(F) LINE   crates/store/src/file.rs:8676  [panic! (failing arm)]  other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
(F) LINE   crates/store/src/file.rs:8894  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:8903  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:8913  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:9015  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:9058  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:9264  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:10209  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file.rs:10335  [assert! message argument]  matches!(
(F) BRANCH crates/store/src/file.rs:6863:82 False  [assert! condition branch (failing side)]  matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
(F) BRANCH crates/store/src/file.rs:9087:13 False  [assert! condition branch (failing side)]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F) BRANCH crates/store/src/file.rs:9087:45 False  [assert! condition branch (failing side)]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F) BRANCH crates/store/src/file.rs:9087:73 False  [assert! condition branch (failing side)]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F) BRANCH crates/store/src/file.rs:10699:24 False  [assert! condition branch (failing side)]  if *ts_micros == bar(10).ts_micros
(F) LINE   crates/store/src/file_fault_tests.rs:320  [panic! (failing arm)]  other => panic!("read {skip}: {other:?}"),
(F) LINE   crates/store/src/file_fault_tests.rs:657  [panic! (failing arm)]  Err(other) => panic!("read {skip}: {other:?}"),
(F) LINE   crates/store/src/file_fault_tests.rs:696  [panic! (failing arm)]  other => panic!("{}: {other:?}", looked.display()),
(F) LINE   crates/store/src/file_fault_tests.rs:869  [panic! (failing arm)]  other => panic!("{kind:?} measure {skip}: {other:?}"),
(F) BRANCH crates/store/src/file_fault_tests.rs:334:9 False  [assert! condition branch (failing side)]  routed >= 1 && refused >= 1,
(F) BRANCH crates/store/src/file_fault_tests.rs:334:24 False  [assert! condition branch (failing side)]  routed >= 1 && refused >= 1,
(F) BRANCH crates/store/src/file_fault_tests.rs:650:21 False  [assert! condition branch (failing side)]  reason.starts_with(&format!("{}: ", srt.display()))
(F) BRANCH crates/store/src/file_fault_tests.rs:651:28 False  [assert! condition branch (failing side)]  && reason.contains("refused by the file test seam"),
(F) BRANCH crates/store/src/file_fault_tests.rs:661:9 False  [assert! condition branch (failing side)]  built >= 1 && refused >= 1,
(F) BRANCH crates/store/src/file_fault_tests.rs:661:23 False  [assert! condition branch (failing side)]  built >= 1 && refused >= 1,
(F) BRANCH crates/store/src/file_fault_tests.rs:755:71 False  [assert! condition branch (failing side)]  matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
(F) BRANCH crates/store/src/file_fault_tests.rs:879:26 False  [assert! condition branch (failing side)]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F) BRANCH crates/store/src/file_fault_tests.rs:879:42 False  [assert! condition branch (failing side)]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F) BRANCH crates/store/src/file_fault_tests.rs:912:13 False  [assert! condition branch (failing side)]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F) BRANCH crates/store/src/file_fault_tests.rs:912:54 False  [assert! condition branch (failing side)]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F) BRANCH crates/store/src/file_fault_tests.rs:949:13 False  [assert! condition branch (failing side)]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F) BRANCH crates/store/src/file_fault_tests.rs:949:54 False  [assert! condition branch (failing side)]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F) LINE   crates/store/src/file_sealed_tests.rs:321  [panic! (failing arm)]  other => panic!("expected Missing naming the .bin, got {other:?}"),
(F) LINE   crates/store/src/file_sealed_tests.rs:419  [panic! (failing arm)]  other => panic!("a damaged frame must be refused by name, got {other:?}"),
(F) LINE   crates/store/src/file_sealed_tests.rs:439  [panic! (failing arm)]  other => panic!("a damaged head must be refused, got {other:?}"),
(F) LINE   crates/store/src/file_sealed_tests.rs:798  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file_sealed_tests.rs:803  [assert! message argument]  crate::provenance::read(&root.0, path)
(F) LINE   crates/store/src/file_sealed_tests.rs:885  [assert_eq! message argument]  page.len()
(F) LINE   crates/store/src/file_sealed_tests.rs:898  [assert_eq! message argument]  page.len()
(F) BRANCH crates/store/src/file_sealed_tests.rs:214:69 False  [assert! condition branch (failing side)]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F) BRANCH crates/store/src/file_sealed_tests.rs:214:83 False  [assert! condition branch (failing side)]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F) BRANCH crates/store/src/file_sealed_tests.rs:315:13 False  [assert! condition branch (failing side)]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F) BRANCH crates/store/src/file_sealed_tests.rs:315:30 False  [assert! condition branch (failing side)]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F) BRANCH crates/store/src/file_sealed_tests.rs:315:50 False  [assert! condition branch (failing side)]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F) BRANCH crates/store/src/file_sealed_tests.rs:407:17 False  [assert! condition branch (failing side)]  !reason.is_empty() && !detail.is_empty(),
(F) BRANCH crates/store/src/file_sealed_tests.rs:407:39 False  [assert! condition branch (failing side)]  !reason.is_empty() && !detail.is_empty(),
(F) BRANCH crates/store/src/file_sealed_tests.rs:824:9 False  [assert! condition branch (failing side)]  refused.starts_with("Err(") && refused.contains("sbar"),
(F) BRANCH crates/store/src/file_sealed_tests.rs:824:40 False  [assert! condition branch (failing side)]  refused.starts_with("Err(") && refused.contains("sbar"),
(F) BRANCH crates/store/src/file_sealed_tests.rs:874:73 False  [assert! condition branch (failing side)]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F) BRANCH crates/store/src/file_sealed_tests.rs:874:92 False  [assert! condition branch (failing side)]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F) LINE   crates/store/src/file_write_door_tests.rs:165  [panic! (failing arm)]  panic!("a new bar at a sealed month is `Sealed`, got {refused}");
(F) LINE   crates/store/src/file_write_door_tests.rs:322  [panic! (failing arm)]  _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:439  [panic! (failing arm)]  _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:539  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:595  [panic! (failing arm)]  other => panic!("{n}: {other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:605  [panic! (failing arm)]  other => panic!("{n}: {other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:721  [assert! message argument]  file.display()
(F) LINE   crates/store/src/file_write_door_tests.rs:749  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:765  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:809  [panic! (failing arm)]  Err(other) => panic!("{other}"),
(F) LINE   crates/store/src/file_write_door_tests.rs:844  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file_write_door_tests.rs:859  [panic! (failing arm)]  panic!("{case}: {repaired:?}");
(F) LINE   crates/store/src/file_write_door_tests.rs:902  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file_write_door_tests.rs:943  [panic! (failing arm)]  panic!("{repaired:?}");
(F) LINE   crates/store/src/file_write_door_tests.rs:983  [assert! message argument]  matches!(
(F) LINE   crates/store/src/file_write_door_tests.rs:1027  [panic! (failing arm)]  panic!("{case}: {repaired:?}");
(F) LINE   crates/store/src/file_write_door_tests.rs:1067  [panic! (failing arm)]  panic!("{case}: {repaired:?}");
(F) BRANCH crates/store/src/file_write_door_tests.rs:67:13 False  [let-else in test]  let Ok(entries) = fs::read_dir(&at) else {
(F) BRANCH crates/store/src/file_write_door_tests.rs:942:9 False  [let-else in test]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F) BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False  [let-else in test]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F) LINE   crates/store/src/flock_patient_tests.rs:109  [panic! (failing arm)]  panic!("contention, not a host refusal: {refused}");
(F) LINE   crates/store/src/flock_patient_tests.rs:173  [panic! (failing arm)]  panic!("a host refusal: {got:?}");
(F) LINE   crates/store/src/flock_patient_tests.rs:195  [panic! (failing arm)]  panic!("{got:?}");
(F) LINE   crates/store/src/flock_patient_tests.rs:265  [panic! (failing arm)]  panic!("{got:?}");
(F) LINE   crates/store/src/flock_patient_tests.rs:439  [panic! (failing arm)]  panic!("it gave up: {refused:?}");
(F) LINE   crates/store/src/flock_patient_tests.rs:630  [panic! (failing arm)]  Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
(F) LINE   crates/store/src/flock_patient_tests.rs:687  [assert! message argument]  matches!(
(F) BRANCH crates/store/src/flock_patient_tests.rs:123:9 False  [assert! condition branch (failing side)]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F) BRANCH crates/store/src/flock_patient_tests.rs:123:26 False  [assert! condition branch (failing side)]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F) BRANCH crates/store/src/flock_patient_tests.rs:172:9 False  [let-else in test]  let Err(Impatient::Host(why)) = got else {
(F) BRANCH crates/store/src/flock_patient_tests.rs:219:62 False  [assert! condition branch (failing side)]  matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
(F) BRANCH crates/store/src/flock_patient_tests.rs:565:9 False  [assert! condition branch (failing side)]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F) BRANCH crates/store/src/flock_patient_tests.rs:565:51 False  [assert! condition branch (failing side)]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F) BRANCH crates/store/src/flock_patient_tests.rs:683:9 False  [assert! condition branch (failing side)]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F) BRANCH crates/store/src/flock_patient_tests.rs:683:50 False  [assert! condition branch (failing side)]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F) LINE   crates/store/src/flock_tests.rs:213  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/src/flock_tests.rs:244  [panic! (failing arm)]  panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
(F) LINE   crates/store/src/flock_tests.rs:264  [panic! (failing arm)]  panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
(F) LINE   crates/store/src/flock_tests.rs:283  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/src/flock_tests.rs:320  [panic! (failing arm)]  panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
(F) LINE   crates/store/src/flock_tests.rs:924  [assert! message argument]  matches!(
(F) BRANCH crates/store/src/flock_tests.rs:732:13 False  [assert! condition branch (failing side)]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F) BRANCH crates/store/src/flock_tests.rs:732:38 False  [assert! condition branch (failing side)]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F) BRANCH crates/store/src/flock_tests.rs:759:9 False  [assert! condition branch (failing side)]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F) BRANCH crates/store/src/flock_tests.rs:759:42 False  [assert! condition branch (failing side)]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F) BRANCH crates/store/src/flock_tests.rs:841:9 False  [assert! condition branch (failing side)]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F) BRANCH crates/store/src/flock_tests.rs:841:51 False  [assert! condition branch (failing side)]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F) BRANCH crates/store/src/flock_tests.rs:952:9 False  [assert! condition branch (failing side)]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F) BRANCH crates/store/src/flock_tests.rs:952:66 False  [assert! condition branch (failing side)]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F) LINE   crates/store/src/month_summary_tests.rs:295  [panic! (failing arm)]  panic!("a read of {} waited for a writer", pipe.display());
(F) LINE   crates/store/src/month_summary_tests.rs:791  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) BRANCH crates/store/src/month_summary_tests.rs:278:52 False  [assert! condition branch (failing side)]  matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
(F) BRANCH crates/store/src/month_summary_tests.rs:293:9 False  [let-else in test]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) BRANCH crates/store/src/month_summary_tests.rs:320:53 False  [assert! condition branch (failing side)]  matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
(F) BRANCH crates/store/src/month_summary_tests.rs:340:53 False  [assert! condition branch (failing side)]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F) BRANCH crates/store/src/month_summary_tests.rs:355:53 False  [assert! condition branch (failing side)]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F) BRANCH crates/store/src/month_summary_tests.rs:377:13 False  [assert! condition branch (failing side)]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F) BRANCH crates/store/src/month_summary_tests.rs:377:40 False  [assert! condition branch (failing side)]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F) BRANCH crates/store/src/month_summary_tests.rs:461:32 False  [assert! condition branch (failing side)]  Refused::Shape(why) if why.contains("flag")
(F) BRANCH crates/store/src/month_summary_tests.rs:467:60 False  [assert! condition branch (failing side)]  matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
(F) BRANCH crates/store/src/month_summary_tests.rs:480:59 False  [assert! condition branch (failing side)]  assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
(F) BRANCH crates/store/src/month_summary_tests.rs:483:63 False  [assert! condition branch (failing side)]  assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
(F) BRANCH crates/store/src/month_summary_tests.rs:488:61 False  [assert! condition branch (failing side)]  assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
(F) BRANCH crates/store/src/month_summary_tests.rs:491:59 False  [assert! condition branch (failing side)]  assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
(F) BRANCH crates/store/src/month_summary_tests.rs:655:13 False  [assert! condition branch (failing side)]  assert!(round.tracks_carries() && !round.wants_carries());
(F) BRANCH crates/store/src/month_summary_tests.rs:655:39 False  [assert! condition branch (failing side)]  assert!(round.tracks_carries() && !round.wants_carries());
(F) BRANCH crates/store/src/month_summary_tests.rs:658:13 False  [assert! condition branch (failing side)]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F) BRANCH crates/store/src/month_summary_tests.rs:658:42 False  [assert! condition branch (failing side)]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F) BRANCH crates/store/src/month_summary_tests.rs:739:37 False  [assert! condition branch (failing side)]  Err(Refused::Shape(why)) if why.contains("not a held minute")
(F) BRANCH crates/store/src/month_summary_tests.rs:757:13 False  [assert! condition branch (failing side)]  assert!(!read.tracks_carries() && read.wants_carries());
(F) BRANCH crates/store/src/month_summary_tests.rs:757:39 False  [assert! condition branch (failing side)]  assert!(!read.tracks_carries() && read.wants_carries());
(F) LINE   crates/store/src/quarantine_tests.rs:145  [panic! (failing arm)]  panic!("expected a dropped generation, got {applied:?}");
(F) LINE   crates/store/src/quarantine_tests.rs:175  [panic! (failing arm)]  panic!("the month is labelled");
(F) LINE   crates/store/src/quarantine_tests.rs:264  [panic! (failing arm)]  panic!("the month is answered by name");
(F) LINE   crates/store/src/quarantine_tests.rs:551  [panic! (failing arm)]  other => panic!("expected the hold named, got {other:?}"),
(F) LINE   crates/store/src/quarantine_tests.rs:800  [panic! (failing arm)]  panic!("the month is still answered by name");
(F) LINE   crates/store/src/quarantine_tests.rs:892  [panic! (failing arm)]  other => panic!("{why}: {other:?}"),
(F) LINE   crates/store/src/quarantine_tests.rs:1092  [panic! (failing arm)]  other => panic!("expected the quarantine to refuse by name, got {other:?}"),
(F) LINE   crates/store/src/quarantine_tests.rs:1258  [panic! (failing arm)]  other => panic!("the rerun indexes the drop: {other:?}"),
(F) LINE   crates/store/src/quarantine_tests.rs:1975  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) BRANCH crates/store/src/quarantine_tests.rs:144:9 False  [let-else in test]  let Applied::Dropped { drop, .. } = applied else {
(F) BRANCH crates/store/src/quarantine_tests.rs:174:9 False  [let-else in test]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
(F) BRANCH crates/store/src/quarantine_tests.rs:210:63 False  [assert! condition branch (failing side)]  assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
(F) BRANCH crates/store/src/quarantine_tests.rs:263:9 False  [let-else in test]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
(F) BRANCH crates/store/src/quarantine_tests.rs:342:66 False  [assert! condition branch (failing side)]  matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
(F) BRANCH crates/store/src/quarantine_tests.rs:348:77 False  [assert! condition branch (failing side)]  assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
(F) BRANCH crates/store/src/quarantine_tests.rs:497:42 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F) BRANCH crates/store/src/quarantine_tests.rs:497:71 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F) BRANCH crates/store/src/quarantine_tests.rs:508:73 False  [assert! condition branch (failing side)]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F) BRANCH crates/store/src/quarantine_tests.rs:508:92 False  [assert! condition branch (failing side)]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F) BRANCH crates/store/src/quarantine_tests.rs:799:9 False  [let-else in test]  let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
(F) BRANCH crates/store/src/quarantine_tests.rs:855:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F) BRANCH crates/store/src/quarantine_tests.rs:855:70 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F) BRANCH crates/store/src/quarantine_tests.rs:1071:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F) BRANCH crates/store/src/quarantine_tests.rs:1072:16 False  [assert! condition branch (failing side)]  && e.bars == 6
(F) BRANCH crates/store/src/quarantine_tests.rs:1073:16 False  [assert! condition branch (failing side)]  && e.causes.said() == "before_listing and foreign_level"
(F) BRANCH crates/store/src/quarantine_tests.rs:1307:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F) BRANCH crates/store/src/quarantine_tests.rs:1307:70 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F) BRANCH crates/store/src/quarantine_tests.rs:1366:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F) BRANCH crates/store/src/quarantine_tests.rs:1366:70 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F) BRANCH crates/store/src/quarantine_tests.rs:1399:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F) BRANCH crates/store/src/quarantine_tests.rs:1446:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.bars == 3
(F) BRANCH crates/store/src/quarantine_tests.rs:1513:9 False  [assert! condition branch (failing side)]  refused.is_err() && words.contains("quarantined"),
(F) BRANCH crates/store/src/quarantine_tests.rs:1513:29 False  [assert! condition branch (failing side)]  refused.is_err() && words.contains("quarantined"),
(F) BRANCH crates/store/src/quarantine_tests.rs:1564:9 False  [assert! condition branch (failing side)]  refused.is_err() && words.contains("quarantined"),
(F) BRANCH crates/store/src/quarantine_tests.rs:1564:29 False  [assert! condition branch (failing side)]  refused.is_err() && words.contains("quarantined"),
(F) BRANCH crates/store/src/quarantine_tests.rs:1619:38 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F) BRANCH crates/store/src/quarantine_tests.rs:1619:67 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F) BRANCH crates/store/src/quarantine_tests.rs:1619:82 False  [assert! condition branch (failing side)]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F) BRANCH crates/store/src/quarantine_tests.rs:1727:58 False  [assert! condition branch (failing side)]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F) BRANCH crates/store/src/quarantine_tests.rs:1727:94 False  [assert! condition branch (failing side)]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F) BRANCH crates/store/src/quarantine_tests.rs:1845:77 False  [assert! condition branch (failing side)]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
(F) LINE   crates/store/src/repair_tests.rs:116  [panic! (failing arm)]  Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
(F) LINE   crates/store/src/repair_tests.rs:630  [assert_eq! message argument]  timeframe.as_str()
(F) LINE   crates/store/src/repair_tests.rs:891  [panic! (failing arm)]  panic!("expected a restated generation, got {done:?}");
(F) BRANCH crates/store/src/repair_tests.rs:824:94 False  [assert! condition branch (failing side)]  matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
(F) LINE   crates/store/src/safe_open_tests.rs:77  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/src/safe_open_tests.rs:79  [panic! (failing arm)]  path.display(),
(F) LINE   crates/store/src/safe_open_tests.rs:80  [panic! (failing arm)]  tried.join("; ")
(F) LINE   crates/store/src/safe_open_tests.rs:103  [assert! message argument]  found.file_type()
(F) LINE   crates/store/src/safe_open_tests.rs:110  [assert! message argument]  link.display()
(F) LINE   crates/store/src/safe_open_tests.rs:124  [panic! (failing arm)]  panic!("an open of {} waited for a writer", pipe.display());
(F) LINE   crates/store/src/safe_open_tests.rs:401  [panic! (failing arm)]  Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
(F) LINE   crates/store/src/safe_open_tests.rs:402  [panic! (failing arm)]  Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
(F) BRANCH crates/store/src/safe_open_tests.rs:95:42 False  [assert! condition branch (failing side)]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F) BRANCH crates/store/src/safe_open_tests.rs:122:9 False  [let-else in test]  let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
(F) BRANCH crates/store/src/safe_open_tests.rs:298:16 False  [assert! condition branch (failing side)]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F) BRANCH crates/store/src/safe_open_tests.rs:298:35 False  [assert! condition branch (failing side)]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F) BRANCH crates/store/src/safe_open_tests.rs:299:20 False  [assert! condition branch (failing side)]  && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
(F) BRANCH crates/store/src/safe_open_tests.rs:459:77 False  [assert! condition branch (failing side)]  matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
(F) LINE   crates/store/src/sealed_codec_tests.rs:81  [assert! message argument]  frame.len()
(F) LINE   crates/store/src/sealed_codec_tests.rs:282  [assert! message argument]  frame.len()
(F) LINE   crates/store/src/sealed_codec_tests.rs:439  [assert! message argument]  largest[rows],
(F) LINE   crates/store/src/sealed_codec_tests.rs:440  [assert! message argument]  max_frame_len(m)
(F) BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False  [assert! condition branch (failing side)]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F) BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False  [assert! condition branch (failing side)]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F) BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False  [assert! condition branch (failing side)]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F) BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False  [assert! condition branch (failing side)]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F) LINE   crates/store/src/sealed_tests.rs:159  [panic! (failing arm)]  Ok(value) => panic!("expected a refusal, got {value:?}"),
(F) LINE   crates/store/src/sealed_tests.rs:502  [panic! (failing arm)]  Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
(F) LINE   crates/store/src/sealed_tests.rs:653  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:837  [panic! (failing arm)]  other => panic!("{field}: {other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1170  [assert! message argument]  packed.len(),
(F) LINE   crates/store/src/sealed_tests.rs:1171  [assert! message argument]  image.len()
(F) LINE   crates/store/src/sealed_tests.rs:1192  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/src/sealed_tests.rs:1241  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1319  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1656  [panic! (failing arm)]  other => panic!("{field}: {other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1673  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1689  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1774  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1802  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1891  [panic! (failing arm)]  other => panic!("{other}"),
(F) LINE   crates/store/src/sealed_tests.rs:1982  [assert! message argument]  matches!(
(F) LINE   crates/store/src/sealed_tests.rs:2104  [panic! (failing arm)]  other => panic!("{other}"),
(F) BRANCH crates/store/src/sealed_tests.rs:696:27 False  [assert! condition branch (failing side)]  found == 0 || bars[found as usize - 1].ts_micros < ts,
(F) BRANCH crates/store/src/sealed_tests.rs:1816:100 False  [assert! condition branch (failing side)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
(F) BRANCH crates/store/src/sealed_tests.rs:1822:105 False  [assert! condition branch (failing side)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F) BRANCH crates/store/src/sealed_tests.rs:1828:105 False  [assert! condition branch (failing side)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
(F) BRANCH crates/store/src/sealed_tests.rs:1836:105 False  [assert! condition branch (failing side)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F) LINE   crates/store/src/settings_tests.rs:152  [panic! (failing arm)]  other => panic!("{bad:?} for {key:?}: {other:?}"),
(F) LINE   crates/store/src/settings_tests.rs:228  [assert! message argument]  matches!(
(F) LINE   crates/store/src/settings_tests.rs:414  [assert! message argument]  reader.faults()
(F) LINE   crates/store/src/settings_tests.rs:570  [assert_eq! message argument]  reader.faults()
(F) LINE   crates/store/src/settings_tests.rs:630  [assert! message argument]  reader.stopped()
(F) LINE   crates/store/src/settings_tests.rs:691  [panic! (failing arm)]  panic!("{what}: the settings read waited instead of naming a fault");
(F) LINE   crates/store/src/settings_tests.rs:969  [assert! message argument]  matches!(
(F) BRANCH crates/store/src/settings_tests.rs:687:9 False  [let-else in test]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) BRANCH crates/store/src/settings_tests.rs:765:48 False  [assert! condition branch (failing side)]  matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
(F) BRANCH crates/store/src/settings_tests.rs:889:60 False  [assert! condition branch (failing side)]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
(F) BRANCH crates/store/src/settings_tests.rs:935:60 False  [assert! condition branch (failing side)]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
(F) BRANCH crates/store/src/settings_tests.rs:1076:9 False  [assert! condition branch (failing side)]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F) BRANCH crates/store/src/settings_tests.rs:1076:52 False  [assert! condition branch (failing side)]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F) BRANCH crates/store/src/settings_tests.rs:1156:43 False  [assert! condition branch (failing side)]  a.superseded_by == Some(3) || a.in_force == Some(2),
(F) BRANCH crates/store/src/small_month_tests.rs:152:52 False  [assert! condition branch (failing side)]  Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
(F) BRANCH crates/store/src/sort_index_tests.rs:324:41 False  [assert! condition branch (failing side)]  assert!(why.contains("kind") || why.contains("reserved"), "{why}");
(F) LINE   crates/store/src/staging_tests.rs:421  [assert_eq! message argument]  within.display()
(F) BRANCH crates/store/src/staging_tests.rs:143:16 False  [assert! condition branch (failing side)]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:143:64 False  [assert! condition branch (failing side)]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:206:67 False  [assert! condition branch (failing side)]  matches!(&refusal, StagingError::Contended { path: p } if *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:238:16 False  [assert! condition branch (failing side)]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:238:64 False  [assert! condition branch (failing side)]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:297:78 False  [assert! condition branch (failing side)]  matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
(F) BRANCH crates/store/src/staging_tests.rs:334:88 False  [assert! condition branch (failing side)]  matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
(F) BRANCH crates/store/src/staging_tests.rs:369:77 False  [assert! condition branch (failing side)]  matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
(F) BRANCH crates/store/src/time_index.rs:921:21 False  [assert! condition branch (failing side)]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F) BRANCH crates/store/src/time_index.rs:921:41 False  [assert! condition branch (failing side)]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F) BRANCH crates/store/src/time_index.rs:1057:17 False  [assert! condition branch (failing side)]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F) BRANCH crates/store/src/time_index.rs:1057:38 False  [assert! condition branch (failing side)]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F) BRANCH crates/store/src/time_index.rs:1057:59 False  [assert! condition branch (failing side)]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F) BRANCH crates/store/src/time_index.rs:1058:17 False  [assert! condition branch (failing side)]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F) BRANCH crates/store/src/time_index.rs:1058:39 False  [assert! condition branch (failing side)]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F) BRANCH crates/store/src/time_index.rs:1105:17 False  [assert! condition branch (failing side)]  assert!(!daily.exact() && g.exact());
(F) BRANCH crates/store/src/time_index.rs:1105:35 False  [assert! condition branch (failing side)]  assert!(!daily.exact() && g.exact());
(F) LINE   crates/store/src/verdict.rs:4219  [panic! (failing arm)]  other => panic!("not image_corrupt: {other:?}"),
(F) LINE   crates/store/src/verdict.rs:4227  [panic! (failing arm)]  other => panic!("not written_by_newer_build: {other:?}"),
(F) LINE   crates/store/src/verdict_tests.rs:366  [panic! (failing arm)]  other => panic!("expected the host's refusal, got {other}"),
(F) LINE   crates/store/src/verdict_tests.rs:462  [panic! (failing arm)]  panic!("a verdict call waited on the pipe at {}", pipe.display());
(F) LINE   crates/store/src/verdict_tests.rs:568  [panic! (failing arm)]  other => panic!("expected the host's refusal, got {other}"),
(F) LINE   crates/store/src/verdict_tests.rs:813  [panic! (failing arm)]  other => panic!("not image_corrupt: {other:?}"),
(F) LINE   crates/store/src/verdict_tests.rs:879  [assert_eq! message argument]  other.dir().display()
(F) LINE   crates/store/src/verdict_tests.rs:951  [panic! (failing arm)]  panic!("the key's record: {:?}", found.found);
(F) BRANCH crates/store/src/verdict_tests.rs:460:9 False  [let-else in test]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) BRANCH crates/store/src/verdict_tests.rs:950:9 False  [let-else in test]  let super::Found::Record { ordinal, record } = found.found else {
(F) LINE   crates/store/tests/catalog.rs:313  [panic! (failing arm)]  Err(why) => panic!("{} could not be made: {why}", full.display()),
(F) LINE   crates/store/tests/catalog.rs:825  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/tests/catalog.rs:827  [panic! (failing arm)]  path.display(),
(F) LINE   crates/store/tests/catalog.rs:828  [panic! (failing arm)]  tried.join("; ")
(F) LINE   crates/store/tests/catalog.rs:852  [assert! message argument]  found.file_type()
(F) LINE   crates/store/tests/catalog.rs:859  [assert! message argument]  link.display()
(F) LINE   crates/store/tests/catalog.rs:875  [assert! message argument]  path.display()
(F) BRANCH crates/store/tests/catalog.rs:317:9 True  [assert! condition branch (failing side)]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F) BRANCH crates/store/tests/catalog.rs:317:39 False  [assert! condition branch (failing side)]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F) BRANCH crates/store/tests/catalog.rs:844:42 False  [assert! condition branch (failing side)]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F) LINE   crates/store/tests/cited_commits.rs:673  [assert! message argument]  cited.len()
(F) LINE   crates/store/tests/cited_commits.rs:709  [assert! message argument]  String::from_utf8_lossy(&out.stderr)
(F) LINE   crates/store/tests/cited_commits.rs:976  [assert! message argument]  sites.len()
(F) BRANCH crates/store/tests/cited_commits.rs:371:9 False  [assert! condition branch (failing side)]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F) BRANCH crates/store/tests/cited_commits.rs:372:16 False  [assert! condition branch (failing side)]  && said.contains("not a git repository"),
(F) BRANCH crates/store/tests/cited_commits.rs:384:9 False  [assert! condition branch (failing side)]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F) BRANCH crates/store/tests/cited_commits.rs:385:16 False  [assert! condition branch (failing side)]  && said.contains("not a git repository"),
(F) BRANCH crates/store/tests/cited_commits.rs:567:13 False  [let-else in test]  let Some(citing) = cited.get(commit) else {
(F) LINE   crates/store/tests/cloud_mutants_store.rs:367  [panic! (failing arm)]  other => panic!("{door}: {other:?}"),
(F) LINE   crates/store/tests/cloud_mutants_store.rs:371  [panic! (failing arm)]  panic!("a moved revision is audited");
(F) LINE   crates/store/tests/cloud_mutants_store.rs:446  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1073  [panic! (failing arm)]  other => panic!("not image_corrupt: {other:?}"),
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1098  [panic! (failing arm)]  panic!("ZZB is in the day");
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1171  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1361  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1568  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/cloud_mutants_store.rs:1925  [panic! (failing arm)]  panic!("{name} is in the day");
(F) LINE   crates/store/tests/cloud_mutants_store.rs:2876  [assert! message argument]  writer.time_lookup()
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False  [assert! condition branch (failing side)]  vanished >= 20 && contended >= 20,
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False  [assert! condition branch (failing side)]  vanished >= 20 && contended >= 20,
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False  [assert! condition branch (failing side)]  } if source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False  [let-else in test]  let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False  [assert! condition branch (failing side)]  audited.starts_with("repair revision 1 of ")
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False  [assert! condition branch (failing side)]  && audited.ends_with(" is not the one its promotion recorded; not audited"),
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False  [let-else in test]  let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False  [assert! condition branch (failing side)]  generation.contains("/nonexistent/cloud/2031-02.tix")
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False  [assert! condition branch (failing side)]  && generation.contains("revision 1 is not the one its promotion recorded"),
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False  [assert! condition branch (failing side)]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False  [assert! condition branch (failing side)]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False  [assert! condition branch (failing side)]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False  [assert! condition branch (failing side)]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False  [assert! condition branch (failing side)]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False  [assert! condition branch (failing side)]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False  [assert! condition branch (failing side)]  if path == &lock_path && *tries == steps + 1
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False  [assert! condition branch (failing side)]  if path == &lock_path && *tries == steps + 1
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False  [assert! condition branch (failing side)]  if path.parent() == Some(year.as_path())
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False  [assert! condition branch (failing side)]  && path.extension().is_some_and(|ext| ext == "tmp")
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False  [assert! condition branch (failing side)]  && source.kind() == std::io::ErrorKind::PermissionDenied
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False  [assert! condition branch (failing side)]  TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False  [assert! condition branch (failing side)]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False  [assert! condition branch (failing side)]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False  [let-else in test]  let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False  [assert! condition branch (failing side)]  if *path == bar_path
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False  [assert! condition branch (failing side)]  TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False  [assert! condition branch (failing side)]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False  [assert! condition branch (failing side)]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False  [assert! condition branch (failing side)]  StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False  [assert! condition branch (failing side)]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F) BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False  [assert! condition branch (failing side)]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F) LINE   crates/store/tests/docs.rs:234  [assert! message argument]  entry.contains(
(F) LINE   crates/store/tests/docs.rs:235  [assert! message argument]  "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
(F) LINE   crates/store/tests/docs.rs:236  [assert! message argument]  ) && entry.contains("this entry supersedes that sentence"),
(F) LINE   crates/store/tests/docs.rs:240  [assert! message argument]  INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
(F) BRANCH crates/store/tests/docs.rs:217:9 False  [assert! condition branch (failing side)]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F) BRANCH crates/store/tests/docs.rs:217:47 False  [assert! condition branch (failing side)]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F) BRANCH crates/store/tests/docs.rs:234:9 True  [assert! condition branch (failing side)]  entry.contains(
(F) BRANCH crates/store/tests/docs.rs:234:9 False  [assert! condition branch (failing side)]  entry.contains(
(F) BRANCH crates/store/tests/docs.rs:236:14 True  [assert! condition branch (failing side)]  ) && entry.contains("this entry supersedes that sentence"),
(F) BRANCH crates/store/tests/docs.rs:236:14 False  [assert! condition branch (failing side)]  ) && entry.contains("this entry supersedes that sentence"),
(F) BRANCH crates/store/tests/docs.rs:253:9 False  [assert! condition branch (failing side)]  FILE_RS.contains("if claimed > header.n_valid {")
(F) BRANCH crates/store/tests/docs.rs:254:16 False  [assert! condition branch (failing side)]  && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
(F) LINE   crates/store/tests/durability.rs:381  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/tests/fault.rs:376  [panic! (failing arm)]  panic!("bit {bit} of byte {byte} took the whole file down: {e}")
(F) BRANCH crates/store/tests/fault.rs:143:36 False  [assert! condition branch (failing side)]  read.n_valid == 150 || read.n_valid == 175,
(F) BRANCH crates/store/tests/fault.rs:220:59 False  [assert! condition branch (failing side)]  horizon == previous_commit.durable_through || horizon == commit.durable_through,
(F) LINE   crates/store/tests/fifo.rs:108  [assert! message argument]  at.display()
(F) LINE   crates/store/tests/fifo.rs:128  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/tests/fifo.rs:130  [panic! (failing arm)]  release.display()
(F) BRANCH crates/store/tests/fifo.rs:157:13 False  [assert! condition branch (failing side)]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F) BRANCH crates/store/tests/fifo.rs:157:61 False  [assert! condition branch (failing side)]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F) BRANCH crates/store/tests/fifo.rs:162:53 False  [assert! condition branch (failing side)]  fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
(F) LINE   crates/store/tests/generation.rs:165  [panic! (failing arm)]  panic!("{done:?}");
(F) LINE   crates/store/tests/generation.rs:217  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/generation.rs:238  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/generation.rs:331  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/generation.rs:367  [panic! (failing arm)]  panic!("repaired");
(F) LINE   crates/store/tests/generation.rs:388  [panic! (failing arm)]  panic!("{second:?}");
(F) LINE   crates/store/tests/generation.rs:475  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/generation.rs:517  [panic! (failing arm)]  other => panic!("{other:?}"),
(F) LINE   crates/store/tests/generation.rs:758  [panic! (failing arm)]  panic!("{repaired:?}");
(F) BRANCH crates/store/tests/generation.rs:179:57 False  [assert! condition branch (failing side)]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F) BRANCH crates/store/tests/generation.rs:179:76 False  [assert! condition branch (failing side)]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F) BRANCH crates/store/tests/generation.rs:670:13 False  [assert! condition branch (failing side)]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F) BRANCH crates/store/tests/generation.rs:670:42 True  [assert! condition branch (failing side)]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F) BRANCH crates/store/tests/generation.rs:670:42 False  [assert! condition branch (failing side)]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F) BRANCH crates/store/tests/generation.rs:694:74 False  [assert! condition branch (failing side)]  matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
(F) BRANCH crates/store/tests/geometry.rs:99:13 False  [assert! condition branch (failing side)]  start >= block_start && end <= block_end,
(F) BRANCH crates/store/tests/geometry.rs:99:37 False  [assert! condition branch (failing side)]  start >= block_start && end <= block_end,
(F) LINE   crates/store/tests/libm_key.rs:1330  [assert_eq! message argument]  if mentions { "is not" } else { "is" }
(F) BRANCH crates/store/tests/libm_key.rs:428:9 False  [assert! condition branch (failing side)]  decision < caveat && caveat < rejected,
(F) BRANCH crates/store/tests/libm_key.rs:428:30 False  [assert! condition branch (failing side)]  decision < caveat && caveat < rejected,
(F) BRANCH crates/store/tests/libm_key.rs:1330:16 True  [assert_eq! condition branch (failing side)]  if mentions { "is not" } else { "is" }
(F) BRANCH crates/store/tests/libm_key.rs:1330:16 False  [assert_eq! condition branch (failing side)]  if mentions { "is not" } else { "is" }
(F) LINE   crates/store/tests/misfiled.rs:184  [panic! (failing arm)]  other => panic!("expected Misfiled, found {other:?}"),
(F) LINE   crates/store/tests/misfiled.rs:321  [assert_eq! message argument]  unborn.len()
(F) BRANCH crates/store/tests/misfiled.rs:217:9 False  [assert! condition branch (failing side)]  said.contains("2019-11-28-1250000-CE")
(F) BRANCH crates/store/tests/misfiled.rs:218:16 False  [assert! condition branch (failing side)]  && said.contains("its store path is not this month's")
(F) BRANCH crates/store/tests/misfiled.rs:219:16 False  [assert! condition branch (failing side)]  && said.contains("DB-B65"),
(F) LINE   crates/store/tests/month_repair.rs:156  [panic! (failing arm)]  other => panic!("not a repair: {other:?}"),
(F) LINE   crates/store/tests/month_repair.rs:324  [assert_eq! message argument]  timeframe.as_str()
(F) LINE   crates/store/tests/month_repair.rs:856  [panic! (failing arm)]  panic!("{outcome:?}");
(F) LINE   crates/store/tests/month_repair.rs:918  [assert! message argument]  left.keys()
(F) LINE   crates/store/tests/month_repair.rs:944  [panic! (failing arm)]  other => panic!("not a repair: {other:?}"),
(F) BRANCH crates/store/tests/month_repair.rs:606:13 False  [assert! condition branch (failing side)]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F) BRANCH crates/store/tests/month_repair.rs:606:47 False  [assert! condition branch (failing side)]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F) BRANCH crates/store/tests/month_repair.rs:739:83 False  [assert! condition branch (failing side)]  matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
(F) BRANCH crates/store/tests/month_repair.rs:879:38 False  [assert! condition branch (failing side)]  Served::Revision(pointer) if pointer.revision.ordinal() == 3
(F) LINE   crates/store/tests/provenance.rs:66  [panic! (failing arm)]  other => panic!("expected a sidecar, found {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:161  [panic! (failing arm)]  Recorded::NothingAppended => panic!("expected a record"),
(F) LINE   crates/store/tests/provenance.rs:451  [panic! (failing arm)]  other => panic!("expected NotTheNewestCommit, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:464  [panic! (failing arm)]  other => panic!("expected NotABarPath, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:476  [panic! (failing arm)]  other => panic!("expected WrongBarFile, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:531  [panic! (failing arm)]  other => panic!("expected OutOfOrder, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:639  [panic! (failing arm)]  other => panic!("expected Damaged, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:698  [panic! (failing arm)]  result => panic!("expected a {field} mismatch, got {result:?}"),
(F) LINE   crates/store/tests/provenance.rs:746  [panic! (failing arm)]  other => panic!("expected PastTheBars, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1200  [panic! (failing arm)]  other => panic!("expected {fault:?}, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1308  [panic! (failing arm)]  other => panic!("expected TooLarge, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1409  [panic! (failing arm)]  other => panic!("expected Record, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1611  [panic! (failing arm)]  other => panic!("expected NotABarPath, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1617  [panic! (failing arm)]  other => panic!("expected NotABarPath, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1671  [panic! (failing arm)]  other => panic!("expected the directory sync refused, got {other:?}"),
(F) LINE   crates/store/tests/provenance.rs:1721  [panic! (failing arm)]  other => panic!("expected the open refused, got {other:?}"),
(F) BRANCH crates/store/tests/repair.rs:506:13 True  [assert! condition branch (failing side)]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F) BRANCH crates/store/tests/repair.rs:506:41 False  [assert! condition branch (failing side)]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F) LINE   crates/store/tests/sealed_codec_alloc.rs:115  [panic! (failing arm)]  panic!("a frame cut to {len} bytes left {frame_answer:?}");
(F) LINE   crates/store/tests/sealed_codec_alloc.rs:125  [assert! message argument]  full.len()
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False  [let-else in test]  let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False  [assert! condition branch (failing side)]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False  [assert! condition branch (failing side)]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False  [assert! condition branch (failing side)]  !text.contains("by its checked rows and no more")
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False  [assert! condition branch (failing side)]  && !text.contains("checked rows times 56 bytes, once"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False  [assert! condition branch (failing side)]  text.contains("the largest of twice") && text.contains("and 4"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False  [assert! condition branch (failing side)]  text.contains("the largest of twice") && text.contains("and 4"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False  [assert! condition branch (failing side)]  text.contains("capacity 0") && text.contains("cleared"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False  [assert! condition branch (failing side)]  text.contains("capacity 0") && text.contains("cleared"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False  [assert! condition branch (failing side)]  !text.contains("random fields cost at most 57 bytes a bar")
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False  [assert! condition branch (failing side)]  && !text.contains("at most 57 bytes a bar, 1 more than raw"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False  [assert! condition branch (failing side)]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
(F) BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False  [assert! condition branch (failing side)]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
(F) LINE   crates/store/tests/sealed_codec_samples.rs:69  [assert! message argument]  path.extension().is_some_and(|e| e == "bin"),
(F) LINE   crates/store/tests/sealed_codec_samples.rs:71  [assert! message argument]  path.display()
(F) LINE   crates/store/tests/sealed_codec_samples.rs:73  [panic! (failing arm)]  let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
(F) LINE   crates/store/tests/sealed_codec_samples.rs:75  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/tests/sealed_codec_samples.rs:77  [panic! (failing arm)]  path.display(),
(F) LINE   crates/store/tests/sealed_codec_samples.rs:78  [panic! (failing arm)]  data.len()
(F) LINE   crates/store/tests/sealed_codec_samples.rs:82  [panic! (failing arm)]  .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
(F) LINE   crates/store/tests/sealed_codec_samples.rs:86  [panic! (failing arm)]  panic!(
(F) LINE   crates/store/tests/sealed_codec_samples.rs:88  [panic! (failing arm)]  path.display()
(F) LINE   crates/store/tests/sealed_codec_samples.rs:297  [assert! message argument]  stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
(F) LINE   crates/store/tests/sealed_codec_samples.rs:302  [assert! message argument]  frame.len() <= rule,
(F) LINE   crates/store/tests/sealed_codec_samples.rs:304  [assert! message argument]  frame.len()
(F) LINE   crates/store/tests/sealed_codec_samples.rs:445  [assert! message argument]  within(self.codec1, self.exact).0,
(F) LINE   crates/store/tests/sealed_codec_samples.rs:451  [assert! message argument]  within(self.codec1, self.judge).0,
(F) LINE   crates/store/tests/sealed_door.rs:241  [assert! message argument]  parts.len() == 7 || parts.len() == 8,
(F) LINE   crates/store/tests/sealed_door.rs:248  [panic! (failing arm)]  .unwrap_or_else(|| panic!("{rel}: vendor"));
(F) LINE   crates/store/tests/sealed_door.rs:255  [panic! (failing arm)]  .unwrap_or_else(|| panic!("{rel}: rung"));
(F) LINE   crates/store/tests/sealed_door.rs:285  [assert! message argument]  !fs::symlink_metadata(&bars_dir)
(F) LINE   crates/store/tests/sealed_door.rs:286  [assert! message argument]  .unwrap()
(F) LINE   crates/store/tests/sealed_door.rs:287  [assert! message argument]  .file_type()
(F) LINE   crates/store/tests/sealed_door.rs:288  [assert! message argument]  .is_symlink(),
(F) LINE   crates/store/tests/sealed_door.rs:290  [assert! message argument]  bars_dir.display()
(F) LINE   crates/store/tests/sealed_door.rs:293  [assert! message argument]  !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
(F) LINE   crates/store/tests/sealed_door.rs:326  [assert_eq! message argument]  sealed.read_range(0, sealed.records()).unwrap(),
(F) BRANCH crates/store/tests/sealed_door.rs:153:17 True  [assert! condition branch (failing side)]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F) BRANCH crates/store/tests/sealed_door.rs:153:31 False  [assert! condition branch (failing side)]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F) BRANCH crates/store/tests/sealed_door.rs:231:9 False  [assert! condition branch (failing side)]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
(F) BRANCH crates/store/tests/sealed_door.rs:231:45 False  [assert! condition branch (failing side)]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
(F) BRANCH crates/store/tests/sealed_door.rs:241:9 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_door.rs:241:9 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_door.rs:241:29 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_door.rs:241:29 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) LINE   crates/store/tests/sealed_samples.rs:66  [assert! message argument]  parts.len() == 7 || parts.len() == 8,
(F) LINE   crates/store/tests/sealed_samples.rs:73  [panic! (failing arm)]  .unwrap_or_else(|| panic!("{rel}: vendor"));
(F) LINE   crates/store/tests/sealed_samples.rs:79  [panic! (failing arm)]  .unwrap_or_else(|| panic!("{rel}: rung"));
(F) LINE   crates/store/tests/sealed_samples.rs:96  [panic! (failing arm)]  let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
(F) LINE   crates/store/tests/sealed_samples.rs:169  [assert_eq! message argument]  bars[first as usize..(first + count) as usize],
(F) LINE   crates/store/tests/sealed_samples.rs:176  [assert_eq! message argument]  raw.read_record(at).unwrap(),
(F) LINE   crates/store/tests/sealed_samples.rs:177  [assert_eq! message argument]  bars[at as usize],
(F) BRANCH crates/store/tests/sealed_samples.rs:66:9 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_samples.rs:66:9 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_samples.rs:66:29 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_samples.rs:66:29 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) LINE   crates/store/tests/sealed_write_door.rs:62  [assert! message argument]  parts.len() == 7 || parts.len() == 8,
(F) LINE   crates/store/tests/sealed_write_door.rs:123  [assert! message argument]  !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
(F) LINE   crates/store/tests/sealed_write_door.rs:127  [assert! message argument]  !fs::symlink_metadata(root.join("bars"))
(F) LINE   crates/store/tests/sealed_write_door.rs:128  [assert! message argument]  .is_ok_and(|meta| meta.file_type().is_symlink()),
(F) LINE   crates/store/tests/sealed_write_door.rs:130  [assert! message argument]  root.display()
(F) LINE   crates/store/tests/sealed_write_door.rs:188  [assert_eq! message argument]  Appended::AlreadyPresent {
(F) LINE   crates/store/tests/sealed_write_door.rs:189  [assert_eq! message argument]  first_index: 0,
(F) LINE   crates/store/tests/sealed_write_door.rs:190  [assert_eq! message argument]  n_valid: held.len() as u64
(F) LINE   crates/store/tests/sealed_write_door.rs:191  [assert_eq! message argument]  },
(F) LINE   crates/store/tests/sealed_write_door.rs:198  [panic! (failing arm)]  panic!("{rel}: {refused}");
(F) LINE   crates/store/tests/sealed_write_door.rs:203  [assert_eq! message argument]  if is_last_day {
(F) LINE   crates/store/tests/sealed_write_door.rs:204  [assert_eq! message argument]  SealedOffer::Follows
(F) LINE   crates/store/tests/sealed_write_door.rs:206  [assert_eq! message argument]  SealedOffer::Disagrees(store::file::Conflict::NotHeld)
(F) LINE   crates/store/tests/sealed_write_door.rs:213  [assert_eq! message argument]  snapshot(&root.join("bars")),
(F) LINE   crates/store/tests/sealed_write_door.rs:234  [panic! (failing arm)]  panic!("{rel}: {repaired:?}");
(F) LINE   crates/store/tests/sealed_write_door.rs:241  [assert! message argument]  now == full,
(F) LINE   crates/store/tests/sealed_write_door.rs:245  [assert_eq! message argument]  snapshot(&root.join("bars")),
(F) BRANCH crates/store/tests/sealed_write_door.rs:62:9 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_write_door.rs:62:9 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_write_door.rs:62:29 True  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_write_door.rs:62:29 False  [assert! condition branch (failing side)]  parts.len() == 7 || parts.len() == 8,
(F) BRANCH crates/store/tests/sealed_write_door.rs:93:13 True  [let-else in test]  let Ok(entries) = fs::read_dir(&at) else {
(F) BRANCH crates/store/tests/sealed_write_door.rs:93:13 False  [let-else in test]  let Ok(entries) = fs::read_dir(&at) else {
(F) BRANCH crates/store/tests/sealed_write_door.rs:197:13 True  [let-else in test]  let StoreError::Sealed { offer, at, .. } = refused else {
(F) BRANCH crates/store/tests/sealed_write_door.rs:197:13 False  [let-else in test]  let StoreError::Sealed { offer, at, .. } = refused else {
(F) BRANCH crates/store/tests/sealed_write_door.rs:203:16 True  [assert_eq! condition branch (failing side)]  if is_last_day {
(F) BRANCH crates/store/tests/sealed_write_door.rs:203:16 False  [assert_eq! condition branch (failing side)]  if is_last_day {
(F) BRANCH crates/store/tests/sealed_write_door.rs:233:13 True  [let-else in test]  let MonthRepair::Repaired { added, .. } = repaired else {
(F) BRANCH crates/store/tests/sealed_write_door.rs:233:13 False  [let-else in test]  let MonthRepair::Repaired { added, .. } = repaired else {
(F) LINE   crates/store/tests/slot_audit.rs:116  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/slot_audit.rs:137  [panic! (failing arm)]  panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
(F) LINE   crates/store/tests/slot_audit.rs:277  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/slot_audit.rs:347  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/slot_audit.rs:410  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/slot_audit.rs:485  [panic! (failing arm)]  panic!("damaged");
(F) BRANCH crates/store/tests/slot_audit.rs:484:9 False  [let-else in test]  let SlotState::Damaged { reason, .. } = audit.slots[1] else {
(F) LINE   crates/store/tests/slot_audit_sealed.rs:95  [assert! message argument]  raw.slot_audit()
(F) LINE   crates/store/tests/slot_damaged_log.rs:94  [panic! (failing arm)]  other => panic!("{key}: {other:?}"),
(F) LINE   crates/store/tests/slot_damaged_log.rs:101  [panic! (failing arm)]  other => panic!("{key}: {other:?}"),
(F) LINE   crates/store/tests/slot_damaged_log.rs:192  [assert_eq! message argument]  found
(F) LINE   crates/store/tests/slot_damaged_log.rs:193  [assert_eq! message argument]  .records
(F) LINE   crates/store/tests/slot_damaged_log.rs:194  [assert_eq! message argument]  .iter()
(F) LINE   crates/store/tests/slot_damaged_log.rs:195  [assert_eq! message argument]  .map(|r| (r.target.clone(), r.message.clone()))
(F) LINE   crates/store/tests/slot_damaged_log.rs:196  [assert_eq! message argument]  .collect::<Vec<_>>()
(F) LINE   crates/store/tests/sort_index.rs:488  [assert! message argument]  index.torn()
(F) LINE   crates/store/tests/sort_index.rs:761  [assert_eq! message argument]  column.as_str()
(F) LINE   crates/store/tests/sort_index.rs:843  [assert_eq! message argument]  column.as_str()
(F) BRANCH crates/store/tests/sort_index.rs:387:62 False  [assert! condition branch (failing side)]  matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
(F) BRANCH crates/store/tests/sort_index.rs:430:56 False  [assert! condition branch (failing side)]  matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
(F) BRANCH crates/store/tests/sort_index.rs:446:63 False  [assert! condition branch (failing side)]  assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
(F) BRANCH crates/store/tests/sort_index.rs:660:58 False  [assert! condition branch (failing side)]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F) BRANCH crates/store/tests/sort_index.rs:736:58 False  [assert! condition branch (failing side)]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F) BRANCH crates/store/tests/sort_index.rs:874:16 False  [assert! condition branch (failing side)]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F) BRANCH crates/store/tests/sort_index.rs:874:54 False  [assert! condition branch (failing side)]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F) BRANCH crates/store/tests/sort_index.rs:889:16 False  [assert! condition branch (failing side)]  if reason.contains("2031-02.new.srt")),
(F) BRANCH crates/store/tests/tail_proof.rs:144:9 False  [assert! condition branch (failing side)]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F) BRANCH crates/store/tests/tail_proof.rs:144:36 False  [assert! condition branch (failing side)]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F) BRANCH crates/store/tests/tail_proof.rs:144:60 False  [assert! condition branch (failing side)]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F) LINE   crates/store/tests/tix.rs:173  [assert_eq! message argument]  timeframe.as_str()
(F) BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False  [assert! condition branch (failing side)]  }) if asked == SYMBOL + 1
(F) LINE   crates/store/tests/tix_small_month.rs:160  [panic! (failing arm)]  other => panic!("not committed: {other:?}"),
(F) LINE   crates/store/tests/tix_small_month.rs:463  [assert! message argument]  reader.time_lookup()
(F) BRANCH crates/store/tests/tix_small_month.rs:461:74 False  [assert! condition branch (failing side)]  matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
(F) LINE   crates/store/tests/tix_warn_once.rs:127  [panic! (failing arm)]  other => panic!("{key} is not a count: {other:?}"),
(F) LINE   crates/store/tests/tix_warn_once.rs:160  [panic! (failing arm)]  other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
(F) LINE   crates/store/tests/tix_warn_once.rs:162  [panic! (failing arm)]  other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
(F) LINE   crates/store/tests/unit.rs:110  [assert_eq! message argument]  input.len()
(F) LINE   crates/store/tests/unit.rs:527  [panic! (failing arm)]  _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
(F) LINE   crates/store/tests/unit.rs:1820  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/unit.rs:1859  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/unit.rs:1874  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/unit.rs:2096  [assert_eq! message argument]  vendor.as_str(),
(F) LINE   crates/store/tests/unit.rs:2826  [assert_ne! message argument]  a.as_str(),
(F) LINE   crates/store/tests/unit.rs:2827  [assert_ne! message argument]  b.as_str()
(F) LINE   crates/store/tests/unit.rs:2833  [assert_ne! message argument]  a.as_str()
(F) LINE   crates/store/tests/unit.rs:2893  [assert_eq! message argument]  tf.as_str()
(F) BRANCH crates/store/tests/unit.rs:399:13 False  [assert! condition branch (failing side)]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F) BRANCH crates/store/tests/unit.rs:399:36 False  [assert! condition branch (failing side)]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F) BRANCH crates/store/tests/unit.rs:868:13 False  [assert! condition branch (failing side)]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(F) BRANCH crates/store/tests/unit.rs:868:54 False  [assert! condition branch (failing side)]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(F) BRANCH crates/store/tests/unit.rs:1475:32 False  [assert! condition branch (failing side)]  read == old || read == new,
(F) BRANCH crates/store/tests/unit.rs:2047:26 False  [assert! condition branch (failing side)]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F) BRANCH crates/store/tests/unit.rs:2047:43 False  [assert! condition branch (failing side)]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F) BRANCH crates/store/tests/unit.rs:2173:13 False  [assert! condition branch (failing side)]  err.to_string().contains(field) && err.to_string().contains("case"),
(F) BRANCH crates/store/tests/unit.rs:2173:48 False  [assert! condition branch (failing side)]  err.to_string().contains(field) && err.to_string().contains("case"),
(F) LINE   crates/store/tests/verdict.rs:129  [panic! (failing arm)]  Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
(F) LINE   crates/store/tests/verdict.rs:168  [panic! (failing arm)]  other => panic!("100x must be refused table_too_large, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:395  [panic! (failing arm)]  other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:450  [panic! (failing arm)]  panic!("{whole} must be found");
(F) LINE   crates/store/tests/verdict.rs:728  [panic! (failing arm)]  other => panic!("expected image_missing, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:793  [panic! (failing arm)]  other => panic!("expected generation_collision, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:1318  [panic! (failing arm)]  other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:1821  [panic! (failing arm)]  other => panic!(
(F) LINE   crates/store/tests/verdict.rs:1823  [panic! (failing arm)]  clean.len()
(F) LINE   crates/store/tests/verdict.rs:1856  [panic! (failing arm)]  other => panic!("expected record_corrupt, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:2283  [assert_eq! message argument]  feed.as_str(),
(F) LINE   crates/store/tests/verdict.rs:2284  [assert_eq! message argument]  timeframe.as_str()
(F) LINE   crates/store/tests/verdict.rs:2435  [panic! (failing arm)]  other => panic!("expected lock_stalled, got {other:?}"),
(F) LINE   crates/store/tests/verdict.rs:2455  [assert! message argument]  matches!(
(F) LINE   crates/store/tests/verdict.rs:2549  [assert! message argument]  image.display()
(F) LINE   crates/store/tests/verdict.rs:2554  [panic! (failing arm)]  other => panic!("expected index_missing, got {other}"),
(F) LINE   crates/store/tests/verdict.rs:2730  [panic! (failing arm)]  other => panic!("{other}"),
(F) BRANCH crates/store/tests/verdict.rs:228:13 False  [assert! condition branch (failing side)]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F) BRANCH crates/store/tests/verdict.rs:228:36 False  [assert! condition branch (failing side)]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F) BRANCH crates/store/tests/verdict.rs:449:13 False  [let-else in test]  let Found::Record { ordinal, record } = &answer.found else {
(F) BRANCH crates/store/tests/verdict.rs:1890:17 False  [assert! condition branch (failing side)]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F) BRANCH crates/store/tests/verdict.rs:1890:40 False  [assert! condition branch (failing side)]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F) BRANCH crates/store/tests/verdict.rs:2123:17 False  [assert! condition branch (failing side)]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F) BRANCH crates/store/tests/verdict.rs:2123:40 False  [assert! condition branch (failing side)]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F) BRANCH crates/store/tests/verdict.rs:2358:58 False  [assert! condition branch (failing side)]  matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
(F) BRANCH crates/store/tests/verdict.rs:2545:21 False  [assert! condition branch (failing side)]  image.extension().is_some_and(|ext| ext == "vdt")
(F) BRANCH crates/store/tests/verdict.rs:2546:28 False  [assert! condition branch (failing side)]  && image.parent().is_some_and(|dir| dir.ends_with("2031"))
(F) BRANCH crates/store/tests/verdict.rs:2547:28 False  [assert! condition branch (failing side)]  && image.exists(),
(F) BRANCH crates/store/tests/verdict.rs:2604:13 False  [assert! condition branch (failing side)]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F) BRANCH crates/store/tests/verdict.rs:2604:22 False  [assert! condition branch (failing side)]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F) BRANCH crates/store/tests/verdict.rs:2782:9 False  [assert! condition branch (failing side)]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F) BRANCH crates/store/tests/verdict.rs:2782:61 False  [assert! condition branch (failing side)]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F) LINE   crates/store/tests/write.rs:1402  [assert! message argument]  sidecar.display()
(F) LINE   crates/store/tests/write.rs:1419  [assert! message argument]  sidecar.display()
(F) LINE   crates/store/tests/write.rs:1619  [panic! (failing arm)]  panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
(F) LINE   crates/store/tests/write.rs:1663  [panic! (failing arm)]  panic!(
(F) BRANCH crates/store/tests/write.rs:1564:13 False  [assert! condition branch (failing side)]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F) BRANCH crates/store/tests/write.rs:1564:62 False  [assert! condition branch (failing side)]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F) BRANCH crates/store/tests/write.rs:2103:85 False  [assert! condition branch (failing side)]  Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
(F) LINE   crates/telemetry/src/level.rs:142  [assert_eq! message argument]  level.rank()
(F) LINE   crates/telemetry/src/record.rs:826  [assert! message argument]  matches!(
(F) BRANCH crates/telemetry/src/run_dirs.rs:234:21 False  [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F) BRANCH crates/telemetry/src/run_dirs.rs:234:44 False  [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F) BRANCH crates/telemetry/src/run_dirs.rs:240:21 False  [assert! condition branch (failing side)]  !other.sub.starts_with(&format!("{}/", dir.sub))
(F) BRANCH crates/telemetry/src/run_dirs.rs:241:28 False  [assert! condition branch (failing side)]  && !dir.sub.starts_with(&format!("{}/", other.sub)),
(F) LINE   crates/telemetry/src/sink.rs:4668  [panic! (failing arm)]  panic!(
(F) LINE   crates/telemetry/src/sink.rs:4670  [panic! (failing arm)]  path.display(),
(F) LINE   crates/telemetry/src/sink.rs:4671  [panic! (failing arm)]  String::from_utf8_lossy(line)
(F) LINE   crates/telemetry/src/sink.rs:5061  [assert! message argument]  written[0].at_unix_millis
(F) LINE   crates/telemetry/src/sink.rs:5098  [assert! message argument]  pair[1].seq,
(F) LINE   crates/telemetry/src/sink.rs:5099  [assert! message argument]  pair[1].at_unix_millis,
(F) LINE   crates/telemetry/src/sink.rs:5100  [assert! message argument]  pair[0].seq,
(F) LINE   crates/telemetry/src/sink.rs:5101  [assert! message argument]  pair[0].at_unix_millis
(F) LINE   crates/telemetry/src/sink.rs:5431  [assert! message argument]  span * 2
(F) LINE   crates/telemetry/src/sink.rs:5650  [assert! message argument]  path.display()
(F) LINE   crates/telemetry/src/sink.rs:6034  [assert! message argument]  rolled.health()
(F) LINE   crates/telemetry/src/sink.rs:7618  [panic! (failing arm)]  other => panic!("{dimension}={value}: {other:?}"),
(F) LINE   crates/telemetry/src/sink.rs:8046  [panic! (failing arm)]  panic!("the reaper did not finish: {:?}", sink.audit_health());
(F) LINE   crates/telemetry/src/sink.rs:8897  [panic! (failing arm)]  panic!("{pointer:?}");
(F) LINE   crates/telemetry/src/sink.rs:10221  [panic! (failing arm)]  AuditLine::Unreadable(why) => panic!("{dimension}={value}: {why}"),
(F) BRANCH crates/telemetry/src/sink.rs:4979:17 False  [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F) BRANCH crates/telemetry/src/sink.rs:4979:65 False  [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F) BRANCH crates/telemetry/src/sink.rs:5667:17 False  [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F) BRANCH crates/telemetry/src/sink.rs:5667:40 False  [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F) BRANCH crates/telemetry/src/sink.rs:5712:13 False  [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F) BRANCH crates/telemetry/src/sink.rs:5712:30 False  [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F) BRANCH crates/telemetry/src/sink.rs:6576:13 False  [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6576:40 False  [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6642:17 False  [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F) BRANCH crates/telemetry/src/sink.rs:6642:54 False  [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F) BRANCH crates/telemetry/src/sink.rs:6662:13 False  [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),
(F) BRANCH crates/telemetry/src/sink.rs:6662:40 False  [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),
(F) BRANCH crates/telemetry/src/sink.rs:6704:13 False  [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6704:53 False  [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:7986:13 False  [let-else in test]  let Ok(entries) = std::fs::read_dir(root) else {
(F) BRANCH crates/telemetry/src/sink.rs:8154:21 False  [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());
(F) BRANCH crates/telemetry/src/sink.rs:8154:48 False  [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());
(F) BRANCH crates/telemetry/src/sink.rs:8896:21 False  [let-else in test]  let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
(F) BRANCH crates/telemetry/src/sink.rs:11041:17 False  [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F) BRANCH crates/telemetry/src/sink.rs:11041:57 False  [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F) BRANCH crates/telemetry/src/sink.rs:11051:36 False  [assert! condition branch (failing side)]  .is_some_and(|why| why.contains("first line does not decode")
(F) BRANCH crates/telemetry/src/sink.rs:11106:17 False  [assert! condition branch (failing side)]  why.contains("last 64 KiB hold no line") && !why.contains("first line")
(F) BRANCH crates/telemetry/src/sink.rs:11172:17 False  [assert! condition branch (failing side)]  why.contains("first line does not decode") && !why.contains("last 64 KiB")
(F) BRANCH crates/telemetry/src/sink.rs:11177:17 False  [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");
(F) BRANCH crates/telemetry/src/sink.rs:11177:28 False  [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");
(F) BRANCH crates/telemetry/src/sink.rs:11208:17 False  [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F) BRANCH crates/telemetry/src/sink.rs:11208:44 False  [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F) LINE   crates/telemetry/src/state_events_tests.rs:85  [panic! (failing arm)]  || panic!("{path} is named by a row and not read by this test"),
(F) LINE   crates/telemetry/src/state_events_tests.rs:105  [assert! message argument]  level.label(),
(F) LINE   crates/telemetry/src/state_events_tests.rs:113  [assert! message argument]  level.label()
(F) BRANCH crates/telemetry/src/state_events_tests.rs:176:17 False  [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F) BRANCH crates/telemetry/src/state_events_tests.rs:176:43 False  [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F) BRANCH crates/telemetry/src/state_events_tests.rs:321:13 False  [assert! condition branch (failing side)]  site.contains(&format!("\"{}\"", row.message))
(F) BRANCH crates/telemetry/src/state_events_tests.rs:322:20 False  [assert! condition branch (failing side)]  && site.contains(&format!("\"{}\"", row.target)),
(F) BRANCH crates/telemetry/src/tail.rs:1862:17 False  [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);
(F) BRANCH crates/telemetry/src/tail.rs:1862:31 False  [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);
(F) LINE   crates/telemetry/tests/cloud_mutants_sink.rs:263  [panic! (failing arm)]  other => panic!("a directory is not a rolled-off line: {other:?}"),
(F) BRANCH crates/vocab/src/expression_search.rs:387:13 False  [let-else in test]  let Some(remaining) = length.checked_sub(code.len()) else {
(F) BRANCH crates/vocab/src/expression_search.rs:403:25 False  [let-else in test]  let Some(&left) = starts.get(depth - 2) else {
(F) BRANCH crates/vocab/src/expression_search.rs:406:25 False  [let-else in test]  let Some(&right) = starts.get(depth - 1) else {
(F) BRANCH crates/vocab/src/expression_search.rs:429:13 False  [let-else in test]  let Ok(length) = u16::try_from(length) else {
(F) BRANCH crates/vocab/src/expression_search.rs:435:17 False  [let-else in test]  let Some(slot) = cursor.code.get_mut(at) else {
(F) BRANCH crates/vocab/src/expression_search.rs:486:13 False  [assert! condition branch (failing side)]  compared > 100_000 && admitted > 1_000,
(F) BRANCH crates/vocab/src/expression_search.rs:486:35 False  [assert! condition branch (failing side)]  compared > 100_000 && admitted > 1_000,
(F) BRANCH crates/vocab/src/expression_search.rs:495:21 False  [assert! condition branch (failing side)]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F) BRANCH crates/vocab/src/expression_search.rs:495:46 False  [assert! condition branch (failing side)]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F) BRANCH crates/vocab/src/expression_search.rs:606:17 False  [assert! condition branch (failing side)]  assert!(choice < call && call < paused);
(F) BRANCH crates/vocab/src/expression_search.rs:606:34 False  [assert! condition branch (failing side)]  assert!(choice < call && call < paused);
(F) BRANCH crates/vocab/src/expression_search.rs:651:17 False  [assert! condition branch (failing side)]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F) BRANCH crates/vocab/src/expression_search.rs:651:54 False  [assert! condition branch (failing side)]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F) BRANCH crates/vocab/src/expression_search.rs:722:17 False  [assert! condition branch (failing side)]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F) BRANCH crates/vocab/src/expression_search.rs:722:37 False  [assert! condition branch (failing side)]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F) LINE   crates/vocab/src/implication.rs:329  [assert! message argument]  rows.len()
(F) BRANCH crates/vocab/src/implication.rs:292:63 True  [assert! condition branch (failing side)]  assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));
(F) BRANCH crates/vocab/src/mask.rs:419:46 False  [assert! condition branch (failing side)]  !bar.hits(&wider) || bar.hits(&base),
(F) BRANCH crates/vocab/src/mask.rs:661:17 False  [assert! condition branch (failing side)]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) BRANCH crates/vocab/src/mask.rs:661:32 False  [assert! condition branch (failing side)]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) BRANCH crates/vocab/src/mask.rs:661:49 False  [assert! condition branch (failing side)]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) BRANCH crates/vocab/src/table.rs:1952:13 False  [assert! condition branch (failing side)]  worst_hit <= 4 && worst_miss <= 7,
(F) BRANCH crates/vocab/src/table.rs:1952:31 False  [assert! condition branch (failing side)]  worst_hit <= 4 && worst_miss <= 7,
(F) BRANCH crates/vocab/src/table.rs:2009:17 False  [assert! condition branch (failing side)]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) BRANCH crates/vocab/src/table.rs:2009:34 False  [assert! condition branch (failing side)]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) BRANCH crates/vocab/src/table.rs:2009:52 False  [assert! condition branch (failing side)]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) BRANCH crates/vocab/src/table.rs:2083:17 False  [assert! condition branch (failing side)]  assert!(m.get(0) && m.popcount() == 1);
(F) BRANCH crates/vocab/src/table.rs:2083:29 False  [assert! condition branch (failing side)]  assert!(m.get(0) && m.popcount() == 1);
(F) BRANCH crates/vocab/src/tolerance.rs:570:45 True  [assert! condition branch (failing side)]  !(shared && range >= RUNG_EXCLUSIVE_MIN_RANGE),
(F) LINE   crates/vocab/tests/expression.rs:403  [assert_eq! message argument]  bytes.get(..16)
(F) BRANCH crates/vocab/tests/expression.rs:414:13 False  [assert! condition branch (failing side)]  assert!(accepted > 0 && refused > accepted);
(F) BRANCH crates/vocab/tests/expression.rs:414:29 False  [assert! condition branch (failing side)]  assert!(accepted > 0 && refused > accepted);
(F) LINE   crates/vocab/tests/expression_search.rs:75  [panic! (failing arm)]  Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),
(F) BRANCH crates/vocab/tests/expression_search.rs:193:13 False  [assert! condition branch (failing side)]  assert!(saw_candidate && saw_backtrack);
(F) BRANCH crates/vocab/tests/expression_search.rs:193:30 False  [assert! condition branch (failing side)]  assert!(saw_candidate && saw_backtrack);
(F) LINE   crates/vocab/tests/expression_search_readiness.rs:127  [panic! (failing arm)]  panic!("the last NOT must finish the maximum-capacity program");
(F) BRANCH crates/vocab/tests/expression_search_readiness.rs:126:9 False  [let-else in test]  let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {
(F) BRANCH crates/vocab/tests/mask.rs:307:17 True  [assert! condition branch (failing side)]  before || !after,
(F) BRANCH crates/vocab/tests/mask.rs:307:27 False  [assert! condition branch (failing side)]  before || !after,
(F) BRANCH crates/vocab/tests/no_float.rs:145:36 False  [assert! condition branch (failing side)]  *module == "lib.rs" || declared.contains(*module),
(F) LINE   crates/vocab/tests/no_py_language.rs:276  [assert! message argument]  root.display(),
(F) LINE   crates/vocab/tests/no_py_language.rs:277  [assert! message argument]  String::from_utf8_lossy(&listed.stderr).trim()
(F) LINE   crates/vocab/tests/no_py_language.rs:437  [panic! (failing arm)]  panic!("{why}");
(F) LINE   crates/vocab/tests/no_py_language.rs:646  [assert! message argument]  String::from_utf8_lossy(&init.stderr)
(F) LINE   crates/vocab/tests/no_py_language.rs:659  [assert! message argument]  String::from_utf8_lossy(&add.stderr)
(F) LINE   crates/vocab/tests/no_py_language.rs:754  [assert! message argument]  String::from_utf8_lossy(&init.stderr)
(F) LINE   crates/vocab/tests/no_py_language.rs:786  [assert! message argument]  String::from_utf8_lossy(&add.stderr)
(F) LINE   crates/vocab/tests/no_runtime_spawn.rs:2560  [assert! message argument]  sources.len()
(F) LINE   crates/vocab/tests/no_runtime_spawn.rs:2585  [assert! message argument]  failures.join("\n  ")
(F) BRANCH crates/vocab/tests/no_runtime_spawn.rs:671:9 False  [let-else in test]  let Some(name) = ident(t.get(i)) else {
(F) BRANCH crates/vocab/tests/no_runtime_spawn.rs:2910:26 False  [assert! condition branch (failing side)]  .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),
(F) LINE   crates/vocab/tests/stale_claims.rs:158  [assert! message argument]  with_commas(count)
(F) LINE   crates/vocab/tests/table.rs:125  [assert! message argument]  doc.len(),
(F) LINE   crates/vocab/tests/table.rs:271  [unreachable! (failing arm)]  unreachable!("235..=273 are void")
(F) LINE   crates/vocab/tests/table.rs:432  [assert! message argument]  first - 1,
(F) BRANCH crates/vocab/tests/table.rs:270:13 False  [let-else in test]  let BitStatus::Void { reason } = def.status else {
(F) BRANCH crates/vocab/tests/table.rs:373:13 False  [assert! condition branch (failing side)]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) BRANCH crates/vocab/tests/table.rs:373:39 False  [assert! condition branch (failing side)]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) BRANCH crates/vocab/tests/table.rs:373:63 False  [assert! condition branch (failing side)]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) BRANCH crates/vocab/tests/table.rs:561:45 False  [assert! condition branch (failing side)]  n == ConditionMask::BITS || ALLOWED.contains(&n),
(F) BRANCH crates/vocab/tests/table.rs:813:17 False  [assert! condition branch (failing side)]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F) BRANCH crates/vocab/tests/table.rs:813:54 False  [assert! condition branch (failing side)]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F) BRANCH crates/vocab/tests/table.rs:819:21 False  [assert! condition branch (failing side)]  status_cells.contains("retired")
(F) BRANCH crates/vocab/tests/table.rs:820:28 False  [assert! condition branch (failing side)]  && status_cells.contains(&format!("duplicates {duplicate_of}")),
(F) BRANCH crates/vocab/tests/table.rs:843:9 False  [assert! condition branch (failing side)]  retired_rows >= retired && void_rows >= void,
(F) BRANCH crates/vocab/tests/table.rs:843:36 False  [assert! condition branch (failing side)]  retired_rows >= retired && void_rows >= void,
(F) BRANCH crates/vocab/tests/table.rs:1012:13 False  [assert! condition branch (failing side)]  !name.contains("vix") && !name.contains("volatility_index"),
(F) BRANCH crates/vocab/tests/table.rs:1012:38 False  [assert! condition branch (failing side)]  !name.contains("vix") && !name.contains("volatility_index"),
(F) LINE   crates/vocab/tests/workspace_is_rust.rs:396  [assert! message argument]  out.len()
(F) LINE   crates/vocab/tests/workspace_is_rust.rs:752  [assert! message argument]  named.join("\n  ")

(U) ITEMS, each with the proof from the code (recipe v4 step 4):

(U) crates/costs/src/stack.rs:907 `)?)` (to_rupee's narrowing refusal). Proof: half_up::<100>(p)*100 for every i64 p lies in [-9_223_372_036_854_775_800, 9_223_372_036_854_775_800]: i64::MAX ends in 07 (rounds down), the largest p that rounds up (..799) lands on ..800, i64::MIN's Euclidean remainder is 92 (rounds towards zero to -..800). Pinned by stack::tests::rounding_to_the_rupee_holds_at_both_ends_of_the_type.
(U) crates/costs/src/stack.rs:720 `)?;` (units_of(square_off_fees, GST, 1) in settle_dhan). Proof: exact.rs units_of refuses only for divisor <= 0 (here 1) or when value*num*10^7 leaves i128; GST is 18/100, |value| <= 2^63, so |value*18*10^7| < 2^63*2^5*2^24 = 2^92 < 2^127; round_half_up with denominator 100 > 0 cannot fail.
(U) crates/costs/src/stack.rs:589 `)?;` (add(levied.stt, levied.stt_exercise)). Proof: both addends are levies of a shipped STT rate below 1% on a paisa value <= i64::MAX (taxes(): Levied::of and stt_exercise.rate().apply), so each is below i64::MAX/100 and their sum below i64::MAX/50.
(U) crates/costs/src/ledger.rs:765-768 the closure of `const _: fn() = || {..}`. Proof: an unnamed `const _` item has no path, so no code can call the closure; it exists only to be type-checked (the compile-time no-Default check). Its trait item `witness` (line 757) IS called, by ledger::tests::the_no_default_witness_is_the_every_type_item_for_both_checked_types.
(U) crates/engine/src/keep.rs:398 False side of `if let Some(evicted) = self.take_root()`. Proof: the enclosing `if` holds only when self.held.first() is Some, so held.len() >= 1 and take_root's `checked_sub(1)?` (keep.rs:439) cannot return None.
(U) crates/core/src/alias.rs:692:27, 701:27, 811:27, 820:27 False side of `row.<key> == <key>` inside `.filter(..)`. Proof: MemberIndex::position (universe.rs:4433) answers only a slot whose stored key equals the probe, and each index is built from the same rows' keys (alias.rs:650-667), so the row it names always carries that key; by_fno refuses the empty key (rows with no F&O name are indexed under "") before probing.
(U) crates/lake/src/reader.rs:727, 737, 749 (`_ => Err(self.mismatch(name))`) and 754-770 (`fn mismatch`). Proof: a LakeFile exists only through open/from_bytes, which refuse any column whose physical type is not its layout's spec (schema.rs:154, ColumnTypeMismatch, driven by tests/synthetic.rs a_wrongly_typed_column_is_refused_by_name); its metadata is never changed after; and parquet's get_column_reader picks the reader from that same descriptor's physical type, so the Int64/Int32/Double arm always matches.
(U, no code) crates/core/src/lib.rs, crates/costs/src/lib.rs, crates/greeks/src/lib.rs, crates/lake/src/lib.rs, crates/store/src/lib.rs: Route B prints NOT READ because llvm-cov maps no region in them; each holds only module declarations, re-exports, attributes and doc comments (no fn body, no const evaluated at run time), so they are 0/0, not gaps. A later attempt re-checks this with grep -n 'fn ' on each at its tip.
(NOT MEASURED, route) every benches/ file of the ten crates (13 files): their [[bench]] targets are test = false, so cargo test --no-run builds no object and Route B cannot read them. NOT PROVED; an operator item (docs/06-limits.md L-BCL-1), never counted covered.

ATTEMPT 2 (28 Oct 2026, tip after WIP 42737ba18; line numbers as in that tip). (U) rows added, each read by hand:
(U) crates/core/src/alias.rs:692:27, 701:27, 811:27, 820:27 False of the FIRST operand (`row.fno == Some(fno)`, `row.store_key == key`, `row.old == old`, `row.new == new`). Proof: `MemberIndex::position` (crates/core/src/universe.rs:4433) answers Some(at) only for the slot whose stored member equals the asked string, and `Indices::build` / `Lineage::build` store exactly rows[i].fno (or ""), rows[i].store_key, rows[i].old, rows[i].new at position i; `by_fno` refuses "" before it probes. So the row found always carries the asked name; only the `status.applies()` operand can be false, and it is (an_unverified_row_is_listed_and_never_applied).
(U) crates/store/src/settings.rs:220:16 False (`if let Some(slot) = self.values.get_mut(key.slot())`). Proof: `values` is `[Option<String>; KEYS]` and `Key::slot` answers 0..KEYS-1 for every variant (its match), so `get_mut` is always Some.
Closed by restructure instead of (U) (D-7324, D-7325): quarantine.rs encode_slot/empty_image/seal_header/put/set_described get_mut arms, note_damaged/note_cut if-let arms; generation.rs Pointer::encode get_mut arms; file.rs write_sealed/write_sealed_refused let-else arms; catalog.rs classify's get(0..5) let-else; sort_index.rs columns.get_mut arms.
`````

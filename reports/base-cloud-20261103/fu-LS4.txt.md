# fu-LS4.txt.md

BASE-COV-LIBS-LS4's (F)/(U) list at code tip `bcbc6524bdb29351f5d211aed11e4f7b76795b7b` (recipe v4 step 4). One fenced block below holds it byte for byte; sha256 of the fenced content: `557ef891a5c441729eb2b3e4e7ff71ab5c13b3634d7662947228bf1b8bb04e00`.

`````text
BASE-COV-LIBS-LS4, Route B on tip bcbc6524bdb29351f5d211aed11e4f7b76795b7b (crates core, costs, engine, greeks, telemetry; whole files; non-root run).
Pin: rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov/llvm-profdata LLVM version 23.1.0-rust-1.99.0-nightly, x86_64-unknown-linux-gnu.
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', CARGO_PROFILE_DEV_OPT_LEVEL=0, kit d3-route-b-cloud.sh (unchanged) with d3-covset.mjs.
Test binaries ran as uid 65534 (nobody) through CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups', umask 000 for the profile folders.

(F)/(U) LIST: 229 rows, (F) 227, (U) 2.
(F) by recipe v4 step 4: the failing arm of assert!/assert_eq!/assert_ne!/panic!/unreachable! inside test code (a #[test] body, a #[cfg(test)] module or file, a tests/ file). First sorted by a brace-matching classifier (scratch classify.mjs: an || operand's False side is (F) only for the last operand; a branch inside a closure in an assert is read by hand), then every row not plainly one of those read by hand. (U) rows carry their proof; they hold on every target, aarch64-apple-darwin included.

(F) LINE   crates/core/src/alias.rs:1276  [panic!/unreachable! (failing arm)]  other => panic!("{asked}: {other:?}"),
(F) LINE   crates/core/src/alias.rs:1320  [panic!/unreachable! (failing arm)]  other => panic!("{asked}: {other:?}"),
(F) BRANCH crates/core/src/alias.rs:1818:58 False  [assert! condition branch (failing side)]  assert!(matches!(got.via, Via::Index(row) if row.store_key == key));
(F) BRANCH crates/core/src/alias.rs:1827:61 False  [assert! condition branch (failing side)]  assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));
(F) BRANCH crates/core/src/alias.rs:1866:36 False  [assert! condition branch (failing side)]  assert!(hit || !name.is_empty());
(F) BRANCH crates/core/src/instrument.rs:779:17 False  [assert! condition branch (failing side)]  assert!(a < b && b < c);
(F) BRANCH crates/core/src/instrument.rs:779:26 False  [assert! condition branch (failing side)]  assert!(a < b && b < c);
(F) BRANCH crates/core/src/instrument.rs:1698:17 False  [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F) BRANCH crates/core/src/instrument.rs:1698:55 False  [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F) LINE   crates/core/src/universe.rs:4837  [panic!/unreachable! (failing arm)]  panic!("{m} is a member, yet its probe reached an empty slot");
(F) LINE   crates/core/src/universe.rs:5634  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  u.bits()
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
(F) BRANCH crates/core/src/universe.rs:4836:21 False  [let-else in test whose else is a panic! (failing side)]  let Some(held) = idx.slots[at] else {
(F) BRANCH crates/core/src/universe.rs:5628:17 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_50)
(F) BRANCH crates/core/src/universe.rs:5629:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_100)
(F) BRANCH crates/core/src/universe.rs:5630:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_200)
(F) BRANCH crates/core/src/universe.rs:5631:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_500)
(F) BRANCH crates/core/src/universe.rs:5632:24 False  [assert! condition branch (failing side)]  && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5640:17 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5640:52 False  [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/core/src/universe.rs:5792:17 False  [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F) BRANCH crates/core/src/universe.rs:5792:46 False  [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F) LINE   crates/core/src/vendor.rs:3066  [panic!/unreachable! (failing arm)]  panic!("{m} is a member, yet its probe reached an empty slot");
(F) BRANCH crates/core/src/vendor.rs:3065:25 False  [let-else in test whose else is a panic! (failing side)]  let Some(held) = idx.slots[at] else {
(F) BRANCH crates/core/tests/banned_lists.rs:113:9 False  [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F) BRANCH crates/core/tests/banned_lists.rs:113:33 False  [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F) BRANCH crates/core/tests/claim_reader.rs:125:13 False  [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F) BRANCH crates/core/tests/claim_reader.rs:125:33 False  [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F) LINE   crates/core/tests/cost_invariants.rs:88  [panic!/unreachable! (failing arm)]  panic!(
(F) LINE   crates/core/tests/cost_invariants.rs:90  [panic!/unreachable! (failing arm)]  path.display()
(F) LINE   crates/core/tests/cost_invariants.rs:109  [panic!/unreachable! (failing arm)]  panic!(
(F) LINE   crates/core/tests/cost_invariants.rs:111  [panic!/unreachable! (failing arm)]  crates_dir.display()
(F) LINE   crates/core/tests/cost_invariants.rs:136  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  crates_dir.display()
(F) LINE   crates/core/tests/cost_invariants.rs:364  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()
(F) LINE   crates/core/tests/cost_invariants.rs:371  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  printed.len(),
(F) LINE   crates/core/tests/cost_invariants.rs:372  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  benches.len()
(F) LINE   crates/core/tests/decision_citations.rs:243  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  path.display()
(F) LINE   crates/core/tests/decision_citations.rs:313  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  cited.len()
(F) LINE   crates/core/tests/decision_citations.rs:323  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  missing.len()
(F) LINE   crates/core/tests/decision_citations.rs:419  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  citations(&none[0].1)
(F) LINE   crates/core/tests/decision_citations.rs:605  [panic!/unreachable! (failing arm)]  panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
(F) LINE   crates/core/tests/decision_citations.rs:608  [panic!/unreachable! (failing arm)]  || panic!("{file} is not under a crate's src/"),
(F) LINE   crates/core/tests/findings.rs:94  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  cells.len()
(F) LINE   crates/core/tests/findings.rs:296  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()
(F) LINE   crates/core/tests/findings.rs:336  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  counted + rest,
(F) LINE   crates/core/tests/findings.rs:337  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()
(F) LINE   crates/core/tests/findings.rs:364  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()
(F) LINE   crates/core/tests/findings.rs:609  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  String::from_utf8_lossy(&output.stderr)
(F) LINE   crates/core/tests/findings.rs:993  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  String::from_utf8_lossy(&output.stderr)
(F) BRANCH crates/core/tests/findings.rs:369:13 False  [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,
(F) BRANCH crates/core/tests/findings.rs:369:33 False  [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,
(F) BRANCH crates/core/tests/graph.rs:528:33 False  [assert! condition branch (failing side)]  !deps.is_empty() || claim.to_lowercase().contains("nothing"),
(F) BRANCH crates/core/tests/graph.rs:563:44 False  [assert! condition branch (failing side)]  !name.contains('*') && !name.contains('/') && !name.is_empty(),
(F) LINE   crates/core/tests/lint.rs:139  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  &next_fn[..next_fn.find('(').unwrap_or(0)],
(F) LINE   crates/core/tests/zerodha_index.rs:60  [panic!/unreachable! (failing arm)]  panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
(F) BRANCH crates/core/tests/zerodha_index.rs:59:13 False  [let-else in test whose else is a panic! (failing side)]  let Decoded::Keep(listing) = decoded else {
(F) BRANCH crates/costs/src/day.rs:897:25 False  [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F) BRANCH crates/costs/src/day.rs:897:57 False  [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F) LINE   crates/costs/src/fill.rs:1010  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  best.buy().raw(),
(F) LINE   crates/costs/src/fill.rs:1011  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  worst.buy().raw()
(F) LINE   crates/costs/src/fill.rs:1017  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  best.sell().raw(),
(F) LINE   crates/costs/src/fill.rs:1018  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  worst.sell().raw()
(F) LINE   crates/costs/src/ledger.rs:1345  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  LINE_NAMES[at],
(F) LINE   crates/costs/src/ledger.rs:1346  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  summed[at],
(F) LINE   crates/costs/src/ledger.rs:1347  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  closed[at]
(F) LINE   crates/costs/src/lot.rs:1498  [panic!/unreachable! (failing arm)]  panic!("{subject} {on:?}: {:?}", got.basis);
(F) LINE   crates/costs/src/lot.rs:1590  [panic!/unreachable! (failing arm)]  panic!("{subject} {on:?}: an unread revision answered a lot");
(F) LINE   crates/costs/src/lot.rs:1614  [matches! no-match arm inside assert! (failing side): the line holds only the expansion's _ => false region]  matches!(
(F) BRANCH crates/costs/src/lot.rs:1497:21 False  [let-else in test whose else is a panic! (failing side)]  let LotBasis::ContractRule(citation) = got.basis else {
(F) BRANCH crates/costs/src/lot.rs:1554:25 False  [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F) BRANCH crates/costs/src/lot.rs:1554:57 False  [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F) BRANCH crates/costs/src/lot.rs:1589:17 False  [let-else in test whose else is a panic! (failing side)]  let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
(F) LINE   crates/costs/src/lot_table_tests.rs:146  [panic!/unreachable! (failing arm)]  other => panic!("expected the unverified row, got {other:?}"),
(F) LINE   crates/costs/src/lot_table_tests.rs:156  [panic!/unreachable! (failing arm)]  other => panic!("expected before-first-row, got {other:?}"),
(F) BRANCH crates/costs/src/lot_table_tests.rs:112:52 False  [assert! condition branch (failing side)]  assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
(F) BRANCH crates/costs/src/lot_table_tests.rs:117:25 False  [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F) BRANCH crates/costs/src/lot_table_tests.rs:117:56 False  [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F) BRANCH crates/costs/src/lot_table_tests.rs:187:56 False  [assert! condition branch (failing side)]  Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
(F) BRANCH crates/costs/src/money.rs:383:17 False  [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);
(F) BRANCH crates/costs/src/money.rs:383:31 False  [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);
(F) LINE   crates/costs/src/schedule.rs:1213  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1280  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1288  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()
(F) LINE   crates/costs/src/schedule.rs:1634  [panic!/unreachable! (failing arm)]  panic!("an options card carries option taxes");
(F) LINE   crates/costs/src/schedule.rs:1643  [panic!/unreachable! (failing arm)]  (card, trip) => panic!(
(F) BRANCH crates/costs/src/schedule.rs:1244:75 False  [assert! condition branch (failing side)]  matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
(F) BRANCH crates/costs/src/schedule.rs:1633:29 False  [let-else in test whose else is a panic! (failing side)]  let Taxes::Options { stt_sell, .. } = card.taxes() else {
(F) LINE   crates/costs/src/scope.rs:249  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  lines[arm]
(U) LINE   crates/costs/src/stack.rs:720  [Exact::sum of the calculator day total cannot overflow i128: page_lines (dhan_calc.rs) multiplies the brokerage by GST_PERCENT=18 unguarded, so any brokerage past i128::MAX/18 panics there first (overflow-checks are on in every profile); a day that gets through has |page.total| <= 1.18 x i128::MAX/18 + 2^80, about 0.066 x i128::MAX, and square_off (Exact::of_paisa of an i64, 10^7 units a paisa) and its GST add under 2^87; the sum stays far inside i128 for every Totals, a hand-built one included]  )?;
(U) LINE   crates/costs/src/stack.rs:907  [to_rupee narrows half_up::<100>(amount) x 100 to i64: for every i64 amount, |amount / 100| <= 92233720368547758.08, its nearest whole rupee has magnitude <= 92233720368547758, and x 100 that is <= 9223372036854775800 < 2^63, so the narrowing never fails; measured at both ends by stack.rs rounding_to_the_rupee_holds_at_both_ends_of_the_type]  )?)
(F) BRANCH crates/costs/src/stack.rs:2681:17 False  [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F) BRANCH crates/costs/src/stack.rs:2681:43 False  [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F) LINE   crates/costs/tests/adversarial.rs:114  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  c.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:115  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  f.raw()
(F) LINE   crates/costs/tests/adversarial.rs:121  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  c.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:122  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  f.raw()
(F) LINE   crates/costs/tests/adversarial.rs:134  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:143  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:151  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:263  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  n.raw(),
(F) LINE   crates/costs/tests/adversarial.rs:264  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()
(F) LINE   crates/costs/tests/adversarial.rs:301  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  out.map(Paisa::raw)
(F) BRANCH crates/costs/tests/adversarial.rs:128:21 False  [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F) BRANCH crates/costs/tests/adversarial.rs:128:44 False  [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F) BRANCH crates/costs/tests/adversarial.rs:194:21 False  [assert! condition branch (failing side)]  ceil.is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:195:28 False  [assert! condition branch (failing side)]  && floor.is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:196:28 False  [assert! condition branch (failing side)]  && levy_ceiling(notional, rate).is_ok()
(F) BRANCH crates/costs/tests/adversarial.rs:197:28 False  [assert! condition branch (failing side)]  && statutory_levy(notional, rate).is_ok(),
(F) LINE   crates/costs/tests/dhan_calculator.rs:666  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  case.page[7]
(F) LINE   crates/costs/tests/dhan_calculator.rs:931  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  case.name, dated_lines[at].0
(F) BRANCH crates/engine/src/keep.rs:855:17 False  [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F) BRANCH crates/engine/src/keep.rs:855:39 False  [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F) BRANCH crates/engine/src/keep.rs:862:13 False  [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F) BRANCH crates/engine/src/keep.rs:862:35 False  [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F) LINE   crates/engine/src/lib.rs:3282  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  first.frequent.capacity(),
(F) LINE   crates/engine/src/lib.rs:3283  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  repeated.len()
(F) BRANCH crates/engine/src/lib.rs:2857:13 False  [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,
(F) BRANCH crates/engine/src/lib.rs:2857:36 False  [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,
(F) BRANCH crates/engine/src/lib.rs:3222:17 False  [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3222:45 False  [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3244:17 False  [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F) BRANCH crates/engine/src/lib.rs:3244:46 False  [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F) BRANCH crates/engine/src/lib.rs:3299:17 False  [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3299:35 False  [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F) BRANCH crates/engine/src/lib.rs:3301:13 False  [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,
(F) BRANCH crates/engine/src/lib.rs:3301:29 False  [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,
(F) BRANCH crates/engine/src/lib.rs:3315:17 False  [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());
(F) BRANCH crates/engine/src/lib.rs:3315:45 False  [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());
(F) BRANCH crates/engine/src/lib.rs:3619:13 False  [assert! condition branch (failing side)]  text.contains("excluded 3 0 AlwaysFalse")
(F) BRANCH crates/engine/src/lib.rs:3620:20 False  [assert! condition branch (failing side)]  && text.contains("excluded 9 8 AlwaysTrue")
(F) BRANCH crates/engine/src/lib.rs:3621:20 False  [assert! condition branch (failing side)]  && text.contains("excluded 240 unmeasured NotLive"),
(F) BRANCH crates/engine/src/lib.rs:3989:13 False  [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),
(F) BRANCH crates/engine/src/lib.rs:3989:34 False  [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),
(F) BRANCH crates/engine/src/lib.rs:5620:21 False  [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F) BRANCH crates/engine/src/lib.rs:5620:34 False  [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F) LINE   crates/engine/src/refusal_cover_tests.rs:112  [panic!/unreachable! (failing arm)]  &mut |_, _, _| panic!("a refused walk reports no level"),
(F) LINE   crates/engine/src/refusal_cover_tests.rs:113  [panic!/unreachable! (failing arm)]  &mut |_, _| panic!("a refused walk retires no level"),
(F) LINE   crates/engine/src/refusal_cover_tests.rs:152  [panic!/unreachable! (failing arm)]  panic!("a walk of two levels took more than 256 reservations");
(F) LINE   crates/engine/tests/production_callers.rs:178  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  file.path.display()
(F) BRANCH crates/engine/tests/production_callers.rs:141:9 False  [assert! condition branch (failing side)]  engine > 1 && others > 1,
(F) BRANCH crates/engine/tests/production_callers.rs:141:23 False  [assert! condition branch (failing side)]  engine > 1 && others > 1,
(F) LINE   crates/engine/tests/resume_readiness.rs:78  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  bytes.get(56..192).expect("header")
(F) LINE   crates/engine/tests/resume_readiness.rs:270  [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F) LINE   crates/engine/tests/resume_readiness.rs:282  [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F) LINE   crates/engine/tests/resume_readiness.rs:294  [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F) BRANCH crates/engine/tests/resume_readiness.rs:98:67 False  [assert! condition branch (failing side)]  matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
(F) BRANCH crates/engine/tests/resume_readiness.rs:600:24 False  [closure inside the assert! condition: a row failing it makes .all() false, which is the assertion failing]  .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
(F) BRANCH crates/engine/tests/resume_readiness.rs:822:65 False  [assert! condition branch (failing side)]  assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
(F) BRANCH crates/engine/tests/resume_readiness.rs:989:13 False  [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F) BRANCH crates/engine/tests/resume_readiness.rs:989:37 False  [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F) BRANCH crates/engine/tests/sweep_readiness_oracle.rs:159:48 False  [closure inside the assert! condition: a pair failing it makes .all() false, which is the assertion failing]  .all(|w| matches!(w, [x, y] if x.mask.words() < y.mask.words())))
(F) LINE   crates/greeks/src/solver.rs:1079  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  BISECTION_STEPS - 1
(F) LINE   crates/greeks/src/solver.rs:1135  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  (high - low) / ulp(low)
(F) BRANCH crates/greeks/src/solver.rs:1133:21 False  [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),
(F) BRANCH crates/greeks/src/solver.rs:1133:36 False  [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),
(F) BRANCH crates/greeks/src/solver.rs:1140:48 False  [assert! condition branch (failing side)]  found.volatility == low || found.volatility == high,
(F) LINE   crates/greeks/tests/vendor_anchor.rs:524  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate_disagreement(root)
(F) LINE   crates/telemetry/src/level.rs:142  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.rank()
(F) BRANCH crates/telemetry/src/lib.rs:423:13 False  [assert! condition branch (failing side)]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/telemetry/src/lib.rs:423:40 False  [assert! condition branch (failing side)]  output.status.success() && stdout.contains("1 passed"),
(F) LINE   crates/telemetry/src/record.rs:841  [matches! no-match arm inside assert! (failing side): the line holds only the expansion's _ => false region]  matches!(
(F) BRANCH crates/telemetry/src/run_dirs.rs:234:21 False  [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F) BRANCH crates/telemetry/src/run_dirs.rs:234:44 False  [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F) BRANCH crates/telemetry/src/run_dirs.rs:240:21 False  [assert! condition branch (failing side)]  !other.sub.starts_with(&format!("{}/", dir.sub))
(F) BRANCH crates/telemetry/src/run_dirs.rs:241:28 False  [assert! condition branch (failing side)]  && !dir.sub.starts_with(&format!("{}/", other.sub)),
(F) LINE   crates/telemetry/src/sink.rs:4688  [panic!/unreachable! (failing arm)]  panic!(
(F) LINE   crates/telemetry/src/sink.rs:4690  [panic!/unreachable! (failing arm)]  path.display(),
(F) LINE   crates/telemetry/src/sink.rs:4691  [panic!/unreachable! (failing arm)]  String::from_utf8_lossy(line)
(F) LINE   crates/telemetry/src/sink.rs:5106  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  written[0].at_unix_millis
(F) LINE   crates/telemetry/src/sink.rs:5143  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[1].seq,
(F) LINE   crates/telemetry/src/sink.rs:5144  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[1].at_unix_millis,
(F) LINE   crates/telemetry/src/sink.rs:5145  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[0].seq,
(F) LINE   crates/telemetry/src/sink.rs:5146  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[0].at_unix_millis
(F) LINE   crates/telemetry/src/sink.rs:5476  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  span * 2
(F) LINE   crates/telemetry/src/sink.rs:5695  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  path.display()
(F) LINE   crates/telemetry/src/sink.rs:6079  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rolled.health()
(F) LINE   crates/telemetry/src/sink.rs:7663  [panic!/unreachable! (failing arm)]  other => panic!("{dimension}={value}: {other:?}"),
(F) LINE   crates/telemetry/src/sink.rs:8952  [panic!/unreachable! (failing arm)]  panic!("{pointer:?}");
(F) LINE   crates/telemetry/src/sink.rs:10275  [panic!/unreachable! (failing arm)]  other => panic!("{dimension}={value}: {other:?}"),
(F) BRANCH crates/telemetry/src/sink.rs:4824:21 False  [assert! condition branch (failing side)]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F) BRANCH crates/telemetry/src/sink.rs:4824:52 False  [assert! condition branch (failing side)]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F) BRANCH crates/telemetry/src/sink.rs:5024:17 False  [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F) BRANCH crates/telemetry/src/sink.rs:5024:65 False  [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F) BRANCH crates/telemetry/src/sink.rs:5712:17 False  [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F) BRANCH crates/telemetry/src/sink.rs:5712:40 False  [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F) BRANCH crates/telemetry/src/sink.rs:5757:13 False  [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F) BRANCH crates/telemetry/src/sink.rs:5757:30 False  [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F) BRANCH crates/telemetry/src/sink.rs:6621:13 False  [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6621:40 False  [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6687:17 False  [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F) BRANCH crates/telemetry/src/sink.rs:6687:54 False  [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F) BRANCH crates/telemetry/src/sink.rs:6707:13 False  [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),
(F) BRANCH crates/telemetry/src/sink.rs:6707:40 False  [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),
(F) BRANCH crates/telemetry/src/sink.rs:6749:13 False  [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:6749:53 False  [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F) BRANCH crates/telemetry/src/sink.rs:8209:21 False  [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());
(F) BRANCH crates/telemetry/src/sink.rs:8209:48 False  [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());
(F) BRANCH crates/telemetry/src/sink.rs:8951:21 False  [let-else in test whose else is a panic! (failing side)]  let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
(F) BRANCH crates/telemetry/src/sink.rs:11095:17 False  [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F) BRANCH crates/telemetry/src/sink.rs:11095:57 False  [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F) BRANCH crates/telemetry/src/sink.rs:11105:36 False  [assert! condition branch (failing side)]  .is_some_and(|why| why.contains("first line does not decode")
(F) BRANCH crates/telemetry/src/sink.rs:11160:17 False  [assert! condition branch (failing side)]  why.contains("last 64 KiB hold no line") && !why.contains("first line")
(F) BRANCH crates/telemetry/src/sink.rs:11226:17 False  [assert! condition branch (failing side)]  why.contains("first line does not decode") && !why.contains("last 64 KiB")
(F) BRANCH crates/telemetry/src/sink.rs:11231:17 False  [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");
(F) BRANCH crates/telemetry/src/sink.rs:11231:28 False  [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");
(F) BRANCH crates/telemetry/src/sink.rs:11262:17 False  [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F) BRANCH crates/telemetry/src/sink.rs:11262:44 False  [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F) BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:13 False  [assert! condition branch (failing side)]  assert!(!parked.failed && parked.open);
(F) BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:31 False  [assert! condition branch (failing side)]  assert!(!parked.failed && parked.open);
(F) LINE   crates/telemetry/src/state_events_tests.rs:85  [panic!/unreachable! (failing arm)]  || panic!("{path} is named by a row and not read by this test"),
(F) LINE   crates/telemetry/src/state_events_tests.rs:105  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.label(),
(F) LINE   crates/telemetry/src/state_events_tests.rs:113  [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.label()
(F) BRANCH crates/telemetry/src/state_events_tests.rs:176:17 False  [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F) BRANCH crates/telemetry/src/state_events_tests.rs:176:43 False  [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F) BRANCH crates/telemetry/src/state_events_tests.rs:321:13 False  [assert! condition branch (failing side)]  site.contains(&format!("\"{}\"", row.message))
(F) BRANCH crates/telemetry/src/state_events_tests.rs:322:20 False  [assert! condition branch (failing side)]  && site.contains(&format!("\"{}\"", row.target)),
(F) BRANCH crates/telemetry/src/tail.rs:1858:17 False  [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);
(F) BRANCH crates/telemetry/src/tail.rs:1858:31 False  [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);
(F) LINE   crates/telemetry/tests/cloud_mutants_sink.rs:263  [panic!/unreachable! (failing arm)]  other => panic!("a directory is not a rolled-off line: {other:?}"),
`````

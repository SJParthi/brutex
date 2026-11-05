# base-cov-libs-fu.txt.md

BASE-COV-LIBS-LSA (the COV-LIBS assembly, attempt 5): the (F)/(U) list of the ten crates, the list the landing gate reads (the Mac's `state/fix/round2/base-cov-libs-fu.txt`). Every (F) and (U) row of the session's measurement with its class and source, then the four shards' own (F)/(U) lists, which hold the proofs, verbatim. One fenced block below holds the list; sha256 of the fenced content: `65fed276cb5545d3526b5e773e1bfa2f7ba030f90e5fcc18e09485ad1a556819`.

`````text
(F) and (U) items of the ten crates at the assembly's tips (run 1 at 3069f0c9, runner at d096f65942d4, core at 0b7aa0d6d801; recipe v4 step 4).
RULES: (F) the failing side of an assert!/assert_eq!/assert_ne!/panic!/unreachable! arm inside test code (first clause); (F) bound/skip/collect/guard/fail: LS1's step-4 rows, a test's or check module's own bounding, skip, failure-collection, fault-mock-guard or failing arm, each with LS1's proof, under the reading LS1 records as the main session's answer of 5 Nov 2026 (second clause); (U) a region no input, file, folder, variable or direct call can reach on aarch64-apple-darwin, with its proof from the code.
Each row below keeps the class its shard's list gave it ([LSx], with the shard's line where the assembly moved the file) or the class decided here ([LSA: why]). The proofs of the shards' rows are their lists, carried verbatim in the APPENDIX below; their line numbers are as at each shard's tip, which equal this branch's lines except in crates/engine/src/lib.rs (LS4's patches and the judgment rows) and the runner files D-8569 seamed (exit_grid_policy.rs, grid.rs, grid_cover_tests.rs, resolved_grid_view.rs, signal_candle_stop.rs, signal_candle_stop_tests.rs), where the row's own [source] names the shard's line.

ROWS (2039)
(F)         LINE   crates/core/src/alias.rs:1276  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("{asked}: {other:?}"),]  other => panic!("{asked}: {other:?}"),
(F)         LINE   crates/core/src/alias.rs:1320  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("{asked}: {other:?}"),]  other => panic!("{asked}: {other:?}"),
(F)         BRANCH crates/core/src/alias.rs:1818:58 False  [LS4: [assert! condition branch (failing side)]  assert!(matches!(got.via, Via::Index(row) if row.store_key == key));]  assert!(matches!(got.via, Via::Index(row) if row.store_key == key));
(F)         BRANCH crates/core/src/alias.rs:1827:61 False  [LS4: [assert! condition branch (failing side)]  assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));]  assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));
(F)         BRANCH crates/core/src/alias.rs:1866:36 False  [LS4: [assert! condition branch (failing side)]  assert!(hit || !name.is_empty());]  assert!(hit || !name.is_empty());
(F)         BRANCH crates/core/src/instrument.rs:779:17 False  [LS4: [assert! condition branch (failing side)]  assert!(a < b && b < c);]  assert!(a < b && b < c);
(F)         BRANCH crates/core/src/instrument.rs:779:26 False  [LS4: [assert! condition branch (failing side)]  assert!(a < b && b < c);]  assert!(a < b && b < c);
(F)         BRANCH crates/core/src/instrument.rs:1698:17 False  [LS4: [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F)         BRANCH crates/core/src/instrument.rs:1698:55 False  [LS4: [assert! condition branch (failing side)]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F)         BRANCH crates/core/src/universe.rs:4755:17 False  [LS4: [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4755:38 False  [LS4: [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4755:60 False  [LS4: [assert! condition branch (failing side)]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4767:17 False  [LS4: [assert! condition branch (failing side)]  assert!(tight.contains("X") && tight.contains("Y"));]  assert!(tight.contains("X") && tight.contains("Y"));
(F)         BRANCH crates/core/src/universe.rs:4767:40 False  [LS4: [assert! condition branch (failing side)]  assert!(tight.contains("X") && tight.contains("Y"));]  assert!(tight.contains("X") && tight.contains("Y"));
(F)         BRANCH crates/core/src/universe.rs:4801:17 False  [LS4: [assert! condition branch (failing side)]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4801:42 False  [LS4: [assert! condition branch (failing side)]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4802:17 False  [LS4: [assert! condition branch (failing side)]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4802:48 False  [LS4: [assert! condition branch (failing side)]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4803:17 False  [LS4: [assert! condition branch (failing side)]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4803:48 False  [LS4: [assert! condition branch (failing side)]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4836:21 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let Some(held) = idx.slots[at] else {]  let Some(held) = idx.slots[at] else {
(F)         LINE   crates/core/src/universe.rs:4837  [LS4: [panic!/unreachable! (failing arm)]  panic!("{m} is a member, yet its probe reached an empty slot");]  panic!("{m} is a member, yet its probe reached an empty slot");
(F)         BRANCH crates/core/src/universe.rs:5628:17 False  [LS4: [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_50)]  u.contains(Universe::NIFTY_50)
(F)         BRANCH crates/core/src/universe.rs:5629:24 False  [LS4: [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_100)]  && u.contains(Universe::NIFTY_100)
(F)         BRANCH crates/core/src/universe.rs:5630:24 False  [LS4: [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_200)]  && u.contains(Universe::NIFTY_200)
(F)         BRANCH crates/core/src/universe.rs:5631:24 False  [LS4: [assert! condition branch (failing side)]  && u.contains(Universe::NIFTY_500)]  && u.contains(Universe::NIFTY_500)
(F)         BRANCH crates/core/src/universe.rs:5632:24 False  [LS4: [assert! condition branch (failing side)]  && u.contains(Universe::TOTAL_MARKET),]  && u.contains(Universe::TOTAL_MARKET),
(F)         LINE   crates/core/src/universe.rs:5634  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  u.bits()]  u.bits()
(F)         BRANCH crates/core/src/universe.rs:5640:17 False  [LS4: [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F)         BRANCH crates/core/src/universe.rs:5640:52 False  [LS4: [assert! condition branch (failing side)]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F)         BRANCH crates/core/src/universe.rs:5792:17 False  [LS4: [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F)         BRANCH crates/core/src/universe.rs:5792:46 False  [LS4: [assert! condition branch (failing side)]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F)         BRANCH crates/core/src/vendor.rs:3065:25 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let Some(held) = idx.slots[at] else {]  let Some(held) = idx.slots[at] else {
(F)         LINE   crates/core/src/vendor.rs:3066  [LS4: [panic!/unreachable! (failing arm)]  panic!("{m} is a member, yet its probe reached an empty slot");]  panic!("{m} is a member, yet its probe reached an empty slot");
(F)         BRANCH crates/core/tests/banned_lists.rs:113:9 False  [LS4: [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F)         BRANCH crates/core/tests/banned_lists.rs:113:33 False  [LS4: [assert! condition branch (failing side)]  forbidden.len() > 20 && forbidden.contains("pyo3"),]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F)         BRANCH crates/core/tests/claim_reader.rs:125:13 False  [LS4: [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F)         BRANCH crates/core/tests/claim_reader.rs:125:33 False  [LS4: [assert! condition branch (failing side)]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F)         LINE   crates/core/tests/cost_invariants.rs:88  [LS4: [panic!/unreachable! (failing arm)]  panic!(]  panic!(
(F)         LINE   crates/core/tests/cost_invariants.rs:90  [LS4: [panic!/unreachable! (failing arm)]  path.display()]  path.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:109  [LS4: [panic!/unreachable! (failing arm)]  panic!(]  panic!(
(F)         LINE   crates/core/tests/cost_invariants.rs:111  [LS4: [panic!/unreachable! (failing arm)]  crates_dir.display()]  crates_dir.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:136  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  crates_dir.display()]  crates_dir.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:364  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()]  rows.len()
(F)         LINE   crates/core/tests/cost_invariants.rs:371  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  printed.len(),]  printed.len(),
(F)         LINE   crates/core/tests/cost_invariants.rs:372  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  benches.len()]  benches.len()
(F)         LINE   crates/core/tests/decision_citations.rs:243  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  path.display()]  path.display()
(F)         LINE   crates/core/tests/decision_citations.rs:313  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  cited.len()]  cited.len()
(F)         LINE   crates/core/tests/decision_citations.rs:323  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  missing.len()]  missing.len()
(F)         LINE   crates/core/tests/decision_citations.rs:419  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  citations(&none[0].1)]  citations(&none[0].1)
(F)         LINE   crates/core/tests/decision_citations.rs:605  [LS4: [panic!/unreachable! (failing arm)]  panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")]  panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
(F)         LINE   crates/core/tests/decision_citations.rs:608  [LS4: [panic!/unreachable! (failing arm)]  || panic!("{file} is not under a crate's src/"),]  || panic!("{file} is not under a crate's src/"),
(F)         LINE   crates/core/tests/findings.rs:94  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  cells.len()]  cells.len()
(F)         LINE   crates/core/tests/findings.rs:296  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()]  rows.len()
(F)         LINE   crates/core/tests/findings.rs:336  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  counted + rest,]  counted + rest,
(F)         LINE   crates/core/tests/findings.rs:337  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()]  rows.len()
(F)         LINE   crates/core/tests/findings.rs:364  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rows.len()]  rows.len()
(F)         BRANCH crates/core/tests/findings.rs:369:13 False  [LS4: [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,]  cells.len() >= 3 && cells[2].len() > 20,
(F)         BRANCH crates/core/tests/findings.rs:369:33 False  [LS4: [assert! condition branch (failing side)]  cells.len() >= 3 && cells[2].len() > 20,]  cells.len() >= 3 && cells[2].len() > 20,
(F)         LINE   crates/core/tests/findings.rs:609  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  String::from_utf8_lossy(&output.stderr)]  String::from_utf8_lossy(&output.stderr)
(F)         LINE   crates/core/tests/findings.rs:993  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  String::from_utf8_lossy(&output.stderr)]  String::from_utf8_lossy(&output.stderr)
(F)         BRANCH crates/core/tests/graph.rs:528:33 False  [LS4: [assert! condition branch (failing side)]  !deps.is_empty() || claim.to_lowercase().contains("nothing"),]  !deps.is_empty() || claim.to_lowercase().contains("nothing"),
(F)         BRANCH crates/core/tests/graph.rs:563:44 False  [LS4: [assert! condition branch (failing side)]  !name.contains('*') && !name.contains('/') && !name.is_empty(),]  !name.contains('*') && !name.contains('/') && !name.is_empty(),
(F)         LINE   crates/core/tests/lint.rs:139  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  &next_fn[..next_fn.find('(').unwrap_or(0)],]  &next_fn[..next_fn.find('(').unwrap_or(0)],
(F)         BRANCH crates/core/tests/zerodha_index.rs:59:13 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let Decoded::Keep(listing) = decoded else {]  let Decoded::Keep(listing) = decoded else {
(F)         LINE   crates/core/tests/zerodha_index.rs:60  [LS4: [panic!/unreachable! (failing arm)]  panic!("{spelled} must be KEPT, not skipped: {decoded:?}");]  panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
(F)         BRANCH crates/costs/src/day.rs:897:25 False  [LS4: [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F)         BRANCH crates/costs/src/day.rs:897:57 False  [LS4: [assert! condition branch (failing side)]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F)         LINE   crates/costs/src/fill.rs:1010  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  best.buy().raw(),]  best.buy().raw(),
(F)         LINE   crates/costs/src/fill.rs:1011  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  worst.buy().raw()]  worst.buy().raw()
(F)         LINE   crates/costs/src/fill.rs:1017  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  best.sell().raw(),]  best.sell().raw(),
(F)         LINE   crates/costs/src/fill.rs:1018  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  worst.sell().raw()]  worst.sell().raw()
(F)         LINE   crates/costs/src/ledger.rs:1345  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  LINE_NAMES[at],]  LINE_NAMES[at],
(F)         LINE   crates/costs/src/ledger.rs:1346  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  summed[at],]  summed[at],
(F)         LINE   crates/costs/src/ledger.rs:1347  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  closed[at]]  closed[at]
(F)         BRANCH crates/costs/src/lot.rs:1497:21 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let LotBasis::ContractRule(citation) = got.basis else {]  let LotBasis::ContractRule(citation) = got.basis else {
(F)         LINE   crates/costs/src/lot.rs:1498  [LS4: [panic!/unreachable! (failing arm)]  panic!("{subject} {on:?}: {:?}", got.basis);]  panic!("{subject} {on:?}: {:?}", got.basis);
(F)         BRANCH crates/costs/src/lot.rs:1554:25 False  [LS4: [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F)         BRANCH crates/costs/src/lot.rs:1554:57 False  [LS4: [assert! condition branch (failing side)]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F)         BRANCH crates/costs/src/lot.rs:1589:17 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {]  let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
(F)         LINE   crates/costs/src/lot.rs:1590  [LS4: [panic!/unreachable! (failing arm)]  panic!("{subject} {on:?}: an unread revision answered a lot");]  panic!("{subject} {on:?}: an unread revision answered a lot");
(F)         LINE   crates/costs/src/lot.rs:1614  [LS4: [matches! no-match arm inside assert! (failing side): the line holds only the expansion's _ => false region]  matches!(]  matches!(
(F)         BRANCH crates/costs/src/lot_table_tests.rs:112:52 False  [LS4: [assert! condition branch (failing side)]  assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);]  assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
(F)         BRANCH crates/costs/src/lot_table_tests.rs:117:25 False  [LS4: [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F)         BRANCH crates/costs/src/lot_table_tests.rs:117:56 False  [LS4: [assert! condition branch (failing side)]  assert!(row.source_sha256.is_none() && row.read.is_none());]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F)         LINE   crates/costs/src/lot_table_tests.rs:146  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("expected the unverified row, got {other:?}"),]  other => panic!("expected the unverified row, got {other:?}"),
(F)         LINE   crates/costs/src/lot_table_tests.rs:156  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("expected before-first-row, got {other:?}"),]  other => panic!("expected before-first-row, got {other:?}"),
(F)         BRANCH crates/costs/src/lot_table_tests.rs:187:56 False  [LS4: [assert! condition branch (failing side)]  Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP]  Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
(F)         BRANCH crates/costs/src/money.rs:383:17 False  [LS4: [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);]  assert!(widest > 0 && widest < i128::MAX);
(F)         BRANCH crates/costs/src/money.rs:383:31 False  [LS4: [assert! condition branch (failing side)]  assert!(widest > 0 && widest < i128::MAX);]  assert!(widest > 0 && widest < i128::MAX);
(F)         LINE   crates/costs/src/schedule.rs:1213  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()]  rate.source()
(F)         BRANCH crates/costs/src/schedule.rs:1244:75 False  [LS4: [assert! condition branch (failing side)]  matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),]  matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
(F)         LINE   crates/costs/src/schedule.rs:1280  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()]  rate.source()
(F)         LINE   crates/costs/src/schedule.rs:1288  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate.source()]  rate.source()
(F)         BRANCH crates/costs/src/schedule.rs:1633:29 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let Taxes::Options { stt_sell, .. } = card.taxes() else {]  let Taxes::Options { stt_sell, .. } = card.taxes() else {
(F)         LINE   crates/costs/src/schedule.rs:1634  [LS4: [panic!/unreachable! (failing arm)]  panic!("an options card carries option taxes");]  panic!("an options card carries option taxes");
(F)         LINE   crates/costs/src/schedule.rs:1643  [LS4: [panic!/unreachable! (failing arm)]  (card, trip) => panic!(]  (card, trip) => panic!(
(F)         LINE   crates/costs/src/scope.rs:249  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  lines[arm]]  lines[arm]
(U)         LINE   crates/costs/src/stack.rs:720  [LS4: [Exact::sum of the calculator day total cannot overflow i128: page_lines (dhan_calc.rs) multiplies the brokerage by GST_PERCENT=18 unguarded, so any brokerage p]  )?;
(U)         LINE   crates/costs/src/stack.rs:907  [LS4: [to_rupee narrows half_up::<100>(amount) x 100 to i64: for every i64 amount, |amount / 100| <= 92233720368547758.08, its nearest whole rupee has magnitude <= 92]  )?)
(F)         BRANCH crates/costs/src/stack.rs:2681:17 False  [LS4: [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F)         BRANCH crates/costs/src/stack.rs:2681:43 False  [LS4: [assert! condition branch (failing side)]  assert!(thin.gross().raw() > 0 && !thin.is_win());]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F)         LINE   crates/costs/tests/adversarial.rs:114  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  c.raw(),]  c.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:115  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  f.raw()]  f.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:121  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  c.raw(),]  c.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:122  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  f.raw()]  f.raw()
(F)         BRANCH crates/costs/tests/adversarial.rs:128:21 False  [LS4: [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F)         BRANCH crates/costs/tests/adversarial.rs:128:44 False  [LS4: [assert! condition branch (failing side)]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F)         LINE   crates/costs/tests/adversarial.rs:134  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:143  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:151  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()]  levy.raw()
(F)         BRANCH crates/costs/tests/adversarial.rs:194:21 False  [LS4: [assert! condition branch (failing side)]  ceil.is_ok()]  ceil.is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:195:28 False  [LS4: [assert! condition branch (failing side)]  && floor.is_ok()]  && floor.is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:196:28 False  [LS4: [assert! condition branch (failing side)]  && levy_ceiling(notional, rate).is_ok()]  && levy_ceiling(notional, rate).is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:197:28 False  [LS4: [assert! condition branch (failing side)]  && statutory_levy(notional, rate).is_ok(),]  && statutory_levy(notional, rate).is_ok(),
(F)         LINE   crates/costs/tests/adversarial.rs:263  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  n.raw(),]  n.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:264  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  levy.raw()]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:301  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  out.map(Paisa::raw)]  out.map(Paisa::raw)
(F)         LINE   crates/costs/tests/dhan_calculator.rs:666  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  case.page[7]]  case.page[7]
(F)         LINE   crates/costs/tests/dhan_calculator.rs:931  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  case.name, dated_lines[at].0]  case.name, dated_lines[at].0
(F)         BRANCH crates/engine/src/keep.rs:855:17 False  [LS4: [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F)         BRANCH crates/engine/src/keep.rs:855:39 False  [LS4: [assert! condition branch (failing side)]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F)         BRANCH crates/engine/src/keep.rs:862:13 False  [LS4: [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F)         BRANCH crates/engine/src/keep.rs:862:35 False  [LS4: [assert! condition branch (failing side)]  !lost.reconciles() && !Tally::of(&lost).reconciles(),]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F)         BRANCH crates/engine/src/lib.rs:2858:13 False  [LS4 line 2857: [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,]  !out_par.is_empty() && inf_par > 0,
(F)         BRANCH crates/engine/src/lib.rs:2858:36 False  [LS4 line 2857: [assert! condition branch (failing side)]  !out_par.is_empty() && inf_par > 0,]  !out_par.is_empty() && inf_par > 0,
(F)         BRANCH crates/engine/src/lib.rs:3035:48 False  [LSA: the failing side of the distinctness assertion of tests::the_join_emits_exactly_one_candidate_per_pair (a repeated survivor); its True side is covered since LSA widened the fixture to five survivors (LS4's judgment row 1)]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
(F)         BRANCH crates/engine/src/lib.rs:3243:17 False  [LS4 line 3222: [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3243:45 False  [LS4 line 3222: [assert! condition branch (failing side)]  assert!(result.levels.is_empty() && result.excluded.is_empty());]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3265:17 False  [LS4 line 3244: [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F)         BRANCH crates/engine/src/lib.rs:3265:46 False  [LS4 line 3244: [assert! condition branch (failing side)]  assert!(retained.halted.is_none() && retained.levels.len() > 1);]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F)         LINE   crates/engine/src/lib.rs:3303  [LS4 line 3282: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  first.frequent.capacity(),]  first.frequent.capacity(),
(F)         LINE   crates/engine/src/lib.rs:3304  [LS4 line 3283: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  repeated.len()]  repeated.len()
(F)         BRANCH crates/engine/src/lib.rs:3320:17 False  [LS4 line 3299: [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3320:35 False  [LS4 line 3299: [assert! condition branch (failing side)]  assert!(halt.is_none() && !previous.frequent.is_empty());]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3322:13 False  [LS4 line 3301: [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,]  admitted > 0 && pairs > 0,
(F)         BRANCH crates/engine/src/lib.rs:3322:29 False  [LS4 line 3301: [assert! condition branch (failing side)]  admitted > 0 && pairs > 0,]  admitted > 0 && pairs > 0,
(F)         BRANCH crates/engine/src/lib.rs:3336:17 False  [LS4 line 3315: [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());]  assert!(next.frequent.is_empty() && next.reconciles());
(F)         BRANCH crates/engine/src/lib.rs:3336:45 False  [LS4 line 3315: [assert! condition branch (failing side)]  assert!(next.frequent.is_empty() && next.reconciles());]  assert!(next.frequent.is_empty() && next.reconciles());
(F)         BRANCH crates/engine/src/lib.rs:3640:13 False  [LS4 line 3619: [assert! condition branch (failing side)]  text.contains("excluded 3 0 AlwaysFalse")]  text.contains("excluded 3 0 AlwaysFalse")
(F)         BRANCH crates/engine/src/lib.rs:3641:20 False  [LS4 line 3620: [assert! condition branch (failing side)]  && text.contains("excluded 9 8 AlwaysTrue")]  && text.contains("excluded 9 8 AlwaysTrue")
(F)         BRANCH crates/engine/src/lib.rs:3642:20 False  [LS4 line 3621: [assert! condition branch (failing side)]  && text.contains("excluded 240 unmeasured NotLive"),]  && text.contains("excluded 240 unmeasured NotLive"),
(F)         BRANCH crates/engine/src/lib.rs:4010:13 False  [LS4 line 3989: [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),]  roomy.completed() && tight.completed(),
(F)         BRANCH crates/engine/src/lib.rs:4010:34 False  [LS4 line 3989: [assert! condition branch (failing side)]  roomy.completed() && tight.completed(),]  roomy.completed() && tight.completed(),
(F)         BRANCH crates/engine/src/lib.rs:5667:21 False  [LS4 line 5620: [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F)         BRANCH crates/engine/src/lib.rs:5667:34 False  [LS4 line 5620: [assert! condition branch (failing side)]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:112  [LS4: [panic!/unreachable! (failing arm)]  &mut |_, _, _| panic!("a refused walk reports no level"),]  &mut |_, _, _| panic!("a refused walk reports no level"),
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:113  [LS4: [panic!/unreachable! (failing arm)]  &mut |_, _| panic!("a refused walk retires no level"),]  &mut |_, _| panic!("a refused walk retires no level"),
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:152  [LS4: [panic!/unreachable! (failing arm)]  panic!("a walk of two levels took more than 256 reservations");]  panic!("a walk of two levels took more than 256 reservations");
(F)         BRANCH crates/engine/tests/production_callers.rs:141:9 False  [LS4: [assert! condition branch (failing side)]  engine > 1 && others > 1,]  engine > 1 && others > 1,
(F)         BRANCH crates/engine/tests/production_callers.rs:141:23 False  [LS4: [assert! condition branch (failing side)]  engine > 1 && others > 1,]  engine > 1 && others > 1,
(F)         LINE   crates/engine/tests/production_callers.rs:178  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  file.path.display()]  file.path.display()
(F)         LINE   crates/engine/tests/resume_readiness.rs:78  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  bytes.get(56..192).expect("header")]  bytes.get(56..192).expect("header")
(F)         BRANCH crates/engine/tests/resume_readiness.rs:98:67 False  [LS4: [assert! condition branch (failing side)]  matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")]  matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
(F)         LINE   crates/engine/tests/resume_readiness.rs:270  [LS4: [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         LINE   crates/engine/tests/resume_readiness.rs:282  [LS4: [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         LINE   crates/engine/tests/resume_readiness.rs:294  [LS4: [panic!/unreachable! (failing arm)]  &mut |_| panic!("a refused resume reaches no checkpoint")]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         BRANCH crates/engine/tests/resume_readiness.rs:600:24 False  [LS4: [closure inside the assert! condition: a row failing it makes .all() false, which is the assertion failing]  .all(|row| row.reason == engine::Why::NotLive && ro]  .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
(F)         BRANCH crates/engine/tests/resume_readiness.rs:822:65 False  [LS4: [assert! condition branch (failing side)]  assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));]  assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
(F)         BRANCH crates/engine/tests/resume_readiness.rs:989:13 False  [LS4: [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F)         BRANCH crates/engine/tests/resume_readiness.rs:989:37 False  [LS4: [assert! condition branch (failing side)]  assert!(expected.completed() && need > 1, "fixture must walk pairs");]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F)         BRANCH crates/engine/tests/sweep_readiness_oracle.rs:159:48 False  [LS4: [closure inside the assert! condition: a pair failing it makes .all() false, which is the assertion failing]  .all(|w| matches!(w, [x, y] if x.mask.words() < y.]  .all(|w| matches!(w, [x, y] if x.mask.words() < y.mask.words())))
(F)         LINE   crates/greeks/src/solver.rs:1079  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  BISECTION_STEPS - 1]  BISECTION_STEPS - 1
(F)         BRANCH crates/greeks/src/solver.rs:1133:21 False  [LS4: [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),]  low <= high && high <= low + ulp(low),
(F)         BRANCH crates/greeks/src/solver.rs:1133:36 False  [LS4: [assert! condition branch (failing side)]  low <= high && high <= low + ulp(low),]  low <= high && high <= low + ulp(low),
(F)         LINE   crates/greeks/src/solver.rs:1135  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  (high - low) / ulp(low)]  (high - low) / ulp(low)
(F)         BRANCH crates/greeks/src/solver.rs:1140:48 False  [LS4: [assert! condition branch (failing side)]  found.volatility == low || found.volatility == high,]  found.volatility == low || found.volatility == high,
(F)         LINE   crates/greeks/tests/vendor_anchor.rs:524  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rate_disagreement(root)]  rate_disagreement(root)
(F)         BRANCH crates/indicators/src/anchored.rs:2247:28 False  [LS3: if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })]  if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
(F)         BRANCH crates/indicators/src/anchored.rs:2813:17 False  [LS3: accepted > 500 && refused > 500,]  accepted > 500 && refused > 500,
(F)         BRANCH crates/indicators/src/anchored.rs:2813:35 False  [LS3: accepted > 500 && refused > 500,]  accepted > 500 && refused > 500,
(F)         LINE   crates/indicators/src/column.rs:1672  [LS3: family.len()]  family.len()
(F)         BRANCH crates/indicators/src/column.rs:1789:25 False  [LS3: assert!(!truth.get(bit) && !known.get(bit));]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1789:44 False  [LS3: assert!(!truth.get(bit) && !known.get(bit));]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1792:25 False  [LS3: assert!(!truth.get(bit) && !known.get(bit));]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1792:44 False  [LS3: assert!(!truth.get(bit) && !known.get(bit));]  assert!(!truth.get(bit) && !known.get(bit));
(F)         LINE   crates/indicators/src/column.rs:2149  [LS3: poisoned_column.bits().len()]  poisoned_column.bits().len()
(F)         BRANCH crates/indicators/src/column.rs:2326:13 False  [LS3: !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),]  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F)         BRANCH crates/indicators/src/column.rs:2326:62 False  [LS3: !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),]  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F)         BRANCH crates/indicators/src/column.rs:2609:13 False  [LS3: projected.sources().contains(&22) && projected.sources().contains(&24),]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F)         BRANCH crates/indicators/src/column.rs:2609:50 False  [LS3: projected.sources().contains(&22) && projected.sources().contains(&24),]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F)         BRANCH crates/indicators/src/daily.rs:869:17 False  [LS3: assert!(!m.get(60) && !m.get(61), "it is neither above nor below");]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F)         BRANCH crates/indicators/src/daily.rs:869:31 False  [LS3: assert!(!m.get(60) && !m.get(61), "it is neither above nor below");]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F)         BRANCH crates/indicators/src/daily.rs:1114:17 False  [LS3: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1114:30 False  [LS3: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1114:43 False  [LS3: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1115:17 False  [LS3: assert!(m.get(184) && m.get(186), "R5/S5 bands not above");]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1115:31 False  [LS3: assert!(m.get(184) && m.get(186), "R5/S5 bands not above");]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1116:17 False  [LS3: assert!(m.get(13) && m.get(15), "pdh/pdl not above");]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F)         BRANCH crates/indicators/src/daily.rs:1116:30 False  [LS3: assert!(m.get(13) && m.get(15), "pdh/pdl not above");]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F)         BRANCH crates/indicators/src/daily.rs:1137:17 False  [LS3: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1137:30 False  [LS3: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1137:43 False  [LS3: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:17 False  [LS3: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:30 False  [LS3: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:43 False  [LS3: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1139:17 False  [LS3: assert!(m.get(181) && m.get(183), "R4/S4 bands not below");]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1139:31 False  [LS3: assert!(m.get(181) && m.get(183), "R4/S4 bands not below");]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1140:17 False  [LS3: assert!(m.get(185) && m.get(187), "R5/S5 bands not below");]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1140:31 False  [LS3: assert!(m.get(185) && m.get(187), "R5/S5 bands not below");]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1141:17 False  [LS3: assert!(m.get(14) && m.get(16), "pdh/pdl not below");]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F)         BRANCH crates/indicators/src/daily.rs:1141:30 False  [LS3: assert!(m.get(14) && m.get(16), "pdh/pdl not below");]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F)         BRANCH crates/indicators/src/daily.rs:1554:13 False  [LS3: cuts.contains("narrow: 80") && cuts.contains("wide: 250"),]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F)         BRANCH crates/indicators/src/daily.rs:1554:44 False  [LS3: cuts.contains("narrow: 80") && cuts.contains("wide: 250"),]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F)         BRANCH crates/indicators/src/daily.rs:1573:13 False  [LS3: rendered.contains("pivot: 96") && rendered.contains("half: 1"),]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F)         BRANCH crates/indicators/src/daily.rs:1573:47 False  [LS3: rendered.contains("pivot: 96") && rendered.contains("half: 1"),]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F)         BRANCH crates/indicators/src/evaluator.rs:1586:78 False  [LS3: Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected]  Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected
(F)         BRANCH crates/indicators/src/evaluator.rs:1891:17 False  [LS3: assert!(truth_only.warmed_up() && full.warmed_up());]  assert!(truth_only.warmed_up() && full.warmed_up());
(F)         BRANCH crates/indicators/src/evaluator.rs:1891:43 False  [LS3: assert!(truth_only.warmed_up() && full.warmed_up());]  assert!(truth_only.warmed_up() && full.warmed_up());
(F)         BRANCH crates/indicators/src/evaluator.rs:1982:17 False  [LS3: !first.get(above) && !first.get(below),]  !first.get(above) && !first.get(below),
(F)         BRANCH crates/indicators/src/evaluator.rs:1982:38 False  [LS3: !first.get(above) && !first.get(below),]  !first.get(above) && !first.get(below),
(F)         BRANCH crates/indicators/src/evaluator.rs:3564:13 False  [LS3: !first.get(276) && !first.get(277),]  !first.get(276) && !first.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3564:32 False  [LS3: !first.get(276) && !first.get(277),]  !first.get(276) && !first.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3588:13 False  [LS3: !flat.get(276) && !flat.get(277),]  !flat.get(276) && !flat.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3588:31 False  [LS3: !flat.get(276) && !flat.get(277),]  !flat.get(276) && !flat.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3651:36 True  [LS3: !(mask.get(278) && mask.get(279)),]  !(mask.get(278) && mask.get(279)),
(F)         BRANCH crates/indicators/src/fib.rs:584:21 False  [LS3: assert!(p != 0 && p != 1000, "rung {p} should have no position here");]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F)         BRANCH crates/indicators/src/fib.rs:584:31 False  [LS3: assert!(p != 0 && p != 1000, "rung {p} should have no position here");]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F)         BRANCH crates/indicators/src/gap.rs:1491:13 False  [LS3: fired > 0 && legs > 0,]  fired > 0 && legs > 0,
(F)         BRANCH crates/indicators/src/gap.rs:1491:26 False  [LS3: fired > 0 && legs > 0,]  fired > 0 && legs > 0,
(F)         BRANCH crates/indicators/src/orb.rs:555:13 False  [LS3: o.extremes(0).is_some() && o.extremes(1).is_some(),]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F)         BRANCH crates/indicators/src/orb.rs:555:40 False  [LS3: o.extremes(0).is_some() && o.extremes(1).is_some(),]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F)         BRANCH crates/indicators/src/orb.rs:559:13 False  [LS3: o.extremes(2).is_none() && o.extremes(3).is_none(),]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F)         BRANCH crates/indicators/src/orb.rs:559:40 False  [LS3: o.extremes(2).is_none() && o.extremes(3).is_none(),]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F)         BRANCH crates/indicators/src/orb.rs:610:17 False  [LS3: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/orb.rs:610:40 False  [LS3: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/orb.rs:610:63 False  [LS3: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/pattern.rs:1558:17 False  [LS3: assert!(shape.bullish() && !shape.bearish());]  assert!(shape.bullish() && !shape.bearish());
(F)         BRANCH crates/indicators/src/pattern.rs:1558:36 False  [LS3: assert!(shape.bullish() && !shape.bearish());]  assert!(shape.bullish() && !shape.bearish());
(F)         BRANCH crates/indicators/src/pattern.rs:1562:17 False  [LS3: assert!(shape.body_at_most(250) && !shape.body_at_most(249));]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F)         BRANCH crates/indicators/src/pattern.rs:1562:44 False  [LS3: assert!(shape.body_at_most(250) && !shape.body_at_most(249));]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F)         BRANCH crates/indicators/src/pattern.rs:1597:13 False  [LS3: m2.get(153) && m2.get(155),]  m2.get(153) && m2.get(155),
(F)         BRANCH crates/indicators/src/pattern.rs:1597:28 False  [LS3: m2.get(153) && m2.get(155),]  m2.get(153) && m2.get(155),
(F)         BRANCH crates/indicators/src/pattern.rs:1711:41 True  [LS3: !(mask.get(bull) && mask.get(bear)),]  !(mask.get(bull) && mask.get(bear)),
(F)         BRANCH crates/indicators/src/session.rs:586:17 False  [LS3: assert!(truth.get(66) && !truth.get(67));]  assert!(truth.get(66) && !truth.get(67));
(F)         BRANCH crates/indicators/src/session.rs:586:34 False  [LS3: assert!(truth.get(66) && !truth.get(67));]  assert!(truth.get(66) && !truth.get(67));
(F)         BRANCH crates/indicators/src/session.rs:587:17 False  [LS3: assert!(known.get(66) && known.get(67));]  assert!(known.get(66) && known.get(67));
(F)         BRANCH crates/indicators/src/session.rs:587:34 False  [LS3: assert!(known.get(66) && known.get(67));]  assert!(known.get(66) && known.get(67));
(F)         BRANCH crates/indicators/src/session.rs:589:13 False  [LS3: !truth.get(68) && !known.get(68),]  !truth.get(68) && !known.get(68),
(F)         BRANCH crates/indicators/src/session.rs:589:31 False  [LS3: !truth.get(68) && !known.get(68),]  !truth.get(68) && !known.get(68),
(F)         BRANCH crates/indicators/src/session.rs:655:13 False  [LS3: !bits.get(42) && !bits.get(43),]  !bits.get(42) && !bits.get(43),
(F)         BRANCH crates/indicators/src/session.rs:655:30 False  [LS3: !bits.get(42) && !bits.get(43),]  !bits.get(42) && !bits.get(43),
(F)         BRANCH crates/indicators/src/session.rs:680:13 False  [LS3: !edge.get(44) && edge.get(45),]  !edge.get(44) && edge.get(45),
(F)         BRANCH crates/indicators/src/session.rs:680:30 False  [LS3: !edge.get(44) && edge.get(45),]  !edge.get(44) && edge.get(45),
(F)         BRANCH crates/indicators/src/session.rs:788:17 False  [LS3: assert!(up.get(30) && !up.get(31));]  assert!(up.get(30) && !up.get(31));
(F)         BRANCH crates/indicators/src/session.rs:788:31 False  [LS3: assert!(up.get(30) && !up.get(31));]  assert!(up.get(30) && !up.get(31));
(F)         BRANCH crates/indicators/src/session.rs:790:17 False  [LS3: assert!(down.get(31) && !down.get(30));]  assert!(down.get(31) && !down.get(30));
(F)         BRANCH crates/indicators/src/session.rs:790:33 False  [LS3: assert!(down.get(31) && !down.get(30));]  assert!(down.get(31) && !down.get(30));
(F)         BRANCH crates/indicators/src/session.rs:793:13 False  [LS3: !flat.get(30) && !flat.get(31),]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:793:30 False  [LS3: !flat.get(30) && !flat.get(31),]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:838:17 False  [LS3: assert!(!mask.get(37) && !mask.get(38));]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:838:34 False  [LS3: assert!(!mask.get(37) && !mask.get(38));]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:858:17 False  [LS3: !mask.get(30) && !mask.get(31),]  !mask.get(30) && !mask.get(31),
(F)         BRANCH crates/indicators/src/session.rs:858:34 False  [LS3: !mask.get(30) && !mask.get(31),]  !mask.get(30) && !mask.get(31),
(F)         BRANCH crates/indicators/src/session.rs:922:13 False  [LS3: !flat.get(30) && !flat.get(31),]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:922:30 False  [LS3: !flat.get(30) && !flat.get(31),]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:931:17 False  [LS3: assert!(!mask.get(37) && !mask.get(38));]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:931:34 False  [LS3: assert!(!mask.get(37) && !mask.get(38));]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:950:13 False  [LS3: !mask.get(37) && !mask.get(38) && !mask.get(39),]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:950:30 False  [LS3: !mask.get(37) && !mask.get(38) && !mask.get(39),]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:950:47 False  [LS3: !mask.get(37) && !mask.get(38) && !mask.get(39),]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:966:17 False  [LS3: assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F)         BRANCH crates/indicators/src/session.rs:966:33 False  [LS3: assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F)         BRANCH crates/indicators/src/session.rs:968:17 False  [LS3: assert!(mask.get(66) && !mask.get(67));]  assert!(mask.get(66) && !mask.get(67));
(F)         BRANCH crates/indicators/src/session.rs:968:33 False  [LS3: assert!(mask.get(66) && !mask.get(67));]  assert!(mask.get(66) && !mask.get(67));
(F)         BRANCH crates/indicators/src/session.rs:972:17 False  [LS3: assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F)         BRANCH crates/indicators/src/session.rs:972:34 False  [LS3: assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F)         BRANCH crates/indicators/src/session.rs:974:13 False  [LS3: !mask.get(66) && !mask.get(67) && !mask.get(68),]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:974:30 False  [LS3: !mask.get(66) && !mask.get(67) && !mask.get(68),]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:974:47 False  [LS3: !mask.get(66) && !mask.get(67) && !mask.get(68),]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:989:17 False  [LS3: assert!(mask.get(50) && !mask.get(51));]  assert!(mask.get(50) && !mask.get(51));
(F)         BRANCH crates/indicators/src/session.rs:989:33 False  [LS3: assert!(mask.get(50) && !mask.get(51));]  assert!(mask.get(50) && !mask.get(51));
(F)         BRANCH crates/indicators/src/session.rs:993:17 False  [LS3: assert!(mask.get(51) && !mask.get(50));]  assert!(mask.get(51) && !mask.get(50));
(F)         BRANCH crates/indicators/src/session.rs:993:33 False  [LS3: assert!(mask.get(51) && !mask.get(50));]  assert!(mask.get(51) && !mask.get(50));
(F)         BRANCH crates/indicators/src/session.rs:1283:13 False  [LS3: !mask.get(66) && !mask.get(67),]  !mask.get(66) && !mask.get(67),
(F)         BRANCH crates/indicators/src/session.rs:1283:30 False  [LS3: !mask.get(66) && !mask.get(67),]  !mask.get(66) && !mask.get(67),
(F)         BRANCH crates/indicators/src/trend.rs:1292:13 False  [LS3: !mask.get(0) && !mask.get(1),]  !mask.get(0) && !mask.get(1),
(F)         BRANCH crates/indicators/src/trend.rs:1292:29 False  [LS3: !mask.get(0) && !mask.get(1),]  !mask.get(0) && !mask.get(1),
(F)         BRANCH crates/indicators/src/trend.rs:1296:13 False  [LS3: !mask.get(2) && !mask.get(3),]  !mask.get(2) && !mask.get(3),
(F)         BRANCH crates/indicators/src/trend.rs:1296:29 False  [LS3: !mask.get(2) && !mask.get(3),]  !mask.get(2) && !mask.get(3),
(F)         BRANCH crates/indicators/src/trend.rs:1300:13 False  [LS3: !mask.get(4) && !mask.get(5),]  !mask.get(4) && !mask.get(5),
(F)         BRANCH crates/indicators/src/trend.rs:1300:29 False  [LS3: !mask.get(4) && !mask.get(5),]  !mask.get(4) && !mask.get(5),
(F)         BRANCH crates/indicators/src/trend.rs:2302:74 False  [LS3: matches!((previous, now), (Some(before), Some(after)) if after < before),]  matches!((previous, now), (Some(before), Some(after)) if after < before),
(U)         BRANCH crates/indicators/src/vwap.rs:212:11 False  [LS3: while i < NEWTON_STEPS {]  while i < NEWTON_STEPS {
(F)         BRANCH crates/indicators/src/vwap.rs:791:17 False  [LS3: assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:791:34 False  [LS3: assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:792:17 False  [LS3: assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:792:34 False  [LS3: assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:904:17 False  [LS3: assert!(v.known(tol()).get(146) && v.known(tol()).get(148));]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F)         BRANCH crates/indicators/src/vwap.rs:904:44 False  [LS3: assert!(v.known(tol()).get(146) && v.known(tol()).get(148));]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F)         BRANCH crates/indicators/src/vwap.rs:1250:21 False  [LS3: !mask.get(u32::from(above)) && !mask.get(u32::from(below)),]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F)         BRANCH crates/indicators/src/vwap.rs:1250:52 False  [LS3: !mask.get(u32::from(above)) && !mask.get(u32::from(below)),]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F)         BRANCH crates/indicators/tests/crossing_known_readiness.rs:84:32 True  [LS3: !(above && below),]  !(above && below),
(F)         BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:13 False  [LS3: false_count > 0 && true_count > 0,]  false_count > 0 && true_count > 0,
(F)         BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:32 False  [LS3: false_count > 0 && true_count > 0,]  false_count > 0 && true_count > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:13 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:26 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:40 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:13 False  [LS3: !truth.get(bit) && !known.get(bit),]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:32 False  [LS3: !truth.get(bit) && !known.get(bit),]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:17 False  [LS3: !truth.get(bit) && !known.get(bit),]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:36 False  [LS3: !truth.get(bit) && !known.get(bit),]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:176:13 False  [LS3: !truth.get(retired) && !known.get(retired),]  !truth.get(retired) && !known.get(retired),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:176:36 False  [LS3: !truth.get(retired) && !known.get(retired),]  !truth.get(retired) && !known.get(retired),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:231:13 False  [LS3: trues > 0 && falses > 0,]  trues > 0 && falses > 0,
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:231:26 False  [LS3: trues > 0 && falses > 0,]  trues > 0 && falses > 0,
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:224:21 False  [LS3: assert!(false_count > 0 && true_count > 0);]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:224:40 False  [LS3: assert!(false_count > 0 && true_count > 0);]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:264:17 False  [LS3: assert!(false_count > 0 && true_count > 0);]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:264:36 False  [LS3: assert!(false_count > 0 && true_count > 0);]  assert!(false_count > 0 && true_count > 0);
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:271  [LS3: positions.len()]  positions.len()
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:321  [LS3: positions.len()]  positions.len()
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:332  [LS3: emitted.len()]  emitted.len()
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:13 False  [LS3: false_count > 0 && true_count > 0 && unknown_count > 0,]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:32 False  [LS3: false_count > 0 && true_count > 0 && unknown_count > 0,]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:50 False  [LS3: false_count > 0 && true_count > 0 && unknown_count > 0,]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/session_from_outside.rs:200:9 False  [LS3: mask.get(46) && !mask.get(47),]  mask.get(46) && !mask.get(47),
(F)         BRANCH crates/indicators/tests/session_from_outside.rs:200:25 False  [LS3: mask.get(46) && !mask.get(47),]  mask.get(46) && !mask.get(47),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:367:13 False  [LS3: !truth.get(position) && !known.get(position),]  !truth.get(position) && !known.get(position),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:367:37 False  [LS3: !truth.get(position) && !known.get(position),]  !truth.get(position) && !known.get(position),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:371:13 False  [LS3: assert!(known.get(48) && known.get(49));]  assert!(known.get(48) && known.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:371:30 False  [LS3: assert!(known.get(48) && known.get(49));]  assert!(known.get(48) && known.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:372:13 False  [LS3: assert!(!truth.get(48) && !truth.get(49));]  assert!(!truth.get(48) && !truth.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:372:31 False  [LS3: assert!(!truth.get(48) && !truth.get(49));]  assert!(!truth.get(48) && !truth.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:459:13 False  [LS3: assert!(known.get(33) && !truth.get(33));]  assert!(known.get(33) && !truth.get(33));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:459:30 False  [LS3: assert!(known.get(33) && !truth.get(33));]  assert!(known.get(33) && !truth.get(33));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:460:13 False  [LS3: assert!(!known.get(37) && !known.get(66));]  assert!(!known.get(37) && !known.get(66));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:460:31 False  [LS3: assert!(!known.get(37) && !known.get(66));]  assert!(!known.get(37) && !known.get(66));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:13 False  [LS3: assert!(truth.get(48) && truth.get(50) && truth.get(68));]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:30 False  [LS3: assert!(truth.get(48) && truth.get(50) && truth.get(68));]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:47 False  [LS3: assert!(truth.get(48) && truth.get(50) && truth.get(68));]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:9 False  [LS3: hammers > 0 && hammers < cases,]  hammers > 0 && hammers < cases,
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:24 False  [LS3: hammers > 0 && hammers < cases,]  hammers > 0 && hammers < cases,
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:13 False  [LS3: assert!(bullish > 0 && bearish > 0);]  assert!(bullish > 0 && bearish > 0);
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:28 False  [LS3: assert!(bullish > 0 && bearish > 0);]  assert!(bullish > 0 && bearish > 0);
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:17 False  [LS3: assert!(!truth.get(30) && !truth.get(31));]  assert!(!truth.get(30) && !truth.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:35 False  [LS3: assert!(!truth.get(30) && !truth.get(31));]  assert!(!truth.get(30) && !truth.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:17 False  [LS3: assert!(known.get(30) && known.get(31));]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:34 False  [LS3: assert!(known.get(30) && known.get(31));]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:17 False  [LS3: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:34 False  [LS3: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:51 False  [LS3: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:68 False  [LS3: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:9 False  [LS3: !next_day.get(157) && !next_day.get(176),]  !next_day.get(157) && !next_day.get(176),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:31 False  [LS3: !next_day.get(157) && !next_day.get(176),]  !next_day.get(157) && !next_day.get(176),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:9 False  [LS3: !next_day.get(276) && !next_day.get(277),]  !next_day.get(276) && !next_day.get(277),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:31 False  [LS3: !next_day.get(276) && !next_day.get(277),]  !next_day.get(276) && !next_day.get(277),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:17 False  [LS3: assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:47 False  [LS3: assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:21 False  [LS3: assert!(known.get(30) && known.get(31));]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:38 False  [LS3: assert!(known.get(30) && known.get(31));]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:17 False  [LS3: assert!(!truth.get(13) && !truth.get(16));]  assert!(!truth.get(13) && !truth.get(16));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:35 False  [LS3: assert!(!truth.get(13) && !truth.get(16));]  assert!(!truth.get(13) && !truth.get(16));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:13 False  [LS3: known.get(13) && known.get(16),]  known.get(13) && known.get(16),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:30 False  [LS3: known.get(13) && known.get(16),]  known.get(13) && known.get(16),
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:183:45 False  [LS3: assert!(!truth.get(position) || known.get(position));]  assert!(!truth.get(position) || known.get(position));
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:13 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:26 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:40 False  [LS3: trues > 0 && falses > 0 && unknowns > 0,]  trues > 0 && falses > 0 && unknowns > 0,
(F)         LINE   crates/lake/src/bar.rs:237  [LS3: other => panic!("expected NotRepresentable, got {other:?}"),]  other => panic!("expected NotRepresentable, got {other:?}"),
(F)         LINE   crates/lake/src/bar.rs:247  [LS3: other => panic!("expected NotRepresentable, got {other:?}"),]  other => panic!("expected NotRepresentable, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:412  [LS3: ContractKind::Future => panic!("a CE is not a future"),]  ContractKind::Future => panic!("a CE is not a future"),
(F)         LINE   crates/lake/src/contract.rs:563  [LS3: other => panic!("expected TooLong, got {other:?}"),]  other => panic!("expected TooLong, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:571  [LS3: name.len()]  name.len()
(F)         LINE   crates/lake/src/contract.rs:614  [LS3: ContractKind::Future => panic!("not a future"),]  ContractKind::Future => panic!("not a future"),
(F)         LINE   crates/lake/src/contract.rs:687  [LS3: other => panic!("{padded} must be refused, got {other:?}"),]  other => panic!("{padded} must be refused, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:721  [LS3: other => panic!("{signed} must be refused, got {other:?}"),]  other => panic!("{signed} must be refused, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:753  [LS3: other => panic!("{name} must be refused, got {other:?}"),]  other => panic!("{name} must be refused, got {other:?}"),
(F)         BRANCH crates/lake/src/contract.rs:786:61 False  [LS3: Err(ContractError::BadExpiryShape { found }) if found == "0\u{e9}Apr2"]  Err(ContractError::BadExpiryShape { found }) if found == "0\u{e9}Apr2"
(F)         BRANCH crates/lake/src/footer_tests.rs:32:12 True  [LS3: if let Err(e) = r {]  if let Err(e) = r {
(F)         LINE   crates/lake/src/footer_tests.rs:33  [LS3: panic!("expected the walk to admit, got {e:?}");]  panic!("expected the walk to admit, got {e:?}");
(F)         LINE   crates/lake/src/footer_tests.rs:40  [LS3: other => panic!("expected FooterUnreadable, got {other:?}"),]  other => panic!("expected FooterUnreadable, got {other:?}"),
(F)         LINE   crates/lake/src/footer_tests.rs:69  [LS3: other => panic!("expected FooterUnreadable, got {:?}", other.err()),]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F)         LINE   crates/lake/src/footer_tests.rs:80  [LS3: other => panic!("expected FooterUnreadable, got {:?}", other.err()),]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F)         BRANCH crates/lake/src/page.rs:631:13 False  [LS3: short.to_string().contains("4 bytes") && short.to_string().contains('5'),]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:631:54 False  [LS3: short.to_string().contains("4 bytes") && short.to_string().contains('5'),]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:640:13 False  [LS3: long.to_string().contains("4 bytes") && long.to_string().contains('3'),]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F)         BRANCH crates/lake/src/page.rs:640:53 False  [LS3: long.to_string().contains("4 bytes") && long.to_string().contains('3'),]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F)         BRANCH crates/lake/src/page.rs:681:13 False  [LS3: wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:681:65 False  [LS3: wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:741:13 False  [LS3: text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F)         BRANCH crates/lake/src/page.rs:741:51 False  [LS3: text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F)         LINE   crates/lake/src/page.rs:878  [LS3: other => panic!("expected a data page, got {other:?}"),]  other => panic!("expected a data page, got {other:?}"),
(F)         LINE   crates/lake/src/page.rs:986  [LS3: frame.len(),]  frame.len(),
(F)         LINE   crates/lake/src/page.rs:987  [LS3: plain.len()]  plain.len()
(F)         LINE   crates/lake/src/page.rs:1016  [LS3: other => panic!("expected a data page, got {other:?}"),]  other => panic!("expected a data page, got {other:?}"),
(F)         LINE   crates/lake/src/page.rs:1147  [LS3: frame.len()]  frame.len()
(F)         BRANCH crates/lake/src/page.rs:1192:13 False  [LS3: text.contains("zstd frame refused") && text.contains("134217729"),]  text.contains("zstd frame refused") && text.contains("134217729"),
(F)         BRANCH crates/lake/src/page.rs:1192:52 False  [LS3: text.contains("zstd frame refused") && text.contains("134217729"),]  text.contains("zstd frame refused") && text.contains("134217729"),
(F)         LINE   crates/lake/src/reader.rs:900  [LS3: other => panic!("expected TooLarge, got {:?}", other.err()),]  other => panic!("expected TooLarge, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:912  [LS3: matches!(]  matches!(
(F)         LINE   crates/lake/src/reader.rs:919  [LS3: matches!(]  matches!(
(F)         LINE   crates/lake/src/reader.rs:953  [LS3: other => panic!("expected NotParquet, got {other:?}", other = other.err()),]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/src/reader.rs:966  [LS3: other => panic!("expected NotParquet, got {other:?}", other = other.err()),]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/src/reader.rs:979  [LS3: other => panic!("len {len}: expected Truncated, got {:?}", other.err()),]  other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:1053  [LS3: other => panic!("expected ShortColumnChunk, got {other:?}"),]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1099  [LS3: other => panic!("expected ShortColumnChunk, got {other:?}"),]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1116  [LS3: other => panic!("expected ShortColumnChunk, got {other:?}"),]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1146  [LS3: other => panic!("expected UnexpectedNull, got {other:?}"),]  other => panic!("expected UnexpectedNull, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1281  [LS3: _ => panic!("the cash layout is INT64 and DOUBLE only"),]  _ => panic!("the cash layout is INT64 and DOUBLE only"),
(F)         LINE   crates/lake/src/reader.rs:1388  [LS3: other => panic!("expected ImpossibleLength, got {:?}", other.err()),]  other => panic!("expected ImpossibleLength, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:1422  [LS3: Err(other) => panic!("refused for the wrong reason: {other:?}"),]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1423  [LS3: Ok(batch) => panic!(]  Ok(batch) => panic!(
(F)         LINE   crates/lake/src/reader.rs:1425  [LS3: batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()]  batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()
(F)         LINE   crates/lake/src/reader.rs:1479  [LS3: Err(other) => panic!("refused for the wrong reason: {other:?}"),]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1480  [LS3: Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1530  [LS3: other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),]  other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1554  [LS3: Err(other) => panic!("refused for the wrong reason: {other:?}"),]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1555  [LS3: Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1626  [LS3: Err(other) => panic!("refused for the wrong reason: {other:?}"),]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1627  [LS3: Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),]  Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1727  [LS3: other => panic!("the padded group was not refused as short: {other:?}"),]  other => panic!("the padded group was not refused as short: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1756  [LS3: match cut {  (the zero-count region on `match cut {` is the `Err(RowCountsDisagree)` arm, whose body is the panic on the next lines; the other arm runs.)]  match cut {
(F)         LINE   crates/lake/src/reader.rs:1758  [LS3: panic!("the cut and its copy sum to the file count")]  panic!("the cut and its copy sum to the file count")
(F)         LINE   crates/lake/src/reader.rs:1801  [LS3: other => panic!("group {group} under {claim}: {other:?}"),]  other => panic!("group {group} under {claim}: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1896  [LS3: crate::error::ColumnType::Other => panic!("no lake column is Other"),]  crate::error::ColumnType::Other => panic!("no lake column is Other"),
(F)         LINE   crates/lake/src/reader.rs:1943  [LS3: _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),]  _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),
(F)         LINE   crates/lake/src/reader.rs:1983  [LS3: other => panic!("expected ColumnTypeMismatch, got {other:?}"),]  other => panic!("expected ColumnTypeMismatch, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:2061  [LS3: other => panic!("{lie}: expected ImpossibleLength, got {:?}", other.err()),]  other => panic!("{lie}: expected ImpossibleLength, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:2117  [LS3: panic!("the patched length did not settle");]  panic!("the patched length did not settle");
(F)         LINE   crates/lake/src/reader.rs:2139  [LS3: other => panic!("{past} past: expected PageDecode, got {:?}", other.err()),]  other => panic!("{past} past: expected PageDecode, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:2150  [LS3: other => panic!("a chunk ending at the last byte: {:?}", other.err()),]  other => panic!("a chunk ending at the last byte: {:?}", other.err()),
(F)         BRANCH crates/lake/src/reader.rs:2166:20 False  [LS3: if column == "timestamp" && reason.ends_with("runs past the end of the file")]  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F)         BRANCH crates/lake/src/reader.rs:2166:45 False  [LS3: if column == "timestamp" && reason.ends_with("runs past the end of the file")]  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F)         LINE   crates/lake/src/reader_cover_tests.rs:23  [LS3: other => panic!("a missing file must be an Io refusal: {other:?}"),]  other => panic!("a missing file must be an Io refusal: {other:?}"),
(F)         LINE   crates/lake/src/reader_cover_tests.rs:36  [LS3: other => panic!("/dev/zero under cap {cap}: {other:?}"),]  other => panic!("/dev/zero under cap {cap}: {other:?}"),
(U)         BRANCH crates/lake/src/schema.rs:125:12 True  [LS3: if i >= n {]  if i >= n {
(U)         LINE   crates/lake/src/schema.rs:126  [LS3: return Err(LakeError::MissingColumn { name: s.name });]  return Err(LakeError::MissingColumn { name: s.name });
(F)         LINE   crates/lake/src/schema_cover_tests.rs:58  [LS3: ColumnType::Int32 | ColumnType::Other => panic!("the cash layout has neither"),]  ColumnType::Int32 | ColumnType::Other => panic!("the cash layout has neither"),
(F)         LINE   crates/lake/src/schema_cover_tests.rs:97  [LS3: other => panic!("{shown}: expected UnsupportedTimestamp, got {other:?}"),]  other => panic!("{shown}: expected UnsupportedTimestamp, got {other:?}"),
(F)         LINE   crates/lake/src/zstd_v082_frames.rs:175  [LS3: other => panic!("no synthetic input called {other}"),]  other => panic!("no synthetic input called {other}"),
(F)         LINE   crates/lake/tests/events.rs:159  [LS3: _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),]  _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
(F)         LINE   crates/lake/tests/events.rs:366  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/events.rs:401  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:176  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:234  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:310  [LS3: Ok(batch) => {]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:311  [LS3: let got: Vec<Option<i64>> = (0..batch.len())]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:312  [LS3: .map(|i| batch.row(i).expect("row in range").open_interest())]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:313  [LS3: .collect();]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:314  [LS3: let real = got.iter().filter(|v| v.is_some()).count();]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:315  [LS3: let invented = got.len() - real;]  let invented = got.len() - real;
(F)         LINE   crates/lake/tests/refusals.rs:316  [LS3: let first = got.iter().position(Option::is_none);]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:317  [LS3: let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));]  let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));
(F)         LINE   crates/lake/tests/refusals.rs:318  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:326  [LS3: batch.len()]  batch.len()
(F)         LINE   crates/lake/tests/refusals.rs:377  [LS3: Ok(batch) => {]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:378  [LS3: let got: Vec<Option<i64>> = (0..batch.len())]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:379  [LS3: .map(|i| batch.row(i).expect("row in range").open_interest())]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:380  [LS3: .collect();]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:381  [LS3: let real = got.iter().filter(|v| v.is_some()).count();]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:382  [LS3: let first = got.iter().position(Option::is_none);]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:383  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:386  [LS3: batch.len(),]  batch.len(),
(F)         LINE   crates/lake/tests/refusals.rs:387  [LS3: got.len() - real,]  got.len() - real,
(F)         LINE   crates/lake/tests/refusals.rs:388  [LS3: first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))]  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F)         LINE   crates/lake/tests/refusals.rs:432  [LS3: Ok(batch) => {]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:433  [LS3: let got: Vec<Option<i64>> = (0..batch.len())]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:434  [LS3: .map(|i| batch.row(i).expect("row in range").open_interest())]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:435  [LS3: .collect();]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:436  [LS3: let real = got.iter().filter(|v| v.is_some()).count();]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:437  [LS3: let first = got.iter().position(Option::is_none);]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:438  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:442  [LS3: batch.len(),]  batch.len(),
(F)         LINE   crates/lake/tests/refusals.rs:443  [LS3: got.len() - real,]  got.len() - real,
(F)         LINE   crates/lake/tests/refusals.rs:444  [LS3: first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))]  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F)         LINE   crates/lake/tests/refusals.rs:558  [LS3: _ => panic!("one INT64 column"),]  _ => panic!("one INT64 column"),
(F)         LINE   crates/lake/tests/refusals.rs:682  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:703  [LS3: Err(other) => panic!(]  Err(other) => panic!(
(F)         LINE   crates/lake/tests/refusals.rs:715  [LS3: Ok(file) => panic!(]  Ok(file) => panic!(
(F)         LINE   crates/lake/tests/refusals.rs:795  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:814  [LS3: other => panic!("a {what} leaf must be refused by name, got {other:?}"),]  other => panic!("a {what} leaf must be refused by name, got {other:?}"),
(F)         BRANCH crates/lake/tests/refusals.rs:1061:17 False  [LS3: let ColumnWriter::Int64ColumnWriter(typed) = column.untyped() else {]  let ColumnWriter::Int64ColumnWriter(typed) = column.untyped() else {
(F)         LINE   crates/lake/tests/refusals.rs:1062  [LS3: panic!("every leaf is INT64");]  panic!("every leaf is INT64");
(F)         LINE   crates/lake/tests/refusals.rs:1203  [LS3: s.unreadable[0]]  s.unreadable[0]
(F)         LINE   crates/lake/tests/refusals.rs:1240  [LS3: s.unreadable[0]]  s.unreadable[0]
(F)         LINE   crates/lake/tests/refusals.rs:1558  [LS3: Ok(other) => {]  Ok(other) => {
(F)         LINE   crates/lake/tests/refusals.rs:1559  [LS3: panic!(]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:1569  [LS3: other.to_string() == padded,]  other.to_string() == padded,
(F)         LINE   crates/lake/tests/refusals.rs:1570  [LS3: other == good,]  other == good,
(F)         LINE   crates/lake/tests/refusals.rs:1571  [LS3: other.key() == good.key()]  other.key() == good.key()
(F)         BRANCH crates/lake/tests/refusals.rs:1619:9 False  [LS3: text.contains("400") && text.contains("2480"),]  text.contains("400") && text.contains("2480"),
(F)         BRANCH crates/lake/tests/refusals.rs:1619:33 False  [LS3: text.contains("400") && text.contains("2480"),]  text.contains("400") && text.contains("2480"),
(F)         LINE   crates/lake/tests/refusals.rs:1699  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         BRANCH crates/lake/tests/refusals.rs:1748:71 False  [LS3: Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at]  Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
(F)         LINE   crates/lake/tests/refusals.rs:1841  [LS3: _ => panic!("the cash layout has no other physical type"),]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:1870  [LS3: Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:1871  [LS3: Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),]  Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:1949  [LS3: _ => panic!("this fixture writes no other physical type"),]  _ => panic!("this fixture writes no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:1980  [LS3: Err(other) => panic!("refused as the wrong thing: {other:?}"),]  Err(other) => panic!("refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:1981  [LS3: Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),]  Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
(F)         LINE   crates/lake/tests/refusals.rs:1999  [LS3: Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),]  Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2000  [LS3: Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),]  Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2050  [LS3: Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),]  Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2051  [LS3: Ok(_) => panic!("an INT96 timestamp opened"),]  Ok(_) => panic!("an INT96 timestamp opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2101  [LS3: Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2102  [LS3: Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),]  Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2137  [LS3: other => panic!("an INT32 {leaf}: {:?}", other.map(|f| f.num_rows())),]  other => panic!("an INT32 {leaf}: {:?}", other.map(|f| f.num_rows())),
(F)         LINE   crates/lake/tests/synthetic.rs:136  [LS3: _ => panic!("writer type does not match the column"),]  _ => panic!("writer type does not match the column"),
(F)         LINE   crates/lake/tests/synthetic.rs:231  [LS3: f.created_by()]  f.created_by()
(F)         LINE   crates/lake/tests/synthetic.rs:353  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:355  [LS3: other = other.err()]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:403  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:405  [LS3: other = other.err()]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:453  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:455  [LS3: other = other.err()]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:473  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:475  [LS3: other = other.err()]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:626  [LS3: other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),]  other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/tests/synthetic.rs:639  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:641  [LS3: other = other.err()]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:662  [LS3: matches!(]  matches!(
(F)         LINE   crates/lake/tests/synthetic.rs:780  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:782  [LS3: other.map(|b| b.len())]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:820  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:822  [LS3: other.map(|b| b.len())]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:952  [LS3: other => panic!("page type {} is not one this writer emits", other.0),]  other => panic!("page type {} is not one this writer emits", other.0),
(F)         LINE   crates/lake/tests/synthetic.rs:1154  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:1156  [LS3: other.map(|groups| groups.len())]  other.map(|groups| groups.len())
(F)         LINE   crates/lake/tests/synthetic.rs:1221  [LS3: other => panic!(]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:1223  [LS3: other.map(|b| b.len())]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:1307  [LS3: Ok(rows) => panic!("decoded {} rows from a broken page", rows.len()),]  Ok(rows) => panic!("decoded {} rows from a broken page", rows.len()),
(U)         BRANCH crates/runner/src/admission.rs:2194:36 True  [LS2: [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5056:36 True  [LS2: [test code]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5228:36 True  [LS2: [production]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5771:45 True  [LS2: [test code]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
(U)         BRANCH crates/runner/src/admission.rs:6121:45 True  [LS2: [production]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
(F)         LINE   crates/runner/src/admission.rs:7423  [LS2: [assert_eq! message argument]  reason.name()]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7429  [LS2: [assert_eq! message argument]  reason.name()]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7435  [LS2: [assert_eq! message argument]  reason.name()]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7466  [LS2: [assert_eq! message argument]  left.name(),]  left.name(),
(F)         LINE   crates/runner/src/admission.rs:7467  [LS2: [assert_eq! message argument]  right.name()]  right.name()
(F)         BRANCH crates/runner/src/admission_reader_cover_tests.rs:285:70 False  [LS2: [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F)         BRANCH crates/runner/src/admission_reader_cover_tests.rs:289:70 False  [LS2: [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F)         BRANCH crates/runner/src/audit.rs:2224:13 False  [LS2: [assert! condition branch (failing side)]  ranked]  ranked
(F)         BRANCH crates/runner/src/audit.rs:2227:20 False  [LS2: [assert! condition branch (failing side)]  && ranked.ends_with("what selection ranks on, gross of every charge"),]  && ranked.ends_with("what selection ranks on, gross of every charge"),
(F)         BRANCH crates/runner/src/audit.rs:2358:44 False  [LS2: [assert! condition branch (failing side): the False side of a disjunction's last operand]  line.is_empty() || line.starts_with("  "),]  line.is_empty() || line.starts_with("  "),
(F)         BRANCH crates/runner/src/audit.rs:2467:29 False  [LS2: [assert! condition branch (failing side): the False side of a disjunction's last operand]  decision || token == "V6",]  decision || token == "V6",
(F)         BRANCH crates/runner/src/audit.rs:2620:40 False  [LS2: [assert! condition branch (failing side): the False side of a disjunction's last operand]  line.is_empty() || line.starts_with("  "),]  line.is_empty() || line.starts_with("  "),
(F)         BRANCH crates/runner/src/audit.rs:2788:13 False  [LS2: [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F)         BRANCH crates/runner/src/audit.rs:2788:54 False  [LS2: [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F)         BRANCH crates/runner/src/audit.rs:3067:17 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3067:35 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3067:54 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:17 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:35 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:54 False  [LS2: [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(U)         BRANCH crates/runner/src/bootstrap.rs:628:17 False  [LS2: [production]  let Some(own) = stats.get(s) else { continue };]  let Some(own) = stats.get(s) else { continue };
(U)         BRANCH crates/runner/src/bootstrap.rs:727:8 True  [LS2: [production]  if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {]  if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {
(U)         LINE   crates/runner/src/bootstrap.rs:766  [LS2: [production]  )?,]  )?,
(U)         BRANCH crates/runner/src/bootstrap.rs:802:8 True  [LS2: [production]  if !observed.is_finite() {]  if !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:803  [LS2: [production]  return None;]  return None;
(U)         LINE   crates/runner/src/bootstrap.rs:851  [LS2: [production]  )?,]  )?,
(U)         BRANCH crates/runner/src/bootstrap.rs:868:8 True  [LS2: [production]  if stats.iter().any(|summary| {]  if stats.iter().any(|summary| {
(U)         BRANCH crates/runner/src/bootstrap.rs:869:9 True  [LS2: [production]  !summary.mean.is_finite()]  !summary.mean.is_finite()
(U)         BRANCH crates/runner/src/bootstrap.rs:870:16 True  [LS2: [production]  || !summary.standard_error.is_finite()]  || !summary.standard_error.is_finite()
(U)         LINE   crates/runner/src/bootstrap.rs:873  [LS2: [production]  return None;]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:892:8 True  [LS2: [production]  if matched_or_exceeded > draws || !observed.is_finite() {]  if matched_or_exceeded > draws || !observed.is_finite() {
(U)         BRANCH crates/runner/src/bootstrap.rs:892:39 True  [LS2: [production]  if matched_or_exceeded > draws || !observed.is_finite() {]  if matched_or_exceeded > draws || !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:893  [LS2: [production]  return None;]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:900:8 True  [LS2: [production]  if !p_value.is_finite() {]  if !p_value.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:901  [LS2: [production]  return None;]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:1347:8 True  [LS2: [production]  if observed.iter().any(|value| !value.is_finite()) {]  if observed.iter().any(|value| !value.is_finite()) {
(U)         LINE   crates/runner/src/bootstrap.rs:1348  [LS2: [production]  return None;]  return None;
(U)         LINE   crates/runner/src/bootstrap.rs:1362  [LS2: [production]  _ => left.cmp(right),]  _ => left.cmp(right),
(U)         LINE   crates/runner/src/bootstrap.rs:1547  [LS2: [production]  _ => left.cmp(right),]  _ => left.cmp(right),
(U)         BRANCH crates/runner/src/bootstrap.rs:1565:13 False  [LS2: [production]  let Some(&strategy) = order.get(rank) else {]  let Some(&strategy) = order.get(rank) else {
(U)         LINE   crates/runner/src/bootstrap.rs:1566  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/bootstrap.rs:1568:13 False  [LS2: [production]  let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {]  let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {
(U)         LINE   crates/runner/src/bootstrap.rs:1569  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/bootstrap.rs:1588:16 False  [LS2: [production]  if let Some(slot) = bars.get_mut(rank) {]  if let Some(slot) = bars.get_mut(rank) {
(U)         BRANCH crates/runner/src/bootstrap.rs:1599:17 False  [LS2: [production]  let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))]  let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))
(U)         LINE   crates/runner/src/bootstrap.rs:1601  [LS2: [production]  break;]  break;
(U)         BRANCH crates/runner/src/bootstrap.rs:1620:16 False  [LS2: [production]  if let Some(this_round) = out.get_mut(first..) {]  if let Some(this_round) = out.get_mut(first..) {
(U)         LINE   crates/runner/src/bootstrap.rs:1622  [LS2: [production]  }]  }
(F)         LINE   crates/runner/src/bootstrap.rs:2108  [LS2: [assert! message argument]  v.calibration()]  v.calibration()
(F)         LINE   crates/runner/src/bootstrap.rs:2188  [LS2: [assert! message argument]  (n as f64).sqrt()]  (n as f64).sqrt()
(F)         BRANCH crates/runner/src/bootstrap.rs:2435:13 False  [LS2: [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),]  names.contains(&18) && names.contains(&19),
(F)         BRANCH crates/runner/src/bootstrap.rs:2435:36 False  [LS2: [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),]  names.contains(&18) && names.contains(&19),
(F)         BRANCH crates/runner/src/bootstrap.rs:2652:17 False  [LS2: [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F)         BRANCH crates/runner/src/bootstrap.rs:2652:29 False  [LS2: [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F)         LINE   crates/runner/src/bootstrap.rs:2722  [LS2: [assert! message argument]  draw.runs.len()]  draw.runs.len()
(F)         BRANCH crates/runner/src/bootstrap.rs:3409:13 False  [LS2: [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),]  rounds < rejected.len() && rejected.len() <= set.len(),
(F)         BRANCH crates/runner/src/bootstrap.rs:3409:40 False  [LS2: [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),]  rounds < rejected.len() && rejected.len() <= set.len(),
(F)         LINE   crates/runner/src/bootstrap.rs:3411  [LS2: [assert! message argument]  rounds + 1,]  rounds + 1,
(F)         LINE   crates/runner/src/bootstrap.rs:3412  [LS2: [assert! message argument]  rejected.len(),]  rejected.len(),
(F)         LINE   crates/runner/src/bootstrap.rs:3413  [LS2: [assert! message argument]  set.len()]  set.len()
(F)         LINE   crates/runner/src/bootstrap.rs:3520  [LS2: [assert! message argument]  rc.map(|v| v.p_value)]  rc.map(|v| v.p_value)
(F)         LINE   crates/runner/src/bootstrap.rs:3526  [LS2: [assert! message argument]  v.map(|v| (v.p_value, v.clears()))]  v.map(|v| (v.p_value, v.clears()))
(U)         BRANCH crates/runner/src/bootstrap_family_pass.rs:461:20 True  [LS2: [production]  || !stat.standard_error.is_finite()]  || !stat.standard_error.is_finite()
(F)         BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:13 False  [LS2: [assert! condition branch (failing side)]  matched > 0 && matched < draws,]  matched > 0 && matched < draws,
(F)         BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:28 False  [LS2: [assert! condition branch (failing side)]  matched > 0 && matched < draws,]  matched > 0 && matched < draws,
(U)         BRANCH crates/runner/src/bootstrap_zero_v2.rs:187:45 True  [LS2: [production]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
(U)         BRANCH crates/runner/src/bootstrap_zero_v2.rs:191:12 True  [LS2: [production]  if !observed.is_finite() {]  if !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap_zero_v2.rs:192  [LS2: [production]  return Err(Refusal::Numerical);]  return Err(Refusal::Numerical);
(U)         BRANCH crates/runner/src/excursion.rs:234:24 False  [LS2: [production]  if let Some(prefix) = observed.get_mut(..bound) {]  if let Some(prefix) = observed.get_mut(..bound) {
(U)         BRANCH crates/runner/src/excursion.rs:1221:20 False  [LS2: [production]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1232:20 False  [LS2: [production]  if let Some(slot) = out.favourable.get_mut(target_cursor) {]  if let Some(slot) = out.favourable.get_mut(target_cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1253:20 False  [LS2: [production]  if let Some(slot) = armed_from.get_mut(arm) {]  if let Some(slot) = armed_from.get_mut(arm) {
(U)         BRANCH crates/runner/src/excursion.rs:1397:13 False  [LS2: [production]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
(U)         LINE   crates/runner/src/excursion.rs:1405  [LS2: [production]  break;]  break;
(U)         BRANCH crates/runner/src/excursion.rs:1472:16 False  [LS2: [production]  if let Some(slot) = crossed_at.get_mut(*cursor) {]  if let Some(slot) = crossed_at.get_mut(*cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1479:16 False  [LS2: [production]  if let Some(slot) = peak.get_mut(*cursor) {]  if let Some(slot) = peak.get_mut(*cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1482:16 False  [LS2: [production]  if let Some(slot) = peak_raised.get_mut(*cursor) {]  if let Some(slot) = peak_raised.get_mut(*cursor) {
(F)         LINE   crates/runner/src/excursion.rs:1682  [LS2: [assert! message argument]  tightest(8),]  tightest(8),
(F)         LINE   crates/runner/src/excursion.rs:1683  [LS2: [assert! message argument]  tightest(4)]  tightest(4)
(F)         LINE   crates/runner/src/excursion.rs:1688  [LS2: [assert! message argument]  tightest(4),]  tightest(4),
(F)         LINE   crates/runner/src/excursion.rs:1689  [LS2: [assert! message argument]  tightest(2)]  tightest(2)
(F)         LINE   crates/runner/src/excursion.rs:2582  [LS2: [assert_eq! message argument]  sample.len()]  sample.len()
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:605:12 False  [LS2: [production]  && core::ptr::eq(window, evaluated_execution_1m)]  && core::ptr::eq(window, evaluated_execution_1m)
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [LS2: [production]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:615  [LS2: [production]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:676:13 False  [LS2: [let-else whose block is only unreachable!]  let Some(last) = forged.last_mut() else {]  let Some(last) = forged.last_mut() else {
(F)         LINE   crates/runner/src/exit_grid_policy.rs:677  [LS2: [unreachable!/panic! in test code (failing arm)]  unreachable!("the copy holds bars 17..250")]  unreachable!("the copy holds bars 17..250")
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:1132:12 True  [LS2: [production; its proof names line 1131, as at 2b998e4b5f6e]  if pair_count > self.ratios.max_pairs() {]  if pair_count > self.ratios.max_pairs() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1133  [LS2: [production; its proof names line 1132, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1134  [LS2: [production; its proof names line 1133, as at 2b998e4b5f6e]  needed: pair_count,]  needed: pair_count,
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1135  [LS2: [production; its proof names line 1134, as at 2b998e4b5f6e]  max: self.ratios.max_pairs(),]  max: self.ratios.max_pairs(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1136  [LS2: [production; its proof names line 1135, as at 2b998e4b5f6e]  });]  });
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1173  [LS2: [test code; its proof names line 1172, as at 2b998e4b5f6e]  )?;]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2132:25 False  [LS2: [production; its proof names line 2131, as at 2b998e4b5f6e]  let Some(next) = pricing_refused.checked_add(1) else {]  let Some(next) = pricing_refused.checked_add(1) else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2133  [LS2: [production; its proof names line 2132, as at 2b998e4b5f6e]  return false;]  return false;
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2458  [LS2: [test code; its proof names line 2457, as at 2b998e4b5f6e]  )?;]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2676:12 True  [LS2: [production; its proof names line 2675, as at 2b998e4b5f6e]  if !self.chosen_is_in_bounds(coordinate) {]  if !self.chosen_is_in_bounds(coordinate) {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2677  [LS2: [production; its proof names line 2676, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2680:12 True  [LS2: [production; its proof names line 2679, as at 2b998e4b5f6e]  if selected.resolution_digest != self.digest]  if selected.resolution_digest != self.digest
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2681:16 True  [LS2: [production; its proof names line 2680, as at 2b998e4b5f6e]  || selected.run_id != evaluated.run_id]  || selected.run_id != evaluated.run_id
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2682:16 True  [LS2: [production; its proof names line 2681, as at 2b998e4b5f6e]  || selected.mask != evaluated.mask]  || selected.mask != evaluated.mask
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2683:16 True  [LS2: [production; its proof names line 2682, as at 2b998e4b5f6e]  || selected.horizon != evaluated.horizon]  || selected.horizon != evaluated.horizon
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2684:16 True  [LS2: [production; its proof names line 2683, as at 2b998e4b5f6e]  || selected.column_digest != evaluated.column_digest]  || selected.column_digest != evaluated.column_digest
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2685:16 True  [LS2: [production; its proof names line 2684, as at 2b998e4b5f6e]  || selected.evaluation_spec != evaluated.evaluation_spec]  || selected.evaluation_spec != evaluated.evaluation_spec
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2686:16 True  [LS2: [production; its proof names line 2685, as at 2b998e4b5f6e]  || selected.side != evaluated.side]  || selected.side != evaluated.side
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2687:16 True  [LS2: [production; its proof names line 2686, as at 2b998e4b5f6e]  || selected.coordinate != coordinate]  || selected.coordinate != coordinate
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2688:16 True  [LS2: [production; its proof names line 2687, as at 2b998e4b5f6e]  || selected.training_cell != *cell]  || selected.training_cell != *cell
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2689:16 True  [LS2: [production; its proof names line 2688, as at 2b998e4b5f6e]  || selected.selection_digest]  || selected.selection_digest
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2702  [LS2: [production; its proof names line 2701, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::SelectionDigestMismatch);]  return Err(ExitGridErrorV1::SelectionDigestMismatch);
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2877  [LS2: [production; its proof names line 2876, as at 2b998e4b5f6e]  )?;]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3413:13 False  [LS2: [production; its proof names line 3412, as at 2b998e4b5f6e]  let Some(left) = pair.first() else {]  let Some(left) = pair.first() else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3414  [LS2: [production; its proof names line 3413, as at 2b998e4b5f6e]  return true;]  return true;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3416:13 False  [LS2: [production; its proof names line 3415, as at 2b998e4b5f6e]  let Some(right) = pair.last() else {]  let Some(right) = pair.last() else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3417  [LS2: [production; its proof names line 3416, as at 2b998e4b5f6e]  return true;]  return true;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3595:12 True  [LS2 line 3591: [production; its proof names line 3590, as at 2b998e4b5f6e]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3595:45 True  [LS2 line 3591: [production; its proof names line 3590, as at 2b998e4b5f6e]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3672:8 True  [LS2 line 3668: [production; its proof names line 3667, as at 2b998e4b5f6e]  if out.is_empty() {]  if out.is_empty() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3673  [LS2 line 3669: [production; its proof names line 3668, as at 2b998e4b5f6e]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4147:12 True  [LS2 line 4139: [production]  || !census.reconciles()]  || !census.reconciles()
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4148:12 True  [LS2 line 4140: [production]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4149:12 True  [LS2 line 4141: [production]  || census.refused() != 0]  || census.refused() != 0
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4164:8 True  [LS2 line 4156: [production]  if column.bits().len() != column.sources().len() {]  if column.bits().len() != column.sources().len() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4165  [LS2 line 4157: [production]  return Err(ExitGridErrorV1::ColumnShapeMismatch {]  return Err(ExitGridErrorV1::ColumnShapeMismatch {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4166  [LS2 line 4158: [production]  bits: column.bits().len(),]  bits: column.bits().len(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4167  [LS2 line 4159: [production]  sources: column.sources().len(),]  sources: column.sources().len(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4168  [LS2 line 4160: [production]  });]  });
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4181:12 True  [LS2 line 4173: [production]  if previous.is_some_and(|before| source <= before) {]  if previous.is_some_and(|before| source <= before) {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4182  [LS2 line 4174: [production]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4183  [LS2 line 4175: [production]  previous: previous.unwrap_or(source),]  previous: previous.unwrap_or(source),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4184  [LS2 line 4176: [production]  current: source,]  current: source,
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4185  [LS2 line 4177: [production]  });]  });
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:7850:13 False  [LS2 line 7842: [let-else whose block is only unreachable!]  let crate::grid::ReplayPathV1::Priceable(price) = &mut candidate.path else {]  let crate::grid::ReplayPathV1::Priceable(price) = &mut candidate.path else {
(F)         LINE   crates/runner/src/exit_grid_policy.rs:7851  [LS2 line 7843: [unreachable! (failing arm)]  unreachable!("the fixture prices every path")]  unreachable!("the fixture prices every path")
(F)         LINE   crates/runner/src/exit_grid_policy.rs:7993  [LS2 line 7985: [assert! message argument]  universe.candidates.len()]  universe.candidates.len()
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:7997:13 False  [LS2 line 7989: [assert! condition branch (failing side)]  first.entry_bar > 0 && matches!(first.path, crate::grid::ReplayPathV1::Priceable(_))]  first.entry_bar > 0 && matches!(first.path, crate::grid::ReplayPathV1::Priceable(_))
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:13 False  [LSA: assert! condition, failing side (a resolution with no stop rung) in LSA's every_buffer_only_a_refusing_allocator_fails_is_refused_by_name_with_its_size]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:26 False  [LSA: assert! condition, failing side (no target rung), the same test]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:41 False  [LSA: assert! condition, failing side (no trail rung), the same test]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8861:17 False  [LS2 line 8754: [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8861:44 False  [LS2 line 8754: [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8870:17 False  [LS2 line 8763: [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8870:45 False  [LS2 line 8763: [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(U)         BRANCH crates/runner/src/expression_execution.rs:357:12 True  [LS2: [production]  if summary.hits != grid.signals {]  if summary.hits != grid.signals {
(U)         LINE   crates/runner/src/expression_execution.rs:358  [LS2: [production]  return Err("expression exact truth summary differs from grid signals".to_owned());]  return Err("expression exact truth summary differs from grid signals".to_owned());
(U)         LINE   crates/runner/src/expression_oos.rs:299  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/expression_oos.rs:302:12 True  [LS2: [production]  if summary.hits != grid.signals {]  if summary.hits != grid.signals {
(U)         LINE   crates/runner/src/expression_oos.rs:303  [LS2: [production]  return Err("later expression truth/grid signals differ".into());]  return Err("later expression truth/grid signals differ".into());
(U)         LINE   crates/runner/src/expression_validation.rs:325  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/expression_validation.rs:345:12 True  [LS2: [production]  if totals.trades != cell.trades]  if totals.trades != cell.trades
(U)         BRANCH crates/runner/src/expression_validation.rs:346:16 True  [LS2: [production]  || totals.wins != cell.wins]  || totals.wins != cell.wins
(U)         BRANCH crates/runner/src/expression_validation.rs:347:16 True  [LS2: [production]  || totals.return_paisa != cell.pessimistic]  || totals.return_paisa != cell.pessimistic
(U)         LINE   crates/runner/src/expression_validation.rs:349  [LS2: [production]  return Err("fixed-training fold totals disagree with exact later cell".into());]  return Err("fixed-training fold totals disagree with exact later cell".into());
(U)         BRANCH crates/runner/src/expression_validation.rs:516:12 True  [LS2: [production]  if first != last]  if first != last
(U)         BRANCH crates/runner/src/expression_validation.rs:517:16 True  [LS2: [production]  || trade.entry_micros != entry.ts_micros]  || trade.entry_micros != entry.ts_micros
(U)         BRANCH crates/runner/src/expression_validation.rs:518:16 True  [LS2: [production]  || trade.exit_micros != exit.ts_micros]  || trade.exit_micros != exit.ts_micros
(U)         BRANCH crates/runner/src/expression_validation.rs:519:16 True  [LS2: [production]  || day(entry.ts_micros) != day(exit.ts_micros)]  || day(entry.ts_micros) != day(exit.ts_micros)
(U)         BRANCH crates/runner/src/expression_validation.rs:520:16 True  [LS2: [production]  || trade.exit_bar < trade.entry_bar]  || trade.exit_bar < trade.entry_bar
(U)         BRANCH crates/runner/src/expression_validation.rs:521:16 True  [LS2: [production]  || previous.is_some_and(|index| trade.entry_bar <= index)]  || previous.is_some_and(|index| trade.entry_bar <= index)
(U)         LINE   crates/runner/src/expression_validation.rs:523  [LS2: [production]  return Err("fixed-training trade crosses source/fold/session boundaries".into());]  return Err("fixed-training trade crosses source/fold/session boundaries".into());
(U)         BRANCH crates/runner/src/grid.rs:1733:17 True  [LS2: [production]  let at = if at > last { last } else { at };]  let at = if at > last { last } else { at };
(U)         LINE   crates/runner/src/grid.rs:2095  [LS2: [production]  )?;]  )?;
(U)         LINE   crates/runner/src/grid.rs:2673  [LS2: [production]  )?;]  )?;
(U)         LINE   crates/runner/src/grid.rs:2700  [LS2: [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")
(U)         LINE   crates/runner/src/grid.rs:2701  [LS2: [production]  })?;]  })?;
(U)         LINE   crates/runner/src/grid.rs:2722  [LS2: [production]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")
(U)         LINE   crates/runner/src/grid.rs:2723  [LS2: [production]  })?;]  })?;
(U)         LINE   crates/runner/src/grid.rs:3088  [LS2: [production]  )?;]  )?;
(U)         LINE   crates/runner/src/grid.rs:3376  [LS2: [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3377  [LS2: [production]  "candidate count does not fit the V1 receipt",]  "candidate count does not fit the V1 receipt",
(U)         LINE   crates/runner/src/grid.rs:3378  [LS2: [production]  )]  )
(U)         LINE   crates/runner/src/grid.rs:3379  [LS2: [production]  })?;]  })?;
(U)         LINE   crates/runner/src/grid.rs:3435  [LS2 line 3433: [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3436  [LS2 line 3434: [production]  "candidate count does not fit the V1 receipt",]  "candidate count does not fit the V1 receipt",
(U)         LINE   crates/runner/src/grid.rs:3437  [LS2 line 3435: [production]  )]  )
(U)         LINE   crates/runner/src/grid.rs:3438  [LS2 line 3436: [production]  })?;]  })?;
(U)         LINE   crates/runner/src/grid.rs:3488  [LS2 line 3478: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/grid.rs:3623:32 True  [LS2 line 3613: [production]  if measured.trades != 1 || one_row.len() != 1 {]  if measured.trades != 1 || one_row.len() != 1 {
(U)         BRANCH crates/runner/src/grid.rs:3630:8 True  [LS2 line 3620: [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
(U)         BRANCH crates/runner/src/grid.rs:3630:39 True  [LS2 line 3620: [production]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
(U)         LINE   crates/runner/src/grid.rs:3631  [LS2 line 3621: [production]  return Err(]  return Err(
(U)         LINE   crates/runner/src/grid.rs:3632  [LS2 line 3622: [production]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3633  [LS2 line 3623: [production]  "one candidate produced impossible multi-trade quality counts",]  "one candidate produced impossible multi-trade quality counts",
(U)         LINE   crates/runner/src/grid.rs:3634  [LS2 line 3624: [production]  ),]  ),
(U)         LINE   crates/runner/src/grid.rs:3635  [LS2 line 3625: [production]  );]  );
(U)         LINE   crates/runner/src/grid.rs:3641  [LS2 line 3631: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/grid.rs:3647:8 True  [LS2 line 3637: [production]  if row.signal_bar != candidate.signal]  if row.signal_bar != candidate.signal
(U)         BRANCH crates/runner/src/grid.rs:3648:12 True  [LS2 line 3638: [production]  || row.entry_bar != candidate.entry]  || row.entry_bar != candidate.entry
(U)         BRANCH crates/runner/src/grid.rs:3650:12 True  [LS2 line 3640: [production]  || row.exit_micros != stamp]  || row.exit_micros != stamp
(U)         BRANCH crates/runner/src/grid.rs:3651:12 True  [LS2 line 3641: [production]  || row.exit_bar < candidate.entry]  || row.exit_bar < candidate.entry
(U)         BRANCH crates/runner/src/grid.rs:4758:12 False  [LS2 line 4748: [production]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {
(F)         BRANCH crates/runner/src/grid.rs:6264:13 False  [LS2 line 6254: [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),]  !replay.is_empty() && !replay.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6264:35 False  [LS2 line 6254: [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),]  !replay.is_empty() && !replay.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6285:13 False  [LS2 line 6275: [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),]  !in_loop.is_empty() && !in_loop.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6285:36 False  [LS2 line 6275: [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),]  !in_loop.is_empty() && !in_loop.contains(built),
(F)         LINE   crates/runner/src/grid.rs:7098  [LS2 line 7088: [assert! message argument]  stopped.edge_ratio(),]  stopped.edge_ratio(),
(F)         LINE   crates/runner/src/grid.rs:7099  [LS2 line 7089: [assert! message argument]  stopless.edge_ratio()]  stopless.edge_ratio()
(F)         BRANCH crates/runner/src/grid.rs:7992:13 False  [LS2 line 7982: [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),]  !long.cells.is_empty() && !short.cells.is_empty(),
(F)         BRANCH crates/runner/src/grid.rs:7992:39 False  [LS2 line 7982: [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),]  !long.cells.is_empty() && !short.cells.is_empty(),
(F)         LINE   crates/runner/src/grid.rs:8110  [LS2 line 8100: [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  !matches!(]  !matches!(
(F)         BRANCH crates/runner/src/grid.rs:8204:17 False  [LS2 line 8194: [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());]  assert!(base.stop.is_none() && base.target.is_none());
(F)         BRANCH crates/runner/src/grid.rs:8204:40 False  [LS2 line 8194: [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());]  assert!(base.stop.is_none() && base.target.is_none());
(F)         LINE   crates/runner/src/grid.rs:8390  [LS2 line 8380: [assert! message argument]  c.uncertainty()]  c.uncertainty()
(F)         LINE   crates/runner/src/grid.rs:9824  [LS2 line 9814: [assert! message argument]  a.reward_to_risk_bp()]  a.reward_to_risk_bp()
(F)         LINE   crates/runner/src/grid.rs:10293  [LS2 line 10283: [assert! message argument]  sniper.assurance_bp(),]  sniper.assurance_bp(),
(F)         LINE   crates/runner/src/grid.rs:10294  [LS2 line 10284: [assert! message argument]  grinder.assurance_bp()]  grinder.assurance_bp()
(F)         LINE   crates/runner/src/grid.rs:10300  [LS2 line 10290: [assert! message argument]  grinder.assurance_bp(),]  grinder.assurance_bp(),
(F)         LINE   crates/runner/src/grid.rs:10301  [LS2 line 10291: [assert! message argument]  fluke.assurance_bp()]  fluke.assurance_bp()
(F)         LINE   crates/runner/src/grid.rs:10340  [LS2 line 10330: [assert! message argument]  cell.win_rate_bp()]  cell.win_rate_bp()
(F)         LINE   crates/runner/src/grid.rs:10374  [LS2 line 10364: [assert! message argument]  trades - 1]  trades - 1
(F)         LINE   crates/runner/src/grid.rs:11044  [LS2 line 11034: [assert! message argument]  t.eligible.len(),]  t.eligible.len(),
(F)         LINE   crates/runner/src/grid.rs:11045  [LS2 line 11035: [assert! message argument]  t.trades.len()]  t.trades.len()
(F)         BRANCH crates/runner/src/grid_cover_tests.rs:59:13 False  [LS2: [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F)         BRANCH crates/runner/src/grid_cover_tests.rs:59:27 False  [LS2: [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F)         LINE   crates/runner/src/lib.rs:2034  [LS2 line 2028: [assert_eq! message argument]  reads.len()]  reads.len()
(F)         BRANCH crates/runner/src/lib.rs:2038:33 False  [LS2 line 2032: [assert! condition branch (failing side): the False side of a conjunction's operand, through is_some_and]  |&(name, text)| name == "validate.rs" && text.contain]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
(U)         BRANCH crates/runner/src/outcome.rs:812:19 False  [LS2: [production]  while let Some(below) = levels.last() {]  while let Some(below) = levels.last() {
(U)         BRANCH crates/runner/src/outcome.rs:962:13 False  [LS2: [production]  let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {]  let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {
(U)         LINE   crates/runner/src/outcome.rs:963  [LS2: [production]  lanes.absent(Unpriced::Refused);]  lanes.absent(Unpriced::Refused);
(U)         LINE   crates/runner/src/outcome.rs:964  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/outcome.rs:1050:13 False  [LS2: [production]  let Some((later, now)) = bars]  let Some((later, now)) = bars
(U)         LINE   crates/runner/src/outcome.rs:1055  [LS2: [production]  lanes.absent(Unpriced::Refused);]  lanes.absent(Unpriced::Refused);
(U)         LINE   crates/runner/src/outcome.rs:1056  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/outcome.rs:1170:28 False  [LS2: [production]  if let Some(Reverse(moved)) = upper.pop() {]  if let Some(Reverse(moved)) = upper.pop() {
(U)         BRANCH crates/runner/src/outcome.rs:1174:28 False  [LS2: [production]  && let Some(moved) = lower.pop()]  && let Some(moved) = lower.pop()
(U)         BRANCH crates/runner/src/outcome.rs:1193:20 False  [LS2: [production]  && let Some(slot) = out.get_mut(index)]  && let Some(slot) = out.get_mut(index)
(U)         BRANCH crates/runner/src/outcome.rs:2086:24 False  [LS2: [production]  if let Some(freed) = self.slab.get_mut(at) {]  if let Some(freed) = self.slab.get_mut(at) {
(U)         BRANCH crates/runner/src/outcome.rs:2091:24 False  [LS2: [production]  if let Some(staying) = self.slab.get_mut(at) {]  if let Some(staying) = self.slab.get_mut(at) {
(U)         BRANCH crates/runner/src/outcome.rs:2098:16 False  [LS2: [production]  if let Some(head) = self.wheel.get_mut(slot) {]  if let Some(head) = self.wheel.get_mut(slot) {
(U)         BRANCH crates/runner/src/outcome.rs:2116:16 False  [LS2: [production]  if let Some(head) = self.wheel.get_mut(slot) {]  if let Some(head) = self.wheel.get_mut(slot) {
(U)         BRANCH crates/runner/src/outcome.rs:2274:23 True  [LS2: [production]  if !paired || !forward.covers(source) {]  if !paired || !forward.covers(source) {
(F)         LINE   crates/runner/src/outcome.rs:3153  [LS2: [assert! message argument]  column.bits().len()]  column.bits().len()
(F)         LINE   crates/runner/src/outcome.rs:3243  [LS2: [assert! message argument]  minute_of(i)]  minute_of(i)
(F)         LINE   crates/runner/src/outcome.rs:3318  [LS2: [assert! message argument]  bars.len()]  bars.len()
(F)         LINE   crates/runner/src/outcome.rs:3383  [LS2: [assert_eq! message argument]  forced_minute - 1]  forced_minute - 1
(F)         LINE   crates/runner/src/outcome.rs:3413  [LS2: [assert_eq! message argument]  forced_minute - 1,]  forced_minute - 1,
(F)         LINE   crates/runner/src/outcome.rs:3414  [LS2: [assert_eq! message argument]  forced_minute - 2]  forced_minute - 2
(F)         BRANCH crates/runner/src/outcome.rs:3666:13 False  [LS2: [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),]  session.fillable(5) && !session.fillable(6),
(F)         BRANCH crates/runner/src/outcome.rs:3666:36 False  [LS2: [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),]  session.fillable(5) && !session.fillable(6),
(F)         BRANCH crates/runner/src/outcome.rs:3765:13 False  [LS2: [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F)         BRANCH crates/runner/src/outcome.rs:3765:40 False  [LS2: [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F)         BRANCH crates/runner/src/outcome.rs:4147:13 False  [LS2: [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),]  e.t != 0.0 && e.t.is_finite(),
(F)         BRANCH crates/runner/src/outcome.rs:4147:27 False  [LS2: [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),]  e.t != 0.0 && e.t.is_finite(),
(F)         BRANCH crates/runner/src/outcome.rs:4587:13 False  [LS2: [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F)         BRANCH crates/runner/src/outcome.rs:4587:39 False  [LS2: [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F)         LINE   crates/runner/src/outcome.rs:4631  [LS2: [assert_eq! message argument]  i + 1]  i + 1
(F)         BRANCH crates/runner/src/outcome.rs:4899:17 False  [LS2: [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F)         BRANCH crates/runner/src/outcome.rs:4899:36 False  [LS2: [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F)         BRANCH crates/runner/src/outcome.rs:4937:17 False  [LS2: [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         BRANCH crates/runner/src/outcome.rs:4937:30 False  [LS2: [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         BRANCH crates/runner/src/outcome.rs:4937:50 False  [LS2: [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         LINE   crates/runner/src/outcome.rs:5295  [LS2: [assert! message argument]  want[0]]  want[0]
(F)         LINE   crates/runner/src/outcome.rs:5300  [LS2: [assert! message argument]  want[1]]  want[1]
(F)         LINE   crates/runner/src/outcome.rs:5305  [LS2: [assert! message argument]  want[2]]  want[2]
(F)         BRANCH crates/runner/src/outcome.rs:5322:17 False  [LS2: [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5322:34 False  [LS2: [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5322:51 False  [LS2: [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5450:17 False  [LS2: [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F)         BRANCH crates/runner/src/outcome.rs:5450:46 False  [LS2: [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F)         BRANCH crates/runner/src/outcome.rs:5477:13 False  [LS2: [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),]  !f.was_refused(last) && !f.was_missing(last),
(F)         BRANCH crates/runner/src/outcome.rs:5477:37 False  [LS2: [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),]  !f.was_refused(last) && !f.was_missing(last),
(F)         BRANCH crates/runner/src/outcome.rs:5480:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F)         BRANCH crates/runner/src/outcome.rs:5480:47 False  [LS2: [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F)         BRANCH crates/runner/src/outcome.rs:5536:13 False  [LS2: [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F)         BRANCH crates/runner/src/outcome.rs:5536:39 False  [LS2: [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F)         BRANCH crates/runner/src/outcome.rs:5539:17 False  [LS2: [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F)         BRANCH crates/runner/src/outcome.rs:5539:42 False  [LS2: [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F)         BRANCH crates/runner/src/portfolio.rs:915:18 False  [LS2: [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([7; 32])]  } if strategy_digest == StrategyDigest::new([7; 32])
(F)         BRANCH crates/runner/src/portfolio.rs:1017:18 False  [LS2: [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([3; 32])]  } if strategy_digest == StrategyDigest::new([3; 32])
(F)         LINE   crates/runner/src/rank.rs:1264  [LS2: [assert! message argument]  grinder.payoff_bp(),]  grinder.payoff_bp(),
(F)         LINE   crates/runner/src/rank.rs:1265  [LS2: [assert! message argument]  grinder.worst_reward_risk_bp()]  grinder.worst_reward_risk_bp()
(F)         BRANCH crates/runner/src/report.rs:1508:13 False  [LS2: [assert! condition branch (failing side)]  luck > 0.0 && luck < required,]  luck > 0.0 && luck < required,
(F)         BRANCH crates/runner/src/report.rs:1508:27 False  [LS2: [assert! condition branch (failing side)]  luck > 0.0 && luck < required,]  luck > 0.0 && luck < required,
(F)         BRANCH crates/runner/src/report.rs:1521:13 False  [LS2: [assert! condition branch (failing side)]  distinct > 0 && distinct < n,]  distinct > 0 && distinct < n,
(F)         BRANCH crates/runner/src/report.rs:1521:29 False  [LS2: [assert! condition branch (failing side)]  distinct > 0 && distinct < n,]  distinct > 0 && distinct < n,
(F)         BRANCH crates/runner/src/report_cover_tests.rs:192:13 False  [LS2: [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F)         BRANCH crates/runner/src/report_cover_tests.rs:192:45 False  [LS2: [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F)         BRANCH crates/runner/src/report_cover_tests.rs:276:13 False  [LS2: [assert! condition branch (failing side)]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F)         BRANCH crates/runner/src/report_cover_tests.rs:276:38 False  [LS2: [assert! condition branch (failing side)]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F)         BRANCH crates/runner/src/resample.rs:881:21 False  [LS2: [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);]  assert!(b.open <= b.high && b.open >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:881:41 False  [LS2: [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);]  assert!(b.open <= b.high && b.open >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:882:21 False  [LS2: [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);]  assert!(b.close <= b.high && b.close >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:882:42 False  [LS2: [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);]  assert!(b.close <= b.high && b.close >= b.low);
(U)         LINE   crates/runner/src/resolved_grid_view.rs:226  [LSA: LSA (D-8563 item 4): stop_levels_ppm.len().checked_add(1) cannot fail: a Vec<Ppm> (Ppm = i64) holds at most isize::MAX / 8 elements, so its length + 1 is below usize::MAX on every target (LS2 had it [FAULT] at its line 222)]  )?;
(U)         LINE   crates/runner/src/resolved_grid_view.rs:229  [LSA: LSA (D-8563 item 4): target_levels_ppm.len().checked_add(1), the same bound (LS2's line 225)]  )?;
(U)         LINE   crates/runner/src/resolved_grid_view.rs:284  [LSA: LSA (D-8563 item 4): canonical_coordinate_ordinal's target_levels_ppm.len().checked_add(1), the same bound (LS2's line 276)]  )?;
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:895:12 True  [LS2: [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:895:51 True  [LS2: [production]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
(U)         LINE   crates/runner/src/signal_candle_stop.rs:896  [LS2: [production]  return Err(Error::Source);]  return Err(Error::Source);
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1281:20 True  [LS2: [production]  if !self]  if !self
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1287:41 False  [LS2: [production]  .is_some_and(|exit| exit.real && exit.bar == index)]  .is_some_and(|exit| exit.real && exit.bar == index)
(U)         LINE   crates/runner/src/signal_candle_stop.rs:1289  [LS2: [production]  return Ok(Err(Reason::ClosingMinute));]  return Ok(Err(Reason::ClosingMinute));
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1345:12 True  [LS2: [production]  || source.series.feed().is_empty()]  || source.series.feed().is_empty()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1346:12 True  [LS2: [production]  || source.series.commit().is_empty()]  || source.series.commit().is_empty()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1347:12 True  [LS2: [production]  || source.series.calendar_digest() == [0; 32]]  || source.series.calendar_digest() == [0; 32]
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1349:12 True  [LS2: [production]  || column.bits().len() != column.known().len()]  || column.bits().len() != column.known().len()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1350:12 True  [LS2: [production]  || column.len() != column.sources().len()]  || column.len() != column.sources().len()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1352:12 True  [LS2: [production]  || column.evaluation_spec_token().is_none()]  || column.evaluation_spec_token().is_none()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1358:12 True  [LS2: [production]  || column]  || column
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1367:12 True  [LS2: [production]  if !column.accepts(index)]  if !column.accepts(index)
(U)         LINE   crates/runner/src/signal_candle_stop.rs:1509  [LS2 line 1505: [production]  )?;]  )?;
(F)         LINE   crates/runner/src/signal_candle_stop_tests.rs:74  [LS2: [unreachable! (failing arm)]  _ => unreachable!(),]  _ => unreachable!(),
(F)         BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:13 False  [LS2: [assert! condition branch (failing side)]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F)         BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:42 False  [LS2: [assert! condition branch (failing side)]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F)         LINE   crates/runner/src/significance.rs:945  [LS2: [assert! message argument]  bonferroni_t(trials)]  bonferroni_t(trials)
(F)         BRANCH crates/runner/src/significance.rs:1323:21 False  [LS2: [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F)         BRANCH crates/runner/src/significance.rs:1323:42 False  [LS2: [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F)         LINE   crates/runner/src/significance.rs:1341  [LS2: [assert! message argument]  (approx - exact).abs()]  (approx - exact).abs()
(F)         LINE   crates/runner/src/significance.rs:1357  [LS2: [assert! message argument]  bonferroni_t(316)]  bonferroni_t(316)
(F)         BRANCH crates/runner/src/significance.rs:1385:13 False  [LS2: [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,]  bar > 7.0 && bar < 9.0,
(F)         BRANCH crates/runner/src/significance.rs:1385:26 False  [LS2: [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,]  bar > 7.0 && bar < 9.0,
(F)         BRANCH crates/runner/src/significance_cover_tests.rs:63:13 False  [LS2: [assert! condition branch (failing side)]  assert!(converged.is_finite() && converged > 0.0, "{converged}");]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F)         BRANCH crates/runner/src/significance_cover_tests.rs:63:38 False  [LS2: [assert! condition branch (failing side)]  assert!(converged.is_finite() && converged > 0.0, "{converged}");]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F)         LINE   crates/runner/src/split.rs:385  [LS2: [assert! message argument]  bar + horizon,]  bar + horizon,
(F)         BRANCH crates/runner/src/split.rs:576:21 False  [LS2: [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F)         BRANCH crates/runner/src/split.rs:576:45 False  [LS2: [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F)         BRANCH crates/runner/src/synthetic.rs:130:25 False  [LS2: [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);]  assert!(c.open <= c.high && c.open >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:130:45 False  [LS2: [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);]  assert!(c.open <= c.high && c.open >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:131:25 False  [LS2: [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);]  assert!(c.close <= c.high && c.close >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:131:46 False  [LS2: [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);]  assert!(c.close <= c.high && c.close >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:330:17 False  [LS2: [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F)         BRANCH crates/runner/src/synthetic.rs:330:27 False  [LS2: [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(U)         BRANCH crates/runner/src/topn.rs:373:16 False  [LS2: [production]  if let Some(slot) = bytes.get_mut(..16) {]  if let Some(slot) = bytes.get_mut(..16) {
(U)         BRANCH crates/runner/src/topn.rs:376:16 False  [LS2: [production]  if let Some(slot) = bytes.get_mut(16..20) {]  if let Some(slot) = bytes.get_mut(16..20) {
(U)         BRANCH crates/runner/src/topn.rs:379:16 False  [LS2: [production]  if let Some(slot) = bytes.get_mut(20..24) {]  if let Some(slot) = bytes.get_mut(20..24) {
(U)         BRANCH crates/runner/src/topn.rs:382:16 False  [LS2: [production]  if let Some(slot) = bytes.get_mut(24..28) {]  if let Some(slot) = bytes.get_mut(24..28) {
(U)         BRANCH crates/runner/src/topn.rs:389:20 False  [LS2: [production]  if let Some(slot) = bytes.get_mut(start..end) {]  if let Some(slot) = bytes.get_mut(start..end) {
(U)         BRANCH crates/runner/src/trade.rs:679:13 False  [LS2: [production]  let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {]  let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {
(U)         LINE   crates/runner/src/trade.rs:680  [LS2: [production]  return false;]  return false;
(U)         BRANCH crates/runner/src/trade.rs:682:13 False  [LS2: [production]  let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {]  let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {
(U)         LINE   crates/runner/src/trade.rs:683  [LS2: [production]  return false;]  return false;
(U)         BRANCH crates/runner/src/trade.rs:729:16 False  [LS2: [production]  if let [before, after] = pair]  if let [before, after] = pair
(U)         BRANCH crates/runner/src/trade.rs:810:8 True  [LS2: [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
(U)         BRANCH crates/runner/src/trade.rs:810:55 True  [LS2: [production]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
(U)         LINE   crates/runner/src/trade.rs:812  [LS2: [production]  return Err("expression pricing column alignment mismatch".to_owned());]  return Err("expression pricing column alignment mismatch".to_owned());
(U)         BRANCH crates/runner/src/trade.rs:1045:13 False  [LS2: [production]  let Some(entry) = signal.checked_add(step) else {]  let Some(entry) = signal.checked_add(step) else {
(U)         LINE   crates/runner/src/trade.rs:1046  [LS2: [production]  let blocked = open_until.is_some_and(|until| signal <= until);]  let blocked = open_until.is_some_and(|until| signal <= until);
(U)         LINE   crates/runner/src/trade.rs:1047  [LS2: [production]  if blocked {]  if blocked {
(U)         BRANCH crates/runner/src/trade.rs:1047:16 False  [LS2: [production]  if blocked {]  if blocked {
(U)         BRANCH crates/runner/src/trade.rs:1047:16 True  [LS2: [production]  if blocked {]  if blocked {
(U)         LINE   crates/runner/src/trade.rs:1048  [LS2: [production]  out.while_open = out.while_open.saturating_add(1);]  out.while_open = out.while_open.saturating_add(1);
(U)         LINE   crates/runner/src/trade.rs:1049  [LS2: [production]  } else {]  } else {
(U)         LINE   crates/runner/src/trade.rs:1050  [LS2: [production]  out.too_late = out.too_late.saturating_add(1);]  out.too_late = out.too_late.saturating_add(1);
(U)         LINE   crates/runner/src/trade.rs:1051  [LS2: [production]  }]  }
(U)         LINE   crates/runner/src/trade.rs:1052  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/trade.rs:1159:16 True  [LS2: [production]  if blocked {]  if blocked {
(U)         LINE   crates/runner/src/trade.rs:1160  [LS2: [production]  out.while_open = out.while_open.saturating_add(1);]  out.while_open = out.while_open.saturating_add(1);
(U)         BRANCH crates/runner/src/trade.rs:1352:8 True  [LS2: [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
(U)         BRANCH crates/runner/src/trade.rs:1352:37 True  [LS2: [production]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
(U)         LINE   crates/runner/src/trade.rs:1353  [LS2: [production]  return None;]  return None;
(F)         BRANCH crates/runner/src/trade.rs:1692:25 False  [LS2: [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:1692:48 False  [LS2: [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:1878:17 False  [LS2: [let-else whose block is only unreachable!]  let [previous, next] = pair else {]  let [previous, next] = pair else {
(F)         LINE   crates/runner/src/trade.rs:1879  [LS2: [unreachable! (failing arm)]  unreachable!("windows(2) always yields exactly two trades")]  unreachable!("windows(2) always yields exactly two trades")
(F)         BRANCH crates/runner/src/trade.rs:2084:21 False  [LS2: [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:2084:48 False  [LS2: [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F)         LINE   crates/runner/src/trade.rs:2234  [LS2: [assert! message argument]  t.count(),]  t.count(),
(F)         LINE   crates/runner/src/trade.rs:2378  [LS2: [assert! message argument]  ist_minute_of(exit_ts)]  ist_minute_of(exit_ts)
(F)         BRANCH crates/runner/src/trade.rs:2590:17 False  [LS2: [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");]  assert!(saw_refused && saw_missing, "both holes were located");
(F)         BRANCH crates/runner/src/trade.rs:2590:32 False  [LS2: [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");]  assert!(saw_refused && saw_missing, "both holes were located");
(F)         BRANCH crates/runner/src/trade.rs:2704:13 False  [LS2: [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),]  !path.priceable && path.first_refused == Some(refused),
(F)         BRANCH crates/runner/src/trade.rs:2704:32 False  [LS2: [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),]  !path.priceable && path.first_refused == Some(refused),
(F)         LINE   crates/runner/src/trade.rs:3029  [LS2: [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  matches!(]  matches!(
(F)         LINE   crates/runner/src/trade.rs:3138  [LS2: [assert! message argument]  body.len()]  body.len()
(F)         BRANCH crates/runner/src/trade.rs:3212:13 False  [LS2: [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F)         BRANCH crates/runner/src/trade.rs:3212:34 False  [LS2: [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F)         BRANCH crates/runner/src/trade.rs:3236:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F)         BRANCH crates/runner/src/trade.rs:3236:44 False  [LS2: [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F)         BRANCH crates/runner/src/trade.rs:3242:13 False  [LS2: [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F)         BRANCH crates/runner/src/trade.rs:3242:42 False  [LS2: [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F)         BRANCH crates/runner/src/trade.rs:3539:17 False  [LS2: [assert! condition branch (failing side)]  assert!(column.accepts(exit) && column.accepts(exit + 1));]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F)         BRANCH crates/runner/src/trade.rs:3539:41 False  [LS2: [assert! condition branch (failing side)]  assert!(column.accepts(exit) && column.accepts(exit + 1));]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F)         BRANCH crates/runner/src/trade.rs:3593:17 False  [LS2: [assert! condition branch (failing side)]  assert!(column.accepts(entry) && column.accepts(exit));]  assert!(column.accepts(entry) && column.accepts(exit));
(F)         BRANCH crates/runner/src/trade.rs:3593:42 False  [LS2: [assert! condition branch (failing side)]  assert!(column.accepts(entry) && column.accepts(exit));]  assert!(column.accepts(entry) && column.accepts(exit));
(F)         BRANCH crates/runner/src/trade.rs:3674:17 False  [LS2: [assert! condition branch (failing side)]  assert!(column.sources().contains(&late) && column.accepts(midnight));]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F)         BRANCH crates/runner/src/trade.rs:3674:53 False  [LS2: [assert! condition branch (failing side)]  assert!(column.sources().contains(&late) && column.accepts(midnight));]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F)         BRANCH crates/runner/src/trade.rs:3733:17 False  [LS2: [assert! condition branch (failing side)]  assert!(walked.signals > 0 && walked.reconciles());]  assert!(walked.signals > 0 && walked.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:3733:39 False  [LS2: [assert! condition branch (failing side)]  assert!(walked.signals > 0 && walked.reconciles());]  assert!(walked.signals > 0 && walked.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:3735:17 False  [LS2: [assert! condition branch (failing side)]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F)         BRANCH crates/runner/src/trade.rs:3735:45 False  [LS2: [assert! condition branch (failing side)]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F)         BRANCH crates/runner/src/trade.rs:3801:17 False  [LS2: [assert! condition branch (failing side)]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F)         BRANCH crates/runner/src/trade.rs:3801:42 False  [LS2: [assert! condition branch (failing side)]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F)         BRANCH crates/runner/src/trade_clock_tests.rs:38:13 False  [LS2: [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F)         BRANCH crates/runner/src/trade_clock_tests.rs:38:43 False  [LS2: [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(U)         LINE   crates/runner/src/validate.rs:2229  [LS2: [production]  )?;]  )?;
(U)         LINE   crates/runner/src/validate.rs:2286  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2392:8 True  [LS2: [production]  if folds.is_empty() || folds.len() != validated.folds.len() {]  if folds.is_empty() || folds.len() != validated.folds.len() {
(U)         BRANCH crates/runner/src/validate.rs:2392:28 True  [LS2: [production]  if folds.is_empty() || folds.len() != validated.folds.len() {]  if folds.is_empty() || folds.len() != validated.folds.len() {
(U)         LINE   crates/runner/src/validate.rs:2393  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::Empty);]  return Err(AnchoredSearchValidationRefusalV4::Empty);
(U)         BRANCH crates/runner/src/validate.rs:2458:12 True  [LS2: [production]  || column.bits().len() != column.sources().len()]  || column.bits().len() != column.sources().len()
(U)         BRANCH crates/runner/src/validate.rs:2460:12 True  [LS2: [production]  || census.swept != stable_u64_v4(column.bits().len())?]  || census.swept != stable_u64_v4(column.bits().len())?
(U)         BRANCH crates/runner/src/validate.rs:2461:12 True  [LS2: [production]  || !census.reconciles()]  || !census.reconciles()
(U)         BRANCH crates/runner/src/validate.rs:2462:12 True  [LS2: [production]  || !column.acceptance_covers(signal.len())]  || !column.acceptance_covers(signal.len())
(U)         BRANCH crates/runner/src/validate.rs:2463:12 True  [LS2: [production]  || column.acceptance_census() != census]  || column.acceptance_census() != census
(U)         BRANCH crates/runner/src/validate.rs:2464:12 True  [LS2: [production]  || column.evaluation_spec_token().is_none()]  || column.evaluation_spec_token().is_none()
(U)         BRANCH crates/runner/src/validate.rs:2465:12 True  [LS2: [production]  || column.collided() != 0]  || column.collided() != 0
(U)         BRANCH crates/runner/src/validate.rs:2466:12 True  [LS2: [production]  || column.first_swept() != column.sources().first().copied()]  || column.first_swept() != column.sources().first().copied()
(U)         BRANCH crates/runner/src/validate.rs:2467:12 True  [LS2: [production]  || column]  || column
(U)         BRANCH crates/runner/src/validate.rs:2472:12 True  [LS2: [production]  || !column]  || !column
(U)         BRANCH crates/runner/src/validate.rs:2574:12 True  [LS2: [production]  if fold.train.0.start != 0]  if fold.train.0.start != 0
(U)         BRANCH crates/runner/src/validate.rs:2575:16 True  [LS2: [production]  || !fold.train.1.is_empty()]  || !fold.train.1.is_empty()
(U)         BRANCH crates/runner/src/validate.rs:2576:16 True  [LS2: [production]  || fold.train.0.is_empty()]  || fold.train.0.is_empty()
(U)         LINE   crates/runner/src/validate.rs:2600  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2745:16 True  [LS2: [production]  if evaluated_cells != resolved.cell_count() {]  if evaluated_cells != resolved.cell_count() {
(U)         LINE   crates/runner/src/validate.rs:2746  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:2747  [LS2: [production]  "evaluated cell population differs from its resolution",]  "evaluated cell population differs from its resolution",
(U)         LINE   crates/runner/src/validate.rs:2748  [LS2: [production]  ));]  ));
(U)         LINE   crates/runner/src/validate.rs:2788  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2808:12 True  [LS2: [production]  if population_cells != expected_population {]  if population_cells != expected_population {
(U)         LINE   crates/runner/src/validate.rs:2809  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:2810  [LS2: [production]  "mask-by-side population is incomplete",]  "mask-by-side population is incomplete",
(U)         LINE   crates/runner/src/validate.rs:2811  [LS2: [production]  ));]  ));
(U)         LINE   crates/runner/src/validate.rs:2863  [LS2: [production]  )?;]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2951:12 True  [LS2: [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
(U)         BRANCH crates/runner/src/validate.rs:2951:55 True  [LS2: [production]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
(U)         LINE   crates/runner/src/validate.rs:2952  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
(U)         BRANCH crates/runner/src/validate.rs:3103:17 False  [LS2: [production]  let Some(next) = ordinal.checked_add(1) else {]  let Some(next) = ordinal.checked_add(1) else {
(U)         LINE   crates/runner/src/validate.rs:3104  [LS2: [production]  overflowed = true;]  overflowed = true;
(U)         LINE   crates/runner/src/validate.rs:3105  [LS2: [production]  return false;]  return false;
(U)         BRANCH crates/runner/src/validate.rs:3110:12 True  [LS2: [production]  if overflowed || !complete || ordinal != resolved.cell_count() {]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         BRANCH crates/runner/src/validate.rs:3110:26 True  [LS2: [production]  if overflowed || !complete || ordinal != resolved.cell_count() {]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         BRANCH crates/runner/src/validate.rs:3110:39 True  [LS2: [production]  if overflowed || !complete || ordinal != resolved.cell_count() {]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         LINE   crates/runner/src/validate.rs:3111  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:3112  [LS2: [production]  "canonical coordinate order is incomplete",]  "canonical coordinate order is incomplete",
(U)         LINE   crates/runner/src/validate.rs:3113  [LS2: [production]  ));]  ));
(U)         BRANCH crates/runner/src/validate.rs:3604:8 True  [LS2: [production]  if alignment.onto.len() != signal_column.sources().len() {]  if alignment.onto.len() != signal_column.sources().len() {
(U)         LINE   crates/runner/src/validate.rs:3605  [LS2: [production]  return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());]  return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());
(U)         BRANCH crates/runner/src/validate.rs:3625:8 True  [LS2: [production]  if dropped != expected_dropped {]  if dropped != expected_dropped {
(U)         LINE   crates/runner/src/validate.rs:3626  [LS2: [production]  return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());]  return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());
(U)         BRANCH crates/runner/src/validate.rs:3771:13 False  [LS2: [production]  let Some(outcome) = oos.get(ordinal) else {]  let Some(outcome) = oos.get(ordinal) else {
(U)         LINE   crates/runner/src/validate.rs:3772  [LS2: [production]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
(U)         BRANCH crates/runner/src/validate.rs:4123:9 False  [LS2: [production]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {
(U)         LINE   crates/runner/src/validate.rs:4124  [LS2: [production]  return false;]  return false;
(U)         BRANCH crates/runner/src/validate.rs:4515:13 False  [LS2: [production]  let Some(train) = bars.get(fold.train.0.clone()) else {]  let Some(train) = bars.get(fold.train.0.clone()) else {
(U)         LINE   crates/runner/src/validate.rs:4516  [LS2: [production]  continue;]  continue;
(U)         BRANCH crates/runner/src/validate.rs:4971:21 False  [LS2: [production]  let Some(chosen_side) = chosen_side else {]  let Some(chosen_side) = chosen_side else {
(U)         LINE   crates/runner/src/validate.rs:4972  [LS2: [production]  out.refused = Some(]  out.refused = Some(
(U)         LINE   crates/runner/src/validate.rs:4973  [LS2: [production]  "a walk-forward fold selected a candidate without its training side"]  "a walk-forward fold selected a candidate without its training side"
(U)         LINE   crates/runner/src/validate.rs:4974  [LS2: [production]  .to_owned(),]  .to_owned(),
(U)         LINE   crates/runner/src/validate.rs:4975  [LS2: [production]  );]  );
(U)         LINE   crates/runner/src/validate.rs:4976  [LS2: [production]  return out;]  return out;
(U)         BRANCH crates/runner/src/validate.rs:5091:24 True  [LS2: [production]  if *mask == ConditionMask::ZERO {]  if *mask == ConditionMask::ZERO {
(U)         LINE   crates/runner/src/validate.rs:5092  [LS2: [production]  0]  0
(F)         LINE   crates/runner/src/validate.rs:5602  [LS2: [unreachable!/panic! in test code (failing arm)]  unreachable!("the zero-rung door must refuse before building")]  unreachable!("the zero-rung door must refuse before building")
(F)         LINE   crates/runner/src/validate.rs:5735  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate.rs:5768  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate.rs:6027  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk finishes no fold, so the hook never speaks"),]  &|_| unreachable!("a refused walk finishes no fold, so the hook never speaks"),
(F)         LINE   crates/runner/src/validate.rs:6629  [LS2: [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  !matches!(]  !matches!(
(F)         LINE   crates/runner/src/validate.rs:6797  [LS2: [assert_eq! message argument]  f.considered.saturating_sub(f.priced)]  f.considered.saturating_sub(f.priced)
(F)         BRANCH crates/runner/src/validate.rs:6856:13 False  [LS2: [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:6856:29 False  [LS2: [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:6856:50 False  [LS2: [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:7363:19 False  [LS2: [assert! condition branch (failing side)]  }) if refused_fold == fold]  }) if refused_fold == fold
(F)         BRANCH crates/runner/src/validate.rs:7827:20 False  [LS2: [assert! condition branch (failing side)]  if fold == fold_index]  if fold == fold_index
(F)         BRANCH crates/runner/src/validate.rs:7857:24 False  [LS2: [assert! condition branch (failing side)]  if fold == fold_index]  if fold == fold_index
(F)         BRANCH crates/runner/src/validate.rs:7920:17 False  [LS2: [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F)         BRANCH crates/runner/src/validate.rs:7920:53 False  [LS2: [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F)         BRANCH crates/runner/src/validate.rs:7921:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F)         BRANCH crates/runner/src/validate.rs:7921:54 False  [LS2: [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F)         LINE   crates/runner/src/validate.rs:7941  [LS2: [assert! message argument]  body.len()]  body.len()
(F)         LINE   crates/runner/src/validate.rs:8000  [LS2: [assert! message argument]  body.len()]  body.len()
(F)         BRANCH crates/runner/src/validate.rs:8008:13 False  [LS2: [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F)         BRANCH crates/runner/src/validate.rs:8008:45 False  [LS2: [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F)         BRANCH crates/runner/src/validate.rs:8031:13 False  [LS2: [assert! condition branch (failing side)]  !inner.contains("ExecutionRunV1::new(")]  !inner.contains("ExecutionRunV1::new(")
(F)         BRANCH crates/runner/src/validate.rs:8032:20 False  [LS2: [assert! condition branch (failing side)]  && !inner.contains("ExecutionDigestsV1::of(")]  && !inner.contains("ExecutionDigestsV1::of(")
(F)         BRANCH crates/runner/src/validate.rs:8033:20 False  [LS2: [assert! condition branch (failing side)]  && inner.contains("ExecutionRunV1::with_digests(&run, &train_digests)"),]  && inner.contains("ExecutionRunV1::with_digests(&run, &train_digests)"),
(F)         BRANCH crates/runner/src/validate.rs:8059:17 False  [LS2: [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F)         BRANCH crates/runner/src/validate.rs:8059:60 False  [LS2: [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F)         BRANCH crates/runner/src/validate.rs:8061:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F)         BRANCH crates/runner/src/validate.rs:8061:56 False  [LS2: [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F)         BRANCH crates/runner/src/validate.rs:8097:13 False  [LS2: [assert! condition branch (failing side)]  !inner.contains("data_digest_with_execution(")]  !inner.contains("data_digest_with_execution(")
(F)         BRANCH crates/runner/src/validate.rs:8098:20 False  [LS2: [assert! condition branch (failing side)]  && !inner.contains("ExecutionRunV1::new("),]  && !inner.contains("ExecutionRunV1::new("),
(F)         BRANCH crates/runner/src/validate.rs:8133:13 False  [LS2: [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F)         BRANCH crates/runner/src/validate.rs:8133:58 False  [LS2: [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F)         BRANCH crates/runner/src/validate.rs:8137:13 False  [LS2: [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F)         BRANCH crates/runner/src/validate.rs:8137:46 False  [LS2: [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F)         BRANCH crates/runner/src/validate.rs:8141:13 False  [LS2: [assert! condition branch (failing side)]  !inner.contains("attest_training(")]  !inner.contains("attest_training(")
(F)         BRANCH crates/runner/src/validate.rs:8142:20 False  [LS2: [assert! condition branch (failing side)]  && !inner.contains("evaluate_training_grid_attested("),]  && !inner.contains("evaluate_training_grid_attested("),
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1459  [LS2: [unreachable!/panic! in test code (failing arm)]  unreachable!("no fold is built for a refused policy or execution path")]  unreachable!("no fold is built for a refused policy or execution path")
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1869  [LS2: [assert! message argument]  String::from_utf8_lossy(&child.stdout)]  String::from_utf8_lossy(&child.stdout)
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1894  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),]  &|_| unreachable!("a refused walk reports no fold"),
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:1920:13 False  [LS2: [assert! condition branch (failing side)]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:1920:43 False  [LS2: [assert! condition branch (failing side)]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1960  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate_cover_tests.rs:2156  [LS2: [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),]  &|_| unreachable!("a refused walk reports no fold"),
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2317:13 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2317:45 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2368:13 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.candidates.is_empty() && fold.choice.is_none());]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2368:43 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.candidates.is_empty() && fold.choice.is_none());]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2371:13 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_e]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2371:49 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_e]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2401:19 False  [LS2: [assert! condition branch (failing side): the False side of the guard of the condition's matches!]  )) if ambiguous_bars > 0]  )) if ambiguous_bars > 0
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2503:17 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty()]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2503:50 False  [LS2: [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty()]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/tests/base_cov_admission_words.rs:66:69 False  [LS2: [assert! condition branch (failing side): the False side of a disjunction's last operand]  name.starts_with("min_") || name.starts_with("max_") || name.starts_w]  name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
(F)         BRANCH crates/runner/tests/end_to_end.rs:211:9 False  [LS2: [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),]  rc.is_some() && spa.is_some(),
(F)         BRANCH crates/runner/tests/end_to_end.rs:211:25 False  [LS2: [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),]  rc.is_some() && spa.is_some(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!bounds.fillable(0) && !bounds.fillable(1));]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:40 False  [LS2: [assert! condition branch (failing side)]  assert!(!bounds.fillable(0) && !bounds.fillable(1));]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:17 False  [LS2: [assert! condition branch (failing side)]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:41 False  [LS2: [assert! condition branch (failing side)]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:17 False  [LS2: [assert! condition branch (failing side)]  trades.count() > 0 && trades.reconciles(),]  trades.count() > 0 && trades.reconciles(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:39 False  [LS2: [assert! condition branch (failing side)]  trades.count() > 0 && trades.reconciles(),]  trades.count() > 0 && trades.reconciles(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:13 False  [LS2: [assert! condition branch (failing side)]  !column.is_empty() && column.census().refused() == 0,]  !column.is_empty() && column.census().refused() == 0,
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:35 False  [LS2: [assert! condition branch (failing side)]  !column.is_empty() && column.census().refused() == 0,]  !column.is_empty() && column.census().refused() == 0,
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:25 False  [LS2: [assert! condition branch (failing side)]  assert!(trades.signals > 0 && trades.reconciles());]  assert!(trades.signals > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:47 False  [LS2: [assert! condition branch (failing side)]  assert!(trades.signals > 0 && trades.reconciles());]  assert!(trades.signals > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:202:21 False  [LS2: [assert! condition branch (failing side)]  trades.trades.is_empty()]  trades.trades.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:203:28 False  [LS2: [assert! condition branch (failing side)]  && trades.eligible.is_empty()]  && trades.eligible.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:204:28 False  [LS2: [assert! condition branch (failing side)]  && trades.occupancy.is_empty()]  && trades.occupancy.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:13 False  [LS2: [assert! condition branch (failing side)]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:42 False  [LS2: [assert! condition branch (failing side)]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:13 False  [LS2: [assert! condition branch (failing side)]  assert!(trades.count() > 0 && trades.reconciles());]  assert!(trades.count() > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:35 False  [LS2: [assert! condition branch (failing side)]  assert!(trades.count() > 0 && trades.reconciles());]  assert!(trades.count() > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:17 False  [LS2: [assert! condition branch (failing side)]  assert!(ordinary.count() > 0 && ordinary.reconciles());]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:41 False  [LS2: [assert! condition branch (failing side)]  assert!(ordinary.count() > 0 && ordinary.reconciles());]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:21 False  [LS2: [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:44 False  [LS2: [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:291:17 False  [LS2: [assert! condition branch (failing side)]  refused.trades.is_empty()]  refused.trades.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:292:24 False  [LS2: [assert! condition branch (failing side)]  && refused.eligible.is_empty()]  && refused.eligible.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:293:24 False  [LS2: [assert! condition branch (failing side)]  && refused.occupancy.is_empty()]  && refused.occupancy.is_empty()
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:21 False  [LS2: [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());]  assert!(control.count() > 0 && control.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:44 False  [LS2: [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());]  assert!(control.count() > 0 && control.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:21 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:41 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:21 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:49 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:25 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());]  assert!(mask.count() > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:45 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());]  assert!(mask.count() > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:21 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:41 False  [LS2: [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());]  assert!(mask.signals > 0 && mask.reconciles());
(F)         LINE   crates/runner/tests/hole_after_exit.rs:129  [LS2: [assert! message argument]  probes.len()]  probes.len()
(F)         LINE   crates/runner/tests/join_answer_is_unchanged.rs:140  [LS2: [assert_eq! message argument]  level.frequent.len(),]  level.frequent.len(),
(F)         LINE   crates/runner/tests/unwired_validation_record.rs:129  [LS2: [assert! message argument]  path.display()]  path.display()
(F)         BRANCH crates/store/src/catalog_tests.rs:267:9 False  [LS1: : let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {]  let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
(F)         LINE   crates/store/src/catalog_tests.rs:268  [LS1: : panic!("a dangling `bars` link is refused");]  panic!("a dangling `bars` link is refused");
(F)         BRANCH crates/store/src/catalog_tests.rs:281:9 False  [LS1: : let Err(CatalogError::BarsUnreadable { because }) = locked else {]  let Err(CatalogError::BarsUnreadable { because }) = locked else {
(F)         LINE   crates/store/src/catalog_tests.rs:282  [LS1: : panic!("an unsearchable root is refused, not empty: {locked:?}");]  panic!("an unsearchable root is refused, not empty: {locked:?}");
(F)         BRANCH crates/store/src/catalog_tests.rs:314:13 False  [LS1: : assert!(stack.is_empty() && out.held.is_empty());]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:314:33 False  [LS1: : assert!(stack.is_empty() && out.held.is_empty());]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:13 False  [LS1: : assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:32 False  [LS1: : assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:50 False  [LS1: : assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:336:13 False  [LS1: : assert!(stack.is_empty() && out.held.is_empty());]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:336:33 False  [LS1: : assert!(stack.is_empty() && out.held.is_empty());]  assert!(stack.is_empty() && out.held.is_empty());
(F)         LINE   crates/store/src/catalog_tests.rs:460  [LS1: : path.display()]  path.display()
(U)         BRANCH crates/store/src/checksum_audit.rs:288:12 True  [LS1: : || layout.header_len() != HEADER_BYTES as u64]  || layout.header_len() != HEADER_BYTES as u64
(F) bound   BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True  [LS1: : if started.elapsed() > std::time::Duration::from_secs(2) {]  if started.elapsed() > std::time::Duration::from_secs(2) {
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:153  [LS1: : child.kill().expect("bounded timeout");]  child.kill().expect("bounded timeout");
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:154  [LS1: : child.wait().expect("reap child");]  child.wait().expect("reap child");
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:155  [LS1: : return Err("strict checksum source open waited for a FIFO peer".into());]  return Err("strict checksum source open waited for a FIFO peer".into());
(F) guard   BRANCH crates/store/src/emits.rs:311:12 False  [LS1: : if let Some(byte) = region.get_mut(16_384 + 56) {]  if let Some(byte) = region.get_mut(16_384 + 56) {
(F)         LINE   crates/store/src/emits.rs:835  [LS1: : found]  found
(F)         LINE   crates/store/src/emits.rs:836  [LS1: : .records]  .records
(F)         LINE   crates/store/src/emits.rs:837  [LS1: : .iter()]  .iter()
(F)         LINE   crates/store/src/emits.rs:838  [LS1: : .map(|line| (line.target.clone(), line.message.clone()))]  .map(|line| (line.target.clone(), line.message.clone()))
(F)         LINE   crates/store/src/emits.rs:839  [LS1: : .collect::<Vec<_>>()]  .collect::<Vec<_>>()
(F)         LINE   crates/store/src/emits.rs:848  [LS1: : panic!(]  panic!(
(U)         LINE   crates/store/src/file.rs:2891  [LS1: : )?;]  )?;
(U)         LINE   crates/store/src/file.rs:3172  [LS1: : return Err(StoreError::Format {]  return Err(StoreError::Format {
(U)         LINE   crates/store/src/file.rs:3173  [LS1: : path: self.bars_path.clone(),]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file.rs:3174  [LS1: : source,]  source,
(U)         LINE   crates/store/src/file.rs:3175  [LS1: : });]  });
(U)         BRANCH crates/store/src/file.rs:3855:21 False  [LS1: : let Some(tix) = self.tix.as_ref() else {]  let Some(tix) = self.tix.as_ref() else {
(U)         LINE   crates/store/src/file.rs:3856  [LS1: : return Ok(None);]  return Ok(None);
(U)         BRANCH crates/store/src/file.rs:3869:13 False  [LS1: : let Some(tix) = self.tix.as_ref() else {]  let Some(tix) = self.tix.as_ref() else {
(U)         LINE   crates/store/src/file.rs:3870  [LS1: : return Ok(None);]  return Ok(None);
(U)         LINE   crates/store/src/file.rs:3891  [LS1: : Err(other) => Err(index_refused(&tix.path, &other)),]  Err(other) => Err(index_refused(&tix.path, &other)),
(U)         LINE   crates/store/src/file.rs:4013  [LS1: : Err(other) => return Err(index_refused(&tix.path, &other)),]  Err(other) => return Err(index_refused(&tix.path, &other)),
(U)         BRANCH crates/store/src/file.rs:4189:36 True  [LS1: : if cache.block != block || cache.n_valid != n_valid {]  if cache.block != block || cache.n_valid != n_valid {
(U)         LINE   crates/store/src/file.rs:4316  [LS1: : path: self.bars_path.clone(),]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file.rs:4317  [LS1: : source: FormatError::OffsetOverflow,]  source: FormatError::OffsetOverflow,
(U)         LINE   crates/store/src/file.rs:4318  [LS1: : })?;]  })?;
(U)         BRANCH crates/store/src/file.rs:5666:42 True  [LS1: : BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),]  BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
(F)         LINE   crates/store/src/file.rs:6858  [LS1: : parent.display()]  parent.display()
(F)         BRANCH crates/store/src/file.rs:7005:82 False  [LS1: : matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),]  matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
(F)         LINE   crates/store/src/file.rs:7062  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:7082  [LS1: : level.display()]  level.display()
(F)         LINE   crates/store/src/file.rs:7341  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:7395  [LS1: : other => panic!("the flush was refused by path: {other:?}"),]  other => panic!("the flush was refused by path: {other:?}"),
(F)         LINE   crates/store/src/file.rs:7715  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:8479  [LS1: : reads.get()]  reads.get()
(F) guard   BRANCH crates/store/src/file.rs:8801:16 True  [LS1: : if &slot[..7] != b"BRUTEXB" {]  if &slot[..7] != b"BRUTEXB" {
(F) guard   LINE   crates/store/src/file.rs:8802  [LS1: : continue;]  continue;
(F)         LINE   crates/store/src/file.rs:8818  [LS1: : other => panic!("a version-3 month without its flag must be refused, got {other:?}"),]  other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
(F)         LINE   crates/store/src/file.rs:9036  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:9045  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:9055  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:9157  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:9200  [LS1: : matches!(]  matches!(
(F) guard   BRANCH crates/store/src/file.rs:9217:16 False  [LS1: : if let Some(head) = cache.bytes.get_mut(..8) {]  if let Some(head) = cache.bytes.get_mut(..8) {
(F)         BRANCH crates/store/src/file.rs:9229:13 False  [LS1: : shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         BRANCH crates/store/src/file.rs:9229:45 False  [LS1: : shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         BRANCH crates/store/src/file.rs:9229:73 False  [LS1: : shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         LINE   crates/store/src/file.rs:9406  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:10351  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file.rs:10477  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/file.rs:10853:24 False  [LS1: : if *ts_micros == bar(10).ts_micros]  if *ts_micros == bar(10).ts_micros
(F)         BRANCH crates/store/src/file_cover_tests.rs:91:9 False  [LS1: : follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),]  follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
(F)         BRANCH crates/store/src/file_cover_tests.rs:91:65 False  [LS1: : follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),]  follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
(F)         BRANCH crates/store/src/file_cover_tests.rs:118:9 False  [LS1: : whole.starts_with(&last.took) && last.took.len() < whole.len(),]  whole.starts_with(&last.took) && last.took.len() < whole.len(),
(F)         BRANCH crates/store/src/file_cover_tests.rs:118:42 False  [LS1: : whole.starts_with(&last.took) && last.took.len() < whole.len(),]  whole.starts_with(&last.took) && last.took.len() < whole.len(),
(F)         BRANCH crates/store/src/file_fault_tests.rs:315:57 False  [LS1: : Err(StoreError::Denied { path, action }) if fired => {]  Err(StoreError::Denied { path, action }) if fired => {
(F)         LINE   crates/store/src/file_fault_tests.rs:320  [LS1: : other => panic!("read {skip}: {other:?}"),]  other => panic!("read {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:334:9 False  [LS1: : routed >= 1 && refused >= 1,]  routed >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:334:24 False  [LS1: : routed >= 1 && refused >= 1,]  routed >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:650:21 False  [LS1: : reason.starts_with(&format!("{}: ", srt.display()))]  reason.starts_with(&format!("{}: ", srt.display()))
(F)         BRANCH crates/store/src/file_fault_tests.rs:651:28 False  [LS1: : && reason.contains("refused by the file test seam"),]  && reason.contains("refused by the file test seam"),
(F)         LINE   crates/store/src/file_fault_tests.rs:657  [LS1: : Err(other) => panic!("read {skip}: {other:?}"),]  Err(other) => panic!("read {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:661:9 False  [LS1: : built >= 1 && refused >= 1,]  built >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:661:23 False  [LS1: : built >= 1 && refused >= 1,]  built >= 1 && refused >= 1,
(F)         LINE   crates/store/src/file_fault_tests.rs:696  [LS1: : other => panic!("{}: {other:?}", looked.display()),]  other => panic!("{}: {other:?}", looked.display()),
(F)         BRANCH crates/store/src/file_fault_tests.rs:755:71 False  [LS1: : matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),]  matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
(F)         BRANCH crates/store/src/file_fault_tests.rs:845:61 False  [LS1: : Err(StoreError::Denied { path, action }) if fired => {]  Err(StoreError::Denied { path, action }) if fired => {
(F)         LINE   crates/store/src/file_fault_tests.rs:869  [LS1: : other => panic!("{kind:?} measure {skip}: {other:?}"),]  other => panic!("{kind:?} measure {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:879:26 False  [LS1: : _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:879:42 False  [LS1: : _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:912:13 False  [LS1: : assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:912:54 False  [LS1: : assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:949:13 False  [LS1: : assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:949:54 False  [LS1: : assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         LINE   crates/store/src/file_index_cover_tests.rs:158  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/file_index_cover_tests.rs:574:9 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/file_index_cover_tests.rs:574:36 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/file_index_cover_tests.rs:639  [LS1: : other => panic!("the leaf is refused by the last step: {other:?}"),]  other => panic!("the leaf is refused by the last step: {other:?}"),
(U)         BRANCH crates/store/src/file_sealed.rs:135:12 True  [LS1: : if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {]  if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
(U)         LINE   crates/store/src/file_sealed.rs:136  [LS1: : return Err(StoreError::SealedRefused {]  return Err(StoreError::SealedRefused {
(U)         LINE   crates/store/src/file_sealed.rs:137  [LS1: : path: at,]  path: at,
(U)         LINE   crates/store/src/file_sealed.rs:138  [LS1: : reason: "block_rows",]  reason: "block_rows",
(U)         LINE   crates/store/src/file_sealed.rs:139  [LS1: : detail: format!(]  detail: format!(
(U)         LINE   crates/store/src/file_sealed.rs:140  [LS1: : "the head declares {} rows a frame and a handle caches at most {}",]  "the head declares {} rows a frame and a handle caches at most {}",
(U)         LINE   crates/store/src/file_sealed.rs:141  [LS1: : head.block_rows,]  head.block_rows,
(U)         LINE   crates/store/src/file_sealed.rs:142  [LS1: : CACHED_BLOCK_LEN / RECORD_LEN]  CACHED_BLOCK_LEN / RECORD_LEN
(U)         LINE   crates/store/src/file_sealed.rs:143  [LS1: : ),]  ),
(U)         LINE   crates/store/src/file_sealed.rs:144  [LS1: : });]  });
(U)         BRANCH crates/store/src/file_sealed.rs:202:36 True  [LS1: : if cache.block != block || cache.n_valid != n_valid {]  if cache.block != block || cache.n_valid != n_valid {
(U)         LINE   crates/store/src/file_sealed.rs:231  [LS1: : )?;]  )?;
(U)         LINE   crates/store/src/file_sealed.rs:236  [LS1: : path: self.bars_path.clone(),]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file_sealed.rs:237  [LS1: : source: FormatError::OffsetOverflow,]  source: FormatError::OffsetOverflow,
(U)         LINE   crates/store/src/file_sealed.rs:238  [LS1: : })?;]  })?;
(U)         BRANCH crates/store/src/file_sealed.rs:341:20 False  [LS1: : if let Some(count) = first.get_mut(at + 1) {]  if let Some(count) = first.get_mut(at + 1) {
(U)         BRANCH crates/store/src/file_sealed.rs:347:20 False  [LS1: : if let Some(start) = first.get_mut(at) {]  if let Some(start) = first.get_mut(at) {
(U)         BRANCH crates/store/src/file_sealed.rs:354:20 False  [LS1: : if let Some(slot) = next_slot.get_mut(at) {]  if let Some(slot) = next_slot.get_mut(at) {
(U)         BRANCH crates/store/src/file_sealed.rs:355:24 False  [LS1: : if let Some(place) = placed.get_mut(*slot) {]  if let Some(place) = placed.get_mut(*slot) {
(U)         LINE   crates/store/src/file_sealed.rs:359  [LS1: : }]  }
(U)         BRANCH crates/store/src/file_sealed.rs:371:21 False  [LS1: : let Some(&row) = rows.get(asked) else {]  let Some(&row) = rows.get(asked) else {
(U)         LINE   crates/store/src/file_sealed.rs:372  [LS1: : continue;]  continue;
(U)         BRANCH crates/store/src/file_sealed.rs:379:24 False  [LS1: : if let Some(slot) = out.get_mut(asked) {]  if let Some(slot) = out.get_mut(asked) {
(U)         BRANCH crates/store/src/file_sealed.rs:417:9 False  [LS1: : let Some(sealed) = file.sealed.as_ref() else {]  let Some(sealed) = file.sealed.as_ref() else {
(U)         LINE   crates/store/src/file_sealed.rs:418  [LS1: : return Ok(None);]  return Ok(None);
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:69:9 False  [LS1: : fewer.contains("rows 899 against 900") && fewer.contains("disagree"),]  fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:69:51 False  [LS1: : fewer.contains("rows 899 against 900") && fewer.contains("disagree"),]  fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:76:9 False  [LS1: : twice.contains("rows 900 against 900")]  twice.contains("rows 900 against 900")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:77:16 False  [LS1: : && twice.contains("generation 2 against 1")]  && twice.contains("generation 2 against 1")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:78:16 False  [LS1: : && twice.contains("records BLAKE3 equal: true"),]  && twice.contains("records BLAKE3 equal: true"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:92:9 False  [LS1: : shifted.contains("rows 900 against 900")]  shifted.contains("rows 900 against 900")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:93:16 False  [LS1: : && shifted.contains("generation 1 against 1")]  && shifted.contains("generation 1 against 1")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:94:16 False  [LS1: : && shifted.contains("records BLAKE3 equal: false"),]  && shifted.contains("records BLAKE3 equal: false"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:102:9 False  [LS1: : late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),]  late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:102:50 False  [LS1: : late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),]  late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:168:13 False  [LS1: : refused.contains("other.sbar") && refused.contains("disagree"),]  refused.contains("other.sbar") && refused.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:168:47 False  [LS1: : refused.contains("other.sbar") && refused.contains("disagree"),]  refused.contains("other.sbar") && refused.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:214:69 False  [LS1: : Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F)         BRANCH crates/store/src/file_sealed_tests.rs:214:83 False  [LS1: : Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:13 False  [LS1: : assert!(!bin.exists() && retired.exists() && sbar.exists());]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:30 False  [LS1: : assert!(!bin.exists() && retired.exists() && sbar.exists());]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:50 False  [LS1: : assert!(!bin.exists() && retired.exists() && sbar.exists());]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         LINE   crates/store/src/file_sealed_tests.rs:321  [LS1: : other => panic!("expected Missing naming the .bin, got {other:?}"),]  other => panic!("expected Missing naming the .bin, got {other:?}"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:407:17 False  [LS1: : !reason.is_empty() && !detail.is_empty(),]  !reason.is_empty() && !detail.is_empty(),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:407:39 False  [LS1: : !reason.is_empty() && !detail.is_empty(),]  !reason.is_empty() && !detail.is_empty(),
(F)         LINE   crates/store/src/file_sealed_tests.rs:419  [LS1: : other => panic!("a damaged frame must be refused by name, got {other:?}"),]  other => panic!("a damaged frame must be refused by name, got {other:?}"),
(F)         LINE   crates/store/src/file_sealed_tests.rs:439  [LS1: : other => panic!("a damaged head must be refused, got {other:?}"),]  other => panic!("a damaged head must be refused, got {other:?}"),
(F)         LINE   crates/store/src/file_sealed_tests.rs:798  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/file_sealed_tests.rs:803  [LS1: : crate::provenance::read(&root.0, path)]  crate::provenance::read(&root.0, path)
(F)         BRANCH crates/store/src/file_sealed_tests.rs:824:9 False  [LS1: : refused.starts_with("Err(") && refused.contains("sbar"),]  refused.starts_with("Err(") && refused.contains("sbar"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:824:40 False  [LS1: : refused.starts_with("Err(") && refused.contains("sbar"),]  refused.starts_with("Err(") && refused.contains("sbar"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:874:73 False  [LS1: : Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F)         BRANCH crates/store/src/file_sealed_tests.rs:874:92 False  [LS1: : Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F)         LINE   crates/store/src/file_sealed_tests.rs:885  [LS1: : page.len()]  page.len()
(F)         LINE   crates/store/src/file_sealed_tests.rs:898  [LS1: : page.len()]  page.len()
(F) skip    BRANCH crates/store/src/file_write_door_tests.rs:67:13 False  [LS1: : let Ok(entries) = fs::read_dir(&at) else {]  let Ok(entries) = fs::read_dir(&at) else {
(F) skip    LINE   crates/store/src/file_write_door_tests.rs:68  [LS1: : continue;]  continue;
(F)         BRANCH crates/store/src/file_write_door_tests.rs:156:13 False  [LS1: : let StoreError::Sealed {]  let StoreError::Sealed {
(F)         LINE   crates/store/src/file_write_door_tests.rs:165  [LS1: : panic!("a new bar at a sealed month is `Sealed`, got {refused}");]  panic!("a new bar at a sealed month is `Sealed`, got {refused}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:322  [LS1: : _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),]  _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:439  [LS1: : _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),]  _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:539  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:595  [LS1: : other => panic!("{n}: {other}"),]  other => panic!("{n}: {other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:605  [LS1: : other => panic!("{n}: {other}"),]  other => panic!("{n}: {other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:721  [LS1: : file.display()]  file.display()
(F)         LINE   crates/store/src/file_write_door_tests.rs:749  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:765  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:809  [LS1: : Err(other) => panic!("{other}"),]  Err(other) => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:844  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:852:13 False  [LS1: : let MonthRepair::Repaired {]  let MonthRepair::Repaired {
(F)         LINE   crates/store/src/file_write_door_tests.rs:859  [LS1: : panic!("{case}: {repaired:?}");]  panic!("{case}: {repaired:?}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:902  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:942:9 False  [LS1: : let MonthRepair::Repaired { to, skipped, .. } = repaired else {]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F)         LINE   crates/store/src/file_write_door_tests.rs:943  [LS1: : panic!("{repaired:?}");]  panic!("{repaired:?}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:983  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False  [LS1: : let MonthRepair::Repaired { to, skipped, .. } = repaired else {]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F)         LINE   crates/store/src/file_write_door_tests.rs:1027  [LS1: : panic!("{case}: {repaired:?}");]  panic!("{case}: {repaired:?}");
(F)         BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False  [LS1: : let MonthRepair::Repaired {]  let MonthRepair::Repaired {
(F)         LINE   crates/store/src/file_write_door_tests.rs:1067  [LS1: : panic!("{case}: {repaired:?}");]  panic!("{case}: {repaired:?}");
(U)         LINE   crates/store/src/flock.rs:342  [LS1: : || "an unnamed program".to_owned(),]  || "an unnamed program".to_owned(),
(U)         BRANCH crates/store/src/flock.rs:571:24 False  [LS1: : if let Ok(modified) = meta.modified() {]  if let Ok(modified) = meta.modified() {
(U)         BRANCH crates/store/src/flock.rs:574:24 False  [LS1: : if let Ok(accessed) = meta.accessed() {]  if let Ok(accessed) = meta.accessed() {
(F)         BRANCH crates/store/src/flock_cover_tests.rs:72:9 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:72:36 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:120  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/flock_cover_tests.rs:137:17 False  [LS1: : assert!(entry.writer.is_none() && entry.queue.is_none());]  assert!(entry.writer.is_none() && entry.queue.is_none());
(F)         BRANCH crates/store/src/flock_cover_tests.rs:137:43 False  [LS1: : assert!(entry.writer.is_none() && entry.queue.is_none());]  assert!(entry.writer.is_none() && entry.queue.is_none());
(F)         BRANCH crates/store/src/flock_cover_tests.rs:208:9 False  [LS1: : !second.took.is_empty() && !second.took.contains("pid 1 holds it"),]  !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:208:36 False  [LS1: : !second.took.is_empty() && !second.took.contains("pid 1 holds it"),]  !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:214:9 False  [LS1: : whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")]  whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
(F)         BRANCH crates/store/src/flock_cover_tests.rs:215:16 False  [LS1: : && whole_sentence.ends_with("; pid 1 holds it"),]  && whole_sentence.ends_with("; pid 1 holds it"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:367  [LS1: : other => panic!("the host's refusal, not a wait or a lock: {other:?}"),]  other => panic!("the host's refusal, not a wait or a lock: {other:?}"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:400  [LS1: : other => panic!("the host's refusal, not a hold: {other:?}"),]  other => panic!("the host's refusal, not a hold: {other:?}"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:101:9 False  [LS1: : let Impatient::Held {]  let Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:109  [LS1: : panic!("contention, not a host refusal: {refused}");]  panic!("contention, not a host refusal: {refused}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:123:9 False  [LS1: : took >= bound && took < bound + MAX_PAUSE + SLACK,]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:123:26 False  [LS1: : took >= bound && took < bound + MAX_PAUSE + SLACK,]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:172:9 False  [LS1: : let Err(Impatient::Host(why)) = got else {]  let Err(Impatient::Host(why)) = got else {
(F)         LINE   crates/store/src/flock_patient_tests.rs:173  [LS1: : panic!("a host refusal: {got:?}");]  panic!("a host refusal: {got:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:191:9 False  [LS1: : let Err(Impatient::Held {]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:195  [LS1: : panic!("{got:?}");]  panic!("{got:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:219:62 False  [LS1: : matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))]  matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
(F)         BRANCH crates/store/src/flock_patient_tests.rs:261:9 False  [LS1: : let Err(Impatient::Held {]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:265  [LS1: : panic!("{got:?}");]  panic!("{got:?}");
(F) fail    LINE   crates/store/src/flock_patient_tests.rs:323  [LS1: : )?;]  )?;
(F) collect LINE   crates/store/src/flock_patient_tests.rs:364  [LS1: : Err(_) => refused += 1,]  Err(_) => refused += 1,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:435:9 False  [LS1: : let Err(Impatient::Held {]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:439  [LS1: : panic!("it gave up: {refused:?}");]  panic!("it gave up: {refused:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:565:9 False  [LS1: : note.contains("thread ag06-successor") && note.contains("doing the second"),]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:565:51 False  [LS1: : note.contains("thread ag06-successor") && note.contains("doing the second"),]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F) collect LINE   crates/store/src/flock_patient_tests.rs:611  [LS1: : Err(_) => refused += 1,]  Err(_) => refused += 1,
(F)         LINE   crates/store/src/flock_patient_tests.rs:630  [LS1: : Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),]  Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
(F) collect BRANCH crates/store/src/flock_patient_tests.rs:639:16 True  [LS1: : if !named {]  if !named {
(F) collect LINE   crates/store/src/flock_patient_tests.rs:640  [LS1: : unnamed += 1;]  unnamed += 1;
(F)         BRANCH crates/store/src/flock_patient_tests.rs:683:9 False  [LS1: : text.contains("held by this process") && text.contains("doing the journal index build"),]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:683:50 False  [LS1: : text.contains("held by this process") && text.contains("doing the journal index build"),]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F)         LINE   crates/store/src/flock_patient_tests.rs:687  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/flock_tests.rs:273  [LS1: : panic!(]  panic!(
(F)         LINE   crates/store/src/flock_tests.rs:304  [LS1: : panic!("{take:?} (release: {release}) over &File kept the lock — {why}")]  panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
(F)         LINE   crates/store/src/flock_tests.rs:324  [LS1: : panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")]  panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
(F)         LINE   crates/store/src/flock_tests.rs:343  [LS1: : panic!(]  panic!(
(F)         LINE   crates/store/src/flock_tests.rs:380  [LS1: : panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")]  panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
(F) fail    LINE   crates/store/src/flock_tests.rs:759  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/flock_tests.rs:792:13 False  [LS1: : assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F)         BRANCH crates/store/src/flock_tests.rs:792:38 False  [LS1: : assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F)         BRANCH crates/store/src/flock_tests.rs:819:9 False  [LS1: : said.contains("r3-08-holder") && !said.contains("nreadable"),]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F)         BRANCH crates/store/src/flock_tests.rs:819:42 False  [LS1: : said.contains("r3-08-holder") && !said.contains("nreadable"),]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F)         BRANCH crates/store/src/flock_tests.rs:901:9 False  [LS1: : said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F)         BRANCH crates/store/src/flock_tests.rs:901:51 False  [LS1: : said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F)         LINE   crates/store/src/flock_tests.rs:984  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/flock_tests.rs:1012:9 False  [LS1: : note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F)         BRANCH crates/store/src/flock_tests.rs:1012:66 False  [LS1: : note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F)         BRANCH crates/store/src/generation_cover_tests.rs:125:55 False  [LS1: : Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"]  Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
(F)         BRANCH crates/store/src/generation_cover_tests.rs:132:55 False  [LS1: : Err(StoreError::Generation { reason, .. }) if reason.contains("past the")]  Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
(F)         BRANCH crates/store/src/generation_cover_tests.rs:138:55 True  [LS1: : Err(StoreError::Generation { reason, .. }) if reason.contains("past the")]  Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
(F)         BRANCH crates/store/src/generation_door_cover_tests.rs:99:9 False  [LS1: : let Served::Revision(record0) = served(&root.0, month()).unwrap() else {]  let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:100  [LS1: : panic!("revision 1 is served");]  panic!("revision 1 is served");
(F)         BRANCH crates/store/src/generation_door_cover_tests.rs:137:59 False  [LS1: : Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1]  Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:156  [LS1: : other => panic!("the pointer read is refused: {other:?}"),]  other => panic!("the pointer read is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:181  [LS1: : other => panic!("the linked pointer is refused: {other:?}"),]  other => panic!("the linked pointer is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:213  [LS1: : other => panic!("the directory's creation is refused: {other:?}"),]  other => panic!("the directory's creation is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:224  [LS1: : other => panic!("the pointer's open is refused: {other:?}"),]  other => panic!("the pointer's open is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:235  [LS1: : other => panic!("the pointer's lock is refused: {other:?}"),]  other => panic!("the pointer's lock is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:279  [LS1: : other => panic!("the record's write is refused: {other:?}"),]  other => panic!("the record's write is refused: {other:?}"),
(U)         BRANCH crates/store/src/month_summary.rs:939:24 False  [LS1: : if let Some(words) = entry.seconds.as_deref() {]  if let Some(words) = entry.seconds.as_deref() {
(U)         BRANCH crates/store/src/month_summary.rs:1055:12 True  [LS1: : if at.short || at.at != payload.len() {]  if at.short || at.at != payload.len() {
(U)         BRANCH crates/store/src/month_summary.rs:1085:24 False  [LS1: : if let Some(words) = entry.seconds.as_deref_mut() {]  if let Some(words) = entry.seconds.as_deref_mut() {
(U)         LINE   crates/store/src/month_summary.rs:1087  [LS1: : }]  }
(F)         LINE   crates/store/src/month_summary_decode_cover_tests.rs:171  [LS1: : other => panic!("the open is refused in the host's words: {other:?}"),]  other => panic!("the open is refused in the host's words: {other:?}"),
(F)         BRANCH crates/store/src/month_summary_tests.rs:278:52 False  [LS1: : matches!(&built, Summarized::Built(why) if why.starts_with("torn")),]  matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
(F) bound   BRANCH crates/store/src/month_summary_tests.rs:293:9 False  [LS1: : let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/month_summary_tests.rs:294  [LS1: : let _release = fs::OpenOptions::new().write(true).open(pipe);]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/month_summary_tests.rs:295  [LS1: : panic!("a read of {} waited for a writer", pipe.display());]  panic!("a read of {} waited for a writer", pipe.display());
(F)         BRANCH crates/store/src/month_summary_tests.rs:320:53 False  [LS1: : matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),]  matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:340:53 False  [LS1: : matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:355:53 False  [LS1: : matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:377:13 False  [LS1: : assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F)         BRANCH crates/store/src/month_summary_tests.rs:377:40 False  [LS1: : assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F)         BRANCH crates/store/src/month_summary_tests.rs:461:32 False  [LS1: : Refused::Shape(why) if why.contains("flag")]  Refused::Shape(why) if why.contains("flag")
(F)         BRANCH crates/store/src/month_summary_tests.rs:467:60 False  [LS1: : matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),]  matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:480:59 False  [LS1: : assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));]  assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:483:63 False  [LS1: : assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));]  assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:488:61 False  [LS1: : assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));]  assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:491:59 False  [LS1: : assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));]  assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:655:13 False  [LS1: : assert!(round.tracks_carries() && !round.wants_carries());]  assert!(round.tracks_carries() && !round.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:655:39 False  [LS1: : assert!(round.tracks_carries() && !round.wants_carries());]  assert!(round.tracks_carries() && !round.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:658:13 False  [LS1: : assert!(!seconds.tracks_carries() && !seconds.wants_carries());]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:658:42 False  [LS1: : assert!(!seconds.tracks_carries() && !seconds.wants_carries());]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:739:37 False  [LS1: : Err(Refused::Shape(why)) if why.contains("not a held minute")]  Err(Refused::Shape(why)) if why.contains("not a held minute")
(F)         BRANCH crates/store/src/month_summary_tests.rs:757:13 False  [LS1: : assert!(!read.tracks_carries() && read.wants_carries());]  assert!(!read.tracks_carries() && read.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:757:39 False  [LS1: : assert!(!read.tracks_carries() && read.wants_carries());]  assert!(!read.tracks_carries() && read.wants_carries());
(F)         LINE   crates/store/src/month_summary_tests.rs:791  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False  [LS1: : assert!(staging.is_dir() && !root.sum().exists());]  assert!(staging.is_dir() && !root.sum().exists());
(F)         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False  [LS1: : assert!(staging.is_dir() && !root.sum().exists());]  assert!(staging.is_dir() && !root.sum().exists());
(U)         BRANCH crates/store/src/provenance.rs:1665:12 False  [LS1: : if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {]  if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
(U)         BRANCH crates/store/src/provenance.rs:1673:12 False  [LS1: : if let Some(run) = bytes.get(at..at.saturating_add(N)) {]  if let Some(run) = bytes.get(at..at.saturating_add(N)) {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:167  [LS1: : other => panic!("the pointer refuses the read: {other:?}"),]  other => panic!("the pointer refuses the read: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:187  [LS1: : other => panic!("the sealed form refuses the read: {other:?}"),]  other => panic!("the sealed form refuses the read: {other:?}"),
(F)         BRANCH crates/store/src/provenance_cover_tests.rs:204:9 False  [LS1: : let Found::Present(raw) = read(&root.0, june).unwrap() else {]  let Found::Present(raw) = read(&root.0, june).unwrap() else {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:205  [LS1: : panic!("the raw sidecar holds a record");]  panic!("the raw sidecar holds a record");
(F)         LINE   crates/store/src/provenance_cover_tests.rs:219  [LS1: : other => panic!("the sealed image's record is served: {other:?}"),]  other => panic!("the sealed image's record is served: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:253  [LS1: : other => panic!("the open is refused by name: {other:?}"),]  other => panic!("the open is refused by name: {other:?}"),
(F)         BRANCH crates/store/src/provenance_cover_tests.rs:276:9 False  [LS1: : let Found::Present(sidecar) = read(&root.0, june).unwrap() else {]  let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:277  [LS1: : panic!("the batch's record is on disk");]  panic!("the batch's record is on disk");
(F)         LINE   crates/store/src/provenance_cover_tests.rs:326  [LS1: : other => panic!("the undecodable head refuses the read: {other:?}"),]  other => panic!("the undecodable head refuses the read: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:350  [LS1: : other => panic!("the lock refuses the landing: {other:?}"),]  other => panic!("the lock refuses the landing: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:395  [LS1: : other => panic!("the refused look refuses the flush: {other:?}"),]  other => panic!("the refused look refuses the flush: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:425  [LS1: : other => panic!("the header's write refuses the landing: {other:?}"),]  other => panic!("the header's write refuses the landing: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False  [LS1: : matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),]  matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:635  [LS1: : Ok(Level::Current(_)) => panic!("a torn image is never current"),]  Ok(Level::Current(_)) => panic!("a torn image is never current"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:636  [LS1: : Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),]  Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:643  [LS1: : Ok(Level::Current(_)) => panic!("a damaged header is never current"),]  Ok(Level::Current(_)) => panic!("a damaged header is never current"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:644  [LS1: : Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),]  Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:144:9 False  [LS1: : let Applied::Dropped { drop, .. } = applied else {]  let Applied::Dropped { drop, .. } = applied else {
(F)         LINE   crates/store/src/quarantine_tests.rs:145  [LS1: : panic!("expected a dropped generation, got {applied:?}");]  panic!("expected a dropped generation, got {applied:?}");
(F)         BRANCH crates/store/src/quarantine_tests.rs:174:9 False  [LS1: : let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
(F)         LINE   crates/store/src/quarantine_tests.rs:175  [LS1: : panic!("the month is labelled");]  panic!("the month is labelled");
(F)         BRANCH crates/store/src/quarantine_tests.rs:210:63 False  [LS1: : assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));]  assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
(F)         BRANCH crates/store/src/quarantine_tests.rs:263:9 False  [LS1: : let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
(F)         LINE   crates/store/src/quarantine_tests.rs:264  [LS1: : panic!("the month is answered by name");]  panic!("the month is answered by name");
(F)         BRANCH crates/store/src/quarantine_tests.rs:342:66 False  [LS1: : matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),]  matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
(F)         BRANCH crates/store/src/quarantine_tests.rs:348:77 False  [LS1: : assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));]  assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
(F)         BRANCH crates/store/src/quarantine_tests.rs:497:42 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F)         BRANCH crates/store/src/quarantine_tests.rs:497:71 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F)         BRANCH crates/store/src/quarantine_tests.rs:508:73 False  [LS1: : matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F)         BRANCH crates/store/src/quarantine_tests.rs:508:92 False  [LS1: : matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F)         LINE   crates/store/src/quarantine_tests.rs:551  [LS1: : other => panic!("expected the hold named, got {other:?}"),]  other => panic!("expected the hold named, got {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:799:9 False  [LS1: : let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {]  let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
(F)         LINE   crates/store/src/quarantine_tests.rs:800  [LS1: : panic!("the month is still answered by name");]  panic!("the month is still answered by name");
(F)         BRANCH crates/store/src/quarantine_tests.rs:855:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:855:70 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         LINE   crates/store/src/quarantine_tests.rs:892  [LS1: : other => panic!("{why}: {other:?}"),]  other => panic!("{why}: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1071:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F)         BRANCH crates/store/src/quarantine_tests.rs:1072:16 False  [LS1: : && e.bars == 6]  && e.bars == 6
(F)         BRANCH crates/store/src/quarantine_tests.rs:1073:16 False  [LS1: : && e.causes.said() == "before_listing and foreign_level"]  && e.causes.said() == "before_listing and foreign_level"
(F)         LINE   crates/store/src/quarantine_tests.rs:1092  [LS1: : other => panic!("expected the quarantine to refuse by name, got {other:?}"),]  other => panic!("expected the quarantine to refuse by name, got {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:1268  [LS1: : other => panic!("the rerun indexes the drop: {other:?}"),]  other => panic!("the rerun indexes the drop: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1327:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1327:70 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1386:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:1386:70 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:1419:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F)         BRANCH crates/store/src/quarantine_tests.rs:1466:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.bars == 3]  Ok(Probe::Quarantined(e)) if e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1533:9 False  [LS1: : refused.is_err() && words.contains("quarantined"),]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1533:29 False  [LS1: : refused.is_err() && words.contains("quarantined"),]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1584:9 False  [LS1: : refused.is_err() && words.contains("quarantined"),]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1584:29 False  [LS1: : refused.is_err() && words.contains("quarantined"),]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:38 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:67 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:82 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1747:58 False  [LS1: : matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1747:94 False  [LS1: : matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1865:77 False  [LS1: : matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
(F)         LINE   crates/store/src/quarantine_tests.rs:1995  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:42 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:71 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:86 False  [LS1: : Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2316:73 False  [LS1: : matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),
(F)         LINE   crates/store/src/quarantine_tests.rs:2348  [LS1: : other => panic!("the lock's open refuses the act: {other:?}"),]  other => panic!("the lock's open refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2365  [LS1: : other => panic!("the index's open refuses the act: {other:?}"),]  other => panic!("the index's open refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2546  [LS1: : other => panic!("the index read refuses the act: {other:?}"),]  other => panic!("the index read refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2575  [LS1: : other => panic!("the ledger read is refused: {other:?}"),]  other => panic!("the ledger read is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2609  [LS1: : other => panic!("the staged write refuses the act: {other:?}"),]  other => panic!("the staged write refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2641  [LS1: : other => panic!("the rename is refused: {other:?}"),]  other => panic!("the rename is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2680  [LS1: : other => panic!("{point}: the probe is refused: {other:?}"),]  other => panic!("{point}: the probe is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2730  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/quarantine_tests.rs:2747  [LS1: : other => panic!("a refused ask is no writer: {other:?}"),]  other => panic!("a refused ask is no writer: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2775  [LS1: : other => panic!("the lock refuses the act: {other:?}"),]  other => panic!("the lock refuses the act: {other:?}"),
(U)         LINE   crates/store/src/repair.rs:851  [LS1: : rows: header.n_valid,]  rows: header.n_valid,
(U)         LINE   crates/store/src/repair.rs:852  [LS1: : ceiling,]  ceiling,
(U)         LINE   crates/store/src/repair.rs:853  [LS1: : };]  };
(F)         BRANCH crates/store/src/repair_cover_tests.rs:62:41 False  [LS1: : text.contains("refused") || text.contains("drop names"),]  text.contains("refused") || text.contains("drop names"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:225  [LS1: : other @ MonthRestate::NothingNew { .. } => {]  other @ MonthRestate::NothingNew { .. } => {
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:226  [LS1: : panic!("the added bars are published: {other:?}")]  panic!("the added bars are published: {other:?}")
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:250  [LS1: : other @ MonthRepair::NothingNew { .. } => {]  other @ MonthRepair::NothingNew { .. } => {
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:251  [LS1: : panic!("the empty month takes the bars: {other:?}")]  panic!("the empty month takes the bars: {other:?}")
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:327  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:397  [LS1: : other => panic!("the added bar is restated: {other:?}"),]  other => panic!("the added bar is restated: {other:?}"),
(F)         BRANCH crates/store/src/repair_door_cover_tests.rs:428:9 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/repair_door_cover_tests.rs:428:36 False  [LS1: : output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:549  [LS1: : other => panic!("{operation} refuses the publication: {other:?}"),]  other => panic!("{operation} refuses the publication: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:588  [LS1: : other => panic!("the shared lock refuses the publication: {other:?}"),]  other => panic!("the shared lock refuses the publication: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:717  [LS1: : other => panic!("the reservation is refused: {other:?}"),]  other => panic!("the reservation is refused: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:752  [LS1: : other => panic!("the receipt's creation is refused: {other:?}"),]  other => panic!("the receipt's creation is refused: {other:?}"),
(F)         LINE   crates/store/src/repair_tests.rs:116  [LS1: : Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),]  Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
(F)         LINE   crates/store/src/repair_tests.rs:630  [LS1: : timeframe.as_str()]  timeframe.as_str()
(F) skip    BRANCH crates/store/src/repair_tests.rs:691:12 False  [LS1: : if let Ok(entries) = fs::read_dir(dir) {]  if let Ok(entries) = fs::read_dir(dir) {
(F) skip    LINE   crates/store/src/repair_tests.rs:700  [LS1: : }]  }
(F)         BRANCH crates/store/src/repair_tests.rs:824:94 False  [LS1: : matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),]  matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
(F)         BRANCH crates/store/src/repair_tests.rs:882:9 False  [LS1: : let super::MonthRestate::Restated {]  let super::MonthRestate::Restated {
(F)         LINE   crates/store/src/repair_tests.rs:891  [LS1: : panic!("expected a restated generation, got {done:?}");]  panic!("expected a restated generation, got {done:?}");
(U)         BRANCH crates/store/src/safe_open.rs:299:9 False  [LS1: : let Some(flags) = FLAGS else {]  let Some(flags) = FLAGS else {
(U)         LINE   crates/store/src/safe_open.rs:300  [LS1: : return Err(io(]  return Err(io(
(U)         LINE   crates/store/src/safe_open.rs:301  [LS1: : "open",]  "open",
(U)         LINE   crates/store/src/safe_open.rs:302  [LS1: : io::Error::new(]  io::Error::new(
(U)         LINE   crates/store/src/safe_open.rs:303  [LS1: : ErrorKind::Unsupported,]  ErrorKind::Unsupported,
(U)         LINE   crates/store/src/safe_open.rs:304  [LS1: : "safe reads require verified macOS or Linux x86_64/aarch64 open flags",]  "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
(U)         LINE   crates/store/src/safe_open.rs:305  [LS1: : ),]  ),
(U)         LINE   crates/store/src/safe_open.rs:306  [LS1: : ));]  ));
(F) skip    BRANCH crates/store/src/safe_open_tests.rs:66:16 True  [LS1: : if let Err(why) = std::os::unix::fs::symlink(dir, &link) {]  if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
(F) skip    LINE   crates/store/src/safe_open_tests.rs:67  [LS1: : tried.push(format!("{}: {why}", link.display()));]  tried.push(format!("{}: {why}", link.display()));
(F) skip    LINE   crates/store/src/safe_open_tests.rs:68  [LS1: : continue;]  continue;
(F) skip    LINE   crates/store/src/safe_open_tests.rs:74  [LS1: : Err(why) => tried.push(format!("{}: {why}", link.display())),]  Err(why) => tried.push(format!("{}: {why}", link.display())),
(F)         LINE   crates/store/src/safe_open_tests.rs:77  [LS1: : panic!(]  panic!(
(F)         LINE   crates/store/src/safe_open_tests.rs:79  [LS1: : path.display(),]  path.display(),
(F)         LINE   crates/store/src/safe_open_tests.rs:80  [LS1: : tried.join("; ")]  tried.join("; ")
(F)         BRANCH crates/store/src/safe_open_tests.rs:95:42 False  [LS1: : matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F)         LINE   crates/store/src/safe_open_tests.rs:103  [LS1: : found.file_type()]  found.file_type()
(F)         LINE   crates/store/src/safe_open_tests.rs:110  [LS1: : link.display()]  link.display()
(F) bound   BRANCH crates/store/src/safe_open_tests.rs:122:9 False  [LS1: : let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {]  let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/safe_open_tests.rs:123  [LS1: : let _release = fs::OpenOptions::new().write(true).open(pipe);]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/safe_open_tests.rs:124  [LS1: : panic!("an open of {} waited for a writer", pipe.display());]  panic!("an open of {} waited for a writer", pipe.display());
(F) skip    BRANCH crates/store/src/safe_open_tests.rs:203:12 False  [LS1: : if let Some(device) = a_block_device() {]  if let Some(device) = a_block_device() {
(F)         BRANCH crates/store/src/safe_open_tests.rs:309:16 False  [LS1: : if *root == shadow && *bars == shadow.join(BARS_DIR)]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F)         BRANCH crates/store/src/safe_open_tests.rs:309:35 False  [LS1: : if *root == shadow && *bars == shadow.join(BARS_DIR)]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F)         BRANCH crates/store/src/safe_open_tests.rs:310:20 False  [LS1: : && target.as_deref() == Some(real.join(BARS_DIR).as_path())),]  && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
(F)         LINE   crates/store/src/safe_open_tests.rs:412  [LS1: : Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),]  Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
(F)         LINE   crates/store/src/safe_open_tests.rs:413  [LS1: : Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),]  Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
(F)         BRANCH crates/store/src/safe_open_tests.rs:470:77 False  [LS1: : matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),]  matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
(U)         BRANCH crates/store/src/sealed.rs:1499:23 False  [LS1: : } else if let Some(range) = self.ranges.get_mut(5) {]  } else if let Some(range) = self.ranges.get_mut(5) {
(U)         BRANCH crates/store/src/sealed.rs:1525:16 False  [LS1: : if let Some(oi) = columns.get_mut(5) {]  if let Some(oi) = columns.get_mut(5) {
(U)         BRANCH crates/store/src/sealed.rs:2352:35 False  [LS1: : Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),]  Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
(F)         LINE   crates/store/src/sealed_codec_tests.rs:81  [LS1: : frame.len()]  frame.len()
(F)         LINE   crates/store/src/sealed_codec_tests.rs:282  [LS1: : frame.len()]  frame.len()
(F)         LINE   crates/store/src/sealed_codec_tests.rs:439  [LS1: : largest[rows],]  largest[rows],
(F)         LINE   crates/store/src/sealed_codec_tests.rs:440  [LS1: : max_frame_len(m)]  max_frame_len(m)
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False  [LS1: : assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False  [LS1: : assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False  [LS1: : last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False  [LS1: : last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F)         LINE   crates/store/src/sealed_tests.rs:159  [LS1: : Ok(value) => panic!("expected a refusal, got {value:?}"),]  Ok(value) => panic!("expected a refusal, got {value:?}"),
(F) guard   BRANCH crates/store/src/sealed_tests.rs:237:12 False  [LS1: : if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {]  if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
(F) fail    LINE   crates/store/src/sealed_tests.rs:499  [LS1: : Ok(())]  Ok(())
(F)         LINE   crates/store/src/sealed_tests.rs:502  [LS1: : Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),]  Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
(F)         LINE   crates/store/src/sealed_tests.rs:653  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         BRANCH crates/store/src/sealed_tests.rs:696:27 False  [LS1: : found == 0 || bars[found as usize - 1].ts_micros < ts,]  found == 0 || bars[found as usize - 1].ts_micros < ts,
(F)         LINE   crates/store/src/sealed_tests.rs:837  [LS1: : other => panic!("{field}: {other}"),]  other => panic!("{field}: {other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1171  [LS1: : packed.len(),]  packed.len(),
(F)         LINE   crates/store/src/sealed_tests.rs:1172  [LS1: : image.len()]  image.len()
(F)         LINE   crates/store/src/sealed_tests.rs:1193  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1242  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1320  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1657  [LS1: : other => panic!("{field}: {other}"),]  other => panic!("{field}: {other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1674  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1690  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1775  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1803  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         BRANCH crates/store/src/sealed_tests.rs:1817:100 False  [LS1: : matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
(F)         BRANCH crates/store/src/sealed_tests.rs:1823:105 False  [LS1: : matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F)         BRANCH crates/store/src/sealed_tests.rs:1829:105 False  [LS1: : matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
(F)         BRANCH crates/store/src/sealed_tests.rs:1837:105 False  [LS1: : matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F)         LINE   crates/store/src/sealed_tests.rs:1892  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1983  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/sealed_tests.rs:2105  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(U)         BRANCH crates/store/src/settings.rs:220:16 False  [LS1: : if let Some(slot) = self.values.get_mut(key.slot()) {]  if let Some(slot) = self.values.get_mut(key.slot()) {
(U)         BRANCH crates/store/src/settings.rs:688:30 False  [LS1: : let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {]  let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
(F)         BRANCH crates/store/src/settings_cover_tests.rs:198:57 False  [LS1: : Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")]  Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:97:9 False  [LS1: : with_mb.contains("is not a floor") && !with_mb.contains("percent"),]  with_mb.contains("is not a floor") && !with_mb.contains("percent"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:97:47 False  [LS1: : with_mb.contains("is not a floor") && !with_mb.contains("percent"),]  with_mb.contains("is not a floor") && !with_mb.contains("percent"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:128  [LS1: : other => panic!("the listing refuses the save: {other:?}"),]  other => panic!("the listing refuses the save: {other:?}"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:162:9 False  [LS1: : fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),]  fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:162:51 False  [LS1: : fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),]  fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:235:9 False  [LS1: : fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),]  fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:235:60 False  [LS1: : fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),]  fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:261  [LS1: : other => panic!("the taken name refuses the save: {other:?}"),]  other => panic!("the taken name refuses the save: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:288  [LS1: : other => panic!("the refused write refuses the save: {other:?}"),]  other => panic!("the refused write refuses the save: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:324  [LS1: : other => panic!("the refused re-list is the store's fault: {other:?}"),]  other => panic!("the refused re-list is the store's fault: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:377  [LS1: : other => panic!("the vanished temporary refuses the link: {other:?}"),]  other => panic!("the vanished temporary refuses the link: {other:?}"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:412:9 False  [LS1: : read_back.contains("did not read back")]  read_back.contains("did not read back")
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:413:16 False  [LS1: : && read_back.ends_with("it reads when read again, and the refresh did not step to it"),]  && read_back.ends_with("it reads when read again, and the refresh did not step to it"),
(F)         LINE   crates/store/src/settings_tests.rs:152  [LS1: : other => panic!("{bad:?} for {key:?}: {other:?}"),]  other => panic!("{bad:?} for {key:?}: {other:?}"),
(F)         LINE   crates/store/src/settings_tests.rs:228  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/src/settings_tests.rs:417  [LS1: : reader.faults()]  reader.faults()
(F)         LINE   crates/store/src/settings_tests.rs:583  [LS1: : reader.faults()]  reader.faults()
(F)         LINE   crates/store/src/settings_tests.rs:654  [LS1: : reader.stopped()]  reader.stopped()
(F) bound   BRANCH crates/store/src/settings_tests.rs:711:9 False  [LS1: : let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/settings_tests.rs:712  [LS1: : if let Some(pipe) = pipe {]  if let Some(pipe) = pipe {
(F) bound   BRANCH crates/store/src/settings_tests.rs:712:16 False  [LS1: : if let Some(pipe) = pipe {]  if let Some(pipe) = pipe {
(F) bound   BRANCH crates/store/src/settings_tests.rs:712:16 True  [LS1: : if let Some(pipe) = pipe {]  if let Some(pipe) = pipe {
(F) bound   LINE   crates/store/src/settings_tests.rs:713  [LS1: : let _release = std::fs::OpenOptions::new().write(true).open(pipe);]  let _release = std::fs::OpenOptions::new().write(true).open(pipe);
(F) bound   LINE   crates/store/src/settings_tests.rs:714  [LS1: : }]  }
(F)         LINE   crates/store/src/settings_tests.rs:715  [LS1: : panic!("{what}: the settings read waited instead of naming a fault");]  panic!("{what}: the settings read waited instead of naming a fault");
(F)         BRANCH crates/store/src/settings_tests.rs:789:48 False  [LS1: : matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),]  matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
(F)         BRANCH crates/store/src/settings_tests.rs:913:60 False  [LS1: : matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
(F)         BRANCH crates/store/src/settings_tests.rs:959:60 False  [LS1: : matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
(F)         LINE   crates/store/src/settings_tests.rs:993  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/src/settings_tests.rs:1100:9 False  [LS1: : read_back.contains("did not read back") && read_back.contains("generation in force is 1"),]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F)         BRANCH crates/store/src/settings_tests.rs:1100:52 False  [LS1: : read_back.contains("did not read back") && read_back.contains("generation in force is 1"),]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F) bound   BRANCH crates/store/src/settings_tests.rs:1156:20 True  [LS1: : if started.elapsed() > std::time::Duration::from_secs(10) {]  if started.elapsed() > std::time::Duration::from_secs(10) {
(F) bound   LINE   crates/store/src/settings_tests.rs:1157  [LS1: : return None;]  return None;
(F)         BRANCH crates/store/src/settings_tests.rs:1190:13 True  [LS1: : a.superseded_by == Some(3) || a.in_force == Some(2),]  a.superseded_by == Some(3) || a.in_force == Some(2),
(F)         BRANCH crates/store/src/settings_tests.rs:1190:43 False  [LS1: : a.superseded_by == Some(3) || a.in_force == Some(2),]  a.superseded_by == Some(3) || a.in_force == Some(2),
(F)         BRANCH crates/store/src/small_month_tests.rs:152:52 False  [LS1: : Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1]  Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False  [LS1: : Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")]  Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False  [LS1: : Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")]  Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False  [LS1: : Err(Why::Unreadable(why)) if why.contains("is not a regular file")]  Err(Why::Unreadable(why)) if why.contains("is not a regular file")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False  [LS1: : Err(Why::Unreadable(why)) if why.contains("is a symbolic link")]  Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False  [LS1: : Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())]  Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
(F)         BRANCH crates/store/src/sort_index_tests.rs:324:41 False  [LS1: : assert!(why.contains("kind") || why.contains("reserved"), "{why}");]  assert!(why.contains("kind") || why.contains("reserved"), "{why}");
(F)         LINE   crates/store/src/staging_cover_tests.rs:56  [LS1: : Err(other) => panic!("the removal is refused by name: {other:?}"),]  Err(other) => panic!("the removal is refused by name: {other:?}"),
(F)         LINE   crates/store/src/staging_cover_tests.rs:57  [LS1: : Ok(_) => panic!("the removal is refused by name, not staged"),]  Ok(_) => panic!("the removal is refused by name, not staged"),
(F)         BRANCH crates/store/src/staging_tests.rs:143:16 False  [LS1: : if source.kind() == ErrorKind::PermissionDenied && *p == path),]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:143:64 False  [LS1: : if source.kind() == ErrorKind::PermissionDenied && *p == path),]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:206:67 False  [LS1: : matches!(&refusal, StagingError::Contended { path: p } if *p == path),]  matches!(&refusal, StagingError::Contended { path: p } if *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:238:16 False  [LS1: : if source.kind() == ErrorKind::PermissionDenied && *p == path),]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:238:64 False  [LS1: : if source.kind() == ErrorKind::PermissionDenied && *p == path),]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:297:78 False  [LS1: : matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),]  matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:334:88 False  [LS1: : matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),]  matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
(F)         BRANCH crates/store/src/staging_tests.rs:369:77 False  [LS1: : matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),]  matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
(F)         LINE   crates/store/src/staging_tests.rs:421  [LS1: : within.display()]  within.display()
(F)         BRANCH crates/store/src/time_index.rs:921:21 False  [LS1: : assert!(entry_reads <= 1 && bar_reads <= 1);]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F)         BRANCH crates/store/src/time_index.rs:921:41 False  [LS1: : assert!(entry_reads <= 1 && bar_reads <= 1);]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F)         BRANCH crates/store/src/time_index.rs:1057:17 False  [LS1: : assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1057:38 False  [LS1: : assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1057:59 False  [LS1: : assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1058:17 False  [LS1: : assert!(!entry.occupied(1) && !entry.occupied(62));]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F)         BRANCH crates/store/src/time_index.rs:1058:39 False  [LS1: : assert!(!entry.occupied(1) && !entry.occupied(62));]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F)         BRANCH crates/store/src/time_index.rs:1105:17 False  [LS1: : assert!(!daily.exact() && g.exact());]  assert!(!daily.exact() && g.exact());
(F)         BRANCH crates/store/src/time_index.rs:1105:35 False  [LS1: : assert!(!daily.exact() && g.exact());]  assert!(!daily.exact() && g.exact());
(U)         BRANCH crates/store/src/verdict.rs:716:8 True  [LS1: : if Vendor::ALL.get(at) != Some(&feed) {]  if Vendor::ALL.get(at) != Some(&feed) {
(U)         LINE   crates/store/src/verdict.rs:717  [LS1: : return None;]  return None;
(U)         BRANCH crates/store/src/verdict.rs:2278:16 False  [LS1: : if let Some(slot) = out.first.get_mut(ordinal) {]  if let Some(slot) = out.first.get_mut(ordinal) {
(U)         BRANCH crates/store/src/verdict.rs:2312:16 False  [LS1: : if let Some(count) = out.counts.get_mut(usize::from(code.code())) {]  if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
(U)         BRANCH crates/store/src/verdict.rs:2478:16 False  [LS1: : if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {]  if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
(U)         BRANCH crates/store/src/verdict.rs:3143:15 False  [LS1: : while let Some(parent) = at.parent() {]  while let Some(parent) = at.parent() {
(U)         BRANCH crates/store/src/verdict.rs:3417:12 False  [LS1: : if let Some(entry) = days.get_mut(doy) {]  if let Some(entry) = days.get_mut(doy) {
(U)         BRANCH crates/store/src/verdict.rs:3853:24 False  [LS1: : && let Some(oldest) = self]  && let Some(oldest) = self
(U)         BRANCH crates/store/src/verdict.rs:3923:13 False  [LS1: : let Some((file, _)) = self.handle(&path, &mut cost)? else {]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:3924  [LS1: : return Err(VerdictError::ImageMissing { path });]  return Err(VerdictError::ImageMissing { path });
(U)         BRANCH crates/store/src/verdict.rs:4075:13 False  [LS1: : let Some((file, _)) = self.handle(&path, &mut cost)? else {]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:4076  [LS1: : return Err(VerdictError::ImageMissing { path });]  return Err(VerdictError::ImageMissing { path });
(U)         BRANCH crates/store/src/verdict.rs:4136:13 False  [LS1: : let Some((file, _)) = self.handle(&path, &mut cost)? else {]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:4137  [LS1: : return Err(VerdictError::ImageMissing { path });]  return Err(VerdictError::ImageMissing { path });
(F)         LINE   crates/store/src/verdict.rs:4219  [LS1: : other => panic!("not image_corrupt: {other:?}"),]  other => panic!("not image_corrupt: {other:?}"),
(F)         LINE   crates/store/src/verdict.rs:4227  [LS1: : other => panic!("not written_by_newer_build: {other:?}"),]  other => panic!("not written_by_newer_build: {other:?}"),
(U)         BRANCH crates/store/src/verdict.rs:4637:25 False  [LS1: : .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4637:37 False  [LS1: : .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4637:37 True  [LS1: : .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4974:24 False  [LS1: : .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)]  .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
(F)         LINE   crates/store/src/verdict_tests.rs:366  [LS1: : other => panic!("expected the host's refusal, got {other}"),]  other => panic!("expected the host's refusal, got {other}"),
(F) bound   BRANCH crates/store/src/verdict_tests.rs:460:9 False  [LS1: : let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/verdict_tests.rs:461  [LS1: : let _release = fs::OpenOptions::new().write(true).open(pipe);]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/verdict_tests.rs:462  [LS1: : panic!("a verdict call waited on the pipe at {}", pipe.display());]  panic!("a verdict call waited on the pipe at {}", pipe.display());
(F)         LINE   crates/store/src/verdict_tests.rs:568  [LS1: : other => panic!("expected the host's refusal, got {other}"),]  other => panic!("expected the host's refusal, got {other}"),
(F)         LINE   crates/store/src/verdict_tests.rs:813  [LS1: : other => panic!("not image_corrupt: {other:?}"),]  other => panic!("not image_corrupt: {other:?}"),
(F)         LINE   crates/store/src/verdict_tests.rs:879  [LS1: : other.dir().display()]  other.dir().display()
(F)         BRANCH crates/store/src/verdict_tests.rs:950:9 False  [LS1: : let super::Found::Record { ordinal, record } = found.found else {]  let super::Found::Record { ordinal, record } = found.found else {
(F)         LINE   crates/store/src/verdict_tests.rs:951  [LS1: : panic!("the key's record: {:?}", found.found);]  panic!("the key's record: {:?}", found.found);
(U)         BRANCH crates/store/tests/catalog.rs:68:12 False  [LS1: : if let Some(parent) = full.parent() {]  if let Some(parent) = full.parent() {
(F)         LINE   crates/store/tests/catalog.rs:313  [LS1: : Err(why) => panic!("{} could not be made: {why}", full.display()),]  Err(why) => panic!("{} could not be made: {why}", full.display()),
(F)         BRANCH crates/store/tests/catalog.rs:317:9 False  [LS1: : refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F)         BRANCH crates/store/tests/catalog.rs:317:39 False  [LS1: : refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F)         BRANCH crates/store/tests/catalog.rs:317:39 True  [LS1: : refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F) skip    BRANCH crates/store/tests/catalog.rs:814:16 True  [LS1: : if let Err(why) = std::os::unix::fs::symlink(dir, &link) {]  if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
(F) skip    LINE   crates/store/tests/catalog.rs:815  [LS1: : tried.push(format!("{}: {why}", link.display()));]  tried.push(format!("{}: {why}", link.display()));
(F) skip    LINE   crates/store/tests/catalog.rs:816  [LS1: : continue;]  continue;
(F) skip    LINE   crates/store/tests/catalog.rs:822  [LS1: : Err(why) => tried.push(format!("{}: {why}", link.display())),]  Err(why) => tried.push(format!("{}: {why}", link.display())),
(F)         LINE   crates/store/tests/catalog.rs:825  [LS1: : panic!(]  panic!(
(F)         LINE   crates/store/tests/catalog.rs:827  [LS1: : path.display(),]  path.display(),
(F)         LINE   crates/store/tests/catalog.rs:828  [LS1: : tried.join("; ")]  tried.join("; ")
(F)         BRANCH crates/store/tests/catalog.rs:844:42 False  [LS1: : matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F)         LINE   crates/store/tests/catalog.rs:852  [LS1: : found.file_type()]  found.file_type()
(F)         LINE   crates/store/tests/catalog.rs:859  [LS1: : link.display()]  link.display()
(F)         LINE   crates/store/tests/catalog.rs:875  [LS1: : path.display()]  path.display()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:288:12 True  [LS1: : if let Err(why) = Command::new("git").arg("--version").output() {]  if let Err(why) = Command::new("git").arg("--version").output() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:289  [LS1: : println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");]  println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
(F) skip    LINE   crates/store/tests/cited_commits.rs:290  [LS1: : return false;]  return false;
(F) fail    BRANCH crates/store/tests/cited_commits.rs:294:21 False  [LS1: : Err(why) if why.kind() == std::io::ErrorKind::NotFound]  Err(why) if why.kind() == std::io::ErrorKind::NotFound
(U)         BRANCH crates/store/tests/cited_commits.rs:306:12 False  [LS1: : if let Some(above) = root.parent() {]  if let Some(above) = root.parent() {
(F)         BRANCH crates/store/tests/cited_commits.rs:371:9 False  [LS1: : said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F)         BRANCH crates/store/tests/cited_commits.rs:372:16 False  [LS1: : && said.contains("not a git repository"),]  && said.contains("not a git repository"),
(F)         BRANCH crates/store/tests/cited_commits.rs:384:9 False  [LS1: : said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F)         BRANCH crates/store/tests/cited_commits.rs:385:16 False  [LS1: : && said.contains("not a git repository"),]  && said.contains("not a git repository"),
(F) collect BRANCH crates/store/tests/cited_commits.rs:567:13 False  [LS1: : let Some(citing) = cited.get(commit) else {]  let Some(citing) = cited.get(commit) else {
(F) collect LINE   crates/store/tests/cited_commits.rs:568  [LS1: : faults.push(format!(]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:571  [LS1: : continue;]  continue;
(F) collect LINE   crates/store/tests/cited_commits.rs:575  [LS1: : faults.push(format!(]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:576  [LS1: : "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",]  "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
(F) collect LINE   crates/store/tests/cited_commits.rs:577  [LS1: : at.name, at.sentence]  at.name, at.sentence
(F) collect LINE   crates/store/tests/cited_commits.rs:578  [LS1: : ));]  ));
(F) collect LINE   crates/store/tests/cited_commits.rs:579  [LS1: : }]  }
(F) collect BRANCH crates/store/tests/cited_commits.rs:601:16 True  [LS1: : if !is_ancestor_at(root, commit, main_ref) {]  if !is_ancestor_at(root, commit, main_ref) {
(F) collect LINE   crates/store/tests/cited_commits.rs:602  [LS1: : let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();]  let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
(F) collect LINE   crates/store/tests/cited_commits.rs:603  [LS1: : faults.push(format!(]  faults.push(format!(
(F) collect BRANCH crates/store/tests/cited_commits.rs:610:12 True  [LS1: : if is_ancestor_at(root, commit, main_ref) {]  if is_ancestor_at(root, commit, main_ref) {
(F) collect LINE   crates/store/tests/cited_commits.rs:611  [LS1: : faults.push(format!("{commit} is listed as {off:?}, and it is on main"));]  faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
(F) collect BRANCH crates/store/tests/cited_commits.rs:613:35 True  [LS1: : if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {]  if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
(F) collect LINE   crates/store/tests/cited_commits.rs:614  [LS1: : faults.push(format!(]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:615  [LS1: : "{commit} is listed as {off:?}, and it is an ancestor of HEAD"]  "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
(F) collect LINE   crates/store/tests/cited_commits.rs:616  [LS1: : ));]  ));
(F)         LINE   crates/store/tests/cited_commits.rs:673  [LS1: : cited.len()]  cited.len()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:676:8 False  [LS1: : if history_is_here() {]  if history_is_here() {
(F)         LINE   crates/store/tests/cited_commits.rs:709  [LS1: : String::from_utf8_lossy(&out.stderr)]  String::from_utf8_lossy(&out.stderr)
(F) skip    BRANCH crates/store/tests/cited_commits.rs:731:8 True  [LS1: : if Command::new("git").arg("--version").output().is_err() {]  if Command::new("git").arg("--version").output().is_err() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:732  [LS1: : println!("SKIPPING: git cannot run the synthetic history fixture");]  println!("SKIPPING: git cannot run the synthetic history fixture");
(F) skip    LINE   crates/store/tests/cited_commits.rs:733  [LS1: : return;]  return;
(F) skip    BRANCH crates/store/tests/cited_commits.rs:819:8 True  [LS1: : if Command::new("git").arg("--version").output().is_err() {]  if Command::new("git").arg("--version").output().is_err() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:820  [LS1: : println!("SKIPPING: git cannot run the synthetic history fixture");]  println!("SKIPPING: git cannot run the synthetic history fixture");
(F) skip    LINE   crates/store/tests/cited_commits.rs:821  [LS1: : return;]  return;
(F)         LINE   crates/store/tests/cited_commits.rs:976  [LS1: : sites.len()]  sites.len()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:978:8 True  [LS1: : if !history_is_here() {]  if !history_is_here() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:979  [LS1: : return;]  return;
(F) collect BRANCH crates/store/tests/cited_commits.rs:996:12 True  [LS1: : if !takes_a_lock(line) {]  if !takes_a_lock(line) {
(F) collect LINE   crates/store/tests/cited_commits.rs:997  [LS1: : wrong.push(format!("{module}.rs:{number}: {}", line.trim()));]  wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
(F) bound   BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False  [LS1: : while (vanished < 20 || contended < 20) && Instant::now() < deadline {]  while (vanished < 20 || contended < 20) && Instant::now() < deadline {
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False  [LS1: : vanished >= 20 && contended >= 20,]  vanished >= 20 && contended >= 20,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False  [LS1: : vanished >= 20 && contended >= 20,]  vanished >= 20 && contended >= 20,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False  [LS1: : } if source.kind() == std::io::ErrorKind::PermissionDenied]  } if source.kind() == std::io::ErrorKind::PermissionDenied
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:367  [LS1: : other => panic!("{door}: {other:?}"),]  other => panic!("{door}: {other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False  [LS1: : let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {]  let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:371  [LS1: : panic!("a moved revision is audited");]  panic!("a moved revision is audited");
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False  [LS1: : audited.starts_with("repair revision 1 of ")]  audited.starts_with("repair revision 1 of ")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False  [LS1: : && audited.ends_with(" is not the one its promotion recorded; not audited"),]  && audited.ends_with(" is not the one its promotion recorded; not audited"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:446  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1073  [LS1: : other => panic!("not image_corrupt: {other:?}"),]  other => panic!("not image_corrupt: {other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False  [LS1: : let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {]  let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1098  [LS1: : panic!("ZZB is in the day");]  panic!("ZZB is in the day");
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1171  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False  [LS1: : generation.contains("/nonexistent/cloud/2031-02.tix")]  generation.contains("/nonexistent/cloud/2031-02.tix")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False  [LS1: : && generation.contains("revision 1 is not the one its promotion recorded"),]  && generation.contains("revision 1 is not the one its promotion recorded"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1361  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1568  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False  [LS1: : if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False  [LS1: : if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False  [LS1: : if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False  [LS1: : if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False  [LS1: : if path == at && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False  [LS1: : if path == at && source.kind() == std::io::ErrorKind::PermissionDenied]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False  [LS1: : if path == &lock_path && *tries == steps + 1]  if path == &lock_path && *tries == steps + 1
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False  [LS1: : if path == &lock_path && *tries == steps + 1]  if path == &lock_path && *tries == steps + 1
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False  [LS1: : let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)]  let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1925  [LS1: : panic!("{name} is in the day");]  panic!("{name} is in the day");
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False  [LS1: : if path.parent() == Some(year.as_path())]  if path.parent() == Some(year.as_path())
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False  [LS1: : && path.extension().is_some_and(|ext| ext == "tmp")]  && path.extension().is_some_and(|ext| ext == "tmp")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False  [LS1: : && source.kind() == std::io::ErrorKind::PermissionDenied]  && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False  [LS1: : TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix]  TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False  [LS1: : if *path == bar_path && *offset == 32_768 + 56 * 4]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False  [LS1: : if *path == bar_path && *offset == 32_768 + 56 * 4]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F) guard   BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False  [LS1: : let Some(bytes) = self.bytes.get(at..at + buf.len()) else {]  let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
(F) guard   LINE   crates/store/tests/cloud_mutants_store.rs:2385  [LS1: : return Err(std::io::ErrorKind::UnexpectedEof.into());]  return Err(std::io::ErrorKind::UnexpectedEof.into());
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False  [LS1: : if *path == bar_path]  if *path == bar_path
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False  [LS1: : TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros]  TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:2876  [LS1: : writer.time_lookup()]  writer.time_lookup()
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False  [LS1: : if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False  [LS1: : if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False  [LS1: : StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)]  StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False  [LS1: : matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F) bound   BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True  [LS1: : if since.elapsed() > std::time::Duration::from_secs(5) {]  if since.elapsed() > std::time::Duration::from_secs(5) {
(F) bound   LINE   crates/store/tests/cloud_mutants_store.rs:3172  [LS1: : return Err(std::io::Error::other("flushed alone"));]  return Err(std::io::Error::other("flushed alone"));
(F) skip    LINE   crates/store/tests/cloud_mutants_store.rs:3253  [LS1: : _ => None,]  _ => None,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False  [LS1: : matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F)         BRANCH crates/store/tests/docs.rs:220:9 False  [LS1: : BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F)         BRANCH crates/store/tests/docs.rs:220:47 False  [LS1: : BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F)         BRANCH crates/store/tests/docs.rs:234:9 False  [LS1: : entry.contains(]  entry.contains(
(F)         BRANCH crates/store/tests/docs.rs:236:14 False  [LS1: : ) && entry.contains("this entry supersedes that sentence"),]  ) && entry.contains("this entry supersedes that sentence"),
(F)         BRANCH crates/store/tests/docs.rs:253:9 False  [LS1: : FILE_RS.contains("if claimed > header.n_valid {")]  FILE_RS.contains("if claimed > header.n_valid {")
(F)         BRANCH crates/store/tests/docs.rs:254:16 False  [LS1: : && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),]  && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
(F)         LINE   crates/store/tests/durability.rs:381  [LS1: : panic!(]  panic!(
(F)         BRANCH crates/store/tests/fault.rs:143:36 False  [LS1: : read.n_valid == 150 || read.n_valid == 175,]  read.n_valid == 150 || read.n_valid == 175,
(F)         BRANCH crates/store/tests/fault.rs:220:59 False  [LS1: : horizon == previous_commit.durable_through || horizon == commit.durable_through,]  horizon == previous_commit.durable_through || horizon == commit.durable_through,
(F)         LINE   crates/store/tests/fault.rs:376  [LS1: : panic!("bit {bit} of byte {byte} took the whole file down: {e}")]  panic!("bit {bit} of byte {byte} took the whole file down: {e}")
(F)         LINE   crates/store/tests/fifo.rs:108  [LS1: : at.display()]  at.display()
(F) bound   BRANCH crates/store/tests/fifo.rs:121:12 False  [LS1: : if let Ok(outcome) = recv.recv_timeout(BOUND) {]  if let Ok(outcome) = recv.recv_timeout(BOUND) {
(F) bound   LINE   crates/store/tests/fifo.rs:124  [LS1: : }]  }
(F) bound   LINE   crates/store/tests/fifo.rs:126  [LS1: : drop(fs::OpenOptions::new().write(true).open(release));]  drop(fs::OpenOptions::new().write(true).open(release));
(F) bound   LINE   crates/store/tests/fifo.rs:127  [LS1: : drop(worker.join());]  drop(worker.join());
(F)         LINE   crates/store/tests/fifo.rs:128  [LS1: : panic!(]  panic!(
(F)         LINE   crates/store/tests/fifo.rs:130  [LS1: : release.display()]  release.display()
(F)         BRANCH crates/store/tests/fifo.rs:157:13 False  [LS1: : rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F)         BRANCH crates/store/tests/fifo.rs:157:61 False  [LS1: : rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F)         BRANCH crates/store/tests/fifo.rs:162:53 False  [LS1: : fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),]  fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
(F) skip    BRANCH crates/store/tests/fifo.rs:212:8 True  [LS1: : if !null.exists() {]  if !null.exists() {
(F) skip    LINE   crates/store/tests/fifo.rs:213  [LS1: : return;]  return;
(F)         BRANCH crates/store/tests/generation.rs:153:9 False  [LS1: : let MonthRepair::Repaired {]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:165  [LS1: : panic!("{done:?}");]  panic!("{done:?}");
(F)         BRANCH crates/store/tests/generation.rs:179:57 False  [LS1: : assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F)         BRANCH crates/store/tests/generation.rs:179:76 False  [LS1: : assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F)         LINE   crates/store/tests/generation.rs:217  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:238  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:331  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/generation.rs:364:9 False  [LS1: : let MonthRepair::Repaired { to, skipped, .. } =]  let MonthRepair::Repaired { to, skipped, .. } =
(F)         LINE   crates/store/tests/generation.rs:367  [LS1: : panic!("repaired");]  panic!("repaired");
(F)         BRANCH crates/store/tests/generation.rs:384:9 False  [LS1: : let MonthRepair::Repaired {]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:388  [LS1: : panic!("{second:?}");]  panic!("{second:?}");
(F)         LINE   crates/store/tests/generation.rs:475  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:517  [LS1: : other => panic!("{other:?}"),]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/tests/generation.rs:670:13 False  [LS1: : assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:670:42 False  [LS1: : assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:670:42 True  [LS1: : assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:694:74 False  [LS1: : matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),]  matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
(F)         BRANCH crates/store/tests/generation.rs:749:9 False  [LS1: : let MonthRepair::Repaired {]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:758  [LS1: : panic!("{repaired:?}");]  panic!("{repaired:?}");
(F)         BRANCH crates/store/tests/geometry.rs:99:13 False  [LS1: : start >= block_start && end <= block_end,]  start >= block_start && end <= block_end,
(F)         BRANCH crates/store/tests/geometry.rs:99:37 False  [LS1: : start >= block_start && end <= block_end,]  start >= block_start && end <= block_end,
(F) collect BRANCH crates/store/tests/geometry.rs:134:13 True  [LS1: : start < block_start || end > block_end]  start < block_start || end > block_end
(F) fail    LINE   crates/store/tests/libm_key.rs:205  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/libm_key.rs:428:9 False  [LS1: : decision < caveat && caveat < rejected,]  decision < caveat && caveat < rejected,
(F)         BRANCH crates/store/tests/libm_key.rs:428:30 False  [LS1: : decision < caveat && caveat < rejected,]  decision < caveat && caveat < rejected,
(F)         LINE   crates/store/tests/libm_key.rs:1330  [LS1: : if mentions { "is not" } else { "is" }]  if mentions { "is not" } else { "is" }
(F)         BRANCH crates/store/tests/libm_key.rs:1330:16 False  [LS1: : if mentions { "is not" } else { "is" }]  if mentions { "is not" } else { "is" }
(F)         BRANCH crates/store/tests/libm_key.rs:1330:16 True  [LS1: : if mentions { "is not" } else { "is" }]  if mentions { "is not" } else { "is" }
(F)         LINE   crates/store/tests/misfiled.rs:184  [LS1: : other => panic!("expected Misfiled, found {other:?}"),]  other => panic!("expected Misfiled, found {other:?}"),
(F)         BRANCH crates/store/tests/misfiled.rs:217:9 False  [LS1: : said.contains("2019-11-28-1250000-CE")]  said.contains("2019-11-28-1250000-CE")
(F)         BRANCH crates/store/tests/misfiled.rs:218:16 False  [LS1: : && said.contains("its store path is not this month's")]  && said.contains("its store path is not this month's")
(F)         BRANCH crates/store/tests/misfiled.rs:219:16 False  [LS1: : && said.contains("DB-B65"),]  && said.contains("DB-B65"),
(F)         LINE   crates/store/tests/misfiled.rs:321  [LS1: : unborn.len()]  unborn.len()
(F)         LINE   crates/store/tests/month_repair.rs:156  [LS1: : other => panic!("not a repair: {other:?}"),]  other => panic!("not a repair: {other:?}"),
(F)         LINE   crates/store/tests/month_repair.rs:324  [LS1: : timeframe.as_str()]  timeframe.as_str()
(F)         BRANCH crates/store/tests/month_repair.rs:606:13 False  [LS1: : assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F)         BRANCH crates/store/tests/month_repair.rs:606:47 False  [LS1: : assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F) collect LINE   crates/store/tests/month_repair.rs:637  [LS1: : calls += 1;]  calls += 1;
(F) collect LINE   crates/store/tests/month_repair.rs:638  [LS1: : Ok(())]  Ok(())
(F) collect LINE   crates/store/tests/month_repair.rs:639  [LS1: : };]  };
(F)         BRANCH crates/store/tests/month_repair.rs:739:83 False  [LS1: : matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),]  matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
(F)         BRANCH crates/store/tests/month_repair.rs:852:9 False  [LS1: : let Ok(MonthRepair::Repaired {]  let Ok(MonthRepair::Repaired {
(F)         LINE   crates/store/tests/month_repair.rs:856  [LS1: : panic!("{outcome:?}");]  panic!("{outcome:?}");
(F)         BRANCH crates/store/tests/month_repair.rs:879:38 False  [LS1: : Served::Revision(pointer) if pointer.revision.ordinal() == 3]  Served::Revision(pointer) if pointer.revision.ordinal() == 3
(F)         LINE   crates/store/tests/month_repair.rs:918  [LS1: : left.keys()]  left.keys()
(F)         LINE   crates/store/tests/month_repair.rs:944  [LS1: : other => panic!("not a repair: {other:?}"),]  other => panic!("not a repair: {other:?}"),
(F) collect BRANCH crates/store/tests/open_flags.rs:175:16 True  [LS1: : if names_the_x86_bit(line) {]  if names_the_x86_bit(line) {
(F) collect LINE   crates/store/tests/open_flags.rs:176  [LS1: : offenders.push(format!("{}:{}", path.display(), number + 1));]  offenders.push(format!("{}:{}", path.display(), number + 1));
(F)         LINE   crates/store/tests/provenance.rs:66  [LS1: : other => panic!("expected a sidecar, found {other:?}"),]  other => panic!("expected a sidecar, found {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:161  [LS1: : Recorded::NothingAppended => panic!("expected a record"),]  Recorded::NothingAppended => panic!("expected a record"),
(F)         LINE   crates/store/tests/provenance.rs:451  [LS1: : other => panic!("expected NotTheNewestCommit, got {other:?}"),]  other => panic!("expected NotTheNewestCommit, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:464  [LS1: : other => panic!("expected NotABarPath, got {other:?}"),]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:476  [LS1: : other => panic!("expected WrongBarFile, got {other:?}"),]  other => panic!("expected WrongBarFile, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:531  [LS1: : other => panic!("expected OutOfOrder, got {other:?}"),]  other => panic!("expected OutOfOrder, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:639  [LS1: : other => panic!("expected Damaged, got {other:?}"),]  other => panic!("expected Damaged, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:698  [LS1: : result => panic!("expected a {field} mismatch, got {result:?}"),]  result => panic!("expected a {field} mismatch, got {result:?}"),
(F)         LINE   crates/store/tests/provenance.rs:746  [LS1: : other => panic!("expected PastTheBars, got {other:?}"),]  other => panic!("expected PastTheBars, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1200  [LS1: : other => panic!("expected {fault:?}, got {other:?}"),]  other => panic!("expected {fault:?}, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1308  [LS1: : other => panic!("expected TooLarge, got {other:?}"),]  other => panic!("expected TooLarge, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1409  [LS1: : other => panic!("expected Record, got {other:?}"),]  other => panic!("expected Record, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1611  [LS1: : other => panic!("expected NotABarPath, got {other:?}"),]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1617  [LS1: : other => panic!("expected NotABarPath, got {other:?}"),]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1671  [LS1: : other => panic!("expected the directory sync refused, got {other:?}"),]  other => panic!("expected the directory sync refused, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1721  [LS1: : other => panic!("expected the open refused, got {other:?}"),]  other => panic!("expected the open refused, got {other:?}"),
(F)         BRANCH crates/store/tests/repair.rs:506:13 True  [LS1: : assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F)         BRANCH crates/store/tests/repair.rs:506:41 False  [LS1: : assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False  [LS1: : let (Err(FrameError::Truncated { part }), 0) = frame_answer else {]  let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
(F)         LINE   crates/store/tests/sealed_codec_alloc.rs:115  [LS1: : panic!("a frame cut to {len} bytes left {frame_answer:?}");]  panic!("a frame cut to {len} bytes left {frame_answer:?}");
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False  [LS1: : in_a_stream > 1_000 && in_a_stream + 20 > full.len(),]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False  [LS1: : in_a_stream > 1_000 && in_a_stream + 20 > full.len(),]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F)         LINE   crates/store/tests/sealed_codec_alloc.rs:125  [LS1: : full.len()]  full.len()
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False  [LS1: : !text.contains("by its checked rows and no more")]  !text.contains("by its checked rows and no more")
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False  [LS1: : && !text.contains("checked rows times 56 bytes, once"),]  && !text.contains("checked rows times 56 bytes, once"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False  [LS1: : text.contains("the largest of twice") && text.contains("and 4"),]  text.contains("the largest of twice") && text.contains("and 4"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False  [LS1: : text.contains("the largest of twice") && text.contains("and 4"),]  text.contains("the largest of twice") && text.contains("and 4"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False  [LS1: : text.contains("capacity 0") && text.contains("cleared"),]  text.contains("capacity 0") && text.contains("cleared"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False  [LS1: : text.contains("capacity 0") && text.contains("cleared"),]  text.contains("capacity 0") && text.contains("cleared"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False  [LS1: : !text.contains("random fields cost at most 57 bytes a bar")]  !text.contains("random fields cost at most 57 bytes a bar")
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False  [LS1: : && !text.contains("at most 57 bytes a bar, 1 more than raw"),]  && !text.contains("at most 57 bytes a bar, 1 more than raw"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False  [LS1: : SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False  [LS1: : SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
(F)         BRANCH crates/store/tests/sealed_door.rs:153:17 True  [LS1: : assert!(at == rows || bars[at as usize].ts_micros >= probe);]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F)         BRANCH crates/store/tests/sealed_door.rs:153:31 False  [LS1: : assert!(at == rows || bars[at as usize].ts_micros >= probe);]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F)         BRANCH crates/store/tests/sealed_door.rs:231:9 False  [LS1: : large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
(F)         BRANCH crates/store/tests/sealed_door.rs:231:45 False  [LS1: : large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
(F)         LINE   crates/store/tests/slot_audit.rs:116  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/slot_audit.rs:130:13 False  [LS1: : let MonthVerdict::Reverted {]  let MonthVerdict::Reverted {
(F)         LINE   crates/store/tests/slot_audit.rs:137  [LS1: : panic!("v{version}: a damaged newest slot is a revert: {audit:?}");]  panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
(F)         LINE   crates/store/tests/slot_audit.rs:277  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/tests/slot_audit.rs:347  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/tests/slot_audit.rs:410  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/slot_audit.rs:484:9 False  [LS1: : let SlotState::Damaged { reason, .. } = audit.slots[1] else {]  let SlotState::Damaged { reason, .. } = audit.slots[1] else {
(F)         LINE   crates/store/tests/slot_audit.rs:485  [LS1: : panic!("damaged");]  panic!("damaged");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:95  [LS1: : raw.slot_audit()]  raw.slot_audit()
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:149  [LS1: : Ok(file) => {]  Ok(file) => {
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:150  [LS1: : assert_eq!(file.form(), Form::Sealed, "byte {at}");]  assert_eq!(file.form(), Form::Sealed, "byte {at}");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:151  [LS1: : assert_eq!(file.slot_audit(), None, "byte {at}");]  assert_eq!(file.slot_audit(), None, "byte {at}");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:152  [LS1: : assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");]  assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:153  [LS1: : if let Ok(last) = file.read_record(39) {]  if let Ok(last) = file.read_record(39) {
(U)         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False  [LS1: : if let Ok(last) = file.read_record(39) {]  if let Ok(last) = file.read_record(39) {
(U)         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True  [LS1: : if let Ok(last) = file.read_record(39) {]  if let Ok(last) = file.read_record(39) {
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:154  [LS1: : assert_eq!(last.close, 7_039, "byte {at}");]  assert_eq!(last.close, 7_039, "byte {at}");
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:155  [LS1: : }]  }
(F)         LINE   crates/store/tests/slot_damaged_log.rs:94  [LS1: : other => panic!("{key}: {other:?}"),]  other => panic!("{key}: {other:?}"),
(F)         LINE   crates/store/tests/slot_damaged_log.rs:101  [LS1: : other => panic!("{key}: {other:?}"),]  other => panic!("{key}: {other:?}"),
(F)         LINE   crates/store/tests/slot_damaged_log.rs:192  [LS1: : found]  found
(F)         LINE   crates/store/tests/slot_damaged_log.rs:193  [LS1: : .records]  .records
(F)         LINE   crates/store/tests/slot_damaged_log.rs:194  [LS1: : .iter()]  .iter()
(F)         LINE   crates/store/tests/slot_damaged_log.rs:195  [LS1: : .map(|r| (r.target.clone(), r.message.clone()))]  .map(|r| (r.target.clone(), r.message.clone()))
(F)         LINE   crates/store/tests/slot_damaged_log.rs:196  [LS1: : .collect::<Vec<_>>()]  .collect::<Vec<_>>()
(F)         BRANCH crates/store/tests/sort_index.rs:387:62 False  [LS1: : matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),]  matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
(F)         BRANCH crates/store/tests/sort_index.rs:430:56 False  [LS1: : matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),]  matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
(F)         BRANCH crates/store/tests/sort_index.rs:446:63 False  [LS1: : assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));]  assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
(F)         LINE   crates/store/tests/sort_index.rs:488  [LS1: : index.torn()]  index.torn()
(F)         BRANCH crates/store/tests/sort_index.rs:660:58 False  [LS1: : matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F)         BRANCH crates/store/tests/sort_index.rs:736:58 False  [LS1: : matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F)         LINE   crates/store/tests/sort_index.rs:761  [LS1: : column.as_str()]  column.as_str()
(F)         LINE   crates/store/tests/sort_index.rs:843  [LS1: : column.as_str()]  column.as_str()
(F)         BRANCH crates/store/tests/sort_index.rs:874:16 False  [LS1: : if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F)         BRANCH crates/store/tests/sort_index.rs:874:54 False  [LS1: : if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F)         BRANCH crates/store/tests/sort_index.rs:889:16 False  [LS1: : if reason.contains("2031-02.new.srt")),]  if reason.contains("2031-02.new.srt")),
(F) fail    BRANCH crates/store/tests/support/mod.rs:62:15 False  [LS1: : let ran = output.status.success() && stdout.contains("1 passed");]  let ran = output.status.success() && stdout.contains("1 passed");
(F)         BRANCH crates/store/tests/tail_proof.rs:144:9 False  [LS1: : found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         BRANCH crates/store/tests/tail_proof.rs:144:36 False  [LS1: : found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         BRANCH crates/store/tests/tail_proof.rs:144:60 False  [LS1: : found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         LINE   crates/store/tests/tix.rs:173  [LS1: : timeframe.as_str()]  timeframe.as_str()
(F) collect BRANCH crates/store/tests/tix.rs:605:76 True  [LS1: : && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))]  && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
(F)         BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False  [LS1: : }) if asked == SYMBOL + 1]  }) if asked == SYMBOL + 1
(F)         LINE   crates/store/tests/tix_small_month.rs:160  [LS1: : other => panic!("not committed: {other:?}"),]  other => panic!("not committed: {other:?}"),
(F)         BRANCH crates/store/tests/tix_small_month.rs:461:74 False  [LS1: : matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),]  matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
(F)         LINE   crates/store/tests/tix_small_month.rs:463  [LS1: : reader.time_lookup()]  reader.time_lookup()
(F)         LINE   crates/store/tests/tix_warn_once.rs:127  [LS1: : other => panic!("{key} is not a count: {other:?}"),]  other => panic!("{key} is not a count: {other:?}"),
(F)         LINE   crates/store/tests/tix_warn_once.rs:160  [LS1: : other => panic!("unexpected store.tix info line {other:?}: {line:?}"),]  other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
(F)         LINE   crates/store/tests/tix_warn_once.rs:162  [LS1: : other => panic!("unexpected store.tix line at {other:?}: {line:?}"),]  other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
(F)         LINE   crates/store/tests/unit.rs:110  [LS1: : input.len()]  input.len()
(F)         BRANCH crates/store/tests/unit.rs:399:13 False  [LS1: : assert!(real.ohlc_is_sane() && real.counts_are_sane());]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F)         BRANCH crates/store/tests/unit.rs:399:36 False  [LS1: : assert!(real.ohlc_is_sane() && real.counts_are_sane());]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F)         LINE   crates/store/tests/unit.rs:527  [LS1: : _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),]  _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
(F)         BRANCH crates/store/tests/unit.rs:868:13 False  [LS1: : assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(F)         BRANCH crates/store/tests/unit.rs:868:54 False  [LS1: : assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(U)         BRANCH crates/store/tests/unit.rs:908:16 False  [LS1: : if one.magic()[..7] == other.magic()[..7]]  if one.magic()[..7] == other.magic()[..7]
(F)         BRANCH crates/store/tests/unit.rs:1475:32 False  [LS1: : read == old || read == new,]  read == old || read == new,
(F)         LINE   crates/store/tests/unit.rs:1820  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/tests/unit.rs:1859  [LS1: : matches!(]  matches!(
(F)         LINE   crates/store/tests/unit.rs:1874  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/unit.rs:2047:26 False  [LS1: : .all(|c| !c.is_empty() && *c != ".." && *c != "."),]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F)         BRANCH crates/store/tests/unit.rs:2047:43 False  [LS1: : .all(|c| !c.is_empty() && *c != ".." && *c != "."),]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F)         LINE   crates/store/tests/unit.rs:2096  [LS1: : vendor.as_str(),]  vendor.as_str(),
(F)         BRANCH crates/store/tests/unit.rs:2173:13 False  [LS1: : err.to_string().contains(field) && err.to_string().contains("case"),]  err.to_string().contains(field) && err.to_string().contains("case"),
(F)         BRANCH crates/store/tests/unit.rs:2173:48 False  [LS1: : err.to_string().contains(field) && err.to_string().contains("case"),]  err.to_string().contains(field) && err.to_string().contains("case"),
(F)         LINE   crates/store/tests/unit.rs:2826  [LS1: : a.as_str(),]  a.as_str(),
(F)         LINE   crates/store/tests/unit.rs:2827  [LS1: : b.as_str()]  b.as_str()
(F)         LINE   crates/store/tests/unit.rs:2833  [LS1: : a.as_str()]  a.as_str()
(F)         LINE   crates/store/tests/unit.rs:2893  [LS1: : tf.as_str()]  tf.as_str()
(F)         LINE   crates/store/tests/verdict.rs:129  [LS1: : Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),]  Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
(F)         LINE   crates/store/tests/verdict.rs:168  [LS1: : other => panic!("100x must be refused table_too_large, got {other:?}"),]  other => panic!("100x must be refused table_too_large, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:228:13 False  [LS1: : got.cost.reads <= 5 && got.cost.opens <= 2,]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F)         BRANCH crates/store/tests/verdict.rs:228:36 False  [LS1: : got.cost.reads <= 5 && got.cost.opens <= 2,]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F)         LINE   crates/store/tests/verdict.rs:395  [LS1: : other => panic!("a 65-byte instrument key must be refused, got {other:?}"),]  other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:449:13 False  [LS1: : let Found::Record { ordinal, record } = &answer.found else {]  let Found::Record { ordinal, record } = &answer.found else {
(F)         LINE   crates/store/tests/verdict.rs:450  [LS1: : panic!("{whole} must be found");]  panic!("{whole} must be found");
(U)         BRANCH crates/store/tests/verdict.rs:584:12 False  [LS1: : if count > 2 {]  if count > 2 {
(U)         LINE   crates/store/tests/verdict.rs:588  [LS1: : }]  }
(F)         LINE   crates/store/tests/verdict.rs:728  [LS1: : other => panic!("expected image_missing, got {other:?}"),]  other => panic!("expected image_missing, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:793  [LS1: : other => panic!("expected generation_collision, got {other:?}"),]  other => panic!("expected generation_collision, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:1318  [LS1: : other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),]  other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:1821  [LS1: : other => panic!(]  other => panic!(
(F)         LINE   crates/store/tests/verdict.rs:1823  [LS1: : clean.len()]  clean.len()
(F)         LINE   crates/store/tests/verdict.rs:1856  [LS1: : other => panic!("expected record_corrupt, got {other:?}"),]  other => panic!("expected record_corrupt, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:1890:17 False  [LS1: : assert!(got.cost.opens <= 2 && got.cost.reads == 5);]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F)         BRANCH crates/store/tests/verdict.rs:1890:40 False  [LS1: : assert!(got.cost.opens <= 2 && got.cost.reads == 5);]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F)         BRANCH crates/store/tests/verdict.rs:2123:17 False  [LS1: : assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F)         BRANCH crates/store/tests/verdict.rs:2123:40 False  [LS1: : assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F)         LINE   crates/store/tests/verdict.rs:2283  [LS1: : feed.as_str(),]  feed.as_str(),
(F)         LINE   crates/store/tests/verdict.rs:2284  [LS1: : timeframe.as_str()]  timeframe.as_str()
(F)         BRANCH crates/store/tests/verdict.rs:2358:58 False  [LS1: : matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),]  matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
(F)         LINE   crates/store/tests/verdict.rs:2435  [LS1: : other => panic!("expected lock_stalled, got {other:?}"),]  other => panic!("expected lock_stalled, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:2455  [LS1: : matches!(]  matches!(
(F)         BRANCH crates/store/tests/verdict.rs:2545:21 False  [LS1: : image.extension().is_some_and(|ext| ext == "vdt")]  image.extension().is_some_and(|ext| ext == "vdt")
(F)         BRANCH crates/store/tests/verdict.rs:2546:28 False  [LS1: : && image.parent().is_some_and(|dir| dir.ends_with("2031"))]  && image.parent().is_some_and(|dir| dir.ends_with("2031"))
(F)         BRANCH crates/store/tests/verdict.rs:2547:28 False  [LS1: : && image.exists(),]  && image.exists(),
(F)         LINE   crates/store/tests/verdict.rs:2549  [LS1: : image.display()]  image.display()
(F)         LINE   crates/store/tests/verdict.rs:2554  [LS1: : other => panic!("expected index_missing, got {other}"),]  other => panic!("expected index_missing, got {other}"),
(F)         BRANCH crates/store/tests/verdict.rs:2604:13 False  [LS1: : assert!(named && (1..=2).contains(&seen), "seen {seen}");]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F)         BRANCH crates/store/tests/verdict.rs:2604:22 False  [LS1: : assert!(named && (1..=2).contains(&seen), "seen {seen}");]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F)         LINE   crates/store/tests/verdict.rs:2730  [LS1: : other => panic!("{other}"),]  other => panic!("{other}"),
(F)         BRANCH crates/store/tests/verdict.rs:2782:9 False  [LS1: : said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F)         BRANCH crates/store/tests/verdict.rs:2782:61 False  [LS1: : said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F)         LINE   crates/store/tests/write.rs:1402  [LS1: : sidecar.display()]  sidecar.display()
(F)         LINE   crates/store/tests/write.rs:1419  [LS1: : sidecar.display()]  sidecar.display()
(F)         BRANCH crates/store/tests/write.rs:1564:13 False  [LS1: : text.contains(&on_disk.display().to_string()) && text.contains("committed"),]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F)         BRANCH crates/store/tests/write.rs:1564:62 False  [LS1: : text.contains(&on_disk.display().to_string()) && text.contains("committed"),]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F)         LINE   crates/store/tests/write.rs:1619  [LS1: : panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")]  panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
(F)         LINE   crates/store/tests/write.rs:1663  [LS1: : panic!(]  panic!(
(F)         BRANCH crates/store/tests/write.rs:2103:85 False  [LS1: : Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars]  Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
(F)         LINE   crates/telemetry/src/level.rs:142  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.rank()]  level.rank()
(F)         BRANCH crates/telemetry/src/lib.rs:423:13 False  [LS4: [assert! condition branch (failing side)]  output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/telemetry/src/lib.rs:423:40 False  [LS4: [assert! condition branch (failing side)]  output.status.success() && stdout.contains("1 passed"),]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/telemetry/src/record.rs:841  [LS4: [matches! no-match arm inside assert! (failing side): the line holds only the expansion's _ => false region]  matches!(]  matches!(
(F)         BRANCH crates/telemetry/src/run_dirs.rs:234:21 False  [LS4: [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F)         BRANCH crates/telemetry/src/run_dirs.rs:234:44 False  [LS4: [assert! condition branch (failing side)]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F)         BRANCH crates/telemetry/src/run_dirs.rs:240:21 False  [LS4: [assert! condition branch (failing side)]  !other.sub.starts_with(&format!("{}/", dir.sub))]  !other.sub.starts_with(&format!("{}/", dir.sub))
(F)         BRANCH crates/telemetry/src/run_dirs.rs:241:28 False  [LS4: [assert! condition branch (failing side)]  && !dir.sub.starts_with(&format!("{}/", other.sub)),]  && !dir.sub.starts_with(&format!("{}/", other.sub)),
(F)         LINE   crates/telemetry/src/sink.rs:4688  [LS4: [panic!/unreachable! (failing arm)]  panic!(]  panic!(
(F)         LINE   crates/telemetry/src/sink.rs:4690  [LS4: [panic!/unreachable! (failing arm)]  path.display(),]  path.display(),
(F)         LINE   crates/telemetry/src/sink.rs:4691  [LS4: [panic!/unreachable! (failing arm)]  String::from_utf8_lossy(line)]  String::from_utf8_lossy(line)
(F)         BRANCH crates/telemetry/src/sink.rs:4824:21 False  [LS4: [assert! condition branch (failing side)]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F)         BRANCH crates/telemetry/src/sink.rs:4824:52 False  [LS4: [assert! condition branch (failing side)]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F)         BRANCH crates/telemetry/src/sink.rs:5024:17 False  [LS4: [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F)         BRANCH crates/telemetry/src/sink.rs:5024:65 False  [LS4: [assert! condition branch (failing side)]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F)         LINE   crates/telemetry/src/sink.rs:5106  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  written[0].at_unix_millis]  written[0].at_unix_millis
(F)         LINE   crates/telemetry/src/sink.rs:5143  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[1].seq,]  pair[1].seq,
(F)         LINE   crates/telemetry/src/sink.rs:5144  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[1].at_unix_millis,]  pair[1].at_unix_millis,
(F)         LINE   crates/telemetry/src/sink.rs:5145  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[0].seq,]  pair[0].seq,
(F)         LINE   crates/telemetry/src/sink.rs:5146  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  pair[0].at_unix_millis]  pair[0].at_unix_millis
(F)         LINE   crates/telemetry/src/sink.rs:5476  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  span * 2]  span * 2
(F)         LINE   crates/telemetry/src/sink.rs:5695  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  path.display()]  path.display()
(F)         BRANCH crates/telemetry/src/sink.rs:5712:17 False  [LS4: [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F)         BRANCH crates/telemetry/src/sink.rs:5712:40 False  [LS4: [assert! condition branch (failing side)]  assert!(!current.is_empty() && !first_rolled.is_empty());]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F)         BRANCH crates/telemetry/src/sink.rs:5757:13 False  [LS4: [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F)         BRANCH crates/telemetry/src/sink.rs:5757:30 False  [LS4: [assert! condition branch (failing side)]  after_one > 0 && after_one < MIN_FILE_BYTES,]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F)         LINE   crates/telemetry/src/sink.rs:6079  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  rolled.health()]  rolled.health()
(F)         BRANCH crates/telemetry/src/sink.rs:6621:13 False  [LS4: [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),]  why.contains("another") && why.contains(&dir.display().to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6621:40 False  [LS4: [assert! condition branch (failing side)]  why.contains("another") && why.contains(&dir.display().to_string()),]  why.contains("another") && why.contains(&dir.display().to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6687:17 False  [LS4: [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F)         BRANCH crates/telemetry/src/sink.rs:6687:54 False  [LS4: [assert! condition branch (failing side)]  assert!(text.contains("before the close") && text.contains("the successor's line"));]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F)         BRANCH crates/telemetry/src/sink.rs:6707:13 False  [LS4: [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),]  said.contains("closed") && said.contains("no space left"),
(F)         BRANCH crates/telemetry/src/sink.rs:6707:40 False  [LS4: [assert! condition branch (failing side)]  said.contains("closed") && said.contains("no space left"),]  said.contains("closed") && said.contains("no space left"),
(F)         BRANCH crates/telemetry/src/sink.rs:6749:13 False  [LS4: [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6749:53 False  [LS4: [assert! condition branch (failing side)]  named.contains("ahead of the clock") && named.contains(&future.to_string()),]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F)         LINE   crates/telemetry/src/sink.rs:7663  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("{dimension}={value}: {other:?}"),]  other => panic!("{dimension}={value}: {other:?}"),
(F)         BRANCH crates/telemetry/src/sink.rs:8209:21 False  [LS4: [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());]  assert!(!health.reaper_behind() && !health.is_loud());
(F)         BRANCH crates/telemetry/src/sink.rs:8209:48 False  [LS4: [assert! condition branch (failing side)]  assert!(!health.reaper_behind() && !health.is_loud());]  assert!(!health.reaper_behind() && !health.is_loud());
(F)         BRANCH crates/telemetry/src/sink.rs:8951:21 False  [LS4: [let-else in test whose else is a panic! (failing side)]  let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {]  let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
(F)         LINE   crates/telemetry/src/sink.rs:8952  [LS4: [panic!/unreachable! (failing arm)]  panic!("{pointer:?}");]  panic!("{pointer:?}");
(F)         LINE   crates/telemetry/src/sink.rs:10275  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("{dimension}={value}: {other:?}"),]  other => panic!("{dimension}={value}: {other:?}"),
(F)         BRANCH crates/telemetry/src/sink.rs:11095:17 False  [LS4: [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F)         BRANCH crates/telemetry/src/sink.rs:11095:57 False  [LS4: [assert! condition branch (failing side)]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F)         BRANCH crates/telemetry/src/sink.rs:11105:36 False  [LS4: [assert! condition branch (failing side)]  .is_some_and(|why| why.contains("first line does not decode")]  .is_some_and(|why| why.contains("first line does not decode")
(F)         BRANCH crates/telemetry/src/sink.rs:11160:17 False  [LS4: [assert! condition branch (failing side)]  why.contains("last 64 KiB hold no line") && !why.contains("first line")]  why.contains("last 64 KiB hold no line") && !why.contains("first line")
(F)         BRANCH crates/telemetry/src/sink.rs:11226:17 False  [LS4: [assert! condition branch (failing side)]  why.contains("first line does not decode") && !why.contains("last 64 KiB")]  why.contains("first line does not decode") && !why.contains("last 64 KiB")
(F)         BRANCH crates/telemetry/src/sink.rs:11231:17 False  [LS4: [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");]  assert!(a0 < a1 && a1 < a2, "premise");
(F)         BRANCH crates/telemetry/src/sink.rs:11231:28 False  [LS4: [assert! condition branch (failing side)]  assert!(a0 < a1 && a1 < a2, "premise");]  assert!(a0 < a1 && a1 < a2, "premise");
(F)         BRANCH crates/telemetry/src/sink.rs:11262:17 False  [LS4: [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F)         BRANCH crates/telemetry/src/sink.rs:11262:44 False  [LS4: [assert! condition branch (failing side)]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F)         BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:13 False  [LS4: [assert! condition branch (failing side)]  assert!(!parked.failed && parked.open);]  assert!(!parked.failed && parked.open);
(F)         BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:31 False  [LS4: [assert! condition branch (failing side)]  assert!(!parked.failed && parked.open);]  assert!(!parked.failed && parked.open);
(F)         LINE   crates/telemetry/src/state_events_tests.rs:85  [LS4: [panic!/unreachable! (failing arm)]  || panic!("{path} is named by a row and not read by this test"),]  || panic!("{path} is named by a row and not read by this test"),
(F)         LINE   crates/telemetry/src/state_events_tests.rs:105  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.label(),]  level.label(),
(F)         LINE   crates/telemetry/src/state_events_tests.rs:113  [LS4: [assert!/assert_eq! message argument (evaluated only when the assertion fails)]  level.label()]  level.label()
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:176:17 False  [LS4: [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:176:43 False  [LS4: [assert! condition branch (failing side)]  assert!(!row.change.is_empty() && !row.site.is_empty());]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:321:13 False  [LS4: [assert! condition branch (failing side)]  site.contains(&format!("\"{}\"", row.message))]  site.contains(&format!("\"{}\"", row.message))
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:322:20 False  [LS4: [assert! condition branch (failing side)]  && site.contains(&format!("\"{}\"", row.target)),]  && site.contains(&format!("\"{}\"", row.target)),
(F)         BRANCH crates/telemetry/src/tail.rs:1858:17 False  [LS4: [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);]  assert!(newest > 0 && newest < READ_BLOCK);
(F)         BRANCH crates/telemetry/src/tail.rs:1858:31 False  [LS4: [assert! condition branch (failing side)]  assert!(newest > 0 && newest < READ_BLOCK);]  assert!(newest > 0 && newest < READ_BLOCK);
(F)         LINE   crates/telemetry/tests/cloud_mutants_sink.rs:263  [LS4: [panic!/unreachable! (failing arm)]  other => panic!("a directory is not a rolled-off line: {other:?}"),]  other => panic!("a directory is not a rolled-off line: {other:?}"),
(F)         BRANCH crates/vocab/src/expression_search.rs:506:13 False  [LS3: compared > 100_000 && admitted > 1_000,]  compared > 100_000 && admitted > 1_000,
(F)         BRANCH crates/vocab/src/expression_search.rs:506:35 False  [LS3: compared > 100_000 && admitted > 1_000,]  compared > 100_000 && admitted > 1_000,
(F)         BRANCH crates/vocab/src/expression_search.rs:515:21 False  [LS3: assert!(!admits(code, length) && !valid_prefix(code, length));]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F)         BRANCH crates/vocab/src/expression_search.rs:515:46 False  [LS3: assert!(!admits(code, length) && !valid_prefix(code, length));]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F)         BRANCH crates/vocab/src/expression_search.rs:626:17 False  [LS3: assert!(choice < call && call < paused);]  assert!(choice < call && call < paused);
(F)         BRANCH crates/vocab/src/expression_search.rs:626:34 False  [LS3: assert!(choice < call && call < paused);]  assert!(choice < call && call < paused);
(F)         BRANCH crates/vocab/src/expression_search.rs:671:17 False  [LS3: assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F)         BRANCH crates/vocab/src/expression_search.rs:671:54 False  [LS3: assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F)         BRANCH crates/vocab/src/expression_search.rs:742:17 False  [LS3: assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F)         BRANCH crates/vocab/src/expression_search.rs:742:37 False  [LS3: assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F)         BRANCH crates/vocab/src/implication.rs:292:63 True  [LS3: assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));]  assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));
(F)         LINE   crates/vocab/src/implication.rs:329  [LS3: rows.len()]  rows.len()
(F)         BRANCH crates/vocab/src/mask.rs:419:46 False  [LS3: !bar.hits(&wider) || bar.hits(&base),]  !bar.hits(&wider) || bar.hits(&base),
(F)         BRANCH crates/vocab/src/mask.rs:661:17 False  [LS3: assert!(less.get(5) && !less.get(70) && less.get(200));]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F)         BRANCH crates/vocab/src/mask.rs:661:32 False  [LS3: assert!(less.get(5) && !less.get(70) && less.get(200));]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F)         BRANCH crates/vocab/src/mask.rs:661:49 False  [LS3: assert!(less.get(5) && !less.get(70) && less.get(200));]  assert!(less.get(5) && !less.get(70) && less.get(200));
(U)         LINE   crates/vocab/src/table.rs:1704  [LS3: None]  None
(F)         BRANCH crates/vocab/src/table.rs:1977:13 False  [LS3: worst_hit <= 4 && worst_miss <= 7,]  worst_hit <= 4 && worst_miss <= 7,
(F)         BRANCH crates/vocab/src/table.rs:1977:31 False  [LS3: worst_hit <= 4 && worst_miss <= 7,]  worst_hit <= 4 && worst_miss <= 7,
(F)         BRANCH crates/vocab/src/table.rs:2034:17 False  [LS3: assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2034:34 False  [LS3: assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2034:52 False  [LS3: assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2108:17 False  [LS3: assert!(m.get(0) && m.popcount() == 1);]  assert!(m.get(0) && m.popcount() == 1);
(F)         BRANCH crates/vocab/src/table.rs:2108:29 False  [LS3: assert!(m.get(0) && m.popcount() == 1);]  assert!(m.get(0) && m.popcount() == 1);
(F)         BRANCH crates/vocab/src/tolerance.rs:570:45 True  [LS3: !(shared && range >= RUNG_EXCLUSIVE_MIN_RANGE),]  !(shared && range >= RUNG_EXCLUSIVE_MIN_RANGE),
(F)         LINE   crates/vocab/tests/expression.rs:403  [LS3: bytes.get(..16)]  bytes.get(..16)
(F)         BRANCH crates/vocab/tests/expression.rs:414:13 False  [LS3: assert!(accepted > 0 && refused > accepted);]  assert!(accepted > 0 && refused > accepted);
(F)         BRANCH crates/vocab/tests/expression.rs:414:29 False  [LS3: assert!(accepted > 0 && refused > accepted);]  assert!(accepted > 0 && refused > accepted);
(F)         LINE   crates/vocab/tests/expression_search.rs:82  [LS3: Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),]  Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),
(F)         BRANCH crates/vocab/tests/expression_search.rs:201:13 False  [LS3: assert!(saw_candidate && saw_backtrack);]  assert!(saw_candidate && saw_backtrack);
(F)         BRANCH crates/vocab/tests/expression_search.rs:201:30 False  [LS3: assert!(saw_candidate && saw_backtrack);]  assert!(saw_candidate && saw_backtrack);
(F)         BRANCH crates/vocab/tests/expression_search_readiness.rs:126:9 False  [LS3: let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {]  let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {
(F)         LINE   crates/vocab/tests/expression_search_readiness.rs:127  [LS3: panic!("the last NOT must finish the maximum-capacity program");]  panic!("the last NOT must finish the maximum-capacity program");
(F)         BRANCH crates/vocab/tests/mask.rs:315:27 False  [LS3: before || !after,]  before || !after,
(F)         BRANCH crates/vocab/tests/no_float.rs:145:36 False  [LS3: *module == "lib.rs" || declared.contains(*module),]  *module == "lib.rs" || declared.contains(*module),
(F)         LINE   crates/vocab/tests/no_py_language.rs:282  [LS3: root.display(),]  root.display(),
(F)         LINE   crates/vocab/tests/no_py_language.rs:283  [LS3: String::from_utf8_lossy(&listed.stderr).trim()]  String::from_utf8_lossy(&listed.stderr).trim()
(F)         BRANCH crates/vocab/tests/no_py_language.rs:447:12 True  [LS3: if let Err(why) = verdict(&found) {]  if let Err(why) = verdict(&found) {
(F)         LINE   crates/vocab/tests/no_py_language.rs:448  [LS3: panic!("{why}");]  panic!("{why}");
(F)         LINE   crates/vocab/tests/no_py_language.rs:666  [LS3: String::from_utf8_lossy(&init.stderr)]  String::from_utf8_lossy(&init.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:679  [LS3: String::from_utf8_lossy(&add.stderr)]  String::from_utf8_lossy(&add.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:783  [LS3: String::from_utf8_lossy(&init.stderr)]  String::from_utf8_lossy(&init.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:815  [LS3: String::from_utf8_lossy(&add.stderr)]  String::from_utf8_lossy(&add.stderr)
(F)         LINE   crates/vocab/tests/no_runtime_spawn.rs:2560  [LS3: sources.len()]  sources.len()
(F)         LINE   crates/vocab/tests/no_runtime_spawn.rs:2585  [LS3: failures.join("\n  ")]  failures.join("\n  ")
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:2910:26 False  [LS3: .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),]  .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:13 False  [LS3: assert!(out.found.is_empty() && out.in_tests == 0);]  assert!(out.found.is_empty() && out.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:37 False  [LS3: assert!(out.found.is_empty() && out.in_tests == 0);]  assert!(out.found.is_empty() && out.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:13 False  [LS3: assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);]  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:50 False  [LS3: assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);]  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:13 False  [LS3: assert!(block.found.is_empty() && block.in_tests == 0);]  assert!(block.found.is_empty() && block.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:39 False  [LS3: assert!(block.found.is_empty() && block.in_tests == 0);]  assert!(block.found.is_empty() && block.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:13 False  [LS3: assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:41 False  [LS3: assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:67 False  [LS3: assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:13 False  [LS3: assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));]  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:42 False  [LS3: assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));]  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:13 False  [LS3: assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));]  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:47 False  [LS3: assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));]  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F)         LINE   crates/vocab/tests/stale_claims.rs:158  [LS3: with_commas(count)]  with_commas(count)
(F)         LINE   crates/vocab/tests/table.rs:125  [LS3: doc.len(),]  doc.len(),
(F)         BRANCH crates/vocab/tests/table.rs:270:13 False  [LS3: let BitStatus::Void { reason } = def.status else {]  let BitStatus::Void { reason } = def.status else {
(F)         LINE   crates/vocab/tests/table.rs:271  [LS3: unreachable!("235..=273 are void")]  unreachable!("235..=273 are void")
(F)         BRANCH crates/vocab/tests/table.rs:373:13 False  [LS3: !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         BRANCH crates/vocab/tests/table.rs:373:39 False  [LS3: !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         BRANCH crates/vocab/tests/table.rs:373:63 False  [LS3: !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         LINE   crates/vocab/tests/table.rs:432  [LS3: first - 1,]  first - 1,
(F)         BRANCH crates/vocab/tests/table.rs:561:45 False  [LS3: n == ConditionMask::BITS || ALLOWED.contains(&n),]  n == ConditionMask::BITS || ALLOWED.contains(&n),
(F)         BRANCH crates/vocab/tests/table.rs:813:17 False  [LS3: !status_cells.contains("retired") && !status_cells.contains("void"),]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F)         BRANCH crates/vocab/tests/table.rs:813:54 False  [LS3: !status_cells.contains("retired") && !status_cells.contains("void"),]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F)         BRANCH crates/vocab/tests/table.rs:819:21 False  [LS3: status_cells.contains("retired")]  status_cells.contains("retired")
(F)         BRANCH crates/vocab/tests/table.rs:820:28 False  [LS3: && status_cells.contains(&format!("duplicates {duplicate_of}")),]  && status_cells.contains(&format!("duplicates {duplicate_of}")),
(F)         BRANCH crates/vocab/tests/table.rs:843:9 False  [LS3: retired_rows >= retired && void_rows >= void,]  retired_rows >= retired && void_rows >= void,
(F)         BRANCH crates/vocab/tests/table.rs:843:36 False  [LS3: retired_rows >= retired && void_rows >= void,]  retired_rows >= retired && void_rows >= void,
(F)         BRANCH crates/vocab/tests/table.rs:1012:13 False  [LS3: !name.contains("vix") && !name.contains("volatility_index"),]  !name.contains("vix") && !name.contains("volatility_index"),
(F)         BRANCH crates/vocab/tests/table.rs:1012:38 False  [LS3: !name.contains("vix") && !name.contains("volatility_index"),]  !name.contains("vix") && !name.contains("volatility_index"),
(F)         LINE   crates/vocab/tests/workspace_is_rust.rs:397  [LS3: out.len()]  out.len()
(F)         LINE   crates/vocab/tests/workspace_is_rust.rs:753  [LS3: named.join("\n  ")]  named.join("\n  ")

APPENDIX LS1: the fenced content of reports/base-cloud-20261103/fu-LS1.txt.md on branch cloud/cov-libs-ls1-20261103, verbatim
FINAL: Route B (the kit's d3-route-b-cloud.sh, sha256 f032b525..., unchanged; d3-covset.mjs ecef3bdc...) on tip 24bd3a62cd51abb1c7471d5e2edf64b5ca677d8d (the code tip of the second 5 Nov continuation), set 'reading whole:store' (117 .rs files), pinned rustc 1.99.0-nightly (1a98b1e13 2026-08-07) installed as nightly-2026-08-08, LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 root host (4 cores), run under umask 000 so the profile folder takes the nobody re-runs' profiles (D-0995) and the fresh-process children's.
BEFORE (first 5 Nov continuation, tip 4b3ee558): lines 52,295/53,771 (1,476 uncovered), branches 3,405/3,964 (559 uncovered). PROGRESS (this session, tip 095165bb): lines 53,071/54,519 (1,448 uncovered), branches 3,433/3,990 (557 uncovered).

RULES (recipe v4 step 4, and the main session's answer of 5 Nov): (F) the failing side of an assert/panic arm inside test code, and (step 4's second clause) a test's or a check module's own failing, bounding or skip arm, failure-collection lines and fault-mock guards, each with its proof; (U) a region no input, file, folder, variable or direct call can reach on aarch64-apple-darwin, with its proof from the code; a macOS-only arm is owed to the Mac's Route B and never claimed. Every production row a host refusal or a race reaches is driven by a test (D-8561), none recorded as a limit. A row covered here that no test can reach on aarch64-apple-darwin is (U) on that target with its proof (the main session's answer (A) of 5 Nov for safe_open.rs:104/107), and the Mac's Route B records it the same way.

(U) ITEMS, PRODUCTION (81), each with its proof as written on 5 Nov (line numbers INSIDE a proof are as of 4b3ee558; each row's own number is this run's)
  BRANCH crates/store/src/checksum_audit.rs:288:12 True: || layout.header_len() != HEADER_BYTES as u64
      (U) a header region of another size: header_len is slot_count * SLOT_STRIDE (layout.rs:363-365) and every Layout is built by Layout::declared or Layout::declare, which refuse a slot_count outside 2..=MAX_SLOT_COUNT with MAX_SLOT_COUNT = 2 (layout.rs:674-675, format.rs:95); so header_len is 2 * 16,384 = 32,768 = HEADER_BYTES for every Layout (checksum_audit_cover_tests.rs asserts it beside the flag and block cases).
  BRANCH crates/store/src/file_sealed.rs:135:12 True: if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
      (U) block_rows past the one-frame cache: every head reaching line 135 was accepted by SealedMonth::open (file_sealed.rs:129) -> Head::decode, which refuses block_rows == 0 || block_rows > BLOCK_ROWS (sealed.rs:1117-1119), and file.rs:304 asserts at compile time CACHED_BLOCK_LEN == BLOCK_ROWS * RECORD_LEN; so block_rows * RECORD_LEN <= CACHED_BLOCK_LEN always.
  LINE   crates/store/src/file_sealed.rs:136: return Err(StoreError::SealedRefused {
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:137: path: at,
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:138: reason: "block_rows",
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:139: detail: format!(
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:140: "the head declares {} rows a frame and a handle caches at most {}",
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:141: head.block_rows,
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:142: CACHED_BLOCK_LEN / RECORD_LEN
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:143: ),
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:144: });
      (U) as file_sealed.rs:135.
  BRANCH crates/store/src/file_sealed.rs:202:36 True: if cache.block != block || cache.n_valid != n_valid {
      (U) a sealed handle's cache keyed by the counter: the handle's header is set once by open_sealed_at (file_sealed.rs:146) and self.header is assigned elsewhere only at file.rs:3442 inside append, which a sealed handle leaves at file.rs:3226-3227 (sealed_offer) before any write; cache.n_valid is set only to header.n_valid after a fill (file_sealed.rs:207). So cache.block == block implies cache.n_valid == n_valid.
  LINE   crates/store/src/file_sealed.rs:231: )?;
      (U) offset_of refused inside fill_sealed: fill_sealed reaches line 229 only after sealed.read_block accepted block < n_blocks (sealed.rs:2203); n_blocks is a u32 (sealed.rs:1172) pinned to ceil(n_valid / block_rows) (sealed.rs:1240-1244) with block_rows <= 256, so block * rows < 2^40 and Layout::offset_of overflows only past (2^64 - header) / 56 (layout.rs:410-418).
  LINE   crates/store/src/file_sealed.rs:236: path: self.bars_path.clone(),
      (U) a decoded frame longer than the cache: read_block(block) decodes one frame of rows_in(block) <= block_rows <= 256 bars (sealed.rs:993-997, 2202-2213), and the cache has CACHED_BLOCK_LEN / RECORD_LEN = 14,336 / 56 = 256 slots (file.rs:302), so slots.next() never ends first.
  LINE   crates/store/src/file_sealed.rs:237: source: FormatError::OffsetOverflow,
      (U) as file_sealed.rs:236.
  LINE   crates/store/src/file_sealed.rs:238: })?;
      (U) as file_sealed.rs:236.
  BRANCH crates/store/src/file_sealed.rs:341:20 False: if let Some(count) = first.get_mut(at + 1) {
      (U) read_rows_at placement: each group value is an index into blocks (file_sealed.rs:328-333: an existing entry, or blocks.len() just before the push), so at + 1 <= blocks.len() < first.len() = blocks.len() + 1 (file_sealed.rs:338).
  BRANCH crates/store/src/file_sealed.rs:347:20 False: if let Some(start) = first.get_mut(at) {
      (U) read_rows_at placement: at runs over 1..first.len() (file_sealed.rs:345), so first.get_mut(at) is always Some.
  BRANCH crates/store/src/file_sealed.rs:354:20 False: if let Some(slot) = next_slot.get_mut(at) {
      (U) read_rows_at placement: next_slot is first.clone() (file_sealed.rs:351) and at < blocks.len() < next_slot.len().
  BRANCH crates/store/src/file_sealed.rs:355:24 False: if let Some(place) = placed.get_mut(*slot) {
      (U) read_rows_at placement: first is the prefix sum of the group sizes over rows.len() rows, so group g's slots run first[g] .. first[g + 1] - 1 <= rows.len() - 1 < placed.len() (file_sealed.rs:352).
  LINE   crates/store/src/file_sealed.rs:359: }
      (U) the close of file_sealed.rs:354-355's arms; reached only through their else sides, which are (U).
  BRANCH crates/store/src/file_sealed.rs:371:21 False: let Some(&row) = rows.get(asked) else {
      (U) read_rows_at: placed holds the asked positions 0 .. rows.len() (file_sealed.rs:353-356), so rows.get(asked) is always Some.
  LINE   crates/store/src/file_sealed.rs:372: continue;
      (U) as file_sealed.rs:371.
  BRANCH crates/store/src/file_sealed.rs:379:24 False: if let Some(slot) = out.get_mut(asked) {
      (U) read_rows_at: out has rows.len() entries (file_sealed.rs:362) and asked < rows.len().
  BRANCH crates/store/src/file_sealed.rs:417:9 False: let Some(sealed) = file.sealed.as_ref() else {
      (U) audit_sealed: open_sealed_at's only Ok(Some) builds the handle with sealed: Some(month) (file_sealed.rs:146-164), so file.sealed is never None here.
  LINE   crates/store/src/file_sealed.rs:418: return Ok(None);
      (U) as file_sealed.rs:417.
  LINE   crates/store/src/file.rs:2891: )?;
      (U) block::seal refuses only by covered_len, which is layout.covered_byte_range(block, n_valid) itself (block.rs:332-334), already Ok for this block and counter at file.rs:2882-2885, or by a length mismatch, and bytes is slice_of(room, end - start), exactly the covered length (file.rs:2886, 6232-6249).
  LINE   crates/store/src/file.rs:3172: return Err(StoreError::Format {
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3173: path: self.bars_path.clone(),
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3174: source,
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3175: });
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  BRANCH crates/store/src/file.rs:3855:21 False: let Some(tix) = self.tix.as_ref() else {
      (U) index_batch re-reads self.tix after reindex: it is inside the Some(tix) arm of file.rs:3836, self.tix is assigned only when the handle is built (file.rs:2758-2762; file_sealed.rs:161) and nothing sets it to None (set_tix_state, file.rs:3761-3764, writes the state inside the Some), so it is Some here.
  LINE   crates/store/src/file.rs:3856: return Ok(None);
      (U) as file.rs:3855:21 False.
  BRANCH crates/store/src/file.rs:3869:13 False: let Some(tix) = self.tix.as_ref() else {
      (U) index_batch re-reads self.tix after reindex: it is inside the Some(tix) arm of file.rs:3836, self.tix is assigned only when the handle is built (file.rs:2758-2762; file_sealed.rs:161) and nothing sets it to None (set_tix_state, file.rs:3761-3764, writes the state inside the Some), so it is Some here.
  LINE   crates/store/src/file.rs:3870: return Ok(None);
      (U) as file.rs:3855:21 False.
  LINE   crates/store/src/file.rs:3891: Err(other) => Err(index_refused(&tix.path, &other)),
      (U) extend's refusals are Outside, OffGrid, SharedSlot (matched by the arms above) and Unreadable, which only an Err stamp makes (time_index.rs:650); index_batch hands it Ok(row.stamp()) for every row (file.rs:3876).
  LINE   crates/store/src/file.rs:4013: Err(other) => return Err(index_refused(&tix.path, &other)),
      (U) as file.rs:3891: index_at_threshold hands extend Ok(row.stamp()) for every batch row (file.rs:3991).
  BRANCH crates/store/src/file.rs:4189:36 True: if cache.block != block || cache.n_valid != n_valid {
      (U) read_verified keyed by the counter alone: self.header moves only at file.rs:3442 in append, and every append of a month with a sidecar (the only kind read_verified serves, file.rs:4163-4165) clears the cache first in seal_committed (file.rs:2836-2839, called at file.rs:3420); the post-commit sort and summary steps run inside untouched (sort_index_writer.rs:192-206), which puts the cache back. So cache.block == block implies cache.n_valid == n_valid.
  LINE   crates/store/src/file.rs:4316: path: self.bars_path.clone(),
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:4317: source: FormatError::OffsetOverflow,
      (U) a covered range longer than the handle's buffer: covered_byte_range's length is records_in_block * stride <= records_per_block * stride = block_len (layout.rs:543-551), and every geometry a BarFile resolves has block_len <= MAX_BLOCK_LEN (the compile-time assertion beside MAX_BLOCK_LEN that the doc comment at file.rs:4309-4313 names), so cache.bytes.get_mut(..span) is always Some.
  LINE   crates/store/src/file.rs:4318: })?;
      (U) a covered range longer than the handle's buffer: covered_byte_range's length is records_in_block * stride <= records_per_block * stride = block_len (layout.rs:543-551), and every geometry a BarFile resolves has block_len <= MAX_BLOCK_LEN (the compile-time assertion beside MAX_BLOCK_LEN that the doc comment at file.rs:4309-4313 names), so cache.bytes.get_mut(..span) is always Some.
  BRANCH crates/store/src/file.rs:5666:42 True: BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
      (U) O_SYNC is the const Some(open_flags::O_SYNC) on macOS and on Linux/Android x86_64/aarch64 (file.rs:5568-5576); the None definition compiles only on other targets, so O_SYNC.is_none() is false on aarch64-apple-darwin.
  LINE   crates/store/src/flock.rs:342: || "an unnamed program".to_owned(),
      (U) the program name of a process started with an EMPTY argument vector: std::env::args_os().next() is None only then; std::process::Command always passes an argv[0] (arg0 can make it empty, never absent) and the workspace denies unsafe code, so no test can start such a process, and PROCESS is set once per process.
  BRANCH crates/store/src/flock.rs:571:24 False: if let Ok(modified) = meta.modified() {
      (U) Metadata::modified() on Unix is Ok(SystemTime::new(st_mtime, st_mtime_nsec)), an Err only for a nanosecond count outside 0..1e9, which no kernel returns; so the else side is never taken on macOS or Linux.
  BRANCH crates/store/src/flock.rs:574:24 False: if let Ok(accessed) = meta.accessed() {
      (U) as flock.rs:560:24 False, for accessed() and st_atime.
  BRANCH crates/store/src/month_summary.rs:939:24 False: if let Some(words) = entry.seconds.as_deref() {
      (U) a held day without its seconds bitmap while seconds are kept: keeps_seconds() needs self.seconds (month_summary.rs:684); every Summary is built by new(), whose days are DayBits::empty(seconds) (:550), or by decode, which sets self.seconds (:1053) and then rebuilds every day as DayBits::empty(tracked) (:1069); DayBits::empty(true) holds Some (:217), and no code sets a day's seconds to None or flips self.seconds afterwards.
  BRANCH crates/store/src/month_summary.rs:1055:12 True: if at.short || at.at != payload.len() {
      (U) at.short at the end of decode: every short read of a day entry yields last = 0, which makes the entry invalid (refused at line 1076) unless first == 0 and bars > 0, and then that day is held and its minute bitmap's at.bits refuses itself ("a bitmap runs past the payload", line 1253) because the cursor is already past the end; at.bits refuses every short bitmap itself. So decode never reaches line 1055 with at.short set.
  BRANCH crates/store/src/month_summary.rs:1085:24 False: if let Some(words) = entry.seconds.as_deref_mut() {
      (U) as month_summary.rs:939:24 False: decode_days rebuilt every entry as DayBits::empty(tracked) at :1069, so with tracked every entry's seconds is Some at :1085.
  LINE   crates/store/src/month_summary.rs:1087: }
      (U) the close of the if-let at :1085, counted on its else side; as month_summary.rs:1085:24 False.
  BRANCH crates/store/src/provenance.rs:1665:12 False: if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
      (U) put: every call passes a constant offset and a constant-width value inside the fixed-size image it writes (the record image, provenance.rs:373-402; the header image), so the range is always inside.
  BRANCH crates/store/src/provenance.rs:1673:12 False: if let Some(run) = bytes.get(at..at.saturating_add(N)) {
      (U) take: every call reads a constant offset and width inside a fixed-size record or header image (provenance.rs:416-430, 596-603), so the range is always inside.
  LINE   crates/store/src/repair.rs:851: rows: header.n_valid,
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  LINE   crates/store/src/repair.rs:852: ceiling,
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  LINE   crates/store/src/repair.rs:853: };
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  BRANCH crates/store/src/safe_open.rs:299:9 False: let Some(flags) = FLAGS else {
      (U) on every verified target: FLAGS is the const Some(O_NOFOLLOW_NONBLOCK) under cfg(any(macos, linux|android on x86_64|aarch64)) (safe_open.rs:231-238); the None arm compiles only on other targets.
  LINE   crates/store/src/safe_open.rs:300: return Err(io(
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:301: "open",
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:302: io::Error::new(
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:303: ErrorKind::Unsupported,
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:304: "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:305: ),
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:306: ));
      (U) as safe_open.rs:299.
  BRANCH crates/store/src/sealed.rs:1499:23 False: } else if let Some(range) = self.ranges.get_mut(5) {
      (U) self.ranges is a [_; 6] array (one range per column): get_mut(5) is always Some.
  BRANCH crates/store/src/sealed.rs:1525:16 False: if let Some(oi) = columns.get_mut(5) {
      (U) columns is [ColumnRange::default(); 6] (sealed.rs:1519): get_mut(5) is always Some.
  BRANCH crates/store/src/sealed.rs:2352:35 False: Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
      (U) last_before's short window: skip = lo - lo.saturating_sub(1) is 0 when lo = 0 and 1 otherwise (sealed.rs:2341-2343), index = skip + at and block = lo + at (:2349-2350); index >= 1 (Some(previous)) needs lo >= 1 or at >= 1, and either makes block >= 1, so the guard block > 0 holds whenever the arm is reached, for any lo, hi and ts.
  BRANCH crates/store/src/settings.rs:220:16 False: if let Some(slot) = self.values.get_mut(key.slot()) {
      (U) Settings::put: values is [Option<String>; KEYS] with KEYS = 11 (settings.rs:132, 209) and Key::slot maps the eleven keys to 0..=10 (settings.rs:187-200), so get_mut(key.slot()) is always Some.
  BRANCH crates/store/src/settings.rs:688:30 False: let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
      (U) a unit under the floor: every unit scales by at least 10^9 (settings.rs:661-665), MIN_FLOOR_BYTES is 10^9 (:635) and a zero amount is refused at :682-685, so with a unit bytes >= MIN_FLOOR_BYTES and line 687's branch is entered only with unit "" (settings_door_cover_tests.rs asserts 1GB is the floor itself).
  BRANCH crates/store/src/verdict.rs:716:8 True: if Vendor::ALL.get(at) != Some(&feed) {
      (U) core::vendor::Vendor declares Groww, Dhan, TrueData, Gdfl, Zerodha with default discriminants 0..=4 and ALL lists them in that order (core/src/vendor.rs:52-90, :94-104), so ALL.get(feed as usize) == Some(&feed) for every feed.
  LINE   crates/store/src/verdict.rs:717: return None;
      (U) as verdict.rs:716:8 True.
  BRANCH crates/store/src/verdict.rs:2278:16 False: if let Some(slot) = out.first.get_mut(ordinal) {
      (U) out.first is vec![0; planned.len()] (verdict.rs:2259) and ordinal enumerates planned (:2269), so get_mut(ordinal) is always Some. (Filed as test code by the 3 Nov classifier; it is production.)
  BRANCH crates/store/src/verdict.rs:2312:16 False: if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
      (U) counts has CODE_SLOTS = 24 slots (verdict.rs:168, :2301) and Code's discriminants are 1..=18 (verdict.rs:249-289; code() is `self as u8`, :318), so get_mut(code) is always Some. (Filed as test code by the 3 Nov classifier.)
  BRANCH crates/store/src/verdict.rs:2478:16 False: if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
      (U) as verdict.rs:2312:16 False: the record's code was decoded by Code::from_code, which answers only known codes (verdict.rs:1776-1781). (Filed as test code by the 3 Nov classifier.)
  BRANCH crates/store/src/verdict.rs:3143:15 False: while let Some(parent) = at.parent() {
      (U) make_dirs' walk: dir is series.dir() joined with the year, and series.dir() is self.root joined with four fixed relative words (verdict.rs:805-812), so walking parent() from dir meets series.root, where the loop breaks (:3145-3146), before parent() can answer None.
  BRANCH crates/store/src/verdict.rs:3417:12 False: if let Some(entry) = days.get_mut(doy) {
      (U) day_of_year is a civil day's offset in its own year, 0..=365 (verdict.rs:870-874), and every day table holds YEAR_DAYS = 366 entries (verdict.rs:194; built at 2789, 3096, 3382), so get_mut(doy) is always Some.
  BRANCH crates/store/src/verdict.rs:3853:24 False: && let Some(oldest) = self
      (U) handle cache: the let runs only when self.handles.len() >= HANDLE_CACHE >= 1, so min_by_key over a non-empty map is always Some.
  BRANCH crates/store/src/verdict.rs:3923:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) lookup calls self.prefix(.., &path, ..) first (verdict.rs:3921), which answers ImageMissing itself when handle() is None (:4017-4021) and otherwise leaves path in self.handles (handle inserts it, :3861); the second handle(&path) finds contains_key true (:3843) and answers Some without opening, and nothing between removes the entry (eviction happens only inside handle when a new path is inserted).
  LINE   crates/store/src/verdict.rs:3924: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:3923:13 False.
  BRANCH crates/store/src/verdict.rs:4075:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) lookup calls self.prefix(.., &path, ..) first (verdict.rs:3921), which answers ImageMissing itself when handle() is None (:4017-4021) and otherwise leaves path in self.handles (handle inserts it, :3861); the second handle(&path) finds contains_key true (:3843) and answers Some without opening, and nothing between removes the entry (eviction happens only inside handle when a new path is inserted).
  LINE   crates/store/src/verdict.rs:4076: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:3923:13 False.
  BRANCH crates/store/src/verdict.rs:4136:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) lookup calls self.prefix(.., &path, ..) first (verdict.rs:3921), which answers ImageMissing itself when handle() is None (:4017-4021) and otherwise leaves path in self.handles (handle inserts it, :3861); the second handle(&path) finds contains_key true (:3843) and answers Some without opening, and nothing between removes the entry (eviction happens only inside handle when a new path is inserted).
  LINE   crates/store/src/verdict.rs:4137: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:3923:13 False.

PRODUCTION ROWS ONLY A HOST REFUSAL OR A RACE REACHES, NOW DRIVEN (the 61 of 5 Nov and staging.rs's race, 32 entries; lines as of 4b3ee558), each with how and the test that drives it
  crates/store/src/flock.rs:557 (LINE, the fstat's refused arm): HOST, seam; flock::cover_tests::a_noted_lock_whose_fstat_is_refused_puts_back_no_time
  crates/store/src/flock.rs:627 (LINE): HOST, seam; flock::cover_tests::a_patient_shared_lock_the_host_refuses_is_the_hosts_error_at_once
  crates/store/src/flock.rs:681:20 True, :682: HOST, seam; flock::cover_tests::a_note_the_host_will_not_clear_is_said_and_the_lock_still_let_go
  crates/store/src/flock.rs:974: HOST, seam; flock::cover_tests::a_patient_exclusive_lock_the_host_refuses_is_impatient_host
  crates/store/src/generation.rs:519: HOST, seam; generation::door_cover_tests::a_pointer_record_whose_write_is_refused_refuses_the_promotion_as_maybe_visible
  crates/store/src/provenance.rs:1282: HOST, seam; provenance::cover_tests::a_new_sidecar_whose_header_write_is_refused_refuses_the_landing
  crates/store/src/quarantine.rs:831: filed HOST; a REAL race instead (the index cut to its header after the header read); quarantine::tests::an_index_cut_under_a_probe_refuses_it_at_the_read_it_cut
  crates/store/src/quarantine.rs:861: filed HOST; a REAL race instead (the index cut to nothing after its fstat); quarantine::tests::an_index_cut_under_a_probe_refuses_it_at_the_read_it_cut
  crates/store/src/quarantine.rs:1032: HOST, seam (the second look's shared flock); quarantine::tests::a_lock_the_host_will_not_ask_about_is_no_writer_to_a_probe
  crates/store/src/quarantine.rs:1186: HOST, seam; quarantine::tests::an_index_read_the_host_refuses_refuses_the_act_before_its_line
  crates/store/src/quarantine.rs:1187:8 True, :1188-1191: RACE (the index cut to 100 bytes between its fstat and its read); quarantine::tests::an_index_cut_between_its_measure_and_its_read_is_rebuilt_by_the_writer
  crates/store/src/quarantine.rs:1439:25 False: RACE (the audit directory made between the look and the creation); quarantine::tests::an_audit_directory_made_between_the_look_and_the_creation_is_taken
  crates/store/src/quarantine.rs:1467-1471: HOST, seam; quarantine::tests::a_quarantine_lock_the_host_refuses_is_refused_before_anything_is_written
  crates/store/src/quarantine.rs:1759: HOST, seam; quarantine::tests::a_ledger_read_the_host_refuses_is_refused_by_the_ledgers_name
  crates/store/src/quarantine.rs:2218: HOST, seam; quarantine::tests::an_index_image_whose_staged_write_is_refused_is_never_published
  crates/store/src/quarantine.rs:2224: filed HOST; a REAL race instead (a directory made at the index's name before the rename); quarantine::tests::a_directory_made_at_the_index_before_its_rename_refuses_the_publication
  crates/store/src/repair.rs:1183:24 True, :1184-1186: RACE (another writer's valid month written over the published revision); repair::door_cover_tests::a_published_revision_changed_before_its_read_back_is_never_served
  crates/store/src/repair.rs:1342 (filed as the reservation's flush; it is the reservation's create_new): a REAL refusal (a revision directory that takes no new name, mode 0o555 where the bits bind); repair::door_cover_tests::a_revision_directory_that_takes_no_name_refuses_the_reservation
  crates/store/src/repair.rs:1373:12 True, :1374: RACE (another writer's valid month written over the revision before its read-back); repair::door_cover_tests::a_revision_changed_before_its_read_back_is_a_conflict_with_no_receipt
  crates/store/src/repair.rs:1392 (filed as the receipt's write; it is the receipt's create_new): RACE (another actor makes the receipt first); repair::door_cover_tests::a_receipt_another_actor_made_first_is_never_written_over
  crates/store/src/repair.rs:1398 (filed as the receipt's flush; it is the receipt's write): HOST, seam (with the reservation's and the receipt's flushes, one-line calls); repair::door_cover_tests::a_revision_whose_reservation_or_receipt_is_refused_is_refused_by_that_step
  crates/store/src/repair.rs:1458: HOST, seam; repair::door_cover_tests::a_source_whose_shared_lock_the_host_refuses_reserves_nothing
  crates/store/src/safe_open.rs:104:19 False, :107: HOST: a REAL file of none of the seven kinds (Linux: an anonymous inode a child tail -f holds); the Mac: (U) with its proof (the main session's answer (A), section "(U) ON THE MAC'S TARGET ONLY" below); safe_open::tests::a_file_of_none_of_the_seven_kinds_is_unknown
  crates/store/src/file_sealed.rs:425:21 False: RACE (the month's directory replaced by a file before the raw form's look); file::sealed_door::cover_tests::a_raw_form_look_that_fails_otherwise_is_no_missing_raw_form
  crates/store/src/settings.rs:885:8 True, :886-888: RACE (the file grown past the bound between its fstat and its read); settings::door_cover_tests::a_settings_file_grown_past_the_bound_while_read_is_a_fault
  crates/store/src/settings.rs:1227: RACE (a directory made at the temporary's name before the staging door opens it); settings::door_cover_tests::a_name_made_at_the_temporary_refuses_the_save_and_is_left
  crates/store/src/settings.rs:1234:12 True, :1235, :1236: HOST, seam; settings::door_cover_tests::a_temporary_whose_write_is_refused_is_removed_with_nothing_visible
  crates/store/src/settings.rs:1321-1324: RACE (the settings directory replaced by a file between the two listings); settings::door_cover_tests::a_directory_replaced_between_the_two_listings_is_the_stores_fault
  crates/store/src/settings.rs:1367, :1369, :1369:21 True, :1370-1374: RACE (another saver lands the same number first); settings::door_cover_tests::a_link_raced_by_another_saver_or_a_vanished_temporary_is_refused
  crates/store/src/settings.rs:1369:21 False, :1376: RACE (the temporary removed before the link); settings::door_cover_tests::a_link_raced_by_another_saver_or_a_vanished_temporary_is_refused
  crates/store/src/settings.rs:1445: RACE (the generation taken away after the link and put back before the read-back); settings::door_cover_tests::a_landed_save_the_refresh_missed_that_reads_again_says_so
  crates/store/src/staging.rs:299:29 False (new at 095165bb; nondeterministic before): RACE (the leftover removed between the look and the removal); staging::cover_tests::a_leftover_removed_between_the_look_and_the_removal_is_gone

(U) ON THE MAC'S TARGET ONLY (2 rows, covered here): the main session's answer (A) of 5 Nov 2026 to the second continuation's question, recorded by the third continuation (token -4); not in the 81 above, which are this host's uncovered rows. The Mac's Route B records these two the same way (the report's NEEDS THE MAC).
  BRANCH crates/store/src/safe_open.rs:104:19 False: } else if kind.is_block_device() {
      (U) on aarch64-apple-darwin. The side is Found::of's last arm (safe_open.rs:90-109), taken only for a FileType that is none of the seven kinds the chain asks about: a regular file, a symlink, a directory, a FIFO, a socket, a char device, a block device. std::fs::FileType has no constructor, so the arm needs a REAL file whose st_mode carries none of the seven types. On macOS the one such kind is a union mount's whiteout (S_IFWHT), which no test can make; a kqueue descriptor reports S_IFIFO, a pipe; POSIX shared memory needs shm_open, which std does not offer and which needs unsafe code (store is #![forbid(unsafe_code)] at lib.rs:61, under the workspace's unsafe_code = "deny"). A refusal seam or a race hook cannot stand in: the arm classifies what the host reports, and a FileType value cannot be faked; a seam answering Unknown early would not run line 107 at all. Restructuring Found::of to classify the mode's type bits, so a crafted mode reaches the arm, is rejected: D-6017 rejects restructuring a production path so a test can reach it. On Linux the side is covered by a real anonymous inode, the inotify instance a child GNU tail -f holds: safe_open::tests::a_file_of_none_of_the_seven_kinds_is_unknown (cfg(target_os = "linux"), safe_open_tests.rs:496-523), kept.
  LINE   crates/store/src/safe_open.rs:107: Some(Self::Unknown)
      (U) on aarch64-apple-darwin, as safe_open.rs:104:19 False: the line is that arm's body.

PRODUCTION GAPS (0): a production row with no test and no (U) proof

IGNORED TESTS (1108 rows): bodies cargo test never runs, and helpers only they call; owed to the operator (L-BCL-3) or to a run made on purpose
  24 rows: crates/store/src/file.rs
  26 rows: crates/store/src/file_sealed_tests.rs
  410 rows: crates/store/tests/sealed_codec_samples.rs
  95 rows: crates/store/tests/sealed_door.rs
  186 rows: crates/store/tests/sealed_samples.rs
  189 rows: crates/store/tests/sealed_write_door.rs
  178 rows: crates/store/tests/tix_latency.rs

F-bound (26): a test's or check module's own bounding arm, each with its proof (step 4's second clause)
  BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True: if started.elapsed() > std::time::Duration::from_secs(2) {
      the test's 2 s bound on a child whose strict open must not wait for a FIFO peer: taken only when the open waits, the failure the test exists to catch
  LINE   crates/store/src/checksum_audit_tests.rs:153: child.kill().expect("bounded timeout");
      as the 2 s bound above
  LINE   crates/store/src/checksum_audit_tests.rs:154: child.wait().expect("reap child");
      as the 2 s bound above
  LINE   crates/store/src/checksum_audit_tests.rs:155: return Err("strict checksum source open waited for a FIFO peer".into());
      as the 2 s bound above: the bound's own failure
  BRANCH crates/store/src/month_summary_tests.rs:293:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
      a 10 s bound on a read that must answer at once: taken only when it waited on a FIFO writer
  LINE   crates/store/src/month_summary_tests.rs:294: let _release = fs::OpenOptions::new().write(true).open(pipe);
      as the 10 s bound: it wakes the parked reader before the test fails
  BRANCH crates/store/src/safe_open_tests.rs:122:9 False: let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
      a 10 s bound on an open that must answer at once: taken only when it waited on a FIFO writer
  LINE   crates/store/src/safe_open_tests.rs:123: let _release = fs::OpenOptions::new().write(true).open(pipe);
      as the 10 s bound: it wakes the parked open before the test fails
  BRANCH crates/store/src/settings_tests.rs:711:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
      a 10 s bound on a settings read that must answer at once: taken only when it waited
  LINE   crates/store/src/settings_tests.rs:712: if let Some(pipe) = pipe {
      inside the 10 s bound's failure arm: wakes a parked FIFO reader, when there is one, before the test fails
  BRANCH crates/store/src/settings_tests.rs:712:16 True: if let Some(pipe) = pipe {
      inside the 10 s bound's failure arm: wakes a parked FIFO reader, when there is one, before the test fails
  BRANCH crates/store/src/settings_tests.rs:712:16 False: if let Some(pipe) = pipe {
      inside the 10 s bound's failure arm: wakes a parked FIFO reader, when there is one, before the test fails
  LINE   crates/store/src/settings_tests.rs:713: let _release = std::fs::OpenOptions::new().write(true).open(pipe);
      as the 10 s bound's failure arm
  LINE   crates/store/src/settings_tests.rs:714: }
      the close of the 10 s bound's pipe release
  BRANCH crates/store/src/settings_tests.rs:1156:20 True: if started.elapsed() > std::time::Duration::from_secs(10) {
      the racer thread's 10 s bound waiting for generation 2 to appear: taken only when the save under test never lands it, and the thread's None fails the test
  LINE   crates/store/src/settings_tests.rs:1157: return None;
      as the racer's 10 s bound
  BRANCH crates/store/src/verdict_tests.rs:460:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
      a 10 s bound on a verdict call that must answer at once: taken only when it waited on a FIFO
  LINE   crates/store/src/verdict_tests.rs:461: let _release = fs::OpenOptions::new().write(true).open(pipe);
      as the 10 s bound: it wakes the parked call before the test fails
  BRANCH crates/store/tests/cloud_mutants_store.rs:144:29 True: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
      the race loop's own bound: it ends when both counts reach 20 or at its 1-minute deadline, and which count reaches 20 first is the race's; the deadline side is taken only when the race starved, which the asserts after it report
  BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
      the race loop's own bound: it ends when both counts reach 20 or at its 1-minute deadline, and which count reaches 20 first is the race's; the deadline side is taken only when the race starved, which the asserts after it report
  BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True: if since.elapsed() > std::time::Duration::from_secs(5) {
      a 5 s bound waiting for the second flusher: taken only when the batch flushed alone, which the test refuses
  LINE   crates/store/tests/cloud_mutants_store.rs:3172: return Err(std::io::Error::other("flushed alone"));
      as the 5 s bound
  BRANCH crates/store/tests/fifo.rs:121:12 False: if let Ok(outcome) = recv.recv_timeout(BOUND) {
      the bound on an open that must refuse a FIFO at once: its else side is taken only when the open blocked
  LINE   crates/store/tests/fifo.rs:124: }
      the close of the bound's success arm, on its failure side
  LINE   crates/store/tests/fifo.rs:126: drop(fs::OpenOptions::new().write(true).open(release));
      as the bound: it wakes the parked reader before the test fails
  LINE   crates/store/tests/fifo.rs:127: drop(worker.join());
      as the bound: it reaps the worker before the test fails

F-skip (28): a test's or check module's own skip arm, each with its proof (step 4's second clause)
  BRANCH crates/store/src/file_write_door_tests.rs:67:13 False: let Ok(entries) = fs::read_dir(&at) else {
      the snapshot helper's skip of a directory it cannot list: every directory under the scratch root lists
  LINE   crates/store/src/file_write_door_tests.rs:68: continue;
      as the snapshot helper's skip
  BRANCH crates/store/src/repair_tests.rs:691:12 False: if let Ok(entries) = fs::read_dir(dir) {
      the listing helper's skip of a directory it cannot list: every directory under the scratch root lists
  LINE   crates/store/src/repair_tests.rs:700: }
      the close of the listing helper's skip
  BRANCH crates/store/src/safe_open_tests.rs:66:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
      the socket helper's fall-back to the next base when a symlink there fails (expected on a host whose temp path is too long for a socket address, as macOS's is): here the first base takes it
  LINE   crates/store/src/safe_open_tests.rs:67: tried.push(format!("{}: {why}", link.display()));
      as the socket helper's fall-back
  LINE   crates/store/src/safe_open_tests.rs:68: continue;
      as the socket helper's fall-back
  LINE   crates/store/src/safe_open_tests.rs:74: Err(why) => tried.push(format!("{}: {why}", link.display())),
      as the socket helper's fall-back, for a bind refused there
  BRANCH crates/store/src/safe_open_tests.rs:203:12 False: if let Some(device) = a_block_device() {
      the skip of the block-device row on a host with no block device; this host has one
  BRANCH crates/store/tests/catalog.rs:814:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
      the socket helper's fall-back to the next base, as safe_open_tests.rs's
  LINE   crates/store/tests/catalog.rs:815: tried.push(format!("{}: {why}", link.display()));
      as the socket helper's fall-back
  LINE   crates/store/tests/catalog.rs:816: continue;
      as the socket helper's fall-back
  LINE   crates/store/tests/catalog.rs:822: Err(why) => tried.push(format!("{}: {why}", link.display())),
      as the socket helper's fall-back, for a bind refused there
  BRANCH crates/store/tests/cited_commits.rs:288:12 True: if let Err(why) = Command::new("git").arg("--version").output() {
      the skip for a host where git cannot run; git runs on every host this suite runs on
  LINE   crates/store/tests/cited_commits.rs:289: println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
      as the no-git skip
  LINE   crates/store/tests/cited_commits.rs:290: return false;
      as the no-git skip
  BRANCH crates/store/tests/cited_commits.rs:676:8 False: if history_is_here() {
      the skip of the history checks where the checkout holds no full history (a shallow clone or no main); this checkout holds it
  BRANCH crates/store/tests/cited_commits.rs:731:8 True: if Command::new("git").arg("--version").output().is_err() {
      the synthetic-history fixture's skip for a host where git cannot run
  LINE   crates/store/tests/cited_commits.rs:732: println!("SKIPPING: git cannot run the synthetic history fixture");
      as the fixture's no-git skip
  LINE   crates/store/tests/cited_commits.rs:733: return;
      the no-git or no-history skip of its test
  BRANCH crates/store/tests/cited_commits.rs:819:8 True: if Command::new("git").arg("--version").output().is_err() {
      the synthetic-history fixture's skip for a host where git cannot run
  LINE   crates/store/tests/cited_commits.rs:820: println!("SKIPPING: git cannot run the synthetic history fixture");
      as the fixture's no-git skip
  LINE   crates/store/tests/cited_commits.rs:821: return;
      the no-git or no-history skip of its test
  BRANCH crates/store/tests/cited_commits.rs:978:8 True: if !history_is_here() {
      as the no-history skip
  LINE   crates/store/tests/cited_commits.rs:979: return;
      the no-git or no-history skip of its test
  LINE   crates/store/tests/cloud_mutants_store.rs:3253: _ => None,
      the filter's skip of a committed line that carries no time_index text: every store.append "committed" line carries it (file.rs's emit, its last field)
  BRANCH crates/store/tests/fifo.rs:212:8 True: if !null.exists() {
      the skip for a host without /dev/null; every Unix host this suite runs on has it
  LINE   crates/store/tests/fifo.rs:213: return;
      as the /dev/null skip

F-collect (30): a test's or check module's own failure-collection line, each with its proof (step 4's second clause)
  LINE   crates/store/src/flock_patient_tests.rs:364: Err(_) => refused += 1,
      a contention test's refusal count, which it asserts is zero
  LINE   crates/store/src/flock_patient_tests.rs:611: Err(_) => refused += 1,
      a contention test's refusal count, which it asserts is zero
  BRANCH crates/store/src/flock_patient_tests.rs:639:16 True: if !named {
      a held sample with no holder named, counted; the test asserts none
  LINE   crates/store/src/flock_patient_tests.rs:640: unnamed += 1;
      as the unnamed-holder count
  BRANCH crates/store/tests/cited_commits.rs:567:13 False: let Some(citing) = cited.get(commit) else {
      the check module's failure collection: a commit NOT_ON_MAIN lists that no text cites, pushed only when that check fails and reported by the assert after the walk
  LINE   crates/store/tests/cited_commits.rs:568: faults.push(format!(
      the check module's failure collection, pushed only when its check fails and reported by the assert after the walk
  LINE   crates/store/tests/cited_commits.rs:571: continue;
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:575: faults.push(format!(
      the check module's failure collection, pushed only when its check fails and reported by the assert after the walk
  LINE   crates/store/tests/cited_commits.rs:576: "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:577: at.name, at.sentence
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:578: ));
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:579: }
      the close of a failure-collection arm
  BRANCH crates/store/tests/cited_commits.rs:601:16 True: if !is_ancestor_at(root, commit, main_ref) {
      an unlisted cited commit off main: pushed only when that check fails
  LINE   crates/store/tests/cited_commits.rs:602: let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:603: faults.push(format!(
      the check module's failure collection, pushed only when its check fails and reported by the assert after the walk
  BRANCH crates/store/tests/cited_commits.rs:610:12 True: if is_ancestor_at(root, commit, main_ref) {
      a listed commit on main: pushed only when that check fails
  LINE   crates/store/tests/cited_commits.rs:611: faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
      the check module's failure collection, pushed only when its check fails and reported by the assert after the walk
  BRANCH crates/store/tests/cited_commits.rs:613:35 True: if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
      a listed commit HEAD holds: pushed only when that check fails
  LINE   crates/store/tests/cited_commits.rs:614: faults.push(format!(
      the check module's failure collection, pushed only when its check fails and reported by the assert after the walk
  LINE   crates/store/tests/cited_commits.rs:615: "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
      as the failure collection
  LINE   crates/store/tests/cited_commits.rs:616: ));
      as the failure collection
  BRANCH crates/store/tests/cited_commits.rs:996:12 True: if !takes_a_lock(line) {
      a listed bracket site that takes no lock: pushed only when that check fails
  LINE   crates/store/tests/cited_commits.rs:997: wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
      as the failure collection
  BRANCH crates/store/tests/geometry.rs:134:13 True: start < block_start || end > block_end
      a record that straddles its block, counted; the test asserts none does
  LINE   crates/store/tests/month_repair.rs:637: calls += 1;
      the counting step the test asserts is never asked (nothing is about to be promoted): its body runs only when it is asked
  LINE   crates/store/tests/month_repair.rs:638: Ok(())
      as the never-asked counting step
  LINE   crates/store/tests/month_repair.rs:639: };
      as the never-asked counting step
  BRANCH crates/store/tests/open_flags.rs:175:16 True: if names_the_x86_bit(line) {
      the check module's failure collection: a source line outside the owner naming the x86 open bit, pushed only when that check fails
  LINE   crates/store/tests/open_flags.rs:176: offenders.push(format!("{}:{}", path.display(), number + 1));
      as the failure collection
  BRANCH crates/store/tests/tix.rs:605:76 True: && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
      a log line about this test's own files, collected; the test asserts there is none

F-guard (7): a test's or check module's own fault-mock guard, each with its proof (step 4's second clause)
  BRANCH crates/store/src/emits.rs:311:12 False: if let Some(byte) = region.get_mut(16_384 + 56) {
      a fault injection's guard: region is vec![0; HEADER_BYTES] (32,768 bytes), so the flipped byte at 16,440 is always inside
  BRANCH crates/store/src/file.rs:8801:16 True: if &slot[..7] != b"BRUTEXB" {
      a fault injection's guard: the CRC-zeroing loop skips a slot with no header, and both slots of the month this test writes hold one (genesis and its commit)
  LINE   crates/store/src/file.rs:8802: continue;
      as the CRC-zeroing loop's guard
  BRANCH crates/store/src/file.rs:9217:16 False: if let Some(head) = cache.bytes.get_mut(..8) {
      a fault injection's guard: the cache buffer is MAX_BLOCK_LEN bytes, so its first 8 are always there
  BRANCH crates/store/src/sealed_tests.rs:237:12 False: if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
      the failing reader's pass-through for a read that does not reach its fail offset (a fault-mock guard): every read the test makes reaches it
  BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False: let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
      the in-memory reader's refusal of a read past its bytes (a fault-mock guard): every read the door makes of it is inside the month it holds
  LINE   crates/store/tests/cloud_mutants_store.rs:2385: return Err(std::io::ErrorKind::UnexpectedEof.into());
      as the in-memory reader's guard

F-fail (6): a test's or check module's own failing arm, each with its proof (step 4's second clause)
  LINE   crates/store/src/flock_patient_tests.rs:323: )?;
      a patient locker refused within MAX_PATIENCE returns its refusal, which the join after the threads asserts never happens
  LINE   crates/store/src/flock_tests.rs:759: matches!(
      the false outcome of a matches! the assert after it requires true
  LINE   crates/store/src/sealed_tests.rs:499: Ok(())
      an audit that passed a flipped bit: reached only when a flip goes unrefused, which the test then reports
  BRANCH crates/store/tests/cited_commits.rs:294:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound
      a .git the look cannot read (any refusal but NotFound): then git rev-parse is asked, and the assert after it fails loudly; no checkout this suite runs in has an unreadable .git
  LINE   crates/store/tests/libm_key.rs:205: matches!(
      the false outcome of refused_as_a_timestamp_fault, which every caller asserts true
  BRANCH crates/store/tests/support/mod.rs:62:15 False: let ran = output.status.success() && stdout.contains("1 passed");
      a child that passed without running exactly one test (an --exact name that matched nothing), which the assert after it refuses

(U) ITEMS, TEST CODE (14), each with its proof
  BRANCH crates/store/src/verdict.rs:4637:25 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
      the search over this test's fixed image finds a free slot below home first, so the window's upper test is never asked
  BRANCH crates/store/src/verdict.rs:4637:37 True: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
      the search over this test's fixed image finds a free slot below home first, so the window's upper test is never asked
  BRANCH crates/store/src/verdict.rs:4637:37 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
      the search over this test's fixed image finds a free slot below home first, so the window's upper test is never asked
  BRANCH crates/store/src/verdict.rs:4974:24 False: .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
      the first marked slot in the window of this test's fixed image holds the other tag, so the search never meets a marked slot of another tag
  BRANCH crates/store/tests/catalog.rs:68:12 False: if let Some(parent) = full.parent() {
      put()'s full path is root/bars/<rel>, which always has a parent
  BRANCH crates/store/tests/cited_commits.rs:306:12 False: if let Some(above) = root.parent() {
      root is the repository root, two levels above CARGO_MANIFEST_DIR, which always has a parent
  LINE   crates/store/tests/slot_audit_sealed.rs:149: Ok(file) => {
      every byte of the head's first 64 is under its CRC-32C (sealed.rs head_crc covers bytes 0..508), so every single-bit flip is refused and the arm for a byte the head does not cover is never taken
  LINE   crates/store/tests/slot_audit_sealed.rs:153: if let Ok(last) = file.read_record(39) {
      inside the arm above, which no flip reaches
  BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True: if let Ok(last) = file.read_record(39) {
      inside the arm above, which no flip reaches
  BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False: if let Ok(last) = file.read_record(39) {
      inside the arm above, which no flip reaches
  LINE   crates/store/tests/slot_audit_sealed.rs:155: }
      the close of the arm above
  BRANCH crates/store/tests/unit.rs:908:16 False: if one.magic()[..7] == other.magic()[..7]
      all four known layouts carry the family prefix BRUTEXB (format.rs MAGIC_FAMILY; MAGIC_V2, MAGIC, OVERLAY_MAGIC, GREEK_MAGIC), so the prefixes of any two are equal
  BRANCH crates/store/tests/verdict.rs:584:12 False: if count > 2 {
      every code of the 600 verdicts this test commits has more than two records, so the tail-page check is never skipped
  LINE   crates/store/tests/verdict.rs:588: }
      the close of the tail-page check, on its skipped side

MAC (10): macOS-only arms, owed to the Mac's Route B and never claimed here
  LINE   crates/store/tests/catalog.rs:308: match made {
      the macOS volume's refusal of a non-UTF-8 name (APFS, os error 92): its arm is compiled for every host and taken only on macOS
  LINE   crates/store/tests/catalog.rs:310: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
      macOS only (cfg!(target_os = "macos")): APFS refuses a non-UTF-8 name with EILSEQ (92)
  LINE   crates/store/tests/catalog.rs:311: refused_by_the_volume += 1;
      as the macOS volume's refusal
  LINE   crates/store/tests/catalog.rs:312: }
      the close of the macOS volume's refusal arm
  BRANCH crates/store/tests/catalog.rs:320:8 True: if refused_by_the_volume > 0 {
      taken only where the volume refused the names: macOS
  LINE   crates/store/tests/catalog.rs:321: eprintln!(
      as the macOS volume's note
  LINE   crates/store/tests/catalog.rs:322: "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \
      as the macOS volume's note
  LINE   crates/store/tests/catalog.rs:323: the walk is checked on what it holds and the five shapes by \
      as the macOS volume's note
  LINE   crates/store/tests/catalog.rs:324: every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"
      as the macOS volume's note
  LINE   crates/store/tests/catalog.rs:325: );
      as the macOS volume's note

TEST-CODE GAPS (0): test code that is none of the above

(F) ITEMS BY PATTERN (648): the failing side of an assert*/panic!/unreachable! invocation, or the side of an if/let-else/match arm whose block only panics, inside test code
  BRANCH crates/store/src/catalog_tests.rs:267:9 False: let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
  LINE   crates/store/src/catalog_tests.rs:268: panic!("a dangling `bars` link is refused");
  BRANCH crates/store/src/catalog_tests.rs:281:9 False: let Err(CatalogError::BarsUnreadable { because }) = locked else {
  LINE   crates/store/src/catalog_tests.rs:282: panic!("an unsearchable root is refused, not empty: {locked:?}");
  BRANCH crates/store/src/catalog_tests.rs:314:13 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:314:33 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:320:13 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:320:32 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:320:50 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:336:13 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:336:33 False: assert!(stack.is_empty() && out.held.is_empty());
  LINE   crates/store/src/catalog_tests.rs:460: path.display()
  LINE   crates/store/src/emits.rs:835: found
  LINE   crates/store/src/emits.rs:836: .records
  LINE   crates/store/src/emits.rs:837: .iter()
  LINE   crates/store/src/emits.rs:838: .map(|line| (line.target.clone(), line.message.clone()))
  LINE   crates/store/src/emits.rs:839: .collect::<Vec<_>>()
  LINE   crates/store/src/emits.rs:848: panic!(
  BRANCH crates/store/src/file_cover_tests.rs:91:9 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  BRANCH crates/store/src/file_cover_tests.rs:91:65 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  BRANCH crates/store/src/file_cover_tests.rs:118:9 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  BRANCH crates/store/src/file_cover_tests.rs:118:42 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  BRANCH crates/store/src/file_fault_tests.rs:315:57 False: Err(StoreError::Denied { path, action }) if fired => {
  LINE   crates/store/src/file_fault_tests.rs:320: other => panic!("read {skip}: {other:?}"),
  BRANCH crates/store/src/file_fault_tests.rs:334:9 False: routed >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:334:24 False: routed >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:650:21 False: reason.starts_with(&format!("{}: ", srt.display()))
  BRANCH crates/store/src/file_fault_tests.rs:651:28 False: && reason.contains("refused by the file test seam"),
  LINE   crates/store/src/file_fault_tests.rs:657: Err(other) => panic!("read {skip}: {other:?}"),
  BRANCH crates/store/src/file_fault_tests.rs:661:9 False: built >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:661:23 False: built >= 1 && refused >= 1,
  LINE   crates/store/src/file_fault_tests.rs:696: other => panic!("{}: {other:?}", looked.display()),
  BRANCH crates/store/src/file_fault_tests.rs:755:71 False: matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
  BRANCH crates/store/src/file_fault_tests.rs:845:61 False: Err(StoreError::Denied { path, action }) if fired => {
  LINE   crates/store/src/file_fault_tests.rs:869: other => panic!("{kind:?} measure {skip}: {other:?}"),
  BRANCH crates/store/src/file_fault_tests.rs:879:26 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  BRANCH crates/store/src/file_fault_tests.rs:879:42 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  BRANCH crates/store/src/file_fault_tests.rs:912:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:912:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:949:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:949:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  LINE   crates/store/src/file_index_cover_tests.rs:158: matches!(
  BRANCH crates/store/src/file_index_cover_tests.rs:574:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/file_index_cover_tests.rs:574:36 False: output.status.success() && stdout.contains("1 passed"),
  LINE   crates/store/src/file_index_cover_tests.rs:639: other => panic!("the leaf is refused by the last step: {other:?}"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:69:9 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:69:51 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:76:9 False: twice.contains("rows 900 against 900")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:77:16 False: && twice.contains("generation 2 against 1")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:78:16 False: && twice.contains("records BLAKE3 equal: true"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:92:9 False: shifted.contains("rows 900 against 900")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:93:16 False: && shifted.contains("generation 1 against 1")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:94:16 False: && shifted.contains("records BLAKE3 equal: false"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:102:9 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:102:50 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:168:13 False: refused.contains("other.sbar") && refused.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:168:47 False: refused.contains("other.sbar") && refused.contains("disagree"),
  BRANCH crates/store/src/file_sealed_tests.rs:214:69 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  BRANCH crates/store/src/file_sealed_tests.rs:214:83 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  BRANCH crates/store/src/file_sealed_tests.rs:315:13 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  BRANCH crates/store/src/file_sealed_tests.rs:315:30 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  BRANCH crates/store/src/file_sealed_tests.rs:315:50 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  LINE   crates/store/src/file_sealed_tests.rs:321: other => panic!("expected Missing naming the .bin, got {other:?}"),
  BRANCH crates/store/src/file_sealed_tests.rs:407:17 False: !reason.is_empty() && !detail.is_empty(),
  BRANCH crates/store/src/file_sealed_tests.rs:407:39 False: !reason.is_empty() && !detail.is_empty(),
  LINE   crates/store/src/file_sealed_tests.rs:419: other => panic!("a damaged frame must be refused by name, got {other:?}"),
  LINE   crates/store/src/file_sealed_tests.rs:439: other => panic!("a damaged head must be refused, got {other:?}"),
  LINE   crates/store/src/file_sealed_tests.rs:798: matches!(
  LINE   crates/store/src/file_sealed_tests.rs:803: crate::provenance::read(&root.0, path)
  BRANCH crates/store/src/file_sealed_tests.rs:824:9 False: refused.starts_with("Err(") && refused.contains("sbar"),
  BRANCH crates/store/src/file_sealed_tests.rs:824:40 False: refused.starts_with("Err(") && refused.contains("sbar"),
  BRANCH crates/store/src/file_sealed_tests.rs:874:73 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  BRANCH crates/store/src/file_sealed_tests.rs:874:92 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  LINE   crates/store/src/file_sealed_tests.rs:885: page.len()
  LINE   crates/store/src/file_sealed_tests.rs:898: page.len()
  BRANCH crates/store/src/file_write_door_tests.rs:156:13 False: let StoreError::Sealed {
  LINE   crates/store/src/file_write_door_tests.rs:165: panic!("a new bar at a sealed month is `Sealed`, got {refused}");
  LINE   crates/store/src/file_write_door_tests.rs:322: _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
  LINE   crates/store/src/file_write_door_tests.rs:439: _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
  LINE   crates/store/src/file_write_door_tests.rs:539: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:595: other => panic!("{n}: {other}"),
  LINE   crates/store/src/file_write_door_tests.rs:605: other => panic!("{n}: {other}"),
  LINE   crates/store/src/file_write_door_tests.rs:721: file.display()
  LINE   crates/store/src/file_write_door_tests.rs:749: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:765: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:809: Err(other) => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:844: matches!(
  BRANCH crates/store/src/file_write_door_tests.rs:852:13 False: let MonthRepair::Repaired {
  LINE   crates/store/src/file_write_door_tests.rs:859: panic!("{case}: {repaired:?}");
  LINE   crates/store/src/file_write_door_tests.rs:902: matches!(
  BRANCH crates/store/src/file_write_door_tests.rs:942:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  LINE   crates/store/src/file_write_door_tests.rs:943: panic!("{repaired:?}");
  LINE   crates/store/src/file_write_door_tests.rs:983: matches!(
  BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  LINE   crates/store/src/file_write_door_tests.rs:1027: panic!("{case}: {repaired:?}");
  BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False: let MonthRepair::Repaired {
  LINE   crates/store/src/file_write_door_tests.rs:1067: panic!("{case}: {repaired:?}");
  LINE   crates/store/src/file.rs:6858: parent.display()
  BRANCH crates/store/src/file.rs:7005:82 False: matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
  LINE   crates/store/src/file.rs:7062: matches!(
  LINE   crates/store/src/file.rs:7082: level.display()
  LINE   crates/store/src/file.rs:7341: matches!(
  LINE   crates/store/src/file.rs:7395: other => panic!("the flush was refused by path: {other:?}"),
  LINE   crates/store/src/file.rs:7715: matches!(
  LINE   crates/store/src/file.rs:8479: reads.get()
  LINE   crates/store/src/file.rs:8818: other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
  LINE   crates/store/src/file.rs:9036: matches!(
  LINE   crates/store/src/file.rs:9045: matches!(
  LINE   crates/store/src/file.rs:9055: matches!(
  LINE   crates/store/src/file.rs:9157: matches!(
  LINE   crates/store/src/file.rs:9200: matches!(
  BRANCH crates/store/src/file.rs:9229:13 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  BRANCH crates/store/src/file.rs:9229:45 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  BRANCH crates/store/src/file.rs:9229:73 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  LINE   crates/store/src/file.rs:9406: matches!(
  LINE   crates/store/src/file.rs:10351: matches!(
  LINE   crates/store/src/file.rs:10477: matches!(
  BRANCH crates/store/src/file.rs:10853:24 False: if *ts_micros == bar(10).ts_micros
  BRANCH crates/store/src/flock_cover_tests.rs:72:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/flock_cover_tests.rs:72:36 False: output.status.success() && stdout.contains("1 passed"),
  LINE   crates/store/src/flock_cover_tests.rs:120: matches!(
  BRANCH crates/store/src/flock_cover_tests.rs:137:17 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  BRANCH crates/store/src/flock_cover_tests.rs:137:43 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  BRANCH crates/store/src/flock_cover_tests.rs:208:9 False: !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
  BRANCH crates/store/src/flock_cover_tests.rs:208:36 False: !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
  BRANCH crates/store/src/flock_cover_tests.rs:214:9 False: whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
  BRANCH crates/store/src/flock_cover_tests.rs:215:16 False: && whole_sentence.ends_with("; pid 1 holds it"),
  LINE   crates/store/src/flock_cover_tests.rs:367: other => panic!("the host's refusal, not a wait or a lock: {other:?}"),
  LINE   crates/store/src/flock_cover_tests.rs:400: other => panic!("the host's refusal, not a hold: {other:?}"),
  BRANCH crates/store/src/flock_patient_tests.rs:101:9 False: let Impatient::Held {
  LINE   crates/store/src/flock_patient_tests.rs:109: panic!("contention, not a host refusal: {refused}");
  BRANCH crates/store/src/flock_patient_tests.rs:123:9 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  BRANCH crates/store/src/flock_patient_tests.rs:123:26 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  BRANCH crates/store/src/flock_patient_tests.rs:172:9 False: let Err(Impatient::Host(why)) = got else {
  LINE   crates/store/src/flock_patient_tests.rs:173: panic!("a host refusal: {got:?}");
  BRANCH crates/store/src/flock_patient_tests.rs:191:9 False: let Err(Impatient::Held {
  LINE   crates/store/src/flock_patient_tests.rs:195: panic!("{got:?}");
  BRANCH crates/store/src/flock_patient_tests.rs:219:62 False: matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
  BRANCH crates/store/src/flock_patient_tests.rs:261:9 False: let Err(Impatient::Held {
  LINE   crates/store/src/flock_patient_tests.rs:265: panic!("{got:?}");
  BRANCH crates/store/src/flock_patient_tests.rs:435:9 False: let Err(Impatient::Held {
  LINE   crates/store/src/flock_patient_tests.rs:439: panic!("it gave up: {refused:?}");
  BRANCH crates/store/src/flock_patient_tests.rs:565:9 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  BRANCH crates/store/src/flock_patient_tests.rs:565:51 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  LINE   crates/store/src/flock_patient_tests.rs:630: Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
  BRANCH crates/store/src/flock_patient_tests.rs:683:9 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  BRANCH crates/store/src/flock_patient_tests.rs:683:50 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  LINE   crates/store/src/flock_patient_tests.rs:687: matches!(
  LINE   crates/store/src/flock_tests.rs:273: panic!(
  LINE   crates/store/src/flock_tests.rs:304: panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
  LINE   crates/store/src/flock_tests.rs:324: panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
  LINE   crates/store/src/flock_tests.rs:343: panic!(
  LINE   crates/store/src/flock_tests.rs:380: panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
  BRANCH crates/store/src/flock_tests.rs:792:13 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  BRANCH crates/store/src/flock_tests.rs:792:38 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  BRANCH crates/store/src/flock_tests.rs:819:9 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  BRANCH crates/store/src/flock_tests.rs:819:42 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  BRANCH crates/store/src/flock_tests.rs:901:9 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  BRANCH crates/store/src/flock_tests.rs:901:51 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  LINE   crates/store/src/flock_tests.rs:984: matches!(
  BRANCH crates/store/src/flock_tests.rs:1012:9 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  BRANCH crates/store/src/flock_tests.rs:1012:66 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  BRANCH crates/store/src/generation_cover_tests.rs:125:55 False: Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
  BRANCH crates/store/src/generation_cover_tests.rs:132:55 False: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  BRANCH crates/store/src/generation_cover_tests.rs:138:55 True: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  BRANCH crates/store/src/generation_door_cover_tests.rs:99:9 False: let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
  LINE   crates/store/src/generation_door_cover_tests.rs:100: panic!("revision 1 is served");
  BRANCH crates/store/src/generation_door_cover_tests.rs:137:59 False: Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
  LINE   crates/store/src/generation_door_cover_tests.rs:156: other => panic!("the pointer read is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:181: other => panic!("the linked pointer is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:213: other => panic!("the directory's creation is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:224: other => panic!("the pointer's open is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:235: other => panic!("the pointer's lock is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:279: other => panic!("the record's write is refused: {other:?}"),
  LINE   crates/store/src/month_summary_decode_cover_tests.rs:171: other => panic!("the open is refused in the host's words: {other:?}"),
  BRANCH crates/store/src/month_summary_tests.rs:278:52 False: matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
  LINE   crates/store/src/month_summary_tests.rs:295: panic!("a read of {} waited for a writer", pipe.display());
  BRANCH crates/store/src/month_summary_tests.rs:320:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
  BRANCH crates/store/src/month_summary_tests.rs:340:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  BRANCH crates/store/src/month_summary_tests.rs:355:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  BRANCH crates/store/src/month_summary_tests.rs:377:13 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  BRANCH crates/store/src/month_summary_tests.rs:377:40 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  BRANCH crates/store/src/month_summary_tests.rs:461:32 False: Refused::Shape(why) if why.contains("flag")
  BRANCH crates/store/src/month_summary_tests.rs:467:60 False: matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
  BRANCH crates/store/src/month_summary_tests.rs:480:59 False: assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
  BRANCH crates/store/src/month_summary_tests.rs:483:63 False: assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
  BRANCH crates/store/src/month_summary_tests.rs:488:61 False: assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
  BRANCH crates/store/src/month_summary_tests.rs:491:59 False: assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
  BRANCH crates/store/src/month_summary_tests.rs:655:13 False: assert!(round.tracks_carries() && !round.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:655:39 False: assert!(round.tracks_carries() && !round.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:658:13 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:658:42 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:739:37 False: Err(Refused::Shape(why)) if why.contains("not a held minute")
  BRANCH crates/store/src/month_summary_tests.rs:757:13 False: assert!(!read.tracks_carries() && read.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:757:39 False: assert!(!read.tracks_carries() && read.wants_carries());
  LINE   crates/store/src/month_summary_tests.rs:791: other => panic!("{other:?}"),
  BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False: assert!(staging.is_dir() && !root.sum().exists());
  BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False: assert!(staging.is_dir() && !root.sum().exists());
  LINE   crates/store/src/provenance_cover_tests.rs:167: other => panic!("the pointer refuses the read: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:187: other => panic!("the sealed form refuses the read: {other:?}"),
  BRANCH crates/store/src/provenance_cover_tests.rs:204:9 False: let Found::Present(raw) = read(&root.0, june).unwrap() else {
  LINE   crates/store/src/provenance_cover_tests.rs:205: panic!("the raw sidecar holds a record");
  LINE   crates/store/src/provenance_cover_tests.rs:219: other => panic!("the sealed image's record is served: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:253: other => panic!("the open is refused by name: {other:?}"),
  BRANCH crates/store/src/provenance_cover_tests.rs:276:9 False: let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
  LINE   crates/store/src/provenance_cover_tests.rs:277: panic!("the batch's record is on disk");
  LINE   crates/store/src/provenance_cover_tests.rs:326: other => panic!("the undecodable head refuses the read: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:350: other => panic!("the lock refuses the landing: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:395: other => panic!("the refused look refuses the flush: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:425: other => panic!("the header's write refuses the landing: {other:?}"),
  BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False: matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
  LINE   crates/store/src/quarantine_cover_tests.rs:635: Ok(Level::Current(_)) => panic!("a torn image is never current"),
  LINE   crates/store/src/quarantine_cover_tests.rs:636: Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
  LINE   crates/store/src/quarantine_cover_tests.rs:643: Ok(Level::Current(_)) => panic!("a damaged header is never current"),
  LINE   crates/store/src/quarantine_cover_tests.rs:644: Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
  BRANCH crates/store/src/quarantine_tests.rs:144:9 False: let Applied::Dropped { drop, .. } = applied else {
  LINE   crates/store/src/quarantine_tests.rs:145: panic!("expected a dropped generation, got {applied:?}");
  BRANCH crates/store/src/quarantine_tests.rs:174:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
  LINE   crates/store/src/quarantine_tests.rs:175: panic!("the month is labelled");
  BRANCH crates/store/src/quarantine_tests.rs:210:63 False: assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
  BRANCH crates/store/src/quarantine_tests.rs:263:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
  LINE   crates/store/src/quarantine_tests.rs:264: panic!("the month is answered by name");
  BRANCH crates/store/src/quarantine_tests.rs:342:66 False: matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
  BRANCH crates/store/src/quarantine_tests.rs:348:77 False: assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
  BRANCH crates/store/src/quarantine_tests.rs:497:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  BRANCH crates/store/src/quarantine_tests.rs:497:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  BRANCH crates/store/src/quarantine_tests.rs:508:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  BRANCH crates/store/src/quarantine_tests.rs:508:92 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  LINE   crates/store/src/quarantine_tests.rs:551: other => panic!("expected the hold named, got {other:?}"),
  BRANCH crates/store/src/quarantine_tests.rs:799:9 False: let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
  LINE   crates/store/src/quarantine_tests.rs:800: panic!("the month is still answered by name");
  BRANCH crates/store/src/quarantine_tests.rs:855:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:855:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  LINE   crates/store/src/quarantine_tests.rs:892: other => panic!("{why}: {other:?}"),
  BRANCH crates/store/src/quarantine_tests.rs:1071:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  BRANCH crates/store/src/quarantine_tests.rs:1072:16 False: && e.bars == 6
  BRANCH crates/store/src/quarantine_tests.rs:1073:16 False: && e.causes.said() == "before_listing and foreign_level"
  LINE   crates/store/src/quarantine_tests.rs:1092: other => panic!("expected the quarantine to refuse by name, got {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:1268: other => panic!("the rerun indexes the drop: {other:?}"),
  BRANCH crates/store/src/quarantine_tests.rs:1327:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1327:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1386:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  BRANCH crates/store/src/quarantine_tests.rs:1386:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  BRANCH crates/store/src/quarantine_tests.rs:1419:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  BRANCH crates/store/src/quarantine_tests.rs:1466:38 False: Ok(Probe::Quarantined(e)) if e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1533:9 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1533:29 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1584:9 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1584:29 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1639:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1639:67 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1639:82 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1747:58 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  BRANCH crates/store/src/quarantine_tests.rs:1747:94 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  BRANCH crates/store/src/quarantine_tests.rs:1865:77 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
  LINE   crates/store/src/quarantine_tests.rs:1995: other => panic!("{other:?}"),
  BRANCH crates/store/src/quarantine_tests.rs:2261:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2261:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2261:86 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2316:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),
  LINE   crates/store/src/quarantine_tests.rs:2348: other => panic!("the lock's open refuses the act: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2365: other => panic!("the index's open refuses the act: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2546: other => panic!("the index read refuses the act: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2575: other => panic!("the ledger read is refused: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2609: other => panic!("the staged write refuses the act: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2641: other => panic!("the rename is refused: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2680: other => panic!("{point}: the probe is refused: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2730: matches!(
  LINE   crates/store/src/quarantine_tests.rs:2747: other => panic!("a refused ask is no writer: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2775: other => panic!("the lock refuses the act: {other:?}"),
  BRANCH crates/store/src/repair_cover_tests.rs:62:41 False: text.contains("refused") || text.contains("drop names"),
  LINE   crates/store/src/repair_door_cover_tests.rs:225: other @ MonthRestate::NothingNew { .. } => {
  LINE   crates/store/src/repair_door_cover_tests.rs:226: panic!("the added bars are published: {other:?}")
  LINE   crates/store/src/repair_door_cover_tests.rs:250: other @ MonthRepair::NothingNew { .. } => {
  LINE   crates/store/src/repair_door_cover_tests.rs:251: panic!("the empty month takes the bars: {other:?}")
  LINE   crates/store/src/repair_door_cover_tests.rs:327: matches!(
  LINE   crates/store/src/repair_door_cover_tests.rs:397: other => panic!("the added bar is restated: {other:?}"),
  BRANCH crates/store/src/repair_door_cover_tests.rs:428:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/repair_door_cover_tests.rs:428:36 False: output.status.success() && stdout.contains("1 passed"),
  LINE   crates/store/src/repair_door_cover_tests.rs:549: other => panic!("{operation} refuses the publication: {other:?}"),
  LINE   crates/store/src/repair_door_cover_tests.rs:588: other => panic!("the shared lock refuses the publication: {other:?}"),
  LINE   crates/store/src/repair_door_cover_tests.rs:717: other => panic!("the reservation is refused: {other:?}"),
  LINE   crates/store/src/repair_door_cover_tests.rs:752: other => panic!("the receipt's creation is refused: {other:?}"),
  LINE   crates/store/src/repair_tests.rs:116: Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
  LINE   crates/store/src/repair_tests.rs:630: timeframe.as_str()
  BRANCH crates/store/src/repair_tests.rs:824:94 False: matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
  BRANCH crates/store/src/repair_tests.rs:882:9 False: let super::MonthRestate::Restated {
  LINE   crates/store/src/repair_tests.rs:891: panic!("expected a restated generation, got {done:?}");
  LINE   crates/store/src/safe_open_tests.rs:77: panic!(
  LINE   crates/store/src/safe_open_tests.rs:79: path.display(),
  LINE   crates/store/src/safe_open_tests.rs:80: tried.join("; ")
  BRANCH crates/store/src/safe_open_tests.rs:95:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  LINE   crates/store/src/safe_open_tests.rs:103: found.file_type()
  LINE   crates/store/src/safe_open_tests.rs:110: link.display()
  LINE   crates/store/src/safe_open_tests.rs:124: panic!("an open of {} waited for a writer", pipe.display());
  BRANCH crates/store/src/safe_open_tests.rs:309:16 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  BRANCH crates/store/src/safe_open_tests.rs:309:35 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  BRANCH crates/store/src/safe_open_tests.rs:310:20 False: && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
  LINE   crates/store/src/safe_open_tests.rs:412: Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
  LINE   crates/store/src/safe_open_tests.rs:413: Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
  BRANCH crates/store/src/safe_open_tests.rs:470:77 False: matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
  LINE   crates/store/src/sealed_codec_tests.rs:81: frame.len()
  LINE   crates/store/src/sealed_codec_tests.rs:282: frame.len()
  LINE   crates/store/src/sealed_codec_tests.rs:439: largest[rows],
  LINE   crates/store/src/sealed_codec_tests.rs:440: max_frame_len(m)
  BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  LINE   crates/store/src/sealed_tests.rs:159: Ok(value) => panic!("expected a refusal, got {value:?}"),
  LINE   crates/store/src/sealed_tests.rs:502: Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
  LINE   crates/store/src/sealed_tests.rs:653: other => panic!("{other}"),
  BRANCH crates/store/src/sealed_tests.rs:696:27 False: found == 0 || bars[found as usize - 1].ts_micros < ts,
  LINE   crates/store/src/sealed_tests.rs:837: other => panic!("{field}: {other}"),
  LINE   crates/store/src/sealed_tests.rs:1171: packed.len(),
  LINE   crates/store/src/sealed_tests.rs:1172: image.len()
  LINE   crates/store/src/sealed_tests.rs:1193: other => panic!("{other:?}"),
  LINE   crates/store/src/sealed_tests.rs:1242: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1320: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1657: other => panic!("{field}: {other}"),
  LINE   crates/store/src/sealed_tests.rs:1674: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1690: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1775: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1803: other => panic!("{other}"),
  BRANCH crates/store/src/sealed_tests.rs:1817:100 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
  BRANCH crates/store/src/sealed_tests.rs:1823:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  BRANCH crates/store/src/sealed_tests.rs:1829:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
  BRANCH crates/store/src/sealed_tests.rs:1837:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  LINE   crates/store/src/sealed_tests.rs:1892: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1983: matches!(
  LINE   crates/store/src/sealed_tests.rs:2105: other => panic!("{other}"),
  BRANCH crates/store/src/settings_cover_tests.rs:198:57 False: Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
  BRANCH crates/store/src/settings_door_cover_tests.rs:97:9 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:97:47 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  LINE   crates/store/src/settings_door_cover_tests.rs:128: other => panic!("the listing refuses the save: {other:?}"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:162:9 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:162:51 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:235:9 False: fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:235:60 False: fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
  LINE   crates/store/src/settings_door_cover_tests.rs:261: other => panic!("the taken name refuses the save: {other:?}"),
  LINE   crates/store/src/settings_door_cover_tests.rs:288: other => panic!("the refused write refuses the save: {other:?}"),
  LINE   crates/store/src/settings_door_cover_tests.rs:324: other => panic!("the refused re-list is the store's fault: {other:?}"),
  LINE   crates/store/src/settings_door_cover_tests.rs:377: other => panic!("the vanished temporary refuses the link: {other:?}"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:412:9 False: read_back.contains("did not read back")
  BRANCH crates/store/src/settings_door_cover_tests.rs:413:16 False: && read_back.ends_with("it reads when read again, and the refresh did not step to it"),
  LINE   crates/store/src/settings_tests.rs:152: other => panic!("{bad:?} for {key:?}: {other:?}"),
  LINE   crates/store/src/settings_tests.rs:228: matches!(
  LINE   crates/store/src/settings_tests.rs:417: reader.faults()
  LINE   crates/store/src/settings_tests.rs:583: reader.faults()
  LINE   crates/store/src/settings_tests.rs:654: reader.stopped()
  LINE   crates/store/src/settings_tests.rs:715: panic!("{what}: the settings read waited instead of naming a fault");
  BRANCH crates/store/src/settings_tests.rs:789:48 False: matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
  BRANCH crates/store/src/settings_tests.rs:913:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
  BRANCH crates/store/src/settings_tests.rs:959:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
  LINE   crates/store/src/settings_tests.rs:993: matches!(
  BRANCH crates/store/src/settings_tests.rs:1100:9 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  BRANCH crates/store/src/settings_tests.rs:1100:52 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  BRANCH crates/store/src/settings_tests.rs:1190:13 True: a.superseded_by == Some(3) || a.in_force == Some(2),
  BRANCH crates/store/src/settings_tests.rs:1190:43 False: a.superseded_by == Some(3) || a.in_force == Some(2),
  BRANCH crates/store/src/small_month_tests.rs:152:52 False: Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
  BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False: Err(Why::Unreadable(why)) if why.contains("is not a regular file")
  BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False: Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
  BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False: Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
  BRANCH crates/store/src/sort_index_tests.rs:324:41 False: assert!(why.contains("kind") || why.contains("reserved"), "{why}");
  LINE   crates/store/src/staging_cover_tests.rs:56: Err(other) => panic!("the removal is refused by name: {other:?}"),
  LINE   crates/store/src/staging_cover_tests.rs:57: Ok(_) => panic!("the removal is refused by name, not staged"),
  BRANCH crates/store/src/staging_tests.rs:143:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:143:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:206:67 False: matches!(&refusal, StagingError::Contended { path: p } if *p == path),
  BRANCH crates/store/src/staging_tests.rs:238:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:238:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:297:78 False: matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
  BRANCH crates/store/src/staging_tests.rs:334:88 False: matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
  BRANCH crates/store/src/staging_tests.rs:369:77 False: matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
  LINE   crates/store/src/staging_tests.rs:421: within.display()
  BRANCH crates/store/src/time_index.rs:921:21 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  BRANCH crates/store/src/time_index.rs:921:41 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  BRANCH crates/store/src/time_index.rs:1057:17 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1057:38 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1057:59 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1058:17 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  BRANCH crates/store/src/time_index.rs:1058:39 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  BRANCH crates/store/src/time_index.rs:1105:17 False: assert!(!daily.exact() && g.exact());
  BRANCH crates/store/src/time_index.rs:1105:35 False: assert!(!daily.exact() && g.exact());
  LINE   crates/store/src/verdict_tests.rs:366: other => panic!("expected the host's refusal, got {other}"),
  LINE   crates/store/src/verdict_tests.rs:462: panic!("a verdict call waited on the pipe at {}", pipe.display());
  LINE   crates/store/src/verdict_tests.rs:568: other => panic!("expected the host's refusal, got {other}"),
  LINE   crates/store/src/verdict_tests.rs:813: other => panic!("not image_corrupt: {other:?}"),
  LINE   crates/store/src/verdict_tests.rs:879: other.dir().display()
  BRANCH crates/store/src/verdict_tests.rs:950:9 False: let super::Found::Record { ordinal, record } = found.found else {
  LINE   crates/store/src/verdict_tests.rs:951: panic!("the key's record: {:?}", found.found);
  LINE   crates/store/src/verdict.rs:4219: other => panic!("not image_corrupt: {other:?}"),
  LINE   crates/store/src/verdict.rs:4227: other => panic!("not written_by_newer_build: {other:?}"),
  LINE   crates/store/tests/catalog.rs:313: Err(why) => panic!("{} could not be made: {why}", full.display()),
  BRANCH crates/store/tests/catalog.rs:317:9 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  BRANCH crates/store/tests/catalog.rs:317:39 True: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  BRANCH crates/store/tests/catalog.rs:317:39 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  LINE   crates/store/tests/catalog.rs:825: panic!(
  LINE   crates/store/tests/catalog.rs:827: path.display(),
  LINE   crates/store/tests/catalog.rs:828: tried.join("; ")
  BRANCH crates/store/tests/catalog.rs:844:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  LINE   crates/store/tests/catalog.rs:852: found.file_type()
  LINE   crates/store/tests/catalog.rs:859: link.display()
  LINE   crates/store/tests/catalog.rs:875: path.display()
  BRANCH crates/store/tests/cited_commits.rs:371:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  BRANCH crates/store/tests/cited_commits.rs:372:16 False: && said.contains("not a git repository"),
  BRANCH crates/store/tests/cited_commits.rs:384:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  BRANCH crates/store/tests/cited_commits.rs:385:16 False: && said.contains("not a git repository"),
  LINE   crates/store/tests/cited_commits.rs:673: cited.len()
  LINE   crates/store/tests/cited_commits.rs:709: String::from_utf8_lossy(&out.stderr)
  LINE   crates/store/tests/cited_commits.rs:976: sites.len()
  BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False: vanished >= 20 && contended >= 20,
  BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False: vanished >= 20 && contended >= 20,
  BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False: } if source.kind() == std::io::ErrorKind::PermissionDenied
  LINE   crates/store/tests/cloud_mutants_store.rs:367: other => panic!("{door}: {other:?}"),
  BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False: let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
  LINE   crates/store/tests/cloud_mutants_store.rs:371: panic!("a moved revision is audited");
  BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False: audited.starts_with("repair revision 1 of ")
  BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False: && audited.ends_with(" is not the one its promotion recorded; not audited"),
  LINE   crates/store/tests/cloud_mutants_store.rs:446: other => panic!("{other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1073: other => panic!("not image_corrupt: {other:?}"),
  BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False: let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
  LINE   crates/store/tests/cloud_mutants_store.rs:1098: panic!("ZZB is in the day");
  LINE   crates/store/tests/cloud_mutants_store.rs:1171: matches!(
  BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False: generation.contains("/nonexistent/cloud/2031-02.tix")
  BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False: && generation.contains("revision 1 is not the one its promotion recorded"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1361: other => panic!("{other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1568: other => panic!("{other:?}"),
  BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False: if path == &lock_path && *tries == steps + 1
  BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False: if path == &lock_path && *tries == steps + 1
  BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False: let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
  LINE   crates/store/tests/cloud_mutants_store.rs:1925: panic!("{name} is in the day");
  BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False: if path.parent() == Some(year.as_path())
  BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False: && path.extension().is_some_and(|ext| ext == "tmp")
  BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False: && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False: TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
  BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False: if *path == bar_path
  BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False: TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
  LINE   crates/store/tests/cloud_mutants_store.rs:2876: writer.time_lookup()
  BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False: StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
  BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  BRANCH crates/store/tests/docs.rs:220:9 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  BRANCH crates/store/tests/docs.rs:220:47 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  BRANCH crates/store/tests/docs.rs:234:9 False: entry.contains(
  BRANCH crates/store/tests/docs.rs:236:14 False: ) && entry.contains("this entry supersedes that sentence"),
  BRANCH crates/store/tests/docs.rs:253:9 False: FILE_RS.contains("if claimed > header.n_valid {")
  BRANCH crates/store/tests/docs.rs:254:16 False: && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
  LINE   crates/store/tests/durability.rs:381: panic!(
  BRANCH crates/store/tests/fault.rs:143:36 False: read.n_valid == 150 || read.n_valid == 175,
  BRANCH crates/store/tests/fault.rs:220:59 False: horizon == previous_commit.durable_through || horizon == commit.durable_through,
  LINE   crates/store/tests/fault.rs:376: panic!("bit {bit} of byte {byte} took the whole file down: {e}")
  LINE   crates/store/tests/fifo.rs:108: at.display()
  LINE   crates/store/tests/fifo.rs:128: panic!(
  LINE   crates/store/tests/fifo.rs:130: release.display()
  BRANCH crates/store/tests/fifo.rs:157:13 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  BRANCH crates/store/tests/fifo.rs:157:61 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  BRANCH crates/store/tests/fifo.rs:162:53 False: fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
  BRANCH crates/store/tests/generation.rs:153:9 False: let MonthRepair::Repaired {
  LINE   crates/store/tests/generation.rs:165: panic!("{done:?}");
  BRANCH crates/store/tests/generation.rs:179:57 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  BRANCH crates/store/tests/generation.rs:179:76 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  LINE   crates/store/tests/generation.rs:217: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:238: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:331: matches!(
  BRANCH crates/store/tests/generation.rs:364:9 False: let MonthRepair::Repaired { to, skipped, .. } =
  LINE   crates/store/tests/generation.rs:367: panic!("repaired");
  BRANCH crates/store/tests/generation.rs:384:9 False: let MonthRepair::Repaired {
  LINE   crates/store/tests/generation.rs:388: panic!("{second:?}");
  LINE   crates/store/tests/generation.rs:475: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:517: other => panic!("{other:?}"),
  BRANCH crates/store/tests/generation.rs:670:13 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:670:42 True: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:670:42 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:694:74 False: matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
  BRANCH crates/store/tests/generation.rs:749:9 False: let MonthRepair::Repaired {
  LINE   crates/store/tests/generation.rs:758: panic!("{repaired:?}");
  BRANCH crates/store/tests/geometry.rs:99:13 False: start >= block_start && end <= block_end,
  BRANCH crates/store/tests/geometry.rs:99:37 False: start >= block_start && end <= block_end,
  BRANCH crates/store/tests/libm_key.rs:428:9 False: decision < caveat && caveat < rejected,
  BRANCH crates/store/tests/libm_key.rs:428:30 False: decision < caveat && caveat < rejected,
  LINE   crates/store/tests/libm_key.rs:1330: if mentions { "is not" } else { "is" }
  BRANCH crates/store/tests/libm_key.rs:1330:16 True: if mentions { "is not" } else { "is" }
  BRANCH crates/store/tests/libm_key.rs:1330:16 False: if mentions { "is not" } else { "is" }
  LINE   crates/store/tests/misfiled.rs:184: other => panic!("expected Misfiled, found {other:?}"),
  BRANCH crates/store/tests/misfiled.rs:217:9 False: said.contains("2019-11-28-1250000-CE")
  BRANCH crates/store/tests/misfiled.rs:218:16 False: && said.contains("its store path is not this month's")
  BRANCH crates/store/tests/misfiled.rs:219:16 False: && said.contains("DB-B65"),
  LINE   crates/store/tests/misfiled.rs:321: unborn.len()
  LINE   crates/store/tests/month_repair.rs:156: other => panic!("not a repair: {other:?}"),
  LINE   crates/store/tests/month_repair.rs:324: timeframe.as_str()
  BRANCH crates/store/tests/month_repair.rs:606:13 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  BRANCH crates/store/tests/month_repair.rs:606:47 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  BRANCH crates/store/tests/month_repair.rs:739:83 False: matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
  BRANCH crates/store/tests/month_repair.rs:852:9 False: let Ok(MonthRepair::Repaired {
  LINE   crates/store/tests/month_repair.rs:856: panic!("{outcome:?}");
  BRANCH crates/store/tests/month_repair.rs:879:38 False: Served::Revision(pointer) if pointer.revision.ordinal() == 3
  LINE   crates/store/tests/month_repair.rs:918: left.keys()
  LINE   crates/store/tests/month_repair.rs:944: other => panic!("not a repair: {other:?}"),
  LINE   crates/store/tests/provenance.rs:66: other => panic!("expected a sidecar, found {other:?}"),
  LINE   crates/store/tests/provenance.rs:161: Recorded::NothingAppended => panic!("expected a record"),
  LINE   crates/store/tests/provenance.rs:451: other => panic!("expected NotTheNewestCommit, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:464: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:476: other => panic!("expected WrongBarFile, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:531: other => panic!("expected OutOfOrder, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:639: other => panic!("expected Damaged, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:698: result => panic!("expected a {field} mismatch, got {result:?}"),
  LINE   crates/store/tests/provenance.rs:746: other => panic!("expected PastTheBars, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1200: other => panic!("expected {fault:?}, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1308: other => panic!("expected TooLarge, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1409: other => panic!("expected Record, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1611: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1617: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1671: other => panic!("expected the directory sync refused, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1721: other => panic!("expected the open refused, got {other:?}"),
  BRANCH crates/store/tests/repair.rs:506:13 True: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  BRANCH crates/store/tests/repair.rs:506:41 False: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False: let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
  LINE   crates/store/tests/sealed_codec_alloc.rs:115: panic!("a frame cut to {len} bytes left {frame_answer:?}");
  BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  LINE   crates/store/tests/sealed_codec_alloc.rs:125: full.len()
  BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False: !text.contains("by its checked rows and no more")
  BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False: && !text.contains("checked rows times 56 bytes, once"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False: text.contains("the largest of twice") && text.contains("and 4"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False: text.contains("the largest of twice") && text.contains("and 4"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False: text.contains("capacity 0") && text.contains("cleared"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False: text.contains("capacity 0") && text.contains("cleared"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False: !text.contains("random fields cost at most 57 bytes a bar")
  BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False: && !text.contains("at most 57 bytes a bar, 1 more than raw"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  BRANCH crates/store/tests/sealed_door.rs:153:17 True: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  BRANCH crates/store/tests/sealed_door.rs:153:31 False: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  BRANCH crates/store/tests/sealed_door.rs:231:9 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  BRANCH crates/store/tests/sealed_door.rs:231:45 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  LINE   crates/store/tests/slot_audit_sealed.rs:95: raw.slot_audit()
  LINE   crates/store/tests/slot_audit_sealed.rs:150: assert_eq!(file.form(), Form::Sealed, "byte {at}");
  LINE   crates/store/tests/slot_audit_sealed.rs:151: assert_eq!(file.slot_audit(), None, "byte {at}");
  LINE   crates/store/tests/slot_audit_sealed.rs:152: assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
  LINE   crates/store/tests/slot_audit_sealed.rs:154: assert_eq!(last.close, 7_039, "byte {at}");
  LINE   crates/store/tests/slot_audit.rs:116: matches!(
  BRANCH crates/store/tests/slot_audit.rs:130:13 False: let MonthVerdict::Reverted {
  LINE   crates/store/tests/slot_audit.rs:137: panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
  LINE   crates/store/tests/slot_audit.rs:277: matches!(
  LINE   crates/store/tests/slot_audit.rs:347: matches!(
  LINE   crates/store/tests/slot_audit.rs:410: matches!(
  BRANCH crates/store/tests/slot_audit.rs:484:9 False: let SlotState::Damaged { reason, .. } = audit.slots[1] else {
  LINE   crates/store/tests/slot_audit.rs:485: panic!("damaged");
  LINE   crates/store/tests/slot_damaged_log.rs:94: other => panic!("{key}: {other:?}"),
  LINE   crates/store/tests/slot_damaged_log.rs:101: other => panic!("{key}: {other:?}"),
  LINE   crates/store/tests/slot_damaged_log.rs:192: found
  LINE   crates/store/tests/slot_damaged_log.rs:193: .records
  LINE   crates/store/tests/slot_damaged_log.rs:194: .iter()
  LINE   crates/store/tests/slot_damaged_log.rs:195: .map(|r| (r.target.clone(), r.message.clone()))
  LINE   crates/store/tests/slot_damaged_log.rs:196: .collect::<Vec<_>>()
  BRANCH crates/store/tests/sort_index.rs:387:62 False: matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
  BRANCH crates/store/tests/sort_index.rs:430:56 False: matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
  BRANCH crates/store/tests/sort_index.rs:446:63 False: assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
  LINE   crates/store/tests/sort_index.rs:488: index.torn()
  BRANCH crates/store/tests/sort_index.rs:660:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  BRANCH crates/store/tests/sort_index.rs:736:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  LINE   crates/store/tests/sort_index.rs:761: column.as_str()
  LINE   crates/store/tests/sort_index.rs:843: column.as_str()
  BRANCH crates/store/tests/sort_index.rs:874:16 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  BRANCH crates/store/tests/sort_index.rs:874:54 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  BRANCH crates/store/tests/sort_index.rs:889:16 False: if reason.contains("2031-02.new.srt")),
  BRANCH crates/store/tests/tail_proof.rs:144:9 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  BRANCH crates/store/tests/tail_proof.rs:144:36 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  BRANCH crates/store/tests/tail_proof.rs:144:60 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False: }) if asked == SYMBOL + 1
  LINE   crates/store/tests/tix_small_month.rs:160: other => panic!("not committed: {other:?}"),
  BRANCH crates/store/tests/tix_small_month.rs:461:74 False: matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
  LINE   crates/store/tests/tix_small_month.rs:463: reader.time_lookup()
  LINE   crates/store/tests/tix_warn_once.rs:127: other => panic!("{key} is not a count: {other:?}"),
  LINE   crates/store/tests/tix_warn_once.rs:160: other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
  LINE   crates/store/tests/tix_warn_once.rs:162: other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
  LINE   crates/store/tests/tix.rs:173: timeframe.as_str()
  LINE   crates/store/tests/unit.rs:110: input.len()
  BRANCH crates/store/tests/unit.rs:399:13 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  BRANCH crates/store/tests/unit.rs:399:36 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  LINE   crates/store/tests/unit.rs:527: _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
  BRANCH crates/store/tests/unit.rs:868:13 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  BRANCH crates/store/tests/unit.rs:868:54 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  BRANCH crates/store/tests/unit.rs:1475:32 False: read == old || read == new,
  LINE   crates/store/tests/unit.rs:1820: matches!(
  LINE   crates/store/tests/unit.rs:1859: matches!(
  LINE   crates/store/tests/unit.rs:1874: matches!(
  BRANCH crates/store/tests/unit.rs:2047:26 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  BRANCH crates/store/tests/unit.rs:2047:43 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  LINE   crates/store/tests/unit.rs:2096: vendor.as_str(),
  BRANCH crates/store/tests/unit.rs:2173:13 False: err.to_string().contains(field) && err.to_string().contains("case"),
  BRANCH crates/store/tests/unit.rs:2173:48 False: err.to_string().contains(field) && err.to_string().contains("case"),
  LINE   crates/store/tests/unit.rs:2826: a.as_str(),
  LINE   crates/store/tests/unit.rs:2827: b.as_str()
  LINE   crates/store/tests/unit.rs:2833: a.as_str()
  LINE   crates/store/tests/unit.rs:2893: tf.as_str()
  LINE   crates/store/tests/verdict.rs:129: Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
  LINE   crates/store/tests/verdict.rs:168: other => panic!("100x must be refused table_too_large, got {other:?}"),
  BRANCH crates/store/tests/verdict.rs:228:13 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  BRANCH crates/store/tests/verdict.rs:228:36 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  LINE   crates/store/tests/verdict.rs:395: other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
  BRANCH crates/store/tests/verdict.rs:449:13 False: let Found::Record { ordinal, record } = &answer.found else {
  LINE   crates/store/tests/verdict.rs:450: panic!("{whole} must be found");
  LINE   crates/store/tests/verdict.rs:728: other => panic!("expected image_missing, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:793: other => panic!("expected generation_collision, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:1318: other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:1821: other => panic!(
  LINE   crates/store/tests/verdict.rs:1823: clean.len()
  LINE   crates/store/tests/verdict.rs:1856: other => panic!("expected record_corrupt, got {other:?}"),
  BRANCH crates/store/tests/verdict.rs:1890:17 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  BRANCH crates/store/tests/verdict.rs:1890:40 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  BRANCH crates/store/tests/verdict.rs:2123:17 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  BRANCH crates/store/tests/verdict.rs:2123:40 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  LINE   crates/store/tests/verdict.rs:2283: feed.as_str(),
  LINE   crates/store/tests/verdict.rs:2284: timeframe.as_str()
  BRANCH crates/store/tests/verdict.rs:2358:58 False: matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
  LINE   crates/store/tests/verdict.rs:2435: other => panic!("expected lock_stalled, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:2455: matches!(
  BRANCH crates/store/tests/verdict.rs:2545:21 False: image.extension().is_some_and(|ext| ext == "vdt")
  BRANCH crates/store/tests/verdict.rs:2546:28 False: && image.parent().is_some_and(|dir| dir.ends_with("2031"))
  BRANCH crates/store/tests/verdict.rs:2547:28 False: && image.exists(),
  LINE   crates/store/tests/verdict.rs:2549: image.display()
  LINE   crates/store/tests/verdict.rs:2554: other => panic!("expected index_missing, got {other}"),
  BRANCH crates/store/tests/verdict.rs:2604:13 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  BRANCH crates/store/tests/verdict.rs:2604:22 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  LINE   crates/store/tests/verdict.rs:2730: other => panic!("{other}"),
  BRANCH crates/store/tests/verdict.rs:2782:9 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  BRANCH crates/store/tests/verdict.rs:2782:61 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  LINE   crates/store/tests/write.rs:1402: sidecar.display()
  LINE   crates/store/tests/write.rs:1419: sidecar.display()
  BRANCH crates/store/tests/write.rs:1564:13 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  BRANCH crates/store/tests/write.rs:1564:62 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  LINE   crates/store/tests/write.rs:1619: panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
  LINE   crates/store/tests/write.rs:1663: panic!(
  BRANCH crates/store/tests/write.rs:2103:85 False: Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars

APPENDIX LS2: the fenced content of reports/base-cloud-20261103/fu-LS2.txt.md on branch cloud/cov-libs-ls2-20261103, verbatim
(F) and (U) items of runner on tip 473f62e05b5b7220ad3c267a2a8e066a51c1188f (recipe v4 step 4). (F) rows: the failing arm of assert!/assert_eq!/assert_ne!/panic!/unreachable! inside #[cfg(test)] code or tests/, the message argument of such an assert, a let-else or if whose block is only a panic, and the failing side of an assert! condition operand (the False side of a conjunction's operand, or of a disjunction's last operand); rows on lines unchanged since 2b998e4b5f6e keep the earlier session's (F) classification, and this session classified the rest by the same rule. (U) items were each read and proved by hand from the code, for aarch64-apple-darwin as for this host (no proof below depends on the target). A (U) item names its rows as file:line[:column side]; gaps-LS2.txt.md tags each row it covers [U]. The first 76 (U) paragraphs are the earlier session's, carried unchanged (the code they read is unchanged: this session changed test code only), and they name lines as at 2b998e4b5f6e. Those numbers hold at 473f62e05b5b in every file but one: in crates/runner/src/exit_grid_policy.rs each line from 679 to 5862 is one higher at 473f62e05b5b (this session's test edit at 676-678, inside mod subslice_tests, added one line; the lines after 5862 are test code, which no proof names), so a line N of that range named in an earlier paragraph is N+1 here, and each [U] row of gaps-LS2.txt.md whose proof names its earlier line says so. (grid.rs's thin_to_budget, after its test module, also moved; no row or proof names it.) The last 11 paragraphs are this session's and name lines as at 473f62e05b5b.

(F) ITEMS: 274
(F) LINE   crates/runner/src/admission.rs:7423  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7429  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7435  [assert_eq! message argument]  reason.name()
(F) LINE   crates/runner/src/admission.rs:7466  [assert_eq! message argument]  left.name(),
(F) LINE   crates/runner/src/admission.rs:7467  [assert_eq! message argument]  right.name()
(F) BRANCH crates/runner/src/admission_reader_cover_tests.rs:285:70 False  [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F) BRANCH crates/runner/src/admission_reader_cover_tests.rs:289:70 False  [assert! condition branch (failing side)]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F) BRANCH crates/runner/src/audit.rs:2224:13 False  [assert! condition branch (failing side)]  ranked
(F) BRANCH crates/runner/src/audit.rs:2227:20 False  [assert! condition branch (failing side)]  && ranked.ends_with("what selection ranks on, gross of every charge"),
(F) BRANCH crates/runner/src/audit.rs:2358:44 False  [assert! condition branch (failing side): the False side of a disjunction's last operand]  line.is_empty() || line.starts_with("  "),
(F) BRANCH crates/runner/src/audit.rs:2467:29 False  [assert! condition branch (failing side): the False side of a disjunction's last operand]  decision || token == "V6",
(F) BRANCH crates/runner/src/audit.rs:2620:40 False  [assert! condition branch (failing side): the False side of a disjunction's last operand]  line.is_empty() || line.starts_with("  "),
(F) BRANCH crates/runner/src/audit.rs:2788:13 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:2788:54 False  [assert! condition branch (failing side)]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F) BRANCH crates/runner/src/audit.rs:3067:17 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3067:35 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3067:54 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3666:17 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3666:35 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) BRANCH crates/runner/src/audit.rs:3666:54 False  [assert! condition branch (failing side)]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F) LINE   crates/runner/src/bootstrap.rs:2108  [assert! message argument]  v.calibration()
(F) LINE   crates/runner/src/bootstrap.rs:2188  [assert! message argument]  (n as f64).sqrt()
(F) LINE   crates/runner/src/bootstrap.rs:2722  [assert! message argument]  draw.runs.len()
(F) LINE   crates/runner/src/bootstrap.rs:3411  [assert! message argument]  rounds + 1,
(F) LINE   crates/runner/src/bootstrap.rs:3412  [assert! message argument]  rejected.len(),
(F) LINE   crates/runner/src/bootstrap.rs:3413  [assert! message argument]  set.len()
(F) LINE   crates/runner/src/bootstrap.rs:3520  [assert! message argument]  rc.map(|v| v.p_value)
(F) LINE   crates/runner/src/bootstrap.rs:3526  [assert! message argument]  v.map(|v| (v.p_value, v.clears()))
(F) BRANCH crates/runner/src/bootstrap.rs:2435:13 False  [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),
(F) BRANCH crates/runner/src/bootstrap.rs:2435:36 False  [assert! condition branch (failing side)]  names.contains(&18) && names.contains(&19),
(F) BRANCH crates/runner/src/bootstrap.rs:2652:17 False  [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F) BRANCH crates/runner/src/bootstrap.rs:2652:29 False  [assert! condition branch (failing side)]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F) BRANCH crates/runner/src/bootstrap.rs:3409:13 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap.rs:3409:40 False  [assert! condition branch (failing side)]  rounds < rejected.len() && rejected.len() <= set.len(),
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:13 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:28 False  [assert! condition branch (failing side)]  matched > 0 && matched < draws,
(F) LINE   crates/runner/src/excursion.rs:1682  [assert! message argument]  tightest(8),
(F) LINE   crates/runner/src/excursion.rs:1683  [assert! message argument]  tightest(4)
(F) LINE   crates/runner/src/excursion.rs:1688  [assert! message argument]  tightest(4),
(F) LINE   crates/runner/src/excursion.rs:1689  [assert! message argument]  tightest(2)
(F) LINE   crates/runner/src/excursion.rs:2582  [assert_eq! message argument]  sample.len()
(F) LINE   crates/runner/src/exit_grid_policy.rs:677  [unreachable!/panic! in test code (failing arm)]  unreachable!("the copy holds bars 17..250")
(F) LINE   crates/runner/src/exit_grid_policy.rs:7843  [unreachable! (failing arm)]  unreachable!("the fixture prices every path")
(F) LINE   crates/runner/src/exit_grid_policy.rs:7985  [assert! message argument]  universe.candidates.len()
(F) BRANCH crates/runner/src/exit_grid_policy.rs:676:13 False  [let-else whose block is only unreachable!]  let Some(last) = forged.last_mut() else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7842:13 False  [let-else whose block is only unreachable!]  let crate::grid::ReplayPathV1::Priceable(price) = &mut candidate.path else {
(F) BRANCH crates/runner/src/exit_grid_policy.rs:7989:13 False  [assert! condition branch (failing side)]  first.entry_bar > 0 && matches!(first.path, crate::grid::ReplayPathV1::Priceable(_))
(F) BRANCH crates/runner/src/exit_grid_policy.rs:8754:17 False  [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F) BRANCH crates/runner/src/exit_grid_policy.rs:8754:44 False  [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F) BRANCH crates/runner/src/exit_grid_policy.rs:8763:17 False  [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(F) BRANCH crates/runner/src/exit_grid_policy.rs:8763:45 False  [assert! condition branch (failing side)]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(F) LINE   crates/runner/src/grid.rs:7088  [assert! message argument]  stopped.edge_ratio(),
(F) LINE   crates/runner/src/grid.rs:7089  [assert! message argument]  stopless.edge_ratio()
(F) LINE   crates/runner/src/grid.rs:8100  [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  !matches!(
(F) LINE   crates/runner/src/grid.rs:8380  [assert! message argument]  c.uncertainty()
(F) LINE   crates/runner/src/grid.rs:9814  [assert! message argument]  a.reward_to_risk_bp()
(F) LINE   crates/runner/src/grid.rs:10283  [assert! message argument]  sniper.assurance_bp(),
(F) LINE   crates/runner/src/grid.rs:10284  [assert! message argument]  grinder.assurance_bp()
(F) LINE   crates/runner/src/grid.rs:10290  [assert! message argument]  grinder.assurance_bp(),
(F) LINE   crates/runner/src/grid.rs:10291  [assert! message argument]  fluke.assurance_bp()
(F) LINE   crates/runner/src/grid.rs:10330  [assert! message argument]  cell.win_rate_bp()
(F) LINE   crates/runner/src/grid.rs:10364  [assert! message argument]  trades - 1
(F) LINE   crates/runner/src/grid.rs:11034  [assert! message argument]  t.eligible.len(),
(F) LINE   crates/runner/src/grid.rs:11035  [assert! message argument]  t.trades.len()
(F) BRANCH crates/runner/src/grid.rs:6254:13 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6254:35 False  [assert! condition branch (failing side)]  !replay.is_empty() && !replay.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:13 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:6275:36 False  [assert! condition branch (failing side)]  !in_loop.is_empty() && !in_loop.contains(built),
(F) BRANCH crates/runner/src/grid.rs:7982:13 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:7982:39 False  [assert! condition branch (failing side)]  !long.cells.is_empty() && !short.cells.is_empty(),
(F) BRANCH crates/runner/src/grid.rs:8194:17 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid.rs:8194:40 False  [assert! condition branch (failing side)]  assert!(base.stop.is_none() && base.target.is_none());
(F) BRANCH crates/runner/src/grid_cover_tests.rs:59:13 False  [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F) BRANCH crates/runner/src/grid_cover_tests.rs:59:27 False  [assert! condition branch (failing side)]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F) LINE   crates/runner/src/lib.rs:2028  [assert_eq! message argument]  reads.len()
(F) BRANCH crates/runner/src/lib.rs:2032:33 False  [assert! condition branch (failing side): the False side of a conjunction's operand, through is_some_and]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
(F) LINE   crates/runner/src/outcome.rs:3153  [assert! message argument]  column.bits().len()
(F) LINE   crates/runner/src/outcome.rs:3243  [assert! message argument]  minute_of(i)
(F) LINE   crates/runner/src/outcome.rs:3318  [assert! message argument]  bars.len()
(F) LINE   crates/runner/src/outcome.rs:3383  [assert_eq! message argument]  forced_minute - 1
(F) LINE   crates/runner/src/outcome.rs:3413  [assert_eq! message argument]  forced_minute - 1,
(F) LINE   crates/runner/src/outcome.rs:3414  [assert_eq! message argument]  forced_minute - 2
(F) LINE   crates/runner/src/outcome.rs:4631  [assert_eq! message argument]  i + 1
(F) LINE   crates/runner/src/outcome.rs:5295  [assert! message argument]  want[0]
(F) LINE   crates/runner/src/outcome.rs:5300  [assert! message argument]  want[1]
(F) LINE   crates/runner/src/outcome.rs:5305  [assert! message argument]  want[2]
(F) BRANCH crates/runner/src/outcome.rs:3666:13 False  [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),
(F) BRANCH crates/runner/src/outcome.rs:3666:36 False  [assert! condition branch (failing side)]  session.fillable(5) && !session.fillable(6),
(F) BRANCH crates/runner/src/outcome.rs:3765:13 False  [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F) BRANCH crates/runner/src/outcome.rs:3765:40 False  [assert! condition branch (failing side)]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F) BRANCH crates/runner/src/outcome.rs:4147:13 False  [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),
(F) BRANCH crates/runner/src/outcome.rs:4147:27 False  [assert! condition branch (failing side)]  e.t != 0.0 && e.t.is_finite(),
(F) BRANCH crates/runner/src/outcome.rs:4587:13 False  [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F) BRANCH crates/runner/src/outcome.rs:4587:39 False  [assert! condition branch (failing side)]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F) BRANCH crates/runner/src/outcome.rs:4899:17 False  [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F) BRANCH crates/runner/src/outcome.rs:4899:36 False  [assert! condition branch (failing side)]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F) BRANCH crates/runner/src/outcome.rs:4937:17 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:4937:30 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:4937:50 False  [assert! condition branch (failing side)]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F) BRANCH crates/runner/src/outcome.rs:5322:17 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5322:34 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5322:51 False  [assert! condition branch (failing side)]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F) BRANCH crates/runner/src/outcome.rs:5450:17 False  [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F) BRANCH crates/runner/src/outcome.rs:5450:46 False  [assert! condition branch (failing side)]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F) BRANCH crates/runner/src/outcome.rs:5477:13 False  [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),
(F) BRANCH crates/runner/src/outcome.rs:5477:37 False  [assert! condition branch (failing side)]  !f.was_refused(last) && !f.was_missing(last),
(F) BRANCH crates/runner/src/outcome.rs:5480:17 False  [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F) BRANCH crates/runner/src/outcome.rs:5480:47 False  [assert! condition branch (failing side)]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F) BRANCH crates/runner/src/outcome.rs:5536:13 False  [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F) BRANCH crates/runner/src/outcome.rs:5536:39 False  [assert! condition branch (failing side)]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F) BRANCH crates/runner/src/outcome.rs:5539:17 False  [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F) BRANCH crates/runner/src/outcome.rs:5539:42 False  [assert! condition branch (failing side)]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F) BRANCH crates/runner/src/portfolio.rs:915:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([7; 32])
(F) BRANCH crates/runner/src/portfolio.rs:1017:18 False  [assert! condition branch (failing side)]  } if strategy_digest == StrategyDigest::new([3; 32])
(F) LINE   crates/runner/src/rank.rs:1264  [assert! message argument]  grinder.payoff_bp(),
(F) LINE   crates/runner/src/rank.rs:1265  [assert! message argument]  grinder.worst_reward_risk_bp()
(F) BRANCH crates/runner/src/report.rs:1508:13 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1508:27 False  [assert! condition branch (failing side)]  luck > 0.0 && luck < required,
(F) BRANCH crates/runner/src/report.rs:1521:13 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/report.rs:1521:29 False  [assert! condition branch (failing side)]  distinct > 0 && distinct < n,
(F) BRANCH crates/runner/src/report_cover_tests.rs:192:13 False  [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F) BRANCH crates/runner/src/report_cover_tests.rs:192:45 False  [assert! condition branch (failing side)]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F) BRANCH crates/runner/src/report_cover_tests.rs:276:13 False  [assert! condition branch (failing side)]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F) BRANCH crates/runner/src/report_cover_tests.rs:276:38 False  [assert! condition branch (failing side)]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F) BRANCH crates/runner/src/resample.rs:881:21 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:881:41 False  [assert! condition branch (failing side)]  assert!(b.open <= b.high && b.open >= b.low);
(F) BRANCH crates/runner/src/resample.rs:882:21 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) BRANCH crates/runner/src/resample.rs:882:42 False  [assert! condition branch (failing side)]  assert!(b.close <= b.high && b.close >= b.low);
(F) LINE   crates/runner/src/signal_candle_stop_tests.rs:74  [unreachable! (failing arm)]  _ => unreachable!(),
(F) BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:13 False  [assert! condition branch (failing side)]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F) BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:42 False  [assert! condition branch (failing side)]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F) LINE   crates/runner/src/significance.rs:945  [assert! message argument]  bonferroni_t(trials)
(F) LINE   crates/runner/src/significance.rs:1341  [assert! message argument]  (approx - exact).abs()
(F) LINE   crates/runner/src/significance.rs:1357  [assert! message argument]  bonferroni_t(316)
(F) BRANCH crates/runner/src/significance.rs:1323:21 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1323:42 False  [assert! condition branch (failing side)]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F) BRANCH crates/runner/src/significance.rs:1385:13 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) BRANCH crates/runner/src/significance.rs:1385:26 False  [assert! condition branch (failing side)]  bar > 7.0 && bar < 9.0,
(F) BRANCH crates/runner/src/significance_cover_tests.rs:63:13 False  [assert! condition branch (failing side)]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F) BRANCH crates/runner/src/significance_cover_tests.rs:63:38 False  [assert! condition branch (failing side)]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F) LINE   crates/runner/src/split.rs:385  [assert! message argument]  bar + horizon,
(F) BRANCH crates/runner/src/split.rs:576:21 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/split.rs:576:45 False  [assert! condition branch (failing side)]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F) BRANCH crates/runner/src/synthetic.rs:130:25 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:130:45 False  [assert! condition branch (failing side)]  assert!(c.open <= c.high && c.open >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:25 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:131:46 False  [assert! condition branch (failing side)]  assert!(c.close <= c.high && c.close >= c.low);
(F) BRANCH crates/runner/src/synthetic.rs:330:17 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) BRANCH crates/runner/src/synthetic.rs:330:27 False  [assert! condition branch (failing side)]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F) LINE   crates/runner/src/trade.rs:1879  [unreachable! (failing arm)]  unreachable!("windows(2) always yields exactly two trades")
(F) LINE   crates/runner/src/trade.rs:2234  [assert! message argument]  t.count(),
(F) LINE   crates/runner/src/trade.rs:2378  [assert! message argument]  ist_minute_of(exit_ts)
(F) LINE   crates/runner/src/trade.rs:3029  [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  matches!(
(F) LINE   crates/runner/src/trade.rs:3138  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/trade.rs:1692:25 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:1692:48 False  [assert! condition branch (failing side)]  assert!(refused.signals > 0 && refused.reconciles());
(F) BRANCH crates/runner/src/trade.rs:1878:17 False  [let-else whose block is only unreachable!]  let [previous, next] = pair else {
(F) BRANCH crates/runner/src/trade.rs:2084:21 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2084:48 False  [assert! condition branch (failing side)]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F) BRANCH crates/runner/src/trade.rs:2590:17 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2590:32 False  [assert! condition branch (failing side)]  assert!(saw_refused && saw_missing, "both holes were located");
(F) BRANCH crates/runner/src/trade.rs:2704:13 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:2704:32 False  [assert! condition branch (failing side)]  !path.priceable && path.first_refused == Some(refused),
(F) BRANCH crates/runner/src/trade.rs:3212:13 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3212:34 False  [assert! condition branch (failing side)]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F) BRANCH crates/runner/src/trade.rs:3236:17 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3236:44 False  [assert! condition branch (failing side)]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F) BRANCH crates/runner/src/trade.rs:3242:13 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade.rs:3242:42 False  [assert! condition branch (failing side)]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F) BRANCH crates/runner/src/trade.rs:3539:17 False  [assert! condition branch (failing side)]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F) BRANCH crates/runner/src/trade.rs:3539:41 False  [assert! condition branch (failing side)]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F) BRANCH crates/runner/src/trade.rs:3593:17 False  [assert! condition branch (failing side)]  assert!(column.accepts(entry) && column.accepts(exit));
(F) BRANCH crates/runner/src/trade.rs:3593:42 False  [assert! condition branch (failing side)]  assert!(column.accepts(entry) && column.accepts(exit));
(F) BRANCH crates/runner/src/trade.rs:3674:17 False  [assert! condition branch (failing side)]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F) BRANCH crates/runner/src/trade.rs:3674:53 False  [assert! condition branch (failing side)]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F) BRANCH crates/runner/src/trade.rs:3733:17 False  [assert! condition branch (failing side)]  assert!(walked.signals > 0 && walked.reconciles());
(F) BRANCH crates/runner/src/trade.rs:3733:39 False  [assert! condition branch (failing side)]  assert!(walked.signals > 0 && walked.reconciles());
(F) BRANCH crates/runner/src/trade.rs:3735:17 False  [assert! condition branch (failing side)]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F) BRANCH crates/runner/src/trade.rs:3735:45 False  [assert! condition branch (failing side)]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F) BRANCH crates/runner/src/trade.rs:3801:17 False  [assert! condition branch (failing side)]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F) BRANCH crates/runner/src/trade.rs:3801:42 False  [assert! condition branch (failing side)]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:13 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) BRANCH crates/runner/src/trade_clock_tests.rs:38:43 False  [assert! condition branch (failing side)]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F) LINE   crates/runner/src/validate.rs:5602  [unreachable!/panic! in test code (failing arm)]  unreachable!("the zero-rung door must refuse before building")
(F) LINE   crates/runner/src/validate.rs:5735  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),
(F) LINE   crates/runner/src/validate.rs:5768  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),
(F) LINE   crates/runner/src/validate.rs:6027  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk finishes no fold, so the hook never speaks"),
(F) LINE   crates/runner/src/validate.rs:6629  [assert! condition: the matches! arm that fails the assert (llvm-cov counts this line 0 and its scrutinee on every pass)]  !matches!(
(F) LINE   crates/runner/src/validate.rs:6797  [assert_eq! message argument]  f.considered.saturating_sub(f.priced)
(F) LINE   crates/runner/src/validate.rs:7941  [assert! message argument]  body.len()
(F) LINE   crates/runner/src/validate.rs:8000  [assert! message argument]  body.len()
(F) BRANCH crates/runner/src/validate.rs:6856:13 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6856:29 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:6856:50 False  [assert! condition branch (failing side)]  at.is_some() && collect.is_some() && collect < at,
(F) BRANCH crates/runner/src/validate.rs:7363:19 False  [assert! condition branch (failing side)]  }) if refused_fold == fold
(F) BRANCH crates/runner/src/validate.rs:7827:20 False  [assert! condition branch (failing side)]  if fold == fold_index
(F) BRANCH crates/runner/src/validate.rs:7857:24 False  [assert! condition branch (failing side)]  if fold == fold_index
(F) BRANCH crates/runner/src/validate.rs:7920:17 False  [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F) BRANCH crates/runner/src/validate.rs:7920:53 False  [assert! condition branch (failing side)]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F) BRANCH crates/runner/src/validate.rs:7921:17 False  [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F) BRANCH crates/runner/src/validate.rs:7921:54 False  [assert! condition branch (failing side)]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F) BRANCH crates/runner/src/validate.rs:8008:13 False  [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F) BRANCH crates/runner/src/validate.rs:8008:45 False  [assert! condition branch (failing side)]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F) BRANCH crates/runner/src/validate.rs:8031:13 False  [assert! condition branch (failing side)]  !inner.contains("ExecutionRunV1::new(")
(F) BRANCH crates/runner/src/validate.rs:8032:20 False  [assert! condition branch (failing side)]  && !inner.contains("ExecutionDigestsV1::of(")
(F) BRANCH crates/runner/src/validate.rs:8033:20 False  [assert! condition branch (failing side)]  && inner.contains("ExecutionRunV1::with_digests(&run, &train_digests)"),
(F) BRANCH crates/runner/src/validate.rs:8059:17 False  [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F) BRANCH crates/runner/src/validate.rs:8059:60 False  [assert! condition branch (failing side)]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F) BRANCH crates/runner/src/validate.rs:8061:17 False  [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F) BRANCH crates/runner/src/validate.rs:8061:56 False  [assert! condition branch (failing side)]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F) BRANCH crates/runner/src/validate.rs:8097:13 False  [assert! condition branch (failing side)]  !inner.contains("data_digest_with_execution(")
(F) BRANCH crates/runner/src/validate.rs:8098:20 False  [assert! condition branch (failing side)]  && !inner.contains("ExecutionRunV1::new("),
(F) BRANCH crates/runner/src/validate.rs:8133:13 False  [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F) BRANCH crates/runner/src/validate.rs:8133:58 False  [assert! condition branch (failing side)]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F) BRANCH crates/runner/src/validate.rs:8137:13 False  [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F) BRANCH crates/runner/src/validate.rs:8137:46 False  [assert! condition branch (failing side)]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F) BRANCH crates/runner/src/validate.rs:8141:13 False  [assert! condition branch (failing side)]  !inner.contains("attest_training(")
(F) BRANCH crates/runner/src/validate.rs:8142:20 False  [assert! condition branch (failing side)]  && !inner.contains("evaluate_training_grid_attested("),
(F) LINE   crates/runner/src/validate_cover_tests.rs:1459  [unreachable!/panic! in test code (failing arm)]  unreachable!("no fold is built for a refused policy or execution path")
(F) LINE   crates/runner/src/validate_cover_tests.rs:1869  [assert! message argument]  String::from_utf8_lossy(&child.stdout)
(F) LINE   crates/runner/src/validate_cover_tests.rs:1894  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),
(F) LINE   crates/runner/src/validate_cover_tests.rs:1960  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),
(F) LINE   crates/runner/src/validate_cover_tests.rs:2156  [unreachable!/panic! in test code (failing arm)]  &|_| unreachable!("a refused walk reports no fold"),
(F) BRANCH crates/runner/src/validate_cover_tests.rs:1920:13 False  [assert! condition branch (failing side)]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:1920:43 False  [assert! condition branch (failing side)]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2317:13 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2317:45 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2368:13 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2368:43 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2371:13 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2371:49 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2401:19 False  [assert! condition branch (failing side): the False side of the guard of the condition's matches!]  )) if ambiguous_bars > 0
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2503:17 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F) BRANCH crates/runner/src/validate_cover_tests.rs:2503:50 False  [assert! condition branch (failing side): the False side of a conjunction's operand]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F) BRANCH crates/runner/tests/base_cov_admission_words.rs:66:69 False  [assert! condition branch (failing side): the False side of a disjunction's last operand]  name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
(F) BRANCH crates/runner/tests/end_to_end.rs:211:9 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
(F) BRANCH crates/runner/tests/end_to_end.rs:211:25 False  [assert! condition branch (failing side)]  rc.is_some() && spa.is_some(),
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
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:21 False  [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:44 False  [assert! condition branch (failing side)]  assert!(control.count() > 0 && control.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:21 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:41 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:21 False  [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:49 False  [assert! condition branch (failing side)]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:25 False  [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:45 False  [assert! condition branch (failing side)]  assert!(mask.count() > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:21 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:41 False  [assert! condition branch (failing side)]  assert!(mask.signals > 0 && mask.reconciles());
(F) LINE   crates/runner/tests/hole_after_exit.rs:129  [assert! message argument]  probes.len()
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
(U) crates/runner/src/topn.rs:373:16 False, 376:16 False, 379:16 False, 382:16 False, 389:20 False (the `if let Some(slot) = bytes.get_mut(..)` arms of RankingPolicyV1::canonical_bytes). Proof: `bytes` is `[0_u8; RANKING_POLICY_CANONICAL_LEN_V1]` with the constant 76 (topn.rs:47); the four header ranges are the constants ..16, 16..20, 20..24, 24..28, and the weight loop runs over `self.weights.values()`, a `[u32; CRITERIA]` with CRITERIA = 11 (topn.rs:53, 107), so its ranges are 32 + 4i .. 36 + 4i for i in 0..11, the last ending at 76. Every range lies inside the array, so get_mut is Some on every call.
(U) crates/runner/src/bootstrap_zero_v2.rs:187:45 True, 191:12 True, 192 (`!summary.standard_error.is_finite()`, `!observed.is_finite()` in audit_sources), and crates/runner/src/bootstrap_family_pass.rs:461:20 True (`!stat.standard_error.is_finite()`), all on bootstrap::summarise of an i64 series. Proof: summarise (bootstrap.rs:1673-1695) is reached with n >= 2 periods; every |x as f64| <= 9.3e18, so mean is finite and each (x - mean)^2 <= 3.5e38, and the sum over n <= 2^64 terms is below 6.4e57: the variance and standard_error = sqrt(variance / n) are finite and non-negative, never NaN. These lines are reached only when standard_error > 0 (the first operand refuses <= 0). Then observed = mean / standard_error (studentized, bootstrap.rs:1725): a nonzero deviation of integer data from its f64 mean is at least about 5.4e-20 in magnitude (the mean is a nonzero integer sum over n <= 2^64, or an exact zero, and a difference of two doubles of magnitude >= 1 is a multiple of 2^-52), so variance >= 2.9e-39 / (n - 1), standard_error >= 5.4e-20 / n >= 2.9e-39, and |observed| <= 9.3e18 / 2.9e-39 < 3.3e57, finite.
(U) crates/runner/src/validate.rs:3771:13 False, 3772 (`let Some(outcome) = oos.get(ordinal) else { return Err(ProvenanceMismatch) }` in capture_anchored_fold). Proof: the function refuses at its first `if` unless `scored.len() == oos.len()` (line 3760), and the line before refuses unless `scored.get(ordinal)` is Some (3768), so ordinal < scored.len() == oos.len() and oos.get(ordinal) is Some. True for a direct call as well: the order of the two checks is fixed in the body.
(U) crates/runner/src/validate.rs:4123:9 False, 4124 (`let Ok(ordinal) = usize::try_from(choice.ordinal) else { return false }` in choice_matches_visible_v2), and every `usize::try_from(choice.ordinal).map_err(|_| ..StableWidth)?` of the V2/V3/V4 reconciliations that the figures show. Proof: choice.ordinal is a u64 and usize is 64 bits wide on aarch64-apple-darwin (and on this x86_64 host), so the conversion is total there; the arm exists for a 32-bit target, which the bar does not measure.
(U) crates/runner/src/exit_grid_policy.rs:2876 (`.ok_or(ExitGridErrorV1::InvalidOosBoundary { .. })?;` after `bars.get(first_oos)` in replay_selected_universe_with). Proof: OosExecutionSeriesV1's fields are private and its only constructor, OosExecutionSeriesV1::new (exit_grid_policy.rs:716-724), refuses `first_oos >= series.bars.len()`; `bars` here is that same series' bars (`oos.series().bars()`), so bars.get(first_oos) is always Some.
(U) crates/runner/src/exit_grid_policy.rs:2131:25 False, 2132 (the `else { return false }` of `let Some(next) = pricing_refused.checked_add(1)` in evidence_reconciles). Proof: pricing_refused starts at 0 and gains 1 per candidate visited, so it never exceeds candidates.len(), which is below u64::MAX for any vector that fits in memory; checked_add cannot overflow.
(U) crates/runner/src/grid.rs:3376-3379 and 3433-3436 (the `map_err(|_| ReplayEvidenceRefused("candidate count does not fit the V1 receipt"))?` of `u64::try_from(<len>)` in replay_universe_v1 and replay_universe_from_candidates_v1), and every other `u64::try_from(<usize>)` refusal of runner the figures show. Proof: usize is 64 bits wide on aarch64-apple-darwin (and on this x86_64 host), so every usize converts to u64; the arm exists for a wider usize, which neither target has.
(U) crates/runner/src/grid.rs:3478 (`)?;` of `refused_paths.checked_add(u64::from(refused)).ok_or(..)` in replay_universe_from_candidates_v1). Proof: refused_paths gains at most 1 per candidate of a slice, so it never exceeds candidates.len() < u64::MAX; checked_add cannot overflow.
(U) crates/runner/src/exit_grid_policy.rs:605:12 False, 614:9 False, 615 (require_exact_execution_subslice). Proof: 605 is reached only with `window = reference_minute_context.get(offset / size..end)` where offset is evaluated_execution_1m's own address minus the context's and end = offset / size + evaluated_execution_1m.len(), so window starts at evaluated_execution_1m's address and has its length: core::ptr::eq (address and length) is true. At 614, start is a position inside reference_minute_context and evaluated_execution_1m.len() is a slice length, both at most isize::MAX, so their sum fits usize and checked_add cannot fail.
(U) crates/runner/src/exit_grid_policy.rs:3412:13 False, 3413, 3415:13 False, 3416 (`let Some(left) = pair.first() else`, `let Some(right) = pair.last() else` in validate_percentiles). Proof: `pair` comes from `values.windows(2)`, which yields only slices of exactly two elements, so first() and last() are Some.
(U) crates/runner/src/exit_grid_policy.rs:3590:12 True, 3590:45 True (`bar.high <= 0`, `bar.close <= 0` in validate_execution_bars). Proof: the line is reached only after `bar.check()` succeeded (it refuses unless low <= open <= high and low <= close <= high, indicators lib.rs:1704-1718) and `bar.open <= 0` refused; so high >= open > 0, and when the evaluation reaches the close operand low > 0 already holds, so close >= low > 0.
(U) crates/runner/src/exit_grid_policy.rs:1131:12 True, 1132-1135 (`if pair_count > self.ratios.max_pairs() { return Err(RatioPairLimitExceeded) }` in resolve_levels). Proof: ratio_pairs is the Ok of admitted_ratio_pairs (exit_grid_policy.rs:3679-3731), which returns RatioPairLimitExceeded before it pushes a pair once `out.len() >= limits.max_pairs()`, so it never returns more than max_pairs pairs; pair_count <= max_pairs on every value that reaches line 1131.
(U) crates/runner/src/trade.rs:679:13 False, 680, 682:13 False, 683 (`let Some(gaps_before) = self.broken_prefix.get(from + 1)`, `let Some(gaps_after) = self.broken_prefix.get(to + 1)` in SliceFacts::path_accepts). Proof: SliceFacts::of pushes one entry to refused_prefix and one to broken_prefix per bar after one shared initial 0 (trade.rs:540-563), so both tables always have the same length; these lines are reached only after refused_prefix.get(from) and refused_prefix.get(to + 1) were Some with from <= to, so from + 1 <= to + 1 < len and both broken_prefix reads are Some.
(U) crates/runner/src/trade.rs:729:16 False (`if let [before, after] = pair` in next_marked). Proof: `pair` comes from `prefix.windows(2)`, which yields only two-element slices, so the pattern always matches.
(U) crates/runner/src/trade.rs:1045:13 False, 1046-1052 (the `else` of `let Some(entry) = signal.checked_add(step)` in walk_core). Proof: `signal` is a column source, an index into `bars` (a slice, so below isize::MAX), and step is 0 or 1, so the sum cannot overflow usize.
(U) crates/runner/src/outcome.rs:812:19 False (`while let Some(below) = levels.last()` in BlockExtremes::new). Proof: `levels` is created as `vec![base]` on the line before and is only ever pushed to inside the loop, so it is never empty and last() is Some on every iteration; the loop ends only through its `break`.
(U) crates/runner/src/outcome.rs:962:13 False, 963, 964 (`let Some(start) = bars.get(i).map(..) else { lanes.absent(Unpriced::Refused); continue }` in forward_over). Proof: the enclosing loop is `for i in 0..bars.len()` (outcome.rs:948), so bars.get(i) is Some for every i.
(U) crates/runner/src/outcome.rs:1050:13 False, 1055, 1056 (`let Some((later, now)) = bars.get(exit).zip(bars.get(i))... else { .. continue }` in forward_over). Proof: the loop continues at its head unless `facts.covers(bars)`, i.e. the facts' verdict has exactly bars.len() entries (trade.rs SliceFacts::covers), and SliceFacts::of builds refused_prefix with one entry per verdict after an initial 0 (bars.len() + 1 entries). Line 1046 continues unless path_accepts(i, exit), which is false unless i <= exit and refused_prefix.get(exit + 1) is Some (trade.rs path_accepts), so exit + 1 <= bars.len(); and i < exit (line 1003 continues on exit <= i). Both get() calls are Some.
(U) crates/runner/src/outcome.rs:1170:28 False, 1174:28 False (`if let Some(Reverse(moved)) = upper.pop()` and `&& let Some(moved) = lower.pop()` in prefix_median_steps_over). Proof: each pop is guarded by a length test on the same heap: `upper.len() > lower.len() + 1` implies upper is non-empty, and `lower.len() > upper.len()` implies lower is non-empty, so each pop is Some.
(U) crates/runner/src/outcome.rs:1193:20 False (`&& let Some(slot) = out.get_mut(index)`). Proof: the loop is `for index in 0..out.len()` and `out` is not resized inside it, so get_mut(index) is Some.
(U) crates/runner/src/outcome.rs:2086:24 False, 2091:24 False (`if let Some(freed) / Some(staying) = self.slab.get_mut(at)` in OverlapWindow::retire_slot). Proof: both sit inside `while let Some(&hit) = self.slab.get(at)` with the same `at`, and nothing between the read and the write changes slab's length (`add` touches only `sums`, the rest only `live`, `free` and `kept`), so get_mut(at) is Some.
(U) crates/runner/src/outcome.rs:2098:16 False (`if let Some(head) = self.wheel.get_mut(slot)` in retire_slot). Proof: retire_slot's only caller is retire_through, which passes `slot = (self.tick + step) % turn` with `turn = self.wheel.len()`; wheel is `vec![NONE; capacity.max(1)]` (outcome.rs:2000) and never resized, so slot < wheel.len().
(U) crates/runner/src/outcome.rs:2116:16 False (`if let Some(head) = self.wheel.get_mut(slot)` in OverlapWindow::file). Proof: `slot = hit.death % self.wheel.len()` on the first line of the function, with wheel non-empty and never resized (as above), so slot < wheel.len().
(U) crates/runner/src/outcome.rs:2274:23 True (`!forward.covers(source)` with `paired` true, in edge). Proof: paired means forward.bars_len == column.census().offered (Forward::built_from_same_slice_as), and covers(source) is `source < bars_len`. Every Column constructor keeps each source below census.offered: build_withholding_from increments offered once per kept bar before it pushes that bar's kept index (indicators column.rs:701-716), reproject_with refuses any target >= onto_len and then sets offered = onto_len (column.rs:1066-1097), Default has no rows, and try_clone_exact, clear_before and replace_* leave sources and census unchanged. So covers(source) is true whenever paired is.
(U) crates/runner/src/bootstrap.rs:628:17 False (`let Some(own) = stats.get(s) else { continue }` in spa). Proof: `stats` is `returns.iter().map(summarise).collect()` in the same function (bootstrap.rs:577) and `s` enumerates `returns`, so s < returns.len() == stats.len().
(U) crates/runner/src/bootstrap.rs:868:8 True, 869:9 True, 870:16 True, 873 (the finiteness refusal of exact_family_test_inputs_v1). Proof: it is reached only with aligned_for(returns, block) = Some(periods), periods >= 2, which needs a non-empty family whose series all have exactly `periods` entries (aligned, bootstrap.rs:1652-1658), so summarise(series).periods == periods for each. For an i64 series of 2 <= n entries the mean and standard error are finite and non-negative (the bound given for bootstrap_zero_v2.rs:187 above: |x as f64| <= 9.3e18, so the sum of n <= 2^61 terms and the sum of squared deviations are below f64::MAX). No operand is true.
(U) crates/runner/src/bootstrap.rs:727:8 True (`!observed.is_finite()` in white_reality_check_receipt_v1) and 802:8 True, 803 (the same in spa_receipt_v1). Proof: both run only after exact_family_test_inputs_v1 returned Some, so the family is non-empty and every mean and standard error is finite (above). White's observed is the max of root_n * mean with root_n = sqrt(periods) <= 2^31 and |mean| <= 9.3e18, finite; SPA's is the max of studentized(mean, se).max(0.0), where studentized is mean / se for se > 0 and 0 otherwise (bootstrap.rs:1725-1731), and for se > 0 the quotient is finite by the lower bound on se given for bootstrap_zero_v2.rs:187.
(U) crates/runner/src/bootstrap.rs:892:8 True, 892:39 True, 893, 900:8 True, 901 (the refusals of exact_family_test_fields_v1), and 766, 851 (the `)?` of its two calls). Proof: its two callers count matched_or_exceeded at most once per iteration of `for _ in 0..draws` (bootstrap.rs:736-760, 821-848), so it is <= draws; each refuses a non-finite observed before the call (727, 802), so observed is finite; p_value = (matched + 1) / (draws + 1) with 1 <= matched + 1 <= draws + 1 lies in (0, 1]. The other `?` operands cannot fail either: matched + 1 <= draws + 1, and draws + 1 was refused on overflow in exact_family_test_inputs_v1; ordered_family_test_digest_v1's `u64::try_from(usize)` is infallible where usize is 64 bits (aarch64-apple-darwin). So the function returns Some on every call and the `?` never propagates.
(U) crates/runner/src/bootstrap.rs:1347:8 True, 1348 (`observed.iter().any(|value| !value.is_finite())` in romano_wolf_adjusted_p_values_v1). Proof: the function refuses unless every standard error is positive and finite (1334-1339) after aligned_for and periods >= 2, so each observed = mean / se is finite by the bound given for bootstrap_zero_v2.rs:187.
(U) crates/runner/src/bootstrap.rs:1362, 1547 (`_ => left.cmp(right)` of the stepdown sort comparators). Proof: the sorted vector is `(0..returns.len()).collect()` and `observed` has one entry per series (a map over stats, itself a map over returns), so observed.get(left) and observed.get(right) are both Some for every comparison.
(U) crates/runner/src/bootstrap.rs:1565:13 False, 1566, 1568:13 False, 1569, 1588:16 False, 1599:17 False, 1601, 1620:16 False, 1622 (the index guards of romano_wolf_aligned). Proof: `order` is a permutation of 0..returns.len() and stats and observed are maps over returns, so `order.get(rank)` for rank in 0..order.len(), and returns/stats/observed.get(strategy) for any strategy taken from order, are Some; `bars` is `vec![..; order.len()]`, so bars.get_mut(rank) is Some; `first = out.len()` is read before `out.extend(..)`, and `first..` with first <= len is a valid range, so out.get_mut(first..) is Some.
(U) crates/runner/src/expression_oos.rs:299 (the `)?` of `summarize(column, bars, anchor.program())?` in evaluate_expression_oos_on). Proof: the function has already returned unless `later.sources` is Ok, i.e. validate_column_sources(column, bars.len(), 0) passed (expression_oos.rs:153, 273; exit_grid_policy.rs:4150-4180): bits.len() == sources.len(), every source < bars.len() and sources strictly increase; known.len() == bits.len() for every Column (see the signal_candle_stop.rs:1349 proof). So crate::expression::evaluate cannot refuse Alignment or SourceOrder, CountOverflow needs more than u64::MAX rows, and the visitor's two errors cannot occur: bars.get(index) is Some because evaluate passes each row's source, and `support` counts at most one per row.
(U) crates/runner/src/expression_oos.rs:302:12 True, 303 (`summary.hits != grid.signals`). Proof: grid.signals is the `signals` of trade::walk_expression_over over the same bars, column and program (grid.rs:2666-2672, 2133-2150), which walk_core increments once for each column row from first_row 0 whose predicate `expression.evaluate(bits, known) == Truth::True` holds (trade.rs:802-826, 981-993); summary.hits is the count of rows, over the same column's bits, known and sources, for which `expression.evaluate(truth, known)` is Truth::True (expression.rs:91-121). Same rows, same predicate, so the counts are equal (neither can saturate).
(U) crates/runner/src/excursion.rs:234:24 False (`if let Some(prefix) = observed.get_mut(..bound)` in Ladder::from_excursions). Proof: `bound` starts at observed.len() and is only ever lowered (to `at`, inside `if at < bound`), so `..bound` is always within the slice and get_mut is Some.
(U) crates/runner/src/excursion.rs:1221:20 False, 1232:20 False, 1253:20 False (`out.adverse.get_mut(stop_cursor)`, `out.favourable.get_mut(target_cursor)`, `armed_from.get_mut(arm)` in crossings_with). Proof: the walk destructures the same `ladders` it passes to Crossings::empty, which sizes adverse to stops.len() and favourable to targets.len() (excursion.rs:525-526); armed_from is `vec![false; targets.len()]` (1158). Each write sits inside `while ladder.rungs().get(cursor).is_some_and(..)`, so the cursor is below the ladder's length when it is used, and a cursor only reaches targets.len() at most, so `arm` in target_before..target_cursor is below targets.len().
(U) crates/runner/src/excursion.rs:1397:13 False, 1405 (the `else { break }` of the three parallel reads in cross_armed_rows). Proof: the rows come from `out.armed.chunks_exact_mut(width.max(1))` with width = rungs.len() = trails.len() (the caller passes trails.rungs(), excursion.rs:1199-1201), and armed holds targets.len() * trails.len() cells (523), so there are targets.len() rows when trails is non-empty and none when it is empty; armed_from, armed_give_back and armed_cursor each hold targets.len() entries (1158-1160), so every read for arm < targets.len() is Some. The code's own comment says the same.
(U) crates/runner/src/excursion.rs:1472:16 False, 1479:16 False, 1482:16 False (`crossed_at/peak/peak_raised.get_mut(*cursor)` in cross_rungs). Proof: each write sits inside `while rungs.get(*cursor).is_some_and(..)`, so cursor < rungs.len(); both callers pass tables of exactly rungs.len() entries: the plain trail passes out.trailing, out.trail_peak and out.trail_peak_raised, each sized trails.len() (527-529) with rungs = trails.rungs() (1181-1187), and the armed rows pass three chunks_exact_mut chunks of width = rungs.len() (above).
(U) crates/runner/src/admission.rs:6121:45 True, and 5771:45 True in the cfg(test) V2 decoder (`evidence.canonical_bytes().as_slice() != bytes` with `evidence.values == decoded_values`, in decode_evidence_v3 / decode_evidence_v2). Proof: the reader accepts exactly one byte pattern per value: the header is checked field by field (length, magic, domain, reserved byte 5, version, payload length; admission.rs:5801-5856), bools and tags accept only their listed bytes (read_bool 2409-2420 and the tag readers), an observed value refuses a nonzero payload unless measured, identities and integers are raw little-endian, and finish() asserts that every byte was read (2302-2304, 5620-5622, debug builds, i.e. every test). The decoded statistics draft and walk-forward fields are stored as read (AdmissionStatisticsFieldsV3 { values }, from_canonical_fields), and canonical_evidence_bytes_v3 writes the same fields in the same order and encodings (5292-5307). So when the decoded values equal the evidence's values, its canonical bytes are the input bytes; whenever the condition is true, its first operand already is.
(U) crates/runner/src/admission.rs:2194:36 True, 5228:36 True (`computed.canonical_bytes().as_slice() != verdict_bytes` with `supplied == computed`, in the V1 and V3 from_canonical_parts). Proof: `supplied` was decoded from verdict_bytes by the same strict reader discipline (fixed length and header, tags refused outside their domain, raw integers, every byte read), so verdict_bytes is the canonical encoding of `supplied`; equal values have equal canonical bytes, so the operand is false whenever the first one is.
(U) crates/runner/src/expression_validation.rs:345:12 True, 346:16 True, 347:16 True, 349 (fold totals against the exact later cell in materialize_coordinate). Proof: `trades` is EvaluatedExpressionOosV1::materialize(ordinal) of that same cell, which returns rows only when the exact replay rebuilt this cell (`replay.cell == Some(selected)`) and reconcile_rows proved the row count and the sum of `worst` equal cell.trades and cell.pessimistic (grid.rs:3114-3131, 3134-3165); the replay counts a win exactly when the row's pessimistic result is positive (grid.rs:4583-4594, row.worst = pess by row_of 4270-4285). partition (below) adds every row to exactly one fold, one trade, one win when worst > 0 and its worst; so the totals equal the cell's trades, wins and pessimistic.
(U) crates/runner/src/expression_validation.rs:325 (the `)?` of partition) and 516:12 True, 517:16 True, 518:16 True, 519:16 True, 520:16 True, 521:16 True, 523 (the boundary refusal inside partition). Proof: the rows come from the replay over the same later bars (materialize uses self.bars), so entry_micros and exit_micros are bars[entry].ts_micros and bars[exit].ts_micros (row_of), entry < exit, and the walk opens a new position only strictly after the previous exit (trade.rs walk_core, `open_until`), so entry_bar > previous exit_bar. A position never survives its own session: every exit is at the horizon or at the entry day's forced square-off, and an unproved session end prices nothing (outcome.rs forward_over / trade.rs forced_exits), so entry and exit share an IST day; folds are unions of whole IST days (index_folds maps each bar by its day), so mapping[entry] == mapping[exit]. The remaining `?` errors of partition cannot fire: bars.get and mapping.get are Some (mapping has one entry per bar or bind refused earlier), folds.get_mut(first) is Some (a mapping value is a window index), the counts are at most the number of rows, and the return sum equals cell.pessimistic, an i64 (above).
(U) crates/runner/src/validate.rs:3103:17 False, 3104, 3105, 3110:12 True, 3110:26 True, 3110 third operand, 3111-3113 (the coordinate-order guard of authenticate_grid_pair_v4). Proof: the guard is reached only after `&fresh_long == long && &fresh_short == short` (3077), so each side is exactly what resolve_attested computes, whose cell_count is checked_policy_cell_count(stops, targets, trails, ratio_pairs) (exit_grid_policy.rs:1137-1138, 3793-3829). resolved_grid_view::visit_coordinates (326-365) returns false only when the callback does, and this callback does only on `ordinal.checked_add(1)` overflow, which needs 2^64 visits of a count that fits u64; so `complete` is true. The visit enumerates, for each stop setting s in 0..=S and target setting t in 0..=T (skipping a (stop, target) pair that ratio_pair_admitted refuses), the trail settings 0..=R plus t arms times cap trails each with cap summing to R(R+1)/2 over the trail settings: exactly the terms of the closed form (no-target row for every stop setting, every target row without a stop, and one row per admitted pair), so ordinal ends at cell_count. (The equality is also tested on every small configuration by exit_grid_policy::tests::checked_cell_count_matches_every_small_ratio_filtered_enumeration.)
(U) crates/runner/src/validate.rs:2574:12 True, 2575:16 True, 2576:16 True (the first three operands of the fold-shape refusal of walk_forward_exact_grid_v4). Proof: the folds come from Shape::Anchored.folds, i.e. split::anchored_folds (split.rs:276-306), which returns no fold unless splits > 0 and bars >= splits + 1, so width = bars / (splits + 1) >= 1, and builds every fold with train = (0..boundary, 0..0), boundary = (s + 1) * width >= 1: the first range starts at 0 and is non-empty, the second is empty. (The fourth operand, an empty test window, IS reachable: the last fold test_start = min(boundary + h, bars) reaches bars when the horizon spans the rest of the series; it is tested, not proved.)
(U) crates/runner/src/validate.rs:2600 (the `)?` of `ExecutionSeriesV1::new(..)` for a fold's training execution). Proof: ExecutionSeriesV1::new refuses only an empty feed, an empty commit or an all-zero calendar digest (exit_grid_policy.rs:210-233); all three are copied from `execution`, an ExecutionSeriesV1 that the same constructor already admitted, and it never refuses the bars (an empty training prefix is refused one line later, by resolve_attested, which the test the_v4_walk_refuses_no_fold_a_halted_search_and_an_empty_training_execution drives).
(U) crates/runner/src/trade.rs:1443:9 False, 1444, 1446:8 True, 1447 (`bars.get(entry)` None and `!facts.accepts(entry)` in entry_is_priceable). Proof: its two callers are walk_core's round-trip-refused branch (trade.rs:1286) and held_before_hole (942), whose only call is walk_core's path-refused branch (1259); walk_core reaches both only after `if !facts.accepts(entry) { .. continue }` (1056). SliceFacts::accepts(i) is `verdict.get(i)`, and SliceFacts::of keeps a verdict only of exactly bars.len() entries (trade.rs:511-563), so accepts(entry) implies entry < bars.len(): bars.get(entry) is Some and facts.accepts(entry) is true at both lines.
(U) crates/runner/src/grid.rs:3613:32 True (`one_row.len() != 1` with measured.trades == 1), 3620:12 True, 3620:43 True, 3621-3625 (`measured.ambiguous_bars > 1 || measured.gapped > 1`) and 3631 (`one_row.first()` None) in priceable_candidate_path_v1. Proof: one_variant is called with a one-element candidate slice and a cleared one_row (3604-3612). In one_variant's `for c in candidates` loop, a counted candidate runs `cell.trades += 1` (grid.rs:4493) and, with no continue, break or return before it, `rows.push(row_of(..))` (4583), so one counted trade pushes exactly one row; ambiguous_bars grows by u64::from(bool) once per counted trade (4503) and gapped by at most one per trade, in `ordered` (5358-5362). With one candidate, trades == 1 therefore means one row, ambiguous_bars <= 1 and gapped <= 1, and first() of a one-element vector is Some.
(U) crates/runner/src/grid.rs:2722, 2723 (`usize::try_from(requested_cells).map_err(..)?` in reserve_policy_cells_v1) and 3433-3436 (`u64::try_from(candidates.len()).map_err(..)?` in replay_universe_from_candidates_v1). Proof: usize and u64 are both 64 bits wide on aarch64-apple-darwin (as on this x86_64 host), so both conversions are infallible and the closures never run.
(U) crates/runner/src/trade.rs:810:8 True, 810:55 True, 812 (`column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()` in walk_expression_over). Proof: Column's fields are private to indicators::column and every constructor keeps bits, known and source parallel (see the signal_candle_stop.rs:1349 proof: the fold pushes the three together, indicators column.rs:712-715; reproject_with likewise; try_clone_exact copies; Default is empty; clear_before and replace_* rewrite in place at the same length).
(U) crates/runner/src/expression_execution.rs:357:12 True, 358 (`summary.hits != grid.signals` in evaluate_expression_with_attested). Proof: as for expression_oos.rs:302: grid.signals counts the column rows whose `expression.evaluate(bits, known)` is Truth::True (trade::walk_expression_over through walk_core from row 0), and summary.hits counts the same rows under the same predicate (crate::expression::evaluate over the same column). The summary cannot refuse first: AttestedTrainingV1 is built only by attest_training, which ran validate_column_sources(column, bars.len(), 0) (resolved_grid_view.rs:163), so every source indexes a bar and sources strictly increase.
(U) crates/runner/src/validate.rs:2286 (the `)?` of `AnchoredSearchCaptureV3::new(..)?` in the V3 door). Proof: its only refusals are ZeroResolvedRungs, refused by the door itself two statements earlier (`if rungs == 0 { return Err(..) }`, 2276-2278), and stable_u64_v2 of four usize values, `u64::try_from(usize)`, infallible where usize is 64 bits (validate.rs:3822-3845, 4150-4152).
(U) crates/runner/src/validate.rs:2458:12 True, 2460:12 True, 2461:12 True, 2462:12 True, 2463:12 True, 2464:12 True, 2465:12 True, 2466:12 True, 2467:12 True, 2472:12 True (the operands of authenticate_signal_column_shape_v4 after `sourced() != Signal` and `census.offered != signal_bars`). Proof: a column with sourced() == Signal is built only by the fold build_withholding_from (indicators column.rs:688-750; AnchoredColumn uses the same fold, and Default has offered 0, refused at 2459 for the non-empty signal checked at 2451). That fold increments offered once per kept bar and puts each bar in exactly one of warming, swept or a refused bucket (reconciles, column.rs:267-272), pushes bits, known and source together exactly when it increments swept (712-716), so swept == bits.len() == sources.len(); records first_swept as the first pushed index; pushes one acceptance verdict per kept bar, so with offered == signal.len() the acceptance covers the signal; stores acceptance_census = census, spec = evaluator.replay_spec() (Some for both evaluators, column.rs:193-213) with acceptance_known = true, and collided = 0 (738-748); and pushes kept indices of an enumerate walk, so sources strictly increase and stay below offered. clear_before, try_clone_exact and the crate-private replace_* keep all of these.
(U) crates/runner/src/validate.rs:3604:8 True, 3605 (`alignment.onto.len() != signal_column.sources().len()` in project_oos_fold). Proof: align::onto_execution returns Some only after its loop pushed exactly one entry, Some or None, per element of the `sources` it was given (align.rs:112-145: the only early return inside the loop is `signal.get(s)?`, which returns None), and it is given signal_column.sources().
(U) crates/runner/src/exit_grid_policy.rs:2679:12 True, 2680:16 True, 2681:16 True, 2682:16 True, 2683:16 True, 2684:16 True, 2685:16 True, 2686:16 True, 2687:16 True, 2688:16 True, 2701 (the seal check after `let selected = self.seal_selection(evaluated, cell)` in classify_coordinate). Proof: seal_selection (exit_grid_policy.rs:3039-3068) sets resolution_digest = self.digest and run_id, mask, horizon, column_digest, evaluation_spec and side from `evaluated`, coordinate = Chosen::from_cell(cell) (equal to `coordinate` by the check at 2640), training_cell = *cell, and selection_digest = digest_selected over exactly the arguments the check recomputes it from; so every operand is false.
(U) crates/runner/src/exit_grid_policy.rs:2675:12 True, 2676 (`!self.chosen_is_in_bounds(coordinate)` in classify_coordinate). Proof: chosen_is_in_bounds (resolved_grid_view.rs:374-385) is false only for axes out of bounds (refused at the top, 2627), a coordinate missing its stop or its target, or a stop-target pair the ratio limits do not admit. A cell missing its stop or target carries MISSING_STOP or MISSING_TARGET (execution_refusal_bits, 405-412) and has already returned as policy_refused; the ordinal came from canonical_coordinate_ordinal over coordinate_row_offsets, which stores None for every non-admitted stop-target pair (resolved_grid_view.rs:241-246), so a non-admitted pair was refused as InvalidChosenCoordinate before the cell lookup.
(U) crates/runner/src/grid.rs:2673 (the `)?` of `trade::walk_expression_over(..)?` in evaluate_resolved_expression_policy_v1). Proof: walk_expression_over's only refusal is its column alignment check (trade.rs:810-813), which no Column can fail (see the trade.rs:810 proof); the walk itself returns a value.
(U) crates/runner/src/grid.rs:2700, 2701 (`u64::try_from(grid.cells.len()).map_err(..)?` in evaluate_resolved_timed). Proof: usize is 64 bits wide on aarch64-apple-darwin (and on this host), so the conversion is infallible and the closure never runs.
(U) crates/runner/src/trade.rs:1159:16 True, 1160 (a blocked signal in walk_core's not-same-session branch). Proof: for a Fill column entry == signal, so the branch is reached only for a Signal column, entry = signal + 1. open_until is only ever the exit of an earlier position whose entry e0 is before this entry (rows and their sources strictly increase), and every such exit lies on e0's IST day: a priced trade exits at the horizon bar only when its deadline is at or before the day's forced stamp, else at forced.bar, and every held branch uses that exit or forced.bar, where forced is the square-off of e0's own day, equal to the derived table even through walk_with (the check at 1119-1126). If entry <= open_until, then e0 <= signal < entry <= open_until (the signal row is after the row that opened the position), and bars are in strictly increasing time, so signal and entry are on e0's day and same_session is true. So when same_session is false, blocked is false.
(U) crates/runner/src/trade.rs:1352:8 True, 1352:37 True, 1353 (`!facts.accepts(entry_bar) || !facts.accepts(exit_bar)` in round_trip). Proof: round_trip's only production call is walk_core's (trade.rs:1275), reached only after `if !facts.path_accepts(entry, exit) { .. continue }` (1258). SliceFacts::of builds refused_prefix by adding `!is_accepted` for each bar, with is_accepted read from the same verdict that SliceFacts::accepts returns (trade.rs:544-548, 609-615), and path_accepts(from, to) is true only when refused_prefix[to + 1] equals refused_prefix[from], i.e. no bar of from..=to is refused. So both legs are accepted wherever round_trip is called in production (its other calls are this module's tests).
(U) crates/runner/src/exit_grid_policy.rs:1172, 2457 (the `)?` of `ExecutionSeriesV1::new(..)?` in the #[cfg(test)] helpers ExitGridPolicyV1::resolve and ResolvedExitGridV1::evaluate_training_grid). Proof: ExecutionSeriesV1::new refuses only an empty feed, an empty commit or an all-zero calendar digest (exit_grid_policy.rs:210-233), and both helpers pass the literals "test-feed", "test-commit" and [0xA5; 32].
(U) crates/runner/src/validate.rs:2392:8 True, 2392:28 True, 2393 (`folds.is_empty() || folds.len() != validated.folds.len()` in the V4 door). Proof: walk_forward_exact_grid_v4 returns Err(Empty) when its split windows are empty (2562-2564), and otherwise pushes exactly one proof and one visible fold at the end of every loop iteration (2977-2996); every early exit inside the loop returns Err, and Ok is built only after the loop from those two vectors (2999-3005). So an Ok holds equally many, at least one, of each.
(U) crates/runner/src/validate.rs:2229 (the `)?` of `AnchoredAdmissionCaptureV2::new(..)?` in the V2 door). Proof: as for 2286: its only refusals are ZeroResolvedRungs, refused by the door itself just before (2217-2219), and usize-to-u64 conversions, infallible where usize is 64 bits.
(U) crates/runner/src/validate.rs:2745:16 True, 2746-2748 (`evaluated_cells != resolved.cell_count()` in walk_forward_exact_grid_v4) and 2808:12 True, 2809-2811 (`population_cells != expected_population`). Proof: evaluate_with_attested returns a grid only through grid::evaluate_resolved_policy_v1 -> evaluate_resolved_timed, which refuses ReplayCellCountMismatch unless the grid has exactly resolved.cell_count() cells (grid.rs:2637-2649, 2678-2712; exit_grid_policy.rs:2375-2399), so each pair's evaluated_cells equals its side's cell count. Every considered mask is priced once on each side (the `[long, short]` map over closed.kept, 2759-2768), so population_cells = considered_masks x (long + short) = expected_population, which was computed with checked arithmetic before the loop.
(U) crates/runner/src/validate.rs:2788 (the `)?` of `validated.cell(ordinal).ok_or(..)?`). Proof: `validated` is resolved.validate_evaluation(&pair.evaluated), whose cell(ordinal) is `self.evaluated.grid.cells.get(ordinal)` over that same evaluation (exit_grid_policy.rs:1586-1588), and ordinal runs over 0..pair.evaluated.grid().cells.len().
(U) crates/runner/src/validate.rs:3625:8 True, 3626 (`dropped != expected_dropped` in project_oos_fold). Proof: reproject_checked counts exactly the None targets of `onto` as dropped (indicators column.rs:1031-1033; a collision is counted as collided, not dropped). The Nones of alignment.onto are the `unreachable` ones onto_execution pushed while counting them (align.rs:136-141) plus each Some the loop turned into None while counting deliberately_dropped (3609-3615), so dropped == unreachable + deliberately_dropped == expected_dropped.
(U) crates/runner/src/grid.rs:1733:17 True (`if at > last { last }` in single_far_target). Proof: `ran` is non-empty here (1726-1728 return None otherwise), so n = ran.len() >= 1 and last = n - 1; at = n.saturating_mul(9) / 10, and 9n does not saturate because a Vec<Ppm> of 8-byte elements holds fewer than 2^60 elements and 9 * 2^60 < 2^64; so at = floor(9n / 10) <= n - 1 = last for every n >= 1, and the clamp's True side never runs.
(U) crates/runner/src/exit_grid_policy.rs:3667:8 True, 3668 (`if out.is_empty() { return Err(EmptyResolvedAxis(axis)) }` in resolve_axis_with_forced). Proof: `percentiles` is one schedule of the policy's RungPlanV1 (the callers at 1105-1112 pass self.rungs.stop(), target() and trail()); RungPlanV1's fields are private and outside test code its only constructor is RungPlanV1::new (821-846), which refuses an empty schedule through validate_percentiles (3401-3403). The loop's first iteration either returns an Err or pushes its value, because `out.last()` is None and both arms of the inner if fall through to `out.push(value)`; so `out` is non-empty whenever the loop ends.
(U) crates/runner/src/validate.rs:4515:13 False, 4516 (`let Some(train) = bars.get(fold.train.0.clone()) else { continue; }` in walk_forward_core). Proof: the folds are `shape.folds(bars.len(), horizon, splits)` (4510) over the same `bars`, and Shape has two arms (split.rs:154-159). Both anchored_folds (split.rs:276-306) and rolling_folds (213-243) return no fold unless n = bars.len() >= k + 1 >= 2 (k = splits), and give fold s < k the boundary (s + 1) * w with w = floor(n / (k + 1)), so boundary <= k * w <= k * n / (k + 1) < n with no saturating step reached; the anchored training range is 0..boundary and the rolling one (boundary - w)..boundary with boundary >= w. So the training range lies inside the slice and `get` is Some.
(U) crates/runner/src/validate.rs:2863 (the `)?` of `ExecutionSeriesV1::new(..)` for a fold's OOS execution series in walk_forward_exact_grid_v4). Proof: as for 2600: ExecutionSeriesV1::new refuses only an empty feed, an empty commit or an all-zero calendar digest (exit_grid_policy.rs:210-233), never the bars; all three are copied from `execution`, an ExecutionSeriesV1 whose private fields only that same constructor sets, so it already admitted them.
(U) crates/runner/src/grid.rs:3620:8 True, 3620:39 True (the two operands of `measured.ambiguous_bars > 1 || measured.gapped > 1` in priceable_candidate_path_v1, named 3620:12 and 3620:43 in the earlier paragraph above, which took its columns from the interim figures' layout). Proof: as written there, unchanged: one_variant runs over the one-element slice from_ref(candidate), its cell starts from Cell::default(), ambiguous_bars grows by u64::from(bool) once per counted trade and gapped by at most one per trade in `ordered`, so with measured.trades == 1 both are at most 1.
(U) crates/runner/src/validate.rs:2951:12 True, 2951:55 True, 2952 (`final_candidates.len() != pending.len() || oos_all.len() != pending.len()` in walk_forward_exact_grid_v4). Proof: both vectors are created empty for the fold (2830-2831) and are written only inside `for (ordinal, candidate) in pending.iter().enumerate()` (2882-2948), which runs only under `if !pending.is_empty()`. Every iteration either returns Err through one of its `?` or pushes exactly once to each vector, `oos_all.push(outcome.pessimistic)` (2913) and `final_candidates.push(proof)` (2947), with no continue or break between; an empty `pending` leaves both empty. So both lengths equal pending.len() at 2951.
(U) crates/runner/src/validate.rs:4971:21 False, 4972-4976 (`let Some(chosen_side) = chosen_side else { refused }` in walk_forward_core). Proof: `chosen` and `chosen_side` are bound together at 4952-4962 from one `best_ordinal.and_then(|ordinal| scored.get(ordinal)).map_or((None, Summary::default(), None, None, None), |candidate| (Some(candidate.mask), .., Some(candidate.side)))`: both None (the default) or both Some from the same candidate, and neither is reassigned before 4969. The let-else sits in the `Some(mask)` arm of `match chosen`, so `chosen_side` is Some there.
(U) crates/runner/src/validate.rs:5091:24 True, 5092 (`if *mask == ConditionMask::ZERO { 0 }` in walk_forward_core's start_for closure). Proof: start_for is called with candidate.mask for each element of `scored` (5132) and with the chosen mask (5153); every one is the mask of an Itemset of `closed.kept`, a subset of `swept.sweep.all_frequent()` (closed.rs:196-200), and the sweep is `Sweeper::new(per_fold).run_prepared(..)`, i.e. engine::Ladder::walk, whose level-1 masks are `ConditionMask::default().with_bit(p)` (engine lib.rs:1566) and whose level-k candidates are `a.union(b)` of two frequent (k-1)-masks (join_screen, engine lib.rs:2158-2162). By induction every swept mask has a bit set, so none equals ZERO; start_for is a closure local to walk_forward_core, so no direct call reaches it.
(U) crates/runner/src/grid.rs:2095 and 3088 (the `)?` of `trade::walk_expression_over(..)?` in evaluate_expression_over and materialize_expression_cell_over). Proof: as for grid.rs:2673 above: walk_expression_over's only refusal is its column alignment check (trade.rs:810-813), which no Column can fail (see the trade.rs:810 proof); the walk itself returns a value.
(U) crates/runner/src/grid.rs:4748:12 False (`pess > 0` in `if pess > 0 && (cell.min_win == 0 || pess < cell.min_win)` in tally_trade). Proof: the test sits inside the block of `if pess > 0 {` (4687) of the same function, and `pess` is a by-value, non-`mut` parameter of `const fn tally_trade` (4686), assigned nowhere between; when 4748 runs, `pess > 0` is true. (The operand repeats the enclosing test: a finding of this session, not a defect.)
(U) crates/runner/src/grid.rs:3637:8 True, 3638:12 True, 3640:12 True, 3641:12 True (`row.signal_bar != candidate.signal`, `row.entry_bar != candidate.entry`, `row.exit_micros != stamp`, `row.exit_bar < candidate.entry` in priceable_candidate_path_v1). Proof: `row` is the one row in one_row, which one_variant fills from the one-element slice from_ref(candidate) (the check before it refuses any other count) with `row_of(bars, c, exit, ..)` (grid.rs:4583) for c = that candidate; row_of sets signal_bar = c.signal and entry_bar = c.entry (4280-4281) and exit_micros = bars.get(exit).map_or(0, ..) (4286); exit = c.entry.saturating_add(pess_off) (4510) is never below c.entry; and `stamp` reads bars.get(row.exit_bar), the same index, refused by the `?` at 3636 when absent, so it equals exit_micros whenever 3640 is reached. This holds for any bars, candidate and variant, a direct call included. (The other three operands, 3639, 3642 and 3643, are driven by tests: an entry instant that is not its bar's, an occupancy ending before its entry, and an exit stamped before its entry.)
(U) crates/runner/src/exit_grid_policy.rs:4139:12 True, 4140:12 True, 4141:12 True (`!census.reconciles()`, `census.offered != bars`, `census.refused() != 0` in require_complete_acceptance). Proof: they are evaluated only after `!column.acceptance_covers(bars)` and `refused_or_missing != 0` (4137-4138) are false, i.e. the column has a known acceptance of exactly `bars` verdicts, every one true (the let-else at 4127 refused an unknown one). indicators::Column's acceptance fields are private to indicators::column and set only by: build_withholding_from (every Signal column, and AnchoredColumn's, which calls it), which for each kept bar pushes one verdict and adds one to census.offered and exactly one to warming, to swept, or through Census::charge to one refusal bucket (indicators column.rs:686-726), then stores acceptance_census = census; reproject_checked, whose (accepted, census) come from acceptance_of over the execution bars, the same one-verdict-one-bucket loop (column.rs:314-336); reproject and Default, whose acceptance is unknown; try_clone_exact, which copies the fields; clear_before and replace_*, which never touch them. Census::refused() sums the seven buckets charge() increments, one per Corrupt variant (column.rs:239-247, 279-302). So offered = warming + swept + refused() (reconciles), offered is the number of verdicts (= bars), and with every verdict true no bar was charged (refused() = 0).
(U) crates/runner/src/exit_grid_policy.rs:4156:8 True, 4157-4160 (`column.bits().len() != column.sources().len()` in validate_column_sources). Proof: as for trade.rs:810 and signal_candle_stop.rs:1349 above: every Column constructor keeps bits and source parallel, and no mutator changes either length.
(U) crates/runner/src/exit_grid_policy.rs:4173:12 True, 4174-4177 (`previous.is_some_and(|before| source <= before)` in validate_column_sources). Proof: every Column's sources strictly increase. build_withholding_from pushes `index - withheld` for the kept bars in enumerate order, and between two kept bars the enumerate index grows by one more than the withheld count does, so each pushed source exceeds the one before (indicators column.rs:688-716); reproject_with returns None for a target below the last pushed source, counts an equal one as collided without pushing it, and pushes only a greater one (column.rs:1031-1048); try_clone_exact copies; clear_before and replace_* never write sources.
(U) crates/runner/src/admission.rs:5056:36 True (`computed.canonical_bytes().as_slice() != verdict_bytes` in the #[cfg(test)] AdmissionDecisionV2::from_canonical_parts). Proof: as for the V1 and V3 doors at 2194:36 and 5228:36 above: it is evaluated only when `supplied == computed`, and `supplied` was decoded from verdict_bytes by AdmissionVerdictV1::from_canonical_bytes, a strict reader that reads every byte of its one canonical encoding, so verdict_bytes is the canonical encoding of supplied, which is that of computed.

APPENDIX LS3: the fenced content of reports/base-cloud-20261103/fu-LS3.txt.md on branch cloud/cov-libs-ls3-20261103, verbatim
BASE-COV-LIBS-LS3 Route B on tip dc3632704fdbc9a33f40145d2e6b23a8fbe719d0: crates lake, indicators, vocab, every .rs file whole (SET OVERRIDE).
Pinned nightly rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov/llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 (the Mac re-measures).
NOT READ by the route: crates/{lake,indicators,vocab}/benches/ratio.rs ([[bench]] test = false: cargo test --no-run builds none; L-BCL-1) and crates/lake/src/lib.rs (module declarations only).

TOTALS per crate (lines covered/counted; branch sides covered/counted; files read)
  indicators  lines 16909/16914 (uncovered 5)  branches 1892/2078 (uncovered 186)  files 39
  lake        lines 5678/6135 (uncovered 457)  branches 305/364 (uncovered 59)  files 17
  vocab       lines 8185/8205 (uncovered 20)  branches 1240/1298 (uncovered 58)  files 19

Uncovered rows 785: (F) 428, (U) 4, NEEDS THE MAC (#[ignore]d real-lake tests, L-BCL-3) 353, GAP 0.

Rule (recipe v4 step 4): (F) is only the failing side of assert!/assert_eq!/assert_ne!/panic!/unreachable! in test code: a
failing branch side of an assert condition (a side that decides the condition false), a message argument (evaluated only on
failure), a panic! or unreachable! invocation, the lines of a match/else arm whose last statement is a panic! (they only build
its message), a let-else whose else block panics, and the `_ => false` region an assert!(matches!(..)) attributes to its call
line. Classified by a brace-matching reader of each invocation (scratchpad classify.mjs), then the GAP rows read by hand; the
(U) rows were each read and proved by hand. A row here that is not such a side is a GAP: say so and it moves.

(U) ITEMS: 4
(U) BRANCH crates/indicators/src/vwap.rs:212:11 False  while i < NEWTON_STEPS {
    Proof: the loop leaves only through `break`. The seed g0 = 2^ceil(bits/2) lies in [s, 2s] for s = sqrt(v) (the comment above it). Integer Newton from above never goes below floor(s), and its real excess e = g - s obeys e' = e^2 / (2g) <= e^2 / (2s): e0 <= s, e1 <= s/2, e2 <= s/8, e3 <= s/2^7, e4 <= s/2^15, e5 <= s/2^31, e6 <= s/2^63 < 2 for every s < 2^64 (sqrt(i128::MAX) < 2^64), e7 < 1; so within 8 steps g reaches floor(s) and within 10 the step that does not decrease breaks. 10 < NEWTON_STEPS = 16 for every i128 input, and the measured worst case is 8 (vwap::tests::the_iteration_ceiling_is_the_newton_budget). No input exhausts the loop.
(U) LINE   crates/lake/src/schema.rs:126  return Err(LakeError::MissingColumn { name: s.name });
    Proof: the body of the `i >= n` refusal above: same proof.
(U) BRANCH crates/lake/src/schema.rs:125:12 True  if i >= n {
    Proof: detect picks `layout` from `n` (n == Layout::Cash.column_count() -> Cash, n == Layout::Fno.column_count() -> Fno, any other n returns UnexpectedSchema at the line above), and specs(layout) yields exactly column_count(layout) items (CASH_SPEC, then FNO_EXTRA_SPEC for Fno; pinned by schema::tests::cash_is_a_strict_prefix_of_fno), so the enumerate index i runs over 0..n and `i >= n` is false on every iteration. detect takes only a SchemaDescriptor and computes the layout itself, so no file and no direct call reaches the refusal; it is kept because it turns a future drift between the count rule and the spec tables into a named refusal instead of a panic in `schema.column(i)`.
(U) LINE   crates/vocab/src/table.rs:1704  None
    Proof: the `None` after the probe loop. NAME_INDEX = name_index(&TABLE) fills exactly TABLE.len() (COUNT, pinned by table::tests::every_name_is_found_by_its_index_and_no_other_token_is) of its NAME_SLOTS = 2048 slots, so free slots exist; the probe steps `at = (at + 1) & (NAME_SLOTS - 1)` from any start visit every slot once in NAME_SLOTS steps, so every probe meets either the name (returns Some) or a free slot (returns None through `checked_sub(1)?`) before the loop ends. No input reaches the loop exhaustion; it bounds the walk should the table ever fill.

NEEDS THE MAC: 353 rows, for these reasons:
  - every test of this file is #[ignore]d: it needs ~/.brutex/lake (real vendor data, which CI gate 1 keeps out of the repository). Run on the Mac with `cargo test -p lake -- --include-ignored` under Route B.
  - as real_lake.rs: #[ignore]d, needs ~/.brutex/lake.
  - the #[ignore]d real-lake scan; its helpers (scan_file, page_values, month_files, sample_dirs) are driven on synthetic files by the_real_lake_scan_finds_each_defect_in_a_file_made_to_carry_it and the_real_lake_walk_finds_month_files_and_samples_by_stride, so only this body waits for the lake.

(F) ITEMS: 428
(F) assert! condition (failing side)  BRANCH crates/indicators/src/anchored.rs:2247:28 False  if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
(F) assert! condition (failing side)  BRANCH crates/indicators/src/anchored.rs:2813:17 False  accepted > 500 && refused > 500,
(F) assert! condition (failing side)  BRANCH crates/indicators/src/anchored.rs:2813:35 False  accepted > 500 && refused > 500,
(F) assert_eq! message argument  LINE   crates/indicators/src/column.rs:1672  family.len()
(F) assert! message argument  LINE   crates/indicators/src/column.rs:2149  poisoned_column.bits().len()
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:1789:25 False  assert!(!truth.get(bit) && !known.get(bit));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:1789:44 False  assert!(!truth.get(bit) && !known.get(bit));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:1792:25 False  assert!(!truth.get(bit) && !known.get(bit));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:1792:44 False  assert!(!truth.get(bit) && !known.get(bit));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:2326:13 False  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:2326:62 False  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:2609:13 False  projected.sources().contains(&22) && projected.sources().contains(&24),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/column.rs:2609:50 False  projected.sources().contains(&22) && projected.sources().contains(&24),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:869:17 False  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:869:31 False  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1114:17 False  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1114:30 False  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1114:43 False  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1115:17 False  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1115:31 False  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1116:17 False  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1116:30 False  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1137:17 False  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1137:30 False  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1137:43 False  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1138:17 False  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1138:30 False  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1138:43 False  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1139:17 False  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1139:31 False  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1140:17 False  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1140:31 False  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1141:17 False  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1141:30 False  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1554:13 False  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1554:44 False  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1573:13 False  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/daily.rs:1573:47 False  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:1586:78 False  Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:1891:17 False  assert!(truth_only.warmed_up() && full.warmed_up());
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:1891:43 False  assert!(truth_only.warmed_up() && full.warmed_up());
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:1982:17 False  !first.get(above) && !first.get(below),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:1982:38 False  !first.get(above) && !first.get(below),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:3564:13 False  !first.get(276) && !first.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:3564:32 False  !first.get(276) && !first.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:3588:13 False  !flat.get(276) && !flat.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:3588:31 False  !flat.get(276) && !flat.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/evaluator.rs:3651:36 True  !(mask.get(278) && mask.get(279)),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/fib.rs:584:21 False  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/fib.rs:584:31 False  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/gap.rs:1491:13 False  fired > 0 && legs > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/src/gap.rs:1491:26 False  fired > 0 && legs > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:555:13 False  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:555:40 False  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:559:13 False  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:559:40 False  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:610:17 False  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:610:40 False  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/orb.rs:610:63 False  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1558:17 False  assert!(shape.bullish() && !shape.bearish());
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1558:36 False  assert!(shape.bullish() && !shape.bearish());
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1562:17 False  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1562:44 False  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1597:13 False  m2.get(153) && m2.get(155),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1597:28 False  m2.get(153) && m2.get(155),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/pattern.rs:1711:41 True  !(mask.get(bull) && mask.get(bear)),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:586:17 False  assert!(truth.get(66) && !truth.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:586:34 False  assert!(truth.get(66) && !truth.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:587:17 False  assert!(known.get(66) && known.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:587:34 False  assert!(known.get(66) && known.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:589:13 False  !truth.get(68) && !known.get(68),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:589:31 False  !truth.get(68) && !known.get(68),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:655:13 False  !bits.get(42) && !bits.get(43),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:655:30 False  !bits.get(42) && !bits.get(43),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:680:13 False  !edge.get(44) && edge.get(45),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:680:30 False  !edge.get(44) && edge.get(45),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:788:17 False  assert!(up.get(30) && !up.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:788:31 False  assert!(up.get(30) && !up.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:790:17 False  assert!(down.get(31) && !down.get(30));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:790:33 False  assert!(down.get(31) && !down.get(30));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:793:13 False  !flat.get(30) && !flat.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:793:30 False  !flat.get(30) && !flat.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:838:17 False  assert!(!mask.get(37) && !mask.get(38));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:838:34 False  assert!(!mask.get(37) && !mask.get(38));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:858:17 False  !mask.get(30) && !mask.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:858:34 False  !mask.get(30) && !mask.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:922:13 False  !flat.get(30) && !flat.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:922:30 False  !flat.get(30) && !flat.get(31),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:931:17 False  assert!(!mask.get(37) && !mask.get(38));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:931:34 False  assert!(!mask.get(37) && !mask.get(38));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:950:13 False  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:950:30 False  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:950:47 False  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:966:17 False  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:966:33 False  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:968:17 False  assert!(mask.get(66) && !mask.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:968:33 False  assert!(mask.get(66) && !mask.get(67));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:972:17 False  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:972:34 False  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:974:13 False  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:974:30 False  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:974:47 False  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:989:17 False  assert!(mask.get(50) && !mask.get(51));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:989:33 False  assert!(mask.get(50) && !mask.get(51));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:993:17 False  assert!(mask.get(51) && !mask.get(50));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:993:33 False  assert!(mask.get(51) && !mask.get(50));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:1283:13 False  !mask.get(66) && !mask.get(67),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/session.rs:1283:30 False  !mask.get(66) && !mask.get(67),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1292:13 False  !mask.get(0) && !mask.get(1),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1292:29 False  !mask.get(0) && !mask.get(1),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1296:13 False  !mask.get(2) && !mask.get(3),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1296:29 False  !mask.get(2) && !mask.get(3),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1300:13 False  !mask.get(4) && !mask.get(5),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:1300:29 False  !mask.get(4) && !mask.get(5),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/trend.rs:2302:74 False  matches!((previous, now), (Some(before), Some(after)) if after < before),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:791:17 False  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:791:34 False  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:792:17 False  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:792:34 False  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:904:17 False  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:904:44 False  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:1250:21 False  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F) assert! condition (failing side)  BRANCH crates/indicators/src/vwap.rs:1250:52 False  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/crossing_known_readiness.rs:84:32 True  !(above && below),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:13 False  false_count > 0 && true_count > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:32 False  false_count > 0 && true_count > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:13 False  trues > 0 && falses > 0 && unknowns > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:26 False  trues > 0 && falses > 0 && unknowns > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:40 False  trues > 0 && falses > 0 && unknowns > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:13 False  !truth.get(bit) && !known.get(bit),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:32 False  !truth.get(bit) && !known.get(bit),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:17 False  !truth.get(bit) && !known.get(bit),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:36 False  !truth.get(bit) && !known.get(bit),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/fib_known_readiness.rs:176:13 False  !truth.get(retired) && !known.get(retired),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/fib_known_readiness.rs:176:36 False  !truth.get(retired) && !known.get(retired),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/fib_known_readiness.rs:231:13 False  trues > 0 && falses > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/fib_known_readiness.rs:231:26 False  trues > 0 && falses > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/gap_known_readiness.rs:224:21 False  assert!(false_count > 0 && true_count > 0);
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/gap_known_readiness.rs:224:40 False  assert!(false_count > 0 && true_count > 0);
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/gap_known_readiness.rs:264:17 False  assert!(false_count > 0 && true_count > 0);
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/gap_known_readiness.rs:264:36 False  assert!(false_count > 0 && true_count > 0);
(F) assert_eq! message argument  LINE   crates/indicators/tests/module_doc_counts.rs:271  positions.len()
(F) assert_eq! message argument  LINE   crates/indicators/tests/module_doc_counts.rs:321  positions.len()
(F) assert_eq! message argument  LINE   crates/indicators/tests/module_doc_counts.rs:332  emitted.len()
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/orb_known_readiness.rs:195:13 False  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/orb_known_readiness.rs:195:32 False  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/orb_known_readiness.rs:195:50 False  false_count > 0 && true_count > 0 && unknown_count > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_from_outside.rs:200:9 False  mask.get(46) && !mask.get(47),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_from_outside.rs:200:25 False  mask.get(46) && !mask.get(47),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:367:13 False  !truth.get(position) && !known.get(position),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:367:37 False  !truth.get(position) && !known.get(position),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:371:13 False  assert!(known.get(48) && known.get(49));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:371:30 False  assert!(known.get(48) && known.get(49));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:372:13 False  assert!(!truth.get(48) && !truth.get(49));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:372:31 False  assert!(!truth.get(48) && !truth.get(49));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:459:13 False  assert!(known.get(33) && !truth.get(33));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:459:30 False  assert!(known.get(33) && !truth.get(33));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:460:13 False  assert!(!known.get(37) && !known.get(66));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:460:31 False  assert!(!known.get(37) && !known.get(66));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:511:13 False  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:511:30 False  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/session_known_readiness.rs:511:47 False  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:9 False  hammers > 0 && hammers < cases,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:24 False  hammers > 0 && hammers < cases,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:13 False  assert!(bullish > 0 && bearish > 0);
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:28 False  assert!(bullish > 0 && bearish > 0);
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:17 False  assert!(!truth.get(30) && !truth.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:35 False  assert!(!truth.get(30) && !truth.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:17 False  assert!(known.get(30) && known.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:34 False  assert!(known.get(30) && known.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:17 False  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:34 False  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:51 False  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:68 False  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:9 False  !next_day.get(157) && !next_day.get(176),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:31 False  !next_day.get(157) && !next_day.get(176),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:9 False  !next_day.get(276) && !next_day.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:31 False  !next_day.get(276) && !next_day.get(277),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:17 False  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:47 False  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:21 False  assert!(known.get(30) && known.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:38 False  assert!(known.get(30) && known.get(31));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:17 False  assert!(!truth.get(13) && !truth.get(16));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:35 False  assert!(!truth.get(13) && !truth.get(16));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:13 False  known.get(13) && known.get(16),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:30 False  known.get(13) && known.get(16),
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/trend_known_readiness.rs:183:45 False  assert!(!truth.get(position) || known.get(position));
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/trend_known_readiness.rs:257:13 False  trues > 0 && falses > 0 && unknowns > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/trend_known_readiness.rs:257:26 False  trues > 0 && falses > 0 && unknowns > 0,
(F) assert! condition (failing side)  BRANCH crates/indicators/tests/trend_known_readiness.rs:257:40 False  trues > 0 && falses > 0 && unknowns > 0,
(F) panic! (failing arm)  LINE   crates/lake/src/bar.rs:237  other => panic!("expected NotRepresentable, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/bar.rs:247  other => panic!("expected NotRepresentable, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:412  ContractKind::Future => panic!("a CE is not a future"),
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:563  other => panic!("expected TooLong, got {other:?}"),
(F) assert! message argument  LINE   crates/lake/src/contract.rs:571  name.len()
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:614  ContractKind::Future => panic!("not a future"),
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:687  other => panic!("{padded} must be refused, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:721  other => panic!("{signed} must be refused, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/contract.rs:753  other => panic!("{name} must be refused, got {other:?}"),
(F) assert! condition (failing side)  BRANCH crates/lake/src/contract.rs:786:61 False  Err(ContractError::BadExpiryShape { found }) if found == "0\u{e9}Apr2"
(F) panic! (failing arm)  LINE   crates/lake/src/footer_tests.rs:33  panic!("expected the walk to admit, got {e:?}");
(F) panic! (failing arm)  LINE   crates/lake/src/footer_tests.rs:40  other => panic!("expected FooterUnreadable, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/footer_tests.rs:69  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/footer_tests.rs:80  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F) if-let Err in test (the taken side panics)  BRANCH crates/lake/src/footer_tests.rs:32:12 True  if let Err(e) = r {
(F) panic! (failing arm)  LINE   crates/lake/src/page.rs:878  other => panic!("expected a data page, got {other:?}"),
(F) assert! message argument  LINE   crates/lake/src/page.rs:986  frame.len(),
(F) assert! message argument  LINE   crates/lake/src/page.rs:987  plain.len()
(F) panic! (failing arm)  LINE   crates/lake/src/page.rs:1016  other => panic!("expected a data page, got {other:?}"),
(F) assert! message argument  LINE   crates/lake/src/page.rs:1147  frame.len()
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:631:13 False  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:631:54 False  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:640:13 False  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:640:53 False  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:681:13 False  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:681:65 False  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:741:13 False  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:741:51 False  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:1192:13 False  text.contains("zstd frame refused") && text.contains("134217729"),
(F) assert! condition (failing side)  BRANCH crates/lake/src/page.rs:1192:52 False  text.contains("zstd frame refused") && text.contains("134217729"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:900  other => panic!("expected TooLarge, got {:?}", other.err()),
(F) assert!(matches!(..)) failing arm (the `_ => false` region at the call site)  LINE   crates/lake/src/reader.rs:912  matches!(
(F) assert!(matches!(..)) failing arm (the `_ => false` region at the call site)  LINE   crates/lake/src/reader.rs:919  matches!(
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:953  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:966  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:979  other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1053  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1099  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1116  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1146  other => panic!("expected UnexpectedNull, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1281  _ => panic!("the cash layout is INT64 and DOUBLE only"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1388  other => panic!("expected ImpossibleLength, got {:?}", other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1422  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1423  Ok(batch) => panic!(
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1425  batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1479  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1480  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1530  other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1554  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1555  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1626  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1627  Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1727  other => panic!("the padded group was not refused as short: {other:?}"),
(F) panic! failing arm  LINE   crates/lake/src/reader.rs:1756  match cut {  (the zero-count region on `match cut {` is the `Err(RowCountsDisagree)` arm, whose body is the panic on the next lines; the other arm runs.)
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1758  panic!("the cut and its copy sum to the file count")
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1801  other => panic!("group {group} under {claim}: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1896  crate::error::ColumnType::Other => panic!("no lake column is Other"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1943  _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:1983  other => panic!("expected ColumnTypeMismatch, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:2061  other => panic!("{lie}: expected ImpossibleLength, got {:?}", other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:2117  panic!("the patched length did not settle");
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:2139  other => panic!("{past} past: expected PageDecode, got {:?}", other.err()),
(F) panic! (failing arm)  LINE   crates/lake/src/reader.rs:2150  other => panic!("a chunk ending at the last byte: {:?}", other.err()),
(F) assert! condition (failing side)  BRANCH crates/lake/src/reader.rs:2166:20 False  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F) assert! condition (failing side)  BRANCH crates/lake/src/reader.rs:2166:45 False  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F) panic! (failing arm)  LINE   crates/lake/src/reader_cover_tests.rs:23  other => panic!("a missing file must be an Io refusal: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/reader_cover_tests.rs:36  other => panic!("/dev/zero under cap {cap}: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/schema_cover_tests.rs:58  ColumnType::Int32 | ColumnType::Other => panic!("the cash layout has neither"),
(F) panic! (failing arm)  LINE   crates/lake/src/schema_cover_tests.rs:97  other => panic!("{shown}: expected UnsupportedTimestamp, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/src/zstd_v082_frames.rs:175  other => panic!("no synthetic input called {other}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/events.rs:159  _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
(F) panic! (failing arm)  LINE   crates/lake/tests/events.rs:366  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/events.rs:401  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:176  _ => panic!("the cash layout has no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:234  _ => panic!("the cash layout has no other physical type"),
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:310  Ok(batch) => {
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:311  let got: Vec<Option<i64>> = (0..batch.len())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:312  .map(|i| batch.row(i).expect("row in range").open_interest())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:313  .collect();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:314  let real = got.iter().filter(|v| v.is_some()).count();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:315  let invented = got.len() - real;
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:316  let first = got.iter().position(Option::is_none);
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:317  let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:318  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:326  batch.len()
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:377  Ok(batch) => {
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:378  let got: Vec<Option<i64>> = (0..batch.len())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:379  .map(|i| batch.row(i).expect("row in range").open_interest())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:380  .collect();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:381  let real = got.iter().filter(|v| v.is_some()).count();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:382  let first = got.iter().position(Option::is_none);
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:383  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:386  batch.len(),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:387  got.len() - real,
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:388  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:432  Ok(batch) => {
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:433  let got: Vec<Option<i64>> = (0..batch.len())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:434  .map(|i| batch.row(i).expect("row in range").open_interest())
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:435  .collect();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:436  let real = got.iter().filter(|v| v.is_some()).count();
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:437  let first = got.iter().position(Option::is_none);
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:438  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:442  batch.len(),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:443  got.len() - real,
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:444  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:558  _ => panic!("one INT64 column"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:682  _ => panic!("the cash layout has no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:703  Err(other) => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:715  Ok(file) => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:795  _ => panic!("the cash layout has no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:814  other => panic!("a {what} leaf must be refused by name, got {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1062  panic!("every leaf is INT64");
(F) assert! message argument  LINE   crates/lake/tests/refusals.rs:1203  s.unreadable[0]
(F) assert! message argument  LINE   crates/lake/tests/refusals.rs:1240  s.unreadable[0]
(F) panic! failing arm (the arm's own lines)  LINE   crates/lake/tests/refusals.rs:1558  Ok(other) => {
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1559  panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1569  other.to_string() == padded,
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1570  other == good,
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1571  other.key() == good.key()
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1699  _ => panic!("the cash layout has no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1841  _ => panic!("the cash layout has no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1870  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1871  Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1949  _ => panic!("this fixture writes no other physical type"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1980  Err(other) => panic!("refused as the wrong thing: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1981  Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:1999  Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2000  Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2050  Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2051  Ok(_) => panic!("an INT96 timestamp opened"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2101  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2102  Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
(F) panic! (failing arm)  LINE   crates/lake/tests/refusals.rs:2137  other => panic!("an INT32 {leaf}: {:?}", other.map(|f| f.num_rows())),
(F) let-else in test (the else side panics)  BRANCH crates/lake/tests/refusals.rs:1061:17 False  let ColumnWriter::Int64ColumnWriter(typed) = column.untyped() else {
(F) assert! condition (failing side)  BRANCH crates/lake/tests/refusals.rs:1619:9 False  text.contains("400") && text.contains("2480"),
(F) assert! condition (failing side)  BRANCH crates/lake/tests/refusals.rs:1619:33 False  text.contains("400") && text.contains("2480"),
(F) assert! condition (failing side)  BRANCH crates/lake/tests/refusals.rs:1748:71 False  Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:136  _ => panic!("writer type does not match the column"),
(F) assert! message argument  LINE   crates/lake/tests/synthetic.rs:231  f.created_by()
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:353  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:355  other = other.err()
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:403  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:405  other = other.err()
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:453  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:455  other = other.err()
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:473  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:475  other = other.err()
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:626  other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:639  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:641  other = other.err()
(F) assert!(matches!(..)) failing arm (the `_ => false` region at the call site)  LINE   crates/lake/tests/synthetic.rs:662  matches!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:780  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:782  other.map(|b| b.len())
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:820  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:822  other.map(|b| b.len())
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:952  other => panic!("page type {} is not one this writer emits", other.0),
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:1154  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:1156  other.map(|groups| groups.len())
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:1221  other => panic!(
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:1223  other.map(|b| b.len())
(F) panic! (failing arm)  LINE   crates/lake/tests/synthetic.rs:1307  Ok(rows) => panic!("decoded {} rows from a broken page", rows.len()),
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:506:13 False  compared > 100_000 && admitted > 1_000,
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:506:35 False  compared > 100_000 && admitted > 1_000,
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:515:21 False  assert!(!admits(code, length) && !valid_prefix(code, length));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:515:46 False  assert!(!admits(code, length) && !valid_prefix(code, length));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:626:17 False  assert!(choice < call && call < paused);
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:626:34 False  assert!(choice < call && call < paused);
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:671:17 False  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:671:54 False  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:742:17 False  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F) assert! condition (failing side)  BRANCH crates/vocab/src/expression_search.rs:742:37 False  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F) assert! message argument  LINE   crates/vocab/src/implication.rs:329  rows.len()
(F) assert! condition (failing side)  BRANCH crates/vocab/src/implication.rs:292:63 True  assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/mask.rs:419:46 False  !bar.hits(&wider) || bar.hits(&base),
(F) assert! condition (failing side)  BRANCH crates/vocab/src/mask.rs:661:17 False  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/mask.rs:661:32 False  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/mask.rs:661:49 False  assert!(less.get(5) && !less.get(70) && less.get(200));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:1977:13 False  worst_hit <= 4 && worst_miss <= 7,
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:1977:31 False  worst_hit <= 4 && worst_miss <= 7,
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:2034:17 False  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:2034:34 False  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:2034:52 False  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:2108:17 False  assert!(m.get(0) && m.popcount() == 1);
(F) assert! condition (failing side)  BRANCH crates/vocab/src/table.rs:2108:29 False  assert!(m.get(0) && m.popcount() == 1);
(F) assert! condition (failing side)  BRANCH crates/vocab/src/tolerance.rs:570:45 True  !(shared && range >= RUNG_EXCLUSIVE_MIN_RANGE),
(F) assert_eq! message argument  LINE   crates/vocab/tests/expression.rs:403  bytes.get(..16)
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/expression.rs:414:13 False  assert!(accepted > 0 && refused > accepted);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/expression.rs:414:29 False  assert!(accepted > 0 && refused > accepted);
(F) panic! (failing arm)  LINE   crates/vocab/tests/expression_search.rs:82  Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/expression_search.rs:201:13 False  assert!(saw_candidate && saw_backtrack);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/expression_search.rs:201:30 False  assert!(saw_candidate && saw_backtrack);
(F) panic! (failing arm)  LINE   crates/vocab/tests/expression_search_readiness.rs:127  panic!("the last NOT must finish the maximum-capacity program");
(F) let-else in test (the else side panics)  BRANCH crates/vocab/tests/expression_search_readiness.rs:126:9 False  let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/mask.rs:315:27 False  before || !after,
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_float.rs:145:36 False  *module == "lib.rs" || declared.contains(*module),
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:282  root.display(),
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:283  String::from_utf8_lossy(&listed.stderr).trim()
(F) panic! (failing arm)  LINE   crates/vocab/tests/no_py_language.rs:448  panic!("{why}");
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:666  String::from_utf8_lossy(&init.stderr)
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:679  String::from_utf8_lossy(&add.stderr)
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:783  String::from_utf8_lossy(&init.stderr)
(F) assert! message argument  LINE   crates/vocab/tests/no_py_language.rs:815  String::from_utf8_lossy(&add.stderr)
(F) if-let Err in test (the taken side panics)  BRANCH crates/vocab/tests/no_py_language.rs:447:12 True  if let Err(why) = verdict(&found) {
(F) assert! message argument  LINE   crates/vocab/tests/no_runtime_spawn.rs:2560  sources.len()
(F) assert! message argument  LINE   crates/vocab/tests/no_runtime_spawn.rs:2585  failures.join("\n  ")
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:2910:26 False  .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:13 False  assert!(out.found.is_empty() && out.in_tests == 0);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:37 False  assert!(out.found.is_empty() && out.in_tests == 0);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:13 False  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:50 False  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:13 False  assert!(block.found.is_empty() && block.in_tests == 0);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:39 False  assert!(block.found.is_empty() && block.in_tests == 0);
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:13 False  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:41 False  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:67 False  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:13 False  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:42 False  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:13 False  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:47 False  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F) assert! message argument  LINE   crates/vocab/tests/stale_claims.rs:158  with_commas(count)
(F) assert! message argument  LINE   crates/vocab/tests/table.rs:125  doc.len(),
(F) unreachable! (failing arm)  LINE   crates/vocab/tests/table.rs:271  unreachable!("235..=273 are void")
(F) assert! message argument  LINE   crates/vocab/tests/table.rs:432  first - 1,
(F) let-else in test (the else side panics)  BRANCH crates/vocab/tests/table.rs:270:13 False  let BitStatus::Void { reason } = def.status else {
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:373:13 False  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:373:39 False  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:373:63 False  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:561:45 False  n == ConditionMask::BITS || ALLOWED.contains(&n),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:813:17 False  !status_cells.contains("retired") && !status_cells.contains("void"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:813:54 False  !status_cells.contains("retired") && !status_cells.contains("void"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:819:21 False  status_cells.contains("retired")
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:820:28 False  && status_cells.contains(&format!("duplicates {duplicate_of}")),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:843:9 False  retired_rows >= retired && void_rows >= void,
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:843:36 False  retired_rows >= retired && void_rows >= void,
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:1012:13 False  !name.contains("vix") && !name.contains("volatility_index"),
(F) assert! condition (failing side)  BRANCH crates/vocab/tests/table.rs:1012:38 False  !name.contains("vix") && !name.contains("volatility_index"),
(F) assert! message argument  LINE   crates/vocab/tests/workspace_is_rust.rs:397  out.len()
(F) assert! message argument  LINE   crates/vocab/tests/workspace_is_rust.rs:753  named.join("\n  ")

APPENDIX LS4: the fenced content of reports/base-cloud-20261103/fu-LS4.txt.md on branch cloud/cov-libs-ls4-20261103, verbatim
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

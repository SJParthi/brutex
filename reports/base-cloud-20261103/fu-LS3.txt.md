# fu-LS3: the (F) and (U) rows of lake, indicators and vocab

One fenced block: the list as plain text (CLOUD.md item 10).

`````text
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
`````

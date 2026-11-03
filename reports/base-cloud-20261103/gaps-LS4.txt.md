# gaps-LS4.txt.md

BASE-COV-LIBS-LS4's GAP list at code tip `bcbc6524bdb29351f5d211aed11e4f7b76795b7b`: Route B figures per crate and per file, the NOT PROVED files, and every uncovered line or side that is neither (F) nor (U), with its owner. One fenced block below holds it byte for byte; sha256 of the fenced content: `ba7997086deb31de0942d7730c59c1a1bb1be73564ea695efe3eec2e096d062f`.

`````text
BASE-COV-LIBS-LS4, Route B on tip bcbc6524bdb29351f5d211aed11e4f7b76795b7b (crates core, costs, engine, greeks, telemetry; whole files; non-root run).
Pin: rustc 1.99.0-nightly (1a98b1e13 2026-08-07), llvm-cov/llvm-profdata LLVM version 23.1.0-rust-1.99.0-nightly, x86_64-unknown-linux-gnu.
RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', CARGO_PROFILE_DEV_OPT_LEVEL=0, kit d3-route-b-cloud.sh (unchanged) with d3-covset.mjs.
Test binaries ran as uid 65534 (nobody) through CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups', umask 000 for the profile folders.

TOTALS per crate (lines covered/counted, branches covered/counted)
  core       lines 7746/7774 (uncovered 28)  branches 602/640 (uncovered 38)  files read 21
  costs      lines 11568/11670 (uncovered 102)  branches 480/502 (uncovered 22)  files read 26
  engine     lines 5448/5478 (uncovered 30)  branches 530/570 (uncovered 40)  files read 11
  greeks     lines 1918/1921 (uncovered 3)  branches 93/96 (uncovered 3)  files read 7
  telemetry  lines 9370/9397 (uncovered 27)  branches 636/678 (uncovered 42)  files read 16
  ALL        lines 36050/36240 (uncovered 190)  branches 2341/2486 (uncovered 145)  files read 81

NOT PROVED by Route B (9): crates/core/benches/ratio.rs crates/core/src/lib.rs crates/costs/benches/ratio.rs crates/costs/benches/stack.rs crates/costs/src/lib.rs crates/engine/benches/ratio.rs crates/greeks/benches/ratio.rs crates/greeks/src/lib.rs crates/telemetry/benches/ratio.rs
  benches/ files: L-BCL-1 (test = false, so cargo test builds no bench object); lib.rs files with no executable line: L-BCL-2.

PER FILE
  crates/core/src/alias.rs lines 857/859 branches 53/56  UNCOVERED
  crates/core/src/alias_cover_tests.rs lines 90/90 branches 0/0
  crates/core/src/blake3.rs lines 395/395 branches 16/16
  crates/core/src/error.rs lines 92/92 branches 4/4
  crates/core/src/instrument.rs lines 849/849 branches 50/54  UNCOVERED
  crates/core/src/isin.rs lines 140/140 branches 14/14
  crates/core/src/knob.rs lines 69/69 branches 8/8
  crates/core/src/price.rs lines 320/320 branches 28/28
  crates/core/src/secret_shape.rs lines 239/239 branches 34/34
  crates/core/src/symbol.rs lines 109/109 branches 4/4
  crates/core/src/universe.rs lines 966/968 branches 61/82  UNCOVERED
  crates/core/src/vendor.rs lines 1578/1579 branches 83/84  UNCOVERED
  crates/core/tests/banned_lists.rs lines 85/85 branches 2/4  UNCOVERED
  crates/core/tests/bound_probe.rs lines 35/35 branches 0/0
  crates/core/tests/claim_reader.rs lines 72/72 branches 2/4  UNCOVERED
  crates/core/tests/cost_invariants.rs lines 232/240 branches 30/30  UNCOVERED
  crates/core/tests/decision_citations.rs lines 433/439 branches 72/72  UNCOVERED
  crates/core/tests/findings.rs lines 494/501 branches 24/26  UNCOVERED
  crates/core/tests/graph.rs lines 496/496 branches 100/102  UNCOVERED
  crates/core/tests/lint.rs lines 107/108 branches 12/12  UNCOVERED
  crates/core/tests/zerodha_index.rs lines 88/89 branches 5/6  UNCOVERED
  crates/costs/src/charge_error.rs lines 184/184 branches 0/0
  crates/costs/src/dated.rs lines 386/386 branches 32/32
  crates/costs/src/day.rs lines 442/442 branches 28/30  UNCOVERED
  crates/costs/src/dhan_calc.rs lines 467/467 branches 30/30
  crates/costs/src/error.rs lines 283/283 branches 4/4
  crates/costs/src/exact.rs lines 305/305 branches 14/14
  crates/costs/src/expiry.rs lines 622/622 branches 28/28
  crates/costs/src/fill.rs lines 443/447 branches 16/16  UNCOVERED
  crates/costs/src/ledger.rs lines 877/880 branches 24/24  UNCOVERED
  crates/costs/src/lot.rs lines 667/670 branches 74/78  UNCOVERED
  crates/costs/src/lot_table_tests.rs lines 333/355 branches 10/14  UNCOVERED
  crates/costs/src/money.rs lines 153/153 branches 4/6  UNCOVERED
  crates/costs/src/moneyness.rs lines 355/355 branches 18/18
  crates/costs/src/rate.rs lines 117/117 branches 0/0
  crates/costs/src/regime.rs lines 534/534 branches 26/26
  crates/costs/src/schedule.rs lines 729/734 branches 12/14  UNCOVERED
  crates/costs/src/schedule_cover_tests.rs lines 68/68 branches 4/4
  crates/costs/src/scope.rs lines 82/83 branches 0/0  UNCOVERED
  crates/costs/src/stack.rs lines 1610/1612 branches 84/86  UNCOVERED
  crates/costs/src/stack_cover_tests.rs lines 51/51 branches 0/0
  crates/costs/src/strike.rs lines 317/317 branches 10/10
  crates/costs/src/trip.rs lines 1558/1558 branches 32/32
  crates/costs/src/venue.rs lines 77/77 branches 2/2
  crates/costs/tests/adversarial.rs lines 139/149 branches 10/16  UNCOVERED
  crates/costs/tests/dhan_calculator.rs lines 444/446 branches 12/12  UNCOVERED
  crates/costs/tests/fa73061.rs lines 325/375 branches 6/6  UNCOVERED
  crates/engine/src/column.rs lines 407/407 branches 16/16
  crates/engine/src/keep.rs lines 434/434 branches 24/28  UNCOVERED
  crates/engine/src/lib.rs lines 2210/2232 branches 128/156  UNCOVERED
  crates/engine/src/manifest.rs lines 285/285 branches 112/112
  crates/engine/src/refusal_cover_tests.rs lines 101/104 branches 2/2  UNCOVERED
  crates/engine/src/resume.rs lines 546/546 branches 132/132
  crates/engine/tests/adversarial.rs lines 159/159 branches 4/4
  crates/engine/tests/join_probe_screen.rs lines 31/31 branches 0/0
  crates/engine/tests/production_callers.rs lines 238/239 branches 34/36  UNCOVERED
  crates/engine/tests/resume_readiness.rs lines 851/855 branches 49/54  UNCOVERED
  crates/engine/tests/sweep_readiness_oracle.rs lines 186/186 branches 29/30  UNCOVERED
  crates/greeks/src/bsm.rs lines 524/524 branches 20/20
  crates/greeks/src/error.rs lines 100/100 branches 2/2
  crates/greeks/src/moneyness.rs lines 145/145 branches 4/4
  crates/greeks/src/normal.rs lines 143/143 branches 14/14
  crates/greeks/src/solver.rs lines 513/515 branches 41/44  UNCOVERED
  crates/greeks/tests/standalone.rs lines 59/59 branches 0/0
  crates/greeks/tests/vendor_anchor.rs lines 434/435 branches 12/12  UNCOVERED
  crates/telemetry/src/clock.rs lines 192/192 branches 14/14
  crates/telemetry/src/encode.rs lines 206/206 branches 20/20
  crates/telemetry/src/event.rs lines 87/87 branches 0/0
  crates/telemetry/src/json.rs lines 341/341 branches 16/16
  crates/telemetry/src/level.rs lines 68/69 branches 0/0  UNCOVERED
  crates/telemetry/src/lib.rs lines 203/203 branches 16/18  UNCOVERED
  crates/telemetry/src/record.rs lines 425/426 branches 28/28  UNCOVERED
  crates/telemetry/src/run_dirs.rs lines 79/79 branches 4/8  UNCOVERED
  crates/telemetry/src/sink.rs lines 6087/6106 branches 440/468  UNCOVERED
  crates/telemetry/src/sink_recovery_tests.rs lines 246/246 branches 6/8  UNCOVERED
  crates/telemetry/src/state_events.rs lines 28/28 branches 12/12
  crates/telemetry/src/state_events_tests.rs lines 127/130 branches 6/10  UNCOVERED
  crates/telemetry/src/tail.rs lines 863/863 branches 72/74  UNCOVERED
  crates/telemetry/src/value.rs lines 142/142 branches 0/0
  crates/telemetry/tests/cloud_mutants_sink.rs lines 236/237 branches 2/2  UNCOVERED
  crates/telemetry/tests/global_absent.rs lines 40/42 branches 0/0  UNCOVERED

GAP ROWS (106: every uncovered line or side that is neither (F) nor (U); fu-LS4.txt.md holds the 229 (F)/(U) rows)
GAP LINE   crates/costs/src/lot_table_tests.rs:382  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn a_lookup_is_flat_in_the_table() {
GAP LINE   crates/costs/src/lot_table_tests.rs:383  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn table_of(underlyings: usize) -> (LotTable, Vec<String>) {
GAP LINE   crates/costs/src/lot_table_tests.rs:384  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let names: Vec<String> = (0..underlyings).map(|at| format!("U{at:05}")).collect();
GAP LINE   crates/costs/src/lot_table_tests.rs:385  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let mut rows = Vec::new();
GAP LINE   crates/costs/src/lot_table_tests.rs:386  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for name in &names {
GAP LINE   crates/costs/src/lot_table_tests.rs:387  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for step in 0..MAX_LOT_ROWS_PER_UNDERLYING {
GAP LINE   crates/costs/src/lot_table_tests.rs:388  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let ordinal = day(2018, 1, 1).ordinal() + i32::try_from(step * 45).unwrap();
GAP LINE   crates/costs/src/lot_table_tests.rs:389  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  rows.push(sourced(name, TradeDay::from_ordinal(ordinal).unwrap(), 25));
GAP LINE   crates/costs/src/lot_table_tests.rs:390  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }
GAP LINE   crates/costs/src/lot_table_tests.rs:392  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  (LotTable::from_rows(rows).unwrap(), names)
GAP LINE   crates/costs/src/lot_table_tests.rs:393  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }
GAP LINE   crates/costs/src/lot_table_tests.rs:394  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (small, small_names) = table_of(2);
GAP LINE   crates/costs/src/lot_table_tests.rs:395  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (large, large_names) = table_of(1_500);
GAP LINE   crates/costs/src/lot_table_tests.rs:396  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let n = 100_000;
GAP LINE   crates/costs/src/lot_table_tests.rs:397  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (s50, s99, smax) = time_lookups(&small, &small_names, n);
GAP LINE   crates/costs/src/lot_table_tests.rs:398  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (l50, l99, lmax) = time_lookups(&large, &large_names, n);
GAP LINE   crates/costs/src/lot_table_tests.rs:399  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/src/lot_table_tests.rs:403  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(
GAP LINE   crates/costs/src/lot_table_tests.rs:404  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  l99 <= s99.saturating_mul(10).max(2_000),
GAP LINE   crates/costs/src/lot_table_tests.rs:407  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }
GAP LINE   crates/costs/tests/fa73061.rs:549  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn the_dated_lookups_cost_the_same_on_every_row_measured() {
GAP LINE   crates/costs/tests/fa73061.rs:552  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let days = [
GAP LINE   crates/costs/tests/fa73061.rs:553  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2020-06-01 (refusal)", day(2020, 6, 1)),
GAP LINE   crates/costs/tests/fa73061.rs:554  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2024-10-01 (FA64232)", day(2024, 10, 1)),
GAP LINE   crates/costs/tests/fa73061.rs:555  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2026-02-27 (FA64232)", day(2026, 2, 27)),
GAP LINE   crates/costs/tests/fa73061.rs:556  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2026-03-01 (FA73061)", day(2026, 3, 1)),
GAP LINE   crates/costs/tests/fa73061.rs:557  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2100-12-31 (FA73061)", TradeDay::MAX),
GAP LINE   crates/costs/tests/fa73061.rs:558  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ];
GAP LINE   crates/costs/tests/fa73061.rs:559  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for (label, on) in days {
GAP LINE   crates/costs/tests/fa73061.rs:560  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let exchange = exchange_charge_rate(Exchange::Nse, on);
GAP LINE   crates/costs/tests/fa73061.rs:561  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let ipft = ipft_rate(Exchange::Nse, on);
GAP LINE   crates/costs/tests/fa73061.rs:562  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
GAP LINE   crates/costs/tests/fa73061.rs:563  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  exchange_charge_rate(black_box(Exchange::Nse), black_box(on))
GAP LINE   crates/costs/tests/fa73061.rs:564  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });
GAP LINE   crates/costs/tests/fa73061.rs:565  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/tests/fa73061.rs:568  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);
GAP LINE   crates/costs/tests/fa73061.rs:569  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
GAP LINE   crates/costs/tests/fa73061.rs:570  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ipft_rate(black_box(Exchange::Nse), black_box(on))
GAP LINE   crates/costs/tests/fa73061.rs:571  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });
GAP LINE   crates/costs/tests/fa73061.rs:572  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/tests/fa73061.rs:575  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);
GAP LINE   crates/costs/tests/fa73061.rs:576  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
GAP LINE   crates/costs/tests/fa73061.rs:577  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  Rates::resolve(
GAP LINE   crates/costs/tests/fa73061.rs:578  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Broker::Groww),
GAP LINE   crates/costs/tests/fa73061.rs:579  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Exchange::Nse),
GAP LINE   crates/costs/tests/fa73061.rs:580  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(on),
GAP LINE   crates/costs/tests/fa73061.rs:582  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });
GAP LINE   crates/costs/tests/fa73061.rs:583  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/tests/fa73061.rs:586  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);
GAP LINE   crates/costs/tests/fa73061.rs:589  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated);
GAP LINE   crates/costs/tests/fa73061.rs:590  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
GAP LINE   crates/costs/tests/fa73061.rs:591  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  RateCard::resolve(
GAP LINE   crates/costs/tests/fa73061.rs:592  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(TradeKind::Options),
GAP LINE   crates/costs/tests/fa73061.rs:593  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Exchange::Nse),
GAP LINE   crates/costs/tests/fa73061.rs:594  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(on),
GAP LINE   crates/costs/tests/fa73061.rs:595  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(RateSelector::Dated),
GAP LINE   crates/costs/tests/fa73061.rs:597  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });
GAP LINE   crates/costs/tests/fa73061.rs:598  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/tests/fa73061.rs:601  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);
GAP LINE   crates/costs/tests/fa73061.rs:602  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(
GAP LINE   crates/costs/tests/fa73061.rs:603  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),
GAP LINE   crates/costs/tests/fa73061.rs:607  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(exchange_charge_rate(Exchange::Nse, on), exchange);
GAP LINE   crates/costs/tests/fa73061.rs:608  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(ipft_rate(Exchange::Nse, on), ipft);
GAP LINE   crates/costs/tests/fa73061.rs:610  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let on = day(2026, 3, 2);
GAP LINE   crates/costs/tests/fa73061.rs:611  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let trip = nifty_trip(on, on, 10_000, 12_000);
GAP LINE   crates/costs/tests/fa73061.rs:612  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || price(black_box(&trip)));
GAP LINE   crates/costs/tests/fa73061.rs:613  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(
GAP LINE   crates/costs/tests/fa73061.rs:616  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);
GAP LINE   crates/costs/tests/fa73061.rs:617  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(price(&trip).expect("verified").exchange().raw(), 510);
GAP LINE   crates/costs/tests/fa73061.rs:618  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }
GAP LINE   crates/engine/src/lib.rs:4526  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  let code = src
GAP LINE   crates/engine/src/lib.rs:4527  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  .lines()
GAP LINE   crates/engine/src/lib.rs:4528  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  .map(|l| l.split_once("//").map_or(l, |(before, _)| before))
GAP LINE   crates/engine/src/lib.rs:4529  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  .collect::<Vec<_>>()
GAP LINE   crates/engine/src/lib.rs:4530  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  .join("\n");
GAP LINE   crates/engine/src/lib.rs:4531  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  assert!(
GAP LINE   crates/engine/src/lib.rs:4532  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  code.contains(concat!(
GAP LINE   crates/engine/src/lib.rs:4533  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  "while !current.",
GAP LINE   crates/engine/src/lib.rs:4534  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  "frequent.is_empty() && progress.halted.is_none() {"
GAP LINE   crates/engine/src/lib.rs:4535  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  )),
GAP LINE   crates/engine/src/lib.rs:4540  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  assert!(
GAP LINE   crates/engine/src/lib.rs:4541  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  code.contains(concat!("for (_, b) in block.", "iter().skip(")),
GAP LINE   crates/engine/src/lib.rs:4545  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  assert!(
GAP LINE   crates/engine/src/lib.rs:4546  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  code.contains(concat!("for block in keyed.", "chunk_by(")),
GAP LINE   crates/engine/src/lib.rs:4552  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  }
GAP LINE   crates/engine/src/lib.rs:4725  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  assert!(
GAP LINE   crates/engine/src/lib.rs:4726  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  shipped.contains(concat!("Column::", "try_from_rows(bar_bits)")),
GAP LINE   crates/engine/src/lib.rs:4729  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  }
GAP LINE   crates/engine/src/lib.rs:5290  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  continue;
GAP LINE   crates/engine/src/lib.rs:5907  [LSA judgment (engine lib.rs test code, not mine to edit): the observer's rejection arm runs only if the walk calls it after its terminal boundary (a production defect); it is the test failing expressed as an Err; suggested: (F) by extension, or assert!(!terminal_seen, ..)]  return Err("the walk advanced beyond its terminal boundary".into());
GAP BRANCH crates/engine/src/lib.rs:3014:48 True  [LSA judgment (engine lib.rs test code, not mine to edit): the fixture prunes every candidate, so level.frequent is empty and the distinctness assertion checks nothing; suggested: assert the level is empty and rely on sweep_readiness_oracle.rs:159 for distinctness of real survivors, or widen the column so some survive]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
GAP BRANCH crates/engine/src/lib.rs:3014:48 False  [LSA judgment (engine lib.rs test code, not mine to edit): the fixture prunes every candidate, so level.frequent is empty and the distinctness assertion checks nothing; suggested: assert the level is empty and rely on sweep_readiness_oracle.rs:159 for distinctness of real survivors, or widen the column so some survive]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
GAP BRANCH crates/engine/src/lib.rs:4161:17 False  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  let [prev, next] = pair else { continue };
GAP BRANCH crates/engine/src/lib.rs:4164:17 False  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  let Ok(probe) = built else { continue };
GAP BRANCH crates/engine/src/lib.rs:4421:25 True  [LSA patch 1 (D-8566 draft): a line after a failing assertion of one of the two engine reds; covered once the split patch is applied (measured: gone)]  w == "break" || w == "return" || w == "continue"
GAP BRANCH crates/engine/src/lib.rs:5170:28 False  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  && set_positions(&cand).all(|x| prev.contains(&cand.without_bit(x)))
GAP BRANCH crates/engine/src/lib.rs:5289:17 False  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  let (Some(&(_, a0, p0)), Some(&(_, a1, p1))) = (pair.first(), pair.get(1)) else {
GAP BRANCH crates/engine/src/lib.rs:5609:24 False  [LSA patch 2 (in the combined patch): measured gone with it applied (windows(2) pair and expect; the oracle fixture extended; every handed batch asserted non-empty)]  if pending > 0 {
GAP BRANCH crates/engine/src/lib.rs:5906:24 True  [LSA judgment (engine lib.rs test code, not mine to edit): the observer's rejection arm runs only if the walk calls it after its terminal boundary (a production defect); it is the test failing expressed as an Err; suggested: (F) by extension, or assert!(!terminal_seen, ..)]  if terminal_seen {
GAP LINE   crates/telemetry/src/sink.rs:10406  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn an_audit_line_survives_2_gib_of_refusals_at_full_size() {
GAP LINE   crates/telemetry/src/sink.rs:10407  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  survives(
GAP LINE   crates/telemetry/src/sink.rs:10408  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  "survives-full",
GAP LINE   crates/telemetry/src/sink.rs:10409  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  2 * 1024 * 1024 * 1024,
GAP LINE   crates/telemetry/src/sink.rs:10412  [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }
GAP LINE   crates/telemetry/tests/global_absent.rs:106  [BY DESIGN, for the Mac/operator: an argument of emit_if! that runs only when a sink admits; this binary installs none and the test asserts the argument never runs (lines 113-117), so covering it is the test failing. (F) by extension of recipe step 4, or an accepted GAP: not mine to decide]  "did not land",
GAP LINE   crates/telemetry/tests/global_absent.rs:107  [BY DESIGN, for the Mac/operator: as line 106]  "bars" => telemetry::Value::Uint(count()),
`````

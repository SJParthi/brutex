# base-cov-libs-gaps-a1-after.txt.md

Source: `state/fix/round2/base-cov-libs-gaps-a1-after.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
BASE-COV-LIBS attempt 1, AFTER: Route B on tip 5ad5546018350f8be5993dddfc722e94447ef062 for the seven touched crates (core costs engine indicators lake store telemetry); greeks, runner and vocab are untouched and their figures are base-cov-libs-gaps-a1.txt's. Log scratch/deep-BASE-COV-LIBS-logs/5ad554601835-routeb-after.log, profile scratch/deep-BASE-COV-LIBS-prof/5ad554601835-1-B, 21:08-21:50 IST.

TOTALS per crate (lines covered/counted, uncovered; branches covered/counted, uncovered; files read)
  core        lines 7333/7389 (uncovered 56)  branches 581/656 (uncovered 75)  files 20
  costs       lines 11298/11473 (uncovered 175)  branches 476/498 (uncovered 22)  files 24
  engine      lines 5181/5221 (uncovered 40)  branches 516/572 (uncovered 56)  files 9
  indicators  lines 16498/16540 (uncovered 42)  branches 1915/2186 (uncovered 271)  files 32
  lake        lines 4953/5590 (uncovered 637)  branches 273/366 (uncovered 93)  files 15
  store       lines 48403/50223 (uncovered 1820)  branches 3201/3874 (uncovered 673)  files 91
  telemetry   lines 9071/9122 (uncovered 51)  branches 628/680 (uncovered 52)  files 15
  ALL         lines 102737/105558 (uncovered 2821)  branches 7590/8832 (uncovered 1242)  files 206

NOT READ by Route B (15): crates/core/benches/ratio.rs crates/core/src/lib.rs crates/costs/benches/ratio.rs crates/costs/benches/stack.rs crates/costs/src/lib.rs crates/engine/benches/ratio.rs crates/indicators/benches/ratio.rs crates/lake/benches/ratio.rs crates/lake/src/lib.rs crates/store/benches/ratio.rs crates/store/benches/ratio/months.rs crates/store/benches/ratio/quarantine.rs crates/store/benches/ratio/safe_files.rs crates/store/src/lib.rs crates/telemetry/benches/ratio.rs

PER FILE (lines a/b, branches c/d)
  crates/core/src/alias.rs lines 857/859 branches 49/56  GAPS
  crates/core/src/blake3.rs lines 395/395 branches 16/16
  crates/core/src/error.rs lines 92/92 branches 4/4
  crates/core/src/instrument.rs lines 849/849 branches 50/54  GAPS
  crates/core/src/isin.rs lines 140/140 branches 14/14
  crates/core/src/knob.rs lines 69/69 branches 8/8
  crates/core/src/price.rs lines 320/320 branches 28/28
  crates/core/src/secret_shape.rs lines 223/223 branches 32/34  GAPS
  crates/core/src/symbol.rs lines 109/109 branches 4/4
  crates/core/src/universe.rs lines 966/968 branches 61/82  GAPS
  crates/core/src/vendor.rs lines 1578/1579 branches 83/84  GAPS
  crates/core/tests/banned_lists.rs lines 85/85 branches 2/4  GAPS
  crates/core/tests/bound_probe.rs lines 35/35 branches 0/0
  crates/core/tests/claim_reader.rs lines 72/72 branches 2/4  GAPS
  crates/core/tests/cost_invariants.rs lines 165/177 branches 26/34  GAPS
  crates/core/tests/decision_citations.rs lines 384/392 branches 68/72  GAPS
  crates/core/tests/findings.rs lines 361/383 branches 21/36  GAPS
  crates/core/tests/graph.rs lines 469/469 branches 96/102  GAPS
  crates/core/tests/lint.rs lines 76/84 branches 12/14  GAPS
  crates/core/tests/zerodha_index.rs lines 88/89 branches 5/6  GAPS
  crates/costs/src/charge_error.rs lines 184/184 branches 0/0
  crates/costs/src/dated.rs lines 386/386 branches 32/32
  crates/costs/src/day.rs lines 442/442 branches 28/30  GAPS
  crates/costs/src/dhan_calc.rs lines 467/467 branches 30/30
  crates/costs/src/error.rs lines 283/283 branches 4/4
  crates/costs/src/exact.rs lines 305/305 branches 14/14
  crates/costs/src/expiry.rs lines 622/622 branches 28/28
  crates/costs/src/fill.rs lines 443/447 branches 16/16  GAPS
  crates/costs/src/ledger.rs lines 872/879 branches 24/24  GAPS
  crates/costs/src/lot.rs lines 667/670 branches 74/78  GAPS
  crates/costs/src/lot_table_tests.rs lines 293/335 branches 10/14  GAPS
  crates/costs/src/money.rs lines 153/153 branches 4/6  GAPS
  crates/costs/src/moneyness.rs lines 355/355 branches 18/18
  crates/costs/src/rate.rs lines 117/117 branches 0/0
  crates/costs/src/regime.rs lines 534/534 branches 26/26
  crates/costs/src/schedule.rs lines 729/734 branches 12/14  GAPS
  crates/costs/src/scope.rs lines 82/83 branches 0/0  GAPS
  crates/costs/src/stack.rs lines 1609/1612 branches 84/86  GAPS
  crates/costs/src/strike.rs lines 317/317 branches 10/10
  crates/costs/src/trip.rs lines 1558/1558 branches 32/32
  crates/costs/src/venue.rs lines 77/77 branches 2/2
  crates/costs/tests/adversarial.rs lines 139/149 branches 10/16  GAPS
  crates/costs/tests/dhan_calculator.rs lines 382/406 branches 12/12  GAPS
  crates/costs/tests/fa73061.rs lines 282/358 branches 6/6  GAPS
  crates/engine/src/column.rs lines 409/409 branches 17/18  GAPS
  crates/engine/src/keep.rs lines 417/417 branches 24/30  GAPS
  crates/engine/src/lib.rs lines 2199/2229 branches 120/154  GAPS
  crates/engine/src/manifest.rs lines 285/285 branches 112/112
  crates/engine/src/resume.rs lines 546/546 branches 132/132
  crates/engine/tests/adversarial.rs lines 150/150 branches 6/8  GAPS
  crates/engine/tests/production_callers.rs lines 172/175 branches 28/34  GAPS
  crates/engine/tests/resume_readiness.rs lines 817/824 branches 48/54  GAPS
  crates/engine/tests/sweep_readiness_oracle.rs lines 186/186 branches 29/30  GAPS
  crates/indicators/src/anchored.rs lines 1638/1641 branches 104/114  GAPS
  crates/indicators/src/column.rs lines 1258/1260 branches 75/96  GAPS
  crates/indicators/src/daily.rs lines 695/695 branches 61/86  GAPS
  crates/indicators/src/evaluator.rs lines 1707/1712 branches 145/166  GAPS
  crates/indicators/src/fib.rs lines 407/407 branches 24/26  GAPS
  crates/indicators/src/gap.rs lines 723/724 branches 53/58  GAPS
  crates/indicators/src/lib.rs lines 775/777 branches 57/60  GAPS
  crates/indicators/src/orb.rs lines 469/470 branches 49/58  GAPS
  crates/indicators/src/pattern.rs lines 1399/1399 branches 587/594  GAPS
  crates/indicators/src/session.rs lines 698/698 branches 151/194  GAPS
  crates/indicators/src/trend.rs lines 1261/1263 branches 157/174  GAPS
  crates/indicators/src/vwap.rs lines 941/943 branches 85/98  GAPS
  crates/indicators/tests/audit_readiness.rs lines 49/49 branches 0/0
  crates/indicators/tests/crossing_known_readiness.rs lines 177/177 branches 38/40  GAPS
  crates/indicators/tests/current_day_fib_known_readiness.rs lines 206/207 branches 23/28  GAPS
  crates/indicators/tests/evaluator_position_count.rs lines 26/26 branches 0/0
  crates/indicators/tests/exact_minute_orb_gap_readiness.rs lines 339/340 branches 22/30  GAPS
  crates/indicators/tests/extremes.rs lines 462/462 branches 14/14
  crates/indicators/tests/fib_known_readiness.rs lines 297/299 branches 23/30  GAPS
  crates/indicators/tests/fib_rung_rounding.rs lines 178/178 branches 17/18  GAPS
  crates/indicators/tests/gap_known_readiness.rs lines 337/338 branches 23/30  GAPS
  crates/indicators/tests/invariants.rs lines 213/213 branches 0/0
  crates/indicators/tests/module_doc_counts.rs lines 169/182 branches 18/28  GAPS
  crates/indicators/tests/orb_known_readiness.rs lines 261/262 branches 23/28  GAPS
  crates/indicators/tests/session_from_outside.rs lines 186/186 branches 2/4  GAPS
  crates/indicators/tests/session_known_readiness.rs lines 393/394 branches 34/48  GAPS
  crates/indicators/tests/shared_core_doc.rs lines 184/184 branches 2/2
  crates/indicators/tests/sweep_predicate_readiness.rs lines 292/292 branches 72/96  GAPS
  crates/indicators/tests/time_known_readiness.rs lines 145/145 branches 6/6
  crates/indicators/tests/trend_known_readiness.rs lines 276/279 branches 29/38  GAPS
  crates/indicators/tests/withheld_cut_fold.rs lines 133/133 branches 8/8
  crates/indicators/tests/withheld_fold.rs lines 204/205 branches 13/14  GAPS
  crates/lake/src/bar.rs lines 96/98 branches 2/2  GAPS
  crates/lake/src/batch.rs lines 192/192 branches 12/12
  crates/lake/src/contract.rs lines 349/356 branches 25/26  GAPS
  crates/lake/src/error.rs lines 383/383 branches 2/2
  crates/lake/src/footer.rs lines 190/190 branches 36/36
  crates/lake/src/footer_tests.rs lines 150/154 branches 5/6  GAPS
  crates/lake/src/page.rs lines 572/588 branches 33/44  GAPS
  crates/lake/src/reader.rs lines 936/993 branches 39/40  GAPS
  crates/lake/src/schema.rs lines 201/206 branches 29/30  GAPS
  crates/lake/src/zstd_v082_frames.rs lines 67/71 branches 7/8  GAPS
  crates/lake/tests/events.rs lines 194/197 branches 18/18  GAPS
  crates/lake/tests/real_lake.rs lines 0/132 branches 0/18  GAPS
  crates/lake/tests/real_lake_regression.rs lines 0/126 branches 0/16  GAPS
  crates/lake/tests/refusals.rs lines 768/1025 branches 34/72  GAPS
  crates/lake/tests/synthetic.rs lines 855/879 branches 31/36  GAPS
  crates/store/src/block.rs lines 115/115 branches 10/10
  crates/store/src/catalog.rs lines 192/199 branches 49/52  GAPS
  crates/store/src/catalog_tests.rs lines 338/341 branches 9/18  GAPS
  crates/store/src/checksum_audit.rs lines 235/235 branches 27/30  GAPS
  crates/store/src/checksum_audit_tests.rs lines 400/403 branches 11/12  GAPS
  crates/store/src/crc.rs lines 90/90 branches 8/8
  crates/store/src/emits.rs lines 300/306 branches 3/4  GAPS
  crates/store/src/file.rs lines 4532/4638 branches 364/386  GAPS
  crates/store/src/file_fault_tests.rs lines 571/575 branches 25/40  GAPS
  crates/store/src/file_sealed.rs lines 255/271 branches 42/58  GAPS
  crates/store/src/file_sealed_tests.rs lines 584/617 branches 25/36  GAPS
  crates/store/src/file_write_door_tests.rs lines 726/744 branches 24/30  GAPS
  crates/store/src/flock.rs lines 439/454 branches 40/48  GAPS
  crates/store/src/flock_patient_tests.rs lines 495/506 branches 23/36  GAPS
  crates/store/src/flock_tests.rs lines 563/570 branches 18/26  GAPS
  crates/store/src/format.rs lines 272/278 branches 42/42  GAPS
  crates/store/src/generation.rs lines 298/320 branches 51/62  GAPS
  crates/store/src/generation_tests.rs lines 37/37 branches 0/0
  crates/store/src/header.rs lines 524/524 branches 76/76
  crates/store/src/header_audit_tests.rs lines 49/49 branches 0/0
  crates/store/src/layout.rs lines 186/186 branches 40/40
  crates/store/src/month_summary.rs lines 654/683 branches 172/194  GAPS
  crates/store/src/month_summary_tests.rs lines 559/562 branches 32/52  GAPS
  crates/store/src/month_summary_writer.rs lines 146/165 branches 25/34  GAPS
  crates/store/src/open_flags.rs lines 6/6 branches 0/0
  crates/store/src/path.rs lines 206/206 branches 30/30
  crates/store/src/provenance.rs lines 742/765 branches 144/152  GAPS
  crates/store/src/quarantine.rs lines 1385/1501 branches 155/204  GAPS
  crates/store/src/quarantine_tests.rs lines 1514/1523 branches 50/82  GAPS
  crates/store/src/repair.rs lines 700/741 branches 117/132  GAPS
  crates/store/src/repair_tests.rs lines 639/643 branches 15/18  GAPS
  crates/store/src/safe_open.rs lines 169/177 branches 20/22  GAPS
  crates/store/src/safe_open_tests.rs lines 285/298 branches 9/18  GAPS
  crates/store/src/sealed.rs lines 1434/1434 branches 270/274  GAPS
  crates/store/src/sealed_codec.rs lines 646/646 branches 90/90
  crates/store/src/sealed_codec_tests.rs lines 996/1000 branches 30/34  GAPS
  crates/store/src/sealed_tests.rs lines 1719/1740 branches 34/40  GAPS
  crates/store/src/settings.rs lines 700/743 branches 134/150  GAPS
  crates/store/src/settings_tests.rs lines 820/832 branches 17/28  GAPS
  crates/store/src/sidecar_checksum_tests.rs lines 391/391 branches 10/10
  crates/store/src/small_month_tests.rs lines 106/106 branches 1/2  GAPS
  crates/store/src/sort_index.rs lines 724/795 branches 107/130  GAPS
  crates/store/src/sort_index_tests.rs lines 385/385 branches 9/10  GAPS
  crates/store/src/sort_index_writer.rs lines 388/388 branches 62/62
  crates/store/src/sort_index_writer_tests.rs lines 114/114 branches 0/0
  crates/store/src/staging.rs lines 109/109 branches 14/14
  crates/store/src/staging_tests.rs lines 222/223 branches 8/16  GAPS
  crates/store/src/time_index.rs lines 744/744 branches 71/80  GAPS
  crates/store/src/verdict.rs lines 2679/2685 branches 296/310  GAPS
  crates/store/src/verdict_tests.rs lines 594/601 branches 24/26  GAPS
  crates/store/tests/admission.rs lines 182/182 branches 0/0
  crates/store/tests/bisect_cost.rs lines 80/80 branches 0/0
  crates/store/tests/catalog.rs lines 557/568 branches 7/14  GAPS
  crates/store/tests/cited_commits.rs lines 508/532 branches 72/90  GAPS
  crates/store/tests/cloud_mutants_store.rs lines 2190/2203 branches 61/96  GAPS
  crates/store/tests/cold_read.rs lines 199/199 branches 4/4
  crates/store/tests/docs.rs lines 229/242 branches 5/14  GAPS
  crates/store/tests/durability.rs lines 194/195 branches 0/0  GAPS
  crates/store/tests/fault.rs lines 471/472 branches 10/12  GAPS
  crates/store/tests/fifo.rs lines 194/201 branches 7/12  GAPS
  crates/store/tests/generation.rs lines 483/492 branches 8/18  GAPS
  crates/store/tests/geometry.rs lines 433/433 branches 9/12  GAPS
  crates/store/tests/libm_key.rs lines 778/780 branches 8/12  GAPS
  crates/store/tests/misfiled.rs lines 205/207 branches 9/14  GAPS
  crates/store/tests/month_repair.rs lines 605/613 branches 13/18  GAPS
  crates/store/tests/month_summary_log.rs lines 94/96 branches 0/0  GAPS
  crates/store/tests/open_flags.rs lines 97/98 branches 17/18  GAPS
  crates/store/tests/provenance.rs lines 1222/1238 branches 0/0  GAPS
  crates/store/tests/repair.rs lines 384/384 branches 4/6  GAPS
  crates/store/tests/roundtrip.rs lines 111/111 branches 0/0
  crates/store/tests/seal_buffer.rs lines 100/100 branches 0/0
  crates/store/tests/sealed_codec_alloc.rs lines 230/232 branches 15/28  GAPS
  crates/store/tests/sealed_codec_samples.rs lines 0/366 branches 0/44  GAPS
  crates/store/tests/sealed_door.rs lines 126/211 branches 6/20  GAPS
  crates/store/tests/sealed_samples.rs lines 0/174 branches 0/12  GAPS
  crates/store/tests/sealed_write_door.rs lines 0/165 branches 0/24  GAPS
  crates/store/tests/slot_audit.rs lines 348/354 branches 8/10  GAPS
  crates/store/tests/slot_audit_sealed.rs lines 88/96 branches 0/2  GAPS
  crates/store/tests/slot_damaged_log.rs lines 137/145 branches 0/0  GAPS
  crates/store/tests/sort_index.rs lines 599/602 branches 14/22  GAPS
  crates/store/tests/sort_index_log.rs lines 128/130 branches 0/0  GAPS
  crates/store/tests/support/mod.rs lines 32/32 branches 3/4  GAPS
  crates/store/tests/tail_proof.rs lines 175/175 branches 3/6  GAPS
  crates/store/tests/tix.rs lines 406/407 branches 10/12  GAPS
  crates/store/tests/tix_latency.rs lines 0/164 branches 0/14  GAPS
  crates/store/tests/tix_rebuilder.rs lines 336/336 branches 6/8  GAPS
  crates/store/tests/tix_small_month.rs lines 394/396 branches 6/8  GAPS
  crates/store/tests/tix_warn_once.rs lines 212/217 branches 0/0  GAPS
  crates/store/tests/unit.rs lines 1711/1721 branches 30/40  GAPS
  crates/store/tests/verdict.rs lines 1989/2007 branches 28/44  GAPS
  crates/store/tests/write.rs lines 1199/1203 branches 5/8  GAPS
  crates/telemetry/src/clock.rs lines 196/196 branches 16/18  GAPS
  crates/telemetry/src/encode.rs lines 206/206 branches 20/20
  crates/telemetry/src/event.rs lines 87/87 branches 0/0
  crates/telemetry/src/json.rs lines 341/341 branches 16/16
  crates/telemetry/src/level.rs lines 68/69 branches 0/0  GAPS
  crates/telemetry/src/lib.rs lines 203/203 branches 15/16  GAPS
  crates/telemetry/src/record.rs lines 417/418 branches 27/28  GAPS
  crates/telemetry/src/run_dirs.rs lines 79/79 branches 4/8  GAPS
  crates/telemetry/src/sink.rs lines 6061/6098 branches 438/476  GAPS
  crates/telemetry/src/state_events.rs lines 28/28 branches 12/12
  crates/telemetry/src/state_events_tests.rs lines 127/130 branches 6/10  GAPS
  crates/telemetry/src/tail.rs lines 862/865 branches 72/74  GAPS
  crates/telemetry/src/value.rs lines 142/142 branches 0/0
  crates/telemetry/tests/cloud_mutants_sink.rs lines 219/220 branches 2/2  GAPS
  crates/telemetry/tests/global_absent.rs lines 35/40 branches 0/0  GAPS

EVERY UNCOVERED LINE AND BRANCH SIDE (kind: GAP, or F? = a candidate (F) failing arm inside test code, confirmed only in base-cov-libs-fu.txt)
  F? panic! (failing arm)                    LINE   crates/core/src/alias.rs:1276: other => panic!("{asked}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/core/src/alias.rs:1320: other => panic!("{asked}: {other:?}"),
  GAP production                             BRANCH crates/core/src/alias.rs:692:27 False: .filter(|row| row.fno == Some(fno) && row.status.applies())
  GAP production                             BRANCH crates/core/src/alias.rs:701:27 False: .filter(|row| row.store_key == key && row.status.applies())
  GAP production                             BRANCH crates/core/src/alias.rs:811:27 False: .filter(|row| row.old == old && row.status.applies())
  GAP production                             BRANCH crates/core/src/alias.rs:820:27 False: .filter(|row| row.new == new && row.status.applies())
  F? assert! condition branch (failing side) BRANCH crates/core/src/alias.rs:1818:58 False: assert!(matches!(got.via, Via::Index(row) if row.store_key == key));
  F? assert! condition branch (failing side) BRANCH crates/core/src/alias.rs:1827:61 False: assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/alias.rs:1866:36 False: assert!(hit || !name.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/instrument.rs:779:17 False: assert!(a < b && b < c);
  F? assert! condition branch (failing side) BRANCH crates/core/src/instrument.rs:779:26 False: assert!(a < b && b < c);
  F? assert! condition branch (failing side) BRANCH crates/core/src/instrument.rs:1698:17 False: assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
  F? assert! condition branch (failing side) BRANCH crates/core/src/instrument.rs:1698:55 False: assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
  GAP production                             BRANCH crates/core/src/secret_shape.rs:127:48 False: if run >= TOKEN_RUN && (!mixed || (letter && digit)) {
  GAP production                             BRANCH crates/core/src/secret_shape.rs:155:76 True: .trim_end_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'));
  F? panic! (failing arm)                    LINE   crates/core/src/universe.rs:4837: panic!("{m} is a member, yet its probe reached an empty slot");
  F? assert! message argument                LINE   crates/core/src/universe.rs:5634: u.bits()
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4755:17 False: assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4755:38 False: assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4755:60 False: assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4767:17 False: assert!(tight.contains("X") && tight.contains("Y"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4767:40 False: assert!(tight.contains("X") && tight.contains("Y"));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4801:17 False: assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4801:42 False: assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4802:17 False: assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4802:48 False: assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4803:17 False: assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:4803:48 False: assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
  F? let-else in test                        BRANCH crates/core/src/universe.rs:4836:21 False: let Some(held) = idx.slots[at] else {
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5628:17 False: u.contains(Universe::NIFTY_50)
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5629:24 False: && u.contains(Universe::NIFTY_100)
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5630:24 False: && u.contains(Universe::NIFTY_200)
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5631:24 False: && u.contains(Universe::NIFTY_500)
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5632:24 False: && u.contains(Universe::TOTAL_MARKET),
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5640:17 False: u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5640:52 False: u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5792:17 False: assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
  F? assert! condition branch (failing side) BRANCH crates/core/src/universe.rs:5792:46 False: assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
  F? panic! (failing arm)                    LINE   crates/core/src/vendor.rs:3066: panic!("{m} is a member, yet its probe reached an empty slot");
  F? let-else in test                        BRANCH crates/core/src/vendor.rs:3065:25 False: let Some(held) = idx.slots[at] else {
  F? assert! condition branch (failing side) BRANCH crates/core/tests/banned_lists.rs:113:9 False: forbidden.len() > 20 && forbidden.contains("pyo3"),
  F? assert! condition branch (failing side) BRANCH crates/core/tests/banned_lists.rs:113:33 False: forbidden.len() > 20 && forbidden.contains("pyo3"),
  F? assert! condition branch (failing side) BRANCH crates/core/tests/claim_reader.rs:125:13 False: fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
  F? assert! condition branch (failing side) BRANCH crates/core/tests/claim_reader.rs:125:33 False: fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
  F? panic! (failing arm)                    LINE   crates/core/tests/cost_invariants.rs:88: panic!(
  F? panic! (failing arm)                    LINE   crates/core/tests/cost_invariants.rs:90: path.display()
  F? panic! (failing arm)                    LINE   crates/core/tests/cost_invariants.rs:104: panic!(
  F? panic! (failing arm)                    LINE   crates/core/tests/cost_invariants.rs:106: crates_dir.display()
  GAP test code                              LINE   crates/core/tests/cost_invariants.rs:114: continue;
  GAP test code                              LINE   crates/core/tests/cost_invariants.rs:117: continue;
  F? assert! message argument                LINE   crates/core/tests/cost_invariants.rs:128: crates_dir.display()
  GAP test code                              LINE   crates/core/tests/cost_invariants.rs:154: continue;
  GAP test code                              LINE   crates/core/tests/cost_invariants.rs:229: continue;
  F? assert! message argument                LINE   crates/core/tests/cost_invariants.rs:353: rows.len()
  F? assert! message argument                LINE   crates/core/tests/cost_invariants.rs:360: printed.len(),
  F? assert! message argument                LINE   crates/core/tests/cost_invariants.rs:361: benches.len()
  GAP test code                              BRANCH crates/core/tests/cost_invariants.rs:113:12 True: if !path.is_dir() {
  F? let-else in test                        BRANCH crates/core/tests/cost_invariants.rs:116:13 False: let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
  GAP test code                              BRANCH crates/core/tests/cost_invariants.rs:120:16 False: if let Ok(source) = std::fs::read_to_string(&bench) {
  F? let-else in test                        BRANCH crates/core/tests/cost_invariants.rs:153:13 False: let Ok(files) = std::fs::read_dir(entry.path().join("benches")) else {
  GAP test code                              BRANCH crates/core/tests/cost_invariants.rs:158:35 True: if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
  GAP test code                              BRANCH crates/core/tests/cost_invariants.rs:195:28 False: b'a'..=b'z' if saw_digit => {}
  F? let-else in test                        BRANCH crates/core/tests/cost_invariants.rs:228:13 False: let Some(id) = rest.split(" |").next().map(str::trim) else {
  GAP test code                              BRANCH crates/core/tests/cost_invariants.rs:279:16 False: if cites.iter().any(|cited| reached.contains(cited)) {
  F? assert! message argument                LINE   crates/core/tests/decision_citations.rs:243: path.display()
  F? assert! message argument                LINE   crates/core/tests/decision_citations.rs:303: cited.len()
  GAP test code                              LINE   crates/core/tests/decision_citations.rs:308: let _ = writeln!(said, "  {id}  first cited at {path}:{line}");
  GAP test code                              LINE   crates/core/tests/decision_citations.rs:309: }
  F? assert! message argument                LINE   crates/core/tests/decision_citations.rs:316: missing.len()
  F? assert! message argument                LINE   crates/core/tests/decision_citations.rs:412: citations(&none[0].1)
  F? panic! (failing arm)                    LINE   crates/core/tests/decision_citations.rs:536: panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
  F? panic! (failing arm)                    LINE   crates/core/tests/decision_citations.rs:539: || panic!("{file} is not under a crate's src/"),
  GAP test code                              BRANCH crates/core/tests/decision_citations.rs:246:16 False: if name != "target" && !name.starts_with('.') {
  GAP test code                              BRANCH crates/core/tests/decision_citations.rs:246:36 False: if name != "target" && !name.starts_with('.') {
  GAP test code                              BRANCH crates/core/tests/decision_citations.rs:562:19 False: while let Some(&b) = bytes.get(end) {
  GAP test code                              BRANCH crates/core/tests/decision_citations.rs:564:20 False: && end > start
  F? assert! message argument                LINE   crates/core/tests/findings.rs:93: cells.len()
  F? assert! message argument                LINE   crates/core/tests/findings.rs:295: rows.len()
  F? assert_eq! message argument             LINE   crates/core/tests/findings.rs:335: counted + rest,
  F? assert_eq! message argument             LINE   crates/core/tests/findings.rs:336: rows.len()
  F? assert_eq! message argument             LINE   crates/core/tests/findings.rs:361: rows.len()
  GAP test code                              LINE   crates/core/tests/findings.rs:447: let reason = row.disposition.trim_start_matches("REFUTED").trim();
  GAP test code                              LINE   crates/core/tests/findings.rs:448: assert!(
  F? assert! message argument                LINE   crates/core/tests/findings.rs:449: reason.len() >= 20,
  GAP test code                              LINE   crates/core/tests/findings.rs:472: println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
  GAP test code                              LINE   crates/core/tests/findings.rs:473: return false;
  GAP test code                              LINE   crates/core/tests/findings.rs:588: return;
  GAP test code                              LINE   crates/core/tests/findings.rs:605: continue; // the sha-presence test owns this case
  GAP test code                              LINE   crates/core/tests/findings.rs:614: println!(
  GAP test code                              LINE   crates/core/tests/findings.rs:618: continue;
  GAP test code                              LINE   crates/core/tests/findings.rs:687: return;
  GAP test code                              LINE   crates/core/tests/findings.rs:704: continue; // the sha-presence test owns this case
  GAP test code                              LINE   crates/core/tests/findings.rs:714: println!(
  GAP test code                              LINE   crates/core/tests/findings.rs:718: continue;
  GAP test code                              LINE   crates/core/tests/findings.rs:769: return;
  GAP test code                              LINE   crates/core/tests/findings.rs:792: continue; // the sha-presence test owns this case
  GAP test code                              LINE   crates/core/tests/findings.rs:802: println!(
  GAP test code                              LINE   crates/core/tests/findings.rs:806: continue;
  GAP test code                              BRANCH crates/core/tests/findings.rs:355:44 False: .filter(|l| l.starts_with("| ") && !l.starts_with("|---") && !l.starts_with("| Finding"))
  F? assert! condition branch (failing side) BRANCH crates/core/tests/findings.rs:366:13 False: cells.len() >= 3 && cells[2].len() > 20,
  F? assert! condition branch (failing side) BRANCH crates/core/tests/findings.rs:366:33 False: cells.len() >= 3 && cells[2].len() > 20,
  GAP test code                              BRANCH crates/core/tests/findings.rs:471:12 True: if let Err(why) = std::process::Command::new("git").arg("--version").output() {
  GAP test code                              BRANCH crates/core/tests/findings.rs:477:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound
  GAP test code                              BRANCH crates/core/tests/findings.rs:489:12 False: if let Some(above) = root.parent() {
  GAP test code                              BRANCH crates/core/tests/findings.rs:587:8 True: if !history_is_at(&repo) {
  GAP test code                              BRANCH crates/core/tests/findings.rs:604:12 True: if sha.len() < 7 {
  F? let-else in test                        BRANCH crates/core/tests/findings.rs:613:13 False: let Ok(out) = out else {
  GAP test code                              BRANCH crates/core/tests/findings.rs:686:8 True: if !history_is_at(&repo) {
  GAP test code                              BRANCH crates/core/tests/findings.rs:703:12 True: if sha.len() < 7 {
  F? let-else in test                        BRANCH crates/core/tests/findings.rs:713:13 False: let Ok(out) = out else {
  GAP test code                              BRANCH crates/core/tests/findings.rs:768:8 True: if !history_is_at(&repo) {
  GAP test code                              BRANCH crates/core/tests/findings.rs:791:12 True: if sha.len() < 7 {
  F? let-else in test                        BRANCH crates/core/tests/findings.rs:801:13 False: let Ok(out) = out else {
  GAP test code                              BRANCH crates/core/tests/graph.rs:220:16 False: if target == "target" && DEPENDENCY_TABLES.contains(&table.as_str()) =>
  GAP test code                              BRANCH crates/core/tests/graph.rs:220:38 False: if target == "target" && DEPENDENCY_TABLES.contains(&table.as_str()) =>
  GAP test code                              BRANCH crates/core/tests/graph.rs:300:37 True: None if c == '[' || c == '{' => depth += 1,
  GAP test code                              BRANCH crates/core/tests/graph.rs:301:37 True: None if c == ']' || c == '}' => depth = depth.saturating_sub(1),
  F? assert! condition branch (failing side) BRANCH crates/core/tests/graph.rs:528:33 False: !deps.is_empty() || claim.to_lowercase().contains("nothing"),
  F? assert! condition branch (failing side) BRANCH crates/core/tests/graph.rs:563:44 False: !name.contains('*') && !name.contains('/') && !name.is_empty(),
  F? assert! message argument                LINE   crates/core/tests/lint.rs:139: &next_fn[..next_fn.find('(').unwrap_or(0)],
  GAP test code                              LINE   crates/core/tests/lint.rs:199: }
  GAP test code                              LINE   crates/core/tests/lint.rs:200: let reaches = lines[i..lines.len().min(i + 3)]
  GAP test code                              LINE   crates/core/tests/lint.rs:201: .iter()
  GAP test code                              LINE   crates/core/tests/lint.rs:202: .any(|l| l.contains(THE_BOUNDARY));
  GAP test code                              LINE   crates/core/tests/lint.rs:203: assert!(
  F? assert! message argument                LINE   crates/core/tests/lint.rs:204: reaches,
  F? assert! message argument                LINE   crates/core/tests/lint.rs:209: i + 1,
  GAP test code                              BRANCH crates/core/tests/lint.rs:197:16 False: if !line.contains("f64") && !line.contains("f32") {
  GAP test code                              BRANCH crates/core/tests/lint.rs:197:41 False: if !line.contains("f64") && !line.contains("f32") {
  F? panic! (failing arm)                    LINE   crates/core/tests/zerodha_index.rs:60: panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
  F? let-else in test                        BRANCH crates/core/tests/zerodha_index.rs:59:13 False: let Decoded::Keep(listing) = decoded else {
  F? assert! condition branch (failing side) BRANCH crates/costs/src/day.rs:897:25 False: assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
  F? assert! condition branch (failing side) BRANCH crates/costs/src/day.rs:897:57 False: assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
  F? assert! message argument                LINE   crates/costs/src/fill.rs:1010: best.buy().raw(),
  F? assert! message argument                LINE   crates/costs/src/fill.rs:1011: worst.buy().raw()
  F? assert! message argument                LINE   crates/costs/src/fill.rs:1017: best.sell().raw(),
  F? assert! message argument                LINE   crates/costs/src/fill.rs:1018: worst.sell().raw()
  GAP production                             LINE   crates/costs/src/ledger.rs:771: const _: fn() = || {
  GAP production                             LINE   crates/costs/src/ledger.rs:772: let _ = <DayLedger as NoDefault<_>>::witness;
  GAP production                             LINE   crates/costs/src/ledger.rs:773: let _ = <RateSelector as NoDefault<_>>::witness;
  GAP production                             LINE   crates/costs/src/ledger.rs:774: };
  F? assert! message argument                LINE   crates/costs/src/ledger.rs:1340: LINE_NAMES[at],
  F? assert! message argument                LINE   crates/costs/src/ledger.rs:1341: summed[at],
  F? assert! message argument                LINE   crates/costs/src/ledger.rs:1342: closed[at]
  F? panic! (failing arm)                    LINE   crates/costs/src/lot.rs:1498: panic!("{subject} {on:?}: {:?}", got.basis);
  F? panic! (failing arm)                    LINE   crates/costs/src/lot.rs:1590: panic!("{subject} {on:?}: an unread revision answered a lot");
  F? assert! message argument                LINE   crates/costs/src/lot.rs:1614: matches!(
  F? let-else in test                        BRANCH crates/costs/src/lot.rs:1497:21 False: let LotBasis::ContractRule(citation) = got.basis else {
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot.rs:1554:25 False: assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot.rs:1554:57 False: assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
  F? let-else in test                        BRANCH crates/costs/src/lot.rs:1589:17 False: let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
  F? panic! (failing arm)                    LINE   crates/costs/src/lot_table_tests.rs:146: other => panic!("expected the unverified row, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/costs/src/lot_table_tests.rs:156: other => panic!("expected before-first-row, got {other:?}"),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:323: fn time_lookups(table: &LotTable, names: &[String], n: usize) -> (u128, u128, u128) {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:324: let days = [
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:325: day(2019, 3, 4),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:326: day(2022, 8, 1),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:327: day(2025, 5, 5),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:328: day(2026, 9, 30),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:329: ];
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:330: let mut samples = Vec::with_capacity(n);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:331: for at in 0..n {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:332: let name = &names[at % names.len()];
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:333: let when = days[at % days.len()];
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:334: let started = std::time::Instant::now();
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:335: let answer = table.lot_on(name, when);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:336: samples.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:337: std::hint::black_box(answer.is_ok());
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:338: }
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:339: samples.sort_unstable();
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:340: let pick = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:341: (pick(50), pick(99), *samples.last().unwrap())
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:342: }
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:346: fn a_lookup_is_flat_in_the_table() {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:347: fn table_of(underlyings: usize) -> (LotTable, Vec<String>) {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:348: let names: Vec<String> = (0..underlyings).map(|at| format!("U{at:05}")).collect();
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:349: let mut rows = Vec::new();
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:350: for name in &names {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:351: for step in 0..MAX_LOT_ROWS_PER_UNDERLYING {
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:352: let ordinal = day(2018, 1, 1).ordinal() + i32::try_from(step * 45).unwrap();
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:353: rows.push(sourced(name, TradeDay::from_ordinal(ordinal).unwrap(), 25));
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:354: }
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:356: (LotTable::from_rows(rows).unwrap(), names)
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:357: }
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:358: let (small, small_names) = table_of(2);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:359: let (large, large_names) = table_of(1_500);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:360: let n = 100_000;
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:361: let (s50, s99, smax) = time_lookups(&small, &small_names, n);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:362: let (l50, l99, lmax) = time_lookups(&large, &large_names, n);
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:363: println!(
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:367: assert!(
  F? assert! message argument                LINE   crates/costs/src/lot_table_tests.rs:368: l99 <= s99.saturating_mul(10).max(2_000),
  GAP test code                              LINE   crates/costs/src/lot_table_tests.rs:371: }
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot_table_tests.rs:112:52 False: assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot_table_tests.rs:117:25 False: assert!(row.source_sha256.is_none() && row.read.is_none());
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot_table_tests.rs:117:56 False: assert!(row.source_sha256.is_none() && row.read.is_none());
  F? assert! condition branch (failing side) BRANCH crates/costs/src/lot_table_tests.rs:187:56 False: Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
  F? assert! condition branch (failing side) BRANCH crates/costs/src/money.rs:383:17 False: assert!(widest > 0 && widest < i128::MAX);
  F? assert! condition branch (failing side) BRANCH crates/costs/src/money.rs:383:31 False: assert!(widest > 0 && widest < i128::MAX);
  F? assert! message argument                LINE   crates/costs/src/schedule.rs:1213: rate.source()
  F? assert! message argument                LINE   crates/costs/src/schedule.rs:1280: rate.source()
  F? assert! message argument                LINE   crates/costs/src/schedule.rs:1288: rate.source()
  F? panic! (failing arm)                    LINE   crates/costs/src/schedule.rs:1634: panic!("an options card carries option taxes");
  F? panic! (failing arm)                    LINE   crates/costs/src/schedule.rs:1643: (card, trip) => panic!(
  F? assert! condition branch (failing side) BRANCH crates/costs/src/schedule.rs:1244:75 False: matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
  F? let-else in test                        BRANCH crates/costs/src/schedule.rs:1633:29 False: let Taxes::Options { stt_sell, .. } = card.taxes() else {
  F? assert! message argument                LINE   crates/costs/src/scope.rs:249: lines[arm]
  GAP production                             LINE   crates/costs/src/stack.rs:589: )?;
  GAP production                             LINE   crates/costs/src/stack.rs:720: )?;
  GAP production                             LINE   crates/costs/src/stack.rs:907: )?)
  F? assert! condition branch (failing side) BRANCH crates/costs/src/stack.rs:2681:17 False: assert!(thin.gross().raw() > 0 && !thin.is_win());
  F? assert! condition branch (failing side) BRANCH crates/costs/src/stack.rs:2681:43 False: assert!(thin.gross().raw() > 0 && !thin.is_win());
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:114: c.raw(),
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:115: f.raw()
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:121: c.raw(),
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:122: f.raw()
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:134: levy.raw()
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:143: levy.raw()
  F? assert_eq! message argument             LINE   crates/costs/tests/adversarial.rs:151: levy.raw()
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:263: n.raw(),
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:264: levy.raw()
  F? assert! message argument                LINE   crates/costs/tests/adversarial.rs:301: out.map(Paisa::raw)
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:128:21 False: assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:128:44 False: assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:194:21 False: ceil.is_ok()
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:195:28 False: && floor.is_ok()
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:196:28 False: && levy_ceiling(notional, rate).is_ok()
  F? assert! condition branch (failing side) BRANCH crates/costs/tests/adversarial.rs:197:28 False: && statutory_levy(notional, rate).is_ok(),
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:64: const fn case(
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:65: name: &'static str,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:66: kind: TradeKind,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:67: exchange: Exchange,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:68: quantity: i64,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:69: buy: Option<i64>,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:70: sell: Option<i64>,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:71: page: [i64; 8],
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:72: ) -> Case {
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:73: Case {
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:74: name,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:75: kind,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:76: exchange,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:77: quantity,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:78: buy,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:79: sell,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:80: page,
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:81: }
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:82: }
  F? assert! message argument                LINE   crates/costs/tests/dhan_calculator.rs:666: case.page[7]
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:719: )?;
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:743: Err(other) => format!("refused: {other}"),
  F? assert_eq! message argument             LINE   crates/costs/tests/dhan_calculator.rs:926: case.name, dated_lines[at].0
  GAP test code                              LINE   crates/costs/tests/dhan_calculator.rs:1087: )?;
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:514: fn percentiles(mut per_call_ns: Vec<u128>) -> (u128, u128, u128) {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:515: per_call_ns.sort_unstable();
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:516: let at = |q: usize| {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:517: let index = (per_call_ns.len() - 1) * q / 100;
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:518: per_call_ns.get(index).copied().unwrap_or(u128::MAX)
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:519: };
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:520: (
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:521: at(50),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:522: at(99),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:523: per_call_ns.last().copied().unwrap_or(u128::MAX),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:524: )
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:525: }
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:527: fn time_per_call<T>(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:528: batches: usize,
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:529: per_batch: u32,
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:530: mut op: impl FnMut() -> T,
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:531: ) -> (u128, u128, u128) {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:532: let mut samples = Vec::with_capacity(batches);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:533: for _ in 0..batches {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:534: let start = Instant::now();
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:535: for _ in 0..per_batch {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:536: black_box(op());
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:537: }
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:538: samples.push(start.elapsed().as_nanos() / u128::from(per_batch));
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:540: percentiles(samples)
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:541: }
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:549: fn the_dated_lookups_cost_the_same_on_every_row_measured() {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:552: let days = [
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:553: ("2020-06-01 (refusal)", day(2020, 6, 1)),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:554: ("2024-10-01 (FA64232)", day(2024, 10, 1)),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:555: ("2026-02-27 (FA64232)", day(2026, 2, 27)),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:556: ("2026-03-01 (FA73061)", day(2026, 3, 1)),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:557: ("2100-12-31 (FA73061)", TradeDay::MAX),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:558: ];
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:559: for (label, on) in days {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:560: let exchange = exchange_charge_rate(Exchange::Nse, on);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:561: let ipft = ipft_rate(Exchange::Nse, on);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:562: let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:563: exchange_charge_rate(black_box(Exchange::Nse), black_box(on))
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:564: });
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:565: println!(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:568: assert!(p99 < 10_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:569: let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:570: ipft_rate(black_box(Exchange::Nse), black_box(on))
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:571: });
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:572: println!(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:575: assert!(p99 < 10_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:576: let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:577: Rates::resolve(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:578: black_box(Broker::Groww),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:579: black_box(Exchange::Nse),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:580: black_box(on),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:582: });
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:583: println!(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:586: assert!(p99 < 10_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:589: let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:590: let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:591: RateCard::resolve(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:592: black_box(TradeKind::Options),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:593: black_box(Exchange::Nse),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:594: black_box(on),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:595: black_box(RateSelector::Dated),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:597: });
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:598: println!(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:601: assert!(p99 < 10_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:602: assert_eq!(
  F? assert_eq! message argument             LINE   crates/costs/tests/fa73061.rs:603: RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:607: assert_eq!(exchange_charge_rate(Exchange::Nse, on), exchange);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:608: assert_eq!(ipft_rate(Exchange::Nse, on), ipft);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:610: let on = day(2026, 3, 2);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:611: let trip = nifty_trip(on, on, 10_000, 12_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:612: let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || price(black_box(&trip)));
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:613: println!(
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:616: assert!(p99 < 10_000);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:617: assert_eq!(price(&trip).expect("verified").exchange().raw(), 510);
  GAP test code                              LINE   crates/costs/tests/fa73061.rs:618: }
  GAP test code                              BRANCH crates/engine/src/column.rs:305:24 False: if let Some(acc) = hit_words.get_mut(word) {
  GAP production                             BRANCH crates/engine/src/keep.rs:398:20 False: if let Some(evicted) = self.take_root() {
  GAP test code                              BRANCH crates/engine/src/keep.rs:541:13 False: !weaker(&best.held, at.saturating_mul(2).saturating_add(1), at)
  F? assert! condition branch (failing side) BRANCH crates/engine/src/keep.rs:825:17 False: assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
  F? assert! condition branch (failing side) BRANCH crates/engine/src/keep.rs:825:39 False: assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
  F? assert! condition branch (failing side) BRANCH crates/engine/src/keep.rs:832:13 False: !lost.reconciles() && !Tally::of(&lost).reconciles(),
  F? assert! condition branch (failing side) BRANCH crates/engine/src/keep.rs:832:35 False: !lost.reconciles() && !Tally::of(&lost).reconciles(),
  GAP production                             LINE   crates/engine/src/lib.rs:1278: Err(_) => self.refused_on_memory(len_u64(bar_bits.len())),
  GAP production                             LINE   crates/engine/src/lib.rs:1309: return self.refused_on_memory(column.bars());
  GAP production                             LINE   crates/engine/src/lib.rs:1373: return self.refused_streamed_on_memory(column.bars());
  GAP production                             LINE   crates/engine/src/lib.rs:1408: Err(_) => self.refused_streamed_on_memory(len_u64(bar_bits.len())),
  GAP production                             LINE   crates/engine/src/lib.rs:1440: return self.refused_streamed_on_memory(len_u64(bar_bits.len()));
  GAP production                             LINE   crates/engine/src/lib.rs:1443: return self.refused_streamed_on_memory(column.bars());
  GAP production                             LINE   crates/engine/src/lib.rs:1481: Err(_) => Tail {
  GAP production                             LINE   crates/engine/src/lib.rs:1482: excluded: Vec::new(),
  GAP production                             LINE   crates/engine/src/lib.rs:1483: bars: column.bars(),
  GAP production                             LINE   crates/engine/src/lib.rs:1484: min_hits: self.min_hits,
  GAP production                             LINE   crates/engine/src/lib.rs:1485: halted: Some(self.halt(0, 0, 0, Breach::Memory)),
  GAP production                             LINE   crates/engine/src/lib.rs:1486: },
  GAP production                             LINE   crates/engine/src/lib.rs:2034: batch_failure = Some(breach);
  GAP production                             LINE   crates/engine/src/lib.rs:2035: break 'join;
  GAP production                             LINE   crates/engine/src/lib.rs:2058: // A drain commits its counts and rows only after every worker
  GAP production                             LINE   crates/engine/src/lib.rs:2059: // succeeds. Uncommitted candidates must not be labelled infrequent
  GAP production                             LINE   crates/engine/src/lib.rs:2060: // or vanish from a supposedly reconciled completed frontier.
  GAP production                             LINE   crates/engine/src/lib.rs:2061: generated = generated.saturating_sub(len_u64(batch.len()));
  GAP production                             LINE   crates/engine/src/lib.rs:2062: emitted = emitted.saturating_sub(batch.len());
  GAP production                             LINE   crates/engine/src/lib.rs:2063: halted = Some(self.halt(
  GAP production                             LINE   crates/engine/src/lib.rs:2064: k,
  GAP production                             LINE   crates/engine/src/lib.rs:2065: admitted.saturating_add(emitted),
  GAP production                             LINE   crates/engine/src/lib.rs:2066: pairs_walked.saturating_add(pairs),
  GAP production                             LINE   crates/engine/src/lib.rs:2067: breach,
  GAP production                             LINE   crates/engine/src/lib.rs:2068: ));
  GAP production                             LINE   crates/engine/src/lib.rs:2270: None
  F? assert! message argument                LINE   crates/engine/src/lib.rs:3270: first.frequent.capacity(),
  F? assert! message argument                LINE   crates/engine/src/lib.rs:3271: repeated.len()
  GAP test code                              LINE   crates/engine/src/lib.rs:5278: continue;
  GAP test code                              LINE   crates/engine/src/lib.rs:5895: return Err("the walk advanced beyond its terminal boundary".into());
  GAP production                             BRANCH crates/engine/src/lib.rs:191:24 False: if join_screen(frequent, a, b).is_some() {
  GAP production                             BRANCH crates/engine/src/lib.rs:1308:13 False: let Ok(levels) = reserved(level_slots()) else {
  GAP production                             BRANCH crates/engine/src/lib.rs:1372:13 False: let Ok(levels) = reserved(level_slots()) else {
  GAP production                             BRANCH crates/engine/src/lib.rs:1439:13 False: let Ok(column) = Column::try_from_rows(bar_bits) else {
  GAP production                             BRANCH crates/engine/src/lib.rs:1442:13 False: let Ok(levels) = reserved(level_slots()) else {
  GAP production                             BRANCH crates/engine/src/lib.rs:2026:32 True: if let Err(breach) = drain(
  GAP production                             BRANCH crates/engine/src/lib.rs:2057:16 True: if let Some(breach) = failed {
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:2845:13 False: !out_par.is_empty() && inf_par > 0,
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:2845:36 False: !out_par.is_empty() && inf_par > 0,
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3002:48 True: .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3002:48 False: .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3210:17 False: assert!(result.levels.is_empty() && result.excluded.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3210:45 False: assert!(result.levels.is_empty() && result.excluded.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3232:17 False: assert!(retained.halted.is_none() && retained.levels.len() > 1);
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3232:46 False: assert!(retained.halted.is_none() && retained.levels.len() > 1);
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3287:17 False: assert!(halt.is_none() && !previous.frequent.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3287:35 False: assert!(halt.is_none() && !previous.frequent.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3289:13 False: admitted > 0 && pairs > 0,
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3289:29 False: admitted > 0 && pairs > 0,
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3303:17 False: assert!(next.frequent.is_empty() && next.reconciles());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3303:45 False: assert!(next.frequent.is_empty() && next.reconciles());
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3607:13 False: text.contains("excluded 3 0 AlwaysFalse")
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3608:20 False: && text.contains("excluded 9 8 AlwaysTrue")
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3609:20 False: && text.contains("excluded 240 unmeasured NotLive"),
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3977:13 False: roomy.completed() && tight.completed(),
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:3977:34 False: roomy.completed() && tight.completed(),
  GAP test code                              BRANCH crates/engine/src/lib.rs:4149:17 False: let [prev, next] = pair else { continue };
  GAP test code                              BRANCH crates/engine/src/lib.rs:4152:17 False: let Ok(probe) = built else { continue };
  GAP test code                              BRANCH crates/engine/src/lib.rs:5158:28 False: && set_positions(&cand).all(|x| prev.contains(&cand.without_bit(x)))
  F? let-else in test                        BRANCH crates/engine/src/lib.rs:5277:17 False: let (Some(&(_, a0, p0)), Some(&(_, a1, p1))) = (pair.first(), pair.get(1)) else {
  GAP test code                              BRANCH crates/engine/src/lib.rs:5597:24 False: if pending > 0 {
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:5608:21 False: assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
  F? assert! condition branch (failing side) BRANCH crates/engine/src/lib.rs:5608:34 False: assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
  GAP test code                              BRANCH crates/engine/src/lib.rs:5894:24 True: if terminal_seen {
  GAP test code                              BRANCH crates/engine/tests/adversarial.rs:76:12 False: if let Some(p) = all.get(nth % all.len().max(1))
  GAP test code                              BRANCH crates/engine/tests/adversarial.rs:77:16 False: && let Ok(idx) = u16::try_from(*p)
  GAP test code                              LINE   crates/engine/tests/production_callers.rs:48: return;
  F? assert! message argument                LINE   crates/engine/tests/production_callers.rs:198: file.path.display(),
  F? assert! message argument                LINE   crates/engine/tests/production_callers.rs:199: at + 1
  F? let-else in test                        BRANCH crates/engine/tests/production_callers.rs:47:13 False: let Ok(entries) = std::fs::read_dir(dir) else {
  GAP test code                              BRANCH crates/engine/tests/production_callers.rs:54:23 False: } else if path.extension().is_some_and(|e| e == "rs") {
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/production_callers.rs:138:9 False: engine > 1 && others > 1,
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/production_callers.rs:138:23 False: engine > 1 && others > 1,
  GAP test code                              BRANCH crates/engine/tests/production_callers.rs:189:21 False: !l.is_empty()
  GAP test code                              BRANCH crates/engine/tests/production_callers.rs:191:34 True: || !(l.ends_with(';') || l.ends_with('}')))
  F? assert! message argument                LINE   crates/engine/tests/resume_readiness.rs:76: bytes.get(56..192).expect("header")
  F? assert! message argument                LINE   crates/engine/tests/resume_readiness.rs:264: ()
  F? assert! message argument                LINE   crates/engine/tests/resume_readiness.rs:276: &mut |_| Ok(())
  F? assert! message argument                LINE   crates/engine/tests/resume_readiness.rs:288: &mut |_| Ok(())
  GAP test code                              LINE   crates/engine/tests/resume_readiness.rs:417: fn flush(&mut self) -> std::io::Result<()> {
  GAP test code                              LINE   crates/engine/tests/resume_readiness.rs:418: self.inner.flush()
  GAP test code                              LINE   crates/engine/tests/resume_readiness.rs:419: }
  GAP test code                              BRANCH crates/engine/tests/resume_readiness.rs:41:25 False: hits > 0 && hits < bars && hits >= min_hits.max(1)
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/resume_readiness.rs:96:67 False: matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/resume_readiness.rs:592:24 False: .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/resume_readiness.rs:814:65 False: assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/resume_readiness.rs:981:13 False: assert!(expected.completed() && need > 1, "fixture must walk pairs");
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/resume_readiness.rs:981:37 False: assert!(expected.completed() && need > 1, "fixture must walk pairs");
  F? assert! condition branch (failing side) BRANCH crates/engine/tests/sweep_readiness_oracle.rs:159:48 False: .all(|w| matches!(w, [x, y] if x.mask.words() < y.mask.words())))
  GAP production                             LINE   crates/indicators/src/anchored.rs:844: return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index });
  GAP production                             LINE   crates/indicators/src/anchored.rs:892: return Err(ExactMinuteGapRefusal::MalformedMinuteCadence { index: cursor });
  GAP production                             LINE   crates/indicators/src/anchored.rs:909: return Err(ExactMinuteGapRefusal::OverlayLengthMismatch);
  GAP production                             BRANCH crates/indicators/src/anchored.rs:843:17 False: let Some(previous) = exact_minute.get(index.saturating_sub(1)) else {
  GAP production                             BRANCH crates/indicators/src/anchored.rs:878:44 True: if minute.ts_micros != expected || crate::ist_day(minute.ts_micros) != signal_day {
  GAP production                             BRANCH crates/indicators/src/anchored.rs:891:13 False: let Some(mask) = minute_masks.get(cursor).copied() else {
  GAP production                             BRANCH crates/indicators/src/anchored.rs:908:8 True: if !replaced {
  GAP test code                              BRANCH crates/indicators/src/anchored.rs:1966:20 False: if let Some((stamp, last)) = bucket {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/anchored.rs:2177:28 False: if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
  GAP test code                              BRANCH crates/indicators/src/anchored.rs:2330:79 False: if opens.saturating_add(rung).saturating_sub(1) >= SHORT_CLOSE && opens <= SHORT_CLOSE {
  GAP test code                              BRANCH crates/indicators/src/anchored.rs:2659:28 True: if minute > OPEN_IST_MINUTE + 374 || roll < 6 {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/anchored.rs:2731:17 False: accepted > 500 && refused > 500,
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/anchored.rs:2731:35 False: accepted > 500 && refused > 500,
  GAP production                             LINE   crates/indicators/src/column.rs:1676: family.len()
  GAP production                             LINE   crates/indicators/src/column.rs:2154: poisoned_column.bits().len()
  GAP production                             BRANCH crates/indicators/src/column.rs:1412:16 False: if let Some(b) = bars.get_mut(victim) {
  GAP production                             BRANCH crates/indicators/src/column.rs:1493:16 False: if let Some(b) = bars.get_mut(victim) {
  GAP production                             BRANCH crates/indicators/src/column.rs:1594:20 False: if !now.is_empty() {
  GAP production                             BRANCH crates/indicators/src/column.rs:1793:25 False: assert!(!truth.get(bit) && !known.get(bit));
  GAP production                             BRANCH crates/indicators/src/column.rs:1793:44 False: assert!(!truth.get(bit) && !known.get(bit));
  GAP production                             BRANCH crates/indicators/src/column.rs:1796:25 False: assert!(!truth.get(bit) && !known.get(bit));
  GAP production                             BRANCH crates/indicators/src/column.rs:1796:44 False: assert!(!truth.get(bit) && !known.get(bit));
  GAP production                             BRANCH crates/indicators/src/column.rs:2102:16 False: if let Some(slot) = poisoned.get_mut(victim) {
  GAP production                             BRANCH crates/indicators/src/column.rs:2278:16 False: if let Some(bar) = bars.get_mut(victim) {
  GAP production                             BRANCH crates/indicators/src/column.rs:2332:13 False: !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
  GAP production                             BRANCH crates/indicators/src/column.rs:2332:62 False: !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
  GAP test code                              BRANCH crates/indicators/src/column.rs:2573:16 False: if let Some(slot) = onto.get_mut(1) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2576:16 False: if let Some(slot) = onto.get_mut(2) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2579:16 False: if let Some(slot) = onto.get_mut(3) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2582:16 False: if let Some(slot) = onto.get_mut(4) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/column.rs:2619:13 False: projected.sources().contains(&22) && projected.sources().contains(&24),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/column.rs:2619:50 False: projected.sources().contains(&22) && projected.sources().contains(&24),
  GAP test code                              BRANCH crates/indicators/src/column.rs:2694:16 False: if let Some(slot) = across_a_gap.get_mut(1) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2697:16 False: if let Some(slot) = across_a_gap.get_mut(2) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2700:16 False: if let Some(slot) = across_a_gap.get_mut(3) {
  GAP test code                              BRANCH crates/indicators/src/column.rs:2710:16 False: if let Some(slot) = across_a_gap.get_mut(3) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:869:17 False: assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:869:31 False: assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1114:17 False: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1114:30 False: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1114:43 False: assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1115:17 False: assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1115:31 False: assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1116:17 False: assert!(m.get(13) && m.get(15), "pdh/pdl not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1116:30 False: assert!(m.get(13) && m.get(15), "pdh/pdl not above");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1137:17 False: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1137:30 False: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1137:43 False: assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1138:17 False: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1138:30 False: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1138:43 False: assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1139:17 False: assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1139:31 False: assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1140:17 False: assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1140:31 False: assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1141:17 False: assert!(m.get(14) && m.get(16), "pdh/pdl not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1141:30 False: assert!(m.get(14) && m.get(16), "pdh/pdl not below");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1554:13 False: cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1554:44 False: cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1573:13 False: rendered.contains("pivot: 96") && rendered.contains("half: 1"),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/daily.rs:1573:47 False: rendered.contains("pivot: 96") && rendered.contains("half: 1"),
  GAP production                             LINE   crates/indicators/src/evaluator.rs:577: None => 0,
  GAP production                             LINE   crates/indicators/src/evaluator.rs:1000: Err(crate::vwap::Refused::Corrupt(why)) => return Err(why),
  GAP production                             LINE   crates/indicators/src/evaluator.rs:1172: continue;
  GAP production                             LINE   crates/indicators/src/evaluator.rs:1189: Side::Unknown => continue,
  GAP production                             LINE   crates/indicators/src/evaluator.rs:1200: continue;
  GAP production                             BRANCH crates/indicators/src/evaluator.rs:823:36 False: if self.day == next.day && self.seeded {
  GAP production                             BRANCH crates/indicators/src/evaluator.rs:1171:17 False: let Some(last) = self.last_side.get_mut(slot) else {
  GAP production                             BRANCH crates/indicators/src/evaluator.rs:1199:17 False: let Some(count) = self.crossings_seen.get_mut(slot) else {
  GAP production                             BRANCH crates/indicators/src/evaluator.rs:1418:16 False: && self.previous.is_some()
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:1586:78 False: Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:1796:20 False: if let Some(day) = changed.calendar.days.get_mut(index) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:1888:17 False: assert!(truth_only.warmed_up() && full.warmed_up());
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:1888:43 False: assert!(truth_only.warmed_up() && full.warmed_up());
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:1979:17 False: !first.get(above) && !first.get(below),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:1979:38 False: !first.get(above) && !first.get(below),
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3072:16 False: if mask.popcount() > 0 {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:3561:13 False: !first.get(276) && !first.get(277),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:3561:32 False: !first.get(276) && !first.get(277),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:3585:13 False: !flat.get(276) && !flat.get(277),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:3585:31 False: !flat.get(276) && !flat.get(277),
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3622:41 True: let event = mask.get(56) || mask.get(57) || mask.get(58) || mask.get(59);
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3622:57 True: let event = mask.get(56) || mask.get(57) || mask.get(58) || mask.get(59);
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/evaluator.rs:3645:36 True: !(mask.get(278) && mask.get(279)),
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3700:26 False: let era = if y >= 0 { y } else { y - 399 } / 400;
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3802:32 True: if mask.get(56) || mask.get(58) {
  GAP test code                              BRANCH crates/indicators/src/evaluator.rs:3804:23 True: } else if mask.get(57) || mask.get(59) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/fib.rs:584:21 False: assert!(p != 0 && p != 1000, "rung {p} should have no position here");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/fib.rs:584:31 False: assert!(p != 0 && p != 1000, "rung {p} should have no position here");
  GAP production                             LINE   crates/indicators/src/gap.rs:497: return known;
  GAP production                             BRANCH crates/indicators/src/gap.rs:479:24 False: && let Ok(next) =
  GAP production                             BRANCH crates/indicators/src/gap.rs:496:13 False: let Some(range) = leg.length() else {
  GAP production                             BRANCH crates/indicators/src/gap.rs:501:20 False: && tolerance.covers(level, level, range)
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/gap.rs:1491:13 False: fired > 0 && legs > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/gap.rs:1491:26 False: fired > 0 && legs > 0,
  GAP test code                              LINE   crates/indicators/src/lib.rs:2022: }
  GAP test code                              LINE   crates/indicators/src/lib.rs:2023: start = start.saturating_add(day);
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/lib.rs:883:17 True: assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/lib.rs:883:36 False: assert!(ist_day(-1) < 0 || ist_day(-1) == 0);
  GAP test code                              BRANCH crates/indicators/src/lib.rs:2020:16 False: if crate::weekday_bit(start) == Some(365) {
  GAP production                             LINE   crates/indicators/src/orb.rs:330: continue;
  GAP production                             BRANCH crates/indicators/src/orb.rs:180:39 True: if day != self.session_day || !self.live {
  GAP production                             BRANCH crates/indicators/src/orb.rs:329:17 False: let Some(range) = high.checked_sub(low) else {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:555:13 False: o.extremes(0).is_some() && o.extremes(1).is_some(),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:555:40 False: o.extremes(0).is_some() && o.extremes(1).is_some(),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:559:13 False: o.extremes(2).is_none() && o.extremes(3).is_none(),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:559:40 False: o.extremes(2).is_none() && o.extremes(3).is_none(),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:610:17 False: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:610:40 False: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/orb.rs:610:63 False: assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1558:17 False: assert!(shape.bullish() && !shape.bearish());
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1558:36 False: assert!(shape.bullish() && !shape.bearish());
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1562:17 False: assert!(shape.body_at_most(250) && !shape.body_at_most(249));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1562:44 False: assert!(shape.body_at_most(250) && !shape.body_at_most(249));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1597:13 False: m2.get(153) && m2.get(155),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1597:28 False: m2.get(153) && m2.get(155),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/pattern.rs:1711:41 True: !(mask.get(bull) && mask.get(bear)),
  GAP production                             BRANCH crates/indicators/src/session.rs:270:39 True: if day != self.session_day || !self.live {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:586:17 False: assert!(truth.get(66) && !truth.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:586:34 False: assert!(truth.get(66) && !truth.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:587:17 False: assert!(known.get(66) && known.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:587:34 False: assert!(known.get(66) && known.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:589:13 False: !truth.get(68) && !known.get(68),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:589:31 False: !truth.get(68) && !known.get(68),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:655:13 False: !bits.get(42) && !bits.get(43),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:655:30 False: !bits.get(42) && !bits.get(43),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:680:13 False: !edge.get(44) && edge.get(45),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:680:30 False: !edge.get(44) && edge.get(45),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:788:17 False: assert!(up.get(30) && !up.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:788:31 False: assert!(up.get(30) && !up.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:790:17 False: assert!(down.get(31) && !down.get(30));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:790:33 False: assert!(down.get(31) && !down.get(30));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:793:13 False: !flat.get(30) && !flat.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:793:30 False: !flat.get(30) && !flat.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:838:17 False: assert!(!mask.get(37) && !mask.get(38));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:838:34 False: assert!(!mask.get(37) && !mask.get(38));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:858:17 False: !mask.get(30) && !mask.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:858:34 False: !mask.get(30) && !mask.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:922:13 False: !flat.get(30) && !flat.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:922:30 False: !flat.get(30) && !flat.get(31),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:931:17 False: assert!(!mask.get(37) && !mask.get(38));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:931:34 False: assert!(!mask.get(37) && !mask.get(38));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:950:13 False: !mask.get(37) && !mask.get(38) && !mask.get(39),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:950:30 False: !mask.get(37) && !mask.get(38) && !mask.get(39),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:950:47 False: !mask.get(37) && !mask.get(38) && !mask.get(39),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:966:17 False: assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:966:33 False: assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:968:17 False: assert!(mask.get(66) && !mask.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:968:33 False: assert!(mask.get(66) && !mask.get(67));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:972:17 False: assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:972:34 False: assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:974:13 False: !mask.get(66) && !mask.get(67) && !mask.get(68),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:974:30 False: !mask.get(66) && !mask.get(67) && !mask.get(68),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:974:47 False: !mask.get(66) && !mask.get(67) && !mask.get(68),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:989:17 False: assert!(mask.get(50) && !mask.get(51));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:989:33 False: assert!(mask.get(50) && !mask.get(51));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:993:17 False: assert!(mask.get(51) && !mask.get(50));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:993:33 False: assert!(mask.get(51) && !mask.get(50));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:1283:13 False: !mask.get(66) && !mask.get(67),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/session.rs:1283:30 False: !mask.get(66) && !mask.get(67),
  GAP production                             LINE   crates/indicators/src/trend.rs:581: return;
  GAP production                             LINE   crates/indicators/src/trend.rs:590: return;
  GAP production                             BRANCH crates/indicators/src/trend.rs:580:13 False: let Some(Some((mid_high, mid_low))) = window.get(FRACTAL).copied() else {
  GAP production                             BRANCH crates/indicators/src/trend.rs:589:17 False: let Some((high, low)) = *entry else {
  GAP production                             BRANCH crates/indicators/src/trend.rs:891:38 False: if self.supertrend.warm() && self.supertrend.stop().is_some() {
  GAP production                             BRANCH crates/indicators/src/trend.rs:950:20 False: && let Some(fast) = self.fast.value()
  GAP production                             BRANCH crates/indicators/src/trend.rs:955:20 False: && let Some(slow) = self.slow.value()
  GAP production                             BRANCH crates/indicators/src/trend.rs:965:20 False: && let (Some(fast), Some(slow)) = (self.fast.value(), self.slow.value())
  GAP production                             BRANCH crates/indicators/src/trend.rs:975:20 False: && let Some(stop) = self.supertrend.stop()
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1292:13 False: !mask.get(0) && !mask.get(1),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1292:29 False: !mask.get(0) && !mask.get(1),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1296:13 False: !mask.get(2) && !mask.get(3),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1296:29 False: !mask.get(2) && !mask.get(3),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1300:13 False: !mask.get(4) && !mask.get(5),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:1300:29 False: !mask.get(4) && !mask.get(5),
  GAP test code                              BRANCH crates/indicators/src/trend.rs:1759:32 True: if mask.get(56) || mask.get(57) {
  GAP test code                              BRANCH crates/indicators/src/trend.rs:1762:16 True: if mask.get(58) || mask.get(59) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/trend.rs:2296:74 False: matches!((previous, now), (Some(before), Some(after)) if after <= before),
  GAP test code                              BRANCH crates/indicators/src/trend.rs:2299:73 False: if matches!((previous, now), (Some(before), Some(after)) if after < before) {
  GAP test code                              LINE   crates/indicators/src/vwap.rs:1729: return None;
  GAP test code                              LINE   crates/indicators/src/vwap.rs:1841: continue;
  GAP production                             BRANCH crates/indicators/src/vwap.rs:197:11 False: while i < NEWTON_STEPS {
  GAP production                             BRANCH crates/indicators/src/vwap.rs:409:39 True: if day != self.session_day || !self.live {
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:776:17 False: assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:776:34 False: assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:777:17 False: assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:777:34 False: assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:889:17 False: assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:889:44 False: assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:1235:21 False: !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
  F? assert! condition branch (failing side) BRANCH crates/indicators/src/vwap.rs:1235:52 False: !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
  GAP test code                              BRANCH crates/indicators/src/vwap.rs:1728:16 True: if !fits(0) {
  GAP test code                              BRANCH crates/indicators/src/vwap.rs:1840:16 True: if b_cap < 1 {
  GAP test code                              BRANCH crates/indicators/src/vwap.rs:1847:16 False: if r.checked_mul(r).is_none() {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/crossing_known_readiness.rs:84:32 True: !(above && below),
  GAP test code                              BRANCH crates/indicators/tests/crossing_known_readiness.rs:96:24 False: && known.get(u32::from(level.above))
  GAP test code                              LINE   crates/indicators/tests/current_day_fib_known_readiness.rs:80: return None;
  GAP test code                              BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:79:8 True: if range <= 0 || range > i128::from(i64::MAX) {
  GAP test code                              BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:79:22 True: if range <= 0 || range > i128::from(i64::MAX) {
  GAP test code                              BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:96:45 True: if i64::try_from(level).is_err() || tolerance.base() != Some(Base::SessionRange) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:212:13 False: false_count > 0 && true_count > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:212:32 False: false_count > 0 && true_count > 0,
  GAP test code                              LINE   crates/indicators/tests/exact_minute_orb_gap_readiness.rs:130: Truth::Unknown
  GAP test code                              BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:129:12 True: if range == 0 {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:13 False: trues > 0 && falses > 0 && unknowns > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:26 False: trues > 0 && falses > 0 && unknowns > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:40 False: trues > 0 && falses > 0 && unknowns > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:13 False: !truth.get(bit) && !known.get(bit),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:32 False: !truth.get(bit) && !known.get(bit),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:17 False: !truth.get(bit) && !known.get(bit),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:36 False: !truth.get(bit) && !known.get(bit),
  GAP test code                              LINE   crates/indicators/tests/fib_known_readiness.rs:116: return Truth::Unknown;
  GAP test code                              LINE   crates/indicators/tests/fib_known_readiness.rs:434: }
  GAP test code                              BRANCH crates/indicators/tests/fib_known_readiness.rs:93:22 True: if range <= 0 || range > i128::from(i64::MAX) {
  GAP test code                              BRANCH crates/indicators/tests/fib_known_readiness.rs:115:8 True: if tolerance.base() != Some(Base::SessionRange) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/fib_known_readiness.rs:174:13 False: !truth.get(retired) && !known.get(retired),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/fib_known_readiness.rs:174:36 False: !truth.get(retired) && !known.get(retired),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/fib_known_readiness.rs:229:13 False: trues > 0 && falses > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/fib_known_readiness.rs:229:26 False: trues > 0 && falses > 0,
  GAP test code                              BRANCH crates/indicators/tests/fib_known_readiness.rs:418:12 False: if *source >= 375 {
  GAP test code                              BRANCH crates/indicators/tests/fib_rung_rounding.rs:128:32 False: if let Ok(close) = i64::try_from(level + i128::from(*d)) {
  GAP test code                              LINE   crates/indicators/tests/gap_known_readiness.rs:119: return [Truth::Unknown; 11];
  GAP test code                              BRANCH crates/indicators/tests/gap_known_readiness.rs:71:39 True: if span_of(current) == opening || tolerance.base() != Some(Base::SessionRange) {
  GAP test code                              BRANCH crates/indicators/tests/gap_known_readiness.rs:118:8 True: if length == 0 || length > i128::from(i64::MAX) {
  GAP test code                              BRANCH crates/indicators/tests/gap_known_readiness.rs:118:23 True: if length == 0 || length > i128::from(i64::MAX) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/gap_known_readiness.rs:216:21 False: assert!(false_count > 0 && true_count > 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/gap_known_readiness.rs:216:40 False: assert!(false_count > 0 && true_count > 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/gap_known_readiness.rs:256:17 False: assert!(false_count > 0 && true_count > 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/gap_known_readiness.rs:256:36 False: assert!(false_count > 0 && true_count > 0);
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:101: if trimmed.is_empty() {
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:102: continue;
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:103: }
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:104: break;
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:116: }
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:118: None
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:177: let one = part.parse::<u16>().ok()?;
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:178: (one, one)
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:181: return None;
  GAP test code                              LINE   crates/indicators/tests/module_doc_counts.rs:234: return None;
  F? assert_eq! message argument             LINE   crates/indicators/tests/module_doc_counts.rs:271: positions.len()
  F? assert_eq! message argument             LINE   crates/indicators/tests/module_doc_counts.rs:321: positions.len()
  F? assert_eq! message argument             LINE   crates/indicators/tests/module_doc_counts.rs:332: emitted.len()
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:97:12 True: if !trimmed.starts_with("//!") {
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:101:16 True: if trimmed.is_empty() {
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:101:16 False: if trimmed.is_empty() {
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:110:13 False: let Some(end) = rest.find(' ') else { continue };
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:112:16 False: if let Ok(n) = word.parse::<usize>()
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:113:16 False: && rest.contains("vocabulary position")
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:170:31 False: let (lo, hi) = if let Some((lo, hi)) = part.split_once('–').or_else(|| part.split_once('-'))
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:180:12 True: if lo > hi {
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:196:16 True: || line.starts_with("| Module | Positions | Count |")
  GAP test code                              BRANCH crates/indicators/tests/module_doc_counts.rs:233:8 True: if !rest.starts_with("live positions are computable") {
  GAP test code                              LINE   crates/indicators/tests/orb_known_readiness.rs:77: return [Truth::Unknown; 5];
  GAP test code                              BRANCH crates/indicators/tests/orb_known_readiness.rs:76:8 True: if range > i128::from(i64::MAX) {
  GAP test code                              BRANCH crates/indicators/tests/orb_known_readiness.rs:82:26 True: if range == 0 || tolerance.base() != Some(Base::SessionRange) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:13 False: false_count > 0 && true_count > 0 && unknown_count > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:32 False: false_count > 0 && true_count > 0 && unknown_count > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/orb_known_readiness.rs:194:50 False: false_count > 0 && true_count > 0 && unknown_count > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_from_outside.rs:200:9 False: mask.get(46) && !mask.get(47),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_from_outside.rs:200:25 False: mask.get(46) && !mask.get(47),
  GAP test code                              LINE   crates/indicators/tests/session_known_readiness.rs:172: _ => Truth::Unknown,
  GAP test code                              BRANCH crates/indicators/tests/session_known_readiness.rs:168:23 False: 68 if tolerance.base() == Some(Base::SessionRange) => answer(
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:360:13 False: !truth.get(position) && !known.get(position),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:360:37 False: !truth.get(position) && !known.get(position),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:364:13 False: assert!(known.get(48) && known.get(49));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:364:30 False: assert!(known.get(48) && known.get(49));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:365:13 False: assert!(!truth.get(48) && !truth.get(49));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:365:31 False: assert!(!truth.get(48) && !truth.get(49));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:452:13 False: assert!(known.get(33) && !truth.get(33));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:452:30 False: assert!(known.get(33) && !truth.get(33));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:453:13 False: assert!(!known.get(37) && !known.get(66));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:453:31 False: assert!(!known.get(37) && !known.get(66));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:504:13 False: assert!(truth.get(48) && truth.get(50) && truth.get(68));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:504:30 False: assert!(truth.get(48) && truth.get(50) && truth.get(68));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/session_known_readiness.rs:504:47 False: assert!(truth.get(48) && truth.get(50) && truth.get(68));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:9 False: hammers > 0 && hammers < cases,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:24 False: hammers > 0 && hammers < cases,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:13 False: assert!(bullish > 0 && bearish > 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:28 False: assert!(bullish > 0 && bearish > 0);
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:17 False: assert!(!truth.get(30) && !truth.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:35 False: assert!(!truth.get(30) && !truth.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:17 False: assert!(known.get(30) && known.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:34 False: assert!(known.get(30) && known.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:17 False: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:34 False: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:51 False: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:68 False: assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:9 False: !next_day.get(157) && !next_day.get(176),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:31 False: !next_day.get(157) && !next_day.get(176),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:9 False: !next_day.get(276) && !next_day.get(277),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:31 False: !next_day.get(276) && !next_day.get(277),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:17 False: assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:47 False: assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:21 False: assert!(known.get(30) && known.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:38 False: assert!(known.get(30) && known.get(31));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:17 False: assert!(!truth.get(13) && !truth.get(16));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:35 False: assert!(!truth.get(13) && !truth.get(16));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:13 False: known.get(13) && known.get(16),
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:30 False: known.get(13) && known.get(16),
  GAP test code                              LINE   crates/indicators/tests/trend_known_readiness.rs:77: (Some(high), Some(low)) if high.confirmed != low.confirmed => {
  GAP test code                              LINE   crates/indicators/tests/trend_known_readiness.rs:78: Some(high.confirmed > low.confirmed)
  GAP test code                              LINE   crates/indicators/tests/trend_known_readiness.rs:120: return;
  GAP test code                              BRANCH crates/indicators/tests/trend_known_readiness.rs:77:40 True: (Some(high), Some(low)) if high.confirmed != low.confirmed => {
  GAP test code                              BRANCH crates/indicators/tests/trend_known_readiness.rs:77:40 False: (Some(high), Some(low)) if high.confirmed != low.confirmed => {
  GAP test code                              BRANCH crates/indicators/tests/trend_known_readiness.rs:90:32 False: Some(level) if band.base() == Some(Base::SessionRange) && level.range > 0 => {
  GAP test code                              BRANCH crates/indicators/tests/trend_known_readiness.rs:90:75 False: Some(level) if band.base() == Some(Base::SessionRange) && level.range > 0 => {
  GAP test code                              BRANCH crates/indicators/tests/trend_known_readiness.rs:119:12 True: if range > i128::from(i64::MAX) {
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/trend_known_readiness.rs:177:45 False: assert!(!truth.get(position) || known.get(position));
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:13 False: trues > 0 && falses > 0 && unknowns > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:26 False: trues > 0 && falses > 0 && unknowns > 0,
  F? assert! condition branch (failing side) BRANCH crates/indicators/tests/trend_known_readiness.rs:251:40 False: trues > 0 && falses > 0 && unknowns > 0,
  GAP test code                              LINE   crates/indicators/tests/withheld_fold.rs:148: continue;
  F? let-else in test                        BRANCH crates/indicators/tests/withheld_fold.rs:147:13 False: let Some(old) = spliced_at(source) else {
  F? panic! (failing arm)                    LINE   crates/lake/src/bar.rs:237: other => panic!("expected NotRepresentable, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/bar.rs:247: other => panic!("expected NotRepresentable, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:412: ContractKind::Future => panic!("a CE is not a future"),
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:563: other => panic!("expected TooLong, got {other:?}"),
  F? assert! message argument                LINE   crates/lake/src/contract.rs:571: name.len()
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:614: ContractKind::Future => panic!("not a future"),
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:687: other => panic!("{padded} must be refused, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:721: other => panic!("{signed} must be refused, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/contract.rs:753: other => panic!("{name} must be refused, got {other:?}"),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/contract.rs:786:61 False: Err(ContractError::BadExpiryShape { found }) if found == "0\u{e9}Apr2"
  F? panic! (failing arm)                    LINE   crates/lake/src/footer_tests.rs:33: panic!("expected the walk to admit, got {e:?}");
  F? panic! (failing arm)                    LINE   crates/lake/src/footer_tests.rs:40: other => panic!("expected FooterUnreadable, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/footer_tests.rs:69: other => panic!("expected FooterUnreadable, got {:?}", other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/src/footer_tests.rs:80: other => panic!("expected FooterUnreadable, got {:?}", other.err()),
  GAP test code                              BRANCH crates/lake/src/footer_tests.rs:32:12 True: if let Err(e) = r {
  GAP production                             LINE   crates/lake/src/page.rs:400: ParquetError::General("page header length does not fit in usize".to_owned())
  GAP production                             LINE   crates/lake/src/page.rs:401: })?;
  GAP production                             LINE   crates/lake/src/page.rs:474: return Err(ParquetError::General(
  GAP production                             LINE   crates/lake/src/page.rs:475: "page offset runs past the end of the column chunk".to_owned(),
  GAP production                             LINE   crates/lake/src/page.rs:476: ));
  F? panic! (failing arm)                    LINE   crates/lake/src/page.rs:878: other => panic!("expected a data page, got {other:?}"),
  F? assert! message argument                LINE   crates/lake/src/page.rs:986: frame.len(),
  F? assert! message argument                LINE   crates/lake/src/page.rs:987: plain.len()
  GAP test code                              LINE   crates/lake/src/page.rs:1001: 1 => 2,
  GAP test code                              LINE   crates/lake/src/page.rs:1002: 2 => 4,
  GAP test code                              LINE   crates/lake/src/page.rs:1003: _ => 8,
  GAP test code                              LINE   crates/lake/src/page.rs:1007: 1 => 1,
  GAP test code                              LINE   crates/lake/src/page.rs:1008: 2 => 2,
  GAP test code                              LINE   crates/lake/src/page.rs:1009: _ => 4,
  F? panic! (failing arm)                    LINE   crates/lake/src/page.rs:1034: other => panic!("expected a data page, got {other:?}"),
  F? assert! message argument                LINE   crates/lake/src/page.rs:1102: frame.len()
  GAP production                             BRANCH crates/lake/src/page.rs:473:17 False: let Some(rest) = self.chunk.get(self.pos..) else {
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:631:13 False: short.to_string().contains("4 bytes") && short.to_string().contains('5'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:631:54 False: short.to_string().contains("4 bytes") && short.to_string().contains('5'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:640:13 False: long.to_string().contains("4 bytes") && long.to_string().contains('3'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:640:53 False: long.to_string().contains("4 bytes") && long.to_string().contains('3'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:681:13 False: wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:681:65 False: wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:741:13 False: text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:741:51 False: text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:1147:13 False: text.contains("zstd frame refused") && text.contains("134217729"),
  F? assert! condition branch (failing side) BRANCH crates/lake/src/page.rs:1147:52 False: text.contains("zstd frame refused") && text.contains("134217729"),
  GAP production                             LINE   crates/lake/src/reader.rs:97: reason: e.to_string(),
  GAP production                             LINE   crates/lake/src/reader.rs:98: };
  GAP production                             LINE   crates/lake/src/reader.rs:117: return Err(LakeError::TooLarge { bytes: read, cap });
  GAP production                             LINE   crates/lake/src/reader.rs:293: value: declared_rows,
  GAP production                             LINE   crates/lake/src/reader.rs:294: })?;
  GAP production                             LINE   crates/lake/src/reader.rs:538: column: name.to_owned(),
  GAP production                             LINE   crates/lake/src/reader.rs:539: reason: format!("column chunk {start}..{end} runs past the end of the file"),
  GAP production                             LINE   crates/lake/src/reader.rs:540: })?;
  GAP production                             LINE   crates/lake/src/reader.rs:727: _ => Err(self.mismatch(name)),
  GAP production                             LINE   crates/lake/src/reader.rs:737: _ => Err(self.mismatch(name)),
  GAP production                             LINE   crates/lake/src/reader.rs:749: _ => Err(self.mismatch(name)),
  GAP production                             LINE   crates/lake/src/reader.rs:754: fn mismatch(&self, name: &'static str) -> LakeError {
  GAP production                             LINE   crates/lake/src/reader.rs:755: let want = crate::schema::specs(self.file.layout)
  GAP production                             LINE   crates/lake/src/reader.rs:756: .find(|s| s.name == name)
  GAP production                             LINE   crates/lake/src/reader.rs:757: .map_or(crate::error::ColumnType::Other, |s| s.ty);
  GAP production                             LINE   crates/lake/src/reader.rs:758: let got = self
  GAP production                             LINE   crates/lake/src/reader.rs:759: .index_of(name)
  GAP production                             LINE   crates/lake/src/reader.rs:760: .map_or(crate::error::ColumnType::Other, |i| {
  GAP production                             LINE   crates/lake/src/reader.rs:761: crate::schema::physical(
  GAP production                             LINE   crates/lake/src/reader.rs:762: self.file
  GAP production                             LINE   crates/lake/src/reader.rs:763: .meta
  GAP production                             LINE   crates/lake/src/reader.rs:764: .file_metadata()
  GAP production                             LINE   crates/lake/src/reader.rs:765: .schema_descr()
  GAP production                             LINE   crates/lake/src/reader.rs:766: .column(i)
  GAP production                             LINE   crates/lake/src/reader.rs:767: .physical_type(),
  GAP production                             LINE   crates/lake/src/reader.rs:769: });
  GAP production                             LINE   crates/lake/src/reader.rs:770: LakeError::ColumnTypeMismatch { name, want, got }
  GAP production                             LINE   crates/lake/src/reader.rs:771: }
  GAP production                             LINE   crates/lake/src/reader.rs:793: None => out.push(None),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:899: other => panic!("expected TooLarge, got {:?}", other.err()),
  F? assert! message argument                LINE   crates/lake/src/reader.rs:911: matches!(
  F? assert! message argument                LINE   crates/lake/src/reader.rs:918: matches!(
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:952: other => panic!("expected NotParquet, got {other:?}", other = other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:965: other => panic!("expected NotParquet, got {other:?}", other = other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:978: other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1052: other => panic!("expected ShortColumnChunk, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1098: other => panic!("expected ShortColumnChunk, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1115: other => panic!("expected ShortColumnChunk, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1145: other => panic!("expected UnexpectedNull, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1280: _ => panic!("the cash layout is INT64 and DOUBLE only"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1387: other => panic!("expected ImpossibleLength, got {:?}", other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1421: Err(other) => panic!("refused for the wrong reason: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1422: Ok(batch) => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1424: batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1478: Err(other) => panic!("refused for the wrong reason: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1479: Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1529: other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1553: Err(other) => panic!("refused for the wrong reason: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1554: Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1625: Err(other) => panic!("refused for the wrong reason: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1626: Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1726: other => panic!("the padded group was not refused as short: {other:?}"),
  GAP test code                              LINE   crates/lake/src/reader.rs:1755: match cut {
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1757: panic!("the cut and its copy sum to the file count")
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1800: other => panic!("group {group} under {claim}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1883: crate::error::ColumnType::Other => panic!("no lake column is Other"),
  F? panic! (failing arm)                    LINE   crates/lake/src/reader.rs:1924: _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),
  GAP production                             BRANCH crates/lake/src/reader.rs:116:8 True: if read > cap {
  GAP production                             LINE   crates/lake/src/schema.rs:34: const fn spec(name: &'static str, ty: ColumnType) -> ColumnSpec {
  GAP production                             LINE   crates/lake/src/schema.rs:35: ColumnSpec { name, ty }
  GAP production                             LINE   crates/lake/src/schema.rs:36: }
  GAP production                             LINE   crates/lake/src/schema.rs:126: return Err(LakeError::MissingColumn { name: s.name });
  GAP production                             LINE   crates/lake/src/schema.rs:294: Some(other) => Some(format!("{other:?}")),
  GAP production                             BRANCH crates/lake/src/schema.rs:125:12 True: if i >= n {
  GAP production                             LINE   crates/lake/src/zstd_v082_frames.rs:175: other => panic!("no synthetic input called {other}"),
  GAP production                             LINE   crates/lake/src/zstd_v082_frames.rs:214: 1 => 2,
  GAP production                             LINE   crates/lake/src/zstd_v082_frames.rs:215: 2 => 4,
  GAP production                             LINE   crates/lake/src/zstd_v082_frames.rs:216: _ => 8,
  GAP production                             BRANCH crates/lake/src/zstd_v082_frames.rs:234:8 False: if checksum {
  F? panic! (failing arm)                    LINE   crates/lake/tests/events.rs:159: _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/events.rs:366: panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/events.rs:401: panic!(
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:57: fn lake_path(rel: &str) -> Option<PathBuf> {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:58: let home = std::env::var_os("HOME")?;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:59: let p = Path::new(&home).join(rel);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:60: p.exists().then_some(p)
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:61: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:64: fn skipped(what: &str) {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:65: println!("SKIPPING {what}: ~/.brutex/lake is not on this machine.");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:66: println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:67: println!("  this repository — so this test proves nothing here. The decode");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:68: println!("  path is covered by tests/synthetic.rs, which writes its own.");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:69: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:73: fn the_real_fno_sample_decodes_to_the_values_it_holds() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:74: let Some(path) = lake_path(SAMPLE_FNO) else {
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake.rs:75: panic!(
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:83: let f = LakeFile::open(&path).expect("the real sample file must open");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:84: assert_eq!(f.layout(), Layout::Fno, "17 columns with greeks");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:85: assert_eq!(f.created_by(), Some("Polars"));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:86: assert_eq!(f.num_rows(), 2_480);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:87: assert_eq!(f.row_groups(), 1);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:89: let b = f.read_row_group(0).expect("the ZSTD pages must decode");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:90: assert_eq!(b.len(), 2_480);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:94: let r = b.row(0).expect("row 0");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:95: assert_eq!(r.timestamp_micros, 1_584_685_680_000_000);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:98: assert_eq!(r.open.raw(), 4_950);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:99: assert_eq!(r.high.raw(), 4_950);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:100: assert_eq!(r.low.raw(), 4_950);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:101: assert_eq!(r.close.raw(), 4_950);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:102: assert_eq!(r.volume, 76);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:103: assert_eq!(r.open_interest(), Some(75));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:106: assert_eq!(r.spot_at_bar.expect("spot").raw(), 847_310);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:107: assert_eq!(r.greeks_provenance_id, Some(1_380_610));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:110: let g = r.greeks.expect("row 0 has greeks");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:111: assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:112: assert_eq!(g.delta.to_bits(), 0.103_791_597_602_742_98_f64.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:113: assert_eq!(
  F? assert_eq! message argument             LINE   crates/lake/tests/real_lake.rs:114: g.gamma.to_bits(),
  F? assert_eq! message argument             LINE   crates/lake/tests/real_lake.rs:115: 0.000_171_426_804_295_494_02_f64.to_bits()
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:117: assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:118: assert_eq!(g.vega.to_bits(), 2.791_593_074_801_929_3_f64.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:119: assert_eq!(g.rho.to_bits(), 0.276_837_206_382_207_27_f64.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:120: assert_eq!(
  F? assert_eq! message argument             LINE   crates/lake/tests/real_lake.rs:121: g.t_years_used.to_bits(),
  F? assert_eq! message argument             LINE   crates/lake/tests/real_lake.rs:122: 0.033_280_060_882_800_61_f64.to_bits()
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:124: assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:127: assert_eq!(b.iter().count(), 2_480);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:128: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:132: fn the_real_cash_sample_has_the_seven_column_layout() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:133: let Some(path) = lake_path(SAMPLE_CASH) else {
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake.rs:134: panic!(
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:142: let f = LakeFile::open(&path).expect("open");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:143: assert_eq!(f.layout(), Layout::Cash, "no greeks in a cash file");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:144: assert_eq!(f.num_rows(), 2_143);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:146: let b = f.read_row_group(0).expect("decode");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:147: let r = b.row(0).expect("row 0");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:148: assert_eq!(r.timestamp_micros, 1_674_445_500_000_000);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:149: assert_eq!(r.open.raw(), 195_000); // 1950.00
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:150: assert_eq!(r.high.raw(), 195_000);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:151: assert_eq!(r.low.raw(), 193_200); // 1932.00
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:152: assert_eq!(r.close.raw(), 194_300); // 1943.00
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:153: assert_eq!(r.volume, 2_663);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:156: assert_eq!(r.open_interest(), None);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:157: assert_eq!(r.open_interest, i64::MIN);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:160: assert!(!r.has_greeks());
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:161: assert_eq!(r.spot_at_bar, None);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:162: assert_eq!(r.greeks_provenance_id, None);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:163: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:167: fn a_real_contract_directory_name_parses_and_round_trips() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:168: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:169: skipped("the contract directory walk");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:170: return;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:173: let mut seen = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:174: let mut futures = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:175: let entries = std::fs::read_dir(&root).expect("read the F&O directory");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:176: for e in entries.take(500) {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:177: let e = e.expect("entry");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:178: let name = e.file_name();
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:179: let name = name.to_str().expect("a lake name is utf-8");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:181: let parsed = ContractName::parse(name)
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake.rs:182: .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:183: assert_eq!(
  F? assert_eq! message argument             LINE   crates/lake/tests/real_lake.rs:184: parsed.to_string(),
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:188: if parsed.kind() == ContractKind::Future {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:189: futures += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:190: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:191: seen += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:193: assert!(seen > 0, "the F&O directory must not be empty");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:194: println!("parsed {seen} real contract names ({futures} futures), all round-tripped");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:195: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:199: fn a_spread_of_real_files_decodes_with_no_failures() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:200: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:201: skipped("the multi-file decode");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:202: return;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:205: let mut files = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:206: let mut rows = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:207: let mut null_oi = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:208: let mut real_oi = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:209: let mut with_greeks = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:211: let dirs = std::fs::read_dir(&root).expect("read dir");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:212: for d in dirs.take(40) {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:213: let d = d.expect("entry").path();
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:214: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:215: continue;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:217: for y in years {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:218: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:219: continue;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:221: for m in months {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:222: let p = m.expect("month").path();
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:223: let f = LakeFile::open(&p)
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake.rs:224: .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:225: for g in 0..f.row_groups() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:226: let b = f
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:227: .read_row_group(g)
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake.rs:228: .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:229: for bar in b.iter() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:230: rows += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:231: if bar.open_interest().is_none() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:232: null_oi += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:233: } else {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:234: real_oi += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:235: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:236: if bar.has_greeks() {
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:237: with_greeks += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:238: }
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:241: files += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:246: assert!(files > 0, "at least one real file must have been read");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:247: println!("decoded {files} real files, {rows} rows, 0 failures");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:248: println!("  open interest: {real_oi} real, {null_oi} null");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:249: println!("  rows carrying greeks: {with_greeks}");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:253: assert!(rows > 0);
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:254: assert!(null_oi > 0, "the sample must contain a null open interest");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:255: assert!(real_oi > 0, "and a real one");
  GAP test code                              LINE   crates/lake/tests/real_lake.rs:256: }
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:74:9 True: let Some(path) = lake_path(SAMPLE_FNO) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:74:9 False: let Some(path) = lake_path(SAMPLE_FNO) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:133:9 True: let Some(path) = lake_path(SAMPLE_CASH) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:133:9 False: let Some(path) = lake_path(SAMPLE_CASH) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:168:9 True: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:168:9 False: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:188:12 True: if parsed.kind() == ContractKind::Future {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:188:12 False: if parsed.kind() == ContractKind::Future {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:200:9 True: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:200:9 False: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:214:13 True: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:214:13 False: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:218:17 True: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake.rs:218:17 False: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:231:28 True: if bar.open_interest().is_none() {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:231:28 False: if bar.open_interest().is_none() {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:236:28 True: if bar.has_greeks() {
  GAP test code                              BRANCH crates/lake/tests/real_lake.rs:236:28 False: if bar.has_greeks() {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:57: fn eat(mut h: u64, v: u64) -> u64 {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:58: for b in v.to_le_bytes() {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:59: h ^= u64::from(b);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:60: h = h.wrapping_mul(FNV_PRIME);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:61: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:62: h
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:63: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:66: fn eat_opt(h: u64, v: Option<u64>) -> u64 {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:67: match v {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:68: Some(v) => eat(eat(h, 1), v),
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:69: None => eat(h, 0),
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:71: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:78: fn digest_file(path: &Path) -> Result<(u64, usize, Layout), String> {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:79: let file = LakeFile::open(path).map_err(|e| e.to_string())?;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:80: let layout = file.layout();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:81: let mut h = eat(FNV_OFFSET, u64::from(u8::from(layout.has_greeks())));
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:82: let mut rows = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:84: for g in 0..file.row_groups() {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:85: let batch = file
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:86: .read_row_group(g)
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:87: .map_err(|e| format!("group {g}: {e}"))?;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:88: for bar in batch.iter() {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:94: h = eat(h, bar.timestamp_micros as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:95: h = eat(h, bar.open.raw() as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:96: h = eat(h, bar.high.raw() as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:97: h = eat(h, bar.low.raw() as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:98: h = eat(h, bar.close.raw() as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:99: h = eat(h, bar.volume as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:102: h = eat(h, bar.open_interest as u64);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:103: h = eat_opt(h, bar.spot_at_bar.map(|p| p.raw() as u64));
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:104: h = eat_opt(h, bar.greeks_provenance_id.map(|v| i64::from(v) as u64));
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:106: match bar.greeks {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:107: Some(g) => {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:108: h = eat(h, 1);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:109: for v in [
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:110: g.iv,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:111: g.delta,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:112: g.gamma,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:113: g.theta,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:114: g.vega,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:115: g.rho,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:116: g.t_years_used,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:117: g.rate_used,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:118: ] {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:119: h = eat(h, v.to_bits());
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:120: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:122: None => h = eat(h, 0),
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:124: rows += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:127: Ok((h, rows, layout))
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:128: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:131: fn month_files(contract: &Path, into: &mut Vec<PathBuf>) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:132: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:133: return;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:135: let mut ys: Vec<PathBuf> = years.flatten().map(|e| e.path()).collect();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:136: ys.sort();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:137: for y in ys {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:138: let Ok(months) = std::fs::read_dir(&y) else {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:139: continue;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:141: let mut ms: Vec<PathBuf> = months.flatten().map(|e| e.path()).collect();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:142: ms.sort();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:143: into.extend(ms);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:145: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:149: fn sample_dirs(root: &Path, want: usize) -> Vec<PathBuf> {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:150: let Ok(entries) = std::fs::read_dir(root) else {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:151: return Vec::new();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:153: let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:154: all.sort();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:155: if all.is_empty() || want == 0 {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:156: return Vec::new();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:157: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:158: let stride = (all.len() / want).max(1);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:159: all.into_iter().step_by(stride).take(want).collect()
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:160: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:164: fn a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest() {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:165: let Some(home) = std::env::var_os("HOME") else {
  F? panic! (failing arm)                    LINE   crates/lake/tests/real_lake_regression.rs:166: panic!(
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:170: let root = Path::new(&home).join(".brutex/lake/bars/NSE");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:171: assert!(
  F? assert! message argument                LINE   crates/lake/tests/real_lake_regression.rs:172: root.exists(),
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:176: let mut targets: Vec<PathBuf> = Vec::new();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:177: for contract in sample_dirs(&root.join("FNO"), 120) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:178: month_files(&contract, &mut targets);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:179: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:180: for symbol in sample_dirs(&root.join("CASH"), 20) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:181: month_files(&symbol, &mut targets);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:182: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:183: for symbol in sample_dirs(&root.join("INDEX"), 5) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:184: month_files(&symbol, &mut targets);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:185: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:186: targets.sort();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:188: let mut refused: Vec<String> = Vec::new();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:189: let mut files = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:190: let mut rows = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:191: let mut fno = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:192: let mut cash = 0_usize;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:193: let mut whole = FNV_OFFSET;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:195: for path in &targets {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:196: let shown = path.strip_prefix(&root).unwrap_or(path).display();
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:197: match digest_file(path) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:198: Ok((h, n, layout)) => {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:199: println!("{h:016x} {n:>6} {shown}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:200: whole = eat(whole, h);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:201: files += 1;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:202: rows += n;
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:203: match layout {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:204: Layout::Fno => fno += 1,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:205: Layout::Cash => cash += 1,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:208: Err(e) => refused.push(format!("{shown}: {e}")),
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:212: println!("REAL LAKE DIGEST");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:213: println!("  files    : {files}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:214: println!("  rows     : {rows}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:215: println!("  F&O      : {fno}, cash/index: {cash}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:216: println!("  refusals : {}", refused.len());
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:217: println!("  WHOLE    : {whole:016x}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:218: for line in refused.iter().take(20) {
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:219: println!("    REFUSED {line}");
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:220: }
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:222: assert!(
  F? assert! message argument                LINE   crates/lake/tests/real_lake_regression.rs:223: refused.is_empty(),
  F? assert! message argument                LINE   crates/lake/tests/real_lake_regression.rs:225: refused.join("\n  ")
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:227: assert!(
  F? assert! message argument                LINE   crates/lake/tests/real_lake_regression.rs:228: files >= MINIMUM_FILES,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:231: assert!(
  F? assert! message argument                LINE   crates/lake/tests/real_lake_regression.rs:232: fno > 0 && cash > 0,
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:235: assert!(rows > 0);
  GAP test code                              LINE   crates/lake/tests/real_lake_regression.rs:236: }
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:132:9 True: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:132:9 False: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:138:13 True: let Ok(months) = std::fs::read_dir(&y) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:138:13 False: let Ok(months) = std::fs::read_dir(&y) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:150:9 True: let Ok(entries) = std::fs::read_dir(root) else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:150:9 False: let Ok(entries) = std::fs::read_dir(root) else {
  GAP test code                              BRANCH crates/lake/tests/real_lake_regression.rs:155:8 True: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/real_lake_regression.rs:155:8 False: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/real_lake_regression.rs:155:26 True: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/real_lake_regression.rs:155:26 False: if all.is_empty() || want == 0 {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:165:9 True: let Some(home) = std::env::var_os("HOME") else {
  F? let-else in test                        BRANCH crates/lake/tests/real_lake_regression.rs:165:9 False: let Some(home) = std::env::var_os("HOME") else {
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/real_lake_regression.rs:232:9 True: fno > 0 && cash > 0,
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/real_lake_regression.rs:232:9 False: fno > 0 && cash > 0,
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/real_lake_regression.rs:232:20 True: fno > 0 && cash > 0,
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/real_lake_regression.rs:232:20 False: fno > 0 && cash > 0,
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:176: _ => panic!("the cash layout has no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:234: _ => panic!("the cash layout has no other physical type"),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:310: Ok(batch) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:311: let got: Vec<Option<i64>> = (0..batch.len())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:312: .map(|i| batch.row(i).expect("row in range").open_interest())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:313: .collect();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:314: let real = got.iter().filter(|v| v.is_some()).count();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:315: let invented = got.len() - real;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:316: let first = got.iter().position(Option::is_none);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:317: let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:318: panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:326: batch.len()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:375: Ok(batch) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:376: let got: Vec<Option<i64>> = (0..batch.len())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:377: .map(|i| batch.row(i).expect("row in range").open_interest())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:378: .collect();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:379: let real = got.iter().filter(|v| v.is_some()).count();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:380: let first = got.iter().position(Option::is_none);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:381: fabricated.push(format!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:384: batch.len(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:385: got.len() - real,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:386: first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
  GAP test code                              LINE   crates/lake/tests/refusals.rs:405: continue;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:408: continue;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:427: Ok(batch) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:428: let got: Vec<Option<i64>> = (0..batch.len())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:429: .map(|i| batch.row(i).expect("row in range").open_interest())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:430: .collect();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:431: let real = got.iter().filter(|v| v.is_some()).count();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:432: let first = got.iter().position(Option::is_none);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:433: fabricated.push(format!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:437: batch.len(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:438: got.len() - real,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:439: first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
  F? assert! message argument                LINE   crates/lake/tests/refusals.rs:454: fabricated.join("\n  ")
  GAP test code                              LINE   crates/lake/tests/refusals.rs:494: return None;
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:556: _ => panic!("one INT64 column"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:680: _ => panic!("the cash layout has no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:701: Err(other) => panic!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:706: Ok(file) => match file.read_row_group(0) {
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:707: Err(e) => panic!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:711: Ok(batch) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:712: let got: Vec<Option<i64>> = (0..batch.len())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:713: .map(|i| batch.row(i).expect("row in range").open_interest())
  GAP test code                              LINE   crates/lake/tests/refusals.rs:714: .collect();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:715: assert_eq!(
  F? assert_eq! message argument             LINE   crates/lake/tests/refusals.rs:717: vec![Some(7), Some(8), Some(9)],
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:800: _ => panic!("the cash layout has no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:819: other => panic!("a {what} leaf must be refused by name, got {other:?}"),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:858: fn scan_file(path: &std::path::Path, year: &str, out: &mut Scan) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:859: let raw = match std::fs::read(path) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:860: Ok(r) => r,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:861: Err(e) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:862: out.unreadable.push(format!("{}: {e}", path.display()));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:863: return;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:866: let bytes = bytes::Bytes::from(raw);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:867: let meta = match parquet::file::metadata::ParquetMetaDataReader::new().parse_and_finish(&bytes)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:869: Ok(m) => m,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:870: Err(e) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:871: out.unreadable.push(format!("{}: {e}", path.display()));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:872: return;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:875: let schema = meta.file_metadata().schema_descr();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:876: let leaves = schema.num_columns();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:877: out.files += 1;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:878: out.years.insert(year.to_owned());
  GAP test code                              LINE   crates/lake/tests/refusals.rs:879: match leaves {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:880: 17 => out.fno += 1,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:881: 7 => out.cash += 1,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:882: _ => {}
  GAP test code                              LINE   crates/lake/tests/refusals.rs:885: for leaf in 0..leaves {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:886: let col = schema.column(leaf);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:887: if col.max_def_level() != 1 || col.max_rep_level() != 0 {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:888: out.nested.push(format!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:889: "{}: leaf `{}` max_def_level={} max_rep_level={}",
  GAP test code                              LINE   crates/lake/tests/refusals.rs:890: path.display(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:891: col.name(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:892: col.max_def_level(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:893: col.max_rep_level()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:894: ));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:895: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:898: for g in 0..meta.num_row_groups() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:899: let group = meta.row_group(g);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:900: let rows = group.num_rows();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:901: out.row_groups += 1;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:902: out.rows += rows;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:903: for leaf in 0..leaves {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:904: let chunk = group.column(leaf);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:905: let name = schema.column(leaf).name().to_owned();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:906: if chunk.num_values() != rows {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:907: out.declared_short.push(format!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:908: "{} group {g} column `{name}`: declared num_values={} against num_rows={rows}",
  GAP test code                              LINE   crates/lake/tests/refusals.rs:909: path.display(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:910: chunk.num_values()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:911: ));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:912: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:913: match page_values(&bytes, chunk) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:914: Ok(delivered) if delivered != rows => out.pages_short.push(format!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:917: path.display()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:919: Ok(_) => {}
  GAP test code                              LINE   crates/lake/tests/refusals.rs:920: Err(e) => out
  GAP test code                              LINE   crates/lake/tests/refusals.rs:921: .unreadable
  GAP test code                              LINE   crates/lake/tests/refusals.rs:922: .push(format!("{} group {g} column `{name}`: {e}", path.display())),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:926: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:932: fn page_values(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:933: bytes: &[u8],
  GAP test code                              LINE   crates/lake/tests/refusals.rs:934: chunk: &parquet::file::metadata::ColumnChunkMetaData,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:935: ) -> Result<i64, String> {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:936: let start = chunk
  GAP test code                              LINE   crates/lake/tests/refusals.rs:937: .dictionary_page_offset()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:938: .unwrap_or_else(|| chunk.data_page_offset());
  GAP test code                              LINE   crates/lake/tests/refusals.rs:939: let from = usize::try_from(start).map_err(|_| format!("offset {start} is not a position"))?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:940: let len = chunk.compressed_size();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:941: let size = usize::try_from(len).map_err(|_| format!("length {len} is not a length"))?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:942: let to = from.checked_add(size).ok_or("chunk extent overflows")?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:943: let slice = bytes
  GAP test code                              LINE   crates/lake/tests/refusals.rs:944: .get(from..to)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:945: .ok_or_else(|| format!("chunk {from}..{to} runs past the file end {}", bytes.len()))?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:947: let mut at = 0_usize;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:948: let mut total = 0_i64;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:950: let rest = match slice.get(at..) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:951: Some(r) if !r.is_empty() => r,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:952: _ => return Ok(total),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:954: let mut cursor = Cursor::new(rest);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:955: let header = {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:956: let mut proto = TCompactInputProtocol::new(&mut cursor, rest.len());
  GAP test code                              LINE   crates/lake/tests/refusals.rs:957: parquet_format_safe::PageHeader::read_from_in_protocol(&mut proto)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:958: .map_err(|e| format!("page header at {at}: {e}"))?
  GAP test code                              LINE   crates/lake/tests/refusals.rs:960: let header_len = usize::try_from(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:961: std::io::Seek::stream_position(&mut cursor).map_err(|e| e.to_string())?,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:963: .map_err(|e: std::num::TryFromIntError| e.to_string())?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:964: if let Some(d) = header.data_page_header.as_ref() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:965: total += i64::from(d.num_values);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:966: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:967: if let Some(d) = header.data_page_header_v2.as_ref() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:968: total += i64::from(d.num_values);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:969: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:970: let body = usize::try_from(header.compressed_page_size)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:971: .map_err(|_| format!("page size {} is not a length", header.compressed_page_size))?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:972: at = at
  GAP test code                              LINE   crates/lake/tests/refusals.rs:973: .checked_add(header_len)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:974: .and_then(|v| v.checked_add(body))
  GAP test code                              LINE   crates/lake/tests/refusals.rs:975: .ok_or("page offset overflows")?;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:977: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:980: fn month_files(contract: &std::path::Path, into: &mut Vec<(std::path::PathBuf, String)>) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:981: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:982: return;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:984: for y in years.flatten() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:985: let year = y.file_name().to_string_lossy().into_owned();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:986: let Ok(months) = std::fs::read_dir(y.path()) else {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:987: continue;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:989: for m in months.flatten() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:990: into.push((m.path(), year.clone()));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:991: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:993: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:996: fn sample_dirs(root: &std::path::Path, want: usize) -> Vec<std::path::PathBuf> {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:997: let Ok(entries) = std::fs::read_dir(root) else {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:998: return Vec::new();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1000: let all: Vec<std::path::PathBuf> = entries.flatten().map(|e| e.path()).collect();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1001: if all.is_empty() || want == 0 {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1002: return Vec::new();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1003: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1004: let stride = (all.len() / want).max(1);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1005: all.into_iter().step_by(stride).take(want).collect()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1006: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1010: fn no_real_lake_file_triggers_either_defect() {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1011: let Some(home) = std::env::var_os("HOME") else {
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1012: panic!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1016: let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1017: assert!(
  F? assert! message argument                LINE   crates/lake/tests/refusals.rs:1018: root.exists(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1022: let mut targets: Vec<(std::path::PathBuf, String)> = Vec::new();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1023: for contract in sample_dirs(&root.join("FNO"), 400) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1024: month_files(&contract, &mut targets);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1025: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1026: for symbol in sample_dirs(&root.join("CASH"), 30) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1027: month_files(&symbol, &mut targets);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1028: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1029: for symbol in sample_dirs(&root.join("INDEX"), 8) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1030: month_files(&symbol, &mut targets);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1031: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1033: let mut scan = Scan::default();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1034: for (path, year) in &targets {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1035: scan_file(path, year, &mut scan);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1036: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1038: println!("scanned {} real lake files", scan.files);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1039: println!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1043: println!("  row groups: {}, rows: {}", scan.row_groups, scan.rows);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1044: println!("  years covered: {:?}", scan.years);
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1045: println!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1047: scan.declared_short.len()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1049: println!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1051: scan.pages_short.len()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1053: println!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1055: scan.nested.len()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1057: println!(
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1059: scan.unreadable.len()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1061: for line in scan.declared_short.iter().take(10) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1062: println!("    DECLARED SHORT {line}");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1063: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1064: for line in scan.pages_short.iter().take(10) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1065: println!("    PAGES SHORT {line}");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1066: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1067: for line in scan.nested.iter().take(10) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1068: println!("    NESTED {line}");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1069: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1070: for line in scan.unreadable.iter().take(10) {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1071: println!("    UNREADABLE {line}");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1072: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1074: assert!(
  F? assert! message argument                LINE   crates/lake/tests/refusals.rs:1075: scan.files >= 200,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1079: assert!(
  F? assert! message argument                LINE   crates/lake/tests/refusals.rs:1080: scan.years.len() >= 3,
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1084: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1085: assert!(
  F? assert! message argument                LINE   crates/lake/tests/refusals.rs:1086: scan.declared_short.is_empty(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1089: assert!(scan.pages_short.is_empty(), "defect 1 fires on real data");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1090: assert!(scan.nested.is_empty(), "defect 3 fires on real data");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1091: assert!(scan.unreadable.is_empty(), "a real file would not parse");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1092: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1141: return;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1159: }
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1161: let mut checked = 0_usize;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1162: for entry in std::fs::read_dir(&root)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1163: .expect("read the F&O directory")
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1164: .flatten()
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1165: .take(20_000)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1167: let name = entry.file_name();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1168: let name = name.to_string_lossy();
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1169: let parsed = ContractName::parse(&name)
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1170: .unwrap_or_else(|e| panic!("real lake directory {name} was refused: {e}"));
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1171: assert_eq!(
  F? assert_eq! message argument             LINE   crates/lake/tests/refusals.rs:1172: parsed.to_string(),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1176: checked += 1;
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1178: println!("{checked} real contract directories round-tripped byte for byte");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1204: Ok(other) => {
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1205: panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1215: other.to_string() == padded,
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1216: other == good,
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1217: other.key() == good.key()
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1345: _ => panic!("the cash layout has no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1487: _ => panic!("the cash layout has no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1516: Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1517: Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1590: ColumnWriter::Int32ColumnWriter(typed) => {
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1591: typed
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1592: .write_batch(&[1_i32, 2, 3], Some(&[1, 1, 1]), None)
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1593: .expect("i32");
  GAP test code                              LINE   crates/lake/tests/refusals.rs:1594: }
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1595: _ => panic!("this fixture writes no other physical type"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1626: Err(other) => panic!("refused as the wrong thing: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1627: Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1645: Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1646: Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1696: Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1697: Ok(_) => panic!("an INT96 timestamp opened"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1747: Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/lake/tests/refusals.rs:1748: Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:404:13 False: let Some(len) = exact_page_prefix(&sound, OI, pages) else {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:407:12 True: if len >= full {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:493:12 True: if rest.is_empty() {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:887:12 True: if col.max_def_level() != 1 || col.max_rep_level() != 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:887:12 False: if col.max_def_level() != 1 || col.max_rep_level() != 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:887:40 True: if col.max_def_level() != 1 || col.max_rep_level() != 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:887:40 False: if col.max_def_level() != 1 || col.max_rep_level() != 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:906:16 True: if chunk.num_values() != rows {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:906:16 False: if chunk.num_values() != rows {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:914:34 True: Ok(delivered) if delivered != rows => out.pages_short.push(format!(
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:914:34 False: Ok(delivered) if delivered != rows => out.pages_short.push(format!(
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:951:24 True: Some(r) if !r.is_empty() => r,
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:951:24 False: Some(r) if !r.is_empty() => r,
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:964:16 True: if let Some(d) = header.data_page_header.as_ref() {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:964:16 False: if let Some(d) = header.data_page_header.as_ref() {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:967:16 True: if let Some(d) = header.data_page_header_v2.as_ref() {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:967:16 False: if let Some(d) = header.data_page_header_v2.as_ref() {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:981:9 True: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:981:9 False: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:986:13 True: let Ok(months) = std::fs::read_dir(y.path()) else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:986:13 False: let Ok(months) = std::fs::read_dir(y.path()) else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:997:9 True: let Ok(entries) = std::fs::read_dir(root) else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:997:9 False: let Ok(entries) = std::fs::read_dir(root) else {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:1001:8 True: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:1001:8 False: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:1001:26 True: if all.is_empty() || want == 0 {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:1001:26 False: if all.is_empty() || want == 0 {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:1011:9 True: let Some(home) = std::env::var_os("HOME") else {
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:1011:9 False: let Some(home) = std::env::var_os("HOME") else {
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1084:13 True: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1084:13 False: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1084:29 True: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1084:29 False: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
  F? let-else in test                        BRANCH crates/lake/tests/refusals.rs:1140:9 False: let Some(home) = std::env::var_os("HOME") else {
  GAP test code                              BRANCH crates/lake/tests/refusals.rs:1155:8 False: if !root.exists() {
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1265:9 False: text.contains("400") && text.contains("2480"),
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1265:33 False: text.contains("400") && text.contains("2480"),
  F? assert! condition branch (failing side) BRANCH crates/lake/tests/refusals.rs:1394:71 False: Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:136: _ => panic!("writer type does not match the column"),
  F? assert! message argument                LINE   crates/lake/tests/synthetic.rs:231: f.created_by()
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:353: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:355: other = other.err()
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:403: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:405: other = other.err()
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:453: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:455: other = other.err()
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:473: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:475: other = other.err()
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:626: other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:639: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:641: other = other.err()
  F? assert! message argument                LINE   crates/lake/tests/synthetic.rs:662: matches!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:780: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:782: other.map(|b| b.len())
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:820: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:822: other.map(|b| b.len())
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:952: other => panic!("page type {} is not one this writer emits", other.0),
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:1156: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:1158: other.map(|groups| groups.len())
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:1223: other => panic!(
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:1225: other.map(|b| b.len())
  F? panic! (failing arm)                    LINE   crates/lake/tests/synthetic.rs:1311: Ok(rows) => panic!("decoded {} rows from a broken page", rows.len()),
  GAP test code                              BRANCH crates/lake/tests/synthetic.rs:993:12 False: if group.file_offset.is_some() {
  GAP test code                              BRANCH crates/lake/tests/synthetic.rs:996:12 False: if group.total_compressed_size.is_some() {
  GAP test code                              BRANCH crates/lake/tests/synthetic.rs:1283:16 False: if chunk.dictionary_page_offset.is_some() {
  GAP test code                              BRANCH crates/lake/tests/synthetic.rs:1292:12 False: if group.file_offset.is_some() {
  GAP test code                              BRANCH crates/lake/tests/synthetic.rs:1295:12 False: if group.total_compressed_size.is_some() {
  GAP production                             LINE   crates/store/src/catalog.rs:320: return Err(CatalogError::BarsUnreadable {
  GAP production                             LINE   crates/store/src/catalog.rs:321: because: because.to_string(),
  GAP production                             LINE   crates/store/src/catalog.rs:322: });
  GAP production                             LINE   crates/store/src/catalog.rs:436: out.census.wrong_depth = out.census.wrong_depth.saturating_add(1);
  GAP production                             LINE   crates/store/src/catalog.rs:437: return;
  GAP production                             LINE   crates/store/src/catalog.rs:475: out.census.wrong_depth = out.census.wrong_depth.saturating_add(1);
  GAP production                             LINE   crates/store/src/catalog.rs:476: return;
  GAP production                             BRANCH crates/store/src/catalog.rs:319:20 True: if dir == bars {
  GAP production                             BRANCH crates/store/src/catalog.rs:435:9 False: let Ok(rel) = path.strip_prefix(bars) else {
  GAP production                             BRANCH crates/store/src/catalog.rs:467:9 False: let (Some(v), Some(exchange), Some(segment), Some(symbol), Some(rung), Some(file)) = (
  F? panic! (failing arm)                    LINE   crates/store/src/catalog_tests.rs:268: panic!("a dangling `bars` link is refused");
  F? panic! (failing arm)                    LINE   crates/store/src/catalog_tests.rs:282: panic!("an unsearchable root is refused, not empty: {locked:?}");
  F? assert! message argument                LINE   crates/store/src/catalog_tests.rs:460: path.display()
  F? let-else in test                        BRANCH crates/store/src/catalog_tests.rs:267:9 False: let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
  F? let-else in test                        BRANCH crates/store/src/catalog_tests.rs:281:9 False: let Err(CatalogError::BarsUnreadable { because }) = locked else {
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:314:13 False: assert!(stack.is_empty() && out.held.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:314:33 False: assert!(stack.is_empty() && out.held.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:320:13 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:320:32 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:320:50 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:336:13 False: assert!(stack.is_empty() && out.held.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/src/catalog_tests.rs:336:33 False: assert!(stack.is_empty() && out.held.is_empty());
  GAP production                             BRANCH crates/store/src/checksum_audit.rs:286:8 True: if !header.checksums_present()
  GAP production                             BRANCH crates/store/src/checksum_audit.rs:288:12 True: || layout.header_len() != HEADER_BYTES as u64
  GAP production                             BRANCH crates/store/src/checksum_audit.rs:289:12 True: || layout.block_len() > BUFFER_BYTES as u64
  GAP test code                              LINE   crates/store/src/checksum_audit_tests.rs:153: child.kill().expect("bounded timeout");
  GAP test code                              LINE   crates/store/src/checksum_audit_tests.rs:154: child.wait().expect("reap child");
  GAP test code                              LINE   crates/store/src/checksum_audit_tests.rs:155: return Err("strict checksum source open waited for a FIFO peer".into());
  GAP test code                              BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True: if started.elapsed() > std::time::Duration::from_secs(2) {
  GAP production                             LINE   crates/store/src/emits.rs:835: found
  GAP production                             LINE   crates/store/src/emits.rs:836: .records
  GAP production                             LINE   crates/store/src/emits.rs:837: .iter()
  GAP production                             LINE   crates/store/src/emits.rs:838: .map(|line| (line.target.clone(), line.message.clone()))
  GAP production                             LINE   crates/store/src/emits.rs:839: .collect::<Vec<_>>()
  GAP production                             LINE   crates/store/src/emits.rs:848: panic!(
  GAP production                             BRANCH crates/store/src/emits.rs:311:12 False: if let Some(byte) = region.get_mut(16_384 + 56) {
  GAP production                             LINE   crates/store/src/file.rs:944: match self {
  GAP production                             LINE   crates/store/src/file.rs:946: Self::Disagrees(Conflict::Restated) => "restated",
  GAP production                             LINE   crates/store/src/file.rs:947: Self::Disagrees(Conflict::NotHeld) => "not_held",
  GAP production                             LINE   crates/store/src/file.rs:948: Self::Disagrees(Conflict::Skipped) => "skipped",
  GAP production                             LINE   crates/store/src/file.rs:1360: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:1367: )?;
  GAP production                             LINE   crates/store/src/file.rs:1370: SealedOffer::Disagrees(conflict) => write!(f, "{conflict} {ts_micros}")?,
  GAP production                             LINE   crates/store/src/file.rs:1388: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:2869: )?;
  GAP production                             LINE   crates/store/src/file.rs:2875: )?;
  GAP production                             LINE   crates/store/src/file.rs:3151: Some(other) => return Err(other),
  GAP production                             LINE   crates/store/src/file.rs:3156: return Err(StoreError::Format {
  GAP production                             LINE   crates/store/src/file.rs:3157: path: self.bars_path.clone(),
  GAP production                             LINE   crates/store/src/file.rs:3158: source,
  GAP production                             LINE   crates/store/src/file.rs:3159: });
  GAP production                             LINE   crates/store/src/file.rs:3166: Err(source) => {
  GAP production                             LINE   crates/store/src/file.rs:3167: return Err(StoreError::Format {
  GAP production                             LINE   crates/store/src/file.rs:3168: path: self.bars_path.clone(),
  GAP production                             LINE   crates/store/src/file.rs:3169: source,
  GAP production                             LINE   crates/store/src/file.rs:3170: });
  GAP production                             LINE   crates/store/src/file.rs:3840: return Ok(None);
  GAP production                             LINE   crates/store/src/file.rs:3854: return Ok(None);
  GAP production                             LINE   crates/store/src/file.rs:3865: Err(Why::OffGrid { index, ts_micros }) => Err(StoreError::OffGrid {
  GAP production                             LINE   crates/store/src/file.rs:3866: at: index.saturating_sub(held.n_valid),
  GAP production                             LINE   crates/store/src/file.rs:3867: ts_micros,
  GAP production                             LINE   crates/store/src/file.rs:3868: timeframe_secs: self.header.timeframe_secs,
  GAP production                             LINE   crates/store/src/file.rs:3869: }),
  GAP production                             LINE   crates/store/src/file.rs:3870: Err(Why::Outside { index, ts_micros }) => Err(StoreError::OutsideMonth {
  GAP production                             LINE   crates/store/src/file.rs:3871: at: index.saturating_sub(held.n_valid),
  GAP production                             LINE   crates/store/src/file.rs:3872: ts_micros,
  GAP production                             LINE   crates/store/src/file.rs:3873: month: self.admission.month,
  GAP production                             LINE   crates/store/src/file.rs:3874: }),
  GAP production                             LINE   crates/store/src/file.rs:3875: Err(other) => Err(index_refused(&tix.path, &other)),
  GAP production                             LINE   crates/store/src/file.rs:3958: return Ok(None);
  GAP production                             LINE   crates/store/src/file.rs:3978: Err(why @ Why::SharedSlot { .. }) => {
  GAP production                             LINE   crates/store/src/file.rs:3979: let path = tix.path.clone();
  GAP production                             LINE   crates/store/src/file.rs:3980: self.set_tix_state(bisecting(&path, why));
  GAP production                             LINE   crates/store/src/file.rs:3981: return Ok(None);
  GAP production                             LINE   crates/store/src/file.rs:3983: Err(Why::OffGrid { index, ts_micros }) => {
  GAP production                             LINE   crates/store/src/file.rs:3984: return Err(StoreError::OffGrid {
  GAP production                             LINE   crates/store/src/file.rs:3985: at: index.saturating_sub(held.n_valid),
  GAP production                             LINE   crates/store/src/file.rs:3986: ts_micros,
  GAP production                             LINE   crates/store/src/file.rs:3987: timeframe_secs: self.header.timeframe_secs,
  GAP production                             LINE   crates/store/src/file.rs:3988: });
  GAP production                             LINE   crates/store/src/file.rs:3990: Err(Why::Outside { index, ts_micros }) => {
  GAP production                             LINE   crates/store/src/file.rs:3991: return Err(StoreError::OutsideMonth {
  GAP production                             LINE   crates/store/src/file.rs:3992: at: index.saturating_sub(held.n_valid),
  GAP production                             LINE   crates/store/src/file.rs:3993: ts_micros,
  GAP production                             LINE   crates/store/src/file.rs:3994: month: self.admission.month,
  GAP production                             LINE   crates/store/src/file.rs:3995: });
  GAP production                             LINE   crates/store/src/file.rs:3997: Err(other) => return Err(index_refused(&tix.path, &other)),
  GAP production                             LINE   crates/store/src/file.rs:4037: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:4048: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:4065: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:4068: return Ok(());
  GAP production                             LINE   crates/store/src/file.rs:4290: )?;
  GAP production                             LINE   crates/store/src/file.rs:4300: path: self.bars_path.clone(),
  GAP production                             LINE   crates/store/src/file.rs:4301: source: FormatError::OffsetOverflow,
  GAP production                             LINE   crates/store/src/file.rs:4302: })?;
  GAP production                             LINE   crates/store/src/file.rs:4436: return Some(conflict(at, row.stamp(), Conflict::NotHeld));
  GAP production                             LINE   crates/store/src/file.rs:4488: return Ok(None);
  GAP production                             LINE   crates/store/src/file.rs:4918: other => StoreError::TimeIndex {
  GAP production                             LINE   crates/store/src/file.rs:4919: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/file.rs:4920: reason: other.to_string(),
  GAP production                             LINE   crates/store/src/file.rs:4921: },
  F? assert! message argument                LINE   crates/store/src/file.rs:6716: parent.display()
  F? assert! message argument                LINE   crates/store/src/file.rs:6920: matches!(
  F? assert_eq! message argument             LINE   crates/store/src/file.rs:6940: level.display()
  F? assert! message argument                LINE   crates/store/src/file.rs:7199: matches!(
  F? panic! (failing arm)                    LINE   crates/store/src/file.rs:7253: other => panic!("the flush was refused by path: {other:?}"),
  F? assert! message argument                LINE   crates/store/src/file.rs:7573: matches!(
  GAP test code                              LINE   crates/store/src/file.rs:7819: fn the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name() {
  GAP test code                              LINE   crates/store/src/file.rs:7820: let dir = std::env::var_os("BRUTEX_FULL_VOLUME")
  GAP test code                              LINE   crates/store/src/file.rs:7821: .map(PathBuf::from)
  GAP test code                              LINE   crates/store/src/file.rs:7822: .expect("BRUTEX_FULL_VOLUME names a directory on a small volume");
  GAP test code                              LINE   crates/store/src/file.rs:7823: let at = dir.join("brutex-enospc-probe.bin");
  GAP test code                              LINE   crates/store/src/file.rs:7824: let file = std::fs::OpenOptions::new()
  GAP test code                              LINE   crates/store/src/file.rs:7825: .write(true)
  GAP test code                              LINE   crates/store/src/file.rs:7826: .create_new(true)
  GAP test code                              LINE   crates/store/src/file.rs:7827: .open(&at)
  GAP test code                              LINE   crates/store/src/file.rs:7828: .expect("a new file on the small volume");
  GAP test code                              LINE   crates/store/src/file.rs:7829: let chunk = vec![0xAB_u8; 1 << 16];
  GAP test code                              LINE   crates/store/src/file.rs:7830: let mut offset = 0_u64;
  GAP test code                              LINE   crates/store/src/file.rs:7831: let refused = loop {
  GAP test code                              LINE   crates/store/src/file.rs:7832: assert!(offset < 1 << 30, "the volume took 1 GiB without refusing");
  GAP test code                              LINE   crates/store/src/file.rs:7833: match write_fully(&file, &at, offset, &chunk) {
  GAP test code                              LINE   crates/store/src/file.rs:7834: Ok(()) => offset = offset.saturating_add(1 << 16),
  GAP test code                              LINE   crates/store/src/file.rs:7835: Err(refused) => break refused,
  GAP test code                              LINE   crates/store/src/file.rs:7838: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/src/file.rs:7840: StoreError::DiskFull {
  F? assert_eq! message argument             LINE   crates/store/src/file.rs:7841: path: at.clone(),
  F? assert_eq! message argument             LINE   crates/store/src/file.rs:7842: action: Action::Write,
  F? assert_eq! message argument             LINE   crates/store/src/file.rs:7843: },
  GAP test code                              LINE   crates/store/src/file.rs:7847: eprintln!("ENOSPC: DiskFull by name after {offset} bytes accepted");
  GAP test code                              LINE   crates/store/src/file.rs:7848: }
  F? assert! message argument                LINE   crates/store/src/file.rs:8337: reads.get()
  GAP test code                              LINE   crates/store/src/file.rs:8660: continue;
  F? panic! (failing arm)                    LINE   crates/store/src/file.rs:8676: other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
  F? assert! message argument                LINE   crates/store/src/file.rs:8894: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:8903: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:8913: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:9015: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:9058: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:9264: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:10209: matches!(
  F? assert! message argument                LINE   crates/store/src/file.rs:10335: matches!(
  GAP production                             BRANCH crates/store/src/file.rs:1351:9 False: let StoreError::Sealed {
  GAP production                             BRANCH crates/store/src/file.rs:1382:9 False: let StoreError::SealedRefused {
  GAP production                             BRANCH crates/store/src/file.rs:3746:16 False: if let Some(tix) = self.tix.as_mut() {
  GAP production                             BRANCH crates/store/src/file.rs:3839:21 False: let Some(tix) = self.tix.as_ref() else {
  GAP production                             BRANCH crates/store/src/file.rs:3853:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP production                             BRANCH crates/store/src/file.rs:3957:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP production                             BRANCH crates/store/src/file.rs:4036:13 False: let Some(path) = self.tix.as_ref().map(|tix| tix.path.clone()) else {
  GAP production                             BRANCH crates/store/src/file.rs:4047:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP production                             BRANCH crates/store/src/file.rs:4064:13 False: let Some(tix) = self.tix.as_ref() else {
  GAP production                             BRANCH crates/store/src/file.rs:4067:13 False: let Some(TixState::Ready(index)) = tix.state.get() else {
  GAP production                             BRANCH crates/store/src/file.rs:4173:36 True: if cache.block != block || cache.n_valid != n_valid {
  GAP production                             BRANCH crates/store/src/file.rs:4435:16 True: if index >= n_valid {
  GAP production                             BRANCH crates/store/src/file.rs:4487:12 True: if self.header.n_valid == 0 {
  GAP production                             BRANCH crates/store/src/file.rs:5650:42 True: BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
  GAP production                             BRANCH crates/store/src/file.rs:5744:19 False: while let Some(dir) = next {
  F? assert! condition branch (failing side) BRANCH crates/store/src/file.rs:6863:82 False: matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
  GAP test code                              BRANCH crates/store/src/file.rs:8659:16 True: if &slot[..7] != b"BRUTEXB" {
  GAP test code                              BRANCH crates/store/src/file.rs:9075:16 False: if let Some(head) = cache.bytes.get_mut(..8) {
  F? assert! condition branch (failing side) BRANCH crates/store/src/file.rs:9087:13 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file.rs:9087:45 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file.rs:9087:73 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file.rs:10699:24 False: if *ts_micros == bar(10).ts_micros
  F? panic! (failing arm)                    LINE   crates/store/src/file_fault_tests.rs:320: other => panic!("read {skip}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_fault_tests.rs:657: Err(other) => panic!("read {skip}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_fault_tests.rs:696: other => panic!("{}: {other:?}", looked.display()),
  F? panic! (failing arm)                    LINE   crates/store/src/file_fault_tests.rs:869: other => panic!("{kind:?} measure {skip}: {other:?}"),
  GAP test code                              BRANCH crates/store/src/file_fault_tests.rs:315:57 False: Err(StoreError::Denied { path, action }) if fired => {
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:334:9 False: routed >= 1 && refused >= 1,
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:334:24 False: routed >= 1 && refused >= 1,
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:650:21 False: reason.starts_with(&format!("{}: ", srt.display()))
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:651:28 False: && reason.contains("refused by the file test seam"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:661:9 False: built >= 1 && refused >= 1,
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:661:23 False: built >= 1 && refused >= 1,
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:755:71 False: matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
  GAP test code                              BRANCH crates/store/src/file_fault_tests.rs:845:61 False: Err(StoreError::Denied { path, action }) if fired => {
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:879:26 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:879:42 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:912:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:912:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:949:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_fault_tests.rs:949:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  GAP production                             LINE   crates/store/src/file_sealed.rs:136: return Err(StoreError::SealedRefused {
  GAP production                             LINE   crates/store/src/file_sealed.rs:137: path: at,
  GAP production                             LINE   crates/store/src/file_sealed.rs:138: reason: "block_rows",
  GAP production                             LINE   crates/store/src/file_sealed.rs:139: detail: format!(
  GAP production                             LINE   crates/store/src/file_sealed.rs:140: "the head declares {} rows a frame and a handle caches at most {}",
  GAP production                             LINE   crates/store/src/file_sealed.rs:141: head.block_rows,
  GAP production                             LINE   crates/store/src/file_sealed.rs:142: CACHED_BLOCK_LEN / RECORD_LEN
  GAP production                             LINE   crates/store/src/file_sealed.rs:143: ),
  GAP production                             LINE   crates/store/src/file_sealed.rs:144: });
  GAP production                             LINE   crates/store/src/file_sealed.rs:231: )?;
  GAP production                             LINE   crates/store/src/file_sealed.rs:236: path: self.bars_path.clone(),
  GAP production                             LINE   crates/store/src/file_sealed.rs:237: source: FormatError::OffsetOverflow,
  GAP production                             LINE   crates/store/src/file_sealed.rs:238: })?;
  GAP production                             LINE   crates/store/src/file_sealed.rs:359: }
  GAP production                             LINE   crates/store/src/file_sealed.rs:372: continue;
  GAP production                             LINE   crates/store/src/file_sealed.rs:418: return Ok(None);
  GAP production                             BRANCH crates/store/src/file_sealed.rs:135:12 True: if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:202:36 True: if cache.block != block || cache.n_valid != n_valid {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:341:20 False: if let Some(count) = first.get_mut(at + 1) {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:347:20 False: if let Some(start) = first.get_mut(at) {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:354:20 False: if let Some(slot) = next_slot.get_mut(at) {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:355:24 False: if let Some(place) = placed.get_mut(*slot) {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:371:21 False: let Some(&row) = rows.get(asked) else {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:379:24 False: if let Some(slot) = out.get_mut(asked) {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:417:9 False: let Some(sealed) = file.sealed.as_ref() else {
  GAP production                             BRANCH crates/store/src/file_sealed.rs:425:21 False: Err(why) if why.kind() == ErrorKind::NotFound => Err(format!(
  GAP production                             BRANCH crates/store/src/file_sealed.rs:456:16 False: let same = header.n_valid == sealed.header.n_valid
  GAP production                             BRANCH crates/store/src/file_sealed.rs:457:12 False: && header.generation == sealed.header.generation
  GAP production                             BRANCH crates/store/src/file_sealed.rs:458:12 False: && header.first_ts_micros == sealed.header.first_ts_micros
  GAP production                             BRANCH crates/store/src/file_sealed.rs:459:12 False: && header.last_ts_micros == sealed.header.last_ts_micros
  GAP production                             BRANCH crates/store/src/file_sealed.rs:460:12 False: && header.symbol_id == sealed.header.symbol_id
  GAP production                             BRANCH crates/store/src/file_sealed.rs:461:12 False: && header.timeframe_secs == sealed.header.timeframe_secs
  F? panic! (failing arm)                    LINE   crates/store/src/file_sealed_tests.rs:321: other => panic!("expected Missing naming the .bin, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_sealed_tests.rs:419: other => panic!("a damaged frame must be refused by name, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_sealed_tests.rs:439: other => panic!("a damaged head must be refused, got {other:?}"),
  F? assert! message argument                LINE   crates/store/src/file_sealed_tests.rs:798: matches!(
  F? assert! message argument                LINE   crates/store/src/file_sealed_tests.rs:803: crate::provenance::read(&root.0, path)
  F? assert_eq! message argument             LINE   crates/store/src/file_sealed_tests.rs:885: page.len()
  F? assert_eq! message argument             LINE   crates/store/src/file_sealed_tests.rs:898: page.len()
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:913: fn read_rows_at_timing() {
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:914: let _sink_is_mine = crate::emits::hold_the_sink();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:915: for n in [2_250_u64, 22_500, 225_000] {
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:916: let root = Root::new(&format!("rows-at-timing-{n}"));
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:917: let path = month_path(None);
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:918: let bars = seconds(n);
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:919: write_raw(&root.0, path, &bars);
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:920: seal(&root.0, path);
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:921: let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:922: for width in [100_u64, 1_000] {
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:923: let page: Vec<u64> = (0..width).map(|k| (k * 7_919 + 13) % n).collect();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:924: let _warm = sealed.read_rows_at(&page).unwrap();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:925: let mut took = Vec::with_capacity(2_000);
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:926: for _ in 0..2_000 {
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:927: let began = std::time::Instant::now();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:928: let got = sealed.read_rows_at(&page).unwrap();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:929: took.push(began.elapsed());
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:930: assert_eq!(got.len(), page.len());
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:932: took.sort_unstable();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:933: let k = took.len();
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:934: println!(
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:936: took[k / 2],
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:937: took[k * 99 / 100],
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:938: took[k - 1],
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:939: took[k / 2] / u32::try_from(width).unwrap()
  GAP test code                              LINE   crates/store/src/file_sealed_tests.rs:943: }
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:214:69 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:214:83 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:315:13 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:315:30 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:315:50 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:407:17 False: !reason.is_empty() && !detail.is_empty(),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:407:39 False: !reason.is_empty() && !detail.is_empty(),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:824:9 False: refused.starts_with("Err(") && refused.contains("sbar"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:824:40 False: refused.starts_with("Err(") && refused.contains("sbar"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:874:73 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  F? assert! condition branch (failing side) BRANCH crates/store/src/file_sealed_tests.rs:874:92 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  GAP test code                              LINE   crates/store/src/file_write_door_tests.rs:68: continue;
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:165: panic!("a new bar at a sealed month is `Sealed`, got {refused}");
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:322: _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:439: _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:539: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:595: other => panic!("{n}: {other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:605: other => panic!("{n}: {other}"),
  F? assert! message argument                LINE   crates/store/src/file_write_door_tests.rs:721: file.display()
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:749: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:765: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:809: Err(other) => panic!("{other}"),
  F? assert! message argument                LINE   crates/store/src/file_write_door_tests.rs:844: matches!(
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:859: panic!("{case}: {repaired:?}");
  F? assert! message argument                LINE   crates/store/src/file_write_door_tests.rs:902: matches!(
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:943: panic!("{repaired:?}");
  F? assert! message argument                LINE   crates/store/src/file_write_door_tests.rs:983: matches!(
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:1027: panic!("{case}: {repaired:?}");
  F? panic! (failing arm)                    LINE   crates/store/src/file_write_door_tests.rs:1067: panic!("{case}: {repaired:?}");
  F? let-else in test                        BRANCH crates/store/src/file_write_door_tests.rs:67:13 False: let Ok(entries) = fs::read_dir(&at) else {
  GAP test code                              BRANCH crates/store/src/file_write_door_tests.rs:156:13 False: let StoreError::Sealed {
  GAP test code                              BRANCH crates/store/src/file_write_door_tests.rs:852:13 False: let MonthRepair::Repaired {
  F? let-else in test                        BRANCH crates/store/src/file_write_door_tests.rs:942:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  F? let-else in test                        BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  GAP test code                              BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False: let MonthRepair::Repaired {
  GAP production                             LINE   crates/store/src/flock.rs:306: return;
  GAP production                             LINE   crates/store/src/flock.rs:335: || "an unnamed program".to_owned(),
  GAP test code                              LINE   crates/store/src/flock.rs:557: |_| std::fs::FileTimes::new(),
  GAP production                             LINE   crates/store/src/flock.rs:627: Impatient::Host(why) => TryLockError::Error(why),
  GAP production                             LINE   crates/store/src/flock.rs:682: note_unnoted(self.path.as_ref(), &why);
  GAP test code                              LINE   crates/store/src/flock.rs:791: return;
  GAP production                             LINE   crates/store/src/flock.rs:883: )?;
  GAP production                             LINE   crates/store/src/flock.rs:886: None => Ok(()),
  GAP production                             LINE   crates/store/src/flock.rs:889: Self::Host(why) => write!(f, "the host refused the lock: {why}"),
  GAP production                             LINE   crates/store/src/flock.rs:895: fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
  GAP production                             LINE   crates/store/src/flock.rs:896: match self {
  GAP production                             LINE   crates/store/src/flock.rs:897: Self::Held { .. } => None,
  GAP production                             LINE   crates/store/src/flock.rs:898: Self::Host(why) => Some(why),
  GAP production                             LINE   crates/store/src/flock.rs:900: }
  GAP production                             LINE   crates/store/src/flock.rs:974: host @ Impatient::Host(_) => host,
  GAP production                             BRANCH crates/store/src/flock.rs:181:9 False: self.writer.is_none() && self.readers == 0 && self.queue.is_none()
  GAP production                             BRANCH crates/store/src/flock.rs:305:9 False: let Some(holders) = held.as_mut() else {
  GAP test code                              BRANCH crates/store/src/flock.rs:560:24 False: if let Ok(modified) = meta.modified() {
  GAP test code                              BRANCH crates/store/src/flock.rs:563:24 False: if let Ok(accessed) = meta.accessed() {
  GAP production                             BRANCH crates/store/src/flock.rs:681:20 True: if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
  GAP test code                              BRANCH crates/store/src/flock.rs:790:8 True: if SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
  GAP production                             BRANCH crates/store/src/flock.rs:1094:24 False: if idle && let Some(holders) = held.as_mut() {
  GAP production                             BRANCH crates/store/src/flock.rs:1099:16 False: if empty {
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:109: panic!("contention, not a host refusal: {refused}");
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:173: panic!("a host refusal: {got:?}");
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:195: panic!("{got:?}");
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:265: panic!("{got:?}");
  GAP test code                              LINE   crates/store/src/flock_patient_tests.rs:323: )?;
  GAP test code                              LINE   crates/store/src/flock_patient_tests.rs:364: Err(_) => refused += 1,
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:439: panic!("it gave up: {refused:?}");
  GAP test code                              LINE   crates/store/src/flock_patient_tests.rs:611: Err(_) => refused += 1,
  F? panic! (failing arm)                    LINE   crates/store/src/flock_patient_tests.rs:630: Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
  GAP test code                              LINE   crates/store/src/flock_patient_tests.rs:640: unnamed += 1;
  F? assert! message argument                LINE   crates/store/src/flock_patient_tests.rs:687: matches!(
  GAP test code                              BRANCH crates/store/src/flock_patient_tests.rs:101:9 False: let Impatient::Held {
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:123:9 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:123:26 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  F? let-else in test                        BRANCH crates/store/src/flock_patient_tests.rs:172:9 False: let Err(Impatient::Host(why)) = got else {
  GAP test code                              BRANCH crates/store/src/flock_patient_tests.rs:191:9 False: let Err(Impatient::Held {
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:219:62 False: matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
  GAP test code                              BRANCH crates/store/src/flock_patient_tests.rs:261:9 False: let Err(Impatient::Held {
  GAP test code                              BRANCH crates/store/src/flock_patient_tests.rs:435:9 False: let Err(Impatient::Held {
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:565:9 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:565:51 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  GAP test code                              BRANCH crates/store/src/flock_patient_tests.rs:639:16 True: if !named {
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:683:9 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_patient_tests.rs:683:50 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  F? panic! (failing arm)                    LINE   crates/store/src/flock_tests.rs:213: panic!(
  F? panic! (failing arm)                    LINE   crates/store/src/flock_tests.rs:244: panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
  F? panic! (failing arm)                    LINE   crates/store/src/flock_tests.rs:264: panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
  F? panic! (failing arm)                    LINE   crates/store/src/flock_tests.rs:283: panic!(
  F? panic! (failing arm)                    LINE   crates/store/src/flock_tests.rs:320: panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
  GAP test code                              LINE   crates/store/src/flock_tests.rs:699: matches!(
  F? assert! message argument                LINE   crates/store/src/flock_tests.rs:924: matches!(
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:732:13 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:732:38 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:759:9 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:759:42 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:841:9 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:841:51 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:952:9 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/flock_tests.rs:952:66 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  GAP production                             LINE   crates/store/src/format.rs:1332: Self::CounterExceedsMonth { n_valid, slots } => write!(
  GAP production                             LINE   crates/store/src/format.rs:1333: f,
  GAP production                             LINE   crates/store/src/format.rs:1338: first_ts_micros,
  GAP production                             LINE   crates/store/src/format.rs:1339: last_ts_micros,
  GAP production                             LINE   crates/store/src/format.rs:1340: } => write!(
  GAP production                             LINE   crates/store/src/format.rs:1341: f,
  GAP production                             LINE   crates/store/src/generation.rs:309: return Err(refused(&pointer, "it is not a regular file".to_owned()));
  GAP production                             LINE   crates/store/src/generation.rs:313: return Err(refused(
  GAP production                             LINE   crates/store/src/generation.rs:314: &pointer,
  GAP production                             LINE   crates/store/src/generation.rs:315: format!(
  GAP production                             LINE   crates/store/src/generation.rs:316: "it is {len} bytes, past the {MAX_LEN} a pointer of {MAX_RECORDS} records can be"
  GAP production                             LINE   crates/store/src/generation.rs:317: ),
  GAP production                             LINE   crates/store/src/generation.rs:318: ));
  GAP production                             LINE   crates/store/src/generation.rs:343: note_skipped(&pointer, 0, len, 0);
  GAP production                             LINE   crates/store/src/generation.rs:426: return Err(StoreError::NotABarPath { found: path.file() }.into());
  GAP production                             LINE   crates/store/src/generation.rs:450: )?;
  GAP production                             LINE   crates/store/src/generation.rs:466: )?;
  GAP production                             LINE   crates/store/src/generation.rs:473: Err(std::fs::TryLockError::Error(why)) => {
  GAP production                             LINE   crates/store/src/generation.rs:474: return io_at(Err(why), &pointer_file, "lock pointer", false);
  GAP production                             LINE   crates/store/src/generation.rs:493: release(lock, &pointer_file, false)?;
  GAP production                             LINE   crates/store/src/generation.rs:494: return Err(RepairError::RevisionLimit);
  GAP production                             LINE   crates/store/src/generation.rs:512: )?;
  GAP production                             LINE   crates/store/src/generation.rs:539: return Ok(());
  GAP production                             LINE   crates/store/src/generation.rs:600: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/generation.rs:601: operation,
  GAP production                             LINE   crates/store/src/generation.rs:602: kind: error.kind(),
  GAP production                             LINE   crates/store/src/generation.rs:603: publication_may_be_visible: visible,
  GAP production                             LINE   crates/store/src/generation.rs:604: })
  GAP production                             BRANCH crates/store/src/generation.rs:126:16 False: && header.first_ts_micros == self.first_ts_micros
  GAP production                             BRANCH crates/store/src/generation.rs:127:16 False: && header.last_ts_micros == self.last_ts_micros
  GAP production                             BRANCH crates/store/src/generation.rs:144:20 False: if let Some(slot) = bytes.get_mut(at..at + field.len()) {
  GAP production                             BRANCH crates/store/src/generation.rs:149:16 False: if let Some(slot) = bytes.get_mut(60..64) {
  GAP production                             BRANCH crates/store/src/generation.rs:300:21 False: Err(why) if why.kind() == ErrorKind::NotADirectory && !root.is_dir() => {
  GAP production                             BRANCH crates/store/src/generation.rs:308:8 True: if !meta.is_file() {
  GAP production                             BRANCH crates/store/src/generation.rs:312:8 True: if len > MAX_LEN {
  GAP production                             BRANCH crates/store/src/generation.rs:342:12 True: if len > 0 {
  GAP production                             BRANCH crates/store/src/generation.rs:425:8 True: if path.file() != FileKind::Bars {
  GAP production                             BRANCH crates/store/src/generation.rs:492:8 True: if record >= MAX_RECORDS {
  GAP production                             BRANCH crates/store/src/generation.rs:538:9 False: let Ok(below) = leaf.strip_prefix(root) else {
  GAP production                             LINE   crates/store/src/month_summary.rs:440: Self::BelowThreshold => "below_threshold",
  GAP production                             LINE   crates/store/src/month_summary.rs:445: Self::Version(_) => "version",
  GAP production                             LINE   crates/store/src/month_summary.rs:446: Self::Shape(_) => "shape",
  GAP production                             LINE   crates/store/src/month_summary.rs:447: Self::NotThisMonth => "not_this_month",
  GAP production                             LINE   crates/store/src/month_summary.rs:448: Self::Stale { .. } => "stale",
  GAP production                             LINE   crates/store/src/month_summary.rs:458: Self::BelowThreshold => write!(
  GAP production                             LINE   crates/store/src/month_summary.rs:459: f,
  GAP production                             LINE   crates/store/src/month_summary.rs:467: Self::Version(version) => {
  GAP production                             LINE   crates/store/src/month_summary.rs:468: write!(
  GAP production                             LINE   crates/store/src/month_summary.rs:469: f,
  GAP production                             LINE   crates/store/src/month_summary.rs:473: Self::Shape(why) => write!(f, "{reason}: {why}"),
  GAP production                             LINE   crates/store/src/month_summary.rs:475: write!(
  GAP production                             LINE   crates/store/src/month_summary.rs:476: f,
  GAP production                             LINE   crates/store/src/month_summary.rs:480: Self::Stale { summary, header } => write!(
  GAP production                             LINE   crates/store/src/month_summary.rs:481: f,
  GAP production                             LINE   crates/store/src/month_summary.rs:498: 29
  GAP production                             LINE   crates/store/src/month_summary.rs:731: pub const fn first_day(&self) -> i64 {
  GAP production                             LINE   crates/store/src/month_summary.rs:732: self.first_day
  GAP production                             LINE   crates/store/src/month_summary.rs:733: }
  GAP production                             LINE   crates/store/src/month_summary.rs:737: pub fn last_day(&self) -> i64 {
  GAP production                             LINE   crates/store/src/month_summary.rs:738: self.first_day + i64::try_from(self.days.len()).unwrap_or(0) - 1
  GAP production                             LINE   crates/store/src/month_summary.rs:739: }
  GAP production                             LINE   crates/store/src/month_summary.rs:779: return Some(Vec::new());
  GAP production                             LINE   crates/store/src/month_summary.rs:1076: return Err(Refused::Shape("a day's first and last second disagree"));
  GAP production                             LINE   crates/store/src/month_summary.rs:1087: }
  GAP production                             LINE   crates/store/src/month_summary.rs:1128: return Err(Refused::Version(version));
  GAP production                             LINE   crates/store/src/month_summary.rs:1226: None => self.short = true,
  GAP production                             LINE   crates/store/src/month_summary.rs:1337: }
  GAP production                             LINE   crates/store/src/month_summary.rs:1338: return Err(Refused::Unreadable(format!("{}: {why}", path.display())));
  GAP production                             BRANCH crates/store/src/month_summary.rs:239:12 False: if let Some(slot) = words.get_mut(word) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:497:17 True: if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:497:43 True: if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:497:43 False: if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:497:73 True: if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:712:16 True: || self.timeframe_secs != header.timeframe_secs
  GAP production                             BRANCH crates/store/src/month_summary.rs:713:16 True: || self.month != month
  GAP production                             BRANCH crates/store/src/month_summary.rs:717:22 True: let stamps = header.n_valid == 0
  GAP production                             BRANCH crates/store/src/month_summary.rs:718:17 False: || (self.first_ts_micros == header.first_ts_micros
  GAP production                             BRANCH crates/store/src/month_summary.rs:720:86 True: if self.generation != header.generation || self.n_valid != header.n_valid || !stamps {
  GAP production                             BRANCH crates/store/src/month_summary.rs:778:13 False: let Some(entry) = self.day(day) else {
  GAP production                             BRANCH crates/store/src/month_summary.rs:850:39 True: if self.counts.invalid > 0 || self.counts.decreasing > 0 {
  GAP production                             BRANCH crates/store/src/month_summary.rs:939:24 False: if let Some(words) = entry.seconds.as_deref() {
  GAP production                             BRANCH crates/store/src/month_summary.rs:991:40 True: if payload.len() < HEAD_LEN || payload.len() > MAX_PAYLOAD_LEN {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1055:12 True: if at.short || at.at != payload.len() {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1074:24 False: let some = entry.first <= entry.last && entry.last < 86_400 && entry.bars > 0;
  GAP production                             BRANCH crates/store/src/month_summary.rs:1075:27 False: if !(some || (none && entry.bars == 0)) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1075:35 False: if !(some || (none && entry.bars == 0)) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1085:24 False: if let Some(words) = entry.seconds.as_deref_mut() {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1127:12 True: if body.get(2..4) != Some([0, 0].as_slice()) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1195:12 False: if let Some(slot) = words.get_mut(word) {
  GAP production                             BRANCH crates/store/src/month_summary.rs:1332:16 False: if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
  GAP test code                              LINE   crates/store/src/month_summary_tests.rs:294: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? panic! (failing arm)                    LINE   crates/store/src/month_summary_tests.rs:295: panic!("a read of {} waited for a writer", pipe.display());
  F? panic! (failing arm)                    LINE   crates/store/src/month_summary_tests.rs:791: other => panic!("{other:?}"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:278:52 False: matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
  F? let-else in test                        BRANCH crates/store/src/month_summary_tests.rs:293:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:320:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:340:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:355:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:377:13 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:377:40 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:461:32 False: Refused::Shape(why) if why.contains("flag")
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:467:60 False: matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:480:59 False: assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:483:63 False: assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:488:61 False: assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:491:59 False: assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:655:13 False: assert!(round.tracks_carries() && !round.wants_carries());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:655:39 False: assert!(round.tracks_carries() && !round.wants_carries());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:658:13 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:658:42 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:739:37 False: Err(Refused::Shape(why)) if why.contains("not a held minute")
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:757:13 False: assert!(!read.tracks_carries() && read.wants_carries());
  F? assert! condition branch (failing side) BRANCH crates/store/src/month_summary_tests.rs:757:39 False: assert!(!read.tracks_carries() && read.wants_carries());
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:165: return Plan::Whole(refused.to_string());
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:188: Plan::Whole(refused.to_string())
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:256: return Ok(Summarized::Empty);
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:268: Err(Refused::BelowThreshold) => return Ok(Summarized::BelowThreshold),
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:295: return Err(StoreError::Io {
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:296: path: sidecar.path.clone(),
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:297: action: Action::Write,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:298: kind: std::io::ErrorKind::AlreadyExists,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:299: code: None,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:300: });
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:303: crate::staging::StagingError::Io { source, .. } => {
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:304: classify(&staging, Action::Open, &source)
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:309: _ => StoreError::Io {
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:310: path: staging.clone(),
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:311: action: Action::Open,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:312: kind: std::io::ErrorKind::AlreadyExists,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:313: code: None,
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:314: },
  GAP production                             LINE   crates/store/src/month_summary_writer.rs:315: })?;
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:164:13 False: let Refused::Stale { .. } = refused else {
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:176:22 False: let prefix = summary.generation < header.generation
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:177:16 False: && summary.n_valid > 0
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:178:16 False: && summary.n_valid < header.n_valid
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:179:16 False: && summary.counts.folded == summary.n_valid
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:180:16 False: && summary.first_ts_micros == header.first_ts_micros
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:184:12 False: if prefix {
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:255:12 True: if file.header.n_valid == 0 {
  GAP production                             BRANCH crates/store/src/month_summary_writer.rs:293:12 True: && !meta.is_file()
  GAP production                             LINE   crates/store/src/provenance.rs:1105: ProvenanceError::Io {
  GAP production                             LINE   crates/store/src/provenance.rs:1106: path: crate::generation::pointer_path(root, path),
  GAP production                             LINE   crates/store/src/provenance.rs:1107: action: "resolving the month's served generation",
  GAP production                             LINE   crates/store/src/provenance.rs:1108: source: io::Error::other(why.to_string()),
  GAP production                             LINE   crates/store/src/provenance.rs:1109: }
  GAP production                             LINE   crates/store/src/provenance.rs:1110: })?;
  GAP production                             LINE   crates/store/src/provenance.rs:1170: io_error(
  GAP production                             LINE   crates/store/src/provenance.rs:1171: &at,
  GAP production                             LINE   crates/store/src/provenance.rs:1173: io::Error::other(why.to_string()),
  GAP production                             LINE   crates/store/src/provenance.rs:1175: };
  GAP production                             LINE   crates/store/src/provenance.rs:1189: Scan::Born(sidecar) => Found::Present(sidecar),
  GAP production                             LINE   crates/store/src/provenance.rs:1271: fs::TryLockError::Error(why) => io_error(&at, "lock", why),
  GAP production                             LINE   crates/store/src/provenance.rs:1282: )?;
  GAP production                             LINE   crates/store/src/provenance.rs:1566: let deferred = batch
  GAP production                             LINE   crates/store/src/provenance.rs:1567: .defer_file(file, at)
  GAP production                             LINE   crates/store/src/provenance.rs:1568: .map_err(|why| ProvenanceError::Io {
  GAP production                             LINE   crates/store/src/provenance.rs:1569: path: at.to_path_buf(),
  GAP production                             LINE   crates/store/src/provenance.rs:1571: source: io::Error::other(why.to_string()),
  GAP production                             LINE   crates/store/src/provenance.rs:1572: })?;
  GAP production                             LINE   crates/store/src/provenance.rs:1573: if deferred {
  GAP production                             LINE   crates/store/src/provenance.rs:1574: return Ok(());
  GAP production                             LINE   crates/store/src/provenance.rs:1575: }
  GAP production                             LINE   crates/store/src/provenance.rs:1588: return Ok(());
  GAP production                             BRANCH crates/store/src/provenance.rs:615:19 True: } else if self.year != expected.year || self.month != expected.month {
  GAP production                             BRANCH crates/store/src/provenance.rs:1069:8 True: if !file.metadata().is_ok_and(|meta| meta.is_file())
  GAP production                             BRANCH crates/store/src/provenance.rs:1565:12 True: if let Some(batch) = batch {
  GAP production                             BRANCH crates/store/src/provenance.rs:1573:12 True: if deferred {
  GAP production                             BRANCH crates/store/src/provenance.rs:1573:12 False: if deferred {
  GAP production                             BRANCH crates/store/src/provenance.rs:1587:8 True: if batch.is_some_and(|batch| batch.defer_directory(dir)) {
  GAP production                             BRANCH crates/store/src/provenance.rs:1658:12 False: if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
  GAP production                             BRANCH crates/store/src/provenance.rs:1666:12 False: if let Some(run) = bytes.get(at..at.saturating_add(N)) {
  GAP production                             LINE   crates/store/src/quarantine.rs:263: _ => None,
  GAP production                             LINE   crates/store/src/quarantine.rs:319: None => String::new(),
  GAP production                             LINE   crates/store/src/quarantine.rs:329: None
  GAP production                             LINE   crates/store/src/quarantine.rs:366: Self::Dropping => 0,
  GAP production                             LINE   crates/store/src/quarantine.rs:376: _ => None,
  GAP production                             LINE   crates/store/src/quarantine.rs:647: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:648: operation,
  GAP production                             LINE   crates/store/src/quarantine.rs:649: kind: why.kind(),
  GAP production                             LINE   crates/store/src/quarantine.rs:650: })
  GAP production                             LINE   crates/store/src/quarantine.rs:823: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:834: Err(why) => {
  GAP production                             LINE   crates/store/src/quarantine.rs:835: return Err(QuarantineError::Io {
  GAP production                             LINE   crates/store/src/quarantine.rs:836: path: index.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:837: operation: "open the quarantine index",
  GAP production                             LINE   crates/store/src/quarantine.rs:838: kind: refusal_kind(&why),
  GAP production                             LINE   crates/store/src/quarantine.rs:839: });
  GAP production                             LINE   crates/store/src/quarantine.rs:853: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1000: Ok(_) => Err(QuarantineError::Io {
  GAP production                             LINE   crates/store/src/quarantine.rs:1001: path,
  GAP production                             LINE   crates/store/src/quarantine.rs:1002: operation: "inspect the quarantine ledger (not a regular file)",
  GAP production                             LINE   crates/store/src/quarantine.rs:1003: kind: io::ErrorKind::InvalidInput,
  GAP production                             LINE   crates/store/src/quarantine.rs:1004: }),
  GAP production                             LINE   crates/store/src/quarantine.rs:1006: Err(why) => io_at(Err(why), &path, "inspect the quarantine ledger"),
  GAP production                             LINE   crates/store/src/quarantine.rs:1016: return false;
  GAP production                             LINE   crates/store/src/quarantine.rs:1024: Err(std::fs::TryLockError::Error(_)) => false,
  GAP production                             LINE   crates/store/src/quarantine.rs:1028: fn refusal_kind(why: &crate::safe_open::ReadRefusal) -> io::ErrorKind {
  GAP production                             LINE   crates/store/src/quarantine.rs:1029: match why {
  GAP production                             LINE   crates/store/src/quarantine.rs:1030: crate::safe_open::ReadRefusal::Io { source, .. } => source.kind(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1031: _ => io::ErrorKind::InvalidInput,
  GAP production                             LINE   crates/store/src/quarantine.rs:1033: }
  GAP production                             LINE   crates/store/src/quarantine.rs:1090: return Ok(None);
  GAP production                             LINE   crates/store/src/quarantine.rs:1098: return Ok(Self::default());
  GAP production                             LINE   crates/store/src/quarantine.rs:1149: pub fn is_empty(&self) -> bool {
  GAP production                             LINE   crates/store/src/quarantine.rs:1150: self.months.is_empty()
  GAP production                             LINE   crates/store/src/quarantine.rs:1151: }
  GAP production                             LINE   crates/store/src/quarantine.rs:1159: Err(why) => {
  GAP production                             LINE   crates/store/src/quarantine.rs:1160: return Err(QuarantineError::Io {
  GAP production                             LINE   crates/store/src/quarantine.rs:1161: path: index.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1162: operation: "open the quarantine index",
  GAP production                             LINE   crates/store/src/quarantine.rs:1163: kind: refusal_kind(&why),
  GAP production                             LINE   crates/store/src/quarantine.rs:1164: });
  GAP production                             LINE   crates/store/src/quarantine.rs:1168: return Err(QuarantineError::Torn {
  GAP production                             LINE   crates/store/src/quarantine.rs:1169: path: index.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1170: len,
  GAP production                             LINE   crates/store/src/quarantine.rs:1171: });
  GAP production                             LINE   crates/store/src/quarantine.rs:1178: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1180: return Err(QuarantineError::Torn {
  GAP production                             LINE   crates/store/src/quarantine.rs:1181: path: index.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1182: len: image.len() as u64,
  GAP production                             LINE   crates/store/src/quarantine.rs:1183: });
  GAP production                             LINE   crates/store/src/quarantine.rs:1324: return Err(QuarantineError::Reason(
  GAP production                             LINE   crates/store/src/quarantine.rs:1325: "the writer's name is empty or holds a tab or a line break".to_owned(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1326: ));
  GAP production                             LINE   crates/store/src/quarantine.rs:1329: return Err(QuarantineError::Reason(
  GAP production                             LINE   crates/store/src/quarantine.rs:1330: "the named stamps are not strictly ascending".to_owned(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1331: ));
  GAP production                             LINE   crates/store/src/quarantine.rs:1346: path: lock_path(root, feed),
  GAP production                             LINE   crates/store/src/quarantine.rs:1348: kind: io::ErrorKind::Other,
  GAP production                             LINE   crates/store/src/quarantine.rs:1349: })
  GAP production                             LINE   crates/store/src/quarantine.rs:1434: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
  GAP production                             LINE   crates/store/src/quarantine.rs:1435: io_at(Err(why), &dir, "create the audit directory")
  GAP production                             LINE   crates/store/src/quarantine.rs:1439: Err(why) => io_at(Err(why), &dir, "inspect the audit directory"),
  GAP production                             LINE   crates/store/src/quarantine.rs:1455: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1462: Err(other) => Err(QuarantineError::Io {
  GAP production                             LINE   crates/store/src/quarantine.rs:1463: path,
  GAP production                             LINE   crates/store/src/quarantine.rs:1464: operation: "lock the quarantine",
  GAP production                             LINE   crates/store/src/quarantine.rs:1465: kind: impatient_kind(&other),
  GAP production                             LINE   crates/store/src/quarantine.rs:1466: }),
  GAP production                             LINE   crates/store/src/quarantine.rs:1470: fn impatient_kind(why: &Impatient) -> io::ErrorKind {
  GAP production                             LINE   crates/store/src/quarantine.rs:1471: match why {
  GAP production                             LINE   crates/store/src/quarantine.rs:1472: Impatient::Host(error) => error.kind(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1473: Impatient::Held { .. } => io::ErrorKind::WouldBlock,
  GAP production                             LINE   crates/store/src/quarantine.rs:1475: }
  GAP production                             LINE   crates/store/src/quarantine.rs:1567: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1605: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1607: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:1608: offsets: vec![at],
  GAP production                             LINE   crates/store/src/quarantine.rs:1609: };
  GAP production                             LINE   crates/store/src/quarantine.rs:1618: return Err(damaged(at));
  GAP production                             LINE   crates/store/src/quarantine.rs:1625: }
  GAP production                             LINE   crates/store/src/quarantine.rs:1626: Err(damaged(at))
  GAP production                             LINE   crates/store/src/quarantine.rs:1644: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1754: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:1758: Err(why) => {
  GAP production                             LINE   crates/store/src/quarantine.rs:1759: return Err(QuarantineError::Io {
  GAP production                             LINE   crates/store/src/quarantine.rs:1760: path,
  GAP production                             LINE   crates/store/src/quarantine.rs:1761: operation: "open the quarantine ledger",
  GAP production                             LINE   crates/store/src/quarantine.rs:1762: kind: refusal_kind(&why),
  GAP production                             LINE   crates/store/src/quarantine.rs:1763: });
  GAP production                             LINE   crates/store/src/quarantine.rs:1793: ledger.damaged.push(at);
  GAP production                             LINE   crates/store/src/quarantine.rs:1831: return None;
  GAP production                             LINE   crates/store/src/quarantine.rs:1846: return None;
  GAP production                             LINE   crates/store/src/quarantine.rs:2056: return Err(QuarantineError::Damaged {
  GAP production                             LINE   crates/store/src/quarantine.rs:2057: path: index.to_path_buf(),
  GAP production                             LINE   crates/store/src/quarantine.rs:2058: slot,
  GAP production                             LINE   crates/store/src/quarantine.rs:2059: });
  GAP production                             LINE   crates/store/src/quarantine.rs:2124: continue;
  GAP production                             LINE   crates/store/src/quarantine.rs:2163: None => "its header did not check".to_owned(),
  GAP production                             LINE   crates/store/src/quarantine.rs:2179: Err(QuarantineError::Torn { len, .. }) => format!("it was {len} bytes, not {INDEX_LEN}"),
  GAP production                             LINE   crates/store/src/quarantine.rs:2180: Err(other) => return Err(other),
  GAP production                             LINE   crates/store/src/quarantine.rs:2226: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:2232: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:2243: _ => io::ErrorKind::AlreadyExists,
  GAP production                             LINE   crates/store/src/quarantine.rs:2308: released?;
  GAP production                             LINE   crates/store/src/quarantine.rs:2309: Ok(done)
  GAP production                             LINE   crates/store/src/quarantine.rs:2411: return None;
  GAP production                             LINE   crates/store/src/quarantine.rs:2421: return None;
  GAP production                             LINE   crates/store/src/quarantine.rs:2430: pub fn is_empty(&self) -> bool {
  GAP production                             LINE   crates/store/src/quarantine.rs:2431: self.months.is_empty()
  GAP production                             LINE   crates/store/src/quarantine.rs:2432: }
  GAP production                             LINE   crates/store/src/quarantine.rs:2484: Self::Repair(why) => why.fmt(f),
  GAP production                             LINE   crates/store/src/quarantine.rs:2505: fn from(value: crate::file::StoreError) -> Self {
  GAP production                             LINE   crates/store/src/quarantine.rs:2506: Self::Repair(value.into())
  GAP production                             LINE   crates/store/src/quarantine.rs:2507: }
  GAP production                             LINE   crates/store/src/quarantine.rs:2652: )?;
  GAP production                             LINE   crates/store/src/quarantine.rs:2722: Some(Applied::Resumed { bars, .. }) => bars.saturating_add(intent.bars),
  GAP production                             BRANCH crates/store/src/quarantine.rs:328:12 True: if byte == 0 || byte & !Self::EVERY != 0 {
  GAP production                             BRANCH crates/store/src/quarantine.rs:328:25 True: if byte == 0 || byte & !Self::EVERY != 0 {
  GAP production                             BRANCH crates/store/src/quarantine.rs:713:12 False: if let Some(src) = bytes.get(at..at + N) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:773:16 False: if let Some(dst) = bytes.get_mut(at..at + value.len()) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:778:12 False: if let Some(dst) = bytes.get_mut(60..64) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:833:21 False: Err(why) if why.is_missing() => return Ok(None),
  GAP production                             BRANCH crates/store/src/quarantine.rs:865:12 False: && u32::from_le_bytes(field(header, 8)) == INDEX_VERSION
  GAP production                             BRANCH crates/store/src/quarantine.rs:866:12 False: && u32::from_le_bytes(field(header, 12)) == SLOT_U32
  GAP production                             BRANCH crates/store/src/quarantine.rs:867:12 False: && u64::from_le_bytes(field(header, 16)) == CAPACITY
  GAP production                             BRANCH crates/store/src/quarantine.rs:868:12 False: && u64::from_le_bytes(field(header, 24)) == BUCKET
  GAP production                             BRANCH crates/store/src/quarantine.rs:999:21 False: Ok(meta) if meta.file_type().is_file() => Ok(meta.len()),
  GAP production                             BRANCH crates/store/src/quarantine.rs:1005:21 False: Err(why) if why.kind() == io::ErrorKind::NotFound => Ok(0),
  GAP production                             BRANCH crates/store/src/quarantine.rs:1015:9 False: let Ok(file) = crate::safe_open::open_read(&path) else {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1056:35 False: } if held == *hash && at == month => {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1089:17 False: let Some(image) = read_image(&index)? else {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1097:13 False: let Some(image) = image else {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1158:21 False: Err(why) if why.is_missing() => return Ok(None),
  GAP production                             BRANCH crates/store/src/quarantine.rs:1167:8 True: if len != INDEX_LEN {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1179:8 True: if image.len() as u64 != INDEX_LEN {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1323:8 True: if act.who.contains(['\t', '\n', '\r']) || act.who.is_empty() {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1323:48 True: if act.who.contains(['\t', '\n', '\r']) || act.who.is_empty() {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1328:8 True: if !ascending(act.stamps) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1340:46 False: .all(|pair| matches!(pair, [a, b] if a < b))
  GAP production                             BRANCH crates/store/src/quarantine.rs:1392:12 False: if let QuarantineError::LedgerDamaged { offsets, .. } = refused {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1411:12 False: if let QuarantineError::LedgerCut {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1433:21 False: Err(why) if why.kind() == io::ErrorKind::NotFound => match fs::create_dir(&dir) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1434:25 True: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1434:25 False: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1617:12 True: if !raw.ends_with(TORN_MARK) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1623:8 False: if start.len() as u64 == len && LEDGER_HEAD.as_bytes().starts_with(rest) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1623:37 False: if start.len() as u64 == len && LEDGER_HEAD.as_bytes().starts_with(rest) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1757:21 False: Err(why) if why.is_missing() => return Ok(Ledger::default()),
  GAP production                             BRANCH crates/store/src/quarantine.rs:1792:16 True: if raw != LEDGER_HEAD.as_bytes() {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1812:8 True: if crc.len() != 8 || u32::from_str_radix(crc, 16).ok()? != crc32c(body.as_bytes()) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1816:9 False: let [
  GAP production                             BRANCH crates/store/src/quarantine.rs:1845:8 True: if !ascending(&stamps) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1888:56 False: .filter(|line| scopes.contains(&line.scope) && line.key == key && line.month == month)
  GAP production                             BRANCH crates/store/src/quarantine.rs:1960:16 False: if let Some(dst) = image.get_mut(at..at + value.len()) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1970:12 False: if let Some(dst) = image.get_mut(60..64) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:1977:5 False: described(image).is_some()
  GAP production                             BRANCH crates/store/src/quarantine.rs:2053:35 False: } if held == *hash && was == month => return Ok((slot, Some(before))),
  GAP production                             BRANCH crates/store/src/quarantine.rs:2080:12 False: if let Some(dst) = image.get_mut(at..at + SLOT) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2085:16 False: if let Some(dst) = image.get_mut(32..40) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2094:12 False: if let Some(dst) = image.get_mut(40..48) {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2123:13 False: let Some(month) = parse_month(&line.month) else {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2365:16 False: if let Some(described) = described
  GAP production                             BRANCH crates/store/src/quarantine.rs:2397:16 False: if row.causes.is_empty() {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2410:12 True: if self.instruments.is_empty() {
  GAP production                             BRANCH crates/store/src/quarantine.rs:2420:12 True: if self.months.is_empty() {
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:145: panic!("expected a dropped generation, got {applied:?}");
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:175: panic!("the month is labelled");
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:264: panic!("the month is answered by name");
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:551: other => panic!("expected the hold named, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:800: panic!("the month is still answered by name");
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:892: other => panic!("{why}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:1092: other => panic!("expected the quarantine to refuse by name, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:1258: other => panic!("the rerun indexes the drop: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/quarantine_tests.rs:1975: other => panic!("{other:?}"),
  F? let-else in test                        BRANCH crates/store/src/quarantine_tests.rs:144:9 False: let Applied::Dropped { drop, .. } = applied else {
  F? let-else in test                        BRANCH crates/store/src/quarantine_tests.rs:174:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:210:63 False: assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
  F? let-else in test                        BRANCH crates/store/src/quarantine_tests.rs:263:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:342:66 False: matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:348:77 False: assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:497:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:497:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:508:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:508:92 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  F? let-else in test                        BRANCH crates/store/src/quarantine_tests.rs:799:9 False: let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:855:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:855:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1071:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1072:16 False: && e.bars == 6
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1073:16 False: && e.causes.said() == "before_listing and foreign_level"
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1307:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1307:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1366:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1366:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1399:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1446:38 False: Ok(Probe::Quarantined(e)) if e.bars == 3
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1513:9 False: refused.is_err() && words.contains("quarantined"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1513:29 False: refused.is_err() && words.contains("quarantined"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1564:9 False: refused.is_err() && words.contains("quarantined"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1564:29 False: refused.is_err() && words.contains("quarantined"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1619:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1619:67 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1619:82 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1727:58 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1727:94 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  F? assert! condition branch (failing side) BRANCH crates/store/src/quarantine_tests.rs:1845:77 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
  GAP production                             LINE   crates/store/src/repair.rs:283: Self::NoBasisWitness => f.write_str(
  GAP production                             LINE   crates/store/src/repair.rs:284: "repair refused: no offered bar shares a stamp with the month's held bars, so nothing proves the vendor's price basis is unchanged since the
  GAP production                             LINE   crates/store/src/repair.rs:286: Self::IndexedSibling(path) => write!(
  GAP production                             LINE   crates/store/src/repair.rs:287: f,
  GAP production                             LINE   crates/store/src/repair.rs:289: path.display()
  GAP production                             LINE   crates/store/src/repair.rs:291: Self::UnheldStamp(ts) => write!(
  GAP production                             LINE   crates/store/src/repair.rs:292: f,
  GAP production                             LINE   crates/store/src/repair.rs:295: Self::WholeMonth { bars } => write!(
  GAP production                             LINE   crates/store/src/repair.rs:296: f,
  GAP production                             LINE   crates/store/src/repair.rs:299: Self::NothingNamed => f.write_str(
  GAP production                             LINE   crates/store/src/repair.rs:300: "drop refused: it names no bar, or names one out of order or twice; nothing was written",
  GAP production                             LINE   crates/store/src/repair.rs:302: Self::Reason(why) => write!(f, "refused: {why}; nothing was written"),
  GAP production                             LINE   crates/store/src/repair.rs:303: Self::ReadBack { index } => write!(
  GAP production                             LINE   crates/store/src/repair.rs:304: f,
  GAP production                             LINE   crates/store/src/repair.rs:314: Self::QuarantineUnknown(why) => write!(
  GAP production                             LINE   crates/store/src/repair.rs:315: f,
  GAP production                             LINE   crates/store/src/repair.rs:587: return Err(RepairError::Conflict);
  GAP production                             LINE   crates/store/src/repair.rs:600: )?;
  GAP production                             LINE   crates/store/src/repair.rs:643: return;
  GAP production                             LINE   crates/store/src/repair.rs:851: rows: header.n_valid,
  GAP production                             LINE   crates/store/src/repair.rs:852: ceiling,
  GAP production                             LINE   crates/store/src/repair.rs:853: };
  GAP production                             LINE   crates/store/src/repair.rs:936: Err(why) => return Err(why),
  GAP production                             LINE   crates/store/src/repair.rs:939: Err(RepairError::RevisionLimit)
  GAP production                             LINE   crates/store/src/repair.rs:1024: Err(RepairError::RevisionLimit)
  GAP production                             LINE   crates/store/src/repair.rs:1031: return Err(RepairError::IndexedSibling(sibling));
  GAP production                             LINE   crates/store/src/repair.rs:1148: added += 1;
  GAP production                             LINE   crates/store/src/repair.rs:1149: merged.push(*early);
  GAP production                             LINE   crates/store/src/repair.rs:1150: }
  GAP production                             LINE   crates/store/src/repair.rs:1184: return Err(RepairError::ReadBack {
  GAP production                             LINE   crates/store/src/repair.rs:1185: index: index as u64,
  GAP production                             LINE   crates/store/src/repair.rs:1186: });
  GAP production                             LINE   crates/store/src/repair.rs:1206: why @ (RepairError::Conflict
  GAP production                             LINE   crates/store/src/repair.rs:1209: ) => skipped.push((ordinal, why.to_string())),
  GAP production                             LINE   crates/store/src/repair.rs:1210: Err(why) => return Err(why),
  GAP production                             LINE   crates/store/src/repair.rs:1213: Err(RepairError::RevisionLimit)
  GAP production                             LINE   crates/store/src/repair.rs:1228: return;
  GAP production                             LINE   crates/store/src/repair.rs:1374: return Err(RepairError::Conflict);
  GAP production                             LINE   crates/store/src/repair.rs:1392: )?;
  GAP production                             LINE   crates/store/src/repair.rs:1398: )?;
  GAP production                             LINE   crates/store/src/repair.rs:1458: Err(TryLockError::Error(error)) => io_at(Err(error), &path, "lock shared", false),
  GAP production                             BRANCH crates/store/src/repair.rs:581:12 True: if reader.source_header() != expected_source || reader.header().n_valid != kept.len() as u64
  GAP production                             BRANCH crates/store/src/repair.rs:586:16 True: if reader.read_record(index as u64)? != *bar {
  GAP production                             BRANCH crates/store/src/repair.rs:642:9 False: let (Some(first), Some(last)) = (held_back.first(), held_back.last()) else {
  GAP production                             BRANCH crates/store/src/repair.rs:898:32 False: if policy.basis_witness && header.n_valid > 0 && witnessed == 0 {
  GAP production                             BRANCH crates/store/src/repair.rs:1030:12 True: if exists(&sibling)? {
  GAP production                             BRANCH crates/store/src/repair.rs:1147:23 True: while let Some(early) = offer.next_if(|o| o.ts_micros < bar.ts_micros) {
  GAP production                             BRANCH crates/store/src/repair.rs:1165:34 False: if restated_at.is_empty() && added == 0 {
  GAP production                             BRANCH crates/store/src/repair.rs:1183:24 True: if reader.read_record(index as u64)? != *bar {
  GAP production                             BRANCH crates/store/src/repair.rs:1219:9 False: let MonthRestate::Restated {
  GAP production                             BRANCH crates/store/src/repair.rs:1269:44 False: && (held.open_interest == 0 || held.open_interest == crate::format::OI_NULL)
  GAP production                             BRANCH crates/store/src/repair.rs:1271:5 False: held.ts_micros == offered.ts_micros
  GAP production                             BRANCH crates/store/src/repair.rs:1272:12 False: && held.open == offered.open
  GAP production                             BRANCH crates/store/src/repair.rs:1274:12 False: && held.low == offered.low
  GAP production                             BRANCH crates/store/src/repair.rs:1276:12 False: && held.volume == offered.volume
  GAP production                             BRANCH crates/store/src/repair.rs:1373:12 True: if writer.read_record(index as u64)? != *offered {
  F? panic! (failing arm)                    LINE   crates/store/src/repair_tests.rs:116: Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
  F? assert_eq! message argument             LINE   crates/store/src/repair_tests.rs:630: timeframe.as_str()
  GAP test code                              LINE   crates/store/src/repair_tests.rs:700: }
  F? panic! (failing arm)                    LINE   crates/store/src/repair_tests.rs:891: panic!("expected a restated generation, got {done:?}");
  GAP test code                              BRANCH crates/store/src/repair_tests.rs:691:12 False: if let Ok(entries) = fs::read_dir(dir) {
  F? assert! condition branch (failing side) BRANCH crates/store/src/repair_tests.rs:824:94 False: matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
  GAP test code                              BRANCH crates/store/src/repair_tests.rs:882:9 False: let super::MonthRestate::Restated {
  GAP production                             LINE   crates/store/src/safe_open.rs:107: Some(Self::Unknown)
  GAP production                             LINE   crates/store/src/safe_open.rs:300: return Err(io(
  GAP production                             LINE   crates/store/src/safe_open.rs:301: "open",
  GAP production                             LINE   crates/store/src/safe_open.rs:302: io::Error::new(
  GAP production                             LINE   crates/store/src/safe_open.rs:303: ErrorKind::Unsupported,
  GAP production                             LINE   crates/store/src/safe_open.rs:304: "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
  GAP production                             LINE   crates/store/src/safe_open.rs:305: ),
  GAP production                             LINE   crates/store/src/safe_open.rs:306: ));
  GAP production                             BRANCH crates/store/src/safe_open.rs:104:19 False: } else if kind.is_block_device() {
  GAP production                             BRANCH crates/store/src/safe_open.rs:299:9 False: let Some(flags) = FLAGS else {
  GAP test code                              LINE   crates/store/src/safe_open_tests.rs:67: tried.push(format!("{}: {why}", link.display()));
  GAP test code                              LINE   crates/store/src/safe_open_tests.rs:68: continue;
  GAP test code                              LINE   crates/store/src/safe_open_tests.rs:74: Err(why) => tried.push(format!("{}: {why}", link.display())),
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:77: panic!(
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:79: path.display(),
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:80: tried.join("; ")
  F? assert! message argument                LINE   crates/store/src/safe_open_tests.rs:103: found.file_type()
  F? assert! message argument                LINE   crates/store/src/safe_open_tests.rs:110: link.display()
  GAP test code                              LINE   crates/store/src/safe_open_tests.rs:123: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:124: panic!("an open of {} waited for a writer", pipe.display());
  GAP test code                              LINE   crates/store/src/safe_open_tests.rs:272: }
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:401: Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/safe_open_tests.rs:402: Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
  GAP test code                              BRANCH crates/store/src/safe_open_tests.rs:66:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  F? assert! condition branch (failing side) BRANCH crates/store/src/safe_open_tests.rs:95:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  F? let-else in test                        BRANCH crates/store/src/safe_open_tests.rs:122:9 False: let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
  GAP test code                              BRANCH crates/store/src/safe_open_tests.rs:203:12 False: if let Some(device) = a_block_device() {
  GAP test code                              BRANCH crates/store/src/safe_open_tests.rs:268:8 False: if fs::File::open(&path).is_err() {
  F? assert! condition branch (failing side) BRANCH crates/store/src/safe_open_tests.rs:298:16 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  F? assert! condition branch (failing side) BRANCH crates/store/src/safe_open_tests.rs:298:35 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  F? assert! condition branch (failing side) BRANCH crates/store/src/safe_open_tests.rs:299:20 False: && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
  F? assert! condition branch (failing side) BRANCH crates/store/src/safe_open_tests.rs:459:77 False: matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
  GAP production                             BRANCH crates/store/src/sealed.rs:1499:23 False: } else if let Some(range) = self.ranges.get_mut(5) {
  GAP production                             BRANCH crates/store/src/sealed.rs:1504:15 False: while self.table.len() < self.table_rows && self.next_start <= i128::from(ts) {
  GAP production                             BRANCH crates/store/src/sealed.rs:1525:16 False: if let Some(oi) = columns.get_mut(5) {
  GAP production                             BRANCH crates/store/src/sealed.rs:2352:35 False: Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
  F? assert! message argument                LINE   crates/store/src/sealed_codec_tests.rs:81: frame.len()
  F? assert! message argument                LINE   crates/store/src/sealed_codec_tests.rs:282: frame.len()
  F? assert! message argument                LINE   crates/store/src/sealed_codec_tests.rs:439: largest[rows],
  F? assert! message argument                LINE   crates/store/src/sealed_codec_tests.rs:440: max_frame_len(m)
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:159: Ok(value) => panic!("expected a refusal, got {value:?}"),
  GAP test code                              LINE   crates/store/src/sealed_tests.rs:499: Ok(())
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:502: Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:653: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:837: other => panic!("{field}: {other}"),
  GAP test code                              LINE   crates/store/src/sealed_tests.rs:939: fn flush(&mut self) -> io::Result<()> {
  GAP test code                              LINE   crates/store/src/sealed_tests.rs:940: Ok(())
  GAP test code                              LINE   crates/store/src/sealed_tests.rs:941: }
  F? assert! message argument                LINE   crates/store/src/sealed_tests.rs:1170: packed.len(),
  F? assert! message argument                LINE   crates/store/src/sealed_tests.rs:1171: image.len()
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1192: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1241: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1319: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1656: other => panic!("{field}: {other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1673: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1689: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1774: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1802: other => panic!("{other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:1891: other => panic!("{other}"),
  F? assert! message argument                LINE   crates/store/src/sealed_tests.rs:1982: matches!(
  F? panic! (failing arm)                    LINE   crates/store/src/sealed_tests.rs:2104: other => panic!("{other}"),
  GAP test code                              BRANCH crates/store/src/sealed_tests.rs:237:12 False: if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_tests.rs:696:27 False: found == 0 || bars[found as usize - 1].ts_micros < ts,
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_tests.rs:1816:100 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_tests.rs:1822:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_tests.rs:1828:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
  F? assert! condition branch (failing side) BRANCH crates/store/src/sealed_tests.rs:1836:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  GAP production                             LINE   crates/store/src/settings.rs:306: Self::Invalid { key: None, why } => f.write_str(why),
  GAP production                             LINE   crates/store/src/settings.rs:307: Self::Stale { expected, newest } => write!(
  GAP production                             LINE   crates/store/src/settings.rs:308: f,
  GAP production                             LINE   crates/store/src/settings.rs:312: Self::Unchanged => f.write_str("nothing changed: every value is already in force"),
  GAP production                             LINE   crates/store/src/settings.rs:313: Self::Io { path, kind, why } => {
  GAP production                             LINE   crates/store/src/settings.rs:314: write!(f, "the disk refused {}: {kind:?}: {why}", path.display())
  GAP production                             LINE   crates/store/src/settings.rs:427: return None;
  GAP production                             LINE   crates/store/src/settings.rs:796: return Err(format!(
  GAP production                             LINE   crates/store/src/settings.rs:797: "version {version} — this build reads version {VERSION} only"
  GAP production                             LINE   crates/store/src/settings.rs:798: ));
  GAP production                             LINE   crates/store/src/settings.rs:828: return Err(format!("{name} is set twice"));
  GAP production                             LINE   crates/store/src/settings.rs:873: None => refusal.to_string(),
  GAP production                             LINE   crates/store/src/settings.rs:886: return Err(fault(format!(
  GAP production                             LINE   crates/store/src/settings.rs:887: "it grew past the {MAX_FILE_BYTES} bytes a settings file may hold while it was read"
  GAP production                             LINE   crates/store/src/settings.rs:888: )));
  GAP production                             LINE   crates/store/src/settings.rs:1066: self.stopped = Some(format!(
  GAP production                             LINE   crates/store/src/settings.rs:1067: "{MAX_FAULTS} newest generations did not read; no older one was tried"
  GAP production                             LINE   crates/store/src/settings.rs:1068: ));
  GAP production                             LINE   crates/store/src/settings.rs:1069: break;
  GAP production                             LINE   crates/store/src/settings.rs:1114: self.stopped = Some(format!(
  GAP production                             LINE   crates/store/src/settings.rs:1115: "{MAX_STEP} new generations read in one refresh; the rest wait for the next"
  GAP production                             LINE   crates/store/src/settings.rs:1116: ));
  GAP production                             LINE   crates/store/src/settings.rs:1117: break;
  GAP production                             LINE   crates/store/src/settings.rs:1165: fn io_refusal(path: &Path, why: &std::io::Error) -> Refusal {
  GAP production                             LINE   crates/store/src/settings.rs:1166: Refusal::Io {
  GAP production                             LINE   crates/store/src/settings.rs:1167: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/settings.rs:1168: kind: why.kind(),
  GAP production                             LINE   crates/store/src/settings.rs:1169: why: why.to_string(),
  GAP production                             LINE   crates/store/src/settings.rs:1170: }
  GAP production                             LINE   crates/store/src/settings.rs:1171: }
  GAP production                             LINE   crates/store/src/settings.rs:1227: _ => ErrorKind::AlreadyExists,
  GAP production                             LINE   crates/store/src/settings.rs:1235: let _ = std::fs::remove_file(&temp);
  GAP production                             LINE   crates/store/src/settings.rs:1236: return Err(io_refusal(&temp, &why));
  GAP production                             LINE   crates/store/src/settings.rs:1314: path: fault.path,
  GAP production                             LINE   crates/store/src/settings.rs:1315: kind: ErrorKind::Other,
  GAP production                             LINE   crates/store/src/settings.rs:1316: why: format!("the settings directory cannot be listed: {}", fault.why),
  GAP production                             LINE   crates/store/src/settings.rs:1317: })?;
  GAP production                             LINE   crates/store/src/settings.rs:1321: path: fault.path,
  GAP production                             LINE   crates/store/src/settings.rs:1322: kind: ErrorKind::Other,
  GAP production                             LINE   crates/store/src/settings.rs:1323: why: format!("the settings directory cannot be read: {}", fault.why),
  GAP production                             LINE   crates/store/src/settings.rs:1324: })?;
  GAP production                             LINE   crates/store/src/settings.rs:1376: Err(why) => return Err(io_refusal(&target, &why)),
  GAP production                             LINE   crates/store/src/settings.rs:1445: Ok(_) => "it reads when read again, and the refresh did not step to it".to_owned(),
  GAP production                             BRANCH crates/store/src/settings.rs:220:16 False: if let Some(slot) = self.values.get_mut(key.slot()) {
  GAP production                             BRANCH crates/store/src/settings.rs:426:8 True: if digits.len() != 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
  GAP production                             BRANCH crates/store/src/settings.rs:426:30 True: if digits.len() != 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
  GAP production                             BRANCH crates/store/src/settings.rs:464:8 True: if day == 0 || day > last {
  GAP production                             BRANCH crates/store/src/settings.rs:536:16 True: if value.len() > 16 || !value.bytes().all(|b| b.is_ascii_lowercase()) {
  GAP production                             BRANCH crates/store/src/settings.rs:688:30 False: let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
  GAP production                             BRANCH crates/store/src/settings.rs:789:28 True: .or(if rest.is_empty() { Some("") } else { None })
  GAP production                             BRANCH crates/store/src/settings.rs:795:8 True: if version != VERSION.to_string() {
  GAP production                             BRANCH crates/store/src/settings.rs:827:12 True: if settings.get(key).is_some() {
  GAP production                             BRANCH crates/store/src/settings.rs:885:8 True: if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
  GAP production                             BRANCH crates/store/src/settings.rs:1065:20 True: if self.faults.len() >= MAX_FAULTS {
  GAP production                             BRANCH crates/store/src/settings.rs:1113:16 True: if stepped >= MAX_STEP {
  GAP production                             BRANCH crates/store/src/settings.rs:1180:38 True: if text.trim().is_empty() || text.len() > MAX_NOTE_BYTES || !one_line(text) {
  GAP production                             BRANCH crates/store/src/settings.rs:1234:12 True: if let Err(why) = written {
  GAP production                             BRANCH crates/store/src/settings.rs:1369:21 False: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
  GAP production                             BRANCH crates/store/src/settings.rs:1409:16 False: if let Ok(handle) = std::fs::File::open(synced) {
  F? panic! (failing arm)                    LINE   crates/store/src/settings_tests.rs:152: other => panic!("{bad:?} for {key:?}: {other:?}"),
  F? assert! message argument                LINE   crates/store/src/settings_tests.rs:228: matches!(
  GAP test code                              LINE   crates/store/src/settings_tests.rs:261: Ok(())
  F? assert! message argument                LINE   crates/store/src/settings_tests.rs:414: reader.faults()
  F? assert_eq! message argument             LINE   crates/store/src/settings_tests.rs:570: reader.faults()
  F? assert! message argument                LINE   crates/store/src/settings_tests.rs:630: reader.stopped()
  GAP test code                              LINE   crates/store/src/settings_tests.rs:688: if let Some(pipe) = pipe {
  GAP test code                              LINE   crates/store/src/settings_tests.rs:689: let _release = std::fs::OpenOptions::new().write(true).open(pipe);
  GAP test code                              LINE   crates/store/src/settings_tests.rs:690: }
  F? panic! (failing arm)                    LINE   crates/store/src/settings_tests.rs:691: panic!("{what}: the settings read waited instead of naming a fault");
  F? assert! message argument                LINE   crates/store/src/settings_tests.rs:969: matches!(
  GAP test code                              LINE   crates/store/src/settings_tests.rs:1123: return None;
  GAP test code                              BRANCH crates/store/src/settings_tests.rs:255:12 False: if s.get(Key::OpenFeed) == Some("dhan") {
  F? let-else in test                        BRANCH crates/store/src/settings_tests.rs:687:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  GAP test code                              BRANCH crates/store/src/settings_tests.rs:688:16 True: if let Some(pipe) = pipe {
  GAP test code                              BRANCH crates/store/src/settings_tests.rs:688:16 False: if let Some(pipe) = pipe {
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:765:48 False: matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:889:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:935:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:1076:9 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:1076:52 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  GAP test code                              BRANCH crates/store/src/settings_tests.rs:1122:20 True: if started.elapsed() > std::time::Duration::from_secs(10) {
  F? assert! condition branch (failing side) BRANCH crates/store/src/settings_tests.rs:1156:43 False: a.superseded_by == Some(3) || a.in_force == Some(2),
  F? assert! condition branch (failing side) BRANCH crates/store/src/small_month_tests.rs:152:52 False: Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
  GAP production                             LINE   crates/store/src/sort_index.rs:204: pub const fn as_str(self) -> &'static str {
  GAP production                             LINE   crates/store/src/sort_index.rs:205: match self {
  GAP production                             LINE   crates/store/src/sort_index.rs:206: Self::Open => "o",
  GAP production                             LINE   crates/store/src/sort_index.rs:207: Self::High => "h",
  GAP production                             LINE   crates/store/src/sort_index.rs:208: Self::Low => "l",
  GAP production                             LINE   crates/store/src/sort_index.rs:209: Self::Close => "c",
  GAP production                             LINE   crates/store/src/sort_index.rs:210: Self::Volume => "v",
  GAP production                             LINE   crates/store/src/sort_index.rs:211: Self::OpenInterest => "oi",
  GAP production                             LINE   crates/store/src/sort_index.rs:213: }
  GAP production                             LINE   crates/store/src/sort_index.rs:229: pub fn ts(self, origin_micros: i64) -> i64 {
  GAP production                             LINE   crates/store/src/sort_index.rs:230: origin_micros.saturating_add(i64::from(self.sec).saturating_mul(MICROS))
  GAP production                             LINE   crates/store/src/sort_index.rs:231: }
  GAP production                             LINE   crates/store/src/sort_index.rs:297: return Err(Why::Header("its header fails its checksum"));
  GAP production                             LINE   crates/store/src/sort_index.rs:300: return Err(Why::Header("its magic is not BRUTEXS1"));
  GAP production                             LINE   crates/store/src/sort_index.rs:303: return Err(Why::Header("its version is not 1"));
  GAP production                             LINE   crates/store/src/sort_index.rs:551: Self::TooManyRuns(runs) => write!(
  GAP production                             LINE   crates/store/src/sort_index.rs:552: f,
  GAP production                             LINE   crates/store/src/sort_index.rs:560: start,
  GAP production                             LINE   crates/store/src/sort_index.rs:561: column,
  GAP production                             LINE   crates/store/src/sort_index.rs:562: block,
  GAP production                             LINE   crates/store/src/sort_index.rs:563: } => write!(
  GAP production                             LINE   crates/store/src/sort_index.rs:564: f,
  GAP production                             LINE   crates/store/src/sort_index.rs:567: column.as_str()
  GAP production                             LINE   crates/store/src/sort_index.rs:569: Self::Fences { start, column } => write!(
  GAP production                             LINE   crates/store/src/sort_index.rs:570: f,
  GAP production                             LINE   crates/store/src/sort_index.rs:573: column.as_str()
  GAP production                             LINE   crates/store/src/sort_index.rs:580: Self::Outside { row, ts_micros } => {
  GAP production                             LINE   crates/store/src/sort_index.rs:581: write!(f, "bar {row} is stamped {ts_micros}, outside the month")
  GAP production                             LINE   crates/store/src/sort_index.rs:583: Self::Empty => f.write_str("a sort run of no rows is never written"),
  GAP production                             LINE   crates/store/src/sort_index.rs:888: return Err(refuse("its magic is not BRUTEXSR"));
  GAP production                             LINE   crates/store/src/sort_index.rs:897: return Err(refuse("it orders no rows"));
  GAP production                             LINE   crates/store/src/sort_index.rs:914: return Err(refuse("a column's entry width is neither 8 nor 12"));
  GAP production                             LINE   crates/store/src/sort_index.rs:917: return Err(refuse("a column's section is not where its run puts it"));
  GAP production                             LINE   crates/store/src/sort_index.rs:920: return Err(refuse("a column counts nulls it cannot hold"));
  GAP production                             LINE   crates/store/src/sort_index.rs:923: return Err(refuse("a column's largest value is below its smallest"));
  GAP production                             LINE   crates/store/src/sort_index.rs:1177: pub fn weight(&self) -> usize {
  GAP production                             LINE   crates/store/src/sort_index.rs:1178: self.entries.len() * std::mem::size_of::<Entry>() + self.crcs.len() * 4
  GAP production                             LINE   crates/store/src/sort_index.rs:1179: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1216: Err(why) => {
  GAP production                             LINE   crates/store/src/sort_index.rs:1224: if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
  GAP production                             LINE   crates/store/src/sort_index.rs:1225: return Err(Why::Unreadable(format!(
  GAP production                             LINE   crates/store/src/sort_index.rs:1226: "{} is a symbolic link; no link is followed",
  GAP production                             LINE   crates/store/src/sort_index.rs:1227: path.display()
  GAP production                             LINE   crates/store/src/sort_index.rs:1228: )));
  GAP production                             LINE   crates/store/src/sort_index.rs:1229: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1230: Err(Why::Unreadable(format!("{}: {why}", path.display())))
  GAP production                             LINE   crates/store/src/sort_index.rs:1270: pub fn open_walked(path: &Path, shape: &Shape, walked: Walked) -> Result<Self, Why> {
  GAP production                             LINE   crates/store/src/sort_index.rs:1271: let file = open_no_follow(path)?;
  GAP production                             LINE   crates/store/src/sort_index.rs:1272: let meta = file
  GAP production                             LINE   crates/store/src/sort_index.rs:1273: .metadata()
  GAP production                             LINE   crates/store/src/sort_index.rs:1274: .map_err(|why| Why::Unreadable(format!("{}: {why}", path.display())))?;
  GAP production                             LINE   crates/store/src/sort_index.rs:1275: if !meta.is_file() || meta.len() < walked.end {
  GAP production                             LINE   crates/store/src/sort_index.rs:1276: return Err(Why::Unreadable(format!(
  GAP production                             LINE   crates/store/src/sort_index.rs:1277: "{} is not the file that was walked",
  GAP production                             LINE   crates/store/src/sort_index.rs:1278: path.display()
  GAP production                             LINE   crates/store/src/sort_index.rs:1279: )));
  GAP production                             LINE   crates/store/src/sort_index.rs:1280: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1281: Ok(Self {
  GAP production                             LINE   crates/store/src/sort_index.rs:1282: file,
  GAP production                             LINE   crates/store/src/sort_index.rs:1283: path: path.to_path_buf(),
  GAP production                             LINE   crates/store/src/sort_index.rs:1284: shape: *shape,
  GAP production                             LINE   crates/store/src/sort_index.rs:1285: walked,
  GAP production                             LINE   crates/store/src/sort_index.rs:1286: })
  GAP production                             LINE   crates/store/src/sort_index.rs:1287: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1328: pub fn path(&self) -> &Path {
  GAP production                             LINE   crates/store/src/sort_index.rs:1329: &self.path
  GAP production                             LINE   crates/store/src/sort_index.rs:1330: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1334: pub const fn shape(&self) -> &Shape {
  GAP production                             LINE   crates/store/src/sort_index.rs:1335: &self.shape
  GAP production                             LINE   crates/store/src/sort_index.rs:1336: }
  GAP production                             LINE   crates/store/src/sort_index.rs:1414: return Err(refuse());
  GAP production                             BRANCH crates/store/src/sort_index.rs:296:12 True: if crc.to_le_bytes() != take::<4>(bytes, 60) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:299:12 True: if take::<8>(bytes, 0) != MAGIC {
  GAP production                             BRANCH crates/store/src/sort_index.rs:302:12 True: if u16::from_le_bytes(take::<2>(bytes, 8)) != VERSION {
  GAP production                             BRANCH crates/store/src/sort_index.rs:792:16 False: if let Some(slot) = columns.get_mut(column.index()) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:887:8 True: if take::<8>(bytes, 0) != RUN_MAGIC {
  GAP production                             BRANCH crates/store/src/sort_index.rs:896:8 True: if end <= start {
  GAP production                             BRANCH crates/store/src/sort_index.rs:913:36 True: if info.width != NARROW && info.width != WIDE {
  GAP production                             BRANCH crates/store/src/sort_index.rs:916:12 True: if info.blocks != blocks || info.offset != next {
  GAP production                             BRANCH crates/store/src/sort_index.rs:916:37 True: if info.blocks != blocks || info.offset != next {
  GAP production                             BRANCH crates/store/src/sort_index.rs:919:12 True: if info.nulls > rows || (info.nulls > 0 && !column.nullable()) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:919:52 True: if info.nulls > rows || (info.nulls > 0 && !column.nullable()) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:922:12 True: if info.max < info.base {
  GAP production                             BRANCH crates/store/src/sort_index.rs:928:16 False: if let Some(slot) = columns.get_mut(column.index()) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1215:21 False: Err(why) if why.kind() == ErrorKind::NotFound => Err(Why::Absent),
  GAP production                             BRANCH crates/store/src/sort_index.rs:1224:16 True: if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1224:16 False: if why.raw_os_error() == Some(crate::open_flags::ELOOP) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1275:12 True: if !meta.is_file() || meta.len() < walked.end {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1275:12 False: if !meta.is_file() || meta.len() < walked.end {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1275:31 True: if !meta.is_file() || meta.len() < walked.end {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1275:31 False: if !meta.is_file() || meta.len() < walked.end {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1413:12 True: if block >= info.blocks {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1520:12 False: if let Some(src) = bytes.get(at..at + N) {
  GAP production                             BRANCH crates/store/src/sort_index.rs:1529:12 False: if let Some(dst) = out.get_mut(at..at + src.len()) {
  F? assert! condition branch (failing side) BRANCH crates/store/src/sort_index_tests.rs:324:41 False: assert!(why.contains("kind") || why.contains("reserved"), "{why}");
  F? assert_eq! message argument             LINE   crates/store/src/staging_tests.rs:421: within.display()
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:143:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:143:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:206:67 False: matches!(&refusal, StagingError::Contended { path: p } if *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:238:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:238:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:297:78 False: matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:334:88 False: matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
  F? assert! condition branch (failing side) BRANCH crates/store/src/staging_tests.rs:369:77 False: matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:921:21 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:921:41 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1057:17 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1057:38 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1057:59 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1058:17 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1058:39 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1105:17 False: assert!(!daily.exact() && g.exact());
  F? assert! condition branch (failing side) BRANCH crates/store/src/time_index.rs:1105:35 False: assert!(!daily.exact() && g.exact());
  GAP production                             LINE   crates/store/src/verdict.rs:717: return None;
  GAP production                             LINE   crates/store/src/verdict.rs:3924: return Err(VerdictError::ImageMissing { path });
  GAP production                             LINE   crates/store/src/verdict.rs:4076: return Err(VerdictError::ImageMissing { path });
  GAP production                             LINE   crates/store/src/verdict.rs:4137: return Err(VerdictError::ImageMissing { path });
  F? panic! (failing arm)                    LINE   crates/store/src/verdict.rs:4219: other => panic!("not image_corrupt: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/src/verdict.rs:4227: other => panic!("not written_by_newer_build: {other:?}"),
  GAP production                             BRANCH crates/store/src/verdict.rs:716:8 True: if Vendor::ALL.get(at) != Some(&feed) {
  GAP production                             BRANCH crates/store/src/verdict.rs:2278:16 False: if let Some(slot) = out.first.get_mut(ordinal) {
  GAP production                             BRANCH crates/store/src/verdict.rs:2312:16 False: if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
  GAP production                             BRANCH crates/store/src/verdict.rs:2478:16 False: if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
  GAP production                             BRANCH crates/store/src/verdict.rs:3143:15 False: while let Some(parent) = at.parent() {
  GAP production                             BRANCH crates/store/src/verdict.rs:3417:12 False: if let Some(entry) = days.get_mut(doy) {
  GAP production                             BRANCH crates/store/src/verdict.rs:3853:24 False: && let Some(oldest) = self
  GAP production                             BRANCH crates/store/src/verdict.rs:3923:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP production                             BRANCH crates/store/src/verdict.rs:4075:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP production                             BRANCH crates/store/src/verdict.rs:4136:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
  GAP test code                              BRANCH crates/store/src/verdict.rs:4637:25 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP test code                              BRANCH crates/store/src/verdict.rs:4637:37 True: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP test code                              BRANCH crates/store/src/verdict.rs:4637:37 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  GAP test code                              BRANCH crates/store/src/verdict.rs:4974:24 False: .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
  F? panic! (failing arm)                    LINE   crates/store/src/verdict_tests.rs:366: other => panic!("expected the host's refusal, got {other}"),
  GAP test code                              LINE   crates/store/src/verdict_tests.rs:461: let _release = fs::OpenOptions::new().write(true).open(pipe);
  F? panic! (failing arm)                    LINE   crates/store/src/verdict_tests.rs:462: panic!("a verdict call waited on the pipe at {}", pipe.display());
  F? panic! (failing arm)                    LINE   crates/store/src/verdict_tests.rs:568: other => panic!("expected the host's refusal, got {other}"),
  F? panic! (failing arm)                    LINE   crates/store/src/verdict_tests.rs:813: other => panic!("not image_corrupt: {other:?}"),
  F? assert_eq! message argument             LINE   crates/store/src/verdict_tests.rs:879: other.dir().display()
  F? panic! (failing arm)                    LINE   crates/store/src/verdict_tests.rs:951: panic!("the key's record: {:?}", found.found);
  F? let-else in test                        BRANCH crates/store/src/verdict_tests.rs:460:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  F? let-else in test                        BRANCH crates/store/src/verdict_tests.rs:950:9 False: let super::Found::Record { ordinal, record } = found.found else {
  GAP test code                              LINE   crates/store/tests/catalog.rs:309: Ok(()) => {}
  F? panic! (failing arm)                    LINE   crates/store/tests/catalog.rs:313: Err(why) => panic!("{} could not be made: {why}", full.display()),
  GAP test code                              LINE   crates/store/tests/catalog.rs:815: tried.push(format!("{}: {why}", link.display()));
  GAP test code                              LINE   crates/store/tests/catalog.rs:816: continue;
  GAP test code                              LINE   crates/store/tests/catalog.rs:822: Err(why) => tried.push(format!("{}: {why}", link.display())),
  F? panic! (failing arm)                    LINE   crates/store/tests/catalog.rs:825: panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/catalog.rs:827: path.display(),
  F? panic! (failing arm)                    LINE   crates/store/tests/catalog.rs:828: tried.join("; ")
  F? assert! message argument                LINE   crates/store/tests/catalog.rs:852: found.file_type()
  F? assert! message argument                LINE   crates/store/tests/catalog.rs:859: link.display()
  F? assert! message argument                LINE   crates/store/tests/catalog.rs:875: path.display()
  GAP test code                              BRANCH crates/store/tests/catalog.rs:68:12 False: if let Some(parent) = full.parent() {
  GAP test code                              BRANCH crates/store/tests/catalog.rs:310:54 False: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/catalog.rs:317:9 True: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/catalog.rs:317:39 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  GAP test code                              BRANCH crates/store/tests/catalog.rs:320:8 False: if refused_by_the_volume > 0 {
  GAP test code                              BRANCH crates/store/tests/catalog.rs:814:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/catalog.rs:844:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:289: println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:290: return false;
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:568: faults.push(format!(
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:571: continue;
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:575: faults.push(format!(
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:576: "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:577: at.name, at.sentence
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:578: ));
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:579: }
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:602: let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:603: faults.push(format!(
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:611: faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:614: faults.push(format!(
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:615: "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:616: ));
  F? assert! message argument                LINE   crates/store/tests/cited_commits.rs:673: cited.len()
  F? assert! message argument                LINE   crates/store/tests/cited_commits.rs:709: String::from_utf8_lossy(&out.stderr)
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:732: println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:733: return;
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:820: println!("SKIPPING: git cannot run the synthetic history fixture");
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:821: return;
  F? assert! message argument                LINE   crates/store/tests/cited_commits.rs:976: sites.len()
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:979: return;
  GAP test code                              LINE   crates/store/tests/cited_commits.rs:997: wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:288:12 True: if let Err(why) = Command::new("git").arg("--version").output() {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:294:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:306:12 False: if let Some(above) = root.parent() {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cited_commits.rs:371:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cited_commits.rs:372:16 False: && said.contains("not a git repository"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cited_commits.rs:384:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cited_commits.rs:385:16 False: && said.contains("not a git repository"),
  F? let-else in test                        BRANCH crates/store/tests/cited_commits.rs:567:13 False: let Some(citing) = cited.get(commit) else {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:601:16 True: if !is_ancestor_at(root, commit, main_ref) {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:610:12 True: if is_ancestor_at(root, commit, main_ref) {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:613:35 True: if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:676:8 False: if history_is_here() {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:731:8 True: if Command::new("git").arg("--version").output().is_err() {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:819:8 True: if Command::new("git").arg("--version").output().is_err() {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:898:39 False: } else if token.len() == 2 && token.starts_with('v') {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:903:19 False: } else if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:978:8 True: if !history_is_here() {
  GAP test code                              BRANCH crates/store/tests/cited_commits.rs:996:12 True: if !takes_a_lock(line) {
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:367: other => panic!("{door}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:371: panic!("a moved revision is audited");
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:446: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:1073: other => panic!("not image_corrupt: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:1098: panic!("ZZB is in the day");
  F? assert! message argument                LINE   crates/store/tests/cloud_mutants_store.rs:1171: matches!(
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:1361: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:1568: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/cloud_mutants_store.rs:1925: panic!("{name} is in the day");
  GAP test code                              LINE   crates/store/tests/cloud_mutants_store.rs:2385: return Err(std::io::ErrorKind::UnexpectedEof.into());
  F? assert! message argument                LINE   crates/store/tests/cloud_mutants_store.rs:2876: writer.time_lookup()
  GAP test code                              LINE   crates/store/tests/cloud_mutants_store.rs:3172: return Err(std::io::Error::other("flushed alone"));
  GAP test code                              LINE   crates/store/tests/cloud_mutants_store.rs:3253: _ => None,
  GAP test code                              BRANCH crates/store/tests/cloud_mutants_store.rs:144:29 True: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
  GAP test code                              BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False: vanished >= 20 && contended >= 20,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False: vanished >= 20 && contended >= 20,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False: } if source.kind() == std::io::ErrorKind::PermissionDenied
  F? let-else in test                        BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False: let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False: audited.starts_with("repair revision 1 of ")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False: && audited.ends_with(" is not the one its promotion recorded; not audited"),
  F? let-else in test                        BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False: let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False: generation.contains("/nonexistent/cloud/2031-02.tix")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False: && generation.contains("revision 1 is not the one its promotion recorded"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False: if path == &lock_path && *tries == steps + 1
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False: if path == &lock_path && *tries == steps + 1
  GAP test code                              BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False: let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False: if path.parent() == Some(year.as_path())
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False: && path.extension().is_some_and(|ext| ext == "tmp")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False: && source.kind() == std::io::ErrorKind::PermissionDenied
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False: TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  F? let-else in test                        BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False: let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False: if *path == bar_path
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False: TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False: StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  GAP test code                              BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True: if since.elapsed() > std::time::Duration::from_secs(5) {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  GAP test code                              LINE   crates/store/tests/docs.rs:226: }
  GAP test code                              LINE   crates/store/tests/docs.rs:227: let entry = DECISIONS_DOC
  GAP test code                              LINE   crates/store/tests/docs.rs:228: .split("\n### D-0790 ")
  GAP test code                              LINE   crates/store/tests/docs.rs:229: .nth(1)
  GAP test code                              LINE   crates/store/tests/docs.rs:230: .and_then(|rest| rest.split("\n### ").next())
  GAP test code                              LINE   crates/store/tests/docs.rs:231: .map(flat)
  GAP test code                              LINE   crates/store/tests/docs.rs:232: .expect("docs/05-decisions.md has a D-0790 entry");
  GAP test code                              LINE   crates/store/tests/docs.rs:233: assert!(
  F? assert! message argument                LINE   crates/store/tests/docs.rs:234: entry.contains(
  F? assert! message argument                LINE   crates/store/tests/docs.rs:235: "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
  F? assert! message argument                LINE   crates/store/tests/docs.rs:236: ) && entry.contains("this entry supersedes that sentence"),
  GAP test code                              LINE   crates/store/tests/docs.rs:239: assert!(
  F? assert! message argument                LINE   crates/store/tests/docs.rs:240: INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:217:9 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:217:47 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  GAP test code                              BRANCH crates/store/tests/docs.rs:224:8 False: if !flat(INVARIANTS_DOC).contains("no bench in this repository times a syscall") {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:234:9 True: entry.contains(
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:234:9 False: entry.contains(
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:236:14 True: ) && entry.contains("this entry supersedes that sentence"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:236:14 False: ) && entry.contains("this entry supersedes that sentence"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:253:9 False: FILE_RS.contains("if claimed > header.n_valid {")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/docs.rs:254:16 False: && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
  F? panic! (failing arm)                    LINE   crates/store/tests/durability.rs:381: panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/fault.rs:376: panic!("bit {bit} of byte {byte} took the whole file down: {e}")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/fault.rs:143:36 False: read.n_valid == 150 || read.n_valid == 175,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/fault.rs:220:59 False: horizon == previous_commit.durable_through || horizon == commit.durable_through,
  F? assert! message argument                LINE   crates/store/tests/fifo.rs:108: at.display()
  GAP test code                              LINE   crates/store/tests/fifo.rs:124: }
  GAP test code                              LINE   crates/store/tests/fifo.rs:126: drop(fs::OpenOptions::new().write(true).open(release));
  GAP test code                              LINE   crates/store/tests/fifo.rs:127: drop(worker.join());
  F? panic! (failing arm)                    LINE   crates/store/tests/fifo.rs:128: panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/fifo.rs:130: release.display()
  GAP test code                              LINE   crates/store/tests/fifo.rs:213: return;
  GAP test code                              BRANCH crates/store/tests/fifo.rs:121:12 False: if let Ok(outcome) = recv.recv_timeout(BOUND) {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/fifo.rs:157:13 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/fifo.rs:157:61 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/fifo.rs:162:53 False: fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
  GAP test code                              BRANCH crates/store/tests/fifo.rs:212:8 True: if !null.exists() {
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:165: panic!("{done:?}");
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:217: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:238: other => panic!("{other:?}"),
  F? assert! message argument                LINE   crates/store/tests/generation.rs:331: matches!(
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:367: panic!("repaired");
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:388: panic!("{second:?}");
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:475: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:517: other => panic!("{other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/generation.rs:758: panic!("{repaired:?}");
  GAP test code                              BRANCH crates/store/tests/generation.rs:153:9 False: let MonthRepair::Repaired {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:179:57 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:179:76 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  GAP test code                              BRANCH crates/store/tests/generation.rs:364:9 False: let MonthRepair::Repaired { to, skipped, .. } =
  GAP test code                              BRANCH crates/store/tests/generation.rs:384:9 False: let MonthRepair::Repaired {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:670:13 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:670:42 True: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:670:42 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/generation.rs:694:74 False: matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
  GAP test code                              BRANCH crates/store/tests/generation.rs:749:9 False: let MonthRepair::Repaired {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/geometry.rs:99:13 False: start >= block_start && end <= block_end,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/geometry.rs:99:37 False: start >= block_start && end <= block_end,
  GAP test code                              BRANCH crates/store/tests/geometry.rs:134:13 True: start < block_start || end > block_end
  GAP test code                              LINE   crates/store/tests/libm_key.rs:205: matches!(
  F? assert_eq! message argument             LINE   crates/store/tests/libm_key.rs:1330: if mentions { "is not" } else { "is" }
  F? assert! condition branch (failing side) BRANCH crates/store/tests/libm_key.rs:428:9 False: decision < caveat && caveat < rejected,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/libm_key.rs:428:30 False: decision < caveat && caveat < rejected,
  F? assert_eq! condition branch (failing side) BRANCH crates/store/tests/libm_key.rs:1330:16 True: if mentions { "is not" } else { "is" }
  F? assert_eq! condition branch (failing side) BRANCH crates/store/tests/libm_key.rs:1330:16 False: if mentions { "is not" } else { "is" }
  F? panic! (failing arm)                    LINE   crates/store/tests/misfiled.rs:184: other => panic!("expected Misfiled, found {other:?}"),
  F? assert_eq! message argument             LINE   crates/store/tests/misfiled.rs:321: unborn.len()
  GAP test code                              BRANCH crates/store/tests/misfiled.rs:125:16 True: let y = if month <= 2 { year - 1 } else { year };
  GAP test code                              BRANCH crates/store/tests/misfiled.rs:129:26 False: let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i32::from(day) - 1;
  F? assert! condition branch (failing side) BRANCH crates/store/tests/misfiled.rs:217:9 False: said.contains("2019-11-28-1250000-CE")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/misfiled.rs:218:16 False: && said.contains("its store path is not this month's")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/misfiled.rs:219:16 False: && said.contains("DB-B65"),
  F? panic! (failing arm)                    LINE   crates/store/tests/month_repair.rs:156: other => panic!("not a repair: {other:?}"),
  F? assert_eq! message argument             LINE   crates/store/tests/month_repair.rs:324: timeframe.as_str()
  GAP test code                              LINE   crates/store/tests/month_repair.rs:637: calls += 1;
  GAP test code                              LINE   crates/store/tests/month_repair.rs:638: Ok(())
  GAP test code                              LINE   crates/store/tests/month_repair.rs:639: };
  F? panic! (failing arm)                    LINE   crates/store/tests/month_repair.rs:856: panic!("{outcome:?}");
  F? assert! message argument                LINE   crates/store/tests/month_repair.rs:918: left.keys()
  F? panic! (failing arm)                    LINE   crates/store/tests/month_repair.rs:944: other => panic!("not a repair: {other:?}"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/month_repair.rs:606:13 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/month_repair.rs:606:47 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/month_repair.rs:739:83 False: matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
  GAP test code                              BRANCH crates/store/tests/month_repair.rs:852:9 False: let Ok(MonthRepair::Repaired {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/month_repair.rs:879:38 False: Served::Revision(pointer) if pointer.revision.ordinal() == 3
  GAP test code                              LINE   crates/store/tests/month_summary_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  GAP test code                              LINE   crates/store/tests/month_summary_log.rs:75: other => format!("{other:?}"),
  GAP test code                              LINE   crates/store/tests/open_flags.rs:176: offenders.push(format!("{}:{}", path.display(), number + 1));
  GAP test code                              BRANCH crates/store/tests/open_flags.rs:175:16 True: if names_the_x86_bit(line) {
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:66: other => panic!("expected a sidecar, found {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:161: Recorded::NothingAppended => panic!("expected a record"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:451: other => panic!("expected NotTheNewestCommit, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:464: other => panic!("expected NotABarPath, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:476: other => panic!("expected WrongBarFile, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:531: other => panic!("expected OutOfOrder, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:639: other => panic!("expected Damaged, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:698: result => panic!("expected a {field} mismatch, got {result:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:746: other => panic!("expected PastTheBars, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1200: other => panic!("expected {fault:?}, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1308: other => panic!("expected TooLarge, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1409: other => panic!("expected Record, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1611: other => panic!("expected NotABarPath, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1617: other => panic!("expected NotABarPath, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1671: other => panic!("expected the directory sync refused, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/provenance.rs:1721: other => panic!("expected the open refused, got {other:?}"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/repair.rs:506:13 True: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/repair.rs:506:41 False: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_alloc.rs:115: panic!("a frame cut to {len} bytes left {frame_answer:?}");
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_alloc.rs:125: full.len()
  F? let-else in test                        BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False: let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False: !text.contains("by its checked rows and no more")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False: && !text.contains("checked rows times 56 bytes, once"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False: text.contains("the largest of twice") && text.contains("and 4"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False: text.contains("the largest of twice") && text.contains("and 4"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False: text.contains("capacity 0") && text.contains("cleared"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False: text.contains("capacity 0") && text.contains("cleared"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False: !text.contains("random fields cost at most 57 bytes a bar")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False: && !text.contains("at most 57 bytes a bar, 1 more than raw"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:65: fn read_month(path: &Path) -> Vec<Bar> {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:68: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:69: path.extension().is_some_and(|e| e == "bin"),
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:71: path.display()
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:73: let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:74: let header = data.get(..RECORDS_AT).unwrap_or_else(|| {
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:75: panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:77: path.display(),
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:78: data.len()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:81: let header = Header::read_region(header, data.len() as u64)
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:82: .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:83: let n = header.n_valid as usize;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:84: data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:85: .unwrap_or_else(|| {
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:86: panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_codec_samples.rs:88: path.display()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:91: .chunks_exact(STRIDE)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:92: .map(|record| Bar::decode(record).expect("a committed record decodes"))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:93: .collect()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:94: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:97: fn origin_of(name: &str) -> i64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:98: let stem = name.trim_end_matches(".bin");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:99: let month = &stem[stem.len() - 7..];
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:100: let year: u16 = month[..4].parse().expect("year");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:101: let mm: u8 = month[5..].parse().expect("month");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:102: YearMonth::new(year, mm)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:103: .expect("a month")
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:104: .ist_bounds_micros()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:105: .0
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:106: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:111: fn judge_row0(rows: &[Bar]) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:112: fn len(mut v: u64) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:113: let mut n = 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:114: while v >= 0x80 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:115: v >>= 7;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:116: n += 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:117: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:118: n
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:119: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:120: fn zz(x: i64) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:121: ((x << 1) ^ (x >> 63)).cast_unsigned()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:122: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:123: let first = rows.first().map_or(0, |b| b.ts_micros);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:124: rows.iter()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:125: .step_by(256)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:126: .map(|b| {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:127: let d = b.ts_micros - first;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:128: let ts = if d % 1_000_000 == 0 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:129: len(zz(d / 1_000_000))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:131: len(zz(d))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:133: let oi = if b.open_interest == OI_NULL {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:134: 1
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:136: 1 + len(zz(b.open_interest))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:138: ts + 1
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:139: + len(zz(b.close))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:140: + len(zz(b.open - b.close))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:141: + len(zz(b.high - b.close))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:142: + len(zz(b.low - b.close))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:143: + len(zz(b.volume))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:144: + oi
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:145: })
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:146: .sum()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:147: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:150: const fn zz(x: i64) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:151: ((x << 1) ^ (x >> 63)).cast_unsigned()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:152: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:154: const fn varint_len(value: u64) -> usize {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:155: let bits = (64 - value.leading_zeros()) as usize;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:156: if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:157: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:160: fn row0_len(first: &Bar, origin: i64) -> usize {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:161: let d = first.ts_micros.wrapping_sub(origin);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:162: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:163: let oi = if first.open_interest == OI_NULL {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:164: 0
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:166: varint_len(zz(first.open_interest))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:168: varint_len(zz(stamp))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:169: + varint_len(zz(first.close))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:170: + varint_len(zz(first.open.wrapping_sub(first.close)))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:171: + varint_len(zz(first.high.wrapping_sub(first.close)))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:172: + varint_len(zz(first.low.wrapping_sub(first.close)))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:173: + varint_len(zz(first.volume))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:174: + oi
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:175: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:179: fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:180: rows.windows(2)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:181: .map(|pair| {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:182: let (p, b) = (pair[0], pair[1]);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:183: let (value, predictor) = match (column, mode) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:184: (0, _) => (b.ts_micros, p.ts_micros),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:185: (1, _) => (b.close, p.close),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:186: (2, 0) => (b.open, p.close),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:187: (2, _) => (b.open, b.close),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:188: (3, _) => (b.high, b.open.max(b.close)),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:189: (4, _) => (b.low, b.open.min(b.close)),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:190: (5, 0) => (b.volume, 0),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:191: (5, _) => (b.volume, p.volume),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:192: (6, 0) => (b.open_interest, p.open_interest),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:193: _ => (b.open_interest, 0),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:195: value.wrapping_sub(predictor)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:196: })
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:197: .collect()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:198: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:205: fn prototype_stream_len(residuals: &[i64]) -> usize {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:206: fn gcd(mut a: u64, mut b: u64) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:207: while b != 0 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:208: (a, b) = (b, a % b);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:209: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:210: a
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:211: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:212: let n = residuals.len();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:213: let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:214: if g == 0 || g >= 1 << 62 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:215: g = 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:216: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:217: let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:218: let base = values.iter().copied().min().unwrap_or(0);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:219: let mut histogram = [0usize; 65];
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:220: for v in &values {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:221: histogram[(64 - (v - base).leading_zeros()) as usize] += 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:222: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:223: let mut top = 64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:224: while top > 0 && histogram[top] == 0 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:225: top -= 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:226: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:227: let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:228: let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:229: while w > 0 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:230: exceptions += histogram[w];
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:231: w -= 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:232: let cost = n * w + exceptions * (p + top) + 16;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:233: if cost < best {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:234: best = cost;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:235: best_w = w;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:236: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:238: let limit = if best_w == 64 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:239: u64::MAX
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:241: (1 << best_w) - 1
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:243: let exceptions = values.iter().filter(|&&v| v - base > limit).count();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:244: let mut len = 1
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:245: + varint_len(exceptions as u64)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:246: + varint_len(base)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:247: + varint_len(g)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:248: + (n * best_w).div_ceil(8);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:249: if exceptions > 0 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:250: len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:251: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:252: len
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:253: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:256: fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:257: let streams: usize = (0..7)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:258: .map(|column| {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:259: let zero = prototype_stream_len(&residuals(rows, column, 0));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:260: if matches!(column, 2 | 5 | 6) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:261: zero.min(prototype_stream_len(&residuals(rows, column, 1)))
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:263: zero
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:265: })
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:266: .sum();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:267: 4 + row0_len(&rows[0], origin) + streams
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:268: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:281: fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:282: let mut frames = Vec::with_capacity(rows.len().div_ceil(256));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:283: let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:284: for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:285: let rule = prototype_frame_len(chunk, origin);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:286: first_rows += row0_len(&chunk[0], origin) as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:287: let mut frame = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:288: let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:289: assert!(frame.len() <= MAX_FRAME_LEN, "{label}");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:290: assert_eq!(encoded.crc, crc32c(&frame));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:291: let mut back = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:292: decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:293: assert_eq!(back, chunk, "{label}: a frame came back different");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:294: let mut stamps = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:295: decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:296: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:297: stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:301: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:302: frame.len() <= rule,
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:304: frame.len()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:306: bytes += frame.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:307: prototype_rule += rule as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:308: frames.push((frame, encoded.crc));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:310: Sealed {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:311: bytes,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:312: prototype_rule,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:313: row0: first_rows,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:314: frames,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:315: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:316: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:318: fn percentiles(label: &str, mut ns: Vec<u64>) -> String {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:319: ns.sort_unstable();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:320: let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:321: format!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:323: ns.len(),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:324: at(50),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:325: at(99),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:326: ns.last().copied().unwrap_or(0)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:328: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:331: fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:332: let mut decode = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:333: let mut stamps = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:334: let mut out = Vec::with_capacity(512);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:335: let mut ts = Vec::with_capacity(256);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:336: for _ in 0..(2_000 / sealed.frames.len()).max(1) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:337: for (frame, crc) in &sealed.frames {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:338: out.clear();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:339: let t = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:340: decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:341: decode.push(t.elapsed().as_nanos() as u64);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:342: ts.clear();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:343: let t = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:344: decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:345: stamps.push(t.elapsed().as_nanos() as u64);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:346: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:348: let mut encode = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:349: let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:350: for chunk in rows.chunks(256).take(2_000) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:351: buffer.clear();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:352: let t = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:353: encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:354: encode.push(t.elapsed().as_nanos() as u64);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:355: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:357: let mut page = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:358: let mut state = 0x9E37_79B9_7F4A_7C15_u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:359: let n = rows.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:360: for _ in 0..2_000 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:361: state ^= state << 13;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:362: state ^= state >> 7;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:363: state ^= state << 17;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:364: let first = state % n;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:365: let last = (first + 99).min(n - 1);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:366: out.clear();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:367: let t = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:368: for k in (first / 256)..=(last / 256) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:369: let (frame, crc) = &sealed.frames[k as usize];
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:370: decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:371: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:372: page.push(t.elapsed().as_nanos() as u64);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:373: let skip = (first % 256) as usize;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:374: assert_eq!(out[skip], rows[first as usize]);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:376: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:378: percentiles(&format!("{label} frame decode+crc"), decode)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:380: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:382: percentiles(&format!("{label} frame stamps+crc"), stamps)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:384: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:386: percentiles(&format!("{label} frame encode"), encode)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:388: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:390: percentiles(&format!("{label} page100 from memory"), page)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:392: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:409: fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:410: if sealed.prototype_rule != exact {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:411: self.not_exact.push(format!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:412: "{label}: rule {} exact {exact}",
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:413: sealed.prototype_rule
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:414: ));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:415: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:416: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:417: self.larger
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:418: .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:419: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:420: self.codec1 += sealed.bytes;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:421: self.rule += sealed.prototype_rule;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:422: self.exact += exact;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:423: self.judge += judge;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:424: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:426: fn check(&self, label: &str) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:427: let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:428: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:434: d(self.codec1, self.exact),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:435: d(self.codec1, self.judge),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:436: self.not_exact.len(),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:437: self.larger.len()
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:440: assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:441: assert_eq!(self.rule, self.exact, "{label}");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:443: assert!(self.codec1 <= self.rule, "{label}");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:444: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:445: within(self.codec1, self.exact).0,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:450: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_codec_samples.rs:451: within(self.codec1, self.judge).0,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:454: assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:455: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:459: fn per_cent(diff: i64, of: u64) -> String {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:460: let thousandths = diff.unsigned_abs() * 100_000 / of;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:461: let sign = if diff < 0 { "-" } else { "+" };
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:462: format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:463: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:466: fn within(mine: u64, expected: u64) -> (bool, i64) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:467: let diff = mine.cast_signed() - expected.cast_signed();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:469: (diff.unsigned_abs() * 200 <= expected, diff)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:470: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:474: fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:475: let root = std::env::var("BRUTEX_SEAL_SAMPLES")
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:476: .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:477: let root = Path::new(&root);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:488: println!("eight sample months:");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:489: let mut totals = Totals::default();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:490: for (dir, month, body, saved) in SAMPLES {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:491: let path = root
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:492: .join("blocks/samples")
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:493: .join(dir)
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:494: .join(format!("{month}.bin"));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:495: let rows = read_month(&path);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:496: let origin = origin_of(month);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:497: let sealed = seal(&rows, origin, dir);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:498: let blocks = sealed.frames.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:499: let exact = body - 56 * blocks + sealed.row0;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:500: let judge = body - saved;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:501: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:502: "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:503: rows.len(),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:504: sealed.bytes,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:505: sealed.prototype_rule,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:506: per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:507: rows.len() * 56,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:508: sealed.bytes * 100 / rows.len() as u64,
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:509: );
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:510: time(&rows, origin, &sealed, dir);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:511: totals.add(dir, rows.len(), &sealed, exact, judge);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:512: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:513: totals.check("eight samples in all");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:515: let manifest =
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:516: std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:517: let out =
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:518: std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:519: let mut totals = Totals::default();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:520: let (mut months, mut bars, mut empty) = (0, 0, 0);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:521: for (line, result) in manifest.lines().zip(out.lines()) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:522: let fields: Vec<&str> = line.split('\t').collect();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:523: let measured: Vec<&str> = result.split('\t').collect();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:524: let rows = read_month(&root.join("blocks").join(fields[0]));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:525: assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:526: if rows.is_empty() {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:527: empty += 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:528: continue;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:529: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:530: let origin = origin_of(fields[4]);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:531: let sealed = seal(&rows, origin, fields[4]);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:532: let body: u64 = measured[3].parse().expect("body");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:533: let blocks = sealed.frames.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:534: let exact = body - 56 * blocks + sealed.row0;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:535: let judge = body - (56 * blocks - judge_row0(&rows));
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:536: totals.add(fields[4], rows.len(), &sealed, exact, judge);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:537: months += 1;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:538: bars += rows.len();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:540: println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:541: totals.check("the manifest in all");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:542: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:543: println!("extra months (read only, round trip and size):");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:544: for path in extra.lines().filter(|l| !l.trim().is_empty()) {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:545: let path = Path::new(path.trim());
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:546: let rows = read_month(path);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:547: let name = path.to_string_lossy();
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:548: let origin = origin_of(&name);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:549: if rows.is_empty() {
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:550: println!("{name}: empty month");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:551: continue;
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:552: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:553: let sealed = seal(&rows, origin, &name);
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:554: println!(
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:556: rows.len(),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:557: sealed.frames.len(),
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:559: rows.len() * 56
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:561: time(&rows, origin, &sealed, "extra");
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:563: }
  GAP test code                              LINE   crates/store/tests/sealed_codec_samples.rs:564: }
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 True: while v >= 0x80 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 False: while v >= 0x80 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 True: let ts = if d % 1_000_000 == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 False: let ts = if d % 1_000_000 == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 True: let oi = if b.open_interest == OI_NULL {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 False: let oi = if b.open_interest == OI_NULL {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 True: if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 False: if bits == 0 { 1 } else { bits.div_ceil(7) }
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 True: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 False: let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 True: let oi = if first.open_interest == OI_NULL {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 False: let oi = if first.open_interest == OI_NULL {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 True: while b != 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 False: while b != 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 True: if g == 0 || g >= 1 << 62 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 False: if g == 0 || g >= 1 << 62 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 True: if g == 0 || g >= 1 << 62 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 False: if g == 0 || g >= 1 << 62 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 True: while top > 0 && histogram[top] == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 False: while top > 0 && histogram[top] == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 True: while top > 0 && histogram[top] == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 False: while top > 0 && histogram[top] == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 True: while w > 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 False: while w > 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 True: if cost < best {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 False: if cost < best {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 True: let limit = if best_w == 64 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 False: let limit = if best_w == 64 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 True: if exceptions > 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 False: if exceptions > 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 True: if sealed.prototype_rule != exact {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 False: if sealed.prototype_rule != exact {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 True: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 False: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 True: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 False: if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 True: let sign = if diff < 0 { "-" } else { "+" };
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 False: let sign = if diff < 0 { "-" } else { "+" };
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 True: if rows.is_empty() {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 False: if rows.is_empty() {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 True: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 False: if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 True: if rows.is_empty() {
  GAP test code                              BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 False: if rows.is_empty() {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:237: fn store_path(rel: &str) -> StorePath<'static> {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:238: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:239: let parts: Vec<&'static str> = rel.split('/').collect();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:240: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:241: parts.len() == 7 || parts.len() == 8,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:244: assert_eq!(parts[0], "bars", "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:245: let vendor = Vendor::ALL
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:246: .into_iter()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:247: .find(|v| v.as_str() == parts[1])
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_door.rs:248: .unwrap_or_else(|| panic!("{rel}: vendor"));
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:249: let contract = (parts.len() == 8)
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:250: .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:251: let tf = parts[parts.len() - 2];
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:252: let timeframe = *Timeframe::KNOWN
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:253: .iter()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:254: .find(|t| t.as_str() == tf)
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_door.rs:255: .unwrap_or_else(|| panic!("{rel}: rung"));
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:256: let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:257: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:258: StorePath::new(PathParts {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:259: vendor,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:260: exchange: parts[2],
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:261: segment: parts[3],
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:262: symbol: parts[4],
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:263: contract,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:264: timeframe,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:265: month,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:266: file: FileKind::Bars,
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:267: })
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:268: .unwrap()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:269: }
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:280: fn seal_a_private_store_root_for_the_shadow_check() {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:281: let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:282: let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:283: let bars_dir = root.join("bars");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:284: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:285: !fs::symlink_metadata(&bars_dir)
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:286: .unwrap()
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:287: .file_type()
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:288: .is_symlink(),
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:290: bars_dir.display()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:292: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_door.rs:293: !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:296: let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:297: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:298: for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:299: if rel.is_empty() || rel.starts_with('#') {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:300: continue;
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:301: }
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:302: let at = store_path(rel);
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:303: let symbol = rel.split('/').nth(4).unwrap().to_owned();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:304: let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:305: let served = store::generation::served(&root, at).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:306: let served_root = served.root(&root);
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:307: let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:308: if sbar.exists() {
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:309: println!("SKIP {rel}: already sealed");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:310: continue;
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:311: }
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:312: let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:313: assert_eq!(raw.form(), Form::Raw, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:314: let bars = raw.read_range(0, raw.records()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:315: let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:316: let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:317: let staged = sbar.with_extension("sbar-staged");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:318: let mut file = fs::File::create_new(&staged).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:319: built.write_to(&mut file).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:320: file.sync_all().unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:321: drop(file);
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:322: fs::rename(&staged, &sbar).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:323: let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:324: assert_eq!(sealed.form(), Form::Sealed, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:325: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_door.rs:326: sealed.read_range(0, sealed.records()).unwrap(),
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:330: months += 1;
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:331: rows += bars.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:332: raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:333: sealed_bytes += built.file_len();
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:334: println!(
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:336: bars.len(),
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:337: built.file_len(),
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:338: served_root.display()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:341: println!(
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:344: started.elapsed().as_secs_f64()
  GAP test code                              LINE   crates/store/tests/sealed_door.rs:346: }
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:153:17 True: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:153:31 False: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:231:9 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:231:45 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:241:9 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:241:9 False: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:241:29 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_door.rs:241:29 False: parts.len() == 7 || parts.len() == 8,
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:299:12 True: if rel.is_empty() || rel.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:299:12 False: if rel.is_empty() || rel.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:299:30 True: if rel.is_empty() || rel.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:299:30 False: if rel.is_empty() || rel.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:308:12 True: if sbar.exists() {
  GAP test code                              BRANCH crates/store/tests/sealed_door.rs:308:12 False: if sbar.exists() {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:49: fn next(&mut self) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:50: self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:51: let mut z = self.0;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:52: z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:53: z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:54: z ^ (z >> 31)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:55: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:56: fn below(&mut self, n: u64) -> u64 {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:57: self.next() % n
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:58: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:62: fn store_path(rel: &str) -> StorePath<'static> {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:63: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:64: let parts: Vec<&'static str> = rel.split('/').collect();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:65: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_samples.rs:66: parts.len() == 7 || parts.len() == 8,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:69: assert_eq!(parts[0], "bars", "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:70: let vendor = Vendor::ALL
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:71: .into_iter()
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:72: .find(|v| v.as_str() == parts[1])
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_samples.rs:73: .unwrap_or_else(|| panic!("{rel}: vendor"));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:74: let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:75: let tf = parts[parts.len() - 2];
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:76: let timeframe = *Timeframe::KNOWN
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:77: .iter()
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:78: .find(|t| t.as_str() == tf)
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_samples.rs:79: .unwrap_or_else(|| panic!("{rel}: rung"));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:80: let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:81: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:82: StorePath::new(PathParts {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:83: vendor,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:84: exchange: parts[2],
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:85: segment: parts[3],
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:86: symbol: parts[4],
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:87: contract,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:88: timeframe,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:89: month,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:90: file: FileKind::Bars,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:91: })
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:92: .unwrap()
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:93: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:95: fn raw_month(file: &Path) -> (Header, Vec<Bar>) {
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_samples.rs:96: let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:97: let at = HEADER_LEN as usize;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:98: let header = Header::read_region(&data[..at], data.len() as u64).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:99: let n = header.n_valid as usize;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:100: let bars = data[at..at + n * RECORD_LEN]
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:101: .chunks_exact(RECORD_LEN)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:102: .map(|record| Bar::decode(record).unwrap())
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:103: .collect();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:104: (header, bars)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:105: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:108: fn us(ns: u128) -> String {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:109: format!("{}.{}", ns / 1_000, ns % 1_000 / 100)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:110: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:112: fn percentiles(label: &str, mut ns: Vec<u128>) -> String {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:113: ns.sort_unstable();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:114: let n = ns.len();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:115: format!(
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:117: us(ns[n / 2]),
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:118: us(ns[n * 99 / 100]),
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:119: us(ns[n - 1])
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:121: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:125: fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:126: let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:127: let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:128: let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:129: let months: Vec<String> = std::fs::read_to_string(&list)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:130: .unwrap()
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:131: .lines()
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:132: .map(str::trim)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:133: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:134: .map(str::to_owned)
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:135: .collect();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:136: assert!(!months.is_empty(), "{list}: no months");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:137: let mut rng = Rng(2026);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:138: for rel in &months {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:139: let path = store_path(rel);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:140: let bin = path.to_path_buf(&root);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:141: let (header, bars) = raw_month(&bin);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:142: let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:143: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:144: let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:145: let seal_us = started.elapsed().as_micros();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:146: let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:147: std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:148: let mut file = std::fs::File::create(&sbar).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:149: built.write_to(&mut file).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:150: file.sync_all().unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:151: drop(file);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:153: let symbol = header.symbol_id;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:154: let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:155: let audited = sealed.audit().unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:156: assert_eq!(audited.rows, bars.len() as u64, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:157: assert_eq!(sealed.head().bar_header(), header, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:158: assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:159: let raw = BarFile::open_existing(&root, path, symbol).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:160: let n = bars.len() as u64;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:161: let mut page = Vec::new();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:162: let mut first = 0;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:163: while first < n {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:164: let count = (n - first).min(1_000);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:165: page.clear();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:166: sealed.read_rows(first, count, &mut page).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:167: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_samples.rs:169: bars[first as usize..(first + count) as usize],
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:172: first += count;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:174: for at in (0..n).step_by(1 + n as usize / 500) {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:175: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_samples.rs:176: raw.read_record(at).unwrap(),
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_samples.rs:177: bars[at as usize],
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:182: let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:183: let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:184: (Vec::new(), Vec::new(), Vec::new(), Vec::new());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:185: let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:186: let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:187: for _ in 0..2_000 {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:188: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:189: let month =
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:190: SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:191: open_ns.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:192: drop(month);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:193: if n == 0 {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:194: continue;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:195: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:196: let count = n.min(100);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:197: let at = rng.below(n - count + 1);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:198: page.clear();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:199: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:200: sealed.read_rows(at, count, &mut page).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:201: page_sealed.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:202: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:203: for i in at..at + count {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:204: let _ = raw.read_record(i).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:205: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:206: page_raw.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:207: assert_eq!(page, bars[at as usize..(at + count) as usize]);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:209: let at = rng.below(n);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:210: page.clear();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:211: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:212: sealed.read_rows(at, 1, &mut page).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:213: row_sealed.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:214: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:215: let one = raw.read_record(at).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:216: row_raw.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:217: assert_eq!(page, [one]);
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:219: let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:220: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:221: let found = sealed.first_at_or_after(ts).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:222: find_sealed.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:223: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:224: let raw_found = raw.first_at_or_after(ts).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:225: find_raw.push(started.elapsed().as_nanos());
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:226: let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:227: assert_eq!(found, scanned, "{rel} ts {ts}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:228: assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:230: let raw_len = std::fs::metadata(&bin).unwrap().len();
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:231: println!(
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:233: sealed.head().n_blocks,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:234: sealed.head().table_entries,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:235: built.file_len(),
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:236: built.file_len() * 100 / n.max(1),
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:237: built.file_len() * 1_000 / raw_len,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:238: sealed.head().prv_raw_len,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:239: sealed.head().prv_packed_len,
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:241: println!("  {}", percentiles("open sealed (file + head)", open_ns));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:242: if n > 0 {
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:243: println!("  {}", percentiles("100-row page sealed", page_sealed));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:244: println!("  {}", percentiles("100-row page raw   ", page_raw));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:245: println!("  {}", percentiles("one row sealed     ", row_sealed));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:246: println!("  {}", percentiles("one row raw        ", row_raw));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:247: println!("  {}", percentiles("first_at_or_after sealed", find_sealed));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:248: println!("  {}", percentiles("first_at_or_after raw   ", find_raw));
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:249: }
  GAP test code                              LINE   crates/store/tests/sealed_samples.rs:251: }
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_samples.rs:66:9 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_samples.rs:66:9 False: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_samples.rs:66:29 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_samples.rs:66:29 False: parts.len() == 7 || parts.len() == 8,
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:133:24 True: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:133:24 False: .filter(|line| !line.is_empty() && !line.starts_with('#'))
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:163:15 True: while first < n {
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:163:15 False: while first < n {
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:193:16 True: if n == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:193:16 False: if n == 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:242:12 True: if n > 0 {
  GAP test code                              BRANCH crates/store/tests/sealed_samples.rs:242:12 False: if n > 0 {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:44: fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:45: let y = if m <= 2 { y - 1 } else { y };
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:46: let era = y.div_euclid(400);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:47: let yoe = y - era * 400;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:48: let mp = (m + 9) % 12;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:49: let doy = (153 * mp + 2) / 5 + d - 1;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:50: let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:51: era * 146_097 + doe - 719_468
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:52: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:54: fn ist_day(bar: &Bar) -> i64 {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:55: (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:56: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:58: fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:59: let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:60: let parts: Vec<&'static str> = rel.split('/').collect();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:61: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:62: parts.len() == 7 || parts.len() == 8,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:65: let vendor = Vendor::ALL
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:66: .into_iter()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:67: .find(|v| v.as_str() == parts[1])
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:68: .unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:69: let contract = (parts.len() == 8)
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:70: .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:71: let tf = parts[parts.len() - 2];
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:72: let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:73: let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:74: let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:75: let path = StorePath::new(PathParts {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:76: vendor,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:77: exchange: parts[2],
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:78: segment: parts[3],
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:79: symbol: parts[4],
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:80: contract,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:81: timeframe,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:82: month,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:83: file: FileKind::Bars,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:84: })
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:85: .unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:86: (path, parts[4])
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:87: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:89: fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:90: let mut files = BTreeMap::new();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:91: let mut stack = vec![dir.to_path_buf()];
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:92: while let Some(at) = stack.pop() {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:93: let Ok(entries) = fs::read_dir(&at) else {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:94: continue;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:96: for entry in entries {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:97: let path = entry.unwrap().path();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:98: if path.is_dir() {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:99: stack.push(path);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:100: } else {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:101: files.insert(path.clone(), fs::read(&path).unwrap());
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:102: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:105: files
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:106: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:108: fn micros(started: Instant) -> f64 {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:109: started.elapsed().as_secs_f64() * 1e6
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:110: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:117: fn the_write_door_over_real_months_sealed_without_one_day() {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:118: let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:119: let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:120: let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:121: for root in [&source, &work] {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:122: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:123: !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:126: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:127: !fs::symlink_metadata(root.join("bars"))
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:128: .is_ok_and(|meta| meta.file_type().is_symlink()),
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:130: root.display()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:133: for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:134: let line = line.trim();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:135: if line.is_empty() || line.starts_with('#') {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:136: continue;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:137: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:138: let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:139: let (path, symbol) = store_path(rel);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:140: let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:141: let full = {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:142: let file = BarFile::open_existing(&source, path, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:143: file.read_range(0, file.records()).unwrap()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:146: let day = if date == "last" {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:147: ist_day(full.last().expect("a month with bars"))
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:149: let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:150: days_from_civil(ymd[0], ymd[1], ymd[2])
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:152: let (held, offered): (Vec<Bar>, Vec<Bar>) =
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:153: full.iter().copied().partition(|bar| ist_day(bar) != day);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:154: assert!(!offered.is_empty(), "{rel}: no bar on {date}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:155: assert!(!held.is_empty(), "{rel}: nothing but {date}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:156: let root = work.join(format!("m{line_no}"));
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:157: fs::create_dir_all(&root).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:159: let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:160: for chunk in held.chunks(65_536) {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:161: writer.append(chunk).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:162: }
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:164: let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:165: let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:166: drop(raw);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:167: let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:168: let staged = sbar.with_extension("sbar-staged");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:169: fs::write(&staged, built.to_bytes()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:170: fs::rename(&staged, &sbar).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:171: let before = snapshot(&root.join("bars"));
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:173: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:174: let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:175: let open_us = micros(started);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:176: assert_eq!(writer.form(), Form::Sealed, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:177: let first_day = ist_day(&held[0]);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:178: let reoffer: Vec<Bar> = held
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:179: .iter()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:180: .copied()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:181: .filter(|bar| ist_day(bar) == first_day)
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:182: .collect();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:183: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:184: let again = writer.append(&reoffer).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:185: let reoffer_us = micros(started);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:186: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:188: Appended::AlreadyPresent {
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:189: first_index: 0,
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:190: n_valid: held.len() as u64
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:191: },
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:194: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:195: let refused = writer.append(&offered).unwrap_err();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:196: let refused_us = micros(started);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:197: let StoreError::Sealed { offer, at, .. } = refused else {
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_write_door.rs:198: panic!("{rel}: {refused}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:200: let is_last_day = day == ist_day(full.last().unwrap());
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:201: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:203: if is_last_day {
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:204: SealedOffer::Follows
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:206: SealedOffer::Disagrees(store::file::Conflict::NotHeld)
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:210: assert_eq!(at, 0, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:211: drop(writer);
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:212: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:213: snapshot(&root.join("bars")),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:218: let started = Instant::now();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:219: let repaired = store::repair::repair_month(
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:220: &root,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:221: path,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:222: symbol_id,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:223: &offered,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:224: Policy {
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:225: // No offered bar shares a stamp with a held one: the day is
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:226: // wholly absent, so the comparison this sets is never made.
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:227: open_interest_applies: true,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:228: basis_witness: false,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:229: },
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:231: .unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:232: let repair_ms = micros(started) / 1e3;
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:233: let MonthRepair::Repaired { added, .. } = repaired else {
  F? panic! (failing arm)                    LINE   crates/store/tests/sealed_write_door.rs:234: panic!("{rel}: {repaired:?}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:236: assert_eq!(added, offered.len() as u64, "{rel}");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:237: let served = BarFile::open_existing(&root, path, symbol_id).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:238: assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:239: let now = served.read_range(0, served.records()).unwrap();
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:240: assert!(
  F? assert! message argument                LINE   crates/store/tests/sealed_write_door.rs:241: now == full,
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:244: assert_eq!(
  F? assert_eq! message argument             LINE   crates/store/tests/sealed_write_door.rs:245: snapshot(&root.join("bars")),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:249: println!(
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:253: full.len(),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:254: held.len(),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:255: offered.len(),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:256: offer.as_str(),
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:257: built.file_len()
  GAP test code                              LINE   crates/store/tests/sealed_write_door.rs:260: }
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:45:16 True: let y = if m <= 2 { y - 1 } else { y };
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:45:16 False: let y = if m <= 2 { y - 1 } else { y };
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:62:9 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:62:9 False: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:62:29 True: parts.len() == 7 || parts.len() == 8,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:62:29 False: parts.len() == 7 || parts.len() == 8,
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:92:15 True: while let Some(at) = stack.pop() {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:92:15 False: while let Some(at) = stack.pop() {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:93:13 True: let Ok(entries) = fs::read_dir(&at) else {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:93:13 False: let Ok(entries) = fs::read_dir(&at) else {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:98:16 True: if path.is_dir() {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:98:16 False: if path.is_dir() {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:135:12 True: if line.is_empty() || line.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:135:12 False: if line.is_empty() || line.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:135:31 True: if line.is_empty() || line.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:135:31 False: if line.is_empty() || line.starts_with('#') {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:146:22 True: let day = if date == "last" {
  GAP test code                              BRANCH crates/store/tests/sealed_write_door.rs:146:22 False: let day = if date == "last" {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:197:13 True: let StoreError::Sealed { offer, at, .. } = refused else {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:197:13 False: let StoreError::Sealed { offer, at, .. } = refused else {
  F? assert_eq! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:203:16 True: if is_last_day {
  F? assert_eq! condition branch (failing side) BRANCH crates/store/tests/sealed_write_door.rs:203:16 False: if is_last_day {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:233:13 True: let MonthRepair::Repaired { added, .. } = repaired else {
  F? let-else in test                        BRANCH crates/store/tests/sealed_write_door.rs:233:13 False: let MonthRepair::Repaired { added, .. } = repaired else {
  F? assert! message argument                LINE   crates/store/tests/slot_audit.rs:116: matches!(
  F? panic! (failing arm)                    LINE   crates/store/tests/slot_audit.rs:137: panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
  F? assert! message argument                LINE   crates/store/tests/slot_audit.rs:277: matches!(
  F? assert! message argument                LINE   crates/store/tests/slot_audit.rs:347: matches!(
  F? assert! message argument                LINE   crates/store/tests/slot_audit.rs:410: matches!(
  F? panic! (failing arm)                    LINE   crates/store/tests/slot_audit.rs:485: panic!("damaged");
  GAP test code                              BRANCH crates/store/tests/slot_audit.rs:130:13 False: let MonthVerdict::Reverted {
  F? let-else in test                        BRANCH crates/store/tests/slot_audit.rs:484:9 False: let SlotState::Damaged { reason, .. } = audit.slots[1] else {
  F? assert! message argument                LINE   crates/store/tests/slot_audit_sealed.rs:95: raw.slot_audit()
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:149: Ok(file) => {
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:150: assert_eq!(file.form(), Form::Sealed, "byte {at}");
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:151: assert_eq!(file.slot_audit(), None, "byte {at}");
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:152: assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:153: if let Ok(last) = file.read_record(39) {
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:154: assert_eq!(last.close, 7_039, "byte {at}");
  GAP test code                              LINE   crates/store/tests/slot_audit_sealed.rs:155: }
  GAP test code                              BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True: if let Ok(last) = file.read_record(39) {
  GAP test code                              BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False: if let Ok(last) = file.read_record(39) {
  GAP test code                              LINE   crates/store/tests/slot_damaged_log.rs:93: Some(OwnedValue::Uint(value)) => *value,
  F? panic! (failing arm)                    LINE   crates/store/tests/slot_damaged_log.rs:94: other => panic!("{key}: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/slot_damaged_log.rs:101: other => panic!("{key}: {other:?}"),
  F? assert_eq! message argument             LINE   crates/store/tests/slot_damaged_log.rs:192: found
  F? assert_eq! message argument             LINE   crates/store/tests/slot_damaged_log.rs:193: .records
  F? assert_eq! message argument             LINE   crates/store/tests/slot_damaged_log.rs:194: .iter()
  F? assert_eq! message argument             LINE   crates/store/tests/slot_damaged_log.rs:195: .map(|r| (r.target.clone(), r.message.clone()))
  F? assert_eq! message argument             LINE   crates/store/tests/slot_damaged_log.rs:196: .collect::<Vec<_>>()
  F? assert! message argument                LINE   crates/store/tests/sort_index.rs:488: index.torn()
  F? assert_eq! message argument             LINE   crates/store/tests/sort_index.rs:761: column.as_str()
  F? assert_eq! message argument             LINE   crates/store/tests/sort_index.rs:843: column.as_str()
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:387:62 False: matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:430:56 False: matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:446:63 False: assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:660:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:736:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:874:16 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:874:54 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/sort_index.rs:889:16 False: if reason.contains("2031-02.new.srt")),
  GAP test code                              LINE   crates/store/tests/sort_index_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  GAP test code                              LINE   crates/store/tests/sort_index_log.rs:75: other => format!("{other:?}"),
  GAP test code                              BRANCH crates/store/tests/support/mod.rs:62:15 False: let ran = output.status.success() && stdout.contains("1 passed");
  F? assert! condition branch (failing side) BRANCH crates/store/tests/tail_proof.rs:144:9 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/tail_proof.rs:144:36 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/tail_proof.rs:144:60 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  F? assert_eq! message argument             LINE   crates/store/tests/tix.rs:173: timeframe.as_str()
  GAP test code                              BRANCH crates/store/tests/tix.rs:127:12 False: if let Some(last) = bars.last() {
  GAP test code                              BRANCH crates/store/tests/tix.rs:605:76 True: && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:43: fn path(timeframe: Timeframe) -> StorePath<'static> {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:44: StorePath::new(PathParts {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:45: vendor: Vendor::Groww,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:46: exchange: "NSE",
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:47: segment: "INDEX",
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:48: symbol: "NIFTY",
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:49: contract: None,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:50: timeframe,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:51: month: YearMonth::new(2024, 7).unwrap(),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:52: file: FileKind::Bars,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:53: })
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:54: .unwrap()
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:55: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:59: fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:60: let mut out = Vec::new();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:61: let (mut day, mut taken) = (0, 0);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:62: while taken < days {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:64: if !weekdays || day % 7 < 5 {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:65: for k in 0..per_day {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:66: let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:67: out.push(Bar {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:68: ts_micros: ts,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:69: open: 2_000_000 + k,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:70: high: 2_000_100 + k,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:71: low: 1_999_900 + k,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:72: close: 2_000_050 + k,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:73: volume: 10,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:74: open_interest: OI_NULL,
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:75: });
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:76: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:77: taken += 1;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:78: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:79: day += 1;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:81: out
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:82: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:87: fn next(&mut self) -> u64 {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:88: self.0 ^= self.0 >> 12;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:89: self.0 ^= self.0 << 25;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:90: self.0 ^= self.0 >> 27;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:91: self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:92: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:95: fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:96: ns.sort_unstable();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:97: let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:98: (at(500), at(990), *ns.last().unwrap())
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:99: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:101: fn report(case: &str, ns: Vec<u64>) {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:102: let (p50, p99, max) = percentiles(ns);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:103: println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:104: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:107: fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:108: inputs
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:109: .iter()
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:110: .map(|&input| {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:111: let start = Instant::now();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:112: black_box(op(black_box(input)));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:113: start.elapsed().as_nanos() as u64
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:114: })
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:115: .collect()
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:116: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:118: fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:119: let root: PathBuf =
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:120: std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:121: let _ = fs::remove_dir_all(&root);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:122: fs::create_dir_all(&root).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:124: let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:125: for chunk in bars.chunks(22_500) {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:126: writer.append(chunk).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:127: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:129: let n = bars.len() as u64;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:130: let first = bars[0].ts_micros;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:131: let last = bars[bars.len() - 1].ts_micros;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:132: let width = i64::from(timeframe.secs()) * MICROS;
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:133: let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:135: let anywhere: Vec<i64> = (0..LOOKUPS)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:136: .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:137: .collect();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:139: let on_grid: Vec<i64> = anywhere
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:140: .iter()
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:141: .map(|&ts| first + (ts - first) / width * width)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:142: .collect();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:143: let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:145: let tix = path(timeframe).to_path_buf(&root).with_extension("tix");
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:146: let passes: &[bool] = if tix.exists() {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:147: &[true, false]
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:149: &[false]
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:151: for &with_index in passes {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:152: if !with_index && tix.exists() {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:153: fs::remove_file(&tix).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:154: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:155: let tag = if with_index { "index" } else { "bisection" };
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:156: let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:159: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:160: report(
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:161: &label("by time, random us"),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:162: time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:164: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:165: report(
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:166: &label("by time, on grid"),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:167: time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:169: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:170: report(
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:171: &label("by row, random rows (cold block)"),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:172: time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:176: let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:177: let same = vec![rows[0]; LOOKUPS];
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:178: report(
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:179: &label("by row, one row (warm block)"),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:180: time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:183: let _ = fs::remove_dir_all(&root);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:184: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:188: fn time_and_row_lookup_latency() {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:189: measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:190: measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:191: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:209: fn small_month_rule_latency() {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:211: let root: PathBuf =
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:212: std::env::temp_dir().join(format!("brutex-tix-latency-small-{}", std::process::id()));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:213: let _ = fs::remove_dir_all(&root);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:214: fs::create_dir_all(&root).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:215: let all = month(1, 1_100, 1, true);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:216: for (name, n) in [
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:217: ("1s, 1023 bars, below", 1_023_usize),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:218: ("1s, 1024 bars, at", 1_024),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:219: ] {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:220: let at = root.join(name.replace([' ', ','], "-"));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:221: fs::create_dir_all(&at).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:222: BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:223: .unwrap()
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:224: .append(&all[..n])
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:225: .unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:226: let tix = path(Timeframe::SECOND_1)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:227: .to_path_buf(&at)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:228: .with_extension("tix");
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:229: assert_eq!(tix.exists(), n >= 1_024, "{name}");
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:230: let (first, last) = (all[0].ts_micros, all[n - 1].ts_micros);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:231: let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:232: let anywhere: Vec<i64> = (0..LOOKUPS)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:233: .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:234: .collect();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:235: let file = BarFile::open_existing(&at, path(Timeframe::SECOND_1), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:236: report(
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:237: &format!("{name}: by time, random us [{:?}]", file.time_lookup()),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:238: time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:241: for (name, held) in [
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:242: ("commit of 1 bar under the threshold (500 held)", 500_usize),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:243: ("commit of 1 bar reaching the threshold (1023 held)", 1_023),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:244: ("commit of 1 bar past the threshold (1099 held)", 1_099),
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:245: ] {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:246: let mut ns = Vec::with_capacity(COMMITS);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:247: for k in 0..COMMITS {
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:248: let at = root.join(format!("commit-{held}-{k}"));
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:249: fs::create_dir_all(&at).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:250: let mut writer = BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:251: writer.append(&all[..held]).unwrap();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:252: let start = Instant::now();
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:253: black_box(writer.append(black_box(&all[held..=held])).unwrap());
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:254: ns.push(start.elapsed().as_nanos() as u64);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:255: drop(writer);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:256: let _ = fs::remove_dir_all(&at);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:257: }
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:258: report(&format!("{name}, n={COMMITS}"), ns);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:260: let _ = fs::remove_dir_all(&root);
  GAP test code                              LINE   crates/store/tests/tix_latency.rs:261: }
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:62:11 True: while taken < days {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:62:11 False: while taken < days {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:64:12 True: if !weekdays || day % 7 < 5 {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:64:12 False: if !weekdays || day % 7 < 5 {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:64:25 True: if !weekdays || day % 7 < 5 {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:64:25 False: if !weekdays || day % 7 < 5 {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:146:30 True: let passes: &[bool] = if tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:146:30 False: let passes: &[bool] = if tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:152:12 True: if !with_index && tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:152:12 False: if !with_index && tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:152:27 True: if !with_index && tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:152:27 False: if !with_index && tix.exists() {
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:155:22 True: let tag = if with_index { "index" } else { "bisection" };
  GAP test code                              BRANCH crates/store/tests/tix_latency.rs:155:22 False: let tag = if with_index { "index" } else { "bisection" };
  GAP test code                              BRANCH crates/store/tests/tix_rebuilder.rs:154:12 False: if let Some(last) = bars.last() {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False: }) if asked == SYMBOL + 1
  F? panic! (failing arm)                    LINE   crates/store/tests/tix_small_month.rs:160: other => panic!("not committed: {other:?}"),
  F? assert! message argument                LINE   crates/store/tests/tix_small_month.rs:463: reader.time_lookup()
  GAP test code                              BRANCH crates/store/tests/tix_small_month.rs:143:12 False: if let Some(last) = bars.last() {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/tix_small_month.rs:461:74 False: matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
  GAP test code                              LINE   crates/store/tests/tix_warn_once.rs:77: other => format!("{other:?}"),
  GAP test code                              LINE   crates/store/tests/tix_warn_once.rs:125: Some((_, OwnedValue::Uint(n))) => *n,
  F? panic! (failing arm)                    LINE   crates/store/tests/tix_warn_once.rs:127: other => panic!("{key} is not a count: {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/tix_warn_once.rs:160: other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/tix_warn_once.rs:162: other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
  F? assert_eq! message argument             LINE   crates/store/tests/unit.rs:110: input.len()
  F? panic! (failing arm)                    LINE   crates/store/tests/unit.rs:527: _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
  F? assert! message argument                LINE   crates/store/tests/unit.rs:1820: matches!(
  F? assert! message argument                LINE   crates/store/tests/unit.rs:1859: matches!(
  F? assert! message argument                LINE   crates/store/tests/unit.rs:1874: matches!(
  F? assert_eq! message argument             LINE   crates/store/tests/unit.rs:2096: vendor.as_str(),
  F? assert_ne! message argument             LINE   crates/store/tests/unit.rs:2826: a.as_str(),
  F? assert_ne! message argument             LINE   crates/store/tests/unit.rs:2827: b.as_str()
  F? assert_ne! message argument             LINE   crates/store/tests/unit.rs:2833: a.as_str()
  F? assert_eq! message argument             LINE   crates/store/tests/unit.rs:2893: tf.as_str()
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:399:13 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:399:36 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:868:13 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:868:54 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  GAP test code                              BRANCH crates/store/tests/unit.rs:908:16 False: if one.magic()[..7] == other.magic()[..7]
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:1475:32 False: read == old || read == new,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:2047:26 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:2047:43 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:2173:13 False: err.to_string().contains(field) && err.to_string().contains("case"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/unit.rs:2173:48 False: err.to_string().contains(field) && err.to_string().contains("case"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:129: Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:168: other => panic!("100x must be refused table_too_large, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:395: other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:450: panic!("{whole} must be found");
  GAP test code                              LINE   crates/store/tests/verdict.rs:588: }
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:728: other => panic!("expected image_missing, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:793: other => panic!("expected generation_collision, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:1318: other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:1821: other => panic!(
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:1823: clean.len()
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:1856: other => panic!("expected record_corrupt, got {other:?}"),
  F? assert_eq! message argument             LINE   crates/store/tests/verdict.rs:2283: feed.as_str(),
  F? assert_eq! message argument             LINE   crates/store/tests/verdict.rs:2284: timeframe.as_str()
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:2435: other => panic!("expected lock_stalled, got {other:?}"),
  F? assert! message argument                LINE   crates/store/tests/verdict.rs:2455: matches!(
  F? assert! message argument                LINE   crates/store/tests/verdict.rs:2549: image.display()
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:2554: other => panic!("expected index_missing, got {other}"),
  F? panic! (failing arm)                    LINE   crates/store/tests/verdict.rs:2730: other => panic!("{other}"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:228:13 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:228:36 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  F? let-else in test                        BRANCH crates/store/tests/verdict.rs:449:13 False: let Found::Record { ordinal, record } = &answer.found else {
  GAP test code                              BRANCH crates/store/tests/verdict.rs:584:12 False: if count > 2 {
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:1890:17 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:1890:40 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2123:17 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2123:40 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2358:58 False: matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2545:21 False: image.extension().is_some_and(|ext| ext == "vdt")
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2546:28 False: && image.parent().is_some_and(|dir| dir.ends_with("2031"))
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2547:28 False: && image.exists(),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2604:13 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2604:22 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2782:9 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/verdict.rs:2782:61 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  F? assert! message argument                LINE   crates/store/tests/write.rs:1402: sidecar.display()
  F? assert! message argument                LINE   crates/store/tests/write.rs:1419: sidecar.display()
  F? panic! (failing arm)                    LINE   crates/store/tests/write.rs:1619: panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
  F? panic! (failing arm)                    LINE   crates/store/tests/write.rs:1663: panic!(
  F? assert! condition branch (failing side) BRANCH crates/store/tests/write.rs:1564:13 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/write.rs:1564:62 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  F? assert! condition branch (failing side) BRANCH crates/store/tests/write.rs:2103:85 False: Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
  GAP production                             BRANCH crates/telemetry/src/clock.rs:146:16 False: if let Some(slot) = digits.get_mut(count) {
  GAP production                             BRANCH crates/telemetry/src/clock.rs:159:16 False: if let Some(byte) = digits.get(i) {
  F? assert_eq! message argument             LINE   crates/telemetry/src/level.rs:142: level.rank()
  GAP test code                              BRANCH crates/telemetry/src/lib.rs:420:19 False: let ran = output.status.success() && stdout.contains("1 passed");
  F? assert! message argument                LINE   crates/telemetry/src/record.rs:826: matches!(
  GAP production                             BRANCH crates/telemetry/src/record.rs:92:16 False: && u64::from(event.dropped_fields()) == self.dropped_fields
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/run_dirs.rs:234:21 False: assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/run_dirs.rs:234:44 False: assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/run_dirs.rs:240:21 False: !other.sub.starts_with(&format!("{}/", dir.sub))
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/run_dirs.rs:241:28 False: && !dir.sub.starts_with(&format!("{}/", other.sub)),
  GAP production                             LINE   crates/telemetry/src/sink.rs:3804: cannot(
  GAP production                             LINE   crates/telemetry/src/sink.rs:3805: &format!(
  GAP production                             LINE   crates/telemetry/src/sink.rs:3806: "an earlier start's unfinished seed at {} cannot be moved aside",
  GAP production                             LINE   crates/telemetry/src/sink.rs:3807: staging.display()
  GAP production                             LINE   crates/telemetry/src/sink.rs:3808: ),
  GAP production                             LINE   crates/telemetry/src/sink.rs:3809: &e,
  GAP production                             LINE   crates/telemetry/src/sink.rs:3810: )
  GAP production                             LINE   crates/telemetry/src/sink.rs:3811: })?;
  GAP production                             LINE   crates/telemetry/src/sink.rs:3823: return Err(cannot(
  GAP production                             LINE   crates/telemetry/src/sink.rs:3824: "the seed cannot be opened",
  GAP production                             LINE   crates/telemetry/src/sink.rs:3825: &said.unwrap_or_else(|| "no reason given".to_owned()),
  GAP production                             LINE   crates/telemetry/src/sink.rs:3833: }
  GAP test code                              LINE   crates/telemetry/src/sink.rs:4318: self.health.last_error = Some(why.clone());
  GAP test code                              LINE   crates/telemetry/src/sink.rs:4319: return Err(why);
  GAP production                             LINE   crates/telemetry/src/sink.rs:4472: }
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:4668: panic!(
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:4670: path.display(),
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:4671: String::from_utf8_lossy(line)
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5086: written[0].at_unix_millis
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5123: pair[1].seq,
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5124: pair[1].at_unix_millis,
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5125: pair[0].seq,
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5126: pair[0].at_unix_millis
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5456: span * 2
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:5675: path.display()
  F? assert! message argument                LINE   crates/telemetry/src/sink.rs:6059: rolled.health()
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:7643: other => panic!("{dimension}={value}: {other:?}"),
  GAP test code                              LINE   crates/telemetry/src/sink.rs:8012: return 0;
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:8071: panic!("the reaper did not finish: {:?}", sink.audit_health());
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:8922: panic!("{pointer:?}");
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10245: AuditLine::RolledOff => None,
  F? panic! (failing arm)                    LINE   crates/telemetry/src/sink.rs:10246: AuditLine::Unreadable(why) => panic!("{dimension}={value}: {why}"),
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10377: fn an_audit_line_survives_2_gib_of_refusals_at_full_size() {
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10378: survives(
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10379: "survives-full",
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10380: 2 * 1024 * 1024 * 1024,
  GAP test code                              LINE   crates/telemetry/src/sink.rs:10383: }
  GAP production                             BRANCH crates/telemetry/src/sink.rs:897:16 False: if let Some(first) = self.ring_firsts.front_mut()
  GAP production                             BRANCH crates/telemetry/src/sink.rs:908:23 False: } else if let Some(held) = self.recent.get_mut(self.recent_next) {
  GAP production                             BRANCH crates/telemetry/src/sink.rs:926:16 False: if let Some(least) = self.codes.iter_mut().min_by_key(|(_, count)| *count) {
  GAP production                             BRANCH crates/telemetry/src/sink.rs:3822:12 True: if opened == Opened::Failed {
  GAP production                             BRANCH crates/telemetry/src/sink.rs:3830:16 False: if let Some(file) = &seeding.file {
  GAP production                             BRANCH crates/telemetry/src/sink.rs:4058:20 False: if let Some((record, mut line)) = taken {
  GAP test code                              BRANCH crates/telemetry/src/sink.rs:4317:16 True: if let Err(why) = sink_dir_there(&self.dir) {
  GAP production                             BRANCH crates/telemetry/src/sink.rs:4468:24 False: if let Some(parent) = path.parent() {
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:4804:21 False: assert!(target.append(b"b").is_ok() && target.sync().is_ok());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:4804:52 False: assert!(target.append(b"b").is_ok() && target.sync().is_ok());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5004:17 False: said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5004:65 False: said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5692:17 False: assert!(!current.is_empty() && !first_rolled.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5692:40 False: assert!(!current.is_empty() && !first_rolled.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5737:13 False: after_one > 0 && after_one < MIN_FILE_BYTES,
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:5737:30 False: after_one > 0 && after_one < MIN_FILE_BYTES,
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6601:13 False: why.contains("another") && why.contains(&dir.display().to_string()),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6601:40 False: why.contains("another") && why.contains(&dir.display().to_string()),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6667:17 False: assert!(text.contains("before the close") && text.contains("the successor's line"));
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6667:54 False: assert!(text.contains("before the close") && text.contains("the successor's line"));
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6687:13 False: said.contains("closed") && said.contains("no space left"),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6687:40 False: said.contains("closed") && said.contains("no space left"),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6729:13 False: named.contains("ahead of the clock") && named.contains(&future.to_string()),
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:6729:53 False: named.contains("ahead of the clock") && named.contains(&future.to_string()),
  F? let-else in test                        BRANCH crates/telemetry/src/sink.rs:8011:13 False: let Ok(entries) = std::fs::read_dir(root) else {
  GAP test code                              BRANCH crates/telemetry/src/sink.rs:8064:15 False: while std::time::Instant::now() < deadline {
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:8179:21 False: assert!(!health.reaper_behind() && !health.is_loud());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:8179:48 False: assert!(!health.reaper_behind() && !health.is_loud());
  F? let-else in test                        BRANCH crates/telemetry/src/sink.rs:8921:21 False: let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11066:17 False: assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11066:57 False: assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11076:36 False: .is_some_and(|why| why.contains("first line does not decode")
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11131:17 False: why.contains("last 64 KiB hold no line") && !why.contains("first line")
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11197:17 False: why.contains("first line does not decode") && !why.contains("last 64 KiB")
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11202:17 False: assert!(a0 < a1 && a1 < a2, "premise");
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11202:28 False: assert!(a0 < a1 && a1 < a2, "premise");
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11233:17 False: assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/sink.rs:11233:44 False: assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
  F? panic! (failing arm)                    LINE   crates/telemetry/src/state_events_tests.rs:85: || panic!("{path} is named by a row and not read by this test"),
  F? assert! message argument                LINE   crates/telemetry/src/state_events_tests.rs:105: level.label(),
  F? assert! message argument                LINE   crates/telemetry/src/state_events_tests.rs:113: level.label()
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/state_events_tests.rs:176:17 False: assert!(!row.change.is_empty() && !row.site.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/state_events_tests.rs:176:43 False: assert!(!row.change.is_empty() && !row.site.is_empty());
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/state_events_tests.rs:321:13 False: site.contains(&format!("\"{}\"", row.message))
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/state_events_tests.rs:322:20 False: && site.contains(&format!("\"{}\"", row.target)),
  GAP production                             LINE   crates/telemetry/src/tail.rs:409: Err(e) => {
  GAP production                             LINE   crates/telemetry/src/tail.rs:410: out.errors.push(format!("{}: {e}", path.display()));
  GAP production                             LINE   crates/telemetry/src/tail.rs:411: continue;
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/tail.rs:1862:17 False: assert!(newest > 0 && newest < READ_BLOCK);
  F? assert! condition branch (failing side) BRANCH crates/telemetry/src/tail.rs:1862:31 False: assert!(newest > 0 && newest < READ_BLOCK);
  F? panic! (failing arm)                    LINE   crates/telemetry/tests/cloud_mutants_sink.rs:263: other => panic!("a directory is not a rolled-off line: {other:?}"),
  GAP test code                              LINE   crates/telemetry/tests/global_absent.rs:96: evaluated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  GAP test code                              LINE   crates/telemetry/tests/global_absent.rs:97: 7_u64
  GAP test code                              LINE   crates/telemetry/tests/global_absent.rs:98: };
  GAP test code                              LINE   crates/telemetry/tests/global_absent.rs:102: "did not land",
  GAP test code                              LINE   crates/telemetry/tests/global_absent.rs:103: "bars" => telemetry::Value::Uint(count()),
`````

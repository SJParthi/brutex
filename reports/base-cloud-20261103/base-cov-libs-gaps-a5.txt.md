# base-cov-libs-gaps-a5.txt.md

BASE-COV-LIBS-LSA (the COV-LIBS assembly, attempt 5): Route B figures of the ten crates whole, per crate and per file, the NOT PROVED files, and every uncovered line and branch side with its class. Written from the session's own measurement; the (F) and (U) rows with their rules and proofs are in `base-cov-libs-fu.txt.md`. One fenced block below holds the list; sha256 of the fenced content: `13a1135c43e4c276bed248aa1ad6c67f2d2d1047863f583d90b6e65f99413708`.

`````text
ROUTE B (the kit's d3-route-b-cloud.sh, sha256 f032b525..., unchanged; d3-covset.mjs ecef3bdc...): pinned rustc 1.99.0-nightly (1a98b1e13 2026-08-07) installed as nightly-2026-08-08, llvm-cov/llvm-profdata LLVM version 23.1.0-rust-1.99.0-nightly; x86_64-unknown-linux-gnu, 4 cores, root host; test binaries as nobody (CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER='setpriv --reuid=65534 --regid=65534 --clear-groups', umask 000); set reports/base-cloud-20261103/deep-BASE-COV-LIBS-LSA-d096f65942d4-set.txt.md (362 coverage files whole).
Runs (each code commit after a run's tip touches only crates that run did not measure; d096f65942d4 changes runner alone, later commits docs and reports alone):
  run 1: ROUTE-B START 2026-11-05 16:02:36 IST tree /var/tmp/lsa-tree tip 3069f0c9d23c6412789754cfce388bc9b9b9c8b5 packages core costs engine greeks indicators lake store telemetry vocab K /var/tmp/lsa-deep P /var/tmp/lsa-prof/3069f0c9d23c-1-B (entries before: 0)
  run 1: == test costs rc=0 END 2026-11-05 16:04:11 IST (0 min); tests passed 385 failed 0 ignored 2
  run 1: == test engine rc=0 END 2026-11-05 16:05:18 IST (0 min); tests passed 145 failed 0 ignored 0
  run 1: == test greeks rc=0 END 2026-11-05 16:05:39 IST (0 min); tests passed 59 failed 0 ignored 0
  run 1: == test indicators rc=0 END 2026-11-05 16:10:55 IST (5 min); tests passed 528 failed 0 ignored 0
  run 1: == test lake rc=0 END 2026-11-05 16:12:01 IST (0 min); tests passed 150 failed 0 ignored 6
  run 1: == test store rc=0 END 2026-11-05 16:34:22 IST (21 min); tests passed 1104 failed 0 ignored 8
  run 1: == test telemetry rc=0 END 2026-11-05 16:37:39 IST (0 min); tests passed 256 failed 0 ignored 1
  run 1: == test vocab rc=0 END 2026-11-05 16:39:20 IST (1 min); tests passed 190 failed 0 ignored 0
  run 1: ROUTE-B DONE 2026-11-05 16:39:55 IST WITH NOT PROVED lines above
  run 2: ROUTE-B START 2026-11-05 16:17:27 IST tree /var/tmp/lsa-tree2 tip d096f65942d4d3cdb98589e01842dff00c945b39 packages runner K /var/tmp/lsa-deep2 P /var/tmp/lsa-prof/d096f65942d4-1-B (entries before: 0)
  run 2: == test runner rc=0 END 2026-11-05 18:25:12 IST (127 min); tests passed 984 failed 0 ignored 0
  run 2: ROUTE-B DONE 2026-11-05 18:26:20 IST WITH NOT PROVED lines above
  run 3: ROUTE-B START 2026-11-05 16:53:57 IST tree /var/tmp/lsa-tree tip 0b7aa0d6d80120214276e00de82ebd4d231e54b5 packages core K /var/tmp/lsa-deep P /var/tmp/lsa-prof/0b7aa0d6d801-1-B (entries before: 0)
  run 3: == test core rc=0 END 2026-11-05 16:54:13 IST (0 min); tests passed 274 failed 0 ignored 0
  run 3: ROUTE-B DONE 2026-11-05 16:54:16 IST WITH NOT PROVED lines above
  (run 1's core figures are replaced by run 3's: at 3069f0c9 core's decision-citation gate failed, since the D-856x entries it checks were not yet written; its seven success-path lines were the only rows that differed.)
NOT PROVED by the route (each script run exited 3 for these lines alone; never counted covered):
  crates/core/benches/ratio.rs
  crates/core/src/lib.rs
  crates/costs/benches/ratio.rs
  crates/costs/benches/stack.rs
  crates/costs/src/lib.rs
  crates/engine/benches/ratio.rs
  crates/greeks/benches/ratio.rs
  crates/greeks/src/lib.rs
  crates/indicators/benches/ratio.rs
  crates/lake/benches/ratio.rs
  crates/lake/src/lib.rs
  crates/runner/benches/ratio.rs
  crates/store/benches/ratio.rs
  crates/store/benches/ratio/months.rs
  crates/store/benches/ratio/quarantine.rs
  crates/store/benches/ratio/safe_files.rs
  crates/store/src/lib.rs
  crates/telemetry/benches/ratio.rs
  crates/vocab/benches/ratio.rs
  benches/: [[bench]] test = false, so cargo test --no-run builds no object (L-BCL-1); src/lib.rs of core, costs, greeks, lake, store: no executable line (L-BCL-2).
Classes: (F) the failing side of an assertion or panic in test code (recipe v4 step 4, first clause); (F) bound/skip/collect/guard/fail LS1's step-4 rows (second clause, the main session's answer of 5 Nov as LS1 records it); (U) a region no input, file, variable or direct call reaches, with its proof in base-cov-libs-fu.txt.md; IGNORED the body of an #[ignore]d test or a helper only it calls (L-BCL-3); MAC a row only macOS takes, owed to the Mac's run; OPERATOR a row for the operator's ruling (L-BCL-6); GAP none. [source]: the shard whose list gave the class (with its line where the assembly moved it), or LSA for a row classed here by hand.

TOTALS per crate (lines covered/counted; branch sides covered/counted; files read; uncovered rows by class)
  core       lines 7746/7774 (uncovered 28)  branches 602/640 (uncovered 38)  files 21  rows {"(F)":66}
  costs      lines 11568/11670 (uncovered 102)  branches 480/502 (uncovered 22)  files 26  rows {"(F)":52,"IGNORED":70,"(U)":2}
  engine     lines 5484/5494 (uncovered 10)  branches 528/560 (uncovered 32)  files 11  rows {"(F)":42}
  greeks     lines 1918/1921 (uncovered 3)  branches 93/96 (uncovered 3)  files 7  rows {"(F)":6}
  indicators lines 16909/16914 (uncovered 5)  branches 1892/2078 (uncovered 186)  files 39  rows {"(F)":190,"(U)":1}
  lake       lines 5678/6135 (uncovered 457)  branches 305/364 (uncovered 59)  files 17  rows {"(F)":161,"(U)":2,"IGNORED":353}
  runner     lines 52070/52264 (uncovered 194)  branches 3671/4022 (uncovered 351)  files 75  rows {"(F)":277,"(U)":261,"OPERATOR":7}
  store      lines 53276/54692 (uncovered 1416)  branches 3449/3990 (uncovered 541)  files 112  rows {"(F)":648,"(U)":95,"(F) bound":25,"(F) guard":7,"IGNORED":1108,"(F) skip":28,"(F) fail":6,"(F) collect":30,"MAC":10}
  telemetry  lines 9370/9397 (uncovered 27)  branches 636/678 (uncovered 42)  files 16  rows {"(F)":62,"IGNORED":5,"OPERATOR":2}
  vocab      lines 8185/8205 (uncovered 20)  branches 1240/1298 (uncovered 58)  files 19  rows {"(F)":77,"(U)":1}
  ALL TEN    lines 172204/174466 (uncovered 2262)  branches 12896/14228 (uncovered 1332)  files 343
  rows by class: {"(F)":1581,"IGNORED":1536,"(U)":362,"(F) bound":25,"(F) guard":7,"(F) skip":28,"(F) fail":6,"(F) collect":30,"MAC":10,"OPERATOR":9}

PER FILE (every file read; lines a/b, branches c/d; uncovered rows by class)
  crates/core/src/alias.rs lines 857/859 branches 53/56  (F) 5
  crates/core/src/alias_cover_tests.rs lines 90/90 branches 0/0
  crates/core/src/blake3.rs lines 395/395 branches 16/16
  crates/core/src/error.rs lines 92/92 branches 4/4
  crates/core/src/instrument.rs lines 849/849 branches 50/54  (F) 4
  crates/core/src/isin.rs lines 140/140 branches 14/14
  crates/core/src/knob.rs lines 69/69 branches 8/8
  crates/core/src/price.rs lines 320/320 branches 28/28
  crates/core/src/secret_shape.rs lines 239/239 branches 34/34
  crates/core/src/symbol.rs lines 109/109 branches 4/4
  crates/core/src/universe.rs lines 966/968 branches 61/82  (F) 23
  crates/core/src/vendor.rs lines 1578/1579 branches 83/84  (F) 2
  crates/core/tests/banned_lists.rs lines 85/85 branches 2/4  (F) 2
  crates/core/tests/bound_probe.rs lines 35/35 branches 0/0
  crates/core/tests/claim_reader.rs lines 72/72 branches 2/4  (F) 2
  crates/core/tests/cost_invariants.rs lines 232/240 branches 30/30  (F) 8
  crates/core/tests/decision_citations.rs lines 433/439 branches 72/72  (F) 6
  crates/core/tests/findings.rs lines 494/501 branches 24/26  (F) 9
  crates/core/tests/graph.rs lines 496/496 branches 100/102  (F) 2
  crates/core/tests/lint.rs lines 107/108 branches 12/12  (F) 1
  crates/core/tests/zerodha_index.rs lines 88/89 branches 5/6  (F) 2
  crates/costs/src/charge_error.rs lines 184/184 branches 0/0
  crates/costs/src/dated.rs lines 386/386 branches 32/32
  crates/costs/src/day.rs lines 442/442 branches 28/30  (F) 2
  crates/costs/src/dhan_calc.rs lines 467/467 branches 30/30
  crates/costs/src/error.rs lines 283/283 branches 4/4
  crates/costs/src/exact.rs lines 305/305 branches 14/14
  crates/costs/src/expiry.rs lines 622/622 branches 28/28
  crates/costs/src/fill.rs lines 443/447 branches 16/16  (F) 4
  crates/costs/src/ledger.rs lines 877/880 branches 24/24  (F) 3
  crates/costs/src/lot.rs lines 667/670 branches 74/78  (F) 7
  crates/costs/src/lot_table_tests.rs lines 333/355 branches 10/14  (F) 6, IGNORED 20
  crates/costs/src/money.rs lines 153/153 branches 4/6  (F) 2
  crates/costs/src/moneyness.rs lines 355/355 branches 18/18
  crates/costs/src/rate.rs lines 117/117 branches 0/0
  crates/costs/src/regime.rs lines 534/534 branches 26/26
  crates/costs/src/schedule.rs lines 729/734 branches 12/14  (F) 7
  crates/costs/src/schedule_cover_tests.rs lines 68/68 branches 4/4
  crates/costs/src/scope.rs lines 82/83 branches 0/0  (F) 1
  crates/costs/src/stack.rs lines 1610/1612 branches 84/86  (U) 2, (F) 2
  crates/costs/src/stack_cover_tests.rs lines 51/51 branches 0/0
  crates/costs/src/strike.rs lines 317/317 branches 10/10
  crates/costs/src/trip.rs lines 1558/1558 branches 32/32
  crates/costs/src/venue.rs lines 77/77 branches 2/2
  crates/costs/tests/adversarial.rs lines 139/149 branches 10/16  (F) 16
  crates/costs/tests/dhan_calculator.rs lines 444/446 branches 12/12  (F) 2
  crates/costs/tests/fa73061.rs lines 325/375 branches 6/6  IGNORED 50
  crates/engine/src/column.rs lines 407/407 branches 16/16
  crates/engine/src/keep.rs lines 434/434 branches 24/28  (F) 4
  crates/engine/src/lib.rs lines 2246/2248 branches 126/146  (F) 22
  crates/engine/src/manifest.rs lines 285/285 branches 112/112
  crates/engine/src/refusal_cover_tests.rs lines 101/104 branches 2/2  (F) 3
  crates/engine/src/resume.rs lines 546/546 branches 132/132
  crates/engine/tests/adversarial.rs lines 159/159 branches 4/4
  crates/engine/tests/join_probe_screen.rs lines 31/31 branches 0/0
  crates/engine/tests/production_callers.rs lines 238/239 branches 34/36  (F) 3
  crates/engine/tests/resume_readiness.rs lines 851/855 branches 49/54  (F) 9
  crates/engine/tests/sweep_readiness_oracle.rs lines 186/186 branches 29/30  (F) 1
  crates/greeks/src/bsm.rs lines 524/524 branches 20/20
  crates/greeks/src/error.rs lines 100/100 branches 2/2
  crates/greeks/src/moneyness.rs lines 145/145 branches 4/4
  crates/greeks/src/normal.rs lines 143/143 branches 14/14
  crates/greeks/src/solver.rs lines 513/515 branches 41/44  (F) 5
  crates/greeks/tests/standalone.rs lines 59/59 branches 0/0
  crates/greeks/tests/vendor_anchor.rs lines 434/435 branches 12/12  (F) 1
  crates/indicators/src/anchored.rs lines 1682/1682 branches 103/106  (F) 3
  crates/indicators/src/column.rs lines 1244/1246 branches 62/70  (F) 10
  crates/indicators/src/column_cover_tests.rs lines 28/28 branches 0/0
  crates/indicators/src/daily.rs lines 695/695 branches 61/86  (F) 25
  crates/indicators/src/evaluator.rs lines 1705/1705 branches 136/146  (F) 10
  crates/indicators/src/evaluator_cover_tests.rs lines 55/55 branches 0/0
  crates/indicators/src/fib.rs lines 407/407 branches 24/26  (F) 2
  crates/indicators/src/gap.rs lines 724/724 branches 56/58  (F) 2
  crates/indicators/src/gap_cover_tests.rs lines 39/39 branches 0/0
  crates/indicators/src/lib.rs lines 773/773 branches 54/54
  crates/indicators/src/orb.rs lines 470/470 branches 51/58  (F) 7
  crates/indicators/src/orb_cover_tests.rs lines 62/62 branches 0/0
  crates/indicators/src/pattern.rs lines 1399/1399 branches 587/594  (F) 7
  crates/indicators/src/session.rs lines 698/698 branches 152/194  (F) 42
  crates/indicators/src/session_cover_tests.rs lines 26/26 branches 0/0
  crates/indicators/src/trend.rs lines 1257/1257 branches 161/168  (F) 7
  crates/indicators/src/trend_cover_tests.rs lines 42/42 branches 0/0
  crates/indicators/src/vwap.rs lines 951/951 branches 85/94  (U) 1, (F) 8
  crates/indicators/src/vwap_cover_tests.rs lines 35/35 branches 0/0
  crates/indicators/tests/audit_readiness.rs lines 49/49 branches 0/0
  crates/indicators/tests/crossing_known_readiness.rs lines 178/178 branches 37/38  (F) 1
  crates/indicators/tests/current_day_fib_known_readiness.rs lines 206/206 branches 20/22  (F) 2
  crates/indicators/tests/evaluator_position_count.rs lines 26/26 branches 0/0
  crates/indicators/tests/exact_minute_orb_gap_readiness.rs lines 355/355 branches 23/30  (F) 7
  crates/indicators/tests/extremes.rs lines 462/462 branches 14/14
  crates/indicators/tests/fib_known_readiness.rs lines 295/295 branches 20/24  (F) 4
  crates/indicators/tests/fib_rung_rounding.rs lines 182/182 branches 16/16
  crates/indicators/tests/gap_known_readiness.rs lines 337/337 branches 20/24  (F) 4
  crates/indicators/tests/invariants.rs lines 213/213 branches 0/0
  crates/indicators/tests/module_doc_counts.rs lines 211/214 branches 28/28  (F) 3
  crates/indicators/tests/orb_known_readiness.rs lines 259/259 branches 21/24  (F) 3
  crates/indicators/tests/session_from_outside.rs lines 186/186 branches 2/4  (F) 2
  crates/indicators/tests/session_known_readiness.rs lines 394/394 branches 33/46  (F) 13
  crates/indicators/tests/shared_core_doc.rs lines 184/184 branches 2/2
  crates/indicators/tests/sweep_predicate_readiness.rs lines 292/292 branches 72/96  (F) 24
  crates/indicators/tests/time_known_readiness.rs lines 145/145 branches 6/6
  crates/indicators/tests/trend_known_readiness.rs lines 306/306 branches 26/30  (F) 4
  crates/indicators/tests/withheld_cut_fold.rs lines 133/133 branches 8/8
  crates/indicators/tests/withheld_fold.rs lines 204/204 branches 12/12
  crates/lake/src/bar.rs lines 96/98 branches 2/2  (F) 2
  crates/lake/src/batch.rs lines 192/192 branches 12/12
  crates/lake/src/contract.rs lines 349/356 branches 25/26  (F) 8
  crates/lake/src/error.rs lines 383/383 branches 2/2
  crates/lake/src/footer.rs lines 190/190 branches 36/36
  crates/lake/src/footer_tests.rs lines 150/154 branches 5/6  (F) 5
  crates/lake/src/page.rs lines 639/644 branches 38/48  (F) 15
  crates/lake/src/reader.rs lines 1102/1135 branches 46/48  (F) 35
  crates/lake/src/reader_cover_tests.rs lines 31/33 branches 0/0  (F) 2
  crates/lake/src/schema.rs lines 205/206 branches 29/30  (U) 2
  crates/lake/src/schema_cover_tests.rs lines 76/78 branches 2/2  (F) 2
  crates/lake/src/zstd_v082_frames.rs lines 70/71 branches 8/8  (F) 1
  crates/lake/tests/events.rs lines 194/197 branches 18/18  (F) 3
  crates/lake/tests/real_lake.rs lines 0/132 branches 0/18  IGNORED 150
  crates/lake/tests/real_lake_regression.rs lines 0/126 branches 0/16  IGNORED 142
  crates/lake/tests/refusals.rs lines 1154/1269 branches 58/68  (F) 64, IGNORED 61
  crates/lake/tests/synthetic.rs lines 847/871 branches 24/24  (F) 24
  crates/runner/src/admission.rs lines 6265/6270 branches 305/310  (F) 5, (U) 5
  crates/runner/src/admission_reader_cover_tests.rs lines 445/445 branches 6/8  (F) 2
  crates/runner/src/admission_statistics_cover_tests.rs lines 235/235 branches 0/0
  crates/runner/src/align.rs lines 147/147 branches 8/8
  crates/runner/src/audit.rs lines 2119/2119 branches 141/154  (F) 13
  crates/runner/src/bootstrap.rs lines 1789/1810 branches 157/178  (U) 28, (F) 14
  crates/runner/src/bootstrap_family_pass.rs lines 269/269 branches 35/36  (U) 1
  crates/runner/src/bootstrap_family_pass_tests.rs lines 387/387 branches 6/8  (F) 2
  crates/runner/src/bootstrap_zero_v2.rs lines 323/324 branches 48/50  (U) 3
  crates/runner/src/bootstrap_zero_v2_tests.rs lines 426/426 branches 20/20
  crates/runner/src/bound.rs lines 228/228 branches 14/14
  crates/runner/src/closed.rs lines 146/146 branches 10/10
  crates/runner/src/excursion.rs lines 1130/1136 branches 112/120  (U) 9, (F) 5
  crates/runner/src/excursion_clock_tests.rs lines 89/89 branches 8/8
  crates/runner/src/exit_grid_policy.rs lines 5912/5937 branches 307/342  (U) 47, (F) 13
  crates/runner/src/exit_grid_policy_cover_tests.rs lines 90/90 branches 0/0
  crates/runner/src/expression.rs lines 212/212 branches 6/6
  crates/runner/src/expression_execution.rs lines 334/335 branches 29/30  (U) 2
  crates/runner/src/expression_execution_tests.rs lines 504/504 branches 0/0
  crates/runner/src/expression_oos.rs lines 250/252 branches 19/20  (U) 3
  crates/runner/src/expression_oos_tests.rs lines 400/400 branches 6/6
  crates/runner/src/expression_validation.rs lines 401/404 branches 81/90  (U) 12
  crates/runner/src/expression_validation_codec.rs lines 194/194 branches 24/24
  crates/runner/src/expression_validation_tests.rs lines 696/696 branches 6/6
  crates/runner/src/family_allocation_v1.rs lines 112/112 branches 16/16
  crates/runner/src/family_allocation_v1_tests.rs lines 71/71 branches 0/0
  crates/runner/src/grid.rs lines 5197/5232 branches 347/364  (U) 31, (F) 21
  crates/runner/src/grid_cover_tests.rs lines 806/806 branches 4/6  (F) 2
  crates/runner/src/identity.rs lines 655/655 branches 20/20
  crates/runner/src/lib.rs lines 956/957 branches 65/66  (F) 2
  crates/runner/src/outcome.rs lines 2321/2335 branches 267/304  (U) 15, (F) 36
  crates/runner/src/pbo.rs lines 265/265 branches 34/34
  crates/runner/src/portfolio.rs lines 750/750 branches 60/62  (F) 2
  crates/runner/src/rank.rs lines 707/709 branches 14/14  (F) 2
  crates/runner/src/replay_mask.rs lines 136/136 branches 14/14
  crates/runner/src/report.rs lines 996/996 branches 62/66  (F) 4
  crates/runner/src/report_cover_tests.rs lines 194/194 branches 8/12  (F) 4
  crates/runner/src/resample.rs lines 417/417 branches 22/26  (F) 4
  crates/runner/src/research_admission_projection.rs lines 139/139 branches 2/2
  crates/runner/src/research_admission_projection_codec.rs lines 27/27 branches 6/6
  crates/runner/src/research_exit_grid.rs lines 149/149 branches 6/6
  crates/runner/src/research_exit_grid_tests.rs lines 44/44 branches 0/0
  crates/runner/src/research_family.rs lines 163/163 branches 14/14
  crates/runner/src/reservation_refusal.rs lines 34/34 branches 2/2
  crates/runner/src/resolved_grid_view.rs lines 350/355 branches 74/74  (U) 3, OPERATOR 2
  crates/runner/src/search_allocation_v1.rs lines 101/101 branches 12/12
  crates/runner/src/search_allocation_v1_tests.rs lines 124/124 branches 0/0
  crates/runner/src/signal_candle_stop.rs lines 1201/1204 branches 236/248  (U) 15
  crates/runner/src/signal_candle_stop_tests.rs lines 1201/1202 branches 30/32  (F) 3
  crates/runner/src/significance.rs lines 507/510 branches 72/76  (F) 7
  crates/runner/src/significance_cover_tests.rs lines 42/42 branches 2/4  (F) 2
  crates/runner/src/split.rs lines 235/236 branches 36/38  (F) 3
  crates/runner/src/synthetic.rs lines 149/149 branches 8/14  (F) 6
  crates/runner/src/topn.rs lines 820/820 branches 69/74  (U) 5
  crates/runner/src/trade.rs lines 1808/1825 branches 184/222  (U) 23, (F) 32
  crates/runner/src/trade_clock_tests.rs lines 137/137 branches 2/4  (F) 2
  crates/runner/src/validate.rs lines 4292/4332 branches 511/570  (U) 59, OPERATOR 5, (F) 35
  crates/runner/src/validate_cover_tests.rs lines 1555/1560 branches 23/34  (F) 16
  crates/runner/tests/auto_probe_found_nothing.rs lines 71/71 branches 0/0
  crates/runner/tests/auto_probe_reporter_refusal.rs lines 47/47 branches 8/8
  crates/runner/tests/auto_reporting.rs lines 77/77 branches 0/0
  crates/runner/tests/base_cov_admission_words.rs lines 115/115 branches 7/8  (F) 1
  crates/runner/tests/checkpointed_rank_refusals.rs lines 43/43 branches 0/0
  crates/runner/tests/d_0694_record.rs lines 166/166 branches 10/10
  crates/runner/tests/end_to_end.rs lines 240/240 branches 2/4  (F) 2
  crates/runner/tests/expression_pricing.rs lines 158/158 branches 0/0
  crates/runner/tests/fib_rung_sweep.rs lines 119/119 branches 10/10
  crates/runner/tests/fixed_deadline_readiness.rs lines 211/211 branches 30/54  (F) 24
  crates/runner/tests/foreign_slice_deadline_readiness.rs lines 267/267 branches 12/22  (F) 10
  crates/runner/tests/hole_after_exit.rs lines 108/109 branches 4/4  (F) 1
  crates/runner/tests/join_answer_is_unchanged.rs lines 66/67 branches 0/0  (F) 1
  crates/runner/tests/population_door_halt.rs lines 33/33 branches 0/0
  crates/runner/tests/prefix_only_cadence.rs lines 124/124 branches 4/4
  crates/runner/tests/research_family_readiness.rs lines 445/445 branches 2/2
  crates/runner/tests/unwired_validation_record.rs lines 128/129 branches 12/12  (F) 1
  crates/store/src/block.rs lines 115/115 branches 10/10
  crates/store/src/catalog.rs lines 189/189 branches 50/50
  crates/store/src/catalog_cover_tests.rs lines 49/49 branches 0/0
  crates/store/src/catalog_tests.rs lines 338/341 branches 9/18  (F) 12
  crates/store/src/checksum_audit.rs lines 235/235 branches 29/30  (U) 1
  crates/store/src/checksum_audit_cover_tests.rs lines 59/59 branches 0/0
  crates/store/src/checksum_audit_tests.rs lines 400/403 branches 11/12  (F) bound 4
  crates/store/src/crc.rs lines 90/90 branches 8/8
  crates/store/src/emits.rs lines 300/306 branches 3/4  (F) 6, (F) guard 1
  crates/store/src/file.rs lines 4673/4726 branches 381/392  (U) 16, (F) 21, IGNORED 24, (F) guard 3
  crates/store/src/file_cover_tests.rs lines 64/64 branches 6/10  (F) 4
  crates/store/src/file_fault_tests.rs lines 571/575 branches 25/40  (F) 19
  crates/store/src/file_index_cover_tests.rs lines 421/423 branches 8/10  (F) 4
  crates/store/src/file_sealed.rs lines 256/272 branches 49/58  (U) 25
  crates/store/src/file_sealed_cover_tests.rs lines 132/132 branches 12/24  (F) 12
  crates/store/src/file_sealed_tests.rs lines 584/617 branches 25/36  (F) 18, IGNORED 26
  crates/store/src/file_write_door_tests.rs lines 726/744 branches 24/30  (F) skip 2, (F) 22
  crates/store/src/flock.rs lines 463/464 branches 50/52  (U) 3
  crates/store/src/flock_cover_tests.rs lines 298/301 branches 12/20  (F) 11
  crates/store/src/flock_patient_tests.rs lines 495/506 branches 23/36  (F) 19, (F) fail 1, (F) collect 4
  crates/store/src/flock_tests.rs lines 583/590 branches 22/30  (F) 14, (F) fail 1
  crates/store/src/format.rs lines 278/278 branches 42/42
  crates/store/src/generation.rs lines 326/326 branches 60/60
  crates/store/src/generation_cover_tests.rs lines 118/118 branches 3/6  (F) 3
  crates/store/src/generation_door_cover_tests.rs lines 159/166 branches 2/4  (F) 9
  crates/store/src/generation_tests.rs lines 37/37 branches 0/0
  crates/store/src/header.rs lines 524/524 branches 76/76
  crates/store/src/header_audit_tests.rs lines 49/49 branches 0/0
  crates/store/src/layout.rs lines 186/186 branches 40/40
  crates/store/src/month_summary.rs lines 682/683 branches 191/194  (U) 4
  crates/store/src/month_summary_cover_tests.rs lines 147/147 branches 0/0
  crates/store/src/month_summary_decode_cover_tests.rs lines 106/107 branches 0/0  (F) 1
  crates/store/src/month_summary_tests.rs lines 559/562 branches 32/52  (F) bound 2, (F) 21
  crates/store/src/month_summary_writer.rs lines 165/165 branches 34/34
  crates/store/src/month_summary_writer_cover_tests.rs lines 175/175 branches 2/4  (F) 2
  crates/store/src/open_flags.rs lines 6/6 branches 0/0
  crates/store/src/path.rs lines 206/206 branches 30/30
  crates/store/src/provenance.rs lines 766/766 branches 150/152  (U) 2
  crates/store/src/provenance_cover_tests.rs lines 255/265 branches 2/4  (F) 12
  crates/store/src/quarantine.rs lines 1490/1490 branches 188/188
  crates/store/src/quarantine_cover_tests.rs lines 516/520 branches 1/2  (F) 5
  crates/store/src/quarantine_tests.rs lines 1907/1926 branches 54/90  (F) 55
  crates/store/src/repair.rs lines 742/745 branches 132/132  (U) 3
  crates/store/src/repair_cover_tests.rs lines 67/67 branches 3/4  (F) 1
  crates/store/src/repair_door_cover_tests.rs lines 453/463 branches 6/8  (F) 12
  crates/store/src/repair_tests.rs lines 639/643 branches 15/18  (F) 5, (F) skip 2
  crates/store/src/safe_open.rs lines 170/177 branches 21/22  (U) 8
  crates/store/src/safe_open_tests.rs lines 315/327 branches 8/16  (F) skip 5, (F) 13, (F) bound 2
  crates/store/src/sealed.rs lines 1434/1434 branches 271/274  (U) 3
  crates/store/src/sealed_codec.rs lines 646/646 branches 90/90
  crates/store/src/sealed_codec_tests.rs lines 996/1000 branches 30/34  (F) 8
  crates/store/src/sealed_fold_cover_tests.rs lines 28/28 branches 0/0
  crates/store/src/sealed_tests.rs lines 1723/1741 branches 34/40  (F) 22, (F) fail 1, (F) guard 1
  crates/store/src/settings.rs lines 749/749 branches 148/150  (U) 2
  crates/store/src/settings_cover_tests.rs lines 187/187 branches 1/2  (F) 1
  crates/store/src/settings_door_cover_tests.rs lines 233/238 branches 8/16  (F) 13
  crates/store/src/settings_tests.rs lines 835/846 branches 17/28  (F) 14, (F) bound 8
  crates/store/src/sidecar_checksum_tests.rs lines 391/391 branches 10/10
  crates/store/src/small_month_tests.rs lines 106/106 branches 1/2  (F) 1
  crates/store/src/sort_index.rs lines 794/794 branches 126/126
  crates/store/src/sort_index_cover_tests.rs lines 230/230 branches 5/10  (F) 5
  crates/store/src/sort_index_tests.rs lines 385/385 branches 9/10  (F) 1
  crates/store/src/sort_index_writer.rs lines 388/388 branches 62/62
  crates/store/src/sort_index_writer_tests.rs lines 114/114 branches 0/0
  crates/store/src/staging.rs lines 110/110 branches 14/14
  crates/store/src/staging_cover_tests.rs lines 46/48 branches 0/0  (F) 2
  crates/store/src/staging_tests.rs lines 222/223 branches 8/16  (F) 9
  crates/store/src/time_index.rs lines 744/744 branches 71/80  (F) 9
  crates/store/src/verdict.rs lines 2679/2685 branches 296/310  (U) 18, (F) 2
  crates/store/src/verdict_tests.rs lines 594/601 branches 24/26  (F) 7, (F) bound 2
  crates/store/tests/admission.rs lines 182/182 branches 0/0
  crates/store/tests/base_cov_words.rs lines 46/46 branches 0/0
  crates/store/tests/bisect_cost.rs lines 80/80 branches 0/0
  crates/store/tests/catalog.rs lines 549/568 branches 5/12  MAC 10, (F) 11, (F) skip 4, (U) 1
  crates/store/tests/cited_commits.rs lines 523/547 branches 74/90  (F) skip 12, (F) collect 19, (F) 7, (F) fail 1, (U) 1
  crates/store/tests/cloud_mutants_store.rs lines 2190/2203 branches 62/96  (F) 41, (F) guard 2, (F) bound 3, (F) skip 1
  crates/store/tests/cold_read.rs lines 199/199 branches 4/4
  crates/store/tests/docs.rs lines 239/239 branches 6/12  (F) 6
  crates/store/tests/durability.rs lines 194/195 branches 0/0  (F) 1
  crates/store/tests/fault.rs lines 471/472 branches 10/12  (F) 3
  crates/store/tests/fifo.rs lines 194/201 branches 7/12  (F) 6, (F) bound 4, (F) skip 2
  crates/store/tests/generation.rs lines 483/492 branches 8/18  (F) 19
  crates/store/tests/geometry.rs lines 433/433 branches 9/12  (F) 2, (F) collect 1
  crates/store/tests/libm_key.rs lines 778/780 branches 8/12  (F) fail 1, (F) 5
  crates/store/tests/misfiled.rs lines 211/213 branches 11/14  (F) 5
  crates/store/tests/month_repair.rs lines 605/613 branches 13/18  (F) 10, (F) collect 3
  crates/store/tests/month_summary_log.rs lines 104/104 branches 0/0
  crates/store/tests/open_flags.rs lines 97/98 branches 17/18  (F) collect 2
  crates/store/tests/provenance.rs lines 1222/1238 branches 0/0  (F) 16
  crates/store/tests/repair.rs lines 384/384 branches 4/6  (F) 2
  crates/store/tests/roundtrip.rs lines 111/111 branches 0/0
  crates/store/tests/seal_buffer.rs lines 100/100 branches 0/0
  crates/store/tests/sealed_codec_alloc.rs lines 230/232 branches 15/28  (F) 15
  crates/store/tests/sealed_codec_samples.rs lines 0/366 branches 0/44  IGNORED 410
  crates/store/tests/sealed_door.rs lines 126/211 branches 6/20  IGNORED 95, (F) 4
  crates/store/tests/sealed_samples.rs lines 0/174 branches 0/12  IGNORED 186
  crates/store/tests/sealed_write_door.rs lines 0/165 branches 0/24  IGNORED 189
  crates/store/tests/slot_audit.rs lines 348/354 branches 8/10  (F) 8
  crates/store/tests/slot_audit_sealed.rs lines 88/96 branches 0/2  (F) 5, (U) 5
  crates/store/tests/slot_damaged_log.rs lines 153/160 branches 0/0  (F) 7
  crates/store/tests/sort_index.rs lines 599/602 branches 14/22  (F) 11
  crates/store/tests/sort_index_log.rs lines 138/138 branches 0/0
  crates/store/tests/support/mod.rs lines 32/32 branches 3/4  (F) fail 1
  crates/store/tests/tail_proof.rs lines 175/175 branches 3/6  (F) 3
  crates/store/tests/tix.rs lines 412/413 branches 11/12  (F) 1, (F) collect 1
  crates/store/tests/tix_latency.rs lines 0/164 branches 0/14  IGNORED 178
  crates/store/tests/tix_rebuilder.rs lines 342/342 branches 7/8  (F) 1
  crates/store/tests/tix_small_month.rs lines 400/402 branches 7/8  (F) 3
  crates/store/tests/tix_warn_once.rs lines 221/224 branches 0/0  (F) 3
  crates/store/tests/unit.rs lines 1711/1721 branches 30/40  (F) 19, (U) 1
  crates/store/tests/verdict.rs lines 1989/2007 branches 28/44  (F) 32, (U) 2
  crates/store/tests/write.rs lines 1199/1203 branches 5/8  (F) 7
  crates/telemetry/src/clock.rs lines 192/192 branches 14/14
  crates/telemetry/src/encode.rs lines 206/206 branches 20/20
  crates/telemetry/src/event.rs lines 87/87 branches 0/0
  crates/telemetry/src/json.rs lines 341/341 branches 16/16
  crates/telemetry/src/level.rs lines 68/69 branches 0/0  (F) 1
  crates/telemetry/src/lib.rs lines 203/203 branches 16/18  (F) 2
  crates/telemetry/src/record.rs lines 425/426 branches 28/28  (F) 1
  crates/telemetry/src/run_dirs.rs lines 79/79 branches 4/8  (F) 4
  crates/telemetry/src/sink.rs lines 6087/6106 branches 440/468  (F) 42, IGNORED 5
  crates/telemetry/src/sink_recovery_tests.rs lines 246/246 branches 6/8  (F) 2
  crates/telemetry/src/state_events.rs lines 28/28 branches 12/12
  crates/telemetry/src/state_events_tests.rs lines 127/130 branches 6/10  (F) 7
  crates/telemetry/src/tail.rs lines 863/863 branches 72/74  (F) 2
  crates/telemetry/src/value.rs lines 142/142 branches 0/0
  crates/telemetry/tests/cloud_mutants_sink.rs lines 236/237 branches 2/2  (F) 1
  crates/telemetry/tests/global_absent.rs lines 40/42 branches 0/0  OPERATOR 2
  crates/vocab/src/error.rs lines 103/103 branches 0/0
  crates/vocab/src/expression.rs lines 559/559 branches 78/78
  crates/vocab/src/expression_search.rs lines 459/459 branches 86/96  (F) 10
  crates/vocab/src/implication.rs lines 157/158 branches 25/26  (F) 2
  crates/vocab/src/lib.rs lines 55/55 branches 0/0
  crates/vocab/src/mask.rs lines 273/273 branches 14/18  (F) 4
  crates/vocab/src/table.rs lines 434/435 branches 53/60  (U) 1, (F) 7
  crates/vocab/src/tolerance.rs lines 171/171 branches 17/18  (F) 1
  crates/vocab/tests/expression.rs lines 382/383 branches 24/26  (F) 3
  crates/vocab/tests/expression_search.rs lines 221/222 branches 10/12  (F) 3
  crates/vocab/tests/expression_search_readiness.rs lines 122/123 branches 1/2  (F) 2
  crates/vocab/tests/implication_pointers.rs lines 72/72 branches 2/2
  crates/vocab/tests/mask.rs lines 211/211 branches 23/24  (F) 1
  crates/vocab/tests/no_float.rs lines 79/79 branches 5/6  (F) 1
  crates/vocab/tests/no_py_language.rs lines 516/523 branches 55/56  (F) 8
  crates/vocab/tests/no_runtime_spawn.rs lines 2889/2891 branches 706/720  (F) 16
  crates/vocab/tests/stale_claims.rs lines 287/288 branches 14/14  (F) 1
  crates/vocab/tests/table.rs lines 498/501 branches 55/68  (F) 16
  crates/vocab/tests/workspace_is_rust.rs lines 697/699 branches 72/72  (F) 2

ROWS: every uncovered line and branch side. CLASS  KIND file:line[:col side]  [source of the tag]  text
(F)         LINE   crates/core/src/alias.rs:1276  [LS4]  other => panic!("{asked}: {other:?}"),
(F)         LINE   crates/core/src/alias.rs:1320  [LS4]  other => panic!("{asked}: {other:?}"),
(F)         BRANCH crates/core/src/alias.rs:1818:58 False  [LS4]  assert!(matches!(got.via, Via::Index(row) if row.store_key == key));
(F)         BRANCH crates/core/src/alias.rs:1827:61 False  [LS4]  assert!(matches!(ruchi.via, Via::Renamed(change) if change.old == "RUCHI"));
(F)         BRANCH crates/core/src/alias.rs:1866:36 False  [LS4]  assert!(hit || !name.is_empty());
(F)         BRANCH crates/core/src/instrument.rs:779:17 False  [LS4]  assert!(a < b && b < c);
(F)         BRANCH crates/core/src/instrument.rs:779:26 False  [LS4]  assert!(a < b && b < c);
(F)         BRANCH crates/core/src/instrument.rs:1698:17 False  [LS4]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F)         BRANCH crates/core/src/instrument.rs:1698:55 False  [LS4]  assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
(F)         BRANCH crates/core/src/universe.rs:4755:17 False  [LS4]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4755:38 False  [LS4]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4755:60 False  [LS4]  assert!(idx.contains("A") && idx.contains("BB") && idx.contains("CCC"));
(F)         BRANCH crates/core/src/universe.rs:4767:17 False  [LS4]  assert!(tight.contains("X") && tight.contains("Y"));
(F)         BRANCH crates/core/src/universe.rs:4767:40 False  [LS4]  assert!(tight.contains("X") && tight.contains("Y"));
(F)         BRANCH crates/core/src/universe.rs:4801:17 False  [LS4]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4801:42 False  [LS4]  assert!(!NTM_INDEX.is_empty() && !FNO_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4802:17 False  [LS4]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4802:48 False  [LS4]  assert!(!NIFTY_500_INDEX.is_empty() && !NIFTY_50_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4803:17 False  [LS4]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4803:48 False  [LS4]  assert!(!NIFTY_200_INDEX.is_empty() && !NIFTY_100_INDEX.is_empty());
(F)         BRANCH crates/core/src/universe.rs:4836:21 False  [LS4]  let Some(held) = idx.slots[at] else {
(F)         LINE   crates/core/src/universe.rs:4837  [LS4]  panic!("{m} is a member, yet its probe reached an empty slot");
(F)         BRANCH crates/core/src/universe.rs:5628:17 False  [LS4]  u.contains(Universe::NIFTY_50)
(F)         BRANCH crates/core/src/universe.rs:5629:24 False  [LS4]  && u.contains(Universe::NIFTY_100)
(F)         BRANCH crates/core/src/universe.rs:5630:24 False  [LS4]  && u.contains(Universe::NIFTY_200)
(F)         BRANCH crates/core/src/universe.rs:5631:24 False  [LS4]  && u.contains(Universe::NIFTY_500)
(F)         BRANCH crates/core/src/universe.rs:5632:24 False  [LS4]  && u.contains(Universe::TOTAL_MARKET),
(F)         LINE   crates/core/src/universe.rs:5634  [LS4]  u.bits()
(F)         BRANCH crates/core/src/universe.rs:5640:17 False  [LS4]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F)         BRANCH crates/core/src/universe.rs:5640:52 False  [LS4]  u.contains(Universe::NIFTY_500) && u.contains(Universe::TOTAL_MARKET),
(F)         BRANCH crates/core/src/universe.rs:5792:17 False  [LS4]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F)         BRANCH crates/core/src/universe.rs:5792:46 False  [LS4]  assert!(r.contains(Universe::FNO) && r.contains(Universe::TOTAL_MARKET));
(F)         BRANCH crates/core/src/vendor.rs:3065:25 False  [LS4]  let Some(held) = idx.slots[at] else {
(F)         LINE   crates/core/src/vendor.rs:3066  [LS4]  panic!("{m} is a member, yet its probe reached an empty slot");
(F)         BRANCH crates/core/tests/banned_lists.rs:113:9 False  [LS4]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F)         BRANCH crates/core/tests/banned_lists.rs:113:33 False  [LS4]  forbidden.len() > 20 && forbidden.contains("pyo3"),
(F)         BRANCH crates/core/tests/claim_reader.rs:125:13 False  [LS4]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F)         BRANCH crates/core/tests/claim_reader.rs:125:33 False  [LS4]  fields[1] != "-" && !fields[1].bytes().all(|b| b.is_ascii_digit()),
(F)         LINE   crates/core/tests/cost_invariants.rs:88  [LS4]  panic!(
(F)         LINE   crates/core/tests/cost_invariants.rs:90  [LS4]  path.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:109  [LS4]  panic!(
(F)         LINE   crates/core/tests/cost_invariants.rs:111  [LS4]  crates_dir.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:136  [LS4]  crates_dir.display()
(F)         LINE   crates/core/tests/cost_invariants.rs:364  [LS4]  rows.len()
(F)         LINE   crates/core/tests/cost_invariants.rs:371  [LS4]  printed.len(),
(F)         LINE   crates/core/tests/cost_invariants.rs:372  [LS4]  benches.len()
(F)         LINE   crates/core/tests/decision_citations.rs:243  [LS4]  path.display()
(F)         LINE   crates/core/tests/decision_citations.rs:313  [LS4]  cited.len()
(F)         LINE   crates/core/tests/decision_citations.rs:323  [LS4]  missing.len()
(F)         LINE   crates/core/tests/decision_citations.rs:419  [LS4]  citations(&none[0].1)
(F)         LINE   crates/core/tests/decision_citations.rs:605  [LS4]  panic!("{file}: `{name}` = `{rhs}` holds `{term}`, which this test cannot read")
(F)         LINE   crates/core/tests/decision_citations.rs:608  [LS4]  || panic!("{file} is not under a crate's src/"),
(F)         LINE   crates/core/tests/findings.rs:94  [LS4]  cells.len()
(F)         LINE   crates/core/tests/findings.rs:296  [LS4]  rows.len()
(F)         LINE   crates/core/tests/findings.rs:336  [LS4]  counted + rest,
(F)         LINE   crates/core/tests/findings.rs:337  [LS4]  rows.len()
(F)         LINE   crates/core/tests/findings.rs:364  [LS4]  rows.len()
(F)         BRANCH crates/core/tests/findings.rs:369:13 False  [LS4]  cells.len() >= 3 && cells[2].len() > 20,
(F)         BRANCH crates/core/tests/findings.rs:369:33 False  [LS4]  cells.len() >= 3 && cells[2].len() > 20,
(F)         LINE   crates/core/tests/findings.rs:609  [LS4]  String::from_utf8_lossy(&output.stderr)
(F)         LINE   crates/core/tests/findings.rs:993  [LS4]  String::from_utf8_lossy(&output.stderr)
(F)         BRANCH crates/core/tests/graph.rs:528:33 False  [LS4]  !deps.is_empty() || claim.to_lowercase().contains("nothing"),
(F)         BRANCH crates/core/tests/graph.rs:563:44 False  [LS4]  !name.contains('*') && !name.contains('/') && !name.is_empty(),
(F)         LINE   crates/core/tests/lint.rs:139  [LS4]  &next_fn[..next_fn.find('(').unwrap_or(0)],
(F)         BRANCH crates/core/tests/zerodha_index.rs:59:13 False  [LS4]  let Decoded::Keep(listing) = decoded else {
(F)         LINE   crates/core/tests/zerodha_index.rs:60  [LS4]  panic!("{spelled} must be KEPT, not skipped: {decoded:?}");
(F)         BRANCH crates/costs/src/day.rs:897:25 False  [LS4]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F)         BRANCH crates/costs/src/day.rs:897:57 False  [LS4]  assert!(last.day() >= SHORTEST_MONTH && last.day() <= LONGEST_MONTH);
(F)         LINE   crates/costs/src/fill.rs:1010  [LS4]  best.buy().raw(),
(F)         LINE   crates/costs/src/fill.rs:1011  [LS4]  worst.buy().raw()
(F)         LINE   crates/costs/src/fill.rs:1017  [LS4]  best.sell().raw(),
(F)         LINE   crates/costs/src/fill.rs:1018  [LS4]  worst.sell().raw()
(F)         LINE   crates/costs/src/ledger.rs:1345  [LS4]  LINE_NAMES[at],
(F)         LINE   crates/costs/src/ledger.rs:1346  [LS4]  summed[at],
(F)         LINE   crates/costs/src/ledger.rs:1347  [LS4]  closed[at]
(F)         BRANCH crates/costs/src/lot.rs:1497:21 False  [LS4]  let LotBasis::ContractRule(citation) = got.basis else {
(F)         LINE   crates/costs/src/lot.rs:1498  [LS4]  panic!("{subject} {on:?}: {:?}", got.basis);
(F)         BRANCH crates/costs/src/lot.rs:1554:25 False  [LS4]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F)         BRANCH crates/costs/src/lot.rs:1554:57 False  [LS4]  assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
(F)         BRANCH crates/costs/src/lot.rs:1589:17 False  [LS4]  let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
(F)         LINE   crates/costs/src/lot.rs:1590  [LS4]  panic!("{subject} {on:?}: an unread revision answered a lot");
(F)         LINE   crates/costs/src/lot.rs:1614  [LS4]  matches!(
(F)         BRANCH crates/costs/src/lot_table_tests.rs:112:52 False  [LS4]  assert!(sha == FAOP64625_SHA256 || sha == FAOP70616_SHA256);
(F)         BRANCH crates/costs/src/lot_table_tests.rs:117:25 False  [LS4]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F)         BRANCH crates/costs/src/lot_table_tests.rs:117:56 False  [LS4]  assert!(row.source_sha256.is_none() && row.read.is_none());
(F)         LINE   crates/costs/src/lot_table_tests.rs:146  [LS4]  other => panic!("expected the unverified row, got {other:?}"),
(F)         LINE   crates/costs/src/lot_table_tests.rs:156  [LS4]  other => panic!("expected before-first-row, got {other:?}"),
(F)         BRANCH crates/costs/src/lot_table_tests.rs:187:56 False  [LS4]  Err(LotAbsence::BeforeFirstRow { gap, .. }) if gap != LOT_GAP
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:382  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn a_lookup_is_flat_in_the_table() {]  fn a_lookup_is_flat_in_the_table() {
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:383  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn table_of(underlyings: usize) -> (LotTable, Vec<String>) {]  fn table_of(underlyings: usize) -> (LotTable, Vec<String>) {
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:384  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let names: Vec<String> = (0..underlyings).map(|at| format!("U{at:05}")).collect();]  let names: Vec<String> = (0..underlyings).map(|at| format!("U{at:05}")).collect();
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:385  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let mut rows = Vec::new();]  let mut rows = Vec::new();
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:386  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for name in &names {]  for name in &names {
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:387  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for step in 0..MAX_LOT_ROWS_PER_UNDERLYING {]  for step in 0..MAX_LOT_ROWS_PER_UNDERLYING {
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:388  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let ordinal = day(2018, 1, 1).ordinal() + i32::try_from(step * 45).unwrap();]  let ordinal = day(2018, 1, 1).ordinal() + i32::try_from(step * 45).unwrap();
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:389  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  rows.push(sourced(name, TradeDay::from_ordinal(ordinal).unwrap(), 25));]  rows.push(sourced(name, TradeDay::from_ordinal(ordinal).unwrap(), 25));
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:390  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }]  }
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:392  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  (LotTable::from_rows(rows).unwrap(), names)]  (LotTable::from_rows(rows).unwrap(), names)
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:393  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }]  }
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:394  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (small, small_names) = table_of(2);]  let (small, small_names) = table_of(2);
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:395  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (large, large_names) = table_of(1_500);]  let (large, large_names) = table_of(1_500);
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:396  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let n = 100_000;]  let n = 100_000;
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:397  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (s50, s99, smax) = time_lookups(&small, &small_names, n);]  let (s50, s99, smax) = time_lookups(&small, &small_names, n);
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:398  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (l50, l99, lmax) = time_lookups(&large, &large_names, n);]  let (l50, l99, lmax) = time_lookups(&large, &large_names, n);
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:399  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:403  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(]  assert!(
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:404  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  l99 <= s99.saturating_mul(10).max(2_000),]  l99 <= s99.saturating_mul(10).max(2_000),
IGNORED     LINE   crates/costs/src/lot_table_tests.rs:407  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }]  }
(F)         BRANCH crates/costs/src/money.rs:383:17 False  [LS4]  assert!(widest > 0 && widest < i128::MAX);
(F)         BRANCH crates/costs/src/money.rs:383:31 False  [LS4]  assert!(widest > 0 && widest < i128::MAX);
(F)         LINE   crates/costs/src/schedule.rs:1213  [LS4]  rate.source()
(F)         BRANCH crates/costs/src/schedule.rs:1244:75 False  [LS4]  matches!(refusal, ChargeError::OutOfScope { what, .. } if what == name),
(F)         LINE   crates/costs/src/schedule.rs:1280  [LS4]  rate.source()
(F)         LINE   crates/costs/src/schedule.rs:1288  [LS4]  rate.source()
(F)         BRANCH crates/costs/src/schedule.rs:1633:29 False  [LS4]  let Taxes::Options { stt_sell, .. } = card.taxes() else {
(F)         LINE   crates/costs/src/schedule.rs:1634  [LS4]  panic!("an options card carries option taxes");
(F)         LINE   crates/costs/src/schedule.rs:1643  [LS4]  (card, trip) => panic!(
(F)         LINE   crates/costs/src/scope.rs:249  [LS4]  lines[arm]
(U)         LINE   crates/costs/src/stack.rs:720  [LS4]  )?;
(U)         LINE   crates/costs/src/stack.rs:907  [LS4]  )?)
(F)         BRANCH crates/costs/src/stack.rs:2681:17 False  [LS4]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F)         BRANCH crates/costs/src/stack.rs:2681:43 False  [LS4]  assert!(thin.gross().raw() > 0 && !thin.is_win());
(F)         LINE   crates/costs/tests/adversarial.rs:114  [LS4]  c.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:115  [LS4]  f.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:121  [LS4]  c.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:122  [LS4]  f.raw()
(F)         BRANCH crates/costs/tests/adversarial.rs:128:21 False  [LS4]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F)         BRANCH crates/costs/tests/adversarial.rs:128:44 False  [LS4]  assert!(notional.raw() >= 0 && rate.get() >= 0, "{where_}: fixture");
(F)         LINE   crates/costs/tests/adversarial.rs:134  [LS4]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:143  [LS4]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:151  [LS4]  levy.raw()
(F)         BRANCH crates/costs/tests/adversarial.rs:194:21 False  [LS4]  ceil.is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:195:28 False  [LS4]  && floor.is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:196:28 False  [LS4]  && levy_ceiling(notional, rate).is_ok()
(F)         BRANCH crates/costs/tests/adversarial.rs:197:28 False  [LS4]  && statutory_levy(notional, rate).is_ok(),
(F)         LINE   crates/costs/tests/adversarial.rs:263  [LS4]  n.raw(),
(F)         LINE   crates/costs/tests/adversarial.rs:264  [LS4]  levy.raw()
(F)         LINE   crates/costs/tests/adversarial.rs:301  [LS4]  out.map(Paisa::raw)
(F)         LINE   crates/costs/tests/dhan_calculator.rs:666  [LS4]  case.page[7]
(F)         LINE   crates/costs/tests/dhan_calculator.rs:931  [LS4]  case.name, dated_lines[at].0
IGNORED     LINE   crates/costs/tests/fa73061.rs:549  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn the_dated_lookups_cost_the_same_on_every_row_measured() {]  fn the_dated_lookups_cost_the_same_on_every_row_measured() {
IGNORED     LINE   crates/costs/tests/fa73061.rs:552  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let days = []  let days = [
IGNORED     LINE   crates/costs/tests/fa73061.rs:553  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2020-06-01 (refusal)", day(2020, 6, 1)),]  ("2020-06-01 (refusal)", day(2020, 6, 1)),
IGNORED     LINE   crates/costs/tests/fa73061.rs:554  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2024-10-01 (FA64232)", day(2024, 10, 1)),]  ("2024-10-01 (FA64232)", day(2024, 10, 1)),
IGNORED     LINE   crates/costs/tests/fa73061.rs:555  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2026-02-27 (FA64232)", day(2026, 2, 27)),]  ("2026-02-27 (FA64232)", day(2026, 2, 27)),
IGNORED     LINE   crates/costs/tests/fa73061.rs:556  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2026-03-01 (FA73061)", day(2026, 3, 1)),]  ("2026-03-01 (FA73061)", day(2026, 3, 1)),
IGNORED     LINE   crates/costs/tests/fa73061.rs:557  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ("2100-12-31 (FA73061)", TradeDay::MAX),]  ("2100-12-31 (FA73061)", TradeDay::MAX),
IGNORED     LINE   crates/costs/tests/fa73061.rs:558  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ];]  ];
IGNORED     LINE   crates/costs/tests/fa73061.rs:559  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  for (label, on) in days {]  for (label, on) in days {
IGNORED     LINE   crates/costs/tests/fa73061.rs:560  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let exchange = exchange_charge_rate(Exchange::Nse, on);]  let exchange = exchange_charge_rate(Exchange::Nse, on);
IGNORED     LINE   crates/costs/tests/fa73061.rs:561  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let ipft = ipft_rate(Exchange::Nse, on);]  let ipft = ipft_rate(Exchange::Nse, on);
IGNORED     LINE   crates/costs/tests/fa73061.rs:562  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
IGNORED     LINE   crates/costs/tests/fa73061.rs:563  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  exchange_charge_rate(black_box(Exchange::Nse), black_box(on))]  exchange_charge_rate(black_box(Exchange::Nse), black_box(on))
IGNORED     LINE   crates/costs/tests/fa73061.rs:564  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });]  });
IGNORED     LINE   crates/costs/tests/fa73061.rs:565  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:568  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);]  assert!(p99 < 10_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:569  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
IGNORED     LINE   crates/costs/tests/fa73061.rs:570  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  ipft_rate(black_box(Exchange::Nse), black_box(on))]  ipft_rate(black_box(Exchange::Nse), black_box(on))
IGNORED     LINE   crates/costs/tests/fa73061.rs:571  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });]  });
IGNORED     LINE   crates/costs/tests/fa73061.rs:572  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:575  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);]  assert!(p99 < 10_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:576  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
IGNORED     LINE   crates/costs/tests/fa73061.rs:577  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  Rates::resolve(]  Rates::resolve(
IGNORED     LINE   crates/costs/tests/fa73061.rs:578  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Broker::Groww),]  black_box(Broker::Groww),
IGNORED     LINE   crates/costs/tests/fa73061.rs:579  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Exchange::Nse),]  black_box(Exchange::Nse),
IGNORED     LINE   crates/costs/tests/fa73061.rs:580  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(on),]  black_box(on),
IGNORED     LINE   crates/costs/tests/fa73061.rs:582  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });]  });
IGNORED     LINE   crates/costs/tests/fa73061.rs:583  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:586  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);]  assert!(p99 < 10_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:589  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Da]  let card = RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated);
IGNORED     LINE   crates/costs/tests/fa73061.rs:590  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || {
IGNORED     LINE   crates/costs/tests/fa73061.rs:591  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  RateCard::resolve(]  RateCard::resolve(
IGNORED     LINE   crates/costs/tests/fa73061.rs:592  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(TradeKind::Options),]  black_box(TradeKind::Options),
IGNORED     LINE   crates/costs/tests/fa73061.rs:593  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(Exchange::Nse),]  black_box(Exchange::Nse),
IGNORED     LINE   crates/costs/tests/fa73061.rs:594  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(on),]  black_box(on),
IGNORED     LINE   crates/costs/tests/fa73061.rs:595  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  black_box(RateSelector::Dated),]  black_box(RateSelector::Dated),
IGNORED     LINE   crates/costs/tests/fa73061.rs:597  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  });]  });
IGNORED     LINE   crates/costs/tests/fa73061.rs:598  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:601  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);]  assert!(p99 < 10_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:602  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:603  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),]  RateCard::resolve(TradeKind::Options, Exchange::Nse, on, RateSelector::Dated),
IGNORED     LINE   crates/costs/tests/fa73061.rs:607  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(exchange_charge_rate(Exchange::Nse, on), exchange);]  assert_eq!(exchange_charge_rate(Exchange::Nse, on), exchange);
IGNORED     LINE   crates/costs/tests/fa73061.rs:608  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(ipft_rate(Exchange::Nse, on), ipft);]  assert_eq!(ipft_rate(Exchange::Nse, on), ipft);
IGNORED     LINE   crates/costs/tests/fa73061.rs:610  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let on = day(2026, 3, 2);]  let on = day(2026, 3, 2);
IGNORED     LINE   crates/costs/tests/fa73061.rs:611  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let trip = nifty_trip(on, on, 10_000, 12_000);]  let trip = nifty_trip(on, on, 10_000, 12_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:612  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || price(black_box(&trip)));]  let (p50, p99, max) = time_per_call(BATCHES, PER_BATCH, || price(black_box(&trip)));
IGNORED     LINE   crates/costs/tests/fa73061.rs:613  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  println!(]  println!(
IGNORED     LINE   crates/costs/tests/fa73061.rs:616  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert!(p99 < 10_000);]  assert!(p99 < 10_000);
IGNORED     LINE   crates/costs/tests/fa73061.rs:617  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  assert_eq!(price(&trip).expect("verified").exchange().raw(), 510);]  assert_eq!(price(&trip).expect("verified").exchange().raw(), 510);
IGNORED     LINE   crates/costs/tests/fa73061.rs:618  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }]  }
(F)         BRANCH crates/engine/src/keep.rs:855:17 False  [LS4]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F)         BRANCH crates/engine/src/keep.rs:855:39 False  [LS4]  assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");
(F)         BRANCH crates/engine/src/keep.rs:862:13 False  [LS4]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F)         BRANCH crates/engine/src/keep.rs:862:35 False  [LS4]  !lost.reconciles() && !Tally::of(&lost).reconciles(),
(F)         BRANCH crates/engine/src/lib.rs:2858:13 False  [LS4 line 2857]  !out_par.is_empty() && inf_par > 0,
(F)         BRANCH crates/engine/src/lib.rs:2858:36 False  [LS4 line 2857]  !out_par.is_empty() && inf_par > 0,
(F)         BRANCH crates/engine/src/lib.rs:3035:48 False  [LSA]  .all(|w| matches!(w, [x, y] if x.mask != y.mask)),
(F)         BRANCH crates/engine/src/lib.rs:3243:17 False  [LS4 line 3222]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3243:45 False  [LS4 line 3222]  assert!(result.levels.is_empty() && result.excluded.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3265:17 False  [LS4 line 3244]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F)         BRANCH crates/engine/src/lib.rs:3265:46 False  [LS4 line 3244]  assert!(retained.halted.is_none() && retained.levels.len() > 1);
(F)         LINE   crates/engine/src/lib.rs:3303  [LS4 line 3282]  first.frequent.capacity(),
(F)         LINE   crates/engine/src/lib.rs:3304  [LS4 line 3283]  repeated.len()
(F)         BRANCH crates/engine/src/lib.rs:3320:17 False  [LS4 line 3299]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3320:35 False  [LS4 line 3299]  assert!(halt.is_none() && !previous.frequent.is_empty());
(F)         BRANCH crates/engine/src/lib.rs:3322:13 False  [LS4 line 3301]  admitted > 0 && pairs > 0,
(F)         BRANCH crates/engine/src/lib.rs:3322:29 False  [LS4 line 3301]  admitted > 0 && pairs > 0,
(F)         BRANCH crates/engine/src/lib.rs:3336:17 False  [LS4 line 3315]  assert!(next.frequent.is_empty() && next.reconciles());
(F)         BRANCH crates/engine/src/lib.rs:3336:45 False  [LS4 line 3315]  assert!(next.frequent.is_empty() && next.reconciles());
(F)         BRANCH crates/engine/src/lib.rs:3640:13 False  [LS4 line 3619]  text.contains("excluded 3 0 AlwaysFalse")
(F)         BRANCH crates/engine/src/lib.rs:3641:20 False  [LS4 line 3620]  && text.contains("excluded 9 8 AlwaysTrue")
(F)         BRANCH crates/engine/src/lib.rs:3642:20 False  [LS4 line 3621]  && text.contains("excluded 240 unmeasured NotLive"),
(F)         BRANCH crates/engine/src/lib.rs:4010:13 False  [LS4 line 3989]  roomy.completed() && tight.completed(),
(F)         BRANCH crates/engine/src/lib.rs:4010:34 False  [LS4 line 3989]  roomy.completed() && tight.completed(),
(F)         BRANCH crates/engine/src/lib.rs:5667:21 False  [LS4 line 5620]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F)         BRANCH crates/engine/src/lib.rs:5667:34 False  [LS4 line 5620]  assert!(*tail > 0 && *tail <= super::BATCH_PER_LANE);
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:112  [LS4]  &mut |_, _, _| panic!("a refused walk reports no level"),
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:113  [LS4]  &mut |_, _| panic!("a refused walk retires no level"),
(F)         LINE   crates/engine/src/refusal_cover_tests.rs:152  [LS4]  panic!("a walk of two levels took more than 256 reservations");
(F)         BRANCH crates/engine/tests/production_callers.rs:141:9 False  [LS4]  engine > 1 && others > 1,
(F)         BRANCH crates/engine/tests/production_callers.rs:141:23 False  [LS4]  engine > 1 && others > 1,
(F)         LINE   crates/engine/tests/production_callers.rs:178  [LS4]  file.path.display()
(F)         LINE   crates/engine/tests/resume_readiness.rs:78  [LS4]  bytes.get(56..192).expect("header")
(F)         BRANCH crates/engine/tests/resume_readiness.rs:98:67 False  [LS4]  matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
(F)         LINE   crates/engine/tests/resume_readiness.rs:270  [LS4]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         LINE   crates/engine/tests/resume_readiness.rs:282  [LS4]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         LINE   crates/engine/tests/resume_readiness.rs:294  [LS4]  &mut |_| panic!("a refused resume reaches no checkpoint")
(F)         BRANCH crates/engine/tests/resume_readiness.rs:600:24 False  [LS4]  .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
(F)         BRANCH crates/engine/tests/resume_readiness.rs:822:65 False  [LS4]  assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
(F)         BRANCH crates/engine/tests/resume_readiness.rs:989:13 False  [LS4]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F)         BRANCH crates/engine/tests/resume_readiness.rs:989:37 False  [LS4]  assert!(expected.completed() && need > 1, "fixture must walk pairs");
(F)         BRANCH crates/engine/tests/sweep_readiness_oracle.rs:159:48 False  [LS4]  .all(|w| matches!(w, [x, y] if x.mask.words() < y.mask.words())))
(F)         LINE   crates/greeks/src/solver.rs:1079  [LS4]  BISECTION_STEPS - 1
(F)         BRANCH crates/greeks/src/solver.rs:1133:21 False  [LS4]  low <= high && high <= low + ulp(low),
(F)         BRANCH crates/greeks/src/solver.rs:1133:36 False  [LS4]  low <= high && high <= low + ulp(low),
(F)         LINE   crates/greeks/src/solver.rs:1135  [LS4]  (high - low) / ulp(low)
(F)         BRANCH crates/greeks/src/solver.rs:1140:48 False  [LS4]  found.volatility == low || found.volatility == high,
(F)         LINE   crates/greeks/tests/vendor_anchor.rs:524  [LS4]  rate_disagreement(root)
(F)         BRANCH crates/indicators/src/anchored.rs:2247:28 False  [LS3]  if expected_ts_micros == ts(truncated, if rung == 10 { 15 * 60 + 24 } else { CLOSE_MINUTE })
(F)         BRANCH crates/indicators/src/anchored.rs:2813:17 False  [LS3]  accepted > 500 && refused > 500,
(F)         BRANCH crates/indicators/src/anchored.rs:2813:35 False  [LS3]  accepted > 500 && refused > 500,
(F)         LINE   crates/indicators/src/column.rs:1672  [LS3]  family.len()
(F)         BRANCH crates/indicators/src/column.rs:1789:25 False  [LS3]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1789:44 False  [LS3]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1792:25 False  [LS3]  assert!(!truth.get(bit) && !known.get(bit));
(F)         BRANCH crates/indicators/src/column.rs:1792:44 False  [LS3]  assert!(!truth.get(bit) && !known.get(bit));
(F)         LINE   crates/indicators/src/column.rs:2149  [LS3]  poisoned_column.bits().len()
(F)         BRANCH crates/indicators/src/column.rs:2326:13 False  [LS3]  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F)         BRANCH crates/indicators/src/column.rs:2326:62 False  [LS3]  !unchecked.acceptance_covers(execution.len()) && !unchecked.accepts(0),
(F)         BRANCH crates/indicators/src/column.rs:2609:13 False  [LS3]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F)         BRANCH crates/indicators/src/column.rs:2609:50 False  [LS3]  projected.sources().contains(&22) && projected.sources().contains(&24),
(F)         BRANCH crates/indicators/src/daily.rs:869:17 False  [LS3]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F)         BRANCH crates/indicators/src/daily.rs:869:31 False  [LS3]  assert!(!m.get(60) && !m.get(61), "it is neither above nor below");
(F)         BRANCH crates/indicators/src/daily.rs:1114:17 False  [LS3]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1114:30 False  [LS3]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1114:43 False  [LS3]  assert!(m.get(74) && m.get(76) && m.get(78), "R bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1115:17 False  [LS3]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1115:31 False  [LS3]  assert!(m.get(184) && m.get(186), "R5/S5 bands not above");
(F)         BRANCH crates/indicators/src/daily.rs:1116:17 False  [LS3]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F)         BRANCH crates/indicators/src/daily.rs:1116:30 False  [LS3]  assert!(m.get(13) && m.get(15), "pdh/pdl not above");
(F)         BRANCH crates/indicators/src/daily.rs:1137:17 False  [LS3]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1137:30 False  [LS3]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1137:43 False  [LS3]  assert!(m.get(75) && m.get(77) && m.get(79), "R bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:17 False  [LS3]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:30 False  [LS3]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1138:43 False  [LS3]  assert!(m.get(81) && m.get(83) && m.get(85), "S bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1139:17 False  [LS3]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1139:31 False  [LS3]  assert!(m.get(181) && m.get(183), "R4/S4 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1140:17 False  [LS3]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1140:31 False  [LS3]  assert!(m.get(185) && m.get(187), "R5/S5 bands not below");
(F)         BRANCH crates/indicators/src/daily.rs:1141:17 False  [LS3]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F)         BRANCH crates/indicators/src/daily.rs:1141:30 False  [LS3]  assert!(m.get(14) && m.get(16), "pdh/pdl not below");
(F)         BRANCH crates/indicators/src/daily.rs:1554:13 False  [LS3]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F)         BRANCH crates/indicators/src/daily.rs:1554:44 False  [LS3]  cuts.contains("narrow: 80") && cuts.contains("wide: 250"),
(F)         BRANCH crates/indicators/src/daily.rs:1573:13 False  [LS3]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F)         BRANCH crates/indicators/src/daily.rs:1573:47 False  [LS3]  rendered.contains("pivot: 96") && rendered.contains("half: 1"),
(F)         BRANCH crates/indicators/src/evaluator.rs:1586:78 False  [LS3]  Err(vocab::VocabError::WrongBand { expected: e, .. }) if e == expected
(F)         BRANCH crates/indicators/src/evaluator.rs:1891:17 False  [LS3]  assert!(truth_only.warmed_up() && full.warmed_up());
(F)         BRANCH crates/indicators/src/evaluator.rs:1891:43 False  [LS3]  assert!(truth_only.warmed_up() && full.warmed_up());
(F)         BRANCH crates/indicators/src/evaluator.rs:1982:17 False  [LS3]  !first.get(above) && !first.get(below),
(F)         BRANCH crates/indicators/src/evaluator.rs:1982:38 False  [LS3]  !first.get(above) && !first.get(below),
(F)         BRANCH crates/indicators/src/evaluator.rs:3564:13 False  [LS3]  !first.get(276) && !first.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3564:32 False  [LS3]  !first.get(276) && !first.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3588:13 False  [LS3]  !flat.get(276) && !flat.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3588:31 False  [LS3]  !flat.get(276) && !flat.get(277),
(F)         BRANCH crates/indicators/src/evaluator.rs:3651:36 True  [LS3]  !(mask.get(278) && mask.get(279)),
(F)         BRANCH crates/indicators/src/fib.rs:584:21 False  [LS3]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F)         BRANCH crates/indicators/src/fib.rs:584:31 False  [LS3]  assert!(p != 0 && p != 1000, "rung {p} should have no position here");
(F)         BRANCH crates/indicators/src/gap.rs:1491:13 False  [LS3]  fired > 0 && legs > 0,
(F)         BRANCH crates/indicators/src/gap.rs:1491:26 False  [LS3]  fired > 0 && legs > 0,
(F)         BRANCH crates/indicators/src/orb.rs:555:13 False  [LS3]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F)         BRANCH crates/indicators/src/orb.rs:555:40 False  [LS3]  o.extremes(0).is_some() && o.extremes(1).is_some(),
(F)         BRANCH crates/indicators/src/orb.rs:559:13 False  [LS3]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F)         BRANCH crates/indicators/src/orb.rs:559:40 False  [LS3]  o.extremes(2).is_none() && o.extremes(3).is_none(),
(F)         BRANCH crates/indicators/src/orb.rs:610:17 False  [LS3]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/orb.rs:610:40 False  [LS3]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/orb.rs:610:63 False  [LS3]  assert!(!o.window_closed(1) && !o.window_closed(2) && !o.window_closed(3));
(F)         BRANCH crates/indicators/src/pattern.rs:1558:17 False  [LS3]  assert!(shape.bullish() && !shape.bearish());
(F)         BRANCH crates/indicators/src/pattern.rs:1558:36 False  [LS3]  assert!(shape.bullish() && !shape.bearish());
(F)         BRANCH crates/indicators/src/pattern.rs:1562:17 False  [LS3]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F)         BRANCH crates/indicators/src/pattern.rs:1562:44 False  [LS3]  assert!(shape.body_at_most(250) && !shape.body_at_most(249));
(F)         BRANCH crates/indicators/src/pattern.rs:1597:13 False  [LS3]  m2.get(153) && m2.get(155),
(F)         BRANCH crates/indicators/src/pattern.rs:1597:28 False  [LS3]  m2.get(153) && m2.get(155),
(F)         BRANCH crates/indicators/src/pattern.rs:1711:41 True  [LS3]  !(mask.get(bull) && mask.get(bear)),
(F)         BRANCH crates/indicators/src/session.rs:586:17 False  [LS3]  assert!(truth.get(66) && !truth.get(67));
(F)         BRANCH crates/indicators/src/session.rs:586:34 False  [LS3]  assert!(truth.get(66) && !truth.get(67));
(F)         BRANCH crates/indicators/src/session.rs:587:17 False  [LS3]  assert!(known.get(66) && known.get(67));
(F)         BRANCH crates/indicators/src/session.rs:587:34 False  [LS3]  assert!(known.get(66) && known.get(67));
(F)         BRANCH crates/indicators/src/session.rs:589:13 False  [LS3]  !truth.get(68) && !known.get(68),
(F)         BRANCH crates/indicators/src/session.rs:589:31 False  [LS3]  !truth.get(68) && !known.get(68),
(F)         BRANCH crates/indicators/src/session.rs:655:13 False  [LS3]  !bits.get(42) && !bits.get(43),
(F)         BRANCH crates/indicators/src/session.rs:655:30 False  [LS3]  !bits.get(42) && !bits.get(43),
(F)         BRANCH crates/indicators/src/session.rs:680:13 False  [LS3]  !edge.get(44) && edge.get(45),
(F)         BRANCH crates/indicators/src/session.rs:680:30 False  [LS3]  !edge.get(44) && edge.get(45),
(F)         BRANCH crates/indicators/src/session.rs:788:17 False  [LS3]  assert!(up.get(30) && !up.get(31));
(F)         BRANCH crates/indicators/src/session.rs:788:31 False  [LS3]  assert!(up.get(30) && !up.get(31));
(F)         BRANCH crates/indicators/src/session.rs:790:17 False  [LS3]  assert!(down.get(31) && !down.get(30));
(F)         BRANCH crates/indicators/src/session.rs:790:33 False  [LS3]  assert!(down.get(31) && !down.get(30));
(F)         BRANCH crates/indicators/src/session.rs:793:13 False  [LS3]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:793:30 False  [LS3]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:838:17 False  [LS3]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:838:34 False  [LS3]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:858:17 False  [LS3]  !mask.get(30) && !mask.get(31),
(F)         BRANCH crates/indicators/src/session.rs:858:34 False  [LS3]  !mask.get(30) && !mask.get(31),
(F)         BRANCH crates/indicators/src/session.rs:922:13 False  [LS3]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:922:30 False  [LS3]  !flat.get(30) && !flat.get(31),
(F)         BRANCH crates/indicators/src/session.rs:931:17 False  [LS3]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:931:34 False  [LS3]  assert!(!mask.get(37) && !mask.get(38));
(F)         BRANCH crates/indicators/src/session.rs:950:13 False  [LS3]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:950:30 False  [LS3]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:950:47 False  [LS3]  !mask.get(37) && !mask.get(38) && !mask.get(39),
(F)         BRANCH crates/indicators/src/session.rs:966:17 False  [LS3]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F)         BRANCH crates/indicators/src/session.rs:966:33 False  [LS3]  assert!(mask.get(48) && !mask.get(49), "a gap up was not reported");
(F)         BRANCH crates/indicators/src/session.rs:968:17 False  [LS3]  assert!(mask.get(66) && !mask.get(67));
(F)         BRANCH crates/indicators/src/session.rs:968:33 False  [LS3]  assert!(mask.get(66) && !mask.get(67));
(F)         BRANCH crates/indicators/src/session.rs:972:17 False  [LS3]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F)         BRANCH crates/indicators/src/session.rs:972:34 False  [LS3]  assert!(!mask.get(48) && !mask.get(49), "a flat open claimed a gap");
(F)         BRANCH crates/indicators/src/session.rs:974:13 False  [LS3]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:974:30 False  [LS3]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:974:47 False  [LS3]  !mask.get(66) && !mask.get(67) && !mask.get(68),
(F)         BRANCH crates/indicators/src/session.rs:989:17 False  [LS3]  assert!(mask.get(50) && !mask.get(51));
(F)         BRANCH crates/indicators/src/session.rs:989:33 False  [LS3]  assert!(mask.get(50) && !mask.get(51));
(F)         BRANCH crates/indicators/src/session.rs:993:17 False  [LS3]  assert!(mask.get(51) && !mask.get(50));
(F)         BRANCH crates/indicators/src/session.rs:993:33 False  [LS3]  assert!(mask.get(51) && !mask.get(50));
(F)         BRANCH crates/indicators/src/session.rs:1283:13 False  [LS3]  !mask.get(66) && !mask.get(67),
(F)         BRANCH crates/indicators/src/session.rs:1283:30 False  [LS3]  !mask.get(66) && !mask.get(67),
(F)         BRANCH crates/indicators/src/trend.rs:1292:13 False  [LS3]  !mask.get(0) && !mask.get(1),
(F)         BRANCH crates/indicators/src/trend.rs:1292:29 False  [LS3]  !mask.get(0) && !mask.get(1),
(F)         BRANCH crates/indicators/src/trend.rs:1296:13 False  [LS3]  !mask.get(2) && !mask.get(3),
(F)         BRANCH crates/indicators/src/trend.rs:1296:29 False  [LS3]  !mask.get(2) && !mask.get(3),
(F)         BRANCH crates/indicators/src/trend.rs:1300:13 False  [LS3]  !mask.get(4) && !mask.get(5),
(F)         BRANCH crates/indicators/src/trend.rs:1300:29 False  [LS3]  !mask.get(4) && !mask.get(5),
(F)         BRANCH crates/indicators/src/trend.rs:2302:74 False  [LS3]  matches!((previous, now), (Some(before), Some(after)) if after < before),
(U)         BRANCH crates/indicators/src/vwap.rs:212:11 False  [LS3]  while i < NEWTON_STEPS {
(F)         BRANCH crates/indicators/src/vwap.rs:791:17 False  [LS3]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:791:34 False  [LS3]  assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:792:17 False  [LS3]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:792:34 False  [LS3]  assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
(F)         BRANCH crates/indicators/src/vwap.rs:904:17 False  [LS3]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F)         BRANCH crates/indicators/src/vwap.rs:904:44 False  [LS3]  assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
(F)         BRANCH crates/indicators/src/vwap.rs:1250:21 False  [LS3]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F)         BRANCH crates/indicators/src/vwap.rs:1250:52 False  [LS3]  !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
(F)         BRANCH crates/indicators/tests/crossing_known_readiness.rs:84:32 True  [LS3]  !(above && below),
(F)         BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:13 False  [LS3]  false_count > 0 && true_count > 0,
(F)         BRANCH crates/indicators/tests/current_day_fib_known_readiness.rs:220:32 False  [LS3]  false_count > 0 && true_count > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:13 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:26 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:313:40 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:13 False  [LS3]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:338:32 False  [LS3]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:17 False  [LS3]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/exact_minute_orb_gap_readiness.rs:351:36 False  [LS3]  !truth.get(bit) && !known.get(bit),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:176:13 False  [LS3]  !truth.get(retired) && !known.get(retired),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:176:36 False  [LS3]  !truth.get(retired) && !known.get(retired),
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:231:13 False  [LS3]  trues > 0 && falses > 0,
(F)         BRANCH crates/indicators/tests/fib_known_readiness.rs:231:26 False  [LS3]  trues > 0 && falses > 0,
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:224:21 False  [LS3]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:224:40 False  [LS3]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:264:17 False  [LS3]  assert!(false_count > 0 && true_count > 0);
(F)         BRANCH crates/indicators/tests/gap_known_readiness.rs:264:36 False  [LS3]  assert!(false_count > 0 && true_count > 0);
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:271  [LS3]  positions.len()
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:321  [LS3]  positions.len()
(F)         LINE   crates/indicators/tests/module_doc_counts.rs:332  [LS3]  emitted.len()
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:13 False  [LS3]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:32 False  [LS3]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/orb_known_readiness.rs:195:50 False  [LS3]  false_count > 0 && true_count > 0 && unknown_count > 0,
(F)         BRANCH crates/indicators/tests/session_from_outside.rs:200:9 False  [LS3]  mask.get(46) && !mask.get(47),
(F)         BRANCH crates/indicators/tests/session_from_outside.rs:200:25 False  [LS3]  mask.get(46) && !mask.get(47),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:367:13 False  [LS3]  !truth.get(position) && !known.get(position),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:367:37 False  [LS3]  !truth.get(position) && !known.get(position),
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:371:13 False  [LS3]  assert!(known.get(48) && known.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:371:30 False  [LS3]  assert!(known.get(48) && known.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:372:13 False  [LS3]  assert!(!truth.get(48) && !truth.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:372:31 False  [LS3]  assert!(!truth.get(48) && !truth.get(49));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:459:13 False  [LS3]  assert!(known.get(33) && !truth.get(33));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:459:30 False  [LS3]  assert!(known.get(33) && !truth.get(33));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:460:13 False  [LS3]  assert!(!known.get(37) && !known.get(66));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:460:31 False  [LS3]  assert!(!known.get(37) && !known.get(66));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:13 False  [LS3]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:30 False  [LS3]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/session_known_readiness.rs:511:47 False  [LS3]  assert!(truth.get(48) && truth.get(50) && truth.get(68));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:9 False  [LS3]  hammers > 0 && hammers < cases,
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:81:24 False  [LS3]  hammers > 0 && hammers < cases,
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:13 False  [LS3]  assert!(bullish > 0 && bearish > 0);
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:133:28 False  [LS3]  assert!(bullish > 0 && bearish > 0);
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:17 False  [LS3]  assert!(!truth.get(30) && !truth.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:159:35 False  [LS3]  assert!(!truth.get(30) && !truth.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:17 False  [LS3]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:160:34 False  [LS3]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:17 False  [LS3]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:34 False  [LS3]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:51 False  [LS3]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:192:68 False  [LS3]  assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:9 False  [LS3]  !next_day.get(157) && !next_day.get(176),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:215:31 False  [LS3]  !next_day.get(157) && !next_day.get(176),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:9 False  [LS3]  !next_day.get(276) && !next_day.get(277),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:219:31 False  [LS3]  !next_day.get(276) && !next_day.get(277),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:17 False  [LS3]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:352:47 False  [LS3]  assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:21 False  [LS3]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:389:38 False  [LS3]  assert!(known.get(30) && known.get(31));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:17 False  [LS3]  assert!(!truth.get(13) && !truth.get(16));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:426:35 False  [LS3]  assert!(!truth.get(13) && !truth.get(16));
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:13 False  [LS3]  known.get(13) && known.get(16),
(F)         BRANCH crates/indicators/tests/sweep_predicate_readiness.rs:428:30 False  [LS3]  known.get(13) && known.get(16),
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:183:45 False  [LS3]  assert!(!truth.get(position) || known.get(position));
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:13 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:26 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         BRANCH crates/indicators/tests/trend_known_readiness.rs:257:40 False  [LS3]  trues > 0 && falses > 0 && unknowns > 0,
(F)         LINE   crates/lake/src/bar.rs:237  [LS3]  other => panic!("expected NotRepresentable, got {other:?}"),
(F)         LINE   crates/lake/src/bar.rs:247  [LS3]  other => panic!("expected NotRepresentable, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:412  [LS3]  ContractKind::Future => panic!("a CE is not a future"),
(F)         LINE   crates/lake/src/contract.rs:563  [LS3]  other => panic!("expected TooLong, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:571  [LS3]  name.len()
(F)         LINE   crates/lake/src/contract.rs:614  [LS3]  ContractKind::Future => panic!("not a future"),
(F)         LINE   crates/lake/src/contract.rs:687  [LS3]  other => panic!("{padded} must be refused, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:721  [LS3]  other => panic!("{signed} must be refused, got {other:?}"),
(F)         LINE   crates/lake/src/contract.rs:753  [LS3]  other => panic!("{name} must be refused, got {other:?}"),
(F)         BRANCH crates/lake/src/contract.rs:786:61 False  [LS3]  Err(ContractError::BadExpiryShape { found }) if found == "0\u{e9}Apr2"
(F)         BRANCH crates/lake/src/footer_tests.rs:32:12 True  [LS3]  if let Err(e) = r {
(F)         LINE   crates/lake/src/footer_tests.rs:33  [LS3]  panic!("expected the walk to admit, got {e:?}");
(F)         LINE   crates/lake/src/footer_tests.rs:40  [LS3]  other => panic!("expected FooterUnreadable, got {other:?}"),
(F)         LINE   crates/lake/src/footer_tests.rs:69  [LS3]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F)         LINE   crates/lake/src/footer_tests.rs:80  [LS3]  other => panic!("expected FooterUnreadable, got {:?}", other.err()),
(F)         BRANCH crates/lake/src/page.rs:631:13 False  [LS3]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:631:54 False  [LS3]  short.to_string().contains("4 bytes") && short.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:640:13 False  [LS3]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F)         BRANCH crates/lake/src/page.rs:640:53 False  [LS3]  long.to_string().contains("4 bytes") && long.to_string().contains('3'),
(F)         BRANCH crates/lake/src/page.rs:681:13 False  [LS3]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:681:65 False  [LS3]  wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
(F)         BRANCH crates/lake/src/page.rs:741:13 False  [LS3]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F)         BRANCH crates/lake/src/page.rs:741:51 False  [LS3]  text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
(F)         LINE   crates/lake/src/page.rs:878  [LS3]  other => panic!("expected a data page, got {other:?}"),
(F)         LINE   crates/lake/src/page.rs:986  [LS3]  frame.len(),
(F)         LINE   crates/lake/src/page.rs:987  [LS3]  plain.len()
(F)         LINE   crates/lake/src/page.rs:1016  [LS3]  other => panic!("expected a data page, got {other:?}"),
(F)         LINE   crates/lake/src/page.rs:1147  [LS3]  frame.len()
(F)         BRANCH crates/lake/src/page.rs:1192:13 False  [LS3]  text.contains("zstd frame refused") && text.contains("134217729"),
(F)         BRANCH crates/lake/src/page.rs:1192:52 False  [LS3]  text.contains("zstd frame refused") && text.contains("134217729"),
(F)         LINE   crates/lake/src/reader.rs:900  [LS3]  other => panic!("expected TooLarge, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:912  [LS3]  matches!(
(F)         LINE   crates/lake/src/reader.rs:919  [LS3]  matches!(
(F)         LINE   crates/lake/src/reader.rs:953  [LS3]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/src/reader.rs:966  [LS3]  other => panic!("expected NotParquet, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/src/reader.rs:979  [LS3]  other => panic!("len {len}: expected Truncated, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:1053  [LS3]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1099  [LS3]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1116  [LS3]  other => panic!("expected ShortColumnChunk, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1146  [LS3]  other => panic!("expected UnexpectedNull, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1281  [LS3]  _ => panic!("the cash layout is INT64 and DOUBLE only"),
(F)         LINE   crates/lake/src/reader.rs:1388  [LS3]  other => panic!("expected ImpossibleLength, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:1422  [LS3]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1423  [LS3]  Ok(batch) => panic!(
(F)         LINE   crates/lake/src/reader.rs:1425  [LS3]  batch.iter().map(|b| b.open_interest()).collect::<Vec<_>>()
(F)         LINE   crates/lake/src/reader.rs:1479  [LS3]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1480  [LS3]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1530  [LS3]  other => panic!("a num_rows lie alone is the probe's refusal, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1554  [LS3]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1555  [LS3]  Ok(batch) => panic!("an 8-row chunk was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1626  [LS3]  Err(other) => panic!("refused for the wrong reason: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1627  [LS3]  Ok(batch) => panic!("an 8-row file was read as {} rows", batch.len()),
(F)         LINE   crates/lake/src/reader.rs:1727  [LS3]  other => panic!("the padded group was not refused as short: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1756  [LS3]  match cut {
(F)         LINE   crates/lake/src/reader.rs:1758  [LS3]  panic!("the cut and its copy sum to the file count")
(F)         LINE   crates/lake/src/reader.rs:1801  [LS3]  other => panic!("group {group} under {claim}: {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:1896  [LS3]  crate::error::ColumnType::Other => panic!("no lake column is Other"),
(F)         LINE   crates/lake/src/reader.rs:1943  [LS3]  _ => panic!("the lake layouts are INT32, INT64 and DOUBLE only"),
(F)         LINE   crates/lake/src/reader.rs:1983  [LS3]  other => panic!("expected ColumnTypeMismatch, got {other:?}"),
(F)         LINE   crates/lake/src/reader.rs:2061  [LS3]  other => panic!("{lie}: expected ImpossibleLength, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:2117  [LS3]  panic!("the patched length did not settle");
(F)         LINE   crates/lake/src/reader.rs:2139  [LS3]  other => panic!("{past} past: expected PageDecode, got {:?}", other.err()),
(F)         LINE   crates/lake/src/reader.rs:2150  [LS3]  other => panic!("a chunk ending at the last byte: {:?}", other.err()),
(F)         BRANCH crates/lake/src/reader.rs:2166:20 False  [LS3]  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F)         BRANCH crates/lake/src/reader.rs:2166:45 False  [LS3]  if column == "timestamp" && reason.ends_with("runs past the end of the file")
(F)         LINE   crates/lake/src/reader_cover_tests.rs:23  [LS3]  other => panic!("a missing file must be an Io refusal: {other:?}"),
(F)         LINE   crates/lake/src/reader_cover_tests.rs:36  [LS3]  other => panic!("/dev/zero under cap {cap}: {other:?}"),
(U)         BRANCH crates/lake/src/schema.rs:125:12 True  [LS3]  if i >= n {
(U)         LINE   crates/lake/src/schema.rs:126  [LS3]  return Err(LakeError::MissingColumn { name: s.name });
(F)         LINE   crates/lake/src/schema_cover_tests.rs:58  [LS3]  ColumnType::Int32 | ColumnType::Other => panic!("the cash layout has neither"),
(F)         LINE   crates/lake/src/schema_cover_tests.rs:97  [LS3]  other => panic!("{shown}: expected UnsupportedTimestamp, got {other:?}"),
(F)         LINE   crates/lake/src/zstd_v082_frames.rs:175  [LS3]  other => panic!("no synthetic input called {other}"),
(F)         LINE   crates/lake/tests/events.rs:159  [LS3]  _ => panic!("these fixtures hold only INT64 and DOUBLE columns"),
(F)         LINE   crates/lake/tests/events.rs:366  [LS3]  panic!(
(F)         LINE   crates/lake/tests/events.rs:401  [LS3]  panic!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:57  [LS3: fn lake_path(rel: &str) -> Option<PathBuf> {]  fn lake_path(rel: &str) -> Option<PathBuf> {
IGNORED     LINE   crates/lake/tests/real_lake.rs:58  [LS3: let home = std::env::var_os("HOME")?;]  let home = std::env::var_os("HOME")?;
IGNORED     LINE   crates/lake/tests/real_lake.rs:59  [LS3: let p = Path::new(&home).join(rel);]  let p = Path::new(&home).join(rel);
IGNORED     LINE   crates/lake/tests/real_lake.rs:60  [LS3: p.exists().then_some(p)]  p.exists().then_some(p)
IGNORED     LINE   crates/lake/tests/real_lake.rs:61  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:64  [LS3: fn skipped(what: &str) {]  fn skipped(what: &str) {
IGNORED     LINE   crates/lake/tests/real_lake.rs:65  [LS3: println!("SKIPPING {what}: ~/.brutex/lake is not on this machine.");]  println!("SKIPPING {what}: ~/.brutex/lake is not on this machine.");
IGNORED     LINE   crates/lake/tests/real_lake.rs:66  [LS3: println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");]  println!("  The fixture cannot be tracked — CI gate 1 forbids a .parquet in");
IGNORED     LINE   crates/lake/tests/real_lake.rs:67  [LS3: println!("  this repository — so this test proves nothing here. The decode");]  println!("  this repository — so this test proves nothing here. The decode");
IGNORED     LINE   crates/lake/tests/real_lake.rs:68  [LS3: println!("  path is covered by tests/synthetic.rs, which writes its own.");]  println!("  path is covered by tests/synthetic.rs, which writes its own.");
IGNORED     LINE   crates/lake/tests/real_lake.rs:69  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:73  [LS3: fn the_real_fno_sample_decodes_to_the_values_it_holds() {]  fn the_real_fno_sample_decodes_to_the_values_it_holds() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:74  [LS3: let Some(path) = lake_path(SAMPLE_FNO) else {]  let Some(path) = lake_path(SAMPLE_FNO) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:74:9 False  [LS3: let Some(path) = lake_path(SAMPLE_FNO) else {]  let Some(path) = lake_path(SAMPLE_FNO) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:74:9 True  [LS3: let Some(path) = lake_path(SAMPLE_FNO) else {]  let Some(path) = lake_path(SAMPLE_FNO) else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:75  [LS3: panic!(]  panic!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:83  [LS3: let f = LakeFile::open(&path).expect("the real sample file must open");]  let f = LakeFile::open(&path).expect("the real sample file must open");
IGNORED     LINE   crates/lake/tests/real_lake.rs:84  [LS3: assert_eq!(f.layout(), Layout::Fno, "17 columns with greeks");]  assert_eq!(f.layout(), Layout::Fno, "17 columns with greeks");
IGNORED     LINE   crates/lake/tests/real_lake.rs:85  [LS3: assert_eq!(f.created_by(), Some("Polars"));]  assert_eq!(f.created_by(), Some("Polars"));
IGNORED     LINE   crates/lake/tests/real_lake.rs:86  [LS3: assert_eq!(f.num_rows(), 2_480);]  assert_eq!(f.num_rows(), 2_480);
IGNORED     LINE   crates/lake/tests/real_lake.rs:87  [LS3: assert_eq!(f.row_groups(), 1);]  assert_eq!(f.row_groups(), 1);
IGNORED     LINE   crates/lake/tests/real_lake.rs:89  [LS3: let b = f.read_row_group(0).expect("the ZSTD pages must decode");]  let b = f.read_row_group(0).expect("the ZSTD pages must decode");
IGNORED     LINE   crates/lake/tests/real_lake.rs:90  [LS3: assert_eq!(b.len(), 2_480);]  assert_eq!(b.len(), 2_480);
IGNORED     LINE   crates/lake/tests/real_lake.rs:94  [LS3: let r = b.row(0).expect("row 0");]  let r = b.row(0).expect("row 0");
IGNORED     LINE   crates/lake/tests/real_lake.rs:95  [LS3: assert_eq!(r.timestamp_micros, 1_584_685_680_000_000);]  assert_eq!(r.timestamp_micros, 1_584_685_680_000_000);
IGNORED     LINE   crates/lake/tests/real_lake.rs:98  [LS3: assert_eq!(r.open.raw(), 4_950);]  assert_eq!(r.open.raw(), 4_950);
IGNORED     LINE   crates/lake/tests/real_lake.rs:99  [LS3: assert_eq!(r.high.raw(), 4_950);]  assert_eq!(r.high.raw(), 4_950);
IGNORED     LINE   crates/lake/tests/real_lake.rs:100  [LS3: assert_eq!(r.low.raw(), 4_950);]  assert_eq!(r.low.raw(), 4_950);
IGNORED     LINE   crates/lake/tests/real_lake.rs:101  [LS3: assert_eq!(r.close.raw(), 4_950);]  assert_eq!(r.close.raw(), 4_950);
IGNORED     LINE   crates/lake/tests/real_lake.rs:102  [LS3: assert_eq!(r.volume, 76);]  assert_eq!(r.volume, 76);
IGNORED     LINE   crates/lake/tests/real_lake.rs:103  [LS3: assert_eq!(r.open_interest(), Some(75));]  assert_eq!(r.open_interest(), Some(75));
IGNORED     LINE   crates/lake/tests/real_lake.rs:106  [LS3: assert_eq!(r.spot_at_bar.expect("spot").raw(), 847_310);]  assert_eq!(r.spot_at_bar.expect("spot").raw(), 847_310);
IGNORED     LINE   crates/lake/tests/real_lake.rs:107  [LS3: assert_eq!(r.greeks_provenance_id, Some(1_380_610));]  assert_eq!(r.greeks_provenance_id, Some(1_380_610));
IGNORED     LINE   crates/lake/tests/real_lake.rs:110  [LS3: let g = r.greeks.expect("row 0 has greeks");]  let g = r.greeks.expect("row 0 has greeks");
IGNORED     LINE   crates/lake/tests/real_lake.rs:111  [LS3: assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());]  assert_eq!(g.iv.to_bits(), 0.678_618_464_061_773_3_f64.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:112  [LS3: assert_eq!(g.delta.to_bits(), 0.103_791_597_602_742_98_f64.to_bits());]  assert_eq!(g.delta.to_bits(), 0.103_791_597_602_742_98_f64.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:113  [LS3: assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:114  [LS3: g.gamma.to_bits(),]  g.gamma.to_bits(),
IGNORED     LINE   crates/lake/tests/real_lake.rs:115  [LS3: 0.000_171_426_804_295_494_02_f64.to_bits()]  0.000_171_426_804_295_494_02_f64.to_bits()
IGNORED     LINE   crates/lake/tests/real_lake.rs:117  [LS3: assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());]  assert_eq!(g.theta.to_bits(), (-7.788_961_946_081_791_f64).to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:118  [LS3: assert_eq!(g.vega.to_bits(), 2.791_593_074_801_929_3_f64.to_bits());]  assert_eq!(g.vega.to_bits(), 2.791_593_074_801_929_3_f64.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:119  [LS3: assert_eq!(g.rho.to_bits(), 0.276_837_206_382_207_27_f64.to_bits());]  assert_eq!(g.rho.to_bits(), 0.276_837_206_382_207_27_f64.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:120  [LS3: assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:121  [LS3: g.t_years_used.to_bits(),]  g.t_years_used.to_bits(),
IGNORED     LINE   crates/lake/tests/real_lake.rs:122  [LS3: 0.033_280_060_882_800_61_f64.to_bits()]  0.033_280_060_882_800_61_f64.to_bits()
IGNORED     LINE   crates/lake/tests/real_lake.rs:124  [LS3: assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());]  assert_eq!(g.rate_used.to_bits(), 0.065_f64.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake.rs:127  [LS3: assert_eq!(b.iter().count(), 2_480);]  assert_eq!(b.iter().count(), 2_480);
IGNORED     LINE   crates/lake/tests/real_lake.rs:128  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:132  [LS3: fn the_real_cash_sample_has_the_seven_column_layout() {]  fn the_real_cash_sample_has_the_seven_column_layout() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:133  [LS3: let Some(path) = lake_path(SAMPLE_CASH) else {]  let Some(path) = lake_path(SAMPLE_CASH) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:133:9 False  [LS3: let Some(path) = lake_path(SAMPLE_CASH) else {]  let Some(path) = lake_path(SAMPLE_CASH) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:133:9 True  [LS3: let Some(path) = lake_path(SAMPLE_CASH) else {]  let Some(path) = lake_path(SAMPLE_CASH) else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:134  [LS3: panic!(]  panic!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:142  [LS3: let f = LakeFile::open(&path).expect("open");]  let f = LakeFile::open(&path).expect("open");
IGNORED     LINE   crates/lake/tests/real_lake.rs:143  [LS3: assert_eq!(f.layout(), Layout::Cash, "no greeks in a cash file");]  assert_eq!(f.layout(), Layout::Cash, "no greeks in a cash file");
IGNORED     LINE   crates/lake/tests/real_lake.rs:144  [LS3: assert_eq!(f.num_rows(), 2_143);]  assert_eq!(f.num_rows(), 2_143);
IGNORED     LINE   crates/lake/tests/real_lake.rs:146  [LS3: let b = f.read_row_group(0).expect("decode");]  let b = f.read_row_group(0).expect("decode");
IGNORED     LINE   crates/lake/tests/real_lake.rs:147  [LS3: let r = b.row(0).expect("row 0");]  let r = b.row(0).expect("row 0");
IGNORED     LINE   crates/lake/tests/real_lake.rs:148  [LS3: assert_eq!(r.timestamp_micros, 1_674_445_500_000_000);]  assert_eq!(r.timestamp_micros, 1_674_445_500_000_000);
IGNORED     LINE   crates/lake/tests/real_lake.rs:149  [LS3: assert_eq!(r.open.raw(), 195_000); // 1950.00]  assert_eq!(r.open.raw(), 195_000); // 1950.00
IGNORED     LINE   crates/lake/tests/real_lake.rs:150  [LS3: assert_eq!(r.high.raw(), 195_000);]  assert_eq!(r.high.raw(), 195_000);
IGNORED     LINE   crates/lake/tests/real_lake.rs:151  [LS3: assert_eq!(r.low.raw(), 193_200); // 1932.00]  assert_eq!(r.low.raw(), 193_200); // 1932.00
IGNORED     LINE   crates/lake/tests/real_lake.rs:152  [LS3: assert_eq!(r.close.raw(), 194_300); // 1943.00]  assert_eq!(r.close.raw(), 194_300); // 1943.00
IGNORED     LINE   crates/lake/tests/real_lake.rs:153  [LS3: assert_eq!(r.volume, 2_663);]  assert_eq!(r.volume, 2_663);
IGNORED     LINE   crates/lake/tests/real_lake.rs:156  [LS3: assert_eq!(r.open_interest(), None);]  assert_eq!(r.open_interest(), None);
IGNORED     LINE   crates/lake/tests/real_lake.rs:157  [LS3: assert_eq!(r.open_interest, i64::MIN);]  assert_eq!(r.open_interest, i64::MIN);
IGNORED     LINE   crates/lake/tests/real_lake.rs:160  [LS3: assert!(!r.has_greeks());]  assert!(!r.has_greeks());
IGNORED     LINE   crates/lake/tests/real_lake.rs:161  [LS3: assert_eq!(r.spot_at_bar, None);]  assert_eq!(r.spot_at_bar, None);
IGNORED     LINE   crates/lake/tests/real_lake.rs:162  [LS3: assert_eq!(r.greeks_provenance_id, None);]  assert_eq!(r.greeks_provenance_id, None);
IGNORED     LINE   crates/lake/tests/real_lake.rs:163  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:167  [LS3: fn a_real_contract_directory_name_parses_and_round_trips() {]  fn a_real_contract_directory_name_parses_and_round_trips() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:168  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:168:9 False  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:168:9 True  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:169  [LS3: skipped("the contract directory walk");]  skipped("the contract directory walk");
IGNORED     LINE   crates/lake/tests/real_lake.rs:170  [LS3: return;]  return;
IGNORED     LINE   crates/lake/tests/real_lake.rs:173  [LS3: let mut seen = 0_usize;]  let mut seen = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:174  [LS3: let mut futures = 0_usize;]  let mut futures = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:175  [LS3: let entries = std::fs::read_dir(&root).expect("read the F&O directory");]  let entries = std::fs::read_dir(&root).expect("read the F&O directory");
IGNORED     LINE   crates/lake/tests/real_lake.rs:176  [LS3: for e in entries.take(500) {]  for e in entries.take(500) {
IGNORED     LINE   crates/lake/tests/real_lake.rs:177  [LS3: let e = e.expect("entry");]  let e = e.expect("entry");
IGNORED     LINE   crates/lake/tests/real_lake.rs:178  [LS3: let name = e.file_name();]  let name = e.file_name();
IGNORED     LINE   crates/lake/tests/real_lake.rs:179  [LS3: let name = name.to_str().expect("a lake name is utf-8");]  let name = name.to_str().expect("a lake name is utf-8");
IGNORED     LINE   crates/lake/tests/real_lake.rs:181  [LS3: let parsed = ContractName::parse(name)]  let parsed = ContractName::parse(name)
IGNORED     LINE   crates/lake/tests/real_lake.rs:182  [LS3: .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));]  .unwrap_or_else(|err| panic!("real lake directory {name} was refused: {err}"));
IGNORED     LINE   crates/lake/tests/real_lake.rs:183  [LS3: assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/lake/tests/real_lake.rs:184  [LS3: parsed.to_string(),]  parsed.to_string(),
IGNORED     LINE   crates/lake/tests/real_lake.rs:188  [LS3: if parsed.kind() == ContractKind::Future {]  if parsed.kind() == ContractKind::Future {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:188:12 False  [LS3: if parsed.kind() == ContractKind::Future {]  if parsed.kind() == ContractKind::Future {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:188:12 True  [LS3: if parsed.kind() == ContractKind::Future {]  if parsed.kind() == ContractKind::Future {
IGNORED     LINE   crates/lake/tests/real_lake.rs:189  [LS3: futures += 1;]  futures += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:190  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:191  [LS3: seen += 1;]  seen += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:193  [LS3: assert!(seen > 0, "the F&O directory must not be empty");]  assert!(seen > 0, "the F&O directory must not be empty");
IGNORED     LINE   crates/lake/tests/real_lake.rs:194  [LS3: println!("parsed {seen} real contract names ({futures} futures), all round-tripped");]  println!("parsed {seen} real contract names ({futures} futures), all round-tripped");
IGNORED     LINE   crates/lake/tests/real_lake.rs:195  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:199  [LS3: fn a_spread_of_real_files_decodes_with_no_failures() {]  fn a_spread_of_real_files_decodes_with_no_failures() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:200  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:200:9 False  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:200:9 True  [LS3: let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {]  let Some(root) = lake_path(".brutex/lake/bars/NSE/FNO") else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:201  [LS3: skipped("the multi-file decode");]  skipped("the multi-file decode");
IGNORED     LINE   crates/lake/tests/real_lake.rs:202  [LS3: return;]  return;
IGNORED     LINE   crates/lake/tests/real_lake.rs:205  [LS3: let mut files = 0_usize;]  let mut files = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:206  [LS3: let mut rows = 0_usize;]  let mut rows = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:207  [LS3: let mut null_oi = 0_usize;]  let mut null_oi = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:208  [LS3: let mut real_oi = 0_usize;]  let mut real_oi = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:209  [LS3: let mut with_greeks = 0_usize;]  let mut with_greeks = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake.rs:211  [LS3: let dirs = std::fs::read_dir(&root).expect("read dir");]  let dirs = std::fs::read_dir(&root).expect("read dir");
IGNORED     LINE   crates/lake/tests/real_lake.rs:212  [LS3: for d in dirs.take(40) {]  for d in dirs.take(40) {
IGNORED     LINE   crates/lake/tests/real_lake.rs:213  [LS3: let d = d.expect("entry").path();]  let d = d.expect("entry").path();
IGNORED     LINE   crates/lake/tests/real_lake.rs:214  [LS3: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:214:13 False  [LS3: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:214:13 True  [LS3: let Ok(years) = std::fs::read_dir(d.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(d.join("1minute")) else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:215  [LS3: continue;]  continue;
IGNORED     LINE   crates/lake/tests/real_lake.rs:217  [LS3: for y in years {]  for y in years {
IGNORED     LINE   crates/lake/tests/real_lake.rs:218  [LS3: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {]  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:218:17 False  [LS3: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {]  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:218:17 True  [LS3: let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {]  let Ok(months) = std::fs::read_dir(y.expect("year").path()) else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:219  [LS3: continue;]  continue;
IGNORED     LINE   crates/lake/tests/real_lake.rs:221  [LS3: for m in months {]  for m in months {
IGNORED     LINE   crates/lake/tests/real_lake.rs:222  [LS3: let p = m.expect("month").path();]  let p = m.expect("month").path();
IGNORED     LINE   crates/lake/tests/real_lake.rs:223  [LS3: let f = LakeFile::open(&p)]  let f = LakeFile::open(&p)
IGNORED     LINE   crates/lake/tests/real_lake.rs:224  [LS3: .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));]  .unwrap_or_else(|e| panic!("real lake file {} refused: {e}", p.display()));
IGNORED     LINE   crates/lake/tests/real_lake.rs:225  [LS3: for g in 0..f.row_groups() {]  for g in 0..f.row_groups() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:226  [LS3: let b = f]  let b = f
IGNORED     LINE   crates/lake/tests/real_lake.rs:227  [LS3: .read_row_group(g)]  .read_row_group(g)
IGNORED     LINE   crates/lake/tests/real_lake.rs:228  [LS3: .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));]  .unwrap_or_else(|e| panic!("{} group {g} refused: {e}", p.display()));
IGNORED     LINE   crates/lake/tests/real_lake.rs:229  [LS3: for bar in b.iter() {]  for bar in b.iter() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:230  [LS3: rows += 1;]  rows += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:231  [LS3: if bar.open_interest().is_none() {]  if bar.open_interest().is_none() {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:231:28 False  [LS3: if bar.open_interest().is_none() {]  if bar.open_interest().is_none() {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:231:28 True  [LS3: if bar.open_interest().is_none() {]  if bar.open_interest().is_none() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:232  [LS3: null_oi += 1;]  null_oi += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:233  [LS3: } else {]  } else {
IGNORED     LINE   crates/lake/tests/real_lake.rs:234  [LS3: real_oi += 1;]  real_oi += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:235  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:236  [LS3: if bar.has_greeks() {]  if bar.has_greeks() {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:236:28 False  [LS3: if bar.has_greeks() {]  if bar.has_greeks() {
IGNORED     BRANCH crates/lake/tests/real_lake.rs:236:28 True  [LS3: if bar.has_greeks() {]  if bar.has_greeks() {
IGNORED     LINE   crates/lake/tests/real_lake.rs:237  [LS3: with_greeks += 1;]  with_greeks += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:238  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake.rs:241  [LS3: files += 1;]  files += 1;
IGNORED     LINE   crates/lake/tests/real_lake.rs:246  [LS3: assert!(files > 0, "at least one real file must have been read");]  assert!(files > 0, "at least one real file must have been read");
IGNORED     LINE   crates/lake/tests/real_lake.rs:247  [LS3: println!("decoded {files} real files, {rows} rows, 0 failures");]  println!("decoded {files} real files, {rows} rows, 0 failures");
IGNORED     LINE   crates/lake/tests/real_lake.rs:248  [LS3: println!("  open interest: {real_oi} real, {null_oi} null");]  println!("  open interest: {real_oi} real, {null_oi} null");
IGNORED     LINE   crates/lake/tests/real_lake.rs:249  [LS3: println!("  rows carrying greeks: {with_greeks}");]  println!("  rows carrying greeks: {with_greeks}");
IGNORED     LINE   crates/lake/tests/real_lake.rs:253  [LS3: assert!(rows > 0);]  assert!(rows > 0);
IGNORED     LINE   crates/lake/tests/real_lake.rs:254  [LS3: assert!(null_oi > 0, "the sample must contain a null open interest");]  assert!(null_oi > 0, "the sample must contain a null open interest");
IGNORED     LINE   crates/lake/tests/real_lake.rs:255  [LS3: assert!(real_oi > 0, "and a real one");]  assert!(real_oi > 0, "and a real one");
IGNORED     LINE   crates/lake/tests/real_lake.rs:256  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:57  [LS3: fn eat(mut h: u64, v: u64) -> u64 {]  fn eat(mut h: u64, v: u64) -> u64 {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:58  [LS3: for b in v.to_le_bytes() {]  for b in v.to_le_bytes() {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:59  [LS3: h ^= u64::from(b);]  h ^= u64::from(b);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:60  [LS3: h = h.wrapping_mul(FNV_PRIME);]  h = h.wrapping_mul(FNV_PRIME);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:61  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:62  [LS3: h]  h
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:63  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:66  [LS3: fn eat_opt(h: u64, v: Option<u64>) -> u64 {]  fn eat_opt(h: u64, v: Option<u64>) -> u64 {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:67  [LS3: match v {]  match v {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:68  [LS3: Some(v) => eat(eat(h, 1), v),]  Some(v) => eat(eat(h, 1), v),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:69  [LS3: None => eat(h, 0),]  None => eat(h, 0),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:71  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:78  [LS3: fn digest_file(path: &Path) -> Result<(u64, usize, Layout), String> {]  fn digest_file(path: &Path) -> Result<(u64, usize, Layout), String> {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:79  [LS3: let file = LakeFile::open(path).map_err(|e| e.to_string())?;]  let file = LakeFile::open(path).map_err(|e| e.to_string())?;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:80  [LS3: let layout = file.layout();]  let layout = file.layout();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:81  [LS3: let mut h = eat(FNV_OFFSET, u64::from(u8::from(layout.has_greeks())));]  let mut h = eat(FNV_OFFSET, u64::from(u8::from(layout.has_greeks())));
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:82  [LS3: let mut rows = 0_usize;]  let mut rows = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:84  [LS3: for g in 0..file.row_groups() {]  for g in 0..file.row_groups() {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:85  [LS3: let batch = file]  let batch = file
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:86  [LS3: .read_row_group(g)]  .read_row_group(g)
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:87  [LS3: .map_err(|e| format!("group {g}: {e}"))?;]  .map_err(|e| format!("group {g}: {e}"))?;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:88  [LS3: for bar in batch.iter() {]  for bar in batch.iter() {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:94  [LS3: h = eat(h, bar.timestamp_micros as u64);]  h = eat(h, bar.timestamp_micros as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:95  [LS3: h = eat(h, bar.open.raw() as u64);]  h = eat(h, bar.open.raw() as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:96  [LS3: h = eat(h, bar.high.raw() as u64);]  h = eat(h, bar.high.raw() as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:97  [LS3: h = eat(h, bar.low.raw() as u64);]  h = eat(h, bar.low.raw() as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:98  [LS3: h = eat(h, bar.close.raw() as u64);]  h = eat(h, bar.close.raw() as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:99  [LS3: h = eat(h, bar.volume as u64);]  h = eat(h, bar.volume as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:102  [LS3: h = eat(h, bar.open_interest as u64);]  h = eat(h, bar.open_interest as u64);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:103  [LS3: h = eat_opt(h, bar.spot_at_bar.map(|p| p.raw() as u64));]  h = eat_opt(h, bar.spot_at_bar.map(|p| p.raw() as u64));
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:104  [LS3: h = eat_opt(h, bar.greeks_provenance_id.map(|v| i64::from(v) as u64));]  h = eat_opt(h, bar.greeks_provenance_id.map(|v| i64::from(v) as u64));
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:106  [LS3: match bar.greeks {]  match bar.greeks {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:107  [LS3: Some(g) => {]  Some(g) => {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:108  [LS3: h = eat(h, 1);]  h = eat(h, 1);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:109  [LS3: for v in []  for v in [
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:110  [LS3: g.iv,]  g.iv,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:111  [LS3: g.delta,]  g.delta,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:112  [LS3: g.gamma,]  g.gamma,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:113  [LS3: g.theta,]  g.theta,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:114  [LS3: g.vega,]  g.vega,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:115  [LS3: g.rho,]  g.rho,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:116  [LS3: g.t_years_used,]  g.t_years_used,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:117  [LS3: g.rate_used,]  g.rate_used,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:118  [LS3: ] {]  ] {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:119  [LS3: h = eat(h, v.to_bits());]  h = eat(h, v.to_bits());
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:120  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:122  [LS3: None => h = eat(h, 0),]  None => h = eat(h, 0),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:124  [LS3: rows += 1;]  rows += 1;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:127  [LS3: Ok((h, rows, layout))]  Ok((h, rows, layout))
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:128  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:131  [LS3: fn month_files(contract: &Path, into: &mut Vec<PathBuf>) {]  fn month_files(contract: &Path, into: &mut Vec<PathBuf>) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:132  [LS3: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:132:9 False  [LS3: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:132:9 True  [LS3: let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {]  let Ok(years) = std::fs::read_dir(contract.join("1minute")) else {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:133  [LS3: return;]  return;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:135  [LS3: let mut ys: Vec<PathBuf> = years.flatten().map(|e| e.path()).collect();]  let mut ys: Vec<PathBuf> = years.flatten().map(|e| e.path()).collect();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:136  [LS3: ys.sort();]  ys.sort();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:137  [LS3: for y in ys {]  for y in ys {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:138  [LS3: let Ok(months) = std::fs::read_dir(&y) else {]  let Ok(months) = std::fs::read_dir(&y) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:138:13 False  [LS3: let Ok(months) = std::fs::read_dir(&y) else {]  let Ok(months) = std::fs::read_dir(&y) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:138:13 True  [LS3: let Ok(months) = std::fs::read_dir(&y) else {]  let Ok(months) = std::fs::read_dir(&y) else {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:139  [LS3: continue;]  continue;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:141  [LS3: let mut ms: Vec<PathBuf> = months.flatten().map(|e| e.path()).collect();]  let mut ms: Vec<PathBuf> = months.flatten().map(|e| e.path()).collect();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:142  [LS3: ms.sort();]  ms.sort();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:143  [LS3: into.extend(ms);]  into.extend(ms);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:145  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:149  [LS3: fn sample_dirs(root: &Path, want: usize) -> Vec<PathBuf> {]  fn sample_dirs(root: &Path, want: usize) -> Vec<PathBuf> {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:150  [LS3: let Ok(entries) = std::fs::read_dir(root) else {]  let Ok(entries) = std::fs::read_dir(root) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:150:9 False  [LS3: let Ok(entries) = std::fs::read_dir(root) else {]  let Ok(entries) = std::fs::read_dir(root) else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:150:9 True  [LS3: let Ok(entries) = std::fs::read_dir(root) else {]  let Ok(entries) = std::fs::read_dir(root) else {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:151  [LS3: return Vec::new();]  return Vec::new();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:153  [LS3: let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();]  let mut all: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:154  [LS3: all.sort();]  all.sort();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:155  [LS3: if all.is_empty() || want == 0 {]  if all.is_empty() || want == 0 {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:155:8 False  [LS3: if all.is_empty() || want == 0 {]  if all.is_empty() || want == 0 {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:155:8 True  [LS3: if all.is_empty() || want == 0 {]  if all.is_empty() || want == 0 {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:155:26 False  [LS3: if all.is_empty() || want == 0 {]  if all.is_empty() || want == 0 {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:155:26 True  [LS3: if all.is_empty() || want == 0 {]  if all.is_empty() || want == 0 {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:156  [LS3: return Vec::new();]  return Vec::new();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:157  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:158  [LS3: let stride = (all.len() / want).max(1);]  let stride = (all.len() / want).max(1);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:159  [LS3: all.into_iter().step_by(stride).take(want).collect()]  all.into_iter().step_by(stride).take(want).collect()
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:160  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:164  [LS3: fn a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest() {]  fn a_wide_sample_of_the_real_lake_decodes_with_no_refusal_and_a_stable_digest() {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:165  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:165:9 False  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:165:9 True  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:166  [LS3: panic!(]  panic!(
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:170  [LS3: let root = Path::new(&home).join(".brutex/lake/bars/NSE");]  let root = Path::new(&home).join(".brutex/lake/bars/NSE");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:171  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:172  [LS3: root.exists(),]  root.exists(),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:176  [LS3: let mut targets: Vec<PathBuf> = Vec::new();]  let mut targets: Vec<PathBuf> = Vec::new();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:177  [LS3: for contract in sample_dirs(&root.join("FNO"), 120) {]  for contract in sample_dirs(&root.join("FNO"), 120) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:178  [LS3: month_files(&contract, &mut targets);]  month_files(&contract, &mut targets);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:179  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:180  [LS3: for symbol in sample_dirs(&root.join("CASH"), 20) {]  for symbol in sample_dirs(&root.join("CASH"), 20) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:181  [LS3: month_files(&symbol, &mut targets);]  month_files(&symbol, &mut targets);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:182  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:183  [LS3: for symbol in sample_dirs(&root.join("INDEX"), 5) {]  for symbol in sample_dirs(&root.join("INDEX"), 5) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:184  [LS3: month_files(&symbol, &mut targets);]  month_files(&symbol, &mut targets);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:185  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:186  [LS3: targets.sort();]  targets.sort();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:188  [LS3: let mut refused: Vec<String> = Vec::new();]  let mut refused: Vec<String> = Vec::new();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:189  [LS3: let mut files = 0_usize;]  let mut files = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:190  [LS3: let mut rows = 0_usize;]  let mut rows = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:191  [LS3: let mut fno = 0_usize;]  let mut fno = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:192  [LS3: let mut cash = 0_usize;]  let mut cash = 0_usize;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:193  [LS3: let mut whole = FNV_OFFSET;]  let mut whole = FNV_OFFSET;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:195  [LS3: for path in &targets {]  for path in &targets {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:196  [LS3: let shown = path.strip_prefix(&root).unwrap_or(path).display();]  let shown = path.strip_prefix(&root).unwrap_or(path).display();
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:197  [LS3: match digest_file(path) {]  match digest_file(path) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:198  [LS3: Ok((h, n, layout)) => {]  Ok((h, n, layout)) => {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:199  [LS3: println!("{h:016x} {n:>6} {shown}");]  println!("{h:016x} {n:>6} {shown}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:200  [LS3: whole = eat(whole, h);]  whole = eat(whole, h);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:201  [LS3: files += 1;]  files += 1;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:202  [LS3: rows += n;]  rows += n;
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:203  [LS3: match layout {]  match layout {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:204  [LS3: Layout::Fno => fno += 1,]  Layout::Fno => fno += 1,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:205  [LS3: Layout::Cash => cash += 1,]  Layout::Cash => cash += 1,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:208  [LS3: Err(e) => refused.push(format!("{shown}: {e}")),]  Err(e) => refused.push(format!("{shown}: {e}")),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:212  [LS3: println!("REAL LAKE DIGEST");]  println!("REAL LAKE DIGEST");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:213  [LS3: println!("  files    : {files}");]  println!("  files    : {files}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:214  [LS3: println!("  rows     : {rows}");]  println!("  rows     : {rows}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:215  [LS3: println!("  F&O      : {fno}, cash/index: {cash}");]  println!("  F&O      : {fno}, cash/index: {cash}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:216  [LS3: println!("  refusals : {}", refused.len());]  println!("  refusals : {}", refused.len());
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:217  [LS3: println!("  WHOLE    : {whole:016x}");]  println!("  WHOLE    : {whole:016x}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:218  [LS3: for line in refused.iter().take(20) {]  for line in refused.iter().take(20) {
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:219  [LS3: println!("    REFUSED {line}");]  println!("    REFUSED {line}");
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:220  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:222  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:223  [LS3: refused.is_empty(),]  refused.is_empty(),
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:225  [LS3: refused.join("\n  ")]  refused.join("\n  ")
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:227  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:228  [LS3: files >= MINIMUM_FILES,]  files >= MINIMUM_FILES,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:231  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:232  [LS3: fno > 0 && cash > 0,]  fno > 0 && cash > 0,
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:232:9 False  [LS3: fno > 0 && cash > 0,]  fno > 0 && cash > 0,
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:232:9 True  [LS3: fno > 0 && cash > 0,]  fno > 0 && cash > 0,
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:232:20 False  [LS3: fno > 0 && cash > 0,]  fno > 0 && cash > 0,
IGNORED     BRANCH crates/lake/tests/real_lake_regression.rs:232:20 True  [LS3: fno > 0 && cash > 0,]  fno > 0 && cash > 0,
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:235  [LS3: assert!(rows > 0);]  assert!(rows > 0);
IGNORED     LINE   crates/lake/tests/real_lake_regression.rs:236  [LS3: }]  }
(F)         LINE   crates/lake/tests/refusals.rs:176  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:234  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:310  [LS3]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:311  [LS3]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:312  [LS3]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:313  [LS3]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:314  [LS3]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:315  [LS3]  let invented = got.len() - real;
(F)         LINE   crates/lake/tests/refusals.rs:316  [LS3]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:317  [LS3]  let truth = first.map(|i| OI_BASE + i64::try_from(i).expect("fits"));
(F)         LINE   crates/lake/tests/refusals.rs:318  [LS3]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:326  [LS3]  batch.len()
(F)         LINE   crates/lake/tests/refusals.rs:377  [LS3]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:378  [LS3]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:379  [LS3]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:380  [LS3]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:381  [LS3]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:382  [LS3]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:383  [LS3]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:386  [LS3]  batch.len(),
(F)         LINE   crates/lake/tests/refusals.rs:387  [LS3]  got.len() - real,
(F)         LINE   crates/lake/tests/refusals.rs:388  [LS3]  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F)         LINE   crates/lake/tests/refusals.rs:432  [LS3]  Ok(batch) => {
(F)         LINE   crates/lake/tests/refusals.rs:433  [LS3]  let got: Vec<Option<i64>> = (0..batch.len())
(F)         LINE   crates/lake/tests/refusals.rs:434  [LS3]  .map(|i| batch.row(i).expect("row in range").open_interest())
(F)         LINE   crates/lake/tests/refusals.rs:435  [LS3]  .collect();
(F)         LINE   crates/lake/tests/refusals.rs:436  [LS3]  let real = got.iter().filter(|v| v.is_some()).count();
(F)         LINE   crates/lake/tests/refusals.rs:437  [LS3]  let first = got.iter().position(Option::is_none);
(F)         LINE   crates/lake/tests/refusals.rs:438  [LS3]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:442  [LS3]  batch.len(),
(F)         LINE   crates/lake/tests/refusals.rs:443  [LS3]  got.len() - real,
(F)         LINE   crates/lake/tests/refusals.rs:444  [LS3]  first.map(|i| OI_BASE + i64::try_from(i).expect("fits"))
(F)         LINE   crates/lake/tests/refusals.rs:558  [LS3]  _ => panic!("one INT64 column"),
(F)         LINE   crates/lake/tests/refusals.rs:682  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:703  [LS3]  Err(other) => panic!(
(F)         LINE   crates/lake/tests/refusals.rs:715  [LS3]  Ok(file) => panic!(
(F)         LINE   crates/lake/tests/refusals.rs:795  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:814  [LS3]  other => panic!("a {what} leaf must be refused by name, got {other:?}"),
(F)         BRANCH crates/lake/tests/refusals.rs:1061:17 False  [LS3]  let ColumnWriter::Int64ColumnWriter(typed) = column.untyped() else {
(F)         LINE   crates/lake/tests/refusals.rs:1062  [LS3]  panic!("every leaf is INT64");
(F)         LINE   crates/lake/tests/refusals.rs:1203  [LS3]  s.unreadable[0]
(F)         LINE   crates/lake/tests/refusals.rs:1240  [LS3]  s.unreadable[0]
IGNORED     LINE   crates/lake/tests/refusals.rs:1313  [LS3: fn no_real_lake_file_triggers_either_defect() {]  fn no_real_lake_file_triggers_either_defect() {
IGNORED     LINE   crates/lake/tests/refusals.rs:1314  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     BRANCH crates/lake/tests/refusals.rs:1314:9 False  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     BRANCH crates/lake/tests/refusals.rs:1314:9 True  [LS3: let Some(home) = std::env::var_os("HOME") else {]  let Some(home) = std::env::var_os("HOME") else {
IGNORED     LINE   crates/lake/tests/refusals.rs:1315  [LS3: panic!(]  panic!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1319  [LS3: let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");]  let root = std::path::Path::new(&home).join(".brutex/lake/bars/NSE");
IGNORED     LINE   crates/lake/tests/refusals.rs:1320  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1321  [LS3: root.exists(),]  root.exists(),
IGNORED     LINE   crates/lake/tests/refusals.rs:1325  [LS3: let mut targets: Vec<(std::path::PathBuf, String)> = Vec::new();]  let mut targets: Vec<(std::path::PathBuf, String)> = Vec::new();
IGNORED     LINE   crates/lake/tests/refusals.rs:1326  [LS3: for contract in sample_dirs(&root.join("FNO"), 400) {]  for contract in sample_dirs(&root.join("FNO"), 400) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1327  [LS3: month_files(&contract, &mut targets);]  month_files(&contract, &mut targets);
IGNORED     LINE   crates/lake/tests/refusals.rs:1328  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1329  [LS3: for symbol in sample_dirs(&root.join("CASH"), 30) {]  for symbol in sample_dirs(&root.join("CASH"), 30) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1330  [LS3: month_files(&symbol, &mut targets);]  month_files(&symbol, &mut targets);
IGNORED     LINE   crates/lake/tests/refusals.rs:1331  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1332  [LS3: for symbol in sample_dirs(&root.join("INDEX"), 8) {]  for symbol in sample_dirs(&root.join("INDEX"), 8) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1333  [LS3: month_files(&symbol, &mut targets);]  month_files(&symbol, &mut targets);
IGNORED     LINE   crates/lake/tests/refusals.rs:1334  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1336  [LS3: let mut scan = Scan::default();]  let mut scan = Scan::default();
IGNORED     LINE   crates/lake/tests/refusals.rs:1337  [LS3: for (path, year) in &targets {]  for (path, year) in &targets {
IGNORED     LINE   crates/lake/tests/refusals.rs:1338  [LS3: scan_file(path, year, &mut scan);]  scan_file(path, year, &mut scan);
IGNORED     LINE   crates/lake/tests/refusals.rs:1339  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1341  [LS3: println!("scanned {} real lake files", scan.files);]  println!("scanned {} real lake files", scan.files);
IGNORED     LINE   crates/lake/tests/refusals.rs:1342  [LS3: println!(]  println!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1346  [LS3: println!("  row groups: {}, rows: {}", scan.row_groups, scan.rows);]  println!("  row groups: {}, rows: {}", scan.row_groups, scan.rows);
IGNORED     LINE   crates/lake/tests/refusals.rs:1347  [LS3: println!("  years covered: {:?}", scan.years);]  println!("  years covered: {:?}", scan.years);
IGNORED     LINE   crates/lake/tests/refusals.rs:1348  [LS3: println!(]  println!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1350  [LS3: scan.declared_short.len()]  scan.declared_short.len()
IGNORED     LINE   crates/lake/tests/refusals.rs:1352  [LS3: println!(]  println!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1354  [LS3: scan.pages_short.len()]  scan.pages_short.len()
IGNORED     LINE   crates/lake/tests/refusals.rs:1356  [LS3: println!(]  println!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1358  [LS3: scan.nested.len()]  scan.nested.len()
IGNORED     LINE   crates/lake/tests/refusals.rs:1360  [LS3: println!(]  println!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1362  [LS3: scan.unreadable.len()]  scan.unreadable.len()
IGNORED     LINE   crates/lake/tests/refusals.rs:1364  [LS3: for line in scan.declared_short.iter().take(10) {]  for line in scan.declared_short.iter().take(10) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1365  [LS3: println!("    DECLARED SHORT {line}");]  println!("    DECLARED SHORT {line}");
IGNORED     LINE   crates/lake/tests/refusals.rs:1366  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1367  [LS3: for line in scan.pages_short.iter().take(10) {]  for line in scan.pages_short.iter().take(10) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1368  [LS3: println!("    PAGES SHORT {line}");]  println!("    PAGES SHORT {line}");
IGNORED     LINE   crates/lake/tests/refusals.rs:1369  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1370  [LS3: for line in scan.nested.iter().take(10) {]  for line in scan.nested.iter().take(10) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1371  [LS3: println!("    NESTED {line}");]  println!("    NESTED {line}");
IGNORED     LINE   crates/lake/tests/refusals.rs:1372  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1373  [LS3: for line in scan.unreadable.iter().take(10) {]  for line in scan.unreadable.iter().take(10) {
IGNORED     LINE   crates/lake/tests/refusals.rs:1374  [LS3: println!("    UNREADABLE {line}");]  println!("    UNREADABLE {line}");
IGNORED     LINE   crates/lake/tests/refusals.rs:1375  [LS3: }]  }
IGNORED     LINE   crates/lake/tests/refusals.rs:1377  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1378  [LS3: scan.files >= 200,]  scan.files >= 200,
IGNORED     LINE   crates/lake/tests/refusals.rs:1382  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1383  [LS3: scan.years.len() >= 3,]  scan.years.len() >= 3,
IGNORED     LINE   crates/lake/tests/refusals.rs:1387  [LS3: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
IGNORED     BRANCH crates/lake/tests/refusals.rs:1387:13 False  [LS3: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
IGNORED     BRANCH crates/lake/tests/refusals.rs:1387:13 True  [LS3: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
IGNORED     BRANCH crates/lake/tests/refusals.rs:1387:29 False  [LS3: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
IGNORED     BRANCH crates/lake/tests/refusals.rs:1387:29 True  [LS3: assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");]  assert!(scan.fno > 0 && scan.cash > 0, "both schemas must appear");
IGNORED     LINE   crates/lake/tests/refusals.rs:1388  [LS3: assert!(]  assert!(
IGNORED     LINE   crates/lake/tests/refusals.rs:1389  [LS3: scan.declared_short.is_empty(),]  scan.declared_short.is_empty(),
IGNORED     LINE   crates/lake/tests/refusals.rs:1392  [LS3: assert!(scan.pages_short.is_empty(), "defect 1 fires on real data");]  assert!(scan.pages_short.is_empty(), "defect 1 fires on real data");
IGNORED     LINE   crates/lake/tests/refusals.rs:1393  [LS3: assert!(scan.nested.is_empty(), "defect 3 fires on real data");]  assert!(scan.nested.is_empty(), "defect 3 fires on real data");
IGNORED     LINE   crates/lake/tests/refusals.rs:1394  [LS3: assert!(scan.unreadable.is_empty(), "a real file would not parse");]  assert!(scan.unreadable.is_empty(), "a real file would not parse");
IGNORED     LINE   crates/lake/tests/refusals.rs:1395  [LS3: }]  }
(F)         LINE   crates/lake/tests/refusals.rs:1558  [LS3]  Ok(other) => {
(F)         LINE   crates/lake/tests/refusals.rs:1559  [LS3]  panic!(
(F)         LINE   crates/lake/tests/refusals.rs:1569  [LS3]  other.to_string() == padded,
(F)         LINE   crates/lake/tests/refusals.rs:1570  [LS3]  other == good,
(F)         LINE   crates/lake/tests/refusals.rs:1571  [LS3]  other.key() == good.key()
(F)         BRANCH crates/lake/tests/refusals.rs:1619:9 False  [LS3]  text.contains("400") && text.contains("2480"),
(F)         BRANCH crates/lake/tests/refusals.rs:1619:33 False  [LS3]  text.contains("400") && text.contains("2480"),
(F)         LINE   crates/lake/tests/refusals.rs:1699  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         BRANCH crates/lake/tests/refusals.rs:1748:71 False  [LS3]  Err(LakeError::OpenInterestIsNullSentinel { row }) if row == at
(F)         LINE   crates/lake/tests/refusals.rs:1841  [LS3]  _ => panic!("the cash layout has no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:1870  [LS3]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:1871  [LS3]  Ok(_) => panic!("{said}: a timestamp this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:1949  [LS3]  _ => panic!("this fixture writes no other physical type"),
(F)         LINE   crates/lake/tests/refusals.rs:1980  [LS3]  Err(other) => panic!("refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:1981  [LS3]  Ok(_) => panic!("a MILLIS timestamp opened and would decode as micros"),
(F)         LINE   crates/lake/tests/refusals.rs:1999  [LS3]  Err(wrong) => panic!("{other}: refused as the wrong thing: {wrong:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2000  [LS3]  Ok(_) => panic!("{other}: a timestamp this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2050  [LS3]  Err(other) => panic!("INT96 refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2051  [LS3]  Ok(_) => panic!("an INT96 timestamp opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2101  [LS3]  Err(other) => panic!("{said}: refused as the wrong thing: {other:?}"),
(F)         LINE   crates/lake/tests/refusals.rs:2102  [LS3]  Ok(_) => panic!("{said}: an integer column this reader would misread was opened"),
(F)         LINE   crates/lake/tests/refusals.rs:2137  [LS3]  other => panic!("an INT32 {leaf}: {:?}", other.map(|f| f.num_rows())),
(F)         LINE   crates/lake/tests/synthetic.rs:136  [LS3]  _ => panic!("writer type does not match the column"),
(F)         LINE   crates/lake/tests/synthetic.rs:231  [LS3]  f.created_by()
(F)         LINE   crates/lake/tests/synthetic.rs:353  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:355  [LS3]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:403  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:405  [LS3]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:453  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:455  [LS3]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:473  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:475  [LS3]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:626  [LS3]  other => panic!("expected PartialGreeks, got {other:?}", other = other.err()),
(F)         LINE   crates/lake/tests/synthetic.rs:639  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:641  [LS3]  other = other.err()
(F)         LINE   crates/lake/tests/synthetic.rs:662  [LS3]  matches!(
(F)         LINE   crates/lake/tests/synthetic.rs:780  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:782  [LS3]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:820  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:822  [LS3]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:952  [LS3]  other => panic!("page type {} is not one this writer emits", other.0),
(F)         LINE   crates/lake/tests/synthetic.rs:1154  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:1156  [LS3]  other.map(|groups| groups.len())
(F)         LINE   crates/lake/tests/synthetic.rs:1221  [LS3]  other => panic!(
(F)         LINE   crates/lake/tests/synthetic.rs:1223  [LS3]  other.map(|b| b.len())
(F)         LINE   crates/lake/tests/synthetic.rs:1307  [LS3]  Ok(rows) => panic!("decoded {} rows from a broken page", rows.len()),
(U)         BRANCH crates/runner/src/admission.rs:2194:36 True  [LS2]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5056:36 True  [LS2]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5228:36 True  [LS2]  if supplied != computed || computed.canonical_bytes().as_slice() != verdict_bytes {
(U)         BRANCH crates/runner/src/admission.rs:5771:45 True  [LS2]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
(U)         BRANCH crates/runner/src/admission.rs:6121:45 True  [LS2]  if evidence.values != decoded_values || evidence.canonical_bytes().as_slice() != bytes {
(F)         LINE   crates/runner/src/admission.rs:7423  [LS2]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7429  [LS2]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7435  [LS2]  reason.name()
(F)         LINE   crates/runner/src/admission.rs:7466  [LS2]  left.name(),
(F)         LINE   crates/runner/src/admission.rs:7467  [LS2]  right.name()
(F)         BRANCH crates/runner/src/admission_reader_cover_tests.rs:285:70 False  [LS2]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F)         BRANCH crates/runner/src/admission_reader_cover_tests.rs:289:70 False  [LS2]  Err(AdmissionCanonicalRefusalV1::Length { expected, .. }) if expected == at + 9
(F)         BRANCH crates/runner/src/audit.rs:2224:13 False  [LS2]  ranked
(F)         BRANCH crates/runner/src/audit.rs:2227:20 False  [LS2]  && ranked.ends_with("what selection ranks on, gross of every charge"),
(F)         BRANCH crates/runner/src/audit.rs:2358:44 False  [LS2]  line.is_empty() || line.starts_with("  "),
(F)         BRANCH crates/runner/src/audit.rs:2467:29 False  [LS2]  decision || token == "V6",
(F)         BRANCH crates/runner/src/audit.rs:2620:40 False  [LS2]  line.is_empty() || line.starts_with("  "),
(F)         BRANCH crates/runner/src/audit.rs:2788:13 False  [LS2]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F)         BRANCH crates/runner/src/audit.rs:2788:54 False  [LS2]  out.contains("durable selected exit") && out.contains("0/-/-"),
(F)         BRANCH crates/runner/src/audit.rs:3067:17 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3067:35 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3067:54 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:17 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:35 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(F)         BRANCH crates/runner/src/audit.rs:3666:54 False  [LS2]  assert!(best.is_some() && sharp.is_some() && best != sharp);
(U)         BRANCH crates/runner/src/bootstrap.rs:628:17 False  [LS2]  let Some(own) = stats.get(s) else { continue };
(U)         BRANCH crates/runner/src/bootstrap.rs:727:8 True  [LS2]  if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {
(U)         LINE   crates/runner/src/bootstrap.rs:766  [LS2]  )?,
(U)         BRANCH crates/runner/src/bootstrap.rs:802:8 True  [LS2]  if !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:803  [LS2]  return None;
(U)         LINE   crates/runner/src/bootstrap.rs:851  [LS2]  )?,
(U)         BRANCH crates/runner/src/bootstrap.rs:868:8 True  [LS2]  if stats.iter().any(|summary| {
(U)         BRANCH crates/runner/src/bootstrap.rs:869:9 True  [LS2]  !summary.mean.is_finite()
(U)         BRANCH crates/runner/src/bootstrap.rs:870:16 True  [LS2]  || !summary.standard_error.is_finite()
(U)         LINE   crates/runner/src/bootstrap.rs:873  [LS2]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:892:8 True  [LS2]  if matched_or_exceeded > draws || !observed.is_finite() {
(U)         BRANCH crates/runner/src/bootstrap.rs:892:39 True  [LS2]  if matched_or_exceeded > draws || !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:893  [LS2]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:900:8 True  [LS2]  if !p_value.is_finite() {
(U)         LINE   crates/runner/src/bootstrap.rs:901  [LS2]  return None;
(U)         BRANCH crates/runner/src/bootstrap.rs:1347:8 True  [LS2]  if observed.iter().any(|value| !value.is_finite()) {
(U)         LINE   crates/runner/src/bootstrap.rs:1348  [LS2]  return None;
(U)         LINE   crates/runner/src/bootstrap.rs:1362  [LS2]  _ => left.cmp(right),
(U)         LINE   crates/runner/src/bootstrap.rs:1547  [LS2]  _ => left.cmp(right),
(U)         BRANCH crates/runner/src/bootstrap.rs:1565:13 False  [LS2]  let Some(&strategy) = order.get(rank) else {
(U)         LINE   crates/runner/src/bootstrap.rs:1566  [LS2]  continue;
(U)         BRANCH crates/runner/src/bootstrap.rs:1568:13 False  [LS2]  let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {
(U)         LINE   crates/runner/src/bootstrap.rs:1569  [LS2]  continue;
(U)         BRANCH crates/runner/src/bootstrap.rs:1588:16 False  [LS2]  if let Some(slot) = bars.get_mut(rank) {
(U)         BRANCH crates/runner/src/bootstrap.rs:1599:17 False  [LS2]  let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))
(U)         LINE   crates/runner/src/bootstrap.rs:1601  [LS2]  break;
(U)         BRANCH crates/runner/src/bootstrap.rs:1620:16 False  [LS2]  if let Some(this_round) = out.get_mut(first..) {
(U)         LINE   crates/runner/src/bootstrap.rs:1622  [LS2]  }
(F)         LINE   crates/runner/src/bootstrap.rs:2108  [LS2]  v.calibration()
(F)         LINE   crates/runner/src/bootstrap.rs:2188  [LS2]  (n as f64).sqrt()
(F)         BRANCH crates/runner/src/bootstrap.rs:2435:13 False  [LS2]  names.contains(&18) && names.contains(&19),
(F)         BRANCH crates/runner/src/bootstrap.rs:2435:36 False  [LS2]  names.contains(&18) && names.contains(&19),
(F)         BRANCH crates/runner/src/bootstrap.rs:2652:17 False  [LS2]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F)         BRANCH crates/runner/src/bootstrap.rs:2652:29 False  [LS2]  assert!(fast > 0 && kept > 0, "both paths must be exercised");
(F)         LINE   crates/runner/src/bootstrap.rs:2722  [LS2]  draw.runs.len()
(F)         BRANCH crates/runner/src/bootstrap.rs:3409:13 False  [LS2]  rounds < rejected.len() && rejected.len() <= set.len(),
(F)         BRANCH crates/runner/src/bootstrap.rs:3409:40 False  [LS2]  rounds < rejected.len() && rejected.len() <= set.len(),
(F)         LINE   crates/runner/src/bootstrap.rs:3411  [LS2]  rounds + 1,
(F)         LINE   crates/runner/src/bootstrap.rs:3412  [LS2]  rejected.len(),
(F)         LINE   crates/runner/src/bootstrap.rs:3413  [LS2]  set.len()
(F)         LINE   crates/runner/src/bootstrap.rs:3520  [LS2]  rc.map(|v| v.p_value)
(F)         LINE   crates/runner/src/bootstrap.rs:3526  [LS2]  v.map(|v| (v.p_value, v.clears()))
(U)         BRANCH crates/runner/src/bootstrap_family_pass.rs:461:20 True  [LS2]  || !stat.standard_error.is_finite()
(F)         BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:13 False  [LS2]  matched > 0 && matched < draws,
(F)         BRANCH crates/runner/src/bootstrap_family_pass_tests.rs:183:28 False  [LS2]  matched > 0 && matched < draws,
(U)         BRANCH crates/runner/src/bootstrap_zero_v2.rs:187:45 True  [LS2]  if summary.standard_error <= 0.0 || !summary.standard_error.is_finite() {
(U)         BRANCH crates/runner/src/bootstrap_zero_v2.rs:191:12 True  [LS2]  if !observed.is_finite() {
(U)         LINE   crates/runner/src/bootstrap_zero_v2.rs:192  [LS2]  return Err(Refusal::Numerical);
(U)         BRANCH crates/runner/src/excursion.rs:234:24 False  [LS2]  if let Some(prefix) = observed.get_mut(..bound) {
(U)         BRANCH crates/runner/src/excursion.rs:1221:20 False  [LS2]  if let Some(slot) = out.adverse.get_mut(stop_cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1232:20 False  [LS2]  if let Some(slot) = out.favourable.get_mut(target_cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1253:20 False  [LS2]  if let Some(slot) = armed_from.get_mut(arm) {
(U)         BRANCH crates/runner/src/excursion.rs:1397:13 False  [LS2]  let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
(U)         LINE   crates/runner/src/excursion.rs:1405  [LS2]  break;
(U)         BRANCH crates/runner/src/excursion.rs:1472:16 False  [LS2]  if let Some(slot) = crossed_at.get_mut(*cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1479:16 False  [LS2]  if let Some(slot) = peak.get_mut(*cursor) {
(U)         BRANCH crates/runner/src/excursion.rs:1482:16 False  [LS2]  if let Some(slot) = peak_raised.get_mut(*cursor) {
(F)         LINE   crates/runner/src/excursion.rs:1682  [LS2]  tightest(8),
(F)         LINE   crates/runner/src/excursion.rs:1683  [LS2]  tightest(4)
(F)         LINE   crates/runner/src/excursion.rs:1688  [LS2]  tightest(4),
(F)         LINE   crates/runner/src/excursion.rs:1689  [LS2]  tightest(2)
(F)         LINE   crates/runner/src/excursion.rs:2582  [LS2]  sample.len()
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:605:12 False  [LS2]  && core::ptr::eq(window, evaluated_execution_1m)
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:614:9 False  [LS2]  let Some(end) = start.checked_add(evaluated_execution_1m.len()) else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:615  [LS2]  return Err(ExitGridErrorV1::EvaluatedExecutionOutsideReferenceContext);
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:676:13 False  [LS2]  let Some(last) = forged.last_mut() else {
(F)         LINE   crates/runner/src/exit_grid_policy.rs:677  [LS2]  unreachable!("the copy holds bars 17..250")
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:1132:12 True  [LS2]  if pair_count > self.ratios.max_pairs() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1133  [LS2]  return Err(ExitGridErrorV1::RatioPairLimitExceeded {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1134  [LS2]  needed: pair_count,
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1135  [LS2]  max: self.ratios.max_pairs(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1136  [LS2]  });
(U)         LINE   crates/runner/src/exit_grid_policy.rs:1173  [LS2]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2132:25 False  [LS2]  let Some(next) = pricing_refused.checked_add(1) else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2133  [LS2]  return false;
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2458  [LS2]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2676:12 True  [LS2]  if !self.chosen_is_in_bounds(coordinate) {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2677  [LS2]  return Err(ExitGridErrorV1::InvalidChosenCoordinate);
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2680:12 True  [LS2]  if selected.resolution_digest != self.digest
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2681:16 True  [LS2]  || selected.run_id != evaluated.run_id
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2682:16 True  [LS2]  || selected.mask != evaluated.mask
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2683:16 True  [LS2]  || selected.horizon != evaluated.horizon
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2684:16 True  [LS2]  || selected.column_digest != evaluated.column_digest
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2685:16 True  [LS2]  || selected.evaluation_spec != evaluated.evaluation_spec
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2686:16 True  [LS2]  || selected.side != evaluated.side
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2687:16 True  [LS2]  || selected.coordinate != coordinate
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2688:16 True  [LS2]  || selected.training_cell != *cell
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:2689:16 True  [LS2]  || selected.selection_digest
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2702  [LS2]  return Err(ExitGridErrorV1::SelectionDigestMismatch);
(U)         LINE   crates/runner/src/exit_grid_policy.rs:2877  [LS2]  )?;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3413:13 False  [LS2]  let Some(left) = pair.first() else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3414  [LS2]  return true;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3416:13 False  [LS2]  let Some(right) = pair.last() else {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3417  [LS2]  return true;
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3595:12 True  [LS2 line 3591]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3595:45 True  [LS2 line 3591]  if bar.high <= 0 || bar.low <= 0 || bar.close <= 0 {
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:3672:8 True  [LS2 line 3668]  if out.is_empty() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:3673  [LS2 line 3669]  return Err(ExitGridErrorV1::EmptyResolvedAxis(axis));
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4147:12 True  [LS2 line 4139]  || !census.reconciles()
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4148:12 True  [LS2 line 4140]  || census.offered != u64::try_from(bars).unwrap_or(u64::MAX)
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4149:12 True  [LS2 line 4141]  || census.refused() != 0
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4164:8 True  [LS2 line 4156]  if column.bits().len() != column.sources().len() {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4165  [LS2 line 4157]  return Err(ExitGridErrorV1::ColumnShapeMismatch {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4166  [LS2 line 4158]  bits: column.bits().len(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4167  [LS2 line 4159]  sources: column.sources().len(),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4168  [LS2 line 4160]  });
(U)         BRANCH crates/runner/src/exit_grid_policy.rs:4181:12 True  [LS2 line 4173]  if previous.is_some_and(|before| source <= before) {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4182  [LS2 line 4174]  return Err(ExitGridErrorV1::ColumnSourcesNotIncreasing {
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4183  [LS2 line 4175]  previous: previous.unwrap_or(source),
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4184  [LS2 line 4176]  current: source,
(U)         LINE   crates/runner/src/exit_grid_policy.rs:4185  [LS2 line 4177]  });
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:7850:13 False  [LS2 line 7842]  let crate::grid::ReplayPathV1::Priceable(price) = &mut candidate.path else {
(F)         LINE   crates/runner/src/exit_grid_policy.rs:7851  [LS2 line 7843]  unreachable!("the fixture prices every path")
(F)         LINE   crates/runner/src/exit_grid_policy.rs:7993  [LS2 line 7985]  universe.candidates.len()
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:7997:13 False  [LS2 line 7989]  first.entry_bar > 0 && matches!(first.path, crate::grid::ReplayPathV1::Priceable(_))
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:13 False  [LSA]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:26 False  [LSA]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8314:41 False  [LSA]  stops > 0 && targets > 0 && trails > 0,
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8861:17 False  [LS2 line 8754]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8861:44 False  [LS2 line 8754]  assert!(cell.ambiguous_bars > 3 && cell.gapped <= 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8870:17 False  [LS2 line 8763]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(F)         BRANCH crates/runner/src/exit_grid_policy.rs:8870:45 False  [LS2 line 8763]  assert!(cell.ambiguous_bars <= 3 && cell.gapped > 2, "{cell:?}");
(U)         BRANCH crates/runner/src/expression_execution.rs:357:12 True  [LS2]  if summary.hits != grid.signals {
(U)         LINE   crates/runner/src/expression_execution.rs:358  [LS2]  return Err("expression exact truth summary differs from grid signals".to_owned());
(U)         LINE   crates/runner/src/expression_oos.rs:299  [LS2]  )?;
(U)         BRANCH crates/runner/src/expression_oos.rs:302:12 True  [LS2]  if summary.hits != grid.signals {
(U)         LINE   crates/runner/src/expression_oos.rs:303  [LS2]  return Err("later expression truth/grid signals differ".into());
(U)         LINE   crates/runner/src/expression_validation.rs:325  [LS2]  )?;
(U)         BRANCH crates/runner/src/expression_validation.rs:345:12 True  [LS2]  if totals.trades != cell.trades
(U)         BRANCH crates/runner/src/expression_validation.rs:346:16 True  [LS2]  || totals.wins != cell.wins
(U)         BRANCH crates/runner/src/expression_validation.rs:347:16 True  [LS2]  || totals.return_paisa != cell.pessimistic
(U)         LINE   crates/runner/src/expression_validation.rs:349  [LS2]  return Err("fixed-training fold totals disagree with exact later cell".into());
(U)         BRANCH crates/runner/src/expression_validation.rs:516:12 True  [LS2]  if first != last
(U)         BRANCH crates/runner/src/expression_validation.rs:517:16 True  [LS2]  || trade.entry_micros != entry.ts_micros
(U)         BRANCH crates/runner/src/expression_validation.rs:518:16 True  [LS2]  || trade.exit_micros != exit.ts_micros
(U)         BRANCH crates/runner/src/expression_validation.rs:519:16 True  [LS2]  || day(entry.ts_micros) != day(exit.ts_micros)
(U)         BRANCH crates/runner/src/expression_validation.rs:520:16 True  [LS2]  || trade.exit_bar < trade.entry_bar
(U)         BRANCH crates/runner/src/expression_validation.rs:521:16 True  [LS2]  || previous.is_some_and(|index| trade.entry_bar <= index)
(U)         LINE   crates/runner/src/expression_validation.rs:523  [LS2]  return Err("fixed-training trade crosses source/fold/session boundaries".into());
(U)         BRANCH crates/runner/src/grid.rs:1733:17 True  [LS2]  let at = if at > last { last } else { at };
(U)         LINE   crates/runner/src/grid.rs:2095  [LS2]  )?;
(U)         LINE   crates/runner/src/grid.rs:2673  [LS2]  )?;
(U)         LINE   crates/runner/src/grid.rs:2700  [LS2]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("enumerated grid cell count")
(U)         LINE   crates/runner/src/grid.rs:2701  [LS2]  })?;
(U)         LINE   crates/runner/src/grid.rs:2722  [LS2]  crate::exit_grid_policy::ExitGridErrorV1::ArithmeticOverflow("resolved grid allocation")
(U)         LINE   crates/runner/src/grid.rs:2723  [LS2]  })?;
(U)         LINE   crates/runner/src/grid.rs:3088  [LS2]  )?;
(U)         LINE   crates/runner/src/grid.rs:3376  [LS2]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3377  [LS2]  "candidate count does not fit the V1 receipt",
(U)         LINE   crates/runner/src/grid.rs:3378  [LS2]  )
(U)         LINE   crates/runner/src/grid.rs:3379  [LS2]  })?;
(U)         LINE   crates/runner/src/grid.rs:3435  [LS2 line 3433]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3436  [LS2 line 3434]  "candidate count does not fit the V1 receipt",
(U)         LINE   crates/runner/src/grid.rs:3437  [LS2 line 3435]  )
(U)         LINE   crates/runner/src/grid.rs:3438  [LS2 line 3436]  })?;
(U)         LINE   crates/runner/src/grid.rs:3488  [LS2 line 3478]  )?;
(U)         BRANCH crates/runner/src/grid.rs:3623:32 True  [LS2 line 3613]  if measured.trades != 1 || one_row.len() != 1 {
(U)         BRANCH crates/runner/src/grid.rs:3630:8 True  [LS2 line 3620]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
(U)         BRANCH crates/runner/src/grid.rs:3630:39 True  [LS2 line 3620]  if measured.ambiguous_bars > 1 || measured.gapped > 1 {
(U)         LINE   crates/runner/src/grid.rs:3631  [LS2 line 3621]  return Err(
(U)         LINE   crates/runner/src/grid.rs:3632  [LS2 line 3622]  crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(
(U)         LINE   crates/runner/src/grid.rs:3633  [LS2 line 3623]  "one candidate produced impossible multi-trade quality counts",
(U)         LINE   crates/runner/src/grid.rs:3634  [LS2 line 3624]  ),
(U)         LINE   crates/runner/src/grid.rs:3635  [LS2 line 3625]  );
(U)         LINE   crates/runner/src/grid.rs:3641  [LS2 line 3631]  )?;
(U)         BRANCH crates/runner/src/grid.rs:3647:8 True  [LS2 line 3637]  if row.signal_bar != candidate.signal
(U)         BRANCH crates/runner/src/grid.rs:3648:12 True  [LS2 line 3638]  || row.entry_bar != candidate.entry
(U)         BRANCH crates/runner/src/grid.rs:3650:12 True  [LS2 line 3640]  || row.exit_micros != stamp
(U)         BRANCH crates/runner/src/grid.rs:3651:12 True  [LS2 line 3641]  || row.exit_bar < candidate.entry
(U)         BRANCH crates/runner/src/grid.rs:4758:12 False  [LS2 line 4748]  if pess > 0 && (cell.min_win == 0 || pess < cell.min_win) {
(F)         BRANCH crates/runner/src/grid.rs:6264:13 False  [LS2 line 6254]  !replay.is_empty() && !replay.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6264:35 False  [LS2 line 6254]  !replay.is_empty() && !replay.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6285:13 False  [LS2 line 6275]  !in_loop.is_empty() && !in_loop.contains(built),
(F)         BRANCH crates/runner/src/grid.rs:6285:36 False  [LS2 line 6275]  !in_loop.is_empty() && !in_loop.contains(built),
(F)         LINE   crates/runner/src/grid.rs:7098  [LS2 line 7088]  stopped.edge_ratio(),
(F)         LINE   crates/runner/src/grid.rs:7099  [LS2 line 7089]  stopless.edge_ratio()
(F)         BRANCH crates/runner/src/grid.rs:7992:13 False  [LS2 line 7982]  !long.cells.is_empty() && !short.cells.is_empty(),
(F)         BRANCH crates/runner/src/grid.rs:7992:39 False  [LS2 line 7982]  !long.cells.is_empty() && !short.cells.is_empty(),
(F)         LINE   crates/runner/src/grid.rs:8110  [LS2 line 8100]  !matches!(
(F)         BRANCH crates/runner/src/grid.rs:8204:17 False  [LS2 line 8194]  assert!(base.stop.is_none() && base.target.is_none());
(F)         BRANCH crates/runner/src/grid.rs:8204:40 False  [LS2 line 8194]  assert!(base.stop.is_none() && base.target.is_none());
(F)         LINE   crates/runner/src/grid.rs:8390  [LS2 line 8380]  c.uncertainty()
(F)         LINE   crates/runner/src/grid.rs:9824  [LS2 line 9814]  a.reward_to_risk_bp()
(F)         LINE   crates/runner/src/grid.rs:10293  [LS2 line 10283]  sniper.assurance_bp(),
(F)         LINE   crates/runner/src/grid.rs:10294  [LS2 line 10284]  grinder.assurance_bp()
(F)         LINE   crates/runner/src/grid.rs:10300  [LS2 line 10290]  grinder.assurance_bp(),
(F)         LINE   crates/runner/src/grid.rs:10301  [LS2 line 10291]  fluke.assurance_bp()
(F)         LINE   crates/runner/src/grid.rs:10340  [LS2 line 10330]  cell.win_rate_bp()
(F)         LINE   crates/runner/src/grid.rs:10374  [LS2 line 10364]  trades - 1
(F)         LINE   crates/runner/src/grid.rs:11044  [LS2 line 11034]  t.eligible.len(),
(F)         LINE   crates/runner/src/grid.rs:11045  [LS2 line 11035]  t.trades.len()
(F)         BRANCH crates/runner/src/grid_cover_tests.rs:59:13 False  [LS2]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F)         BRANCH crates/runner/src/grid_cover_tests.rs:59:27 False  [LS2]  assert!(needed > 1 && needed < 10_000, "{needed}");
(F)         LINE   crates/runner/src/lib.rs:2034  [LS2 line 2028]  reads.len()
(F)         BRANCH crates/runner/src/lib.rs:2038:33 False  [LS2 line 2032]  |&(name, text)| name == "validate.rs" && text.contains("BRUTEX_GRID_RUNGS")
(U)         BRANCH crates/runner/src/outcome.rs:812:19 False  [LS2]  while let Some(below) = levels.last() {
(U)         BRANCH crates/runner/src/outcome.rs:962:13 False  [LS2]  let Some(start) = bars.get(i).map(|bar| bar.ts_micros) else {
(U)         LINE   crates/runner/src/outcome.rs:963  [LS2]  lanes.absent(Unpriced::Refused);
(U)         LINE   crates/runner/src/outcome.rs:964  [LS2]  continue;
(U)         BRANCH crates/runner/src/outcome.rs:1050:13 False  [LS2]  let Some((later, now)) = bars
(U)         LINE   crates/runner/src/outcome.rs:1055  [LS2]  lanes.absent(Unpriced::Refused);
(U)         LINE   crates/runner/src/outcome.rs:1056  [LS2]  continue;
(U)         BRANCH crates/runner/src/outcome.rs:1170:28 False  [LS2]  if let Some(Reverse(moved)) = upper.pop() {
(U)         BRANCH crates/runner/src/outcome.rs:1174:28 False  [LS2]  && let Some(moved) = lower.pop()
(U)         BRANCH crates/runner/src/outcome.rs:1193:20 False  [LS2]  && let Some(slot) = out.get_mut(index)
(U)         BRANCH crates/runner/src/outcome.rs:2086:24 False  [LS2]  if let Some(freed) = self.slab.get_mut(at) {
(U)         BRANCH crates/runner/src/outcome.rs:2091:24 False  [LS2]  if let Some(staying) = self.slab.get_mut(at) {
(U)         BRANCH crates/runner/src/outcome.rs:2098:16 False  [LS2]  if let Some(head) = self.wheel.get_mut(slot) {
(U)         BRANCH crates/runner/src/outcome.rs:2116:16 False  [LS2]  if let Some(head) = self.wheel.get_mut(slot) {
(U)         BRANCH crates/runner/src/outcome.rs:2274:23 True  [LS2]  if !paired || !forward.covers(source) {
(F)         LINE   crates/runner/src/outcome.rs:3153  [LS2]  column.bits().len()
(F)         LINE   crates/runner/src/outcome.rs:3243  [LS2]  minute_of(i)
(F)         LINE   crates/runner/src/outcome.rs:3318  [LS2]  bars.len()
(F)         LINE   crates/runner/src/outcome.rs:3383  [LS2]  forced_minute - 1
(F)         LINE   crates/runner/src/outcome.rs:3413  [LS2]  forced_minute - 1,
(F)         LINE   crates/runner/src/outcome.rs:3414  [LS2]  forced_minute - 2
(F)         BRANCH crates/runner/src/outcome.rs:3666:13 False  [LS2]  session.fillable(5) && !session.fillable(6),
(F)         BRANCH crates/runner/src/outcome.rs:3666:36 False  [LS2]  session.fillable(5) && !session.fillable(6),
(F)         BRANCH crates/runner/src/outcome.rs:3765:13 False  [LS2]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F)         BRANCH crates/runner/src/outcome.rs:3765:40 False  [LS2]  exact_mean(-1, 3) < 0.0 && exact_mean(1, 3) > 0.0,
(F)         BRANCH crates/runner/src/outcome.rs:4147:13 False  [LS2]  e.t != 0.0 && e.t.is_finite(),
(F)         BRANCH crates/runner/src/outcome.rs:4147:27 False  [LS2]  e.t != 0.0 && e.t.is_finite(),
(F)         BRANCH crates/runner/src/outcome.rs:4587:13 False  [LS2]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F)         BRANCH crates/runner/src/outcome.rs:4587:39 False  [LS2]  jump.highs.len() <= 11 && jump.lows.len() <= 11,
(F)         LINE   crates/runner/src/outcome.rs:4631  [LS2]  i + 1
(F)         BRANCH crates/runner/src/outcome.rs:4899:17 False  [LS2]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F)         BRANCH crates/runner/src/outcome.rs:4899:36 False  [LS2]  e.t.is_finite() && (e.t - want).abs() <= 1e-6 * want.abs().max(1.0),
(F)         BRANCH crates/runner/src/outcome.rs:4937:17 False  [LS2]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         BRANCH crates/runner/src/outcome.rs:4937:30 False  [LS2]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         BRANCH crates/runner/src/outcome.rs:4937:50 False  [LS2]  assert!(e1.n >= 2 && e1.t.is_finite() && e1.t != 0.0, "t={}", e1.t);
(F)         LINE   crates/runner/src/outcome.rs:5295  [LS2]  want[0]
(F)         LINE   crates/runner/src/outcome.rs:5300  [LS2]  want[1]
(F)         LINE   crates/runner/src/outcome.rs:5305  [LS2]  want[2]
(F)         BRANCH crates/runner/src/outcome.rs:5322:17 False  [LS2]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5322:34 False  [LS2]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5322:51 False  [LS2]  assert!(a.is_finite() && b.is_finite() && c.is_finite());
(F)         BRANCH crates/runner/src/outcome.rs:5450:17 False  [LS2]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F)         BRANCH crates/runner/src/outcome.rs:5450:46 False  [LS2]  assert!(f.was_refused(victim - 2) && !f.was_missing(victim - 2));
(F)         BRANCH crates/runner/src/outcome.rs:5477:13 False  [LS2]  !f.was_refused(last) && !f.was_missing(last),
(F)         BRANCH crates/runner/src/outcome.rs:5477:37 False  [LS2]  !f.was_refused(last) && !f.was_missing(last),
(F)         BRANCH crates/runner/src/outcome.rs:5480:17 False  [LS2]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F)         BRANCH crates/runner/src/outcome.rs:5480:47 False  [LS2]  assert!(!f.was_refused(bars.len()) && !f.was_missing(bars.len()));
(F)         BRANCH crates/runner/src/outcome.rs:5536:13 False  [LS2]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F)         BRANCH crates/runner/src/outcome.rs:5536:39 False  [LS2]  !bounds.day_ended(500) && !bounds.day_ended(501),
(F)         BRANCH crates/runner/src/outcome.rs:5539:17 False  [LS2]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F)         BRANCH crates/runner/src/outcome.rs:5539:42 False  [LS2]  assert!(bounds.day_ended(499) && bounds.day_ended(502));
(F)         BRANCH crates/runner/src/portfolio.rs:915:18 False  [LS2]  } if strategy_digest == StrategyDigest::new([7; 32])
(F)         BRANCH crates/runner/src/portfolio.rs:1017:18 False  [LS2]  } if strategy_digest == StrategyDigest::new([3; 32])
(F)         LINE   crates/runner/src/rank.rs:1264  [LS2]  grinder.payoff_bp(),
(F)         LINE   crates/runner/src/rank.rs:1265  [LS2]  grinder.worst_reward_risk_bp()
(F)         BRANCH crates/runner/src/report.rs:1508:13 False  [LS2]  luck > 0.0 && luck < required,
(F)         BRANCH crates/runner/src/report.rs:1508:27 False  [LS2]  luck > 0.0 && luck < required,
(F)         BRANCH crates/runner/src/report.rs:1521:13 False  [LS2]  distinct > 0 && distinct < n,
(F)         BRANCH crates/runner/src/report.rs:1521:29 False  [LS2]  distinct > 0 && distinct < n,
(F)         BRANCH crates/runner/src/report_cover_tests.rs:192:13 False  [LS2]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F)         BRANCH crates/runner/src/report_cover_tests.rs:192:45 False  [LS2]  assert!(!clean.contains("MISPAIRED") && !clean.contains("REFUSED EXITS"));
(F)         BRANCH crates/runner/src/report_cover_tests.rs:276:13 False  [LS2]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F)         BRANCH crates/runner/src/report_cover_tests.rs:276:38 False  [LS2]  assert!(line.contains("9999") && line.contains(" ? "), "{line}");
(F)         BRANCH crates/runner/src/resample.rs:881:21 False  [LS2]  assert!(b.open <= b.high && b.open >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:881:41 False  [LS2]  assert!(b.open <= b.high && b.open >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:882:21 False  [LS2]  assert!(b.close <= b.high && b.close >= b.low);
(F)         BRANCH crates/runner/src/resample.rs:882:42 False  [LS2]  assert!(b.close <= b.high && b.close >= b.low);
(U)         LINE   crates/runner/src/resolved_grid_view.rs:226  [LSA]  )?;
(U)         LINE   crates/runner/src/resolved_grid_view.rs:229  [LSA]  )?;
OPERATOR    LINE   crates/runner/src/resolved_grid_view.rs:232  [LSA: L-BCL-6, operator: (S + 1)(T + 1) passes usize::MAX only with a stop and a target ladder of more than 2^32 rungs each, 32 GiB per ladder (LS2's line 228)]  )?;
OPERATOR    LINE   crates/runner/src/resolved_grid_view.rs:261  [LSA: L-BCL-6, operator: the ? after coordinate_row_width in coordinate_row_offsets refuses only a row wider than usize::MAX cells; a resolution's cell count fits u64 (checked_policy_cell_count), so only a resolution resealed over real ladders of hundreds of MiB (and a gibibyte of row offsets) reaches it (LS2's line 253)]  )?)
(U)         LINE   crates/runner/src/resolved_grid_view.rs:284  [LSA]  )?;
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:895:12 True  [LS2]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:895:51 True  [LS2]  if !facts.covers(source.series.bars()) || facts.step_micros() != MINUTE {
(U)         LINE   crates/runner/src/signal_candle_stop.rs:896  [LS2]  return Err(Error::Source);
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1281:20 True  [LS2]  if !self
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1287:41 False  [LS2]  .is_some_and(|exit| exit.real && exit.bar == index)
(U)         LINE   crates/runner/src/signal_candle_stop.rs:1289  [LS2]  return Ok(Err(Reason::ClosingMinute));
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1345:12 True  [LS2]  || source.series.feed().is_empty()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1346:12 True  [LS2]  || source.series.commit().is_empty()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1347:12 True  [LS2]  || source.series.calendar_digest() == [0; 32]
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1349:12 True  [LS2]  || column.bits().len() != column.known().len()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1350:12 True  [LS2]  || column.len() != column.sources().len()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1352:12 True  [LS2]  || column.evaluation_spec_token().is_none()
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1358:12 True  [LS2]  || column
(U)         BRANCH crates/runner/src/signal_candle_stop.rs:1367:12 True  [LS2]  if !column.accepts(index)
(U)         LINE   crates/runner/src/signal_candle_stop.rs:1509  [LS2 line 1505]  )?;
(F)         LINE   crates/runner/src/signal_candle_stop_tests.rs:74  [LS2]  _ => unreachable!(),
(F)         BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:13 False  [LS2]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F)         BRANCH crates/runner/src/signal_candle_stop_tests.rs:1112:42 False  [LS2]  assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
(F)         LINE   crates/runner/src/significance.rs:945  [LS2]  bonferroni_t(trials)
(F)         BRANCH crates/runner/src/significance.rs:1323:21 False  [LS2]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F)         BRANCH crates/runner/src/significance.rs:1323:42 False  [LS2]  assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
(F)         LINE   crates/runner/src/significance.rs:1341  [LS2]  (approx - exact).abs()
(F)         LINE   crates/runner/src/significance.rs:1357  [LS2]  bonferroni_t(316)
(F)         BRANCH crates/runner/src/significance.rs:1385:13 False  [LS2]  bar > 7.0 && bar < 9.0,
(F)         BRANCH crates/runner/src/significance.rs:1385:26 False  [LS2]  bar > 7.0 && bar < 9.0,
(F)         BRANCH crates/runner/src/significance_cover_tests.rs:63:13 False  [LS2]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F)         BRANCH crates/runner/src/significance_cover_tests.rs:63:38 False  [LS2]  assert!(converged.is_finite() && converged > 0.0, "{converged}");
(F)         LINE   crates/runner/src/split.rs:385  [LS2]  bar + horizon,
(F)         BRANCH crates/runner/src/split.rs:576:21 False  [LS2]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F)         BRANCH crates/runner/src/split.rs:576:45 False  [LS2]  assert!(f.train.0.is_empty() && f.train.1.is_empty());
(F)         BRANCH crates/runner/src/synthetic.rs:130:25 False  [LS2]  assert!(c.open <= c.high && c.open >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:130:45 False  [LS2]  assert!(c.open <= c.high && c.open >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:131:25 False  [LS2]  assert!(c.close <= c.high && c.close >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:131:46 False  [LS2]  assert!(c.close <= c.high && c.close >= c.low);
(F)         BRANCH crates/runner/src/synthetic.rs:330:17 False  [LS2]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(F)         BRANCH crates/runner/src/synthetic.rs:330:27 False  [LS2]  assert!(up > 0 && down > 0, "the series moved in one direction only");
(U)         BRANCH crates/runner/src/topn.rs:373:16 False  [LS2]  if let Some(slot) = bytes.get_mut(..16) {
(U)         BRANCH crates/runner/src/topn.rs:376:16 False  [LS2]  if let Some(slot) = bytes.get_mut(16..20) {
(U)         BRANCH crates/runner/src/topn.rs:379:16 False  [LS2]  if let Some(slot) = bytes.get_mut(20..24) {
(U)         BRANCH crates/runner/src/topn.rs:382:16 False  [LS2]  if let Some(slot) = bytes.get_mut(24..28) {
(U)         BRANCH crates/runner/src/topn.rs:389:20 False  [LS2]  if let Some(slot) = bytes.get_mut(start..end) {
(U)         BRANCH crates/runner/src/trade.rs:679:13 False  [LS2]  let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {
(U)         LINE   crates/runner/src/trade.rs:680  [LS2]  return false;
(U)         BRANCH crates/runner/src/trade.rs:682:13 False  [LS2]  let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {
(U)         LINE   crates/runner/src/trade.rs:683  [LS2]  return false;
(U)         BRANCH crates/runner/src/trade.rs:729:16 False  [LS2]  if let [before, after] = pair
(U)         BRANCH crates/runner/src/trade.rs:810:8 True  [LS2]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
(U)         BRANCH crates/runner/src/trade.rs:810:55 True  [LS2]  if column.bits().len() != column.known().len() || column.bits().len() != column.sources().len()
(U)         LINE   crates/runner/src/trade.rs:812  [LS2]  return Err("expression pricing column alignment mismatch".to_owned());
(U)         BRANCH crates/runner/src/trade.rs:1045:13 False  [LS2]  let Some(entry) = signal.checked_add(step) else {
(U)         LINE   crates/runner/src/trade.rs:1046  [LS2]  let blocked = open_until.is_some_and(|until| signal <= until);
(U)         LINE   crates/runner/src/trade.rs:1047  [LS2]  if blocked {
(U)         BRANCH crates/runner/src/trade.rs:1047:16 False  [LS2]  if blocked {
(U)         BRANCH crates/runner/src/trade.rs:1047:16 True  [LS2]  if blocked {
(U)         LINE   crates/runner/src/trade.rs:1048  [LS2]  out.while_open = out.while_open.saturating_add(1);
(U)         LINE   crates/runner/src/trade.rs:1049  [LS2]  } else {
(U)         LINE   crates/runner/src/trade.rs:1050  [LS2]  out.too_late = out.too_late.saturating_add(1);
(U)         LINE   crates/runner/src/trade.rs:1051  [LS2]  }
(U)         LINE   crates/runner/src/trade.rs:1052  [LS2]  continue;
(U)         BRANCH crates/runner/src/trade.rs:1159:16 True  [LS2]  if blocked {
(U)         LINE   crates/runner/src/trade.rs:1160  [LS2]  out.while_open = out.while_open.saturating_add(1);
(U)         BRANCH crates/runner/src/trade.rs:1352:8 True  [LS2]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
(U)         BRANCH crates/runner/src/trade.rs:1352:37 True  [LS2]  if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
(U)         LINE   crates/runner/src/trade.rs:1353  [LS2]  return None;
(F)         BRANCH crates/runner/src/trade.rs:1692:25 False  [LS2]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:1692:48 False  [LS2]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:1878:17 False  [LS2]  let [previous, next] = pair else {
(F)         LINE   crates/runner/src/trade.rs:1879  [LS2]  unreachable!("windows(2) always yields exactly two trades")
(F)         BRANCH crates/runner/src/trade.rs:2084:21 False  [LS2]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:2084:48 False  [LS2]  assert!(on_its_own.reconciles() && with_future.reconciles());
(F)         LINE   crates/runner/src/trade.rs:2234  [LS2]  t.count(),
(F)         LINE   crates/runner/src/trade.rs:2378  [LS2]  ist_minute_of(exit_ts)
(F)         BRANCH crates/runner/src/trade.rs:2590:17 False  [LS2]  assert!(saw_refused && saw_missing, "both holes were located");
(F)         BRANCH crates/runner/src/trade.rs:2590:32 False  [LS2]  assert!(saw_refused && saw_missing, "both holes were located");
(F)         BRANCH crates/runner/src/trade.rs:2704:13 False  [LS2]  !path.priceable && path.first_refused == Some(refused),
(F)         BRANCH crates/runner/src/trade.rs:2704:32 False  [LS2]  !path.priceable && path.first_refused == Some(refused),
(F)         LINE   crates/runner/src/trade.rs:3029  [LS2]  matches!(
(F)         LINE   crates/runner/src/trade.rs:3138  [LS2]  body.len()
(F)         BRANCH crates/runner/src/trade.rs:3212:13 False  [LS2]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F)         BRANCH crates/runner/src/trade.rs:3212:34 False  [LS2]  alone.fillable(0) && !alone.fillable(whole.len() - 1),
(F)         BRANCH crates/runner/src/trade.rs:3236:17 False  [LS2]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F)         BRANCH crates/runner/src/trade.rs:3236:44 False  [LS2]  assert!(!after.day_ended(9_999) && !after.fillable(9_999));
(F)         BRANCH crates/runner/src/trade.rs:3242:13 False  [LS2]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F)         BRANCH crates/runner/src/trade.rs:3242:42 False  [LS2]  !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
(F)         BRANCH crates/runner/src/trade.rs:3539:17 False  [LS2]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F)         BRANCH crates/runner/src/trade.rs:3539:41 False  [LS2]  assert!(column.accepts(exit) && column.accepts(exit + 1));
(F)         BRANCH crates/runner/src/trade.rs:3593:17 False  [LS2]  assert!(column.accepts(entry) && column.accepts(exit));
(F)         BRANCH crates/runner/src/trade.rs:3593:42 False  [LS2]  assert!(column.accepts(entry) && column.accepts(exit));
(F)         BRANCH crates/runner/src/trade.rs:3674:17 False  [LS2]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F)         BRANCH crates/runner/src/trade.rs:3674:53 False  [LS2]  assert!(column.sources().contains(&late) && column.accepts(midnight));
(F)         BRANCH crates/runner/src/trade.rs:3733:17 False  [LS2]  assert!(walked.signals > 0 && walked.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:3733:39 False  [LS2]  assert!(walked.signals > 0 && walked.reconciles());
(F)         BRANCH crates/runner/src/trade.rs:3735:17 False  [LS2]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F)         BRANCH crates/runner/src/trade.rs:3735:45 False  [LS2]  assert!(walked.trades.is_empty() && walked.eligible.is_empty());
(F)         BRANCH crates/runner/src/trade.rs:3801:17 False  [LS2]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F)         BRANCH crates/runner/src/trade.rs:3801:42 False  [LS2]  assert!(facts.accepts(at - 1) && facts.accepts(at + 1));
(F)         BRANCH crates/runner/src/trade_clock_tests.rs:38:13 False  [LS2]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(F)         BRANCH crates/runner/src/trade_clock_tests.rs:38:43 False  [LS2]  first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
(U)         LINE   crates/runner/src/validate.rs:2229  [LS2]  )?;
(U)         LINE   crates/runner/src/validate.rs:2286  [LS2]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2392:8 True  [LS2]  if folds.is_empty() || folds.len() != validated.folds.len() {
(U)         BRANCH crates/runner/src/validate.rs:2392:28 True  [LS2]  if folds.is_empty() || folds.len() != validated.folds.len() {
(U)         LINE   crates/runner/src/validate.rs:2393  [LS2]  return Err(AnchoredSearchValidationRefusalV4::Empty);
(U)         BRANCH crates/runner/src/validate.rs:2458:12 True  [LS2]  || column.bits().len() != column.sources().len()
(U)         BRANCH crates/runner/src/validate.rs:2460:12 True  [LS2]  || census.swept != stable_u64_v4(column.bits().len())?
(U)         BRANCH crates/runner/src/validate.rs:2461:12 True  [LS2]  || !census.reconciles()
(U)         BRANCH crates/runner/src/validate.rs:2462:12 True  [LS2]  || !column.acceptance_covers(signal.len())
(U)         BRANCH crates/runner/src/validate.rs:2463:12 True  [LS2]  || column.acceptance_census() != census
(U)         BRANCH crates/runner/src/validate.rs:2464:12 True  [LS2]  || column.evaluation_spec_token().is_none()
(U)         BRANCH crates/runner/src/validate.rs:2465:12 True  [LS2]  || column.collided() != 0
(U)         BRANCH crates/runner/src/validate.rs:2466:12 True  [LS2]  || column.first_swept() != column.sources().first().copied()
(U)         BRANCH crates/runner/src/validate.rs:2467:12 True  [LS2]  || column
(U)         BRANCH crates/runner/src/validate.rs:2472:12 True  [LS2]  || !column
(U)         BRANCH crates/runner/src/validate.rs:2574:12 True  [LS2]  if fold.train.0.start != 0
(U)         BRANCH crates/runner/src/validate.rs:2575:16 True  [LS2]  || !fold.train.1.is_empty()
(U)         BRANCH crates/runner/src/validate.rs:2576:16 True  [LS2]  || fold.train.0.is_empty()
(U)         LINE   crates/runner/src/validate.rs:2600  [LS2]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2745:16 True  [LS2]  if evaluated_cells != resolved.cell_count() {
(U)         LINE   crates/runner/src/validate.rs:2746  [LS2]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:2747  [LS2]  "evaluated cell population differs from its resolution",
(U)         LINE   crates/runner/src/validate.rs:2748  [LS2]  ));
(U)         LINE   crates/runner/src/validate.rs:2788  [LS2]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2808:12 True  [LS2]  if population_cells != expected_population {
(U)         LINE   crates/runner/src/validate.rs:2809  [LS2]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:2810  [LS2]  "mask-by-side population is incomplete",
(U)         LINE   crates/runner/src/validate.rs:2811  [LS2]  ));
(U)         LINE   crates/runner/src/validate.rs:2863  [LS2]  )?;
(U)         BRANCH crates/runner/src/validate.rs:2951:12 True  [LS2]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
(U)         BRANCH crates/runner/src/validate.rs:2951:55 True  [LS2]  if final_candidates.len() != pending.len() || oos_all.len() != pending.len() {
(U)         LINE   crates/runner/src/validate.rs:2952  [LS2]  return Err(AnchoredSearchValidationRefusalV4::CandidateFamilyMismatch { fold: index });
OPERATOR    BRANCH crates/runner/src/validate.rs:3070:8 True  [LS2: [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
OPERATOR    BRANCH crates/runner/src/validate.rs:3070:57 True  [LS2: [production]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {]  if long.policy_digest() == short.policy_digest() || long.digest() == short.digest() {
OPERATOR    LINE   crates/runner/src/validate.rs:3071  [LS2: [production]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
OPERATOR    LINE   crates/runner/src/validate.rs:3072  [LS2: [production]  "Long/Short policy or resolution identities are indistinguishable",]  "Long/Short policy or resolution identities are indistinguishable",
OPERATOR    LINE   crates/runner/src/validate.rs:3073  [LS2: [production]  ));]  ));
(U)         BRANCH crates/runner/src/validate.rs:3103:17 False  [LS2]  let Some(next) = ordinal.checked_add(1) else {
(U)         LINE   crates/runner/src/validate.rs:3104  [LS2]  overflowed = true;
(U)         LINE   crates/runner/src/validate.rs:3105  [LS2]  return false;
(U)         BRANCH crates/runner/src/validate.rs:3110:12 True  [LS2]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         BRANCH crates/runner/src/validate.rs:3110:26 True  [LS2]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         BRANCH crates/runner/src/validate.rs:3110:39 True  [LS2]  if overflowed || !complete || ordinal != resolved.cell_count() {
(U)         LINE   crates/runner/src/validate.rs:3111  [LS2]  return Err(AnchoredSearchValidationRefusalV4::GridPairMismatch(
(U)         LINE   crates/runner/src/validate.rs:3112  [LS2]  "canonical coordinate order is incomplete",
(U)         LINE   crates/runner/src/validate.rs:3113  [LS2]  ));
(U)         BRANCH crates/runner/src/validate.rs:3604:8 True  [LS2]  if alignment.onto.len() != signal_column.sources().len() {
(U)         LINE   crates/runner/src/validate.rs:3605  [LS2]  return Err("an OOS alignment is not parallel to its warmed signal column".to_owned());
(U)         BRANCH crates/runner/src/validate.rs:3625:8 True  [LS2]  if dropped != expected_dropped {
(U)         LINE   crates/runner/src/validate.rs:3626  [LS2]  return Err("an OOS projection did not reconcile its exact dropped-row census".to_owned());
(U)         BRANCH crates/runner/src/validate.rs:3771:13 False  [LS2]  let Some(outcome) = oos.get(ordinal) else {
(U)         LINE   crates/runner/src/validate.rs:3772  [LS2]  return Err(AnchoredAdmissionValidationRefusalV2::ProvenanceMismatch { fold });
(U)         BRANCH crates/runner/src/validate.rs:4123:9 False  [LS2]  let Ok(ordinal) = usize::try_from(choice.ordinal) else {
(U)         LINE   crates/runner/src/validate.rs:4124  [LS2]  return false;
(U)         BRANCH crates/runner/src/validate.rs:4515:13 False  [LS2]  let Some(train) = bars.get(fold.train.0.clone()) else {
(U)         LINE   crates/runner/src/validate.rs:4516  [LS2]  continue;
(U)         BRANCH crates/runner/src/validate.rs:4971:21 False  [LS2]  let Some(chosen_side) = chosen_side else {
(U)         LINE   crates/runner/src/validate.rs:4972  [LS2]  out.refused = Some(
(U)         LINE   crates/runner/src/validate.rs:4973  [LS2]  "a walk-forward fold selected a candidate without its training side"
(U)         LINE   crates/runner/src/validate.rs:4974  [LS2]  .to_owned(),
(U)         LINE   crates/runner/src/validate.rs:4975  [LS2]  );
(U)         LINE   crates/runner/src/validate.rs:4976  [LS2]  return out;
(U)         BRANCH crates/runner/src/validate.rs:5091:24 True  [LS2]  if *mask == ConditionMask::ZERO {
(U)         LINE   crates/runner/src/validate.rs:5092  [LS2]  0
(F)         LINE   crates/runner/src/validate.rs:5602  [LS2]  unreachable!("the zero-rung door must refuse before building")
(F)         LINE   crates/runner/src/validate.rs:5735  [LS2]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate.rs:5768  [LS2]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate.rs:6027  [LS2]  &|_| unreachable!("a refused walk finishes no fold, so the hook never speaks"),
(F)         LINE   crates/runner/src/validate.rs:6629  [LS2]  !matches!(
(F)         LINE   crates/runner/src/validate.rs:6797  [LS2]  f.considered.saturating_sub(f.priced)
(F)         BRANCH crates/runner/src/validate.rs:6856:13 False  [LS2]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:6856:29 False  [LS2]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:6856:50 False  [LS2]  at.is_some() && collect.is_some() && collect < at,
(F)         BRANCH crates/runner/src/validate.rs:7363:19 False  [LS2]  }) if refused_fold == fold
(F)         BRANCH crates/runner/src/validate.rs:7827:20 False  [LS2]  if fold == fold_index
(F)         BRANCH crates/runner/src/validate.rs:7857:24 False  [LS2]  if fold == fold_index
(F)         BRANCH crates/runner/src/validate.rs:7920:17 False  [LS2]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F)         BRANCH crates/runner/src/validate.rs:7920:53 False  [LS2]  assert!(tail.contains("walk_over_from(") && tail.contains("&test_facts,"));
(F)         BRANCH crates/runner/src/validate.rs:7921:17 False  [LS2]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F)         BRANCH crates/runner/src/validate.rs:7921:54 False  [LS2]  assert!(!tail.contains("SliceFacts::of(") && !tail.contains("&walk("));
(F)         LINE   crates/runner/src/validate.rs:7941  [LS2]  body.len()
(F)         LINE   crates/runner/src/validate.rs:8000  [LS2]  body.len()
(F)         BRANCH crates/runner/src/validate.rs:8008:13 False  [LS2]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F)         BRANCH crates/runner/src/validate.rs:8008:45 False  [LS2]  body.contains(".par_iter()") && body.contains("evaluate_one(item.mask"),
(F)         BRANCH crates/runner/src/validate.rs:8031:13 False  [LS2]  !inner.contains("ExecutionRunV1::new(")
(F)         BRANCH crates/runner/src/validate.rs:8032:20 False  [LS2]  && !inner.contains("ExecutionDigestsV1::of(")
(F)         BRANCH crates/runner/src/validate.rs:8033:20 False  [LS2]  && inner.contains("ExecutionRunV1::with_digests(&run, &train_digests)"),
(F)         BRANCH crates/runner/src/validate.rs:8059:17 False  [LS2]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F)         BRANCH crates/runner/src/validate.rs:8059:60 False  [LS2]  assert!(pass.contains("with_levels_over_from(") && pass.contains("walk_over_from("));
(F)         BRANCH crates/runner/src/validate.rs:8061:17 False  [LS2]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F)         BRANCH crates/runner/src/validate.rs:8061:56 False  [LS2]  assert!(!pass.contains("with_levels_over(") && !pass.contains("walk_over("));
(F)         BRANCH crates/runner/src/validate.rs:8097:13 False  [LS2]  !inner.contains("data_digest_with_execution(")
(F)         BRANCH crates/runner/src/validate.rs:8098:20 False  [LS2]  && !inner.contains("ExecutionRunV1::new("),
(F)         BRANCH crates/runner/src/validate.rs:8133:13 False  [LS2]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F)         BRANCH crates/runner/src/validate.rs:8133:58 False  [LS2]  hoisted.contains("long.attest_training(") && hoisted.contains("short.attest_training("),
(F)         BRANCH crates/runner/src/validate.rs:8137:13 False  [LS2]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F)         BRANCH crates/runner/src/validate.rs:8137:46 False  [LS2]  inner.contains(".par_iter()") && inner.contains("evaluate_with_attested("),
(F)         BRANCH crates/runner/src/validate.rs:8141:13 False  [LS2]  !inner.contains("attest_training(")
(F)         BRANCH crates/runner/src/validate.rs:8142:20 False  [LS2]  && !inner.contains("evaluate_training_grid_attested("),
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1459  [LS2]  unreachable!("no fold is built for a refused policy or execution path")
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1869  [LS2]  String::from_utf8_lossy(&child.stdout)
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1894  [LS2]  &|_| unreachable!("a refused walk reports no fold"),
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:1920:13 False  [LS2]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:1920:43 False  [LS2]  assert!(windows[1].test.is_empty() && windows[1].test.start == bars.len());
(F)         LINE   crates/runner/src/validate_cover_tests.rs:1960  [LS2]  &|_| unreachable!("a refused walk reports no fold"),
(F)         LINE   crates/runner/src/validate_cover_tests.rs:2156  [LS2]  &|_| unreachable!("a refused walk reports no fold"),
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2317:13 False  [LS2]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2317:45 False  [LS2]  assert!(swept.sweep.halted.is_none() && !swept.is_complete());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2368:13 False  [LS2]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2368:43 False  [LS2]  assert!(fold.candidates.is_empty() && fold.choice.is_none());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2371:13 False  [LS2]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2371:49 False  [LS2]  assert!(visible.in_sample_all.is_empty() && visible.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2401:19 False  [LS2]  )) if ambiguous_bars > 0
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2503:17 False  [LS2]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/src/validate_cover_tests.rs:2503:50 False  [LS2]  assert!(fold.in_sample_all.is_empty() && fold.out_of_sample_all.is_empty());
(F)         BRANCH crates/runner/tests/base_cov_admission_words.rs:66:69 False  [LS2]  name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
(F)         BRANCH crates/runner/tests/end_to_end.rs:211:9 False  [LS2]  rc.is_some() && spa.is_some(),
(F)         BRANCH crates/runner/tests/end_to_end.rs:211:25 False  [LS2]  rc.is_some() && spa.is_some(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:17 False  [LS2]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:132:40 False  [LS2]  assert!(!bounds.fillable(0) && !bounds.fillable(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:17 False  [LS2]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:133:41 False  [LS2]  assert!(!bounds.day_ended(0) && !bounds.day_ended(1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:17 False  [LS2]  trades.count() > 0 && trades.reconciles(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:154:39 False  [LS2]  trades.count() > 0 && trades.reconciles(),
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:13 False  [LS2]  !column.is_empty() && column.census().refused() == 0,
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:172:35 False  [LS2]  !column.is_empty() && column.census().refused() == 0,
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:25 False  [LS2]  assert!(trades.signals > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:200:47 False  [LS2]  assert!(trades.signals > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:202:21 False  [LS2]  trades.trades.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:203:28 False  [LS2]  && trades.eligible.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:204:28 False  [LS2]  && trades.occupancy.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:13 False  [LS2]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:221:42 False  [LS2]  assert!(facts.accepts(chosen - 1) && facts.accepts(chosen + 1));
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:13 False  [LS2]  assert!(trades.count() > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:231:35 False  [LS2]  assert!(trades.count() > 0 && trades.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:17 False  [LS2]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:257:41 False  [LS2]  assert!(ordinary.count() > 0 && ordinary.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:21 False  [LS2]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:289:44 False  [LS2]  assert!(refused.signals > 0 && refused.reconciles());
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:291:17 False  [LS2]  refused.trades.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:292:24 False  [LS2]  && refused.eligible.is_empty()
(F)         BRANCH crates/runner/tests/fixed_deadline_readiness.rs:293:24 False  [LS2]  && refused.occupancy.is_empty()
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:21 False  [LS2]  assert!(control.count() > 0 && control.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:89:44 False  [LS2]  assert!(control.count() > 0 && control.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:21 False  [LS2]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:112:41 False  [LS2]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:21 False  [LS2]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:117:49 False  [LS2]  assert!(mask.eligible.is_empty() && mask.occupancy.is_empty());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:25 False  [LS2]  assert!(mask.count() > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:179:45 False  [LS2]  assert!(mask.count() > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:21 False  [LS2]  assert!(mask.signals > 0 && mask.reconciles());
(F)         BRANCH crates/runner/tests/foreign_slice_deadline_readiness.rs:250:41 False  [LS2]  assert!(mask.signals > 0 && mask.reconciles());
(F)         LINE   crates/runner/tests/hole_after_exit.rs:129  [LS2]  probes.len()
(F)         LINE   crates/runner/tests/join_answer_is_unchanged.rs:140  [LS2]  level.frequent.len(),
(F)         LINE   crates/runner/tests/unwired_validation_record.rs:129  [LS2]  path.display()
(F)         BRANCH crates/store/src/catalog_tests.rs:267:9 False  [LS1]  let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
(F)         LINE   crates/store/src/catalog_tests.rs:268  [LS1]  panic!("a dangling `bars` link is refused");
(F)         BRANCH crates/store/src/catalog_tests.rs:281:9 False  [LS1]  let Err(CatalogError::BarsUnreadable { because }) = locked else {
(F)         LINE   crates/store/src/catalog_tests.rs:282  [LS1]  panic!("an unsearchable root is refused, not empty: {locked:?}");
(F)         BRANCH crates/store/src/catalog_tests.rs:314:13 False  [LS1]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:314:33 False  [LS1]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:13 False  [LS1]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:32 False  [LS1]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:320:50 False  [LS1]  assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
(F)         BRANCH crates/store/src/catalog_tests.rs:336:13 False  [LS1]  assert!(stack.is_empty() && out.held.is_empty());
(F)         BRANCH crates/store/src/catalog_tests.rs:336:33 False  [LS1]  assert!(stack.is_empty() && out.held.is_empty());
(F)         LINE   crates/store/src/catalog_tests.rs:460  [LS1]  path.display()
(U)         BRANCH crates/store/src/checksum_audit.rs:288:12 True  [LS1]  || layout.header_len() != HEADER_BYTES as u64
(F) bound   BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True  [LS1]  if started.elapsed() > std::time::Duration::from_secs(2) {
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:153  [LS1]  child.kill().expect("bounded timeout");
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:154  [LS1]  child.wait().expect("reap child");
(F) bound   LINE   crates/store/src/checksum_audit_tests.rs:155  [LS1]  return Err("strict checksum source open waited for a FIFO peer".into());
(F) guard   BRANCH crates/store/src/emits.rs:311:12 False  [LS1]  if let Some(byte) = region.get_mut(16_384 + 56) {
(F)         LINE   crates/store/src/emits.rs:835  [LS1]  found
(F)         LINE   crates/store/src/emits.rs:836  [LS1]  .records
(F)         LINE   crates/store/src/emits.rs:837  [LS1]  .iter()
(F)         LINE   crates/store/src/emits.rs:838  [LS1]  .map(|line| (line.target.clone(), line.message.clone()))
(F)         LINE   crates/store/src/emits.rs:839  [LS1]  .collect::<Vec<_>>()
(F)         LINE   crates/store/src/emits.rs:848  [LS1]  panic!(
(U)         LINE   crates/store/src/file.rs:2891  [LS1]  )?;
(U)         LINE   crates/store/src/file.rs:3172  [LS1]  return Err(StoreError::Format {
(U)         LINE   crates/store/src/file.rs:3173  [LS1]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file.rs:3174  [LS1]  source,
(U)         LINE   crates/store/src/file.rs:3175  [LS1]  });
(U)         BRANCH crates/store/src/file.rs:3855:21 False  [LS1]  let Some(tix) = self.tix.as_ref() else {
(U)         LINE   crates/store/src/file.rs:3856  [LS1]  return Ok(None);
(U)         BRANCH crates/store/src/file.rs:3869:13 False  [LS1]  let Some(tix) = self.tix.as_ref() else {
(U)         LINE   crates/store/src/file.rs:3870  [LS1]  return Ok(None);
(U)         LINE   crates/store/src/file.rs:3891  [LS1]  Err(other) => Err(index_refused(&tix.path, &other)),
(U)         LINE   crates/store/src/file.rs:4013  [LS1]  Err(other) => return Err(index_refused(&tix.path, &other)),
(U)         BRANCH crates/store/src/file.rs:4189:36 True  [LS1]  if cache.block != block || cache.n_valid != n_valid {
(U)         LINE   crates/store/src/file.rs:4316  [LS1]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file.rs:4317  [LS1]  source: FormatError::OffsetOverflow,
(U)         LINE   crates/store/src/file.rs:4318  [LS1]  })?;
(U)         BRANCH crates/store/src/file.rs:5666:42 True  [LS1]  BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
(F)         LINE   crates/store/src/file.rs:6858  [LS1]  parent.display()
(F)         BRANCH crates/store/src/file.rs:7005:82 False  [LS1]  matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
(F)         LINE   crates/store/src/file.rs:7062  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:7082  [LS1]  level.display()
(F)         LINE   crates/store/src/file.rs:7341  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:7395  [LS1]  other => panic!("the flush was refused by path: {other:?}"),
(F)         LINE   crates/store/src/file.rs:7715  [LS1]  matches!(
IGNORED     LINE   crates/store/src/file.rs:7961  [LS1: : fn the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name() {]  fn the_kernels_own_enospc_on_a_full_volume_is_disk_full_by_name() {
IGNORED     LINE   crates/store/src/file.rs:7962  [LS1: : let dir = std::env::var_os("BRUTEX_FULL_VOLUME")]  let dir = std::env::var_os("BRUTEX_FULL_VOLUME")
IGNORED     LINE   crates/store/src/file.rs:7963  [LS1: : .map(PathBuf::from)]  .map(PathBuf::from)
IGNORED     LINE   crates/store/src/file.rs:7964  [LS1: : .expect("BRUTEX_FULL_VOLUME names a directory on a small volume");]  .expect("BRUTEX_FULL_VOLUME names a directory on a small volume");
IGNORED     LINE   crates/store/src/file.rs:7965  [LS1: : let at = dir.join("brutex-enospc-probe.bin");]  let at = dir.join("brutex-enospc-probe.bin");
IGNORED     LINE   crates/store/src/file.rs:7966  [LS1: : let file = std::fs::OpenOptions::new()]  let file = std::fs::OpenOptions::new()
IGNORED     LINE   crates/store/src/file.rs:7967  [LS1: : .write(true)]  .write(true)
IGNORED     LINE   crates/store/src/file.rs:7968  [LS1: : .create_new(true)]  .create_new(true)
IGNORED     LINE   crates/store/src/file.rs:7969  [LS1: : .open(&at)]  .open(&at)
IGNORED     LINE   crates/store/src/file.rs:7970  [LS1: : .expect("a new file on the small volume");]  .expect("a new file on the small volume");
IGNORED     LINE   crates/store/src/file.rs:7971  [LS1: : let chunk = vec![0xAB_u8; 1 << 16];]  let chunk = vec![0xAB_u8; 1 << 16];
IGNORED     LINE   crates/store/src/file.rs:7972  [LS1: : let mut offset = 0_u64;]  let mut offset = 0_u64;
IGNORED     LINE   crates/store/src/file.rs:7973  [LS1: : let refused = loop {]  let refused = loop {
IGNORED     LINE   crates/store/src/file.rs:7974  [LS1: : assert!(offset < 1 << 30, "the volume took 1 GiB without refusing");]  assert!(offset < 1 << 30, "the volume took 1 GiB without refusing");
IGNORED     LINE   crates/store/src/file.rs:7975  [LS1: : match write_fully(&file, &at, offset, &chunk) {]  match write_fully(&file, &at, offset, &chunk) {
IGNORED     LINE   crates/store/src/file.rs:7976  [LS1: : Ok(()) => offset = offset.saturating_add(1 << 16),]  Ok(()) => offset = offset.saturating_add(1 << 16),
IGNORED     LINE   crates/store/src/file.rs:7977  [LS1: : Err(refused) => break refused,]  Err(refused) => break refused,
IGNORED     LINE   crates/store/src/file.rs:7980  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/src/file.rs:7982  [LS1: : StoreError::DiskFull {]  StoreError::DiskFull {
IGNORED     LINE   crates/store/src/file.rs:7983  [LS1: : path: at.clone(),]  path: at.clone(),
IGNORED     LINE   crates/store/src/file.rs:7984  [LS1: : action: Action::Write,]  action: Action::Write,
IGNORED     LINE   crates/store/src/file.rs:7985  [LS1: : },]  },
IGNORED     LINE   crates/store/src/file.rs:7989  [LS1: : eprintln!("ENOSPC: DiskFull by name after {offset} bytes accepted");]  eprintln!("ENOSPC: DiskFull by name after {offset} bytes accepted");
IGNORED     LINE   crates/store/src/file.rs:7990  [LS1: : }]  }
(F)         LINE   crates/store/src/file.rs:8479  [LS1]  reads.get()
(F) guard   BRANCH crates/store/src/file.rs:8801:16 True  [LS1]  if &slot[..7] != b"BRUTEXB" {
(F) guard   LINE   crates/store/src/file.rs:8802  [LS1]  continue;
(F)         LINE   crates/store/src/file.rs:8818  [LS1]  other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
(F)         LINE   crates/store/src/file.rs:9036  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:9045  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:9055  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:9157  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:9200  [LS1]  matches!(
(F) guard   BRANCH crates/store/src/file.rs:9217:16 False  [LS1]  if let Some(head) = cache.bytes.get_mut(..8) {
(F)         BRANCH crates/store/src/file.rs:9229:13 False  [LS1]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         BRANCH crates/store/src/file.rs:9229:45 False  [LS1]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         BRANCH crates/store/src/file.rs:9229:73 False  [LS1]  shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
(F)         LINE   crates/store/src/file.rs:9406  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:10351  [LS1]  matches!(
(F)         LINE   crates/store/src/file.rs:10477  [LS1]  matches!(
(F)         BRANCH crates/store/src/file.rs:10853:24 False  [LS1]  if *ts_micros == bar(10).ts_micros
(F)         BRANCH crates/store/src/file_cover_tests.rs:91:9 False  [LS1]  follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
(F)         BRANCH crates/store/src/file_cover_tests.rs:91:65 False  [LS1]  follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
(F)         BRANCH crates/store/src/file_cover_tests.rs:118:9 False  [LS1]  whole.starts_with(&last.took) && last.took.len() < whole.len(),
(F)         BRANCH crates/store/src/file_cover_tests.rs:118:42 False  [LS1]  whole.starts_with(&last.took) && last.took.len() < whole.len(),
(F)         BRANCH crates/store/src/file_fault_tests.rs:315:57 False  [LS1]  Err(StoreError::Denied { path, action }) if fired => {
(F)         LINE   crates/store/src/file_fault_tests.rs:320  [LS1]  other => panic!("read {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:334:9 False  [LS1]  routed >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:334:24 False  [LS1]  routed >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:650:21 False  [LS1]  reason.starts_with(&format!("{}: ", srt.display()))
(F)         BRANCH crates/store/src/file_fault_tests.rs:651:28 False  [LS1]  && reason.contains("refused by the file test seam"),
(F)         LINE   crates/store/src/file_fault_tests.rs:657  [LS1]  Err(other) => panic!("read {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:661:9 False  [LS1]  built >= 1 && refused >= 1,
(F)         BRANCH crates/store/src/file_fault_tests.rs:661:23 False  [LS1]  built >= 1 && refused >= 1,
(F)         LINE   crates/store/src/file_fault_tests.rs:696  [LS1]  other => panic!("{}: {other:?}", looked.display()),
(F)         BRANCH crates/store/src/file_fault_tests.rs:755:71 False  [LS1]  matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
(F)         BRANCH crates/store/src/file_fault_tests.rs:845:61 False  [LS1]  Err(StoreError::Denied { path, action }) if fired => {
(F)         LINE   crates/store/src/file_fault_tests.rs:869  [LS1]  other => panic!("{kind:?} measure {skip}: {other:?}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:879:26 False  [LS1]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:879:42 False  [LS1]  _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
(F)         BRANCH crates/store/src/file_fault_tests.rs:912:13 False  [LS1]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:912:54 False  [LS1]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:949:13 False  [LS1]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         BRANCH crates/store/src/file_fault_tests.rs:949:54 False  [LS1]  assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
(F)         LINE   crates/store/src/file_index_cover_tests.rs:158  [LS1]  matches!(
(F)         BRANCH crates/store/src/file_index_cover_tests.rs:574:9 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/file_index_cover_tests.rs:574:36 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/file_index_cover_tests.rs:639  [LS1]  other => panic!("the leaf is refused by the last step: {other:?}"),
(U)         BRANCH crates/store/src/file_sealed.rs:135:12 True  [LS1]  if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
(U)         LINE   crates/store/src/file_sealed.rs:136  [LS1]  return Err(StoreError::SealedRefused {
(U)         LINE   crates/store/src/file_sealed.rs:137  [LS1]  path: at,
(U)         LINE   crates/store/src/file_sealed.rs:138  [LS1]  reason: "block_rows",
(U)         LINE   crates/store/src/file_sealed.rs:139  [LS1]  detail: format!(
(U)         LINE   crates/store/src/file_sealed.rs:140  [LS1]  "the head declares {} rows a frame and a handle caches at most {}",
(U)         LINE   crates/store/src/file_sealed.rs:141  [LS1]  head.block_rows,
(U)         LINE   crates/store/src/file_sealed.rs:142  [LS1]  CACHED_BLOCK_LEN / RECORD_LEN
(U)         LINE   crates/store/src/file_sealed.rs:143  [LS1]  ),
(U)         LINE   crates/store/src/file_sealed.rs:144  [LS1]  });
(U)         BRANCH crates/store/src/file_sealed.rs:202:36 True  [LS1]  if cache.block != block || cache.n_valid != n_valid {
(U)         LINE   crates/store/src/file_sealed.rs:231  [LS1]  )?;
(U)         LINE   crates/store/src/file_sealed.rs:236  [LS1]  path: self.bars_path.clone(),
(U)         LINE   crates/store/src/file_sealed.rs:237  [LS1]  source: FormatError::OffsetOverflow,
(U)         LINE   crates/store/src/file_sealed.rs:238  [LS1]  })?;
(U)         BRANCH crates/store/src/file_sealed.rs:341:20 False  [LS1]  if let Some(count) = first.get_mut(at + 1) {
(U)         BRANCH crates/store/src/file_sealed.rs:347:20 False  [LS1]  if let Some(start) = first.get_mut(at) {
(U)         BRANCH crates/store/src/file_sealed.rs:354:20 False  [LS1]  if let Some(slot) = next_slot.get_mut(at) {
(U)         BRANCH crates/store/src/file_sealed.rs:355:24 False  [LS1]  if let Some(place) = placed.get_mut(*slot) {
(U)         LINE   crates/store/src/file_sealed.rs:359  [LS1]  }
(U)         BRANCH crates/store/src/file_sealed.rs:371:21 False  [LS1]  let Some(&row) = rows.get(asked) else {
(U)         LINE   crates/store/src/file_sealed.rs:372  [LS1]  continue;
(U)         BRANCH crates/store/src/file_sealed.rs:379:24 False  [LS1]  if let Some(slot) = out.get_mut(asked) {
(U)         BRANCH crates/store/src/file_sealed.rs:417:9 False  [LS1]  let Some(sealed) = file.sealed.as_ref() else {
(U)         LINE   crates/store/src/file_sealed.rs:418  [LS1]  return Ok(None);
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:69:9 False  [LS1]  fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:69:51 False  [LS1]  fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:76:9 False  [LS1]  twice.contains("rows 900 against 900")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:77:16 False  [LS1]  && twice.contains("generation 2 against 1")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:78:16 False  [LS1]  && twice.contains("records BLAKE3 equal: true"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:92:9 False  [LS1]  shifted.contains("rows 900 against 900")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:93:16 False  [LS1]  && shifted.contains("generation 1 against 1")
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:94:16 False  [LS1]  && shifted.contains("records BLAKE3 equal: false"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:102:9 False  [LS1]  late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:102:50 False  [LS1]  late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:168:13 False  [LS1]  refused.contains("other.sbar") && refused.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_cover_tests.rs:168:47 False  [LS1]  refused.contains("other.sbar") && refused.contains("disagree"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:214:69 False  [LS1]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F)         BRANCH crates/store/src/file_sealed_tests.rs:214:83 False  [LS1]  Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:13 False  [LS1]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:30 False  [LS1]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         BRANCH crates/store/src/file_sealed_tests.rs:315:50 False  [LS1]  assert!(!bin.exists() && retired.exists() && sbar.exists());
(F)         LINE   crates/store/src/file_sealed_tests.rs:321  [LS1]  other => panic!("expected Missing naming the .bin, got {other:?}"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:407:17 False  [LS1]  !reason.is_empty() && !detail.is_empty(),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:407:39 False  [LS1]  !reason.is_empty() && !detail.is_empty(),
(F)         LINE   crates/store/src/file_sealed_tests.rs:419  [LS1]  other => panic!("a damaged frame must be refused by name, got {other:?}"),
(F)         LINE   crates/store/src/file_sealed_tests.rs:439  [LS1]  other => panic!("a damaged head must be refused, got {other:?}"),
(F)         LINE   crates/store/src/file_sealed_tests.rs:798  [LS1]  matches!(
(F)         LINE   crates/store/src/file_sealed_tests.rs:803  [LS1]  crate::provenance::read(&root.0, path)
(F)         BRANCH crates/store/src/file_sealed_tests.rs:824:9 False  [LS1]  refused.starts_with("Err(") && refused.contains("sbar"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:824:40 False  [LS1]  refused.starts_with("Err(") && refused.contains("sbar"),
(F)         BRANCH crates/store/src/file_sealed_tests.rs:874:73 False  [LS1]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F)         BRANCH crates/store/src/file_sealed_tests.rs:874:92 False  [LS1]  Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
(F)         LINE   crates/store/src/file_sealed_tests.rs:885  [LS1]  page.len()
(F)         LINE   crates/store/src/file_sealed_tests.rs:898  [LS1]  page.len()
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:913  [LS1: : fn read_rows_at_timing() {]  fn read_rows_at_timing() {
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:914  [LS1: : let _sink_is_mine = crate::emits::hold_the_sink();]  let _sink_is_mine = crate::emits::hold_the_sink();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:915  [LS1: : for n in [2_250_u64, 22_500, 225_000] {]  for n in [2_250_u64, 22_500, 225_000] {
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:916  [LS1: : let root = Root::new(&format!("rows-at-timing-{n}"));]  let root = Root::new(&format!("rows-at-timing-{n}"));
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:917  [LS1: : let path = month_path(None);]  let path = month_path(None);
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:918  [LS1: : let bars = seconds(n);]  let bars = seconds(n);
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:919  [LS1: : write_raw(&root.0, path, &bars);]  write_raw(&root.0, path, &bars);
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:920  [LS1: : seal(&root.0, path);]  seal(&root.0, path);
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:921  [LS1: : let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();]  let sealed = BarFile::open_existing(&root.0, path, SYMBOL).unwrap();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:922  [LS1: : for width in [100_u64, 1_000] {]  for width in [100_u64, 1_000] {
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:923  [LS1: : let page: Vec<u64> = (0..width).map(|k| (k * 7_919 + 13) % n).collect();]  let page: Vec<u64> = (0..width).map(|k| (k * 7_919 + 13) % n).collect();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:924  [LS1: : let _warm = sealed.read_rows_at(&page).unwrap();]  let _warm = sealed.read_rows_at(&page).unwrap();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:925  [LS1: : let mut took = Vec::with_capacity(2_000);]  let mut took = Vec::with_capacity(2_000);
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:926  [LS1: : for _ in 0..2_000 {]  for _ in 0..2_000 {
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:927  [LS1: : let began = std::time::Instant::now();]  let began = std::time::Instant::now();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:928  [LS1: : let got = sealed.read_rows_at(&page).unwrap();]  let got = sealed.read_rows_at(&page).unwrap();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:929  [LS1: : took.push(began.elapsed());]  took.push(began.elapsed());
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:930  [LS1: : assert_eq!(got.len(), page.len());]  assert_eq!(got.len(), page.len());
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:932  [LS1: : took.sort_unstable();]  took.sort_unstable();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:933  [LS1: : let k = took.len();]  let k = took.len();
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:934  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:936  [LS1: : took[k / 2],]  took[k / 2],
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:937  [LS1: : took[k * 99 / 100],]  took[k * 99 / 100],
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:938  [LS1: : took[k - 1],]  took[k - 1],
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:939  [LS1: : took[k / 2] / u32::try_from(width).unwrap()]  took[k / 2] / u32::try_from(width).unwrap()
IGNORED     LINE   crates/store/src/file_sealed_tests.rs:943  [LS1: : }]  }
(F) skip    BRANCH crates/store/src/file_write_door_tests.rs:67:13 False  [LS1]  let Ok(entries) = fs::read_dir(&at) else {
(F) skip    LINE   crates/store/src/file_write_door_tests.rs:68  [LS1]  continue;
(F)         BRANCH crates/store/src/file_write_door_tests.rs:156:13 False  [LS1]  let StoreError::Sealed {
(F)         LINE   crates/store/src/file_write_door_tests.rs:165  [LS1]  panic!("a new bar at a sealed month is `Sealed`, got {refused}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:322  [LS1]  _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:439  [LS1]  _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:539  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:595  [LS1]  other => panic!("{n}: {other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:605  [LS1]  other => panic!("{n}: {other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:721  [LS1]  file.display()
(F)         LINE   crates/store/src/file_write_door_tests.rs:749  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:765  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:809  [LS1]  Err(other) => panic!("{other}"),
(F)         LINE   crates/store/src/file_write_door_tests.rs:844  [LS1]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:852:13 False  [LS1]  let MonthRepair::Repaired {
(F)         LINE   crates/store/src/file_write_door_tests.rs:859  [LS1]  panic!("{case}: {repaired:?}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:902  [LS1]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:942:9 False  [LS1]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F)         LINE   crates/store/src/file_write_door_tests.rs:943  [LS1]  panic!("{repaired:?}");
(F)         LINE   crates/store/src/file_write_door_tests.rs:983  [LS1]  matches!(
(F)         BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False  [LS1]  let MonthRepair::Repaired { to, skipped, .. } = repaired else {
(F)         LINE   crates/store/src/file_write_door_tests.rs:1027  [LS1]  panic!("{case}: {repaired:?}");
(F)         BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False  [LS1]  let MonthRepair::Repaired {
(F)         LINE   crates/store/src/file_write_door_tests.rs:1067  [LS1]  panic!("{case}: {repaired:?}");
(U)         LINE   crates/store/src/flock.rs:342  [LS1]  || "an unnamed program".to_owned(),
(U)         BRANCH crates/store/src/flock.rs:571:24 False  [LS1]  if let Ok(modified) = meta.modified() {
(U)         BRANCH crates/store/src/flock.rs:574:24 False  [LS1]  if let Ok(accessed) = meta.accessed() {
(F)         BRANCH crates/store/src/flock_cover_tests.rs:72:9 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:72:36 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:120  [LS1]  matches!(
(F)         BRANCH crates/store/src/flock_cover_tests.rs:137:17 False  [LS1]  assert!(entry.writer.is_none() && entry.queue.is_none());
(F)         BRANCH crates/store/src/flock_cover_tests.rs:137:43 False  [LS1]  assert!(entry.writer.is_none() && entry.queue.is_none());
(F)         BRANCH crates/store/src/flock_cover_tests.rs:208:9 False  [LS1]  !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:208:36 False  [LS1]  !second.took.is_empty() && !second.took.contains("pid 1 holds it"),
(F)         BRANCH crates/store/src/flock_cover_tests.rs:214:9 False  [LS1]  whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
(F)         BRANCH crates/store/src/flock_cover_tests.rs:215:16 False  [LS1]  && whole_sentence.ends_with("; pid 1 holds it"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:367  [LS1]  other => panic!("the host's refusal, not a wait or a lock: {other:?}"),
(F)         LINE   crates/store/src/flock_cover_tests.rs:400  [LS1]  other => panic!("the host's refusal, not a hold: {other:?}"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:101:9 False  [LS1]  let Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:109  [LS1]  panic!("contention, not a host refusal: {refused}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:123:9 False  [LS1]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:123:26 False  [LS1]  took >= bound && took < bound + MAX_PAUSE + SLACK,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:172:9 False  [LS1]  let Err(Impatient::Host(why)) = got else {
(F)         LINE   crates/store/src/flock_patient_tests.rs:173  [LS1]  panic!("a host refusal: {got:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:191:9 False  [LS1]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:195  [LS1]  panic!("{got:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:219:62 False  [LS1]  matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
(F)         BRANCH crates/store/src/flock_patient_tests.rs:261:9 False  [LS1]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:265  [LS1]  panic!("{got:?}");
(F) fail    LINE   crates/store/src/flock_patient_tests.rs:323  [LS1]  )?;
(F) collect LINE   crates/store/src/flock_patient_tests.rs:364  [LS1]  Err(_) => refused += 1,
(F)         BRANCH crates/store/src/flock_patient_tests.rs:435:9 False  [LS1]  let Err(Impatient::Held {
(F)         LINE   crates/store/src/flock_patient_tests.rs:439  [LS1]  panic!("it gave up: {refused:?}");
(F)         BRANCH crates/store/src/flock_patient_tests.rs:565:9 False  [LS1]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:565:51 False  [LS1]  note.contains("thread ag06-successor") && note.contains("doing the second"),
(F) collect LINE   crates/store/src/flock_patient_tests.rs:611  [LS1]  Err(_) => refused += 1,
(F)         LINE   crates/store/src/flock_patient_tests.rs:630  [LS1]  Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
(F) collect BRANCH crates/store/src/flock_patient_tests.rs:639:16 True  [LS1]  if !named {
(F) collect LINE   crates/store/src/flock_patient_tests.rs:640  [LS1]  unnamed += 1;
(F)         BRANCH crates/store/src/flock_patient_tests.rs:683:9 False  [LS1]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F)         BRANCH crates/store/src/flock_patient_tests.rs:683:50 False  [LS1]  text.contains("held by this process") && text.contains("doing the journal index build"),
(F)         LINE   crates/store/src/flock_patient_tests.rs:687  [LS1]  matches!(
(F)         LINE   crates/store/src/flock_tests.rs:273  [LS1]  panic!(
(F)         LINE   crates/store/src/flock_tests.rs:304  [LS1]  panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
(F)         LINE   crates/store/src/flock_tests.rs:324  [LS1]  panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
(F)         LINE   crates/store/src/flock_tests.rs:343  [LS1]  panic!(
(F)         LINE   crates/store/src/flock_tests.rs:380  [LS1]  panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
(F) fail    LINE   crates/store/src/flock_tests.rs:759  [LS1]  matches!(
(F)         BRANCH crates/store/src/flock_tests.rs:792:13 False  [LS1]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F)         BRANCH crates/store/src/flock_tests.rs:792:38 False  [LS1]  assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
(F)         BRANCH crates/store/src/flock_tests.rs:819:9 False  [LS1]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F)         BRANCH crates/store/src/flock_tests.rs:819:42 False  [LS1]  said.contains("r3-08-holder") && !said.contains("nreadable"),
(F)         BRANCH crates/store/src/flock_tests.rs:901:9 False  [LS1]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F)         BRANCH crates/store/src/flock_tests.rs:901:51 False  [LS1]  said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
(F)         LINE   crates/store/src/flock_tests.rs:984  [LS1]  matches!(
(F)         BRANCH crates/store/src/flock_tests.rs:1012:9 False  [LS1]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F)         BRANCH crates/store/src/flock_tests.rs:1012:66 False  [LS1]  note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
(F)         BRANCH crates/store/src/generation_cover_tests.rs:125:55 False  [LS1]  Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
(F)         BRANCH crates/store/src/generation_cover_tests.rs:132:55 False  [LS1]  Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
(F)         BRANCH crates/store/src/generation_cover_tests.rs:138:55 True  [LS1]  Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
(F)         BRANCH crates/store/src/generation_door_cover_tests.rs:99:9 False  [LS1]  let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:100  [LS1]  panic!("revision 1 is served");
(F)         BRANCH crates/store/src/generation_door_cover_tests.rs:137:59 False  [LS1]  Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:156  [LS1]  other => panic!("the pointer read is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:181  [LS1]  other => panic!("the linked pointer is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:213  [LS1]  other => panic!("the directory's creation is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:224  [LS1]  other => panic!("the pointer's open is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:235  [LS1]  other => panic!("the pointer's lock is refused: {other:?}"),
(F)         LINE   crates/store/src/generation_door_cover_tests.rs:279  [LS1]  other => panic!("the record's write is refused: {other:?}"),
(U)         BRANCH crates/store/src/month_summary.rs:939:24 False  [LS1]  if let Some(words) = entry.seconds.as_deref() {
(U)         BRANCH crates/store/src/month_summary.rs:1055:12 True  [LS1]  if at.short || at.at != payload.len() {
(U)         BRANCH crates/store/src/month_summary.rs:1085:24 False  [LS1]  if let Some(words) = entry.seconds.as_deref_mut() {
(U)         LINE   crates/store/src/month_summary.rs:1087  [LS1]  }
(F)         LINE   crates/store/src/month_summary_decode_cover_tests.rs:171  [LS1]  other => panic!("the open is refused in the host's words: {other:?}"),
(F)         BRANCH crates/store/src/month_summary_tests.rs:278:52 False  [LS1]  matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
(F) bound   BRANCH crates/store/src/month_summary_tests.rs:293:9 False  [LS1]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/month_summary_tests.rs:294  [LS1]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/month_summary_tests.rs:295  [LS1]  panic!("a read of {} waited for a writer", pipe.display());
(F)         BRANCH crates/store/src/month_summary_tests.rs:320:53 False  [LS1]  matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:340:53 False  [LS1]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:355:53 False  [LS1]  matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:377:13 False  [LS1]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F)         BRANCH crates/store/src/month_summary_tests.rs:377:40 False  [LS1]  assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
(F)         BRANCH crates/store/src/month_summary_tests.rs:461:32 False  [LS1]  Refused::Shape(why) if why.contains("flag")
(F)         BRANCH crates/store/src/month_summary_tests.rs:467:60 False  [LS1]  matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
(F)         BRANCH crates/store/src/month_summary_tests.rs:480:59 False  [LS1]  assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:483:63 False  [LS1]  assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:488:61 False  [LS1]  assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:491:59 False  [LS1]  assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
(F)         BRANCH crates/store/src/month_summary_tests.rs:655:13 False  [LS1]  assert!(round.tracks_carries() && !round.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:655:39 False  [LS1]  assert!(round.tracks_carries() && !round.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:658:13 False  [LS1]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:658:42 False  [LS1]  assert!(!seconds.tracks_carries() && !seconds.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:739:37 False  [LS1]  Err(Refused::Shape(why)) if why.contains("not a held minute")
(F)         BRANCH crates/store/src/month_summary_tests.rs:757:13 False  [LS1]  assert!(!read.tracks_carries() && read.wants_carries());
(F)         BRANCH crates/store/src/month_summary_tests.rs:757:39 False  [LS1]  assert!(!read.tracks_carries() && read.wants_carries());
(F)         LINE   crates/store/src/month_summary_tests.rs:791  [LS1]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False  [LS1]  assert!(staging.is_dir() && !root.sum().exists());
(F)         BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False  [LS1]  assert!(staging.is_dir() && !root.sum().exists());
(U)         BRANCH crates/store/src/provenance.rs:1665:12 False  [LS1]  if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
(U)         BRANCH crates/store/src/provenance.rs:1673:12 False  [LS1]  if let Some(run) = bytes.get(at..at.saturating_add(N)) {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:167  [LS1]  other => panic!("the pointer refuses the read: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:187  [LS1]  other => panic!("the sealed form refuses the read: {other:?}"),
(F)         BRANCH crates/store/src/provenance_cover_tests.rs:204:9 False  [LS1]  let Found::Present(raw) = read(&root.0, june).unwrap() else {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:205  [LS1]  panic!("the raw sidecar holds a record");
(F)         LINE   crates/store/src/provenance_cover_tests.rs:219  [LS1]  other => panic!("the sealed image's record is served: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:253  [LS1]  other => panic!("the open is refused by name: {other:?}"),
(F)         BRANCH crates/store/src/provenance_cover_tests.rs:276:9 False  [LS1]  let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
(F)         LINE   crates/store/src/provenance_cover_tests.rs:277  [LS1]  panic!("the batch's record is on disk");
(F)         LINE   crates/store/src/provenance_cover_tests.rs:326  [LS1]  other => panic!("the undecodable head refuses the read: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:350  [LS1]  other => panic!("the lock refuses the landing: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:395  [LS1]  other => panic!("the refused look refuses the flush: {other:?}"),
(F)         LINE   crates/store/src/provenance_cover_tests.rs:425  [LS1]  other => panic!("the header's write refuses the landing: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False  [LS1]  matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:635  [LS1]  Ok(Level::Current(_)) => panic!("a torn image is never current"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:636  [LS1]  Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:643  [LS1]  Ok(Level::Current(_)) => panic!("a damaged header is never current"),
(F)         LINE   crates/store/src/quarantine_cover_tests.rs:644  [LS1]  Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:144:9 False  [LS1]  let Applied::Dropped { drop, .. } = applied else {
(F)         LINE   crates/store/src/quarantine_tests.rs:145  [LS1]  panic!("expected a dropped generation, got {applied:?}");
(F)         BRANCH crates/store/src/quarantine_tests.rs:174:9 False  [LS1]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
(F)         LINE   crates/store/src/quarantine_tests.rs:175  [LS1]  panic!("the month is labelled");
(F)         BRANCH crates/store/src/quarantine_tests.rs:210:63 False  [LS1]  assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
(F)         BRANCH crates/store/src/quarantine_tests.rs:263:9 False  [LS1]  let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
(F)         LINE   crates/store/src/quarantine_tests.rs:264  [LS1]  panic!("the month is answered by name");
(F)         BRANCH crates/store/src/quarantine_tests.rs:342:66 False  [LS1]  matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
(F)         BRANCH crates/store/src/quarantine_tests.rs:348:77 False  [LS1]  assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
(F)         BRANCH crates/store/src/quarantine_tests.rs:497:42 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F)         BRANCH crates/store/src/quarantine_tests.rs:497:71 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
(F)         BRANCH crates/store/src/quarantine_tests.rs:508:73 False  [LS1]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F)         BRANCH crates/store/src/quarantine_tests.rs:508:92 False  [LS1]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
(F)         LINE   crates/store/src/quarantine_tests.rs:551  [LS1]  other => panic!("expected the hold named, got {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:799:9 False  [LS1]  let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
(F)         LINE   crates/store/src/quarantine_tests.rs:800  [LS1]  panic!("the month is still answered by name");
(F)         BRANCH crates/store/src/quarantine_tests.rs:855:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:855:70 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         LINE   crates/store/src/quarantine_tests.rs:892  [LS1]  other => panic!("{why}: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1071:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F)         BRANCH crates/store/src/quarantine_tests.rs:1072:16 False  [LS1]  && e.bars == 6
(F)         BRANCH crates/store/src/quarantine_tests.rs:1073:16 False  [LS1]  && e.causes.said() == "before_listing and foreign_level"
(F)         LINE   crates/store/src/quarantine_tests.rs:1092  [LS1]  other => panic!("expected the quarantine to refuse by name, got {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:1268  [LS1]  other => panic!("the rerun indexes the drop: {other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1327:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1327:70 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1386:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:1386:70 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:1419:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
(F)         BRANCH crates/store/src/quarantine_tests.rs:1466:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.bars == 3
(F)         BRANCH crates/store/src/quarantine_tests.rs:1533:9 False  [LS1]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1533:29 False  [LS1]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1584:9 False  [LS1]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1584:29 False  [LS1]  refused.is_err() && words.contains("quarantined"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:38 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:67 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1639:82 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
(F)         BRANCH crates/store/src/quarantine_tests.rs:1747:58 False  [LS1]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1747:94 False  [LS1]  matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
(F)         BRANCH crates/store/src/quarantine_tests.rs:1865:77 False  [LS1]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
(F)         LINE   crates/store/src/quarantine_tests.rs:1995  [LS1]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:42 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:71 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2261:86 False  [LS1]  Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
(F)         BRANCH crates/store/src/quarantine_tests.rs:2316:73 False  [LS1]  matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),
(F)         LINE   crates/store/src/quarantine_tests.rs:2348  [LS1]  other => panic!("the lock's open refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2365  [LS1]  other => panic!("the index's open refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2546  [LS1]  other => panic!("the index read refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2575  [LS1]  other => panic!("the ledger read is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2609  [LS1]  other => panic!("the staged write refuses the act: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2641  [LS1]  other => panic!("the rename is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2680  [LS1]  other => panic!("{point}: the probe is refused: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2730  [LS1]  matches!(
(F)         LINE   crates/store/src/quarantine_tests.rs:2747  [LS1]  other => panic!("a refused ask is no writer: {other:?}"),
(F)         LINE   crates/store/src/quarantine_tests.rs:2775  [LS1]  other => panic!("the lock refuses the act: {other:?}"),
(U)         LINE   crates/store/src/repair.rs:851  [LS1]  rows: header.n_valid,
(U)         LINE   crates/store/src/repair.rs:852  [LS1]  ceiling,
(U)         LINE   crates/store/src/repair.rs:853  [LS1]  };
(F)         BRANCH crates/store/src/repair_cover_tests.rs:62:41 False  [LS1]  text.contains("refused") || text.contains("drop names"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:225  [LS1]  other @ MonthRestate::NothingNew { .. } => {
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:226  [LS1]  panic!("the added bars are published: {other:?}")
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:250  [LS1]  other @ MonthRepair::NothingNew { .. } => {
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:251  [LS1]  panic!("the empty month takes the bars: {other:?}")
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:327  [LS1]  matches!(
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:397  [LS1]  other => panic!("the added bar is restated: {other:?}"),
(F)         BRANCH crates/store/src/repair_door_cover_tests.rs:428:9 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/store/src/repair_door_cover_tests.rs:428:36 False  [LS1]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:549  [LS1]  other => panic!("{operation} refuses the publication: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:588  [LS1]  other => panic!("the shared lock refuses the publication: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:717  [LS1]  other => panic!("the reservation is refused: {other:?}"),
(F)         LINE   crates/store/src/repair_door_cover_tests.rs:752  [LS1]  other => panic!("the receipt's creation is refused: {other:?}"),
(F)         LINE   crates/store/src/repair_tests.rs:116  [LS1]  Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
(F)         LINE   crates/store/src/repair_tests.rs:630  [LS1]  timeframe.as_str()
(F) skip    BRANCH crates/store/src/repair_tests.rs:691:12 False  [LS1]  if let Ok(entries) = fs::read_dir(dir) {
(F) skip    LINE   crates/store/src/repair_tests.rs:700  [LS1]  }
(F)         BRANCH crates/store/src/repair_tests.rs:824:94 False  [LS1]  matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
(F)         BRANCH crates/store/src/repair_tests.rs:882:9 False  [LS1]  let super::MonthRestate::Restated {
(F)         LINE   crates/store/src/repair_tests.rs:891  [LS1]  panic!("expected a restated generation, got {done:?}");
(U)         BRANCH crates/store/src/safe_open.rs:299:9 False  [LS1]  let Some(flags) = FLAGS else {
(U)         LINE   crates/store/src/safe_open.rs:300  [LS1]  return Err(io(
(U)         LINE   crates/store/src/safe_open.rs:301  [LS1]  "open",
(U)         LINE   crates/store/src/safe_open.rs:302  [LS1]  io::Error::new(
(U)         LINE   crates/store/src/safe_open.rs:303  [LS1]  ErrorKind::Unsupported,
(U)         LINE   crates/store/src/safe_open.rs:304  [LS1]  "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
(U)         LINE   crates/store/src/safe_open.rs:305  [LS1]  ),
(U)         LINE   crates/store/src/safe_open.rs:306  [LS1]  ));
(F) skip    BRANCH crates/store/src/safe_open_tests.rs:66:16 True  [LS1]  if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
(F) skip    LINE   crates/store/src/safe_open_tests.rs:67  [LS1]  tried.push(format!("{}: {why}", link.display()));
(F) skip    LINE   crates/store/src/safe_open_tests.rs:68  [LS1]  continue;
(F) skip    LINE   crates/store/src/safe_open_tests.rs:74  [LS1]  Err(why) => tried.push(format!("{}: {why}", link.display())),
(F)         LINE   crates/store/src/safe_open_tests.rs:77  [LS1]  panic!(
(F)         LINE   crates/store/src/safe_open_tests.rs:79  [LS1]  path.display(),
(F)         LINE   crates/store/src/safe_open_tests.rs:80  [LS1]  tried.join("; ")
(F)         BRANCH crates/store/src/safe_open_tests.rs:95:42 False  [LS1]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F)         LINE   crates/store/src/safe_open_tests.rs:103  [LS1]  found.file_type()
(F)         LINE   crates/store/src/safe_open_tests.rs:110  [LS1]  link.display()
(F) bound   BRANCH crates/store/src/safe_open_tests.rs:122:9 False  [LS1]  let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/safe_open_tests.rs:123  [LS1]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/safe_open_tests.rs:124  [LS1]  panic!("an open of {} waited for a writer", pipe.display());
(F) skip    BRANCH crates/store/src/safe_open_tests.rs:203:12 False  [LS1]  if let Some(device) = a_block_device() {
(F)         BRANCH crates/store/src/safe_open_tests.rs:309:16 False  [LS1]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F)         BRANCH crates/store/src/safe_open_tests.rs:309:35 False  [LS1]  if *root == shadow && *bars == shadow.join(BARS_DIR)
(F)         BRANCH crates/store/src/safe_open_tests.rs:310:20 False  [LS1]  && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
(F)         LINE   crates/store/src/safe_open_tests.rs:412  [LS1]  Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
(F)         LINE   crates/store/src/safe_open_tests.rs:413  [LS1]  Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
(F)         BRANCH crates/store/src/safe_open_tests.rs:470:77 False  [LS1]  matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
(U)         BRANCH crates/store/src/sealed.rs:1499:23 False  [LS1]  } else if let Some(range) = self.ranges.get_mut(5) {
(U)         BRANCH crates/store/src/sealed.rs:1525:16 False  [LS1]  if let Some(oi) = columns.get_mut(5) {
(U)         BRANCH crates/store/src/sealed.rs:2352:35 False  [LS1]  Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
(F)         LINE   crates/store/src/sealed_codec_tests.rs:81  [LS1]  frame.len()
(F)         LINE   crates/store/src/sealed_codec_tests.rs:282  [LS1]  frame.len()
(F)         LINE   crates/store/src/sealed_codec_tests.rs:439  [LS1]  largest[rows],
(F)         LINE   crates/store/src/sealed_codec_tests.rs:440  [LS1]  max_frame_len(m)
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False  [LS1]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False  [LS1]  assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False  [LS1]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F)         BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False  [LS1]  last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
(F)         LINE   crates/store/src/sealed_tests.rs:159  [LS1]  Ok(value) => panic!("expected a refusal, got {value:?}"),
(F) guard   BRANCH crates/store/src/sealed_tests.rs:237:12 False  [LS1]  if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
(F) fail    LINE   crates/store/src/sealed_tests.rs:499  [LS1]  Ok(())
(F)         LINE   crates/store/src/sealed_tests.rs:502  [LS1]  Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
(F)         LINE   crates/store/src/sealed_tests.rs:653  [LS1]  other => panic!("{other}"),
(F)         BRANCH crates/store/src/sealed_tests.rs:696:27 False  [LS1]  found == 0 || bars[found as usize - 1].ts_micros < ts,
(F)         LINE   crates/store/src/sealed_tests.rs:837  [LS1]  other => panic!("{field}: {other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1171  [LS1]  packed.len(),
(F)         LINE   crates/store/src/sealed_tests.rs:1172  [LS1]  image.len()
(F)         LINE   crates/store/src/sealed_tests.rs:1193  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1242  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1320  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1657  [LS1]  other => panic!("{field}: {other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1674  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1690  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1775  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1803  [LS1]  other => panic!("{other}"),
(F)         BRANCH crates/store/src/sealed_tests.rs:1817:100 False  [LS1]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
(F)         BRANCH crates/store/src/sealed_tests.rs:1823:105 False  [LS1]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F)         BRANCH crates/store/src/sealed_tests.rs:1829:105 False  [LS1]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
(F)         BRANCH crates/store/src/sealed_tests.rs:1837:105 False  [LS1]  matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
(F)         LINE   crates/store/src/sealed_tests.rs:1892  [LS1]  other => panic!("{other}"),
(F)         LINE   crates/store/src/sealed_tests.rs:1983  [LS1]  matches!(
(F)         LINE   crates/store/src/sealed_tests.rs:2105  [LS1]  other => panic!("{other}"),
(U)         BRANCH crates/store/src/settings.rs:220:16 False  [LS1]  if let Some(slot) = self.values.get_mut(key.slot()) {
(U)         BRANCH crates/store/src/settings.rs:688:30 False  [LS1]  let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
(F)         BRANCH crates/store/src/settings_cover_tests.rs:198:57 False  [LS1]  Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:97:9 False  [LS1]  with_mb.contains("is not a floor") && !with_mb.contains("percent"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:97:47 False  [LS1]  with_mb.contains("is not a floor") && !with_mb.contains("percent"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:128  [LS1]  other => panic!("the listing refuses the save: {other:?}"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:162:9 False  [LS1]  fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:162:51 False  [LS1]  fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:235:9 False  [LS1]  fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:235:60 False  [LS1]  fault.why.contains("grew past the 65536 bytes") && fault.why.contains("while it was read"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:261  [LS1]  other => panic!("the taken name refuses the save: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:288  [LS1]  other => panic!("the refused write refuses the save: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:324  [LS1]  other => panic!("the refused re-list is the store's fault: {other:?}"),
(F)         LINE   crates/store/src/settings_door_cover_tests.rs:377  [LS1]  other => panic!("the vanished temporary refuses the link: {other:?}"),
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:412:9 False  [LS1]  read_back.contains("did not read back")
(F)         BRANCH crates/store/src/settings_door_cover_tests.rs:413:16 False  [LS1]  && read_back.ends_with("it reads when read again, and the refresh did not step to it"),
(F)         LINE   crates/store/src/settings_tests.rs:152  [LS1]  other => panic!("{bad:?} for {key:?}: {other:?}"),
(F)         LINE   crates/store/src/settings_tests.rs:228  [LS1]  matches!(
(F)         LINE   crates/store/src/settings_tests.rs:417  [LS1]  reader.faults()
(F)         LINE   crates/store/src/settings_tests.rs:583  [LS1]  reader.faults()
(F)         LINE   crates/store/src/settings_tests.rs:654  [LS1]  reader.stopped()
(F) bound   BRANCH crates/store/src/settings_tests.rs:711:9 False  [LS1]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/settings_tests.rs:712  [LS1]  if let Some(pipe) = pipe {
(F) bound   BRANCH crates/store/src/settings_tests.rs:712:16 False  [LS1]  if let Some(pipe) = pipe {
(F) bound   BRANCH crates/store/src/settings_tests.rs:712:16 True  [LS1]  if let Some(pipe) = pipe {
(F) bound   LINE   crates/store/src/settings_tests.rs:713  [LS1]  let _release = std::fs::OpenOptions::new().write(true).open(pipe);
(F) bound   LINE   crates/store/src/settings_tests.rs:714  [LS1]  }
(F)         LINE   crates/store/src/settings_tests.rs:715  [LS1]  panic!("{what}: the settings read waited instead of naming a fault");
(F)         BRANCH crates/store/src/settings_tests.rs:789:48 False  [LS1]  matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
(F)         BRANCH crates/store/src/settings_tests.rs:913:60 False  [LS1]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
(F)         BRANCH crates/store/src/settings_tests.rs:959:60 False  [LS1]  matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
(F)         LINE   crates/store/src/settings_tests.rs:993  [LS1]  matches!(
(F)         BRANCH crates/store/src/settings_tests.rs:1100:9 False  [LS1]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F)         BRANCH crates/store/src/settings_tests.rs:1100:52 False  [LS1]  read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
(F) bound   BRANCH crates/store/src/settings_tests.rs:1156:20 True  [LS1]  if started.elapsed() > std::time::Duration::from_secs(10) {
(F) bound   LINE   crates/store/src/settings_tests.rs:1157  [LS1]  return None;
(F)         BRANCH crates/store/src/settings_tests.rs:1190:13 True  [LS1]  a.superseded_by == Some(3) || a.in_force == Some(2),
(F)         BRANCH crates/store/src/settings_tests.rs:1190:43 False  [LS1]  a.superseded_by == Some(3) || a.in_force == Some(2),
(F)         BRANCH crates/store/src/small_month_tests.rs:152:52 False  [LS1]  Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False  [LS1]  Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False  [LS1]  Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False  [LS1]  Err(Why::Unreadable(why)) if why.contains("is not a regular file")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False  [LS1]  Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
(F)         BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False  [LS1]  Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
(F)         BRANCH crates/store/src/sort_index_tests.rs:324:41 False  [LS1]  assert!(why.contains("kind") || why.contains("reserved"), "{why}");
(F)         LINE   crates/store/src/staging_cover_tests.rs:56  [LS1]  Err(other) => panic!("the removal is refused by name: {other:?}"),
(F)         LINE   crates/store/src/staging_cover_tests.rs:57  [LS1]  Ok(_) => panic!("the removal is refused by name, not staged"),
(F)         BRANCH crates/store/src/staging_tests.rs:143:16 False  [LS1]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:143:64 False  [LS1]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:206:67 False  [LS1]  matches!(&refusal, StagingError::Contended { path: p } if *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:238:16 False  [LS1]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:238:64 False  [LS1]  if source.kind() == ErrorKind::PermissionDenied && *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:297:78 False  [LS1]  matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
(F)         BRANCH crates/store/src/staging_tests.rs:334:88 False  [LS1]  matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
(F)         BRANCH crates/store/src/staging_tests.rs:369:77 False  [LS1]  matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
(F)         LINE   crates/store/src/staging_tests.rs:421  [LS1]  within.display()
(F)         BRANCH crates/store/src/time_index.rs:921:21 False  [LS1]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F)         BRANCH crates/store/src/time_index.rs:921:41 False  [LS1]  assert!(entry_reads <= 1 && bar_reads <= 1);
(F)         BRANCH crates/store/src/time_index.rs:1057:17 False  [LS1]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1057:38 False  [LS1]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1057:59 False  [LS1]  assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
(F)         BRANCH crates/store/src/time_index.rs:1058:17 False  [LS1]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F)         BRANCH crates/store/src/time_index.rs:1058:39 False  [LS1]  assert!(!entry.occupied(1) && !entry.occupied(62));
(F)         BRANCH crates/store/src/time_index.rs:1105:17 False  [LS1]  assert!(!daily.exact() && g.exact());
(F)         BRANCH crates/store/src/time_index.rs:1105:35 False  [LS1]  assert!(!daily.exact() && g.exact());
(U)         BRANCH crates/store/src/verdict.rs:716:8 True  [LS1]  if Vendor::ALL.get(at) != Some(&feed) {
(U)         LINE   crates/store/src/verdict.rs:717  [LS1]  return None;
(U)         BRANCH crates/store/src/verdict.rs:2278:16 False  [LS1]  if let Some(slot) = out.first.get_mut(ordinal) {
(U)         BRANCH crates/store/src/verdict.rs:2312:16 False  [LS1]  if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
(U)         BRANCH crates/store/src/verdict.rs:2478:16 False  [LS1]  if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
(U)         BRANCH crates/store/src/verdict.rs:3143:15 False  [LS1]  while let Some(parent) = at.parent() {
(U)         BRANCH crates/store/src/verdict.rs:3417:12 False  [LS1]  if let Some(entry) = days.get_mut(doy) {
(U)         BRANCH crates/store/src/verdict.rs:3853:24 False  [LS1]  && let Some(oldest) = self
(U)         BRANCH crates/store/src/verdict.rs:3923:13 False  [LS1]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:3924  [LS1]  return Err(VerdictError::ImageMissing { path });
(U)         BRANCH crates/store/src/verdict.rs:4075:13 False  [LS1]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:4076  [LS1]  return Err(VerdictError::ImageMissing { path });
(U)         BRANCH crates/store/src/verdict.rs:4136:13 False  [LS1]  let Some((file, _)) = self.handle(&path, &mut cost)? else {
(U)         LINE   crates/store/src/verdict.rs:4137  [LS1]  return Err(VerdictError::ImageMissing { path });
(F)         LINE   crates/store/src/verdict.rs:4219  [LS1]  other => panic!("not image_corrupt: {other:?}"),
(F)         LINE   crates/store/src/verdict.rs:4227  [LS1]  other => panic!("not written_by_newer_build: {other:?}"),
(U)         BRANCH crates/store/src/verdict.rs:4637:25 False  [LS1]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4637:37 False  [LS1]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4637:37 True  [LS1]  .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
(U)         BRANCH crates/store/src/verdict.rs:4974:24 False  [LS1]  .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
(F)         LINE   crates/store/src/verdict_tests.rs:366  [LS1]  other => panic!("expected the host's refusal, got {other}"),
(F) bound   BRANCH crates/store/src/verdict_tests.rs:460:9 False  [LS1]  let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
(F) bound   LINE   crates/store/src/verdict_tests.rs:461  [LS1]  let _release = fs::OpenOptions::new().write(true).open(pipe);
(F)         LINE   crates/store/src/verdict_tests.rs:462  [LS1]  panic!("a verdict call waited on the pipe at {}", pipe.display());
(F)         LINE   crates/store/src/verdict_tests.rs:568  [LS1]  other => panic!("expected the host's refusal, got {other}"),
(F)         LINE   crates/store/src/verdict_tests.rs:813  [LS1]  other => panic!("not image_corrupt: {other:?}"),
(F)         LINE   crates/store/src/verdict_tests.rs:879  [LS1]  other.dir().display()
(F)         BRANCH crates/store/src/verdict_tests.rs:950:9 False  [LS1]  let super::Found::Record { ordinal, record } = found.found else {
(F)         LINE   crates/store/src/verdict_tests.rs:951  [LS1]  panic!("the key's record: {:?}", found.found);
(U)         BRANCH crates/store/tests/catalog.rs:68:12 False  [LS1]  if let Some(parent) = full.parent() {
MAC         LINE   crates/store/tests/catalog.rs:308  [LS1: : match made {]  match made {
MAC         LINE   crates/store/tests/catalog.rs:310  [LS1: : Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {]  Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
MAC         LINE   crates/store/tests/catalog.rs:311  [LS1: : refused_by_the_volume += 1;]  refused_by_the_volume += 1;
MAC         LINE   crates/store/tests/catalog.rs:312  [LS1: : }]  }
(F)         LINE   crates/store/tests/catalog.rs:313  [LS1]  Err(why) => panic!("{} could not be made: {why}", full.display()),
(F)         BRANCH crates/store/tests/catalog.rs:317:9 False  [LS1]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F)         BRANCH crates/store/tests/catalog.rs:317:39 False  [LS1]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
(F)         BRANCH crates/store/tests/catalog.rs:317:39 True  [LS1]  refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
MAC         BRANCH crates/store/tests/catalog.rs:320:8 True  [LS1: : if refused_by_the_volume > 0 {]  if refused_by_the_volume > 0 {
MAC         LINE   crates/store/tests/catalog.rs:321  [LS1: : eprintln!(]  eprintln!(
MAC         LINE   crates/store/tests/catalog.rs:322  [LS1: : "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \]  "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \
MAC         LINE   crates/store/tests/catalog.rs:323  [LS1: : the walk is checked on what it holds and the five shapes by \]  the walk is checked on what it holds and the five shapes by \
MAC         LINE   crates/store/tests/catalog.rs:324  [LS1: : every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"]  every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"
MAC         LINE   crates/store/tests/catalog.rs:325  [LS1: : );]  );
(F) skip    BRANCH crates/store/tests/catalog.rs:814:16 True  [LS1]  if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
(F) skip    LINE   crates/store/tests/catalog.rs:815  [LS1]  tried.push(format!("{}: {why}", link.display()));
(F) skip    LINE   crates/store/tests/catalog.rs:816  [LS1]  continue;
(F) skip    LINE   crates/store/tests/catalog.rs:822  [LS1]  Err(why) => tried.push(format!("{}: {why}", link.display())),
(F)         LINE   crates/store/tests/catalog.rs:825  [LS1]  panic!(
(F)         LINE   crates/store/tests/catalog.rs:827  [LS1]  path.display(),
(F)         LINE   crates/store/tests/catalog.rs:828  [LS1]  tried.join("; ")
(F)         BRANCH crates/store/tests/catalog.rs:844:42 False  [LS1]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F)         LINE   crates/store/tests/catalog.rs:852  [LS1]  found.file_type()
(F)         LINE   crates/store/tests/catalog.rs:859  [LS1]  link.display()
(F)         LINE   crates/store/tests/catalog.rs:875  [LS1]  path.display()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:288:12 True  [LS1]  if let Err(why) = Command::new("git").arg("--version").output() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:289  [LS1]  println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
(F) skip    LINE   crates/store/tests/cited_commits.rs:290  [LS1]  return false;
(F) fail    BRANCH crates/store/tests/cited_commits.rs:294:21 False  [LS1]  Err(why) if why.kind() == std::io::ErrorKind::NotFound
(U)         BRANCH crates/store/tests/cited_commits.rs:306:12 False  [LS1]  if let Some(above) = root.parent() {
(F)         BRANCH crates/store/tests/cited_commits.rs:371:9 False  [LS1]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F)         BRANCH crates/store/tests/cited_commits.rs:372:16 False  [LS1]  && said.contains("not a git repository"),
(F)         BRANCH crates/store/tests/cited_commits.rs:384:9 False  [LS1]  said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
(F)         BRANCH crates/store/tests/cited_commits.rs:385:16 False  [LS1]  && said.contains("not a git repository"),
(F) collect BRANCH crates/store/tests/cited_commits.rs:567:13 False  [LS1]  let Some(citing) = cited.get(commit) else {
(F) collect LINE   crates/store/tests/cited_commits.rs:568  [LS1]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:571  [LS1]  continue;
(F) collect LINE   crates/store/tests/cited_commits.rs:575  [LS1]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:576  [LS1]  "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
(F) collect LINE   crates/store/tests/cited_commits.rs:577  [LS1]  at.name, at.sentence
(F) collect LINE   crates/store/tests/cited_commits.rs:578  [LS1]  ));
(F) collect LINE   crates/store/tests/cited_commits.rs:579  [LS1]  }
(F) collect BRANCH crates/store/tests/cited_commits.rs:601:16 True  [LS1]  if !is_ancestor_at(root, commit, main_ref) {
(F) collect LINE   crates/store/tests/cited_commits.rs:602  [LS1]  let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
(F) collect LINE   crates/store/tests/cited_commits.rs:603  [LS1]  faults.push(format!(
(F) collect BRANCH crates/store/tests/cited_commits.rs:610:12 True  [LS1]  if is_ancestor_at(root, commit, main_ref) {
(F) collect LINE   crates/store/tests/cited_commits.rs:611  [LS1]  faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
(F) collect BRANCH crates/store/tests/cited_commits.rs:613:35 True  [LS1]  if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
(F) collect LINE   crates/store/tests/cited_commits.rs:614  [LS1]  faults.push(format!(
(F) collect LINE   crates/store/tests/cited_commits.rs:615  [LS1]  "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
(F) collect LINE   crates/store/tests/cited_commits.rs:616  [LS1]  ));
(F)         LINE   crates/store/tests/cited_commits.rs:673  [LS1]  cited.len()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:676:8 False  [LS1]  if history_is_here() {
(F)         LINE   crates/store/tests/cited_commits.rs:709  [LS1]  String::from_utf8_lossy(&out.stderr)
(F) skip    BRANCH crates/store/tests/cited_commits.rs:731:8 True  [LS1]  if Command::new("git").arg("--version").output().is_err() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:732  [LS1]  println!("SKIPPING: git cannot run the synthetic history fixture");
(F) skip    LINE   crates/store/tests/cited_commits.rs:733  [LS1]  return;
(F) skip    BRANCH crates/store/tests/cited_commits.rs:819:8 True  [LS1]  if Command::new("git").arg("--version").output().is_err() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:820  [LS1]  println!("SKIPPING: git cannot run the synthetic history fixture");
(F) skip    LINE   crates/store/tests/cited_commits.rs:821  [LS1]  return;
(F)         LINE   crates/store/tests/cited_commits.rs:976  [LS1]  sites.len()
(F) skip    BRANCH crates/store/tests/cited_commits.rs:978:8 True  [LS1]  if !history_is_here() {
(F) skip    LINE   crates/store/tests/cited_commits.rs:979  [LS1]  return;
(F) collect BRANCH crates/store/tests/cited_commits.rs:996:12 True  [LS1]  if !takes_a_lock(line) {
(F) collect LINE   crates/store/tests/cited_commits.rs:997  [LS1]  wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
(F) bound   BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False  [LS1]  while (vanished < 20 || contended < 20) && Instant::now() < deadline {
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False  [LS1]  vanished >= 20 && contended >= 20,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False  [LS1]  vanished >= 20 && contended >= 20,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False  [LS1]  } if source.kind() == std::io::ErrorKind::PermissionDenied
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:367  [LS1]  other => panic!("{door}: {other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False  [LS1]  let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:371  [LS1]  panic!("a moved revision is audited");
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False  [LS1]  audited.starts_with("repair revision 1 of ")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False  [LS1]  && audited.ends_with(" is not the one its promotion recorded; not audited"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:446  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1073  [LS1]  other => panic!("not image_corrupt: {other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False  [LS1]  let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1098  [LS1]  panic!("ZZB is in the day");
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1171  [LS1]  matches!(
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False  [LS1]  generation.contains("/nonexistent/cloud/2031-02.tix")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False  [LS1]  && generation.contains("revision 1 is not the one its promotion recorded"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1361  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1568  [LS1]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False  [LS1]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False  [LS1]  if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False  [LS1]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False  [LS1]  if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False  [LS1]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False  [LS1]  if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False  [LS1]  if path == &lock_path && *tries == steps + 1
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False  [LS1]  if path == &lock_path && *tries == steps + 1
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False  [LS1]  let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:1925  [LS1]  panic!("{name} is in the day");
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False  [LS1]  if path.parent() == Some(year.as_path())
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False  [LS1]  && path.extension().is_some_and(|ext| ext == "tmp")
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False  [LS1]  && source.kind() == std::io::ErrorKind::PermissionDenied
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False  [LS1]  TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False  [LS1]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False  [LS1]  if *path == bar_path && *offset == 32_768 + 56 * 4
(F) guard   BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False  [LS1]  let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
(F) guard   LINE   crates/store/tests/cloud_mutants_store.rs:2385  [LS1]  return Err(std::io::ErrorKind::UnexpectedEof.into());
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False  [LS1]  if *path == bar_path
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False  [LS1]  TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
(F)         LINE   crates/store/tests/cloud_mutants_store.rs:2876  [LS1]  writer.time_lookup()
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False  [LS1]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False  [LS1]  if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False  [LS1]  StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False  [LS1]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F) bound   BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True  [LS1]  if since.elapsed() > std::time::Duration::from_secs(5) {
(F) bound   LINE   crates/store/tests/cloud_mutants_store.rs:3172  [LS1]  return Err(std::io::Error::other("flushed alone"));
(F) skip    LINE   crates/store/tests/cloud_mutants_store.rs:3253  [LS1]  _ => None,
(F)         BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False  [LS1]  matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
(F)         BRANCH crates/store/tests/docs.rs:220:9 False  [LS1]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F)         BRANCH crates/store/tests/docs.rs:220:47 False  [LS1]  BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
(F)         BRANCH crates/store/tests/docs.rs:234:9 False  [LS1]  entry.contains(
(F)         BRANCH crates/store/tests/docs.rs:236:14 False  [LS1]  ) && entry.contains("this entry supersedes that sentence"),
(F)         BRANCH crates/store/tests/docs.rs:253:9 False  [LS1]  FILE_RS.contains("if claimed > header.n_valid {")
(F)         BRANCH crates/store/tests/docs.rs:254:16 False  [LS1]  && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
(F)         LINE   crates/store/tests/durability.rs:381  [LS1]  panic!(
(F)         BRANCH crates/store/tests/fault.rs:143:36 False  [LS1]  read.n_valid == 150 || read.n_valid == 175,
(F)         BRANCH crates/store/tests/fault.rs:220:59 False  [LS1]  horizon == previous_commit.durable_through || horizon == commit.durable_through,
(F)         LINE   crates/store/tests/fault.rs:376  [LS1]  panic!("bit {bit} of byte {byte} took the whole file down: {e}")
(F)         LINE   crates/store/tests/fifo.rs:108  [LS1]  at.display()
(F) bound   BRANCH crates/store/tests/fifo.rs:121:12 False  [LS1]  if let Ok(outcome) = recv.recv_timeout(BOUND) {
(F) bound   LINE   crates/store/tests/fifo.rs:124  [LS1]  }
(F) bound   LINE   crates/store/tests/fifo.rs:126  [LS1]  drop(fs::OpenOptions::new().write(true).open(release));
(F) bound   LINE   crates/store/tests/fifo.rs:127  [LS1]  drop(worker.join());
(F)         LINE   crates/store/tests/fifo.rs:128  [LS1]  panic!(
(F)         LINE   crates/store/tests/fifo.rs:130  [LS1]  release.display()
(F)         BRANCH crates/store/tests/fifo.rs:157:13 False  [LS1]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F)         BRANCH crates/store/tests/fifo.rs:157:61 False  [LS1]  rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
(F)         BRANCH crates/store/tests/fifo.rs:162:53 False  [LS1]  fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
(F) skip    BRANCH crates/store/tests/fifo.rs:212:8 True  [LS1]  if !null.exists() {
(F) skip    LINE   crates/store/tests/fifo.rs:213  [LS1]  return;
(F)         BRANCH crates/store/tests/generation.rs:153:9 False  [LS1]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:165  [LS1]  panic!("{done:?}");
(F)         BRANCH crates/store/tests/generation.rs:179:57 False  [LS1]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F)         BRANCH crates/store/tests/generation.rs:179:76 False  [LS1]  assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
(F)         LINE   crates/store/tests/generation.rs:217  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:238  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:331  [LS1]  matches!(
(F)         BRANCH crates/store/tests/generation.rs:364:9 False  [LS1]  let MonthRepair::Repaired { to, skipped, .. } =
(F)         LINE   crates/store/tests/generation.rs:367  [LS1]  panic!("repaired");
(F)         BRANCH crates/store/tests/generation.rs:384:9 False  [LS1]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:388  [LS1]  panic!("{second:?}");
(F)         LINE   crates/store/tests/generation.rs:475  [LS1]  other => panic!("{other:?}"),
(F)         LINE   crates/store/tests/generation.rs:517  [LS1]  other => panic!("{other:?}"),
(F)         BRANCH crates/store/tests/generation.rs:670:13 False  [LS1]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:670:42 False  [LS1]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:670:42 True  [LS1]  assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
(F)         BRANCH crates/store/tests/generation.rs:694:74 False  [LS1]  matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
(F)         BRANCH crates/store/tests/generation.rs:749:9 False  [LS1]  let MonthRepair::Repaired {
(F)         LINE   crates/store/tests/generation.rs:758  [LS1]  panic!("{repaired:?}");
(F)         BRANCH crates/store/tests/geometry.rs:99:13 False  [LS1]  start >= block_start && end <= block_end,
(F)         BRANCH crates/store/tests/geometry.rs:99:37 False  [LS1]  start >= block_start && end <= block_end,
(F) collect BRANCH crates/store/tests/geometry.rs:134:13 True  [LS1]  start < block_start || end > block_end
(F) fail    LINE   crates/store/tests/libm_key.rs:205  [LS1]  matches!(
(F)         BRANCH crates/store/tests/libm_key.rs:428:9 False  [LS1]  decision < caveat && caveat < rejected,
(F)         BRANCH crates/store/tests/libm_key.rs:428:30 False  [LS1]  decision < caveat && caveat < rejected,
(F)         LINE   crates/store/tests/libm_key.rs:1330  [LS1]  if mentions { "is not" } else { "is" }
(F)         BRANCH crates/store/tests/libm_key.rs:1330:16 False  [LS1]  if mentions { "is not" } else { "is" }
(F)         BRANCH crates/store/tests/libm_key.rs:1330:16 True  [LS1]  if mentions { "is not" } else { "is" }
(F)         LINE   crates/store/tests/misfiled.rs:184  [LS1]  other => panic!("expected Misfiled, found {other:?}"),
(F)         BRANCH crates/store/tests/misfiled.rs:217:9 False  [LS1]  said.contains("2019-11-28-1250000-CE")
(F)         BRANCH crates/store/tests/misfiled.rs:218:16 False  [LS1]  && said.contains("its store path is not this month's")
(F)         BRANCH crates/store/tests/misfiled.rs:219:16 False  [LS1]  && said.contains("DB-B65"),
(F)         LINE   crates/store/tests/misfiled.rs:321  [LS1]  unborn.len()
(F)         LINE   crates/store/tests/month_repair.rs:156  [LS1]  other => panic!("not a repair: {other:?}"),
(F)         LINE   crates/store/tests/month_repair.rs:324  [LS1]  timeframe.as_str()
(F)         BRANCH crates/store/tests/month_repair.rs:606:13 False  [LS1]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F)         BRANCH crates/store/tests/month_repair.rs:606:47 False  [LS1]  assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
(F) collect LINE   crates/store/tests/month_repair.rs:637  [LS1]  calls += 1;
(F) collect LINE   crates/store/tests/month_repair.rs:638  [LS1]  Ok(())
(F) collect LINE   crates/store/tests/month_repair.rs:639  [LS1]  };
(F)         BRANCH crates/store/tests/month_repair.rs:739:83 False  [LS1]  matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
(F)         BRANCH crates/store/tests/month_repair.rs:852:9 False  [LS1]  let Ok(MonthRepair::Repaired {
(F)         LINE   crates/store/tests/month_repair.rs:856  [LS1]  panic!("{outcome:?}");
(F)         BRANCH crates/store/tests/month_repair.rs:879:38 False  [LS1]  Served::Revision(pointer) if pointer.revision.ordinal() == 3
(F)         LINE   crates/store/tests/month_repair.rs:918  [LS1]  left.keys()
(F)         LINE   crates/store/tests/month_repair.rs:944  [LS1]  other => panic!("not a repair: {other:?}"),
(F) collect BRANCH crates/store/tests/open_flags.rs:175:16 True  [LS1]  if names_the_x86_bit(line) {
(F) collect LINE   crates/store/tests/open_flags.rs:176  [LS1]  offenders.push(format!("{}:{}", path.display(), number + 1));
(F)         LINE   crates/store/tests/provenance.rs:66  [LS1]  other => panic!("expected a sidecar, found {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:161  [LS1]  Recorded::NothingAppended => panic!("expected a record"),
(F)         LINE   crates/store/tests/provenance.rs:451  [LS1]  other => panic!("expected NotTheNewestCommit, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:464  [LS1]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:476  [LS1]  other => panic!("expected WrongBarFile, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:531  [LS1]  other => panic!("expected OutOfOrder, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:639  [LS1]  other => panic!("expected Damaged, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:698  [LS1]  result => panic!("expected a {field} mismatch, got {result:?}"),
(F)         LINE   crates/store/tests/provenance.rs:746  [LS1]  other => panic!("expected PastTheBars, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1200  [LS1]  other => panic!("expected {fault:?}, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1308  [LS1]  other => panic!("expected TooLarge, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1409  [LS1]  other => panic!("expected Record, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1611  [LS1]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1617  [LS1]  other => panic!("expected NotABarPath, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1671  [LS1]  other => panic!("expected the directory sync refused, got {other:?}"),
(F)         LINE   crates/store/tests/provenance.rs:1721  [LS1]  other => panic!("expected the open refused, got {other:?}"),
(F)         BRANCH crates/store/tests/repair.rs:506:13 True  [LS1]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F)         BRANCH crates/store/tests/repair.rs:506:41 False  [LS1]  assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False  [LS1]  let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
(F)         LINE   crates/store/tests/sealed_codec_alloc.rs:115  [LS1]  panic!("a frame cut to {len} bytes left {frame_answer:?}");
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False  [LS1]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False  [LS1]  in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
(F)         LINE   crates/store/tests/sealed_codec_alloc.rs:125  [LS1]  full.len()
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False  [LS1]  !text.contains("by its checked rows and no more")
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False  [LS1]  && !text.contains("checked rows times 56 bytes, once"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False  [LS1]  text.contains("the largest of twice") && text.contains("and 4"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False  [LS1]  text.contains("the largest of twice") && text.contains("and 4"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False  [LS1]  text.contains("capacity 0") && text.contains("cleared"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False  [LS1]  text.contains("capacity 0") && text.contains("cleared"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False  [LS1]  !text.contains("random fields cost at most 57 bytes a bar")
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False  [LS1]  && !text.contains("at most 57 bytes a bar, 1 more than raw"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False  [LS1]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
(F)         BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False  [LS1]  SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:65  [LS1: : fn read_month(path: &Path) -> Vec<Bar> {]  fn read_month(path: &Path) -> Vec<Bar> {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:68  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:69  [LS1: : path.extension().is_some_and(|e| e == "bin"),]  path.extension().is_some_and(|e| e == "bin"),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:71  [LS1: : path.display()]  path.display()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:73  [LS1: : let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));]  let data = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:74  [LS1: : let header = data.get(..RECORDS_AT).unwrap_or_else(|| {]  let header = data.get(..RECORDS_AT).unwrap_or_else(|| {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:75  [LS1: : panic!(]  panic!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:77  [LS1: : path.display(),]  path.display(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:78  [LS1: : data.len()]  data.len()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:81  [LS1: : let header = Header::read_region(header, data.len() as u64)]  let header = Header::read_region(header, data.len() as u64)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:82  [LS1: : .unwrap_or_else(|e| panic!("{}: {e}", path.display()));]  .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:83  [LS1: : let n = header.n_valid as usize;]  let n = header.n_valid as usize;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:84  [LS1: : data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)]  data.get(RECORDS_AT..RECORDS_AT + n * STRIDE)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:85  [LS1: : .unwrap_or_else(|| {]  .unwrap_or_else(|| {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:86  [LS1: : panic!(]  panic!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:88  [LS1: : path.display()]  path.display()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:91  [LS1: : .chunks_exact(STRIDE)]  .chunks_exact(STRIDE)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:92  [LS1: : .map(|record| Bar::decode(record).expect("a committed record decodes"))]  .map(|record| Bar::decode(record).expect("a committed record decodes"))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:93  [LS1: : .collect()]  .collect()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:94  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:97  [LS1: : fn origin_of(name: &str) -> i64 {]  fn origin_of(name: &str) -> i64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:98  [LS1: : let stem = name.trim_end_matches(".bin");]  let stem = name.trim_end_matches(".bin");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:99  [LS1: : let month = &stem[stem.len() - 7..];]  let month = &stem[stem.len() - 7..];
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:100  [LS1: : let year: u16 = month[..4].parse().expect("year");]  let year: u16 = month[..4].parse().expect("year");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:101  [LS1: : let mm: u8 = month[5..].parse().expect("month");]  let mm: u8 = month[5..].parse().expect("month");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:102  [LS1: : YearMonth::new(year, mm)]  YearMonth::new(year, mm)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:103  [LS1: : .expect("a month")]  .expect("a month")
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:104  [LS1: : .ist_bounds_micros()]  .ist_bounds_micros()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:105  [LS1: : .0]  .0
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:106  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:111  [LS1: : fn judge_row0(rows: &[Bar]) -> u64 {]  fn judge_row0(rows: &[Bar]) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:112  [LS1: : fn len(mut v: u64) -> u64 {]  fn len(mut v: u64) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:113  [LS1: : let mut n = 1;]  let mut n = 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:114  [LS1: : while v >= 0x80 {]  while v >= 0x80 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 False  [LS1: : while v >= 0x80 {]  while v >= 0x80 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:114:15 True  [LS1: : while v >= 0x80 {]  while v >= 0x80 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:115  [LS1: : v >>= 7;]  v >>= 7;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:116  [LS1: : n += 1;]  n += 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:117  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:118  [LS1: : n]  n
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:119  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:120  [LS1: : fn zz(x: i64) -> u64 {]  fn zz(x: i64) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:121  [LS1: : ((x << 1) ^ (x >> 63)).cast_unsigned()]  ((x << 1) ^ (x >> 63)).cast_unsigned()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:122  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:123  [LS1: : let first = rows.first().map_or(0, |b| b.ts_micros);]  let first = rows.first().map_or(0, |b| b.ts_micros);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:124  [LS1: : rows.iter()]  rows.iter()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:125  [LS1: : .step_by(256)]  .step_by(256)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:126  [LS1: : .map(|b| {]  .map(|b| {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:127  [LS1: : let d = b.ts_micros - first;]  let d = b.ts_micros - first;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:128  [LS1: : let ts = if d % 1_000_000 == 0 {]  let ts = if d % 1_000_000 == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 False  [LS1: : let ts = if d % 1_000_000 == 0 {]  let ts = if d % 1_000_000 == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:128:25 True  [LS1: : let ts = if d % 1_000_000 == 0 {]  let ts = if d % 1_000_000 == 0 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:129  [LS1: : len(zz(d / 1_000_000))]  len(zz(d / 1_000_000))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:131  [LS1: : len(zz(d))]  len(zz(d))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:133  [LS1: : let oi = if b.open_interest == OI_NULL {]  let oi = if b.open_interest == OI_NULL {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 False  [LS1: : let oi = if b.open_interest == OI_NULL {]  let oi = if b.open_interest == OI_NULL {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:133:25 True  [LS1: : let oi = if b.open_interest == OI_NULL {]  let oi = if b.open_interest == OI_NULL {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:134  [LS1: : 1]  1
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:136  [LS1: : 1 + len(zz(b.open_interest))]  1 + len(zz(b.open_interest))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:138  [LS1: : ts + 1]  ts + 1
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:139  [LS1: : + len(zz(b.close))]  + len(zz(b.close))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:140  [LS1: : + len(zz(b.open - b.close))]  + len(zz(b.open - b.close))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:141  [LS1: : + len(zz(b.high - b.close))]  + len(zz(b.high - b.close))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:142  [LS1: : + len(zz(b.low - b.close))]  + len(zz(b.low - b.close))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:143  [LS1: : + len(zz(b.volume))]  + len(zz(b.volume))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:144  [LS1: : + oi]  + oi
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:145  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:146  [LS1: : .sum()]  .sum()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:147  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:150  [LS1: : const fn zz(x: i64) -> u64 {]  const fn zz(x: i64) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:151  [LS1: : ((x << 1) ^ (x >> 63)).cast_unsigned()]  ((x << 1) ^ (x >> 63)).cast_unsigned()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:152  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:154  [LS1: : const fn varint_len(value: u64) -> usize {]  const fn varint_len(value: u64) -> usize {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:155  [LS1: : let bits = (64 - value.leading_zeros()) as usize;]  let bits = (64 - value.leading_zeros()) as usize;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:156  [LS1: : if bits == 0 { 1 } else { bits.div_ceil(7) }]  if bits == 0 { 1 } else { bits.div_ceil(7) }
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 False  [LS1: : if bits == 0 { 1 } else { bits.div_ceil(7) }]  if bits == 0 { 1 } else { bits.div_ceil(7) }
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:156:8 True  [LS1: : if bits == 0 { 1 } else { bits.div_ceil(7) }]  if bits == 0 { 1 } else { bits.div_ceil(7) }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:157  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:160  [LS1: : fn row0_len(first: &Bar, origin: i64) -> usize {]  fn row0_len(first: &Bar, origin: i64) -> usize {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:161  [LS1: : let d = first.ts_micros.wrapping_sub(origin);]  let d = first.ts_micros.wrapping_sub(origin);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:162  [LS1: : let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };]  let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 False  [LS1: : let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };]  let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:162:20 True  [LS1: : let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };]  let stamp = if d % 1_000_000 == 0 { d / 1_000_000 } else { d };
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:163  [LS1: : let oi = if first.open_interest == OI_NULL {]  let oi = if first.open_interest == OI_NULL {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 False  [LS1: : let oi = if first.open_interest == OI_NULL {]  let oi = if first.open_interest == OI_NULL {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:163:17 True  [LS1: : let oi = if first.open_interest == OI_NULL {]  let oi = if first.open_interest == OI_NULL {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:164  [LS1: : 0]  0
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:166  [LS1: : varint_len(zz(first.open_interest))]  varint_len(zz(first.open_interest))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:168  [LS1: : varint_len(zz(stamp))]  varint_len(zz(stamp))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:169  [LS1: : + varint_len(zz(first.close))]  + varint_len(zz(first.close))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:170  [LS1: : + varint_len(zz(first.open.wrapping_sub(first.close)))]  + varint_len(zz(first.open.wrapping_sub(first.close)))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:171  [LS1: : + varint_len(zz(first.high.wrapping_sub(first.close)))]  + varint_len(zz(first.high.wrapping_sub(first.close)))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:172  [LS1: : + varint_len(zz(first.low.wrapping_sub(first.close)))]  + varint_len(zz(first.low.wrapping_sub(first.close)))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:173  [LS1: : + varint_len(zz(first.volume))]  + varint_len(zz(first.volume))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:174  [LS1: : + oi]  + oi
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:175  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:179  [LS1: : fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {]  fn residuals(rows: &[Bar], column: usize, mode: u8) -> Vec<i64> {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:180  [LS1: : rows.windows(2)]  rows.windows(2)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:181  [LS1: : .map(|pair| {]  .map(|pair| {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:182  [LS1: : let (p, b) = (pair[0], pair[1]);]  let (p, b) = (pair[0], pair[1]);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:183  [LS1: : let (value, predictor) = match (column, mode) {]  let (value, predictor) = match (column, mode) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:184  [LS1: : (0, _) => (b.ts_micros, p.ts_micros),]  (0, _) => (b.ts_micros, p.ts_micros),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:185  [LS1: : (1, _) => (b.close, p.close),]  (1, _) => (b.close, p.close),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:186  [LS1: : (2, 0) => (b.open, p.close),]  (2, 0) => (b.open, p.close),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:187  [LS1: : (2, _) => (b.open, b.close),]  (2, _) => (b.open, b.close),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:188  [LS1: : (3, _) => (b.high, b.open.max(b.close)),]  (3, _) => (b.high, b.open.max(b.close)),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:189  [LS1: : (4, _) => (b.low, b.open.min(b.close)),]  (4, _) => (b.low, b.open.min(b.close)),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:190  [LS1: : (5, 0) => (b.volume, 0),]  (5, 0) => (b.volume, 0),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:191  [LS1: : (5, _) => (b.volume, p.volume),]  (5, _) => (b.volume, p.volume),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:192  [LS1: : (6, 0) => (b.open_interest, p.open_interest),]  (6, 0) => (b.open_interest, p.open_interest),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:193  [LS1: : _ => (b.open_interest, 0),]  _ => (b.open_interest, 0),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:195  [LS1: : value.wrapping_sub(predictor)]  value.wrapping_sub(predictor)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:196  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:197  [LS1: : .collect()]  .collect()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:198  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:205  [LS1: : fn prototype_stream_len(residuals: &[i64]) -> usize {]  fn prototype_stream_len(residuals: &[i64]) -> usize {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:206  [LS1: : fn gcd(mut a: u64, mut b: u64) -> u64 {]  fn gcd(mut a: u64, mut b: u64) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:207  [LS1: : while b != 0 {]  while b != 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 False  [LS1: : while b != 0 {]  while b != 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:207:15 True  [LS1: : while b != 0 {]  while b != 0 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:208  [LS1: : (a, b) = (b, a % b);]  (a, b) = (b, a % b);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:209  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:210  [LS1: : a]  a
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:211  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:212  [LS1: : let n = residuals.len();]  let n = residuals.len();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:213  [LS1: : let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));]  let mut g = residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs()));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:214  [LS1: : if g == 0 || g >= 1 << 62 {]  if g == 0 || g >= 1 << 62 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 False  [LS1: : if g == 0 || g >= 1 << 62 {]  if g == 0 || g >= 1 << 62 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:214:8 True  [LS1: : if g == 0 || g >= 1 << 62 {]  if g == 0 || g >= 1 << 62 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 False  [LS1: : if g == 0 || g >= 1 << 62 {]  if g == 0 || g >= 1 << 62 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:214:18 True  [LS1: : if g == 0 || g >= 1 << 62 {]  if g == 0 || g >= 1 << 62 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:215  [LS1: : g = 1;]  g = 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:216  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:217  [LS1: : let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();]  let values: Vec<u64> = residuals.iter().map(|&r| zz(r / g.cast_signed())).collect();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:218  [LS1: : let base = values.iter().copied().min().unwrap_or(0);]  let base = values.iter().copied().min().unwrap_or(0);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:219  [LS1: : let mut histogram = [0usize; 65];]  let mut histogram = [0usize; 65];
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:220  [LS1: : for v in &values {]  for v in &values {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:221  [LS1: : histogram[(64 - (v - base).leading_zeros()) as usize] += 1;]  histogram[(64 - (v - base).leading_zeros()) as usize] += 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:222  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:223  [LS1: : let mut top = 64;]  let mut top = 64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:224  [LS1: : while top > 0 && histogram[top] == 0 {]  while top > 0 && histogram[top] == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 False  [LS1: : while top > 0 && histogram[top] == 0 {]  while top > 0 && histogram[top] == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:224:11 True  [LS1: : while top > 0 && histogram[top] == 0 {]  while top > 0 && histogram[top] == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 False  [LS1: : while top > 0 && histogram[top] == 0 {]  while top > 0 && histogram[top] == 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:224:22 True  [LS1: : while top > 0 && histogram[top] == 0 {]  while top > 0 && histogram[top] == 0 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:225  [LS1: : top -= 1;]  top -= 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:226  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:227  [LS1: : let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);]  let p = ((usize::BITS - n.saturating_sub(1).leading_zeros()) as usize).max(1);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:228  [LS1: : let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);]  let (mut best, mut best_w, mut exceptions, mut w) = (n * top, top, 0, top);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:229  [LS1: : while w > 0 {]  while w > 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 False  [LS1: : while w > 0 {]  while w > 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:229:11 True  [LS1: : while w > 0 {]  while w > 0 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:230  [LS1: : exceptions += histogram[w];]  exceptions += histogram[w];
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:231  [LS1: : w -= 1;]  w -= 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:232  [LS1: : let cost = n * w + exceptions * (p + top) + 16;]  let cost = n * w + exceptions * (p + top) + 16;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:233  [LS1: : if cost < best {]  if cost < best {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 False  [LS1: : if cost < best {]  if cost < best {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:233:12 True  [LS1: : if cost < best {]  if cost < best {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:234  [LS1: : best = cost;]  best = cost;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:235  [LS1: : best_w = w;]  best_w = w;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:236  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:238  [LS1: : let limit = if best_w == 64 {]  let limit = if best_w == 64 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 False  [LS1: : let limit = if best_w == 64 {]  let limit = if best_w == 64 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:238:20 True  [LS1: : let limit = if best_w == 64 {]  let limit = if best_w == 64 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:239  [LS1: : u64::MAX]  u64::MAX
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:241  [LS1: : (1 << best_w) - 1]  (1 << best_w) - 1
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:243  [LS1: : let exceptions = values.iter().filter(|&&v| v - base > limit).count();]  let exceptions = values.iter().filter(|&&v| v - base > limit).count();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:244  [LS1: : let mut len = 1]  let mut len = 1
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:245  [LS1: : + varint_len(exceptions as u64)]  + varint_len(exceptions as u64)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:246  [LS1: : + varint_len(base)]  + varint_len(base)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:247  [LS1: : + varint_len(g)]  + varint_len(g)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:248  [LS1: : + (n * best_w).div_ceil(8);]  + (n * best_w).div_ceil(8);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:249  [LS1: : if exceptions > 0 {]  if exceptions > 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 False  [LS1: : if exceptions > 0 {]  if exceptions > 0 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:249:8 True  [LS1: : if exceptions > 0 {]  if exceptions > 0 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:250  [LS1: : len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);]  len += 1 + (exceptions * p).div_ceil(8) + (exceptions * top).div_ceil(8);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:251  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:252  [LS1: : len]  len
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:253  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:256  [LS1: : fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {]  fn prototype_frame_len(rows: &[Bar], origin: i64) -> usize {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:257  [LS1: : let streams: usize = (0..7)]  let streams: usize = (0..7)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:258  [LS1: : .map(|column| {]  .map(|column| {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:259  [LS1: : let zero = prototype_stream_len(&residuals(rows, column, 0));]  let zero = prototype_stream_len(&residuals(rows, column, 0));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:260  [LS1: : if matches!(column, 2 | 5 | 6) {]  if matches!(column, 2 | 5 | 6) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:261  [LS1: : zero.min(prototype_stream_len(&residuals(rows, column, 1)))]  zero.min(prototype_stream_len(&residuals(rows, column, 1)))
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:263  [LS1: : zero]  zero
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:265  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:266  [LS1: : .sum();]  .sum();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:267  [LS1: : 4 + row0_len(&rows[0], origin) + streams]  4 + row0_len(&rows[0], origin) + streams
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:268  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:281  [LS1: : fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {]  fn seal(rows: &[Bar], origin: i64, label: &str) -> Sealed {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:282  [LS1: : let mut frames = Vec::with_capacity(rows.len().div_ceil(256));]  let mut frames = Vec::with_capacity(rows.len().div_ceil(256));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:283  [LS1: : let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);]  let (mut bytes, mut prototype_rule, mut first_rows) = (0, 0, 0);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:284  [LS1: : for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {]  for chunk in rows.chunks(usize::from(BLOCK_ROWS)) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:285  [LS1: : let rule = prototype_frame_len(chunk, origin);]  let rule = prototype_frame_len(chunk, origin);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:286  [LS1: : first_rows += row0_len(&chunk[0], origin) as u64;]  first_rows += row0_len(&chunk[0], origin) as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:287  [LS1: : let mut frame = Vec::new();]  let mut frame = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:288  [LS1: : let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");]  let encoded = encode_frame(chunk, origin, BLOCK_ROWS, &mut frame).expect("encodes");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:289  [LS1: : assert!(frame.len() <= MAX_FRAME_LEN, "{label}");]  assert!(frame.len() <= MAX_FRAME_LEN, "{label}");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:290  [LS1: : assert_eq!(encoded.crc, crc32c(&frame));]  assert_eq!(encoded.crc, crc32c(&frame));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:291  [LS1: : let mut back = Vec::new();]  let mut back = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:292  [LS1: : decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");]  decode_frame(&frame, encoded.crc, origin, BLOCK_ROWS, &mut back).expect("decodes");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:293  [LS1: : assert_eq!(back, chunk, "{label}: a frame came back different");]  assert_eq!(back, chunk, "{label}: a frame came back different");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:294  [LS1: : let mut stamps = Vec::new();]  let mut stamps = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:295  [LS1: : decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");]  decode_stamps(&frame, encoded.crc, origin, BLOCK_ROWS, &mut stamps).expect("stamps");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:296  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:297  [LS1: : stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),]  stamps.iter().zip(chunk).all(|(s, b)| *s == b.ts_micros),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:301  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:302  [LS1: : frame.len() <= rule,]  frame.len() <= rule,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:304  [LS1: : frame.len()]  frame.len()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:306  [LS1: : bytes += frame.len() as u64;]  bytes += frame.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:307  [LS1: : prototype_rule += rule as u64;]  prototype_rule += rule as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:308  [LS1: : frames.push((frame, encoded.crc));]  frames.push((frame, encoded.crc));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:310  [LS1: : Sealed {]  Sealed {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:311  [LS1: : bytes,]  bytes,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:312  [LS1: : prototype_rule,]  prototype_rule,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:313  [LS1: : row0: first_rows,]  row0: first_rows,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:314  [LS1: : frames,]  frames,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:315  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:316  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:318  [LS1: : fn percentiles(label: &str, mut ns: Vec<u64>) -> String {]  fn percentiles(label: &str, mut ns: Vec<u64>) -> String {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:319  [LS1: : ns.sort_unstable();]  ns.sort_unstable();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:320  [LS1: : let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];]  let at = |q: usize| ns[((ns.len() - 1) * q + 50) / 100];
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:321  [LS1: : format!(]  format!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:323  [LS1: : ns.len(),]  ns.len(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:324  [LS1: : at(50),]  at(50),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:325  [LS1: : at(99),]  at(99),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:326  [LS1: : ns.last().copied().unwrap_or(0)]  ns.last().copied().unwrap_or(0)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:328  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:331  [LS1: : fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {]  fn time(rows: &[Bar], origin: i64, sealed: &Sealed, label: &str) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:332  [LS1: : let mut decode = Vec::new();]  let mut decode = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:333  [LS1: : let mut stamps = Vec::new();]  let mut stamps = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:334  [LS1: : let mut out = Vec::with_capacity(512);]  let mut out = Vec::with_capacity(512);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:335  [LS1: : let mut ts = Vec::with_capacity(256);]  let mut ts = Vec::with_capacity(256);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:336  [LS1: : for _ in 0..(2_000 / sealed.frames.len()).max(1) {]  for _ in 0..(2_000 / sealed.frames.len()).max(1) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:337  [LS1: : for (frame, crc) in &sealed.frames {]  for (frame, crc) in &sealed.frames {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:338  [LS1: : out.clear();]  out.clear();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:339  [LS1: : let t = Instant::now();]  let t = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:340  [LS1: : decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");]  decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:341  [LS1: : decode.push(t.elapsed().as_nanos() as u64);]  decode.push(t.elapsed().as_nanos() as u64);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:342  [LS1: : ts.clear();]  ts.clear();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:343  [LS1: : let t = Instant::now();]  let t = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:344  [LS1: : decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");]  decode_stamps(frame, *crc, origin, BLOCK_ROWS, &mut ts).expect("stamps");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:345  [LS1: : stamps.push(t.elapsed().as_nanos() as u64);]  stamps.push(t.elapsed().as_nanos() as u64);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:346  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:348  [LS1: : let mut encode = Vec::new();]  let mut encode = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:349  [LS1: : let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);]  let mut buffer = Vec::with_capacity(MAX_FRAME_LEN);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:350  [LS1: : for chunk in rows.chunks(256).take(2_000) {]  for chunk in rows.chunks(256).take(2_000) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:351  [LS1: : buffer.clear();]  buffer.clear();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:352  [LS1: : let t = Instant::now();]  let t = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:353  [LS1: : encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");]  encode_frame(chunk, origin, BLOCK_ROWS, &mut buffer).expect("encodes");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:354  [LS1: : encode.push(t.elapsed().as_nanos() as u64);]  encode.push(t.elapsed().as_nanos() as u64);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:355  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:357  [LS1: : let mut page = Vec::new();]  let mut page = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:358  [LS1: : let mut state = 0x9E37_79B9_7F4A_7C15_u64;]  let mut state = 0x9E37_79B9_7F4A_7C15_u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:359  [LS1: : let n = rows.len() as u64;]  let n = rows.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:360  [LS1: : for _ in 0..2_000 {]  for _ in 0..2_000 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:361  [LS1: : state ^= state << 13;]  state ^= state << 13;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:362  [LS1: : state ^= state >> 7;]  state ^= state >> 7;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:363  [LS1: : state ^= state << 17;]  state ^= state << 17;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:364  [LS1: : let first = state % n;]  let first = state % n;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:365  [LS1: : let last = (first + 99).min(n - 1);]  let last = (first + 99).min(n - 1);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:366  [LS1: : out.clear();]  out.clear();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:367  [LS1: : let t = Instant::now();]  let t = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:368  [LS1: : for k in (first / 256)..=(last / 256) {]  for k in (first / 256)..=(last / 256) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:369  [LS1: : let (frame, crc) = &sealed.frames[k as usize];]  let (frame, crc) = &sealed.frames[k as usize];
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:370  [LS1: : decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");]  decode_frame(frame, *crc, origin, BLOCK_ROWS, &mut out).expect("decodes");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:371  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:372  [LS1: : page.push(t.elapsed().as_nanos() as u64);]  page.push(t.elapsed().as_nanos() as u64);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:373  [LS1: : let skip = (first % 256) as usize;]  let skip = (first % 256) as usize;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:374  [LS1: : assert_eq!(out[skip], rows[first as usize]);]  assert_eq!(out[skip], rows[first as usize]);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:376  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:378  [LS1: : percentiles(&format!("{label} frame decode+crc"), decode)]  percentiles(&format!("{label} frame decode+crc"), decode)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:380  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:382  [LS1: : percentiles(&format!("{label} frame stamps+crc"), stamps)]  percentiles(&format!("{label} frame stamps+crc"), stamps)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:384  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:386  [LS1: : percentiles(&format!("{label} frame encode"), encode)]  percentiles(&format!("{label} frame encode"), encode)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:388  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:390  [LS1: : percentiles(&format!("{label} page100 from memory"), page)]  percentiles(&format!("{label} page100 from memory"), page)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:392  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:409  [LS1: : fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {]  fn add(&mut self, label: &str, bars: usize, sealed: &Sealed, exact: u64, judge: u64) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:410  [LS1: : if sealed.prototype_rule != exact {]  if sealed.prototype_rule != exact {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 False  [LS1: : if sealed.prototype_rule != exact {]  if sealed.prototype_rule != exact {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:410:12 True  [LS1: : if sealed.prototype_rule != exact {]  if sealed.prototype_rule != exact {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:411  [LS1: : self.not_exact.push(format!(]  self.not_exact.push(format!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:412  [LS1: : "{label}: rule {} exact {exact}",]  "{label}: rule {} exact {exact}",
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:413  [LS1: : sealed.prototype_rule]  sealed.prototype_rule
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:414  [LS1: : ));]  ));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:415  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:416  [LS1: : if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {]  if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 False  [LS1: : if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {]  if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:416:12 True  [LS1: : if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {]  if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 False  [LS1: : if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {]  if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:416:29 True  [LS1: : if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {]  if bars >= 1_024 && sealed.bytes * 200 > judge * 201 {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:417  [LS1: : self.larger]  self.larger
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:418  [LS1: : .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));]  .push(format!("{label}: codec1 {} judge {judge}", sealed.bytes));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:419  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:420  [LS1: : self.codec1 += sealed.bytes;]  self.codec1 += sealed.bytes;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:421  [LS1: : self.rule += sealed.prototype_rule;]  self.rule += sealed.prototype_rule;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:422  [LS1: : self.exact += exact;]  self.exact += exact;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:423  [LS1: : self.judge += judge;]  self.judge += judge;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:424  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:426  [LS1: : fn check(&self, label: &str) {]  fn check(&self, label: &str) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:427  [LS1: : let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);]  let d = |x: u64, of: u64| per_cent(x.cast_signed() - of.cast_signed(), of);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:428  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:434  [LS1: : d(self.codec1, self.exact),]  d(self.codec1, self.exact),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:435  [LS1: : d(self.codec1, self.judge),]  d(self.codec1, self.judge),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:436  [LS1: : self.not_exact.len(),]  self.not_exact.len(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:437  [LS1: : self.larger.len()]  self.larger.len()
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:440  [LS1: : assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);]  assert!(self.not_exact.is_empty(), "{label}: {:?}", self.not_exact);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:441  [LS1: : assert_eq!(self.rule, self.exact, "{label}");]  assert_eq!(self.rule, self.exact, "{label}");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:443  [LS1: : assert!(self.codec1 <= self.rule, "{label}");]  assert!(self.codec1 <= self.rule, "{label}");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:444  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:445  [LS1: : within(self.codec1, self.exact).0,]  within(self.codec1, self.exact).0,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:450  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:451  [LS1: : within(self.codec1, self.judge).0,]  within(self.codec1, self.judge).0,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:454  [LS1: : assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);]  assert!(self.larger.is_empty(), "{label}: {:?}", self.larger);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:455  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:459  [LS1: : fn per_cent(diff: i64, of: u64) -> String {]  fn per_cent(diff: i64, of: u64) -> String {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:460  [LS1: : let thousandths = diff.unsigned_abs() * 100_000 / of;]  let thousandths = diff.unsigned_abs() * 100_000 / of;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:461  [LS1: : let sign = if diff < 0 { "-" } else { "+" };]  let sign = if diff < 0 { "-" } else { "+" };
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 False  [LS1: : let sign = if diff < 0 { "-" } else { "+" };]  let sign = if diff < 0 { "-" } else { "+" };
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:461:19 True  [LS1: : let sign = if diff < 0 { "-" } else { "+" };]  let sign = if diff < 0 { "-" } else { "+" };
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:462  [LS1: : format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)]  format!("{sign}{}.{:03}%", thousandths / 1_000, thousandths % 1_000)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:463  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:466  [LS1: : fn within(mine: u64, expected: u64) -> (bool, i64) {]  fn within(mine: u64, expected: u64) -> (bool, i64) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:467  [LS1: : let diff = mine.cast_signed() - expected.cast_signed();]  let diff = mine.cast_signed() - expected.cast_signed();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:469  [LS1: : (diff.unsigned_abs() * 200 <= expected, diff)]  (diff.unsigned_abs() * 200 <= expected, diff)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:470  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:474  [LS1: : fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {]  fn the_sample_months_encode_to_the_prototype_sizes_less_the_first_row_saving() {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:475  [LS1: : let root = std::env::var("BRUTEX_SEAL_SAMPLES")]  let root = std::env::var("BRUTEX_SEAL_SAMPLES")
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:476  [LS1: : .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");]  .expect("BRUTEX_SEAL_SAMPLES names the store-compression evidence directory");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:477  [LS1: : let root = Path::new(&root);]  let root = Path::new(&root);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:488  [LS1: : println!("eight sample months:");]  println!("eight sample months:");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:489  [LS1: : let mut totals = Totals::default();]  let mut totals = Totals::default();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:490  [LS1: : for (dir, month, body, saved) in SAMPLES {]  for (dir, month, body, saved) in SAMPLES {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:491  [LS1: : let path = root]  let path = root
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:492  [LS1: : .join("blocks/samples")]  .join("blocks/samples")
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:493  [LS1: : .join(dir)]  .join(dir)
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:494  [LS1: : .join(format!("{month}.bin"));]  .join(format!("{month}.bin"));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:495  [LS1: : let rows = read_month(&path);]  let rows = read_month(&path);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:496  [LS1: : let origin = origin_of(month);]  let origin = origin_of(month);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:497  [LS1: : let sealed = seal(&rows, origin, dir);]  let sealed = seal(&rows, origin, dir);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:498  [LS1: : let blocks = sealed.frames.len() as u64;]  let blocks = sealed.frames.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:499  [LS1: : let exact = body - 56 * blocks + sealed.row0;]  let exact = body - 56 * blocks + sealed.row0;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:500  [LS1: : let judge = body - saved;]  let judge = body - saved;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:501  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:502  [LS1: : "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",]  "{dir} {month}: bars={} frames={blocks} codec1={} rule={} exact={exact} judge={judge} codec1-judge={} raw={} ({} B/bar x100)",
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:503  [LS1: : rows.len(),]  rows.len(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:504  [LS1: : sealed.bytes,]  sealed.bytes,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:505  [LS1: : sealed.prototype_rule,]  sealed.prototype_rule,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:506  [LS1: : per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),]  per_cent(sealed.bytes.cast_signed() - judge.cast_signed(), judge),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:507  [LS1: : rows.len() * 56,]  rows.len() * 56,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:508  [LS1: : sealed.bytes * 100 / rows.len() as u64,]  sealed.bytes * 100 / rows.len() as u64,
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:509  [LS1: : );]  );
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:510  [LS1: : time(&rows, origin, &sealed, dir);]  time(&rows, origin, &sealed, dir);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:511  [LS1: : totals.add(dir, rows.len(), &sealed, exact, judge);]  totals.add(dir, rows.len(), &sealed, exact, judge);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:512  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:513  [LS1: : totals.check("eight samples in all");]  totals.check("eight samples in all");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:515  [LS1: : let manifest =]  let manifest =
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:516  [LS1: : std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");]  std::fs::read_to_string(root.join("blocks/proto/strat-manifest.tsv")).expect("manifest");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:517  [LS1: : let out =]  let out =
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:518  [LS1: : std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");]  std::fs::read_to_string(root.join("blocks/proto/strat-out.tsv")).expect("prototype output");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:519  [LS1: : let mut totals = Totals::default();]  let mut totals = Totals::default();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:520  [LS1: : let (mut months, mut bars, mut empty) = (0, 0, 0);]  let (mut months, mut bars, mut empty) = (0, 0, 0);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:521  [LS1: : for (line, result) in manifest.lines().zip(out.lines()) {]  for (line, result) in manifest.lines().zip(out.lines()) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:522  [LS1: : let fields: Vec<&str> = line.split('\t').collect();]  let fields: Vec<&str> = line.split('\t').collect();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:523  [LS1: : let measured: Vec<&str> = result.split('\t').collect();]  let measured: Vec<&str> = result.split('\t').collect();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:524  [LS1: : let rows = read_month(&root.join("blocks").join(fields[0]));]  let rows = read_month(&root.join("blocks").join(fields[0]));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:525  [LS1: : assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);]  assert_eq!(rows.len().to_string(), measured[2], "{}", fields[4]);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:526  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 False  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:526:12 True  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:527  [LS1: : empty += 1;]  empty += 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:528  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:529  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:530  [LS1: : let origin = origin_of(fields[4]);]  let origin = origin_of(fields[4]);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:531  [LS1: : let sealed = seal(&rows, origin, fields[4]);]  let sealed = seal(&rows, origin, fields[4]);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:532  [LS1: : let body: u64 = measured[3].parse().expect("body");]  let body: u64 = measured[3].parse().expect("body");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:533  [LS1: : let blocks = sealed.frames.len() as u64;]  let blocks = sealed.frames.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:534  [LS1: : let exact = body - 56 * blocks + sealed.row0;]  let exact = body - 56 * blocks + sealed.row0;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:535  [LS1: : let judge = body - (56 * blocks - judge_row0(&rows));]  let judge = body - (56 * blocks - judge_row0(&rows));
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:536  [LS1: : totals.add(fields[4], rows.len(), &sealed, exact, judge);]  totals.add(fields[4], rows.len(), &sealed, exact, judge);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:537  [LS1: : months += 1;]  months += 1;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:538  [LS1: : bars += rows.len();]  bars += rows.len();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:540  [LS1: : println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");]  println!("539-month manifest: {months} months sealed, {empty} empty, {bars} bars");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:541  [LS1: : totals.check("the manifest in all");]  totals.check("the manifest in all");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:542  [LS1: : if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {]  if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 False  [LS1: : if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {]  if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:542:12 True  [LS1: : if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {]  if let Ok(extra) = std::env::var("BRUTEX_SEAL_EXTRA") {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:543  [LS1: : println!("extra months (read only, round trip and size):");]  println!("extra months (read only, round trip and size):");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:544  [LS1: : for path in extra.lines().filter(|l| !l.trim().is_empty()) {]  for path in extra.lines().filter(|l| !l.trim().is_empty()) {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:545  [LS1: : let path = Path::new(path.trim());]  let path = Path::new(path.trim());
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:546  [LS1: : let rows = read_month(path);]  let rows = read_month(path);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:547  [LS1: : let name = path.to_string_lossy();]  let name = path.to_string_lossy();
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:548  [LS1: : let origin = origin_of(&name);]  let origin = origin_of(&name);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:549  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 False  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     BRANCH crates/store/tests/sealed_codec_samples.rs:549:16 True  [LS1: : if rows.is_empty() {]  if rows.is_empty() {
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:550  [LS1: : println!("{name}: empty month");]  println!("{name}: empty month");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:551  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:552  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:553  [LS1: : let sealed = seal(&rows, origin, &name);]  let sealed = seal(&rows, origin, &name);
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:554  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:556  [LS1: : rows.len(),]  rows.len(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:557  [LS1: : sealed.frames.len(),]  sealed.frames.len(),
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:559  [LS1: : rows.len() * 56]  rows.len() * 56
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:561  [LS1: : time(&rows, origin, &sealed, "extra");]  time(&rows, origin, &sealed, "extra");
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:563  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_codec_samples.rs:564  [LS1: : }]  }
(F)         BRANCH crates/store/tests/sealed_door.rs:153:17 True  [LS1]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F)         BRANCH crates/store/tests/sealed_door.rs:153:31 False  [LS1]  assert!(at == rows || bars[at as usize].ts_micros >= probe);
(F)         BRANCH crates/store/tests/sealed_door.rs:231:9 False  [LS1]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
(F)         BRANCH crates/store/tests/sealed_door.rs:231:45 False  [LS1]  large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
IGNORED     LINE   crates/store/tests/sealed_door.rs:237  [LS1: : fn store_path(rel: &str) -> StorePath<'static> {]  fn store_path(rel: &str) -> StorePath<'static> {
IGNORED     LINE   crates/store/tests/sealed_door.rs:238  [LS1: : let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());]  let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
IGNORED     LINE   crates/store/tests/sealed_door.rs:239  [LS1: : let parts: Vec<&'static str> = rel.split('/').collect();]  let parts: Vec<&'static str> = rel.split('/').collect();
IGNORED     LINE   crates/store/tests/sealed_door.rs:240  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:241  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_door.rs:241:9 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_door.rs:241:9 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_door.rs:241:29 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_door.rs:241:29 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     LINE   crates/store/tests/sealed_door.rs:244  [LS1: : assert_eq!(parts[0], "bars", "{rel}");]  assert_eq!(parts[0], "bars", "{rel}");
IGNORED     LINE   crates/store/tests/sealed_door.rs:245  [LS1: : let vendor = Vendor::ALL]  let vendor = Vendor::ALL
IGNORED     LINE   crates/store/tests/sealed_door.rs:246  [LS1: : .into_iter()]  .into_iter()
IGNORED     LINE   crates/store/tests/sealed_door.rs:247  [LS1: : .find(|v| v.as_str() == parts[1])]  .find(|v| v.as_str() == parts[1])
IGNORED     LINE   crates/store/tests/sealed_door.rs:248  [LS1: : .unwrap_or_else(|| panic!("{rel}: vendor"));]  .unwrap_or_else(|| panic!("{rel}: vendor"));
IGNORED     LINE   crates/store/tests/sealed_door.rs:249  [LS1: : let contract = (parts.len() == 8)]  let contract = (parts.len() == 8)
IGNORED     LINE   crates/store/tests/sealed_door.rs:250  [LS1: : .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));]  .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
IGNORED     LINE   crates/store/tests/sealed_door.rs:251  [LS1: : let tf = parts[parts.len() - 2];]  let tf = parts[parts.len() - 2];
IGNORED     LINE   crates/store/tests/sealed_door.rs:252  [LS1: : let timeframe = *Timeframe::KNOWN]  let timeframe = *Timeframe::KNOWN
IGNORED     LINE   crates/store/tests/sealed_door.rs:253  [LS1: : .iter()]  .iter()
IGNORED     LINE   crates/store/tests/sealed_door.rs:254  [LS1: : .find(|t| t.as_str() == tf)]  .find(|t| t.as_str() == tf)
IGNORED     LINE   crates/store/tests/sealed_door.rs:255  [LS1: : .unwrap_or_else(|| panic!("{rel}: rung"));]  .unwrap_or_else(|| panic!("{rel}: rung"));
IGNORED     LINE   crates/store/tests/sealed_door.rs:256  [LS1: : let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");]  let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
IGNORED     LINE   crates/store/tests/sealed_door.rs:257  [LS1: : let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();]  let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:258  [LS1: : StorePath::new(PathParts {]  StorePath::new(PathParts {
IGNORED     LINE   crates/store/tests/sealed_door.rs:259  [LS1: : vendor,]  vendor,
IGNORED     LINE   crates/store/tests/sealed_door.rs:260  [LS1: : exchange: parts[2],]  exchange: parts[2],
IGNORED     LINE   crates/store/tests/sealed_door.rs:261  [LS1: : segment: parts[3],]  segment: parts[3],
IGNORED     LINE   crates/store/tests/sealed_door.rs:262  [LS1: : symbol: parts[4],]  symbol: parts[4],
IGNORED     LINE   crates/store/tests/sealed_door.rs:263  [LS1: : contract,]  contract,
IGNORED     LINE   crates/store/tests/sealed_door.rs:264  [LS1: : timeframe,]  timeframe,
IGNORED     LINE   crates/store/tests/sealed_door.rs:265  [LS1: : month,]  month,
IGNORED     LINE   crates/store/tests/sealed_door.rs:266  [LS1: : file: FileKind::Bars,]  file: FileKind::Bars,
IGNORED     LINE   crates/store/tests/sealed_door.rs:267  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_door.rs:268  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/sealed_door.rs:269  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_door.rs:280  [LS1: : fn seal_a_private_store_root_for_the_shadow_check() {]  fn seal_a_private_store_root_for_the_shadow_check() {
IGNORED     LINE   crates/store/tests/sealed_door.rs:281  [LS1: : let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));]  let root = PathBuf::from(std::env::var("BRUTEX_SC3_ROOT").expect("BRUTEX_SC3_ROOT"));
IGNORED     LINE   crates/store/tests/sealed_door.rs:282  [LS1: : let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");]  let list = std::env::var("BRUTEX_SC3_MONTHS").expect("BRUTEX_SC3_MONTHS");
IGNORED     LINE   crates/store/tests/sealed_door.rs:283  [LS1: : let bars_dir = root.join("bars");]  let bars_dir = root.join("bars");
IGNORED     LINE   crates/store/tests/sealed_door.rs:284  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:285  [LS1: : !fs::symlink_metadata(&bars_dir)]  !fs::symlink_metadata(&bars_dir)
IGNORED     LINE   crates/store/tests/sealed_door.rs:286  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/sealed_door.rs:287  [LS1: : .file_type()]  .file_type()
IGNORED     LINE   crates/store/tests/sealed_door.rs:288  [LS1: : .is_symlink(),]  .is_symlink(),
IGNORED     LINE   crates/store/tests/sealed_door.rs:290  [LS1: : bars_dir.display()]  bars_dir.display()
IGNORED     LINE   crates/store/tests/sealed_door.rs:292  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:293  [LS1: : !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),]  !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
IGNORED     LINE   crates/store/tests/sealed_door.rs:296  [LS1: : let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);]  let (mut months, mut rows, mut raw_bytes, mut sealed_bytes) = (0u64, 0u64, 0u64, 0u64);
IGNORED     LINE   crates/store/tests/sealed_door.rs:297  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_door.rs:298  [LS1: : for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {]  for rel in fs::read_to_string(&list).unwrap().lines().map(str::trim) {
IGNORED     LINE   crates/store/tests/sealed_door.rs:299  [LS1: : if rel.is_empty() || rel.starts_with('#') {]  if rel.is_empty() || rel.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:299:12 False  [LS1: : if rel.is_empty() || rel.starts_with('#') {]  if rel.is_empty() || rel.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:299:12 True  [LS1: : if rel.is_empty() || rel.starts_with('#') {]  if rel.is_empty() || rel.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:299:30 False  [LS1: : if rel.is_empty() || rel.starts_with('#') {]  if rel.is_empty() || rel.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:299:30 True  [LS1: : if rel.is_empty() || rel.starts_with('#') {]  if rel.is_empty() || rel.starts_with('#') {
IGNORED     LINE   crates/store/tests/sealed_door.rs:300  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_door.rs:301  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_door.rs:302  [LS1: : let at = store_path(rel);]  let at = store_path(rel);
IGNORED     LINE   crates/store/tests/sealed_door.rs:303  [LS1: : let symbol = rel.split('/').nth(4).unwrap().to_owned();]  let symbol = rel.split('/').nth(4).unwrap().to_owned();
IGNORED     LINE   crates/store/tests/sealed_door.rs:304  [LS1: : let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;]  let symbol_id = brutex_core::universe::fnv1a(&symbol) as u32;
IGNORED     LINE   crates/store/tests/sealed_door.rs:305  [LS1: : let served = store::generation::served(&root, at).unwrap();]  let served = store::generation::served(&root, at).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:306  [LS1: : let served_root = served.root(&root);]  let served_root = served.root(&root);
IGNORED     LINE   crates/store/tests/sealed_door.rs:307  [LS1: : let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);]  let sbar = at.with_file(FileKind::SealedBars).to_path_buf(&served_root);
IGNORED     LINE   crates/store/tests/sealed_door.rs:308  [LS1: : if sbar.exists() {]  if sbar.exists() {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:308:12 False  [LS1: : if sbar.exists() {]  if sbar.exists() {
IGNORED     BRANCH crates/store/tests/sealed_door.rs:308:12 True  [LS1: : if sbar.exists() {]  if sbar.exists() {
IGNORED     LINE   crates/store/tests/sealed_door.rs:309  [LS1: : println!("SKIP {rel}: already sealed");]  println!("SKIP {rel}: already sealed");
IGNORED     LINE   crates/store/tests/sealed_door.rs:310  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_door.rs:311  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_door.rs:312  [LS1: : let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();]  let raw = BarFile::open_existing(&root, at, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:313  [LS1: : assert_eq!(raw.form(), Form::Raw, "{rel}");]  assert_eq!(raw.form(), Form::Raw, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_door.rs:314  [LS1: : let bars = raw.read_range(0, raw.records()).unwrap();]  let bars = raw.read_range(0, raw.records()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:315  [LS1: : let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();]  let prv = fs::read(at.with_file(FileKind::Provenance).to_path_buf(&served_root)).ok();
IGNORED     LINE   crates/store/tests/sealed_door.rs:316  [LS1: : let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();]  let built = store::sealed::build(at, &raw.header(), &bars, prv.as_deref(), None).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:317  [LS1: : let staged = sbar.with_extension("sbar-staged");]  let staged = sbar.with_extension("sbar-staged");
IGNORED     LINE   crates/store/tests/sealed_door.rs:318  [LS1: : let mut file = fs::File::create_new(&staged).unwrap();]  let mut file = fs::File::create_new(&staged).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:319  [LS1: : built.write_to(&mut file).unwrap();]  built.write_to(&mut file).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:320  [LS1: : file.sync_all().unwrap();]  file.sync_all().unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:321  [LS1: : drop(file);]  drop(file);
IGNORED     LINE   crates/store/tests/sealed_door.rs:322  [LS1: : fs::rename(&staged, &sbar).unwrap();]  fs::rename(&staged, &sbar).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:323  [LS1: : let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();]  let sealed = BarFile::open_existing(&root, at, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_door.rs:324  [LS1: : assert_eq!(sealed.form(), Form::Sealed, "{rel}");]  assert_eq!(sealed.form(), Form::Sealed, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_door.rs:325  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:326  [LS1: : sealed.read_range(0, sealed.records()).unwrap(),]  sealed.read_range(0, sealed.records()).unwrap(),
IGNORED     LINE   crates/store/tests/sealed_door.rs:330  [LS1: : months += 1;]  months += 1;
IGNORED     LINE   crates/store/tests/sealed_door.rs:331  [LS1: : rows += bars.len() as u64;]  rows += bars.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_door.rs:332  [LS1: : raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();]  raw_bytes += fs::metadata(at.to_path_buf(&served_root)).unwrap().len();
IGNORED     LINE   crates/store/tests/sealed_door.rs:333  [LS1: : sealed_bytes += built.file_len();]  sealed_bytes += built.file_len();
IGNORED     LINE   crates/store/tests/sealed_door.rs:334  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:336  [LS1: : bars.len(),]  bars.len(),
IGNORED     LINE   crates/store/tests/sealed_door.rs:337  [LS1: : built.file_len(),]  built.file_len(),
IGNORED     LINE   crates/store/tests/sealed_door.rs:338  [LS1: : served_root.display()]  served_root.display()
IGNORED     LINE   crates/store/tests/sealed_door.rs:341  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_door.rs:344  [LS1: : started.elapsed().as_secs_f64()]  started.elapsed().as_secs_f64()
IGNORED     LINE   crates/store/tests/sealed_door.rs:346  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:49  [LS1: : fn next(&mut self) -> u64 {]  fn next(&mut self) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:50  [LS1: : self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);]  self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:51  [LS1: : let mut z = self.0;]  let mut z = self.0;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:52  [LS1: : z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);]  z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:53  [LS1: : z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);]  z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:54  [LS1: : z ^ (z >> 31)]  z ^ (z >> 31)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:55  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:56  [LS1: : fn below(&mut self, n: u64) -> u64 {]  fn below(&mut self, n: u64) -> u64 {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:57  [LS1: : self.next() % n]  self.next() % n
IGNORED     LINE   crates/store/tests/sealed_samples.rs:58  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:62  [LS1: : fn store_path(rel: &str) -> StorePath<'static> {]  fn store_path(rel: &str) -> StorePath<'static> {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:63  [LS1: : let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());]  let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:64  [LS1: : let parts: Vec<&'static str> = rel.split('/').collect();]  let parts: Vec<&'static str> = rel.split('/').collect();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:65  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_samples.rs:66  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:66:9 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:66:9 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:66:29 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:66:29 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:69  [LS1: : assert_eq!(parts[0], "bars", "{rel}");]  assert_eq!(parts[0], "bars", "{rel}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:70  [LS1: : let vendor = Vendor::ALL]  let vendor = Vendor::ALL
IGNORED     LINE   crates/store/tests/sealed_samples.rs:71  [LS1: : .into_iter()]  .into_iter()
IGNORED     LINE   crates/store/tests/sealed_samples.rs:72  [LS1: : .find(|v| v.as_str() == parts[1])]  .find(|v| v.as_str() == parts[1])
IGNORED     LINE   crates/store/tests/sealed_samples.rs:73  [LS1: : .unwrap_or_else(|| panic!("{rel}: vendor"));]  .unwrap_or_else(|| panic!("{rel}: vendor"));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:74  [LS1: : let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));]  let contract = (parts.len() == 8).then(|| Contract::parse(parts[5]).expect("contract"));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:75  [LS1: : let tf = parts[parts.len() - 2];]  let tf = parts[parts.len() - 2];
IGNORED     LINE   crates/store/tests/sealed_samples.rs:76  [LS1: : let timeframe = *Timeframe::KNOWN]  let timeframe = *Timeframe::KNOWN
IGNORED     LINE   crates/store/tests/sealed_samples.rs:77  [LS1: : .iter()]  .iter()
IGNORED     LINE   crates/store/tests/sealed_samples.rs:78  [LS1: : .find(|t| t.as_str() == tf)]  .find(|t| t.as_str() == tf)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:79  [LS1: : .unwrap_or_else(|| panic!("{rel}: rung"));]  .unwrap_or_else(|| panic!("{rel}: rung"));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:80  [LS1: : let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");]  let stem = parts[parts.len() - 1].strip_suffix(".bin").expect(".bin");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:81  [LS1: : let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();]  let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:82  [LS1: : StorePath::new(PathParts {]  StorePath::new(PathParts {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:83  [LS1: : vendor,]  vendor,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:84  [LS1: : exchange: parts[2],]  exchange: parts[2],
IGNORED     LINE   crates/store/tests/sealed_samples.rs:85  [LS1: : segment: parts[3],]  segment: parts[3],
IGNORED     LINE   crates/store/tests/sealed_samples.rs:86  [LS1: : symbol: parts[4],]  symbol: parts[4],
IGNORED     LINE   crates/store/tests/sealed_samples.rs:87  [LS1: : contract,]  contract,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:88  [LS1: : timeframe,]  timeframe,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:89  [LS1: : month,]  month,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:90  [LS1: : file: FileKind::Bars,]  file: FileKind::Bars,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:91  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_samples.rs:92  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/sealed_samples.rs:93  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:95  [LS1: : fn raw_month(file: &Path) -> (Header, Vec<Bar>) {]  fn raw_month(file: &Path) -> (Header, Vec<Bar>) {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:96  [LS1: : let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));]  let data = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:97  [LS1: : let at = HEADER_LEN as usize;]  let at = HEADER_LEN as usize;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:98  [LS1: : let header = Header::read_region(&data[..at], data.len() as u64).unwrap();]  let header = Header::read_region(&data[..at], data.len() as u64).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:99  [LS1: : let n = header.n_valid as usize;]  let n = header.n_valid as usize;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:100  [LS1: : let bars = data[at..at + n * RECORD_LEN]]  let bars = data[at..at + n * RECORD_LEN]
IGNORED     LINE   crates/store/tests/sealed_samples.rs:101  [LS1: : .chunks_exact(RECORD_LEN)]  .chunks_exact(RECORD_LEN)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:102  [LS1: : .map(|record| Bar::decode(record).unwrap())]  .map(|record| Bar::decode(record).unwrap())
IGNORED     LINE   crates/store/tests/sealed_samples.rs:103  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:104  [LS1: : (header, bars)]  (header, bars)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:105  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:108  [LS1: : fn us(ns: u128) -> String {]  fn us(ns: u128) -> String {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:109  [LS1: : format!("{}.{}", ns / 1_000, ns % 1_000 / 100)]  format!("{}.{}", ns / 1_000, ns % 1_000 / 100)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:110  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:112  [LS1: : fn percentiles(label: &str, mut ns: Vec<u128>) -> String {]  fn percentiles(label: &str, mut ns: Vec<u128>) -> String {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:113  [LS1: : ns.sort_unstable();]  ns.sort_unstable();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:114  [LS1: : let n = ns.len();]  let n = ns.len();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:115  [LS1: : format!(]  format!(
IGNORED     LINE   crates/store/tests/sealed_samples.rs:117  [LS1: : us(ns[n / 2]),]  us(ns[n / 2]),
IGNORED     LINE   crates/store/tests/sealed_samples.rs:118  [LS1: : us(ns[n * 99 / 100]),]  us(ns[n * 99 / 100]),
IGNORED     LINE   crates/store/tests/sealed_samples.rs:119  [LS1: : us(ns[n - 1])]  us(ns[n - 1])
IGNORED     LINE   crates/store/tests/sealed_samples.rs:121  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:125  [LS1: : fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {]  fn real_months_seal_read_back_bit_for_bit_and_answer_like_the_raw_form() {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:126  [LS1: : let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));]  let root = PathBuf::from(std::env::var("BRUTEX_SEALED_ROOT").expect("BRUTEX_SEALED_ROOT"));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:127  [LS1: : let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");]  let list = std::env::var("BRUTEX_SEALED_MONTHS").expect("BRUTEX_SEALED_MONTHS");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:128  [LS1: : let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));]  let out = PathBuf::from(std::env::var("BRUTEX_SEALED_OUT").expect("BRUTEX_SEALED_OUT"));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:129  [LS1: : let months: Vec<String> = std::fs::read_to_string(&list)]  let months: Vec<String> = std::fs::read_to_string(&list)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:130  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/sealed_samples.rs:131  [LS1: : .lines()]  .lines()
IGNORED     LINE   crates/store/tests/sealed_samples.rs:132  [LS1: : .map(str::trim)]  .map(str::trim)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:133  [LS1: : .filter(|line| !line.is_empty() && !line.starts_with('#'))]  .filter(|line| !line.is_empty() && !line.starts_with('#'))
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:133:24 False  [LS1: : .filter(|line| !line.is_empty() && !line.starts_with('#'))]  .filter(|line| !line.is_empty() && !line.starts_with('#'))
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:133:24 True  [LS1: : .filter(|line| !line.is_empty() && !line.starts_with('#'))]  .filter(|line| !line.is_empty() && !line.starts_with('#'))
IGNORED     LINE   crates/store/tests/sealed_samples.rs:134  [LS1: : .map(str::to_owned)]  .map(str::to_owned)
IGNORED     LINE   crates/store/tests/sealed_samples.rs:135  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:136  [LS1: : assert!(!months.is_empty(), "{list}: no months");]  assert!(!months.is_empty(), "{list}: no months");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:137  [LS1: : let mut rng = Rng(2026);]  let mut rng = Rng(2026);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:138  [LS1: : for rel in &months {]  for rel in &months {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:139  [LS1: : let path = store_path(rel);]  let path = store_path(rel);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:140  [LS1: : let bin = path.to_path_buf(&root);]  let bin = path.to_path_buf(&root);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:141  [LS1: : let (header, bars) = raw_month(&bin);]  let (header, bars) = raw_month(&bin);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:142  [LS1: : let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();]  let prv = std::fs::read(path.with_file(FileKind::Provenance).to_path_buf(&root)).ok();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:143  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:144  [LS1: : let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();]  let built = build(path, &header, &bars, prv.as_deref(), None).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:145  [LS1: : let seal_us = started.elapsed().as_micros();]  let seal_us = started.elapsed().as_micros();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:146  [LS1: : let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);]  let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&out);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:147  [LS1: : std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();]  std::fs::create_dir_all(sbar.parent().unwrap()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:148  [LS1: : let mut file = std::fs::File::create(&sbar).unwrap();]  let mut file = std::fs::File::create(&sbar).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:149  [LS1: : built.write_to(&mut file).unwrap();]  built.write_to(&mut file).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:150  [LS1: : file.sync_all().unwrap();]  file.sync_all().unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:151  [LS1: : drop(file);]  drop(file);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:153  [LS1: : let symbol = header.symbol_id;]  let symbol = header.symbol_id;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:154  [LS1: : let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();]  let sealed = SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:155  [LS1: : let audited = sealed.audit().unwrap();]  let audited = sealed.audit().unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:156  [LS1: : assert_eq!(audited.rows, bars.len() as u64, "{rel}");]  assert_eq!(audited.rows, bars.len() as u64, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:157  [LS1: : assert_eq!(sealed.head().bar_header(), header, "{rel}");]  assert_eq!(sealed.head().bar_header(), header, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:158  [LS1: : assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");]  assert_eq!(sealed.provenance().unwrap(), prv, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:159  [LS1: : let raw = BarFile::open_existing(&root, path, symbol).unwrap();]  let raw = BarFile::open_existing(&root, path, symbol).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:160  [LS1: : let n = bars.len() as u64;]  let n = bars.len() as u64;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:161  [LS1: : let mut page = Vec::new();]  let mut page = Vec::new();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:162  [LS1: : let mut first = 0;]  let mut first = 0;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:163  [LS1: : while first < n {]  while first < n {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:163:15 False  [LS1: : while first < n {]  while first < n {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:163:15 True  [LS1: : while first < n {]  while first < n {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:164  [LS1: : let count = (n - first).min(1_000);]  let count = (n - first).min(1_000);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:165  [LS1: : page.clear();]  page.clear();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:166  [LS1: : sealed.read_rows(first, count, &mut page).unwrap();]  sealed.read_rows(first, count, &mut page).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:167  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_samples.rs:169  [LS1: : bars[first as usize..(first + count) as usize],]  bars[first as usize..(first + count) as usize],
IGNORED     LINE   crates/store/tests/sealed_samples.rs:172  [LS1: : first += count;]  first += count;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:174  [LS1: : for at in (0..n).step_by(1 + n as usize / 500) {]  for at in (0..n).step_by(1 + n as usize / 500) {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:175  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_samples.rs:176  [LS1: : raw.read_record(at).unwrap(),]  raw.read_record(at).unwrap(),
IGNORED     LINE   crates/store/tests/sealed_samples.rs:177  [LS1: : bars[at as usize],]  bars[at as usize],
IGNORED     LINE   crates/store/tests/sealed_samples.rs:182  [LS1: : let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());]  let (mut open_ns, mut page_sealed, mut page_raw) = (Vec::new(), Vec::new(), Vec::new());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:183  [LS1: : let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =]  let (mut row_sealed, mut row_raw, mut find_sealed, mut find_raw) =
IGNORED     LINE   crates/store/tests/sealed_samples.rs:184  [LS1: : (Vec::new(), Vec::new(), Vec::new(), Vec::new());]  (Vec::new(), Vec::new(), Vec::new(), Vec::new());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:185  [LS1: : let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);]  let first_ts = bars.first().map_or(0, |bar| bar.ts_micros);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:186  [LS1: : let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);]  let last_ts = bars.last().map_or(0, |bar| bar.ts_micros);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:187  [LS1: : for _ in 0..2_000 {]  for _ in 0..2_000 {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:188  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:189  [LS1: : let month =]  let month =
IGNORED     LINE   crates/store/tests/sealed_samples.rs:190  [LS1: : SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();]  SealedMonth::open(std::fs::File::open(&sbar).unwrap(), path, symbol).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:191  [LS1: : open_ns.push(started.elapsed().as_nanos());]  open_ns.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:192  [LS1: : drop(month);]  drop(month);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:193  [LS1: : if n == 0 {]  if n == 0 {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:193:16 False  [LS1: : if n == 0 {]  if n == 0 {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:193:16 True  [LS1: : if n == 0 {]  if n == 0 {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:194  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:195  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:196  [LS1: : let count = n.min(100);]  let count = n.min(100);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:197  [LS1: : let at = rng.below(n - count + 1);]  let at = rng.below(n - count + 1);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:198  [LS1: : page.clear();]  page.clear();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:199  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:200  [LS1: : sealed.read_rows(at, count, &mut page).unwrap();]  sealed.read_rows(at, count, &mut page).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:201  [LS1: : page_sealed.push(started.elapsed().as_nanos());]  page_sealed.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:202  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:203  [LS1: : for i in at..at + count {]  for i in at..at + count {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:204  [LS1: : let _ = raw.read_record(i).unwrap();]  let _ = raw.read_record(i).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:205  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:206  [LS1: : page_raw.push(started.elapsed().as_nanos());]  page_raw.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:207  [LS1: : assert_eq!(page, bars[at as usize..(at + count) as usize]);]  assert_eq!(page, bars[at as usize..(at + count) as usize]);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:209  [LS1: : let at = rng.below(n);]  let at = rng.below(n);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:210  [LS1: : page.clear();]  page.clear();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:211  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:212  [LS1: : sealed.read_rows(at, 1, &mut page).unwrap();]  sealed.read_rows(at, 1, &mut page).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:213  [LS1: : row_sealed.push(started.elapsed().as_nanos());]  row_sealed.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:214  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:215  [LS1: : let one = raw.read_record(at).unwrap();]  let one = raw.read_record(at).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:216  [LS1: : row_raw.push(started.elapsed().as_nanos());]  row_raw.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:217  [LS1: : assert_eq!(page, [one]);]  assert_eq!(page, [one]);
IGNORED     LINE   crates/store/tests/sealed_samples.rs:219  [LS1: : let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;]  let ts = first_ts + rng.below((last_ts - first_ts) as u64 + 1) as i64;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:220  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:221  [LS1: : let found = sealed.first_at_or_after(ts).unwrap();]  let found = sealed.first_at_or_after(ts).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:222  [LS1: : find_sealed.push(started.elapsed().as_nanos());]  find_sealed.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:223  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:224  [LS1: : let raw_found = raw.first_at_or_after(ts).unwrap();]  let raw_found = raw.first_at_or_after(ts).unwrap();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:225  [LS1: : find_raw.push(started.elapsed().as_nanos());]  find_raw.push(started.elapsed().as_nanos());
IGNORED     LINE   crates/store/tests/sealed_samples.rs:226  [LS1: : let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;]  let scanned = bars.partition_point(|bar| bar.ts_micros < ts) as u64;
IGNORED     LINE   crates/store/tests/sealed_samples.rs:227  [LS1: : assert_eq!(found, scanned, "{rel} ts {ts}");]  assert_eq!(found, scanned, "{rel} ts {ts}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:228  [LS1: : assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");]  assert_eq!(raw_found, scanned, "{rel} raw ts {ts}");
IGNORED     LINE   crates/store/tests/sealed_samples.rs:230  [LS1: : let raw_len = std::fs::metadata(&bin).unwrap().len();]  let raw_len = std::fs::metadata(&bin).unwrap().len();
IGNORED     LINE   crates/store/tests/sealed_samples.rs:231  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_samples.rs:233  [LS1: : sealed.head().n_blocks,]  sealed.head().n_blocks,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:234  [LS1: : sealed.head().table_entries,]  sealed.head().table_entries,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:235  [LS1: : built.file_len(),]  built.file_len(),
IGNORED     LINE   crates/store/tests/sealed_samples.rs:236  [LS1: : built.file_len() * 100 / n.max(1),]  built.file_len() * 100 / n.max(1),
IGNORED     LINE   crates/store/tests/sealed_samples.rs:237  [LS1: : built.file_len() * 1_000 / raw_len,]  built.file_len() * 1_000 / raw_len,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:238  [LS1: : sealed.head().prv_raw_len,]  sealed.head().prv_raw_len,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:239  [LS1: : sealed.head().prv_packed_len,]  sealed.head().prv_packed_len,
IGNORED     LINE   crates/store/tests/sealed_samples.rs:241  [LS1: : println!("  {}", percentiles("open sealed (file + head)", open_ns));]  println!("  {}", percentiles("open sealed (file + head)", open_ns));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:242  [LS1: : if n > 0 {]  if n > 0 {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:242:12 False  [LS1: : if n > 0 {]  if n > 0 {
IGNORED     BRANCH crates/store/tests/sealed_samples.rs:242:12 True  [LS1: : if n > 0 {]  if n > 0 {
IGNORED     LINE   crates/store/tests/sealed_samples.rs:243  [LS1: : println!("  {}", percentiles("100-row page sealed", page_sealed));]  println!("  {}", percentiles("100-row page sealed", page_sealed));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:244  [LS1: : println!("  {}", percentiles("100-row page raw   ", page_raw));]  println!("  {}", percentiles("100-row page raw   ", page_raw));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:245  [LS1: : println!("  {}", percentiles("one row sealed     ", row_sealed));]  println!("  {}", percentiles("one row sealed     ", row_sealed));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:246  [LS1: : println!("  {}", percentiles("one row raw        ", row_raw));]  println!("  {}", percentiles("one row raw        ", row_raw));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:247  [LS1: : println!("  {}", percentiles("first_at_or_after sealed", find_sealed));]  println!("  {}", percentiles("first_at_or_after sealed", find_sealed));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:248  [LS1: : println!("  {}", percentiles("first_at_or_after raw   ", find_raw));]  println!("  {}", percentiles("first_at_or_after raw   ", find_raw));
IGNORED     LINE   crates/store/tests/sealed_samples.rs:249  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_samples.rs:251  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:44  [LS1: : fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {]  fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:45  [LS1: : let y = if m <= 2 { y - 1 } else { y };]  let y = if m <= 2 { y - 1 } else { y };
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:45:16 False  [LS1: : let y = if m <= 2 { y - 1 } else { y };]  let y = if m <= 2 { y - 1 } else { y };
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:45:16 True  [LS1: : let y = if m <= 2 { y - 1 } else { y };]  let y = if m <= 2 { y - 1 } else { y };
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:46  [LS1: : let era = y.div_euclid(400);]  let era = y.div_euclid(400);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:47  [LS1: : let yoe = y - era * 400;]  let yoe = y - era * 400;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:48  [LS1: : let mp = (m + 9) % 12;]  let mp = (m + 9) % 12;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:49  [LS1: : let doy = (153 * mp + 2) / 5 + d - 1;]  let doy = (153 * mp + 2) / 5 + d - 1;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:50  [LS1: : let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;]  let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:51  [LS1: : era * 146_097 + doe - 719_468]  era * 146_097 + doe - 719_468
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:52  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:54  [LS1: : fn ist_day(bar: &Bar) -> i64 {]  fn ist_day(bar: &Bar) -> i64 {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:55  [LS1: : (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)]  (bar.ts_micros / 1_000_000 + IST).div_euclid(86_400)
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:56  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:58  [LS1: : fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {]  fn store_path(rel: &str) -> (StorePath<'static>, &'static str) {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:59  [LS1: : let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());]  let rel: &'static str = Box::leak(rel.to_owned().into_boxed_str());
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:60  [LS1: : let parts: Vec<&'static str> = rel.split('/').collect();]  let parts: Vec<&'static str> = rel.split('/').collect();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:61  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:62  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:62:9 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:62:9 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:62:29 False  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:62:29 True  [LS1: : parts.len() == 7 || parts.len() == 8,]  parts.len() == 7 || parts.len() == 8,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:65  [LS1: : let vendor = Vendor::ALL]  let vendor = Vendor::ALL
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:66  [LS1: : .into_iter()]  .into_iter()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:67  [LS1: : .find(|v| v.as_str() == parts[1])]  .find(|v| v.as_str() == parts[1])
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:68  [LS1: : .unwrap();]  .unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:69  [LS1: : let contract = (parts.len() == 8)]  let contract = (parts.len() == 8)
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:70  [LS1: : .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));]  .then(|| brutex_core::instrument::Contract::parse(parts[5]).expect("contract"));
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:71  [LS1: : let tf = parts[parts.len() - 2];]  let tf = parts[parts.len() - 2];
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:72  [LS1: : let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();]  let timeframe = *Timeframe::KNOWN.iter().find(|t| t.as_str() == tf).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:73  [LS1: : let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();]  let stem = parts[parts.len() - 1].strip_suffix(".bin").unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:74  [LS1: : let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();]  let month = YearMonth::new(stem[..4].parse().unwrap(), stem[5..7].parse().unwrap()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:75  [LS1: : let path = StorePath::new(PathParts {]  let path = StorePath::new(PathParts {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:76  [LS1: : vendor,]  vendor,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:77  [LS1: : exchange: parts[2],]  exchange: parts[2],
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:78  [LS1: : segment: parts[3],]  segment: parts[3],
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:79  [LS1: : symbol: parts[4],]  symbol: parts[4],
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:80  [LS1: : contract,]  contract,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:81  [LS1: : timeframe,]  timeframe,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:82  [LS1: : month,]  month,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:83  [LS1: : file: FileKind::Bars,]  file: FileKind::Bars,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:84  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:85  [LS1: : .unwrap();]  .unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:86  [LS1: : (path, parts[4])]  (path, parts[4])
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:87  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:89  [LS1: : fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {]  fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:90  [LS1: : let mut files = BTreeMap::new();]  let mut files = BTreeMap::new();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:91  [LS1: : let mut stack = vec![dir.to_path_buf()];]  let mut stack = vec![dir.to_path_buf()];
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:92  [LS1: : while let Some(at) = stack.pop() {]  while let Some(at) = stack.pop() {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:92:15 False  [LS1: : while let Some(at) = stack.pop() {]  while let Some(at) = stack.pop() {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:92:15 True  [LS1: : while let Some(at) = stack.pop() {]  while let Some(at) = stack.pop() {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:93  [LS1: : let Ok(entries) = fs::read_dir(&at) else {]  let Ok(entries) = fs::read_dir(&at) else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:93:13 False  [LS1: : let Ok(entries) = fs::read_dir(&at) else {]  let Ok(entries) = fs::read_dir(&at) else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:93:13 True  [LS1: : let Ok(entries) = fs::read_dir(&at) else {]  let Ok(entries) = fs::read_dir(&at) else {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:94  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:96  [LS1: : for entry in entries {]  for entry in entries {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:97  [LS1: : let path = entry.unwrap().path();]  let path = entry.unwrap().path();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:98  [LS1: : if path.is_dir() {]  if path.is_dir() {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:98:16 False  [LS1: : if path.is_dir() {]  if path.is_dir() {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:98:16 True  [LS1: : if path.is_dir() {]  if path.is_dir() {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:99  [LS1: : stack.push(path);]  stack.push(path);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:100  [LS1: : } else {]  } else {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:101  [LS1: : files.insert(path.clone(), fs::read(&path).unwrap());]  files.insert(path.clone(), fs::read(&path).unwrap());
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:102  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:105  [LS1: : files]  files
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:106  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:108  [LS1: : fn micros(started: Instant) -> f64 {]  fn micros(started: Instant) -> f64 {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:109  [LS1: : started.elapsed().as_secs_f64() * 1e6]  started.elapsed().as_secs_f64() * 1e6
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:110  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:117  [LS1: : fn the_write_door_over_real_months_sealed_without_one_day() {]  fn the_write_door_over_real_months_sealed_without_one_day() {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:118  [LS1: : let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));]  let source = PathBuf::from(std::env::var("BRUTEX_SC4_SOURCE").expect("BRUTEX_SC4_SOURCE"));
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:119  [LS1: : let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));]  let work = PathBuf::from(std::env::var("BRUTEX_SC4_WORK").expect("BRUTEX_SC4_WORK"));
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:120  [LS1: : let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");]  let list = std::env::var("BRUTEX_SC4_MONTHS").expect("BRUTEX_SC4_MONTHS");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:121  [LS1: : for root in [&source, &work] {]  for root in [&source, &work] {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:122  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:123  [LS1: : !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),]  !root.starts_with("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime"),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:126  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:127  [LS1: : !fs::symlink_metadata(root.join("bars"))]  !fs::symlink_metadata(root.join("bars"))
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:128  [LS1: : .is_ok_and(|meta| meta.file_type().is_symlink()),]  .is_ok_and(|meta| meta.file_type().is_symlink()),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:130  [LS1: : root.display()]  root.display()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:133  [LS1: : for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {]  for (line_no, line) in fs::read_to_string(&list).unwrap().lines().enumerate() {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:134  [LS1: : let line = line.trim();]  let line = line.trim();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:135  [LS1: : if line.is_empty() || line.starts_with('#') {]  if line.is_empty() || line.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:135:12 False  [LS1: : if line.is_empty() || line.starts_with('#') {]  if line.is_empty() || line.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:135:12 True  [LS1: : if line.is_empty() || line.starts_with('#') {]  if line.is_empty() || line.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:135:31 False  [LS1: : if line.is_empty() || line.starts_with('#') {]  if line.is_empty() || line.starts_with('#') {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:135:31 True  [LS1: : if line.is_empty() || line.starts_with('#') {]  if line.is_empty() || line.starts_with('#') {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:136  [LS1: : continue;]  continue;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:137  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:138  [LS1: : let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");]  let (rel, date) = line.split_once(' ').expect("<path> <yyyy-mm-dd>");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:139  [LS1: : let (path, symbol) = store_path(rel);]  let (path, symbol) = store_path(rel);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:140  [LS1: : let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;]  let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:141  [LS1: : let full = {]  let full = {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:142  [LS1: : let file = BarFile::open_existing(&source, path, symbol_id).unwrap();]  let file = BarFile::open_existing(&source, path, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:143  [LS1: : file.read_range(0, file.records()).unwrap()]  file.read_range(0, file.records()).unwrap()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:146  [LS1: : let day = if date == "last" {]  let day = if date == "last" {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:146:22 False  [LS1: : let day = if date == "last" {]  let day = if date == "last" {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:146:22 True  [LS1: : let day = if date == "last" {]  let day = if date == "last" {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:147  [LS1: : ist_day(full.last().expect("a month with bars"))]  ist_day(full.last().expect("a month with bars"))
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:149  [LS1: : let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();]  let ymd: Vec<i64> = date.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:150  [LS1: : days_from_civil(ymd[0], ymd[1], ymd[2])]  days_from_civil(ymd[0], ymd[1], ymd[2])
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:152  [LS1: : let (held, offered): (Vec<Bar>, Vec<Bar>) =]  let (held, offered): (Vec<Bar>, Vec<Bar>) =
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:153  [LS1: : full.iter().copied().partition(|bar| ist_day(bar) != day);]  full.iter().copied().partition(|bar| ist_day(bar) != day);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:154  [LS1: : assert!(!offered.is_empty(), "{rel}: no bar on {date}");]  assert!(!offered.is_empty(), "{rel}: no bar on {date}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:155  [LS1: : assert!(!held.is_empty(), "{rel}: nothing but {date}");]  assert!(!held.is_empty(), "{rel}: nothing but {date}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:156  [LS1: : let root = work.join(format!("m{line_no}"));]  let root = work.join(format!("m{line_no}"));
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:157  [LS1: : fs::create_dir_all(&root).unwrap();]  fs::create_dir_all(&root).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:159  [LS1: : let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();]  let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:160  [LS1: : for chunk in held.chunks(65_536) {]  for chunk in held.chunks(65_536) {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:161  [LS1: : writer.append(chunk).unwrap();]  writer.append(chunk).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:162  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:164  [LS1: : let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();]  let raw = BarFile::open_existing(&root, path, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:165  [LS1: : let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();]  let built = store::sealed::build(path, &raw.header(), &held, None, None).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:166  [LS1: : drop(raw);]  drop(raw);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:167  [LS1: : let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);]  let sbar = path.with_file(FileKind::SealedBars).to_path_buf(&root);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:168  [LS1: : let staged = sbar.with_extension("sbar-staged");]  let staged = sbar.with_extension("sbar-staged");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:169  [LS1: : fs::write(&staged, built.to_bytes()).unwrap();]  fs::write(&staged, built.to_bytes()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:170  [LS1: : fs::rename(&staged, &sbar).unwrap();]  fs::rename(&staged, &sbar).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:171  [LS1: : let before = snapshot(&root.join("bars"));]  let before = snapshot(&root.join("bars"));
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:173  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:174  [LS1: : let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();]  let mut writer = BarFile::open_or_create(&root, path, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:175  [LS1: : let open_us = micros(started);]  let open_us = micros(started);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:176  [LS1: : assert_eq!(writer.form(), Form::Sealed, "{rel}");]  assert_eq!(writer.form(), Form::Sealed, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:177  [LS1: : let first_day = ist_day(&held[0]);]  let first_day = ist_day(&held[0]);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:178  [LS1: : let reoffer: Vec<Bar> = held]  let reoffer: Vec<Bar> = held
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:179  [LS1: : .iter()]  .iter()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:180  [LS1: : .copied()]  .copied()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:181  [LS1: : .filter(|bar| ist_day(bar) == first_day)]  .filter(|bar| ist_day(bar) == first_day)
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:182  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:183  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:184  [LS1: : let again = writer.append(&reoffer).unwrap();]  let again = writer.append(&reoffer).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:185  [LS1: : let reoffer_us = micros(started);]  let reoffer_us = micros(started);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:186  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:188  [LS1: : Appended::AlreadyPresent {]  Appended::AlreadyPresent {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:189  [LS1: : first_index: 0,]  first_index: 0,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:190  [LS1: : n_valid: held.len() as u64]  n_valid: held.len() as u64
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:191  [LS1: : },]  },
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:194  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:195  [LS1: : let refused = writer.append(&offered).unwrap_err();]  let refused = writer.append(&offered).unwrap_err();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:196  [LS1: : let refused_us = micros(started);]  let refused_us = micros(started);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:197  [LS1: : let StoreError::Sealed { offer, at, .. } = refused else {]  let StoreError::Sealed { offer, at, .. } = refused else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:197:13 False  [LS1: : let StoreError::Sealed { offer, at, .. } = refused else {]  let StoreError::Sealed { offer, at, .. } = refused else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:197:13 True  [LS1: : let StoreError::Sealed { offer, at, .. } = refused else {]  let StoreError::Sealed { offer, at, .. } = refused else {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:198  [LS1: : panic!("{rel}: {refused}");]  panic!("{rel}: {refused}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:200  [LS1: : let is_last_day = day == ist_day(full.last().unwrap());]  let is_last_day = day == ist_day(full.last().unwrap());
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:201  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:203  [LS1: : if is_last_day {]  if is_last_day {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:203:16 False  [LS1: : if is_last_day {]  if is_last_day {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:203:16 True  [LS1: : if is_last_day {]  if is_last_day {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:204  [LS1: : SealedOffer::Follows]  SealedOffer::Follows
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:206  [LS1: : SealedOffer::Disagrees(store::file::Conflict::NotHeld)]  SealedOffer::Disagrees(store::file::Conflict::NotHeld)
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:210  [LS1: : assert_eq!(at, 0, "{rel}");]  assert_eq!(at, 0, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:211  [LS1: : drop(writer);]  drop(writer);
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:212  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:213  [LS1: : snapshot(&root.join("bars")),]  snapshot(&root.join("bars")),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:218  [LS1: : let started = Instant::now();]  let started = Instant::now();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:219  [LS1: : let repaired = store::repair::repair_month(]  let repaired = store::repair::repair_month(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:220  [LS1: : &root,]  &root,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:221  [LS1: : path,]  path,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:222  [LS1: : symbol_id,]  symbol_id,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:223  [LS1: : &offered,]  &offered,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:224  [LS1: : Policy {]  Policy {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:225  [LS1: : // No offered bar shares a stamp with a held one: the day is]  // No offered bar shares a stamp with a held one: the day is
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:226  [LS1: : // wholly absent, so the comparison this sets is never made.]  // wholly absent, so the comparison this sets is never made.
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:227  [LS1: : open_interest_applies: true,]  open_interest_applies: true,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:228  [LS1: : basis_witness: false,]  basis_witness: false,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:229  [LS1: : },]  },
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:231  [LS1: : .unwrap();]  .unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:232  [LS1: : let repair_ms = micros(started) / 1e3;]  let repair_ms = micros(started) / 1e3;
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:233  [LS1: : let MonthRepair::Repaired { added, .. } = repaired else {]  let MonthRepair::Repaired { added, .. } = repaired else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:233:13 False  [LS1: : let MonthRepair::Repaired { added, .. } = repaired else {]  let MonthRepair::Repaired { added, .. } = repaired else {
IGNORED     BRANCH crates/store/tests/sealed_write_door.rs:233:13 True  [LS1: : let MonthRepair::Repaired { added, .. } = repaired else {]  let MonthRepair::Repaired { added, .. } = repaired else {
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:234  [LS1: : panic!("{rel}: {repaired:?}");]  panic!("{rel}: {repaired:?}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:236  [LS1: : assert_eq!(added, offered.len() as u64, "{rel}");]  assert_eq!(added, offered.len() as u64, "{rel}");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:237  [LS1: : let served = BarFile::open_existing(&root, path, symbol_id).unwrap();]  let served = BarFile::open_existing(&root, path, symbol_id).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:238  [LS1: : assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");]  assert_eq!(served.form(), Form::Raw, "{rel}: the successor is raw");
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:239  [LS1: : let now = served.read_range(0, served.records()).unwrap();]  let now = served.read_range(0, served.records()).unwrap();
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:240  [LS1: : assert!(]  assert!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:241  [LS1: : now == full,]  now == full,
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:244  [LS1: : assert_eq!(]  assert_eq!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:245  [LS1: : snapshot(&root.join("bars")),]  snapshot(&root.join("bars")),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:249  [LS1: : println!(]  println!(
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:253  [LS1: : full.len(),]  full.len(),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:254  [LS1: : held.len(),]  held.len(),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:255  [LS1: : offered.len(),]  offered.len(),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:256  [LS1: : offer.as_str(),]  offer.as_str(),
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:257  [LS1: : built.file_len()]  built.file_len()
IGNORED     LINE   crates/store/tests/sealed_write_door.rs:260  [LS1: : }]  }
(F)         LINE   crates/store/tests/slot_audit.rs:116  [LS1]  matches!(
(F)         BRANCH crates/store/tests/slot_audit.rs:130:13 False  [LS1]  let MonthVerdict::Reverted {
(F)         LINE   crates/store/tests/slot_audit.rs:137  [LS1]  panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
(F)         LINE   crates/store/tests/slot_audit.rs:277  [LS1]  matches!(
(F)         LINE   crates/store/tests/slot_audit.rs:347  [LS1]  matches!(
(F)         LINE   crates/store/tests/slot_audit.rs:410  [LS1]  matches!(
(F)         BRANCH crates/store/tests/slot_audit.rs:484:9 False  [LS1]  let SlotState::Damaged { reason, .. } = audit.slots[1] else {
(F)         LINE   crates/store/tests/slot_audit.rs:485  [LS1]  panic!("damaged");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:95  [LS1]  raw.slot_audit()
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:149  [LS1]  Ok(file) => {
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:150  [LS1]  assert_eq!(file.form(), Form::Sealed, "byte {at}");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:151  [LS1]  assert_eq!(file.slot_audit(), None, "byte {at}");
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:152  [LS1]  assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:153  [LS1]  if let Ok(last) = file.read_record(39) {
(U)         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False  [LS1]  if let Ok(last) = file.read_record(39) {
(U)         BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True  [LS1]  if let Ok(last) = file.read_record(39) {
(F)         LINE   crates/store/tests/slot_audit_sealed.rs:154  [LS1]  assert_eq!(last.close, 7_039, "byte {at}");
(U)         LINE   crates/store/tests/slot_audit_sealed.rs:155  [LS1]  }
(F)         LINE   crates/store/tests/slot_damaged_log.rs:94  [LS1]  other => panic!("{key}: {other:?}"),
(F)         LINE   crates/store/tests/slot_damaged_log.rs:101  [LS1]  other => panic!("{key}: {other:?}"),
(F)         LINE   crates/store/tests/slot_damaged_log.rs:192  [LS1]  found
(F)         LINE   crates/store/tests/slot_damaged_log.rs:193  [LS1]  .records
(F)         LINE   crates/store/tests/slot_damaged_log.rs:194  [LS1]  .iter()
(F)         LINE   crates/store/tests/slot_damaged_log.rs:195  [LS1]  .map(|r| (r.target.clone(), r.message.clone()))
(F)         LINE   crates/store/tests/slot_damaged_log.rs:196  [LS1]  .collect::<Vec<_>>()
(F)         BRANCH crates/store/tests/sort_index.rs:387:62 False  [LS1]  matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
(F)         BRANCH crates/store/tests/sort_index.rs:430:56 False  [LS1]  matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
(F)         BRANCH crates/store/tests/sort_index.rs:446:63 False  [LS1]  assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
(F)         LINE   crates/store/tests/sort_index.rs:488  [LS1]  index.torn()
(F)         BRANCH crates/store/tests/sort_index.rs:660:58 False  [LS1]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F)         BRANCH crates/store/tests/sort_index.rs:736:58 False  [LS1]  matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
(F)         LINE   crates/store/tests/sort_index.rs:761  [LS1]  column.as_str()
(F)         LINE   crates/store/tests/sort_index.rs:843  [LS1]  column.as_str()
(F)         BRANCH crates/store/tests/sort_index.rs:874:16 False  [LS1]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F)         BRANCH crates/store/tests/sort_index.rs:874:54 False  [LS1]  if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
(F)         BRANCH crates/store/tests/sort_index.rs:889:16 False  [LS1]  if reason.contains("2031-02.new.srt")),
(F) fail    BRANCH crates/store/tests/support/mod.rs:62:15 False  [LS1]  let ran = output.status.success() && stdout.contains("1 passed");
(F)         BRANCH crates/store/tests/tail_proof.rs:144:9 False  [LS1]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         BRANCH crates/store/tests/tail_proof.rs:144:36 False  [LS1]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         BRANCH crates/store/tests/tail_proof.rs:144:60 False  [LS1]  found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
(F)         LINE   crates/store/tests/tix.rs:173  [LS1]  timeframe.as_str()
(F) collect BRANCH crates/store/tests/tix.rs:605:76 True  [LS1]  && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
IGNORED     LINE   crates/store/tests/tix_latency.rs:43  [LS1: : fn path(timeframe: Timeframe) -> StorePath<'static> {]  fn path(timeframe: Timeframe) -> StorePath<'static> {
IGNORED     LINE   crates/store/tests/tix_latency.rs:44  [LS1: : StorePath::new(PathParts {]  StorePath::new(PathParts {
IGNORED     LINE   crates/store/tests/tix_latency.rs:45  [LS1: : vendor: Vendor::Groww,]  vendor: Vendor::Groww,
IGNORED     LINE   crates/store/tests/tix_latency.rs:46  [LS1: : exchange: "NSE",]  exchange: "NSE",
IGNORED     LINE   crates/store/tests/tix_latency.rs:47  [LS1: : segment: "INDEX",]  segment: "INDEX",
IGNORED     LINE   crates/store/tests/tix_latency.rs:48  [LS1: : symbol: "NIFTY",]  symbol: "NIFTY",
IGNORED     LINE   crates/store/tests/tix_latency.rs:49  [LS1: : contract: None,]  contract: None,
IGNORED     LINE   crates/store/tests/tix_latency.rs:50  [LS1: : timeframe,]  timeframe,
IGNORED     LINE   crates/store/tests/tix_latency.rs:51  [LS1: : month: YearMonth::new(2024, 7).unwrap(),]  month: YearMonth::new(2024, 7).unwrap(),
IGNORED     LINE   crates/store/tests/tix_latency.rs:52  [LS1: : file: FileKind::Bars,]  file: FileKind::Bars,
IGNORED     LINE   crates/store/tests/tix_latency.rs:53  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/tix_latency.rs:54  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/tix_latency.rs:55  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:59  [LS1: : fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {]  fn month(days: i64, per_day: i64, step: i64, weekdays: bool) -> Vec<Bar> {
IGNORED     LINE   crates/store/tests/tix_latency.rs:60  [LS1: : let mut out = Vec::new();]  let mut out = Vec::new();
IGNORED     LINE   crates/store/tests/tix_latency.rs:61  [LS1: : let (mut day, mut taken) = (0, 0);]  let (mut day, mut taken) = (0, 0);
IGNORED     LINE   crates/store/tests/tix_latency.rs:62  [LS1: : while taken < days {]  while taken < days {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:62:11 False  [LS1: : while taken < days {]  while taken < days {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:62:11 True  [LS1: : while taken < days {]  while taken < days {
IGNORED     LINE   crates/store/tests/tix_latency.rs:64  [LS1: : if !weekdays || day % 7 < 5 {]  if !weekdays || day % 7 < 5 {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:64:12 False  [LS1: : if !weekdays || day % 7 < 5 {]  if !weekdays || day % 7 < 5 {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:64:12 True  [LS1: : if !weekdays || day % 7 < 5 {]  if !weekdays || day % 7 < 5 {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:64:25 False  [LS1: : if !weekdays || day % 7 < 5 {]  if !weekdays || day % 7 < 5 {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:64:25 True  [LS1: : if !weekdays || day % 7 < 5 {]  if !weekdays || day % 7 < 5 {
IGNORED     LINE   crates/store/tests/tix_latency.rs:65  [LS1: : for k in 0..per_day {]  for k in 0..per_day {
IGNORED     LINE   crates/store/tests/tix_latency.rs:66  [LS1: : let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;]  let ts = JULY_START + day * 86_400 * MICROS + (555 * 60 + k * step) * MICROS;
IGNORED     LINE   crates/store/tests/tix_latency.rs:67  [LS1: : out.push(Bar {]  out.push(Bar {
IGNORED     LINE   crates/store/tests/tix_latency.rs:68  [LS1: : ts_micros: ts,]  ts_micros: ts,
IGNORED     LINE   crates/store/tests/tix_latency.rs:69  [LS1: : open: 2_000_000 + k,]  open: 2_000_000 + k,
IGNORED     LINE   crates/store/tests/tix_latency.rs:70  [LS1: : high: 2_000_100 + k,]  high: 2_000_100 + k,
IGNORED     LINE   crates/store/tests/tix_latency.rs:71  [LS1: : low: 1_999_900 + k,]  low: 1_999_900 + k,
IGNORED     LINE   crates/store/tests/tix_latency.rs:72  [LS1: : close: 2_000_050 + k,]  close: 2_000_050 + k,
IGNORED     LINE   crates/store/tests/tix_latency.rs:73  [LS1: : volume: 10,]  volume: 10,
IGNORED     LINE   crates/store/tests/tix_latency.rs:74  [LS1: : open_interest: OI_NULL,]  open_interest: OI_NULL,
IGNORED     LINE   crates/store/tests/tix_latency.rs:75  [LS1: : });]  });
IGNORED     LINE   crates/store/tests/tix_latency.rs:76  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:77  [LS1: : taken += 1;]  taken += 1;
IGNORED     LINE   crates/store/tests/tix_latency.rs:78  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:79  [LS1: : day += 1;]  day += 1;
IGNORED     LINE   crates/store/tests/tix_latency.rs:81  [LS1: : out]  out
IGNORED     LINE   crates/store/tests/tix_latency.rs:82  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:87  [LS1: : fn next(&mut self) -> u64 {]  fn next(&mut self) -> u64 {
IGNORED     LINE   crates/store/tests/tix_latency.rs:88  [LS1: : self.0 ^= self.0 >> 12;]  self.0 ^= self.0 >> 12;
IGNORED     LINE   crates/store/tests/tix_latency.rs:89  [LS1: : self.0 ^= self.0 << 25;]  self.0 ^= self.0 << 25;
IGNORED     LINE   crates/store/tests/tix_latency.rs:90  [LS1: : self.0 ^= self.0 >> 27;]  self.0 ^= self.0 >> 27;
IGNORED     LINE   crates/store/tests/tix_latency.rs:91  [LS1: : self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)]  self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
IGNORED     LINE   crates/store/tests/tix_latency.rs:92  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:95  [LS1: : fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {]  fn percentiles(mut ns: Vec<u64>) -> (u64, u64, u64) {
IGNORED     LINE   crates/store/tests/tix_latency.rs:96  [LS1: : ns.sort_unstable();]  ns.sort_unstable();
IGNORED     LINE   crates/store/tests/tix_latency.rs:97  [LS1: : let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];]  let at = |q: usize| ns[(ns.len() * q / 1000).min(ns.len() - 1)];
IGNORED     LINE   crates/store/tests/tix_latency.rs:98  [LS1: : (at(500), at(990), *ns.last().unwrap())]  (at(500), at(990), *ns.last().unwrap())
IGNORED     LINE   crates/store/tests/tix_latency.rs:99  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:101  [LS1: : fn report(case: &str, ns: Vec<u64>) {]  fn report(case: &str, ns: Vec<u64>) {
IGNORED     LINE   crates/store/tests/tix_latency.rs:102  [LS1: : let (p50, p99, max) = percentiles(ns);]  let (p50, p99, max) = percentiles(ns);
IGNORED     LINE   crates/store/tests/tix_latency.rs:103  [LS1: : println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");]  println!("{case:<58} p50 {p50:>8} ns   p99 {p99:>8} ns   max {max:>9} ns");
IGNORED     LINE   crates/store/tests/tix_latency.rs:104  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:107  [LS1: : fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {]  fn time<T: Copy>(inputs: &[T], mut op: impl FnMut(T) -> u64) -> Vec<u64> {
IGNORED     LINE   crates/store/tests/tix_latency.rs:108  [LS1: : inputs]  inputs
IGNORED     LINE   crates/store/tests/tix_latency.rs:109  [LS1: : .iter()]  .iter()
IGNORED     LINE   crates/store/tests/tix_latency.rs:110  [LS1: : .map(|&input| {]  .map(|&input| {
IGNORED     LINE   crates/store/tests/tix_latency.rs:111  [LS1: : let start = Instant::now();]  let start = Instant::now();
IGNORED     LINE   crates/store/tests/tix_latency.rs:112  [LS1: : black_box(op(black_box(input)));]  black_box(op(black_box(input)));
IGNORED     LINE   crates/store/tests/tix_latency.rs:113  [LS1: : start.elapsed().as_nanos() as u64]  start.elapsed().as_nanos() as u64
IGNORED     LINE   crates/store/tests/tix_latency.rs:114  [LS1: : })]  })
IGNORED     LINE   crates/store/tests/tix_latency.rs:115  [LS1: : .collect()]  .collect()
IGNORED     LINE   crates/store/tests/tix_latency.rs:116  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:118  [LS1: : fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {]  fn measure(name: &str, timeframe: Timeframe, bars: &[Bar]) {
IGNORED     LINE   crates/store/tests/tix_latency.rs:119  [LS1: : let root: PathBuf =]  let root: PathBuf =
IGNORED     LINE   crates/store/tests/tix_latency.rs:120  [LS1: : std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));]  std::env::temp_dir().join(format!("brutex-tix-latency-{name}-{}", std::process::id()));
IGNORED     LINE   crates/store/tests/tix_latency.rs:121  [LS1: : let _ = fs::remove_dir_all(&root);]  let _ = fs::remove_dir_all(&root);
IGNORED     LINE   crates/store/tests/tix_latency.rs:122  [LS1: : fs::create_dir_all(&root).unwrap();]  fs::create_dir_all(&root).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:124  [LS1: : let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();]  let mut writer = BarFile::open_or_create(&root, path(timeframe), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:125  [LS1: : for chunk in bars.chunks(22_500) {]  for chunk in bars.chunks(22_500) {
IGNORED     LINE   crates/store/tests/tix_latency.rs:126  [LS1: : writer.append(chunk).unwrap();]  writer.append(chunk).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:127  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:129  [LS1: : let n = bars.len() as u64;]  let n = bars.len() as u64;
IGNORED     LINE   crates/store/tests/tix_latency.rs:130  [LS1: : let first = bars[0].ts_micros;]  let first = bars[0].ts_micros;
IGNORED     LINE   crates/store/tests/tix_latency.rs:131  [LS1: : let last = bars[bars.len() - 1].ts_micros;]  let last = bars[bars.len() - 1].ts_micros;
IGNORED     LINE   crates/store/tests/tix_latency.rs:132  [LS1: : let width = i64::from(timeframe.secs()) * MICROS;]  let width = i64::from(timeframe.secs()) * MICROS;
IGNORED     LINE   crates/store/tests/tix_latency.rs:133  [LS1: : let mut rng = Rng(0x9E37_79B9_7F4A_7C15);]  let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
IGNORED     LINE   crates/store/tests/tix_latency.rs:135  [LS1: : let anywhere: Vec<i64> = (0..LOOKUPS)]  let anywhere: Vec<i64> = (0..LOOKUPS)
IGNORED     LINE   crates/store/tests/tix_latency.rs:136  [LS1: : .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)]  .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
IGNORED     LINE   crates/store/tests/tix_latency.rs:137  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/tix_latency.rs:139  [LS1: : let on_grid: Vec<i64> = anywhere]  let on_grid: Vec<i64> = anywhere
IGNORED     LINE   crates/store/tests/tix_latency.rs:140  [LS1: : .iter()]  .iter()
IGNORED     LINE   crates/store/tests/tix_latency.rs:141  [LS1: : .map(|&ts| first + (ts - first) / width * width)]  .map(|&ts| first + (ts - first) / width * width)
IGNORED     LINE   crates/store/tests/tix_latency.rs:142  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/tix_latency.rs:143  [LS1: : let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();]  let rows: Vec<u64> = (0..LOOKUPS).map(|_| rng.next() % n).collect();
IGNORED     LINE   crates/store/tests/tix_latency.rs:145  [LS1: : let tix = path(timeframe).to_path_buf(&root).with_extension("tix");]  let tix = path(timeframe).to_path_buf(&root).with_extension("tix");
IGNORED     LINE   crates/store/tests/tix_latency.rs:146  [LS1: : let passes: &[bool] = if tix.exists() {]  let passes: &[bool] = if tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:146:30 False  [LS1: : let passes: &[bool] = if tix.exists() {]  let passes: &[bool] = if tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:146:30 True  [LS1: : let passes: &[bool] = if tix.exists() {]  let passes: &[bool] = if tix.exists() {
IGNORED     LINE   crates/store/tests/tix_latency.rs:147  [LS1: : &[true, false]]  &[true, false]
IGNORED     LINE   crates/store/tests/tix_latency.rs:149  [LS1: : &[false]]  &[false]
IGNORED     LINE   crates/store/tests/tix_latency.rs:151  [LS1: : for &with_index in passes {]  for &with_index in passes {
IGNORED     LINE   crates/store/tests/tix_latency.rs:152  [LS1: : if !with_index && tix.exists() {]  if !with_index && tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:152:12 False  [LS1: : if !with_index && tix.exists() {]  if !with_index && tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:152:12 True  [LS1: : if !with_index && tix.exists() {]  if !with_index && tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:152:27 False  [LS1: : if !with_index && tix.exists() {]  if !with_index && tix.exists() {
IGNORED     BRANCH crates/store/tests/tix_latency.rs:152:27 True  [LS1: : if !with_index && tix.exists() {]  if !with_index && tix.exists() {
IGNORED     LINE   crates/store/tests/tix_latency.rs:153  [LS1: : fs::remove_file(&tix).unwrap();]  fs::remove_file(&tix).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:154  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:155  [LS1: : let tag = if with_index { "index" } else { "bisection" };]  let tag = if with_index { "index" } else { "bisection" };
IGNORED     BRANCH crates/store/tests/tix_latency.rs:155:22 False  [LS1: : let tag = if with_index { "index" } else { "bisection" };]  let tag = if with_index { "index" } else { "bisection" };
IGNORED     BRANCH crates/store/tests/tix_latency.rs:155:22 True  [LS1: : let tag = if with_index { "index" } else { "bisection" };]  let tag = if with_index { "index" } else { "bisection" };
IGNORED     LINE   crates/store/tests/tix_latency.rs:156  [LS1: : let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");]  let label = |what: &str| format!("{name} ({n} bars) {what} [{tag}]");
IGNORED     LINE   crates/store/tests/tix_latency.rs:159  [LS1: : let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();]  let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:160  [LS1: : report(]  report(
IGNORED     LINE   crates/store/tests/tix_latency.rs:161  [LS1: : &label("by time, random us"),]  &label("by time, random us"),
IGNORED     LINE   crates/store/tests/tix_latency.rs:162  [LS1: : time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),]  time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
IGNORED     LINE   crates/store/tests/tix_latency.rs:164  [LS1: : let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();]  let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:165  [LS1: : report(]  report(
IGNORED     LINE   crates/store/tests/tix_latency.rs:166  [LS1: : &label("by time, on grid"),]  &label("by time, on grid"),
IGNORED     LINE   crates/store/tests/tix_latency.rs:167  [LS1: : time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),]  time(&on_grid, |ts| file.first_at_or_after(ts).unwrap()),
IGNORED     LINE   crates/store/tests/tix_latency.rs:169  [LS1: : let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();]  let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:170  [LS1: : report(]  report(
IGNORED     LINE   crates/store/tests/tix_latency.rs:171  [LS1: : &label("by row, random rows (cold block)"),]  &label("by row, random rows (cold block)"),
IGNORED     LINE   crates/store/tests/tix_latency.rs:172  [LS1: : time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),]  time(&rows, |row| file.read_record(row).unwrap().ts_micros as u64),
IGNORED     LINE   crates/store/tests/tix_latency.rs:176  [LS1: : let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();]  let file = BarFile::open_existing(&root, path(timeframe), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:177  [LS1: : let same = vec![rows[0]; LOOKUPS];]  let same = vec![rows[0]; LOOKUPS];
IGNORED     LINE   crates/store/tests/tix_latency.rs:178  [LS1: : report(]  report(
IGNORED     LINE   crates/store/tests/tix_latency.rs:179  [LS1: : &label("by row, one row (warm block)"),]  &label("by row, one row (warm block)"),
IGNORED     LINE   crates/store/tests/tix_latency.rs:180  [LS1: : time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),]  time(&same, |row| file.read_record(row).unwrap().ts_micros as u64),
IGNORED     LINE   crates/store/tests/tix_latency.rs:183  [LS1: : let _ = fs::remove_dir_all(&root);]  let _ = fs::remove_dir_all(&root);
IGNORED     LINE   crates/store/tests/tix_latency.rs:184  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:188  [LS1: : fn time_and_row_lookup_latency() {]  fn time_and_row_lookup_latency() {
IGNORED     LINE   crates/store/tests/tix_latency.rs:189  [LS1: : measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));]  measure("1min", Timeframe::MINUTE_1, &month(31, 375, 60, false));
IGNORED     LINE   crates/store/tests/tix_latency.rs:190  [LS1: : measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));]  measure("1s", Timeframe::SECOND_1, &month(23, 22_500, 1, true));
IGNORED     LINE   crates/store/tests/tix_latency.rs:191  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:209  [LS1: : fn small_month_rule_latency() {]  fn small_month_rule_latency() {
IGNORED     LINE   crates/store/tests/tix_latency.rs:211  [LS1: : let root: PathBuf =]  let root: PathBuf =
IGNORED     LINE   crates/store/tests/tix_latency.rs:212  [LS1: : std::env::temp_dir().join(format!("brutex-tix-latency-small-{}", std::process::id()));]  std::env::temp_dir().join(format!("brutex-tix-latency-small-{}", std::process::id()));
IGNORED     LINE   crates/store/tests/tix_latency.rs:213  [LS1: : let _ = fs::remove_dir_all(&root);]  let _ = fs::remove_dir_all(&root);
IGNORED     LINE   crates/store/tests/tix_latency.rs:214  [LS1: : fs::create_dir_all(&root).unwrap();]  fs::create_dir_all(&root).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:215  [LS1: : let all = month(1, 1_100, 1, true);]  let all = month(1, 1_100, 1, true);
IGNORED     LINE   crates/store/tests/tix_latency.rs:216  [LS1: : for (name, n) in []  for (name, n) in [
IGNORED     LINE   crates/store/tests/tix_latency.rs:217  [LS1: : ("1s, 1023 bars, below", 1_023_usize),]  ("1s, 1023 bars, below", 1_023_usize),
IGNORED     LINE   crates/store/tests/tix_latency.rs:218  [LS1: : ("1s, 1024 bars, at", 1_024),]  ("1s, 1024 bars, at", 1_024),
IGNORED     LINE   crates/store/tests/tix_latency.rs:219  [LS1: : ] {]  ] {
IGNORED     LINE   crates/store/tests/tix_latency.rs:220  [LS1: : let at = root.join(name.replace([' ', ','], "-"));]  let at = root.join(name.replace([' ', ','], "-"));
IGNORED     LINE   crates/store/tests/tix_latency.rs:221  [LS1: : fs::create_dir_all(&at).unwrap();]  fs::create_dir_all(&at).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:222  [LS1: : BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7)]  BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7)
IGNORED     LINE   crates/store/tests/tix_latency.rs:223  [LS1: : .unwrap()]  .unwrap()
IGNORED     LINE   crates/store/tests/tix_latency.rs:224  [LS1: : .append(&all[..n])]  .append(&all[..n])
IGNORED     LINE   crates/store/tests/tix_latency.rs:225  [LS1: : .unwrap();]  .unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:226  [LS1: : let tix = path(Timeframe::SECOND_1)]  let tix = path(Timeframe::SECOND_1)
IGNORED     LINE   crates/store/tests/tix_latency.rs:227  [LS1: : .to_path_buf(&at)]  .to_path_buf(&at)
IGNORED     LINE   crates/store/tests/tix_latency.rs:228  [LS1: : .with_extension("tix");]  .with_extension("tix");
IGNORED     LINE   crates/store/tests/tix_latency.rs:229  [LS1: : assert_eq!(tix.exists(), n >= 1_024, "{name}");]  assert_eq!(tix.exists(), n >= 1_024, "{name}");
IGNORED     LINE   crates/store/tests/tix_latency.rs:230  [LS1: : let (first, last) = (all[0].ts_micros, all[n - 1].ts_micros);]  let (first, last) = (all[0].ts_micros, all[n - 1].ts_micros);
IGNORED     LINE   crates/store/tests/tix_latency.rs:231  [LS1: : let mut rng = Rng(0x9E37_79B9_7F4A_7C15);]  let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
IGNORED     LINE   crates/store/tests/tix_latency.rs:232  [LS1: : let anywhere: Vec<i64> = (0..LOOKUPS)]  let anywhere: Vec<i64> = (0..LOOKUPS)
IGNORED     LINE   crates/store/tests/tix_latency.rs:233  [LS1: : .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)]  .map(|_| first + (rng.next() % (last - first + 1) as u64) as i64)
IGNORED     LINE   crates/store/tests/tix_latency.rs:234  [LS1: : .collect();]  .collect();
IGNORED     LINE   crates/store/tests/tix_latency.rs:235  [LS1: : let file = BarFile::open_existing(&at, path(Timeframe::SECOND_1), 7).unwrap();]  let file = BarFile::open_existing(&at, path(Timeframe::SECOND_1), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:236  [LS1: : report(]  report(
IGNORED     LINE   crates/store/tests/tix_latency.rs:237  [LS1: : &format!("{name}: by time, random us [{:?}]", file.time_lookup()),]  &format!("{name}: by time, random us [{:?}]", file.time_lookup()),
IGNORED     LINE   crates/store/tests/tix_latency.rs:238  [LS1: : time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),]  time(&anywhere, |ts| file.first_at_or_after(ts).unwrap()),
IGNORED     LINE   crates/store/tests/tix_latency.rs:241  [LS1: : for (name, held) in []  for (name, held) in [
IGNORED     LINE   crates/store/tests/tix_latency.rs:242  [LS1: : ("commit of 1 bar under the threshold (500 held)", 500_usize),]  ("commit of 1 bar under the threshold (500 held)", 500_usize),
IGNORED     LINE   crates/store/tests/tix_latency.rs:243  [LS1: : ("commit of 1 bar reaching the threshold (1023 held)", 1_023),]  ("commit of 1 bar reaching the threshold (1023 held)", 1_023),
IGNORED     LINE   crates/store/tests/tix_latency.rs:244  [LS1: : ("commit of 1 bar past the threshold (1099 held)", 1_099),]  ("commit of 1 bar past the threshold (1099 held)", 1_099),
IGNORED     LINE   crates/store/tests/tix_latency.rs:245  [LS1: : ] {]  ] {
IGNORED     LINE   crates/store/tests/tix_latency.rs:246  [LS1: : let mut ns = Vec::with_capacity(COMMITS);]  let mut ns = Vec::with_capacity(COMMITS);
IGNORED     LINE   crates/store/tests/tix_latency.rs:247  [LS1: : for k in 0..COMMITS {]  for k in 0..COMMITS {
IGNORED     LINE   crates/store/tests/tix_latency.rs:248  [LS1: : let at = root.join(format!("commit-{held}-{k}"));]  let at = root.join(format!("commit-{held}-{k}"));
IGNORED     LINE   crates/store/tests/tix_latency.rs:249  [LS1: : fs::create_dir_all(&at).unwrap();]  fs::create_dir_all(&at).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:250  [LS1: : let mut writer = BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7).unwrap();]  let mut writer = BarFile::open_or_create(&at, path(Timeframe::SECOND_1), 7).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:251  [LS1: : writer.append(&all[..held]).unwrap();]  writer.append(&all[..held]).unwrap();
IGNORED     LINE   crates/store/tests/tix_latency.rs:252  [LS1: : let start = Instant::now();]  let start = Instant::now();
IGNORED     LINE   crates/store/tests/tix_latency.rs:253  [LS1: : black_box(writer.append(black_box(&all[held..=held])).unwrap());]  black_box(writer.append(black_box(&all[held..=held])).unwrap());
IGNORED     LINE   crates/store/tests/tix_latency.rs:254  [LS1: : ns.push(start.elapsed().as_nanos() as u64);]  ns.push(start.elapsed().as_nanos() as u64);
IGNORED     LINE   crates/store/tests/tix_latency.rs:255  [LS1: : drop(writer);]  drop(writer);
IGNORED     LINE   crates/store/tests/tix_latency.rs:256  [LS1: : let _ = fs::remove_dir_all(&at);]  let _ = fs::remove_dir_all(&at);
IGNORED     LINE   crates/store/tests/tix_latency.rs:257  [LS1: : }]  }
IGNORED     LINE   crates/store/tests/tix_latency.rs:258  [LS1: : report(&format!("{name}, n={COMMITS}"), ns);]  report(&format!("{name}, n={COMMITS}"), ns);
IGNORED     LINE   crates/store/tests/tix_latency.rs:260  [LS1: : let _ = fs::remove_dir_all(&root);]  let _ = fs::remove_dir_all(&root);
IGNORED     LINE   crates/store/tests/tix_latency.rs:261  [LS1: : }]  }
(F)         BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False  [LS1]  }) if asked == SYMBOL + 1
(F)         LINE   crates/store/tests/tix_small_month.rs:160  [LS1]  other => panic!("not committed: {other:?}"),
(F)         BRANCH crates/store/tests/tix_small_month.rs:461:74 False  [LS1]  matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
(F)         LINE   crates/store/tests/tix_small_month.rs:463  [LS1]  reader.time_lookup()
(F)         LINE   crates/store/tests/tix_warn_once.rs:127  [LS1]  other => panic!("{key} is not a count: {other:?}"),
(F)         LINE   crates/store/tests/tix_warn_once.rs:160  [LS1]  other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
(F)         LINE   crates/store/tests/tix_warn_once.rs:162  [LS1]  other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
(F)         LINE   crates/store/tests/unit.rs:110  [LS1]  input.len()
(F)         BRANCH crates/store/tests/unit.rs:399:13 False  [LS1]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F)         BRANCH crates/store/tests/unit.rs:399:36 False  [LS1]  assert!(real.ohlc_is_sane() && real.counts_are_sane());
(F)         LINE   crates/store/tests/unit.rs:527  [LS1]  _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
(F)         BRANCH crates/store/tests/unit.rs:868:13 False  [LS1]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(F)         BRANCH crates/store/tests/unit.rs:868:54 False  [LS1]  assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
(U)         BRANCH crates/store/tests/unit.rs:908:16 False  [LS1]  if one.magic()[..7] == other.magic()[..7]
(F)         BRANCH crates/store/tests/unit.rs:1475:32 False  [LS1]  read == old || read == new,
(F)         LINE   crates/store/tests/unit.rs:1820  [LS1]  matches!(
(F)         LINE   crates/store/tests/unit.rs:1859  [LS1]  matches!(
(F)         LINE   crates/store/tests/unit.rs:1874  [LS1]  matches!(
(F)         BRANCH crates/store/tests/unit.rs:2047:26 False  [LS1]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F)         BRANCH crates/store/tests/unit.rs:2047:43 False  [LS1]  .all(|c| !c.is_empty() && *c != ".." && *c != "."),
(F)         LINE   crates/store/tests/unit.rs:2096  [LS1]  vendor.as_str(),
(F)         BRANCH crates/store/tests/unit.rs:2173:13 False  [LS1]  err.to_string().contains(field) && err.to_string().contains("case"),
(F)         BRANCH crates/store/tests/unit.rs:2173:48 False  [LS1]  err.to_string().contains(field) && err.to_string().contains("case"),
(F)         LINE   crates/store/tests/unit.rs:2826  [LS1]  a.as_str(),
(F)         LINE   crates/store/tests/unit.rs:2827  [LS1]  b.as_str()
(F)         LINE   crates/store/tests/unit.rs:2833  [LS1]  a.as_str()
(F)         LINE   crates/store/tests/unit.rs:2893  [LS1]  tf.as_str()
(F)         LINE   crates/store/tests/verdict.rs:129  [LS1]  Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
(F)         LINE   crates/store/tests/verdict.rs:168  [LS1]  other => panic!("100x must be refused table_too_large, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:228:13 False  [LS1]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F)         BRANCH crates/store/tests/verdict.rs:228:36 False  [LS1]  got.cost.reads <= 5 && got.cost.opens <= 2,
(F)         LINE   crates/store/tests/verdict.rs:395  [LS1]  other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:449:13 False  [LS1]  let Found::Record { ordinal, record } = &answer.found else {
(F)         LINE   crates/store/tests/verdict.rs:450  [LS1]  panic!("{whole} must be found");
(U)         BRANCH crates/store/tests/verdict.rs:584:12 False  [LS1]  if count > 2 {
(U)         LINE   crates/store/tests/verdict.rs:588  [LS1]  }
(F)         LINE   crates/store/tests/verdict.rs:728  [LS1]  other => panic!("expected image_missing, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:793  [LS1]  other => panic!("expected generation_collision, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:1318  [LS1]  other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:1821  [LS1]  other => panic!(
(F)         LINE   crates/store/tests/verdict.rs:1823  [LS1]  clean.len()
(F)         LINE   crates/store/tests/verdict.rs:1856  [LS1]  other => panic!("expected record_corrupt, got {other:?}"),
(F)         BRANCH crates/store/tests/verdict.rs:1890:17 False  [LS1]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F)         BRANCH crates/store/tests/verdict.rs:1890:40 False  [LS1]  assert!(got.cost.opens <= 2 && got.cost.reads == 5);
(F)         BRANCH crates/store/tests/verdict.rs:2123:17 False  [LS1]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F)         BRANCH crates/store/tests/verdict.rs:2123:40 False  [LS1]  assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
(F)         LINE   crates/store/tests/verdict.rs:2283  [LS1]  feed.as_str(),
(F)         LINE   crates/store/tests/verdict.rs:2284  [LS1]  timeframe.as_str()
(F)         BRANCH crates/store/tests/verdict.rs:2358:58 False  [LS1]  matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
(F)         LINE   crates/store/tests/verdict.rs:2435  [LS1]  other => panic!("expected lock_stalled, got {other:?}"),
(F)         LINE   crates/store/tests/verdict.rs:2455  [LS1]  matches!(
(F)         BRANCH crates/store/tests/verdict.rs:2545:21 False  [LS1]  image.extension().is_some_and(|ext| ext == "vdt")
(F)         BRANCH crates/store/tests/verdict.rs:2546:28 False  [LS1]  && image.parent().is_some_and(|dir| dir.ends_with("2031"))
(F)         BRANCH crates/store/tests/verdict.rs:2547:28 False  [LS1]  && image.exists(),
(F)         LINE   crates/store/tests/verdict.rs:2549  [LS1]  image.display()
(F)         LINE   crates/store/tests/verdict.rs:2554  [LS1]  other => panic!("expected index_missing, got {other}"),
(F)         BRANCH crates/store/tests/verdict.rs:2604:13 False  [LS1]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F)         BRANCH crates/store/tests/verdict.rs:2604:22 False  [LS1]  assert!(named && (1..=2).contains(&seen), "seen {seen}");
(F)         LINE   crates/store/tests/verdict.rs:2730  [LS1]  other => panic!("{other}"),
(F)         BRANCH crates/store/tests/verdict.rs:2782:9 False  [LS1]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F)         BRANCH crates/store/tests/verdict.rs:2782:61 False  [LS1]  said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
(F)         LINE   crates/store/tests/write.rs:1402  [LS1]  sidecar.display()
(F)         LINE   crates/store/tests/write.rs:1419  [LS1]  sidecar.display()
(F)         BRANCH crates/store/tests/write.rs:1564:13 False  [LS1]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F)         BRANCH crates/store/tests/write.rs:1564:62 False  [LS1]  text.contains(&on_disk.display().to_string()) && text.contains("committed"),
(F)         LINE   crates/store/tests/write.rs:1619  [LS1]  panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
(F)         LINE   crates/store/tests/write.rs:1663  [LS1]  panic!(
(F)         BRANCH crates/store/tests/write.rs:2103:85 False  [LS1]  Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
(F)         LINE   crates/telemetry/src/level.rs:142  [LS4]  level.rank()
(F)         BRANCH crates/telemetry/src/lib.rs:423:13 False  [LS4]  output.status.success() && stdout.contains("1 passed"),
(F)         BRANCH crates/telemetry/src/lib.rs:423:40 False  [LS4]  output.status.success() && stdout.contains("1 passed"),
(F)         LINE   crates/telemetry/src/record.rs:841  [LS4]  matches!(
(F)         BRANCH crates/telemetry/src/run_dirs.rs:234:21 False  [LS4]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F)         BRANCH crates/telemetry/src/run_dirs.rs:234:44 False  [LS4]  assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
(F)         BRANCH crates/telemetry/src/run_dirs.rs:240:21 False  [LS4]  !other.sub.starts_with(&format!("{}/", dir.sub))
(F)         BRANCH crates/telemetry/src/run_dirs.rs:241:28 False  [LS4]  && !dir.sub.starts_with(&format!("{}/", other.sub)),
(F)         LINE   crates/telemetry/src/sink.rs:4688  [LS4]  panic!(
(F)         LINE   crates/telemetry/src/sink.rs:4690  [LS4]  path.display(),
(F)         LINE   crates/telemetry/src/sink.rs:4691  [LS4]  String::from_utf8_lossy(line)
(F)         BRANCH crates/telemetry/src/sink.rs:4824:21 False  [LS4]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F)         BRANCH crates/telemetry/src/sink.rs:4824:52 False  [LS4]  assert!(target.append(b"b").is_ok() && target.sync().is_ok());
(F)         BRANCH crates/telemetry/src/sink.rs:5024:17 False  [LS4]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F)         BRANCH crates/telemetry/src/sink.rs:5024:65 False  [LS4]  said.contains(&MAX_TARGET_BYTES.to_string()) && said.contains("level_for"),
(F)         LINE   crates/telemetry/src/sink.rs:5106  [LS4]  written[0].at_unix_millis
(F)         LINE   crates/telemetry/src/sink.rs:5143  [LS4]  pair[1].seq,
(F)         LINE   crates/telemetry/src/sink.rs:5144  [LS4]  pair[1].at_unix_millis,
(F)         LINE   crates/telemetry/src/sink.rs:5145  [LS4]  pair[0].seq,
(F)         LINE   crates/telemetry/src/sink.rs:5146  [LS4]  pair[0].at_unix_millis
(F)         LINE   crates/telemetry/src/sink.rs:5476  [LS4]  span * 2
(F)         LINE   crates/telemetry/src/sink.rs:5695  [LS4]  path.display()
(F)         BRANCH crates/telemetry/src/sink.rs:5712:17 False  [LS4]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F)         BRANCH crates/telemetry/src/sink.rs:5712:40 False  [LS4]  assert!(!current.is_empty() && !first_rolled.is_empty());
(F)         BRANCH crates/telemetry/src/sink.rs:5757:13 False  [LS4]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F)         BRANCH crates/telemetry/src/sink.rs:5757:30 False  [LS4]  after_one > 0 && after_one < MIN_FILE_BYTES,
(F)         LINE   crates/telemetry/src/sink.rs:6079  [LS4]  rolled.health()
(F)         BRANCH crates/telemetry/src/sink.rs:6621:13 False  [LS4]  why.contains("another") && why.contains(&dir.display().to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6621:40 False  [LS4]  why.contains("another") && why.contains(&dir.display().to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6687:17 False  [LS4]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F)         BRANCH crates/telemetry/src/sink.rs:6687:54 False  [LS4]  assert!(text.contains("before the close") && text.contains("the successor's line"));
(F)         BRANCH crates/telemetry/src/sink.rs:6707:13 False  [LS4]  said.contains("closed") && said.contains("no space left"),
(F)         BRANCH crates/telemetry/src/sink.rs:6707:40 False  [LS4]  said.contains("closed") && said.contains("no space left"),
(F)         BRANCH crates/telemetry/src/sink.rs:6749:13 False  [LS4]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F)         BRANCH crates/telemetry/src/sink.rs:6749:53 False  [LS4]  named.contains("ahead of the clock") && named.contains(&future.to_string()),
(F)         LINE   crates/telemetry/src/sink.rs:7663  [LS4]  other => panic!("{dimension}={value}: {other:?}"),
(F)         BRANCH crates/telemetry/src/sink.rs:8209:21 False  [LS4]  assert!(!health.reaper_behind() && !health.is_loud());
(F)         BRANCH crates/telemetry/src/sink.rs:8209:48 False  [LS4]  assert!(!health.reaper_behind() && !health.is_loud());
(F)         BRANCH crates/telemetry/src/sink.rs:8951:21 False  [LS4]  let AuditLine::Found(record) = audit_read(&dir, serial, pointer) else {
(F)         LINE   crates/telemetry/src/sink.rs:8952  [LS4]  panic!("{pointer:?}");
(F)         LINE   crates/telemetry/src/sink.rs:10275  [LS4]  other => panic!("{dimension}={value}: {other:?}"),
IGNORED     LINE   crates/telemetry/src/sink.rs:10406  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  fn an_audit_line_survives_2_gib_of_refusals_at_full_size() {]  fn an_audit_line_survives_2_gib_of_refusals_at_full_size() {
IGNORED     LINE   crates/telemetry/src/sink.rs:10407  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  survives(]  survives(
IGNORED     LINE   crates/telemetry/src/sink.rs:10408  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  "survives-full",]  "survives-full",
IGNORED     LINE   crates/telemetry/src/sink.rs:10409  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  2 * 1024 * 1024 * 1024,]  2 * 1024 * 1024 * 1024,
IGNORED     LINE   crates/telemetry/src/sink.rs:10412  [LS4: [L-BCL-3: inside an #[ignore]d test the gate never runs (operator's item)]  }]  }
(F)         BRANCH crates/telemetry/src/sink.rs:11095:17 False  [LS4]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F)         BRANCH crates/telemetry/src/sink.rs:11095:57 False  [LS4]  assert!(unknown.overlaps(i64::MIN, i64::MIN) && unknown.overlaps(i64::MAX, i64::MAX));
(F)         BRANCH crates/telemetry/src/sink.rs:11105:36 False  [LS4]  .is_some_and(|why| why.contains("first line does not decode")
(F)         BRANCH crates/telemetry/src/sink.rs:11160:17 False  [LS4]  why.contains("last 64 KiB hold no line") && !why.contains("first line")
(F)         BRANCH crates/telemetry/src/sink.rs:11226:17 False  [LS4]  why.contains("first line does not decode") && !why.contains("last 64 KiB")
(F)         BRANCH crates/telemetry/src/sink.rs:11231:17 False  [LS4]  assert!(a0 < a1 && a1 < a2, "premise");
(F)         BRANCH crates/telemetry/src/sink.rs:11231:28 False  [LS4]  assert!(a0 < a1 && a1 < a2, "premise");
(F)         BRANCH crates/telemetry/src/sink.rs:11262:17 False  [LS4]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F)         BRANCH crates/telemetry/src/sink.rs:11262:44 False  [LS4]  assert!(closed.overlaps(a1, a1) && !closed.overlaps(a3, a3));
(F)         BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:13 False  [LS4]  assert!(!parked.failed && parked.open);
(F)         BRANCH crates/telemetry/src/sink_recovery_tests.rs:320:31 False  [LS4]  assert!(!parked.failed && parked.open);
(F)         LINE   crates/telemetry/src/state_events_tests.rs:85  [LS4]  || panic!("{path} is named by a row and not read by this test"),
(F)         LINE   crates/telemetry/src/state_events_tests.rs:105  [LS4]  level.label(),
(F)         LINE   crates/telemetry/src/state_events_tests.rs:113  [LS4]  level.label()
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:176:17 False  [LS4]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:176:43 False  [LS4]  assert!(!row.change.is_empty() && !row.site.is_empty());
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:321:13 False  [LS4]  site.contains(&format!("\"{}\"", row.message))
(F)         BRANCH crates/telemetry/src/state_events_tests.rs:322:20 False  [LS4]  && site.contains(&format!("\"{}\"", row.target)),
(F)         BRANCH crates/telemetry/src/tail.rs:1858:17 False  [LS4]  assert!(newest > 0 && newest < READ_BLOCK);
(F)         BRANCH crates/telemetry/src/tail.rs:1858:31 False  [LS4]  assert!(newest > 0 && newest < READ_BLOCK);
(F)         LINE   crates/telemetry/tests/cloud_mutants_sink.rs:263  [LS4]  other => panic!("a directory is not a rolled-off line: {other:?}"),
OPERATOR    LINE   crates/telemetry/tests/global_absent.rs:106  [LS4: [BY DESIGN, for the Mac/operator: an argument of emit_if! that runs only when a sink admits; this binary installs none and the test asserts the argument never r]  "did not land",
OPERATOR    LINE   crates/telemetry/tests/global_absent.rs:107  [LS4: [BY DESIGN, for the Mac/operator: as line 106]  "bars" => telemetry::Value::Uint(count()),]  "bars" => telemetry::Value::Uint(count()),
(F)         BRANCH crates/vocab/src/expression_search.rs:506:13 False  [LS3]  compared > 100_000 && admitted > 1_000,
(F)         BRANCH crates/vocab/src/expression_search.rs:506:35 False  [LS3]  compared > 100_000 && admitted > 1_000,
(F)         BRANCH crates/vocab/src/expression_search.rs:515:21 False  [LS3]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F)         BRANCH crates/vocab/src/expression_search.rs:515:46 False  [LS3]  assert!(!admits(code, length) && !valid_prefix(code, length));
(F)         BRANCH crates/vocab/src/expression_search.rs:626:17 False  [LS3]  assert!(choice < call && call < paused);
(F)         BRANCH crates/vocab/src/expression_search.rs:626:34 False  [LS3]  assert!(choice < call && call < paused);
(F)         BRANCH crates/vocab/src/expression_search.rs:671:17 False  [LS3]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F)         BRANCH crates/vocab/src/expression_search.rs:671:54 False  [LS3]  assert!(printed.contains("offered: [], ") && printed.contains("digits: [], "));
(F)         BRANCH crates/vocab/src/expression_search.rs:742:17 False  [LS3]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F)         BRANCH crates/vocab/src/expression_search.rs:742:37 False  [LS3]  assert!(accepted > 1_000 && refused > 1_000, "{accepted} {refused}");
(F)         BRANCH crates/vocab/src/implication.rs:292:63 True  [LS3]  assert!(!(implies(*above_high, *above_low) && implies(*above_low, *above_high)));
(F)         LINE   crates/vocab/src/implication.rs:329  [LS3]  rows.len()
(F)         BRANCH crates/vocab/src/mask.rs:419:46 False  [LS3]  !bar.hits(&wider) || bar.hits(&base),
(F)         BRANCH crates/vocab/src/mask.rs:661:17 False  [LS3]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F)         BRANCH crates/vocab/src/mask.rs:661:32 False  [LS3]  assert!(less.get(5) && !less.get(70) && less.get(200));
(F)         BRANCH crates/vocab/src/mask.rs:661:49 False  [LS3]  assert!(less.get(5) && !less.get(70) && less.get(200));
(U)         LINE   crates/vocab/src/table.rs:1704  [LS3]  None
(F)         BRANCH crates/vocab/src/table.rs:1977:13 False  [LS3]  worst_hit <= 4 && worst_miss <= 7,
(F)         BRANCH crates/vocab/src/table.rs:1977:31 False  [LS3]  worst_hit <= 4 && worst_miss <= 7,
(F)         BRANCH crates/vocab/src/table.rs:2034:17 False  [LS3]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2034:34 False  [LS3]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2034:52 False  [LS3]  assert!(!clean.get(6) && !clean.get(19) && !clean.get(25));
(F)         BRANCH crates/vocab/src/table.rs:2108:17 False  [LS3]  assert!(m.get(0) && m.popcount() == 1);
(F)         BRANCH crates/vocab/src/table.rs:2108:29 False  [LS3]  assert!(m.get(0) && m.popcount() == 1);
(F)         BRANCH crates/vocab/src/tolerance.rs:570:45 True  [LS3]  !(shared && range >= RUNG_EXCLUSIVE_MIN_RANGE),
(F)         LINE   crates/vocab/tests/expression.rs:403  [LS3]  bytes.get(..16)
(F)         BRANCH crates/vocab/tests/expression.rs:414:13 False  [LS3]  assert!(accepted > 0 && refused > accepted);
(F)         BRANCH crates/vocab/tests/expression.rs:414:29 False  [LS3]  assert!(accepted > 0 && refused > accepted);
(F)         LINE   crates/vocab/tests/expression_search.rs:82  [LS3]  Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),
(F)         BRANCH crates/vocab/tests/expression_search.rs:201:13 False  [LS3]  assert!(saw_candidate && saw_backtrack);
(F)         BRANCH crates/vocab/tests/expression_search.rs:201:30 False  [LS3]  assert!(saw_candidate && saw_backtrack);
(F)         BRANCH crates/vocab/tests/expression_search_readiness.rs:126:9 False  [LS3]  let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {
(F)         LINE   crates/vocab/tests/expression_search_readiness.rs:127  [LS3]  panic!("the last NOT must finish the maximum-capacity program");
(F)         BRANCH crates/vocab/tests/mask.rs:315:27 False  [LS3]  before || !after,
(F)         BRANCH crates/vocab/tests/no_float.rs:145:36 False  [LS3]  *module == "lib.rs" || declared.contains(*module),
(F)         LINE   crates/vocab/tests/no_py_language.rs:282  [LS3]  root.display(),
(F)         LINE   crates/vocab/tests/no_py_language.rs:283  [LS3]  String::from_utf8_lossy(&listed.stderr).trim()
(F)         BRANCH crates/vocab/tests/no_py_language.rs:447:12 True  [LS3]  if let Err(why) = verdict(&found) {
(F)         LINE   crates/vocab/tests/no_py_language.rs:448  [LS3]  panic!("{why}");
(F)         LINE   crates/vocab/tests/no_py_language.rs:666  [LS3]  String::from_utf8_lossy(&init.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:679  [LS3]  String::from_utf8_lossy(&add.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:783  [LS3]  String::from_utf8_lossy(&init.stderr)
(F)         LINE   crates/vocab/tests/no_py_language.rs:815  [LS3]  String::from_utf8_lossy(&add.stderr)
(F)         LINE   crates/vocab/tests/no_runtime_spawn.rs:2560  [LS3]  sources.len()
(F)         LINE   crates/vocab/tests/no_runtime_spawn.rs:2585  [LS3]  failures.join("\n  ")
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:2910:26 False  [LS3]  .any(|f| f.starts_with("crates/x/src/lib.rs:") && f.contains("outside test code")),
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:13 False  [LS3]  assert!(out.found.is_empty() && out.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4219:37 False  [LS3]  assert!(out.found.is_empty() && out.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:13 False  [LS3]  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4270:50 False  [LS3]  assert!(declared.decls[0].also.is_empty() && !declared.decls[0].test);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:13 False  [LS3]  assert!(block.found.is_empty() && block.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4303:39 False  [LS3]  assert!(block.found.is_empty() && block.in_tests == 0);
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:13 False  [LS3]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:41 False  [LS3]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4406:67 False  [LS3]  assert!(named.targets.is_empty() && named.every.is_empty() && named.unread.is_empty());
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:13 False  [LS3]  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4407:42 False  [LS3]  assert!(is_manifest("Cargo.toml") && is_manifest("crates/x/Cargo.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:13 False  [LS3]  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F)         BRANCH crates/vocab/tests/no_runtime_spawn.rs:4408:47 False  [LS3]  assert!(!is_manifest("web/Cargo.toml") && !is_manifest("crates/x/a.toml"));
(F)         LINE   crates/vocab/tests/stale_claims.rs:158  [LS3]  with_commas(count)
(F)         LINE   crates/vocab/tests/table.rs:125  [LS3]  doc.len(),
(F)         BRANCH crates/vocab/tests/table.rs:270:13 False  [LS3]  let BitStatus::Void { reason } = def.status else {
(F)         LINE   crates/vocab/tests/table.rs:271  [LS3]  unreachable!("235..=273 are void")
(F)         BRANCH crates/vocab/tests/table.rs:373:13 False  [LS3]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         BRANCH crates/vocab/tests/table.rs:373:39 False  [LS3]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         BRANCH crates/vocab/tests/table.rs:373:63 False  [LS3]  !name.starts_with('_') && !name.ends_with('_') && !name.contains("__"),
(F)         LINE   crates/vocab/tests/table.rs:432  [LS3]  first - 1,
(F)         BRANCH crates/vocab/tests/table.rs:561:45 False  [LS3]  n == ConditionMask::BITS || ALLOWED.contains(&n),
(F)         BRANCH crates/vocab/tests/table.rs:813:17 False  [LS3]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F)         BRANCH crates/vocab/tests/table.rs:813:54 False  [LS3]  !status_cells.contains("retired") && !status_cells.contains("void"),
(F)         BRANCH crates/vocab/tests/table.rs:819:21 False  [LS3]  status_cells.contains("retired")
(F)         BRANCH crates/vocab/tests/table.rs:820:28 False  [LS3]  && status_cells.contains(&format!("duplicates {duplicate_of}")),
(F)         BRANCH crates/vocab/tests/table.rs:843:9 False  [LS3]  retired_rows >= retired && void_rows >= void,
(F)         BRANCH crates/vocab/tests/table.rs:843:36 False  [LS3]  retired_rows >= retired && void_rows >= void,
(F)         BRANCH crates/vocab/tests/table.rs:1012:13 False  [LS3]  !name.contains("vix") && !name.contains("volatility_index"),
(F)         BRANCH crates/vocab/tests/table.rs:1012:38 False  [LS3]  !name.contains("vix") && !name.contains("volatility_index"),
(F)         LINE   crates/vocab/tests/workspace_is_rust.rs:397  [LS3]  out.len()
(F)         LINE   crates/vocab/tests/workspace_is_rust.rs:753  [LS3]  named.join("\n  ")
`````

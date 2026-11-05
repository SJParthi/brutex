# gaps-PS2.txt.md

Written by BASE-COV-PULL-PS2 from Route B run 5 on the code tip `02f268e24f8bbf230099ed53ccdb8656aadf68d2` (the kit's `d3-route-b-cloud.sh`, unchanged, on the pinned nightly): the per-file table of every file of the set (crate `pull`, reading whole:pull) and every uncovered line and side of G2's 93 files, each with its class. The (U) proofs and the (F) reasons are in `fu-PS2.txt.md`. Rows of the other groups' files are theirs and are not listed. One fenced block below holds it.

`````text
BASE-COV-PULL-PS2 run 5: Route B on 02f268e24f8bbf230099ed53ccdb8656aadf68d2, reading whole:pull, set = the 222 src/ and tests/ files of crates/pull at the tip (every "coverage <file> whole"; benches are not built by cargo test, as in the kit's f98a899398f6 set).
ROUTE-B START 2026-11-05 10:24:45 IST tree /home/user/brutex tip 02f268e24f8bbf230099ed53ccdb8656aadf68d2 packages pull K /home/user/ps2-work/deep P /home/user/ps2-work/prof/02f268e24f8b-5-B (entries before: 0)
== build START 2026-11-05 10:24:45 IST: cargo test -p pull --locked --no-run --message-format=json (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', opt-level 0, CARGO_TARGET_DIR=/home/user/ps2-work/deep/cov)
== build rc=0 END 2026-11-05 10:25:36 IST (0 min); 167 artifacts, 129 reused
== test pull START 2026-11-05 10:25:36 IST
== test pull rc=101 END 2026-11-05 10:29:09 IST (3 min); tests passed 3177 failed 6 ignored 54
ROUTE-B DONE 2026-11-05 10:53:53 IST: every package measured and read (pull)
whole pull (222 files): lines 159895/166046 (6151 uncovered), branches 11903/14346 (2443 sides)
G2 (93 files: SHARDS.md's 77 and the 16 test files this session added): lines 58766/60545 (1779 uncovered), branches 4140/4724 (584 sides)
G2's uncovered rows: 2363 (1779 lines, 584 sides); by class: (F) by hand 9; (U) 125; (U, type invariant) 21; (F) by pattern 614; IGNORED 1594

Per-file table (G2 marked "G2"):
crates/pull/src/alerts_ps2_tests.rs lines 260/265, branches 9/18 G2
crates/pull/src/alerts_tests.rs lines 1615/1700, branches 49/82 G2
crates/pull/src/alerts.rs lines 982/982, branches 165/166 G2
crates/pull/src/archive.rs lines 472/480, branches 28/32
crates/pull/src/audit_ledger_tests.rs lines 148/148, branches 0/0 G2
crates/pull/src/audit_ledger.rs lines 50/50, branches 10/10 G2
crates/pull/src/base_cov_codes_tests.rs lines 423/423, branches 0/0 G2
crates/pull/src/base_cov_words_tests.rs lines 358/359, branches 1/2 G2
crates/pull/src/calendar_cov_tests.rs lines 212/212, branches 4/4 G2
crates/pull/src/calendar_tradable_tests.rs lines 765/784, branches 31/46
crates/pull/src/calendar.rs lines 2557/2610, branches 342/370 G2
crates/pull/src/capture.rs lines 317/329, branches 4/10
crates/pull/src/cash_auction.rs lines 836/912, branches 97/100
crates/pull/src/cash_session_cache_lock_tests.rs lines 291/302, branches 15/22
crates/pull/src/cash_session_cache.rs lines 1393/1582, branches 75/114
crates/pull/src/census_store_cov_tests.rs lines 34/34, branches 0/0
crates/pull/src/census_store_kept_tests.rs lines 712/788, branches 40/46 G2
crates/pull/src/census_store_link_tests.rs lines 549/550, branches 8/14 G2
crates/pull/src/census_store_ps2_tests.rs lines 1119/1120, branches 10/18 G2
crates/pull/src/census_store_tests.rs lines 1147/1178, branches 46/78 G2
crates/pull/src/census_store.rs lines 2329/2340, branches 366/372 G2
crates/pull/src/chain_ps2_tests.rs lines 61/62, branches 3/4 G2
crates/pull/src/chain.rs lines 356/362, branches 25/30 G2
crates/pull/src/config.rs lines 386/386, branches 60/60 G2
crates/pull/src/corporate_actions_cov_tests.rs lines 289/291, branches 9/12
crates/pull/src/corporate_actions_tests.rs lines 1279/1293, branches 52/84
crates/pull/src/corporate_actions.rs lines 1000/1000, branches 166/166
crates/pull/src/csv.rs lines 1016/1017, branches 95/98 G2
crates/pull/src/day_universe.rs lines 112/112, branches 14/22
crates/pull/src/daycheck.rs lines 352/352, branches 27/30 G2
crates/pull/src/dhan.rs lines 118/118, branches 2/2 G2
crates/pull/src/disk_projection_tests.rs lines 352/422, branches 1/8 G2
crates/pull/src/disk_projection.rs lines 278/278, branches 29/30 G2
crates/pull/src/emit_sites.rs lines 1096/1109, branches 6/6 G2
crates/pull/src/expiry_alias.rs lines 679/687, branches 72/90
crates/pull/src/fetch.rs lines 565/584, branches 50/56
crates/pull/src/fno_membership_cov_tests.rs lines 71/72, branches 1/2
crates/pull/src/fno_membership_tests.rs lines 1005/1026, branches 50/64
crates/pull/src/fno_membership.rs lines 1599/1681, branches 212/258
crates/pull/src/fno_ps2_tests.rs lines 79/80, branches 1/2 G2
crates/pull/src/fno.rs lines 608/612, branches 53/62 G2
crates/pull/src/fnowork_cov_tests.rs lines 201/203, branches 0/0
crates/pull/src/fnowork_futures_tests.rs lines 1132/1188, branches 23/40 G2
crates/pull/src/fnowork.rs lines 871/871, branches 62/62
crates/pull/src/fold_ps2_tests.rs lines 18/18, branches 0/0 G2
crates/pull/src/fold.rs lines 1188/1200, branches 143/152 G2
crates/pull/src/folder.rs lines 206/212, branches 16/20
crates/pull/src/gap_backfill.rs lines 222/232, branches 36/48
crates/pull/src/gaps_not_owed_tests.rs lines 486/486, branches 26/26 G2
crates/pull/src/gaps.rs lines 714/720, branches 96/104
crates/pull/src/gdfl_archive.rs lines 1451/1520, branches 77/86
crates/pull/src/gdfl_backfill_tests.rs lines 863/868, branches 17/22 G2
crates/pull/src/gdfl_cm_admission_tests.rs lines 1282/1334, branches 6/12
crates/pull/src/gdfl_cm_auction_v2_tests.rs lines 399/410, branches 5/12
crates/pull/src/gdfl_cm_auction.rs lines 603/619, branches 79/82
crates/pull/src/gdfl_cm_ps2_tests.rs lines 91/91, branches 0/0 G2
crates/pull/src/gdfl_cm.rs lines 3147/3153, branches 359/364 G2
crates/pull/src/gdfl_dayzips_ps2_tests.rs lines 70/70, branches 0/0 G2
crates/pull/src/gdfl_dayzips_tests.rs lines 361/409, branches 0/2 G2
crates/pull/src/gdfl_dayzips.rs lines 201/202, branches 34/34 G2
crates/pull/src/gdfl_dropbox_cov_tests.rs lines 36/39, branches 5/8
crates/pull/src/gdfl_dropbox_tests.rs lines 2627/2745, branches 115/150
crates/pull/src/gdfl_dropbox.rs lines 1970/2133, branches 367/450
crates/pull/src/gdfl_fixtures.rs lines 197/198, branches 14/14
crates/pull/src/gdfl_format_change_tests.rs lines 657/658, branches 22/28 G2
crates/pull/src/gdfl_format_ps2_tests.rs lines 31/31, branches 0/0 G2
crates/pull/src/gdfl_format.rs lines 32/32, branches 4/4 G2
crates/pull/src/gdfl_ia16_tests.rs lines 2136/2187, branches 84/136
crates/pull/src/gdfl_ia3_tests.rs lines 660/869, branches 8/28 G2
crates/pull/src/gdfl_ia4_tests.rs lines 192/193, branches 4/6
crates/pull/src/gdfl_import_causes.rs lines 339/349, branches 6/6
crates/pull/src/gdfl_import_census_ps2_tests.rs lines 16/16, branches 0/0 G2
crates/pull/src/gdfl_import_census.rs lines 133/133, branches 12/12 G2
crates/pull/src/gdfl_import_cov_tests.rs lines 899/899, branches 16/26
crates/pull/src/gdfl_import_day.rs lines 1373/1476, branches 217/246
crates/pull/src/gdfl_import_filing_ps2_tests.rs lines 498/509, branches 9/14 G2
crates/pull/src/gdfl_import_filing.rs lines 952/953, branches 137/140 G2
crates/pull/src/gdfl_import_ia2_tests.rs lines 449/551, branches 11/22
crates/pull/src/gdfl_import_journal.rs lines 775/788, branches 193/208
crates/pull/src/gdfl_import_lineage.rs lines 367/385, branches 64/82
crates/pull/src/gdfl_import_quality_ps2_tests.rs lines 115/115, branches 0/0 G2
crates/pull/src/gdfl_import_quality.rs lines 468/468, branches 108/110 G2
crates/pull/src/gdfl_import_scope.rs lines 250/250, branches 14/18 G2
crates/pull/src/gdfl_import_split_tests.rs lines 401/414, branches 34/44
crates/pull/src/gdfl_import_tests.rs lines 4420/4458, branches 188/286
crates/pull/src/gdfl_import.rs lines 1753/1770, branches 213/214
crates/pull/src/gdfl_lineage_tests.rs lines 898/1015, branches 11/26
crates/pull/src/gdfl_lineage.rs lines 817/822, branches 146/154
crates/pull/src/gdfl_nfo_attack_tests.rs lines 1264/1301, branches 142/178
crates/pull/src/gdfl_nfo_tests.rs lines 1718/2523, branches 36/190
crates/pull/src/gdfl_nfo.rs lines 916/920, branches 177/180
crates/pull/src/gdfl_r2_attack_tests.rs lines 1324/1328, branches 148/160 G2
crates/pull/src/gdfl_r3_attack_tests.rs lines 982/997, branches 125/140
crates/pull/src/gdfl_r4_attack_tests.rs lines 586/595, branches 48/62
crates/pull/src/gdfl_rekey_cov_tests.rs lines 38/38, branches 0/0
crates/pull/src/gdfl_rekey_ps2_tests.rs lines 950/954, branches 11/20 G2
crates/pull/src/gdfl_rekey_tests.rs lines 1803/2011, branches 75/144 G2
crates/pull/src/gdfl_rekey.rs lines 1500/1511, branches 258/268 G2
crates/pull/src/gdfl_run_cov_tests.rs lines 477/479, branches 1/2 G2
crates/pull/src/gdfl_run_ia2_tests.rs lines 766/942, branches 38/74
crates/pull/src/gdfl_run_ledger_cov_tests.rs lines 100/100, branches 0/0 G2
crates/pull/src/gdfl_run_ledger_ps2_tests.rs lines 252/255, branches 5/8 G2
crates/pull/src/gdfl_run_ledger.rs lines 418/423, branches 68/72 G2
crates/pull/src/gdfl_run_locks_cov_tests.rs lines 285/285, branches 5/10
crates/pull/src/gdfl_run_locks.rs lines 650/715, branches 146/158
crates/pull/src/gdfl_run_scope.rs lines 56/62, branches 7/8
crates/pull/src/gdfl_run_start_cov_tests.rs lines 90/90, branches 2/2
crates/pull/src/gdfl_run_start.rs lines 70/71, branches 5/6
crates/pull/src/gdfl_run_stop_cov_tests.rs lines 66/67, branches 1/2
crates/pull/src/gdfl_run_stop.rs lines 227/232, branches 36/40
crates/pull/src/gdfl_run_tests.rs lines 1094/1252, branches 12/26 G2
crates/pull/src/gdfl_run.rs lines 1419/1425, branches 166/170 G2
crates/pull/src/gdfl_scope_tests.rs lines 1837/1845, branches 38/54 G2
crates/pull/src/gdfl_sealed_tests.rs lines 305/305, branches 17/20 G2
crates/pull/src/gdfl_seconds_attack_tests.rs lines 1211/1213, branches 62/76 G2
crates/pull/src/gdfl_series_code_tests.rs lines 363/363, branches 2/4
crates/pull/src/gdfl_tickstore.rs lines 1149/1167, branches 111/120
crates/pull/src/gdfl_v3_ps2_tests.rs lines 27/27, branches 0/0 G2
crates/pull/src/gdfl_v3_tests.rs lines 630/759, branches 16/46 G2
crates/pull/src/gdfl_v3.rs lines 304/304, branches 60/60 G2
crates/pull/src/gdfl_wp11c_tests.rs lines 402/402, branches 6/6 G2
crates/pull/src/gdfl_wp12e_tests.rs lines 516/633, branches 9/14
crates/pull/src/groww.rs lines 41/41, branches 0/0 G2
crates/pull/src/http_cov_tests.rs lines 74/76, branches 2/4
crates/pull/src/http.rs lines 4316/4463, branches 324/416
crates/pull/src/identity_screen.rs lines 472/474, branches 63/64
crates/pull/src/ingest_cov_tests.rs lines 213/213, branches 2/4
crates/pull/src/ingest_failkind_tests.rs lines 574/574, branches 7/10
crates/pull/src/ingest_failkind.rs lines 124/125, branches 10/10
crates/pull/src/ingest.rs lines 3100/3268, branches 243/296
crates/pull/src/kite_cov_tests.rs lines 6/6, branches 0/0
crates/pull/src/kite.rs lines 57/57, branches 0/0
crates/pull/src/landing_run_tests.rs lines 1761/1890, branches 54/80 G2
crates/pull/src/landing_run.rs lines 338/338, branches 26/26 G2
crates/pull/src/lib.rs lines 5/5, branches 0/0 G2
crates/pull/src/listing_tests.rs lines 397/400, branches 3/4
crates/pull/src/listing.rs lines 237/237, branches 38/38
crates/pull/src/manifest_cov_tests.rs lines 16/16, branches 0/0
crates/pull/src/manifest.rs lines 1346/1366, branches 151/170
crates/pull/src/master_snapshots_cov_tests.rs lines 627/628, branches 2/4
crates/pull/src/master_snapshots_first_listed_tests.rs lines 423/424, branches 22/44
crates/pull/src/master_snapshots_first_listed.rs lines 531/569, branches 71/86
crates/pull/src/master_snapshots_tests.rs lines 1358/1514, branches 34/68
crates/pull/src/master_snapshots.rs lines 1663/1671, branches 256/270
crates/pull/src/masters_cov_tests.rs lines 92/92, branches 2/2
crates/pull/src/masters.rs lines 1643/1723, branches 173/214
crates/pull/src/nse.rs lines 496/508, branches 62/78
crates/pull/src/nseindex.rs lines 231/231, branches 14/14 G2
crates/pull/src/padded_tests.rs lines 414/415, branches 7/8 G2
crates/pull/src/padded.rs lines 109/109, branches 28/28 G2
crates/pull/src/pricing.rs lines 697/713, branches 33/52
crates/pull/src/rate.rs lines 254/254, branches 26/26 G2
crates/pull/src/reference_data_cov_tests.rs lines 418/418, branches 2/4
crates/pull/src/reference_data_tests.rs lines 965/1014, branches 24/40
crates/pull/src/reference_data.rs lines 1876/1934, branches 211/238
crates/pull/src/refusal.rs lines 109/114, branches 6/8
crates/pull/src/rehearsal_tests.rs lines 614/615, branches 14/18
crates/pull/src/rehearsal.rs lines 551/557, branches 95/98
crates/pull/src/repair_plan_not_owed_tests.rs lines 272/274, branches 7/10
crates/pull/src/repair_plan_ps2_tests.rs lines 254/255, branches 6/10 G2
crates/pull/src/repair_plan.rs lines 537/541, branches 61/66 G2
crates/pull/src/request_minutes.rs lines 1421/1447, branches 115/142
crates/pull/src/resolve.rs lines 610/639, branches 28/34
crates/pull/src/response_keep_tests.rs lines 2462/2470, branches 39/50 G2
crates/pull/src/response_keep.rs lines 1317/1322, branches 191/200 G2
crates/pull/src/riskfree.rs lines 1145/1147, branches 93/96 G2
crates/pull/src/rolling.rs lines 815/839, branches 64/80
crates/pull/src/scrub_blocks_tests.rs lines 92/92, branches 0/0 G2
crates/pull/src/scrub.rs lines 430/435, branches 21/24
crates/pull/src/secret.rs lines 122/122, branches 8/8 G2
crates/pull/src/series_identity.rs lines 672/673, branches 60/64
crates/pull/src/session.rs lines 692/703, branches 108/116
crates/pull/src/ssm_cov_tests.rs lines 159/159, branches 2/2
crates/pull/src/ssm.rs lines 896/940, branches 88/116
crates/pull/src/tenor.rs lines 192/196, branches 6/8
crates/pull/src/tickstore_write_tests.rs lines 1181/1351, branches 44/68
crates/pull/src/tickstore_write.rs lines 1003/1089, branches 167/188
crates/pull/src/totp.rs lines 301/306, branches 29/34
crates/pull/src/universe.rs lines 415/418, branches 12/12
crates/pull/src/upkeep_tests.rs lines 1475/1485, branches 55/78
crates/pull/src/upkeep.rs lines 933/934, branches 147/150
crates/pull/src/vendor_cov_tests.rs lines 27/27, branches 0/0
crates/pull/src/vendor.rs lines 2213/2248, branches 133/184
crates/pull/src/verdict_words.rs lines 93/94, branches 5/8
crates/pull/src/work.rs lines 129/129, branches 14/14 G2
crates/pull/src/zerodha_spool_cov_tests.rs lines 443/454, branches 23/42 G2
crates/pull/src/zerodha_spool.rs lines 834/841, branches 134/136 G2
crates/pull/src/zerodha_verify_cov_tests.rs lines 31/31, branches 0/0
crates/pull/src/zerodha_verify_tests.rs lines 3715/3759, branches 49/78
crates/pull/src/zerodha_verify.rs lines 2627/2863, branches 494/614
crates/pull/tests/alerts_ps2.rs lines 34/34, branches 0/0 G2
crates/pull/tests/alerts_seams.rs lines 78/79, branches 0/0
crates/pull/tests/allocation.rs lines 384/390, branches 5/8
crates/pull/tests/anchor.rs lines 516/522, branches 24/32
crates/pull/tests/broker.rs lines 761/768, branches 28/34 G2
crates/pull/tests/census_day_pace.rs lines 87/88, branches 4/8
crates/pull/tests/census_resume.rs lines 151/155, branches 7/12 G2
crates/pull/tests/census_v4.rs lines 549/628, branches 4/6
crates/pull/tests/census.rs lines 777/789, branches 24/38
crates/pull/tests/cloud_mutants_import.rs lines 1215/1225, branches 17/30 G2
crates/pull/tests/concurrency.rs lines 51/51, branches 0/0 G2
crates/pull/tests/derive.rs lines 1396/1408, branches 39/56
crates/pull/tests/folder.rs lines 489/492, branches 13/20
crates/pull/tests/futures_events.rs lines 130/131, branches 3/6
crates/pull/tests/futures_refusal_events.rs lines 70/70, branches 0/0 G2
crates/pull/tests/gdfl_bad_row_audit.rs lines 239/245, branches 3/6
crates/pull/tests/gdfl_census_stop.rs lines 146/151, branches 10/18
crates/pull/tests/integration.rs lines 237/238, branches 0/0
crates/pull/tests/no_scheduler_delete.rs lines 295/300, branches 96/112
crates/pull/tests/padded_refold.rs lines 389/403, branches 4/4 G2
crates/pull/tests/pipeline.rs lines 1048/1062, branches 18/28
crates/pull/tests/refusal.rs lines 309/316, branches 17/20
crates/pull/tests/request_gap_events.rs lines 105/106, branches 8/14
crates/pull/tests/sealed_month.rs lines 350/354, branches 17/24
crates/pull/tests/store_evidence.rs lines 31/336, branches 0/22 G2
crates/pull/tests/support/mod.rs lines 32/32, branches 3/4
crates/pull/tests/tls.rs lines 14/14, branches 0/0 G2
crates/pull/tests/token_identity.rs lines 222/225, branches 52/62
crates/pull/tests/unit.rs lines 4525/4553, branches 108/140
crates/pull/tests/zerodha_gaps.rs lines 1006/1012, branches 84/106 G2
crates/pull/tests/zerodha_identity.rs lines 1498/1510, branches 60/68 G2
crates/pull/tests/zerodha_repair.rs lines 473/476, branches 5/6 G2

Every uncovered line and side of G2, as "<class> <file:line[:col side]>: <text>":
(U) crates/pull/src/alerts.rs:1458:28 False: if let Some(mark) = acked.get_mut(i) {
(F) crates/pull/src/alerts_ps2_tests.rs:167: panic!("{put:?}");
(F) crates/pull/src/alerts_ps2_tests.rs:267: panic!("{refused:?}");
(F) crates/pull/src/alerts_ps2_tests.rs:294: panic!("{refused:?}");
(F) crates/pull/src/alerts_ps2_tests.rs:349: panic!("{refused:?}");
(F) crates/pull/src/alerts_ps2_tests.rs:387: panic!("{refused:?}");
(F) crates/pull/src/alerts_ps2_tests.rs:163:9 False: let Put::Written {
(F) crates/pull/src/alerts_ps2_tests.rs:193:13 False: assert!(!page.exists && page.alerts.is_empty());
(F) crates/pull/src/alerts_ps2_tests.rs:193:29 False: assert!(!page.exists && page.alerts.is_empty());
(F) crates/pull/src/alerts_ps2_tests.rs:224:13 False: assert!(by.starts_with(&record.text) && record.text.len() < by.len());
(F) crates/pull/src/alerts_ps2_tests.rs:224:45 False: assert!(by.starts_with(&record.text) && record.text.len() < by.len());
(F) crates/pull/src/alerts_ps2_tests.rs:266:9 False: let AlertError::Io { op, kind, text } = refused else {
(F) crates/pull/src/alerts_ps2_tests.rs:293:9 False: let AlertError::Io { op, kind, .. } = refused else {
(F) crates/pull/src/alerts_ps2_tests.rs:348:9 False: let Err(AlertError::Io { op, kind, .. }) = refused else {
(F) crates/pull/src/alerts_ps2_tests.rs:386:9 False: let AlertError::Io { op, .. } = refused else {
(F) crates/pull/src/alerts_tests.rs:78: Put::Duplicate { ordinal } => panic!("a duplicate of {ordinal}, not a write"),
(F) crates/pull/src/alerts_tests.rs:210: matches!(
(F) crates/pull/src/alerts_tests.rs:243: panic!("{got:?}")
(F) crates/pull/src/alerts_tests.rs:414: panic!("{:?}", kept[3])
(F) crates/pull/src/alerts_tests.rs:1105: matches!(
(F) crates/pull/src/alerts_tests.rs:1139: panic!("{refused:?}")
(F) crates/pull/src/alerts_tests.rs:1595: matches!(
IGNORED crates/pull/src/alerts_tests.rs:2070: fn spread(mut micros: Vec<u128>) -> (u128, u128, u128) {
IGNORED crates/pull/src/alerts_tests.rs:2071: micros.sort_unstable();
IGNORED crates/pull/src/alerts_tests.rs:2072: let at = |q: usize| micros[(micros.len() * q / 100).min(micros.len() - 1)];
IGNORED crates/pull/src/alerts_tests.rs:2073: (at(50), at(99), micros[micros.len() - 1])
IGNORED crates/pull/src/alerts_tests.rs:2074: }
IGNORED crates/pull/src/alerts_tests.rs:2081: fn the_operators_older_steps_timed() {
IGNORED crates/pull/src/alerts_tests.rs:2082: let scratch = Scratch::new("timed");
IGNORED crates/pull/src/alerts_tests.rs:2083: let records = 102_400_u32;
IGNORED crates/pull/src/alerts_tests.rs:2084: raw_queue(&scratch, &open_alerts(records));
IGNORED crates/pull/src/alerts_tests.rs:2085: for (label, before, later) in [
IGNORED crates/pull/src/alerts_tests.rs:2086: ("1,024 records after the page", records - 1_023, 1_024_u32),
IGNORED crates/pull/src/alerts_tests.rs:2087: ("51,201 records after the page", 51_200, 51_201),
IGNORED crates/pull/src/alerts_tests.rs:2088: ("101,376 records after the page", 1_025, 101_376),
IGNORED crates/pull/src/alerts_tests.rs:2089: ] {
IGNORED crates/pull/src/alerts_tests.rs:2090: let mut micros = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2091: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2092: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2093: let page = page_settled(scratch.root(), before).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2094: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2095: assert_eq!(page.alerts.len(), 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2096: assert_eq!(page.older_unacknowledged, before - 1 - 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2097: assert_eq!(u64::from(later), page.records - u64::from(before) + 1);
IGNORED crates/pull/src/alerts_tests.rs:2099: let (p50, p99, max) = spread(micros);
IGNORED crates/pull/src/alerts_tests.rs:2100: println!("page_settled, {label}: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
IGNORED crates/pull/src/alerts_tests.rs:2102: let mut one_read = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2103: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2104: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2105: let page = page_of(&scratch, Some(51_200));
IGNORED crates/pull/src/alerts_tests.rs:2106: one_read.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2107: assert_eq!(page.alerts.len(), 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2109: let (p50, p99, max) = spread(one_read);
IGNORED crates/pull/src/alerts_tests.rs:2110: println!(
IGNORED crates/pull/src/alerts_tests.rs:2116: let mut walk = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2117: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2118: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2119: let found = older(scratch.root()).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2120: walk.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2121: assert_eq!(found.alerts.len(), 101_376);
IGNORED crates/pull/src/alerts_tests.rs:2122: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2124: let (p50, p99, max) = spread(walk);
IGNORED crates/pull/src/alerts_tests.rs:2125: println!("older, 102,400 records: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
IGNORED crates/pull/src/alerts_tests.rs:2129: let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
IGNORED crates/pull/src/alerts_tests.rs:2130: let walker = {
IGNORED crates/pull/src/alerts_tests.rs:2131: let stop = std::sync::Arc::clone(&stop);
IGNORED crates/pull/src/alerts_tests.rs:2132: let root = scratch.root().to_path_buf();
IGNORED crates/pull/src/alerts_tests.rs:2133: std::thread::spawn(move || {
IGNORED crates/pull/src/alerts_tests.rs:2134: let mut walks = 0_u32;
IGNORED crates/pull/src/alerts_tests.rs:2135: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2136: assert!(older(&root).unwrap().alerts.len() >= 101_376);
IGNORED crates/pull/src/alerts_tests.rs:2137: walks += 1;
IGNORED crates/pull/src/alerts_tests.rs:2139: walks
IGNORED crates/pull/src/alerts_tests.rs:2140: })
IGNORED crates/pull/src/alerts_tests.rs:2142: let mut puts = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2143: for i in 0..200_u32 {
IGNORED crates/pull/src/alerts_tests.rs:2144: let key = format!("timed-put-{i}");
IGNORED crates/pull/src/alerts_tests.rs:2145: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2146: let put = put_at(scratch.root(), &alert(6, &key, "t"), NOW).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2147: puts.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2148: assert_eq!(written(put), records + 1 + i);
IGNORED crates/pull/src/alerts_tests.rs:2150: stop.store(true, Ordering::Relaxed);
IGNORED crates/pull/src/alerts_tests.rs:2151: let walks = walker.join().unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2152: let (p50, p99, max) = spread(puts);
IGNORED crates/pull/src/alerts_tests.rs:2153: println!(
IGNORED crates/pull/src/alerts_tests.rs:2159: for size in [7_u32, 102_400] {
IGNORED crates/pull/src/alerts_tests.rs:2160: let n: u64 = if size == 7 { 100 } else { 30 };
IGNORED crates/pull/src/alerts_tests.rs:2161: let mut micros = Vec::with_capacity(100);
IGNORED crates/pull/src/alerts_tests.rs:2162: for i in 0..n {
IGNORED crates/pull/src/alerts_tests.rs:2163: let scratch = Scratch::new("timed-replace");
IGNORED crates/pull/src/alerts_tests.rs:2164: raw_queue(&scratch, &open_alerts(size));
IGNORED crates/pull/src/alerts_tests.rs:2165: spoil(&scratch, u64::from(size) - 1);
IGNORED crates/pull/src/alerts_tests.rs:2166: spoil(&scratch, u64::from(size));
IGNORED crates/pull/src/alerts_tests.rs:2167: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2168: let done = replace_damaged_at(scratch.root(), NOW + i).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2169: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2170: assert_eq!((done.open, done.ordinal), (Some(size - 2), 1));
IGNORED crates/pull/src/alerts_tests.rs:2172: let (p50, p99, max) = spread(micros);
IGNORED crates/pull/src/alerts_tests.rs:2173: println!(
IGNORED crates/pull/src/alerts_tests.rs:2177: }
(F) crates/pull/src/alerts_tests.rs:237:9 False: let Put::Written {
(F) crates/pull/src/alerts_tests.rs:413:9 False: let Kept::Good(4, record) = &kept[3] else {
(F) crates/pull/src/alerts_tests.rs:504:13 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
(F) crates/pull/src/alerts_tests.rs:504:30 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
(F) crates/pull/src/alerts_tests.rs:504:59 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
(F) crates/pull/src/alerts_tests.rs:779:52 False: assert!(matches!(&seen[0], Kept::Good(1, r) if r.text == "invented k1"));
(F) crates/pull/src/alerts_tests.rs:780:52 False: assert!(matches!(&seen[2], Kept::Good(3, r) if r.open_after == 3));
(F) crates/pull/src/alerts_tests.rs:892:13 False: assert!(names.queue.is_file() && names.lock.is_file());
(F) crates/pull/src/alerts_tests.rs:892:38 False: assert!(names.queue.is_file() && names.lock.is_file());
(F) crates/pull/src/alerts_tests.rs:1138:9 False: let Err(AlertError::Io { kind, op, .. }) = &refused else {
(F) crates/pull/src/alerts_tests.rs:1259:17 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
(F) crates/pull/src/alerts_tests.rs:1259:32 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
(F) crates/pull/src/alerts_tests.rs:1259:58 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
(F, by hand) crates/pull/src/alerts_tests.rs:1526:47 False: seen.damaged == [3, 2] || replaced,
(F) crates/pull/src/alerts_tests.rs:1545:56 False: matches!(&first, Some(Kept::Good(1, r)) if r.kind == KIND_QUEUE_REPLACED),
(F) crates/pull/src/alerts_tests.rs:1858:13 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
(F) crates/pull/src/alerts_tests.rs:1858:28 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
(F) crates/pull/src/alerts_tests.rs:1858:54 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
(F) crates/pull/src/alerts_tests.rs:1865:13 False: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
(F) crates/pull/src/alerts_tests.rs:1865:40 False: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
IGNORED crates/pull/src/alerts_tests.rs:2122:17 True: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:17 False: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:45 True: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:45 False: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2135:19 True: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2135:19 False: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2160:25 True: let n: u64 = if size == 7 { 100 } else { 30 };
IGNORED crates/pull/src/alerts_tests.rs:2160:25 False: let n: u64 = if size == 7 { 100 } else { 30 };
(F) crates/pull/src/alerts_tests.rs:2193:13 False: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
(F) crates/pull/src/alerts_tests.rs:2193:38 False: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
(F) crates/pull/src/alerts_tests.rs:2226:9 False: before <= now && now <= after,
(F) crates/pull/src/alerts_tests.rs:2226:26 False: before <= now && now <= after,
(F) crates/pull/src/alerts_tests.rs:2475:62 False: matches!(&refused, Err(AlertError::Io { op, .. }) if *op == "keep the damaged queue"),
(F) crates/pull/src/base_cov_words_tests.rs:127: panic!("Dhan is an HTTP broker");
(F) crates/pull/src/base_cov_words_tests.rs:125:9 False: let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
(U) crates/pull/src/calendar.rs:679: return DayKind::Unmeasured;
(U) crates/pull/src/calendar.rs:742: DayKind::Unmeasured => return None,
(U) crates/pull/src/calendar.rs:2555: Err(SettlementRefusal::NoSessionWithin { stated })
(U) crates/pull/src/calendar.rs:2614: Ok(None)
(U, type invariant) crates/pull/src/calendar.rs:3582: return Err(outside);
(F) crates/pull/src/calendar.rs:4212: DayKind::Unmeasured => panic!("day {day} is inside the range and unmeasured"),
(F) crates/pull/src/calendar.rs:4334: panic!("{index} {day}")
(F) crates/pull/src/calendar.rs:4368: panic!("2021-02-24 traded");
(F) crates/pull/src/calendar.rs:4384: panic!("2025-10-21 traded");
(F) crates/pull/src/calendar.rs:4394: panic!("2024-03-02 traded");
(F) crates/pull/src/calendar.rs:4411: panic!("an ordinary weekday traded");
(F) crates/pull/src/calendar.rs:4585: panic!("it traded");
(U) crates/pull/src/calendar.rs:5173: return changed;
(F) crates/pull/src/calendar.rs:5371: panic!("{label} is open on the calendar of record")
(F) crates/pull/src/calendar.rs:5388: other => panic!("{label}: sourced_kind_of answered {other:?}"),
(F, by hand) crates/pull/src/calendar.rs:5444: _ => false,
(F) crates/pull/src/calendar.rs:5448: matches!(
(F) crates/pull/src/calendar.rs:6451: ExpiryDay::Unmeasured => panic!("day {day} is answered"),
IGNORED crates/pull/src/calendar.rs:6564: fn calendar_of_record_costs() {
IGNORED crates/pull/src/calendar.rs:6565: let mut days: Vec<i64> = SPECIAL_SESSIONS.iter().map(|s| s.day).collect();
IGNORED crates/pull/src/calendar.rs:6566: days.extend(HALTS.iter().map(|h| h.day));
IGNORED crates/pull/src/calendar.rs:6567: days.extend((SOURCED_FIRST_DAY..=SOURCED_LAST_DAY).step_by(97));
IGNORED crates/pull/src/calendar.rs:6568: days.push(SOURCED_LAST_DAY + 1);
IGNORED crates/pull/src/calendar.rs:6569: measure("exchange kind of a day", &days, exchange_kind_of);
IGNORED crates/pull/src/calendar.rs:6570: measure("phase at one minute", &days, |d| {
IGNORED crates/pull/src/calendar.rs:6571: phase_at(Venue::NseCash, d, 915, CasEligibility::Eligible)
IGNORED crates/pull/src/calendar.rs:6572: });
IGNORED crates/pull/src/calendar.rs:6573: measure("phases of a whole day", &days, |d| {
IGNORED crates/pull/src/calendar.rs:6574: phases_of(Venue::NseCash, d, CasEligibility::Eligible)
IGNORED crates/pull/src/calendar.rs:6575: });
IGNORED crates/pull/src/calendar.rs:6576: measure("settlement on or before", &days, settlement_on_or_before);
IGNORED crates/pull/src/calendar.rs:6577: measure("record of a day", &days, record_of);
IGNORED crates/pull/src/calendar.rs:6578: }
IGNORED crates/pull/src/calendar.rs:6583: fn measure<T>(name: &str, days: &[i64], op: impl Fn(i64) -> T) {
IGNORED crates/pull/src/calendar.rs:6586: let mut samples = Vec::new();
IGNORED crates/pull/src/calendar.rs:6587: for _ in 0..200 {
IGNORED crates/pull/src/calendar.rs:6588: for &day in days {
IGNORED crates/pull/src/calendar.rs:6589: let started = Instant::now();
IGNORED crates/pull/src/calendar.rs:6590: for _ in 0..16 {
IGNORED crates/pull/src/calendar.rs:6591: black_box(op(black_box(day)));
IGNORED crates/pull/src/calendar.rs:6592: }
IGNORED crates/pull/src/calendar.rs:6593: samples.push(started.elapsed().as_nanos() / 16);
IGNORED crates/pull/src/calendar.rs:6596: samples.sort_unstable();
IGNORED crates/pull/src/calendar.rs:6597: let pick = |permille: usize| {
IGNORED crates/pull/src/calendar.rs:6598: samples
IGNORED crates/pull/src/calendar.rs:6599: .get(samples.len() * permille / 1_000)
IGNORED crates/pull/src/calendar.rs:6600: .copied()
IGNORED crates/pull/src/calendar.rs:6601: .unwrap_or(u128::MAX)
IGNORED crates/pull/src/calendar.rs:6602: };
IGNORED crates/pull/src/calendar.rs:6603: let (p50, p99) = (pick(500), pick(990));
IGNORED crates/pull/src/calendar.rs:6604: eprintln!("{name}: p50 {p50} ns, p99 {p99} ns, n = {}", samples.len());
IGNORED crates/pull/src/calendar.rs:6605: assert!(p99 < 50_000, "{name}: p99 {p99} ns");
IGNORED crates/pull/src/calendar.rs:6606: }
(U) crates/pull/src/calendar.rs:419:20 False: if let Some(w) = self.windows.get(i) {
(U) crates/pull/src/calendar.rs:678:9 False: let Ok(offset) = usize::try_from(epoch_day - FIRST_DAY) else {
(U) crates/pull/src/calendar.rs:2128:16 False: if let Ok(minute) = u16::try_from(minute)
(U) crates/pull/src/calendar.rs:2129:16 False: && minute < MINUTES_PER_DAY
(U) crates/pull/src/calendar.rs:2130:20 False: && let Some(slot) = cuts.get_mut(used)
(U) crates/pull/src/calendar.rs:2435:42 False: DayKind::OpenLengthUnmeasured if listed(&SOURCED_SPECIAL, epoch_day) => {
(U) crates/pull/src/calendar.rs:3537:20 False: if let Some(count) = running.get_mut(slot) {
(U, type invariant) crates/pull/src/calendar.rs:3581:13 False: let (Some(before), Some(through)) = (self.sums.get(low), self.sums.get(high)) else {
(U) crates/pull/src/calendar.rs:3752:12 False: if newest && civil_year(notice.day) == i64::from(year) {
(F) crates/pull/src/calendar.rs:3869:17 False: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
(F) crates/pull/src/calendar.rs:3869:56 False: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
(F) crates/pull/src/calendar.rs:3879:17 False: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
(F) crates/pull/src/calendar.rs:3879:55 False: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
(F) crates/pull/src/calendar.rs:3890:65 False: matches!(record.kind, DayKind::Open(session) if session.bars() == 375),
(F) crates/pull/src/calendar.rs:3900:62 False: .all(|pair| matches!(pair, [older, newer] if older.day < newer.day)),
(F) crates/pull/src/calendar.rs:4039:17 False: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
(F) crates/pull/src/calendar.rs:4039:35 False: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
(F) crates/pull/src/calendar.rs:4333:21 False: let DayKind::Open(session) = kind_of(day) else {
(F) crates/pull/src/calendar.rs:4348:71 False: .all(|pair| matches!(pair, [first, second] if second - first == 60))
(F) crates/pull/src/calendar.rs:4367:13 False: let DayKind::Open(outage) = kind_of(SYSTEMS_OUTAGE_DAY) else {
(F) crates/pull/src/calendar.rs:4383:13 False: let DayKind::Open(muhurat) = kind_of(20_382) else {
(F) crates/pull/src/calendar.rs:4393:13 False: let DayKind::Open(dr) = kind_of(19_784) else {
(F) crates/pull/src/calendar.rs:4410:13 False: let DayKind::Open(full) = kind_of(20_668) else {
(F) crates/pull/src/calendar.rs:4584:13 False: let DayKind::Open(session) = cal.kind_of(20_382) else {
(U) crates/pull/src/calendar.rs:5169:13 False: let (Ok(start), Ok(end)) = (
(F) crates/pull/src/calendar.rs:5370:17 False: let DayKind::Open(session) = exchange_kind_of(day) else {
(F) crates/pull/src/calendar.rs:6467:21 False: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
(F) crates/pull/src/calendar.rs:6467:49 False: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
(U) crates/pull/src/census_store.rs:2363: return Err(format!(
(U) crates/pull/src/census_store.rs:2364: "{}: the split holds {keys} keys and {rows} rows, the census {} and {}; \
(U) crates/pull/src/census_store.rs:2365: nothing was written",
(U) crates/pull/src/census_store.rs:2366: legacy.display(),
(U) crates/pull/src/census_store.rs:2367: census.keys(),
(U) crates/pull/src/census_store.rs:2368: census.total_rows()
(U) crates/pull/src/census_store.rs:2369: ));
(U) crates/pull/src/census_store.rs:2879: return Err(format!(
(U) crates/pull/src/census_store.rs:2880: "{} could not be created for the requirement; nothing was written",
(U) crates/pull/src/census_store.rs:2881: at.display()
(U) crates/pull/src/census_store.rs:2882: ));
(U) crates/pull/src/census_store.rs:2362:8 True: if keys != census.keys() || rows != census.total_rows() {
(U) crates/pull/src/census_store.rs:2362:33 True: if keys != census.keys() || rows != census.total_rows() {
(U) crates/pull/src/census_store.rs:2878:9 False: let Some(mut file) = open_requires(&at, true)? else {
(U) crates/pull/src/census_store.rs:3016:16 False: if let Some(magic) = image.get_mut(header) {
(U) crates/pull/src/census_store.rs:3019:16 False: if let Some(version) = image.get_mut(slot + STUB_VERSION_AT..slot + STUB_VERSION_AT + 2) {
(U) crates/pull/src/census_store.rs:3022:16 False: if let Some(text) = image.get_mut(slot + 64..slot + 64 + note.len()) {
(F) crates/pull/src/census_store_kept_tests.rs:697: &mut |step: WholeStep| panic!("refused before any step, yet {step:?} ran"),
(F) crates/pull/src/census_store_kept_tests.rs:751: &mut |step: WholeStep| panic!("refused before any step, yet {step:?} ran"),
IGNORED crates/pull/src/census_store_kept_tests.rs:1099: fn the_kept_publish_is_flat_measured() {
IGNORED crates/pull/src/census_store_kept_tests.rs:1102: let base = std::env::var_os("R319_BENCH_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
IGNORED crates/pull/src/census_store_kept_tests.rs:1103: let mut p50s = Vec::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1104: for keys in [400_000_u32, 2_000_000, 4_000_000, 8_400_000] {
IGNORED crates/pull/src/census_store_kept_tests.rs:1105: let root = base.join(format!("brutex-r319-bench-{keys}-{}", std::process::id()));
IGNORED crates/pull/src/census_store_kept_tests.rs:1106: std::fs::create_dir_all(root.join("manifest")).expect("a bench root");
IGNORED crates/pull/src/census_store_kept_tests.rs:1107: let built = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1108: let capacity = u64::from(keys) + u64::from(N * FRESH) + 4_096;
IGNORED crates/pull/src/census_store_kept_tests.rs:1109: let mut census =
IGNORED crates/pull/src/census_store_kept_tests.rs:1110: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
IGNORED crates/pull/src/census_store_kept_tests.rs:1111: for n in 0..keys {
IGNORED crates/pull/src/census_store_kept_tests.rs:1112: census.record_held(row(n, 1)).expect("held");
IGNORED crates/pull/src/census_store_kept_tests.rs:1113: }
IGNORED crates/pull/src/census_store_kept_tests.rs:1114: let path = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
IGNORED crates/pull/src/census_store_kept_tests.rs:1115: std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
IGNORED crates/pull/src/census_store_kept_tests.rs:1116: std::fs::write(&path, census.image()).expect("epoch 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1117: drop(census);
IGNORED crates/pull/src/census_store_kept_tests.rs:1118: let built = built.elapsed().as_millis();
IGNORED crates/pull/src/census_store_kept_tests.rs:1119: let mut kept = Kept::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1120: let whole = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1121: let first = record_kept(
IGNORED crates/pull/src/census_store_kept_tests.rs:1122: &mut kept,
IGNORED crates/pull/src/census_store_kept_tests.rs:1123: &root,
IGNORED crates/pull/src/census_store_kept_tests.rs:1124: Vendor::Gdfl,
IGNORED crates/pull/src/census_store_kept_tests.rs:1125: &(0..FRESH).map(|i| row(keys + i, 1)).collect::<Vec<_>>(),
IGNORED crates/pull/src/census_store_kept_tests.rs:1127: .expect("day 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1128: let whole = whole.elapsed().as_micros();
IGNORED crates/pull/src/census_store_kept_tests.rs:1129: assert_eq!(first[0].read, CensusRead::Whole("first"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1130: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
IGNORED crates/pull/src/census_store_kept_tests.rs:1131: for d in 1..=N {
IGNORED crates/pull/src/census_store_kept_tests.rs:1132: let rows: Vec<Held> = (0..FRESH).map(|i| row(keys + d * FRESH + i, 1)).collect();
IGNORED crates/pull/src/census_store_kept_tests.rs:1133: let start = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1134: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
IGNORED crates/pull/src/census_store_kept_tests.rs:1135: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/census_store_kept_tests.rs:1136: assert_eq!(got[0].read, CensusRead::Kept, "{keys}: day {d}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1138: let (p50, p99, max) = percentiles(&mut micros);
IGNORED crates/pull/src/census_store_kept_tests.rs:1139: eprintln!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1143: std::fs::metadata(&path).map_or(0, |m| m.len())
IGNORED crates/pull/src/census_store_kept_tests.rs:1145: p50s.push(p50);
IGNORED crates/pull/src/census_store_kept_tests.rs:1146: drop(kept);
IGNORED crates/pull/src/census_store_kept_tests.rs:1147: let _ = std::fs::remove_dir_all(&root);
IGNORED crates/pull/src/census_store_kept_tests.rs:1149: let (small, large) = (p50s[0], p50s[3]);
IGNORED crates/pull/src/census_store_kept_tests.rs:1150: eprintln!("R3-19 ratio p50(8.4M) / p50(0.4M) = {large} / {small}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1151: assert!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1152: large <= small.saturating_mul(3).max(1),
IGNORED crates/pull/src/census_store_kept_tests.rs:1155: }
IGNORED crates/pull/src/census_store_kept_tests.rs:1166: fn the_kept_publish_on_a_copy_of_a_real_epoch_measured() {
IGNORED crates/pull/src/census_store_kept_tests.rs:1169: let root = PathBuf::from(std::env::var_os("R319_REAL_ROOT").expect("R319_REAL_ROOT"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1170: let invented = |n: u32| {
IGNORED crates/pull/src/census_store_kept_tests.rs:1171: let mut held = row(n, 1);
IGNORED crates/pull/src/census_store_kept_tests.rs:1172: held.entry.key.symbol = Symbol::new("INVENTEDR").expect("a symbol");
IGNORED crates/pull/src/census_store_kept_tests.rs:1173: held
IGNORED crates/pull/src/census_store_kept_tests.rs:1174: };
IGNORED crates/pull/src/census_store_kept_tests.rs:1175: let mut kept = Kept::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1176: let whole = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1177: let first = record_kept(
IGNORED crates/pull/src/census_store_kept_tests.rs:1178: &mut kept,
IGNORED crates/pull/src/census_store_kept_tests.rs:1179: &root,
IGNORED crates/pull/src/census_store_kept_tests.rs:1180: Vendor::Gdfl,
IGNORED crates/pull/src/census_store_kept_tests.rs:1181: &(0..FRESH).map(invented).collect::<Vec<_>>(),
IGNORED crates/pull/src/census_store_kept_tests.rs:1183: .expect("day 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1184: let whole = whole.elapsed().as_micros();
IGNORED crates/pull/src/census_store_kept_tests.rs:1185: assert_eq!(first[0].read, CensusRead::Whole("first"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1186: let entries = first[0].entries;
IGNORED crates/pull/src/census_store_kept_tests.rs:1187: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
IGNORED crates/pull/src/census_store_kept_tests.rs:1188: for d in 1..=N {
IGNORED crates/pull/src/census_store_kept_tests.rs:1189: let rows: Vec<Held> = (0..FRESH).map(|i| invented(d * FRESH + i)).collect();
IGNORED crates/pull/src/census_store_kept_tests.rs:1190: let start = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1191: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
IGNORED crates/pull/src/census_store_kept_tests.rs:1192: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/census_store_kept_tests.rs:1193: assert_eq!(got[0].read, CensusRead::Kept, "day {d}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1195: let (p50, p99, max) = percentiles(&mut micros);
IGNORED crates/pull/src/census_store_kept_tests.rs:1196: eprintln!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1201: }
(F) crates/pull/src/census_store_kept_tests.rs:777:9 False: why.contains(&vendor_dir(root, Vendor::Gdfl).display().to_string())
(F) crates/pull/src/census_store_kept_tests.rs:778:16 False: && why.contains("is not a directory"),
(F) crates/pull/src/census_store_kept_tests.rs:887:13 False: assert!(holds_sealed_stub_as_before(&stub) && holds_sealed_stub(&stub));
(F) crates/pull/src/census_store_kept_tests.rs:887:51 False: assert!(holds_sealed_stub_as_before(&stub) && holds_sealed_stub(&stub));
(F) crates/pull/src/census_store_kept_tests.rs:1033:17 False: assert!(meta.is_file() && meta.nlink() == 1);
(F) crates/pull/src/census_store_kept_tests.rs:1033:35 False: assert!(meta.is_file() && meta.nlink() == 1);
(F) crates/pull/src/census_store_link_tests.rs:205: target.display()
(F) crates/pull/src/census_store_link_tests.rs:286:17 False: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
(F) crates/pull/src/census_store_link_tests.rs:286:35 False: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
(F) crates/pull/src/census_store_link_tests.rs:336:13 False: left.is_file() && left.nlink() == 1,
(F) crates/pull/src/census_store_link_tests.rs:336:31 False: left.is_file() && left.nlink() == 1,
(F) crates/pull/src/census_store_link_tests.rs:668:13 False: assert!(meta.is_file() && meta.nlink() == 1);
(F) crates/pull/src/census_store_link_tests.rs:668:31 False: assert!(meta.is_file() && meta.nlink() == 1);
(F) crates/pull/src/census_store_ps2_tests.rs:486: panic!("no why: {line:?}");
(F) crates/pull/src/census_store_ps2_tests.rs:386:13 False: assert!(done[0].compacted.is_some() && done[0].degraded.is_none());
(F) crates/pull/src/census_store_ps2_tests.rs:386:44 False: assert!(done[0].compacted.is_some() && done[0].degraded.is_none());
(F) crates/pull/src/census_store_ps2_tests.rs:485:9 False: let Some(telemetry::OwnedValue::Str(why)) = line.field("why") else {
(F) crates/pull/src/census_store_ps2_tests.rs:977:13 False: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
(F) crates/pull/src/census_store_ps2_tests.rs:977:39 False: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
(F) crates/pull/src/census_store_ps2_tests.rs:977:68 False: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
(F) crates/pull/src/census_store_ps2_tests.rs:1020:13 False: assert!(run >= before && run < ahead, "{run}");
(F) crates/pull/src/census_store_ps2_tests.rs:1020:30 False: assert!(run >= before && run < ahead, "{run}");
(F) crates/pull/src/census_store_tests.rs:307: done.refused[0].1
(F) crates/pull/src/census_store_tests.rs:587: done.left_open[2]
(F) crates/pull/src/census_store_tests.rs:976: panic!("an empty batch flushes nothing")
(F) crates/pull/src/census_store_tests.rs:1313: seal_receipt(&at)
IGNORED crates/pull/src/census_store_tests.rs:1795: fn legacy_drift_cost_per_call_is_measured() {
IGNORED crates/pull/src/census_store_tests.rs:1796: let scratch = Scratch::new("DRIFT-TIMING");
IGNORED crates/pull/src/census_store_tests.rs:1797: assert_eq!(
IGNORED crates/pull/src/census_store_tests.rs:1798: record(scratch.root(), Vendor::Gdfl, &[row(0, 1)]).map(|c| c.len()),
IGNORED crates/pull/src/census_store_tests.rs:1801: migrate(scratch.root(), Vendor::Gdfl).expect("it migrates");
IGNORED crates/pull/src/census_store_tests.rs:1802: let real = std::env::var_os("BRUTEX_STORE_ROOT").map(PathBuf::from);
IGNORED crates/pull/src/census_store_tests.rs:1803: let mut roots: Vec<(&str, &Path)> = vec![("scratch", scratch.root())];
IGNORED crates/pull/src/census_store_tests.rs:1804: if let Some(real) = real.as_deref() {
IGNORED crates/pull/src/census_store_tests.rs:1805: roots.push(("store", real));
IGNORED crates/pull/src/census_store_tests.rs:1806: }
IGNORED crates/pull/src/census_store_tests.rs:1807: for (label, root) in roots {
IGNORED crates/pull/src/census_store_tests.rs:1808: let want = legacy_drift(root, Vendor::Gdfl);
IGNORED crates/pull/src/census_store_tests.rs:1809: let mut micros: Vec<u128> = Vec::with_capacity(400);
IGNORED crates/pull/src/census_store_tests.rs:1810: for _ in 0..400 {
IGNORED crates/pull/src/census_store_tests.rs:1811: let started = std::time::Instant::now();
IGNORED crates/pull/src/census_store_tests.rs:1812: let got = legacy_drift(root, Vendor::Gdfl);
IGNORED crates/pull/src/census_store_tests.rs:1813: micros.push(started.elapsed().as_micros());
IGNORED crates/pull/src/census_store_tests.rs:1814: assert_eq!(got, want, "{label}: the answer does not move between calls");
IGNORED crates/pull/src/census_store_tests.rs:1816: micros.sort_unstable();
IGNORED crates/pull/src/census_store_tests.rs:1817: let at = |q: usize| micros.get(399 * q / 100).copied().unwrap_or(u128::MAX);
IGNORED crates/pull/src/census_store_tests.rs:1818: let max = micros.last().copied().unwrap_or(u128::MAX);
IGNORED crates/pull/src/census_store_tests.rs:1819: eprintln!(
IGNORED crates/pull/src/census_store_tests.rs:1821: root.display(),
IGNORED crates/pull/src/census_store_tests.rs:1822: at(50),
IGNORED crates/pull/src/census_store_tests.rs:1823: at(99)
IGNORED crates/pull/src/census_store_tests.rs:1825: assert!(at(99) < 20_000, "{label}: p99 {}us", at(99));
IGNORED crates/pull/src/census_store_tests.rs:1827: }
(F) crates/pull/src/census_store_tests.rs:610:13 False: assert!(again.closed.is_empty() && again.run.is_none());
(F) crates/pull/src/census_store_tests.rs:610:40 False: assert!(again.closed.is_empty() && again.run.is_none());
(F) crates/pull/src/census_store_tests.rs:719:13 False: assert!(again.closed.is_empty() && again.run.is_none());
(F) crates/pull/src/census_store_tests.rs:719:40 False: assert!(again.closed.is_empty() && again.run.is_none());
(F) crates/pull/src/census_store_tests.rs:818:13 False: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
(F) crates/pull/src/census_store_tests.rs:818:41 False: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
(F) crates/pull/src/census_store_tests.rs:819:13 False: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
(F) crates/pull/src/census_store_tests.rs:819:42 False: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
(F) crates/pull/src/census_store_tests.rs:823:13 False: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
(F) crates/pull/src/census_store_tests.rs:823:40 False: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
(F) crates/pull/src/census_store_tests.rs:1188:13 False: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
(F) crates/pull/src/census_store_tests.rs:1188:49 False: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
(F) crates/pull/src/census_store_tests.rs:1327:17 False: assert!(!again.wrote && again.torn_receipt.is_none());
(F) crates/pull/src/census_store_tests.rs:1327:33 False: assert!(!again.wrote && again.torn_receipt.is_none());
(F) crates/pull/src/census_store_tests.rs:1336:13 False: assert!(seal.wrote && seal.torn_receipt.is_none());
(F) crates/pull/src/census_store_tests.rs:1336:27 False: assert!(seal.wrote && seal.torn_receipt.is_none());
(F) crates/pull/src/census_store_tests.rs:1350:9 False: why.contains("SEALED")
(F) crates/pull/src/census_store_tests.rs:1351:16 False: && why.contains("census-seal-v2")
(F) crates/pull/src/census_store_tests.rs:1352:16 False: && why.contains("nothing was written"),
(F) crates/pull/src/census_store_tests.rs:1362:9 False: why.contains("SEALED") && why.contains("not a regular file"),
(F) crates/pull/src/census_store_tests.rs:1362:35 False: why.contains("SEALED") && why.contains("not a regular file"),
(F) crates/pull/src/census_store_tests.rs:1497:9 False: why.contains("manifest/REQUIRES")
(F) crates/pull/src/census_store_tests.rs:1498:16 False: && why.contains("line 3")
(F) crates/pull/src/census_store_tests.rs:1499:16 False: && why.contains("\"census-v5\"")
(F) crates/pull/src/census_store_tests.rs:1500:16 False: && why.contains("this build does not have")
(F) crates/pull/src/census_store_tests.rs:1501:16 False: && why.contains("DB-B68"),
(F) crates/pull/src/census_store_tests.rs:1519:9 False: every.contains("line 4") && every.contains("\"bars-v4\""),
(F) crates/pull/src/census_store_tests.rs:1519:37 False: every.contains("line 4") && every.contains("\"bars-v4\""),
(F) crates/pull/src/census_store_tests.rs:1770:9 False: unread.contains("MIGRATED") && unread.contains("cannot be read"),
(F) crates/pull/src/census_store_tests.rs:1770:40 False: unread.contains("MIGRATED") && unread.contains("cannot be read"),
IGNORED crates/pull/src/census_store_tests.rs:1804:12 True: if let Some(real) = real.as_deref() {
IGNORED crates/pull/src/census_store_tests.rs:1804:12 False: if let Some(real) = real.as_deref() {
(F) crates/pull/src/chain.rs:516: panic!("Dhan publishes no contract name to discover");
(F) crates/pull/src/chain.rs:539: panic!("an archive has no vendor to publish a lookup");
(F) crates/pull/src/chain.rs:619: seen[0]
(F) crates/pull/src/chain.rs:720: chain.unreadable[0]
(F) crates/pull/src/chain.rs:826: from.asked.borrow()
(F) crates/pull/src/chain.rs:854: panic!("nothing answered, so the walk cannot have succeeded");
(F) crates/pull/src/chain.rs:507:13 False: let Err(why) = month(
(F) crates/pull/src/chain.rs:530:13 False: let Err(why) = month(
(F) crates/pull/src/chain.rs:716:13 False: chain.unreadable[0].contains("FINNIFTY")
(F) crates/pull/src/chain.rs:717:20 False: && chain.unreadable[0].contains("NIFTY was asked for"),
(F) crates/pull/src/chain.rs:845:13 False: let Err(why) = month(
(F) crates/pull/src/chain_ps2_tests.rs:86: panic!("{refused:?}");
(F) crates/pull/src/chain_ps2_tests.rs:85:9 False: let Err(ChainError::Transport { url, why }) = refused else {
(F) crates/pull/src/csv.rs:1900: columns.count()
(F, by hand) crates/pull/src/csv.rs:1524:50 False: assert!(!text.is_char_boundary(4) || !text.is_ascii());
(F) crates/pull/src/csv.rs:1947:66 False: Err(CsvError::HeaderMismatch { ref got, .. }) if *got == swapped
(F) crates/pull/src/csv.rs:1956:62 False: Err(CsvError::HeaderMismatch { ref got, .. }) if got.is_empty()
(U) crates/pull/src/daycheck.rs:282:28 False: && let Some(bar) = rest.first()
(F) crates/pull/src/daycheck.rs:509:17 False: assert!(report.consistent() && !report.clean());
(F) crates/pull/src/daycheck.rs:509:40 False: assert!(report.consistent() && !report.clean());
(U) crates/pull/src/disk_projection.rs:261:20 False: if let Some(list) = per_class.get_mut(class.slot()) {
IGNORED crates/pull/src/disk_projection_tests.rs:391: fn real_delivery_sizes_project_through_the_module_and_meet_the_floor() {
IGNORED crates/pull/src/disk_projection_tests.rs:392: let read = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
IGNORED crates/pull/src/disk_projection_tests.rs:393: let unit_of = |text: &str| {
IGNORED crates/pull/src/disk_projection_tests.rs:394: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:395: let (name, class) = rest.split_once('@').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:396: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:397: OptionClass::Index
IGNORED crates/pull/src/disk_projection_tests.rs:399: OptionClass::Stock
IGNORED crates/pull/src/disk_projection_tests.rs:401: Unit::Underlying {
IGNORED crates/pull/src/disk_projection_tests.rs:402: name: name.to_owned(),
IGNORED crates/pull/src/disk_projection_tests.rs:403: class,
IGNORED crates/pull/src/disk_projection_tests.rs:404: }
IGNORED crates/pull/src/disk_projection_tests.rs:406: let mut words = text.split('/');
IGNORED crates/pull/src/disk_projection_tests.rs:407: let (feed, k, rung) = (
IGNORED crates/pull/src/disk_projection_tests.rs:408: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:409: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:410: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:411: );
IGNORED crates/pull/src/disk_projection_tests.rs:412: Unit::Kind(KindKey::new(feed, k, rung))
IGNORED crates/pull/src/disk_projection_tests.rs:414: };
IGNORED crates/pull/src/disk_projection_tests.rs:415: let items = |name: &str| -> Vec<(Unit, String)> {
IGNORED crates/pull/src/disk_projection_tests.rs:416: read(name)
IGNORED crates/pull/src/disk_projection_tests.rs:417: .split(';')
IGNORED crates/pull/src/disk_projection_tests.rs:418: .filter(|s| !s.is_empty())
IGNORED crates/pull/src/disk_projection_tests.rs:419: .map(|item| {
IGNORED crates/pull/src/disk_projection_tests.rs:420: let (key, value) = item.split_once('=').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:421: (unit_of(key), value.to_owned())
IGNORED crates/pull/src/disk_projection_tests.rs:422: })
IGNORED crates/pull/src/disk_projection_tests.rs:423: .collect()
IGNORED crates/pull/src/disk_projection_tests.rs:424: };
IGNORED crates/pull/src/disk_projection_tests.rs:425: let mut totals = Totals::new();
IGNORED crates/pull/src/disk_projection_tests.rs:426: for (unit, value) in items("BRUTEX_REAL_SAMPLE") {
IGNORED crates/pull/src/disk_projection_tests.rs:427: let (bytes, days) = value.split_once(':').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:428: let (bytes, days) = (bytes.parse().unwrap(), days.parse().unwrap());
IGNORED crates/pull/src/disk_projection_tests.rs:429: match unit {
IGNORED crates/pull/src/disk_projection_tests.rs:430: Unit::Kind(key) => totals.add_kind(key, bytes, days).unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:431: Unit::Underlying { name, class } => {
IGNORED crates/pull/src/disk_projection_tests.rs:432: totals.add_underlying(&name, class, bytes, days).unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:433: }
IGNORED crates/pull/src/disk_projection_tests.rs:436: let sealed = totals.seal();
IGNORED crates/pull/src/disk_projection_tests.rs:437: let plan: Vec<PlanItem> = items("BRUTEX_REAL_PLAN")
IGNORED crates/pull/src/disk_projection_tests.rs:438: .into_iter()
IGNORED crates/pull/src/disk_projection_tests.rs:439: .map(|(unit, days)| PlanItem {
IGNORED crates/pull/src/disk_projection_tests.rs:440: unit,
IGNORED crates/pull/src/disk_projection_tests.rs:441: days: days.parse().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:443: })
IGNORED crates/pull/src/disk_projection_tests.rs:444: .collect();
IGNORED crates/pull/src/disk_projection_tests.rs:445: let projection = project(&sealed, &plan);
IGNORED crates/pull/src/disk_projection_tests.rs:446: for line in &projection.lines {
IGNORED crates/pull/src/disk_projection_tests.rs:447: println!(
IGNORED crates/pull/src/disk_projection_tests.rs:451: assert_eq!(line.bytes, line.per_day * line.days, "days x bytes a day");
IGNORED crates/pull/src/disk_projection_tests.rs:452: if let Basis::Extrapolated { class, .. } = line.basis {
IGNORED crates/pull/src/disk_projection_tests.rs:453: assert_eq!(Some(line.per_day), sealed.median_of(class).map(|(m, _)| m));
IGNORED crates/pull/src/disk_projection_tests.rs:454: assert!(
IGNORED crates/pull/src/disk_projection_tests.rs:455: projection.extrapolated.contains(&line.unit.to_string()),
IGNORED crates/pull/src/disk_projection_tests.rs:458: }
IGNORED crates/pull/src/disk_projection_tests.rs:460: println!("{}", projection.summary());
IGNORED crates/pull/src/disk_projection_tests.rs:461: assert!(
IGNORED crates/pull/src/disk_projection_tests.rs:462: projection.unmeasured.is_empty(),
IGNORED crates/pull/src/disk_projection_tests.rs:465: let space = parse_df(&read("BRUTEX_REAL_DF")).unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:466: let verdict = floor_check(space, projection.bytes, DEFAULT_FLOOR);
IGNORED crates/pull/src/disk_projection_tests.rs:467: println!(
IGNORED crates/pull/src/disk_projection_tests.rs:471: DEFAULT_FLOOR.bytes_of(space.total)
IGNORED crates/pull/src/disk_projection_tests.rs:473: let fits = space.free
IGNORED crates/pull/src/disk_projection_tests.rs:474: >= projection
IGNORED crates/pull/src/disk_projection_tests.rs:475: .bytes
IGNORED crates/pull/src/disk_projection_tests.rs:476: .saturating_add(DEFAULT_FLOOR.bytes_of(space.total));
IGNORED crates/pull/src/disk_projection_tests.rs:477: assert_eq!(
IGNORED crates/pull/src/disk_projection_tests.rs:478: verdict.is_ok(),
IGNORED crates/pull/src/disk_projection_tests.rs:482: }
IGNORED crates/pull/src/disk_projection_tests.rs:394:16 True: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:394:16 False: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:396:28 True: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:396:28 False: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:452:16 True: if let Basis::Extrapolated { class, .. } = line.basis {
IGNORED crates/pull/src/disk_projection_tests.rs:452:16 False: if let Basis::Extrapolated { class, .. } = line.basis {
(F) crates/pull/src/disk_projection_tests.rs:546:83 False: matches!(refused, FloorRefusal::NeedOverflows { projected: p, .. } if p == projected),
(F) crates/pull/src/emit_sites.rs:1554: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
(F) crates/pull/src/emit_sites.rs:1599: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
(F) crates/pull/src/emit_sites.rs:1628: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
(F) crates/pull/src/emit_sites.rs:1657: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
(F) crates/pull/src/emit_sites.rs:1725: matches!(
(F) crates/pull/src/emit_sites.rs:2026: panic!("this feed authenticates, so its transport is HTTP")
(F) crates/pull/src/emit_sites.rs:2124: landed
(F) crates/pull/src/emit_sites.rs:2125: .iter()
(F) crates/pull/src/emit_sites.rs:2126: .map(|record| (&record.message, record.field(key)))
(F) crates/pull/src/emit_sites.rs:2127: .collect::<Vec<_>>()
(F) crates/pull/src/emit_sites.rs:2176: said.iter()
(F) crates/pull/src/emit_sites.rs:2177: .map(|record| &record.message)
(F) crates/pull/src/emit_sites.rs:2178: .collect::<Vec<_>>()
(F) crates/pull/src/fno.rs:406: panic!("Groww is an HTTP broker");
(F) crates/pull/src/fno.rs:533: panic!("Dhan is an HTTP broker");
(U) crates/pull/src/fno.rs:1339: return false;
(U) crates/pull/src/fno.rs:1345: return false;
(F) crates/pull/src/fno.rs:405:13 False: let Transport::Http(spec) = Feed::Groww.descriptor().transport else {
(F) crates/pull/src/fno.rs:532:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
(F) crates/pull/src/fno.rs:957:17 False: assert!(found.contract.is_option() && !found.contract.is_future());
(F) crates/pull/src/fno.rs:957:47 False: assert!(found.contract.is_option() && !found.contract.is_future());
(F) crates/pull/src/fno.rs:989:13 False: floor.contains("2019") && floor.contains("2020"),
(F) crates/pull/src/fno.rs:989:39 False: floor.contains("2019") && floor.contains("2020"),
(U) crates/pull/src/fno.rs:1331:9 False: let Some(rest) = rest else { return false };
(U) crates/pull/src/fno.rs:1338:9 False: let Ok(month) = u8::try_from(month + 1) else {
(U) crates/pull/src/fno.rs:1344:9 False: let Some(year) = 2000u16.checked_add(year) else {
(F) crates/pull/src/fno_ps2_tests.rs:18: panic!("Zerodha is an HTTP broker");
(F) crates/pull/src/fno_ps2_tests.rs:17:9 False: let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) crates/pull/src/fnowork_futures_tests.rs:56: panic!("Zerodha is an HTTP feed");
(F) crates/pull/src/fnowork_futures_tests.rs:166: _ => unreachable!("february_reply asked for {url}"),
(F) crates/pull/src/fnowork_futures_tests.rs:292: panic!("Groww is an HTTP feed");
(F) crates/pull/src/fnowork_futures_tests.rs:657: landed.balance_line()
IGNORED crates/pull/src/fnowork_futures_tests.rs:990: fn a_real_size_plan_does_not_grow_with_the_snapshots_held() {
IGNORED crates/pull/src/fnowork_futures_tests.rs:991: let path = std::env::var_os("BRUTEX_KITE_DUMP").expect("BRUTEX_KITE_DUMP");
IGNORED crates/pull/src/fnowork_futures_tests.rs:992: let body = std::fs::read(path).expect("the dump reads");
IGNORED crates/pull/src/fnowork_futures_tests.rs:993: let root = scratch("real-size");
IGNORED crates/pull/src/fnowork_futures_tests.rs:996: let first = day(2026, 7, 1);
IGNORED crates/pull/src/fnowork_futures_tests.rs:997: let asked = window(day(2026, 10, 1), day(2026, 12, 31));
IGNORED crates/pull/src/fnowork_futures_tests.rs:998: let mut held = 0_u32;
IGNORED crates/pull/src/fnowork_futures_tests.rs:999: let mut p50s = Vec::new();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1000: for count in [1_u32, 10, 100] {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001: while held < count {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1002: let on = Day::from_days(first.days_from_epoch() + held).expect("a real day");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1003: let started = std::time::Instant::now();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1004: master_snapshots::snapshot(&root, Master::Kite, on, URL, &body).expect("a snapshot");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1005: println!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1006: "MEASURED snapshot {} written, its futures taken in: {} ms",
IGNORED crates/pull/src/fnowork_futures_tests.rs:1007: held + 1,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1008: started.elapsed().as_millis()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1009: );
IGNORED crates/pull/src/fnowork_futures_tests.rs:1010: held += 1;
IGNORED crates/pull/src/fnowork_futures_tests.rs:1011: }
IGNORED crates/pull/src/fnowork_futures_tests.rs:1012: let mut ns: Vec<u128> = (0..50)
IGNORED crates/pull/src/fnowork_futures_tests.rs:1013: .map(|_| {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1014: let started = std::time::Instant::now();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1015: let book = Book::open(&root).expect("a book");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1016: let plan = futures_plan(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1017: &book,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1018: route(),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1019: "NIFTY",
IGNORED crates/pull/src/fnowork_futures_tests.rs:1020: asked,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1021: day(2026, 10, 22),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1022: day(2026, 12, 31),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1024: .expect("a plan");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1025: assert!(plan.refused.is_empty(), "{:?}", plan.refused);
IGNORED crates/pull/src/fnowork_futures_tests.rs:1026: assert!(!plan.asks.is_empty(), "NIFTY futures are asked");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1027: started.elapsed().as_nanos()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1028: })
IGNORED crates/pull/src/fnowork_futures_tests.rs:1029: .collect();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1030: ns.sort_unstable();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1031: println!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1033: ns[24] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1034: ns[48] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1035: ns[49] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1036: ns.len()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1038: p50s.push(ns[24]);
IGNORED crates/pull/src/fnowork_futures_tests.rs:1040: assert!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1041: p50s[2] <= p50s[0] * 3,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1043: p50s[2],
IGNORED crates/pull/src/fnowork_futures_tests.rs:1044: p50s[0]
IGNORED crates/pull/src/fnowork_futures_tests.rs:1046: }
(F) crates/pull/src/fnowork_futures_tests.rs:1456: alike.series[0]
(F) crates/pull/src/fnowork_futures_tests.rs:1470: unlike.balance_line()
(F) crates/pull/src/fnowork_futures_tests.rs:1475: unlike.series[0]
(F) crates/pull/src/fnowork_futures_tests.rs:54:9 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
(F) crates/pull/src/fnowork_futures_tests.rs:290:9 False: let crate::vendor::Transport::Http(groww) = crate::vendor::Feed::Groww.descriptor().transport
(F) crates/pull/src/fnowork_futures_tests.rs:665:9 False: stopped.contains("0 bars written, 0 offered from 0 rows")
(F) crates/pull/src/fnowork_futures_tests.rs:666:16 False: && stopped.contains("; then stopped: vendor_refused"),
(F) crates/pull/src/fnowork_futures_tests.rs:844:9 False: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
(F) crates/pull/src/fnowork_futures_tests.rs:844:49 False: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
(F) crates/pull/src/fnowork_futures_tests.rs:885:27 False: .filter(|why| why.starts_with("contract_duplicated")
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001:15 True: while held < count {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001:15 False: while held < count {
(F) crates/pull/src/fnowork_futures_tests.rs:1136:9 False: said.starts_with("generation g1 of the futures index did not match its commit log")
(F) crates/pull/src/fnowork_futures_tests.rs:1137:16 False: && said.contains(why)
(F) crates/pull/src/fnowork_futures_tests.rs:1138:16 False: && said.contains("generation g2 was started"),
(F) crates/pull/src/fnowork_futures_tests.rs:1473:9 False: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
(F) crates/pull/src/fnowork_futures_tests.rs:1473:58 False: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
(F) crates/pull/src/fnowork_futures_tests.rs:1502:9 False: why.starts_with("snapshot_from_the_future: ")
(F) crates/pull/src/fnowork_futures_tests.rs:1503:16 False: && why.contains("2030-01-05")
(F) crates/pull/src/fnowork_futures_tests.rs:1504:16 False: && why.contains("after today 2026-01-07"),
(U) crates/pull/src/fold.rs:345: return Err(FoldError::OutOfOrder {
(U) crates/pull/src/fold.rs:346: at: i,
(U) crates/pull/src/fold.rs:347: previous: start,
(U) crates/pull/src/fold.rs:348: found: snap.ts_micros,
(U) crates/pull/src/fold.rs:349: });
(U) crates/pull/src/fold.rs:1667: return Err(format!(
(U) crates/pull/src/fold.rs:1668: "{venue}: session hours cannot attest the fixed stored grid"
(U) crates/pull/src/fold.rs:1669: ));
(F) crates/pull/src/fold.rs:1925: diagnostics.len(),
(F) crates/pull/src/fold.rs:1926: diagnostics.iter().take(7).collect::<Vec<_>>()
(F) crates/pull/src/fold.rs:2270: derived.diagnostics[0]
(F) crates/pull/src/fold.rs:2322: lines[0]
(U, type invariant) crates/pull/src/fold.rs:272:38 False: (Some(first), Some(last)) if width > 0 => {
(U) crates/pull/src/fold.rs:343:17 False: let Some(bar) = out.last_mut() else {
(U) crates/pull/src/fold.rs:1329:20 False: if let Some(note) = day_note_at.and_then(|at| out.notes.get_mut(at)) {
(U) crates/pull/src/fold.rs:1336:33 False: let complete = valid && expected > 0 && i128::from(count) == expected && traded == count;
(U) crates/pull/src/fold.rs:1340:16 False: && expected > 0
(U) crates/pull/src/fold.rs:1343:52 False: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
(U) crates/pull/src/fold.rs:1343:68 False: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
(U) crates/pull/src/fold.rs:1664:8 True: if hours.kind() != crate::vendor::SessionKind::Continuous
(U) crates/pull/src/fold.rs:1665:12 True: || hours.open_minute() != u32::from(crate::calendar::OPEN_MINUTE)
(F) crates/pull/src/gdfl_backfill_tests.rs:321: lines[intent]
(F) crates/pull/src/gdfl_backfill_tests.rs:339: lines[repair]
(F) crates/pull/src/gdfl_backfill_tests.rs:801: nifty[0].why
(F) crates/pull/src/gdfl_backfill_tests.rs:1268: refused.failures[0]
(F) crates/pull/src/gdfl_backfill_tests.rs:1281: refused.failures[0]
(F) crates/pull/src/gdfl_backfill_tests.rs:401:35 False: intents.iter().any(|line| line
(F) crates/pull/src/gdfl_backfill_tests.rs:522:13 False: failure.why.contains("the versioned repair refused it too")
(F) crates/pull/src/gdfl_backfill_tests.rs:523:20 False: && failure.why.contains("bar-revisions-v1"),
(F) crates/pull/src/gdfl_backfill_tests.rs:643:9 False: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
(F) crates/pull/src/gdfl_backfill_tests.rs:643:57 False: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
(U, type invariant) crates/pull/src/gdfl_cm.rs:1804: self.skip();
(U, type invariant) crates/pull/src/gdfl_cm.rs:1805: return;
(U) crates/pull/src/gdfl_cm.rs:2040: Ok(CasEligibility::Unknown) => Auction::Unknown(format!(
(U) crates/pull/src/gdfl_cm.rs:2041: "UNVERIFIED cash eligibility of {symbol}: the master holds no flag"
(U) crates/pull/src/gdfl_cm.rs:2042: )),
(U) crates/pull/src/gdfl_cm.rs:3510: return false;
(U, type invariant) crates/pull/src/gdfl_cm.rs:1386:41 False: Some((share, _)) if self.series => share,
(U, type invariant) crates/pull/src/gdfl_cm.rs:1803:13 False: let Some(cell) = cell else {
(U, type invariant) crates/pull/src/gdfl_cm.rs:2884:58 False: if matches!(self.band, MatchBand::One(_)) && self.booked > 0 {
(U) crates/pull/src/gdfl_cm.rs:2930:20 False: if let Some(row) = next {
(U) crates/pull/src/gdfl_cm.rs:3509:9 False: let Ok(window) = crate::session::Window::new(day, day) else {
(U, type invariant) crates/pull/src/gdfl_dayzips.rs:380: (None, None) => Ok(None),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:540: fn day_table_sizing_timing() {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:541: fn stats(mut took: Vec<std::time::Duration>) -> String {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:542: took.sort_unstable();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:543: let n = took.len();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:544: format!(
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:546: took[n / 2],
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:547: took[n * 99 / 100],
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:548: took[n - 1]
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:550: }
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:551: let alloc = |sized: bool| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:552: (0..2_000)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:553: .map(|_| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:554: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555: let table: HashMap<(LooseKind, Day), Copies> = if sized {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:556: HashMap::with_capacity(DAYZIPS_KEYS_MAX)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:558: HashMap::default()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:560: let took = began.elapsed();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:561: assert!(table.capacity() >= usize::from(sized) * DAYZIPS_KEYS_MAX);
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:562: took
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:563: })
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:564: .collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:565: };
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:566: let empty = fresh("timing.empty");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:567: let edges = fresh("timing.edges");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:568: for kind in LooseKind::ALL {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:569: for day in [
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:570: Day::new(2018, 1, 1).unwrap(),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:571: Day::new(2026, 12, 31).unwrap(),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:572: ] {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:573: put(&edges, &kind.path_of(day), b"");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:574: }
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:576: let walk = |root: &Path| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:577: (0..200)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:578: .map(|_| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:579: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:580: let zips = DayZips::open(root).unwrap();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:581: let took = began.elapsed();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:582: assert!(zips.days.capacity() >= DAYZIPS_KEYS_MAX);
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:583: took
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:584: })
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:585: .collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:586: };
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:587: println!(
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:589: stats(alloc(false)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:590: stats(alloc(true)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:591: stats(walk(&empty)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:592: stats(walk(&edges))
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:594: }
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555:67 True: let table: HashMap<(LooseKind, Day), Copies> = if sized {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555:67 False: let table: HashMap<(LooseKind, Day), Copies> = if sized {
(F) crates/pull/src/gdfl_format_change_tests.rs:373: other => panic!("{other:?}"),
(F) crates/pull/src/gdfl_format_change_tests.rs:129:13 False: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
(F) crates/pull/src/gdfl_format_change_tests.rs:129:52 False: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
(F) crates/pull/src/gdfl_format_change_tests.rs:221:22 False: .any(|f| f.instrument.contains("2024-04-09") && f.why.contains(&shape)),
(F) crates/pull/src/gdfl_format_change_tests.rs:274:13 False: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
(F) crates/pull/src/gdfl_format_change_tests.rs:274:41 False: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
(F) crates/pull/src/gdfl_format_change_tests.rs:283:22 False: .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
IGNORED crates/pull/src/gdfl_ia3_tests.rs:389: fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:390: samples.sort_unstable();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:391: let at = |q: usize| samples.get((samples.len() * q / 100).min(samples.len().saturating_sub(1)));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:392: (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:393: at(50).copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:394: at(99).copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:395: samples.last().copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:396: )
IGNORED crates/pull/src/gdfl_ia3_tests.rs:397: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:407: fn ia3_lookup_costs() {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:408: let days: Vec<Day> = (0..2_600)
IGNORED crates/pull/src/gdfl_ia3_tests.rs:409: .filter_map(|n| Day::from_days(Day::new(2018, 9, 3).unwrap().days_from_epoch() + n).ok())
IGNORED crates/pull/src/gdfl_ia3_tests.rs:410: .collect();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:412: tickstore_costs(Path::new(&root), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:413: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:415: archive_costs(Path::new(&path), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:416: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:418: dayzips_costs(Path::new(&root), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:419: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:420: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:424: fn tickstore_costs(root: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:425: let store = TickStore::new(root);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:426: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:427: let table = store.table();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:428: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:430: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:435: table.stray_named.iter().take(4).collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:437: let mut probe: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:438: for day in days.iter().cycle().take(20_000) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:439: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:440: let _ = std::hint::black_box(table.misfiled(0, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:441: probe.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:442: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:443: let n = probe.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:444: let (p50, p99, max) = quantiles(&mut probe);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:445: println!("tickstore misfiled probe: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:446: let mut listing: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:447: let mut sums: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:448: for day in days.iter().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:449: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:450: let got = store.day(CmKind::Indices, *day);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:451: listing.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:453: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:454: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:455: sums.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:456: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:458: let n = listing.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:459: let (p50, p99, max) = quantiles(&mut listing);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:460: println!("tickstore day listing (open + index): n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:461: let n = sums.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:462: let (p50, p99, max) = quantiles(&mut sums);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:463: println!("indices listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:464: let nfo = crate::gdfl_nfo::NfoTickStore::new(root);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:465: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:466: let table = nfo.table();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:467: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:469: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:474: let mut sums: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:475: for day in days.iter().rev().take(60) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:477: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:478: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:479: sums.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:480: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:482: let n = sums.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:483: let (p50, p99, max) = quantiles(&mut sums);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:484: println!("options listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:485: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:488: fn archive_costs(path: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:489: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:490: let archive = Archive::open(path).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:491: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:493: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:494: archive.census()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:496: let mut lookups: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:497: for day in days.iter().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:498: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:499: let _ = std::hint::black_box(archive.day(CmKind::Indices, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:500: lookups.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:501: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:502: let n = lookups.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:503: let (p50, p99, max) = quantiles(&mut lookups);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:504: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:507: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:510: fn dayzips_costs(root: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:511: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:512: let loose = crate::gdfl_dayzips::DayZips::open(root).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:513: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:515: cold.elapsed().as_micros(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:516: loose.census()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:518: let mut lookups: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:519: for day in days.iter().rev().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:520: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:521: let _ = std::hint::black_box(CmSource::day(&loose, CmKind::Indices, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:522: lookups.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:523: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:524: let n = lookups.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:525: let (p50, p99, max) = quantiles(&mut lookups);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:526: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:529: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:545: fn one_day<'a>(kind: ImportKind, d: Day, only: &'a [String], at: &'a Path) -> Run<'a> {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:546: Run {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:547: kind,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:548: from: d,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:549: to: d,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:550: only,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:551: store_root: at,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:552: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:553: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:556: fn listing_of(line: &str) -> Option<&str> {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:557: line.split(' ')
IGNORED crates/pull/src/gdfl_ia3_tests.rs:558: .find_map(|field| field.strip_prefix("listing="))
IGNORED crates/pull/src/gdfl_ia3_tests.rs:559: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:570: fn real_days_close_with_the_same_listing_from_two_sources() {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:571: let var = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:573: var("BRUTEX_GDFL_TICKSTORE"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:574: var("BRUTEX_GDFL_CM_ARCHIVE"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:575: var("BRUTEX_GDFL_NFO_ARCHIVES"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:577: return;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:579: let archive = Archive::open(&cm).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:580: let store = TickStore::new(&ts);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:581: let root = scratch("ia3-real");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:582: let cm_case = |tag: &str, kind: ImportKind, d: Day, only: &[String]| {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:583: let a = root.join(format!("{tag}-zip"));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:584: let b = root.join(format!("{tag}-ts"));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:585: let zip_report = run_cm(&archive, &one_day(kind, d, only, &a)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:586: let ts_report = run_cm(&store, &one_day(kind, d, only, &b)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:587: let (zip_line, store_line) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:588: close_of(&a, kind.as_str(), d),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:589: close_of(&b, kind.as_str(), d),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:590: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:591: println!("{tag} zip: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:592: println!("{tag} tickstore: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:593: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:594: assert_eq!(listing_of(&zip_line), listing_of(&store_line), "{tag}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:595: assert_eq!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:596: (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:597: zip_report.files,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:598: zip_report.seconds,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:599: zip_report.files_skipped
IGNORED crates/pull/src/gdfl_ia3_tests.rs:600: ),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:601: (ts_report.files, ts_report.seconds, ts_report.files_skipped),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:604: };
IGNORED crates/pull/src/gdfl_ia3_tests.rs:605: let nifty = ["NIFTY".to_owned()];
IGNORED crates/pull/src/gdfl_ia3_tests.rs:606: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:607: "nifty-spot",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:608: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:609: Day::new(2024, 4, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:610: &nifty,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:611: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:612: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:613: "muhurat",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:614: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:615: Day::new(2024, 11, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:616: &nifty,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:617: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:618: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:619: "banknifty",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:620: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:621: Day::new(2020, 3, 23).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:622: &["BANKNIFTY".to_owned()],
IGNORED crates/pull/src/gdfl_ia3_tests.rs:623: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:624: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:625: "reliance",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:626: ImportKind::Stocks,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:627: Day::new(2024, 4, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:628: &["RELIANCE".to_owned()],
IGNORED crates/pull/src/gdfl_ia3_tests.rs:629: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:630: let d = Day::new(2024, 4, 4).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:631: let a = root.join("nifty-options-zip");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:632: let b = root.join("nifty-options-ts");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:633: let options = ImportKind::Options;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:634: let zips = run_nfo(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:635: &crate::gdfl_nfo::NfoZips::new(&nfo),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:636: &one_day(options, d, &nifty, &a),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:638: .unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:639: let tick = run_nfo(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:640: &crate::gdfl_nfo::NfoTickStore::new(&ts),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:641: &one_day(options, d, &nifty, &b),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:643: .unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:644: let (zip_line, store_line) = (close_of(&a, "options", d), close_of(&b, "options", d));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:645: println!("nifty-options zip: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:646: println!("nifty-options tickstore: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:647: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:648: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:649: assert_eq!((zips.files, zips.seconds), (tick.files, tick.seconds));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:651: let loose = crate::gdfl_dayzips::DayZips::open(&loose).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:652: let d = Day::new(2026, 9, 25).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:653: let a = root.join("loose-nifty");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:654: let b = root.join("loose-nifty-ts");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:655: let indices = ImportKind::Indices;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:656: let from_loose = run_cm(&loose, &one_day(indices, d, &nifty, &a)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:657: let from_ts = run_cm(&store, &one_day(indices, d, &nifty, &b)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:658: let (zip_line, store_line) = (close_of(&a, "indices", d), close_of(&b, "indices", d));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:659: println!("loose 2026-09-25: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:660: println!("tickstore 2026-09-25: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:661: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:662: assert_eq!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:663: (from_loose.files, from_loose.seconds),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:664: (from_ts.files, from_ts.seconds)
IGNORED crates/pull/src/gdfl_ia3_tests.rs:666: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:667: println!("scratch stores under {}", root.display());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:668: }
(F) crates/pull/src/gdfl_ia3_tests.rs:174:9 False: !line.contains("listing=") && !line.contains("skipped="),
(F) crates/pull/src/gdfl_ia3_tests.rs:174:39 False: !line.contains("listing=") && !line.contains("skipped="),
(F) crates/pull/src/gdfl_ia3_tests.rs:322:9 False: line.starts_with("refused indices 2024-04-01 * ")
(F) crates/pull/src/gdfl_ia3_tests.rs:323:16 False: && line.contains("stage=vendor reason=day_file_misfiled"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411:12 True: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411:12 False: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414:12 True: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414:12 False: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417:12 True: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417:12 False: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452:16 True: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452:16 False: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476:16 True: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476:16 False: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572:9 True: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572:9 False: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650:12 True: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650:12 False: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
(F) crates/pull/src/gdfl_ia3_tests.rs:867:13 False: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
(F) crates/pull/src/gdfl_ia3_tests.rs:867:53 False: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
(U) crates/pull/src/gdfl_import_filing.rs:116: Ok(store::provenance::Recorded::NothingAppended) => Ok(false),
(U) crates/pull/src/gdfl_import_filing.rs:356:32 False: if let Some(slot) = slots.get_mut(index) {
(U) crates/pull/src/gdfl_import_filing.rs:696:20 False: && let Some(held) = done.pending.as_ref()
(U) crates/pull/src/gdfl_import_filing.rs:750:31 False: Some(kind) if at + 1 == last => {
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:119: &|_: &i32| -> i32 { unreachable!("a stopped worker takes no job") },
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:201: &|_: &i32| -> i32 { unreachable!("no worker started, so no job runs") },
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:381: panic!("one filed job");
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:389: filed
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:390: .failures
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:391: .iter()
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:392: .map(|s| &s.failure.why)
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:393: .collect::<Vec<_>>()
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:515: panic!("the first read, the second not taken");
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:630: panic!("{:?}", filed.failures);
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:670: panic!("{:?}", filed.failures);
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:183:47 False: ImportRefusal::Journal { why } if why == "the day's begin line was not written"
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:380:13 False: let [Ok((filed, None))] = done.as_slice() else {
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:514:13 False: let [JobRead::Read(None), JobRead::Refused(second)] = answered.as_slice() else {
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:629:13 False: let [one] = filed.failures.as_slice() else {
(F) crates/pull/src/gdfl_import_filing_ps2_tests.rs:669:13 False: let [one] = filed.failures.as_slice() else {
(U) crates/pull/src/gdfl_import_quality.rs:790:24 False: if let Some(slot) = self.shared.get_mut(from) {
(U) crates/pull/src/gdfl_import_quality.rs:793:24 False: if let Some(slot) = self.shared.get_mut(to) {
(F) crates/pull/src/gdfl_import_scope.rs:327:17 False: assert!(none.members().is_empty() && none.set().is_empty());
(F) crates/pull/src/gdfl_import_scope.rs:327:46 False: assert!(none.members().is_empty() && none.set().is_empty());
(F) crates/pull/src/gdfl_import_scope.rs:374:17 False: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
(F) crates/pull/src/gdfl_import_scope.rs:374:57 False: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1128: ra.failures[0].why
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1415: unreachable!("a refused or done journal reads no day")
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1712: matches!(
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1724: matches!(
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1055:9 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1055:33 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1055:51 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1071:9 False: weekday(expiry) && settles_as_stated("NIFTY", expiry, on),
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1071:28 False: weekday(expiry) && settles_as_stated("NIFTY", expiry, on),
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1672:9 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1672:25 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1672:44 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1672:61 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1771:13 False: assert!(weekday(cutover) && weekday(weeklies));
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1771:33 False: assert!(weekday(cutover) && weekday(weeklies));
(F) crates/pull/src/gdfl_r2_attack_tests.rs:1935:13 False: !text.contains("2023-06-29") && !text.contains("2023-08-15")
(U) crates/pull/src/gdfl_rekey.rs:863: return Ok(out);
(U) crates/pull/src/gdfl_rekey.rs:883: Err(_) => return Ok(out),
(U) crates/pull/src/gdfl_rekey.rs:1194: Some((seen, _)) => {
(U) crates/pull/src/gdfl_rekey.rs:1195: return Verdict::Unresolved(format!(
(U) crates/pull/src/gdfl_rekey.rs:1196: "the decoder settles its days on two expiries, {seen} and {}; \
(U) crates/pull/src/gdfl_rekey.rs:1197: refused rather than split",
(U) crates/pull/src/gdfl_rekey.rs:1198: this.0
(U) crates/pull/src/gdfl_rekey.rs:1199: ));
(U, type invariant) crates/pull/src/gdfl_rekey.rs:1438: continue;
(U) crates/pull/src/gdfl_rekey.rs:1747: continue;
(U) crates/pull/src/gdfl_rekey.rs:2293: }
(U) crates/pull/src/gdfl_rekey.rs:320:35 False: while let Some(next) = self.superseded.get(&on) {
(U) crates/pull/src/gdfl_rekey.rs:859:9 False: let (Some(first), Some(last)) = (
(U) crates/pull/src/gdfl_rekey.rs:1193:27 False: Some(seen) if seen == this => {}
(U) crates/pull/src/gdfl_rekey.rs:1230:39 False: Some((expiry, Some(kind))) if expiry != key.expiry => {
(U) crates/pull/src/gdfl_rekey.rs:1328:16 False: if let Some(median) = lower_median(&mut strikes) {
(U, type invariant) crates/pull/src/gdfl_rekey.rs:1437:17 False: let Ok(month) = day.year_month() else {
(U) crates/pull/src/gdfl_rekey.rs:1673:12 False: if at < out.moves.len() {
(U) crates/pull/src/gdfl_rekey.rs:1746:17 False: let Some(one) = out.moves.get(at) else {
(U) crates/pull/src/gdfl_rekey.rs:1774:12 False: if at < out.moves.len() {
(U) crates/pull/src/gdfl_rekey.rs:2291:12 False: if let Some(dir) = lineage.parent() {
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:811: done.refused[0].why
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1062: done.refused[0].why
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1235: panic!("a contract past the store's width is not moved");
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1243: panic!("a contract that renders moves");
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:349:13 False: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:349:38 False: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:349:59 False: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:479:13 False: assert!(decided.recorded_moves.is_empty() && decided.recorded_retired.is_empty());
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:479:50 False: assert!(decided.recorded_moves.is_empty() && decided.recorded_retired.is_empty());
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1064:13 False: assert!(done.moved.is_empty() && done.lines_written == 0);
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1064:38 False: assert!(done.moved.is_empty() && done.lines_written == 0);
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1234:9 False: let Verdict::Unresolved(why) = judge(&wide, &HashMap::new()) else {
(F) crates/pull/src/gdfl_rekey_ps2_tests.rs:1242:9 False: let Verdict::Move(to, kind) = judge(&narrow, &HashMap::new()) else {
(F) crates/pull/src/gdfl_rekey_tests.rs:629: done.refused[0].why
(F) crates/pull/src/gdfl_rekey_tests.rs:814: refusal.refusal()
(F) crates/pull/src/gdfl_rekey_tests.rs:1284: decided.unresolved[0].why
(F) crates/pull/src/gdfl_rekey_tests.rs:1316: || panic!("{key}: {:?}", decided.unresolved),
(F) crates/pull/src/gdfl_rekey_tests.rs:1760: panic!("{:?}", decided.census);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2078: fn synthetic(n: usize) -> (Survey, Withdrawn) {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2079: let under = sym("NIFTY");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2080: let mut keys = Vec::with_capacity(n);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2081: let mut ledger = String::new();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2082: for at in 0..n {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2083: let stated = at % 10 == 9;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2085: (d(2023, 6, 29), d(2020, 6, 15))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2087: (d(2019, 6, 27), d(2019, 6, 3))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2089: let strike = 1_000_000 + 5_000 * i64::try_from(at / 2).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2091: let text = format!("{expiry}-{strike}-{side}");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2092: let contract = c(&text);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2093: let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + 34_000) * 1_000_000;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2094: keys.push(Stored {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2095: underlying: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2096: contract,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2097: expiry,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2098: strike,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2100: OptionSide::Put
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2102: OptionSide::Call
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2104: months: vec![Month {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2105: month: day.year_month().unwrap(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2106: rows: 2,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2107: first_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2108: last_ts_micros: ts + 1_000_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2109: }],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2110: days: BTreeMap::from([(day, 2)]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2113: let to = c(&format!("2023-06-28-{strike}-{side}"));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2114: let record = SupersededRecord {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2115: underlying: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2116: old: contract,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2117: new: to,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2118: kind: AliasKind::VendorRedated,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2119: months: 1,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2120: bars: 2,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2121: digest: [9; 32],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2122: };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2123: ledger.push_str(&record.line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2124: ledger.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2125: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2127: keys.sort_by(|a, b| a.contract.as_str().cmp(b.contract.as_str()));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2128: (
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2129: Survey {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2130: keys,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2131: ..Survey::default()
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2132: },
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2133: Withdrawn::parse(&ledger).unwrap(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2134: )
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2135: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2142: fn plan_cost_per_key_is_flat_in_the_number_of_keys() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2143: for n in [1_000_usize, 10_000] {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2144: let (found, ledger) = synthetic(n);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2145: let mut per_key: Vec<u128> = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2146: let mut judged = 0;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2147: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2148: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2149: let decided = plan(&found, &ledger);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2150: per_key.push(started.elapsed().as_nanos() / n as u128);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2151: judged = decided.keys;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2152: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2153: assert_eq!(judged, n as u64);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2154: per_key.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2155: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2157: per_key[99], per_key[197], per_key[199]
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2160: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2170: fn census_step_cost_is_one_load_per_plan_flat_per_entry() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2171: for n in [10_000_usize, 100_000] {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2172: let scratch = Scratch::new(&format!("census-cost-{n}"));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2173: let root = scratch.root();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2174: let under = sym("NIFTY");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2175: let month = YearMonth::new(2020, 6).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2176: let mut census =
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2177: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, u64::try_from(n).unwrap())
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2178: .unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2179: let mut ledger = String::new();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2180: for at in 0..n {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2181: let key = c(&format!("2023-06-28-{}-PE", 100_000 + at * 5));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2182: let ts = 1_592_193_400_000_000 + i64::try_from(at).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2183: census
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2184: .record_held(Held::unknown(Entry {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2185: key: EntryKey {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2186: contract: Some(key),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2187: exchange: Exchange::Nse,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2188: segment: Segment::Fno,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2189: symbol: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2190: timeframe: RUNG,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2191: month,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2192: },
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2193: rows: 1,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2194: first_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2195: last_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2196: }))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2197: .unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198: if at < 13 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2199: ledger.push_str(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2200: &RetiredRecord {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2201: key,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2202: ..refusal_of("2019-05-30-1150000-CE")
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2203: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2204: .line(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2205: );
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2206: ledger.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2207: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2209: let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2210: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2211: std::fs::write(&path, census.image()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2212: let ledger = Withdrawn::parse(&ledger).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2213: let mut per_plan: Vec<u128> = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2214: let mut listed = 0;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2215: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2216: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2217: let step = census_step(root, &ledger);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2218: per_plan.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2219: listed = step.pending();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2220: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2221: assert_eq!(listed, 13);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2222: per_plan.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2223: let per_entry = |ns: u128| ns / n as u128;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2224: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2227: per_plan[99] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2228: per_plan[197] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2229: per_plan[199] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2230: per_entry(per_plan[99]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2231: per_entry(per_plan[197]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2232: per_entry(per_plan[199])
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2235: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2582: fn spread(mut samples: Vec<u128>) -> String {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2583: samples.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2584: format!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2586: samples[99],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2587: samples[197],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2588: samples[199],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2589: samples.len()
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2591: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2603: fn apply_guard_cost_per_operation() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2604: let fixture = Fixture::new("guardcost");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2605: fixture.counted();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2606: let root = fixture.root();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2607: let at = ledger_lock_path(root);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2608: let mut lock = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2609: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2610: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2611: let file = open_ledger_lock(&at).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2612: let held = store::flock::Flock::try_lock(&file, at.as_path()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2613: held.release().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2614: lock.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2615: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2616: let mut text = String::with_capacity(10_000 * 160);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2617: for strike in 0..10_000_i64 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2618: let old = format!("2023-06-29-{}-PE", 1_000_000 + strike * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2619: let new = format!("2023-06-28-{}-PE", 1_000_000 + strike * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2620: text.push_str(&pointer(&old, &new).line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2621: text.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2622: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2623: let read = Withdrawn::parse(&text).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2624: let again = Withdrawn::parse(&text).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2625: let mut compare = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2626: let mut rules = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2627: let mut narrowed = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2628: let only: BTreeSet<Symbol> = [sym("NIFTY")].into_iter().collect();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2629: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2630: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2631: read.same_records(&again, None).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2632: compare.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2633: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2634: read.same_records(&again, Some(&only)).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2635: narrowed.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2636: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2637: read.targets_continue().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2638: rules.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2639: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2640: let mut chain = String::with_capacity(10_000 * 160);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2641: for at in 0..10_000_i64 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2642: let from = format!("2023-06-29-{}-PE", 1_000_000 + at * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2643: let to = format!("2023-06-29-{}-PE", 1_000_000 + (at + 1) * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2644: chain.push_str(&pointer(&from, &to).line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2645: chain.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2646: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2647: let linked = Withdrawn::parse(&chain).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2648: let mut walk = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2649: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2650: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2651: linked.targets_continue().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2652: walk.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2653: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2654: let mut source = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2655: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2656: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2657: let (months, bars) = source_now(root, sym("NIFTY"), c(STATED)).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2658: source.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2659: assert_eq!((months.len(), bars), (2, 3));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2661: eprintln!("APPLY_GUARD lock_open_take_release {}", spread(lock));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2662: eprintln!("APPLY_GUARD same_records_10000_lines {}", spread(compare));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2663: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2665: spread(narrowed)
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2667: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2669: spread(rules)
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2671: eprintln!("APPLY_GUARD targets_continue_10000_chain {}", spread(walk));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2672: eprintln!("APPLY_GUARD source_now_2_months {}", spread(source));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2673: }
(F) crates/pull/src/gdfl_rekey_tests.rs:425:13 False: assert!(decided.retired.is_empty() && decided.moves.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:425:43 False: assert!(decided.retired.is_empty() && decided.moves.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:543:13 False: assert!(again.moves.is_empty() && again.retired.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:543:39 False: assert!(again.moves.is_empty() && again.retired.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1024:13 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:1024:39 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:1027:9 False: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1027:34 False: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1048:13 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:1048:39 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:1162:13 False: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1162:49 False: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1278:13 False: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1278:41 False: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1409:9 False: why.contains("and ledger line 1 retires it as")
(F) crates/pull/src/gdfl_rekey_tests.rs:1410:16 False: && why.contains("one retirement has one account"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1445:13 False: assert!(done.refused.is_empty() && done.moved.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1445:40 False: assert!(done.refused.is_empty() && done.moved.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1581:9 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
(F) crates/pull/src/gdfl_rekey_tests.rs:1581:37 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
(F) crates/pull/src/gdfl_rekey_tests.rs:1581:67 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
(F) crates/pull/src/gdfl_rekey_tests.rs:1705:13 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1705:40 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1705:69 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
(F) crates/pull/src/gdfl_rekey_tests.rs:1759:9 False: let CensusStep::Unreadable(why) = &decided.census else {
(F) crates/pull/src/gdfl_rekey_tests.rs:1867:24 False: .any(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}")
(F) crates/pull/src/gdfl_rekey_tests.rs:1894:9 False: left.why.contains("(apply's own check: ")
(F) crates/pull/src/gdfl_rekey_tests.rs:1895:16 False: && left.why.contains("would precede or contradict"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1945:9 False: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1945:46 False: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1952:9 False: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1952:46 False: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1956:9 False: why.contains("0707070707070707") && why.contains("0808080808080808"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1956:45 False: why.contains("0707070707070707") && why.contains("0808080808080808"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1965:9 False: why.contains("line 1 retires") && why.contains("and line 2 as"),
(F) crates/pull/src/gdfl_rekey_tests.rs:1965:43 False: why.contains("line 1 retires") && why.contains("and line 2 as"),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084:32 True: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084:32 False: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090:23 True: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090:23 False: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099:22 True: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099:22 False: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:12 True: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:12 False: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:22 True: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:22 False: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198:16 True: if at < 13 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198:16 False: if at < 13 {
(F) crates/pull/src/gdfl_rekey_tests.rs:2301:17 False: why.contains("the ledger changed since this plan; re-plan")
(F) crates/pull/src/gdfl_rekey_tests.rs:2302:24 False: && why.contains("nothing was written"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2339:9 False: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2339:46 False: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
(F, by hand) crates/pull/src/gdfl_rekey_tests.rs:2394:17 False: refused.contains("nothing was written")
(F, by hand) crates/pull/src/gdfl_rekey_tests.rs:2396:28 False: || refused.contains("the ledger changed since this plan")),
(F) crates/pull/src/gdfl_rekey_tests.rs:2427:13 False: why.contains("the plan counted 3 bar(s)")
(F) crates/pull/src/gdfl_rekey_tests.rs:2428:20 False: && why.contains("bars landed under it after the survey; re-plan"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2454:13 False: why.contains("retires NSE-FNO-NIFTY-2020-06-25-191250000-CE")
(F) crates/pull/src/gdfl_rekey_tests.rs:2455:20 False: && why.contains("bars move only into a key that continues"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2458:17 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:2458:43 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
(F) crates/pull/src/gdfl_rekey_tests.rs:2574:9 False: left.why.contains("retires it with 1 bar(s) on 1 day(s)")
(F) crates/pull/src/gdfl_rekey_tests.rs:2575:16 False: && left.why.contains("holds 2 bar(s) on 1 day(s) now"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2707:9 False: refused.why.contains("the plan counted 1 bar(s) to retire")
(F) crates/pull/src/gdfl_rekey_tests.rs:2708:16 False: && refused.why.contains("it holds 2 bar(s) in 1 month(s) now")
(F) crates/pull/src/gdfl_rekey_tests.rs:2709:16 False: && refused.why.ends_with("re-plan"),
(F) crates/pull/src/gdfl_rekey_tests.rs:2721:9 False: census_before.contains(&row) && listed.contains(&row),
(F) crates/pull/src/gdfl_rekey_tests.rs:2721:41 False: census_before.contains(&row) && listed.contains(&row),
(F) crates/pull/src/gdfl_rekey_tests.rs:2795:13 False: why.contains("the ledger changed since this plan; re-plan")
(F) crates/pull/src/gdfl_rekey_tests.rs:2796:20 False: && why.contains("nothing was written"),
(U) crates/pull/src/gdfl_run.rs:1431: }
(U) crates/pull/src/gdfl_run.rs:1490: return Some(month(first));
(U, type invariant) crates/pull/src/gdfl_run.rs:1499: return None;
(U) crates/pull/src/gdfl_run.rs:1950: format!(
(U) crates/pull/src/gdfl_run.rs:1952: kind.as_str()
(U) crates/pull/src/gdfl_run.rs:1954: })
(U, type invariant) crates/pull/src/gdfl_run.rs:1412:16 False: && held.source == source
(U) crates/pull/src/gdfl_run.rs:1423:16 False: if let Some(entry) = self.held.get_mut(slot) {
(U) crates/pull/src/gdfl_run.rs:1489:21 False: let Ok(next) = day.succ() else {
(U, type invariant) crates/pull/src/gdfl_run.rs:1498:13 False: let Ok(next) = end.succ() else {
(F) crates/pull/src/gdfl_run_cov_tests.rs:421: other => panic!("{source:?}: {other:?}"),
(F) crates/pull/src/gdfl_run_cov_tests.rs:467: panic!("{end:?}");
(F) crates/pull/src/gdfl_run_cov_tests.rs:466:9 False: let KindEnd::Refused(refusal) = &end else {
(U) crates/pull/src/gdfl_run_ledger.rs:186: return Err(format!(
(U) crates/pull/src/gdfl_run_ledger.rs:187: "a record body of {} bytes is past {BODY}",
(U) crates/pull/src/gdfl_run_ledger.rs:188: body.len()
(U) crates/pull/src/gdfl_run_ledger.rs:189: ));
(U) crates/pull/src/gdfl_run_ledger.rs:223: return None;
(U) crates/pull/src/gdfl_run_ledger.rs:185:12 True: if body.len() > BODY {
(U) crates/pull/src/gdfl_run_ledger.rs:202:16 False: if let Some(last) = out.last_mut() {
(U) crates/pull/src/gdfl_run_ledger.rs:222:13 False: let [id, mode, kind, by, pid, started] = fixed else {
(U) crates/pull/src/gdfl_run_ledger.rs:540:12 False: if let Some(last) = record.last_mut() {
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:165: panic!("not an io fault");
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:197: panic!("not an io fault");
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:212: panic!("not an io fault");
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:164:17 False: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:196:9 False: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
(F) crates/pull/src/gdfl_run_ledger_ps2_tests.rs:211:9 False: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
(F) crates/pull/src/gdfl_run_tests.rs:627: panic!("{end:?}");
(F) crates/pull/src/gdfl_run_tests.rs:681: panic!("{ends:?}");
IGNORED crates/pull/src/gdfl_run_tests.rs:856: fn source_cache_hit_timing() {
IGNORED crates/pull/src/gdfl_run_tests.rs:857: let root = scratch("cache.timing");
IGNORED crates/pull/src/gdfl_run_tests.rs:858: let ids = [SourceId::CmZip, SourceId::NfoZips, SourceId::TickStore];
IGNORED crates/pull/src/gdfl_run_tests.rs:859: let mut dirs = Vec::new();
IGNORED crates/pull/src/gdfl_run_tests.rs:860: for id in ids {
IGNORED crates/pull/src/gdfl_run_tests.rs:861: let dir = root.join(id.as_str());
IGNORED crates/pull/src/gdfl_run_tests.rs:862: std::fs::create_dir_all(&dir).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:863: std::fs::write(dir.join("FORMAT.md"), b"synthetic\n").unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:864: dirs.push(dir);
IGNORED crates/pull/src/gdfl_run_tests.rs:865: }
IGNORED crates/pull/src/gdfl_run_tests.rs:866: let mut cache = SourceCache::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:867: for (id, dir) in ids.iter().zip(&dirs) {
IGNORED crates/pull/src/gdfl_run_tests.rs:868: let _first = cache.origin(*id, dir, Some("abc"));
IGNORED crates/pull/src/gdfl_run_tests.rs:869: }
IGNORED crates/pull/src/gdfl_run_tests.rs:870: let mut took = Vec::with_capacity(6_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:871: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:872: for (id, dir) in ids.iter().zip(&dirs) {
IGNORED crates/pull/src/gdfl_run_tests.rs:873: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:874: let _hit = cache.origin(*id, dir, Some("abc"));
IGNORED crates/pull/src/gdfl_run_tests.rs:875: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:876: }
IGNORED crates/pull/src/gdfl_run_tests.rs:878: assert_eq!(cache.measures(), 3, "every call after the first three hit");
IGNORED crates/pull/src/gdfl_run_tests.rs:879: took.sort_unstable();
IGNORED crates/pull/src/gdfl_run_tests.rs:880: let n = took.len();
IGNORED crates/pull/src/gdfl_run_tests.rs:881: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:883: took[n / 2],
IGNORED crates/pull/src/gdfl_run_tests.rs:884: took[n * 99 / 100],
IGNORED crates/pull/src/gdfl_run_tests.rs:885: took[n - 1]
IGNORED crates/pull/src/gdfl_run_tests.rs:887: std::fs::remove_dir_all(root).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:888: }
(F) crates/pull/src/gdfl_run_tests.rs:1467: panic!("a prefix of a journal is a journal: cut at {cut_at}");
IGNORED crates/pull/src/gdfl_run_tests.rs:1497: fn journal_load_and_refresh_costs() {
IGNORED crates/pull/src/gdfl_run_tests.rs:1498: let store = scratch("costs");
IGNORED crates/pull/src/gdfl_run_tests.rs:1499: let path = journal_path(&store);
IGNORED crates/pull/src/gdfl_run_tests.rs:1500: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1501: let names: Vec<String> = (0..60).map(|i| format!("SYMQ{i:04}")).collect();
IGNORED crates/pull/src/gdfl_run_tests.rs:1502: let long_filter = names.join(",");
IGNORED crates/pull/src/gdfl_run_tests.rs:1505: let runs: [(&str, usize); 5] = [
IGNORED crates/pull/src/gdfl_run_tests.rs:1506: ("*", 9_180),
IGNORED crates/pull/src/gdfl_run_tests.rs:1507: ("*", 91_800),
IGNORED crates/pull/src/gdfl_run_tests.rs:1508: ("*", 918_000),
IGNORED crates/pull/src/gdfl_run_tests.rs:1509: ("long", 23_270),
IGNORED crates/pull/src/gdfl_run_tests.rs:1510: ("long", 232_700),
IGNORED crates/pull/src/gdfl_run_tests.rs:1511: ];
IGNORED crates/pull/src/gdfl_run_tests.rs:1512: for (filter, lines) in runs {
IGNORED crates/pull/src/gdfl_run_tests.rs:1513: let line_of = |on: &str| {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1515: done(on)
IGNORED crates/pull/src/gdfl_run_tests.rs:1517: format!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1522: };
IGNORED crates/pull/src/gdfl_run_tests.rs:1523: let mut text = format!("{RUN} definition={BAR_DEFINITION}\n");
IGNORED crates/pull/src/gdfl_run_tests.rs:1524: let mut on = day(1990, 1, 1);
IGNORED crates/pull/src/gdfl_run_tests.rs:1525: for _ in 0..lines {
IGNORED crates/pull/src/gdfl_run_tests.rs:1526: text.push_str(&line_of(&on.to_string()));
IGNORED crates/pull/src/gdfl_run_tests.rs:1527: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1528: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1529: std::fs::write(&path, &text).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1530: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1531: let mut loads = Vec::with_capacity(n);
IGNORED crates/pull/src/gdfl_run_tests.rs:1532: for _ in 0..n {
IGNORED crates/pull/src/gdfl_run_tests.rs:1533: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1534: let _reader = JournalReader::open(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1535: loads.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1536: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1537: let mut reader = JournalReader::open(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1538: let mut refreshes = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_run_tests.rs:1539: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1542: let mut file = std::fs::OpenOptions::new()
IGNORED crates/pull/src/gdfl_run_tests.rs:1543: .append(true)
IGNORED crates/pull/src/gdfl_run_tests.rs:1544: .open(&path)
IGNORED crates/pull/src/gdfl_run_tests.rs:1545: .unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1546: file.write_all(line_of(&on.to_string()).as_bytes()).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1548: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1549: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1550: assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
IGNORED crates/pull/src/gdfl_run_tests.rs:1551: refreshes.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1553: let pct = |v: &mut Vec<std::time::Duration>| {
IGNORED crates/pull/src/gdfl_run_tests.rs:1554: v.sort();
IGNORED crates/pull/src/gdfl_run_tests.rs:1555: let at = |q: usize| v[(v.len() * q / 100).min(v.len() - 1)];
IGNORED crates/pull/src/gdfl_run_tests.rs:1556: (at(50), at(99), v[v.len() - 1], v.len())
IGNORED crates/pull/src/gdfl_run_tests.rs:1557: };
IGNORED crates/pull/src/gdfl_run_tests.rs:1558: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1560: text.len(),
IGNORED crates/pull/src/gdfl_run_tests.rs:1561: pct(&mut loads),
IGNORED crates/pull/src/gdfl_run_tests.rs:1562: pct(&mut refreshes)
IGNORED crates/pull/src/gdfl_run_tests.rs:1565: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1566: std::fs::copy(sample, &path).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1567: let mut loads = Vec::with_capacity(50);
IGNORED crates/pull/src/gdfl_run_tests.rs:1568: let mut loaded = 0;
IGNORED crates/pull/src/gdfl_run_tests.rs:1569: for _ in 0..50 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1570: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1571: loaded = JournalReader::open(&store).unwrap().loaded();
IGNORED crates/pull/src/gdfl_run_tests.rs:1572: loads.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1573: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1574: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1576: spread(&mut loads)
IGNORED crates/pull/src/gdfl_run_tests.rs:1578: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1579: std::fs::remove_dir_all(store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1580: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1583: fn spread(samples: &mut [std::time::Duration]) -> String {
IGNORED crates/pull/src/gdfl_run_tests.rs:1584: samples.sort();
IGNORED crates/pull/src/gdfl_run_tests.rs:1585: let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
IGNORED crates/pull/src/gdfl_run_tests.rs:1586: format!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1588: at(50),
IGNORED crates/pull/src/gdfl_run_tests.rs:1589: at(99),
IGNORED crates/pull/src/gdfl_run_tests.rs:1590: samples[samples.len() - 1],
IGNORED crates/pull/src/gdfl_run_tests.rs:1591: samples.len()
IGNORED crates/pull/src/gdfl_run_tests.rs:1593: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1601: fn progress_and_failure_costs() {
IGNORED crates/pull/src/gdfl_run_tests.rs:1602: let root = scratch("write-costs");
IGNORED crates/pull/src/gdfl_run_tests.rs:1603: let store = root.join("store");
IGNORED crates/pull/src/gdfl_run_tests.rs:1604: std::fs::create_dir_all(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1605: let log = installed();
IGNORED crates/pull/src/gdfl_run_tests.rs:1606: let planned = plan(
IGNORED crates/pull/src/gdfl_run_tests.rs:1607: request(&[ImportKind::Options]),
IGNORED crates/pull/src/gdfl_run_tests.rs:1608: &process(&store, &log),
IGNORED crates/pull/src/gdfl_run_tests.rs:1609: &env_with(vec![(TICKSTORE_VAR, root.clone())]),
IGNORED crates/pull/src/gdfl_run_tests.rs:1610: &mut SourceCache::default(),
IGNORED crates/pull/src/gdfl_run_tests.rs:1612: .unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1613: let progress = ProgressFile::new(
IGNORED crates/pull/src/gdfl_run_tests.rs:1614: store.join("imports").join("gdfl.options.progress"),
IGNORED crates/pull/src/gdfl_run_tests.rs:1615: &planned,
IGNORED crates/pull/src/gdfl_run_tests.rs:1616: &planned.kinds[0],
IGNORED crates/pull/src/gdfl_run_tests.rs:1619: let mut report = Report::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:1620: let mut writes = Vec::with_capacity(1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1621: for closed in 1..=1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1622: report.days_imported = closed;
IGNORED crates/pull/src/gdfl_run_tests.rs:1623: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1624: progress.day_passed(ImportKind::Options, day(2024, 4, 1), &report);
IGNORED crates/pull/src/gdfl_run_tests.rs:1625: writes.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1626: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1627: assert_eq!(progress.writes(), 1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1628: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1630: spread(&mut writes)
IGNORED crates/pull/src/gdfl_run_tests.rs:1632: let cause = Cause::open(Stage::Vendor, "malformed_row");
IGNORED crates/pull/src/gdfl_run_tests.rs:1633: let mut named = Report::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:1634: let mut said = String::new();
IGNORED crates/pull/src/gdfl_run_tests.rs:1635: for total in [FAILURES_KEPT, 10 * FAILURES_KEPT, 100 * FAILURES_KEPT] {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636: while named.failures_total < total - 1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1637: named.name(cause, failure(named.failures_total));
IGNORED crates/pull/src/gdfl_run_tests.rs:1638: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1639: let mut each = Vec::with_capacity(1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1640: for _ in 0..1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1641: let one = failure(named.failures_total);
IGNORED crates/pull/src/gdfl_run_tests.rs:1642: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1643: named.name(cause, one);
IGNORED crates/pull/src/gdfl_run_tests.rs:1644: each.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1645: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1646: assert!(named.failures.len() <= 2 * FAILURES_KEPT);
IGNORED crates/pull/src/gdfl_run_tests.rs:1647: let _ = std::fmt::Write::write_fmt(
IGNORED crates/pull/src/gdfl_run_tests.rs:1648: &mut said,
IGNORED crates/pull/src/gdfl_run_tests.rs:1649: format_args!("at {total} named: {}; ", spread(&mut each)),
IGNORED crates/pull/src/gdfl_run_tests.rs:1650: );
IGNORED crates/pull/src/gdfl_run_tests.rs:1652: println!("C-GR-06 Report::name: {said}");
IGNORED crates/pull/src/gdfl_run_tests.rs:1653: std::fs::remove_dir_all(root).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1654: }
(F) crates/pull/src/gdfl_run_tests.rs:370:9 False: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
(F) crates/pull/src/gdfl_run_tests.rs:370:62 False: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
(F) crates/pull/src/gdfl_run_tests.rs:626:13 False: let KindEnd::Report(report) = end else {
(F) crates/pull/src/gdfl_run_tests.rs:680:9 False: let KindEnd::Unopened(refused) = &ends[0].1 else {
(F) crates/pull/src/gdfl_run_tests.rs:1381:44 False: Refreshed::Reloaded { now, .. } if now == head.len() as u64
(F) crates/pull/src/gdfl_run_tests.rs:1466:13 False: let Ok(mut reader) = JournalReader::open(&store) else {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514:16 True: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514:16 False: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1530:20 True: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1530:20 False: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1565:12 True: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1565:12 False: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636:15 True: while named.failures_total < total - 1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636:15 False: while named.failures_total < total - 1_000 {
(F) crates/pull/src/gdfl_scope_tests.rs:217: path.display()
(F) crates/pull/src/gdfl_scope_tests.rs:745: after.keys().collect::<Vec<_>>()
(F) crates/pull/src/gdfl_scope_tests.rs:1275: closes[2]
(F) crates/pull/src/gdfl_scope_tests.rs:1694: panic!("{d}: a close with no scope owes the members beyond the swept surface");
(F) crates/pull/src/gdfl_scope_tests.rs:2231: journal_text(&store)
(F) crates/pull/src/gdfl_scope_tests.rs:2287: journal_text(&store)
(F) crates/pull/src/gdfl_scope_tests.rs:2450: journal_text(&store)
(F) crates/pull/src/gdfl_scope_tests.rs:2551: journal_text(&store)
(F) crates/pull/src/gdfl_scope_tests.rs:575:13 False: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
(F) crates/pull/src/gdfl_scope_tests.rs:575:45 False: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
(F) crates/pull/src/gdfl_scope_tests.rs:749:9 False: nifty.len() > before.len() && nifty.starts_with(&before),
(F) crates/pull/src/gdfl_scope_tests.rs:749:39 False: nifty.len() > before.len() && nifty.starts_with(&before),
(F) crates/pull/src/gdfl_scope_tests.rs:1273:9 False: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
(F) crates/pull/src/gdfl_scope_tests.rs:1273:47 False: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
(F) crates/pull/src/gdfl_scope_tests.rs:1693:13 False: let Owed::Probe(beyond) = owed else {
(F) crates/pull/src/gdfl_scope_tests.rs:1708:13 False: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
(F) crates/pull/src/gdfl_scope_tests.rs:1708:54 False: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
(F) crates/pull/src/gdfl_scope_tests.rs:2059:13 False: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
(F) crates/pull/src/gdfl_scope_tests.rs:2059:48 False: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
(F) crates/pull/src/gdfl_scope_tests.rs:2078:13 False: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
(F) crates/pull/src/gdfl_scope_tests.rs:2078:43 False: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
(F) crates/pull/src/gdfl_scope_tests.rs:2283:17 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
(F) crates/pull/src/gdfl_scope_tests.rs:2283:44 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
(F) crates/pull/src/gdfl_scope_tests.rs:2283:65 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
(F, by hand) crates/pull/src/gdfl_sealed_tests.rs:51:29 False: parts.len() == 7 || parts.len() == 8,
(F) crates/pull/src/gdfl_sealed_tests.rs:132:17 False: assert!(got.seconds > 0 && got.files > 0);
(F) crates/pull/src/gdfl_sealed_tests.rs:132:36 False: assert!(got.seconds > 0 && got.files > 0);
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:344: (got, want) => panic!("case {case} {kind:?} {ticks:?}: {got:?} against {want:?}"),
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:832: matches!(
(F, by hand) crates/pull/src/gdfl_seconds_attack_tests.rs:147:27 False: assert!(k == 0 || placed[k - 1].0 <= *s, "placement runs forward");
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:349:9 False: refused > 100 && agreed > 90_000 && bars > 200_000,
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:349:26 False: refused > 100 && agreed > 90_000 && bars > 200_000,
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:349:45 False: refused > 100 && agreed > 90_000 && bars > 200_000,
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:966:13 False: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:966:25 False: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:976:17 False: assert!(r.ltp >= 5 && r.sod < 86_400);
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:976:31 False: assert!(r.ltp >= 5 && r.sod < 86_400);
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1066:9 False: listed > 1_000 && rebuilt_clean > 1_000,
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1066:27 False: listed > 1_000 && rebuilt_clean > 1_000,
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1382:13 False: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1382:29 False: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1666:13 False: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
(F) crates/pull/src/gdfl_seconds_attack_tests.rs:1666:44 False: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
(F) crates/pull/src/gdfl_v3_tests.rs:207: panic!("a listing without its directories agreed");
(F) crates/pull/src/gdfl_v3_tests.rs:221: panic!("a configured ledger is unset");
(F) crates/pull/src/gdfl_v3_tests.rs:304: panic!("two differences agreed");
(F) crates/pull/src/gdfl_v3_tests.rs:311: panic!("a renamed path agreed");
(F) crates/pull/src/gdfl_v3_tests.rs:337: panic!("a longer listing agreed");
(F) crates/pull/src/gdfl_v3_tests.rs:465: panic!("a directory at the marker's name was trusted");
(F) crates/pull/src/gdfl_v3_tests.rs:574: panic!("a directory at the manifest's name was trusted");
(F) crates/pull/src/gdfl_v3_tests.rs:763: panic!("an empty manifest agreed with an entry");
IGNORED crates/pull/src/gdfl_v3_tests.rs:864: fn the_real_v3_ledger_agrees_with_the_real_year_zips() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:866: let env = |var: &str| std::env::var_os(var);
IGNORED crates/pull/src/gdfl_v3_tests.rs:867: let ledger = V3Ledger::from_env(&env);
IGNORED crates/pull/src/gdfl_v3_tests.rs:868: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:869: panic!("BRUTEX_GDFL_V3_LEDGER is unset");
IGNORED crates/pull/src/gdfl_v3_tests.rs:871: let dir = std::path::PathBuf::from(env("BRUTEX_GDFL_NFO_ARCHIVES").expect("archives"));
IGNORED crates/pull/src/gdfl_v3_tests.rs:872: let zips = NfoZips::new(&dir);
IGNORED crates/pull/src/gdfl_v3_tests.rs:874: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:875: let first = zips.day(day(2019, 4, 1));
IGNORED crates/pull/src/gdfl_v3_tests.rs:876: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:878: started.elapsed(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:879: matches!(first, Ok(Some(_)))
IGNORED crates/pull/src/gdfl_v3_tests.rs:881: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:882: let again = zips.day(day(2019, 4, 2));
IGNORED crates/pull/src/gdfl_v3_tests.rs:883: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:885: started.elapsed(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:886: matches!(again, Ok(Some(_)))
IGNORED crates/pull/src/gdfl_v3_tests.rs:888: let parse = |text: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:889: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED crates/pull/src/gdfl_v3_tests.rs:890: day(
IGNORED crates/pull/src/gdfl_v3_tests.rs:891: parts[0],
IGNORED crates/pull/src/gdfl_v3_tests.rs:892: u8::try_from(parts[1]).unwrap(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:893: u8::try_from(parts[2]).unwrap(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:895: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:897: for year in 2018..=2026_u16 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:898: let count = |sub: &str, suffix: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:899: std::fs::read_dir(root.join(format!("{year}/_v3/{sub}")))
IGNORED crates/pull/src/gdfl_v3_tests.rs:900: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:901: .filter(|name| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:902: name.as_ref()
IGNORED crates/pull/src/gdfl_v3_tests.rs:903: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:904: .file_name()
IGNORED crates/pull/src/gdfl_v3_tests.rs:905: .to_str()
IGNORED crates/pull/src/gdfl_v3_tests.rs:906: .is_some_and(|n| n.ends_with(suffix))
IGNORED crates/pull/src/gdfl_v3_tests.rs:907: })
IGNORED crates/pull/src/gdfl_v3_tests.rs:908: .count()
IGNORED crates/pull/src/gdfl_v3_tests.rs:909: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:910: let (manifests, markers) = (count("manifest", ".ok.tsv"), count("ok", ".ok"));
IGNORED crates/pull/src/gdfl_v3_tests.rs:911: eprintln!("v3 {year}: manifests={manifests} ok_markers={markers}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:912: assert_eq!(manifests, markers, "{year}: one ok marker a manifest");
IGNORED crates/pull/src/gdfl_v3_tests.rs:913: assert!(manifests > 0, "{year}: no manifest");
IGNORED crates/pull/src/gdfl_v3_tests.rs:916: let mut records = std::collections::HashMap::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:917: for year in 2018..=2026_u16 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:919: continue;
IGNORED crates/pull/src/gdfl_v3_tests.rs:921: let len = file.metadata().unwrap().len();
IGNORED crates/pull/src/gdfl_v3_tests.rs:922: for entry in crate::gdfl_archive::zip_entries(&file, 0, len).unwrap() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:923: records.insert(entry.name.clone(), (entry.len, entry.crc32));
IGNORED crates/pull/src/gdfl_v3_tests.rs:924: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:926: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:927: list.split(',').map(parse).collect()
IGNORED crates/pull/src/gdfl_v3_tests.rs:929: let from = std::env::var("BRUTEX_V3_FROM").map_or(day(2018, 1, 1), |t| parse(&t));
IGNORED crates/pull/src/gdfl_v3_tests.rs:930: let to = std::env::var("BRUTEX_V3_TO").map_or(day(2026, 12, 31), |t| parse(&t));
IGNORED crates/pull/src/gdfl_v3_tests.rs:931: let mut out = Vec::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:932: let mut at = from;
IGNORED crates/pull/src/gdfl_v3_tests.rs:933: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:934: out.push(at);
IGNORED crates/pull/src/gdfl_v3_tests.rs:935: at = at.succ().unwrap();
IGNORED crates/pull/src/gdfl_v3_tests.rs:936: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:937: out
IGNORED crates/pull/src/gdfl_v3_tests.rs:939: let (mut codes, mut held, mut marker_agrees, mut marker_differs) =
IGNORED crates/pull/src/gdfl_v3_tests.rs:940: ([0_usize; 4], 0_usize, 0_usize, 0_usize);
IGNORED crates/pull/src/gdfl_v3_tests.rs:941: let mut nanos: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:942: let mut lines = 0_usize;
IGNORED crates/pull/src/gdfl_v3_tests.rs:943: for d in days {
IGNORED crates/pull/src/gdfl_v3_tests.rs:944: let listing = match zips.day(d) {
IGNORED crates/pull/src/gdfl_v3_tests.rs:945: Ok(listing) => listing,
IGNORED crates/pull/src/gdfl_v3_tests.rs:946: Err(why) => {
IGNORED crates/pull/src/gdfl_v3_tests.rs:947: eprintln!("v3 {d}: source refused: {why}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:948: continue;
IGNORED crates/pull/src/gdfl_v3_tests.rs:951: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:952: held += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:953: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:954: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:955: let answer = ledger.check(d, listing.as_ref());
IGNORED crates/pull/src/gdfl_v3_tests.rs:956: let took = started.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_v3_tests.rs:957: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:958: continue; // neither holds the day: a weekend or a holiday
IGNORED crates/pull/src/gdfl_v3_tests.rs:959: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:960: nanos.push(took);
IGNORED crates/pull/src/gdfl_v3_tests.rs:961: codes[usize::from(answer.code())] += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:962: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:963: lines += n;
IGNORED crates/pull/src/gdfl_v3_tests.rs:964: } else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:965: eprintln!("v3 {d}: {answer}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:966: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:968: let name = NfoZips::day_zip_name(d);
IGNORED crates/pull/src/gdfl_v3_tests.rs:969: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:970: std::fs::read_to_string(ledger.marker_path(d).unwrap()),
IGNORED crates/pull/src/gdfl_v3_tests.rs:971: records.get(&name),
IGNORED crates/pull/src/gdfl_v3_tests.rs:973: let value = |key: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:974: text.lines()
IGNORED crates/pull/src/gdfl_v3_tests.rs:975: .find_map(|line| line.strip_prefix(&format!("{key}=")))
IGNORED crates/pull/src/gdfl_v3_tests.rs:976: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:977: .to_owned()
IGNORED crates/pull/src/gdfl_v3_tests.rs:978: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:979: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:980: marker_agrees += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:981: } else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:982: marker_differs += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:983: eprintln!("v3 {d}: the ok marker's crc/size are not the day zip's");
IGNORED crates/pull/src/gdfl_v3_tests.rs:984: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:985: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:987: nanos.sort_unstable();
IGNORED crates/pull/src/gdfl_v3_tests.rs:988: let at = |q: usize| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:989: nanos
IGNORED crates/pull/src/gdfl_v3_tests.rs:990: .get((nanos.len().saturating_sub(1)) * q / 100)
IGNORED crates/pull/src/gdfl_v3_tests.rs:991: .copied()
IGNORED crates/pull/src/gdfl_v3_tests.rs:992: .unwrap_or(0)
IGNORED crates/pull/src/gdfl_v3_tests.rs:993: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:994: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:996: codes[0], codes[1], codes[2], codes[3]
IGNORED crates/pull/src/gdfl_v3_tests.rs:998: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:1000: nanos.len(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:1001: at(50) / 1_000,
IGNORED crates/pull/src/gdfl_v3_tests.rs:1002: at(99) / 1_000,
IGNORED crates/pull/src/gdfl_v3_tests.rs:1003: nanos.last().copied().unwrap_or(0) / 1_000
IGNORED crates/pull/src/gdfl_v3_tests.rs:1005: assert_eq!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:1006: codes[2] + codes[3],
IGNORED crates/pull/src/gdfl_v3_tests.rs:1010: assert_eq!(marker_differs, 0);
IGNORED crates/pull/src/gdfl_v3_tests.rs:1011: }
(F) crates/pull/src/gdfl_v3_tests.rs:206:9 False: let V3::Disagrees { count, first } = v3.check(d, Some(&files_only)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:220:9 False: let V3Ledger::At { root } = &v3 else {
(F) crates/pull/src/gdfl_v3_tests.rs:303:9 False: let V3::Disagrees { count, first } = check(&both, Some(99)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:310:9 False: let V3::Disagrees { count, first } = check(&renamed, Some(99)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:336:9 False: let V3::Disagrees { count, first } = check(&longer, Some(99)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:464:9 False: let V3::Unverified { why } = dir_marker.check(d, Some(&good)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:573:9 False: let V3::Unverified { why } = dir_manifest.check(d, Some(&good)) else {
(F) crates/pull/src/gdfl_v3_tests.rs:759:9 False: let V3::Disagrees { count, first } = v3.check(
IGNORED crates/pull/src/gdfl_v3_tests.rs:868:9 True: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:868:9 False: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918:13 True: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918:13 False: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:926:33 True: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:926:33 False: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:933:15 True: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:933:15 False: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:951:12 True: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:951:12 False: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:12 True: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:12 False: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:33 True: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:33 False: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:962:16 True: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:962:16 False: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:969:16 True: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:969:16 False: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:16 True: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:16 False: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:52 True: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:52 False: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
(F) crates/pull/src/landing_run_tests.rs:62: Transport::LocalArchive(_) => panic!("Zerodha is an HTTP feed"),
(F) crates/pull/src/landing_run_tests.rs:387: mode.word()
(F) crates/pull/src/landing_run_tests.rs:1155: ledger_text(&root, "repairs")
(F) crates/pull/src/landing_run_tests.rs:1330: before.len()
(F) crates/pull/src/landing_run_tests.rs:1692: notes[0]
(F) crates/pull/src/landing_run_tests.rs:1882: hook.series_room()
IGNORED crates/pull/src/landing_run_tests.rs:1985: fn ia10c_timings() {
IGNORED crates/pull/src/landing_run_tests.rs:1987: fn pct(mut v: Vec<std::time::Duration>) -> String {
IGNORED crates/pull/src/landing_run_tests.rs:1988: v.sort_unstable();
IGNORED crates/pull/src/landing_run_tests.rs:1989: let n = v.len();
IGNORED crates/pull/src/landing_run_tests.rs:1990: format!(
IGNORED crates/pull/src/landing_run_tests.rs:1992: v[n / 2],
IGNORED crates/pull/src/landing_run_tests.rs:1993: v[n * 99 / 100],
IGNORED crates/pull/src/landing_run_tests.rs:1994: v[n - 1]
IGNORED crates/pull/src/landing_run_tests.rs:1996: }
IGNORED crates/pull/src/landing_run_tests.rs:1997: let base = std::env::var_os("BRUTEX_IA10C_TIMING_DIR")
IGNORED crates/pull/src/landing_run_tests.rs:1998: .map_or_else(|| scratch("timings"), PathBuf::from);
IGNORED crates/pull/src/landing_run_tests.rs:1999: std::fs::create_dir_all(&base).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2000: let root = base.join(format!("ia10c-{}", std::process::id()));
IGNORED crates/pull/src/landing_run_tests.rs:2001: std::fs::create_dir_all(&root).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2002: println!("timing under {}", root.display());
IGNORED crates/pull/src/landing_run_tests.rs:2003: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2004: for _ in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2005: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2006: allocate(&root, Mode::Spot, By::Api, "zerodha").unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2007: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2008: }
IGNORED crates/pull/src/landing_run_tests.rs:2009: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2011: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2016: let big = base.join(format!("ia10c-claims-{}", std::process::id()));
IGNORED crates/pull/src/landing_run_tests.rs:2017: std::fs::create_dir_all(big.join("audit")).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2018: let mut text = String::with_capacity(52 << 20);
IGNORED crates/pull/src/landing_run_tests.rs:2019: let mut id = CLOCK;
IGNORED crates/pull/src/landing_run_tests.rs:2020: while text.len() < 50 << 20 {
IGNORED crates/pull/src/landing_run_tests.rs:2021: let record = ledger::Entry {
IGNORED crates/pull/src/landing_run_tests.rs:2022: id,
IGNORED crates/pull/src/landing_run_tests.rs:2023: mode: Mode::Spot.word().to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2024: kind: kind_of(Vendor::Zerodha).to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2025: by: By::Api.word().to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2026: pid: 4242,
IGNORED crates/pull/src/landing_run_tests.rs:2027: started: CLOCK,
IGNORED crates/pull/src/landing_run_tests.rs:2028: resumes: 0,
IGNORED crates/pull/src/landing_run_tests.rs:2029: }
IGNORED crates/pull/src/landing_run_tests.rs:2030: .record()
IGNORED crates/pull/src/landing_run_tests.rs:2031: .unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2032: let body = std::str::from_utf8(&record[..114]).unwrap().trim_end();
IGNORED crates/pull/src/landing_run_tests.rs:2033: let crc = std::str::from_utf8(&record[114..126]).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2034: writeln!(text, "run {body} {crc}").unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2035: id += 1;
IGNORED crates/pull/src/landing_run_tests.rs:2036: }
IGNORED crates/pull/src/landing_run_tests.rs:2037: std::fs::write(big.join(ledger::CLAIMS), &text).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2038: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2039: let seeded = allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2040: let seed_took = began.elapsed();
IGNORED crates/pull/src/landing_run_tests.rs:2041: assert_eq!(seeded, id, "seeded past the last claim");
IGNORED crates/pull/src/landing_run_tests.rs:2042: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2044: id - CLOCK,
IGNORED crates/pull/src/landing_run_tests.rs:2045: text.len()
IGNORED crates/pull/src/landing_run_tests.rs:2047: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2048: for _ in 0..200 {
IGNORED crates/pull/src/landing_run_tests.rs:2049: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2050: allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2051: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2052: }
IGNORED crates/pull/src/landing_run_tests.rs:2053: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2055: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2057: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2058: for _ in 0..20 {
IGNORED crates/pull/src/landing_run_tests.rs:2059: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2060: let largest = ledger::largest_claimed_run(&big).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2061: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2062: assert!(largest >= seeded);
IGNORED crates/pull/src/landing_run_tests.rs:2064: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2066: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2068: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2069: for n in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2070: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2071: let held = claim(
IGNORED crates/pull/src/landing_run_tests.rs:2072: &root,
IGNORED crates/pull/src/landing_run_tests.rs:2073: Vendor::Zerodha,
IGNORED crates/pull/src/landing_run_tests.rs:2074: crate::gdfl_run::locks::Hold::Shared,
IGNORED crates/pull/src/landing_run_tests.rs:2075: &format!("api pid 1 run {n}"),
IGNORED crates/pull/src/landing_run_tests.rs:2076: )
IGNORED crates/pull/src/landing_run_tests.rs:2077: .unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2078: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2079: drop(held);
IGNORED crates/pull/src/landing_run_tests.rs:2080: }
IGNORED crates/pull/src/landing_run_tests.rs:2081: println!("vendor landing claim, shared: {}", pct(took));
IGNORED crates/pull/src/landing_run_tests.rs:2082: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2083: for n in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2084: let line = vec![format!(
IGNORED crates/pull/src/landing_run_tests.rs:2085: "gap NSE-INDEX-ZZT{n} 2024-04-23 v=1 rung=1min owed=375 held=370 missing=5 first_missing=09:15 runs=1 longest=5 longest_at=09:15 why=vendor_
IGNORED crates/pull/src/landing_run_tests.rs:2086: )];
IGNORED crates/pull/src/landing_run_tests.rs:2087: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2088: crate::ingest::audit_ledger::append(&root, Vendor::Zerodha, "gaps", &line).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2089: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2090: }
IGNORED crates/pull/src/landing_run_tests.rs:2091: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2093: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2095: let month = YearMonth::new(2024, 4).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2096: let days = regular_days(month);
IGNORED crates/pull/src/landing_run_tests.rs:2097: let hook = RunHook::new(&root, 77, Vendor::Zerodha, 1);
IGNORED crates/pull/src/landing_run_tests.rs:2098: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2099: for n in 0..200_usize {
IGNORED crates/pull/src/landing_run_tests.rs:2100: let on = days[n % days.len()];
IGNORED crates/pull/src/landing_run_tests.rs:2101: let window = Window::new(on, on).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2102: let body = kite_body(&[on], &[]);
IGNORED crates/pull/src/landing_run_tests.rs:2103: let (rows, answered) = answered(&body);
IGNORED crates/pull/src/landing_run_tests.rs:2104: let landing = land(&root, &rows, window, Door::FromWindow, 77);
IGNORED crates/pull/src/landing_run_tests.rs:2105: let key = key();
IGNORED crates/pull/src/landing_run_tests.rs:2106: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2107: let hooked = hook.after_landing(&Landed {
IGNORED crates/pull/src/landing_run_tests.rs:2108: key: &key,
IGNORED crates/pull/src/landing_run_tests.rs:2109: granularity: crate::vendor::Granularity::Minute1,
IGNORED crates/pull/src/landing_run_tests.rs:2110: asked: window,
IGNORED crates/pull/src/landing_run_tests.rs:2111: answered: &answered,
IGNORED crates/pull/src/landing_run_tests.rs:2112: rows: &rows.rows,
IGNORED crates/pull/src/landing_run_tests.rs:2113: landing: &landing,
IGNORED crates/pull/src/landing_run_tests.rs:2114: door: Door::FromWindow,
IGNORED crates/pull/src/landing_run_tests.rs:2115: encoding: spec().timestamps,
IGNORED crates/pull/src/landing_run_tests.rs:2116: });
IGNORED crates/pull/src/landing_run_tests.rs:2117: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2118: assert!(hooked.kept.is_ok());
IGNORED crates/pull/src/landing_run_tests.rs:2120: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2122: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2124: }
(F) crates/pull/src/landing_run_tests.rs:2243: panic!("{on} is not a session of record");
(F) crates/pull/src/landing_run_tests.rs:2324: dir.display()
(F) crates/pull/src/landing_run_tests.rs:2458: panic!("{hooked:?}");
(F) crates/pull/src/landing_run_tests.rs:345:9 False: gdfl.windows(2).all(|pair| pair[0] < pair[1])
(F) crates/pull/src/landing_run_tests.rs:346:16 False: && zerodha.windows(2).all(|pair| pair[0] < pair[1]),
(F) crates/pull/src/landing_run_tests.rs:394:26 False: .all(|entry| entry.mode == "import" && entry.by == "cli"),
(F) crates/pull/src/landing_run_tests.rs:681:21 False: grown.starts_with(&claims)
(F) crates/pull/src/landing_run_tests.rs:682:28 False: && grown.lines().count() == 3
(F) crates/pull/src/landing_run_tests.rs:683:28 False: && grown
(F) crates/pull/src/landing_run_tests.rs:909:13 False: said.contains(&format!("run {wider} is past {largest}"))
(F) crates/pull/src/landing_run_tests.rs:910:20 False: && said.contains("32-bit run field (offset 60)")
(F) crates/pull/src/landing_run_tests.rs:911:20 False: && said.contains("nothing is asked of the vendor"),
(F) crates/pull/src/landing_run_tests.rs:1034:9 False: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
(F) crates/pull/src/landing_run_tests.rs:1034:46 False: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
(F) crates/pull/src/landing_run_tests.rs:1423:13 False: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
(F) crates/pull/src/landing_run_tests.rs:1423:58 False: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
(F) crates/pull/src/landing_run_tests.rs:1535:67 False: matches!(hooked.kept, Err(NotKept::RunTooWide { run }) if run == u64::from(u32::MAX) + 1)
(F) crates/pull/src/landing_run_tests.rs:1690:9 False: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
(F) crates/pull/src/landing_run_tests.rs:1690:36 False: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
IGNORED crates/pull/src/landing_run_tests.rs:2020:11 True: while text.len() < 50 << 20 {
IGNORED crates/pull/src/landing_run_tests.rs:2020:11 False: while text.len() < 50 << 20 {
(F) crates/pull/src/landing_run_tests.rs:2240:9 False: let crate::calendar::DayKind::Open(session) =
(F) crates/pull/src/landing_run_tests.rs:2284:13 False: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
(F) crates/pull/src/landing_run_tests.rs:2284:25 False: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
(F) crates/pull/src/landing_run_tests.rs:2291:13 False: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
(F) crates/pull/src/landing_run_tests.rs:2291:36 False: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
(F) crates/pull/src/landing_run_tests.rs:2457:9 False: let Some(Err(why)) = &hooked.spooled else {
(F) crates/pull/src/landing_run_tests.rs:2678:13 False: assert!(february.contains(&day(2021, 2, 23)) && february.contains(&day(2021, 2, 25)));
(F) crates/pull/src/landing_run_tests.rs:2678:53 False: assert!(february.contains(&day(2021, 2, 23)) && february.contains(&day(2021, 2, 25)));
(F) crates/pull/src/padded_tests.rs:305: panic!("2024-03-02 is a recorded special session: {session:?}");
(F) crates/pull/src/padded_tests.rs:304:9 False: let DayKind::Open(s) = session else {
(U) crates/pull/src/repair_plan.rs:269: return None;
(U, type invariant) crates/pull/src/repair_plan.rs:472: break;
(U, type invariant) crates/pull/src/repair_plan.rs:573: continue;
(F) crates/pull/src/repair_plan.rs:958: other => panic!("{other:?}"),
(U) crates/pull/src/repair_plan.rs:268:12 True: if hours.kind() != SessionKind::Continuous {
(U, type invariant) crates/pull/src/repair_plan.rs:459:15 False: while let Ok(month) = at.year_month() {
(U, type invariant) crates/pull/src/repair_plan.rs:471:13 False: let Ok(next) = Day::from_days(end.days_from_epoch().saturating_add(1)) else {
(U, type invariant) crates/pull/src/repair_plan.rs:572:13 False: let Ok(day) = Day::from_days(number) else {
(F) crates/pull/src/repair_plan.rs:1008:39 False: assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
(F) crates/pull/src/repair_plan_ps2_tests.rs:124: panic!("{months:?}");
(F) crates/pull/src/repair_plan_ps2_tests.rs:123:9 False: let [planned] = months.as_slice() else {
(F) crates/pull/src/repair_plan_ps2_tests.rs:128:13 False: assert!(planned.short.is_empty() && planned.missing.is_empty());
(F) crates/pull/src/repair_plan_ps2_tests.rs:128:41 False: assert!(planned.short.is_empty() && planned.missing.is_empty());
(F) crates/pull/src/repair_plan_ps2_tests.rs:146:35 False: assert!(months.iter().all(|m| m.refused.is_none() && m.bars == 0));
(U) crates/pull/src/response_keep.rs:1136: _ => io::ErrorKind::Other,
(U) crates/pull/src/response_keep.rs:1467: Ok(None) => Err(KeepError::NotRegular { path: path.clone() }),
(U) crates/pull/src/response_keep.rs:1655: return Err(KeepError::NotRegular {
(U) crates/pull/src/response_keep.rs:1656: path: path.to_path_buf(),
(U) crates/pull/src/response_keep.rs:1657: });
(U) crates/pull/src/response_keep.rs:406:20 False: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
(U) crates/pull/src/response_keep.rs:437:16 False: if let Some(slot) = out.get_mut(124..) {
(U) crates/pull/src/response_keep.rs:587:16 False: if let Some(slot) = out.get_mut(0..8) {
(U) crates/pull/src/response_keep.rs:590:16 False: if let Some(slot) = out.get_mut(8..12) {
(U) crates/pull/src/response_keep.rs:593:16 False: if let Some(slot) = out.get_mut(12) {
(U) crates/pull/src/response_keep.rs:597:16 False: if let Some(slot) = out.get_mut(20..24) {
(U) crates/pull/src/response_keep.rs:1024:20 False: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
(U) crates/pull/src/response_keep.rs:1038:16 False: if let Some(slot) = out.get_mut(60..) {
(U) crates/pull/src/response_keep.rs:1654:9 False: let Some(file) = open_regular(path, true)? else {
(F) crates/pull/src/response_keep_tests.rs:325: panic!("{refused:?}");
(F) crates/pull/src/response_keep_tests.rs:925: matches!(
(F) crates/pull/src/response_keep_tests.rs:966: panic!("{refused:?}");
(F) crates/pull/src/response_keep_tests.rs:1605: started.elapsed()
(F) crates/pull/src/response_keep_tests.rs:2003: panic!("one further month: {outcome:?}");
(F) crates/pull/src/response_keep_tests.rs:2091: matches!(
(F) crates/pull/src/response_keep_tests.rs:2129: panic!("July holds exactly the missing pointer: {july:?}");
(F) crates/pull/src/response_keep_tests.rs:3038: panic!("segment 1 holds the one pointer: {months:?}");
(F) crates/pull/src/response_keep_tests.rs:324:9 False: let Err(KeepError::ReceiptsFull { pack: named }) = &refused else {
(F) crates/pull/src/response_keep_tests.rs:533:9 False: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
(F) crates/pull/src/response_keep_tests.rs:533:43 False: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
(F) crates/pull/src/response_keep_tests.rs:961:9 False: let Err(KeepError::LockStalled {
(F) crates/pull/src/response_keep_tests.rs:2002:9 False: let [july] = outcome.further.as_slice() else {
(F) crates/pull/src/response_keep_tests.rs:2128:9 False: let [pointer] = july.as_slice() else {
(F) crates/pull/src/response_keep_tests.rs:2465:13 False: assert!(july.receipts.exists() && !july.bodies.exists());
(F) crates/pull/src/response_keep_tests.rs:2465:39 False: assert!(july.receipts.exists() && !july.bodies.exists());
(F) crates/pull/src/response_keep_tests.rs:2747:9 False: !at.bodies.exists() && !at.refusals.exists(),
(F) crates/pull/src/response_keep_tests.rs:2747:32 False: !at.bodies.exists() && !at.refusals.exists(),
(F) crates/pull/src/response_keep_tests.rs:3037:9 False: let [pointer] = months[1].1.receipts.as_slice() else {
(F) crates/pull/src/riskfree.rs:1109: unreachable!("no gap row in this table, so nothing is carried")
(F) crates/pull/src/riskfree.rs:1595: holed.on(asked)
(F) crates/pull/src/riskfree.rs:1632:13 False: said.contains("2020-01-20") && said.contains("unexplained"),
(F) crates/pull/src/riskfree.rs:1632:44 False: said.contains("2020-01-20") && said.contains("unexplained"),
(F) crates/pull/src/riskfree.rs:1831:56 False: matches!(tails.as_slice(), [small, big] if big.1 <= small.1.saturating_mul(10).max(2_000)),
(U) crates/pull/src/zerodha_spool.rs:423: other => Self::io(at, "open", io::Error::other(other.to_string())),
(U, type invariant) crates/pull/src/zerodha_spool.rs:869: why: format!("day {}: {why}", record.day),
(U, type invariant) crates/pull/src/zerodha_spool.rs:870: })?;
(U) crates/pull/src/zerodha_spool.rs:913: continue;
(U) crates/pull/src/zerodha_spool.rs:1092: path: path.to_path_buf(),
(U) crates/pull/src/zerodha_spool.rs:1093: why: format!("its length {len} does not fit this host's memory"),
(U) crates/pull/src/zerodha_spool.rs:1094: })?;
(U) crates/pull/src/zerodha_spool.rs:912:17 False: let Some(entry) = open.get_mut(&id) else {
(U) crates/pull/src/zerodha_spool.rs:1427:16 False: && let Some(dir) = path.parent()
(F) crates/pull/src/zerodha_spool_cov_tests.rs:230: panic!("the widest detail decodes");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:262: panic!("{probed:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:288: panic!("a fresh run's lock is free");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:295: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:341: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:407: panic!("{why}: {refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:467: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:521: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:562: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:606: panic!("{probed:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:638: panic!("{refused:?}");
(F) crates/pull/src/zerodha_spool_cov_tests.rs:221:13 False: said.starts_with("record_refused: detail kind")
(F) crates/pull/src/zerodha_spool_cov_tests.rs:222:20 False: && said.contains("a spool detail is a kind above 0 and at most 128 bytes"),
(F) crates/pull/src/zerodha_spool_cov_tests.rs:229:9 False: let Ok(Decoded::Detail(_, kept)) = decode_record(&bytes[160..]) else {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:243:13 False: assert!(!root.join(SPOOL_DIR).exists() && !root.join(LIVE_DIR).exists());
(F) crates/pull/src/zerodha_spool_cov_tests.rs:243:47 False: assert!(!root.join(SPOOL_DIR).exists() && !root.join(LIVE_DIR).exists());
(F) crates/pull/src/zerodha_spool_cov_tests.rs:256:9 False: let Err(SpoolError::Io {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:273:83 False: matches!(&refused, Err(SpoolError::Io { action: "open", path: p, .. }) if *p == path),
(F) crates/pull/src/zerodha_spool_cov_tests.rs:287:9 False: let Live::Taken(held) = live_probe(&root, 7).unwrap() else {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:294:9 False: let Err(SpoolError::Busy { run, holder }) = &refused else {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:335:9 False: let Err(SpoolError::Io {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:401:13 False: let Err(SpoolError::Damaged {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:466:9 False: let Err(SpoolError::Io { action, source, .. }) = &refused else {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:474:68 False: matches!(&refused, Err(SpoolError::NotRegular { path }) if *path == dir),
(F) crates/pull/src/zerodha_spool_cov_tests.rs:520:9 False: let Err(SpoolError::Io { action, source, .. }) = &refused else {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:556:9 False: let SpoolError::Io {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:578:74 False: matches!(read_log(&dir), Err(SpoolError::NotRegular { path }) if path == dir),
(F) crates/pull/src/zerodha_spool_cov_tests.rs:582:83 False: matches!(read_spool(&dir, &id()), Err(SpoolError::NotRegular { path }) if path == dir),
(F) crates/pull/src/zerodha_spool_cov_tests.rs:600:9 False: let Err(SpoolError::Io {
(F) crates/pull/src/zerodha_spool_cov_tests.rs:632:9 False: let Err(SpoolError::Io {
(F) crates/pull/tests/broker.rs:398: bars.display()
(F) crates/pull/tests/broker.rs:766: at("1day").display()
(F, by hand) crates/pull/tests/broker.rs:923: PathSegment::Value { placeholder, .. } => {
(F) crates/pull/tests/broker.rs:924: panic!("a discovery path names a value: {placeholder}")
(F) crates/pull/tests/broker.rs:932: panic!("Groww is an HTTP broker");
(F) crates/pull/tests/broker.rs:964: panic!("Dhan is an HTTP broker");
(F) crates/pull/tests/broker.rs:1093: contract.as_str()
(F) crates/pull/tests/broker.rs:892:9 False: why.contains("1min") && why.contains("1day"),
(F) crates/pull/tests/broker.rs:892:33 False: why.contains("1min") && why.contains("1day"),
(F) crates/pull/tests/broker.rs:931:9 False: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
(F) crates/pull/tests/broker.rs:963:9 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
(F) crates/pull/tests/broker.rs:1409:13 False: assert!(bins.is_empty() && ovls.is_empty());
(F) crates/pull/tests/broker.rs:1409:32 False: assert!(bins.is_empty() && ovls.is_empty());
(F) crates/pull/tests/census_resume.rs:128: panic!("one CASH row after day one: {:?}", cash_row(root));
(F) crates/pull/tests/census_resume.rs:147: panic!("{stopped:?}");
(F) crates/pull/tests/census_resume.rs:191: panic!("one CASH row after the recount: {:?}", cash_row(root));
(F) crates/pull/tests/census_resume.rs:209: panic!("one CASH row after day three: {:?}", cash_row(root));
(F) crates/pull/tests/census_resume.rs:127:9 False: let [(rows_one, _)] = cash_row(root)[..] else {
(F) crates/pull/tests/census_resume.rs:146:9 False: let ImportRefusal::CensusRefused { day: at, .. } = &stopped else {
(F) crates/pull/tests/census_resume.rs:190:9 False: let [(rows_two, _)] = cash_row(root)[..] else {
(F) crates/pull/tests/census_resume.rs:208:9 False: let [(rows_three, _)] = cash_row(root)[..] else {
(F) crates/pull/tests/census_resume.rs:258:36 False: .is_some_and(|journal| journal.closed.is_empty() && journal.run.is_none()),
(F) crates/pull/tests/cloud_mutants_import.rs:307: other => panic!("a rotted record read as {other:?}"),
(F) crates/pull/tests/cloud_mutants_import.rs:373: other => panic!("{other:?}"),
(F) crates/pull/tests/cloud_mutants_import.rs:402: other => panic!("{other:?}"),
(F) crates/pull/tests/cloud_mutants_import.rs:867: panic!(
(F) crates/pull/tests/cloud_mutants_import.rs:869: path.display(),
(F) crates/pull/tests/cloud_mutants_import.rs:870: tried.join("; ")
(F) crates/pull/tests/cloud_mutants_import.rs:897: link.display()
(F) crates/pull/tests/cloud_mutants_import.rs:927: link.display()
(F) crates/pull/tests/cloud_mutants_import.rs:952: other => panic!("{other:?}"),
(F) crates/pull/tests/cloud_mutants_import.rs:1804: lines[0]
(F) crates/pull/tests/cloud_mutants_import.rs:335:13 False: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
(F) crates/pull/tests/cloud_mutants_import.rs:335:56 False: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
(F) crates/pull/tests/cloud_mutants_import.rs:497:9 False: said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
(F) crates/pull/tests/cloud_mutants_import.rs:498:16 False: && said.contains("Refused rather than saturated"),
(F) crates/pull/tests/cloud_mutants_import.rs:915:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F) crates/pull/tests/cloud_mutants_import.rs:1019:9 False: gap.starts_with("gap NSE-INDEX-NIFTY 2025-07-01 v=1 rung=1min owed=375 held=374 missing=1")
(F) crates/pull/tests/cloud_mutants_import.rs:1020:16 False: && gap.ends_with("why=vendor_empty run=4242"),
(F, by hand) crates/pull/tests/cloud_mutants_import.rs:1312:26 True: .any(|m| m.contains(".BE") || m.contains("ZZSHARE"))
(F) crates/pull/tests/cloud_mutants_import.rs:1382:82 False: matches!(listing.auction_for(&cash("TCS")), Auction::Unknown(why) if !why.is_empty())
(F) crates/pull/tests/cloud_mutants_import.rs:1386:38 False: Auction::Unknown(why) if why.contains("records no ISIN")
(F) crates/pull/tests/cloud_mutants_import.rs:1458:38 False: Auction::Unknown(why) if why.contains("core::universe records no ISIN")
(F) crates/pull/tests/cloud_mutants_import.rs:1802:13 False: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
(F) crates/pull/tests/cloud_mutants_import.rs:1802:56 False: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
(F) crates/pull/tests/padded_refold.rs:290: refused[0]
(F) crates/pull/tests/padded_refold.rs:350: refused[0]
(F) crates/pull/tests/padded_refold.rs:381: refused[0]
(F) crates/pull/tests/padded_refold.rs:400: journal[0]
(F) crates/pull/tests/padded_refold.rs:405: journal[0]
(F) crates/pull/tests/padded_refold.rs:446: refused[0]
(F) crates/pull/tests/padded_refold.rs:459: refused[0]
(F) crates/pull/tests/padded_refold.rs:464: refused[0]
(F) crates/pull/tests/padded_refold.rs:474: journal[0]
(F) crates/pull/tests/padded_refold.rs:516: refused[0]
(F) crates/pull/tests/padded_refold.rs:522: refused[0]
(F) crates/pull/tests/padded_refold.rs:578: refused[0]
(F) crates/pull/tests/padded_refold.rs:598: refused[0]
(F) crates/pull/tests/padded_refold.rs:620: refused[0]
IGNORED crates/pull/tests/store_evidence.rs:30: fn store_root() -> PathBuf {
IGNORED crates/pull/tests/store_evidence.rs:31: PathBuf::from(
IGNORED crates/pull/tests/store_evidence.rs:32: std::env::var_os("BRUTEX_EVIDENCE_STORE")
IGNORED crates/pull/tests/store_evidence.rs:33: .expect("BRUTEX_EVIDENCE_STORE names the store root to read"),
IGNORED crates/pull/tests/store_evidence.rs:35: }
IGNORED crates/pull/tests/store_evidence.rs:38: fn read_month(
IGNORED crates/pull/tests/store_evidence.rs:39: root: &Path,
IGNORED crates/pull/tests/store_evidence.rs:40: segment: &str,
IGNORED crates/pull/tests/store_evidence.rs:41: symbol: &str,
IGNORED crates/pull/tests/store_evidence.rs:42: tf: Timeframe,
IGNORED crates/pull/tests/store_evidence.rs:43: month: YearMonth,
IGNORED crates/pull/tests/store_evidence.rs:44: ) -> Vec<store::format::Bar> {
IGNORED crates/pull/tests/store_evidence.rs:45: let path = StorePath::new(PathParts {
IGNORED crates/pull/tests/store_evidence.rs:46: vendor: Vendor::Zerodha,
IGNORED crates/pull/tests/store_evidence.rs:47: exchange: "NSE",
IGNORED crates/pull/tests/store_evidence.rs:48: segment,
IGNORED crates/pull/tests/store_evidence.rs:49: symbol,
IGNORED crates/pull/tests/store_evidence.rs:50: contract: None,
IGNORED crates/pull/tests/store_evidence.rs:51: timeframe: tf,
IGNORED crates/pull/tests/store_evidence.rs:52: month,
IGNORED crates/pull/tests/store_evidence.rs:53: file: FileKind::Bars,
IGNORED crates/pull/tests/store_evidence.rs:54: })
IGNORED crates/pull/tests/store_evidence.rs:55: .unwrap();
IGNORED crates/pull/tests/store_evidence.rs:56: let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:57: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:58: return Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:60: (0..file.header().n_valid)
IGNORED crates/pull/tests/store_evidence.rs:61: .map(|i| file.read_record(i).unwrap())
IGNORED crates/pull/tests/store_evidence.rs:62: .collect()
IGNORED crates/pull/tests/store_evidence.rs:63: }
IGNORED crates/pull/tests/store_evidence.rs:66: fn on_day(day: Day, bars: &[store::format::Bar]) -> usize {
IGNORED crates/pull/tests/store_evidence.rs:67: let midnight = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
IGNORED crates/pull/tests/store_evidence.rs:68: bars.iter()
IGNORED crates/pull/tests/store_evidence.rs:69: .filter(|b| (midnight..midnight + 86_400_000_000).contains(&b.ts_micros))
IGNORED crates/pull/tests/store_evidence.rs:70: .count()
IGNORED crates/pull/tests/store_evidence.rs:71: }
IGNORED crates/pull/tests/store_evidence.rs:86: fn replay(root: &Path, segment: &str, symbol: &str, month: YearMonth) -> Replay {
IGNORED crates/pull/tests/store_evidence.rs:87: let minutes = read_month(root, segment, symbol, Timeframe::MINUTE_1, month);
IGNORED crates/pull/tests/store_evidence.rs:88: assert!(
IGNORED crates/pull/tests/store_evidence.rs:89: !minutes.is_empty(),
IGNORED crates/pull/tests/store_evidence.rs:92: let rows: Vec<pull::fetch::RawRow> = minutes
IGNORED crates/pull/tests/store_evidence.rs:93: .iter()
IGNORED crates/pull/tests/store_evidence.rs:94: .map(|b| pull::fetch::RawRow {
IGNORED crates/pull/tests/store_evidence.rs:95: timestamp: b.ts_micros.div_euclid(1_000_000),
IGNORED crates/pull/tests/store_evidence.rs:96: open: b.open,
IGNORED crates/pull/tests/store_evidence.rs:97: high: b.high,
IGNORED crates/pull/tests/store_evidence.rs:98: low: b.low,
IGNORED crates/pull/tests/store_evidence.rs:99: close: b.close,
IGNORED crates/pull/tests/store_evidence.rs:100: volume: b.volume,
IGNORED crates/pull/tests/store_evidence.rs:101: open_interest: None,
IGNORED crates/pull/tests/store_evidence.rs:102: })
IGNORED crates/pull/tests/store_evidence.rs:103: .collect();
IGNORED crates/pull/tests/store_evidence.rs:104: let first = Day::new(month.year(), month.month(), 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:105: let request = pull::fetch::BarRequest {
IGNORED crates/pull/tests/store_evidence.rs:106: instrument_id: String::new(),
IGNORED crates/pull/tests/store_evidence.rs:107: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:108: pull::vendor::Listing::Index
IGNORED crates/pull/tests/store_evidence.rs:110: pull::vendor::Listing::Equity
IGNORED crates/pull/tests/store_evidence.rs:112: window: Window::new(first, first.end_of_month()).unwrap(),
IGNORED crates/pull/tests/store_evidence.rs:113: granularity: pull::vendor::Granularity::Minute1,
IGNORED crates/pull/tests/store_evidence.rs:115: let plan = pull::ingest::Plan {
IGNORED crates/pull/tests/store_evidence.rs:116: calendar: pull::calendar::Runtime::default(),
IGNORED crates/pull/tests/store_evidence.rs:117: cash_schedule: None,
IGNORED crates/pull/tests/store_evidence.rs:118: columns: pull::csv::Columns::Gdfl,
IGNORED crates/pull/tests/store_evidence.rs:119: request: &request,
IGNORED crates/pull/tests/store_evidence.rs:120: encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
IGNORED crates/pull/tests/store_evidence.rs:121: scale: pull::vendor::PriceScale::Paisa,
IGNORED crates/pull/tests/store_evidence.rs:122: vendor: Vendor::Zerodha,
IGNORED crates/pull/tests/store_evidence.rs:123: exchange: "NSE",
IGNORED crates/pull/tests/store_evidence.rs:124: segment,
IGNORED crates/pull/tests/store_evidence.rs:125: contract: None,
IGNORED crates/pull/tests/store_evidence.rs:126: run: 0,
IGNORED crates/pull/tests/store_evidence.rs:127: };
IGNORED crates/pull/tests/store_evidence.rs:128: let scratch = std::env::temp_dir().join(format!(
IGNORED crates/pull/tests/store_evidence.rs:129: "brutex-EVIDENCE-{symbol}-{month}-{}",
IGNORED crates/pull/tests/store_evidence.rs:130: std::process::id()
IGNORED crates/pull/tests/store_evidence.rs:131: ));
IGNORED crates/pull/tests/store_evidence.rs:132: let _fresh = fs::remove_dir_all(&scratch);
IGNORED crates/pull/tests/store_evidence.rs:133: fs::create_dir_all(&scratch).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:134: let done = pull::ingest::from_window(
IGNORED crates/pull/tests/store_evidence.rs:135: &pull::fetch::RawWindow { rows },
IGNORED crates/pull/tests/store_evidence.rs:136: symbol,
IGNORED crates/pull/tests/store_evidence.rs:137: "STORE EVIDENCE",
IGNORED crates/pull/tests/store_evidence.rs:138: &scratch,
IGNORED crates/pull/tests/store_evidence.rs:139: plan,
IGNORED crates/pull/tests/store_evidence.rs:141: let ledger = gap_ledger(&scratch);
IGNORED crates/pull/tests/store_evidence.rs:142: Replay {
IGNORED crates/pull/tests/store_evidence.rs:143: scratch,
IGNORED crates/pull/tests/store_evidence.rs:144: done,
IGNORED crates/pull/tests/store_evidence.rs:145: ledger,
IGNORED crates/pull/tests/store_evidence.rs:146: }
IGNORED crates/pull/tests/store_evidence.rs:147: }
IGNORED crates/pull/tests/store_evidence.rs:223: fn owed_days(month: YearMonth, minutes: &[store::format::Bar]) -> Vec<(Day, Owed)> {
IGNORED crates/pull/tests/store_evidence.rs:224: let first = Day::new(month.year(), month.month(), 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:225: let calendar = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:226: let mut out = Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:227: let mut day = first;
IGNORED crates/pull/tests/store_evidence.rs:228: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:229: let at = i64::from(day.days_from_epoch());
IGNORED crates/pull/tests/store_evidence.rs:230: let stored = on_day(day, minutes);
IGNORED crates/pull/tests/store_evidence.rs:231: match pull::session::with_recorded_window(at, calendar.kind_of(at)) {
IGNORED crates/pull/tests/store_evidence.rs:232: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:233: out.push((
IGNORED crates/pull/tests/store_evidence.rs:234: day,
IGNORED crates/pull/tests/store_evidence.rs:235: Owed::Short {
IGNORED crates/pull/tests/store_evidence.rs:236: owed: session.bars(),
IGNORED crates/pull/tests/store_evidence.rs:237: stored,
IGNORED crates/pull/tests/store_evidence.rs:238: },
IGNORED crates/pull/tests/store_evidence.rs:239: ));
IGNORED crates/pull/tests/store_evidence.rs:240: }
IGNORED crates/pull/tests/store_evidence.rs:241: pull::calendar::DayKind::OpenLengthUnmeasured => {
IGNORED crates/pull/tests/store_evidence.rs:242: out.push((day, Owed::Unsized { stored }));
IGNORED crates/pull/tests/store_evidence.rs:243: }
IGNORED crates/pull/tests/store_evidence.rs:246: | pull::calendar::DayKind::Unmeasured => {}
IGNORED crates/pull/tests/store_evidence.rs:248: day = day.succ().unwrap();
IGNORED crates/pull/tests/store_evidence.rs:250: out
IGNORED crates/pull/tests/store_evidence.rs:251: }
IGNORED crates/pull/tests/store_evidence.rs:254: fn gap_lines<'a>(ledger: &'a str, key: &str, day: Day) -> Vec<&'a str> {
IGNORED crates/pull/tests/store_evidence.rs:255: let prefix = format!("gap {key} {day} ");
IGNORED crates/pull/tests/store_evidence.rs:256: ledger
IGNORED crates/pull/tests/store_evidence.rs:257: .lines()
IGNORED crates/pull/tests/store_evidence.rs:258: .filter(|line| line.starts_with(&prefix))
IGNORED crates/pull/tests/store_evidence.rs:259: .collect()
IGNORED crates/pull/tests/store_evidence.rs:260: }
IGNORED crates/pull/tests/store_evidence.rs:262: fn derive_line<'a>(ledger: &'a str, rung: &str) -> &'a str {
IGNORED crates/pull/tests/store_evidence.rs:263: ledger
IGNORED crates/pull/tests/store_evidence.rs:264: .lines()
IGNORED crates/pull/tests/store_evidence.rs:265: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:266: .unwrap_or("(no derive line)")
IGNORED crates/pull/tests/store_evidence.rs:267: }
IGNORED crates/pull/tests/store_evidence.rs:272: fn banknifty_2024_03_02_derives_from_its_stored_minutes() {
IGNORED crates/pull/tests/store_evidence.rs:273: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:274: let month = YearMonth::new(2024, 3).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:275: let saturday = Day::new(2024, 3, 2).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:276: let stored_minutes = read_month(&root, "INDEX", "BANKNIFTY", Timeframe::MINUTE_1, month);
IGNORED crates/pull/tests/store_evidence.rs:277: let replay = replay(&root, "INDEX", "BANKNIFTY", month);
IGNORED crates/pull/tests/store_evidence.rs:278: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:280: on_day(saturday, &stored_minutes)
IGNORED crates/pull/tests/store_evidence.rs:282: for tf in [
IGNORED crates/pull/tests/store_evidence.rs:283: Timeframe::MINUTE_2,
IGNORED crates/pull/tests/store_evidence.rs:284: Timeframe::MINUTE_5,
IGNORED crates/pull/tests/store_evidence.rs:285: Timeframe::MINUTE_15,
IGNORED crates/pull/tests/store_evidence.rs:286: Timeframe::MINUTE_60,
IGNORED crates/pull/tests/store_evidence.rs:287: ] {
IGNORED crates/pull/tests/store_evidence.rs:288: let stored = read_month(&root, "INDEX", "BANKNIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:289: let derived = read_month(&replay.scratch, "INDEX", "BANKNIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:290: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:291: "BANKNIFTY 2024-03 {}: stored {} bars ({} on 03-02); this build derives {} ({} on 03-02)",
IGNORED crates/pull/tests/store_evidence.rs:292: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:293: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:294: on_day(saturday, &stored),
IGNORED crates/pull/tests/store_evidence.rs:295: derived.len(),
IGNORED crates/pull/tests/store_evidence.rs:296: on_day(saturday, &derived)
IGNORED crates/pull/tests/store_evidence.rs:297: );
IGNORED crates/pull/tests/store_evidence.rs:298: }
IGNORED crates/pull/tests/store_evidence.rs:299: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:300: let derived = read_month(
IGNORED crates/pull/tests/store_evidence.rs:301: &replay.scratch,
IGNORED crates/pull/tests/store_evidence.rs:302: "INDEX",
IGNORED crates/pull/tests/store_evidence.rs:303: "BANKNIFTY",
IGNORED crates/pull/tests/store_evidence.rs:305: month,
IGNORED crates/pull/tests/store_evidence.rs:307: assert_eq!(on_day(saturday, &stored_minutes), 105);
IGNORED crates/pull/tests/store_evidence.rs:308: assert_eq!(on_day(saturday, &derived), 21);
IGNORED crates/pull/tests/store_evidence.rs:309: }
IGNORED crates/pull/tests/store_evidence.rs:315: fn nifty_2026_09_derives_every_session() {
IGNORED crates/pull/tests/store_evidence.rs:316: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:317: let month = YearMonth::new(2026, 9).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:318: let replay = replay(&root, "INDEX", "NIFTY", month);
IGNORED crates/pull/tests/store_evidence.rs:319: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
IGNORED crates/pull/tests/store_evidence.rs:320: let stored = read_month(&root, "INDEX", "NIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:321: let derived = read_month(&replay.scratch, "INDEX", "NIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:322: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:324: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:325: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:326: stored.last().map_or(0, |b| b.close),
IGNORED crates/pull/tests/store_evidence.rs:327: derived.len(),
IGNORED crates/pull/tests/store_evidence.rs:328: derived.last().map_or(0, |b| b.close)
IGNORED crates/pull/tests/store_evidence.rs:331: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:332: eprintln!("failures: {}", replay.done.failures.len());
IGNORED crates/pull/tests/store_evidence.rs:333: for failure in replay.done.failures.iter().take(5) {
IGNORED crates/pull/tests/store_evidence.rs:334: eprintln!("  {}", failure.why.chars().take(240).collect::<String>());
IGNORED crates/pull/tests/store_evidence.rs:335: }
IGNORED crates/pull/tests/store_evidence.rs:336: assert!(derive_line(&replay.ledger, "5min").contains(" days=21 "));
IGNORED crates/pull/tests/store_evidence.rs:337: }
IGNORED crates/pull/tests/store_evidence.rs:349: fn the_gap_ledger_names_each_short_day_of_a_stored_month() {
IGNORED crates/pull/tests/store_evidence.rs:350: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:351: let calendar = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:352: let mut checked = 0_usize;
IGNORED crates/pull/tests/store_evidence.rs:353: let mut special = Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:354: for (symbol, year, month) in [("NIFTY", 2020, 11), ("NIFTY", 2024, 11), ("NIFTY", 2021, 2)] {
IGNORED crates/pull/tests/store_evidence.rs:355: let ym = YearMonth::new(year, month).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:356: let minutes = read_month(&root, "INDEX", symbol, Timeframe::MINUTE_1, ym);
IGNORED crates/pull/tests/store_evidence.rs:357: let replay = replay(&root, "INDEX", symbol, ym);
IGNORED crates/pull/tests/store_evidence.rs:358: let key = format!("NSE-INDEX-{symbol}");
IGNORED crates/pull/tests/store_evidence.rs:359: let owed = owed_days(ym, &minutes);
IGNORED crates/pull/tests/store_evidence.rs:360: let gaps = replay
IGNORED crates/pull/tests/store_evidence.rs:361: .ledger
IGNORED crates/pull/tests/store_evidence.rs:362: .lines()
IGNORED crates/pull/tests/store_evidence.rs:363: .filter(|line| line.starts_with(&format!("gap {key} ")))
IGNORED crates/pull/tests/store_evidence.rs:364: .count();
IGNORED crates/pull/tests/store_evidence.rs:365: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:367: owed.len(),
IGNORED crates/pull/tests/store_evidence.rs:368: replay.done.failures.len()
IGNORED crates/pull/tests/store_evidence.rs:370: for (day, what) in &owed {
IGNORED crates/pull/tests/store_evidence.rs:371: let named = gap_lines(&replay.ledger, &key, *day);
IGNORED crates/pull/tests/store_evidence.rs:372: eprintln!("  {day} {what:?}: {named:?}");
IGNORED crates/pull/tests/store_evidence.rs:373: assert_eq!(
IGNORED crates/pull/tests/store_evidence.rs:374: named.len(),
IGNORED crates/pull/tests/store_evidence.rs:377: named.len()
IGNORED crates/pull/tests/store_evidence.rs:379: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:380: assert!(
IGNORED crates/pull/tests/store_evidence.rs:381: named[0].contains(" why=withheld_unmeasured_session "),
IGNORED crates/pull/tests/store_evidence.rs:383: named[0]
IGNORED crates/pull/tests/store_evidence.rs:385: }
IGNORED crates/pull/tests/store_evidence.rs:386: let at = i64::from(day.days_from_epoch());
IGNORED crates/pull/tests/store_evidence.rs:387: let regular = matches!(
IGNORED crates/pull/tests/store_evidence.rs:388: calendar.kind_of(at),
IGNORED crates/pull/tests/store_evidence.rs:389: pull::calendar::DayKind::Open(session)
IGNORED crates/pull/tests/store_evidence.rs:390: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:392: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:393: special.push(*day);
IGNORED crates/pull/tests/store_evidence.rs:394: }
IGNORED crates/pull/tests/store_evidence.rs:395: checked += 1;
IGNORED crates/pull/tests/store_evidence.rs:399: let mut day = Day::new(year, month, 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:400: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:401: let named = gap_lines(&replay.ledger, &key, day);
IGNORED crates/pull/tests/store_evidence.rs:402: let expected = usize::from(owed.iter().any(|(owed_day, _)| *owed_day == day));
IGNORED crates/pull/tests/store_evidence.rs:403: assert_eq!(
IGNORED crates/pull/tests/store_evidence.rs:404: named.len(),
IGNORED crates/pull/tests/store_evidence.rs:408: named.len()
IGNORED crates/pull/tests/store_evidence.rs:410: day = day.succ().unwrap();
IGNORED crates/pull/tests/store_evidence.rs:412: eprintln!("  {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:417: eprintln!("{checked} day(s) checked; special sessions among them: {special:?}");
IGNORED crates/pull/tests/store_evidence.rs:418: assert!(checked > 0, "no owed day in three months chosen for theirs");
IGNORED crates/pull/tests/store_evidence.rs:419: for year in [2020, 2024] {
IGNORED crates/pull/tests/store_evidence.rs:420: assert!(
IGNORED crates/pull/tests/store_evidence.rs:421: special
IGNORED crates/pull/tests/store_evidence.rs:422: .iter()
IGNORED crates/pull/tests/store_evidence.rs:423: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:427: }
IGNORED crates/pull/tests/store_evidence.rs:432: fn an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets() {
IGNORED crates/pull/tests/store_evidence.rs:433: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:434: let month = YearMonth::new(2023, 2).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:435: let replay = replay(&root, "CASH", "360ONE", month);
IGNORED crates/pull/tests/store_evidence.rs:436: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
IGNORED crates/pull/tests/store_evidence.rs:437: let stored = read_month(&root, "CASH", "360ONE", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:438: let derived = read_month(&replay.scratch, "CASH", "360ONE", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:439: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:440: "360ONE 2023-02 {}: stored {} bars; this build derives {}",
IGNORED crates/pull/tests/store_evidence.rs:441: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:442: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:443: derived.len()
IGNORED crates/pull/tests/store_evidence.rs:444: );
IGNORED crates/pull/tests/store_evidence.rs:445: eprintln!("  {}", derive_line(&replay.ledger, tf.as_str()));
IGNORED crates/pull/tests/store_evidence.rs:446: }
IGNORED crates/pull/tests/store_evidence.rs:447: let gaps = replay
IGNORED crates/pull/tests/store_evidence.rs:448: .ledger
IGNORED crates/pull/tests/store_evidence.rs:449: .lines()
IGNORED crates/pull/tests/store_evidence.rs:450: .filter(|l| l.starts_with("gap "))
IGNORED crates/pull/tests/store_evidence.rs:451: .count();
IGNORED crates/pull/tests/store_evidence.rs:452: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:454: replay.done.failures.len()
IGNORED crates/pull/tests/store_evidence.rs:456: let stored = read_month(&root, "CASH", "360ONE", Timeframe::MINUTE_60, month);
IGNORED crates/pull/tests/store_evidence.rs:457: let derived = read_month(
IGNORED crates/pull/tests/store_evidence.rs:458: &replay.scratch,
IGNORED crates/pull/tests/store_evidence.rs:459: "CASH",
IGNORED crates/pull/tests/store_evidence.rs:460: "360ONE",
IGNORED crates/pull/tests/store_evidence.rs:462: month,
IGNORED crates/pull/tests/store_evidence.rs:464: assert!(derived.len() > stored.len());
IGNORED crates/pull/tests/store_evidence.rs:465: }
IGNORED crates/pull/tests/store_evidence.rs:473: fn the_derive_fold_costs_per_month() {
IGNORED crates/pull/tests/store_evidence.rs:474: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:475: for (segment, symbol, year, month) in [("INDEX", "NIFTY", 2026, 9), ("CASH", "360ONE", 2023, 2)]
IGNORED crates/pull/tests/store_evidence.rs:477: let ym = YearMonth::new(year, month).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:478: let minutes = read_month(&root, segment, symbol, Timeframe::MINUTE_1, ym);
IGNORED crates/pull/tests/store_evidence.rs:479: let venue = if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:480: pull::vendor::Venue::NseIndex
IGNORED crates/pull/tests/store_evidence.rs:482: pull::vendor::Venue::NseCash
IGNORED crates/pull/tests/store_evidence.rs:484: let bucket = pull::fold::Bucket::of_secs(300).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:485: let runtime = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:486: let mut aggregated = Vec::with_capacity(200);
IGNORED crates/pull/tests/store_evidence.rs:487: let mut detailed = Vec::with_capacity(200);
IGNORED crates/pull/tests/store_evidence.rs:488: for _ in 0..200 {
IGNORED crates/pull/tests/store_evidence.rs:489: let start = std::time::Instant::now();
IGNORED crates/pull/tests/store_evidence.rs:490: let derived =
IGNORED crates/pull/tests/store_evidence.rs:491: pull::fold::derive_minutes(&minutes, bucket, venue, None, runtime).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:492: aggregated.push(start.elapsed());
IGNORED crates/pull/tests/store_evidence.rs:493: std::hint::black_box(derived);
IGNORED crates/pull/tests/store_evidence.rs:494: let start = std::time::Instant::now();
IGNORED crates/pull/tests/store_evidence.rs:495: let folded =
IGNORED crates/pull/tests/store_evidence.rs:496: pull::fold::complete_minutes_with_calendar(&minutes, bucket, venue, None, runtime)
IGNORED crates/pull/tests/store_evidence.rs:497: .unwrap();
IGNORED crates/pull/tests/store_evidence.rs:498: detailed.push(start.elapsed());
IGNORED crates/pull/tests/store_evidence.rs:499: std::hint::black_box(folded);
IGNORED crates/pull/tests/store_evidence.rs:500: }
IGNORED crates/pull/tests/store_evidence.rs:501: aggregated.sort_unstable();
IGNORED crates/pull/tests/store_evidence.rs:502: detailed.sort_unstable();
IGNORED crates/pull/tests/store_evidence.rs:503: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:505: minutes.len(),
IGNORED crates/pull/tests/store_evidence.rs:506: aggregated[99],
IGNORED crates/pull/tests/store_evidence.rs:507: aggregated[197],
IGNORED crates/pull/tests/store_evidence.rs:508: detailed[99],
IGNORED crates/pull/tests/store_evidence.rs:509: detailed[197]
IGNORED crates/pull/tests/store_evidence.rs:511: assert!(!minutes.is_empty());
IGNORED crates/pull/tests/store_evidence.rs:513: }
IGNORED crates/pull/tests/store_evidence.rs:57:9 True: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:57:9 False: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:107:21 True: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:107:21 False: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:228:11 True: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:228:11 False: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:232:55 True: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:232:55 False: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:265:22 True: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:265:22 False: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:379:20 True: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:379:20 False: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:390:24 True: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:390:24 False: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:392:16 True: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:392:16 False: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:400:15 True: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:400:15 False: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:423:28 True: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:423:28 False: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:479:24 True: let venue = if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:479:24 False: let venue = if segment == "INDEX" {
(F) crates/pull/tests/zerodha_gaps.rs:225: gaps[0].why
(F) crates/pull/tests/zerodha_gaps.rs:235: lines[0]
(F) crates/pull/tests/zerodha_gaps.rs:543: other => panic!("2021-02-24 is a session of record: {other:?}"),
(F) crates/pull/tests/zerodha_gaps.rs:651: panic!("a regular cash session owes one window");
(F) crates/pull/tests/zerodha_gaps.rs:715: series.lines[0]
(F) crates/pull/tests/zerodha_gaps.rs:824: other => panic!("{other:?}"),
(F) crates/pull/tests/zerodha_gaps.rs:599:17 False: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
(F) crates/pull/tests/zerodha_gaps.rs:599:44 False: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
(F) crates/pull/tests/zerodha_gaps.rs:605:13 False: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:605:46 False: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:610:13 False: assert!(absent.short.is_empty() && !absent.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:610:40 False: assert!(absent.short.is_empty() && !absent.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:630:22 False: .all(|m| m.short.is_empty() && m.length_unjudged.is_empty())
(F) crates/pull/tests/zerodha_gaps.rs:650:9 False: let Owes::Windows((windows, 1)) = repair_plan::owed("CASH", from.days_from_epoch()) else {
(F) crates/pull/tests/zerodha_gaps.rs:712:9 False: series.lines[0].starts_with("gap NSE-CASH-INVENTEDSHR 2026-08-06 v=1 rung=1min owed=360 held=359 missing=1 first_missing=11:00 ")
(F) crates/pull/tests/zerodha_gaps.rs:713:16 False: && series.lines[0].ends_with(" fill=interior source=store absence=may_be_no_trade"),
(F) crates/pull/tests/zerodha_gaps.rs:833:13 False: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
(F) crates/pull/tests/zerodha_gaps.rs:833:52 False: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
(F) crates/pull/tests/zerodha_gaps.rs:876:9 False: second.lines.is_empty() && second.tally.already == 5,
(F) crates/pull/tests/zerodha_gaps.rs:876:36 False: second.lines.is_empty() && second.tally.already == 5,
(F) crates/pull/tests/zerodha_gaps.rs:958:9 False: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
(F) crates/pull/tests/zerodha_gaps.rs:958:62 False: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
(F) crates/pull/tests/zerodha_gaps.rs:991:13 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:991:37 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:991:63 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
(F) crates/pull/tests/zerodha_gaps.rs:1144:13 False: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
(F) crates/pull/tests/zerodha_gaps.rs:1144:32 False: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
(F) crates/pull/tests/zerodha_gaps.rs:1348:22 False: .all(|l| l.contains(" why=stored_short ") && l.ends_with(" source=store")),
(F) crates/pull/tests/zerodha_identity.rs:304: rung.as_str()
(F) crates/pull/tests/zerodha_identity.rs:560: Err(why) => panic!("{symbol} {}: {why}", tf.as_str()),
(F) crates/pull/tests/zerodha_identity.rs:717: rung.as_str()
(F) crates/pull/tests/zerodha_identity.rs:836: named[0]
(F) crates/pull/tests/zerodha_identity.rs:1755: lines[0]
(F) crates/pull/tests/zerodha_identity.rs:1764: lines[0]
(F) crates/pull/tests/zerodha_identity.rs:1769: lines[0]
(F) crates/pull/tests/zerodha_identity.rs:1810: identity_lines(sink, "PATANJALI")
(F) crates/pull/tests/zerodha_identity.rs:1957: named[0]
(F) crates/pull/tests/zerodha_identity.rs:1966: said[0]
(F) crates/pull/tests/zerodha_identity.rs:1971: said[0]
(F) crates/pull/tests/zerodha_identity.rs:1976: said[0]
(F) crates/pull/tests/zerodha_identity.rs:288:13 False: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
(F) crates/pull/tests/zerodha_identity.rs:288:37 False: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
(F) crates/pull/tests/zerodha_identity.rs:302:30 False: .all(|b| b.high <= LEVEL + 2_000 && b.low >= LEVEL - 1_000),
(F) crates/pull/tests/zerodha_identity.rs:1495:22 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
(F) crates/pull/tests/zerodha_identity.rs:1495:36 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
(F) crates/pull/tests/zerodha_identity.rs:1495:50 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
(F) crates/pull/tests/zerodha_identity.rs:1862:22 False: .any(|l| l.contains("the day rung would not open")
(F) crates/pull/tests/zerodha_identity.rs:1863:20 False: && l.contains("\"why\":")
(F) crates/pull/tests/zerodha_repair.rs:287: lines[0]
(F) crates/pull/tests/zerodha_repair.rs:355: front[0]
(F) crates/pull/tests/zerodha_repair.rs:405: lines[11]
(F) crates/pull/tests/zerodha_repair.rs:340:22 False: .any(|f| f.why.contains("2020-12:") && f.why.contains("this month only")),
`````

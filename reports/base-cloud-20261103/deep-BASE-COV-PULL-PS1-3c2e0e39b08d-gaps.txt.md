# deep-BASE-COV-PULL-PS1-3c2e0e39b08d-gaps.txt.md

Written by BASE-COV-PULL-PS1 from Route B run 9 on the code tip `3c2e0e39b08d0ef1a465d7630f19beda3e1c22c2` (the kit's `d3-route-b-cloud.sh`, pinned nightly; 5 Nov 2026): the per-file table of crate `pull` and every uncovered line and side, in the kit's gaps-file shape. One fenced block below holds it.

`````text
BASE-COV-PULL-PS1 run 9: Route B (kit d3-route-b-cloud.sh) on 3c2e0e39b08d0ef1a465d7630f19beda3e1c22c2, reading whole:pull, set = the kit f98a899398f6 set plus the 23 files step (2) added (229 coverage lines); source routeb-3c2e0e39-9.log (ROUTE-B DONE 2026-11-05 11:00:55 IST). Per-file table, then every uncovered line and side with its text. Not classified here: G1's rows are classified in base-cov-pull-ps1-fu.txt.md; every other row is G2's, G3's or another group's.
crates/pull/src/alerts.rs lines 964/979, branches 151/166
crates/pull/src/alerts_tests.rs lines 1605/1695, branches 55/94
crates/pull/src/archive.rs lines 472/480, branches 28/32
crates/pull/src/audit_ledger.rs lines 50/50, branches 10/10
crates/pull/src/audit_ledger_tests.rs lines 148/148, branches 0/0
crates/pull/src/base_cov_codes_tests.rs lines 423/423, branches 0/0
crates/pull/src/base_cov_words_tests.rs lines 358/359, branches 1/2
crates/pull/src/calendar.rs lines 2551/2610, branches 330/370
crates/pull/src/calendar_cov_tests.rs lines 27/27, branches 0/0
crates/pull/src/calendar_tradable_tests.rs lines 792/793, branches 37/50
crates/pull/src/capture.rs lines 340/340, branches 8/10
crates/pull/src/capture_ps1_tests.rs lines 58/58, branches 0/0
crates/pull/src/cash_auction.rs lines 870/927, branches 98/100
crates/pull/src/cash_auction_ps1_tests.rs lines 18/18, branches 0/0
crates/pull/src/cash_session_cache.rs lines 1455/1615, branches 75/110
crates/pull/src/cash_session_cache_lock_tests.rs lines 291/302, branches 15/22
crates/pull/src/cash_session_cache_ps1_tests.rs lines 52/54, branches 2/4
crates/pull/src/census_store.rs lines 2137/2295, branches 297/370
crates/pull/src/census_store_cov_tests.rs lines 34/34, branches 0/0
crates/pull/src/census_store_kept_tests.rs lines 665/745, branches 38/48
crates/pull/src/census_store_link_tests.rs lines 533/542, branches 8/14
crates/pull/src/census_store_tests.rs lines 1154/1196, branches 55/96
crates/pull/src/chain.rs lines 356/370, branches 23/34
crates/pull/src/config.rs lines 386/386, branches 60/60
crates/pull/src/corporate_actions.rs lines 1000/1000, branches 166/166
crates/pull/src/corporate_actions_cov_tests.rs lines 289/291, branches 9/12
crates/pull/src/corporate_actions_tests.rs lines 1279/1293, branches 52/84
crates/pull/src/csv.rs lines 1006/1009, branches 95/98
crates/pull/src/day_universe.rs lines 112/112, branches 14/22
crates/pull/src/daycheck.rs lines 339/339, branches 26/30
crates/pull/src/dhan.rs lines 118/118, branches 2/2
crates/pull/src/disk_projection.rs lines 278/278, branches 29/30
crates/pull/src/disk_projection_tests.rs lines 352/422, branches 1/8
crates/pull/src/emit_sites.rs lines 1101/1115, branches 9/14
crates/pull/src/expiry_alias.rs lines 683/689, branches 74/88
crates/pull/src/fetch.rs lines 570/584, branches 54/56
crates/pull/src/fetch_ps1_tests.rs lines 98/99, branches 1/2
crates/pull/src/fno.rs lines 595/612, branches 50/62
crates/pull/src/fno_membership.rs lines 1686/1692, branches 249/260
crates/pull/src/fno_membership_cov_tests.rs lines 71/72, branches 1/2
crates/pull/src/fno_membership_ps1_tests.rs lines 760/767, branches 22/28
crates/pull/src/fno_membership_tests.rs lines 1035/1040, branches 49/66
crates/pull/src/fnowork.rs lines 871/871, branches 62/62
crates/pull/src/fnowork_cov_tests.rs lines 201/203, branches 0/0
crates/pull/src/fnowork_futures_tests.rs lines 1132/1188, branches 23/40
crates/pull/src/fold.rs lines 1187/1200, branches 142/152
crates/pull/src/folder.rs lines 206/212, branches 16/20
crates/pull/src/gap_backfill.rs lines 222/232, branches 36/48
crates/pull/src/gaps.rs lines 714/720, branches 96/104
crates/pull/src/gaps_not_owed_tests.rs lines 486/486, branches 26/26
crates/pull/src/gdfl_archive.rs lines 1451/1520, branches 77/86
crates/pull/src/gdfl_backfill_tests.rs lines 864/870, branches 18/24
crates/pull/src/gdfl_cm.rs lines 3144/3153, branches 354/364
crates/pull/src/gdfl_cm_admission_tests.rs lines 1282/1334, branches 6/12
crates/pull/src/gdfl_cm_auction.rs lines 628/631, branches 81/82
crates/pull/src/gdfl_cm_auction_ps1_tests.rs lines 121/125, branches 2/4
crates/pull/src/gdfl_cm_auction_v2_tests.rs lines 416/421, branches 6/10
crates/pull/src/gdfl_dayzips.rs lines 201/202, branches 32/34
crates/pull/src/gdfl_dayzips_tests.rs lines 361/409, branches 0/2
crates/pull/src/gdfl_dropbox.rs lines 1970/2133, branches 367/450
crates/pull/src/gdfl_dropbox_cov_tests.rs lines 36/39, branches 5/8
crates/pull/src/gdfl_dropbox_tests.rs lines 2627/2745, branches 115/150
crates/pull/src/gdfl_fixtures.rs lines 198/198, branches 14/14
crates/pull/src/gdfl_fixtures_ps1_tests.rs lines 34/34, branches 0/0
crates/pull/src/gdfl_format.rs lines 31/32, branches 4/4
crates/pull/src/gdfl_format_change_tests.rs lines 657/658, branches 22/28
crates/pull/src/gdfl_ia16_tests.rs lines 2136/2187, branches 84/136
crates/pull/src/gdfl_ia3_tests.rs lines 660/869, branches 8/28
crates/pull/src/gdfl_ia4_tests.rs lines 192/193, branches 4/6
crates/pull/src/gdfl_import.rs lines 1769/1770, branches 214/214
crates/pull/src/gdfl_import_causes.rs lines 349/349, branches 6/6
crates/pull/src/gdfl_import_census.rs lines 132/133, branches 11/12
crates/pull/src/gdfl_import_cov_tests.rs lines 912/912, branches 16/26
crates/pull/src/gdfl_import_day.rs lines 1475/1482, branches 246/246
crates/pull/src/gdfl_import_day_ps1_tests.rs lines 1255/1294, branches 39/74
crates/pull/src/gdfl_import_filing.rs lines 875/912, branches 123/136
crates/pull/src/gdfl_import_ia2_tests.rs lines 449/551, branches 11/22
crates/pull/src/gdfl_import_journal.rs lines 775/788, branches 193/208
crates/pull/src/gdfl_import_lineage.rs lines 367/385, branches 64/82
crates/pull/src/gdfl_import_ps1_tests.rs lines 192/193, branches 3/4
crates/pull/src/gdfl_import_quality.rs lines 463/468, branches 102/110
crates/pull/src/gdfl_import_scope.rs lines 250/250, branches 14/18
crates/pull/src/gdfl_import_split_tests.rs lines 401/414, branches 34/44
crates/pull/src/gdfl_import_tests.rs lines 4683/4709, branches 201/294
crates/pull/src/gdfl_lineage.rs lines 817/822, branches 146/154
crates/pull/src/gdfl_lineage_tests.rs lines 898/1015, branches 11/26
crates/pull/src/gdfl_nfo.rs lines 920/920, branches 180/180
crates/pull/src/gdfl_nfo_attack_tests.rs lines 1264/1301, branches 142/178
crates/pull/src/gdfl_nfo_ps1_tests.rs lines 96/96, branches 2/2
crates/pull/src/gdfl_nfo_tests.rs lines 1849/2631, branches 48/188
crates/pull/src/gdfl_r2_attack_tests.rs lines 1266/1277, branches 153/172
crates/pull/src/gdfl_r3_attack_tests.rs lines 983/993, branches 125/136
crates/pull/src/gdfl_r4_attack_tests.rs lines 582/585, branches 43/54
crates/pull/src/gdfl_rekey.rs lines 1365/1479, branches 212/266
crates/pull/src/gdfl_rekey_cov_tests.rs lines 38/38, branches 0/0
crates/pull/src/gdfl_rekey_tests.rs lines 1796/2006, branches 75/148
crates/pull/src/gdfl_run.rs lines 1401/1410, branches 166/170
crates/pull/src/gdfl_run_cov_tests.rs lines 446/448, branches 1/2
crates/pull/src/gdfl_run_ia2_tests.rs lines 766/942, branches 38/74
crates/pull/src/gdfl_run_ledger.rs lines 361/394, branches 59/70
crates/pull/src/gdfl_run_ledger_cov_tests.rs lines 100/100, branches 0/0
crates/pull/src/gdfl_run_locks.rs lines 722/731, branches 155/158
crates/pull/src/gdfl_run_locks_cov_tests.rs lines 285/285, branches 5/10
crates/pull/src/gdfl_run_locks_ps1_tests.rs lines 297/302, branches 11/18
crates/pull/src/gdfl_run_scope.rs lines 56/62, branches 7/8
crates/pull/src/gdfl_run_start.rs lines 70/71, branches 5/6
crates/pull/src/gdfl_run_start_cov_tests.rs lines 90/90, branches 2/2
crates/pull/src/gdfl_run_stop.rs lines 232/232, branches 40/40
crates/pull/src/gdfl_run_stop_cov_tests.rs lines 146/147, branches 1/2
crates/pull/src/gdfl_run_tests.rs lines 1093/1252, branches 13/28
crates/pull/src/gdfl_scope_tests.rs lines 1837/1845, branches 38/54
crates/pull/src/gdfl_sealed_tests.rs lines 301/303, branches 17/24
crates/pull/src/gdfl_seconds_attack_tests.rs lines 1195/1197, branches 63/78
crates/pull/src/gdfl_series_code_tests.rs lines 363/363, branches 2/4
crates/pull/src/gdfl_tickstore.rs lines 1149/1167, branches 111/120
crates/pull/src/gdfl_v3.rs lines 302/304, branches 59/60
crates/pull/src/gdfl_v3_tests.rs lines 630/759, branches 16/46
crates/pull/src/gdfl_wp11c_tests.rs lines 402/402, branches 6/6
crates/pull/src/gdfl_wp12e_tests.rs lines 516/633, branches 9/14
crates/pull/src/groww.rs lines 41/41, branches 0/0
crates/pull/src/http.rs lines 4316/4463, branches 324/416
crates/pull/src/http_cov_tests.rs lines 74/76, branches 2/4
crates/pull/src/identity_screen.rs lines 473/474, branches 63/64
crates/pull/src/identity_screen_ps1_tests.rs lines 72/72, branches 0/0
crates/pull/src/ingest.rs lines 3100/3268, branches 243/296
crates/pull/src/ingest_cov_tests.rs lines 213/213, branches 2/4
crates/pull/src/ingest_failkind.rs lines 124/125, branches 10/10
crates/pull/src/ingest_failkind_tests.rs lines 574/574, branches 7/10
crates/pull/src/kite.rs lines 57/57, branches 0/0
crates/pull/src/kite_cov_tests.rs lines 6/6, branches 0/0
crates/pull/src/landing_run.rs lines 286/322, branches 19/26
crates/pull/src/landing_run_tests.rs lines 1479/1607, branches 52/78
crates/pull/src/lib.rs lines 5/5, branches 0/0
crates/pull/src/listing.rs lines 237/237, branches 38/38
crates/pull/src/listing_tests.rs lines 397/400, branches 3/4
crates/pull/src/manifest.rs lines 1366/1366, branches 169/170
crates/pull/src/manifest_cov_tests.rs lines 16/16, branches 0/0
crates/pull/src/manifest_ps1_tests.rs lines 156/157, branches 0/0
crates/pull/src/master_snapshots.rs lines 1677/1679, branches 263/270
crates/pull/src/master_snapshots_cov_tests.rs lines 627/628, branches 2/4
crates/pull/src/master_snapshots_first_listed.rs lines 567/569, branches 85/86
crates/pull/src/master_snapshots_first_listed_ps1_tests.rs lines 348/348, branches 2/2
crates/pull/src/master_snapshots_first_listed_tests.rs lines 423/424, branches 22/44
crates/pull/src/master_snapshots_ps1_tests.rs lines 159/159, branches 0/0
crates/pull/src/master_snapshots_tests.rs lines 1358/1514, branches 34/68
crates/pull/src/masters.rs lines 1643/1723, branches 173/214
crates/pull/src/masters_cov_tests.rs lines 92/92, branches 2/2
crates/pull/src/nse.rs lines 503/508, branches 64/78
crates/pull/src/nse_ps1_tests.rs lines 36/38, branches 2/4
crates/pull/src/nseindex.rs lines 231/231, branches 14/14
crates/pull/src/padded.rs lines 108/109, branches 27/28
crates/pull/src/padded_tests.rs lines 402/403, branches 8/10
crates/pull/src/pricing.rs lines 705/713, branches 34/52
crates/pull/src/pricing_ps1_tests.rs lines 69/71, branches 2/4
crates/pull/src/rate.rs lines 254/254, branches 26/26
crates/pull/src/reference_data.rs lines 1930/1943, branches 238/238
crates/pull/src/reference_data_cov_tests.rs lines 418/418, branches 2/4
crates/pull/src/reference_data_ps1_tests.rs lines 486/490, branches 4/8
crates/pull/src/reference_data_tests.rs lines 980/1023, branches 22/36
crates/pull/src/refusal.rs lines 114/114, branches 8/8
crates/pull/src/refusal_ps1_tests.rs lines 47/47, branches 0/0
crates/pull/src/rehearsal.rs lines 553/557, branches 97/98
crates/pull/src/rehearsal_ps1_tests.rs lines 47/47, branches 0/0
crates/pull/src/rehearsal_tests.rs lines 614/615, branches 14/18
crates/pull/src/repair_plan.rs lines 500/527, branches 52/64
crates/pull/src/repair_plan_not_owed_tests.rs lines 272/274, branches 7/10
crates/pull/src/request_minutes.rs lines 1422/1446, branches 117/142
crates/pull/src/request_minutes_ps1_tests.rs lines 66/66, branches 0/0
crates/pull/src/resolve.rs lines 610/639, branches 28/34
crates/pull/src/response_keep.rs lines 1203/1269, branches 166/194
crates/pull/src/response_keep_tests.rs lines 1788/1800, branches 27/38
crates/pull/src/riskfree.rs lines 1133/1139, branches 92/96
crates/pull/src/rolling.rs lines 815/839, branches 64/80
crates/pull/src/scrub.rs lines 430/435, branches 21/24
crates/pull/src/scrub_blocks_tests.rs lines 92/92, branches 0/0
crates/pull/src/secret.rs lines 122/122, branches 8/8
crates/pull/src/series_identity.rs lines 672/673, branches 60/64
crates/pull/src/session.rs lines 692/703, branches 108/116
crates/pull/src/ssm.rs lines 912/939, branches 86/114
crates/pull/src/ssm_cov_tests.rs lines 159/159, branches 2/2
crates/pull/src/ssm_ps1_tests.rs lines 74/74, branches 4/4
crates/pull/src/tenor.rs lines 192/196, branches 6/8
crates/pull/src/tickstore_write.rs lines 1003/1089, branches 167/188
crates/pull/src/tickstore_write_tests.rs lines 1181/1351, branches 44/68
crates/pull/src/totp.rs lines 301/306, branches 29/34
crates/pull/src/universe.rs lines 415/418, branches 12/12
crates/pull/src/upkeep.rs lines 933/934, branches 147/150
crates/pull/src/upkeep_tests.rs lines 1475/1485, branches 55/78
crates/pull/src/vendor.rs lines 2272/2293, branches 141/178
crates/pull/src/vendor_cov_tests.rs lines 74/74, branches 0/0
crates/pull/src/verdict_words.rs lines 93/94, branches 2/4
crates/pull/src/work.rs lines 129/129, branches 14/14
crates/pull/src/zerodha_spool.rs lines 758/820, branches 106/136
crates/pull/src/zerodha_spool_cov_tests.rs lines 75/75, branches 0/0
crates/pull/src/zerodha_verify.rs lines 2625/2863, branches 492/614
crates/pull/src/zerodha_verify_cov_tests.rs lines 31/31, branches 0/0
crates/pull/src/zerodha_verify_tests.rs lines 3715/3759, branches 49/78
crates/pull/tests/alerts_seams.rs lines 78/79, branches 0/0
crates/pull/tests/allocation.rs lines 385/388, branches 5/8
crates/pull/tests/anchor.rs lines 516/522, branches 24/32
crates/pull/tests/broker.rs lines 746/756, branches 24/32
crates/pull/tests/census.rs lines 777/789, branches 24/38
crates/pull/tests/census_day_pace.rs lines 87/88, branches 4/8
crates/pull/tests/census_resume.rs lines 151/155, branches 7/12
crates/pull/tests/census_v4.rs lines 549/628, branches 4/6
crates/pull/tests/cloud_mutants_import.rs lines 1168/1187, branches 16/30
crates/pull/tests/concurrency.rs lines 51/51, branches 0/0
crates/pull/tests/derive.rs lines 1396/1408, branches 39/56
crates/pull/tests/folder.rs lines 489/492, branches 13/20
crates/pull/tests/futures_events.rs lines 130/131, branches 3/6
crates/pull/tests/futures_refusal_events.rs lines 70/70, branches 0/0
crates/pull/tests/gdfl_bad_row_audit.rs lines 239/245, branches 3/6
crates/pull/tests/gdfl_census_stop.rs lines 146/151, branches 10/18
crates/pull/tests/gdfl_run_stop.rs lines 15/15, branches 0/0
crates/pull/tests/integration.rs lines 237/238, branches 0/0
crates/pull/tests/no_scheduler_delete.rs lines 295/300, branches 96/112
crates/pull/tests/padded_refold.rs lines 389/403, branches 4/4
crates/pull/tests/pipeline.rs lines 1048/1062, branches 18/28
crates/pull/tests/refusal.rs lines 310/316, branches 15/16
crates/pull/tests/request_gap_events.rs lines 105/106, branches 8/14
crates/pull/tests/sealed_month.rs lines 348/348, branches 16/20
crates/pull/tests/store_evidence.rs lines 16/324, branches 0/22
crates/pull/tests/support/mod.rs lines 32/32, branches 3/4
crates/pull/tests/tls.rs lines 14/14, branches 0/0
crates/pull/tests/token_identity.rs lines 231/231, branches 54/56
crates/pull/tests/unit.rs lines 4528/4550, branches 104/132
crates/pull/tests/zerodha_gaps.rs lines 1006/1012, branches 84/106
crates/pull/tests/zerodha_identity.rs lines 1486/1500, branches 59/68
crates/pull/tests/zerodha_repair.rs lines 473/476, branches 5/6

UNCOVERED LINE crates/pull/src/alerts.rs:723: Err(why) => {
UNCOVERED LINE crates/pull/src/alerts.rs:724: let _partial = fs::remove_file(&names.reserve);
UNCOVERED LINE crates/pull/src/alerts.rs:725: Reserve::Absent(why.kind())
UNCOVERED LINE crates/pull/src/alerts.rs:816: return Err(AlertError::NoSuchAlert { ordinal: 0 });
UNCOVERED LINE crates/pull/src/alerts.rs:1159: return Err(AlertError::NoSuchAlert { ordinal });
UNCOVERED LINE crates/pull/src/alerts.rs:1254: const fn seam_between_reads(_root: &Path) {}
UNCOVERED LINE crates/pull/src/alerts.rs:1300: return Ok(None);
UNCOVERED LINE crates/pull/src/alerts.rs:1595: return Err(AlertError::Io {
UNCOVERED LINE crates/pull/src/alerts.rs:1596: op: "open the kept file",
UNCOVERED LINE crates/pull/src/alerts.rs:1597: kind: io::ErrorKind::NotFound,
UNCOVERED LINE crates/pull/src/alerts.rs:1598: text: path.display().to_string(),
UNCOVERED LINE crates/pull/src/alerts.rs:1599: });
UNCOVERED LINE crates/pull/src/alerts.rs:1822: let why = AlertError::io("put the new queue in place", &why);
UNCOVERED LINE crates/pull/src/alerts.rs:1823: note_not_saved(&alert, &why);
UNCOVERED LINE crates/pull/src/alerts.rs:1824: return Err(why);
UNCOVERED BRANCH crates/pull/src/alerts.rs:370:12 True: if bytes.len() != 128 || crc32c(body) != le_u32(bytes, 124) {
UNCOVERED BRANCH crates/pull/src/alerts.rs:573:19 True: if !crc_ok || head.get(..8) != Some(MAGIC.as_slice()) {
UNCOVERED BRANCH crates/pull/src/alerts.rs:655:21 False: Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
UNCOVERED BRANCH crates/pull/src/alerts.rs:815:16 True: if !create {
UNCOVERED BRANCH crates/pull/src/alerts.rs:881:20 False: if let Slot::Good(r) = slot
UNCOVERED BRANCH crates/pull/src/alerts.rs:981:19 False: } else if record.kind_of_record == TYPE_ALERT
UNCOVERED BRANCH crates/pull/src/alerts.rs:1109:19 True: flags: if text_cut { FLAG_TEXT_CUT } else { 0 },
UNCOVERED BRANCH crates/pull/src/alerts.rs:1158:12 True: if end == 0 {
UNCOVERED BRANCH crates/pull/src/alerts.rs:1299:12 True: if !regular(&names.queue)? {
UNCOVERED BRANCH crates/pull/src/alerts.rs:1458:28 False: if let Some(mark) = acked.get_mut(i) {
UNCOVERED BRANCH crates/pull/src/alerts.rs:1594:8 True: if !regular(path)? {
UNCOVERED BRANCH crates/pull/src/alerts.rs:1700:35 False: .is_ok_and(|meta| meta.dev() == queue.dev() && meta.ino() == queue.ino());
UNCOVERED BRANCH crates/pull/src/alerts.rs:1720:25 False: Err(why) if why.kind() == io::ErrorKind::AlreadyExists && attempt < 64 => {
UNCOVERED BRANCH crates/pull/src/alerts.rs:1801:19 True: flags: if text_cut { FLAG_TEXT_CUT } else { 0 },
UNCOVERED BRANCH crates/pull/src/alerts.rs:1821:12 True: if let Err(why) = fs::rename(&tmp, &names.queue) {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:78: Put::Duplicate { ordinal } => panic!("a duplicate of {ordinal}, not a write"),
UNCOVERED LINE crates/pull/src/alerts_tests.rs:210: matches!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:243: panic!("{got:?}")
UNCOVERED LINE crates/pull/src/alerts_tests.rs:414: panic!("{:?}", kept[3])
UNCOVERED LINE crates/pull/src/alerts_tests.rs:1100: matches!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:1134: panic!("{refused:?}")
UNCOVERED LINE crates/pull/src/alerts_tests.rs:1588: matches!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2063: fn spread(mut micros: Vec<u128>) -> (u128, u128, u128) {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2064: micros.sort_unstable();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2065: let at = |q: usize| micros[(micros.len() * q / 100).min(micros.len() - 1)];
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2066: (at(50), at(99), micros[micros.len() - 1])
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2067: }
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2074: fn the_operators_older_steps_timed() {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2075: let scratch = Scratch::new("timed");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2076: let records = 102_400_u32;
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2077: raw_queue(&scratch, &open_alerts(records));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2078: for (label, before, later) in [
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2079: ("1,024 records after the page", records - 1_023, 1_024_u32),
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2080: ("51,201 records after the page", 51_200, 51_201),
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2081: ("101,376 records after the page", 1_025, 101_376),
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2082: ] {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2083: let mut micros = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2084: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2085: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2086: let page = page_settled(scratch.root(), before).unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2087: micros.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2088: assert_eq!(page.alerts.len(), 1_024);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2089: assert_eq!(page.older_unacknowledged, before - 1 - 1_024);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2090: assert_eq!(u64::from(later), page.records - u64::from(before) + 1);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2092: let (p50, p99, max) = spread(micros);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2093: println!("page_settled, {label}: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2095: let mut one_read = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2096: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2097: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2098: let page = page_of(&scratch, Some(51_200));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2099: one_read.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2100: assert_eq!(page.alerts.len(), 1_024);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2102: let (p50, p99, max) = spread(one_read);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2103: println!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2109: let mut walk = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2110: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2111: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2112: let found = older(scratch.root()).unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2113: walk.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2114: assert_eq!(found.alerts.len(), 101_376);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2115: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2117: let (p50, p99, max) = spread(walk);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2118: println!("older, 102,400 records: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2122: let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2123: let walker = {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2124: let stop = std::sync::Arc::clone(&stop);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2125: let root = scratch.root().to_path_buf();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2126: std::thread::spawn(move || {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2127: let mut walks = 0_u32;
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2128: while !stop.load(Ordering::Relaxed) {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2129: assert!(older(&root).unwrap().alerts.len() >= 101_376);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2130: walks += 1;
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2132: walks
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2133: })
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2135: let mut puts = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2136: for i in 0..200_u32 {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2137: let key = format!("timed-put-{i}");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2138: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2139: let put = put_at(scratch.root(), &alert(6, &key, "t"), NOW).unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2140: puts.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2141: assert_eq!(written(put), records + 1 + i);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2143: stop.store(true, Ordering::Relaxed);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2144: let walks = walker.join().unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2145: let (p50, p99, max) = spread(puts);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2146: println!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2152: for size in [7_u32, 102_400] {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2153: let n: u64 = if size == 7 { 100 } else { 30 };
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2154: let mut micros = Vec::with_capacity(100);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2155: for i in 0..n {
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2156: let scratch = Scratch::new("timed-replace");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2157: raw_queue(&scratch, &open_alerts(size));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2158: spoil(&scratch, u64::from(size) - 1);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2159: spoil(&scratch, u64::from(size));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2160: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2161: let done = replace_damaged_at(scratch.root(), NOW + i).unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2162: micros.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2163: assert_eq!((done.open, done.ordinal), (Some(size - 2), 1));
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2165: let (p50, p99, max) = spread(micros);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2166: println!(
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2170: }
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2336: let stale = scratch.root().join("stale");
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2337: fs::write(&stale, b"left by a killed run").unwrap();
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2338: drop(create_fresh(&stale).unwrap());
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2339: assert_eq!(fs::metadata(&stale).unwrap().len(), 0);
UNCOVERED LINE crates/pull/src/alerts_tests.rs:2340: }
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:237:9 False: let Put::Written {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:413:9 False: let Kept::Good(4, record) = &kept[3] else {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:504:13 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:504:30 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:504:59 False: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:779:52 False: assert!(matches!(&seen[0], Kept::Good(1, r) if r.text == "invented k1"));
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:780:52 False: assert!(matches!(&seen[2], Kept::Good(3, r) if r.open_after == 3));
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:892:13 False: assert!(names.queue.is_file() && names.lock.is_file());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:892:38 False: assert!(names.queue.is_file() && names.lock.is_file());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:984:5 False: page.exists && page.alerts.is_empty() && page.first == 0 && page.damaged.is_empty()
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:984:20 False: page.exists && page.alerts.is_empty() && page.first == 0 && page.damaged.is_empty()
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:984:46 False: page.exists && page.alerts.is_empty() && page.first == 0 && page.damaged.is_empty()
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1133:9 False: let Err(AlertError::Io { kind, op, .. }) = &refused else {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1161:46 False: if target >= 1 && target % 7 == 0 && is_alert(&records[(target - 1) as usize]) {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1251:17 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1251:32 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1251:58 False: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1515:38 False: .is_some_and(|a| a.ordinal == 1 && a.record.kind == KIND_QUEUE_REPLACED);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1517:21 True: seen.damaged == [3, 2] || replaced,
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1517:47 False: seen.damaged == [3, 2] || replaced,
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1538:56 False: matches!(&first, Some(Kept::Good(1, r)) if r.kind == KIND_QUEUE_REPLACED),
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1851:13 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1851:28 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1851:54 False: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1858:13 False: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:1858:40 False: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2115:17 True: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2115:17 False: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2115:45 True: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2115:45 False: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2128:19 True: while !stop.load(Ordering::Relaxed) {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2128:19 False: while !stop.load(Ordering::Relaxed) {
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2153:25 True: let n: u64 = if size == 7 { 100 } else { 30 };
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2153:25 False: let n: u64 = if size == 7 { 100 } else { 30 };
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2186:13 False: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2186:38 False: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2219:9 False: before <= now && now <= after,
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2219:26 False: before <= now && now <= after,
UNCOVERED BRANCH crates/pull/src/alerts_tests.rs:2458:62 False: matches!(&refused, Err(AlertError::Io { op, .. }) if *op == "keep the damaged queue"),
UNCOVERED LINE crates/pull/src/archive.rs:310: return String::new();
UNCOVERED LINE crates/pull/src/archive.rs:717: path: dir.to_path_buf(),
UNCOVERED LINE crates/pull/src/archive.rs:718: detail: e.to_string(),
UNCOVERED LINE crates/pull/src/archive.rs:719: })?;
UNCOVERED LINE crates/pull/src/archive.rs:871: return Err(too_large(read as u64));
UNCOVERED LINE crates/pull/src/archive.rs:1319: walked.len()
UNCOVERED LINE crates/pull/src/archive.rs:1335: sentence.len()
UNCOVERED LINE crates/pull/src/archive.rs:1370: matches!(
UNCOVERED BRANCH crates/pull/src/archive.rs:309:9 False: let Some(stem) = path.file_stem() else {
UNCOVERED BRANCH crates/pull/src/archive.rs:316:16 False: if !ext.is_empty()
UNCOVERED BRANCH crates/pull/src/archive.rs:870:8 True: if read as u64 > cap {
UNCOVERED BRANCH crates/pull/src/archive.rs:1118:77 False: matches!(why, ArchiveError::MemberMalformed { ref path, .. } if *path == odd),
UNCOVERED LINE crates/pull/src/base_cov_words_tests.rs:127: panic!("Dhan is an HTTP broker");
UNCOVERED BRANCH crates/pull/src/base_cov_words_tests.rs:125:9 False: let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED LINE crates/pull/src/calendar.rs:338: return false;
UNCOVERED LINE crates/pull/src/calendar.rs:679: return DayKind::Unmeasured;
UNCOVERED LINE crates/pull/src/calendar.rs:742: DayKind::Unmeasured => return None,
UNCOVERED LINE crates/pull/src/calendar.rs:1949: return Phase::Unverified;
UNCOVERED LINE crates/pull/src/calendar.rs:2555: Err(SettlementRefusal::NoSessionWithin { stated })
UNCOVERED LINE crates/pull/src/calendar.rs:2614: Ok(None)
UNCOVERED LINE crates/pull/src/calendar.rs:3119: None => (FIRST_DAY, last),
UNCOVERED LINE crates/pull/src/calendar.rs:3154: measured_first
UNCOVERED LINE crates/pull/src/calendar.rs:3262: DayKind::Unmeasured => out.push(4),
UNCOVERED LINE crates/pull/src/calendar.rs:3582: return Err(outside);
UNCOVERED LINE crates/pull/src/calendar.rs:4212: DayKind::Unmeasured => panic!("day {day} is inside the range and unmeasured"),
UNCOVERED LINE crates/pull/src/calendar.rs:4334: panic!("{index} {day}")
UNCOVERED LINE crates/pull/src/calendar.rs:4368: panic!("2021-02-24 traded");
UNCOVERED LINE crates/pull/src/calendar.rs:4384: panic!("2025-10-21 traded");
UNCOVERED LINE crates/pull/src/calendar.rs:4394: panic!("2024-03-02 traded");
UNCOVERED LINE crates/pull/src/calendar.rs:4411: panic!("an ordinary weekday traded");
UNCOVERED LINE crates/pull/src/calendar.rs:4585: panic!("it traded");
UNCOVERED LINE crates/pull/src/calendar.rs:5042: continue;
UNCOVERED LINE crates/pull/src/calendar.rs:5173: return changed;
UNCOVERED LINE crates/pull/src/calendar.rs:5371: panic!("{label} is open on the calendar of record")
UNCOVERED LINE crates/pull/src/calendar.rs:5388: other => panic!("{label}: sourced_kind_of answered {other:?}"),
UNCOVERED LINE crates/pull/src/calendar.rs:5444: _ => false,
UNCOVERED LINE crates/pull/src/calendar.rs:5448: matches!(
UNCOVERED LINE crates/pull/src/calendar.rs:6451: ExpiryDay::Unmeasured => panic!("day {day} is answered"),
UNCOVERED LINE crates/pull/src/calendar.rs:6564: fn calendar_of_record_costs() {
UNCOVERED LINE crates/pull/src/calendar.rs:6565: let mut days: Vec<i64> = SPECIAL_SESSIONS.iter().map(|s| s.day).collect();
UNCOVERED LINE crates/pull/src/calendar.rs:6566: days.extend(HALTS.iter().map(|h| h.day));
UNCOVERED LINE crates/pull/src/calendar.rs:6567: days.extend((SOURCED_FIRST_DAY..=SOURCED_LAST_DAY).step_by(97));
UNCOVERED LINE crates/pull/src/calendar.rs:6568: days.push(SOURCED_LAST_DAY + 1);
UNCOVERED LINE crates/pull/src/calendar.rs:6569: measure("exchange kind of a day", &days, exchange_kind_of);
UNCOVERED LINE crates/pull/src/calendar.rs:6570: measure("phase at one minute", &days, |d| {
UNCOVERED LINE crates/pull/src/calendar.rs:6571: phase_at(Venue::NseCash, d, 915, CasEligibility::Eligible)
UNCOVERED LINE crates/pull/src/calendar.rs:6572: });
UNCOVERED LINE crates/pull/src/calendar.rs:6573: measure("phases of a whole day", &days, |d| {
UNCOVERED LINE crates/pull/src/calendar.rs:6574: phases_of(Venue::NseCash, d, CasEligibility::Eligible)
UNCOVERED LINE crates/pull/src/calendar.rs:6575: });
UNCOVERED LINE crates/pull/src/calendar.rs:6576: measure("settlement on or before", &days, settlement_on_or_before);
UNCOVERED LINE crates/pull/src/calendar.rs:6577: measure("record of a day", &days, record_of);
UNCOVERED LINE crates/pull/src/calendar.rs:6578: }
UNCOVERED LINE crates/pull/src/calendar.rs:6583: fn measure<T>(name: &str, days: &[i64], op: impl Fn(i64) -> T) {
UNCOVERED LINE crates/pull/src/calendar.rs:6586: let mut samples = Vec::new();
UNCOVERED LINE crates/pull/src/calendar.rs:6587: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/calendar.rs:6588: for &day in days {
UNCOVERED LINE crates/pull/src/calendar.rs:6589: let started = Instant::now();
UNCOVERED LINE crates/pull/src/calendar.rs:6590: for _ in 0..16 {
UNCOVERED LINE crates/pull/src/calendar.rs:6591: black_box(op(black_box(day)));
UNCOVERED LINE crates/pull/src/calendar.rs:6592: }
UNCOVERED LINE crates/pull/src/calendar.rs:6593: samples.push(started.elapsed().as_nanos() / 16);
UNCOVERED LINE crates/pull/src/calendar.rs:6596: samples.sort_unstable();
UNCOVERED LINE crates/pull/src/calendar.rs:6597: let pick = |permille: usize| {
UNCOVERED LINE crates/pull/src/calendar.rs:6598: samples
UNCOVERED LINE crates/pull/src/calendar.rs:6599: .get(samples.len() * permille / 1_000)
UNCOVERED LINE crates/pull/src/calendar.rs:6600: .copied()
UNCOVERED LINE crates/pull/src/calendar.rs:6601: .unwrap_or(u128::MAX)
UNCOVERED LINE crates/pull/src/calendar.rs:6602: };
UNCOVERED LINE crates/pull/src/calendar.rs:6603: let (p50, p99) = (pick(500), pick(990));
UNCOVERED LINE crates/pull/src/calendar.rs:6604: eprintln!("{name}: p50 {p50} ns, p99 {p99} ns, n = {}", samples.len());
UNCOVERED LINE crates/pull/src/calendar.rs:6605: assert!(p99 < 50_000, "{name}: p99 {p99} ns");
UNCOVERED LINE crates/pull/src/calendar.rs:6606: }
UNCOVERED BRANCH crates/pull/src/calendar.rs:337:12 True: if !CalendarVersion::on(*row).record_is_unbroken() {
UNCOVERED BRANCH crates/pull/src/calendar.rs:418:42 False: while i < self.count as usize && i < MAX_WINDOWS {
UNCOVERED BRANCH crates/pull/src/calendar.rs:419:20 False: if let Some(w) = self.windows.get(i) {
UNCOVERED BRANCH crates/pull/src/calendar.rs:435:42 False: while i < self.count as usize && i < MAX_WINDOWS {
UNCOVERED BRANCH crates/pull/src/calendar.rs:678:9 False: let Ok(offset) = usize::try_from(epoch_day - FIRST_DAY) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:833:18 False: let era = if y >= 0 { y } else { y - 399 } / 400;
UNCOVERED BRANCH crates/pull/src/calendar.rs:903:11 False: while year < u16::MAX
UNCOVERED BRANCH crates/pull/src/calendar.rs:1015:18 False: let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
UNCOVERED BRANCH crates/pull/src/calendar.rs:1948:9 False: let Ok(minute) = u16::try_from(second / 60) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:1969:12 False: && open == OPEN_MINUTE
UNCOVERED BRANCH crates/pull/src/calendar.rs:2076:16 False: if band.from <= minute && minute <= band.to {
UNCOVERED BRANCH crates/pull/src/calendar.rs:2101:16 False: if let Some(slot) = self.bands.get_mut(used) {
UNCOVERED BRANCH crates/pull/src/calendar.rs:2128:16 False: if let Ok(minute) = u16::try_from(minute)
UNCOVERED BRANCH crates/pull/src/calendar.rs:2129:16 False: && minute < MINUTES_PER_DAY
UNCOVERED BRANCH crates/pull/src/calendar.rs:2130:20 False: && let Some(slot) = cuts.get_mut(used)
UNCOVERED BRANCH crates/pull/src/calendar.rs:2435:42 False: DayKind::OpenLengthUnmeasured if listed(&SOURCED_SPECIAL, epoch_day) => {
UNCOVERED BRANCH crates/pull/src/calendar.rs:3153:24 True: let first = if measured_first < self.sourced.0 {
UNCOVERED BRANCH crates/pull/src/calendar.rs:3537:20 False: if let Some(count) = running.get_mut(slot) {
UNCOVERED BRANCH crates/pull/src/calendar.rs:3581:13 False: let (Some(before), Some(through)) = (self.sums.get(low), self.sums.get(high)) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:3752:12 False: if newest && civil_year(notice.day) == i64::from(year) {
UNCOVERED BRANCH crates/pull/src/calendar.rs:3869:17 False: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
UNCOVERED BRANCH crates/pull/src/calendar.rs:3869:56 False: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
UNCOVERED BRANCH crates/pull/src/calendar.rs:3879:17 False: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
UNCOVERED BRANCH crates/pull/src/calendar.rs:3879:55 False: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
UNCOVERED BRANCH crates/pull/src/calendar.rs:3890:65 False: matches!(record.kind, DayKind::Open(session) if session.bars() == 375),
UNCOVERED BRANCH crates/pull/src/calendar.rs:3900:62 False: .all(|pair| matches!(pair, [older, newer] if older.day < newer.day)),
UNCOVERED BRANCH crates/pull/src/calendar.rs:4039:17 False: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
UNCOVERED BRANCH crates/pull/src/calendar.rs:4039:35 False: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
UNCOVERED BRANCH crates/pull/src/calendar.rs:4333:21 False: let DayKind::Open(session) = kind_of(day) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:4348:71 False: .all(|pair| matches!(pair, [first, second] if second - first == 60))
UNCOVERED BRANCH crates/pull/src/calendar.rs:4367:13 False: let DayKind::Open(outage) = kind_of(SYSTEMS_OUTAGE_DAY) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:4383:13 False: let DayKind::Open(muhurat) = kind_of(20_382) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:4393:13 False: let DayKind::Open(dr) = kind_of(19_784) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:4410:13 False: let DayKind::Open(full) = kind_of(20_668) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:4584:13 False: let DayKind::Open(session) = cal.kind_of(20_382) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:5041:17 False: let Some(slot) = kinds.get_mut(at) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:5169:13 False: let (Ok(start), Ok(end)) = (
UNCOVERED BRANCH crates/pull/src/calendar.rs:5370:17 False: let DayKind::Open(session) = exchange_kind_of(day) else {
UNCOVERED BRANCH crates/pull/src/calendar.rs:6467:21 False: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
UNCOVERED BRANCH crates/pull/src/calendar.rs:6467:49 False: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
UNCOVERED LINE crates/pull/src/calendar_tradable_tests.rs:252: matches!(
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:247:13 False: notice.source.contains(" https://") && notice.source.contains(" of "),
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:247:52 False: notice.source.contains(" https://") && notice.source.contains(" of "),
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:341:13 False: assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:341:51 False: assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1028:20 False: if (record_first, record_last) == (SOURCED_FIRST_DAY, SOURCED_LAST_DAY)),
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1190:13 False: assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1190:37 False: assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1204:13 False: assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1204:37 False: assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1219:13 False: assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1219:35 False: assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1234:13 False: assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/calendar_tradable_tests.rs:1234:37 False: assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
UNCOVERED BRANCH crates/pull/src/capture.rs:598:13 False: !text.to_ascii_lowercase().contains("authorization")
UNCOVERED BRANCH crates/pull/src/capture.rs:599:20 False: && !text.to_ascii_lowercase().contains("access-token"),
UNCOVERED LINE crates/pull/src/cash_auction.rs:713: None => break,
UNCOVERED LINE crates/pull/src/cash_auction.rs:971: matches!(
UNCOVERED LINE crates/pull/src/cash_auction.rs:1453: fn actual_dated_masters_match_every_current_fno_cash_identity() {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1456: let root = std::path::PathBuf::from(
UNCOVERED LINE crates/pull/src/cash_auction.rs:1457: std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR").expect("local sample directory"),
UNCOVERED LINE crates/pull/src/cash_auction.rs:1459: let members: Vec<_> = FNO_UNDERLYINGS
UNCOVERED LINE crates/pull/src/cash_auction.rs:1460: .into_iter()
UNCOVERED LINE crates/pull/src/cash_auction.rs:1461: .filter(|symbol| NTM_INDEX.contains(symbol))
UNCOVERED LINE crates/pull/src/cash_auction.rs:1462: .collect();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1463: assert_eq!(members.len(), 208, "current F&O/total-market intersection");
UNCOVERED LINE crates/pull/src/cash_auction.rs:1464: let first = Day::new(2026, 8, 3).unwrap().days_from_epoch();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1465: let last = Day::new(2026, 9, 4).unwrap().days_from_epoch();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1466: let mut days = 0;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1467: let mut mismatches = Vec::new();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1468: for number in first..=last {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1469: let day = Day::from_days(number).unwrap();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1470: let path = root.join(format!(
UNCOVERED LINE crates/pull/src/cash_auction.rs:1471: "NSE_CM_security_{:02}{:02}{:04}.csv.gz",
UNCOVERED LINE crates/pull/src/cash_auction.rs:1472: day.day(),
UNCOVERED LINE crates/pull/src/cash_auction.rs:1473: day.month(),
UNCOVERED LINE crates/pull/src/cash_auction.rs:1474: day.year()
UNCOVERED LINE crates/pull/src/cash_auction.rs:1475: ));
UNCOVERED LINE crates/pull/src/cash_auction.rs:1476: if !path.try_exists().unwrap() {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1477: continue;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1478: }
UNCOVERED LINE crates/pull/src/cash_auction.rs:1479: days += 1;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1480: let master = read_actual_master(&path);
UNCOVERED LINE crates/pull/src/cash_auction.rs:1481: let mut eligible = 0;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1482: let mut ineligible = 0;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1483: for symbol in &members {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1484: let isin = nse_isin(symbol).expect("core cash identity must carry an ISIN");
UNCOVERED LINE crates/pull/src/cash_auction.rs:1485: match master.eligibility(symbol, isin.as_str()) {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1486: Ok(true) => eligible += 1,
UNCOVERED LINE crates/pull/src/cash_auction.rs:1487: Ok(false) => {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1488: ineligible += 1;
UNCOVERED LINE crates/pull/src/cash_auction.rs:1489: eprintln!("{day} {symbol} ISIN={isin}: explicit eligibility=0");
UNCOVERED LINE crates/pull/src/cash_auction.rs:1490: }
UNCOVERED LINE crates/pull/src/cash_auction.rs:1491: Err(why) => {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1492: let same_symbol = master.by_symbol.get(*symbol);
UNCOVERED LINE crates/pull/src/cash_auction.rs:1493: let mut same_isin: Vec<_> = master
UNCOVERED LINE crates/pull/src/cash_auction.rs:1494: .by_symbol
UNCOVERED LINE crates/pull/src/cash_auction.rs:1495: .iter()
UNCOVERED LINE crates/pull/src/cash_auction.rs:1496: .filter(|(_, identity)| identity.isin == isin.as_str())
UNCOVERED LINE crates/pull/src/cash_auction.rs:1497: .map(|(name, identity)| format!("{name}: {identity:?}"))
UNCOVERED LINE crates/pull/src/cash_auction.rs:1498: .collect();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1499: same_isin.sort_unstable();
UNCOVERED LINE crates/pull/src/cash_auction.rs:1500: mismatches.push(format!("{day} {symbol} expected_ISIN={isin}: {why}; same_symbol={same_symbol:?}; same_ISIN_rows={same_isin:?}"));
UNCOVERED LINE crates/pull/src/cash_auction.rs:1504: eprintln!(
UNCOVERED LINE crates/pull/src/cash_auction.rs:1506: members.len() - eligible - ineligible
UNCOVERED LINE crates/pull/src/cash_auction.rs:1509: assert_eq!(days, 25, "all supplied dated masters must be inspected");
UNCOVERED LINE crates/pull/src/cash_auction.rs:1510: for mismatch in &mismatches {
UNCOVERED LINE crates/pull/src/cash_auction.rs:1511: eprintln!("{mismatch}");
UNCOVERED LINE crates/pull/src/cash_auction.rs:1512: }
UNCOVERED LINE crates/pull/src/cash_auction.rs:1513: assert!(
UNCOVERED LINE crates/pull/src/cash_auction.rs:1514: mismatches.is_empty(),
UNCOVERED LINE crates/pull/src/cash_auction.rs:1516: mismatches.len()
UNCOVERED LINE crates/pull/src/cash_auction.rs:1518: }
UNCOVERED BRANCH crates/pull/src/cash_auction.rs:1476:16 True: if !path.try_exists().unwrap() {
UNCOVERED BRANCH crates/pull/src/cash_auction.rs:1476:16 False: if !path.try_exists().unwrap() {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1134: fn held_masters_give_every_cas_session_its_auction_band() -> Result<(), String> {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1137: let root = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1138: .filter(|value| !value.is_empty())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1139: .map(PathBuf::from)
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1140: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the receipted master directory")?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1141: let last = Day::new(2026, 9, 30).map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1142: let days = required_days(Window::new(day(3), last).map_err(|why| why.to_string())?)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1143: assert_eq!(days.len(), 42);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1144: let reliance = nse_isin("RELIANCE").ok_or("RELIANCE has a cash ISIN")?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1145: for date in &days {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1146: let master = read_local_lifecycle(&root, *date)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1147: let phases = master.cash_phases("RELIANCE", reliance.as_str())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1148: assert_eq!(phases.minutes(calendar::Phase::Continuous), 360, "{date}");
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1149: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1150: phases.phase_at(15 * 60 + 15),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1154: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1155: phases.phase_at(15 * 60 + 30),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1160: let sept_24 = Day::new(2026, 9, 24).map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1161: let master = read_local_lifecycle(&root, sept_24)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1162: let dalbharat = nse_isin("DALBHARAT").ok_or("DALBHARAT has a cash ISIN")?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1163: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1164: master.cas_eligibility("DALBHARAT", dalbharat.as_str()),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1167: let ineligible = master.cash_phases("DALBHARAT", dalbharat.as_str())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1168: assert_eq!(ineligible.minutes(calendar::Phase::Continuous), 375);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1169: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1170: ineligible.phase_at(15 * 60 + 15),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1173: eprintln!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1175: days.len()
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1177: Ok(())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1178: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1182: fn actual_receipted_lifecycle_snapshot_never_claims_complete_history() -> Result<(), String> {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1186: let root = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1187: .filter(|value| !value.is_empty())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1188: .map(PathBuf::from)
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1189: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the receipted master directory")?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1190: let date = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1191: let master = read_local_lifecycle(&root, date)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1192: assert_eq!(master.provenance().master_day, date);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1193: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1194: master.provenance().source_url,
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1197: let mut count = 0;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1198: let mut unavailable = 0;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1199: let mut partial = 0;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1200: for symbol in FNO_UNDERLYINGS
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1201: .into_iter()
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1202: .filter(|symbol| NTM_INDEX.contains(symbol))
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1204: let isin = nse_isin(symbol).expect("current cash table identity");
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1205: let report = master.inspect(symbol, isin.as_str())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1206: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1211: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1216: match report.identity.metadata.listing {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1218: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1223: unavailable += 1;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1226: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1231: partial += 1;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1233: ref other => {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1234: return Err(format!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1235: "unexpected actual listing evidence for {symbol}: {other:?}"
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1236: ));
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1239: count += 1;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1241: assert_eq!(count, 208);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1242: assert_eq!(partial + unavailable, count);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1243: assert!(partial > 0 && unavailable > 0);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1244: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1245: master
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1246: .inspect("ADANIENT", "INE423A01024")?
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1252: for (symbol, isin, expected) in [
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1253: ("IRFC", "INE053F01010", Day::new(2021, 1, 29)),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1254: ("FORCEMOT", "INE451A01017", Day::new(2019, 8, 19)),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1255: ("RELIANCE", "INE002A01018", Day::new(1995, 11, 29)),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1256: ] {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1257: let report = master.inspect(symbol, isin)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1258: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1259: report.identity.metadata.listing.date(),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1260: Some(expected.expect("fixture date"))
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1263: eprintln!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1266: Ok(())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1267: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1277: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1278: unreachable!("{url}: a fully cached window must not use the network")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1526: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1527: unreachable!("{url}: cached dates must never fetch again")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1607: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1608: unreachable!("{url}: a root with no receipt is refused before any fetch")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1874: async fn official_25_dated_masters_install_and_validate_without_network() -> Result<(), String>
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1875: {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1876: let source = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1877: .filter(|value| !value.is_empty())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1878: .map(PathBuf::from)
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1879: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the public NSE sample directory")?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1880: let end = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1881: let span = Window::new(day(3), end).map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1884: let mut expected: Vec<_> = [
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1885: 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 24, 25, 26, 27, 28, 31,
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1886: ]
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1887: .into_iter()
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1888: .map(day)
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1889: .collect();
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1890: for date in 1..=4 {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1891: expected.push(Day::new(2026, 9, date).map_err(|why| why.to_string())?);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1893: assert_eq!(expected.len(), 25);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1894: let mut missing: std::collections::BTreeSet<_> = expected.iter().copied().collect();
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1895: let temp = Temp::new();
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1896: let entries = fs::read_dir(&source)
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1897: .map_err(|why| format!("cannot read sample directory {}: {why}", source.display()))?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1898: for entry in entries {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1899: let entry = entry.map_err(|why| why.to_string())?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1900: let name = entry.file_name();
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1901: let Some(name) = name.to_str() else { continue };
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1902: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1903: continue;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1904: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1905: let date = sample_day(name)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1906: if !missing.remove(&date) {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1907: return Err(format!("unexpected or duplicate dated master: {name}"));
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1908: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1909: let bytes = read_limited(&entry.path(), MAX_COMPRESSED)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1910: install(&temp.0, date, &bytes).map_err(|why| format!("{name}: {why}"))?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1912: if !missing.is_empty() {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1913: return Err(format!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1914: "missing official dated master samples: {missing:?}"
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1915: ));
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1916: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1917: let mut observed = HashMap::new();
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1918: prepare_observed_with(&temp.0, &expected, &mut observed, |url| async move {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1919: Err(format!("sample audit must not fetch: {url}"))
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1920: })
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1921: .await?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1922: assert_eq!(observed.len(), 25);
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1923: for date in expected {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1924: let bytes =
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1925: read_entry(&temp.0, date)?.ok_or_else(|| format!("cache omitted {date}"))?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1926: let eligibility = decode(&bytes)?;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1927: assert!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1928: eligibility.eligibility("HINDALCO", "INE038A01020")?,
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1931: assert!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1932: observed
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1933: .get(&date.days_from_epoch())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1934: .expect("observed master retained")
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1935: .eligibility("HINDALCO", "INE038A01020")?
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1937: assert!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1938: !eligibility.eligibility("20MICRONS", "INE144J01027")?,
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1942: let result = prepare_with(&temp.0, span, |url| async move {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1943: Err(format!("sample audit must not fetch: {url}"))
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1944: })
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1945: .await;
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1946: match required_days(span) {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1947: Ok(days) => assert_eq!(result?.len(), days.len()),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1948: Err(calendar_refusal) => {
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1949: assert_eq!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1950: result.expect_err("unmeasured calendar must refuse"),
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1951: calendar_refusal
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1952: );
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1953: eprintln!(
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1954: "All 25 official masters validated and reloaded; full-window prepare refused: {calendar_refusal}"
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1955: );
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1956: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1957: }
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1958: Ok(())
UNCOVERED LINE crates/pull/src/cash_session_cache.rs:1959: }
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:933:17 False: assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:933:47 False: assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:938:17 False: assert!(why.contains("absent") && why.contains("no download attempted"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:938:43 False: assert!(why.contains("absent") && why.contains("no download attempted"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:940:17 False: assert!(!payload.exists() && !metadata.exists());
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:940:38 False: assert!(!payload.exists() && !metadata.exists());
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:983:17 False: why.contains("UNVERIFIED") && why.contains("receipt"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:983:47 False: why.contains("UNVERIFIED") && why.contains("receipt"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1005:13 False: why.contains("unavailable") && why.contains("30 ms bound"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1005:44 False: why.contains("unavailable") && why.contains("30 ms bound"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1243:17 True: assert!(partial > 0 && unavailable > 0);
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1243:17 False: assert!(partial > 0 && unavailable > 0);
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1243:32 True: assert!(partial > 0 && unavailable > 0);
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1243:32 False: assert!(partial > 0 && unavailable > 0);
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1626:17 False: assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1626:49 False: assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1650:17 False: assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1650:49 False: assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1901:17 True: let Some(name) = name.to_str() else { continue };
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1901:17 False: let Some(name) = name.to_str() else { continue };
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1902:16 True: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1902:16 False: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1902:57 True: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1902:57 False: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1906:16 True: if !missing.remove(&date) {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1906:16 False: if !missing.remove(&date) {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1912:12 True: if !missing.is_empty() {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:1912:12 False: if !missing.is_empty() {
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2212:13 False: why.contains("readers or another installer held it")
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2213:20 False: && why.contains("for the whole 30 ms bound")
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2214:20 False: && why.contains("nothing was installed"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2284:21 False: why.starts_with("UNVERIFIED cannot open local lifecycle lock")
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2285:28 False: && why.contains(&lock.display().to_string()),
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2377:21 False: why.starts_with(&format!(
UNCOVERED BRANCH crates/pull/src/cash_session_cache.rs:2379:27 False: )) && why.ends_with("; crash durability UNVERIFIED"),
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:232: (_, Ok(answer)) => assert_eq!(answer, whole, "reader {nth}"),
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:372: let held = PathBuf::from(held);
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:373: let file = OpenOptions::new()
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:374: .read(true)
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:375: .write(true)
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:376: .open(&held)
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:377: .expect("the parent's lock file");
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:378: file.try_lock().expect("the child holds the day alone");
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:379: fs::write(held.with_extension("ready"), b"held").expect("ready marker");
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:380: thread::sleep(Duration::from_mins(2));
UNCOVERED LINE crates/pull/src/cash_session_cache_lock_tests.rs:382: std::process::exit(3);
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:130:53 False: assert!(solo.iter().all(|&(_, hindalco, micro)| hindalco && !micro));
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:193:9 False: why.contains("unavailable")
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:194:16 False: && why.contains("an installer held it exclusively")
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:195:16 False: && why.contains("40 ms bound")
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:196:16 False: && why.contains("not read as absent"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:234:50 False: why.contains("is absent") || why.contains("missing locally"),
UNCOVERED BRANCH crates/pull/src/cash_session_cache_lock_tests.rs:371:12 True: if let Some(held) = std::env::var_os("BRUTEX_AF03_HOLD") {
UNCOVERED LINE crates/pull/src/cash_session_cache_ps1_tests.rs:63: panic!("the host took a shared flock on an O_PATH descriptor");
UNCOVERED LINE crates/pull/src/cash_session_cache_ps1_tests.rs:66: panic!("the host took an exclusive flock on an O_PATH descriptor");
UNCOVERED BRANCH crates/pull/src/cash_session_cache_ps1_tests.rs:62:9 False: let Err(TryLockError::Error(shared)) = unlockable(&lock).try_lock_shared() else {
UNCOVERED BRANCH crates/pull/src/cash_session_cache_ps1_tests.rs:65:9 False: let Err(TryLockError::Error(exclusive)) = unlockable(&lock).try_lock() else {
UNCOVERED LINE crates/pull/src/census_store.rs:249: Err(absent)
UNCOVERED LINE crates/pull/src/census_store.rs:255: return Ok(None);
UNCOVERED LINE crates/pull/src/census_store.rs:271: sentence: format!("{} could not be measured: {why}", entry.path().display()),
UNCOVERED LINE crates/pull/src/census_store.rs:272: kind: why.kind(),
UNCOVERED LINE crates/pull/src/census_store.rs:273: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:668: return Err(named(&Unbounded::PastCap { cap, size }));
UNCOVERED LINE crates/pull/src/census_store.rs:957: epoch_name(done.to_epoch)
UNCOVERED LINE crates/pull/src/census_store.rs:1180: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:1181: "{}'s census is a single file ({}); a key is withdrawn by installing the \
UNCOVERED LINE crates/pull/src/census_store.rs:1182: partition's next epoch, which only a migrated census has. Run `cli \
UNCOVERED LINE crates/pull/src/census_store.rs:1183: census-migrate {}` first (D-3316); nothing was written",
UNCOVERED LINE crates/pull/src/census_store.rs:1184: vendor.as_str(),
UNCOVERED LINE crates/pull/src/census_store.rs:1185: target.named().display(),
UNCOVERED LINE crates/pull/src/census_store.rs:1186: vendor.as_str()
UNCOVERED LINE crates/pull/src/census_store.rs:1187: ));
UNCOVERED LINE crates/pull/src/census_store.rs:1255: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:1256: "{}: a single-file census is never compacted by a write (D-3316)",
UNCOVERED LINE crates/pull/src/census_store.rs:1257: target.named().display()
UNCOVERED LINE crates/pull/src/census_store.rs:1258: ));
UNCOVERED LINE crates/pull/src/census_store.rs:1424: }
UNCOVERED LINE crates/pull/src/census_store.rs:1722: format!(" (published: {})", published.join(", "))
UNCOVERED LINE crates/pull/src/census_store.rs:1871: pub const fn last(&self) -> Option<Pace> {
UNCOVERED LINE crates/pull/src/census_store.rs:1872: self.last
UNCOVERED LINE crates/pull/src/census_store.rs:1873: }
UNCOVERED LINE crates/pull/src/census_store.rs:1925: Ok(_) => return Err("not_regular"),
UNCOVERED LINE crates/pull/src/census_store.rs:1926: Err(_) => return Err("unmeasured"),
UNCOVERED LINE crates/pull/src/census_store.rs:1958: return Ok(Err("unopened"));
UNCOVERED LINE crates/pull/src/census_store.rs:1961: return Ok(Err("unmeasured"));
UNCOVERED LINE crates/pull/src/census_store.rs:1972: return Ok(Err("header_unread"));
UNCOVERED LINE crates/pull/src/census_store.rs:1975: return Ok(Err("header_moved"));
UNCOVERED LINE crates/pull/src/census_store.rs:2193: }
UNCOVERED LINE crates/pull/src/census_store.rs:2291: return Ok(None);
UNCOVERED LINE crates/pull/src/census_store.rs:2304: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/census_store.rs:2305: &telemetry::Event::error(TARGET, "census verb lock not released")
UNCOVERED LINE crates/pull/src/census_store.rs:2306: .with("run", run)
UNCOVERED LINE crates/pull/src/census_store.rs:2307: .with("why", why.as_str()),
UNCOVERED LINE crates/pull/src/census_store.rs:2308: );
UNCOVERED LINE crates/pull/src/census_store.rs:2310: }
UNCOVERED LINE crates/pull/src/census_store.rs:2351: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2352: "{}: the split holds {keys} keys and {rows} rows, the census {} and {}; \
UNCOVERED LINE crates/pull/src/census_store.rs:2353: nothing was written",
UNCOVERED LINE crates/pull/src/census_store.rs:2354: legacy.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:2355: census.keys(),
UNCOVERED LINE crates/pull/src/census_store.rs:2356: census.total_rows()
UNCOVERED LINE crates/pull/src/census_store.rs:2357: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2372: )?;
UNCOVERED LINE crates/pull/src/census_store.rs:2375: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2378: staging.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:2379: dir.display()
UNCOVERED LINE crates/pull/src/census_store.rs:2381: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:2445: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2446: "{}: a split produced a census with no segment; nothing was renamed \
UNCOVERED LINE crates/pull/src/census_store.rs:2447: into place",
UNCOVERED LINE crates/pull/src/census_store.rs:2448: staging.display()
UNCOVERED LINE crates/pull/src/census_store.rs:2449: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2471: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2472: "{} did not read back as written: {} keys {} rows {} entries \
UNCOVERED LINE crates/pull/src/census_store.rs:2473: capacity {} against {} {} {} {}; nothing was renamed into place",
UNCOVERED LINE crates/pull/src/census_store.rs:2474: path.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:2475: back.keys(),
UNCOVERED LINE crates/pull/src/census_store.rs:2476: back.total_rows(),
UNCOVERED LINE crates/pull/src/census_store.rs:2477: back.entries(),
UNCOVERED LINE crates/pull/src/census_store.rs:2478: back.capacity(),
UNCOVERED LINE crates/pull/src/census_store.rs:2479: part.keys(),
UNCOVERED LINE crates/pull/src/census_store.rs:2480: part.total_rows(),
UNCOVERED LINE crates/pull/src/census_store.rs:2481: part.entries(),
UNCOVERED LINE crates/pull/src/census_store.rs:2482: part.capacity()
UNCOVERED LINE crates/pull/src/census_store.rs:2483: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2588: return Some(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2589: "{} is {} bytes; it was {recorded} when {} was migrated and sealed, so \
UNCOVERED LINE crates/pull/src/census_store.rs:2590: the superseded census was written after the seal",
UNCOVERED LINE crates/pull/src/census_store.rs:2591: superseded.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:2592: kept.len(),
UNCOVERED LINE crates/pull/src/census_store.rs:2593: vendor.as_str()
UNCOVERED LINE crates/pull/src/census_store.rs:2594: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2746: return Some(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2747: "{} could not be read ({why}); {}'s census is refused rather than guessed \
UNCOVERED LINE crates/pull/src/census_store.rs:2748: at (DB-B68)",
UNCOVERED LINE crates/pull/src/census_store.rs:2749: at.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:2750: vendor.as_str()
UNCOVERED LINE crates/pull/src/census_store.rs:2751: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2858: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:2859: "{} could not be created for the requirement; nothing was written",
UNCOVERED LINE crates/pull/src/census_store.rs:2860: at.display()
UNCOVERED LINE crates/pull/src/census_store.rs:2861: ));
UNCOVERED LINE crates/pull/src/census_store.rs:2867: return Ok(false);
UNCOVERED LINE crates/pull/src/census_store.rs:2916: Err(why) => return Err(format!("{}: {why}", at.display())),
UNCOVERED LINE crates/pull/src/census_store.rs:3165: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3167: receipt.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3169: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3197: return Err(format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3198: "{} is {} bytes and not a plain file of the {recorded} bytes the migration \
UNCOVERED LINE crates/pull/src/census_store.rs:3199: recorded; nothing was written",
UNCOVERED LINE crates/pull/src/census_store.rs:3200: legacy.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3201: held.len()
UNCOVERED LINE crates/pull/src/census_store.rs:3202: ));
UNCOVERED LINE crates/pull/src/census_store.rs:3208: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3210: legacy.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3211: superseded.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3213: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3247: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3249: staged.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3250: legacy.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3252: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3255: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3257: staged.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3258: legacy.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3259: legacy.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3261: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3418: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3421: path.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3422: aside.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3424: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3461: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3463: staged.display(),
UNCOVERED LINE crates/pull/src/census_store.rs:3464: path.display()
UNCOVERED LINE crates/pull/src/census_store.rs:3466: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3658: stray(&mut found, exchange_path);
UNCOVERED LINE crates/pull/src/census_store.rs:3659: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:3663: stray(&mut found, segment_path);
UNCOVERED LINE crates/pull/src/census_store.rs:3664: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:3671: stray(&mut found, symbol_path);
UNCOVERED LINE crates/pull/src/census_store.rs:3672: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:3701: }
UNCOVERED LINE crates/pull/src/census_store.rs:3709: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3714: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3730: format!(
UNCOVERED LINE crates/pull/src/census_store.rs:3734: })?;
UNCOVERED LINE crates/pull/src/census_store.rs:3736: }
UNCOVERED LINE crates/pull/src/census_store.rs:3768: None => stray(walk.found, rung_path),
UNCOVERED LINE crates/pull/src/census_store.rs:3771: } else {
UNCOVERED LINE crates/pull/src/census_store.rs:3772: stray(walk.found, path);
UNCOVERED LINE crates/pull/src/census_store.rs:3773: }
UNCOVERED LINE crates/pull/src/census_store.rs:3809: {
UNCOVERED LINE crates/pull/src/census_store.rs:3810: stray(walk.found, path);
UNCOVERED LINE crates/pull/src/census_store.rs:3819: stray(walk.found, path);
UNCOVERED LINE crates/pull/src/census_store.rs:3820: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:3827: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:3840: walk.found.slot_damaged.push((path, audit));
UNCOVERED LINE crates/pull/src/census_store.rs:3843: let _ = walk.unreadable_months.insert(month);
UNCOVERED LINE crates/pull/src/census_store.rs:3847: walk.found.files_empty += 1;
UNCOVERED LINE crates/pull/src/census_store.rs:3848: }
UNCOVERED LINE crates/pull/src/census_store.rs:3895: return Ok((None, file.slot_audit()));
UNCOVERED LINE crates/pull/src/census_store.rs:3933: return None;
UNCOVERED LINE crates/pull/src/census_store.rs:3940: fn stray(found: &mut Recount, path: PathBuf) {
UNCOVERED LINE crates/pull/src/census_store.rs:3941: found.not_store_names += 1;
UNCOVERED LINE crates/pull/src/census_store.rs:3942: if found.not_store_named.len() < NAMED {
UNCOVERED LINE crates/pull/src/census_store.rs:3943: found.not_store_named.push(path);
UNCOVERED LINE crates/pull/src/census_store.rs:3944: }
UNCOVERED LINE crates/pull/src/census_store.rs:3945: }
UNCOVERED LINE crates/pull/src/census_store.rs:3957: Err(why) => return Err(format!("{} could not be listed: {why}", dir.display())),
UNCOVERED LINE crates/pull/src/census_store.rs:4173: Err(why) => return Err(format!("{}: {why}", path.display())),
UNCOVERED LINE crates/pull/src/census_store.rs:4207: continue;
UNCOVERED LINE crates/pull/src/census_store.rs:4238: break;
UNCOVERED LINE crates/pull/src/census_store.rs:4287: _ => None,
UNCOVERED LINE crates/pull/src/census_store.rs:4455: "refused" => KeyState::Refused,
UNCOVERED LINE crates/pull/src/census_store.rs:4456: _ => return Err(foreign()),
UNCOVERED LINE crates/pull/src/census_store.rs:4493: }
UNCOVERED BRANCH crates/pull/src/census_store.rs:667:8 True: if size > cap {
UNCOVERED BRANCH crates/pull/src/census_store.rs:970:35 True: if !appends.is_empty() || room.carried.is_some() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1022:16 False: let keep = cycle.degraded.is_none()
UNCOVERED BRANCH crates/pull/src/census_store.rs:1023:12 False: && cycle.compacted.is_none()
UNCOVERED BRANCH crates/pull/src/census_store.rs:1179:8 True: if target.segment.is_none() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1254:9 False: let Some(segment) = target.segment else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1421:16 False: if let Some(parent) = dir.parent() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1719:12 False: if published.is_empty() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1924:25 False: Ok(meta) if meta.is_file() => Stamp::of(&meta),
UNCOVERED BRANCH crates/pull/src/census_store.rs:1957:13 False: let Ok(file) = store::safe_open::open_read(&self.loaded.path) else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1960:13 False: let Ok(meta) = file.metadata() else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1971:12 True: if file.read_exact_at(&mut region, 0).is_err() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:1974:12 True: if newest_committed(&region) != Some(self.loaded.census.header()) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2068:54 False: Err(ManifestError::TooManyEntries { .. }) if census.segment().is_some() => {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2095:50 True: if target.segment.is_some() && (compacted || repairing) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2100:22 False: let why = if compacted { "full" } else { "repair" };
UNCOVERED BRANCH crates/pull/src/census_store.rs:2184:24 False: if let Some(epoch) = current_epoch(root, vendor, segment)? {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2290:8 True: if vendor != Vendor::Gdfl {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2301:12 False: if let Some(hold) = hold {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2303:16 True: if let Err(why) = hold.release() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2350:8 True: if keys != census.keys() || rows != census.total_rows() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2350:33 True: if keys != census.keys() || rows != census.total_rows() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2444:13 False: let Some(segment) = part.segment() else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2465:12 True: if back.keys() != part.keys()
UNCOVERED BRANCH crates/pull/src/census_store.rs:2466:16 True: || back.total_rows() != part.total_rows()
UNCOVERED BRANCH crates/pull/src/census_store.rs:2467:16 True: || back.entries() != part.entries()
UNCOVERED BRANCH crates/pull/src/census_store.rs:2468:16 True: || back.capacity() != part.capacity()
UNCOVERED BRANCH crates/pull/src/census_store.rs:2469:16 True: || back.degraded_reason().is_some()
UNCOVERED BRANCH crates/pull/src/census_store.rs:2587:12 True: if kept.len() != recorded {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2745:12 True: if let Err(why) = file.take(REQUIRES_CAP + 1).read_to_end(&mut bytes) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2857:9 False: let Some(mut file) = open_requires(&at, true)? else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2866:8 True: if names(&held) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2908:21 False: Err(why) if !append && why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
UNCOVERED BRANCH crates/pull/src/census_store.rs:2909:21 False: Err(why) if why.raw_os_error() == Some(store::open_flags::ELOOP) => {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2995:16 False: if let Some(magic) = image.get_mut(header) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:2998:16 False: if let Some(version) = image.get_mut(slot + STUB_VERSION_AT..slot + STUB_VERSION_AT + 2) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3001:16 False: if let Some(text) = image.get_mut(slot + 64..slot + 64 + note.len()) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3037:5 False: file.read_exact(&mut head).is_ok()
UNCOVERED BRANCH crates/pull/src/census_store.rs:3038:12 False: && head.starts_with(b"BRUTEXM")
UNCOVERED BRANCH crates/pull/src/census_store.rs:3075:27 False: .is_ok_and(|held| held.dev() == kept.dev() && held.ino() == kept.ino())
UNCOVERED BRANCH crates/pull/src/census_store.rs:3180:38 False: if holds_sealed_stub(&legacy) && fs::symlink_metadata(&superseded).is_ok() {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3184:21 True: let wrote = require_feature(root, vendor, FEATURE_CENSUS_V4)? || receipt_written;
UNCOVERED BRANCH crates/pull/src/census_store.rs:3196:8 True: if !held.is_file() || held.len() != recorded {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3196:27 True: if !held.is_file() || held.len() != recorded {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3361:29 False: Some(kept_bytes) if value("superseded").is_some_and(|kept| !kept.is_empty()) => {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3580:16 False: && self.slot_damaged.is_empty()
UNCOVERED BRANCH crates/pull/src/census_store.rs:3581:16 False: && self.refused() == 0
UNCOVERED BRANCH crates/pull/src/census_store.rs:3657:13 False: let Ok(exchange) = Exchange::parse(&exchange_name) else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3662:17 False: let Ok(segment) = Segment::parse(&segment_name) else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3667:21 False: let Some(symbol) = Symbol::new(&symbol_name)
UNCOVERED BRANCH crates/pull/src/census_store.rs:3696:8 False: if vendor == Vendor::Gdfl {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3717:8 False: if vendor == Vendor::Gdfl {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3764:23 False: } else if let Some(contract) = Contract::parse(&name).filter(|c| c.as_str() == name) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3806:20 True: if !FileKind::ALL
UNCOVERED BRANCH crates/pull/src/census_store.rs:3818:13 False: let Some(month) = month_named(stem) else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3826:12 True: if !walk.span.holds(month) {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3839:24 True: if let Some(audit) = named {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3842:20 True: if reverted {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3844:31 False: } else if let Some(row) = row {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3853:20 False: if walk.found.unreadable_named.len() < NAMED {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3894:8 True: if header.n_valid == 0 {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3932:8 True: if year.len() != 4 || month.len() != 2 {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3932:27 True: if year.len() != 4 || month.len() != 2 {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3942:8 True: if found.not_store_named.len() < NAMED {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3942:8 False: if found.not_store_named.len() < NAMED {
UNCOVERED BRANCH crates/pull/src/census_store.rs:3956:24 False: Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
UNCOVERED BRANCH crates/pull/src/census_store.rs:4172:24 False: Err(absent) if absent.kind() == ErrorKind::NotFound => String::new(),
UNCOVERED BRANCH crates/pull/src/census_store.rs:4206:13 False: let (Some(kind), Some(day)) = (words.next(), words.next().and_then(parse_day)) else {
UNCOVERED BRANCH crates/pull/src/census_store.rs:4237:12 True: if *held != id {
UNCOVERED BRANCH crates/pull/src/census_store.rs:4415:8 True: if y.len() != 4 || m.len() != 2 || d.len() != 2 {
UNCOVERED BRANCH crates/pull/src/census_store.rs:4415:24 True: if y.len() != 4 || m.len() != 2 || d.len() != 2 {
UNCOVERED BRANCH crates/pull/src/census_store.rs:4450:32 False: "begin" if rest.is_empty() => KeyState::Open,
UNCOVERED BRANCH crates/pull/src/census_store.rs:4485:8 False: if file.metadata().map_err(refused)?.len() > 0 {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:675: &mut |_: WholeStep| Ok(()),
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:729: &mut |_: WholeStep| Ok(()),
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:808: // Release the open that waits, so the suite does not hang; then fail.
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:809: let _ = std::fs::OpenOptions::new().write(true).open(&stub);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:850: return false;
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:853: return false;
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1029: fn the_kept_publish_is_flat_measured() {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1032: let base = std::env::var_os("R319_BENCH_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1033: let mut p50s = Vec::new();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1034: for keys in [400_000_u32, 2_000_000, 4_000_000, 8_400_000] {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1035: let root = base.join(format!("brutex-r319-bench-{keys}-{}", std::process::id()));
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1036: std::fs::create_dir_all(root.join("manifest")).expect("a bench root");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1037: let built = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1038: let capacity = u64::from(keys) + u64::from(N * FRESH) + 4_096;
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1039: let mut census =
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1040: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1041: for n in 0..keys {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1042: census.record_held(row(n, 1)).expect("held");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1043: }
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1044: let path = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1045: std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1046: std::fs::write(&path, census.image()).expect("epoch 1");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1047: drop(census);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1048: let built = built.elapsed().as_millis();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1049: let mut kept = Kept::new();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1050: let whole = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1051: let first = record_kept(
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1052: &mut kept,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1053: &root,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1054: Vendor::Gdfl,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1055: &(0..FRESH).map(|i| row(keys + i, 1)).collect::<Vec<_>>(),
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1057: .expect("day 1");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1058: let whole = whole.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1059: assert_eq!(first[0].read, CensusRead::Whole("first"));
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1060: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1061: for d in 1..=N {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1062: let rows: Vec<Held> = (0..FRESH).map(|i| row(keys + d * FRESH + i, 1)).collect();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1063: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1064: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1065: micros.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1066: assert_eq!(got[0].read, CensusRead::Kept, "{keys}: day {d}");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1068: let (p50, p99, max) = percentiles(&mut micros);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1069: eprintln!(
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1073: std::fs::metadata(&path).map_or(0, |m| m.len())
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1075: p50s.push(p50);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1076: drop(kept);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1077: let _ = std::fs::remove_dir_all(&root);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1079: let (small, large) = (p50s[0], p50s[3]);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1080: eprintln!("R3-19 ratio p50(8.4M) / p50(0.4M) = {large} / {small}");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1081: assert!(
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1082: large <= small.saturating_mul(3).max(1),
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1085: }
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1096: fn the_kept_publish_on_a_copy_of_a_real_epoch_measured() {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1099: let root = PathBuf::from(std::env::var_os("R319_REAL_ROOT").expect("R319_REAL_ROOT"));
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1100: let invented = |n: u32| {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1101: let mut held = row(n, 1);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1102: held.entry.key.symbol = Symbol::new("INVENTEDR").expect("a symbol");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1103: held
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1104: };
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1105: let mut kept = Kept::new();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1106: let whole = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1107: let first = record_kept(
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1108: &mut kept,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1109: &root,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1110: Vendor::Gdfl,
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1111: &(0..FRESH).map(invented).collect::<Vec<_>>(),
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1113: .expect("day 1");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1114: let whole = whole.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1115: assert_eq!(first[0].read, CensusRead::Whole("first"));
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1116: let entries = first[0].entries;
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1117: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1118: for d in 1..=N {
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1119: let rows: Vec<Held> = (0..FRESH).map(|i| invented(d * FRESH + i)).collect();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1120: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1121: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1122: micros.push(start.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1123: assert_eq!(got[0].read, CensusRead::Kept, "day {d}");
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1125: let (p50, p99, max) = percentiles(&mut micros);
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1126: eprintln!(
UNCOVERED LINE crates/pull/src/census_store_kept_tests.rs:1131: }
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:607:41 False: Err(why) if why.contains("another ingest holds the census lock") => {
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:755:9 False: why.contains(&vendor_dir(root, Vendor::Gdfl).display().to_string())
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:756:16 False: && why.contains("is not a directory"),
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:807:8 True: if got.is_err() {
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:849:9 False: let Ok(meta) = std::fs::symlink_metadata(path) else {
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:852:8 True: if !meta.is_file() || meta.len() != sealed_stub(Vendor::Gdfl).len() as u64 {
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:852:27 True: if !meta.is_file() || meta.len() != sealed_stub(Vendor::Gdfl).len() as u64 {
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:856:5 False: std::fs::File::open(path)
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:963:17 False: assert!(meta.is_file() && meta.nlink() == 1);
UNCOVERED BRANCH crates/pull/src/census_store_kept_tests.rs:963:35 False: assert!(meta.is_file() && meta.nlink() == 1);
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:205: target.display()
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:361: assert!(why.contains("Permission denied"), "{why}");
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:362: assert!(why.contains(&tmp_two(root).display().to_string()), "{why}");
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:363: assert_eq!(inode(&one), before);
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:364: nothing_installed(root);
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:365: }
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:692: assert!(why.contains("ermission denied"), "the host's words: {why}");
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:693: assert_eq!(std::fs::read(&at).expect("unchanged"), before);
UNCOVERED LINE crates/pull/src/census_store_link_tests.rs:694: }
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:286:17 False: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:286:35 False: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:336:13 False: left.is_file() && left.nlink() == 1,
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:336:31 False: left.is_file() && left.nlink() == 1,
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:660:13 False: assert!(meta.is_file() && meta.nlink() == 1);
UNCOVERED BRANCH crates/pull/src/census_store_link_tests.rs:660:31 False: assert!(meta.is_file() && meta.nlink() == 1);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:305: done.refused[0].1
UNCOVERED LINE crates/pull/src/census_store_tests.rs:585: done.left_open[2]
UNCOVERED LINE crates/pull/src/census_store_tests.rs:975: flushed += 1;
UNCOVERED LINE crates/pull/src/census_store_tests.rs:976: Ok(())
UNCOVERED LINE crates/pull/src/census_store_tests.rs:977: })
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1027: together.push((both, new, "two slot writes"));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1032: together.push((both, new, "entries and a slot"));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1043: cases.push(flying.clone());
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1062: let loaded =
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1063: loads(&torn).map(|header| (header.n_valid, header.generation));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1064: behind.push((both, new, case.clone(), cut, loaded, held.generation));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1079: format!(
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1083: })
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1139: changed.reverse();
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1321: seal_receipt(&at)
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1803: fn legacy_drift_cost_per_call_is_measured() {
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1804: let scratch = Scratch::new("DRIFT-TIMING");
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1805: assert_eq!(
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1806: record(scratch.root(), Vendor::Gdfl, &[row(0, 1)]).map(|c| c.len()),
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1809: migrate(scratch.root(), Vendor::Gdfl).expect("it migrates");
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1810: let real = std::env::var_os("BRUTEX_STORE_ROOT").map(PathBuf::from);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1811: let mut roots: Vec<(&str, &Path)> = vec![("scratch", scratch.root())];
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1812: if let Some(real) = real.as_deref() {
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1813: roots.push(("store", real));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1814: }
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1815: for (label, root) in roots {
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1816: let want = legacy_drift(root, Vendor::Gdfl);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1817: let mut micros: Vec<u128> = Vec::with_capacity(400);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1818: for _ in 0..400 {
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1819: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1820: let got = legacy_drift(root, Vendor::Gdfl);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1821: micros.push(started.elapsed().as_micros());
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1822: assert_eq!(got, want, "{label}: the answer does not move between calls");
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1824: micros.sort_unstable();
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1825: let at = |q: usize| micros.get(399 * q / 100).copied().unwrap_or(u128::MAX);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1826: let max = micros.last().copied().unwrap_or(u128::MAX);
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1827: eprintln!(
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1829: root.display(),
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1830: at(50),
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1831: at(99)
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1833: assert!(at(99) < 20_000, "{label}: p99 {}us", at(99));
UNCOVERED LINE crates/pull/src/census_store_tests.rs:1835: }
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:608:13 False: assert!(again.closed.is_empty() && again.run.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:608:40 False: assert!(again.closed.is_empty() && again.run.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:717:13 False: assert!(again.closed.is_empty() && again.run.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:717:40 False: assert!(again.closed.is_empty() && again.run.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:816:13 False: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:816:41 False: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:817:13 False: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:817:42 False: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:821:13 False: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:821:40 False: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1026:20 True: if flying.len() > 1 {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1031:24 True: if !flying.is_empty() {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1042:20 True: if flying.len() > 1 {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1059:29 False: header.generation >= held.generation && committed.contains(&header)
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1061:28 True: if !ok {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1086:9 False: together.is_empty() && torn_behind.is_empty(),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1086:32 False: together.is_empty() && torn_behind.is_empty(),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1138:16 True: if was.generation.is_multiple_of(2) {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1161:12 False: if let Some(first) = appends.first_mut() {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1196:13 False: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1196:49 False: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1335:17 False: assert!(!again.wrote && again.torn_receipt.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1335:33 False: assert!(!again.wrote && again.torn_receipt.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1344:13 False: assert!(seal.wrote && seal.torn_receipt.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1344:27 False: assert!(seal.wrote && seal.torn_receipt.is_none());
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1358:9 False: why.contains("SEALED")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1359:16 False: && why.contains("census-seal-v2")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1360:16 False: && why.contains("nothing was written"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1370:9 False: why.contains("SEALED") && why.contains("not a regular file"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1370:35 False: why.contains("SEALED") && why.contains("not a regular file"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1505:9 False: why.contains("manifest/REQUIRES")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1506:16 False: && why.contains("line 3")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1507:16 False: && why.contains("\"census-v5\"")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1508:16 False: && why.contains("this build does not have")
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1509:16 False: && why.contains("DB-B68"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1527:9 False: every.contains("line 4") && every.contains("\"bars-v4\""),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1527:37 False: every.contains("line 4") && every.contains("\"bars-v4\""),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1778:9 False: unread.contains("MIGRATED") && unread.contains("cannot be read"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1778:40 False: unread.contains("MIGRATED") && unread.contains("cannot be read"),
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1812:12 True: if let Some(real) = real.as_deref() {
UNCOVERED BRANCH crates/pull/src/census_store_tests.rs:1812:12 False: if let Some(real) = real.as_deref() {
UNCOVERED LINE crates/pull/src/chain.rs:344: chain.unreadable.push(format!(
UNCOVERED LINE crates/pull/src/chain.rs:348: continue;
UNCOVERED LINE crates/pull/src/chain.rs:352: url: url.clone(),
UNCOVERED LINE crates/pull/src/chain.rs:353: why: why.to_string(),
UNCOVERED LINE crates/pull/src/chain.rs:354: })?;
UNCOVERED LINE crates/pull/src/chain.rs:430: return None;
UNCOVERED LINE crates/pull/src/chain.rs:516: panic!("Dhan publishes no contract name to discover");
UNCOVERED LINE crates/pull/src/chain.rs:539: panic!("an archive has no vendor to publish a lookup");
UNCOVERED LINE crates/pull/src/chain.rs:592: return Err(Refusal::transport("no answer left".to_owned()));
UNCOVERED LINE crates/pull/src/chain.rs:620: seen[0]
UNCOVERED LINE crates/pull/src/chain.rs:721: chain.unreadable[0]
UNCOVERED LINE crates/pull/src/chain.rs:806: return Err(Refusal::transport("no answer left".to_owned()));
UNCOVERED LINE crates/pull/src/chain.rs:828: from.asked.borrow()
UNCOVERED LINE crates/pull/src/chain.rs:856: panic!("nothing answered, so the walk cannot have succeeded");
UNCOVERED BRANCH crates/pull/src/chain.rs:343:13 False: let Some(keyed_expiry) = iso_expiry(&keyed.expiry) else {
UNCOVERED BRANCH crates/pull/src/chain.rs:429:8 True: if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
UNCOVERED BRANCH crates/pull/src/chain.rs:429:29 True: if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
UNCOVERED BRANCH crates/pull/src/chain.rs:429:60 True: if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
UNCOVERED BRANCH crates/pull/src/chain.rs:507:13 False: let Err(why) = month(
UNCOVERED BRANCH crates/pull/src/chain.rs:530:13 False: let Err(why) = month(
UNCOVERED BRANCH crates/pull/src/chain.rs:591:20 True: if left.is_empty() {
UNCOVERED BRANCH crates/pull/src/chain.rs:717:13 False: chain.unreadable[0].contains("FINNIFTY")
UNCOVERED BRANCH crates/pull/src/chain.rs:718:20 False: && chain.unreadable[0].contains("NIFTY was asked for"),
UNCOVERED BRANCH crates/pull/src/chain.rs:805:20 True: if left.is_empty() {
UNCOVERED BRANCH crates/pull/src/chain.rs:847:13 False: let Err(why) = month(
UNCOVERED LINE crates/pull/src/corporate_actions_cov_tests.rs:177: refused(&backwards)
UNCOVERED LINE crates/pull/src/corporate_actions_cov_tests.rs:408: panic!("{found:#?}");
UNCOVERED BRANCH crates/pull/src/corporate_actions_cov_tests.rs:134:9 False: said.contains("\"BCO\"") && said.contains("share-days"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_cov_tests.rs:134:37 False: said.contains("\"BCO\"") && said.contains("share-days"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_cov_tests.rs:407:9 False: let Finding::FactorDiffers { step, product } = &found[0] else {
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:265: panic!("the runs load from the store");
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:293: panic!("the ledger loads from the store");
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:368: rights[0].ratio
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:584: panic!("a swapped table is never served");
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:663: refused(&twice)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:697: refused(&overlap)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:866: refused(&backwards)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:879: refused(&kinds)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:892: refused(&symbols)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:905: refused(&split_up)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:914: refused(&runs_back)
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:1189: alt_steps.iter().map(|j| j.verdict).collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:1265: found[0]
UNCOVERED LINE crates/pull/src/corporate_actions_tests.rs:1307: rows[0].evidence
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:159:13 False: assert!(r("4765/10000") <= factor && factor <= r("4767/10000"));
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:159:42 False: assert!(r("4765/10000") <= factor && factor <= r("4767/10000"));
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:264:9 False: let Slot::Store { table: loaded, .. } = load_runs(&store) else {
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:287:9 False: let Slot::Store {
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:366:9 False: r("104/100") < rights[0].ratio && rights[0].ratio < r("106/100"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:366:43 False: r("104/100") < rights[0].ratio && rights[0].ratio < r("106/100"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:409:9 False: said.contains("SPLITCO")
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:410:16 False: && said.contains("(2023-06-30, 2023-07-03]")
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:411:16 False: && said.contains("factor 2/1"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:426:75 False: assert!(matches!(&found[0], Finding::FactorDiffers { product, .. } if *product == r("5")));
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:519:13 False: assert!(!measured.excuses() && measured.class().is_none());
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:519:36 False: assert!(!measured.excuses() && measured.class().is_none());
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:583:9 False: let Slot::Refused(why) = load_actions(&store) else {
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:592:9 False: said.contains("corporate_actions") && said.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:592:47 False: said.contains("corporate_actions") && said.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1077:57 False: assert!(matches!(&found[0], Finding::NoRow(step) if step.symbol == "REALCO"));
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1216:9 False: said.contains("basis_anomaly SEGCO (2022-01-28, 2022-01-31]")
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1217:16 False: && said.contains(
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1292:63 False: Verdict::Anomaly(Anomaly::WithinWobble { wobble }) if wobble > r("1012/1000")
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1305:9 False: rows[0].evidence.contains("lasting (") && rows[0].day_known(),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1305:51 False: rows[0].evidence.contains("lasting (") && rows[0].day_known(),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1312:78 False: Finding::BasisAnomaly { step, why: Anomaly::WithinWobble { .. } } if step.symbol == "WOBCO"
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1317:58 False: .any(|f| matches!(f, Finding::NoRow(step) if step.symbol == "LASTCO"))
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1476:13 False: assert!(tenfold.step.low <= r("472/47") && r("472/47") <= tenfold.step.high);
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1476:48 False: assert!(tenfold.step.low <= r("472/47") && r("472/47") <= tenfold.step.high);
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1492:13 False: assert!(tenfold.step.low <= row.ratio && row.ratio <= tenfold.step.high);
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1492:46 False: assert!(tenfold.step.low <= row.ratio && row.ratio <= tenfold.step.high);
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1546:13 False: assert!(r("1007/1000") < own && own < r("11/10"), "{own}");
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1546:37 False: assert!(r("1007/1000") < own && own < r("11/10"), "{own}");
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1589:22 False: .all(|j| !j.step.up() && j.step.far() < judged[0].step.near()),
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1752:9 False: said.contains("product_too_large SPLITCO (2023-06-30, 2023-07-03]: the 2 rows there")
UNCOVERED BRANCH crates/pull/src/corporate_actions_tests.rs:1753:16 False: && !said.contains("multiply to 1/1"),
UNCOVERED LINE crates/pull/src/csv.rs:1157: format!("{year:04}-{month:02}-{day:02} 00:00:00")
UNCOVERED LINE crates/pull/src/csv.rs:1505: DateFormat::DashedYmdMidnight => 19,
UNCOVERED LINE crates/pull/src/csv.rs:1889: columns.count()
UNCOVERED BRANCH crates/pull/src/csv.rs:1513:50 False: assert!(!text.is_char_boundary(4) || !text.is_ascii());
UNCOVERED BRANCH crates/pull/src/csv.rs:1936:66 False: Err(CsvError::HeaderMismatch { ref got, .. }) if *got == swapped
UNCOVERED BRANCH crates/pull/src/csv.rs:1945:62 False: Err(CsvError::HeaderMismatch { ref got, .. }) if got.is_empty()
UNCOVERED BRANCH crates/pull/src/day_universe.rs:110:16 False: if let Some((_, why)) = series.get_mut(slot) {
UNCOVERED BRANCH crates/pull/src/day_universe.rs:197:17 False: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:197:44 False: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:197:73 False: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:200:17 False: assert!(twice.entrants.is_empty() && twice.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:200:46 False: assert!(twice.entrants.is_empty() && twice.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:209:17 False: assert!(got.entrants.is_empty() && got.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/day_universe.rs:209:44 False: assert!(got.entrants.is_empty() && got.leavers.is_empty());
UNCOVERED BRANCH crates/pull/src/daycheck.rs:144:16 False: if self.first_truncated.is_none() {
UNCOVERED BRANCH crates/pull/src/daycheck.rs:282:28 False: && let Some(bar) = rest.first()
UNCOVERED BRANCH crates/pull/src/daycheck.rs:509:17 False: assert!(report.consistent() && !report.clean());
UNCOVERED BRANCH crates/pull/src/daycheck.rs:509:40 False: assert!(report.consistent() && !report.clean());
UNCOVERED BRANCH crates/pull/src/disk_projection.rs:261:20 False: if let Some(list) = per_class.get_mut(class.slot()) {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:391: fn real_delivery_sizes_project_through_the_module_and_meet_the_floor() {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:392: let read = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:393: let unit_of = |text: &str| {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:394: if let Some(rest) = text.strip_prefix("U:") {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:395: let (name, class) = rest.split_once('@').unwrap();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:396: let class = if class == "index" {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:397: OptionClass::Index
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:399: OptionClass::Stock
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:401: Unit::Underlying {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:402: name: name.to_owned(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:403: class,
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:404: }
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:406: let mut words = text.split('/');
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:407: let (feed, k, rung) = (
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:408: words.next().unwrap(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:409: words.next().unwrap(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:410: words.next().unwrap(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:411: );
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:412: Unit::Kind(KindKey::new(feed, k, rung))
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:414: };
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:415: let items = |name: &str| -> Vec<(Unit, String)> {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:416: read(name)
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:417: .split(';')
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:418: .filter(|s| !s.is_empty())
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:419: .map(|item| {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:420: let (key, value) = item.split_once('=').unwrap();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:421: (unit_of(key), value.to_owned())
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:422: })
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:423: .collect()
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:424: };
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:425: let mut totals = Totals::new();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:426: for (unit, value) in items("BRUTEX_REAL_SAMPLE") {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:427: let (bytes, days) = value.split_once(':').unwrap();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:428: let (bytes, days) = (bytes.parse().unwrap(), days.parse().unwrap());
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:429: match unit {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:430: Unit::Kind(key) => totals.add_kind(key, bytes, days).unwrap(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:431: Unit::Underlying { name, class } => {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:432: totals.add_underlying(&name, class, bytes, days).unwrap();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:433: }
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:436: let sealed = totals.seal();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:437: let plan: Vec<PlanItem> = items("BRUTEX_REAL_PLAN")
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:438: .into_iter()
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:439: .map(|(unit, days)| PlanItem {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:440: unit,
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:441: days: days.parse().unwrap(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:443: })
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:444: .collect();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:445: let projection = project(&sealed, &plan);
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:446: for line in &projection.lines {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:447: println!(
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:451: assert_eq!(line.bytes, line.per_day * line.days, "days x bytes a day");
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:452: if let Basis::Extrapolated { class, .. } = line.basis {
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:453: assert_eq!(Some(line.per_day), sealed.median_of(class).map(|(m, _)| m));
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:454: assert!(
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:455: projection.extrapolated.contains(&line.unit.to_string()),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:458: }
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:460: println!("{}", projection.summary());
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:461: assert!(
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:462: projection.unmeasured.is_empty(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:465: let space = parse_df(&read("BRUTEX_REAL_DF")).unwrap();
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:466: let verdict = floor_check(space, projection.bytes, DEFAULT_FLOOR);
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:467: println!(
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:471: DEFAULT_FLOOR.bytes_of(space.total)
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:473: let fits = space.free
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:474: >= projection
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:475: .bytes
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:476: .saturating_add(DEFAULT_FLOOR.bytes_of(space.total));
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:477: assert_eq!(
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:478: verdict.is_ok(),
UNCOVERED LINE crates/pull/src/disk_projection_tests.rs:482: }
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:394:16 True: if let Some(rest) = text.strip_prefix("U:") {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:394:16 False: if let Some(rest) = text.strip_prefix("U:") {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:396:28 True: let class = if class == "index" {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:396:28 False: let class = if class == "index" {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:452:16 True: if let Basis::Extrapolated { class, .. } = line.basis {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:452:16 False: if let Basis::Extrapolated { class, .. } = line.basis {
UNCOVERED BRANCH crates/pull/src/disk_projection_tests.rs:546:83 False: matches!(refused, FloorRefusal::NeedOverflows { projected: p, .. } if p == projected),
UNCOVERED LINE crates/pull/src/emit_sites.rs:448: return;
UNCOVERED LINE crates/pull/src/emit_sites.rs:1556: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/emit_sites.rs:1601: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/emit_sites.rs:1630: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/emit_sites.rs:1659: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/emit_sites.rs:1727: matches!(
UNCOVERED LINE crates/pull/src/emit_sites.rs:2028: panic!("this feed authenticates, so its transport is HTTP")
UNCOVERED LINE crates/pull/src/emit_sites.rs:2126: landed
UNCOVERED LINE crates/pull/src/emit_sites.rs:2127: .iter()
UNCOVERED LINE crates/pull/src/emit_sites.rs:2128: .map(|record| (&record.message, record.field(key)))
UNCOVERED LINE crates/pull/src/emit_sites.rs:2129: .collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/emit_sites.rs:2176: record.message != "runs ledger behind its claims"
UNCOVERED LINE crates/pull/src/emit_sites.rs:2177: && record.message != "runs claims tail damaged"
UNCOVERED LINE crates/pull/src/emit_sites.rs:2178: }),
UNCOVERED BRANCH crates/pull/src/emit_sites.rs:447:13 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/emit_sites.rs:2176:17 True: record.message != "runs ledger behind its claims"
UNCOVERED BRANCH crates/pull/src/emit_sites.rs:2176:17 False: record.message != "runs ledger behind its claims"
UNCOVERED BRANCH crates/pull/src/emit_sites.rs:2319:22 False: .any(|f| f.why.contains("DEGRADED") || f.why.contains("degraded")),
UNCOVERED BRANCH crates/pull/src/emit_sites.rs:2411:12 False: if let Some(parent) = path.parent() {
UNCOVERED LINE crates/pull/src/expiry_alias.rs:919: && matches!(
UNCOVERED LINE crates/pull/src/expiry_alias.rs:1101: matches!(
UNCOVERED LINE crates/pull/src/expiry_alias.rs:1250: panic!("aliased")
UNCOVERED LINE crates/pull/src/expiry_alias.rs:1274: panic!("aliased")
UNCOVERED LINE crates/pull/src/expiry_alias.rs:1607: panic!("{under} {stated}: {:?}", old.settlement);
UNCOVERED LINE crates/pull/src/expiry_alias.rs:1659: panic!("{:?}", old.settlement);
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1108:21 False: assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1108:33 False: assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1109:21 False: assert!(first <= settled && first < stated, "{row:?}: first day");
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1109:41 False: assert!(first <= settled && first < stated, "{row:?}: first day");
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1182:55 True: !(a.underlying == b.underlying && a.stated == b.stated),
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1249:13 False: let Settlement::Aliased(row) = chain.settlement else {
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1273:13 False: let Settlement::Aliased(row) = got.settlement else {
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1429:17 False: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1429:38 False: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1429:57 False: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1606:17 False: let Settlement::Aliased(row) = old.settlement else {
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1658:21 False: let Settlement::ExpiredEarly(row) = old.settlement else {
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1697:21 False: assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
UNCOVERED BRANCH crates/pull/src/expiry_alias.rs:1697:49 False: assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
UNCOVERED LINE crates/pull/src/fetch.rs:518: break;
UNCOVERED LINE crates/pull/src/fetch.rs:521: break;
UNCOVERED LINE crates/pull/src/fetch.rs:870: fn note_refused(days: &mut Vec<RefusedDay>, day: u32) {
UNCOVERED LINE crates/pull/src/fetch.rs:871: match days.iter_mut().find(|held| held.day == day) {
UNCOVERED LINE crates/pull/src/fetch.rs:872: Some(held) => held.rows = held.rows.saturating_add(1),
UNCOVERED LINE crates/pull/src/fetch.rs:873: None => days.push(RefusedDay { day, rows: 1 }),
UNCOVERED LINE crates/pull/src/fetch.rs:875: }
UNCOVERED LINE crates/pull/src/fetch.rs:928: FetchError::BodyNotUnderstood {
UNCOVERED LINE crates/pull/src/fetch.rs:929: detail: why.to_string(),
UNCOVERED LINE crates/pull/src/fetch.rs:930: }
UNCOVERED LINE crates/pull/src/fetch.rs:931: })?;
UNCOVERED LINE crates/pull/src/fetch.rs:1032: Err(crate::session::SessionError::SessionLengthUnmeasured { days }) => {
UNCOVERED LINE crates/pull/src/fetch.rs:1033: note_refused(&mut refused_days, days);
UNCOVERED LINE crates/pull/src/fetch.rs:1034: continue;
UNCOVERED BRANCH crates/pull/src/fetch.rs:515:17 False: let (Some(&open), Some(&high), Some(&low), Some(&close)) =
UNCOVERED BRANCH crates/pull/src/fetch.rs:520:17 False: let (Some(&volume), Some(&timestamp)) = (a.volume.get(i), a.timestamp.get(i)) else {
UNCOVERED LINE crates/pull/src/fetch_ps1_tests.rs:123: panic!("a short array is a length disagreement");
UNCOVERED BRANCH crates/pull/src/fetch_ps1_tests.rs:113:13 False: let FetchError::LengthDisagreement {
UNCOVERED LINE crates/pull/src/fno.rs:243: continue;
UNCOVERED LINE crates/pull/src/fno.rs:299: | ParamValue::Kind => None,
UNCOVERED LINE crates/pull/src/fno.rs:349: PathSegment::Value { placeholder, .. } => {
UNCOVERED LINE crates/pull/src/fno.rs:350: return Err(format!(
UNCOVERED LINE crates/pull/src/fno.rs:351: "the path word {placeholder:?} is not one a futures request can spell"
UNCOVERED LINE crates/pull/src/fno.rs:352: ));
UNCOVERED LINE crates/pull/src/fno.rs:378: return Err(format!(
UNCOVERED LINE crates/pull/src/fno.rs:379: "the query word {:?} is not one a futures request can spell",
UNCOVERED LINE crates/pull/src/fno.rs:380: param.name
UNCOVERED LINE crates/pull/src/fno.rs:381: ));
UNCOVERED LINE crates/pull/src/fno.rs:406: panic!("Groww is an HTTP broker");
UNCOVERED LINE crates/pull/src/fno.rs:533: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/src/fno.rs:1325: None => return false,
UNCOVERED LINE crates/pull/src/fno.rs:1333: return false;
UNCOVERED LINE crates/pull/src/fno.rs:1339: return false;
UNCOVERED LINE crates/pull/src/fno.rs:1342: return false;
UNCOVERED LINE crates/pull/src/fno.rs:1345: return false;
UNCOVERED BRANCH crates/pull/src/fno.rs:242:13 False: let Some(value) = resolve(p.value, ask) else {
UNCOVERED BRANCH crates/pull/src/fno.rs:405:13 False: let Transport::Http(spec) = Feed::Groww.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/fno.rs:532:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/fno.rs:957:17 False: assert!(found.contract.is_option() && !found.contract.is_future());
UNCOVERED BRANCH crates/pull/src/fno.rs:957:47 False: assert!(found.contract.is_option() && !found.contract.is_future());
UNCOVERED BRANCH crates/pull/src/fno.rs:989:13 False: floor.contains("2019") && floor.contains("2020"),
UNCOVERED BRANCH crates/pull/src/fno.rs:989:39 False: floor.contains("2019") && floor.contains("2020"),
UNCOVERED BRANCH crates/pull/src/fno.rs:1331:9 False: let Some(rest) = rest else { return false };
UNCOVERED BRANCH crates/pull/src/fno.rs:1332:9 False: let Some(word) = rest.get(0..3).map(str::to_ascii_lowercase) else {
UNCOVERED BRANCH crates/pull/src/fno.rs:1338:9 False: let Ok(month) = u8::try_from(month + 1) else {
UNCOVERED BRANCH crates/pull/src/fno.rs:1341:9 False: let Some(Ok(year)) = rest.get(3..).map(str::parse::<u16>) else {
UNCOVERED BRANCH crates/pull/src/fno.rs:1344:9 False: let Some(year) = 2000u16.checked_add(year) else {
UNCOVERED LINE crates/pull/src/fno_membership.rs:1315: )?;
UNCOVERED LINE crates/pull/src/fno_membership.rs:2638: return Vec::new();
UNCOVERED LINE crates/pull/src/fno_membership.rs:2642: continue;
UNCOVERED LINE crates/pull/src/fno_membership.rs:2680: continue;
UNCOVERED LINE crates/pull/src/fno_membership.rs:2695: continue;
UNCOVERED LINE crates/pull/src/fno_membership.rs:2920: }
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2166:20 False: if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2193:24 False: if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2326:20 False: if let Some(slot) = usize::try_from(spelling.series_id)
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2384:28 False: if let Some(word) = out.bits.get_mut(day_bits + series_at / 64) {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2634:13 False: let (Ok(first), Ok(last)) = (
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2641:17 False: let Ok(day) = Day::from_days(epoch) else {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2679:17 False: let Ok(day) = Day::from_days(self.first_epoch.saturating_add(offset)) else {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2694:25 False: let Some(share) = self.series(id).map(|spelling| spelling.name) else {
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2761:16 False: if let Some((offset, slot)) = previous
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2914:28 False: && let Some(table) = open.take()
UNCOVERED BRANCH crates/pull/src/fno_membership.rs:2918:24 False: if work.touched {
UNCOVERED LINE crates/pull/src/fno_membership_cov_tests.rs:109: panic!("{refused:?}");
UNCOVERED BRANCH crates/pull/src/fno_membership_cov_tests.rs:108:9 False: let MembershipError::Corrupt { why, .. } = &refused else {
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:166: fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:167: unreachable!("membership is read from listed names: no tick file is fetched")
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:218: panic!("{refusal:?}");
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:234: panic!("{refusal:?}");
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:1035: panic!("an invented non-ticker decoded");
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:1060: panic!("{:?}", record.handovers);
UNCOVERED LINE crates/pull/src/fno_membership_ps1_tests.rs:1115: panic!("both faults are one Io");
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:217:9 False: let MembershipError::Corrupt { path, why } = &refusal else {
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:233:9 False: let MembershipError::Stale { path, why } = &refusal else {
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:1034:9 False: let Err(why) = decode_ticker("ZQNOTATICKER", monday) else {
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:1059:9 False: let [handover] = record.handovers[..] else {
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:1112:9 False: let MembershipError::Io { path, why } =
UNCOVERED BRANCH crates/pull/src/fno_membership_ps1_tests.rs:1131:62 False: matches!(&alone, MembershipError::Io { path, .. } if *path == nowhere),
UNCOVERED LINE crates/pull/src/fno_membership_tests.rs:78: fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
UNCOVERED LINE crates/pull/src/fno_membership_tests.rs:79: unreachable!("membership is read from listed names: no tick file is fetched")
UNCOVERED LINE crates/pull/src/fno_membership_tests.rs:265: panic!("one handover: {:?}", held.handovers());
UNCOVERED LINE crates/pull/src/fno_membership_tests.rs:1204: panic!("{reported:?}")
UNCOVERED LINE crates/pull/src/fno_membership_tests.rs:1473: matches!(
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:264:9 False: let [handover] = held.handovers() else {
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:728:16 False: if *path == second && why.contains("BLAKE3")),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:728:35 False: if *path == second && why.contains("BLAKE3")),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:744:55 False: Err(MembershipError::Corrupt { path, .. }) if path.ends_with("notes.txt")
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:843:66 False: matches!(&refusal, MembershipError::Stale { why, .. } if why.contains("carry")),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1203:9 False: let [illiq, one] = reported.as_slice() else {
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1207:9 False: illiq.starts_with("ZZILLIQ -> ZZNEWBIE")
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1208:16 False: && illiq.ends_with("a minority of the new spelling's keys"),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1212:9 False: one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1212:49 False: one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1253:9 False: strays[0].1.starts_with("ZZOLDCO listed 1 contract key(s)")
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1254:16 False: && strays[0].1.contains("filed under ZZNEWCO"),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1286:16 False: if *day == d(2024, 3, 6) && !why.is_empty()),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1286:41 False: if *day == d(2024, 3, 6) && !why.is_empty()),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1342:16 False: if *path == missing && why.contains("no such membership directory")),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1342:36 False: if *path == missing && why.contains("no such membership directory")),
UNCOVERED BRANCH crates/pull/src/fno_membership_tests.rs:1467:26 False: assert!(a.is_ok() || b.is_ok(), "round {round}: {a:?} | {b:?}");
UNCOVERED LINE crates/pull/src/fnowork_cov_tests.rs:196: fn never(url: &str) -> Result<String, Refusal> {
UNCOVERED LINE crates/pull/src/fnowork_cov_tests.rs:197: panic!("nothing should be asked, and {url} was");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:56: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:160: _ => return Err(Refusal::answered(400, "InputException".to_owned())),
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:286: panic!("Groww is an HTTP feed");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:651: landed.balance_line()
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:984: fn a_real_size_plan_does_not_grow_with_the_snapshots_held() {
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:985: let path = std::env::var_os("BRUTEX_KITE_DUMP").expect("BRUTEX_KITE_DUMP");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:986: let body = std::fs::read(path).expect("the dump reads");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:987: let root = scratch("real-size");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:990: let first = day(2026, 7, 1);
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:991: let asked = window(day(2026, 10, 1), day(2026, 12, 31));
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:992: let mut held = 0_u32;
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:993: let mut p50s = Vec::new();
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:994: for count in [1_u32, 10, 100] {
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:995: while held < count {
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:996: let on = Day::from_days(first.days_from_epoch() + held).expect("a real day");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:997: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:998: master_snapshots::snapshot(&root, Master::Kite, on, URL, &body).expect("a snapshot");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:999: println!(
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1000: "MEASURED snapshot {} written, its futures taken in: {} ms",
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1001: held + 1,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1002: started.elapsed().as_millis()
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1003: );
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1004: held += 1;
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1005: }
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1006: let mut ns: Vec<u128> = (0..50)
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1007: .map(|_| {
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1008: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1009: let book = Book::open(&root).expect("a book");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1010: let plan = futures_plan(
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1011: &book,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1012: route(),
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1013: "NIFTY",
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1014: asked,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1015: day(2026, 10, 22),
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1016: day(2026, 12, 31),
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1018: .expect("a plan");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1019: assert!(plan.refused.is_empty(), "{:?}", plan.refused);
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1020: assert!(!plan.asks.is_empty(), "NIFTY futures are asked");
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1021: started.elapsed().as_nanos()
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1022: })
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1023: .collect();
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1024: ns.sort_unstable();
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1025: println!(
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1027: ns[24] / 1_000,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1028: ns[48] / 1_000,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1029: ns[49] / 1_000,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1030: ns.len()
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1032: p50s.push(ns[24]);
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1034: assert!(
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1035: p50s[2] <= p50s[0] * 3,
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1037: p50s[2],
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1038: p50s[0]
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1040: }
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1450: alike.series[0]
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1464: unlike.balance_line()
UNCOVERED LINE crates/pull/src/fnowork_futures_tests.rs:1469: unlike.series[0]
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:54:9 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:284:9 False: let crate::vendor::Transport::Http(groww) = crate::vendor::Feed::Groww.descriptor().transport
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:659:9 False: stopped.contains("0 bars written, 0 offered from 0 rows")
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:660:16 False: && stopped.contains("; then stopped: vendor_refused"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:838:9 False: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:838:49 False: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:879:27 False: .filter(|why| why.starts_with("contract_duplicated")
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:995:15 True: while held < count {
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:995:15 False: while held < count {
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1130:9 False: said.starts_with("generation g1 of the futures index did not match its commit log")
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1131:16 False: && said.contains(why)
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1132:16 False: && said.contains("generation g2 was started"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1467:9 False: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1467:58 False: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1496:9 False: why.starts_with("snapshot_from_the_future: ")
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1497:16 False: && why.contains("2030-01-05")
UNCOVERED BRANCH crates/pull/src/fnowork_futures_tests.rs:1498:16 False: && why.contains("after today 2026-01-07"),
UNCOVERED LINE crates/pull/src/fold.rs:345: return Err(FoldError::OutOfOrder {
UNCOVERED LINE crates/pull/src/fold.rs:346: at: i,
UNCOVERED LINE crates/pull/src/fold.rs:347: previous: start,
UNCOVERED LINE crates/pull/src/fold.rs:348: found: snap.ts_micros,
UNCOVERED LINE crates/pull/src/fold.rs:349: });
UNCOVERED LINE crates/pull/src/fold.rs:1381: return;
UNCOVERED LINE crates/pull/src/fold.rs:1667: return Err(format!(
UNCOVERED LINE crates/pull/src/fold.rs:1668: "{venue}: session hours cannot attest the fixed stored grid"
UNCOVERED LINE crates/pull/src/fold.rs:1669: ));
UNCOVERED LINE crates/pull/src/fold.rs:1925: diagnostics.len(),
UNCOVERED LINE crates/pull/src/fold.rs:1926: diagnostics.iter().take(7).collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/fold.rs:2270: derived.diagnostics[0]
UNCOVERED LINE crates/pull/src/fold.rs:2322: lines[0]
UNCOVERED BRANCH crates/pull/src/fold.rs:272:38 False: (Some(first), Some(last)) if width > 0 => {
UNCOVERED BRANCH crates/pull/src/fold.rs:343:17 False: let Some(bar) = out.last_mut() else {
UNCOVERED BRANCH crates/pull/src/fold.rs:1329:20 False: if let Some(note) = day_note_at.and_then(|at| out.notes.get_mut(at)) {
UNCOVERED BRANCH crates/pull/src/fold.rs:1336:33 False: let complete = valid && expected > 0 && i128::from(count) == expected && traded == count;
UNCOVERED BRANCH crates/pull/src/fold.rs:1340:16 False: && expected > 0
UNCOVERED BRANCH crates/pull/src/fold.rs:1343:52 False: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
UNCOVERED BRANCH crates/pull/src/fold.rs:1343:68 False: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
UNCOVERED BRANCH crates/pull/src/fold.rs:1380:8 True: if expected <= 0 {
UNCOVERED BRANCH crates/pull/src/fold.rs:1664:8 True: if hours.kind() != crate::vendor::SessionKind::Continuous
UNCOVERED BRANCH crates/pull/src/fold.rs:1665:12 True: || hours.open_minute() != u32::from(crate::calendar::OPEN_MINUTE)
UNCOVERED LINE crates/pull/src/folder.rs:521: Err(why) => {
UNCOVERED LINE crates/pull/src/folder.rs:522: note_refused(dir, feed, &why);
UNCOVERED LINE crates/pull/src/folder.rs:523: Err(why)
UNCOVERED LINE crates/pull/src/folder.rs:553: Err(why) => {
UNCOVERED LINE crates/pull/src/folder.rs:554: note_refused(dir, feed, &why);
UNCOVERED LINE crates/pull/src/folder.rs:555: Err(why)
UNCOVERED BRANCH crates/pull/src/folder.rs:638:13 False: !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
UNCOVERED BRANCH crates/pull/src/folder.rs:638:47 False: !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
UNCOVERED BRANCH crates/pull/src/folder.rs:669:13 False: said.contains(ROOT_ENV) && said.contains("HOME"),
UNCOVERED BRANCH crates/pull/src/folder.rs:669:40 False: said.contains(ROOT_ENV) && said.contains("HOME"),
UNCOVERED LINE crates/pull/src/gap_backfill.rs:182: Err(_) => fault(have, number),
UNCOVERED LINE crates/pull/src/gap_backfill.rs:265: continue;
UNCOVERED LINE crates/pull/src/gap_backfill.rs:271: out.tally.refused.push(format!("{month}: {why}"));
UNCOVERED LINE crates/pull/src/gap_backfill.rs:272: continue;
UNCOVERED LINE crates/pull/src/gap_backfill.rs:292: out.tally.refused.push(format!("{month}: {why}"));
UNCOVERED LINE crates/pull/src/gap_backfill.rs:338: out.tally.length_unjudged += 1;
UNCOVERED LINE crates/pull/src/gap_backfill.rs:339: continue;
UNCOVERED LINE crates/pull/src/gap_backfill.rs:348: continue;
UNCOVERED LINE crates/pull/src/gap_backfill.rs:362: }
UNCOVERED LINE crates/pull/src/gap_backfill.rs:434: telemetry::Event::error("pull.gap_backfill", "stored-month gap lines NOT written")
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:127:8 True: if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:127:53 True: if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:127:73 True: if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:175:47 False: (Some(key), Some(day)) if !key.is_empty() && !key.contains('=') => {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:175:66 False: (Some(key), Some(day)) if !key.is_empty() && !key.contains('=') => {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:195:8 False: if have.faults.last() != Some(&number) {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:264:13 False: let Some((from, to)) = clip(month, window) else {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:270:16 True: if let Some(why) = &plan.refused {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:284:16 True: if let Err(why) = month_lines(
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:335:13 False: let Owes::Windows((windows, count)) =
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:347:13 False: let Some(open) = file.as_ref() else {
UNCOVERED BRANCH crates/pull/src/gap_backfill.rs:351:16 False: if let Some((mut line, missing)) =
UNCOVERED LINE crates/pull/src/gaps.rs:506: pub fn absent_minutes(&self) -> u32 {
UNCOVERED LINE crates/pull/src/gaps.rs:507: self.gaps
UNCOVERED LINE crates/pull/src/gaps.rs:508: .iter()
UNCOVERED LINE crates/pull/src/gaps.rs:509: .map(|gap| gap.minutes())
UNCOVERED LINE crates/pull/src/gaps.rs:510: .fold(0_u32, u32::saturating_add)
UNCOVERED LINE crates/pull/src/gaps.rs:511: }
UNCOVERED BRANCH crates/pull/src/gaps.rs:815:11 False: while day <= last {
UNCOVERED BRANCH crates/pull/src/gaps.rs:973:16 False: if open.day == day && open.reason == reason && open.to.saturating_add(1) == from =>
UNCOVERED BRANCH crates/pull/src/gaps.rs:1138:32 False: .any(|gap| gap.reason == Reason::OutsideWindow
UNCOVERED BRANCH crates/pull/src/gaps.rs:1176:40 False: lost.gaps.iter().any(|gap| gap.day == day
UNCOVERED BRANCH crates/pull/src/gaps.rs:1178:20 False: && gap.to == 939
UNCOVERED BRANCH crates/pull/src/gaps.rs:1242:16 False: && gap.to == 763
UNCOVERED BRANCH crates/pull/src/gaps.rs:1281:47 False: .any(|gap| gap.from == 915 && gap.to == 929 && gap.reason == Reason::VendorHole)
UNCOVERED BRANCH crates/pull/src/gaps.rs:1428:26 False: .any(|g| g.reason == Reason::OutsideWindow && g.from == 600),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:209: return Ok(None);
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:291: }
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2413: fn the_real_containers_open_with_no_prefix_and_print_their_facts() {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2414: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2415: let archive = Archive::open(Path::new(&path)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2416: for (tree, name) in [(0, "indices"), (1, "stocks")] {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2417: let held: Vec<usize> = archive.days[tree]
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2418: .iter()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2419: .enumerate()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2420: .filter_map(|(slot, cell)| matches!(cell, Some(Slot::Held(_))).then_some(slot))
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2421: .collect();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2422: let day = |slot: usize| {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2423: u32::try_from(slot)
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2424: .ok()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2425: .and_then(|slot| Day::from_days(archive.first + slot).ok())
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2426: };
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2427: println!(
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2429: held.len(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2430: held.first().map(|slot| day(*slot)),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2431: held.last().map(|slot| day(*slot)),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2434: println!(
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2436: archive
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2437: .days
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2438: .iter()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2439: .flatten()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2440: .filter(|cell| matches!(cell, Some(Slot::Refused(_))))
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2441: .count(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2442: archive.census()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2444: let span = Span::of(&archive.source, archive.len);
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2445: let entries = central_directory(&span).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2446: let count = |test: &dyn Fn(&Entry) -> bool| entries.iter().filter(|e| test(e)).count();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2447: println!(
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2449: entries.len(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2450: count(&|e| e.flags & 8 != 0),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2451: count(&|e| e.flags & ENCRYPTED != 0),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2452: count(&|e| e.method != 0),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2454: }
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2455: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2456: let mut years: Vec<_> = std::fs::read_dir(&dir)
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2457: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2458: .map(|entry| entry.unwrap().path())
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2459: .filter(|path| path.extension().is_some_and(|ext| ext == "zip"))
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2460: .collect();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2461: years.sort();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2462: for year in years {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2463: let file = std::fs::File::open(&year).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2464: let len = file.metadata().unwrap().len();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2465: let entries = zip_entries(&file, 0, len).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2466: let zips: Vec<&ZipEntry> = entries
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2467: .iter()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2468: .filter(|e| {
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2469: Path::new(&e.name)
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2470: .extension()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2471: .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2472: })
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2473: .collect();
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2474: println!(
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2477: year.display(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2478: entries.len(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2479: zips.len(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2480: zips.first().map(|e| &e.name),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2481: zips.last().map(|e| &e.name),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2482: zips.iter().filter(|e| e.locator.flags & 8 != 0).count(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2483: zips.iter()
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2484: .filter(|e| e.locator.flags & ENCRYPTED != 0)
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2485: .count(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2486: zips.iter().filter(|e| e.locator.method != 0).count(),
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2489: }
UNCOVERED LINE crates/pull/src/gdfl_archive.rs:2490: }
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:208:9 False: let Some(at) = end_at.checked_sub(20 + ZIP64_RECORD_LEN) else {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:212:17 False: let found = signed(&head, 0, ZIP64_END_SIGNATURE)
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:242:8 True: if count == 0 || shifted {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:287:16 False: if let Some(found) = zip64_record_at(span, end_at)? {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:653:20 False: if let Some(cell) = days
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:2414:16 True: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:2414:16 False: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:2455:16 True: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
UNCOVERED BRANCH crates/pull/src/gdfl_archive.rs:2455:16 False: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:255: store::file::TimeLookup::Indexed
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:322: lines[intent]
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:340: lines[repair]
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:802: nifty[0].why
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:1269: refused.failures[0]
UNCOVERED LINE crates/pull/src/gdfl_backfill_tests.rs:1282: refused.failures[0]
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:252:12 False: if small {
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:402:35 False: intents.iter().any(|line| line
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:523:13 False: failure.why.contains("the versioned repair refused it too")
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:524:20 False: && failure.why.contains("bar-revisions-v1"),
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:644:9 False: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_backfill_tests.rs:644:57 False: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:271: Self::OpenInterestSpaced => HEADER_OPEN_INTEREST_SPACED,
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:1244: }
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:1483: return None;
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:1804: self.skip();
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:1805: return;
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:2040: Ok(CasEligibility::Unknown) => Auction::Unknown(format!(
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:2041: "UNVERIFIED cash eligibility of {symbol}: the master holds no flag"
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:2042: )),
UNCOVERED LINE crates/pull/src/gdfl_cm.rs:3510: return false;
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:404:12 True: if read == 0 || self.rows != read || self.mixed {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:417:58 False: before >= MAX_BAD_ROWS && self.rows == before && !self.mixed && self.first == Some(count)
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:1236:16 False: if row.status.applies() && !swept {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:1241:16 False: if change.status.applies() {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:1386:41 False: Some((share, _)) if self.series => share,
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:1482:12 True: if name.len() > SYMBOL_CAPACITY + 1 + MAX_SERIES_CODE {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:1803:13 False: let Some(cell) = cell else {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:2884:58 False: if matches!(self.band, MatchBand::One(_)) && self.booked > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:2930:20 False: if let Some(row) = next {
UNCOVERED BRANCH crates/pull/src/gdfl_cm.rs:3509:9 False: let Ok(window) = crate::session::Window::new(day, day) else {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:704: other => panic!("{stem} {d}: {other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1583: fn the_match_band_costs_one_pass_a_file() {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1584: let mut rows: Vec<String> = (0..30_000_u32)
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1585: .map(|i| {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1586: quoted(
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1587: BAND_STEM,
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1588: band_day(),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1589: at(9, 15, 0) + i * 21_599 / 30_000,
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1590: "4444.15",
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1591: ("4444.10", "4444.20"),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1594: })
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1595: .collect();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1596: for (sod, quotes) in [
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1597: (at(15, 28, 3), BAND_BOOK),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1598: (at(15, 28, 7), BAND_ZERO),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1599: (at(15, 30, 1), BAND_BOOK),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1600: ] {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1601: rows.push(quoted(BAND_STEM, band_day(), sod, "4446.65", quotes, 383));
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1602: }
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1603: rows.push(quoted(
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1604: BAND_STEM,
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1605: band_day(),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1606: at(15, 52, 0),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1607: "4446.65",
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1611: let bytes = csv(&rows);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1612: let mut eligible = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1613: let mut ineligible = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1614: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1615: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1616: let file = band_decode(&bytes, Auction::Eligible).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1617: eligible.push(start.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1618: assert_eq!(file.auction.len(), 3);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1619: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1620: let file = band_decode(&bytes, Auction::NotEligible).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1621: ineligible.push(start.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1622: assert_eq!(file.rows.len(), rows.len());
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1624: let stats = |runs: &mut [std::time::Duration]| {
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1625: runs.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1626: (
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1627: runs[runs.len() / 2],
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1628: runs[runs.len() * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1629: runs[runs.len() - 1],
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1630: )
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1631: };
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1632: let (e50, e99, emax) = stats(&mut eligible);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1633: let (n50, n99, nmax) = stats(&mut ineligible);
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1634: println!(
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1637: rows.len()
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1639: assert!(
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1640: e50 <= n50 * 2 + std::time::Duration::from_millis(1),
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1643: }
UNCOVERED LINE crates/pull/src/gdfl_cm_admission_tests.rs:1663: path.display()
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:788:13 False: assert!(scope.is_stored() && !CmScope::swept().is_stored());
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:788:34 False: assert!(scope.is_stored() && !CmScope::swept().is_stored());
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:952:32 False: assert!(vix.iter().all(|b| b.close == 6_141 || b.close == 6_142));
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:1692:51 False: Err(CmRefusal::AuctionSidecar { why }) if why.contains("new generation")
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:1697:51 False: Err(CmRefusal::AuctionSidecar { why }) if why.contains("never an insertion")
UNCOVERED BRANCH crates/pull/src/gdfl_cm_admission_tests.rs:1851:20 False: .all(|one| one.instrument.ends_with("BANKNIFTY") && one.why.contains("absent"))
UNCOVERED LINE crates/pull/src/gdfl_cm_auction.rs:650: }
UNCOVERED LINE crates/pull/src/gdfl_cm_auction.rs:998: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_cm_auction.rs:1158: other => panic!("{other:?}"),
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction.rs:642:12 False: if let Some(parent) = path.parent() {
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_ps1_tests.rs:42: other => panic!("not a sidecar refusal: {other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_ps1_tests.rs:82: panic!("the host took a shared flock on an O_PATH descriptor");
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_ps1_tests.rs:85: panic!("the host took an exclusive flock on an O_PATH descriptor");
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_ps1_tests.rs:174: cut: &|_, _| unreachable!("a fresh sidecar has no torn tail to cut"),
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_ps1_tests.rs:81:9 False: let Err(std::fs::TryLockError::Error(shared)) = unlockable(&lock_at).try_lock_shared() else {
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_ps1_tests.rs:84:9 False: let Err(std::fs::TryLockError::Error(exclusive)) = unlockable(&lock_at).try_lock() else {
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_v2_tests.rs:80: Ok(sidecar) => panic!(
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_v2_tests.rs:82: sidecar.records.len()
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_v2_tests.rs:91: other => panic!("not a sidecar refusal: {other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_v2_tests.rs:260: made.display()
UNCOVERED LINE crates/pull/src/gdfl_cm_auction_v2_tests.rs:419: Err(refusal) => panic!("a reader beside an append was refused: {refusal}"),
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:17 False: refusal.why.contains("record 1") && refusal.why.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:53 False: refusal.why.contains("record 1") && refusal.why.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:25 False: Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {}
UNCOVERED BRANCH crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:39 False: Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {}
UNCOVERED LINE crates/pull/src/gdfl_dayzips.rs:380: (None, None) => Ok(None),
UNCOVERED BRANCH crates/pull/src/gdfl_dayzips.rs:219:8 False: if named.len() < cap {
UNCOVERED BRANCH crates/pull/src/gdfl_dayzips.rs:285:37 False: if kind.is_dir() && !name.starts_with('.') {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:540: fn day_table_sizing_timing() {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:541: fn stats(mut took: Vec<std::time::Duration>) -> String {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:542: took.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:543: let n = took.len();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:544: format!(
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:546: took[n / 2],
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:547: took[n * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:548: took[n - 1]
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:550: }
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:551: let alloc = |sized: bool| {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:552: (0..2_000)
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:553: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:554: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:555: let table: HashMap<(LooseKind, Day), Copies> = if sized {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:556: HashMap::with_capacity(DAYZIPS_KEYS_MAX)
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:558: HashMap::default()
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:560: let took = began.elapsed();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:561: assert!(table.capacity() >= usize::from(sized) * DAYZIPS_KEYS_MAX);
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:562: took
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:563: })
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:564: .collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:565: };
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:566: let empty = fresh("timing.empty");
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:567: let edges = fresh("timing.edges");
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:568: for kind in LooseKind::ALL {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:569: for day in [
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:570: Day::new(2018, 1, 1).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:571: Day::new(2026, 12, 31).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:572: ] {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:573: put(&edges, &kind.path_of(day), b"");
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:574: }
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:576: let walk = |root: &Path| {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:577: (0..200)
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:578: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:579: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:580: let zips = DayZips::open(root).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:581: let took = began.elapsed();
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:582: assert!(zips.days.capacity() >= DAYZIPS_KEYS_MAX);
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:583: took
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:584: })
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:585: .collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:586: };
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:587: println!(
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:589: stats(alloc(false)),
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:590: stats(alloc(true)),
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:591: stats(walk(&empty)),
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:592: stats(walk(&edges))
UNCOVERED LINE crates/pull/src/gdfl_dayzips_tests.rs:594: }
UNCOVERED BRANCH crates/pull/src/gdfl_dayzips_tests.rs:555:67 True: let table: HashMap<(LooseKind, Day), Copies> = if sized {
UNCOVERED BRANCH crates/pull/src/gdfl_dayzips_tests.rs:555:67 False: let table: HashMap<(LooseKind, Day), Copies> = if sized {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:245: Self::LinkMalformed { why } => write!(f, "the Dropbox config is not readable: {why}"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:246: Self::ConfigTooOpen { mode } => write!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:247: f,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:301: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:302: return Err(Wait::LinkMalformed {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:303: why: format!("{}: {why}", path.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:304: });
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:313: return Err(Wait::LinkMalformed {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:314: why: format!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:315: "{} is not a file of at most {MAX_CONFIG_BYTES} bytes",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:316: path.display()
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:317: ),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:318: });
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:321: why: format!("{}: {why}", path.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:322: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:406: return Ok(());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:409: why: format!("{}: {why}", folder.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:410: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:583: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:584: ["HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:585: .iter()
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:586: .find_map(|name| std::env::var(name).ok())
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:587: .and_then(|value| proxy_host_of(&value))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:776: return Err(stopped_during(what));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:786: RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:788: format!("{what}: {}", transport_words(&why, self.via.as_deref())),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:790: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:798: return Err(Wait::TokenDead.into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:827: "server_error"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:857: "timed out"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:859: "could not connect"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:861: "redirect"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:865: "failed"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:874: Some(proxy) => format!("{class} (host {host}, via_proxy={proxy})"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:923: return Err(format!("a page past {cap} bytes"));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:982: "deleted" => {}
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:983: _ => return Err(shape(".tag")),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1049: self.repeats += 1;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1050: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1053: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1054: "listing_incomplete",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1055: format!("more than {MAX_LISTED} files"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1056: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1057: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1060: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1061: "listing_incomplete",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1062: format!("more than {MAX_FOLDERS} folders"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1063: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1064: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1089: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1090: "listing_incomplete",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1091: format!("more than {MAX_FOLDERS} folders or deeper than {MAX_DEPTH}"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1092: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1093: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1173: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1174: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1175: "listing_incomplete",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1176: format!("pages={pages} a later page answered {}", answer.status),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1177: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1178: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1190: Stop::Refused(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1191: "listing_incomplete",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1192: format!("pages={pages} {}", refusal.detail),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1193: ))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1196: Stop::Refused(RunRefusal::new("link_unreachable", refusal.detail))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1345: return;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1377: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1378: Err(RunRefusal::new("journal_unwritable", why.clone()).into())
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1525: Err(why) => return Err(format!("{}: {why}", path.display())),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1566: return;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1655: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1680: Err(why) => return Err(refused(why)),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1774: Ok(outcome) => Handed {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1775: ok: true,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1776: verdict: outcome.verdict().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1777: },
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:1935: (Ok(()), Err(lost)) => Err(lost),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2069: let code = if why.kind() == std::io::ErrorKind::PermissionDenied {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2070: "store_not_writable"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2072: "store_lost"
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2074: RunRefusal::new(code, format!("{}: {why}", path.display()))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2075: };
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2083: Err(why) => return Err(lost(&at, why).into()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2281: journal.line(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2282: "stopped",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2283: &[
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2284: ("stage", "download".to_owned()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2285: ("before", quoted(&file.name)),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2286: ],
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2288: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2289: "stopped",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2290: format!("the operator pressed Stop before {}", file.name),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2291: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2292: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2372: _ => break,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2393: RunRefusal::new("journal_unreadable", format!("the import journal: {why}"))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2394: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2571: Tree::Cm(crate::gdfl_cm::CmKind::Stocks) => "stocks",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2848: refuse_file(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2849: journal,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2850: report,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2851: file,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2853: format!("{} exists and is never overwritten", target.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2855: return Ok(None);
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2871: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2872: Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2873: "store_lost",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2874: format!("{} is gone", ctx.fetch.store_root.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2875: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2876: .into())
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2895: part = new_part(ctx.run_dir, file, journal)?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2896: continue;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2917: refuse_file(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2918: journal,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2919: report,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2920: file,
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2922: format!("cut {cuts} times; the part is kept for the next press"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:2924: return Ok(None);
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3036: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3037: return Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3038: "staging_write_failed",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3039: format!("{}: {why}", path.display()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3040: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3041: .into());
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3045: Err(RunRefusal::new(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3046: "staging_write_failed",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3047: format!("64 parts of {} exist", file.name),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3048: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3049: .into())
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3109: Err(Stop::Refused(refusal)) if refusal.code == "transport" => return Ok(Got::Cut(from)),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3111: Err(Stop::Refused(refusal)) if refusal.code == "stopped" => return Ok(Got::Stopped(from)),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3112: Err(stop) => return Err(stop),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3145: return Some(Got::Restart("remote_changed"));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3154: return Some(Got::Refused(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3155: "http_status",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3156: format!("status={}", answer.status),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3157: ));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3178: return Some(Got::Refused(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3179: "remote_changed",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3180: format!("listed rev {} served rev {rev}", file.rev),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3181: ));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3209: RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3210: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3214: RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3215: })?
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3232: let _ = out.sync_data();
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3233: return Ok(Got::Refused(
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3234: "size_mismatch",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3235: format!("the body runs past the listed {size} bytes"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3236: ));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3245: RunRefusal::new("staging_write_failed", format!("{}: {why}", path.display()))
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3246: })?;
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3248: return Ok(Got::Cut(have));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3295: (
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3296: "not_a_day_zip",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3297: "not a canonical GDFL day zip name".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3298: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3299: })?
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3316: return Err(("zip_holds_no_member", "the zip lists no member".to_owned()));
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3331: return Err((
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3332: "member_size_mismatch",
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3333: format!("{} is not {} bytes", entry.name, entry.len),
UNCOVERED LINE crates/pull/src/gdfl_dropbox.rs:3334: ));
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:296:25 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:312:12 True: if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:312:31 True: if !meta.is_file() || meta.len() > MAX_CONFIG_BYTES {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:341:16 True: if trimmed.is_empty() || trimmed.starts_with('#') {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:405:9 False: let Some(folder) = path.parent() else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:412:29 True: if mode & 0o022 != 0 || folder_meta.uid() != meta.uid() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:464:8 True: if link.len() > 2_048 || !link.bytes().all(|b| b.is_ascii_graphic()) {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:581:12 False: if !self.proxy {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:650:12 False: if attempt.previous().len() < 3 && attempt.url().scheme() == scheme && allowed.allows(&host)
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:662:8 False: if !endpoints.proxy {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:775:16 True: if self.stop.is_some_and(|flag| flag.load(Ordering::Acquire)) {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:792:33 False: if status == 401 && !reread {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:797:16 True: if status == 401 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:824:35 False: let code = if status == 429 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:856:20 True: let class = if why.is_timeout() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:858:15 True: } else if why.is_connect() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:860:15 True: } else if why.is_redirect() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:862:15 True: } else if why.is_body() || why.is_decode() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:862:32 False: } else if why.is_body() || why.is_decode() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:922:12 True: if out.len() + chunk.len() > cap {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1045:16 False: if self.seen_folders.insert(sub.clone()) {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1052:12 True: if self.files.len() > MAX_LISTED {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1059:12 True: if self.seen_folders.len() > MAX_FOLDERS {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1088:12 True: if folders > MAX_FOLDERS || depth > MAX_DEPTH {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1088:37 True: if folders > MAX_FOLDERS || depth > MAX_DEPTH {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1115:40 True: stalled = if added == 0 || kept_cursor {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1126:28 False: if folder.is_empty() { "/" } else { &folder }
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1169:55 False: .map_err(|stop| incomplete_unless_first(stop, first && folder.is_empty(), pages))?;
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1171:12 False: if first && folder.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1171:21 False: if first && folder.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1189:35 True: Stop::Refused(refusal) if !first && refusal.code == "transport" => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1189:45 True: Stop::Refused(refusal) if !first && refusal.code == "transport" => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1189:45 False: Stop::Refused(refusal) if !first && refusal.code == "transport" => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1195:35 False: Stop::Refused(refusal) if first && refusal.code == "transport" => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1195:44 True: Stop::Refused(refusal) if first && refusal.code == "transport" => {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1313:16 True: if key != "v" || value != "1" {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1344:12 True: if lines.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1363:16 False: if let Some(root) = self
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1424:12 False: if let Some(dir) = path.parent()
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1429:76 False: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && dir.is_dir() => {}
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1524:25 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => Vec::new(),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1536:32 False: if whole.is_empty() && bytes.first() != Some(&b'\n') {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1565:21 False: let Some(link) = fields.get("link") else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1570:28 False: if let Some(count) = number("count") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1586:24 False: if let Some(path) = fields.get("part") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1604:24 False: if let Some(path) = fields.get("path") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1616:20 False: if self.verified.get(&key).map(PathBuf::as_path)
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1632:24 False: if let Some(name) = fields.get("name") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1637:24 False: if let Some(name) = fields.get("name") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1679:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:1697:24 False: if from > 0 && !lines.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2069:23 True: let code = if why.kind() == std::io::ErrorKind::PermissionDenied {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2069:23 False: let code = if why.kind() == std::io::ErrorKind::PermissionDenied {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2082:25 False: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && at.is_dir() => {}
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2082:76 False: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && at.is_dir() => {}
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2191:26 False: .filter(|shrink| shrink.was == was && shrink.now == count);
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2280:12 True: if ctx.fetch.stopped() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2348:8 True: if tick.starts_with(&staging) || staging.starts_with(&tick) {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2524:37 True: if upper.contains("BSE") || upper.contains("MCX") {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2550:30 True: if file.size == 0 || file.size > MAX_FILE_BYTES {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2588:6 False: (!file.hash.is_empty() && *hash != file.hash).then(|| {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2599:27 True: if name.is_empty() || name == "." || name == ".." {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2847:8 True: if std::fs::symlink_metadata(&target).is_ok() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2869:8 False: if ctx.fetch.store_root.is_dir() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2894:12 True: if have > file.size {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:2916:20 True: if cuts > MAX_RESUMES {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3035:25 False: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3109:40 True: Err(Stop::Refused(refusal)) if refusal.code == "transport" => return Ok(Got::Cut(from)),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3109:40 False: Err(Stop::Refused(refusal)) if refusal.code == "transport" => return Ok(Got::Cut(from)),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3111:40 True: Err(Stop::Refused(refusal)) if refusal.code == "stopped" => return Ok(Got::Stopped(from)),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3111:40 False: Err(Stop::Refused(refusal)) if refusal.code == "stopped" => return Ok(Got::Stopped(from)),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3144:8 True: if answer.status == 416 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3147:8 True: if answer.status == 404 || answer.status == 409 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3153:32 True: if answer.status != 200 && answer.status != 206 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3174:12 False: if let Some(rev) = rev
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3175:12 False: && !file.rev.is_empty()
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3176:12 True: && rev != file.rev
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3189:57 False: Some((start, _, total)) if start == from && total == file.size => {}
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3231:12 True: if have.saturating_add(len) > size {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3247:8 True: if have < size {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3282:12 False: && have.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX)) > at
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3315:8 True: if entries.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox.rs:3330:12 True: if u64::try_from(bytes.len()).unwrap_or(u64::MAX) != entry.len {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_cov_tests.rs:63: panic!("a refused read is a missing token");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_cov_tests.rs:71: panic!("no field is a missing token");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_cov_tests.rs:78: panic!("no field is a missing token on the re-read too");
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_cov_tests.rs:62:9 False: let Wait::TokenMissing { why } = refused.read().unwrap_err() else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_cov_tests.rs:70:9 False: let Wait::TokenMissing { why } = unnamed.read().unwrap_err() else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_cov_tests.rs:77:9 False: let Wait::TokenMissing { why } = unnamed.reread(&read).unwrap_err() else {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:153: _ => Reply::Full(404, Vec::new(), Vec::new()),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:234: return json_reply(
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:236: &serde_json::json!({"error_summary": "path/not_found/"}),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:318: continue;
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:348: return None;
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:957: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1005: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1093: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1141: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1198: panic!("{cut:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1228: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1252: panic!("{shrank:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1403: panic!("{storm:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1431: panic!("{down:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1603: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1687: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1766: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1812: panic!("{report:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1953: panic!("{full:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1992: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:1994: [format!("bytes={kept}-")],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2058: again.refused[0].detail
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2093: panic!("{stopped:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2347: report.refused[0]
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2472: panic!("{report:#?}");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2580: panic!("{outcome:?}");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2783: assert!(report.journal_error.is_some());
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2784: assert_eq!(report.listed, 0, "nothing listed: {report:#?}");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2786: assert!(handed.is_empty(), "{handed:?}");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2787: assert!(server.log.lock().unwrap().is_empty(), "no request at all");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2788: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2871: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2972: fn measure_the_journal_costs_of_a_press() {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2973: let mut names = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2974: 'outer: for year in 2019..2027 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2975: for month in 1..=12 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2976: for day in 1..=28 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2977: names.push(format!("GFDLNFO_TICK_{day:02}{month:02}{year}.zip"));
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2978: if names.len() == 1_000 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2979: break 'outer;
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2980: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2984: let files: Vec<(String, Vec<u8>)> = names
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2985: .iter()
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2986: .enumerate()
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2987: .map(|(i, name)| {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2988: (
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2989: format!("/{name}"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2990: day_zip(u32::try_from(i).unwrap() + 1, 1, 30),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2991: )
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2992: })
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2993: .collect();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2994: let refs: Vec<(&str, Vec<u8>)> =
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2995: files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2996: let server = Server::start(fake_with(&refs));
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2997: let world = roots("r1-measure");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2998: for name in &names {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:2999: hold(&world, name);
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3000: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3001: let mut before = 0;
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3002: let mut added = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3003: for press_no in 1..=3 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3004: let start = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3005: let report = go(&server, &world, &mut Recorder::default());
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3006: let took = start.elapsed();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3007: let lines = journal_text(&world).lines().count();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3008: println!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3009: "MEASURE press {press_no}: n=1000 held={} outcome={} took_ms={} lines_added={}",
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3010: report.held,
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3011: report.outcome.word(),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3012: took.as_millis(),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3013: lines - before
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3014: );
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3015: added.push(lines - before);
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3016: before = lines;
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3017: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3018: assert_eq!(added[1], added[2]);
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3019: assert!(added[1] <= 6, "{added:?}");
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3020: for count in [1_000_usize, 10_000, 100_000] {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3021: let roots = roots(&format!("r1-measure-{count}"));
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3022: let path = journal_path(&roots.store);
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3023: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3024: let mut text = String::new();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3025: for i in 0..count {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3026: let _ = writeln!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3027: text,
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3028: "v=1 verified run=r at_ms={i} name=\"GFDLNFO_TICK_{:02}072019 \u{e9}t\u{e9} {i}.zip\" \
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3029: rev=\"{i:011}\" hash=\"\" path=\"/s/{i}\" bytes=100 entries=1 sha256={i:064x} how=downloaded",
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3030: 1 + i % 28
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3031: );
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3032: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3033: std::fs::write(&path, &text).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3034: let mut load = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3035: let mut tail = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3036: for _ in 0..31 {
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3037: let start = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3038: let memory = Memory::load(&path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3039: load.push(start.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3040: assert_eq!(memory.verified.len(), count);
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3041: let start = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3042: assert!(status(&roots.store).unwrap().is_some());
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3043: tail.push(start.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3045: load.sort();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3046: tail.sort();
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3047: println!(
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3049: text.len(),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3050: load[15],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3051: load[30],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3052: load[30],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3053: tail[15],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3054: tail[30],
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3055: tail[30]
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3058: }
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3078: report.refused[0]
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3155: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3205: accepted[0]
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3459: codes(&two)
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3521: panic!("{two:#?}")
UNCOVERED LINE crates/pull/src/gdfl_dropbox_tests.rs:3687: other => (format!("{other:?}"), String::new()),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:233:13 False: let Some(file) = self.files.iter().find(|f| f.path == path).cloned() else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:316:21 False: let Ok(mut stream) = stream else { continue };
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:317:21 False: let Some(request) = read_request(&mut stream) else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:347:12 True: if stream.read(&mut byte).ok()? == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:482:16 False: && self.got.len() == *after
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:868:9 False: journal_text(&roots).contains("v=1 wait ")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:869:16 False: && journal_text(&roots).contains("reason=dropbox_link_missing")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:956:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1004:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1092:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1140:13 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1197:9 False: let Outcome::Refused(refusal) = &cut.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1227:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1251:9 False: let Outcome::Refused(refusal) = &shrank.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1402:9 False: let Outcome::Refused(refusal) = &storm.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1430:9 False: let Outcome::Refused(refusal) = &down.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1602:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1686:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1765:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1784:60 False: assert!(matches!(&lost.outcome, Outcome::Refused(r) if r.code == "store_lost"));
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1811:9 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1952:9 False: let Outcome::Refused(refusal) = &full.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:1991:8 True: if kept > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2092:9 False: let Outcome::Refused(refusal) = &stopped.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2101:16 False: && l.contains("stage=download")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2127:57 False: matches!(&early.outcome, Outcome::Refused(r) if r.code == "stopped"),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2358:24 False: && l.contains("GFDLNFO_TICK_04072019.zip")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2442:20 False: && l.contains("complete=false")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2471:13 False: let Outcome::Refused(refusal) = &report.outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2579:13 False: let Outcome::Refused(refusal) = &outcome else {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2978:24 True: if names.len() == 1_000 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:2978:24 False: if names.len() == 1_000 {
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:3202:9 False: accepted[0].contains(" complete=true count=2 ")
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:3203:16 False: && accepted[0].ends_with(&format!(" accepted_shrink={token} was=3")),
UNCOVERED BRANCH crates/pull/src/gdfl_dropbox_tests.rs:3520:9 False: let Outcome::Refused(refusal) = &two.outcome else {
UNCOVERED LINE crates/pull/src/gdfl_format.rs:81: Err(_) => true,
UNCOVERED LINE crates/pull/src/gdfl_format_change_tests.rs:373: other => panic!("{other:?}"),
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:129:13 False: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:129:52 False: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:221:22 False: .any(|f| f.instrument.contains("2024-04-09") && f.why.contains(&shape)),
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:274:13 False: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:274:41 False: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
UNCOVERED BRANCH crates/pull/src/gdfl_format_change_tests.rs:283:22 False: .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:1382: let path = entry.unwrap().path();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:1383: 1 + if path.is_dir() {
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:1384: entries_under(&path)
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:1386: 0
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:1388: })
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2108: Refused::at(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2109: Cause::open(Stage::File, "file_taken_twice"),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2110: failure("file", "taken twice".to_owned()),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2112: })
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2222: matches!(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2283: matches!(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2431: Refused::at(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2432: Cause::open(Stage::File, "file_taken_twice"),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2433: failure("file", "taken twice".to_owned()),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2435: })
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2479: }
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2719: let said = ended.to_string();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2720: assert!(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2721: said.contains("chflags nouchg")
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2722: && said.contains("chmod -N")
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2723: && said.contains("read-write"),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2726: let done = ended.report().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2727: assert_eq!(done.closed_incomplete, vec![d2()], "{attempt}: {done:?}");
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2728: let text = journal(&store);
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2729: assert!(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2730: !text.contains("begin indices 2024-04-03"),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2734: chmod_all(&nifty, 0o644);
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2735: let resumed = plain(&root, ImportKind::Indices, (d2(), d3()), &store);
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2736: assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2737: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2738: bars(&store, "INDEX", "NIFTY", None),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2739: bars(&whole, "INDEX", "NIFTY", None)
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2743: let planted = root.join("P");
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2744: std::fs::create_dir_all(&planted).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2745: filing::inject::plant(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2746: &planted,
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2747: "NIFTY 50.NSE_IDX.csv",
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2748: FailKind::Io {
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2749: kind: std::io::ErrorKind::ReadOnlyFilesystem,
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2750: publication_may_be_visible: false,
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2751: },
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2754: let ended = import_with(
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2755: &root,
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2756: (ImportKind::Indices, d1(), d2()),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2757: (&planted, &[]),
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2758: options,
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2760: .unwrap_err();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2761: assert_eq!(ended.code(), "store_not_writable", "{ended}");
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2762: assert!(!journal(&planted).contains("begin indices 2024-04-02"));
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2763: std::fs::remove_dir_all(root).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia16_tests.rs:2764: }
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:198:13 False: kind.is_hole() && HOLE_REASONS.contains(&kind.code()),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:198:31 False: kind.is_hole() && HOLE_REASONS.contains(&kind.code()),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:314:13 False: assert!(restated.restated && !restated.done && restated.hole.is_none());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:314:34 False: assert!(restated.restated && !restated.done && restated.hole.is_none());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:314:52 False: assert!(restated.restated && !restated.done && restated.hole.is_none());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:342:13 False: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:342:37 False: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:342:64 False: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:394:13 False: line.starts_with("incomplete ")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:395:20 False: && line.contains(" stage=file ")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:396:20 False: && line.contains(" reason=needs_month_generation"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:563:13 False: line.starts_with("incomplete ") && line.contains(" reason=needs_month_generation"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:563:48 False: line.starts_with("incomplete ") && line.contains(" reason=needs_month_generation"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:647:29 False: .any(|line| line.starts_with("repairing ") && line.contains("sealed-month")),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:747:13 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:747:48 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:846:9 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:846:44 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:873:13 False: again.restated.is_empty() && again.resumed.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:873:42 False: again.restated.is_empty() && again.resumed.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:955:9 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:955:44 False: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:966:9 False: failure.why.contains(&first_offered.to_string()) && failure.why.contains("2024-04"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:966:61 False: failure.why.contains(&first_offered.to_string()) && failure.why.contains("2024-04"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1368:12 False: if day == self.after {
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1383:20 True: 1 + if path.is_dir() {
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1383:20 False: 1 + if path.is_dir() {
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1827:71 False: matches!(&ended, ImportRefusal::SourceLost { day, .. } if *day == d2()),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1833:13 False: !text.contains("2024-04-02") && !text.contains("2024-04-03"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1833:45 False: !text.contains("2024-04-02") && !text.contains("2024-04-03"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1878:13 False: line.starts_with("refused ") && line.contains(" reason=no_wanted_file"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1878:45 False: line.starts_with("refused ") && line.contains(" reason=no_wanted_file"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:1930:24 False: .any(|one| one.instrument.ends_with("BANKNIFTY") && one.why.contains("absent")),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2133:9 False: line.starts_with("done ") && !line.contains("vendor_refused"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2133:38 False: line.starts_with("done ") && !line.contains("vendor_refused"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2167:68 False: Some(RunRefusal::RepairReasonInvalid { bytes }) if bytes == bad.len()
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2473:12 False: if !filled {
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2485:13 False: assert!(said.contains("NIFTY") && said.contains("4097"), "{said}");
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2485:39 False: assert!(said.contains("NIFTY") && said.contains("4097"), "{said}");
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2721:13 True: said.contains("chflags nouchg")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2721:13 False: said.contains("chflags nouchg")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2722:20 True: && said.contains("chmod -N")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2722:20 False: && said.contains("chmod -N")
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2723:20 True: && said.contains("read-write"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2723:20 False: && said.contains("read-write"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2821:9 False: said.contains("999 bytes free of 10000") && said.contains("floor of 1000"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2821:53 False: said.contains("999 bytes free of 10000") && said.contains("floor of 1000"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2862:13 False: assert!(space.total >= space.free && space.total > 0, "{space:?}");
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2862:42 False: assert!(space.total >= space.free && space.total > 0, "{space:?}");
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2920:9 False: two.starts_with("incomplete ") && two.contains(" reason=source_crc_mismatch"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2920:43 False: two.starts_with("incomplete ") && two.contains(" reason=source_crc_mismatch"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia16_tests.rs:2957:56 True: !(one.instrument.contains("NIFTY BANK") && one.why.contains("needs_month_generation")),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:389: fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:390: samples.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:391: let at = |q: usize| samples.get((samples.len() * q / 100).min(samples.len().saturating_sub(1)));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:392: (
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:393: at(50).copied().unwrap_or(0),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:394: at(99).copied().unwrap_or(0),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:395: samples.last().copied().unwrap_or(0),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:396: )
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:397: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:407: fn ia3_lookup_costs() {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:408: let days: Vec<Day> = (0..2_600)
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:409: .filter_map(|n| Day::from_days(Day::new(2018, 9, 3).unwrap().days_from_epoch() + n).ok())
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:410: .collect();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:411: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:412: tickstore_costs(Path::new(&root), &days);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:413: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:414: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:415: archive_costs(Path::new(&path), &days);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:416: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:417: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:418: dayzips_costs(Path::new(&root), &days);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:419: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:420: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:424: fn tickstore_costs(root: &Path, days: &[Day]) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:425: let store = TickStore::new(root);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:426: let cold = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:427: let table = store.table();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:428: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:430: cold.elapsed().as_millis(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:435: table.stray_named.iter().take(4).collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:437: let mut probe: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:438: for day in days.iter().cycle().take(20_000) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:439: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:440: let _ = std::hint::black_box(table.misfiled(0, *day));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:441: probe.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:442: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:443: let n = probe.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:444: let (p50, p99, max) = quantiles(&mut probe);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:445: println!("tickstore misfiled probe: n={n} p50={p50} ns p99={p99} ns max={max} ns");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:446: let mut listing: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:447: let mut sums: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:448: for day in days.iter().take(400) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:449: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:450: let got = store.day(CmKind::Indices, *day);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:451: listing.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:452: if let Ok(Some(listed)) = got {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:453: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:454: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:455: sums.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:456: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:458: let n = listing.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:459: let (p50, p99, max) = quantiles(&mut listing);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:460: println!("tickstore day listing (open + index): n={n} p50={p50} ns p99={p99} ns max={max} ns");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:461: let n = sums.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:462: let (p50, p99, max) = quantiles(&mut sums);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:463: println!("indices listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:464: let nfo = crate::gdfl_nfo::NfoTickStore::new(root);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:465: let cold = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:466: let table = nfo.table();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:467: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:469: cold.elapsed().as_millis(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:474: let mut sums: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:475: for day in days.iter().rev().take(60) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:476: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:477: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:478: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:479: sums.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:480: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:482: let n = sums.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:483: let (p50, p99, max) = quantiles(&mut sums);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:484: println!("options listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:485: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:488: fn archive_costs(path: &Path, days: &[Day]) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:489: let cold = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:490: let archive = Archive::open(path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:491: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:493: cold.elapsed().as_millis(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:494: archive.census()
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:496: let mut lookups: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:497: for day in days.iter().take(400) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:498: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:499: let _ = std::hint::black_box(archive.day(CmKind::Indices, *day));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:500: lookups.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:501: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:502: let n = lookups.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:503: let (p50, p99, max) = quantiles(&mut lookups);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:504: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:507: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:510: fn dayzips_costs(root: &Path, days: &[Day]) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:511: let cold = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:512: let loose = crate::gdfl_dayzips::DayZips::open(root).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:513: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:515: cold.elapsed().as_micros(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:516: loose.census()
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:518: let mut lookups: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:519: for day in days.iter().rev().take(400) {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:520: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:521: let _ = std::hint::black_box(CmSource::day(&loose, CmKind::Indices, *day));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:522: lookups.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:523: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:524: let n = lookups.len();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:525: let (p50, p99, max) = quantiles(&mut lookups);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:526: println!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:529: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:545: fn one_day<'a>(kind: ImportKind, d: Day, only: &'a [String], at: &'a Path) -> Run<'a> {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:546: Run {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:547: kind,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:548: from: d,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:549: to: d,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:550: only,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:551: store_root: at,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:552: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:553: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:556: fn listing_of(line: &str) -> Option<&str> {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:557: line.split(' ')
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:558: .find_map(|field| field.strip_prefix("listing="))
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:559: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:570: fn real_days_close_with_the_same_listing_from_two_sources() {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:571: let var = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:572: let (Some(ts), Some(cm), Some(nfo)) = (
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:573: var("BRUTEX_GDFL_TICKSTORE"),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:574: var("BRUTEX_GDFL_CM_ARCHIVE"),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:575: var("BRUTEX_GDFL_NFO_ARCHIVES"),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:577: return;
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:579: let archive = Archive::open(&cm).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:580: let store = TickStore::new(&ts);
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:581: let root = scratch("ia3-real");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:582: let cm_case = |tag: &str, kind: ImportKind, d: Day, only: &[String]| {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:583: let a = root.join(format!("{tag}-zip"));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:584: let b = root.join(format!("{tag}-ts"));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:585: let zip_report = run_cm(&archive, &one_day(kind, d, only, &a)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:586: let ts_report = run_cm(&store, &one_day(kind, d, only, &b)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:587: let (zip_line, store_line) = (
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:588: close_of(&a, kind.as_str(), d),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:589: close_of(&b, kind.as_str(), d),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:590: );
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:591: println!("{tag} zip: {zip_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:592: println!("{tag} tickstore: {store_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:593: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:594: assert_eq!(listing_of(&zip_line), listing_of(&store_line), "{tag}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:595: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:596: (
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:597: zip_report.files,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:598: zip_report.seconds,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:599: zip_report.files_skipped
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:600: ),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:601: (ts_report.files, ts_report.seconds, ts_report.files_skipped),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:604: };
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:605: let nifty = ["NIFTY".to_owned()];
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:606: cm_case(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:607: "nifty-spot",
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:608: ImportKind::Indices,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:609: Day::new(2024, 4, 1).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:610: &nifty,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:611: );
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:612: cm_case(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:613: "muhurat",
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:614: ImportKind::Indices,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:615: Day::new(2024, 11, 1).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:616: &nifty,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:617: );
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:618: cm_case(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:619: "banknifty",
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:620: ImportKind::Indices,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:621: Day::new(2020, 3, 23).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:622: &["BANKNIFTY".to_owned()],
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:623: );
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:624: cm_case(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:625: "reliance",
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:626: ImportKind::Stocks,
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:627: Day::new(2024, 4, 1).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:628: &["RELIANCE".to_owned()],
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:629: );
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:630: let d = Day::new(2024, 4, 4).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:631: let a = root.join("nifty-options-zip");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:632: let b = root.join("nifty-options-ts");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:633: let options = ImportKind::Options;
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:634: let zips = run_nfo(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:635: &crate::gdfl_nfo::NfoZips::new(&nfo),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:636: &one_day(options, d, &nifty, &a),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:638: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:639: let tick = run_nfo(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:640: &crate::gdfl_nfo::NfoTickStore::new(&ts),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:641: &one_day(options, d, &nifty, &b),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:643: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:644: let (zip_line, store_line) = (close_of(&a, "options", d), close_of(&b, "options", d));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:645: println!("nifty-options zip: {zip_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:646: println!("nifty-options tickstore: {store_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:647: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:648: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:649: assert_eq!((zips.files, zips.seconds), (tick.files, tick.seconds));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:650: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:651: let loose = crate::gdfl_dayzips::DayZips::open(&loose).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:652: let d = Day::new(2026, 9, 25).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:653: let a = root.join("loose-nifty");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:654: let b = root.join("loose-nifty-ts");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:655: let indices = ImportKind::Indices;
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:656: let from_loose = run_cm(&loose, &one_day(indices, d, &nifty, &a)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:657: let from_ts = run_cm(&store, &one_day(indices, d, &nifty, &b)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:658: let (zip_line, store_line) = (close_of(&a, "indices", d), close_of(&b, "indices", d));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:659: println!("loose 2026-09-25: {zip_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:660: println!("tickstore 2026-09-25: {store_line}");
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:661: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:662: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:663: (from_loose.files, from_loose.seconds),
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:664: (from_ts.files, from_ts.seconds)
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:666: }
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:667: println!("scratch stores under {}", root.display());
UNCOVERED LINE crates/pull/src/gdfl_ia3_tests.rs:668: }
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:174:9 False: !line.contains("listing=") && !line.contains("skipped="),
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:174:39 False: !line.contains("listing=") && !line.contains("skipped="),
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:322:9 False: line.starts_with("refused indices 2024-04-01 * ")
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:323:16 False: && line.contains("stage=vendor reason=day_file_misfiled"),
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:411:12 True: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:411:12 False: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:414:12 True: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:414:12 False: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:417:12 True: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:417:12 False: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:452:16 True: if let Ok(Some(listed)) = got {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:452:16 False: if let Ok(Some(listed)) = got {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:476:16 True: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:476:16 False: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:572:9 True: let (Some(ts), Some(cm), Some(nfo)) = (
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:572:9 False: let (Some(ts), Some(cm), Some(nfo)) = (
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:650:12 True: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:650:12 False: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:867:13 False: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
UNCOVERED BRANCH crates/pull/src/gdfl_ia3_tests.rs:867:53 False: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
UNCOVERED LINE crates/pull/src/gdfl_ia4_tests.rs:154: kind.as_str()
UNCOVERED BRANCH crates/pull/src/gdfl_ia4_tests.rs:275:9 False: last.starts_with("done ") && last.contains(" definition=5 "),
UNCOVERED BRANCH crates/pull/src/gdfl_ia4_tests.rs:275:38 False: last.starts_with("done ") && last.contains(" definition=5 "),
UNCOVERED LINE crates/pull/src/gdfl_import.rs:2654: Ok(None) => false,
UNCOVERED LINE crates/pull/src/gdfl_import_census.rs:123: Err(why) => return Err(format!("{}: {why}", path.display())),
UNCOVERED BRANCH crates/pull/src/gdfl_import_census.rs:122:24 False: Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Vec::new(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:48:9 False: why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:48:56 False: why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:57:9 False: why.contains("lies past the end") && !why.contains("over the cap"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:57:46 False: why.contains("lies past the end") && !why.contains("over the cap"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:89:32 False: .is_some_and(|why| why.contains("plain.zip") && why.contains("no end-of-central")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:150:9 False: fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:150:74 False: fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:179:9 False: fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:179:40 False: fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_cov_tests.rs:262:62 False: matches!(&refused, ImportRefusal::Journal { why } if why.contains("gdfl.journal")),
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1207: Refused::at(
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1208: Cause::open(Stage::File, "file_taken_twice"),
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1209: failure("file", "a handed file was taken twice".to_owned()),
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1211: })
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1747: from: first,
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1748: to: last,
UNCOVERED LINE crates/pull/src/gdfl_import_day.rs:1749: })?,
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:429: panic!("a day before the range is never read whole");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:434: panic!("without the flag the close still owes ZQB");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:478: panic!("ZQZ's own key is still open");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:500: panic!("a close under an unrecorded scope owes the members beyond the swept surface");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:503: panic!("so does the second day's");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:535: panic!("{:?}", report.failures);
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:580: panic!("a restated day is not read when the run asks not to restate")
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:622: unreachable!("a day every share of which stands asks no scope")
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:732: Ask::All => panic!("the day was closed done: read again for its new members only"),
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:776: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:816: Ask::All => panic!("both days were closed done: read again for ZQB only"),
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:826: panic!("{flags}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:940: panic!("{key_held:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:948: panic!("{run_finished:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:957: panic!("{not_written:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:982: panic!("{unrecorded_said:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1048: |_, _| -> DayRead { panic!("no day is begun") },
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1059: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1089: panic!("a refused run reads no day")
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1093: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1124: |_, _| -> DayRead { panic!("a refused run reads no day") },
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1134: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1177: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1221: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1276: Ask::All => panic!("no day of the range is begun"),
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1287: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1298: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1386: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1398: panic!("{reach:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1473: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1597: Ask::All => panic!("the day was closed done: read again for ZQB only"),
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1621: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1666: panic!("{:?}", refused.failures);
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1701: scopes_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1761: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1780: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1813: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1827: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_import_day_ps1_tests.rs:1858: panic!("{got:?}");
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:428:9 False: let Owed::Probe(early) = ctx.owed(&own(&key, april(1), true)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:433:9 False: let Owed::Probe(beyond) = plain.owed(&own(&key, april(1), false)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:477:9 False: let Owed::Pending(members) = ctx.owed(&own(&key, april(1), false)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:499:9 False: let Owed::Probe(one) = ctx.owed(&own(&first, april(1), false)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:502:9 False: let Owed::Probe(two) = ctx.owed(&own(&second, april(2), false)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:534:9 False: let [refused] = report.failures.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:539:9 False: refused.why.starts_with("the import journal: ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:540:16 False: && refused.why.contains(&ledger.display().to_string()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:591:13 False: assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:591:46 False: assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:772:66 False: last_line_of(&store, &member_key).is_some_and(|line| line
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:825:9 False: let [line] = lines.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:939:9 False: let [held] = key_held.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:947:9 False: let [finished] = run_finished.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:956:9 False: let [unwritten] = not_written.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:981:9 False: let [said] = unrecorded_said.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1016:9 False: said.starts_with("run 1790100201 ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1017:16 False: && said.contains(&format!("\ndone indices {} * ", april(1))),
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1051:9 False: let ImportRefusal::Stopped {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1092:9 False: let ImportRefusal::Journal { why } = &got else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1127:9 False: let ImportRefusal::StoreLost {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1170:9 False: let ImportRefusal::StoreLost {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1181:9 False: why.starts_with(&format!(
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1184:15 False: )) && why.contains("File exists"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1214:9 False: let ImportRefusal::StoreLost {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1280:9 False: let ImportRefusal::StoreNotWritable {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1397:9 False: let [said] = reach.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1665:9 False: let [named] = refused.failures.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1754:9 False: let ImportRefusal::SourceLost {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1777:33 False: .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1806:9 False: let ImportRefusal::StoreNotWritable {
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:9 False: why.contains("planted by the test") && why.contains("chflags nouchg"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:48 False: why.contains("planted by the test") && why.contains("chflags nouchg"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1824:33 False: .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
UNCOVERED BRANCH crates/pull/src/gdfl_import_day_ps1_tests.rs:1857:9 False: let ImportRefusal::StoreLost { why, .. } = &got else {
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:116: Ok(store::provenance::Recorded::NothingAppended) => Ok(false),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:217: why: "the day's begin line was not written".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:218: }));
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:311: break;
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:335: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:336: &telemetry::Event::warn(TARGET, "fewer workers than asked for")
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:337: .with("asked", want)
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:338: .with("started", handles.len())
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:339: .with("why", why),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:340: );
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:352: Err(panicked) => std::panic::resume_unwind(panicked),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:499: Ok(_) => Some(format!(
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:500: "the loss probe found {} is no longer a directory: the store's root went away",
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:501: store_root.display()
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:502: )),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:661: filed.store_lost = Some(format!("{why}; {gone}"));
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:718: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:719: let refusal = ImportRefusal::Filing { why };
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:720: filed
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:721: .failures
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:722: .push(note_staged(run, self.day, &file.name, &refusal));
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:723: }
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:746: _ => ImportRefusal::Filing { why: failure.why },
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1013: why: "the day's begin line was not written".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1014: })
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1321: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1322: let day_text = self.day.to_string();
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1323: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1324: &telemetry::Event::warn(
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1325: TARGET,
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1326: "no worker thread started; the day's files run on one thread",
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1327: )
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1328: .with("day", day_text.as_str())
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1329: .with("why", why.as_str()),
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1330: );
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1331: }
UNCOVERED LINE crates/pull/src/gdfl_import_filing.rs:1541: _ => "store_not_writable",
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:310:28 True: if stop.is_some_and(crate::gdfl_run::stop::StopControl::requested) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:334:16 True: if let Some(why) = refused.as_deref() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:347:32 False: if let Some(slot) = slots.get_mut(index) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:487:21 False: Ok(meta) if meta.is_dir() => match at_start {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:660:24 True: if let Some(gone) = store_root_lost(run.store_root, self.store_id) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:687:20 False: && let Some(held) = done.pending.as_ref()
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:739:31 False: Some(kind) if at + 1 == last => {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:952:38 False: self.origin.is_some() && handed.facts.member.is_some() && self.source.is_some();
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:1221:16 False: if self.held_back_cap.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:1411:12 False: if self.disk_full.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:1417:12 False: if self.not_writable.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:1614:58 False: let Some(at) = after.iter().position(|(r, name)| r == root && name == file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_filing.rs:1621:12 False: if std::fs::rename(root, &gone).is_ok() {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:119: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:271: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:464: fn journal_append_cost_is_flat_with_a_second_writer() {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:465: for size in [1_000_u32, 20_000] {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:466: let store = scratch(&format!("flat-{size}"));
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:467: std::fs::create_dir_all(store.join("imports")).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:468: let path = journal_path(&store);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:469: let mut seed = String::new();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:470: for i in 0..size {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:471: let _ = writeln!(seed, "begin indices 2024-04-01 S{i}");
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:472: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:473: std::fs::write(&path, seed).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:474: let mine = Journal::load(path.clone()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:475: let other = Journal::load(path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:476: let mut took = Vec::with_capacity(300);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:477: for i in 0..300_u32 {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:478: other
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:479: .append(&format!("begin stocks 2024-04-01 O{i}"))
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:480: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:481: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:482: mine.append(&format!("begin indices 2024-04-01 M{i}"))
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:483: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:484: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:485: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:486: took.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:487: println!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:490: took[150], took[296], took[299]
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:493: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:708: fn ia2_repair2_timings() {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:709: fn pct(mut v: Vec<std::time::Duration>) -> String {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:710: v.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:711: let n = v.len();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:712: format!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:714: v[n / 2],
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:715: v[n * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:716: v[n - 1]
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:718: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:719: let keys = 100_000_u32;
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:720: let lines: Vec<String> = (0..keys)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:721: .flat_map(|i| {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:722: let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:723: [
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:724: format!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:725: "incomplete {key} definition=4 files=1 seconds={} failures=1 stage=file \
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:726: reason=filing at=1790000001 run={}",
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:727: i % 2,
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:728: 1_790_000_000 + u64::from(i)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:729: ),
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:730: format!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:731: "done {key} definition=4 files=1 seconds=60 failures=0 at=1790000002 run={}",
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:732: 1_790_100_000 + u64::from(i)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:733: ),
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:734: ]
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:735: })
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:736: .collect();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:737: let mut journal = Journal::default();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:738: let mut took = Vec::with_capacity(lines.len());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:739: for line in &lines {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:740: let fields: Vec<&str> = line.split(' ').collect();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:741: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:742: journal.read_line(&fields).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:743: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:744: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:745: println!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:747: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:749: let mut took = Vec::with_capacity(100_000);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:750: for i in 0..100_000_u32 {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:751: let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:752: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:753: let got = journal.history(&key);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:754: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:755: assert!(got.last_done.is_some());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:757: println!("IA-2 r2 history probe, 100,000 keys: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:759: let store = scratch("r2-append");
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:760: let ledger = store.join("imports").join("gdfl.lineage");
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:761: std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:762: let mut seed = String::with_capacity(16_000);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:763: for i in 0..1_000 {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:764: writeln!(seed, "alias seed{i}").unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:765: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:766: std::fs::write(&ledger, seed).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:767: let mut took = Vec::with_capacity(300);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:768: for i in 0..300 {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:769: let line = vec![format!("alias timed{i}")];
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:770: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:771: append_lines(&ledger, &line).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:772: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:773: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:774: println!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:776: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:780: let lock = journal_lock_path(&ledger);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:781: let mut took = Vec::with_capacity(1_000);
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:782: for _ in 0..1_000 {
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:783: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:784: crate::gdfl_run::locks::take_path_within(&lock, crate::gdfl_run::locks::LOCK_WAIT)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:785: .map(drop)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:786: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:787: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:788: }
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:789: println!(
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:791: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_import_ia2_tests.rs:793: }
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:115:9 False: let ImportRefusal::Stopped {
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:142:9 False: closed.starts_with("incomplete indices 2024-04-02 * ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:143:16 False: && closed.contains(" stage=run reason=stopped at=")
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:144:16 False: && closed.ends_with(" run=1790000101"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:151:9 False: last.starts_with("run 1790000101 ended=")
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:152:16 False: && last.contains(" outcome=stopped ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:153:16 False: && last.ends_with(&format!(" definition={BAR_DEFINITION}")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:204:71 False: matches!(&got, ImportRefusal::Stopped { day: Some(d), .. } if *d == day(2)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:225:77 False: matches!(&got, ImportRefusal::Stopped { day: Some(d), done, .. } if *d == day(1) && done.days_imported == 1),
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:225:93 False: matches!(&got, ImportRefusal::Stopped { day: Some(d), done, .. } if *d == day(1) && done.days_imported == 1),
UNCOVERED BRANCH crates/pull/src/gdfl_import_ia2_tests.rs:267:9 False: let ImportRefusal::Stopped {
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:698: let _dropped_when_filtered = telemetry::emit(&said);
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:785: Some(restated) => {
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:786: self.auction_restated.insert(of_day.clone(), restated);
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:787: }
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:798: return;
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:801: return;
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:896: continue;
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1093: Some(ended) => filters.insert(only.to_owned(), ended),
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1304: }
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1316: why: format!("{}: {fault}", self.path.display()),
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1317: })?;
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1376: return Ok(());
UNCOVERED LINE crates/pull/src/gdfl_import_journal.rs:1471: }
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:561:28 True: || !self.whole_every.is_empty()
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:697:16 True: if let Some(said) = load_warning(&shown, bytes, lines) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:745:20 False: if let Some((day, only)) = key.rsplit_once(' ')
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:797:13 False: let Some((day, only)) = key.rsplit_once(' ') else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:800:12 True: if only.contains('+') {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:810:16 False: if filters.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:893:17 False: let (Some(verb), Some(kind), Some(on), Some(only)) =
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1030:24 False: if !ended.whole.iter().any(|held| held == scope) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1089:28 False: && let Some((before, _)) = self.begun_early.remove(&key)
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1090:28 False: && let Some(filters) = self.auction_keys.get_mut(&format!("{kind} {day}"))
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1302:16 False: if let Some(dir) = self.path.parent() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1375:12 True: if text.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1448:21 False: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && dir.is_dir() => Ok(()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1448:72 True: Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && dir.is_dir() => Ok(()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_journal.rs:1469:12 False: if let Some(dir) = path.parent() {
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:219: }
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:220: if self.lots.0 != self.lots.1 {
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:221: return EventKind::LotStep;
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:222: }
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:223: self.forward
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:224: .iter()
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:225: .find_map(|(&old, &new)| {
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:226: let shift = contract_parts(old)?.1.checked_sub(contract_parts(new)?.1)?;
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:227: (shift != 0).then_some(EventKind::StrikeOffset { shift })
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:228: })
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:229: .unwrap_or(EventKind::Relisted)
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:346: }
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:366: }
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:407: Err(why) => Err(format!("{}: {why}", path.display())),
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:419: continue;
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:612: continue;
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:658: }
UNCOVERED LINE crates/pull/src/gdfl_import_lineage.rs:695: break;
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:161:16 False: if let Some(expiry) = expiry_of(key)
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:217:16 False: if let Some(kind) = self.kind {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:220:12 True: if self.lots.0 != self.lots.1 {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:220:12 False: if self.lots.0 != self.lots.1 {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:326:20 False: if let Some(alias) = AliasRecord::parse(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:331:20 False: if let Some(record) = EventRecord::parse(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:340:20 False: if let Some(record) = LineageRecord::parse(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:349:20 False: if let Some(record) = StoppedRecord::parse(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:357:20 False: if let Some(record) = StitchedRecord::parse(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:406:25 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:418:17 False: let Some(record) = LotRecord::parse(line) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:435:16 False: if expiry_of(record.contract)
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:562:20 False: if let Some(record) = entry.stitched(underlying, key) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:566:49 True: if (had && !fresh && entry.recorded) || entry.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:570:12 False: if !keeps {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:611:17 False: let Some(underlying) = todays.first().map(|c| c.ticker.underlying) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:653:28 False: if let Some(record) = retired.stitched(underlying, contract) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_lineage.rs:694:13 False: let Some(prev) = u32::try_from(at).ok().and_then(|n| Day::from_days(n).ok()) else {
UNCOVERED LINE crates/pull/src/gdfl_import_ps1_tests.rs:171: panic!("one failure, not {:?}", extras.failures);
UNCOVERED BRANCH crates/pull/src/gdfl_import_ps1_tests.rs:170:9 False: let [staged] = extras.failures.as_slice() else {
UNCOVERED LINE crates/pull/src/gdfl_import_quality.rs:723: pub const fn files(&self) -> u64 {
UNCOVERED LINE crates/pull/src/gdfl_import_quality.rs:724: self.files
UNCOVERED LINE crates/pull/src/gdfl_import_quality.rs:725: }
UNCOVERED LINE crates/pull/src/gdfl_import_quality.rs:796: }
UNCOVERED LINE crates/pull/src/gdfl_import_quality.rs:1024: continue;
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:460:16 False: && tick.ltp != 0
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:777:16 False: if let Some(first) = file.first_trade {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:789:16 False: if to < self.shared.len() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:790:24 False: if let Some(slot) = self.shared.get_mut(from) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:793:24 False: if let Some(slot) = self.shared.get_mut(to) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:813:38 True: if self.files_traded == 0 || self.shared.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:1009:20 False: if let Some((key, unsourced)) = keyed(line) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_quality.rs:1023:17 False: let Some((key, unsourced)) = keyed(&line) else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_scope.rs:327:17 False: assert!(none.members().is_empty() && none.set().is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_import_scope.rs:327:46 False: assert!(none.members().is_empty() && none.set().is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_import_scope.rs:374:17 False: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
UNCOVERED BRANCH crates/pull/src/gdfl_import_scope.rs:374:57 False: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:273: for record in bytes.chunks_mut(64) {
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:274: for at in (48..56).chain(60..64) {
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:275: if let Some(byte) = record.get_mut(at) {
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:276: *byte = 0;
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:277: }
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:861: keep(&mut out, h);
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:862: }
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:911: String::from_utf8(here).unwrap()
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:916: println!(
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:921: return;
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:931: gone.push(line);
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:937: new.push(line);
UNCOVERED LINE crates/pull/src/gdfl_import_split_tests.rs:954: }
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:269:16 True: if rel
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:275:32 True: if let Some(byte) = record.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:275:32 False: if let Some(byte) = record.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:910:23 True: let text = if sha256_hex(&here) == *sha {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:912:23 False: } else if let Some(text) = blob(blob_id) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:925:12 False: if let Some(before_text) = blob(BEFORE_BLOB) {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:930:16 True: if have < *n {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:936:16 True: if before.kept.get(line).copied().unwrap_or(0) < *n {
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:941:13 False: gone.is_empty() && new.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_split_tests.rs:941:32 False: gone.is_empty() && new.is_empty(),
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:382: texts[8]
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:387: texts[9]
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:558: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:1070: matches!(
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:2074: bin.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:2630: panic!("{got:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:4440: panic!("{rel}: not a bar month of the store");
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:4707: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:4898: indices[0].0
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:4948: bytes.len()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5186: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5382: durability.as_str()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5388: durability.as_str()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5394: durability.as_str(),
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5395: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5521: panic!("the run stopped at the day's flush: {refused:?}");
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5675: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5685: one.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5837: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5976: Err(std::io::Error::other(format!(
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5977: "{word}: the back-fill day's batch flushed {}",
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5978: path.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5979: )))
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:5980: }),
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:6036: rel.display()
UNCOVERED LINE crates/pull/src/gdfl_import_tests.rs:6389: panic!("the run is refused at the census: {got:?}")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:247:40 False: assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:380:9 False: texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:380:63 False: texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:400:13 False: assert!(all[0].report().is_none() && all[11].report().is_none());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:400:42 False: assert!(all[0].report().is_none() && all[11].report().is_none());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:550:9 False: let ImportRefusal::CensusFull {
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:562:13 False: assert!(!measured.fits() && measured.free() == 1);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:562:33 False: assert!(!measured.fits() && measured.free() == 1);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:616:19 False: )) && !l.contains(" stage=")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:623:9 False: last.starts_with(&format!(
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:625:15 False: )) && last.contains(" failures=1 ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:626:16 False: && last.contains(" stage=census reason=too_many_entries run="),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:646:9 False: journal.contains("\ndone options 2024-04-04 * ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:647:16 False: && journal.contains("\ndone options 2024-04-05 * "),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:685:9 False: text.contains("refused stocks 2024-04-01 although the day's batch fits")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:686:16 False: && text.contains("another ingest holds the census lock"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:730:13 False: assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:730:27 False: assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1028:13 False: assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1028:47 False: assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1029:13 False: assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1029:47 False: assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1031:9 False: journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1031:43 False: journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1034:13 False: assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1034:53 False: assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1042:9 False: journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1042:43 False: journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1045:13 False: assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1045:51 False: assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1364:13 False: assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1364:38 False: assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1455:9 False: damaged.digest.is_none()
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1456:16 False: && damaged
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1489:9 False: unformatted.digest.is_none()
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1490:16 False: && unformatted
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1632:9 False: second.contains(" source=zip ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1633:16 False: && second.contains(" commit=unstamped ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1634:16 False: && second.contains(" started=1750000000 ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1635:16 False: && second.ends_with(&format!(
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1656:9 False: first.starts_with("run ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1657:16 False: && first.contains(
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1704:9 False: line.starts_with("incomplete indices 2024-04-01 ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1707:16 False: && line.contains(" stage=file reason=bars_differ_held run="),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1719:9 False: line.starts_with("incomplete options 2024-04-01 ")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:1720:16 False: && line.contains(" stage=vendor reason="),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2010:13 False: ra.seconds > 0 && ra.seconds_committed == ra.seconds,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2010:31 False: ra.seconds > 0 && ra.seconds_committed == ra.seconds,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2022:13 False: assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2022:41 False: assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2037:22 False: .all(|l| l.contains(" member_bytes=0 ") && l.ends_with(" member_src=none")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2042:22 False: .all(|l| !l.contains(" member_bytes=0 ") && l.ends_with(" member_src=zip")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2083:22 False: .all(|b| b.volume == 0 && b.open_interest == OI_NULL)
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2092:31 False: assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000));
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2622:9 False: let ImportRefusal::CensusRefused {
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2636:34 False: .is_err_and(|unread| unread.contains("gdfl.man") && unread.contains("not a regular")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2644:22 False: .any(|f| f.instrument == "2024-04-01 census" && f.why.contains("nothing of the batch")),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2650:9 False: text.contains("indices 2024-04-01")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2651:16 False: && text.contains("its room could not be measured")
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2652:16 False: && text.contains("the run stopped there after 0 day(s) imported"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2659:9 False: last.starts_with(&format!(
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2661:15 False: )) && last.contains(" stage=census reason=census_refused run="),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:2913:74 False: matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder"))
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4356:20 False: || crate::gdfl_lineage::EventRecord::parse(&line).is_some(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4439:17 False: let [_, _, _, _, under, contract, ..] = parts.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4683:17 False: assert!(one.provenance_records > 0 && one.seconds_committed > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4683:47 False: assert!(one.provenance_records > 0 && one.seconds_committed > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4896:9 False: indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4896:36 False: indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4944:21 False: u64::try_from(bytes.len()).unwrap() <= store::format::HEADER_LEN
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4945:28 False: && rest.iter().all(|&byte| byte == 0)
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:4946:28 False: && store::header::Header::decode(slot).is_err(),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5128:17 False: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5128:38 False: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5128:60 False: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5128:75 False: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5133:17 False: status.success() && !open_day && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5133:37 False: status.success() && !open_day && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5133:50 False: status.success() && !open_day && unborn == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5140:13 False: unborn == 0 || open_day,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5140:28 True: unborn == 0 || open_day,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5140:28 False: unborn == 0 || open_day,
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5374:13 False: assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5374:50 False: assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5514:9 False: let Err(ImportRefusal::Durability {
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5525:9 False: why.contains("syncing") && why.contains(&store.display().to_string()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5525:36 False: why.contains("syncing") && why.contains(&store.display().to_string()),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5820:13 False: last.starts_with(&format!("incomplete options {}", d2()))
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5821:20 False: && last.contains(&format!(" failures={} ", refused + 1)),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5908:13 False: assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:5908:41 False: assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:6181:46 False: why.contains("holds") || why.contains("lock"),
UNCOVERED BRANCH crates/pull/src/gdfl_import_tests.rs:6388:9 False: let Err(ImportRefusal::CensusRefused { day: at, why, .. }) = &got else {
UNCOVERED LINE crates/pull/src/gdfl_lineage.rs:591: return (0, 1);
UNCOVERED LINE crates/pull/src/gdfl_lineage.rs:1072: _ => None,
UNCOVERED LINE crates/pull/src/gdfl_lineage.rs:1653: unpaired.push(old_contract);
UNCOVERED LINE crates/pull/src/gdfl_lineage.rs:1654: continue;
UNCOVERED LINE crates/pull/src/gdfl_lineage.rs:1698: continue;
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:590:8 True: if num == 0 || den == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:590:20 True: if num == 0 || den == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:834:36 False: Some((under, contract)) if under == underlying => Ok(Some(contract)),
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:915:41 False: (basis == LotEvidence::BASIS && version == RECORD_VERSION && record.line() == line)
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:1184:24 False: (consistent && version == RECORD_VERSION && record.line() == line).then_some(record)
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:1652:13 False: let Some(new_contract) = new.contract() else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:1697:13 False: let Some(old_contract) = old.contract() else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage.rs:1799:43 False: None if name_break || (sourced && relisted > 0) => EventKind::Relisted,
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1093: panic!("an old name on its change day");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1148: fn the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day() {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1149: let strikes: Vec<i64> = (0..50).map(|i| 1_000_000 + i * 5_000).collect();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1150: let mut day = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1151: for expiry in [
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1152: d(2020, 9, 17),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1153: d(2020, 9, 24),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1154: d(2020, 10, 1),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1155: d(2020, 10, 29),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1156: ] {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1157: day.extend(ladder(expiry, &strikes, 75));
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1158: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1159: assert_eq!(day.len(), 400);
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1160: let mut nanos: Vec<u128> = (0..2_000)
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1161: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1162: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1163: let got = detect(sym("NIFTY"), d(2020, 9, 16), &day, &day);
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1164: let took = at.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1165: assert_eq!(got, None);
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1166: took
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1167: })
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1168: .collect();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1169: nanos.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1170: println!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1172: nanos[1_000], nanos[1_980], nanos[1_999]
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1174: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1178: fn joined_across_renames(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1179: keys: &HashMap<String, std::collections::BTreeMap<String, (Day, Day)>>,
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1180: ) -> std::collections::BTreeMap<String, u32> {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1182: let mut joined: BTreeMap<String, u32> = BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1183: for (key, stems) in keys {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1184: if stems.len() < 2 {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1185: continue;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1186: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1187: let mut lives: Vec<(&String, &(Day, Day))> = stems.iter().collect();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1188: lives.sort_by_key(|(_, life)| life.0.days_from_epoch());
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1189: for pair in lives.windows(2) {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1190: assert!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1191: pair[0].1.1.days_from_epoch() < pair[1].1.0.days_from_epoch(),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1195: let label = lives
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1196: .iter()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1197: .map(|(stem, _)| stem.as_str())
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1198: .collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1199: .join("->");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1200: *joined.entry(label).or_default() += 1;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1202: joined
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1203: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1216: fn the_real_names_are_one_key_per_contract_across_renames_and_redatings() {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1218: let path = std::env::var_os("BRUTEX_NAME_CENSUS").expect("BRUTEX_NAME_CENSUS");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1219: let text = std::fs::read_to_string(path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1221: let mut keys: HashMap<String, BTreeMap<String, (Day, Day)>> = HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1222: let (mut names, mut decoded, mut stale, mut aliased) = (0_u32, 0_u32, Vec::new(), 0_u32);
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1223: let mut refused: BTreeMap<String, u32> = BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1224: for line in text.lines() {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1225: let mut fields = line.split('\t');
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1226: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1228: continue;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1230: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1231: continue;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1233: names += 1;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1234: let mut both = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1235: for on in [first, last] {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1236: match crate::gdfl_nfo::decode_ticker(name, on) {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1237: Ok(ticker) => both.push(ticker),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1238: Err(crate::gdfl_nfo::NfoRefusal::UnderlyingStale { .. }) => {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1239: stale.push(format!("{name} on {on}"));
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1240: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1241: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1242: let word = format!("{why:?}");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1243: let word = word.split([' ', '{']).next().unwrap_or_default().to_owned();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1244: *refused.entry(word).or_default() += 1;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1245: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1248: let [a, b] = both.as_slice() else {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1249: continue;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1251: decoded += 1;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1252: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1253: (a.underlying, a.contract),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1254: (b.underlying, b.contract),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1257: aliased += u32::from(matches!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1258: a.settlement,
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1261: let stem_len = name.len() - 2;
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1262: let stem: String = name[..stem_len]
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1263: .chars()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1264: .take_while(|c| !c.is_ascii_digit())
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1265: .collect();
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1266: let slot = keys
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1267: .entry(option_key(a.underlying, a.contract))
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1268: .or_default()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1269: .entry(stem)
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1270: .or_insert((first, last));
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1271: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1272: first
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1274: slot.0
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1276: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1277: last
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1279: slot.1
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1282: let joined = joined_across_renames(&keys);
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1283: println!("names {names}, decoded on both days {decoded}, aliased names {aliased}");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1284: println!("refused (first or last day), by kind: {refused:?}");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1285: println!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1287: joined.values().sum::<u32>(),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1288: joined.len()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1290: for (label, n) in &joined {
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1291: println!("  {label}: {n}");
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1292: }
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1293: assert!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1294: stale.is_empty(),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1296: stale.len(),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1297: stale.first()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1299: assert!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1300: joined.values().sum::<u32>() > 0,
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1303: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1305: crate::expiry_alias::ALIASES
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1306: .iter()
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1307: .map(|row| row.contracts)
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1308: .sum::<u32>(),
UNCOVERED LINE crates/pull/src/gdfl_lineage_tests.rs:1311: }
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1092:9 False: let Err(Stale::AfterChange(change)) = filed("NIITTECH", d(2020, 8, 20)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1135:9 False: said.contains("NIITTECH27AUG201900CE")
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1136:16 False: && said.contains("NIITTECH became COFORGE on 2020-08-20"),
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1184:12 True: if stems.len() < 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1184:12 False: if stems.len() < 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1226:13 True: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1226:13 False: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1230:13 True: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1230:13 False: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1248:13 True: let [a, b] = both.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1248:13 False: let [a, b] = both.as_slice() else {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1271:21 True: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1271:21 False: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1276:21 True: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
UNCOVERED BRANCH crates/pull/src/gdfl_lineage_tests.rs:1276:21 False: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:312: misread.push(format!("{ticker} on {on} -> {}", got.contract.as_str()));
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:313: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:318: NfoRefusal::MonthlyExpiryUnmeasured { .. } => unstated += 1,
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:322: NfoRefusal::TickerAmbiguous { .. } => ambiguous += 1,
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:328: why => panic!("{ticker} on {on}: {why:?}"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:331: && !matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:335: {
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:336: missed.push(format!("{ticker} on {on}: {why:?}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:352: misread.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:353: &misread[..8.min(misread.len())]
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:358: missed.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:359: &missed[..8.min(missed.len())]
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:368: return None;
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:371: return None;
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:553: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:650: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:751: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:811: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:953: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1048: got.contract.as_str(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1049: reencode(&got)
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1068: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1069: on >= d(2018, 9, 3),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1072: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1073: is_index(got.underlying.as_str()),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1076: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1077: !monthly_plausible(&ticker, on),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1103: return false;
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1115: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1384: NfoRefusal::HeaderUnknown { .. } => panic!("the header was intact"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1389: | NfoRefusal::OpenInterestRefused { line } => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1397: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1399: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1742: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1750: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1932: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_attack_tests.rs:1951: matches!(
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:301:27 False: } else if is_index(got.underlying.as_str())
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:302:24 False: && on >= d(2018, 9, 3)
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:303:24 False: && got.form == TickerForm::Day
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:304:24 False: && reencodes(&ticker, &got)
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:331:24 True: && !matches!(
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:367:8 True: if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || whole.len() > 15 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:367:28 True: if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || whole.len() > 15 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:367:74 True: if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || whole.len() > 15 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:370:8 True: if !frac.bytes().all(|b| b.is_ascii_digit()) || frac.len() > 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:370:53 True: if !frac.bytes().all(|b| b.is_ascii_digit()) || frac.len() > 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:414:12 True: if (year, month) < (on.year(), on.month())
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:416:16 True: || d(year, month, 1).days_from_epoch() > on.days_from_epoch() + EXPIRY_HORIZON_DAYS
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:675:9 False: rekeyed > 1_000 && unsettled > 1_000,
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:675:28 False: rekeyed > 1_000 && unsettled > 1_000,
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:998:16 False: if out.is_char_boundary(at) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1032:24 False: && monthly_reference(&ticker, on)
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1063:21 False: expiry >= on
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1064:28 False: && expiry.days_from_epoch() <= on.days_from_epoch() + EXPIRY_HORIZON_DAYS
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1067:20 True: if on < cutover() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1085:25 False: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1085:70 True: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1085:70 False: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1099:9 False: let Some(body) = ticker
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1114:12 True: if strike_paisa(&rest[5..]).is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1118:12 False: if first.end_of_month() >= on
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1375:29 False: assert!(row.ltp >= 0 && row.oi >= 0);
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1375:45 False: assert!(row.ltp >= 0 && row.oi >= 0);
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1376:45 False: assert!(row.ltq == 0 || row.ltp >= costs::rate::TICK.raw());
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1391:33 False: assert!(at >= 2 && at <= n + 2, "line {line} of {n} rows");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1391:44 False: assert!(at >= 2 && at <= n + 2, "line {line} of {n} rows");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1392:32 False: if let Some(first) = bent_at {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1404:13 False: assert!(ok > 1_000 && refused > 1_000, "{ok} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1404:27 False: assert!(ok > 1_000 && refused > 1_000, "{ok} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1516:17 False: !ticker.is_empty() && !ticker.contains(['\\', '/']),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1516:39 False: !ticker.is_empty() && !ticker.contains(['\\', '/']),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_attack_tests.rs:1522:24 False: || entry.ends_with(&format!("{ticker}.NFO.CSV")),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:295: matches!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:314: fn day(&self, _: Day) -> Result<Option<NfoDay<u8>>, CmRefusal> {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:315: unreachable!("read_file is handed its listing: it never asks the source for a day")
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:358: panic!("{name}: {got:?}")
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1244: fn the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1245: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1246: let bound = |var: &str, fallback: Day| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1247: std::env::var(var).ok().map_or(fallback, |text| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1248: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1249: day(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1250: parts[0],
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1251: u8::try_from(parts[1]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1252: u8::try_from(parts[2]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1254: })
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1255: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1256: let (from, to) = (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1257: bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1258: bound("BRUTEX_WP12B_TO", day(2026, 9, 24)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1259: );
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1260: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1261: let (mut days, mut names, mut rekeyed) = (0_u64, 0_u64, 0_u64);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1262: let mut rekeyed_names: HashSet<String> = HashSet::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1263: let mut unsettled: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1264: let mut implausible: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1267: let mut collisions: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1268: let mut by_rule: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1271: let mut high: Vec<(i64, Day, String)> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1272: let mut low: Vec<(i64, Day, String)> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1273: let mut on = from;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1274: while on <= to {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1275: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1276: days += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1277: let mut seen: HashMap<(String, String), String> = HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1278: let mut ladders: HashMap<String, (i64, i64)> = HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1279: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1280: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1281: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1283: names += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1284: match decode_ticker(name, on) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1285: Ok(ticker) => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1286: if ticker.expiry != ticker.stated {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1287: rekeyed += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1288: rekeyed_names.insert(name.to_owned());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1289: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1290: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1291: *by_rule
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1292: .entry(format!("{} {}", ticker.underlying.as_str(), ticker.stated))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1293: .or_default() += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1294: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1295: let key = (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1296: ticker.underlying.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1297: ticker.contract.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1298: );
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1299: if let Some(first) = seen.insert(key, name.to_owned()) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1300: collisions.push(format!("{on} {first} {name}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1301: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1302: if listing.check_strike(&ticker, name).is_err() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1303: implausible.push(format!("{on} {name}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1304: } else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1305: let span = ladders
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1306: .entry(ticker.underlying.as_str().to_owned())
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1307: .or_insert((ticker.strike, ticker.strike));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1308: span.0 = span.0.min(ticker.strike);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1309: span.1 = span.1.max(ticker.strike);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1310: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1312: Err(NfoRefusal::ExpiryUnsettled { why, .. }) => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1313: unsettled.push(format!("{on} {name}: {why}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1314: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1315: Err(_) => {}
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1318: for (under, (lo, hi)) in ladders {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1319: let median = listing.strike_median(&under).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1320: high.push((hi * 100 / median, on, under.clone()));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1321: if lo > 0 {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1322: low.push((median * 100 / lo, on, under));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1323: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1325: high.sort_unstable_by(|a, b| b.cmp(a));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1326: high.truncate(3);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1327: low.sort_unstable_by(|a, b| b.cmp(a));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1328: low.truncate(3);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1329: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1330: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1332: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1334: rekeyed_names.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1335: by_rule.values().sum::<u64>(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1336: unsettled.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1337: collisions.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1338: implausible.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1339: &unsettled[..unsettled.len().min(20)],
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1340: &collisions[..collisions.len().min(20)]
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1342: assert!(days > 0, "no vendor day in range");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1343: assert!(unsettled.is_empty(), "{unsettled:?}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1344: assert!(collisions.is_empty(), "{collisions:?}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1347: assert!(by_rule.is_empty(), "{by_rule:?}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1349: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1350: high.iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1351: .all(|(ratio, ..)| *ratio < STRIKE_PLAUSIBLE_MULTIPLE * 100),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1354: let impossible = [
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1355: "2019-05-30 NIFTY25JUN201912500CE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1356: "2019-05-30 NIFTY25JUN201912700CE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1357: "2019-05-30 NIFTY25JUN201913000CE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1358: ];
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1359: if (from..=to).contains(&day(2019, 5, 30)) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1360: implausible.sort();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1361: assert_eq!(implausible, impossible);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1363: assert!(implausible.is_empty(), "{implausible:?}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1365: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1366: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1367: u32::try_from(rekeyed_names.len()).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1368: crate::expiry_alias::ALIASES
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1369: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1370: .map(|row| row.contracts)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1371: .sum::<u32>(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1374: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1375: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1387: fn the_real_names_decode_to_the_stored_contracts_less_the_refused_and_the_resettled() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1388: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1389: let store =
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1390: std::path::PathBuf::from(std::env::var_os("BRUTEX_STORE_FNO").expect("BRUTEX_STORE_FNO"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1391: let unders = std::env::var("BRUTEX_WP12B_UNDERLYINGS").unwrap_or_else(|_| "NIFTY".to_owned());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1392: let bound = |var: &str, fallback: Day| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1393: std::env::var(var).ok().map_or(fallback, |text| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1394: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1395: day(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1396: parts[0],
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1397: u8::try_from(parts[1]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1398: u8::try_from(parts[2]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1400: })
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1401: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1402: let (from, to) = (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1403: bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1404: bound("BRUTEX_WP12B_TO", day(2020, 9, 15)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1405: );
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1406: let wanted: HashSet<&str> = unders.split(',').collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1407: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1408: let mut decoded: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1409: let mut refused: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1410: let mut refused_contracts: HashSet<(String, String)> = HashSet::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1412: let mut resettled: std::collections::BTreeMap<(String, String), (String, String, &str)> =
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1413: std::collections::BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1414: let mut on = from;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1415: while on <= to {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1416: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1417: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1418: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1419: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1421: match decode_ticker(name, on) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1422: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1423: if let Err(why) = listing.check_strike(&ticker, name) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1424: refused.push(format!("{on} {why}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1425: refused_contracts.insert((
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1426: ticker.underlying.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1427: ticker.contract.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1428: ));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1429: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1430: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1431: if ticker.expiry != ticker.stated {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1432: let stated = crate::gdfl_lineage::ContractDay {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1433: expiry: ticker.stated,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1434: strike: ticker.strike,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1435: side: ticker.side,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1436: lot: crate::gdfl_lineage::LotEvidence::NoTrade,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1437: first_oi: None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1438: close_oi: None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1439: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1440: .contract()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1441: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1442: resettled.insert(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1443: (ticker.underlying.as_str().to_owned(), name.to_owned()),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1444: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1445: stated.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1446: ticker.contract.as_str().to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1447: ticker.settlement.as_str(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1448: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1449: );
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1450: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1451: decoded
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1452: .entry(ticker.underlying.as_str().to_owned())
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1453: .or_default()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1454: .insert(ticker.contract.as_str().to_owned());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1456: Ok(_) => {}
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1457: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1458: if wanted.iter().any(|u| name.starts_with(u)) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1459: refused.push(format!("{on} {name}: {why}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1460: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1464: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1465: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1467: for under in &wanted {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1468: let held: std::collections::BTreeSet<String> = std::fs::read_dir(store.join(under))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1469: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1470: .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1471: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1472: let mine = decoded.remove(*under).unwrap_or_default();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1473: let only_held: Vec<&String> = held.difference(&mine).collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1474: let only_mine: Vec<&String> = mine.difference(&held).collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1475: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1477: mine.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1478: held.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1479: only_held.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1480: only_mine.len()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1488: let closed = |seg: &str| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1489: let day = crate::gdfl_lineage::parse_day(&seg[..10]).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1490: crate::calendar::sourced_kind_of(i64::from(day.days_from_epoch()))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1491: == crate::calendar::DayKind::Closed
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1492: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1493: let mut changed = 0_usize;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1494: for ((_, name), (stated, settled, _)) in resettled.iter().filter(|((u, _), _)| u == under) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1495: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1496: held.contains(stated),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1499: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1500: mine.contains(settled) && !held.contains(settled),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1503: changed += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1505: for (u, seg) in refused_contracts.iter().filter(|(u, _)| u == under) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1506: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1507: changed += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1509: let other_held: Vec<&&String> = only_held
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1510: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1511: .filter(|seg| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1512: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1513: })
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1514: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1515: let other_mine: Vec<&&String> = only_mine
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1516: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1517: .filter(|seg| !resettled.values().any(|(_, settled, _)| settled == **seg))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1518: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1519: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1523: eprintln!("refused {}: {refused:?}", refused.len());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1524: eprintln!("resettled {}: {resettled:?}", resettled.len());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1525: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1533: fn the_real_ex_dates_are_detected_from_the_data_and_ordinary_days_are_not() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1535: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1536: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1538: let day_of = |under: &str, on: Day| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1539: let mut out: Vec<(ContractDay, Vec<(u32, i64)>)> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1540: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1541: return out;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1543: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1544: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1545: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1547: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1548: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1550: if ticker.underlying.as_str() != under {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1551: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1552: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1553: let read = read_file(&zips, &listing, file).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1554: let rows: Vec<(u32, u64, i64)> = read
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1555: .file
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1556: .rows
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1557: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1558: .map(|r| (r.sod, r.ltq, r.oi))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1559: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1560: out.push((
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1561: ContractDay::from_rows(&read.ticker, rows.iter().copied()),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1562: rows.iter().map(|&(sod, _, oi)| (sod, oi)).collect(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1565: out
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1566: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1569: let cases = [
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1570: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1571: "BRITANNIA",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1572: day(2018, 11, 28),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1573: day(2018, 11, 29),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1574: day(2018, 11, 30),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1575: (2, 1),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1576: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1577: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1578: "NTPC",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1579: day(2019, 3, 18),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1580: day(2019, 3, 19),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1581: day(2019, 3, 20),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1582: (6, 5),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1583: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1584: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1585: "HCLTECH",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1586: day(2019, 12, 4),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1587: day(2019, 12, 5),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1588: day(2019, 12, 6),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1589: (2, 1),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1590: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1591: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1592: "BAJAJFINSV",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1593: day(2022, 9, 12),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1594: day(2022, 9, 13),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1595: day(2022, 9, 14),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1596: (10, 1),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1597: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1598: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1599: "RELIANCE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1600: day(2024, 10, 25),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1601: day(2024, 10, 28),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1602: day(2024, 10, 29),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1603: (2, 1),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1604: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1605: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1606: "BAJFINANCE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1607: day(2025, 6, 13),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1608: day(2025, 6, 16),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1609: day(2025, 6, 17),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1610: (10, 1),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1611: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1612: ];
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1613: for (under, before, ex, next, (num, den)) in cases {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1614: let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1615: let (eve, post, later) = (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1616: day_of(under, before),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1617: day_of(under, ex),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1618: day_of(under, next),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1619: );
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1623: let mut latest: std::collections::BTreeMap<String, (ContractDay, Vec<(u32, i64)>)> =
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1624: std::collections::BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1625: let mut on = Day::from_days(before.days_from_epoch() - 100).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1626: while on <= before {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1627: for (c, rows) in day_of(under, on) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1628: if c.expiry >= ex {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1629: latest.insert(c.contract().unwrap().to_string(), (c, rows));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1630: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1632: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1634: let pre: Vec<(ContractDay, Vec<(u32, i64)>)> = latest.into_values().collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1635: let strip = |days: &[(ContractDay, Vec<(u32, i64)>)]| -> Vec<ContractDay> {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1636: days.iter().map(|(c, _)| *c).collect()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1637: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1638: let eve_only = detect(symbol, ex, &strip(&eve), &strip(&post)).expect(under);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1639: let event = detect(symbol, ex, &strip(&pre), &strip(&post)).expect(under);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1640: assert!(event.lineage.len() >= eve_only.lineage.len(), "{under}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1641: let count = |carry: OiCarry| event.lineage.iter().filter(|l| l.oi == carry).count();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1642: let reused = event.lineage.iter().filter(|l| l.reused).count();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1644: let mut stale_rows = 0_usize;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1645: let mut records: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1646: for pair in event.lineage.iter().filter(|l| l.oi == OiCarry::Unadjusted) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1647: let new = post
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1648: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1649: .find(|(c, _)| c.contract() == Some(pair.new))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1650: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1651: let stale = pair.stale_oi(&new.1);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1652: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1653: stale.rows > 0 && stale.until_sod.is_some(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1656: stale_rows += stale.rows;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1657: records.push(pair.record(&new.1).line());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1661: for line in &records {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1662: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1663: crate::gdfl_lineage::LineageRecord::parse(line).is_some(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1669: let odd: Vec<i64> = event
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1670: .lineage
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1671: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1672: .filter(|l| l.oi == OiCarry::Unexplained)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1673: .map(|pair| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1674: let old = pre
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1675: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1676: .find(|(c, _)| c.contract() == Some(pair.old))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1677: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1678: let new = post
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1679: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1680: .find(|(c, _)| c.contract() == Some(pair.new))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1681: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1682: new.0.first_oi.unwrap() * 1_000 / old.0.close_oi.unwrap()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1683: })
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1684: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1685: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1689: pre.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1690: eve.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1691: eve_only.lineage.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1692: post.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1693: event.lineage.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1694: event.unpaired.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1695: event.stitched.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1696: count(OiCarry::Scaled),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1697: count(OiCarry::Unadjusted),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1698: count(OiCarry::Indistinct),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1699: count(OiCarry::Unexplained),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1700: count(OiCarry::Missing),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1702: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1703: event.lot_after * den,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1704: event.lot_before * num,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1707: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1708: eve_only.lineage.len() * 2 > eve.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1712: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1713: detect(symbol, next, &strip(&post), &strip(&later)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1719: let retired = event.retired();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1720: let last = pre.iter().map(|(c, _)| c.expiry).max().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1721: let mut again: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1722: let mut on = ex;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1723: while on <= last {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1725: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1726: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1727: let decoded = listing.ticker_of(file).map(|t| decode_ticker(t, on));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1728: if let Some(Ok(ticker)) = decoded
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1729: && ticker.underlying.as_str() == under
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1730: && retired.contains(&ticker.contract)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1731: {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1732: again.insert(ticker.contract.to_string());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1733: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1735: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1736: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1738: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1740: retired.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1741: again.len()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1743: assert!(again.len() >= event.stitched.len(), "{under}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1745: match under {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1746: "NTPC" => assert_eq!(again.len(), 35, "NTPC's stitched series"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1747: "BRITANNIA" => assert_eq!(retired.len(), 56, "BRITANNIA's series ending early"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1748: "BAJFINANCE" => assert_eq!(count(OiCarry::Unadjusted), 18, "stale ex-date OI"),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1749: _ => {}
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1752: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1760: fn the_decoders_costs_on_real_days_are_measured() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1761: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1762: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1763: let pct = |mut ns: Vec<u128>, q: usize| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1764: ns.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1765: ns[(ns.len() * q / 100).min(ns.len() - 1)]
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1766: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1767: for on in [day(2023, 6, 27), day(2026, 8, 3)] {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1768: let listing = zips.day(on).unwrap().expect("a vendor day");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1769: let names: Vec<&str> = listing
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1770: .entries()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1771: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1772: .filter_map(|f| listing.ticker_of(f))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1773: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1774: let mut decode_ns = Vec::with_capacity(names.len());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1775: let mut decoded = Vec::with_capacity(names.len());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1776: for name in &names {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1777: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1778: let got = decode_ticker(name, on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1779: decode_ns.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1780: decoded.extend(got.ok().map(|t| (t, *name)));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1782: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1783: let first = decoded
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1784: .first()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1785: .map(|(t, _)| listing.strike_median(t.underlying.as_str()));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1786: let medians_ns = at.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1787: let mut check_ns = Vec::with_capacity(decoded.len());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1788: for (ticker, name) in &decoded {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1789: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1790: let _ = listing.check_strike(ticker, name);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1791: check_ns.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1792: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1793: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1795: names.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1796: decoded.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1797: pct(decode_ns.clone(), 50),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1798: pct(decode_ns, 99),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1799: medians_ns / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1800: pct(check_ns.clone(), 50),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1801: pct(check_ns, 99),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1803: assert!(first.flatten().is_some(), "{on}: a median");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1804: assert_eq!(decoded.len(), names.len(), "{on}: every real name decodes");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:1806: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2188: fn the_real_unchanged_lot_events_are_detected_and_ordinary_days_are_not() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2190: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2191: let only = std::env::var("BRUTEX_WP12E_ONLY").ok();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2192: let lookback: u32 = std::env::var("BRUTEX_WP12E_LOOKBACK")
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2193: .ok()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2194: .and_then(|text| text.parse().ok())
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2195: .unwrap_or(30);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2196: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2197: let day_of = |under: &str, on: Day| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2198: let mut out: Vec<ContractDay> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2199: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2200: return out;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2202: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2203: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2204: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2206: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2207: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2209: if ticker.underlying.as_str() != under {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2210: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2211: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2212: let read = read_file(&zips, &listing, file).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2213: out.push(ContractDay::from_rows(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2214: &read.ticker,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2215: read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2218: out
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2219: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2222: let offset = |shift| Some(EventKind::StrikeOffset { shift });
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2223: let cases = [
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2224: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2225: "OIL",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2226: day(2020, 2, 19),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2227: day(2020, 2, 20),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2228: day(2020, 2, 24),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2229: offset(900),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2230: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2231: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2232: "COALINDIA",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2233: day(2020, 3, 18),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2234: day(2020, 3, 19),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2235: day(2020, 3, 20),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2236: offset(1_200),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2237: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2238: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2239: "VEDL",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2240: day(2022, 7, 25),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2241: day(2022, 7, 26),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2242: day(2022, 7, 27),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2243: offset(1_950),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2244: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2245: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2246: "HEROMOTOCO",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2247: day(2023, 2, 16),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2248: day(2023, 2, 17),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2249: day(2023, 2, 20),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2250: offset(6_500),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2251: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2252: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2253: "VEDL",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2254: day(2024, 9, 9),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2255: day(2024, 9, 10),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2256: day(2024, 9, 11),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2257: offset(2_000),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2258: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2259: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2260: "TATACHEM",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2261: day(2020, 3, 3),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2262: day(2020, 3, 4),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2263: day(2020, 3, 5),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2264: Some(EventKind::Relisted),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2265: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2266: // Ordinary days: an index across a weekly expiry, and the last full
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2267: // options day the store holds.
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2268: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2269: "NIFTY",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2270: day(2020, 9, 10),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2271: day(2020, 9, 11),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2272: day(2020, 9, 14),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2273: None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2274: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2275: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2276: "BANKNIFTY",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2277: day(2020, 9, 14),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2278: day(2020, 9, 15),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2279: day(2020, 9, 16),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2280: None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2281: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2282: (
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2283: "RELIANCE",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2284: day(2020, 9, 14),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2285: day(2020, 9, 15),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2286: day(2020, 9, 16),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2287: None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2288: ),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2289: ];
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2290: for (under, before, ex, next, want) in cases {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2291: if only.as_deref().is_some_and(|name| name != under) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2292: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2293: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2294: let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2296: let back = if want.is_none() && under != "RELIANCE" {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2297: 0
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2299: lookback
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2301: let mut latest: std::collections::BTreeMap<String, ContractDay> =
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2302: std::collections::BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2303: let mut on = Day::from_days(before.days_from_epoch() - back).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2304: while on <= before {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2305: for c in day_of(under, on) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2306: if c.expiry >= ex {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2307: latest.insert(c.contract().unwrap().to_string(), c);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2308: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2310: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2312: let pre: Vec<ContractDay> = latest.into_values().collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2313: let (post, later) = (day_of(under, ex), day_of(under, next));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2314: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2315: let got = detect(symbol, ex, &pre, &post);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2316: let took = at.elapsed();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2317: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2319: pre.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2320: post.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2321: took.as_micros(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2322: got.as_ref().map_or_else(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2323: || "none".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2324: |a| format!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2329: a.lineage.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2330: a.unpaired.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2331: a.stitched.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2332: a.lineage.iter().filter(|l| l.reused).count()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2336: assert_eq!(got.as_ref().map(|a| a.kind), want, "{under} {ex}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2337: if let Some(action) = &got {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2338: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2342: for pair in &action.lineage {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2343: let line = pair.record(&[]).line();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2344: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2345: crate::gdfl_lineage::LineageRecord::parse(&line).is_some(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2349: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2351: let control = detect(symbol, next, &post, &later);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2352: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2353: control.map(|a| a.kind),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2358: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2369: fn the_real_window_sweep_lists_every_event_the_detector_declares() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2372: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2373: let bound = |var: &str| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2374: let text = std::env::var(var).expect(var);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2375: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2376: day(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2377: parts[0],
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2378: u8::try_from(parts[1]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2379: u8::try_from(parts[2]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2381: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2382: let (from, to) = (bound("BRUTEX_WP12E_FROM"), bound("BRUTEX_WP12E_TO"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2383: let indices = std::env::var_os("BRUTEX_WP12E_INDICES").is_some();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2384: let zips = NfoZips::new(Path::new(&dir));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2385: let mut state: HashMap<String, HashMap<(Day, i64, OptionSide), ContractDay>> = HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2386: let (mut days, mut underlying_days, mut unread) = (0_u32, 0_u32, 0_u32);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2387: let mut events: Vec<String> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2388: let mut nanos: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2389: let mut on = from;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2390: while on <= to {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2391: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2392: days += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2393: let mut by: BTreeMap<String, Vec<ContractDay>> = BTreeMap::new();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2394: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2395: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2396: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2398: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2399: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2401: let under = ticker.underlying.as_str();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2402: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2403: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2404: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2405: let Ok(read) = read_file(&zips, &listing, file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2406: unread += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2407: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2409: by.entry(under.to_owned())
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2410: .or_default()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2411: .push(ContractDay::from_rows(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2412: &read.ticker,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2413: read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2416: for (under, todays) in by {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2417: let latest = state.entry(under.clone()).or_default();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2418: latest.retain(|key, _| key.0 >= on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2419: let before: Vec<ContractDay> = latest.values().copied().collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2420: let symbol = brutex_core::symbol::Symbol::new(&under).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2421: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2422: let got = detect(symbol, on, &before, &todays);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2423: nanos.push(at.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2424: underlying_days += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2425: if let Some(action) = got {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2426: events.push(format!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2427: "{on} {under} {:?} lot {}->{} pairs {} unpaired {} stitched {} (pre-ex keys {}, day keys {})",
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2428: action.kind,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2429: action.lot_before,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2430: action.lot_after,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2431: action.lineage.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2432: action.unpaired.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2433: action.stitched.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2434: before.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2435: todays.len()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2436: ));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2437: latest.clear();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2438: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2439: for c in todays {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2440: latest.insert((c.expiry, c.strike, c.side), c);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2441: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2443: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2444: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2446: nanos.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2447: let at = |q: usize| {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2448: nanos
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2449: .get(nanos.len().saturating_sub(1) * q / 100)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2450: .copied()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2451: .unwrap_or(0)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2452: };
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2453: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2455: events.len()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2457: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2459: nanos.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2460: at(50),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2461: at(99),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2462: at(100)
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2464: for event in &events {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2465: eprintln!("  {event}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2466: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2467: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2468: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2589: _ => None,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2607: (Err(_), Ok(_)) => false,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2805: fn the_fixed_array_decodes_real_days_as_the_field_vector_did() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2806: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2807: let store = NfoTickStore::new(Path::new(&root));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2808: for on in [day(2018, 9, 3), day(2023, 11, 12), day(2026, 8, 3)] {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2809: let listing = store.day(on).unwrap().expect("a tick-store day");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2810: let (mut files, mut rows, mut refused) = (0_u64, 0_u64, 0_u64);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2811: let (mut fixed_ns, mut vector_ns) = (0_u128, 0_u128);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2812: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2813: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2814: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2816: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2817: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2819: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2820: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2821: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2822: let bytes = store.fetch(file).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2823: crate::gdfl_cm::verify(file, &bytes).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2824: let stem = format!("{name}{ROW_SUFFIX}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2825: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2826: let fixed = decode(&bytes, &stem, on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2827: fixed_ns += at.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2828: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2829: let vector = decode_by_field_vector(&bytes, &stem, on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2830: vector_ns += at.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2831: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2832: definition_5_differs_only_where_4_refused(&fixed, &vector),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2835: files += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2836: match fixed {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2837: Ok(read) => rows += u64::try_from(read.rows.len()).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2838: Err(_) => refused += 1,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2841: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2844: fixed_ns / 1_000_000,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2845: vector_ns / 1_000_000
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2847: assert!(files > 0, "{on}: the day holds the three underlyings");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:2849: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3514: fn the_measured_bad_option_files_are_read_row_by_row() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3515: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3516: let store = NfoTickStore::new(Path::new(&root));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3517: let named = [
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3518: (day(2018, 11, 9), Some("AXISBANK18NOV580PE")),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3519: (day(2018, 11, 6), Some("BANKNIFTY15NOV1826700CE")),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3520: (day(2026, 4, 16), None),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3521: (day(2025, 3, 11), Some("BANKNIFTY26JUN2546500CE")),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3522: ];
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3523: for (on, only) in named {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3524: let listing = store.day(on).unwrap().expect("a tick-store day");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3525: let mut recovered = 0_u32;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3526: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3527: let Some(name) = listing.ticker_of(file) else {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3528: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3530: if only.is_some_and(|wanted| wanted != name) {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3531: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3532: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3533: let bytes = store.fetch(file).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3534: crate::gdfl_cm::verify(file, &bytes).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3535: let stem = format!("{name}{ROW_SUFFIX}");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3536: let old = decode_by_field_vector(&bytes, &stem, on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3537: if old.is_ok() && only.is_none() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3538: continue;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3539: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3540: let new = decode(&bytes, &stem, on);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3541: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3542: definition_5_differs_only_where_4_refused(&new, &old),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3545: let got = new.unwrap_or_else(|why| panic!("{on} {name}: {why}"));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3546: let lines: Vec<String> = got
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3547: .refused
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3548: .named
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3549: .iter()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3550: .map(|row| format!("{}:{}", row.line, row.fault.code()))
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3551: .collect();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3552: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3554: old.as_ref().map_or_else(ToString::to_string, |f| format!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3556: f.rows.len()
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3558: got.rows.len(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3559: got.refused.total(),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3560: lines.join(" "),
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3563: recovered += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3565: assert!(recovered > 0, "{on}: no file of the measured shape found");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3567: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3622: fn no_real_option_file_of_2018_ends_in_a_bare_cr() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3623: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3624: let store = NfoTickStore::new(Path::new(&root));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3625: let (mut on, last) = (day(2018, 9, 3), day(2018, 12, 31));
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3626: let (mut days, mut files, mut lf, mut bare_cr, mut no_newline) =
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3627: (0_u32, 0_u64, 0_u64, 0_u64, 0_u64);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3628: while on <= last {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3629: if let Some(listing) = store.day(on).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3630: days += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3631: let mut newline_less = 0_u64;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3632: for file in listing.entries() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3633: let bytes = store.fetch(file).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3634: crate::gdfl_cm::verify(file, &bytes).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3635: files += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3636: match bytes.last() {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3637: Some(b'\n') => lf += 1,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3638: Some(b'\r') => {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3639: bare_cr += 1;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3640: eprintln!("{on}: {} ends in a bare CR", file.entry);
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3641: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3642: _ => newline_less += 1,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3645: if newline_less > 0 {
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3646: eprintln!("{on}: {newline_less} option files end with no newline");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3647: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3648: no_newline += newline_less;
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3649: }
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3650: on = on.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3652: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3656: assert!(
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3657: days > 0 && files > 0,
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3660: assert_eq!(bare_cr, 0, "a real file ends in a bare CR");
UNCOVERED LINE crates/pull/src/gdfl_nfo_tests.rs:3661: }
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:352:13 False: let NfoRefusal::StrikeImplausible {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1274:11 True: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1274:11 False: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1275:16 True: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1275:16 False: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1280:21 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1280:21 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1286:28 True: if ticker.expiry != ticker.stated {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1286:28 False: if ticker.expiry != ticker.stated {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1290:28 True: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1290:28 False: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1299:32 True: if let Some(first) = seen.insert(key, name.to_owned()) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1299:32 False: if let Some(first) = seen.insert(key, name.to_owned()) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1302:28 True: if listing.check_strike(&ticker, name).is_err() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1302:28 False: if listing.check_strike(&ticker, name).is_err() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1321:20 True: if lo > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1321:20 False: if lo > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1359:8 True: if (from..=to).contains(&day(2019, 5, 30)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1359:8 False: if (from..=to).contains(&day(2019, 5, 30)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1365:8 True: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1365:8 False: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1415:11 True: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1415:11 False: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1416:16 True: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1416:16 False: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1418:21 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1418:21 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1422:35 True: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1422:35 False: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1423:32 True: if let Err(why) = listing.check_strike(&ticker, name) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1423:32 False: if let Err(why) = listing.check_strike(&ticker, name) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1431:28 True: if ticker.expiry != ticker.stated {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1431:28 False: if ticker.expiry != ticker.stated {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1458:28 True: if wanted.iter().any(|u| name.starts_with(u)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1458:28 False: if wanted.iter().any(|u| name.starts_with(u)) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1500:17 True: mine.contains(settled) && !held.contains(settled),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1500:17 False: mine.contains(settled) && !held.contains(settled),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1500:43 True: mine.contains(settled) && !held.contains(settled),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1500:43 False: mine.contains(settled) && !held.contains(settled),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1506:21 True: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1506:21 False: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1506:43 True: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1506:43 False: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1512:17 True: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1512:17 False: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1540:13 True: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1540:13 False: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1544:17 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1544:17 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1547:17 True: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1547:17 False: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1550:16 True: if ticker.underlying.as_str() != under {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1550:16 False: if ticker.underlying.as_str() != under {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1626:15 True: while on <= before {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1626:15 False: while on <= before {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1628:20 True: if c.expiry >= ex {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1628:20 False: if c.expiry >= ex {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1653:17 True: stale.rows > 0 && stale.until_sod.is_some(),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1653:17 False: stale.rows > 0 && stale.until_sod.is_some(),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1653:35 True: stale.rows > 0 && stale.until_sod.is_some(),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1653:35 False: stale.rows > 0 && stale.until_sod.is_some(),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1723:15 True: while on <= last {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1723:15 False: while on <= last {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1725:20 True: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1725:20 False: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1728:28 True: if let Some(Ok(ticker)) = decoded
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1728:28 False: if let Some(Ok(ticker)) = decoded
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1729:28 True: && ticker.underlying.as_str() == under
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1729:28 False: && ticker.underlying.as_str() == under
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1730:28 True: && retired.contains(&ticker.contract)
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1730:28 False: && retired.contains(&ticker.contract)
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1909:9 False: words.contains("2026.zip:") && words.contains("2025.zip:"),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:1909:40 False: words.contains("2026.zip:") && words.contains("2025.zip:"),
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2199:13 True: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2199:13 False: let Some(listing) = zips.day(on).unwrap() else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2203:17 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2203:17 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2206:17 True: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2206:17 False: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2209:16 True: if ticker.underlying.as_str() != under {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2209:16 False: if ticker.underlying.as_str() != under {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2291:12 True: if only.as_deref().is_some_and(|name| name != under) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2291:12 False: if only.as_deref().is_some_and(|name| name != under) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2296:23 True: let back = if want.is_none() && under != "RELIANCE" {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2296:23 False: let back = if want.is_none() && under != "RELIANCE" {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2296:41 True: let back = if want.is_none() && under != "RELIANCE" {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2296:41 False: let back = if want.is_none() && under != "RELIANCE" {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2304:15 True: while on <= before {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2304:15 False: while on <= before {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2306:20 True: if c.expiry >= ex {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2306:20 False: if c.expiry >= ex {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2337:16 True: if let Some(action) = &got {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2337:16 False: if let Some(action) = &got {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2390:11 True: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2390:11 False: while on <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2391:16 True: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2391:16 False: if let Some(listing) = zips.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2395:21 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2395:21 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2398:21 True: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2398:21 False: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2402:20 True: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2402:20 False: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2402:32 True: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2402:32 False: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2405:21 True: let Ok(read) = read_file(&zips, &listing, file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2405:21 False: let Ok(read) = read_file(&zips, &listing, file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2425:24 True: if let Some(action) = got {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2425:24 False: if let Some(action) = got {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2467:13 True: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2467:13 False: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2467:25 True: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2467:25 False: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2593:13 False: new.rows == old.rows
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2594:20 False: && new.quoted_rows == old.quoted_rows
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2595:20 False: && new.refused.total() == 0
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2813:17 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2813:17 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2816:17 True: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2816:17 False: let Ok(ticker) = decode_ticker(name, on) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2819:16 True: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:2819:16 False: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3527:17 True: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3527:17 False: let Some(name) = listing.ticker_of(file) else {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3530:16 True: if only.is_some_and(|wanted| wanted != name) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3530:16 False: if only.is_some_and(|wanted| wanted != name) {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3537:16 True: if old.is_ok() && only.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3537:16 False: if old.is_ok() && only.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3537:31 True: if old.is_ok() && only.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3537:31 False: if old.is_ok() && only.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3628:11 True: while on <= last {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3628:11 False: while on <= last {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3629:16 True: if let Some(listing) = store.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3629:16 False: if let Some(listing) = store.day(on).unwrap() {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3645:16 True: if newline_less > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3645:16 False: if newline_less > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3657:9 True: days > 0 && files > 0,
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3657:9 False: days > 0 && files > 0,
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3657:21 True: days > 0 && files > 0,
UNCOVERED BRANCH crates/pull/src/gdfl_nfo_tests.rs:3657:21 False: days > 0 && files > 0,
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:260: return None;
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:333: continue;
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:810: want.totals.files_refused += 1;
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:811: late_failures.push((format!("{on} {name}"), "the LTP is not a two-place"));
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:812: continue;
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1347: ..DayRead::default()
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1348: })
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1643: matches!(
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1653: assert!(
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1654: t.contract.as_str().starts_with("2012-"),
UNCOVERED LINE crates/pull/src/gdfl_r2_attack_tests.rs:1656: t.contract.as_str()
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:241:12 False: if !every_row && r.ltq == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:259:12 False: if !every_row && i64::try_from(r.ltq).is_err() {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:259:26 True: if !every_row && i64::try_from(r.ltq).is_err() {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:262:25 True: let volume = if every_row { 0 } else { u128::from(r.ltq) };
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:311:20 False: if let Some(r) = last {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:332:13 False: let Ok(entries) = std::fs::read_dir(&at) else {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:809:33 True: if *bent && rows.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1060:9 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1060:33 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1060:51 False: bars_checked > 1_000 && refusals > 500 && filtered > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1582:34 False: .any(|r| &r.instrument == name && r.why.contains("same contract")),
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1603:9 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1603:25 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1603:44 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1603:61 False: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1651:31 True: } else if let Ok(t) = got {
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1699:13 False: assert!(weekday(cutover) && weekday(weeklies));
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1699:33 False: assert!(weekday(cutover) && weekday(weeklies));
UNCOVERED BRANCH crates/pull/src/gdfl_r2_attack_tests.rs:1863:13 False: !text.contains("2023-06-29") && !text.contains("2023-08-15")
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:442: String::from_utf8_lossy(tail)
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:451: String::from_utf8_lossy(tail)
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:476: String::from_utf8_lossy(tail)
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:772: block.len(),
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:773: got.as_ref().map(Vec::len)
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:788: block.len(),
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:789: got.as_ref().map(Vec::len)
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:820: got.map(|b| b.len())
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:903: unreachable!("only swept index names are drawn")
UNCOVERED LINE crates/pull/src/gdfl_r3_attack_tests.rs:1420: all.files + all.files_refused
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:377:34 False: name == want_name || name.starts_with(&format!("{want_name}.")),
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:589:9 False: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:589:27 False: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:589:49 False: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:818:29 False: got.is_err() || rebuilt_whole,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1219:9 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1219:25 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1219:43 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1228:9 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1228:25 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED BRANCH crates/pull/src/gdfl_r3_attack_tests.rs:1228:43 False: bars > 1_000 && refusals > 300 && filtered > 150,
UNCOVERED LINE crates/pull/src/gdfl_r4_attack_tests.rs:130: journal_text(&root)
UNCOVERED LINE crates/pull/src/gdfl_r4_attack_tests.rs:205: journal_text(&root)
UNCOVERED LINE crates/pull/src/gdfl_r4_attack_tests.rs:678: journal_text(&root)
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:282:13 False: last.starts_with(&format!(
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:284:19 False: )) && last.contains(" late=1 late_unresolved=0 max_back_s=7 ")
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:285:20 False: && last.contains(" stage=file reason=bars_differ_held"),
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:491:13 False: assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:491:33 False: assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:799:20 False: || appended.starts_with("done stocks 2024-04-")
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:800:24 False: && appended.contains(&format!(" definition={BAR_DEFINITION} ")),
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:818:13 False: assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:818:54 False: assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:819:13 False: assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_r4_attack_tests.rs:819:62 False: assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:334: of.first().map_or(usize::MAX, |(at_line, _)| *at_line)
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:335: };
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:401: || {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:402: read.difference(&now)
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:403: .min()
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:404: .map_or_else(String::new, |gone| {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:405: format!("; the plan read {gone:?}, which the ledger no longer holds")
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:406: })
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:407: },
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:714: Err(why) => Err(format!("{} could not be read: {why}", path.display())),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:822: Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:823: Err(why) => return Err(format!("{} could not be listed: {why}", dir.display())),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:840: return None;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:857: return Ok(out);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:863: return Ok(out);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:883: Err(_) => return Ok(out),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:913: found.stray(path);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:914: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:938: found.other_rungs.push(format!("{} {rung}", key.key()));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:939: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:958: {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:959: found.stray(path);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:964: found.stray(path);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:965: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:976: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:977: found.note_unreadable(path, why);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:978: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:983: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:998: Err(why) => found.note_unreadable(path, why.to_string()),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1169: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1170: refused.push(Refused {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1171: day,
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1172: why: RetiredWhy::ExpiryUnsettled,
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1173: median: None,
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1174: text: format!("{day}: expiry unsettled: {why}"),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1175: });
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1176: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1194: Some((seen, _)) => {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1195: return Verdict::Unresolved(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1196: "the decoder settles its days on two expiries, {seen} and {}; \
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1197: refused rather than split",
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1198: this.0
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1199: ));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1233: None => Verdict::Unresolved(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1234: "the settled expiry {expiry} does not render as a store contract segment"
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1235: )),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1337: out.unresolved.push(Unresolved {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1338: key: key.clone(),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1339: why: format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1340: "holds a rung other than {}, which the re-key does not move",
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1341: RUNG.as_str()
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1342: ),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1343: });
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1344: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1438: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1564: (Verdict::Keep, Some(done), _) => left(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1565: "the decoder now files it where it is, and ledger line {line} supersedes it by {}",
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1566: option_key(key.underlying, done.new)
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1567: )),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1612: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1717: for &at in sources {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1718: let _ = refused.insert(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1719: at,
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1720: format!("its decoded key {target} is itself refused or unresolved"),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1721: );
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1722: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1723: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1726: for &at in sources {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1727: let _ = refused.insert(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1728: at,
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1729: format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1730: "its decoded key {target} is withdrawn by ledger line {line}, so bars \
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1731: moved there would be served by no reader"
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1732: ),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1733: );
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1734: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1735: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1747: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1750: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1913: Err(why) => return Err(format!("{}: {why}", shown.display())),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1917: return Ok(Write::Already);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1920: return Ok(Write::Append);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:1943: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2037: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2040: return Err(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2041: "{target} {month}: two of the keys moving here start on one second"
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2042: ));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2071: return Err(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2072: "{target} {month}: the keys moving here hold interleaved or repeated \
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2073: seconds; refused rather than merged"
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2074: ));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2120: Appended::AlreadyPresent { .. } => {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2121: done.already = done.already.saturating_add(1);
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2122: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2213: Err(TryLockError::Error(why)) => {
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2214: return Err(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2215: "{} could not be locked: {why}; nothing was written",
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2216: lock_at.display()
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2217: ));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2230: format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2235: })?;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2245: format!("{why} (the moved months are written and counted; run again to finish)")
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2246: })?;
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2288: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2316: }
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2319: return Err(format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2320: "{} is not a regular file; nothing was written",
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2321: path.display()
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2322: ));
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2326: Err(why) => return Err(format!("{}: {why}; nothing was written", path.display())),
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2335: format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2337: path.display()
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2339: })
UNCOVERED LINE crates/pull/src/gdfl_rekey.rs:2387: Err(why) => format!(
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:253:59 False: if line.ends_with(" (torn)") || (torn_tail && at == count) {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:320:35 False: while let Some(next) = self.superseded.get(&on) {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:336:28 False: if earliest
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:396:12 False: if held && counted == fresh.lines.values().filter(|under| within(under)).count() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:529:12 True: if key.exchange != Exchange::Nse || key.segment != Segment::Fno {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:687:20 False: && held != record
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:713:24 False: Err(absent) if absent.kind() == ErrorKind::NotFound => Ok(Withdrawn::default()),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:803:12 False: if self.strays_named.len() < NAMED {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:810:12 False: if self.unreadable_named.len() < NAMED {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:822:24 True: Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:822:24 False: Err(absent) if absent.kind() == ErrorKind::NotFound => return Ok(BTreeMap::new()),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:839:8 True: if year.len() != 4 || month.len() != 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:839:27 True: if year.len() != 4 || month.len() != 2 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:856:8 True: if header.n_valid == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:859:9 False: let (Some(first), Some(last)) = (
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:909:13 False: let Some(underlying) = Symbol::new(&name)
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:937:20 True: if rung != RUNG.as_str() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:943:16 False: if !key.months.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:955:16 True: if !FileKind::ALL
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:963:13 False: let Some(month) = month_named(stem) else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:982:12 True: if header.n_valid == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1122:9 True: !self.moves.is_empty() || !self.retired.is_empty() || self.census.pending() > 0
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1122:35 True: !self.moves.is_empty() || !self.retired.is_empty() || self.census.pending() > 0
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1193:27 False: Some(seen) if seen == this => {}
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1230:39 False: Some((expiry, Some(kind))) if expiry != key.expiry => {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1328:16 False: if let Some(median) = lower_median(&mut strikes) {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1437:17 False: let Ok(month) = day.year_month() else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1447:24 True: if !all || month_of(target, month).is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1508:43 False: if done.bars == key.bars() && done.months == months {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1547:41 False: if done.bars == now.bars && done.days == now.days {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1607:17 False: let Some(stored) = out
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1639:20 False: && there.rows > 0
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1640:20 False: && first <= there.last_ts_micros
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1673:12 False: if at < out.moves.len() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1716:12 True: if blocked.contains(target.as_str()) {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1725:16 True: if let Some(line) = ledger.line_of(*underlying, *to) {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1746:17 False: let Some(one) = out.moves.get(at) else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1749:17 False: let Some(stored) = held.get(&(one.underlying, one.from)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1774:12 False: if at < out.moves.len() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1888:12 False: if rows > 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1916:9 False: let (Some(first), Some(last)) = (bars.first(), bars.last()) else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1919:8 True: if header.n_valid == 0 || first.ts_micros > header.last_ts_micros {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1919:31 True: if header.n_valid == 0 || first.ts_micros > header.last_ts_micros {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:1929:8 False: if fits {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2036:21 False: let Some(first) = bars.first().map(|bar| bar.ts_micros) else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2039:20 True: if runs.entry(month).or_default().insert(first, bars).is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2050:60 True: if read != one.bars || bars_now != one.bars || months_now != one.months {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2067:16 True: if merged
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2069:54 True: .any(|pair| matches!(pair, [a, b] if a.ts_micros >= b.ts_micros))
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2286:12 False: if let Some(dir) = lineage.parent() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2313:12 False: if let Some(dir) = path.parent() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2318:21 True: Ok(meta) if !meta.file_type().is_file() => {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2325:24 False: Err(absent) if absent.kind() == ErrorKind::NotFound => {}
UNCOVERED BRANCH crates/pull/src/gdfl_rekey.rs:2467:9 False: key.exchange == Exchange::Nse
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:629: done.refused[0].why
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:814: refusal.refusal()
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:1284: decided.unresolved[0].why
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:1316: || panic!("{key}: {:?}", decided.unresolved),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:1760: panic!("{:?}", decided.census);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2078: fn synthetic(n: usize) -> (Survey, Withdrawn) {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2079: let under = sym("NIFTY");
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2080: let mut keys = Vec::with_capacity(n);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2081: let mut ledger = String::new();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2082: for at in 0..n {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2083: let stated = at % 10 == 9;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2084: let (expiry, day) = if stated {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2085: (d(2023, 6, 29), d(2020, 6, 15))
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2087: (d(2019, 6, 27), d(2019, 6, 3))
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2089: let strike = 1_000_000 + 5_000 * i64::try_from(at / 2).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2090: let side = if at % 2 == 0 { "PE" } else { "CE" };
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2091: let text = format!("{expiry}-{strike}-{side}");
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2092: let contract = c(&text);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2093: let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + 34_000) * 1_000_000;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2094: keys.push(Stored {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2095: underlying: under,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2096: contract,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2097: expiry,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2098: strike,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2099: side: if at % 2 == 0 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2100: OptionSide::Put
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2102: OptionSide::Call
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2104: months: vec![Month {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2105: month: day.year_month().unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2106: rows: 2,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2107: first_ts_micros: ts,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2108: last_ts_micros: ts + 1_000_000,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2109: }],
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2110: days: BTreeMap::from([(day, 2)]),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2112: if stated && at % 20 == 19 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2113: let to = c(&format!("2023-06-28-{strike}-{side}"));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2114: let record = SupersededRecord {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2115: underlying: under,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2116: old: contract,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2117: new: to,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2118: kind: AliasKind::VendorRedated,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2119: months: 1,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2120: bars: 2,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2121: digest: [9; 32],
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2122: };
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2123: ledger.push_str(&record.line());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2124: ledger.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2125: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2127: keys.sort_by(|a, b| a.contract.as_str().cmp(b.contract.as_str()));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2128: (
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2129: Survey {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2130: keys,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2131: ..Survey::default()
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2132: },
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2133: Withdrawn::parse(&ledger).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2134: )
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2135: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2142: fn plan_cost_per_key_is_flat_in_the_number_of_keys() {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2143: for n in [1_000_usize, 10_000] {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2144: let (found, ledger) = synthetic(n);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2145: let mut per_key: Vec<u128> = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2146: let mut judged = 0;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2147: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2148: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2149: let decided = plan(&found, &ledger);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2150: per_key.push(started.elapsed().as_nanos() / n as u128);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2151: judged = decided.keys;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2152: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2153: assert_eq!(judged, n as u64);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2154: per_key.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2155: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2157: per_key[99], per_key[197], per_key[199]
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2160: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2170: fn census_step_cost_is_one_load_per_plan_flat_per_entry() {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2171: for n in [10_000_usize, 100_000] {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2172: let scratch = Scratch::new(&format!("census-cost-{n}"));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2173: let root = scratch.root();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2174: let under = sym("NIFTY");
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2175: let month = YearMonth::new(2020, 6).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2176: let mut census =
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2177: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, u64::try_from(n).unwrap())
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2178: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2179: let mut ledger = String::new();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2180: for at in 0..n {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2181: let key = c(&format!("2023-06-28-{}-PE", 100_000 + at * 5));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2182: let ts = 1_592_193_400_000_000 + i64::try_from(at).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2183: census
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2184: .record_held(Held::unknown(Entry {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2185: key: EntryKey {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2186: contract: Some(key),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2187: exchange: Exchange::Nse,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2188: segment: Segment::Fno,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2189: symbol: under,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2190: timeframe: RUNG,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2191: month,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2192: },
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2193: rows: 1,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2194: first_ts_micros: ts,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2195: last_ts_micros: ts,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2196: }))
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2197: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2198: if at < 13 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2199: ledger.push_str(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2200: &RetiredRecord {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2201: key,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2202: ..refusal_of("2019-05-30-1150000-CE")
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2203: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2204: .line(),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2205: );
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2206: ledger.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2207: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2209: let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2210: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2211: std::fs::write(&path, census.image()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2212: let ledger = Withdrawn::parse(&ledger).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2213: let mut per_plan: Vec<u128> = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2214: let mut listed = 0;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2215: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2216: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2217: let step = census_step(root, &ledger);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2218: per_plan.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2219: listed = step.pending();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2220: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2221: assert_eq!(listed, 13);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2222: per_plan.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2223: let per_entry = |ns: u128| ns / n as u128;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2224: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2227: per_plan[99] / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2228: per_plan[197] / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2229: per_plan[199] / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2230: per_entry(per_plan[99]),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2231: per_entry(per_plan[197]),
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2232: per_entry(per_plan[199])
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2235: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2422: assert!(!again.unresolved.is_empty(), "{tag}");
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2423: continue;
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2572: fn spread(mut samples: Vec<u128>) -> String {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2573: samples.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2574: format!(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2576: samples[99],
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2577: samples[197],
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2578: samples[199],
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2579: samples.len()
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2581: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2593: fn apply_guard_cost_per_operation() {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2594: let fixture = Fixture::new("guardcost");
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2595: fixture.counted();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2596: let root = fixture.root();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2597: let at = ledger_lock_path(root);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2598: let mut lock = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2599: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2600: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2601: let file = open_ledger_lock(&at).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2602: let held = store::flock::Flock::try_lock(&file, at.as_path()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2603: held.release().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2604: lock.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2605: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2606: let mut text = String::with_capacity(10_000 * 160);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2607: for strike in 0..10_000_i64 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2608: let old = format!("2023-06-29-{}-PE", 1_000_000 + strike * 500);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2609: let new = format!("2023-06-28-{}-PE", 1_000_000 + strike * 500);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2610: text.push_str(&pointer(&old, &new).line());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2611: text.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2612: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2613: let read = Withdrawn::parse(&text).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2614: let again = Withdrawn::parse(&text).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2615: let mut compare = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2616: let mut rules = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2617: let mut narrowed = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2618: let only: BTreeSet<Symbol> = [sym("NIFTY")].into_iter().collect();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2619: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2620: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2621: read.same_records(&again, None).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2622: compare.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2623: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2624: read.same_records(&again, Some(&only)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2625: narrowed.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2626: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2627: read.targets_continue().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2628: rules.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2629: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2630: let mut chain = String::with_capacity(10_000 * 160);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2631: for at in 0..10_000_i64 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2632: let from = format!("2023-06-29-{}-PE", 1_000_000 + at * 500);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2633: let to = format!("2023-06-29-{}-PE", 1_000_000 + (at + 1) * 500);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2634: chain.push_str(&pointer(&from, &to).line());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2635: chain.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2636: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2637: let linked = Withdrawn::parse(&chain).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2638: let mut walk = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2639: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2640: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2641: linked.targets_continue().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2642: walk.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2643: }
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2644: let mut source = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2645: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2646: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2647: let (months, bars) = source_now(root, sym("NIFTY"), c(STATED)).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2648: source.push(started.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2649: assert_eq!((months.len(), bars), (2, 3));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2651: eprintln!("APPLY_GUARD lock_open_take_release {}", spread(lock));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2652: eprintln!("APPLY_GUARD same_records_10000_lines {}", spread(compare));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2653: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2655: spread(narrowed)
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2657: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2659: spread(rules)
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2661: eprintln!("APPLY_GUARD targets_continue_10000_chain {}", spread(walk));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2662: eprintln!("APPLY_GUARD source_now_2_months {}", spread(source));
UNCOVERED LINE crates/pull/src/gdfl_rekey_tests.rs:2663: }
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:425:13 False: assert!(decided.retired.is_empty() && decided.moves.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:425:43 False: assert!(decided.retired.is_empty() && decided.moves.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:543:13 False: assert!(again.moves.is_empty() && again.retired.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:543:39 False: assert!(again.moves.is_empty() && again.retired.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1024:13 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1024:39 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1027:9 False: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1027:34 False: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1048:13 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1048:39 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1162:13 False: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1162:49 False: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1278:13 False: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1278:41 False: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1409:9 False: why.contains("and ledger line 1 retires it as")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1410:16 False: && why.contains("one retirement has one account"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1445:13 False: assert!(done.refused.is_empty() && done.moved.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1445:40 False: assert!(done.refused.is_empty() && done.moved.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1581:9 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1581:37 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1581:67 False: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1705:13 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1705:40 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1705:69 False: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1759:9 False: let CensusStep::Unreadable(why) = &decided.census else {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1867:24 False: .any(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1894:9 False: left.why.contains("(apply's own check: ")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1895:16 False: && left.why.contains("would precede or contradict"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1945:9 False: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1945:46 False: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1952:9 False: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1952:46 False: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1956:9 False: why.contains("0707070707070707") && why.contains("0808080808080808"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1956:45 False: why.contains("0707070707070707") && why.contains("0808080808080808"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1965:9 False: why.contains("line 1 retires") && why.contains("and line 2 as"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:1965:43 False: why.contains("line 1 retires") && why.contains("and line 2 as"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2084:32 True: let (expiry, day) = if stated {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2084:32 False: let (expiry, day) = if stated {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2090:23 True: let side = if at % 2 == 0 { "PE" } else { "CE" };
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2090:23 False: let side = if at % 2 == 0 { "PE" } else { "CE" };
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2099:22 True: side: if at % 2 == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2099:22 False: side: if at % 2 == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2112:12 True: if stated && at % 20 == 19 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2112:12 False: if stated && at % 20 == 19 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2112:22 True: if stated && at % 20 == 19 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2112:22 False: if stated && at % 20 == 19 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2198:16 True: if at < 13 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2198:16 False: if at < 13 {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2301:17 False: why.contains("the ledger changed since this plan; re-plan")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2302:24 False: && why.contains("nothing was written"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2339:9 False: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2339:46 False: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2377:17 False: refused.contains("nothing was written")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2379:28 False: || refused.contains("the ledger changed since this plan")),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2410:13 False: why.contains("the plan counted 3 bar(s)")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2411:20 False: && why.contains("bars landed under it after the survey; re-plan"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2419:12 True: if again.moves.is_empty() {
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2442:13 False: why.contains("retires NSE-FNO-NIFTY-2020-06-25-191250000-CE")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2443:20 False: && why.contains("bars move only into a key that continues"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2446:17 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2446:43 False: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2453:9 False: why.contains("lines 1, 2") || why.contains("lines 2, 1"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2453:39 True: why.contains("lines 1, 2") || why.contains("lines 2, 1"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2453:39 False: why.contains("lines 1, 2") || why.contains("lines 2, 1"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2564:9 False: left.why.contains("retires it with 1 bar(s) on 1 day(s)")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2565:16 False: && left.why.contains("holds 2 bar(s) on 1 day(s) now"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2697:9 False: refused.why.contains("the plan counted 1 bar(s) to retire")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2698:16 False: && refused.why.contains("it holds 2 bar(s) in 1 month(s) now")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2699:16 False: && refused.why.ends_with("re-plan"),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2711:9 False: census_before.contains(&row) && listed.contains(&row),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2711:41 False: census_before.contains(&row) && listed.contains(&row),
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2785:13 False: why.contains("the ledger changed since this plan; re-plan")
UNCOVERED BRANCH crates/pull/src/gdfl_rekey_tests.rs:2786:20 False: && why.contains("nothing was written"),
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1431: }
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1490: return Some(month(first));
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1499: return None;
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1903: why: format!("{}: {why}", taken.claim_name()),
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1904: })?;
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1949: format!(
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1951: kind.as_str()
UNCOVERED LINE crates/pull/src/gdfl_run.rs:1953: })
UNCOVERED LINE crates/pull/src/gdfl_run.rs:2795: Err(why) => self.failed(kept, &why),
UNCOVERED BRANCH crates/pull/src/gdfl_run.rs:1412:16 False: && held.source == source
UNCOVERED BRANCH crates/pull/src/gdfl_run.rs:1423:16 False: if let Some(entry) = self.held.get_mut(slot) {
UNCOVERED BRANCH crates/pull/src/gdfl_run.rs:1489:21 False: let Ok(next) = day.succ() else {
UNCOVERED BRANCH crates/pull/src/gdfl_run.rs:1498:13 False: let Ok(next) = end.succ() else {
UNCOVERED LINE crates/pull/src/gdfl_run_cov_tests.rs:421: other => panic!("{source:?}: {other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_run_cov_tests.rs:467: panic!("{end:?}");
UNCOVERED BRANCH crates/pull/src/gdfl_run_cov_tests.rs:466:9 False: let KindEnd::Refused(refusal) = &end else {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:353: assert!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:354: refused.to_string().contains("runs.claims")
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:355: && refused.to_string().contains("was not handed out"),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:358: assert_eq!(ids(), [0, first, first + 1], "recorded, never handed out");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:359: std::fs::set_permissions(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:360: at("runs.claims"),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:361: std::os::unix::fs::PermissionsExt::from_mode(0o644),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:363: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:364: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:365: allocate(&blocked, NOW).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:366: first + 2,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:369: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:422: panic!("{api:?}");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:545: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:582: panic!("{seen:?}");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:791: fn ia2_timings() {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:792: fn pct(mut v: Vec<Duration>) -> String {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:793: v.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:794: let n = v.len();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:795: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:797: v[n / 2],
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:798: v[n * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:799: v[n - 1]
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:801: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:802: let store = scratch("timings");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:803: std::fs::create_dir_all(store.join("imports")).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:804: let control = stop::StopControl::new(store.join("imports/gdfl.stop"), 1);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:805: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:806: for _ in 0..10_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:807: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:808: let _seen = control.requested();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:809: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:810: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:811: println!("C-GR-07 stop check, no stop file: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:812: let other = stop::StopRequest {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:813: run: 2,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:814: requested: NOW,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:815: by: "cli".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:816: };
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:817: stop::write_stop(&store.join("imports/gdfl.stop"), &other).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:818: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:819: for _ in 0..10_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:820: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:821: let _seen = control.requested();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:822: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:823: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:824: println!("C-GR-07 stop check, another run's stop file: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:825: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:826: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:827: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:828: allocate(&store, NOW).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:829: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:830: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:831: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:833: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:835: let path = store.join(ledger::LEDGER);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:836: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:837: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:838: let mut bytes = std::fs::read(&path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:839: bytes.extend_from_slice(&[b'x'; 77]);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:840: std::fs::write(&path, &bytes).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:841: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:842: allocate(&store, NOW).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:843: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:844: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:845: println!("C-GR-08 run id allocation over a torn tail: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:846: let lock = crate::gdfl_import::journal_lock_path(&crate::gdfl_import::journal_path(&store));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:847: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:848: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:849: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:850: drop(locks::take_path_within(&lock, locks::LOCK_WAIT).unwrap());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:851: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:852: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:853: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:855: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:857: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:858: for _ in 0..500 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:859: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:860: let claimed = claim_with(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:861: &store,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:862: template("options", "cli"),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:863: locks::PROBE_RETRY,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:864: None,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:865: NOW,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:866: )
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:867: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:868: claimed.release().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:869: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:870: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:871: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:873: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:875: let held = claim_with(&store, template("options", "cli"), 0, None, NOW).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:876: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:877: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:878: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:879: let seen = locks::lock_state(&store, 0);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:880: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:881: assert!(!seen.is_empty());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:883: println!("lock_state probe, one held lock: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:884: held.release().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:885: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1206: panic!("{busy:?}");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1288: fn ia2_repair_timings() {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1289: fn pct(mut v: Vec<Duration>) -> String {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1290: v.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1291: let n = v.len();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1292: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1294: v[n / 2],
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1295: v[n * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1296: v[n - 1]
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1298: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1299: let root = scratch("repair-timings");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1300: let v4 = v4(&root);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1301: std::fs::create_dir_all(v4.join("imports")).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1302: let path = v4.join("imports/gdfl.options.stop");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1303: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1304: for at in 0..500_u64 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1305: let _fresh = std::fs::remove_file(&path);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1306: let request = stop::StopRequest {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1307: run: 9,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1308: requested: NOW + at,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1309: by: "cli".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1310: };
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1311: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1312: stop::write_stop(&path, &request).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1313: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1314: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1315: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1317: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1319: let again = stop::StopRequest {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1320: run: 9,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1321: requested: NOW,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1322: by: "api".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1323: };
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1324: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1325: for _ in 0..10_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1326: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1327: let _first = stop::write_stop(&path, &again).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1328: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1329: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1330: println!("stop asked again, file names the run: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1331: let _fresh = std::fs::remove_file(&path);
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1332: let held: Vec<_> = ["indices", "stocks", "options"]
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1333: .into_iter()
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1334: .map(|kind| claim_with(&v4, template(kind, "cli"), 0, None, NOW).unwrap())
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1335: .collect();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1336: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1337: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1338: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1339: let refused = RunLocks::take_store(&v4, 0).unwrap_err();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1340: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1341: assert!(matches!(refused, LockFault::Busy { .. }));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1343: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1345: pct(took)
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1347: let refused_sink = LogState::Refused("held".to_owned());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1348: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1349: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1350: let request = Request::new(
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1351: Kinds::of(&[ImportKind::Options]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1352: SourceWord::TickStore,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1353: (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 1).unwrap()),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1355: let process = Process {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1356: store_root: &v4,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1357: commit: Some("feedc0de"),
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1358: log: &refused_sink,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1359: unstamped: None,
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1360: };
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1361: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1362: let refused = plan(request, &process, &|_| None, &mut SourceCache::default()).unwrap_err();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1363: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1364: assert_eq!(refused.code(), "run_busy");
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1366: println!("plan refused before its sink, holder named: {}", pct(took));
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1367: for one in held {
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1368: one.release().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1369: }
UNCOVERED LINE crates/pull/src/gdfl_run_ia2_tests.rs:1370: }
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:127:13 True: assert!(!store.join(ledger::LEDGER).exists() || ledger::read_all(&store).unwrap().is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:127:53 False: assert!(!store.join(ledger::LEDGER).exists() || ledger::read_all(&store).unwrap().is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:275:17 False: assert!(field.starts_with("crc=") && field.len() == 12, "{field}");
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:275:46 False: assert!(field.starts_with("crc=") && field.len() == 12, "{field}");
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:338:9 False: refused.to_string().contains("runs.claims")
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:339:16 False: && refused.to_string().contains("not a regular file"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:354:9 True: refused.to_string().contains("runs.claims")
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:354:9 False: refused.to_string().contains("runs.claims")
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:355:16 True: && refused.to_string().contains("was not handed out"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:355:16 False: && refused.to_string().contains("was not handed out"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:399:66 False: matches!(&got, LockFault::Stalled { lock, waited_ms } if lock == "gdfl.journal.lock" && *waited_ms >= 60),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:399:97 False: matches!(&got, LockFault::Stalled { lock, waited_ms } if lock == "gdfl.journal.lock" && *waited_ms >= 60),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:421:9 False: let RunRefusal::RunBusy { lock, holder } = &api else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:426:9 False: holder.contains(&format!("run={} ", cli.run)) && holder.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:426:58 False: holder.contains(&format!("run={} ", cli.run)) && holder.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:540:13 False: assert!(!claimed.claim.host.contains(' ') && !claimed.claim.host.contains('='));
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:540:50 False: assert!(!claimed.claim.host.contains(' ') && !claimed.claim.host.contains('='));
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:550:96 False: seen.iter().any(|s| matches!(s, LockState::Held { claim: ClaimState::Claim(c), .. } if c.run == claimed.run)),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:574:9 False: let [
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:592:93 False: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("crc"))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:596:93 False: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("digest"))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:600:93 False: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("torn"))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:607:106 False: matches!(seen.as_slice(), [LockState::Interrupted { claim: ClaimState::Unreadable(why), .. }] if why.contains("\\x1b")),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:647:67 False: matches!(&busy, RunRefusal::RunBusy { holder, .. } if holder.contains(&format!("run={run} ")))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:906:9 False: busy.starts_with("run_busy: ")
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:907:16 False: && busy.contains(&format!("run={} ", run.run))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:908:16 False: && busy.ends_with("Nothing was written"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:929:9 False: claim.contains(" mode=census-recount kind=census ") && claim.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:929:64 False: claim.contains(" mode=census-recount kind=census ") && claim.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:963:9 False: claim.contains(" mode=dayflags kind=options ") && claim.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:963:59 False: claim.contains(" mode=dayflags kind=options ") && claim.contains(" by=cli "),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:1146:62 False: matches!(&unnamed, LockFault::Busy { holder, .. } if holder.is_empty()),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:1201:13 False: let RunRefusal::RunBusy {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:1210:13 False: named.contains(&format!("run={} ", holder.run))
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:1211:20 False: && named.contains(" pid=4242 ")
UNCOVERED BRANCH crates/pull/src/gdfl_run_ia2_tests.rs:1212:20 False: && named.contains(" by=cli "),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:186: return Err(format!(
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:187: "a record body of {} bytes is past {BODY}",
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:188: body.len()
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:189: ));
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:223: return None;
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:348: why: why.to_string(),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:349: };
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:357: Err(why) if why.kind() == ErrorKind::UnexpectedEof => Ok(None),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:358: Err(why) => Err(io(why)),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:446: why: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:450: })?,
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:526: return None;
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:564: return Ok(ClaimsTail::default());
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:621: RunsLedgerFault::Io {
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:622: why: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:623: "its claims line was not written ({why}); run {id}'s record is in the ledger \
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:624: and the id was not handed out"
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:625: ),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:626: }
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:627: })
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:725: let why = why.to_string();
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:726: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:727: &telemetry::Event::error(TARGET, "runs ledger alert not put")
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:728: .with("key", key.as_str())
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:729: .with("why", why.as_str()),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:730: );
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:765: let why = why.to_string();
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:766: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:767: &telemetry::Event::error(TARGET, "runs ledger alert not put")
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:768: .with("offset", offset)
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:769: .with("why", why.as_str()),
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:770: );
UNCOVERED LINE crates/pull/src/gdfl_run_ledger.rs:859: ""
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:185:12 True: if body.len() > BODY {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:202:16 False: if let Some(last) = out.last_mut() {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:222:13 False: let [id, mode, kind, by, pid, started] = fixed else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:357:29 True: Err(why) if why.kind() == ErrorKind::UnexpectedEof => Ok(None),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:357:29 False: Err(why) if why.kind() == ErrorKind::UnexpectedEof => Ok(None),
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:525:8 True: if body.len() > BODY {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:535:12 False: if let Some(last) = record.last_mut() {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:563:9 False: let Some(file) = open_regular(&path, true)? else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:724:12 True: if let Err(why) = crate::alerts::put(store, &alert) {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:764:12 True: if let Err(why) = crate::alerts::put(store, &alert) {
UNCOVERED BRANCH crates/pull/src/gdfl_run_ledger.rs:856:20 False: if no_link {
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:424: return Err(LockFault::Refused {
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:425: lock: name.to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:426: why: "the lock file was lost between two tries".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:427: });
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:508: return Err(LockFault::Refused {
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:509: lock: name.to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:510: why: "the lock file was lost between two tries".to_owned(),
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:511: });
UNCOVERED LINE crates/pull/src/gdfl_run_locks.rs:1168: }
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks.rs:423:13 False: let Some(one) = file.take() else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks.rs:507:13 False: let Some(one) = file.take() else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks.rs:1164:12 False: if let Some(dir) = path.parent() {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_cov_tests.rs:70:9 False: said.contains("was v3 when") && said.contains("is v4 now"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_cov_tests.rs:70:41 False: said.contains("was v3 when") && said.contains("is v4 now"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_cov_tests.rs:161:9 False: long.len() <= HOST_MAX && long.len().is_multiple_of(3),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_cov_tests.rs:161:35 False: long.len() <= HOST_MAX && long.len().is_multiple_of(3),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_cov_tests.rs:301:61 False: matches!(&state, LockState::Unprobed { why, .. } if why == "is a directory"),
UNCOVERED LINE crates/pull/src/gdfl_run_locks_ps1_tests.rs:67: panic!("a file at the census's name was located");
UNCOVERED LINE crates/pull/src/gdfl_run_locks_ps1_tests.rs:212: panic!("the root opened as a lock file");
UNCOVERED LINE crates/pull/src/gdfl_run_locks_ps1_tests.rs:229: panic!("a directory opened as a lock file");
UNCOVERED LINE crates/pull/src/gdfl_run_locks_ps1_tests.rs:283: panic!("the host took a flock it refuses");
UNCOVERED LINE crates/pull/src/gdfl_run_locks_ps1_tests.rs:353: matches!(
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:66:9 False: let Err(why) = locate(&store, Vendor::Gdfl) else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:211:9 False: let Err(LockFault::Refused { lock, why }) = open_lock_file(&store, "/") else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:228:9 False: let Err(LockFault::Refused { lock, why }) = take_path_within(&path, Duration::ZERO) else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:282:9 False: let Err(TryLockError::Error(host)) = unlockable(&path).try_lock() else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:9 False: output.status.success() && said.contains("1 passed"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:36 False: output.status.success() && said.contains("1 passed"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_locks_ps1_tests.rs:503:35 False: faults.iter().all(|fault| fault
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:88: names.insert(row.store_key.to_owned());
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:89: }
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:93: names.insert(share.to_string());
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:94: names.insert(brutex_core::alias::LINEAGE.current(share).to_owned());
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:95: }
UNCOVERED LINE crates/pull/src/gdfl_run_scope.rs:121: only.extend(stored_filter(kind, &scope));
UNCOVERED BRANCH crates/pull/src/gdfl_run_scope.rs:120:8 True: if stored_only {
UNCOVERED LINE crates/pull/src/gdfl_run_start.rs:145: return Gate::Release;
UNCOVERED BRANCH crates/pull/src/gdfl_run_start.rs:144:12 True: if spawned.is_err() {
UNCOVERED LINE crates/pull/src/gdfl_run_stop_cov_tests.rs:94: panic!("{refused:?}");
UNCOVERED BRANCH crates/pull/src/gdfl_run_stop_cov_tests.rs:93:9 False: let StopRefusal::ClaimBeingWritten { lock } = &refused else {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:385: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:624: panic!("{end:?}");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:678: panic!("{ends:?}");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:853: fn source_cache_hit_timing() {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:854: let root = scratch("cache.timing");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:855: let ids = [SourceId::CmZip, SourceId::NfoZips, SourceId::TickStore];
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:856: let mut dirs = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:857: for id in ids {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:858: let dir = root.join(id.as_str());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:859: std::fs::create_dir_all(&dir).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:860: std::fs::write(dir.join("FORMAT.md"), b"synthetic\n").unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:861: dirs.push(dir);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:862: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:863: let mut cache = SourceCache::default();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:864: for (id, dir) in ids.iter().zip(&dirs) {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:865: let _first = cache.origin(*id, dir, Some("abc"));
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:866: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:867: let mut took = Vec::with_capacity(6_000);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:868: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:869: for (id, dir) in ids.iter().zip(&dirs) {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:870: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:871: let _hit = cache.origin(*id, dir, Some("abc"));
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:872: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:873: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:875: assert_eq!(cache.measures(), 3, "every call after the first three hit");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:876: took.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:877: let n = took.len();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:878: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:880: took[n / 2],
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:881: took[n * 99 / 100],
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:882: took[n - 1]
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:884: std::fs::remove_dir_all(root).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:885: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1464: panic!("a prefix of a journal is a journal: cut at {cut_at}");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1494: fn journal_load_and_refresh_costs() {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1495: let store = scratch("costs");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1496: let path = journal_path(&store);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1497: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1498: let names: Vec<String> = (0..60).map(|i| format!("SYMQ{i:04}")).collect();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1499: let long_filter = names.join(",");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1502: let runs: [(&str, usize); 5] = [
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1503: ("*", 9_180),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1504: ("*", 91_800),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1505: ("*", 918_000),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1506: ("long", 23_270),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1507: ("long", 232_700),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1508: ];
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1509: for (filter, lines) in runs {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1510: let line_of = |on: &str| {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1511: if filter == "*" {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1512: done(on)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1514: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1519: };
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1520: let mut text = format!("{RUN} definition={BAR_DEFINITION}\n");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1521: let mut on = day(1990, 1, 1);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1522: for _ in 0..lines {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1523: text.push_str(&line_of(&on.to_string()));
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1524: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1525: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1526: std::fs::write(&path, &text).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1527: let n = if text.len() > 32 << 20 { 20 } else { 200 };
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1528: let mut loads = Vec::with_capacity(n);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1529: for _ in 0..n {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1530: let at = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1531: let _reader = JournalReader::open(&store).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1532: loads.push(at.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1533: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1534: let mut reader = JournalReader::open(&store).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1535: let mut refreshes = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1536: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1539: let mut file = std::fs::OpenOptions::new()
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1540: .append(true)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1541: .open(&path)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1542: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1543: file.write_all(line_of(&on.to_string()).as_bytes()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1545: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1546: let at = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1547: assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1548: refreshes.push(at.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1550: let pct = |v: &mut Vec<std::time::Duration>| {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1551: v.sort();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1552: let at = |q: usize| v[(v.len() * q / 100).min(v.len() - 1)];
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1553: (at(50), at(99), v[v.len() - 1], v.len())
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1554: };
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1555: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1557: text.len(),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1558: pct(&mut loads),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1559: pct(&mut refreshes)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1562: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1563: std::fs::copy(sample, &path).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1564: let mut loads = Vec::with_capacity(50);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1565: let mut loaded = 0;
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1566: for _ in 0..50 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1567: let at = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1568: loaded = JournalReader::open(&store).unwrap().loaded();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1569: loads.push(at.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1570: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1571: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1573: spread(&mut loads)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1575: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1576: std::fs::remove_dir_all(store).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1577: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1580: fn spread(samples: &mut [std::time::Duration]) -> String {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1581: samples.sort();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1582: let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1583: format!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1585: at(50),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1586: at(99),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1587: samples[samples.len() - 1],
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1588: samples.len()
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1590: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1598: fn progress_and_failure_costs() {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1599: let root = scratch("write-costs");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1600: let store = root.join("store");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1601: std::fs::create_dir_all(&store).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1602: let log = installed();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1603: let planned = plan(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1604: request(&[ImportKind::Options]),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1605: &process(&store, &log),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1606: &env_with(vec![(TICKSTORE_VAR, root.clone())]),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1607: &mut SourceCache::default(),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1609: .unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1610: let progress = ProgressFile::new(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1611: store.join("imports").join("gdfl.options.progress"),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1612: &planned,
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1613: &planned.kinds[0],
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1616: let mut report = Report::default();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1617: let mut writes = Vec::with_capacity(1_000);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1618: for closed in 1..=1_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1619: report.days_imported = closed;
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1620: let at = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1621: progress.day_passed(ImportKind::Options, day(2024, 4, 1), &report);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1622: writes.push(at.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1623: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1624: assert_eq!(progress.writes(), 1_000);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1625: println!(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1627: spread(&mut writes)
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1629: let cause = Cause::open(Stage::Vendor, "malformed_row");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1630: let mut named = Report::default();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1631: let mut said = String::new();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1632: for total in [FAILURES_KEPT, 10 * FAILURES_KEPT, 100 * FAILURES_KEPT] {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1633: while named.failures_total < total - 1_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1634: named.name(cause, failure(named.failures_total));
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1635: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1636: let mut each = Vec::with_capacity(1_000);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1637: for _ in 0..1_000 {
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1638: let one = failure(named.failures_total);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1639: let at = Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1640: named.name(cause, one);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1641: each.push(at.elapsed());
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1642: }
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1643: assert!(named.failures.len() <= 2 * FAILURES_KEPT);
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1644: let _ = std::fmt::Write::write_fmt(
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1645: &mut said,
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1646: format_args!("at {total} named: {}; ", spread(&mut each)),
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1647: );
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1649: println!("C-GR-06 Report::name: {said}");
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1650: std::fs::remove_dir_all(root).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_run_tests.rs:1651: }
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:370:9 False: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:370:62 False: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:380:8 False: if telemetry::global().is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:623:13 False: let KindEnd::Report(report) = end else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:677:9 False: let KindEnd::Unopened(refused) = &ends[0].1 else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1378:44 False: Refreshed::Reloaded { now, .. } if now == head.len() as u64
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1463:13 False: let Ok(mut reader) = JournalReader::open(&store) else {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1511:16 True: if filter == "*" {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1511:16 False: if filter == "*" {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1527:20 True: let n = if text.len() > 32 << 20 { 20 } else { 200 };
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1527:20 False: let n = if text.len() > 32 << 20 { 20 } else { 200 };
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1562:12 True: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1562:12 False: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1633:15 True: while named.failures_total < total - 1_000 {
UNCOVERED BRANCH crates/pull/src/gdfl_run_tests.rs:1633:15 False: while named.failures_total < total - 1_000 {
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:217: path.display()
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:745: after.keys().collect::<Vec<_>>()
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:1275: closes[2]
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:1694: panic!("{d}: a close with no scope owes the members beyond the swept surface");
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:2231: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:2287: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:2450: journal_text(&store)
UNCOVERED LINE crates/pull/src/gdfl_scope_tests.rs:2551: journal_text(&store)
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:575:13 False: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:575:45 False: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:749:9 False: nifty.len() > before.len() && nifty.starts_with(&before),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:749:39 False: nifty.len() > before.len() && nifty.starts_with(&before),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:1273:9 False: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:1273:47 False: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:1693:13 False: let Owed::Probe(beyond) = owed else {
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:1708:13 False: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:1708:54 False: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2059:13 False: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2059:48 False: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2078:13 False: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2078:43 False: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2283:17 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2283:44 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
UNCOVERED BRANCH crates/pull/src/gdfl_scope_tests.rs:2283:65 False: names_restated(&report) && report.files == 0 && report.seconds == 0,
UNCOVERED LINE crates/pull/src/gdfl_sealed_tests.rs:154: }
UNCOVERED LINE crates/pull/src/gdfl_sealed_tests.rs:300: BTreeMap::new()
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:51:29 False: parts.len() == 7 || parts.len() == 8,
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:132:17 False: assert!(got.seconds > 0 && got.files > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:132:36 False: assert!(got.seconds > 0 && got.files > 0);
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:149:16 False: if sealed_months.contains(&path) {
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:214:28 True: .filter(|path| path.starts_with("bars") || path.starts_with("bar-revisions-v1"))
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:214:28 False: .filter(|path| path.starts_with("bars") || path.starts_with("bar-revisions-v1"))
UNCOVERED BRANCH crates/pull/src/gdfl_sealed_tests.rs:297:8 False: if dir.exists() {
UNCOVERED LINE crates/pull/src/gdfl_seconds_attack_tests.rs:344: (got, want) => panic!("case {case} {kind:?} {ticks:?}: {got:?} against {want:?}"),
UNCOVERED LINE crates/pull/src/gdfl_seconds_attack_tests.rs:832: matches!(
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:147:27 False: assert!(k == 0 || placed[k - 1].0 <= *s, "placement runs forward");
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:349:9 False: refused > 100 && agreed > 90_000 && bars > 200_000,
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:349:26 False: refused > 100 && agreed > 90_000 && bars > 200_000,
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:349:45 False: refused > 100 && agreed > 90_000 && bars > 200_000,
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:966:13 False: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:966:25 False: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:976:17 False: assert!(r.ltp >= 5 && r.sod < 86_400);
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:976:31 False: assert!(r.ltp >= 5 && r.sod < 86_400);
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1056:16 False: if u64::try_from(rebuilt.len()).unwrap() == entry.size && crc32(&rebuilt) == entry.crc {
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1069:9 False: listed > 1_000 && rebuilt_clean > 1_000,
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1069:27 False: listed > 1_000 && rebuilt_clean > 1_000,
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1354:13 False: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1354:29 False: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1638:13 False: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_seconds_attack_tests.rs:1638:44 False: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
UNCOVERED BRANCH crates/pull/src/gdfl_series_code_tests.rs:234:58 False: Some(CmRefusal::SeriesNotStored { code, .. }) if *code == *edge
UNCOVERED BRANCH crates/pull/src/gdfl_series_code_tests.rs:238:59 False: Some(CmRefusal::SeriesNotStored { share, .. }) if &*share == widest
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:600: table.fault = Some(CmRefusal::TickStoreMalformed {
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:601: what: WALK_OVER_CAP,
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:602: });
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:603: break 'walk;
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:608: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:609: table.fault = Some(CmRefusal::TickStoreUnavailable { kind: why.kind() });
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:610: break 'walk;
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:649: return Err(CmRefusal::DayFileMisfiled {
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:650: found: found.clone(),
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:651: expected: expected.clone(),
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:652: });
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1691: for path in [&cm_day, &nfo_day] {
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1692: let _release = std::fs::OpenOptions::new().write(true).open(path);
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1693: }
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1785: frame.len()
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1834: out.push(0x60);
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1835: out.extend_from_slice(&u16::try_from(n - 256).unwrap().to_le_bytes());
UNCOVERED LINE crates/pull/src/gdfl_tickstore.rs:1836: }
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:591:75 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound && rel == top => break,
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:599:20 True: if visited > TICKSTORE_WALK_CAP {
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:615:37 False: if kind.is_dir() && !name.starts_with('.') {
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:630:28 False: if table.stray_named.len() < TICKSTORE_NAMED {
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:648:16 True: if let Some((found, expected)) = self.misfiled(tree, day) {
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:1688:20 True: if answered.is_err() {
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:1701:28 False: if what.contains(named)),
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:1727:73 False: matches!(answer, Err(CmRefusal::TickStoreMalformed { what }) if *what == FRAME)
UNCOVERED BRANCH crates/pull/src/gdfl_tickstore.rs:1830:12 False: if n < 256 {
UNCOVERED LINE crates/pull/src/gdfl_v3.rs:360: Self::Missing => f.write_str("missing"),
UNCOVERED LINE crates/pull/src/gdfl_v3.rs:629: return false;
UNCOVERED BRANCH crates/pull/src/gdfl_v3.rs:625:9 False: let Some(rest) = path
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:207: panic!("a listing without its directories agreed");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:221: panic!("a configured ledger is unset");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:304: panic!("two differences agreed");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:311: panic!("a renamed path agreed");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:337: panic!("a longer listing agreed");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:465: panic!("a directory at the marker's name was trusted");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:574: panic!("a directory at the manifest's name was trusted");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:763: panic!("an empty manifest agreed with an entry");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:864: fn the_real_v3_ledger_agrees_with_the_real_year_zips() {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:866: let env = |var: &str| std::env::var_os(var);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:867: let ledger = V3Ledger::from_env(&env);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:868: let V3Ledger::At { root } = &ledger else {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:869: panic!("BRUTEX_GDFL_V3_LEDGER is unset");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:871: let dir = std::path::PathBuf::from(env("BRUTEX_GDFL_NFO_ARCHIVES").expect("archives"));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:872: let zips = NfoZips::new(&dir);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:874: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:875: let first = zips.day(day(2019, 4, 1));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:876: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:878: started.elapsed(),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:879: matches!(first, Ok(Some(_)))
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:881: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:882: let again = zips.day(day(2019, 4, 2));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:883: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:885: started.elapsed(),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:886: matches!(again, Ok(Some(_)))
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:888: let parse = |text: &str| {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:889: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:890: day(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:891: parts[0],
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:892: u8::try_from(parts[1]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:893: u8::try_from(parts[2]).unwrap(),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:895: };
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:897: for year in 2018..=2026_u16 {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:898: let count = |sub: &str, suffix: &str| {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:899: std::fs::read_dir(root.join(format!("{year}/_v3/{sub}")))
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:900: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:901: .filter(|name| {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:902: name.as_ref()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:903: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:904: .file_name()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:905: .to_str()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:906: .is_some_and(|n| n.ends_with(suffix))
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:907: })
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:908: .count()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:909: };
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:910: let (manifests, markers) = (count("manifest", ".ok.tsv"), count("ok", ".ok"));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:911: eprintln!("v3 {year}: manifests={manifests} ok_markers={markers}");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:912: assert_eq!(manifests, markers, "{year}: one ok marker a manifest");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:913: assert!(manifests > 0, "{year}: no manifest");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:916: let mut records = std::collections::HashMap::new();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:917: for year in 2018..=2026_u16 {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:918: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:919: continue;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:921: let len = file.metadata().unwrap().len();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:922: for entry in crate::gdfl_archive::zip_entries(&file, 0, len).unwrap() {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:923: records.insert(entry.name.clone(), (entry.len, entry.crc32));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:924: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:926: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:927: list.split(',').map(parse).collect()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:929: let from = std::env::var("BRUTEX_V3_FROM").map_or(day(2018, 1, 1), |t| parse(&t));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:930: let to = std::env::var("BRUTEX_V3_TO").map_or(day(2026, 12, 31), |t| parse(&t));
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:931: let mut out = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:932: let mut at = from;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:933: while at <= to {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:934: out.push(at);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:935: at = at.succ().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:936: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:937: out
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:939: let (mut codes, mut held, mut marker_agrees, mut marker_differs) =
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:940: ([0_usize; 4], 0_usize, 0_usize, 0_usize);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:941: let mut nanos: Vec<u128> = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:942: let mut lines = 0_usize;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:943: for d in days {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:944: let listing = match zips.day(d) {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:945: Ok(listing) => listing,
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:946: Err(why) => {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:947: eprintln!("v3 {d}: source refused: {why}");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:948: continue;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:951: if listing.is_some() {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:952: held += 1;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:953: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:954: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:955: let answer = ledger.check(d, listing.as_ref());
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:956: let took = started.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:957: if listing.is_none() && answer.code() == 0 {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:958: continue; // neither holds the day: a weekend or a holiday
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:959: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:960: nanos.push(took);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:961: codes[usize::from(answer.code())] += 1;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:962: if let V3::Agrees { lines: n } = answer {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:963: lines += n;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:964: } else {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:965: eprintln!("v3 {d}: {answer}");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:966: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:968: let name = NfoZips::day_zip_name(d);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:969: if let (Ok(text), Some(&(len, crc))) = (
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:970: std::fs::read_to_string(ledger.marker_path(d).unwrap()),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:971: records.get(&name),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:973: let value = |key: &str| {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:974: text.lines()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:975: .find_map(|line| line.strip_prefix(&format!("{key}=")))
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:976: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:977: .to_owned()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:978: };
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:979: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:980: marker_agrees += 1;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:981: } else {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:982: marker_differs += 1;
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:983: eprintln!("v3 {d}: the ok marker's crc/size are not the day zip's");
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:984: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:985: }
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:987: nanos.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:988: let at = |q: usize| {
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:989: nanos
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:990: .get((nanos.len().saturating_sub(1)) * q / 100)
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:991: .copied()
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:992: .unwrap_or(0)
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:993: };
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:994: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:996: codes[0], codes[1], codes[2], codes[3]
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:998: eprintln!(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1000: nanos.len(),
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1001: at(50) / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1002: at(99) / 1_000,
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1003: nanos.last().copied().unwrap_or(0) / 1_000
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1005: assert_eq!(
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1006: codes[2] + codes[3],
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1010: assert_eq!(marker_differs, 0);
UNCOVERED LINE crates/pull/src/gdfl_v3_tests.rs:1011: }
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:206:9 False: let V3::Disagrees { count, first } = v3.check(d, Some(&files_only)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:220:9 False: let V3Ledger::At { root } = &v3 else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:303:9 False: let V3::Disagrees { count, first } = check(&both, Some(99)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:310:9 False: let V3::Disagrees { count, first } = check(&renamed, Some(99)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:336:9 False: let V3::Disagrees { count, first } = check(&longer, Some(99)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:464:9 False: let V3::Unverified { why } = dir_marker.check(d, Some(&good)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:573:9 False: let V3::Unverified { why } = dir_manifest.check(d, Some(&good)) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:759:9 False: let V3::Disagrees { count, first } = v3.check(
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:868:9 True: let V3Ledger::At { root } = &ledger else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:868:9 False: let V3Ledger::At { root } = &ledger else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:918:13 True: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:918:13 False: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:926:33 True: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:926:33 False: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:933:15 True: while at <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:933:15 False: while at <= to {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:951:12 True: if listing.is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:951:12 False: if listing.is_some() {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:957:12 True: if listing.is_none() && answer.code() == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:957:12 False: if listing.is_none() && answer.code() == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:957:33 True: if listing.is_none() && answer.code() == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:957:33 False: if listing.is_none() && answer.code() == 0 {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:962:16 True: if let V3::Agrees { lines: n } = answer {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:962:16 False: if let V3::Agrees { lines: n } = answer {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:969:16 True: if let (Ok(text), Some(&(len, crc))) = (
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:969:16 False: if let (Ok(text), Some(&(len, crc))) = (
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:979:16 True: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:979:16 False: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:979:52 True: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
UNCOVERED BRANCH crates/pull/src/gdfl_v3_tests.rs:979:52 False: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:724: fn spread(samples: &mut [u128]) -> (u128, u128, u128) {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:725: samples.sort_unstable();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:726: let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:727: (at(50), at(99), samples[samples.len() - 1])
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:728: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:732: fn synthetic_ledger(events: usize, ex: Day, expiry: Day) -> String {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:733: let mut text = String::new();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:734: for u in 0..events {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:735: let underlying = sym(&format!("ZZ{u:05}"));
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:736: let key = |strike: i64| {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:737: cd(expiry, strike, OptionSide::Call, 1, 0, 0)
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:738: .contract()
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:739: .unwrap()
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:740: };
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:741: for i in 0..40_i64 {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:742: let record = crate::gdfl_lineage::LineageRecord {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:743: underlying,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:744: old: key(20_000 + i * 500),
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:745: new: key(20_000 + i * 500 - 900),
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:746: ex_date: ex,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:747: factor: (1, 1),
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:748: lot_old: 3_400,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:749: lot_new: 3_400,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:750: oi_unadjusted_until: 0,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:751: };
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:752: text.push_str(&record.line());
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:753: text.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:754: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:755: for i in 0..10_i64 {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:756: let record = StoppedRecord {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:757: underlying,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:758: key: key(90_000 + i * 500),
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:759: ex_date: ex,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:760: };
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:761: text.push_str(&record.line());
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:762: text.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:763: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:764: let record = EventRecord {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:765: underlying,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:766: ex_date: ex,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:767: kind: EventKind::StrikeOffset { shift: 900 },
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:768: lot_old: 3_400,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:769: lot_new: 3_400,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:770: paired: 40,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:771: stopped: 10,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:772: };
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:773: text.push_str(&record.line());
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:774: text.push('\n');
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:776: text
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:777: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:787: fn the_ledger_load_and_a_resumed_day_are_timed() {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:788: let (ex, later, expiry) = (d(2020, 2, 20), d(2020, 3, 11), d(2020, 3, 26));
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:789: let mut folds = Vec::new();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:790: for events in [200_usize, 2_000] {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:791: let store = scratch("wp12e-timed");
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:792: let text = synthetic_ledger(events, ex, expiry);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:793: let lines = text.lines().count();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:794: let path = lineage_path(&store);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:795: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:796: std::fs::write(&path, &text).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:798: let mut loads: Vec<u128> = (0..30)
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:799: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:800: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:801: let run = LineageRun::load(&store, later).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:802: let took = at.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:803: assert!(run.last_day().is_none());
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:804: took
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:805: })
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:806: .collect();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:807: let (p50, p99, max) = spread(&mut loads);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:808: println!(
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:815: let day_files: Vec<ContractDay> = (0..400_i64)
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:816: .map(|i| cd(expiry, 20_000 + i * 500, OptionSide::Call, 3_400, 10, 20))
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:817: .collect();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:818: let mut run = LineageRun::load(&store, later).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:819: let first = fold(&mut run, later, "ZZ00007", &day_files);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:820: assert!(first.lineage >= 50, "the 50 reused texts are stitched");
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:821: let contracts = files("ZZ00007", &day_files);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:822: let mut days: Vec<u128> = (0..500)
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:823: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:824: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:825: let written = run.day(later, &contracts, &mut |_| Ok(Vec::new())).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:826: let took = at.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:827: assert_eq!((written.lineage, written.lots), (0, 0));
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:828: took
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:829: })
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:830: .collect();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:831: let (p50, p99, max) = spread(&mut days);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:832: println!(
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:835: folds.push(p50);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:836: std::fs::remove_dir_all(store).unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:838: assert!(
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:839: folds[1] <= folds[0].max(1) * 3,
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:842: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:849: fn the_split_probe_is_timed() {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:850: let later = d(2020, 9, 16);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:851: let real = std::env::var_os("BRUTEX_WP12E_STORE").map(std::path::PathBuf::from);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:852: let store = real.clone().unwrap_or_else(|| scratch("wp12e-timed-split"));
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:853: let run = options_run(&store, later, &[]);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:854: let held = run.split_underlyings().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:855: let mut probes: Vec<u128> = (0..200)
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:856: .map(|_| {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:857: let at = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:858: let got = run.split_underlyings().unwrap();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:859: let took = at.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:860: assert_eq!(got, held);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:861: took
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:862: })
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:863: .collect();
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:864: let (p50, p99, max) = spread(&mut probes);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:865: println!(
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:867: store.display(),
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:868: held.len()
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:870: if real.is_none() {
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:871: let _ = std::fs::remove_dir_all(store);
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:872: }
UNCOVERED LINE crates/pull/src/gdfl_wp12e_tests.rs:873: }
UNCOVERED BRANCH crates/pull/src/gdfl_wp12e_tests.rs:373:43 False: if !line.starts_with("event ") && !line.starts_with("stopped ") {
UNCOVERED BRANCH crates/pull/src/gdfl_wp12e_tests.rs:610:57 False: Err(ImportRefusal::UnderlyingSplit { held }) if held == [("TATAMOTORS".to_owned(), "TMPV".to_owned())]
UNCOVERED BRANCH crates/pull/src/gdfl_wp12e_tests.rs:713:16 False: if path.is_dir() {
UNCOVERED BRANCH crates/pull/src/gdfl_wp12e_tests.rs:870:8 True: if real.is_none() {
UNCOVERED BRANCH crates/pull/src/gdfl_wp12e_tests.rs:870:8 False: if real.is_none() {
UNCOVERED LINE crates/pull/src/http.rs:528: detail: format!("the HTTPS client could not be built: {why}"),
UNCOVERED LINE crates/pull/src/http.rs:529: })?;
UNCOVERED LINE crates/pull/src/http.rs:2359: _ => None,
UNCOVERED LINE crates/pull/src/http.rs:2931: builder = builder.header(*header, *word);
UNCOVERED LINE crates/pull/src/http.rs:2932: }
UNCOVERED LINE crates/pull/src/http.rs:3081: return;
UNCOVERED LINE crates/pull/src/http.rs:3086: return;
UNCOVERED LINE crates/pull/src/http.rs:3135: builder = builder.header(*header, *word);
UNCOVERED LINE crates/pull/src/http.rs:3136: }
UNCOVERED LINE crates/pull/src/http.rs:3254: )?;
UNCOVERED LINE crates/pull/src/http.rs:3312: builder = builder.header(*header, *word);
UNCOVERED LINE crates/pull/src/http.rs:3313: }
UNCOVERED LINE crates/pull/src/http.rs:3443: error,
UNCOVERED LINE crates/pull/src/http.rs:3444: refused: Some(crate::response_keep::RefusedAnswer::of(status, &text)),
UNCOVERED LINE crates/pull/src/http.rs:3445: })?;
UNCOVERED LINE crates/pull/src/http.rs:3473: other => other.to_string(),
UNCOVERED LINE crates/pull/src/http.rs:3510: if crate::capture::unread_kept(feed) >= crate::capture::PER_SLOT {
UNCOVERED LINE crates/pull/src/http.rs:3511: return;
UNCOVERED LINE crates/pull/src/http.rs:3512: }
UNCOVERED LINE crates/pull/src/http.rs:3513: let Some(root) = crate::capture::root_or_refused(feed, "unreadable", crate::folder::root())
UNCOVERED LINE crates/pull/src/http.rs:3515: return;
UNCOVERED LINE crates/pull/src/http.rs:3517: drop(crate::capture::record_unreadable(
UNCOVERED LINE crates/pull/src/http.rs:3518: &root, feed, url, body, why,
UNCOVERED LINE crates/pull/src/http.rs:3714: detail: format!("bar {i} is {row}, and this vendor sends one ARRAY per bar"),
UNCOVERED LINE crates/pull/src/http.rs:3715: })?;
UNCOVERED LINE crates/pull/src/http.rs:3719: return Err(FetchError::TransportFailed {
UNCOVERED LINE crates/pull/src/http.rs:3720: detail: format!(
UNCOVERED LINE crates/pull/src/http.rs:3721: "bar {i} carries {} cell(s); this vendor sends six \
UNCOVERED LINE crates/pull/src/http.rs:3722: (timestamp, open, high, low, close, volume) or seven with \
UNCOVERED LINE crates/pull/src/http.rs:3723: open interest last",
UNCOVERED LINE crates/pull/src/http.rs:3724: cells.len()
UNCOVERED LINE crates/pull/src/http.rs:3725: ),
UNCOVERED LINE crates/pull/src/http.rs:3726: });
UNCOVERED LINE crates/pull/src/http.rs:3730: detail: format!("bar {i} has no cell {at}"),
UNCOVERED LINE crates/pull/src/http.rs:3731: })
UNCOVERED LINE crates/pull/src/http.rs:3753: null_bars += 1;
UNCOVERED LINE crates/pull/src/http.rs:3754: continue;
UNCOVERED LINE crates/pull/src/http.rs:3811: serde_json::Value::Null => 0,
UNCOVERED LINE crates/pull/src/http.rs:3821: // The event carries the fact to the FILE; the print carries it to a
UNCOVERED LINE crates/pull/src/http.rs:3822: // terminal. See the sibling site above, and gate 22.
UNCOVERED LINE crates/pull/src/http.rs:3823: let _noted = telemetry::emit(
UNCOVERED LINE crates/pull/src/http.rs:3824: &telemetry::Event::new(
UNCOVERED LINE crates/pull/src/http.rs:3825: telemetry::Level::Warn,
UNCOVERED LINE crates/pull/src/http.rs:3826: "pull.decode",
UNCOVERED LINE crates/pull/src/http.rs:3827: "bars carried a null price and were skipped",
UNCOVERED LINE crates/pull/src/http.rs:3828: )
UNCOVERED LINE crates/pull/src/http.rs:3829: .with("skipped", u64::try_from(null_bars).unwrap_or(u64::MAX))
UNCOVERED LINE crates/pull/src/http.rs:3830: .with("bars", u64::try_from(rows.len()).unwrap_or(u64::MAX)),
UNCOVERED LINE crates/pull/src/http.rs:3831: );
UNCOVERED LINE crates/pull/src/http.rs:3832: eprintln!(
UNCOVERED LINE crates/pull/src/http.rs:3833: // "in those intervals", not "in those minutes": this decoder is
UNCOVERED LINE crates/pull/src/http.rs:3834: // rung-blind by design and a daily pull comes through it too.
UNCOVERED LINE crates/pull/src/http.rs:3835: "brutex: {null_bars} of {} bars carried a null price and were skipped \
UNCOVERED LINE crates/pull/src/http.rs:3836: — the vendor reported no trade in those intervals",
UNCOVERED LINE crates/pull/src/http.rs:3837: rows.len()
UNCOVERED LINE crates/pull/src/http.rs:3838: );
UNCOVERED LINE crates/pull/src/http.rs:3863: detail: format!("bar {at} stamps {v}, and this feed spells its timestamps as text"),
UNCOVERED LINE crates/pull/src/http.rs:3864: })?;
UNCOVERED LINE crates/pull/src/http.rs:3880: detail: format!("bar {at} stamps {v}, and this feed spells its timestamps as text"),
UNCOVERED LINE crates/pull/src/http.rs:3881: })?;
UNCOVERED LINE crates/pull/src/http.rs:3890: detail: format!(
UNCOVERED LINE crates/pull/src/http.rs:3897: })?;
UNCOVERED LINE crates/pull/src/http.rs:3901: detail: format!("bar {at} stamps {text:?}, whose offset overflows the epoch"),
UNCOVERED LINE crates/pull/src/http.rs:3902: })
UNCOVERED LINE crates/pull/src/http.rs:3991: return Err(FetchError::PathSegmentUnusable {
UNCOVERED LINE crates/pull/src/http.rs:3992: placeholder,
UNCOVERED LINE crates/pull/src/http.rs:3993: value: value.to_owned(),
UNCOVERED LINE crates/pull/src/http.rs:3994: });
UNCOVERED LINE crates/pull/src/http.rs:4003: Err(FetchError::PathSegmentUnusable {
UNCOVERED LINE crates/pull/src/http.rs:4004: placeholder,
UNCOVERED LINE crates/pull/src/http.rs:4005: value: value.to_owned(),
UNCOVERED LINE crates/pull/src/http.rs:4006: })
UNCOVERED LINE crates/pull/src/http.rs:4133: panic!("{feed:?} is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4156: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4180: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4199: panic!("request {} in one second was admitted", per_second + 1)
UNCOVERED LINE crates/pull/src/http.rs:4245: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4319: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4361: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:4561: panic!("a repeated key must be refused: {refused:?}");
UNCOVERED LINE crates/pull/src/http.rs:4895: panic!("{sent} is not zero and must not be stored as one")
UNCOVERED LINE crates/pull/src/http.rs:5225: panic!("{name} ({volume}, {interest}): one row goes, not the window: {why}")
UNCOVERED LINE crates/pull/src/http.rs:5836: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:5849: panic!("a body naming DH-901 is a refusal whatever the status says")
UNCOVERED LINE crates/pull/src/http.rs:5866: panic!("the sentence names a dead token")
UNCOVERED LINE crates/pull/src/http.rs:5895: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:5949: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:6042: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:6150: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:6300: panic!("a missing timestamp array must refuse")
UNCOVERED LINE crates/pull/src/http.rs:6386: panic!("the descriptor says top level, so `data` is not searched")
UNCOVERED LINE crates/pull/src/http.rs:6410: panic!("the declared envelope is absent and must be named")
UNCOVERED LINE crates/pull/src/http.rs:6422: panic!("an array is not an envelope")
UNCOVERED LINE crates/pull/src/http.rs:6432: panic!("an empty object holds no envelope")
UNCOVERED LINE crates/pull/src/http.rs:6461: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:6517: || zoneless.bars.first().map(|b| b.ts_micros)
UNCOVERED LINE crates/pull/src/http.rs:6518: != landed.bars.first().map(|b| b.ts_micros),
UNCOVERED LINE crates/pull/src/http.rs:6521: zoneless.bars.len(),
UNCOVERED LINE crates/pull/src/http.rs:6522: landed.bars.len()
UNCOVERED LINE crates/pull/src/http.rs:6550: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:6571: matches!(
UNCOVERED LINE crates/pull/src/http.rs:6587: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:6645: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:6650: panic!("Groww is an HTTP feed");
UNCOVERED LINE crates/pull/src/http.rs:6713: return;
UNCOVERED LINE crates/pull/src/http.rs:6773: return;
UNCOVERED LINE crates/pull/src/http.rs:6782: break;
UNCOVERED LINE crates/pull/src/http.rs:6879: panic!("a redirect is a refusal this build reports, not one it hides")
UNCOVERED LINE crates/pull/src/http.rs:6926: panic!("429 is a refusal the governor needs to see")
UNCOVERED LINE crates/pull/src/http.rs:6970: panic!("an answer declaring more than the cap is refused, not decoded")
UNCOVERED LINE crates/pull/src/http.rs:7014: panic!("an answer past the cap is refused, not decoded")
UNCOVERED LINE crates/pull/src/http.rs:7078: Ok(body) => panic!(
UNCOVERED LINE crates/pull/src/http.rs:7081: body.len()
UNCOVERED LINE crates/pull/src/http.rs:7161: return;
UNCOVERED LINE crates/pull/src/http.rs:7175: Ok(body) => panic!("an invalid byte is refused, not replaced: {body:?}"),
UNCOVERED LINE crates/pull/src/http.rs:7206: return;
UNCOVERED LINE crates/pull/src/http.rs:7315: credited.push(format!("{what} ({lowered} to {now})"));
UNCOVERED LINE crates/pull/src/http.rs:7318: returned.push(what);
UNCOVERED LINE crates/pull/src/http.rs:7324: credited.join("; ")
UNCOVERED LINE crates/pull/src/http.rs:7347: allowance(&held)
UNCOVERED LINE crates/pull/src/http.rs:7412: panic!("500 is a refusal, however long its body")
UNCOVERED LINE crates/pull/src/http.rs:7427: detail.chars().count()
UNCOVERED LINE crates/pull/src/http.rs:7608: panic!("the variant a bad field already uses, not a new one")
UNCOVERED LINE crates/pull/src/http.rs:7879: panic!("500 is a refusal")
UNCOVERED LINE crates/pull/src/http.rs:7904: panic!("a 307 is still a redirect this build refuses")
UNCOVERED LINE crates/pull/src/http.rs:7935: panic!("a 301 is still a redirect this build refuses")
UNCOVERED LINE crates/pull/src/http.rs:7959: panic!("an empty object holds no bars and must be refused")
UNCOVERED LINE crates/pull/src/http.rs:7980: panic!("the sync seam cannot work and must say so")
UNCOVERED LINE crates/pull/src/http.rs:8019: panic!("{bad} is not a whole count and must be refused, not truncated")
UNCOVERED LINE crates/pull/src/http.rs:8039: panic!("an unreachable host is a transport failure, not a window")
UNCOVERED LINE crates/pull/src/http.rs:8057: panic!("9999-12-31 has no day after it")
UNCOVERED LINE crates/pull/src/http.rs:8140: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
UNCOVERED LINE crates/pull/src/http.rs:8161: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
UNCOVERED LINE crates/pull/src/http.rs:8343: panic!(
UNCOVERED LINE crates/pull/src/http.rs:8423: panic!("a bar with no close must be refused, not defaulted")
UNCOVERED LINE crates/pull/src/http.rs:8462: panic!("the declared envelope is absent and must be named")
UNCOVERED LINE crates/pull/src/http.rs:8532: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/src/http.rs:8545: crate::vendor::PathSegment::Value { placeholder, .. } => placeholder,
UNCOVERED LINE crates/pull/src/http.rs:8613: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/src/http.rs:8654: panic!("Groww is an HTTP broker");
UNCOVERED LINE crates/pull/src/http.rs:8684: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:8695: panic!("a body naming DH-901 is a refusal whatever the status says")
UNCOVERED LINE crates/pull/src/http.rs:8722: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
UNCOVERED LINE crates/pull/src/http.rs:8758: panic!("a refusal under a 200 is still a refusal")
UNCOVERED LINE crates/pull/src/http.rs:8770: panic!("a body that is not JSON fails the decode")
UNCOVERED LINE crates/pull/src/http.rs:8788: panic!("this feed is HTTP")
UNCOVERED LINE crates/pull/src/http.rs:8915: panic!("Dhan is an HTTP feed");
UNCOVERED BRANCH crates/pull/src/http.rs:1032:27 False: while let Some(&inner) = bytes.get(end) {
UNCOVERED BRANCH crates/pull/src/http.rs:1046:28 False: && let Some(Some(keys)) = open.last_mut()
UNCOVERED BRANCH crates/pull/src/http.rs:1047:28 False: && let Some(key) = body
UNCOVERED BRANCH crates/pull/src/http.rs:3000:16 False: && status.as_u16() != 429
UNCOVERED BRANCH crates/pull/src/http.rs:3001:20 False: && let Some(lock) = self.governor.as_ref()
UNCOVERED BRANCH crates/pull/src/http.rs:3028:20 False: && let Some(lock) = self.governor.as_ref()
UNCOVERED BRANCH crates/pull/src/http.rs:3080:12 True: if crate::capture::kept(feed, method) >= crate::capture::PER_SLOT {
UNCOVERED BRANCH crates/pull/src/http.rs:3083:13 False: let Some(root) =
UNCOVERED BRANCH crates/pull/src/http.rs:3353:20 False: && let Some(lock) = self.governor.as_ref()
UNCOVERED BRANCH crates/pull/src/http.rs:3507:13 True: let Some(feed) = self.feed else {
UNCOVERED BRANCH crates/pull/src/http.rs:3510:12 True: if crate::capture::unread_kept(feed) >= crate::capture::PER_SLOT {
UNCOVERED BRANCH crates/pull/src/http.rs:3510:12 False: if crate::capture::unread_kept(feed) >= crate::capture::PER_SLOT {
UNCOVERED BRANCH crates/pull/src/http.rs:3513:13 True: let Some(root) = crate::capture::root_or_refused(feed, "unreadable", crate::folder::root())
UNCOVERED BRANCH crates/pull/src/http.rs:3513:13 False: let Some(root) = crate::capture::root_or_refused(feed, "unreadable", crate::folder::root())
UNCOVERED BRANCH crates/pull/src/http.rs:3632:24 False: && let Some(lock) = self.governor.as_ref()
UNCOVERED BRANCH crates/pull/src/http.rs:3644:16 False: if let Some(lock) = self.governor.as_ref() {
UNCOVERED BRANCH crates/pull/src/http.rs:3718:32 True: if cells.len() != 6 && cells.len() != 7 {
UNCOVERED BRANCH crates/pull/src/http.rs:3752:12 True: if (1..=4).any(|at| cell(at).is_ok_and(serde_json::Value::is_null)) {
UNCOVERED BRANCH crates/pull/src/http.rs:3820:8 True: if null_bars > 0 {
UNCOVERED BRANCH crates/pull/src/http.rs:3990:8 True: if value == "." || value == ".." {
UNCOVERED BRANCH crates/pull/src/http.rs:3990:24 True: if value == "." || value == ".." {
UNCOVERED BRANCH crates/pull/src/http.rs:3996:18 False: let usable = !value.is_empty()
UNCOVERED BRANCH crates/pull/src/http.rs:3999:22 False: .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~'));
UNCOVERED BRANCH crates/pull/src/http.rs:4000:8 False: if usable {
UNCOVERED BRANCH crates/pull/src/http.rs:4132:17 False: let crate::vendor::Transport::Http(spec) = feed.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/http.rs:4154:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:4178:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:4243:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:4317:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:4359:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:4560:13 False: let Err(FetchError::TransportFailed { detail }) = refused else {
UNCOVERED BRANCH crates/pull/src/http.rs:4564:13 False: detail.contains("repeats") && detail.contains("open"),
UNCOVERED BRANCH crates/pull/src/http.rs:4564:43 False: detail.contains("repeats") && detail.contains("open"),
UNCOVERED BRANCH crates/pull/src/http.rs:4890:17 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:5499:13 False: said.contains("no timestamp") && said.contains("a price below zero"),
UNCOVERED BRANCH crates/pull/src/http.rs:5499:46 False: said.contains("no timestamp") && said.contains("a price below zero"),
UNCOVERED BRANCH crates/pull/src/http.rs:5846:13 False: let Err(FetchError::VendorRefused { status, named, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:5863:13 False: let Err(FetchError::VendorRefused { named, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:6295:13 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:6381:13 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:6405:13 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:6417:13 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:6427:13 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:6458:13 False: let crate::vendor::Transport::Http(zerodha) =
UNCOVERED BRANCH crates/pull/src/http.rs:6516:13 False: zoneless.bars.len() != landed.bars.len()
UNCOVERED BRANCH crates/pull/src/http.rs:6517:20 True: || zoneless.bars.first().map(|b| b.ts_micros)
UNCOVERED BRANCH crates/pull/src/http.rs:6517:20 False: || zoneless.bars.first().map(|b| b.ts_micros)
UNCOVERED BRANCH crates/pull/src/http.rs:6547:13 False: let crate::vendor::Transport::Http(zerodha) =
UNCOVERED BRANCH crates/pull/src/http.rs:6584:13 False: let crate::vendor::Transport::Http(zerodha) =
UNCOVERED BRANCH crates/pull/src/http.rs:6634:31 False: .any(|(h, v)| *h == "X-Kite-Version" && *v == "3"),
UNCOVERED BRANCH crates/pull/src/http.rs:6642:13 False: let crate::vendor::Transport::Http(zerodha) =
UNCOVERED BRANCH crates/pull/src/http.rs:6647:13 False: let crate::vendor::Transport::Http(groww) =
UNCOVERED BRANCH crates/pull/src/http.rs:6712:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/http.rs:6719:20 False: if let Some(ref body) = answer {
UNCOVERED BRANCH crates/pull/src/http.rs:6772:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/http.rs:6781:20 True: if !finished {
UNCOVERED BRANCH crates/pull/src/http.rs:6794:34 False: Ok(n) if n > 0 => {
UNCOVERED BRANCH crates/pull/src/http.rs:6798:37 True: Err(why) if why.kind() == ErrorKind::Interrupted => {}
UNCOVERED BRANCH crates/pull/src/http.rs:6878:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
UNCOVERED BRANCH crates/pull/src/http.rs:6925:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
UNCOVERED BRANCH crates/pull/src/http.rs:6969:13 False: let Err(FetchError::TransportFailed { detail }) = outcome else {
UNCOVERED BRANCH crates/pull/src/http.rs:7011:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:7087:13 False: refusal.detail.contains("ceiling")
UNCOVERED BRANCH crates/pull/src/http.rs:7088:20 False: && refusal.detail.contains(&MAX_RESPONSE_BYTES.to_string()),
UNCOVERED BRANCH crates/pull/src/http.rs:7160:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/http.rs:7205:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/http.rs:7314:16 True: if now != lowered {
UNCOVERED BRANCH crates/pull/src/http.rs:7317:16 True: if outcome.is_ok() {
UNCOVERED BRANCH crates/pull/src/http.rs:7409:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:7607:13 False: let FetchError::TransportFailed { detail } = why else {
UNCOVERED BRANCH crates/pull/src/http.rs:7830:13 True: sent.contains("access-token: SUPERSECRET") || sent.contains("x-token: SUPERSECRET")
UNCOVERED BRANCH crates/pull/src/http.rs:7830:59 False: sent.contains("access-token: SUPERSECRET") || sent.contains("x-token: SUPERSECRET")
UNCOVERED BRANCH crates/pull/src/http.rs:7878:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
UNCOVERED BRANCH crates/pull/src/http.rs:7901:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:7932:13 False: let Err(FetchError::VendorRefused { status, detail, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:7956:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:7979:13 False: let Err(FetchError::TransportFailed { detail }) = source.window(&one_day()) else {
UNCOVERED BRANCH crates/pull/src/http.rs:8014:17 False: let Err(FetchError::TransportFailed { detail }) = decode_body(
UNCOVERED BRANCH crates/pull/src/http.rs:8036:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8054:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8342:13 False: let Err(FetchError::RungNotSpellable { rung, field }) = source.block_on(&daily) else {
UNCOVERED BRANCH crates/pull/src/http.rs:8420:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8459:13 False: let Err(FetchError::TransportFailed { detail }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8530:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:8611:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:8651:13 False: let crate::vendor::Transport::Http(spec) =
UNCOVERED BRANCH crates/pull/src/http.rs:8692:13 False: let Err(FetchError::VendorRefused { detail, named, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8755:13 False: let Err(FetchError::VendorRefused { named, .. }) =
UNCOVERED BRANCH crates/pull/src/http.rs:8765:13 False: let Ok(Err(FetchError::TransportFailed { detail })) = source.settle_answer(
UNCOVERED BRANCH crates/pull/src/http.rs:8786:13 False: let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:8913:13 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
UNCOVERED BRANCH crates/pull/src/http.rs:9181:86 False: matches!(unanswered.error, FetchError::TransportFailed { ref detail } if detail.contains("Refused before the body was read")),
UNCOVERED LINE crates/pull/src/http_cov_tests.rs:59: panic!("{refused:?}");
UNCOVERED LINE crates/pull/src/http_cov_tests.rs:123: panic!("{refused:?}");
UNCOVERED BRANCH crates/pull/src/http_cov_tests.rs:58:13 False: let FetchError::TransportFailed { detail } = &refused else {
UNCOVERED BRANCH crates/pull/src/http_cov_tests.rs:122:13 False: let FetchError::NotABarsParam { field } = &refused else {
UNCOVERED LINE crates/pull/src/identity_screen.rs:780: (FirstDay::On(_), None) => false,
UNCOVERED BRANCH crates/pull/src/identity_screen.rs:747:16 False: if let Ok(month) = day.year_month() {
UNCOVERED LINE crates/pull/src/ingest.rs:632: _ => String::new(),
UNCOVERED LINE crates/pull/src/ingest.rs:932: Err(why) => return (refused_whole(members, &census_path, why), refusals),
UNCOVERED LINE crates/pull/src/ingest.rs:953: return (refused_whole(members, &census_path, why), refusals);
UNCOVERED LINE crates/pull/src/ingest.rs:1443: Err(differ) => {
UNCOVERED LINE crates/pull/src/ingest.rs:1444: *unpaired = unpaired.saturating_add(
UNCOVERED LINE crates/pull/src/ingest.rs:1445: u32::try_from(named.local_seconds.len().max(named.causes.len()))
UNCOVERED LINE crates/pull/src/ingest.rs:1446: .unwrap_or(u32::MAX),
UNCOVERED LINE crates/pull/src/ingest.rs:1447: );
UNCOVERED LINE crates/pull/src/ingest.rs:1448: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/ingest.rs:1449: &telemetry::Event::error(
UNCOVERED LINE crates/pull/src/ingest.rs:1450: "pull.refused",
UNCOVERED LINE crates/pull/src/ingest.rs:1451: "refused rows whose causes do not pair with their stamps",
UNCOVERED LINE crates/pull/src/ingest.rs:1452: )
UNCOVERED LINE crates/pull/src/ingest.rs:1453: .with("seconds", telemetry::Value::Uint(differ.seconds as u64))
UNCOVERED LINE crates/pull/src/ingest.rs:1454: .with("causes", telemetry::Value::Uint(differ.causes as u64)),
UNCOVERED LINE crates/pull/src/ingest.rs:1455: );
UNCOVERED LINE crates/pull/src/ingest.rs:1456: }
UNCOVERED LINE crates/pull/src/ingest.rs:1705: return None;
UNCOVERED LINE crates/pull/src/ingest.rs:1924: return (done, Filed::Appended);
UNCOVERED LINE crates/pull/src/ingest.rs:2035: note_not_filed(instrument, "overlay", &why);
UNCOVERED LINE crates/pull/src/ingest.rs:2036: done.failures
UNCOVERED LINE crates/pull/src/ingest.rs:2037: .push(Failure::new(instrument, why).in_month(ym));
UNCOVERED LINE crates/pull/src/ingest.rs:2118: Err(_) => census.count(crate::session::DropReason::BeforeWindow),
UNCOVERED LINE crates/pull/src/ingest.rs:2144: failure.why = format!("{} (from {origin})", failure.why);
UNCOVERED LINE crates/pull/src/ingest.rs:2145: }
UNCOVERED LINE crates/pull/src/ingest.rs:2564: seconds.extend(offered.iter().map(|bar| local_second_of(bar.ts_micros)));
UNCOVERED LINE crates/pull/src/ingest.rs:2565: return;
UNCOVERED LINE crates/pull/src/ingest.rs:2575: index = end;
UNCOVERED LINE crates/pull/src/ingest.rs:2576: break;
UNCOVERED LINE crates/pull/src/ingest.rs:2799: kind: FailKind::of_store(&why),
UNCOVERED LINE crates/pull/src/ingest.rs:2800: why: format!("{appended}; the generation served could not be read: {why}"),
UNCOVERED LINE crates/pull/src/ingest.rs:2801: })?
UNCOVERED LINE crates/pull/src/ingest.rs:2806: kind: appended.kind,
UNCOVERED LINE crates/pull/src/ingest.rs:2807: why: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:2810: })?;
UNCOVERED LINE crates/pull/src/ingest.rs:2895: return appended;
UNCOVERED LINE crates/pull/src/ingest.rs:2910: Err(why) => appended.and(&format!(
UNCOVERED LINE crates/pull/src/ingest.rs:2911: "; the month as served could not be read to tell which hole the day is ({why})"
UNCOVERED LINE crates/pull/src/ingest.rs:2912: )),
UNCOVERED LINE crates/pull/src/ingest.rs:2931: held_of_served(store_root, path, symbol_id, key)?,
UNCOVERED LINE crates/pull/src/ingest.rs:2933: Vec::new(),
UNCOVERED LINE crates/pull/src/ingest.rs:2966: kind: FailKind::Io {
UNCOVERED LINE crates/pull/src/ingest.rs:2967: kind: ErrorKind::Other,
UNCOVERED LINE crates/pull/src/ingest.rs:2968: publication_may_be_visible: true,
UNCOVERED LINE crates/pull/src/ingest.rs:2969: },
UNCOVERED LINE crates/pull/src/ingest.rs:2970: why,
UNCOVERED LINE crates/pull/src/ingest.rs:2971: })
UNCOVERED LINE crates/pull/src/ingest.rs:2981: Err(journal) => Err(DoorRefusal {
UNCOVERED LINE crates/pull/src/ingest.rs:2982: kind,
UNCOVERED LINE crates/pull/src/ingest.rs:2983: why: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:2984: "{refused}; and the repair journal line was not written: {journal}"
UNCOVERED LINE crates/pull/src/ingest.rs:2985: ),
UNCOVERED LINE crates/pull/src/ingest.rs:2986: }),
UNCOVERED LINE crates/pull/src/ingest.rs:3062: return Ok(Lack::AppendDoors);
UNCOVERED LINE crates/pull/src/ingest.rs:3071: return Ok(Lack::AppendDoors);
UNCOVERED LINE crates/pull/src/ingest.rs:3076: Err(store::file::StoreError::Missing { .. }) => return Ok(Lack::AppendDoors),
UNCOVERED LINE crates/pull/src/ingest.rs:3077: Err(why) => return Err(why),
UNCOVERED LINE crates/pull/src/ingest.rs:3081: return Ok(Lack::AppendDoors);
UNCOVERED LINE crates/pull/src/ingest.rs:3095: }
UNCOVERED LINE crates/pull/src/ingest.rs:3148: Filed::Appended
UNCOVERED LINE crates/pull/src/ingest.rs:3174: Ok(Lack::Overlaps { .. }) if busy => return Err(appended),
UNCOVERED LINE crates/pull/src/ingest.rs:3183: Err(why) => {
UNCOVERED LINE crates/pull/src/ingest.rs:3184: let unread = DoorRefusal::store(&why);
UNCOVERED LINE crates/pull/src/ingest.rs:3185: return Err(DoorRefusal {
UNCOVERED LINE crates/pull/src/ingest.rs:3186: kind: unread.kind,
UNCOVERED LINE crates/pull/src/ingest.rs:3187: why: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:3188: "{appended}; not back-filled: the month as served could not be read to judge it ({unread}), so nothing was written"
UNCOVERED LINE crates/pull/src/ingest.rs:3189: ),
UNCOVERED LINE crates/pull/src/ingest.rs:3190: });
UNCOVERED LINE crates/pull/src/ingest.rs:3198: DoorRefusal {
UNCOVERED LINE crates/pull/src/ingest.rs:3199: kind: appended.kind,
UNCOVERED LINE crates/pull/src/ingest.rs:3200: why: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:3201: "{appended}; not back-filled: the repair journal would not take the `repairing` line ({why}), so nothing was published"
UNCOVERED LINE crates/pull/src/ingest.rs:3202: ),
UNCOVERED LINE crates/pull/src/ingest.rs:3203: }
UNCOVERED LINE crates/pull/src/ingest.rs:3204: })?;
UNCOVERED LINE crates/pull/src/ingest.rs:3214: Filed::Appended
UNCOVERED LINE crates/pull/src/ingest.rs:3278: "-".to_owned()
UNCOVERED LINE crates/pull/src/ingest.rs:3334: note_not_filed(parts.symbol, "repair journal", &why);
UNCOVERED LINE crates/pull/src/ingest.rs:3335: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:3339: })
UNCOVERED LINE crates/pull/src/ingest.rs:3664: break;
UNCOVERED LINE crates/pull/src/ingest.rs:3781: return Err(format!(
UNCOVERED LINE crates/pull/src/ingest.rs:3782: "{instrument} {month}: committed source timestamp belongs to {day}"
UNCOVERED LINE crates/pull/src/ingest.rs:3783: ));
UNCOVERED LINE crates/pull/src/ingest.rs:3973: Err(why) => {
UNCOVERED LINE crates/pull/src/ingest.rs:3974: for rung in derived_from(source) {
UNCOVERED LINE crates/pull/src/ingest.rs:3975: note_not_derived(instrument, rung, &why);
UNCOVERED LINE crates/pull/src/ingest.rs:3976: }
UNCOVERED LINE crates/pull/src/ingest.rs:3977: failures.push(Failure::new(instrument, why).in_month(into.month));
UNCOVERED LINE crates/pull/src/ingest.rs:3978: return failures;
UNCOVERED LINE crates/pull/src/ingest.rs:4121: return failures;
UNCOVERED LINE crates/pull/src/ingest.rs:4135: Err(why) => {
UNCOVERED LINE crates/pull/src/ingest.rs:4136: for rung in derived_from(source) {
UNCOVERED LINE crates/pull/src/ingest.rs:4137: note_not_derived(instrument, rung, &why);
UNCOVERED LINE crates/pull/src/ingest.rs:4138: }
UNCOVERED LINE crates/pull/src/ingest.rs:4139: failures.push(Failure::new(instrument, why).in_month(into.month));
UNCOVERED LINE crates/pull/src/ingest.rs:4140: return failures;
UNCOVERED LINE crates/pull/src/ingest.rs:4158: continue;
UNCOVERED LINE crates/pull/src/ingest.rs:4168: Err(why) => {
UNCOVERED LINE crates/pull/src/ingest.rs:4169: failures.push(rung_refused(instrument, rung, into.month, &why.to_string()));
UNCOVERED LINE crates/pull/src/ingest.rs:4170: continue;
UNCOVERED LINE crates/pull/src/ingest.rs:4190: continue;
UNCOVERED LINE crates/pull/src/ingest.rs:4240: Err(journal) => format!("{why}; and the repair journal line was not written: {journal}"),
UNCOVERED LINE crates/pull/src/ingest.rs:4288: Err(why) => return Err(why.to_string()),
UNCOVERED LINE crates/pull/src/ingest.rs:4319: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:4323: })?;
UNCOVERED LINE crates/pull/src/ingest.rs:4554: {
UNCOVERED LINE crates/pull/src/ingest.rs:4555: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/ingest.rs:4556: &telemetry::Event::warn(
UNCOVERED LINE crates/pull/src/ingest.rs:4557: "pull.daycheck",
UNCOVERED LINE crates/pull/src/ingest.rs:4558: "pulled 1day holds fields one paisa under its own 1min on the 5-paisa grid",
UNCOVERED LINE crates/pull/src/ingest.rs:4559: )
UNCOVERED LINE crates/pull/src/ingest.rs:4560: .with("instrument", telemetry::Value::Str(instrument))
UNCOVERED LINE crates/pull/src/ingest.rs:4561: .with("month", telemetry::Value::Str(&month))
UNCOVERED LINE crates/pull/src/ingest.rs:4562: .with("rule", telemetry::Value::Str("vendor_truncated"))
UNCOVERED LINE crates/pull/src/ingest.rs:4563: .with(
UNCOVERED LINE crates/pull/src/ingest.rs:4564: "fields",
UNCOVERED LINE crates/pull/src/ingest.rs:4565: telemetry::Value::Uint(u64::try_from(report.vendor_truncated).unwrap_or(u64::MAX)),
UNCOVERED LINE crates/pull/src/ingest.rs:4566: )
UNCOVERED LINE crates/pull/src/ingest.rs:4567: .with(
UNCOVERED LINE crates/pull/src/ingest.rs:4568: "why",
UNCOVERED LINE crates/pull/src/ingest.rs:4569: telemetry::Value::Str(report.first_truncated.as_deref().unwrap_or("")),
UNCOVERED LINE crates/pull/src/ingest.rs:4570: ),
UNCOVERED LINE crates/pull/src/ingest.rs:4571: );
UNCOVERED LINE crates/pull/src/ingest.rs:4710: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:4714: })?;
UNCOVERED LINE crates/pull/src/ingest.rs:5008: format!(
UNCOVERED LINE crates/pull/src/ingest.rs:5021: }
UNCOVERED LINE crates/pull/src/ingest.rs:5501: return Ok(Err(Unbounded::PastCap { cap, size }));
UNCOVERED LINE crates/pull/src/ingest.rs:5711: Err(why) => {
UNCOVERED LINE crates/pull/src/ingest.rs:5712: return Err(format!(
UNCOVERED LINE crates/pull/src/ingest.rs:5713: "the census lock at {} opened but cannot be measured: {why}. \
UNCOVERED LINE crates/pull/src/ingest.rs:5714: Refused rather than run without it.",
UNCOVERED LINE crates/pull/src/ingest.rs:5715: lock_path.display()
UNCOVERED LINE crates/pull/src/ingest.rs:5716: ));
UNCOVERED LINE crates/pull/src/ingest.rs:5786: Some(Err(stat))
UNCOVERED LINE crates/pull/src/ingest.rs:5787: if !matches!(
UNCOVERED LINE crates/pull/src/ingest.rs:5790: ) =>
UNCOVERED LINE crates/pull/src/ingest.rs:5792: Err(format!(
UNCOVERED LINE crates/pull/src/ingest.rs:5793: "the census lock at {} could not be opened: {why}, and \
UNCOVERED LINE crates/pull/src/ingest.rs:5794: the directory that holds it could not be measured: \
UNCOVERED LINE crates/pull/src/ingest.rs:5795: {stat}. Refused rather than run without the lock.",
UNCOVERED LINE crates/pull/src/ingest.rs:5796: lock_path.display()
UNCOVERED LINE crates/pull/src/ingest.rs:5797: ))
UNCOVERED LINE crates/pull/src/ingest.rs:5822: return "a block device";
UNCOVERED LINE crates/pull/src/ingest.rs:5832: "an entry of a kind this host does not name"
UNCOVERED LINE crates/pull/src/ingest.rs:6430: field("why")
UNCOVERED LINE crates/pull/src/ingest.rs:6478: panic!("a lock that cannot be opened must refuse, never run unlocked")
UNCOVERED LINE crates/pull/src/ingest.rs:6553: panic!("a lock that could not be created must refuse, never run unlocked")
UNCOVERED LINE crates/pull/src/ingest.rs:6561: panic!("a regular lock that would not open must refuse, never run unlocked")
UNCOVERED LINE crates/pull/src/ingest.rs:6657: panic!("the premise: a held census lock refuses a second run")
UNCOVERED LINE crates/pull/src/ingest.rs:6663: panic!("the dropped lock was released despite the duplicate: {why}")
UNCOVERED LINE crates/pull/src/ingest.rs:6700: // Release the writer blocked in open(2) before failing.
UNCOVERED LINE crates/pull/src/ingest.rs:6701: let _reader = std::fs::File::open(&lock_path);
UNCOVERED LINE crates/pull/src/ingest.rs:6704: panic!("a FIFO at the lock path must refuse promptly, got {answer:?}")
UNCOVERED LINE crates/pull/src/ingest.rs:6738: tried.push(format!("{}: {why}", link.display()));
UNCOVERED LINE crates/pull/src/ingest.rs:6739: continue;
UNCOVERED LINE crates/pull/src/ingest.rs:6745: Err(why) => tried.push(format!("{}: {why}", link.display())),
UNCOVERED LINE crates/pull/src/ingest.rs:6748: panic!(
UNCOVERED LINE crates/pull/src/ingest.rs:6750: path.display(),
UNCOVERED LINE crates/pull/src/ingest.rs:6751: tried.join("; ")
UNCOVERED LINE crates/pull/src/ingest.rs:6776: found.file_type()
UNCOVERED LINE crates/pull/src/ingest.rs:6783: link.display()
UNCOVERED LINE crates/pull/src/ingest.rs:6836: panic!("a socket at the lock path must refuse, got {refused:?}")
UNCOVERED LINE crates/pull/src/ingest.rs:6849: panic!("a dangling symlink at the lock path must refuse, got {refused:?}")
UNCOVERED LINE crates/pull/src/ingest.rs:7067: // Release the stuck open so the suite does not hang, then fail.
UNCOVERED LINE crates/pull/src/ingest.rs:7068: let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
UNCOVERED BRANCH crates/pull/src/ingest.rs:952:12 True: if let Err(why) = target.still_where_located() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:1704:8 True: if held.is_empty() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:1923:9 False: let Some((first, last)) = bars.first().zip(bars.last()) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2034:12 True: if let Err(why) = write_overlay(overlays, store_root, symbol_id, parts) {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2453:20 False: if let Ok(day) = u32::try_from(local_day(ts))
UNCOVERED BRANCH crates/pull/src/ingest.rs:2541:28 False: && let Ok(number) = u32::try_from(day)
UNCOVERED BRANCH crates/pull/src/ingest.rs:2563:9 False: let (Some(mut index), Some(end)) = (at(day_start), at(day_end)) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2569:15 False: while index < end {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2573:21 False: let Ok(record) = file.read_record(index) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2586:12 True: if index >= end || stored != Some(bar.ts_micros) {
UNCOVERED BRANCH crates/pull/src/ingest.rs:2862:46 True: if appended.kind.repairable() || appended.kind.files_as_successor() =>
UNCOVERED BRANCH crates/pull/src/ingest.rs:2894:9 False: let Ok(path) = StorePath::new(parts) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3061:9 False: let Some((first, last)) = bars.first().zip(bars.last()) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3066:8 True: if bars
UNCOVERED BRANCH crates/pull/src/ingest.rs:3080:8 True: if header.n_valid == 0 {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3088:8 False: if at < header.n_valid {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3147:32 True: let filed = if added_at.is_empty() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3174:38 True: Ok(Lack::Overlaps { .. }) if busy => return Err(appended),
UNCOVERED BRANCH crates/pull/src/ingest.rs:3213:24 True: let filed = if added_at.is_empty() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3277:8 True: if field.is_empty() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3654:13 False: let Some(first) = bars.get(start) else { break };
UNCOVERED BRANCH crates/pull/src/ingest.rs:3663:13 False: let Some(slice) = bars.get(start..end) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:3780:16 True: if day.year_month().map_err(|why| why.to_string())? != month {
UNCOVERED BRANCH crates/pull/src/ingest.rs:4117:8 True: if !derives_for(into.vendor)
UNCOVERED BRANCH crates/pull/src/ingest.rs:4118:12 True: || into.contract.is_some_and(|c| c.is_option())
UNCOVERED BRANCH crates/pull/src/ingest.rs:4119:12 True: || source != Timeframe::MINUTE_1
UNCOVERED BRANCH crates/pull/src/ingest.rs:4157:13 False: let Some(bucket) = crate::fold::Bucket::of_secs(rung.secs()) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:4189:12 True: if bars.is_empty() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:4501:16 False: && target.secs().is_multiple_of(source.secs())
UNCOVERED BRANCH crates/pull/src/ingest.rs:4553:12 True: && report.vendor_truncated > 0
UNCOVERED BRANCH crates/pull/src/ingest.rs:5003:22 False: let why = if gaps == 1 {
UNCOVERED BRANCH crates/pull/src/ingest.rs:5015:12 False: if let Some(last) = last {
UNCOVERED BRANCH crates/pull/src/ingest.rs:5500:8 True: if size > cap {
UNCOVERED BRANCH crates/pull/src/ingest.rs:5607:16 False: if let Some(dir) = path.parent() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:5787:20 True: if !matches!(
UNCOVERED BRANCH crates/pull/src/ingest.rs:5821:12 True: if found.is_block_device() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:5829:15 False: } else if found.is_file() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6333:16 False: if let Some(second) = moved.get_mut(1) {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6477:13 False: let Err(why) = CensusLock::take(&census) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6552:13 False: let Err(why) = CensusLock::unopened(&lock_path, &full) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6555:17 False: assert!(why.contains("man.lock") && why.contains("Refused"), "{why}");
UNCOVERED BRANCH crates/pull/src/ingest.rs:6555:45 False: assert!(why.contains("man.lock") && why.contains("Refused"), "{why}");
UNCOVERED BRANCH crates/pull/src/ingest.rs:6560:13 False: let Err(why) = CensusLock::unopened(&lock_path, &starved) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6656:13 False: let Err(busy) = CensusLock::take(&census) else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6699:12 True: if answer.is_err() {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6703:13 False: let Ok(Err(why)) = answer else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6737:20 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6768:46 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
UNCOVERED BRANCH crates/pull/src/ingest.rs:6835:13 False: let Err(why) = refused else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6848:13 False: let Err(why) = refused else {
UNCOVERED BRANCH crates/pull/src/ingest.rs:6906:17 False: !why.contains("another ingest") && !why.contains("Wait for the other run"),
UNCOVERED BRANCH crates/pull/src/ingest.rs:6906:52 False: !why.contains("another ingest") && !why.contains("Wait for the other run"),
UNCOVERED BRANCH crates/pull/src/ingest.rs:7066:12 True: if read.is_err() {
UNCOVERED BRANCH crates/pull/src/ingest_cov_tests.rs:266:9 False: none.flagged.is_empty() && none.months.is_empty(),
UNCOVERED BRANCH crates/pull/src/ingest_cov_tests.rs:266:36 False: none.flagged.is_empty() && none.months.is_empty(),
UNCOVERED LINE crates/pull/src/ingest_failkind.rs:149: _ => Self::Other,
UNCOVERED BRANCH crates/pull/src/ingest_failkind_tests.rs:353:49 False: && (rest.ends_with(" {") || rest.ends_with(','))
UNCOVERED BRANCH crates/pull/src/ingest_failkind_tests.rs:640:13 False: assert!(!late.is_hole() && late.files_as_successor());
UNCOVERED BRANCH crates/pull/src/ingest_failkind_tests.rs:640:32 False: assert!(!late.is_hole() && late.files_as_successor());
UNCOVERED LINE crates/pull/src/landing_run.rs:313: self.say_unwritten("landing claim not noted", &why);
UNCOVERED LINE crates/pull/src/landing_run.rs:318: fn say_unwritten(&self, what: &'static str, why: &std::io::Error) {
UNCOVERED LINE crates/pull/src/landing_run.rs:319: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/landing_run.rs:320: &telemetry::Event::warn(TARGET, what)
UNCOVERED LINE crates/pull/src/landing_run.rs:321: .with("lock", telemetry::Value::Str(&self.name))
UNCOVERED LINE crates/pull/src/landing_run.rs:322: .with("why", telemetry::Value::Str(&why.to_string())),
UNCOVERED LINE crates/pull/src/landing_run.rs:324: }
UNCOVERED LINE crates/pull/src/landing_run.rs:339: return;
UNCOVERED LINE crates/pull/src/landing_run.rs:355: self.say_unwritten("landing claim note not restored", &why);
UNCOVERED LINE crates/pull/src/landing_run.rs:394: claim.say_unwritten("landing claim not remembered", why);
UNCOVERED LINE crates/pull/src/landing_run.rs:596: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/landing_run.rs:597: &telemetry::Event::error(TARGET, "verdicts not spooled")
UNCOVERED LINE crates/pull/src/landing_run.rs:598: .with("run", telemetry::Value::Uint(self.run))
UNCOVERED LINE crates/pull/src/landing_run.rs:599: .with("instrument", telemetry::Value::Str(&window.key.to_string()))
UNCOVERED LINE crates/pull/src/landing_run.rs:600: .with("why", telemetry::Value::Str(why)),
UNCOVERED LINE crates/pull/src/landing_run.rs:601: );
UNCOVERED LINE crates/pull/src/landing_run.rs:689: Err(refusal) => {
UNCOVERED LINE crates/pull/src/landing_run.rs:690: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/landing_run.rs:691: &telemetry::Event::error(TARGET, "first day bar unread")
UNCOVERED LINE crates/pull/src/landing_run.rs:692: .with("series", telemetry::Value::Str(&series.key_text()))
UNCOVERED LINE crates/pull/src/landing_run.rs:693: .with("why", telemetry::Value::Str(&refusal.why)),
UNCOVERED LINE crates/pull/src/landing_run.rs:695: None
UNCOVERED LINE crates/pull/src/landing_run.rs:708: pub fn note_unhooked(asked: crate::session::Window) {
UNCOVERED LINE crates/pull/src/landing_run.rs:709: let _dropped_when_filtered = telemetry::emit(
UNCOVERED LINE crates/pull/src/landing_run.rs:710: &telemetry::Event::error(TARGET, "response not kept")
UNCOVERED LINE crates/pull/src/landing_run.rs:711: .with("from", telemetry::Value::Str(&asked.from().to_string()))
UNCOVERED LINE crates/pull/src/landing_run.rs:712: .with("to", telemetry::Value::Str(&asked.to().to_string()))
UNCOVERED LINE crates/pull/src/landing_run.rs:713: .with(
UNCOVERED LINE crates/pull/src/landing_run.rs:714: "why",
UNCOVERED LINE crates/pull/src/landing_run.rs:715: telemetry::Value::Str(
UNCOVERED LINE crates/pull/src/landing_run.rs:716: "the window reached its door with no landing run, key or answer carried \
UNCOVERED LINE crates/pull/src/landing_run.rs:717: beside it",
UNCOVERED LINE crates/pull/src/landing_run.rs:718: ),
UNCOVERED LINE crates/pull/src/landing_run.rs:719: ),
UNCOVERED LINE crates/pull/src/landing_run.rs:721: }
UNCOVERED LINE crates/pull/src/landing_run.rs:728: matches!(
UNCOVERED BRANCH crates/pull/src/landing_run.rs:307:20 False: if let Some(file) = self.file {
UNCOVERED BRANCH crates/pull/src/landing_run.rs:312:16 True: if let Err(why) = written {
UNCOVERED BRANCH crates/pull/src/landing_run.rs:338:13 False: let Some(file) = self.file else {
UNCOVERED BRANCH crates/pull/src/landing_run.rs:354:16 True: if let Err(why) = written {
UNCOVERED BRANCH crates/pull/src/landing_run.rs:393:12 True: if let Err(why) = &identity {
UNCOVERED BRANCH crates/pull/src/landing_run.rs:589:39 True: (Ok(outcome), Some(_)) if !body_in_pack(outcome) => (None, Some("body_not_in_pack")),
UNCOVERED BRANCH crates/pull/src/landing_run.rs:595:16 True: if let Some(Err(why)) = &spooled {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:62: Transport::LocalArchive(_) => panic!("Zerodha is an HTTP feed"),
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:387: mode.word()
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1149: ledger_text(&root, "repairs")
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1324: before.len()
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1676: notes[0]
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1866: hook.series_room()
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1969: fn ia10c_timings() {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1971: fn pct(mut v: Vec<std::time::Duration>) -> String {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1972: v.sort_unstable();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1973: let n = v.len();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1974: format!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1976: v[n / 2],
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1977: v[n * 99 / 100],
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1978: v[n - 1]
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1980: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1981: let base = std::env::var_os("BRUTEX_IA10C_TIMING_DIR")
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1982: .map_or_else(|| scratch("timings"), PathBuf::from);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1983: std::fs::create_dir_all(&base).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1984: let root = base.join(format!("ia10c-{}", std::process::id()));
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1985: std::fs::create_dir_all(&root).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1986: println!("timing under {}", root.display());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1987: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1988: for _ in 0..1_000 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1989: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1990: allocate(&root, Mode::Spot, By::Api, "zerodha").unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1991: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1992: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1993: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:1995: pct(took)
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2000: let big = base.join(format!("ia10c-claims-{}", std::process::id()));
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2001: std::fs::create_dir_all(big.join("audit")).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2002: let mut text = String::with_capacity(52 << 20);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2003: let mut id = CLOCK;
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2004: while text.len() < 50 << 20 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2005: let record = ledger::Entry {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2006: id,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2007: mode: Mode::Spot.word().to_owned(),
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2008: kind: kind_of(Vendor::Zerodha).to_owned(),
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2009: by: By::Api.word().to_owned(),
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2010: pid: 4242,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2011: started: CLOCK,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2012: resumes: 0,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2013: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2014: .record()
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2015: .unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2016: let body = std::str::from_utf8(&record[..114]).unwrap().trim_end();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2017: let crc = std::str::from_utf8(&record[114..126]).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2018: writeln!(text, "run {body} {crc}").unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2019: id += 1;
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2020: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2021: std::fs::write(big.join(ledger::CLAIMS), &text).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2022: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2023: let seeded = allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2024: let seed_took = began.elapsed();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2025: assert_eq!(seeded, id, "seeded past the last claim");
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2026: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2028: id - CLOCK,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2029: text.len()
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2031: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2032: for _ in 0..200 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2033: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2034: allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2035: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2036: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2037: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2039: pct(took)
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2041: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2042: for _ in 0..20 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2043: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2044: let largest = ledger::largest_claimed_run(&big).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2045: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2046: assert!(largest >= seeded);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2048: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2050: pct(took)
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2052: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2053: for n in 0..1_000 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2054: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2055: let held = claim(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2056: &root,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2057: Vendor::Zerodha,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2058: crate::gdfl_run::locks::Hold::Shared,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2059: &format!("api pid 1 run {n}"),
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2060: )
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2061: .unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2062: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2063: drop(held);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2064: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2065: println!("vendor landing claim, shared: {}", pct(took));
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2066: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2067: for n in 0..1_000 {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2068: let line = vec![format!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2069: "gap NSE-INDEX-ZZT{n} 2024-04-23 v=1 rung=1min owed=375 held=370 missing=5 first_missing=09:15 runs=1 longest=5 longest_at=09:15 why=vendor_
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2070: )];
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2071: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2072: crate::ingest::audit_ledger::append(&root, Vendor::Zerodha, "gaps", &line).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2073: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2074: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2075: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2077: pct(took)
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2079: let month = YearMonth::new(2024, 4).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2080: let days = regular_days(month);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2081: let hook = RunHook::new(&root, 77, Vendor::Zerodha, 1);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2082: let mut took = Vec::new();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2083: for n in 0..200_usize {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2084: let on = days[n % days.len()];
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2085: let window = Window::new(on, on).unwrap();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2086: let body = kite_body(&[on], &[]);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2087: let (rows, answered) = answered(&body);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2088: let landing = land(&root, &rows, window, Door::FromWindow, 77);
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2089: let key = key();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2090: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2091: let hooked = hook.after_landing(&Landed {
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2092: key: &key,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2093: granularity: crate::vendor::Granularity::Minute1,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2094: asked: window,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2095: answered: &answered,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2096: rows: &rows.rows,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2097: landing: &landing,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2098: door: Door::FromWindow,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2099: encoding: spec().timestamps,
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2100: });
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2101: took.push(began.elapsed());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2102: assert!(hooked.kept.is_ok());
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2104: println!(
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2106: pct(took)
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2108: }
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2227: panic!("{on} is not a session of record");
UNCOVERED LINE crates/pull/src/landing_run_tests.rs:2308: dir.display()
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:113:16 False: && session.bars() == 375
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:345:9 False: gdfl.windows(2).all(|pair| pair[0] < pair[1])
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:346:16 False: && zerodha.windows(2).all(|pair| pair[0] < pair[1]),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:394:26 False: .all(|entry| entry.mode == "import" && entry.by == "cli"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:590:25 False: .filter(|alert| alert.record.kind == 8 && alert.record.key == key)
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:675:21 False: grown.starts_with(&claims)
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:676:28 False: && grown.lines().count() == 3
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:677:28 False: && grown
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:903:13 False: said.contains(&format!("run {wider} is past {largest}"))
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:904:20 False: && said.contains("32-bit run field (offset 60)")
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:905:20 False: && said.contains("nothing is asked of the vendor"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1028:9 False: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1028:46 False: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1403:25 False: .filter(|f| f.instrument == "ZZIDX" && f.month == Some(month))
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1407:13 False: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1407:58 False: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1519:67 False: matches!(hooked.kept, Err(NotKept::RunTooWide { run }) if run == u64::from(u32::MAX) + 1)
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1674:9 False: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:1674:36 False: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2004:11 True: while text.len() < 50 << 20 {
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2004:11 False: while text.len() < 50 << 20 {
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2224:9 False: let crate::calendar::DayKind::Open(session) =
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2268:13 False: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2268:25 False: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2275:13 False: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
UNCOVERED BRANCH crates/pull/src/landing_run_tests.rs:2275:36 False: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
UNCOVERED LINE crates/pull/src/listing_tests.rs:146: panic!("a master span names its master");
UNCOVERED LINE crates/pull/src/listing_tests.rs:490: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {}
UNCOVERED LINE crates/pull/src/listing_tests.rs:491: Err(why) => panic!("an odd name: {why}"),
UNCOVERED BRANCH crates/pull/src/listing_tests.rs:145:9 False: let Source::NseSecurityMaster { file, sha256 } = span.source else {
UNCOVERED BRANCH crates/pull/src/manifest.rs:2175:20 False: if let Some(slot) = closes.get_mut(C_CONTRACT..C_CONTRACT + text.len()) {
UNCOVERED LINE crates/pull/src/manifest_ps1_tests.rs:108: String::from_utf8_lossy(&magic)
UNCOVERED LINE crates/pull/src/master_snapshots.rs:968: Err(why) => return refused(why),
UNCOVERED LINE crates/pull/src/master_snapshots.rs:2143: continue;
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2142:17 False: let Some(before) = old_rows.get(at) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2145:20 False: if let Some(slot) = matched.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2187:20 False: && let Some(before) = old_rows.get(at)
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2194:20 False: if let Some(slot) = matched.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2228:20 False: && let Some(before) = old_rows.get(at)
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2231:20 False: if let Some(slot) = matched.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/master_snapshots.rs:2486:20 False: if let Some(count) = counts.get_mut(one.change.index()) {
UNCOVERED LINE crates/pull/src/master_snapshots_cov_tests.rs:695: wrote[0]
UNCOVERED BRANCH crates/pull/src/master_snapshots_cov_tests.rs:380:9 False: words.contains(&"expiry") && words.contains(&"lot_size"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_cov_tests.rs:380:38 False: words.contains(&"expiry") && words.contains(&"lot_size"),
UNCOVERED LINE crates/pull/src/master_snapshots_first_listed.rs:713: }
UNCOVERED LINE crates/pull/src/master_snapshots_first_listed.rs:768: Rebuilt::StillDamaged(why) => Err(format!("futures_index_damaged: {why}")),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed.rs:711:16 False: if let Some(path) = path_of(dir, route.exchange, name) {
UNCOVERED LINE crates/pull/src/master_snapshots_first_listed_tests.rs:515: other @ Rebuilt::Done(..) => panic!("{other:?}"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:107:13 False: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:107:32 False: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:107:52 False: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:107:72 False: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:199:9 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:199:38 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:234:13 False: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:234:38 False: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:234:65 False: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:279:13 False: listed.refused[0]
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:281:20 False: && listed.refused[0].contains("recorded when the futures index took it in"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:380:9 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:380:38 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:448:13 False: said.starts_with("generation g1 of the futures index did not match its commit log")
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:449:20 False: && said.contains(why)
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:450:20 False: && said.contains("generation g2 was started"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:467:13 False: record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:467:64 False: record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:518:9 False: why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:518:40 False: why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:581:9 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED BRANCH crates/pull/src/master_snapshots_first_listed_tests.rs:581:38 False: listed.refused.is_empty() && listed.rebuilt.is_none(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:154: panic!("the first date resolves");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:161: panic!("the second date resolves");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:184: other => panic!("a reused token between snapshots is ambiguous, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:192: panic!("an agreeing token resolves");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:198: panic!("after the newest snapshot the newest answers");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:211: other => panic!("before the first snapshot nothing resolves, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:284: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:305: panic!("the receipted refresh answers");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:407: fn measure_the_hook_with_incomplete_entries_held() {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:409: let body = dump(&[fut(9_000_051, "ZQIDX26JUNFUT", "2026-06-30", 75)]);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:410: for held in [0_usize, INCOMPLETE_NAMED, 1_000] {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:411: let dir = scratch(&format!("measure-{held}"));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:412: let root = dir.join(SNAPSHOT_DIR);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:413: fs::create_dir_all(&root).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:414: let first = day(2020, 1, 1).days_from_epoch();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:415: for n in 0..held {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:416: let on = Day::from_days(first + u32::try_from(n).unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:417: fs::write(root.join(file_name(Kind::Kite, stamp(on, 1))), b"cut").unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:418: }
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:419: let start = day(2026, 1, 1).days_from_epoch();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:420: let mut costs: Vec<u128> = (0..N)
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:421: .map(|n| {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:422: let on = Day::from_days(start + u32::try_from(n).unwrap()).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:423: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:424: let outcome = after_land(&dir, kite_source(), &body, &written(), Ok(on)).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:425: let json = outcome.json();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:426: let cost = began.elapsed().as_micros();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:427: assert_eq!(outcome.incomplete.as_ref().map(Vec::len), Ok(held));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:428: assert!(json.contains(&format!(r#""held":{held},"#)), "{json}");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:429: cost
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:430: })
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:431: .collect();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:432: costs.sort_unstable();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:433: println!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:435: costs[N / 2],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:436: costs[N * 99 / 100 - 1],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:437: costs[N - 1]
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:440: }
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:761: k1.json()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:847: other => panic!("a torn snapshot is unreadable, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:924: panic!("one row is found");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:944: panic!("found");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:996: other => panic!("resolves, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1013: other => panic!("expired, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1051: other => panic!("{on}: expired, got {other:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1057: matches!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1065: matches!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1225: outcome.json()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1241: fn a_real_size_dump_is_measured() {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1244: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1245: panic!("set BRUTEX_KITE_DUMP to a copy of a landed Kite dump");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1247: let body = fs::read_to_string(&path).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1248: let root = scratch("real-size");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1249: let (d1, d2, between) = (day(2026, 10, 15), day(2026, 10, 22), day(2026, 10, 18));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1250: let started = Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1251: snapshot(&root, Kind::Kite, d1, URL, body.as_bytes()).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1252: let write_ms = started.elapsed().as_millis();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1253: let changed = body.replacen(",75,FUT,", ",65,FUT,", 3);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1254: snapshot(&root, Kind::Kite, d2, URL, changed.as_bytes()).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1255: let started = Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1256: let parsed = Snapshot::parse(stamp(d1, 1), &body).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1257: let parse_ms = started.elapsed().as_millis();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1258: let started = Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1259: let summary = diff_for(&root, stamp(d2, 1)).unwrap().unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1260: let diff_ms = started.elapsed().as_millis();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1261: let book = Book::open(&root).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1262: let step = (parsed.len() / 200).max(1);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1263: let tokens: Vec<(String, u64)> = parsed
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1264: .rows()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1265: .step_by(step)
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1266: .take(200)
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1267: .map(|listed| (listed.exchange().to_owned(), listed.token()))
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1268: .collect();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1269: let started = Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1270: let first = book.token_on(between, &tokens[0].0, tokens[0].1).word();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1271: let cold_ms = started.elapsed().as_millis();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1272: let mut ns: Vec<u128> = tokens
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1273: .iter()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1274: .map(|(exchange, token)| {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1275: let started = Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1276: black_box(book.token_on(black_box(between), exchange, *token));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1277: started.elapsed().as_nanos()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1278: })
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1279: .collect();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1280: ns.sort_unstable();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1281: let n = ns.len();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1282: let compressed = fs::metadata(root.join(file_name(Kind::Kite, stamp(d1, 1))))
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1283: .unwrap()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1284: .len();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1285: println!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1289: parsed.len(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1290: body.len(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1291: summary.counts_line(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1292: ns[n / 2],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1293: ns[n * 99 / 100],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1294: ns[n - 1],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1296: assert_eq!(summary.counts[4], 3, "the three injected lot changes");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1297: }
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1341: panic!("the day's newest snapshot agrees, so the older one is believed");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1359: answer => panic!("the day's newest disagrees, so ambiguous; got {answer:?}"),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1390: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1437: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1472: panic!("a diff against the day before: {:?}", outcome.diff);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1516: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1623: book.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1637: book.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1672: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1736: catalog.incomplete()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1959: panic!("read in any column order");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:1970: panic!("found");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2002: panic!("the row beside it answers");
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2047: fn measure_the_catalog_listing_and_its_probes() {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2049: let first = day(2020, 1, 1).days_from_epoch();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2050: for held in [1_u32, 100, 1_000] {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2051: let root = scratch(&format!("measure-catalog-{held}"));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2052: for n in 0..held {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2053: let on = Day::from_days(first + n / 2).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2054: let s = stamp(on, 1 + n % 2);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2055: fs::write(root.join(file_name(Kind::Kite, s)), b"").unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2056: fs::write(root.join(receipt_name(Kind::Kite, s)), b"").unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2057: }
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2058: let newest = stamp(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2059: Day::from_days(first + (held - 1) / 2).unwrap(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2060: 1 + (held - 1) % 2,
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2062: let (mut listing, mut probes): (Vec<u128>, Vec<u128>) = (0..N)
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2063: .map(|_| {
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2064: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2065: let catalog = Catalog::read(&root).unwrap();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2066: let listed = began.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2067: let began = std::time::Instant::now();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2068: let answers = (
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2069: std::hint::black_box(catalog.of_day(Kind::Kite, newest.day)),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2070: std::hint::black_box(catalog.before(Kind::Kite, newest)),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2071: std::hint::black_box(catalog.is_receipted(Kind::Kite, newest)),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2072: );
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2073: let probed = began.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2074: assert_eq!(answers.0, Some(newest));
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2075: assert_eq!(answers.1.is_some(), held > 1);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2076: assert!(answers.2);
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2077: assert_eq!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2078: catalog.receipted(Kind::Kite).len(),
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2079: usize::try_from(held).unwrap()
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2081: (listed, probed)
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2082: })
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2083: .unzip();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2084: listing.sort_unstable();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2085: probes.sort_unstable();
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2086: println!(
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2089: held * 2,
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2090: listing[N / 2] / 1_000,
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2091: listing[N * 99 / 100 - 1] / 1_000,
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2092: listing[N - 1] / 1_000,
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2093: probes[N / 2],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2094: probes[N * 99 / 100 - 1],
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2095: probes[N - 1]
UNCOVERED LINE crates/pull/src/master_snapshots_tests.rs:2098: }
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:149:9 False: let TokenOn::Resolved {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:156:9 False: let TokenOn::Resolved {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:191:9 False: let TokenOn::Resolved { snapshot, listed } = book.token_on(between, "NFO", 9_000_002) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:197:9 False: let TokenOn::Resolved { listed, .. } = book.token_on(day(2026, 2, 20), "NFO", 9_000_001) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:304:9 False: let TokenOn::Resolved { listed, .. } = book.token_on(d1, "NFO", 9_000_021) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:362:9 False: json.contains(r#""incomplete":{"held":3,"named":[{"file":"instruments_20260304.csv.gz","stamp":"2026-03-04","why":"a body with no receipt (a
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:363:16 False: && json.contains(r#""unnamed":0,"refusal":null}"#),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:369:9 False: why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:369:52 False: why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:466:9 False: json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:466:57 False: json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:823:9 False: why.contains("after today 2026-02-01")
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:824:16 False: && why.contains(&file_name(Kind::Kite, stamp(d2, 1)))
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:825:16 False: && why.contains("refused rather than believed fresh"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:886:9 False: garbage.contains("line 2") && garbage.contains("instrument_token"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:886:39 False: garbage.contains("line 2") && garbage.contains("instrument_token"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:923:9 False: let Lookup::Listed(listed) = one.lookup("NFO", 9_800_001) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:943:9 False: let Lookup::Listed(again) = other.lookup("NFO", 9_800_001) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1219:9 False: why.contains("no snapshot was written") && why.contains("1970"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1219:52 False: why.contains("no snapshot was written") && why.contains("1970"),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1244:9 True: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1244:9 False: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1336:9 False: let TokenOn::Resolved {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1387:16 False: && one.to_string().starts_with(&partial)
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1471:9 False: let Some(Ok(Some(ref summary))) = outcome.diff else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1511:47 False: catalog.incomplete().iter().any(|one| one.file == cut
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1512:16 False: && one.kind == Kind::Kite
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1513:16 False: && one.stamp == b
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1635:24 False: .any(|one| one.stamp == cut && one.why.contains("no receipt")),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1670:24 False: .any(|one| one.file == twin && one.why.contains("another case")),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1734:24 False: .any(|one| one.file == upper && one.why == CUT_DIFF),
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1958:9 False: let Lookup::Listed(listed) = one.lookup("NFO", 9_800_101) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:1969:9 False: let Lookup::Listed(first) = parsed.lookup("NFO", 9_800_102) else {
UNCOVERED BRANCH crates/pull/src/master_snapshots_tests.rs:2001:9 False: let Lookup::Listed(other) = parsed.lookup("NFO", 9_900_102) else {
UNCOVERED LINE crates/pull/src/masters.rs:797: }
UNCOVERED LINE crates/pull/src/masters.rs:834: return Err(Refusal::answered(
UNCOVERED LINE crates/pull/src/masters.rs:835: status.as_u16(),
UNCOVERED LINE crates/pull/src/masters.rs:836: format!("{url} sent more than the {MAX_BODY_BYTES}-byte ceiling"),
UNCOVERED LINE crates/pull/src/masters.rs:837: ));
UNCOVERED LINE crates/pull/src/masters.rs:963: Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => failure,
UNCOVERED LINE crates/pull/src/masters.rs:964: Err(cleanup) => format!(
UNCOVERED LINE crates/pull/src/masters.rs:966: temporary.display()
UNCOVERED LINE crates/pull/src/masters.rs:986: drop(file);
UNCOVERED LINE crates/pull/src/masters.rs:987: return Err(remove_partial_after(
UNCOVERED LINE crates/pull/src/masters.rs:988: &temporary,
UNCOVERED LINE crates/pull/src/masters.rs:989: format!("{} could not be written — {why}", temporary.display()),
UNCOVERED LINE crates/pull/src/masters.rs:990: ));
UNCOVERED LINE crates/pull/src/masters.rs:993: drop(file);
UNCOVERED LINE crates/pull/src/masters.rs:994: return Err(remove_partial_after(
UNCOVERED LINE crates/pull/src/masters.rs:995: &temporary,
UNCOVERED LINE crates/pull/src/masters.rs:996: format!("{} could not be synced — {why}", temporary.display()),
UNCOVERED LINE crates/pull/src/masters.rs:997: ));
UNCOVERED LINE crates/pull/src/masters.rs:1002: return Err(remove_partial_after(
UNCOVERED LINE crates/pull/src/masters.rs:1003: &temporary,
UNCOVERED LINE crates/pull/src/masters.rs:1004: format!(
UNCOVERED LINE crates/pull/src/masters.rs:1005: "{} could not replace {} — {why}",
UNCOVERED LINE crates/pull/src/masters.rs:1006: temporary.display(),
UNCOVERED LINE crates/pull/src/masters.rs:1007: target.display()
UNCOVERED LINE crates/pull/src/masters.rs:1008: ),
UNCOVERED LINE crates/pull/src/masters.rs:1009: ));
UNCOVERED LINE crates/pull/src/masters.rs:1021: Err(why) => Ok(Err(format!(
UNCOVERED LINE crates/pull/src/masters.rs:1022: "{} replaced {}, but the containing directory could not be synced — {why}; \
UNCOVERED LINE crates/pull/src/masters.rs:1023: the new bytes are visible, but crash durability is UNVERIFIED",
UNCOVERED LINE crates/pull/src/masters.rs:1024: temporary.display(),
UNCOVERED LINE crates/pull/src/masters.rs:1025: target.display()
UNCOVERED LINE crates/pull/src/masters.rs:1026: ))),
UNCOVERED LINE crates/pull/src/masters.rs:1273: continue;
UNCOVERED LINE crates/pull/src/masters.rs:1347: return Landed::Refused(format!(
UNCOVERED LINE crates/pull/src/masters.rs:1348: "the masters directory {} cannot be created — {why}",
UNCOVERED LINE crates/pull/src/masters.rs:1349: dir.display()
UNCOVERED LINE crates/pull/src/masters.rs:1350: ));
UNCOVERED LINE crates/pull/src/masters.rs:1359: Err(why) => return Landed::Refused(why),
UNCOVERED LINE crates/pull/src/masters.rs:1396: Ok(Err(why)) => Landed::Uncertain {
UNCOVERED LINE crates/pull/src/masters.rs:1397: bytes: body.len(),
UNCOVERED LINE crates/pull/src/masters.rs:1398: changed,
UNCOVERED LINE crates/pull/src/masters.rs:1399: why,
UNCOVERED LINE crates/pull/src/masters.rs:1400: },
UNCOVERED LINE crates/pull/src/masters.rs:1401: Err(why) => Landed::Refused(why),
UNCOVERED LINE crates/pull/src/masters.rs:1424: return Ok(false);
UNCOVERED LINE crates/pull/src/masters.rs:1438: return Ok(false);
UNCOVERED LINE crates/pull/src/masters.rs:1441: return Ok(false);
UNCOVERED LINE crates/pull/src/masters.rs:1821: return;
UNCOVERED LINE crates/pull/src/masters.rs:1882: return an_index_csv();
UNCOVERED LINE crates/pull/src/masters.rs:1929: fn an_index_csv() -> String {
UNCOVERED LINE crates/pull/src/masters.rs:1931: let mut body = String::from("index_name,category\n");
UNCOVERED LINE crates/pull/src/masters.rs:1932: let mut n = 0;
UNCOVERED LINE crates/pull/src/masters.rs:1933: while body.len() <= MIN_BODY_BYTES {
UNCOVERED LINE crates/pull/src/masters.rs:1934: let _ = writeln!(body, "NIFTY TEST {n},Broad Market Indices");
UNCOVERED LINE crates/pull/src/masters.rs:1935: n += 1;
UNCOVERED LINE crates/pull/src/masters.rs:1936: }
UNCOVERED LINE crates/pull/src/masters.rs:1937: body
UNCOVERED LINE crates/pull/src/masters.rs:1938: }
UNCOVERED LINE crates/pull/src/masters.rs:1973: why.chars().take(200).collect::<String>()
UNCOVERED LINE crates/pull/src/masters.rs:1981: why.chars().take(200).collect::<String>()
UNCOVERED LINE crates/pull/src/masters.rs:2059: panic!("{name} must be refused");
UNCOVERED LINE crates/pull/src/masters.rs:2122: panic!("a short body must be refused");
UNCOVERED LINE crates/pull/src/masters.rs:2141: panic!("a long HTML page is not a CSV");
UNCOVERED LINE crates/pull/src/masters.rs:2173: panic!("JSON is not the CSV a master is");
UNCOVERED LINE crates/pull/src/masters.rs:2295: panic!("the first write lands");
UNCOVERED LINE crates/pull/src/masters.rs:2301: panic!("the second write lands");
UNCOVERED LINE crates/pull/src/masters.rs:2309: panic!("the third write lands");
UNCOVERED LINE crates/pull/src/masters.rs:2345: panic!("the identical landing is still written");
UNCOVERED LINE crates/pull/src/masters.rs:2366: panic!("the repair lands");
UNCOVERED LINE crates/pull/src/masters.rs:2376: panic!("the repair lands");
UNCOVERED LINE crates/pull/src/masters.rs:2404: panic!("an unreadable data row must refuse");
UNCOVERED LINE crates/pull/src/masters.rs:2543: panic!("a malformed row refuses");
UNCOVERED LINE crates/pull/src/masters.rs:2823: return Err(Refusal::transport("the script ran out".to_owned()));
UNCOVERED LINE crates/pull/src/masters.rs:3159: matches!(
UNCOVERED LINE crates/pull/src/masters.rs:3189: other => panic!("expected a refusal, got {other:?}"),
UNCOVERED LINE crates/pull/src/masters.rs:3397: unreachable!("an unconvertible body cannot be written")
UNCOVERED LINE crates/pull/src/masters.rs:3423: unreachable!("a body with no columns cannot be a master")
UNCOVERED LINE crates/pull/src/masters.rs:3565: unreachable!("a master the reader cannot read must not be written")
UNCOVERED LINE crates/pull/src/masters.rs:3627: let _ = std::fs::OpenOptions::new().write(true).open(&target);
UNCOVERED LINE crates/pull/src/masters.rs:3632: panic!("a target that is not a regular file is refused: {landed:?}");
UNCOVERED BRANCH crates/pull/src/masters.rs:478:41 False: if skipped.categories.is_empty() && skipped.elements == 0 {
UNCOVERED BRANCH crates/pull/src/masters.rs:789:16 False: if let Some(ref host) = host {
UNCOVERED BRANCH crates/pull/src/masters.rs:833:16 True: if held.len().saturating_add(bytes.len()) > MAX_BODY_BYTES {
UNCOVERED BRANCH crates/pull/src/masters.rs:963:25 True: Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => failure,
UNCOVERED BRANCH crates/pull/src/masters.rs:963:25 False: Err(cleanup) if cleanup.kind() == std::io::ErrorKind::NotFound => failure,
UNCOVERED BRANCH crates/pull/src/masters.rs:985:12 True: if let Err(why) = file.write_all(body.as_bytes()) {
UNCOVERED BRANCH crates/pull/src/masters.rs:992:12 True: if let Err(why) = file.sync_all() {
UNCOVERED BRANCH crates/pull/src/masters.rs:1001:12 True: if let Err(why) = std::fs::rename(&temporary, &target) {
UNCOVERED BRANCH crates/pull/src/masters.rs:1188:32 True: if opener == Some(b'{') || opener == Some(b'[') || opener == Some(b'<') {
UNCOVERED BRANCH crates/pull/src/masters.rs:1272:12 True: if line.is_empty() {
UNCOVERED BRANCH crates/pull/src/masters.rs:1346:12 True: if let Err(why) = std::fs::create_dir_all(dir) {
UNCOVERED BRANCH crates/pull/src/masters.rs:1384:20 False: if !changed && refresh_mtime(&target).is_ok() {
UNCOVERED BRANCH crates/pull/src/masters.rs:1423:9 False: let Ok(meta) = file.metadata() else {
UNCOVERED BRANCH crates/pull/src/masters.rs:1437:13 False: let Some(window) = chunk.get_mut(..expected.len()) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:1440:12 True: if reader.read_exact(window).is_err() {
UNCOVERED BRANCH crates/pull/src/masters.rs:1820:9 False: let Some(prime) = source.prime else {
UNCOVERED BRANCH crates/pull/src/masters.rs:1881:13 False: let Some(vendor) = source.vendor else {
UNCOVERED BRANCH crates/pull/src/masters.rs:1933:15 True: while body.len() <= MIN_BODY_BYTES {
UNCOVERED BRANCH crates/pull/src/masters.rs:1933:15 False: while body.len() <= MIN_BODY_BYTES {
UNCOVERED BRANCH crates/pull/src/masters.rs:1954:56 False: source.file == NSE_INDICES_FILE || source.file == NSE_TOTAL_MARKET_FILE,
UNCOVERED BRANCH crates/pull/src/masters.rs:1979:13 False: !why.contains("ceiling") && why.contains("no comma"),
UNCOVERED BRANCH crates/pull/src/masters.rs:1979:41 False: !why.contains("ceiling") && why.contains("no comma"),
UNCOVERED BRANCH crates/pull/src/masters.rs:2058:17 False: let Landed::Refused(why) = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2121:13 False: let Landed::Refused(why) = outcome else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2140:13 False: let Landed::Refused(why) = land(&dir, &SOURCES[0], &html) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2172:13 False: let Landed::Refused(why) = land(&dir, &SOURCES[0], &json) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2294:13 False: let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2300:13 False: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2308:13 False: let Landed::Written { changed, .. } = land(&dir, source, &moved) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2344:13 False: let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2361:16 False: if let Some(last) = same_length.iter_mut().rev().nth(1) {
UNCOVERED BRANCH crates/pull/src/masters.rs:2365:13 False: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2375:13 False: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2403:17 False: let Landed::Refused(why) = land(&dir, source, &candidate) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2542:13 False: let Landed::Refused(why) = land(&dir, source, &bad) else {
UNCOVERED BRANCH crates/pull/src/masters.rs:2819:16 True: if self.answers.borrow().is_empty() {
UNCOVERED BRANCH crates/pull/src/masters.rs:3396:13 False: let Landed::Refused(ref why) = landed else {
UNCOVERED BRANCH crates/pull/src/masters.rs:3422:13 False: let Landed::Refused(ref why) = landed else {
UNCOVERED BRANCH crates/pull/src/masters.rs:3564:13 False: let Landed::Refused(ref why) = landed else {
UNCOVERED BRANCH crates/pull/src/masters.rs:3626:12 True: if landed.is_err() {
UNCOVERED BRANCH crates/pull/src/masters.rs:3631:13 False: let Landed::Refused(ref why) = landed else {
UNCOVERED LINE crates/pull/src/nse.rs:400: return out;
UNCOVERED LINE crates/pull/src/nse.rs:443: break;
UNCOVERED LINE crates/pull/src/nse.rs:585: break;
UNCOVERED LINE crates/pull/src/nse.rs:885: panic!("an interstitial must refuse as NotCsv, got {why:?}");
UNCOVERED LINE crates/pull/src/nse.rs:1258: panic!("got {why:?}");
UNCOVERED BRANCH crates/pull/src/nse.rs:396:16 False: if let Some(head) = rest.get(..at) {
UNCOVERED BRANCH crates/pull/src/nse.rs:399:13 False: let Some(after) = rest.get(at.saturating_add(4)..) else {
UNCOVERED BRANCH crates/pull/src/nse.rs:442:13 False: let Some(after) = rest.get(at.saturating_add(5)..) else {
UNCOVERED BRANCH crates/pull/src/nse.rs:457:13 False: let Some(value) = value else { break };
UNCOVERED BRANCH crates/pull/src/nse.rs:584:13 False: let Some(after) = rest.get(at.saturating_add(5)..) else {
UNCOVERED BRANCH crates/pull/src/nse.rs:595:13 False: let Some(value) = value else { break };
UNCOVERED BRANCH crates/pull/src/nse.rs:824:24 False: if let Some(field) = line.get(start..at) {
UNCOVERED BRANCH crates/pull/src/nse.rs:833:12 False: if let Some(field) = line.get(start..) {
UNCOVERED BRANCH crates/pull/src/nse.rs:884:13 False: let NseError::NotCsv { ref head } = why else {
UNCOVERED BRANCH crates/pull/src/nse.rs:1018:24 False: if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES
UNCOVERED BRANCH crates/pull/src/nse.rs:1018:59 False: if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES
UNCOVERED BRANCH crates/pull/src/nse.rs:1026:51 False: NseError::HeaderUnexpected { got } if got.len() == MAX_DOCUMENT_BYTES
UNCOVERED BRANCH crates/pull/src/nse.rs:1217:72 False: matches!(refused, NseError::TooManyIndexLinks { links } if links > MAX_INDEX_LINKS),
UNCOVERED BRANCH crates/pull/src/nse.rs:1257:13 False: let NseError::NoConstituentLink { ref page } = why else {
UNCOVERED LINE crates/pull/src/nse_ps1_tests.rs:34: panic!("an open value is no link");
UNCOVERED LINE crates/pull/src/nse_ps1_tests.rs:65: panic!("the bound is TooManyRows");
UNCOVERED BRANCH crates/pull/src/nse_ps1_tests.rs:31:9 False: let NseError::NoConstituentLink { page } =
UNCOVERED BRANCH crates/pull/src/nse_ps1_tests.rs:62:9 False: let NseError::TooManyRows { rows, cap } =
UNCOVERED LINE crates/pull/src/padded.rs:108: return Vec::new();
UNCOVERED BRANCH crates/pull/src/padded.rs:107:9 False: let DayKind::Open(s) = session else {
UNCOVERED LINE crates/pull/src/padded_tests.rs:305: panic!("2024-03-02 is a recorded special session: {session:?}");
UNCOVERED BRANCH crates/pull/src/padded_tests.rs:304:9 False: let DayKind::Open(s) = session else {
UNCOVERED BRANCH crates/pull/src/padded_tests.rs:488:35 False: if (m - OPEN) % 7 == 0 && m + 3 < OPEN + 375 {
UNCOVERED LINE crates/pull/src/pricing.rs:598: return Err(PricingError::BasisMismatch {
UNCOVERED LINE crates/pull/src/pricing.rs:599: rate: rate.basis(),
UNCOVERED LINE crates/pull/src/pricing.rs:600: asked: basis,
UNCOVERED LINE crates/pull/src/pricing.rs:601: });
UNCOVERED LINE crates/pull/src/pricing.rs:1421: matches!(
UNCOVERED LINE crates/pull/src/pricing.rs:1434: matches!(
UNCOVERED LINE crates/pull/src/pricing.rs:1616: matches!(
UNCOVERED LINE crates/pull/src/pricing.rs:1873: matches!(
UNCOVERED BRANCH crates/pull/src/pricing.rs:597:8 True: if rate.basis() != basis {
UNCOVERED BRANCH crates/pull/src/pricing.rs:1178:13 False: solved.volatility > 0.0 && solved.volatility < 5.0,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1178:40 False: solved.volatility > 0.0 && solved.volatility < 5.0,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1310:13 False: at_zero > at_twelve && at_five_five > at_eight,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1310:36 False: at_zero > at_twelve && at_five_five > at_eight,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1361:64 False: Err(PricingError::RateNotFinite { annual }) if annual.is_nan()
UNCOVERED BRANCH crates/pull/src/pricing.rs:1399:79 False: matches!(error, PricingError::NotPositive { field: f, .. } if f == field),
UNCOVERED BRANCH crates/pull/src/pricing.rs:1501:17 False: assert!(row.volatility > 0.0 && row.volatility < 5.0);
UNCOVERED BRANCH crates/pull/src/pricing.rs:1501:41 False: assert!(row.volatility > 0.0 && row.volatility < 5.0);
UNCOVERED BRANCH crates/pull/src/pricing.rs:1833:45 False: OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
UNCOVERED BRANCH crates/pull/src/pricing.rs:1833:64 False: OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
UNCOVERED BRANCH crates/pull/src/pricing.rs:1834:44 False: OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
UNCOVERED BRANCH crates/pull/src/pricing.rs:1834:63 False: OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
UNCOVERED BRANCH crates/pull/src/pricing.rs:1836:21 False: assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
UNCOVERED BRANCH crates/pull/src/pricing.rs:1836:50 False: assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
UNCOVERED BRANCH crates/pull/src/pricing.rs:1889:13 False: price.to_bits() == 100_f64.to_bits() && floor > price,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1889:53 False: price.to_bits() == 100_f64.to_bits() && floor > price,
UNCOVERED BRANCH crates/pull/src/pricing.rs:1928:77 False: matches!(why, PricingError::NotPositive { field: f, .. } if f == field),
UNCOVERED LINE crates/pull/src/pricing_ps1_tests.rs:59: panic!("a day the ladder does not cover is LadderUnknown");
UNCOVERED LINE crates/pull/src/pricing_ps1_tests.rs:72: panic!("a spot on rung zero is LadderUnknown");
UNCOVERED BRANCH crates/pull/src/pricing_ps1_tests.rs:56:9 False: let PricingError::LadderUnknown { why } =
UNCOVERED BRANCH crates/pull/src/pricing_ps1_tests.rs:69:9 False: let PricingError::LadderUnknown { why: rung } =
UNCOVERED LINE crates/pull/src/reference_data.rs:2519: table,
UNCOVERED LINE crates/pull/src/reference_data.rs:2520: path: dir.clone(),
UNCOVERED LINE crates/pull/src/reference_data.rs:2521: generation: None,
UNCOVERED LINE crates/pull/src/reference_data.rs:2522: kind: RefusalKind::Row,
UNCOVERED LINE crates/pull/src/reference_data.rs:2523: detail: format!("the compiled table cannot be rendered: {why}"),
UNCOVERED LINE crates/pull/src/reference_data.rs:2524: })?;
UNCOVERED LINE crates/pull/src/reference_data.rs:2526: table,
UNCOVERED LINE crates/pull/src/reference_data.rs:2527: path: dir.clone(),
UNCOVERED LINE crates/pull/src/reference_data.rs:2528: generation: None,
UNCOVERED LINE crates/pull/src/reference_data.rs:2529: kind: RefusalKind::Row,
UNCOVERED LINE crates/pull/src/reference_data.rs:2530: detail: format!("the compiled table cannot be rendered: {why}"),
UNCOVERED LINE crates/pull/src/reference_data.rs:2531: })?;
UNCOVERED LINE crates/pull/src/reference_data.rs:3212: return Err(format!("day {day} inside the record is unmeasured"));
UNCOVERED BRANCH crates/pull/src/reference_data_cov_tests.rs:391:9 False: current.contains(&"ZQA") && current.contains(&"ZQNOW"),
UNCOVERED BRANCH crates/pull/src/reference_data_cov_tests.rs:391:37 False: current.contains(&"ZQA") && current.contains(&"ZQNOW"),
UNCOVERED LINE crates/pull/src/reference_data_ps1_tests.rs:626: panic!("{outcome:?}");
UNCOVERED LINE crates/pull/src/reference_data_ps1_tests.rs:652: panic!("a draft with no decision was written");
UNCOVERED LINE crates/pull/src/reference_data_ps1_tests.rs:707: panic!("{outcome:?}");
UNCOVERED LINE crates/pull/src/reference_data_ps1_tests.rs:761: panic!("another fault is Io");
UNCOVERED BRANCH crates/pull/src/reference_data_ps1_tests.rs:625:9 False: let Err(WriteError::Refused(refused)) = &outcome else {
UNCOVERED BRANCH crates/pull/src/reference_data_ps1_tests.rs:651:9 False: let Err(WriteError::Draft(why)) = write(&store, &unrendered) else {
UNCOVERED BRANCH crates/pull/src/reference_data_ps1_tests.rs:706:9 False: let Err(WriteError::Io(said)) = &outcome else {
UNCOVERED BRANCH crates/pull/src/reference_data_ps1_tests.rs:760:9 False: let WriteError::Io(why) = write_linking(&store, &fno, &gone).unwrap_err() else {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:428: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:481: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:592: mine[0]
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:774: other => panic!("a span past MAX_ROWS days must be refused: {other:?}"),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:881: Err(other) => panic!("{other}"),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1042: fn replay_a_real_rate_table_under_the_gap_rule() {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1043: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1044: panic!("BRUTEX_R330_RATES names no file");
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1046: let text = std::fs::read_to_string(&path).unwrap();
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1047: let draft = rate_draft(&text, &path, day(2026, 10, 20)).unwrap();
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1048: let store = scratch("ref-real-rates");
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1049: write(&store, &draft).unwrap();
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1050: let loaded = load(&store);
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1051: let series = loaded.rates.table().unwrap();
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1052: let first = i64::from(series.first_release().ordinal()) + 1;
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1053: let last = (i64::from(series.last_release().ordinal())
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1054: + i64::from(crate::riskfree::MAX_AGE_DAYS))
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1055: .min(crate::calendar::SOURCED_LAST_DAY);
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1056: let (mut sessions, mut stale, mut carried) = (0_u32, Vec::new(), Vec::new());
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1057: for at in first.max(crate::calendar::SOURCED_FIRST_DAY)..=last {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1058: if !matches!(
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1059: crate::calendar::sourced_kind_of(at),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1062: continue;
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1063: }
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1064: sessions += 1;
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1065: let civil = day_of(at).unwrap();
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1066: match series.on(crate::pricing::trade_day_of(civil).unwrap()) {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1067: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1068: Ok(_) => {}
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1069: Err(crate::riskfree::LookupError::Stale { .. }) => stale.push(civil.to_string()),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1070: Err(crate::riskfree::LookupError::HoleBeforeGap { gap, .. }) => {
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1071: stale.push(format!(
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1072: "{civil} (gap {} follows an unexplained hole)",
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1073: gap.released
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1074: ));
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1075: }
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1076: Err(why) => panic!("{why}"),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1079: eprintln!(
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1082: series.auction_count(),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1083: series.gap_count(),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1084: stale.len(),
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1085: carried.len()
UNCOVERED LINE crates/pull/src/reference_data_tests.rs:1087: }
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:144:9 False: said.contains("index_identity") && said.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:144:44 False: said.contains("index_identity") && said.contains("checksum"),
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:197:21 False: assert!(receipt.is_some() && refused.is_none());
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:197:42 False: assert!(receipt.is_some() && refused.is_none());
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:523:13 False: assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:523:47 False: assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1043:9 True: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1043:9 False: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1058:12 True: if !matches!(
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1058:12 False: if !matches!(
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1067:26 True: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1067:26 False: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1345:9 False: said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
UNCOVERED BRANCH crates/pull/src/reference_data_tests.rs:1345:70 False: said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
UNCOVERED LINE crates/pull/src/rehearsal.rs:917: Err(why) => Err(Refusal::ListUnreadable {
UNCOVERED LINE crates/pull/src/rehearsal.rs:918: kind: kind.to_owned(),
UNCOVERED LINE crates/pull/src/rehearsal.rs:919: why: why.to_owned(),
UNCOVERED LINE crates/pull/src/rehearsal.rs:920: }),
UNCOVERED BRANCH crates/pull/src/rehearsal.rs:922:12 False: if let Err(refusal) = &answer {
UNCOVERED LINE crates/pull/src/rehearsal_tests.rs:277: why.len()
UNCOVERED BRANCH crates/pull/src/rehearsal_tests.rs:275:9 False: why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
UNCOVERED BRANCH crates/pull/src/rehearsal_tests.rs:275:39 False: why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
UNCOVERED BRANCH crates/pull/src/rehearsal_tests.rs:498:13 False: why.contains("S-90: operator_yes") && why.contains(says),
UNCOVERED BRANCH crates/pull/src/rehearsal_tests.rs:498:51 False: why.contains("S-90: operator_yes") && why.contains(says),
UNCOVERED LINE crates/pull/src/repair_plan.rs:161: self
UNCOVERED LINE crates/pull/src/repair_plan.rs:229: DayKind::OpenLengthUnmeasured => return Owes::Unsized,
UNCOVERED LINE crates/pull/src/repair_plan.rs:269: return None;
UNCOVERED LINE crates/pull/src/repair_plan.rs:472: break;
UNCOVERED LINE crates/pull/src/repair_plan.rs:495: return Vec::new();
UNCOVERED LINE crates/pull/src/repair_plan.rs:546: Err(why) => {
UNCOVERED LINE crates/pull/src/repair_plan.rs:547: out.refused = Some(why.to_string());
UNCOVERED LINE crates/pull/src/repair_plan.rs:548: return out;
UNCOVERED LINE crates/pull/src/repair_plan.rs:563: Err(why) => {
UNCOVERED LINE crates/pull/src/repair_plan.rs:564: out.refused = Some(why.to_string());
UNCOVERED LINE crates/pull/src/repair_plan.rs:565: return out;
UNCOVERED LINE crates/pull/src/repair_plan.rs:571: continue;
UNCOVERED LINE crates/pull/src/repair_plan.rs:576: out.unattested.push(day);
UNCOVERED LINE crates/pull/src/repair_plan.rs:577: continue;
UNCOVERED LINE crates/pull/src/repair_plan.rs:591: Err(fault) => {
UNCOVERED LINE crates/pull/src/repair_plan.rs:592: out.refused = Some(format!("{day}: {fault}"));
UNCOVERED LINE crates/pull/src/repair_plan.rs:593: return out;
UNCOVERED LINE crates/pull/src/repair_plan.rs:610: Err(why) => {
UNCOVERED LINE crates/pull/src/repair_plan.rs:611: out.refused = Some(format!("{day}: {why}"));
UNCOVERED LINE crates/pull/src/repair_plan.rs:612: return out;
UNCOVERED LINE crates/pull/src/repair_plan.rs:629: Err(why) => {
UNCOVERED LINE crates/pull/src/repair_plan.rs:630: out.refused = Some(format!("{day}: {why}"));
UNCOVERED LINE crates/pull/src/repair_plan.rs:631: return out;
UNCOVERED LINE crates/pull/src/repair_plan.rs:634: Owes::Unsized | Owes::Unattested | Owes::Closed => out.length_unjudged.push(day),
UNCOVERED LINE crates/pull/src/repair_plan.rs:737: continue;
UNCOVERED LINE crates/pull/src/repair_plan.rs:756: return Ok(None);
UNCOVERED LINE crates/pull/src/repair_plan.rs:954: other => panic!("{other:?}"),
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:158:12 False: if other.rank() > self.rank() {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:268:12 True: if hours.kind() != SessionKind::Continuous {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:426:6 False: (parts.exchange == "NSE" && parts.segment == "CASH" && parts.contract.is_none())
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:459:15 False: while let Ok(month) = at.year_month() {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:471:13 False: let Ok(next) = Day::from_days(end.days_from_epoch().saturating_add(1)) else {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:490:9 False: let Some(dir) = StorePath::new(parts)
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:507:18 False: (text.len() == width && text.bytes().all(|b| b.is_ascii_digit()))
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:557:16 False: if out.bars > 0 {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:570:13 False: let Ok(day) = Day::from_days(number) else {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:736:12 True: if to <= from {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:755:9 False: let Some(bounds) = bounds else {
UNCOVERED BRANCH crates/pull/src/repair_plan.rs:1004:39 False: assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
UNCOVERED LINE crates/pull/src/repair_plan_not_owed_tests.rs:93: panic!("{on}: a landed session states its windows");
UNCOVERED LINE crates/pull/src/repair_plan_not_owed_tests.rs:172: panic!("{}: {:?}", session.day, session.why);
UNCOVERED BRANCH crates/pull/src/repair_plan_not_owed_tests.rs:91:13 False: let super::Owes::Windows((windows, count)) = super::owed("CASH", on.days_from_epoch())
UNCOVERED BRANCH crates/pull/src/repair_plan_not_owed_tests.rs:171:13 False: let NotOwed::InstrumentInactive(interval) = session.why else {
UNCOVERED BRANCH crates/pull/src/repair_plan_not_owed_tests.rs:282:22 False: .all(|m| !m.needs_repair() && m.refused.is_none())
UNCOVERED LINE crates/pull/src/request_minutes.rs:378: other => {
UNCOVERED LINE crates/pull/src/request_minutes.rs:379: return DayPlan::Unverified("venue hours", format!("venue hours {other:?}"));
UNCOVERED LINE crates/pull/src/request_minutes.rs:456: audit.findings.push(Finding {
UNCOVERED LINE crates/pull/src/request_minutes.rs:457: why: Why::Unverified,
UNCOVERED LINE crates/pull/src/request_minutes.rs:458: day: None,
UNCOVERED LINE crates/pull/src/request_minutes.rs:459: detail: "invalid day",
UNCOVERED LINE crates/pull/src/request_minutes.rs:460: minutes: 0,
UNCOVERED LINE crates/pull/src/request_minutes.rs:461: text: format!("request minute coverage UNVERIFIED: invalid day {number}"),
UNCOVERED LINE crates/pull/src/request_minutes.rs:462: });
UNCOVERED LINE crates/pull/src/request_minutes.rs:463: continue;
UNCOVERED LINE crates/pull/src/request_minutes.rs:1320: texts(&none)
UNCOVERED LINE crates/pull/src/request_minutes.rs:1360: panic!("{day} is attested")
UNCOVERED LINE crates/pull/src/request_minutes.rs:1479: groups[1].text
UNCOVERED LINE crates/pull/src/request_minutes.rs:1879: panic!("{:?}", audit.findings);
UNCOVERED LINE crates/pull/src/request_minutes.rs:1914: panic!(
UNCOVERED LINE crates/pull/src/request_minutes.rs:1951: panic!("{groups:?}");
UNCOVERED LINE crates/pull/src/request_minutes.rs:2058: panic!("{:?}", audit.days);
UNCOVERED LINE crates/pull/src/request_minutes.rs:2136: panic!("{:?}", audit.findings);
UNCOVERED LINE crates/pull/src/request_minutes.rs:2180: panic!("{:?}", done.failures);
UNCOVERED LINE crates/pull/src/request_minutes.rs:2202: lines[0]
UNCOVERED LINE crates/pull/src/request_minutes.rs:2238: named.failures[1].why
UNCOVERED LINE crates/pull/src/request_minutes.rs:2255: panic!("{day}: {:?}", audit.findings);
UNCOVERED LINE crates/pull/src/request_minutes.rs:2289: panic!("{:?}", audit.days);
UNCOVERED LINE crates/pull/src/request_minutes.rs:2351: panic!("{:?}", unflagged.findings);
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:377:22 False: Ok(hours) if hours.kind() == SessionKind::Continuous => hours,
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:455:13 False: let Ok(day) = Day::from_days(number) else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1283:13 False: only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1283:47 False: only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1300:13 False: whole.findings.is_empty() && whole.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1300:42 False: whole.findings.is_empty() && whole.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1475:13 False: groups[1].text.starts_with(
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1477:18 False: ) && groups[1].text.ends_with("the first: gap b"),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1683:21 False: first.starts_with(head) && first.contains(" P0 "),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1683:48 False: first.starts_with(head) && first.contains(" P0 "),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1878:13 False: let [finding] = audit.findings.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1886:13 False: finding
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1889:20 False: && finding
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1913:13 False: let Some(number) = found else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:1950:13 False: let [group] = groups.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2057:13 False: let [line] = audit.days.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2135:13 False: let [finding] = audit.findings.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2179:13 False: let [failure] = done.failures.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2254:17 False: let [finding] = audit.findings.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2288:13 False: let [line] = audit.days.as_slice() else {
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2317:13 False: audit.findings.is_empty() && audit.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2317:42 False: audit.findings.is_empty() && audit.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2332:13 False: audit.findings.is_empty() && audit.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2332:42 False: audit.findings.is_empty() && audit.days.is_empty(),
UNCOVERED BRANCH crates/pull/src/request_minutes.rs:2350:13 False: let [finding] = unflagged.findings.as_slice() else {
UNCOVERED LINE crates/pull/src/resolve.rs:180: return Err(
UNCOVERED LINE crates/pull/src/resolve.rs:181: "a resolution's buckets do not add up to the names it published. \
UNCOVERED LINE crates/pull/src/resolve.rs:182: That is a defect in the JOIN and not in the data, so re-fetching \
UNCOVERED LINE crates/pull/src/resolve.rs:183: will not close it. Nothing was published."
UNCOVERED LINE crates/pull/src/resolve.rs:184: .to_owned(),
UNCOVERED LINE crates/pull/src/resolve.rs:185: );
UNCOVERED LINE crates/pull/src/resolve.rs:315: Err(why) => {
UNCOVERED LINE crates/pull/src/resolve.rs:316: let refusal = note_refused(listing_url, why.to_string());
UNCOVERED LINE crates/pull/src/resolve.rs:317: failures.push(refusal);
UNCOVERED LINE crates/pull/src/resolve.rs:318: continue;
UNCOVERED LINE crates/pull/src/resolve.rs:378: Err(why) => {
UNCOVERED LINE crates/pull/src/resolve.rs:379: let refusal = note_refused(page_url, why.to_string());
UNCOVERED LINE crates/pull/src/resolve.rs:380: failures.push(refusal);
UNCOVERED LINE crates/pull/src/resolve.rs:381: continue;
UNCOVERED LINE crates/pull/src/resolve.rs:386: Err(why) => {
UNCOVERED LINE crates/pull/src/resolve.rs:387: let refusal = note_refused(csv_url, why);
UNCOVERED LINE crates/pull/src/resolve.rs:388: failures.push(refusal);
UNCOVERED LINE crates/pull/src/resolve.rs:389: continue;
UNCOVERED LINE crates/pull/src/resolve.rs:1033: snap.unlinked[0]
UNCOVERED LINE crates/pull/src/resolve.rs:1040: snap.unlinked[0]
UNCOVERED LINE crates/pull/src/resolve.rs:1045: snap.unlinked[0]
UNCOVERED LINE crates/pull/src/resolve.rs:1120: return;
UNCOVERED LINE crates/pull/src/resolve.rs:1212: return 0;
UNCOVERED LINE crates/pull/src/resolve.rs:1222: return 0;
UNCOVERED LINE crates/pull/src/resolve.rs:1242: if chunked {
UNCOVERED LINE crates/pull/src/resolve.rs:1243: let _end = stream.write_all(b"0\r\n\r\n");
UNCOVERED LINE crates/pull/src/resolve.rs:1244: }
UNCOVERED LINE crates/pull/src/resolve.rs:1245: let _flushed = stream.flush();
UNCOVERED LINE crates/pull/src/resolve.rs:1246: written
UNCOVERED BRANCH crates/pull/src/resolve.rs:179:12 True: if !self.is_sound() {
UNCOVERED BRANCH crates/pull/src/resolve.rs:1119:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/resolve.rs:1211:17 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/src/resolve.rs:1221:16 True: if stream.write_all(head.as_bytes()).is_err() {
UNCOVERED BRANCH crates/pull/src/resolve.rs:1242:16 True: if chunked {
UNCOVERED BRANCH crates/pull/src/resolve.rs:1242:16 False: if chunked {
UNCOVERED LINE crates/pull/src/response_keep.rs:298: _ => None,
UNCOVERED LINE crates/pull/src/response_keep.rs:341: _ => None,
UNCOVERED LINE crates/pull/src/response_keep.rs:794: fn io(op: &'static str, path: &Path, why: &io::Error) -> Self {
UNCOVERED LINE crates/pull/src/response_keep.rs:795: Self::Io {
UNCOVERED LINE crates/pull/src/response_keep.rs:796: op,
UNCOVERED LINE crates/pull/src/response_keep.rs:797: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:798: kind: why.kind(),
UNCOVERED LINE crates/pull/src/response_keep.rs:799: }
UNCOVERED LINE crates/pull/src/response_keep.rs:800: }
UNCOVERED LINE crates/pull/src/response_keep.rs:860: fn from(why: PathError) -> Self {
UNCOVERED LINE crates/pull/src/response_keep.rs:861: Self::Path(why)
UNCOVERED LINE crates/pull/src/response_keep.rs:862: }
UNCOVERED LINE crates/pull/src/response_keep.rs:1133: }
UNCOVERED LINE crates/pull/src/response_keep.rs:1134: let kind = match &why {
UNCOVERED LINE crates/pull/src/response_keep.rs:1135: store::safe_open::ReadRefusal::Io { source, .. } => source.kind(),
UNCOVERED LINE crates/pull/src/response_keep.rs:1136: _ => io::ErrorKind::Other,
UNCOVERED LINE crates/pull/src/response_keep.rs:1138: KeepError::Io {
UNCOVERED LINE crates/pull/src/response_keep.rs:1139: op: "open",
UNCOVERED LINE crates/pull/src/response_keep.rs:1140: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:1141: kind,
UNCOVERED LINE crates/pull/src/response_keep.rs:1142: }
UNCOVERED LINE crates/pull/src/response_keep.rs:1164: match created {
UNCOVERED LINE crates/pull/src/response_keep.rs:1166: Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
UNCOVERED LINE crates/pull/src/response_keep.rs:1167: store::safe_open::open_read_write(path)
UNCOVERED LINE crates/pull/src/response_keep.rs:1168: .map(|(file, _len)| Some(file))
UNCOVERED LINE crates/pull/src/response_keep.rs:1169: .map_err(refused)
UNCOVERED LINE crates/pull/src/response_keep.rs:1171: Err(why) => Err(KeepError::io("create", path, &why)),
UNCOVERED LINE crates/pull/src/response_keep.rs:1182: return Err(KeepError::Damaged {
UNCOVERED LINE crates/pull/src/response_keep.rs:1183: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:1184: ordinal: cap / RECEIPT_LEN as u64,
UNCOVERED LINE crates/pull/src/response_keep.rs:1185: });
UNCOVERED LINE crates/pull/src/response_keep.rs:1345: return Err(KeepError::BodyMismatch {
UNCOVERED LINE crates/pull/src/response_keep.rs:1346: path,
UNCOVERED LINE crates/pull/src/response_keep.rs:1347: what: "is missing",
UNCOVERED LINE crates/pull/src/response_keep.rs:1348: });
UNCOVERED LINE crates/pull/src/response_keep.rs:1379: return Err(KeepError::BodyMismatch {
UNCOVERED LINE crates/pull/src/response_keep.rs:1380: path,
UNCOVERED LINE crates/pull/src/response_keep.rs:1381: what: "does not match its receipt's BLAKE3",
UNCOVERED LINE crates/pull/src/response_keep.rs:1382: });
UNCOVERED LINE crates/pull/src/response_keep.rs:1385: path,
UNCOVERED LINE crates/pull/src/response_keep.rs:1387: })
UNCOVERED LINE crates/pull/src/response_keep.rs:1423: return Err(KeepError::Damaged { path, ordinal: 0 });
UNCOVERED LINE crates/pull/src/response_keep.rs:1465: Ok(None) => Err(KeepError::NotRegular { path: path.clone() }),
UNCOVERED LINE crates/pull/src/response_keep.rs:1466: Err(why) => Err(why),
UNCOVERED LINE crates/pull/src/response_keep.rs:1471: Err(std::fs::TryLockError::Error(why)) => return Err(KeepError::io("lock", &path, &why)),
UNCOVERED LINE crates/pull/src/response_keep.rs:1512: Some(answer) => answer,
UNCOVERED LINE crates/pull/src/response_keep.rs:1523: Err(why) => Err(KeepError::io("lock", &path, &why)),
UNCOVERED LINE crates/pull/src/response_keep.rs:1613: return Some(answer);
UNCOVERED LINE crates/pull/src/response_keep.rs:1640: return Err(KeepError::NotRegular {
UNCOVERED LINE crates/pull/src/response_keep.rs:1641: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:1642: });
UNCOVERED LINE crates/pull/src/response_keep.rs:1667: return Ok(());
UNCOVERED LINE crates/pull/src/response_keep.rs:1780: |_| key.to_owned(),
UNCOVERED LINE crates/pull/src/response_keep.rs:1935: )?;
UNCOVERED LINE crates/pull/src/response_keep.rs:2000: return Err(KeepError::Damaged {
UNCOVERED LINE crates/pull/src/response_keep.rs:2001: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:2002: ordinal: MAX_WINDOW_RECORDS,
UNCOVERED LINE crates/pull/src/response_keep.rs:2003: });
UNCOVERED LINE crates/pull/src/response_keep.rs:2012: return Ok((0, None));
UNCOVERED LINE crates/pull/src/response_keep.rs:2020: return Ok((0, None));
UNCOVERED LINE crates/pull/src/response_keep.rs:2078: path: journal.to_path_buf(),
UNCOVERED LINE crates/pull/src/response_keep.rs:2079: ordinal: next.saturating_sub(1),
UNCOVERED LINE crates/pull/src/response_keep.rs:2080: };
UNCOVERED LINE crates/pull/src/response_keep.rs:2154: note_rolled(root, pack, &key, one);
UNCOVERED LINE crates/pull/src/response_keep.rs:2281: if !matches!(
UNCOVERED LINE crates/pull/src/response_keep.rs:2288: return Err(why);
UNCOVERED BRANCH crates/pull/src/response_keep.rs:406:20 False: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:437:16 False: if let Some(slot) = out.get_mut(124..) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:587:16 False: if let Some(slot) = out.get_mut(0..8) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:590:16 False: if let Some(slot) = out.get_mut(8..12) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:593:16 False: if let Some(slot) = out.get_mut(12) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:597:16 False: if let Some(slot) = out.get_mut(20..24) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1024:20 False: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1038:16 False: if let Some(slot) = out.get_mut(60..) {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1067:9 False: receipt.asked_from == self.asked_from
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1068:16 False: && receipt.asked_to == self.asked_to
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1069:16 False: && receipt.digest == self.digest
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1129:12 False: if why.found().is_some() {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1166:21 True: Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1166:21 False: Err(why) if why.kind() == io::ErrorKind::AlreadyExists => {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1181:8 True: if len > cap {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1344:9 False: let Some(file) = open_regular(&path, false)? else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1378:8 True: if brutex_core::blake3::hash(&raw) != receipt.digest {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1422:9 False: let Some(file) = open_regular(&path, false)? else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1612:16 True: if let Ok(answer) = rx.try_recv() {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1639:9 False: let Some(file) = open_regular(path, true)? else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1666:9 False: let Some(dir) = path.parent() else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1683:15 False: while let Some(one) = at {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:1999:8 True: if whole > MAX_WINDOW_RECORDS {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:2011:9 False: let Some(last) = whole.checked_sub(1) else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:2019:9 False: let Some(before) = last.checked_sub(1) else {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:2153:12 True: if one.rolled {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:2210:40 False: if (receipts_full || (body_full && fits_empty) || rows_full) && index + 1 < MAX_SEGMENTS {
UNCOVERED BRANCH crates/pull/src/response_keep.rs:2281:16 True: if !matches!(
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:118: return out;
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:123: out.extend(files_under(&path));
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:325: panic!("{refused:?}");
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:925: matches!(
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:966: panic!("{refused:?}");
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:1605: started.elapsed()
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:1947: assert!(
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:1948: std::time::Instant::now() < deadline,
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:1951: std::thread::sleep(Duration::from_millis(10));
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:2001: panic!("one further month: {outcome:?}");
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:2089: matches!(
UNCOVERED LINE crates/pull/src/response_keep_tests.rs:2127: panic!("July holds exactly the missing pointer: {july:?}");
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:117:9 False: let Ok(entries) = fs::read_dir(dir) else {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:122:12 True: if path.is_dir() {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:324:9 False: let Err(KeepError::ReceiptsFull { pack: named }) = &refused else {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:533:9 False: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:533:43 False: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:961:9 False: let Err(KeepError::LockStalled {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:1946:11 True: while parked_on(&lock_file) > 0 {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:2000:9 False: let [july] = outcome.further.as_slice() else {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:2126:9 False: let [pointer] = july.as_slice() else {
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:2463:13 False: assert!(july.receipts.exists() && !july.bodies.exists());
UNCOVERED BRANCH crates/pull/src/response_keep_tests.rs:2463:39 False: assert!(july.receipts.exists() && !july.bodies.exists());
UNCOVERED LINE crates/pull/src/riskfree.rs:587: return Err(RowFault::FieldCount {
UNCOVERED LINE crates/pull/src/riskfree.rs:588: found: fields.len(),
UNCOVERED LINE crates/pull/src/riskfree.rs:589: });
UNCOVERED LINE crates/pull/src/riskfree.rs:1107: LookupError::CarriedTooLong { .. } => "carried too long",
UNCOVERED LINE crates/pull/src/riskfree.rs:1108: LookupError::HoleBeforeGap { .. } => "hole before a gap",
UNCOVERED LINE crates/pull/src/riskfree.rs:1593: holed.on(asked)
UNCOVERED BRANCH crates/pull/src/riskfree.rs:576:9 False: let [
UNCOVERED BRANCH crates/pull/src/riskfree.rs:1630:13 False: said.contains("2020-01-20") && said.contains("unexplained"),
UNCOVERED BRANCH crates/pull/src/riskfree.rs:1630:44 False: said.contains("2020-01-20") && said.contains("unexplained"),
UNCOVERED BRANCH crates/pull/src/riskfree.rs:1829:56 False: matches!(tails.as_slice(), [small, big] if big.1 <= small.1.saturating_mul(10).max(2_000)),
UNCOVERED LINE crates/pull/src/rolling.rs:134: })?
UNCOVERED LINE crates/pull/src/rolling.rs:141: })?,
UNCOVERED LINE crates/pull/src/rolling.rs:149: })?;
UNCOVERED LINE crates/pull/src/rolling.rs:187: crate::calendar::DayKind::Unmeasured => {}
UNCOVERED LINE crates/pull/src/rolling.rs:192: },
UNCOVERED LINE crates/pull/src/rolling.rs:238: Err(_) => Err(RollingError::NoExpiry {
UNCOVERED LINE crates/pull/src/rolling.rs:239: why: "the day is before this weekly regime was verified from",
UNCOVERED LINE crates/pull/src/rolling.rs:240: }),
UNCOVERED LINE crates/pull/src/rolling.rs:247: }),
UNCOVERED LINE crates/pull/src/rolling.rs:281: })?;
UNCOVERED LINE crates/pull/src/rolling.rs:286: RollingError::NoExpiry {
UNCOVERED LINE crates/pull/src/rolling.rs:287: why: "the bar's own day is not a real date",
UNCOVERED LINE crates/pull/src/rolling.rs:288: }
UNCOVERED LINE crates/pull/src/rolling.rs:289: })?;
UNCOVERED LINE crates/pull/src/rolling.rs:543: crate::vendor::PathSegment::Value { placeholder, .. } => {
UNCOVERED LINE crates/pull/src/rolling.rs:544: out.push_str(placeholder);
UNCOVERED LINE crates/pull/src/rolling.rs:545: }
UNCOVERED LINE crates/pull/src/rolling.rs:802: Some(a) => Some(paisa(a, at, scale, "strike")?),
UNCOVERED LINE crates/pull/src/rolling.rs:935: PriceScale::Paisa => cell
UNCOVERED LINE crates/pull/src/rolling.rs:936: .as_i64()
UNCOVERED LINE crates/pull/src/rolling.rs:937: .ok_or(RollingError::Unrepresentable { field })?,
UNCOVERED LINE crates/pull/src/rolling.rs:1084: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/src/rolling.rs:1410: matches!(
UNCOVERED LINE crates/pull/src/rolling.rs:1470: panic!("Dhan is an HTTP broker");
UNCOVERED BRANCH crates/pull/src/rolling.rs:1083:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/rolling.rs:1469:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/rolling.rs:1546:52 False: Err(RollingError::NoExpiry { why }) if why.contains("not one this vendor serves")
UNCOVERED BRANCH crates/pull/src/rolling.rs:1565:60 False: Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
UNCOVERED BRANCH crates/pull/src/rolling.rs:1602:60 False: Err(RollingError::NoExpiry { why }) if why.contains("full regular session")
UNCOVERED BRANCH crates/pull/src/rolling.rs:1615:52 False: Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
UNCOVERED BRANCH crates/pull/src/rolling.rs:1727:77 False: Err(RollingError::Uncountable { field: "oi", text }) if text == cell
UNCOVERED BRANCH crates/pull/src/rolling.rs:1733:17 False: assert!(said.contains("`oi`") && said.contains("1234.5"), "{said}");
UNCOVERED BRANCH crates/pull/src/rolling.rs:1733:42 False: assert!(said.contains("`oi`") && said.contains("1234.5"), "{said}");
UNCOVERED BRANCH crates/pull/src/rolling.rs:1789:95 False: matches!(&got, Err(RollingError::Unstampable { field: "timestamp", text }) if *text == stamp),
UNCOVERED BRANCH crates/pull/src/rolling.rs:1810:92 False: matches!(&got, Err(RollingError::Uncountable { field: "volume", text }) if text == volume),
UNCOVERED BRANCH crates/pull/src/rolling.rs:1831:13 False: said.contains("`timestamp`") && said.contains("null"),
UNCOVERED BRANCH crates/pull/src/rolling.rs:1831:45 False: said.contains("`timestamp`") && said.contains("null"),
UNCOVERED BRANCH crates/pull/src/rolling.rs:1844:89 False: matches!(&got, Err(RollingError::NotAPrice { field: "close", text }) if text == close),
UNCOVERED BRANCH crates/pull/src/rolling.rs:1862:17 False: assert!(said.contains("`close`") && said.contains("-5"), "{said}");
UNCOVERED BRANCH crates/pull/src/rolling.rs:1862:45 False: assert!(said.contains("`close`") && said.contains("-5"), "{said}");
UNCOVERED LINE crates/pull/src/scrub.rs:219: Err(why) => {
UNCOVERED LINE crates/pull/src/scrub.rs:220: return Finding::Unreadable {
UNCOVERED LINE crates/pull/src/scrub.rs:221: path: format!("{:?}", entry.key.symbol),
UNCOVERED LINE crates/pull/src/scrub.rs:222: why: format!("the entry does not name a legal store path: {why}"),
UNCOVERED LINE crates/pull/src/scrub.rs:223: };
UNCOVERED BRANCH crates/pull/src/scrub.rs:718:50 False: Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty()
UNCOVERED BRANCH crates/pull/src/scrub.rs:718:86 False: Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty()
UNCOVERED BRANCH crates/pull/src/scrub.rs:759:31 False: i == j || one != other,
UNCOVERED LINE crates/pull/src/series_identity.rs:438: return Verdict::Unchecked(Unchecked::NoDayBar);
UNCOVERED BRANCH crates/pull/src/series_identity.rs:437:13 False: let Some(range) = self.days.get(at) else {
UNCOVERED BRANCH crates/pull/src/series_identity.rs:977:17 False: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
UNCOVERED BRANCH crates/pull/src/series_identity.rs:977:39 False: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
UNCOVERED BRANCH crates/pull/src/series_identity.rs:977:61 False: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
UNCOVERED LINE crates/pull/src/session.rs:265: Self::VenueHoursUnknown { venue, days } => write!(
UNCOVERED LINE crates/pull/src/session.rs:266: f,
UNCOVERED LINE crates/pull/src/session.rs:1253: return Err(SessionError::SessionLengthUnmeasured { days: day });
UNCOVERED LINE crates/pull/src/session.rs:1262: venue: why.venue().label(),
UNCOVERED LINE crates/pull/src/session.rs:1263: days: why.day().days_from_epoch(),
UNCOVERED LINE crates/pull/src/session.rs:1264: })?;
UNCOVERED LINE crates/pull/src/session.rs:1746: panic!("{epoch_day} is an open day in the calendar");
UNCOVERED LINE crates/pull/src/session.rs:1815: panic!("{epoch_day}: a recorded window");
UNCOVERED LINE crates/pull/src/session.rs:1951: c.from(),
UNCOVERED LINE crates/pull/src/session.rs:1952: c.to()
UNCOVERED LINE crates/pull/src/session.rs:2165: panic!("there is no December 1969 in this calendar")
UNCOVERED BRANCH crates/pull/src/session.rs:1563:11 False: while end < last {
UNCOVERED BRANCH crates/pull/src/session.rs:1744:17 False: let crate::calendar::DayKind::Open(session) = crate::calendar::kind_of(epoch_day)
UNCOVERED BRANCH crates/pull/src/session.rs:1814:17 False: let DayKind::Open(window) = with_recorded_window(epoch_day, identity_answer) else {
UNCOVERED BRANCH crates/pull/src/session.rs:1879:17 False: assert!(text.contains("18935") && text.contains("length"), "{text}");
UNCOVERED BRANCH crates/pull/src/session.rs:1879:43 False: assert!(text.contains("18935") && text.contains("length"), "{text}");
UNCOVERED BRANCH crates/pull/src/session.rs:1949:76 False: c.to() == c.to().end_of_month() || c.to() == to || inside_a_wide_month,
UNCOVERED BRANCH crates/pull/src/session.rs:2044:27 False: .find(|c| c.from() <= muhurat && muhurat <= c.to())
UNCOVERED BRANCH crates/pull/src/session.rs:2164:13 False: let Err(why) = first.months_before(1) else {
UNCOVERED LINE crates/pull/src/ssm.rs:925: SsmError::unreachable(format!("the HTTPS client could not be built: {why}"))
UNCOVERED LINE crates/pull/src/ssm.rs:926: })?;
UNCOVERED LINE crates/pull/src/ssm.rs:966: let status = answer.status();
UNCOVERED LINE crates/pull/src/ssm.rs:970: let text = answer_within(&mut answer).await?;
UNCOVERED LINE crates/pull/src/ssm.rs:971: answer_of(region, status.as_u16(), &text)
UNCOVERED LINE crates/pull/src/ssm.rs:1391: token.is_some()
UNCOVERED LINE crates/pull/src/ssm.rs:1479: panic!("{region:?} is not ap-south-1 and must not be signed for")
UNCOVERED LINE crates/pull/src/ssm.rs:1508: panic!("a stamp with no date in it cannot scope a signature")
UNCOVERED LINE crates/pull/src/ssm.rs:1550: panic!("{wrong} carries no Parameter.Value and must be refused")
UNCOVERED LINE crates/pull/src/ssm.rs:1555: panic!("a non-JSON answer is a different fault and is named as one")
UNCOVERED LINE crates/pull/src/ssm.rs:1657: panic!("a profile that is not in the file must be refused")
UNCOVERED LINE crates/pull/src/ssm.rs:1668: panic!("an id with no secret cannot sign anything")
UNCOVERED LINE crates/pull/src/ssm.rs:1694: panic!("{tag}: a blank half of the pair is no pair")
UNCOVERED LINE crates/pull/src/ssm.rs:1721: panic!("an empty exported key id must not become an identity")
UNCOVERED LINE crates/pull/src/ssm.rs:1768: panic!("a half-set environment must be refused: {found:?}")
UNCOVERED LINE crates/pull/src/ssm.rs:1806: panic!("a file that is not there cannot hold an identity")
UNCOVERED LINE crates/pull/src/ssm.rs:1865: panic!("a foreign region must not yield a parameter value")
UNCOVERED LINE crates/pull/src/ssm.rs:1885: panic!("{bad} not UTF-8 must be refused")
UNCOVERED LINE crates/pull/src/ssm.rs:1949: if answer.is_err() {
UNCOVERED LINE crates/pull/src/ssm.rs:1950: // Release the reader blocked in open(2) before failing.
UNCOVERED LINE crates/pull/src/ssm.rs:1951: let _writer = std::fs::OpenOptions::new().write(true).open(&path);
UNCOVERED LINE crates/pull/src/ssm.rs:1952: }
UNCOVERED LINE crates/pull/src/ssm.rs:1953: panic!("a FIFO credentials file must refuse promptly, got {answer:?}")
UNCOVERED LINE crates/pull/src/ssm.rs:1988: panic!("one byte past the bound must be refused")
UNCOVERED LINE crates/pull/src/ssm.rs:1998: panic!("a file that is not text holds no profile")
UNCOVERED LINE crates/pull/src/ssm.rs:2050: panic!("one byte past the cap must be refused")
UNCOVERED LINE crates/pull/src/ssm.rs:2057: panic!("a body cut short is not an answer")
UNCOVERED BRANCH crates/pull/src/ssm.rs:1387:45 False: FIXED.contains(name) || sent.contains(name),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1478:17 False: let Err(SsmError { detail, kind }) = foreign.authorization(&identity()) else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1507:13 False: let Err(SsmError { detail, .. }) = bad.authorization(&identity()) else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1549:17 False: let Err(SsmError { detail, .. }) = value_of(wrong) else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1554:13 False: let Err(SsmError { detail, .. }) = value_of("not json") else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1656:13 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "nope") else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1666:13 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
UNCOVERED BRANCH crates/pull/src/ssm.rs:1692:17 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
UNCOVERED BRANCH crates/pull/src/ssm.rs:1717:13 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_lookup(env(&[
UNCOVERED BRANCH crates/pull/src/ssm.rs:1767:17 False: let Err(SsmError { detail, .. }) = found else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1771:17 False: detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1771:57 False: detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1804:13 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&missing, "default")
UNCOVERED BRANCH crates/pull/src/ssm.rs:1835:17 False: !value.contains("Put") && !value.contains("Delete"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1835:43 False: !value.contains("Put") && !value.contains("Delete"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1857:13 False: let Err(SsmError { detail, kind }) = get_parameter(
UNCOVERED BRANCH crates/pull/src/ssm.rs:1884:17 False: let Err(SsmError { detail, .. }) = refuse_non_unicode(read) else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1907:13 False: !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1907:50 False: !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1911:13 False: shown.contains("ap-south-1") && shown.contains("<redacted>"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1911:45 False: shown.contains("ap-south-1") && shown.contains("<redacted>"),
UNCOVERED BRANCH crates/pull/src/ssm.rs:1948:13 False: let Ok(Err(SsmError { detail, kind })) = answer else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1949:16 True: if answer.is_err() {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1949:16 False: if answer.is_err() {
UNCOVERED BRANCH crates/pull/src/ssm.rs:1986:13 False: let Err(SsmError { detail, kind }) = AwsIdentity::from_credentials_file(&over, "default")
UNCOVERED BRANCH crates/pull/src/ssm.rs:1996:13 False: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&binary, "default")
UNCOVERED BRANCH crates/pull/src/ssm.rs:2049:13 False: let Err(SsmError { detail, kind }) = answered("", vec![b'a'; MAX_ANSWER_BYTES + 1]) else {
UNCOVERED BRANCH crates/pull/src/ssm.rs:2056:13 False: let Err(SsmError { detail, .. }) = answered("Content-Length: 4\r\n", b"ab".to_vec()) else {
UNCOVERED LINE crates/pull/src/tenor.rs:203: Self::HoursUnverified(refusal) => write!(
UNCOVERED LINE crates/pull/src/tenor.rs:204: f,
UNCOVERED LINE crates/pull/src/tenor.rs:490: at.years(YearBasis::Calendar365)
UNCOVERED LINE crates/pull/src/tenor.rs:510: born.years(YearBasis::Calendar365)
UNCOVERED BRANCH crates/pull/src/tenor.rs:676:13 False: words.starts_with("the expiry could not be settled: ") && words.contains("2024-03-29"),
UNCOVERED BRANCH crates/pull/src/tenor.rs:676:71 False: words.starts_with("the expiry could not be settled: ") && words.contains("2024-03-29"),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:201: Err(refuse())
UNCOVERED LINE crates/pull/src/tickstore_write.rs:602: _ => write!(f, "{code}"),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:748: pub fn run_id(&self) -> &str {
UNCOVERED LINE crates/pull/src/tickstore_write.rs:749: &self.run_id
UNCOVERED LINE crates/pull/src/tickstore_write.rs:750: }
UNCOVERED LINE crates/pull/src/tickstore_write.rs:844: return Ok(None);
UNCOVERED LINE crates/pull/src/tickstore_write.rs:847: return Ok(None);
UNCOVERED LINE crates/pull/src/tickstore_write.rs:851: return Ok(None);
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1065: return Err(Refusal::TooManyEntries {
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1066: entries: entries.len(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1067: });
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1147: return Err(corrupt(format!(
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1148: "{} bytes, the record states {}",
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1149: bytes.len(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1150: entry.len
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1151: )));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1174: name: entry.name.clone(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1175: why,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1176: };
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1190: return Err(failed(
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1191: "the block does not rebuild to the member".to_owned(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1192: ));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1313: fn flush(&mut self) -> std::io::Result<()> {
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1314: let mut handle: &File = self.file;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1315: handle.flush()
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1316: }
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1327: path: temp.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1328: step,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1329: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1330: };
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1336: store::staging::StagingError::Io { source, .. } => failed("create", &source),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1352: name: "the index".to_owned(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1353: why,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1354: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1372: failed("write", &why)
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1399: Mismatch::Zip(why) => format!("the zip: {why}"),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1424: return Err(Mismatch::File(format!(
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1425: "{} entries, the zip holds {}",
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1426: index.len(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1427: entries.len()
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1428: )));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1454: return Err(Mismatch::File(format!(
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1455: "`{}`: kind {} for this entry",
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1456: entry.name, stored.kind
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1457: )));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1476: return Err(verify_refusal("an entry went missing".to_owned()));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1495: other => Mismatch::File(other.to_string()),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1512: path: dir.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1514: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1515: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1522: }
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1743: path: final_path.clone(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1745: kind,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1746: }));
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1753: }
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1795: lock: lock_path.clone(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1796: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1797: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1801: Err(std::fs::TryLockError::Error(why)) => Err(Refusal::LockFailed {
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1802: lock: lock_path,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1803: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1804: }),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1819: _ => std::io::ErrorKind::Other,
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1855: path: dir.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1857: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1858: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1865: path: parent.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1867: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1868: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1877: path: temp.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1879: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1880: };
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1895: path: temp.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1897: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1898: })?;
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1902: path: dir.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1904: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1905: })
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1917: return Err(Refusal::EncodeFailed {
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1918: name: String::new(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1919: why: "an entry went missing".to_owned(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1920: });
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1926: return Err(Refusal::DayTooLarge { bytes });
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1936: path: temp.to_path_buf(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1938: kind: why.kind(),
UNCOVERED LINE crates/pull/src/tickstore_write.rs:1939: };
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:198:12 False: if identity.canonical_stem() == stem {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:843:9 False: let Some((&header, body)) = lines.split_first() else {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:846:9 False: let Ok(header_len) = u32::try_from(header.len()) else {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:850:9 False: let Ok(ncols32) = u32::try_from(ncols) else {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:933:14 False: (a.is_ascii_digit() && b.is_ascii_digit())
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:966:31 True: if decimals == Some(0) || d > 15 {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1064:8 True: if u32::try_from(entries.len()).is_err() {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1146:8 True: if wide(bytes.len()) != entry.len {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1189:8 True: if back != data {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1366:27 False: return Err(if why.kind() == std::io::ErrorKind::StorageFull {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1423:8 True: if index.len() != entries.len() {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1443:20 False: let same = stored.name.as_bytes() == entry.raw_name.as_slice()
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1445:16 False: && stored.crc == entry.crc32
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1453:12 True: if (stored.kind == KIND_DIR) != is_directory(entry) {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1475:13 False: let (Some(stored), Some(entry)) = (index.get(at), entries.get(at)) else {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1519:12 False: if name.starts_with(&prefix) {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1751:12 False: if !left.is_empty() {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1845:15 False: while let Some(here) = at {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1847:25 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1916:13 False: let Some(entry) = entries.get(at) else {
UNCOVERED BRANCH crates/pull/src/tickstore_write.rs:1925:12 True: if bytes > DAY_BLOCKS_CAP {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:232: panic!("written: {:?}", run.result);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:359: panic!("{first:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:369: panic!("{again:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:399: panic!("{first:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:407: panic!("{again:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:521: 1 => 2,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:522: 2 => 4,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:523: _ => 8,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:573: data.len()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:594: panic!("{run:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:699: panic!("byte {at}: {run:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:861: matches!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:904: panic!("{held:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:911: panic!("{ours:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:971: panic!("{run:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1086: String::from_utf8_lossy(field)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1135: encoded.block.len(),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1136: text.len()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1259: fn quantiles(mut ns: Vec<u128>) -> (u128, u128, u128) {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1260: ns.sort_unstable();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1261: let at = |q: usize| {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1262: ns.get(ns.len().saturating_sub(1) * q / 100)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1263: .copied()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1264: .unwrap_or(0)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1265: };
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1266: (at(50), at(99), ns.last().copied().unwrap_or(0))
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1267: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1270: fn refuse_the_real_store(path: &Path) {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1271: let real = Path::new("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1272: let tick = Path::new("/Volumes/WD_BLACK/brutex/tickstore-data");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1273: assert!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1274: !path.starts_with(real) && !path.starts_with(tick),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1276: path.display()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1278: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1293: fn measure_real_days_against_the_tools_files() {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1294: let spec = std::env::var("BRUTEX_TSW_REAL").expect("BRUTEX_TSW_REAL names the days");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1295: let out = PathBuf::from(std::env::var("BRUTEX_TSW_OUT").expect("BRUTEX_TSW_OUT"));
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1296: refuse_the_real_store(&out);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1297: std::fs::create_dir_all(&out).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1298: for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1299: let (zip, tool) = pair.split_once('=').expect("ZIP=TOOL_BTS");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1300: let (zip, tool) = (Path::new(zip), Path::new(tool));
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1301: let name = zip.file_name().unwrap().to_string_lossy().into_owned();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1302: let identity = DayIdentity::of_zip_name(&name).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1303: let start = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1304: let run = write_day(zip, &out, &WriteOptions::new("measure"));
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1305: let write_ns = start.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1306: let mine = match run.result {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1307: Ok(Outcome::Written(report) | Outcome::Held(report)) => report,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1308: Err(why) => panic!("{name}: {why}"),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1310: let source = File::open(zip).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1311: let len = source.metadata().unwrap().len();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1312: let entries = zip_entries(&source, 0, len).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1313: let start = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1314: let theirs = verify_file(tool, &source, &entries, 8).expect("the tool's file matches");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1315: let verify_ns = start.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1316: let largest = entries.iter().map(|e| e.len).max().unwrap_or(0);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1317: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1326: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1330: u128::from(mine.stored_bytes) * 1_000 / u128::from(theirs.stored_bytes.max(1)),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1333: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1335: write_ns / 1_000_000,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1336: u128::from(mine.csv_bytes) * 1_000 / write_ns.max(1),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1337: verify_ns / 1_000_000,
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1338: u128::from(mine.csv_bytes) * 1_000 / verify_ns.max(1)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1340: for (label, path) in [
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1341: ("this writer", &mine.path),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1342: ("the tool", &tool.to_path_buf()),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1343: ] {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1344: let file = File::open(path).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1345: let size = file.metadata().unwrap().len();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1346: let index = read_index(&file, size).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1347: let mut ns = Vec::new();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1348: let mut bytes = 0_u64;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1349: let start = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1350: for entry in index.iter().filter(|e| e.kind != KIND_DIR) {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1351: let t = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1352: let back = rebuild(&file, entry).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1353: ns.push(t.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1354: bytes += back.len() as u64;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1355: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1356: let all = start.elapsed().as_nanos();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1357: let (p50, p99, max) = quantiles(ns.clone());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1358: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1360: ns.len(),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1361: u128::from(bytes) * 1_000 / all.max(1)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1365: let mut ns = Vec::new();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1366: match identity.tree {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1368: let listed = NfoTickStore::new(&out).day(identity.day).unwrap().unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1369: let tickers: Vec<String> = listed
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1370: .entries()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1371: .iter()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1372: .filter_map(|f| {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1373: let tail = f.entry.rsplit(['/', '\\']).next()?;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1374: Some(tail.strip_suffix(".NFO.csv")?.to_owned())
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1375: })
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1376: .collect();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1377: for _ in 0..10 {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1378: for ticker in &tickers {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1379: let t = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1380: std::hint::black_box(listed.locate(ticker).is_ok());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1381: ns.push(t.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1382: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1385: Tree::Cm(kind) => {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1386: let listing = TickStore::new(&out)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1387: .day(kind, identity.day)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1388: .unwrap()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1389: .unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1390: let stems: Vec<String> = listing
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1391: .entries()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1392: .iter()
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1393: .filter_map(|f| {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1394: let tail = f.entry.rsplit(['/', '\\']).next()?;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1395: Some(tail.strip_suffix(".csv")?.to_owned())
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1396: })
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1397: .collect();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1398: for _ in 0..10 {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1399: for stem in &stems {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1400: let t = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1401: std::hint::black_box(listing.locate(stem).is_ok());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1402: ns.push(t.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1403: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1407: let n = ns.len();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1408: let (p50, p99, max) = quantiles(ns);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1409: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1412: assert!(n > 0, "{name}: nothing was looked up");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1414: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1425: fn measure_ruzstd_fastest_on_bar_months() {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1429: let spec = std::env::var("BRUTEX_TSW_BAR_MONTHS").expect("BRUTEX_TSW_BAR_MONTHS");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1430: for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1431: let (label, path) = pair.split_once('=').expect("LABEL=PATH");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1432: let bytes = std::fs::read(path).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1433: let rows = &bytes[HEADER..];
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1434: assert_eq!(rows.len() % ROW, 0, "{label}: whole rows");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1435: let bars = rows.len() / ROW;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1436: let whole = zstd(rows).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1437: let frames: Vec<Vec<u8>> = rows
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1438: .chunks(ROW * PAGE)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1439: .map(|chunk| zstd(chunk).unwrap())
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1440: .collect();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1441: let paged: usize = frames.iter().map(Vec::len).sum::<usize>() + frames.len() * 8;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1442: let mut state = 0x9E37_79B9_7F4A_7C15_u64;
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1443: let mut ns = Vec::new();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1444: for _ in 0..2_000 {
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1445: state = state
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1446: .wrapping_mul(6_364_136_223_846_793_005)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1447: .wrapping_add(1);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1448: let at = usize::try_from(state >> 33).unwrap() % frames.len();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1449: let want = rows.chunks(ROW * PAGE).nth(at).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1450: let t = Instant::now();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1451: let mut back = Vec::with_capacity(want.len());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1452: let mut decoder =
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1453: ruzstd::decoding::StreamingDecoder::new(frames[at].as_slice()).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1454: std::io::Read::read_to_end(&mut decoder, &mut back).unwrap();
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1455: ns.push(t.elapsed().as_nanos());
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1456: assert_eq!(back, want, "{label}: page {at} round trip");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1458: let (p50, p99, max) = quantiles(ns);
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1459: println!(
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1461: rows.len(),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1462: rows.len() * 1_000 / bars.max(1),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1463: whole.len(),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1464: whole.len() * 1_000 / bars.max(1),
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1466: paged * 1_000 / bars.max(1)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1469: }
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1576: log_of(&root)
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1589: panic!("{run:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1699: panic!("{run:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1715: panic!("{first:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1729: panic!("{again:?}");
UNCOVERED LINE crates/pull/src/tickstore_write_tests.rs:1743: panic!("{dated:?}");
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:231:9 False: let Ok(Outcome::Written(report)) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:358:9 False: let Ok(Outcome::Written(written)) = first.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:368:13 False: let Ok(Outcome::Held(held)) = again.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:398:9 False: let Ok(Outcome::Written(written)) = first.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:406:9 False: let Err(Refusal::HeldDayDiffers { path, why }) = &again.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:537:17 False: at + if checksum { 4 } else { 0 },
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:593:9 False: let Err(Refusal::MemberCorrupt { name, why }) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:698:13 False: let Err(Refusal::VerifyFailed { temp, .. }) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:903:9 False: let Ok(Outcome::Held(report)) = &held.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:910:9 False: let Ok(Outcome::Written(mine)) = &ours.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:970:9 False: let Err(Refusal::HeldDayDiffers { why, .. }) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1274:9 True: !path.starts_with(real) && !path.starts_with(tick),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1274:9 False: !path.starts_with(real) && !path.starts_with(tick),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1274:36 True: !path.starts_with(real) && !path.starts_with(tick),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1274:36 False: !path.starts_with(real) && !path.starts_with(tick),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1588:9 False: let Err(Refusal::TempNotCreated { word, .. }) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1698:9 False: let Ok(Outcome::Written(report)) = &run.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1714:9 False: let Ok(Outcome::Written(written)) = first.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1728:9 False: let Err(Refusal::ZipCorruptOnVerify { path, why }) = &again.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1733:9 False: why.contains("member_corrupt") && why.contains("CRC-32"),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1733:43 False: why.contains("member_corrupt") && why.contains("CRC-32"),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1742:9 False: let Err(Refusal::HeldDayDiffers { why, .. }) = &dated.result else {
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1746:9 False: why.contains("NIFTY.NSE_IDX.csv") && why.contains("DOS time"),
UNCOVERED BRANCH crates/pull/src/tickstore_write_tests.rs:1746:46 False: why.contains("NIFTY.NSE_IDX.csv") && why.contains("DOS time"),
UNCOVERED LINE crates/pull/src/totp.rs:234: TotpError::HmacRefusedTheKey => "hmac-refused",
UNCOVERED LINE crates/pull/src/totp.rs:356: return Err(TotpError::Empty);
UNCOVERED LINE crates/pull/src/totp.rs:360: return Err(TotpError::Empty);
UNCOVERED LINE crates/pull/src/totp.rs:793: continue;
UNCOVERED LINE crates/pull/src/totp.rs:807: offenders.push(path.display().to_string());
UNCOVERED BRANCH crates/pull/src/totp.rs:354:9 False: let Some(&last) = tag.last() else {
UNCOVERED BRANCH crates/pull/src/totp.rs:359:9 False: let Some(window) = tag.get(offset..offset + 4) else {
UNCOVERED BRANCH crates/pull/src/totp.rs:789:20 True: if !path
UNCOVERED BRANCH crates/pull/src/totp.rs:806:20 True: if text.contains(&needle) || text.contains(&glob) {
UNCOVERED BRANCH crates/pull/src/totp.rs:806:46 True: if text.contains(&needle) || text.contains(&glob) {
UNCOVERED LINE crates/pull/src/universe.rs:798: master.len(),
UNCOVERED LINE crates/pull/src/universe.rs:799: index.len(),
UNCOVERED LINE crates/pull/src/universe.rs:800: index.capacity()
UNCOVERED LINE crates/pull/src/upkeep.rs:1581: continue;
UNCOVERED BRANCH crates/pull/src/upkeep.rs:1200:16 False: && why.kind() == ErrorKind::NotFound
UNCOVERED BRANCH crates/pull/src/upkeep.rs:1580:20 True: if !scope.admits(2, &symbol) {
UNCOVERED BRANCH crates/pull/src/upkeep.rs:1676:20 False: if scope.prefix.len() <= 3 && into(walked, what, &path, true) {
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:372: lines[0]
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:380: lines[4]
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:388: lines[6]
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:398: lines[6]
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:460: built.fetch_add(1, Ordering::Relaxed);
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:461: clean(key, &at)
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:462: };
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:1741: built.fetch_add(1, Ordering::Relaxed);
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:1742: clean(key, &at)
UNCOVERED LINE crates/pull/src/upkeep_tests.rs:1743: };
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:472:9 False: why.contains("space") && why.contains("nothing was written"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:472:34 False: why.contains("space") && why.contains("nothing was written"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:521:9 False: why.contains("another srt upkeep run of gdfl holds")
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:522:16 False: && why.contains("one run of a kind at a time"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1001:9 False: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1001:47 True: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1001:47 False: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1042:9 False: why.starts_with(&format!("{}: ", gone.join("audit").display()))
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1043:16 False: && why.ends_with("; nothing was written"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1399:9 False: why.starts_with("NSE/FNO/NIFTY/2019-03-28-1160000-CE names nothing under bars/gdfl: ")
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1400:16 False: && why.ends_with("2019-03-28-1160000-CE is absent"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1517:9 False: why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1517:44 False: why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1553:9 False: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1553:42 False: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1553:72 False: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1555:13 False: assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1555:57 False: assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1657:41 False: if text.starts_with("item ") && !self.refused.load(Ordering::Acquire) {
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1941:9 False: text.contains(
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:1944:14 False: ) && text.contains("end 300 how=stopped n=2 ok=2 of=6 t="),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:2085:9 False: why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
UNCOVERED BRANCH crates/pull/src/upkeep_tests.rs:2085:45 False: why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
UNCOVERED LINE crates/pull/src/vendor.rs:505: Some(Timeframe::MINUTE_30)
UNCOVERED LINE crates/pull/src/vendor.rs:508: Some(Timeframe::MINUTE_60)
UNCOVERED LINE crates/pull/src/vendor.rs:6890: other => panic!("{rung} sits on {other:?}, which the store cannot file"),
UNCOVERED LINE crates/pull/src/vendor.rs:7011: _ => false,
UNCOVERED LINE crates/pull/src/vendor.rs:7082: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7090: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7096: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7121: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7159: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7199: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7258: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7263: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:7742: panic!("Groww is an HTTP feed");
UNCOVERED LINE crates/pull/src/vendor.rs:7760: panic!("Dhan is an HTTP feed");
UNCOVERED LINE crates/pull/src/vendor.rs:8163: panic!(
UNCOVERED LINE crates/pull/src/vendor.rs:8167: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:8174: feed.display()
UNCOVERED LINE crates/pull/src/vendor.rs:8214: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/src/vendor.rs:8425: spec.path_template()
UNCOVERED LINE crates/pull/src/vendor.rs:9133: other.standing.label()
UNCOVERED LINE crates/pull/src/vendor.rs:9399: panic!("Groww is an HTTP feed");
UNCOVERED BRANCH crates/pull/src/vendor.rs:504:31 True: Self::Minute30 if Timeframe::MINUTE_30.aligns_with_the_open() => {
UNCOVERED BRANCH crates/pull/src/vendor.rs:507:28 True: Self::Hour1 if Timeframe::MINUTE_60.aligns_with_the_open() => {
UNCOVERED BRANCH crates/pull/src/vendor.rs:1175:20 True: if interval_secs == 0 || !session_secs.is_multiple_of(interval_secs) {
UNCOVERED BRANCH crates/pull/src/vendor.rs:7088:37 True: !(named_in_query && named_in_path),
UNCOVERED BRANCH crates/pull/src/vendor.rs:7293:17 False: assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
UNCOVERED BRANCH crates/pull/src/vendor.rs:7293:53 False: assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
UNCOVERED BRANCH crates/pull/src/vendor.rs:7317:17 False: assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
UNCOVERED BRANCH crates/pull/src/vendor.rs:7317:56 False: assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
UNCOVERED BRANCH crates/pull/src/vendor.rs:7415:13 False: index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
UNCOVERED BRANCH crates/pull/src/vendor.rs:7415:60 False: index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
UNCOVERED BRANCH crates/pull/src/vendor.rs:7673:39 False: assert!(i == j || one != other, "offset {one} appears twice");
UNCOVERED BRANCH crates/pull/src/vendor.rs:7741:13 False: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/vendor.rs:7759:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/vendor.rs:8132:17 False: !row.granularities.is_empty() && !row.segments.is_empty(),
UNCOVERED BRANCH crates/pull/src/vendor.rs:8132:50 False: !row.granularities.is_empty() && !row.segments.is_empty(),
UNCOVERED BRANCH crates/pull/src/vendor.rs:8213:13 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/vendor.rs:8596:13 False: fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
UNCOVERED BRANCH crates/pull/src/vendor.rs:8596:54 False: fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
UNCOVERED BRANCH crates/pull/src/vendor.rs:8715:13 False: !TRUEDATA_INDEX.columns.contains(&Column::Volume)
UNCOVERED BRANCH crates/pull/src/vendor.rs:8716:20 False: && !TRUEDATA_INDEX.columns.contains(&Column::OpenInterest),
UNCOVERED BRANCH crates/pull/src/vendor.rs:8981:48 False: row.contested.is_none() || !row.binds_because.is_empty(),
UNCOVERED BRANCH crates/pull/src/vendor.rs:9398:13 False: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
UNCOVERED BRANCH crates/pull/src/vendor.rs:9681:13 False: OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
UNCOVERED BRANCH crates/pull/src/vendor.rs:9681:54 False: OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
UNCOVERED BRANCH crates/pull/src/vendor.rs:9709:17 False: assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9709:54 False: assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9734:17 False: assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9734:53 False: assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9760:17 False: assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
UNCOVERED BRANCH crates/pull/src/vendor.rs:9760:45 False: assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
UNCOVERED BRANCH crates/pull/src/vendor.rs:9788:17 False: assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9788:51 False: assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
UNCOVERED BRANCH crates/pull/src/vendor.rs:9889:17 False: assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
UNCOVERED BRANCH crates/pull/src/vendor.rs:9889:45 False: assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
UNCOVERED BRANCH crates/pull/src/vendor.rs:9992:17 False: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
UNCOVERED BRANCH crates/pull/src/vendor.rs:9992:54 False: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
UNCOVERED BRANCH crates/pull/src/vendor.rs:9992:80 False: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
UNCOVERED LINE crates/pull/src/verdict_words.rs:256: cause.code()
UNCOVERED BRANCH crates/pull/src/verdict_words.rs:184:21 False: !word.is_empty()
UNCOVERED BRANCH crates/pull/src/verdict_words.rs:185:28 False: && word
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:177: Err(std::fs::TryLockError::Error(why)) => Err(SpoolError::io(&path, "lock", why)),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:186: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:188: source: io::Error::other(why.to_string()),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:189: })
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:388: Self::Io { .. } => "io",
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:394: fn io(path: &Path, action: &'static str, source: io::Error) -> Self {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:395: Self::Io {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:396: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:397: action,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:398: source,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:399: }
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:400: }
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:407: fn refused(at: &Path, why: store::safe_open::ReadRefusal) -> Self {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:408: match why {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:409: store::safe_open::ReadRefusal::NotRegular { path, .. } => Self::NotRegular { path },
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:411: path,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:412: action,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:413: source,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:414: } => Self::Io {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:415: path,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:416: action,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:417: source,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:418: },
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:420: other => Self::io(at, "open", io::Error::other(other.to_string())),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:422: }
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:448: path,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:449: action,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:450: source,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:451: } => write!(f, "io: {action} {}: {source}", path.display()),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:680: return Err("has reserved bytes that are not zero");
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:693: return Err("holds a key whose hash is not its text's");
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:732: return Err("holds a detail no writer writes");
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:737: _ => Err("names no record kind"),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:826: return Err(SpoolError::Busy {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:827: run: self.run,
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:828: holder: store::flock::holder_note(&path),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:829: });
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:831: Err(std::fs::TryLockError::Error(why)) => {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:832: return Err(SpoolError::io(&path, "lock", why));
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:863: why: format!("day {}: {why}", record.day),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:864: })?;
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:907: continue;
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:967: Err(why) => return Err(SpoolError::io(&path, "stat", why)),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1086: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1087: why: format!("its length {len} does not fit this host's memory"),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1088: })?;
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1096: empty.torn = Some((HEADER_LEN, len - HEADER_LEN));
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1150: return Err(damaged("follows a record that lacks a detail it declares"));
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1168: _ => return Err(damaged("is a detail with no record of its key before it")),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1202: return Ok(end);
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1223: Ok(after)
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1352: Err(why) => return Err(SpoolError::refused(path, why)),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1404: Err(why) => return Err(SpoolError::io(path, "stat", why)),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1406: return Err(SpoolError::NotRegular {
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1407: path: path.to_path_buf(),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1408: });
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1463: out.stray.push(seg_path);
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1464: continue;
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1468: out.stray.push(tf_path);
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1469: continue;
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1513: Err(why) => return Err(SpoolError::io(dir, "list", why)),
UNCOVERED LINE crates/pull/src/zerodha_spool.rs:1527: _ => stray.push(path),
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:627:12 True: if detail.kind == 0 || detail.payload.len() > DETAIL_PAYLOAD {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:674:8 True: if bytes
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:677:12 True: || declared > MAX_DETAILS
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:678:29 True: || (declared > 0 && u8_at(bytes, 149) != KIND_VERDICT)
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:692:16 True: if key.hash() != hash {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:731:16 True: if kind == 0 || len > DETAIL_PAYLOAD {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:731:29 True: if kind == 0 || len > DETAIL_PAYLOAD {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:822:59 False: Err(std::fs::TryLockError::WouldBlock) if started.elapsed() < LIVE_WAIT => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:906:17 False: let Some(entry) = open.get_mut(&id) else {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:958:25 False: Err(why) if why.kind() == ErrorKind::NotFound => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1095:12 True: if len > HEADER_LEN {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1149:20 True: if owed > 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1161:32 False: Some(owner) if owner.key.hash() == hash && owed > 0 => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1165:32 False: Some(owner) if owner.key.hash() == hash => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1201:8 True: if window == 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1351:21 False: Err(why) if why.is_missing() => return Ok(LogRead::default()),
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1398:21 False: Err(why) if why.kind() == ErrorKind::NotFound => OpenOptions::new()
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1405:21 True: Ok(meta) if !meta.file_type().is_file() => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1421:16 False: && let Some(dir) = path.parent()
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1462:17 False: let Some(segment) = segment_named(&seg_name) else {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1467:21 False: let Some(timeframe) = timeframe_named(&tf_name) else {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1512:21 False: Err(why) if why.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1524:36 False: Some(name) if (dirs && kind.is_dir()) || (!dirs && kind.is_file()) => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1524:55 False: Some(name) if (dirs && kind.is_dir()) || (!dirs && kind.is_file()) => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1524:64 False: Some(name) if (dirs && kind.is_dir()) || (!dirs && kind.is_file()) => {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1534:5 False: !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1561:27 True: if year.len() != 4 || number.len() != 2 || !all_digits(year) || !all_digits(number) {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1561:48 True: if year.len() != 4 || number.len() != 2 || !all_digits(year) || !all_digits(number) {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1561:69 True: if year.len() != 4 || number.len() != 2 || !all_digits(year) || !all_digits(number) {
UNCOVERED BRANCH crates/pull/src/zerodha_spool.rs:1571:12 False: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:218: _ => return None,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:303: Err(why) => return Err(format!("{}: {why}", path.display())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:345: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:351: Err(_) => None,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:436: return None;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:656: DayKind::OpenLengthUnmeasured => return Permit::Unsized,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:668: return Permit::Unattested;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:671: return Permit::Unsized;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:679: return Permit::Unsized;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:686: _ => Permit::Unsized,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:769: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:796: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:800: Permit::Closed => continue,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:802: refuse(&mut out, RowCause::Timestamp.code());
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:803: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:806: out.days.entry(number).or_default().withheld = true;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:807: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:811: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:815: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:817: refuse(&mut out, RowCause::Price.code());
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:818: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:844: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:941: return Vec::new();
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1006: not_this_series(&mut facts);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1134: facts.volume_diff += 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1135: differs = true;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1209: facts.flags |= flags::QUALITY_DIFFERS;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1210: details.push(Detail {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1211: kind: words::DETAIL_QUALITY_DIFFERS,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1212: payload: payload(&[
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1213: &1_u16.to_le_bytes(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1214: &landing.to_le_bytes(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1215: &oracle.to_le_bytes(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1216: ]),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1217: });
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1244: worsen(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1245: Code::WithheldByRule,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1246: reason_of("withheld_unmeasured_session"),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1247: &mut code,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1248: &mut reason,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1249: );
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1269: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1301: let word = if i64::from(number) > calendar::SOURCED_LAST_DAY {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1302: "calendar_past_record"
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1304: "calendar_unverified"
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1306: if matches!(code, Code::Match | Code::NoResponseKept) {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1307: code = Code::NotCompared;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1308: reason = reason_of(word);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1309: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1311: Owes::Unsized => worsen(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1312: Code::WithheldByRule,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1313: reason_of("withheld_unmeasured_session"),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1314: &mut code,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1315: &mut reason,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1515: Owes::Closed => None,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1516: Owes::Unattested => return (Vec::new(), Some("calendar_unverified")),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1517: Owes::Unsized => return (Vec::new(), Some("withheld_unmeasured_session")),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1535: bucket.bar.open_interest = bar.open_interest;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1607: facts.code = if word == "calendar_unverified" {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1608: Code::NotCompared
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1610: Code::WithheldByRule
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1612: facts.reason = reason_of(word);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1613: return (facts, details);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1636: facts.oi_diff += 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1637: differs = true;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1647: (false, Some(_)) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1648: facts.extra += 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1649: note_diff(bucket.start, &mut first_diff);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1650: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1671: (Code::ExtraInStore, "unexplained")
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1705: receipt = answer.receipt;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1706: note_shape(answer, &mut details);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1707: let served = answer.bars.get(&0);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1708: facts.source_units = u32::from(served.is_some());
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1709: match (served, inputs.day_bar.as_ref()) {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1710: (Some(want), Some(held)) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1711: facts.value_diff = u32::from(!same_prices(want, held));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1712: facts.volume_diff = u32::from(want.volume != held.volume);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1713: facts.oi_diff = u32::from(want.open_interest != held.open_interest);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1714: if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1715: facts.code = Code::ValueMismatch;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1716: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1718: (Some(_), None) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1719: facts.missing = 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1720: facts.code = Code::MissingInStore;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1721: facts.reason = reason_of("unexplained");
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1722: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1723: (None, Some(_)) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1724: facts.extra = 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1725: facts.code = Code::ExtraInStore;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1726: facts.reason = reason_of("unexplained");
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1727: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1728: (None, None) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1729: facts.code = Code::VendorHole;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1730: facts.reason = reason_of("vendor_empty");
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1731: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1736: facts.flags |= flags::BODY_NOT_KEPT;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1739: facts.missing = 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1740: facts.code = Code::MissingInStore;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1741: facts.reason = reason_of("unexplained");
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1819: fn from(refusal: MonthRefusal) -> Self {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1820: refusal.why
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:1821: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2158: Err(why) => return Err(format!("{}: {why}", dir.display())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2210: Err(why) => return Err(format!("{}: {why}", dir.display())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2219: *stray += 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2220: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2230: None => *stray += 1,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2249: return None;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2268: busy.push(words);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2331: Err(why) => (Err(why.to_string()), Shape::default()),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2349: return (out, faults);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2358: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2359: faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2361: series.key_text(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2362: rung.as_str()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2364: return (out, faults);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2369: faults.push("the Zerodha descriptor is not an HTTP feed".to_owned());
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2370: return (out, faults);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2393: for day in &days {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2394: out.entry(*day).or_default().push(unkept(why));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2395: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2396: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2425: faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2427: series.key_text(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2428: rung.as_str()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2430: Vec::new()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2431: });
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2455: response_keep::Kept::NotKept => Some(match receipt.reason {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2456: response_keep::Reason::StorageFull => "storage_full",
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2457: _ => "pack_full",
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2459: response_keep::Kept::RefusedHead => Some("refused_answer"),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2495: return Ok(None);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2498: return Ok(None);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2632: faults.push(words);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2633: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2741: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2742: faults.push(format!("{key} {day} 1min: {why}"));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2743: (0, Vec::new(), Some(why))
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2770: Err(why) => faults.push(format!("{key} {day} {}: {why}", rung.as_str())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2775: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2776: faults.push(format!("{key} {day} 1day: {why}"));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2777: (None, false)
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2823: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2824: faults.push(format!("{key} {day} gdfl 1s peer: {why}"));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2825: None
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2985: break;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:2998: Err(_) => true,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3001: failed.store(true, Ordering::Relaxed);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3006: break;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3062: Err(_) => record.refusals_total = record.refusals_total.saturating_add(1),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3075: return Err("no cold verification worker could start".to_owned());
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3084: return Err(made
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3085: .err()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3086: .unwrap_or_else(|| "a cold worker answered for no series".to_owned()));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3112: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3113: let _ = telemetry::emit(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3114: &telemetry::Event::warn(TARGET, "fewer workers than asked for")
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3115: .with("asked", telemetry::Value::Uint(want as u64))
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3116: .with("started", telemetry::Value::Uint(handles.len() as u64))
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3117: .with("why", telemetry::Value::Str(&why.to_string())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3118: );
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3119: break;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3149: let words = format!("{key} first day bar: {}", why.why);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3150: if why.busy {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3151: report.busy.push(words);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3152: } else {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3153: report.refused.push(words);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3154: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3155: None
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3156: });
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3161: report.refused.push(format!("{key} 1min months: {why}"));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3162: Vec::new()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3163: });
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3178: report.stopped = true;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3179: return Ok(report);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3195: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3217: Err(_) => break,
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3253: report
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3254: .refused
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3255: .push(format!("{key} {} months: {why}", rung.as_str()));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3256: Vec::new()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3257: });
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3303: {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3304: let _ = telemetry::emit(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3305: &telemetry::Event::error(TARGET, "alert not written")
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3306: .with("why", telemetry::Value::Str(&fault)),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3307: );
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3362: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3375: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3377: continue;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3394: day_of(rungs.daily.as_ref(), number)?.1.first().copied(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3395: rungs.daily.is_some(),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3401: (&[], &answers)
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3450: telemetry::Event::error(TARGET, "month refused to open")
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3463: Err("the Zerodha descriptor is not an HTTP feed".to_owned())
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3552: cut -= 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3553: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3555: record.refusals.remove(0);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3636: Self::Lock(why) => write!(f, "merge_lock_refused: {why}"),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3637: Self::Spool(why) => write!(f, "{why}"),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3731: Err(std::fs::TryLockError::Error(why)) => Err(MergeError::Lock(why.to_string())),
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3755: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3756: if let Some(progress) = progress {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3757: progress_end(progress, false, Some(&why.to_string()));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3758: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3759: return Err(MergeError::Spool(why));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3828: Err(why) => outcome.faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3916: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3917: faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3921: false
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:3945: Err(why) => faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4032: None => outcome.faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4034: id.path(root).display()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4094: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4095: refuse(root, id, &path, (None, true), &why.to_string(), outcome);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4096: return;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4119: refuse(root, id, &path, (None, false), &why, outcome);
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4120: return;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4147: break;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4223: outcome.torn.push((path.to_path_buf(), at, bytes));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4265: Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4266: Err(why) => {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4267: outcome.not_removed.push((file, why.to_string()));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4268: return;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4307: {
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4308: // NAMED, NEVER DROPPED: without the line the next merge tries the
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4309: // spool again, which is safe, but the refusal must not be silent.
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4310: outcome.faults.push(format!(
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4311: "{}: the merge log's refused line was not written: {failed}",
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4312: id.log_path(root).display()
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4313: ));
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4407: return false;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4411: }
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4412: at = image.header.previous;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4413: steps += 1;
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4415: false
UNCOVERED LINE crates/pull/src/zerodha_verify.rs:4429: continue;
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:236:40 False: .filter(|tf| tf.secs() > 60 && tf.secs().is_multiple_of(60) && *tf != Timeframe::DAY_1)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:302:25 False: Err(why) if why.is_missing() => return Ok(()),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:344:16 True: if line.is_empty() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:363:25 False: .filter(|k| !k.is_empty() && !k.contains('='))?;
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:388:25 False: .filter(|k| !k.is_empty() && !k.contains('='))?;
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:435:8 True: if day.len() != 2 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:444:27 True: if year.len() != 4 || month.len() != 2 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:667:9 False: let Ok(day) = Day::from_days(number) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:670:9 False: let Some(venue) = crate::vendor::Venue::for_segment(Exchange::Nse, segment) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:678:13 False: let Ok(hours) = venue.share_hours_on(day, cas) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:685:23 False: Some(span) if span.0 < span.1 => Permit::Windows(([span, (0, 0)], 1)),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:738:9 True: daily
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:768:13 False: let Ok(number) = u32::try_from(day) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:795:12 True: if utc_s < asked.0 || utc_s > asked.1 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:795:31 True: if utc_s < asked.0 || utc_s > asked.1 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:798:12 False: if !daily {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:810:24 True: if !in_windows(&windows, minute) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:816:13 False: let Some(bar) = scaled(row, utc_s) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:820:23 True: let slot = if daily { 0 } else { minute };
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:843:17 False: let (Ok(number), Ok(minute)) = (u32::try_from(day), u32::try_from(minute)) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:857:45 True: if landing_judged != self.judged || u64::from(receipt_refused) != oracle_total {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:940:9 False: let Ok(key) = Key::instrument(&key_text) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:983:41 False: if inputs.beyond_minutes && bars.is_none() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:998:38 False: let owes_no_day_bar = foreign && inputs.day_bar.is_none() && inputs.day_answers.is_empty();
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1001:41 True: } else if (inputs.day_month_held || inputs.day_bar.is_some() || !inputs.day_answers.is_empty())
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1001:69 True: } else if (inputs.day_month_held || inputs.day_bar.is_some() || !inputs.day_answers.is_empty())
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1005:12 True: if foreign {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1133:20 True: if held.volume != bar.volume {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1155:16 False: if second >= from
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1158:20 False: && !refused_at.contains(minute)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1208:16 True: if let Some((landing, oracle)) = answer.quality {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1238:26 False: .any(|a| second >= a.asked.0 && second <= a.asked.1)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1243:12 True: if answer.withheld {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1254:27 True: let differs = later.bars.len() != answer.bars.len()
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1259:16 False: if differs {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1275:48 False: Some(line) if line.origin_store && line.why == reason_of("repaired") => {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1301:27 True: let word = if i64::from(number) > calendar::SOURCED_LAST_DAY {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1301:27 False: let word = if i64::from(number) > calendar::SOURCED_LAST_DAY {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1382:34 False: answered.is_some_and(|a| second >= a.asked.0 && second <= a.asked.1)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1396:16 False: && answer.last_stamp < answer.asked.1
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1468:31 False: "vendor_empty" if !answered => (Code::VendorHole, line.why),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1534:20 True: if bar.open_interest != OI_NULL {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1581:12 False: if let Some(first) = buckets.iter().find(|b| b.complete) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1606:12 True: if let Some(word) = unjudged {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1607:25 True: facts.code = if word == "calendar_unverified" {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1607:25 False: facts.code = if word == "calendar_unverified" {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1635:20 True: if bar.open_interest != bucket.bar.open_interest {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1670:19 True: } else if facts.extra > 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1704:12 True: if let Some(answer) = kept.first() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1714:20 True: if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1714:20 False: if facts.value_diff + facts.volume_diff + facts.oi_diff > 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1735:12 True: if !inputs.day_answers.is_empty() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1738:40 True: if inputs.day_bar.is_none() && !inputs.minutes.is_empty() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1748:16 False: && let Ok(report) = crate::daycheck::compare(inputs.minutes, &[bar])
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:1749:36 True: && (report.differed > 0 || report.contradicted > 0)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2027:12 False: if self.examples.len() < MAX_EXAMPLES
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2051:16 False: if let Some(slot) = self.by_code.get_mut(usize::from(code.code())) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2126:24 False: if holds_a_rung(&contract_path) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2157:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2209:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2218:13 False: let Some(name) = name.to_str() else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2248:8 True: if series.contract.is_some() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2267:16 True: if why.busy {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2348:9 False: let Some(instrument) = series.instrument() else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2367:9 False: let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2392:20 True: if let Some(why) = not_kept_word(receipt) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2399:37 False: if shape.unknown > 0 && !std::mem::replace(&mut shape_alerted, true) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2494:9 False: let Some(month) = open_month(root, series.parts(Timeframe::DAY_1, first))? else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2497:8 True: if month.bars == 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2629:20 False: if why.busy {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2750:17 False: if !open && minutes.is_empty() && unread.is_none() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2750:39 False: if !open && minutes.is_empty() && unread.is_none() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2755:30 False: if minutes.is_empty() && unread.is_none() && lives.first_day.is_some_and(|first| day < first) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2763:30 False: if minutes.is_empty() && unread.is_none() && lives.other_name_owes(day) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2787:12 False: && !held.day_answers.contains_key(&number)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2792:12 False: && derived.iter().all(|(_, bars, _, _)| bars.is_empty())
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:2984:16 True: if failed.load(Ordering::Relaxed) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3000:16 True: if made.is_err() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3005:16 True: if stopped {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3074:8 True: if finished.is_empty() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3150:12 True: if why.busy {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3150:12 False: if why.busy {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3177:12 True: if stop.is_some_and(StopControl::requested) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3194:13 False: let Ok(mut day) = Day::new(month.year(), month.month(), 1) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3298:16 False: && let Some(month) = days_of(window.asked)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3302:16 True: && let Some(fault) = shape_alert(root, window.series, window.rung, month, window.shape)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3361:13 False: let Ok(day) = Day::from_days(number) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3365:12 False: if held.as_ref().is_none_or(|rungs| rungs.month != month) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3371:18 False: (!daily && !series.is_option(), daily),
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3376:13 False: let Some(rungs) = held.as_ref() else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3392:38 True: let (day_bar, day_held) = if daily {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3400:78 True: let (minute_answers, day_answers): (&[DayAnswer], &[DayAnswer]) = if daily {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3428:16 False: if (made.timeframe == Timeframe::DAY_1) == daily {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3447:24 False: let event = if busy {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3551:19 True: while !why.is_char_boundary(cut) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3554:16 True: if record.refusals.len() >= crate::gdfl_run::PROGRESS_SLOTS {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3756:24 True: if let Some(progress) = progress {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3756:24 False: if let Some(progress) = progress {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3989:16 False: if let Some(group) = groups.get_mut(slot) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:3999:21 False: let Some(id) = spools.get(at) else { continue };
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4025:16 False: if let Some(slot) = slots.get_mut(at) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4118:12 True: if let Err(why) = note_seen(&log_path, &mut log, read.records.len()) {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4146:13 False: let Some(((exchange, day, _, _), _)) = keyed.get(start).copied() else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4152:46 False: .is_some_and(|((x, d, _, _), _)| *x == exchange && *d == day)
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4222:12 True: if let Some((at, bytes)) = read.torn {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4230:8 False: if read.torn.is_none() && read.unsynced.is_none() {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4265:25 True: Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4265:25 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4274:15 False: while let Some(at) = dir {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4297:16 True: && let Err(failed) = spool::append_log(
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4405:11 False: while at > base && steps < MAX_WALK_BACK {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4405:24 False: while at > base && steps < MAX_WALK_BACK {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4406:13 False: let Ok(image) = verdict::read_image(&series.image_path(day, at)) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4409:12 False: if image.header.run == run {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4428:21 False: let Some(base) = entries.get_mut(index) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4432:24 True: || base.facts.value_diff != record.facts.value_diff
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4433:24 True: || base.facts.volume_diff != record.facts.volume_diff
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4434:24 True: || base.facts.oi_diff != record.facts.oi_diff
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4435:24 True: || base.facts.missing != record.facts.missing
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4437:31 False: if differs && base.facts.flags & flags::VENDOR_REVISED == 0 {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4503:15 False: }) if covers => {
UNCOVERED BRANCH crates/pull/src/zerodha_verify.rs:4505:31 False: if now != base && written_by(&series, at, (base, now), id.run) {
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:179: Found::NoRecord(none) => panic!("no record: {none:?}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:216: panic!("a regular day owes windows");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:757: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:1562: panic!("the Muhurat of record owes windows");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:2591: other => panic!("a log past the cap is refused by name: {other:?}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:2603: other => panic!("a log of the cap is read: {other:?}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:2955: panic!("Zerodha is an HTTP feed");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:3717: std::fs::set_permissions(&folder, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:3718: }
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:3815: Err(_) => 0,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:4267: first.live[0].2
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:4384: spool::Live::Spooling(holder) => panic!("free run 15 read as live: {holder}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:4770: other => panic!("a full spool took the append: {other:?}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5101: SpoolError::Io { source, .. } => source.kind(),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5102: other => panic!("not the host's words: {other}"),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5103: };
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5108: let resumed = SpoolWriter::new(&root, 53, Mode::ZerodhaMerge, 1);
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5109: let refused = resumed
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5110: .append(
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5111: Segment::Index,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5113: &[spooled(on, "NSE-INDEX-ZZDENZ", Code::Match, 0)],
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5115: .unwrap_err();
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5116: assert_eq!(refused.code(), "io", "{refused}");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5117: assert_eq!(host(refused), std::io::ErrorKind::PermissionDenied);
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5118: drop(resumed);
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5119: assert_eq!(
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5120: host(spool::read_log(&log).unwrap_err()),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5123: let mut read = spool::LogRead::default();
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5124: let record = LogRecord {
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5125: day: 1,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5126: generation: 1,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5127: state: LogState::Committed,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5128: exchange: 1,
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5129: };
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5130: assert_eq!(
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5131: host(spool::append_log(&log, &mut read, record).unwrap_err()),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5135: let link = root.join("verify/linked.log");
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5136: std::os::unix::fs::symlink(&log, &link).unwrap();
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5137: assert!(matches!(
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5138: spool::read_log(&link),
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5141: for name in [&path, &log] {
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5142: std::fs::set_permissions(name, std::fs::Permissions::from_mode(0o644)).unwrap();
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5143: }
UNCOVERED LINE crates/pull/src/zerodha_verify_tests.rs:5144: }
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:209:35 False: DayKind::Open(session) if session == calendar::Session::full()
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:215:9 False: let Owes::Windows(windows) = repair_plan::owed("INDEX", regular().days_from_epoch()) else {
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:755:9 False: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:1560:9 False: let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", muhurat.days_from_epoch())
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:1569:13 False: assert!(!owed.is_empty() && owed.len() < 375);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:1569:33 False: assert!(!owed.is_empty() && owed.len() < 375);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:1643:13 False: assert!(absent.faults.is_empty() && absent.lines == 0);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:1643:41 False: assert!(absent.faults.is_empty() && absent.lines == 0);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:2472:9 False: report.busy[0].contains("5min") && report.busy[0].contains("month_busy"),
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:2472:44 False: report.busy[0].contains("5min") && report.busy[0].contains("month_busy"),
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:2953:9 False: let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:3013:23 False: .find(|r| r.day == on && r.key.text() == "NSE-INDEX-ZZIDX")
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:3614:17 False: assert!(open_before > 0 && open_after > 0, "{old}: both parts trade");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:3614:36 False: assert!(open_before > 0 && open_after > 0, "{old}: both parts trade");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:3787:27 False: .find(|r| r.day == on && r.key.text() == "NSE-INDEX-ZZIDX")
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4091:13 False: assert!(one.faults.is_empty() && four.faults.is_empty(), "{four:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4091:38 False: assert!(one.faults.is_empty() && four.faults.is_empty(), "{four:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4117:13 False: assert!(merge_workers() >= 1 && merge_workers() <= MAX_MERGE_WORKERS);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4117:37 False: assert!(merge_workers() >= 1 && merge_workers() <= MAX_MERGE_WORKERS);
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4269:13 False: assert!(first.faults.is_empty() && first.refused.is_empty());
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4269:40 False: assert!(first.faults.is_empty() && first.refused.is_empty());
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4299:13 False: assert!(last.live.is_empty() && last.waiting == 0, "{last:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4299:37 False: assert!(last.live.is_empty() && last.waiting == 0, "{last:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4481:13 False: assert!(rest.refused.is_empty() && rest.waiting == 0, "{rest:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4481:40 False: assert!(rest.refused.is_empty() && rest.waiting == 0, "{rest:?}");
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:4628:19 False: .find(|r| r.key.text() == "NSE-INDEX-ZZIDX" && r.day == april)
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:5032:13 False: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:5032:42 False: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
UNCOVERED BRANCH crates/pull/src/zerodha_verify_tests.rs:5032:70 False: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
UNCOVERED LINE crates/pull/tests/alerts_seams.rs:59: Put::Duplicate { ordinal } => panic!("a fresh key was a duplicate of #{ordinal}"),
UNCOVERED LINE crates/pull/tests/allocation.rs:173: wide.len()
UNCOVERED LINE crates/pull/tests/allocation.rs:303: panic!("a frame cut to {len} bytes was {:?}", frame_answer.0);
UNCOVERED LINE crates/pull/tests/allocation.rs:323: full.len()
UNCOVERED BRANCH crates/pull/tests/allocation.rs:302:17 False: let Err(FrameError::Truncated { part }) = frame_answer.0 else {
UNCOVERED BRANCH crates/pull/tests/allocation.rs:321:13 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
UNCOVERED BRANCH crates/pull/tests/allocation.rs:321:36 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
UNCOVERED LINE crates/pull/tests/anchor.rs:78: first - OPEN_UTC
UNCOVERED LINE crates/pull/tests/anchor.rs:176: panic!("known session")
UNCOVERED LINE crates/pull/tests/anchor.rs:442: panic!("known session")
UNCOVERED LINE crates/pull/tests/anchor.rs:562: diagnostics[0]
UNCOVERED LINE crates/pull/tests/anchor.rs:639: matches!(
UNCOVERED LINE crates/pull/tests/anchor.rs:725: first - midnight_edge
UNCOVERED BRANCH crates/pull/tests/anchor.rs:175:13 False: let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
UNCOVERED BRANCH crates/pull/tests/anchor.rs:441:9 False: let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
UNCOVERED BRANCH crates/pull/tests/anchor.rs:560:13 False: diagnostics[0].contains("closed") && diagnostics[0].contains("store defect"),
UNCOVERED BRANCH crates/pull/tests/anchor.rs:560:50 False: diagnostics[0].contains("closed") && diagnostics[0].contains("store defect"),
UNCOVERED BRANCH crates/pull/tests/anchor.rs:593:9 False: rendered.contains("-7") && rendered.contains("negative"),
UNCOVERED BRANCH crates/pull/tests/anchor.rs:593:36 False: rendered.contains("-7") && rendered.contains("negative"),
UNCOVERED BRANCH crates/pull/tests/anchor.rs:648:9 False: rendered.contains("Refused rather than saturated") && rendered.contains("an i64"),
UNCOVERED BRANCH crates/pull/tests/anchor.rs:648:63 False: rendered.contains("Refused rather than saturated") && rendered.contains("an i64"),
UNCOVERED LINE crates/pull/tests/broker.rs:115: return;
UNCOVERED LINE crates/pull/tests/broker.rs:397: bars.display()
UNCOVERED LINE crates/pull/tests/broker.rs:765: at("1day").display()
UNCOVERED LINE crates/pull/tests/broker.rs:920: PathSegment::Value { placeholder, .. } => placeholder,
UNCOVERED LINE crates/pull/tests/broker.rs:927: panic!("Groww is an HTTP broker");
UNCOVERED LINE crates/pull/tests/broker.rs:959: panic!("Dhan is an HTTP broker");
UNCOVERED LINE crates/pull/tests/broker.rs:1088: contract.as_str()
UNCOVERED LINE crates/pull/tests/broker.rs:1101: return;
UNCOVERED LINE crates/pull/tests/broker.rs:1275: return;
UNCOVERED LINE crates/pull/tests/broker.rs:1380: continue;
UNCOVERED BRANCH crates/pull/tests/broker.rs:114:13 False: let Ok((mut stream, _)) = socket.accept() else {
UNCOVERED BRANCH crates/pull/tests/broker.rs:891:9 False: why.contains("1min") && why.contains("1day"),
UNCOVERED BRANCH crates/pull/tests/broker.rs:891:33 False: why.contains("1min") && why.contains("1day"),
UNCOVERED BRANCH crates/pull/tests/broker.rs:926:9 False: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
UNCOVERED BRANCH crates/pull/tests/broker.rs:958:9 False: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
UNCOVERED BRANCH crates/pull/tests/broker.rs:1100:9 False: let Ok(entries) = std::fs::read_dir(dir) else {
UNCOVERED BRANCH crates/pull/tests/broker.rs:1274:9 False: let Ok(entries) = std::fs::read_dir(dir) else {
UNCOVERED BRANCH crates/pull/tests/broker.rs:1379:13 False: let Ok(entries) = std::fs::read_dir(&dir) else {
UNCOVERED LINE crates/pull/tests/census.rs:304: path.display()
UNCOVERED LINE crates/pull/tests/census.rs:334: rungs(),
UNCOVERED LINE crates/pull/tests/census.rs:494: rungs()
UNCOVERED LINE crates/pull/tests/census.rs:620: done.failures[0].why
UNCOVERED LINE crates/pull/tests/census.rs:724: done.failures[0].why
UNCOVERED LINE crates/pull/tests/census.rs:743: done.failures[0].why
UNCOVERED LINE crates/pull/tests/census.rs:750: done.failures[0].why
UNCOVERED LINE crates/pull/tests/census.rs:937: done.failures[0].why
UNCOVERED LINE crates/pull/tests/census.rs:1104: panic!(
UNCOVERED LINE crates/pull/tests/census.rs:1127: store.join("bars").display()
UNCOVERED LINE crates/pull/tests/census.rs:1199: n + 2
UNCOVERED LINE crates/pull/tests/census.rs:1222: n + 2,
UNCOVERED BRANCH crates/pull/tests/census.rs:332:9 False: total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:332:45 False: total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:492:9 False: total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:492:33 False: total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:655:13 False: why.contains("BSE") && why.contains("D-0017"),
UNCOVERED BRANCH crates/pull/tests/census.rs:655:36 False: why.contains("BSE") && why.contains("D-0017"),
UNCOVERED BRANCH crates/pull/tests/census.rs:775:9 False: why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
UNCOVERED BRANCH crates/pull/tests/census.rs:775:53 False: why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
UNCOVERED BRANCH crates/pull/tests/census.rs:1028:9 False: total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:1028:38 False: total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:1130:9 False: done.counted == 0 && done.rows_read > 0,
UNCOVERED BRANCH crates/pull/tests/census.rs:1130:30 False: done.counted == 0 && done.rows_read > 0,
UNCOVERED BRANCH crates/pull/tests/census.rs:1371:9 False: total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
UNCOVERED BRANCH crates/pull/tests/census.rs:1371:41 False: total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
UNCOVERED LINE crates/pull/tests/census_day_pace.rs:129: at + 1
UNCOVERED BRANCH crates/pull/tests/census_day_pace.rs:126:13 False: line.contains(&format!("\"read\":\"{read}\""))
UNCOVERED BRANCH crates/pull/tests/census_day_pace.rs:127:20 False: && line.contains(&format!("\"why\":\"{why}\"")),
UNCOVERED BRANCH crates/pull/tests/census_day_pace.rs:132:13 False: line.contains("\"load_us\":") && line.contains("\"total_us\":"),
UNCOVERED BRANCH crates/pull/tests/census_day_pace.rs:132:46 False: line.contains("\"load_us\":") && line.contains("\"total_us\":"),
UNCOVERED LINE crates/pull/tests/census_resume.rs:128: panic!("one CASH row after day one: {:?}", cash_row(root));
UNCOVERED LINE crates/pull/tests/census_resume.rs:147: panic!("{stopped:?}");
UNCOVERED LINE crates/pull/tests/census_resume.rs:191: panic!("one CASH row after the recount: {:?}", cash_row(root));
UNCOVERED LINE crates/pull/tests/census_resume.rs:209: panic!("one CASH row after day three: {:?}", cash_row(root));
UNCOVERED BRANCH crates/pull/tests/census_resume.rs:127:9 False: let [(rows_one, _)] = cash_row(root)[..] else {
UNCOVERED BRANCH crates/pull/tests/census_resume.rs:146:9 False: let ImportRefusal::CensusRefused { day: at, .. } = &stopped else {
UNCOVERED BRANCH crates/pull/tests/census_resume.rs:190:9 False: let [(rows_two, _)] = cash_row(root)[..] else {
UNCOVERED BRANCH crates/pull/tests/census_resume.rs:208:9 False: let [(rows_three, _)] = cash_row(root)[..] else {
UNCOVERED BRANCH crates/pull/tests/census_resume.rs:258:36 False: .is_some_and(|journal| journal.closed.is_empty() && journal.run.is_none()),
UNCOVERED LINE crates/pull/tests/census_v4.rs:866: fn the_audit_scale_census_migrates_and_compacts_with_equal_totals() {
UNCOVERED LINE crates/pull/tests/census_v4.rs:867: let scratch = Scratch::new("SCALE");
UNCOVERED LINE crates/pull/tests/census_v4.rs:868: let root = scratch.root();
UNCOVERED LINE crates/pull/tests/census_v4.rs:869: let keys = 242_341u32;
UNCOVERED LINE crates/pull/tests/census_v4.rs:870: let entries = 2_093_049u32;
UNCOVERED LINE crates/pull/tests/census_v4.rs:871: let index_keys = 2_341u32;
UNCOVERED LINE crates/pull/tests/census_v4.rs:874: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/census_v4.rs:875: let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
UNCOVERED LINE crates/pull/tests/census_v4.rs:876: for i in 0..entries {
UNCOVERED LINE crates/pull/tests/census_v4.rs:877: let n = i % keys;
UNCOVERED LINE crates/pull/tests/census_v4.rs:878: let pass = u64::from(i / keys) + 1;
UNCOVERED LINE crates/pull/tests/census_v4.rs:879: let key = if n < index_keys {
UNCOVERED LINE crates/pull/tests/census_v4.rs:880: index_key(n)
UNCOVERED LINE crates/pull/tests/census_v4.rs:882: option_key(n - index_keys)
UNCOVERED LINE crates/pull/tests/census_v4.rs:884: single.record_held(row(key, pass)).expect("recorded");
UNCOVERED LINE crates/pull/tests/census_v4.rs:886: assert_eq!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:887: (single.entries(), single.keys()),
UNCOVERED LINE crates/pull/tests/census_v4.rs:888: (u64::from(entries), u64::from(keys))
UNCOVERED LINE crates/pull/tests/census_v4.rs:890: let legacy = manifest_path(root, Vendor::Gdfl);
UNCOVERED LINE crates/pull/tests/census_v4.rs:891: std::fs::write(&legacy, single.image()).expect("written");
UNCOVERED LINE crates/pull/tests/census_v4.rs:892: let legacy_bytes = std::fs::read(&legacy).expect("bytes");
UNCOVERED LINE crates/pull/tests/census_v4.rs:893: let built = started.elapsed();
UNCOVERED LINE crates/pull/tests/census_v4.rs:895: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/census_v4.rs:896: let done = census_store::migrate(root, Vendor::Gdfl).expect("it migrates");
UNCOVERED LINE crates/pull/tests/census_v4.rs:897: let migrated = started.elapsed();
UNCOVERED LINE crates/pull/tests/census_v4.rs:900: assert!(done.seal.as_ref().is_some_and(Result::is_ok), "sealed");
UNCOVERED LINE crates/pull/tests/census_v4.rs:901: assert_eq!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:902: std::fs::read(census_store::superseded_path(root, Vendor::Gdfl)).expect("bytes"),
UNCOVERED LINE crates/pull/tests/census_v4.rs:906: let (_, fno) = current(root, Segment::Fno);
UNCOVERED LINE crates/pull/tests/census_v4.rs:907: let (_, index) = current(root, Segment::Index);
UNCOVERED LINE crates/pull/tests/census_v4.rs:908: let (_, cash) = current(root, Segment::Cash);
UNCOVERED LINE crates/pull/tests/census_v4.rs:909: assert_eq!(fno.keys() + index.keys() + cash.keys(), single.keys());
UNCOVERED LINE crates/pull/tests/census_v4.rs:910: assert_eq!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:911: fno.total_rows() + index.total_rows() + cash.total_rows(),
UNCOVERED LINE crates/pull/tests/census_v4.rs:912: single.total_rows()
UNCOVERED LINE crates/pull/tests/census_v4.rs:914: assert_eq!(fno.entries(), fno.keys(), "migrated one row per key");
UNCOVERED LINE crates/pull/tests/census_v4.rs:915: assert_eq!(fno.capacity(), FNO_FLOOR);
UNCOVERED LINE crates/pull/tests/census_v4.rs:916: assert_eq!(fno.headroom(), FNO_FLOOR - fno.keys());
UNCOVERED LINE crates/pull/tests/census_v4.rs:919: let log_keys = 242_000u32;
UNCOVERED LINE crates/pull/tests/census_v4.rs:920: let log_entries = 2_200_000u32;
UNCOVERED LINE crates/pull/tests/census_v4.rs:921: let mut census =
UNCOVERED LINE crates/pull/tests/census_v4.rs:922: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, FNO_FLOOR).expect("genesis");
UNCOVERED LINE crates/pull/tests/census_v4.rs:923: for i in 0..log_entries {
UNCOVERED LINE crates/pull/tests/census_v4.rs:924: census
UNCOVERED LINE crates/pull/tests/census_v4.rs:925: .record_held(row(option_key(i % log_keys), u64::from(i / log_keys) + 1))
UNCOVERED LINE crates/pull/tests/census_v4.rs:926: .expect("recorded");
UNCOVERED LINE crates/pull/tests/census_v4.rs:927: }
UNCOVERED LINE crates/pull/tests/census_v4.rs:928: assert!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:929: census.entries() > MAX_ENTRIES,
UNCOVERED LINE crates/pull/tests/census_v4.rs:932: let path = install(root, Segment::Fno, 2, &census);
UNCOVERED LINE crates/pull/tests/census_v4.rs:933: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/census_v4.rs:934: let loaded = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
UNCOVERED LINE crates/pull/tests/census_v4.rs:935: let read = started.elapsed();
UNCOVERED LINE crates/pull/tests/census_v4.rs:936: assert_eq!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:937: (loaded.entries(), loaded.keys(), loaded.total_rows()),
UNCOVERED LINE crates/pull/tests/census_v4.rs:938: (census.entries(), census.keys(), census.total_rows())
UNCOVERED LINE crates/pull/tests/census_v4.rs:940: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/census_v4.rs:941: let compacted = loaded.compacted().expect("it compacts");
UNCOVERED LINE crates/pull/tests/census_v4.rs:942: let compact = started.elapsed();
UNCOVERED LINE crates/pull/tests/census_v4.rs:943: assert_eq!(compacted.entries(), u64::from(log_keys));
UNCOVERED LINE crates/pull/tests/census_v4.rs:944: assert_eq!(compacted.total_rows(), census.total_rows());
UNCOVERED LINE crates/pull/tests/census_v4.rs:945: let path = install(root, Segment::Fno, 3, &compacted);
UNCOVERED LINE crates/pull/tests/census_v4.rs:946: let started = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/census_v4.rs:947: let reopened = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
UNCOVERED LINE crates/pull/tests/census_v4.rs:948: let reread = started.elapsed();
UNCOVERED LINE crates/pull/tests/census_v4.rs:949: assert_eq!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:950: (reopened.entries(), reopened.keys(), reopened.total_rows()),
UNCOVERED LINE crates/pull/tests/census_v4.rs:951: (u64::from(log_keys), census.keys(), census.total_rows())
UNCOVERED LINE crates/pull/tests/census_v4.rs:953: println!(
UNCOVERED LINE crates/pull/tests/census_v4.rs:958: fno.keys(),
UNCOVERED LINE crates/pull/tests/census_v4.rs:959: index.keys(),
UNCOVERED LINE crates/pull/tests/census_v4.rs:960: cash.keys(),
UNCOVERED LINE crates/pull/tests/census_v4.rs:961: done.partitions
UNCOVERED LINE crates/pull/tests/census_v4.rs:962: .iter()
UNCOVERED LINE crates/pull/tests/census_v4.rs:963: .find(|part| part.segment == Segment::Fno)
UNCOVERED LINE crates/pull/tests/census_v4.rs:964: .map_or(0, |part| part.bytes),
UNCOVERED LINE crates/pull/tests/census_v4.rs:965: HEADER_LEN + u64::from(log_entries) * 128,
UNCOVERED LINE crates/pull/tests/census_v4.rs:966: std::fs::metadata(&path).expect("its metadata").len()
UNCOVERED LINE crates/pull/tests/census_v4.rs:968: }
UNCOVERED BRANCH crates/pull/tests/census_v4.rs:879:22 True: let key = if n < index_keys {
UNCOVERED BRANCH crates/pull/tests/census_v4.rs:879:22 False: let key = if n < index_keys {
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:307: other => panic!("a rotted record read as {other:?}"),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:373: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:402: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:844: tried.push(format!("{}: {why}", link.display()));
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:845: continue;
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:851: Err(why) => tried.push(format!("{}: {why}", link.display())),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:854: panic!(
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:856: path.display(),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:857: tried.join("; ")
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:884: link.display()
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:909: other => panic!("{other:?}"),
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1446: fn fetch(
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1447: &self,
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1448: _listing: &DayListing<()>,
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1449: _file: &ListedFile<()>,
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1450: ) -> Result<Vec<u8>, CmRefusal> {
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1451: Err(CmRefusal::NotSwept)
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1452: }
UNCOVERED LINE crates/pull/tests/cloud_mutants_import.rs:1752: lines[0]
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:335:13 False: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:335:56 False: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:497:9 False: said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:498:16 False: && said.contains("Refused rather than saturated"),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:843:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:872:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:976:9 False: gap.starts_with("gap NSE-INDEX-NIFTY 2025-07-01 v=1 rung=1min owed=375 held=374 missing=1")
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:977:16 False: && gap.ends_with("why=vendor_empty run=4242"),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1269:26 True: .any(|m| m.contains(".BE") || m.contains("ZZSHARE"))
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1339:82 False: matches!(listing.auction_for(&cash("TCS")), Auction::Unknown(why) if !why.is_empty())
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1343:38 False: Auction::Unknown(why) if why.contains("records no ISIN")
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1415:38 False: Auction::Unknown(why) if why.contains("core::universe records no ISIN")
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1750:13 False: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
UNCOVERED BRANCH crates/pull/tests/cloud_mutants_import.rs:1750:56 False: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
UNCOVERED LINE crates/pull/tests/derive.rs:164: tf.as_str()
UNCOVERED LINE crates/pull/tests/derive.rs:314: Err(_already) => telemetry::global().expect("install named the sink that exists"),
UNCOVERED LINE crates/pull/tests/derive.rs:407: conflicts[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:414: conflicts[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:612: refusal.why.len(),
UNCOVERED LINE crates/pull/tests/derive.rs:613: refusal.why.get(..300)
UNCOVERED LINE crates/pull/tests/derive.rs:687: reports[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:744: audit[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:876: gaps[1]
UNCOVERED LINE crates/pull/tests/derive.rs:973: done.failures[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:1047: done.failures[0].why
UNCOVERED LINE crates/pull/tests/derive.rs:1466: tf.as_str()
UNCOVERED BRANCH crates/pull/tests/derive.rs:284:13 False: assert!(fut.is_future() && !fut.is_option());
UNCOVERED BRANCH crates/pull/tests/derive.rs:284:32 False: assert!(fut.is_future() && !fut.is_option());
UNCOVERED BRANCH crates/pull/tests/derive.rs:285:13 False: assert!(ce.is_option() && pe.is_option());
UNCOVERED BRANCH crates/pull/tests/derive.rs:285:31 False: assert!(ce.is_option() && pe.is_option());
UNCOVERED BRANCH crates/pull/tests/derive.rs:422:22 False: .all(|f| f.why.contains("incomplete or invalid minute coverage")
UNCOVERED BRANCH crates/pull/tests/derive.rs:468:22 False: .any(|f| f.why.contains("UNVERIFIED") && f.why.contains("cash eligibility"))
UNCOVERED BRANCH crates/pull/tests/derive.rs:736:9 False: audit[0]
UNCOVERED BRANCH crates/pull/tests/derive.rs:739:16 False: && audit[0]
UNCOVERED BRANCH crates/pull/tests/derive.rs:742:16 False: && audit[0].why.contains("observed trading does not attest"),
UNCOVERED BRANCH crates/pull/tests/derive.rs:747:9 False: audit[1].why.contains("2027-01-07")
UNCOVERED BRANCH crates/pull/tests/derive.rs:748:16 False: && audit[1]
UNCOVERED BRANCH crates/pull/tests/derive.rs:755:22 False: .any(|f| !f.why.starts_with("request minute coverage")
UNCOVERED BRANCH crates/pull/tests/derive.rs:1312:42 False: assert!(done.failures.iter().all(|f| !f.why.contains("versioned")
UNCOVERED BRANCH crates/pull/tests/derive.rs:1396:20 False: && f.why.contains(expected)
UNCOVERED BRANCH crates/pull/tests/derive.rs:1736:9 False: per_day.contains("read_entry(root, day)") && per_day.contains("decode(&bytes)"),
UNCOVERED BRANCH crates/pull/tests/derive.rs:1736:54 False: per_day.contains("read_entry(root, day)") && per_day.contains("decode(&bytes)"),
UNCOVERED BRANCH crates/pull/tests/derive.rs:1973:21 False: .filter(|f| f.why.contains("NIFTY: 2020-12: ") && f.why.contains("this month only"))
UNCOVERED LINE crates/pull/tests/folder.rs:269: panic!("a folder holding two days must report them: {reach:?}")
UNCOVERED LINE crates/pull/tests/folder.rs:459: panic!("{reach:?}")
UNCOVERED LINE crates/pull/tests/folder.rs:925: panic!("{:?}", census.reach)
UNCOVERED BRANCH crates/pull/tests/folder.rs:188:67 False: assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Dhan));
UNCOVERED BRANCH crates/pull/tests/folder.rs:262:9 False: let Reach::Days {
UNCOVERED BRANCH crates/pull/tests/folder.rs:394:67 False: assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Groww));
UNCOVERED BRANCH crates/pull/tests/folder.rs:418:64 False: assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
UNCOVERED BRANCH crates/pull/tests/folder.rs:455:9 False: let Reach::Days {
UNCOVERED BRANCH crates/pull/tests/folder.rs:751:64 False: assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
UNCOVERED BRANCH crates/pull/tests/folder.rs:921:9 False: let Reach::Days {
UNCOVERED LINE crates/pull/tests/futures_events.rs:90: panic!("Zerodha is an HTTP feed");
UNCOVERED BRANCH crates/pull/tests/futures_events.rs:88:9 False: let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED BRANCH crates/pull/tests/futures_events.rs:207:13 False: line.contains("\"line\":\"ZQIDX 2023-11-30-FUT: ") && line.contains("\"detail\":\""),
UNCOVERED BRANCH crates/pull/tests/futures_events.rs:207:67 False: line.contains("\"line\":\"ZQIDX 2023-11-30-FUT: ") && line.contains("\"detail\":\""),
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:230: lines[0]
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:235: lines[0]
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:293: lines[0]
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:336: lines[0]
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:341: lines[0]
UNCOVERED LINE crates/pull/tests/gdfl_bad_row_audit.rs:346: || lines[0].contains("pull.gdfl.format")
UNCOVERED BRANCH crates/pull/tests/gdfl_bad_row_audit.rs:345:9 False: lines[0].contains("\"target\":\"pull.gdfl.format\"")
UNCOVERED BRANCH crates/pull/tests/gdfl_bad_row_audit.rs:346:16 True: || lines[0].contains("pull.gdfl.format")
UNCOVERED BRANCH crates/pull/tests/gdfl_bad_row_audit.rs:346:16 False: || lines[0].contains("pull.gdfl.format")
UNCOVERED LINE crates/pull/tests/gdfl_census_stop.rs:117: panic!("{got:?}");
UNCOVERED LINE crates/pull/tests/gdfl_census_stop.rs:177: stop[0]
UNCOVERED LINE crates/pull/tests/gdfl_census_stop.rs:182: stop[0]
UNCOVERED LINE crates/pull/tests/gdfl_census_stop.rs:214: imported[0]
UNCOVERED LINE crates/pull/tests/gdfl_census_stop.rs:221: refused[0]
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:116:9 False: let ImportRefusal::CensusRefused { day: at, done, .. } = &got else {
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:125:30 False: .is_some_and(|l| l.starts_with("incomplete stocks 2024-04-01 * ")
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:175:9 False: stop[0].contains("\"error\"") && stop[0].contains("\"2024-04-01\""),
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:175:42 False: stop[0].contains("\"error\"") && stop[0].contains("\"2024-04-01\""),
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:212:9 False: imported[0].contains("\"done_with_refusals\"") && imported[0].contains("\"info\""),
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:212:59 False: imported[0].contains("\"done_with_refusals\"") && imported[0].contains("\"info\""),
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:219:9 False: refused[0].contains("\"warn\"") && refused[0].contains("\"refused\""),
UNCOVERED BRANCH crates/pull/tests/gdfl_census_stop.rs:219:44 False: refused[0].contains("\"warn\"") && refused[0].contains("\"refused\""),
UNCOVERED LINE crates/pull/tests/integration.rs:326: at.day()
UNCOVERED LINE crates/pull/tests/no_scheduler_delete.rs:150: || code[line_start..].trim_start().starts_with("use ")
UNCOVERED LINE crates/pull/tests/no_scheduler_delete.rs:190: }
UNCOVERED LINE crates/pull/tests/no_scheduler_delete.rs:193: out.push('\n');
UNCOVERED LINE crates/pull/tests/no_scheduler_delete.rs:218: break;
UNCOVERED LINE crates/pull/tests/no_scheduler_delete.rs:225: out.push('\n');
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:149:20 False: || before.ends_with("::")
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:153:13 True: before.ends_with("libc::")
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:176:19 False: while i < chars.len() && chars[i] != '\n' {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:181:19 False: while i < chars.len() {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:182:39 False: if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:185:46 False: } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:188:24 False: if depth == 0 {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:192:24 True: if chars[i] == '\n' {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:199:17 True: && (i == 0 || !ident(chars[i - 1]))
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:208:32 True: let mut j = i + if c == 'b' { 2 } else { 1 };
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:217:20 True: if j >= chars.len() {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:224:20 True: if chars[j] == '\n' {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:233:19 False: while i < chars.len() && chars[i] != '"' {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:245:19 False: while i < chars.len() && chars[i] != '\'' {
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:290:30 False: ended |= opened && depth == 0;
UNCOVERED BRANCH crates/pull/tests/no_scheduler_delete.rs:310:19 False: while i < bytes.len() && word(bytes[i]) {
UNCOVERED LINE crates/pull/tests/padded_refold.rs:290: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:350: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:381: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:400: journal[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:405: journal[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:446: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:459: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:464: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:474: journal[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:516: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:522: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:578: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:598: refused[0]
UNCOVERED LINE crates/pull/tests/padded_refold.rs:620: refused[0]
UNCOVERED LINE crates/pull/tests/pipeline.rs:192: cases[4]
UNCOVERED LINE crates/pull/tests/pipeline.rs:372: Ok((members, rejected)) => panic!(
UNCOVERED LINE crates/pull/tests/pipeline.rs:375: members.len(),
UNCOVERED LINE crates/pull/tests/pipeline.rs:376: rejected.len(),
UNCOVERED LINE crates/pull/tests/pipeline.rs:377: members.len() + rejected.len()
UNCOVERED LINE crates/pull/tests/pipeline.rs:437: wide.why.len(),
UNCOVERED LINE crates/pull/tests/pipeline.rs:438: wide.why.get(wide.why.len().saturating_sub(48)..)
UNCOVERED LINE crates/pull/tests/pipeline.rs:443: wide.why.capacity()
UNCOVERED LINE crates/pull/tests/pipeline.rs:517: cases[4]
UNCOVERED LINE crates/pull/tests/pipeline.rs:726: cases[4]
UNCOVERED LINE crates/pull/tests/pipeline.rs:1568: done.failures[0].why
UNCOVERED LINE crates/pull/tests/pipeline.rs:1594: done.failures[0].why
UNCOVERED LINE crates/pull/tests/pipeline.rs:1657: continue;
UNCOVERED LINE crates/pull/tests/pipeline.rs:1701: done.failures[0].why
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:185:9 False: capped.contains('7') && capped.contains(&MAX_MEMBERS.to_string()),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:185:33 False: capped.contains('7') && capped.contains(&MAX_MEMBERS.to_string()),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:944:9 False: group.why.contains("row 1") && group.why.contains("before the Unix epoch"),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:944:40 False: group.why.contains("row 1") && group.why.contains("before the Unix epoch"),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1028:9 False: text.contains(&120_000_000.to_string())
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1029:16 False: && text.contains(&60_000_000.to_string())
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1030:16 False: && text.contains('1'),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1487:16 False: if path.ends_with("ZZZZ.csv")),
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1656:13 False: let Ok(entries) = std::fs::read_dir(&at) else {
UNCOVERED BRANCH crates/pull/tests/pipeline.rs:1664:24 False: && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
UNCOVERED LINE crates/pull/tests/refusal.rs:554: feed.display()
UNCOVERED LINE crates/pull/tests/refusal.rs:588: feed.display()
UNCOVERED LINE crates/pull/tests/refusal.rs:645: other => panic!(
UNCOVERED LINE crates/pull/tests/refusal.rs:650: other.master_columns().isin
UNCOVERED LINE crates/pull/tests/refusal.rs:658: vendor.master_columns().isin
UNCOVERED LINE crates/pull/tests/refusal.rs:693: panic!("Zerodha is an HTTP feed");
UNCOVERED BRANCH crates/pull/tests/refusal.rs:691:9 False: let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
UNCOVERED LINE crates/pull/tests/request_gap_events.rs:145: lines[0]
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:138:9 False: lines[0].contains("\"count\":2")
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:139:16 False: && lines[0].contains("\"first_day\":\"2025-07-01\"")
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:140:16 False: && lines[0].contains("\"last_day\":\"2025-07-01\"")
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:141:16 False: && lines[0].contains("\"month\":\"2025-07\"")
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:142:16 False: && lines[0].contains("\"code\":\"vendor_empty\"")
UNCOVERED BRANCH crates/pull/tests/request_gap_events.rs:143:16 False: && lines[0].contains("\"feed\":\"truedata\""),
UNCOVERED BRANCH crates/pull/tests/sealed_month.rs:437:9 False: words.contains("repair revision 3") && words.contains("restates"),
UNCOVERED BRANCH crates/pull/tests/sealed_month.rs:437:48 False: words.contains("repair revision 3") && words.contains("restates"),
UNCOVERED BRANCH crates/pull/tests/sealed_month.rs:497:9 False: words.contains("sealed") && words.contains("restates"),
UNCOVERED BRANCH crates/pull/tests/sealed_month.rs:497:37 False: words.contains("sealed") && words.contains("restates"),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:30: fn store_root() -> PathBuf {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:31: PathBuf::from(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:32: std::env::var_os("BRUTEX_EVIDENCE_STORE")
UNCOVERED LINE crates/pull/tests/store_evidence.rs:33: .expect("BRUTEX_EVIDENCE_STORE names the store root to read"),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:35: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:38: fn read_month(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:39: root: &Path,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:40: segment: &str,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:41: symbol: &str,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:42: tf: Timeframe,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:43: month: YearMonth,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:44: ) -> Vec<store::format::Bar> {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:45: let path = StorePath::new(PathParts {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:46: vendor: Vendor::Zerodha,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:47: exchange: "NSE",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:48: segment,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:49: symbol,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:50: contract: None,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:51: timeframe: tf,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:52: month,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:53: file: FileKind::Bars,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:54: })
UNCOVERED LINE crates/pull/tests/store_evidence.rs:55: .unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:56: let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:57: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:58: return Vec::new();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:60: (0..file.header().n_valid)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:61: .map(|i| file.read_record(i).unwrap())
UNCOVERED LINE crates/pull/tests/store_evidence.rs:62: .collect()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:63: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:66: fn on_day(day: Day, bars: &[store::format::Bar]) -> usize {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:67: let midnight = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
UNCOVERED LINE crates/pull/tests/store_evidence.rs:68: bars.iter()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:69: .filter(|b| (midnight..midnight + 86_400_000_000).contains(&b.ts_micros))
UNCOVERED LINE crates/pull/tests/store_evidence.rs:70: .count()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:71: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:81: fn drop(&mut self) {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:82: let _best_effort = fs::remove_dir_all(&self.scratch);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:83: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:86: fn replay(root: &Path, segment: &str, symbol: &str, month: YearMonth) -> Replay {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:87: let minutes = read_month(root, segment, symbol, Timeframe::MINUTE_1, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:88: assert!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:89: !minutes.is_empty(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:92: let rows: Vec<pull::fetch::RawRow> = minutes
UNCOVERED LINE crates/pull/tests/store_evidence.rs:93: .iter()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:94: .map(|b| pull::fetch::RawRow {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:95: timestamp: b.ts_micros.div_euclid(1_000_000),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:96: open: b.open,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:97: high: b.high,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:98: low: b.low,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:99: close: b.close,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:100: volume: b.volume,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:101: open_interest: None,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:102: })
UNCOVERED LINE crates/pull/tests/store_evidence.rs:103: .collect();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:104: let first = Day::new(month.year(), month.month(), 1).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:105: let request = pull::fetch::BarRequest {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:106: instrument_id: String::new(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:107: listing: if segment == "INDEX" {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:108: pull::vendor::Listing::Index
UNCOVERED LINE crates/pull/tests/store_evidence.rs:110: pull::vendor::Listing::Equity
UNCOVERED LINE crates/pull/tests/store_evidence.rs:112: window: Window::new(first, first.end_of_month()).unwrap(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:113: granularity: pull::vendor::Granularity::Minute1,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:115: let plan = pull::ingest::Plan {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:116: calendar: pull::calendar::Runtime::default(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:117: cash_schedule: None,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:118: columns: pull::csv::Columns::Gdfl,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:119: request: &request,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:120: encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:121: scale: pull::vendor::PriceScale::Paisa,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:122: vendor: Vendor::Zerodha,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:123: exchange: "NSE",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:124: segment,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:125: contract: None,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:126: run: 0,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:127: };
UNCOVERED LINE crates/pull/tests/store_evidence.rs:128: let scratch = std::env::temp_dir().join(format!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:129: "brutex-EVIDENCE-{symbol}-{month}-{}",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:130: std::process::id()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:131: ));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:132: let _fresh = fs::remove_dir_all(&scratch);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:133: fs::create_dir_all(&scratch).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:134: let done = pull::ingest::from_window(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:135: &pull::fetch::RawWindow { rows },
UNCOVERED LINE crates/pull/tests/store_evidence.rs:136: symbol,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:137: "STORE EVIDENCE",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:138: &scratch,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:139: plan,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:141: let ledger = gap_ledger(&scratch);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:142: Replay {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:143: scratch,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:144: done,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:145: ledger,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:146: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:147: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:206: fn owed_days(month: YearMonth, minutes: &[store::format::Bar]) -> Vec<(Day, Owed)> {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:207: let first = Day::new(month.year(), month.month(), 1).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:208: let calendar = pull::calendar::Runtime::default();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:209: let mut out = Vec::new();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:210: let mut day = first;
UNCOVERED LINE crates/pull/tests/store_evidence.rs:211: while day.month() == first.month() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:212: let at = i64::from(day.days_from_epoch());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:213: let stored = on_day(day, minutes);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:214: match pull::session::with_recorded_window(at, calendar.kind_of(at)) {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:215: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:216: out.push((
UNCOVERED LINE crates/pull/tests/store_evidence.rs:217: day,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:218: Owed::Short {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:219: owed: session.bars(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:220: stored,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:221: },
UNCOVERED LINE crates/pull/tests/store_evidence.rs:222: ));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:223: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:224: pull::calendar::DayKind::OpenLengthUnmeasured => {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:225: out.push((day, Owed::Unsized { stored }));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:226: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:229: | pull::calendar::DayKind::Unmeasured => {}
UNCOVERED LINE crates/pull/tests/store_evidence.rs:231: day = day.succ().unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:233: out
UNCOVERED LINE crates/pull/tests/store_evidence.rs:234: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:237: fn gap_lines<'a>(ledger: &'a str, key: &str, day: Day) -> Vec<&'a str> {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:238: let prefix = format!("gap {key} {day} ");
UNCOVERED LINE crates/pull/tests/store_evidence.rs:239: ledger
UNCOVERED LINE crates/pull/tests/store_evidence.rs:240: .lines()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:241: .filter(|line| line.starts_with(&prefix))
UNCOVERED LINE crates/pull/tests/store_evidence.rs:242: .collect()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:243: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:245: fn derive_line<'a>(ledger: &'a str, rung: &str) -> &'a str {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:246: ledger
UNCOVERED LINE crates/pull/tests/store_evidence.rs:247: .lines()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:248: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
UNCOVERED LINE crates/pull/tests/store_evidence.rs:249: .unwrap_or("(no derive line)")
UNCOVERED LINE crates/pull/tests/store_evidence.rs:250: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:255: fn banknifty_2024_03_02_derives_from_its_stored_minutes() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:256: let root = store_root();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:257: let month = YearMonth::new(2024, 3).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:258: let saturday = Day::new(2024, 3, 2).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:259: let stored_minutes = read_month(&root, "INDEX", "BANKNIFTY", Timeframe::MINUTE_1, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:260: let replay = replay(&root, "INDEX", "BANKNIFTY", month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:261: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:263: on_day(saturday, &stored_minutes)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:265: for tf in [
UNCOVERED LINE crates/pull/tests/store_evidence.rs:266: Timeframe::MINUTE_2,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:267: Timeframe::MINUTE_5,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:268: Timeframe::MINUTE_15,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:269: Timeframe::MINUTE_60,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:270: ] {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:271: let stored = read_month(&root, "INDEX", "BANKNIFTY", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:272: let derived = read_month(&replay.scratch, "INDEX", "BANKNIFTY", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:273: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:274: "BANKNIFTY 2024-03 {}: stored {} bars ({} on 03-02); this build derives {} ({} on 03-02)",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:275: tf.as_str(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:276: stored.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:277: on_day(saturday, &stored),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:278: derived.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:279: on_day(saturday, &derived)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:280: );
UNCOVERED LINE crates/pull/tests/store_evidence.rs:281: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:282: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:283: let derived = read_month(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:284: &replay.scratch,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:285: "INDEX",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:286: "BANKNIFTY",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:288: month,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:290: assert_eq!(on_day(saturday, &stored_minutes), 105);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:291: assert_eq!(on_day(saturday, &derived), 21);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:292: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:298: fn nifty_2026_09_derives_every_session() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:299: let root = store_root();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:300: let month = YearMonth::new(2026, 9).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:301: let replay = replay(&root, "INDEX", "NIFTY", month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:302: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:303: let stored = read_month(&root, "INDEX", "NIFTY", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:304: let derived = read_month(&replay.scratch, "INDEX", "NIFTY", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:305: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:307: tf.as_str(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:308: stored.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:309: stored.last().map_or(0, |b| b.close),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:310: derived.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:311: derived.last().map_or(0, |b| b.close)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:314: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:315: eprintln!("failures: {}", replay.done.failures.len());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:316: for failure in replay.done.failures.iter().take(5) {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:317: eprintln!("  {}", failure.why.chars().take(240).collect::<String>());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:318: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:319: assert!(derive_line(&replay.ledger, "5min").contains(" days=21 "));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:320: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:332: fn the_gap_ledger_names_each_short_day_of_a_stored_month() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:333: let root = store_root();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:334: let calendar = pull::calendar::Runtime::default();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:335: let mut checked = 0_usize;
UNCOVERED LINE crates/pull/tests/store_evidence.rs:336: let mut special = Vec::new();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:337: for (symbol, year, month) in [("NIFTY", 2020, 11), ("NIFTY", 2024, 11), ("NIFTY", 2021, 2)] {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:338: let ym = YearMonth::new(year, month).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:339: let minutes = read_month(&root, "INDEX", symbol, Timeframe::MINUTE_1, ym);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:340: let replay = replay(&root, "INDEX", symbol, ym);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:341: let key = format!("NSE-INDEX-{symbol}");
UNCOVERED LINE crates/pull/tests/store_evidence.rs:342: let owed = owed_days(ym, &minutes);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:343: let gaps = replay
UNCOVERED LINE crates/pull/tests/store_evidence.rs:344: .ledger
UNCOVERED LINE crates/pull/tests/store_evidence.rs:345: .lines()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:346: .filter(|line| line.starts_with(&format!("gap {key} ")))
UNCOVERED LINE crates/pull/tests/store_evidence.rs:347: .count();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:348: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:350: owed.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:351: replay.done.failures.len()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:353: for (day, what) in &owed {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:354: let named = gap_lines(&replay.ledger, &key, *day);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:355: eprintln!("  {day} {what:?}: {named:?}");
UNCOVERED LINE crates/pull/tests/store_evidence.rs:356: assert_eq!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:357: named.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:360: named.len()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:362: if let Owed::Unsized { .. } = what {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:363: assert!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:364: named[0].contains(" why=withheld_unmeasured_session "),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:366: named[0]
UNCOVERED LINE crates/pull/tests/store_evidence.rs:368: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:369: let at = i64::from(day.days_from_epoch());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:370: let regular = matches!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:371: calendar.kind_of(at),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:372: pull::calendar::DayKind::Open(session)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:373: if session == pull::calendar::Session::full()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:375: if !regular {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:376: special.push(*day);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:377: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:378: checked += 1;
UNCOVERED LINE crates/pull/tests/store_evidence.rs:382: let mut day = Day::new(year, month, 1).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:383: while day.month() == month {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:384: let named = gap_lines(&replay.ledger, &key, day);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:385: let expected = usize::from(owed.iter().any(|(owed_day, _)| *owed_day == day));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:386: assert_eq!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:387: named.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:391: named.len()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:393: day = day.succ().unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:395: eprintln!("  {}", derive_line(&replay.ledger, "5min"));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:400: eprintln!("{checked} day(s) checked; special sessions among them: {special:?}");
UNCOVERED LINE crates/pull/tests/store_evidence.rs:401: assert!(checked > 0, "no owed day in three months chosen for theirs");
UNCOVERED LINE crates/pull/tests/store_evidence.rs:402: for year in [2020, 2024] {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:403: assert!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:404: special
UNCOVERED LINE crates/pull/tests/store_evidence.rs:405: .iter()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:406: .any(|day| day.year() == year && day.month() == 11),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:410: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:415: fn an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:416: let root = store_root();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:417: let month = YearMonth::new(2023, 2).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:418: let replay = replay(&root, "CASH", "360ONE", month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:419: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:420: let stored = read_month(&root, "CASH", "360ONE", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:421: let derived = read_month(&replay.scratch, "CASH", "360ONE", tf, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:422: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:423: "360ONE 2023-02 {}: stored {} bars; this build derives {}",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:424: tf.as_str(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:425: stored.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:426: derived.len()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:427: );
UNCOVERED LINE crates/pull/tests/store_evidence.rs:428: eprintln!("  {}", derive_line(&replay.ledger, tf.as_str()));
UNCOVERED LINE crates/pull/tests/store_evidence.rs:429: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:430: let gaps = replay
UNCOVERED LINE crates/pull/tests/store_evidence.rs:431: .ledger
UNCOVERED LINE crates/pull/tests/store_evidence.rs:432: .lines()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:433: .filter(|l| l.starts_with("gap "))
UNCOVERED LINE crates/pull/tests/store_evidence.rs:434: .count();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:435: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:437: replay.done.failures.len()
UNCOVERED LINE crates/pull/tests/store_evidence.rs:439: let stored = read_month(&root, "CASH", "360ONE", Timeframe::MINUTE_60, month);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:440: let derived = read_month(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:441: &replay.scratch,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:442: "CASH",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:443: "360ONE",
UNCOVERED LINE crates/pull/tests/store_evidence.rs:445: month,
UNCOVERED LINE crates/pull/tests/store_evidence.rs:447: assert!(derived.len() > stored.len());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:448: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:456: fn the_derive_fold_costs_per_month() {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:457: let root = store_root();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:458: for (segment, symbol, year, month) in [("INDEX", "NIFTY", 2026, 9), ("CASH", "360ONE", 2023, 2)]
UNCOVERED LINE crates/pull/tests/store_evidence.rs:460: let ym = YearMonth::new(year, month).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:461: let minutes = read_month(&root, segment, symbol, Timeframe::MINUTE_1, ym);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:462: let venue = if segment == "INDEX" {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:463: pull::vendor::Venue::NseIndex
UNCOVERED LINE crates/pull/tests/store_evidence.rs:465: pull::vendor::Venue::NseCash
UNCOVERED LINE crates/pull/tests/store_evidence.rs:467: let bucket = pull::fold::Bucket::of_secs(300).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:468: let runtime = pull::calendar::Runtime::default();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:469: let mut aggregated = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:470: let mut detailed = Vec::with_capacity(200);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:471: for _ in 0..200 {
UNCOVERED LINE crates/pull/tests/store_evidence.rs:472: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:473: let derived =
UNCOVERED LINE crates/pull/tests/store_evidence.rs:474: pull::fold::derive_minutes(&minutes, bucket, venue, None, runtime).unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:475: aggregated.push(start.elapsed());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:476: std::hint::black_box(derived);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:477: let start = std::time::Instant::now();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:478: let folded =
UNCOVERED LINE crates/pull/tests/store_evidence.rs:479: pull::fold::complete_minutes_with_calendar(&minutes, bucket, venue, None, runtime)
UNCOVERED LINE crates/pull/tests/store_evidence.rs:480: .unwrap();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:481: detailed.push(start.elapsed());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:482: std::hint::black_box(folded);
UNCOVERED LINE crates/pull/tests/store_evidence.rs:483: }
UNCOVERED LINE crates/pull/tests/store_evidence.rs:484: aggregated.sort_unstable();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:485: detailed.sort_unstable();
UNCOVERED LINE crates/pull/tests/store_evidence.rs:486: eprintln!(
UNCOVERED LINE crates/pull/tests/store_evidence.rs:488: minutes.len(),
UNCOVERED LINE crates/pull/tests/store_evidence.rs:489: aggregated[99],
UNCOVERED LINE crates/pull/tests/store_evidence.rs:490: aggregated[197],
UNCOVERED LINE crates/pull/tests/store_evidence.rs:491: detailed[99],
UNCOVERED LINE crates/pull/tests/store_evidence.rs:492: detailed[197]
UNCOVERED LINE crates/pull/tests/store_evidence.rs:494: assert!(!minutes.is_empty());
UNCOVERED LINE crates/pull/tests/store_evidence.rs:496: }
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:57:9 True: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:57:9 False: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:107:21 True: listing: if segment == "INDEX" {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:107:21 False: listing: if segment == "INDEX" {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:211:11 True: while day.month() == first.month() {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:211:11 False: while day.month() == first.month() {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:215:55 True: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:215:55 False: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:248:22 True: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:248:22 False: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:362:20 True: if let Owed::Unsized { .. } = what {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:362:20 False: if let Owed::Unsized { .. } = what {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:373:24 True: if session == pull::calendar::Session::full()
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:373:24 False: if session == pull::calendar::Session::full()
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:375:16 True: if !regular {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:375:16 False: if !regular {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:383:15 True: while day.month() == month {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:383:15 False: while day.month() == month {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:406:28 True: .any(|day| day.year() == year && day.month() == 11),
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:406:28 False: .any(|day| day.year() == year && day.month() == 11),
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:462:24 True: let venue = if segment == "INDEX" {
UNCOVERED BRANCH crates/pull/tests/store_evidence.rs:462:24 False: let venue = if segment == "INDEX" {
UNCOVERED BRANCH crates/pull/tests/support/mod.rs:62:15 False: let ran = output.status.success() && stdout.contains("1 passed");
UNCOVERED BRANCH crates/pull/tests/token_identity.rs:374:15 True: !(TYPES.iter().any(|name| {
UNCOVERED BRANCH crates/pull/tests/token_identity.rs:378:19 True: }) || FUNCTIONS.iter().any(|(_, name)| {
UNCOVERED LINE crates/pull/tests/unit.rs:143: other => panic!("expected Unreadable, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:766: other => panic!("expected an unreadable file, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:1303: other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:1324: other => panic!("expected a checksum refusal, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:1629: manifest.entries()
UNCOVERED LINE crates/pull/tests/unit.rs:1691: other => panic!("the recovery must name what it stepped over, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:1786: other => panic!("expected the slot checksum to be reported, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:1869: manifest.reserved()
UNCOVERED LINE crates/pull/tests/unit.rs:1874: manifest.reserved()
UNCOVERED LINE crates/pull/tests/unit.rs:2011: manifest.reserved()
UNCOVERED LINE crates/pull/tests/unit.rs:2069: other => panic!("expected entry 1 to be named, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:2209: other => panic!("expected a checksum refusal, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:2935: Verdict::Admit => panic!("a drained bucket admits nothing at once"),
UNCOVERED LINE crates/pull/tests/unit.rs:3052: governor.permitted(WindowSpan::Day)
UNCOVERED LINE crates/pull/tests/unit.rs:3384: panic!("the eighth request must be refused");
UNCOVERED LINE crates/pull/tests/unit.rs:4125: other => panic!("month {other}"),
UNCOVERED LINE crates/pull/tests/unit.rs:4891: Some(other) => panic!("a bar inside its own window was dropped as {other}"),
UNCOVERED LINE crates/pull/tests/unit.rs:5359: errors[0]
UNCOVERED LINE crates/pull/tests/unit.rs:6794: other => panic!("prefix of {cut} bytes was not refused by name: {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:6814: other => panic!("a corrupt header slot must be refused by name, got {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:7881: other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
UNCOVERED LINE crates/pull/tests/unit.rs:7952: layout.version()
UNCOVERED BRANCH crates/pull/tests/unit.rs:126:9 False: first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
UNCOVERED BRANCH crates/pull/tests/unit.rs:126:51 False: first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
UNCOVERED BRANCH crates/pull/tests/unit.rs:801:13 False: assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:801:49 False: assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:1016:13 False: assert!(rendered.contains("groww") && rendered.contains("fieldone"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:1016:43 False: assert!(rendered.contains("groww") && rendered.contains("fieldone"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:1017:13 False: assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:1017:45 False: assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:1627:40 False: manifest.entries() == 0 || manifest.entries() == 1,
UNCOVERED BRANCH crates/pull/tests/unit.rs:3383:9 False: let Verdict::Deny { span, wait_micros } = governor.admit(0) else {
UNCOVERED BRANCH crates/pull/tests/unit.rs:5203:13 False: assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5203:46 False: assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5204:13 False: assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5204:48 False: assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5366:13 False: assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5366:53 False: assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
UNCOVERED BRANCH crates/pull/tests/unit.rs:5410:13 False: assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5410:48 False: assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5411:13 False: assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5411:49 False: assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5425:13 False: assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5425:46 False: assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5426:13 False: assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5426:47 False: assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
UNCOVERED BRANCH crates/pull/tests/unit.rs:5556:9 False: text.contains("volume 2") && text.contains("open 3"),
UNCOVERED BRANCH crates/pull/tests/unit.rs:5556:38 False: text.contains("volume 2") && text.contains("open 3"),
UNCOVERED BRANCH crates/pull/tests/unit.rs:5736:9 False: group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),
UNCOVERED BRANCH crates/pull/tests/unit.rs:5736:46 False: group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:225: gaps[0].why
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:235: lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:543: other => panic!("2021-02-24 is a session of record: {other:?}"),
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:651: panic!("a regular cash session owes one window");
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:715: series.lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_gaps.rs:824: other => panic!("{other:?}"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:599:17 False: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:599:44 False: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:605:13 False: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:605:46 False: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:610:13 False: assert!(absent.short.is_empty() && !absent.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:610:40 False: assert!(absent.short.is_empty() && !absent.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:630:22 False: .all(|m| m.short.is_empty() && m.length_unjudged.is_empty())
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:650:9 False: let Owes::Windows((windows, 1)) = repair_plan::owed("CASH", from.days_from_epoch()) else {
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:712:9 False: series.lines[0].starts_with("gap NSE-CASH-INVENTEDSHR 2026-08-06 v=1 rung=1min owed=360 held=359 missing=1 first_missing=11:00 ")
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:713:16 False: && series.lines[0].ends_with(" fill=interior source=store absence=may_be_no_trade"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:833:13 False: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:833:52 False: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:876:9 False: second.lines.is_empty() && second.tally.already == 5,
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:876:36 False: second.lines.is_empty() && second.tally.already == 5,
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:958:9 False: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:958:62 False: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:991:13 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:991:37 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:991:63 False: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:1144:13 False: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:1144:32 False: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
UNCOVERED BRANCH crates/pull/tests/zerodha_gaps.rs:1348:22 False: .all(|l| l.contains(" why=stored_short ") && l.ends_with(" source=store")),
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:120: return None;
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:304: rung.as_str()
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:559: Err(store::file::StoreError::Missing { .. }) => Vec::new(),
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:560: Err(why) => panic!("{symbol} {}: {why}", tf.as_str()),
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:717: rung.as_str()
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:836: named[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1755: lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1764: lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1769: lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1810: identity_lines(sink, "PATANJALI")
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1957: named[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1966: said[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1971: said[0]
UNCOVERED LINE crates/pull/tests/zerodha_identity.rs:1976: said[0]
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:119:17 False: let pull::calendar::DayKind::Open(session) = kind else {
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:288:13 False: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:288:37 False: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:302:30 False: .all(|b| b.high <= LEVEL + 2_000 && b.low >= LEVEL - 1_000),
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:1495:22 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:1495:36 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:1495:50 False: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:1862:22 False: .any(|l| l.contains("the day rung would not open")
UNCOVERED BRANCH crates/pull/tests/zerodha_identity.rs:1863:20 False: && l.contains("\"why\":")
UNCOVERED LINE crates/pull/tests/zerodha_repair.rs:287: lines[0]
UNCOVERED LINE crates/pull/tests/zerodha_repair.rs:355: front[0]
UNCOVERED LINE crates/pull/tests/zerodha_repair.rs:405: lines[11]
UNCOVERED BRANCH crates/pull/tests/zerodha_repair.rs:340:22 False: .any(|f| f.why.contains("2020-12:") && f.why.contains("this month only")),
`````

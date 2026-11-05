# gaps-PSA.txt.md

Written by BASE-COV-PULL-PSA from Route B 2 on the code tip `cef062ebf194068b9c7a8a2ce790b20021139476`: the per-file table of every file of the set (crate `pull`, reading `whole:pull`) and every uncovered line and branch side with its class. The (U) proofs, (F) reasons and GAP owners are in `base-cov-pull-fu.txt.md`. One fenced block below.

`````text
BASE-COV-PULL-PSA: Route B (the kit's d3-route-b-cloud.sh, unchanged) on the code tip cef062ebf194068b9c7a8a2ce790b20021139476, run 2, reading whole:pull,
set deep-BASE-COV-PULL-PSA-cef062ebf194-set.txt (255 coverage lines: every tracked .rs of crates/pull but the 8 bench files, in no test object).
Pins: rustc 1.99.0-nightly (1a98b1e13 2026-08-07); llvm-cov and llvm-profdata LLVM version 23.1.0-rust-1.99.0-nightly. Host: Linux x86_64, root, 4 cores,
under the four conditions of D-7339 (TMPDIR a 1777 tmpfs, profiles under a 0777 folder, the script under umask 0000 with a umask-0022 cargo, kernel.pid_max 4194304).

ROUTE-B START 2026-11-05 17:10:38 IST tree /home/user/brutex tip cef062ebf194068b9c7a8a2ce790b20021139476 packages pull K /home/user/psa-k P /home/user/psa-prof/cef062ebf194-2-B (entries before: 0)
versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
OPS-10: every tracked file of /home/user/brutex touched 2026-11-05 17:10:38 IST
== build START 2026-11-05 17:10:38 IST: cargo test -p pull --locked --no-run --message-format=json (RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch', opt-level 0, CARGO_TARGET_DIR=/home/user/psa-k/cov)
== build rc=0 END 2026-11-05 17:11:42 IST (1 min); 168 artifacts, 129 reused
objects: 33 (/home/user/psa-prof/cef062ebf194-2-B/objects.txt, from this build's own report)
== test pull START 2026-11-05 17:11:42 IST
== test pull rc=0 END 2026-11-05 17:16:04 IST (4 min); tests passed 3607 failed 0 ignored 54
profraw files: 132
merged /home/user/psa-prof/cef062ebf194-2-B/pull/merged.profdata
ROUTE-B DONE 2026-11-05 17:50:52 IST: every package measured and read (pull)

whole pull (255 files): lines 175299/179878 (4579 uncovered), branches 12622/14464 (1842 sides)
G1 (every file PS2 and PS3 do not name: PS1's 89 and repair 2's census_store_cov_tests.rs and gdfl_rekey_cov_tests.rs, which no shard counted, both whole; 91 files): lines 57738/59292 (1554 uncovered), branches 4193/4954 (761 sides)
G2 (PS2's 93 files, as its per-file table marks them; 93 in this set): lines 58766/60545 (1779 uncovered), branches 4140/4724 (584 sides)
G3 (PS3's 71 files; 71 in this set): lines 58795/60041 (1246 uncovered), branches 4289/4786 (497 sides)
uncovered rows: 6421; by class: F 2231, U 364, IGNORED 3798, UNMEASURED 11, GAP 17
  G1: F 854, U 138, IGNORED 1306, UNMEASURED 0, GAP 17
  G2: F 623, U 146, IGNORED 1594, UNMEASURED 0, GAP 0
  G3: F 754, U 80, IGNORED 898, UNMEASURED 11, GAP 0
Classes: F and U as recipe v4 step 4 limits them, each carried from the owning shard's list of the same row (same file, line, column, side and text;
each line mapped back to the shard's tip through git diff -U0); IGNORED = inside an #[ignore] real-data or measurement test, or a helper
only those reach (L-BCP-3, the operator's); UNMEASURED = runs only in a child process its test kills with SIGKILL, so no profile is written (L-BCP-5);
GAP = left open, each with its owner in base-cov-pull-fu.txt.md. Every row is a row of a shard's list, but those of the assembly's own test code, read by hand (marked PSA).

Per-file table (lines a/b, branches c/d; the group after each):
crates/pull/src/alerts.rs lines 982/982, branches 165/166 G2
crates/pull/src/alerts_ps2_tests.rs lines 260/265, branches 9/18 G2
crates/pull/src/alerts_tests.rs lines 1615/1700, branches 49/82 G2
crates/pull/src/archive.rs lines 481/484, branches 31/32 G3
crates/pull/src/archive_cov_tests.rs lines 84/84, branches 0/0 G3
crates/pull/src/audit_ledger.rs lines 50/50, branches 10/10 G2
crates/pull/src/audit_ledger_tests.rs lines 148/148, branches 0/0 G2
crates/pull/src/base_cov_codes_tests.rs lines 423/423, branches 0/0 G2
crates/pull/src/base_cov_words_tests.rs lines 358/359, branches 1/2 G2
crates/pull/src/calendar.rs lines 2557/2610, branches 342/370 G2
crates/pull/src/calendar_cov_tests.rs lines 212/212, branches 4/4 G2
crates/pull/src/calendar_tradable_tests.rs lines 792/793, branches 37/50 G1
crates/pull/src/capture.rs lines 340/340, branches 8/10 G1
crates/pull/src/capture_ps1_tests.rs lines 58/58, branches 0/0 G1
crates/pull/src/cash_auction.rs lines 870/927, branches 98/100 G1
crates/pull/src/cash_auction_ps1_tests.rs lines 18/18, branches 0/0 G1
crates/pull/src/cash_session_cache.rs lines 1455/1615, branches 75/110 G1
crates/pull/src/cash_session_cache_lock_tests.rs lines 296/306, branches 15/22 G3
crates/pull/src/cash_session_cache_ps1_tests.rs lines 52/54, branches 2/4 G1
crates/pull/src/census_store.rs lines 2329/2340, branches 366/372 G2
crates/pull/src/census_store_cov_tests.rs lines 34/34, branches 0/0 G1
crates/pull/src/census_store_kept_tests.rs lines 712/788, branches 40/46 G2
crates/pull/src/census_store_link_tests.rs lines 549/550, branches 8/14 G2
crates/pull/src/census_store_ps2_tests.rs lines 1119/1120, branches 10/18 G2
crates/pull/src/census_store_tests.rs lines 1147/1178, branches 46/78 G2
crates/pull/src/chain.rs lines 356/362, branches 25/30 G2
crates/pull/src/chain_ps2_tests.rs lines 61/62, branches 3/4 G2
crates/pull/src/config.rs lines 386/386, branches 60/60 G2
crates/pull/src/corporate_actions.rs lines 1000/1000, branches 166/166 G3
crates/pull/src/corporate_actions_cov_tests.rs lines 289/291, branches 9/12 G3
crates/pull/src/corporate_actions_tests.rs lines 1279/1293, branches 52/84 G3
crates/pull/src/csv.rs lines 1016/1017, branches 95/98 G2
crates/pull/src/day_universe.rs lines 112/112, branches 14/22 G3
crates/pull/src/daycheck.rs lines 352/352, branches 27/30 G2
crates/pull/src/dhan.rs lines 118/118, branches 2/2 G2
crates/pull/src/disk_projection.rs lines 278/278, branches 29/30 G2
crates/pull/src/disk_projection_tests.rs lines 352/422, branches 1/8 G2
crates/pull/src/emit_sites.rs lines 1096/1109, branches 6/6 G2
crates/pull/src/expiry_alias.rs lines 683/689, branches 74/88 G1
crates/pull/src/fetch.rs lines 570/584, branches 54/56 G1
crates/pull/src/fetch_ps1_tests.rs lines 98/99, branches 1/2 G1
crates/pull/src/fno.rs lines 608/612, branches 53/62 G2
crates/pull/src/fno_membership.rs lines 1686/1692, branches 249/260 G1
crates/pull/src/fno_membership_cov_tests.rs lines 71/72, branches 1/2 G1
crates/pull/src/fno_membership_ps1_tests.rs lines 760/767, branches 22/28 G1
crates/pull/src/fno_membership_tests.rs lines 1035/1040, branches 49/66 G1
crates/pull/src/fno_ps2_tests.rs lines 79/80, branches 1/2 G2
crates/pull/src/fnowork.rs lines 871/871, branches 62/62 G3
crates/pull/src/fnowork_cov_tests.rs lines 201/203, branches 0/0 G3
crates/pull/src/fnowork_futures_tests.rs lines 1132/1188, branches 23/40 G2
crates/pull/src/fold.rs lines 1188/1200, branches 143/152 G2
crates/pull/src/fold_ps2_tests.rs lines 18/18, branches 0/0 G2
crates/pull/src/folder.rs lines 206/212, branches 16/20 G1
crates/pull/src/gap_backfill.rs lines 230/230, branches 46/46 G3
crates/pull/src/gap_backfill_cov_tests.rs lines 432/434, branches 18/18 G3
crates/pull/src/gaps.rs lines 728/728, branches 82/82 G3
crates/pull/src/gaps_cov_tests.rs lines 108/108, branches 3/6 G3
crates/pull/src/gaps_not_owed_tests.rs lines 486/486, branches 26/26 G2
crates/pull/src/gdfl_archive.rs lines 1530/1597, branches 81/86 G3
crates/pull/src/gdfl_backfill_tests.rs lines 863/868, branches 17/22 G2
crates/pull/src/gdfl_cm.rs lines 3147/3153, branches 359/364 G2
crates/pull/src/gdfl_cm_admission_tests.rs lines 1295/1347, branches 7/12 G3
crates/pull/src/gdfl_cm_auction.rs lines 628/631, branches 81/82 G1
crates/pull/src/gdfl_cm_auction_ps1_tests.rs lines 121/125, branches 2/4 G1
crates/pull/src/gdfl_cm_auction_v2_tests.rs lines 416/421, branches 6/10 G1
crates/pull/src/gdfl_cm_ps2_tests.rs lines 91/91, branches 0/0 G2
crates/pull/src/gdfl_dayzips.rs lines 201/202, branches 34/34 G2
crates/pull/src/gdfl_dayzips_ps2_tests.rs lines 70/70, branches 0/0 G2
crates/pull/src/gdfl_dayzips_tests.rs lines 361/409, branches 0/2 G2
crates/pull/src/gdfl_dropbox.rs lines 2145/2145, branches 447/448 G3
crates/pull/src/gdfl_dropbox_cov_tests.rs lines 649/661, branches 22/38 G3
crates/pull/src/gdfl_dropbox_tests.rs lines 3689/3797, branches 138/166 G3
crates/pull/src/gdfl_fixtures.rs lines 198/198, branches 14/14 G1
crates/pull/src/gdfl_fixtures_ps1_tests.rs lines 34/34, branches 0/0 G1
crates/pull/src/gdfl_format.rs lines 32/32, branches 4/4 G2
crates/pull/src/gdfl_format_change_tests.rs lines 657/658, branches 22/28 G2
crates/pull/src/gdfl_format_ps2_tests.rs lines 31/31, branches 0/0 G2
crates/pull/src/gdfl_ia16_tests.rs lines 2186/2188, branches 87/132 G3
crates/pull/src/gdfl_ia3_tests.rs lines 660/869, branches 8/28 G2
crates/pull/src/gdfl_ia4_tests.rs lines 192/193, branches 4/6 G3
crates/pull/src/gdfl_import.rs lines 1769/1770, branches 214/214 G1
crates/pull/src/gdfl_import_causes.rs lines 349/349, branches 6/6 G1
crates/pull/src/gdfl_import_census.rs lines 133/133, branches 12/12 G2
crates/pull/src/gdfl_import_census_ps2_tests.rs lines 16/16, branches 0/0 G2
crates/pull/src/gdfl_import_cov_tests.rs lines 912/912, branches 16/26 G1
crates/pull/src/gdfl_import_day.rs lines 1475/1482, branches 246/246 G1
crates/pull/src/gdfl_import_day_ps1_tests.rs lines 1255/1294, branches 39/74 G1
crates/pull/src/gdfl_import_filing.rs lines 952/953, branches 137/140 G2
crates/pull/src/gdfl_import_filing_ps2_tests.rs lines 498/509, branches 9/14 G2
crates/pull/src/gdfl_import_ia2_tests.rs lines 449/551, branches 11/22 G3
crates/pull/src/gdfl_import_journal.rs lines 791/791, branches 207/208 G3
crates/pull/src/gdfl_import_journal_cov_tests.rs lines 323/323, branches 2/2 G3
crates/pull/src/gdfl_import_lineage.rs lines 380/380, branches 78/78 G3
crates/pull/src/gdfl_import_lineage_cov_tests.rs lines 265/265, branches 0/0 G3
crates/pull/src/gdfl_import_ps1_tests.rs lines 192/193, branches 3/4 G1
crates/pull/src/gdfl_import_quality.rs lines 468/468, branches 108/110 G2
crates/pull/src/gdfl_import_quality_ps2_tests.rs lines 115/115, branches 0/0 G2
crates/pull/src/gdfl_import_scope.rs lines 250/250, branches 14/18 G2
crates/pull/src/gdfl_import_split_tests.rs lines 462/463, branches 38/40 G3
crates/pull/src/gdfl_import_tests.rs lines 4705/4732, branches 209/304 G1
crates/pull/src/gdfl_lineage.rs lines 822/822, branches 154/154 G3
crates/pull/src/gdfl_lineage_tests.rs lines 1116/1212, branches 13/26 G3
crates/pull/src/gdfl_nfo.rs lines 920/920, branches 180/180 G1
crates/pull/src/gdfl_nfo_attack_tests.rs lines 1304/1329, branches 152/174 G3
crates/pull/src/gdfl_nfo_ps1_tests.rs lines 96/96, branches 2/2 G1
crates/pull/src/gdfl_nfo_tests.rs lines 1849/2631, branches 48/188 G1
crates/pull/src/gdfl_r2_attack_tests.rs lines 1324/1328, branches 148/160 G2
crates/pull/src/gdfl_r3_attack_tests.rs lines 983/993, branches 125/136 G1
crates/pull/src/gdfl_r4_attack_tests.rs lines 582/585, branches 43/54 G1
crates/pull/src/gdfl_rekey.rs lines 1500/1511, branches 258/268 G2
crates/pull/src/gdfl_rekey_cov_tests.rs lines 38/38, branches 0/0 G1
crates/pull/src/gdfl_rekey_ps2_tests.rs lines 950/954, branches 11/20 G2
crates/pull/src/gdfl_rekey_tests.rs lines 1803/2011, branches 75/144 G2
crates/pull/src/gdfl_run.rs lines 1419/1425, branches 166/170 G2
crates/pull/src/gdfl_run_cov_tests.rs lines 477/479, branches 1/2 G2
crates/pull/src/gdfl_run_ia2_tests.rs lines 782/946, branches 40/74 G3
crates/pull/src/gdfl_run_ledger.rs lines 418/423, branches 68/72 G2
crates/pull/src/gdfl_run_ledger_cov_tests.rs lines 100/100, branches 0/0 G2
crates/pull/src/gdfl_run_ledger_ps2_tests.rs lines 252/255, branches 5/8 G2
crates/pull/src/gdfl_run_locks.rs lines 722/731, branches 155/158 G1
crates/pull/src/gdfl_run_locks_cov_tests.rs lines 285/285, branches 5/10 G1
crates/pull/src/gdfl_run_locks_ps1_tests.rs lines 297/302, branches 11/18 G1
crates/pull/src/gdfl_run_scope.rs lines 62/62, branches 8/8 G3
crates/pull/src/gdfl_run_scope_cov_tests.rs lines 157/157, branches 2/2 G3
crates/pull/src/gdfl_run_start.rs lines 71/71, branches 6/6 G3
crates/pull/src/gdfl_run_start_cov_tests.rs lines 130/130, branches 2/2 G3
crates/pull/src/gdfl_run_stop.rs lines 232/232, branches 40/40 G1
crates/pull/src/gdfl_run_stop_cov_tests.rs lines 146/147, branches 1/2 G1
crates/pull/src/gdfl_run_tests.rs lines 1094/1252, branches 12/26 G2
crates/pull/src/gdfl_scope_tests.rs lines 1837/1845, branches 38/54 G2
crates/pull/src/gdfl_sealed_tests.rs lines 305/305, branches 17/20 G2
crates/pull/src/gdfl_seconds_attack_tests.rs lines 1211/1213, branches 62/76 G2
crates/pull/src/gdfl_series_code_tests.rs lines 363/363, branches 2/4 G3
crates/pull/src/gdfl_tickstore.rs lines 1185/1186, branches 115/116 G3
crates/pull/src/gdfl_tickstore_cov_tests.rs lines 157/157, branches 2/2 G3
crates/pull/src/gdfl_v3.rs lines 304/304, branches 60/60 G2
crates/pull/src/gdfl_v3_ps2_tests.rs lines 27/27, branches 0/0 G2
crates/pull/src/gdfl_v3_tests.rs lines 630/759, branches 16/46 G2
crates/pull/src/gdfl_wp11c_tests.rs lines 402/402, branches 6/6 G2
crates/pull/src/gdfl_wp12e_tests.rs lines 517/634, branches 7/10 G3
crates/pull/src/groww.rs lines 41/41, branches 0/0 G2
crates/pull/src/http.rs lines 4431/4501, branches 339/394 G3
crates/pull/src/http_cov_tests.rs lines 484/493, branches 14/22 G3
crates/pull/src/identity_screen.rs lines 473/474, branches 63/64 G1
crates/pull/src/identity_screen_ps1_tests.rs lines 72/72, branches 0/0 G1
crates/pull/src/ingest.rs lines 3269/3286, branches 270/284 G3
crates/pull/src/ingest_cov_tests.rs lines 213/213, branches 2/4 G3
crates/pull/src/ingest_failkind.rs lines 124/125, branches 10/10 G3
crates/pull/src/ingest_failkind_tests.rs lines 583/583, branches 8/10 G3
crates/pull/src/ingest_gap_tests.rs lines 1064/1064, branches 0/0 G3
crates/pull/src/kite.rs lines 57/57, branches 0/0 G3
crates/pull/src/kite_cov_tests.rs lines 6/6, branches 0/0 G3
crates/pull/src/landing_run.rs lines 338/338, branches 26/26 G2
crates/pull/src/landing_run_tests.rs lines 1761/1890, branches 54/80 G2
crates/pull/src/lib.rs lines 5/5, branches 0/0 G2
crates/pull/src/listing.rs lines 237/237, branches 38/38 G1
crates/pull/src/listing_tests.rs lines 397/400, branches 3/4 G1
crates/pull/src/manifest.rs lines 1366/1366, branches 169/170 G1
crates/pull/src/manifest_cov_tests.rs lines 16/16, branches 0/0 G1
crates/pull/src/manifest_ps1_tests.rs lines 156/157, branches 0/0 G1
crates/pull/src/master_snapshots.rs lines 1677/1679, branches 263/270 G1
crates/pull/src/master_snapshots_cov_tests.rs lines 627/628, branches 2/4 G1
crates/pull/src/master_snapshots_first_listed.rs lines 567/569, branches 85/86 G1
crates/pull/src/master_snapshots_first_listed_ps1_tests.rs lines 348/348, branches 2/2 G1
crates/pull/src/master_snapshots_first_listed_tests.rs lines 423/424, branches 22/44 G1
crates/pull/src/master_snapshots_ps1_tests.rs lines 159/159, branches 0/0 G1
crates/pull/src/master_snapshots_tests.rs lines 1358/1514, branches 34/68 G1
crates/pull/src/masters.rs lines 1742/1765, branches 184/204 G3
crates/pull/src/masters_cov_tests.rs lines 121/121, branches 4/4 G3
crates/pull/src/masters_land_tests.rs lines 245/248, branches 7/10 G3
crates/pull/src/nse.rs lines 503/508, branches 64/78 G1
crates/pull/src/nse_ps1_tests.rs lines 36/38, branches 2/4 G1
crates/pull/src/nseindex.rs lines 231/231, branches 14/14 G2
crates/pull/src/padded.rs lines 109/109, branches 28/28 G2
crates/pull/src/padded_tests.rs lines 414/415, branches 7/8 G2
crates/pull/src/pricing.rs lines 705/713, branches 34/52 G1
crates/pull/src/pricing_ps1_tests.rs lines 69/71, branches 2/4 G1
crates/pull/src/rate.rs lines 254/254, branches 26/26 G2
crates/pull/src/reference_data.rs lines 1930/1943, branches 238/238 G1
crates/pull/src/reference_data_cov_tests.rs lines 418/418, branches 2/4 G1
crates/pull/src/reference_data_ps1_tests.rs lines 486/490, branches 4/8 G1
crates/pull/src/reference_data_tests.rs lines 980/1023, branches 22/36 G1
crates/pull/src/refusal.rs lines 114/114, branches 8/8 G1
crates/pull/src/refusal_ps1_tests.rs lines 47/47, branches 0/0 G1
crates/pull/src/rehearsal.rs lines 553/557, branches 97/98 G1
crates/pull/src/rehearsal_ps1_tests.rs lines 47/47, branches 0/0 G1
crates/pull/src/rehearsal_tests.rs lines 614/615, branches 14/18 G1
crates/pull/src/repair_plan.rs lines 537/541, branches 61/66 G2
crates/pull/src/repair_plan_not_owed_tests.rs lines 272/274, branches 7/10 G3
crates/pull/src/repair_plan_ps2_tests.rs lines 254/255, branches 6/10 G2
crates/pull/src/request_minutes.rs lines 1422/1446, branches 117/142 G1
crates/pull/src/request_minutes_ps1_tests.rs lines 66/66, branches 0/0 G1
crates/pull/src/resolve.rs lines 667/680, branches 23/24 G3
crates/pull/src/response_keep.rs lines 1317/1322, branches 191/200 G2
crates/pull/src/response_keep_tests.rs lines 2462/2470, branches 39/50 G2
crates/pull/src/riskfree.rs lines 1145/1147, branches 93/96 G2
crates/pull/src/rolling.rs lines 1030/1035, branches 64/80 G3
crates/pull/src/scrub.rs lines 430/435, branches 21/24 G1
crates/pull/src/scrub_blocks_tests.rs lines 92/92, branches 0/0 G2
crates/pull/src/secret.rs lines 122/122, branches 8/8 G2
crates/pull/src/series_identity.rs lines 672/673, branches 60/64 G1
crates/pull/src/session.rs lines 738/747, branches 106/112 G3
crates/pull/src/ssm.rs lines 912/939, branches 86/114 G1
crates/pull/src/ssm_cov_tests.rs lines 159/159, branches 2/2 G1
crates/pull/src/ssm_ps1_tests.rs lines 74/74, branches 4/4 G1
crates/pull/src/tenor.rs lines 194/196, branches 6/8 G3
crates/pull/src/tickstore_write.rs lines 1121/1129, branches 184/188 G3
crates/pull/src/tickstore_write_tests.rs lines 1779/1931, branches 49/68 G3
crates/pull/src/totp.rs lines 370/372, branches 24/26 G3
crates/pull/src/universe.rs lines 415/418, branches 12/12 G1
crates/pull/src/upkeep.rs lines 933/934, branches 147/150 G1
crates/pull/src/upkeep_tests.rs lines 1475/1485, branches 55/78 G1
crates/pull/src/vendor.rs lines 2318/2339, branches 141/178 G1
crates/pull/src/vendor_cov_tests.rs lines 74/74, branches 0/0 G1
crates/pull/src/verdict_words.rs lines 93/94, branches 2/4 G1
crates/pull/src/work.rs lines 129/129, branches 14/14 G2
crates/pull/src/zerodha_spool.rs lines 834/841, branches 134/136 G2
crates/pull/src/zerodha_spool_cov_tests.rs lines 443/454, branches 23/42 G2
crates/pull/src/zerodha_verify.rs lines 2859/2872, branches 596/614 G3
crates/pull/src/zerodha_verify_cov_store_tests.rs lines 1104/1106, branches 10/12 G3
crates/pull/src/zerodha_verify_cov_tests.rs lines 849/850, branches 3/4 G3
crates/pull/src/zerodha_verify_tests.rs lines 3762/3773, branches 46/72 G3
crates/pull/tests/alerts_ps2.rs lines 34/34, branches 0/0 G2
crates/pull/tests/alerts_seams.rs lines 78/79, branches 0/0 G1
crates/pull/tests/allocation.rs lines 385/388, branches 5/8 G1
crates/pull/tests/anchor.rs lines 516/522, branches 24/32 G3
crates/pull/tests/broker.rs lines 761/768, branches 28/34 G2
crates/pull/tests/census.rs lines 777/789, branches 24/38 G1
crates/pull/tests/census_day_pace.rs lines 87/88, branches 4/8 G1
crates/pull/tests/census_resume.rs lines 151/155, branches 7/12 G2
crates/pull/tests/census_v4.rs lines 549/628, branches 4/6 G3
crates/pull/tests/cloud_mutants_import.rs lines 1215/1225, branches 17/30 G2
crates/pull/tests/concurrency.rs lines 51/51, branches 0/0 G2
crates/pull/tests/derive.rs lines 1396/1407, branches 38/54 G3
crates/pull/tests/folder.rs lines 489/492, branches 13/20 G1
crates/pull/tests/futures_events.rs lines 130/131, branches 3/6 G3
crates/pull/tests/futures_refusal_events.rs lines 70/70, branches 0/0 G2
crates/pull/tests/gdfl_bad_row_audit.rs lines 239/245, branches 3/6 G3
crates/pull/tests/gdfl_census_stop.rs lines 146/151, branches 10/18 G3
crates/pull/tests/gdfl_run_stop.rs lines 15/15, branches 0/0 G1
crates/pull/tests/integration.rs lines 237/238, branches 0/0 G3
crates/pull/tests/no_scheduler_delete.rs lines 318/318, branches 112/112 G3
crates/pull/tests/padded_refold.rs lines 389/403, branches 4/4 G2
crates/pull/tests/pipeline.rs lines 1047/1060, branches 16/24 G3
crates/pull/tests/refusal.rs lines 310/316, branches 15/16 G1
crates/pull/tests/request_gap_events.rs lines 105/106, branches 8/14 G1
crates/pull/tests/sealed_month.rs lines 348/348, branches 16/20 G1
crates/pull/tests/store_evidence.rs lines 31/336, branches 0/22 G2
crates/pull/tests/support/mod.rs lines 32/32, branches 3/4 G1
crates/pull/tests/tls.rs lines 14/14, branches 0/0 G2
crates/pull/tests/token_identity.rs lines 231/231, branches 54/56 G1
crates/pull/tests/unit.rs lines 4528/4550, branches 104/132 G1
crates/pull/tests/zerodha_gaps.rs lines 1006/1012, branches 84/106 G2
crates/pull/tests/zerodha_identity.rs lines 1498/1510, branches 60/68 G2
crates/pull/tests/zerodha_repair.rs lines 473/476, branches 5/6 G2

Every uncovered line and side, with its class, by whom it was classed, and its text:
U crates/pull/src/alerts.rs:1458:28 False [G2 PS2]: if let Some(mark) = acked.get_mut(i) {
F crates/pull/src/alerts_ps2_tests.rs:163:9 False [G2 PS2]: let Put::Written {
F crates/pull/src/alerts_ps2_tests.rs:167 [G2 PS2]: panic!("{put:?}");
F crates/pull/src/alerts_ps2_tests.rs:193:13 False [G2 PS2]: assert!(!page.exists && page.alerts.is_empty());
F crates/pull/src/alerts_ps2_tests.rs:193:29 False [G2 PS2]: assert!(!page.exists && page.alerts.is_empty());
F crates/pull/src/alerts_ps2_tests.rs:224:13 False [G2 PS2]: assert!(by.starts_with(&record.text) && record.text.len() < by.len());
F crates/pull/src/alerts_ps2_tests.rs:224:45 False [G2 PS2]: assert!(by.starts_with(&record.text) && record.text.len() < by.len());
F crates/pull/src/alerts_ps2_tests.rs:266:9 False [G2 PS2]: let AlertError::Io { op, kind, text } = refused else {
F crates/pull/src/alerts_ps2_tests.rs:267 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/alerts_ps2_tests.rs:293:9 False [G2 PS2]: let AlertError::Io { op, kind, .. } = refused else {
F crates/pull/src/alerts_ps2_tests.rs:294 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/alerts_ps2_tests.rs:348:9 False [G2 PS2]: let Err(AlertError::Io { op, kind, .. }) = refused else {
F crates/pull/src/alerts_ps2_tests.rs:349 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/alerts_ps2_tests.rs:386:9 False [G2 PS2]: let AlertError::Io { op, .. } = refused else {
F crates/pull/src/alerts_ps2_tests.rs:387 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/alerts_tests.rs:78 [G2 PS2]: Put::Duplicate { ordinal } => panic!("a duplicate of {ordinal}, not a write"),
F crates/pull/src/alerts_tests.rs:210 [G2 PS2]: matches!(
F crates/pull/src/alerts_tests.rs:237:9 False [G2 PS2]: let Put::Written {
F crates/pull/src/alerts_tests.rs:243 [G2 PS2]: panic!("{got:?}")
F crates/pull/src/alerts_tests.rs:413:9 False [G2 PS2]: let Kept::Good(4, record) = &kept[3] else {
F crates/pull/src/alerts_tests.rs:414 [G2 PS2]: panic!("{:?}", kept[3])
F crates/pull/src/alerts_tests.rs:504:13 False [G2 PS2]: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
F crates/pull/src/alerts_tests.rs:504:30 False [G2 PS2]: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
F crates/pull/src/alerts_tests.rs:504:59 False [G2 PS2]: assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
F crates/pull/src/alerts_tests.rs:779:52 False [G2 PS2]: assert!(matches!(&seen[0], Kept::Good(1, r) if r.text == "invented k1"));
F crates/pull/src/alerts_tests.rs:780:52 False [G2 PS2]: assert!(matches!(&seen[2], Kept::Good(3, r) if r.open_after == 3));
F crates/pull/src/alerts_tests.rs:892:13 False [G2 PS2]: assert!(names.queue.is_file() && names.lock.is_file());
F crates/pull/src/alerts_tests.rs:892:38 False [G2 PS2]: assert!(names.queue.is_file() && names.lock.is_file());
F crates/pull/src/alerts_tests.rs:1105 [G2 PS2]: matches!(
F crates/pull/src/alerts_tests.rs:1138:9 False [G2 PS2]: let Err(AlertError::Io { kind, op, .. }) = &refused else {
F crates/pull/src/alerts_tests.rs:1139 [G2 PS2]: panic!("{refused:?}")
F crates/pull/src/alerts_tests.rs:1259:17 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
F crates/pull/src/alerts_tests.rs:1259:32 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
F crates/pull/src/alerts_tests.rs:1259:58 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.first == 0);
F crates/pull/src/alerts_tests.rs:1526:47 False [G2 PS2]: seen.damaged == [3, 2] || replaced,
F crates/pull/src/alerts_tests.rs:1545:56 False [G2 PS2]: matches!(&first, Some(Kept::Good(1, r)) if r.kind == KIND_QUEUE_REPLACED),
F crates/pull/src/alerts_tests.rs:1595 [G2 PS2]: matches!(
F crates/pull/src/alerts_tests.rs:1858:13 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
F crates/pull/src/alerts_tests.rs:1858:28 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
F crates/pull/src/alerts_tests.rs:1858:54 False [G2 PS2]: assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
F crates/pull/src/alerts_tests.rs:1865:13 False [G2 PS2]: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
F crates/pull/src/alerts_tests.rs:1865:40 False [G2 PS2]: assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
IGNORED crates/pull/src/alerts_tests.rs:2070 [G2 PS2]: fn spread(mut micros: Vec<u128>) -> (u128, u128, u128) {
IGNORED crates/pull/src/alerts_tests.rs:2071 [G2 PS2]: micros.sort_unstable();
IGNORED crates/pull/src/alerts_tests.rs:2072 [G2 PS2]: let at = |q: usize| micros[(micros.len() * q / 100).min(micros.len() - 1)];
IGNORED crates/pull/src/alerts_tests.rs:2073 [G2 PS2]: (at(50), at(99), micros[micros.len() - 1])
IGNORED crates/pull/src/alerts_tests.rs:2074 [G2 PS2]: }
IGNORED crates/pull/src/alerts_tests.rs:2081 [G2 PS2]: fn the_operators_older_steps_timed() {
IGNORED crates/pull/src/alerts_tests.rs:2082 [G2 PS2]: let scratch = Scratch::new("timed");
IGNORED crates/pull/src/alerts_tests.rs:2083 [G2 PS2]: let records = 102_400_u32;
IGNORED crates/pull/src/alerts_tests.rs:2084 [G2 PS2]: raw_queue(&scratch, &open_alerts(records));
IGNORED crates/pull/src/alerts_tests.rs:2085 [G2 PS2]: for (label, before, later) in [
IGNORED crates/pull/src/alerts_tests.rs:2086 [G2 PS2]: ("1,024 records after the page", records - 1_023, 1_024_u32),
IGNORED crates/pull/src/alerts_tests.rs:2087 [G2 PS2]: ("51,201 records after the page", 51_200, 51_201),
IGNORED crates/pull/src/alerts_tests.rs:2088 [G2 PS2]: ("101,376 records after the page", 1_025, 101_376),
IGNORED crates/pull/src/alerts_tests.rs:2089 [G2 PS2]: ] {
IGNORED crates/pull/src/alerts_tests.rs:2090 [G2 PS2]: let mut micros = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2091 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2092 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2093 [G2 PS2]: let page = page_settled(scratch.root(), before).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2094 [G2 PS2]: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2095 [G2 PS2]: assert_eq!(page.alerts.len(), 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2096 [G2 PS2]: assert_eq!(page.older_unacknowledged, before - 1 - 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2097 [G2 PS2]: assert_eq!(u64::from(later), page.records - u64::from(before) + 1);
IGNORED crates/pull/src/alerts_tests.rs:2099 [G2 PS2]: let (p50, p99, max) = spread(micros);
IGNORED crates/pull/src/alerts_tests.rs:2100 [G2 PS2]: println!("page_settled, {label}: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
IGNORED crates/pull/src/alerts_tests.rs:2102 [G2 PS2]: let mut one_read = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2103 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2104 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2105 [G2 PS2]: let page = page_of(&scratch, Some(51_200));
IGNORED crates/pull/src/alerts_tests.rs:2106 [G2 PS2]: one_read.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2107 [G2 PS2]: assert_eq!(page.alerts.len(), 1_024);
IGNORED crates/pull/src/alerts_tests.rs:2109 [G2 PS2]: let (p50, p99, max) = spread(one_read);
IGNORED crates/pull/src/alerts_tests.rs:2110 [G2 PS2]: println!(
IGNORED crates/pull/src/alerts_tests.rs:2116 [G2 PS2]: let mut walk = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2117 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/alerts_tests.rs:2118 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2119 [G2 PS2]: let found = older(scratch.root()).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2120 [G2 PS2]: walk.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2121 [G2 PS2]: assert_eq!(found.alerts.len(), 101_376);
IGNORED crates/pull/src/alerts_tests.rs:2122 [G2 PS2]: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:17 False [G2 PS2]: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:17 True [G2 PS2]: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:45 False [G2 PS2]: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2122:45 True [G2 PS2]: assert!(found.damaged.is_empty() && found.newer_damaged == 0);
IGNORED crates/pull/src/alerts_tests.rs:2124 [G2 PS2]: let (p50, p99, max) = spread(walk);
IGNORED crates/pull/src/alerts_tests.rs:2125 [G2 PS2]: println!("older, 102,400 records: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
IGNORED crates/pull/src/alerts_tests.rs:2129 [G2 PS2]: let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
IGNORED crates/pull/src/alerts_tests.rs:2130 [G2 PS2]: let walker = {
IGNORED crates/pull/src/alerts_tests.rs:2131 [G2 PS2]: let stop = std::sync::Arc::clone(&stop);
IGNORED crates/pull/src/alerts_tests.rs:2132 [G2 PS2]: let root = scratch.root().to_path_buf();
IGNORED crates/pull/src/alerts_tests.rs:2133 [G2 PS2]: std::thread::spawn(move || {
IGNORED crates/pull/src/alerts_tests.rs:2134 [G2 PS2]: let mut walks = 0_u32;
IGNORED crates/pull/src/alerts_tests.rs:2135 [G2 PS2]: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2135:19 False [G2 PS2]: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2135:19 True [G2 PS2]: while !stop.load(Ordering::Relaxed) {
IGNORED crates/pull/src/alerts_tests.rs:2136 [G2 PS2]: assert!(older(&root).unwrap().alerts.len() >= 101_376);
IGNORED crates/pull/src/alerts_tests.rs:2137 [G2 PS2]: walks += 1;
IGNORED crates/pull/src/alerts_tests.rs:2139 [G2 PS2]: walks
IGNORED crates/pull/src/alerts_tests.rs:2140 [G2 PS2]: })
IGNORED crates/pull/src/alerts_tests.rs:2142 [G2 PS2]: let mut puts = Vec::with_capacity(200);
IGNORED crates/pull/src/alerts_tests.rs:2143 [G2 PS2]: for i in 0..200_u32 {
IGNORED crates/pull/src/alerts_tests.rs:2144 [G2 PS2]: let key = format!("timed-put-{i}");
IGNORED crates/pull/src/alerts_tests.rs:2145 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2146 [G2 PS2]: let put = put_at(scratch.root(), &alert(6, &key, "t"), NOW).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2147 [G2 PS2]: puts.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2148 [G2 PS2]: assert_eq!(written(put), records + 1 + i);
IGNORED crates/pull/src/alerts_tests.rs:2150 [G2 PS2]: stop.store(true, Ordering::Relaxed);
IGNORED crates/pull/src/alerts_tests.rs:2151 [G2 PS2]: let walks = walker.join().unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2152 [G2 PS2]: let (p50, p99, max) = spread(puts);
IGNORED crates/pull/src/alerts_tests.rs:2153 [G2 PS2]: println!(
IGNORED crates/pull/src/alerts_tests.rs:2159 [G2 PS2]: for size in [7_u32, 102_400] {
IGNORED crates/pull/src/alerts_tests.rs:2160 [G2 PS2]: let n: u64 = if size == 7 { 100 } else { 30 };
IGNORED crates/pull/src/alerts_tests.rs:2160:25 False [G2 PS2]: let n: u64 = if size == 7 { 100 } else { 30 };
IGNORED crates/pull/src/alerts_tests.rs:2160:25 True [G2 PS2]: let n: u64 = if size == 7 { 100 } else { 30 };
IGNORED crates/pull/src/alerts_tests.rs:2161 [G2 PS2]: let mut micros = Vec::with_capacity(100);
IGNORED crates/pull/src/alerts_tests.rs:2162 [G2 PS2]: for i in 0..n {
IGNORED crates/pull/src/alerts_tests.rs:2163 [G2 PS2]: let scratch = Scratch::new("timed-replace");
IGNORED crates/pull/src/alerts_tests.rs:2164 [G2 PS2]: raw_queue(&scratch, &open_alerts(size));
IGNORED crates/pull/src/alerts_tests.rs:2165 [G2 PS2]: spoil(&scratch, u64::from(size) - 1);
IGNORED crates/pull/src/alerts_tests.rs:2166 [G2 PS2]: spoil(&scratch, u64::from(size));
IGNORED crates/pull/src/alerts_tests.rs:2167 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/alerts_tests.rs:2168 [G2 PS2]: let done = replace_damaged_at(scratch.root(), NOW + i).unwrap();
IGNORED crates/pull/src/alerts_tests.rs:2169 [G2 PS2]: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/alerts_tests.rs:2170 [G2 PS2]: assert_eq!((done.open, done.ordinal), (Some(size - 2), 1));
IGNORED crates/pull/src/alerts_tests.rs:2172 [G2 PS2]: let (p50, p99, max) = spread(micros);
IGNORED crates/pull/src/alerts_tests.rs:2173 [G2 PS2]: println!(
IGNORED crates/pull/src/alerts_tests.rs:2177 [G2 PS2]: }
F crates/pull/src/alerts_tests.rs:2193:13 False [G2 PS2]: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
F crates/pull/src/alerts_tests.rs:2193:38 False [G2 PS2]: assert!(names.queue.is_file() && !names.lock.exists(), "the premise");
F crates/pull/src/alerts_tests.rs:2226:9 False [G2 PS2]: before <= now && now <= after,
F crates/pull/src/alerts_tests.rs:2226:26 False [G2 PS2]: before <= now && now <= after,
F crates/pull/src/alerts_tests.rs:2475:62 False [G2 PS2]: matches!(&refused, Err(AlertError::Io { op, .. }) if *op == "keep the damaged queue"),
F crates/pull/src/archive.rs:1132:77 False [G3 PS3]: matches!(why, ArchiveError::MemberMalformed { ref path, .. } if *path == odd),
F crates/pull/src/archive.rs:1333 [G3 PS3]: walked.len()
F crates/pull/src/archive.rs:1349 [G3 PS3]: sentence.len()
F crates/pull/src/archive.rs:1384 [G3 PS3]: matches!(
F crates/pull/src/base_cov_words_tests.rs:125:9 False [G2 PS2]: let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/base_cov_words_tests.rs:127 [G2 PS2]: panic!("Dhan is an HTTP broker");
U crates/pull/src/calendar.rs:419:20 False [G2 PS2]: if let Some(w) = self.windows.get(i) {
U crates/pull/src/calendar.rs:678:9 False [G2 PS2]: let Ok(offset) = usize::try_from(epoch_day - FIRST_DAY) else {
U crates/pull/src/calendar.rs:679 [G2 PS2]: return DayKind::Unmeasured;
U crates/pull/src/calendar.rs:742 [G2 PS2]: DayKind::Unmeasured => return None,
U crates/pull/src/calendar.rs:2128:16 False [G2 PS2]: if let Ok(minute) = u16::try_from(minute)
U crates/pull/src/calendar.rs:2129:16 False [G2 PS2]: && minute < MINUTES_PER_DAY
U crates/pull/src/calendar.rs:2130:20 False [G2 PS2]: && let Some(slot) = cuts.get_mut(used)
U crates/pull/src/calendar.rs:2435:42 False [G2 PS2]: DayKind::OpenLengthUnmeasured if listed(&SOURCED_SPECIAL, epoch_day) => {
U crates/pull/src/calendar.rs:2555 [G2 PS2]: Err(SettlementRefusal::NoSessionWithin { stated })
U crates/pull/src/calendar.rs:2614 [G2 PS2]: Ok(None)
U crates/pull/src/calendar.rs:3537:20 False [G2 PS2]: if let Some(count) = running.get_mut(slot) {
U crates/pull/src/calendar.rs:3581:13 False [G2 PS2]: let (Some(before), Some(through)) = (self.sums.get(low), self.sums.get(high)) else {
U crates/pull/src/calendar.rs:3582 [G2 PS2]: return Err(outside);
U crates/pull/src/calendar.rs:3752:12 False [G2 PS2]: if newest && civil_year(notice.day) == i64::from(year) {
F crates/pull/src/calendar.rs:3869:17 False [G2 PS2]: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
F crates/pull/src/calendar.rs:3869:56 False [G2 PS2]: assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
F crates/pull/src/calendar.rs:3879:17 False [G2 PS2]: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
F crates/pull/src/calendar.rs:3879:55 False [G2 PS2]: assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
F crates/pull/src/calendar.rs:3890:65 False [G2 PS2]: matches!(record.kind, DayKind::Open(session) if session.bars() == 375),
F crates/pull/src/calendar.rs:3900:62 False [G2 PS2]: .all(|pair| matches!(pair, [older, newer] if older.day < newer.day)),
F crates/pull/src/calendar.rs:4039:17 False [G2 PS2]: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
F crates/pull/src/calendar.rs:4039:35 False [G2 PS2]: assert!(soon.ends_soon && !soon.ended, "{soon:?}");
F crates/pull/src/calendar.rs:4212 [G2 PS2]: DayKind::Unmeasured => panic!("day {day} is inside the range and unmeasured"),
F crates/pull/src/calendar.rs:4333:21 False [G2 PS2]: let DayKind::Open(session) = kind_of(day) else {
F crates/pull/src/calendar.rs:4334 [G2 PS2]: panic!("{index} {day}")
F crates/pull/src/calendar.rs:4348:71 False [G2 PS2]: .all(|pair| matches!(pair, [first, second] if second - first == 60))
F crates/pull/src/calendar.rs:4367:13 False [G2 PS2]: let DayKind::Open(outage) = kind_of(SYSTEMS_OUTAGE_DAY) else {
F crates/pull/src/calendar.rs:4368 [G2 PS2]: panic!("2021-02-24 traded");
F crates/pull/src/calendar.rs:4383:13 False [G2 PS2]: let DayKind::Open(muhurat) = kind_of(20_382) else {
F crates/pull/src/calendar.rs:4384 [G2 PS2]: panic!("2025-10-21 traded");
F crates/pull/src/calendar.rs:4393:13 False [G2 PS2]: let DayKind::Open(dr) = kind_of(19_784) else {
F crates/pull/src/calendar.rs:4394 [G2 PS2]: panic!("2024-03-02 traded");
F crates/pull/src/calendar.rs:4410:13 False [G2 PS2]: let DayKind::Open(full) = kind_of(20_668) else {
F crates/pull/src/calendar.rs:4411 [G2 PS2]: panic!("an ordinary weekday traded");
F crates/pull/src/calendar.rs:4584:13 False [G2 PS2]: let DayKind::Open(session) = cal.kind_of(20_382) else {
F crates/pull/src/calendar.rs:4585 [G2 PS2]: panic!("it traded");
U crates/pull/src/calendar.rs:5169:13 False [G2 PS2]: let (Ok(start), Ok(end)) = (
U crates/pull/src/calendar.rs:5173 [G2 PS2]: return changed;
F crates/pull/src/calendar.rs:5370:17 False [G2 PS2]: let DayKind::Open(session) = exchange_kind_of(day) else {
F crates/pull/src/calendar.rs:5371 [G2 PS2]: panic!("{label} is open on the calendar of record")
F crates/pull/src/calendar.rs:5388 [G2 PS2]: other => panic!("{label}: sourced_kind_of answered {other:?}"),
F crates/pull/src/calendar.rs:5444 [G2 PS2]: _ => false,
F crates/pull/src/calendar.rs:5448 [G2 PS2]: matches!(
F crates/pull/src/calendar.rs:6451 [G2 PS2]: ExpiryDay::Unmeasured => panic!("day {day} is answered"),
F crates/pull/src/calendar.rs:6467:21 False [G2 PS2]: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
F crates/pull/src/calendar.rs:6467:49 False [G2 PS2]: assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
IGNORED crates/pull/src/calendar.rs:6564 [G2 PS2]: fn calendar_of_record_costs() {
IGNORED crates/pull/src/calendar.rs:6565 [G2 PS2]: let mut days: Vec<i64> = SPECIAL_SESSIONS.iter().map(|s| s.day).collect();
IGNORED crates/pull/src/calendar.rs:6566 [G2 PS2]: days.extend(HALTS.iter().map(|h| h.day));
IGNORED crates/pull/src/calendar.rs:6567 [G2 PS2]: days.extend((SOURCED_FIRST_DAY..=SOURCED_LAST_DAY).step_by(97));
IGNORED crates/pull/src/calendar.rs:6568 [G2 PS2]: days.push(SOURCED_LAST_DAY + 1);
IGNORED crates/pull/src/calendar.rs:6569 [G2 PS2]: measure("exchange kind of a day", &days, exchange_kind_of);
IGNORED crates/pull/src/calendar.rs:6570 [G2 PS2]: measure("phase at one minute", &days, |d| {
IGNORED crates/pull/src/calendar.rs:6571 [G2 PS2]: phase_at(Venue::NseCash, d, 915, CasEligibility::Eligible)
IGNORED crates/pull/src/calendar.rs:6572 [G2 PS2]: });
IGNORED crates/pull/src/calendar.rs:6573 [G2 PS2]: measure("phases of a whole day", &days, |d| {
IGNORED crates/pull/src/calendar.rs:6574 [G2 PS2]: phases_of(Venue::NseCash, d, CasEligibility::Eligible)
IGNORED crates/pull/src/calendar.rs:6575 [G2 PS2]: });
IGNORED crates/pull/src/calendar.rs:6576 [G2 PS2]: measure("settlement on or before", &days, settlement_on_or_before);
IGNORED crates/pull/src/calendar.rs:6577 [G2 PS2]: measure("record of a day", &days, record_of);
IGNORED crates/pull/src/calendar.rs:6578 [G2 PS2]: }
IGNORED crates/pull/src/calendar.rs:6583 [G2 PS2]: fn measure<T>(name: &str, days: &[i64], op: impl Fn(i64) -> T) {
IGNORED crates/pull/src/calendar.rs:6586 [G2 PS2]: let mut samples = Vec::new();
IGNORED crates/pull/src/calendar.rs:6587 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/calendar.rs:6588 [G2 PS2]: for &day in days {
IGNORED crates/pull/src/calendar.rs:6589 [G2 PS2]: let started = Instant::now();
IGNORED crates/pull/src/calendar.rs:6590 [G2 PS2]: for _ in 0..16 {
IGNORED crates/pull/src/calendar.rs:6591 [G2 PS2]: black_box(op(black_box(day)));
IGNORED crates/pull/src/calendar.rs:6592 [G2 PS2]: }
IGNORED crates/pull/src/calendar.rs:6593 [G2 PS2]: samples.push(started.elapsed().as_nanos() / 16);
IGNORED crates/pull/src/calendar.rs:6596 [G2 PS2]: samples.sort_unstable();
IGNORED crates/pull/src/calendar.rs:6597 [G2 PS2]: let pick = |permille: usize| {
IGNORED crates/pull/src/calendar.rs:6598 [G2 PS2]: samples
IGNORED crates/pull/src/calendar.rs:6599 [G2 PS2]: .get(samples.len() * permille / 1_000)
IGNORED crates/pull/src/calendar.rs:6600 [G2 PS2]: .copied()
IGNORED crates/pull/src/calendar.rs:6601 [G2 PS2]: .unwrap_or(u128::MAX)
IGNORED crates/pull/src/calendar.rs:6602 [G2 PS2]: };
IGNORED crates/pull/src/calendar.rs:6603 [G2 PS2]: let (p50, p99) = (pick(500), pick(990));
IGNORED crates/pull/src/calendar.rs:6604 [G2 PS2]: eprintln!("{name}: p50 {p50} ns, p99 {p99} ns, n = {}", samples.len());
IGNORED crates/pull/src/calendar.rs:6605 [G2 PS2]: assert!(p99 < 50_000, "{name}: p99 {p99} ns");
IGNORED crates/pull/src/calendar.rs:6606 [G2 PS2]: }
F crates/pull/src/calendar_tradable_tests.rs:247:13 False [G1 PS1]: notice.source.contains(" https://") && notice.source.contains(" of "),
F crates/pull/src/calendar_tradable_tests.rs:247:52 False [G1 PS1]: notice.source.contains(" https://") && notice.source.contains(" of "),
F crates/pull/src/calendar_tradable_tests.rs:252 [G1 PS1]: matches!(
F crates/pull/src/calendar_tradable_tests.rs:341:13 False [G1 PS1]: assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
F crates/pull/src/calendar_tradable_tests.rs:341:51 False [G1 PS1]: assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
F crates/pull/src/calendar_tradable_tests.rs:1028:20 False [G1 PS1]: if (record_first, record_last) == (SOURCED_FIRST_DAY, SOURCED_LAST_DAY)),
F crates/pull/src/calendar_tradable_tests.rs:1190:13 False [G1 PS1]: assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1190:37 False [G1 PS1]: assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1204:13 False [G1 PS1]: assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1204:37 False [G1 PS1]: assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1219:13 False [G1 PS1]: assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1219:35 False [G1 PS1]: assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1234:13 False [G1 PS1]: assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
F crates/pull/src/calendar_tradable_tests.rs:1234:37 False [G1 PS1]: assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
F crates/pull/src/capture.rs:598:13 False [G1 PS1]: !text.to_ascii_lowercase().contains("authorization")
F crates/pull/src/capture.rs:599:20 False [G1 PS1]: && !text.to_ascii_lowercase().contains("access-token"),
U crates/pull/src/cash_auction.rs:713 [G1 PS1]: None => break,
F crates/pull/src/cash_auction.rs:971 [G1 PS1]: matches!(
IGNORED crates/pull/src/cash_auction.rs:1453 [G1 PS1]: fn actual_dated_masters_match_every_current_fno_cash_identity() {
IGNORED crates/pull/src/cash_auction.rs:1456 [G1 PS1]: let root = std::path::PathBuf::from(
IGNORED crates/pull/src/cash_auction.rs:1457 [G1 PS1]: std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR").expect("local sample directory"),
IGNORED crates/pull/src/cash_auction.rs:1459 [G1 PS1]: let members: Vec<_> = FNO_UNDERLYINGS
IGNORED crates/pull/src/cash_auction.rs:1460 [G1 PS1]: .into_iter()
IGNORED crates/pull/src/cash_auction.rs:1461 [G1 PS1]: .filter(|symbol| NTM_INDEX.contains(symbol))
IGNORED crates/pull/src/cash_auction.rs:1462 [G1 PS1]: .collect();
IGNORED crates/pull/src/cash_auction.rs:1463 [G1 PS1]: assert_eq!(members.len(), 208, "current F&O/total-market intersection");
IGNORED crates/pull/src/cash_auction.rs:1464 [G1 PS1]: let first = Day::new(2026, 8, 3).unwrap().days_from_epoch();
IGNORED crates/pull/src/cash_auction.rs:1465 [G1 PS1]: let last = Day::new(2026, 9, 4).unwrap().days_from_epoch();
IGNORED crates/pull/src/cash_auction.rs:1466 [G1 PS1]: let mut days = 0;
IGNORED crates/pull/src/cash_auction.rs:1467 [G1 PS1]: let mut mismatches = Vec::new();
IGNORED crates/pull/src/cash_auction.rs:1468 [G1 PS1]: for number in first..=last {
IGNORED crates/pull/src/cash_auction.rs:1469 [G1 PS1]: let day = Day::from_days(number).unwrap();
IGNORED crates/pull/src/cash_auction.rs:1470 [G1 PS1]: let path = root.join(format!(
IGNORED crates/pull/src/cash_auction.rs:1471 [G1 PS1]: "NSE_CM_security_{:02}{:02}{:04}.csv.gz",
IGNORED crates/pull/src/cash_auction.rs:1472 [G1 PS1]: day.day(),
IGNORED crates/pull/src/cash_auction.rs:1473 [G1 PS1]: day.month(),
IGNORED crates/pull/src/cash_auction.rs:1474 [G1 PS1]: day.year()
IGNORED crates/pull/src/cash_auction.rs:1475 [G1 PS1]: ));
IGNORED crates/pull/src/cash_auction.rs:1476 [G1 PS1]: if !path.try_exists().unwrap() {
IGNORED crates/pull/src/cash_auction.rs:1476:16 False [G1 PS1]: if !path.try_exists().unwrap() {
IGNORED crates/pull/src/cash_auction.rs:1476:16 True [G1 PS1]: if !path.try_exists().unwrap() {
IGNORED crates/pull/src/cash_auction.rs:1477 [G1 PS1]: continue;
IGNORED crates/pull/src/cash_auction.rs:1478 [G1 PS1]: }
IGNORED crates/pull/src/cash_auction.rs:1479 [G1 PS1]: days += 1;
IGNORED crates/pull/src/cash_auction.rs:1480 [G1 PS1]: let master = read_actual_master(&path);
IGNORED crates/pull/src/cash_auction.rs:1481 [G1 PS1]: let mut eligible = 0;
IGNORED crates/pull/src/cash_auction.rs:1482 [G1 PS1]: let mut ineligible = 0;
IGNORED crates/pull/src/cash_auction.rs:1483 [G1 PS1]: for symbol in &members {
IGNORED crates/pull/src/cash_auction.rs:1484 [G1 PS1]: let isin = nse_isin(symbol).expect("core cash identity must carry an ISIN");
IGNORED crates/pull/src/cash_auction.rs:1485 [G1 PS1]: match master.eligibility(symbol, isin.as_str()) {
IGNORED crates/pull/src/cash_auction.rs:1486 [G1 PS1]: Ok(true) => eligible += 1,
IGNORED crates/pull/src/cash_auction.rs:1487 [G1 PS1]: Ok(false) => {
IGNORED crates/pull/src/cash_auction.rs:1488 [G1 PS1]: ineligible += 1;
IGNORED crates/pull/src/cash_auction.rs:1489 [G1 PS1]: eprintln!("{day} {symbol} ISIN={isin}: explicit eligibility=0");
IGNORED crates/pull/src/cash_auction.rs:1490 [G1 PS1]: }
IGNORED crates/pull/src/cash_auction.rs:1491 [G1 PS1]: Err(why) => {
IGNORED crates/pull/src/cash_auction.rs:1492 [G1 PS1]: let same_symbol = master.by_symbol.get(*symbol);
IGNORED crates/pull/src/cash_auction.rs:1493 [G1 PS1]: let mut same_isin: Vec<_> = master
IGNORED crates/pull/src/cash_auction.rs:1494 [G1 PS1]: .by_symbol
IGNORED crates/pull/src/cash_auction.rs:1495 [G1 PS1]: .iter()
IGNORED crates/pull/src/cash_auction.rs:1496 [G1 PS1]: .filter(|(_, identity)| identity.isin == isin.as_str())
IGNORED crates/pull/src/cash_auction.rs:1497 [G1 PS1]: .map(|(name, identity)| format!("{name}: {identity:?}"))
IGNORED crates/pull/src/cash_auction.rs:1498 [G1 PS1]: .collect();
IGNORED crates/pull/src/cash_auction.rs:1499 [G1 PS1]: same_isin.sort_unstable();
IGNORED crates/pull/src/cash_auction.rs:1500 [G1 PS1]: mismatches.push(format!("{day} {symbol} expected_ISIN={isin}: {why}; same_symbol={same_symbol:?}; same_ISIN_rows={same_isin:?}"));
IGNORED crates/pull/src/cash_auction.rs:1504 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/cash_auction.rs:1506 [G1 PS1]: members.len() - eligible - ineligible
IGNORED crates/pull/src/cash_auction.rs:1509 [G1 PS1]: assert_eq!(days, 25, "all supplied dated masters must be inspected");
IGNORED crates/pull/src/cash_auction.rs:1510 [G1 PS1]: for mismatch in &mismatches {
IGNORED crates/pull/src/cash_auction.rs:1511 [G1 PS1]: eprintln!("{mismatch}");
IGNORED crates/pull/src/cash_auction.rs:1512 [G1 PS1]: }
IGNORED crates/pull/src/cash_auction.rs:1513 [G1 PS1]: assert!(
IGNORED crates/pull/src/cash_auction.rs:1514 [G1 PS1]: mismatches.is_empty(),
IGNORED crates/pull/src/cash_auction.rs:1516 [G1 PS1]: mismatches.len()
IGNORED crates/pull/src/cash_auction.rs:1518 [G1 PS1]: }
F crates/pull/src/cash_session_cache.rs:933:17 False [G1 PS1]: assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
F crates/pull/src/cash_session_cache.rs:933:47 False [G1 PS1]: assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
F crates/pull/src/cash_session_cache.rs:938:17 False [G1 PS1]: assert!(why.contains("absent") && why.contains("no download attempted"));
F crates/pull/src/cash_session_cache.rs:938:43 False [G1 PS1]: assert!(why.contains("absent") && why.contains("no download attempted"));
F crates/pull/src/cash_session_cache.rs:940:17 False [G1 PS1]: assert!(!payload.exists() && !metadata.exists());
F crates/pull/src/cash_session_cache.rs:940:38 False [G1 PS1]: assert!(!payload.exists() && !metadata.exists());
F crates/pull/src/cash_session_cache.rs:983:17 False [G1 PS1]: why.contains("UNVERIFIED") && why.contains("receipt"),
F crates/pull/src/cash_session_cache.rs:983:47 False [G1 PS1]: why.contains("UNVERIFIED") && why.contains("receipt"),
F crates/pull/src/cash_session_cache.rs:1005:13 False [G1 PS1]: why.contains("unavailable") && why.contains("30 ms bound"),
F crates/pull/src/cash_session_cache.rs:1005:44 False [G1 PS1]: why.contains("unavailable") && why.contains("30 ms bound"),
IGNORED crates/pull/src/cash_session_cache.rs:1134 [G1 PS1]: fn held_masters_give_every_cas_session_its_auction_band() -> Result<(), String> {
IGNORED crates/pull/src/cash_session_cache.rs:1137 [G1 PS1]: let root = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
IGNORED crates/pull/src/cash_session_cache.rs:1138 [G1 PS1]: .filter(|value| !value.is_empty())
IGNORED crates/pull/src/cash_session_cache.rs:1139 [G1 PS1]: .map(PathBuf::from)
IGNORED crates/pull/src/cash_session_cache.rs:1140 [G1 PS1]: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the receipted master directory")?;
IGNORED crates/pull/src/cash_session_cache.rs:1141 [G1 PS1]: let last = Day::new(2026, 9, 30).map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1142 [G1 PS1]: let days = required_days(Window::new(day(3), last).map_err(|why| why.to_string())?)?;
IGNORED crates/pull/src/cash_session_cache.rs:1143 [G1 PS1]: assert_eq!(days.len(), 42);
IGNORED crates/pull/src/cash_session_cache.rs:1144 [G1 PS1]: let reliance = nse_isin("RELIANCE").ok_or("RELIANCE has a cash ISIN")?;
IGNORED crates/pull/src/cash_session_cache.rs:1145 [G1 PS1]: for date in &days {
IGNORED crates/pull/src/cash_session_cache.rs:1146 [G1 PS1]: let master = read_local_lifecycle(&root, *date)?;
IGNORED crates/pull/src/cash_session_cache.rs:1147 [G1 PS1]: let phases = master.cash_phases("RELIANCE", reliance.as_str())?;
IGNORED crates/pull/src/cash_session_cache.rs:1148 [G1 PS1]: assert_eq!(phases.minutes(calendar::Phase::Continuous), 360, "{date}");
IGNORED crates/pull/src/cash_session_cache.rs:1149 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1150 [G1 PS1]: phases.phase_at(15 * 60 + 15),
IGNORED crates/pull/src/cash_session_cache.rs:1154 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1155 [G1 PS1]: phases.phase_at(15 * 60 + 30),
IGNORED crates/pull/src/cash_session_cache.rs:1160 [G1 PS1]: let sept_24 = Day::new(2026, 9, 24).map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1161 [G1 PS1]: let master = read_local_lifecycle(&root, sept_24)?;
IGNORED crates/pull/src/cash_session_cache.rs:1162 [G1 PS1]: let dalbharat = nse_isin("DALBHARAT").ok_or("DALBHARAT has a cash ISIN")?;
IGNORED crates/pull/src/cash_session_cache.rs:1163 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1164 [G1 PS1]: master.cas_eligibility("DALBHARAT", dalbharat.as_str()),
IGNORED crates/pull/src/cash_session_cache.rs:1167 [G1 PS1]: let ineligible = master.cash_phases("DALBHARAT", dalbharat.as_str())?;
IGNORED crates/pull/src/cash_session_cache.rs:1168 [G1 PS1]: assert_eq!(ineligible.minutes(calendar::Phase::Continuous), 375);
IGNORED crates/pull/src/cash_session_cache.rs:1169 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1170 [G1 PS1]: ineligible.phase_at(15 * 60 + 15),
IGNORED crates/pull/src/cash_session_cache.rs:1173 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/cash_session_cache.rs:1175 [G1 PS1]: days.len()
IGNORED crates/pull/src/cash_session_cache.rs:1177 [G1 PS1]: Ok(())
IGNORED crates/pull/src/cash_session_cache.rs:1178 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1182 [G1 PS1]: fn actual_receipted_lifecycle_snapshot_never_claims_complete_history() -> Result<(), String> {
IGNORED crates/pull/src/cash_session_cache.rs:1186 [G1 PS1]: let root = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
IGNORED crates/pull/src/cash_session_cache.rs:1187 [G1 PS1]: .filter(|value| !value.is_empty())
IGNORED crates/pull/src/cash_session_cache.rs:1188 [G1 PS1]: .map(PathBuf::from)
IGNORED crates/pull/src/cash_session_cache.rs:1189 [G1 PS1]: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the receipted master directory")?;
IGNORED crates/pull/src/cash_session_cache.rs:1190 [G1 PS1]: let date = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1191 [G1 PS1]: let master = read_local_lifecycle(&root, date)?;
IGNORED crates/pull/src/cash_session_cache.rs:1192 [G1 PS1]: assert_eq!(master.provenance().master_day, date);
IGNORED crates/pull/src/cash_session_cache.rs:1193 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1194 [G1 PS1]: master.provenance().source_url,
IGNORED crates/pull/src/cash_session_cache.rs:1197 [G1 PS1]: let mut count = 0;
IGNORED crates/pull/src/cash_session_cache.rs:1198 [G1 PS1]: let mut unavailable = 0;
IGNORED crates/pull/src/cash_session_cache.rs:1199 [G1 PS1]: let mut partial = 0;
IGNORED crates/pull/src/cash_session_cache.rs:1200 [G1 PS1]: for symbol in FNO_UNDERLYINGS
IGNORED crates/pull/src/cash_session_cache.rs:1201 [G1 PS1]: .into_iter()
IGNORED crates/pull/src/cash_session_cache.rs:1202 [G1 PS1]: .filter(|symbol| NTM_INDEX.contains(symbol))
IGNORED crates/pull/src/cash_session_cache.rs:1204 [G1 PS1]: let isin = nse_isin(symbol).expect("current cash table identity");
IGNORED crates/pull/src/cash_session_cache.rs:1205 [G1 PS1]: let report = master.inspect(symbol, isin.as_str())?;
IGNORED crates/pull/src/cash_session_cache.rs:1206 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1211 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1216 [G1 PS1]: match report.identity.metadata.listing {
IGNORED crates/pull/src/cash_session_cache.rs:1218 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1223 [G1 PS1]: unavailable += 1;
IGNORED crates/pull/src/cash_session_cache.rs:1226 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1231 [G1 PS1]: partial += 1;
IGNORED crates/pull/src/cash_session_cache.rs:1233 [G1 PS1]: ref other => {
IGNORED crates/pull/src/cash_session_cache.rs:1234 [G1 PS1]: return Err(format!(
IGNORED crates/pull/src/cash_session_cache.rs:1235 [G1 PS1]: "unexpected actual listing evidence for {symbol}: {other:?}"
IGNORED crates/pull/src/cash_session_cache.rs:1236 [G1 PS1]: ));
IGNORED crates/pull/src/cash_session_cache.rs:1239 [G1 PS1]: count += 1;
IGNORED crates/pull/src/cash_session_cache.rs:1241 [G1 PS1]: assert_eq!(count, 208);
IGNORED crates/pull/src/cash_session_cache.rs:1242 [G1 PS1]: assert_eq!(partial + unavailable, count);
IGNORED crates/pull/src/cash_session_cache.rs:1243 [G1 PS1]: assert!(partial > 0 && unavailable > 0);
IGNORED crates/pull/src/cash_session_cache.rs:1243:17 False [G1 PS1]: assert!(partial > 0 && unavailable > 0);
IGNORED crates/pull/src/cash_session_cache.rs:1243:17 True [G1 PS1]: assert!(partial > 0 && unavailable > 0);
IGNORED crates/pull/src/cash_session_cache.rs:1243:32 False [G1 PS1]: assert!(partial > 0 && unavailable > 0);
IGNORED crates/pull/src/cash_session_cache.rs:1243:32 True [G1 PS1]: assert!(partial > 0 && unavailable > 0);
IGNORED crates/pull/src/cash_session_cache.rs:1244 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1245 [G1 PS1]: master
IGNORED crates/pull/src/cash_session_cache.rs:1246 [G1 PS1]: .inspect("ADANIENT", "INE423A01024")?
IGNORED crates/pull/src/cash_session_cache.rs:1252 [G1 PS1]: for (symbol, isin, expected) in [
IGNORED crates/pull/src/cash_session_cache.rs:1253 [G1 PS1]: ("IRFC", "INE053F01010", Day::new(2021, 1, 29)),
IGNORED crates/pull/src/cash_session_cache.rs:1254 [G1 PS1]: ("FORCEMOT", "INE451A01017", Day::new(2019, 8, 19)),
IGNORED crates/pull/src/cash_session_cache.rs:1255 [G1 PS1]: ("RELIANCE", "INE002A01018", Day::new(1995, 11, 29)),
IGNORED crates/pull/src/cash_session_cache.rs:1256 [G1 PS1]: ] {
IGNORED crates/pull/src/cash_session_cache.rs:1257 [G1 PS1]: let report = master.inspect(symbol, isin)?;
IGNORED crates/pull/src/cash_session_cache.rs:1258 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1259 [G1 PS1]: report.identity.metadata.listing.date(),
IGNORED crates/pull/src/cash_session_cache.rs:1260 [G1 PS1]: Some(expected.expect("fixture date"))
IGNORED crates/pull/src/cash_session_cache.rs:1263 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/cash_session_cache.rs:1266 [G1 PS1]: Ok(())
IGNORED crates/pull/src/cash_session_cache.rs:1267 [G1 PS1]: }
F crates/pull/src/cash_session_cache.rs:1277 [G1 PS1]: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
F crates/pull/src/cash_session_cache.rs:1278 [G1 PS1]: unreachable!("{url}: a fully cached window must not use the network")
F crates/pull/src/cash_session_cache.rs:1526 [G1 PS1]: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
F crates/pull/src/cash_session_cache.rs:1527 [G1 PS1]: unreachable!("{url}: cached dates must never fetch again")
F crates/pull/src/cash_session_cache.rs:1607 [G1 PS1]: |url| -> std::future::Ready<Result<Vec<u8>, String>> {
F crates/pull/src/cash_session_cache.rs:1608 [G1 PS1]: unreachable!("{url}: a root with no receipt is refused before any fetch")
F crates/pull/src/cash_session_cache.rs:1626:17 False [G1 PS1]: assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
F crates/pull/src/cash_session_cache.rs:1626:49 False [G1 PS1]: assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
F crates/pull/src/cash_session_cache.rs:1650:17 False [G1 PS1]: assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
F crates/pull/src/cash_session_cache.rs:1650:49 False [G1 PS1]: assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
IGNORED crates/pull/src/cash_session_cache.rs:1874 [G1 PS1]: async fn official_25_dated_masters_install_and_validate_without_network() -> Result<(), String>
IGNORED crates/pull/src/cash_session_cache.rs:1875 [G1 PS1]: {
IGNORED crates/pull/src/cash_session_cache.rs:1876 [G1 PS1]: let source = std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR")
IGNORED crates/pull/src/cash_session_cache.rs:1877 [G1 PS1]: .filter(|value| !value.is_empty())
IGNORED crates/pull/src/cash_session_cache.rs:1878 [G1 PS1]: .map(PathBuf::from)
IGNORED crates/pull/src/cash_session_cache.rs:1879 [G1 PS1]: .ok_or("BRUTEX_NSE_CASH_SAMPLE_DIR must name the public NSE sample directory")?;
IGNORED crates/pull/src/cash_session_cache.rs:1880 [G1 PS1]: let end = Day::new(2026, 9, 4).map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1881 [G1 PS1]: let span = Window::new(day(3), end).map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1884 [G1 PS1]: let mut expected: Vec<_> = [
IGNORED crates/pull/src/cash_session_cache.rs:1885 [G1 PS1]: 3, 4, 5, 6, 7, 10, 11, 12, 13, 14, 17, 18, 19, 20, 21, 24, 25, 26, 27, 28, 31,
IGNORED crates/pull/src/cash_session_cache.rs:1886 [G1 PS1]: ]
IGNORED crates/pull/src/cash_session_cache.rs:1887 [G1 PS1]: .into_iter()
IGNORED crates/pull/src/cash_session_cache.rs:1888 [G1 PS1]: .map(day)
IGNORED crates/pull/src/cash_session_cache.rs:1889 [G1 PS1]: .collect();
IGNORED crates/pull/src/cash_session_cache.rs:1890 [G1 PS1]: for date in 1..=4 {
IGNORED crates/pull/src/cash_session_cache.rs:1891 [G1 PS1]: expected.push(Day::new(2026, 9, date).map_err(|why| why.to_string())?);
IGNORED crates/pull/src/cash_session_cache.rs:1893 [G1 PS1]: assert_eq!(expected.len(), 25);
IGNORED crates/pull/src/cash_session_cache.rs:1894 [G1 PS1]: let mut missing: std::collections::BTreeSet<_> = expected.iter().copied().collect();
IGNORED crates/pull/src/cash_session_cache.rs:1895 [G1 PS1]: let temp = Temp::new();
IGNORED crates/pull/src/cash_session_cache.rs:1896 [G1 PS1]: let entries = fs::read_dir(&source)
IGNORED crates/pull/src/cash_session_cache.rs:1897 [G1 PS1]: .map_err(|why| format!("cannot read sample directory {}: {why}", source.display()))?;
IGNORED crates/pull/src/cash_session_cache.rs:1898 [G1 PS1]: for entry in entries {
IGNORED crates/pull/src/cash_session_cache.rs:1899 [G1 PS1]: let entry = entry.map_err(|why| why.to_string())?;
IGNORED crates/pull/src/cash_session_cache.rs:1900 [G1 PS1]: let name = entry.file_name();
IGNORED crates/pull/src/cash_session_cache.rs:1901 [G1 PS1]: let Some(name) = name.to_str() else { continue };
IGNORED crates/pull/src/cash_session_cache.rs:1901:17 False [G1 PS1]: let Some(name) = name.to_str() else { continue };
IGNORED crates/pull/src/cash_session_cache.rs:1901:17 True [G1 PS1]: let Some(name) = name.to_str() else { continue };
IGNORED crates/pull/src/cash_session_cache.rs:1902 [G1 PS1]: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
IGNORED crates/pull/src/cash_session_cache.rs:1902:16 False [G1 PS1]: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
IGNORED crates/pull/src/cash_session_cache.rs:1902:16 True [G1 PS1]: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
IGNORED crates/pull/src/cash_session_cache.rs:1902:57 False [G1 PS1]: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
IGNORED crates/pull/src/cash_session_cache.rs:1902:57 True [G1 PS1]: if !name.starts_with("NSE_CM_security_") || !name.ends_with(".csv.gz") {
IGNORED crates/pull/src/cash_session_cache.rs:1903 [G1 PS1]: continue;
IGNORED crates/pull/src/cash_session_cache.rs:1904 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1905 [G1 PS1]: let date = sample_day(name)?;
IGNORED crates/pull/src/cash_session_cache.rs:1906 [G1 PS1]: if !missing.remove(&date) {
IGNORED crates/pull/src/cash_session_cache.rs:1906:16 False [G1 PS1]: if !missing.remove(&date) {
IGNORED crates/pull/src/cash_session_cache.rs:1906:16 True [G1 PS1]: if !missing.remove(&date) {
IGNORED crates/pull/src/cash_session_cache.rs:1907 [G1 PS1]: return Err(format!("unexpected or duplicate dated master: {name}"));
IGNORED crates/pull/src/cash_session_cache.rs:1908 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1909 [G1 PS1]: let bytes = read_limited(&entry.path(), MAX_COMPRESSED)?;
IGNORED crates/pull/src/cash_session_cache.rs:1910 [G1 PS1]: install(&temp.0, date, &bytes).map_err(|why| format!("{name}: {why}"))?;
IGNORED crates/pull/src/cash_session_cache.rs:1912 [G1 PS1]: if !missing.is_empty() {
IGNORED crates/pull/src/cash_session_cache.rs:1912:12 False [G1 PS1]: if !missing.is_empty() {
IGNORED crates/pull/src/cash_session_cache.rs:1912:12 True [G1 PS1]: if !missing.is_empty() {
IGNORED crates/pull/src/cash_session_cache.rs:1913 [G1 PS1]: return Err(format!(
IGNORED crates/pull/src/cash_session_cache.rs:1914 [G1 PS1]: "missing official dated master samples: {missing:?}"
IGNORED crates/pull/src/cash_session_cache.rs:1915 [G1 PS1]: ));
IGNORED crates/pull/src/cash_session_cache.rs:1916 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1917 [G1 PS1]: let mut observed = HashMap::new();
IGNORED crates/pull/src/cash_session_cache.rs:1918 [G1 PS1]: prepare_observed_with(&temp.0, &expected, &mut observed, |url| async move {
IGNORED crates/pull/src/cash_session_cache.rs:1919 [G1 PS1]: Err(format!("sample audit must not fetch: {url}"))
IGNORED crates/pull/src/cash_session_cache.rs:1920 [G1 PS1]: })
IGNORED crates/pull/src/cash_session_cache.rs:1921 [G1 PS1]: .await?;
IGNORED crates/pull/src/cash_session_cache.rs:1922 [G1 PS1]: assert_eq!(observed.len(), 25);
IGNORED crates/pull/src/cash_session_cache.rs:1923 [G1 PS1]: for date in expected {
IGNORED crates/pull/src/cash_session_cache.rs:1924 [G1 PS1]: let bytes =
IGNORED crates/pull/src/cash_session_cache.rs:1925 [G1 PS1]: read_entry(&temp.0, date)?.ok_or_else(|| format!("cache omitted {date}"))?;
IGNORED crates/pull/src/cash_session_cache.rs:1926 [G1 PS1]: let eligibility = decode(&bytes)?;
IGNORED crates/pull/src/cash_session_cache.rs:1927 [G1 PS1]: assert!(
IGNORED crates/pull/src/cash_session_cache.rs:1928 [G1 PS1]: eligibility.eligibility("HINDALCO", "INE038A01020")?,
IGNORED crates/pull/src/cash_session_cache.rs:1931 [G1 PS1]: assert!(
IGNORED crates/pull/src/cash_session_cache.rs:1932 [G1 PS1]: observed
IGNORED crates/pull/src/cash_session_cache.rs:1933 [G1 PS1]: .get(&date.days_from_epoch())
IGNORED crates/pull/src/cash_session_cache.rs:1934 [G1 PS1]: .expect("observed master retained")
IGNORED crates/pull/src/cash_session_cache.rs:1935 [G1 PS1]: .eligibility("HINDALCO", "INE038A01020")?
IGNORED crates/pull/src/cash_session_cache.rs:1937 [G1 PS1]: assert!(
IGNORED crates/pull/src/cash_session_cache.rs:1938 [G1 PS1]: !eligibility.eligibility("20MICRONS", "INE144J01027")?,
IGNORED crates/pull/src/cash_session_cache.rs:1942 [G1 PS1]: let result = prepare_with(&temp.0, span, |url| async move {
IGNORED crates/pull/src/cash_session_cache.rs:1943 [G1 PS1]: Err(format!("sample audit must not fetch: {url}"))
IGNORED crates/pull/src/cash_session_cache.rs:1944 [G1 PS1]: })
IGNORED crates/pull/src/cash_session_cache.rs:1945 [G1 PS1]: .await;
IGNORED crates/pull/src/cash_session_cache.rs:1946 [G1 PS1]: match required_days(span) {
IGNORED crates/pull/src/cash_session_cache.rs:1947 [G1 PS1]: Ok(days) => assert_eq!(result?.len(), days.len()),
IGNORED crates/pull/src/cash_session_cache.rs:1948 [G1 PS1]: Err(calendar_refusal) => {
IGNORED crates/pull/src/cash_session_cache.rs:1949 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/cash_session_cache.rs:1950 [G1 PS1]: result.expect_err("unmeasured calendar must refuse"),
IGNORED crates/pull/src/cash_session_cache.rs:1951 [G1 PS1]: calendar_refusal
IGNORED crates/pull/src/cash_session_cache.rs:1952 [G1 PS1]: );
IGNORED crates/pull/src/cash_session_cache.rs:1953 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/cash_session_cache.rs:1954 [G1 PS1]: "All 25 official masters validated and reloaded; full-window prepare refused: {calendar_refusal}"
IGNORED crates/pull/src/cash_session_cache.rs:1955 [G1 PS1]: );
IGNORED crates/pull/src/cash_session_cache.rs:1956 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1957 [G1 PS1]: }
IGNORED crates/pull/src/cash_session_cache.rs:1958 [G1 PS1]: Ok(())
IGNORED crates/pull/src/cash_session_cache.rs:1959 [G1 PS1]: }
F crates/pull/src/cash_session_cache.rs:2212:13 False [G1 PS1]: why.contains("readers or another installer held it")
F crates/pull/src/cash_session_cache.rs:2213:20 False [G1 PS1]: && why.contains("for the whole 30 ms bound")
F crates/pull/src/cash_session_cache.rs:2214:20 False [G1 PS1]: && why.contains("nothing was installed"),
F crates/pull/src/cash_session_cache.rs:2284:21 False [G1 PS1]: why.starts_with("UNVERIFIED cannot open local lifecycle lock")
F crates/pull/src/cash_session_cache.rs:2285:28 False [G1 PS1]: && why.contains(&lock.display().to_string()),
F crates/pull/src/cash_session_cache.rs:2377:21 False [G1 PS1]: why.starts_with(&format!(
F crates/pull/src/cash_session_cache.rs:2379:27 False [G1 PS1]: )) && why.ends_with("; crash durability UNVERIFIED"),
F crates/pull/src/cash_session_cache_lock_tests.rs:130:53 False [G3 PS3]: assert!(solo.iter().all(|&(_, hindalco, micro)| hindalco && !micro));
F crates/pull/src/cash_session_cache_lock_tests.rs:193:9 False [G3 PS3]: why.contains("unavailable")
F crates/pull/src/cash_session_cache_lock_tests.rs:194:16 False [G3 PS3]: && why.contains("an installer held it exclusively")
F crates/pull/src/cash_session_cache_lock_tests.rs:195:16 False [G3 PS3]: && why.contains("40 ms bound")
F crates/pull/src/cash_session_cache_lock_tests.rs:196:16 False [G3 PS3]: && why.contains("not read as absent"),
F crates/pull/src/cash_session_cache_lock_tests.rs:233:46 False [G3 PS3]: why.contains("is absent") || why.contains("missing locally"),
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:377:12 True [G3 PS3]: if let Some(held) = std::env::var_os("BRUTEX_AF03_HOLD") {
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:378 [G3 PS3]: let held = PathBuf::from(held);
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:379 [G3 PS3]: let file = OpenOptions::new()
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:380 [G3 PS3]: .read(true)
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:381 [G3 PS3]: .write(true)
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:382 [G3 PS3]: .open(&held)
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:383 [G3 PS3]: .expect("the parent's lock file");
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:384 [G3 PS3]: file.try_lock().expect("the child holds the day alone");
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:385 [G3 PS3]: fs::write(held.with_extension("ready"), b"held").expect("ready marker");
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:386 [G3 PS3]: thread::sleep(Duration::from_mins(2));
UNMEASURED crates/pull/src/cash_session_cache_lock_tests.rs:388 [G3 PS3]: std::process::exit(3);
F crates/pull/src/cash_session_cache_ps1_tests.rs:62:9 False [G1 PS1]: let Err(TryLockError::Error(shared)) = unlockable(&lock).try_lock_shared() else {
F crates/pull/src/cash_session_cache_ps1_tests.rs:63 [G1 PS1]: panic!("the host took a shared flock on an O_PATH descriptor");
F crates/pull/src/cash_session_cache_ps1_tests.rs:65:9 False [G1 PS1]: let Err(TryLockError::Error(exclusive)) = unlockable(&lock).try_lock() else {
F crates/pull/src/cash_session_cache_ps1_tests.rs:66 [G1 PS1]: panic!("the host took an exclusive flock on an O_PATH descriptor");
U crates/pull/src/census_store.rs:2362:8 True [G2 PS2]: if keys != census.keys() || rows != census.total_rows() {
U crates/pull/src/census_store.rs:2362:33 True [G2 PS2]: if keys != census.keys() || rows != census.total_rows() {
U crates/pull/src/census_store.rs:2363 [G2 PS2]: return Err(format!(
U crates/pull/src/census_store.rs:2364 [G2 PS2]: "{}: the split holds {keys} keys and {rows} rows, the census {} and {}; \
U crates/pull/src/census_store.rs:2365 [G2 PS2]: nothing was written",
U crates/pull/src/census_store.rs:2366 [G2 PS2]: legacy.display(),
U crates/pull/src/census_store.rs:2367 [G2 PS2]: census.keys(),
U crates/pull/src/census_store.rs:2368 [G2 PS2]: census.total_rows()
U crates/pull/src/census_store.rs:2369 [G2 PS2]: ));
U crates/pull/src/census_store.rs:2878:9 False [G2 PS2]: let Some(mut file) = open_requires(&at, true)? else {
U crates/pull/src/census_store.rs:2879 [G2 PS2]: return Err(format!(
U crates/pull/src/census_store.rs:2880 [G2 PS2]: "{} could not be created for the requirement; nothing was written",
U crates/pull/src/census_store.rs:2881 [G2 PS2]: at.display()
U crates/pull/src/census_store.rs:2882 [G2 PS2]: ));
U crates/pull/src/census_store.rs:3016:16 False [G2 PS2]: if let Some(magic) = image.get_mut(header) {
U crates/pull/src/census_store.rs:3019:16 False [G2 PS2]: if let Some(version) = image.get_mut(slot + STUB_VERSION_AT..slot + STUB_VERSION_AT + 2) {
U crates/pull/src/census_store.rs:3022:16 False [G2 PS2]: if let Some(text) = image.get_mut(slot + 64..slot + 64 + note.len()) {
F crates/pull/src/census_store_kept_tests.rs:697 [G2 PS2]: &mut |step: WholeStep| panic!("refused before any step, yet {step:?} ran"),
F crates/pull/src/census_store_kept_tests.rs:751 [G2 PS2]: &mut |step: WholeStep| panic!("refused before any step, yet {step:?} ran"),
F crates/pull/src/census_store_kept_tests.rs:777:9 False [G2 PS2]: why.contains(&vendor_dir(root, Vendor::Gdfl).display().to_string())
F crates/pull/src/census_store_kept_tests.rs:778:16 False [G2 PS2]: && why.contains("is not a directory"),
F crates/pull/src/census_store_kept_tests.rs:887:13 False [G2 PS2]: assert!(holds_sealed_stub_as_before(&stub) && holds_sealed_stub(&stub));
F crates/pull/src/census_store_kept_tests.rs:887:51 False [G2 PS2]: assert!(holds_sealed_stub_as_before(&stub) && holds_sealed_stub(&stub));
F crates/pull/src/census_store_kept_tests.rs:1033:17 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1);
F crates/pull/src/census_store_kept_tests.rs:1033:35 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1);
IGNORED crates/pull/src/census_store_kept_tests.rs:1099 [G2 PS2]: fn the_kept_publish_is_flat_measured() {
IGNORED crates/pull/src/census_store_kept_tests.rs:1102 [G2 PS2]: let base = std::env::var_os("R319_BENCH_ROOT").map_or_else(std::env::temp_dir, PathBuf::from);
IGNORED crates/pull/src/census_store_kept_tests.rs:1103 [G2 PS2]: let mut p50s = Vec::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1104 [G2 PS2]: for keys in [400_000_u32, 2_000_000, 4_000_000, 8_400_000] {
IGNORED crates/pull/src/census_store_kept_tests.rs:1105 [G2 PS2]: let root = base.join(format!("brutex-r319-bench-{keys}-{}", std::process::id()));
IGNORED crates/pull/src/census_store_kept_tests.rs:1106 [G2 PS2]: std::fs::create_dir_all(root.join("manifest")).expect("a bench root");
IGNORED crates/pull/src/census_store_kept_tests.rs:1107 [G2 PS2]: let built = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1108 [G2 PS2]: let capacity = u64::from(keys) + u64::from(N * FRESH) + 4_096;
IGNORED crates/pull/src/census_store_kept_tests.rs:1109 [G2 PS2]: let mut census =
IGNORED crates/pull/src/census_store_kept_tests.rs:1110 [G2 PS2]: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, capacity).expect("genesis");
IGNORED crates/pull/src/census_store_kept_tests.rs:1111 [G2 PS2]: for n in 0..keys {
IGNORED crates/pull/src/census_store_kept_tests.rs:1112 [G2 PS2]: census.record_held(row(n, 1)).expect("held");
IGNORED crates/pull/src/census_store_kept_tests.rs:1113 [G2 PS2]: }
IGNORED crates/pull/src/census_store_kept_tests.rs:1114 [G2 PS2]: let path = epoch_path(&root, Vendor::Gdfl, Segment::Fno, 1);
IGNORED crates/pull/src/census_store_kept_tests.rs:1115 [G2 PS2]: std::fs::create_dir_all(path.parent().expect("a parent")).expect("dir");
IGNORED crates/pull/src/census_store_kept_tests.rs:1116 [G2 PS2]: std::fs::write(&path, census.image()).expect("epoch 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1117 [G2 PS2]: drop(census);
IGNORED crates/pull/src/census_store_kept_tests.rs:1118 [G2 PS2]: let built = built.elapsed().as_millis();
IGNORED crates/pull/src/census_store_kept_tests.rs:1119 [G2 PS2]: let mut kept = Kept::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1120 [G2 PS2]: let whole = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1121 [G2 PS2]: let first = record_kept(
IGNORED crates/pull/src/census_store_kept_tests.rs:1122 [G2 PS2]: &mut kept,
IGNORED crates/pull/src/census_store_kept_tests.rs:1123 [G2 PS2]: &root,
IGNORED crates/pull/src/census_store_kept_tests.rs:1124 [G2 PS2]: Vendor::Gdfl,
IGNORED crates/pull/src/census_store_kept_tests.rs:1125 [G2 PS2]: &(0..FRESH).map(|i| row(keys + i, 1)).collect::<Vec<_>>(),
IGNORED crates/pull/src/census_store_kept_tests.rs:1127 [G2 PS2]: .expect("day 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1128 [G2 PS2]: let whole = whole.elapsed().as_micros();
IGNORED crates/pull/src/census_store_kept_tests.rs:1129 [G2 PS2]: assert_eq!(first[0].read, CensusRead::Whole("first"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1130 [G2 PS2]: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
IGNORED crates/pull/src/census_store_kept_tests.rs:1131 [G2 PS2]: for d in 1..=N {
IGNORED crates/pull/src/census_store_kept_tests.rs:1132 [G2 PS2]: let rows: Vec<Held> = (0..FRESH).map(|i| row(keys + d * FRESH + i, 1)).collect();
IGNORED crates/pull/src/census_store_kept_tests.rs:1133 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1134 [G2 PS2]: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
IGNORED crates/pull/src/census_store_kept_tests.rs:1135 [G2 PS2]: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/census_store_kept_tests.rs:1136 [G2 PS2]: assert_eq!(got[0].read, CensusRead::Kept, "{keys}: day {d}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1138 [G2 PS2]: let (p50, p99, max) = percentiles(&mut micros);
IGNORED crates/pull/src/census_store_kept_tests.rs:1139 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1143 [G2 PS2]: std::fs::metadata(&path).map_or(0, |m| m.len())
IGNORED crates/pull/src/census_store_kept_tests.rs:1145 [G2 PS2]: p50s.push(p50);
IGNORED crates/pull/src/census_store_kept_tests.rs:1146 [G2 PS2]: drop(kept);
IGNORED crates/pull/src/census_store_kept_tests.rs:1147 [G2 PS2]: let _ = std::fs::remove_dir_all(&root);
IGNORED crates/pull/src/census_store_kept_tests.rs:1149 [G2 PS2]: let (small, large) = (p50s[0], p50s[3]);
IGNORED crates/pull/src/census_store_kept_tests.rs:1150 [G2 PS2]: eprintln!("R3-19 ratio p50(8.4M) / p50(0.4M) = {large} / {small}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1151 [G2 PS2]: assert!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1152 [G2 PS2]: large <= small.saturating_mul(3).max(1),
IGNORED crates/pull/src/census_store_kept_tests.rs:1155 [G2 PS2]: }
IGNORED crates/pull/src/census_store_kept_tests.rs:1166 [G2 PS2]: fn the_kept_publish_on_a_copy_of_a_real_epoch_measured() {
IGNORED crates/pull/src/census_store_kept_tests.rs:1169 [G2 PS2]: let root = PathBuf::from(std::env::var_os("R319_REAL_ROOT").expect("R319_REAL_ROOT"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1170 [G2 PS2]: let invented = |n: u32| {
IGNORED crates/pull/src/census_store_kept_tests.rs:1171 [G2 PS2]: let mut held = row(n, 1);
IGNORED crates/pull/src/census_store_kept_tests.rs:1172 [G2 PS2]: held.entry.key.symbol = Symbol::new("INVENTEDR").expect("a symbol");
IGNORED crates/pull/src/census_store_kept_tests.rs:1173 [G2 PS2]: held
IGNORED crates/pull/src/census_store_kept_tests.rs:1174 [G2 PS2]: };
IGNORED crates/pull/src/census_store_kept_tests.rs:1175 [G2 PS2]: let mut kept = Kept::new();
IGNORED crates/pull/src/census_store_kept_tests.rs:1176 [G2 PS2]: let whole = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1177 [G2 PS2]: let first = record_kept(
IGNORED crates/pull/src/census_store_kept_tests.rs:1178 [G2 PS2]: &mut kept,
IGNORED crates/pull/src/census_store_kept_tests.rs:1179 [G2 PS2]: &root,
IGNORED crates/pull/src/census_store_kept_tests.rs:1180 [G2 PS2]: Vendor::Gdfl,
IGNORED crates/pull/src/census_store_kept_tests.rs:1181 [G2 PS2]: &(0..FRESH).map(invented).collect::<Vec<_>>(),
IGNORED crates/pull/src/census_store_kept_tests.rs:1183 [G2 PS2]: .expect("day 1");
IGNORED crates/pull/src/census_store_kept_tests.rs:1184 [G2 PS2]: let whole = whole.elapsed().as_micros();
IGNORED crates/pull/src/census_store_kept_tests.rs:1185 [G2 PS2]: assert_eq!(first[0].read, CensusRead::Whole("first"));
IGNORED crates/pull/src/census_store_kept_tests.rs:1186 [G2 PS2]: let entries = first[0].entries;
IGNORED crates/pull/src/census_store_kept_tests.rs:1187 [G2 PS2]: let mut micros = Vec::with_capacity(usize::try_from(N).unwrap());
IGNORED crates/pull/src/census_store_kept_tests.rs:1188 [G2 PS2]: for d in 1..=N {
IGNORED crates/pull/src/census_store_kept_tests.rs:1189 [G2 PS2]: let rows: Vec<Held> = (0..FRESH).map(|i| invented(d * FRESH + i)).collect();
IGNORED crates/pull/src/census_store_kept_tests.rs:1190 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/src/census_store_kept_tests.rs:1191 [G2 PS2]: let got = record_kept(&mut kept, &root, Vendor::Gdfl, &rows).expect("a day");
IGNORED crates/pull/src/census_store_kept_tests.rs:1192 [G2 PS2]: micros.push(start.elapsed().as_micros());
IGNORED crates/pull/src/census_store_kept_tests.rs:1193 [G2 PS2]: assert_eq!(got[0].read, CensusRead::Kept, "day {d}");
IGNORED crates/pull/src/census_store_kept_tests.rs:1195 [G2 PS2]: let (p50, p99, max) = percentiles(&mut micros);
IGNORED crates/pull/src/census_store_kept_tests.rs:1196 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/census_store_kept_tests.rs:1201 [G2 PS2]: }
F crates/pull/src/census_store_link_tests.rs:205 [G2 PS2]: target.display()
F crates/pull/src/census_store_link_tests.rs:286:17 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
F crates/pull/src/census_store_link_tests.rs:286:35 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1, "a file of its own");
F crates/pull/src/census_store_link_tests.rs:336:13 False [G2 PS2]: left.is_file() && left.nlink() == 1,
F crates/pull/src/census_store_link_tests.rs:336:31 False [G2 PS2]: left.is_file() && left.nlink() == 1,
F crates/pull/src/census_store_link_tests.rs:668:13 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1);
F crates/pull/src/census_store_link_tests.rs:668:31 False [G2 PS2]: assert!(meta.is_file() && meta.nlink() == 1);
F crates/pull/src/census_store_ps2_tests.rs:386:13 False [G2 PS2]: assert!(done[0].compacted.is_some() && done[0].degraded.is_none());
F crates/pull/src/census_store_ps2_tests.rs:386:44 False [G2 PS2]: assert!(done[0].compacted.is_some() && done[0].degraded.is_none());
F crates/pull/src/census_store_ps2_tests.rs:485:9 False [G2 PS2]: let Some(telemetry::OwnedValue::Str(why)) = line.field("why") else {
F crates/pull/src/census_store_ps2_tests.rs:486 [G2 PS2]: panic!("no why: {line:?}");
F crates/pull/src/census_store_ps2_tests.rs:977:13 False [G2 PS2]: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
F crates/pull/src/census_store_ps2_tests.rs:977:39 False [G2 PS2]: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
F crates/pull/src/census_store_ps2_tests.rs:977:68 False [G2 PS2]: assert!(done.closed.is_empty() && done.left_open.is_empty() && done.run.is_none());
F crates/pull/src/census_store_ps2_tests.rs:1020:13 False [G2 PS2]: assert!(run >= before && run < ahead, "{run}");
F crates/pull/src/census_store_ps2_tests.rs:1020:30 False [G2 PS2]: assert!(run >= before && run < ahead, "{run}");
F crates/pull/src/census_store_tests.rs:307 [G2 PS2]: done.refused[0].1
F crates/pull/src/census_store_tests.rs:587 [G2 PS2]: done.left_open[2]
F crates/pull/src/census_store_tests.rs:610:13 False [G2 PS2]: assert!(again.closed.is_empty() && again.run.is_none());
F crates/pull/src/census_store_tests.rs:610:40 False [G2 PS2]: assert!(again.closed.is_empty() && again.run.is_none());
F crates/pull/src/census_store_tests.rs:719:13 False [G2 PS2]: assert!(again.closed.is_empty() && again.run.is_none());
F crates/pull/src/census_store_tests.rs:719:40 False [G2 PS2]: assert!(again.closed.is_empty() && again.run.is_none());
F crates/pull/src/census_store_tests.rs:818:13 False [G2 PS2]: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
F crates/pull/src/census_store_tests.rs:818:41 False [G2 PS2]: assert!(span.holds(ym(2019, 11)) && span.holds(ym(2020, 9)));
F crates/pull/src/census_store_tests.rs:819:13 False [G2 PS2]: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
F crates/pull/src/census_store_tests.rs:819:42 False [G2 PS2]: assert!(!span.holds(ym(2019, 10)) && !span.holds(ym(2020, 10)));
F crates/pull/src/census_store_tests.rs:823:13 False [G2 PS2]: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
F crates/pull/src/census_store_tests.rs:823:40 False [G2 PS2]: assert!(open.holds(ym(2026, 9)) && open.contains(Day::new(2026, 9, 30).expect("a day")));
F crates/pull/src/census_store_tests.rs:976 [G2 PS2]: panic!("an empty batch flushes nothing")
F crates/pull/src/census_store_tests.rs:1188:13 False [G2 PS2]: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
F crates/pull/src/census_store_tests.rs:1188:49 False [G2 PS2]: assert!(text.contains("SEALED (DB-B68)") && text.contains(V4_READER_SINCE));
F crates/pull/src/census_store_tests.rs:1313 [G2 PS2]: seal_receipt(&at)
F crates/pull/src/census_store_tests.rs:1327:17 False [G2 PS2]: assert!(!again.wrote && again.torn_receipt.is_none());
F crates/pull/src/census_store_tests.rs:1327:33 False [G2 PS2]: assert!(!again.wrote && again.torn_receipt.is_none());
F crates/pull/src/census_store_tests.rs:1336:13 False [G2 PS2]: assert!(seal.wrote && seal.torn_receipt.is_none());
F crates/pull/src/census_store_tests.rs:1336:27 False [G2 PS2]: assert!(seal.wrote && seal.torn_receipt.is_none());
F crates/pull/src/census_store_tests.rs:1350:9 False [G2 PS2]: why.contains("SEALED")
F crates/pull/src/census_store_tests.rs:1351:16 False [G2 PS2]: && why.contains("census-seal-v2")
F crates/pull/src/census_store_tests.rs:1352:16 False [G2 PS2]: && why.contains("nothing was written"),
F crates/pull/src/census_store_tests.rs:1362:9 False [G2 PS2]: why.contains("SEALED") && why.contains("not a regular file"),
F crates/pull/src/census_store_tests.rs:1362:35 False [G2 PS2]: why.contains("SEALED") && why.contains("not a regular file"),
F crates/pull/src/census_store_tests.rs:1497:9 False [G2 PS2]: why.contains("manifest/REQUIRES")
F crates/pull/src/census_store_tests.rs:1498:16 False [G2 PS2]: && why.contains("line 3")
F crates/pull/src/census_store_tests.rs:1499:16 False [G2 PS2]: && why.contains("\"census-v5\"")
F crates/pull/src/census_store_tests.rs:1500:16 False [G2 PS2]: && why.contains("this build does not have")
F crates/pull/src/census_store_tests.rs:1501:16 False [G2 PS2]: && why.contains("DB-B68"),
F crates/pull/src/census_store_tests.rs:1519:9 False [G2 PS2]: every.contains("line 4") && every.contains("\"bars-v4\""),
F crates/pull/src/census_store_tests.rs:1519:37 False [G2 PS2]: every.contains("line 4") && every.contains("\"bars-v4\""),
F crates/pull/src/census_store_tests.rs:1770:9 False [G2 PS2]: unread.contains("MIGRATED") && unread.contains("cannot be read"),
F crates/pull/src/census_store_tests.rs:1770:40 False [G2 PS2]: unread.contains("MIGRATED") && unread.contains("cannot be read"),
IGNORED crates/pull/src/census_store_tests.rs:1795 [G2 PS2]: fn legacy_drift_cost_per_call_is_measured() {
IGNORED crates/pull/src/census_store_tests.rs:1796 [G2 PS2]: let scratch = Scratch::new("DRIFT-TIMING");
IGNORED crates/pull/src/census_store_tests.rs:1797 [G2 PS2]: assert_eq!(
IGNORED crates/pull/src/census_store_tests.rs:1798 [G2 PS2]: record(scratch.root(), Vendor::Gdfl, &[row(0, 1)]).map(|c| c.len()),
IGNORED crates/pull/src/census_store_tests.rs:1801 [G2 PS2]: migrate(scratch.root(), Vendor::Gdfl).expect("it migrates");
IGNORED crates/pull/src/census_store_tests.rs:1802 [G2 PS2]: let real = std::env::var_os("BRUTEX_STORE_ROOT").map(PathBuf::from);
IGNORED crates/pull/src/census_store_tests.rs:1803 [G2 PS2]: let mut roots: Vec<(&str, &Path)> = vec![("scratch", scratch.root())];
IGNORED crates/pull/src/census_store_tests.rs:1804 [G2 PS2]: if let Some(real) = real.as_deref() {
IGNORED crates/pull/src/census_store_tests.rs:1804:12 False [G2 PS2]: if let Some(real) = real.as_deref() {
IGNORED crates/pull/src/census_store_tests.rs:1804:12 True [G2 PS2]: if let Some(real) = real.as_deref() {
IGNORED crates/pull/src/census_store_tests.rs:1805 [G2 PS2]: roots.push(("store", real));
IGNORED crates/pull/src/census_store_tests.rs:1806 [G2 PS2]: }
IGNORED crates/pull/src/census_store_tests.rs:1807 [G2 PS2]: for (label, root) in roots {
IGNORED crates/pull/src/census_store_tests.rs:1808 [G2 PS2]: let want = legacy_drift(root, Vendor::Gdfl);
IGNORED crates/pull/src/census_store_tests.rs:1809 [G2 PS2]: let mut micros: Vec<u128> = Vec::with_capacity(400);
IGNORED crates/pull/src/census_store_tests.rs:1810 [G2 PS2]: for _ in 0..400 {
IGNORED crates/pull/src/census_store_tests.rs:1811 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/census_store_tests.rs:1812 [G2 PS2]: let got = legacy_drift(root, Vendor::Gdfl);
IGNORED crates/pull/src/census_store_tests.rs:1813 [G2 PS2]: micros.push(started.elapsed().as_micros());
IGNORED crates/pull/src/census_store_tests.rs:1814 [G2 PS2]: assert_eq!(got, want, "{label}: the answer does not move between calls");
IGNORED crates/pull/src/census_store_tests.rs:1816 [G2 PS2]: micros.sort_unstable();
IGNORED crates/pull/src/census_store_tests.rs:1817 [G2 PS2]: let at = |q: usize| micros.get(399 * q / 100).copied().unwrap_or(u128::MAX);
IGNORED crates/pull/src/census_store_tests.rs:1818 [G2 PS2]: let max = micros.last().copied().unwrap_or(u128::MAX);
IGNORED crates/pull/src/census_store_tests.rs:1819 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/census_store_tests.rs:1821 [G2 PS2]: root.display(),
IGNORED crates/pull/src/census_store_tests.rs:1822 [G2 PS2]: at(50),
IGNORED crates/pull/src/census_store_tests.rs:1823 [G2 PS2]: at(99)
IGNORED crates/pull/src/census_store_tests.rs:1825 [G2 PS2]: assert!(at(99) < 20_000, "{label}: p99 {}us", at(99));
IGNORED crates/pull/src/census_store_tests.rs:1827 [G2 PS2]: }
F crates/pull/src/chain.rs:507:13 False [G2 PS2]: let Err(why) = month(
F crates/pull/src/chain.rs:516 [G2 PS2]: panic!("Dhan publishes no contract name to discover");
F crates/pull/src/chain.rs:530:13 False [G2 PS2]: let Err(why) = month(
F crates/pull/src/chain.rs:539 [G2 PS2]: panic!("an archive has no vendor to publish a lookup");
F crates/pull/src/chain.rs:619 [G2 PS2]: seen[0]
F crates/pull/src/chain.rs:716:13 False [G2 PS2]: chain.unreadable[0].contains("FINNIFTY")
F crates/pull/src/chain.rs:717:20 False [G2 PS2]: && chain.unreadable[0].contains("NIFTY was asked for"),
F crates/pull/src/chain.rs:720 [G2 PS2]: chain.unreadable[0]
F crates/pull/src/chain.rs:826 [G2 PS2]: from.asked.borrow()
F crates/pull/src/chain.rs:845:13 False [G2 PS2]: let Err(why) = month(
F crates/pull/src/chain.rs:854 [G2 PS2]: panic!("nothing answered, so the walk cannot have succeeded");
F crates/pull/src/chain_ps2_tests.rs:85:9 False [G2 PS2]: let Err(ChainError::Transport { url, why }) = refused else {
F crates/pull/src/chain_ps2_tests.rs:86 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/corporate_actions_cov_tests.rs:134:9 False [G3 PS3]: said.contains("\"BCO\"") && said.contains("share-days"),
F crates/pull/src/corporate_actions_cov_tests.rs:134:37 False [G3 PS3]: said.contains("\"BCO\"") && said.contains("share-days"),
F crates/pull/src/corporate_actions_cov_tests.rs:177 [G3 PS3]: refused(&backwards)
F crates/pull/src/corporate_actions_cov_tests.rs:407:9 False [G3 PS3]: let Finding::FactorDiffers { step, product } = &found[0] else {
F crates/pull/src/corporate_actions_cov_tests.rs:408 [G3 PS3]: panic!("{found:#?}");
F crates/pull/src/corporate_actions_tests.rs:159:13 False [G3 PS3]: assert!(r("4765/10000") <= factor && factor <= r("4767/10000"));
F crates/pull/src/corporate_actions_tests.rs:159:42 False [G3 PS3]: assert!(r("4765/10000") <= factor && factor <= r("4767/10000"));
F crates/pull/src/corporate_actions_tests.rs:264:9 False [G3 PS3]: let Slot::Store { table: loaded, .. } = load_runs(&store) else {
F crates/pull/src/corporate_actions_tests.rs:265 [G3 PS3]: panic!("the runs load from the store");
F crates/pull/src/corporate_actions_tests.rs:287:9 False [G3 PS3]: let Slot::Store {
F crates/pull/src/corporate_actions_tests.rs:293 [G3 PS3]: panic!("the ledger loads from the store");
F crates/pull/src/corporate_actions_tests.rs:366:9 False [G3 PS3]: r("104/100") < rights[0].ratio && rights[0].ratio < r("106/100"),
F crates/pull/src/corporate_actions_tests.rs:366:43 False [G3 PS3]: r("104/100") < rights[0].ratio && rights[0].ratio < r("106/100"),
F crates/pull/src/corporate_actions_tests.rs:368 [G3 PS3]: rights[0].ratio
F crates/pull/src/corporate_actions_tests.rs:409:9 False [G3 PS3]: said.contains("SPLITCO")
F crates/pull/src/corporate_actions_tests.rs:410:16 False [G3 PS3]: && said.contains("(2023-06-30, 2023-07-03]")
F crates/pull/src/corporate_actions_tests.rs:411:16 False [G3 PS3]: && said.contains("factor 2/1"),
F crates/pull/src/corporate_actions_tests.rs:426:75 False [G3 PS3]: assert!(matches!(&found[0], Finding::FactorDiffers { product, .. } if *product == r("5")));
F crates/pull/src/corporate_actions_tests.rs:519:13 False [G3 PS3]: assert!(!measured.excuses() && measured.class().is_none());
F crates/pull/src/corporate_actions_tests.rs:519:36 False [G3 PS3]: assert!(!measured.excuses() && measured.class().is_none());
F crates/pull/src/corporate_actions_tests.rs:583:9 False [G3 PS3]: let Slot::Refused(why) = load_actions(&store) else {
F crates/pull/src/corporate_actions_tests.rs:584 [G3 PS3]: panic!("a swapped table is never served");
F crates/pull/src/corporate_actions_tests.rs:592:9 False [G3 PS3]: said.contains("corporate_actions") && said.contains("checksum"),
F crates/pull/src/corporate_actions_tests.rs:592:47 False [G3 PS3]: said.contains("corporate_actions") && said.contains("checksum"),
F crates/pull/src/corporate_actions_tests.rs:663 [G3 PS3]: refused(&twice)
F crates/pull/src/corporate_actions_tests.rs:697 [G3 PS3]: refused(&overlap)
F crates/pull/src/corporate_actions_tests.rs:866 [G3 PS3]: refused(&backwards)
F crates/pull/src/corporate_actions_tests.rs:879 [G3 PS3]: refused(&kinds)
F crates/pull/src/corporate_actions_tests.rs:892 [G3 PS3]: refused(&symbols)
F crates/pull/src/corporate_actions_tests.rs:905 [G3 PS3]: refused(&split_up)
F crates/pull/src/corporate_actions_tests.rs:914 [G3 PS3]: refused(&runs_back)
F crates/pull/src/corporate_actions_tests.rs:1077:57 False [G3 PS3]: assert!(matches!(&found[0], Finding::NoRow(step) if step.symbol == "REALCO"));
F crates/pull/src/corporate_actions_tests.rs:1189 [G3 PS3]: alt_steps.iter().map(|j| j.verdict).collect::<Vec<_>>()
F crates/pull/src/corporate_actions_tests.rs:1216:9 False [G3 PS3]: said.contains("basis_anomaly SEGCO (2022-01-28, 2022-01-31]")
F crates/pull/src/corporate_actions_tests.rs:1217:16 False [G3 PS3]: && said.contains(
F crates/pull/src/corporate_actions_tests.rs:1265 [G3 PS3]: found[0]
F crates/pull/src/corporate_actions_tests.rs:1292:63 False [G3 PS3]: Verdict::Anomaly(Anomaly::WithinWobble { wobble }) if wobble > r("1012/1000")
F crates/pull/src/corporate_actions_tests.rs:1305:9 False [G3 PS3]: rows[0].evidence.contains("lasting (") && rows[0].day_known(),
F crates/pull/src/corporate_actions_tests.rs:1305:51 False [G3 PS3]: rows[0].evidence.contains("lasting (") && rows[0].day_known(),
F crates/pull/src/corporate_actions_tests.rs:1307 [G3 PS3]: rows[0].evidence
F crates/pull/src/corporate_actions_tests.rs:1312:78 False [G3 PS3]: Finding::BasisAnomaly { step, why: Anomaly::WithinWobble { .. } } if step.symbol == "WOBCO"
F crates/pull/src/corporate_actions_tests.rs:1317:58 False [G3 PS3]: .any(|f| matches!(f, Finding::NoRow(step) if step.symbol == "LASTCO"))
F crates/pull/src/corporate_actions_tests.rs:1476:13 False [G3 PS3]: assert!(tenfold.step.low <= r("472/47") && r("472/47") <= tenfold.step.high);
F crates/pull/src/corporate_actions_tests.rs:1476:48 False [G3 PS3]: assert!(tenfold.step.low <= r("472/47") && r("472/47") <= tenfold.step.high);
F crates/pull/src/corporate_actions_tests.rs:1492:13 False [G3 PS3]: assert!(tenfold.step.low <= row.ratio && row.ratio <= tenfold.step.high);
F crates/pull/src/corporate_actions_tests.rs:1492:46 False [G3 PS3]: assert!(tenfold.step.low <= row.ratio && row.ratio <= tenfold.step.high);
F crates/pull/src/corporate_actions_tests.rs:1546:13 False [G3 PS3]: assert!(r("1007/1000") < own && own < r("11/10"), "{own}");
F crates/pull/src/corporate_actions_tests.rs:1546:37 False [G3 PS3]: assert!(r("1007/1000") < own && own < r("11/10"), "{own}");
F crates/pull/src/corporate_actions_tests.rs:1589:22 False [G3 PS3]: .all(|j| !j.step.up() && j.step.far() < judged[0].step.near()),
F crates/pull/src/corporate_actions_tests.rs:1752:9 False [G3 PS3]: said.contains("product_too_large SPLITCO (2023-06-30, 2023-07-03]: the 2 rows there")
F crates/pull/src/corporate_actions_tests.rs:1753:16 False [G3 PS3]: && !said.contains("multiply to 1/1"),
F crates/pull/src/csv.rs:1524:50 False [G2 PS2]: assert!(!text.is_char_boundary(4) || !text.is_ascii());
F crates/pull/src/csv.rs:1900 [G2 PS2]: columns.count()
F crates/pull/src/csv.rs:1947:66 False [G2 PS2]: Err(CsvError::HeaderMismatch { ref got, .. }) if *got == swapped
F crates/pull/src/csv.rs:1956:62 False [G2 PS2]: Err(CsvError::HeaderMismatch { ref got, .. }) if got.is_empty()
U crates/pull/src/day_universe.rs:110:16 False [G3 PS3]: if let Some((_, why)) = series.get_mut(slot) {
F crates/pull/src/day_universe.rs:197:17 False [G3 PS3]: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
F crates/pull/src/day_universe.rs:197:44 False [G3 PS3]: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
F crates/pull/src/day_universe.rs:197:73 False [G3 PS3]: assert!(empty.series.is_empty() && empty.entrants.is_empty() && empty.leavers.is_empty());
F crates/pull/src/day_universe.rs:200:17 False [G3 PS3]: assert!(twice.entrants.is_empty() && twice.leavers.is_empty());
F crates/pull/src/day_universe.rs:200:46 False [G3 PS3]: assert!(twice.entrants.is_empty() && twice.leavers.is_empty());
F crates/pull/src/day_universe.rs:209:17 False [G3 PS3]: assert!(got.entrants.is_empty() && got.leavers.is_empty());
F crates/pull/src/day_universe.rs:209:44 False [G3 PS3]: assert!(got.entrants.is_empty() && got.leavers.is_empty());
U crates/pull/src/daycheck.rs:282:28 False [G2 PS2]: && let Some(bar) = rest.first()
F crates/pull/src/daycheck.rs:509:17 False [G2 PS2]: assert!(report.consistent() && !report.clean());
F crates/pull/src/daycheck.rs:509:40 False [G2 PS2]: assert!(report.consistent() && !report.clean());
U crates/pull/src/disk_projection.rs:261:20 False [G2 PS2]: if let Some(list) = per_class.get_mut(class.slot()) {
IGNORED crates/pull/src/disk_projection_tests.rs:391 [G2 PS2]: fn real_delivery_sizes_project_through_the_module_and_meet_the_floor() {
IGNORED crates/pull/src/disk_projection_tests.rs:392 [G2 PS2]: let read = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
IGNORED crates/pull/src/disk_projection_tests.rs:393 [G2 PS2]: let unit_of = |text: &str| {
IGNORED crates/pull/src/disk_projection_tests.rs:394 [G2 PS2]: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:394:16 False [G2 PS2]: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:394:16 True [G2 PS2]: if let Some(rest) = text.strip_prefix("U:") {
IGNORED crates/pull/src/disk_projection_tests.rs:395 [G2 PS2]: let (name, class) = rest.split_once('@').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:396 [G2 PS2]: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:396:28 False [G2 PS2]: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:396:28 True [G2 PS2]: let class = if class == "index" {
IGNORED crates/pull/src/disk_projection_tests.rs:397 [G2 PS2]: OptionClass::Index
IGNORED crates/pull/src/disk_projection_tests.rs:399 [G2 PS2]: OptionClass::Stock
IGNORED crates/pull/src/disk_projection_tests.rs:401 [G2 PS2]: Unit::Underlying {
IGNORED crates/pull/src/disk_projection_tests.rs:402 [G2 PS2]: name: name.to_owned(),
IGNORED crates/pull/src/disk_projection_tests.rs:403 [G2 PS2]: class,
IGNORED crates/pull/src/disk_projection_tests.rs:404 [G2 PS2]: }
IGNORED crates/pull/src/disk_projection_tests.rs:406 [G2 PS2]: let mut words = text.split('/');
IGNORED crates/pull/src/disk_projection_tests.rs:407 [G2 PS2]: let (feed, k, rung) = (
IGNORED crates/pull/src/disk_projection_tests.rs:408 [G2 PS2]: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:409 [G2 PS2]: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:410 [G2 PS2]: words.next().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:411 [G2 PS2]: );
IGNORED crates/pull/src/disk_projection_tests.rs:412 [G2 PS2]: Unit::Kind(KindKey::new(feed, k, rung))
IGNORED crates/pull/src/disk_projection_tests.rs:414 [G2 PS2]: };
IGNORED crates/pull/src/disk_projection_tests.rs:415 [G2 PS2]: let items = |name: &str| -> Vec<(Unit, String)> {
IGNORED crates/pull/src/disk_projection_tests.rs:416 [G2 PS2]: read(name)
IGNORED crates/pull/src/disk_projection_tests.rs:417 [G2 PS2]: .split(';')
IGNORED crates/pull/src/disk_projection_tests.rs:418 [G2 PS2]: .filter(|s| !s.is_empty())
IGNORED crates/pull/src/disk_projection_tests.rs:419 [G2 PS2]: .map(|item| {
IGNORED crates/pull/src/disk_projection_tests.rs:420 [G2 PS2]: let (key, value) = item.split_once('=').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:421 [G2 PS2]: (unit_of(key), value.to_owned())
IGNORED crates/pull/src/disk_projection_tests.rs:422 [G2 PS2]: })
IGNORED crates/pull/src/disk_projection_tests.rs:423 [G2 PS2]: .collect()
IGNORED crates/pull/src/disk_projection_tests.rs:424 [G2 PS2]: };
IGNORED crates/pull/src/disk_projection_tests.rs:425 [G2 PS2]: let mut totals = Totals::new();
IGNORED crates/pull/src/disk_projection_tests.rs:426 [G2 PS2]: for (unit, value) in items("BRUTEX_REAL_SAMPLE") {
IGNORED crates/pull/src/disk_projection_tests.rs:427 [G2 PS2]: let (bytes, days) = value.split_once(':').unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:428 [G2 PS2]: let (bytes, days) = (bytes.parse().unwrap(), days.parse().unwrap());
IGNORED crates/pull/src/disk_projection_tests.rs:429 [G2 PS2]: match unit {
IGNORED crates/pull/src/disk_projection_tests.rs:430 [G2 PS2]: Unit::Kind(key) => totals.add_kind(key, bytes, days).unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:431 [G2 PS2]: Unit::Underlying { name, class } => {
IGNORED crates/pull/src/disk_projection_tests.rs:432 [G2 PS2]: totals.add_underlying(&name, class, bytes, days).unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:433 [G2 PS2]: }
IGNORED crates/pull/src/disk_projection_tests.rs:436 [G2 PS2]: let sealed = totals.seal();
IGNORED crates/pull/src/disk_projection_tests.rs:437 [G2 PS2]: let plan: Vec<PlanItem> = items("BRUTEX_REAL_PLAN")
IGNORED crates/pull/src/disk_projection_tests.rs:438 [G2 PS2]: .into_iter()
IGNORED crates/pull/src/disk_projection_tests.rs:439 [G2 PS2]: .map(|(unit, days)| PlanItem {
IGNORED crates/pull/src/disk_projection_tests.rs:440 [G2 PS2]: unit,
IGNORED crates/pull/src/disk_projection_tests.rs:441 [G2 PS2]: days: days.parse().unwrap(),
IGNORED crates/pull/src/disk_projection_tests.rs:443 [G2 PS2]: })
IGNORED crates/pull/src/disk_projection_tests.rs:444 [G2 PS2]: .collect();
IGNORED crates/pull/src/disk_projection_tests.rs:445 [G2 PS2]: let projection = project(&sealed, &plan);
IGNORED crates/pull/src/disk_projection_tests.rs:446 [G2 PS2]: for line in &projection.lines {
IGNORED crates/pull/src/disk_projection_tests.rs:447 [G2 PS2]: println!(
IGNORED crates/pull/src/disk_projection_tests.rs:451 [G2 PS2]: assert_eq!(line.bytes, line.per_day * line.days, "days x bytes a day");
IGNORED crates/pull/src/disk_projection_tests.rs:452 [G2 PS2]: if let Basis::Extrapolated { class, .. } = line.basis {
IGNORED crates/pull/src/disk_projection_tests.rs:452:16 False [G2 PS2]: if let Basis::Extrapolated { class, .. } = line.basis {
IGNORED crates/pull/src/disk_projection_tests.rs:452:16 True [G2 PS2]: if let Basis::Extrapolated { class, .. } = line.basis {
IGNORED crates/pull/src/disk_projection_tests.rs:453 [G2 PS2]: assert_eq!(Some(line.per_day), sealed.median_of(class).map(|(m, _)| m));
IGNORED crates/pull/src/disk_projection_tests.rs:454 [G2 PS2]: assert!(
IGNORED crates/pull/src/disk_projection_tests.rs:455 [G2 PS2]: projection.extrapolated.contains(&line.unit.to_string()),
IGNORED crates/pull/src/disk_projection_tests.rs:458 [G2 PS2]: }
IGNORED crates/pull/src/disk_projection_tests.rs:460 [G2 PS2]: println!("{}", projection.summary());
IGNORED crates/pull/src/disk_projection_tests.rs:461 [G2 PS2]: assert!(
IGNORED crates/pull/src/disk_projection_tests.rs:462 [G2 PS2]: projection.unmeasured.is_empty(),
IGNORED crates/pull/src/disk_projection_tests.rs:465 [G2 PS2]: let space = parse_df(&read("BRUTEX_REAL_DF")).unwrap();
IGNORED crates/pull/src/disk_projection_tests.rs:466 [G2 PS2]: let verdict = floor_check(space, projection.bytes, DEFAULT_FLOOR);
IGNORED crates/pull/src/disk_projection_tests.rs:467 [G2 PS2]: println!(
IGNORED crates/pull/src/disk_projection_tests.rs:471 [G2 PS2]: DEFAULT_FLOOR.bytes_of(space.total)
IGNORED crates/pull/src/disk_projection_tests.rs:473 [G2 PS2]: let fits = space.free
IGNORED crates/pull/src/disk_projection_tests.rs:474 [G2 PS2]: >= projection
IGNORED crates/pull/src/disk_projection_tests.rs:475 [G2 PS2]: .bytes
IGNORED crates/pull/src/disk_projection_tests.rs:476 [G2 PS2]: .saturating_add(DEFAULT_FLOOR.bytes_of(space.total));
IGNORED crates/pull/src/disk_projection_tests.rs:477 [G2 PS2]: assert_eq!(
IGNORED crates/pull/src/disk_projection_tests.rs:478 [G2 PS2]: verdict.is_ok(),
IGNORED crates/pull/src/disk_projection_tests.rs:482 [G2 PS2]: }
F crates/pull/src/disk_projection_tests.rs:546:83 False [G2 PS2]: matches!(refused, FloorRefusal::NeedOverflows { projected: p, .. } if p == projected),
F crates/pull/src/emit_sites.rs:1554 [G2 PS2]: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
F crates/pull/src/emit_sites.rs:1599 [G2 PS2]: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
F crates/pull/src/emit_sites.rs:1628 [G2 PS2]: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
F crates/pull/src/emit_sites.rs:1657 [G2 PS2]: crate::vendor::Transport::LocalArchive(_) => unreachable!("this feed is HTTP"),
F crates/pull/src/emit_sites.rs:1725 [G2 PS2]: matches!(
F crates/pull/src/emit_sites.rs:2026 [G2 PS2]: panic!("this feed authenticates, so its transport is HTTP")
F crates/pull/src/emit_sites.rs:2124 [G2 PS2]: landed
F crates/pull/src/emit_sites.rs:2125 [G2 PS2]: .iter()
F crates/pull/src/emit_sites.rs:2126 [G2 PS2]: .map(|record| (&record.message, record.field(key)))
F crates/pull/src/emit_sites.rs:2127 [G2 PS2]: .collect::<Vec<_>>()
F crates/pull/src/emit_sites.rs:2176 [G2 PS2]: said.iter()
F crates/pull/src/emit_sites.rs:2177 [G2 PS2]: .map(|record| &record.message)
F crates/pull/src/emit_sites.rs:2178 [G2 PS2]: .collect::<Vec<_>>()
U crates/pull/src/expiry_alias.rs:919 [G1 PS1]: && matches!(
F crates/pull/src/expiry_alias.rs:1101 [G1 PS1]: matches!(
F crates/pull/src/expiry_alias.rs:1108:21 False [G1 PS1]: assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
F crates/pull/src/expiry_alias.rs:1108:33 False [G1 PS1]: assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
F crates/pull/src/expiry_alias.rs:1109:21 False [G1 PS1]: assert!(first <= settled && first < stated, "{row:?}: first day");
F crates/pull/src/expiry_alias.rs:1109:41 False [G1 PS1]: assert!(first <= settled && first < stated, "{row:?}: first day");
F crates/pull/src/expiry_alias.rs:1182:55 True [G1 PS1]: !(a.underlying == b.underlying && a.stated == b.stated),
F crates/pull/src/expiry_alias.rs:1249:13 False [G1 PS1]: let Settlement::Aliased(row) = chain.settlement else {
F crates/pull/src/expiry_alias.rs:1250 [G1 PS1]: panic!("aliased")
F crates/pull/src/expiry_alias.rs:1273:13 False [G1 PS1]: let Settlement::Aliased(row) = got.settlement else {
F crates/pull/src/expiry_alias.rs:1274 [G1 PS1]: panic!("aliased")
F crates/pull/src/expiry_alias.rs:1429:17 False [G1 PS1]: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
F crates/pull/src/expiry_alias.rs:1429:38 False [G1 PS1]: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
F crates/pull/src/expiry_alias.rs:1429:57 False [G1 PS1]: assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
F crates/pull/src/expiry_alias.rs:1606:17 False [G1 PS1]: let Settlement::Aliased(row) = old.settlement else {
F crates/pull/src/expiry_alias.rs:1607 [G1 PS1]: panic!("{under} {stated}: {:?}", old.settlement);
F crates/pull/src/expiry_alias.rs:1658:21 False [G1 PS1]: let Settlement::ExpiredEarly(row) = old.settlement else {
F crates/pull/src/expiry_alias.rs:1659 [G1 PS1]: panic!("{:?}", old.settlement);
F crates/pull/src/expiry_alias.rs:1697:21 False [G1 PS1]: assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
F crates/pull/src/expiry_alias.rs:1697:49 False [G1 PS1]: assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
U crates/pull/src/fetch.rs:515:17 False [G1 PS1]: let (Some(&open), Some(&high), Some(&low), Some(&close)) =
U crates/pull/src/fetch.rs:518 [G1 PS1]: break;
U crates/pull/src/fetch.rs:520:17 False [G1 PS1]: let (Some(&volume), Some(&timestamp)) = (a.volume.get(i), a.timestamp.get(i)) else {
U crates/pull/src/fetch.rs:521 [G1 PS1]: break;
U crates/pull/src/fetch.rs:870 [G1 PS1]: fn note_refused(days: &mut Vec<RefusedDay>, day: u32) {
U crates/pull/src/fetch.rs:871 [G1 PS1]: match days.iter_mut().find(|held| held.day == day) {
U crates/pull/src/fetch.rs:872 [G1 PS1]: Some(held) => held.rows = held.rows.saturating_add(1),
U crates/pull/src/fetch.rs:873 [G1 PS1]: None => days.push(RefusedDay { day, rows: 1 }),
U crates/pull/src/fetch.rs:875 [G1 PS1]: }
U crates/pull/src/fetch.rs:928 [G1 PS1]: FetchError::BodyNotUnderstood {
U crates/pull/src/fetch.rs:929 [G1 PS1]: detail: why.to_string(),
U crates/pull/src/fetch.rs:930 [G1 PS1]: }
U crates/pull/src/fetch.rs:931 [G1 PS1]: })?;
U crates/pull/src/fetch.rs:1032 [G1 PS1]: Err(crate::session::SessionError::SessionLengthUnmeasured { days }) => {
U crates/pull/src/fetch.rs:1033 [G1 PS1]: note_refused(&mut refused_days, days);
U crates/pull/src/fetch.rs:1034 [G1 PS1]: continue;
F crates/pull/src/fetch_ps1_tests.rs:113:13 False [G1 PS1]: let FetchError::LengthDisagreement {
F crates/pull/src/fetch_ps1_tests.rs:123 [G1 PS1]: panic!("a short array is a length disagreement");
F crates/pull/src/fno.rs:405:13 False [G2 PS2]: let Transport::Http(spec) = Feed::Groww.descriptor().transport else {
F crates/pull/src/fno.rs:406 [G2 PS2]: panic!("Groww is an HTTP broker");
F crates/pull/src/fno.rs:532:13 False [G2 PS2]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/src/fno.rs:533 [G2 PS2]: panic!("Dhan is an HTTP broker");
F crates/pull/src/fno.rs:957:17 False [G2 PS2]: assert!(found.contract.is_option() && !found.contract.is_future());
F crates/pull/src/fno.rs:957:47 False [G2 PS2]: assert!(found.contract.is_option() && !found.contract.is_future());
F crates/pull/src/fno.rs:989:13 False [G2 PS2]: floor.contains("2019") && floor.contains("2020"),
F crates/pull/src/fno.rs:989:39 False [G2 PS2]: floor.contains("2019") && floor.contains("2020"),
U crates/pull/src/fno.rs:1331:9 False [G2 PS2]: let Some(rest) = rest else { return false };
U crates/pull/src/fno.rs:1338:9 False [G2 PS2]: let Ok(month) = u8::try_from(month + 1) else {
U crates/pull/src/fno.rs:1339 [G2 PS2]: return false;
U crates/pull/src/fno.rs:1344:9 False [G2 PS2]: let Some(year) = 2000u16.checked_add(year) else {
U crates/pull/src/fno.rs:1345 [G2 PS2]: return false;
U crates/pull/src/fno_membership.rs:1315 [G1 PS1]: )?;
U crates/pull/src/fno_membership.rs:2166:20 False [G1 PS1]: if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
U crates/pull/src/fno_membership.rs:2193:24 False [G1 PS1]: if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
U crates/pull/src/fno_membership.rs:2326:20 False [G1 PS1]: if let Some(slot) = usize::try_from(spelling.series_id)
U crates/pull/src/fno_membership.rs:2384:28 False [G1 PS1]: if let Some(word) = out.bits.get_mut(day_bits + series_at / 64) {
U crates/pull/src/fno_membership.rs:2634:13 False [G1 PS1]: let (Ok(first), Ok(last)) = (
U crates/pull/src/fno_membership.rs:2638 [G1 PS1]: return Vec::new();
U crates/pull/src/fno_membership.rs:2641:17 False [G1 PS1]: let Ok(day) = Day::from_days(epoch) else {
U crates/pull/src/fno_membership.rs:2642 [G1 PS1]: continue;
U crates/pull/src/fno_membership.rs:2679:17 False [G1 PS1]: let Ok(day) = Day::from_days(self.first_epoch.saturating_add(offset)) else {
U crates/pull/src/fno_membership.rs:2680 [G1 PS1]: continue;
U crates/pull/src/fno_membership.rs:2694:25 False [G1 PS1]: let Some(share) = self.series(id).map(|spelling| spelling.name) else {
U crates/pull/src/fno_membership.rs:2695 [G1 PS1]: continue;
U crates/pull/src/fno_membership.rs:2761:16 False [G1 PS1]: if let Some((offset, slot)) = previous
U crates/pull/src/fno_membership.rs:2914:28 False [G1 PS1]: && let Some(table) = open.take()
U crates/pull/src/fno_membership.rs:2918:24 False [G1 PS1]: if work.touched {
U crates/pull/src/fno_membership.rs:2920 [G1 PS1]: }
F crates/pull/src/fno_membership_cov_tests.rs:108:9 False [G1 PS1]: let MembershipError::Corrupt { why, .. } = &refused else {
F crates/pull/src/fno_membership_cov_tests.rs:109 [G1 PS1]: panic!("{refused:?}");
F crates/pull/src/fno_membership_ps1_tests.rs:166 [G1 PS1]: fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
F crates/pull/src/fno_membership_ps1_tests.rs:167 [G1 PS1]: unreachable!("membership is read from listed names: no tick file is fetched")
F crates/pull/src/fno_membership_ps1_tests.rs:217:9 False [G1 PS1]: let MembershipError::Corrupt { path, why } = &refusal else {
F crates/pull/src/fno_membership_ps1_tests.rs:218 [G1 PS1]: panic!("{refusal:?}");
F crates/pull/src/fno_membership_ps1_tests.rs:233:9 False [G1 PS1]: let MembershipError::Stale { path, why } = &refusal else {
F crates/pull/src/fno_membership_ps1_tests.rs:234 [G1 PS1]: panic!("{refusal:?}");
F crates/pull/src/fno_membership_ps1_tests.rs:1034:9 False [G1 PS1]: let Err(why) = decode_ticker("ZQNOTATICKER", monday) else {
F crates/pull/src/fno_membership_ps1_tests.rs:1035 [G1 PS1]: panic!("an invented non-ticker decoded");
F crates/pull/src/fno_membership_ps1_tests.rs:1059:9 False [G1 PS1]: let [handover] = record.handovers[..] else {
F crates/pull/src/fno_membership_ps1_tests.rs:1060 [G1 PS1]: panic!("{:?}", record.handovers);
F crates/pull/src/fno_membership_ps1_tests.rs:1112:9 False [G1 PS1]: let MembershipError::Io { path, why } =
F crates/pull/src/fno_membership_ps1_tests.rs:1115 [G1 PS1]: panic!("both faults are one Io");
F crates/pull/src/fno_membership_ps1_tests.rs:1131:62 False [G1 PS1]: matches!(&alone, MembershipError::Io { path, .. } if *path == nowhere),
F crates/pull/src/fno_membership_tests.rs:78 [G1 PS1]: fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
F crates/pull/src/fno_membership_tests.rs:79 [G1 PS1]: unreachable!("membership is read from listed names: no tick file is fetched")
F crates/pull/src/fno_membership_tests.rs:264:9 False [G1 PS1]: let [handover] = held.handovers() else {
F crates/pull/src/fno_membership_tests.rs:265 [G1 PS1]: panic!("one handover: {:?}", held.handovers());
F crates/pull/src/fno_membership_tests.rs:728:16 False [G1 PS1]: if *path == second && why.contains("BLAKE3")),
F crates/pull/src/fno_membership_tests.rs:728:35 False [G1 PS1]: if *path == second && why.contains("BLAKE3")),
F crates/pull/src/fno_membership_tests.rs:744:55 False [G1 PS1]: Err(MembershipError::Corrupt { path, .. }) if path.ends_with("notes.txt")
F crates/pull/src/fno_membership_tests.rs:843:66 False [G1 PS1]: matches!(&refusal, MembershipError::Stale { why, .. } if why.contains("carry")),
F crates/pull/src/fno_membership_tests.rs:1203:9 False [G1 PS1]: let [illiq, one] = reported.as_slice() else {
F crates/pull/src/fno_membership_tests.rs:1204 [G1 PS1]: panic!("{reported:?}")
F crates/pull/src/fno_membership_tests.rs:1207:9 False [G1 PS1]: illiq.starts_with("ZZILLIQ -> ZZNEWBIE")
F crates/pull/src/fno_membership_tests.rs:1208:16 False [G1 PS1]: && illiq.ends_with("a minority of the new spelling's keys"),
F crates/pull/src/fno_membership_tests.rs:1212:9 False [G1 PS1]: one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
F crates/pull/src/fno_membership_tests.rs:1212:49 False [G1 PS1]: one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
F crates/pull/src/fno_membership_tests.rs:1253:9 False [G1 PS1]: strays[0].1.starts_with("ZZOLDCO listed 1 contract key(s)")
F crates/pull/src/fno_membership_tests.rs:1254:16 False [G1 PS1]: && strays[0].1.contains("filed under ZZNEWCO"),
F crates/pull/src/fno_membership_tests.rs:1286:16 False [G1 PS1]: if *day == d(2024, 3, 6) && !why.is_empty()),
F crates/pull/src/fno_membership_tests.rs:1286:41 False [G1 PS1]: if *day == d(2024, 3, 6) && !why.is_empty()),
F crates/pull/src/fno_membership_tests.rs:1342:16 False [G1 PS1]: if *path == missing && why.contains("no such membership directory")),
F crates/pull/src/fno_membership_tests.rs:1342:36 False [G1 PS1]: if *path == missing && why.contains("no such membership directory")),
F crates/pull/src/fno_membership_tests.rs:1467:26 False [G1 PS1]: assert!(a.is_ok() || b.is_ok(), "round {round}: {a:?} | {b:?}");
F crates/pull/src/fno_membership_tests.rs:1473 [G1 PS1]: matches!(
F crates/pull/src/fno_ps2_tests.rs:17:9 False [G2 PS2]: let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
F crates/pull/src/fno_ps2_tests.rs:18 [G2 PS2]: panic!("Zerodha is an HTTP broker");
F crates/pull/src/fnowork_cov_tests.rs:196 [G3 PS3]: fn never(url: &str) -> Result<String, Refusal> {
F crates/pull/src/fnowork_cov_tests.rs:197 [G3 PS3]: panic!("nothing should be asked, and {url} was");
F crates/pull/src/fnowork_futures_tests.rs:54:9 False [G2 PS2]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
F crates/pull/src/fnowork_futures_tests.rs:56 [G2 PS2]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/fnowork_futures_tests.rs:166 [G2 PS2]: _ => unreachable!("february_reply asked for {url}"),
F crates/pull/src/fnowork_futures_tests.rs:290:9 False [G2 PS2]: let crate::vendor::Transport::Http(groww) = crate::vendor::Feed::Groww.descriptor().transport
F crates/pull/src/fnowork_futures_tests.rs:292 [G2 PS2]: panic!("Groww is an HTTP feed");
F crates/pull/src/fnowork_futures_tests.rs:657 [G2 PS2]: landed.balance_line()
F crates/pull/src/fnowork_futures_tests.rs:665:9 False [G2 PS2]: stopped.contains("0 bars written, 0 offered from 0 rows")
F crates/pull/src/fnowork_futures_tests.rs:666:16 False [G2 PS2]: && stopped.contains("; then stopped: vendor_refused"),
F crates/pull/src/fnowork_futures_tests.rs:844:9 False [G2 PS2]: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
F crates/pull/src/fnowork_futures_tests.rs:844:49 False [G2 PS2]: plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
F crates/pull/src/fnowork_futures_tests.rs:885:27 False [G2 PS2]: .filter(|why| why.starts_with("contract_duplicated")
IGNORED crates/pull/src/fnowork_futures_tests.rs:990 [G2 PS2]: fn a_real_size_plan_does_not_grow_with_the_snapshots_held() {
IGNORED crates/pull/src/fnowork_futures_tests.rs:991 [G2 PS2]: let path = std::env::var_os("BRUTEX_KITE_DUMP").expect("BRUTEX_KITE_DUMP");
IGNORED crates/pull/src/fnowork_futures_tests.rs:992 [G2 PS2]: let body = std::fs::read(path).expect("the dump reads");
IGNORED crates/pull/src/fnowork_futures_tests.rs:993 [G2 PS2]: let root = scratch("real-size");
IGNORED crates/pull/src/fnowork_futures_tests.rs:996 [G2 PS2]: let first = day(2026, 7, 1);
IGNORED crates/pull/src/fnowork_futures_tests.rs:997 [G2 PS2]: let asked = window(day(2026, 10, 1), day(2026, 12, 31));
IGNORED crates/pull/src/fnowork_futures_tests.rs:998 [G2 PS2]: let mut held = 0_u32;
IGNORED crates/pull/src/fnowork_futures_tests.rs:999 [G2 PS2]: let mut p50s = Vec::new();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1000 [G2 PS2]: for count in [1_u32, 10, 100] {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001 [G2 PS2]: while held < count {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001:15 False [G2 PS2]: while held < count {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1001:15 True [G2 PS2]: while held < count {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1002 [G2 PS2]: let on = Day::from_days(first.days_from_epoch() + held).expect("a real day");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1003 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1004 [G2 PS2]: master_snapshots::snapshot(&root, Master::Kite, on, URL, &body).expect("a snapshot");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1005 [G2 PS2]: println!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1006 [G2 PS2]: "MEASURED snapshot {} written, its futures taken in: {} ms",
IGNORED crates/pull/src/fnowork_futures_tests.rs:1007 [G2 PS2]: held + 1,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1008 [G2 PS2]: started.elapsed().as_millis()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1009 [G2 PS2]: );
IGNORED crates/pull/src/fnowork_futures_tests.rs:1010 [G2 PS2]: held += 1;
IGNORED crates/pull/src/fnowork_futures_tests.rs:1011 [G2 PS2]: }
IGNORED crates/pull/src/fnowork_futures_tests.rs:1012 [G2 PS2]: let mut ns: Vec<u128> = (0..50)
IGNORED crates/pull/src/fnowork_futures_tests.rs:1013 [G2 PS2]: .map(|_| {
IGNORED crates/pull/src/fnowork_futures_tests.rs:1014 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1015 [G2 PS2]: let book = Book::open(&root).expect("a book");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1016 [G2 PS2]: let plan = futures_plan(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1017 [G2 PS2]: &book,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1018 [G2 PS2]: route(),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1019 [G2 PS2]: "NIFTY",
IGNORED crates/pull/src/fnowork_futures_tests.rs:1020 [G2 PS2]: asked,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1021 [G2 PS2]: day(2026, 10, 22),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1022 [G2 PS2]: day(2026, 12, 31),
IGNORED crates/pull/src/fnowork_futures_tests.rs:1024 [G2 PS2]: .expect("a plan");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1025 [G2 PS2]: assert!(plan.refused.is_empty(), "{:?}", plan.refused);
IGNORED crates/pull/src/fnowork_futures_tests.rs:1026 [G2 PS2]: assert!(!plan.asks.is_empty(), "NIFTY futures are asked");
IGNORED crates/pull/src/fnowork_futures_tests.rs:1027 [G2 PS2]: started.elapsed().as_nanos()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1028 [G2 PS2]: })
IGNORED crates/pull/src/fnowork_futures_tests.rs:1029 [G2 PS2]: .collect();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1030 [G2 PS2]: ns.sort_unstable();
IGNORED crates/pull/src/fnowork_futures_tests.rs:1031 [G2 PS2]: println!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1033 [G2 PS2]: ns[24] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1034 [G2 PS2]: ns[48] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1035 [G2 PS2]: ns[49] / 1_000,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1036 [G2 PS2]: ns.len()
IGNORED crates/pull/src/fnowork_futures_tests.rs:1038 [G2 PS2]: p50s.push(ns[24]);
IGNORED crates/pull/src/fnowork_futures_tests.rs:1040 [G2 PS2]: assert!(
IGNORED crates/pull/src/fnowork_futures_tests.rs:1041 [G2 PS2]: p50s[2] <= p50s[0] * 3,
IGNORED crates/pull/src/fnowork_futures_tests.rs:1043 [G2 PS2]: p50s[2],
IGNORED crates/pull/src/fnowork_futures_tests.rs:1044 [G2 PS2]: p50s[0]
IGNORED crates/pull/src/fnowork_futures_tests.rs:1046 [G2 PS2]: }
F crates/pull/src/fnowork_futures_tests.rs:1136:9 False [G2 PS2]: said.starts_with("generation g1 of the futures index did not match its commit log")
F crates/pull/src/fnowork_futures_tests.rs:1137:16 False [G2 PS2]: && said.contains(why)
F crates/pull/src/fnowork_futures_tests.rs:1138:16 False [G2 PS2]: && said.contains("generation g2 was started"),
F crates/pull/src/fnowork_futures_tests.rs:1456 [G2 PS2]: alike.series[0]
F crates/pull/src/fnowork_futures_tests.rs:1470 [G2 PS2]: unlike.balance_line()
F crates/pull/src/fnowork_futures_tests.rs:1473:9 False [G2 PS2]: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
F crates/pull/src/fnowork_futures_tests.rs:1473:58 False [G2 PS2]: unlike.series[0].contains("; then stopped: ") && !unlike.series[0].contains("already held"),
F crates/pull/src/fnowork_futures_tests.rs:1475 [G2 PS2]: unlike.series[0]
F crates/pull/src/fnowork_futures_tests.rs:1502:9 False [G2 PS2]: why.starts_with("snapshot_from_the_future: ")
F crates/pull/src/fnowork_futures_tests.rs:1503:16 False [G2 PS2]: && why.contains("2030-01-05")
F crates/pull/src/fnowork_futures_tests.rs:1504:16 False [G2 PS2]: && why.contains("after today 2026-01-07"),
U crates/pull/src/fold.rs:272:38 False [G2 PS2]: (Some(first), Some(last)) if width > 0 => {
U crates/pull/src/fold.rs:343:17 False [G2 PS2]: let Some(bar) = out.last_mut() else {
U crates/pull/src/fold.rs:345 [G2 PS2]: return Err(FoldError::OutOfOrder {
U crates/pull/src/fold.rs:346 [G2 PS2]: at: i,
U crates/pull/src/fold.rs:347 [G2 PS2]: previous: start,
U crates/pull/src/fold.rs:348 [G2 PS2]: found: snap.ts_micros,
U crates/pull/src/fold.rs:349 [G2 PS2]: });
U crates/pull/src/fold.rs:1329:20 False [G2 PS2]: if let Some(note) = day_note_at.and_then(|at| out.notes.get_mut(at)) {
U crates/pull/src/fold.rs:1336:33 False [G2 PS2]: let complete = valid && expected > 0 && i128::from(count) == expected && traded == count;
U crates/pull/src/fold.rs:1340:16 False [G2 PS2]: && expected > 0
U crates/pull/src/fold.rs:1343:52 False [G2 PS2]: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
U crates/pull/src/fold.rs:1343:68 False [G2 PS2]: let only_padded = judge_padded && valid && expected > 0 && count > 0 && traded == 0;
U crates/pull/src/fold.rs:1664:8 True [G2 PS2]: if hours.kind() != crate::vendor::SessionKind::Continuous
U crates/pull/src/fold.rs:1665:12 True [G2 PS2]: || hours.open_minute() != u32::from(crate::calendar::OPEN_MINUTE)
U crates/pull/src/fold.rs:1667 [G2 PS2]: return Err(format!(
U crates/pull/src/fold.rs:1668 [G2 PS2]: "{venue}: session hours cannot attest the fixed stored grid"
U crates/pull/src/fold.rs:1669 [G2 PS2]: ));
F crates/pull/src/fold.rs:1925 [G2 PS2]: diagnostics.len(),
F crates/pull/src/fold.rs:1926 [G2 PS2]: diagnostics.iter().take(7).collect::<Vec<_>>()
F crates/pull/src/fold.rs:2270 [G2 PS2]: derived.diagnostics[0]
F crates/pull/src/fold.rs:2322 [G2 PS2]: lines[0]
U crates/pull/src/folder.rs:521 [G1 PS1]: Err(why) => {
U crates/pull/src/folder.rs:522 [G1 PS1]: note_refused(dir, feed, &why);
U crates/pull/src/folder.rs:523 [G1 PS1]: Err(why)
U crates/pull/src/folder.rs:553 [G1 PS1]: Err(why) => {
U crates/pull/src/folder.rs:554 [G1 PS1]: note_refused(dir, feed, &why);
U crates/pull/src/folder.rs:555 [G1 PS1]: Err(why)
F crates/pull/src/folder.rs:638:13 False [G1 PS1]: !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
F crates/pull/src/folder.rs:638:47 False [G1 PS1]: !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
F crates/pull/src/folder.rs:669:13 False [G1 PS1]: said.contains(ROOT_ENV) && said.contains("HOME"),
F crates/pull/src/folder.rs:669:40 False [G1 PS1]: said.contains(ROOT_ENV) && said.contains("HOME"),
F crates/pull/src/gap_backfill_cov_tests.rs:604 [G3 PS3]: found.lines[0]
F crates/pull/src/gap_backfill_cov_tests.rs:625 [G3 PS3]: traded.lines[0]
F crates/pull/src/gaps_cov_tests.rs:175:13 False [G3 PS3]: assert!(before != after && after != unread && before != unread);
F crates/pull/src/gaps_cov_tests.rs:175:32 False [G3 PS3]: assert!(before != after && after != unread && before != unread);
F crates/pull/src/gaps_cov_tests.rs:175:51 False [G3 PS3]: assert!(before != after && after != unread && before != unread);
U crates/pull/src/gdfl_archive.rs:653:20 False [G3 PS3]: if let Some(cell) = days
IGNORED crates/pull/src/gdfl_archive.rs:2544 [G3 PS3]: fn the_real_containers_open_with_no_prefix_and_print_their_facts() {
IGNORED crates/pull/src/gdfl_archive.rs:2545 [G3 PS3]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_archive.rs:2545:16 False [G3 PS3]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_archive.rs:2545:16 True [G3 PS3]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_archive.rs:2546 [G3 PS3]: let archive = Archive::open(Path::new(&path)).unwrap();
IGNORED crates/pull/src/gdfl_archive.rs:2547 [G3 PS3]: for (tree, name) in [(0, "indices"), (1, "stocks")] {
IGNORED crates/pull/src/gdfl_archive.rs:2548 [G3 PS3]: let held: Vec<usize> = archive.days[tree]
IGNORED crates/pull/src/gdfl_archive.rs:2549 [G3 PS3]: .iter()
IGNORED crates/pull/src/gdfl_archive.rs:2550 [G3 PS3]: .enumerate()
IGNORED crates/pull/src/gdfl_archive.rs:2551 [G3 PS3]: .filter_map(|(slot, cell)| matches!(cell, Some(Slot::Held(_))).then_some(slot))
IGNORED crates/pull/src/gdfl_archive.rs:2552 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_archive.rs:2553 [G3 PS3]: let day = |slot: usize| {
IGNORED crates/pull/src/gdfl_archive.rs:2554 [G3 PS3]: u32::try_from(slot)
IGNORED crates/pull/src/gdfl_archive.rs:2555 [G3 PS3]: .ok()
IGNORED crates/pull/src/gdfl_archive.rs:2556 [G3 PS3]: .and_then(|slot| Day::from_days(archive.first + slot).ok())
IGNORED crates/pull/src/gdfl_archive.rs:2557 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_archive.rs:2558 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_archive.rs:2560 [G3 PS3]: held.len(),
IGNORED crates/pull/src/gdfl_archive.rs:2561 [G3 PS3]: held.first().map(|slot| day(*slot)),
IGNORED crates/pull/src/gdfl_archive.rs:2562 [G3 PS3]: held.last().map(|slot| day(*slot)),
IGNORED crates/pull/src/gdfl_archive.rs:2565 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_archive.rs:2567 [G3 PS3]: archive
IGNORED crates/pull/src/gdfl_archive.rs:2568 [G3 PS3]: .days
IGNORED crates/pull/src/gdfl_archive.rs:2569 [G3 PS3]: .iter()
IGNORED crates/pull/src/gdfl_archive.rs:2570 [G3 PS3]: .flatten()
IGNORED crates/pull/src/gdfl_archive.rs:2571 [G3 PS3]: .filter(|cell| matches!(cell, Some(Slot::Refused(_))))
IGNORED crates/pull/src/gdfl_archive.rs:2572 [G3 PS3]: .count(),
IGNORED crates/pull/src/gdfl_archive.rs:2573 [G3 PS3]: archive.census()
IGNORED crates/pull/src/gdfl_archive.rs:2575 [G3 PS3]: let span = Span::of(&archive.source, archive.len);
IGNORED crates/pull/src/gdfl_archive.rs:2576 [G3 PS3]: let entries = central_directory(&span).unwrap();
IGNORED crates/pull/src/gdfl_archive.rs:2577 [G3 PS3]: let count = |test: &dyn Fn(&Entry) -> bool| entries.iter().filter(|e| test(e)).count();
IGNORED crates/pull/src/gdfl_archive.rs:2578 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_archive.rs:2580 [G3 PS3]: entries.len(),
IGNORED crates/pull/src/gdfl_archive.rs:2581 [G3 PS3]: count(&|e| e.flags & 8 != 0),
IGNORED crates/pull/src/gdfl_archive.rs:2582 [G3 PS3]: count(&|e| e.flags & ENCRYPTED != 0),
IGNORED crates/pull/src/gdfl_archive.rs:2583 [G3 PS3]: count(&|e| e.method != 0),
IGNORED crates/pull/src/gdfl_archive.rs:2585 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_archive.rs:2586 [G3 PS3]: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
IGNORED crates/pull/src/gdfl_archive.rs:2586:16 False [G3 PS3]: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
IGNORED crates/pull/src/gdfl_archive.rs:2586:16 True [G3 PS3]: if let Some(dir) = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES") {
IGNORED crates/pull/src/gdfl_archive.rs:2587 [G3 PS3]: let mut years: Vec<_> = std::fs::read_dir(&dir)
IGNORED crates/pull/src/gdfl_archive.rs:2588 [G3 PS3]: .unwrap()
IGNORED crates/pull/src/gdfl_archive.rs:2589 [G3 PS3]: .map(|entry| entry.unwrap().path())
IGNORED crates/pull/src/gdfl_archive.rs:2590 [G3 PS3]: .filter(|path| path.extension().is_some_and(|ext| ext == "zip"))
IGNORED crates/pull/src/gdfl_archive.rs:2591 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_archive.rs:2592 [G3 PS3]: years.sort();
IGNORED crates/pull/src/gdfl_archive.rs:2593 [G3 PS3]: for year in years {
IGNORED crates/pull/src/gdfl_archive.rs:2594 [G3 PS3]: let file = std::fs::File::open(&year).unwrap();
IGNORED crates/pull/src/gdfl_archive.rs:2595 [G3 PS3]: let len = file.metadata().unwrap().len();
IGNORED crates/pull/src/gdfl_archive.rs:2596 [G3 PS3]: let entries = zip_entries(&file, 0, len).unwrap();
IGNORED crates/pull/src/gdfl_archive.rs:2597 [G3 PS3]: let zips: Vec<&ZipEntry> = entries
IGNORED crates/pull/src/gdfl_archive.rs:2598 [G3 PS3]: .iter()
IGNORED crates/pull/src/gdfl_archive.rs:2599 [G3 PS3]: .filter(|e| {
IGNORED crates/pull/src/gdfl_archive.rs:2600 [G3 PS3]: Path::new(&e.name)
IGNORED crates/pull/src/gdfl_archive.rs:2601 [G3 PS3]: .extension()
IGNORED crates/pull/src/gdfl_archive.rs:2602 [G3 PS3]: .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
IGNORED crates/pull/src/gdfl_archive.rs:2603 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_archive.rs:2604 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_archive.rs:2605 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_archive.rs:2608 [G3 PS3]: year.display(),
IGNORED crates/pull/src/gdfl_archive.rs:2609 [G3 PS3]: entries.len(),
IGNORED crates/pull/src/gdfl_archive.rs:2610 [G3 PS3]: zips.len(),
IGNORED crates/pull/src/gdfl_archive.rs:2611 [G3 PS3]: zips.first().map(|e| &e.name),
IGNORED crates/pull/src/gdfl_archive.rs:2612 [G3 PS3]: zips.last().map(|e| &e.name),
IGNORED crates/pull/src/gdfl_archive.rs:2613 [G3 PS3]: zips.iter().filter(|e| e.locator.flags & 8 != 0).count(),
IGNORED crates/pull/src/gdfl_archive.rs:2614 [G3 PS3]: zips.iter()
IGNORED crates/pull/src/gdfl_archive.rs:2615 [G3 PS3]: .filter(|e| e.locator.flags & ENCRYPTED != 0)
IGNORED crates/pull/src/gdfl_archive.rs:2616 [G3 PS3]: .count(),
IGNORED crates/pull/src/gdfl_archive.rs:2617 [G3 PS3]: zips.iter().filter(|e| e.locator.method != 0).count(),
IGNORED crates/pull/src/gdfl_archive.rs:2620 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_archive.rs:2621 [G3 PS3]: }
F crates/pull/src/gdfl_backfill_tests.rs:321 [G2 PS2]: lines[intent]
F crates/pull/src/gdfl_backfill_tests.rs:339 [G2 PS2]: lines[repair]
F crates/pull/src/gdfl_backfill_tests.rs:401:35 False [G2 PS2]: intents.iter().any(|line| line
F crates/pull/src/gdfl_backfill_tests.rs:522:13 False [G2 PS2]: failure.why.contains("the versioned repair refused it too")
F crates/pull/src/gdfl_backfill_tests.rs:523:20 False [G2 PS2]: && failure.why.contains("bar-revisions-v1"),
F crates/pull/src/gdfl_backfill_tests.rs:643:9 False [G2 PS2]: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_backfill_tests.rs:643:57 False [G2 PS2]: closed.contains("indices 2024-04-01 NIFTY ") && closed.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_backfill_tests.rs:801 [G2 PS2]: nifty[0].why
F crates/pull/src/gdfl_backfill_tests.rs:1268 [G2 PS2]: refused.failures[0]
F crates/pull/src/gdfl_backfill_tests.rs:1281 [G2 PS2]: refused.failures[0]
U crates/pull/src/gdfl_cm.rs:1386:41 False [G2 PS2]: Some((share, _)) if self.series => share,
U crates/pull/src/gdfl_cm.rs:1803:13 False [G2 PS2]: let Some(cell) = cell else {
U crates/pull/src/gdfl_cm.rs:1804 [G2 PS2]: self.skip();
U crates/pull/src/gdfl_cm.rs:1805 [G2 PS2]: return;
U crates/pull/src/gdfl_cm.rs:2040 [G2 PS2]: Ok(CasEligibility::Unknown) => Auction::Unknown(format!(
U crates/pull/src/gdfl_cm.rs:2041 [G2 PS2]: "UNVERIFIED cash eligibility of {symbol}: the master holds no flag"
U crates/pull/src/gdfl_cm.rs:2042 [G2 PS2]: )),
U crates/pull/src/gdfl_cm.rs:2884:58 False [G2 PS2]: if matches!(self.band, MatchBand::One(_)) && self.booked > 0 {
U crates/pull/src/gdfl_cm.rs:2930:20 False [G2 PS2]: if let Some(row) = next {
U crates/pull/src/gdfl_cm.rs:3509:9 False [G2 PS2]: let Ok(window) = crate::session::Window::new(day, day) else {
U crates/pull/src/gdfl_cm.rs:3510 [G2 PS2]: return false;
F crates/pull/src/gdfl_cm_admission_tests.rs:704 [G3 PS3]: other => panic!("{stem} {d}: {other:?}"),
F crates/pull/src/gdfl_cm_admission_tests.rs:788:13 False [G3 PS3]: assert!(scope.is_stored() && !CmScope::swept().is_stored());
F crates/pull/src/gdfl_cm_admission_tests.rs:788:34 False [G3 PS3]: assert!(scope.is_stored() && !CmScope::swept().is_stored());
F crates/pull/src/gdfl_cm_admission_tests.rs:952:32 False [G3 PS3]: assert!(vix.iter().all(|b| b.close == 6_141 || b.close == 6_142));
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1583 [G3 PS3]: fn the_match_band_costs_one_pass_a_file() {
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1584 [G3 PS3]: let mut rows: Vec<String> = (0..30_000_u32)
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1585 [G3 PS3]: .map(|i| {
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1586 [G3 PS3]: quoted(
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1587 [G3 PS3]: BAND_STEM,
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1588 [G3 PS3]: band_day(),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1589 [G3 PS3]: at(9, 15, 0) + i * 21_599 / 30_000,
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1590 [G3 PS3]: "4444.15",
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1591 [G3 PS3]: ("4444.10", "4444.20"),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1594 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1595 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1596 [G3 PS3]: for (sod, quotes) in [
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1597 [G3 PS3]: (at(15, 28, 3), BAND_BOOK),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1598 [G3 PS3]: (at(15, 28, 7), BAND_ZERO),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1599 [G3 PS3]: (at(15, 30, 1), BAND_BOOK),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1600 [G3 PS3]: ] {
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1601 [G3 PS3]: rows.push(quoted(BAND_STEM, band_day(), sod, "4446.65", quotes, 383));
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1602 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1603 [G3 PS3]: rows.push(quoted(
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1604 [G3 PS3]: BAND_STEM,
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1605 [G3 PS3]: band_day(),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1606 [G3 PS3]: at(15, 52, 0),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1607 [G3 PS3]: "4446.65",
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1611 [G3 PS3]: let bytes = csv(&rows);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1612 [G3 PS3]: let mut eligible = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1613 [G3 PS3]: let mut ineligible = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1614 [G3 PS3]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1615 [G3 PS3]: let start = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1616 [G3 PS3]: let file = band_decode(&bytes, Auction::Eligible).unwrap();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1617 [G3 PS3]: eligible.push(start.elapsed());
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1618 [G3 PS3]: assert_eq!(file.auction.len(), 3);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1619 [G3 PS3]: let start = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1620 [G3 PS3]: let file = band_decode(&bytes, Auction::NotEligible).unwrap();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1621 [G3 PS3]: ineligible.push(start.elapsed());
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1622 [G3 PS3]: assert_eq!(file.rows.len(), rows.len());
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1624 [G3 PS3]: let stats = |runs: &mut [std::time::Duration]| {
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1625 [G3 PS3]: runs.sort_unstable();
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1626 [G3 PS3]: (
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1627 [G3 PS3]: runs[runs.len() / 2],
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1628 [G3 PS3]: runs[runs.len() * 99 / 100],
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1629 [G3 PS3]: runs[runs.len() - 1],
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1630 [G3 PS3]: )
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1631 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1632 [G3 PS3]: let (e50, e99, emax) = stats(&mut eligible);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1633 [G3 PS3]: let (n50, n99, nmax) = stats(&mut ineligible);
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1634 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1637 [G3 PS3]: rows.len()
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1639 [G3 PS3]: assert!(
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1640 [G3 PS3]: e50 <= n50 * 2 + std::time::Duration::from_millis(1),
IGNORED crates/pull/src/gdfl_cm_admission_tests.rs:1643 [G3 PS3]: }
F crates/pull/src/gdfl_cm_admission_tests.rs:1663 [G3 PS3]: path.display()
F crates/pull/src/gdfl_cm_admission_tests.rs:1692:51 False [G3 PS3]: Err(CmRefusal::AuctionSidecar { why }) if why.contains("new generation")
F crates/pull/src/gdfl_cm_admission_tests.rs:1697:51 False [G3 PS3]: Err(CmRefusal::AuctionSidecar { why }) if why.contains("never an insertion")
U crates/pull/src/gdfl_cm_auction.rs:642:12 False [G1 PS1]: if let Some(parent) = path.parent() {
U crates/pull/src/gdfl_cm_auction.rs:650 [G1 PS1]: }
F crates/pull/src/gdfl_cm_auction.rs:998 [G1 PS1]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_cm_auction.rs:1158 [G1 PS1]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:42 [G1 PS1]: other => panic!("not a sidecar refusal: {other:?}"),
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:81:9 False [G1 PS1]: let Err(std::fs::TryLockError::Error(shared)) = unlockable(&lock_at).try_lock_shared() else {
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:82 [G1 PS1]: panic!("the host took a shared flock on an O_PATH descriptor");
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:84:9 False [G1 PS1]: let Err(std::fs::TryLockError::Error(exclusive)) = unlockable(&lock_at).try_lock() else {
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:85 [G1 PS1]: panic!("the host took an exclusive flock on an O_PATH descriptor");
F crates/pull/src/gdfl_cm_auction_ps1_tests.rs:174 [G1 PS1]: cut: &|_, _| unreachable!("a fresh sidecar has no torn tail to cut"),
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:80 [G1 PS1]: Ok(sidecar) => panic!(
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:82 [G1 PS1]: sidecar.records.len()
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:91 [G1 PS1]: other => panic!("not a sidecar refusal: {other:?}"),
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:17 False [G1 PS1]: refusal.why.contains("record 1") && refusal.why.contains("checksum"),
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:53 False [G1 PS1]: refusal.why.contains("record 1") && refusal.why.contains("checksum"),
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:260 [G1 PS1]: made.display()
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:25 False [G1 PS1]: Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {}
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:39 False [G1 PS1]: Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {}
F crates/pull/src/gdfl_cm_auction_v2_tests.rs:419 [G1 PS1]: Err(refusal) => panic!("a reader beside an append was refused: {refusal}"),
U crates/pull/src/gdfl_dayzips.rs:380 [G2 PS2]: (None, None) => Ok(None),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:540 [G2 PS2]: fn day_table_sizing_timing() {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:541 [G2 PS2]: fn stats(mut took: Vec<std::time::Duration>) -> String {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:542 [G2 PS2]: took.sort_unstable();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:543 [G2 PS2]: let n = took.len();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:544 [G2 PS2]: format!(
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:546 [G2 PS2]: took[n / 2],
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:547 [G2 PS2]: took[n * 99 / 100],
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:548 [G2 PS2]: took[n - 1]
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:550 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:551 [G2 PS2]: let alloc = |sized: bool| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:552 [G2 PS2]: (0..2_000)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:553 [G2 PS2]: .map(|_| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:554 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555 [G2 PS2]: let table: HashMap<(LooseKind, Day), Copies> = if sized {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555:67 False [G2 PS2]: let table: HashMap<(LooseKind, Day), Copies> = if sized {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:555:67 True [G2 PS2]: let table: HashMap<(LooseKind, Day), Copies> = if sized {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:556 [G2 PS2]: HashMap::with_capacity(DAYZIPS_KEYS_MAX)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:558 [G2 PS2]: HashMap::default()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:560 [G2 PS2]: let took = began.elapsed();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:561 [G2 PS2]: assert!(table.capacity() >= usize::from(sized) * DAYZIPS_KEYS_MAX);
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:562 [G2 PS2]: took
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:563 [G2 PS2]: })
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:564 [G2 PS2]: .collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:565 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:566 [G2 PS2]: let empty = fresh("timing.empty");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:567 [G2 PS2]: let edges = fresh("timing.edges");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:568 [G2 PS2]: for kind in LooseKind::ALL {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:569 [G2 PS2]: for day in [
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:570 [G2 PS2]: Day::new(2018, 1, 1).unwrap(),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:571 [G2 PS2]: Day::new(2026, 12, 31).unwrap(),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:572 [G2 PS2]: ] {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:573 [G2 PS2]: put(&edges, &kind.path_of(day), b"");
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:574 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:576 [G2 PS2]: let walk = |root: &Path| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:577 [G2 PS2]: (0..200)
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:578 [G2 PS2]: .map(|_| {
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:579 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:580 [G2 PS2]: let zips = DayZips::open(root).unwrap();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:581 [G2 PS2]: let took = began.elapsed();
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:582 [G2 PS2]: assert!(zips.days.capacity() >= DAYZIPS_KEYS_MAX);
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:583 [G2 PS2]: took
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:584 [G2 PS2]: })
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:585 [G2 PS2]: .collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:586 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:587 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:589 [G2 PS2]: stats(alloc(false)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:590 [G2 PS2]: stats(alloc(true)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:591 [G2 PS2]: stats(walk(&empty)),
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:592 [G2 PS2]: stats(walk(&edges))
IGNORED crates/pull/src/gdfl_dayzips_tests.rs:594 [G2 PS2]: }
U crates/pull/src/gdfl_dropbox.rs:870:15 True [G3 PS3]: } else if why.is_body() || why.is_decode() {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:70:9 False [G3 PS3]: let Wait::TokenMissing { why } = refused.read().unwrap_err() else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:71 [G3 PS3]: panic!("a refused read is a missing token");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:78:9 False [G3 PS3]: let Wait::TokenMissing { why } = unnamed.read().unwrap_err() else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:79 [G3 PS3]: panic!("no field is a missing token");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:85:9 False [G3 PS3]: let Wait::TokenMissing { why } = unnamed.reread(&read).unwrap_err() else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:86 [G3 PS3]: panic!("no field is a missing token on the re-read too");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:249:9 False [G3 PS3]: let Err(Wait::LinkMalformed { why }) = Config::load(&path) else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:250 [G3 PS3]: panic!("not UTF-8 is malformed");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:345 [G3 PS3]: foreign.display()
F crates/pull/src/gdfl_dropbox_cov_tests.rs:420:13 False [G3 PS3]: assert!(!named.contains('@') && !named.contains("://"), "{real:?}");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:420:37 False [G3 PS3]: assert!(!named.contains('@') && !named.contains("://"), "{real:?}");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:481 [G3 PS3]: Ok(None) => panic!("a cut body ended cleanly"),
F crates/pull/src/gdfl_dropbox_cov_tests.rs:498:13 False [G3 PS3]: assert!(decode.is_decode() && !decode.is_body(), "{decode:?}");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:498:35 False [G3 PS3]: assert!(decode.is_decode() && !decode.is_body(), "{decode:?}");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:616 [G3 PS3]: Err(Stop::Wait(wait)) => panic!("a wait: {wait:?}"),
F crates/pull/src/gdfl_dropbox_cov_tests.rs:617 [G3 PS3]: Ok(added) => panic!("taken: {added}"),
F crates/pull/src/gdfl_dropbox_cov_tests.rs:712:13 False [G3 PS3]: assert!(memory.complete.is_empty() && memory.shrinks.is_empty());
F crates/pull/src/gdfl_dropbox_cov_tests.rs:712:43 False [G3 PS3]: assert!(memory.complete.is_empty() && memory.shrinks.is_empty());
F crates/pull/src/gdfl_dropbox_cov_tests.rs:754:13 False [G3 PS3]: assert!(memory.hashes.is_empty() && memory.writer_refused.is_empty());
F crates/pull/src/gdfl_dropbox_cov_tests.rs:754:41 False [G3 PS3]: assert!(memory.hashes.is_empty() && memory.writer_refused.is_empty());
F crates/pull/src/gdfl_dropbox_cov_tests.rs:872:9 False [G3 PS3]: let Err(Stop::Refused(missing)) = new_part(&gone, &file, &mut journal) else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:873 [G3 PS3]: panic!("a part in a missing folder");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:896:9 False [G3 PS3]: let Err(Stop::Refused(full)) = new_part(&run, &file, &mut journal) else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:897 [G3 PS3]: panic!("a 65th part");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:966:9 False [G3 PS3]: let Err(Stop::Refused(busy)) = lock_staging(&store, &staging, "1-1-2") else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:967 [G3 PS3]: panic!("a second press beside the first");
F crates/pull/src/gdfl_dropbox_cov_tests.rs:980:9 False [G3 PS3]: let Ok(second) = lock_staging(&store, &staging, "1-1-2") else {
F crates/pull/src/gdfl_dropbox_cov_tests.rs:981 [G3 PS3]: panic!("the copy of an ended press's description kept the lock");
F crates/pull/src/gdfl_dropbox_tests.rs:868:9 False [G3 PS3]: journal_text(&roots).contains("v=1 wait ")
F crates/pull/src/gdfl_dropbox_tests.rs:869:16 False [G3 PS3]: && journal_text(&roots).contains("reason=dropbox_link_missing")
F crates/pull/src/gdfl_dropbox_tests.rs:956:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:957 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1004:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1005 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1092:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1093 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1140:13 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1141 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1197:9 False [G3 PS3]: let Outcome::Refused(refusal) = &cut.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1198 [G3 PS3]: panic!("{cut:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1227:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1228 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1251:9 False [G3 PS3]: let Outcome::Refused(refusal) = &shrank.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1252 [G3 PS3]: panic!("{shrank:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1402:9 False [G3 PS3]: let Outcome::Refused(refusal) = &storm.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1403 [G3 PS3]: panic!("{storm:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1430:9 False [G3 PS3]: let Outcome::Refused(refusal) = &down.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1431 [G3 PS3]: panic!("{down:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1602:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1603 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1686:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1687 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1765:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1766 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1784:60 False [G3 PS3]: assert!(matches!(&lost.outcome, Outcome::Refused(r) if r.code == "store_lost"));
F crates/pull/src/gdfl_dropbox_tests.rs:1811:9 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1812 [G3 PS3]: panic!("{report:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:1952:9 False [G3 PS3]: let Outcome::Refused(refusal) = &full.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:1953 [G3 PS3]: panic!("{full:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:2061 [G3 PS3]: again.refused[0].detail
F crates/pull/src/gdfl_dropbox_tests.rs:2095:9 False [G3 PS3]: let Outcome::Refused(refusal) = &stopped.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:2096 [G3 PS3]: panic!("{stopped:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:2133:57 False [G3 PS3]: matches!(&early.outcome, Outcome::Refused(r) if r.code == "stopped"),
F crates/pull/src/gdfl_dropbox_tests.rs:2306 [G3 PS3]: other => panic!("not refused: {other:?}"),
F crates/pull/src/gdfl_dropbox_tests.rs:2354 [G3 PS3]: report.refused[0]
F crates/pull/src/gdfl_dropbox_tests.rs:2481:13 False [G3 PS3]: let Outcome::Refused(refusal) = &report.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:2482 [G3 PS3]: panic!("{report:#?}");
F crates/pull/src/gdfl_dropbox_tests.rs:2589:13 False [G3 PS3]: let Outcome::Refused(refusal) = &outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:2590 [G3 PS3]: panic!("{outcome:?}");
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:2999 [G3 PS3]: fn measure_the_journal_costs_of_a_press() {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3000 [G3 PS3]: let mut names = Vec::new();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3001 [G3 PS3]: 'outer: for year in 2019..2027 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3002 [G3 PS3]: for month in 1..=12 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3003 [G3 PS3]: for day in 1..=28 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3004 [G3 PS3]: names.push(format!("GFDLNFO_TICK_{day:02}{month:02}{year}.zip"));
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3005 [G3 PS3]: if names.len() == 1_000 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3005:24 False [G3 PS3]: if names.len() == 1_000 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3005:24 True [G3 PS3]: if names.len() == 1_000 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3006 [G3 PS3]: break 'outer;
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3007 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3011 [G3 PS3]: let files: Vec<(String, Vec<u8>)> = names
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3012 [G3 PS3]: .iter()
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3013 [G3 PS3]: .enumerate()
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3014 [G3 PS3]: .map(|(i, name)| {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3015 [G3 PS3]: (
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3016 [G3 PS3]: format!("/{name}"),
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3017 [G3 PS3]: day_zip(u32::try_from(i).unwrap() + 1, 1, 30),
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3018 [G3 PS3]: )
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3019 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3020 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3021 [G3 PS3]: let refs: Vec<(&str, Vec<u8>)> =
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3022 [G3 PS3]: files.iter().map(|(p, b)| (p.as_str(), b.clone())).collect();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3023 [G3 PS3]: let server = Server::start(fake_with(&refs));
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3024 [G3 PS3]: let world = roots("r1-measure");
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3025 [G3 PS3]: for name in &names {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3026 [G3 PS3]: hold(&world, name);
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3027 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3028 [G3 PS3]: let mut before = 0;
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3029 [G3 PS3]: let mut added = Vec::new();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3030 [G3 PS3]: for press_no in 1..=3 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3031 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3032 [G3 PS3]: let report = go(&server, &world, &mut Recorder::default());
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3033 [G3 PS3]: let took = start.elapsed();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3034 [G3 PS3]: let lines = journal_text(&world).lines().count();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3035 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3036 [G3 PS3]: "MEASURE press {press_no}: n=1000 held={} outcome={} took_ms={} lines_added={}",
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3037 [G3 PS3]: report.held,
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3038 [G3 PS3]: report.outcome.word(),
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3039 [G3 PS3]: took.as_millis(),
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3040 [G3 PS3]: lines - before
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3041 [G3 PS3]: );
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3042 [G3 PS3]: added.push(lines - before);
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3043 [G3 PS3]: before = lines;
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3044 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3045 [G3 PS3]: assert_eq!(added[1], added[2]);
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3046 [G3 PS3]: assert!(added[1] <= 6, "{added:?}");
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3047 [G3 PS3]: for count in [1_000_usize, 10_000, 100_000] {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3048 [G3 PS3]: let roots = roots(&format!("r1-measure-{count}"));
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3049 [G3 PS3]: let path = journal_path(&roots.store);
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3050 [G3 PS3]: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3051 [G3 PS3]: let mut text = String::new();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3052 [G3 PS3]: for i in 0..count {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3053 [G3 PS3]: let _ = writeln!(
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3054 [G3 PS3]: text,
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3055 [G3 PS3]: "v=1 verified run=r at_ms={i} name=\"GFDLNFO_TICK_{:02}072019 \u{e9}t\u{e9} {i}.zip\" \
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3056 [G3 PS3]: rev=\"{i:011}\" hash=\"\" path=\"/s/{i}\" bytes=100 entries=1 sha256={i:064x} how=downloaded",
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3057 [G3 PS3]: 1 + i % 28
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3058 [G3 PS3]: );
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3059 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3060 [G3 PS3]: std::fs::write(&path, &text).unwrap();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3061 [G3 PS3]: let mut load = Vec::new();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3062 [G3 PS3]: let mut tail = Vec::new();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3063 [G3 PS3]: for _ in 0..31 {
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3064 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3065 [G3 PS3]: let memory = Memory::load(&path).unwrap();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3066 [G3 PS3]: load.push(start.elapsed());
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3067 [G3 PS3]: assert_eq!(memory.verified.len(), count);
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3068 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3069 [G3 PS3]: assert!(status(&roots.store).unwrap().is_some());
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3070 [G3 PS3]: tail.push(start.elapsed());
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3072 [G3 PS3]: load.sort();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3073 [G3 PS3]: tail.sort();
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3074 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3076 [G3 PS3]: text.len(),
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3077 [G3 PS3]: load[15],
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3078 [G3 PS3]: load[30],
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3079 [G3 PS3]: load[30],
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3080 [G3 PS3]: tail[15],
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3081 [G3 PS3]: tail[30],
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3082 [G3 PS3]: tail[30]
IGNORED crates/pull/src/gdfl_dropbox_tests.rs:3085 [G3 PS3]: }
F crates/pull/src/gdfl_dropbox_tests.rs:3105 [G3 PS3]: report.refused[0]
F crates/pull/src/gdfl_dropbox_tests.rs:3182 [G3 PS3]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_dropbox_tests.rs:3229:9 False [G3 PS3]: accepted[0].contains(" complete=true count=2 ")
F crates/pull/src/gdfl_dropbox_tests.rs:3230:16 False [G3 PS3]: && accepted[0].ends_with(&format!(" accepted_shrink={token} was=3")),
F crates/pull/src/gdfl_dropbox_tests.rs:3232 [G3 PS3]: accepted[0]
F crates/pull/src/gdfl_dropbox_tests.rs:3486 [G3 PS3]: codes(&two)
F crates/pull/src/gdfl_dropbox_tests.rs:3547:9 False [G3 PS3]: let Outcome::Refused(refusal) = &two.outcome else {
F crates/pull/src/gdfl_dropbox_tests.rs:3548 [G3 PS3]: panic!("{two:#?}")
F crates/pull/src/gdfl_dropbox_tests.rs:3909 [G3 PS3]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_dropbox_tests.rs:4092:13 False [G3 PS3]: detail.starts_with("pages=1 listing: ") && detail.ends_with("(host 127.0.0.1)"),
F crates/pull/src/gdfl_dropbox_tests.rs:4092:56 False [G3 PS3]: detail.starts_with("pages=1 listing: ") && detail.ends_with("(host 127.0.0.1)"),
F crates/pull/src/gdfl_dropbox_tests.rs:4557 [G3 PS3]: journal_text(&roots)
F crates/pull/src/gdfl_dropbox_tests.rs:4602 [G3 PS3]: journal_text(&roots)
F crates/pull/src/gdfl_format_change_tests.rs:129:13 False [G2 PS2]: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
F crates/pull/src/gdfl_format_change_tests.rs:129:52 False [G2 PS2]: assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
F crates/pull/src/gdfl_format_change_tests.rs:221:22 False [G2 PS2]: .any(|f| f.instrument.contains("2024-04-09") && f.why.contains(&shape)),
F crates/pull/src/gdfl_format_change_tests.rs:274:13 False [G2 PS2]: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
F crates/pull/src/gdfl_format_change_tests.rs:274:41 False [G2 PS2]: assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
F crates/pull/src/gdfl_format_change_tests.rs:283:22 False [G2 PS2]: .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
F crates/pull/src/gdfl_format_change_tests.rs:373 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_ia16_tests.rs:198:13 False [G3 PS3]: kind.is_hole() && HOLE_REASONS.contains(&kind.code()),
F crates/pull/src/gdfl_ia16_tests.rs:198:31 False [G3 PS3]: kind.is_hole() && HOLE_REASONS.contains(&kind.code()),
F crates/pull/src/gdfl_ia16_tests.rs:314:13 False [G3 PS3]: assert!(restated.restated && !restated.done && restated.hole.is_none());
F crates/pull/src/gdfl_ia16_tests.rs:314:34 False [G3 PS3]: assert!(restated.restated && !restated.done && restated.hole.is_none());
F crates/pull/src/gdfl_ia16_tests.rs:314:52 False [G3 PS3]: assert!(restated.restated && !restated.done && restated.hole.is_none());
F crates/pull/src/gdfl_ia16_tests.rs:342:13 False [G3 PS3]: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
F crates/pull/src/gdfl_ia16_tests.rs:342:37 False [G3 PS3]: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
F crates/pull/src/gdfl_ia16_tests.rs:342:64 False [G3 PS3]: assert!(hole.cause.is_hole() && differs.cause.is_hole() && !busy.cause.is_hole());
F crates/pull/src/gdfl_ia16_tests.rs:394:13 False [G3 PS3]: line.starts_with("incomplete ")
F crates/pull/src/gdfl_ia16_tests.rs:395:20 False [G3 PS3]: && line.contains(" stage=file ")
F crates/pull/src/gdfl_ia16_tests.rs:396:20 False [G3 PS3]: && line.contains(" reason=needs_month_generation"),
F crates/pull/src/gdfl_ia16_tests.rs:563:13 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=needs_month_generation"),
F crates/pull/src/gdfl_ia16_tests.rs:563:48 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=needs_month_generation"),
F crates/pull/src/gdfl_ia16_tests.rs:647:29 False [G3 PS3]: .any(|line| line.starts_with("repairing ") && line.contains("sealed-month")),
F crates/pull/src/gdfl_ia16_tests.rs:747:13 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:747:48 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:846:9 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:846:44 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:873:13 False [G3 PS3]: again.restated.is_empty() && again.resumed.is_empty(),
F crates/pull/src/gdfl_ia16_tests.rs:873:42 False [G3 PS3]: again.restated.is_empty() && again.resumed.is_empty(),
F crates/pull/src/gdfl_ia16_tests.rs:955:9 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:955:44 False [G3 PS3]: line.starts_with("incomplete ") && line.contains(" reason=bars_differ_held"),
F crates/pull/src/gdfl_ia16_tests.rs:966:9 False [G3 PS3]: failure.why.contains(&first_offered.to_string()) && failure.why.contains("2024-04"),
F crates/pull/src/gdfl_ia16_tests.rs:966:61 False [G3 PS3]: failure.why.contains(&first_offered.to_string()) && failure.why.contains("2024-04"),
F crates/pull/src/gdfl_ia16_tests.rs:1840:71 False [G3 PS3]: matches!(&ended, ImportRefusal::SourceLost { day, .. } if *day == d2()),
F crates/pull/src/gdfl_ia16_tests.rs:1846:13 False [G3 PS3]: !text.contains("2024-04-02") && !text.contains("2024-04-03"),
F crates/pull/src/gdfl_ia16_tests.rs:1846:45 False [G3 PS3]: !text.contains("2024-04-02") && !text.contains("2024-04-03"),
F crates/pull/src/gdfl_ia16_tests.rs:1891:13 False [G3 PS3]: line.starts_with("refused ") && line.contains(" reason=no_wanted_file"),
F crates/pull/src/gdfl_ia16_tests.rs:1891:45 False [G3 PS3]: line.starts_with("refused ") && line.contains(" reason=no_wanted_file"),
F crates/pull/src/gdfl_ia16_tests.rs:1943:24 False [G3 PS3]: .any(|one| one.instrument.ends_with("BANKNIFTY") && one.why.contains("absent")),
F crates/pull/src/gdfl_ia16_tests.rs:2142:9 False [G3 PS3]: line.starts_with("done ") && !line.contains("vendor_refused"),
F crates/pull/src/gdfl_ia16_tests.rs:2142:38 False [G3 PS3]: line.starts_with("done ") && !line.contains("vendor_refused"),
F crates/pull/src/gdfl_ia16_tests.rs:2176:68 False [G3 PS3]: Some(RunRefusal::RepairReasonInvalid { bytes }) if bytes == bad.len()
F crates/pull/src/gdfl_ia16_tests.rs:2231 [G3 PS3]: matches!(
F crates/pull/src/gdfl_ia16_tests.rs:2292 [G3 PS3]: matches!(
F crates/pull/src/gdfl_ia16_tests.rs:2488:13 False [G3 PS3]: assert!(said.contains("NIFTY") && said.contains("4097"), "{said}");
F crates/pull/src/gdfl_ia16_tests.rs:2488:39 False [G3 PS3]: assert!(said.contains("NIFTY") && said.contains("4097"), "{said}");
F crates/pull/src/gdfl_ia16_tests.rs:2733:13 False [G3 PS3]: said.contains("chflags nouchg")
F crates/pull/src/gdfl_ia16_tests.rs:2734:20 False [G3 PS3]: && said.contains("chmod -N")
F crates/pull/src/gdfl_ia16_tests.rs:2735:20 False [G3 PS3]: && said.contains("read-write"),
F crates/pull/src/gdfl_ia16_tests.rs:2833:9 False [G3 PS3]: said.contains("999 bytes free of 10000") && said.contains("floor of 1000"),
F crates/pull/src/gdfl_ia16_tests.rs:2833:53 False [G3 PS3]: said.contains("999 bytes free of 10000") && said.contains("floor of 1000"),
F crates/pull/src/gdfl_ia16_tests.rs:2874:13 False [G3 PS3]: assert!(space.total >= space.free && space.total > 0, "{space:?}");
F crates/pull/src/gdfl_ia16_tests.rs:2874:42 False [G3 PS3]: assert!(space.total >= space.free && space.total > 0, "{space:?}");
F crates/pull/src/gdfl_ia16_tests.rs:2932:9 False [G3 PS3]: two.starts_with("incomplete ") && two.contains(" reason=source_crc_mismatch"),
F crates/pull/src/gdfl_ia16_tests.rs:2932:43 False [G3 PS3]: two.starts_with("incomplete ") && two.contains(" reason=source_crc_mismatch"),
F crates/pull/src/gdfl_ia16_tests.rs:2969:56 True [G3 PS3]: !(one.instrument.contains("NIFTY BANK") && one.why.contains("needs_month_generation")),
F crates/pull/src/gdfl_ia3_tests.rs:174:9 False [G2 PS2]: !line.contains("listing=") && !line.contains("skipped="),
F crates/pull/src/gdfl_ia3_tests.rs:174:39 False [G2 PS2]: !line.contains("listing=") && !line.contains("skipped="),
F crates/pull/src/gdfl_ia3_tests.rs:322:9 False [G2 PS2]: line.starts_with("refused indices 2024-04-01 * ")
F crates/pull/src/gdfl_ia3_tests.rs:323:16 False [G2 PS2]: && line.contains("stage=vendor reason=day_file_misfiled"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:389 [G2 PS2]: fn quantiles(samples: &mut [u128]) -> (u128, u128, u128) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:390 [G2 PS2]: samples.sort_unstable();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:391 [G2 PS2]: let at = |q: usize| samples.get((samples.len() * q / 100).min(samples.len().saturating_sub(1)));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:392 [G2 PS2]: (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:393 [G2 PS2]: at(50).copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:394 [G2 PS2]: at(99).copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:395 [G2 PS2]: samples.last().copied().unwrap_or(0),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:396 [G2 PS2]: )
IGNORED crates/pull/src/gdfl_ia3_tests.rs:397 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:407 [G2 PS2]: fn ia3_lookup_costs() {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:408 [G2 PS2]: let days: Vec<Day> = (0..2_600)
IGNORED crates/pull/src/gdfl_ia3_tests.rs:409 [G2 PS2]: .filter_map(|n| Day::from_days(Day::new(2018, 9, 3).unwrap().days_from_epoch() + n).ok())
IGNORED crates/pull/src/gdfl_ia3_tests.rs:410 [G2 PS2]: .collect();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411 [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411:12 False [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:411:12 True [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_TICKSTORE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:412 [G2 PS2]: tickstore_costs(Path::new(&root), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:413 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414 [G2 PS2]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414:12 False [G2 PS2]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:414:12 True [G2 PS2]: if let Some(path) = std::env::var_os("BRUTEX_GDFL_CM_ARCHIVE") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:415 [G2 PS2]: archive_costs(Path::new(&path), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:416 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417 [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417:12 False [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:417:12 True [G2 PS2]: if let Some(root) = std::env::var_os("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:418 [G2 PS2]: dayzips_costs(Path::new(&root), &days);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:419 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:420 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:424 [G2 PS2]: fn tickstore_costs(root: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:425 [G2 PS2]: let store = TickStore::new(root);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:426 [G2 PS2]: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:427 [G2 PS2]: let table = store.table();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:428 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:430 [G2 PS2]: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:435 [G2 PS2]: table.stray_named.iter().take(4).collect::<Vec<_>>()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:437 [G2 PS2]: let mut probe: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:438 [G2 PS2]: for day in days.iter().cycle().take(20_000) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:439 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:440 [G2 PS2]: let _ = std::hint::black_box(table.misfiled(0, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:441 [G2 PS2]: probe.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:442 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:443 [G2 PS2]: let n = probe.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:444 [G2 PS2]: let (p50, p99, max) = quantiles(&mut probe);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:445 [G2 PS2]: println!("tickstore misfiled probe: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:446 [G2 PS2]: let mut listing: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:447 [G2 PS2]: let mut sums: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:448 [G2 PS2]: for day in days.iter().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:449 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:450 [G2 PS2]: let got = store.day(CmKind::Indices, *day);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:451 [G2 PS2]: listing.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452 [G2 PS2]: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452:16 False [G2 PS2]: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:452:16 True [G2 PS2]: if let Ok(Some(listed)) = got {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:453 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:454 [G2 PS2]: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:455 [G2 PS2]: sums.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:456 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:458 [G2 PS2]: let n = listing.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:459 [G2 PS2]: let (p50, p99, max) = quantiles(&mut listing);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:460 [G2 PS2]: println!("tickstore day listing (open + index): n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:461 [G2 PS2]: let n = sums.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:462 [G2 PS2]: let (p50, p99, max) = quantiles(&mut sums);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:463 [G2 PS2]: println!("indices listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:464 [G2 PS2]: let nfo = crate::gdfl_nfo::NfoTickStore::new(root);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:465 [G2 PS2]: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:466 [G2 PS2]: let table = nfo.table();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:467 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:469 [G2 PS2]: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:474 [G2 PS2]: let mut sums: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:475 [G2 PS2]: for day in days.iter().rev().take(60) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476 [G2 PS2]: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476:16 False [G2 PS2]: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:476:16 True [G2 PS2]: if let Ok(Some(listed)) = crate::gdfl_nfo::NfoSource::day(&nfo, *day) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:477 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:478 [G2 PS2]: let _ = std::hint::black_box(ListingSum::of(listed.entries()));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:479 [G2 PS2]: sums.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:480 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:482 [G2 PS2]: let n = sums.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:483 [G2 PS2]: let (p50, p99, max) = quantiles(&mut sums);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:484 [G2 PS2]: println!("options listing digest: n={n} p50={p50} ns p99={p99} ns max={max} ns");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:485 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:488 [G2 PS2]: fn archive_costs(path: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:489 [G2 PS2]: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:490 [G2 PS2]: let archive = Archive::open(path).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:491 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:493 [G2 PS2]: cold.elapsed().as_millis(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:494 [G2 PS2]: archive.census()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:496 [G2 PS2]: let mut lookups: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:497 [G2 PS2]: for day in days.iter().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:498 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:499 [G2 PS2]: let _ = std::hint::black_box(archive.day(CmKind::Indices, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:500 [G2 PS2]: lookups.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:501 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:502 [G2 PS2]: let n = lookups.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:503 [G2 PS2]: let (p50, p99, max) = quantiles(&mut lookups);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:504 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:507 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:510 [G2 PS2]: fn dayzips_costs(root: &Path, days: &[Day]) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:511 [G2 PS2]: let cold = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:512 [G2 PS2]: let loose = crate::gdfl_dayzips::DayZips::open(root).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:513 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:515 [G2 PS2]: cold.elapsed().as_micros(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:516 [G2 PS2]: loose.census()
IGNORED crates/pull/src/gdfl_ia3_tests.rs:518 [G2 PS2]: let mut lookups: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:519 [G2 PS2]: for day in days.iter().rev().take(400) {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:520 [G2 PS2]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:521 [G2 PS2]: let _ = std::hint::black_box(CmSource::day(&loose, CmKind::Indices, *day));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:522 [G2 PS2]: lookups.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:523 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:524 [G2 PS2]: let n = lookups.len();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:525 [G2 PS2]: let (p50, p99, max) = quantiles(&mut lookups);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:526 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:529 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:545 [G2 PS2]: fn one_day<'a>(kind: ImportKind, d: Day, only: &'a [String], at: &'a Path) -> Run<'a> {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:546 [G2 PS2]: Run {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:547 [G2 PS2]: kind,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:548 [G2 PS2]: from: d,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:549 [G2 PS2]: to: d,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:550 [G2 PS2]: only,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:551 [G2 PS2]: store_root: at,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:552 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:553 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:556 [G2 PS2]: fn listing_of(line: &str) -> Option<&str> {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:557 [G2 PS2]: line.split(' ')
IGNORED crates/pull/src/gdfl_ia3_tests.rs:558 [G2 PS2]: .find_map(|field| field.strip_prefix("listing="))
IGNORED crates/pull/src/gdfl_ia3_tests.rs:559 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:570 [G2 PS2]: fn real_days_close_with_the_same_listing_from_two_sources() {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:571 [G2 PS2]: let var = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572 [G2 PS2]: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572:9 False [G2 PS2]: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:572:9 True [G2 PS2]: let (Some(ts), Some(cm), Some(nfo)) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:573 [G2 PS2]: var("BRUTEX_GDFL_TICKSTORE"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:574 [G2 PS2]: var("BRUTEX_GDFL_CM_ARCHIVE"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:575 [G2 PS2]: var("BRUTEX_GDFL_NFO_ARCHIVES"),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:577 [G2 PS2]: return;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:579 [G2 PS2]: let archive = Archive::open(&cm).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:580 [G2 PS2]: let store = TickStore::new(&ts);
IGNORED crates/pull/src/gdfl_ia3_tests.rs:581 [G2 PS2]: let root = scratch("ia3-real");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:582 [G2 PS2]: let cm_case = |tag: &str, kind: ImportKind, d: Day, only: &[String]| {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:583 [G2 PS2]: let a = root.join(format!("{tag}-zip"));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:584 [G2 PS2]: let b = root.join(format!("{tag}-ts"));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:585 [G2 PS2]: let zip_report = run_cm(&archive, &one_day(kind, d, only, &a)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:586 [G2 PS2]: let ts_report = run_cm(&store, &one_day(kind, d, only, &b)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:587 [G2 PS2]: let (zip_line, store_line) = (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:588 [G2 PS2]: close_of(&a, kind.as_str(), d),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:589 [G2 PS2]: close_of(&b, kind.as_str(), d),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:590 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:591 [G2 PS2]: println!("{tag} zip: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:592 [G2 PS2]: println!("{tag} tickstore: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:593 [G2 PS2]: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:594 [G2 PS2]: assert_eq!(listing_of(&zip_line), listing_of(&store_line), "{tag}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:595 [G2 PS2]: assert_eq!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:596 [G2 PS2]: (
IGNORED crates/pull/src/gdfl_ia3_tests.rs:597 [G2 PS2]: zip_report.files,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:598 [G2 PS2]: zip_report.seconds,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:599 [G2 PS2]: zip_report.files_skipped
IGNORED crates/pull/src/gdfl_ia3_tests.rs:600 [G2 PS2]: ),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:601 [G2 PS2]: (ts_report.files, ts_report.seconds, ts_report.files_skipped),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:604 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_ia3_tests.rs:605 [G2 PS2]: let nifty = ["NIFTY".to_owned()];
IGNORED crates/pull/src/gdfl_ia3_tests.rs:606 [G2 PS2]: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:607 [G2 PS2]: "nifty-spot",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:608 [G2 PS2]: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:609 [G2 PS2]: Day::new(2024, 4, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:610 [G2 PS2]: &nifty,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:611 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:612 [G2 PS2]: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:613 [G2 PS2]: "muhurat",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:614 [G2 PS2]: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:615 [G2 PS2]: Day::new(2024, 11, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:616 [G2 PS2]: &nifty,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:617 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:618 [G2 PS2]: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:619 [G2 PS2]: "banknifty",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:620 [G2 PS2]: ImportKind::Indices,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:621 [G2 PS2]: Day::new(2020, 3, 23).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:622 [G2 PS2]: &["BANKNIFTY".to_owned()],
IGNORED crates/pull/src/gdfl_ia3_tests.rs:623 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:624 [G2 PS2]: cm_case(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:625 [G2 PS2]: "reliance",
IGNORED crates/pull/src/gdfl_ia3_tests.rs:626 [G2 PS2]: ImportKind::Stocks,
IGNORED crates/pull/src/gdfl_ia3_tests.rs:627 [G2 PS2]: Day::new(2024, 4, 1).unwrap(),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:628 [G2 PS2]: &["RELIANCE".to_owned()],
IGNORED crates/pull/src/gdfl_ia3_tests.rs:629 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_ia3_tests.rs:630 [G2 PS2]: let d = Day::new(2024, 4, 4).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:631 [G2 PS2]: let a = root.join("nifty-options-zip");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:632 [G2 PS2]: let b = root.join("nifty-options-ts");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:633 [G2 PS2]: let options = ImportKind::Options;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:634 [G2 PS2]: let zips = run_nfo(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:635 [G2 PS2]: &crate::gdfl_nfo::NfoZips::new(&nfo),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:636 [G2 PS2]: &one_day(options, d, &nifty, &a),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:638 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:639 [G2 PS2]: let tick = run_nfo(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:640 [G2 PS2]: &crate::gdfl_nfo::NfoTickStore::new(&ts),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:641 [G2 PS2]: &one_day(options, d, &nifty, &b),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:643 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:644 [G2 PS2]: let (zip_line, store_line) = (close_of(&a, "options", d), close_of(&b, "options", d));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:645 [G2 PS2]: println!("nifty-options zip: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:646 [G2 PS2]: println!("nifty-options tickstore: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:647 [G2 PS2]: assert!(listing_of(&zip_line).is_some(), "{zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:648 [G2 PS2]: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:649 [G2 PS2]: assert_eq!((zips.files, zips.seconds), (tick.files, tick.seconds));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650 [G2 PS2]: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650:12 False [G2 PS2]: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:650:12 True [G2 PS2]: if let Some(loose) = var("BRUTEX_GDFL_DAYZIPS") {
IGNORED crates/pull/src/gdfl_ia3_tests.rs:651 [G2 PS2]: let loose = crate::gdfl_dayzips::DayZips::open(&loose).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:652 [G2 PS2]: let d = Day::new(2026, 9, 25).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:653 [G2 PS2]: let a = root.join("loose-nifty");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:654 [G2 PS2]: let b = root.join("loose-nifty-ts");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:655 [G2 PS2]: let indices = ImportKind::Indices;
IGNORED crates/pull/src/gdfl_ia3_tests.rs:656 [G2 PS2]: let from_loose = run_cm(&loose, &one_day(indices, d, &nifty, &a)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:657 [G2 PS2]: let from_ts = run_cm(&store, &one_day(indices, d, &nifty, &b)).unwrap();
IGNORED crates/pull/src/gdfl_ia3_tests.rs:658 [G2 PS2]: let (zip_line, store_line) = (close_of(&a, "indices", d), close_of(&b, "indices", d));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:659 [G2 PS2]: println!("loose 2026-09-25: {zip_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:660 [G2 PS2]: println!("tickstore 2026-09-25: {store_line}");
IGNORED crates/pull/src/gdfl_ia3_tests.rs:661 [G2 PS2]: assert_eq!(listing_of(&zip_line), listing_of(&store_line));
IGNORED crates/pull/src/gdfl_ia3_tests.rs:662 [G2 PS2]: assert_eq!(
IGNORED crates/pull/src/gdfl_ia3_tests.rs:663 [G2 PS2]: (from_loose.files, from_loose.seconds),
IGNORED crates/pull/src/gdfl_ia3_tests.rs:664 [G2 PS2]: (from_ts.files, from_ts.seconds)
IGNORED crates/pull/src/gdfl_ia3_tests.rs:666 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_ia3_tests.rs:667 [G2 PS2]: println!("scratch stores under {}", root.display());
IGNORED crates/pull/src/gdfl_ia3_tests.rs:668 [G2 PS2]: }
F crates/pull/src/gdfl_ia3_tests.rs:867:13 False [G2 PS2]: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
F crates/pull/src/gdfl_ia3_tests.rs:867:53 False [G2 PS2]: assert!(!why.to_string().contains("corrupt") && !why.to_string().contains("CRC"));
F crates/pull/src/gdfl_ia4_tests.rs:154 [G3 PS3]: kind.as_str()
F crates/pull/src/gdfl_ia4_tests.rs:275:9 False [G3 PS3]: last.starts_with("done ") && last.contains(" definition=5 "),
F crates/pull/src/gdfl_ia4_tests.rs:275:38 False [G3 PS3]: last.starts_with("done ") && last.contains(" definition=5 "),
U crates/pull/src/gdfl_import.rs:2654 [G1 PS1]: Ok(None) => false,
F crates/pull/src/gdfl_import_cov_tests.rs:48:9 False [G1 PS1]: why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
F crates/pull/src/gdfl_import_cov_tests.rs:48:56 False [G1 PS1]: why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
F crates/pull/src/gdfl_import_cov_tests.rs:57:9 False [G1 PS1]: why.contains("lies past the end") && !why.contains("over the cap"),
F crates/pull/src/gdfl_import_cov_tests.rs:57:46 False [G1 PS1]: why.contains("lies past the end") && !why.contains("over the cap"),
F crates/pull/src/gdfl_import_cov_tests.rs:89:32 False [G1 PS1]: .is_some_and(|why| why.contains("plain.zip") && why.contains("no end-of-central")),
F crates/pull/src/gdfl_import_cov_tests.rs:150:9 False [G1 PS1]: fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
F crates/pull/src/gdfl_import_cov_tests.rs:150:74 False [G1 PS1]: fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
F crates/pull/src/gdfl_import_cov_tests.rs:179:9 False [G1 PS1]: fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
F crates/pull/src/gdfl_import_cov_tests.rs:179:40 False [G1 PS1]: fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
F crates/pull/src/gdfl_import_cov_tests.rs:262:62 False [G1 PS1]: matches!(&refused, ImportRefusal::Journal { why } if why.contains("gdfl.journal")),
U crates/pull/src/gdfl_import_day.rs:1207 [G1 PS1]: Refused::at(
U crates/pull/src/gdfl_import_day.rs:1208 [G1 PS1]: Cause::open(Stage::File, "file_taken_twice"),
U crates/pull/src/gdfl_import_day.rs:1209 [G1 PS1]: failure("file", "a handed file was taken twice".to_owned()),
U crates/pull/src/gdfl_import_day.rs:1211 [G1 PS1]: })
U crates/pull/src/gdfl_import_day.rs:1747 [G1 PS1]: from: first,
U crates/pull/src/gdfl_import_day.rs:1748 [G1 PS1]: to: last,
U crates/pull/src/gdfl_import_day.rs:1749 [G1 PS1]: })?,
F crates/pull/src/gdfl_import_day_ps1_tests.rs:428:9 False [G1 PS1]: let Owed::Probe(early) = ctx.owed(&own(&key, april(1), true)) else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:429 [G1 PS1]: panic!("a day before the range is never read whole");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:433:9 False [G1 PS1]: let Owed::Probe(beyond) = plain.owed(&own(&key, april(1), false)) else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:434 [G1 PS1]: panic!("without the flag the close still owes ZQB");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:477:9 False [G1 PS1]: let Owed::Pending(members) = ctx.owed(&own(&key, april(1), false)) else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:478 [G1 PS1]: panic!("ZQZ's own key is still open");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:499:9 False [G1 PS1]: let Owed::Probe(one) = ctx.owed(&own(&first, april(1), false)) else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:500 [G1 PS1]: panic!("a close under an unrecorded scope owes the members beyond the swept surface");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:502:9 False [G1 PS1]: let Owed::Probe(two) = ctx.owed(&own(&second, april(2), false)) else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:503 [G1 PS1]: panic!("so does the second day's");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:534:9 False [G1 PS1]: let [refused] = report.failures.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:535 [G1 PS1]: panic!("{:?}", report.failures);
F crates/pull/src/gdfl_import_day_ps1_tests.rs:539:9 False [G1 PS1]: refused.why.starts_with("the import journal: ")
F crates/pull/src/gdfl_import_day_ps1_tests.rs:540:16 False [G1 PS1]: && refused.why.contains(&ledger.display().to_string()),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:580 [G1 PS1]: panic!("a restated day is not read when the run asks not to restate")
F crates/pull/src/gdfl_import_day_ps1_tests.rs:591:13 False [G1 PS1]: assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
F crates/pull/src/gdfl_import_day_ps1_tests.rs:591:46 False [G1 PS1]: assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
F crates/pull/src/gdfl_import_day_ps1_tests.rs:622 [G1 PS1]: unreachable!("a day every share of which stands asks no scope")
F crates/pull/src/gdfl_import_day_ps1_tests.rs:732 [G1 PS1]: Ask::All => panic!("the day was closed done: read again for its new members only"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:772:66 False [G1 PS1]: last_line_of(&store, &member_key).is_some_and(|line| line
F crates/pull/src/gdfl_import_day_ps1_tests.rs:776 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:816 [G1 PS1]: Ask::All => panic!("both days were closed done: read again for ZQB only"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:825:9 False [G1 PS1]: let [line] = lines.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:826 [G1 PS1]: panic!("{flags}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:939:9 False [G1 PS1]: let [held] = key_held.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:940 [G1 PS1]: panic!("{key_held:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:947:9 False [G1 PS1]: let [finished] = run_finished.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:948 [G1 PS1]: panic!("{run_finished:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:956:9 False [G1 PS1]: let [unwritten] = not_written.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:957 [G1 PS1]: panic!("{not_written:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:981:9 False [G1 PS1]: let [said] = unrecorded_said.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:982 [G1 PS1]: panic!("{unrecorded_said:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1016:9 False [G1 PS1]: said.starts_with("run 1790100201 ")
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1017:16 False [G1 PS1]: && said.contains(&format!("\ndone indices {} * ", april(1))),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1048 [G1 PS1]: |_, _| -> DayRead { panic!("no day is begun") },
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1051:9 False [G1 PS1]: let ImportRefusal::Stopped {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1059 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1089 [G1 PS1]: panic!("a refused run reads no day")
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1092:9 False [G1 PS1]: let ImportRefusal::Journal { why } = &got else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1093 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1124 [G1 PS1]: |_, _| -> DayRead { panic!("a refused run reads no day") },
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1127:9 False [G1 PS1]: let ImportRefusal::StoreLost {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1134 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1170:9 False [G1 PS1]: let ImportRefusal::StoreLost {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1177 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1181:9 False [G1 PS1]: why.starts_with(&format!(
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1184:15 False [G1 PS1]: )) && why.contains("File exists"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1214:9 False [G1 PS1]: let ImportRefusal::StoreLost {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1221 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1276 [G1 PS1]: Ask::All => panic!("no day of the range is begun"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1280:9 False [G1 PS1]: let ImportRefusal::StoreNotWritable {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1287 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1298 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1386 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1397:9 False [G1 PS1]: let [said] = reach.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1398 [G1 PS1]: panic!("{reach:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1473 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1597 [G1 PS1]: Ask::All => panic!("the day was closed done: read again for ZQB only"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1621 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1665:9 False [G1 PS1]: let [named] = refused.failures.as_slice() else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1666 [G1 PS1]: panic!("{:?}", refused.failures);
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1701 [G1 PS1]: scopes_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1754:9 False [G1 PS1]: let ImportRefusal::SourceLost {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1761 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1777:33 False [G1 PS1]: .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1780 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1806:9 False [G1 PS1]: let ImportRefusal::StoreNotWritable {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1813 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:9 False [G1 PS1]: why.contains("planted by the test") && why.contains("chflags nouchg"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:48 False [G1 PS1]: why.contains("planted by the test") && why.contains("chflags nouchg"),
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1824:33 False [G1 PS1]: .is_some_and(|line| line.starts_with(&format!("incomplete {key} "))
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1827 [G1 PS1]: journal_text(&store)
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1857:9 False [G1 PS1]: let ImportRefusal::StoreLost { why, .. } = &got else {
F crates/pull/src/gdfl_import_day_ps1_tests.rs:1858 [G1 PS1]: panic!("{got:?}");
U crates/pull/src/gdfl_import_filing.rs:116 [G2 PS2]: Ok(store::provenance::Recorded::NothingAppended) => Ok(false),
U crates/pull/src/gdfl_import_filing.rs:356:32 False [G2 PS2]: if let Some(slot) = slots.get_mut(index) {
U crates/pull/src/gdfl_import_filing.rs:696:20 False [G2 PS2]: && let Some(held) = done.pending.as_ref()
U crates/pull/src/gdfl_import_filing.rs:750:31 False [G2 PS2]: Some(kind) if at + 1 == last => {
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:119 [G2 PS2]: &|_: &i32| -> i32 { unreachable!("a stopped worker takes no job") },
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:183:47 False [G2 PS2]: ImportRefusal::Journal { why } if why == "the day's begin line was not written"
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:201 [G2 PS2]: &|_: &i32| -> i32 { unreachable!("no worker started, so no job runs") },
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:380:13 False [G2 PS2]: let [Ok((filed, None))] = done.as_slice() else {
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:381 [G2 PS2]: panic!("one filed job");
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:389 [G2 PS2]: filed
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:390 [G2 PS2]: .failures
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:391 [G2 PS2]: .iter()
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:392 [G2 PS2]: .map(|s| &s.failure.why)
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:393 [G2 PS2]: .collect::<Vec<_>>()
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:514:13 False [G2 PS2]: let [JobRead::Read(None), JobRead::Refused(second)] = answered.as_slice() else {
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:515 [G2 PS2]: panic!("the first read, the second not taken");
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:629:13 False [G2 PS2]: let [one] = filed.failures.as_slice() else {
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:630 [G2 PS2]: panic!("{:?}", filed.failures);
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:669:13 False [G2 PS2]: let [one] = filed.failures.as_slice() else {
F crates/pull/src/gdfl_import_filing_ps2_tests.rs:670 [G2 PS2]: panic!("{:?}", filed.failures);
F crates/pull/src/gdfl_import_ia2_tests.rs:115:9 False [G3 PS3]: let ImportRefusal::Stopped {
F crates/pull/src/gdfl_import_ia2_tests.rs:119 [G3 PS3]: panic!("{got:?}");
F crates/pull/src/gdfl_import_ia2_tests.rs:142:9 False [G3 PS3]: closed.starts_with("incomplete indices 2024-04-02 * ")
F crates/pull/src/gdfl_import_ia2_tests.rs:143:16 False [G3 PS3]: && closed.contains(" stage=run reason=stopped at=")
F crates/pull/src/gdfl_import_ia2_tests.rs:144:16 False [G3 PS3]: && closed.ends_with(" run=1790000101"),
F crates/pull/src/gdfl_import_ia2_tests.rs:151:9 False [G3 PS3]: last.starts_with("run 1790000101 ended=")
F crates/pull/src/gdfl_import_ia2_tests.rs:152:16 False [G3 PS3]: && last.contains(" outcome=stopped ")
F crates/pull/src/gdfl_import_ia2_tests.rs:153:16 False [G3 PS3]: && last.ends_with(&format!(" definition={BAR_DEFINITION}")),
F crates/pull/src/gdfl_import_ia2_tests.rs:204:71 False [G3 PS3]: matches!(&got, ImportRefusal::Stopped { day: Some(d), .. } if *d == day(2)),
F crates/pull/src/gdfl_import_ia2_tests.rs:225:77 False [G3 PS3]: matches!(&got, ImportRefusal::Stopped { day: Some(d), done, .. } if *d == day(1) && done.days_imported == 1),
F crates/pull/src/gdfl_import_ia2_tests.rs:225:93 False [G3 PS3]: matches!(&got, ImportRefusal::Stopped { day: Some(d), done, .. } if *d == day(1) && done.days_imported == 1),
F crates/pull/src/gdfl_import_ia2_tests.rs:267:9 False [G3 PS3]: let ImportRefusal::Stopped {
F crates/pull/src/gdfl_import_ia2_tests.rs:271 [G3 PS3]: panic!("{got:?}");
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:464 [G3 PS3]: fn journal_append_cost_is_flat_with_a_second_writer() {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:465 [G3 PS3]: for size in [1_000_u32, 20_000] {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:466 [G3 PS3]: let store = scratch(&format!("flat-{size}"));
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:467 [G3 PS3]: std::fs::create_dir_all(store.join("imports")).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:468 [G3 PS3]: let path = journal_path(&store);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:469 [G3 PS3]: let mut seed = String::new();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:470 [G3 PS3]: for i in 0..size {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:471 [G3 PS3]: let _ = writeln!(seed, "begin indices 2024-04-01 S{i}");
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:472 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:473 [G3 PS3]: std::fs::write(&path, seed).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:474 [G3 PS3]: let mine = Journal::load(path.clone()).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:475 [G3 PS3]: let other = Journal::load(path).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:476 [G3 PS3]: let mut took = Vec::with_capacity(300);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:477 [G3 PS3]: for i in 0..300_u32 {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:478 [G3 PS3]: other
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:479 [G3 PS3]: .append(&format!("begin stocks 2024-04-01 O{i}"))
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:480 [G3 PS3]: .unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:481 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:482 [G3 PS3]: mine.append(&format!("begin indices 2024-04-01 M{i}"))
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:483 [G3 PS3]: .unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:484 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:485 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:486 [G3 PS3]: took.sort_unstable();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:487 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:490 [G3 PS3]: took[150], took[296], took[299]
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:493 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:708 [G3 PS3]: fn ia2_repair2_timings() {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:709 [G3 PS3]: fn pct(mut v: Vec<std::time::Duration>) -> String {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:710 [G3 PS3]: v.sort_unstable();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:711 [G3 PS3]: let n = v.len();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:712 [G3 PS3]: format!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:714 [G3 PS3]: v[n / 2],
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:715 [G3 PS3]: v[n * 99 / 100],
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:716 [G3 PS3]: v[n - 1]
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:718 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:719 [G3 PS3]: let keys = 100_000_u32;
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:720 [G3 PS3]: let lines: Vec<String> = (0..keys)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:721 [G3 PS3]: .flat_map(|i| {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:722 [G3 PS3]: let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:723 [G3 PS3]: [
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:724 [G3 PS3]: format!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:725 [G3 PS3]: "incomplete {key} definition=4 files=1 seconds={} failures=1 stage=file \
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:726 [G3 PS3]: reason=filing at=1790000001 run={}",
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:727 [G3 PS3]: i % 2,
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:728 [G3 PS3]: 1_790_000_000 + u64::from(i)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:729 [G3 PS3]: ),
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:730 [G3 PS3]: format!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:731 [G3 PS3]: "done {key} definition=4 files=1 seconds=60 failures=0 at=1790000002 run={}",
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:732 [G3 PS3]: 1_790_100_000 + u64::from(i)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:733 [G3 PS3]: ),
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:734 [G3 PS3]: ]
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:735 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:736 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:737 [G3 PS3]: let mut journal = Journal::default();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:738 [G3 PS3]: let mut took = Vec::with_capacity(lines.len());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:739 [G3 PS3]: for line in &lines {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:740 [G3 PS3]: let fields: Vec<&str> = line.split(' ').collect();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:741 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:742 [G3 PS3]: journal.read_line(&fields).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:743 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:744 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:745 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:747 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:749 [G3 PS3]: let mut took = Vec::with_capacity(100_000);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:750 [G3 PS3]: for i in 0..100_000_u32 {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:751 [G3 PS3]: let key = format!("options 2024-05-{:02} N{i}", 1 + i % 28);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:752 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:753 [G3 PS3]: let got = journal.history(&key);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:754 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:755 [G3 PS3]: assert!(got.last_done.is_some());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:757 [G3 PS3]: println!("IA-2 r2 history probe, 100,000 keys: {}", pct(took));
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:759 [G3 PS3]: let store = scratch("r2-append");
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:760 [G3 PS3]: let ledger = store.join("imports").join("gdfl.lineage");
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:761 [G3 PS3]: std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:762 [G3 PS3]: let mut seed = String::with_capacity(16_000);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:763 [G3 PS3]: for i in 0..1_000 {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:764 [G3 PS3]: writeln!(seed, "alias seed{i}").unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:765 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:766 [G3 PS3]: std::fs::write(&ledger, seed).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:767 [G3 PS3]: let mut took = Vec::with_capacity(300);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:768 [G3 PS3]: for i in 0..300 {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:769 [G3 PS3]: let line = vec![format!("alias timed{i}")];
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:770 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:771 [G3 PS3]: append_lines(&ledger, &line).unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:772 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:773 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:774 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:776 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:780 [G3 PS3]: let lock = journal_lock_path(&ledger);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:781 [G3 PS3]: let mut took = Vec::with_capacity(1_000);
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:782 [G3 PS3]: for _ in 0..1_000 {
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:783 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:784 [G3 PS3]: crate::gdfl_run::locks::take_path_within(&lock, crate::gdfl_run::locks::LOCK_WAIT)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:785 [G3 PS3]: .map(drop)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:786 [G3 PS3]: .unwrap();
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:787 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:788 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:789 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:791 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_import_ia2_tests.rs:793 [G3 PS3]: }
U crates/pull/src/gdfl_import_journal.rs:1089:28 False [G3 PS3]: && let Some((before, _)) = self.begun_early.remove(&key)
F crates/pull/src/gdfl_import_ps1_tests.rs:170:9 False [G1 PS1]: let [staged] = extras.failures.as_slice() else {
F crates/pull/src/gdfl_import_ps1_tests.rs:171 [G1 PS1]: panic!("one failure, not {:?}", extras.failures);
U crates/pull/src/gdfl_import_quality.rs:790:24 False [G2 PS2]: if let Some(slot) = self.shared.get_mut(from) {
U crates/pull/src/gdfl_import_quality.rs:793:24 False [G2 PS2]: if let Some(slot) = self.shared.get_mut(to) {
F crates/pull/src/gdfl_import_scope.rs:327:17 False [G2 PS2]: assert!(none.members().is_empty() && none.set().is_empty());
F crates/pull/src/gdfl_import_scope.rs:327:46 False [G2 PS2]: assert!(none.members().is_empty() && none.set().is_empty());
F crates/pull/src/gdfl_import_scope.rs:374:17 False [G2 PS2]: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
F crates/pull/src/gdfl_import_scope.rs:374:57 False [G2 PS2]: assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
F crates/pull/src/gdfl_import_split_tests.rs:933:13 False [G3 PS3]: gone.is_empty() && new.is_empty(),
F crates/pull/src/gdfl_import_split_tests.rs:933:32 False [G3 PS3]: gone.is_empty() && new.is_empty(),
F crates/pull/src/gdfl_import_split_tests.rs:998 [G3 PS3]: panic!("the history is asked")
F crates/pull/src/gdfl_import_tests.rs:247:40 False [G1 PS1]: assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100));
F crates/pull/src/gdfl_import_tests.rs:380:9 False [G1 PS1]: texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
F crates/pull/src/gdfl_import_tests.rs:380:63 False [G1 PS1]: texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
F crates/pull/src/gdfl_import_tests.rs:382 [G1 PS1]: texts[8]
F crates/pull/src/gdfl_import_tests.rs:387 [G1 PS1]: texts[9]
F crates/pull/src/gdfl_import_tests.rs:400:13 False [G1 PS1]: assert!(all[0].report().is_none() && all[11].report().is_none());
F crates/pull/src/gdfl_import_tests.rs:400:42 False [G1 PS1]: assert!(all[0].report().is_none() && all[11].report().is_none());
F crates/pull/src/gdfl_import_tests.rs:550:9 False [G1 PS1]: let ImportRefusal::CensusFull {
F crates/pull/src/gdfl_import_tests.rs:558 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_tests.rs:562:13 False [G1 PS1]: assert!(!measured.fits() && measured.free() == 1);
F crates/pull/src/gdfl_import_tests.rs:562:33 False [G1 PS1]: assert!(!measured.fits() && measured.free() == 1);
F crates/pull/src/gdfl_import_tests.rs:616:19 False [G1 PS1]: )) && !l.contains(" stage=")
F crates/pull/src/gdfl_import_tests.rs:623:9 False [G1 PS1]: last.starts_with(&format!(
F crates/pull/src/gdfl_import_tests.rs:625:15 False [G1 PS1]: )) && last.contains(" failures=1 ")
F crates/pull/src/gdfl_import_tests.rs:626:16 False [G1 PS1]: && last.contains(" stage=census reason=too_many_entries run="),
F crates/pull/src/gdfl_import_tests.rs:646:9 False [G1 PS1]: journal.contains("\ndone options 2024-04-04 * ")
F crates/pull/src/gdfl_import_tests.rs:647:16 False [G1 PS1]: && journal.contains("\ndone options 2024-04-05 * "),
F crates/pull/src/gdfl_import_tests.rs:685:9 False [G1 PS1]: text.contains("refused stocks 2024-04-01 although the day's batch fits")
F crates/pull/src/gdfl_import_tests.rs:686:16 False [G1 PS1]: && text.contains("another ingest holds the census lock"),
F crates/pull/src/gdfl_import_tests.rs:730:13 False [G1 PS1]: assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
F crates/pull/src/gdfl_import_tests.rs:730:27 False [G1 PS1]: assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
F crates/pull/src/gdfl_import_tests.rs:1028:13 False [G1 PS1]: assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
F crates/pull/src/gdfl_import_tests.rs:1028:47 False [G1 PS1]: assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
F crates/pull/src/gdfl_import_tests.rs:1029:13 False [G1 PS1]: assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
F crates/pull/src/gdfl_import_tests.rs:1029:47 False [G1 PS1]: assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
F crates/pull/src/gdfl_import_tests.rs:1031:9 False [G1 PS1]: journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
F crates/pull/src/gdfl_import_tests.rs:1031:43 False [G1 PS1]: journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
F crates/pull/src/gdfl_import_tests.rs:1034:13 False [G1 PS1]: assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
F crates/pull/src/gdfl_import_tests.rs:1034:53 False [G1 PS1]: assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
F crates/pull/src/gdfl_import_tests.rs:1042:9 False [G1 PS1]: journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
F crates/pull/src/gdfl_import_tests.rs:1042:43 False [G1 PS1]: journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
F crates/pull/src/gdfl_import_tests.rs:1045:13 False [G1 PS1]: assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
F crates/pull/src/gdfl_import_tests.rs:1045:51 False [G1 PS1]: assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
F crates/pull/src/gdfl_import_tests.rs:1070 [G1 PS1]: matches!(
F crates/pull/src/gdfl_import_tests.rs:1364:13 False [G1 PS1]: assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
F crates/pull/src/gdfl_import_tests.rs:1364:38 False [G1 PS1]: assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
F crates/pull/src/gdfl_import_tests.rs:1455:9 False [G1 PS1]: damaged.digest.is_none()
F crates/pull/src/gdfl_import_tests.rs:1456:16 False [G1 PS1]: && damaged
F crates/pull/src/gdfl_import_tests.rs:1489:9 False [G1 PS1]: unformatted.digest.is_none()
F crates/pull/src/gdfl_import_tests.rs:1490:16 False [G1 PS1]: && unformatted
F crates/pull/src/gdfl_import_tests.rs:1632:9 False [G1 PS1]: second.contains(" source=zip ")
F crates/pull/src/gdfl_import_tests.rs:1633:16 False [G1 PS1]: && second.contains(" commit=unstamped ")
F crates/pull/src/gdfl_import_tests.rs:1634:16 False [G1 PS1]: && second.contains(" started=1750000000 ")
F crates/pull/src/gdfl_import_tests.rs:1635:16 False [G1 PS1]: && second.ends_with(&format!(
F crates/pull/src/gdfl_import_tests.rs:1656:9 False [G1 PS1]: first.starts_with("run ")
F crates/pull/src/gdfl_import_tests.rs:1657:16 False [G1 PS1]: && first.contains(
F crates/pull/src/gdfl_import_tests.rs:1704:9 False [G1 PS1]: line.starts_with("incomplete indices 2024-04-01 ")
F crates/pull/src/gdfl_import_tests.rs:1707:16 False [G1 PS1]: && line.contains(" stage=file reason=bars_differ_held run="),
F crates/pull/src/gdfl_import_tests.rs:1719:9 False [G1 PS1]: line.starts_with("incomplete options 2024-04-01 ")
F crates/pull/src/gdfl_import_tests.rs:1720:16 False [G1 PS1]: && line.contains(" stage=vendor reason="),
F crates/pull/src/gdfl_import_tests.rs:2010:13 False [G1 PS1]: ra.seconds > 0 && ra.seconds_committed == ra.seconds,
F crates/pull/src/gdfl_import_tests.rs:2010:31 False [G1 PS1]: ra.seconds > 0 && ra.seconds_committed == ra.seconds,
F crates/pull/src/gdfl_import_tests.rs:2022:13 False [G1 PS1]: assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
F crates/pull/src/gdfl_import_tests.rs:2022:41 False [G1 PS1]: assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
F crates/pull/src/gdfl_import_tests.rs:2037:22 False [G1 PS1]: .all(|l| l.contains(" member_bytes=0 ") && l.ends_with(" member_src=none")),
F crates/pull/src/gdfl_import_tests.rs:2042:22 False [G1 PS1]: .all(|l| !l.contains(" member_bytes=0 ") && l.ends_with(" member_src=zip")),
F crates/pull/src/gdfl_import_tests.rs:2074 [G1 PS1]: bin.display()
F crates/pull/src/gdfl_import_tests.rs:2083:22 False [G1 PS1]: .all(|b| b.volume == 0 && b.open_interest == OI_NULL)
F crates/pull/src/gdfl_import_tests.rs:2092:31 False [G1 PS1]: assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000));
F crates/pull/src/gdfl_import_tests.rs:2622:9 False [G1 PS1]: let ImportRefusal::CensusRefused {
F crates/pull/src/gdfl_import_tests.rs:2630 [G1 PS1]: panic!("{got:?}");
F crates/pull/src/gdfl_import_tests.rs:2636:34 False [G1 PS1]: .is_err_and(|unread| unread.contains("gdfl.man") && unread.contains("not a regular")),
F crates/pull/src/gdfl_import_tests.rs:2644:22 False [G1 PS1]: .any(|f| f.instrument == "2024-04-01 census" && f.why.contains("nothing of the batch")),
F crates/pull/src/gdfl_import_tests.rs:2650:9 False [G1 PS1]: text.contains("indices 2024-04-01")
F crates/pull/src/gdfl_import_tests.rs:2651:16 False [G1 PS1]: && text.contains("its room could not be measured")
F crates/pull/src/gdfl_import_tests.rs:2652:16 False [G1 PS1]: && text.contains("the run stopped there after 0 day(s) imported"),
F crates/pull/src/gdfl_import_tests.rs:2659:9 False [G1 PS1]: last.starts_with(&format!(
F crates/pull/src/gdfl_import_tests.rs:2661:15 False [G1 PS1]: )) && last.contains(" stage=census reason=census_refused run="),
F crates/pull/src/gdfl_import_tests.rs:2913:74 False [G1 PS1]: matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder"))
F crates/pull/src/gdfl_import_tests.rs:4356:20 False [G1 PS1]: || crate::gdfl_lineage::EventRecord::parse(&line).is_some(),
F crates/pull/src/gdfl_import_tests.rs:4439:17 False [G1 PS1]: let [_, _, _, _, under, contract, ..] = parts.as_slice() else {
F crates/pull/src/gdfl_import_tests.rs:4440 [G1 PS1]: panic!("{rel}: not a bar month of the store");
F crates/pull/src/gdfl_import_tests.rs:4683:17 False [G1 PS1]: assert!(one.provenance_records > 0 && one.seconds_committed > 0);
F crates/pull/src/gdfl_import_tests.rs:4683:47 False [G1 PS1]: assert!(one.provenance_records > 0 && one.seconds_committed > 0);
F crates/pull/src/gdfl_import_tests.rs:4707 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:4896:9 False [G1 PS1]: indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
F crates/pull/src/gdfl_import_tests.rs:4896:36 False [G1 PS1]: indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
F crates/pull/src/gdfl_import_tests.rs:4898 [G1 PS1]: indices[0].0
F crates/pull/src/gdfl_import_tests.rs:4944:21 False [G1 PS1]: u64::try_from(bytes.len()).unwrap() <= store::format::HEADER_LEN
F crates/pull/src/gdfl_import_tests.rs:4945:28 False [G1 PS1]: && rest.iter().all(|&byte| byte == 0)
F crates/pull/src/gdfl_import_tests.rs:4946:28 False [G1 PS1]: && store::header::Header::decode(slot).is_err(),
F crates/pull/src/gdfl_import_tests.rs:4948 [G1 PS1]: bytes.len()
F crates/pull/src/gdfl_import_tests.rs:5013 [G1 PSA]: month.display()
F crates/pull/src/gdfl_import_tests.rs:5171:17 False [G1 PSA]: !status.success() && open_day && months == 0 && unborn == 1,
F crates/pull/src/gdfl_import_tests.rs:5171:38 False [G1 PSA]: !status.success() && open_day && months == 0 && unborn == 1,
F crates/pull/src/gdfl_import_tests.rs:5171:50 False [G1 PSA]: !status.success() && open_day && months == 0 && unborn == 1,
F crates/pull/src/gdfl_import_tests.rs:5171:65 False [G1 PSA]: !status.success() && open_day && months == 0 && unborn == 1,
F crates/pull/src/gdfl_import_tests.rs:5176:17 False [G1 PS1]: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5176:38 False [G1 PS1]: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5176:60 False [G1 PS1]: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5176:75 False [G1 PS1]: !status.success() && journal.is_empty() && months == 0 && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5181:17 False [G1 PS1]: status.success() && !open_day && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5181:37 False [G1 PS1]: status.success() && !open_day && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5181:50 False [G1 PS1]: status.success() && !open_day && unborn == 0,
F crates/pull/src/gdfl_import_tests.rs:5188:28 False [G1 PS1]: unborn == 0 || open_day,
F crates/pull/src/gdfl_import_tests.rs:5234 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:5422:13 False [G1 PS1]: assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
F crates/pull/src/gdfl_import_tests.rs:5422:50 False [G1 PS1]: assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
F crates/pull/src/gdfl_import_tests.rs:5430 [G1 PS1]: durability.as_str()
F crates/pull/src/gdfl_import_tests.rs:5436 [G1 PS1]: durability.as_str()
F crates/pull/src/gdfl_import_tests.rs:5442 [G1 PS1]: durability.as_str(),
F crates/pull/src/gdfl_import_tests.rs:5443 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:5562:9 False [G1 PS1]: let Err(ImportRefusal::Durability {
F crates/pull/src/gdfl_import_tests.rs:5569 [G1 PS1]: panic!("the run stopped at the day's flush: {refused:?}");
F crates/pull/src/gdfl_import_tests.rs:5573:9 False [G1 PS1]: why.contains("syncing") && why.contains(&store.display().to_string()),
F crates/pull/src/gdfl_import_tests.rs:5573:36 False [G1 PS1]: why.contains("syncing") && why.contains(&store.display().to_string()),
F crates/pull/src/gdfl_import_tests.rs:5723 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:5733 [G1 PS1]: one.display()
F crates/pull/src/gdfl_import_tests.rs:5868:13 False [G1 PS1]: last.starts_with(&format!("incomplete options {}", d2()))
F crates/pull/src/gdfl_import_tests.rs:5869:20 False [G1 PS1]: && last.contains(&format!(" failures={} ", refused + 1)),
F crates/pull/src/gdfl_import_tests.rs:5885 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:5956:13 False [G1 PS1]: assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
F crates/pull/src/gdfl_import_tests.rs:5956:41 False [G1 PS1]: assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
F crates/pull/src/gdfl_import_tests.rs:6024 [G1 PS1]: Err(std::io::Error::other(format!(
F crates/pull/src/gdfl_import_tests.rs:6025 [G1 PS1]: "{word}: the back-fill day's batch flushed {}",
F crates/pull/src/gdfl_import_tests.rs:6026 [G1 PS1]: path.display()
F crates/pull/src/gdfl_import_tests.rs:6027 [G1 PS1]: )))
F crates/pull/src/gdfl_import_tests.rs:6028 [G1 PS1]: }),
F crates/pull/src/gdfl_import_tests.rs:6084 [G1 PS1]: rel.display()
F crates/pull/src/gdfl_import_tests.rs:6229:46 False [G1 PS1]: why.contains("holds") || why.contains("lock"),
F crates/pull/src/gdfl_import_tests.rs:6436:9 False [G1 PS1]: let Err(ImportRefusal::CensusRefused { day: at, why, .. }) = &got else {
F crates/pull/src/gdfl_import_tests.rs:6437 [G1 PS1]: panic!("the run is refused at the census: {got:?}")
F crates/pull/src/gdfl_lineage_tests.rs:1092:9 False [G3 PS3]: let Err(Stale::AfterChange(change)) = filed("NIITTECH", d(2020, 8, 20)) else {
F crates/pull/src/gdfl_lineage_tests.rs:1093 [G3 PS3]: panic!("an old name on its change day");
F crates/pull/src/gdfl_lineage_tests.rs:1135:9 False [G3 PS3]: said.contains("NIITTECH27AUG201900CE")
F crates/pull/src/gdfl_lineage_tests.rs:1136:16 False [G3 PS3]: && said.contains("NIITTECH became COFORGE on 2020-08-20"),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1148 [G3 PS3]: fn the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1149 [G3 PS3]: let strikes: Vec<i64> = (0..50).map(|i| 1_000_000 + i * 5_000).collect();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1150 [G3 PS3]: let mut day = Vec::new();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1151 [G3 PS3]: for expiry in [
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1152 [G3 PS3]: d(2020, 9, 17),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1153 [G3 PS3]: d(2020, 9, 24),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1154 [G3 PS3]: d(2020, 10, 1),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1155 [G3 PS3]: d(2020, 10, 29),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1156 [G3 PS3]: ] {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1157 [G3 PS3]: day.extend(ladder(expiry, &strikes, 75));
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1158 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1159 [G3 PS3]: assert_eq!(day.len(), 400);
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1160 [G3 PS3]: let mut nanos: Vec<u128> = (0..2_000)
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1161 [G3 PS3]: .map(|_| {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1162 [G3 PS3]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1163 [G3 PS3]: let got = detect(sym("NIFTY"), d(2020, 9, 16), &day, &day);
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1164 [G3 PS3]: let took = at.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1165 [G3 PS3]: assert_eq!(got, None);
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1166 [G3 PS3]: took
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1167 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1168 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1169 [G3 PS3]: nanos.sort_unstable();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1170 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1172 [G3 PS3]: nanos[1_000], nanos[1_980], nanos[1_999]
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1174 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1216 [G3 PS3]: fn the_real_names_are_one_key_per_contract_across_renames_and_redatings() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1218 [G3 PS3]: let path = std::env::var_os("BRUTEX_NAME_CENSUS").expect("BRUTEX_NAME_CENSUS");
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1219 [G3 PS3]: let text = std::fs::read_to_string(path).unwrap();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1221 [G3 PS3]: let mut keys: HashMap<String, BTreeMap<String, (Day, Day)>> = HashMap::new();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1222 [G3 PS3]: let (mut names, mut decoded, mut stale, mut aliased) = (0_u32, 0_u32, Vec::new(), 0_u32);
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1223 [G3 PS3]: let mut refused: BTreeMap<String, u32> = BTreeMap::new();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1224 [G3 PS3]: for line in text.lines() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1225 [G3 PS3]: let mut fields = line.split('\t');
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1226 [G3 PS3]: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1226:13 False [G3 PS3]: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1226:13 True [G3 PS3]: let (Some(name), Some(first), Some(last)) = (fields.next(), fields.next(), fields.next())
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1228 [G3 PS3]: continue;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1230 [G3 PS3]: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1230:13 False [G3 PS3]: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1230:13 True [G3 PS3]: let (Some(first), Some(last)) = (parse_day(first), parse_day(last)) else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1231 [G3 PS3]: continue;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1233 [G3 PS3]: names += 1;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1234 [G3 PS3]: let mut both = Vec::new();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1235 [G3 PS3]: for on in [first, last] {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1236 [G3 PS3]: match crate::gdfl_nfo::decode_ticker(name, on) {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1237 [G3 PS3]: Ok(ticker) => both.push(ticker),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1238 [G3 PS3]: Err(crate::gdfl_nfo::NfoRefusal::UnderlyingStale { .. }) => {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1239 [G3 PS3]: stale.push(format!("{name} on {on}"));
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1240 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1241 [G3 PS3]: Err(why) => {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1242 [G3 PS3]: let word = format!("{why:?}");
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1243 [G3 PS3]: let word = word.split([' ', '{']).next().unwrap_or_default().to_owned();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1244 [G3 PS3]: *refused.entry(word).or_default() += 1;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1245 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1248 [G3 PS3]: let [a, b] = both.as_slice() else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1248:13 False [G3 PS3]: let [a, b] = both.as_slice() else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1248:13 True [G3 PS3]: let [a, b] = both.as_slice() else {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1249 [G3 PS3]: continue;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1251 [G3 PS3]: decoded += 1;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1252 [G3 PS3]: assert_eq!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1253 [G3 PS3]: (a.underlying, a.contract),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1254 [G3 PS3]: (b.underlying, b.contract),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1257 [G3 PS3]: aliased += u32::from(matches!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1258 [G3 PS3]: a.settlement,
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1261 [G3 PS3]: let stem_len = name.len() - 2;
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1262 [G3 PS3]: let stem: String = name[..stem_len]
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1263 [G3 PS3]: .chars()
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1264 [G3 PS3]: .take_while(|c| !c.is_ascii_digit())
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1265 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1266 [G3 PS3]: let slot = keys
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1267 [G3 PS3]: .entry(option_key(a.underlying, a.contract))
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1268 [G3 PS3]: .or_default()
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1269 [G3 PS3]: .entry(stem)
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1270 [G3 PS3]: .or_insert((first, last));
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1271 [G3 PS3]: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1271:21 False [G3 PS3]: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1271:21 True [G3 PS3]: slot.0 = if first.days_from_epoch() < slot.0.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1272 [G3 PS3]: first
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1274 [G3 PS3]: slot.0
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1276 [G3 PS3]: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1276:21 False [G3 PS3]: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1276:21 True [G3 PS3]: slot.1 = if last.days_from_epoch() > slot.1.days_from_epoch() {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1277 [G3 PS3]: last
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1279 [G3 PS3]: slot.1
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1282 [G3 PS3]: let joined = joined_across_renames(&keys);
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1283 [G3 PS3]: println!("names {names}, decoded on both days {decoded}, aliased names {aliased}");
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1284 [G3 PS3]: println!("refused (first or last day), by kind: {refused:?}");
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1285 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1287 [G3 PS3]: joined.values().sum::<u32>(),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1288 [G3 PS3]: joined.len()
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1290 [G3 PS3]: for (label, n) in &joined {
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1291 [G3 PS3]: println!("  {label}: {n}");
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1292 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1293 [G3 PS3]: assert!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1294 [G3 PS3]: stale.is_empty(),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1296 [G3 PS3]: stale.len(),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1297 [G3 PS3]: stale.first()
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1299 [G3 PS3]: assert!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1300 [G3 PS3]: joined.values().sum::<u32>() > 0,
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1303 [G3 PS3]: assert_eq!(
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1305 [G3 PS3]: crate::expiry_alias::ALIASES
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1306 [G3 PS3]: .iter()
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1307 [G3 PS3]: .map(|row| row.contracts)
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1308 [G3 PS3]: .sum::<u32>(),
IGNORED crates/pull/src/gdfl_lineage_tests.rs:1311 [G3 PS3]: }
F crates/pull/src/gdfl_nfo_attack_tests.rs:301:27 False [G3 PS3]: } else if is_index(got.underlying.as_str())
F crates/pull/src/gdfl_nfo_attack_tests.rs:302:24 False [G3 PS3]: && on >= d(2018, 9, 3)
F crates/pull/src/gdfl_nfo_attack_tests.rs:303:24 False [G3 PS3]: && got.form == TickerForm::Day
F crates/pull/src/gdfl_nfo_attack_tests.rs:304:24 False [G3 PS3]: && reencodes(&ticker, &got)
F crates/pull/src/gdfl_nfo_attack_tests.rs:312 [G3 PS3]: misread.push(format!("{ticker} on {on} -> {}", got.contract.as_str()));
F crates/pull/src/gdfl_nfo_attack_tests.rs:313 [G3 PS3]: }
F crates/pull/src/gdfl_nfo_attack_tests.rs:318 [G3 PS3]: NfoRefusal::MonthlyExpiryUnmeasured { .. } => unstated += 1,
F crates/pull/src/gdfl_nfo_attack_tests.rs:322 [G3 PS3]: NfoRefusal::TickerAmbiguous { .. } => ambiguous += 1,
F crates/pull/src/gdfl_nfo_attack_tests.rs:328 [G3 PS3]: why => panic!("{ticker} on {on}: {why:?}"),
F crates/pull/src/gdfl_nfo_attack_tests.rs:331 [G3 PS3]: && !matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:331:24 True [G3 PS3]: && !matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:335 [G3 PS3]: {
F crates/pull/src/gdfl_nfo_attack_tests.rs:336 [G3 PS3]: missed.push(format!("{ticker} on {on}: {why:?}"));
F crates/pull/src/gdfl_nfo_attack_tests.rs:352 [G3 PS3]: misread.len(),
F crates/pull/src/gdfl_nfo_attack_tests.rs:353 [G3 PS3]: &misread[..8.min(misread.len())]
F crates/pull/src/gdfl_nfo_attack_tests.rs:358 [G3 PS3]: missed.len(),
F crates/pull/src/gdfl_nfo_attack_tests.rs:359 [G3 PS3]: &missed[..8.min(missed.len())]
F crates/pull/src/gdfl_nfo_attack_tests.rs:597 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:694 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:719:9 False [G3 PS3]: rekeyed > 1_000 && unsettled > 1_000,
F crates/pull/src/gdfl_nfo_attack_tests.rs:719:28 False [G3 PS3]: rekeyed > 1_000 && unsettled > 1_000,
F crates/pull/src/gdfl_nfo_attack_tests.rs:795 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:855 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:997 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:1099 [G3 PS3]: got.contract.as_str(),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1100 [G3 PS3]: reencode(&got)
F crates/pull/src/gdfl_nfo_attack_tests.rs:1114:21 False [G3 PS3]: expiry >= on
F crates/pull/src/gdfl_nfo_attack_tests.rs:1115:28 False [G3 PS3]: && expiry.days_from_epoch() <= on.days_from_epoch() + EXPIRY_HORIZON_DAYS
F crates/pull/src/gdfl_nfo_attack_tests.rs:1136:25 False [G3 PS3]: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
F crates/pull/src/gdfl_nfo_attack_tests.rs:1136:70 False [G3 PS3]: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
F crates/pull/src/gdfl_nfo_attack_tests.rs:1136:70 True [G3 PS3]: assert!(why.to_string().contains(ticker.as_str()) || ticker.is_empty());
F crates/pull/src/gdfl_nfo_attack_tests.rs:1429:29 False [G3 PS3]: assert!(row.ltp >= 0 && row.oi >= 0);
F crates/pull/src/gdfl_nfo_attack_tests.rs:1429:45 False [G3 PS3]: assert!(row.ltp >= 0 && row.oi >= 0);
F crates/pull/src/gdfl_nfo_attack_tests.rs:1430:45 False [G3 PS3]: assert!(row.ltq == 0 || row.ltp >= costs::rate::TICK.raw());
F crates/pull/src/gdfl_nfo_attack_tests.rs:1438 [G3 PS3]: NfoRefusal::HeaderUnknown { .. } => panic!("the header was intact"),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1445:33 False [G3 PS3]: assert!(at >= 2 && at <= n + 2, "line {line} of {n} rows");
F crates/pull/src/gdfl_nfo_attack_tests.rs:1445:44 False [G3 PS3]: assert!(at >= 2 && at <= n + 2, "line {line} of {n} rows");
F crates/pull/src/gdfl_nfo_attack_tests.rs:1454 [G3 PS3]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1459:13 False [G3 PS3]: assert!(ok > 1_000 && refused > 1_000, "{ok} {refused}");
F crates/pull/src/gdfl_nfo_attack_tests.rs:1459:27 False [G3 PS3]: assert!(ok > 1_000 && refused > 1_000, "{ok} {refused}");
F crates/pull/src/gdfl_nfo_attack_tests.rs:1571:17 False [G3 PS3]: !ticker.is_empty() && !ticker.contains(['\\', '/']),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1571:39 False [G3 PS3]: !ticker.is_empty() && !ticker.contains(['\\', '/']),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1577:24 False [G3 PS3]: || entry.ends_with(&format!("{ticker}.NFO.CSV")),
F crates/pull/src/gdfl_nfo_attack_tests.rs:1797 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:1805 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:1987 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_attack_tests.rs:2006 [G3 PS3]: matches!(
F crates/pull/src/gdfl_nfo_tests.rs:295 [G1 PS1]: matches!(
F crates/pull/src/gdfl_nfo_tests.rs:314 [G1 PS1]: fn day(&self, _: Day) -> Result<Option<NfoDay<u8>>, CmRefusal> {
F crates/pull/src/gdfl_nfo_tests.rs:315 [G1 PS1]: unreachable!("read_file is handed its listing: it never asks the source for a day")
F crates/pull/src/gdfl_nfo_tests.rs:352:13 False [G1 PS1]: let NfoRefusal::StrikeImplausible {
F crates/pull/src/gdfl_nfo_tests.rs:358 [G1 PS1]: panic!("{name}: {got:?}")
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1244 [G1 PS1]: fn the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1245 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1246 [G1 PS1]: let bound = |var: &str, fallback: Day| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1247 [G1 PS1]: std::env::var(var).ok().map_or(fallback, |text| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1248 [G1 PS1]: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1249 [G1 PS1]: day(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1250 [G1 PS1]: parts[0],
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1251 [G1 PS1]: u8::try_from(parts[1]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1252 [G1 PS1]: u8::try_from(parts[2]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1254 [G1 PS1]: })
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1255 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1256 [G1 PS1]: let (from, to) = (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1257 [G1 PS1]: bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1258 [G1 PS1]: bound("BRUTEX_WP12B_TO", day(2026, 9, 24)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1259 [G1 PS1]: );
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1260 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1261 [G1 PS1]: let (mut days, mut names, mut rekeyed) = (0_u64, 0_u64, 0_u64);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1262 [G1 PS1]: let mut rekeyed_names: HashSet<String> = HashSet::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1263 [G1 PS1]: let mut unsettled: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1264 [G1 PS1]: let mut implausible: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1267 [G1 PS1]: let mut collisions: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1268 [G1 PS1]: let mut by_rule: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1271 [G1 PS1]: let mut high: Vec<(i64, Day, String)> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1272 [G1 PS1]: let mut low: Vec<(i64, Day, String)> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1273 [G1 PS1]: let mut on = from;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1274 [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1274:11 False [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1274:11 True [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1275 [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1275:16 False [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1275:16 True [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1276 [G1 PS1]: days += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1277 [G1 PS1]: let mut seen: HashMap<(String, String), String> = HashMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1278 [G1 PS1]: let mut ladders: HashMap<String, (i64, i64)> = HashMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1279 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1280 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1280:21 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1280:21 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1281 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1283 [G1 PS1]: names += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1284 [G1 PS1]: match decode_ticker(name, on) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1285 [G1 PS1]: Ok(ticker) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1286 [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1286:28 False [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1286:28 True [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1287 [G1 PS1]: rekeyed += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1288 [G1 PS1]: rekeyed_names.insert(name.to_owned());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1289 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1290 [G1 PS1]: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1290:28 False [G1 PS1]: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1290:28 True [G1 PS1]: if ticker.settlement == crate::expiry_alias::Settlement::ClosedDayRule {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1291 [G1 PS1]: *by_rule
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1292 [G1 PS1]: .entry(format!("{} {}", ticker.underlying.as_str(), ticker.stated))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1293 [G1 PS1]: .or_default() += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1294 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1295 [G1 PS1]: let key = (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1296 [G1 PS1]: ticker.underlying.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1297 [G1 PS1]: ticker.contract.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1298 [G1 PS1]: );
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1299 [G1 PS1]: if let Some(first) = seen.insert(key, name.to_owned()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1299:32 False [G1 PS1]: if let Some(first) = seen.insert(key, name.to_owned()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1299:32 True [G1 PS1]: if let Some(first) = seen.insert(key, name.to_owned()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1300 [G1 PS1]: collisions.push(format!("{on} {first} {name}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1301 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1302 [G1 PS1]: if listing.check_strike(&ticker, name).is_err() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1302:28 False [G1 PS1]: if listing.check_strike(&ticker, name).is_err() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1302:28 True [G1 PS1]: if listing.check_strike(&ticker, name).is_err() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1303 [G1 PS1]: implausible.push(format!("{on} {name}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1304 [G1 PS1]: } else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1305 [G1 PS1]: let span = ladders
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1306 [G1 PS1]: .entry(ticker.underlying.as_str().to_owned())
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1307 [G1 PS1]: .or_insert((ticker.strike, ticker.strike));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1308 [G1 PS1]: span.0 = span.0.min(ticker.strike);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1309 [G1 PS1]: span.1 = span.1.max(ticker.strike);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1310 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1312 [G1 PS1]: Err(NfoRefusal::ExpiryUnsettled { why, .. }) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1313 [G1 PS1]: unsettled.push(format!("{on} {name}: {why}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1314 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1315 [G1 PS1]: Err(_) => {}
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1318 [G1 PS1]: for (under, (lo, hi)) in ladders {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1319 [G1 PS1]: let median = listing.strike_median(&under).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1320 [G1 PS1]: high.push((hi * 100 / median, on, under.clone()));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1321 [G1 PS1]: if lo > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1321:20 False [G1 PS1]: if lo > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1321:20 True [G1 PS1]: if lo > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1322 [G1 PS1]: low.push((median * 100 / lo, on, under));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1323 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1325 [G1 PS1]: high.sort_unstable_by(|a, b| b.cmp(a));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1326 [G1 PS1]: high.truncate(3);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1327 [G1 PS1]: low.sort_unstable_by(|a, b| b.cmp(a));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1328 [G1 PS1]: low.truncate(3);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1329 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1330 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1332 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1334 [G1 PS1]: rekeyed_names.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1335 [G1 PS1]: by_rule.values().sum::<u64>(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1336 [G1 PS1]: unsettled.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1337 [G1 PS1]: collisions.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1338 [G1 PS1]: implausible.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1339 [G1 PS1]: &unsettled[..unsettled.len().min(20)],
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1340 [G1 PS1]: &collisions[..collisions.len().min(20)]
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1342 [G1 PS1]: assert!(days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1343 [G1 PS1]: assert!(unsettled.is_empty(), "{unsettled:?}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1344 [G1 PS1]: assert!(collisions.is_empty(), "{collisions:?}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1347 [G1 PS1]: assert!(by_rule.is_empty(), "{by_rule:?}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1349 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1350 [G1 PS1]: high.iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1351 [G1 PS1]: .all(|(ratio, ..)| *ratio < STRIKE_PLAUSIBLE_MULTIPLE * 100),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1354 [G1 PS1]: let impossible = [
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1355 [G1 PS1]: "2019-05-30 NIFTY25JUN201912500CE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1356 [G1 PS1]: "2019-05-30 NIFTY25JUN201912700CE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1357 [G1 PS1]: "2019-05-30 NIFTY25JUN201913000CE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1358 [G1 PS1]: ];
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1359 [G1 PS1]: if (from..=to).contains(&day(2019, 5, 30)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1359:8 False [G1 PS1]: if (from..=to).contains(&day(2019, 5, 30)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1359:8 True [G1 PS1]: if (from..=to).contains(&day(2019, 5, 30)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1360 [G1 PS1]: implausible.sort();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1361 [G1 PS1]: assert_eq!(implausible, impossible);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1363 [G1 PS1]: assert!(implausible.is_empty(), "{implausible:?}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1365 [G1 PS1]: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1365:8 False [G1 PS1]: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1365:8 True [G1 PS1]: if (from, to) == (day(2018, 9, 3), day(2026, 9, 24)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1366 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1367 [G1 PS1]: u32::try_from(rekeyed_names.len()).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1368 [G1 PS1]: crate::expiry_alias::ALIASES
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1369 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1370 [G1 PS1]: .map(|row| row.contracts)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1371 [G1 PS1]: .sum::<u32>(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1374 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1375 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1387 [G1 PS1]: fn the_real_names_decode_to_the_stored_contracts_less_the_refused_and_the_resettled() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1388 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1389 [G1 PS1]: let store =
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1390 [G1 PS1]: std::path::PathBuf::from(std::env::var_os("BRUTEX_STORE_FNO").expect("BRUTEX_STORE_FNO"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1391 [G1 PS1]: let unders = std::env::var("BRUTEX_WP12B_UNDERLYINGS").unwrap_or_else(|_| "NIFTY".to_owned());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1392 [G1 PS1]: let bound = |var: &str, fallback: Day| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1393 [G1 PS1]: std::env::var(var).ok().map_or(fallback, |text| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1394 [G1 PS1]: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1395 [G1 PS1]: day(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1396 [G1 PS1]: parts[0],
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1397 [G1 PS1]: u8::try_from(parts[1]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1398 [G1 PS1]: u8::try_from(parts[2]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1400 [G1 PS1]: })
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1401 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1402 [G1 PS1]: let (from, to) = (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1403 [G1 PS1]: bound("BRUTEX_WP12B_FROM", day(2018, 9, 3)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1404 [G1 PS1]: bound("BRUTEX_WP12B_TO", day(2020, 9, 15)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1405 [G1 PS1]: );
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1406 [G1 PS1]: let wanted: HashSet<&str> = unders.split(',').collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1407 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1408 [G1 PS1]: let mut decoded: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1409 [G1 PS1]: let mut refused: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1410 [G1 PS1]: let mut refused_contracts: HashSet<(String, String)> = HashSet::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1412 [G1 PS1]: let mut resettled: std::collections::BTreeMap<(String, String), (String, String, &str)> =
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1413 [G1 PS1]: std::collections::BTreeMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1414 [G1 PS1]: let mut on = from;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1415 [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1415:11 False [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1415:11 True [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1416 [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1416:16 False [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1416:16 True [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1417 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1418 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1418:21 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1418:21 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1419 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1421 [G1 PS1]: match decode_ticker(name, on) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1422 [G1 PS1]: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1422:35 False [G1 PS1]: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1422:35 True [G1 PS1]: Ok(ticker) if wanted.contains(ticker.underlying.as_str()) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1423 [G1 PS1]: if let Err(why) = listing.check_strike(&ticker, name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1423:32 False [G1 PS1]: if let Err(why) = listing.check_strike(&ticker, name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1423:32 True [G1 PS1]: if let Err(why) = listing.check_strike(&ticker, name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1424 [G1 PS1]: refused.push(format!("{on} {why}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1425 [G1 PS1]: refused_contracts.insert((
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1426 [G1 PS1]: ticker.underlying.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1427 [G1 PS1]: ticker.contract.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1428 [G1 PS1]: ));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1429 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1430 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1431 [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1431:28 False [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1431:28 True [G1 PS1]: if ticker.expiry != ticker.stated {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1432 [G1 PS1]: let stated = crate::gdfl_lineage::ContractDay {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1433 [G1 PS1]: expiry: ticker.stated,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1434 [G1 PS1]: strike: ticker.strike,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1435 [G1 PS1]: side: ticker.side,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1436 [G1 PS1]: lot: crate::gdfl_lineage::LotEvidence::NoTrade,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1437 [G1 PS1]: first_oi: None,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1438 [G1 PS1]: close_oi: None,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1439 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1440 [G1 PS1]: .contract()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1441 [G1 PS1]: .unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1442 [G1 PS1]: resettled.insert(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1443 [G1 PS1]: (ticker.underlying.as_str().to_owned(), name.to_owned()),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1444 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1445 [G1 PS1]: stated.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1446 [G1 PS1]: ticker.contract.as_str().to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1447 [G1 PS1]: ticker.settlement.as_str(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1448 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1449 [G1 PS1]: );
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1450 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1451 [G1 PS1]: decoded
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1452 [G1 PS1]: .entry(ticker.underlying.as_str().to_owned())
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1453 [G1 PS1]: .or_default()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1454 [G1 PS1]: .insert(ticker.contract.as_str().to_owned());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1456 [G1 PS1]: Ok(_) => {}
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1457 [G1 PS1]: Err(why) => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1458 [G1 PS1]: if wanted.iter().any(|u| name.starts_with(u)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1458:28 False [G1 PS1]: if wanted.iter().any(|u| name.starts_with(u)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1458:28 True [G1 PS1]: if wanted.iter().any(|u| name.starts_with(u)) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1459 [G1 PS1]: refused.push(format!("{on} {name}: {why}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1460 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1464 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1465 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1467 [G1 PS1]: for under in &wanted {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1468 [G1 PS1]: let held: std::collections::BTreeSet<String> = std::fs::read_dir(store.join(under))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1469 [G1 PS1]: .unwrap()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1470 [G1 PS1]: .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1471 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1472 [G1 PS1]: let mine = decoded.remove(*under).unwrap_or_default();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1473 [G1 PS1]: let only_held: Vec<&String> = held.difference(&mine).collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1474 [G1 PS1]: let only_mine: Vec<&String> = mine.difference(&held).collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1475 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1477 [G1 PS1]: mine.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1478 [G1 PS1]: held.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1479 [G1 PS1]: only_held.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1480 [G1 PS1]: only_mine.len()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1488 [G1 PS1]: let closed = |seg: &str| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1489 [G1 PS1]: let day = crate::gdfl_lineage::parse_day(&seg[..10]).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1490 [G1 PS1]: crate::calendar::sourced_kind_of(i64::from(day.days_from_epoch()))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1491 [G1 PS1]: == crate::calendar::DayKind::Closed
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1492 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1493 [G1 PS1]: let mut changed = 0_usize;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1494 [G1 PS1]: for ((_, name), (stated, settled, _)) in resettled.iter().filter(|((u, _), _)| u == under) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1495 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1496 [G1 PS1]: held.contains(stated),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1499 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1500 [G1 PS1]: mine.contains(settled) && !held.contains(settled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1500:17 False [G1 PS1]: mine.contains(settled) && !held.contains(settled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1500:17 True [G1 PS1]: mine.contains(settled) && !held.contains(settled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1500:43 False [G1 PS1]: mine.contains(settled) && !held.contains(settled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1500:43 True [G1 PS1]: mine.contains(settled) && !held.contains(settled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1503 [G1 PS1]: changed += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1505 [G1 PS1]: for (u, seg) in refused_contracts.iter().filter(|(u, _)| u == under) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1506 [G1 PS1]: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1506:21 False [G1 PS1]: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1506:21 True [G1 PS1]: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1506:43 False [G1 PS1]: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1506:43 True [G1 PS1]: assert!(held.contains(seg) && !mine.contains(seg), "{u}: {seg}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1507 [G1 PS1]: changed += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1509 [G1 PS1]: let other_held: Vec<&&String> = only_held
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1510 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1511 [G1 PS1]: .filter(|seg| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1512 [G1 PS1]: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1512:17 False [G1 PS1]: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1512:17 True [G1 PS1]: !closed(seg) && !refused_contracts.contains(&((*under).to_owned(), (**seg).clone()))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1513 [G1 PS1]: })
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1514 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1515 [G1 PS1]: let other_mine: Vec<&&String> = only_mine
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1516 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1517 [G1 PS1]: .filter(|seg| !resettled.values().any(|(_, settled, _)| settled == **seg))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1518 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1519 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1523 [G1 PS1]: eprintln!("refused {}: {refused:?}", refused.len());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1524 [G1 PS1]: eprintln!("resettled {}: {resettled:?}", resettled.len());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1525 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1533 [G1 PS1]: fn the_real_ex_dates_are_detected_from_the_data_and_ordinary_days_are_not() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1535 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1536 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1538 [G1 PS1]: let day_of = |under: &str, on: Day| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1539 [G1 PS1]: let mut out: Vec<(ContractDay, Vec<(u32, i64)>)> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1540 [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1540:13 False [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1540:13 True [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1541 [G1 PS1]: return out;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1543 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1544 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1544:17 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1544:17 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1545 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1547 [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1547:17 False [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1547:17 True [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1548 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1550 [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1550:16 False [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1550:16 True [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1551 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1552 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1553 [G1 PS1]: let read = read_file(&zips, &listing, file).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1554 [G1 PS1]: let rows: Vec<(u32, u64, i64)> = read
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1555 [G1 PS1]: .file
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1556 [G1 PS1]: .rows
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1557 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1558 [G1 PS1]: .map(|r| (r.sod, r.ltq, r.oi))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1559 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1560 [G1 PS1]: out.push((
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1561 [G1 PS1]: ContractDay::from_rows(&read.ticker, rows.iter().copied()),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1562 [G1 PS1]: rows.iter().map(|&(sod, _, oi)| (sod, oi)).collect(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1565 [G1 PS1]: out
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1566 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1569 [G1 PS1]: let cases = [
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1570 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1571 [G1 PS1]: "BRITANNIA",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1572 [G1 PS1]: day(2018, 11, 28),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1573 [G1 PS1]: day(2018, 11, 29),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1574 [G1 PS1]: day(2018, 11, 30),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1575 [G1 PS1]: (2, 1),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1576 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1577 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1578 [G1 PS1]: "NTPC",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1579 [G1 PS1]: day(2019, 3, 18),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1580 [G1 PS1]: day(2019, 3, 19),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1581 [G1 PS1]: day(2019, 3, 20),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1582 [G1 PS1]: (6, 5),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1583 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1584 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1585 [G1 PS1]: "HCLTECH",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1586 [G1 PS1]: day(2019, 12, 4),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1587 [G1 PS1]: day(2019, 12, 5),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1588 [G1 PS1]: day(2019, 12, 6),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1589 [G1 PS1]: (2, 1),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1590 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1591 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1592 [G1 PS1]: "BAJAJFINSV",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1593 [G1 PS1]: day(2022, 9, 12),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1594 [G1 PS1]: day(2022, 9, 13),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1595 [G1 PS1]: day(2022, 9, 14),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1596 [G1 PS1]: (10, 1),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1597 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1598 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1599 [G1 PS1]: "RELIANCE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1600 [G1 PS1]: day(2024, 10, 25),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1601 [G1 PS1]: day(2024, 10, 28),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1602 [G1 PS1]: day(2024, 10, 29),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1603 [G1 PS1]: (2, 1),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1604 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1605 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1606 [G1 PS1]: "BAJFINANCE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1607 [G1 PS1]: day(2025, 6, 13),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1608 [G1 PS1]: day(2025, 6, 16),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1609 [G1 PS1]: day(2025, 6, 17),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1610 [G1 PS1]: (10, 1),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1611 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1612 [G1 PS1]: ];
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1613 [G1 PS1]: for (under, before, ex, next, (num, den)) in cases {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1614 [G1 PS1]: let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1615 [G1 PS1]: let (eve, post, later) = (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1616 [G1 PS1]: day_of(under, before),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1617 [G1 PS1]: day_of(under, ex),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1618 [G1 PS1]: day_of(under, next),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1619 [G1 PS1]: );
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1623 [G1 PS1]: let mut latest: std::collections::BTreeMap<String, (ContractDay, Vec<(u32, i64)>)> =
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1624 [G1 PS1]: std::collections::BTreeMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1625 [G1 PS1]: let mut on = Day::from_days(before.days_from_epoch() - 100).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1626 [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1626:15 False [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1626:15 True [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1627 [G1 PS1]: for (c, rows) in day_of(under, on) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1628 [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1628:20 False [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1628:20 True [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1629 [G1 PS1]: latest.insert(c.contract().unwrap().to_string(), (c, rows));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1630 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1632 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1634 [G1 PS1]: let pre: Vec<(ContractDay, Vec<(u32, i64)>)> = latest.into_values().collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1635 [G1 PS1]: let strip = |days: &[(ContractDay, Vec<(u32, i64)>)]| -> Vec<ContractDay> {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1636 [G1 PS1]: days.iter().map(|(c, _)| *c).collect()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1637 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1638 [G1 PS1]: let eve_only = detect(symbol, ex, &strip(&eve), &strip(&post)).expect(under);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1639 [G1 PS1]: let event = detect(symbol, ex, &strip(&pre), &strip(&post)).expect(under);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1640 [G1 PS1]: assert!(event.lineage.len() >= eve_only.lineage.len(), "{under}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1641 [G1 PS1]: let count = |carry: OiCarry| event.lineage.iter().filter(|l| l.oi == carry).count();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1642 [G1 PS1]: let reused = event.lineage.iter().filter(|l| l.reused).count();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1644 [G1 PS1]: let mut stale_rows = 0_usize;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1645 [G1 PS1]: let mut records: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1646 [G1 PS1]: for pair in event.lineage.iter().filter(|l| l.oi == OiCarry::Unadjusted) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1647 [G1 PS1]: let new = post
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1648 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1649 [G1 PS1]: .find(|(c, _)| c.contract() == Some(pair.new))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1650 [G1 PS1]: .unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1651 [G1 PS1]: let stale = pair.stale_oi(&new.1);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1652 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1653 [G1 PS1]: stale.rows > 0 && stale.until_sod.is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1653:17 False [G1 PS1]: stale.rows > 0 && stale.until_sod.is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1653:17 True [G1 PS1]: stale.rows > 0 && stale.until_sod.is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1653:35 False [G1 PS1]: stale.rows > 0 && stale.until_sod.is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1653:35 True [G1 PS1]: stale.rows > 0 && stale.until_sod.is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1656 [G1 PS1]: stale_rows += stale.rows;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1657 [G1 PS1]: records.push(pair.record(&new.1).line());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1661 [G1 PS1]: for line in &records {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1662 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1663 [G1 PS1]: crate::gdfl_lineage::LineageRecord::parse(line).is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1669 [G1 PS1]: let odd: Vec<i64> = event
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1670 [G1 PS1]: .lineage
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1671 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1672 [G1 PS1]: .filter(|l| l.oi == OiCarry::Unexplained)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1673 [G1 PS1]: .map(|pair| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1674 [G1 PS1]: let old = pre
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1675 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1676 [G1 PS1]: .find(|(c, _)| c.contract() == Some(pair.old))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1677 [G1 PS1]: .unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1678 [G1 PS1]: let new = post
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1679 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1680 [G1 PS1]: .find(|(c, _)| c.contract() == Some(pair.new))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1681 [G1 PS1]: .unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1682 [G1 PS1]: new.0.first_oi.unwrap() * 1_000 / old.0.close_oi.unwrap()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1683 [G1 PS1]: })
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1684 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1685 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1689 [G1 PS1]: pre.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1690 [G1 PS1]: eve.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1691 [G1 PS1]: eve_only.lineage.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1692 [G1 PS1]: post.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1693 [G1 PS1]: event.lineage.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1694 [G1 PS1]: event.unpaired.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1695 [G1 PS1]: event.stitched.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1696 [G1 PS1]: count(OiCarry::Scaled),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1697 [G1 PS1]: count(OiCarry::Unadjusted),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1698 [G1 PS1]: count(OiCarry::Indistinct),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1699 [G1 PS1]: count(OiCarry::Unexplained),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1700 [G1 PS1]: count(OiCarry::Missing),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1702 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1703 [G1 PS1]: event.lot_after * den,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1704 [G1 PS1]: event.lot_before * num,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1707 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1708 [G1 PS1]: eve_only.lineage.len() * 2 > eve.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1712 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1713 [G1 PS1]: detect(symbol, next, &strip(&post), &strip(&later)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1719 [G1 PS1]: let retired = event.retired();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1720 [G1 PS1]: let last = pre.iter().map(|(c, _)| c.expiry).max().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1721 [G1 PS1]: let mut again: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1722 [G1 PS1]: let mut on = ex;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1723 [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1723:15 False [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1723:15 True [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1725 [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1725:20 False [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1725:20 True [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1726 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1727 [G1 PS1]: let decoded = listing.ticker_of(file).map(|t| decode_ticker(t, on));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1728 [G1 PS1]: if let Some(Ok(ticker)) = decoded
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1728:28 False [G1 PS1]: if let Some(Ok(ticker)) = decoded
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1728:28 True [G1 PS1]: if let Some(Ok(ticker)) = decoded
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1729 [G1 PS1]: && ticker.underlying.as_str() == under
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1729:28 False [G1 PS1]: && ticker.underlying.as_str() == under
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1729:28 True [G1 PS1]: && ticker.underlying.as_str() == under
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1730 [G1 PS1]: && retired.contains(&ticker.contract)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1730:28 False [G1 PS1]: && retired.contains(&ticker.contract)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1730:28 True [G1 PS1]: && retired.contains(&ticker.contract)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1731 [G1 PS1]: {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1732 [G1 PS1]: again.insert(ticker.contract.to_string());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1733 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1735 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1736 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1738 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1740 [G1 PS1]: retired.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1741 [G1 PS1]: again.len()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1743 [G1 PS1]: assert!(again.len() >= event.stitched.len(), "{under}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1745 [G1 PS1]: match under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1746 [G1 PS1]: "NTPC" => assert_eq!(again.len(), 35, "NTPC's stitched series"),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1747 [G1 PS1]: "BRITANNIA" => assert_eq!(retired.len(), 56, "BRITANNIA's series ending early"),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1748 [G1 PS1]: "BAJFINANCE" => assert_eq!(count(OiCarry::Unadjusted), 18, "stale ex-date OI"),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1749 [G1 PS1]: _ => {}
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1752 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1760 [G1 PS1]: fn the_decoders_costs_on_real_days_are_measured() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1761 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1762 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1763 [G1 PS1]: let pct = |mut ns: Vec<u128>, q: usize| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1764 [G1 PS1]: ns.sort_unstable();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1765 [G1 PS1]: ns[(ns.len() * q / 100).min(ns.len() - 1)]
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1766 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1767 [G1 PS1]: for on in [day(2023, 6, 27), day(2026, 8, 3)] {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1768 [G1 PS1]: let listing = zips.day(on).unwrap().expect("a vendor day");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1769 [G1 PS1]: let names: Vec<&str> = listing
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1770 [G1 PS1]: .entries()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1771 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1772 [G1 PS1]: .filter_map(|f| listing.ticker_of(f))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1773 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1774 [G1 PS1]: let mut decode_ns = Vec::with_capacity(names.len());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1775 [G1 PS1]: let mut decoded = Vec::with_capacity(names.len());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1776 [G1 PS1]: for name in &names {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1777 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1778 [G1 PS1]: let got = decode_ticker(name, on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1779 [G1 PS1]: decode_ns.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1780 [G1 PS1]: decoded.extend(got.ok().map(|t| (t, *name)));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1782 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1783 [G1 PS1]: let first = decoded
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1784 [G1 PS1]: .first()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1785 [G1 PS1]: .map(|(t, _)| listing.strike_median(t.underlying.as_str()));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1786 [G1 PS1]: let medians_ns = at.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1787 [G1 PS1]: let mut check_ns = Vec::with_capacity(decoded.len());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1788 [G1 PS1]: for (ticker, name) in &decoded {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1789 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1790 [G1 PS1]: let _ = listing.check_strike(ticker, name);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1791 [G1 PS1]: check_ns.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1792 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1793 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1795 [G1 PS1]: names.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1796 [G1 PS1]: decoded.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1797 [G1 PS1]: pct(decode_ns.clone(), 50),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1798 [G1 PS1]: pct(decode_ns, 99),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1799 [G1 PS1]: medians_ns / 1_000,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1800 [G1 PS1]: pct(check_ns.clone(), 50),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1801 [G1 PS1]: pct(check_ns, 99),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1803 [G1 PS1]: assert!(first.flatten().is_some(), "{on}: a median");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1804 [G1 PS1]: assert_eq!(decoded.len(), names.len(), "{on}: every real name decodes");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:1806 [G1 PS1]: }
F crates/pull/src/gdfl_nfo_tests.rs:1909:9 False [G1 PS1]: words.contains("2026.zip:") && words.contains("2025.zip:"),
F crates/pull/src/gdfl_nfo_tests.rs:1909:40 False [G1 PS1]: words.contains("2026.zip:") && words.contains("2025.zip:"),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2188 [G1 PS1]: fn the_real_unchanged_lot_events_are_detected_and_ordinary_days_are_not() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2190 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2191 [G1 PS1]: let only = std::env::var("BRUTEX_WP12E_ONLY").ok();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2192 [G1 PS1]: let lookback: u32 = std::env::var("BRUTEX_WP12E_LOOKBACK")
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2193 [G1 PS1]: .ok()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2194 [G1 PS1]: .and_then(|text| text.parse().ok())
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2195 [G1 PS1]: .unwrap_or(30);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2196 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2197 [G1 PS1]: let day_of = |under: &str, on: Day| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2198 [G1 PS1]: let mut out: Vec<ContractDay> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2199 [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2199:13 False [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2199:13 True [G1 PS1]: let Some(listing) = zips.day(on).unwrap() else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2200 [G1 PS1]: return out;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2202 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2203 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2203:17 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2203:17 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2204 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2206 [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2206:17 False [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2206:17 True [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2207 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2209 [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2209:16 False [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2209:16 True [G1 PS1]: if ticker.underlying.as_str() != under {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2210 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2211 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2212 [G1 PS1]: let read = read_file(&zips, &listing, file).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2213 [G1 PS1]: out.push(ContractDay::from_rows(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2214 [G1 PS1]: &read.ticker,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2215 [G1 PS1]: read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2218 [G1 PS1]: out
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2219 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2222 [G1 PS1]: let offset = |shift| Some(EventKind::StrikeOffset { shift });
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2223 [G1 PS1]: let cases = [
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2224 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2225 [G1 PS1]: "OIL",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2226 [G1 PS1]: day(2020, 2, 19),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2227 [G1 PS1]: day(2020, 2, 20),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2228 [G1 PS1]: day(2020, 2, 24),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2229 [G1 PS1]: offset(900),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2230 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2231 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2232 [G1 PS1]: "COALINDIA",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2233 [G1 PS1]: day(2020, 3, 18),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2234 [G1 PS1]: day(2020, 3, 19),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2235 [G1 PS1]: day(2020, 3, 20),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2236 [G1 PS1]: offset(1_200),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2237 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2238 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2239 [G1 PS1]: "VEDL",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2240 [G1 PS1]: day(2022, 7, 25),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2241 [G1 PS1]: day(2022, 7, 26),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2242 [G1 PS1]: day(2022, 7, 27),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2243 [G1 PS1]: offset(1_950),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2244 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2245 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2246 [G1 PS1]: "HEROMOTOCO",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2247 [G1 PS1]: day(2023, 2, 16),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2248 [G1 PS1]: day(2023, 2, 17),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2249 [G1 PS1]: day(2023, 2, 20),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2250 [G1 PS1]: offset(6_500),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2251 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2252 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2253 [G1 PS1]: "VEDL",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2254 [G1 PS1]: day(2024, 9, 9),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2255 [G1 PS1]: day(2024, 9, 10),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2256 [G1 PS1]: day(2024, 9, 11),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2257 [G1 PS1]: offset(2_000),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2258 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2259 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2260 [G1 PS1]: "TATACHEM",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2261 [G1 PS1]: day(2020, 3, 3),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2262 [G1 PS1]: day(2020, 3, 4),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2263 [G1 PS1]: day(2020, 3, 5),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2264 [G1 PS1]: Some(EventKind::Relisted),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2265 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2266 [G1 PS1]: // Ordinary days: an index across a weekly expiry, and the last full
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2267 [G1 PS1]: // options day the store holds.
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2268 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2269 [G1 PS1]: "NIFTY",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2270 [G1 PS1]: day(2020, 9, 10),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2271 [G1 PS1]: day(2020, 9, 11),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2272 [G1 PS1]: day(2020, 9, 14),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2273 [G1 PS1]: None,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2274 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2275 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2276 [G1 PS1]: "BANKNIFTY",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2277 [G1 PS1]: day(2020, 9, 14),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2278 [G1 PS1]: day(2020, 9, 15),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2279 [G1 PS1]: day(2020, 9, 16),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2280 [G1 PS1]: None,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2281 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2282 [G1 PS1]: (
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2283 [G1 PS1]: "RELIANCE",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2284 [G1 PS1]: day(2020, 9, 14),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2285 [G1 PS1]: day(2020, 9, 15),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2286 [G1 PS1]: day(2020, 9, 16),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2287 [G1 PS1]: None,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2288 [G1 PS1]: ),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2289 [G1 PS1]: ];
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2290 [G1 PS1]: for (under, before, ex, next, want) in cases {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2291 [G1 PS1]: if only.as_deref().is_some_and(|name| name != under) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2291:12 False [G1 PS1]: if only.as_deref().is_some_and(|name| name != under) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2291:12 True [G1 PS1]: if only.as_deref().is_some_and(|name| name != under) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2292 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2293 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2294 [G1 PS1]: let symbol = brutex_core::symbol::Symbol::new(under).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2296 [G1 PS1]: let back = if want.is_none() && under != "RELIANCE" {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2296:23 False [G1 PS1]: let back = if want.is_none() && under != "RELIANCE" {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2296:23 True [G1 PS1]: let back = if want.is_none() && under != "RELIANCE" {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2296:41 False [G1 PS1]: let back = if want.is_none() && under != "RELIANCE" {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2296:41 True [G1 PS1]: let back = if want.is_none() && under != "RELIANCE" {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2297 [G1 PS1]: 0
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2299 [G1 PS1]: lookback
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2301 [G1 PS1]: let mut latest: std::collections::BTreeMap<String, ContractDay> =
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2302 [G1 PS1]: std::collections::BTreeMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2303 [G1 PS1]: let mut on = Day::from_days(before.days_from_epoch() - back).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2304 [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2304:15 False [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2304:15 True [G1 PS1]: while on <= before {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2305 [G1 PS1]: for c in day_of(under, on) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2306 [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2306:20 False [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2306:20 True [G1 PS1]: if c.expiry >= ex {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2307 [G1 PS1]: latest.insert(c.contract().unwrap().to_string(), c);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2308 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2310 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2312 [G1 PS1]: let pre: Vec<ContractDay> = latest.into_values().collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2313 [G1 PS1]: let (post, later) = (day_of(under, ex), day_of(under, next));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2314 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2315 [G1 PS1]: let got = detect(symbol, ex, &pre, &post);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2316 [G1 PS1]: let took = at.elapsed();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2317 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2319 [G1 PS1]: pre.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2320 [G1 PS1]: post.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2321 [G1 PS1]: took.as_micros(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2322 [G1 PS1]: got.as_ref().map_or_else(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2323 [G1 PS1]: || "none".to_owned(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2324 [G1 PS1]: |a| format!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2329 [G1 PS1]: a.lineage.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2330 [G1 PS1]: a.unpaired.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2331 [G1 PS1]: a.stitched.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2332 [G1 PS1]: a.lineage.iter().filter(|l| l.reused).count()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2336 [G1 PS1]: assert_eq!(got.as_ref().map(|a| a.kind), want, "{under} {ex}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2337 [G1 PS1]: if let Some(action) = &got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2337:16 False [G1 PS1]: if let Some(action) = &got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2337:16 True [G1 PS1]: if let Some(action) = &got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2338 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2342 [G1 PS1]: for pair in &action.lineage {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2343 [G1 PS1]: let line = pair.record(&[]).line();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2344 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2345 [G1 PS1]: crate::gdfl_lineage::LineageRecord::parse(&line).is_some(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2349 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2351 [G1 PS1]: let control = detect(symbol, next, &post, &later);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2352 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2353 [G1 PS1]: control.map(|a| a.kind),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2358 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2369 [G1 PS1]: fn the_real_window_sweep_lists_every_event_the_detector_declares() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2372 [G1 PS1]: let dir = std::env::var_os("BRUTEX_GDFL_NFO_ARCHIVES").expect("BRUTEX_GDFL_NFO_ARCHIVES");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2373 [G1 PS1]: let bound = |var: &str| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2374 [G1 PS1]: let text = std::env::var(var).expect(var);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2375 [G1 PS1]: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2376 [G1 PS1]: day(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2377 [G1 PS1]: parts[0],
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2378 [G1 PS1]: u8::try_from(parts[1]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2379 [G1 PS1]: u8::try_from(parts[2]).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2381 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2382 [G1 PS1]: let (from, to) = (bound("BRUTEX_WP12E_FROM"), bound("BRUTEX_WP12E_TO"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2383 [G1 PS1]: let indices = std::env::var_os("BRUTEX_WP12E_INDICES").is_some();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2384 [G1 PS1]: let zips = NfoZips::new(Path::new(&dir));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2385 [G1 PS1]: let mut state: HashMap<String, HashMap<(Day, i64, OptionSide), ContractDay>> = HashMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2386 [G1 PS1]: let (mut days, mut underlying_days, mut unread) = (0_u32, 0_u32, 0_u32);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2387 [G1 PS1]: let mut events: Vec<String> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2388 [G1 PS1]: let mut nanos: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2389 [G1 PS1]: let mut on = from;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2390 [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2390:11 False [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2390:11 True [G1 PS1]: while on <= to {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2391 [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2391:16 False [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2391:16 True [G1 PS1]: if let Some(listing) = zips.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2392 [G1 PS1]: days += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2393 [G1 PS1]: let mut by: BTreeMap<String, Vec<ContractDay>> = BTreeMap::new();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2394 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2395 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2395:21 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2395:21 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2396 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2398 [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2398:21 False [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2398:21 True [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2399 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2401 [G1 PS1]: let under = ticker.underlying.as_str();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2402 [G1 PS1]: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2402:20 False [G1 PS1]: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2402:20 True [G1 PS1]: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2402:32 False [G1 PS1]: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2402:32 True [G1 PS1]: if !indices && brutex_core::alias::index_by_fno(under).is_some() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2403 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2404 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2405 [G1 PS1]: let Ok(read) = read_file(&zips, &listing, file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2405:21 False [G1 PS1]: let Ok(read) = read_file(&zips, &listing, file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2405:21 True [G1 PS1]: let Ok(read) = read_file(&zips, &listing, file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2406 [G1 PS1]: unread += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2407 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2409 [G1 PS1]: by.entry(under.to_owned())
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2410 [G1 PS1]: .or_default()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2411 [G1 PS1]: .push(ContractDay::from_rows(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2412 [G1 PS1]: &read.ticker,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2413 [G1 PS1]: read.file.rows.iter().map(|r| (r.sod, r.ltq, r.oi)),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2416 [G1 PS1]: for (under, todays) in by {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2417 [G1 PS1]: let latest = state.entry(under.clone()).or_default();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2418 [G1 PS1]: latest.retain(|key, _| key.0 >= on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2419 [G1 PS1]: let before: Vec<ContractDay> = latest.values().copied().collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2420 [G1 PS1]: let symbol = brutex_core::symbol::Symbol::new(&under).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2421 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2422 [G1 PS1]: let got = detect(symbol, on, &before, &todays);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2423 [G1 PS1]: nanos.push(at.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2424 [G1 PS1]: underlying_days += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2425 [G1 PS1]: if let Some(action) = got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2425:24 False [G1 PS1]: if let Some(action) = got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2425:24 True [G1 PS1]: if let Some(action) = got {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2426 [G1 PS1]: events.push(format!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2427 [G1 PS1]: "{on} {under} {:?} lot {}->{} pairs {} unpaired {} stitched {} (pre-ex keys {}, day keys {})",
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2428 [G1 PS1]: action.kind,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2429 [G1 PS1]: action.lot_before,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2430 [G1 PS1]: action.lot_after,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2431 [G1 PS1]: action.lineage.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2432 [G1 PS1]: action.unpaired.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2433 [G1 PS1]: action.stitched.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2434 [G1 PS1]: before.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2435 [G1 PS1]: todays.len()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2436 [G1 PS1]: ));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2437 [G1 PS1]: latest.clear();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2438 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2439 [G1 PS1]: for c in todays {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2440 [G1 PS1]: latest.insert((c.expiry, c.strike, c.side), c);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2441 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2443 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2444 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2446 [G1 PS1]: nanos.sort_unstable();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2447 [G1 PS1]: let at = |q: usize| {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2448 [G1 PS1]: nanos
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2449 [G1 PS1]: .get(nanos.len().saturating_sub(1) * q / 100)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2450 [G1 PS1]: .copied()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2451 [G1 PS1]: .unwrap_or(0)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2452 [G1 PS1]: };
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2453 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2455 [G1 PS1]: events.len()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2457 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2459 [G1 PS1]: nanos.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2460 [G1 PS1]: at(50),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2461 [G1 PS1]: at(99),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2462 [G1 PS1]: at(100)
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2464 [G1 PS1]: for event in &events {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2465 [G1 PS1]: eprintln!("  {event}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2466 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2467 [G1 PS1]: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2467:13 False [G1 PS1]: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2467:13 True [G1 PS1]: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2467:25 False [G1 PS1]: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2467:25 True [G1 PS1]: assert!(days > 0 && underlying_days > 0, "no vendor day in range");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2468 [G1 PS1]: }
F crates/pull/src/gdfl_nfo_tests.rs:2589 [G1 PS1]: _ => None,
F crates/pull/src/gdfl_nfo_tests.rs:2593:13 False [G1 PS1]: new.rows == old.rows
F crates/pull/src/gdfl_nfo_tests.rs:2594:20 False [G1 PS1]: && new.quoted_rows == old.quoted_rows
F crates/pull/src/gdfl_nfo_tests.rs:2595:20 False [G1 PS1]: && new.refused.total() == 0
F crates/pull/src/gdfl_nfo_tests.rs:2607 [G1 PS1]: (Err(_), Ok(_)) => false,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2805 [G1 PS1]: fn the_fixed_array_decodes_real_days_as_the_field_vector_did() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2806 [G1 PS1]: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2807 [G1 PS1]: let store = NfoTickStore::new(Path::new(&root));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2808 [G1 PS1]: for on in [day(2018, 9, 3), day(2023, 11, 12), day(2026, 8, 3)] {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2809 [G1 PS1]: let listing = store.day(on).unwrap().expect("a tick-store day");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2810 [G1 PS1]: let (mut files, mut rows, mut refused) = (0_u64, 0_u64, 0_u64);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2811 [G1 PS1]: let (mut fixed_ns, mut vector_ns) = (0_u128, 0_u128);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2812 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2813 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2813:17 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2813:17 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2814 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2816 [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2816:17 False [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2816:17 True [G1 PS1]: let Ok(ticker) = decode_ticker(name, on) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2817 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2819 [G1 PS1]: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2819:16 False [G1 PS1]: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2819:16 True [G1 PS1]: if !["NIFTY", "BANKNIFTY", "RELIANCE"].contains(&ticker.underlying.as_str()) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2820 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2821 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2822 [G1 PS1]: let bytes = store.fetch(file).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2823 [G1 PS1]: crate::gdfl_cm::verify(file, &bytes).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2824 [G1 PS1]: let stem = format!("{name}{ROW_SUFFIX}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2825 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2826 [G1 PS1]: let fixed = decode(&bytes, &stem, on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2827 [G1 PS1]: fixed_ns += at.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2828 [G1 PS1]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2829 [G1 PS1]: let vector = decode_by_field_vector(&bytes, &stem, on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2830 [G1 PS1]: vector_ns += at.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2831 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2832 [G1 PS1]: definition_5_differs_only_where_4_refused(&fixed, &vector),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2835 [G1 PS1]: files += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2836 [G1 PS1]: match fixed {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2837 [G1 PS1]: Ok(read) => rows += u64::try_from(read.rows.len()).unwrap(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2838 [G1 PS1]: Err(_) => refused += 1,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2841 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2844 [G1 PS1]: fixed_ns / 1_000_000,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2845 [G1 PS1]: vector_ns / 1_000_000
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2847 [G1 PS1]: assert!(files > 0, "{on}: the day holds the three underlyings");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:2849 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3514 [G1 PS1]: fn the_measured_bad_option_files_are_read_row_by_row() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3515 [G1 PS1]: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3516 [G1 PS1]: let store = NfoTickStore::new(Path::new(&root));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3517 [G1 PS1]: let named = [
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3518 [G1 PS1]: (day(2018, 11, 9), Some("AXISBANK18NOV580PE")),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3519 [G1 PS1]: (day(2018, 11, 6), Some("BANKNIFTY15NOV1826700CE")),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3520 [G1 PS1]: (day(2026, 4, 16), None),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3521 [G1 PS1]: (day(2025, 3, 11), Some("BANKNIFTY26JUN2546500CE")),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3522 [G1 PS1]: ];
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3523 [G1 PS1]: for (on, only) in named {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3524 [G1 PS1]: let listing = store.day(on).unwrap().expect("a tick-store day");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3525 [G1 PS1]: let mut recovered = 0_u32;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3526 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3527 [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3527:17 False [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3527:17 True [G1 PS1]: let Some(name) = listing.ticker_of(file) else {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3528 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3530 [G1 PS1]: if only.is_some_and(|wanted| wanted != name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3530:16 False [G1 PS1]: if only.is_some_and(|wanted| wanted != name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3530:16 True [G1 PS1]: if only.is_some_and(|wanted| wanted != name) {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3531 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3532 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3533 [G1 PS1]: let bytes = store.fetch(file).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3534 [G1 PS1]: crate::gdfl_cm::verify(file, &bytes).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3535 [G1 PS1]: let stem = format!("{name}{ROW_SUFFIX}");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3536 [G1 PS1]: let old = decode_by_field_vector(&bytes, &stem, on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3537 [G1 PS1]: if old.is_ok() && only.is_none() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3537:16 False [G1 PS1]: if old.is_ok() && only.is_none() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3537:16 True [G1 PS1]: if old.is_ok() && only.is_none() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3537:31 False [G1 PS1]: if old.is_ok() && only.is_none() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3537:31 True [G1 PS1]: if old.is_ok() && only.is_none() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3538 [G1 PS1]: continue;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3539 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3540 [G1 PS1]: let new = decode(&bytes, &stem, on);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3541 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3542 [G1 PS1]: definition_5_differs_only_where_4_refused(&new, &old),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3545 [G1 PS1]: let got = new.unwrap_or_else(|why| panic!("{on} {name}: {why}"));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3546 [G1 PS1]: let lines: Vec<String> = got
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3547 [G1 PS1]: .refused
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3548 [G1 PS1]: .named
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3549 [G1 PS1]: .iter()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3550 [G1 PS1]: .map(|row| format!("{}:{}", row.line, row.fault.code()))
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3551 [G1 PS1]: .collect();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3552 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3554 [G1 PS1]: old.as_ref().map_or_else(ToString::to_string, |f| format!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3556 [G1 PS1]: f.rows.len()
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3558 [G1 PS1]: got.rows.len(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3559 [G1 PS1]: got.refused.total(),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3560 [G1 PS1]: lines.join(" "),
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3563 [G1 PS1]: recovered += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3565 [G1 PS1]: assert!(recovered > 0, "{on}: no file of the measured shape found");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3567 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3622 [G1 PS1]: fn no_real_option_file_of_2018_ends_in_a_bare_cr() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3623 [G1 PS1]: let root = std::env::var_os("BRUTEX_GDFL_TICKSTORE").expect("BRUTEX_GDFL_TICKSTORE");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3624 [G1 PS1]: let store = NfoTickStore::new(Path::new(&root));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3625 [G1 PS1]: let (mut on, last) = (day(2018, 9, 3), day(2018, 12, 31));
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3626 [G1 PS1]: let (mut days, mut files, mut lf, mut bare_cr, mut no_newline) =
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3627 [G1 PS1]: (0_u32, 0_u64, 0_u64, 0_u64, 0_u64);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3628 [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3628:11 False [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3628:11 True [G1 PS1]: while on <= last {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3629 [G1 PS1]: if let Some(listing) = store.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3629:16 False [G1 PS1]: if let Some(listing) = store.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3629:16 True [G1 PS1]: if let Some(listing) = store.day(on).unwrap() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3630 [G1 PS1]: days += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3631 [G1 PS1]: let mut newline_less = 0_u64;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3632 [G1 PS1]: for file in listing.entries() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3633 [G1 PS1]: let bytes = store.fetch(file).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3634 [G1 PS1]: crate::gdfl_cm::verify(file, &bytes).unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3635 [G1 PS1]: files += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3636 [G1 PS1]: match bytes.last() {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3637 [G1 PS1]: Some(b'\n') => lf += 1,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3638 [G1 PS1]: Some(b'\r') => {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3639 [G1 PS1]: bare_cr += 1;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3640 [G1 PS1]: eprintln!("{on}: {} ends in a bare CR", file.entry);
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3641 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3642 [G1 PS1]: _ => newline_less += 1,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3645 [G1 PS1]: if newline_less > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3645:16 False [G1 PS1]: if newline_less > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3645:16 True [G1 PS1]: if newline_less > 0 {
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3646 [G1 PS1]: eprintln!("{on}: {newline_less} option files end with no newline");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3647 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3648 [G1 PS1]: no_newline += newline_less;
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3649 [G1 PS1]: }
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3650 [G1 PS1]: on = on.succ().unwrap();
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3652 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3656 [G1 PS1]: assert!(
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3657 [G1 PS1]: days > 0 && files > 0,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3657:9 False [G1 PS1]: days > 0 && files > 0,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3657:9 True [G1 PS1]: days > 0 && files > 0,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3657:21 False [G1 PS1]: days > 0 && files > 0,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3657:21 True [G1 PS1]: days > 0 && files > 0,
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3660 [G1 PS1]: assert_eq!(bare_cr, 0, "a real file ends in a bare CR");
IGNORED crates/pull/src/gdfl_nfo_tests.rs:3661 [G1 PS1]: }
F crates/pull/src/gdfl_r2_attack_tests.rs:1055:9 False [G2 PS2]: bars_checked > 1_000 && refusals > 500 && filtered > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1055:33 False [G2 PS2]: bars_checked > 1_000 && refusals > 500 && filtered > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1055:51 False [G2 PS2]: bars_checked > 1_000 && refusals > 500 && filtered > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1071:9 False [G2 PS2]: weekday(expiry) && settles_as_stated("NIFTY", expiry, on),
F crates/pull/src/gdfl_r2_attack_tests.rs:1071:28 False [G2 PS2]: weekday(expiry) && settles_as_stated("NIFTY", expiry, on),
F crates/pull/src/gdfl_r2_attack_tests.rs:1128 [G2 PS2]: ra.failures[0].why
F crates/pull/src/gdfl_r2_attack_tests.rs:1415 [G2 PS2]: unreachable!("a refused or done journal reads no day")
F crates/pull/src/gdfl_r2_attack_tests.rs:1672:9 False [G2 PS2]: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1672:25 False [G2 PS2]: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1672:44 False [G2 PS2]: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1672:61 False [G2 PS2]: handed > 500 && ambiguous > 100 && tried > 6_000 && implausible_names > 50,
F crates/pull/src/gdfl_r2_attack_tests.rs:1712 [G2 PS2]: matches!(
F crates/pull/src/gdfl_r2_attack_tests.rs:1724 [G2 PS2]: matches!(
F crates/pull/src/gdfl_r2_attack_tests.rs:1771:13 False [G2 PS2]: assert!(weekday(cutover) && weekday(weeklies));
F crates/pull/src/gdfl_r2_attack_tests.rs:1771:33 False [G2 PS2]: assert!(weekday(cutover) && weekday(weeklies));
F crates/pull/src/gdfl_r2_attack_tests.rs:1935:13 False [G2 PS2]: !text.contains("2023-06-29") && !text.contains("2023-08-15")
F crates/pull/src/gdfl_r3_attack_tests.rs:377:34 False [G1 PS1]: name == want_name || name.starts_with(&format!("{want_name}.")),
F crates/pull/src/gdfl_r3_attack_tests.rs:442 [G1 PS1]: String::from_utf8_lossy(tail)
F crates/pull/src/gdfl_r3_attack_tests.rs:451 [G1 PS1]: String::from_utf8_lossy(tail)
F crates/pull/src/gdfl_r3_attack_tests.rs:476 [G1 PS1]: String::from_utf8_lossy(tail)
F crates/pull/src/gdfl_r3_attack_tests.rs:589:9 False [G1 PS1]: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
F crates/pull/src/gdfl_r3_attack_tests.rs:589:27 False [G1 PS1]: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
F crates/pull/src/gdfl_r3_attack_tests.rs:589:49 False [G1 PS1]: loads > 15_000 && line_tears > 2_000 && close_tears > 500,
F crates/pull/src/gdfl_r3_attack_tests.rs:772 [G1 PS1]: block.len(),
F crates/pull/src/gdfl_r3_attack_tests.rs:773 [G1 PS1]: got.as_ref().map(Vec::len)
F crates/pull/src/gdfl_r3_attack_tests.rs:788 [G1 PS1]: block.len(),
F crates/pull/src/gdfl_r3_attack_tests.rs:789 [G1 PS1]: got.as_ref().map(Vec::len)
F crates/pull/src/gdfl_r3_attack_tests.rs:818:29 False [G1 PS1]: got.is_err() || rebuilt_whole,
F crates/pull/src/gdfl_r3_attack_tests.rs:820 [G1 PS1]: got.map(|b| b.len())
F crates/pull/src/gdfl_r3_attack_tests.rs:903 [G1 PS1]: unreachable!("only swept index names are drawn")
F crates/pull/src/gdfl_r3_attack_tests.rs:1219:9 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1219:25 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1219:43 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1228:9 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1228:25 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1228:43 False [G1 PS1]: bars > 1_000 && refusals > 300 && filtered > 150,
F crates/pull/src/gdfl_r3_attack_tests.rs:1420 [G1 PS1]: all.files + all.files_refused
F crates/pull/src/gdfl_r4_attack_tests.rs:130 [G1 PS1]: journal_text(&root)
F crates/pull/src/gdfl_r4_attack_tests.rs:205 [G1 PS1]: journal_text(&root)
F crates/pull/src/gdfl_r4_attack_tests.rs:282:13 False [G1 PS1]: last.starts_with(&format!(
F crates/pull/src/gdfl_r4_attack_tests.rs:284:19 False [G1 PS1]: )) && last.contains(" late=1 late_unresolved=0 max_back_s=7 ")
F crates/pull/src/gdfl_r4_attack_tests.rs:285:20 False [G1 PS1]: && last.contains(" stage=file reason=bars_differ_held"),
F crates/pull/src/gdfl_r4_attack_tests.rs:491:13 False [G1 PS1]: assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
F crates/pull/src/gdfl_r4_attack_tests.rs:491:33 False [G1 PS1]: assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
F crates/pull/src/gdfl_r4_attack_tests.rs:678 [G1 PS1]: journal_text(&root)
F crates/pull/src/gdfl_r4_attack_tests.rs:799:20 False [G1 PS1]: || appended.starts_with("done stocks 2024-04-")
F crates/pull/src/gdfl_r4_attack_tests.rs:800:24 False [G1 PS1]: && appended.contains(&format!(" definition={BAR_DEFINITION} ")),
F crates/pull/src/gdfl_r4_attack_tests.rs:818:13 False [G1 PS1]: assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
F crates/pull/src/gdfl_r4_attack_tests.rs:818:54 False [G1 PS1]: assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
F crates/pull/src/gdfl_r4_attack_tests.rs:819:13 False [G1 PS1]: assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
F crates/pull/src/gdfl_r4_attack_tests.rs:819:62 False [G1 PS1]: assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
U crates/pull/src/gdfl_rekey.rs:320:35 False [G2 PS2]: while let Some(next) = self.superseded.get(&on) {
U crates/pull/src/gdfl_rekey.rs:859:9 False [G2 PS2]: let (Some(first), Some(last)) = (
U crates/pull/src/gdfl_rekey.rs:863 [G2 PS2]: return Ok(out);
U crates/pull/src/gdfl_rekey.rs:883 [G2 PS2]: Err(_) => return Ok(out),
U crates/pull/src/gdfl_rekey.rs:1193:27 False [G2 PS2]: Some(seen) if seen == this => {}
U crates/pull/src/gdfl_rekey.rs:1194 [G2 PS2]: Some((seen, _)) => {
U crates/pull/src/gdfl_rekey.rs:1195 [G2 PS2]: return Verdict::Unresolved(format!(
U crates/pull/src/gdfl_rekey.rs:1196 [G2 PS2]: "the decoder settles its days on two expiries, {seen} and {}; \
U crates/pull/src/gdfl_rekey.rs:1197 [G2 PS2]: refused rather than split",
U crates/pull/src/gdfl_rekey.rs:1198 [G2 PS2]: this.0
U crates/pull/src/gdfl_rekey.rs:1199 [G2 PS2]: ));
U crates/pull/src/gdfl_rekey.rs:1230:39 False [G2 PS2]: Some((expiry, Some(kind))) if expiry != key.expiry => {
U crates/pull/src/gdfl_rekey.rs:1328:16 False [G2 PS2]: if let Some(median) = lower_median(&mut strikes) {
U crates/pull/src/gdfl_rekey.rs:1437:17 False [G2 PS2]: let Ok(month) = day.year_month() else {
U crates/pull/src/gdfl_rekey.rs:1438 [G2 PS2]: continue;
U crates/pull/src/gdfl_rekey.rs:1673:12 False [G2 PS2]: if at < out.moves.len() {
U crates/pull/src/gdfl_rekey.rs:1746:17 False [G2 PS2]: let Some(one) = out.moves.get(at) else {
U crates/pull/src/gdfl_rekey.rs:1747 [G2 PS2]: continue;
U crates/pull/src/gdfl_rekey.rs:1774:12 False [G2 PS2]: if at < out.moves.len() {
U crates/pull/src/gdfl_rekey.rs:2291:12 False [G2 PS2]: if let Some(dir) = lineage.parent() {
U crates/pull/src/gdfl_rekey.rs:2293 [G2 PS2]: }
F crates/pull/src/gdfl_rekey_ps2_tests.rs:349:13 False [G2 PS2]: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
F crates/pull/src/gdfl_rekey_ps2_tests.rs:349:38 False [G2 PS2]: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
F crates/pull/src/gdfl_rekey_ps2_tests.rs:349:59 False [G2 PS2]: assert!(empty.keys.is_empty() && empty.strays == 0 && empty.files == 0);
F crates/pull/src/gdfl_rekey_ps2_tests.rs:479:13 False [G2 PS2]: assert!(decided.recorded_moves.is_empty() && decided.recorded_retired.is_empty());
F crates/pull/src/gdfl_rekey_ps2_tests.rs:479:50 False [G2 PS2]: assert!(decided.recorded_moves.is_empty() && decided.recorded_retired.is_empty());
F crates/pull/src/gdfl_rekey_ps2_tests.rs:811 [G2 PS2]: done.refused[0].why
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1062 [G2 PS2]: done.refused[0].why
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1064:13 False [G2 PS2]: assert!(done.moved.is_empty() && done.lines_written == 0);
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1064:38 False [G2 PS2]: assert!(done.moved.is_empty() && done.lines_written == 0);
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1234:9 False [G2 PS2]: let Verdict::Unresolved(why) = judge(&wide, &HashMap::new()) else {
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1235 [G2 PS2]: panic!("a contract past the store's width is not moved");
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1242:9 False [G2 PS2]: let Verdict::Move(to, kind) = judge(&narrow, &HashMap::new()) else {
F crates/pull/src/gdfl_rekey_ps2_tests.rs:1243 [G2 PS2]: panic!("a contract that renders moves");
F crates/pull/src/gdfl_rekey_tests.rs:425:13 False [G2 PS2]: assert!(decided.retired.is_empty() && decided.moves.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:425:43 False [G2 PS2]: assert!(decided.retired.is_empty() && decided.moves.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:543:13 False [G2 PS2]: assert!(again.moves.is_empty() && again.retired.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:543:39 False [G2 PS2]: assert!(again.moves.is_empty() && again.retired.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:629 [G2 PS2]: done.refused[0].why
F crates/pull/src/gdfl_rekey_tests.rs:814 [G2 PS2]: refusal.refusal()
F crates/pull/src/gdfl_rekey_tests.rs:1024:13 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:1024:39 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:1027:9 False [G2 PS2]: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
F crates/pull/src/gdfl_rekey_tests.rs:1027:34 False [G2 PS2]: why.contains(SETTLED) && why.contains("2023-06-27-1000000-PE"),
F crates/pull/src/gdfl_rekey_tests.rs:1048:13 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:1048:39 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:1162:13 False [G2 PS2]: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1162:49 False [G2 PS2]: assert!(before.recorded_moves.is_empty() && before.recorded_retired.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1278:13 False [G2 PS2]: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1278:41 False [G2 PS2]: assert!(decided.moves.is_empty() && decided.recorded_moves.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1284 [G2 PS2]: decided.unresolved[0].why
F crates/pull/src/gdfl_rekey_tests.rs:1316 [G2 PS2]: || panic!("{key}: {:?}", decided.unresolved),
F crates/pull/src/gdfl_rekey_tests.rs:1409:9 False [G2 PS2]: why.contains("and ledger line 1 retires it as")
F crates/pull/src/gdfl_rekey_tests.rs:1410:16 False [G2 PS2]: && why.contains("one retirement has one account"),
F crates/pull/src/gdfl_rekey_tests.rs:1445:13 False [G2 PS2]: assert!(done.refused.is_empty() && done.moved.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1445:40 False [G2 PS2]: assert!(done.refused.is_empty() && done.moved.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1581:9 False [G2 PS2]: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
F crates/pull/src/gdfl_rekey_tests.rs:1581:37 False [G2 PS2]: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
F crates/pull/src/gdfl_rekey_tests.rs:1581:67 False [G2 PS2]: decided.moves.is_empty() && decided.retired.is_empty() && decided.unresolved.is_empty(),
F crates/pull/src/gdfl_rekey_tests.rs:1705:13 False [G2 PS2]: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1705:40 False [G2 PS2]: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1705:69 False [G2 PS2]: assert!(second.moves.is_empty() && second.retired.is_empty() && second.unresolved.is_empty());
F crates/pull/src/gdfl_rekey_tests.rs:1759:9 False [G2 PS2]: let CensusStep::Unreadable(why) = &decided.census else {
F crates/pull/src/gdfl_rekey_tests.rs:1760 [G2 PS2]: panic!("{:?}", decided.census);
F crates/pull/src/gdfl_rekey_tests.rs:1867:24 False [G2 PS2]: .any(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}")
F crates/pull/src/gdfl_rekey_tests.rs:1894:9 False [G2 PS2]: left.why.contains("(apply's own check: ")
F crates/pull/src/gdfl_rekey_tests.rs:1895:16 False [G2 PS2]: && left.why.contains("would precede or contradict"),
F crates/pull/src/gdfl_rekey_tests.rs:1945:9 False [G2 PS2]: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
F crates/pull/src/gdfl_rekey_tests.rs:1945:46 False [G2 PS2]: why.contains("line 1 supersedes") && why.contains("and line 2 with 2 month(s), 4 bar(s)"),
F crates/pull/src/gdfl_rekey_tests.rs:1952:9 False [G2 PS2]: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
F crates/pull/src/gdfl_rekey_tests.rs:1952:46 False [G2 PS2]: why.contains("line 1 supersedes") && why.contains("and line 3 with"),
F crates/pull/src/gdfl_rekey_tests.rs:1956:9 False [G2 PS2]: why.contains("0707070707070707") && why.contains("0808080808080808"),
F crates/pull/src/gdfl_rekey_tests.rs:1956:45 False [G2 PS2]: why.contains("0707070707070707") && why.contains("0808080808080808"),
F crates/pull/src/gdfl_rekey_tests.rs:1965:9 False [G2 PS2]: why.contains("line 1 retires") && why.contains("and line 2 as"),
F crates/pull/src/gdfl_rekey_tests.rs:1965:43 False [G2 PS2]: why.contains("line 1 retires") && why.contains("and line 2 as"),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2078 [G2 PS2]: fn synthetic(n: usize) -> (Survey, Withdrawn) {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2079 [G2 PS2]: let under = sym("NIFTY");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2080 [G2 PS2]: let mut keys = Vec::with_capacity(n);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2081 [G2 PS2]: let mut ledger = String::new();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2082 [G2 PS2]: for at in 0..n {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2083 [G2 PS2]: let stated = at % 10 == 9;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084 [G2 PS2]: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084:32 False [G2 PS2]: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2084:32 True [G2 PS2]: let (expiry, day) = if stated {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2085 [G2 PS2]: (d(2023, 6, 29), d(2020, 6, 15))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2087 [G2 PS2]: (d(2019, 6, 27), d(2019, 6, 3))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2089 [G2 PS2]: let strike = 1_000_000 + 5_000 * i64::try_from(at / 2).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090 [G2 PS2]: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090:23 False [G2 PS2]: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2090:23 True [G2 PS2]: let side = if at % 2 == 0 { "PE" } else { "CE" };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2091 [G2 PS2]: let text = format!("{expiry}-{strike}-{side}");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2092 [G2 PS2]: let contract = c(&text);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2093 [G2 PS2]: let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + 34_000) * 1_000_000;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2094 [G2 PS2]: keys.push(Stored {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2095 [G2 PS2]: underlying: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2096 [G2 PS2]: contract,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2097 [G2 PS2]: expiry,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2098 [G2 PS2]: strike,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099 [G2 PS2]: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099:22 False [G2 PS2]: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2099:22 True [G2 PS2]: side: if at % 2 == 0 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2100 [G2 PS2]: OptionSide::Put
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2102 [G2 PS2]: OptionSide::Call
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2104 [G2 PS2]: months: vec![Month {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2105 [G2 PS2]: month: day.year_month().unwrap(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2106 [G2 PS2]: rows: 2,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2107 [G2 PS2]: first_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2108 [G2 PS2]: last_ts_micros: ts + 1_000_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2109 [G2 PS2]: }],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2110 [G2 PS2]: days: BTreeMap::from([(day, 2)]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112 [G2 PS2]: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:12 False [G2 PS2]: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:12 True [G2 PS2]: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:22 False [G2 PS2]: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2112:22 True [G2 PS2]: if stated && at % 20 == 19 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2113 [G2 PS2]: let to = c(&format!("2023-06-28-{strike}-{side}"));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2114 [G2 PS2]: let record = SupersededRecord {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2115 [G2 PS2]: underlying: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2116 [G2 PS2]: old: contract,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2117 [G2 PS2]: new: to,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2118 [G2 PS2]: kind: AliasKind::VendorRedated,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2119 [G2 PS2]: months: 1,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2120 [G2 PS2]: bars: 2,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2121 [G2 PS2]: digest: [9; 32],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2122 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2123 [G2 PS2]: ledger.push_str(&record.line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2124 [G2 PS2]: ledger.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2125 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2127 [G2 PS2]: keys.sort_by(|a, b| a.contract.as_str().cmp(b.contract.as_str()));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2128 [G2 PS2]: (
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2129 [G2 PS2]: Survey {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2130 [G2 PS2]: keys,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2131 [G2 PS2]: ..Survey::default()
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2132 [G2 PS2]: },
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2133 [G2 PS2]: Withdrawn::parse(&ledger).unwrap(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2134 [G2 PS2]: )
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2135 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2142 [G2 PS2]: fn plan_cost_per_key_is_flat_in_the_number_of_keys() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2143 [G2 PS2]: for n in [1_000_usize, 10_000] {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2144 [G2 PS2]: let (found, ledger) = synthetic(n);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2145 [G2 PS2]: let mut per_key: Vec<u128> = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2146 [G2 PS2]: let mut judged = 0;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2147 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2148 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2149 [G2 PS2]: let decided = plan(&found, &ledger);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2150 [G2 PS2]: per_key.push(started.elapsed().as_nanos() / n as u128);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2151 [G2 PS2]: judged = decided.keys;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2152 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2153 [G2 PS2]: assert_eq!(judged, n as u64);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2154 [G2 PS2]: per_key.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2155 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2157 [G2 PS2]: per_key[99], per_key[197], per_key[199]
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2160 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2170 [G2 PS2]: fn census_step_cost_is_one_load_per_plan_flat_per_entry() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2171 [G2 PS2]: for n in [10_000_usize, 100_000] {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2172 [G2 PS2]: let scratch = Scratch::new(&format!("census-cost-{n}"));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2173 [G2 PS2]: let root = scratch.root();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2174 [G2 PS2]: let under = sym("NIFTY");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2175 [G2 PS2]: let month = YearMonth::new(2020, 6).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2176 [G2 PS2]: let mut census =
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2177 [G2 PS2]: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, u64::try_from(n).unwrap())
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2178 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2179 [G2 PS2]: let mut ledger = String::new();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2180 [G2 PS2]: for at in 0..n {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2181 [G2 PS2]: let key = c(&format!("2023-06-28-{}-PE", 100_000 + at * 5));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2182 [G2 PS2]: let ts = 1_592_193_400_000_000 + i64::try_from(at).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2183 [G2 PS2]: census
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2184 [G2 PS2]: .record_held(Held::unknown(Entry {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2185 [G2 PS2]: key: EntryKey {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2186 [G2 PS2]: contract: Some(key),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2187 [G2 PS2]: exchange: Exchange::Nse,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2188 [G2 PS2]: segment: Segment::Fno,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2189 [G2 PS2]: symbol: under,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2190 [G2 PS2]: timeframe: RUNG,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2191 [G2 PS2]: month,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2192 [G2 PS2]: },
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2193 [G2 PS2]: rows: 1,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2194 [G2 PS2]: first_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2195 [G2 PS2]: last_ts_micros: ts,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2196 [G2 PS2]: }))
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2197 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198 [G2 PS2]: if at < 13 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198:16 False [G2 PS2]: if at < 13 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2198:16 True [G2 PS2]: if at < 13 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2199 [G2 PS2]: ledger.push_str(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2200 [G2 PS2]: &RetiredRecord {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2201 [G2 PS2]: key,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2202 [G2 PS2]: ..refusal_of("2019-05-30-1150000-CE")
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2203 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2204 [G2 PS2]: .line(),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2205 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2206 [G2 PS2]: ledger.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2207 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2209 [G2 PS2]: let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2210 [G2 PS2]: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2211 [G2 PS2]: std::fs::write(&path, census.image()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2212 [G2 PS2]: let ledger = Withdrawn::parse(&ledger).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2213 [G2 PS2]: let mut per_plan: Vec<u128> = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2214 [G2 PS2]: let mut listed = 0;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2215 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2216 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2217 [G2 PS2]: let step = census_step(root, &ledger);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2218 [G2 PS2]: per_plan.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2219 [G2 PS2]: listed = step.pending();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2220 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2221 [G2 PS2]: assert_eq!(listed, 13);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2222 [G2 PS2]: per_plan.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2223 [G2 PS2]: let per_entry = |ns: u128| ns / n as u128;
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2224 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2227 [G2 PS2]: per_plan[99] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2228 [G2 PS2]: per_plan[197] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2229 [G2 PS2]: per_plan[199] / 1_000,
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2230 [G2 PS2]: per_entry(per_plan[99]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2231 [G2 PS2]: per_entry(per_plan[197]),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2232 [G2 PS2]: per_entry(per_plan[199])
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2235 [G2 PS2]: }
F crates/pull/src/gdfl_rekey_tests.rs:2301:17 False [G2 PS2]: why.contains("the ledger changed since this plan; re-plan")
F crates/pull/src/gdfl_rekey_tests.rs:2302:24 False [G2 PS2]: && why.contains("nothing was written"),
F crates/pull/src/gdfl_rekey_tests.rs:2339:9 False [G2 PS2]: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
F crates/pull/src/gdfl_rekey_tests.rs:2339:46 False [G2 PS2]: why.contains("gdfl.lineage.lock") && why.contains("held by another re-key apply"),
F crates/pull/src/gdfl_rekey_tests.rs:2394:17 False [G2 PS2]: refused.contains("nothing was written")
F crates/pull/src/gdfl_rekey_tests.rs:2396:28 False [G2 PS2]: || refused.contains("the ledger changed since this plan")),
F crates/pull/src/gdfl_rekey_tests.rs:2427:13 False [G2 PS2]: why.contains("the plan counted 3 bar(s)")
F crates/pull/src/gdfl_rekey_tests.rs:2428:20 False [G2 PS2]: && why.contains("bars landed under it after the survey; re-plan"),
F crates/pull/src/gdfl_rekey_tests.rs:2454:13 False [G2 PS2]: why.contains("retires NSE-FNO-NIFTY-2020-06-25-191250000-CE")
F crates/pull/src/gdfl_rekey_tests.rs:2455:20 False [G2 PS2]: && why.contains("bars move only into a key that continues"),
F crates/pull/src/gdfl_rekey_tests.rs:2458:17 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:2458:43 False [G2 PS2]: assert!(why.contains("line 1") && why.contains("line 2"), "{why}");
F crates/pull/src/gdfl_rekey_tests.rs:2574:9 False [G2 PS2]: left.why.contains("retires it with 1 bar(s) on 1 day(s)")
F crates/pull/src/gdfl_rekey_tests.rs:2575:16 False [G2 PS2]: && left.why.contains("holds 2 bar(s) on 1 day(s) now"),
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2582 [G2 PS2]: fn spread(mut samples: Vec<u128>) -> String {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2583 [G2 PS2]: samples.sort_unstable();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2584 [G2 PS2]: format!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2586 [G2 PS2]: samples[99],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2587 [G2 PS2]: samples[197],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2588 [G2 PS2]: samples[199],
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2589 [G2 PS2]: samples.len()
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2591 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2603 [G2 PS2]: fn apply_guard_cost_per_operation() {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2604 [G2 PS2]: let fixture = Fixture::new("guardcost");
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2605 [G2 PS2]: fixture.counted();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2606 [G2 PS2]: let root = fixture.root();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2607 [G2 PS2]: let at = ledger_lock_path(root);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2608 [G2 PS2]: let mut lock = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2609 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2610 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2611 [G2 PS2]: let file = open_ledger_lock(&at).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2612 [G2 PS2]: let held = store::flock::Flock::try_lock(&file, at.as_path()).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2613 [G2 PS2]: held.release().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2614 [G2 PS2]: lock.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2615 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2616 [G2 PS2]: let mut text = String::with_capacity(10_000 * 160);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2617 [G2 PS2]: for strike in 0..10_000_i64 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2618 [G2 PS2]: let old = format!("2023-06-29-{}-PE", 1_000_000 + strike * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2619 [G2 PS2]: let new = format!("2023-06-28-{}-PE", 1_000_000 + strike * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2620 [G2 PS2]: text.push_str(&pointer(&old, &new).line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2621 [G2 PS2]: text.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2622 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2623 [G2 PS2]: let read = Withdrawn::parse(&text).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2624 [G2 PS2]: let again = Withdrawn::parse(&text).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2625 [G2 PS2]: let mut compare = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2626 [G2 PS2]: let mut rules = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2627 [G2 PS2]: let mut narrowed = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2628 [G2 PS2]: let only: BTreeSet<Symbol> = [sym("NIFTY")].into_iter().collect();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2629 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2630 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2631 [G2 PS2]: read.same_records(&again, None).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2632 [G2 PS2]: compare.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2633 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2634 [G2 PS2]: read.same_records(&again, Some(&only)).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2635 [G2 PS2]: narrowed.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2636 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2637 [G2 PS2]: read.targets_continue().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2638 [G2 PS2]: rules.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2639 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2640 [G2 PS2]: let mut chain = String::with_capacity(10_000 * 160);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2641 [G2 PS2]: for at in 0..10_000_i64 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2642 [G2 PS2]: let from = format!("2023-06-29-{}-PE", 1_000_000 + at * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2643 [G2 PS2]: let to = format!("2023-06-29-{}-PE", 1_000_000 + (at + 1) * 500);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2644 [G2 PS2]: chain.push_str(&pointer(&from, &to).line());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2645 [G2 PS2]: chain.push('\n');
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2646 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2647 [G2 PS2]: let linked = Withdrawn::parse(&chain).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2648 [G2 PS2]: let mut walk = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2649 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2650 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2651 [G2 PS2]: linked.targets_continue().unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2652 [G2 PS2]: walk.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2653 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2654 [G2 PS2]: let mut source = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2655 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2656 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2657 [G2 PS2]: let (months, bars) = source_now(root, sym("NIFTY"), c(STATED)).unwrap();
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2658 [G2 PS2]: source.push(started.elapsed().as_nanos());
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2659 [G2 PS2]: assert_eq!((months.len(), bars), (2, 3));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2661 [G2 PS2]: eprintln!("APPLY_GUARD lock_open_take_release {}", spread(lock));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2662 [G2 PS2]: eprintln!("APPLY_GUARD same_records_10000_lines {}", spread(compare));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2663 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2665 [G2 PS2]: spread(narrowed)
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2667 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2669 [G2 PS2]: spread(rules)
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2671 [G2 PS2]: eprintln!("APPLY_GUARD targets_continue_10000_chain {}", spread(walk));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2672 [G2 PS2]: eprintln!("APPLY_GUARD source_now_2_months {}", spread(source));
IGNORED crates/pull/src/gdfl_rekey_tests.rs:2673 [G2 PS2]: }
F crates/pull/src/gdfl_rekey_tests.rs:2707:9 False [G2 PS2]: refused.why.contains("the plan counted 1 bar(s) to retire")
F crates/pull/src/gdfl_rekey_tests.rs:2708:16 False [G2 PS2]: && refused.why.contains("it holds 2 bar(s) in 1 month(s) now")
F crates/pull/src/gdfl_rekey_tests.rs:2709:16 False [G2 PS2]: && refused.why.ends_with("re-plan"),
F crates/pull/src/gdfl_rekey_tests.rs:2721:9 False [G2 PS2]: census_before.contains(&row) && listed.contains(&row),
F crates/pull/src/gdfl_rekey_tests.rs:2721:41 False [G2 PS2]: census_before.contains(&row) && listed.contains(&row),
F crates/pull/src/gdfl_rekey_tests.rs:2795:13 False [G2 PS2]: why.contains("the ledger changed since this plan; re-plan")
F crates/pull/src/gdfl_rekey_tests.rs:2796:20 False [G2 PS2]: && why.contains("nothing was written"),
U crates/pull/src/gdfl_run.rs:1412:16 False [G2 PS2]: && held.source == source
U crates/pull/src/gdfl_run.rs:1423:16 False [G2 PS2]: if let Some(entry) = self.held.get_mut(slot) {
U crates/pull/src/gdfl_run.rs:1431 [G2 PS2]: }
U crates/pull/src/gdfl_run.rs:1489:21 False [G2 PS2]: let Ok(next) = day.succ() else {
U crates/pull/src/gdfl_run.rs:1490 [G2 PS2]: return Some(month(first));
U crates/pull/src/gdfl_run.rs:1498:13 False [G2 PS2]: let Ok(next) = end.succ() else {
U crates/pull/src/gdfl_run.rs:1499 [G2 PS2]: return None;
U crates/pull/src/gdfl_run.rs:1950 [G2 PS2]: format!(
U crates/pull/src/gdfl_run.rs:1952 [G2 PS2]: kind.as_str()
U crates/pull/src/gdfl_run.rs:1954 [G2 PS2]: })
F crates/pull/src/gdfl_run_cov_tests.rs:421 [G2 PS2]: other => panic!("{source:?}: {other:?}"),
F crates/pull/src/gdfl_run_cov_tests.rs:466:9 False [G2 PS2]: let KindEnd::Refused(refusal) = &end else {
F crates/pull/src/gdfl_run_cov_tests.rs:467 [G2 PS2]: panic!("{end:?}");
F crates/pull/src/gdfl_run_ia2_tests.rs:127:13 True [G3 PS3]: assert!(!store.join(ledger::LEDGER).exists() || ledger::read_all(&store).unwrap().is_empty());
F crates/pull/src/gdfl_run_ia2_tests.rs:127:53 False [G3 PS3]: assert!(!store.join(ledger::LEDGER).exists() || ledger::read_all(&store).unwrap().is_empty());
F crates/pull/src/gdfl_run_ia2_tests.rs:284:17 False [G3 PS3]: assert!(field.starts_with("crc=") && field.len() == 12, "{field}");
F crates/pull/src/gdfl_run_ia2_tests.rs:284:46 False [G3 PS3]: assert!(field.starts_with("crc=") && field.len() == 12, "{field}");
F crates/pull/src/gdfl_run_ia2_tests.rs:347:9 False [G3 PS3]: refused.to_string().contains("runs.claims")
F crates/pull/src/gdfl_run_ia2_tests.rs:348:16 False [G3 PS3]: && refused.to_string().contains("not a regular file"),
F crates/pull/src/gdfl_run_ia2_tests.rs:363:9 False [G3 PS3]: refused.to_string().contains("runs.claims")
F crates/pull/src/gdfl_run_ia2_tests.rs:364:16 False [G3 PS3]: && refused.to_string().contains("was not handed out"),
F crates/pull/src/gdfl_run_ia2_tests.rs:408:66 False [G3 PS3]: matches!(&got, LockFault::Stalled { lock, waited_ms } if lock == "gdfl.journal.lock" && *waited_ms >= 60),
F crates/pull/src/gdfl_run_ia2_tests.rs:408:97 False [G3 PS3]: matches!(&got, LockFault::Stalled { lock, waited_ms } if lock == "gdfl.journal.lock" && *waited_ms >= 60),
F crates/pull/src/gdfl_run_ia2_tests.rs:430:9 False [G3 PS3]: let RunRefusal::RunBusy { lock, holder } = &api else {
F crates/pull/src/gdfl_run_ia2_tests.rs:431 [G3 PS3]: panic!("{api:?}");
F crates/pull/src/gdfl_run_ia2_tests.rs:435:9 False [G3 PS3]: holder.contains(&format!("run={} ", cli.run)) && holder.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:435:58 False [G3 PS3]: holder.contains(&format!("run={} ", cli.run)) && holder.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:549:13 False [G3 PS3]: assert!(!claimed.claim.host.contains(' ') && !claimed.claim.host.contains('='));
F crates/pull/src/gdfl_run_ia2_tests.rs:549:50 False [G3 PS3]: assert!(!claimed.claim.host.contains(' ') && !claimed.claim.host.contains('='));
F crates/pull/src/gdfl_run_ia2_tests.rs:554 [G3 PS3]: other => panic!("{other:?}"),
F crates/pull/src/gdfl_run_ia2_tests.rs:559:96 False [G3 PS3]: seen.iter().any(|s| matches!(s, LockState::Held { claim: ClaimState::Claim(c), .. } if c.run == claimed.run)),
F crates/pull/src/gdfl_run_ia2_tests.rs:583:9 False [G3 PS3]: let [
F crates/pull/src/gdfl_run_ia2_tests.rs:591 [G3 PS3]: panic!("{seen:?}");
F crates/pull/src/gdfl_run_ia2_tests.rs:601:93 False [G3 PS3]: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("crc"))
F crates/pull/src/gdfl_run_ia2_tests.rs:605:93 False [G3 PS3]: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("digest"))
F crates/pull/src/gdfl_run_ia2_tests.rs:609:93 False [G3 PS3]: matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("torn"))
F crates/pull/src/gdfl_run_ia2_tests.rs:616:106 False [G3 PS3]: matches!(seen.as_slice(), [LockState::Interrupted { claim: ClaimState::Unreadable(why), .. }] if why.contains("\\x1b")),
F crates/pull/src/gdfl_run_ia2_tests.rs:656:67 False [G3 PS3]: matches!(&busy, RunRefusal::RunBusy { holder, .. } if holder.contains(&format!("run={run} ")))
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:800 [G3 PS3]: fn ia2_timings() {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:801 [G3 PS3]: fn pct(mut v: Vec<Duration>) -> String {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:802 [G3 PS3]: v.sort_unstable();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:803 [G3 PS3]: let n = v.len();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:804 [G3 PS3]: format!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:806 [G3 PS3]: v[n / 2],
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:807 [G3 PS3]: v[n * 99 / 100],
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:808 [G3 PS3]: v[n - 1]
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:810 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:811 [G3 PS3]: let store = scratch("timings");
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:812 [G3 PS3]: std::fs::create_dir_all(store.join("imports")).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:813 [G3 PS3]: let control = stop::StopControl::new(store.join("imports/gdfl.stop"), 1);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:814 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:815 [G3 PS3]: for _ in 0..10_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:816 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:817 [G3 PS3]: let _seen = control.requested();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:818 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:819 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:820 [G3 PS3]: println!("C-GR-07 stop check, no stop file: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:821 [G3 PS3]: let other = stop::StopRequest {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:822 [G3 PS3]: run: 2,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:823 [G3 PS3]: requested: NOW,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:824 [G3 PS3]: by: "cli".to_owned(),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:825 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:826 [G3 PS3]: stop::write_stop(&store.join("imports/gdfl.stop"), &other).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:827 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:828 [G3 PS3]: for _ in 0..10_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:829 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:830 [G3 PS3]: let _seen = control.requested();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:831 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:832 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:833 [G3 PS3]: println!("C-GR-07 stop check, another run's stop file: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:834 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:835 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:836 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:837 [G3 PS3]: allocate(&store, NOW).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:838 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:839 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:840 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:842 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:844 [G3 PS3]: let path = store.join(ledger::LEDGER);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:845 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:846 [G3 PS3]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:847 [G3 PS3]: let mut bytes = std::fs::read(&path).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:848 [G3 PS3]: bytes.extend_from_slice(&[b'x'; 77]);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:849 [G3 PS3]: std::fs::write(&path, &bytes).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:850 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:851 [G3 PS3]: allocate(&store, NOW).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:852 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:853 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:854 [G3 PS3]: println!("C-GR-08 run id allocation over a torn tail: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:855 [G3 PS3]: let lock = crate::gdfl_import::journal_lock_path(&crate::gdfl_import::journal_path(&store));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:856 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:857 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:858 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:859 [G3 PS3]: drop(locks::take_path_within(&lock, locks::LOCK_WAIT).unwrap());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:860 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:861 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:862 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:864 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:866 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:867 [G3 PS3]: for _ in 0..500 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:868 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:869 [G3 PS3]: let claimed = claim_with(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:870 [G3 PS3]: &store,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:871 [G3 PS3]: template("options", "cli"),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:872 [G3 PS3]: locks::PROBE_RETRY,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:873 [G3 PS3]: None,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:874 [G3 PS3]: NOW,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:875 [G3 PS3]: )
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:876 [G3 PS3]: .unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:877 [G3 PS3]: claimed.release().unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:878 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:879 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:880 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:882 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:884 [G3 PS3]: let held = claim_with(&store, template("options", "cli"), 0, None, NOW).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:885 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:886 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:887 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:888 [G3 PS3]: let seen = locks::lock_state(&store, 0);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:889 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:890 [G3 PS3]: assert!(!seen.is_empty());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:892 [G3 PS3]: println!("lock_state probe, one held lock: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:893 [G3 PS3]: held.release().unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:894 [G3 PS3]: }
F crates/pull/src/gdfl_run_ia2_tests.rs:915:9 False [G3 PS3]: busy.starts_with("run_busy: ")
F crates/pull/src/gdfl_run_ia2_tests.rs:916:16 False [G3 PS3]: && busy.contains(&format!("run={} ", run.run))
F crates/pull/src/gdfl_run_ia2_tests.rs:917:16 False [G3 PS3]: && busy.ends_with("Nothing was written"),
F crates/pull/src/gdfl_run_ia2_tests.rs:938:9 False [G3 PS3]: claim.contains(" mode=census-recount kind=census ") && claim.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:938:64 False [G3 PS3]: claim.contains(" mode=census-recount kind=census ") && claim.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:972:9 False [G3 PS3]: claim.contains(" mode=dayflags kind=options ") && claim.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:972:59 False [G3 PS3]: claim.contains(" mode=dayflags kind=options ") && claim.contains(" by=cli "),
F crates/pull/src/gdfl_run_ia2_tests.rs:1155:62 False [G3 PS3]: matches!(&unnamed, LockFault::Busy { holder, .. } if holder.is_empty()),
F crates/pull/src/gdfl_run_ia2_tests.rs:1210:13 False [G3 PS3]: let RunRefusal::RunBusy {
F crates/pull/src/gdfl_run_ia2_tests.rs:1215 [G3 PS3]: panic!("{busy:?}");
F crates/pull/src/gdfl_run_ia2_tests.rs:1219:13 False [G3 PS3]: named.contains(&format!("run={} ", holder.run))
F crates/pull/src/gdfl_run_ia2_tests.rs:1220:20 False [G3 PS3]: && named.contains(" pid=4242 ")
F crates/pull/src/gdfl_run_ia2_tests.rs:1221:20 False [G3 PS3]: && named.contains(" by=cli "),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1297 [G3 PS3]: fn ia2_repair_timings() {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1298 [G3 PS3]: fn pct(mut v: Vec<Duration>) -> String {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1299 [G3 PS3]: v.sort_unstable();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1300 [G3 PS3]: let n = v.len();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1301 [G3 PS3]: format!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1303 [G3 PS3]: v[n / 2],
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1304 [G3 PS3]: v[n * 99 / 100],
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1305 [G3 PS3]: v[n - 1]
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1307 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1308 [G3 PS3]: let root = scratch("repair-timings");
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1309 [G3 PS3]: let v4 = v4(&root);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1310 [G3 PS3]: std::fs::create_dir_all(v4.join("imports")).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1311 [G3 PS3]: let path = v4.join("imports/gdfl.options.stop");
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1312 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1313 [G3 PS3]: for at in 0..500_u64 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1314 [G3 PS3]: let _fresh = std::fs::remove_file(&path);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1315 [G3 PS3]: let request = stop::StopRequest {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1316 [G3 PS3]: run: 9,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1317 [G3 PS3]: requested: NOW + at,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1318 [G3 PS3]: by: "cli".to_owned(),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1319 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1320 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1321 [G3 PS3]: stop::write_stop(&path, &request).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1322 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1323 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1324 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1326 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1328 [G3 PS3]: let again = stop::StopRequest {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1329 [G3 PS3]: run: 9,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1330 [G3 PS3]: requested: NOW,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1331 [G3 PS3]: by: "api".to_owned(),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1332 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1333 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1334 [G3 PS3]: for _ in 0..10_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1335 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1336 [G3 PS3]: let _first = stop::write_stop(&path, &again).unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1337 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1338 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1339 [G3 PS3]: println!("stop asked again, file names the run: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1340 [G3 PS3]: let _fresh = std::fs::remove_file(&path);
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1341 [G3 PS3]: let held: Vec<_> = ["indices", "stocks", "options"]
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1342 [G3 PS3]: .into_iter()
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1343 [G3 PS3]: .map(|kind| claim_with(&v4, template(kind, "cli"), 0, None, NOW).unwrap())
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1344 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1345 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1346 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1347 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1348 [G3 PS3]: let refused = RunLocks::take_store(&v4, 0).unwrap_err();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1349 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1350 [G3 PS3]: assert!(matches!(refused, LockFault::Busy { .. }));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1352 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1354 [G3 PS3]: pct(took)
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1356 [G3 PS3]: let refused_sink = LogState::Refused("held".to_owned());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1357 [G3 PS3]: let mut took = Vec::new();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1358 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1359 [G3 PS3]: let request = Request::new(
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1360 [G3 PS3]: Kinds::of(&[ImportKind::Options]).unwrap(),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1361 [G3 PS3]: SourceWord::TickStore,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1362 [G3 PS3]: (Day::new(2024, 4, 1).unwrap(), Day::new(2024, 4, 1).unwrap()),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1364 [G3 PS3]: let process = Process {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1365 [G3 PS3]: store_root: &v4,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1366 [G3 PS3]: commit: Some("feedc0de"),
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1367 [G3 PS3]: log: &refused_sink,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1368 [G3 PS3]: unstamped: None,
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1369 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1370 [G3 PS3]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1371 [G3 PS3]: let refused = plan(request, &process, &|_| None, &mut SourceCache::default()).unwrap_err();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1372 [G3 PS3]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1373 [G3 PS3]: assert_eq!(refused.code(), "run_busy");
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1375 [G3 PS3]: println!("plan refused before its sink, holder named: {}", pct(took));
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1376 [G3 PS3]: for one in held {
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1377 [G3 PS3]: one.release().unwrap();
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1378 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_run_ia2_tests.rs:1379 [G3 PS3]: }
U crates/pull/src/gdfl_run_ledger.rs:185:12 True [G2 PS2]: if body.len() > BODY {
U crates/pull/src/gdfl_run_ledger.rs:186 [G2 PS2]: return Err(format!(
U crates/pull/src/gdfl_run_ledger.rs:187 [G2 PS2]: "a record body of {} bytes is past {BODY}",
U crates/pull/src/gdfl_run_ledger.rs:188 [G2 PS2]: body.len()
U crates/pull/src/gdfl_run_ledger.rs:189 [G2 PS2]: ));
U crates/pull/src/gdfl_run_ledger.rs:202:16 False [G2 PS2]: if let Some(last) = out.last_mut() {
U crates/pull/src/gdfl_run_ledger.rs:222:13 False [G2 PS2]: let [id, mode, kind, by, pid, started] = fixed else {
U crates/pull/src/gdfl_run_ledger.rs:223 [G2 PS2]: return None;
U crates/pull/src/gdfl_run_ledger.rs:540:12 False [G2 PS2]: if let Some(last) = record.last_mut() {
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:164:17 False [G2 PS2]: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:165 [G2 PS2]: panic!("not an io fault");
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:196:9 False [G2 PS2]: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:197 [G2 PS2]: panic!("not an io fault");
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:211:9 False [G2 PS2]: let RunsLedgerFault::Io { why } = allocate(&store, &entry()).unwrap_err() else {
F crates/pull/src/gdfl_run_ledger_ps2_tests.rs:212 [G2 PS2]: panic!("not an io fault");
U crates/pull/src/gdfl_run_locks.rs:423:13 False [G1 PS1]: let Some(one) = file.take() else {
U crates/pull/src/gdfl_run_locks.rs:424 [G1 PS1]: return Err(LockFault::Refused {
U crates/pull/src/gdfl_run_locks.rs:425 [G1 PS1]: lock: name.to_owned(),
U crates/pull/src/gdfl_run_locks.rs:426 [G1 PS1]: why: "the lock file was lost between two tries".to_owned(),
U crates/pull/src/gdfl_run_locks.rs:427 [G1 PS1]: });
U crates/pull/src/gdfl_run_locks.rs:507:13 False [G1 PS1]: let Some(one) = file.take() else {
U crates/pull/src/gdfl_run_locks.rs:508 [G1 PS1]: return Err(LockFault::Refused {
U crates/pull/src/gdfl_run_locks.rs:509 [G1 PS1]: lock: name.to_owned(),
U crates/pull/src/gdfl_run_locks.rs:510 [G1 PS1]: why: "the lock file was lost between two tries".to_owned(),
U crates/pull/src/gdfl_run_locks.rs:511 [G1 PS1]: });
U crates/pull/src/gdfl_run_locks.rs:1164:12 False [G1 PS1]: if let Some(dir) = path.parent() {
U crates/pull/src/gdfl_run_locks.rs:1168 [G1 PS1]: }
F crates/pull/src/gdfl_run_locks_cov_tests.rs:70:9 False [G1 PS1]: said.contains("was v3 when") && said.contains("is v4 now"),
F crates/pull/src/gdfl_run_locks_cov_tests.rs:70:41 False [G1 PS1]: said.contains("was v3 when") && said.contains("is v4 now"),
F crates/pull/src/gdfl_run_locks_cov_tests.rs:161:9 False [G1 PS1]: long.len() <= HOST_MAX && long.len().is_multiple_of(3),
F crates/pull/src/gdfl_run_locks_cov_tests.rs:161:35 False [G1 PS1]: long.len() <= HOST_MAX && long.len().is_multiple_of(3),
F crates/pull/src/gdfl_run_locks_cov_tests.rs:301:61 False [G1 PS1]: matches!(&state, LockState::Unprobed { why, .. } if why == "is a directory"),
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:66:9 False [G1 PS1]: let Err(why) = locate(&store, Vendor::Gdfl) else {
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:67 [G1 PS1]: panic!("a file at the census's name was located");
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:211:9 False [G1 PS1]: let Err(LockFault::Refused { lock, why }) = open_lock_file(&store, "/") else {
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:212 [G1 PS1]: panic!("the root opened as a lock file");
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:228:9 False [G1 PS1]: let Err(LockFault::Refused { lock, why }) = take_path_within(&path, Duration::ZERO) else {
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:229 [G1 PS1]: panic!("a directory opened as a lock file");
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:282:9 False [G1 PS1]: let Err(TryLockError::Error(host)) = unlockable(&path).try_lock() else {
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:283 [G1 PS1]: panic!("the host took a flock it refuses");
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:353 [G1 PS1]: matches!(
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:9 False [G1 PS1]: output.status.success() && said.contains("1 passed"),
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:36 False [G1 PS1]: output.status.success() && said.contains("1 passed"),
F crates/pull/src/gdfl_run_locks_ps1_tests.rs:503:35 False [G1 PS1]: faults.iter().all(|fault| fault
F crates/pull/src/gdfl_run_stop_cov_tests.rs:93:9 False [G1 PS1]: let StopRefusal::ClaimBeingWritten { lock } = &refused else {
F crates/pull/src/gdfl_run_stop_cov_tests.rs:94 [G1 PS1]: panic!("{refused:?}");
F crates/pull/src/gdfl_run_tests.rs:370:9 False [G2 PS2]: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
F crates/pull/src/gdfl_run_tests.rs:370:62 False [G2 PS2]: words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
F crates/pull/src/gdfl_run_tests.rs:626:13 False [G2 PS2]: let KindEnd::Report(report) = end else {
F crates/pull/src/gdfl_run_tests.rs:627 [G2 PS2]: panic!("{end:?}");
F crates/pull/src/gdfl_run_tests.rs:680:9 False [G2 PS2]: let KindEnd::Unopened(refused) = &ends[0].1 else {
F crates/pull/src/gdfl_run_tests.rs:681 [G2 PS2]: panic!("{ends:?}");
IGNORED crates/pull/src/gdfl_run_tests.rs:856 [G2 PS2]: fn source_cache_hit_timing() {
IGNORED crates/pull/src/gdfl_run_tests.rs:857 [G2 PS2]: let root = scratch("cache.timing");
IGNORED crates/pull/src/gdfl_run_tests.rs:858 [G2 PS2]: let ids = [SourceId::CmZip, SourceId::NfoZips, SourceId::TickStore];
IGNORED crates/pull/src/gdfl_run_tests.rs:859 [G2 PS2]: let mut dirs = Vec::new();
IGNORED crates/pull/src/gdfl_run_tests.rs:860 [G2 PS2]: for id in ids {
IGNORED crates/pull/src/gdfl_run_tests.rs:861 [G2 PS2]: let dir = root.join(id.as_str());
IGNORED crates/pull/src/gdfl_run_tests.rs:862 [G2 PS2]: std::fs::create_dir_all(&dir).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:863 [G2 PS2]: std::fs::write(dir.join("FORMAT.md"), b"synthetic\n").unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:864 [G2 PS2]: dirs.push(dir);
IGNORED crates/pull/src/gdfl_run_tests.rs:865 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:866 [G2 PS2]: let mut cache = SourceCache::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:867 [G2 PS2]: for (id, dir) in ids.iter().zip(&dirs) {
IGNORED crates/pull/src/gdfl_run_tests.rs:868 [G2 PS2]: let _first = cache.origin(*id, dir, Some("abc"));
IGNORED crates/pull/src/gdfl_run_tests.rs:869 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:870 [G2 PS2]: let mut took = Vec::with_capacity(6_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:871 [G2 PS2]: for _ in 0..2_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:872 [G2 PS2]: for (id, dir) in ids.iter().zip(&dirs) {
IGNORED crates/pull/src/gdfl_run_tests.rs:873 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:874 [G2 PS2]: let _hit = cache.origin(*id, dir, Some("abc"));
IGNORED crates/pull/src/gdfl_run_tests.rs:875 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:876 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:878 [G2 PS2]: assert_eq!(cache.measures(), 3, "every call after the first three hit");
IGNORED crates/pull/src/gdfl_run_tests.rs:879 [G2 PS2]: took.sort_unstable();
IGNORED crates/pull/src/gdfl_run_tests.rs:880 [G2 PS2]: let n = took.len();
IGNORED crates/pull/src/gdfl_run_tests.rs:881 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:883 [G2 PS2]: took[n / 2],
IGNORED crates/pull/src/gdfl_run_tests.rs:884 [G2 PS2]: took[n * 99 / 100],
IGNORED crates/pull/src/gdfl_run_tests.rs:885 [G2 PS2]: took[n - 1]
IGNORED crates/pull/src/gdfl_run_tests.rs:887 [G2 PS2]: std::fs::remove_dir_all(root).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:888 [G2 PS2]: }
F crates/pull/src/gdfl_run_tests.rs:1381:44 False [G2 PS2]: Refreshed::Reloaded { now, .. } if now == head.len() as u64
F crates/pull/src/gdfl_run_tests.rs:1466:13 False [G2 PS2]: let Ok(mut reader) = JournalReader::open(&store) else {
F crates/pull/src/gdfl_run_tests.rs:1467 [G2 PS2]: panic!("a prefix of a journal is a journal: cut at {cut_at}");
IGNORED crates/pull/src/gdfl_run_tests.rs:1497 [G2 PS2]: fn journal_load_and_refresh_costs() {
IGNORED crates/pull/src/gdfl_run_tests.rs:1498 [G2 PS2]: let store = scratch("costs");
IGNORED crates/pull/src/gdfl_run_tests.rs:1499 [G2 PS2]: let path = journal_path(&store);
IGNORED crates/pull/src/gdfl_run_tests.rs:1500 [G2 PS2]: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1501 [G2 PS2]: let names: Vec<String> = (0..60).map(|i| format!("SYMQ{i:04}")).collect();
IGNORED crates/pull/src/gdfl_run_tests.rs:1502 [G2 PS2]: let long_filter = names.join(",");
IGNORED crates/pull/src/gdfl_run_tests.rs:1505 [G2 PS2]: let runs: [(&str, usize); 5] = [
IGNORED crates/pull/src/gdfl_run_tests.rs:1506 [G2 PS2]: ("*", 9_180),
IGNORED crates/pull/src/gdfl_run_tests.rs:1507 [G2 PS2]: ("*", 91_800),
IGNORED crates/pull/src/gdfl_run_tests.rs:1508 [G2 PS2]: ("*", 918_000),
IGNORED crates/pull/src/gdfl_run_tests.rs:1509 [G2 PS2]: ("long", 23_270),
IGNORED crates/pull/src/gdfl_run_tests.rs:1510 [G2 PS2]: ("long", 232_700),
IGNORED crates/pull/src/gdfl_run_tests.rs:1511 [G2 PS2]: ];
IGNORED crates/pull/src/gdfl_run_tests.rs:1512 [G2 PS2]: for (filter, lines) in runs {
IGNORED crates/pull/src/gdfl_run_tests.rs:1513 [G2 PS2]: let line_of = |on: &str| {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514 [G2 PS2]: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514:16 False [G2 PS2]: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1514:16 True [G2 PS2]: if filter == "*" {
IGNORED crates/pull/src/gdfl_run_tests.rs:1515 [G2 PS2]: done(on)
IGNORED crates/pull/src/gdfl_run_tests.rs:1517 [G2 PS2]: format!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1522 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_run_tests.rs:1523 [G2 PS2]: let mut text = format!("{RUN} definition={BAR_DEFINITION}\n");
IGNORED crates/pull/src/gdfl_run_tests.rs:1524 [G2 PS2]: let mut on = day(1990, 1, 1);
IGNORED crates/pull/src/gdfl_run_tests.rs:1525 [G2 PS2]: for _ in 0..lines {
IGNORED crates/pull/src/gdfl_run_tests.rs:1526 [G2 PS2]: text.push_str(&line_of(&on.to_string()));
IGNORED crates/pull/src/gdfl_run_tests.rs:1527 [G2 PS2]: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1528 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1529 [G2 PS2]: std::fs::write(&path, &text).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1530 [G2 PS2]: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1530:20 False [G2 PS2]: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1530:20 True [G2 PS2]: let n = if text.len() > 32 << 20 { 20 } else { 200 };
IGNORED crates/pull/src/gdfl_run_tests.rs:1531 [G2 PS2]: let mut loads = Vec::with_capacity(n);
IGNORED crates/pull/src/gdfl_run_tests.rs:1532 [G2 PS2]: for _ in 0..n {
IGNORED crates/pull/src/gdfl_run_tests.rs:1533 [G2 PS2]: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1534 [G2 PS2]: let _reader = JournalReader::open(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1535 [G2 PS2]: loads.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1536 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1537 [G2 PS2]: let mut reader = JournalReader::open(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1538 [G2 PS2]: let mut refreshes = Vec::with_capacity(200);
IGNORED crates/pull/src/gdfl_run_tests.rs:1539 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1542 [G2 PS2]: let mut file = std::fs::OpenOptions::new()
IGNORED crates/pull/src/gdfl_run_tests.rs:1543 [G2 PS2]: .append(true)
IGNORED crates/pull/src/gdfl_run_tests.rs:1544 [G2 PS2]: .open(&path)
IGNORED crates/pull/src/gdfl_run_tests.rs:1545 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1546 [G2 PS2]: file.write_all(line_of(&on.to_string()).as_bytes()).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1548 [G2 PS2]: on = Day::from_days(on.days_from_epoch() + 1).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1549 [G2 PS2]: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1550 [G2 PS2]: assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
IGNORED crates/pull/src/gdfl_run_tests.rs:1551 [G2 PS2]: refreshes.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1553 [G2 PS2]: let pct = |v: &mut Vec<std::time::Duration>| {
IGNORED crates/pull/src/gdfl_run_tests.rs:1554 [G2 PS2]: v.sort();
IGNORED crates/pull/src/gdfl_run_tests.rs:1555 [G2 PS2]: let at = |q: usize| v[(v.len() * q / 100).min(v.len() - 1)];
IGNORED crates/pull/src/gdfl_run_tests.rs:1556 [G2 PS2]: (at(50), at(99), v[v.len() - 1], v.len())
IGNORED crates/pull/src/gdfl_run_tests.rs:1557 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_run_tests.rs:1558 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1560 [G2 PS2]: text.len(),
IGNORED crates/pull/src/gdfl_run_tests.rs:1561 [G2 PS2]: pct(&mut loads),
IGNORED crates/pull/src/gdfl_run_tests.rs:1562 [G2 PS2]: pct(&mut refreshes)
IGNORED crates/pull/src/gdfl_run_tests.rs:1565 [G2 PS2]: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1565:12 False [G2 PS2]: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1565:12 True [G2 PS2]: if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
IGNORED crates/pull/src/gdfl_run_tests.rs:1566 [G2 PS2]: std::fs::copy(sample, &path).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1567 [G2 PS2]: let mut loads = Vec::with_capacity(50);
IGNORED crates/pull/src/gdfl_run_tests.rs:1568 [G2 PS2]: let mut loaded = 0;
IGNORED crates/pull/src/gdfl_run_tests.rs:1569 [G2 PS2]: for _ in 0..50 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1570 [G2 PS2]: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1571 [G2 PS2]: loaded = JournalReader::open(&store).unwrap().loaded();
IGNORED crates/pull/src/gdfl_run_tests.rs:1572 [G2 PS2]: loads.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1573 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1574 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1576 [G2 PS2]: spread(&mut loads)
IGNORED crates/pull/src/gdfl_run_tests.rs:1578 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1579 [G2 PS2]: std::fs::remove_dir_all(store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1580 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1583 [G2 PS2]: fn spread(samples: &mut [std::time::Duration]) -> String {
IGNORED crates/pull/src/gdfl_run_tests.rs:1584 [G2 PS2]: samples.sort();
IGNORED crates/pull/src/gdfl_run_tests.rs:1585 [G2 PS2]: let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
IGNORED crates/pull/src/gdfl_run_tests.rs:1586 [G2 PS2]: format!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1588 [G2 PS2]: at(50),
IGNORED crates/pull/src/gdfl_run_tests.rs:1589 [G2 PS2]: at(99),
IGNORED crates/pull/src/gdfl_run_tests.rs:1590 [G2 PS2]: samples[samples.len() - 1],
IGNORED crates/pull/src/gdfl_run_tests.rs:1591 [G2 PS2]: samples.len()
IGNORED crates/pull/src/gdfl_run_tests.rs:1593 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1601 [G2 PS2]: fn progress_and_failure_costs() {
IGNORED crates/pull/src/gdfl_run_tests.rs:1602 [G2 PS2]: let root = scratch("write-costs");
IGNORED crates/pull/src/gdfl_run_tests.rs:1603 [G2 PS2]: let store = root.join("store");
IGNORED crates/pull/src/gdfl_run_tests.rs:1604 [G2 PS2]: std::fs::create_dir_all(&store).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1605 [G2 PS2]: let log = installed();
IGNORED crates/pull/src/gdfl_run_tests.rs:1606 [G2 PS2]: let planned = plan(
IGNORED crates/pull/src/gdfl_run_tests.rs:1607 [G2 PS2]: request(&[ImportKind::Options]),
IGNORED crates/pull/src/gdfl_run_tests.rs:1608 [G2 PS2]: &process(&store, &log),
IGNORED crates/pull/src/gdfl_run_tests.rs:1609 [G2 PS2]: &env_with(vec![(TICKSTORE_VAR, root.clone())]),
IGNORED crates/pull/src/gdfl_run_tests.rs:1610 [G2 PS2]: &mut SourceCache::default(),
IGNORED crates/pull/src/gdfl_run_tests.rs:1612 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1613 [G2 PS2]: let progress = ProgressFile::new(
IGNORED crates/pull/src/gdfl_run_tests.rs:1614 [G2 PS2]: store.join("imports").join("gdfl.options.progress"),
IGNORED crates/pull/src/gdfl_run_tests.rs:1615 [G2 PS2]: &planned,
IGNORED crates/pull/src/gdfl_run_tests.rs:1616 [G2 PS2]: &planned.kinds[0],
IGNORED crates/pull/src/gdfl_run_tests.rs:1619 [G2 PS2]: let mut report = Report::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:1620 [G2 PS2]: let mut writes = Vec::with_capacity(1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1621 [G2 PS2]: for closed in 1..=1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1622 [G2 PS2]: report.days_imported = closed;
IGNORED crates/pull/src/gdfl_run_tests.rs:1623 [G2 PS2]: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1624 [G2 PS2]: progress.day_passed(ImportKind::Options, day(2024, 4, 1), &report);
IGNORED crates/pull/src/gdfl_run_tests.rs:1625 [G2 PS2]: writes.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1626 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1627 [G2 PS2]: assert_eq!(progress.writes(), 1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1628 [G2 PS2]: println!(
IGNORED crates/pull/src/gdfl_run_tests.rs:1630 [G2 PS2]: spread(&mut writes)
IGNORED crates/pull/src/gdfl_run_tests.rs:1632 [G2 PS2]: let cause = Cause::open(Stage::Vendor, "malformed_row");
IGNORED crates/pull/src/gdfl_run_tests.rs:1633 [G2 PS2]: let mut named = Report::default();
IGNORED crates/pull/src/gdfl_run_tests.rs:1634 [G2 PS2]: let mut said = String::new();
IGNORED crates/pull/src/gdfl_run_tests.rs:1635 [G2 PS2]: for total in [FAILURES_KEPT, 10 * FAILURES_KEPT, 100 * FAILURES_KEPT] {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636 [G2 PS2]: while named.failures_total < total - 1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636:15 False [G2 PS2]: while named.failures_total < total - 1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1636:15 True [G2 PS2]: while named.failures_total < total - 1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1637 [G2 PS2]: named.name(cause, failure(named.failures_total));
IGNORED crates/pull/src/gdfl_run_tests.rs:1638 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1639 [G2 PS2]: let mut each = Vec::with_capacity(1_000);
IGNORED crates/pull/src/gdfl_run_tests.rs:1640 [G2 PS2]: for _ in 0..1_000 {
IGNORED crates/pull/src/gdfl_run_tests.rs:1641 [G2 PS2]: let one = failure(named.failures_total);
IGNORED crates/pull/src/gdfl_run_tests.rs:1642 [G2 PS2]: let at = Instant::now();
IGNORED crates/pull/src/gdfl_run_tests.rs:1643 [G2 PS2]: named.name(cause, one);
IGNORED crates/pull/src/gdfl_run_tests.rs:1644 [G2 PS2]: each.push(at.elapsed());
IGNORED crates/pull/src/gdfl_run_tests.rs:1645 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_run_tests.rs:1646 [G2 PS2]: assert!(named.failures.len() <= 2 * FAILURES_KEPT);
IGNORED crates/pull/src/gdfl_run_tests.rs:1647 [G2 PS2]: let _ = std::fmt::Write::write_fmt(
IGNORED crates/pull/src/gdfl_run_tests.rs:1648 [G2 PS2]: &mut said,
IGNORED crates/pull/src/gdfl_run_tests.rs:1649 [G2 PS2]: format_args!("at {total} named: {}; ", spread(&mut each)),
IGNORED crates/pull/src/gdfl_run_tests.rs:1650 [G2 PS2]: );
IGNORED crates/pull/src/gdfl_run_tests.rs:1652 [G2 PS2]: println!("C-GR-06 Report::name: {said}");
IGNORED crates/pull/src/gdfl_run_tests.rs:1653 [G2 PS2]: std::fs::remove_dir_all(root).unwrap();
IGNORED crates/pull/src/gdfl_run_tests.rs:1654 [G2 PS2]: }
F crates/pull/src/gdfl_scope_tests.rs:217 [G2 PS2]: path.display()
F crates/pull/src/gdfl_scope_tests.rs:575:13 False [G2 PS2]: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
F crates/pull/src/gdfl_scope_tests.rs:575:45 False [G2 PS2]: said.contains("gdfl.scopes") && said.contains("not a scope ledger line"),
F crates/pull/src/gdfl_scope_tests.rs:745 [G2 PS2]: after.keys().collect::<Vec<_>>()
F crates/pull/src/gdfl_scope_tests.rs:749:9 False [G2 PS2]: nifty.len() > before.len() && nifty.starts_with(&before),
F crates/pull/src/gdfl_scope_tests.rs:749:39 False [G2 PS2]: nifty.len() > before.len() && nifty.starts_with(&before),
F crates/pull/src/gdfl_scope_tests.rs:1273:9 False [G2 PS2]: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
F crates/pull/src/gdfl_scope_tests.rs:1273:47 False [G2 PS2]: closes[2].contains(" failures=0 ") && !closes[2].contains("vendor_refused"),
F crates/pull/src/gdfl_scope_tests.rs:1275 [G2 PS2]: closes[2]
F crates/pull/src/gdfl_scope_tests.rs:1693:13 False [G2 PS2]: let Owed::Probe(beyond) = owed else {
F crates/pull/src/gdfl_scope_tests.rs:1694 [G2 PS2]: panic!("{d}: a close with no scope owes the members beyond the swept surface");
F crates/pull/src/gdfl_scope_tests.rs:1708:13 False [G2 PS2]: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
F crates/pull/src/gdfl_scope_tests.rs:1708:54 False [G2 PS2]: assert!(beyond.contains("NSE-CASH-SUZLON.BE") && !beyond.contains("NSE-CASH-SUZLON"));
F crates/pull/src/gdfl_scope_tests.rs:2059:13 False [G2 PS2]: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
F crates/pull/src/gdfl_scope_tests.rs:2059:48 False [G2 PS2]: assert!(!share_held(&store, "RELIANCE") && !share_held(&store, "HDFCBANK"));
F crates/pull/src/gdfl_scope_tests.rs:2078:13 False [G2 PS2]: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
F crates/pull/src/gdfl_scope_tests.rs:2078:43 False [G2 PS2]: assert!(share_held(&store, "INFY") && !share_held(&store, "RELIANCE"));
F crates/pull/src/gdfl_scope_tests.rs:2231 [G2 PS2]: journal_text(&store)
F crates/pull/src/gdfl_scope_tests.rs:2283:17 False [G2 PS2]: names_restated(&report) && report.files == 0 && report.seconds == 0,
F crates/pull/src/gdfl_scope_tests.rs:2283:44 False [G2 PS2]: names_restated(&report) && report.files == 0 && report.seconds == 0,
F crates/pull/src/gdfl_scope_tests.rs:2283:65 False [G2 PS2]: names_restated(&report) && report.files == 0 && report.seconds == 0,
F crates/pull/src/gdfl_scope_tests.rs:2287 [G2 PS2]: journal_text(&store)
F crates/pull/src/gdfl_scope_tests.rs:2450 [G2 PS2]: journal_text(&store)
F crates/pull/src/gdfl_scope_tests.rs:2551 [G2 PS2]: journal_text(&store)
F crates/pull/src/gdfl_sealed_tests.rs:51:29 False [G2 PS2]: parts.len() == 7 || parts.len() == 8,
F crates/pull/src/gdfl_sealed_tests.rs:132:17 False [G2 PS2]: assert!(got.seconds > 0 && got.files > 0);
F crates/pull/src/gdfl_sealed_tests.rs:132:36 False [G2 PS2]: assert!(got.seconds > 0 && got.files > 0);
F crates/pull/src/gdfl_seconds_attack_tests.rs:147:27 False [G2 PS2]: assert!(k == 0 || placed[k - 1].0 <= *s, "placement runs forward");
F crates/pull/src/gdfl_seconds_attack_tests.rs:344 [G2 PS2]: (got, want) => panic!("case {case} {kind:?} {ticks:?}: {got:?} against {want:?}"),
F crates/pull/src/gdfl_seconds_attack_tests.rs:349:9 False [G2 PS2]: refused > 100 && agreed > 90_000 && bars > 200_000,
F crates/pull/src/gdfl_seconds_attack_tests.rs:349:26 False [G2 PS2]: refused > 100 && agreed > 90_000 && bars > 200_000,
F crates/pull/src/gdfl_seconds_attack_tests.rs:349:45 False [G2 PS2]: refused > 100 && agreed > 90_000 && bars > 200_000,
F crates/pull/src/gdfl_seconds_attack_tests.rs:832 [G2 PS2]: matches!(
F crates/pull/src/gdfl_seconds_attack_tests.rs:966:13 False [G2 PS2]: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
F crates/pull/src/gdfl_seconds_attack_tests.rs:966:25 False [G2 PS2]: assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
F crates/pull/src/gdfl_seconds_attack_tests.rs:976:17 False [G2 PS2]: assert!(r.ltp >= 5 && r.sod < 86_400);
F crates/pull/src/gdfl_seconds_attack_tests.rs:976:31 False [G2 PS2]: assert!(r.ltp >= 5 && r.sod < 86_400);
F crates/pull/src/gdfl_seconds_attack_tests.rs:1066:9 False [G2 PS2]: listed > 1_000 && rebuilt_clean > 1_000,
F crates/pull/src/gdfl_seconds_attack_tests.rs:1066:27 False [G2 PS2]: listed > 1_000 && rebuilt_clean > 1_000,
F crates/pull/src/gdfl_seconds_attack_tests.rs:1382:13 False [G2 PS2]: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
F crates/pull/src/gdfl_seconds_attack_tests.rs:1382:29 False [G2 PS2]: assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
F crates/pull/src/gdfl_seconds_attack_tests.rs:1666:13 False [G2 PS2]: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
F crates/pull/src/gdfl_seconds_attack_tests.rs:1666:44 False [G2 PS2]: assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
F crates/pull/src/gdfl_series_code_tests.rs:234:58 False [G3 PS3]: Some(CmRefusal::SeriesNotStored { code, .. }) if *code == *edge
F crates/pull/src/gdfl_series_code_tests.rs:238:59 False [G3 PS3]: Some(CmRefusal::SeriesNotStored { share, .. }) if &*share == widest
F crates/pull/src/gdfl_tickstore.rs:1724:28 False [G3 PS3]: if what.contains(named)),
F crates/pull/src/gdfl_tickstore.rs:1810 [G3 PS3]: frame.len()
F crates/pull/src/gdfl_v3_tests.rs:206:9 False [G2 PS2]: let V3::Disagrees { count, first } = v3.check(d, Some(&files_only)) else {
F crates/pull/src/gdfl_v3_tests.rs:207 [G2 PS2]: panic!("a listing without its directories agreed");
F crates/pull/src/gdfl_v3_tests.rs:220:9 False [G2 PS2]: let V3Ledger::At { root } = &v3 else {
F crates/pull/src/gdfl_v3_tests.rs:221 [G2 PS2]: panic!("a configured ledger is unset");
F crates/pull/src/gdfl_v3_tests.rs:303:9 False [G2 PS2]: let V3::Disagrees { count, first } = check(&both, Some(99)) else {
F crates/pull/src/gdfl_v3_tests.rs:304 [G2 PS2]: panic!("two differences agreed");
F crates/pull/src/gdfl_v3_tests.rs:310:9 False [G2 PS2]: let V3::Disagrees { count, first } = check(&renamed, Some(99)) else {
F crates/pull/src/gdfl_v3_tests.rs:311 [G2 PS2]: panic!("a renamed path agreed");
F crates/pull/src/gdfl_v3_tests.rs:336:9 False [G2 PS2]: let V3::Disagrees { count, first } = check(&longer, Some(99)) else {
F crates/pull/src/gdfl_v3_tests.rs:337 [G2 PS2]: panic!("a longer listing agreed");
F crates/pull/src/gdfl_v3_tests.rs:464:9 False [G2 PS2]: let V3::Unverified { why } = dir_marker.check(d, Some(&good)) else {
F crates/pull/src/gdfl_v3_tests.rs:465 [G2 PS2]: panic!("a directory at the marker's name was trusted");
F crates/pull/src/gdfl_v3_tests.rs:573:9 False [G2 PS2]: let V3::Unverified { why } = dir_manifest.check(d, Some(&good)) else {
F crates/pull/src/gdfl_v3_tests.rs:574 [G2 PS2]: panic!("a directory at the manifest's name was trusted");
F crates/pull/src/gdfl_v3_tests.rs:759:9 False [G2 PS2]: let V3::Disagrees { count, first } = v3.check(
F crates/pull/src/gdfl_v3_tests.rs:763 [G2 PS2]: panic!("an empty manifest agreed with an entry");
IGNORED crates/pull/src/gdfl_v3_tests.rs:864 [G2 PS2]: fn the_real_v3_ledger_agrees_with_the_real_year_zips() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:866 [G2 PS2]: let env = |var: &str| std::env::var_os(var);
IGNORED crates/pull/src/gdfl_v3_tests.rs:867 [G2 PS2]: let ledger = V3Ledger::from_env(&env);
IGNORED crates/pull/src/gdfl_v3_tests.rs:868 [G2 PS2]: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:868:9 False [G2 PS2]: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:868:9 True [G2 PS2]: let V3Ledger::At { root } = &ledger else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:869 [G2 PS2]: panic!("BRUTEX_GDFL_V3_LEDGER is unset");
IGNORED crates/pull/src/gdfl_v3_tests.rs:871 [G2 PS2]: let dir = std::path::PathBuf::from(env("BRUTEX_GDFL_NFO_ARCHIVES").expect("archives"));
IGNORED crates/pull/src/gdfl_v3_tests.rs:872 [G2 PS2]: let zips = NfoZips::new(&dir);
IGNORED crates/pull/src/gdfl_v3_tests.rs:874 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:875 [G2 PS2]: let first = zips.day(day(2019, 4, 1));
IGNORED crates/pull/src/gdfl_v3_tests.rs:876 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:878 [G2 PS2]: started.elapsed(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:879 [G2 PS2]: matches!(first, Ok(Some(_)))
IGNORED crates/pull/src/gdfl_v3_tests.rs:881 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:882 [G2 PS2]: let again = zips.day(day(2019, 4, 2));
IGNORED crates/pull/src/gdfl_v3_tests.rs:883 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:885 [G2 PS2]: started.elapsed(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:886 [G2 PS2]: matches!(again, Ok(Some(_)))
IGNORED crates/pull/src/gdfl_v3_tests.rs:888 [G2 PS2]: let parse = |text: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:889 [G2 PS2]: let parts: Vec<u16> = text.split('-').map(|p| p.parse().unwrap()).collect();
IGNORED crates/pull/src/gdfl_v3_tests.rs:890 [G2 PS2]: day(
IGNORED crates/pull/src/gdfl_v3_tests.rs:891 [G2 PS2]: parts[0],
IGNORED crates/pull/src/gdfl_v3_tests.rs:892 [G2 PS2]: u8::try_from(parts[1]).unwrap(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:893 [G2 PS2]: u8::try_from(parts[2]).unwrap(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:895 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:897 [G2 PS2]: for year in 2018..=2026_u16 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:898 [G2 PS2]: let count = |sub: &str, suffix: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:899 [G2 PS2]: std::fs::read_dir(root.join(format!("{year}/_v3/{sub}")))
IGNORED crates/pull/src/gdfl_v3_tests.rs:900 [G2 PS2]: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:901 [G2 PS2]: .filter(|name| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:902 [G2 PS2]: name.as_ref()
IGNORED crates/pull/src/gdfl_v3_tests.rs:903 [G2 PS2]: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:904 [G2 PS2]: .file_name()
IGNORED crates/pull/src/gdfl_v3_tests.rs:905 [G2 PS2]: .to_str()
IGNORED crates/pull/src/gdfl_v3_tests.rs:906 [G2 PS2]: .is_some_and(|n| n.ends_with(suffix))
IGNORED crates/pull/src/gdfl_v3_tests.rs:907 [G2 PS2]: })
IGNORED crates/pull/src/gdfl_v3_tests.rs:908 [G2 PS2]: .count()
IGNORED crates/pull/src/gdfl_v3_tests.rs:909 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:910 [G2 PS2]: let (manifests, markers) = (count("manifest", ".ok.tsv"), count("ok", ".ok"));
IGNORED crates/pull/src/gdfl_v3_tests.rs:911 [G2 PS2]: eprintln!("v3 {year}: manifests={manifests} ok_markers={markers}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:912 [G2 PS2]: assert_eq!(manifests, markers, "{year}: one ok marker a manifest");
IGNORED crates/pull/src/gdfl_v3_tests.rs:913 [G2 PS2]: assert!(manifests > 0, "{year}: no manifest");
IGNORED crates/pull/src/gdfl_v3_tests.rs:916 [G2 PS2]: let mut records = std::collections::HashMap::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:917 [G2 PS2]: for year in 2018..=2026_u16 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918 [G2 PS2]: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918:13 False [G2 PS2]: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:918:13 True [G2 PS2]: let Ok(file) = std::fs::File::open(dir.join(format!("{year}.zip"))) else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:919 [G2 PS2]: continue;
IGNORED crates/pull/src/gdfl_v3_tests.rs:921 [G2 PS2]: let len = file.metadata().unwrap().len();
IGNORED crates/pull/src/gdfl_v3_tests.rs:922 [G2 PS2]: for entry in crate::gdfl_archive::zip_entries(&file, 0, len).unwrap() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:923 [G2 PS2]: records.insert(entry.name.clone(), (entry.len, entry.crc32));
IGNORED crates/pull/src/gdfl_v3_tests.rs:924 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:926 [G2 PS2]: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:926:33 False [G2 PS2]: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:926:33 True [G2 PS2]: let days: Vec<Day> = if let Ok(list) = std::env::var("BRUTEX_V3_DAYS") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:927 [G2 PS2]: list.split(',').map(parse).collect()
IGNORED crates/pull/src/gdfl_v3_tests.rs:929 [G2 PS2]: let from = std::env::var("BRUTEX_V3_FROM").map_or(day(2018, 1, 1), |t| parse(&t));
IGNORED crates/pull/src/gdfl_v3_tests.rs:930 [G2 PS2]: let to = std::env::var("BRUTEX_V3_TO").map_or(day(2026, 12, 31), |t| parse(&t));
IGNORED crates/pull/src/gdfl_v3_tests.rs:931 [G2 PS2]: let mut out = Vec::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:932 [G2 PS2]: let mut at = from;
IGNORED crates/pull/src/gdfl_v3_tests.rs:933 [G2 PS2]: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:933:15 False [G2 PS2]: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:933:15 True [G2 PS2]: while at <= to {
IGNORED crates/pull/src/gdfl_v3_tests.rs:934 [G2 PS2]: out.push(at);
IGNORED crates/pull/src/gdfl_v3_tests.rs:935 [G2 PS2]: at = at.succ().unwrap();
IGNORED crates/pull/src/gdfl_v3_tests.rs:936 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:937 [G2 PS2]: out
IGNORED crates/pull/src/gdfl_v3_tests.rs:939 [G2 PS2]: let (mut codes, mut held, mut marker_agrees, mut marker_differs) =
IGNORED crates/pull/src/gdfl_v3_tests.rs:940 [G2 PS2]: ([0_usize; 4], 0_usize, 0_usize, 0_usize);
IGNORED crates/pull/src/gdfl_v3_tests.rs:941 [G2 PS2]: let mut nanos: Vec<u128> = Vec::new();
IGNORED crates/pull/src/gdfl_v3_tests.rs:942 [G2 PS2]: let mut lines = 0_usize;
IGNORED crates/pull/src/gdfl_v3_tests.rs:943 [G2 PS2]: for d in days {
IGNORED crates/pull/src/gdfl_v3_tests.rs:944 [G2 PS2]: let listing = match zips.day(d) {
IGNORED crates/pull/src/gdfl_v3_tests.rs:945 [G2 PS2]: Ok(listing) => listing,
IGNORED crates/pull/src/gdfl_v3_tests.rs:946 [G2 PS2]: Err(why) => {
IGNORED crates/pull/src/gdfl_v3_tests.rs:947 [G2 PS2]: eprintln!("v3 {d}: source refused: {why}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:948 [G2 PS2]: continue;
IGNORED crates/pull/src/gdfl_v3_tests.rs:951 [G2 PS2]: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:951:12 False [G2 PS2]: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:951:12 True [G2 PS2]: if listing.is_some() {
IGNORED crates/pull/src/gdfl_v3_tests.rs:952 [G2 PS2]: held += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:953 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:954 [G2 PS2]: let started = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_v3_tests.rs:955 [G2 PS2]: let answer = ledger.check(d, listing.as_ref());
IGNORED crates/pull/src/gdfl_v3_tests.rs:956 [G2 PS2]: let took = started.elapsed().as_nanos();
IGNORED crates/pull/src/gdfl_v3_tests.rs:957 [G2 PS2]: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:12 False [G2 PS2]: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:12 True [G2 PS2]: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:33 False [G2 PS2]: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:957:33 True [G2 PS2]: if listing.is_none() && answer.code() == 0 {
IGNORED crates/pull/src/gdfl_v3_tests.rs:958 [G2 PS2]: continue; // neither holds the day: a weekend or a holiday
IGNORED crates/pull/src/gdfl_v3_tests.rs:959 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:960 [G2 PS2]: nanos.push(took);
IGNORED crates/pull/src/gdfl_v3_tests.rs:961 [G2 PS2]: codes[usize::from(answer.code())] += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:962 [G2 PS2]: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:962:16 False [G2 PS2]: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:962:16 True [G2 PS2]: if let V3::Agrees { lines: n } = answer {
IGNORED crates/pull/src/gdfl_v3_tests.rs:963 [G2 PS2]: lines += n;
IGNORED crates/pull/src/gdfl_v3_tests.rs:964 [G2 PS2]: } else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:965 [G2 PS2]: eprintln!("v3 {d}: {answer}");
IGNORED crates/pull/src/gdfl_v3_tests.rs:966 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:968 [G2 PS2]: let name = NfoZips::day_zip_name(d);
IGNORED crates/pull/src/gdfl_v3_tests.rs:969 [G2 PS2]: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:969:16 False [G2 PS2]: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:969:16 True [G2 PS2]: if let (Ok(text), Some(&(len, crc))) = (
IGNORED crates/pull/src/gdfl_v3_tests.rs:970 [G2 PS2]: std::fs::read_to_string(ledger.marker_path(d).unwrap()),
IGNORED crates/pull/src/gdfl_v3_tests.rs:971 [G2 PS2]: records.get(&name),
IGNORED crates/pull/src/gdfl_v3_tests.rs:973 [G2 PS2]: let value = |key: &str| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:974 [G2 PS2]: text.lines()
IGNORED crates/pull/src/gdfl_v3_tests.rs:975 [G2 PS2]: .find_map(|line| line.strip_prefix(&format!("{key}=")))
IGNORED crates/pull/src/gdfl_v3_tests.rs:976 [G2 PS2]: .unwrap()
IGNORED crates/pull/src/gdfl_v3_tests.rs:977 [G2 PS2]: .to_owned()
IGNORED crates/pull/src/gdfl_v3_tests.rs:978 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:979 [G2 PS2]: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:16 False [G2 PS2]: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:16 True [G2 PS2]: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:52 False [G2 PS2]: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:979:52 True [G2 PS2]: if value("size") == len.to_string() && value("crc") == format!("{crc:08x}") {
IGNORED crates/pull/src/gdfl_v3_tests.rs:980 [G2 PS2]: marker_agrees += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:981 [G2 PS2]: } else {
IGNORED crates/pull/src/gdfl_v3_tests.rs:982 [G2 PS2]: marker_differs += 1;
IGNORED crates/pull/src/gdfl_v3_tests.rs:983 [G2 PS2]: eprintln!("v3 {d}: the ok marker's crc/size are not the day zip's");
IGNORED crates/pull/src/gdfl_v3_tests.rs:984 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:985 [G2 PS2]: }
IGNORED crates/pull/src/gdfl_v3_tests.rs:987 [G2 PS2]: nanos.sort_unstable();
IGNORED crates/pull/src/gdfl_v3_tests.rs:988 [G2 PS2]: let at = |q: usize| {
IGNORED crates/pull/src/gdfl_v3_tests.rs:989 [G2 PS2]: nanos
IGNORED crates/pull/src/gdfl_v3_tests.rs:990 [G2 PS2]: .get((nanos.len().saturating_sub(1)) * q / 100)
IGNORED crates/pull/src/gdfl_v3_tests.rs:991 [G2 PS2]: .copied()
IGNORED crates/pull/src/gdfl_v3_tests.rs:992 [G2 PS2]: .unwrap_or(0)
IGNORED crates/pull/src/gdfl_v3_tests.rs:993 [G2 PS2]: };
IGNORED crates/pull/src/gdfl_v3_tests.rs:994 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:996 [G2 PS2]: codes[0], codes[1], codes[2], codes[3]
IGNORED crates/pull/src/gdfl_v3_tests.rs:998 [G2 PS2]: eprintln!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:1000 [G2 PS2]: nanos.len(),
IGNORED crates/pull/src/gdfl_v3_tests.rs:1001 [G2 PS2]: at(50) / 1_000,
IGNORED crates/pull/src/gdfl_v3_tests.rs:1002 [G2 PS2]: at(99) / 1_000,
IGNORED crates/pull/src/gdfl_v3_tests.rs:1003 [G2 PS2]: nanos.last().copied().unwrap_or(0) / 1_000
IGNORED crates/pull/src/gdfl_v3_tests.rs:1005 [G2 PS2]: assert_eq!(
IGNORED crates/pull/src/gdfl_v3_tests.rs:1006 [G2 PS2]: codes[2] + codes[3],
IGNORED crates/pull/src/gdfl_v3_tests.rs:1010 [G2 PS2]: assert_eq!(marker_differs, 0);
IGNORED crates/pull/src/gdfl_v3_tests.rs:1011 [G2 PS2]: }
F crates/pull/src/gdfl_wp12e_tests.rs:613:57 False [G3 PS3]: Err(ImportRefusal::UnderlyingSplit { held }) if held == [("TATAMOTORS".to_owned(), "TMPV".to_owned())]
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:725 [G3 PS3]: fn spread(samples: &mut [u128]) -> (u128, u128, u128) {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:726 [G3 PS3]: samples.sort_unstable();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:727 [G3 PS3]: let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:728 [G3 PS3]: (at(50), at(99), samples[samples.len() - 1])
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:729 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:733 [G3 PS3]: fn synthetic_ledger(events: usize, ex: Day, expiry: Day) -> String {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:734 [G3 PS3]: let mut text = String::new();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:735 [G3 PS3]: for u in 0..events {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:736 [G3 PS3]: let underlying = sym(&format!("ZZ{u:05}"));
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:737 [G3 PS3]: let key = |strike: i64| {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:738 [G3 PS3]: cd(expiry, strike, OptionSide::Call, 1, 0, 0)
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:739 [G3 PS3]: .contract()
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:740 [G3 PS3]: .unwrap()
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:741 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:742 [G3 PS3]: for i in 0..40_i64 {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:743 [G3 PS3]: let record = crate::gdfl_lineage::LineageRecord {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:744 [G3 PS3]: underlying,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:745 [G3 PS3]: old: key(20_000 + i * 500),
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:746 [G3 PS3]: new: key(20_000 + i * 500 - 900),
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:747 [G3 PS3]: ex_date: ex,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:748 [G3 PS3]: factor: (1, 1),
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:749 [G3 PS3]: lot_old: 3_400,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:750 [G3 PS3]: lot_new: 3_400,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:751 [G3 PS3]: oi_unadjusted_until: 0,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:752 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:753 [G3 PS3]: text.push_str(&record.line());
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:754 [G3 PS3]: text.push('\n');
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:755 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:756 [G3 PS3]: for i in 0..10_i64 {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:757 [G3 PS3]: let record = StoppedRecord {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:758 [G3 PS3]: underlying,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:759 [G3 PS3]: key: key(90_000 + i * 500),
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:760 [G3 PS3]: ex_date: ex,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:761 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:762 [G3 PS3]: text.push_str(&record.line());
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:763 [G3 PS3]: text.push('\n');
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:764 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:765 [G3 PS3]: let record = EventRecord {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:766 [G3 PS3]: underlying,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:767 [G3 PS3]: ex_date: ex,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:768 [G3 PS3]: kind: EventKind::StrikeOffset { shift: 900 },
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:769 [G3 PS3]: lot_old: 3_400,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:770 [G3 PS3]: lot_new: 3_400,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:771 [G3 PS3]: paired: 40,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:772 [G3 PS3]: stopped: 10,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:773 [G3 PS3]: };
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:774 [G3 PS3]: text.push_str(&record.line());
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:775 [G3 PS3]: text.push('\n');
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:777 [G3 PS3]: text
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:778 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:788 [G3 PS3]: fn the_ledger_load_and_a_resumed_day_are_timed() {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:789 [G3 PS3]: let (ex, later, expiry) = (d(2020, 2, 20), d(2020, 3, 11), d(2020, 3, 26));
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:790 [G3 PS3]: let mut folds = Vec::new();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:791 [G3 PS3]: for events in [200_usize, 2_000] {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:792 [G3 PS3]: let store = scratch("wp12e-timed");
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:793 [G3 PS3]: let text = synthetic_ledger(events, ex, expiry);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:794 [G3 PS3]: let lines = text.lines().count();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:795 [G3 PS3]: let path = lineage_path(&store);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:796 [G3 PS3]: std::fs::create_dir_all(path.parent().unwrap()).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:797 [G3 PS3]: std::fs::write(&path, &text).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:799 [G3 PS3]: let mut loads: Vec<u128> = (0..30)
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:800 [G3 PS3]: .map(|_| {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:801 [G3 PS3]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:802 [G3 PS3]: let run = LineageRun::load(&store, later).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:803 [G3 PS3]: let took = at.elapsed().as_micros();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:804 [G3 PS3]: assert!(run.last_day().is_none());
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:805 [G3 PS3]: took
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:806 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:807 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:808 [G3 PS3]: let (p50, p99, max) = spread(&mut loads);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:809 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:816 [G3 PS3]: let day_files: Vec<ContractDay> = (0..400_i64)
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:817 [G3 PS3]: .map(|i| cd(expiry, 20_000 + i * 500, OptionSide::Call, 3_400, 10, 20))
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:818 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:819 [G3 PS3]: let mut run = LineageRun::load(&store, later).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:820 [G3 PS3]: let first = fold(&mut run, later, "ZZ00007", &day_files);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:821 [G3 PS3]: assert!(first.lineage >= 50, "the 50 reused texts are stitched");
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:822 [G3 PS3]: let contracts = files("ZZ00007", &day_files);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:823 [G3 PS3]: let mut days: Vec<u128> = (0..500)
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:824 [G3 PS3]: .map(|_| {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:825 [G3 PS3]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:826 [G3 PS3]: let written = run.day(later, &contracts, &mut |_| Ok(Vec::new())).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:827 [G3 PS3]: let took = at.elapsed().as_micros();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:828 [G3 PS3]: assert_eq!((written.lineage, written.lots), (0, 0));
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:829 [G3 PS3]: took
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:830 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:831 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:832 [G3 PS3]: let (p50, p99, max) = spread(&mut days);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:833 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:836 [G3 PS3]: folds.push(p50);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:837 [G3 PS3]: std::fs::remove_dir_all(store).unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:839 [G3 PS3]: assert!(
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:840 [G3 PS3]: folds[1] <= folds[0].max(1) * 3,
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:843 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:850 [G3 PS3]: fn the_split_probe_is_timed() {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:851 [G3 PS3]: let later = d(2020, 9, 16);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:852 [G3 PS3]: let real = std::env::var_os("BRUTEX_WP12E_STORE").map(std::path::PathBuf::from);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:853 [G3 PS3]: let store = real.clone().unwrap_or_else(|| scratch("wp12e-timed-split"));
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:854 [G3 PS3]: let run = options_run(&store, later, &[]);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:855 [G3 PS3]: let held = run.split_underlyings().unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:856 [G3 PS3]: let mut probes: Vec<u128> = (0..200)
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:857 [G3 PS3]: .map(|_| {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:858 [G3 PS3]: let at = std::time::Instant::now();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:859 [G3 PS3]: let got = run.split_underlyings().unwrap();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:860 [G3 PS3]: let took = at.elapsed().as_micros();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:861 [G3 PS3]: assert_eq!(got, held);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:862 [G3 PS3]: took
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:863 [G3 PS3]: })
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:864 [G3 PS3]: .collect();
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:865 [G3 PS3]: let (p50, p99, max) = spread(&mut probes);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:866 [G3 PS3]: println!(
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:868 [G3 PS3]: store.display(),
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:869 [G3 PS3]: held.len()
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:871 [G3 PS3]: if real.is_none() {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:871:8 False [G3 PS3]: if real.is_none() {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:871:8 True [G3 PS3]: if real.is_none() {
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:872 [G3 PS3]: let _ = std::fs::remove_dir_all(store);
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:873 [G3 PS3]: }
IGNORED crates/pull/src/gdfl_wp12e_tests.rs:874 [G3 PS3]: }
U crates/pull/src/http.rs:3742 [G3 PS3]: detail: format!("bar {i} has no cell {at}"),
U crates/pull/src/http.rs:3743 [G3 PS3]: })
U crates/pull/src/http.rs:3913 [G3 PS3]: detail: format!("bar {at} stamps {text:?}, whose offset overflows the epoch"),
U crates/pull/src/http.rs:3914 [G3 PS3]: })
F crates/pull/src/http.rs:4144:17 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = feed.descriptor().transport else {
F crates/pull/src/http.rs:4145 [G3 PS3]: panic!("{feed:?} is an HTTP feed");
F crates/pull/src/http.rs:4166:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:4168 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:4190:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:4192 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:4211 [G3 PS3]: panic!("request {} in one second was admitted", per_second + 1)
F crates/pull/src/http.rs:4255:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:4257 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:4329:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:4331 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:4371:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:4373 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:4572:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = refused else {
F crates/pull/src/http.rs:4573 [G3 PS3]: panic!("a repeated key must be refused: {refused:?}");
F crates/pull/src/http.rs:4576:13 False [G3 PS3]: detail.contains("repeats") && detail.contains("open"),
F crates/pull/src/http.rs:4576:43 False [G3 PS3]: detail.contains("repeats") && detail.contains("open"),
F crates/pull/src/http.rs:4902:17 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:4907 [G3 PS3]: panic!("{sent} is not zero and must not be stored as one")
F crates/pull/src/http.rs:5237 [G3 PS3]: panic!("{name} ({volume}, {interest}): one row goes, not the window: {why}")
F crates/pull/src/http.rs:5511:13 False [G3 PS3]: said.contains("no timestamp") && said.contains("a price below zero"),
F crates/pull/src/http.rs:5511:46 False [G3 PS3]: said.contains("no timestamp") && said.contains("a price below zero"),
F crates/pull/src/http.rs:5848 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:5858:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, named, .. }) =
F crates/pull/src/http.rs:5861 [G3 PS3]: panic!("a body naming DH-901 is a refusal whatever the status says")
F crates/pull/src/http.rs:5875:13 False [G3 PS3]: let Err(FetchError::VendorRefused { named, .. }) =
F crates/pull/src/http.rs:5878 [G3 PS3]: panic!("the sentence names a dead token")
F crates/pull/src/http.rs:5907 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:5961 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:6064 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:6172 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:6317:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:6322 [G3 PS3]: panic!("a missing timestamp array must refuse")
F crates/pull/src/http.rs:6403:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:6408 [G3 PS3]: panic!("the descriptor says top level, so `data` is not searched")
F crates/pull/src/http.rs:6427:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:6432 [G3 PS3]: panic!("the declared envelope is absent and must be named")
F crates/pull/src/http.rs:6439:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:6444 [G3 PS3]: panic!("an array is not an envelope")
F crates/pull/src/http.rs:6449:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:6454 [G3 PS3]: panic!("an empty object holds no envelope")
F crates/pull/src/http.rs:6480:13 False [G3 PS3]: let crate::vendor::Transport::Http(zerodha) =
F crates/pull/src/http.rs:6483 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/http.rs:6540:24 False [G3 PS3]: || zoneless.bars.first().map(|b| b.ts_micros)
F crates/pull/src/http.rs:6544 [G3 PS3]: zoneless.bars.len(),
F crates/pull/src/http.rs:6545 [G3 PS3]: landed.bars.len()
F crates/pull/src/http.rs:6601:13 False [G3 PS3]: let crate::vendor::Transport::Http(zerodha) =
F crates/pull/src/http.rs:6604 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/http.rs:6625 [G3 PS3]: matches!(
F crates/pull/src/http.rs:6638:13 False [G3 PS3]: let crate::vendor::Transport::Http(zerodha) =
F crates/pull/src/http.rs:6641 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/http.rs:6693:13 False [G3 PS3]: let crate::vendor::Transport::Http(zerodha) =
F crates/pull/src/http.rs:6696 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/http.rs:6698:13 False [G3 PS3]: let crate::vendor::Transport::Http(groww) =
F crates/pull/src/http.rs:6701 [G3 PS3]: panic!("Groww is an HTTP feed");
F crates/pull/src/http.rs:6884:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = &refused else {
F crates/pull/src/http.rs:6885 [G3 PS3]: panic!("{refused:?}")
F crates/pull/src/http.rs:6959:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
F crates/pull/src/http.rs:6960 [G3 PS3]: panic!("a redirect is a refusal this build reports, not one it hides")
F crates/pull/src/http.rs:7006:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
F crates/pull/src/http.rs:7007 [G3 PS3]: panic!("429 is a refusal the governor needs to see")
F crates/pull/src/http.rs:7050:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = outcome else {
F crates/pull/src/http.rs:7051 [G3 PS3]: panic!("an answer declaring more than the cap is refused, not decoded")
F crates/pull/src/http.rs:7092:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:7095 [G3 PS3]: panic!("an answer past the cap is refused, not decoded")
F crates/pull/src/http.rs:7159 [G3 PS3]: Ok(body) => panic!(
F crates/pull/src/http.rs:7162 [G3 PS3]: body.len()
F crates/pull/src/http.rs:7168:13 False [G3 PS3]: refusal.detail.contains("ceiling")
F crates/pull/src/http.rs:7169:20 False [G3 PS3]: && refusal.detail.contains(&MAX_RESPONSE_BYTES.to_string()),
F crates/pull/src/http.rs:7254 [G3 PS3]: Ok(body) => panic!("an invalid byte is refused, not replaced: {body:?}"),
F crates/pull/src/http.rs:7416 [G3 PS3]: allowance(&held)
F crates/pull/src/http.rs:7478:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) =
F crates/pull/src/http.rs:7481 [G3 PS3]: panic!("500 is a refusal, however long its body")
F crates/pull/src/http.rs:7496 [G3 PS3]: detail.chars().count()
F crates/pull/src/http.rs:7676:13 False [G3 PS3]: let FetchError::TransportFailed { detail } = why else {
F crates/pull/src/http.rs:7677 [G3 PS3]: panic!("the variant a bad field already uses, not a new one")
F crates/pull/src/http.rs:7948:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
F crates/pull/src/http.rs:7949 [G3 PS3]: panic!("500 is a refusal")
F crates/pull/src/http.rs:7971:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) =
F crates/pull/src/http.rs:7974 [G3 PS3]: panic!("a 307 is still a redirect this build refuses")
F crates/pull/src/http.rs:8002:13 False [G3 PS3]: let Err(FetchError::VendorRefused { status, detail, .. }) =
F crates/pull/src/http.rs:8005 [G3 PS3]: panic!("a 301 is still a redirect this build refuses")
F crates/pull/src/http.rs:8026:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:8029 [G3 PS3]: panic!("an empty object holds no bars and must be refused")
F crates/pull/src/http.rs:8049:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = source.window(&one_day()) else {
F crates/pull/src/http.rs:8050 [G3 PS3]: panic!("the sync seam cannot work and must say so")
F crates/pull/src/http.rs:8084:17 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) = decode_body(
F crates/pull/src/http.rs:8089 [G3 PS3]: panic!("{bad} is not a whole count and must be refused, not truncated")
F crates/pull/src/http.rs:8106:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:8109 [G3 PS3]: panic!("an unreachable host is a transport failure, not a window")
F crates/pull/src/http.rs:8124:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:8127 [G3 PS3]: panic!("9999-12-31 has no day after it")
F crates/pull/src/http.rs:8210 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
F crates/pull/src/http.rs:8231 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
F crates/pull/src/http.rs:8412:13 False [G3 PS3]: let Err(FetchError::RungNotSpellable { rung, field }) = source.block_on(&daily) else {
F crates/pull/src/http.rs:8413 [G3 PS3]: panic!(
F crates/pull/src/http.rs:8490:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:8493 [G3 PS3]: panic!("a bar with no close must be refused, not defaulted")
F crates/pull/src/http.rs:8529:13 False [G3 PS3]: let Err(FetchError::TransportFailed { detail }) =
F crates/pull/src/http.rs:8532 [G3 PS3]: panic!("the declared envelope is absent and must be named")
F crates/pull/src/http.rs:8600:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:8602 [G3 PS3]: panic!("Dhan is an HTTP broker");
F crates/pull/src/http.rs:8633:13 False [G3 PS3]: let crate::vendor::Transport::Http(zerodha) =
F crates/pull/src/http.rs:8636 [G3 PS3]: panic!("Zerodha is an HTTP broker");
F crates/pull/src/http.rs:8693:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:8695 [G3 PS3]: panic!("Dhan is an HTTP broker");
F crates/pull/src/http.rs:8733:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) =
F crates/pull/src/http.rs:8736 [G3 PS3]: panic!("Groww is an HTTP broker");
F crates/pull/src/http.rs:8766 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:8774:13 False [G3 PS3]: let Err(FetchError::VendorRefused { detail, named, .. }) =
F crates/pull/src/http.rs:8777 [G3 PS3]: panic!("a body naming DH-901 is a refusal whatever the status says")
F crates/pull/src/http.rs:8804 [G3 PS3]: crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
F crates/pull/src/http.rs:8837:13 False [G3 PS3]: let Err(FetchError::VendorRefused { named, .. }) =
F crates/pull/src/http.rs:8840 [G3 PS3]: panic!("a refusal under a 200 is still a refusal")
F crates/pull/src/http.rs:8847:13 False [G3 PS3]: let Ok(Err(FetchError::TransportFailed { detail })) = source.settle_answer(
F crates/pull/src/http.rs:8852 [G3 PS3]: panic!("a body that is not JSON fails the decode")
F crates/pull/src/http.rs:8868:13 False [G3 PS3]: let crate::vendor::Transport::Http(dhan) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:8870 [G3 PS3]: panic!("this feed is HTTP")
F crates/pull/src/http.rs:8995:13 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http.rs:8997 [G3 PS3]: panic!("Dhan is an HTTP feed");
F crates/pull/src/http.rs:9263:86 False [G3 PS3]: matches!(unanswered.error, FetchError::TransportFailed { ref detail } if detail.contains("Refused before the body was read")),
F crates/pull/src/http_cov_tests.rs:59:13 False [G3 PS3]: let FetchError::TransportFailed { detail } = &refused else {
F crates/pull/src/http_cov_tests.rs:60 [G3 PS3]: panic!("{refused:?}");
F crates/pull/src/http_cov_tests.rs:123:13 False [G3 PS3]: let FetchError::NotABarsParam { field } = &refused else {
F crates/pull/src/http_cov_tests.rs:124 [G3 PS3]: panic!("{refused:?}");
F crates/pull/src/http_cov_tests.rs:173:9 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Dhan.descriptor().transport
F crates/pull/src/http_cov_tests.rs:175 [G3 PS3]: panic!("Dhan is an HTTP broker")
F crates/pull/src/http_cov_tests.rs:244 [G3 PS3]: || panic!("the client was built before the credential was checked"),
F crates/pull/src/http_cov_tests.rs:296 [G3 PS3]: matches!(
F crates/pull/src/http_cov_tests.rs:451:9 False [G3 PS3]: let FetchError::TransportFailed { detail } = &refused else {
F crates/pull/src/http_cov_tests.rs:452 [G3 PS3]: panic!("{refused:?}")
F crates/pull/src/http_cov_tests.rs:467 [G3 PS3]: matches!(
F crates/pull/src/http_cov_tests.rs:543:13 False [G3 PS3]: let FetchError::TransportFailed { detail } = &refused else {
F crates/pull/src/http_cov_tests.rs:544 [G3 PS3]: panic!("{refused:?}")
F crates/pull/src/http_cov_tests.rs:599:9 False [G3 PS3]: let FetchError::TransportFailed { detail } = &refused else {
F crates/pull/src/http_cov_tests.rs:600 [G3 PS3]: panic!("{refused:?}")
F crates/pull/src/http_cov_tests.rs:673:9 False [G3 PS3]: output.status.success() && stdout.contains("1 passed"),
F crates/pull/src/http_cov_tests.rs:673:36 False [G3 PS3]: output.status.success() && stdout.contains("1 passed"),
U crates/pull/src/identity_screen.rs:747:16 False [G1 PS1]: if let Ok(month) = day.year_month() {
U crates/pull/src/identity_screen.rs:780 [G1 PS1]: (FirstDay::On(_), None) => false,
U crates/pull/src/ingest.rs:632 [G3 PS3]: _ => String::new(),
U crates/pull/src/ingest.rs:4155:13 False [G3 PS3]: let Some(bucket) = crate::fold::Bucket::of_secs(rung.secs()) else {
U crates/pull/src/ingest.rs:4156 [G3 PS3]: continue;
U crates/pull/src/ingest.rs:5835:15 False [G3 PS3]: } else if found.is_file() {
U crates/pull/src/ingest.rs:5838 [G3 PS3]: "an entry of a kind this host does not name"
F crates/pull/src/ingest.rs:6434 [G3 PS3]: field("why")
F crates/pull/src/ingest.rs:6481:13 False [G3 PS3]: let Err(why) = CensusLock::take(&census) else {
F crates/pull/src/ingest.rs:6482 [G3 PS3]: panic!("a lock that cannot be opened must refuse, never run unlocked")
F crates/pull/src/ingest.rs:6556:13 False [G3 PS3]: let Err(why) = CensusLock::unopened(&lock_path, &full) else {
F crates/pull/src/ingest.rs:6557 [G3 PS3]: panic!("a lock that could not be created must refuse, never run unlocked")
F crates/pull/src/ingest.rs:6559:17 False [G3 PS3]: assert!(why.contains("man.lock") && why.contains("Refused"), "{why}");
F crates/pull/src/ingest.rs:6559:45 False [G3 PS3]: assert!(why.contains("man.lock") && why.contains("Refused"), "{why}");
F crates/pull/src/ingest.rs:6564:13 False [G3 PS3]: let Err(why) = CensusLock::unopened(&lock_path, &starved) else {
F crates/pull/src/ingest.rs:6565 [G3 PS3]: panic!("a regular lock that would not open must refuse, never run unlocked")
F crates/pull/src/ingest.rs:6660:13 False [G3 PS3]: let Err(busy) = CensusLock::take(&census) else {
F crates/pull/src/ingest.rs:6661 [G3 PS3]: panic!("the premise: a held census lock refuses a second run")
F crates/pull/src/ingest.rs:6667 [G3 PS3]: panic!("the dropped lock was released despite the duplicate: {why}")
F crates/pull/src/ingest.rs:6710:13 False [G3 PS3]: let Ok(Err(why)) = answer else {
F crates/pull/src/ingest.rs:6711 [G3 PS3]: panic!("a FIFO at the lock path must refuse promptly, got {answer:?}")
F crates/pull/src/ingest.rs:6770 [G3 PS3]: panic!(
F crates/pull/src/ingest.rs:6772 [G3 PS3]: path.display(),
F crates/pull/src/ingest.rs:6773 [G3 PS3]: tried.join("; ")
F crates/pull/src/ingest.rs:6790:46 False [G3 PS3]: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
F crates/pull/src/ingest.rs:6798 [G3 PS3]: found.file_type()
F crates/pull/src/ingest.rs:6805 [G3 PS3]: link.display()
F crates/pull/src/ingest.rs:6876:13 False [G3 PS3]: let Err(why) = refused else {
F crates/pull/src/ingest.rs:6877 [G3 PS3]: panic!("a socket at the lock path must refuse, got {refused:?}")
F crates/pull/src/ingest.rs:6889:13 False [G3 PS3]: let Err(why) = refused else {
F crates/pull/src/ingest.rs:6890 [G3 PS3]: panic!("a dangling symlink at the lock path must refuse, got {refused:?}")
F crates/pull/src/ingest.rs:6947:17 False [G3 PS3]: !why.contains("another ingest") && !why.contains("Wait for the other run"),
F crates/pull/src/ingest.rs:6947:52 False [G3 PS3]: !why.contains("another ingest") && !why.contains("Wait for the other run"),
F crates/pull/src/ingest_cov_tests.rs:266:9 False [G3 PS3]: none.flagged.is_empty() && none.months.is_empty(),
F crates/pull/src/ingest_cov_tests.rs:266:36 False [G3 PS3]: none.flagged.is_empty() && none.months.is_empty(),
U crates/pull/src/ingest_failkind.rs:149 [G3 PS3]: _ => Self::Other,
F crates/pull/src/ingest_failkind_tests.rs:658:13 False [G3 PS3]: assert!(!late.is_hole() && late.files_as_successor());
F crates/pull/src/ingest_failkind_tests.rs:658:32 False [G3 PS3]: assert!(!late.is_hole() && late.files_as_successor());
F crates/pull/src/landing_run_tests.rs:62 [G2 PS2]: Transport::LocalArchive(_) => panic!("Zerodha is an HTTP feed"),
F crates/pull/src/landing_run_tests.rs:345:9 False [G2 PS2]: gdfl.windows(2).all(|pair| pair[0] < pair[1])
F crates/pull/src/landing_run_tests.rs:346:16 False [G2 PS2]: && zerodha.windows(2).all(|pair| pair[0] < pair[1]),
F crates/pull/src/landing_run_tests.rs:387 [G2 PS2]: mode.word()
F crates/pull/src/landing_run_tests.rs:394:26 False [G2 PS2]: .all(|entry| entry.mode == "import" && entry.by == "cli"),
F crates/pull/src/landing_run_tests.rs:681:21 False [G2 PS2]: grown.starts_with(&claims)
F crates/pull/src/landing_run_tests.rs:682:28 False [G2 PS2]: && grown.lines().count() == 3
F crates/pull/src/landing_run_tests.rs:683:28 False [G2 PS2]: && grown
F crates/pull/src/landing_run_tests.rs:909:13 False [G2 PS2]: said.contains(&format!("run {wider} is past {largest}"))
F crates/pull/src/landing_run_tests.rs:910:20 False [G2 PS2]: && said.contains("32-bit run field (offset 60)")
F crates/pull/src/landing_run_tests.rs:911:20 False [G2 PS2]: && said.contains("nothing is asked of the vendor"),
F crates/pull/src/landing_run_tests.rs:1034:9 False [G2 PS2]: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
F crates/pull/src/landing_run_tests.rs:1034:46 False [G2 PS2]: filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
F crates/pull/src/landing_run_tests.rs:1155 [G2 PS2]: ledger_text(&root, "repairs")
F crates/pull/src/landing_run_tests.rs:1330 [G2 PS2]: before.len()
F crates/pull/src/landing_run_tests.rs:1423:13 False [G2 PS2]: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
F crates/pull/src/landing_run_tests.rs:1423:58 False [G2 PS2]: named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
F crates/pull/src/landing_run_tests.rs:1535:67 False [G2 PS2]: matches!(hooked.kept, Err(NotKept::RunTooWide { run }) if run == u64::from(u32::MAX) + 1)
F crates/pull/src/landing_run_tests.rs:1690:9 False [G2 PS2]: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
F crates/pull/src/landing_run_tests.rs:1690:36 False [G2 PS2]: notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
F crates/pull/src/landing_run_tests.rs:1692 [G2 PS2]: notes[0]
F crates/pull/src/landing_run_tests.rs:1882 [G2 PS2]: hook.series_room()
IGNORED crates/pull/src/landing_run_tests.rs:1985 [G2 PS2]: fn ia10c_timings() {
IGNORED crates/pull/src/landing_run_tests.rs:1987 [G2 PS2]: fn pct(mut v: Vec<std::time::Duration>) -> String {
IGNORED crates/pull/src/landing_run_tests.rs:1988 [G2 PS2]: v.sort_unstable();
IGNORED crates/pull/src/landing_run_tests.rs:1989 [G2 PS2]: let n = v.len();
IGNORED crates/pull/src/landing_run_tests.rs:1990 [G2 PS2]: format!(
IGNORED crates/pull/src/landing_run_tests.rs:1992 [G2 PS2]: v[n / 2],
IGNORED crates/pull/src/landing_run_tests.rs:1993 [G2 PS2]: v[n * 99 / 100],
IGNORED crates/pull/src/landing_run_tests.rs:1994 [G2 PS2]: v[n - 1]
IGNORED crates/pull/src/landing_run_tests.rs:1996 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:1997 [G2 PS2]: let base = std::env::var_os("BRUTEX_IA10C_TIMING_DIR")
IGNORED crates/pull/src/landing_run_tests.rs:1998 [G2 PS2]: .map_or_else(|| scratch("timings"), PathBuf::from);
IGNORED crates/pull/src/landing_run_tests.rs:1999 [G2 PS2]: std::fs::create_dir_all(&base).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2000 [G2 PS2]: let root = base.join(format!("ia10c-{}", std::process::id()));
IGNORED crates/pull/src/landing_run_tests.rs:2001 [G2 PS2]: std::fs::create_dir_all(&root).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2002 [G2 PS2]: println!("timing under {}", root.display());
IGNORED crates/pull/src/landing_run_tests.rs:2003 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2004 [G2 PS2]: for _ in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2005 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2006 [G2 PS2]: allocate(&root, Mode::Spot, By::Api, "zerodha").unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2007 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2008 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2009 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2011 [G2 PS2]: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2016 [G2 PS2]: let big = base.join(format!("ia10c-claims-{}", std::process::id()));
IGNORED crates/pull/src/landing_run_tests.rs:2017 [G2 PS2]: std::fs::create_dir_all(big.join("audit")).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2018 [G2 PS2]: let mut text = String::with_capacity(52 << 20);
IGNORED crates/pull/src/landing_run_tests.rs:2019 [G2 PS2]: let mut id = CLOCK;
IGNORED crates/pull/src/landing_run_tests.rs:2020 [G2 PS2]: while text.len() < 50 << 20 {
IGNORED crates/pull/src/landing_run_tests.rs:2020:11 False [G2 PS2]: while text.len() < 50 << 20 {
IGNORED crates/pull/src/landing_run_tests.rs:2020:11 True [G2 PS2]: while text.len() < 50 << 20 {
IGNORED crates/pull/src/landing_run_tests.rs:2021 [G2 PS2]: let record = ledger::Entry {
IGNORED crates/pull/src/landing_run_tests.rs:2022 [G2 PS2]: id,
IGNORED crates/pull/src/landing_run_tests.rs:2023 [G2 PS2]: mode: Mode::Spot.word().to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2024 [G2 PS2]: kind: kind_of(Vendor::Zerodha).to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2025 [G2 PS2]: by: By::Api.word().to_owned(),
IGNORED crates/pull/src/landing_run_tests.rs:2026 [G2 PS2]: pid: 4242,
IGNORED crates/pull/src/landing_run_tests.rs:2027 [G2 PS2]: started: CLOCK,
IGNORED crates/pull/src/landing_run_tests.rs:2028 [G2 PS2]: resumes: 0,
IGNORED crates/pull/src/landing_run_tests.rs:2029 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2030 [G2 PS2]: .record()
IGNORED crates/pull/src/landing_run_tests.rs:2031 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2032 [G2 PS2]: let body = std::str::from_utf8(&record[..114]).unwrap().trim_end();
IGNORED crates/pull/src/landing_run_tests.rs:2033 [G2 PS2]: let crc = std::str::from_utf8(&record[114..126]).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2034 [G2 PS2]: writeln!(text, "run {body} {crc}").unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2035 [G2 PS2]: id += 1;
IGNORED crates/pull/src/landing_run_tests.rs:2036 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2037 [G2 PS2]: std::fs::write(big.join(ledger::CLAIMS), &text).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2038 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2039 [G2 PS2]: let seeded = allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2040 [G2 PS2]: let seed_took = began.elapsed();
IGNORED crates/pull/src/landing_run_tests.rs:2041 [G2 PS2]: assert_eq!(seeded, id, "seeded past the last claim");
IGNORED crates/pull/src/landing_run_tests.rs:2042 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2044 [G2 PS2]: id - CLOCK,
IGNORED crates/pull/src/landing_run_tests.rs:2045 [G2 PS2]: text.len()
IGNORED crates/pull/src/landing_run_tests.rs:2047 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2048 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/src/landing_run_tests.rs:2049 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2050 [G2 PS2]: allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2051 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2052 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2053 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2055 [G2 PS2]: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2057 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2058 [G2 PS2]: for _ in 0..20 {
IGNORED crates/pull/src/landing_run_tests.rs:2059 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2060 [G2 PS2]: let largest = ledger::largest_claimed_run(&big).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2061 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2062 [G2 PS2]: assert!(largest >= seeded);
IGNORED crates/pull/src/landing_run_tests.rs:2064 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2066 [G2 PS2]: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2068 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2069 [G2 PS2]: for n in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2070 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2071 [G2 PS2]: let held = claim(
IGNORED crates/pull/src/landing_run_tests.rs:2072 [G2 PS2]: &root,
IGNORED crates/pull/src/landing_run_tests.rs:2073 [G2 PS2]: Vendor::Zerodha,
IGNORED crates/pull/src/landing_run_tests.rs:2074 [G2 PS2]: crate::gdfl_run::locks::Hold::Shared,
IGNORED crates/pull/src/landing_run_tests.rs:2075 [G2 PS2]: &format!("api pid 1 run {n}"),
IGNORED crates/pull/src/landing_run_tests.rs:2076 [G2 PS2]: )
IGNORED crates/pull/src/landing_run_tests.rs:2077 [G2 PS2]: .unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2078 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2079 [G2 PS2]: drop(held);
IGNORED crates/pull/src/landing_run_tests.rs:2080 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2081 [G2 PS2]: println!("vendor landing claim, shared: {}", pct(took));
IGNORED crates/pull/src/landing_run_tests.rs:2082 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2083 [G2 PS2]: for n in 0..1_000 {
IGNORED crates/pull/src/landing_run_tests.rs:2084 [G2 PS2]: let line = vec![format!(
IGNORED crates/pull/src/landing_run_tests.rs:2085 [G2 PS2]: "gap NSE-INDEX-ZZT{n} 2024-04-23 v=1 rung=1min owed=375 held=370 missing=5 first_missing=09:15 runs=1 longest=5 longest_at=09:15 why=vendor_
IGNORED crates/pull/src/landing_run_tests.rs:2086 [G2 PS2]: )];
IGNORED crates/pull/src/landing_run_tests.rs:2087 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2088 [G2 PS2]: crate::ingest::audit_ledger::append(&root, Vendor::Zerodha, "gaps", &line).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2089 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2090 [G2 PS2]: }
IGNORED crates/pull/src/landing_run_tests.rs:2091 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2093 [G2 PS2]: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2095 [G2 PS2]: let month = YearMonth::new(2024, 4).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2096 [G2 PS2]: let days = regular_days(month);
IGNORED crates/pull/src/landing_run_tests.rs:2097 [G2 PS2]: let hook = RunHook::new(&root, 77, Vendor::Zerodha, 1);
IGNORED crates/pull/src/landing_run_tests.rs:2098 [G2 PS2]: let mut took = Vec::new();
IGNORED crates/pull/src/landing_run_tests.rs:2099 [G2 PS2]: for n in 0..200_usize {
IGNORED crates/pull/src/landing_run_tests.rs:2100 [G2 PS2]: let on = days[n % days.len()];
IGNORED crates/pull/src/landing_run_tests.rs:2101 [G2 PS2]: let window = Window::new(on, on).unwrap();
IGNORED crates/pull/src/landing_run_tests.rs:2102 [G2 PS2]: let body = kite_body(&[on], &[]);
IGNORED crates/pull/src/landing_run_tests.rs:2103 [G2 PS2]: let (rows, answered) = answered(&body);
IGNORED crates/pull/src/landing_run_tests.rs:2104 [G2 PS2]: let landing = land(&root, &rows, window, Door::FromWindow, 77);
IGNORED crates/pull/src/landing_run_tests.rs:2105 [G2 PS2]: let key = key();
IGNORED crates/pull/src/landing_run_tests.rs:2106 [G2 PS2]: let began = std::time::Instant::now();
IGNORED crates/pull/src/landing_run_tests.rs:2107 [G2 PS2]: let hooked = hook.after_landing(&Landed {
IGNORED crates/pull/src/landing_run_tests.rs:2108 [G2 PS2]: key: &key,
IGNORED crates/pull/src/landing_run_tests.rs:2109 [G2 PS2]: granularity: crate::vendor::Granularity::Minute1,
IGNORED crates/pull/src/landing_run_tests.rs:2110 [G2 PS2]: asked: window,
IGNORED crates/pull/src/landing_run_tests.rs:2111 [G2 PS2]: answered: &answered,
IGNORED crates/pull/src/landing_run_tests.rs:2112 [G2 PS2]: rows: &rows.rows,
IGNORED crates/pull/src/landing_run_tests.rs:2113 [G2 PS2]: landing: &landing,
IGNORED crates/pull/src/landing_run_tests.rs:2114 [G2 PS2]: door: Door::FromWindow,
IGNORED crates/pull/src/landing_run_tests.rs:2115 [G2 PS2]: encoding: spec().timestamps,
IGNORED crates/pull/src/landing_run_tests.rs:2116 [G2 PS2]: });
IGNORED crates/pull/src/landing_run_tests.rs:2117 [G2 PS2]: took.push(began.elapsed());
IGNORED crates/pull/src/landing_run_tests.rs:2118 [G2 PS2]: assert!(hooked.kept.is_ok());
IGNORED crates/pull/src/landing_run_tests.rs:2120 [G2 PS2]: println!(
IGNORED crates/pull/src/landing_run_tests.rs:2122 [G2 PS2]: pct(took)
IGNORED crates/pull/src/landing_run_tests.rs:2124 [G2 PS2]: }
F crates/pull/src/landing_run_tests.rs:2240:9 False [G2 PS2]: let crate::calendar::DayKind::Open(session) =
F crates/pull/src/landing_run_tests.rs:2243 [G2 PS2]: panic!("{on} is not a session of record");
F crates/pull/src/landing_run_tests.rs:2284:13 False [G2 PS2]: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
F crates/pull/src/landing_run_tests.rs:2284:25 False [G2 PS2]: assert!(owed > 0 && owed < 375, "a short special session: {owed}");
F crates/pull/src/landing_run_tests.rs:2291:13 False [G2 PS2]: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
F crates/pull/src/landing_run_tests.rs:2291:36 False [G2 PS2]: assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());
F crates/pull/src/landing_run_tests.rs:2324 [G2 PS2]: dir.display()
F crates/pull/src/landing_run_tests.rs:2457:9 False [G2 PS2]: let Some(Err(why)) = &hooked.spooled else {
F crates/pull/src/landing_run_tests.rs:2458 [G2 PS2]: panic!("{hooked:?}");
F crates/pull/src/landing_run_tests.rs:2678:13 False [G2 PS2]: assert!(february.contains(&day(2021, 2, 23)) && february.contains(&day(2021, 2, 25)));
F crates/pull/src/landing_run_tests.rs:2678:53 False [G2 PS2]: assert!(february.contains(&day(2021, 2, 23)) && february.contains(&day(2021, 2, 25)));
F crates/pull/src/listing_tests.rs:145:9 False [G1 PS1]: let Source::NseSecurityMaster { file, sha256 } = span.source else {
F crates/pull/src/listing_tests.rs:146 [G1 PS1]: panic!("a master span names its master");
GAP crates/pull/src/listing_tests.rs:490 [G1 PS1]: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {}
F crates/pull/src/listing_tests.rs:491 [G1 PS1]: Err(why) => panic!("an odd name: {why}"),
U crates/pull/src/manifest.rs:2175:20 False [G1 PS1]: if let Some(slot) = closes.get_mut(C_CONTRACT..C_CONTRACT + text.len()) {
F crates/pull/src/manifest_ps1_tests.rs:108 [G1 PS1]: String::from_utf8_lossy(&magic)
U crates/pull/src/master_snapshots.rs:968 [G1 PS1]: Err(why) => return refused(why),
U crates/pull/src/master_snapshots.rs:2142:17 False [G1 PS1]: let Some(before) = old_rows.get(at) else {
U crates/pull/src/master_snapshots.rs:2143 [G1 PS1]: continue;
U crates/pull/src/master_snapshots.rs:2145:20 False [G1 PS1]: if let Some(slot) = matched.get_mut(at) {
U crates/pull/src/master_snapshots.rs:2187:20 False [G1 PS1]: && let Some(before) = old_rows.get(at)
U crates/pull/src/master_snapshots.rs:2194:20 False [G1 PS1]: if let Some(slot) = matched.get_mut(at) {
U crates/pull/src/master_snapshots.rs:2228:20 False [G1 PS1]: && let Some(before) = old_rows.get(at)
U crates/pull/src/master_snapshots.rs:2231:20 False [G1 PS1]: if let Some(slot) = matched.get_mut(at) {
U crates/pull/src/master_snapshots.rs:2486:20 False [G1 PS1]: if let Some(count) = counts.get_mut(one.change.index()) {
F crates/pull/src/master_snapshots_cov_tests.rs:380:9 False [G1 PS1]: words.contains(&"expiry") && words.contains(&"lot_size"),
F crates/pull/src/master_snapshots_cov_tests.rs:380:38 False [G1 PS1]: words.contains(&"expiry") && words.contains(&"lot_size"),
F crates/pull/src/master_snapshots_cov_tests.rs:695 [G1 PS1]: wrote[0]
U crates/pull/src/master_snapshots_first_listed.rs:711:16 False [G1 PS1]: if let Some(path) = path_of(dir, route.exchange, name) {
U crates/pull/src/master_snapshots_first_listed.rs:713 [G1 PS1]: }
U crates/pull/src/master_snapshots_first_listed.rs:768 [G1 PS1]: Rebuilt::StillDamaged(why) => Err(format!("futures_index_damaged: {why}")),
F crates/pull/src/master_snapshots_first_listed_tests.rs:107:13 False [G1 PS1]: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
F crates/pull/src/master_snapshots_first_listed_tests.rs:107:32 False [G1 PS1]: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
F crates/pull/src/master_snapshots_first_listed_tests.rs:107:52 False [G1 PS1]: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
F crates/pull/src/master_snapshots_first_listed_tests.rs:107:72 False [G1 PS1]: assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
F crates/pull/src/master_snapshots_first_listed_tests.rs:199:9 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_first_listed_tests.rs:199:38 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_first_listed_tests.rs:234:13 False [G1 PS1]: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
F crates/pull/src/master_snapshots_first_listed_tests.rs:234:38 False [G1 PS1]: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
F crates/pull/src/master_snapshots_first_listed_tests.rs:234:65 False [G1 PS1]: assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
F crates/pull/src/master_snapshots_first_listed_tests.rs:279:13 False [G1 PS1]: listed.refused[0]
F crates/pull/src/master_snapshots_first_listed_tests.rs:281:20 False [G1 PS1]: && listed.refused[0].contains("recorded when the futures index took it in"),
F crates/pull/src/master_snapshots_first_listed_tests.rs:380:9 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_first_listed_tests.rs:380:38 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_first_listed_tests.rs:448:13 False [G1 PS1]: said.starts_with("generation g1 of the futures index did not match its commit log")
F crates/pull/src/master_snapshots_first_listed_tests.rs:449:20 False [G1 PS1]: && said.contains(why)
F crates/pull/src/master_snapshots_first_listed_tests.rs:450:20 False [G1 PS1]: && said.contains("generation g2 was started"),
F crates/pull/src/master_snapshots_first_listed_tests.rs:467:13 False [G1 PS1]: record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
F crates/pull/src/master_snapshots_first_listed_tests.rs:467:64 False [G1 PS1]: record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
F crates/pull/src/master_snapshots_first_listed_tests.rs:515 [G1 PS1]: other @ Rebuilt::Done(..) => panic!("{other:?}"),
F crates/pull/src/master_snapshots_first_listed_tests.rs:518:9 False [G1 PS1]: why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
F crates/pull/src/master_snapshots_first_listed_tests.rs:518:40 False [G1 PS1]: why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
F crates/pull/src/master_snapshots_first_listed_tests.rs:581:9 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_first_listed_tests.rs:581:38 False [G1 PS1]: listed.refused.is_empty() && listed.rebuilt.is_none(),
F crates/pull/src/master_snapshots_tests.rs:149:9 False [G1 PS1]: let TokenOn::Resolved {
F crates/pull/src/master_snapshots_tests.rs:154 [G1 PS1]: panic!("the first date resolves");
F crates/pull/src/master_snapshots_tests.rs:156:9 False [G1 PS1]: let TokenOn::Resolved {
F crates/pull/src/master_snapshots_tests.rs:161 [G1 PS1]: panic!("the second date resolves");
F crates/pull/src/master_snapshots_tests.rs:184 [G1 PS1]: other => panic!("a reused token between snapshots is ambiguous, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:191:9 False [G1 PS1]: let TokenOn::Resolved { snapshot, listed } = book.token_on(between, "NFO", 9_000_002) else {
F crates/pull/src/master_snapshots_tests.rs:192 [G1 PS1]: panic!("an agreeing token resolves");
F crates/pull/src/master_snapshots_tests.rs:197:9 False [G1 PS1]: let TokenOn::Resolved { listed, .. } = book.token_on(day(2026, 2, 20), "NFO", 9_000_001) else {
F crates/pull/src/master_snapshots_tests.rs:198 [G1 PS1]: panic!("after the newest snapshot the newest answers");
F crates/pull/src/master_snapshots_tests.rs:211 [G1 PS1]: other => panic!("before the first snapshot nothing resolves, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:284 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:304:9 False [G1 PS1]: let TokenOn::Resolved { listed, .. } = book.token_on(d1, "NFO", 9_000_021) else {
F crates/pull/src/master_snapshots_tests.rs:305 [G1 PS1]: panic!("the receipted refresh answers");
F crates/pull/src/master_snapshots_tests.rs:362:9 False [G1 PS1]: json.contains(r#""incomplete":{"held":3,"named":[{"file":"instruments_20260304.csv.gz","stamp":"2026-03-04","why":"a body with no receipt (a
F crates/pull/src/master_snapshots_tests.rs:363:16 False [G1 PS1]: && json.contains(r#""unnamed":0,"refusal":null}"#),
F crates/pull/src/master_snapshots_tests.rs:369:9 False [G1 PS1]: why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
F crates/pull/src/master_snapshots_tests.rs:369:52 False [G1 PS1]: why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
IGNORED crates/pull/src/master_snapshots_tests.rs:407 [G1 PS1]: fn measure_the_hook_with_incomplete_entries_held() {
IGNORED crates/pull/src/master_snapshots_tests.rs:409 [G1 PS1]: let body = dump(&[fut(9_000_051, "ZQIDX26JUNFUT", "2026-06-30", 75)]);
IGNORED crates/pull/src/master_snapshots_tests.rs:410 [G1 PS1]: for held in [0_usize, INCOMPLETE_NAMED, 1_000] {
IGNORED crates/pull/src/master_snapshots_tests.rs:411 [G1 PS1]: let dir = scratch(&format!("measure-{held}"));
IGNORED crates/pull/src/master_snapshots_tests.rs:412 [G1 PS1]: let root = dir.join(SNAPSHOT_DIR);
IGNORED crates/pull/src/master_snapshots_tests.rs:413 [G1 PS1]: fs::create_dir_all(&root).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:414 [G1 PS1]: let first = day(2020, 1, 1).days_from_epoch();
IGNORED crates/pull/src/master_snapshots_tests.rs:415 [G1 PS1]: for n in 0..held {
IGNORED crates/pull/src/master_snapshots_tests.rs:416 [G1 PS1]: let on = Day::from_days(first + u32::try_from(n).unwrap()).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:417 [G1 PS1]: fs::write(root.join(file_name(Kind::Kite, stamp(on, 1))), b"cut").unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:418 [G1 PS1]: }
IGNORED crates/pull/src/master_snapshots_tests.rs:419 [G1 PS1]: let start = day(2026, 1, 1).days_from_epoch();
IGNORED crates/pull/src/master_snapshots_tests.rs:420 [G1 PS1]: let mut costs: Vec<u128> = (0..N)
IGNORED crates/pull/src/master_snapshots_tests.rs:421 [G1 PS1]: .map(|n| {
IGNORED crates/pull/src/master_snapshots_tests.rs:422 [G1 PS1]: let on = Day::from_days(start + u32::try_from(n).unwrap()).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:423 [G1 PS1]: let began = std::time::Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:424 [G1 PS1]: let outcome = after_land(&dir, kite_source(), &body, &written(), Ok(on)).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:425 [G1 PS1]: let json = outcome.json();
IGNORED crates/pull/src/master_snapshots_tests.rs:426 [G1 PS1]: let cost = began.elapsed().as_micros();
IGNORED crates/pull/src/master_snapshots_tests.rs:427 [G1 PS1]: assert_eq!(outcome.incomplete.as_ref().map(Vec::len), Ok(held));
IGNORED crates/pull/src/master_snapshots_tests.rs:428 [G1 PS1]: assert!(json.contains(&format!(r#""held":{held},"#)), "{json}");
IGNORED crates/pull/src/master_snapshots_tests.rs:429 [G1 PS1]: cost
IGNORED crates/pull/src/master_snapshots_tests.rs:430 [G1 PS1]: })
IGNORED crates/pull/src/master_snapshots_tests.rs:431 [G1 PS1]: .collect();
IGNORED crates/pull/src/master_snapshots_tests.rs:432 [G1 PS1]: costs.sort_unstable();
IGNORED crates/pull/src/master_snapshots_tests.rs:433 [G1 PS1]: println!(
IGNORED crates/pull/src/master_snapshots_tests.rs:435 [G1 PS1]: costs[N / 2],
IGNORED crates/pull/src/master_snapshots_tests.rs:436 [G1 PS1]: costs[N * 99 / 100 - 1],
IGNORED crates/pull/src/master_snapshots_tests.rs:437 [G1 PS1]: costs[N - 1]
IGNORED crates/pull/src/master_snapshots_tests.rs:440 [G1 PS1]: }
F crates/pull/src/master_snapshots_tests.rs:466:9 False [G1 PS1]: json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
F crates/pull/src/master_snapshots_tests.rs:466:57 False [G1 PS1]: json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
F crates/pull/src/master_snapshots_tests.rs:761 [G1 PS1]: k1.json()
F crates/pull/src/master_snapshots_tests.rs:823:9 False [G1 PS1]: why.contains("after today 2026-02-01")
F crates/pull/src/master_snapshots_tests.rs:824:16 False [G1 PS1]: && why.contains(&file_name(Kind::Kite, stamp(d2, 1)))
F crates/pull/src/master_snapshots_tests.rs:825:16 False [G1 PS1]: && why.contains("refused rather than believed fresh"),
F crates/pull/src/master_snapshots_tests.rs:847 [G1 PS1]: other => panic!("a torn snapshot is unreadable, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:886:9 False [G1 PS1]: garbage.contains("line 2") && garbage.contains("instrument_token"),
F crates/pull/src/master_snapshots_tests.rs:886:39 False [G1 PS1]: garbage.contains("line 2") && garbage.contains("instrument_token"),
F crates/pull/src/master_snapshots_tests.rs:923:9 False [G1 PS1]: let Lookup::Listed(listed) = one.lookup("NFO", 9_800_001) else {
F crates/pull/src/master_snapshots_tests.rs:924 [G1 PS1]: panic!("one row is found");
F crates/pull/src/master_snapshots_tests.rs:943:9 False [G1 PS1]: let Lookup::Listed(again) = other.lookup("NFO", 9_800_001) else {
F crates/pull/src/master_snapshots_tests.rs:944 [G1 PS1]: panic!("found");
F crates/pull/src/master_snapshots_tests.rs:996 [G1 PS1]: other => panic!("resolves, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:1013 [G1 PS1]: other => panic!("expired, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:1051 [G1 PS1]: other => panic!("{on}: expired, got {other:?}"),
F crates/pull/src/master_snapshots_tests.rs:1057 [G1 PS1]: matches!(
F crates/pull/src/master_snapshots_tests.rs:1065 [G1 PS1]: matches!(
F crates/pull/src/master_snapshots_tests.rs:1219:9 False [G1 PS1]: why.contains("no snapshot was written") && why.contains("1970"),
F crates/pull/src/master_snapshots_tests.rs:1219:52 False [G1 PS1]: why.contains("no snapshot was written") && why.contains("1970"),
F crates/pull/src/master_snapshots_tests.rs:1225 [G1 PS1]: outcome.json()
IGNORED crates/pull/src/master_snapshots_tests.rs:1241 [G1 PS1]: fn a_real_size_dump_is_measured() {
IGNORED crates/pull/src/master_snapshots_tests.rs:1244 [G1 PS1]: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
IGNORED crates/pull/src/master_snapshots_tests.rs:1244:9 False [G1 PS1]: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
IGNORED crates/pull/src/master_snapshots_tests.rs:1244:9 True [G1 PS1]: let Some(path) = std::env::var_os("BRUTEX_KITE_DUMP") else {
IGNORED crates/pull/src/master_snapshots_tests.rs:1245 [G1 PS1]: panic!("set BRUTEX_KITE_DUMP to a copy of a landed Kite dump");
IGNORED crates/pull/src/master_snapshots_tests.rs:1247 [G1 PS1]: let body = fs::read_to_string(&path).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1248 [G1 PS1]: let root = scratch("real-size");
IGNORED crates/pull/src/master_snapshots_tests.rs:1249 [G1 PS1]: let (d1, d2, between) = (day(2026, 10, 15), day(2026, 10, 22), day(2026, 10, 18));
IGNORED crates/pull/src/master_snapshots_tests.rs:1250 [G1 PS1]: let started = Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:1251 [G1 PS1]: snapshot(&root, Kind::Kite, d1, URL, body.as_bytes()).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1252 [G1 PS1]: let write_ms = started.elapsed().as_millis();
IGNORED crates/pull/src/master_snapshots_tests.rs:1253 [G1 PS1]: let changed = body.replacen(",75,FUT,", ",65,FUT,", 3);
IGNORED crates/pull/src/master_snapshots_tests.rs:1254 [G1 PS1]: snapshot(&root, Kind::Kite, d2, URL, changed.as_bytes()).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1255 [G1 PS1]: let started = Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:1256 [G1 PS1]: let parsed = Snapshot::parse(stamp(d1, 1), &body).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1257 [G1 PS1]: let parse_ms = started.elapsed().as_millis();
IGNORED crates/pull/src/master_snapshots_tests.rs:1258 [G1 PS1]: let started = Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:1259 [G1 PS1]: let summary = diff_for(&root, stamp(d2, 1)).unwrap().unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1260 [G1 PS1]: let diff_ms = started.elapsed().as_millis();
IGNORED crates/pull/src/master_snapshots_tests.rs:1261 [G1 PS1]: let book = Book::open(&root).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:1262 [G1 PS1]: let step = (parsed.len() / 200).max(1);
IGNORED crates/pull/src/master_snapshots_tests.rs:1263 [G1 PS1]: let tokens: Vec<(String, u64)> = parsed
IGNORED crates/pull/src/master_snapshots_tests.rs:1264 [G1 PS1]: .rows()
IGNORED crates/pull/src/master_snapshots_tests.rs:1265 [G1 PS1]: .step_by(step)
IGNORED crates/pull/src/master_snapshots_tests.rs:1266 [G1 PS1]: .take(200)
IGNORED crates/pull/src/master_snapshots_tests.rs:1267 [G1 PS1]: .map(|listed| (listed.exchange().to_owned(), listed.token()))
IGNORED crates/pull/src/master_snapshots_tests.rs:1268 [G1 PS1]: .collect();
IGNORED crates/pull/src/master_snapshots_tests.rs:1269 [G1 PS1]: let started = Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:1270 [G1 PS1]: let first = book.token_on(between, &tokens[0].0, tokens[0].1).word();
IGNORED crates/pull/src/master_snapshots_tests.rs:1271 [G1 PS1]: let cold_ms = started.elapsed().as_millis();
IGNORED crates/pull/src/master_snapshots_tests.rs:1272 [G1 PS1]: let mut ns: Vec<u128> = tokens
IGNORED crates/pull/src/master_snapshots_tests.rs:1273 [G1 PS1]: .iter()
IGNORED crates/pull/src/master_snapshots_tests.rs:1274 [G1 PS1]: .map(|(exchange, token)| {
IGNORED crates/pull/src/master_snapshots_tests.rs:1275 [G1 PS1]: let started = Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:1276 [G1 PS1]: black_box(book.token_on(black_box(between), exchange, *token));
IGNORED crates/pull/src/master_snapshots_tests.rs:1277 [G1 PS1]: started.elapsed().as_nanos()
IGNORED crates/pull/src/master_snapshots_tests.rs:1278 [G1 PS1]: })
IGNORED crates/pull/src/master_snapshots_tests.rs:1279 [G1 PS1]: .collect();
IGNORED crates/pull/src/master_snapshots_tests.rs:1280 [G1 PS1]: ns.sort_unstable();
IGNORED crates/pull/src/master_snapshots_tests.rs:1281 [G1 PS1]: let n = ns.len();
IGNORED crates/pull/src/master_snapshots_tests.rs:1282 [G1 PS1]: let compressed = fs::metadata(root.join(file_name(Kind::Kite, stamp(d1, 1))))
IGNORED crates/pull/src/master_snapshots_tests.rs:1283 [G1 PS1]: .unwrap()
IGNORED crates/pull/src/master_snapshots_tests.rs:1284 [G1 PS1]: .len();
IGNORED crates/pull/src/master_snapshots_tests.rs:1285 [G1 PS1]: println!(
IGNORED crates/pull/src/master_snapshots_tests.rs:1289 [G1 PS1]: parsed.len(),
IGNORED crates/pull/src/master_snapshots_tests.rs:1290 [G1 PS1]: body.len(),
IGNORED crates/pull/src/master_snapshots_tests.rs:1291 [G1 PS1]: summary.counts_line(),
IGNORED crates/pull/src/master_snapshots_tests.rs:1292 [G1 PS1]: ns[n / 2],
IGNORED crates/pull/src/master_snapshots_tests.rs:1293 [G1 PS1]: ns[n * 99 / 100],
IGNORED crates/pull/src/master_snapshots_tests.rs:1294 [G1 PS1]: ns[n - 1],
IGNORED crates/pull/src/master_snapshots_tests.rs:1296 [G1 PS1]: assert_eq!(summary.counts[4], 3, "the three injected lot changes");
IGNORED crates/pull/src/master_snapshots_tests.rs:1297 [G1 PS1]: }
F crates/pull/src/master_snapshots_tests.rs:1336:9 False [G1 PS1]: let TokenOn::Resolved {
F crates/pull/src/master_snapshots_tests.rs:1341 [G1 PS1]: panic!("the day's newest snapshot agrees, so the older one is believed");
F crates/pull/src/master_snapshots_tests.rs:1359 [G1 PS1]: answer => panic!("the day's newest disagrees, so ambiguous; got {answer:?}"),
F crates/pull/src/master_snapshots_tests.rs:1387:16 False [G1 PS1]: && one.to_string().starts_with(&partial)
F crates/pull/src/master_snapshots_tests.rs:1390 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1437 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1471:9 False [G1 PS1]: let Some(Ok(Some(ref summary))) = outcome.diff else {
F crates/pull/src/master_snapshots_tests.rs:1472 [G1 PS1]: panic!("a diff against the day before: {:?}", outcome.diff);
F crates/pull/src/master_snapshots_tests.rs:1511:47 False [G1 PS1]: catalog.incomplete().iter().any(|one| one.file == cut
F crates/pull/src/master_snapshots_tests.rs:1512:16 False [G1 PS1]: && one.kind == Kind::Kite
F crates/pull/src/master_snapshots_tests.rs:1513:16 False [G1 PS1]: && one.stamp == b
F crates/pull/src/master_snapshots_tests.rs:1516 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1623 [G1 PS1]: book.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1635:24 False [G1 PS1]: .any(|one| one.stamp == cut && one.why.contains("no receipt")),
F crates/pull/src/master_snapshots_tests.rs:1637 [G1 PS1]: book.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1670:24 False [G1 PS1]: .any(|one| one.file == twin && one.why.contains("another case")),
F crates/pull/src/master_snapshots_tests.rs:1672 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1734:24 False [G1 PS1]: .any(|one| one.file == upper && one.why == CUT_DIFF),
F crates/pull/src/master_snapshots_tests.rs:1736 [G1 PS1]: catalog.incomplete()
F crates/pull/src/master_snapshots_tests.rs:1958:9 False [G1 PS1]: let Lookup::Listed(listed) = one.lookup("NFO", 9_800_101) else {
F crates/pull/src/master_snapshots_tests.rs:1959 [G1 PS1]: panic!("read in any column order");
F crates/pull/src/master_snapshots_tests.rs:1969:9 False [G1 PS1]: let Lookup::Listed(first) = parsed.lookup("NFO", 9_800_102) else {
F crates/pull/src/master_snapshots_tests.rs:1970 [G1 PS1]: panic!("found");
F crates/pull/src/master_snapshots_tests.rs:2001:9 False [G1 PS1]: let Lookup::Listed(other) = parsed.lookup("NFO", 9_900_102) else {
F crates/pull/src/master_snapshots_tests.rs:2002 [G1 PS1]: panic!("the row beside it answers");
IGNORED crates/pull/src/master_snapshots_tests.rs:2047 [G1 PS1]: fn measure_the_catalog_listing_and_its_probes() {
IGNORED crates/pull/src/master_snapshots_tests.rs:2049 [G1 PS1]: let first = day(2020, 1, 1).days_from_epoch();
IGNORED crates/pull/src/master_snapshots_tests.rs:2050 [G1 PS1]: for held in [1_u32, 100, 1_000] {
IGNORED crates/pull/src/master_snapshots_tests.rs:2051 [G1 PS1]: let root = scratch(&format!("measure-catalog-{held}"));
IGNORED crates/pull/src/master_snapshots_tests.rs:2052 [G1 PS1]: for n in 0..held {
IGNORED crates/pull/src/master_snapshots_tests.rs:2053 [G1 PS1]: let on = Day::from_days(first + n / 2).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:2054 [G1 PS1]: let s = stamp(on, 1 + n % 2);
IGNORED crates/pull/src/master_snapshots_tests.rs:2055 [G1 PS1]: fs::write(root.join(file_name(Kind::Kite, s)), b"").unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:2056 [G1 PS1]: fs::write(root.join(receipt_name(Kind::Kite, s)), b"").unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:2057 [G1 PS1]: }
IGNORED crates/pull/src/master_snapshots_tests.rs:2058 [G1 PS1]: let newest = stamp(
IGNORED crates/pull/src/master_snapshots_tests.rs:2059 [G1 PS1]: Day::from_days(first + (held - 1) / 2).unwrap(),
IGNORED crates/pull/src/master_snapshots_tests.rs:2060 [G1 PS1]: 1 + (held - 1) % 2,
IGNORED crates/pull/src/master_snapshots_tests.rs:2062 [G1 PS1]: let (mut listing, mut probes): (Vec<u128>, Vec<u128>) = (0..N)
IGNORED crates/pull/src/master_snapshots_tests.rs:2063 [G1 PS1]: .map(|_| {
IGNORED crates/pull/src/master_snapshots_tests.rs:2064 [G1 PS1]: let began = std::time::Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:2065 [G1 PS1]: let catalog = Catalog::read(&root).unwrap();
IGNORED crates/pull/src/master_snapshots_tests.rs:2066 [G1 PS1]: let listed = began.elapsed().as_nanos();
IGNORED crates/pull/src/master_snapshots_tests.rs:2067 [G1 PS1]: let began = std::time::Instant::now();
IGNORED crates/pull/src/master_snapshots_tests.rs:2068 [G1 PS1]: let answers = (
IGNORED crates/pull/src/master_snapshots_tests.rs:2069 [G1 PS1]: std::hint::black_box(catalog.of_day(Kind::Kite, newest.day)),
IGNORED crates/pull/src/master_snapshots_tests.rs:2070 [G1 PS1]: std::hint::black_box(catalog.before(Kind::Kite, newest)),
IGNORED crates/pull/src/master_snapshots_tests.rs:2071 [G1 PS1]: std::hint::black_box(catalog.is_receipted(Kind::Kite, newest)),
IGNORED crates/pull/src/master_snapshots_tests.rs:2072 [G1 PS1]: );
IGNORED crates/pull/src/master_snapshots_tests.rs:2073 [G1 PS1]: let probed = began.elapsed().as_nanos();
IGNORED crates/pull/src/master_snapshots_tests.rs:2074 [G1 PS1]: assert_eq!(answers.0, Some(newest));
IGNORED crates/pull/src/master_snapshots_tests.rs:2075 [G1 PS1]: assert_eq!(answers.1.is_some(), held > 1);
IGNORED crates/pull/src/master_snapshots_tests.rs:2076 [G1 PS1]: assert!(answers.2);
IGNORED crates/pull/src/master_snapshots_tests.rs:2077 [G1 PS1]: assert_eq!(
IGNORED crates/pull/src/master_snapshots_tests.rs:2078 [G1 PS1]: catalog.receipted(Kind::Kite).len(),
IGNORED crates/pull/src/master_snapshots_tests.rs:2079 [G1 PS1]: usize::try_from(held).unwrap()
IGNORED crates/pull/src/master_snapshots_tests.rs:2081 [G1 PS1]: (listed, probed)
IGNORED crates/pull/src/master_snapshots_tests.rs:2082 [G1 PS1]: })
IGNORED crates/pull/src/master_snapshots_tests.rs:2083 [G1 PS1]: .unzip();
IGNORED crates/pull/src/master_snapshots_tests.rs:2084 [G1 PS1]: listing.sort_unstable();
IGNORED crates/pull/src/master_snapshots_tests.rs:2085 [G1 PS1]: probes.sort_unstable();
IGNORED crates/pull/src/master_snapshots_tests.rs:2086 [G1 PS1]: println!(
IGNORED crates/pull/src/master_snapshots_tests.rs:2089 [G1 PS1]: held * 2,
IGNORED crates/pull/src/master_snapshots_tests.rs:2090 [G1 PS1]: listing[N / 2] / 1_000,
IGNORED crates/pull/src/master_snapshots_tests.rs:2091 [G1 PS1]: listing[N * 99 / 100 - 1] / 1_000,
IGNORED crates/pull/src/master_snapshots_tests.rs:2092 [G1 PS1]: listing[N - 1] / 1_000,
IGNORED crates/pull/src/master_snapshots_tests.rs:2093 [G1 PS1]: probes[N / 2],
IGNORED crates/pull/src/master_snapshots_tests.rs:2094 [G1 PS1]: probes[N * 99 / 100 - 1],
IGNORED crates/pull/src/master_snapshots_tests.rs:2095 [G1 PS1]: probes[N - 1]
IGNORED crates/pull/src/master_snapshots_tests.rs:2098 [G1 PS1]: }
U crates/pull/src/masters.rs:789:16 False [G3 PS3]: if let Some(ref host) = host {
U crates/pull/src/masters.rs:797 [G3 PS3]: }
F crates/pull/src/masters.rs:2054:56 False [G3 PS3]: source.file == NSE_INDICES_FILE || source.file == NSE_TOTAL_MARKET_FILE,
F crates/pull/src/masters.rs:2073 [G3 PS3]: why.chars().take(200).collect::<String>()
F crates/pull/src/masters.rs:2079:13 False [G3 PS3]: !why.contains("ceiling") && why.contains("no comma"),
F crates/pull/src/masters.rs:2079:41 False [G3 PS3]: !why.contains("ceiling") && why.contains("no comma"),
F crates/pull/src/masters.rs:2081 [G3 PS3]: why.chars().take(200).collect::<String>()
F crates/pull/src/masters.rs:2158:17 False [G3 PS3]: let Landed::Refused(why) = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2159 [G3 PS3]: panic!("{name} must be refused");
F crates/pull/src/masters.rs:2221:13 False [G3 PS3]: let Landed::Refused(why) = outcome else {
F crates/pull/src/masters.rs:2222 [G3 PS3]: panic!("a short body must be refused");
F crates/pull/src/masters.rs:2240:13 False [G3 PS3]: let Landed::Refused(why) = land(&dir, &SOURCES[0], &html) else {
F crates/pull/src/masters.rs:2241 [G3 PS3]: panic!("a long HTML page is not a CSV");
F crates/pull/src/masters.rs:2272:13 False [G3 PS3]: let Landed::Refused(why) = land(&dir, &SOURCES[0], &json) else {
F crates/pull/src/masters.rs:2273 [G3 PS3]: panic!("JSON is not the CSV a master is");
F crates/pull/src/masters.rs:2394:13 False [G3 PS3]: let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2395 [G3 PS3]: panic!("the first write lands");
F crates/pull/src/masters.rs:2400:13 False [G3 PS3]: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2401 [G3 PS3]: panic!("the second write lands");
F crates/pull/src/masters.rs:2408:13 False [G3 PS3]: let Landed::Written { changed, .. } = land(&dir, source, &moved) else {
F crates/pull/src/masters.rs:2409 [G3 PS3]: panic!("the third write lands");
F crates/pull/src/masters.rs:2444:13 False [G3 PS3]: let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2445 [G3 PS3]: panic!("the identical landing is still written");
F crates/pull/src/masters.rs:2468:13 False [G3 PS3]: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2469 [G3 PS3]: panic!("the repair lands");
F crates/pull/src/masters.rs:2478:13 False [G3 PS3]: let Landed::Written { changed, .. } = land(&dir, source, &body) else {
F crates/pull/src/masters.rs:2479 [G3 PS3]: panic!("the repair lands");
F crates/pull/src/masters.rs:2506:17 False [G3 PS3]: let Landed::Refused(why) = land(&dir, source, &candidate) else {
F crates/pull/src/masters.rs:2507 [G3 PS3]: panic!("an unreadable data row must refuse");
F crates/pull/src/masters.rs:2645:13 False [G3 PS3]: let Landed::Refused(why) = land(&dir, source, &bad) else {
F crates/pull/src/masters.rs:2646 [G3 PS3]: panic!("a malformed row refuses");
F crates/pull/src/masters.rs:3263 [G3 PS3]: matches!(
F crates/pull/src/masters.rs:3293 [G3 PS3]: other => panic!("expected a refusal, got {other:?}"),
F crates/pull/src/masters.rs:3500:13 False [G3 PS3]: let Landed::Refused(ref why) = landed else {
F crates/pull/src/masters.rs:3501 [G3 PS3]: unreachable!("an unconvertible body cannot be written")
F crates/pull/src/masters.rs:3526:13 False [G3 PS3]: let Landed::Refused(ref why) = landed else {
F crates/pull/src/masters.rs:3527 [G3 PS3]: unreachable!("a body with no columns cannot be a master")
F crates/pull/src/masters.rs:3668:13 False [G3 PS3]: let Landed::Refused(ref why) = landed else {
F crates/pull/src/masters.rs:3669 [G3 PS3]: unreachable!("a master the reader cannot read must not be written")
F crates/pull/src/masters.rs:3733 [G3 PS3]: let _ = std::fs::OpenOptions::new().write(true).open(&target);
F crates/pull/src/masters.rs:3734 [G3 PS3]: panic!("a FIFO at the target must not hold the landing");
F crates/pull/src/masters.rs:3737:13 False [G3 PS3]: let Landed::Refused(ref why) = landed else {
F crates/pull/src/masters.rs:3738 [G3 PS3]: panic!("a target that is not a regular file is refused: {landed:?}");
F crates/pull/src/masters_land_tests.rs:64:9 False [G3 PS3]: let Landed::Refused(why) = landed else {
F crates/pull/src/masters_land_tests.rs:65 [G3 PS3]: panic!("refused, got {landed:?}");
F crates/pull/src/masters_land_tests.rs:232:9 False [G3 PS3]: let Landed::Uncertain {
F crates/pull/src/masters_land_tests.rs:238 [G3 PS3]: panic!("uncertain, got {landed:?}");
F crates/pull/src/masters_land_tests.rs:271:9 False [G3 PS3]: let Landed::Written { changed, bytes } = landed else {
F crates/pull/src/masters_land_tests.rs:272 [G3 PS3]: panic!("written, got {landed:?}");
U crates/pull/src/nse.rs:396:16 False [G1 PS1]: if let Some(head) = rest.get(..at) {
U crates/pull/src/nse.rs:399:13 False [G1 PS1]: let Some(after) = rest.get(at.saturating_add(4)..) else {
U crates/pull/src/nse.rs:400 [G1 PS1]: return out;
U crates/pull/src/nse.rs:442:13 False [G1 PS1]: let Some(after) = rest.get(at.saturating_add(5)..) else {
U crates/pull/src/nse.rs:443 [G1 PS1]: break;
U crates/pull/src/nse.rs:457:13 False [G1 PS1]: let Some(value) = value else { break };
U crates/pull/src/nse.rs:584:13 False [G1 PS1]: let Some(after) = rest.get(at.saturating_add(5)..) else {
U crates/pull/src/nse.rs:585 [G1 PS1]: break;
U crates/pull/src/nse.rs:595:13 False [G1 PS1]: let Some(value) = value else { break };
U crates/pull/src/nse.rs:824:24 False [G1 PS1]: if let Some(field) = line.get(start..at) {
U crates/pull/src/nse.rs:833:12 False [G1 PS1]: if let Some(field) = line.get(start..) {
F crates/pull/src/nse.rs:884:13 False [G1 PS1]: let NseError::NotCsv { ref head } = why else {
F crates/pull/src/nse.rs:885 [G1 PS1]: panic!("an interstitial must refuse as NotCsv, got {why:?}");
F crates/pull/src/nse.rs:1018:24 False [G1 PS1]: if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES
F crates/pull/src/nse.rs:1018:59 False [G1 PS1]: if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES
F crates/pull/src/nse.rs:1026:51 False [G1 PS1]: NseError::HeaderUnexpected { got } if got.len() == MAX_DOCUMENT_BYTES
F crates/pull/src/nse.rs:1217:72 False [G1 PS1]: matches!(refused, NseError::TooManyIndexLinks { links } if links > MAX_INDEX_LINKS),
F crates/pull/src/nse.rs:1257:13 False [G1 PS1]: let NseError::NoConstituentLink { ref page } = why else {
F crates/pull/src/nse.rs:1258 [G1 PS1]: panic!("got {why:?}");
F crates/pull/src/nse_ps1_tests.rs:31:9 False [G1 PS1]: let NseError::NoConstituentLink { page } =
F crates/pull/src/nse_ps1_tests.rs:34 [G1 PS1]: panic!("an open value is no link");
F crates/pull/src/nse_ps1_tests.rs:62:9 False [G1 PS1]: let NseError::TooManyRows { rows, cap } =
F crates/pull/src/nse_ps1_tests.rs:65 [G1 PS1]: panic!("the bound is TooManyRows");
F crates/pull/src/padded_tests.rs:304:9 False [G2 PS2]: let DayKind::Open(s) = session else {
F crates/pull/src/padded_tests.rs:305 [G2 PS2]: panic!("2024-03-02 is a recorded special session: {session:?}");
U crates/pull/src/pricing.rs:597:8 True [G1 PS1]: if rate.basis() != basis {
U crates/pull/src/pricing.rs:598 [G1 PS1]: return Err(PricingError::BasisMismatch {
U crates/pull/src/pricing.rs:599 [G1 PS1]: rate: rate.basis(),
U crates/pull/src/pricing.rs:600 [G1 PS1]: asked: basis,
U crates/pull/src/pricing.rs:601 [G1 PS1]: });
F crates/pull/src/pricing.rs:1178:13 False [G1 PS1]: solved.volatility > 0.0 && solved.volatility < 5.0,
F crates/pull/src/pricing.rs:1178:40 False [G1 PS1]: solved.volatility > 0.0 && solved.volatility < 5.0,
F crates/pull/src/pricing.rs:1310:13 False [G1 PS1]: at_zero > at_twelve && at_five_five > at_eight,
F crates/pull/src/pricing.rs:1310:36 False [G1 PS1]: at_zero > at_twelve && at_five_five > at_eight,
F crates/pull/src/pricing.rs:1361:64 False [G1 PS1]: Err(PricingError::RateNotFinite { annual }) if annual.is_nan()
F crates/pull/src/pricing.rs:1399:79 False [G1 PS1]: matches!(error, PricingError::NotPositive { field: f, .. } if f == field),
F crates/pull/src/pricing.rs:1421 [G1 PS1]: matches!(
F crates/pull/src/pricing.rs:1434 [G1 PS1]: matches!(
F crates/pull/src/pricing.rs:1501:17 False [G1 PS1]: assert!(row.volatility > 0.0 && row.volatility < 5.0);
F crates/pull/src/pricing.rs:1501:41 False [G1 PS1]: assert!(row.volatility > 0.0 && row.volatility < 5.0);
F crates/pull/src/pricing.rs:1616 [G1 PS1]: matches!(
F crates/pull/src/pricing.rs:1833:45 False [G1 PS1]: OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
F crates/pull/src/pricing.rs:1833:64 False [G1 PS1]: OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
F crates/pull/src/pricing.rs:1834:44 False [G1 PS1]: OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
F crates/pull/src/pricing.rs:1834:63 False [G1 PS1]: OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
F crates/pull/src/pricing.rs:1836:21 False [G1 PS1]: assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
F crates/pull/src/pricing.rs:1836:50 False [G1 PS1]: assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
F crates/pull/src/pricing.rs:1873 [G1 PS1]: matches!(
F crates/pull/src/pricing.rs:1889:13 False [G1 PS1]: price.to_bits() == 100_f64.to_bits() && floor > price,
F crates/pull/src/pricing.rs:1889:53 False [G1 PS1]: price.to_bits() == 100_f64.to_bits() && floor > price,
F crates/pull/src/pricing.rs:1928:77 False [G1 PS1]: matches!(why, PricingError::NotPositive { field: f, .. } if f == field),
F crates/pull/src/pricing_ps1_tests.rs:56:9 False [G1 PS1]: let PricingError::LadderUnknown { why } =
F crates/pull/src/pricing_ps1_tests.rs:59 [G1 PS1]: panic!("a day the ladder does not cover is LadderUnknown");
F crates/pull/src/pricing_ps1_tests.rs:69:9 False [G1 PS1]: let PricingError::LadderUnknown { why: rung } =
F crates/pull/src/pricing_ps1_tests.rs:72 [G1 PS1]: panic!("a spot on rung zero is LadderUnknown");
U crates/pull/src/reference_data.rs:2519 [G1 PS1]: table,
U crates/pull/src/reference_data.rs:2520 [G1 PS1]: path: dir.clone(),
U crates/pull/src/reference_data.rs:2521 [G1 PS1]: generation: None,
U crates/pull/src/reference_data.rs:2522 [G1 PS1]: kind: RefusalKind::Row,
U crates/pull/src/reference_data.rs:2523 [G1 PS1]: detail: format!("the compiled table cannot be rendered: {why}"),
U crates/pull/src/reference_data.rs:2524 [G1 PS1]: })?;
U crates/pull/src/reference_data.rs:2526 [G1 PS1]: table,
U crates/pull/src/reference_data.rs:2527 [G1 PS1]: path: dir.clone(),
U crates/pull/src/reference_data.rs:2528 [G1 PS1]: generation: None,
U crates/pull/src/reference_data.rs:2529 [G1 PS1]: kind: RefusalKind::Row,
U crates/pull/src/reference_data.rs:2530 [G1 PS1]: detail: format!("the compiled table cannot be rendered: {why}"),
U crates/pull/src/reference_data.rs:2531 [G1 PS1]: })?;
U crates/pull/src/reference_data.rs:3212 [G1 PS1]: return Err(format!("day {day} inside the record is unmeasured"));
F crates/pull/src/reference_data_cov_tests.rs:391:9 False [G1 PS1]: current.contains(&"ZQA") && current.contains(&"ZQNOW"),
F crates/pull/src/reference_data_cov_tests.rs:391:37 False [G1 PS1]: current.contains(&"ZQA") && current.contains(&"ZQNOW"),
F crates/pull/src/reference_data_ps1_tests.rs:625:9 False [G1 PS1]: let Err(WriteError::Refused(refused)) = &outcome else {
F crates/pull/src/reference_data_ps1_tests.rs:626 [G1 PS1]: panic!("{outcome:?}");
F crates/pull/src/reference_data_ps1_tests.rs:651:9 False [G1 PS1]: let Err(WriteError::Draft(why)) = write(&store, &unrendered) else {
F crates/pull/src/reference_data_ps1_tests.rs:652 [G1 PS1]: panic!("a draft with no decision was written");
F crates/pull/src/reference_data_ps1_tests.rs:706:9 False [G1 PS1]: let Err(WriteError::Io(said)) = &outcome else {
F crates/pull/src/reference_data_ps1_tests.rs:707 [G1 PS1]: panic!("{outcome:?}");
F crates/pull/src/reference_data_ps1_tests.rs:760:9 False [G1 PS1]: let WriteError::Io(why) = write_linking(&store, &fno, &gone).unwrap_err() else {
F crates/pull/src/reference_data_ps1_tests.rs:761 [G1 PS1]: panic!("another fault is Io");
F crates/pull/src/reference_data_tests.rs:144:9 False [G1 PS1]: said.contains("index_identity") && said.contains("checksum"),
F crates/pull/src/reference_data_tests.rs:144:44 False [G1 PS1]: said.contains("index_identity") && said.contains("checksum"),
F crates/pull/src/reference_data_tests.rs:197:21 False [G1 PS1]: assert!(receipt.is_some() && refused.is_none());
F crates/pull/src/reference_data_tests.rs:197:42 False [G1 PS1]: assert!(receipt.is_some() && refused.is_none());
F crates/pull/src/reference_data_tests.rs:428 [G1 PS1]: other => panic!("{other:?}"),
F crates/pull/src/reference_data_tests.rs:481 [G1 PS1]: other => panic!("{other:?}"),
F crates/pull/src/reference_data_tests.rs:523:13 False [G1 PS1]: assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
F crates/pull/src/reference_data_tests.rs:523:47 False [G1 PS1]: assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
F crates/pull/src/reference_data_tests.rs:592 [G1 PS1]: mine[0]
F crates/pull/src/reference_data_tests.rs:774 [G1 PS1]: other => panic!("a span past MAX_ROWS days must be refused: {other:?}"),
F crates/pull/src/reference_data_tests.rs:881 [G1 PS1]: Err(other) => panic!("{other}"),
IGNORED crates/pull/src/reference_data_tests.rs:1042 [G1 PS1]: fn replay_a_real_rate_table_under_the_gap_rule() {
IGNORED crates/pull/src/reference_data_tests.rs:1043 [G1 PS1]: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
IGNORED crates/pull/src/reference_data_tests.rs:1043:9 False [G1 PS1]: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
IGNORED crates/pull/src/reference_data_tests.rs:1043:9 True [G1 PS1]: let Ok(path) = std::env::var("BRUTEX_R330_RATES") else {
IGNORED crates/pull/src/reference_data_tests.rs:1044 [G1 PS1]: panic!("BRUTEX_R330_RATES names no file");
IGNORED crates/pull/src/reference_data_tests.rs:1046 [G1 PS1]: let text = std::fs::read_to_string(&path).unwrap();
IGNORED crates/pull/src/reference_data_tests.rs:1047 [G1 PS1]: let draft = rate_draft(&text, &path, day(2026, 10, 20)).unwrap();
IGNORED crates/pull/src/reference_data_tests.rs:1048 [G1 PS1]: let store = scratch("ref-real-rates");
IGNORED crates/pull/src/reference_data_tests.rs:1049 [G1 PS1]: write(&store, &draft).unwrap();
IGNORED crates/pull/src/reference_data_tests.rs:1050 [G1 PS1]: let loaded = load(&store);
IGNORED crates/pull/src/reference_data_tests.rs:1051 [G1 PS1]: let series = loaded.rates.table().unwrap();
IGNORED crates/pull/src/reference_data_tests.rs:1052 [G1 PS1]: let first = i64::from(series.first_release().ordinal()) + 1;
IGNORED crates/pull/src/reference_data_tests.rs:1053 [G1 PS1]: let last = (i64::from(series.last_release().ordinal())
IGNORED crates/pull/src/reference_data_tests.rs:1054 [G1 PS1]: + i64::from(crate::riskfree::MAX_AGE_DAYS))
IGNORED crates/pull/src/reference_data_tests.rs:1055 [G1 PS1]: .min(crate::calendar::SOURCED_LAST_DAY);
IGNORED crates/pull/src/reference_data_tests.rs:1056 [G1 PS1]: let (mut sessions, mut stale, mut carried) = (0_u32, Vec::new(), Vec::new());
IGNORED crates/pull/src/reference_data_tests.rs:1057 [G1 PS1]: for at in first.max(crate::calendar::SOURCED_FIRST_DAY)..=last {
IGNORED crates/pull/src/reference_data_tests.rs:1058 [G1 PS1]: if !matches!(
IGNORED crates/pull/src/reference_data_tests.rs:1058:12 False [G1 PS1]: if !matches!(
IGNORED crates/pull/src/reference_data_tests.rs:1058:12 True [G1 PS1]: if !matches!(
IGNORED crates/pull/src/reference_data_tests.rs:1059 [G1 PS1]: crate::calendar::sourced_kind_of(at),
IGNORED crates/pull/src/reference_data_tests.rs:1062 [G1 PS1]: continue;
IGNORED crates/pull/src/reference_data_tests.rs:1063 [G1 PS1]: }
IGNORED crates/pull/src/reference_data_tests.rs:1064 [G1 PS1]: sessions += 1;
IGNORED crates/pull/src/reference_data_tests.rs:1065 [G1 PS1]: let civil = day_of(at).unwrap();
IGNORED crates/pull/src/reference_data_tests.rs:1066 [G1 PS1]: match series.on(crate::pricing::trade_day_of(civil).unwrap()) {
IGNORED crates/pull/src/reference_data_tests.rs:1067 [G1 PS1]: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
IGNORED crates/pull/src/reference_data_tests.rs:1067:26 False [G1 PS1]: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
IGNORED crates/pull/src/reference_data_tests.rs:1067:26 True [G1 PS1]: Ok(as_of) if as_of.carried.is_some() => carried.push(civil.to_string()),
IGNORED crates/pull/src/reference_data_tests.rs:1068 [G1 PS1]: Ok(_) => {}
IGNORED crates/pull/src/reference_data_tests.rs:1069 [G1 PS1]: Err(crate::riskfree::LookupError::Stale { .. }) => stale.push(civil.to_string()),
IGNORED crates/pull/src/reference_data_tests.rs:1070 [G1 PS1]: Err(crate::riskfree::LookupError::HoleBeforeGap { gap, .. }) => {
IGNORED crates/pull/src/reference_data_tests.rs:1071 [G1 PS1]: stale.push(format!(
IGNORED crates/pull/src/reference_data_tests.rs:1072 [G1 PS1]: "{civil} (gap {} follows an unexplained hole)",
IGNORED crates/pull/src/reference_data_tests.rs:1073 [G1 PS1]: gap.released
IGNORED crates/pull/src/reference_data_tests.rs:1074 [G1 PS1]: ));
IGNORED crates/pull/src/reference_data_tests.rs:1075 [G1 PS1]: }
IGNORED crates/pull/src/reference_data_tests.rs:1076 [G1 PS1]: Err(why) => panic!("{why}"),
IGNORED crates/pull/src/reference_data_tests.rs:1079 [G1 PS1]: eprintln!(
IGNORED crates/pull/src/reference_data_tests.rs:1082 [G1 PS1]: series.auction_count(),
IGNORED crates/pull/src/reference_data_tests.rs:1083 [G1 PS1]: series.gap_count(),
IGNORED crates/pull/src/reference_data_tests.rs:1084 [G1 PS1]: stale.len(),
IGNORED crates/pull/src/reference_data_tests.rs:1085 [G1 PS1]: carried.len()
IGNORED crates/pull/src/reference_data_tests.rs:1087 [G1 PS1]: }
F crates/pull/src/reference_data_tests.rs:1345:9 False [G1 PS1]: said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
F crates/pull/src/reference_data_tests.rs:1345:70 False [G1 PS1]: said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
U crates/pull/src/rehearsal.rs:917 [G1 PS1]: Err(why) => Err(Refusal::ListUnreadable {
U crates/pull/src/rehearsal.rs:918 [G1 PS1]: kind: kind.to_owned(),
U crates/pull/src/rehearsal.rs:919 [G1 PS1]: why: why.to_owned(),
U crates/pull/src/rehearsal.rs:920 [G1 PS1]: }),
U crates/pull/src/rehearsal.rs:922:12 False [G1 PS1]: if let Err(refusal) = &answer {
F crates/pull/src/rehearsal_tests.rs:275:9 False [G1 PS1]: why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
F crates/pull/src/rehearsal_tests.rs:275:39 False [G1 PS1]: why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
F crates/pull/src/rehearsal_tests.rs:277 [G1 PS1]: why.len()
F crates/pull/src/rehearsal_tests.rs:498:13 False [G1 PS1]: why.contains("S-90: operator_yes") && why.contains(says),
F crates/pull/src/rehearsal_tests.rs:498:51 False [G1 PS1]: why.contains("S-90: operator_yes") && why.contains(says),
U crates/pull/src/repair_plan.rs:268:12 True [G2 PS2]: if hours.kind() != SessionKind::Continuous {
U crates/pull/src/repair_plan.rs:269 [G2 PS2]: return None;
U crates/pull/src/repair_plan.rs:459:15 False [G2 PS2]: while let Ok(month) = at.year_month() {
U crates/pull/src/repair_plan.rs:471:13 False [G2 PS2]: let Ok(next) = Day::from_days(end.days_from_epoch().saturating_add(1)) else {
U crates/pull/src/repair_plan.rs:472 [G2 PS2]: break;
U crates/pull/src/repair_plan.rs:572:13 False [G2 PS2]: let Ok(day) = Day::from_days(number) else {
U crates/pull/src/repair_plan.rs:573 [G2 PS2]: continue;
F crates/pull/src/repair_plan.rs:958 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/src/repair_plan.rs:1008:39 False [G2 PS2]: assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
F crates/pull/src/repair_plan_not_owed_tests.rs:91:13 False [G3 PS3]: let super::Owes::Windows((windows, count)) = super::owed("CASH", on.days_from_epoch())
F crates/pull/src/repair_plan_not_owed_tests.rs:93 [G3 PS3]: panic!("{on}: a landed session states its windows");
F crates/pull/src/repair_plan_not_owed_tests.rs:171:13 False [G3 PS3]: let NotOwed::InstrumentInactive(interval) = session.why else {
F crates/pull/src/repair_plan_not_owed_tests.rs:172 [G3 PS3]: panic!("{}: {:?}", session.day, session.why);
F crates/pull/src/repair_plan_not_owed_tests.rs:282:22 False [G3 PS3]: .all(|m| !m.needs_repair() && m.refused.is_none())
F crates/pull/src/repair_plan_ps2_tests.rs:123:9 False [G2 PS2]: let [planned] = months.as_slice() else {
F crates/pull/src/repair_plan_ps2_tests.rs:124 [G2 PS2]: panic!("{months:?}");
F crates/pull/src/repair_plan_ps2_tests.rs:128:13 False [G2 PS2]: assert!(planned.short.is_empty() && planned.missing.is_empty());
F crates/pull/src/repair_plan_ps2_tests.rs:128:41 False [G2 PS2]: assert!(planned.short.is_empty() && planned.missing.is_empty());
F crates/pull/src/repair_plan_ps2_tests.rs:146:35 False [G2 PS2]: assert!(months.iter().all(|m| m.refused.is_none() && m.bars == 0));
U crates/pull/src/request_minutes.rs:377:22 False [G1 PS1]: Ok(hours) if hours.kind() == SessionKind::Continuous => hours,
U crates/pull/src/request_minutes.rs:378 [G1 PS1]: other => {
U crates/pull/src/request_minutes.rs:379 [G1 PS1]: return DayPlan::Unverified("venue hours", format!("venue hours {other:?}"));
U crates/pull/src/request_minutes.rs:455:13 False [G1 PS1]: let Ok(day) = Day::from_days(number) else {
U crates/pull/src/request_minutes.rs:456 [G1 PS1]: audit.findings.push(Finding {
U crates/pull/src/request_minutes.rs:457 [G1 PS1]: why: Why::Unverified,
U crates/pull/src/request_minutes.rs:458 [G1 PS1]: day: None,
U crates/pull/src/request_minutes.rs:459 [G1 PS1]: detail: "invalid day",
U crates/pull/src/request_minutes.rs:460 [G1 PS1]: minutes: 0,
U crates/pull/src/request_minutes.rs:461 [G1 PS1]: text: format!("request minute coverage UNVERIFIED: invalid day {number}"),
U crates/pull/src/request_minutes.rs:462 [G1 PS1]: });
U crates/pull/src/request_minutes.rs:463 [G1 PS1]: continue;
F crates/pull/src/request_minutes.rs:1283:13 False [G1 PS1]: only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
F crates/pull/src/request_minutes.rs:1283:47 False [G1 PS1]: only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
F crates/pull/src/request_minutes.rs:1300:13 False [G1 PS1]: whole.findings.is_empty() && whole.days.is_empty(),
F crates/pull/src/request_minutes.rs:1300:42 False [G1 PS1]: whole.findings.is_empty() && whole.days.is_empty(),
F crates/pull/src/request_minutes.rs:1320 [G1 PS1]: texts(&none)
F crates/pull/src/request_minutes.rs:1360 [G1 PS1]: panic!("{day} is attested")
F crates/pull/src/request_minutes.rs:1475:13 False [G1 PS1]: groups[1].text.starts_with(
F crates/pull/src/request_minutes.rs:1477:18 False [G1 PS1]: ) && groups[1].text.ends_with("the first: gap b"),
F crates/pull/src/request_minutes.rs:1479 [G1 PS1]: groups[1].text
F crates/pull/src/request_minutes.rs:1683:21 False [G1 PS1]: first.starts_with(head) && first.contains(" P0 "),
F crates/pull/src/request_minutes.rs:1683:48 False [G1 PS1]: first.starts_with(head) && first.contains(" P0 "),
F crates/pull/src/request_minutes.rs:1878:13 False [G1 PS1]: let [finding] = audit.findings.as_slice() else {
F crates/pull/src/request_minutes.rs:1879 [G1 PS1]: panic!("{:?}", audit.findings);
F crates/pull/src/request_minutes.rs:1886:13 False [G1 PS1]: finding
F crates/pull/src/request_minutes.rs:1889:20 False [G1 PS1]: && finding
F crates/pull/src/request_minutes.rs:1913:13 False [G1 PS1]: let Some(number) = found else {
F crates/pull/src/request_minutes.rs:1914 [G1 PS1]: panic!(
F crates/pull/src/request_minutes.rs:1950:13 False [G1 PS1]: let [group] = groups.as_slice() else {
F crates/pull/src/request_minutes.rs:1951 [G1 PS1]: panic!("{groups:?}");
F crates/pull/src/request_minutes.rs:2057:13 False [G1 PS1]: let [line] = audit.days.as_slice() else {
F crates/pull/src/request_minutes.rs:2058 [G1 PS1]: panic!("{:?}", audit.days);
F crates/pull/src/request_minutes.rs:2135:13 False [G1 PS1]: let [finding] = audit.findings.as_slice() else {
F crates/pull/src/request_minutes.rs:2136 [G1 PS1]: panic!("{:?}", audit.findings);
F crates/pull/src/request_minutes.rs:2179:13 False [G1 PS1]: let [failure] = done.failures.as_slice() else {
F crates/pull/src/request_minutes.rs:2180 [G1 PS1]: panic!("{:?}", done.failures);
F crates/pull/src/request_minutes.rs:2202 [G1 PS1]: lines[0]
F crates/pull/src/request_minutes.rs:2238 [G1 PS1]: named.failures[1].why
F crates/pull/src/request_minutes.rs:2254:17 False [G1 PS1]: let [finding] = audit.findings.as_slice() else {
F crates/pull/src/request_minutes.rs:2255 [G1 PS1]: panic!("{day}: {:?}", audit.findings);
F crates/pull/src/request_minutes.rs:2288:13 False [G1 PS1]: let [line] = audit.days.as_slice() else {
F crates/pull/src/request_minutes.rs:2289 [G1 PS1]: panic!("{:?}", audit.days);
F crates/pull/src/request_minutes.rs:2317:13 False [G1 PS1]: audit.findings.is_empty() && audit.days.is_empty(),
F crates/pull/src/request_minutes.rs:2317:42 False [G1 PS1]: audit.findings.is_empty() && audit.days.is_empty(),
F crates/pull/src/request_minutes.rs:2332:13 False [G1 PS1]: audit.findings.is_empty() && audit.days.is_empty(),
F crates/pull/src/request_minutes.rs:2332:42 False [G1 PS1]: audit.findings.is_empty() && audit.days.is_empty(),
F crates/pull/src/request_minutes.rs:2350:13 False [G1 PS1]: let [finding] = unflagged.findings.as_slice() else {
F crates/pull/src/request_minutes.rs:2351 [G1 PS1]: panic!("{:?}", unflagged.findings);
U crates/pull/src/resolve.rs:179:12 True [G3 PS3]: if !self.is_sound() {
U crates/pull/src/resolve.rs:180 [G3 PS3]: return Err(
U crates/pull/src/resolve.rs:181 [G3 PS3]: "a resolution's buckets do not add up to the names it published. \
U crates/pull/src/resolve.rs:182 [G3 PS3]: That is a defect in the JOIN and not in the data, so re-fetching \
U crates/pull/src/resolve.rs:183 [G3 PS3]: will not close it. Nothing was published."
U crates/pull/src/resolve.rs:184 [G3 PS3]: .to_owned(),
U crates/pull/src/resolve.rs:185 [G3 PS3]: );
U crates/pull/src/resolve.rs:378 [G3 PS3]: Err(why) => {
U crates/pull/src/resolve.rs:379 [G3 PS3]: let refusal = note_refused(page_url, why.to_string());
U crates/pull/src/resolve.rs:380 [G3 PS3]: failures.push(refusal);
U crates/pull/src/resolve.rs:381 [G3 PS3]: continue;
F crates/pull/src/resolve.rs:1033 [G3 PS3]: snap.unlinked[0]
F crates/pull/src/resolve.rs:1040 [G3 PS3]: snap.unlinked[0]
F crates/pull/src/resolve.rs:1045 [G3 PS3]: snap.unlinked[0]
U crates/pull/src/response_keep.rs:406:20 False [G2 PS2]: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
U crates/pull/src/response_keep.rs:437:16 False [G2 PS2]: if let Some(slot) = out.get_mut(124..) {
U crates/pull/src/response_keep.rs:587:16 False [G2 PS2]: if let Some(slot) = out.get_mut(0..8) {
U crates/pull/src/response_keep.rs:590:16 False [G2 PS2]: if let Some(slot) = out.get_mut(8..12) {
U crates/pull/src/response_keep.rs:593:16 False [G2 PS2]: if let Some(slot) = out.get_mut(12) {
U crates/pull/src/response_keep.rs:597:16 False [G2 PS2]: if let Some(slot) = out.get_mut(20..24) {
U crates/pull/src/response_keep.rs:1024:20 False [G2 PS2]: if let Some(slot) = out.get_mut(at..at + bytes.len()) {
U crates/pull/src/response_keep.rs:1038:16 False [G2 PS2]: if let Some(slot) = out.get_mut(60..) {
U crates/pull/src/response_keep.rs:1136 [G2 PS2]: _ => io::ErrorKind::Other,
U crates/pull/src/response_keep.rs:1467 [G2 PS2]: Ok(None) => Err(KeepError::NotRegular { path: path.clone() }),
U crates/pull/src/response_keep.rs:1654:9 False [G2 PS2]: let Some(file) = open_regular(path, true)? else {
U crates/pull/src/response_keep.rs:1655 [G2 PS2]: return Err(KeepError::NotRegular {
U crates/pull/src/response_keep.rs:1656 [G2 PS2]: path: path.to_path_buf(),
U crates/pull/src/response_keep.rs:1657 [G2 PS2]: });
F crates/pull/src/response_keep_tests.rs:324:9 False [G2 PS2]: let Err(KeepError::ReceiptsFull { pack: named }) = &refused else {
F crates/pull/src/response_keep_tests.rs:325 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/response_keep_tests.rs:533:9 False [G2 PS2]: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
F crates/pull/src/response_keep_tests.rs:533:43 False [G2 PS2]: shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
F crates/pull/src/response_keep_tests.rs:925 [G2 PS2]: matches!(
F crates/pull/src/response_keep_tests.rs:961:9 False [G2 PS2]: let Err(KeepError::LockStalled {
F crates/pull/src/response_keep_tests.rs:966 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/response_keep_tests.rs:1605 [G2 PS2]: started.elapsed()
F crates/pull/src/response_keep_tests.rs:2002:9 False [G2 PS2]: let [july] = outcome.further.as_slice() else {
F crates/pull/src/response_keep_tests.rs:2003 [G2 PS2]: panic!("one further month: {outcome:?}");
F crates/pull/src/response_keep_tests.rs:2091 [G2 PS2]: matches!(
F crates/pull/src/response_keep_tests.rs:2128:9 False [G2 PS2]: let [pointer] = july.as_slice() else {
F crates/pull/src/response_keep_tests.rs:2129 [G2 PS2]: panic!("July holds exactly the missing pointer: {july:?}");
F crates/pull/src/response_keep_tests.rs:2465:13 False [G2 PS2]: assert!(july.receipts.exists() && !july.bodies.exists());
F crates/pull/src/response_keep_tests.rs:2465:39 False [G2 PS2]: assert!(july.receipts.exists() && !july.bodies.exists());
F crates/pull/src/response_keep_tests.rs:2747:9 False [G2 PS2]: !at.bodies.exists() && !at.refusals.exists(),
F crates/pull/src/response_keep_tests.rs:2747:32 False [G2 PS2]: !at.bodies.exists() && !at.refusals.exists(),
F crates/pull/src/response_keep_tests.rs:3037:9 False [G2 PS2]: let [pointer] = months[1].1.receipts.as_slice() else {
F crates/pull/src/response_keep_tests.rs:3038 [G2 PS2]: panic!("segment 1 holds the one pointer: {months:?}");
F crates/pull/src/riskfree.rs:1109 [G2 PS2]: unreachable!("no gap row in this table, so nothing is carried")
F crates/pull/src/riskfree.rs:1595 [G2 PS2]: holed.on(asked)
F crates/pull/src/riskfree.rs:1632:13 False [G2 PS2]: said.contains("2020-01-20") && said.contains("unexplained"),
F crates/pull/src/riskfree.rs:1632:44 False [G2 PS2]: said.contains("2020-01-20") && said.contains("unexplained"),
F crates/pull/src/riskfree.rs:1831:56 False [G2 PS2]: matches!(tails.as_slice(), [small, big] if big.1 <= small.1.saturating_mul(10).max(2_000)),
U crates/pull/src/rolling.rs:147 [G3 PS3]: })?;
U crates/pull/src/rolling.rs:190 [G3 PS3]: },
F crates/pull/src/rolling.rs:1101:13 False [G3 PS3]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/src/rolling.rs:1102 [G3 PS3]: panic!("Dhan is an HTTP broker");
F crates/pull/src/rolling.rs:1428 [G3 PS3]: matches!(
F crates/pull/src/rolling.rs:1487:13 False [G3 PS3]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/src/rolling.rs:1488 [G3 PS3]: panic!("Dhan is an HTTP broker");
F crates/pull/src/rolling.rs:1564:52 False [G3 PS3]: Err(RollingError::NoExpiry { why }) if why.contains("not one this vendor serves")
F crates/pull/src/rolling.rs:1583:60 False [G3 PS3]: Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
F crates/pull/src/rolling.rs:1620:60 False [G3 PS3]: Err(RollingError::NoExpiry { why }) if why.contains("full regular session")
F crates/pull/src/rolling.rs:1633:52 False [G3 PS3]: Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
F crates/pull/src/rolling.rs:1745:77 False [G3 PS3]: Err(RollingError::Uncountable { field: "oi", text }) if text == cell
F crates/pull/src/rolling.rs:1751:17 False [G3 PS3]: assert!(said.contains("`oi`") && said.contains("1234.5"), "{said}");
F crates/pull/src/rolling.rs:1751:42 False [G3 PS3]: assert!(said.contains("`oi`") && said.contains("1234.5"), "{said}");
F crates/pull/src/rolling.rs:1807:95 False [G3 PS3]: matches!(&got, Err(RollingError::Unstampable { field: "timestamp", text }) if *text == stamp),
F crates/pull/src/rolling.rs:1828:92 False [G3 PS3]: matches!(&got, Err(RollingError::Uncountable { field: "volume", text }) if text == volume),
F crates/pull/src/rolling.rs:1849:13 False [G3 PS3]: said.contains("`timestamp`") && said.contains("null"),
F crates/pull/src/rolling.rs:1849:45 False [G3 PS3]: said.contains("`timestamp`") && said.contains("null"),
F crates/pull/src/rolling.rs:1862:89 False [G3 PS3]: matches!(&got, Err(RollingError::NotAPrice { field: "close", text }) if text == close),
F crates/pull/src/rolling.rs:1880:17 False [G3 PS3]: assert!(said.contains("`close`") && said.contains("-5"), "{said}");
F crates/pull/src/rolling.rs:1880:45 False [G3 PS3]: assert!(said.contains("`close`") && said.contains("-5"), "{said}");
U crates/pull/src/scrub.rs:219 [G1 PS1]: Err(why) => {
U crates/pull/src/scrub.rs:220 [G1 PS1]: return Finding::Unreadable {
U crates/pull/src/scrub.rs:221 [G1 PS1]: path: format!("{:?}", entry.key.symbol),
U crates/pull/src/scrub.rs:222 [G1 PS1]: why: format!("the entry does not name a legal store path: {why}"),
U crates/pull/src/scrub.rs:223 [G1 PS1]: };
F crates/pull/src/scrub.rs:718:50 False [G1 PS1]: Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty()
F crates/pull/src/scrub.rs:718:86 False [G1 PS1]: Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty()
F crates/pull/src/scrub.rs:759:31 False [G1 PS1]: i == j || one != other,
U crates/pull/src/series_identity.rs:437:13 False [G1 PS1]: let Some(range) = self.days.get(at) else {
U crates/pull/src/series_identity.rs:438 [G1 PS1]: return Verdict::Unchecked(Unchecked::NoDayBar);
F crates/pull/src/series_identity.rs:977:17 False [G1 PS1]: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
F crates/pull/src/series_identity.rs:977:39 False [G1 PS1]: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
F crates/pull/src/series_identity.rs:977:61 False [G1 PS1]: assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
U crates/pull/src/session.rs:1253 [G3 PS3]: return Err(SessionError::SessionLengthUnmeasured { days: day });
U crates/pull/src/session.rs:1262 [G3 PS3]: venue: why.venue().label(),
U crates/pull/src/session.rs:1263 [G3 PS3]: days: why.day().days_from_epoch(),
U crates/pull/src/session.rs:1264 [G3 PS3]: })?;
F crates/pull/src/session.rs:1749:17 False [G3 PS3]: let crate::calendar::DayKind::Open(session) = crate::calendar::kind_of(epoch_day)
F crates/pull/src/session.rs:1751 [G3 PS3]: panic!("{epoch_day} is an open day in the calendar");
F crates/pull/src/session.rs:1819:17 False [G3 PS3]: let DayKind::Open(window) = with_recorded_window(epoch_day, identity_answer) else {
F crates/pull/src/session.rs:1820 [G3 PS3]: panic!("{epoch_day}: a recorded window");
F crates/pull/src/session.rs:1884:17 False [G3 PS3]: assert!(text.contains("18935") && text.contains("length"), "{text}");
F crates/pull/src/session.rs:1884:43 False [G3 PS3]: assert!(text.contains("18935") && text.contains("length"), "{text}");
F crates/pull/src/session.rs:1985:76 False [G3 PS3]: c.to() == c.to().end_of_month() || c.to() == to || inside_a_wide_month,
F crates/pull/src/session.rs:1987 [G3 PS3]: c.from(),
F crates/pull/src/session.rs:1988 [G3 PS3]: c.to()
F crates/pull/src/session.rs:2251:13 False [G3 PS3]: let Err(why) = first.months_before(1) else {
F crates/pull/src/session.rs:2252 [G3 PS3]: panic!("there is no December 1969 in this calendar")
U crates/pull/src/ssm.rs:925 [G1 PS1]: SsmError::unreachable(format!("the HTTPS client could not be built: {why}"))
U crates/pull/src/ssm.rs:926 [G1 PS1]: })?;
GAP crates/pull/src/ssm.rs:966 [G1 PS1]: let status = answer.status();
GAP crates/pull/src/ssm.rs:970 [G1 PS1]: let text = answer_within(&mut answer).await?;
GAP crates/pull/src/ssm.rs:971 [G1 PS1]: answer_of(region, status.as_u16(), &text)
F crates/pull/src/ssm.rs:1387:45 False [G1 PS1]: FIXED.contains(name) || sent.contains(name),
F crates/pull/src/ssm.rs:1391 [G1 PS1]: token.is_some()
F crates/pull/src/ssm.rs:1478:17 False [G1 PS1]: let Err(SsmError { detail, kind }) = foreign.authorization(&identity()) else {
F crates/pull/src/ssm.rs:1479 [G1 PS1]: panic!("{region:?} is not ap-south-1 and must not be signed for")
F crates/pull/src/ssm.rs:1507:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = bad.authorization(&identity()) else {
F crates/pull/src/ssm.rs:1508 [G1 PS1]: panic!("a stamp with no date in it cannot scope a signature")
F crates/pull/src/ssm.rs:1549:17 False [G1 PS1]: let Err(SsmError { detail, .. }) = value_of(wrong) else {
F crates/pull/src/ssm.rs:1550 [G1 PS1]: panic!("{wrong} carries no Parameter.Value and must be refused")
F crates/pull/src/ssm.rs:1554:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = value_of("not json") else {
F crates/pull/src/ssm.rs:1555 [G1 PS1]: panic!("a non-JSON answer is a different fault and is named as one")
F crates/pull/src/ssm.rs:1656:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "nope") else {
F crates/pull/src/ssm.rs:1657 [G1 PS1]: panic!("a profile that is not in the file must be refused")
F crates/pull/src/ssm.rs:1666:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
F crates/pull/src/ssm.rs:1668 [G1 PS1]: panic!("an id with no secret cannot sign anything")
F crates/pull/src/ssm.rs:1692:17 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
F crates/pull/src/ssm.rs:1694 [G1 PS1]: panic!("{tag}: a blank half of the pair is no pair")
F crates/pull/src/ssm.rs:1717:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_lookup(env(&[
F crates/pull/src/ssm.rs:1721 [G1 PS1]: panic!("an empty exported key id must not become an identity")
F crates/pull/src/ssm.rs:1767:17 False [G1 PS1]: let Err(SsmError { detail, .. }) = found else {
F crates/pull/src/ssm.rs:1768 [G1 PS1]: panic!("a half-set environment must be refused: {found:?}")
F crates/pull/src/ssm.rs:1771:17 False [G1 PS1]: detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
F crates/pull/src/ssm.rs:1771:57 False [G1 PS1]: detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
F crates/pull/src/ssm.rs:1804:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&missing, "default")
F crates/pull/src/ssm.rs:1806 [G1 PS1]: panic!("a file that is not there cannot hold an identity")
F crates/pull/src/ssm.rs:1835:17 False [G1 PS1]: !value.contains("Put") && !value.contains("Delete"),
F crates/pull/src/ssm.rs:1835:43 False [G1 PS1]: !value.contains("Put") && !value.contains("Delete"),
F crates/pull/src/ssm.rs:1857:13 False [G1 PS1]: let Err(SsmError { detail, kind }) = get_parameter(
F crates/pull/src/ssm.rs:1865 [G1 PS1]: panic!("a foreign region must not yield a parameter value")
F crates/pull/src/ssm.rs:1884:17 False [G1 PS1]: let Err(SsmError { detail, .. }) = refuse_non_unicode(read) else {
F crates/pull/src/ssm.rs:1885 [G1 PS1]: panic!("{bad} not UTF-8 must be refused")
F crates/pull/src/ssm.rs:1907:13 False [G1 PS1]: !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
F crates/pull/src/ssm.rs:1907:50 False [G1 PS1]: !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
F crates/pull/src/ssm.rs:1911:13 False [G1 PS1]: shown.contains("ap-south-1") && shown.contains("<redacted>"),
F crates/pull/src/ssm.rs:1911:45 False [G1 PS1]: shown.contains("ap-south-1") && shown.contains("<redacted>"),
F crates/pull/src/ssm.rs:1948:13 False [G1 PS1]: let Ok(Err(SsmError { detail, kind })) = answer else {
F crates/pull/src/ssm.rs:1949 [G1 PS1]: if answer.is_err() {
F crates/pull/src/ssm.rs:1949:16 False [G1 PS1]: if answer.is_err() {
F crates/pull/src/ssm.rs:1949:16 True [G1 PS1]: if answer.is_err() {
F crates/pull/src/ssm.rs:1950 [G1 PS1]: // Release the reader blocked in open(2) before failing.
F crates/pull/src/ssm.rs:1951 [G1 PS1]: let _writer = std::fs::OpenOptions::new().write(true).open(&path);
F crates/pull/src/ssm.rs:1952 [G1 PS1]: }
F crates/pull/src/ssm.rs:1953 [G1 PS1]: panic!("a FIFO credentials file must refuse promptly, got {answer:?}")
F crates/pull/src/ssm.rs:1986:13 False [G1 PS1]: let Err(SsmError { detail, kind }) = AwsIdentity::from_credentials_file(&over, "default")
F crates/pull/src/ssm.rs:1988 [G1 PS1]: panic!("one byte past the bound must be refused")
F crates/pull/src/ssm.rs:1996:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&binary, "default")
F crates/pull/src/ssm.rs:1998 [G1 PS1]: panic!("a file that is not text holds no profile")
F crates/pull/src/ssm.rs:2049:13 False [G1 PS1]: let Err(SsmError { detail, kind }) = answered("", vec![b'a'; MAX_ANSWER_BYTES + 1]) else {
F crates/pull/src/ssm.rs:2050 [G1 PS1]: panic!("one byte past the cap must be refused")
F crates/pull/src/ssm.rs:2056:13 False [G1 PS1]: let Err(SsmError { detail, .. }) = answered("Content-Length: 4\r\n", b"ab".to_vec()) else {
F crates/pull/src/ssm.rs:2057 [G1 PS1]: panic!("a body cut short is not an answer")
F crates/pull/src/tenor.rs:490 [G3 PS3]: at.years(YearBasis::Calendar365)
F crates/pull/src/tenor.rs:510 [G3 PS3]: born.years(YearBasis::Calendar365)
F crates/pull/src/tenor.rs:676:13 False [G3 PS3]: words.starts_with("the expiry could not be settled: ") && words.contains("2024-03-29"),
F crates/pull/src/tenor.rs:676:71 False [G3 PS3]: words.starts_with("the expiry could not be settled: ") && words.contains("2024-03-29"),
U crates/pull/src/tickstore_write.rs:198:12 False [G3 PS3]: if identity.canonical_stem() == stem {
U crates/pull/src/tickstore_write.rs:201 [G3 PS3]: Err(refuse())
U crates/pull/src/tickstore_write.rs:855:9 False [G3 PS3]: let Some((&header, body)) = lines.split_first() else {
U crates/pull/src/tickstore_write.rs:856 [G3 PS3]: return Ok(None);
U crates/pull/src/tickstore_write.rs:1522:13 False [G3 PS3]: let (Some(stored), Some(entry)) = (index.get(at), entries.get(at)) else {
U crates/pull/src/tickstore_write.rs:1523 [G3 PS3]: return Err(verify_refusal("an entry went missing".to_owned()));
U crates/pull/src/tickstore_write.rs:1901 [G3 PS3]: _ => std::io::ErrorKind::Other,
U crates/pull/src/tickstore_write.rs:2010:13 False [G3 PS3]: let Some(entry) = entries.get(at) else {
U crates/pull/src/tickstore_write.rs:2011 [G3 PS3]: return Err(Refusal::EncodeFailed {
U crates/pull/src/tickstore_write.rs:2012 [G3 PS3]: name: String::new(),
U crates/pull/src/tickstore_write.rs:2013 [G3 PS3]: why: "an entry went missing".to_owned(),
U crates/pull/src/tickstore_write.rs:2014 [G3 PS3]: });
F crates/pull/src/tickstore_write_tests.rs:231:9 False [G3 PS3]: let Ok(Outcome::Written(report)) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:232 [G3 PS3]: panic!("written: {:?}", run.result);
F crates/pull/src/tickstore_write_tests.rs:358:9 False [G3 PS3]: let Ok(Outcome::Written(written)) = first.result else {
F crates/pull/src/tickstore_write_tests.rs:359 [G3 PS3]: panic!("{first:?}");
F crates/pull/src/tickstore_write_tests.rs:368:13 False [G3 PS3]: let Ok(Outcome::Held(held)) = again.result else {
F crates/pull/src/tickstore_write_tests.rs:369 [G3 PS3]: panic!("{again:?}");
F crates/pull/src/tickstore_write_tests.rs:398:9 False [G3 PS3]: let Ok(Outcome::Written(written)) = first.result else {
F crates/pull/src/tickstore_write_tests.rs:399 [G3 PS3]: panic!("{first:?}");
F crates/pull/src/tickstore_write_tests.rs:406:9 False [G3 PS3]: let Err(Refusal::HeldDayDiffers { path, why }) = &again.result else {
F crates/pull/src/tickstore_write_tests.rs:407 [G3 PS3]: panic!("{again:?}");
F crates/pull/src/tickstore_write_tests.rs:573 [G3 PS3]: data.len()
F crates/pull/src/tickstore_write_tests.rs:593:9 False [G3 PS3]: let Err(Refusal::MemberCorrupt { name, why }) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:594 [G3 PS3]: panic!("{run:?}");
F crates/pull/src/tickstore_write_tests.rs:698:13 False [G3 PS3]: let Err(Refusal::VerifyFailed { temp, .. }) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:699 [G3 PS3]: panic!("byte {at}: {run:?}");
F crates/pull/src/tickstore_write_tests.rs:861 [G3 PS3]: matches!(
F crates/pull/src/tickstore_write_tests.rs:903:9 False [G3 PS3]: let Ok(Outcome::Held(report)) = &held.result else {
F crates/pull/src/tickstore_write_tests.rs:904 [G3 PS3]: panic!("{held:?}");
F crates/pull/src/tickstore_write_tests.rs:910:9 False [G3 PS3]: let Ok(Outcome::Written(mine)) = &ours.result else {
F crates/pull/src/tickstore_write_tests.rs:911 [G3 PS3]: panic!("{ours:?}");
F crates/pull/src/tickstore_write_tests.rs:970:9 False [G3 PS3]: let Err(Refusal::HeldDayDiffers { why, .. }) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:971 [G3 PS3]: panic!("{run:?}");
F crates/pull/src/tickstore_write_tests.rs:1086 [G3 PS3]: String::from_utf8_lossy(field)
F crates/pull/src/tickstore_write_tests.rs:1135 [G3 PS3]: encoded.block.len(),
F crates/pull/src/tickstore_write_tests.rs:1136 [G3 PS3]: text.len()
IGNORED crates/pull/src/tickstore_write_tests.rs:1293 [G3 PS3]: fn measure_real_days_against_the_tools_files() {
IGNORED crates/pull/src/tickstore_write_tests.rs:1294 [G3 PS3]: let spec = std::env::var("BRUTEX_TSW_REAL").expect("BRUTEX_TSW_REAL names the days");
IGNORED crates/pull/src/tickstore_write_tests.rs:1295 [G3 PS3]: let out = PathBuf::from(std::env::var("BRUTEX_TSW_OUT").expect("BRUTEX_TSW_OUT"));
IGNORED crates/pull/src/tickstore_write_tests.rs:1296 [G3 PS3]: refuse_the_real_store(&out);
IGNORED crates/pull/src/tickstore_write_tests.rs:1297 [G3 PS3]: std::fs::create_dir_all(&out).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1298 [G3 PS3]: for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
IGNORED crates/pull/src/tickstore_write_tests.rs:1299 [G3 PS3]: let (zip, tool) = pair.split_once('=').expect("ZIP=TOOL_BTS");
IGNORED crates/pull/src/tickstore_write_tests.rs:1300 [G3 PS3]: let (zip, tool) = (Path::new(zip), Path::new(tool));
IGNORED crates/pull/src/tickstore_write_tests.rs:1301 [G3 PS3]: let name = zip.file_name().unwrap().to_string_lossy().into_owned();
IGNORED crates/pull/src/tickstore_write_tests.rs:1302 [G3 PS3]: let identity = DayIdentity::of_zip_name(&name).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1303 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1304 [G3 PS3]: let run = write_day(zip, &out, &WriteOptions::new("measure"));
IGNORED crates/pull/src/tickstore_write_tests.rs:1305 [G3 PS3]: let write_ns = start.elapsed().as_nanos();
IGNORED crates/pull/src/tickstore_write_tests.rs:1306 [G3 PS3]: let mine = match run.result {
IGNORED crates/pull/src/tickstore_write_tests.rs:1307 [G3 PS3]: Ok(Outcome::Written(report) | Outcome::Held(report)) => report,
IGNORED crates/pull/src/tickstore_write_tests.rs:1308 [G3 PS3]: Err(why) => panic!("{name}: {why}"),
IGNORED crates/pull/src/tickstore_write_tests.rs:1310 [G3 PS3]: let source = File::open(zip).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1311 [G3 PS3]: let len = source.metadata().unwrap().len();
IGNORED crates/pull/src/tickstore_write_tests.rs:1312 [G3 PS3]: let entries = zip_entries(&source, 0, len).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1313 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1314 [G3 PS3]: let theirs = verify_file(tool, &source, &entries, 8).expect("the tool's file matches");
IGNORED crates/pull/src/tickstore_write_tests.rs:1315 [G3 PS3]: let verify_ns = start.elapsed().as_nanos();
IGNORED crates/pull/src/tickstore_write_tests.rs:1316 [G3 PS3]: let largest = entries.iter().map(|e| e.len).max().unwrap_or(0);
IGNORED crates/pull/src/tickstore_write_tests.rs:1317 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1326 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1330 [G3 PS3]: u128::from(mine.stored_bytes) * 1_000 / u128::from(theirs.stored_bytes.max(1)),
IGNORED crates/pull/src/tickstore_write_tests.rs:1333 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1335 [G3 PS3]: write_ns / 1_000_000,
IGNORED crates/pull/src/tickstore_write_tests.rs:1336 [G3 PS3]: u128::from(mine.csv_bytes) * 1_000 / write_ns.max(1),
IGNORED crates/pull/src/tickstore_write_tests.rs:1337 [G3 PS3]: verify_ns / 1_000_000,
IGNORED crates/pull/src/tickstore_write_tests.rs:1338 [G3 PS3]: u128::from(mine.csv_bytes) * 1_000 / verify_ns.max(1)
IGNORED crates/pull/src/tickstore_write_tests.rs:1340 [G3 PS3]: for (label, path) in [
IGNORED crates/pull/src/tickstore_write_tests.rs:1341 [G3 PS3]: ("this writer", &mine.path),
IGNORED crates/pull/src/tickstore_write_tests.rs:1342 [G3 PS3]: ("the tool", &tool.to_path_buf()),
IGNORED crates/pull/src/tickstore_write_tests.rs:1343 [G3 PS3]: ] {
IGNORED crates/pull/src/tickstore_write_tests.rs:1344 [G3 PS3]: let file = File::open(path).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1345 [G3 PS3]: let size = file.metadata().unwrap().len();
IGNORED crates/pull/src/tickstore_write_tests.rs:1346 [G3 PS3]: let index = read_index(&file, size).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1347 [G3 PS3]: let mut ns = Vec::new();
IGNORED crates/pull/src/tickstore_write_tests.rs:1348 [G3 PS3]: let mut bytes = 0_u64;
IGNORED crates/pull/src/tickstore_write_tests.rs:1349 [G3 PS3]: let start = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1350 [G3 PS3]: for entry in index.iter().filter(|e| e.kind != KIND_DIR) {
IGNORED crates/pull/src/tickstore_write_tests.rs:1351 [G3 PS3]: let t = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1352 [G3 PS3]: let back = rebuild(&file, entry).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1353 [G3 PS3]: ns.push(t.elapsed().as_nanos());
IGNORED crates/pull/src/tickstore_write_tests.rs:1354 [G3 PS3]: bytes += back.len() as u64;
IGNORED crates/pull/src/tickstore_write_tests.rs:1355 [G3 PS3]: }
IGNORED crates/pull/src/tickstore_write_tests.rs:1356 [G3 PS3]: let all = start.elapsed().as_nanos();
IGNORED crates/pull/src/tickstore_write_tests.rs:1357 [G3 PS3]: let (p50, p99, max) = quantiles(ns.clone());
IGNORED crates/pull/src/tickstore_write_tests.rs:1358 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1360 [G3 PS3]: ns.len(),
IGNORED crates/pull/src/tickstore_write_tests.rs:1361 [G3 PS3]: u128::from(bytes) * 1_000 / all.max(1)
IGNORED crates/pull/src/tickstore_write_tests.rs:1365 [G3 PS3]: let mut ns = Vec::new();
IGNORED crates/pull/src/tickstore_write_tests.rs:1366 [G3 PS3]: match identity.tree {
IGNORED crates/pull/src/tickstore_write_tests.rs:1368 [G3 PS3]: let listed = NfoTickStore::new(&out).day(identity.day).unwrap().unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1369 [G3 PS3]: let tickers: Vec<String> = listed
IGNORED crates/pull/src/tickstore_write_tests.rs:1370 [G3 PS3]: .entries()
IGNORED crates/pull/src/tickstore_write_tests.rs:1371 [G3 PS3]: .iter()
IGNORED crates/pull/src/tickstore_write_tests.rs:1372 [G3 PS3]: .filter_map(|f| {
IGNORED crates/pull/src/tickstore_write_tests.rs:1373 [G3 PS3]: let tail = f.entry.rsplit(['/', '\\']).next()?;
IGNORED crates/pull/src/tickstore_write_tests.rs:1374 [G3 PS3]: Some(tail.strip_suffix(".NFO.csv")?.to_owned())
IGNORED crates/pull/src/tickstore_write_tests.rs:1375 [G3 PS3]: })
IGNORED crates/pull/src/tickstore_write_tests.rs:1376 [G3 PS3]: .collect();
IGNORED crates/pull/src/tickstore_write_tests.rs:1377 [G3 PS3]: for _ in 0..10 {
IGNORED crates/pull/src/tickstore_write_tests.rs:1378 [G3 PS3]: for ticker in &tickers {
IGNORED crates/pull/src/tickstore_write_tests.rs:1379 [G3 PS3]: let t = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1380 [G3 PS3]: std::hint::black_box(listed.locate(ticker).is_ok());
IGNORED crates/pull/src/tickstore_write_tests.rs:1381 [G3 PS3]: ns.push(t.elapsed().as_nanos());
IGNORED crates/pull/src/tickstore_write_tests.rs:1382 [G3 PS3]: }
IGNORED crates/pull/src/tickstore_write_tests.rs:1385 [G3 PS3]: Tree::Cm(kind) => {
IGNORED crates/pull/src/tickstore_write_tests.rs:1386 [G3 PS3]: let listing = TickStore::new(&out)
IGNORED crates/pull/src/tickstore_write_tests.rs:1387 [G3 PS3]: .day(kind, identity.day)
IGNORED crates/pull/src/tickstore_write_tests.rs:1388 [G3 PS3]: .unwrap()
IGNORED crates/pull/src/tickstore_write_tests.rs:1389 [G3 PS3]: .unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1390 [G3 PS3]: let stems: Vec<String> = listing
IGNORED crates/pull/src/tickstore_write_tests.rs:1391 [G3 PS3]: .entries()
IGNORED crates/pull/src/tickstore_write_tests.rs:1392 [G3 PS3]: .iter()
IGNORED crates/pull/src/tickstore_write_tests.rs:1393 [G3 PS3]: .filter_map(|f| {
IGNORED crates/pull/src/tickstore_write_tests.rs:1394 [G3 PS3]: let tail = f.entry.rsplit(['/', '\\']).next()?;
IGNORED crates/pull/src/tickstore_write_tests.rs:1395 [G3 PS3]: Some(tail.strip_suffix(".csv")?.to_owned())
IGNORED crates/pull/src/tickstore_write_tests.rs:1396 [G3 PS3]: })
IGNORED crates/pull/src/tickstore_write_tests.rs:1397 [G3 PS3]: .collect();
IGNORED crates/pull/src/tickstore_write_tests.rs:1398 [G3 PS3]: for _ in 0..10 {
IGNORED crates/pull/src/tickstore_write_tests.rs:1399 [G3 PS3]: for stem in &stems {
IGNORED crates/pull/src/tickstore_write_tests.rs:1400 [G3 PS3]: let t = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1401 [G3 PS3]: std::hint::black_box(listing.locate(stem).is_ok());
IGNORED crates/pull/src/tickstore_write_tests.rs:1402 [G3 PS3]: ns.push(t.elapsed().as_nanos());
IGNORED crates/pull/src/tickstore_write_tests.rs:1403 [G3 PS3]: }
IGNORED crates/pull/src/tickstore_write_tests.rs:1407 [G3 PS3]: let n = ns.len();
IGNORED crates/pull/src/tickstore_write_tests.rs:1408 [G3 PS3]: let (p50, p99, max) = quantiles(ns);
IGNORED crates/pull/src/tickstore_write_tests.rs:1409 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1412 [G3 PS3]: assert!(n > 0, "{name}: nothing was looked up");
IGNORED crates/pull/src/tickstore_write_tests.rs:1414 [G3 PS3]: }
IGNORED crates/pull/src/tickstore_write_tests.rs:1425 [G3 PS3]: fn measure_ruzstd_fastest_on_bar_months() {
IGNORED crates/pull/src/tickstore_write_tests.rs:1429 [G3 PS3]: let spec = std::env::var("BRUTEX_TSW_BAR_MONTHS").expect("BRUTEX_TSW_BAR_MONTHS");
IGNORED crates/pull/src/tickstore_write_tests.rs:1430 [G3 PS3]: for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
IGNORED crates/pull/src/tickstore_write_tests.rs:1431 [G3 PS3]: let (label, path) = pair.split_once('=').expect("LABEL=PATH");
IGNORED crates/pull/src/tickstore_write_tests.rs:1432 [G3 PS3]: let bytes = std::fs::read(path).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1433 [G3 PS3]: let rows = &bytes[HEADER..];
IGNORED crates/pull/src/tickstore_write_tests.rs:1434 [G3 PS3]: assert_eq!(rows.len() % ROW, 0, "{label}: whole rows");
IGNORED crates/pull/src/tickstore_write_tests.rs:1435 [G3 PS3]: let bars = rows.len() / ROW;
IGNORED crates/pull/src/tickstore_write_tests.rs:1436 [G3 PS3]: let whole = zstd(rows).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1437 [G3 PS3]: let frames: Vec<Vec<u8>> = rows
IGNORED crates/pull/src/tickstore_write_tests.rs:1438 [G3 PS3]: .chunks(ROW * PAGE)
IGNORED crates/pull/src/tickstore_write_tests.rs:1439 [G3 PS3]: .map(|chunk| zstd(chunk).unwrap())
IGNORED crates/pull/src/tickstore_write_tests.rs:1440 [G3 PS3]: .collect();
IGNORED crates/pull/src/tickstore_write_tests.rs:1441 [G3 PS3]: let paged: usize = frames.iter().map(Vec::len).sum::<usize>() + frames.len() * 8;
IGNORED crates/pull/src/tickstore_write_tests.rs:1442 [G3 PS3]: let mut state = 0x9E37_79B9_7F4A_7C15_u64;
IGNORED crates/pull/src/tickstore_write_tests.rs:1443 [G3 PS3]: let mut ns = Vec::new();
IGNORED crates/pull/src/tickstore_write_tests.rs:1444 [G3 PS3]: for _ in 0..2_000 {
IGNORED crates/pull/src/tickstore_write_tests.rs:1445 [G3 PS3]: state = state
IGNORED crates/pull/src/tickstore_write_tests.rs:1446 [G3 PS3]: .wrapping_mul(6_364_136_223_846_793_005)
IGNORED crates/pull/src/tickstore_write_tests.rs:1447 [G3 PS3]: .wrapping_add(1);
IGNORED crates/pull/src/tickstore_write_tests.rs:1448 [G3 PS3]: let at = usize::try_from(state >> 33).unwrap() % frames.len();
IGNORED crates/pull/src/tickstore_write_tests.rs:1449 [G3 PS3]: let want = rows.chunks(ROW * PAGE).nth(at).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1450 [G3 PS3]: let t = Instant::now();
IGNORED crates/pull/src/tickstore_write_tests.rs:1451 [G3 PS3]: let mut back = Vec::with_capacity(want.len());
IGNORED crates/pull/src/tickstore_write_tests.rs:1452 [G3 PS3]: let mut decoder =
IGNORED crates/pull/src/tickstore_write_tests.rs:1453 [G3 PS3]: ruzstd::decoding::StreamingDecoder::new(frames[at].as_slice()).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1454 [G3 PS3]: std::io::Read::read_to_end(&mut decoder, &mut back).unwrap();
IGNORED crates/pull/src/tickstore_write_tests.rs:1455 [G3 PS3]: ns.push(t.elapsed().as_nanos());
IGNORED crates/pull/src/tickstore_write_tests.rs:1456 [G3 PS3]: assert_eq!(back, want, "{label}: page {at} round trip");
IGNORED crates/pull/src/tickstore_write_tests.rs:1458 [G3 PS3]: let (p50, p99, max) = quantiles(ns);
IGNORED crates/pull/src/tickstore_write_tests.rs:1459 [G3 PS3]: println!(
IGNORED crates/pull/src/tickstore_write_tests.rs:1461 [G3 PS3]: rows.len(),
IGNORED crates/pull/src/tickstore_write_tests.rs:1462 [G3 PS3]: rows.len() * 1_000 / bars.max(1),
IGNORED crates/pull/src/tickstore_write_tests.rs:1463 [G3 PS3]: whole.len(),
IGNORED crates/pull/src/tickstore_write_tests.rs:1464 [G3 PS3]: whole.len() * 1_000 / bars.max(1),
IGNORED crates/pull/src/tickstore_write_tests.rs:1466 [G3 PS3]: paged * 1_000 / bars.max(1)
IGNORED crates/pull/src/tickstore_write_tests.rs:1469 [G3 PS3]: }
F crates/pull/src/tickstore_write_tests.rs:1576 [G3 PS3]: log_of(&root)
F crates/pull/src/tickstore_write_tests.rs:1588:9 False [G3 PS3]: let Err(Refusal::TempNotCreated { word, .. }) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:1589 [G3 PS3]: panic!("{run:?}");
F crates/pull/src/tickstore_write_tests.rs:1698:9 False [G3 PS3]: let Ok(Outcome::Written(report)) = &run.result else {
F crates/pull/src/tickstore_write_tests.rs:1699 [G3 PS3]: panic!("{run:?}");
F crates/pull/src/tickstore_write_tests.rs:1714:9 False [G3 PS3]: let Ok(Outcome::Written(written)) = first.result else {
F crates/pull/src/tickstore_write_tests.rs:1715 [G3 PS3]: panic!("{first:?}");
F crates/pull/src/tickstore_write_tests.rs:1728:9 False [G3 PS3]: let Err(Refusal::ZipCorruptOnVerify { path, why }) = &again.result else {
F crates/pull/src/tickstore_write_tests.rs:1729 [G3 PS3]: panic!("{again:?}");
F crates/pull/src/tickstore_write_tests.rs:1733:9 False [G3 PS3]: why.contains("member_corrupt") && why.contains("CRC-32"),
F crates/pull/src/tickstore_write_tests.rs:1733:43 False [G3 PS3]: why.contains("member_corrupt") && why.contains("CRC-32"),
F crates/pull/src/tickstore_write_tests.rs:1742:9 False [G3 PS3]: let Err(Refusal::HeldDayDiffers { why, .. }) = &dated.result else {
F crates/pull/src/tickstore_write_tests.rs:1743 [G3 PS3]: panic!("{dated:?}");
F crates/pull/src/tickstore_write_tests.rs:1746:9 False [G3 PS3]: why.contains("NIFTY.NSE_IDX.csv") && why.contains("DOS time"),
F crates/pull/src/tickstore_write_tests.rs:1746:46 False [G3 PS3]: why.contains("NIFTY.NSE_IDX.csv") && why.contains("DOS time"),
F crates/pull/src/tickstore_write_tests.rs:1940 [G3 PS3]: String::from_utf8_lossy(field)
U crates/pull/src/totp.rs:354:9 False [G3 PS3]: let Some(&last) = tag.last() else {
U crates/pull/src/totp.rs:356 [G3 PS3]: return Err(TotpError::Empty);
U crates/pull/src/totp.rs:359:9 False [G3 PS3]: let Some(window) = tag.get(offset..offset + 4) else {
U crates/pull/src/totp.rs:360 [G3 PS3]: return Err(TotpError::Empty);
F crates/pull/src/universe.rs:798 [G1 PS1]: master.len(),
F crates/pull/src/universe.rs:799 [G1 PS1]: index.len(),
F crates/pull/src/universe.rs:800 [G1 PS1]: index.capacity()
GAP crates/pull/src/upkeep.rs:1200:16 False [G1 PS1]: && why.kind() == ErrorKind::NotFound
GAP crates/pull/src/upkeep.rs:1580:20 True [G1 PS1]: if !scope.admits(2, &symbol) {
GAP crates/pull/src/upkeep.rs:1581 [G1 PS1]: continue;
GAP crates/pull/src/upkeep.rs:1676:20 False [G1 PS1]: if scope.prefix.len() <= 3 && into(walked, what, &path, true) {
F crates/pull/src/upkeep_tests.rs:372 [G1 PS1]: lines[0]
F crates/pull/src/upkeep_tests.rs:380 [G1 PS1]: lines[4]
F crates/pull/src/upkeep_tests.rs:388 [G1 PS1]: lines[6]
F crates/pull/src/upkeep_tests.rs:398 [G1 PS1]: lines[6]
GAP crates/pull/src/upkeep_tests.rs:460 [G1 PS1]: built.fetch_add(1, Ordering::Relaxed);
GAP crates/pull/src/upkeep_tests.rs:461 [G1 PS1]: clean(key, &at)
GAP crates/pull/src/upkeep_tests.rs:462 [G1 PS1]: };
F crates/pull/src/upkeep_tests.rs:472:9 False [G1 PS1]: why.contains("space") && why.contains("nothing was written"),
F crates/pull/src/upkeep_tests.rs:472:34 False [G1 PS1]: why.contains("space") && why.contains("nothing was written"),
F crates/pull/src/upkeep_tests.rs:521:9 False [G1 PS1]: why.contains("another srt upkeep run of gdfl holds")
F crates/pull/src/upkeep_tests.rs:522:16 False [G1 PS1]: && why.contains("one run of a kind at a time"),
GAP crates/pull/src/upkeep_tests.rs:1001:9 False [G1 PS1]: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
F crates/pull/src/upkeep_tests.rs:1001:47 False [G1 PS1]: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
GAP crates/pull/src/upkeep_tests.rs:1001:47 True [G1 PS1]: why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
F crates/pull/src/upkeep_tests.rs:1042:9 False [G1 PS1]: why.starts_with(&format!("{}: ", gone.join("audit").display()))
F crates/pull/src/upkeep_tests.rs:1043:16 False [G1 PS1]: && why.ends_with("; nothing was written"),
F crates/pull/src/upkeep_tests.rs:1399:9 False [G1 PS1]: why.starts_with("NSE/FNO/NIFTY/2019-03-28-1160000-CE names nothing under bars/gdfl: ")
F crates/pull/src/upkeep_tests.rs:1400:16 False [G1 PS1]: && why.ends_with("2019-03-28-1160000-CE is absent"),
F crates/pull/src/upkeep_tests.rs:1517:9 False [G1 PS1]: why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
F crates/pull/src/upkeep_tests.rs:1517:44 False [G1 PS1]: why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
F crates/pull/src/upkeep_tests.rs:1553:9 False [G1 PS1]: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
F crates/pull/src/upkeep_tests.rs:1553:42 False [G1 PS1]: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
F crates/pull/src/upkeep_tests.rs:1553:72 False [G1 PS1]: lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
F crates/pull/src/upkeep_tests.rs:1555:13 False [G1 PS1]: assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
F crates/pull/src/upkeep_tests.rs:1555:57 False [G1 PS1]: assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
GAP crates/pull/src/upkeep_tests.rs:1657:41 False [G1 PS1]: if text.starts_with("item ") && !self.refused.load(Ordering::Acquire) {
GAP crates/pull/src/upkeep_tests.rs:1741 [G1 PS1]: built.fetch_add(1, Ordering::Relaxed);
GAP crates/pull/src/upkeep_tests.rs:1742 [G1 PS1]: clean(key, &at)
GAP crates/pull/src/upkeep_tests.rs:1743 [G1 PS1]: };
F crates/pull/src/upkeep_tests.rs:1941:9 False [G1 PS1]: text.contains(
F crates/pull/src/upkeep_tests.rs:1944:14 False [G1 PS1]: ) && text.contains("end 300 how=stopped n=2 ok=2 of=6 t="),
F crates/pull/src/upkeep_tests.rs:2085:9 False [G1 PS1]: why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
F crates/pull/src/upkeep_tests.rs:2085:45 False [G1 PS1]: why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
U crates/pull/src/vendor.rs:504:31 True [G1 PS1]: Self::Minute30 if Timeframe::MINUTE_30.aligns_with_the_open() => {
U crates/pull/src/vendor.rs:505 [G1 PS1]: Some(Timeframe::MINUTE_30)
U crates/pull/src/vendor.rs:507:28 True [G1 PS1]: Self::Hour1 if Timeframe::MINUTE_60.aligns_with_the_open() => {
U crates/pull/src/vendor.rs:508 [G1 PS1]: Some(Timeframe::MINUTE_60)
U crates/pull/src/vendor.rs:1175:20 True [G1 PS1]: if interval_secs == 0 || !session_secs.is_multiple_of(interval_secs) {
F crates/pull/src/vendor.rs:6890 [G1 PS1]: other => panic!("{rung} sits on {other:?}, which the store cannot file"),
F crates/pull/src/vendor.rs:7011 [G1 PS1]: _ => false,
F crates/pull/src/vendor.rs:7082 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7088:37 True [G1 PS1]: !(named_in_query && named_in_path),
F crates/pull/src/vendor.rs:7090 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7096 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7121 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7159 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7199 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7258 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7263 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:7293:17 False [G1 PS1]: assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
F crates/pull/src/vendor.rs:7293:53 False [G1 PS1]: assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
F crates/pull/src/vendor.rs:7317:17 False [G1 PS1]: assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
F crates/pull/src/vendor.rs:7317:56 False [G1 PS1]: assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
F crates/pull/src/vendor.rs:7415:13 False [G1 PS1]: index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
F crates/pull/src/vendor.rs:7415:60 False [G1 PS1]: index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
F crates/pull/src/vendor.rs:7673:39 False [G1 PS1]: assert!(i == j || one != other, "offset {one} appears twice");
F crates/pull/src/vendor.rs:7741:13 False [G1 PS1]: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
F crates/pull/src/vendor.rs:7742 [G1 PS1]: panic!("Groww is an HTTP feed");
F crates/pull/src/vendor.rs:7759:13 False [G1 PS1]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/src/vendor.rs:7760 [G1 PS1]: panic!("Dhan is an HTTP feed");
F crates/pull/src/vendor.rs:8194:17 False [G1 PS1]: !row.granularities.is_empty() && !row.segments.is_empty(),
F crates/pull/src/vendor.rs:8194:50 False [G1 PS1]: !row.granularities.is_empty() && !row.segments.is_empty(),
F crates/pull/src/vendor.rs:8225 [G1 PS1]: panic!(
F crates/pull/src/vendor.rs:8229 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:8236 [G1 PS1]: feed.display()
F crates/pull/src/vendor.rs:8275:13 False [G1 PS1]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/src/vendor.rs:8276 [G1 PS1]: panic!("Dhan is an HTTP broker");
F crates/pull/src/vendor.rs:8487 [G1 PS1]: spec.path_template()
F crates/pull/src/vendor.rs:8658:13 False [G1 PS1]: fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
F crates/pull/src/vendor.rs:8658:54 False [G1 PS1]: fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
F crates/pull/src/vendor.rs:8777:13 False [G1 PS1]: !TRUEDATA_INDEX.columns.contains(&Column::Volume)
F crates/pull/src/vendor.rs:8778:20 False [G1 PS1]: && !TRUEDATA_INDEX.columns.contains(&Column::OpenInterest),
F crates/pull/src/vendor.rs:9043:48 False [G1 PS1]: row.contested.is_none() || !row.binds_because.is_empty(),
F crates/pull/src/vendor.rs:9195 [G1 PS1]: other.standing.label()
F crates/pull/src/vendor.rs:9460:13 False [G1 PS1]: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
F crates/pull/src/vendor.rs:9461 [G1 PS1]: panic!("Groww is an HTTP feed");
F crates/pull/src/vendor.rs:9743:13 False [G1 PS1]: OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
F crates/pull/src/vendor.rs:9743:54 False [G1 PS1]: OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
F crates/pull/src/vendor.rs:9771:17 False [G1 PS1]: assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
F crates/pull/src/vendor.rs:9771:54 False [G1 PS1]: assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
F crates/pull/src/vendor.rs:9796:17 False [G1 PS1]: assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
F crates/pull/src/vendor.rs:9796:53 False [G1 PS1]: assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
F crates/pull/src/vendor.rs:9822:17 False [G1 PS1]: assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
F crates/pull/src/vendor.rs:9822:45 False [G1 PS1]: assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
F crates/pull/src/vendor.rs:9850:17 False [G1 PS1]: assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
F crates/pull/src/vendor.rs:9850:51 False [G1 PS1]: assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
F crates/pull/src/vendor.rs:9951:17 False [G1 PS1]: assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
F crates/pull/src/vendor.rs:9951:45 False [G1 PS1]: assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
F crates/pull/src/vendor.rs:10054:17 False [G1 PS1]: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
F crates/pull/src/vendor.rs:10054:54 False [G1 PS1]: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
F crates/pull/src/vendor.rs:10054:80 False [G1 PS1]: said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
F crates/pull/src/verdict_words.rs:184:21 False [G1 PS1]: !word.is_empty()
F crates/pull/src/verdict_words.rs:185:28 False [G1 PS1]: && word
F crates/pull/src/verdict_words.rs:256 [G1 PS1]: cause.code()
U crates/pull/src/zerodha_spool.rs:423 [G2 PS2]: other => Self::io(at, "open", io::Error::other(other.to_string())),
U crates/pull/src/zerodha_spool.rs:869 [G2 PS2]: why: format!("day {}: {why}", record.day),
U crates/pull/src/zerodha_spool.rs:870 [G2 PS2]: })?;
U crates/pull/src/zerodha_spool.rs:912:17 False [G2 PS2]: let Some(entry) = open.get_mut(&id) else {
U crates/pull/src/zerodha_spool.rs:913 [G2 PS2]: continue;
U crates/pull/src/zerodha_spool.rs:1092 [G2 PS2]: path: path.to_path_buf(),
U crates/pull/src/zerodha_spool.rs:1093 [G2 PS2]: why: format!("its length {len} does not fit this host's memory"),
U crates/pull/src/zerodha_spool.rs:1094 [G2 PS2]: })?;
U crates/pull/src/zerodha_spool.rs:1427:16 False [G2 PS2]: && let Some(dir) = path.parent()
F crates/pull/src/zerodha_spool_cov_tests.rs:221:13 False [G2 PS2]: said.starts_with("record_refused: detail kind")
F crates/pull/src/zerodha_spool_cov_tests.rs:222:20 False [G2 PS2]: && said.contains("a spool detail is a kind above 0 and at most 128 bytes"),
F crates/pull/src/zerodha_spool_cov_tests.rs:229:9 False [G2 PS2]: let Ok(Decoded::Detail(_, kept)) = decode_record(&bytes[160..]) else {
F crates/pull/src/zerodha_spool_cov_tests.rs:230 [G2 PS2]: panic!("the widest detail decodes");
F crates/pull/src/zerodha_spool_cov_tests.rs:243:13 False [G2 PS2]: assert!(!root.join(SPOOL_DIR).exists() && !root.join(LIVE_DIR).exists());
F crates/pull/src/zerodha_spool_cov_tests.rs:243:47 False [G2 PS2]: assert!(!root.join(SPOOL_DIR).exists() && !root.join(LIVE_DIR).exists());
F crates/pull/src/zerodha_spool_cov_tests.rs:256:9 False [G2 PS2]: let Err(SpoolError::Io {
F crates/pull/src/zerodha_spool_cov_tests.rs:262 [G2 PS2]: panic!("{probed:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:273:83 False [G2 PS2]: matches!(&refused, Err(SpoolError::Io { action: "open", path: p, .. }) if *p == path),
F crates/pull/src/zerodha_spool_cov_tests.rs:287:9 False [G2 PS2]: let Live::Taken(held) = live_probe(&root, 7).unwrap() else {
F crates/pull/src/zerodha_spool_cov_tests.rs:288 [G2 PS2]: panic!("a fresh run's lock is free");
F crates/pull/src/zerodha_spool_cov_tests.rs:294:9 False [G2 PS2]: let Err(SpoolError::Busy { run, holder }) = &refused else {
F crates/pull/src/zerodha_spool_cov_tests.rs:295 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:335:9 False [G2 PS2]: let Err(SpoolError::Io {
F crates/pull/src/zerodha_spool_cov_tests.rs:341 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:401:13 False [G2 PS2]: let Err(SpoolError::Damaged {
F crates/pull/src/zerodha_spool_cov_tests.rs:407 [G2 PS2]: panic!("{why}: {refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:466:9 False [G2 PS2]: let Err(SpoolError::Io { action, source, .. }) = &refused else {
F crates/pull/src/zerodha_spool_cov_tests.rs:467 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:474:68 False [G2 PS2]: matches!(&refused, Err(SpoolError::NotRegular { path }) if *path == dir),
F crates/pull/src/zerodha_spool_cov_tests.rs:520:9 False [G2 PS2]: let Err(SpoolError::Io { action, source, .. }) = &refused else {
F crates/pull/src/zerodha_spool_cov_tests.rs:521 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:556:9 False [G2 PS2]: let SpoolError::Io {
F crates/pull/src/zerodha_spool_cov_tests.rs:562 [G2 PS2]: panic!("{refused:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:578:74 False [G2 PS2]: matches!(read_log(&dir), Err(SpoolError::NotRegular { path }) if path == dir),
F crates/pull/src/zerodha_spool_cov_tests.rs:582:83 False [G2 PS2]: matches!(read_spool(&dir, &id()), Err(SpoolError::NotRegular { path }) if path == dir),
F crates/pull/src/zerodha_spool_cov_tests.rs:600:9 False [G2 PS2]: let Err(SpoolError::Io {
F crates/pull/src/zerodha_spool_cov_tests.rs:606 [G2 PS2]: panic!("{probed:?}");
F crates/pull/src/zerodha_spool_cov_tests.rs:632:9 False [G2 PS2]: let Err(SpoolError::Io {
F crates/pull/src/zerodha_spool_cov_tests.rs:638 [G2 PS2]: panic!("{refused:?}");
U crates/pull/src/zerodha_verify.rs:236:40 False [G3 PS3]: .filter(|tf| tf.secs() > 60 && tf.secs().is_multiple_of(60) && *tf != Timeframe::DAY_1)
U crates/pull/src/zerodha_verify.rs:667:9 False [G3 PS3]: let Ok(day) = Day::from_days(number) else {
U crates/pull/src/zerodha_verify.rs:668 [G3 PS3]: return Permit::Unattested;
U crates/pull/src/zerodha_verify.rs:670:9 False [G3 PS3]: let Some(venue) = crate::vendor::Venue::for_segment(Exchange::Nse, segment) else {
U crates/pull/src/zerodha_verify.rs:671 [G3 PS3]: return Permit::Unsized;
U crates/pull/src/zerodha_verify.rs:678:13 False [G3 PS3]: let Ok(hours) = venue.share_hours_on(day, cas) else {
U crates/pull/src/zerodha_verify.rs:679 [G3 PS3]: return Permit::Unsized;
U crates/pull/src/zerodha_verify.rs:685:23 False [G3 PS3]: Some(span) if span.0 < span.1 => Permit::Windows(([span, (0, 0)], 1)),
U crates/pull/src/zerodha_verify.rs:686 [G3 PS3]: _ => Permit::Unsized,
U crates/pull/src/zerodha_verify.rs:816:13 False [G3 PS3]: let Some(bar) = scaled(row, utc_s) else {
U crates/pull/src/zerodha_verify.rs:817 [G3 PS3]: refuse(&mut out, RowCause::Price.code());
U crates/pull/src/zerodha_verify.rs:818 [G3 PS3]: continue;
U crates/pull/src/zerodha_verify.rs:1581:12 False [G3 PS3]: if let Some(first) = buckets.iter().find(|b| b.complete) {
U crates/pull/src/zerodha_verify.rs:1749:36 True [G3 PS3]: && (report.differed > 0 || report.contradicted > 0)
U crates/pull/src/zerodha_verify.rs:2051:16 False [G3 PS3]: if let Some(slot) = self.by_code.get_mut(usize::from(code.code())) {
U crates/pull/src/zerodha_verify.rs:2367:9 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
U crates/pull/src/zerodha_verify.rs:2369 [G3 PS3]: faults.push("the Zerodha descriptor is not an HTTP feed".to_owned());
U crates/pull/src/zerodha_verify.rs:2370 [G3 PS3]: return (out, faults);
U crates/pull/src/zerodha_verify.rs:3194:13 False [G3 PS3]: let Ok(mut day) = Day::new(month.year(), month.month(), 1) else {
U crates/pull/src/zerodha_verify.rs:3195 [G3 PS3]: continue;
U crates/pull/src/zerodha_verify.rs:3376:13 False [G3 PS3]: let Some(rungs) = held.as_ref() else {
U crates/pull/src/zerodha_verify.rs:3377 [G3 PS3]: continue;
U crates/pull/src/zerodha_verify.rs:3463 [G3 PS3]: Err("the Zerodha descriptor is not an HTTP feed".to_owned())
U crates/pull/src/zerodha_verify.rs:4010:16 False [G3 PS3]: if let Some(group) = groups.get_mut(slot) {
U crates/pull/src/zerodha_verify.rs:4020:21 False [G3 PS3]: let Some(id) = spools.get(at) else { continue };
U crates/pull/src/zerodha_verify.rs:4046:16 False [G3 PS3]: if let Some(slot) = slots.get_mut(at) {
U crates/pull/src/zerodha_verify.rs:4167:13 False [G3 PS3]: let Some(((exchange, day, _, _), _)) = keyed.get(start).copied() else {
U crates/pull/src/zerodha_verify.rs:4168 [G3 PS3]: break;
U crates/pull/src/zerodha_verify.rs:4295:15 False [G3 PS3]: while let Some(at) = dir {
U crates/pull/src/zerodha_verify.rs:4449:21 False [G3 PS3]: let Some(base) = entries.get_mut(index) else {
U crates/pull/src/zerodha_verify.rs:4450 [G3 PS3]: continue;
F crates/pull/src/zerodha_verify_cov_store_tests.rs:1526:9 False [G3 PS3]: let Err(refused) = lock_answer(
F crates/pull/src/zerodha_verify_cov_store_tests.rs:1532 [G3 PS3]: panic!("the host's refusal answered a lock");
F crates/pull/src/zerodha_verify_cov_store_tests.rs:1538:9 False [G3 PS3]: let Err(busy) = lock_answer(&path, Err(std::fs::TryLockError::WouldBlock)) else {
F crates/pull/src/zerodha_verify_cov_store_tests.rs:1539 [G3 PS3]: panic!("a lock held elsewhere answered a lock");
F crates/pull/src/zerodha_verify_cov_tests.rs:408:9 False [G3 PS3]: let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", muhurat.days_from_epoch())
F crates/pull/src/zerodha_verify_cov_tests.rs:410 [G3 PS3]: panic!("the Muhurat of record owes windows");
F crates/pull/src/zerodha_verify_tests.rs:185 [G3 PS3]: Found::NoRecord(none) => panic!("no record: {none:?}"),
F crates/pull/src/zerodha_verify_tests.rs:215:35 False [G3 PS3]: DayKind::Open(session) if session == calendar::Session::full()
F crates/pull/src/zerodha_verify_tests.rs:221:9 False [G3 PS3]: let Owes::Windows(windows) = repair_plan::owed("INDEX", regular().days_from_epoch()) else {
F crates/pull/src/zerodha_verify_tests.rs:222 [G3 PS3]: panic!("a regular day owes windows");
F crates/pull/src/zerodha_verify_tests.rs:761:9 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
F crates/pull/src/zerodha_verify_tests.rs:763 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/zerodha_verify_tests.rs:1566:9 False [G3 PS3]: let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", muhurat.days_from_epoch())
F crates/pull/src/zerodha_verify_tests.rs:1568 [G3 PS3]: panic!("the Muhurat of record owes windows");
F crates/pull/src/zerodha_verify_tests.rs:1575:13 False [G3 PS3]: assert!(!owed.is_empty() && owed.len() < 375);
F crates/pull/src/zerodha_verify_tests.rs:1575:33 False [G3 PS3]: assert!(!owed.is_empty() && owed.len() < 375);
F crates/pull/src/zerodha_verify_tests.rs:1649:13 False [G3 PS3]: assert!(absent.faults.is_empty() && absent.lines == 0);
F crates/pull/src/zerodha_verify_tests.rs:1649:41 False [G3 PS3]: assert!(absent.faults.is_empty() && absent.lines == 0);
F crates/pull/src/zerodha_verify_tests.rs:2478:9 False [G3 PS3]: report.busy[0].contains("5min") && report.busy[0].contains("month_busy"),
F crates/pull/src/zerodha_verify_tests.rs:2478:44 False [G3 PS3]: report.busy[0].contains("5min") && report.busy[0].contains("month_busy"),
F crates/pull/src/zerodha_verify_tests.rs:2597 [G3 PS3]: other => panic!("a log past the cap is refused by name: {other:?}"),
F crates/pull/src/zerodha_verify_tests.rs:2609 [G3 PS3]: other => panic!("a log of the cap is read: {other:?}"),
F crates/pull/src/zerodha_verify_tests.rs:2959:9 False [G3 PS3]: let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
F crates/pull/src/zerodha_verify_tests.rs:2961 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/src/zerodha_verify_tests.rs:3623:17 False [G3 PS3]: assert!(open_before > 0 && open_after > 0, "{old}: both parts trade");
F crates/pull/src/zerodha_verify_tests.rs:3623:36 False [G3 PS3]: assert!(open_before > 0 && open_after > 0, "{old}: both parts trade");
F crates/pull/src/zerodha_verify_tests.rs:4112:13 False [G3 PS3]: assert!(one.faults.is_empty() && four.faults.is_empty(), "{four:?}");
F crates/pull/src/zerodha_verify_tests.rs:4112:38 False [G3 PS3]: assert!(one.faults.is_empty() && four.faults.is_empty(), "{four:?}");
F crates/pull/src/zerodha_verify_tests.rs:4138:13 False [G3 PS3]: assert!(merge_workers() >= 1 && merge_workers() <= MAX_MERGE_WORKERS);
F crates/pull/src/zerodha_verify_tests.rs:4138:37 False [G3 PS3]: assert!(merge_workers() >= 1 && merge_workers() <= MAX_MERGE_WORKERS);
F crates/pull/src/zerodha_verify_tests.rs:4288 [G3 PS3]: first.live[0].2
F crates/pull/src/zerodha_verify_tests.rs:4290:13 False [G3 PS3]: assert!(first.faults.is_empty() && first.refused.is_empty());
F crates/pull/src/zerodha_verify_tests.rs:4290:40 False [G3 PS3]: assert!(first.faults.is_empty() && first.refused.is_empty());
F crates/pull/src/zerodha_verify_tests.rs:4320:13 False [G3 PS3]: assert!(last.live.is_empty() && last.waiting == 0, "{last:?}");
F crates/pull/src/zerodha_verify_tests.rs:4320:37 False [G3 PS3]: assert!(last.live.is_empty() && last.waiting == 0, "{last:?}");
F crates/pull/src/zerodha_verify_tests.rs:4405 [G3 PS3]: spool::Live::Spooling(holder) => panic!("free run 15 read as live: {holder}"),
F crates/pull/src/zerodha_verify_tests.rs:4502:13 False [G3 PS3]: assert!(rest.refused.is_empty() && rest.waiting == 0, "{rest:?}");
F crates/pull/src/zerodha_verify_tests.rs:4502:40 False [G3 PS3]: assert!(rest.refused.is_empty() && rest.waiting == 0, "{rest:?}");
F crates/pull/src/zerodha_verify_tests.rs:4794 [G3 PS3]: other => panic!("a full spool took the append: {other:?}"),
F crates/pull/src/zerodha_verify_tests.rs:5056:13 False [G3 PS3]: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
F crates/pull/src/zerodha_verify_tests.rs:5056:42 False [G3 PS3]: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
F crates/pull/src/zerodha_verify_tests.rs:5056:70 False [G3 PS3]: assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
F crates/pull/src/zerodha_verify_tests.rs:5135 [G3 PS3]: other => panic!("not the host's words: {other}"),
F crates/pull/tests/alerts_seams.rs:59 [G1 PS1]: Put::Duplicate { ordinal } => panic!("a fresh key was a duplicate of #{ordinal}"),
F crates/pull/tests/allocation.rs:173 [G1 PS1]: wide.len()
F crates/pull/tests/allocation.rs:302:17 False [G1 PS1]: let Err(FrameError::Truncated { part }) = frame_answer.0 else {
F crates/pull/tests/allocation.rs:303 [G1 PS1]: panic!("a frame cut to {len} bytes was {:?}", frame_answer.0);
F crates/pull/tests/allocation.rs:321:13 False [G1 PS1]: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
F crates/pull/tests/allocation.rs:321:36 False [G1 PS1]: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
F crates/pull/tests/allocation.rs:323 [G1 PS1]: full.len()
F crates/pull/tests/anchor.rs:78 [G3 PS3]: first - OPEN_UTC
F crates/pull/tests/anchor.rs:175:13 False [G3 PS3]: let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
F crates/pull/tests/anchor.rs:176 [G3 PS3]: panic!("known session")
F crates/pull/tests/anchor.rs:441:9 False [G3 PS3]: let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
F crates/pull/tests/anchor.rs:442 [G3 PS3]: panic!("known session")
F crates/pull/tests/anchor.rs:560:13 False [G3 PS3]: diagnostics[0].contains("closed") && diagnostics[0].contains("store defect"),
F crates/pull/tests/anchor.rs:560:50 False [G3 PS3]: diagnostics[0].contains("closed") && diagnostics[0].contains("store defect"),
F crates/pull/tests/anchor.rs:562 [G3 PS3]: diagnostics[0]
F crates/pull/tests/anchor.rs:593:9 False [G3 PS3]: rendered.contains("-7") && rendered.contains("negative"),
F crates/pull/tests/anchor.rs:593:36 False [G3 PS3]: rendered.contains("-7") && rendered.contains("negative"),
F crates/pull/tests/anchor.rs:639 [G3 PS3]: matches!(
F crates/pull/tests/anchor.rs:648:9 False [G3 PS3]: rendered.contains("Refused rather than saturated") && rendered.contains("an i64"),
F crates/pull/tests/anchor.rs:648:63 False [G3 PS3]: rendered.contains("Refused rather than saturated") && rendered.contains("an i64"),
F crates/pull/tests/anchor.rs:725 [G3 PS3]: first - midnight_edge
F crates/pull/tests/broker.rs:398 [G2 PS2]: bars.display()
F crates/pull/tests/broker.rs:766 [G2 PS2]: at("1day").display()
F crates/pull/tests/broker.rs:892:9 False [G2 PS2]: why.contains("1min") && why.contains("1day"),
F crates/pull/tests/broker.rs:892:33 False [G2 PS2]: why.contains("1min") && why.contains("1day"),
F crates/pull/tests/broker.rs:923 [G2 PS2]: PathSegment::Value { placeholder, .. } => {
F crates/pull/tests/broker.rs:924 [G2 PS2]: panic!("a discovery path names a value: {placeholder}")
F crates/pull/tests/broker.rs:931:9 False [G2 PS2]: let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
F crates/pull/tests/broker.rs:932 [G2 PS2]: panic!("Groww is an HTTP broker");
F crates/pull/tests/broker.rs:963:9 False [G2 PS2]: let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
F crates/pull/tests/broker.rs:964 [G2 PS2]: panic!("Dhan is an HTTP broker");
F crates/pull/tests/broker.rs:1093 [G2 PS2]: contract.as_str()
F crates/pull/tests/broker.rs:1409:13 False [G2 PS2]: assert!(bins.is_empty() && ovls.is_empty());
F crates/pull/tests/broker.rs:1409:32 False [G2 PS2]: assert!(bins.is_empty() && ovls.is_empty());
F crates/pull/tests/census.rs:304 [G1 PS1]: path.display()
F crates/pull/tests/census.rs:332:9 False [G1 PS1]: total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
F crates/pull/tests/census.rs:332:45 False [G1 PS1]: total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
F crates/pull/tests/census.rs:334 [G1 PS1]: rungs(),
F crates/pull/tests/census.rs:492:9 False [G1 PS1]: total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
F crates/pull/tests/census.rs:492:33 False [G1 PS1]: total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
F crates/pull/tests/census.rs:494 [G1 PS1]: rungs()
F crates/pull/tests/census.rs:620 [G1 PS1]: done.failures[0].why
F crates/pull/tests/census.rs:655:13 False [G1 PS1]: why.contains("BSE") && why.contains("D-0017"),
F crates/pull/tests/census.rs:655:36 False [G1 PS1]: why.contains("BSE") && why.contains("D-0017"),
F crates/pull/tests/census.rs:724 [G1 PS1]: done.failures[0].why
F crates/pull/tests/census.rs:743 [G1 PS1]: done.failures[0].why
F crates/pull/tests/census.rs:750 [G1 PS1]: done.failures[0].why
F crates/pull/tests/census.rs:775:9 False [G1 PS1]: why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
F crates/pull/tests/census.rs:775:53 False [G1 PS1]: why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
F crates/pull/tests/census.rs:937 [G1 PS1]: done.failures[0].why
F crates/pull/tests/census.rs:1028:9 False [G1 PS1]: total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
F crates/pull/tests/census.rs:1028:38 False [G1 PS1]: total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
F crates/pull/tests/census.rs:1104 [G1 PS1]: panic!(
F crates/pull/tests/census.rs:1127 [G1 PS1]: store.join("bars").display()
F crates/pull/tests/census.rs:1130:9 False [G1 PS1]: done.counted == 0 && done.rows_read > 0,
F crates/pull/tests/census.rs:1130:30 False [G1 PS1]: done.counted == 0 && done.rows_read > 0,
F crates/pull/tests/census.rs:1199 [G1 PS1]: n + 2
F crates/pull/tests/census.rs:1222 [G1 PS1]: n + 2,
F crates/pull/tests/census.rs:1371:9 False [G1 PS1]: total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
F crates/pull/tests/census.rs:1371:41 False [G1 PS1]: total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
F crates/pull/tests/census_day_pace.rs:126:13 False [G1 PS1]: line.contains(&format!("\"read\":\"{read}\""))
F crates/pull/tests/census_day_pace.rs:127:20 False [G1 PS1]: && line.contains(&format!("\"why\":\"{why}\"")),
F crates/pull/tests/census_day_pace.rs:129 [G1 PS1]: at + 1
F crates/pull/tests/census_day_pace.rs:132:13 False [G1 PS1]: line.contains("\"load_us\":") && line.contains("\"total_us\":"),
F crates/pull/tests/census_day_pace.rs:132:46 False [G1 PS1]: line.contains("\"load_us\":") && line.contains("\"total_us\":"),
F crates/pull/tests/census_resume.rs:127:9 False [G2 PS2]: let [(rows_one, _)] = cash_row(root)[..] else {
F crates/pull/tests/census_resume.rs:128 [G2 PS2]: panic!("one CASH row after day one: {:?}", cash_row(root));
F crates/pull/tests/census_resume.rs:146:9 False [G2 PS2]: let ImportRefusal::CensusRefused { day: at, .. } = &stopped else {
F crates/pull/tests/census_resume.rs:147 [G2 PS2]: panic!("{stopped:?}");
F crates/pull/tests/census_resume.rs:190:9 False [G2 PS2]: let [(rows_two, _)] = cash_row(root)[..] else {
F crates/pull/tests/census_resume.rs:191 [G2 PS2]: panic!("one CASH row after the recount: {:?}", cash_row(root));
F crates/pull/tests/census_resume.rs:208:9 False [G2 PS2]: let [(rows_three, _)] = cash_row(root)[..] else {
F crates/pull/tests/census_resume.rs:209 [G2 PS2]: panic!("one CASH row after day three: {:?}", cash_row(root));
F crates/pull/tests/census_resume.rs:258:36 False [G2 PS2]: .is_some_and(|journal| journal.closed.is_empty() && journal.run.is_none()),
IGNORED crates/pull/tests/census_v4.rs:866 [G3 PS3]: fn the_audit_scale_census_migrates_and_compacts_with_equal_totals() {
IGNORED crates/pull/tests/census_v4.rs:867 [G3 PS3]: let scratch = Scratch::new("SCALE");
IGNORED crates/pull/tests/census_v4.rs:868 [G3 PS3]: let root = scratch.root();
IGNORED crates/pull/tests/census_v4.rs:869 [G3 PS3]: let keys = 242_341u32;
IGNORED crates/pull/tests/census_v4.rs:870 [G3 PS3]: let entries = 2_093_049u32;
IGNORED crates/pull/tests/census_v4.rs:871 [G3 PS3]: let index_keys = 2_341u32;
IGNORED crates/pull/tests/census_v4.rs:874 [G3 PS3]: let started = std::time::Instant::now();
IGNORED crates/pull/tests/census_v4.rs:875 [G3 PS3]: let mut single = Manifest::open(Vendor::Gdfl, &[], &[]).expect("genesis");
IGNORED crates/pull/tests/census_v4.rs:876 [G3 PS3]: for i in 0..entries {
IGNORED crates/pull/tests/census_v4.rs:877 [G3 PS3]: let n = i % keys;
IGNORED crates/pull/tests/census_v4.rs:878 [G3 PS3]: let pass = u64::from(i / keys) + 1;
IGNORED crates/pull/tests/census_v4.rs:879 [G3 PS3]: let key = if n < index_keys {
IGNORED crates/pull/tests/census_v4.rs:879:22 False [G3 PS3]: let key = if n < index_keys {
IGNORED crates/pull/tests/census_v4.rs:879:22 True [G3 PS3]: let key = if n < index_keys {
IGNORED crates/pull/tests/census_v4.rs:880 [G3 PS3]: index_key(n)
IGNORED crates/pull/tests/census_v4.rs:882 [G3 PS3]: option_key(n - index_keys)
IGNORED crates/pull/tests/census_v4.rs:884 [G3 PS3]: single.record_held(row(key, pass)).expect("recorded");
IGNORED crates/pull/tests/census_v4.rs:886 [G3 PS3]: assert_eq!(
IGNORED crates/pull/tests/census_v4.rs:887 [G3 PS3]: (single.entries(), single.keys()),
IGNORED crates/pull/tests/census_v4.rs:888 [G3 PS3]: (u64::from(entries), u64::from(keys))
IGNORED crates/pull/tests/census_v4.rs:890 [G3 PS3]: let legacy = manifest_path(root, Vendor::Gdfl);
IGNORED crates/pull/tests/census_v4.rs:891 [G3 PS3]: std::fs::write(&legacy, single.image()).expect("written");
IGNORED crates/pull/tests/census_v4.rs:892 [G3 PS3]: let legacy_bytes = std::fs::read(&legacy).expect("bytes");
IGNORED crates/pull/tests/census_v4.rs:893 [G3 PS3]: let built = started.elapsed();
IGNORED crates/pull/tests/census_v4.rs:895 [G3 PS3]: let started = std::time::Instant::now();
IGNORED crates/pull/tests/census_v4.rs:896 [G3 PS3]: let done = census_store::migrate(root, Vendor::Gdfl).expect("it migrates");
IGNORED crates/pull/tests/census_v4.rs:897 [G3 PS3]: let migrated = started.elapsed();
IGNORED crates/pull/tests/census_v4.rs:900 [G3 PS3]: assert!(done.seal.as_ref().is_some_and(Result::is_ok), "sealed");
IGNORED crates/pull/tests/census_v4.rs:901 [G3 PS3]: assert_eq!(
IGNORED crates/pull/tests/census_v4.rs:902 [G3 PS3]: std::fs::read(census_store::superseded_path(root, Vendor::Gdfl)).expect("bytes"),
IGNORED crates/pull/tests/census_v4.rs:906 [G3 PS3]: let (_, fno) = current(root, Segment::Fno);
IGNORED crates/pull/tests/census_v4.rs:907 [G3 PS3]: let (_, index) = current(root, Segment::Index);
IGNORED crates/pull/tests/census_v4.rs:908 [G3 PS3]: let (_, cash) = current(root, Segment::Cash);
IGNORED crates/pull/tests/census_v4.rs:909 [G3 PS3]: assert_eq!(fno.keys() + index.keys() + cash.keys(), single.keys());
IGNORED crates/pull/tests/census_v4.rs:910 [G3 PS3]: assert_eq!(
IGNORED crates/pull/tests/census_v4.rs:911 [G3 PS3]: fno.total_rows() + index.total_rows() + cash.total_rows(),
IGNORED crates/pull/tests/census_v4.rs:912 [G3 PS3]: single.total_rows()
IGNORED crates/pull/tests/census_v4.rs:914 [G3 PS3]: assert_eq!(fno.entries(), fno.keys(), "migrated one row per key");
IGNORED crates/pull/tests/census_v4.rs:915 [G3 PS3]: assert_eq!(fno.capacity(), FNO_FLOOR);
IGNORED crates/pull/tests/census_v4.rs:916 [G3 PS3]: assert_eq!(fno.headroom(), FNO_FLOOR - fno.keys());
IGNORED crates/pull/tests/census_v4.rs:919 [G3 PS3]: let log_keys = 242_000u32;
IGNORED crates/pull/tests/census_v4.rs:920 [G3 PS3]: let log_entries = 2_200_000u32;
IGNORED crates/pull/tests/census_v4.rs:921 [G3 PS3]: let mut census =
IGNORED crates/pull/tests/census_v4.rs:922 [G3 PS3]: Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, FNO_FLOOR).expect("genesis");
IGNORED crates/pull/tests/census_v4.rs:923 [G3 PS3]: for i in 0..log_entries {
IGNORED crates/pull/tests/census_v4.rs:924 [G3 PS3]: census
IGNORED crates/pull/tests/census_v4.rs:925 [G3 PS3]: .record_held(row(option_key(i % log_keys), u64::from(i / log_keys) + 1))
IGNORED crates/pull/tests/census_v4.rs:926 [G3 PS3]: .expect("recorded");
IGNORED crates/pull/tests/census_v4.rs:927 [G3 PS3]: }
IGNORED crates/pull/tests/census_v4.rs:928 [G3 PS3]: assert!(
IGNORED crates/pull/tests/census_v4.rs:929 [G3 PS3]: census.entries() > MAX_ENTRIES,
IGNORED crates/pull/tests/census_v4.rs:932 [G3 PS3]: let path = install(root, Segment::Fno, 2, &census);
IGNORED crates/pull/tests/census_v4.rs:933 [G3 PS3]: let started = std::time::Instant::now();
IGNORED crates/pull/tests/census_v4.rs:934 [G3 PS3]: let loaded = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
IGNORED crates/pull/tests/census_v4.rs:935 [G3 PS3]: let read = started.elapsed();
IGNORED crates/pull/tests/census_v4.rs:936 [G3 PS3]: assert_eq!(
IGNORED crates/pull/tests/census_v4.rs:937 [G3 PS3]: (loaded.entries(), loaded.keys(), loaded.total_rows()),
IGNORED crates/pull/tests/census_v4.rs:938 [G3 PS3]: (census.entries(), census.keys(), census.total_rows())
IGNORED crates/pull/tests/census_v4.rs:940 [G3 PS3]: let started = std::time::Instant::now();
IGNORED crates/pull/tests/census_v4.rs:941 [G3 PS3]: let compacted = loaded.compacted().expect("it compacts");
IGNORED crates/pull/tests/census_v4.rs:942 [G3 PS3]: let compact = started.elapsed();
IGNORED crates/pull/tests/census_v4.rs:943 [G3 PS3]: assert_eq!(compacted.entries(), u64::from(log_keys));
IGNORED crates/pull/tests/census_v4.rs:944 [G3 PS3]: assert_eq!(compacted.total_rows(), census.total_rows());
IGNORED crates/pull/tests/census_v4.rs:945 [G3 PS3]: let path = install(root, Segment::Fno, 3, &compacted);
IGNORED crates/pull/tests/census_v4.rs:946 [G3 PS3]: let started = std::time::Instant::now();
IGNORED crates/pull/tests/census_v4.rs:947 [G3 PS3]: let reopened = census_store::read_epoch(&path, Vendor::Gdfl, Segment::Fno).expect("reads");
IGNORED crates/pull/tests/census_v4.rs:948 [G3 PS3]: let reread = started.elapsed();
IGNORED crates/pull/tests/census_v4.rs:949 [G3 PS3]: assert_eq!(
IGNORED crates/pull/tests/census_v4.rs:950 [G3 PS3]: (reopened.entries(), reopened.keys(), reopened.total_rows()),
IGNORED crates/pull/tests/census_v4.rs:951 [G3 PS3]: (u64::from(log_keys), census.keys(), census.total_rows())
IGNORED crates/pull/tests/census_v4.rs:953 [G3 PS3]: println!(
IGNORED crates/pull/tests/census_v4.rs:958 [G3 PS3]: fno.keys(),
IGNORED crates/pull/tests/census_v4.rs:959 [G3 PS3]: index.keys(),
IGNORED crates/pull/tests/census_v4.rs:960 [G3 PS3]: cash.keys(),
IGNORED crates/pull/tests/census_v4.rs:961 [G3 PS3]: done.partitions
IGNORED crates/pull/tests/census_v4.rs:962 [G3 PS3]: .iter()
IGNORED crates/pull/tests/census_v4.rs:963 [G3 PS3]: .find(|part| part.segment == Segment::Fno)
IGNORED crates/pull/tests/census_v4.rs:964 [G3 PS3]: .map_or(0, |part| part.bytes),
IGNORED crates/pull/tests/census_v4.rs:965 [G3 PS3]: HEADER_LEN + u64::from(log_entries) * 128,
IGNORED crates/pull/tests/census_v4.rs:966 [G3 PS3]: std::fs::metadata(&path).expect("its metadata").len()
IGNORED crates/pull/tests/census_v4.rs:968 [G3 PS3]: }
F crates/pull/tests/cloud_mutants_import.rs:307 [G2 PS2]: other => panic!("a rotted record read as {other:?}"),
F crates/pull/tests/cloud_mutants_import.rs:335:13 False [G2 PS2]: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
F crates/pull/tests/cloud_mutants_import.rs:335:56 False [G2 PS2]: why.contains("without block checksums") && why.contains(&bin.display().to_string()),
F crates/pull/tests/cloud_mutants_import.rs:373 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/tests/cloud_mutants_import.rs:402 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/tests/cloud_mutants_import.rs:497:9 False [G2 PS2]: said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
F crates/pull/tests/cloud_mutants_import.rs:498:16 False [G2 PS2]: && said.contains("Refused rather than saturated"),
F crates/pull/tests/cloud_mutants_import.rs:867 [G2 PS2]: panic!(
F crates/pull/tests/cloud_mutants_import.rs:869 [G2 PS2]: path.display(),
F crates/pull/tests/cloud_mutants_import.rs:870 [G2 PS2]: tried.join("; ")
F crates/pull/tests/cloud_mutants_import.rs:897 [G2 PS2]: link.display()
F crates/pull/tests/cloud_mutants_import.rs:915:42 False [G2 PS2]: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
F crates/pull/tests/cloud_mutants_import.rs:927 [G2 PS2]: link.display()
F crates/pull/tests/cloud_mutants_import.rs:952 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/tests/cloud_mutants_import.rs:1019:9 False [G2 PS2]: gap.starts_with("gap NSE-INDEX-NIFTY 2025-07-01 v=1 rung=1min owed=375 held=374 missing=1")
F crates/pull/tests/cloud_mutants_import.rs:1020:16 False [G2 PS2]: && gap.ends_with("why=vendor_empty run=4242"),
F crates/pull/tests/cloud_mutants_import.rs:1312:26 True [G2 PS2]: .any(|m| m.contains(".BE") || m.contains("ZZSHARE"))
F crates/pull/tests/cloud_mutants_import.rs:1382:82 False [G2 PS2]: matches!(listing.auction_for(&cash("TCS")), Auction::Unknown(why) if !why.is_empty())
F crates/pull/tests/cloud_mutants_import.rs:1386:38 False [G2 PS2]: Auction::Unknown(why) if why.contains("records no ISIN")
F crates/pull/tests/cloud_mutants_import.rs:1458:38 False [G2 PS2]: Auction::Unknown(why) if why.contains("core::universe records no ISIN")
F crates/pull/tests/cloud_mutants_import.rs:1802:13 False [G2 PS2]: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
F crates/pull/tests/cloud_mutants_import.rs:1802:56 False [G2 PS2]: lines[0].contains("\"rows_refused\":2") && lines[0].contains("\"malformed\":2"),
F crates/pull/tests/cloud_mutants_import.rs:1804 [G2 PS2]: lines[0]
F crates/pull/tests/derive.rs:167 [G3 PS3]: tf.as_str()
F crates/pull/tests/derive.rs:287:13 False [G3 PS3]: assert!(fut.is_future() && !fut.is_option());
F crates/pull/tests/derive.rs:287:32 False [G3 PS3]: assert!(fut.is_future() && !fut.is_option());
F crates/pull/tests/derive.rs:288:13 False [G3 PS3]: assert!(ce.is_option() && pe.is_option());
F crates/pull/tests/derive.rs:288:31 False [G3 PS3]: assert!(ce.is_option() && pe.is_option());
F crates/pull/tests/derive.rs:418 [G3 PS3]: conflicts[0].why
F crates/pull/tests/derive.rs:425 [G3 PS3]: conflicts[0].why
F crates/pull/tests/derive.rs:433:22 False [G3 PS3]: .all(|f| f.why.contains("incomplete or invalid minute coverage")
F crates/pull/tests/derive.rs:479:22 False [G3 PS3]: .any(|f| f.why.contains("UNVERIFIED") && f.why.contains("cash eligibility"))
F crates/pull/tests/derive.rs:623 [G3 PS3]: refusal.why.len(),
F crates/pull/tests/derive.rs:624 [G3 PS3]: refusal.why.get(..300)
F crates/pull/tests/derive.rs:698 [G3 PS3]: reports[0].why
F crates/pull/tests/derive.rs:747:9 False [G3 PS3]: audit[0]
F crates/pull/tests/derive.rs:750:16 False [G3 PS3]: && audit[0]
F crates/pull/tests/derive.rs:753:16 False [G3 PS3]: && audit[0].why.contains("observed trading does not attest"),
F crates/pull/tests/derive.rs:755 [G3 PS3]: audit[0].why
F crates/pull/tests/derive.rs:758:9 False [G3 PS3]: audit[1].why.contains("2027-01-07")
F crates/pull/tests/derive.rs:759:16 False [G3 PS3]: && audit[1]
F crates/pull/tests/derive.rs:766:22 False [G3 PS3]: .any(|f| !f.why.starts_with("request minute coverage")
F crates/pull/tests/derive.rs:887 [G3 PS3]: gaps[1]
F crates/pull/tests/derive.rs:984 [G3 PS3]: done.failures[0].why
F crates/pull/tests/derive.rs:1058 [G3 PS3]: done.failures[0].why
F crates/pull/tests/derive.rs:1323:42 False [G3 PS3]: assert!(done.failures.iter().all(|f| !f.why.contains("versioned")
F crates/pull/tests/derive.rs:1407:20 False [G3 PS3]: && f.why.contains(expected)
F crates/pull/tests/derive.rs:1477 [G3 PS3]: tf.as_str()
F crates/pull/tests/derive.rs:1747:9 False [G3 PS3]: per_day.contains("read_entry(root, day)") && per_day.contains("decode(&bytes)"),
F crates/pull/tests/derive.rs:1747:54 False [G3 PS3]: per_day.contains("read_entry(root, day)") && per_day.contains("decode(&bytes)"),
F crates/pull/tests/folder.rs:188:67 False [G1 PS1]: assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Dhan));
F crates/pull/tests/folder.rs:262:9 False [G1 PS1]: let Reach::Days {
F crates/pull/tests/folder.rs:269 [G1 PS1]: panic!("a folder holding two days must report them: {reach:?}")
F crates/pull/tests/folder.rs:394:67 False [G1 PS1]: assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Groww));
F crates/pull/tests/folder.rs:418:64 False [G1 PS1]: assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
F crates/pull/tests/folder.rs:455:9 False [G1 PS1]: let Reach::Days {
F crates/pull/tests/folder.rs:459 [G1 PS1]: panic!("{reach:?}")
F crates/pull/tests/folder.rs:751:64 False [G1 PS1]: assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
F crates/pull/tests/folder.rs:921:9 False [G1 PS1]: let Reach::Days {
F crates/pull/tests/folder.rs:925 [G1 PS1]: panic!("{:?}", census.reach)
F crates/pull/tests/futures_events.rs:88:9 False [G3 PS3]: let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
F crates/pull/tests/futures_events.rs:90 [G3 PS3]: panic!("Zerodha is an HTTP feed");
F crates/pull/tests/futures_events.rs:207:13 False [G3 PS3]: line.contains("\"line\":\"ZQIDX 2023-11-30-FUT: ") && line.contains("\"detail\":\""),
F crates/pull/tests/futures_events.rs:207:67 False [G3 PS3]: line.contains("\"line\":\"ZQIDX 2023-11-30-FUT: ") && line.contains("\"detail\":\""),
F crates/pull/tests/gdfl_bad_row_audit.rs:230 [G3 PS3]: lines[0]
F crates/pull/tests/gdfl_bad_row_audit.rs:235 [G3 PS3]: lines[0]
F crates/pull/tests/gdfl_bad_row_audit.rs:293 [G3 PS3]: lines[0]
F crates/pull/tests/gdfl_bad_row_audit.rs:336 [G3 PS3]: lines[0]
F crates/pull/tests/gdfl_bad_row_audit.rs:341 [G3 PS3]: lines[0]
F crates/pull/tests/gdfl_bad_row_audit.rs:345:9 False [G3 PS3]: lines[0].contains("\"target\":\"pull.gdfl.format\"")
F crates/pull/tests/gdfl_bad_row_audit.rs:346 [G3 PS3]: || lines[0].contains("pull.gdfl.format")
F crates/pull/tests/gdfl_bad_row_audit.rs:346:16 False [G3 PS3]: || lines[0].contains("pull.gdfl.format")
F crates/pull/tests/gdfl_bad_row_audit.rs:346:16 True [G3 PS3]: || lines[0].contains("pull.gdfl.format")
F crates/pull/tests/gdfl_census_stop.rs:116:9 False [G3 PS3]: let ImportRefusal::CensusRefused { day: at, done, .. } = &got else {
F crates/pull/tests/gdfl_census_stop.rs:117 [G3 PS3]: panic!("{got:?}");
F crates/pull/tests/gdfl_census_stop.rs:125:30 False [G3 PS3]: .is_some_and(|l| l.starts_with("incomplete stocks 2024-04-01 * ")
F crates/pull/tests/gdfl_census_stop.rs:175:9 False [G3 PS3]: stop[0].contains("\"error\"") && stop[0].contains("\"2024-04-01\""),
F crates/pull/tests/gdfl_census_stop.rs:175:42 False [G3 PS3]: stop[0].contains("\"error\"") && stop[0].contains("\"2024-04-01\""),
F crates/pull/tests/gdfl_census_stop.rs:177 [G3 PS3]: stop[0]
F crates/pull/tests/gdfl_census_stop.rs:182 [G3 PS3]: stop[0]
F crates/pull/tests/gdfl_census_stop.rs:212:9 False [G3 PS3]: imported[0].contains("\"done_with_refusals\"") && imported[0].contains("\"info\""),
F crates/pull/tests/gdfl_census_stop.rs:212:59 False [G3 PS3]: imported[0].contains("\"done_with_refusals\"") && imported[0].contains("\"info\""),
F crates/pull/tests/gdfl_census_stop.rs:214 [G3 PS3]: imported[0]
F crates/pull/tests/gdfl_census_stop.rs:219:9 False [G3 PS3]: refused[0].contains("\"warn\"") && refused[0].contains("\"refused\""),
F crates/pull/tests/gdfl_census_stop.rs:219:44 False [G3 PS3]: refused[0].contains("\"warn\"") && refused[0].contains("\"refused\""),
F crates/pull/tests/gdfl_census_stop.rs:221 [G3 PS3]: refused[0]
F crates/pull/tests/integration.rs:326 [G3 PS3]: at.day()
F crates/pull/tests/padded_refold.rs:290 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:350 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:381 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:400 [G2 PS2]: journal[0]
F crates/pull/tests/padded_refold.rs:405 [G2 PS2]: journal[0]
F crates/pull/tests/padded_refold.rs:446 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:459 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:464 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:474 [G2 PS2]: journal[0]
F crates/pull/tests/padded_refold.rs:516 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:522 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:578 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:598 [G2 PS2]: refused[0]
F crates/pull/tests/padded_refold.rs:620 [G2 PS2]: refused[0]
F crates/pull/tests/pipeline.rs:185:9 False [G3 PS3]: capped.contains('7') && capped.contains(&MAX_MEMBERS.to_string()),
F crates/pull/tests/pipeline.rs:185:33 False [G3 PS3]: capped.contains('7') && capped.contains(&MAX_MEMBERS.to_string()),
F crates/pull/tests/pipeline.rs:192 [G3 PS3]: cases[4]
F crates/pull/tests/pipeline.rs:372 [G3 PS3]: Ok((members, rejected)) => panic!(
F crates/pull/tests/pipeline.rs:375 [G3 PS3]: members.len(),
F crates/pull/tests/pipeline.rs:376 [G3 PS3]: rejected.len(),
F crates/pull/tests/pipeline.rs:377 [G3 PS3]: members.len() + rejected.len()
F crates/pull/tests/pipeline.rs:437 [G3 PS3]: wide.why.len(),
F crates/pull/tests/pipeline.rs:438 [G3 PS3]: wide.why.get(wide.why.len().saturating_sub(48)..)
F crates/pull/tests/pipeline.rs:443 [G3 PS3]: wide.why.capacity()
F crates/pull/tests/pipeline.rs:517 [G3 PS3]: cases[4]
F crates/pull/tests/pipeline.rs:726 [G3 PS3]: cases[4]
F crates/pull/tests/pipeline.rs:944:9 False [G3 PS3]: group.why.contains("row 1") && group.why.contains("before the Unix epoch"),
F crates/pull/tests/pipeline.rs:944:40 False [G3 PS3]: group.why.contains("row 1") && group.why.contains("before the Unix epoch"),
F crates/pull/tests/pipeline.rs:1028:9 False [G3 PS3]: text.contains(&120_000_000.to_string())
F crates/pull/tests/pipeline.rs:1029:16 False [G3 PS3]: && text.contains(&60_000_000.to_string())
F crates/pull/tests/pipeline.rs:1030:16 False [G3 PS3]: && text.contains('1'),
F crates/pull/tests/pipeline.rs:1487:16 False [G3 PS3]: if path.ends_with("ZZZZ.csv")),
F crates/pull/tests/pipeline.rs:1568 [G3 PS3]: done.failures[0].why
F crates/pull/tests/pipeline.rs:1594 [G3 PS3]: done.failures[0].why
F crates/pull/tests/pipeline.rs:1701 [G3 PS3]: done.failures[0].why
F crates/pull/tests/refusal.rs:554 [G1 PS1]: feed.display()
F crates/pull/tests/refusal.rs:588 [G1 PS1]: feed.display()
F crates/pull/tests/refusal.rs:645 [G1 PS1]: other => panic!(
F crates/pull/tests/refusal.rs:650 [G1 PS1]: other.master_columns().isin
F crates/pull/tests/refusal.rs:658 [G1 PS1]: vendor.master_columns().isin
F crates/pull/tests/refusal.rs:691:9 False [G1 PS1]: let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
F crates/pull/tests/refusal.rs:693 [G1 PS1]: panic!("Zerodha is an HTTP feed");
F crates/pull/tests/request_gap_events.rs:138:9 False [G1 PS1]: lines[0].contains("\"count\":2")
F crates/pull/tests/request_gap_events.rs:139:16 False [G1 PS1]: && lines[0].contains("\"first_day\":\"2025-07-01\"")
F crates/pull/tests/request_gap_events.rs:140:16 False [G1 PS1]: && lines[0].contains("\"last_day\":\"2025-07-01\"")
F crates/pull/tests/request_gap_events.rs:141:16 False [G1 PS1]: && lines[0].contains("\"month\":\"2025-07\"")
F crates/pull/tests/request_gap_events.rs:142:16 False [G1 PS1]: && lines[0].contains("\"code\":\"vendor_empty\"")
F crates/pull/tests/request_gap_events.rs:143:16 False [G1 PS1]: && lines[0].contains("\"feed\":\"truedata\""),
F crates/pull/tests/request_gap_events.rs:145 [G1 PS1]: lines[0]
F crates/pull/tests/sealed_month.rs:437:9 False [G1 PS1]: words.contains("repair revision 3") && words.contains("restates"),
F crates/pull/tests/sealed_month.rs:437:48 False [G1 PS1]: words.contains("repair revision 3") && words.contains("restates"),
F crates/pull/tests/sealed_month.rs:497:9 False [G1 PS1]: words.contains("sealed") && words.contains("restates"),
F crates/pull/tests/sealed_month.rs:497:37 False [G1 PS1]: words.contains("sealed") && words.contains("restates"),
IGNORED crates/pull/tests/store_evidence.rs:30 [G2 PS2]: fn store_root() -> PathBuf {
IGNORED crates/pull/tests/store_evidence.rs:31 [G2 PS2]: PathBuf::from(
IGNORED crates/pull/tests/store_evidence.rs:32 [G2 PS2]: std::env::var_os("BRUTEX_EVIDENCE_STORE")
IGNORED crates/pull/tests/store_evidence.rs:33 [G2 PS2]: .expect("BRUTEX_EVIDENCE_STORE names the store root to read"),
IGNORED crates/pull/tests/store_evidence.rs:35 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:38 [G2 PS2]: fn read_month(
IGNORED crates/pull/tests/store_evidence.rs:39 [G2 PS2]: root: &Path,
IGNORED crates/pull/tests/store_evidence.rs:40 [G2 PS2]: segment: &str,
IGNORED crates/pull/tests/store_evidence.rs:41 [G2 PS2]: symbol: &str,
IGNORED crates/pull/tests/store_evidence.rs:42 [G2 PS2]: tf: Timeframe,
IGNORED crates/pull/tests/store_evidence.rs:43 [G2 PS2]: month: YearMonth,
IGNORED crates/pull/tests/store_evidence.rs:44 [G2 PS2]: ) -> Vec<store::format::Bar> {
IGNORED crates/pull/tests/store_evidence.rs:45 [G2 PS2]: let path = StorePath::new(PathParts {
IGNORED crates/pull/tests/store_evidence.rs:46 [G2 PS2]: vendor: Vendor::Zerodha,
IGNORED crates/pull/tests/store_evidence.rs:47 [G2 PS2]: exchange: "NSE",
IGNORED crates/pull/tests/store_evidence.rs:48 [G2 PS2]: segment,
IGNORED crates/pull/tests/store_evidence.rs:49 [G2 PS2]: symbol,
IGNORED crates/pull/tests/store_evidence.rs:50 [G2 PS2]: contract: None,
IGNORED crates/pull/tests/store_evidence.rs:51 [G2 PS2]: timeframe: tf,
IGNORED crates/pull/tests/store_evidence.rs:52 [G2 PS2]: month,
IGNORED crates/pull/tests/store_evidence.rs:53 [G2 PS2]: file: FileKind::Bars,
IGNORED crates/pull/tests/store_evidence.rs:54 [G2 PS2]: })
IGNORED crates/pull/tests/store_evidence.rs:55 [G2 PS2]: .unwrap();
IGNORED crates/pull/tests/store_evidence.rs:56 [G2 PS2]: let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:57 [G2 PS2]: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:57:9 False [G2 PS2]: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:57:9 True [G2 PS2]: let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
IGNORED crates/pull/tests/store_evidence.rs:58 [G2 PS2]: return Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:60 [G2 PS2]: (0..file.header().n_valid)
IGNORED crates/pull/tests/store_evidence.rs:61 [G2 PS2]: .map(|i| file.read_record(i).unwrap())
IGNORED crates/pull/tests/store_evidence.rs:62 [G2 PS2]: .collect()
IGNORED crates/pull/tests/store_evidence.rs:63 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:66 [G2 PS2]: fn on_day(day: Day, bars: &[store::format::Bar]) -> usize {
IGNORED crates/pull/tests/store_evidence.rs:67 [G2 PS2]: let midnight = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
IGNORED crates/pull/tests/store_evidence.rs:68 [G2 PS2]: bars.iter()
IGNORED crates/pull/tests/store_evidence.rs:69 [G2 PS2]: .filter(|b| (midnight..midnight + 86_400_000_000).contains(&b.ts_micros))
IGNORED crates/pull/tests/store_evidence.rs:70 [G2 PS2]: .count()
IGNORED crates/pull/tests/store_evidence.rs:71 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:86 [G2 PS2]: fn replay(root: &Path, segment: &str, symbol: &str, month: YearMonth) -> Replay {
IGNORED crates/pull/tests/store_evidence.rs:87 [G2 PS2]: let minutes = read_month(root, segment, symbol, Timeframe::MINUTE_1, month);
IGNORED crates/pull/tests/store_evidence.rs:88 [G2 PS2]: assert!(
IGNORED crates/pull/tests/store_evidence.rs:89 [G2 PS2]: !minutes.is_empty(),
IGNORED crates/pull/tests/store_evidence.rs:92 [G2 PS2]: let rows: Vec<pull::fetch::RawRow> = minutes
IGNORED crates/pull/tests/store_evidence.rs:93 [G2 PS2]: .iter()
IGNORED crates/pull/tests/store_evidence.rs:94 [G2 PS2]: .map(|b| pull::fetch::RawRow {
IGNORED crates/pull/tests/store_evidence.rs:95 [G2 PS2]: timestamp: b.ts_micros.div_euclid(1_000_000),
IGNORED crates/pull/tests/store_evidence.rs:96 [G2 PS2]: open: b.open,
IGNORED crates/pull/tests/store_evidence.rs:97 [G2 PS2]: high: b.high,
IGNORED crates/pull/tests/store_evidence.rs:98 [G2 PS2]: low: b.low,
IGNORED crates/pull/tests/store_evidence.rs:99 [G2 PS2]: close: b.close,
IGNORED crates/pull/tests/store_evidence.rs:100 [G2 PS2]: volume: b.volume,
IGNORED crates/pull/tests/store_evidence.rs:101 [G2 PS2]: open_interest: None,
IGNORED crates/pull/tests/store_evidence.rs:102 [G2 PS2]: })
IGNORED crates/pull/tests/store_evidence.rs:103 [G2 PS2]: .collect();
IGNORED crates/pull/tests/store_evidence.rs:104 [G2 PS2]: let first = Day::new(month.year(), month.month(), 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:105 [G2 PS2]: let request = pull::fetch::BarRequest {
IGNORED crates/pull/tests/store_evidence.rs:106 [G2 PS2]: instrument_id: String::new(),
IGNORED crates/pull/tests/store_evidence.rs:107 [G2 PS2]: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:107:21 False [G2 PS2]: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:107:21 True [G2 PS2]: listing: if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:108 [G2 PS2]: pull::vendor::Listing::Index
IGNORED crates/pull/tests/store_evidence.rs:110 [G2 PS2]: pull::vendor::Listing::Equity
IGNORED crates/pull/tests/store_evidence.rs:112 [G2 PS2]: window: Window::new(first, first.end_of_month()).unwrap(),
IGNORED crates/pull/tests/store_evidence.rs:113 [G2 PS2]: granularity: pull::vendor::Granularity::Minute1,
IGNORED crates/pull/tests/store_evidence.rs:115 [G2 PS2]: let plan = pull::ingest::Plan {
IGNORED crates/pull/tests/store_evidence.rs:116 [G2 PS2]: calendar: pull::calendar::Runtime::default(),
IGNORED crates/pull/tests/store_evidence.rs:117 [G2 PS2]: cash_schedule: None,
IGNORED crates/pull/tests/store_evidence.rs:118 [G2 PS2]: columns: pull::csv::Columns::Gdfl,
IGNORED crates/pull/tests/store_evidence.rs:119 [G2 PS2]: request: &request,
IGNORED crates/pull/tests/store_evidence.rs:120 [G2 PS2]: encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
IGNORED crates/pull/tests/store_evidence.rs:121 [G2 PS2]: scale: pull::vendor::PriceScale::Paisa,
IGNORED crates/pull/tests/store_evidence.rs:122 [G2 PS2]: vendor: Vendor::Zerodha,
IGNORED crates/pull/tests/store_evidence.rs:123 [G2 PS2]: exchange: "NSE",
IGNORED crates/pull/tests/store_evidence.rs:124 [G2 PS2]: segment,
IGNORED crates/pull/tests/store_evidence.rs:125 [G2 PS2]: contract: None,
IGNORED crates/pull/tests/store_evidence.rs:126 [G2 PS2]: run: 0,
IGNORED crates/pull/tests/store_evidence.rs:127 [G2 PS2]: };
IGNORED crates/pull/tests/store_evidence.rs:128 [G2 PS2]: let scratch = std::env::temp_dir().join(format!(
IGNORED crates/pull/tests/store_evidence.rs:129 [G2 PS2]: "brutex-EVIDENCE-{symbol}-{month}-{}",
IGNORED crates/pull/tests/store_evidence.rs:130 [G2 PS2]: std::process::id()
IGNORED crates/pull/tests/store_evidence.rs:131 [G2 PS2]: ));
IGNORED crates/pull/tests/store_evidence.rs:132 [G2 PS2]: let _fresh = fs::remove_dir_all(&scratch);
IGNORED crates/pull/tests/store_evidence.rs:133 [G2 PS2]: fs::create_dir_all(&scratch).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:134 [G2 PS2]: let done = pull::ingest::from_window(
IGNORED crates/pull/tests/store_evidence.rs:135 [G2 PS2]: &pull::fetch::RawWindow { rows },
IGNORED crates/pull/tests/store_evidence.rs:136 [G2 PS2]: symbol,
IGNORED crates/pull/tests/store_evidence.rs:137 [G2 PS2]: "STORE EVIDENCE",
IGNORED crates/pull/tests/store_evidence.rs:138 [G2 PS2]: &scratch,
IGNORED crates/pull/tests/store_evidence.rs:139 [G2 PS2]: plan,
IGNORED crates/pull/tests/store_evidence.rs:141 [G2 PS2]: let ledger = gap_ledger(&scratch);
IGNORED crates/pull/tests/store_evidence.rs:142 [G2 PS2]: Replay {
IGNORED crates/pull/tests/store_evidence.rs:143 [G2 PS2]: scratch,
IGNORED crates/pull/tests/store_evidence.rs:144 [G2 PS2]: done,
IGNORED crates/pull/tests/store_evidence.rs:145 [G2 PS2]: ledger,
IGNORED crates/pull/tests/store_evidence.rs:146 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:147 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:223 [G2 PS2]: fn owed_days(month: YearMonth, minutes: &[store::format::Bar]) -> Vec<(Day, Owed)> {
IGNORED crates/pull/tests/store_evidence.rs:224 [G2 PS2]: let first = Day::new(month.year(), month.month(), 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:225 [G2 PS2]: let calendar = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:226 [G2 PS2]: let mut out = Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:227 [G2 PS2]: let mut day = first;
IGNORED crates/pull/tests/store_evidence.rs:228 [G2 PS2]: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:228:11 False [G2 PS2]: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:228:11 True [G2 PS2]: while day.month() == first.month() {
IGNORED crates/pull/tests/store_evidence.rs:229 [G2 PS2]: let at = i64::from(day.days_from_epoch());
IGNORED crates/pull/tests/store_evidence.rs:230 [G2 PS2]: let stored = on_day(day, minutes);
IGNORED crates/pull/tests/store_evidence.rs:231 [G2 PS2]: match pull::session::with_recorded_window(at, calendar.kind_of(at)) {
IGNORED crates/pull/tests/store_evidence.rs:232 [G2 PS2]: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:232:55 False [G2 PS2]: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:232:55 True [G2 PS2]: pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
IGNORED crates/pull/tests/store_evidence.rs:233 [G2 PS2]: out.push((
IGNORED crates/pull/tests/store_evidence.rs:234 [G2 PS2]: day,
IGNORED crates/pull/tests/store_evidence.rs:235 [G2 PS2]: Owed::Short {
IGNORED crates/pull/tests/store_evidence.rs:236 [G2 PS2]: owed: session.bars(),
IGNORED crates/pull/tests/store_evidence.rs:237 [G2 PS2]: stored,
IGNORED crates/pull/tests/store_evidence.rs:238 [G2 PS2]: },
IGNORED crates/pull/tests/store_evidence.rs:239 [G2 PS2]: ));
IGNORED crates/pull/tests/store_evidence.rs:240 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:241 [G2 PS2]: pull::calendar::DayKind::OpenLengthUnmeasured => {
IGNORED crates/pull/tests/store_evidence.rs:242 [G2 PS2]: out.push((day, Owed::Unsized { stored }));
IGNORED crates/pull/tests/store_evidence.rs:243 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:246 [G2 PS2]: | pull::calendar::DayKind::Unmeasured => {}
IGNORED crates/pull/tests/store_evidence.rs:248 [G2 PS2]: day = day.succ().unwrap();
IGNORED crates/pull/tests/store_evidence.rs:250 [G2 PS2]: out
IGNORED crates/pull/tests/store_evidence.rs:251 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:254 [G2 PS2]: fn gap_lines<'a>(ledger: &'a str, key: &str, day: Day) -> Vec<&'a str> {
IGNORED crates/pull/tests/store_evidence.rs:255 [G2 PS2]: let prefix = format!("gap {key} {day} ");
IGNORED crates/pull/tests/store_evidence.rs:256 [G2 PS2]: ledger
IGNORED crates/pull/tests/store_evidence.rs:257 [G2 PS2]: .lines()
IGNORED crates/pull/tests/store_evidence.rs:258 [G2 PS2]: .filter(|line| line.starts_with(&prefix))
IGNORED crates/pull/tests/store_evidence.rs:259 [G2 PS2]: .collect()
IGNORED crates/pull/tests/store_evidence.rs:260 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:262 [G2 PS2]: fn derive_line<'a>(ledger: &'a str, rung: &str) -> &'a str {
IGNORED crates/pull/tests/store_evidence.rs:263 [G2 PS2]: ledger
IGNORED crates/pull/tests/store_evidence.rs:264 [G2 PS2]: .lines()
IGNORED crates/pull/tests/store_evidence.rs:265 [G2 PS2]: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:265:22 False [G2 PS2]: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:265:22 True [G2 PS2]: .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
IGNORED crates/pull/tests/store_evidence.rs:266 [G2 PS2]: .unwrap_or("(no derive line)")
IGNORED crates/pull/tests/store_evidence.rs:267 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:272 [G2 PS2]: fn banknifty_2024_03_02_derives_from_its_stored_minutes() {
IGNORED crates/pull/tests/store_evidence.rs:273 [G2 PS2]: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:274 [G2 PS2]: let month = YearMonth::new(2024, 3).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:275 [G2 PS2]: let saturday = Day::new(2024, 3, 2).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:276 [G2 PS2]: let stored_minutes = read_month(&root, "INDEX", "BANKNIFTY", Timeframe::MINUTE_1, month);
IGNORED crates/pull/tests/store_evidence.rs:277 [G2 PS2]: let replay = replay(&root, "INDEX", "BANKNIFTY", month);
IGNORED crates/pull/tests/store_evidence.rs:278 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:280 [G2 PS2]: on_day(saturday, &stored_minutes)
IGNORED crates/pull/tests/store_evidence.rs:282 [G2 PS2]: for tf in [
IGNORED crates/pull/tests/store_evidence.rs:283 [G2 PS2]: Timeframe::MINUTE_2,
IGNORED crates/pull/tests/store_evidence.rs:284 [G2 PS2]: Timeframe::MINUTE_5,
IGNORED crates/pull/tests/store_evidence.rs:285 [G2 PS2]: Timeframe::MINUTE_15,
IGNORED crates/pull/tests/store_evidence.rs:286 [G2 PS2]: Timeframe::MINUTE_60,
IGNORED crates/pull/tests/store_evidence.rs:287 [G2 PS2]: ] {
IGNORED crates/pull/tests/store_evidence.rs:288 [G2 PS2]: let stored = read_month(&root, "INDEX", "BANKNIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:289 [G2 PS2]: let derived = read_month(&replay.scratch, "INDEX", "BANKNIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:290 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:291 [G2 PS2]: "BANKNIFTY 2024-03 {}: stored {} bars ({} on 03-02); this build derives {} ({} on 03-02)",
IGNORED crates/pull/tests/store_evidence.rs:292 [G2 PS2]: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:293 [G2 PS2]: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:294 [G2 PS2]: on_day(saturday, &stored),
IGNORED crates/pull/tests/store_evidence.rs:295 [G2 PS2]: derived.len(),
IGNORED crates/pull/tests/store_evidence.rs:296 [G2 PS2]: on_day(saturday, &derived)
IGNORED crates/pull/tests/store_evidence.rs:297 [G2 PS2]: );
IGNORED crates/pull/tests/store_evidence.rs:298 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:299 [G2 PS2]: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:300 [G2 PS2]: let derived = read_month(
IGNORED crates/pull/tests/store_evidence.rs:301 [G2 PS2]: &replay.scratch,
IGNORED crates/pull/tests/store_evidence.rs:302 [G2 PS2]: "INDEX",
IGNORED crates/pull/tests/store_evidence.rs:303 [G2 PS2]: "BANKNIFTY",
IGNORED crates/pull/tests/store_evidence.rs:305 [G2 PS2]: month,
IGNORED crates/pull/tests/store_evidence.rs:307 [G2 PS2]: assert_eq!(on_day(saturday, &stored_minutes), 105);
IGNORED crates/pull/tests/store_evidence.rs:308 [G2 PS2]: assert_eq!(on_day(saturday, &derived), 21);
IGNORED crates/pull/tests/store_evidence.rs:309 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:315 [G2 PS2]: fn nifty_2026_09_derives_every_session() {
IGNORED crates/pull/tests/store_evidence.rs:316 [G2 PS2]: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:317 [G2 PS2]: let month = YearMonth::new(2026, 9).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:318 [G2 PS2]: let replay = replay(&root, "INDEX", "NIFTY", month);
IGNORED crates/pull/tests/store_evidence.rs:319 [G2 PS2]: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
IGNORED crates/pull/tests/store_evidence.rs:320 [G2 PS2]: let stored = read_month(&root, "INDEX", "NIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:321 [G2 PS2]: let derived = read_month(&replay.scratch, "INDEX", "NIFTY", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:322 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:324 [G2 PS2]: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:325 [G2 PS2]: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:326 [G2 PS2]: stored.last().map_or(0, |b| b.close),
IGNORED crates/pull/tests/store_evidence.rs:327 [G2 PS2]: derived.len(),
IGNORED crates/pull/tests/store_evidence.rs:328 [G2 PS2]: derived.last().map_or(0, |b| b.close)
IGNORED crates/pull/tests/store_evidence.rs:331 [G2 PS2]: eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:332 [G2 PS2]: eprintln!("failures: {}", replay.done.failures.len());
IGNORED crates/pull/tests/store_evidence.rs:333 [G2 PS2]: for failure in replay.done.failures.iter().take(5) {
IGNORED crates/pull/tests/store_evidence.rs:334 [G2 PS2]: eprintln!("  {}", failure.why.chars().take(240).collect::<String>());
IGNORED crates/pull/tests/store_evidence.rs:335 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:336 [G2 PS2]: assert!(derive_line(&replay.ledger, "5min").contains(" days=21 "));
IGNORED crates/pull/tests/store_evidence.rs:337 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:349 [G2 PS2]: fn the_gap_ledger_names_each_short_day_of_a_stored_month() {
IGNORED crates/pull/tests/store_evidence.rs:350 [G2 PS2]: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:351 [G2 PS2]: let calendar = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:352 [G2 PS2]: let mut checked = 0_usize;
IGNORED crates/pull/tests/store_evidence.rs:353 [G2 PS2]: let mut special = Vec::new();
IGNORED crates/pull/tests/store_evidence.rs:354 [G2 PS2]: for (symbol, year, month) in [("NIFTY", 2020, 11), ("NIFTY", 2024, 11), ("NIFTY", 2021, 2)] {
IGNORED crates/pull/tests/store_evidence.rs:355 [G2 PS2]: let ym = YearMonth::new(year, month).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:356 [G2 PS2]: let minutes = read_month(&root, "INDEX", symbol, Timeframe::MINUTE_1, ym);
IGNORED crates/pull/tests/store_evidence.rs:357 [G2 PS2]: let replay = replay(&root, "INDEX", symbol, ym);
IGNORED crates/pull/tests/store_evidence.rs:358 [G2 PS2]: let key = format!("NSE-INDEX-{symbol}");
IGNORED crates/pull/tests/store_evidence.rs:359 [G2 PS2]: let owed = owed_days(ym, &minutes);
IGNORED crates/pull/tests/store_evidence.rs:360 [G2 PS2]: let gaps = replay
IGNORED crates/pull/tests/store_evidence.rs:361 [G2 PS2]: .ledger
IGNORED crates/pull/tests/store_evidence.rs:362 [G2 PS2]: .lines()
IGNORED crates/pull/tests/store_evidence.rs:363 [G2 PS2]: .filter(|line| line.starts_with(&format!("gap {key} ")))
IGNORED crates/pull/tests/store_evidence.rs:364 [G2 PS2]: .count();
IGNORED crates/pull/tests/store_evidence.rs:365 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:367 [G2 PS2]: owed.len(),
IGNORED crates/pull/tests/store_evidence.rs:368 [G2 PS2]: replay.done.failures.len()
IGNORED crates/pull/tests/store_evidence.rs:370 [G2 PS2]: for (day, what) in &owed {
IGNORED crates/pull/tests/store_evidence.rs:371 [G2 PS2]: let named = gap_lines(&replay.ledger, &key, *day);
IGNORED crates/pull/tests/store_evidence.rs:372 [G2 PS2]: eprintln!("  {day} {what:?}: {named:?}");
IGNORED crates/pull/tests/store_evidence.rs:373 [G2 PS2]: assert_eq!(
IGNORED crates/pull/tests/store_evidence.rs:374 [G2 PS2]: named.len(),
IGNORED crates/pull/tests/store_evidence.rs:377 [G2 PS2]: named.len()
IGNORED crates/pull/tests/store_evidence.rs:379 [G2 PS2]: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:379:20 False [G2 PS2]: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:379:20 True [G2 PS2]: if let Owed::Unsized { .. } = what {
IGNORED crates/pull/tests/store_evidence.rs:380 [G2 PS2]: assert!(
IGNORED crates/pull/tests/store_evidence.rs:381 [G2 PS2]: named[0].contains(" why=withheld_unmeasured_session "),
IGNORED crates/pull/tests/store_evidence.rs:383 [G2 PS2]: named[0]
IGNORED crates/pull/tests/store_evidence.rs:385 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:386 [G2 PS2]: let at = i64::from(day.days_from_epoch());
IGNORED crates/pull/tests/store_evidence.rs:387 [G2 PS2]: let regular = matches!(
IGNORED crates/pull/tests/store_evidence.rs:388 [G2 PS2]: calendar.kind_of(at),
IGNORED crates/pull/tests/store_evidence.rs:389 [G2 PS2]: pull::calendar::DayKind::Open(session)
IGNORED crates/pull/tests/store_evidence.rs:390 [G2 PS2]: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:390:24 False [G2 PS2]: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:390:24 True [G2 PS2]: if session == pull::calendar::Session::full()
IGNORED crates/pull/tests/store_evidence.rs:392 [G2 PS2]: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:392:16 False [G2 PS2]: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:392:16 True [G2 PS2]: if !regular {
IGNORED crates/pull/tests/store_evidence.rs:393 [G2 PS2]: special.push(*day);
IGNORED crates/pull/tests/store_evidence.rs:394 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:395 [G2 PS2]: checked += 1;
IGNORED crates/pull/tests/store_evidence.rs:399 [G2 PS2]: let mut day = Day::new(year, month, 1).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:400 [G2 PS2]: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:400:15 False [G2 PS2]: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:400:15 True [G2 PS2]: while day.month() == month {
IGNORED crates/pull/tests/store_evidence.rs:401 [G2 PS2]: let named = gap_lines(&replay.ledger, &key, day);
IGNORED crates/pull/tests/store_evidence.rs:402 [G2 PS2]: let expected = usize::from(owed.iter().any(|(owed_day, _)| *owed_day == day));
IGNORED crates/pull/tests/store_evidence.rs:403 [G2 PS2]: assert_eq!(
IGNORED crates/pull/tests/store_evidence.rs:404 [G2 PS2]: named.len(),
IGNORED crates/pull/tests/store_evidence.rs:408 [G2 PS2]: named.len()
IGNORED crates/pull/tests/store_evidence.rs:410 [G2 PS2]: day = day.succ().unwrap();
IGNORED crates/pull/tests/store_evidence.rs:412 [G2 PS2]: eprintln!("  {}", derive_line(&replay.ledger, "5min"));
IGNORED crates/pull/tests/store_evidence.rs:417 [G2 PS2]: eprintln!("{checked} day(s) checked; special sessions among them: {special:?}");
IGNORED crates/pull/tests/store_evidence.rs:418 [G2 PS2]: assert!(checked > 0, "no owed day in three months chosen for theirs");
IGNORED crates/pull/tests/store_evidence.rs:419 [G2 PS2]: for year in [2020, 2024] {
IGNORED crates/pull/tests/store_evidence.rs:420 [G2 PS2]: assert!(
IGNORED crates/pull/tests/store_evidence.rs:421 [G2 PS2]: special
IGNORED crates/pull/tests/store_evidence.rs:422 [G2 PS2]: .iter()
IGNORED crates/pull/tests/store_evidence.rs:423 [G2 PS2]: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:423:28 False [G2 PS2]: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:423:28 True [G2 PS2]: .any(|day| day.year() == year && day.month() == 11),
IGNORED crates/pull/tests/store_evidence.rs:427 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:432 [G2 PS2]: fn an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets() {
IGNORED crates/pull/tests/store_evidence.rs:433 [G2 PS2]: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:434 [G2 PS2]: let month = YearMonth::new(2023, 2).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:435 [G2 PS2]: let replay = replay(&root, "CASH", "360ONE", month);
IGNORED crates/pull/tests/store_evidence.rs:436 [G2 PS2]: for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
IGNORED crates/pull/tests/store_evidence.rs:437 [G2 PS2]: let stored = read_month(&root, "CASH", "360ONE", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:438 [G2 PS2]: let derived = read_month(&replay.scratch, "CASH", "360ONE", tf, month);
IGNORED crates/pull/tests/store_evidence.rs:439 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:440 [G2 PS2]: "360ONE 2023-02 {}: stored {} bars; this build derives {}",
IGNORED crates/pull/tests/store_evidence.rs:441 [G2 PS2]: tf.as_str(),
IGNORED crates/pull/tests/store_evidence.rs:442 [G2 PS2]: stored.len(),
IGNORED crates/pull/tests/store_evidence.rs:443 [G2 PS2]: derived.len()
IGNORED crates/pull/tests/store_evidence.rs:444 [G2 PS2]: );
IGNORED crates/pull/tests/store_evidence.rs:445 [G2 PS2]: eprintln!("  {}", derive_line(&replay.ledger, tf.as_str()));
IGNORED crates/pull/tests/store_evidence.rs:446 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:447 [G2 PS2]: let gaps = replay
IGNORED crates/pull/tests/store_evidence.rs:448 [G2 PS2]: .ledger
IGNORED crates/pull/tests/store_evidence.rs:449 [G2 PS2]: .lines()
IGNORED crates/pull/tests/store_evidence.rs:450 [G2 PS2]: .filter(|l| l.starts_with("gap "))
IGNORED crates/pull/tests/store_evidence.rs:451 [G2 PS2]: .count();
IGNORED crates/pull/tests/store_evidence.rs:452 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:454 [G2 PS2]: replay.done.failures.len()
IGNORED crates/pull/tests/store_evidence.rs:456 [G2 PS2]: let stored = read_month(&root, "CASH", "360ONE", Timeframe::MINUTE_60, month);
IGNORED crates/pull/tests/store_evidence.rs:457 [G2 PS2]: let derived = read_month(
IGNORED crates/pull/tests/store_evidence.rs:458 [G2 PS2]: &replay.scratch,
IGNORED crates/pull/tests/store_evidence.rs:459 [G2 PS2]: "CASH",
IGNORED crates/pull/tests/store_evidence.rs:460 [G2 PS2]: "360ONE",
IGNORED crates/pull/tests/store_evidence.rs:462 [G2 PS2]: month,
IGNORED crates/pull/tests/store_evidence.rs:464 [G2 PS2]: assert!(derived.len() > stored.len());
IGNORED crates/pull/tests/store_evidence.rs:465 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:473 [G2 PS2]: fn the_derive_fold_costs_per_month() {
IGNORED crates/pull/tests/store_evidence.rs:474 [G2 PS2]: let root = store_root();
IGNORED crates/pull/tests/store_evidence.rs:475 [G2 PS2]: for (segment, symbol, year, month) in [("INDEX", "NIFTY", 2026, 9), ("CASH", "360ONE", 2023, 2)]
IGNORED crates/pull/tests/store_evidence.rs:477 [G2 PS2]: let ym = YearMonth::new(year, month).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:478 [G2 PS2]: let minutes = read_month(&root, segment, symbol, Timeframe::MINUTE_1, ym);
IGNORED crates/pull/tests/store_evidence.rs:479 [G2 PS2]: let venue = if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:479:24 False [G2 PS2]: let venue = if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:479:24 True [G2 PS2]: let venue = if segment == "INDEX" {
IGNORED crates/pull/tests/store_evidence.rs:480 [G2 PS2]: pull::vendor::Venue::NseIndex
IGNORED crates/pull/tests/store_evidence.rs:482 [G2 PS2]: pull::vendor::Venue::NseCash
IGNORED crates/pull/tests/store_evidence.rs:484 [G2 PS2]: let bucket = pull::fold::Bucket::of_secs(300).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:485 [G2 PS2]: let runtime = pull::calendar::Runtime::default();
IGNORED crates/pull/tests/store_evidence.rs:486 [G2 PS2]: let mut aggregated = Vec::with_capacity(200);
IGNORED crates/pull/tests/store_evidence.rs:487 [G2 PS2]: let mut detailed = Vec::with_capacity(200);
IGNORED crates/pull/tests/store_evidence.rs:488 [G2 PS2]: for _ in 0..200 {
IGNORED crates/pull/tests/store_evidence.rs:489 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/tests/store_evidence.rs:490 [G2 PS2]: let derived =
IGNORED crates/pull/tests/store_evidence.rs:491 [G2 PS2]: pull::fold::derive_minutes(&minutes, bucket, venue, None, runtime).unwrap();
IGNORED crates/pull/tests/store_evidence.rs:492 [G2 PS2]: aggregated.push(start.elapsed());
IGNORED crates/pull/tests/store_evidence.rs:493 [G2 PS2]: std::hint::black_box(derived);
IGNORED crates/pull/tests/store_evidence.rs:494 [G2 PS2]: let start = std::time::Instant::now();
IGNORED crates/pull/tests/store_evidence.rs:495 [G2 PS2]: let folded =
IGNORED crates/pull/tests/store_evidence.rs:496 [G2 PS2]: pull::fold::complete_minutes_with_calendar(&minutes, bucket, venue, None, runtime)
IGNORED crates/pull/tests/store_evidence.rs:497 [G2 PS2]: .unwrap();
IGNORED crates/pull/tests/store_evidence.rs:498 [G2 PS2]: detailed.push(start.elapsed());
IGNORED crates/pull/tests/store_evidence.rs:499 [G2 PS2]: std::hint::black_box(folded);
IGNORED crates/pull/tests/store_evidence.rs:500 [G2 PS2]: }
IGNORED crates/pull/tests/store_evidence.rs:501 [G2 PS2]: aggregated.sort_unstable();
IGNORED crates/pull/tests/store_evidence.rs:502 [G2 PS2]: detailed.sort_unstable();
IGNORED crates/pull/tests/store_evidence.rs:503 [G2 PS2]: eprintln!(
IGNORED crates/pull/tests/store_evidence.rs:505 [G2 PS2]: minutes.len(),
IGNORED crates/pull/tests/store_evidence.rs:506 [G2 PS2]: aggregated[99],
IGNORED crates/pull/tests/store_evidence.rs:507 [G2 PS2]: aggregated[197],
IGNORED crates/pull/tests/store_evidence.rs:508 [G2 PS2]: detailed[99],
IGNORED crates/pull/tests/store_evidence.rs:509 [G2 PS2]: detailed[197]
IGNORED crates/pull/tests/store_evidence.rs:511 [G2 PS2]: assert!(!minutes.is_empty());
IGNORED crates/pull/tests/store_evidence.rs:513 [G2 PS2]: }
F crates/pull/tests/support/mod.rs:62:15 False [G1 PS1]: let ran = output.status.success() && stdout.contains("1 passed");
F crates/pull/tests/token_identity.rs:374:15 True [G1 PS1]: !(TYPES.iter().any(|name| {
F crates/pull/tests/token_identity.rs:378:19 True [G1 PS1]: }) || FUNCTIONS.iter().any(|(_, name)| {
F crates/pull/tests/unit.rs:126:9 False [G1 PS1]: first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
F crates/pull/tests/unit.rs:126:51 False [G1 PS1]: first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
F crates/pull/tests/unit.rs:143 [G1 PS1]: other => panic!("expected Unreadable, got {other:?}"),
F crates/pull/tests/unit.rs:766 [G1 PS1]: other => panic!("expected an unreadable file, got {other:?}"),
F crates/pull/tests/unit.rs:801:13 False [G1 PS1]: assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
F crates/pull/tests/unit.rs:801:49 False [G1 PS1]: assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
F crates/pull/tests/unit.rs:1016:13 False [G1 PS1]: assert!(rendered.contains("groww") && rendered.contains("fieldone"));
F crates/pull/tests/unit.rs:1016:43 False [G1 PS1]: assert!(rendered.contains("groww") && rendered.contains("fieldone"));
F crates/pull/tests/unit.rs:1017:13 False [G1 PS1]: assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
F crates/pull/tests/unit.rs:1017:45 False [G1 PS1]: assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
F crates/pull/tests/unit.rs:1303 [G1 PS1]: other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
F crates/pull/tests/unit.rs:1324 [G1 PS1]: other => panic!("expected a checksum refusal, got {other:?}"),
F crates/pull/tests/unit.rs:1627:40 False [G1 PS1]: manifest.entries() == 0 || manifest.entries() == 1,
F crates/pull/tests/unit.rs:1629 [G1 PS1]: manifest.entries()
F crates/pull/tests/unit.rs:1691 [G1 PS1]: other => panic!("the recovery must name what it stepped over, got {other:?}"),
F crates/pull/tests/unit.rs:1786 [G1 PS1]: other => panic!("expected the slot checksum to be reported, got {other:?}"),
F crates/pull/tests/unit.rs:1869 [G1 PS1]: manifest.reserved()
F crates/pull/tests/unit.rs:1874 [G1 PS1]: manifest.reserved()
F crates/pull/tests/unit.rs:2011 [G1 PS1]: manifest.reserved()
F crates/pull/tests/unit.rs:2069 [G1 PS1]: other => panic!("expected entry 1 to be named, got {other:?}"),
F crates/pull/tests/unit.rs:2209 [G1 PS1]: other => panic!("expected a checksum refusal, got {other:?}"),
F crates/pull/tests/unit.rs:2935 [G1 PS1]: Verdict::Admit => panic!("a drained bucket admits nothing at once"),
F crates/pull/tests/unit.rs:3052 [G1 PS1]: governor.permitted(WindowSpan::Day)
F crates/pull/tests/unit.rs:3383:9 False [G1 PS1]: let Verdict::Deny { span, wait_micros } = governor.admit(0) else {
F crates/pull/tests/unit.rs:3384 [G1 PS1]: panic!("the eighth request must be refused");
F crates/pull/tests/unit.rs:4125 [G1 PS1]: other => panic!("month {other}"),
F crates/pull/tests/unit.rs:4891 [G1 PS1]: Some(other) => panic!("a bar inside its own window was dropped as {other}"),
F crates/pull/tests/unit.rs:5203:13 False [G1 PS1]: assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
F crates/pull/tests/unit.rs:5203:46 False [G1 PS1]: assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
F crates/pull/tests/unit.rs:5204:13 False [G1 PS1]: assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
F crates/pull/tests/unit.rs:5204:48 False [G1 PS1]: assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
F crates/pull/tests/unit.rs:5359 [G1 PS1]: errors[0]
F crates/pull/tests/unit.rs:5366:13 False [G1 PS1]: assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
F crates/pull/tests/unit.rs:5366:53 False [G1 PS1]: assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
F crates/pull/tests/unit.rs:5410:13 False [G1 PS1]: assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
F crates/pull/tests/unit.rs:5410:48 False [G1 PS1]: assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
F crates/pull/tests/unit.rs:5411:13 False [G1 PS1]: assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
F crates/pull/tests/unit.rs:5411:49 False [G1 PS1]: assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
F crates/pull/tests/unit.rs:5425:13 False [G1 PS1]: assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
F crates/pull/tests/unit.rs:5425:46 False [G1 PS1]: assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
F crates/pull/tests/unit.rs:5426:13 False [G1 PS1]: assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
F crates/pull/tests/unit.rs:5426:47 False [G1 PS1]: assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
F crates/pull/tests/unit.rs:5556:9 False [G1 PS1]: text.contains("volume 2") && text.contains("open 3"),
F crates/pull/tests/unit.rs:5556:38 False [G1 PS1]: text.contains("volume 2") && text.contains("open 3"),
F crates/pull/tests/unit.rs:5736:9 False [G1 PS1]: group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),
F crates/pull/tests/unit.rs:5736:46 False [G1 PS1]: group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),
F crates/pull/tests/unit.rs:6794 [G1 PS1]: other => panic!("prefix of {cut} bytes was not refused by name: {other:?}"),
F crates/pull/tests/unit.rs:6814 [G1 PS1]: other => panic!("a corrupt header slot must be refused by name, got {other:?}"),
F crates/pull/tests/unit.rs:7881 [G1 PS1]: other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
F crates/pull/tests/unit.rs:7952 [G1 PS1]: layout.version()
F crates/pull/tests/zerodha_gaps.rs:225 [G2 PS2]: gaps[0].why
F crates/pull/tests/zerodha_gaps.rs:235 [G2 PS2]: lines[0]
F crates/pull/tests/zerodha_gaps.rs:543 [G2 PS2]: other => panic!("2021-02-24 is a session of record: {other:?}"),
F crates/pull/tests/zerodha_gaps.rs:599:17 False [G2 PS2]: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
F crates/pull/tests/zerodha_gaps.rs:599:44 False [G2 PS2]: assert!(short.owed > short.held && short.missing() == short.owed - short.held);
F crates/pull/tests/zerodha_gaps.rs:605:13 False [G2 PS2]: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:605:46 False [G2 PS2]: assert!(!Missing::Tail.needs_repair() && !Missing::BeforeFirst.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:610:13 False [G2 PS2]: assert!(absent.short.is_empty() && !absent.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:610:40 False [G2 PS2]: assert!(absent.short.is_empty() && !absent.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:630:22 False [G2 PS2]: .all(|m| m.short.is_empty() && m.length_unjudged.is_empty())
F crates/pull/tests/zerodha_gaps.rs:650:9 False [G2 PS2]: let Owes::Windows((windows, 1)) = repair_plan::owed("CASH", from.days_from_epoch()) else {
F crates/pull/tests/zerodha_gaps.rs:651 [G2 PS2]: panic!("a regular cash session owes one window");
F crates/pull/tests/zerodha_gaps.rs:712:9 False [G2 PS2]: series.lines[0].starts_with("gap NSE-CASH-INVENTEDSHR 2026-08-06 v=1 rung=1min owed=360 held=359 missing=1 first_missing=11:00 ")
F crates/pull/tests/zerodha_gaps.rs:713:16 False [G2 PS2]: && series.lines[0].ends_with(" fill=interior source=store absence=may_be_no_trade"),
F crates/pull/tests/zerodha_gaps.rs:715 [G2 PS2]: series.lines[0]
F crates/pull/tests/zerodha_gaps.rs:824 [G2 PS2]: other => panic!("{other:?}"),
F crates/pull/tests/zerodha_gaps.rs:833:13 False [G2 PS2]: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
F crates/pull/tests/zerodha_gaps.rs:833:52 False [G2 PS2]: line.contains(" why=stored_short ") && line.ends_with(" source=store"),
F crates/pull/tests/zerodha_gaps.rs:876:9 False [G2 PS2]: second.lines.is_empty() && second.tally.already == 5,
F crates/pull/tests/zerodha_gaps.rs:876:36 False [G2 PS2]: second.lines.is_empty() && second.tally.already == 5,
F crates/pull/tests/zerodha_gaps.rs:958:9 False [G2 PS2]: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
F crates/pull/tests/zerodha_gaps.rs:958:62 False [G2 PS2]: named.contains(" missing=1 first_missing=15:20 ") && named.contains(" why=refused_"),
F crates/pull/tests/zerodha_gaps.rs:991:13 False [G2 PS2]: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:991:37 False [G2 PS2]: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:991:63 False [G2 PS2]: assert!(month.generation > 0 && month.short.is_empty() && !month.needs_repair());
F crates/pull/tests/zerodha_gaps.rs:1144:13 False [G2 PS2]: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
F crates/pull/tests/zerodha_gaps.rs:1144:32 False [G2 PS2]: assert!(have.is_empty() && have.faults.is_empty(), "no ledger yet");
F crates/pull/tests/zerodha_gaps.rs:1348:22 False [G2 PS2]: .all(|l| l.contains(" why=stored_short ") && l.ends_with(" source=store")),
F crates/pull/tests/zerodha_identity.rs:288:13 False [G2 PS2]: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
F crates/pull/tests/zerodha_identity.rs:288:37 False [G2 PS2]: top <= LEVEL + 2_000 && bottom >= LEVEL - 1_000,
F crates/pull/tests/zerodha_identity.rs:302:30 False [G2 PS2]: .all(|b| b.high <= LEVEL + 2_000 && b.low >= LEVEL - 1_000),
F crates/pull/tests/zerodha_identity.rs:304 [G2 PS2]: rung.as_str()
F crates/pull/tests/zerodha_identity.rs:560 [G2 PS2]: Err(why) => panic!("{symbol} {}: {why}", tf.as_str()),
F crates/pull/tests/zerodha_identity.rs:717 [G2 PS2]: rung.as_str()
F crates/pull/tests/zerodha_identity.rs:836 [G2 PS2]: named[0]
F crates/pull/tests/zerodha_identity.rs:1495:22 False [G2 PS2]: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
F crates/pull/tests/zerodha_identity.rs:1495:36 False [G2 PS2]: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
F crates/pull/tests/zerodha_identity.rs:1495:50 False [G2 PS2]: .all(|b| b.open > 0 && b.high > 0 && b.low > 0 && b.close > 0),
F crates/pull/tests/zerodha_identity.rs:1755 [G2 PS2]: lines[0]
F crates/pull/tests/zerodha_identity.rs:1764 [G2 PS2]: lines[0]
F crates/pull/tests/zerodha_identity.rs:1769 [G2 PS2]: lines[0]
F crates/pull/tests/zerodha_identity.rs:1810 [G2 PS2]: identity_lines(sink, "PATANJALI")
F crates/pull/tests/zerodha_identity.rs:1862:22 False [G2 PS2]: .any(|l| l.contains("the day rung would not open")
F crates/pull/tests/zerodha_identity.rs:1863:20 False [G2 PS2]: && l.contains("\"why\":")
F crates/pull/tests/zerodha_identity.rs:1957 [G2 PS2]: named[0]
F crates/pull/tests/zerodha_identity.rs:1966 [G2 PS2]: said[0]
F crates/pull/tests/zerodha_identity.rs:1971 [G2 PS2]: said[0]
F crates/pull/tests/zerodha_identity.rs:1976 [G2 PS2]: said[0]
F crates/pull/tests/zerodha_repair.rs:287 [G2 PS2]: lines[0]
F crates/pull/tests/zerodha_repair.rs:340:22 False [G2 PS2]: .any(|f| f.why.contains("2020-12:") && f.why.contains("this month only")),
F crates/pull/tests/zerodha_repair.rs:355 [G2 PS2]: front[0]
F crates/pull/tests/zerodha_repair.rs:405 [G2 PS2]: lines[11]
`````

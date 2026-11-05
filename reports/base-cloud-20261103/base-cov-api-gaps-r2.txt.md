# base-cov-api-gaps-r2.txt.md

Source: `state/fix/round2/base-cov-api-gaps-r2.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
BASE-COV-API repair 2, Route B on tip 8091e72f7d857df474de2224a382b1e41c4c5ce9, whole:api, log /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-API-logs/8091e72f7d85-routeb-r2.log
files read 182; NOT PROVED (no object holds them) 23: crates/api/benches/ratio.rs crates/api/benches/ratio/basis.rs crates/api/benches/ratio/calendar.rs crates/api/benches/ratio/census.rs crates/api/benches/ratio/chain.rs crates/api/benches/ratio/dayflags.rs crates/api/benches/ratio/db_page.rs crates/api/benches/ratio/export.rs crates/api/benches/ratio/feed_checks.rs crates/api/benches/ratio/gaps.rs crates/api/benches/ratio/greeks.rs crates/api/benches/ratio/logs_dirs.rs crates/api/benches/ratio/months.rs crates/api/benches/ratio/options.rs crates/api/benches/ratio/provenance.rs crates/api/benches/ratio/reads.rs crates/api/benches/ratio/rollup.rs crates/api/benches/ratio/served.rs crates/api/benches/ratio/settings.rs crates/api/benches/ratio/sources.rs crates/api/benches/ratio/spot.rs crates/api/benches/ratio/window.rs crates/api/src/lib.rs
TOTAL lines 171626/176239 (4613 uncovered), branches 12637/15420 (2783 uncovered sides)
classified strictly: (F) 427 lines 1053 sides; GAP 4186 lines 1730 sides (production 3514 lines 1353 sides; in #[ignore]d tests 233 lines 12 sides)
per file (lines a/b, branches c/d; GAP lines/sides):
  crates/api/src/aliases.rs: lines 374/375, branches 35/36; GAP 1/1
  crates/api/src/assets.rs: lines 833/841, branches 71/82; GAP 5/6
  crates/api/src/audit.rs: lines 3242/3257, branches 176/182; GAP 9/0
  crates/api/src/audit_json.rs: lines 1290/1338, branches 89/102; GAP 46/11
  crates/api/src/audit_json_commit_tests.rs: lines 208/208, branches 5/10; GAP 0/0
  crates/api/src/audit_lock_wait_measure.rs: lines 71/77, branches 2/2; GAP 6/0
  crates/api/src/audit_lock_wait_tests.rs: lines 502/514, branches 17/24; GAP 11/4
  crates/api/src/autopilot.rs: lines 4380/4448, branches 318/378; GAP 33/13
  crates/api/src/autopilot_stall_tests.rs: lines 257/258, branches 7/8; GAP 0/1
  crates/api/src/backtest.rs: lines 1163/1165, branches 44/50; GAP 1/4
  crates/api/src/bars.rs: lines 2734/2761, branches 284/314; GAP 22/24
  crates/api/src/bars_change.rs: lines 659/660, branches 99/104; GAP 1/1
  crates/api/src/bars_export.rs: lines 1134/1204, branches 81/118; GAP 68/33
  crates/api/src/bars_page_tests.rs: lines 372/376, branches 11/14; GAP 3/1
  crates/api/src/bars_route.rs: lines 1557/1573, branches 158/178; GAP 15/17
  crates/api/src/bars_sealed_tests.rs: lines 184/184, branches 2/2; GAP 0/0
  crates/api/src/bars_series.rs: lines 1520/1541, branches 255/270; GAP 17/15
  crates/api/src/bars_sort.rs: lines 3056/3106, branches 422/474; GAP 48/43
  crates/api/src/bars_sort_tests.rs: lines 1276/1282, branches 80/122; GAP 4/3
  crates/api/src/bars_unknown_series_tests.rs: lines 512/513, branches 3/4; GAP 1/1
  crates/api/src/bars_window_route_tests.rs: lines 1776/1785, branches 47/58; GAP 5/6
  crates/api/src/basis_route.rs: lines 233/233, branches 30/30; GAP 0/0
  crates/api/src/basis_route_tests.rs: lines 653/656, branches 4/4; GAP 0/0
  crates/api/src/boolean_observation_budget.rs: lines 71/72, branches 17/24; GAP 1/2
  crates/api/src/boolean_search_budget.rs: lines 42/42, branches 7/8; GAP 0/1
  crates/api/src/booleanadmission_projection.rs: lines 64/65, branches 7/8; GAP 1/1
  crates/api/src/booleanadmission_projection_tests.rs: lines 174/174, branches 8/8; GAP 0/0
  crates/api/src/booleancampaignjson.rs: lines 102/111, branches 16/24; GAP 9/8
  crates/api/src/booleancampaignjson_tests.rs: lines 318/318, branches 2/2; GAP 0/0
  crates/api/src/booleanevidencejson.rs: lines 189/373, branches 29/42; GAP 184/13
  crates/api/src/booleanevidencejson_tests.rs: lines 434/436, branches 51/62; GAP 2/7
  crates/api/src/booleanjson.rs: lines 302/326, branches 36/42; GAP 24/6
  crates/api/src/booleanjson_tests.rs: lines 404/404, branches 10/12; GAP 0/0
  crates/api/src/booleanlaunch.rs: lines 361/393, branches 72/88; GAP 32/16
  crates/api/src/booleanlaunch_tests.rs: lines 568/571, branches 15/18; GAP 3/3
  crates/api/src/booleanlaunch_work.rs: lines 33/34, branches 0/0; GAP 1/0
  crates/api/src/booleanoosjson.rs: lines 177/186, branches 24/30; GAP 9/6
  crates/api/src/booleanoosjson_tests.rs: lines 209/209, branches 6/8; GAP 0/0
  crates/api/src/booleanqualification_projection.rs: lines 44/143, branches 6/12; GAP 99/6
  crates/api/src/booleanqualification_projection_tests.rs: lines 71/71, branches 0/0; GAP 0/0
  crates/api/src/booleansearchjson.rs: lines 123/240, branches 42/68; GAP 117/26
  crates/api/src/booleansearchjson_tests.rs: lines 110/110, branches 0/0; GAP 0/0
  crates/api/src/calendar.rs: lines 421/421, branches 24/24; GAP 0/0
  crates/api/src/calendar_build.rs: lines 460/470, branches 38/50; GAP 4/7
  crates/api/src/calendar_of.rs: lines 1650/1705, branches 106/132; GAP 53/22
  crates/api/src/calendar_record.rs: lines 652/669, branches 80/90; GAP 17/10
  crates/api/src/calendar_record_tests.rs: lines 430/432, branches 18/24; GAP 0/1
  crates/api/src/calendar_route.rs: lines 766/813, branches 76/94; GAP 47/18
  crates/api/src/calendar_route_tests.rs: lines 642/643, branches 25/46; GAP 0/0
  crates/api/src/candidatejson.rs: lines 674/695, branches 67/80; GAP 21/13
  crates/api/src/catalog.rs: lines 659/659, branches 36/42; GAP 0/0
  crates/api/src/census.rs: lines 1518/1624, branches 165/242; GAP 104/68
  crates/api/src/census_cache.rs: lines 1036/1100, branches 114/138; GAP 64/24
  crates/api/src/census_epoch_link_tests.rs: lines 199/206, branches 5/8; GAP 1/1
  crates/api/src/census_request_tests.rs: lines 2274/2291, branches 52/76; GAP 10/7
  crates/api/src/census_rollup.rs: lines 300/331, branches 24/28; GAP 31/4
  crates/api/src/census_tail_tests.rs: lines 377/379, branches 2/4; GAP 2/0
  crates/api/src/census_warm_tests.rs: lines 1235/1250, branches 89/114; GAP 4/19
  crates/api/src/chain.rs: lines 617/678, branches 107/130; GAP 61/23
  crates/api/src/chain_route_tests.rs: lines 652/652, branches 4/8; GAP 0/0
  crates/api/src/constituents.rs: lines 873/873, branches 56/66; GAP 0/4
  crates/api/src/cost_limits_tests.rs: lines 336/336, branches 38/76; GAP 0/0
  crates/api/src/coverage.rs: lines 670/670, branches 35/40; GAP 0/0
  crates/api/src/credential_law.rs: lines 173/181, branches 18/20; GAP 8/2
  crates/api/src/credential_law_tests.rs: lines 502/509, branches 23/42; GAP 2/3
  crates/api/src/dayflags.rs: lines 6001/6221, branches 611/720; GAP 214/101
  crates/api/src/dayflags_filter_tests.rs: lines 576/581, branches 7/10; GAP 0/3
  crates/api/src/dayflags_rules.rs: lines 940/941, branches 184/188; GAP 1/4
  crates/api/src/dayflags_rules_tests.rs: lines 3471/3520, branches 53/82; GAP 19/9
  crates/api/src/dayflags_v2_tests.rs: lines 63/63, branches 4/4; GAP 0/0
  crates/api/src/detail.rs: lines 1264/1326, branches 107/122; GAP 60/9
  crates/api/src/disk_route.rs: lines 1205/1234, branches 137/170; GAP 26/20
  crates/api/src/disk_route_seal_tests.rs: lines 495/495, branches 10/14; GAP 0/0
  crates/api/src/emitted.rs: lines 962/962, branches 78/78; GAP 0/0
  crates/api/src/expressionsearchjson.rs: lines 148/181, branches 18/36; GAP 33/18
  crates/api/src/facets.rs: lines 1344/1422, branches 186/214; GAP 78/28
  crates/api/src/facets_tests.rs: lines 1250/1250, branches 17/22; GAP 0/0
  crates/api/src/fault_log.rs: lines 331/333, branches 35/40; GAP 2/5
  crates/api/src/feed_checks.rs: lines 789/818, branches 99/116; GAP 29/17
  crates/api/src/feed_checks_files.rs: lines 771/832, branches 85/114; GAP 61/29
  crates/api/src/feed_checks_tests.rs: lines 1078/1082, branches 30/44; GAP 4/0
  crates/api/src/feeds_route.rs: lines 1691/1732, branches 148/196; GAP 41/30
  crates/api/src/fno_boundary_tests.rs: lines 1243/1246, branches 24/24; GAP 3/0
  crates/api/src/folder.rs: lines 324/339, branches 15/20; GAP 15/2
  crates/api/src/frontierjson.rs: lines 861/900, branches 33/46; GAP 39/7
  crates/api/src/gap_peer_route_tests.rs: lines 343/349, branches 1/2; GAP 5/1
  crates/api/src/gaps_route.rs: lines 3851/3865, branches 291/326; GAP 5/5
  crates/api/src/hand_stop_tests.rs: lines 155/155, branches 0/0; GAP 0/0
  crates/api/src/head_deadline_body_tests.rs: lines 678/697, branches 56/88; GAP 9/4
  crates/api/src/http_admission_tests.rs: lines 437/440, branches 8/10; GAP 3/2
  crates/api/src/import_journal.rs: lines 1023/1030, branches 59/66; GAP 7/3
  crates/api/src/imports.rs: lines 1610/1641, branches 178/204; GAP 30/24
  crates/api/src/imports_filter_tests.rs: lines 428/428, branches 7/8; GAP 0/0
  crates/api/src/index_consistency_projection.rs: lines 169/258, branches 2/12; GAP 89/10
  crates/api/src/indexmap.rs: lines 769/777, branches 48/58; GAP 8/6
  crates/api/src/indexstop_projection_tests.rs: lines 208/208, branches 4/4; GAP 0/0
  crates/api/src/indexstopcandlesjson.rs: lines 122/216, branches 19/38; GAP 94/19
  crates/api/src/indexstopjson.rs: lines 243/250, branches 27/32; GAP 7/5
  crates/api/src/indexstoplaunch.rs: lines 288/326, branches 103/136; GAP 38/33
  crates/api/src/indexstoplaunch_metadata.rs: lines 166/182, branches 29/38; GAP 16/9
  crates/api/src/indexstoplaunch_tests.rs: lines 556/559, branches 11/14; GAP 3/3
  crates/api/src/indexstopqualificationjson.rs: lines 285/479, branches 24/46; GAP 194/22
  crates/api/src/indexstoprankingjson.rs: lines 308/502, branches 24/48; GAP 194/21
  crates/api/src/indexstopvix_projection_tests.rs: lines 243/243, branches 12/12; GAP 0/0
  crates/api/src/indexstopvixjson.rs: lines 264/275, branches 22/26; GAP 11/4
  crates/api/src/ingest.rs: lines 2066/2106, branches 132/154; GAP 27/10
  crates/api/src/ingest_refusals.rs: lines 385/390, branches 28/30; GAP 5/2
  crates/api/src/instruments_route.rs: lines 690/695, branches 42/58; GAP 5/3
  crates/api/src/isolated.rs: lines 56/56, branches 3/4; GAP 0/1
  crates/api/src/ladder.rs: lines 687/694, branches 64/70; GAP 2/6
  crates/api/src/livejson.rs: lines 366/369, branches 22/36; GAP 1/0
  crates/api/src/logs.rs: lines 7151/7238, branches 908/1018; GAP 62/59
  crates/api/src/logs_units_tests.rs: lines 171/172, branches 12/20; GAP 1/0
  crates/api/src/main.rs: lines 175/181, branches 1/2; GAP 6/1
  crates/api/src/master.rs: lines 611/620, branches 43/60; GAP 7/3
  crates/api/src/mastersrun.rs: lines 1136/1188, branches 71/90; GAP 52/16
  crates/api/src/merge.rs: lines 761/764, branches 59/74; GAP 3/5
  crates/api/src/month_change.rs: lines 233/234, branches 32/36; GAP 1/1
  crates/api/src/nse_identity.rs: lines 322/327, branches 15/20; GAP 0/4
  crates/api/src/operation_audit.rs: lines 198/223, branches 40/46; GAP 25/6
  crates/api/src/operation_audit_tests.rs: lines 534/545, branches 6/8; GAP 10/1
  crates/api/src/ops_safety_tests.rs: lines 344/351, branches 17/32; GAP 5/7
  crates/api/src/optiongreeks.rs: lines 441/479, branches 45/56; GAP 38/11
  crates/api/src/optiongreeks_route_tests.rs: lines 644/647, branches 21/38; GAP 1/0
  crates/api/src/owed_tests.rs: lines 333/334, branches 17/22; GAP 1/2
  crates/api/src/page_report_route.rs: lines 129/130, branches 23/24; GAP 1/1
  crates/api/src/page_report_route_tests.rs: lines 97/97, branches 1/2; GAP 0/0
  crates/api/src/pool_table_tests.rs: lines 1105/1142, branches 28/42; GAP 19/8
  crates/api/src/provenance.rs: lines 1086/1170, branches 134/164; GAP 84/30
  crates/api/src/provenance_tests.rs: lines 1323/1325, branches 5/6; GAP 0/0
  crates/api/src/pullrun.rs: lines 6729/6756, branches 445/520; GAP 16/17
  crates/api/src/pullrun_restart_tests.rs: lines 1011/1024, branches 25/50; GAP 9/1
  crates/api/src/quarantine_route.rs: lines 119/130, branches 12/12; GAP 11/0
  crates/api/src/quarantine_route_tests.rs: lines 415/420, branches 15/20; GAP 3/4
  crates/api/src/recovery.rs: lines 2377/2766, branches 271/452; GAP 382/175
  crates/api/src/recovery_control.rs: lines 310/319, branches 18/26; GAP 9/8
  crates/api/src/recovery_journal.rs: lines 1261/1277, branches 88/98; GAP 16/6
  crates/api/src/render.rs: lines 2636/2648, branches 248/288; GAP 6/6
  crates/api/src/repair_route.rs: lines 541/557, branches 30/40; GAP 15/9
  crates/api/src/research_doors_tests.rs: lines 87/89, branches 2/2; GAP 1/0
  crates/api/src/saved_response_boundary_tests.rs: lines 180/183, branches 1/2; GAP 3/1
  crates/api/src/scratch.rs: lines 107/107, branches 8/8; GAP 0/0
  crates/api/src/scrub_route.rs: lines 1519/1530, branches 170/186; GAP 11/16
  crates/api/src/selectionv6json.rs: lines 154/163, branches 14/16; GAP 9/2
  crates/api/src/selectionv6json_tests.rs: lines 182/182, branches 2/2; GAP 0/0
  crates/api/src/serve_edge_tests.rs: lines 235/243, branches 14/16; GAP 8/2
  crates/api/src/served_routes_tests.rs: lines 277/277, branches 4/4; GAP 0/0
  crates/api/src/server.rs: lines 18625/19053, branches 1238/1512; GAP 373/95
  crates/api/src/server_cloud_mutant_tests_a.rs: lines 1297/1305, branches 37/46; GAP 5/5
  crates/api/src/server_cloud_mutant_tests_b.rs: lines 1626/1659, branches 15/28; GAP 12/11
  crates/api/src/server_cloud_mutant_tests_c.rs: lines 2382/2398, branches 33/44; GAP 11/6
  crates/api/src/server_cloud_mutant_tests_d.rs: lines 581/584, branches 5/6; GAP 1/0
  crates/api/src/server_cloud_mutant_tests_e.rs: lines 545/548, branches 11/16; GAP 2/5
  crates/api/src/settings_route.rs: lines 657/694, branches 71/80; GAP 37/9
  crates/api/src/settings_route_tests.rs: lines 734/748, branches 24/40; GAP 10/6
  crates/api/src/sources.rs: lines 2679/2766, branches 475/542; GAP 87/67
  crates/api/src/sources_day_truth_tests.rs: lines 490/490, branches 4/4; GAP 0/0
  crates/api/src/sources_tests.rs: lines 1698/1701, branches 8/10; GAP 1/1
  crates/api/src/speed.rs: lines 525/557, branches 72/82; GAP 32/10
  crates/api/src/speed_route.rs: lines 47/54, branches 4/4; GAP 7/0
  crates/api/src/speed_tests.rs: lines 733/734, branches 30/56; GAP 0/0
  crates/api/src/spot.rs: lines 409/409, branches 56/56; GAP 0/0
  crates/api/src/spot_route_tests.rs: lines 429/429, branches 8/16; GAP 0/0
  crates/api/src/stop_tests.rs: lines 426/436, branches 17/32; GAP 6/5
  crates/api/src/store_route.rs: lines 1527/1551, branches 126/144; GAP 24/9
  crates/api/src/store_wire.rs: lines 321/329, branches 15/18; GAP 6/3
  crates/api/src/strict_sweep_tests.rs: lines 501/512, branches 14/24; GAP 10/10
  crates/api/src/sweepevidence.rs: lines 486/496, branches 45/50; GAP 10/5
  crates/api/src/sweeprun.rs: lines 4150/4234, branches 281/334; GAP 62/35
  crates/api/src/sweeprun_accepted_tests.rs: lines 308/308, branches 10/10; GAP 0/0
  crates/api/src/sweeprun_admission_tests.rs: lines 157/157, branches 0/0; GAP 0/0
  crates/api/src/sweeprun_status_cache_tests.rs: lines 438/440, branches 3/4; GAP 1/1
  crates/api/src/topjson.rs: lines 370/384, branches 15/24; GAP 14/9
  crates/api/src/trades.rs: lines 955/979, branches 35/46; GAP 24/9
  crates/api/src/verification_route_tests.rs: lines 182/182, branches 0/0; GAP 0/0
  crates/api/src/verify.rs: lines 176/187, branches 26/28; GAP 9/2
  crates/api/tests/binary.rs: lines 100/100, branches 0/0; GAP 0/0
  crates/api/tests/cloud_mutants_bars.rs: lines 1749/1751, branches 17/32; GAP 0/5
  crates/api/tests/logs_r3_24.rs: lines 247/255, branches 13/16; GAP 8/3
  crates/api/tests/pool_load.rs: lines 509/528, branches 48/70; GAP 11/18
  crates/api/tests/state_stream_emits.rs: lines 481/502, branches 149/168; GAP 21/19
  crates/api/tests/store_step_forms.rs: lines 51/53, branches 0/0; GAP 2/0

rows:
GAP prod LINE crates/api/src/aliases.rs:195  out.push(',');
GAP prod BRANCH crates/api/src/aliases.rs:194:12 True  if n > 0 {
GAP prod LINE crates/api/src/assets.rs:417  Err(_) => tree.unreadable = tree.unreadable.saturating_add(1),
GAP prod LINE crates/api/src/assets.rs:643  break;
GAP prod LINE crates/api/src/assets.rs:651  continue;
GAP prod LINE crates/api/src/assets.rs:658  continue;
GAP test LINE crates/api/src/assets.rs:1568  built.build()
GAP prod BRANCH crates/api/src/assets.rs:293:51 False  matches!(segments, [app, immutable, _, ..] if app == IMMUTABLE_DIR && immutable == "immutable")
GAP prod BRANCH crates/api/src/assets.rs:293:75 False  matches!(segments, [app, immutable, _, ..] if app == IMMUTABLE_DIR && immutable == "immutable")
GAP prod BRANCH crates/api/src/assets.rs:642:12 True  if seen >= limit {
GAP prod BRANCH crates/api/src/assets.rs:650:17 False  let Ok(kind) = entry.file_type() else {
GAP prod BRANCH crates/api/src/assets.rs:657:17 False  let Ok(at) = entry.metadata().and_then(|m| m.modified()) else {
GAP test BRANCH crates/api/src/assets.rs:1083:16 False  if let Some(parent) = path.parent() {
GAP prod LINE crates/api/src/audit.rs:634  _ => 0,
GAP prod LINE crates/api/src/audit.rs:1916  )?;
GAP prod LINE crates/api/src/audit.rs:2000  format!(
GAP prod LINE crates/api/src/audit.rs:2002  self.path.display()
GAP prod LINE crates/api/src/audit.rs:2004  })?;
GAP ignored LINE crates/api/src/audit.rs:5365  fn append_synced_cost_per_entry_is_measured() {
GAP ignored LINE crates/api/src/audit.rs:5366  appends_timed("audit-append-timing", 400, 250_000);
GAP ignored LINE crates/api/src/audit.rs:5367  }
GAP test LINE crates/api/src/audit.rs:5589  page[0].whole_note
GAP prod LINE crates/api/src/audit_json.rs:365  Err(ref why) => (0, 0, 0, 0, 0, Some(why.clone())),
GAP prod LINE crates/api/src/audit_json.rs:391  render::json_string(&trouble)
GAP prod LINE crates/api/src/audit_json.rs:413  Err(why) => Err(why),
GAP prod LINE crates/api/src/audit_json.rs:428  Err(why) => {
GAP prod LINE crates/api/src/audit_json.rs:429  let _ = write!(
GAP prod LINE crates/api/src/audit_json.rs:430  out,
GAP prod LINE crates/api/src/audit_json.rs:431  r#","last_runs":[],"last_runs_error":{}"#,
GAP prod LINE crates/api/src/audit_json.rs:432  render::json_string(&why)
GAP prod LINE crates/api/src/audit_json.rs:433  );
GAP prod LINE crates/api/src/audit_json.rs:434  }
GAP prod LINE crates/api/src/audit_json.rs:450  Err(why) => {
GAP prod LINE crates/api/src/audit_json.rs:451  let _ = write!(
GAP prod LINE crates/api/src/audit_json.rs:452  out,
GAP prod LINE crates/api/src/audit_json.rs:453  r#","runs":[],"runs_error":{}"#,
GAP prod LINE crates/api/src/audit_json.rs:454  render::json_string(&why)
GAP prod LINE crates/api/src/audit_json.rs:455  );
GAP prod LINE crates/api/src/audit_json.rs:456  }
GAP prod LINE crates/api/src/audit_json.rs:482  format!(
GAP prod LINE crates/api/src/audit_json.rs:485  feed.as_str()
GAP prod LINE crates/api/src/audit_json.rs:552  out.push((0, end, None));
GAP prod LINE crates/api/src/audit_json.rs:634  continue;
GAP prod LINE crates/api/src/audit_json.rs:674  Err(_) => self.faults = self.faults.saturating_add(1),
GAP prod LINE crates/api/src/audit_json.rs:679  return Err(format!(
GAP prod LINE crates/api/src/audit_json.rs:680  "{}: the file ended before record {} of {total}",
GAP prod LINE crates/api/src/audit_json.rs:681  path.display(),
GAP prod LINE crates/api/src/audit_json.rs:682  self.scanned
GAP prod LINE crates/api/src/audit_json.rs:683  ));
GAP prod LINE crates/api/src/audit_json.rs:801  Err(ref fault) => {
GAP prod LINE crates/api/src/audit_json.rs:802  let _ = write!(
GAP prod LINE crates/api/src/audit_json.rs:803  out,
GAP prod LINE crates/api/src/audit_json.rs:804  r#"{{"ordinal":{},"fault":{}}}"#,
GAP prod LINE crates/api/src/audit_json.rs:805  entry.ordinal,
GAP prod LINE crates/api/src/audit_json.rs:806  render::json_string(&fault.to_string())
GAP prod LINE crates/api/src/audit_json.rs:807  );
GAP prod LINE crates/api/src/audit_json.rs:808  }
GAP prod LINE crates/api/src/audit_json.rs:852  match entry.whole_note {
GAP prod LINE crates/api/src/audit_json.rs:853  Some(Ok(ref whole)) => render::json_string(whole),
GAP prod LINE crates/api/src/audit_json.rs:856  match entry.whole_note {
GAP prod LINE crates/api/src/audit_json.rs:857  Some(Err(ref why)) => render::json_string(why),
GAP prod LINE crates/api/src/audit_json.rs:1016  || std::sync::Arc::new(FeedRollup::default()),
GAP prod LINE crates/api/src/audit_json.rs:1041  || format!("no census row for {} in this build", feed.as_str()),
GAP prod LINE crates/api/src/audit_json.rs:1054  || render::json_string(""),
GAP prod LINE crates/api/src/audit_json.rs:1075  |day| render::json_string(&day.to_string())
GAP prod LINE crates/api/src/audit_json.rs:1249  Err(why) => Err(why.to_string()),
GAP prod LINE crates/api/src/audit_json.rs:1300  "no commit time can be stated"
GAP prod LINE crates/api/src/audit_json.rs:1345  Err(ref why) => Some(format!("{path} could not be stat'ed: {why}")),
GAP prod BRANCH crates/api/src/audit_json.rs:243:28 True  if asked.is_empty() || asked.parse::<usize>().is_ok() {
GAP prod BRANCH crates/api/src/audit_json.rs:388:12 False  if trouble.is_empty() {
GAP prod BRANCH crates/api/src/audit_json.rs:475:15 False  } else if http {
GAP prod BRANCH crates/api/src/audit_json.rs:546:16 False  if first < end {
GAP prod BRANCH crates/api/src/audit_json.rs:551:12 True  if end > 0 {
GAP prod BRANCH crates/api/src/audit_json.rs:633:17 False  let Some(entry) = record.into_iter().next() else {
GAP prod BRANCH crates/api/src/audit_json.rs:678:16 True  if whole == 0 {
GAP prod BRANCH crates/api/src/audit_json.rs:1068:27 False  if behind.kept && behind.unread.is_none() {
GAP prod BRANCH crates/api/src/audit_json.rs:1248:24 False  Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Ok(None),
GAP prod BRANCH crates/api/src/audit_json.rs:1297:20 False  if at.is_some() {
GAP test BRANCH crates/api/src/audit_json.rs:2008:26 True  [inside a closure whose false side does not fail the check]  .any(|s| s.contains("groww") || s == "Reference indices")
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:33  fn measure_append_latency_under_contention() {
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:34  let root = std::env::var("BRUTEX_AG06_MEASURE_ROOT")
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:35  .expect("BRUTEX_AG06_MEASURE_ROOT names a store root holding a CLONE of the journal");
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:36  let _refused = latency_over(Path::new(&root), 200);
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:37  }
GAP test LINE crates/api/src/audit_lock_wait_measure.rs:84  Err(_) => journal.close_unopened(&head, &close),
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:105  return Vec::new();
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:214  member(7).image().to_vec()
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:215  })
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:291  unnamed += 1;
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:323  closed("z").image().to_vec()
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:324  })
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:654  let path = PathBuf::from(path);
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:655  let held = Journal { path: path.clone() };
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:656  let _guard = hold(&held);
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:657  std::fs::write(path.with_extension("held"), b"1").expect("the marker");
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:658  std::thread::sleep(Duration::from_secs(30));
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:104:8 True  if n == 0 {
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:288:21 False  held_now(&journal) && wrong()
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:290:20 True  if still {
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:652:12 True  if let Ok(path) = std::env::var(CHILD_HOLDS) {
GAP prod LINE crates/api/src/autopilot.rs:319  _ => String::new(),
GAP prod LINE crates/api/src/autopilot.rs:811  why
GAP prod LINE crates/api/src/autopilot.rs:959  done = done.saturating_add(1);
GAP prod LINE crates/api/src/autopilot.rs:960  continue;
GAP prod LINE crates/api/src/autopilot.rs:2899  return 0;
GAP prod LINE crates/api/src/autopilot.rs:2993  }
GAP prod LINE crates/api/src/autopilot.rs:2994  let Some(saying) = clock_wait(clock_waits) else {
GAP prod LINE crates/api/src/autopilot.rs:2995  site.autopilot.publish(|status| {
GAP prod LINE crates/api/src/autopilot.rs:2996  status.phase = Phase::Halted;
GAP prod LINE crates/api/src/autopilot.rs:2997  status.detail = format!(
GAP prod LINE crates/api/src/autopilot.rs:3003  u64::from(CLOCK_WAITS) * IDLE_POLL_SECS / 60
GAP prod LINE crates/api/src/autopilot.rs:3005  status.due_unix = 0;
GAP prod LINE crates/api/src/autopilot.rs:3006  });
GAP prod LINE crates/api/src/autopilot.rs:3007  return;
GAP prod LINE crates/api/src/autopilot.rs:3009  site.autopilot.publish(move |status| {
GAP prod LINE crates/api/src/autopilot.rs:3010  status.phase = Phase::Halted;
GAP prod LINE crates/api/src/autopilot.rs:3011  status.detail = saying;
GAP prod LINE crates/api/src/autopilot.rs:3012  status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
GAP prod LINE crates/api/src/autopilot.rs:3013  status.due_unix = 0;
GAP prod LINE crates/api/src/autopilot.rs:3014  });
GAP prod LINE crates/api/src/autopilot.rs:3015  clock_waits = clock_waits.saturating_add(1);
GAP prod LINE crates/api/src/autopilot.rs:3016  tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECS)).await;
GAP prod LINE crates/api/src/autopilot.rs:3048  site.autopilot.publish(|status| {
GAP prod LINE crates/api/src/autopilot.rs:3049  status.phase = Phase::Halted;
GAP prod LINE crates/api/src/autopilot.rs:3050  status.detail = String::from("the daily rung has no directory in this store build");
GAP prod LINE crates/api/src/autopilot.rs:3051  });
GAP prod LINE crates/api/src/autopilot.rs:3052  return;
GAP prod LINE crates/api/src/autopilot.rs:3103  continue;
GAP prod LINE crates/api/src/autopilot.rs:3115  }
GAP prod LINE crates/api/src/autopilot.rs:3639  return Pass::owed(IDLE_POLL_SECS);
GAP prod LINE crates/api/src/autopilot.rs:3727  return Pass::owed(IDLE_POLL_SECS);
GAP test LINE crates/api/src/autopilot.rs:8619  Ok(other) => panic!("unknown case {other}"),
GAP test LINE crates/api/src/autopilot.rs:8912  });
GAP prod BRANCH crates/api/src/autopilot.rs:800:16 False  if why.kind() == std::io::ErrorKind::AlreadyExists {
GAP prod BRANCH crates/api/src/autopilot.rs:958:21 False  let Ok(next) = day.succ() else {
GAP prod BRANCH crates/api/src/autopilot.rs:2898:9 False  let (Ok(from), Ok(to)) = (floor.year_month(), yesterday.year_month()) else {
GAP prod BRANCH crates/api/src/autopilot.rs:2991:16 False  if let Some(day) = yesterday_ist(std::time::SystemTime::now()) {
GAP prod BRANCH crates/api/src/autopilot.rs:2994:13 True  let Some(saying) = clock_wait(clock_waits) else {
GAP prod BRANCH crates/api/src/autopilot.rs:2994:13 False  let Some(saying) = clock_wait(clock_waits) else {
GAP prod BRANCH crates/api/src/autopilot.rs:3047:9 False  let Some(timeframe) = granularity.store_timeframe() else {
GAP prod BRANCH crates/api/src/autopilot.rs:3102:13 False  let Some(rung_timeframe) = rung.store_timeframe() else {
GAP prod BRANCH crates/api/src/autopilot.rs:3113:12 False  if waited > 0 {
GAP prod BRANCH crates/api/src/autopilot.rs:3638:9 False  let Some(yesterday) = yesterday_ist(std::time::SystemTime::now()) else {
GAP prod BRANCH crates/api/src/autopilot.rs:3726:9 False  let Some(state) = feeds.get_mut(slot) else {
GAP test BRANCH crates/api/src/autopilot.rs:4812:13 False  let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Dhan.descriptor().transport
GAP test BRANCH crates/api/src/autopilot.rs:8177:28 False  [inside a closure whose false side does not fail the check]  .any(|one| one.symbol.as_str() == "NIFTY" && one.exchange == Exchange::Nse),
GAP test BRANCH crates/api/src/autopilot_stall_tests.rs:65:20 False  if let Some(next) = month_after(state.frontier) {
GAP test LINE crates/api/src/backtest.rs:2192  [read: the second operand of an || in the condition, never reached because the first held: not the failing arm]  || status == axum::http::StatusCode::SERVICE_UNAVAILABLE
GAP prod BRANCH crates/api/src/backtest.rs:927:40 False  self.version == VERSION && self.refusal.is_none() && !self.partial_tail
GAP test BRANCH crates/api/src/backtest.rs:2191:13 False  [read: the first operand of an || continued on the next line: its false side does not fail the check]  status == axum::http::StatusCode::OK
GAP test BRANCH crates/api/src/backtest.rs:2192:20 True  || status == axum::http::StatusCode::SERVICE_UNAVAILABLE
GAP test BRANCH crates/api/src/backtest.rs:2192:20 False  || status == axum::http::StatusCode::SERVICE_UNAVAILABLE
GAP prod LINE crates/api/src/bars.rs:749  break;
GAP prod LINE crates/api/src/bars.rs:956  Self::Change => 0,
GAP prod LINE crates/api/src/bars.rs:2176  return Behind::Nothing;
GAP prod LINE crates/api/src/bars.rs:2219  return Ok(Vec::new());
GAP prod LINE crates/api/src/bars.rs:2243  continue;
GAP prod LINE crates/api/src/bars.rs:2246  continue;
GAP prod LINE crates/api/src/bars.rs:2368  break;
GAP prod LINE crates/api/src/bars.rs:2371  break;
GAP prod LINE crates/api/src/bars.rs:2457  Behind::Unreadable
GAP prod LINE crates/api/src/bars.rs:2493  continue;
GAP prod LINE crates/api/src/bars.rs:2500  continue;
GAP prod LINE crates/api/src/bars.rs:2504  continue;
GAP prod LINE crates/api/src/bars.rs:2572  continue;
GAP prod LINE crates/api/src/bars.rs:2575  continue;
GAP prod LINE crates/api/src/bars.rs:2587  Err(why) => faults.push(why),
GAP prod LINE crates/api/src/bars.rs:2609  continue;
GAP test LINE crates/api/src/bars.rs:2823  return;
GAP test LINE crates/api/src/bars.rs:2826  let path = entry.path();
GAP test LINE crates/api/src/bars.rs:2827  if path.is_dir() {
GAP test LINE crates/api/src/bars.rs:2828  walk(&path, into);
GAP test LINE crates/api/src/bars.rs:2829  }
GAP test LINE crates/api/src/bars.rs:2830  into.push(path.display().to_string());
GAP prod BRANCH crates/api/src/bars.rs:738:16 False  && let (Ok(first), Ok(end)) = (
GAP prod BRANCH crates/api/src/bars.rs:748:13 False  let Ok(index) = u64::try_from(i) else {
GAP prod BRANCH crates/api/src/bars.rs:1788:29 True  Err(why) if needs_scan(&why) => return Err(why),
GAP prod BRANCH crates/api/src/bars.rs:1868:29 True  Err(why) if needs_scan(&why) => return Err(why),
GAP prod BRANCH crates/api/src/bars.rs:2175:9 False  let Some(month) = plan.month_of_slot(run.slot) else {
GAP prod BRANCH crates/api/src/bars.rs:2218:13 False  let Some(plan) = plans.first() else {
GAP prod BRANCH crates/api/src/bars.rs:2242:13 False  let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
GAP prod BRANCH crates/api/src/bars.rs:2245:13 False  let Some(run) = plan.runs.get(at.run).copied() else {
GAP prod BRANCH crates/api/src/bars.rs:2253:20 False  if let Some((_, rows)) = unopened.get_mut(index) {
GAP prod BRANCH crates/api/src/bars.rs:2256:20 False  if let Some(slot) = last.get_mut(series) {
GAP prod BRANCH crates/api/src/bars.rs:2267:16 False  if let Some(slot) = last.get_mut(series) {
GAP prod BRANCH crates/api/src/bars.rs:2367:13 False  let Some((_, series)) = best else {
GAP prod BRANCH crates/api/src/bars.rs:2370:13 False  let Some(Some((_, at))) = heads.get(series).copied() else {
GAP prod BRANCH crates/api/src/bars.rs:2378:16 False  if let (Some(plan), Some(slot)) = (plans.get_mut(series), heads.get_mut(series)) {
GAP prod BRANCH crates/api/src/bars.rs:2454:25 False  let stands = if *expected == row {
GAP prod BRANCH crates/api/src/bars.rs:2492:13 False  let Some(series_rules) = rules.get(series) else {
GAP prod BRANCH crates/api/src/bars.rs:2498:13 False  let (Ok(series_at), Ok(runs)) = (u32::try_from(series), u32::try_from(plan.runs.len()))
GAP prod BRANCH crates/api/src/bars.rs:2503:17 False  let Some(run) = plan.runs.get(run_at as usize).copied() else {
GAP prod BRANCH crates/api/src/bars.rs:2571:13 False  let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
GAP prod BRANCH crates/api/src/bars.rs:2574:13 False  let Some(run) = plan.runs.get(chosen.run as usize).copied() else {
GAP prod BRANCH crates/api/src/bars.rs:2607:17 False  let Some((month, rows)) = plan.slots.get(slot).map(|at| (at.month, at.held.n_valid))
GAP test BRANCH crates/api/src/bars.rs:2822:17 False  let Ok(entries) = std::fs::read_dir(dir) else {
GAP test BRANCH crates/api/src/bars.rs:2827:20 True  if path.is_dir() {
GAP test BRANCH crates/api/src/bars.rs:2827:20 False  if path.is_dir() {
GAP prod LINE crates/api/src/bars_change.rs:442  None
GAP prod BRANCH crates/api/src/bars_change.rs:435:11 False  while at >= day - MAX_CLOSED_WALK {
GAP prod LINE crates/api/src/bars_export.rs:183  _ => None,
GAP prod LINE crates/api/src/bars_export.rs:308  out.push_str(",oi_null,");
GAP prod LINE crates/api/src/bars_export.rs:406  fn sentence(&self) -> String {
GAP prod LINE crates/api/src/bars_export.rs:407  match self {
GAP prod LINE crates/api/src/bars_export.rs:408  Self::Pool(why, heavy) => format!(
GAP prod LINE crates/api/src/bars_export.rs:410  if *heavy { "scan" } else { "store-read" }
GAP prod LINE crates/api/src/bars_export.rs:412  Self::Read(why) => why.clone(),
GAP prod LINE crates/api/src/bars_export.rs:414  }
GAP prod LINE crates/api/src/bars_export.rs:444  match answered {
GAP prod LINE crates/api/src/bars_export.rs:446  Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
GAP prod LINE crates/api/src/bars_export.rs:447  Ok(Err(why)) => return Err(Refused::Read(why)),
GAP prod LINE crates/api/src/bars_export.rs:549  return Err(format!(
GAP prod LINE crates/api/src/bars_export.rs:550  "{count} record(s) or month(s) of this query would not read past row {}, \
GAP prod LINE crates/api/src/bars_export.rs:551  first: {first}",
GAP prod LINE crates/api/src/bars_export.rs:552  self.written
GAP prod LINE crates/api/src/bars_export.rs:553  ));
GAP prod LINE crates/api/src/bars_export.rs:565  return Err(format!(
GAP prod LINE crates/api/src/bars_export.rs:566  "the page at row {} held {} row(s) of a query keeping {}",
GAP prod LINE crates/api/src/bars_export.rs:567  self.written, page.rows, self.total
GAP prod LINE crates/api/src/bars_export.rs:568  ));
GAP prod LINE crates/api/src/bars_export.rs:588  return Poll::Ready(None);
GAP prod LINE crates/api/src/bars_export.rs:612  fn is_end_stream(&self) -> bool {
GAP prod LINE crates/api/src/bars_export.rs:613  self.ended && self.ready.is_none()
GAP prod LINE crates/api/src/bars_export.rs:614  }
GAP prod LINE crates/api/src/bars_export.rs:780  "selection"
GAP prod LINE crates/api/src/bars_export.rs:789  safe.push('-');
GAP prod LINE crates/api/src/bars_export.rs:790  dashed = true;
GAP prod LINE crates/api/src/bars_export.rs:791  }
GAP prod LINE crates/api/src/bars_export.rs:802  match why {
GAP prod LINE crates/api/src/bars_export.rs:806  "a bar export scan"
GAP prod LINE crates/api/src/bars_export.rs:811  super::bars_route::MAX_SCAN_CONCURRENT
GAP prod LINE crates/api/src/bars_export.rs:825  Refused::Read(why) if pull::gdfl_lineage::is_contract_refused(why) => {
GAP prod LINE crates/api/src/bars_export.rs:826  refused(StatusCode::GONE, "contract_refused", why)
GAP prod LINE crates/api/src/bars_export.rs:828  Refused::Read(why) => {
GAP prod LINE crates/api/src/bars_export.rs:829  match crate::detail::being_written_answer(std::slice::from_ref(why)) {
GAP prod LINE crates/api/src/bars_export.rs:830  Some((status, body)) => (
GAP prod LINE crates/api/src/bars_export.rs:831  status,
GAP prod LINE crates/api/src/bars_export.rs:832  [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
GAP prod LINE crates/api/src/bars_export.rs:833  body,
GAP prod LINE crates/api/src/bars_export.rs:834  )
GAP prod LINE crates/api/src/bars_export.rs:835  .into_response(),
GAP prod LINE crates/api/src/bars_export.rs:836  None => refused(StatusCode::BAD_REQUEST, "bad_query", why),
GAP prod LINE crates/api/src/bars_export.rs:890  return Some(refused(
GAP prod LINE crates/api/src/bars_export.rs:891  StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/bars_export.rs:892  "export_not_whole",
GAP prod LINE crates/api/src/bars_export.rs:893  &format!(
GAP prod LINE crates/api/src/bars_export.rs:894  "the first page held {} row(s) of a query keeping {}, so it is not exported",
GAP prod LINE crates/api/src/bars_export.rs:895  first.rows, first.total
GAP prod LINE crates/api/src/bars_export.rs:896  ),
GAP prod LINE crates/api/src/bars_export.rs:897  ));
GAP prod LINE crates/api/src/bars_export.rs:974  (Err(why), _) | (_, Err(why)) => return refused_read(&why),
GAP prod LINE crates/api/src/bars_export.rs:979  Err(refusal) => return *refusal,
GAP prod LINE crates/api/src/bars_export.rs:984  Err(why) => return refused(StatusCode::INTERNAL_SERVER_ERROR, "file_name", &why),
GAP test LINE crates/api/src/bars_export.rs:1110  tokio::time::sleep(Duration::from_millis(20)).await;
GAP test LINE crates/api/src/bars_export.rs:1111  continue;
GAP test LINE crates/api/src/bars_export.rs:1133  tokio::time::sleep(Duration::from_millis(20)).await;
GAP test LINE crates/api/src/bars_export.rs:1134  continue;
GAP test LINE crates/api/src/bars_export.rs:1215  match read {
GAP test LINE crates/api/src/bars_export.rs:1216  Ok(page) => return Ok(page),
GAP test LINE crates/api/src/bars_export.rs:1218  tokio::time::sleep(Duration::from_millis(20)).await;
GAP test LINE crates/api/src/bars_export.rs:1220  Err(why) => return Err(Refused::Pool(why, false)),
GAP test LINE crates/api/src/bars_export.rs:1223  });
GAP test LINE crates/api/src/bars_export.rs:1242  }
GAP test LINE crates/api/src/bars_export.rs:1243  assert!(
GAP test LINE crates/api/src/bars_export.rs:1244  began.elapsed() < Duration::from_secs(45),
GAP test LINE crates/api/src/bars_export.rs:1625  }
GAP test LINE crates/api/src/bars_export.rs:1626  tokio::time::sleep(Duration::from_millis(20)).await;
GAP test LINE crates/api/src/bars_export.rs:1649  }
GAP prod BRANCH crates/api/src/bars_export.rs:307:15 True  } else if bar.open_interest == store::format::OI_NULL {
GAP prod BRANCH crates/api/src/bars_export.rs:410:20 True  if *heavy { "scan" } else { "store-read" }
GAP prod BRANCH crates/api/src/bars_export.rs:410:20 False  if *heavy { "scan" } else { "store-read" }
GAP prod BRANCH crates/api/src/bars_export.rs:446:29 True  Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
GAP prod BRANCH crates/api/src/bars_export.rs:446:29 False  Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
GAP prod BRANCH crates/api/src/bars_export.rs:446:39 True  Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
GAP prod BRANCH crates/api/src/bars_export.rs:446:39 False  Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,
GAP prod BRANCH crates/api/src/bars_export.rs:548:16 True  if let Some((first, count)) = page.fault {
GAP prod BRANCH crates/api/src/bars_export.rs:564:12 True  if page.rows == 0 || after > self.total {
GAP prod BRANCH crates/api/src/bars_export.rs:564:30 True  if page.rows == 0 || after > self.total {
GAP prod BRANCH crates/api/src/bars_export.rs:587:12 True  if this.ended {
GAP prod BRANCH crates/api/src/bars_export.rs:613:9 True  self.ended && self.ready.is_none()
GAP prod BRANCH crates/api/src/bars_export.rs:613:9 False  self.ended && self.ready.is_none()
GAP prod BRANCH crates/api/src/bars_export.rs:736:16 False  && let Some(days) = first.days()
GAP prod BRANCH crates/api/src/bars_export.rs:777:19 False  let what = if !symbol.is_empty() && query.series.iter().all(|key| key.symbol == symbol) {
GAP prod BRANCH crates/api/src/bars_export.rs:777:41 False  let what = if !symbol.is_empty() && query.series.iter().all(|key| key.symbol == symbol) {
GAP prod BRANCH crates/api/src/bars_export.rs:785:12 False  if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
GAP prod BRANCH crates/api/src/bars_export.rs:788:19 True  } else if !dashed {
GAP prod BRANCH crates/api/src/bars_export.rs:788:19 False  } else if !dashed {
GAP prod BRANCH crates/api/src/bars_export.rs:805:20 True  if *heavy {
GAP prod BRANCH crates/api/src/bars_export.rs:810:20 True  if *heavy {
GAP prod BRANCH crates/api/src/bars_export.rs:825:31 True  Refused::Read(why) if pull::gdfl_lineage::is_contract_refused(why) => {
GAP prod BRANCH crates/api/src/bars_export.rs:825:31 False  Refused::Read(why) if pull::gdfl_lineage::is_contract_refused(why) => {
GAP prod BRANCH crates/api/src/bars_export.rs:889:8 True  if first.rows == 0 || first.rows > first.total {
GAP prod BRANCH crates/api/src/bars_export.rs:889:27 True  if first.rows == 0 || first.rows > first.total {
GAP test BRANCH crates/api/src/bars_export.rs:1109:20 True  if answer.status() == StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/bars_export.rs:1132:20 True  if status == StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/bars_export.rs:1180:17 False  crate::emitted::says(record, "route", ROUTE)
GAP test BRANCH crates/api/src/bars_export.rs:1207:36 False  && began.elapsed() < Duration::from_secs(30)
GAP test BRANCH crates/api/src/bars_export.rs:1240:16 False  if started_rx.try_recv().is_ok() {
GAP test BRANCH crates/api/src/bars_export.rs:1623:16 False  if answer.status() != StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/bars_export.rs:1644:19 False  while let Some(frame) = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await
GAP test BRANCH crates/api/src/bars_export.rs:1646:20 False  if let Err(why) = frame {
GAP test LINE crates/api/src/bars_page_tests.rs:458  other => panic!("not a route here: {other}"),
GAP test LINE crates/api/src/bars_page_tests.rs:462  }
GAP test LINE crates/api/src/bars_page_tests.rs:463  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test BRANCH crates/api/src/bars_page_tests.rs:460:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP prod LINE crates/api/src/bars_route.rs:489  faults.push(one);
GAP prod LINE crates/api/src/bars_route.rs:490  }
GAP prod LINE crates/api/src/bars_route.rs:788  _ => None,
GAP prod LINE crates/api/src/bars_route.rs:976  86_399
GAP prod LINE crates/api/src/bars_route.rs:1122  None
GAP prod LINE crates/api/src/bars_route.rs:1149  Some((status, body)) => (status, json(), body),
GAP prod LINE crates/api/src/bars_route.rs:1480  format!(
GAP prod LINE crates/api/src/bars_route.rs:1482  window
GAP prod LINE crates/api/src/bars_route.rs:1483  .series
GAP prod LINE crates/api/src/bars_route.rs:1484  .get(month.series)
GAP prod LINE crates/api/src/bars_route.rs:1485  .map_or_else(|| "null".to_owned(), |held| render::json_string(&held.key)),
GAP prod LINE crates/api/src/bars_route.rs:1486  render::json_string(&month.month.to_string()),
GAP prod LINE crates/api/src/bars_route.rs:1490  }));
GAP test LINE crates/api/src/bars_route.rs:2315  assert_eq!(scan, axum::http::StatusCode::TOO_MANY_REQUESTS, "{refused}");
GAP test LINE crates/api/src/bars_route.rs:2518  assert_eq!(
GAP prod BRANCH crates/api/src/bars_route.rs:573:16 False  if entry.scope == store::quarantine::Scope::WholeMonth =>
GAP prod BRANCH crates/api/src/bars_route.rs:975:25 True  let hi = if tod_to.is_empty() {
GAP prod BRANCH crates/api/src/bars_route.rs:1119:40 False  let waiting_on_writer = if window.faulted_records == 0 {
GAP prod BRANCH crates/api/src/bars_route.rs:1182:12 False  && withheld == 0
GAP prod BRANCH crates/api/src/bars_route.rs:1189:33 False  let status = if !faulted && withheld == 0 {
GAP test BRANCH crates/api/src/bars_route.rs:2030:29 False  *status == axum::http::StatusCode::INTERNAL_SERVER_ERROR
GAP test BRANCH crates/api/src/bars_route.rs:2031:36 False  && body.contains(sentence)
GAP test BRANCH crates/api/src/bars_route.rs:2032:36 False  && body.contains(MONTH_DAMAGED)
GAP test BRANCH crates/api/src/bars_route.rs:2312:12 False  if scan == axum::http::StatusCode::OK {
GAP test BRANCH crates/api/src/bars_route.rs:2329:12 False  if seek == axum::http::StatusCode::OK {
GAP test BRANCH crates/api/src/bars_route.rs:2337:16 False  if status == axum::http::StatusCode::OK {
GAP test BRANCH crates/api/src/bars_route.rs:2360:49 False  while !cancel.is_cancelled() && std::time::Instant::now() < deadline {
GAP test BRANCH crates/api/src/bars_route.rs:2365:16 True  if started_rx
GAP test BRANCH crates/api/src/bars_route.rs:2415:24 False  if let Ok(mut slot) = held.lock() {
GAP test BRANCH crates/api/src/bars_route.rs:2420:16 False  if answered.is_ok() {
GAP test BRANCH crates/api/src/bars_route.rs:2507:12 False  if indexed == axum::http::StatusCode::OK {
GAP test BRANCH crates/api/src/bars_route.rs:2536:16 False  if status == axum::http::StatusCode::OK {
GAP prod LINE crates/api/src/bars_series.rs:526  return self.bars;
GAP prod LINE crates/api/src/bars_series.rs:655  return Err(format!(
GAP prod LINE crates/api/src/bars_series.rs:657  summary.bars_on(day),
GAP prod LINE crates/api/src/bars_series.rs:659  if seconds { "seconds" } else { "minutes" }
GAP prod LINE crates/api/src/bars_series.rs:666  return Err(format!(
GAP prod LINE crates/api/src/bars_series.rs:667  "its summary's days hold {before} rows, the file {}",
GAP prod LINE crates/api/src/bars_series.rs:668  held.n_valid
GAP prod LINE crates/api/src/bars_series.rs:669  ));
GAP prod LINE crates/api/src/bars_series.rs:685  return 0;
GAP prod LINE crates/api/src/bars_series.rs:1215  return;
GAP prod LINE crates/api/src/bars_series.rs:1234  Err(why) => {
GAP prod LINE crates/api/src/bars_series.rs:1235  self.faults.push(format!("{month}: {why}"));
GAP prod LINE crates/api/src/bars_series.rs:1236  return;
GAP prod LINE crates/api/src/bars_series.rs:1273  continue;
GAP prod LINE crates/api/src/bars_series.rs:1329  continue;
GAP prod LINE crates/api/src/bars_series.rs:1514  }
GAP test LINE crates/api/src/bars_series.rs:2575  |_, _| { unreachable!("an empty stretch reads no row") }
GAP prod BRANCH crates/api/src/bars_series.rs:525:13 False  let Ok(local) = usize::try_from(local) else {
GAP prod BRANCH crates/api/src/bars_series.rs:549:20 False  if let Some(word) = usize::try_from(unit / 64 - first_word)
GAP prod BRANCH crates/api/src/bars_series.rs:600:22 True  let stamps = held.n_valid == 0
GAP prod BRANCH crates/api/src/bars_series.rs:631:24 False  if let (Some(slot), Ok(minute)) = (
GAP prod BRANCH crates/api/src/bars_series.rs:654:16 True  if entry.bars != u64::from(summary.bars_on(day)) {
GAP prod BRANCH crates/api/src/bars_series.rs:659:24 True  if seconds { "seconds" } else { "minutes" }
GAP prod BRANCH crates/api/src/bars_series.rs:659:24 False  if seconds { "seconds" } else { "minutes" }
GAP prod BRANCH crates/api/src/bars_series.rs:665:12 True  if before != held.n_valid {
GAP prod BRANCH crates/api/src/bars_series.rs:684:13 False  let Ok(at) = usize::try_from(day - self.first_day) else {
GAP prod BRANCH crates/api/src/bars_series.rs:875:16 False  if let Ok(journal) = crate::server::import_journal::snapshot(root)
GAP prod BRANCH crates/api/src/bars_series.rs:1214:13 False  let Ok(address) = StorePath::new(parts) else {
GAP prod BRANCH crates/api/src/bars_series.rs:1222:20 False  if entry.scope == store::quarantine::Scope::WholeMonth =>
GAP prod BRANCH crates/api/src/bars_series.rs:1272:17 False  let Some(at) = self.slots.get(slot) else {
GAP prod BRANCH crates/api/src/bars_series.rs:1328:17 False  let Some(at) = self.slots.get(slot) else {
GAP prod BRANCH crates/api/src/bars_series.rs:1507:16 False  if let Some(held) = self.slots.get(run.slot).map(|slot| slot.held) {
GAP prod LINE crates/api/src/bars_sort.rs:518  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1054  ends.push(None);
GAP prod LINE crates/api/src/bars_sort.rs:1055  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1064  return Ok(None);
GAP prod LINE crates/api/src/bars_sort.rs:1070  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1099  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1291  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1401  return Err("no valued row to order".to_owned());
GAP prod LINE crates/api/src/bars_sort.rs:1422  return Err(format!(
GAP prod LINE crates/api/src/bars_sort.rs:1423  "the value bisection found {vk} for row {k} and no row holds it"
GAP prod LINE crates/api/src/bars_sort.rs:1424  ));
GAP prod LINE crates/api/src/bars_sort.rs:1461  continue;
GAP prod LINE crates/api/src/bars_sort.rs:1568  }
GAP prod LINE crates/api/src/bars_sort.rs:1960  }
GAP prod LINE crates/api/src/bars_sort.rs:1998  return Err(Attempt::Refused(super::needs_scan_sentence(
GAP prod LINE crates/api/src/bars_sort.rs:1999  &format!("ordering by {}", column.as_str()),
GAP prod LINE crates/api/src/bars_sort.rs:2000  read_rows,
GAP prod LINE crates/api/src/bars_sort.rs:2001  budget,
GAP prod LINE crates/api/src/bars_sort.rs:2002  )));
GAP prod LINE crates/api/src/bars_sort.rs:2070  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2075  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2085  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2145  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2172  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2175  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2178  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2216  Some(None) => continue,
GAP prod LINE crates/api/src/bars_sort.rs:2220  || (Arc::from(""), Arc::from("")),
GAP prod LINE crates/api/src/bars_sort.rs:2275  return Err(Attempt::Refused(super::needs_scan_sentence(
GAP prod LINE crates/api/src/bars_sort.rs:2276  "the extremes",
GAP prod LINE crates/api/src/bars_sort.rs:2277  owed,
GAP prod LINE crates/api/src/bars_sort.rs:2278  budget,
GAP prod LINE crates/api/src/bars_sort.rs:2279  )));
GAP prod LINE crates/api/src/bars_sort.rs:2303  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2381  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2384  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2388  Err(why) => {
GAP prod LINE crates/api/src/bars_sort.rs:2389  faults.push_once(why);
GAP prod LINE crates/api/src/bars_sort.rs:2390  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2407  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2410  faults.push(format!("{}: slot {slot} has no kept run", plan.key.text()));
GAP prod LINE crates/api/src/bars_sort.rs:2411  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2456  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2531  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2549  return Err(super::needs_scan_sentence("the extremes", owed, budget));
GAP prod LINE crates/api/src/bars_sort.rs:2566  continue;
GAP test LINE crates/api/src/bars_sort.rs:4196  super::Attempt::Refused(why) | super::Attempt::Broken { why, .. } => why.clone(),
GAP test LINE crates/api/src/bars_sort.rs:4197  })
GAP prod BRANCH crates/api/src/bars_sort.rs:517:17 False  let Some(run) = plan.runs.get(run_at).copied() else {
GAP prod BRANCH crates/api/src/bars_sort.rs:582:32 False  if until > from {
GAP prod BRANCH crates/api/src/bars_sort.rs:1053:16 True  if len == 0 {
GAP prod BRANCH crates/api/src/bars_sort.rs:1063:12 True  if tlo > thi {
GAP prod BRANCH crates/api/src/bars_sort.rs:1069:21 False  let Some((first, last)) = *end else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1098:17 False  let Some((first, last)) = *end else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1290:17 False  let Some(run) = self.runs.get_mut(array) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1333:20 False  if let Some(run) = self.runs.get(array)
GAP prod BRANCH crates/api/src/bars_sort.rs:1347:16 False  if time.end > time.start {
GAP prod BRANCH crates/api/src/bars_sort.rs:1400:13 False  let Some((lo, hi)) = arrays.domain() else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1421:13 False  let Some((ts, series)) = arrays.select_in(root, plans, &slices, k - before)? else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1460:13 False  let Some(run) = arrays.runs.get_mut(array) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:1492:16 False  if let Some(run) = arrays.runs.get_mut(array)
GAP prod BRANCH crates/api/src/bars_sort.rs:1559:16 False  if let Some(Some((_, found))) = heads.get(at) {
GAP prod BRANCH crates/api/src/bars_sort.rs:1562:16 False  if let Some(cursor) = cursors.get_mut(at) {
GAP prod BRANCH crates/api/src/bars_sort.rs:1565:20 False  if let Some(slot) = heads.get_mut(at) {
GAP prod BRANCH crates/api/src/bars_sort.rs:1719:16 False  && let Some(key) = key
GAP prod BRANCH crates/api/src/bars_sort.rs:1898:28 False  if let Piece::Cut { entries, .. } = piece {
GAP prod BRANCH crates/api/src/bars_sort.rs:1946:28 False  if let Some(month) = month {
GAP prod BRANCH crates/api/src/bars_sort.rs:1997:12 True  if let Some(budget) = budget.filter(|&budget| read_rows > budget) {
GAP prod BRANCH crates/api/src/bars_sort.rs:2069:13 False  let Some(series_rules) = rules.get(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2074:13 False  let Ok(series) = u32::try_from(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2084:17 False  let Some(plan) = plans.get_mut(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2144:13 False  let Ok(series) = u32::try_from(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2151:20 False  && let Some(run) = plans
GAP prod BRANCH crates/api/src/bars_sort.rs:2171:17 False  let Some((found, stamp)) = views.get(&(cut.series, slot)).cloned() else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2174:17 False  let Ok(view) = found.as_ref() else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2177:17 False  let Some(info) = view.walked.runs.get(run).copied() else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2274:16 True  if let Some(budget) = budget.filter(|&budget| owed > budget) {
GAP prod BRANCH crates/api/src/bars_sort.rs:2302:25 False  let Some(plan) = plans.get_mut(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2315:28 False  if let Some(kept) = extremes.as_mut() {
GAP prod BRANCH crates/api/src/bars_sort.rs:2380:21 False  let Some(plan) = plans.get_mut(series as usize) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2383:21 False  let Some(slot) = plan.runs.get(run).map(|at| at.slot) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2406:21 False  let Some(plan) = plans.get_mut(series as usize) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2409:21 False  let Some(run) = run_of_slot(plan, slot) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2455:13 False  let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2515:28 False  if let Some(summary) = summary {
GAP prod BRANCH crates/api/src/bars_sort.rs:2530:17 False  let Some(plan) = plans.get(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2548:12 True  if let Some(budget) = budget.filter(|&budget| owed > budget) {
GAP prod BRANCH crates/api/src/bars_sort.rs:2565:13 False  let Some(plan) = plans.get_mut(series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2593:20 False  && let Some(at) = plan.slots.get(slot)
GAP test BRANCH crates/api/src/bars_sort.rs:3013:28 False  [inside a closure whose false side does not fail the check]  .any(|why| why.contains("2031-01") && why.contains("failed when it was read")),
GAP test BRANCH crates/api/src/bars_sort.rs:4327:43 False  [inside a closure whose false side does not fail the check]  got.faults.iter().any(|fault| fault.contains(&format!(
GAP test LINE crates/api/src/bars_sort_tests.rs:1083  || crate::emitted::says(line, "symbol", "HOLETEST")
GAP test LINE crates/api/src/bars_sort_tests.rs:1508  assert_eq!(
GAP test LINE crates/api/src/bars_sort_tests.rs:1509  own.get(&row.bar.ts_micros),
GAP test LINE crates/api/src/bars_sort_tests.rs:1510  Some(&row.bar),
GAP test BRANCH crates/api/src/bars_sort_tests.rs:499:13 False  at >= tod.0 && at <= tod.1
GAP test BRANCH crates/api/src/bars_sort_tests.rs:955:54 True  if better || (now.0 == held.0 && (now.1, now.2) < (held.1, held.2)) {
GAP test BRANCH crates/api/src/bars_sort_tests.rs:1082:17 False  crate::emitted::says(line, "key", "HOLETEST")
GAP test LINE crates/api/src/bars_unknown_series_tests.rs:457  assert_eq!(answer.expect("held").total, 2);
GAP test BRANCH crates/api/src/bars_unknown_series_tests.rs:455:8 True  if listable {
GAP test LINE crates/api/src/bars_window_route_tests.rs:110  }
GAP test LINE crates/api/src/bars_window_route_tests.rs:111  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test LINE crates/api/src/bars_window_route_tests.rs:125  }
GAP test LINE crates/api/src/bars_window_route_tests.rs:126  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test LINE crates/api/src/bars_window_route_tests.rs:2103  [read: the second operand of an || in the condition, never reached: not the failing arm]  || pull::gdfl_lineage::is_contract_refused(note)),
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:108:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:123:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:1640:12 False  [read: an if that picks a value inside the compared expression: not a check]  if truth[0].1 == 0 { CE } else { PE }
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:2102:29 True  .any(|note| note.contains("gdfl-rekey")
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:2102:29 False  .any(|note| note.contains("gdfl-rekey")
GAP test BRANCH crates/api/src/bars_window_route_tests.rs:2110:34 False  [inside a closure whose false side does not fail the check]  copied.iter().any(|note| note.starts_with(&format!(
GAP test LINE crates/api/src/boolean_observation_budget.rs:90  return Err(format!("invalid budget accepted: {value}"));
GAP prod BRANCH crates/api/src/boolean_observation_budget.rs:27:16 True  || value.len() > 20
GAP test BRANCH crates/api/src/boolean_observation_budget.rs:89:17 False  let Err(why) = BooleanObservationBudget::from_value(Some(OsStr::new(value))) else {
GAP prod BRANCH crates/api/src/boolean_search_budget.rs:20:16 True  || value.len() > 20
GAP prod LINE crates/api/src/booleanadmission_projection.rs:14  return Err("saved admission verdict partitions do not reconcile".to_owned());
GAP prod BRANCH crates/api/src/booleanadmission_projection.rs:13:8 True  if !verdict.reconciles() {
GAP prod LINE crates/api/src/booleancampaignjson.rs:71  Ok(body) => {
GAP prod LINE crates/api/src/booleancampaignjson.rs:72  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/booleancampaignjson.rs:73  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleancampaignjson.rs:74  reply
GAP prod LINE crates/api/src/booleancampaignjson.rs:76  refused(
GAP prod LINE crates/api/src/booleancampaignjson.rs:78  "campaign comparison exceeds response byte ceiling; no partial rows returned",
GAP prod LINE crates/api/src/booleancampaignjson.rs:107  "completed"
GAP prod LINE crates/api/src/booleancampaignjson.rs:109  "refused"
GAP prod LINE crates/api/src/booleancampaignjson.rs:111  "running"
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:73:16 True  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:73:16 False  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:105:162 True  let rows:Vec<_>=selected.iter().map(|(index,(slot,rung))|json!({"rung":rung,"rung_index":index.to_string(),"unit":crate::server::hex32(slot.
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:105:206 True  let rows:Vec<_>=selected.iter().map(|(index,(slot,rung))|json!({"rung":rung,"rung_index":index.to_string(),"unit":crate::server::hex32(slot.
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:105:248 True  let rows:Vec<_>=selected.iter().map(|(index,(slot,rung))|json!({"rung":rung,"rung_index":index.to_string(),"unit":crate::server::hex32(slot.
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:106:20 True  let state = if selected.iter().all(|(_, (s, _))| s.complete.is_some()) {
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:108:15 True  } else if selected.iter().any(|(_, (s, _))| !s.reason.is_empty()) {
GAP prod BRANCH crates/api/src/booleancampaignjson.rs:110:15 True  } else if selected.iter().any(|(_, (s, _))| s.started) {
GAP prod LINE crates/api/src/booleanevidencejson.rs:147  pub(crate) fn qualification_observation(
GAP prod LINE crates/api/src/booleanevidencejson.rs:148  reader: &Qualification,
GAP prod LINE crates/api/src/booleanevidencejson.rs:149  completion: [u8; 32],
GAP prod LINE crates/api/src/booleanevidencejson.rs:150  offset: usize,
GAP prod LINE crates/api/src/booleanevidencejson.rs:151  limit: usize,
GAP prod LINE crates/api/src/booleanevidencejson.rs:152  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:153  qualification_projection::project(
GAP prod LINE crates/api/src/booleanevidencejson.rs:154  reader,
GAP prod LINE crates/api/src/booleanevidencejson.rs:155  &Asked {
GAP prod LINE crates/api/src/booleanevidencejson.rs:156  model: Model::Qualification,
GAP prod LINE crates/api/src/booleanevidencejson.rs:157  identity: reader.identity(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:158  completion: Some(completion),
GAP prod LINE crates/api/src/booleanevidencejson.rs:159  kind: "candidates".into(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:160  offset,
GAP prod LINE crates/api/src/booleanevidencejson.rs:161  limit,
GAP prod LINE crates/api/src/booleanevidencejson.rs:162  setting: None,
GAP prod LINE crates/api/src/booleanevidencejson.rs:163  period: "full".into(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:164  },
GAP prod LINE crates/api/src/booleanevidencejson.rs:166  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:169  pub(crate) fn search_comparison(
GAP prod LINE crates/api/src/booleanevidencejson.rs:170  reader: &Qualification,
GAP prod LINE crates/api/src/booleanevidencejson.rs:171  index: usize,
GAP prod LINE crates/api/src/booleanevidencejson.rs:172  row: &cli::boolean_evidence::SearchRow,
GAP prod LINE crates/api/src/booleanevidencejson.rs:173  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:174  let mut result = admission_projection::row(
GAP prod LINE crates/api/src/booleanevidencejson.rs:175  index,
GAP prod LINE crates/api/src/booleanevidencejson.rs:176  &cli::boolean_evidence::AdmissionRow {
GAP prod LINE crates/api/src/booleanevidencejson.rs:177  identity: row.source.original,
GAP prod LINE crates/api/src/booleanevidencejson.rs:178  source_index: index,
GAP prod LINE crates/api/src/booleanevidencejson.rs:179  values: row.projection.values(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:180  verdict: row.projection.verdict(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:181  },
GAP prod LINE crates/api/src/booleanevidencejson.rs:182  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:183  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:184  &mut result,
GAP prod LINE crates/api/src/booleanevidencejson.rs:185  "policy_digest",
GAP prod LINE crates/api/src/booleanevidencejson.rs:186  json!(crate::server::hex32(row.projection.policy_digest())),
GAP prod LINE crates/api/src/booleanevidencejson.rs:187  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:188  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:189  &mut result,
GAP prod LINE crates/api/src/booleanevidencejson.rs:190  "index_consistency",
GAP prod LINE crates/api/src/booleanevidencejson.rs:191  index_consistency_projection::setting(reader, index, row.projection.verdict().status())?,
GAP prod LINE crates/api/src/booleanevidencejson.rs:192  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:193  Ok(result)
GAP prod LINE crates/api/src/booleanevidencejson.rs:194  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:207  Ok(body) => {
GAP prod LINE crates/api/src/booleanevidencejson.rs:208  let response = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/booleanevidencejson.rs:209  if response.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleanevidencejson.rs:210  response
GAP prod LINE crates/api/src/booleanevidencejson.rs:212  refused(
GAP prod LINE crates/api/src/booleanevidencejson.rs:214  "Boolean evidence response exceeds its byte ceiling; no prefix returned",
GAP prod LINE crates/api/src/booleanevidencejson.rs:238  fn require_current(&self) -> Result<(), String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:239  match self {
GAP prod LINE crates/api/src/booleanevidencejson.rs:240  Self::Statistics(reader) => reader.require_current(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:241  Self::Admission(reader) => reader.require_current(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:242  Self::Qualification(reader) => reader.require_current(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:244  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:300  held.is_some_and(|held| held.reader.require_current().is_ok())
GAP prod LINE crates/api/src/booleanevidencejson.rs:301  }) {
GAP prod LINE crates/api/src/booleanevidencejson.rs:308  *cache = Some(Cached { key, reader });
GAP prod LINE crates/api/src/booleanevidencejson.rs:309  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:310  let reader = &cache
GAP prod LINE crates/api/src/booleanevidencejson.rs:311  .as_ref()
GAP prod LINE crates/api/src/booleanevidencejson.rs:312  .ok_or("Boolean evidence cache disappeared")?
GAP prod LINE crates/api/src/booleanevidencejson.rs:314  let mut body = match reader {
GAP prod LINE crates/api/src/booleanevidencejson.rs:315  Reader::Statistics(reader) => statistics(reader, asked),
GAP prod LINE crates/api/src/booleanevidencejson.rs:316  Reader::Admission(reader) => admission(reader, asked),
GAP prod LINE crates/api/src/booleanevidencejson.rs:317  Reader::Qualification(reader) => qualification_projection::project(reader, asked),
GAP prod LINE crates/api/src/booleanevidencejson.rs:318  }?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:319  crate::detail::put_equity_note(&mut body, equity_note(reader))?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:320  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:321  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:322  "observation_byte_limit",
GAP prod LINE crates/api/src/booleanevidencejson.rs:323  json!(budget.bytes().to_string()),
GAP prod LINE crates/api/src/booleanevidencejson.rs:324  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:325  Ok(body)
GAP prod LINE crates/api/src/booleanevidencejson.rs:339  fn equity_note(reader: &Reader) -> String {
GAP prod LINE crates/api/src/booleanevidencejson.rs:340  sources_note(match reader {
GAP prod LINE crates/api/src/booleanevidencejson.rs:341  Reader::Statistics(reader) => reader.sources(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:342  Reader::Admission(reader) => reader.statistics().sources(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:343  Reader::Qualification(reader) => reader.original().statistics().sources(),
GAP prod LINE crates/api/src/booleanevidencejson.rs:345  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:372  fn statistics(reader: &Statistics, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:373  reader.require_current()?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:374  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/booleanevidencejson.rs:375  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod LINE crates/api/src/booleanevidencejson.rs:376  return Err("statistics completion pin differs".to_owned());
GAP prod LINE crates/api/src/booleanevidencejson.rs:377  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:378  let (total,rows)=match asked.kind.as_str() {
GAP prod LINE crates/api/src/booleanevidencejson.rs:379  "candidates"=>(reader.summary().candidates,reader.rows(pin,asked.offset,asked.limit)?.iter().enumerate().map(|(n,row)|statistics_row(reader,
GAP prod LINE crates/api/src/booleanevidencejson.rs:380  "splits"=>(reader.summary().splits,reader.splits(pin,asked.offset,asked.limit)?.iter().enumerate().map(|(n,row)|json!({"index":(asked.offset
GAP prod LINE crates/api/src/booleanevidencejson.rs:381  "sources"=>{let sources=reader.sources();let end=asked.offset.checked_add(asked.limit).ok_or("source page overflow")?.min(sources.len());let
GAP prod LINE crates/api/src/booleanevidencejson.rs:382  _=>return Err("unknown statistics page kind".to_owned()),
GAP prod LINE crates/api/src/booleanevidencejson.rs:384  let mut body = base(asked, pin, total, rows)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:385  put(&mut body, "summary", summary(reader.summary())?)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:386  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:387  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:388  "admitted_bytes",
GAP prod LINE crates/api/src/booleanevidencejson.rs:389  json!(reader.admitted_bytes().to_string()),
GAP prod LINE crates/api/src/booleanevidencejson.rs:390  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:391  reader.require_current()?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:392  Ok(body)
GAP prod LINE crates/api/src/booleanevidencejson.rs:393  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:394  fn admission(reader: &Admission, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:395  reader.require_current()?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:396  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/booleanevidencejson.rs:397  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod LINE crates/api/src/booleanevidencejson.rs:398  return Err("admission completion pin differs".to_owned());
GAP prod LINE crates/api/src/booleanevidencejson.rs:399  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:400  let rows = reader.rows(pin, asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:401  let stats = reader.statistics();
GAP prod LINE crates/api/src/booleanevidencejson.rs:402  let observations = stats.rows(stats.completion_digest(), asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:403  if observations.len() != rows.len() {
GAP prod LINE crates/api/src/booleanevidencejson.rs:404  return Err("admission page differs from linked statistics extent".to_owned());
GAP prod LINE crates/api/src/booleanevidencejson.rs:405  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:406  let mut rendered = Vec::new();
GAP prod LINE crates/api/src/booleanevidencejson.rs:407  for (n, (row, observation)) in rows.iter().zip(&observations).enumerate() {
GAP prod LINE crates/api/src/booleanevidencejson.rs:408  let index = asked.offset + n;
GAP prod LINE crates/api/src/booleanevidencejson.rs:409  if row.source_index != index || row.identity != observation.identity {
GAP prod LINE crates/api/src/booleanevidencejson.rs:410  return Err("admission row differs from linked candidate statistics".to_owned());
GAP prod LINE crates/api/src/booleanevidencejson.rs:411  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:412  let mut value = admission_projection::row(index, row)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:413  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:414  &mut value,
GAP prod LINE crates/api/src/booleanevidencejson.rs:415  "statistics",
GAP prod LINE crates/api/src/booleanevidencejson.rs:416  statistics_row(stats, index, observation)?,
GAP prod LINE crates/api/src/booleanevidencejson.rs:417  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:418  rendered.push(value);
GAP prod LINE crates/api/src/booleanevidencejson.rs:420  let mut body = base(asked, pin, reader.row_count(), rendered)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:421  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:422  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:423  "statistics_identity",
GAP prod LINE crates/api/src/booleanevidencejson.rs:424  json!(crate::server::hex32(stats.identity())),
GAP prod LINE crates/api/src/booleanevidencejson.rs:425  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:426  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:427  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:428  "statistics_completion",
GAP prod LINE crates/api/src/booleanevidencejson.rs:429  json!(crate::server::hex32(stats.completion_digest())),
GAP prod LINE crates/api/src/booleanevidencejson.rs:430  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:431  put(&mut body, "summary", summary(stats.summary())?)?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:432  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:433  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:434  "policy",
GAP prod LINE crates/api/src/booleanevidencejson.rs:435  admission_projection::policy(&reader.policy()),
GAP prod LINE crates/api/src/booleanevidencejson.rs:436  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:437  put(
GAP prod LINE crates/api/src/booleanevidencejson.rs:438  &mut body,
GAP prod LINE crates/api/src/booleanevidencejson.rs:439  "admitted_bytes",
GAP prod LINE crates/api/src/booleanevidencejson.rs:440  json!(
GAP prod LINE crates/api/src/booleanevidencejson.rs:441  reader
GAP prod LINE crates/api/src/booleanevidencejson.rs:442  .body_bytes()
GAP prod LINE crates/api/src/booleanevidencejson.rs:443  .checked_add(112)
GAP prod LINE crates/api/src/booleanevidencejson.rs:444  .and_then(|bytes| bytes.checked_add(stats.admitted_bytes()))
GAP prod LINE crates/api/src/booleanevidencejson.rs:445  .ok_or("admission observed-byte count overflow")?
GAP prod LINE crates/api/src/booleanevidencejson.rs:446  .to_string()
GAP prod LINE crates/api/src/booleanevidencejson.rs:448  )?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:449  reader.require_current()?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:450  Ok(body)
GAP prod LINE crates/api/src/booleanevidencejson.rs:451  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:452  fn source(index: usize, row: &StatisticsSource) -> Value {
GAP prod LINE crates/api/src/booleanevidencejson.rs:453  json!({"index":index.to_string(),"instrument":row.family.instrument().to_string(),"cash":row.family.is_cash(),"membership_digest":crate::ser
GAP prod LINE crates/api/src/booleanevidencejson.rs:454  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:455  fn statistics_row(reader: &Statistics, index: usize, row: &StatisticsRow) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:456  let linked = reader
GAP prod LINE crates/api/src/booleanevidencejson.rs:457  .sources()
GAP prod LINE crates/api/src/booleanevidencejson.rs:458  .get(row.source)
GAP prod LINE crates/api/src/booleanevidencejson.rs:459  .ok_or("statistics source link missing")?;
GAP prod LINE crates/api/src/booleanevidencejson.rs:460  let romano=row.romano.map(|[strategy,rank,statistic,exceedances,initial_n,initial_d,adjusted_n,adjusted_d]|json!({"strategy":strategy.to_str
GAP prod LINE crates/api/src/booleanevidencejson.rs:462  json!({"index":index.to_string(),"identity":crate::server::hex32(row.identity),"run":crate::server::hex32(row.run),"source":source(row.sourc
GAP prod LINE crates/api/src/booleanevidencejson.rs:464  }
GAP prod LINE crates/api/src/booleanevidencejson.rs:467  1 => Ok("measured"),
GAP prod LINE crates/api/src/booleanevidencejson.rs:491  fn summary(summary: &StatisticsSummary) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanevidencejson.rs:492  let [draws, seed, block_length] = summary.procedure;
GAP prod LINE crates/api/src/booleanevidencejson.rs:494  families,
GAP prod LINE crates/api/src/booleanevidencejson.rs:495  candidates,
GAP prod LINE crates/api/src/booleanevidencejson.rs:496  observations,
GAP prod LINE crates/api/src/booleanevidencejson.rs:497  bootstrap_work,
GAP prod LINE crates/api/src/booleanevidencejson.rs:498  split_work,
GAP prod LINE crates/api/src/booleanevidencejson.rs:499  memory_bytes,
GAP prod LINE crates/api/src/booleanevidencejson.rs:500  bytes,
GAP prod LINE crates/api/src/booleanevidencejson.rs:501  ] = summary.bounds;
GAP prod LINE crates/api/src/booleanevidencejson.rs:502  let [white, spa, romano] = summary.test_families;
GAP prod LINE crates/api/src/booleanevidencejson.rs:504  json!({"scope":crate::server::hex32(summary.scope),"cohort":crate::server::hex32(summary.cohort),"layout":crate::server::hex32(summary.layou
GAP prod LINE crates/api/src/booleanevidencejson.rs:506  }
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:209:16 True  if response.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:209:16 False  if response.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:299:8 False  if crate::detail::must_admit(held.is_some(), asked.completion.is_some(), || {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:375:8 True  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:375:8 False  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:397:8 True  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:397:8 False  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:403:8 True  if observations.len() != rows.len() {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:403:8 False  if observations.len() != rows.len() {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:409:12 True  if row.source_index != index || row.identity != observation.identity {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:409:12 False  if row.source_index != index || row.identity != observation.identity {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:409:41 True  if row.source_index != index || row.identity != observation.identity {
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:409:41 False  if row.source_index != index || row.identity != observation.identity {
GAP test LINE crates/api/src/booleanevidencejson_tests.rs:336  false
GAP test LINE crates/api/src/booleanevidencejson_tests.rs:421  admitted.push(kind);
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:241:27 False  while let Some(inner) = chars.next() {
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:256:23 False  while let Some(inner) = chars.next() {
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:275:31 False  } else if c == '/' && chars.peek() == Some(&'*') {
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:279:23 False  while let Some(inner) = chars.next() {
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:285:43 False  } else if inner == '*' && chars.peek() == Some(&'/') {
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:313:23 False  let keyword = !part_of_a_word(at.checked_sub(1).and_then(|before| bytes.get(before)))
GAP test BRANCH crates/api/src/booleanevidencejson_tests.rs:420:12 True  if on_every_page(&body.replacen(call, &replacement, 1), call) {
GAP prod LINE crates/api/src/booleanjson.rs:68  _ => return Err("unknown catalog page kind".to_owned()),
GAP prod LINE crates/api/src/booleanjson.rs:84  _ => return Err("side must be long or short".to_owned()),
GAP prod LINE crates/api/src/booleanjson.rs:90  _ => return Err("unknown saved grid axis".to_owned()),
GAP prod LINE crates/api/src/booleanjson.rs:136  Ok(body) => {
GAP prod LINE crates/api/src/booleanjson.rs:137  let result = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/booleanjson.rs:138  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleanjson.rs:139  result
GAP prod LINE crates/api/src/booleanjson.rs:141  refused(
GAP prod LINE crates/api/src/booleanjson.rs:143  "catalog response exceeds byte admission; no prefix returned",
GAP prod LINE crates/api/src/booleanjson.rs:286  _ => return Err("unsupported catalog page".to_owned()),
GAP prod LINE crates/api/src/booleanjson.rs:364  values
GAP prod LINE crates/api/src/booleanjson.rs:365  .get(n)
GAP prod LINE crates/api/src/booleanjson.rs:366  .map(ToString::to_string)
GAP prod LINE crates/api/src/booleanjson.rs:367  .ok_or("coordinate ladder index missing".to_owned())
GAP prod LINE crates/api/src/booleanjson.rs:368  })
GAP prod LINE crates/api/src/booleanjson.rs:380  ForcedStopV1::IncludeExactObserved(ppm) => {
GAP prod LINE crates/api/src/booleanjson.rs:381  json!({"kind":"include-exact-observed","ppm":ppm.to_string()})
GAP prod LINE crates/api/src/booleanjson.rs:383  ForcedStopV1::RequireExactObserved(ppm) => {
GAP prod LINE crates/api/src/booleanjson.rs:384  json!({"kind":"require-exact-observed","ppm":ppm.to_string()})
GAP prod LINE crates/api/src/booleanjson.rs:407  "trail" => grid.policy.rungs().trail(),
GAP prod LINE crates/api/src/booleanjson.rs:408  _ => return Err("saved requested axis missing".to_owned()),
GAP prod LINE crates/api/src/booleanjson.rs:414  "target" => &grid.targets,
GAP prod LINE crates/api/src/booleanjson.rs:415  "trail" => &grid.trails,
GAP prod LINE crates/api/src/booleanjson.rs:416  _ => return Err("saved resolved axis missing".to_owned()),
GAP prod BRANCH crates/api/src/booleanjson.rs:94:36 True  || (kind == "grid" && (side.is_none() || axis.is_none()))
GAP prod BRANCH crates/api/src/booleanjson.rs:94:54 True  || (kind == "grid" && (side.is_none() || axis.is_none()))
GAP prod BRANCH crates/api/src/booleanjson.rs:95:54 True  || (kind != "grid" && (side.is_some() || axis.is_some()))
GAP prod BRANCH crates/api/src/booleanjson.rs:138:16 True  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanjson.rs:138:16 False  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanjson.rs:194:30 False  held.root == root && held.identity == asked.identity && held.budget == budget
GAP prod LINE crates/api/src/booleanlaunch.rs:164  }
GAP prod LINE crates/api/src/booleanlaunch.rs:224  )?;
GAP prod LINE crates/api/src/booleanlaunch.rs:242  return Err("Boolean dashboard observation/replay limits cannot read the admitted search; configure BRUTEX_BOOLEAN_OBSERVATION_BYTES and BRUT
GAP prod LINE crates/api/src/booleanlaunch.rs:277  status.observe(next)?;
GAP prod LINE crates/api/src/booleanlaunch.rs:278  let mut held = site
GAP prod LINE crates/api/src/booleanlaunch.rs:279  .sweep
GAP prod LINE crates/api/src/booleanlaunch.rs:280  .lock()
GAP prod LINE crates/api/src/booleanlaunch.rs:281  .map_err(|_| "Boolean status lock is poisoned")?;
GAP prod LINE crates/api/src/booleanlaunch.rs:282  let target = held
GAP prod LINE crates/api/src/booleanlaunch.rs:283  .as_mut()
GAP prod LINE crates/api/src/booleanlaunch.rs:284  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod LINE crates/api/src/booleanlaunch.rs:285  .ok_or(
GAP prod LINE crates/api/src/booleanlaunch.rs:287  )?;
GAP prod LINE crates/api/src/booleanlaunch.rs:288  target.boolean_search = Some(Box::new(status.clone()));
GAP prod LINE crates/api/src/booleanlaunch.rs:289  Ok(())
GAP prod LINE crates/api/src/booleanlaunch.rs:290  })
GAP prod LINE crates/api/src/booleanlaunch.rs:295  Ok(report) => progress.report = Some(report.into()),
GAP prod LINE crates/api/src/booleanlaunch.rs:314  Err(why) => response(
GAP prod LINE crates/api/src/booleanlaunch.rs:316  &json!({"schema_version":1,
GAP prod LINE crates/api/src/booleanlaunch.rs:317  "model":"boolean-qualified-search-launch","ready":false,"refusal":format!("bounded configuration read unavailable: {why:?}")}),
GAP prod LINE crates/api/src/booleanlaunch.rs:367  Err(why) => {
GAP prod LINE crates/api/src/booleanlaunch.rs:368  note_metadata_refusal(&mut failures, why);
GAP prod LINE crates/api/src/booleanlaunch.rs:369  None
GAP prod LINE crates/api/src/booleanlaunch.rs:408  Err(why) => {
GAP prod LINE crates/api/src/booleanlaunch.rs:409  note_metadata_refusal(&mut failures, why);
GAP prod LINE crates/api/src/booleanlaunch.rs:410  None
GAP prod LINE crates/api/src/booleanlaunch.rs:422  {
GAP prod LINE crates/api/src/booleanlaunch.rs:423  note_metadata_refusal(&mut failures, why);
GAP prod LINE crates/api/src/booleanlaunch.rs:426  note_metadata_refusal(
GAP prod LINE crates/api/src/booleanlaunch.rs:427  &mut failures,
GAP prod LINE crates/api/src/booleanlaunch.rs:428  "The serving store is not an existing absolute directory".into(),
GAP prod LINE crates/api/src/booleanlaunch.rs:429  );
GAP prod BRANCH crates/api/src/booleanlaunch.rs:90:48 False  || self.exhausted == Some(true) && next.exhausted != Some(true)
GAP prod BRANCH crates/api/src/booleanlaunch.rs:109:8 True  if asked.command != COMMAND
GAP prod BRANCH crates/api/src/booleanlaunch.rs:111:12 True  || asked.symbols.len() > cli::boolean_search_launch::max_symbols()
GAP prod BRANCH crates/api/src/booleanlaunch.rs:113:13 True  symbol.is_empty()
GAP prod BRANCH crates/api/src/booleanlaunch.rs:150:8 False  if asked.bits != "all" {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:152:12 True  if asked.bits.len() > 4096 || asked.bits.is_empty() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:241:8 True  if observation < needed || replay < strict.max_records() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:241:32 True  if observation < needed || replay < strict.max_records() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:284:31 True  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod BRANCH crates/api/src/booleanlaunch.rs:284:31 False  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod BRANCH crates/api/src/booleanlaunch.rs:421:16 True  && let Err(why) = observation_limits(strict)
GAP prod BRANCH crates/api/src/booleanlaunch.rs:425:8 True  if !root.is_absolute() || !root.is_dir() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:425:31 True  if !root.is_absolute() || !root.is_dir() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:450:12 False  if let Some(fields) = policy.as_object_mut() {
GAP prod BRANCH crates/api/src/booleanlaunch.rs:471:8 True  if raw.is_empty()
GAP prod BRANCH crates/api/src/booleanlaunch.rs:472:12 True  || raw.len() > 20
GAP test LINE crates/api/src/booleanlaunch_tests.rs:463  child.kill().unwrap();
GAP test LINE crates/api/src/booleanlaunch_tests.rs:464  child.wait().unwrap();
GAP test LINE crates/api/src/booleanlaunch_tests.rs:465  return Err("private configuration child exceeded 45s/1MiB".into());
GAP test BRANCH crates/api/src/booleanlaunch_tests.rs:439:12 False  if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
GAP test BRANCH crates/api/src/booleanlaunch_tests.rs:460:12 True  if start.elapsed() > std::time::Duration::from_secs(45)
GAP test BRANCH crates/api/src/booleanlaunch_tests.rs:461:16 True  || std::fs::metadata(&log).unwrap().len() > 1024 * 1024
GAP prod LINE crates/api/src/booleanlaunch_work.rs:19  Err(why) => json!({"version":1,"state":"refused","refusal":why}),
GAP prod LINE crates/api/src/booleanoosjson.rs:101  Ok(body) => {
GAP prod LINE crates/api/src/booleanoosjson.rs:102  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/booleanoosjson.rs:103  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleanoosjson.rs:104  reply
GAP prod LINE crates/api/src/booleanoosjson.rs:106  refused(
GAP prod LINE crates/api/src/booleanoosjson.rs:108  "later comparison response exceeds byte ceiling; no prefix returned",
GAP prod LINE crates/api/src/booleanoosjson.rs:160  }
GAP prod LINE crates/api/src/booleanoosjson.rs:167  )?;
GAP prod LINE crates/api/src/booleanoosjson.rs:245  _=>Err("unsupported later comparison page".into()),
GAP prod BRANCH crates/api/src/booleanoosjson.rs:103:16 True  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanoosjson.rs:103:16 False  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanoosjson.rs:148:12 False  || !cache.as_ref().is_some_and(|held| {
GAP prod BRANCH crates/api/src/booleanoosjson.rs:149:13 False  held.root == root && held.identity == asked.identity && held.budget == budget
GAP prod BRANCH crates/api/src/booleanoosjson.rs:149:34 False  held.root == root && held.identity == asked.identity && held.budget == budget
GAP prod BRANCH crates/api/src/booleanoosjson.rs:235:16 True  if later.len()!=original.len(){return Err("later/original page cardinality differs".into());}
GAP prod LINE crates/api/src/booleanqualification_projection.rs:9  pub(super) fn project(reader: &Qualification, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:10  reader.require_current()?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:11  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:12  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:13  return Err("qualification completion pin differs; no replacement page returned".into());
GAP prod LINE crates/api/src/booleanqualification_projection.rs:14  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:15  if matches!(asked.kind.as_str(), "index-weeks" | "index-days") {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:16  return super::index_consistency_projection::page(reader, asked);
GAP prod LINE crates/api/src/booleanqualification_projection.rs:17  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:18  let rows = reader.rows(pin, asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:19  let stats = reader.original().statistics();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:20  let observations = stats.rows(stats.completion_digest(), asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:21  if rows.len() != observations.len() {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:22  return Err("qualification page differs from original statistics extent".into());
GAP prod LINE crates/api/src/booleanqualification_projection.rs:23  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:24  let mut rendered = Vec::new();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:25  for (n, (row, original)) in rows.iter().zip(&observations).enumerate() {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:26  let index = asked.offset + n;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:27  same_coordinate(row.original, row.family, row.coordinate, original)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:28  let mut value = admission_projection::row(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:29  index,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:30  &AdmissionRow {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:31  identity: row.original,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:32  source_index: index,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:33  values: row.values,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:34  verdict: row.verdict,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:35  },
GAP prod LINE crates/api/src/booleanqualification_projection.rs:36  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:37  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:38  &mut value,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:39  "statistics",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:40  statistics_row(stats, index, original)?,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:41  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:42  put(&mut value, "qualification", row_detail(reader, row)?)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:43  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:44  &mut value,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:45  "index_consistency",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:46  super::index_consistency_projection::setting(reader, index, row.verdict.status())?,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:47  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:48  rendered.push(value);
GAP prod LINE crates/api/src/booleanqualification_projection.rs:50  let mut body = base(asked, pin, reader.row_count(), rendered)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:51  put(&mut body, "summary", summary(stats.summary())?)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:52  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:53  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:54  "statistics_identity",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:55  json!(crate::server::hex32(stats.identity())),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:56  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:57  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:58  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:59  "statistics_completion",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:60  json!(crate::server::hex32(stats.completion_digest())),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:61  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:62  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:63  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:64  "policy",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:65  admission_projection::policy(&reader.policy()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:66  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:67  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:68  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:69  "admitted_bytes",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:70  json!(reader.admitted_bytes().to_string()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:71  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:72  let allocation = reader.allocation();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:73  let threshold = allocation.threshold();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:74  let minimum = allocation.minimum_draws();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:75  let [draws, seed, block_length] = reader.procedure();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:76  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:77  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:78  "qualification",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:79  json!({
GAP prod LINE crates/api/src/booleanqualification_projection.rs:80  "scope":crate::server::hex32(reader.scope()),"unit":crate::server::hex32(reader.unit()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:81  "original":{"identity":crate::server::hex32(reader.original().identity()),"completion":crate::server::hex32(reader.original().completion_dig
GAP prod LINE crates/api/src/booleanqualification_projection.rs:82  "fold_count":reader.fold_count().to_string(),"procedure":{"draws":draws.to_string(),"seed":seed.to_string(),"block_length":block_length.to_s
GAP prod LINE crates/api/src/booleanqualification_projection.rs:83  "allocation":{"digest":crate::server::hex32(allocation.digest()),"batch":allocation.batch().to_string(),"batches":allocation.batches().to_st
GAP prod LINE crates/api/src/booleanqualification_projection.rs:84  "later":reader.later().iter().enumerate().map(|(index,later)|json!({"index":index.to_string(),"identity":crate::server::hex32(later.identity
GAP prod LINE crates/api/src/booleanqualification_projection.rs:86  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:87  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:88  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:89  "scope",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:90  json!(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:93  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:94  reader.require_current()?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:95  Ok(body)
GAP prod LINE crates/api/src/booleanqualification_projection.rs:96  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:119  fn row_detail(reader: &Qualification, row: &QualificationRow) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:120  let later = reader
GAP prod LINE crates/api/src/booleanqualification_projection.rs:121  .later()
GAP prod LINE crates/api/src/booleanqualification_projection.rs:122  .get(row.family)
GAP prod LINE crates/api/src/booleanqualification_projection.rs:123  .ok_or("qualification later family missing")?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:124  let folds = if let Some(folds) = &row.folds {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:125  Some(json!({
GAP prod LINE crates/api/src/booleanqualification_projection.rs:126  "plan":crate::server::hex32(folds.plan_digest()),"selected":crate::server::hex32(folds.selected_digest()),"anchor":crate::server::hex32(fold
GAP prod LINE crates/api/src/booleanqualification_projection.rs:127  "later":crate::server::hex32(folds.later_digest()),"training_run":crate::server::hex32(folds.training_run_id()),"later_run":crate::server::h
GAP prod LINE crates/api/src/booleanqualification_projection.rs:128  "ordinal":folds.ordinal().to_string(),"training_last_day":folds.training_last_day().to_string(),"first_day":folds.requested().first_day().to
GAP prod LINE crates/api/src/booleanqualification_projection.rs:129  "decided":folds.decided_folds().to_string(),"profitable":folds.profitable_oos_folds().to_string(),"return_paisa":folds.aggregate_oos_paisa()
GAP prod LINE crates/api/src/booleanqualification_projection.rs:130  "rows":folds.folds().iter().enumerate().map(|(index,fold)|json!({"index":index.to_string(),"first_day":fold.window.first_day().to_string(),"
GAP prod LINE crates/api/src/booleanqualification_projection.rs:133  None
GAP prod LINE crates/api/src/booleanqualification_projection.rs:136  json!({"identity":crate::server::hex32(row.later),"family":row.family.to_string(),"coordinate":row.coordinate.to_string(),"source":{"identit
GAP prod LINE crates/api/src/booleanqualification_projection.rs:138  }
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:12:8 True  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:12:8 False  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:21:8 True  if rows.len() != observations.len() {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:21:8 False  if rows.len() != observations.len() {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:124:24 True  let folds = if let Some(folds) = &row.folds {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:124:24 False  let folds = if let Some(folds) = &row.folds {
GAP prod LINE crates/api/src/booleansearchjson.rs:72  return Err("search page limit must be 1..=256".into());
GAP prod LINE crates/api/src/booleansearchjson.rs:109  render(&root, &asked, budget.bytes(), replay.nodes()).map_err(|why| {
GAP prod LINE crates/api/src/booleansearchjson.rs:110  format!(
GAP prod LINE crates/api/src/booleansearchjson.rs:112  budget.context(&why),
GAP prod LINE crates/api/src/booleansearchjson.rs:113  replay.nodes()
GAP prod LINE crates/api/src/booleansearchjson.rs:115  })
GAP prod LINE crates/api/src/booleansearchjson.rs:118  Ok(body) => {
GAP prod LINE crates/api/src/booleansearchjson.rs:119  let result = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/booleansearchjson.rs:120  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleansearchjson.rs:121  refused(
GAP prod LINE crates/api/src/booleansearchjson.rs:123  "search page exceeds response byte limit; no prefix returned",
GAP prod LINE crates/api/src/booleansearchjson.rs:126  result
GAP prod LINE crates/api/src/booleansearchjson.rs:144  if asked.pin.is_some_and(|pin| pin != reader.pin()) {
GAP prod LINE crates/api/src/booleansearchjson.rs:145  return Err("search checkpoint pin changed; explicitly refresh overview, no replacement page returned".into());
GAP prod LINE crates/api/src/booleansearchjson.rs:146  }
GAP prod LINE crates/api/src/booleansearchjson.rs:147  reader.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:148  let mut body = match asked.detail {
GAP prod LINE crates/api/src/booleansearchjson.rs:149  Some((batch, rung)) => detail(&reader, asked, batch, rung, bytes)?,
GAP prod LINE crates/api/src/booleansearchjson.rs:150  None => overview(&reader, bytes)?,
GAP prod LINE crates/api/src/booleansearchjson.rs:152  let object = body
GAP prod LINE crates/api/src/booleansearchjson.rs:153  .as_object_mut()
GAP prod LINE crates/api/src/booleansearchjson.rs:154  .ok_or("search projection is not an object")?;
GAP prod LINE crates/api/src/booleansearchjson.rs:155  object.insert(
GAP prod LINE crates/api/src/booleansearchjson.rs:156  "projection_version".into(),
GAP prod LINE crates/api/src/booleansearchjson.rs:157  json!(reader.projection_version()),
GAP prod LINE crates/api/src/booleansearchjson.rs:159  object.insert("timeframes".into(), json!(reader.rungs().labels()));
GAP prod LINE crates/api/src/booleansearchjson.rs:160  object.insert("replay_node_limit".into(), json!(nodes.to_string()));
GAP prod LINE crates/api/src/booleansearchjson.rs:161  if asked.detail.is_none() {
GAP prod LINE crates/api/src/booleansearchjson.rs:162  object.insert(
GAP prod LINE crates/api/src/booleansearchjson.rs:163  "planned_campaign".into(),
GAP prod LINE crates/api/src/booleansearchjson.rs:164  json!(reader.planned_campaign()?.map(crate::server::hex32)),
GAP prod LINE crates/api/src/booleansearchjson.rs:166  object.insert(
GAP prod LINE crates/api/src/booleansearchjson.rs:167  "planned_campaign_authority".into(),
GAP prod LINE crates/api/src/booleansearchjson.rs:168  json!("derived-plan-identity"),
GAP prod LINE crates/api/src/booleansearchjson.rs:170  }
GAP prod LINE crates/api/src/booleansearchjson.rs:171  object
GAP prod LINE crates/api/src/booleansearchjson.rs:172  .entry("replay_nodes_charged")
GAP prod LINE crates/api/src/booleansearchjson.rs:173  .or_insert_with(|| json!(reader.replay_nodes().to_string()));
GAP prod LINE crates/api/src/booleansearchjson.rs:174  reader.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:175  Ok(body)
GAP prod LINE crates/api/src/booleansearchjson.rs:185  "running"
GAP prod LINE crates/api/src/booleansearchjson.rs:218  fn overview(reader: &QualifiedSearch, bytes: u64) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleansearchjson.rs:219  let (phase, state) = phase(reader.phase(), reader.owner_observed(), reader.exhausted())?;
GAP prod LINE crates/api/src/booleansearchjson.rs:220  let rows = selected_summaries(reader.summaries(), reader.rungs())?;
GAP prod LINE crates/api/src/booleansearchjson.rs:221  let node_only = phase == "complete" && rows.iter().all(|row| row["qualification"].is_null());
GAP prod LINE crates/api/src/booleansearchjson.rs:223  json!({"schema_version":1,"status":"saved","model":"qualified-search","kind":"overview","authority":"acknowledged-search-history","identity"
GAP prod LINE crates/api/src/booleansearchjson.rs:225  }
GAP prod LINE crates/api/src/booleansearchjson.rs:237  fn fraction(value: SearchFraction) -> Value {
GAP prod LINE crates/api/src/booleansearchjson.rs:238  json!({"numerator":value.numerator().to_string(),"denominator":value.denominator().to_string()})
GAP prod LINE crates/api/src/booleansearchjson.rs:239  }
GAP prod LINE crates/api/src/booleansearchjson.rs:240  fn allocation(value: SearchAllocation, draws: u64) -> Value {
GAP prod LINE crates/api/src/booleansearchjson.rs:241  let minimum = value.minimum_draws();
GAP prod LINE crates/api/src/booleansearchjson.rs:242  json!({"digest":crate::server::hex32(value.digest()),"batch":value.batch().to_string(),"rung":value.rung().to_string(),"rungs":value.rungs()
GAP prod LINE crates/api/src/booleansearchjson.rs:243  }
GAP prod LINE crates/api/src/booleansearchjson.rs:244  fn detail(
GAP prod LINE crates/api/src/booleansearchjson.rs:245  reader: &QualifiedSearch,
GAP prod LINE crates/api/src/booleansearchjson.rs:246  asked: &Asked,
GAP prod LINE crates/api/src/booleansearchjson.rs:247  batch: u64,
GAP prod LINE crates/api/src/booleansearchjson.rs:248  rung: usize,
GAP prod LINE crates/api/src/booleansearchjson.rs:249  bytes: u64,
GAP prod LINE crates/api/src/booleansearchjson.rs:250  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleansearchjson.rs:251  let selected = reader.rung(reader.pin(), batch, rung)?;
GAP prod LINE crates/api/src/booleansearchjson.rs:252  selected.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:253  let source = selected.source();
GAP prod LINE crates/api/src/booleansearchjson.rs:254  let completion = source.completion_digest();
GAP prod LINE crates/api/src/booleansearchjson.rs:255  if asked.completion.is_some_and(|pin| pin != completion) {
GAP prod LINE crates/api/src/booleansearchjson.rs:256  return Err("qualification child pin changed; no replacement page returned".into());
GAP prod LINE crates/api/src/booleansearchjson.rs:257  }
GAP prod LINE crates/api/src/booleansearchjson.rs:258  source.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:259  let rows = selected.rows(completion, asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleansearchjson.rs:260  let search_policy = selected.policy();
GAP prod LINE crates/api/src/booleansearchjson.rs:261  let search_policy_digest = search_policy.digest();
GAP prod LINE crates/api/src/booleansearchjson.rs:262  if rows
GAP prod LINE crates/api/src/booleansearchjson.rs:263  .iter()
GAP prod LINE crates/api/src/booleansearchjson.rs:264  .any(|row| row.projection.policy_digest() != search_policy_digest)
GAP prod LINE crates/api/src/booleansearchjson.rs:266  return Err("search comparison policy differs from its shared probability ceiling".into());
GAP prod LINE crates/api/src/booleansearchjson.rs:267  }
GAP prod LINE crates/api/src/booleansearchjson.rs:268  let original = crate::booleanevidencejson::qualification_observation(
GAP prod LINE crates/api/src/booleansearchjson.rs:269  source,
GAP prod LINE crates/api/src/booleansearchjson.rs:270  completion,
GAP prod LINE crates/api/src/booleansearchjson.rs:271  asked.offset,
GAP prod LINE crates/api/src/booleansearchjson.rs:272  asked.limit,
GAP prod LINE crates/api/src/booleansearchjson.rs:273  )?;
GAP prod LINE crates/api/src/booleansearchjson.rs:274  let rows=rows.iter().enumerate().map(|(n,row)|{
GAP prod LINE crates/api/src/booleansearchjson.rs:275  let [romano,white,spa]=row.probabilities;
GAP prod LINE crates/api/src/booleansearchjson.rs:276  Ok(json!({"comparison":crate::booleanevidencejson::search_comparison(source,asked.offset+n,row)?,"probabilities":{"romano":fraction(romano),
GAP prod LINE crates/api/src/booleansearchjson.rs:277  }).collect::<Result<Vec<_>,String>>()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:278  let [[white_n, white_d], [spa_n, spa_d]] = source.family_probabilities();
GAP prod LINE crates/api/src/booleansearchjson.rs:279  let [draws, _, _] = source.procedure();
GAP prod LINE crates/api/src/booleansearchjson.rs:280  let summary = summary(rung, &selected.summary())?;
GAP prod LINE crates/api/src/booleansearchjson.rs:281  let admitted = selected.admitted_bytes()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:282  let replay_nodes = selected.replay_nodes()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:283  let body = json!({"schema_version":1,"status":"saved","model":"qualified-search","kind":"rung","authority":"authenticated-search-projection"
GAP prod LINE crates/api/src/booleansearchjson.rs:284  let mut body = body;
GAP prod LINE crates/api/src/booleansearchjson.rs:285  if selected.projection_version() == 1 {
GAP prod LINE crates/api/src/booleansearchjson.rs:286  body.as_object_mut()
GAP prod LINE crates/api/src/booleansearchjson.rs:287  .ok_or("search detail is not an object")?
GAP prod LINE crates/api/src/booleansearchjson.rs:288  .insert("scope".into(), json!("Historical V1 comparison retains its original probability ceilings and exact original arithmetic. White and S
GAP prod LINE crates/api/src/booleansearchjson.rs:289  }
GAP prod LINE crates/api/src/booleansearchjson.rs:290  body.as_object_mut()
GAP prod LINE crates/api/src/booleansearchjson.rs:291  .ok_or("search detail is not an object")?
GAP prod LINE crates/api/src/booleansearchjson.rs:292  .insert(
GAP prod LINE crates/api/src/booleansearchjson.rs:293  "search_policy".into(),
GAP prod LINE crates/api/src/booleansearchjson.rs:294  crate::booleanevidencejson::search_policy(&search_policy),
GAP prod LINE crates/api/src/booleansearchjson.rs:296  body.as_object_mut()
GAP prod LINE crates/api/src/booleansearchjson.rs:297  .ok_or("search detail is not an object")?
GAP prod LINE crates/api/src/booleansearchjson.rs:298  .insert(
GAP prod LINE crates/api/src/booleansearchjson.rs:299  "replay_nodes_charged".into(),
GAP prod LINE crates/api/src/booleansearchjson.rs:300  json!(replay_nodes.to_string()),
GAP prod LINE crates/api/src/booleansearchjson.rs:304  crate::detail::put_equity_note(
GAP prod LINE crates/api/src/booleansearchjson.rs:305  &mut body,
GAP prod LINE crates/api/src/booleansearchjson.rs:306  crate::booleanevidencejson::sources_note(source.original().statistics().sources()),
GAP prod LINE crates/api/src/booleansearchjson.rs:307  )?;
GAP prod LINE crates/api/src/booleansearchjson.rs:308  source.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:309  selected.require_current()?;
GAP prod LINE crates/api/src/booleansearchjson.rs:310  Ok(body)
GAP prod LINE crates/api/src/booleansearchjson.rs:311  }
GAP prod BRANCH crates/api/src/booleansearchjson.rs:54:20 False  if completion.is_none() && !seen.contains("offset") && !seen.contains("limit") =>
GAP prod BRANCH crates/api/src/booleansearchjson.rs:59:37 False  if pin.is_some() && rung < 8 && (offset == 0 || completion.is_some()) =>
GAP prod BRANCH crates/api/src/booleansearchjson.rs:71:12 True  if !(1..=256).contains(&limit) {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:120:20 True  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:120:20 False  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:144:8 True  if asked.pin.is_some_and(|pin| pin != reader.pin()) {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:144:8 False  if asked.pin.is_some_and(|pin| pin != reader.pin()) {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:161:8 True  if asked.detail.is_none() {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:161:8 False  if asked.detail.is_none() {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:184:23 True  } else if owner {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:190:14 False  2 if !exhausted => Ok(("refused", "refused")),
GAP prod BRANCH crates/api/src/booleansearchjson.rs:202:8 True  if sum != s.count
GAP prod BRANCH crates/api/src/booleansearchjson.rs:203:26 True  || (complete && (s.projection == [0; 32] || s.allocation == [0; 32]))
GAP prod BRANCH crates/api/src/booleansearchjson.rs:203:53 True  || (complete && (s.projection == [0; 32] || s.allocation == [0; 32]))
GAP prod BRANCH crates/api/src/booleansearchjson.rs:206:20 True  || s.child.pin != [0; 32]
GAP prod BRANCH crates/api/src/booleansearchjson.rs:207:20 True  || s.count != 0
GAP prod BRANCH crates/api/src/booleansearchjson.rs:208:20 True  || s.projection != [0; 32]
GAP prod BRANCH crates/api/src/booleansearchjson.rs:209:20 True  || s.allocation != [0; 32]))
GAP prod BRANCH crates/api/src/booleansearchjson.rs:221:21 True  let node_only = phase == "complete" && rows.iter().all(|row| row["qualification"].is_null());
GAP prod BRANCH crates/api/src/booleansearchjson.rs:221:21 False  let node_only = phase == "complete" && rows.iter().all(|row| row["qualification"].is_null());
GAP prod BRANCH crates/api/src/booleansearchjson.rs:255:8 True  if asked.completion.is_some_and(|pin| pin != completion) {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:255:8 False  if asked.completion.is_some_and(|pin| pin != completion) {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:262:8 True  if rows
GAP prod BRANCH crates/api/src/booleansearchjson.rs:262:8 False  if rows
GAP prod BRANCH crates/api/src/booleansearchjson.rs:285:8 True  if selected.projection_version() == 1 {
GAP prod BRANCH crates/api/src/booleansearchjson.rs:285:8 False  if selected.projection_version() == 1 {
GAP prod LINE crates/api/src/calendar_build.rs:234  return Some(answer.clone());
GAP prod LINE crates/api/src/calendar_build.rs:733  return false;
GAP test LINE crates/api/src/calendar_build.rs:930  other => panic!("a queued key is followed: {other:?}"),
GAP test LINE crates/api/src/calendar_build.rs:986  other => panic!("room for build {n}: {other:?}"),
GAP prod BRANCH crates/api/src/calendar_build.rs:233:32 True  if let Some((_, answer)) = state.done.as_ref() {
GAP prod BRANCH crates/api/src/calendar_build.rs:254:10 False  (at.elapsed() <= PARKED_FOR && same_stamp).then(|| answer.clone())
GAP prod BRANCH crates/api/src/calendar_build.rs:427:16 False  && !build.finished()
GAP prod BRANCH crates/api/src/calendar_build.rs:471:12 False  if flights
GAP prod BRANCH crates/api/src/calendar_build.rs:732:12 True  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
GAP prod BRANCH crates/api/src/calendar_build.rs:732:44 True  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
GAP prod BRANCH crates/api/src/calendar_build.rs:732:44 False  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
GAP prod LINE crates/api/src/calendar_of.rs:554  }
GAP prod LINE crates/api/src/calendar_of.rs:616  Err(why) => report.unreadable.push(why),
GAP prod LINE crates/api/src/calendar_of.rs:671  return;
GAP prod LINE crates/api/src/calendar_of.rs:690  return;
GAP prod LINE crates/api/src/calendar_of.rs:831  Err(_) => unreadable = unreadable.saturating_add(1),
GAP prod LINE crates/api/src/calendar_of.rs:836  return Err(format!(
GAP prod LINE crates/api/src/calendar_of.rs:837  "{unreadable} minute record(s) in {month} could not be read, so the \
GAP prod LINE crates/api/src/calendar_of.rs:838  session windows this month would report are not the ones it traded. \
GAP prod LINE crates/api/src/calendar_of.rs:839  A minute that did not decode splits a run, and a split run is a \
GAP prod LINE crates/api/src/calendar_of.rs:840  midday break the exchange never took."
GAP prod LINE crates/api/src/calendar_of.rs:841  ));
GAP test LINE crates/api/src/calendar_of.rs:1349  other => panic!("day {day} is {other:?}, not an open session"),
GAP ignored LINE crates/api/src/calendar_of.rs:2158  fn it_reproduces_the_operators_store() {
GAP ignored LINE crates/api/src/calendar_of.rs:2159  let Some(home) = std::env::var_os("HOME") else {
GAP ignored LINE crates/api/src/calendar_of.rs:2160  panic!("no HOME");
GAP ignored LINE crates/api/src/calendar_of.rs:2162  let root = std::path::PathBuf::from(home).join(".brutex/store");
GAP ignored LINE crates/api/src/calendar_of.rs:2163  assert!(
GAP ignored LINE crates/api/src/calendar_of.rs:2164  root.join("bars").is_dir(),
GAP ignored LINE crates/api/src/calendar_of.rs:2167  root.display()
GAP ignored LINE crates/api/src/calendar_of.rs:2170  let mut months = Vec::new();
GAP ignored LINE crates/api/src/calendar_of.rs:2171  for year in 2019..=2026_u16 {
GAP ignored LINE crates/api/src/calendar_of.rs:2172  for month in 1..=12_u8 {
GAP ignored LINE crates/api/src/calendar_of.rs:2173  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored LINE crates/api/src/calendar_of.rs:2174  months.push(ym);
GAP ignored LINE crates/api/src/calendar_of.rs:2175  }
GAP ignored LINE crates/api/src/calendar_of.rs:2179  let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
GAP ignored LINE crates/api/src/calendar_of.rs:2181  println!(
GAP ignored LINE crates/api/src/calendar_of.rs:2183  cal.span(),
GAP ignored LINE crates/api/src/calendar_of.rs:2184  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2187  report.unreadable.len()
GAP ignored LINE crates/api/src/calendar_of.rs:2190  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2191  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2197  let mut owed = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2198  let mut unmeasured = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2199  for day in cal.first_day()..=cal.last_day() {
GAP ignored LINE crates/api/src/calendar_of.rs:2200  match cal.expected_bars(day) {
GAP ignored LINE crates/api/src/calendar_of.rs:2201  Some(n) => owed = owed.saturating_add(u32::from(n)),
GAP ignored LINE crates/api/src/calendar_of.rs:2203  if matches!(
GAP ignored LINE crates/api/src/calendar_of.rs:2204  cal.kind_of(day),
GAP ignored LINE crates/api/src/calendar_of.rs:2206  ) {
GAP ignored LINE crates/api/src/calendar_of.rs:2207  unmeasured = unmeasured.saturating_add(1);
GAP ignored LINE crates/api/src/calendar_of.rs:2208  }
GAP ignored LINE crates/api/src/calendar_of.rs:2212  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2217  println!("minute bars owed: {owed}");
GAP ignored LINE crates/api/src/calendar_of.rs:2218  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2224  }
GAP prod LINE crates/api/src/calendar_of.rs:2671  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2684  flush(&mut out, done);
GAP prod LINE crates/api/src/calendar_of.rs:2935  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2946  if n > 0 {
GAP prod LINE crates/api/src/calendar_of.rs:2947  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2948  }
GAP prod LINE crates/api/src/calendar_of.rs:2949  out.push_str(&crate::pullrun::quote_for_json(name));
GAP prod BRANCH crates/api/src/calendar_of.rs:385:16 False  if let Some(key) = self.key.take() {
GAP prod BRANCH crates/api/src/calendar_of.rs:549:28 False  if let std::collections::btree_map::Entry::Vacant(slot) = traded.entry(day) {
GAP prod BRANCH crates/api/src/calendar_of.rs:551:32 False  if let Some(civil) = day_month(day) {
GAP prod BRANCH crates/api/src/calendar_of.rs:585:28 False  if let Some(slot) = traded.get_mut(day) {
GAP prod BRANCH crates/api/src/calendar_of.rs:666:9 False  let Some(mut month) = u32::try_from(calendar.first_day())
GAP prod BRANCH crates/api/src/calendar_of.rs:677:15 True  while ahead.next_if(|held| **held < key).is_some() {}
GAP prod BRANCH crates/api/src/calendar_of.rs:684:24 False  && let Ok(named) = YearMonth::new(month.year(), month.month())
GAP prod BRANCH crates/api/src/calendar_of.rs:689:13 False  let Ok(next) = end.succ() else {
GAP prod BRANCH crates/api/src/calendar_of.rs:827:35 True  Some(last) if last.1 == minute => {}
GAP prod BRANCH crates/api/src/calendar_of.rs:835:8 True  if unreadable > 0 {
GAP test BRANCH crates/api/src/calendar_of.rs:1929:46 False  while cache.waiting() < followers && std::time::Instant::now() < deadline {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2159:13 True  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2159:13 False  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2173:24 True  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2173:24 False  if let Ok(ym) = YearMonth::new(year, month) {
GAP prod BRANCH crates/api/src/calendar_of.rs:2412:12 False  if let Some(at) = entry {
GAP prod BRANCH crates/api/src/calendar_of.rs:2670:12 True  if !first {
GAP prod BRANCH crates/api/src/calendar_of.rs:2683:12 True  if let Some(done) = run {
GAP prod BRANCH crates/api/src/calendar_of.rs:2934:12 True  if position > 0 {
GAP prod BRANCH crates/api/src/calendar_of.rs:2946:16 True  if n > 0 {
GAP prod BRANCH crates/api/src/calendar_of.rs:2946:16 False  if n > 0 {
GAP test BRANCH crates/api/src/calendar_of.rs:3227:27 False  let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
GAP prod LINE crates/api/src/calendar_record.rs:135  || epoch_day.to_string(),
GAP test LINE crates/api/src/calendar_record.rs:317  DayKind::Unmeasured => None,
GAP prod LINE crates/api/src/calendar_record.rs:341  continue;
GAP prod LINE crates/api/src/calendar_record.rs:349  continue;
GAP prod LINE crates/api/src/calendar_record.rs:356  }
GAP prod LINE crates/api/src/calendar_record.rs:502  return None;
GAP prod LINE crates/api/src/calendar_record.rs:615  None => serde_json::json!({
GAP prod LINE crates/api/src/calendar_record.rs:616  "from": iso(from),
GAP prod LINE crates/api/src/calendar_record.rs:617  "to": iso(to),
GAP prod LINE crates/api/src/calendar_record.rs:618  "regular": serde_json::Value::Null,
GAP prod LINE crates/api/src/calendar_record.rs:619  "why": "the derivatives session table carries no verified hours for these days",
GAP prod LINE crates/api/src/calendar_record.rs:644  return None;
GAP prod LINE crates/api/src/calendar_record.rs:754  None => {}
GAP prod LINE crates/api/src/calendar_record.rs:838  walked
GAP prod LINE crates/api/src/calendar_record.rs:839  .withheld
GAP prod LINE crates/api/src/calendar_record.rs:840  .push(serde_json::json!({ "from": start, "to": end }));
GAP prod LINE crates/api/src/calendar_record.rs:881  "the series asked".to_owned()
GAP prod BRANCH crates/api/src/calendar_record.rs:180:20 True  || session != Session::full()
GAP prod BRANCH crates/api/src/calendar_record.rs:340:17 False  let Ok(one) = pull::session::Day::new(month.year(), month.month(), 1) else {
GAP prod BRANCH crates/api/src/calendar_record.rs:348:21 False  let Ok(at) = usize::try_from(day.saturating_sub(origin)) else {
GAP prod BRANCH crates/api/src/calendar_record.rs:351:20 False  if counts.len() <= at {
GAP prod BRANCH crates/api/src/calendar_record.rs:354:24 False  if let Some(slot) = counts.get_mut(at) {
GAP prod BRANCH crates/api/src/calendar_record.rs:501:8 True  if !auction_day(high) {
GAP prod BRANCH crates/api/src/calendar_record.rs:580:27 False  while from <= last && rows.len() <= 8 {
GAP prod BRANCH crates/api/src/calendar_record.rs:643:17 False  let DayKind::Open(session) = pull::calendar::exchange_kind_of(day) else {
GAP prod BRANCH crates/api/src/calendar_record.rs:837:23 True  } else if let Some((start, end)) = run.take() {
GAP prod BRANCH crates/api/src/calendar_record.rs:880:20 True  if names.is_empty() {
GAP test BRANCH crates/api/src/calendar_record_tests.rs:476:56 False  if record_code(day).is_some_and(is_session) && day != at(2022, 3, 1) {
GAP prod LINE crates/api/src/calendar_route.rs:268  && answer.0 == axum::http::StatusCode::OK
GAP prod LINE crates/api/src/calendar_route.rs:269  && let Some(widened) = crate::calendar_of::record::widen_to_record(&answer.2, rung)
GAP prod LINE crates/api/src/calendar_route.rs:271  return (answer.0, answer.1, widened);
GAP prod LINE crates/api/src/calendar_route.rs:276  && let Some(marked) = crate::calendar_of::record::not_widened(&answer.2)
GAP prod LINE crates/api/src/calendar_route.rs:278  return (answer.0, answer.1, marked);
GAP prod LINE crates/api/src/calendar_route.rs:311  Joined::Full => return builds_full(),
GAP prod LINE crates/api/src/calendar_route.rs:671  continue;
GAP prod LINE crates/api/src/calendar_route.rs:768  Err(crate::calendar_of::MonthFault::Unreadable(why)) => {
GAP prod LINE crates/api/src/calendar_route.rs:769  unreadable.push(format!("{}: {why}", symbol.as_str()));
GAP prod LINE crates/api/src/calendar_route.rs:770  months.push((*month, None));
GAP prod LINE crates/api/src/calendar_route.rs:771  }
GAP prod LINE crates/api/src/calendar_route.rs:1044  Err(refused) => return refused,
GAP prod LINE crates/api/src/calendar_route.rs:1155  return;
GAP prod LINE crates/api/src/calendar_route.rs:1159  None => warmed.unqueued = warmed.unqueued.saturating_add(1),
GAP prod LINE crates/api/src/calendar_route.rs:1167  self.site.calendars.builds.abandon(
GAP prod LINE crates/api/src/calendar_route.rs:1168  &key,
GAP prod LINE crates/api/src/calendar_route.rs:1169  &build,
GAP prod LINE crates/api/src/calendar_route.rs:1170  crate::calendar_of::KeptAnswer {
GAP prod LINE crates/api/src/calendar_route.rs:1171  status: axum::http::StatusCode::SERVICE_UNAVAILABLE.as_u16(),
GAP prod LINE crates/api/src/calendar_route.rs:1172  body: serde_json::json!({
GAP prod LINE crates/api/src/calendar_route.rs:1173  "status": "refused",
GAP prod LINE crates/api/src/calendar_route.rs:1174  "feed": key.0.as_str(),
GAP prod LINE crates/api/src/calendar_route.rs:1175  "refusal": "the start-up build stopped before this answer was built; \
GAP prod LINE crates/api/src/calendar_route.rs:1176  ask again, and the request leads a build of its own",
GAP prod LINE crates/api/src/calendar_route.rs:1177  })
GAP prod LINE crates/api/src/calendar_route.rs:1178  .to_string(),
GAP prod LINE crates/api/src/calendar_route.rs:1179  },
GAP prod LINE crates/api/src/calendar_route.rs:1180  );
GAP prod LINE crates/api/src/calendar_route.rs:1181  }
GAP prod LINE crates/api/src/calendar_route.rs:1226  (
GAP prod LINE crates/api/src/calendar_route.rs:1227  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/calendar_route.rs:1228  [(
GAP prod LINE crates/api/src/calendar_route.rs:1229  axum::http::header::CONTENT_TYPE,
GAP prod LINE crates/api/src/calendar_route.rs:1230  "application/json; charset=utf-8",
GAP prod LINE crates/api/src/calendar_route.rs:1231  )],
GAP prod LINE crates/api/src/calendar_route.rs:1232  serde_json::json!({
GAP prod LINE crates/api/src/calendar_route.rs:1233  "status": "refused",
GAP prod LINE crates/api/src/calendar_route.rs:1234  "refusal": format!("the start-up build could not spell {text:?} as a request: {why}"),
GAP prod LINE crates/api/src/calendar_route.rs:1235  })
GAP prod LINE crates/api/src/calendar_route.rs:1236  .to_string(),
GAP prod LINE crates/api/src/calendar_route.rs:1237  )
GAP prod LINE crates/api/src/calendar_route.rs:1238  })
GAP prod LINE crates/api/src/calendar_route.rs:1298  Err(refused) => refused,
GAP prod LINE crates/api/src/calendar_route.rs:1302  warmed.refused = warmed.refused.saturating_add(1);
GAP prod LINE crates/api/src/calendar_route.rs:1311  Ok(Err(why)) => Err(why),
GAP prod LINE crates/api/src/calendar_route.rs:1312  Err(_) => Err("the pull journal's walk at start unwound".to_owned()),
GAP prod LINE crates/api/src/calendar_route.rs:1314  Err(why) => Err(format!("the pull journal's walk could not start: {why}")),
GAP prod BRANCH crates/api/src/calendar_route.rs:267:8 True  if span == "record"
GAP prod BRANCH crates/api/src/calendar_route.rs:268:12 True  && answer.0 == axum::http::StatusCode::OK
GAP prod BRANCH crates/api/src/calendar_route.rs:268:12 False  && answer.0 == axum::http::StatusCode::OK
GAP prod BRANCH crates/api/src/calendar_route.rs:269:16 True  && let Some(widened) = crate::calendar_of::record::widen_to_record(&answer.2, rung)
GAP prod BRANCH crates/api/src/calendar_route.rs:269:16 False  && let Some(widened) = crate::calendar_of::record::widen_to_record(&answer.2, rung)
GAP prod BRANCH crates/api/src/calendar_route.rs:275:8 True  if span == "record"
GAP prod BRANCH crates/api/src/calendar_route.rs:276:16 True  && let Some(marked) = crate::calendar_of::record::not_widened(&answer.2)
GAP prod BRANCH crates/api/src/calendar_route.rs:276:16 False  && let Some(marked) = crate::calendar_of::record::not_widened(&answer.2)
GAP prod BRANCH crates/api/src/calendar_route.rs:491:20 True  Some(_) if progress.owed > 0 => format!(
GAP prod BRANCH crates/api/src/calendar_route.rs:548:12 False  if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
GAP prod BRANCH crates/api/src/calendar_route.rs:645:27 True  (false, false) if rung == timeframe => parse(&rung).map(Some),
GAP prod BRANCH crates/api/src/calendar_route.rs:670:12 True  if key.contract.is_some() {
GAP prod BRANCH crates/api/src/calendar_route.rs:674:25 False  let reference = key.exchange == brutex_core::instrument::Exchange::Nse
GAP prod BRANCH crates/api/src/calendar_route.rs:901:13 False  *exchange == brutex_core::instrument::Exchange::Nse
GAP prod BRANCH crates/api/src/calendar_route.rs:981:43 True  if read.contains_key(identity) || !rungs.contains_key(&rung) {
GAP prod BRANCH crates/api/src/calendar_route.rs:1080:28 False  if keep && let Some(at) = stamp {
GAP prod BRANCH crates/api/src/calendar_route.rs:1154:12 True  if self.builds.iter().any(|(queued, _)| *queued == key) {
GAP prod BRANCH crates/api/src/calendar_route.rs:1301:12 True  if answer.0 != axum::http::StatusCode::OK {
GAP prod LINE crates/api/src/candidatejson.rs:70  _ => return Err("candidate direction must be long or short".to_owned()),
GAP prod LINE crates/api/src/candidatejson.rs:75  return Err("candidate page limit must be 1..=256".to_owned());
GAP prod LINE crates/api/src/candidatejson.rs:78  return Err(
GAP prod LINE crates/api/src/candidatejson.rs:79  "candidate trade pages require both positive rank and direction".to_owned(),
GAP prod LINE crates/api/src/candidatejson.rs:80  );
GAP prod LINE crates/api/src/candidatejson.rs:138  Ok(_) => refusal(
GAP prod LINE crates/api/src/candidatejson.rs:140  "candidate JSON exceeds its response bound; no prefix exposed",
GAP prod LINE crates/api/src/candidatejson.rs:142  Err(why) => refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why),
GAP prod LINE crates/api/src/candidatejson.rs:173  )?
GAP prod LINE crates/api/src/candidatejson.rs:183  return Err("candidate catalog changed; restart from its first page".to_owned());
GAP prod LINE crates/api/src/candidatejson.rs:238  _ => return Err("candidate key is incomplete".to_owned()),
GAP prod LINE crates/api/src/candidatejson.rs:247  )?
GAP prod LINE crates/api/src/candidatejson.rs:250  return Err("candidate catalog changed during read; no mixed page exposed".to_owned());
GAP prod LINE crates/api/src/candidatejson.rs:257  )?;
GAP prod LINE crates/api/src/candidatejson.rs:269  return Err("empty candidate catalog has only its initial metadata page".to_owned());
GAP test LINE crates/api/src/candidatejson.rs:458  )?;
GAP test LINE crates/api/src/candidatejson.rs:472  )?;
GAP test LINE crates/api/src/candidatejson.rs:484  )?)
GAP test LINE crates/api/src/candidatejson.rs:535  )?;
GAP test LINE crates/api/src/candidatejson.rs:616  )?
GAP test LINE crates/api/src/candidatejson.rs:792  )?;
GAP prod BRANCH crates/api/src/candidatejson.rs:74:12 True  if limit == 0 || limit > crate::detail::MAX_PAGE_ROWS {
GAP prod BRANCH crates/api/src/candidatejson.rs:74:26 True  if limit == 0 || limit > crate::detail::MAX_PAGE_ROWS {
GAP prod BRANCH crates/api/src/candidatejson.rs:77:12 True  if rank.is_some() != direction.is_some() || rank == Some(0) {
GAP prod BRANCH crates/api/src/candidatejson.rs:77:53 True  if rank.is_some() != direction.is_some() || rank == Some(0) {
GAP prod BRANCH crates/api/src/candidatejson.rs:135:21 False  Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
GAP prod BRANCH crates/api/src/candidatejson.rs:182:8 True  if asked.digest.is_some_and(|digest| digest != summary.digest) {
GAP prod BRANCH crates/api/src/candidatejson.rs:249:8 True  if observed.digest != summary.digest {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:8 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:27 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:48 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:327:9 True  held.model != summary.model
GAP prod BRANCH crates/api/src/candidatejson.rs:330:16 True  || held.attempt != summary.attempt
GAP prod BRANCH crates/api/src/candidatejson.rs:331:16 True  || held.digest != summary.digest
GAP prod LINE crates/api/src/census.rs:265  }
GAP prod LINE crates/api/src/census.rs:266  let mut level = (*self.settled).clone();
GAP prod LINE crates/api/src/census.rs:267  level.held.extend(self.fresh.held.drain());
GAP prod LINE crates/api/src/census.rs:268  level.changes.append(&mut self.fresh.changes);
GAP prod LINE crates/api/src/census.rs:269  self.settled = Arc::new(level);
GAP prod LINE crates/api/src/census.rs:304  pub fn is_empty(&self) -> bool {
GAP prod LINE crates/api/src/census.rs:305  self.older.is_empty() && self.settled.is_empty() && self.fresh.is_empty()
GAP prod LINE crates/api/src/census.rs:306  }
GAP prod LINE crates/api/src/census.rs:310  pub fn read_back(&self) -> usize {
GAP prod LINE crates/api/src/census.rs:311  self.older.len()
GAP prod LINE crates/api/src/census.rs:312  }
GAP prod LINE crates/api/src/census.rs:528  return None;
GAP prod LINE crates/api/src/census.rs:564  return Some(Vec::new());
GAP prod LINE crates/api/src/census.rs:582  }
GAP prod LINE crates/api/src/census.rs:586  return None;
GAP prod LINE crates/api/src/census.rs:595  return None;
GAP prod LINE crates/api/src/census.rs:667  return header.n_valid == 0;
GAP prod LINE crates/api/src/census.rs:670  return false;
GAP prod LINE crates/api/src/census.rs:678  return false;
GAP prod LINE crates/api/src/census.rs:681  return false;
GAP prod LINE crates/api/src/census.rs:685  return false;
GAP prod LINE crates/api/src/census.rs:708  (Some(mine), Some(theirs)) => Arc::ptr_eq(mine, theirs),
GAP prod LINE crates/api/src/census.rs:767  pub fn partitions(manifests: Vec<Manifest>) -> Self {
GAP prod LINE crates/api/src/census.rs:768  Self {
GAP prod LINE crates/api/src/census.rs:769  parts: manifests.into_iter().map(Part::in_memory).collect(),
GAP prod LINE crates/api/src/census.rs:770  partitioned: true,
GAP prod LINE crates/api/src/census.rs:771  }
GAP prod LINE crates/api/src/census.rs:772  }
GAP prod LINE crates/api/src/census.rs:824  pub fn closes(&self, key: &EntryKey) -> Option<Closes> {
GAP prod LINE crates/api/src/census.rs:825  self.held(key).map(|held| held.closes)
GAP prod LINE crates/api/src/census.rs:826  }
GAP prod LINE crates/api/src/census.rs:1005  (Census::Absent, Census::Absent) => true,
GAP prod LINE crates/api/src/census.rs:1167  || "the single file".to_owned(),
GAP prod LINE crates/api/src/census.rs:1327  let reason = format!(
GAP prod LINE crates/api/src/census.rs:1330  dir.display(),
GAP prod LINE crates/api/src/census.rs:1331  vendor.as_str(),
GAP prod LINE crates/api/src/census.rs:1332  vendor.as_str()
GAP prod LINE crates/api/src/census.rs:1334  Located::Refused(
GAP prod LINE crates/api/src/census.rs:1335  dir,
GAP prod LINE crates/api/src/census.rs:1336  Census::Unreadable {
GAP prod LINE crates/api/src/census.rs:1337  reason,
GAP prod LINE crates/api/src/census.rs:1338  fault: Fault::Refused,
GAP prod LINE crates/api/src/census.rs:1339  },
GAP prod LINE crates/api/src/census.rs:1340  )
GAP prod LINE crates/api/src/census.rs:1343  if matches!(
GAP prod LINE crates/api/src/census.rs:1352  Err(why) => {
GAP prod LINE crates/api/src/census.rs:1353  let reason = format!(
GAP prod LINE crates/api/src/census.rs:1356  dir.display(),
GAP prod LINE crates/api/src/census.rs:1357  vendor.as_str()
GAP prod LINE crates/api/src/census.rs:1359  Located::Refused(
GAP prod LINE crates/api/src/census.rs:1360  dir,
GAP prod LINE crates/api/src/census.rs:1361  Census::Unreadable {
GAP prod LINE crates/api/src/census.rs:1362  reason,
GAP prod LINE crates/api/src/census.rs:1363  fault: Fault::Io(why.kind()),
GAP prod LINE crates/api/src/census.rs:1364  },
GAP prod LINE crates/api/src/census.rs:1365  )
GAP prod LINE crates/api/src/census.rs:1401  reason: format!(
GAP prod LINE crates/api/src/census.rs:1402  "it holds a version-4 epoch of the {} partition, not a \
GAP prod LINE crates/api/src/census.rs:1403  single-file census; refused rather than read as all of {}'s \
GAP prod LINE crates/api/src/census.rs:1404  census (D-3316)",
GAP prod LINE crates/api/src/census.rs:1405  manifest.segment().map_or("", Segment::as_str),
GAP prod LINE crates/api/src/census.rs:1406  vendor.as_str()
GAP prod LINE crates/api/src/census.rs:1407  ),
GAP prod LINE crates/api/src/census.rs:1408  fault: Fault::Refused,
GAP prod LINE crates/api/src/census.rs:1409  },
GAP prod LINE crates/api/src/census.rs:1454  Err(why) => {
GAP prod LINE crates/api/src/census.rs:1455  return Census::Unreadable {
GAP prod LINE crates/api/src/census.rs:1456  reason: why.to_string(),
GAP prod LINE crates/api/src/census.rs:1457  fault: Fault::Io(why.kind()),
GAP prod LINE crates/api/src/census.rs:1458  };
GAP prod LINE crates/api/src/census.rs:1491  Err(why) => {
GAP prod LINE crates/api/src/census.rs:1492  return Census::Unreadable {
GAP prod LINE crates/api/src/census.rs:1493  reason: named(&why),
GAP prod LINE crates/api/src/census.rs:1494  fault: Fault::Io(why.kind()),
GAP prod LINE crates/api/src/census.rs:1495  };
GAP prod LINE crates/api/src/census.rs:1572  Err(refused) => return Ok(Err(refused)),
GAP prod LINE crates/api/src/census.rs:1591  return Ok(Err(format!(
GAP prod LINE crates/api/src/census.rs:1592  "grew past {cap} bytes while it was read; this reader refuses more"
GAP prod LINE crates/api/src/census.rs:1593  )));
GAP prod LINE crates/api/src/census.rs:1646  return None;
GAP prod LINE crates/api/src/census.rs:1676  return None;
GAP prod LINE crates/api/src/census.rs:1694  return None;
GAP prod LINE crates/api/src/census.rs:1711  return None;
GAP prod LINE crates/api/src/census.rs:1732  (None, None) if before.n_valid == 0 => {}
GAP prod LINE crates/api/src/census.rs:1733  _ => return None,
GAP prod LINE crates/api/src/census.rs:1762  return None;
GAP prod LINE crates/api/src/census.rs:1777  return None;
GAP prod LINE crates/api/src/census.rs:1788  return None;
GAP prod LINE crates/api/src/census.rs:1877  return Ok(Err(format!(
GAP prod LINE crates/api/src/census.rs:1878  "grew past {MAX_MANIFEST_BYTES} bytes while it was read ({size} when opened); this reader refuses more"
GAP prod LINE crates/api/src/census.rs:1879  )));
GAP prod LINE crates/api/src/census.rs:1941  other => Ok(Err(other.to_string())),
GAP prod LINE crates/api/src/census.rs:1955  }
GAP prod LINE crates/api/src/census.rs:1966  }
GAP prod LINE crates/api/src/census.rs:1968  let _ = kind;
GAP prod LINE crates/api/src/census.rs:1969  "device or other special file"
GAP prod LINE crates/api/src/census.rs:2185  )?;
GAP prod LINE crates/api/src/census.rs:2432  return false;
GAP prod LINE crates/api/src/census.rs:2435  return false;
GAP prod LINE crates/api/src/census.rs:2438  return false;
GAP prod LINE crates/api/src/census.rs:2441  return false;
GAP prod LINE crates/api/src/census.rs:2463  _ => true,
GAP test LINE crates/api/src/census.rs:2869  // Release the stuck open so the suite does not hang, then fail.
GAP test LINE crates/api/src/census.rs:2870  let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
GAP prod BRANCH crates/api/src/census.rs:263:12 False  if self.fresh.changes.len() < FRESH_LIMIT {
GAP prod BRANCH crates/api/src/census.rs:305:9 True  self.older.is_empty() && self.settled.is_empty() && self.fresh.is_empty()
GAP prod BRANCH crates/api/src/census.rs:305:9 False  self.older.is_empty() && self.settled.is_empty() && self.fresh.is_empty()
GAP prod BRANCH crates/api/src/census.rs:305:34 True  self.older.is_empty() && self.settled.is_empty() && self.fresh.is_empty()
GAP prod BRANCH crates/api/src/census.rs:305:34 False  self.older.is_empty() && self.settled.is_empty() && self.fresh.is_empty()
GAP prod BRANCH crates/api/src/census.rs:527:12 True  if self.base.degraded_reason().is_some() {
GAP prod BRANCH crates/api/src/census.rs:563:12 True  if from >= to {
GAP prod BRANCH crates/api/src/census.rs:572:12 True  if !meta.is_file() || FileId::of(&meta)? != id {
GAP prod BRANCH crates/api/src/census.rs:577:16 False  if let Some(first) = self.first_entry.as_deref() {
GAP prod BRANCH crates/api/src/census.rs:579:16 True  if file.read_exact_at(&mut again, HEADER_LEN).is_err() || again.as_slice() != first {
GAP prod BRANCH crates/api/src/census.rs:585:12 True  if meta.len() < end {
GAP prod BRANCH crates/api/src/census.rs:594:16 True  if base.segment.is_some_and(|segment| key.segment != segment) {
GAP prod BRANCH crates/api/src/census.rs:646:20 False  let same = self.segment == other.segment
GAP prod BRANCH crates/api/src/census.rs:647:16 False  && self.epoch == other.epoch
GAP prod BRANCH crates/api/src/census.rs:651:16 False  && lineage(self).is_some()
GAP prod BRANCH crates/api/src/census.rs:652:16 False  && lineage(self) == lineage(other)
GAP prod BRANCH crates/api/src/census.rs:653:16 False  && self.base.degraded_reason().is_none()
GAP prod BRANCH crates/api/src/census.rs:654:16 False  && other.base.degraded_reason().is_none()
GAP prod BRANCH crates/api/src/census.rs:655:16 False  && self.first_entry.is_some()
GAP prod BRANCH crates/api/src/census.rs:666:13 False  let Some(last) = other.last_entry.as_deref() else {
GAP prod BRANCH crates/api/src/census.rs:669:13 False  let Ok(layout) = Layout::for_version(header.format_version) else {
GAP prod BRANCH crates/api/src/census.rs:677:13 False  let Some(at) = at else {
GAP prod BRANCH crates/api/src/census.rs:680:13 False  let Ok(file) = open_without_waiting(&self.path) else {
GAP prod BRANCH crates/api/src/census.rs:684:12 True  if identity.is_none() || identity != self.file {
GAP prod BRANCH crates/api/src/census.rs:684:34 True  if identity.is_none() || identity != self.file {
GAP prod BRANCH crates/api/src/census.rs:688:9 False  file.read_exact_at(&mut again, at).is_ok() && again.as_slice() == last
GAP prod BRANCH crates/api/src/census.rs:702:9 False  self.segment == other.segment
GAP prod BRANCH crates/api/src/census.rs:703:16 False  && self.epoch == other.epoch
GAP prod BRANCH crates/api/src/census.rs:704:16 False  && self.path == other.path
GAP prod BRANCH crates/api/src/census.rs:784:9 False  self.partitioned == other.partitioned
GAP prod BRANCH crates/api/src/census.rs:785:16 False  && self.parts.len() == other.parts.len()
GAP prod BRANCH crates/api/src/census.rs:1002:9 False  self.vendor == other.vendor
GAP prod BRANCH crates/api/src/census.rs:1325:22 False  Ok(found) if found.is_dir() => Located::Partitioned(dir),
GAP prod BRANCH crates/api/src/census.rs:1400:33 True  Ok(manifest) if manifest.segment().is_some() => Census::Unreadable {
GAP prod BRANCH crates/api/src/census.rs:1590:8 True  if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > cap {
GAP prod BRANCH crates/api/src/census.rs:1645:12 True  if header.generation % 2 != index {
GAP prod BRANCH crates/api/src/census.rs:1675:8 True  if previous.base.degraded_reason().is_some() {
GAP prod BRANCH crates/api/src/census.rs:1680:8 True  if !meta.is_file() || FileId::of(&meta)? != id {
GAP prod BRANCH crates/api/src/census.rs:1688:8 True  if newest.vendor != vendor
GAP prod BRANCH crates/api/src/census.rs:1689:12 True  || newest.segment != before.segment
GAP prod BRANCH crates/api/src/census.rs:1690:12 True  || newest.format_version != before.format_version
GAP prod BRANCH crates/api/src/census.rs:1691:12 True  || newest.entry_stride != before.entry_stride
GAP prod BRANCH crates/api/src/census.rs:1692:12 True  || newest.limit() != before.limit()
GAP prod BRANCH crates/api/src/census.rs:1701:12 True  || newest.n_valid > newest.limit()
GAP prod BRANCH crates/api/src/census.rs:1702:12 True  || newest.generation.checked_sub(newest.n_valid)?
GAP prod BRANCH crates/api/src/census.rs:1710:8 True  if meta.len() < end {
GAP prod BRANCH crates/api/src/census.rs:1721:9 False  file.read_exact_at(&mut again, at).is_ok() && again.as_slice() == kept
GAP prod BRANCH crates/api/src/census.rs:1728:47 True  if !same_at(first, HEADER_LEN) || !same_at(last, start.checked_sub(stride)?) {
GAP prod BRANCH crates/api/src/census.rs:1732:25 True  (None, None) if before.n_valid == 0 => {}
GAP prod BRANCH crates/api/src/census.rs:1732:25 False  (None, None) if before.n_valid == 0 => {}
GAP prod BRANCH crates/api/src/census.rs:1761:12 True  if newest.segment.is_some_and(|segment| key.segment != segment) {
GAP prod BRANCH crates/api/src/census.rs:1774:20 True  if held.entry.rows < was.entry.rows
GAP prod BRANCH crates/api/src/census.rs:1775:24 True  || held.entry.last_ts_micros < was.entry.last_ts_micros
GAP prod BRANCH crates/api/src/census.rs:1787:8 True  if keys != newest.n_keys || rows != newest.total_rows {
GAP prod BRANCH crates/api/src/census.rs:1787:33 True  if keys != newest.n_keys || rows != newest.total_rows {
GAP prod BRANCH crates/api/src/census.rs:1876:8 True  if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_MANIFEST_BYTES {
GAP prod BRANCH crates/api/src/census.rs:1952:12 False  if let Some(flags) = NONBLOCK {
GAP prod BRANCH crates/api/src/census.rs:1964:12 False  if kind.is_fifo() {
GAP prod BRANCH crates/api/src/census.rs:2431:12 True  if self.segment.is_some_and(|s| s != series.segment) {
GAP prod BRANCH crates/api/src/census.rs:2434:12 True  if self.timeframe.is_some_and(|t| t != series.timeframe) {
GAP prod BRANCH crates/api/src/census.rs:2437:12 True  if self.from.is_some_and(|f| month < f) {
GAP prod BRANCH crates/api/src/census.rs:2440:12 True  if self.to.is_some_and(|t| month > t) {
GAP prod BRANCH crates/api/src/census.rs:2460:33 False  Some(ref needle) if !needle.is_empty() => {
GAP prod BRANCH crates/api/src/census.rs:2475:16 False  && self.timeframe.is_none()
GAP prod BRANCH crates/api/src/census.rs:2476:16 False  && self.from.is_none()
GAP prod BRANCH crates/api/src/census.rs:2477:16 False  && self.to.is_none()
GAP test BRANCH crates/api/src/census.rs:2868:12 True  if read.is_err() {
GAP test BRANCH crates/api/src/census.rs:3193:13 True  [an operand followed by ||: neither side fails the check by itself]  rows == 0 || filled > 0,
GAP prod LINE crates/api/src/census_cache.rs:471  return (
GAP prod LINE crates/api/src/census_cache.rs:472  Arc::new(Slot {
GAP prod LINE crates/api/src/census_cache.rs:473  read_at: std::time::SystemTime::now(),
GAP prod LINE crates/api/src/census_cache.rs:474  census: fresh_read(&site.store_root, vendor, None),
GAP prod LINE crates/api/src/census_cache.rs:475  entries: OnceLock::new(),
GAP prod LINE crates/api/src/census_cache.rs:476  }),
GAP prod LINE crates/api/src/census_cache.rs:477  Freshness::Current,
GAP prod LINE crates/api/src/census_cache.rs:478  None,
GAP prod LINE crates/api/src/census_cache.rs:479  );
GAP prod LINE crates/api/src/census_cache.rs:616  return false;
GAP prod LINE crates/api/src/census_cache.rs:628  return;
GAP prod LINE crates/api/src/census_cache.rs:639  }
GAP prod LINE crates/api/src/census_cache.rs:643  cell.refreshing.store(false, Ordering::Release);
GAP prod LINE crates/api/src/census_cache.rs:644  note_warmth(
GAP prod LINE crates/api/src/census_cache.rs:645  vendors,
GAP prod LINE crates/api/src/census_cache.rs:647  vendor,
GAP prod LINE crates/api/src/census_cache.rs:648  WarmState::Failed,
GAP prod LINE crates/api/src/census_cache.rs:649  Some(format!(
GAP prod LINE crates/api/src/census_cache.rs:650  "the background re-read could not be started ({why}); readers read it themselves"
GAP prod LINE crates/api/src/census_cache.rs:651  )),
GAP prod LINE crates/api/src/census_cache.rs:653  return false;
GAP prod LINE crates/api/src/census_cache.rs:1010  vendors
GAP prod LINE crates/api/src/census_cache.rs:1011  .warm_running
GAP prod LINE crates/api/src/census_cache.rs:1012  .store(false, std::sync::atomic::Ordering::Release);
GAP prod LINE crates/api/src/census_cache.rs:1091  for name in [JOURNAL, INDEX_MAP, FACETS] {
GAP prod LINE crates/api/src/census_cache.rs:1092  note_warmth(
GAP prod LINE crates/api/src/census_cache.rs:1093  vendors,
GAP prod LINE crates/api/src/census_cache.rs:1094  name,
GAP prod LINE crates/api/src/census_cache.rs:1095  vendor,
GAP prod LINE crates/api/src/census_cache.rs:1096  WarmState::Failed,
GAP prod LINE crates/api/src/census_cache.rs:1097  Some(format!(
GAP prod LINE crates/api/src/census_cache.rs:1098  "not built: the census read was not kept, because the manifest's stamps \
GAP prod LINE crates/api/src/census_cache.rs:1099  contradict it ({}); the first request that needs it builds it",
GAP prod LINE crates/api/src/census_cache.rs:1100  slot.census.note()
GAP prod LINE crates/api/src/census_cache.rs:1101  )),
GAP prod LINE crates/api/src/census_cache.rs:1102  );
GAP prod LINE crates/api/src/census_cache.rs:1103  }
GAP prod LINE crates/api/src/census_cache.rs:1104  }
GAP prod LINE crates/api/src/census_cache.rs:1132  Err(why) => {
GAP prod LINE crates/api/src/census_cache.rs:1133  warn_line!(
GAP prod LINE crates/api/src/census_cache.rs:1137  None
GAP prod LINE crates/api/src/census_cache.rs:1199  Err(_) => warm_one(site, vendors, feed),
GAP prod LINE crates/api/src/census_cache.rs:1210  // A WARM THAT PANICKED leaves its rows as they were (`warming`,
GAP prod LINE crates/api/src/census_cache.rs:1211  // or the last state it reached), and readers read for themselves.
GAP prod LINE crates/api/src/census_cache.rs:1212  warn_line!("a census warm thread panicked; its caches are read by their first request");
GAP prod LINE crates/api/src/census_cache.rs:1250  Some(why) => note_warmth(vendors, JOURNAL, vendor, WarmState::Failed, Some(why)),
GAP prod LINE crates/api/src/census_cache.rs:1290  // NOT "READY" OVER A READ IT DOES NOT ANSWER: an index another
GAP prod LINE crates/api/src/census_cache.rs:1291  // thread moved meanwhile to a read this one cannot stand for.
GAP prod LINE crates/api/src/census_cache.rs:1292  note_warmth(
GAP prod LINE crates/api/src/census_cache.rs:1293  vendors,
GAP prod LINE crates/api/src/census_cache.rs:1294  FACETS,
GAP prod LINE crates/api/src/census_cache.rs:1295  vendor,
GAP prod LINE crates/api/src/census_cache.rs:1296  WarmState::Failed,
GAP prod LINE crates/api/src/census_cache.rs:1297  Some(format!(
GAP prod LINE crates/api/src/census_cache.rs:1298  "the {} index does not answer the census read it was built for; the next \
GAP prod LINE crates/api/src/census_cache.rs:1299  request that needs it starts its build",
GAP prod LINE crates/api/src/census_cache.rs:1300  behind.join(", ")
GAP prod LINE crates/api/src/census_cache.rs:1301  )),
GAP prod LINE crates/api/src/census_cache.rs:1302  );
GAP prod LINE crates/api/src/census_cache.rs:1303  }
GAP prod LINE crates/api/src/census_cache.rs:1767  Ok(meta) => stamp_of(Ok(meta)),
GAP prod LINE crates/api/src/census_cache.rs:1842  Err(why) => return SegmentStamp::Faulted(why.kind()),
GAP prod LINE crates/api/src/census_cache.rs:1850  None => None,
GAP prod LINE crates/api/src/census_cache.rs:1854  ))?;
GAP prod BRANCH crates/api/src/census_cache.rs:468:9 False  let Some(cell) = vendors.cells.get(vendor as usize) else {
GAP prod BRANCH crates/api/src/census_cache.rs:615:9 False  let Some(cell) = vendors.cells.get(vendor as usize) else {
GAP prod BRANCH crates/api/src/census_cache.rs:627:17 False  let Some(cell) = mine.cells.get(vendor as usize) else {
GAP prod BRANCH crates/api/src/census_cache.rs:635:20 False  if again.manifests.get(vendor as usize) == Some(&stamp)
GAP prod BRANCH crates/api/src/census_cache.rs:636:24 False  && again.root_is_dir == root_is_dir
GAP prod BRANCH crates/api/src/census_cache.rs:642:12 True  if let Err(why) = spawned {
GAP prod BRANCH crates/api/src/census_cache.rs:768:16 False  && let Some(at) = stamps.manifests.get_mut(vendor as usize)
GAP prod BRANCH crates/api/src/census_cache.rs:774:58 False  if matches!(census.state, census::Census::Absent) && !freshness.is_stale() {
GAP prod BRANCH crates/api/src/census_cache.rs:879:12 False  if row.state != state {
GAP prod BRANCH crates/api/src/census_cache.rs:918:16 False  if let Some(row) = rows
GAP prod BRANCH crates/api/src/census_cache.rs:1009:8 True  if spawned.is_err() {
GAP prod BRANCH crates/api/src/census_cache.rs:1048:41 True  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
GAP prod BRANCH crates/api/src/census_cache.rs:1088:8 False  if keeps(site, vendor, &slot) {
GAP prod BRANCH crates/api/src/census_cache.rs:1090:15 True  } else if derived_feeds(vendors).contains(&vendor) {
GAP prod BRANCH crates/api/src/census_cache.rs:1090:15 False  } else if derived_feeds(vendors).contains(&vendor) {
GAP prod BRANCH crates/api/src/census_cache.rs:1141:12 False  if let Some(sender) = sender {
GAP prod BRANCH crates/api/src/census_cache.rs:1209:12 True  if handle.join().is_err() {
GAP prod BRANCH crates/api/src/census_cache.rs:1226:12 False  if let Some(cell) = vendors.cells.get(vendor as usize) {
GAP prod BRANCH crates/api/src/census_cache.rs:1287:8 False  if behind.is_empty() {
GAP prod BRANCH crates/api/src/census_cache.rs:1607:16 False  if let Some(at) = stamps.manifests.get_mut(vendor) {
GAP prod BRANCH crates/api/src/census_cache.rs:1629:20 False  if let Some(at) = again.manifests.get_mut(vendor) {
GAP prod BRANCH crates/api/src/census_cache.rs:1764:21 False  Ok(meta) if meta.is_dir() => {
GAP prod BRANCH crates/api/src/census_cache.rs:1839:29 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/census_cache.rs:2170:20 False  if stamp_could_read(stamp, self.root_is_dir, &row.state) =>
GAP test LINE crates/api/src/census_epoch_link_tests.rs:173  }
GAP test BRANCH crates/api/src/census_epoch_link_tests.rs:166:16 False  if let Some(epoch) = entry
GAP test LINE crates/api/src/census_request_tests.rs:942  torn.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
GAP test LINE crates/api/src/census_request_tests.rs:1211  }
GAP test LINE crates/api/src/census_request_tests.rs:1212  std::thread::sleep(Duration::from_millis(1));
GAP test LINE crates/api/src/census_request_tests.rs:1213  change();
GAP test LINE crates/api/src/census_request_tests.rs:1215  assert_ne!(
GAP test LINE crates/api/src/census_request_tests.rs:1216  changed(),
GAP test LINE crates/api/src/census_request_tests.rs:1219  path.display()
GAP test LINE crates/api/src/census_request_tests.rs:3300  pull::vendor::Transport::LocalArchive(_) => None,
GAP test LINE crates/api/src/census_request_tests.rs:3474  (Self::Answer(_), Self::Calendar(_)) | (Self::Calendar(_), Self::Answer(_)) => false,
GAP test LINE crates/api/src/census_request_tests.rs:4016  fn census_never_read(_site: &Site) -> (CensusStamps, CensusNow) {
GAP test BRANCH crates/api/src/census_request_tests.rs:941:24 True  if entries.len() != 1 || dhan != Some("held") {
GAP test BRANCH crates/api/src/census_request_tests.rs:941:46 True  if entries.len() != 1 || dhan != Some("held") {
GAP test BRANCH crates/api/src/census_request_tests.rs:1209:12 False  if changed() != before {
GAP test BRANCH crates/api/src/census_request_tests.rs:2037:18 False  } if !matches!(kind, ErrorKind::PermissionDenied | ErrorKind::IsADirectory)
GAP test BRANCH crates/api/src/census_request_tests.rs:2335:12 False  if let Some(dhan) = one_off.get_mut(1) {
GAP test BRANCH crates/api/src/census_request_tests.rs:2419:30 False  } if reason == CONTRADICTED_ABSENCE
GAP test BRANCH crates/api/src/census_request_tests.rs:2590:22 False  } if reason == CONTRADICTED_ABSENCE
GAP prod LINE crates/api/src/census_rollup.rs:168  bad(format!(
GAP prod LINE crates/api/src/census_rollup.rs:169  "exchange {:?}: {}",
GAP prod LINE crates/api/src/census_rollup.rs:170  echo(word),
GAP prod LINE crates/api/src/census_rollup.rs:171  echo(&why.to_string())
GAP prod LINE crates/api/src/census_rollup.rs:172  ))
GAP prod LINE crates/api/src/census_rollup.rs:173  })?),
GAP prod LINE crates/api/src/census_rollup.rs:201  return None;
GAP prod LINE crates/api/src/census_rollup.rs:355  Ok(exchange)
GAP prod LINE crates/api/src/census_rollup.rs:409  None => return not_held(index, asked, symbol),
GAP prod LINE crates/api/src/census_rollup.rs:414  error_json(
GAP prod LINE crates/api/src/census_rollup.rs:415  "exchange_ambiguous",
GAP prod LINE crates/api/src/census_rollup.rs:416  &format!(
GAP prod LINE crates/api/src/census_rollup.rs:418  symbol.as_str(),
GAP prod LINE crates/api/src/census_rollup.rs:419  asked.feed.as_str(),
GAP prod LINE crates/api/src/census_rollup.rs:420  asked.segment.as_str(),
GAP prod LINE crates/api/src/census_rollup.rs:421  holders
GAP prod LINE crates/api/src/census_rollup.rs:422  .iter()
GAP prod LINE crates/api/src/census_rollup.rs:423  .map(|e| e.as_str())
GAP prod LINE crates/api/src/census_rollup.rs:424  .collect::<Vec<_>>()
GAP prod LINE crates/api/src/census_rollup.rs:425  .join(" and ")
GAP prod LINE crates/api/src/census_rollup.rs:434  return (
GAP prod LINE crates/api/src/census_rollup.rs:435  axum::http::StatusCode::UNPROCESSABLE_ENTITY,
GAP prod LINE crates/api/src/census_rollup.rs:436  error_json(
GAP prod LINE crates/api/src/census_rollup.rs:437  "rollup_too_many",
GAP prod LINE crates/api/src/census_rollup.rs:438  &format!(
GAP prod LINE crates/api/src/census_rollup.rs:439  "the selection holds {} (month, rung) cells; one answer carries at most \
GAP prod LINE crates/api/src/census_rollup.rs:440  {CELLS_MOST}: name an underlying",
GAP prod LINE crates/api/src/census_rollup.rs:441  cells.len()
GAP prod LINE crates/api/src/census_rollup.rs:442  ),
GAP prod LINE crates/api/src/census_rollup.rs:443  ),
GAP prod LINE crates/api/src/census_rollup.rs:444  );
GAP prod BRANCH crates/api/src/census_rollup.rs:200:8 True  if first > last {
GAP prod BRANCH crates/api/src/census_rollup.rs:354:19 True  return if index.holds(exchange, symbol) {
GAP prod BRANCH crates/api/src/census_rollup.rs:411:29 True  Err(holders) if holders.len() > 1 => {
GAP prod BRANCH crates/api/src/census_rollup.rs:433:8 True  if cells.len() > CELLS_MOST {
GAP test LINE crates/api/src/census_tail_tests.rs:110  Census::Absent | Census::Unreadable { .. } => panic!("not held: {}", census.note()),
GAP test LINE crates/api/src/census_tail_tests.rs:501  Census::Absent | Census::Held { .. } => panic!("served past its capacity: {}", over.note()),
GAP test LINE crates/api/src/census_warm_tests.rs:1255  }
GAP test LINE crates/api/src/census_warm_tests.rs:1256  assert!(
GAP test LINE crates/api/src/census_warm_tests.rs:1257  Instant::now() < deadline,
GAP test LINE crates/api/src/census_warm_tests.rs:1260  std::thread::sleep(Duration::from_millis(10));
GAP test BRANCH crates/api/src/census_warm_tests.rs:173:12 False  if !rows.is_empty()
GAP test BRANCH crates/api/src/census_warm_tests.rs:447:47 False  if freshness == Freshness::Current && generation == before + 1 {
GAP test BRANCH crates/api/src/census_warm_tests.rs:591:13 False  || (view.starts_with("/indexmap.json")
GAP test BRANCH crates/api/src/census_warm_tests.rs:592:16 False  && reply.starts_with("HTTP/1.1 500")
GAP test BRANCH crates/api/src/census_warm_tests.rs:970:65 False  if header(&headers, CENSUS_REFRESHING_HEADER) == "0" && served(&body) == walked(&store.fno)
GAP test BRANCH crates/api/src/census_warm_tests.rs:1159:64 False  while inside.load(Ordering::SeqCst) < 2 && Instant::now() < deadline {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1199:47 False  if freshness == Freshness::Current && generation == was + 1 && facets == vendor_generation {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1248:24 False  && crate::emitted::says(record, "state", "ready")
GAP test BRANCH crates/api/src/census_warm_tests.rs:1253:12 False  if WARMED.into_iter().all(named) && !warmed.is_empty() {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1253:45 False  if WARMED.into_iter().all(named) && !warmed.is_empty() {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1384:47 False  if freshness == Freshness::Current && generation == was + 1 {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1413:45 False  if body["generation"] == was + 1 && header(&headers, CENSUS_REFRESHING_HEADER) == "0" {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1435:47 False  if freshness == Freshness::Current && at == generation {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1449:12 False  if code == 200
GAP test BRANCH crates/api/src/census_warm_tests.rs:1584:24 False  if code == 200
GAP test BRANCH crates/api/src/census_warm_tests.rs:1586:28 False  && header(&headers, CENSUS_REFRESHING_HEADER) == "0"
GAP test BRANCH crates/api/src/census_warm_tests.rs:1626:27 False  if code == 503 && body["error"] == "census_unreadable" && facets == WarmState::Failed {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1659:20 False  if held && now.freshness() == Freshness::Current {
GAP test BRANCH crates/api/src/census_warm_tests.rs:1740:13 False  crate::emitted::says(record, "feed", "gdfl")
GAP prod LINE crates/api/src/chain.rs:405  return refused(
GAP prod LINE crates/api/src/chain.rs:406  stored,
GAP prod LINE crates/api/src/chain.rs:407  None,
GAP prod LINE crates/api/src/chain.rs:408  "no_strike_stored_by_then",
GAP prod LINE crates/api/src/chain.rs:409  "no strike of this expiry has a bar stored at or before this instant".to_owned(),
GAP prod LINE crates/api/src/chain.rs:410  );
GAP prod LINE crates/api/src/chain.rs:510  _ => false,
GAP prod LINE crates/api/src/chain.rs:585  return;
GAP prod LINE crates/api/src/chain.rs:637  fn next_month(month: YearMonth) -> Option<YearMonth> {
GAP prod LINE crates/api/src/chain.rs:638  let (year, number) = if month.month() == 12 {
GAP prod LINE crates/api/src/chain.rs:639  (month.year().checked_add(1)?, 1)
GAP prod LINE crates/api/src/chain.rs:641  (month.year(), month.month() + 1)
GAP prod LINE crates/api/src/chain.rs:643  YearMonth::new(year, number).ok()
GAP prod LINE crates/api/src/chain.rs:644  }
GAP prod LINE crates/api/src/chain.rs:692  Ok(None)
GAP prod LINE crates/api/src/chain.rs:697  Ok(Some(ts)) => {
GAP prod LINE crates/api/src/chain.rs:698  faults.push(format!(
GAP prod LINE crates/api/src/chain.rs:700  file.path().display(),
GAP prod LINE crates/api/src/chain.rs:701  from.div_euclid(MICROS)
GAP prod LINE crates/api/src/chain.rs:703  return None;
GAP prod LINE crates/api/src/chain.rs:705  Ok(None) => {}
GAP prod LINE crates/api/src/chain.rs:706  Err(why) => {
GAP prod LINE crates/api/src/chain.rs:707  faults.push(format!("{}: {why}", file.path().display()));
GAP prod LINE crates/api/src/chain.rs:708  return None;
GAP prod LINE crates/api/src/chain.rs:712  Err(refusal) => match refusal.kind {
GAP prod LINE crates/api/src/chain.rs:713  OpenRefusalKind::Absent => {}
GAP prod LINE crates/api/src/chain.rs:715  faults.push(refusal.message);
GAP prod LINE crates/api/src/chain.rs:716  return None;
GAP prod LINE crates/api/src/chain.rs:720  month = next_month(month)?;
GAP prod LINE crates/api/src/chain.rs:722  None
GAP prod LINE crates/api/src/chain.rs:752  Err(why) => return Lineage::Unreadable(format!("{}: {why}", path.display())),
GAP prod LINE crates/api/src/chain.rs:755  return Lineage::Unreadable(format!(
GAP prod LINE crates/api/src/chain.rs:756  "{}: {} bytes, past the {MAX_LINEAGE_BYTES} this route reads",
GAP prod LINE crates/api/src/chain.rs:757  path.display(),
GAP prod LINE crates/api/src/chain.rs:758  meta.len()
GAP prod LINE crates/api/src/chain.rs:759  ));
GAP prod LINE crates/api/src/chain.rs:772  Ok(Err(_)) => return Lineage::Unreadable(format!("{}: not UTF-8 text", path.display())),
GAP prod LINE crates/api/src/chain.rs:773  Err(why) => return Lineage::Unreadable(format!("{}: {why}", path.display())),
GAP prod LINE crates/api/src/chain.rs:798  return Err(format!(
GAP prod LINE crates/api/src/chain.rs:799  "at={} is not an instant any calendar can place",
GAP prod LINE crates/api/src/chain.rs:800  at.div_euclid(MICROS)
GAP prod LINE crates/api/src/chain.rs:801  ));
GAP prod LINE crates/api/src/chain.rs:819  OpenRefusalKind::Invalid => return Err(refusal.message),
GAP prod LINE crates/api/src/chain.rs:831  }
GAP prod LINE crates/api/src/chain.rs:833  return Ok(Spot::Unreadable);
GAP prod LINE crates/api/src/chain.rs:837  faults.push(format!(
GAP prod LINE crates/api/src/chain.rs:839  file.path().display(),
GAP prod LINE crates/api/src/chain.rs:840  at.div_euclid(MICROS)
GAP prod LINE crates/api/src/chain.rs:842  Spot::Unreadable
GAP prod LINE crates/api/src/chain.rs:843  }),
GAP prod LINE crates/api/src/chain.rs:871  return Some((
GAP prod LINE crates/api/src/chain.rs:872  "census_state_unknown",
GAP prod LINE crates/api/src/chain.rs:873  format!(
GAP prod LINE crates/api/src/chain.rs:874  "the import journal that says whether the census is behind would not read: {why}"
GAP prod LINE crates/api/src/chain.rs:875  ),
GAP prod LINE crates/api/src/chain.rs:876  ));
GAP prod LINE crates/api/src/chain.rs:917  return bad_request("no census row for this feed");
GAP prod LINE crates/api/src/chain.rs:1006  Err(why) => return bad_request(&why),
GAP prod LINE crates/api/src/chain.rs:1056  Lineage::Unreadable(why) => ("unreadable", 0, 0, Some(why.as_str())),
GAP prod LINE crates/api/src/chain.rs:1164  continue;
GAP prod LINE crates/api/src/chain.rs:1205  Err(why) => not_admitted("a chain read", &why),
GAP prod BRANCH crates/api/src/chain.rs:404:8 True  if stored.is_empty() {
GAP prod BRANCH crates/api/src/chain.rs:530:17 True  pull::expiry_alias::AliasRecord::parse(line).is_some()
GAP prod BRANCH crates/api/src/chain.rs:584:13 False  let Some(days) = self.adjusted.get(&underlying) else {
GAP prod BRANCH crates/api/src/chain.rs:603:35 False  if !before && !names(renamed, key) {
GAP prod BRANCH crates/api/src/chain.rs:638:29 True  let (year, number) = if month.month() == 12 {
GAP prod BRANCH crates/api/src/chain.rs:638:29 False  let (year, number) = if month.month() == 12 {
GAP prod BRANCH crates/api/src/chain.rs:672:11 False  while month <= last {
GAP prod BRANCH crates/api/src/chain.rs:689:24 False  if at < file.header().n_valid {
GAP prod BRANCH crates/api/src/chain.rs:696:37 False  Ok(Some(ts)) if ts >= from => return Some(ts),
GAP prod BRANCH crates/api/src/chain.rs:751:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Lineage::Absent,
GAP prod BRANCH crates/api/src/chain.rs:754:8 True  if meta.len() > MAX_LINEAGE_BYTES {
GAP prod BRANCH crates/api/src/chain.rs:764:12 False  && *was_path == path
GAP prod BRANCH crates/api/src/chain.rs:765:12 False  && *len == meta.len()
GAP prod BRANCH crates/api/src/chain.rs:766:12 False  && *was_modified == modified
GAP prod BRANCH crates/api/src/chain.rs:776:12 False  if let Some(modified) = modified {
GAP prod BRANCH crates/api/src/chain.rs:797:13 False  let Some(month) = month_of(at) else {
GAP prod BRANCH crates/api/src/chain.rs:803:12 False  if !self.opened.iter().any(|(held, _)| *held == month) {
GAP prod BRANCH crates/api/src/chain.rs:832:13 False  let Some((_, opened)) = self.opened.iter().find(|(held, _)| *held == month) else {
GAP prod BRANCH crates/api/src/chain.rs:870:16 True  if let Some(why) = &self.journal_unread {
GAP prod BRANCH crates/api/src/chain.rs:916:9 False  let Some(census) = now.census() else {
GAP prod BRANCH crates/api/src/chain.rs:971:26 False  let leg = if call { "CE" } else { "PE" };
GAP prod BRANCH crates/api/src/chain.rs:1051:16 False  if index.unread > 0 { "partial" } else { "read" },
GAP prod BRANCH crates/api/src/chain.rs:1163:17 False  let Some(strike) = strikes.get(n) else {
GAP prod BRANCH crates/api/src/constituents.rs:533:16 False  if let Some(id) = id
GAP prod BRANCH crates/api/src/constituents.rs:534:20 False  && let Some(slot) = self.per_vendor.get_mut(vendor as usize)
GAP test BRANCH crates/api/src/constituents.rs:1619:26 False  [inside a closure whose false side does not fail the check]  .any(|l| l.contains("NO ISIN") && l.contains("DUMMYINXGN")),
GAP test BRANCH crates/api/src/constituents.rs:1625:26 False  [inside a closure whose false side does not fail the check]  .any(|l| l.contains("NO ISIN") && l.contains("not a legal symbol")),
GAP prod LINE crates/api/src/credential_law.rs:100  pub const fn of_secret(kind: pull::secret::SecretError, why: String) -> Self {
GAP prod LINE crates/api/src/credential_law.rs:101  match kind {
GAP prod LINE crates/api/src/credential_law.rs:102  pull::secret::SecretError::Unreachable => Self::transport(why),
GAP prod LINE crates/api/src/credential_law.rs:103  _ => Self::configuration(why),
GAP prod LINE crates/api/src/credential_law.rs:105  }
GAP prod LINE crates/api/src/credential_law.rs:167  Self::Ssm => crate::server::credentialed_source(feed, &spec).await,
GAP test LINE crates/api/src/credential_law.rs:230  Err(Unreadable::configuration("the script is empty".to_owned()))
GAP test LINE crates/api/src/credential_law.rs:231  }),
GAP prod BRANCH crates/api/src/credential_law.rs:418:28 False  if let Some(rejected) = rejected
GAP prod BRANCH crates/api/src/credential_law.rs:419:28 False  && !self.is_dead(rejected)
GAP test LINE crates/api/src/credential_law_tests.rs:56  });
GAP test LINE crates/api/src/credential_law_tests.rs:775  });
GAP test BRANCH crates/api/src/credential_law_tests.rs:207:13 False  record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/credential_law_tests.rs:208:20 False  && crate::emitted::says(record, "field", "access-token")
GAP test BRANCH crates/api/src/credential_law_tests.rs:390:9 False  let autopilot::Next::Halt { reason } = state.observe(&autopilot::outcome_of(&run, false, None))
GAP prod LINE crates/api/src/dayflags.rs:356  format!("contract={contract_text:?} is not a contract segment this store can name")
GAP prod LINE crates/api/src/dayflags.rs:357  })?)
GAP prod LINE crates/api/src/dayflags.rs:389  continue;
GAP prod LINE crates/api/src/dayflags.rs:753  continue;
GAP prod LINE crates/api/src/dayflags.rs:768  continue;
GAP prod LINE crates/api/src/dayflags.rs:792  }
GAP prod LINE crates/api/src/dayflags.rs:915  continue;
GAP prod LINE crates/api/src/dayflags.rs:1072  Err(why) => json!({ "present": true, "records": [], "why": why }),
GAP prod LINE crates/api/src/dayflags.rs:1261  }
GAP prod LINE crates/api/src/dayflags.rs:1272  continue;
GAP prod LINE crates/api/src/dayflags.rs:1309  }
GAP prod LINE crates/api/src/dayflags.rs:1395  pull::gdfl_cm::CmKind::Stocks
GAP prod LINE crates/api/src/dayflags.rs:1543  }
GAP prod LINE crates/api/src/dayflags.rs:1564  }
GAP prod LINE crates/api/src/dayflags.rs:1796  Err(e) => return Err(format!("{}: {e}", path.display())),
GAP prod LINE crates/api/src/dayflags.rs:1835  }
GAP prod LINE crates/api/src/dayflags.rs:2188  return;
GAP prod LINE crates/api/src/dayflags.rs:2390  return;
GAP prod LINE crates/api/src/dayflags.rs:2433  return Err(format!("{name} appears twice"));
GAP prod LINE crates/api/src/dayflags.rs:2475  return Err(format!(
GAP prod LINE crates/api/src/dayflags.rs:2476  "v={} is not 1, the version this reader reads",
GAP prod LINE crates/api/src/dayflags.rs:2477  value(0)
GAP prod LINE crates/api/src/dayflags.rs:2478  ));
GAP prod LINE crates/api/src/dayflags.rs:2537  return Err(format!(
GAP prod LINE crates/api/src/dayflags.rs:2538  "v={} is not 1, the version this reader reads",
GAP prod LINE crates/api/src/dayflags.rs:2539  value(0)
GAP prod LINE crates/api/src/dayflags.rs:2540  ));
GAP prod LINE crates/api/src/dayflags.rs:2663  self.note.clone()
GAP prod LINE crates/api/src/dayflags.rs:2750  break;
GAP prod LINE crates/api/src/dayflags.rs:2764  break;
GAP prod LINE crates/api/src/dayflags.rs:2792  Self::Failed(why) => format!(
GAP prod LINE crates/api/src/dayflags.rs:2815  Self::Failed(why) => json!({ "present": null, "error": why }),
GAP prod LINE crates/api/src/dayflags.rs:2890  return false;
GAP prod LINE crates/api/src/dayflags.rs:2932  return None;
GAP prod LINE crates/api/src/dayflags.rs:2994  Err(e) => return Err(PullUnread::Failed(format!("{}: {e}", path.display()))),
GAP prod LINE crates/api/src/dayflags.rs:3057  Via::Walk { ref why } => json!({ "kind": "walk", "why": why }),
GAP prod LINE crates/api/src/dayflags.rs:3117  Err(why) => {
GAP prod LINE crates/api/src/dayflags.rs:3118  parsed.faults.note(number, "", &format!("{why:?}"));
GAP prod LINE crates/api/src/dayflags.rs:3119  continue;
GAP prod LINE crates/api/src/dayflags.rs:3126  parsed.faults.note(
GAP prod LINE crates/api/src/dayflags.rs:3127  number,
GAP prod LINE crates/api/src/dayflags.rs:3128  &record.source,
GAP prod LINE crates/api/src/dayflags.rs:3129  "the journal's index names this record under another source or kind",
GAP prod LINE crates/api/src/dayflags.rs:3131  continue;
GAP prod LINE crates/api/src/dayflags.rs:3155  Err(ref why) => {
GAP prod LINE crates/api/src/dayflags.rs:3156  rest.faults
GAP prod LINE crates/api/src/dayflags.rs:3157  .note(ordinal.saturating_add(1), "", &format!("{why:?}"));
GAP prod LINE crates/api/src/dayflags.rs:3158  continue;
GAP prod LINE crates/api/src/dayflags.rs:3195  *tail = PullTail::fresh(identity);
GAP prod LINE crates/api/src/dayflags.rs:3266  Err(e) => return Err(format!("{}: {e}", path.display())),
GAP prod LINE crates/api/src/dayflags.rs:3295  *tail = PullTail::fresh(identity);
GAP prod LINE crates/api/src/dayflags.rs:3355  return;
GAP prod LINE crates/api/src/dayflags.rs:3362  return;
GAP prod LINE crates/api/src/dayflags.rs:3365  parsed
GAP prod LINE crates/api/src/dayflags.rs:3366  .faults
GAP prod LINE crates/api/src/dayflags.rs:3367  .note(number, "", "a record kind this reader does not know");
GAP prod LINE crates/api/src/dayflags.rs:3368  return;
GAP prod LINE crates/api/src/dayflags.rs:3371  Err(why) => {
GAP prod LINE crates/api/src/dayflags.rs:3372  parsed.faults.note(number, "", &format!("{why:?}"));
GAP prod LINE crates/api/src/dayflags.rs:3373  if !member {
GAP prod LINE crates/api/src/dayflags.rs:3374  // A RUN THAT WILL NOT DECODE names no vendor: its members are
GAP prod LINE crates/api/src/dayflags.rs:3375  // counted apart, never given the previous run's vendor.
GAP prod LINE crates/api/src/dayflags.rs:3376  *vendor = None;
GAP prod LINE crates/api/src/dayflags.rs:3377  }
GAP prod LINE crates/api/src/dayflags.rs:3441  read.read_micros / 1_000,
GAP prod LINE crates/api/src/dayflags.rs:3442  match read.via {
GAP prod LINE crates/api/src/dayflags.rs:3443  Via::Walk { ref why } => why.as_str(),
GAP prod LINE crates/api/src/dayflags.rs:3444  Via::Index { .. } | Via::Absent => "the index covers only part of it",
GAP prod LINE crates/api/src/dayflags.rs:3476  return false;
GAP prod LINE crates/api/src/dayflags.rs:3547  return SeriesStart::NoneHeld;
GAP prod LINE crates/api/src/dayflags.rs:3565  return SeriesStart::NoneHeld;
GAP prod LINE crates/api/src/dayflags.rs:3725  Err(why) => return Opened::Refused(format!("{} {month:?}: {why}", inst.key())),
GAP prod LINE crates/api/src/dayflags.rs:3905  Err(why) => Opened::Refused(why.to_string()),
GAP prod LINE crates/api/src/dayflags.rs:4233  return Err(format!(
GAP prod LINE crates/api/src/dayflags.rs:4234  "it holds {n} records, more than the {MAX_DAY_BARS} a month of days can"
GAP prod LINE crates/api/src/dayflags.rs:4235  ));
GAP prod LINE crates/api/src/dayflags.rs:4320  Err(why) => Opened::Refused(why.to_string()),
GAP prod LINE crates/api/src/dayflags.rs:4325  Opened::Refused(why) => Err(why),
GAP prod LINE crates/api/src/dayflags.rs:4329  Err(why) => {
GAP prod LINE crates/api/src/dayflags.rs:4330  let what = format!("zerodha {} 1day {first}", inst.key());
GAP prod LINE crates/api/src/dayflags.rs:4331  let held = if month_refused(&what, &why, &mut notes, &mut faults) {
GAP prod LINE crates/api/src/dayflags.rs:4332  DayBarAt::BeingWritten
GAP prod LINE crates/api/src/dayflags.rs:4334  DayBarAt::Unreadable
GAP prod LINE crates/api/src/dayflags.rs:4336  per_day.extend(month_days.iter().map(|_| held));
GAP prod LINE crates/api/src/dayflags.rs:4718  Some(other) => (other, "unnamed_code"),
GAP prod LINE crates/api/src/dayflags.rs:4728  SpecialKind::SystemsOutage => "systems_outage",
GAP prod LINE crates/api/src/dayflags.rs:4888  _ => String::new(),
GAP prod LINE crates/api/src/dayflags.rs:5425  DayBarAt::Unreadable => json!({ "held": null, "why": "unreadable" }),
GAP prod LINE crates/api/src/dayflags.rs:5426  DayBarAt::BeingWritten => json!({ "held": null, "why": "being_written" }),
GAP prod LINE crates/api/src/dayflags.rs:5480  }
GAP prod LINE crates/api/src/dayflags.rs:5481  let mut kept: String = text.chars().take(MAX_PHRASE_CHARS).collect();
GAP prod LINE crates/api/src/dayflags.rs:5482  kept.push_str("...");
GAP prod LINE crates/api/src/dayflags.rs:5483  kept
GAP prod LINE crates/api/src/dayflags.rs:5634  (None, Some(other)) => format!(
GAP prod LINE crates/api/src/dayflags.rs:5637  tally.seen(),
GAP prod LINE crates/api/src/dayflags.rs:5638  tally.unseen()
GAP prod LINE crates/api/src/dayflags.rs:5720  "sourced"
GAP prod LINE crates/api/src/dayflags.rs:5906  None => format!("{earlier} refusal(s) logged by an earlier run are history"),
GAP prod LINE crates/api/src/dayflags.rs:5959  out.push(format!(
GAP prod LINE crates/api/src/dayflags.rs:5961  n("holes_5s"),
GAP prod LINE crates/api/src/dayflags.rs:5962  n("hole_max_s"),
GAP prod LINE crates/api/src/dayflags.rs:5963  line.clock("hole_max_at")
GAP prod LINE crates/api/src/dayflags.rs:5964  .map_or_else(|| "-".to_owned(), hms)
GAP prod LINE crates/api/src/dayflags.rs:6039  out.push(format!(
GAP prod LINE crates/api/src/dayflags.rs:6040  "{}: {} row(s) refused at import",
GAP prod LINE crates/api/src/dayflags.rs:6041  kind.as_str(),
GAP prod LINE crates/api/src/dayflags.rs:6042  n("rows_refused")
GAP prod LINE crates/api/src/dayflags.rs:6043  ));
GAP prod LINE crates/api/src/dayflags.rs:6046  out.push(format!(
GAP prod LINE crates/api/src/dayflags.rs:6047  "{}: {} LTQ=0 row(s) carry a new price (flagged; bars unchanged)",
GAP prod LINE crates/api/src/dayflags.rs:6048  kind.as_str(),
GAP prod LINE crates/api/src/dayflags.rs:6049  n("ltq0_new_ltp_rows")
GAP prod LINE crates/api/src/dayflags.rs:6050  ));
GAP prod LINE crates/api/src/dayflags.rs:6089  "Zerodha withheld the day: a session of unmeasured length".to_owned()
GAP prod LINE crates/api/src/dayflags.rs:6096  other => format!("Zerodha ledger: {other}"),
GAP prod LINE crates/api/src/dayflags.rs:6119  format!(
GAP prod LINE crates/api/src/dayflags.rs:6145  format!(" (and {more} more record(s))")
GAP prod LINE crates/api/src/dayflags.rs:6246  Held::Unreadable => out.push(format!("{}'s month could not be read", name(peer))),
GAP prod LINE crates/api/src/dayflags.rs:6291  Err(why) => json!({ "route": "/imports.json?feed=gdfl", "journal": null, "error": why }),
GAP prod LINE crates/api/src/dayflags.rs:6310  Via::Walk { ref why } => json!({
GAP prod LINE crates/api/src/dayflags.rs:6311  "kind": "walk",
GAP prod LINE crates/api/src/dayflags.rs:6312  "why": why,
GAP prod LINE crates/api/src/dayflags.rs:6313  "refusals_of": "every instrument",
GAP prod LINE crates/api/src/dayflags.rs:6352  Some(Err(unread @ PullUnread::Failed(_))) => Some(unread.note()),
GAP prod LINE crates/api/src/dayflags.rs:6369  notes.push(format!(
GAP prod LINE crates/api/src/dayflags.rs:6370  "The import journal could not be read, so every record's import is null: {why}"
GAP prod LINE crates/api/src/dayflags.rs:6371  ));
GAP prod LINE crates/api/src/dayflags.rs:6387  Some(Err(why)) => notes.push(format!(
GAP test LINE crates/api/src/dayflags.rs:8937  Err(PullUnread::Failed(why)) => panic!("the journal is there: {why}"),
GAP test LINE crates/api/src/dayflags.rs:8938  Ok(read) => panic!(
GAP test LINE crates/api/src/dayflags.rs:9474  Err(other) => panic!("the month's time index: {other}"),
GAP ignored LINE crates/api/src/dayflags.rs:9741  fn the_real_days_are_labelled_and_the_answer_is_timed() {
GAP ignored LINE crates/api/src/dayflags.rs:9742  let root = PathBuf::from(
GAP ignored LINE crates/api/src/dayflags.rs:9743  std::env::var_os("BRUTEX_DAYFLAGS_STORE")
GAP ignored LINE crates/api/src/dayflags.rs:9744  .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
GAP ignored LINE crates/api/src/dayflags.rs:9746  let get = |instrument: &str, on: &str| -> Value {
GAP ignored LINE crates/api/src/dayflags.rs:9747  let got = answer(
GAP ignored LINE crates/api/src/dayflags.rs:9748  &root,
GAP ignored LINE crates/api/src/dayflags.rs:9749  &ask(&format!(
GAP ignored LINE crates/api/src/dayflags.rs:9750  "feed=gdfl&instrument={instrument}&from={on}&to={on}"
GAP ignored LINE crates/api/src/dayflags.rs:9751  )),
GAP ignored LINE crates/api/src/dayflags.rs:9753  let rec = record(&got, on).clone();
GAP ignored LINE crates/api/src/dayflags.rs:9754  println!(
GAP ignored LINE crates/api/src/dayflags.rs:9756  rec["held"]["bars"],
GAP ignored LINE crates/api/src/dayflags.rs:9757  rec["calendar"]["code"],
GAP ignored LINE crates/api/src/dayflags.rs:9758  rec["sampled"],
GAP ignored LINE crates/api/src/dayflags.rs:9759  rec["events"]["halt"]["held"],
GAP ignored LINE crates/api/src/dayflags.rs:9760  rec["degraded"],
GAP ignored LINE crates/api/src/dayflags.rs:9761  rec["says"]
GAP ignored LINE crates/api/src/dayflags.rs:9763  rec
GAP ignored LINE crates/api/src/dayflags.rs:9764  };
GAP ignored LINE crates/api/src/dayflags.rs:9765  let sampled_day = [
GAP ignored LINE crates/api/src/dayflags.rs:9766  "NSE-INDEX-NIFTY",
GAP ignored LINE crates/api/src/dayflags.rs:9767  "NSE-INDEX-BANKNIFTY",
GAP ignored LINE crates/api/src/dayflags.rs:9768  "NSE-FNO-NIFTY-2019-05-30-1190000-CE",
GAP ignored LINE crates/api/src/dayflags.rs:9769  "NSE-FNO-NIFTY-2019-05-30-1300000-PE",
GAP ignored LINE crates/api/src/dayflags.rs:9770  "NSE-CASH-RELIANCE",
GAP ignored LINE crates/api/src/dayflags.rs:9771  ]
GAP ignored LINE crates/api/src/dayflags.rs:9772  .map(|instrument| get(instrument, "2019-05-30"));
GAP ignored LINE crates/api/src/dayflags.rs:9773  let ordinary = get("NSE-INDEX-NIFTY", "2019-05-29");
GAP ignored LINE crates/api/src/dayflags.rs:9774  let halts = [
GAP ignored LINE crates/api/src/dayflags.rs:9775  ("NSE-INDEX-NIFTY", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9776  ("NSE-INDEX-NIFTY", "2020-03-23"),
GAP ignored LINE crates/api/src/dayflags.rs:9777  ("NSE-INDEX-BANKNIFTY", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9778  ("NSE-FNO-NIFTY-2020-03-19-950000-PE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9779  ("NSE-FNO-NIFTY-2020-03-26-1250000-CE", "2020-03-23"),
GAP ignored LINE crates/api/src/dayflags.rs:9780  ("NSE-FNO-BANKNIFTY-2020-03-19-2500000-PE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9781  ("NSE-CASH-RELIANCE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9782  ]
GAP ignored LINE crates/api/src/dayflags.rs:9783  .map(|(instrument, on)| get(instrument, on));
GAP ignored LINE crates/api/src/dayflags.rs:9784  for query in [
GAP ignored LINE crates/api/src/dayflags.rs:9785  "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9786  "feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9787  "feed=gdfl&instrument=NSE-FNO-NIFTY-2020-03-26-950000-PE&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9788  "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9789  ] {
GAP ignored LINE crates/api/src/dayflags.rs:9790  let asked = ask(query);
GAP ignored LINE crates/api/src/dayflags.rs:9791  let _warm = answer(&root, &asked);
GAP ignored LINE crates/api/src/dayflags.rs:9792  let mut micros: Vec<u128> = (0..200)
GAP ignored LINE crates/api/src/dayflags.rs:9793  .map(|_| {
GAP ignored LINE crates/api/src/dayflags.rs:9794  let at = std::time::Instant::now();
GAP ignored LINE crates/api/src/dayflags.rs:9795  let got = answer(&root, &asked);
GAP ignored LINE crates/api/src/dayflags.rs:9796  let took = at.elapsed().as_micros();
GAP ignored LINE crates/api/src/dayflags.rs:9797  assert_eq!(got["days"].as_array().map(Vec::len), Some(91), "{query}");
GAP ignored LINE crates/api/src/dayflags.rs:9798  took
GAP ignored LINE crates/api/src/dayflags.rs:9799  })
GAP ignored LINE crates/api/src/dayflags.rs:9800  .collect();
GAP ignored LINE crates/api/src/dayflags.rs:9801  micros.sort_unstable();
GAP ignored LINE crates/api/src/dayflags.rs:9802  println!(
GAP ignored LINE crates/api/src/dayflags.rs:9804  micros[99], micros[197], micros[199]
GAP ignored LINE crates/api/src/dayflags.rs:9809  for rec in &sampled_day[..2] {
GAP ignored LINE crates/api/src/dayflags.rs:9810  assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9811  assert_eq!(rec["sampled"]["of"], "own", "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9812  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9813  says(rec).iter().any(|s| s.contains("minute-sampled")),
GAP ignored LINE crates/api/src/dayflags.rs:9816  assert!(!rec["degraded"].as_array().unwrap().is_empty(), "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9818  for rec in &sampled_day[2..4] {
GAP ignored LINE crates/api/src/dayflags.rs:9819  assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9820  assert_eq!(rec["sampled"]["of"], "underlying", "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9824  assert_eq!(
GAP ignored LINE crates/api/src/dayflags.rs:9825  sampled_day[4]["sampled"]["of"], "market",
GAP ignored LINE crates/api/src/dayflags.rs:9827  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9829  assert_eq!(
GAP ignored LINE crates/api/src/dayflags.rs:9830  sampled_day[4]["sampled"]["is"], true,
GAP ignored LINE crates/api/src/dayflags.rs:9832  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9834  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9835  sampled_day[4]["sampled"]["first_held"].is_string(),
GAP ignored LINE crates/api/src/dayflags.rs:9837  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9839  assert_eq!(ordinary["sampled"]["is"], false, "{ordinary:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9842  for rec in &halts {
GAP ignored LINE crates/api/src/dayflags.rs:9843  assert!(rec["events"]["halt"]["held"]["bars"].is_u64(), "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9844  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9845  says(rec).iter().any(|s| s.contains("inside the halt")),
GAP ignored LINE crates/api/src/dayflags.rs:9849  }
GAP prod BRANCH crates/api/src/dayflags.rs:388:13 False  let (Some(head), Some(tail)) = (rest.get(..at), rest.get(at.saturating_add(1)..)) else {
GAP prod BRANCH crates/api/src/dayflags.rs:392:20 False  && let Some(contract) = Contract::parse(tail)
GAP prod BRANCH crates/api/src/dayflags.rs:410:12 False  && dash(4)
GAP prod BRANCH crates/api/src/dayflags.rs:411:12 False  && digits(5, 7)
GAP prod BRANCH crates/api/src/dayflags.rs:412:12 False  && dash(7)
GAP prod BRANCH crates/api/src/dayflags.rs:413:12 False  && digits(8, 10)
GAP prod BRANCH crates/api/src/dayflags.rs:414:12 False  && dash(10)
GAP prod BRANCH crates/api/src/dayflags.rs:752:13 False  let Some(fields) = record.as_object_mut() else {
GAP prod BRANCH crates/api/src/dayflags.rs:763:13 False  let Some(list) = fields
GAP prod BRANCH crates/api/src/dayflags.rs:772:20 False  if let Some(object) = note.as_object_mut() {
GAP prod BRANCH crates/api/src/dayflags.rs:914:12 True  if line.starts_with("alias ") {
GAP prod BRANCH crates/api/src/dayflags.rs:1017:12 False  && *kept == stamp
GAP prod BRANCH crates/api/src/dayflags.rs:1247:12 False  if page == 0 {
GAP prod BRANCH crates/api/src/dayflags.rs:1268:17 False  let (Some(day), Some(filter)) = (
GAP prod BRANCH crates/api/src/dayflags.rs:1278:20 False  if !ids.contains(&run) {
GAP prod BRANCH crates/api/src/dayflags.rs:1307:12 False  if u64::try_from(page).unwrap_or(u64::MAX) >= pages {
GAP prod BRANCH crates/api/src/dayflags.rs:1387:8 True  if about.is_empty() || decisive.into_iter().any(|word| word == about) {
GAP prod BRANCH crates/api/src/dayflags.rs:1392:27 False  let tree = if inst.segment == Segment::Index {
GAP prod BRANCH crates/api/src/dayflags.rs:1532:16 False  if later.insert(pair.clone()) {
GAP prod BRANCH crates/api/src/dayflags.rs:1539:24 False  if out.later_this_first.is_none() {
GAP prod BRANCH crates/api/src/dayflags.rs:1551:16 False  if out.superseded.len() < MAX_REFUSALS_NAMED {
GAP prod BRANCH crates/api/src/dayflags.rs:1575:16 False  if out.this.len() < MAX_REFUSALS_NAMED {
GAP prod BRANCH crates/api/src/dayflags.rs:1785:19 False  Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/dayflags.rs:1809:37 True  if tail.identity != identity || len < tail.upto {
GAP prod BRANCH crates/api/src/dayflags.rs:1823:16 False  if let Some(end) = whole_lines_end(&fresh) {
GAP prod BRANCH crates/api/src/dayflags.rs:1867:31 True  if parts.len() != want || parts.iter().any(|part| part.len() != 2) {
GAP prod BRANCH crates/api/src/dayflags.rs:1910:12 False  if self.named.len() < MAX_NAMED_FAULTS {
GAP prod BRANCH crates/api/src/dayflags.rs:2187:12 True  if text.is_empty() {
GAP prod BRANCH crates/api/src/dayflags.rs:2218:32 False  ImportKind::Indices => !scope.is_empty() && scope != "*" && !scope.contains('='),
GAP prod BRANCH crates/api/src/dayflags.rs:2389:12 True  if text.is_empty() {
GAP prod BRANCH crates/api/src/dayflags.rs:2430:24 False  && let Some(slot) = found.get_mut(at)
GAP prod BRANCH crates/api/src/dayflags.rs:2431:20 True  && slot.replace(value).is_some()
GAP prod BRANCH crates/api/src/dayflags.rs:2454:23 False  .filter(|key| !key.is_empty() && !key.contains('='))
GAP prod BRANCH crates/api/src/dayflags.rs:2474:8 True  if value(0) != "1" {
GAP prod BRANCH crates/api/src/dayflags.rs:2490:8 True  if why.is_empty() || !why.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
GAP prod BRANCH crates/api/src/dayflags.rs:2522:23 False  .filter(|key| !key.is_empty() && !key.contains('='))
GAP prod BRANCH crates/api/src/dayflags.rs:2536:8 True  if value(0) != "1" {
GAP prod BRANCH crates/api/src/dayflags.rs:2660:12 False  if self.cut {
GAP prod BRANCH crates/api/src/dayflags.rs:2678:13 False  self.from_days <= day && day <= self.to_days
GAP prod BRANCH crates/api/src/dayflags.rs:2749:16 True  if buffer.is_empty() {
GAP prod BRANCH crates/api/src/dayflags.rs:2763:16 True  if kept < got {
GAP prod BRANCH crates/api/src/dayflags.rs:2889:9 False  let Some(note) = image.get(PULL_NOTE_BYTES) else {
GAP prod BRANCH crates/api/src/dayflags.rs:2931:26 False  if !(whole_window || note.contains("refused") && names_day.is_some()) {
GAP prod BRANCH crates/api/src/dayflags.rs:2931:54 False  if !(whole_window || note.contains("refused") && names_day.is_some()) {
GAP prod BRANCH crates/api/src/dayflags.rs:2981:19 False  Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/dayflags.rs:3125:12 True  if record.kind != crate::audit::Kind::MemberFailure || record.source != symbol {
GAP prod BRANCH crates/api/src/dayflags.rs:3125:64 True  if record.kind != crate::audit::Kind::MemberFailure || record.source != symbol {
GAP prod BRANCH crates/api/src/dayflags.rs:3137:16 False  if let Some(refusal) = pull_refusal(number, &record) {
GAP prod BRANCH crates/api/src/dayflags.rs:3163:16 False  if let Some(found) = rest.by_symbol.remove(symbol) {
GAP prod BRANCH crates/api/src/dayflags.rs:3194:12 True  if tail.identity != identity || whole < tail.upto {
GAP prod BRANCH crates/api/src/dayflags.rs:3194:41 True  if tail.identity != identity || whole < tail.upto {
GAP prod BRANCH crates/api/src/dayflags.rs:3200:16 False  && whole > tail.upto
GAP prod BRANCH crates/api/src/dayflags.rs:3219:12 False  if let Some(tail) = held.get_mut(path)
GAP prod BRANCH crates/api/src/dayflags.rs:3220:12 False  && tail.identity == walk.identity
GAP prod BRANCH crates/api/src/dayflags.rs:3259:19 False  Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/dayflags.rs:3294:12 True  if tail.identity != identity || whole < tail.upto {
GAP prod BRANCH crates/api/src/dayflags.rs:3294:41 True  if tail.identity != identity || whole < tail.upto {
GAP prod BRANCH crates/api/src/dayflags.rs:3320:16 False  if let Some(tail) = held.get_mut(&path)
GAP prod BRANCH crates/api/src/dayflags.rs:3321:16 False  && read.is_ok()
GAP prod BRANCH crates/api/src/dayflags.rs:3322:16 False  && tail.identity == walk.identity
GAP prod BRANCH crates/api/src/dayflags.rs:3323:16 False  && tail.upto <= walk.upto
GAP prod BRANCH crates/api/src/dayflags.rs:3341:16 False  if let Some(tail) = pull_tails()
GAP prod BRANCH crates/api/src/dayflags.rs:3354:9 False  let Ok(image) = <&[u8; crate::audit::RECORD_LEN]>::try_from(image) else {
GAP prod BRANCH crates/api/src/dayflags.rs:3359:18 True  if member && !may_refuse(image) {
GAP prod BRANCH crates/api/src/dayflags.rs:3364:19 True  if !member && kind != Some(crate::audit::Kind::Run.code()) {
GAP prod BRANCH crates/api/src/dayflags.rs:3373:16 True  if !member {
GAP prod BRANCH crates/api/src/dayflags.rs:3373:16 False  if !member {
GAP prod BRANCH crates/api/src/dayflags.rs:3387:20 False  if let Some(refusal) = pull_refusal(number, &record) {
GAP prod BRANCH crates/api/src/dayflags.rs:3436:27 True  Some(Ok(read)) if read.read_now > FIRST_WALK_RECORDS => vec![format!(
GAP prod BRANCH crates/api/src/dayflags.rs:3466:9 False  let Ok(path) = StorePath::new(PathParts {
GAP prod BRANCH crates/api/src/dayflags.rs:3546:9 False  let Ok(month) = page_first.year_month() else {
GAP prod BRANCH crates/api/src/dayflags.rs:3563:9 False  let Opened::File(file) = open_month(root, Vendor::Zerodha, inst, Timeframe::DAY_1, first)
GAP prod BRANCH crates/api/src/dayflags.rs:3732:12 False  && inst.contract.is_some()
GAP prod BRANCH crates/api/src/dayflags.rs:3739:59 False  Err(StoreError::Missing { path: missing, .. }) if missing == path.to_path_buf(root) => {
GAP prod BRANCH crates/api/src/dayflags.rs:4232:8 True  if n > MAX_DAY_BARS {
GAP prod BRANCH crates/api/src/dayflags.rs:4331:31 True  let held = if month_refused(&what, &why, &mut notes, &mut faults) {
GAP prod BRANCH crates/api/src/dayflags.rs:4331:31 False  let held = if month_refused(&what, &why, &mut notes, &mut faults) {
GAP prod BRANCH crates/api/src/dayflags.rs:4358:16 False  && let Some(first) = days.first()
GAP prod BRANCH crates/api/src/dayflags.rs:4590:8 True  if secs == 0 || 60 / secs <= pull::gdfl_import::quality::SAMPLED_SS_DISTINCT {
GAP prod BRANCH crates/api/src/dayflags.rs:4830:28 False  "standing": if halt.evidence.is_suspected() { "suspected" } else { "sourced" },
GAP prod BRANCH crates/api/src/dayflags.rs:4947:25 False  && (zero.saturating_mul(2) >= traded || lead.saturating_mul(2) >= traded)
GAP prod BRANCH crates/api/src/dayflags.rs:4947:61 True  && (zero.saturating_mul(2) >= traded || lead.saturating_mul(2) >= traded)
GAP prod BRANCH crates/api/src/dayflags.rs:4947:61 False  && (zero.saturating_mul(2) >= traded || lead.saturating_mul(2) >= traded)
GAP prod BRANCH crates/api/src/dayflags.rs:4989:47 True  if day_bar.is_some_and(|b| b.open <= 0 || b.high <= 0 || b.low <= 0 || b.close <= 0) {
GAP prod BRANCH crates/api/src/dayflags.rs:4989:62 True  if day_bar.is_some_and(|b| b.open <= 0 || b.high <= 0 || b.low <= 0 || b.close <= 0) {
GAP prod BRANCH crates/api/src/dayflags.rs:4997:12 False  && bar.close > 0
GAP prod BRANCH crates/api/src/dayflags.rs:4998:12 False  && prev > 0
GAP prod BRANCH crates/api/src/dayflags.rs:5478:8 False  if text.chars().count() <= MAX_PHRASE_CHARS {
GAP prod BRANCH crates/api/src/dayflags.rs:5562:16 False  if missing == 1 { "is" } else { "are" },
GAP prod BRANCH crates/api/src/dayflags.rs:5631:32 False  (None, Some(other)) if tally.whole() => {
GAP prod BRANCH crates/api/src/dayflags.rs:5640:25 True  (None, None) if tally.whole() => head,
GAP prod BRANCH crates/api/src/dayflags.rs:5656:20 False  if let Some(says) = degraded.get("says").and_then(Value::as_str) {
GAP prod BRANCH crates/api/src/dayflags.rs:5717:31 False  let standing = if halt.evidence.is_suspected() {
GAP prod BRANCH crates/api/src/dayflags.rs:5958:16 True  if n("holes_5s") > 0 {
GAP prod BRANCH crates/api/src/dayflags.rs:6038:16 True  if n("rows_refused") > 0 {
GAP prod BRANCH crates/api/src/dayflags.rs:6045:16 True  if n("ltq0_new_ltp_rows") > 0 {
GAP prod BRANCH crates/api/src/dayflags.rs:6112:45 False  (Some(refusal), Some(first)) if self.journal_front == Some(true) => out.push(format!(
GAP prod BRANCH crates/api/src/dayflags.rs:6118:29 True  out.push(if self.journal_present == Some(true) {
GAP prod BRANCH crates/api/src/dayflags.rs:6144:20 True  if more > 0 {
GAP prod BRANCH crates/api/src/dayflags.rs:6368:12 True  if let Some(Err(why)) = journal {
GAP test BRANCH crates/api/src/dayflags.rs:9473:26 False  Err(gone) if gone.kind() == std::io::ErrorKind::NotFound => {}
GAP test BRANCH crates/api/src/dayflags_filter_tests.rs:536:29 False  [inside a closure whose false side does not fail the check]  said.iter().any(|s| s.starts_with("incomplete at import")
GAP test BRANCH crates/api/src/dayflags_filter_tests.rs:693:12 False  if n == 1 { "is" } else { "are" }
GAP test BRANCH crates/api/src/dayflags_filter_tests.rs:704:22 True  [inside a closure whose false side does not fail the check]  .any(|s| s.contains(ABC) || s.contains("none about this instrument")),
GAP prod LINE crates/api/src/dayflags_rules.rs:320  return Ok(kept);
GAP prod BRANCH crates/api/src/dayflags_rules.rs:319:16 True  if let Some(kept) = flags().get(&key) {
GAP prod BRANCH crates/api/src/dayflags_rules.rs:598:21 False  self.share
GAP prod BRANCH crates/api/src/dayflags_rules.rs:602:28 False  && !self.refused.contains_key(&day.days_from_epoch())
GAP prod BRANCH crates/api/src/dayflags_rules.rs:1190:20 False  if let Some(through) = through {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2632  fn the_day_band_is_timed_cold_warm_concurrent_and_growing() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2633  day_band_timed([1, 10, 100], [20, 200, 60], 300);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2634  }
GAP test LINE crates/api/src/dayflags_rules_tests.rs:2818  _ => panic!("the month opens"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2902  fn the_cold_auction_era_share_page_is_timed() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2903  let root = std::path::PathBuf::from(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2904  std::env::var_os("BRUTEX_DAYFLAGS_STORE")
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2905  .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2907  cold_share_pages_timed(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2908  &root,
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2909  &[
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2910  ("NSE-CASH-DALBHARAT", "2026-08-10", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2911  ("NSE-CASH-RELIANCE", "2026-08-03", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2912  ],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2914  [20, 10],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2916  }
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3027  fn the_day_band_pages_for_a_profiler() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3028  profiled_pages("r3-13-profile", 10);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3029  }
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1526:60 False  [inside a closure whose false side does not fail the check]  says(record(&got, "2024-10-03")).iter().any(|said| said
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1904:9 False  [an operand followed by ||: neither side fails the check by itself]  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1904:39 True  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1904:39 False  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3171:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3199:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3476:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3725:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3769:13 False  let super::super::Opened::File(file) =
GAP prod LINE crates/api/src/detail.rs:332  left.stay();
GAP prod LINE crates/api/src/detail.rs:333  return None;
GAP prod LINE crates/api/src/detail.rs:739  pub async fn run_scan<T, F>(work: F) -> Result<T, RunError>
GAP prod LINE crates/api/src/detail.rs:740  where
GAP prod LINE crates/api/src/detail.rs:741  T: Send + 'static,
GAP prod LINE crates/api/src/detail.rs:742  F: FnOnce() -> T + Send + 'static,
GAP prod LINE crates/api/src/detail.rs:743  {
GAP prod LINE crates/api/src/detail.rs:744  let permit = SCAN.take().await.ok_or(RunError::Saturated)?;
GAP prod LINE crates/api/src/detail.rs:745  admitted(permit, work).await
GAP prod LINE crates/api/src/detail.rs:746  }
GAP prod LINE crates/api/src/detail.rs:942  Err(why) => {
GAP prod LINE crates/api/src/detail.rs:943  let said = serde_json::json!({
GAP prod LINE crates/api/src/detail.rs:944  "error": format!("this 503's own body did not read: {why}"),
GAP prod LINE crates/api/src/detail.rs:946  return (
GAP prod LINE crates/api/src/detail.rs:947  axum::response::Response::from_parts(
GAP prod LINE crates/api/src/detail.rs:948  parts,
GAP prod LINE crates/api/src/detail.rs:949  axum::body::Body::from(said.to_string()),
GAP prod LINE crates/api/src/detail.rs:950  ),
GAP prod LINE crates/api/src/detail.rs:951  false,
GAP prod LINE crates/api/src/detail.rs:952  );
GAP prod LINE crates/api/src/detail.rs:1305  Err(why) => {
GAP prod LINE crates/api/src/detail.rs:1306  return Err(format!(
GAP prod LINE crates/api/src/detail.rs:1307  "{name} at {} could not be measured before its bounded read: {why}",
GAP prod LINE crates/api/src/detail.rs:1308  path.display()
GAP prod LINE crates/api/src/detail.rs:1309  ));
GAP prod LINE crates/api/src/detail.rs:1685  fn default() -> Self {
GAP prod LINE crates/api/src/detail.rs:1686  Self::new()
GAP prod LINE crates/api/src/detail.rs:1687  }
GAP prod LINE crates/api/src/detail.rs:1715  )?
GAP prod LINE crates/api/src/detail.rs:1751  )??;
GAP test LINE crates/api/src/detail.rs:1927  asked += 1;
GAP test LINE crates/api/src/detail.rs:1928  current
GAP test LINE crates/api/src/detail.rs:1929  }),
GAP test LINE crates/api/src/detail.rs:2305  || {
GAP test LINE crates/api/src/detail.rs:2306  reopened = true;
GAP test LINE crates/api/src/detail.rs:2307  Ok(9)
GAP test LINE crates/api/src/detail.rs:2308  },
GAP test LINE crates/api/src/detail.rs:2344  |_| Ok(()),
GAP test LINE crates/api/src/detail.rs:2350  |_| Err("nothing was cached for this root".to_owned()),
GAP test LINE crates/api/src/detail.rs:2360  || Err("the handle is cached".to_owned()),
GAP test LINE crates/api/src/detail.rs:2416  |_| {
GAP test LINE crates/api/src/detail.rs:2417  refreshes += 1;
GAP test LINE crates/api/src/detail.rs:2418  Ok(())
GAP test LINE crates/api/src/detail.rs:2419  },
GAP test LINE crates/api/src/detail.rs:2431  || {
GAP test LINE crates/api/src/detail.rs:2432  opens += 1;
GAP test LINE crates/api/src/detail.rs:2433  Ok(99)
GAP test LINE crates/api/src/detail.rs:2434  },
GAP test LINE crates/api/src/detail.rs:2469  |_| {
GAP test LINE crates/api/src/detail.rs:2470  refreshes += 1;
GAP test LINE crates/api/src/detail.rs:2471  Ok(())
GAP test LINE crates/api/src/detail.rs:2472  },
GAP test LINE crates/api/src/detail.rs:2488  |_| {
GAP test LINE crates/api/src/detail.rs:2489  refreshes += 1;
GAP test LINE crates/api/src/detail.rs:2490  Ok(())
GAP test LINE crates/api/src/detail.rs:2491  },
GAP test LINE crates/api/src/detail.rs:2503  || {
GAP test LINE crates/api/src/detail.rs:2504  opens += 1;
GAP test LINE crates/api/src/detail.rs:2505  Ok(0)
GAP test LINE crates/api/src/detail.rs:2506  },
GAP prod BRANCH crates/api/src/detail.rs:331:13 False  let Some(clock) = self.clock() else {
GAP prod BRANCH crates/api/src/detail.rs:957:25 False  if being_written && !parts.headers.contains_key(axum::http::header::RETRY_AFTER) {
GAP prod BRANCH crates/api/src/detail.rs:1304:25 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
GAP prod BRANCH crates/api/src/detail.rs:1669:20 False  if held
GAP test BRANCH crates/api/src/detail.rs:2743:49 False  while !cancel.is_cancelled() && std::time::Instant::now() < deadline {
GAP test BRANCH crates/api/src/detail.rs:2773:20 True  while (pool.cancelled.load(Ordering::Relaxed) == before
GAP test BRANCH crates/api/src/detail.rs:2774:20 True  || pool.active.load(Ordering::Acquire) != 0)
GAP test BRANCH crates/api/src/detail.rs:2775:20 True  && std::time::Instant::now() < deadline
GAP test BRANCH crates/api/src/detail.rs:2775:20 False  && std::time::Instant::now() < deadline
GAP prod LINE crates/api/src/disk_route.rs:274  return Err(format!(
GAP prod LINE crates/api/src/disk_route.rs:275  "{} has no mount point to name: statfs answered {mount:?}",
GAP prod LINE crates/api/src/disk_route.rs:276  root.display()
GAP prod LINE crates/api/src/disk_route.rs:277  ));
GAP prod LINE crates/api/src/disk_route.rs:575  "not a regular file"
GAP prod LINE crates/api/src/disk_route.rs:670  "neither a directory nor a regular file"
GAP prod LINE crates/api/src/disk_route.rs:912  format!(
GAP prod LINE crates/api/src/disk_route.rs:914  render::json_string(&why)
GAP prod LINE crates/api/src/disk_route.rs:916  })
GAP prod LINE crates/api/src/disk_route.rs:966  let _fragment = lines.next();
GAP prod LINE crates/api/src/disk_route.rs:1056  let (_stamps, now) = vendor_census(site, vendor);
GAP prod LINE crates/api/src/disk_route.rs:1057  now.census().and_then(crate::census::VendorCensus::counters)
GAP prod LINE crates/api/src/disk_route.rs:1058  });
GAP prod LINE crates/api/src/disk_route.rs:1065  months
GAP prod LINE crates/api/src/disk_route.rs:1066  .saturating_mul(store::format::HEADER_LEN)
GAP prod LINE crates/api/src/disk_route.rs:1067  .saturating_add(rows.saturating_mul(store::format::RECORD_STRIDE))
GAP prod LINE crates/api/src/disk_route.rs:1068  });
GAP prod LINE crates/api/src/disk_route.rs:1163  out.push(',');
GAP prod LINE crates/api/src/disk_route.rs:1176  out.push(',');
GAP test LINE crates/api/src/disk_route.rs:1362  [read: the second operand of an || in the condition, never reached: not the failing arm]  || parsed["filesystem"]["error"].is_string(),
GAP test LINE crates/api/src/disk_route.rs:1650  }
GAP test LINE crates/api/src/disk_route.rs:1828  continue;
GAP test LINE crates/api/src/disk_route.rs:1838  assert_eq!(fs.used, used, "{}", root.display());
GAP ignored LINE crates/api/src/disk_route.rs:2069  fn the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50() {
GAP ignored LINE crates/api/src/disk_route.rs:2070  timed_against_the_child("disk-route-timing", 1_000, 1);
GAP ignored LINE crates/api/src/disk_route.rs:2071  }
GAP prod BRANCH crates/api/src/disk_route.rs:273:8 True  if !mount.starts_with('/') {
GAP prod BRANCH crates/api/src/disk_route.rs:547:12 False  if let Ok(meta) = std::fs::symlink_metadata(&receipt) {
GAP prod BRANCH crates/api/src/disk_route.rs:572:15 False  } else if meta.is_dir() {
GAP prod BRANCH crates/api/src/disk_route.rs:667:15 False  } else if meta.is_file() {
GAP prod BRANCH crates/api/src/disk_route.rs:965:8 True  if cut {
GAP prod BRANCH crates/api/src/disk_route.rs:1012:15 False  20 if is(16, b':') => number(17, 2),
GAP prod BRANCH crates/api/src/disk_route.rs:1034:5 False  is(4, b'-')
GAP prod BRANCH crates/api/src/disk_route.rs:1035:12 False  && is(7, b'-')
GAP prod BRANCH crates/api/src/disk_route.rs:1037:12 False  && is(13, b':')
GAP prod BRANCH crates/api/src/disk_route.rs:1038:12 False  && bytes.last() == Some(&b'Z')
GAP prod BRANCH crates/api/src/disk_route.rs:1042:12 False  && minute < 60
GAP prod BRANCH crates/api/src/disk_route.rs:1152:57 True  .filter(|(_, files, seal)| !files.is_empty() || seal.migrated || seal.fault.is_some())
GAP prod BRANCH crates/api/src/disk_route.rs:1162:12 True  if n > 0 {
GAP prod BRANCH crates/api/src/disk_route.rs:1175:12 True  if n > 0 {
GAP test BRANCH crates/api/src/disk_route.rs:1361:13 False  [read: the first operand of an || continued on the next line: not the failing arm]  parsed["filesystem"]["free_bytes"].as_u64().is_some()
GAP test BRANCH crates/api/src/disk_route.rs:1362:20 True  || parsed["filesystem"]["error"].is_string(),
GAP test BRANCH crates/api/src/disk_route.rs:1362:20 False  || parsed["filesystem"]["error"].is_string(),
GAP test BRANCH crates/api/src/disk_route.rs:1639:16 False  if (fs.total, fs.used, fs.free)
GAP test BRANCH crates/api/src/disk_route.rs:1827:20 True  if (fs.total, fs.free) != (total, free) {
GAP test BRANCH crates/api/src/disk_route.rs:1830:20 False  if apfs {
GAP prod LINE crates/api/src/expressionsearchjson.rs:44  return Err("search continuation requires its exact snapshot".to_owned());
GAP prod LINE crates/api/src/expressionsearchjson.rs:92  Ok(_) => refused(
GAP prod LINE crates/api/src/expressionsearchjson.rs:94  "search JSON exceeds its response admission; no prefix exposed",
GAP prod LINE crates/api/src/expressionsearchjson.rs:96  Err(why) => refused(StatusCode::SERVICE_UNAVAILABLE, &why),
GAP prod LINE crates/api/src/expressionsearchjson.rs:123  held.root == root
GAP prod LINE crates/api/src/expressionsearchjson.rs:124  && held.identity == asked.identity
GAP prod LINE crates/api/src/expressionsearchjson.rs:125  && held.reader.progress().checkpoint == Some(snapshot)
GAP prod LINE crates/api/src/expressionsearchjson.rs:126  })
GAP prod LINE crates/api/src/expressionsearchjson.rs:133  sessions.retain(|held| {
GAP prod LINE crates/api/src/expressionsearchjson.rs:134  held.root != root
GAP prod LINE crates/api/src/expressionsearchjson.rs:135  || held.identity != asked.identity
GAP prod LINE crates/api/src/expressionsearchjson.rs:136  || held.reader.progress().checkpoint != reader.progress().checkpoint
GAP prod LINE crates/api/src/expressionsearchjson.rs:137  });
GAP prod LINE crates/api/src/expressionsearchjson.rs:138  if sessions.len() == 8 {
GAP prod LINE crates/api/src/expressionsearchjson.rs:139  sessions.pop_front();
GAP prod LINE crates/api/src/expressionsearchjson.rs:140  }
GAP prod LINE crates/api/src/expressionsearchjson.rs:141  sessions.push_back(Session {
GAP prod LINE crates/api/src/expressionsearchjson.rs:142  root: root.to_path_buf(),
GAP prod LINE crates/api/src/expressionsearchjson.rs:143  identity: asked.identity,
GAP prod LINE crates/api/src/expressionsearchjson.rs:144  reader,
GAP prod LINE crates/api/src/expressionsearchjson.rs:145  });
GAP prod LINE crates/api/src/expressionsearchjson.rs:146  sessions.len() - 1
GAP prod LINE crates/api/src/expressionsearchjson.rs:148  let held = sessions
GAP prod LINE crates/api/src/expressionsearchjson.rs:149  .get_mut(index)
GAP prod LINE crates/api/src/expressionsearchjson.rs:150  .ok_or("search snapshot cache entry missing")?;
GAP prod LINE crates/api/src/expressionsearchjson.rs:151  let page = held
GAP prod LINE crates/api/src/expressionsearchjson.rs:152  .reader
GAP prod LINE crates/api/src/expressionsearchjson.rs:153  .page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)?;
GAP prod LINE crates/api/src/expressionsearchjson.rs:154  Ok(body(held.reader.progress(), &page, asked.limit))
GAP prod LINE crates/api/src/expressionsearchjson.rs:161  "exhausted-observed"
GAP prod LINE crates/api/src/expressionsearchjson.rs:164  } else if progress.checkpoint.is_some() {
GAP prod LINE crates/api/src/expressionsearchjson.rs:165  "paused-or-stopped"
GAP prod LINE crates/api/src/expressionsearchjson.rs:167  "awaiting-first-checkpoint"
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:43:32 True  if cursor.is_some() && snapshot.is_none() {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:64:41 False  (Some(sequence), Some(seal)) if sequence > 0 => Ok(Some(Anchor { sequence, seal })),
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:89:21 False  Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:123:17 True  held.root == root
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:123:17 False  held.root == root
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:124:24 True  && held.identity == asked.identity
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:124:24 False  && held.identity == asked.identity
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:129:13 True  let Some(reader) = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)?
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:134:13 True  held.root != root
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:134:13 False  held.root != root
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:135:20 True  || held.identity != asked.identity
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:135:20 False  || held.identity != asked.identity
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:138:12 True  if sessions.len() == 8 {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:138:12 False  if sessions.len() == 8 {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:160:20 True  let state = if progress.exhausted {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:162:15 False  } else if progress.writer_observed {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:164:15 True  } else if progress.checkpoint.is_some() {
GAP prod BRANCH crates/api/src/expressionsearchjson.rs:164:15 False  } else if progress.checkpoint.is_some() {
GAP prod LINE crates/api/src/facets.rs:396  if old.is_none() {
GAP prod LINE crates/api/src/facets.rs:397  under.unread_contracts = under.unread_contracts.saturating_add(1);
GAP prod LINE crates/api/src/facets.rs:398  }
GAP prod LINE crates/api/src/facets.rs:399  return;
GAP prod LINE crates/api/src/facets.rs:517  return Err(since);
GAP prod LINE crates/api/src/facets.rs:640  let fresh = whole(segment, slot);
GAP prod LINE crates/api/src/facets.rs:649  fresh
GAP prod LINE crates/api/src/facets.rs:655  }
GAP prod LINE crates/api/src/facets.rs:790  return None;
GAP prod LINE crates/api/src/facets.rs:794  Err(since) => return Some(since),
GAP prod LINE crates/api/src/facets.rs:818  Err(why) => {
GAP prod LINE crates/api/src/facets.rs:819  warn_line!(
GAP prod LINE crates/api/src/facets.rs:822  vendor.as_str(),
GAP prod LINE crates/api/src/facets.rs:823  segment.as_str()
GAP prod LINE crates/api/src/facets.rs:825  None
GAP prod LINE crates/api/src/facets.rs:919  return Ok((read(&held), Answered::Current));
GAP prod LINE crates/api/src/facets.rs:932  None => Err(ms_since(since)),
GAP test LINE crates/api/src/facets.rs:1039  return true;
GAP test LINE crates/api/src/facets.rs:1042  *displaced = Some(emptied(index, slot));
GAP test LINE crates/api/src/facets.rs:1043  return true;
GAP test LINE crates/api/src/facets.rs:1049  return false;
GAP prod LINE crates/api/src/facets.rs:1138  return true;
GAP prod LINE crates/api/src/facets.rs:1158  return true;
GAP prod LINE crates/api/src/facets.rs:1276  return fresh;
GAP prod LINE crates/api/src/facets.rs:1661  Err((code, body)) => (code, headers, body),
GAP prod LINE crates/api/src/facets.rs:1881  return Err((
GAP prod LINE crates/api/src/facets.rs:1882  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/facets.rs:1883  error_json(
GAP prod LINE crates/api/src/facets.rs:1884  "rows_unreadable",
GAP prod LINE crates/api/src/facets.rs:1885  &format!("contract {text:?} of the index does not parse back"),
GAP prod LINE crates/api/src/facets.rs:1886  ),
GAP prod LINE crates/api/src/facets.rs:1887  ));
GAP prod LINE crates/api/src/facets.rs:1894  continue;
GAP prod LINE crates/api/src/facets.rs:1943  Err(refusal) => return (Err(refusal), generation),
GAP prod LINE crates/api/src/facets.rs:1959  return (
GAP prod LINE crates/api/src/facets.rs:1960  Err((
GAP prod LINE crates/api/src/facets.rs:1961  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/facets.rs:1962  error_json(
GAP prod LINE crates/api/src/facets.rs:1963  "rows_incomplete",
GAP prod LINE crates/api/src/facets.rs:1964  "the rows this index can name do not sum to what the census holds for \
GAP prod LINE crates/api/src/facets.rs:1965  the selection (a key outside the index's reading); read the census \
GAP prod LINE crates/api/src/facets.rs:1966  whole from /store.json",
GAP prod LINE crates/api/src/facets.rs:1967  ),
GAP prod LINE crates/api/src/facets.rs:1968  )),
GAP prod LINE crates/api/src/facets.rs:1969  generation,
GAP prod LINE crates/api/src/facets.rs:1970  );
GAP prod LINE crates/api/src/facets.rs:1980  None => super::store_wire::compact(&[], &entries, asked.feed),
GAP prod LINE crates/api/src/facets.rs:1984  Err(why) => {
GAP prod LINE crates/api/src/facets.rs:1985  return (
GAP prod LINE crates/api/src/facets.rs:1986  Err((
GAP prod LINE crates/api/src/facets.rs:1987  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/facets.rs:1988  error_json("census_encoding_failed", &why),
GAP prod LINE crates/api/src/facets.rs:1989  )),
GAP prod LINE crates/api/src/facets.rs:1990  generation,
GAP prod LINE crates/api/src/facets.rs:1991  );
GAP prod LINE crates/api/src/facets.rs:2102  return Err((
GAP prod LINE crates/api/src/facets.rs:2103  axum::http::StatusCode::GONE,
GAP prod LINE crates/api/src/facets.rs:2104  error_json(
GAP prod LINE crates/api/src/facets.rs:2105  "since_too_old",
GAP prod LINE crates/api/src/facets.rs:2106  "this feed has no census for the segment, so no generation is held",
GAP prod LINE crates/api/src/facets.rs:2107  ),
GAP prod LINE crates/api/src/facets.rs:2108  ));
GAP prod LINE crates/api/src/facets.rs:2239  (Some(_), None) => None,
GAP prod LINE crates/api/src/facets.rs:2320  ContractId::Option { .. } | ContractId::Future => {}
GAP prod LINE crates/api/src/facets.rs:2417  key.contract
GAP prod LINE crates/api/src/facets.rs:2418  .and_then(|contract| contract_of(contract.as_str()))
GAP prod LINE crates/api/src/facets.rs:2419  .is_some_and(|(day, _)| day == wanted)
GAP prod LINE crates/api/src/facets.rs:2420  })
GAP prod LINE crates/api/src/facets.rs:2436  fn contracts_of(&self, id: &(Exchange, Symbol), expiry: Day) -> BTreeSet<ContractId> {
GAP prod LINE crates/api/src/facets.rs:2437  self.mine(id)
GAP prod LINE crates/api/src/facets.rs:2438  .filter_map(|key| {
GAP prod LINE crates/api/src/facets.rs:2439  key.contract
GAP prod LINE crates/api/src/facets.rs:2440  .and_then(|contract| contract_of(contract.as_str()))
GAP prod LINE crates/api/src/facets.rs:2441  })
GAP prod LINE crates/api/src/facets.rs:2442  .filter(|(day, _)| *day == expiry)
GAP prod LINE crates/api/src/facets.rs:2443  .map(|(_, contract)| contract)
GAP prod LINE crates/api/src/facets.rs:2444  .collect()
GAP prod LINE crates/api/src/facets.rs:2445  }
GAP prod BRANCH crates/api/src/facets.rs:395:13 False  let Some((expiry, id)) = contract_of(contract.as_str()) else {
GAP prod BRANCH crates/api/src/facets.rs:396:16 True  if old.is_none() {
GAP prod BRANCH crates/api/src/facets.rs:396:16 False  if old.is_none() {
GAP prod BRANCH crates/api/src/facets.rs:516:16 True  if let Some(since) = pending.since.filter(|_| pending.holders > 0) {
GAP prod BRANCH crates/api/src/facets.rs:626:12 False  if !current {
GAP prod BRANCH crates/api/src/facets.rs:648:20 True  if answers(site, vendor, &held, slot) {
GAP prod BRANCH crates/api/src/facets.rs:757:44 False  if part_of(slot, segment).is_none() && moved_off_lock(site, vendor, cell, segment, slot) {
GAP prod BRANCH crates/api/src/facets.rs:789:8 True  if unreadable(&target) {
GAP prod BRANCH crates/api/src/facets.rs:839:9 False  part_of(reflects, segment).is_some() || part_of(slot, segment).is_none()
GAP prod BRANCH crates/api/src/facets.rs:918:8 True  if answers(site, vendor, &held, slot) {
GAP test BRANCH crates/api/src/facets.rs:1034:8 True  if index
GAP test BRANCH crates/api/src/facets.rs:1041:8 True  if part_of(slot, segment).is_none() {
GAP test BRANCH crates/api/src/facets.rs:1048:9 False  let Some((part, was, changes)) = tail_of(&before, index.generation, segment, slot) else {
GAP prod BRANCH crates/api/src/facets.rs:1114:16 False  if let Some(now) = part.held(key) {
GAP prod BRANCH crates/api/src/facets.rs:1137:12 True  if answers(site, vendor, &held, slot) {
GAP prod BRANCH crates/api/src/facets.rs:1157:8 True  if answers(site, vendor, &held, slot) {
GAP prod BRANCH crates/api/src/facets.rs:1162:16 False  let same = held
GAP prod BRANCH crates/api/src/facets.rs:1167:8 False  if same {
GAP prod BRANCH crates/api/src/facets.rs:1275:9 False  let Some(part) = part_of(slot, segment) else {
GAP prod BRANCH crates/api/src/facets.rs:1282:16 False  if let Some(held) = part.held(key) {
GAP prod BRANCH crates/api/src/facets.rs:1880:17 False  let Some(contract) = brutex_core::instrument::Contract::parse(&text) else {
GAP prod BRANCH crates/api/src/facets.rs:1893:17 False  let Some((from, to)) = agg.span else {
GAP prod BRANCH crates/api/src/facets.rs:1958:8 True  if counted != sums {
GAP prod BRANCH crates/api/src/facets.rs:2097:17 False  let Some(part) = index
GAP prod BRANCH crates/api/src/facets.rs:2305:60 False  ContractId::Option { strike, call } if keep(contract) => {
GAP prod BRANCH crates/api/src/facets.rs:2307:32 False  if let Some(side) = sides.get_mut(usize::from(!call)) {
GAP prod BRANCH crates/api/src/facets.rs:2311:43 False  ContractId::Future if keep(contract) => {
GAP prod BRANCH crates/api/src/facets.rs:2399:16 False  if key.segment == segment && seen.insert(*key) {
GAP prod LINE crates/api/src/fault_log.rs:386  end -= 1;
GAP prod LINE crates/api/src/fault_log.rs:387  }
GAP prod BRANCH crates/api/src/fault_log.rs:135:14 False  ) if a == b && c == d => Cause::LostSidecarThrough(*block),
GAP prod BRANCH crates/api/src/fault_log.rs:135:24 False  ) if a == b && c == d => Cause::LostSidecarThrough(*block),
GAP prod BRANCH crates/api/src/fault_log.rs:186:34 False  self.named.is_empty() && self.unnamed == 0 && self.open.is_none()
GAP prod BRANCH crates/api/src/fault_log.rs:320:76 False  (StoreError::ChecksumsMissing { block, .. }, Some(through)) if through != *block => {
GAP prod BRANCH crates/api/src/fault_log.rs:385:11 True  while !sentence.is_char_boundary(end) {
GAP prod LINE crates/api/src/feed_checks.rs:215  Some(value) => (
GAP prod LINE crates/api/src/feed_checks.rs:216  PathBuf::from(value),
GAP prod LINE crates/api/src/feed_checks.rs:217  format!("the environment variable {env}"),
GAP prod LINE crates/api/src/feed_checks.rs:218  ),
GAP prod LINE crates/api/src/feed_checks.rs:300  Err(reason) => json!(reason.name()),
GAP prod LINE crates/api/src/feed_checks.rs:660  return Err(Refusal::Asked(
GAP prod LINE crates/api/src/feed_checks.rs:661  "a source row is asked by year, version and line".to_owned(),
GAP prod LINE crates/api/src/feed_checks.rs:662  ));
GAP prod LINE crates/api/src/feed_checks.rs:684  Refusal::Absent(format!(
GAP prod LINE crates/api/src/feed_checks.rs:685  "{} version {} holds no row of `{symbol}` in {month}",
GAP prod LINE crates/api/src/feed_checks.rs:686  held.year, held.version
GAP prod LINE crates/api/src/feed_checks.rs:687  ))
GAP prod LINE crates/api/src/feed_checks.rs:688  })?;
GAP prod LINE crates/api/src/feed_checks.rs:755  return Err(Refusal::Asked(
GAP prod LINE crates/api/src/feed_checks.rs:756  "a source row is asked by year, version and line".to_owned(),
GAP prod LINE crates/api/src/feed_checks.rs:757  ));
GAP prod LINE crates/api/src/feed_checks.rs:791  return Err(Refusal::Asked(
GAP prod LINE crates/api/src/feed_checks.rs:792  "a source row is asked by year, version and line alone".to_owned(),
GAP prod LINE crates/api/src/feed_checks.rs:793  ));
GAP prod LINE crates/api/src/feed_checks.rs:854  json!({ "record": "owed", "note": "no NSE holiday circular is recorded; days outside the measured window are Unmeasured" })
GAP prod LINE crates/api/src/feed_checks.rs:885  Err(why) => *field(&mut out, "refused") = json!(why.to_string()),
GAP prod LINE crates/api/src/feed_checks.rs:917  "suspected": halt.evidence.is_suspected(),
GAP prod LINE crates/api/src/feed_checks.rs:918  "evidence": halt.evidence.text(),
GAP prod LINE crates/api/src/feed_checks.rs:974  0
GAP prod LINE crates/api/src/feed_checks.rs:994  return Err(Refusal::Absent(why.to_owned()));
GAP prod LINE crates/api/src/feed_checks.rs:998  return Err(Refusal::Asked(format!(
GAP prod LINE crates/api/src/feed_checks.rs:999  "{year} is not a year this calendar spells"
GAP prod LINE crates/api/src/feed_checks.rs:1000  )));
GAP prod LINE crates/api/src/feed_checks.rs:1051  Refusal::Refused(_) => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod BRANCH crates/api/src/feed_checks.rs:256:8 True  if symbol.is_empty()
GAP prod BRANCH crates/api/src/feed_checks.rs:257:12 True  || symbol.len() > 64
GAP prod BRANCH crates/api/src/feed_checks.rs:260:22 True  .any(|b| b.is_ascii_control() || b == b',' || b == b'/')
GAP prod BRANCH crates/api/src/feed_checks.rs:460:32 True  if symbol.is_some() || named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:460:56 True  if symbol.is_some() || named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:494:12 True  if symbol.is_some() || named(query, "limit") || named(query, "offset") {
GAP prod BRANCH crates/api/src/feed_checks.rs:494:57 True  if symbol.is_some() || named(query, "limit") || named(query, "offset") {
GAP prod BRANCH crates/api/src/feed_checks.rs:659:8 True  if named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:659:32 True  if named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:739:12 False  && month.year() != year
GAP prod BRANCH crates/api/src/feed_checks.rs:754:12 True  if named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:754:36 True  if named(query, "line") || named(query, "version") {
GAP prod BRANCH crates/api/src/feed_checks.rs:790:12 True  if named(query, "limit") || named(query, "offset") {
GAP prod BRANCH crates/api/src/feed_checks.rs:790:37 True  if named(query, "limit") || named(query, "offset") {
GAP prod BRANCH crates/api/src/feed_checks.rs:973:24 True  let total = if first == 0 {
GAP prod BRANCH crates/api/src/feed_checks.rs:993:12 True  if let Some(why) = counted.get("refused").and_then(Value::as_str) {
GAP prod BRANCH crates/api/src/feed_checks.rs:997:9 False  let (Ok(start), Ok(end)) = (Day::new(year, 1, 1), Day::new(year, 12, 31)) else {
GAP prod LINE crates/api/src/feed_checks_files.rs:72  other => other,
GAP prod LINE crates/api/src/feed_checks_files.rs:202  Refusal::Refused(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:203  "`{}` has no modification time: {why}",
GAP prod LINE crates/api/src/feed_checks_files.rs:204  dir.display()
GAP prod LINE crates/api/src/feed_checks_files.rs:205  ))
GAP prod LINE crates/api/src/feed_checks_files.rs:206  })?),
GAP prod LINE crates/api/src/feed_checks_files.rs:214  Err(why) => {
GAP prod LINE crates/api/src/feed_checks_files.rs:215  return Err(Refusal::Refused(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:216  "`{}` cannot be read: {why}",
GAP prod LINE crates/api/src/feed_checks_files.rs:217  dir.display()
GAP prod LINE crates/api/src/feed_checks_files.rs:218  )));
GAP prod LINE crates/api/src/feed_checks_files.rs:238  continue;
GAP prod LINE crates/api/src/feed_checks_files.rs:306  return None;
GAP prod LINE crates/api/src/feed_checks_files.rs:520  return Err("an empty `symbol`".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:539  return Err("more than 64 distinct flag names".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:549  (z, g) => {
GAP prod LINE crates/api/src/feed_checks_files.rs:550  return Err(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:551  "`median_z` {z:?} and `median_g` {g:?}: a factor is two paisa prices of at least 1"
GAP prod LINE crates/api/src/feed_checks_files.rs:552  ));
GAP prod LINE crates/api/src/feed_checks_files.rs:621  return Err(
GAP prod LINE crates/api/src/feed_checks_files.rs:622  "its column line is not this build's `cli::feed_reconcile::COLUMNS`".to_owned(),
GAP prod LINE crates/api/src/feed_checks_files.rs:623  );
GAP prod LINE crates/api/src/feed_checks_files.rs:636  return Err("it has no column line".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:648  return Err(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:649  "line {}: a row after the comment block",
GAP prod LINE crates/api/src/feed_checks_files.rs:650  number + 1
GAP prod LINE crates/api/src/feed_checks_files.rs:651  ));
GAP prod LINE crates/api/src/feed_checks_files.rs:655  return Err(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:656  "line {}: {} cells, the column line names {width}",
GAP prod LINE crates/api/src/feed_checks_files.rs:657  number + 1,
GAP prod LINE crates/api/src/feed_checks_files.rs:658  cells.len()
GAP prod LINE crates/api/src/feed_checks_files.rs:659  ));
GAP prod LINE crates/api/src/feed_checks_files.rs:692  return Err("it has no `# summary:` line, the line the writer prints last".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:836  return Err(Refusal::Refused(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:837  "`{}` changed between two reads of it: a version file is never rewritten, so this \
GAP prod LINE crates/api/src/feed_checks_files.rs:838  one is refused",
GAP prod LINE crates/api/src/feed_checks_files.rs:839  file.display()
GAP prod LINE crates/api/src/feed_checks_files.rs:840  )));
GAP prod LINE crates/api/src/feed_checks_files.rs:1089  return Err(wrong());
GAP prod LINE crates/api/src/feed_checks_files.rs:1098  return Err(wrong());
GAP prod LINE crates/api/src/feed_checks_files.rs:1101  return Err(wrong());
GAP prod LINE crates/api/src/feed_checks_files.rs:1128  return Err("an empty `symbol`".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:1200  return Err(format!("month {month}: a share's slot {slot} is not held"));
GAP prod LINE crates/api/src/feed_checks_files.rs:1233  return Err(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:1234  "month {}: a share's slot {slot} is not held",
GAP prod LINE crates/api/src/feed_checks_files.rs:1235  held.month
GAP prod LINE crates/api/src/feed_checks_files.rs:1236  ));
GAP prod LINE crates/api/src/feed_checks_files.rs:1291  return Err("it is empty".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:1294  return Err("its header is not this build's `cli::tick_second_check::HEADER`".to_owned());
GAP prod LINE crates/api/src/feed_checks_files.rs:1313  return Err(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:1314  "line {}: {} cells, the header names {width}",
GAP prod LINE crates/api/src/feed_checks_files.rs:1315  number + 1,
GAP prod LINE crates/api/src/feed_checks_files.rs:1316  cells.len()
GAP prod LINE crates/api/src/feed_checks_files.rs:1317  ));
GAP prod LINE crates/api/src/feed_checks_files.rs:1385  Refusal::Absent(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:1386  "`{}` holds no tick-to-second index of {year}",
GAP prod LINE crates/api/src/feed_checks_files.rs:1387  dir.display()
GAP prod LINE crates/api/src/feed_checks_files.rs:1388  ))
GAP prod LINE crates/api/src/feed_checks_files.rs:1389  })?;
GAP prod LINE crates/api/src/feed_checks_files.rs:1429  Refusal::Refused(format!("`{}` line {}: {why}", file.display(), place.line))
GAP prod LINE crates/api/src/feed_checks_files.rs:1430  })?;
GAP prod BRANCH crates/api/src/feed_checks_files.rs:119:12 False  if let Some(count) = counts.get_mut(at) {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:213:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
GAP prod BRANCH crates/api/src/feed_checks_files.rs:237:17 False  let Ok(name) = entry.file_name().into_string() else {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:302:8 True  if !year.bytes().all(|b| b.is_ascii_digit())
GAP prod BRANCH crates/api/src/feed_checks_files.rs:303:12 True  || digits.is_empty()
GAP prod BRANCH crates/api/src/feed_checks_files.rs:304:12 True  || !digits.bytes().all(|b| b.is_ascii_digit())
GAP prod BRANCH crates/api/src/feed_checks_files.rs:519:8 True  if symbol.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:538:12 True  if bit >= 64 {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:548:31 False  (Some(z), Some(g)) if z >= 1 && g >= 1 => Some((z, g)),
GAP prod BRANCH crates/api/src/feed_checks_files.rs:548:41 False  (Some(z), Some(g)) if z >= 1 && g >= 1 => Some((z, g)),
GAP prod BRANCH crates/api/src/feed_checks_files.rs:620:12 True  if line != cli::feed_reconcile::COLUMNS {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:635:8 True  if parsed.columns.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:647:12 True  if parsed.summary.is_some() || line.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:647:40 True  if parsed.summary.is_some() || line.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:654:12 True  if cells.len() != width {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:685:16 False  if let Some(stock) = parsed.stocks.last_mut() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:691:8 True  if parsed.summary.is_none() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:835:8 True  if brutex_core::blake3::hash(&bytes) != digest {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1088:8 True  if text.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1088:28 True  if text.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1088:59 True  if text.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1097:9 False  let (Some(y), Some(m), Some(d)) = (number(0..4), number(5..7), number(8..10)) else {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1100:8 True  if y != year {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1127:8 True  if symbol.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1198:9 False  let (Some(stock), Some(members)) = (held.stocks.get_mut(slot), held.members.get_mut(slot))
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1232:17 False  let (Some(stock), Some(rows)) = (held.stocks.get(slot), members.get_mut(slot)) else {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1290:9 False  let Some((_, (_, header))) = lines.next() else {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1293:8 True  if header != cli::tick_second_check::HEADER {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1312:12 True  if cells.len() != width {
GAP test LINE crates/api/src/feed_checks_tests.rs:198  other => panic!("expected a 400 refusal, got {other:?}"),
GAP test LINE crates/api/src/feed_checks_tests.rs:205  other => panic!("expected a 404, got {other:?}"),
GAP test LINE crates/api/src/feed_checks_tests.rs:212  other => panic!("expected a 500 refusal, got {other:?}"),
GAP test LINE crates/api/src/feed_checks_tests.rs:900  Err(why) => assert_eq!(year["refused"], why.to_string()),
GAP prod LINE crates/api/src/feeds_route.rs:403  "none"
GAP prod LINE crates/api/src/feeds_route.rs:513  (
GAP prod LINE crates/api/src/feeds_route.rs:514  false,
GAP prod LINE crates/api/src/feeds_route.rs:515  format!(
GAP prod LINE crates/api/src/feeds_route.rs:516  "{} has no store prefix, so nothing it pulled could be filed",
GAP prod LINE crates/api/src/feeds_route.rs:517  feed.display()
GAP prod LINE crates/api/src/feeds_route.rs:518  ),
GAP prod LINE crates/api/src/feeds_route.rs:519  )
GAP prod LINE crates/api/src/feeds_route.rs:616  Some(why) => render::json_string(&why),
GAP prod LINE crates/api/src/feeds_route.rs:729  return;
GAP prod LINE crates/api/src/feeds_route.rs:734  }
GAP prod LINE crates/api/src/feeds_route.rs:793  return PartSpan::walk(part);
GAP prod LINE crates/api/src/feeds_route.rs:808  (PartSpan::walk(part), walks.saturating_add(1))
GAP prod LINE crates/api/src/feeds_route.rs:811  Some((_, walks)) => (PartSpan::walk(part), walks.saturating_add(1)),
GAP prod LINE crates/api/src/feeds_route.rs:833  (Some(a), Some(b)) => Some(a.widen(b)),
GAP prod LINE crates/api/src/feeds_route.rs:875  matches!(
GAP prod LINE crates/api/src/feeds_route.rs:932  break;
GAP prod LINE crates/api/src/feeds_route.rs:1052  return none(format!("{segment:?} is not a segment"));
GAP prod LINE crates/api/src/feeds_route.rs:1194  pull::vendor::SourceKind::Rest => NOT_PULLED,
GAP prod LINE crates/api/src/feeds_route.rs:1236  span: Err(format!(
GAP prod LINE crates/api/src/feeds_route.rs:1237  "names {} as its last day, after today {} (IST): a day that has not happened \
GAP prod LINE crates/api/src/feeds_route.rs:1238  shows no delivery, so this evidence sets neither end of the archive",
GAP prod LINE crates/api/src/feeds_route.rs:1239  day_text(span.last),
GAP prod LINE crates/api/src/feeds_route.rs:1240  day_text(today)
GAP prod LINE crates/api/src/feeds_route.rs:1241  )),
GAP prod LINE crates/api/src/feeds_route.rs:1242  ..evidence
GAP prod LINE crates/api/src/feeds_route.rs:1243  },
GAP prod LINE crates/api/src/feeds_route.rs:1285  "the journal names no day an import wrote bars on".to_owned()
GAP prod LINE crates/api/src/feeds_route.rs:1293  Err(why) => Evidence {
GAP prod LINE crates/api/src/feeds_route.rs:1294  by: "journal",
GAP prod LINE crates/api/src/feeds_route.rs:1295  span: Err(format!("the GDFL import journal cannot be read: {why}")),
GAP prod LINE crates/api/src/feeds_route.rs:1296  source: journal_source.to_owned(),
GAP prod LINE crates/api/src/feeds_route.rs:1297  },
GAP prod LINE crates/api/src/feeds_route.rs:1506  return refuse(format!(
GAP prod LINE crates/api/src/feeds_route.rs:1507  "the operator's window has no first or last day in force (from: {}; to: {})",
GAP prod LINE crates/api/src/feeds_route.rs:1508  from.source, to.source
GAP prod LINE crates/api/src/feeds_route.rs:1509  ));
GAP prod LINE crates/api/src/feeds_route.rs:1666  "true"
GAP ignored LINE crates/api/src/feeds_route.rs:2971  fn held_span_growth_bench() {
GAP ignored LINE crates/api/src/feeds_route.rs:2972  held_span_growth([1_000, 10_000, 100_000], 200, 4);
GAP ignored LINE crates/api/src/feeds_route.rs:2973  }
GAP prod BRANCH crates/api/src/feeds_route.rs:339:27 False  if !served && row.is_none() {
GAP prod BRANCH crates/api/src/feeds_route.rs:397:27 False  } else if spec.fno.by_token().is_some() {
GAP prod BRANCH crates/api/src/feeds_route.rs:510:15 True  } else if feed.store_vendor().is_none() {
GAP prod BRANCH crates/api/src/feeds_route.rs:728:13 False  let Some(held) = part.held(key) else {
GAP prod BRANCH crates/api/src/feeds_route.rs:732:16 False  if let Some(slot) = self.segments.get_mut(segment_slot(key.segment)) {
GAP prod BRANCH crates/api/src/feeds_route.rs:792:8 True  if part.path().as_os_str().is_empty() {
GAP prod BRANCH crates/api/src/feeds_route.rs:799:32 False  Some((span, walks)) if span.generation < generation => {
GAP prod BRANCH crates/api/src/feeds_route.rs:800:20 False  if let Some(since) = part.changed_since(span.generation) {
GAP prod BRANCH crates/api/src/feeds_route.rs:923:42 False  if last_written.is_none() && wrote(row.outcome) {
GAP prod BRANCH crates/api/src/feeds_route.rs:926:20 False  if let Some(slot) = reach.closed.get_mut(kind_slot(row.kind))
GAP prod BRANCH crates/api/src/feeds_route.rs:931:16 False  if last_written.is_some() && reach.closed.iter().all(Option::is_some) {
GAP prod BRANCH crates/api/src/feeds_route.rs:931:42 True  if last_written.is_some() && reach.closed.iter().all(Option::is_some) {
GAP prod BRANCH crates/api/src/feeds_route.rs:1051:9 False  let Ok(segment) = brutex_core::instrument::Segment::parse(segment) else {
GAP prod BRANCH crates/api/src/feeds_route.rs:1094:25 False  row.kind == *kind
GAP prod BRANCH crates/api/src/feeds_route.rs:1125:16 False  && let (Some(first), Some(last)) = (civil(days.0), civil(last))
GAP prod BRANCH crates/api/src/feeds_route.rs:1128:16 False  if row.kind == *kind
GAP prod BRANCH crates/api/src/feeds_route.rs:1129:20 False  && row.filter == "*"
GAP prod BRANCH crates/api/src/feeds_route.rs:1131:20 False  && !row.previous.is_some_and(wrote)
GAP prod BRANCH crates/api/src/feeds_route.rs:1235:36 True  (Ok(span), Some(today)) if span.last > today => Evidence {
GAP prod BRANCH crates/api/src/feeds_route.rs:1284:34 True  span: Err(if snapshot.present {
GAP prod BRANCH crates/api/src/feeds_route.rs:1348:21 False  .find(|row| wrote(row.outcome) && epoch(row.day) <= now)
GAP prod BRANCH crates/api/src/feeds_route.rs:1498:13 False  && (from.origin != "generation" || to.origin != "generation")
GAP prod BRANCH crates/api/src/feeds_route.rs:1498:44 True  && (from.origin != "generation" || to.origin != "generation")
GAP prod BRANCH crates/api/src/feeds_route.rs:1498:44 False  && (from.origin != "generation" || to.origin != "generation")
GAP prod BRANCH crates/api/src/feeds_route.rs:1505:9 False  let (Some(first), Some(last)) = (from.value, to.value) else {
GAP prod BRANCH crates/api/src/feeds_route.rs:1573:24 False  && target.secs().is_multiple_of(source)
GAP prod BRANCH crates/api/src/feeds_route.rs:1665:20 True  if day.is_fold {
GAP test BRANCH crates/api/src/feeds_route.rs:1790:13 False  [an operand followed by ||: neither side fails the check by itself]  !body.contains(r#""oldest":"2021-08-11""#) || oldest != "2021-08-11",
GAP test BRANCH crates/api/src/feeds_route.rs:1790:59 True  !body.contains(r#""oldest":"2021-08-11""#) || oldest != "2021-08-11",
GAP test BRANCH crates/api/src/feeds_route.rs:1790:59 False  !body.contains(r#""oldest":"2021-08-11""#) || oldest != "2021-08-11",
GAP test LINE crates/api/src/fno_boundary_tests.rs:42  pull::vendor::Transport::LocalArchive(_) => None,
GAP test LINE crates/api/src/fno_boundary_tests.rs:178  });
GAP test LINE crates/api/src/fno_boundary_tests.rs:220  pull::vendor::Transport::LocalArchive(_) => None,
GAP prod LINE crates/api/src/folder.rs:125  Err(why) => return (axum::http::StatusCode::BAD_REQUEST, head, refused(&why)),
GAP prod LINE crates/api/src/folder.rs:138  Err(why) => {
GAP prod LINE crates/api/src/folder.rs:139  return (
GAP prod LINE crates/api/src/folder.rs:140  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/folder.rs:141  head,
GAP prod LINE crates/api/src/folder.rs:142  refused(&why.to_string()),
GAP prod LINE crates/api/src/folder.rs:143  );
GAP prod LINE crates/api/src/folder.rs:151  Ok(answered) => answered,
GAP prod LINE crates/api/src/folder.rs:335  if at > 0 {
GAP prod LINE crates/api/src/folder.rs:336  strays.push(',');
GAP prod LINE crates/api/src/folder.rs:337  }
GAP prod LINE crates/api/src/folder.rs:338  let _ = write!(
GAP prod LINE crates/api/src/folder.rs:339  strays,
GAP prod LINE crates/api/src/folder.rs:341  render::json_string(&bad.path.display().to_string()),
GAP prod LINE crates/api/src/folder.rs:342  render::json_string(&bad.why),
GAP prod BRANCH crates/api/src/folder.rs:335:12 True  if at > 0 {
GAP prod BRANCH crates/api/src/folder.rs:335:12 False  if at > 0 {
GAP prod LINE crates/api/src/frontierjson.rs:125  return unavailable(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why);
GAP prod LINE crates/api/src/frontierjson.rs:130  Err(why) => return refuse(json, &why),
GAP prod LINE crates/api/src/frontierjson.rs:194  return refuse(
GAP prod LINE crates/api/src/frontierjson.rs:195  json,
GAP prod LINE crates/api/src/frontierjson.rs:196  "the validated frontier page window did not fit its verified row vector; no fallback page is exposed",
GAP prod LINE crates/api/src/frontierjson.rs:266  return too_large(
GAP prod LINE crates/api/src/frontierjson.rs:267  json,
GAP prod LINE crates/api/src/frontierjson.rs:268  &identity,
GAP prod LINE crates/api/src/frontierjson.rs:269  u64::try_from(rows.len()).unwrap_or(u64::MAX),
GAP prod LINE crates/api/src/frontierjson.rs:270  &format!(
GAP prod LINE crates/api/src/frontierjson.rs:271  "the bounded frontier response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
GAP prod LINE crates/api/src/frontierjson.rs:272  out.len(),
GAP prod LINE crates/api/src/frontierjson.rs:273  crate::detail::MAX_RESPONSE_BYTES
GAP prod LINE crates/api/src/frontierjson.rs:274  ),
GAP prod LINE crates/api/src/frontierjson.rs:310  Err(unreadable) => {
GAP prod LINE crates/api/src/frontierjson.rs:311  return refuse(
GAP prod LINE crates/api/src/frontierjson.rs:312  json,
GAP prod LINE crates/api/src/frontierjson.rs:313  &format!(
GAP prod LINE crates/api/src/frontierjson.rs:314  "{why}. {} could not be checked for absence: {unreadable}",
GAP prod LINE crates/api/src/frontierjson.rs:315  child.display()
GAP prod LINE crates/api/src/frontierjson.rs:316  ),
GAP prod LINE crates/api/src/frontierjson.rs:355  return range_refusal(json, identity, 0, &range);
GAP prod LINE crates/api/src/frontierjson.rs:388  admitted = admitted.saturating_add(1);
GAP prod LINE crates/api/src/frontierjson.rs:484  let _ = std::fmt::Write::write_fmt(
GAP prod LINE crates/api/src/frontierjson.rs:485  &mut *out,
GAP prod LINE crates/api/src/frontierjson.rs:486  format_args!(
GAP prod LINE crates/api/src/frontierjson.rs:487  r#"],"count":{},"total_count":{},"admitted":{},"total_admitted":{},"rules":null,"page":{},"limit":{},"page_complete":true,"complete":{},"nex
GAP prod LINE crates/api/src/frontierjson.rs:488  count,
GAP prod LINE crates/api/src/frontierjson.rs:489  total_count,
GAP prod LINE crates/api/src/frontierjson.rs:490  admitted,
GAP prod LINE crates/api/src/frontierjson.rs:491  total_admitted,
GAP prod LINE crates/api/src/frontierjson.rs:492  page.number,
GAP prod LINE crates/api/src/frontierjson.rs:493  page.limit,
GAP prod LINE crates/api/src/frontierjson.rs:494  window.complete,
GAP prod LINE crates/api/src/frontierjson.rs:495  next,
GAP prod LINE crates/api/src/frontierjson.rs:496  refusal
GAP prod LINE crates/api/src/frontierjson.rs:497  ),
GAP prod LINE crates/api/src/frontierjson.rs:498  );
GAP prod LINE crates/api/src/frontierjson.rs:499  return;
GAP prod BRANCH crates/api/src/frontierjson.rs:120:12 True  if let Err(why) = crate::detail::preflight([
GAP prod BRANCH crates/api/src/frontierjson.rs:193:9 False  let Some(page_rows) = rows.get(window.start..window.end) else {
GAP prod BRANCH crates/api/src/frontierjson.rs:265:8 True  if out.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/frontierjson.rs:300:25 False  Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {}
GAP prod BRANCH crates/api/src/frontierjson.rs:354:12 True  if let Err(range) = crate::detail::window(0, page) {
GAP prod BRANCH crates/api/src/frontierjson.rs:387:12 True  if v.admitted {
GAP prod BRANCH crates/api/src/frontierjson.rs:483:9 False  let Some(rules) = rules else {
GAP test LINE crates/api/src/gap_peer_route_tests.rs:43  matches!(
GAP test LINE crates/api/src/gap_peer_route_tests.rs:393  }
GAP test LINE crates/api/src/gap_peer_route_tests.rs:394  assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{answer}");
GAP test LINE crates/api/src/gap_peer_route_tests.rs:395  assert_eq!(answer["status"], "building", "{answer}");
GAP test LINE crates/api/src/gap_peer_route_tests.rs:396  tokio::time::sleep(std::time::Duration::from_millis(10)).await;
GAP test BRANCH crates/api/src/gap_peer_route_tests.rs:391:16 False  if status == axum::http::StatusCode::OK {
GAP prod LINE crates/api/src/gaps_route.rs:506  return (span, false);
GAP prod LINE crates/api/src/gaps_route.rs:1655  return empty(Some(format!(
GAP prod LINE crates/api/src/gaps_route.rs:1656  "{month} is a month this build's calendar cannot name a first day \
GAP prod LINE crates/api/src/gaps_route.rs:1657  for, so there is no range to audit against"
GAP prod LINE crates/api/src/gaps_route.rs:1658  )));
GAP prod BRANCH crates/api/src/gaps_route.rs:505:13 False  let Some(next) = next_month(cursor) else {
GAP prod BRANCH crates/api/src/gaps_route.rs:1654:9 False  let Ok(first_day) = pull::session::Day::new(month.year(), month.month(), 1) else {
GAP test BRANCH crates/api/src/gaps_route.rs:2795:13 False  [an operand followed by ||: neither side fails the check by itself]  held.contains("does not exist") || held.contains("absent"),
GAP test BRANCH crates/api/src/gaps_route.rs:2795:48 True  held.contains("does not exist") || held.contains("absent"),
GAP test BRANCH crates/api/src/gaps_route.rs:2795:48 False  held.contains("does not exist") || held.contains("absent"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:138  Ok(())
GAP test LINE crates/api/src/head_deadline_body_tests.rs:139  }),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:199  Err(elapsed) => panic!("{elapsed}: no byte for 10 s after {} of {want}", have.len()),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:218  Err(_elapsed) => return (String::from_utf8_lossy(&got).into_owned(), false),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:857  Ok(0) | Err(_) => break,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:890  Poll::Ready(Err(why)) => panic!("{why}"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:894  assert_eq!(wrapped.alarm.deadline(), timer, "a stall reset twice");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:904  assert_eq!(wrapped.alarm.deadline(), armed, "no wait, no reset");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:980  Poll::Pending => panic!("the stall deadline passed and the write still waited"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:243:12 True  if let Some(at) = body.iter().enumerate().position(|(i, b)| *b != pattern(i)) {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:893:20 True  if stall.is_some() {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:895:27 False  } else if wrapped.alarm.deadline() != timer {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:903:8 True  if waited == 0 {
GAP test LINE crates/api/src/http_admission_tests.rs:53  && !head_of(&answer).contains("x-brutex-request-audit");
GAP test LINE crates/api/src/http_admission_tests.rs:56  }
GAP test LINE crates/api/src/http_admission_tests.rs:57  tokio::time::sleep(std::time::Duration::from_millis(10)).await;
GAP test BRANCH crates/api/src/http_admission_tests.rs:52:20 True  let busy = status_line(&answer).contains("429")
GAP test BRANCH crates/api/src/http_admission_tests.rs:54:12 False  if !busy {
GAP prod LINE crates/api/src/import_journal.rs:429  Err(_) => self.runs_unindexed = self.runs_unindexed.saturating_add(1),
GAP ignored LINE crates/api/src/import_journal.rs:848  fn end_line_fold_timing() {
GAP ignored LINE crates/api/src/import_journal.rs:849  end_lines_folded_beside(100_000);
GAP ignored LINE crates/api/src/import_journal.rs:850  }
GAP ignored LINE crates/api/src/import_journal.rs:1091  fn end_line_fold_cost_by_field_count() {
GAP ignored LINE crates/api/src/import_journal.rs:1092  end_line_fold_at_widths(2_000);
GAP ignored LINE crates/api/src/import_journal.rs:1093  }
GAP prod BRANCH crates/api/src/import_journal.rs:417:28 False  && let Some(run) = self.runs.get_mut(start)
GAP prod BRANCH crates/api/src/import_journal.rs:536:16 False  if let Some(last) = self.last.get_mut(slot) {
GAP test BRANCH crates/api/src/import_journal.rs:1498:25 False  .find(|row| row.kind == kind && row.filter == "*")
GAP prod LINE crates/api/src/imports.rs:331  Ok(Err(why)) => (
GAP prod LINE crates/api/src/imports.rs:332  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/imports.rs:333  json_header(),
GAP prod LINE crates/api/src/imports.rs:334  json!({ "error": why }).to_string(),
GAP prod LINE crates/api/src/imports.rs:335  ),
GAP prod LINE crates/api/src/imports.rs:404  Err(why) => return json!({ "error": why, "route": "/imports.json?feed=gdfl" }),
GAP prod LINE crates/api/src/imports.rs:925  continue;
GAP prod LINE crates/api/src/imports.rs:1283  parts.push(format!(
GAP prod LINE crates/api/src/imports.rs:1286  if unlisted.unnamed == 1 {
GAP prod LINE crates/api/src/imports.rs:1287  "entry"
GAP prod LINE crates/api/src/imports.rs:1289  "entries"
GAP prod LINE crates/api/src/imports.rs:1336  format!("logs/{here} ({error})")
GAP prod LINE crates/api/src/imports.rs:1343  odd.unnamed = odd.unnamed.saturating_add(1);
GAP prod LINE crates/api/src/imports.rs:1344  continue;
GAP prod LINE crates/api/src/imports.rs:1355  odd.unnamed = odd.unnamed.saturating_add(1);
GAP prod LINE crates/api/src/imports.rs:1356  continue;
GAP prod LINE crates/api/src/imports.rs:1394  Ok(super::dayflags::answer_at(root, &asked, band).to_string())
GAP prod LINE crates/api/src/imports.rs:1432  let _absent = object.insert("end_line".to_owned(), Value::from(end));
GAP prod LINE crates/api/src/imports.rs:1473  format!("{id}.{nth}")
GAP prod LINE crates/api/src/imports.rs:1544  notes.push(format!(
GAP prod LINE crates/api/src/imports.rs:1545  "{} line(s) of the journal are not lines this reader names; they are listed under \
GAP prod LINE crates/api/src/imports.rs:1546  journal.foreign and left out of every row. The importer refuses such a journal \
GAP prod LINE crates/api/src/imports.rs:1547  (D-2814), so its next run will name them too.",
GAP prod LINE crates/api/src/imports.rs:1548  snapshot.parsed.foreign_count
GAP prod LINE crates/api/src/imports.rs:1549  ));
GAP prod LINE crates/api/src/imports.rs:1552  notes.push(format!(
GAP prod LINE crates/api/src/imports.rs:1553  "The journal's last {} byte(s) end without a newline: a line being written, or a \
GAP prod LINE crates/api/src/imports.rs:1554  crash's fragment the importer closes on its next load. Not read.",
GAP prod LINE crates/api/src/imports.rs:1555  snapshot.unterminated
GAP prod LINE crates/api/src/imports.rs:1556  ));
GAP prod BRANCH crates/api/src/imports.rs:416:12 False  && std::sync::Arc::ptr_eq(refusals, &evidence)
GAP prod BRANCH crates/api/src/imports.rs:417:12 False  && snapshot.present
GAP prod BRANCH crates/api/src/imports.rs:440:12 False  if let Some(journal) = value.get_mut("journal").and_then(Value::as_object_mut) {
GAP prod BRANCH crates/api/src/imports.rs:540:9 True  field(record, "stage").as_deref() == Some(census)
GAP prod BRANCH crates/api/src/imports.rs:654:21 False  why.starts_with("n_valid is ")
GAP prod BRANCH crates/api/src/imports.rs:752:5 False  asked.kind.is_none_or(|kind| kind == row.kind)
GAP prod BRANCH crates/api/src/imports.rs:848:16 False  if let Some(slot) = DayOutcome::ALL
GAP prod BRANCH crates/api/src/imports.rs:860:22 False  if census && !row.outcome.is_done() {
GAP prod BRANCH crates/api/src/imports.rs:862:19 True  } else if row.census_closes > 0 || (row.recounted && row.previous.is_some()) {
GAP prod BRANCH crates/api/src/imports.rs:862:62 False  } else if row.census_closes > 0 || (row.recounted && row.previous.is_some()) {
GAP prod BRANCH crates/api/src/imports.rs:921:13 False  let (Ok(kind), Some(day)) = (
GAP prod BRANCH crates/api/src/imports.rs:1033:8 True  if about.is_empty()
GAP prod BRANCH crates/api/src/imports.rs:1129:48 True  "days_per_hour_at_p50": if pace.p50 > 0 { 3_600_000 / pace.p50 } else { 0 },
GAP prod BRANCH crates/api/src/imports.rs:1282:8 True  if unlisted.unnamed > 0 {
GAP prod BRANCH crates/api/src/imports.rs:1286:16 True  if unlisted.unnamed == 1 {
GAP prod BRANCH crates/api/src/imports.rs:1286:16 False  if unlisted.unnamed == 1 {
GAP prod BRANCH crates/api/src/imports.rs:1333:30 False  odd.dirs.push(if here.is_empty() {
GAP prod BRANCH crates/api/src/imports.rs:1342:13 False  let Ok(entry) = entry else {
GAP prod BRANCH crates/api/src/imports.rs:1348:21 False  || (kind.is_symlink()
GAP prod BRANCH crates/api/src/imports.rs:1354:13 False  let Ok(name) = entry.file_name().into_string() else {
GAP prod BRANCH crates/api/src/imports.rs:1431:12 True  if let (Some(end), Some(object)) = (run.end_line, json.as_object_mut()) {
GAP prod BRANCH crates/api/src/imports.rs:1470:24 False  if nth == 0 {
GAP prod BRANCH crates/api/src/imports.rs:1543:8 True  if snapshot.parsed.foreign_count > 0 {
GAP prod BRANCH crates/api/src/imports.rs:1551:8 True  if snapshot.unterminated > 0 {
GAP prod LINE crates/api/src/index_consistency_projection.rs:21  pub(super) fn setting(
GAP prod LINE crates/api/src/index_consistency_projection.rs:22  reader: &Qualification,
GAP prod LINE crates/api/src/index_consistency_projection.rs:23  index: usize,
GAP prod LINE crates/api/src/index_consistency_projection.rs:24  institutional: cli::boolean_evidence::AdmissionStatusV1,
GAP prod LINE crates/api/src/index_consistency_projection.rs:25  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/index_consistency_projection.rs:26  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/index_consistency_projection.rs:27  let qualification = json!({"identity":hex(reader.identity()),"completion":hex(pin)});
GAP prod LINE crates/api/src/index_consistency_projection.rs:28  let Some(record) = reader.index_consistency(pin, index)? else {
GAP prod LINE crates/api/src/index_consistency_projection.rs:29  return Ok(
GAP prod LINE crates/api/src/index_consistency_projection.rs:30  json!({"schema_version":1,"state":"not_assessed","policy_digest":null,"policy":null,"receipt":null,
GAP prod LINE crates/api/src/index_consistency_projection.rs:31  "qualification":qualification,"setting_index":index.to_string(),"combined_qualifies":null,
GAP prod LINE crates/api/src/index_consistency_projection.rs:32  "evaluation":null,"days_count":"0","weeks_count":"0","evaluated_scope":null,"periods":null}),
GAP prod LINE crates/api/src/index_consistency_projection.rs:33  );
GAP prod LINE crates/api/src/index_consistency_projection.rs:35  let receipt = reader
GAP prod LINE crates/api/src/index_consistency_projection.rs:36  .index_consistency_receipt()
GAP prod LINE crates/api/src/index_consistency_projection.rs:37  .ok_or("consistency assessment has no receipt")?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:38  saved_record(
GAP prod LINE crates/api/src/index_consistency_projection.rs:39  reader.identity(),
GAP prod LINE crates/api/src/index_consistency_projection.rs:40  pin,
GAP prod LINE crates/api/src/index_consistency_projection.rs:41  index,
GAP prod LINE crates/api/src/index_consistency_projection.rs:42  record,
GAP prod LINE crates/api/src/index_consistency_projection.rs:43  receipt,
GAP prod LINE crates/api/src/index_consistency_projection.rs:44  institutional,
GAP prod LINE crates/api/src/index_consistency_projection.rs:46  }
GAP prod LINE crates/api/src/index_consistency_projection.rs:114  pub(super) fn page(reader: &Qualification, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/index_consistency_projection.rs:115  let setting = asked
GAP prod LINE crates/api/src/index_consistency_projection.rs:116  .setting
GAP prod LINE crates/api/src/index_consistency_projection.rs:117  .ok_or("index consistency page requires exact setting")?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:118  let pin = asked
GAP prod LINE crates/api/src/index_consistency_projection.rs:119  .completion
GAP prod LINE crates/api/src/index_consistency_projection.rs:120  .ok_or("index consistency page requires exact qualification completion")?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:121  let record = reader
GAP prod LINE crates/api/src/index_consistency_projection.rs:122  .index_consistency(pin, setting)?
GAP prod LINE crates/api/src/index_consistency_projection.rs:123  .ok_or("historical qualification has no index consistency assessment")?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:124  let receipt = reader
GAP prod LINE crates/api/src/index_consistency_projection.rs:125  .index_consistency_receipt()
GAP prod LINE crates/api/src/index_consistency_projection.rs:126  .ok_or("index consistency receipt absent")?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:127  let (evaluation, days) = match asked.period.as_str() {
GAP prod LINE crates/api/src/index_consistency_projection.rs:128  "full" => (&record.evaluation, record.sessions.as_slice()),
GAP prod LINE crates/api/src/index_consistency_projection.rs:129  "training" => (
GAP prod LINE crates/api/src/index_consistency_projection.rs:130  &record.training,
GAP prod LINE crates/api/src/index_consistency_projection.rs:131  record
GAP prod LINE crates/api/src/index_consistency_projection.rs:132  .sessions
GAP prod LINE crates/api/src/index_consistency_projection.rs:133  .get(..record.training_session_count)
GAP prod LINE crates/api/src/index_consistency_projection.rs:134  .ok_or("index training day extent missing")?,
GAP prod LINE crates/api/src/index_consistency_projection.rs:136  "later" => (
GAP prod LINE crates/api/src/index_consistency_projection.rs:137  &record.later,
GAP prod LINE crates/api/src/index_consistency_projection.rs:138  record
GAP prod LINE crates/api/src/index_consistency_projection.rs:139  .sessions
GAP prod LINE crates/api/src/index_consistency_projection.rs:140  .get(record.training_session_count..)
GAP prod LINE crates/api/src/index_consistency_projection.rs:141  .ok_or("index later day extent missing")?,
GAP prod LINE crates/api/src/index_consistency_projection.rs:143  _ => return Err("index consistency period unknown".into()),
GAP prod LINE crates/api/src/index_consistency_projection.rs:145  let total = if asked.kind == "index-weeks" {
GAP prod LINE crates/api/src/index_consistency_projection.rs:146  evaluation.weeks.len()
GAP prod LINE crates/api/src/index_consistency_projection.rs:148  days.len()
GAP prod LINE crates/api/src/index_consistency_projection.rs:150  if asked.offset > total || !(1..=256).contains(&asked.limit) {
GAP prod LINE crates/api/src/index_consistency_projection.rs:151  return Err("index consistency page is outside saved extent".into());
GAP prod LINE crates/api/src/index_consistency_projection.rs:152  }
GAP prod LINE crates/api/src/index_consistency_projection.rs:153  let end = asked.offset.saturating_add(asked.limit).min(total);
GAP prod LINE crates/api/src/index_consistency_projection.rs:154  let rows: Vec<_> = if asked.kind == "index-weeks" {
GAP prod LINE crates/api/src/index_consistency_projection.rs:155  evaluation
GAP prod LINE crates/api/src/index_consistency_projection.rs:156  .weeks
GAP prod LINE crates/api/src/index_consistency_projection.rs:157  .get(asked.offset..end)
GAP prod LINE crates/api/src/index_consistency_projection.rs:158  .ok_or("index week page outside saved extent")?
GAP prod LINE crates/api/src/index_consistency_projection.rs:159  .iter()
GAP prod LINE crates/api/src/index_consistency_projection.rs:160  .enumerate()
GAP prod LINE crates/api/src/index_consistency_projection.rs:161  .map(|(n, row)| week(asked.offset + n, row))
GAP prod LINE crates/api/src/index_consistency_projection.rs:162  .collect()
GAP prod LINE crates/api/src/index_consistency_projection.rs:164  days.get(asked.offset..end)
GAP prod LINE crates/api/src/index_consistency_projection.rs:165  .ok_or("index day page outside saved extent")?
GAP prod LINE crates/api/src/index_consistency_projection.rs:166  .iter()
GAP prod LINE crates/api/src/index_consistency_projection.rs:167  .enumerate()
GAP prod LINE crates/api/src/index_consistency_projection.rs:168  .map(|(n, row)| day(asked.offset + n, *row, evaluation.policy))
GAP prod LINE crates/api/src/index_consistency_projection.rs:169  .collect()
GAP prod LINE crates/api/src/index_consistency_projection.rs:171  reader.require_current()?;
GAP prod LINE crates/api/src/index_consistency_projection.rs:172  Ok(
GAP prod LINE crates/api/src/index_consistency_projection.rs:173  json!({"schema_version":1,"status":"saved","model":"qualification","kind":asked.kind,"identity":hex(reader.identity()),"completion":hex(pin)
GAP prod LINE crates/api/src/index_consistency_projection.rs:174  "receipt":{"identity":hex(receipt.identity),"completion":hex(receipt.completion)},"period":asked.period,
GAP prod LINE crates/api/src/index_consistency_projection.rs:175  "setting":setting.to_string(),"total":total.to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"rows":rows,"refusal":null}),
GAP prod LINE crates/api/src/index_consistency_projection.rs:176  )
GAP prod LINE crates/api/src/index_consistency_projection.rs:177  }
GAP prod LINE crates/api/src/index_consistency_projection.rs:192  pub(crate) fn week(index: usize, value: &Week) -> Value {
GAP prod LINE crates/api/src/index_consistency_projection.rs:193  json!({"index":index.to_string(),"monday":value.monday.to_string(),"kind":value.kind.as_str(),"state":value.outcome.as_str(),
GAP prod LINE crates/api/src/index_consistency_projection.rs:194  "weekday_days":value.weekday_days.to_string(),"eligible_days":value.eligible_days.to_string(),"observed_days":value.observed_days.to_string(
GAP prod LINE crates/api/src/index_consistency_projection.rs:195  "winning_days":value.winning_days.to_string(),"losing_days":value.losing_days.to_string(),"zero_days":value.zero_days.to_string(),
GAP prod LINE crates/api/src/index_consistency_projection.rs:196  "no_trade_days":value.no_trade_days.to_string(),"missing_days":value.missing_days.to_string(),"unmeasured_days":value.unmeasured_days.to_str
GAP prod LINE crates/api/src/index_consistency_projection.rs:197  "excluded_days":value.excluded_days.to_string(),"closed_weekdays":value.closed_weekdays.to_string(),"weekend_sessions":value.weekend_session
GAP prod LINE crates/api/src/index_consistency_projection.rs:198  "trades":value.trades.to_string(),"pessimistic_paisa":value.pessimistic_paisa.to_string()})
GAP prod LINE crates/api/src/index_consistency_projection.rs:199  }
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:28:9 True  let Some(record) = reader.index_consistency(pin, index)? else {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:28:9 False  let Some(record) = reader.index_consistency(pin, index)? else {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:145:20 True  let total = if asked.kind == "index-weeks" {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:145:20 False  let total = if asked.kind == "index-weeks" {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:150:8 True  if asked.offset > total || !(1..=256).contains(&asked.limit) {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:150:8 False  if asked.offset > total || !(1..=256).contains(&asked.limit) {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:150:32 True  if asked.offset > total || !(1..=256).contains(&asked.limit) {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:150:32 False  if asked.offset > total || !(1..=256).contains(&asked.limit) {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:154:27 True  let rows: Vec<_> = if asked.kind == "index-weeks" {
GAP prod BRANCH crates/api/src/index_consistency_projection.rs:154:27 False  let rows: Vec<_> = if asked.kind == "index-weeks" {
GAP prod LINE crates/api/src/indexmap.rs:638  return census_refused(&format!(
GAP prod LINE crates/api/src/indexmap.rs:639  "{}: the census read returned no row for this feed",
GAP prod LINE crates/api/src/indexmap.rs:640  feed.as_str()
GAP prod LINE crates/api/src/indexmap.rs:641  ));
GAP prod LINE crates/api/src/indexmap.rs:671  return Err(format!(
GAP prod LINE crates/api/src/indexmap.rs:672  "{}: the census read returned no row for this feed",
GAP prod LINE crates/api/src/indexmap.rs:673  feed.as_str()
GAP prod LINE crates/api/src/indexmap.rs:674  ));
GAP prod BRANCH crates/api/src/indexmap.rs:119:38 False  .filter(|(collapsed, _)| !collapsed.is_empty() && seen.insert(collapsed.clone()))
GAP prod BRANCH crates/api/src/indexmap.rs:282:54 False  Ok((alone, Basis::Published)) if alone == name => {
GAP prod BRANCH crates/api/src/indexmap.rs:497:28 False  && key.contract.is_none()
GAP prod BRANCH crates/api/src/indexmap.rs:592:16 False  if let Some(held) = self
GAP prod BRANCH crates/api/src/indexmap.rs:637:9 False  let Some(census) = now.census() else {
GAP prod BRANCH crates/api/src/indexmap.rs:670:9 False  let Some(census) = now.census() else {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:84  const fn kind(&self) -> &'static str {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:85  if self.trade.is_some() {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:86  "candles"
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:88  "source"
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:90  }
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:115  Ok(body) => {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:116  let result = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:117  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:118  refused(
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:120  "original-source response exceeds byte admission; no prefix returned",
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:123  result
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:161  cached.root == root
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:162  && cached.identity == asked.identity
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:163  && cached.pin == asked.pin
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:164  && cached.bounds == bounds
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:165  }) {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:169  *held = Some(Cached {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:170  root: root.to_owned(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:171  identity: asked.identity,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:172  pin: asked.pin,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:173  bounds,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:174  reader,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:175  });
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:176  }
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:177  let reader = &mut held
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:178  .as_mut()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:179  .ok_or("original-source reader admission absent")?
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:181  if let Err(why) = reader.require_current() {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:182  *held = None;
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:183  return Err(why.to_string());
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:184  }
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:185  let body = reader
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:186  .select_setting(asked.setting)
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:187  .and_then(|()| reader.with_current(|view| project(view, asked)));
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:188  if body.is_err() && reader.require_current().is_err() {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:189  // The current request still refuses. An explicit retry may re-admit
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:190  // byte-identical restored files under the same original completion.
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:191  *held = None;
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:192  }
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:193  body.map_err(|why| why.to_string())
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:196  fn project(reader: &View<'_>, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:197  let meta = reader.metadata();
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:198  let direction = match reader.direction() {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:199  Direction::Long => "long",
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:200  Direction::Short => "short",
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:201  Direction::Undirected => return Err("saved candle source has no trading direction".into()),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:203  let window = asked
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:204  .trade
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:205  .map(|trade| {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:206  reader
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:207  .window(
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:208  trade,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:209  asked.before.ok_or("original candle context is missing")?,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:210  asked.after.ok_or("original candle context is missing")?,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:212  .map_err(|why| why.to_string())
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:213  })
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:214  .transpose()?;
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:215  let candles: Vec<Value> = window
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:216  .as_ref()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:217  .map(|window| {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:218  window
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:219  .candles
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:220  .iter()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:221  .map(|bar| {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:222  json!({"micros":bar.ts_micros.to_string(),"open_paisa":bar.open.to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:223  "high_paisa":bar.high.to_string(),"low_paisa":bar.low.to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:224  "close_paisa":bar.close.to_string(),"volume":bar.volume.to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:225  "open_interest":bar.open_interest.to_string()})
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:226  })
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:227  .collect()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:228  })
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:229  .unwrap_or_default();
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:230  let names: Vec<Value> = reader
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:231  .condition_names()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:232  .iter()
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:233  .map(|row| json!({"bit":row.bit,"name":row.name}))
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:234  .collect();
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:236  json!({"schema_version":1,"model":"index-stop-candles","provenance_status":"verified_original_source",
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:237  "kind":asked.kind(),"catalog_identity":hex(meta.catalog_identity),"catalog_completion":hex(meta.catalog_completion),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:238  "setting":meta.setting.to_string(),"source_id":hex(meta.source_id),"run_id":hex(meta.run_id),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:239  "source_context_identity":hex(meta.source_context_identity),"source_context_completion":hex(meta.source_context_completion),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:240  "feed":meta.feed,"instrument":meta.instrument,"timeframe":meta.timeframe,"expression":meta.expression,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:241  "original_build_commit":meta.original_build_commit,"vocabulary_version":meta.vocabulary_version,
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:242  "condition_names":names,"direction":direction,"source_first_day":meta.source_first_day.to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:243  "source_last_day":meta.source_last_day.to_string(),"measurement_first_day":meta.measurement_first_day.to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:244  "measurement_last_day":meta.measurement_last_day.to_string(),"trade_index":asked.trade.map(|n|n.to_string()),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:245  "before":asked.before.map(|n|n.to_string()),"after":asked.after.map(|n|n.to_string()),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:246  "first_bar":window.as_ref().map(|window|window.first_bar.to_string()),"total_bars":reader.total_bars().to_string(),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:247  "candles":candles,"trade":window.as_ref().zip(asked.trade).map(|(window,index)|crate::indexstopjson::trade(index,&window.trade)),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:248  "admitted_bytes":reader.admitted_bytes().to_string(),"observation_byte_limit":reader.observation_byte_limit().to_string()}),
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:250  }
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:251  fn hex(value: [u8; 32]) -> String {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:252  crate::server::hex32(value)
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:253  }
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:56:64 False  "source" if trade.is_none() && before.is_none() && after.is_none() => {}
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:57:45 False  "candles" if trade.is_some() && before.is_some() && after.is_some() => {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:85:12 True  if self.trade.is_some() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:85:12 False  if self.trade.is_some() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:117:16 True  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:117:16 False  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:160:8 False  if !held.as_ref().is_some_and(|cached| {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:161:9 True  cached.root == root
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:161:9 False  cached.root == root
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:162:16 True  && cached.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:162:16 False  && cached.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:163:16 True  && cached.pin == asked.pin
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:163:16 False  && cached.pin == asked.pin
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:181:12 True  if let Err(why) = reader.require_current() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:181:12 False  if let Err(why) = reader.require_current() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:188:8 True  if body.is_err() && reader.require_current().is_err() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:188:8 False  if body.is_err() && reader.require_current().is_err() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:188:25 True  if body.is_err() && reader.require_current().is_err() {
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:188:25 False  if body.is_err() && reader.require_current().is_err() {
GAP prod LINE crates/api/src/indexstopjson.rs:97  Ok(body) => {
GAP prod LINE crates/api/src/indexstopjson.rs:98  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstopjson.rs:99  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstopjson.rs:100  reply
GAP prod LINE crates/api/src/indexstopjson.rs:102  refused(
GAP prod LINE crates/api/src/indexstopjson.rs:104  "single-stop response exceeds byte limit; no prefix returned",
GAP prod LINE crates/api/src/indexstopjson.rs:217  _ => return Err("single-stop unsupported page".into()),
GAP prod BRANCH crates/api/src/indexstopjson.rs:99:16 True  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopjson.rs:99:16 False  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopjson.rs:133:12 True  || !held.as_ref().is_some_and(|value| {
GAP prod BRANCH crates/api/src/indexstopjson.rs:134:13 False  value.root == root && value.identity == asked.identity && value.budget == budget
GAP prod BRANCH crates/api/src/indexstopjson.rs:134:35 False  value.root == root && value.identity == asked.identity && value.budget == budget
GAP prod LINE crates/api/src/indexstoplaunch.rs:82  return Err("single-stop launch requires its exact native command".into());
GAP prod LINE crates/api/src/indexstoplaunch.rs:139  Observation::Search(progress) => self.observe(progress),
GAP prod LINE crates/api/src/indexstoplaunch.rs:152  return Err("single-stop refused source cannot silently restart".into());
GAP prod LINE crates/api/src/indexstoplaunch.rs:198  return Err(
GAP prod LINE crates/api/src/indexstoplaunch.rs:199  "single-stop current batch is not its next acknowledged reservation".into(),
GAP prod LINE crates/api/src/indexstoplaunch.rs:200  );
GAP prod LINE crates/api/src/indexstoplaunch.rs:207  return Err("single-stop progress omitted a selected timeframe or contains a foreign/invalid qualification".into());
GAP prod LINE crates/api/src/indexstoplaunch.rs:213  return Err("single-stop stage does not match its exact selected scope".into());
GAP prod LINE crates/api/src/indexstoplaunch.rs:218  return Err(
GAP prod LINE crates/api/src/indexstoplaunch.rs:219  "single-stop saved boundary does not match its qualification link".into(),
GAP prod LINE crates/api/src/indexstoplaunch.rs:220  );
GAP prod LINE crates/api/src/indexstoplaunch.rs:278  && (Some(retained.ordinal) != next.completed_batches.checked_sub(1)
GAP prod LINE crates/api/src/indexstoplaunch.rs:279  || retained.qualifications != next.qualifications)
GAP prod LINE crates/api/src/indexstoplaunch.rs:330  )?;
GAP prod LINE crates/api/src/indexstoplaunch.rs:336  return Err("single-stop active request changed; no further batch starts".into());
GAP prod LINE crates/api/src/indexstoplaunch.rs:345  Ok(report) => progress.report = Some(report.into()),
GAP prod LINE crates/api/src/indexstoplaunch.rs:358  pub async fn metadata(
GAP prod LINE crates/api/src/indexstoplaunch.rs:359  axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
GAP prod LINE crates/api/src/indexstoplaunch.rs:360  uri: Uri,
GAP prod LINE crates/api/src/indexstoplaunch.rs:361  ) -> Response {
GAP prod LINE crates/api/src/indexstoplaunch.rs:362  let query = uri.query().map(str::to_owned);
GAP prod LINE crates/api/src/indexstoplaunch.rs:363  match crate::detail::run(move || configuration::value(&site.store_root, query.as_deref())).await
GAP prod LINE crates/api/src/indexstoplaunch.rs:365  Ok(value) => response(StatusCode::OK, &value),
GAP prod LINE crates/api/src/indexstoplaunch.rs:366  Err(why) => response(
GAP prod LINE crates/api/src/indexstoplaunch.rs:368  &json!({"schema_version":1,"model":"index-stop-qualified-search-launch",
GAP prod LINE crates/api/src/indexstoplaunch.rs:369  "ready":false,"refusal":format!("bounded configuration read unavailable: {why:?}")}),
GAP prod LINE crates/api/src/indexstoplaunch.rs:372  }
GAP prod LINE crates/api/src/indexstoplaunch.rs:373  fn response(status: StatusCode, value: &Value) -> Response {
GAP prod LINE crates/api/src/indexstoplaunch.rs:374  (
GAP prod LINE crates/api/src/indexstoplaunch.rs:375  status,
GAP prod LINE crates/api/src/indexstoplaunch.rs:376  [(
GAP prod LINE crates/api/src/indexstoplaunch.rs:377  axum::http::header::CONTENT_TYPE,
GAP prod LINE crates/api/src/indexstoplaunch.rs:378  "application/json; charset=utf-8",
GAP prod LINE crates/api/src/indexstoplaunch.rs:379  )],
GAP prod LINE crates/api/src/indexstoplaunch.rs:380  value.to_string(),
GAP prod LINE crates/api/src/indexstoplaunch.rs:381  )
GAP prod LINE crates/api/src/indexstoplaunch.rs:382  }
GAP prod LINE crates/api/src/indexstoplaunch.rs:424  RungStage::Refused => 5,
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:81:8 True  if asked.command != COMMAND {
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:151:20 True  if *slot == Some(RungStage::Refused) && stage != RungStage::Refused {
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:151:57 True  if *slot == Some(RungStage::Refused) && stage != RungStage::Refused {
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:151:57 False  if *slot == Some(RungStage::Refused) && stage != RungStage::Refused {
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:169:24 True  || next.completed_programs < prior.completed_programs
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:170:24 True  || next.completed_work < prior.completed_work
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:171:43 False  || prior.exhausted && prior != &next
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:173:29 True  && (next.completed_programs != prior.completed_programs
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:187:17 True  && (has_links
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:189:20 True  || next.completed_programs != 0
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:190:20 True  || next.completed_work != 0)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:194:12 True  if next
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:196:34 True  .is_some_and(|batch| batch != next.completed_batches || next.exhausted)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:204:16 True  if link.is_some() != (has_links && selected)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:205:49 True  || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:205:20 True  || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:210:29 True  if !selected && stage.is_some()
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:211:64 True  || next.current_batch.is_some() && selected && stage.is_none()
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:215:48 True  if next.current_batch.is_none() && stage.is_some() != link.is_some()
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:216:38 True  || link.is_some() && stage != Some(cli::index_stop_search::RungStage::Saved)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:262:49 True  || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:262:20 True  || link.is_some_and(|(id, pin)| id == [0; 32] || pin == [0; 32])
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:275:13 True  retained.ordinal < prior.ordinal
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:276:20 False  || retained.ordinal == prior.ordinal && retained != *prior
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:277:15 True  }) || next.qualifications.iter().any(Option::is_some)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:278:17 True  && (Some(retained.ordinal) != next.completed_batches.checked_sub(1)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:278:17 False  && (Some(retained.ordinal) != next.completed_batches.checked_sub(1)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:279:20 True  || retained.qualifications != next.qualifications)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:279:20 False  || retained.qualifications != next.qualifications)
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:327:31 False  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:331:16 True  if target
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:384:8 True  if raw.is_empty()
GAP prod BRANCH crates/api/src/indexstoplaunch.rs:385:12 True  || raw.len() > 20
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:43  crate::booleanlaunch::note_metadata_refusal(
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:44  &mut failures,
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:45  "node_allowance exceeds the admitted native record bound".into(),
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:46  );
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:49  crate::booleanlaunch::note_metadata_refusal(&mut failures, why);
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:84  crate::booleanlaunch::note_metadata_refusal(
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:85  &mut failures,
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:86  "The serving store is not an existing absolute directory".into(),
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:87  );
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:121  return Err(format!(
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:122  "Saved single-stop comparison needs observation admission of at least {needed} bytes; BRUTEX_BOOLEAN_OBSERVATION_BYTES currently admits {ava
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:123  ));
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:130  return Err(format!(
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:131  "Saved single-stop comparison must fit independent cold numerical replay admission before launch: bootstrap work {}, CSCV work {}, estimated
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:132  replay.bootstrap_work, replay.split_work, replay.memory_bytes
GAP prod LINE crates/api/src/indexstoplaunch_metadata.rs:133  ));
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:42:16 True  if number("node_allowance").is_some_and(|nodes| nodes > config.capture.records) {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:48:20 True  if let Err(why) = observation_limits(config) {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:52:20 False  if let Some(fields) = policy.as_object_mut() {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:83:8 True  if !root.is_absolute() || !root.is_dir() {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:83:31 True  if !root.is_absolute() || !root.is_dir() {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:120:8 True  if available < needed {
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:126:8 True  if config.qualification.bootstrap_work > replay.bootstrap_work
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:127:12 True  || config.qualification.split_work > replay.split_work
GAP prod BRANCH crates/api/src/indexstoplaunch_metadata.rs:128:12 True  || config.qualification.memory_bytes > replay.memory_bytes
GAP test LINE crates/api/src/indexstoplaunch_tests.rs:571  child.kill().unwrap();
GAP test LINE crates/api/src/indexstoplaunch_tests.rs:572  child.wait().unwrap();
GAP test LINE crates/api/src/indexstoplaunch_tests.rs:573  return Err("private launch child exceeded45seconds/1MiB".into());
GAP test BRANCH crates/api/src/indexstoplaunch_tests.rs:539:12 False  if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
GAP test BRANCH crates/api/src/indexstoplaunch_tests.rs:568:12 True  if start.elapsed() > std::time::Duration::from_secs(45)
GAP test BRANCH crates/api/src/indexstoplaunch_tests.rs:569:16 True  || std::fs::metadata(&log).unwrap().len() > 1024 * 1024
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:58  _ => return Err("unknown native qualification assessment period".into()),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:112  Ok(body) => {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:113  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:114  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:115  reply
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:117  refused(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:119  "native qualification response exceeds byte limit; no prefix returned",
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:150  let mut held = crate::detail::Checkout::take(CACHE.get_or_init(|| Mutex::new(None)));
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:151  if !held.as_ref().is_some_and(|value| {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:152  value.root == root
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:153  && value.identity == asked.identity
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:154  && value.budget == budget
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:155  && value.replay == replay
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:156  }) {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:157  *held = None;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:158  let reader = Reader::open_with_replay_bounds(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:159  root,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:160  asked.identity,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:161  budget.bytes(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:162  budget.bytes() / 64,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:163  replay,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:165  .map_err(|why| {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:166  budget.context(&format!(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:167  "Native qualification {} unavailable under {}: {why}; independent cold replay bootstrap work {} and split work {}, estimated numeric-memory 
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:168  hex(asked.identity),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:169  root.display(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:170  replay.bootstrap_work,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:171  replay.split_work,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:172  replay.memory_bytes
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:173  ))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:174  })?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:175  let (ranked, summary) = reader.with_current(|| rank(&reader, budget.bytes()))?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:176  *held = Some(Cached {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:177  root: root.to_owned(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:178  identity: asked.identity,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:179  budget,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:180  replay,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:181  reader,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:182  ranked,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:183  summary,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:184  });
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:185  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:186  let cached = held
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:187  .as_ref()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:188  .ok_or("native qualification cache admission absent")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:189  if let Err(why) = cached.reader.require_current() {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:192  *held = None;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:193  return Err(why);
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:194  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:195  cached.reader.with_current(|| project(cached, asked))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:205  fn rank(reader: &Reader, budget: u64) -> Result<(Vec<(i64, usize)>, Value), String> {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:206  let memory = u64::try_from(reader.rows().len())
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:207  .map_err(display)?
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:208  .checked_mul(u64::try_from(std::mem::size_of::<(i64, usize)>()).map_err(display)?)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:209  .ok_or("native rank extent overflow")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:210  if reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:211  .admitted_bytes()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:212  .checked_add(memory)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:213  .is_none_or(|bytes| bytes > budget)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:215  return Err("native ranking plus retained evidence exceeds read admission".into());
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:216  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:217  let mut ranked = Vec::new();
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:218  ranked
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:219  .try_reserve_exact(reader.rows().len())
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:220  .map_err(display)?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:221  let (mut admitted, mut rejected, mut refused, mut unmeasured, mut combined) =
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:222  (0_u64, 0_u64, 0_u64, 0_u64, 0_u64);
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:223  for (index, row) in reader.rows().iter().enumerate() {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:224  let after = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:225  .later()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:226  .records()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:227  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:228  .ok_or("native ranking later coordinate absent")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:229  let daily = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:230  .consistencies()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:231  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:232  .ok_or("native ranking day assessment absent")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:233  ranked.push((after.metrics().pessimistic_paisa, index));
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:234  match row.projection.verdict().status() {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:235  cli::boolean_evidence::AdmissionStatusV1::Admitted => admitted += 1,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:236  cli::boolean_evidence::AdmissionStatusV1::Rejected => rejected += 1,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:237  cli::boolean_evidence::AdmissionStatusV1::Refused => refused += 1,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:238  cli::boolean_evidence::AdmissionStatusV1::Unmeasured => unmeasured += 1,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:240  combined += u64::from(daily.combined_qualifies(row.projection.verdict().status()));
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:242  sort_ranks(&mut ranked);
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:243  Ok((
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:244  ranked,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:245  json!({"institutional_admitted":admitted.to_string(),"rejected":rejected.to_string(),"refused":refused.to_string(),"unmeasured":unmeasured.t
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:246  ))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:247  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:251  fn project(cached: &Cached, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:252  let reader = &cached.reader;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:253  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:254  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:255  return Err("native qualification pin differs; no replacement page returned".into());
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:256  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:257  if matches!(asked.kind.as_str(), "index-days" | "index-weeks") {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:258  return days(reader, asked);
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:259  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:260  let original = json!({"identity":hex(reader.training().identity()),"completion":hex(reader.training().completion_digest())});
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:261  let later = json!({"identity":hex(reader.later().identity()),"completion":hex(reader.later().completion_digest())});
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:262  if asked.kind == "folds" {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:263  let index = asked.setting.ok_or("native fold setting missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:264  let row = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:265  .rows()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:266  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:267  .ok_or("native fold setting outside saved extent")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:268  let rows = window(&row.folds, asked)?
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:269  .iter()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:270  .enumerate()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:271  .map(|(n, row)| fold(asked.offset + n, row))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:272  .collect::<Vec<_>>();
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:273  return Ok(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:274  json!({"schema_version":1,"status":"saved","model":MODEL,"kind":"folds","identity":hex(reader.identity()),"completion":hex(pin),"original":o
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:275  "setting":index.to_string(),"total":row.folds.len().to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"rows":rows,"refusal":
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:276  );
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:277  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:278  let rows = window(&cached.ranked, asked)?
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:279  .iter()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:280  .enumerate()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:281  .map(|(n, (_, index))| setting(reader, *index, asked.offset + n + 1))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:282  .collect::<Result<Vec<_>, _>>()?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:283  Ok(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:284  json!({"schema_version":1,"status":"saved","model":MODEL,"kind":"settings","identity":hex(reader.identity()),"completion":hex(pin),"search_i
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:285  "batch":reader.allocation().batch().to_string(),"rung":reader.allocation().rung().to_string(),"original":original,"later":later,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:286  "total":reader.rows().len().to_string(),"offset":asked.offset.to_string(),"limit":asked.limit,"summary":cached.summary,"ranking":"later_pess
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:287  "policy":admission_projection::policy(&reader.policy()),"rows":rows,"refusal":null}),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:288  )
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:289  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:290  pub(crate) fn setting(reader: &Reader, index: usize, rank: usize) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:291  let row = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:292  .rows()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:293  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:294  .ok_or("native comparison coordinate missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:295  let original = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:296  .training()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:297  .records()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:298  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:299  .ok_or("native original coordinate missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:300  let later = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:301  .later()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:302  .records()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:303  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:304  .ok_or("native later coordinate missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:305  let record = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:306  .consistencies()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:307  .get(index)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:308  .ok_or("native daily coordinate missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:309  let institutional = admission_projection::row(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:310  index,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:311  &cli::boolean_evidence::AdmissionRow {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:312  identity: row.later_run,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:313  source_index: index,
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:314  values: row.projection.values(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:315  verdict: row.projection.verdict(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:316  },
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:317  )?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:319  json!({"index":index.to_string(),"rank":rank.to_string(),"original":crate::indexstopjson::setting(index,original),"later":crate::indexstopjs
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:320  "index_consistency":consistency::saved_record(reader.identity(),reader.completion_digest(),index,record,reader.consistency_receipt(),row.pro
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:322  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:323  fn days(reader: &Reader, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:324  let setting = asked.setting.ok_or("native day/week setting missing")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:325  let record = reader
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:326  .consistencies()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:327  .get(setting)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:328  .ok_or("native day/week setting outside saved extent")?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:329  let (evaluation, days) = period(record, &asked.period)?;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:330  let (total, rows) = if asked.kind == "index-weeks" {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:332  evaluation.weeks.len(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:333  window(&evaluation.weeks, asked)?
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:334  .iter()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:335  .enumerate()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:336  .map(|(n, row)| consistency::week(asked.offset + n, row))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:337  .collect::<Vec<_>>(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:341  days.len(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:342  window(days, asked)?
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:343  .iter()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:344  .enumerate()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:345  .map(|(n, row)| consistency::day(asked.offset + n, *row, evaluation.policy))
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:346  .collect(),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:349  let receipt = reader.consistency_receipt();
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:350  Ok(
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:351  json!({"schema_version":1,"status":"saved","model":MODEL,"kind":asked.kind,"identity":hex(reader.identity()),"completion":hex(reader.complet
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:352  "receipt":{"identity":hex(receipt.identity),"completion":hex(receipt.completion)},"period":asked.period,"setting":setting.to_string(),"total
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:353  )
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:354  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:381  _ => Err("native assessment period unknown".into()),
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:394  fn hex(value: [u8; 32]) -> String {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:395  crate::server::hex32(value)
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:396  }
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:397  fn display(value: impl std::fmt::Display) -> String {
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:398  value.to_string()
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:399  }
GAP test LINE crates/api/src/indexstopqualificationjson.rs:587  )?;
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:70:12 True  if identity == [0; 32]
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:71:16 True  || completion == Some([0; 32])
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:114:16 True  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:114:16 False  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:151:8 True  if !held.as_ref().is_some_and(|value| {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:151:8 False  if !held.as_ref().is_some_and(|value| {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:152:9 True  value.root == root
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:152:9 False  value.root == root
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:153:16 True  && value.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:153:16 False  && value.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:154:16 True  && value.budget == budget
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:154:16 False  && value.budget == budget
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:189:12 True  if let Err(why) = cached.reader.require_current() {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:189:12 False  if let Err(why) = cached.reader.require_current() {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:210:8 True  if reader
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:210:8 False  if reader
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:254:8 True  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:254:8 False  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:262:8 True  if asked.kind == "folds" {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:262:8 False  if asked.kind == "folds" {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:330:28 True  let (total, rows) = if asked.kind == "index-weeks" {
GAP prod BRANCH crates/api/src/indexstopqualificationjson.rs:330:28 False  let (total, rows) = if asked.kind == "index-weeks" {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:22  const fn label(self) -> &'static str {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:23  match self {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:24  Self::All => "all",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:25  Self::Qualified => "qualified",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:27  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:130  Ok(body) => {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:131  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:132  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:133  refused(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:135  "cumulative response exceeds byte admission; no prefix returned",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:138  reply
GAP prod LINE crates/api/src/indexstoprankingjson.rs:196  old.root == root
GAP prod LINE crates/api/src/indexstoprankingjson.rs:197  && old.identity == asked.identity
GAP prod LINE crates/api/src/indexstoprankingjson.rs:198  && old.rung == asked.rung
GAP prod LINE crates/api/src/indexstoprankingjson.rs:199  && old.budget == budget
GAP prod LINE crates/api/src/indexstoprankingjson.rs:200  && old.reader.checkpoint() == pin
GAP prod LINE crates/api/src/indexstoprankingjson.rs:201  }) {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:206  let (ranked, qualified, summary) = rank(&reader, budget.bytes)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:207  *held = Some(Cached {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:208  root: root.to_owned(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:209  identity: asked.identity,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:210  rung: asked.rung,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:211  budget,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:212  reader,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:213  ranked,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:214  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:215  summary,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:216  });
GAP prod LINE crates/api/src/indexstoprankingjson.rs:217  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:218  let cached = held.as_ref().ok_or("cumulative cache admission missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:219  let result = cached.reader.with_current(|| project(cached, asked));
GAP prod LINE crates/api/src/indexstoprankingjson.rs:220  if result.is_err() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:221  *held = None;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:222  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:223  result
GAP prod LINE crates/api/src/indexstoprankingjson.rs:233  fn rank(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:234  reader.with_current(|| rank_current(reader, budget))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:235  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:237  fn rank_current(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:238  let count = reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:239  .families()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:240  .iter()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:241  .try_fold(0_usize, |count, family| {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:242  count
GAP prod LINE crates/api/src/indexstoprankingjson.rs:243  .checked_add(family.reader.rows().len())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:244  .ok_or("cumulative rank count overflow")
GAP prod LINE crates/api/src/indexstoprankingjson.rs:245  })?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:246  let extra = u64::try_from(count)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:247  .map_err(display)?
GAP prod LINE crates/api/src/indexstoprankingjson.rs:248  .checked_mul(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:249  u64::try_from(std::mem::size_of::<Rank>() + 2 * std::mem::size_of::<usize>())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:250  .map_err(display)?,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:252  .ok_or("cumulative rank byte count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:253  if reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:254  .admitted_bytes()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:255  .checked_add(reader.projection_scratch_bytes())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:256  .and_then(|bytes| bytes.checked_add(extra))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:257  .is_none_or(|bytes| bytes > budget)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:259  return Err("cumulative evidence and complete rank indexes exceed read admission".into());
GAP prod LINE crates/api/src/indexstoprankingjson.rs:260  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:261  let mut rows = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:262  rows.try_reserve_exact(count).map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:263  let mut counts = [0_u64; 5];
GAP prod LINE crates/api/src/indexstoprankingjson.rs:264  for (family_index, family) in reader.families().iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:265  let first = rows.len();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:266  for (index, row) in family.reader.rows().iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:267  let later = family
GAP prod LINE crates/api/src/indexstoprankingjson.rs:268  .reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:269  .later()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:270  .records()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:271  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:272  .ok_or("cumulative later coordinate absent")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:273  let daily = family
GAP prod LINE crates/api/src/indexstoprankingjson.rs:274  .reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:275  .consistencies()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:276  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:277  .ok_or("cumulative daily coordinate absent")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:278  let state = row.projection.verdict().status();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:279  let qualified = daily.combined_qualifies(state);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:280  let slot = match state {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:281  cli::boolean_evidence::AdmissionStatusV1::Admitted => 0,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:282  cli::boolean_evidence::AdmissionStatusV1::Rejected => 1,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:283  cli::boolean_evidence::AdmissionStatusV1::Refused => 2,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:284  cli::boolean_evidence::AdmissionStatusV1::Unmeasured => 3,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:286  let count = counts
GAP prod LINE crates/api/src/indexstoprankingjson.rs:287  .get_mut(slot)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:288  .ok_or("cumulative verdict slot unavailable")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:289  *count = count
GAP prod LINE crates/api/src/indexstoprankingjson.rs:290  .checked_add(1)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:291  .ok_or("cumulative verdict count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:292  counts[4] = counts[4]
GAP prod LINE crates/api/src/indexstoprankingjson.rs:293  .checked_add(u64::from(qualified))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:294  .ok_or("cumulative combined count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:295  rows.push(Rank {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:296  score: later.metrics().pessimistic_paisa,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:297  family: family_index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:298  batch: family.batch,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:299  setting: index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:300  family_position: 0,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:301  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:302  });
GAP prod LINE crates/api/src/indexstoprankingjson.rs:306  let family_rows = rows
GAP prod LINE crates/api/src/indexstoprankingjson.rs:307  .get_mut(first..)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:308  .ok_or("cumulative family rank extent missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:309  sort_ranks(family_rows);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:310  for (rank, row) in family_rows.iter_mut().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:311  row.family_position = rank + 1;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:312  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:314  sort_ranks(&mut rows);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:315  let mut qualified = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:316  qualified
GAP prod LINE crates/api/src/indexstoprankingjson.rs:317  .try_reserve_exact(usize::try_from(counts[4]).map_err(display)?)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:318  .map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:319  for (index, row) in rows.iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:320  if row.qualified {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:321  qualified.push(index);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:322  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:324  Ok((
GAP prod LINE crates/api/src/indexstoprankingjson.rs:325  rows,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:326  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:327  json!({"institutional_admitted":counts[0].to_string(),"rejected":counts[1].to_string(),"refused":counts[2].to_string(),"unmeasured":counts[3
GAP prod LINE crates/api/src/indexstoprankingjson.rs:328  ))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:329  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:330  fn project(cached: &Cached, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:331  let reader = &cached.reader;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:332  let total = match asked.filter {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:333  Filter::All => cached.ranked.len(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:334  Filter::Qualified => cached.qualified.len(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:336  if asked.offset > total {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:337  return Err("cumulative ranking page lies outside its pinned filtered extent".into());
GAP prod LINE crates/api/src/indexstoprankingjson.rs:338  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:339  let end = asked.offset.saturating_add(asked.limit).min(total);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:340  let mut rows = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:341  rows.try_reserve_exact(end - asked.offset)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:342  .map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:343  for index in asked.offset..end {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:344  let global = match asked.filter {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:345  Filter::All => index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:346  Filter::Qualified => *cached
GAP prod LINE crates/api/src/indexstoprankingjson.rs:347  .qualified
GAP prod LINE crates/api/src/indexstoprankingjson.rs:348  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:349  .ok_or("qualified rank index missing")?,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:351  let ranked = cached
GAP prod LINE crates/api/src/indexstoprankingjson.rs:352  .ranked
GAP prod LINE crates/api/src/indexstoprankingjson.rs:353  .get(global)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:354  .ok_or("cumulative rank index missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:355  let family = reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:356  .families()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:357  .get(ranked.family)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:358  .ok_or("cumulative family missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:361  rows.push(row(&family.reader, *ranked, index + 1, global + 1)?);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:363  let (sequence, pin) = reader.checkpoint();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:364  let (grammar, bootstrap, splits) = reader.charged_work();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:365  let declared = reader.declaration();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:366  let timeframe = cli::EVERY_RUNG
GAP prod LINE crates/api/src/indexstoprankingjson.rs:367  .get(cached.rung)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:368  .ok_or("cumulative timeframe unavailable")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:370  json!({"schema_version":1,"status":"saved","model":MODEL,"identity":hex(cached.identity),"timeframe":timeframe,"rung":cached.rung.to_string(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:371  "checkpoint":{"sequence":sequence.to_string(),"completion":hex(pin)},
GAP prod LINE crates/api/src/indexstoprankingjson.rs:372  "scope":{"completed_batches":reader.completed_batches().to_string(),"completed_programs":reader.completed_programs().to_string(),"completed_
GAP prod LINE crates/api/src/indexstoprankingjson.rs:373  "included_families":reader.families().len().to_string(),"first_batch":reader.families().first().map(|family|family.batch.to_string()),"last_
GAP prod LINE crates/api/src/indexstoprankingjson.rs:374  "pending_next_batch":reader.pending(),"exhausted":reader.exhausted(),"interrupted_checkpoints":reader.interrupted().to_string(),"writer_obse
GAP prod LINE crates/api/src/indexstoprankingjson.rs:375  "instrument":declared.index(),"feed":declared.feed(),"training_first_day":declared.training_days().0.to_string(),"training_last_day":declare
GAP prod LINE crates/api/src/indexstoprankingjson.rs:376  "ranking":"later_pessimistic_descending_then_batch_then_setting","filter":asked.filter.label(),"total":total.to_string(),"all_total":cached.
GAP prod LINE crates/api/src/indexstoprankingjson.rs:377  "read_work":{"retained_serialized_bytes":reader.admitted_bytes().to_string(),"charged_grammar_nodes":grammar.to_string(),"charged_required_b
GAP prod LINE crates/api/src/indexstoprankingjson.rs:378  "cold":"complete_acknowledged_prefix_then_global_sort","warm":"ancestor_generation_checks_then_bounded_indexed_page"},"rows":rows,"refusal":
GAP prod LINE crates/api/src/indexstoprankingjson.rs:380  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:381  fn row(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:382  reader: &cli::index_stop_qualification::Reader,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:383  ranked: Rank,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:384  rank: usize,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:385  global_rank: usize,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:386  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:387  let index = ranked.setting;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:388  let mut value = crate::indexstopqualificationjson::setting(reader, index, rank)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:389  let fields = value
GAP prod LINE crates/api/src/indexstoprankingjson.rs:390  .as_object_mut()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:391  .ok_or("native setting serializer returned a nonobject")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:392  let extra = json!({"global_rank":global_rank.to_string(),"batch":ranked.batch.to_string(),"qualification_rank":ranked.family_position.to_str
GAP prod LINE crates/api/src/indexstoprankingjson.rs:393  "qualification":{"identity":hex(reader.identity()),"completion":hex(reader.completion_digest())},"qualification_total":reader.rows().len().t
GAP prod LINE crates/api/src/indexstoprankingjson.rs:394  "original_source":{"identity":hex(reader.training().identity()),"completion":hex(reader.training().completion_digest())},"later_source":{"id
GAP prod LINE crates/api/src/indexstoprankingjson.rs:395  fields.extend(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:396  extra
GAP prod LINE crates/api/src/indexstoprankingjson.rs:397  .as_object()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:398  .ok_or("native ranking links returned a nonobject")?
GAP prod LINE crates/api/src/indexstoprankingjson.rs:399  .clone(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:401  Ok(value)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:402  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:403  fn hex(value: [u8; 32]) -> String {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:404  crate::server::hex32(value)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:405  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:406  fn display(value: impl std::fmt::Display) -> String {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:407  value.to_string()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:408  }
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:88:12 True  if identity == [0; 32]
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:89:16 True  || completion == Some([0; 32])
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:132:16 True  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:132:16 False  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:195:8 False  if !held.as_ref().is_some_and(|old| {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:196:9 True  old.root == root
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:196:9 False  old.root == root
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:197:16 True  && old.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:197:16 False  && old.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:198:16 True  && old.rung == asked.rung
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:198:16 False  && old.rung == asked.rung
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:199:16 True  && old.budget == budget
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:199:16 False  && old.budget == budget
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:220:8 True  if result.is_err() {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:220:8 False  if result.is_err() {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:253:8 True  if reader
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:253:8 False  if reader
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:320:12 True  if row.qualified {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:320:12 False  if row.qualified {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:336:8 True  if asked.offset > total {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:336:8 False  if asked.offset > total {
GAP prod LINE crates/api/src/indexstopvixjson.rs:105  Ok(body) => {
GAP prod LINE crates/api/src/indexstopvixjson.rs:106  let result = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstopvixjson.rs:107  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstopvixjson.rs:108  result
GAP prod LINE crates/api/src/indexstopvixjson.rs:110  refused(
GAP prod LINE crates/api/src/indexstopvixjson.rs:112  "saved-VIX response exceeds byte admission; no prefix returned",
GAP test LINE crates/api/src/indexstopvixjson.rs:337  return Err("generated exact reference fixture must contain a candle".into());
GAP test LINE crates/api/src/indexstopvixjson.rs:380  [read: a closure passed inside the compared expression and never called: not the failing arm]  |_| {
GAP test LINE crates/api/src/indexstopvixjson.rs:381  rechecked = true;
GAP test LINE crates/api/src/indexstopvixjson.rs:382  Ok(())
GAP test LINE crates/api/src/indexstopvixjson.rs:383  }
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:107:16 True  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:107:16 False  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:151:16 False  && cached.identity == asked.identity
GAP test BRANCH crates/api/src/indexstopvixjson.rs:336:13 False  let Stamp::Exact(candle) = &mut original else {
GAP prod LINE crates/api/src/ingest.rs:504  Self::Everything => Universe::NONE,
GAP prod LINE crates/api/src/ingest.rs:869  _ => write!(f, "REFUSED · a member field this build cannot describe"),
GAP prod LINE crates/api/src/ingest.rs:898  _ => write!(f, "REFUSED · a rung this build cannot describe"),
GAP prod LINE crates/api/src/ingest.rs:992  Self::UnreadableRate { ref got } => write!(
GAP prod LINE crates/api/src/ingest.rs:993  f,
GAP prod LINE crates/api/src/ingest.rs:1334  return Err(bad());
GAP prod LINE crates/api/src/ingest.rs:1485  Refusal::UnreadableRate { .. } => Some("rate"),
GAP prod LINE crates/api/src/ingest.rs:1546  None => event,
GAP prod LINE crates/api/src/ingest.rs:1850  got: why.to_string(),
GAP prod LINE crates/api/src/ingest.rs:1851  })?,
GAP test LINE crates/api/src/ingest.rs:3677  rung.dir()
GAP test LINE crates/api/src/ingest.rs:3712  rung.dir()
GAP test LINE crates/api/src/ingest.rs:3748  feed.wire()
GAP prod LINE crates/api/src/ingest.rs:4896  pub fn discovery_ask(request: &FnoRequest) -> pull::fno::Ask {
GAP prod LINE crates/api/src/ingest.rs:4898  underlying: request.underlying.as_str().to_owned(),
GAP prod LINE crates/api/src/ingest.rs:4902  year: request.expiry.unwrap_or_else(|| request.window.to()).year(),
GAP prod LINE crates/api/src/ingest.rs:4903  month: request
GAP prod LINE crates/api/src/ingest.rs:4904  .expiry
GAP prod LINE crates/api/src/ingest.rs:4905  .unwrap_or_else(|| request.window.to())
GAP prod LINE crates/api/src/ingest.rs:4906  .month(),
GAP prod LINE crates/api/src/ingest.rs:4911  expiry: String::new(),
GAP prod LINE crates/api/src/ingest.rs:4913  }
GAP prod LINE crates/api/src/ingest.rs:4979  Series::Futures => found.contract.is_future(),
GAP prod LINE crates/api/src/ingest.rs:4996  found.expiry.year() == one.year()
GAP prod LINE crates/api/src/ingest.rs:4997  && found.expiry.month() == one.month()
GAP prod LINE crates/api/src/ingest.rs:4998  && found.expiry.day() == one.day()
GAP prod LINE crates/api/src/ingest.rs:4999  });
GAP prod BRANCH crates/api/src/ingest.rs:1332:9 False  let (Ok(year), Ok(month), Ok(day)) = (y.parse::<u16>(), m.parse::<u8>(), d.parse::<u8>())
GAP prod BRANCH crates/api/src/ingest.rs:1388:24 True  if y.is_empty() || m.is_empty() || d.is_empty() {
GAP prod BRANCH crates/api/src/ingest.rs:1388:40 True  if y.is_empty() || m.is_empty() || d.is_empty() {
GAP prod BRANCH crates/api/src/ingest.rs:1758:12 True  || !param(body, "expiry_y").is_empty()
GAP prod BRANCH crates/api/src/ingest.rs:1759:12 True  || !param(body, "expiry_m").is_empty()
GAP prod BRANCH crates/api/src/ingest.rs:4996:17 True  found.expiry.year() == one.year()
GAP prod BRANCH crates/api/src/ingest.rs:4996:17 False  found.expiry.year() == one.year()
GAP prod BRANCH crates/api/src/ingest.rs:4997:24 True  && found.expiry.month() == one.month()
GAP prod BRANCH crates/api/src/ingest.rs:4997:24 False  && found.expiry.month() == one.month()
GAP prod BRANCH crates/api/src/ingest.rs:5000:13 False  right_series && right_expiry
GAP prod LINE crates/api/src/ingest_refusals.rs:174  continue;
GAP prod LINE crates/api/src/ingest_refusals.rs:249  Err(why) => {
GAP prod LINE crates/api/src/ingest_refusals.rs:250  errors.push(json!({ "symbol": symbol, "error": why }));
GAP prod LINE crates/api/src/ingest_refusals.rs:251  }
GAP prod LINE crates/api/src/ingest_refusals.rs:291  Ok(body) => (axum::http::StatusCode::OK, json_header(), body.to_string()),
GAP prod BRANCH crates/api/src/ingest_refusals.rs:133:12 True  if symbol.len() > 64
GAP prod BRANCH crates/api/src/ingest_refusals.rs:173:13 False  let Some(month) = day.get(..7) else {
GAP prod LINE crates/api/src/instruments_route.rs:209  Err(why) => (
GAP prod LINE crates/api/src/instruments_route.rs:210  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/instruments_route.rs:211  headers,
GAP prod LINE crates/api/src/instruments_route.rs:212  format!(r#"{{"error":{}}}"#, render::json_string(&why)).into(),
GAP prod LINE crates/api/src/instruments_route.rs:213  ),
GAP prod BRANCH crates/api/src/instruments_route.rs:194:51 False  || (master == MasterState::Unavailable && feed.publishes_master())
GAP prod BRANCH crates/api/src/instruments_route.rs:204:20 False  if let Ok(value) = axum::http::HeaderValue::from_str(&tagged.etag) {
GAP prod BRANCH crates/api/src/instruments_route.rs:303:17 False  crate::catalog::tracked(entry.universe)
GAP test BRANCH crates/api/src/isolated.rs:118:15 False  let ran = output.status.success() && stdout.contains("1 passed");
GAP prod LINE crates/api/src/ladder.rs:464  return out;
GAP test LINE crates/api/src/ladder.rs:853  _ => None,
GAP prod BRANCH crates/api/src/ladder.rs:455:16 False  if let Ok(ym) = at.year_month() {
GAP prod BRANCH crates/api/src/ladder.rs:462:13 False  let Ok(next) = pull::session::Day::from_days(last.days_from_epoch().saturating_add(1))
GAP test BRANCH crates/api/src/ladder.rs:885:13 False  k.timeframe == Timeframe::DAY_1
GAP test BRANCH crates/api/src/ladder.rs:926:13 False  let Gate::RungFirst {
GAP test BRANCH crates/api/src/ladder.rs:1042:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && k.month >= at(2024, 11);
GAP test BRANCH crates/api/src/ladder.rs:1059:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && k.month != at(2024, 10);
GAP prod LINE crates/api/src/livejson.rs:135  Err(why) => return unavailable(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why),
GAP prod LINE crates/api/src/logs.rs:1548  None => Ok(None),
GAP prod LINE crates/api/src/logs.rs:1553  Ok(None) => return None,
GAP prod LINE crates/api/src/logs.rs:2468  q.push_str("&ring=1");
GAP prod LINE crates/api/src/logs.rs:3324  Self::Day => "day",
GAP prod LINE crates/api/src/logs.rs:3402  Err(_) => About::Unfiled,
GAP prod LINE crates/api/src/logs.rs:3428  return None;
GAP prod LINE crates/api/src/logs.rs:3456  return None;
GAP prod LINE crates/api/src/logs.rs:3485  return None;
GAP prod LINE crates/api/src/logs.rs:3583  out.push(("feed", feed.clone()));
GAP prod LINE crates/api/src/logs.rs:3584  if gdfl {
GAP prod LINE crates/api/src/logs.rs:3585  out.push(("target", pull::gdfl_import::TARGET.to_owned()));
GAP prod LINE crates/api/src/logs.rs:3586  }
GAP prod LINE crates/api/src/logs.rs:4006  continue;
GAP prod LINE crates/api/src/logs.rs:4059  )? {
GAP prod LINE crates/api/src/logs.rs:4082  unknown.push("its last line's time cannot be read");
GAP prod LINE crates/api/src/logs.rs:4422  None => break,
GAP prod LINE crates/api/src/logs.rs:4440  return None;
GAP prod LINE crates/api/src/logs.rs:4753  Err(why) => {
GAP prod LINE crates/api/src/logs.rs:4754  out.errors.push(why);
GAP prod LINE crates/api/src/logs.rs:4755  opened.slot_of.push(None);
GAP prod LINE crates/api/src/logs.rs:4756  opened.fixed.push(from);
GAP prod LINE crates/api/src/logs.rs:4757  }
GAP prod LINE crates/api/src/logs.rs:4830  break;
GAP prod LINE crates/api/src/logs.rs:4927  Err(e) => {
GAP prod LINE crates/api/src/logs.rs:4928  out.errors.push(format!("{}: {e}", path.display()));
GAP prod LINE crates/api/src/logs.rs:4929  continue;
GAP prod LINE crates/api/src/logs.rs:4933  continue;
GAP prod LINE crates/api/src/logs.rs:4936  out.hit_cap = true;
GAP prod LINE crates/api/src/logs.rs:4937  break;
GAP prod LINE crates/api/src/logs.rs:4947  out.errors.push(format!("{}: {e}", path.display()));
GAP prod LINE crates/api/src/logs.rs:4948  continue;
GAP prod LINE crates/api/src/logs.rs:4994  return NO_LOG_JSON.to_owned();
GAP prod LINE crates/api/src/logs.rs:5076  right
GAP prod LINE crates/api/src/logs.rs:5077  .3
GAP prod LINE crates/api/src/logs.rs:5078  .at_unix_millis
GAP prod LINE crates/api/src/logs.rs:5079  .cmp(&left.3.at_unix_millis)
GAP prod LINE crates/api/src/logs.rs:5080  .then_with(|| right.3.seq.cmp(&left.3.seq))
GAP prod LINE crates/api/src/logs.rs:5081  });
GAP prod LINE crates/api/src/logs.rs:5085  right
GAP prod LINE crates/api/src/logs.rs:5086  .3
GAP prod LINE crates/api/src/logs.rs:5087  .at_unix_millis
GAP prod LINE crates/api/src/logs.rs:5088  .cmp(&left.3.at_unix_millis)
GAP prod LINE crates/api/src/logs.rs:5089  .then_with(|| right.3.seq.cmp(&left.3.seq))
GAP prod LINE crates/api/src/logs.rs:5090  });
GAP prod LINE crates/api/src/logs.rs:5502  _ => {}
GAP prod LINE crates/api/src/logs.rs:5519  }
GAP prod LINE crates/api/src/logs.rs:5548  self.malformed = self.malformed.saturating_add(1);
GAP prod LINE crates/api/src/logs.rs:5549  return;
GAP prod LINE crates/api/src/logs.rs:5563  }
GAP prod LINE crates/api/src/logs.rs:5570  return;
GAP prod LINE crates/api/src/logs.rs:5890  several => {
GAP prod LINE crates/api/src/logs.rs:5891  let lists: Vec<&[telemetry::Record]> =
GAP prod LINE crates/api/src/logs.rs:5892  several.iter().map(|(_, records)| *records).collect();
GAP prod LINE crates/api/src/logs.rs:5893  std::borrow::Cow::Owned(merged_oldest_first(&lists, |record| record.at_unix_millis))
GAP prod LINE crates/api/src/logs.rs:6028  }
GAP test LINE crates/api/src/logs.rs:7119  }
GAP test LINE crates/api/src/logs.rs:9354  let _restored =
GAP test LINE crates/api/src/logs.rs:9355  std::fs::set_permissions(&current, std::fs::Permissions::from_mode(0o644));
GAP test LINE crates/api/src/logs.rs:9767  super::Probe::Damaged(why) => panic!("no damage in this file: {why}"),
GAP test LINE crates/api/src/logs.rs:9814  Ok(super::Probe::Damaged(why)) => format!("damaged: {why}"),
GAP test LINE crates/api/src/logs.rs:9815  Err(why) => format!("refused: {why}"),
GAP test LINE crates/api/src/logs.rs:12014  None
GAP prod BRANCH crates/api/src/logs.rs:1381:12 True  || query.target.is_some()
GAP prod BRANCH crates/api/src/logs.rs:1382:12 True  || query.run.is_some()
GAP prod BRANCH crates/api/src/logs.rs:2467:8 True  if asked.ring {
GAP prod BRANCH crates/api/src/logs.rs:2572:33 True  .map(|(from, until)| if to { until } else { from })
GAP prod BRANCH crates/api/src/logs.rs:2572:33 False  .map(|(from, until)| if to { until } else { from })
GAP prod BRANCH crates/api/src/logs.rs:2573:22 True  .filter(|at| *at != i64::MIN && *at != i64::MAX)
GAP prod BRANCH crates/api/src/logs.rs:2573:22 False  .filter(|at| *at != i64::MIN && *at != i64::MAX)
GAP prod BRANCH crates/api/src/logs.rs:2664:16 False  if let Some(route) = DB_READS.iter().copied().find(|route| *route == path) {
GAP prod BRANCH crates/api/src/logs.rs:3401:19 False  Err(_) if !about.ends_with("CE") && !about.ends_with("PE") => About::WholeDay { segment },
GAP prod BRANCH crates/api/src/logs.rs:3401:45 False  Err(_) if !about.ends_with("CE") && !about.ends_with("PE") => About::WholeDay { segment },
GAP prod BRANCH crates/api/src/logs.rs:3425:19 False  } else if text_of(record, "month") == asked.get(..7) {
GAP prod BRANCH crates/api/src/logs.rs:3454:12 True  && run_of(record) != Some(run)
GAP prod BRANCH crates/api/src/logs.rs:3483:13 True  && (record.at_unix_millis < from || record.at_unix_millis > to)
GAP prod BRANCH crates/api/src/logs.rs:3483:45 True  && (record.at_unix_millis < from || record.at_unix_millis > to)
GAP prod BRANCH crates/api/src/logs.rs:3561:12 False  if gdfl {
GAP prod BRANCH crates/api/src/logs.rs:3569:12 False  if gdfl {
GAP prod BRANCH crates/api/src/logs.rs:3582:19 True  } else if let Some(ref feed) = keys.feed {
GAP prod BRANCH crates/api/src/logs.rs:3584:12 True  if gdfl {
GAP prod BRANCH crates/api/src/logs.rs:3584:12 False  if gdfl {
GAP prod BRANCH crates/api/src/logs.rs:3589:19 False  } else if let Some((from, to)) = keys.span {
GAP prod BRANCH crates/api/src/logs.rs:4005:13 False  let Some(path) = telemetry::audit_path_for(dir, current, serial) else {
GAP prod BRANCH crates/api/src/logs.rs:4081:17 True  if !open && span.last_ms.is_none() {
GAP prod BRANCH crates/api/src/logs.rs:4439:12 True  if *budget == 0 {
GAP prod BRANCH crates/api/src/logs.rs:4453:20 False  && let Some(&(at, _)) = self.page.pointers.get(index.saturating_sub(1))
GAP prod BRANCH crates/api/src/logs.rs:4608:20 False  && let Some(why) = stream.unread_at.and_then(|at| out.errors.get_mut(at))
GAP prod BRANCH crates/api/src/logs.rs:4829:13 False  let (Some(at), Some((pointer, record))) = (chosen.head_at(), chosen.head.take()) else {
GAP prod BRANCH crates/api/src/logs.rs:4843:28 False  && held.serial == pointer.serial
GAP prod BRANCH crates/api/src/logs.rs:4847:24 False  && let Some(at) = other.head_at()
GAP prod BRANCH crates/api/src/logs.rs:4897:5 False  asked.level.is_none_or(|level| record.level.at_least(level))
GAP prod BRANCH crates/api/src/logs.rs:4926:23 False  Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
GAP prod BRANCH crates/api/src/logs.rs:4932:12 True  if len == 0 {
GAP prod BRANCH crates/api/src/logs.rs:4935:12 True  if left == 0 {
GAP prod BRANCH crates/api/src/logs.rs:4946:16 True  if let Err(e) = read {
GAP prod BRANCH crates/api/src/logs.rs:4959:20 False  if let Ok(record) = telemetry::Record::decode(line)
GAP prod BRANCH crates/api/src/logs.rs:4993:9 False  let Some(pair) = named_pair(halves) else {
GAP prod BRANCH crates/api/src/logs.rs:5067:20 False  if seen.insert((half, record.seq)) {
GAP prod BRANCH crates/api/src/logs.rs:5484:34 False  "command started" if record.run != 0 => {
GAP prod BRANCH crates/api/src/logs.rs:5495:35 False  "command finished" if self.current == Some(record.run) => {
GAP prod BRANCH crates/api/src/logs.rs:5496:24 False  if let Some(attempt) = self.attempts.get_mut(&record.run) {
GAP prod BRANCH crates/api/src/logs.rs:5511:16 False  if let Some(attempt) = self.attempts.get_mut(&id) {
GAP prod BRANCH crates/api/src/logs.rs:5514:24 False  && let Some(run) = record.field("run").and_then(telemetry::OwnedValue::as_u64)
GAP prod BRANCH crates/api/src/logs.rs:5515:20 False  && !attempt.importer_runs.contains(&run)
GAP prod BRANCH crates/api/src/logs.rs:5525:16 False  if !filed.contains(&id) {
GAP prod BRANCH crates/api/src/logs.rs:5547:13 False  let Ok(record) = telemetry::Record::decode(line) else {
GAP prod BRANCH crates/api/src/logs.rs:5557:20 False  if let Some(kind) = text_of(&record, "kind") {
GAP prod BRANCH crates/api/src/logs.rs:5569:13 False  let (Some(kind), Some(day)) = (text_of(&record, "kind"), text_of(&record, "day")) else {
GAP prod BRANCH crates/api/src/logs.rs:5970:20 False  if !out.contains(id) {
GAP prod BRANCH crates/api/src/logs.rs:5997:9 False  self.dirs.len() == other.len()
GAP prod BRANCH crates/api/src/logs.rs:5999:17 False  mine.half == theirs.half
GAP prod BRANCH crates/api/src/logs.rs:6023:16 False  if let (Some(list), Some(head)) = (lists.get(n), heads.get_mut(n)) {
GAP prod BRANCH crates/api/src/logs.rs:6024:20 False  if let Some(item) = list.get(*head) {
GAP prod BRANCH crates/api/src/logs.rs:6663:9 False  record.target == pull::gdfl_import::TARGET
GAP prod BRANCH crates/api/src/logs.rs:6664:16 False  && record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/logs.rs:7114:20 False  if let [newer, older] = pair {
GAP test BRANCH crates/api/src/logs.rs:8643:38 False  [inside a closure whose false side does not fail the check]  .any(|(param, word)| *param == "ring" && word == "yes")
GAP test BRANCH crates/api/src/logs.rs:8666:13 True  [the true side of a condition is a passing path]  keyed_page.contains("/logs.json?limit=50&amp;feed=gdfl&amp;day=2022-03-07")
GAP test BRANCH crates/api/src/logs.rs:9353:12 True  if std::fs::read(&current).is_ok() {
GAP test BRANCH crates/api/src/logs.rs:10853:28 False  [inside a closure whose false side does not fail the check]  .any(|why| why.contains("logs/cli") && why.contains("not read yet")),
GAP test BRANCH crates/api/src/logs.rs:11798:28 False  [inside a closure whose false side does not fail the check]  .any(|why| why.starts_with("logs/cli audit stream:")
GAP test LINE crates/api/src/logs_units_tests.rs:250  other => panic!("a file is no log directory: {}", other.word()),
GAP prod LINE crates/api/src/main.rs:44  Err(why) => {
GAP prod LINE crates/api/src/main.rs:45  eprintln!("FAILED: the async runtime could not start: {why}");
GAP prod LINE crates/api/src/main.rs:46  note_exit(api::server::FAILED, count, false);
GAP prod LINE crates/api/src/main.rs:47  return std::process::ExitCode::from(api::server::FAILED);
GAP prod LINE crates/api/src/main.rs:60  let _inside = runtime.enter();
GAP prod LINE crates/api/src/main.rs:61  Box::pin(api::speed::stop_asked())
GAP prod BRANCH crates/api/src/main.rs:59:22 True  Ok(words) if api::server::catches_stops(words) => {
GAP prod LINE crates/api/src/master.rs:337  "directory"
GAP prod LINE crates/api/src/master.rs:357  return Err(format!(
GAP prod LINE crates/api/src/master.rs:358  "{}: grew past {MAX_MASTER_BYTES} bytes while it was read; this \
GAP prod LINE crates/api/src/master.rs:359  reader holds at most {MAX_MASTER_BYTES}",
GAP prod LINE crates/api/src/master.rs:360  path.display()
GAP prod LINE crates/api/src/master.rs:361  ));
GAP test LINE crates/api/src/master.rs:1076  let _ = std::fs::OpenOptions::new().write(true).open(&fifo);
GAP prod BRANCH crates/api/src/master.rs:336:16 True  if meta.is_dir() {
GAP prod BRANCH crates/api/src/master.rs:356:8 True  if text.len() as u64 > MAX_MASTER_BYTES {
GAP test BRANCH crates/api/src/master.rs:1075:12 True  if read.is_err() {
GAP prod LINE crates/api/src/mastersrun.rs:154  "the master was refetched and had not changed"
GAP prod LINE crates/api/src/mastersrun.rs:222  masters::Verdict::Reprime => "refused_repriming",
GAP prod LINE crates/api/src/mastersrun.rs:351  Ok(ref wire) => {
GAP prod LINE crates/api/src/mastersrun.rs:352  let (got, steps) = obtain(wire, clock, dir, source).await;
GAP prod LINE crates/api/src/mastersrun.rs:353  *outcome = Ok(got);
GAP prod LINE crates/api/src/mastersrun.rs:354  *tried = steps;
GAP prod LINE crates/api/src/mastersrun.rs:391  telemetry::Level::Error
GAP prod LINE crates/api/src/mastersrun.rs:396  Err(_) => "the universe could NOT be re-parsed, so the previous one still stands",
GAP prod LINE crates/api/src/mastersrun.rs:469  || format!("`{}` — no URL was reachable", source.url),
GAP prod LINE crates/api/src/mastersrun.rs:494  return Err(
GAP prod LINE crates/api/src/mastersrun.rs:495  "Zerodha's descriptor is not an HTTP transport, so its instrument \
GAP prod LINE crates/api/src/mastersrun.rs:496  dump cannot be fetched"
GAP prod LINE crates/api/src/mastersrun.rs:497  .to_owned(),
GAP prod LINE crates/api/src/mastersrun.rs:498  );
GAP prod LINE crates/api/src/mastersrun.rs:548  }
GAP prod LINE crates/api/src/mastersrun.rs:560  Ok(answer) => answer,
GAP prod LINE crates/api/src/mastersrun.rs:561  Err(why) => (
GAP prod LINE crates/api/src/mastersrun.rs:562  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/mastersrun.rs:563  json_headers(),
GAP prod LINE crates/api/src/mastersrun.rs:564  format!(
GAP prod LINE crates/api/src/mastersrun.rs:565  r#"{{"landed":[],"refusal":{}}}"#,
GAP prod LINE crates/api/src/mastersrun.rs:566  crate::render::json_string(&format!(
GAP prod LINE crates/api/src/mastersrun.rs:567  "the refresh task did not return ({why}); /masters/status.json and /logs say what landed"
GAP prod LINE crates/api/src/mastersrun.rs:568  ))
GAP prod LINE crates/api/src/mastersrun.rs:569  ),
GAP prod LINE crates/api/src/mastersrun.rs:570  ),
GAP prod LINE crates/api/src/mastersrun.rs:572  }
GAP prod LINE crates/api/src/mastersrun.rs:593  let from = match pull::masters::PublicFetch::new() {
GAP prod LINE crates/api/src/mastersrun.rs:594  Ok(from) => from,
GAP prod LINE crates/api/src/mastersrun.rs:595  Err(why) => {
GAP prod LINE crates/api/src/mastersrun.rs:596  return (
GAP prod LINE crates/api/src/mastersrun.rs:597  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/mastersrun.rs:598  json_headers(),
GAP prod LINE crates/api/src/mastersrun.rs:599  format!(
GAP prod LINE crates/api/src/mastersrun.rs:600  r#"{{"landed":[],"refusal":{}}}"#,
GAP prod LINE crates/api/src/mastersrun.rs:601  crate::render::json_string(&why)
GAP prod LINE crates/api/src/mastersrun.rs:602  ),
GAP prod LINE crates/api/src/mastersrun.rs:603  );
GAP prod LINE crates/api/src/mastersrun.rs:607  refresh_over(&site, &dir, &from, &masters::Clock).await
GAP prod LINE crates/api/src/mastersrun.rs:651  telemetry::Level::Info
GAP prod LINE crates/api/src/mastersrun.rs:662  "the instrument masters were refreshed from the browser",
GAP prod LINE crates/api/src/mastersrun.rs:667  "landed" => telemetry::Value::Uint(written as u64),
GAP prod LINE crates/api/src/mastersrun.rs:668  "skipped" => telemetry::Value::Uint(
GAP prod LINE crates/api/src/mastersrun.rs:669  landed.iter().filter(|(_, l, _)| l.is_err()).count() as u64
GAP prod LINE crates/api/src/mastersrun.rs:671  "asked" => telemetry::Value::Uint(landed.len() as u64),
GAP prod LINE crates/api/src/mastersrun.rs:676  "waited_ms" => telemetry::Value::Uint(
GAP prod LINE crates/api/src/mastersrun.rs:677  landed.iter().map(|(_, _, t)| t.waited_ms()).sum::<u64>()
GAP prod LINE crates/api/src/mastersrun.rs:711  axum::http::StatusCode::OK
GAP prod LINE crates/api/src/mastersrun.rs:850  _ => "another feed",
GAP test LINE crates/api/src/mastersrun.rs:1326  ref other => format!("not a refusal: {other:?}"),
GAP test LINE crates/api/src/mastersrun.rs:1446  "NSE"
GAP test LINE crates/api/src/mastersrun.rs:1855  }
GAP prod BRANCH crates/api/src/mastersrun.rs:151:16 False  if changed {
GAP prod BRANCH crates/api/src/mastersrun.rs:388:12 False  if reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:493:9 False  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
GAP prod BRANCH crates/api/src/mastersrun.rs:586:9 True  let Ok(dir) = crate::server::masters_dir() else {
GAP prod BRANCH crates/api/src/mastersrun.rs:650:12 True  if written == landed.len() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:21 True  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:41 True  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:41 False  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:53 True  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:53 False  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP test BRANCH crates/api/src/mastersrun.rs:1384:31 False  [inside a closure whose false side does not fail the check]  .any(|record| record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/mastersrun.rs:1397:31 False  [inside a closure whose false side does not fail the check]  .any(|record| crate::emitted::says(record, "kind", "instruments")
GAP test BRANCH crates/api/src/mastersrun.rs:1445:31 True  } else if vendor == brutex_core::vendor::Vendor::Zerodha {
GAP test BRANCH crates/api/src/mastersrun.rs:1674:64 True  [inside a closure whose false side does not fail the check]  .any(|(_, l, _)| matches!(*l, Ok(ref inner) if inner.is_written())),
GAP test BRANCH crates/api/src/mastersrun.rs:1852:16 False  if let Landed::Refused(ref why) = landed {
GAP test BRANCH crates/api/src/mastersrun.rs:2233:21 False  crate::emitted::says(record, "url", url)
GAP prod LINE crates/api/src/merge.rs:698  || e.vendor_isins.get(vendor as usize).copied().flatten() != listing.isin))
GAP prod LINE crates/api/src/merge.rs:711  }
GAP prod LINE crates/api/src/merge.rs:718  }
GAP prod BRANCH crates/api/src/merge.rs:694:12 False  if let Some(slot) = e.ids.get_mut(vendor as usize) {
GAP prod BRANCH crates/api/src/merge.rs:697:21 False  && (*slot != Some(listing.vendor_id)
GAP prod BRANCH crates/api/src/merge.rs:698:24 True  || e.vendor_isins.get(vendor as usize).copied().flatten() != listing.isin))
GAP prod BRANCH crates/api/src/merge.rs:698:24 False  || e.vendor_isins.get(vendor as usize).copied().flatten() != listing.isin))
GAP prod BRANCH crates/api/src/merge.rs:712:12 False  if let Some(slot) = e.vendor_isins.get_mut(vendor as usize) {
GAP prod LINE crates/api/src/month_change.rs:267  break;
GAP prod BRANCH crates/api/src/month_change.rs:266:13 False  let Some(earlier) = month_before(at) else {
GAP test BRANCH crates/api/src/nse_identity.rs:502:13 False  let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
GAP test BRANCH crates/api/src/nse_identity.rs:523:13 False  let Err(Refusal::ZerodhaUndated(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
GAP test BRANCH crates/api/src/nse_identity.rs:531:13 False  let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
GAP test BRANCH crates/api/src/nse_identity.rs:540:13 False  let Err(Refusal::FileUnavailable(why)) = Reference::read(&dir).confirm(&merged, "RELIANCE")
GAP prod LINE crates/api/src/operation_audit.rs:154  Err(why) => failure(
GAP prod LINE crates/api/src/operation_audit.rs:155  &format!(
GAP prod LINE crates/api/src/operation_audit.rs:156  "the audited handler did not return ({why}); its invocation record holds the outcome"
GAP prod LINE crates/api/src/operation_audit.rs:157  ),
GAP prod LINE crates/api/src/operation_audit.rs:160  .into_response(),
GAP prod LINE crates/api/src/operation_audit.rs:189  let mut refusal = failure(
GAP prod LINE crates/api/src/operation_audit.rs:190  "bounded request audit capacity is full; no handler was dispatched and no work was queued",
GAP prod LINE crates/api/src/operation_audit.rs:193  refusal.0 = StatusCode::TOO_MANY_REQUESTS;
GAP prod LINE crates/api/src/operation_audit.rs:194  return refusal.into_response();
GAP prod LINE crates/api/src/operation_audit.rs:196  Err(why) => {
GAP prod LINE crates/api/src/operation_audit.rs:197  return failure(
GAP prod LINE crates/api/src/operation_audit.rs:198  &format!("bounded request audit could not start: {why:?}"),
GAP prod LINE crates/api/src/operation_audit.rs:201  .into_response();
GAP prod LINE crates/api/src/operation_audit.rs:234  Err(why) => failure(
GAP prod LINE crates/api/src/operation_audit.rs:235  &format!("bounded terminal audit could not settle: {why:?}"),
GAP prod LINE crates/api/src/operation_audit.rs:238  .into_response(),
GAP prod LINE crates/api/src/operation_audit.rs:268  return Err("audit query exceeds 96 bytes".to_owned());
GAP prod LINE crates/api/src/operation_audit.rs:285  return Err("duplicate audit query field".to_owned());
GAP prod LINE crates/api/src/operation_audit.rs:298  return Err("an exact invocation cannot also request a page".to_owned());
GAP prod LINE crates/api/src/operation_audit.rs:320  Err(why) => {
GAP prod LINE crates/api/src/operation_audit.rs:321  return (
GAP prod LINE crates/api/src/operation_audit.rs:322  StatusCode::BAD_REQUEST,
GAP prod LINE crates/api/src/operation_audit.rs:323  headers(),
GAP prod LINE crates/api/src/operation_audit.rs:324  serde_json::json!({"refusal":why}).to_string(),
GAP prod LINE crates/api/src/operation_audit.rs:325  );
GAP prod BRANCH crates/api/src/operation_audit.rs:226:20 False  if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
GAP prod BRANCH crates/api/src/operation_audit.rs:260:8 True  if raw.is_empty() || raw.starts_with('0') || !raw.bytes().all(|b| b.is_ascii_digit()) {
GAP prod BRANCH crates/api/src/operation_audit.rs:267:8 True  if query.len() > 96 {
GAP prod BRANCH crates/api/src/operation_audit.rs:284:12 True  if slot.is_some() {
GAP prod BRANCH crates/api/src/operation_audit.rs:297:12 True  if before.is_some() || limit.is_some() {
GAP prod BRANCH crates/api/src/operation_audit.rs:297:32 True  if before.is_some() || limit.is_some() {
GAP test LINE crates/api/src/operation_audit_tests.rs:422  async {
GAP test LINE crates/api/src/operation_audit_tests.rs:423  called.fetch_add(1, Ordering::Relaxed);
GAP test LINE crates/api/src/operation_audit_tests.rs:424  StatusCode::ACCEPTED.into_response()
GAP test LINE crates/api/src/operation_audit_tests.rs:425  },
GAP test LINE crates/api/src/operation_audit_tests.rs:451  })
GAP test LINE crates/api/src/operation_audit_tests.rs:453  });
GAP test LINE crates/api/src/operation_audit_tests.rs:483  });
GAP test LINE crates/api/src/operation_audit_tests.rs:517  called.fetch_add(1, Ordering::Relaxed);
GAP test LINE crates/api/src/operation_audit_tests.rs:518  StatusCode::OK.into_response()
GAP test LINE crates/api/src/operation_audit_tests.rs:519  })
GAP test BRANCH crates/api/src/operation_audit_tests.rs:321:14 False  } if b == before
GAP test LINE crates/api/src/ops_safety_tests.rs:79  }
GAP test LINE crates/api/src/ops_safety_tests.rs:80  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test LINE crates/api/src/ops_safety_tests.rs:378  counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
GAP test LINE crates/api/src/ops_safety_tests.rs:379  async { "wrote" }
GAP test LINE crates/api/src/ops_safety_tests.rs:380  }),
GAP test BRANCH crates/api/src/ops_safety_tests.rs:77:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/ops_safety_tests.rs:147:9 True  [an operand followed by ||: neither side fails the check by itself]  label == "unstamped" || (label.len() == 40 && label.bytes().all(|b| b.is_ascii_hexdigit())),
GAP test BRANCH crates/api/src/ops_safety_tests.rs:190:9 False  [an operand followed by ||: neither side fails the check by itself]  state == "behind" || state == "unreadable",
GAP test BRANCH crates/api/src/ops_safety_tests.rs:190:30 True  state == "behind" || state == "unreadable",
GAP test BRANCH crates/api/src/ops_safety_tests.rs:190:30 False  state == "behind" || state == "unreadable",
GAP test BRANCH crates/api/src/ops_safety_tests.rs:228:33 False  [inside a closure whose false side does not fail the check]  said.iter().any(|value| value.contains("manifest/REQUIRES")
GAP test BRANCH crates/api/src/ops_safety_tests.rs:229:16 False  [read: an operand inside an .any() closure opened on the line above: one element's false side does not fail the check]  && value.contains("line 3")
GAP prod LINE crates/api/src/optiongreeks.rs:213  if let Some(held) = kept.as_mut() {
GAP prod LINE crates/api/src/optiongreeks.rs:214  held.loads = loads;
GAP prod LINE crates/api/src/optiongreeks.rs:215  }
GAP prod LINE crates/api/src/optiongreeks.rs:438  pub fn answer(
GAP prod LINE crates/api/src/optiongreeks.rs:439  &self,
GAP prod LINE crates/api/src/optiongreeks.rs:440  store_root: &std::path::Path,
GAP prod LINE crates/api/src/optiongreeks.rs:441  rate: &RateTable,
GAP prod LINE crates/api/src/optiongreeks.rs:442  ) -> Result<(Vec<Cell>, Vec<String>), String> {
GAP prod LINE crates/api/src/optiongreeks.rs:443  self.answer_with(store_root, rate, &crate::bars::Cancel::never())
GAP prod LINE crates/api/src/optiongreeks.rs:444  }
GAP prod LINE crates/api/src/optiongreeks.rs:469  faults.push(refusal.message.clone());
GAP prod LINE crates/api/src/optiongreeks.rs:472  let why = if crate::detail::is_being_written(&refusal.message) {
GAP prod LINE crates/api/src/optiongreeks.rs:473  "option_being_written"
GAP prod LINE crates/api/src/optiongreeks.rs:475  "option_unreadable"
GAP prod LINE crates/api/src/optiongreeks.rs:477  Err((why, Some(refusal.message)))
GAP prod LINE crates/api/src/optiongreeks.rs:492  OpenRefusalKind::Invalid => return Err(refusal.message),
GAP prod LINE crates/api/src/optiongreeks.rs:543  faults.push(format!(
GAP prod LINE crates/api/src/optiongreeks.rs:545  file.path().display(),
GAP prod LINE crates/api/src/optiongreeks.rs:546  ts.div_euclid(MICROS)
GAP prod LINE crates/api/src/optiongreeks.rs:548  Spot::Unreadable
GAP prod LINE crates/api/src/optiongreeks.rs:549  })
GAP prod LINE crates/api/src/optiongreeks.rs:556  Spot::OutsideMonth => return Cell::refused("t_outside_month", None),
GAP prod LINE crates/api/src/optiongreeks.rs:557  Spot::OffGrid => return Cell::refused("t_off_grid", None),
GAP prod LINE crates/api/src/optiongreeks.rs:558  Spot::OrderBroken => return Cell::refused("option_order_broken", None),
GAP prod LINE crates/api/src/optiongreeks.rs:559  Spot::NoFile => return Cell::refused("no_option_file", None),
GAP prod LINE crates/api/src/optiongreeks.rs:560  Spot::Unreadable => return Cell::refused("option_unreadable", None),
GAP prod LINE crates/api/src/optiongreeks.rs:561  Spot::BeingWritten => return Cell::refused("option_being_written", None),
GAP prod LINE crates/api/src/optiongreeks.rs:613  return refused(
GAP prod LINE crates/api/src/optiongreeks.rs:614  "tenor_refused",
GAP prod LINE crates/api/src/optiongreeks.rs:615  format!("the stamp {ts} is not a trade day the rate calendar holds"),
GAP prod LINE crates/api/src/optiongreeks.rs:616  );
GAP prod LINE crates/api/src/optiongreeks.rs:794  Err(why) => (
GAP prod LINE crates/api/src/optiongreeks.rs:795  "null".to_owned(),
GAP prod LINE crates/api/src/optiongreeks.rs:796  render::json_string(&format!("unsettled: {why}")),
GAP prod LINE crates/api/src/optiongreeks.rs:797  ),
GAP prod LINE crates/api/src/optiongreeks.rs:818  render::json_string(&faults.join("; "))
GAP prod LINE crates/api/src/optiongreeks.rs:892  axum::http::StatusCode::PARTIAL_CONTENT
GAP prod LINE crates/api/src/optiongreeks.rs:925  Err(why) => not_admitted("a greeks read", &why),
GAP prod BRANCH crates/api/src/optiongreeks.rs:196:16 False  && held.path == path
GAP prod BRANCH crates/api/src/optiongreeks.rs:213:24 True  if let Some(held) = kept.as_mut() {
GAP prod BRANCH crates/api/src/optiongreeks.rs:213:24 False  if let Some(held) = kept.as_mut() {
GAP prod BRANCH crates/api/src/optiongreeks.rs:325:40 True  if y.len() != 4 || m.len() != 2 || d.len() != 2 {
GAP prod BRANCH crates/api/src/optiongreeks.rs:377:12 True  if exchange.is_empty() || segment.is_empty() || symbol.is_empty() {
GAP prod BRANCH crates/api/src/optiongreeks.rs:377:57 True  if exchange.is_empty() || segment.is_empty() || symbol.is_empty() {
GAP prod BRANCH crates/api/src/optiongreeks.rs:472:34 True  let why = if crate::detail::is_being_written(&refusal.message) {
GAP prod BRANCH crates/api/src/optiongreeks.rs:472:34 False  let why = if crate::detail::is_being_written(&refusal.message) {
GAP prod BRANCH crates/api/src/optiongreeks.rs:609:13 False  let Some(day) = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(MICROS))
GAP prod BRANCH crates/api/src/optiongreeks.rs:815:16 False  if faults.is_empty() {
GAP prod BRANCH crates/api/src/optiongreeks.rs:889:21 False  let status = if faults.is_empty() {
GAP test LINE crates/api/src/optiongreeks_route_tests.rs:479  other => format!("not unreadable: {other:?}"),
GAP test LINE crates/api/src/owed_tests.rs:396  }
GAP test BRANCH crates/api/src/owed_tests.rs:388:12 False  if let Some(top) = answer.as_object_mut() {
GAP test BRANCH crates/api/src/owed_tests.rs:392:16 False  if let Some(row) = month.as_object_mut() {
GAP prod LINE crates/api/src/page_report_route.rs:172  WRITTEN.load(Ordering::Relaxed)
GAP prod BRANCH crates/api/src/page_report_route.rs:169:28 False  let written_total = if written {
GAP test LINE crates/api/src/pool_table_tests.rs:49  const fn pool(
GAP test LINE crates/api/src/pool_table_tests.rs:50  named: Named,
GAP test LINE crates/api/src/pool_table_tests.rs:51  file: &'static str,
GAP test LINE crates/api/src/pool_table_tests.rs:52  function: &'static str,
GAP test LINE crates/api/src/pool_table_tests.rs:53  takes: &'static str,
GAP test LINE crates/api/src/pool_table_tests.rs:54  ) -> Runs {
GAP test LINE crates/api/src/pool_table_tests.rs:55  Runs::Pool {
GAP test LINE crates/api/src/pool_table_tests.rs:56  named,
GAP test LINE crates/api/src/pool_table_tests.rs:57  file,
GAP test LINE crates/api/src/pool_table_tests.rs:58  function,
GAP test LINE crates/api/src/pool_table_tests.rs:59  takes,
GAP test LINE crates/api/src/pool_table_tests.rs:60  }
GAP test LINE crates/api/src/pool_table_tests.rs:61  }
GAP test LINE crates/api/src/pool_table_tests.rs:659  }
GAP test LINE crates/api/src/pool_table_tests.rs:662  index += 3 + text.get(index + 3..)?.find('\'')?;
GAP test LINE crates/api/src/pool_table_tests.rs:676  None
GAP test LINE crates/api/src/pool_table_tests.rs:813  fn head_text(answer: &str) -> String {
GAP test LINE crates/api/src/pool_table_tests.rs:814  answer.chars().take(600).collect()
GAP test LINE crates/api/src/pool_table_tests.rs:815  }
GAP test BRANCH crates/api/src/pool_table_tests.rs:639:11 False  while index < bytes.len() {
GAP test BRANCH crates/api/src/pool_table_tests.rs:646:23 False  while index < bytes.len() && at(index) != b'"' {
GAP test BRANCH crates/api/src/pool_table_tests.rs:651:20 False  && !at(index.wrapping_sub(1)).is_ascii_alphanumeric()
GAP test BRANCH crates/api/src/pool_table_tests.rs:652:20 False  && at(index.wrapping_sub(1)) != b'_' =>
GAP test BRANCH crates/api/src/pool_table_tests.rs:655:20 False  if at(index + 1 + hashes) == b'"' {
GAP test BRANCH crates/api/src/pool_table_tests.rs:661:22 True  b'\'' if at(index + 1) == b'\\' => {
GAP test BRANCH crates/api/src/pool_table_tests.rs:664:22 False  b'\'' if at(index + 2) == b'\'' => index += 2,
GAP test BRANCH crates/api/src/pool_table_tests.rs:2022:16 False  && std::time::Instant::now() < deadline
GAP prod LINE crates/api/src/provenance.rs:412  }
GAP prod LINE crates/api/src/provenance.rs:422  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:423  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:425  asked,
GAP prod LINE crates/api/src/provenance.rs:426  &format!("{} {}: {why}", asked.key(), asked.month),
GAP prod LINE crates/api/src/provenance.rs:427  Value::Null,
GAP prod LINE crates/api/src/provenance.rs:445  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:446  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:448  asked,
GAP prod LINE crates/api/src/provenance.rs:449  &format!("{relative}: {why}"),
GAP prod LINE crates/api/src/provenance.rs:450  json!({ "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:475  Err(why @ StoreError::Missing { .. }) => {
GAP prod LINE crates/api/src/provenance.rs:476  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:478  asked,
GAP prod LINE crates/api/src/provenance.rs:479  &format!("{why}: this month is not held"),
GAP prod LINE crates/api/src/provenance.rs:480  json!({ "held": false, "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:483  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:484  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:486  asked,
GAP prod LINE crates/api/src/provenance.rs:487  &why.to_string(),
GAP prod LINE crates/api/src/provenance.rs:488  json!({ "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:523  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:524  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:526  asked,
GAP prod LINE crates/api/src/provenance.rs:527  &why,
GAP prod LINE crates/api/src/provenance.rs:528  json!({ "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:702  }
GAP prod LINE crates/api/src/provenance.rs:710  return "the month file holds no committed bar, so there is nothing to attribute"
GAP prod LINE crates/api/src/provenance.rs:711  .to_owned();
GAP prod LINE crates/api/src/provenance.rs:815  Self::Unborn { path, bytes } => json!({
GAP prod LINE crates/api/src/provenance.rs:816  "path": path,
GAP prod LINE crates/api/src/provenance.rs:817  "state": "unborn",
GAP prod LINE crates/api/src/provenance.rs:818  "records": 0,
GAP prod LINE crates/api/src/provenance.rs:819  "bytes": bytes,
GAP prod LINE crates/api/src/provenance.rs:820  "why": "its header never reached the disk, so it holds no record; the next \
GAP prod LINE crates/api/src/provenance.rs:857  Ok(Found::Unborn { bytes }) => SidecarRead::Unborn { path, bytes },
GAP prod LINE crates/api/src/provenance.rs:860  Err(why) => SidecarRead::Refused {
GAP prod LINE crates/api/src/provenance.rs:861  path,
GAP prod LINE crates/api/src/provenance.rs:862  why: why.to_string(),
GAP prod LINE crates/api/src/provenance.rs:863  },
GAP prod LINE crates/api/src/provenance.rs:954  (Err(why), _) => json!({ "source": "imports/gdfl.journal", "error": why }),
GAP prod LINE crates/api/src/provenance.rs:955  (Ok(_), None) => json!({
GAP prod LINE crates/api/src/provenance.rs:956  "source": "imports/gdfl.journal",
GAP prod LINE crates/api/src/provenance.rs:957  "error": format!("{} is not a day the calendar can name", record.day),
GAP prod LINE crates/api/src/provenance.rs:976  return Value::Null;
GAP prod LINE crates/api/src/provenance.rs:994  Err(fault) => json!({
GAP prod LINE crates/api/src/provenance.rs:995  "source": "audit/pull.journal",
GAP prod LINE crates/api/src/provenance.rs:996  "ordinal": entry.ordinal,
GAP prod LINE crates/api/src/provenance.rs:997  "error": fault.to_string(),
GAP prod LINE crates/api/src/provenance.rs:1000  None => json!({
GAP prod LINE crates/api/src/provenance.rs:1001  "source": "audit/pull.journal",
GAP prod LINE crates/api/src/provenance.rs:1002  "error": format!("record {} was not read", record.journal_ordinal),
GAP prod LINE crates/api/src/provenance.rs:1005  Err(why) => json!({ "source": "audit/pull.journal", "error": why }),
GAP prod LINE crates/api/src/provenance.rs:1034  Some(Err(why)) => return json!({ "source": source, "error": why }),
GAP prod LINE crates/api/src/provenance.rs:1035  None => return json!({ "source": source, "error": "the import journal was not read" }),
GAP prod LINE crates/api/src/provenance.rs:1038  return json!({ "source": source, "error": "the month names no calendar day" });
GAP prod LINE crates/api/src/provenance.rs:1079  Some(_) => {}
GAP prod LINE crates/api/src/provenance.rs:1151  Err(why) => json!({ "error": why }),
GAP prod LINE crates/api/src/provenance.rs:1237  Err(_) => (bars, errors.saturating_add(1)),
GAP prod LINE crates/api/src/provenance.rs:1300  Segment::Fno => None,
GAP prod LINE crates/api/src/provenance.rs:1372  }
GAP prod LINE crates/api/src/provenance.rs:1539  Err(why) => return Err(format!("{}: {why}", path.display())),
GAP prod LINE crates/api/src/provenance.rs:1577  return Err(format!(
GAP prod LINE crates/api/src/provenance.rs:1578  "{}: the file ended before record {} of {whole}",
GAP prod LINE crates/api/src/provenance.rs:1579  path.display(),
GAP prod LINE crates/api/src/provenance.rs:1580  tail.upto
GAP prod LINE crates/api/src/provenance.rs:1581  ));
GAP prod LINE crates/api/src/provenance.rs:1605  return;
GAP prod LINE crates/api/src/provenance.rs:1621  _ => index.faults = index.faults.saturating_add(1),
GAP prod LINE crates/api/src/provenance.rs:1645  Segment::Fno => audit::Scope::Fno,
GAP prod LINE crates/api/src/provenance.rs:1664  Err(why) => return json!({ "source": source, "error": why }),
GAP prod LINE crates/api/src/provenance.rs:1667  notes.push(format!(
GAP prod LINE crates/api/src/provenance.rs:1668  "the pull journal keeps no index of its runs, so this process's first read of it \
GAP prod LINE crates/api/src/provenance.rs:1669  decoded {} records in {} ms; later requests read only what is appended",
GAP prod LINE crates/api/src/provenance.rs:1670  read.read_now,
GAP prod LINE crates/api/src/provenance.rs:1671  read.read_micros / 1_000
GAP prod LINE crates/api/src/provenance.rs:1672  ));
GAP prod LINE crates/api/src/provenance.rs:1675  return json!({ "source": source, "error": "the month names no calendar day" });
GAP prod LINE crates/api/src/provenance.rs:1793  continue;
GAP prod LINE crates/api/src/provenance.rs:1820  ) {
GAP prod LINE crates/api/src/provenance.rs:1821  excluded.not_run = excluded.not_run.saturating_add(1);
GAP prod LINE crates/api/src/provenance.rs:1946  return Value::Null;
GAP prod LINE crates/api/src/provenance.rs:1956  pull::vendor::SpanStanding::Measured => "measured",
GAP prod LINE crates/api/src/provenance.rs:1963  Some(PARTLY_OUTSIDE_WINDOW)
GAP prod BRANCH crates/api/src/provenance.rs:407:16 False  if let Some(object) = body.as_object_mut() {
GAP prod BRANCH crates/api/src/provenance.rs:409:20 False  if let Value::Object(more) = extra {
GAP prod BRANCH crates/api/src/provenance.rs:437:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/provenance.rs:670:12 False  if let Some(object) = body.as_object_mut() {
GAP prod BRANCH crates/api/src/provenance.rs:709:8 True  if n_valid == 0 {
GAP prod BRANCH crates/api/src/provenance.rs:975:9 False  let Some((journal, records)) = &journals.pull else {
GAP prod BRANCH crates/api/src/provenance.rs:1037:9 False  let Some((from, to)) = asked.days() else {
GAP prod BRANCH crates/api/src/provenance.rs:1071:25 False  Some(id) if named.iter().all(|seen| *seen != id) => {
GAP prod BRANCH crates/api/src/provenance.rs:1211:35 True  Some((last, span)) if *last == day => return span.clone(),
GAP prod BRANCH crates/api/src/provenance.rs:1212:43 False  Some((last, Ok((_, end)))) if *last < day => Some(*end),
GAP prod BRANCH crates/api/src/provenance.rs:1363:8 False  if !fresh {
GAP prod BRANCH crates/api/src/provenance.rs:1448:25 False  if from == 0 && to == 0 {
GAP prod BRANCH crates/api/src/provenance.rs:1452:46 False  (Some(first), Some(last)) if first <= last && last - first < MAX_INDEXED_MONTHS => {
GAP prod BRANCH crates/api/src/provenance.rs:1528:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/provenance.rs:1555:37 True  if tail.identity != identity || whole < tail.upto {
GAP prod BRANCH crates/api/src/provenance.rs:1576:16 True  if got == 0 {
GAP prod BRANCH crates/api/src/provenance.rs:1604:9 False  let Ok(image) = <&[u8; audit::RECORD_LEN]>::try_from(image) else {
GAP prod BRANCH crates/api/src/provenance.rs:1616:23 False  Ok(record) if record.kind == audit::Kind::Run => index.push(PullRun {
GAP prod BRANCH crates/api/src/provenance.rs:1666:8 True  if read.read_now > FIRST_WALK_RECORDS {
GAP prod BRANCH crates/api/src/provenance.rs:1674:9 False  let Some((from, to)) = asked.days() else {
GAP prod BRANCH crates/api/src/provenance.rs:1792:13 False  let Some(run) = index.runs.get(position) else {
GAP prod BRANCH crates/api/src/provenance.rs:1796:55 True  if record.to_days < from.days_from_epoch() || record.from_days > to.days_from_epoch() {
GAP prod BRANCH crates/api/src/provenance.rs:1822:66 True  } else if record.outcome == audit::Outcome::Empty || record.bars_stored == 0 {
GAP prod BRANCH crates/api/src/provenance.rs:1849:20 False  && close.ordinal > open.ordinal
GAP prod BRANCH crates/api/src/provenance.rs:1850:20 False  && close.record.from_days == open.record.from_days
GAP prod BRANCH crates/api/src/provenance.rs:1851:20 False  && close.record.to_days == open.record.to_days
GAP prod BRANCH crates/api/src/provenance.rs:1852:20 False  && close.ran_at(Some(open.record.at_unix_secs))
GAP prod BRANCH crates/api/src/provenance.rs:1941:9 False  let Some((first, last)) = (Asked {
GAP prod BRANCH crates/api/src/provenance.rs:1949:19 False  let touches = first.days_from_epoch() <= span.to.days_from_epoch()
GAP prod BRANCH crates/api/src/provenance.rs:1962:19 True  } else if touches {
GAP prod LINE crates/api/src/pullrun.rs:876  return String::new();
GAP prod LINE crates/api/src/pullrun.rs:1726  continue;
GAP prod LINE crates/api/src/pullrun.rs:2093  break;
GAP prod LINE crates/api/src/pullrun.rs:3523  run.to_string()
GAP prod LINE crates/api/src/pullrun.rs:3552  sink.release_run(run);
GAP test LINE crates/api/src/pullrun.rs:4448  other => panic!("a disagreeing leg must be refused, got {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:4589  other => panic!("a repeated payload field must refuse the run: {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:4685  other => panic!("a four-part leg must be refused, got {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:4734  other => panic!("one leg past the bound must be named, got {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:4796  other => panic!("a leg naming no feed must be refused, got {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:7494  other => panic!("NOT KNOWN was expected, and the scan said {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:8597  &|_site, _leg| {
GAP test LINE crates/api/src/pullrun.rs:9152  other => panic!("NOT KNOWN was expected, and the scan said {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:9737  '+' => out.push_str("%2B"),
GAP test LINE crates/api/src/pullrun.rs:9770  other => panic!("a payload past the bound must refuse the run: {other:?}"),
GAP test LINE crates/api/src/pullrun.rs:11021  other => panic!("expected a guessed answer, got {other:?}"),
GAP prod BRANCH crates/api/src/pullrun.rs:875:8 True  if names.is_empty() {
GAP prod BRANCH crates/api/src/pullrun.rs:1725:13 False  let Some((_, made)) = read.get(*at) else {
GAP prod BRANCH crates/api/src/pullrun.rs:2092:12 True  if page.is_empty() {
GAP prod BRANCH crates/api/src/pullrun.rs:2890:29 False  out.push_str(if horizon.ends_soon { "true" } else { "false" });
GAP prod BRANCH crates/api/src/pullrun.rs:3520:8 False  if run == 0 {
GAP prod BRANCH crates/api/src/pullrun.rs:3551:16 True  if let (Some(sink), Some(run)) = (telemetry::global(), self.0) {
GAP prod BRANCH crates/api/src/pullrun.rs:4004:20 False  if let Some(feed) = progress.feeds.get_mut(nth) {
GAP prod BRANCH crates/api/src/pullrun.rs:4044:24 False  if let Some(feed) = progress.feeds.get_mut(nth) {
GAP prod BRANCH crates/api/src/pullrun.rs:4049:16 False  if let Some(held) = outcomes.get_mut(nth) {
GAP test BRANCH crates/api/src/pullrun.rs:4421:29 True  let series = if dir == "futures" { "fut" } else { "opt" };
GAP test BRANCH crates/api/src/pullrun.rs:4742:53 True  if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
GAP test BRANCH crates/api/src/pullrun.rs:5140:16 False  if let Some(progress) = site
GAP test BRANCH crates/api/src/pullrun.rs:9435:13 False  [an operand followed by ||: neither side fails the check by itself]  body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
GAP test BRANCH crates/api/src/pullrun.rs:9435:41 True  body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
GAP test BRANCH crates/api/src/pullrun.rs:9435:41 False  body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
GAP test BRANCH crates/api/src/pullrun.rs:9435:65 True  body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
GAP test BRANCH crates/api/src/pullrun.rs:9435:65 False  body.contains("F&amp;O") || body.contains("fno") || body.contains("F&O"),
GAP test LINE crates/api/src/pullrun_restart_tests.rs:114  Restart::NotKnown(why) => why.clone(),
GAP test LINE crates/api/src/pullrun_restart_tests.rs:116  other => panic!("expected NOT KNOWN, got {other:?}"),
GAP test LINE crates/api/src/pullrun_restart_tests.rs:633  other => panic!("expected a guessed NOT KNOWN, got {other:?}"),
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1415  fn restart_cost_with_64_open_runs() {
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1416  costs_with_64_open_runs(8_001, 200);
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1417  }
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1513  fn restart_cost_of_the_walks_worst_case() {
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1514  costs_of_the_walks_worst_case(200);
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1515  }
GAP test BRANCH crates/api/src/pullrun_restart_tests.rs:362:25 False  [inside a closure whose false side does not fail the check]  .any(|line| crate::emitted::counts(line, "unnamed", 6)
GAP prod LINE crates/api/src/quarantine_route.rs:132  cli::MISUSED => axum::http::StatusCode::BAD_REQUEST,
GAP prod LINE crates/api/src/quarantine_route.rs:133  _ => axum::http::StatusCode::UNPROCESSABLE_ENTITY,
GAP prod LINE crates/api/src/quarantine_route.rs:151  Err(why) => {
GAP prod LINE crates/api/src/quarantine_route.rs:152  return refused(
GAP prod LINE crates/api/src/quarantine_route.rs:154  "quarantine_question",
GAP prod LINE crates/api/src/quarantine_route.rs:155  &why,
GAP prod LINE crates/api/src/quarantine_route.rs:189  Err(why) => {
GAP prod LINE crates/api/src/quarantine_route.rs:190  return refused(
GAP prod LINE crates/api/src/quarantine_route.rs:192  "quarantine_question",
GAP prod LINE crates/api/src/quarantine_route.rs:193  &why,
GAP prod LINE crates/api/src/quarantine_route.rs:219  Err(why) => busy(&why),
GAP test LINE crates/api/src/quarantine_route_tests.rs:134  }
GAP test LINE crates/api/src/quarantine_route_tests.rs:135  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test LINE crates/api/src/quarantine_route_tests.rs:151  }
GAP test BRANCH crates/api/src/quarantine_route_tests.rs:132:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP test BRANCH crates/api/src/quarantine_route_tests.rs:142:12 False  if let Ok(entries) = fs::read_dir(dir) {
GAP test BRANCH crates/api/src/quarantine_route_tests.rs:210:9 True  [an operand followed by ||: neither side fails the check by itself]  text.contains(&february[0].ts_micros.to_string()) || text.contains("410"),
GAP test BRANCH crates/api/src/quarantine_route_tests.rs:398:12 False  if go_rx.recv().is_ok() {
GAP prod LINE crates/api/src/recovery.rs:184  return Err("recovery plan exceeds its reported 100,000-window bound; nothing was truncated".to_owned());
GAP prod LINE crates/api/src/recovery.rs:242  return Err("recovery plan has an unknown generation descriptor".to_owned());
GAP prod LINE crates/api/src/recovery.rs:245  return Err("recovery plan has an unknown generation policy".to_owned());
GAP prod LINE crates/api/src/recovery.rs:300  || !matches!(
GAP prod LINE crates/api/src/recovery.rs:305  return Err("recovery plan seal has invalid control state or quantities".to_owned());
GAP prod LINE crates/api/src/recovery.rs:325  return Err(
GAP prod LINE crates/api/src/recovery.rs:326  "recovery sealed window inventory does not match its original plan identity"
GAP prod LINE crates/api/src/recovery.rs:327  .to_owned(),
GAP prod LINE crates/api/src/recovery.rs:328  );
GAP prod LINE crates/api/src/recovery.rs:333  return Err(
GAP prod LINE crates/api/src/recovery.rs:334  "successor seal does not bind its exact scope, predecessor and generation"
GAP prod LINE crates/api/src/recovery.rs:335  .to_owned(),
GAP prod LINE crates/api/src/recovery.rs:336  );
GAP prod LINE crates/api/src/recovery.rs:353  return Err(
GAP prod LINE crates/api/src/recovery.rs:354  "previously activated recovery plan has no durable seal; history was not reseeded"
GAP prod LINE crates/api/src/recovery.rs:355  .to_owned(),
GAP prod LINE crates/api/src/recovery.rs:356  );
GAP prod LINE crates/api/src/recovery.rs:360  }
GAP prod LINE crates/api/src/recovery.rs:397  )?;
GAP prod LINE crates/api/src/recovery.rs:441  return Err("successor was already activated; preparation cannot reset it".to_owned());
GAP prod LINE crates/api/src/recovery.rs:447  return Err(
GAP prod LINE crates/api/src/recovery.rs:448  "predecessor plan exists; the missing-plan repair cannot replace it".to_owned(),
GAP prod LINE crates/api/src/recovery.rs:449  );
GAP prod LINE crates/api/src/recovery.rs:471  return Err(
GAP prod LINE crates/api/src/recovery.rs:472  "existing successor contains changed scope, activation or work; nothing was reset"
GAP prod LINE crates/api/src/recovery.rs:473  .to_owned(),
GAP prod LINE crates/api/src/recovery.rs:474  );
GAP prod LINE crates/api/src/recovery.rs:480  return Err(
GAP prod LINE crates/api/src/recovery.rs:481  "existing successor window disagrees with the requested queued scope"
GAP prod LINE crates/api/src/recovery.rs:482  .to_owned(),
GAP prod LINE crates/api/src/recovery.rs:483  );
GAP prod LINE crates/api/src/recovery.rs:511  return Err("successor must be prepared with its exact predecessor and generation before activation".to_owned());
GAP prod LINE crates/api/src/recovery.rs:519  return Err("prepared successor scope is incomplete; activation refused".to_owned());
GAP prod LINE crates/api/src/recovery.rs:633  return (
GAP prod LINE crates/api/src/recovery.rs:634  StatusCode::CONFLICT,
GAP prod LINE crates/api/src/recovery.rs:635  headers,
GAP prod LINE crates/api/src/recovery.rs:636  serde_json::json!({"started":false,"why":why}).to_string(),
GAP prod LINE crates/api/src/recovery.rs:637  );
GAP prod LINE crates/api/src/recovery.rs:648  return (
GAP prod LINE crates/api/src/recovery.rs:649  StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/recovery.rs:650  headers,
GAP prod LINE crates/api/src/recovery.rs:651  serde_json::json!({"started":false,"why":why}).to_string(),
GAP prod LINE crates/api/src/recovery.rs:652  );
GAP prod LINE crates/api/src/recovery.rs:669  result => {
GAP prod LINE crates/api/src/recovery.rs:670  let why = result
GAP prod LINE crates/api/src/recovery.rs:671  .err()
GAP prod LINE crates/api/src/recovery.rs:672  .unwrap_or_else(|| "stopped during durable activation".to_owned());
GAP prod LINE crates/api/src/recovery.rs:673  let stop = crate::recovery_control::stop(&site);
GAP prod LINE crates/api/src/recovery.rs:674  crate::recovery_control::idle(&site);
GAP prod LINE crates/api/src/recovery.rs:675  let why = stop.err().map_or(why.clone(), |error| {
GAP prod LINE crates/api/src/recovery.rs:676  format!("{why}; stop persistence: {error}")
GAP prod LINE crates/api/src/recovery.rs:677  });
GAP prod LINE crates/api/src/recovery.rs:678  update(&site, |progress| {
GAP prod LINE crates/api/src/recovery.rs:679  progress.finished = Some(format!("Recovery BLOCKED: {why}"));
GAP prod LINE crates/api/src/recovery.rs:680  });
GAP prod LINE crates/api/src/recovery.rs:681  return Err(why);
GAP prod LINE crates/api/src/recovery.rs:733  let _task = tokio::spawn(drive(site, id, prepared, false));
GAP prod LINE crates/api/src/recovery.rs:734  })
GAP prod LINE crates/api/src/recovery.rs:769  return Ok(None);
GAP prod LINE crates/api/src/recovery.rs:773  return Err("active recovery identity mismatch".to_owned());
GAP prod LINE crates/api/src/recovery.rs:779  if why.kind() == std::io::ErrorKind::NotFound {
GAP prod LINE crates/api/src/recovery.rs:780  missing_plan(id)
GAP prod LINE crates/api/src/recovery.rs:782  failure(why)
GAP prod LINE crates/api/src/recovery.rs:784  })?;
GAP test LINE crates/api/src/recovery.rs:801  let _task = tokio::spawn(drive(site, id, prepared, false));
GAP test LINE crates/api/src/recovery.rs:802  })
GAP prod LINE crates/api/src/recovery.rs:814  return Ok(false);
GAP prod LINE crates/api/src/recovery.rs:818  return Err("active recovery identity mismatch".to_owned());
GAP prod LINE crates/api/src/recovery.rs:827  failure(why)
GAP prod LINE crates/api/src/recovery.rs:866  Err(why) => return Err(failure(why)),
GAP prod LINE crates/api/src/recovery.rs:883  return Err("recovery plan was not durably seeded; no vendor work is allowed".to_owned());
GAP prod LINE crates/api/src/recovery.rs:918  Err(why) => return Err(failure(why)),
GAP prod LINE crates/api/src/recovery.rs:981  Status::Blocked
GAP prod LINE crates/api/src/recovery.rs:1023  |master| {
GAP prod LINE crates/api/src/recovery.rs:1024  let proof = master.provenance();
GAP prod LINE crates/api/src/recovery.rs:1025  format!(
GAP prod LINE crates/api/src/recovery.rs:1028  hex(proof.sha256),
GAP prod LINE crates/api/src/recovery.rs:1031  },
GAP prod LINE crates/api/src/recovery.rs:1071  return Err("stopped at a durable window boundary".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1093  Ok(mut found) => {
GAP prod LINE crates/api/src/recovery.rs:1094  for day in &found.retry_days {
GAP prod LINE crates/api/src/recovery.rs:1095  retry_day(site, journal, attempts, &item.body, *day, &lifecycle).await?;
GAP prod LINE crates/api/src/recovery.rs:1097  if !found.retry_days.is_empty() {
GAP prod LINE crates/api/src/recovery.rs:1098  found = assess(site, &item.body, &lifecycle).await?;
GAP prod LINE crates/api/src/recovery.rs:1099  }
GAP prod LINE crates/api/src/recovery.rs:1100  item.status = found.status();
GAP prod LINE crates/api/src/recovery.rs:1101  item.missing = found.missing;
GAP prod LINE crates/api/src/recovery.rs:1102  item.unverified = found.unverified;
GAP prod LINE crates/api/src/recovery.rs:1103  item.diagnostics = found.evidence_issues;
GAP prod LINE crates/api/src/recovery.rs:1135  Status::Verified => counts.0 += 1,
GAP prod LINE crates/api/src/recovery.rs:1136  Status::NotApplicable => counts.1 += 1,
GAP prod LINE crates/api/src/recovery.rs:1137  Status::Unverified => counts.2 += 1,
GAP prod LINE crates/api/src/recovery.rs:1162  return Err("shared recovery attempt index reached its 100,000-unit bound".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1239  return Err("stopped while reconciling interrupted requests".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1251  let asked = checked(&item.body, today)?;
GAP prod LINE crates/api/src/recovery.rs:1252  if item.key != key(&item.body)
GAP prod LINE crates/api/src/recovery.rs:1253  || asked.window.days() != 1
GAP prod LINE crates/api/src/recovery.rs:1254  || server::param(&item.body, "recovery_kind") != "gap"
GAP prod LINE crates/api/src/recovery.rs:1256  return Err("shared attempt has invalid identity or non-day scope".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1257  }
GAP prod LINE crates/api/src/recovery.rs:1258  if !windows
GAP prod LINE crates/api/src/recovery.rs:1259  .iter()
GAP prod LINE crates/api/src/recovery.rs:1260  .any(|window| asked.window.from() >= window.from() && asked.window.to() <= window.to())
GAP prod LINE crates/api/src/recovery.rs:1262  continue;
GAP prod LINE crates/api/src/recovery.rs:1263  }
GAP prod LINE crates/api/src/recovery.rs:1264  match assess(site, &item.body, lifecycle).await {
GAP prod LINE crates/api/src/recovery.rs:1265  Ok(found) => match reassessed(&item, &found) {
GAP prod LINE crates/api/src/recovery.rs:1266  Some(next) => item = next,
GAP prod LINE crates/api/src/recovery.rs:1268  None => continue,
GAP prod LINE crates/api/src/recovery.rs:1270  Err(why) => {
GAP prod LINE crates/api/src/recovery.rs:1271  item.status = Status::Blocked;
GAP prod LINE crates/api/src/recovery.rs:1272  item.diagnostics = item.diagnostics.saturating_add(1);
GAP prod LINE crates/api/src/recovery.rs:1273  note(&item.body, &why);
GAP prod LINE crates/api/src/recovery.rs:1274  }
GAP prod LINE crates/api/src/recovery.rs:1276  append_attempt(journal, attempts, item)?;
GAP prod LINE crates/api/src/recovery.rs:1331  Status::Exhausted
GAP prod LINE crates/api/src/recovery.rs:1344  return Err("recovery unit must name exactly one symbol".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1405  return Err("source month exceeds the reported recovery read bound".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1411  return Err(
GAP prod LINE crates/api/src/recovery.rs:1412  "source timestamp belongs to a different instrument-month window".to_owned(),
GAP prod LINE crates/api/src/recovery.rs:1413  );
GAP prod LINE crates/api/src/recovery.rs:1436  fn read_sources(
GAP prod LINE crates/api/src/recovery.rs:1437  root: &Path,
GAP prod LINE crates/api/src/recovery.rs:1438  name: &str,
GAP prod LINE crates/api/src/recovery.rs:1439  month: store::path::YearMonth,
GAP prod LINE crates/api/src/recovery.rs:1440  window: Window,
GAP prod LINE crates/api/src/recovery.rs:1441  is_daily: bool,
GAP prod LINE crates/api/src/recovery.rs:1442  ) -> Result<(Vec<i64>, Vec<i64>), String> {
GAP prod LINE crates/api/src/recovery.rs:1443  let daily = read_stamps(root, name, "1day", month)?;
GAP prod LINE crates/api/src/recovery.rs:1444  let held_days = days(&daily)?;
GAP prod LINE crates/api/src/recovery.rs:1445  if held_days.len() != daily.len() {
GAP prod LINE crates/api/src/recovery.rs:1446  return Err("multiple daily bars name the same date".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1447  }
GAP prod LINE crates/api/src/recovery.rs:1448  let need_minutes = !is_daily
GAP prod LINE crates/api/src/recovery.rs:1449  || (window.from().days_from_epoch()..=window.to().days_from_epoch()).any(|number| {
GAP prod LINE crates/api/src/recovery.rs:1450  Day::from_days(number).is_ok_and(|day| {
GAP prod LINE crates/api/src/recovery.rs:1451  !held_days.contains(&day)
GAP prod LINE crates/api/src/recovery.rs:1452  && matches!(
GAP prod LINE crates/api/src/recovery.rs:1453  pull::calendar::kind_of(i64::from(number)),
GAP prod LINE crates/api/src/recovery.rs:1456  })
GAP prod LINE crates/api/src/recovery.rs:1457  });
GAP prod LINE crates/api/src/recovery.rs:1458  let minutes = if need_minutes {
GAP prod LINE crates/api/src/recovery.rs:1459  read_stamps(root, name, "1min", month)?
GAP prod LINE crates/api/src/recovery.rs:1461  Vec::new()
GAP prod LINE crates/api/src/recovery.rs:1463  Ok((daily, minutes))
GAP prod LINE crates/api/src/recovery.rs:1464  }
GAP prod LINE crates/api/src/recovery.rs:1469  let name = symbol(&asked)?.to_owned();
GAP prod LINE crates/api/src/recovery.rs:1470  let month = asked.window.from().year_month().map_err(failure)?;
GAP prod LINE crates/api/src/recovery.rs:1471  if month != asked.window.to().year_month().map_err(failure)? {
GAP prod LINE crates/api/src/recovery.rs:1472  return Err("recovery scan must be contained in one month".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1473  }
GAP prod LINE crates/api/src/recovery.rs:1474  let source = site.store_root.clone();
GAP prod LINE crates/api/src/recovery.rs:1475  let read_name = name.clone();
GAP prod LINE crates/api/src/recovery.rs:1476  let window = asked.window;
GAP prod LINE crates/api/src/recovery.rs:1477  let is_daily = asked.granularity == Granularity::Day1;
GAP prod LINE crates/api/src/recovery.rs:1478  let (daily, minutes) = tokio::task::spawn_blocking(move || {
GAP prod LINE crates/api/src/recovery.rs:1479  read_sources(&source, &read_name, month, window, is_daily)
GAP prod LINE crates/api/src/recovery.rs:1480  })
GAP prod LINE crates/api/src/recovery.rs:1481  .await
GAP prod LINE crates/api/src/recovery.rs:1482  .map_err(failure)??;
GAP prod LINE crates/api/src/recovery.rs:1483  let daily_days = days(&daily)?;
GAP prod LINE crates/api/src/recovery.rs:1484  let minute_days = days(&minutes)?;
GAP prod LINE crates/api/src/recovery.rs:1485  let first = i64::from(asked.window.from().days_from_epoch());
GAP prod LINE crates/api/src/recovery.rs:1486  let last = i64::from(asked.window.to().days_from_epoch());
GAP prod LINE crates/api/src/recovery.rs:1487  if asked.granularity == Granularity::Day1 {
GAP prod LINE crates/api/src/recovery.rs:1488  let mut found = assess_daily(asked.window, &daily_days, &minute_days, &name)?;
GAP prod LINE crates/api/src/recovery.rs:1489  qualify_lifecycle(
GAP prod LINE crates/api/src/recovery.rs:1490  &mut found,
GAP prod LINE crates/api/src/recovery.rs:1491  lifecycle,
GAP prod LINE crates/api/src/recovery.rs:1492  &name,
GAP prod LINE crates/api/src/recovery.rs:1493  asked.window,
GAP prod LINE crates/api/src/recovery.rs:1494  &daily_days,
GAP prod LINE crates/api/src/recovery.rs:1495  &minute_days,
GAP prod LINE crates/api/src/recovery.rs:1497  return Ok(found);
GAP prod LINE crates/api/src/recovery.rs:1498  }
GAP prod LINE crates/api/src/recovery.rs:1499  let schedule = if is_index(&name) {
GAP prod LINE crates/api/src/recovery.rs:1500  None
GAP prod LINE crates/api/src/recovery.rs:1502  server::recovery_cash_schedule(site, &name, asked.window).await?
GAP prod LINE crates/api/src/recovery.rs:1504  let bounded: Vec<_> = minutes
GAP prod LINE crates/api/src/recovery.rs:1505  .into_iter()
GAP prod LINE crates/api/src/recovery.rs:1506  .filter(|stamp| {
GAP prod LINE crates/api/src/recovery.rs:1507  day_of(*stamp).is_ok_and(|day| day >= asked.window.from() && day <= asked.window.to())
GAP prod LINE crates/api/src/recovery.rs:1508  })
GAP prod LINE crates/api/src/recovery.rs:1509  .collect();
GAP prod LINE crates/api/src/recovery.rs:1510  let ledger = if is_index(&name) {
GAP prod LINE crates/api/src/recovery.rs:1511  pull::gaps::classify_spot_index_against(&bounded, first, last, None)
GAP prod LINE crates/api/src/recovery.rs:1513  pull::gaps::classify_cash(
GAP prod LINE crates/api/src/recovery.rs:1514  &bounded,
GAP prod LINE crates/api/src/recovery.rs:1515  first,
GAP prod LINE crates/api/src/recovery.rs:1516  last,
GAP prod LINE crates/api/src/recovery.rs:1517  schedule.as_ref(),
GAP prod LINE crates/api/src/recovery.rs:1518  &pull::gaps::Key::series("NSE", "CASH", &name, None),
GAP prod LINE crates/api/src/recovery.rs:1521  if ledger.truncated || ledger.invalid_timestamps != 0 {
GAP prod LINE crates/api/src/recovery.rs:1522  return Err("source gap audit is truncated or has invalid timestamps".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1523  }
GAP prod LINE crates/api/src/recovery.rs:1524  let mut found = Assessment {
GAP prod LINE crates/api/src/recovery.rs:1525  held: bounded.len() as u64,
GAP prod LINE crates/api/src/recovery.rs:1526  evidence_issues: outside_session_count(&bounded, &name, first, last, schedule.as_ref())?,
GAP prod LINE crates/api/src/recovery.rs:1527  ..Assessment::default()
GAP prod LINE crates/api/src/recovery.rs:1529  let mut retry = std::collections::BTreeSet::new();
GAP prod LINE crates/api/src/recovery.rs:1530  for gap in ledger.gaps {
GAP prod LINE crates/api/src/recovery.rs:1531  let day = Day::from_days(u32::try_from(gap.day).map_err(failure)?).map_err(failure)?;
GAP prod LINE crates/api/src/recovery.rs:1532  if inactive(&name, day) {
GAP prod LINE crates/api/src/recovery.rs:1533  if daily_days.contains(&day) || minute_days.contains(&day) {
GAP prod LINE crates/api/src/recovery.rs:1534  found.unverified += u64::from(gap.minutes());
GAP prod LINE crates/api/src/recovery.rs:1535  }
GAP prod LINE crates/api/src/recovery.rs:1536  continue;
GAP prod LINE crates/api/src/recovery.rs:1537  }
GAP prod LINE crates/api/src/recovery.rs:1538  match gap.reason {
GAP prod LINE crates/api/src/recovery.rs:1540  if daily_days.contains(&day) || minute_days.contains(&day) =>
GAP prod LINE crates/api/src/recovery.rs:1541  {
GAP prod LINE crates/api/src/recovery.rs:1542  found.missing += u64::from(gap.minutes());
GAP prod LINE crates/api/src/recovery.rs:1543  retry.insert(day);
GAP prod LINE crates/api/src/recovery.rs:1544  }
GAP prod LINE crates/api/src/recovery.rs:1545  pull::gaps::Reason::VendorHole | pull::gaps::Reason::Unmeasured => {
GAP prod LINE crates/api/src/recovery.rs:1546  found.unverified += u64::from(gap.minutes());
GAP prod LINE crates/api/src/recovery.rs:1547  }
GAP prod LINE crates/api/src/recovery.rs:1548  _ => {}
GAP prod LINE crates/api/src/recovery.rs:1551  found.retry_days = retry.into_iter().collect();
GAP prod LINE crates/api/src/recovery.rs:1552  qualify_lifecycle(
GAP prod LINE crates/api/src/recovery.rs:1553  &mut found,
GAP prod LINE crates/api/src/recovery.rs:1554  lifecycle,
GAP prod LINE crates/api/src/recovery.rs:1555  &name,
GAP prod LINE crates/api/src/recovery.rs:1556  asked.window,
GAP prod LINE crates/api/src/recovery.rs:1557  &daily_days,
GAP prod LINE crates/api/src/recovery.rs:1558  &minute_days,
GAP prod LINE crates/api/src/recovery.rs:1560  Ok(found)
GAP prod LINE crates/api/src/recovery.rs:1576  pull::gaps::classify_cash(
GAP prod LINE crates/api/src/recovery.rs:1577  &[],
GAP prod LINE crates/api/src/recovery.rs:1578  first,
GAP prod LINE crates/api/src/recovery.rs:1579  last,
GAP prod LINE crates/api/src/recovery.rs:1580  schedule,
GAP prod LINE crates/api/src/recovery.rs:1581  &pull::gaps::Key::series("NSE", "CASH", name, None),
GAP prod LINE crates/api/src/recovery.rs:1585  return Err("independent session shape could not be fully checked".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1608  _ => return Err("stored minute lies outside the audited session shape".to_owned()),
GAP prod LINE crates/api/src/recovery.rs:1620  fn qualify_lifecycle(
GAP prod LINE crates/api/src/recovery.rs:1621  found: &mut Assessment,
GAP prod LINE crates/api/src/recovery.rs:1622  lifecycle: &Lifecycle,
GAP prod LINE crates/api/src/recovery.rs:1623  symbol: &str,
GAP prod LINE crates/api/src/recovery.rs:1624  window: Window,
GAP prod LINE crates/api/src/recovery.rs:1625  daily: &HashSet<Day>,
GAP prod LINE crates/api/src/recovery.rs:1626  minutes: &HashSet<Day>,
GAP prod LINE crates/api/src/recovery.rs:1627  ) {
GAP prod LINE crates/api/src/recovery.rs:1628  if is_index(symbol) {
GAP prod LINE crates/api/src/recovery.rs:1629  return;
GAP prod LINE crates/api/src/recovery.rs:1630  }
GAP prod LINE crates/api/src/recovery.rs:1633  for day in daily.union(minutes) {
GAP prod LINE crates/api/src/recovery.rs:1634  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod LINE crates/api/src/recovery.rs:1635  found.evidence_issues += 1;
GAP prod LINE crates/api/src/recovery.rs:1636  }
GAP prod LINE crates/api/src/recovery.rs:1638  let proof = brutex_core::universe::nse_isin(symbol).and_then(|isin| {
GAP prod LINE crates/api/src/recovery.rs:1639  lifecycle
GAP prod LINE crates/api/src/recovery.rs:1640  .as_ref()
GAP prod LINE crates/api/src/recovery.rs:1641  .and_then(|master| master.inspect(symbol, isin.as_str()).ok())
GAP prod LINE crates/api/src/recovery.rs:1642  });
GAP prod LINE crates/api/src/recovery.rs:1643  if let Some(proof) = proof {
GAP prod LINE crates/api/src/recovery.rs:1644  if matches!(
GAP prod LINE crates/api/src/recovery.rs:1645  proof.assessment.quality,
GAP prod LINE crates/api/src/recovery.rs:1648  ) {
GAP prod LINE crates/api/src/recovery.rs:1649  found.evidence_issues += 1;
GAP prod LINE crates/api/src/recovery.rs:1650  found.retry_days.clear();
GAP prod LINE crates/api/src/recovery.rs:1651  }
GAP prod LINE crates/api/src/recovery.rs:1652  if let Some(listed) = proof.identity.metadata.listing.date() {
GAP prod LINE crates/api/src/recovery.rs:1653  let conflicts = daily
GAP prod LINE crates/api/src/recovery.rs:1654  .iter()
GAP prod LINE crates/api/src/recovery.rs:1655  .chain(minutes.iter())
GAP prod LINE crates/api/src/recovery.rs:1656  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod LINE crates/api/src/recovery.rs:1657  if conflicts {
GAP prod LINE crates/api/src/recovery.rs:1658  found.evidence_issues += 1;
GAP prod LINE crates/api/src/recovery.rs:1659  }
GAP prod LINE crates/api/src/recovery.rs:1662  found.retry_days.retain(|day| *day >= listed);
GAP prod LINE crates/api/src/recovery.rs:1663  }
GAP prod LINE crates/api/src/recovery.rs:1664  if proof.assessment.quality == pull::cash_auction::LifecycleQuality::Unavailable
GAP prod LINE crates/api/src/recovery.rs:1665  && (found.held > 0 || found.missing > 0)
GAP prod LINE crates/api/src/recovery.rs:1666  {
GAP prod LINE crates/api/src/recovery.rs:1667  found.evidence_issues += 1;
GAP prod LINE crates/api/src/recovery.rs:1668  }
GAP prod LINE crates/api/src/recovery.rs:1669  } else if found.held > 0 || found.missing > 0 {
GAP prod LINE crates/api/src/recovery.rs:1670  found.evidence_issues += 1;
GAP prod LINE crates/api/src/recovery.rs:1671  found.retry_days.clear();
GAP prod LINE crates/api/src/recovery.rs:1672  }
GAP prod LINE crates/api/src/recovery.rs:1673  }
GAP prod LINE crates/api/src/recovery.rs:1731  return Err("recovery reached its 100,000-unit state bound; the remainder was not silently truncated".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1736  journal.append(item).map_err(failure)?;
GAP prod LINE crates/api/src/recovery.rs:1737  return Ok(());
GAP prod LINE crates/api/src/recovery.rs:1740  item.status = Status::Exhausted;
GAP prod LINE crates/api/src/recovery.rs:1741  append_attempt(journal, attempts, item)?;
GAP prod LINE crates/api/src/recovery.rs:1742  return Ok(());
GAP prod LINE crates/api/src/recovery.rs:1746  return Err("stopped before the next source request".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1763  if !reserve(&mut item) {
GAP prod LINE crates/api/src/recovery.rs:1764  break;
GAP prod LINE crates/api/src/recovery.rs:1765  }
GAP prod LINE crates/api/src/recovery.rs:1766  append_attempt(journal, attempts, item.clone())?;
GAP prod LINE crates/api/src/recovery.rs:1767  let run = server::recovery_spot(site, &asked, seat).await?;
GAP prod LINE crates/api/src/recovery.rs:1768  item.committed = item
GAP prod LINE crates/api/src/recovery.rs:1769  .committed
GAP prod LINE crates/api/src/recovery.rs:1770  .saturating_add(run.total.bars_committed as u64);
GAP prod LINE crates/api/src/recovery.rs:1771  item.diagnostics = run.total.failures.len() as u64;
GAP prod LINE crates/api/src/recovery.rs:1772  item.unchanged = if run.total.bars_committed == 0 {
GAP prod LINE crates/api/src/recovery.rs:1773  item.unchanged.saturating_add(1)
GAP prod LINE crates/api/src/recovery.rs:1775  0
GAP prod LINE crates/api/src/recovery.rs:1777  item.http_status = run
GAP prod LINE crates/api/src/recovery.rs:1778  .blocked
GAP prod LINE crates/api/src/recovery.rs:1779  .as_ref()
GAP prod LINE crates/api/src/recovery.rs:1780  .map_or(if run.reached > 0 { 200 } else { 502 }, |block| {
GAP prod LINE crates/api/src/recovery.rs:1781  block.code.as_u16()
GAP prod LINE crates/api/src/recovery.rs:1782  });
GAP prod LINE crates/api/src/recovery.rs:1783  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod LINE crates/api/src/recovery.rs:1784  item.status = Status::Blocked;
GAP prod LINE crates/api/src/recovery.rs:1785  append_attempt(journal, attempts, item)?;
GAP prod LINE crates/api/src/recovery.rs:1786  return Err("source request blocked, stopped or credentials unavailable; inspect /audit; budgets preserved".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1787  }
GAP prod LINE crates/api/src/recovery.rs:1788  let current = assess(site, &item.body, lifecycle).await?;
GAP prod LINE crates/api/src/recovery.rs:1789  item.missing = current.missing;
GAP prod LINE crates/api/src/recovery.rs:1790  item.unverified = current.unverified;
GAP prod LINE crates/api/src/recovery.rs:1791  item.diagnostics = item.diagnostics.saturating_add(current.evidence_issues);
GAP prod LINE crates/api/src/recovery.rs:1793  item.status = receipt_state(
GAP prod LINE crates/api/src/recovery.rs:1794  item.attempts,
GAP prod LINE crates/api/src/recovery.rs:1795  run.total.balances(),
GAP prod LINE crates/api/src/recovery.rs:1796  current.status(),
GAP prod LINE crates/api/src/recovery.rs:1797  !current.retry_days.is_empty(),
GAP prod LINE crates/api/src/recovery.rs:1798  );
GAP prod LINE crates/api/src/recovery.rs:1799  append_attempt(journal, attempts, item.clone())?;
GAP prod LINE crates/api/src/recovery.rs:1800  let stored_rows = crate::pullrun::rows_now(site);
GAP prod LINE crates/api/src/recovery.rs:1801  update(site, |progress| {
GAP prod LINE crates/api/src/recovery.rs:1802  progress.rows_now = stored_rows;
GAP prod LINE crates/api/src/recovery.rs:1803  progress.retries = progress
GAP prod LINE crates/api/src/recovery.rs:1804  .retries
GAP prod LINE crates/api/src/recovery.rs:1805  .saturating_add(u32::from(item.attempts > 1));
GAP prod LINE crates/api/src/recovery.rs:1806  });
GAP prod LINE crates/api/src/recovery.rs:1807  if item.status != Status::Queued {
GAP prod LINE crates/api/src/recovery.rs:1808  break;
GAP prod LINE crates/api/src/recovery.rs:1809  }
GAP prod LINE crates/api/src/recovery.rs:1810  tokio::time::sleep(crate::pullrun::RETRY_WAIT).await;
GAP prod LINE crates/api/src/recovery.rs:1812  Ok(())
GAP prod LINE crates/api/src/recovery.rs:1833  return Err("active recovery identity mismatch".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1839  failure(why)
GAP prod LINE crates/api/src/recovery.rs:1847  return Err("active recovery changed while reading events; retry this snapshot".to_owned());
GAP prod LINE crates/api/src/recovery.rs:1850  None
GAP prod LINE crates/api/src/recovery.rs:1931  Err(why) => (
GAP prod LINE crates/api/src/recovery.rs:1932  Err(format!(
GAP prod LINE crates/api/src/recovery.rs:1933  "the recovery journal was not read ({why:?}): at most {} store reads run at \
GAP prod LINE crates/api/src/recovery.rs:1934  once; ask again",
GAP prod LINE crates/api/src/recovery.rs:1935  crate::detail::MAX_STORE_READ_CONCURRENT
GAP prod LINE crates/api/src/recovery.rs:1936  )),
GAP prod LINE crates/api/src/recovery.rs:1937  None,
GAP prod LINE crates/api/src/recovery.rs:1938  ),
GAP prod LINE crates/api/src/recovery.rs:1980  let _ = write!(
GAP prod LINE crates/api/src/recovery.rs:1981  out,
GAP prod LINE crates/api/src/recovery.rs:1982  "<p>{}: {} of {} scan windows checked. {}</p>",
GAP prod LINE crates/api/src/recovery.rs:1983  escape(&feed.vendor),
GAP prod LINE crates/api/src/recovery.rs:1984  feed.legs_done,
GAP prod LINE crates/api/src/recovery.rs:1985  feed.legs,
GAP prod LINE crates/api/src/recovery.rs:1986  escape(&feed.doing)
GAP prod LINE crates/api/src/recovery.rs:1987  );
GAP prod LINE crates/api/src/recovery.rs:1988  }
GAP prod LINE crates/api/src/recovery.rs:2004  Status::InFlight => "Attempt reserved",
GAP prod LINE crates/api/src/recovery.rs:2005  Status::Verified => "Stored window checked",
GAP prod LINE crates/api/src/recovery.rs:2006  Status::NotApplicable => "Not applicable",
GAP prod LINE crates/api/src/recovery.rs:2008  Status::Exhausted => "Missing / retry limit",
GAP prod LINE crates/api/src/recovery.rs:2009  Status::Blocked => "Blocked",
GAP test LINE crates/api/src/recovery.rs:2365  tokio::task::yield_now().await;
GAP test LINE crates/api/src/recovery.rs:2665  assert!(
GAP test LINE crates/api/src/recovery.rs:2666  !why.contains("recovery requires explicit F&O spot members"),
GAP test LINE crates/api/src/recovery.rs:3414  tokio::task::yield_now().await;
GAP ignored LINE crates/api/src/recovery.rs:3537  fn authentic_saved_request_reproduces_its_accepted_scope_identity_without_rebuilding_history() {
GAP ignored LINE crates/api/src/recovery.rs:3538  let form = std::env::var("BRUTEX_RECOVERY_AUDIT_FORM").expect("explicit saved form path");
GAP ignored LINE crates/api/src/recovery.rs:3539  let receipt =
GAP ignored LINE crates/api/src/recovery.rs:3540  std::env::var("BRUTEX_RECOVERY_AUDIT_RECEIPT").expect("explicit saved receipt path");
GAP ignored LINE crates/api/src/recovery.rs:3541  let body = std::fs::read_to_string(form).unwrap();
GAP ignored LINE crates/api/src/recovery.rs:3542  let receipt: serde_json::Value =
GAP ignored LINE crates/api/src/recovery.rs:3543  serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
GAP ignored LINE crates/api/src/recovery.rs:3544  let selected = submission(body.trim_end(), today()).unwrap();
GAP ignored LINE crates/api/src/recovery.rs:3545  assert_eq!(hex(selected.id), receipt["plan"].as_str().unwrap());
GAP ignored LINE crates/api/src/recovery.rs:3546  assert_eq!(
GAP ignored LINE crates/api/src/recovery.rs:3547  selected.units.len() as u64,
GAP ignored LINE crates/api/src/recovery.rs:3548  receipt["windows"].as_u64().unwrap()
GAP ignored LINE crates/api/src/recovery.rs:3550  assert_eq!(receipt["coverage_certified"], false);
GAP ignored LINE crates/api/src/recovery.rs:3551  println!(
GAP ignored LINE crates/api/src/recovery.rs:3553  hex(selected.id),
GAP ignored LINE crates/api/src/recovery.rs:3554  selected.units.len()
GAP ignored LINE crates/api/src/recovery.rs:3556  }
GAP prod BRANCH crates/api/src/recovery.rs:145:8 True  if asked.feed != pull::vendor::Feed::Zerodha
GAP prod BRANCH crates/api/src/recovery.rs:146:12 True  || asked.target != ingest::SpotTarget::Fno
GAP prod BRANCH crates/api/src/recovery.rs:147:12 True  || asked.cash_identity != CashIdentity::ZerodhaCrossChecked
GAP prod BRANCH crates/api/src/recovery.rs:148:12 True  || !matches!(asked.granularity, Granularity::Day1 | Granularity::Minute1)
GAP prod BRANCH crates/api/src/recovery.rs:173:12 True  if leg.route != Route::Spot || leg.vendor != "zerodha" || leg.dir != asked.granularity.dir()
GAP prod BRANCH crates/api/src/recovery.rs:173:40 True  if leg.route != Route::Spot || leg.vendor != "zerodha" || leg.dir != asked.granularity.dir()
GAP prod BRANCH crates/api/src/recovery.rs:183:20 True  if bodies.len() > MAX_UNITS {
GAP prod BRANCH crates/api/src/recovery.rs:241:9 False  let [policy, predecessor, generation] = parts.as_slice() else {
GAP prod BRANCH crates/api/src/recovery.rs:244:8 True  if *policy != SUCCESSOR_POLICY {
GAP prod BRANCH crates/api/src/recovery.rs:273:12 True  if row.attempts == 0 || pointer_identity(&row.body)? != row.key || *row != expected {
GAP prod BRANCH crates/api/src/recovery.rs:273:33 True  if row.attempts == 0 || pointer_identity(&row.body)? != row.key || *row != expected {
GAP prod BRANCH crates/api/src/recovery.rs:299:8 True  if *control != expected
GAP prod BRANCH crates/api/src/recovery.rs:300:12 True  || !matches!(
GAP prod BRANCH crates/api/src/recovery.rs:313:12 True  || scans.len() > MAX_UNITS
GAP prod BRANCH crates/api/src/recovery.rs:314:12 True  || scans.iter().any(|row| row.key != key(&row.body))
GAP prod BRANCH crates/api/src/recovery.rs:324:12 True  if scope != id {
GAP prod BRANCH crates/api/src/recovery.rs:332:12 True  if predecessor != scope || generation_id != id {
GAP prod BRANCH crates/api/src/recovery.rs:332:36 True  if predecessor != scope || generation_id != id {
GAP prod BRANCH crates/api/src/recovery.rs:352:12 True  if !journal.latest.contains_key(&CONTROL) && history.latest.contains_key(&id) {
GAP prod BRANCH crates/api/src/recovery.rs:352:54 True  if !journal.latest.contains_key(&CONTROL) && history.latest.contains_key(&id) {
GAP prod BRANCH crates/api/src/recovery.rs:352:54 False  if !journal.latest.contains_key(&CONTROL) && history.latest.contains_key(&id) {
GAP prod BRANCH crates/api/src/recovery.rs:358:12 False  if journal.latest.contains_key(&CONTROL) {
GAP prod BRANCH crates/api/src/recovery.rs:440:8 True  if history.latest.contains_key(&asked.id) {
GAP prod BRANCH crates/api/src/recovery.rs:443:8 True  if plan_path(site, *predecessor)
GAP prod BRANCH crates/api/src/recovery.rs:464:8 True  if journal.latest.iter().any(|(key, row)| {
GAP prod BRANCH crates/api/src/recovery.rs:466:13 True  row.body != *seal || row.status != Status::Queued
GAP prod BRANCH crates/api/src/recovery.rs:468:13 True  !expected.contains(key) || row.status != Status::Queued || row.attempts != 0
GAP prod BRANCH crates/api/src/recovery.rs:468:40 True  !expected.contains(key) || row.status != Status::Queued || row.attempts != 0
GAP prod BRANCH crates/api/src/recovery.rs:479:16 True  if existing != row {
GAP prod BRANCH crates/api/src/recovery.rs:506:12 True  if journal
GAP prod BRANCH crates/api/src/recovery.rs:513:12 True  if asked.units.iter().any(|row| {
GAP prod BRANCH crates/api/src/recovery.rs:632:12 True  if let Err(why) = claim(&site, Some((id, true))) {
GAP prod BRANCH crates/api/src/recovery.rs:647:12 True  if let Err(why) = prepared {
GAP prod BRANCH crates/api/src/recovery.rs:668:24 False  Ok(journal) if !stopping(&site) => journal,
GAP prod BRANCH crates/api/src/recovery.rs:768:9 False  let Some(pointer) = active.latest.values().max_by_key(|record| record.attempts) else {
GAP prod BRANCH crates/api/src/recovery.rs:772:8 True  if pointer.key != id {
GAP prod BRANCH crates/api/src/recovery.rs:779:12 True  if why.kind() == std::io::ErrorKind::NotFound {
GAP prod BRANCH crates/api/src/recovery.rs:779:12 False  if why.kind() == std::io::ErrorKind::NotFound {
GAP prod BRANCH crates/api/src/recovery.rs:813:9 False  let Some(pointer) = active.latest.values().max_by_key(|record| record.attempts) else {
GAP prod BRANCH crates/api/src/recovery.rs:817:8 True  if pointer.key != id {
GAP prod BRANCH crates/api/src/recovery.rs:824:12 False  if why.kind() == std::io::ErrorKind::NotFound {
GAP prod BRANCH crates/api/src/recovery.rs:865:25 False  Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
GAP prod BRANCH crates/api/src/recovery.rs:865:76 False  Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists && directory.is_dir() => {}
GAP prod BRANCH crates/api/src/recovery.rs:882:15 True  } else if !journal.latest.contains_key(&CONTROL) {
GAP prod BRANCH crates/api/src/recovery.rs:917:25 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
GAP prod BRANCH crates/api/src/recovery.rs:978:29 False  control.status = if answer.is_ok() {
GAP prod BRANCH crates/api/src/recovery.rs:1070:12 True  if stopping(site) {
GAP prod BRANCH crates/api/src/recovery.rs:1079:20 False  if let Some(feed) = progress.feeds.first_mut() {
GAP prod BRANCH crates/api/src/recovery.rs:1097:20 True  if !found.retry_days.is_empty() {
GAP prod BRANCH crates/api/src/recovery.rs:1097:20 False  if !found.retry_days.is_empty() {
GAP prod BRANCH crates/api/src/recovery.rs:1123:20 False  if let Some(feed) = progress.feeds.first_mut() {
GAP prod BRANCH crates/api/src/recovery.rs:1161:50 True  if !shared.latest.contains_key(&item.key) && shared.latest.len() >= MAX_UNITS {
GAP prod BRANCH crates/api/src/recovery.rs:1233:19 False  ) || (explicit && row.status == Status::Blocked)
GAP prod BRANCH crates/api/src/recovery.rs:1238:12 True  if stopping(site) {
GAP prod BRANCH crates/api/src/recovery.rs:1248:13 True  let Some(windows) = scopes.get(&stored_scope_key(&item.body)?) else {
GAP prod BRANCH crates/api/src/recovery.rs:1252:12 True  if item.key != key(&item.body)
GAP prod BRANCH crates/api/src/recovery.rs:1252:12 False  if item.key != key(&item.body)
GAP prod BRANCH crates/api/src/recovery.rs:1253:16 True  || asked.window.days() != 1
GAP prod BRANCH crates/api/src/recovery.rs:1253:16 False  || asked.window.days() != 1
GAP prod BRANCH crates/api/src/recovery.rs:1254:16 True  || server::param(&item.body, "recovery_kind") != "gap"
GAP prod BRANCH crates/api/src/recovery.rs:1254:16 False  || server::param(&item.body, "recovery_kind") != "gap"
GAP prod BRANCH crates/api/src/recovery.rs:1258:12 True  if !windows
GAP prod BRANCH crates/api/src/recovery.rs:1258:12 False  if !windows
GAP prod BRANCH crates/api/src/recovery.rs:1260:27 True  .any(|window| asked.window.from() >= window.from() && asked.window.to() <= window.to())
GAP prod BRANCH crates/api/src/recovery.rs:1260:27 False  .any(|window| asked.window.from() >= window.from() && asked.window.to() <= window.to())
GAP prod BRANCH crates/api/src/recovery.rs:1296:12 False  && item.unverified == found.unverified
GAP prod BRANCH crates/api/src/recovery.rs:1330:19 True  } else if self.missing > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1343:8 True  if asked.members.len() != 1 {
GAP prod BRANCH crates/api/src/recovery.rs:1370:21 False  segment: if is_index(symbol) { "INDEX" } else { "CASH" },
GAP prod BRANCH crates/api/src/recovery.rs:1398:12 False  if is_index(symbol) { "INDEX" } else { "CASH" },
GAP prod BRANCH crates/api/src/recovery.rs:1404:8 True  if file.header().n_valid > 50_000 {
GAP prod BRANCH crates/api/src/recovery.rs:1410:16 True  if day_of(bar.ts_micros)?.year_month().map_err(failure)? != month {
GAP prod BRANCH crates/api/src/recovery.rs:1430:60 False  if stamps.windows(2).any(|pair| matches!(pair,[a,b] if a>=b)) {
GAP prod BRANCH crates/api/src/recovery.rs:1445:8 True  if held_days.len() != daily.len() {
GAP prod BRANCH crates/api/src/recovery.rs:1445:8 False  if held_days.len() != daily.len() {
GAP prod BRANCH crates/api/src/recovery.rs:1448:24 True  let need_minutes = !is_daily
GAP prod BRANCH crates/api/src/recovery.rs:1448:24 False  let need_minutes = !is_daily
GAP prod BRANCH crates/api/src/recovery.rs:1451:17 True  !held_days.contains(&day)
GAP prod BRANCH crates/api/src/recovery.rs:1451:17 False  !held_days.contains(&day)
GAP prod BRANCH crates/api/src/recovery.rs:1458:22 True  let minutes = if need_minutes {
GAP prod BRANCH crates/api/src/recovery.rs:1458:22 False  let minutes = if need_minutes {
GAP prod BRANCH crates/api/src/recovery.rs:1471:8 True  if month != asked.window.to().year_month().map_err(failure)? {
GAP prod BRANCH crates/api/src/recovery.rs:1471:8 False  if month != asked.window.to().year_month().map_err(failure)? {
GAP prod BRANCH crates/api/src/recovery.rs:1487:8 True  if asked.granularity == Granularity::Day1 {
GAP prod BRANCH crates/api/src/recovery.rs:1487:8 False  if asked.granularity == Granularity::Day1 {
GAP prod BRANCH crates/api/src/recovery.rs:1499:23 True  let schedule = if is_index(&name) {
GAP prod BRANCH crates/api/src/recovery.rs:1499:23 False  let schedule = if is_index(&name) {
GAP prod BRANCH crates/api/src/recovery.rs:1507:44 True  day_of(*stamp).is_ok_and(|day| day >= asked.window.from() && day <= asked.window.to())
GAP prod BRANCH crates/api/src/recovery.rs:1507:44 False  day_of(*stamp).is_ok_and(|day| day >= asked.window.from() && day <= asked.window.to())
GAP prod BRANCH crates/api/src/recovery.rs:1510:21 True  let ledger = if is_index(&name) {
GAP prod BRANCH crates/api/src/recovery.rs:1510:21 False  let ledger = if is_index(&name) {
GAP prod BRANCH crates/api/src/recovery.rs:1521:8 True  if ledger.truncated || ledger.invalid_timestamps != 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1521:8 False  if ledger.truncated || ledger.invalid_timestamps != 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1521:28 True  if ledger.truncated || ledger.invalid_timestamps != 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1521:28 False  if ledger.truncated || ledger.invalid_timestamps != 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1532:12 True  if inactive(&name, day) {
GAP prod BRANCH crates/api/src/recovery.rs:1532:12 False  if inactive(&name, day) {
GAP prod BRANCH crates/api/src/recovery.rs:1533:16 True  if daily_days.contains(&day) || minute_days.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1533:16 False  if daily_days.contains(&day) || minute_days.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1533:45 True  if daily_days.contains(&day) || minute_days.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1533:45 False  if daily_days.contains(&day) || minute_days.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1540:20 True  if daily_days.contains(&day) || minute_days.contains(&day) =>
GAP prod BRANCH crates/api/src/recovery.rs:1540:20 False  if daily_days.contains(&day) || minute_days.contains(&day) =>
GAP prod BRANCH crates/api/src/recovery.rs:1540:49 True  if daily_days.contains(&day) || minute_days.contains(&day) =>
GAP prod BRANCH crates/api/src/recovery.rs:1540:49 False  if daily_days.contains(&day) || minute_days.contains(&day) =>
GAP prod BRANCH crates/api/src/recovery.rs:1573:20 False  let shape = if is_index(name) {
GAP prod BRANCH crates/api/src/recovery.rs:1584:8 True  if shape.truncated || shape.invalid_timestamps > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1584:27 True  if shape.truncated || shape.invalid_timestamps > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1595:32 True  .is_some_and(|gap| gap.day < day || (gap.day == day && u32::from(gap.to) < minute))
GAP prod BRANCH crates/api/src/recovery.rs:1595:50 False  .is_some_and(|gap| gap.day < day || (gap.day == day && u32::from(gap.to) < minute))
GAP prod BRANCH crates/api/src/recovery.rs:1600:26 False  Some(gap) if gap.day == day && u32::from(gap.from) <= minute => {
GAP prod BRANCH crates/api/src/recovery.rs:1600:44 False  Some(gap) if gap.day == day && u32::from(gap.from) <= minute => {
GAP prod BRANCH crates/api/src/recovery.rs:1628:8 True  if is_index(symbol) {
GAP prod BRANCH crates/api/src/recovery.rs:1628:8 False  if is_index(symbol) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:12 True  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:12 False  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:37 True  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:37 False  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:60 True  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1634:60 False  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1643:12 True  if let Some(proof) = proof {
GAP prod BRANCH crates/api/src/recovery.rs:1643:12 False  if let Some(proof) = proof {
GAP prod BRANCH crates/api/src/recovery.rs:1652:16 True  if let Some(listed) = proof.identity.metadata.listing.date() {
GAP prod BRANCH crates/api/src/recovery.rs:1652:16 False  if let Some(listed) = proof.identity.metadata.listing.date() {
GAP prod BRANCH crates/api/src/recovery.rs:1656:28 True  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1656:28 False  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1656:53 True  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1656:53 False  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1657:16 True  if conflicts {
GAP prod BRANCH crates/api/src/recovery.rs:1657:16 False  if conflicts {
GAP prod BRANCH crates/api/src/recovery.rs:1664:12 True  if proof.assessment.quality == pull::cash_auction::LifecycleQuality::Unavailable
GAP prod BRANCH crates/api/src/recovery.rs:1664:12 False  if proof.assessment.quality == pull::cash_auction::LifecycleQuality::Unavailable
GAP prod BRANCH crates/api/src/recovery.rs:1665:17 True  && (found.held > 0 || found.missing > 0)
GAP prod BRANCH crates/api/src/recovery.rs:1665:17 False  && (found.held > 0 || found.missing > 0)
GAP prod BRANCH crates/api/src/recovery.rs:1665:35 True  && (found.held > 0 || found.missing > 0)
GAP prod BRANCH crates/api/src/recovery.rs:1665:35 False  && (found.held > 0 || found.missing > 0)
GAP prod BRANCH crates/api/src/recovery.rs:1669:15 True  } else if found.held > 0 || found.missing > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1669:15 False  } else if found.held > 0 || found.missing > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1669:33 True  } else if found.held > 0 || found.missing > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1669:33 False  } else if found.held > 0 || found.missing > 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1685:40 True  if daily.contains(&day) || minutes.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1730:8 False  if !journal.latest.contains_key(&new.key) && journal.latest.len() >= MAX_UNITS {
GAP prod BRANCH crates/api/src/recovery.rs:1730:50 True  if !journal.latest.contains_key(&new.key) && journal.latest.len() >= MAX_UNITS {
GAP prod BRANCH crates/api/src/recovery.rs:1739:8 True  if item.attempts >= ATTEMPT_LIMIT {
GAP prod BRANCH crates/api/src/recovery.rs:1744:11 False  while item.attempts < ATTEMPT_LIMIT {
GAP prod BRANCH crates/api/src/recovery.rs:1745:12 True  if stopping(site) {
GAP prod BRANCH crates/api/src/recovery.rs:1756:13 True  let Some(seat) = site.autopilot.take_seat(asked.feed) else {
GAP prod BRANCH crates/api/src/recovery.rs:1763:12 True  if !reserve(&mut item) {
GAP prod BRANCH crates/api/src/recovery.rs:1763:12 False  if !reserve(&mut item) {
GAP prod BRANCH crates/api/src/recovery.rs:1772:29 True  item.unchanged = if run.total.bars_committed == 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1772:29 False  item.unchanged = if run.total.bars_committed == 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1780:24 True  .map_or(if run.reached > 0 { 200 } else { 502 }, |block| {
GAP prod BRANCH crates/api/src/recovery.rs:1780:24 False  .map_or(if run.reached > 0 { 200 } else { 502 }, |block| {
GAP prod BRANCH crates/api/src/recovery.rs:1783:12 True  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1783:12 False  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1783:35 True  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1783:35 False  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1783:60 True  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1783:60 False  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
GAP prod BRANCH crates/api/src/recovery.rs:1807:12 True  if item.status != Status::Queued {
GAP prod BRANCH crates/api/src/recovery.rs:1807:12 False  if item.status != Status::Queued {
GAP prod BRANCH crates/api/src/recovery.rs:1832:8 True  if id != pointer.key {
GAP prod BRANCH crates/api/src/recovery.rs:1836:12 False  if why.kind() == std::io::ErrorKind::NotFound {
GAP prod BRANCH crates/api/src/recovery.rs:1842:8 True  if crate::recovery_journal::tail(&active_path(site), 1)
GAP prod BRANCH crates/api/src/recovery.rs:1849:26 True  let predecessor = if pointer.body.len() == 64 {
GAP test BRANCH crates/api/src/recovery.rs:2364:19 True  while site.run.lock().unwrap().as_ref().unwrap().running() {
GAP test BRANCH crates/api/src/recovery.rs:2491:17 False  let server::RecoveryStart::Grounded(line) = server::recovery_at_start(&site, gate)
GAP test BRANCH crates/api/src/recovery.rs:2500:34 False  [inside a closure whose false side does not fail the check]  .any(|event| crate::emitted::says(event, "plan", &hex(id))
GAP test BRANCH crates/api/src/recovery.rs:2557:13 False  [an operand followed by ||: neither side fails the check by itself]  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2557:47 True  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2557:47 False  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2598:30 False  [inside a closure whose false side does not fail the check]  .any(|event| crate::emitted::says(event, "symbol", "NIFTY")
GAP test BRANCH crates/api/src/recovery.rs:2621:30 False  [inside a closure whose false side does not fail the check]  .any(|event| crate::emitted::says(event, "plan", &hex(id))
GAP test BRANCH crates/api/src/recovery.rs:2664:16 True  if let Err(why) = &ran {
GAP test BRANCH crates/api/src/recovery.rs:3413:19 True  while site.run.lock().unwrap().as_ref().unwrap().running() {
GAP prod LINE crates/api/src/recovery_control.rs:67  Ok(_) => return Err("recovery STOP path is not a regular file".to_owned()),
GAP prod LINE crates/api/src/recovery_control.rs:69  Err(error) => return Err(error.to_string()),
GAP prod LINE crates/api/src/recovery_control.rs:88  Ok(_) => return Err("recovery STOP path is not a regular file".to_owned()),
GAP prod LINE crates/api/src/recovery_control.rs:90  Err(error) => return Err(error.to_string()),
GAP prod LINE crates/api/src/recovery_control.rs:132  return Err(
GAP prod LINE crates/api/src/recovery_control.rs:133  "configured source store is not a directory; no replacement created".to_owned(),
GAP prod LINE crates/api/src/recovery_control.rs:134  );
GAP prod LINE crates/api/src/recovery_control.rs:167  return Err("recovery STOP journal contains foreign plan keys".to_owned());
GAP prod LINE crates/api/src/recovery_control.rs:214  Err(error) => Err(error.to_string()),
GAP prod BRANCH crates/api/src/recovery_control.rs:66:25 False  Ok(metadata) if metadata.is_file() => {}
GAP prod BRANCH crates/api/src/recovery_control.rs:68:23 False  Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
GAP prod BRANCH crates/api/src/recovery_control.rs:87:25 False  Ok(metadata) if metadata.is_file() => {}
GAP prod BRANCH crates/api/src/recovery_control.rs:89:23 False  Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
GAP prod BRANCH crates/api/src/recovery_control.rs:131:8 True  if !metadata.is_dir() {
GAP prod BRANCH crates/api/src/recovery_control.rs:166:8 True  if latest.len() != 1 {
GAP prod BRANCH crates/api/src/recovery_control.rs:172:67 True  if !matches!(row.status, Status::Queued | Status::Blocked) || *row != record(id, row.status) {
GAP prod BRANCH crates/api/src/recovery_control.rs:204:23 False  Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
GAP prod LINE crates/api/src/recovery_journal.rs:192  )?;
GAP prod LINE crates/api/src/recovery_journal.rs:324  TryLockError::Error(error) => error,
GAP prod LINE crates/api/src/recovery_journal.rs:329  return Err(invalid_data("recovery journal is not a regular file"));
GAP prod LINE crates/api/src/recovery_journal.rs:339  return Err(invalid_data(
GAP prod LINE crates/api/src/recovery_journal.rs:340  "recovery journal length changed during locked replay",
GAP prod LINE crates/api/src/recovery_journal.rs:341  ));
GAP prod LINE crates/api/src/recovery_journal.rs:386  io::Error::other("recovery journal length cannot fit its next record")
GAP prod LINE crates/api/src/recovery_journal.rs:387  })?;
GAP prod LINE crates/api/src/recovery_journal.rs:454  io::Error::other("recovery journal length cannot fit its next records")
GAP prod LINE crates/api/src/recovery_journal.rs:455  })?;
GAP prod LINE crates/api/src/recovery_journal.rs:555  return Err(invalid_data("recovery journal is not a regular file"));
GAP prod LINE crates/api/src/recovery_journal.rs:670  TryLockError::Error(error) => error,
GAP prod LINE crates/api/src/recovery_journal.rs:674  return Err(invalid_data("recovery journal is not a regular file"));
GAP test LINE crates/api/src/recovery_journal.rs:1538  fn flush(&mut self) -> io::Result<()> {
GAP test LINE crates/api/src/recovery_journal.rs:1539  Ok(())
GAP test LINE crates/api/src/recovery_journal.rs:1540  }
GAP prod BRANCH crates/api/src/recovery_journal.rs:328:12 True  if !metadata.is_file() {
GAP prod BRANCH crates/api/src/recovery_journal.rs:338:12 True  if file.metadata()?.len() != bytes {
GAP prod BRANCH crates/api/src/recovery_journal.rs:521:16 True  if !metadata.is_file() || metadata.dev() != *device || metadata.ino() != *inode {
GAP prod BRANCH crates/api/src/recovery_journal.rs:521:39 True  if !metadata.is_file() || metadata.dev() != *device || metadata.ino() != *inode {
GAP prod BRANCH crates/api/src/recovery_journal.rs:554:8 True  if !metadata.is_file() {
GAP prod BRANCH crates/api/src/recovery_journal.rs:673:8 True  if !metadata.is_file() {
GAP test LINE crates/api/src/render.rs:3230  called = true;
GAP test LINE crates/api/src/render.rs:3231  vec!["unexpected".to_owned()]
GAP test LINE crates/api/src/render.rs:3232  });
GAP prod LINE crates/api/src/render.rs:3284  continue;
GAP test LINE crates/api/src/render.rs:3371  }
GAP test LINE crates/api/src/render.rs:5137  return Some(lower[i..j].to_owned());
GAP prod BRANCH crates/api/src/render.rs:3283:13 False  let Ok(kind) = e.file_type() else {
GAP test BRANCH crates/api/src/render.rs:3365:16 False  if archive_suggestions() == Ok(true) {
GAP test BRANCH crates/api/src/render.rs:4219:13 True  [an operand followed by ||: neither side fails the check by itself]  html.contains("class=\"r0\"") || html.contains("class=\"r1\""),
GAP test BRANCH crates/api/src/render.rs:4459:13 True  [an operand followed by ||: neither side fails the check by itself]  html.contains("2 record(s) held") || html.contains("showing 2"),
GAP test BRANCH crates/api/src/render.rs:5133:19 False  while j < b.len() && b[j].is_ascii_alphabetic() {
GAP test BRANCH crates/api/src/render.rs:5136:29 True  if j > i + 2 && b.get(j) == Some(&b'=') {
GAP prod LINE crates/api/src/repair_route.rs:121  return refused(&format!("{feed_text:?} is not a feed this build stores"));
GAP prod LINE crates/api/src/repair_route.rs:125  return refused(&format!("{instrument:?} is not a store symbol"));
GAP prod LINE crates/api/src/repair_route.rs:130  other => {
GAP prod LINE crates/api/src/repair_route.rs:131  return refused(&format!(
GAP prod LINE crates/api/src/repair_route.rs:132  "segment {other:?}: a repair plan reads INDEX or CASH"
GAP prod LINE crates/api/src/repair_route.rs:133  ));
GAP prod LINE crates/api/src/repair_route.rs:144  timeframe = Some(*known);
GAP prod LINE crates/api/src/repair_route.rs:162  Err(why) => return refused(&why.to_string()),
GAP prod LINE crates/api/src/repair_route.rs:167  return refused("the window's first month is outside the store's calendar");
GAP prod LINE crates/api/src/repair_route.rs:340  "before the series' first held bar: a listing, the start of the feed's history, or a hole; a repair of the month lands whatever the feed ser
GAP prod LINE crates/api/src/repair_route.rs:362  Timeframe::MINUTE_1
GAP prod LINE crates/api/src/repair_route.rs:380  format!(
GAP prod LINE crates/api/src/repair_route.rs:383  })
GAP prod LINE crates/api/src/repair_route.rs:426  return serde_json::json!({ "error": format!("cannot read {}", path.display()) });
GAP prod LINE crates/api/src/repair_route.rs:434  return serde_json::json!({ "error": format!("{} is not text", path.display()) });
GAP prod BRANCH crates/api/src/repair_route.rs:120:9 False  let Some(vendor) = ingest::parse_vendor(&feed_text) else {
GAP prod BRANCH crates/api/src/repair_route.rs:124:8 True  if brutex_core::symbol::Symbol::new(&instrument).is_err() {
GAP prod BRANCH crates/api/src/repair_route.rs:143:12 True  if known.as_str() == rung_text {
GAP prod BRANCH crates/api/src/repair_route.rs:166:9 False  let Ok(first_month) = window.from().year_month() else {
GAP prod BRANCH crates/api/src/repair_route.rs:255:20 False  if let Ok(first) = Day::new(month.month.year(), month.month.month(), 1) {
GAP prod BRANCH crates/api/src/repair_route.rs:361:21 True  let pulled = if derived {
GAP prod BRANCH crates/api/src/repair_route.rs:411:5 True  rung.secs() > Timeframe::MINUTE_1.secs() && rung.secs() < Timeframe::DAY_1.secs()
GAP prod BRANCH crates/api/src/repair_route.rs:425:8 True  if file.seek(SeekFrom::Start(from)).is_err() {
GAP prod BRANCH crates/api/src/repair_route.rs:429:8 True  if file
GAP test LINE crates/api/src/research_doors_tests.rs:35  other => panic!("no door named {other}"),
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:125  return Err(format!(
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:126  "{name} queued an invalid query instead of refusing immediately"
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:127  ));
GAP test BRANCH crates/api/src/saved_response_boundary_tests.rs:124:9 False  let Poll::Ready(reply) = reply else {
GAP prod LINE crates/api/src/scrub_route.rs:428  continue;
GAP prod LINE crates/api/src/scrub_route.rs:518  }
GAP prod LINE crates/api/src/scrub_route.rs:551  continue;
GAP prod LINE crates/api/src/scrub_route.rs:568  }
GAP prod LINE crates/api/src/scrub_route.rs:576  }
GAP prod LINE crates/api/src/scrub_route.rs:652  continue;
GAP prod LINE crates/api/src/scrub_route.rs:836  break;
GAP test LINE crates/api/src/scrub_route.rs:915  Some((_, count)) => *count += 1,
GAP prod LINE crates/api/src/scrub_route.rs:1052  return Ok(read(&Index::default()));
GAP prod LINE crates/api/src/scrub_route.rs:1067  return Ok(read(&Index::default()));
GAP prod LINE crates/api/src/scrub_route.rs:1321  Ok(Ok(body)) => (axum::http::StatusCode::OK, headers, body),
GAP prod BRANCH crates/api/src/scrub_route.rs:409:24 False  if let (Ok(run), Some(pass)) = (
GAP prod BRANCH crates/api/src/scrub_route.rs:427:17 False  let Some((series, file)) = second.rsplit_once('/') else {
GAP prod BRANCH crates/api/src/scrub_route.rs:513:16 False  if let Some(count) = self.busy_in.get_mut(&pass) {
GAP prod BRANCH crates/api/src/scrub_route.rs:546:17 False  let Some(mut finding) = self
GAP prod BRANCH crates/api/src/scrub_route.rs:563:20 False  if let Some(held) = self.by_pass.get_mut(&older) {
GAP prod BRANCH crates/api/src/scrub_route.rs:569:20 False  if let Some(counts) = self.series.get_mut(&*key.0)
GAP prod BRANCH crates/api/src/scrub_route.rs:570:24 False  && let Some(count) = counts.get_mut(&older)
GAP prod BRANCH crates/api/src/scrub_route.rs:579:20 False  && let Some(counts) = self.series.get_mut(&*key.0)
GAP prod BRANCH crates/api/src/scrub_route.rs:647:21 False  let Some(mut finding) = self
GAP prod BRANCH crates/api/src/scrub_route.rs:835:13 False  let (Some(slot), Some(range)) = (next.get_mut(at), ranges.get_mut(at)) else {
GAP prod BRANCH crates/api/src/scrub_route.rs:838:16 False  if let Some(taken) = slot.take() {
GAP prod BRANCH crates/api/src/scrub_route.rs:985:51 False  None if full && !self.in_long_line && from < fresh.len() => {
GAP prod BRANCH crates/api/src/scrub_route.rs:1051:9 False  let Some(cell) = KEPT.get(vendor as usize) else {
GAP prod BRANCH crates/api/src/scrub_route.rs:1066:21 True  Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
GAP prod BRANCH crates/api/src/scrub_route.rs:1076:44 False  Some(kept) if kept.path == path && kept.file == identity && kept.consumed <= stamp.0 => {
GAP prod BRANCH crates/api/src/scrub_route.rs:1076:69 False  Some(kept) if kept.path == path && kept.file == identity && kept.consumed <= stamp.0 => {
GAP prod LINE crates/api/src/selectionv6json.rs:174  refused(
GAP prod LINE crates/api/src/selectionv6json.rs:176  "Selection V6 response exceeds its byte ceiling; no prefix returned",
GAP prod LINE crates/api/src/selectionv6json.rs:218  total,
GAP prod LINE crates/api/src/selectionv6json.rs:219  from,
GAP prod LINE crates/api/src/selectionv6json.rs:220  records,
GAP prod LINE crates/api/src/selectionv6json.rs:222  json!({"rung":rung,"status":"saved",
GAP prod LINE crates/api/src/selectionv6json.rs:223  "path":null,"refusal":null,
GAP prod LINE crates/api/src/selectionv6json.rs:224  "total_blocks":total.to_string(),"from":from.to_string(),
GAP prod LINE crates/api/src/selectionv6json.rs:225  "records":records.iter().map(|record| project(record, family)).collect::<Vec<_>>()})
GAP prod BRANCH crates/api/src/selectionv6json.rs:171:16 False  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/selectionv6json.rs:233:22 True  "status": if saved { "saved" } else { "absent" },
GAP test LINE crates/api/src/serve_edge_tests.rs:206  fn flush(&mut self) -> std::io::Result<()> {
GAP test LINE crates/api/src/serve_edge_tests.rs:207  Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
GAP test LINE crates/api/src/serve_edge_tests.rs:208  }
GAP test LINE crates/api/src/serve_edge_tests.rs:332  !line
GAP test LINE crates/api/src/serve_edge_tests.rs:333  .get(..at)
GAP test LINE crates/api/src/serve_edge_tests.rs:334  .and_then(|head| head.chars().last())
GAP test LINE crates/api/src/serve_edge_tests.rs:335  .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
GAP test LINE crates/api/src/serve_edge_tests.rs:336  })
GAP test BRANCH crates/api/src/serve_edge_tests.rs:335:46 True  .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
GAP test BRANCH crates/api/src/serve_edge_tests.rs:335:46 False  .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
GAP prod LINE crates/api/src/server.rs:1923  return (
GAP prod LINE crates/api/src/server.rs:1924  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:1925  headers,
GAP prod LINE crates/api/src/server.rs:1926  no_such_feed_json(&asked),
GAP prod LINE crates/api/src/server.rs:1927  );
GAP prod LINE crates/api/src/server.rs:1991  body.push('\n');
GAP prod LINE crates/api/src/server.rs:5020  telemetry::now_millis().unsigned_abs()
GAP prod LINE crates/api/src/server.rs:5825  Err(pull::session::SessionError::SessionLengthUnmeasured { .. }) => false,
GAP prod LINE crates/api/src/server.rs:5826  Err(why) => return Err(why.to_string()),
GAP prod LINE crates/api/src/server.rs:6174  None => None,
GAP prod LINE crates/api/src/server.rs:6177  pull::ingest::landing_run::Mode::Repair
GAP prod LINE crates/api/src/server.rs:6614  || spot_audit_origin(asked.cash_identity, &run.origin),
GAP prod LINE crates/api/src/server.rs:6633  return Err(why);
GAP prod LINE crates/api/src/server.rs:8997  async fn read_credential(
GAP prod LINE crates/api/src/server.rs:8998  identity: &pull::ssm::AwsIdentity,
GAP prod LINE crates/api/src/server.rs:8999  config: &pull::config::CredentialConfig,
GAP prod LINE crates/api/src/server.rs:9000  vendor: brutex_core::vendor::Vendor,
GAP prod LINE crates/api/src/server.rs:9001  auth: pull::vendor::Auth,
GAP prod LINE crates/api/src/server.rs:9002  ) -> Result<pull::http::Credential, crate::credential_law::Unreadable> {
GAP prod LINE crates/api/src/server.rs:9008  let stamp = pull::ssm::now_stamp().map_err(|why| Unreadable::configuration(why.detail))?;
GAP prod LINE crates/api/src/server.rs:9009  let read = async |field: &str| -> Result<String, Unreadable> {
GAP prod LINE crates/api/src/server.rs:9010  let path = config.path_for(vendor, field).map_err(|why| {
GAP prod LINE crates/api/src/server.rs:9011  Unreadable::configuration(format!(
GAP prod LINE crates/api/src/server.rs:9014  })?;
GAP prod LINE crates/api/src/server.rs:9015  pull::ssm::get_parameter(identity, config.region(), &path.to_string(), &stamp)
GAP prod LINE crates/api/src/server.rs:9016  .await
GAP prod LINE crates/api/src/server.rs:9017  .map_err(|why| {
GAP prod LINE crates/api/src/server.rs:9018  Unreadable::of_secret(
GAP prod LINE crates/api/src/server.rs:9019  why.kind,
GAP prod LINE crates/api/src/server.rs:9020  format!(
GAP prod LINE crates/api/src/server.rs:9025  })
GAP prod LINE crates/api/src/server.rs:9026  };
GAP prod LINE crates/api/src/server.rs:9028  let token = read("access-token").await?;
GAP prod LINE crates/api/src/server.rs:9029  match auth.key_field {
GAP prod LINE crates/api/src/server.rs:9033  None => Ok(pull::http::Credential::token(token)),
GAP prod LINE crates/api/src/server.rs:9037  Some(field) => Ok(pull::http::Credential::pair(read(field).await?, token)),
GAP prod LINE crates/api/src/server.rs:9039  }
GAP prod LINE crates/api/src/server.rs:9093  let identity = pull::ssm::AwsIdentity::discover()
GAP prod LINE crates/api/src/server.rs:9094  .map_err(|why| Unreadable::configuration(format!("no AWS identity: {why}")))?;
GAP prod LINE crates/api/src/server.rs:9116  let vendor = feed.store_vendor().ok_or_else(|| {
GAP prod LINE crates/api/src/server.rs:9117  Unreadable::configuration(format!(
GAP prod LINE crates/api/src/server.rs:9122  feed.display(),
GAP prod LINE crates/api/src/server.rs:9123  feed.display()
GAP prod LINE crates/api/src/server.rs:9125  })?;
GAP prod LINE crates/api/src/server.rs:9126  let credential = read_credential(&identity, &config, vendor, spec.auth).await?;
GAP prod LINE crates/api/src/server.rs:9134  let source = pull::http::HttpSource::new(*spec, credential)
GAP prod LINE crates/api/src/server.rs:9135  .map_err(|why| Unreadable::configuration(why.to_string()))?;
GAP prod LINE crates/api/src/server.rs:9137  Ok((source, vendor))
GAP prod LINE crates/api/src/server.rs:9464  return Err(format!(
GAP prod LINE crates/api/src/server.rs:9465  "{} has no store prefix of its own, so there is nowhere to file its \
GAP prod LINE crates/api/src/server.rs:9466  bars. Nothing is written rather than another vendor's prefix being \
GAP prod LINE crates/api/src/server.rs:9467  borrowed.",
GAP prod LINE crates/api/src/server.rs:9468  vendor.as_str()
GAP prod LINE crates/api/src/server.rs:9469  ));
GAP prod LINE crates/api/src/server.rs:9884  format!(
GAP prod LINE crates/api/src/server.rs:9888  feed.display()
GAP prod LINE crates/api/src/server.rs:9890  })?;
GAP prod LINE crates/api/src/server.rs:9961  return Err(format!(
GAP prod LINE crates/api/src/server.rs:9962  "{} declares no FNO column layout, and this path files under FNO. \
GAP prod LINE crates/api/src/server.rs:9963  Refused rather than decoded against another vendor's shape — a \
GAP prod LINE crates/api/src/server.rs:9964  file read with the wrong columns is wrong in every field, and the \
GAP prod LINE crates/api/src/server.rs:9965  store is append-only.",
GAP prod LINE crates/api/src/server.rs:9966  feed.display()
GAP prod LINE crates/api/src/server.rs:9967  ));
GAP prod LINE crates/api/src/server.rs:10097  pull::ingest::landing_run::kind_of(brutex_core::vendor::Vendor::Zerodha)
GAP prod LINE crates/api/src/server.rs:10114  return (
GAP prod LINE crates/api/src/server.rs:10115  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/server.rs:10116  json_headers(),
GAP prod LINE crates/api/src/server.rs:10117  run_refusal_json(&crate::pullrun::Refusal::AlreadyRunning.why()),
GAP prod LINE crates/api/src/server.rs:10118  );
GAP prod LINE crates/api/src/server.rs:10122  Err(fault) => {
GAP prod LINE crates/api/src/server.rs:10123  let why =
GAP prod LINE crates/api/src/server.rs:10124  format!("{fault}; the run took no id, so it did not start and no vendor was asked");
GAP prod LINE crates/api/src/server.rs:10125  progress.finished = Some(why.clone());
GAP prod LINE crates/api/src/server.rs:10126  return (
GAP prod LINE crates/api/src/server.rs:10127  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:10128  json_headers(),
GAP prod LINE crates/api/src/server.rs:10129  run_refusal_json(&why),
GAP prod LINE crates/api/src/server.rs:10130  );
GAP prod LINE crates/api/src/server.rs:10594  Err(why) => (
GAP prod LINE crates/api/src/server.rs:10595  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/server.rs:10596  refusal_html("Spot pull", &why),
GAP prod LINE crates/api/src/server.rs:10597  ),
GAP prod LINE crates/api/src/server.rs:11550  note_price_refusal(&mut out, "the chunk's first day names no month file");
GAP prod LINE crates/api/src/server.rs:11551  return out;
GAP prod LINE crates/api/src/server.rs:11929  return page.say(
GAP prod LINE crates/api/src/server.rs:11930  facts,
GAP prod LINE crates/api/src/server.rs:11932  audit::Outcome::NotStarted,
GAP prod LINE crates/api/src/server.rs:11933  "this feed declares no expired-derivative history at all, so there \
GAP prod LINE crates/api/src/server.rs:11934  is nothing to ask it for",
GAP prod LINE crates/api/src/server.rs:11993  Ok(chain) => fno_report(&page, facts, &chain, asked, site, &wire).await,
GAP prod LINE crates/api/src/server.rs:12070  return months;
GAP prod LINE crates/api/src/server.rs:12949  Err(why) => return Some(format!("the Greek-file month could not be named: {why}")),
GAP prod LINE crates/api/src/server.rs:13020  out.refused = out.refused.saturating_add(1);
GAP prod LINE crates/api/src/server.rs:13021  note_price_refusal(
GAP prod LINE crates/api/src/server.rs:13022  &mut out,
GAP prod LINE crates/api/src/server.rs:13023  "the bar's stamp is not a moment any calendar places",
GAP prod LINE crates/api/src/server.rs:13025  continue;
GAP prod LINE crates/api/src/server.rs:14360  }
GAP prod LINE crates/api/src/server.rs:14361  let last_settled = match ingest::last_settled_day(page.today) {
GAP prod LINE crates/api/src/server.rs:14362  Ok(day) => day,
GAP prod LINE crates/api/src/server.rs:14363  Err(why) => {
GAP prod LINE crates/api/src/server.rs:14364  return page.say(
GAP prod LINE crates/api/src/server.rs:14365  facts,
GAP prod LINE crates/api/src/server.rs:14367  audit::Outcome::NotStarted,
GAP prod LINE crates/api/src/server.rs:14368  &why.to_string(),
GAP prod LINE crates/api/src/server.rs:14375  let planned = off_the_workers(|| {
GAP prod LINE crates/api/src/server.rs:14376  let book = masters_dir().and_then(|dir| {
GAP prod LINE crates/api/src/server.rs:14377  pull::master_snapshots::Book::open(&dir.join(pull::master_snapshots::SNAPSHOT_DIR))
GAP prod LINE crates/api/src/server.rs:14378  })?;
GAP prod LINE crates/api/src/server.rs:14379  pull::fnowork::futures_plan(
GAP prod LINE crates/api/src/server.rs:14380  &book,
GAP prod LINE crates/api/src/server.rs:14381  route,
GAP prod LINE crates/api/src/server.rs:14382  asked.underlying.as_str(),
GAP prod LINE crates/api/src/server.rs:14383  asked.window,
GAP prod LINE crates/api/src/server.rs:14384  page.today,
GAP prod LINE crates/api/src/server.rs:14385  last_settled,
GAP prod LINE crates/api/src/server.rs:14387  });
GAP prod LINE crates/api/src/server.rs:14388  let plan = match planned {
GAP prod LINE crates/api/src/server.rs:14389  Ok(plan) => plan,
GAP prod LINE crates/api/src/server.rs:14390  Err(why) => return page.say(facts, unavailable, audit::Outcome::NotStarted, &why),
GAP prod LINE crates/api/src/server.rs:14392  let plan = match narrowed_to_expiry(plan, asked) {
GAP prod LINE crates/api/src/server.rs:14393  Ok(plan) => plan,
GAP prod LINE crates/api/src/server.rs:14394  Err(why) => return page.say(facts, unavailable, audit::Outcome::NotStarted, &why),
GAP prod LINE crates/api/src/server.rs:14396  facts.push(("Dated master snapshot", plan.newest.to_string()));
GAP prod LINE crates/api/src/server.rs:14397  facts.push(("Series asked", plan.asks.len().to_string()));
GAP prod LINE crates/api/src/server.rs:14398  if let Some(ref why) = plan.rebuilt {
GAP prod LINE crates/api/src/server.rs:14399  facts.push(("Futures index rebuilt", why.clone()));
GAP prod LINE crates/api/src/server.rs:14400  }
GAP prod LINE crates/api/src/server.rs:14406  if !plan.incomplete.is_empty() {
GAP prod LINE crates/api/src/server.rs:14407  facts.push((
GAP prod LINE crates/api/src/server.rs:14408  "Incomplete master-snapshot entries",
GAP prod LINE crates/api/src/server.rs:14409  plan.incomplete.join(" · "),
GAP prod LINE crates/api/src/server.rs:14410  ));
GAP prod LINE crates/api/src/server.rs:14411  }
GAP prod LINE crates/api/src/server.rs:14412  if !plan.refused.is_empty() {
GAP prod LINE crates/api/src/server.rs:14413  facts.push(("Not asked", plan.refused.join(" · ")));
GAP prod LINE crates/api/src/server.rs:14414  }
GAP prod LINE crates/api/src/server.rs:14415  let governed = Governed {
GAP prod LINE crates/api/src/server.rs:14416  inner: &wire.source,
GAP prod LINE crates/api/src/server.rs:14417  feed: asked.feed,
GAP prod LINE crates/api/src/server.rs:14418  site,
GAP prod LINE crates/api/src/server.rs:14419  };
GAP prod LINE crates/api/src/server.rs:14420  let origin = wire.source.endpoint(route.granularity);
GAP prod LINE crates/api/src/server.rs:14421  let landed = pull::fnowork::pull_futures_day(
GAP prod LINE crates/api/src/server.rs:14422  &governed,
GAP prod LINE crates/api/src/server.rs:14423  &wire.spec,
GAP prod LINE crates/api/src/server.rs:14424  &plan,
GAP prod LINE crates/api/src/server.rs:14425  &site.store_root,
GAP prod LINE crates/api/src/server.rs:14426  wire.store_vendor,
GAP prod LINE crates/api/src/server.rs:14427  &origin,
GAP prod LINE crates/api/src/server.rs:14428  )
GAP prod LINE crates/api/src/server.rs:14429  .await;
GAP prod LINE crates/api/src/server.rs:14430  futures_answer(page, facts, &plan, &landed)
GAP prod LINE crates/api/src/server.rs:14661  Err(refused) => return refused,
GAP prod LINE crates/api/src/server.rs:14899  Err(why) => (
GAP prod LINE crates/api/src/server.rs:14900  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/server.rs:14901  refusal_html("Expired F&O pull", &why),
GAP prod LINE crates/api/src/server.rs:14902  ),
GAP prod LINE crates/api/src/server.rs:15926  Err(why) => return unlocatable(site, &symbol, &Unlocated::Refused(why)),
GAP test LINE crates/api/src/server.rs:18926  Ok(())
GAP test LINE crates/api/src/server.rs:18927  }),
GAP test LINE crates/api/src/server.rs:18952  Err(_elapsed) => return (String::from_utf8_lossy(&got).into_owned(), false),
GAP test LINE crates/api/src/server.rs:19078  Ok(())
GAP test LINE crates/api/src/server.rs:19079  }),
GAP test LINE crates/api/src/server.rs:19129  break;
GAP test LINE crates/api/src/server.rs:19397  break;
GAP prod LINE crates/api/src/server.rs:19789  format!(
GAP prod LINE crates/api/src/server.rs:19791  store_root.display()
GAP prod LINE crates/api/src/server.rs:19793  })?;
GAP prod LINE crates/api/src/server.rs:19935  || file.set_len(0),
GAP prod LINE crates/api/src/server.rs:19937  )?;
GAP prod LINE crates/api/src/server.rs:19939  note_unstamped_lock(&warning, |line| warn_line!("{line}"));
GAP prod LINE crates/api/src/server.rs:20530  }
GAP prod LINE crates/api/src/server.rs:20534  let (program, args) = browser_handler(CURRENT_BROWSER_HOST);
GAP prod LINE crates/api/src/server.rs:20535  std::process::Command::new(program)
GAP prod LINE crates/api/src/server.rs:20536  .args(args)
GAP prod LINE crates/api/src/server.rs:20537  .arg(url)
GAP prod LINE crates/api/src/server.rs:20538  // DETACHED. The child is the operator's browser and outlives this
GAP prod LINE crates/api/src/server.rs:20539  // process; inheriting stdout would interleave its noise with the
GAP prod LINE crates/api/src/server.rs:20540  // banner, and waiting on it would block the server behind a window.
GAP prod LINE crates/api/src/server.rs:20541  .stdout(std::process::Stdio::null())
GAP prod LINE crates/api/src/server.rs:20542  .stderr(std::process::Stdio::null())
GAP prod LINE crates/api/src/server.rs:20543  .spawn()
GAP prod LINE crates/api/src/server.rs:20544  .map_err(|why| format!("{program}: {why}"))
GAP prod LINE crates/api/src/server.rs:20545  .and_then(|child| {
GAP prod LINE crates/api/src/server.rs:20546  reap_detached(child)
GAP prod LINE crates/api/src/server.rs:20547  .map(drop)
GAP prod LINE crates/api/src/server.rs:20548  .map_err(|why| format!("{program}: the child cannot be reaped: {why}"))
GAP prod LINE crates/api/src/server.rs:20549  })
GAP prod LINE crates/api/src/server.rs:20830  Err(why) => {
GAP prod LINE crates/api/src/server.rs:20831  warn_line!(
GAP prod LINE crates/api/src/server.rs:20834  return FAILED;
GAP prod LINE crates/api/src/server.rs:20875  Err(why) => {
GAP prod LINE crates/api/src/server.rs:20876  warn_line!(
GAP prod LINE crates/api/src/server.rs:20880  return FAILED;
GAP prod LINE crates/api/src/server.rs:20954  Loading::Failed(why) => return load_failed(&why),
GAP prod LINE crates/api/src/server.rs:20999  say!("  NO WRITES: {why}");
GAP prod LINE crates/api/src/server.rs:21002  say!(
GAP prod LINE crates/api/src/server.rs:21003  "  autopilot: starting in {}s — http://{addr}/autopilot.json",
GAP prod LINE crates/api/src/server.rs:21004  autopilot::GRACE_SECS
GAP prod LINE crates/api/src/server.rs:21005  );
GAP prod LINE crates/api/src/server.rs:21071  Ok(()) => say!("  opening: {home}"),
GAP prod LINE crates/api/src/server.rs:21086  warn_line!("Calendars NOT warmed at start: {why}");
GAP prod LINE crates/api/src/server.rs:21092  warn_line!("Census caches NOT warmed at start: {why}");
GAP test LINE crates/api/src/server.rs:21395  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP ignored LINE crates/api/src/server.rs:22119  fn pull_runtime_costs() {
GAP ignored LINE crates/api/src/server.rs:22120  costs_of_the_pull_runtime("pull-runtime-costs-store", 200, 100_000, 8_000);
GAP ignored LINE crates/api/src/server.rs:22121  }
GAP ignored LINE crates/api/src/server.rs:22252  fn the_pull_order_on_a_real_census() {
GAP ignored LINE crates/api/src/server.rs:22253  let masters = std::env::var("BRUTEX_WP62_MASTERS")
GAP ignored LINE crates/api/src/server.rs:22254  .expect("BRUTEX_WP62_MASTERS names the masters directory");
GAP ignored LINE crates/api/src/server.rs:22255  let store =
GAP ignored LINE crates/api/src/server.rs:22256  std::env::var("BRUTEX_WP62_STORE").expect("BRUTEX_WP62_STORE names a shadow store");
GAP ignored LINE crates/api/src/server.rs:22257  let site = Site::load(Path::new(&masters), Path::new(&store));
GAP ignored LINE crates/api/src/server.rs:22258  println!(
GAP ignored LINE crates/api/src/server.rs:22260  site.restart.word(),
GAP ignored LINE crates/api/src/server.rs:22261  site.restart.words()
GAP ignored LINE crates/api/src/server.rs:22263  let censuses = census::read_all(&site.store_root);
GAP ignored LINE crates/api/src/server.rs:22264  for target in ["indices", "fno"] {
GAP ignored LINE crates/api/src/server.rs:22265  let asked = ingest::parse_spot(
GAP ignored LINE crates/api/src/server.rs:22266  &format!(
GAP ignored LINE crates/api/src/server.rs:22267  "vendor=zerodha&target={target}&cash_identity=zerodha_nse_checked\
GAP ignored LINE crates/api/src/server.rs:22268  &granularity=1min&from=2018-01-01&to=2026-09-30"
GAP ignored LINE crates/api/src/server.rs:22269  ),
GAP ignored LINE crates/api/src/server.rs:22270  day(2026, 10, 12),
GAP ignored LINE crates/api/src/server.rs:22272  .expect("the operator's own window");
GAP ignored LINE crates/api/src/server.rs:22273  let mut targets = spot_targets(&asked, &site);
GAP ignored LINE crates/api/src/server.rs:22274  targets.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22275  let asked_for = targets.len();
GAP ignored LINE crates/api/src/server.rs:22276  assert!(asked_for > 0, "{target}: the masters name no instrument");
GAP ignored LINE crates/api/src/server.rs:22277  let whole = ladder_refusal(&asked, &targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22278  let began = std::time::Instant::now();
GAP ignored LINE crates/api/src/server.rs:22279  let gate = order_gate(&asked, &mut targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22280  let took = began.elapsed();
GAP ignored LINE crates/api/src/server.rs:22281  let whole = whole.unwrap_or_else(|| {
GAP ignored LINE crates/api/src/server.rs:22282  panic!("{target}: the whole-run gate is open, so this store shows nothing")
GAP ignored LINE crates/api/src/server.rs:22284  println!(
GAP ignored LINE crates/api/src/server.rs:22286  whole.chars().take(160).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22288  let withheld = gate
GAP ignored LINE crates/api/src/server.rs:22289  .unwrap_or_else(|why| panic!("{target}: still refused whole in {took:?}: {why}"));
GAP ignored LINE crates/api/src/server.rs:22290  println!(
GAP ignored LINE crates/api/src/server.rs:22292  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22293  targets.len()
GAP ignored LINE crates/api/src/server.rs:22295  assert!(
GAP ignored LINE crates/api/src/server.rs:22296  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored LINE crates/api/src/server.rs:22298  withheld.len()
GAP ignored LINE crates/api/src/server.rs:22300  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22301  targets.len() + withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22305  let mut named: Vec<_> = withheld.iter().map(|(key, _)| *key).collect();
GAP ignored LINE crates/api/src/server.rs:22306  named.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22307  named.dedup();
GAP ignored LINE crates/api/src/server.rs:22308  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22309  named.len(),
GAP ignored LINE crates/api/src/server.rs:22310  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22313  for (key, why) in withheld {
GAP ignored LINE crates/api/src/server.rs:22314  assert!(
GAP ignored LINE crates/api/src/server.rs:22315  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored LINE crates/api/src/server.rs:22319  println!(
GAP ignored LINE crates/api/src/server.rs:22322  why.chars().take(150).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22326  }
GAP test LINE crates/api/src/server.rs:28258  [read: the second operand of an || in the condition, never reached: not the failing arm]  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test LINE crates/api/src/server.rs:28259  [read: part of that never-reached operand]  .unwrap_or_default()
GAP test LINE crates/api/src/server.rs:28260  [read: part of that never-reached operand]  .contains(") Z "),
GAP test LINE crates/api/src/server.rs:29487  let lock_path = root.join("serve.lock");
GAP test LINE crates/api/src/server.rs:29488  let busy = std::fs::OpenOptions::new()
GAP test LINE crates/api/src/server.rs:29489  .read(true)
GAP test LINE crates/api/src/server.rs:29490  .write(true)
GAP test LINE crates/api/src/server.rs:29491  .open(&lock_path)
GAP test LINE crates/api/src/server.rs:29492  .is_ok_and(|file| store::flock::Flock::try_lock(file, lock_path.clone()).is_err());
GAP test LINE crates/api/src/server.rs:29493  if busy {
GAP test LINE crates/api/src/server.rs:29499  assert_eq!(
GAP test LINE crates/api/src/server.rs:29500  run(&argv(&["serve", "127.0.0.1:0"]), fired()).await,
GAP test LINE crates/api/src/server.rs:29504  let refused =
GAP test LINE crates/api/src/server.rs:29505  crate::emitted::landed(from, "api.serve", "refused: this store cannot be served");
GAP test LINE crates/api/src/server.rs:29506  assert!(
GAP test LINE crates/api/src/server.rs:29507  refused.iter().any(|record| {
GAP test LINE crates/api/src/server.rs:29508  record.level == telemetry::Level::Error
GAP test LINE crates/api/src/server.rs:29509  && crate::emitted::says(record, "why", "already serving this store")
GAP test LINE crates/api/src/server.rs:29510  }),
GAP test LINE crates/api/src/server.rs:29513  return;
GAP test LINE crates/api/src/server.rs:29514  }
GAP test LINE crates/api/src/server.rs:29516  let expected =
GAP test LINE crates/api/src/server.rs:29517  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test LINE crates/api/src/server.rs:29523  assert_eq!(
GAP test LINE crates/api/src/server.rs:29524  run(&argv(&["serve", "127.0.0.1:0"]), after_the_load(&root)).await,
GAP test LINE crates/api/src/server.rs:29529  let listening = crate::emitted::landed(from, "api.serve", "listening");
GAP test LINE crates/api/src/server.rs:29530  let mine = listening
GAP test LINE crates/api/src/server.rs:29531  .iter()
GAP test LINE crates/api/src/server.rs:29532  .find(|record| crate::emitted::says(record, "addr", "127.0.0.1:"))
GAP test LINE crates/api/src/server.rs:29533  .expect("the process's own configuration is the first thing it writes");
GAP test LINE crates/api/src/server.rs:29534  assert_eq!(mine.level, telemetry::Level::Info);
GAP test LINE crates/api/src/server.rs:29538  assert!(
GAP test LINE crates/api/src/server.rs:29565  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:29920  [read: the second operand of an || in the condition, never reached: not the failing arm]  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test LINE crates/api/src/server.rs:30101  matches!(
GAP test LINE crates/api/src/server.rs:30688  feed.display()
GAP test LINE crates/api/src/server.rs:31713  (axum::http::StatusCode::OK, String::new())
GAP test LINE crates/api/src/server.rs:31714  })
GAP test LINE crates/api/src/server.rs:32057  assert!(
GAP test LINE crates/api/src/server.rs:32058  spec.window_caps.is_empty(),
GAP test LINE crates/api/src/server.rs:32063  feed.display()
GAP test LINE crates/api/src/server.rs:32065  continue;
GAP test LINE crates/api/src/server.rs:32456  || "null".to_owned(),
GAP test LINE crates/api/src/server.rs:32656  other => panic!("attempt {attempt} of a 429 gave {other:?}"),
GAP test LINE crates/api/src/server.rs:34423  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:34537  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:34757  key("NIFTYCHEMICALS", m)
GAP test LINE crates/api/src/server.rs:34758  })
GAP test LINE crates/api/src/server.rs:34777  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP prod LINE crates/api/src/server.rs:36715  let feed = vendor.as_str().to_owned();
GAP prod LINE crates/api/src/server.rs:36731  let owned = {
GAP prod LINE crates/api/src/server.rs:36732  let held = site.universe();
GAP prod LINE crates/api/src/server.rs:36733  resolved_master_rows(&held.read.merged, vendor)
GAP prod LINE crates/api/src/server.rs:36735  let master: Vec<pull::universe::VendorInstrument<'_>> = owned
GAP prod LINE crates/api/src/server.rs:36736  .iter()
GAP prod LINE crates/api/src/server.rs:36737  .map(|(id, symbol, isin)| pull::universe::VendorInstrument {
GAP prod LINE crates/api/src/server.rs:36738  vendor_id: id,
GAP prod LINE crates/api/src/server.rs:36739  trading_symbol: symbol,
GAP prod LINE crates/api/src/server.rs:36740  isin,
GAP prod LINE crates/api/src/server.rs:36741  })
GAP prod LINE crates/api/src/server.rs:36742  .collect();
GAP prod LINE crates/api/src/server.rs:36744  let source = match pull::resolve::HttpDocuments::new() {
GAP prod LINE crates/api/src/server.rs:36745  Ok(client) => client,
GAP prod LINE crates/api/src/server.rs:36746  Err(why) => {
GAP prod LINE crates/api/src/server.rs:36747  return (
GAP prod LINE crates/api/src/server.rs:36748  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:36749  json,
GAP prod LINE crates/api/src/server.rs:36750  format!(r#"{{"ok":false,"why":{}}}"#, render::json_string(&why)),
GAP prod LINE crates/api/src/server.rs:36751  );
GAP prod LINE crates/api/src/server.rs:36757  let today = match crate::ingest::today_ist() {
GAP prod LINE crates/api/src/server.rs:36758  Ok(day) => day,
GAP prod LINE crates/api/src/server.rs:36759  Err(why) => {
GAP prod LINE crates/api/src/server.rs:36760  return (
GAP prod LINE crates/api/src/server.rs:36761  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:36762  json,
GAP prod LINE crates/api/src/server.rs:36763  format!(
GAP prod LINE crates/api/src/server.rs:36764  r#"{{"ok":false,"why":{}}}"#,
GAP prod LINE crates/api/src/server.rs:36765  render::json_string(&why.to_string())
GAP prod LINE crates/api/src/server.rs:36766  ),
GAP prod LINE crates/api/src/server.rs:36767  );
GAP prod LINE crates/api/src/server.rs:36771  let snap = pull::resolve::crawl(
GAP prod LINE crates/api/src/server.rs:36772  &source,
GAP prod LINE crates/api/src/server.rs:36773  INDEX_HOST,
GAP prod LINE crates/api/src/server.rs:36774  today,
GAP prod LINE crates/api/src/server.rs:36775  &feed,
GAP prod LINE crates/api/src/server.rs:36776  // ASKED OF THE VENDOR, NOT ASSUMED -- and this line used to assume.
GAP prod LINE crates/api/src/server.rs:36777  //
GAP prod LINE crates/api/src/server.rs:36778  // It was the literal `JoinKey::Isin`, under a comment reading "EVERY
GAP prod LINE crates/api/src/server.rs:36779  // FEED IN THIS BUILD PUBLISHES AN ISIN. The symbol key exists for the
GAP prod LINE crates/api/src/server.rs:36780  // one that does not, and asking the descriptor rather than assuming is
GAP prod LINE crates/api/src/server.rs:36781  // what keeps that true when it lands." **It landed on 14 Aug 2026.**
GAP prod LINE crates/api/src/server.rs:36782  // Zerodha's master has twelve columns and none is an ISIN, so this
GAP prod LINE crates/api/src/server.rs:36783  // literal joined its rows on an EMPTY field -- and `universe::resolve`
GAP prod LINE crates/api/src/server.rs:36784  // skips an empty key deliberately, so nothing would have errored. Every
GAP prod LINE crates/api/src/server.rs:36785  // Zerodha instrument would have landed in `Verdict::Lacks`, which reads
GAP prod LINE crates/api/src/server.rs:36786  // exactly like a vendor that lists nothing.
GAP prod LINE crates/api/src/server.rs:36787  //
GAP prod LINE crates/api/src/server.rs:36788  // `JoinKey::for_vendor` derives it from `master_columns().isin`, which
GAP prod LINE crates/api/src/server.rs:36789  // already spells Zerodha's as empty, so there is no second table to
GAP prod LINE crates/api/src/server.rs:36790  // fall behind the first.
GAP prod LINE crates/api/src/server.rs:36791  pull::universe::JoinKey::for_vendor(vendor),
GAP prod LINE crates/api/src/server.rs:36792  &master,
GAP prod LINE crates/api/src/server.rs:36793  )
GAP prod LINE crates/api/src/server.rs:36794  .await;
GAP prod LINE crates/api/src/server.rs:36796  let verdict = snap.admits_publication(None);
GAP prod LINE crates/api/src/server.rs:36797  (
GAP prod LINE crates/api/src/server.rs:36798  axum::http::StatusCode::OK,
GAP prod LINE crates/api/src/server.rs:36799  json,
GAP prod LINE crates/api/src/server.rs:36800  universe_snapshot_json(&snap, &verdict),
GAP prod LINE crates/api/src/server.rs:36801  )
GAP prod LINE crates/api/src/server.rs:37499  break;
GAP prod LINE crates/api/src/server.rs:37506  break;
GAP prod BRANCH crates/api/src/server.rs:1922:9 False  let Some(census) = now.census() else {
GAP prod BRANCH crates/api/src/server.rs:1990:8 True  if !body.ends_with('\n') {
GAP prod BRANCH crates/api/src/server.rs:2758:31 False  if counted && let Some(n) = targets.get_mut(slot) {
GAP prod BRANCH crates/api/src/server.rs:4521:16 False  && let Ok(one) = pull::session::Window::new(day, day)
GAP prod BRANCH crates/api/src/server.rs:5019:21 True  let id = if run == 0 {
GAP prod BRANCH crates/api/src/server.rs:6075:12 True  if vendor != Vendor::Zerodha
GAP prod BRANCH crates/api/src/server.rs:6176:27 True  let mode = if asked.repair.is_some() {
GAP prod BRANCH crates/api/src/server.rs:6632:12 True  if let Some(why) = run.ledger.refused.clone() {
GAP prod BRANCH crates/api/src/server.rs:6855:68 False  if asked.window.to() > today || (asked.window.to() == today && !closed) {
GAP prod BRANCH crates/api/src/server.rs:9463:9 False  let Some(store_vendor) = feed.store_vendor() else {
GAP prod BRANCH crates/api/src/server.rs:9960:9 False  let Some(layout) = archive.layout(Segment::Fno) else {
GAP prod BRANCH crates/api/src/server.rs:10093:19 True  let kind = if legs
GAP prod BRANCH crates/api/src/server.rs:10113:9 False  let Some(progress) = held.as_mut().filter(|progress| progress.generation == run) else {
GAP prod BRANCH crates/api/src/server.rs:11549:9 False  let Ok(month) = month_of.chunk.from().year_month() else {
GAP prod BRANCH crates/api/src/server.rs:11928:8 True  if !spec.fno.serves() {
GAP prod BRANCH crates/api/src/server.rs:12069:13 False  let Some(&last) = months.last() else {
GAP prod BRANCH crates/api/src/server.rs:13019:13 False  let Ok(at) = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(1_000_000)) else {
GAP prod BRANCH crates/api/src/server.rs:14332:32 False  if plan.asks.is_empty() && plan.refused.is_empty() {
GAP prod BRANCH crates/api/src/server.rs:14358:12 False  if let Some(why) = futures_refusal(asked, route) {
GAP prod BRANCH crates/api/src/server.rs:14398:12 True  if let Some(ref why) = plan.rebuilt {
GAP prod BRANCH crates/api/src/server.rs:14398:12 False  if let Some(ref why) = plan.rebuilt {
GAP prod BRANCH crates/api/src/server.rs:14406:8 True  if !plan.incomplete.is_empty() {
GAP prod BRANCH crates/api/src/server.rs:14406:8 False  if !plan.incomplete.is_empty() {
GAP prod BRANCH crates/api/src/server.rs:14412:8 True  if !plan.refused.is_empty() {
GAP prod BRANCH crates/api/src/server.rs:14412:8 False  if !plan.refused.is_empty() {
GAP test BRANCH crates/api/src/server.rs:19128:16 True  if began.elapsed() > T * 3 {
GAP test BRANCH crates/api/src/server.rs:19133:17 False  [an operand followed by ||: neither side fails the check by itself]  assert!(cut || closed, "a dripping client outlived the deadline");
GAP test BRANCH crates/api/src/server.rs:19133:24 True  assert!(cut || closed, "a dripping client outlived the deadline");
GAP test BRANCH crates/api/src/server.rs:19133:24 False  assert!(cut || closed, "a dripping client outlived the deadline");
GAP test BRANCH crates/api/src/server.rs:19278:13 False  [an operand followed by ||: neither side fails the check by itself]  said.starts_with("HTTP/1.1 431") || said.starts_with("HTTP/1.1 408") || said.is_empty(),
GAP test BRANCH crates/api/src/server.rs:19278:49 True  said.starts_with("HTTP/1.1 431") || said.starts_with("HTTP/1.1 408") || said.is_empty(),
GAP test BRANCH crates/api/src/server.rs:19278:49 False  said.starts_with("HTTP/1.1 431") || said.starts_with("HTTP/1.1 408") || said.is_empty(),
GAP test BRANCH crates/api/src/server.rs:19278:85 True  said.starts_with("HTTP/1.1 431") || said.starts_with("HTTP/1.1 408") || said.is_empty(),
GAP test BRANCH crates/api/src/server.rs:19278:85 False  said.starts_with("HTTP/1.1 431") || said.starts_with("HTTP/1.1 408") || said.is_empty(),
GAP test BRANCH crates/api/src/server.rs:19396:16 True  if began.elapsed() > T * 3 {
GAP test BRANCH crates/api/src/server.rs:19401:17 False  [an operand followed by ||: neither side fails the check by itself]  assert!(cut || closed, "a dripping body outlived the deadline");
GAP test BRANCH crates/api/src/server.rs:19401:24 True  assert!(cut || closed, "a dripping body outlived the deadline");
GAP test BRANCH crates/api/src/server.rs:19401:24 False  assert!(cut || closed, "a dripping body outlived the deadline");
GAP prod BRANCH crates/api/src/server.rs:19938:12 True  if let Some(warning) = stamped {
GAP prod BRANCH crates/api/src/server.rs:20528:8 False  if suppressed.is_some() {
GAP prod BRANCH crates/api/src/server.rs:20998:24 True  if let Some(why) = gate.unstamped {
GAP prod BRANCH crates/api/src/server.rs:21001:20 True  if flying_on_start {
GAP prod BRANCH crates/api/src/server.rs:21085:24 True  if let Err(why) = calendar_route::warm_at_start(&site) {
GAP prod BRANCH crates/api/src/server.rs:21091:24 True  if let Err(why) = census_cache::start(&site, &calendar_route::offered_feeds()) {
GAP test BRANCH crates/api/src/server.rs:21730:30 False  [read: the body of an .any() closure opened on the line above: not the failing arm]  |record| crate::emitted::says(record, "instrument", "BANKNIFTY")
GAP ignored BRANCH crates/api/src/server.rs:22296:17 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22296:17 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22296:41 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22296:41 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22315:21 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22315:21 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22315:59 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22315:59 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP test BRANCH crates/api/src/server.rs:22706:13 False  let pull::vendor::Transport::Http(spec) =
GAP test BRANCH crates/api/src/server.rs:24491:28 False  [read: an operand inside an .any() closure opened above: not the failing arm]  && f.why.contains("(it ends on 2026-12-31): 2027-01-08")
GAP test BRANCH crates/api/src/server.rs:24497:34 False  [inside a closure whose false side does not fail the check]  .any(|f| !f.why.starts_with("request minute coverage")
GAP test BRANCH crates/api/src/server.rs:25280:17 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:26532:28 False  && record.message == "listening"
GAP test BRANCH crates/api/src/server.rs:27199:17 False  [an operand followed by ||: neither side fails the check by itself]  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:27199:61 True  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:27199:61 False  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:28257:13 False  [read: the first operand of an || continued on the next line: not the failing arm]  !std::path::Path::new(&format!("/proc/{pid}")).exists()
GAP test BRANCH crates/api/src/server.rs:28258:20 True  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test BRANCH crates/api/src/server.rs:28258:20 False  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test BRANCH crates/api/src/server.rs:29467:13 True  let Some(root) = root else {
GAP test BRANCH crates/api/src/server.rs:29477:21 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29493:12 True  if busy {
GAP test BRANCH crates/api/src/server.rs:29493:12 False  if busy {
GAP test BRANCH crates/api/src/server.rs:29508:21 True  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29508:21 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29517:51 True  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test BRANCH crates/api/src/server.rs:29517:51 False  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test BRANCH crates/api/src/server.rs:29539:13 True  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29539:13 False  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29539:50 True  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29539:50 False  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29919:13 False  [read: the first operand of an || continued on the next line: not the failing arm]  whole_body.contains(r#""reason":"closed""#)
GAP test BRANCH crates/api/src/server.rs:29920:20 True  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test BRANCH crates/api/src/server.rs:29920:20 False  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test BRANCH crates/api/src/server.rs:31088:13 False  let pull::vendor::Transport::Http(dhan) = pull::vendor::Feed::Dhan.descriptor().transport
GAP test BRANCH crates/api/src/server.rs:31152:13 False  let pull::vendor::Transport::Http(spec) =
GAP test BRANCH crates/api/src/server.rs:32035:17 False  let Some(cap) = spec.window_cap_days(pull::vendor::Granularity::Minute1) else {
GAP test BRANCH crates/api/src/server.rs:34349:24 False  if let Some(bytes) = answer {
GAP test BRANCH crates/api/src/server.rs:34825:17 False  record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/server.rs:34826:24 False  && crate::emitted::says(record, "instrument", "NIFTYWAVES")
GAP test BRANCH crates/api/src/server.rs:34933:13 False  let pull::vendor::Transport::Http(shipped) =
GAP test BRANCH crates/api/src/server.rs:35113:17 False  record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/server.rs:35114:24 False  && crate::emitted::says(record, "instrument", "NSE:NIFTY")
GAP test BRANCH crates/api/src/server.rs:35130:17 False  record.level == telemetry::Level::Info
GAP test BRANCH crates/api/src/server.rs:35131:24 False  && crate::emitted::counts(record, "days", 25)
GAP test BRANCH crates/api/src/server.rs:35132:24 False  && crate::emitted::counts(record, "instruments", 208)
GAP test BRANCH crates/api/src/server.rs:35870:17 False  crate::emitted::counts(record, "instruments", 0)
GAP prod BRANCH crates/api/src/server.rs:36698:9 True  let Some(vendor) = named else {
GAP prod BRANCH crates/api/src/server.rs:37498:13 False  let Ok(position) = u16::try_from(index) else {
GAP prod BRANCH crates/api/src/server.rs:37505:13 False  let Some(def) = vocab::table::definition(position) else {
GAP test LINE crates/api/src/server_cloud_mutant_tests_a.rs:484  }
GAP test LINE crates/api/src/server_cloud_mutant_tests_a.rs:1415  assert!(has_line(&stdout, "CLOUDA-ARMED=true"), "{stdout}");
GAP test LINE crates/api/src/server_cloud_mutant_tests_a.rs:1416  assert!(
GAP test LINE crates/api/src/server_cloud_mutant_tests_a.rs:1417  has_line(&stdout, &format!("  NO WRITES: {why}")),
GAP test LINE crates/api/src/server_cloud_mutant_tests_a.rs:1420  assert!(has_line(&stdout, grounded), "{stdout}");
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:480:8 False  if std::env::var_os(LOG_DIR_ENV).is_none() {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:882:13 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:890:13 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:1083:23 False  if !(read_only && names_autopilot) {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:1414:12 True  if let Some(why) = cli::unstamped_write_refusal(cli::commit_stamp(), Some(zero)) {
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:30  || panic!("no {label:?} fact in {facts:?}"),
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:191  .expect("the loopback serves");
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:192  });
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:358  Err(pull::chain::Refusal::transport(
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:359  "script exhausted".to_owned(),
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:360  ))
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:361  })
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:1979  fn visible(body: &str) -> &str {
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:1980  body.find("<section class=\"wrap\">")
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:1981  .and_then(|at| body.get(at..))
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:1982  .unwrap_or(body)
GAP test LINE crates/api/src/server_cloud_mutant_tests_b.rs:1983  }
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2186:13 False  crate::emitted::says(record, "endpoint", "cloudb-endpoint")
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2187:20 False  && crate::emitted::says(record, "underlying", "NIFTY")
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2188:20 False  && crate::emitted::counts(record, "chunks", 7)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2189:20 False  && crate::emitted::counts(record, "offsets", 21)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2190:20 False  && crate::emitted::counts(record, "cadences", 2)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2198:13 False  crate::emitted::says(record, "expiry_flag", "MONTH")
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2199:20 False  && crate::emitted::counts(record, "stored_so_far", 918_274)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2200:20 False  && crate::emitted::counts(record, "failed_so_far", 3)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2208:13 False  crate::emitted::counts(record, "stored", 918_275)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2209:20 False  && crate::emitted::counts(record, "failed", 5)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2210:20 False  && crate::emitted::counts(record, "declined", 6)
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:153  });
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:493  pull::vendor::Transport::LocalArchive(_) => panic!("Dhan is HTTP"),
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1717  contract: None,
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1718  exchange: brutex_core::instrument::Exchange::Nse,
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1719  segment: brutex_core::instrument::Segment::Index,
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1720  symbol: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1722  month,
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1723  });
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:1893  }
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:2852  (200, dhan_bar(AUG_03))
GAP test LINE crates/api/src/server_cloud_mutant_tests_c.rs:2897  (200, dhan_bar(AUG_03))
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:861:16 False  if let Some(site) = pressed.get() {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:904:16 False  if let Some(site) = pressed.get() {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:1885:16 False  if let Some(site) = held.get() {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2445:16 False  && !site.autopilot.json().contains(r#""ticks":0,"#)
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2849:12 False  if nth == 0 {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2889:16 False  if nth == 0 {
GAP test LINE crates/api/src/server_cloud_mutant_tests_d.rs:160  pull::vendor::Transport::LocalArchive(_) => None,
GAP test LINE crates/api/src/server_cloud_mutant_tests_e.rs:317  return Ok(());
GAP test LINE crates/api/src/server_cloud_mutant_tests_e.rs:328  Ok(()) => Ok(()),
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:255:9 False  let HeadState::Awaiting {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:314:8 True  if to != today {
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:361:9 False  [an operand followed by ||: neither side fails the check by itself]  got == before || got == after,
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:361:26 True  got == before || got == after,
GAP test BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:361:26 False  got == before || got == after,
GAP prod LINE crates/api/src/settings_route.rs:154  pub async fn snapshot_off_worker(root: PathBuf) -> Snapshot {
GAP prod LINE crates/api/src/settings_route.rs:155  match tokio::task::spawn_blocking(move || snapshot(&root)).await {
GAP prod LINE crates/api/src/settings_route.rs:156  Ok(snapshot) => snapshot,
GAP prod LINE crates/api/src/settings_route.rs:157  Err(why) => Snapshot {
GAP prod LINE crates/api/src/settings_route.rs:158  highest: 0,
GAP prod LINE crates/api/src/settings_route.rs:159  newest: None,
GAP prod LINE crates/api/src/settings_route.rs:160  faults: Vec::new(),
GAP prod LINE crates/api/src/settings_route.rs:161  stopped: Some(format!("the settings read did not finish: {why}")),
GAP prod LINE crates/api/src/settings_route.rs:162  holes: None,
GAP prod LINE crates/api/src/settings_route.rs:163  },
GAP prod LINE crates/api/src/settings_route.rs:165  }
GAP prod LINE crates/api/src/settings_route.rs:213  None => (None, "no window is recorded in this build".to_owned()),
GAP prod LINE crates/api/src/settings_route.rs:331  None => Effective {
GAP prod LINE crates/api/src/settings_route.rs:332  value: None,
GAP prod LINE crates/api/src/settings_route.rs:333  source: format!(
GAP prod LINE crates/api/src/settings_route.rs:334  "environment {name} holds {value:?}, which is refused ({refusal}): \
GAP prod LINE crates/api/src/settings_route.rs:335  nothing is in force for {}",
GAP prod LINE crates/api/src/settings_route.rs:336  key.name()
GAP prod LINE crates/api/src/settings_route.rs:337  ),
GAP prod LINE crates/api/src/settings_route.rs:338  origin: "environment",
GAP prod LINE crates/api/src/settings_route.rs:339  },
GAP prod LINE crates/api/src/settings_route.rs:437  || micros.to_string(),
GAP prod LINE crates/api/src/settings_route.rs:722  }
GAP prod LINE crates/api/src/settings_route.rs:744  Key::WindowTo
GAP prod LINE crates/api/src/settings_route.rs:773  {
GAP prod LINE crates/api/src/settings_route.rs:774  fields.push(field);
GAP prod LINE crates/api/src/settings_route.rs:967  serde_json::json!({
GAP prod LINE crates/api/src/settings_route.rs:968  "path": path.display().to_string(),
GAP prod LINE crates/api/src/settings_route.rs:969  "bytes": bytes,
GAP prod LINE crates/api/src/settings_route.rs:971  }),
GAP prod LINE crates/api/src/settings_route.rs:1022  (StatusCode::INSUFFICIENT_STORAGE, "disk_full", None)
GAP prod LINE crates/api/src/settings_route.rs:1053  Err(why) => (
GAP prod LINE crates/api/src/settings_route.rs:1054  StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/settings_route.rs:1055  json_headers(),
GAP prod LINE crates/api/src/settings_route.rs:1056  serde_json::json!({"saved": false, "refusal": format!("the save did not finish: {why}")})
GAP prod LINE crates/api/src/settings_route.rs:1057  .to_string(),
GAP prod LINE crates/api/src/settings_route.rs:1058  ),
GAP prod BRANCH crates/api/src/settings_route.rs:127:8 False  if keep {
GAP prod BRANCH crates/api/src/settings_route.rs:592:28 False  if let Some(object) = row.as_object_mut() {
GAP prod BRANCH crates/api/src/settings_route.rs:717:12 False  if !offered {
GAP prod BRANCH crates/api/src/settings_route.rs:726:12 False  if let (Some(first), Some(last)) = (&from.value, &to.value)
GAP prod BRANCH crates/api/src/settings_route.rs:733:20 False  if let Some(slot) = unset.get_mut(key.slot()) {
GAP prod BRANCH crates/api/src/settings_route.rs:741:19 True  } else if moved(Key::WindowFrom) || set(Key::WindowFrom) {
GAP prod BRANCH crates/api/src/settings_route.rs:741:45 False  } else if moved(Key::WindowFrom) || set(Key::WindowFrom) {
GAP prod BRANCH crates/api/src/settings_route.rs:772:20 True  && !fields.contains(&field)
GAP prod BRANCH crates/api/src/settings_route.rs:1021:49 True  Refusal::Io { kind, .. } if *kind == std::io::ErrorKind::StorageFull => {
GAP test LINE crates/api/src/settings_route_tests.rs:398  Ok(value) if !value.trim().is_empty() => {
GAP test LINE crates/api/src/settings_route_tests.rs:399  assert_eq!(masters["origin"], "environment");
GAP test LINE crates/api/src/settings_route_tests.rs:400  assert_eq!(masters["value"], value.as_str());
GAP test LINE crates/api/src/settings_route_tests.rs:556  let _ = std::fs::OpenOptions::new().write(true).open(pipe);
GAP test LINE crates/api/src/settings_route_tests.rs:557  };
GAP test LINE crates/api/src/settings_route_tests.rs:560  release(&pipe);
GAP test LINE crates/api/src/settings_route_tests.rs:569  release(&pipe);
GAP test LINE crates/api/src/settings_route_tests.rs:576  release(&pipe);
GAP test LINE crates/api/src/settings_route_tests.rs:861  return None;
GAP test LINE crates/api/src/settings_route_tests.rs:895  return out;
GAP test BRANCH crates/api/src/settings_route_tests.rs:128:24 False  [inside a closure whose false side does not fail the check]  .any(|row| row.label == "POST /settings" && row.response_status == 200),
GAP test BRANCH crates/api/src/settings_route_tests.rs:138:24 False  [inside a closure whose false side does not fail the check]  .any(|row| row.label == "POST /settings" && row.response_status == 409),
GAP test BRANCH crates/api/src/settings_route_tests.rs:398:22 True  Ok(value) if !value.trim().is_empty() => {
GAP test BRANCH crates/api/src/settings_route_tests.rs:398:22 False  Ok(value) if !value.trim().is_empty() => {
GAP test BRANCH crates/api/src/settings_route_tests.rs:860:20 True  if started.elapsed() > std::time::Duration::from_secs(10) {
GAP test BRANCH crates/api/src/settings_route_tests.rs:894:9 False  let Ok(entries) = std::fs::read_dir(dir) else {
GAP prod LINE crates/api/src/sources.rs:196  fn bad(why: impl Into<String>) -> Self {
GAP prod LINE crates/api/src/sources.rs:197  Self {
GAP prod LINE crates/api/src/sources.rs:198  status: axum::http::StatusCode::BAD_REQUEST,
GAP prod LINE crates/api/src/sources.rs:199  why: why.into(),
GAP prod LINE crates/api/src/sources.rs:200  }
GAP prod LINE crates/api/src/sources.rs:201  }
GAP prod LINE crates/api/src/sources.rs:217  return Self::gone(why);
GAP prod LINE crates/api/src/sources.rs:424  out.push('%');
GAP prod LINE crates/api/src/sources.rs:425  for nibble in [byte >> 4, byte & 0x0F] {
GAP prod LINE crates/api/src/sources.rs:426  if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
GAP prod LINE crates/api/src/sources.rs:427  out.push(digit.to_ascii_uppercase());
GAP prod LINE crates/api/src/sources.rs:428  }
GAP prod LINE crates/api/src/sources.rs:468  const fn new() -> Self {
GAP prod LINE crates/api/src/sources.rs:469  Self(OnceLock::new())
GAP prod LINE crates/api/src/sources.rs:470  }
GAP prod LINE crates/api/src/sources.rs:486  held.clear();
GAP prod LINE crates/api/src/sources.rs:508  Err(why) => return Err(format!("cannot read {}: {why}", dir.display())),
GAP prod LINE crates/api/src/sources.rs:706  Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:708  open.file.path().display()
GAP prod LINE crates/api/src/sources.rs:710  });
GAP prod LINE crates/api/src/sources.rs:721  Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:723  file.path().display()
GAP prod LINE crates/api/src/sources.rs:725  })
GAP prod LINE crates/api/src/sources.rs:846  }
GAP prod LINE crates/api/src/sources.rs:1062  _ => Err(bad()),
GAP prod LINE crates/api/src/sources.rs:1100  Self::Unexpected => "unexpected",
GAP prod LINE crates/api/src/sources.rs:1153  Basis::Unverified(why) => json!({
GAP prod LINE crates/api/src/sources.rs:1154  "price": null, "volume": null, "adjusted_as_of": null,
GAP prod LINE crates/api/src/sources.rs:1155  "close": null, "close_label": null, "unverified": why,
GAP prod LINE crates/api/src/sources.rs:1300  Err(why) => faults.push(why),
GAP prod LINE crates/api/src/sources.rs:1315  Refusal::store(format!("cannot read store root {}: {why}", root.display()))
GAP prod LINE crates/api/src/sources.rs:1316  })?;
GAP prod LINE crates/api/src/sources.rs:1318  return Err(Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:1319  "store root {} is not a directory",
GAP prod LINE crates/api/src/sources.rs:1320  root.display()
GAP prod LINE crates/api/src/sources.rs:1321  )));
GAP prod LINE crates/api/src/sources.rs:1428  Sessions::default()
GAP prod LINE crates/api/src/sources.rs:1500  return Sessions {
GAP prod LINE crates/api/src/sources.rs:1501  json: json!({ "calendar": link, "why": "the calendar link would not parse" }),
GAP prod LINE crates/api/src/sources.rs:1502  not_held: None,
GAP prod LINE crates/api/src/sources.rs:1503  };
GAP prod LINE crates/api/src/sources.rs:1633  continue;
GAP prod LINE crates/api/src/sources.rs:1767  "unexpected"
GAP prod LINE crates/api/src/sources.rs:2044  }
GAP prod LINE crates/api/src/sources.rs:2267  notes.push(format!(
GAP prod LINE crates/api/src/sources.rs:2268  "no offered feed holds {} at {} or a finer rung that folds to it",
GAP prod LINE crates/api/src/sources.rs:2269  asked.inst.key(),
GAP prod LINE crates/api/src/sources.rs:2270  asked.rung.as_str()
GAP prod LINE crates/api/src/sources.rs:2271  ));
GAP prod LINE crates/api/src/sources.rs:2388  return none;
GAP prod LINE crates/api/src/sources.rs:2403  ((one, a), (other, b))
GAP prod LINE crates/api/src/sources.rs:2462  fields.insert(name.to_owned(), json!({ "why": "not_a_price" }));
GAP prod LINE crates/api/src/sources.rs:2463  continue;
GAP prod LINE crates/api/src/sources.rs:2535  flags.push("v");
GAP prod LINE crates/api/src/sources.rs:2539  None => json!({ "why": "no_volume_to_compare" }),
GAP prod LINE crates/api/src/sources.rs:2560  _ => "unverified",
GAP prod LINE crates/api/src/sources.rs:2598  match compared.basis {
GAP prod LINE crates/api/src/sources.rs:2672  return Err(format!("from={from} is after to={to}"));
GAP prod LINE crates/api/src/sources.rs:2677  other => return Err(format!("order={other:?} is not asc or desc")),
GAP prod LINE crates/api/src/sources.rs:2762  return Err(Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:2763  "pull::fold made {} day bars of one IST day's records",
GAP prod LINE crates/api/src/sources.rs:2764  folded.len()
GAP prod LINE crates/api/src/sources.rs:2765  )));
GAP prod LINE crates/api/src/sources.rs:2942  return Ok(None);
GAP prod LINE crates/api/src/sources.rs:2976  return Ok(Vec::new());
GAP prod LINE crates/api/src/sources.rs:3013  return Err(Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:3014  "{} holds {} day records on {day}; a day rung holds one",
GAP prod LINE crates/api/src/sources.rs:3015  inst.key(),
GAP prod LINE crates/api/src/sources.rs:3016  zerodha_day.len()
GAP prod LINE crates/api/src/sources.rs:3017  )));
GAP prod LINE crates/api/src/sources.rs:3091  Err(why) => {
GAP prod LINE crates/api/src/sources.rs:3092  return Auction::Unreadable {
GAP prod LINE crates/api/src/sources.rs:3093  file: String::new(),
GAP prod LINE crates/api/src/sources.rs:3094  ordinal: None,
GAP prod LINE crates/api/src/sources.rs:3095  why,
GAP prod LINE crates/api/src/sources.rs:3096  };
GAP prod LINE crates/api/src/sources.rs:3132  continue;
GAP prod LINE crates/api/src/sources.rs:3252  kept
GAP prod LINE crates/api/src/sources.rs:3281  Err(why) => Eligibility::Unverified(why),
GAP prod LINE crates/api/src/sources.rs:3543  }
GAP prod LINE crates/api/src/sources.rs:3753  return Value::Null;
GAP prod LINE crates/api/src/sources.rs:3805  return "no_feed_stores_this_rung";
GAP prod LINE crates/api/src/sources.rs:3813  return "before_archive";
GAP prod LINE crates/api/src/sources.rs:3823  return "outside_your_window";
GAP prod LINE crates/api/src/sources.rs:3856  (true, false) => tally.zerodha_only += 1,
GAP prod LINE crates/api/src/sources.rs:4108  flags.push("open_mismatch");
GAP prod LINE crates/api/src/sources.rs:4117  flags.push("gdfl_extremes_outside_day");
GAP prod BRANCH crates/api/src/sources.rs:216:12 True  if pull::gdfl_lineage::is_contract_refused(&why) {
GAP prod BRANCH crates/api/src/sources.rs:248:20 False  && let Some((status, body)) =
GAP prod BRANCH crates/api/src/sources.rs:264:12 False  && inst.contract.is_some()
GAP prod BRANCH crates/api/src/sources.rs:426:24 True  if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
GAP prod BRANCH crates/api/src/sources.rs:426:24 False  if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
GAP prod BRANCH crates/api/src/sources.rs:485:12 True  if held.len() >= MEMO_CAP {
GAP prod BRANCH crates/api/src/sources.rs:507:21 False  Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Arc::from([])),
GAP prod BRANCH crates/api/src/sources.rs:841:16 False  if !pending.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:896:18 True  if b <= 0 || a < 0 {
GAP prod BRANCH crates/api/src/sources.rs:908:18 True  if b <= 0 || a < 0 {
GAP prod BRANCH crates/api/src/sources.rs:926:5 False  z > 0 && (g - z).abs() * 1_000 > FACTOR_THRESHOLD_PERMILLE * z
GAP prod BRANCH crates/api/src/sources.rs:939:5 False  base > 0 && (g1 * z0 - base).abs() * 1_000 > FACTOR_THRESHOLD_PERMILLE * base
GAP prod BRANCH crates/api/src/sources.rs:983:51 False  pull::calendar::DayKind::Open(session) if session.count > 0 => session
GAP prod BRANCH crates/api/src/sources.rs:1044:29 False  .filter(|field| field.len() == width && field.bytes().all(|b| b.is_ascii_digit()))
GAP prod BRANCH crates/api/src/sources.rs:1317:8 True  if !meta.is_dir() {
GAP prod BRANCH crates/api/src/sources.rs:1336:31 False  if held.is_empty() && faults.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:1416:48 False  if rung.rung == Timeframe::SECOND_1 && rung.kind == Kind::Stored {
GAP prod BRANCH crates/api/src/sources.rs:1499:9 False  let Ok(uri) = link.parse::<axum::http::Uri>() else {
GAP prod BRANCH crates/api/src/sources.rs:1509:8 True  if !status.is_success() || answer.get("status").and_then(Value::as_str) != Some("ok") {
GAP prod BRANCH crates/api/src/sources.rs:1543:16 False  whole: !truncated && u64::try_from(days.len()).is_ok_and(|named| named == count),
GAP prod BRANCH crates/api/src/sources.rs:1573:25 False  if derived.whole && minutes.whole {
GAP prod BRANCH crates/api/src/sources.rs:1632:13 False  let (Some(&before), Some(&after)) = (pair.first(), pair.get(1)) else {
GAP prod BRANCH crates/api/src/sources.rs:1638:16 False  if named.len() < MAX_NAMED {
GAP prod BRANCH crates/api/src/sources.rs:1653:15 True  while theirs.next_if(|at| **at < month).is_some() {}
GAP prod BRANCH crates/api/src/sources.rs:1695:16 False  if uncounted.len() < MAX_NAMED {
GAP prod BRANCH crates/api/src/sources.rs:1739:37 True  (!missing.is_empty() || days.is_some_and(|d| d > 0) || sessions_short).then(|| {
GAP prod BRANCH crates/api/src/sources.rs:1765:40 True  held.kind == Kind::Derived && (sessions.not_held.is_none() || minute.not_held.is_none());
GAP prod BRANCH crates/api/src/sources.rs:1766:20 True  let state = if held.kind == Kind::Unexpected {
GAP prod BRANCH crates/api/src/sources.rs:1770:31 True  } else if !census_held || uncounted_count > 0 {
GAP prod BRANCH crates/api/src/sources.rs:1927:12 True  if limit == 0 || limit > most {
GAP prod BRANCH crates/api/src/sources.rs:2036:16 False  if !held.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:2129:20 False  if let Some(i) = at.get_mut(n)
GAP prod BRANCH crates/api/src/sources.rs:2266:8 True  if sources.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:2387:8 True  if one.series.vendor == other.series.vendor {
GAP prod BRANCH crates/api/src/sources.rs:2400:55 False  let ((z_source, z_cell), (g_source, g_cell)) = if !is_base(one) && is_base(other) {
GAP prod BRANCH crates/api/src/sources.rs:2400:72 False  let ((z_source, z_cell), (g_source, g_cell)) = if !is_base(one) && is_base(other) {
GAP prod BRANCH crates/api/src/sources.rs:2433:35 True  let row_basis = if !shares || bases != Some(true) {
GAP prod BRANCH crates/api/src/sources.rs:2458:16 False  if let Some(slot) = equal.get_mut(at) {
GAP prod BRANCH crates/api/src/sources.rs:2461:13 False  let Some((bps, beyond)) = price_cmp(gv, zv) else {
GAP prod BRANCH crates/api/src/sources.rs:2534:24 True  if beyond {
GAP prod BRANCH crates/api/src/sources.rs:2671:12 True  if from > to {
GAP prod BRANCH crates/api/src/sources.rs:2761:13 False  let [only] = folded.as_slice() else {
GAP prod BRANCH crates/api/src/sources.rs:2941:17 False  let Some((_, vendor)) = feed_holding(rung) else {
GAP prod BRANCH crates/api/src/sources.rs:2975:9 False  let Some((series, months)) = held else {
GAP prod BRANCH crates/api/src/sources.rs:3012:8 True  if zerodha_day.len() > 1 {
GAP prod BRANCH crates/api/src/sources.rs:3127:12 True  if t < month_lo || t >= month_hi {
GAP prod BRANCH crates/api/src/sources.rs:3131:12 True  if t < lo || t >= hi {
GAP prod BRANCH crates/api/src/sources.rs:3131:22 True  if t < lo || t >= hi {
GAP prod BRANCH crates/api/src/sources.rs:3139:17 False  bar.close == record.price_paisa
GAP prod BRANCH crates/api/src/sources.rs:3251:25 True  let master = if let Some(kept) = MASTERS.get(&memo) {
GAP prod BRANCH crates/api/src/sources.rs:3537:12 False  if let Some(records) = flags.get("days").and_then(Value::as_array) {
GAP prod BRANCH crates/api/src/sources.rs:3539:20 False  if let Some(day) = record.get("day").and_then(Value::as_str) {
GAP prod BRANCH crates/api/src/sources.rs:3578:16 False  if let Some(object) = row.as_object_mut() {
GAP prod BRANCH crates/api/src/sources.rs:3681:12 False  if let Some(flag_notes) = flag_notes {
GAP prod BRANCH crates/api/src/sources.rs:3752:9 False  let Some((tables, symbol)) = &context.measured else {
GAP prod BRANCH crates/api/src/sources.rs:3804:9 False  let Some((series, months)) = held else {
GAP prod BRANCH crates/api/src/sources.rs:3812:12 True  if day < span.from {
GAP prod BRANCH crates/api/src/sources.rs:3819:8 True  if descriptor
GAP prod BRANCH crates/api/src/sources.rs:3821:29 True  .is_some_and(|span| day < span.from || day > span.to)
GAP prod BRANCH crates/api/src/sources.rs:3884:19 False  shares && bases == Some(true) && factor_pair.is_some_and(|(g, z)| basis_differs(g, z));
GAP prod BRANCH crates/api/src/sources.rs:3891:40 False  .map(|(before, now)| shares && bases == Some(true) && factor_stepped(before, now));
GAP prod BRANCH crates/api/src/sources.rs:3955:35 True  .is_some_and(|(d, g)| g.bar.high > d.high || g.bar.low < d.low);
GAP prod BRANCH crates/api/src/sources.rs:4071:17 False  scaled > 0 && (zv * zc - scaled).abs() * 1_000 > VOLUME_TOLERANCE_PERMILLE * scaled
GAP prod BRANCH crates/api/src/sources.rs:4087:17 False  gc > 0 && zg > (gg * zc + gc - 1) / gc
GAP prod BRANCH crates/api/src/sources.rs:4107:8 True  if open_mismatch {
GAP prod BRANCH crates/api/src/sources.rs:4116:8 True  if gdfl_outside {
GAP prod BRANCH crates/api/src/sources.rs:4130:22 False  if minute_off && !index {
GAP test LINE crates/api/src/sources_tests.rs:429  other => panic!("not a route here: {other}"),
GAP test BRANCH crates/api/src/sources_tests.rs:867:33 False  [inside a closure whose false side does not fail the check]  notes.iter().any(|note| note.starts_with("Zerodha")
GAP prod LINE crates/api/src/speed.rs:349  _ => 0,
GAP prod LINE crates/api/src/speed.rs:426  max
GAP prod LINE crates/api/src/speed.rs:530  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
GAP prod LINE crates/api/src/speed.rs:531  f.debug_struct("Speed")
GAP prod LINE crates/api/src/speed.rs:532  .field("started_ms", &self.started_ms)
GAP prod LINE crates/api/src/speed.rs:533  .field("cap", &self.cap)
GAP prod LINE crates/api/src/speed.rs:534  .finish_non_exhaustive()
GAP prod LINE crates/api/src/speed.rs:535  }
GAP prod LINE crates/api/src/speed.rs:543  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
GAP prod LINE crates/api/src/speed.rs:544  f.debug_tuple("Handle").field(&self.0.name).finish()
GAP prod LINE crates/api/src/speed.rs:545  }
GAP prod LINE crates/api/src/speed.rs:586  return Handle(Arc::clone(row));
GAP prod LINE crates/api/src/speed.rs:756  let _said = telemetry::emit(
GAP prod LINE crates/api/src/speed.rs:757  &telemetry::Event::error("api.speed", "the minute thread did not start")
GAP prod LINE crates/api/src/speed.rs:758  .with("why", telemetry::Value::Str(&why.to_string())),
GAP prod LINE crates/api/src/speed.rs:759  );
GAP prod LINE crates/api/src/speed.rs:775  }
GAP prod LINE crates/api/src/speed.rs:888  Ok::<(), std::io::Error>(())
GAP prod LINE crates/api/src/speed.rs:889  };
GAP prod LINE crates/api/src/speed.rs:895  std::future::pending::<()>().await;
GAP prod LINE crates/api/src/speed.rs:896  }
GAP prod LINE crates/api/src/speed.rs:897  })
GAP prod LINE crates/api/src/speed.rs:900  Err(why) => {
GAP prod LINE crates/api/src/speed.rs:901  let _said = telemetry::emit(
GAP prod LINE crates/api/src/speed.rs:902  &telemetry::Event::error("api.speed", "SIGTERM cannot be caught")
GAP prod LINE crates/api/src/speed.rs:903  .with("why", telemetry::Value::Str(&why.to_string())),
GAP prod LINE crates/api/src/speed.rs:905  ctrl_c.await
GAP prod LINE crates/api/src/speed.rs:908  }
GAP prod LINE crates/api/src/speed.rs:930  pub fn stage(name: &str, micros: u64, outcome: Outcome) {
GAP prod LINE crates/api/src/speed.rs:931  let now = now_ms();
GAP prod LINE crates/api/src/speed.rs:932  global().record(&format!("stage:{name}"), Kind::Stage, micros, outcome, now);
GAP prod LINE crates/api/src/speed.rs:933  }
GAP prod BRANCH crates/api/src/speed.rs:270:16 False  if let Some(bucket) = self.live.buckets.get(bucket_of(micros)) {
GAP prod BRANCH crates/api/src/speed.rs:273:16 False  if let Some(count) = self.live.outcomes.get(outcome.slot()) {
GAP prod BRANCH crates/api/src/speed.rs:585:16 True  if let Some(row) = rows.by_name.get(name) {
GAP prod BRANCH crates/api/src/speed.rs:640:20 False  if let Some((figures, window_ms)) = figures
GAP prod BRANCH crates/api/src/speed.rs:755:16 True  if let Err(why) = spawned {
GAP prod BRANCH crates/api/src/speed.rs:771:16 False  if let Some(table) = TABLE.get() {
GAP prod BRANCH crates/api/src/speed.rs:894:24 True  if term.recv().await.is_none() {
GAP prod BRANCH crates/api/src/speed.rs:894:24 False  if term.recv().await.is_none() {
GAP prod BRANCH crates/api/src/speed.rs:1087:16 False  if let Some(row) = self.row.take() {
GAP prod BRANCH crates/api/src/speed.rs:1184:16 True  || now_ms < window.opened_ms
GAP prod LINE crates/api/src/speed_route.rs:69  Err(why) => (
GAP prod LINE crates/api/src/speed_route.rs:70  StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/speed_route.rs:71  format!(
GAP prod LINE crates/api/src/speed_route.rs:72  "{{\"error\":\"the speed report would not serialise: {}\"}}",
GAP prod LINE crates/api/src/speed_route.rs:73  why.to_string().replace(['"', '\\'], "'")
GAP prod LINE crates/api/src/speed_route.rs:74  ),
GAP prod LINE crates/api/src/speed_route.rs:75  ),
GAP test LINE crates/api/src/stop_tests.rs:270  fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
GAP test LINE crates/api/src/stop_tests.rs:271  Ok(())
GAP test LINE crates/api/src/stop_tests.rs:272  }
GAP test LINE crates/api/src/stop_tests.rs:282  fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
GAP test LINE crates/api/src/stop_tests.rs:283  Ok(())
GAP test LINE crates/api/src/stop_tests.rs:284  }
GAP test BRANCH crates/api/src/stop_tests.rs:143:26 False  .any(|(name, n)| name == route && *n == 1)
GAP test BRANCH crates/api/src/stop_tests.rs:163:27 False  [inside a closure whose false side does not fail the check]  .any(|record| field_of(record, "drain_ms") == 200
GAP test BRANCH crates/api/src/stop_tests.rs:174:13 False  crate::emitted::says(record, "routes", &format!("{route}=1"))
GAP test BRANCH crates/api/src/stop_tests.rs:423:34 False  [inside a closure whose false side does not fail the check]  said.iter().any(|record| field_of(record, "grace_ms") == 100
GAP test BRANCH crates/api/src/stop_tests.rs:597:9 False  let Loading::Failed(why) = loaded_or_stopped(
GAP prod LINE crates/api/src/store_route.rs:331  parts.push(format!(
GAP prod LINE crates/api/src/store_route.rs:332  "the import journal could not be read ({why}), so whether this census is \
GAP prod LINE crates/api/src/store_route.rs:333  behind its import is not known"
GAP prod LINE crates/api/src/store_route.rs:334  ));
GAP prod LINE crates/api/src/store_route.rs:437  Err(why) => {
GAP prod LINE crates/api/src/store_route.rs:438  return Behind {
GAP prod LINE crates/api/src/store_route.rs:439  kept: true,
GAP prod LINE crates/api/src/store_route.rs:440  unread: Some(why),
GAP prod LINE crates/api/src/store_route.rs:441  ..Behind::default()
GAP prod LINE crates/api/src/store_route.rs:442  };
GAP prod LINE crates/api/src/store_route.rs:515  census::Census::Absent | census::Census::Unreadable { .. } => None,
GAP prod LINE crates/api/src/store_route.rs:914  Err(why) => {
GAP prod LINE crates/api/src/store_route.rs:915  return (
GAP prod LINE crates/api/src/store_route.rs:916  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/store_route.rs:917  headers,
GAP prod LINE crates/api/src/store_route.rs:918  serde_json::json!({"error":why}).to_string().into(),
GAP prod LINE crates/api/src/store_route.rs:919  );
GAP prod LINE crates/api/src/store_route.rs:1045  (month.year().checked_add(1)?, 1)
GAP prod LINE crates/api/src/store_route.rs:1165  }
GAP prod LINE crates/api/src/store_route.rs:1245  census::Census::Absent | census::Census::Unreadable { .. } => None,
GAP prod LINE crates/api/src/store_route.rs:1297  Err(why) => (
GAP prod LINE crates/api/src/store_route.rs:1298  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/store_route.rs:1299  serde_json::json!({"error":why}).to_string(),
GAP prod LINE crates/api/src/store_route.rs:1300  ),
GAP prod BRANCH crates/api/src/store_route.rs:330:16 True  if let Some(why) = &behind.unread {
GAP prod BRANCH crates/api/src/store_route.rs:447:12 False  if let Ok(held) = cache.0.lock()
GAP prod BRANCH crates/api/src/store_route.rs:455:12 False  if let Ok(mut held) = cache.0.lock() {
GAP prod BRANCH crates/api/src/store_route.rs:482:34 True  if !refused_by_census || counted == Some(row.day) {
GAP prod BRANCH crates/api/src/store_route.rs:887:29 False  if !since.is_empty() && code == axum::http::StatusCode::OK {
GAP prod BRANCH crates/api/src/store_route.rs:928:12 False  if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
GAP prod BRANCH crates/api/src/store_route.rs:1042:30 False  let (year, ordinal) = if month.month() < 12 {
GAP prod BRANCH crates/api/src/store_route.rs:1069:20 False  && parts.iter().all(|part| {
GAP prod BRANCH crates/api/src/store_route.rs:1157:20 False  if let Some(next) = month_after(key.month) {
GAP prod LINE crates/api/src/store_wire.rs:72  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
GAP prod LINE crates/api/src/store_wire.rs:73  f.write_str("KeptCell")
GAP prod LINE crates/api/src/store_wire.rs:74  }
GAP prod LINE crates/api/src/store_wire.rs:278  continue;
GAP test LINE crates/api/src/store_wire.rs:477  [read: a builder closure inside the condition, never called because the cache refused first: not the failing arm]  "[]".into()
GAP test LINE crates/api/src/store_wire.rs:522  || -> Result<u64, String> { panic!("derived twice") },
GAP prod BRANCH crates/api/src/store_wire.rs:154:16 False  && *kept_extra == extra
GAP prod BRANCH crates/api/src/store_wire.rs:155:20 False  && let Ok(value) = Arc::clone(value).downcast::<T>()
GAP prod BRANCH crates/api/src/store_wire.rs:277:17 False  let Some(row) = held_row(census, &series.at(*month)) else {
GAP test LINE crates/api/src/strict_sweep_tests.rs:380  called.set(true);
GAP test LINE crates/api/src/strict_sweep_tests.rs:381  Ok(config())
GAP test LINE crates/api/src/strict_sweep_tests.rs:382  });
GAP test LINE crates/api/src/strict_sweep_tests.rs:462  assert!(why.contains("verified clean build identity"), "{why}");
GAP test LINE crates/api/src/strict_sweep_tests.rs:490  configured.set(true);
GAP test LINE crates/api/src/strict_sweep_tests.rs:491  Ok(config())
GAP test LINE crates/api/src/strict_sweep_tests.rs:492  });
GAP test LINE crates/api/src/strict_sweep_tests.rs:537  configured.set(true);
GAP test LINE crates/api/src/strict_sweep_tests.rs:538  Ok(config())
GAP test LINE crates/api/src/strict_sweep_tests.rs:539  });
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:351:26 False  [inside a closure whose false side does not fail the check]  .any(|event| event.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:352:20 False  [read: an operand inside an .any() closure opened above: not the failing arm]  && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_RECEIPTS")
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:353:20 False  [read: an operand inside an .any() closure opened above: not the failing arm]  && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_MAX_BYTES")
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:354:20 False  [read: an operand inside an .any() closure opened above: not the failing arm]  && crate::emitted::says(event, "why", "BRUTEX_CHECKSUM_MAX_RECORDS")
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:461:8 True  if cli::commit_stamp().is_none() {
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:465:13 False  [an operand followed by ||: neither side fails the check by itself]  why.contains("No such file") || why.contains("missing") || why.contains("absent"),
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:465:45 True  why.contains("No such file") || why.contains("missing") || why.contains("absent"),
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:465:45 False  why.contains("No such file") || why.contains("missing") || why.contains("absent"),
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:465:72 True  why.contains("No such file") || why.contains("missing") || why.contains("absent"),
GAP test BRANCH crates/api/src/strict_sweep_tests.rs:465:72 False  why.contains("No such file") || why.contains("missing") || why.contains("absent"),
GAP prod LINE crates/api/src/sweepevidence.rs:98  Ok(_) => refusal(
GAP prod LINE crates/api/src/sweepevidence.rs:100  "saved evidence exceeds the JSON response bound; no prefix was exposed",
GAP prod LINE crates/api/src/sweepevidence.rs:111  return Err("the requested saved attempt is no longer available; no replacement page was served".to_owned());
GAP prod LINE crates/api/src/sweepevidence.rs:156  )? {
GAP prod LINE crates/api/src/sweepevidence.rs:169  )? {
GAP prod LINE crates/api/src/sweepevidence.rs:176  }
GAP prod LINE crates/api/src/sweepevidence.rs:179  return Err(
GAP prod LINE crates/api/src/sweepevidence.rs:180  "saved evidence changed during the bounded read; no mixed snapshot was exposed"
GAP prod LINE crates/api/src/sweepevidence.rs:181  .to_owned(),
GAP prod LINE crates/api/src/sweepevidence.rs:182  );
GAP prod BRANCH crates/api/src/sweepevidence.rs:95:21 False  Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
GAP prod BRANCH crates/api/src/sweepevidence.rs:110:12 True  if asked.attempt.is_some() || asked.page.number != 0 {
GAP prod BRANCH crates/api/src/sweepevidence.rs:110:39 True  if asked.attempt.is_some() || asked.page.number != 0 {
GAP prod BRANCH crates/api/src/sweepevidence.rs:148:8 False  if limit != 0 {
GAP prod BRANCH crates/api/src/sweepevidence.rs:177:8 True  if sweep_evidence::read(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? != Some(evidence)
GAP prod LINE crates/api/src/sweeprun.rs:223  }
GAP prod LINE crates/api/src/sweeprun.rs:1433  None => work(),
GAP prod LINE crates/api/src/sweeprun.rs:1459  _ => Phase::Failed,
GAP prod LINE crates/api/src/sweeprun.rs:1845  Refusal::Unobservable(format!(
GAP prod LINE crates/api/src/sweeprun.rs:1846  "required durable invocation audit could not start: {why}. No engine work was started."
GAP prod LINE crates/api/src/sweeprun.rs:1847  ))
GAP prod LINE crates/api/src/sweeprun.rs:1848  })
GAP prod LINE crates/api/src/sweeprun.rs:2018  Refusal::Unobservable(why.to_string())
GAP prod LINE crates/api/src/sweeprun.rs:2025  )?;
GAP prod LINE crates/api/src/sweeprun.rs:2026  }
GAP prod LINE crates/api/src/sweeprun.rs:2131  let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
GAP prod LINE crates/api/src/sweeprun.rs:2132  return Err(refused(&why));
GAP prod LINE crates/api/src/sweeprun.rs:2281  let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
GAP prod LINE crates/api/src/sweeprun.rs:2282  return Err(refused(&why));
GAP prod LINE crates/api/src/sweeprun.rs:2413  Err(why) => {
GAP prod LINE crates/api/src/sweeprun.rs:2414  let why = format!(
GAP prod LINE crates/api/src/sweeprun.rs:2417  (
GAP prod LINE crates/api/src/sweeprun.rs:2418  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/sweeprun.rs:2419  json_headers(),
GAP prod LINE crates/api/src/sweeprun.rs:2420  unknown_status(&why),
GAP prod LINE crates/api/src/sweeprun.rs:2421  )
GAP prod LINE crates/api/src/sweeprun.rs:2609  _ => return None,
GAP prod LINE crates/api/src/sweeprun.rs:2755  "No sweep has run; the retained CLI lifecycle log names no command.".to_owned()
GAP prod LINE crates/api/src/sweeprun.rs:3079  None
GAP prod LINE crates/api/src/sweeprun.rs:3093  return uncertain_observation(Some(marker.at_unix_millis), &why);
GAP prod LINE crates/api/src/sweeprun.rs:3098  _ => "unknown",
GAP prod LINE crates/api/src/sweeprun.rs:3228  Self::WalkNotKept => "walk-not-kept",
GAP prod LINE crates/api/src/sweeprun.rs:3241  "The CLI log was walked and the walk was not remembered: a file of the log changed while it was read, or could not be described by stat. The
GAP prod LINE crates/api/src/sweeprun.rs:3941  "refused: declared Boolean research requires its prepared audited browser dispatch"
GAP prod LINE crates/api/src/sweeprun.rs:3942  .into()
GAP prod LINE crates/api/src/sweeprun.rs:3948  (Ok(configuration), Ok(root)) => {
GAP prod LINE crates/api/src/sweeprun.rs:3949  conduct_strict_command(asked, now_micros, attempt, &configuration, &root)
GAP prod LINE crates/api/src/sweeprun.rs:3955  (_, Err(why)) => {
GAP prod LINE crates/api/src/sweeprun.rs:3956  settle(&mut progress, format!("refused: {why}"), now_micros);
GAP prod LINE crates/api/src/sweeprun.rs:3957  progress
GAP prod LINE crates/api/src/sweeprun.rs:4160  let _terminal = audit.finish(cli::operation_audit::Phase::Refused, 0);
GAP prod LINE crates/api/src/sweeprun.rs:4161  return Err(refused(&why));
GAP prod LINE crates/api/src/sweeprun.rs:4196  Some(config) => {
GAP prod LINE crates/api/src/sweeprun.rs:4197  conduct_strict_command(&asked, started, attempt, config, &store_root)
GAP prod LINE crates/api/src/sweeprun.rs:4349  _ => Err(
GAP prod LINE crates/api/src/sweeprun.rs:4350  "strict input configuration requires the explicit audit-audited-range command"
GAP prod LINE crates/api/src/sweeprun.rs:4351  .to_owned(),
GAP prod LINE crates/api/src/sweeprun.rs:4352  ),
GAP prod LINE crates/api/src/sweeprun.rs:4377  "1".clone_into(value);
GAP prod LINE crates/api/src/sweeprun.rs:4385  return None;
GAP test LINE crates/api/src/sweeprun.rs:4593  }
GAP test LINE crates/api/src/sweeprun.rs:4594  if month > 3_000 {
GAP test LINE crates/api/src/sweeprun.rs:4595  break "a complete report the stop never reached".to_owned();
GAP test LINE crates/api/src/sweeprun.rs:4596  }
GAP test LINE crates/api/src/sweeprun.rs:4597  std::thread::sleep(std::time::Duration::from_millis(20));
GAP test LINE crates/api/src/sweeprun.rs:4681  || {
GAP test LINE crates/api/src/sweeprun.rs:4682  first_busy_ref.store(true, Ordering::SeqCst);
GAP test LINE crates/api/src/sweeprun.rs:4683  busy()
GAP test LINE crates/api/src/sweeprun.rs:4684  },
GAP test LINE crates/api/src/sweeprun.rs:4702  second_prepared.store(true, Ordering::SeqCst);
GAP test LINE crates/api/src/sweeprun.rs:4703  Ok((running(2), "second"))
GAP test LINE crates/api/src/sweeprun.rs:4704  })
GAP prod LINE crates/api/src/sweeprun.rs:5270  why.why()
GAP prod LINE crates/api/src/sweeprun.rs:5275  why.why()
GAP prod LINE crates/api/src/sweeprun.rs:5871  path.display()
GAP prod LINE crates/api/src/sweeprun.rs:6202  why.why()
GAP prod LINE crates/api/src/sweeprun.rs:6228  why.why()
GAP prod BRANCH crates/api/src/sweeprun.rs:204:20 False  && let Some(fields) = search.fields().as_object()
GAP prod BRANCH crates/api/src/sweeprun.rs:212:20 False  if let Some(object) = fields.as_object_mut() {
GAP prod BRANCH crates/api/src/sweeprun.rs:2021:12 False  if let Some(dir) = crate::logs::cli_log_dir() {
GAP prod BRANCH crates/api/src/sweeprun.rs:2130:16 True  if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
GAP prod BRANCH crates/api/src/sweeprun.rs:2280:16 True  if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
GAP prod BRANCH crates/api/src/sweeprun.rs:2479:12 False  && external
GAP prod BRANCH crates/api/src/sweeprun.rs:2608:58 False  Some(telemetry::OwnedValue::Str(command)) if !command.is_empty() => command.clone(),
GAP prod BRANCH crates/api/src/sweeprun.rs:2661:12 True  if lifecycle.hit_scan_cap
GAP prod BRANCH crates/api/src/sweeprun.rs:2723:19 False  Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
GAP prod BRANCH crates/api/src/sweeprun.rs:2736:24 False  if !names.contains(&name) {
GAP prod BRANCH crates/api/src/sweeprun.rs:2980:12 False  if same(&through).is_some() && same(&through) == same(&lifecycle) {
GAP prod BRANCH crates/api/src/sweeprun.rs:2980:40 False  if same(&through).is_some() && same(&through) == same(&lifecycle) {
GAP prod BRANCH crates/api/src/sweeprun.rs:3076:19 False  quiet: if launch_clear {
GAP prod BRANCH crates/api/src/sweeprun.rs:3092:12 True  if let Some(why) = tail_fault_unless_answered(activity, true) {
GAP prod BRANCH crates/api/src/sweeprun.rs:3100:102 False  let named_sweep = matches!(marker.field("command"), Some(telemetry::OwnedValue::Str(command)) if cli::is_sweep_command(command));
GAP prod BRANCH crates/api/src/sweeprun.rs:3104:45 False  if age <= STALE_AFTER_MILLIS && marker.run > 0 && named_sweep =>
GAP prod BRANCH crates/api/src/sweeprun.rs:3104:63 False  if age <= STALE_AFTER_MILLIS && marker.run > 0 && named_sweep =>
GAP prod BRANCH crates/api/src/sweeprun.rs:3108:46 False  ("command finished", "completed") if marker.run > 0 && named_sweep => ("completed", ""),
GAP prod BRANCH crates/api/src/sweeprun.rs:3109:44 False  ("command finished", "refused") if marker.run > 0 && named_sweep => (
GAP prod BRANCH crates/api/src/sweeprun.rs:3109:62 False  ("command finished", "refused") if marker.run > 0 && named_sweep => (
GAP prod BRANCH crates/api/src/sweeprun.rs:3450:20 False  && named_sweep
GAP prod BRANCH crates/api/src/sweeprun.rs:3482:16 False  if record.origin == cli::operation_audit::Origin::Cli && record.label == *command =>
GAP prod BRANCH crates/api/src/sweeprun.rs:4159:16 True  if let Some(why) = marker_refusal(emit_attempt_started(&accepted)) {
GAP prod BRANCH crates/api/src/sweeprun.rs:4376:42 True  if *name == "BRUTEX_VALIDATE" && value != "0" {
GAP prod BRANCH crates/api/src/sweeprun.rs:4384:9 False  let Command::AuditAuditedRange { span, knobs, .. } = asked else {
GAP test BRANCH crates/api/src/sweeprun.rs:4591:28 False  if let Err(why) = cli::cancel::check(|| format!("generated month {month}")) {
GAP test BRANCH crates/api/src/sweeprun.rs:4594:24 True  if month > 3_000 {
GAP test BRANCH crates/api/src/sweeprun.rs:4594:24 False  if month > 3_000 {
GAP prod BRANCH crates/api/src/sweeprun.rs:5189:17 False  name != "BRUTEX_STORE" && name != "BRUTEX_LOG_DIR",
GAP prod BRANCH crates/api/src/sweeprun.rs:5189:43 False  name != "BRUTEX_STORE" && name != "BRUTEX_LOG_DIR",
GAP prod BRANCH crates/api/src/sweeprun.rs:5774:21 False  body.contains("strict_runtime_settings_invalid")
GAP prod BRANCH crates/api/src/sweeprun.rs:5775:28 False  && body.contains("BRUTEX_SCREEN_BUDGET_MS"),
GAP prod BRANCH crates/api/src/sweeprun.rs:5869:17 False  path.starts_with(&journal) || before.iter().any(|(was, _)| *was == path),
GAP prod BRANCH crates/api/src/sweeprun.rs:5869:47 True  path.starts_with(&journal) || before.iter().any(|(was, _)| *was == path),
GAP prod BRANCH crates/api/src/sweeprun.rs:5869:47 False  path.starts_with(&journal) || before.iter().any(|(was, _)| *was == path),
GAP test LINE crates/api/src/sweeprun_status_cache_tests.rs:26  }
GAP test BRANCH crates/api/src/sweeprun_status_cache_tests.rs:23:12 False  if !dir.exists() {
GAP prod LINE crates/api/src/topjson.rs:81  Err(why) => return refusal(axum::http::StatusCode::BAD_REQUEST, &why),
GAP prod LINE crates/api/src/topjson.rs:137  return refusal(
GAP prod LINE crates/api/src/topjson.rs:139  "top report exceeds the bounded JSON response size; no prefix was exposed",
GAP prod LINE crates/api/src/topjson.rs:170  Err(why) => return Err(why),
GAP prod LINE crates/api/src/topjson.rs:177  return Err("top result exceeds the 4096-row detail bound; no prefix was read".to_owned());
GAP prod LINE crates/api/src/topjson.rs:180  return Ok(cli::render_top_record(
GAP prod LINE crates/api/src/topjson.rs:181  &row,
GAP prod LINE crates/api/src/topjson.rs:182  &[],
GAP prod LINE crates/api/src/topjson.rs:183  None,
GAP prod LINE crates/api/src/topjson.rs:184  "No committed result-set receipt exists; no orphan frontier rows are displayed.",
GAP prod LINE crates/api/src/topjson.rs:185  ));
GAP prod LINE crates/api/src/topjson.rs:192  )??;
GAP prod LINE crates/api/src/topjson.rs:194  return Err(why);
GAP test LINE crates/api/src/topjson.rs:474  |_| Ok(()),
GAP prod BRANCH crates/api/src/topjson.rs:59:24 False  if row.pessimistic >= old.pessimistic {
GAP prod BRANCH crates/api/src/topjson.rs:110:25 False  (true, true) if seen.is_empty() => Ok(None),
GAP prod BRANCH crates/api/src/topjson.rs:136:16 True  if body.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/topjson.rs:169:21 False  Err(why) if why.contains("nothing was created") || why.contains("nothing was written") => return Ok("  NO RUN HAS BEEN RECORDED YET. The led
GAP prod BRANCH crates/api/src/topjson.rs:169:60 True  Err(why) if why.contains("nothing was created") || why.contains("nothing was written") => return Ok("  NO RUN HAS BEEN RECORDED YET. The led
GAP prod BRANCH crates/api/src/topjson.rs:169:60 False  Err(why) if why.contains("nothing was created") || why.contains("nothing was written") => return Ok("  NO RUN HAS BEEN RECORDED YET. The led
GAP prod BRANCH crates/api/src/topjson.rs:176:8 True  if receipt.is_some_and(|receipt| receipt.frontier_rows > MAX_RESULT_ROWS) {
GAP prod BRANCH crates/api/src/topjson.rs:179:9 False  let Some(receipt) = receipt else {
GAP prod BRANCH crates/api/src/topjson.rs:193:12 True  if let Some(why) = partial {
GAP prod LINE crates/api/src/trades.rs:104  return unavailable(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why);
GAP prod LINE crates/api/src/trades.rs:113  Err(why) => return refuse(json, &why),
GAP prod LINE crates/api/src/trades.rs:181  return refuse(
GAP prod LINE crates/api/src/trades.rs:182  json,
GAP prod LINE crates/api/src/trades.rs:183  "the validated trade page window did not fit its verified row vector; no fallback page is exposed",
GAP prod LINE crates/api/src/trades.rs:253  return too_large(
GAP prod LINE crates/api/src/trades.rs:254  json,
GAP prod LINE crates/api/src/trades.rs:255  &identity,
GAP prod LINE crates/api/src/trades.rs:256  u64::try_from(rows.len()).unwrap_or(u64::MAX),
GAP prod LINE crates/api/src/trades.rs:257  &format!(
GAP prod LINE crates/api/src/trades.rs:258  "the bounded trade response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
GAP prod LINE crates/api/src/trades.rs:259  out.len(),
GAP prod LINE crates/api/src/trades.rs:260  crate::detail::MAX_RESPONSE_BYTES
GAP prod LINE crates/api/src/trades.rs:261  ),
GAP prod LINE crates/api/src/trades.rs:299  out.push(',');
GAP prod LINE crates/api/src/trades.rs:350  Err(unreadable) => {
GAP prod LINE crates/api/src/trades.rs:351  return refuse(
GAP prod LINE crates/api/src/trades.rs:352  json,
GAP prod LINE crates/api/src/trades.rs:353  &format!(
GAP prod LINE crates/api/src/trades.rs:354  "{why}. {} could not be checked for absence: {unreadable}",
GAP prod LINE crates/api/src/trades.rs:355  child.display()
GAP prod LINE crates/api/src/trades.rs:356  ),
GAP prod LINE crates/api/src/trades.rs:363  return range_refusal(json, identity, 0, &range);
GAP prod LINE crates/api/src/trades.rs:401  return range_refusal(json, identity, 0, &range);
GAP prod BRANCH crates/api/src/trades.rs:99:12 True  if let Err(why) = crate::detail::preflight([
GAP prod BRANCH crates/api/src/trades.rs:180:9 False  let Some(page_rows) = rows.get(window.start..window.end) else {
GAP prod BRANCH crates/api/src/trades.rs:252:8 True  if out.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/trades.rs:298:16 True  if n > 0 {
GAP prod BRANCH crates/api/src/trades.rs:340:25 False  Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {}
GAP prod BRANCH crates/api/src/trades.rs:362:20 True  if let Err(range) = crate::detail::window(0, page) {
GAP prod BRANCH crates/api/src/trades.rs:400:12 True  if let Err(range) = crate::detail::window(0, page) {
GAP test BRANCH crates/api/src/trades.rs:1010:16 False  if let Some(byte) = damaged.get_mut(40) {
GAP test BRANCH crates/api/src/trades.rs:1075:16 False  if let Some(byte) = damaged.get_mut(40) {
GAP prod LINE crates/api/src/verify.rs:277  report.refused = Some(format!(
GAP prod LINE crates/api/src/verify.rs:278  "{}'s counter loaded DEGRADED — {why}. The {} entry(s) it still \
GAP prod LINE crates/api/src/verify.rs:279  holds were checked and are reported above; the ones it lost cannot \
GAP prod LINE crates/api/src/verify.rs:280  be, because a scrub can only ask about a month the counter \
GAP prod LINE crates/api/src/verify.rs:281  remembers.",
GAP prod LINE crates/api/src/verify.rs:282  census.vendor.as_str(),
GAP prod LINE crates/api/src/verify.rs:283  report.tally.seen()
GAP prod LINE crates/api/src/verify.rs:284  ));
GAP prod LINE crates/api/src/verify.rs:326  crate::speed::Outcome::Cancelled
GAP prod BRANCH crates/api/src/verify.rs:276:12 True  if let Some(why) = degraded {
GAP prod BRANCH crates/api/src/verify.rs:321:26 True  let outcome = if report
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:413:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:449:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:456:13 False  let Gate::RungFirst {
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:565:13 False  let Gate::RungFirst {
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:1440:30 False  [inside a closure whose false side does not fail the check]  .any(|fault| fault.contains("2025-05") && fault.contains("another writer holds")),
GAP test LINE crates/api/tests/logs_r3_24.rs:274  short.push(format!("k={k} {query}: {records} line(s), said {said:?}"));
GAP test LINE crates/api/tests/logs_r3_24.rs:355  None
GAP test LINE crates/api/tests/logs_r3_24.rs:399  None => short.push(format!("run {run} limit {limit}: no end in 200 pages")),
GAP test LINE crates/api/tests/logs_r3_24.rs:403  short.push(format!(
GAP test LINE crates/api/tests/logs_r3_24.rs:404  "run {run} limit {limit}: {} line(s), {} distinct, last said {said:?}",
GAP test LINE crates/api/tests/logs_r3_24.rs:405  seqs.len(),
GAP test LINE crates/api/tests/logs_r3_24.rs:406  distinct.len()
GAP test LINE crates/api/tests/logs_r3_24.rs:407  ));
GAP test BRANCH crates/api/tests/logs_r3_24.rs:273:16 True  if records != TOTAL {
GAP test BRANCH crates/api/tests/logs_r3_24.rs:402:24 True  if seqs.len() != BEFORE + AFTER || distinct.len() != BEFORE + AFTER {
GAP test BRANCH crates/api/tests/logs_r3_24.rs:402:56 True  if seqs.len() != BEFORE + AFTER || distinct.len() != BEFORE + AFTER {
GAP test LINE crates/api/tests/pool_load.rs:302  "the 10 ms target is measurable on this host now".to_owned()
GAP test LINE crates/api/tests/pool_load.rs:312  assert!(
GAP test LINE crates/api/tests/pool_load.rs:313  p99 < TARGET,
GAP test LINE crates/api/tests/pool_load.rs:454  Err(store::file::StoreError::Locked { .. }) => break None,
GAP test LINE crates/api/tests/pool_load.rs:455  Err(why) => panic!("the writer takes the month: {why}"),
GAP test LINE crates/api/tests/pool_load.rs:492  continue;
GAP test LINE crates/api/tests/pool_load.rs:498  continue;
GAP test LINE crates/api/tests/pool_load.rs:502  continue;
GAP test LINE crates/api/tests/pool_load.rs:606  continue;
GAP test LINE crates/api/tests/pool_load.rs:610  continue;
GAP test LINE crates/api/tests/pool_load.rs:617  continue;
GAP test BRANCH crates/api/tests/pool_load.rs:225:24 False  if get_blocking(addr, path).is_some() {
GAP test BRANCH crates/api/tests/pool_load.rs:301:16 True  if measurable {
GAP test BRANCH crates/api/tests/pool_load.rs:311:12 True  if measurable {
GAP test BRANCH crates/api/tests/pool_load.rs:365:23 False  !(answer.status == 200 || (answer.status == 429 && answer.retry_after))
GAP test BRANCH crates/api/tests/pool_load.rs:365:48 True  !(answer.status == 200 || (answer.status == 429 && answer.retry_after))
GAP test BRANCH crates/api/tests/pool_load.rs:365:48 False  !(answer.status == 200 || (answer.status == 429 && answer.retry_after))
GAP test BRANCH crates/api/tests/pool_load.rs:449:36 False  if !stop.load(Ordering::Acquire) =>
GAP test BRANCH crates/api/tests/pool_load.rs:458:25 False  let Some(mut file) = file else { break };
GAP test BRANCH crates/api/tests/pool_load.rs:491:33 False  let Ok(mut socket) = std::net::TcpStream::connect(addr) else {
GAP test BRANCH crates/api/tests/pool_load.rs:497:32 True  if socket.write_all(request.as_bytes()).is_err() {
GAP test BRANCH crates/api/tests/pool_load.rs:501:32 True  if socket.read_to_string(&mut answer).is_err() {
GAP test BRANCH crates/api/tests/pool_load.rs:536:31 False  *status == 503 && *retry && body.contains("being written, ask again")
GAP test BRANCH crates/api/tests/pool_load.rs:558:34 False  !(*status == 200 || (*status == 503 && *retry && body.contains("being written")))
GAP test BRANCH crates/api/tests/pool_load.rs:558:52 False  !(*status == 200 || (*status == 503 && *retry && body.contains("being written")))
GAP test BRANCH crates/api/tests/pool_load.rs:582:8 False  if seconds >= 3 {
GAP test BRANCH crates/api/tests/pool_load.rs:605:12 True  if socket.write_all(request.as_bytes()).await.is_err() {
GAP test BRANCH crates/api/tests/pool_load.rs:609:12 True  if socket.read_to_end(&mut answer).await.is_err() {
GAP test BRANCH crates/api/tests/pool_load.rs:616:13 False  let Some(status) = head.split(' ').nth(1).and_then(|code| code.parse().ok()) else {
GAP test LINE crates/api/tests/state_stream_emits.rs:173  return j;
GAP test LINE crates/api/tests/state_stream_emits.rs:180  j
GAP test LINE crates/api/tests/state_stream_emits.rs:237  return None;
GAP test LINE crates/api/tests/state_stream_emits.rs:249  'n' => out.push('\n'),
GAP test LINE crates/api/tests/state_stream_emits.rs:250  't' => out.push('\t'),
GAP test LINE crates/api/tests/state_stream_emits.rs:292  return None;
GAP test LINE crates/api/tests/state_stream_emits.rs:296  }
GAP test LINE crates/api/tests/state_stream_emits.rs:297  let mut values: Vec<String> = crate_texts
GAP test LINE crates/api/tests/state_stream_emits.rs:298  .iter()
GAP test LINE crates/api/tests/state_stream_emits.rs:299  .filter_map(|file| const_str(file, expr))
GAP test LINE crates/api/tests/state_stream_emits.rs:300  .collect();
GAP test LINE crates/api/tests/state_stream_emits.rs:301  values.sort();
GAP test LINE crates/api/tests/state_stream_emits.rs:302  values.dedup();
GAP test LINE crates/api/tests/state_stream_emits.rs:303  (values.len() == 1).then(|| values.remove(0))
GAP test LINE crates/api/tests/state_stream_emits.rs:328  args
GAP test LINE crates/api/tests/state_stream_emits.rs:359  continue;
GAP test LINE crates/api/tests/state_stream_emits.rs:368  _ => continue,
GAP test LINE crates/api/tests/state_stream_emits.rs:373  continue;
GAP test LINE crates/api/tests/state_stream_emits.rs:382  continue;
GAP test LINE crates/api/tests/state_stream_emits.rs:407  sources(&path, out);
GAP test LINE crates/api/tests/state_stream_emits.rs:412  }
GAP test BRANCH crates/api/tests/state_stream_emits.rs:65:19 False  while i < text.len() && text[i] != '\n' {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:72:19 False  while i < text.len() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:100:23 False  while j < text.len() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:113:19 False  while j < text.len() && text[j] != '"' {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:125:23 False  while j < text.len() && text[j] != '\'' {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:154:11 False  while j < text.len() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:159:24 False  ';' if depth == 0 => return j + 1,
GAP test BRANCH crates/api/tests/state_stream_emits.rs:160:24 False  '{' if depth == 0 => {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:162:27 False  while j < text.len() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:236:12 True  if c == '"' {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:270:15 False  while i < chars.len() && !mask[i] {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:274:16 False  if let Some(text) = literal(&run) {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:291:8 True  if !expr.chars().all(ident_char) {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:294:12 False  if let Some(value) = const_str(text, expr) {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:358:12 True  if chars.get(open) != Some(&'(') {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:372:16 True  if args.is_empty() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:381:13 False  let (Some(target), Some(message)) = (args.first(), args.get(1)) else {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:406:12 True  if path.is_dir() {
GAP test BRANCH crates/api/tests/state_stream_emits.rs:408:19 False  } else if path.extension().is_some_and(|ext| ext == "rs") {
GAP test LINE crates/api/tests/store_step_forms.rs:62  let parsed = api::ingest::parse_fno(body, today());
GAP test LINE crates/api/tests/store_step_forms.rs:63  assert!(parsed.is_ok(), "{body}: {parsed:?}");
`````

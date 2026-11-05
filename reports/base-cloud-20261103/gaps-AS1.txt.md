# gaps-AS1.txt.md

Session BASE-COV-API-AS1. One fenced block below: the figures, then every uncovered row of G1's files that is a GAP (production, test, or inside an #[ignore]d test) or a HOST row, each with its text. (F) and (U) rows are in `fu-AS1.txt.md`.

`````text
BASE-COV-API repair 3, shard AS1 (G1 of SHARDS.md), Route B on code tip 3f324ee88b6c24fff98aa59779ac164896a13a31, reading whole:api, set whole-api-set-4.txt
G1 files read 76 of 76: lines 55261/56448 (1187 uncovered), branches 4033/4890 (857 uncovered sides)
classified: GAP prod 821 lines 366 sides; GAP test 116 lines 112 sides; GAP ignored 37 lines 0 sides; HOST 64 lines 10 sides; (F) 125 lines 357 sides; (U) 24 lines 12 sides
HOST = covered on the Mac's run of START (the kit's lists hold no row for it) and not here: the test that reaches it is a host red of this 4-core root host or one of the two tests skipped here as 4-core hangs; owed to the Mac's Route B.
per file (lines a/b, branches c/d; GAP lines/sides):
  crates/api/src/aliases.rs: lines 374/375, branches 35/36; GAP 1/1
  crates/api/src/assets.rs: lines 833/841, branches 71/82; GAP 5/6
  crates/api/src/audit.rs: lines 3352/3376, branches 182/192; GAP 5/0
  crates/api/src/audit_json.rs: lines 1311/1338, branches 93/102; GAP 24/6
  crates/api/src/audit_json_commit_tests.rs: lines 208/208, branches 5/10; GAP 0/0
  crates/api/src/audit_lock_wait_measure.rs: lines 128/133, branches 8/8; GAP 5/0
  crates/api/src/audit_lock_wait_tests.rs: lines 496/514, branches 19/24; GAP 10/2
  crates/api/src/autopilot_stall_tests.rs: lines 257/258, branches 7/8; GAP 0/1
  crates/api/src/backtest.rs: lines 1163/1165, branches 44/50; GAP 1/4
  crates/api/src/bars.rs: lines 2734/2761, branches 284/314; GAP 22/24
  crates/api/src/bars_change.rs: lines 659/660, branches 99/104; GAP 1/1
  crates/api/src/bars_page_tests.rs: lines 372/376, branches 11/14; GAP 3/1
  crates/api/src/bars_sealed_tests.rs: lines 184/184, branches 2/2; GAP 0/0
  crates/api/src/basis_route.rs: lines 233/233, branches 30/30; GAP 0/0
  crates/api/src/basis_route_tests.rs: lines 646/656, branches 4/4; GAP 0/0
  crates/api/src/booleanoosjson.rs: lines 177/186, branches 24/30; GAP 9/6
  crates/api/src/booleanoosjson_tests.rs: lines 209/209, branches 6/8; GAP 0/0
  crates/api/src/booleansearchjson.rs: lines 123/240, branches 42/68; GAP 117/26
  crates/api/src/booleansearchjson_tests.rs: lines 110/110, branches 0/0; GAP 0/0
  crates/api/src/calendar.rs: lines 421/421, branches 24/24; GAP 0/0
  crates/api/src/calendar_build.rs: lines 463/473, branches 41/54; GAP 2/6
  crates/api/src/candidatejson.rs: lines 679/695, branches 71/80; GAP 16/9
  crates/api/src/catalog.rs: lines 659/659, branches 36/42; GAP 0/0
  crates/api/src/census.rs: lines 1561/1624, branches 171/242; GAP 61/62
  crates/api/src/census_epoch_link_tests.rs: lines 199/206, branches 5/8; GAP 1/1
  crates/api/src/census_warm_tests.rs: lines 1235/1250, branches 86/114; GAP 4/19
  crates/api/src/chain.rs: lines 647/678, branches 117/130; GAP 31/13
  crates/api/src/chain_route_tests.rs: lines 652/652, branches 4/8; GAP 0/0
  crates/api/src/cost_limits_tests.rs: lines 336/336, branches 38/76; GAP 0/0
  crates/api/src/coverage.rs: lines 670/670, branches 35/40; GAP 0/0
  crates/api/src/credential_law.rs: lines 178/181, branches 18/20; GAP 3/2
  crates/api/src/credential_law_tests.rs: lines 524/531, branches 23/42; GAP 2/3
  crates/api/src/dayflags_v2_tests.rs: lines 63/63, branches 4/4; GAP 0/0
  crates/api/src/detail.rs: lines 1312/1325, branches 109/122; GAP 2/7
  crates/api/src/emitted.rs: lines 1021/1021, branches 78/78; GAP 0/0
  crates/api/src/facets.rs: lines 1344/1422, branches 186/214; GAP 78/28
  crates/api/src/facets_tests.rs: lines 1308/1308, branches 20/28; GAP 0/0
  crates/api/src/feed_checks.rs: lines 789/818, branches 99/116; GAP 29/17
  crates/api/src/feed_checks_files.rs: lines 802/832, branches 105/114; GAP 20/5
  crates/api/src/feed_checks_tests.rs: lines 1078/1082, branches 33/50; GAP 1/0
  crates/api/src/folder.rs: lines 361/369, branches 17/20; GAP 8/0
  crates/api/src/gap_peer_route_tests.rs: lines 343/349, branches 1/2; GAP 5/1
  crates/api/src/hand_stop_tests.rs: lines 155/155, branches 0/0; GAP 0/0
  crates/api/src/import_journal.rs: lines 1102/1109, branches 63/70; GAP 7/1
  crates/api/src/imports_filter_tests.rs: lines 428/428, branches 7/8; GAP 0/0
  crates/api/src/index_consistency_projection.rs: lines 169/258, branches 2/12; GAP 89/10
  crates/api/src/indexmap.rs: lines 769/777, branches 48/58; GAP 8/6
  crates/api/src/indexstop_projection_tests.rs: lines 208/208, branches 4/4; GAP 0/0
  crates/api/src/indexstopvix_projection_tests.rs: lines 243/243, branches 12/12; GAP 0/0
  crates/api/src/isolated.rs: lines 56/56, branches 3/4; GAP 0/1
  crates/api/src/main.rs: lines 175/181, branches 1/2; GAP 6/1
  crates/api/src/merge.rs: lines 762/764, branches 62/74; GAP 0/0
  crates/api/src/optiongreeks_route_tests.rs: lines 644/647, branches 21/38; GAP 1/0
  crates/api/src/page_report_route.rs: lines 129/130, branches 23/24; GAP 1/1
  crates/api/src/page_report_route_tests.rs: lines 97/97, branches 1/2; GAP 0/0
  crates/api/src/pullrun.rs: lines 6743/6768, branches 457/530; GAP 3/6
  crates/api/src/pullrun_restart_tests.rs: lines 1047/1081, branches 27/54; GAP 6/1
  crates/api/src/recovery.rs: lines 2477/2766, branches 317/452; GAP 282/129
  crates/api/src/recovery_journal.rs: lines 1264/1277, branches 91/98; GAP 13/3
  crates/api/src/repair_route.rs: lines 555/557, branches 37/40; GAP 0/0
  crates/api/src/saved_response_boundary_tests.rs: lines 180/183, branches 1/2; GAP 3/1
  crates/api/src/scratch.rs: lines 107/107, branches 8/8; GAP 0/0
  crates/api/src/served_routes_tests.rs: lines 277/277, branches 4/4; GAP 0/0
  crates/api/src/server_cloud_mutant_tests_a.rs: lines 1352/1360, branches 39/50; GAP 5/5
  crates/api/src/settings_route.rs: lines 660/694, branches 70/80; GAP 24/9
  crates/api/src/settings_route_tests.rs: lines 759/773, branches 26/42; GAP 10/6
  crates/api/src/spot.rs: lines 408/409, branches 56/56; GAP 0/0
  crates/api/src/spot_route_tests.rs: lines 429/429, branches 8/16; GAP 0/0
  crates/api/src/stop_tests.rs: lines 416/436, branches 17/32; GAP 6/5
  crates/api/src/store_route.rs: lines 1701/1713, branches 134/156; GAP 12/7
  crates/api/src/sweeprun_accepted_tests.rs: lines 308/308, branches 10/10; GAP 0/0
  crates/api/src/sweeprun_admission_tests.rs: lines 157/157, branches 0/0; GAP 0/0
  crates/api/src/verification_route_tests.rs: lines 182/182, branches 0/0; GAP 0/0
  crates/api/tests/binary.rs: lines 100/100, branches 0/0; GAP 0/0
  crates/api/tests/pool_load.rs: lines 507/528, branches 48/70; GAP 11/16
  crates/api/tests/state_stream_emits.rs: lines 481/502, branches 149/168; GAP 21/19
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
GAP ignored LINE crates/api/src/audit.rs:5365  fn append_synced_cost_per_entry_is_measured() {
GAP ignored LINE crates/api/src/audit.rs:5366  appends_timed("audit-append-timing", 400, 250_000);
GAP ignored LINE crates/api/src/audit.rs:5367  }
GAP test LINE crates/api/src/audit.rs:5672  [START line 5589] page[0].whole_note
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
GAP prod LINE crates/api/src/audit_json.rs:1016  || std::sync::Arc::new(FeedRollup::default()),
GAP prod LINE crates/api/src/audit_json.rs:1041  || format!("no census row for {} in this build", feed.as_str()),
GAP prod LINE crates/api/src/audit_json.rs:1054  || render::json_string(""),
GAP prod LINE crates/api/src/audit_json.rs:1075  |day| render::json_string(&day.to_string())
GAP prod LINE crates/api/src/audit_json.rs:1249  Err(why) => Err(why.to_string()),
GAP prod LINE crates/api/src/audit_json.rs:1300  "no commit time can be stated"
GAP prod LINE crates/api/src/audit_json.rs:1345  Err(ref why) => Some(format!("{path} could not be stat'ed: {why}")),
GAP prod BRANCH crates/api/src/audit_json.rs:243:28 True  if asked.is_empty() || asked.parse::<usize>().is_ok() {
GAP prod BRANCH crates/api/src/audit_json.rs:388:12 False  if trouble.is_empty() {
GAP prod BRANCH crates/api/src/audit_json.rs:1068:27 False  if behind.kept && behind.unread.is_none() {
GAP prod BRANCH crates/api/src/audit_json.rs:1248:24 False  Err(absent) if absent.kind() == std::io::ErrorKind::NotFound => Ok(None),
GAP prod BRANCH crates/api/src/audit_json.rs:1297:20 False  if at.is_some() {
GAP test BRANCH crates/api/src/audit_json.rs:2008:26 True  .any(|s| s.contains("groww") || s == "Reference indices")
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:37  [START line 33] fn measure_append_latency_under_contention() {
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:38  [START line 34] let root = std::env::var("BRUTEX_AG06_MEASURE_ROOT")
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:39  [START line 35] .expect("BRUTEX_AG06_MEASURE_ROOT names a store root holding a CLONE of the journal");
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:41  [START line 37] }
GAP ignored LINE crates/api/src/audit_lock_wait_measure.rs:40  [inside the #[ignore]d real-data measurement measure_append_latency_under_contention (BRUTEX_AG06_MEASURE_ROOT): owed to the Mac, L-BCA-3] [a line of this change] let _measured = latency_over(Path::new(&root), 200);
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:105  return Vec::new();
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:214  member(7).image().to_vec()
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:215  })
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:323  closed("z").image().to_vec()
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:324  })
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:654  let path = PathBuf::from(path);
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:655  let held = Journal { path: path.clone() };
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:656  let _guard = hold(&held);
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:657  std::fs::write(path.with_extension("held"), b"1").expect("the marker");
GAP test LINE crates/api/src/audit_lock_wait_tests.rs:658  std::thread::sleep(Duration::from_secs(30));
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:104:8 True  if n == 0 {
GAP test BRANCH crates/api/src/audit_lock_wait_tests.rs:652:12 True  if let Ok(path) = std::env::var(CHILD_HOLDS) {
GAP test BRANCH crates/api/src/autopilot_stall_tests.rs:65:20 False  if let Some(next) = month_after(state.frontier) {
GAP test LINE crates/api/src/backtest.rs:2192  || status == axum::http::StatusCode::SERVICE_UNAVAILABLE
GAP prod BRANCH crates/api/src/backtest.rs:927:40 False  self.version == VERSION && self.refusal.is_none() && !self.partial_tail
GAP test BRANCH crates/api/src/backtest.rs:2191:13 False  status == axum::http::StatusCode::OK
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
GAP test LINE crates/api/src/bars_page_tests.rs:458  other => panic!("not a route here: {other}"),
GAP test LINE crates/api/src/bars_page_tests.rs:462  }
GAP test LINE crates/api/src/bars_page_tests.rs:463  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test BRANCH crates/api/src/bars_page_tests.rs:460:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
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
GAP prod BRANCH crates/api/src/calendar_build.rs:233:32 True  if let Some((_, answer)) = state.done.as_ref() {
GAP prod BRANCH crates/api/src/calendar_build.rs:254:10 False  (at.elapsed() <= PARKED_FOR && same_stamp).then(|| answer.clone())
GAP prod BRANCH crates/api/src/calendar_build.rs:427:16 False  && !build.finished()
GAP prod BRANCH crates/api/src/calendar_build.rs:732:12 True  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
GAP prod BRANCH crates/api/src/calendar_build.rs:732:44 True  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
GAP prod BRANCH crates/api/src/calendar_build.rs:732:44 False  if kept.len() >= MAX_KEPT_VOTES && !kept.contains_key(&key) {
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
GAP prod BRANCH crates/api/src/candidatejson.rs:135:21 False  Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
GAP prod BRANCH crates/api/src/candidatejson.rs:182:8 True  if asked.digest.is_some_and(|digest| digest != summary.digest) {
GAP prod BRANCH crates/api/src/candidatejson.rs:249:8 True  if observed.digest != summary.digest {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:8 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:27 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:268:48 True  if asked.tier != 0 || asked.offset != 0 || asked.rank.is_some() {
GAP prod BRANCH crates/api/src/candidatejson.rs:327:9 True  held.model != summary.model
GAP prod BRANCH crates/api/src/candidatejson.rs:330:16 True  || held.attempt != summary.attempt
GAP prod BRANCH crates/api/src/candidatejson.rs:331:16 True  || held.digest != summary.digest
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
GAP prod LINE crates/api/src/census.rs:1005  (Census::Absent, Census::Absent) => true,
GAP prod LINE crates/api/src/census.rs:1167  || "the single file".to_owned(),
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
GAP test BRANCH crates/api/src/census.rs:3193:13 True  rows == 0 || filled > 0,
GAP test LINE crates/api/src/census_epoch_link_tests.rs:173  }
GAP test BRANCH crates/api/src/census_epoch_link_tests.rs:166:16 False  if let Some(epoch) = entry
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
GAP prod LINE crates/api/src/chain.rs:510  _ => false,
GAP prod LINE crates/api/src/chain.rs:585  return;
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
GAP prod LINE crates/api/src/chain.rs:819  OpenRefusalKind::Invalid => return Err(refusal.message),
GAP prod LINE crates/api/src/chain.rs:831  }
GAP prod LINE crates/api/src/chain.rs:833  return Ok(Spot::Unreadable);
GAP prod LINE crates/api/src/chain.rs:837  faults.push(format!(
GAP prod LINE crates/api/src/chain.rs:839  file.path().display(),
GAP prod LINE crates/api/src/chain.rs:840  at.div_euclid(MICROS)
GAP prod LINE crates/api/src/chain.rs:842  Spot::Unreadable
GAP prod LINE crates/api/src/chain.rs:843  }),
GAP prod LINE crates/api/src/chain.rs:917  return bad_request("no census row for this feed");
GAP prod LINE crates/api/src/chain.rs:1006  Err(why) => return bad_request(&why),
GAP prod LINE crates/api/src/chain.rs:1056  Lineage::Unreadable(why) => ("unreadable", 0, 0, Some(why.as_str())),
GAP prod LINE crates/api/src/chain.rs:1164  continue;
GAP prod LINE crates/api/src/chain.rs:1205  Err(why) => not_admitted("a chain read", &why),
GAP prod BRANCH crates/api/src/chain.rs:530:17 True  pull::expiry_alias::AliasRecord::parse(line).is_some()
GAP prod BRANCH crates/api/src/chain.rs:584:13 False  let Some(days) = self.adjusted.get(&underlying) else {
GAP prod BRANCH crates/api/src/chain.rs:603:35 False  if !before && !names(renamed, key) {
GAP prod BRANCH crates/api/src/chain.rs:672:11 False  while month <= last {
GAP prod BRANCH crates/api/src/chain.rs:689:24 False  if at < file.header().n_valid {
GAP prod BRANCH crates/api/src/chain.rs:696:37 False  Ok(Some(ts)) if ts >= from => return Some(ts),
GAP prod BRANCH crates/api/src/chain.rs:776:12 False  if let Some(modified) = modified {
GAP prod BRANCH crates/api/src/chain.rs:803:12 False  if !self.opened.iter().any(|(held, _)| *held == month) {
GAP prod BRANCH crates/api/src/chain.rs:832:13 False  let Some((_, opened)) = self.opened.iter().find(|(held, _)| *held == month) else {
GAP prod BRANCH crates/api/src/chain.rs:916:9 False  let Some(census) = now.census() else {
GAP prod BRANCH crates/api/src/chain.rs:971:26 False  let leg = if call { "CE" } else { "PE" };
GAP prod BRANCH crates/api/src/chain.rs:1051:16 False  if index.unread > 0 { "partial" } else { "read" },
GAP prod BRANCH crates/api/src/chain.rs:1163:17 False  let Some(strike) = strikes.get(n) else {
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
GAP prod LINE crates/api/src/detail.rs:332  left.stay();
GAP prod LINE crates/api/src/detail.rs:333  return None;
GAP prod BRANCH crates/api/src/detail.rs:331:13 False  let Some(clock) = self.clock() else {
GAP prod BRANCH crates/api/src/detail.rs:1669:20 False  if held
GAP test BRANCH crates/api/src/detail.rs:2758:49 False  [START line 2743] while !cancel.is_cancelled() && std::time::Instant::now() < deadline {
GAP test BRANCH crates/api/src/detail.rs:2788:20 True  [START line 2773] while (pool.cancelled.load(Ordering::Relaxed) == before
GAP test BRANCH crates/api/src/detail.rs:2789:20 True  [START line 2774] || pool.active.load(Ordering::Acquire) != 0)
GAP test BRANCH crates/api/src/detail.rs:2790:20 True  [START line 2775] && std::time::Instant::now() < deadline
GAP test BRANCH crates/api/src/detail.rs:2790:20 False  [START line 2775] && std::time::Instant::now() < deadline
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
GAP prod LINE crates/api/src/feed_checks_files.rs:836  return Err(Refusal::Refused(format!(
GAP prod LINE crates/api/src/feed_checks_files.rs:837  "`{}` changed between two reads of it: a version file is never rewritten, so this \
GAP prod LINE crates/api/src/feed_checks_files.rs:838  one is refused",
GAP prod LINE crates/api/src/feed_checks_files.rs:839  file.display()
GAP prod LINE crates/api/src/feed_checks_files.rs:840  )));
GAP prod LINE crates/api/src/feed_checks_files.rs:1128  return Err("an empty `symbol`".to_owned());
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
GAP prod BRANCH crates/api/src/feed_checks_files.rs:835:8 True  if brutex_core::blake3::hash(&bytes) != digest {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1127:8 True  if symbol.is_empty() {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1290:9 False  let Some((_, (_, header))) = lines.next() else {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1293:8 True  if header != cli::tick_second_check::HEADER {
GAP prod BRANCH crates/api/src/feed_checks_files.rs:1312:12 True  if cells.len() != width {
GAP test LINE crates/api/src/feed_checks_tests.rs:900  Err(why) => assert_eq!(year["refused"], why.to_string()),
GAP prod LINE crates/api/src/folder.rs:125  Err(why) => return (axum::http::StatusCode::BAD_REQUEST, head, refused(&why)),
GAP prod LINE crates/api/src/folder.rs:138  Err(why) => {
GAP prod LINE crates/api/src/folder.rs:139  return (
GAP prod LINE crates/api/src/folder.rs:140  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/folder.rs:141  head,
GAP prod LINE crates/api/src/folder.rs:142  refused(&why.to_string()),
GAP prod LINE crates/api/src/folder.rs:143  );
GAP prod LINE crates/api/src/folder.rs:151  Ok(answered) => answered,
GAP test LINE crates/api/src/gap_peer_route_tests.rs:43  matches!(
GAP test LINE crates/api/src/gap_peer_route_tests.rs:393  }
GAP test LINE crates/api/src/gap_peer_route_tests.rs:394  assert_eq!(status, axum::http::StatusCode::ACCEPTED, "{answer}");
GAP test LINE crates/api/src/gap_peer_route_tests.rs:395  assert_eq!(answer["status"], "building", "{answer}");
GAP test LINE crates/api/src/gap_peer_route_tests.rs:396  tokio::time::sleep(std::time::Duration::from_millis(10)).await;
GAP test BRANCH crates/api/src/gap_peer_route_tests.rs:391:16 False  if status == axum::http::StatusCode::OK {
GAP prod LINE crates/api/src/import_journal.rs:429  Err(_) => self.runs_unindexed = self.runs_unindexed.saturating_add(1),
GAP ignored LINE crates/api/src/import_journal.rs:848  fn end_line_fold_timing() {
GAP ignored LINE crates/api/src/import_journal.rs:849  end_lines_folded_beside(100_000);
GAP ignored LINE crates/api/src/import_journal.rs:850  }
GAP ignored LINE crates/api/src/import_journal.rs:1162  [START line 1091] fn end_line_fold_cost_by_field_count() {
GAP ignored LINE crates/api/src/import_journal.rs:1163  [START line 1092] end_line_fold_at_widths(2_000);
GAP ignored LINE crates/api/src/import_journal.rs:1164  [START line 1093] }
GAP test BRANCH crates/api/src/import_journal.rs:1617:25 False  [START line 1498] .find(|row| row.kind == kind && row.filter == "*")
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
GAP test BRANCH crates/api/src/isolated.rs:118:15 False  let ran = output.status.success() && stdout.contains("1 passed");
GAP prod LINE crates/api/src/main.rs:44  Err(why) => {
GAP prod LINE crates/api/src/main.rs:45  eprintln!("FAILED: the async runtime could not start: {why}");
GAP prod LINE crates/api/src/main.rs:46  note_exit(api::server::FAILED, count, false);
GAP prod LINE crates/api/src/main.rs:47  return std::process::ExitCode::from(api::server::FAILED);
GAP prod LINE crates/api/src/main.rs:60  let _inside = runtime.enter();
GAP prod LINE crates/api/src/main.rs:61  Box::pin(api::speed::stop_asked())
GAP prod BRANCH crates/api/src/main.rs:59:22 True  Ok(words) if api::server::catches_stops(words) => {
GAP test LINE crates/api/src/optiongreeks_route_tests.rs:479  other => format!("not unreadable: {other:?}"),
GAP prod LINE crates/api/src/page_report_route.rs:172  WRITTEN.load(Ordering::Relaxed)
GAP prod BRANCH crates/api/src/page_report_route.rs:169:28 False  let written_total = if written {
GAP prod LINE crates/api/src/pullrun.rs:1726  continue;
GAP prod LINE crates/api/src/pullrun.rs:2093  break;
GAP test LINE crates/api/src/pullrun.rs:9754  [START line 9737] '+' => out.push_str("%2B"),
GAP prod BRANCH crates/api/src/pullrun.rs:1725:13 False  let Some((_, made)) = read.get(*at) else {
GAP prod BRANCH crates/api/src/pullrun.rs:2092:12 True  if page.is_empty() {
GAP prod BRANCH crates/api/src/pullrun.rs:4004:20 False  if let Some(feed) = progress.feeds.get_mut(nth) {
GAP prod BRANCH crates/api/src/pullrun.rs:4044:24 False  if let Some(feed) = progress.feeds.get_mut(nth) {
GAP prod BRANCH crates/api/src/pullrun.rs:4049:16 False  if let Some(held) = outcomes.get_mut(nth) {
GAP test BRANCH crates/api/src/pullrun.rs:4740:53 True  [START line 4742] if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1417  [START line 1415] fn restart_cost_with_64_open_runs() {
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1418  [START line 1416] costs_with_64_open_runs(8_001, 200);
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1419  [START line 1417] }
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1565  [START line 1513] fn restart_cost_of_the_walks_worst_case() {
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1566  [START line 1514] costs_of_the_walks_worst_case(200);
GAP ignored LINE crates/api/src/pullrun_restart_tests.rs:1567  [START line 1515] }
GAP test BRANCH crates/api/src/pullrun_restart_tests.rs:364:25 False  [START line 362] .any(|line| crate::emitted::counts(line, "unnamed", 6)
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
GAP prod LINE crates/api/src/recovery.rs:1411  return Err(
GAP prod LINE crates/api/src/recovery.rs:1412  "source timestamp belongs to a different instrument-month window".to_owned(),
GAP prod LINE crates/api/src/recovery.rs:1413  );
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
GAP prod LINE crates/api/src/recovery.rs:1764  break;
GAP prod LINE crates/api/src/recovery.rs:1775  0
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
GAP prod BRANCH crates/api/src/recovery.rs:1410:16 True  if day_of(bar.ts_micros)?.year_month().map_err(failure)? != month {
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
GAP prod BRANCH crates/api/src/recovery.rs:1634:12 False  if *day >= window.from() && *day <= window.to() && inactive(symbol, *day) {
GAP prod BRANCH crates/api/src/recovery.rs:1656:28 False  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1656:53 False  .any(|day| *day >= window.from() && *day <= window.to() && *day < listed);
GAP prod BRANCH crates/api/src/recovery.rs:1665:35 True  && (found.held > 0 || found.missing > 0)
GAP prod BRANCH crates/api/src/recovery.rs:1685:40 True  if daily.contains(&day) || minutes.contains(&day) {
GAP prod BRANCH crates/api/src/recovery.rs:1744:11 False  while item.attempts < ATTEMPT_LIMIT {
GAP prod BRANCH crates/api/src/recovery.rs:1763:12 True  if !reserve(&mut item) {
GAP prod BRANCH crates/api/src/recovery.rs:1772:29 False  item.unchanged = if run.total.bars_committed == 0 {
GAP prod BRANCH crates/api/src/recovery.rs:1780:24 True  .map_or(if run.reached > 0 { 200 } else { 502 }, |block| {
GAP prod BRANCH crates/api/src/recovery.rs:1783:12 True  if run.credential_dead || run.blocked.is_some() || run.stopped.is_some() {
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
GAP test BRANCH crates/api/src/recovery.rs:2500:34 False  .any(|event| crate::emitted::says(event, "plan", &hex(id))
GAP test BRANCH crates/api/src/recovery.rs:2557:13 False  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2557:47 True  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2557:47 False  why.contains("trailing bytes") || why.contains("recovery"),
GAP test BRANCH crates/api/src/recovery.rs:2598:30 False  .any(|event| crate::emitted::says(event, "symbol", "NIFTY")
GAP test BRANCH crates/api/src/recovery.rs:2621:30 False  .any(|event| crate::emitted::says(event, "plan", &hex(id))
GAP test BRANCH crates/api/src/recovery.rs:2664:16 True  if let Err(why) = &ran {
GAP test BRANCH crates/api/src/recovery.rs:3413:19 True  while site.run.lock().unwrap().as_ref().unwrap().running() {
GAP prod LINE crates/api/src/recovery_journal.rs:192  )?;
GAP prod LINE crates/api/src/recovery_journal.rs:324  TryLockError::Error(error) => error,
GAP prod LINE crates/api/src/recovery_journal.rs:339  return Err(invalid_data(
GAP prod LINE crates/api/src/recovery_journal.rs:340  "recovery journal length changed during locked replay",
GAP prod LINE crates/api/src/recovery_journal.rs:341  ));
GAP prod LINE crates/api/src/recovery_journal.rs:386  io::Error::other("recovery journal length cannot fit its next record")
GAP prod LINE crates/api/src/recovery_journal.rs:387  })?;
GAP prod LINE crates/api/src/recovery_journal.rs:454  io::Error::other("recovery journal length cannot fit its next records")
GAP prod LINE crates/api/src/recovery_journal.rs:455  })?;
GAP prod LINE crates/api/src/recovery_journal.rs:670  TryLockError::Error(error) => error,
GAP test LINE crates/api/src/recovery_journal.rs:1538  fn flush(&mut self) -> io::Result<()> {
GAP test LINE crates/api/src/recovery_journal.rs:1539  Ok(())
GAP test LINE crates/api/src/recovery_journal.rs:1540  }
GAP prod BRANCH crates/api/src/recovery_journal.rs:338:12 True  if file.metadata()?.len() != bytes {
GAP prod BRANCH crates/api/src/recovery_journal.rs:521:16 True  if !metadata.is_file() || metadata.dev() != *device || metadata.ino() != *inode {
GAP prod BRANCH crates/api/src/recovery_journal.rs:521:39 True  if !metadata.is_file() || metadata.dev() != *device || metadata.ino() != *inode {
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:125  return Err(format!(
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:126  "{name} queued an invalid query instead of refusing immediately"
GAP test LINE crates/api/src/saved_response_boundary_tests.rs:127  ));
GAP test BRANCH crates/api/src/saved_response_boundary_tests.rs:124:9 False  let Poll::Ready(reply) = reply else {
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
GAP prod LINE crates/api/src/settings_route.rs:157  Err(why) => Snapshot {
GAP prod LINE crates/api/src/settings_route.rs:158  highest: 0,
GAP prod LINE crates/api/src/settings_route.rs:159  newest: None,
GAP prod LINE crates/api/src/settings_route.rs:160  faults: Vec::new(),
GAP prod LINE crates/api/src/settings_route.rs:161  stopped: Some(format!("the settings read did not finish: {why}")),
GAP prod LINE crates/api/src/settings_route.rs:162  holes: None,
GAP prod LINE crates/api/src/settings_route.rs:163  },
GAP prod LINE crates/api/src/settings_route.rs:213  None => (None, "no window is recorded in this build".to_owned()),
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
GAP test BRANCH crates/api/src/settings_route_tests.rs:128:24 False  .any(|row| row.label == "POST /settings" && row.response_status == 200),
GAP test BRANCH crates/api/src/settings_route_tests.rs:138:24 False  .any(|row| row.label == "POST /settings" && row.response_status == 409),
GAP test BRANCH crates/api/src/settings_route_tests.rs:398:22 True  Ok(value) if !value.trim().is_empty() => {
GAP test BRANCH crates/api/src/settings_route_tests.rs:398:22 False  Ok(value) if !value.trim().is_empty() => {
GAP test BRANCH crates/api/src/settings_route_tests.rs:860:20 True  if started.elapsed() > std::time::Duration::from_secs(10) {
GAP test BRANCH crates/api/src/settings_route_tests.rs:894:9 False  let Ok(entries) = std::fs::read_dir(dir) else {
GAP test LINE crates/api/src/stop_tests.rs:270  fn append(&mut self, _bytes: &[u8]) -> std::io::Result<()> {
GAP test LINE crates/api/src/stop_tests.rs:271  Ok(())
GAP test LINE crates/api/src/stop_tests.rs:272  }
GAP test LINE crates/api/src/stop_tests.rs:282  fn reopen(&mut self, _path: &Path) -> std::io::Result<()> {
GAP test LINE crates/api/src/stop_tests.rs:283  Ok(())
GAP test LINE crates/api/src/stop_tests.rs:284  }
GAP test BRANCH crates/api/src/stop_tests.rs:143:26 False  .any(|(name, n)| name == route && *n == 1)
GAP test BRANCH crates/api/src/stop_tests.rs:163:27 False  .any(|record| field_of(record, "drain_ms") == 200
GAP test BRANCH crates/api/src/stop_tests.rs:174:13 False  crate::emitted::says(record, "routes", &format!("{route}=1"))
GAP test BRANCH crates/api/src/stop_tests.rs:423:34 False  said.iter().any(|record| field_of(record, "grace_ms") == 100
GAP test BRANCH crates/api/src/stop_tests.rs:597:9 False  let Loading::Failed(why) = loaded_or_stopped(
GAP prod LINE crates/api/src/store_route.rs:914  Err(why) => {
GAP prod LINE crates/api/src/store_route.rs:915  return (
GAP prod LINE crates/api/src/store_route.rs:916  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/store_route.rs:917  headers,
GAP prod LINE crates/api/src/store_route.rs:918  serde_json::json!({"error":why}).to_string().into(),
GAP prod LINE crates/api/src/store_route.rs:919  );
GAP prod LINE crates/api/src/store_route.rs:1165  }
GAP prod LINE crates/api/src/store_route.rs:1245  census::Census::Absent | census::Census::Unreadable { .. } => None,
GAP prod LINE crates/api/src/store_route.rs:1297  Err(why) => (
GAP prod LINE crates/api/src/store_route.rs:1298  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/store_route.rs:1299  serde_json::json!({"error":why}).to_string(),
GAP prod LINE crates/api/src/store_route.rs:1300  ),
GAP prod BRANCH crates/api/src/store_route.rs:447:12 False  if let Ok(held) = cache.0.lock()
GAP prod BRANCH crates/api/src/store_route.rs:455:12 False  if let Ok(mut held) = cache.0.lock() {
GAP prod BRANCH crates/api/src/store_route.rs:482:34 True  if !refused_by_census || counted == Some(row.day) {
GAP prod BRANCH crates/api/src/store_route.rs:887:29 False  if !since.is_empty() && code == axum::http::StatusCode::OK {
GAP prod BRANCH crates/api/src/store_route.rs:928:12 False  if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
GAP prod BRANCH crates/api/src/store_route.rs:1069:20 False  && parts.iter().all(|part| {
GAP prod BRANCH crates/api/src/store_route.rs:1157:20 False  if let Some(next) = month_after(key.month) {
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
HOST LINE crates/api/src/audit.rs:3129  [START line 3129, covered on the Mac's run] Err(e) => return Err(format!("{}: {e}", self.path.display())),
HOST LINE crates/api/src/audit.rs:3329  [START line 3329, covered on the Mac's run] Err(ref why) => telemetry::Event::error("api.audit", "journal index not written")
HOST LINE crates/api/src/audit.rs:3330  [START line 3330, covered on the Mac's run] .with("why", telemetry::Value::Str(why)),
HOST LINE crates/api/src/audit.rs:5899  [START line 5816, covered on the Mac's run] assert!(why.contains("ermission denied"), "{why}");
HOST LINE crates/api/src/audit.rs:5900  [START line 5817, covered on the Mac's run] let logged = crate::emitted::landed(from, "api.audit", "journal index not written")
HOST LINE crates/api/src/audit.rs:5901  [START line 5818, covered on the Mac's run] .into_iter()
HOST LINE crates/api/src/audit.rs:5902  [START line 5819, covered on the Mac's run] .filter(|record| crate::emitted::says(record, "path", &shown))
HOST LINE crates/api/src/audit.rs:5903  [START line 5820, covered on the Mac's run] .collect::<Vec<_>>();
HOST LINE crates/api/src/audit.rs:5904  [START line 5821, covered on the Mac's run] assert_eq!(logged.len(), 1, "{logged:?}");
HOST LINE crates/api/src/audit.rs:5905  [START line 5822, covered on the Mac's run] assert!(crate::emitted::says(&logged[0], "why", "ermission denied"));
HOST LINE crates/api/src/audit.rs:5906  [START line 5823, covered on the Mac's run] assert_eq!(logged[0].level, telemetry::Level::Error);
HOST LINE crates/api/src/audit.rs:5907  [START line 5824, covered on the Mac's run] }
HOST BRANCH crates/api/src/audit.rs:3126:23 False  [START line 3126, covered on the Mac's run] Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
HOST BRANCH crates/api/src/audit.rs:6161:13 False  [START line 6078, covered on the Mac's run] let [entry] = page.as_slice() else {
HOST LINE crates/api/src/audit_lock_wait_tests.rs:313  [START line 313, covered on the Mac's run] let _ = std::fs::remove_dir_all(&root);
HOST LINE crates/api/src/audit_lock_wait_tests.rs:314  [START line 314, covered on the Mac's run] }
HOST LINE crates/api/src/audit_lock_wait_tests.rs:389  [START line 389, covered on the Mac's run] assert!(
HOST LINE crates/api/src/audit_lock_wait_tests.rs:390  [START line 390, covered on the Mac's run] why.contains(&format!(
HOST LINE crates/api/src/audit_lock_wait_tests.rs:391  [START line 391, covered on the Mac's run] "{} ms bound (APPEND_LOCK_WAIT)",
HOST LINE crates/api/src/audit_lock_wait_tests.rs:392  [START line 392, covered on the Mac's run] bound.as_millis()
HOST LINE crates/api/src/audit_lock_wait_tests.rs:393  [START line 393, covered on the Mac's run] )),
HOST LINE crates/api/src/basis_route_tests.rs:491  [START line 491, covered on the Mac's run] let refused = tables(&store);
HOST LINE crates/api/src/basis_route_tests.rs:492  [START line 492, covered on the Mac's run] assert!(refused.actions.table().is_err(), "unreadable: refused");
HOST LINE crates/api/src/basis_route_tests.rs:493  [START line 493, covered on the Mac's run] mode(0o644);
HOST LINE crates/api/src/basis_route_tests.rs:494  [START line 494, covered on the Mac's run] let repaired = tables(&store);
HOST LINE crates/api/src/basis_route_tests.rs:495  [START line 495, covered on the Mac's run] assert_eq!(
HOST LINE crates/api/src/basis_route_tests.rs:496  [START line 496, covered on the Mac's run] repaired.actions.receipt().map(|held| held.generation),
HOST LINE crates/api/src/basis_route_tests.rs:501  [START line 501, covered on the Mac's run] }
HOST BRANCH crates/api/src/census_warm_tests.rs:1199:72 False  [START line 1199, covered on the Mac's run] if freshness == Freshness::Current && generation == was + 1 && facets == vendor_generation {
HOST BRANCH crates/api/src/census_warm_tests.rs:1247:24 False  [START line 1247, covered on the Mac's run] && crate::emitted::says(record, "feed", "gdfl")
HOST BRANCH crates/api/src/census_warm_tests.rs:1626:67 False  [START line 1626, covered on the Mac's run] if code == 503 && body["error"] == "census_unreadable" && facets == WarmState::Failed {
HOST LINE crates/api/src/pullrun.rs:2090  [START line 2090, covered on the Mac's run] Err(why) => return not_known(format!("the pull journal's newest records: {why}")),
HOST LINE crates/api/src/pullrun.rs:9530  [START line 9513, covered on the Mac's run] }
HOST LINE crates/api/src/pullrun_restart_tests.rs:1321  [START line 1319, covered on the Mac's run] let root = fresh_root("torn");
HOST LINE crates/api/src/pullrun_restart_tests.rs:1322  [START line 1320, covered on the Mac's run] let journal = journal_at(&root, &[opened(100, Scope::Spot, "ag01-torn", FROM, TO, 1)]);
HOST LINE crates/api/src/pullrun_restart_tests.rs:1323  [START line 1321, covered on the Mac's run] let file = crate::audit::journal_path(&root);
HOST LINE crates/api/src/pullrun_restart_tests.rs:1324  [START line 1322, covered on the Mac's run] let close = closed(120, Scope::Spot, Outcome::Stored, "ag01-torn", FROM, TO).image();
HOST LINE crates/api/src/pullrun_restart_tests.rs:1325  [START line 1323, covered on the Mac's run] let mut bytes = std::fs::read(&file).expect("read");
HOST LINE crates/api/src/pullrun_restart_tests.rs:1326  [START line 1324, covered on the Mac's run] bytes.extend_from_slice(&close[..crate::audit::RECORD_LEN / 2]);
HOST LINE crates/api/src/pullrun_restart_tests.rs:1327  [START line 1325, covered on the Mac's run] std::fs::write(&file, bytes).expect("write");
HOST LINE crates/api/src/pullrun_restart_tests.rs:1328  [START line 1326, covered on the Mac's run] let run = restart_scan(&journal)
HOST LINE crates/api/src/pullrun_restart_tests.rs:1329  [START line 1327, covered on the Mac's run] .interrupted()
HOST LINE crates/api/src/pullrun_restart_tests.rs:1330  [START line 1328, covered on the Mac's run] .cloned()
HOST LINE crates/api/src/pullrun_restart_tests.rs:1331  [START line 1329, covered on the Mac's run] .expect("the close never landed whole");
HOST LINE crates/api/src/pullrun_restart_tests.rs:1332  [START line 1330, covered on the Mac's run] assert_eq!((run.ordinal, run.source.as_str()), (0, "ag01-torn"));
HOST LINE crates/api/src/pullrun_restart_tests.rs:1334  [START line 1332, covered on the Mac's run] let mut records = vec![opened(100, Scope::Spot, "ag01-801", FROM, TO, 1)];
HOST LINE crates/api/src/pullrun_restart_tests.rs:1335  [START line 1333, covered on the Mac's run] records.extend((0..INTERRUPTED_SCAN).map(|offset| failure(200 + offset)));
HOST LINE crates/api/src/pullrun_restart_tests.rs:1336  [START line 1334, covered on the Mac's run] let journal = journal_of("record-801", &records);
HOST LINE crates/api/src/pullrun_restart_tests.rs:1337  [START line 1335, covered on the Mac's run] let run = restart_scan(&journal)
HOST LINE crates/api/src/pullrun_restart_tests.rs:1338  [START line 1336, covered on the Mac's run] .interrupted()
HOST LINE crates/api/src/pullrun_restart_tests.rs:1339  [START line 1337, covered on the Mac's run] .cloned()
HOST LINE crates/api/src/pullrun_restart_tests.rs:1340  [START line 1338, covered on the Mac's run] .expect("found through the index");
HOST LINE crates/api/src/pullrun_restart_tests.rs:1341  [START line 1339, covered on the Mac's run] assert_eq!(
HOST LINE crates/api/src/pullrun_restart_tests.rs:1342  [START line 1340, covered on the Mac's run] (run.ordinal, run.failures, run.open_count, run.runs_unread),
HOST LINE crates/api/src/pullrun_restart_tests.rs:1345  [START line 1343, covered on the Mac's run] }
HOST BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:971:13 False  [START line 971, covered on the Mac's run] record.level == telemetry::Level::Error
HOST BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:1010:13 False  [START line 1010, covered on the Mac's run] record.level == telemetry::Level::Warn && crate::emitted::says(record, "why", &head)
HOST LINE crates/api/src/settings_route.rs:250  [START line 250, covered on the Mac's run] return Some(stopped.clone());
HOST BRANCH crates/api/src/settings_route.rs:249:12 True  [START line 249, covered on the Mac's run] if let Some(stopped) = &snapshot.stopped {
HOST LINE crates/api/src/spot.rs:722  [START line 722, covered on the Mac's run] Err(why) => not_admitted("a spot read", &why),
HOST LINE crates/api/src/stop_tests.rs:541  [START line 541, covered on the Mac's run] assert!(
HOST LINE crates/api/src/stop_tests.rs:542  [START line 542, covered on the Mac's run] whole >= SHUTDOWN_GRACE,
HOST LINE crates/api/src/stop_tests.rs:545  [START line 545, covered on the Mac's run] assert!(
HOST LINE crates/api/src/stop_tests.rs:546  [START line 546, covered on the Mac's run] said_last,
HOST LINE crates/api/src/stop_tests.rs:549  [START line 549, covered on the Mac's run] assert!(
HOST LINE crates/api/src/stop_tests.rs:550  [START line 550, covered on the Mac's run] !store_is_free(&root),
HOST LINE crates/api/src/stop_tests.rs:553  [START line 553, covered on the Mac's run] assert!(!port_is_free(addr), "and so does the port");
HOST LINE crates/api/src/stop_tests.rs:554  [START line 554, covered on the Mac's run] assert!(log_is_free(&root), "the log is closed");
HOST LINE crates/api/src/stop_tests.rs:555  [START line 555, covered on the Mac's run] drop(finish);
HOST LINE crates/api/src/stop_tests.rs:556  [START line 556, covered on the Mac's run] }
HOST LINE crates/api/tests/pool_load.rs:601  [START line 601, covered on the Mac's run] tokio::time::sleep(Duration::from_millis(5)).await;
HOST LINE crates/api/tests/pool_load.rs:602  [START line 602, covered on the Mac's run] continue;
HOST BRANCH crates/api/tests/pool_load.rs:347:36 False  [START line 347, covered on the Mac's run] if let Some(answer) = get_blocking(addr, WINDOW) {
HOST BRANCH crates/api/tests/pool_load.rs:600:13 False  [START line 600, covered on the Mac's run] let Ok(mut socket) = tokio::net::TcpStream::connect(addr).await else {
`````

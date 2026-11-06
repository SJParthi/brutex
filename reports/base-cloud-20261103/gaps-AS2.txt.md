# gaps-AS2.txt.md

Source: this shard's Route B whole:api readout of tip `c7da9149` (`d3-route-b-cloud.sh`, the kit's set file, this Linux host), classified as its first lines say. One fenced block below holds it.

`````text
BASE-COV-API-AS2 (repair round 3, shard G2), Route B on tip c7da9149876743b0a6c3363de8e6207e069588f0, whole:api (the kit's set file deep-BASE-COV-API-6ea46e59c074-set.txt, 205 coverage files), host Linux x86_64, root, 4 cores; test api rc=101 END 2026-11-06 09:48:19 IST (5 min); tests passed 3110 failed 14 ignored 16
NOT PROVED lines of the run: 23 files, every one of them crates/api/benches/** or crates/api/src/lib.rs (no object holds them: L-BCA-2)
TOTAL (whole api, this host) lines 173618/178248 (4630 uncovered), branches 12686/15364 (2678 uncovered sides)
G2 (58 files of SHARDS.md's G2) lines 63723/65137 (1414 uncovered), branches 4499/5376 (877 uncovered sides): (F) 139 lines 459 sides; (U) 33 lines 18 sides; GAP 1242 lines 400 sides (production 1002 lines 300 sides; test code 130 lines 88 sides; inside #[ignore]d tests 110 lines 12 sides)
Classes: a row on a line unchanged since START keeps repair r2's hand-read class (base-cov-api-gaps-r2.txt, base-cov-api-fu.txt); (U) rows carry their proof (fu-AS2.txt); every other row was classed by recipe v4 step 4 as r2 read it and read by hand in this session. Other groups' files were measured in the same run and are their shards' to list.
per file (lines a/b, branches c/d; GAP lines/sides):
  crates/api/src/bars_sort.rs: lines 3060/3107, branches 420/470; GAP 46/41
  crates/api/src/bars_sort_tests.rs: lines 1263/1266, branches 73/112; GAP 0/0
  crates/api/src/bars_unknown_series_tests.rs: lines 516/516, branches 2/2; GAP 0/0
  crates/api/src/boolean_observation_budget.rs: lines 79/79, branches 17/22; GAP 0/0
  crates/api/src/boolean_search_budget.rs: lines 47/47, branches 8/8; GAP 0/0
  crates/api/src/booleancampaignjson.rs: lines 102/111, branches 16/24; GAP 9/8
  crates/api/src/booleancampaignjson_tests.rs: lines 318/318, branches 2/2; GAP 0/0
  crates/api/src/booleanjson.rs: lines 305/326, branches 39/42; GAP 21/3
  crates/api/src/booleanjson_tests.rs: lines 428/428, branches 10/12; GAP 0/0
  crates/api/src/booleanlaunch.rs: lines 376/393, branches 85/88; GAP 17/2
  crates/api/src/booleanlaunch_tests.rs: lines 710/710, branches 18/22; GAP 0/0
  crates/api/src/booleanqualification_projection.rs: lines 44/143, branches 6/12; GAP 99/6
  crates/api/src/booleanqualification_projection_tests.rs: lines 71/71, branches 0/0; GAP 0/0
  crates/api/src/calendar_of.rs: lines 1647/1702, branches 103/128; GAP 52/20
  crates/api/src/calendar_record.rs: lines 654/669, branches 82/90; GAP 13/5
  crates/api/src/calendar_record_tests.rs: lines 469/471, branches 18/24; GAP 0/1
  crates/api/src/calendar_route.rs: lines 772/813, branches 86/94; GAP 41/8
  crates/api/src/calendar_route_tests.rs: lines 730/735, branches 24/46; GAP 4/1
  crates/api/src/census_cache.rs: lines 1028/1100, branches 110/138; GAP 72/28
  crates/api/src/census_tail_tests.rs: lines 376/378, branches 4/8; GAP 0/0
  crates/api/src/constituents.rs: lines 875/875, branches 54/62; GAP 0/2
  crates/api/src/dayflags_rules.rs: lines 940/941, branches 186/188; GAP 1/1
  crates/api/src/dayflags_rules_tests.rs: lines 3657/3706, branches 53/82; GAP 19/9
  crates/api/src/disk_route.rs: lines 1197/1224, branches 134/166; GAP 25/19
  crates/api/src/disk_route_seal_tests.rs: lines 495/495, branches 10/14; GAP 0/0
  crates/api/src/expressionsearchjson.rs: lines 148/181, branches 18/36; GAP 33/18
  crates/api/src/fault_log.rs: lines 403/403, branches 39/40; GAP 0/0
  crates/api/src/fno_boundary_tests.rs: lines 1239/1241, branches 26/28; GAP 0/0
  crates/api/src/head_deadline_body_tests.rs: lines 635/695, branches 35/60; GAP 52/7
  crates/api/src/indexstoprankingjson.rs: lines 308/502, branches 24/48; GAP 194/21
  crates/api/src/indexstopvixjson.rs: lines 264/275, branches 22/26; GAP 11/4
  crates/api/src/instruments_route.rs: lines 775/775, branches 43/58; GAP 0/1
  crates/api/src/livejson.rs: lines 405/407, branches 22/36; GAP 0/0
  crates/api/src/master.rs: lines 654/661, branches 43/58; GAP 5/1
  crates/api/src/operation_audit.rs: lines 212/223, branches 45/46; GAP 11/1
  crates/api/src/operation_audit_tests.rs: lines 634/636, branches 6/8; GAP 0/1
  crates/api/src/provenance.rs: lines 1109/1170, branches 136/164; GAP 61/28
  crates/api/src/provenance_tests.rs: lines 1414/1416, branches 5/6; GAP 0/0
  crates/api/src/quarantine_route.rs: lines 129/130, branches 12/12; GAP 1/0
  crates/api/src/quarantine_route_tests.rs: lines 503/507, branches 13/14; GAP 2/1
  crates/api/src/recovery_control.rs: lines 387/388, branches 26/26; GAP 1/0
  crates/api/src/render.rs: lines 2654/2661, branches 245/278; GAP 1/1
  crates/api/src/research_doors_tests.rs: lines 96/97, branches 2/2; GAP 0/0
  crates/api/src/scrub_route.rs: lines 1590/1597, branches 175/186; GAP 7/11
  crates/api/src/server.rs: lines 18669/19099, branches 1236/1512; GAP 362/95
  crates/api/src/server_cloud_mutant_tests_e.rs: lines 585/587, branches 11/12; GAP 0/0
  crates/api/src/sources.rs: lines 2719/2766, branches 500/542; GAP 42/38
  crates/api/src/sources_day_truth_tests.rs: lines 490/490, branches 4/4; GAP 0/0
  crates/api/src/sources_tests.rs: lines 2007/2009, branches 10/14; GAP 0/1
  crates/api/src/speed.rs: lines 536/557, branches 74/82; GAP 20/5
  crates/api/src/speed_tests.rs: lines 766/767, branches 32/60; GAP 0/0
  crates/api/src/store_wire.rs: lines 427/431, branches 18/18; GAP 2/0
  crates/api/src/topjson.rs: lines 468/478, branches 21/24; GAP 10/3
  crates/api/src/trades.rs: lines 1029/1048, branches 39/46; GAP 7/3
  crates/api/src/verify.rs: lines 246/250, branches 28/30; GAP 1/1
  crates/api/tests/cloud_mutants_bars.rs: lines 1773/1775, branches 19/34; GAP 0/5
  crates/api/tests/logs_r3_24.rs: lines 240/241, branches 10/10; GAP 0/0
  crates/api/tests/store_step_forms.rs: lines 50/50, branches 0/0; GAP 0/0

rows:
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
GAP prod BRANCH crates/api/src/booleanjson.rs:138:16 True  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanjson.rs:138:16 False  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/booleanjson.rs:194:30 False  held.root == root && held.identity == asked.identity && held.budget == budget
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
GAP prod BRANCH crates/api/src/booleanlaunch.rs:284:31 True  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod BRANCH crates/api/src/booleanlaunch.rs:284:31 False  .filter(|run| run.attempt == attempt && run.in_flight())
GAP prod LINE crates/api/src/booleanqualification_projection.rs:9  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  pub(super) fn project(reader: &Qualification, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:10  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reader.require_current()?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:11  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let pin = reader.completion_digest();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:12  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:13  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  return Err("qualification completion pin differs; no replacement page returned".into());
GAP prod LINE crates/api/src/booleanqualification_projection.rs:14  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:15  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if matches!(asked.kind.as_str(), "index-weeks" | "index-days") {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:16  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  return super::index_consistency_projection::page(reader, asked);
GAP prod LINE crates/api/src/booleanqualification_projection.rs:17  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:18  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let rows = reader.rows(pin, asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:19  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let stats = reader.original().statistics();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:20  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let observations = stats.rows(stats.completion_digest(), asked.offset, asked.limit)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:21  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if rows.len() != observations.len() {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:22  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  return Err("qualification page differs from original statistics extent".into());
GAP prod LINE crates/api/src/booleanqualification_projection.rs:23  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:24  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut rendered = Vec::new();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:25  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for (n, (row, original)) in rows.iter().zip(&observations).enumerate() {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:26  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let index = asked.offset + n;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:27  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  same_coordinate(row.original, row.family, row.coordinate, original)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:28  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut value = admission_projection::row(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:29  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  index,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:30  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &AdmissionRow {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:31  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  identity: row.original,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:32  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  source_index: index,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:33  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  values: row.values,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:34  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  verdict: row.verdict,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:35  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  },
GAP prod LINE crates/api/src/booleanqualification_projection.rs:36  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:37  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:38  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut value,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:39  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "statistics",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:40  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  statistics_row(stats, index, original)?,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:41  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:42  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(&mut value, "qualification", row_detail(reader, row)?)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:43  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:44  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut value,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:45  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "index_consistency",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:46  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  super::index_consistency_projection::setting(reader, index, row.verdict.status())?,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:47  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:48  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rendered.push(value);
GAP prod LINE crates/api/src/booleanqualification_projection.rs:50  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut body = base(asked, pin, reader.row_count(), rendered)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:51  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(&mut body, "summary", summary(stats.summary())?)?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:52  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:53  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:54  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "statistics_identity",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:55  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!(crate::server::hex32(stats.identity())),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:56  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:57  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:58  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:59  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "statistics_completion",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:60  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!(crate::server::hex32(stats.completion_digest())),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:61  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:62  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:63  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:64  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "policy",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:65  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  admission_projection::policy(&reader.policy()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:66  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:67  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:68  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:69  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "admitted_bytes",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:70  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!(reader.admitted_bytes().to_string()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:71  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:72  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let allocation = reader.allocation();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:73  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let threshold = allocation.threshold();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:74  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let minimum = allocation.minimum_draws();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:75  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let [draws, seed, block_length] = reader.procedure();
GAP prod LINE crates/api/src/booleanqualification_projection.rs:76  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:77  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:78  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "qualification",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:79  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!({
GAP prod LINE crates/api/src/booleanqualification_projection.rs:80  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "scope":crate::server::hex32(reader.scope()),"unit":crate::server::hex32(reader.unit()),
GAP prod LINE crates/api/src/booleanqualification_projection.rs:81  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "original":{"identity":crate::server::hex32(reader.original().identity()),"completion":crate::server::hex32(reader.original().completion_dig
GAP prod LINE crates/api/src/booleanqualification_projection.rs:82  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "fold_count":reader.fold_count().to_string(),"procedure":{"draws":draws.to_string(),"seed":seed.to_string(),"block_length":block_length.to_s
GAP prod LINE crates/api/src/booleanqualification_projection.rs:83  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "allocation":{"digest":crate::server::hex32(allocation.digest()),"batch":allocation.batch().to_string(),"batches":allocation.batches().to_st
GAP prod LINE crates/api/src/booleanqualification_projection.rs:84  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "later":reader.later().iter().enumerate().map(|(index,later)|json!({"index":index.to_string(),"identity":crate::server::hex32(later.identity
GAP prod LINE crates/api/src/booleanqualification_projection.rs:86  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:87  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  put(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:88  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  &mut body,
GAP prod LINE crates/api/src/booleanqualification_projection.rs:89  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "scope",
GAP prod LINE crates/api/src/booleanqualification_projection.rs:90  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!(
GAP prod LINE crates/api/src/booleanqualification_projection.rs:93  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  )?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:94  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reader.require_current()?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:95  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Ok(body)
GAP prod LINE crates/api/src/booleanqualification_projection.rs:96  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/booleanqualification_projection.rs:119  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn row_detail(reader: &Qualification, row: &QualificationRow) -> Result<Value, String> {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:120  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let later = reader
GAP prod LINE crates/api/src/booleanqualification_projection.rs:121  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .later()
GAP prod LINE crates/api/src/booleanqualification_projection.rs:122  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(row.family)
GAP prod LINE crates/api/src/booleanqualification_projection.rs:123  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("qualification later family missing")?;
GAP prod LINE crates/api/src/booleanqualification_projection.rs:124  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let folds = if let Some(folds) = &row.folds {
GAP prod LINE crates/api/src/booleanqualification_projection.rs:125  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Some(json!({
GAP prod LINE crates/api/src/booleanqualification_projection.rs:126  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "plan":crate::server::hex32(folds.plan_digest()),"selected":crate::server::hex32(folds.selected_digest()),"anchor":crate::server::hex32(fold
GAP prod LINE crates/api/src/booleanqualification_projection.rs:127  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "later":crate::server::hex32(folds.later_digest()),"training_run":crate::server::hex32(folds.training_run_id()),"later_run":crate::server::h
GAP prod LINE crates/api/src/booleanqualification_projection.rs:128  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "ordinal":folds.ordinal().to_string(),"training_last_day":folds.training_last_day().to_string(),"first_day":folds.requested().first_day().to
GAP prod LINE crates/api/src/booleanqualification_projection.rs:129  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "decided":folds.decided_folds().to_string(),"profitable":folds.profitable_oos_folds().to_string(),"return_paisa":folds.aggregate_oos_paisa()
GAP prod LINE crates/api/src/booleanqualification_projection.rs:130  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "rows":folds.folds().iter().enumerate().map(|(index,fold)|json!({"index":index.to_string(),"first_day":fold.window.first_day().to_string(),"
GAP prod LINE crates/api/src/booleanqualification_projection.rs:133  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  None
GAP prod LINE crates/api/src/booleanqualification_projection.rs:136  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!({"identity":crate::server::hex32(row.later),"family":row.family.to_string(),"coordinate":row.coordinate.to_string(),"source":{"identit
GAP prod LINE crates/api/src/booleanqualification_projection.rs:138  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:12:8 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:12:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.completion.is_some_and(|expected| expected != pin) {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:21:8 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if rows.len() != observations.len() {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:21:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if rows.len() != observations.len() {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:124:24 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let folds = if let Some(folds) = &row.folds {
GAP prod BRANCH crates/api/src/booleanqualification_projection.rs:124:24 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let folds = if let Some(folds) = &row.folds {
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
GAP ignored LINE crates/api/src/calendar_of.rs:2159  fn it_reproduces_the_operators_store() {
GAP ignored LINE crates/api/src/calendar_of.rs:2160  let Some(home) = std::env::var_os("HOME") else {
GAP ignored LINE crates/api/src/calendar_of.rs:2161  panic!("no HOME");
GAP ignored LINE crates/api/src/calendar_of.rs:2163  let root = std::path::PathBuf::from(home).join(".brutex/store");
GAP ignored LINE crates/api/src/calendar_of.rs:2164  assert!(
GAP ignored LINE crates/api/src/calendar_of.rs:2165  root.join("bars").is_dir(),
GAP ignored LINE crates/api/src/calendar_of.rs:2168  root.display()
GAP ignored LINE crates/api/src/calendar_of.rs:2171  let mut months = Vec::new();
GAP ignored LINE crates/api/src/calendar_of.rs:2172  for year in 2019..=2026_u16 {
GAP ignored LINE crates/api/src/calendar_of.rs:2173  for month in 1..=12_u8 {
GAP ignored LINE crates/api/src/calendar_of.rs:2174  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored LINE crates/api/src/calendar_of.rs:2175  months.push(ym);
GAP ignored LINE crates/api/src/calendar_of.rs:2176  }
GAP ignored LINE crates/api/src/calendar_of.rs:2180  let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
GAP ignored LINE crates/api/src/calendar_of.rs:2182  println!(
GAP ignored LINE crates/api/src/calendar_of.rs:2184  cal.span(),
GAP ignored LINE crates/api/src/calendar_of.rs:2185  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2188  report.unreadable.len()
GAP ignored LINE crates/api/src/calendar_of.rs:2191  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2192  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2198  let mut owed = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2199  let mut unmeasured = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2200  for day in cal.first_day()..=cal.last_day() {
GAP ignored LINE crates/api/src/calendar_of.rs:2201  match cal.expected_bars(day) {
GAP ignored LINE crates/api/src/calendar_of.rs:2202  Some(n) => owed = owed.saturating_add(u32::from(n)),
GAP ignored LINE crates/api/src/calendar_of.rs:2204  if matches!(
GAP ignored LINE crates/api/src/calendar_of.rs:2205  cal.kind_of(day),
GAP ignored LINE crates/api/src/calendar_of.rs:2207  ) {
GAP ignored LINE crates/api/src/calendar_of.rs:2208  unmeasured = unmeasured.saturating_add(1);
GAP ignored LINE crates/api/src/calendar_of.rs:2209  }
GAP ignored LINE crates/api/src/calendar_of.rs:2213  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2218  println!("minute bars owed: {owed}");
GAP ignored LINE crates/api/src/calendar_of.rs:2219  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2225  }
GAP prod LINE crates/api/src/calendar_of.rs:2672  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2685  flush(&mut out, done);
GAP prod LINE crates/api/src/calendar_of.rs:2936  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2947  if n > 0 {
GAP prod LINE crates/api/src/calendar_of.rs:2948  out.push(',');
GAP prod LINE crates/api/src/calendar_of.rs:2949  }
GAP prod LINE crates/api/src/calendar_of.rs:2950  out.push_str(&crate::pullrun::quote_for_json(name));
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
GAP ignored BRANCH crates/api/src/calendar_of.rs:2160:13 True  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2160:13 False  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2174:24 True  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2174:24 False  if let Ok(ym) = YearMonth::new(year, month) {
GAP prod BRANCH crates/api/src/calendar_of.rs:2413:12 False  if let Some(at) = entry {
GAP prod BRANCH crates/api/src/calendar_of.rs:2671:12 True  if !first {
GAP prod BRANCH crates/api/src/calendar_of.rs:2684:12 True  if let Some(done) = run {
GAP prod BRANCH crates/api/src/calendar_of.rs:2935:12 True  if position > 0 {
GAP prod BRANCH crates/api/src/calendar_of.rs:2947:16 True  if n > 0 {
GAP prod BRANCH crates/api/src/calendar_of.rs:2947:16 False  if n > 0 {
GAP test LINE crates/api/src/calendar_record.rs:317  DayKind::Unmeasured => None,
GAP prod LINE crates/api/src/calendar_record.rs:356  }
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
GAP prod BRANCH crates/api/src/calendar_record.rs:580:27 False  while from <= last && rows.len() <= 8 {
GAP prod BRANCH crates/api/src/calendar_record.rs:643:17 False  let DayKind::Open(session) = pull::calendar::exchange_kind_of(day) else {
GAP prod BRANCH crates/api/src/calendar_record.rs:837:23 True  } else if let Some((start, end)) = run.take() {
GAP prod BRANCH crates/api/src/calendar_record.rs:880:20 True  if names.is_empty() {
GAP test BRANCH crates/api/src/calendar_record_tests.rs:476:56 False  if record_code(day).is_some_and(is_session) && day != at(2022, 3, 1) {
GAP prod LINE crates/api/src/calendar_route.rs:311  Joined::Full => return builds_full(),
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
GAP prod BRANCH crates/api/src/calendar_route.rs:276:16 False  && let Some(marked) = crate::calendar_of::record::not_widened(&answer.2)
GAP prod BRANCH crates/api/src/calendar_route.rs:491:20 True  Some(_) if progress.owed > 0 => format!(
GAP prod BRANCH crates/api/src/calendar_route.rs:548:12 False  if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
GAP prod BRANCH crates/api/src/calendar_route.rs:901:13 False  *exchange == brutex_core::instrument::Exchange::Nse
GAP prod BRANCH crates/api/src/calendar_route.rs:981:43 True  if read.contains_key(identity) || !rungs.contains_key(&rung) {
GAP prod BRANCH crates/api/src/calendar_route.rs:1080:28 False  if keep && let Some(at) = stamp {
GAP prod BRANCH crates/api/src/calendar_route.rs:1154:12 True  if self.builds.iter().any(|(queued, _)| *queued == key) {
GAP prod BRANCH crates/api/src/calendar_route.rs:1301:12 True  if answer.0 != axum::http::StatusCode::OK {
GAP test LINE crates/api/src/calendar_route_tests.rs:654  }
GAP test LINE crates/api/src/calendar_route_tests.rs:655  assert_eq!(
GAP test LINE crates/api/src/calendar_route_tests.rs:660  assert_eq!(answer["status"], "building", "{query}: {answer}");
GAP test LINE crates/api/src/calendar_route_tests.rs:661  tokio::time::sleep(std::time::Duration::from_millis(10)).await;
GAP test BRANCH crates/api/src/calendar_route_tests.rs:652:16 False  if status == axum::http::StatusCode::OK {
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
GAP prod LINE crates/api/src/census_cache.rs:1042  }
GAP prod LINE crates/api/src/census_cache.rs:1043  let rows = vendors
GAP prod LINE crates/api/src/census_cache.rs:1044  .warmth
GAP prod LINE crates/api/src/census_cache.rs:1045  .lock()
GAP prod LINE crates/api/src/census_cache.rs:1046  .unwrap_or_else(PoisonError::into_inner);
GAP prod LINE crates/api/src/census_cache.rs:1047  rows.iter()
GAP prod LINE crates/api/src/census_cache.rs:1048  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
GAP prod LINE crates/api/src/census_cache.rs:1049  .map(|row| row.since)
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
GAP prod BRANCH crates/api/src/census_cache.rs:1037:8 False  if !vendors
GAP prod BRANCH crates/api/src/census_cache.rs:1048:21 True  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
GAP prod BRANCH crates/api/src/census_cache.rs:1048:21 False  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
GAP prod BRANCH crates/api/src/census_cache.rs:1048:41 True  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
GAP prod BRANCH crates/api/src/census_cache.rs:1048:41 False  .find(|row| row.name == name && row.vendor == vendor && row.state == WarmState::Warming)
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
GAP prod BRANCH crates/api/src/constituents.rs:533:16 False  if let Some(id) = id
GAP prod BRANCH crates/api/src/constituents.rs:534:20 False  && let Some(slot) = self.per_vendor.get_mut(vendor as usize)
GAP prod LINE crates/api/src/dayflags_rules.rs:320  return Ok(kept);
GAP prod BRANCH crates/api/src/dayflags_rules.rs:319:16 True  if let Some(kept) = flags().get(&key) {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2709  fn the_day_band_is_timed_cold_warm_concurrent_and_growing() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2710  [inside an #[ignore]d test]  let _ = day_band_timed(&[1, 10, 100], [20, 200, 60], 300);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2711  }
GAP test LINE crates/api/src/dayflags_rules_tests.rs:2971  _ => panic!("the month opens"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3055  fn the_cold_auction_era_share_page_is_timed() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3056  let root = std::path::PathBuf::from(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3057  std::env::var_os("BRUTEX_DAYFLAGS_STORE")
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3058  .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3060  cold_share_pages_timed(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3061  &root,
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3062  &[
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3063  ("NSE-CASH-DALBHARAT", "2026-08-10", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3064  ("NSE-CASH-RELIANCE", "2026-08-03", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3065  ],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3067  [20, 10],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3069  }
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3289  fn the_day_band_pages_for_a_profiler() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3290  [inside an #[ignore]d test]  let _ = profiled_pages("r3-13-profile", 10, &TIMED);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3291  }
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1603:60 False  [inside a closure whose false side does not fail the check]  says(record(&got, "2024-10-03")).iter().any(|said| said
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1981:9 False  [an operand followed by ||: neither side fails the check by itself]  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1981:39 True  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:1981:39 False  from >= resume_hm.as_str() || from < "09:15",
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3495:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3523:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:3800:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:4049:9 False  let super::super::Opened::File(file) =
GAP test BRANCH crates/api/src/dayflags_rules_tests.rs:4093:13 False  let super::super::Opened::File(file) =
GAP prod LINE crates/api/src/disk_route.rs:307  format!(
GAP prod LINE crates/api/src/disk_route.rs:309  root.display()
GAP prod LINE crates/api/src/disk_route.rs:311  })?;
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
GAP test LINE crates/api/src/disk_route.rs:1636  continue;
GAP test LINE crates/api/src/disk_route.rs:1650  }
GAP test LINE crates/api/src/disk_route.rs:1685  }
GAP ignored LINE crates/api/src/disk_route.rs:2120  fn the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50() {
GAP ignored LINE crates/api/src/disk_route.rs:2121  timed_against_the_child("disk-route-timing", 1_000, 1);
GAP ignored LINE crates/api/src/disk_route.rs:2122  }
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
GAP test BRANCH crates/api/src/disk_route.rs:1635:16 True  [the true side of a condition is a passing path]  if counts_of(&dir) != before {
GAP test BRANCH crates/api/src/disk_route.rs:1639:16 False  if (fs.total, fs.used, fs.free)
GAP test BRANCH crates/api/src/disk_route.rs:1678:20 False  if let Some(parent) = mount.parent() {
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
GAP test LINE crates/api/src/head_deadline_body_tests.rs:68  return std::task::Poll::Ready(None);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:97  tokio::time::sleep(T * 3).await;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:98  "slow"
GAP test LINE crates/api/src/head_deadline_body_tests.rs:99  }),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:220  got.len()
GAP test LINE crates/api/src/head_deadline_body_tests.rs:307  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:308  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:309  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:310  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  took >= T * 4,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:313  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  let said = drain(&mut client, T * 10).await;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:314  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:315  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:521  [host red here (this kernel cuts the paced reader, proved on START): pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is passes on the Mac, whose Route B owes this row]  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:522  [host red here (this kernel cuts the paced reader, proved on START): pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is passes on the Mac, whose Route B owes this row]  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:523  [host red here (this kernel cuts the paced reader, proved on START): pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is passes on the Mac, whose Route B owes this row]  });
GAP test LINE crates/api/src/head_deadline_body_tests.rs:546  [host red here (this kernel cuts the paced reader, proved on START): pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is passes on the Mac, whose Route B owes this row]  assert!(lines(from, "response cut", "test=pauses-under").is_empty());
GAP test LINE crates/api/src/head_deadline_body_tests.rs:547  [host red here (this kernel cuts the paced reader, proved on START): pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is passes on the Mac, whose Route B owes this row]  assert_eq!(lines(from, "response cut", "test=pause-over").len(), 1);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:575  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row]  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:576  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row]  let said = drain(&mut client, T * 20).await;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:577  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row]  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:578  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:629  fn unchunk(mut framed: &[u8]) -> Vec<u8> {
GAP test LINE crates/api/src/head_deadline_body_tests.rs:630  let mut body = Vec::new();
GAP test LINE crates/api/src/head_deadline_body_tests.rs:632  let line = framed
GAP test LINE crates/api/src/head_deadline_body_tests.rs:633  .windows(2)
GAP test LINE crates/api/src/head_deadline_body_tests.rs:634  .position(|w| w == b"\r\n")
GAP test LINE crates/api/src/head_deadline_body_tests.rs:635  .expect("a chunk size line");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:636  let size = usize::from_str_radix(std::str::from_utf8(&framed[..line]).unwrap(), 16)
GAP test LINE crates/api/src/head_deadline_body_tests.rs:637  .expect("a hex chunk size");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:638  framed = &framed[line + 2..];
GAP test LINE crates/api/src/head_deadline_body_tests.rs:639  if size == 0 {
GAP test LINE crates/api/src/head_deadline_body_tests.rs:640  assert_eq!(framed, b"\r\n", "the last chunk ends the body");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:641  return body;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:642  }
GAP test LINE crates/api/src/head_deadline_body_tests.rs:643  body.extend_from_slice(&framed[..size]);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:644  assert_eq!(&framed[size..size + 2], b"\r\n");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:645  framed = &framed[size + 2..];
GAP test LINE crates/api/src/head_deadline_body_tests.rs:647  }
GAP test LINE crates/api/src/head_deadline_body_tests.rs:688  [host red here (this kernel cuts the paced reader, proved on START): a_chunked_body_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  let body = unchunk(&framed);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:689  [host red here (this kernel cuts the paced reader, proved on START): a_chunked_body_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:690  [host red here (this kernel cuts the paced reader, proved on START): a_chunked_body_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:691  [host red here (this kernel cuts the paced reader, proved on START): a_chunked_body_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row]  assert!(began.elapsed() >= T * 4);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:800  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:801  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  let done = began.elapsed();
GAP test LINE crates/api/src/head_deadline_body_tests.rs:802  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  drop(client);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:803  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  done
GAP test LINE crates/api/src/head_deadline_body_tests.rs:804  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  }));
GAP test LINE crates/api/src/head_deadline_body_tests.rs:818  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:819  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  answered + EARLY >= first_done,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:822  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:823  [host red here (this kernel cuts the paced reader, proved on START): slow_readers_at_the_cap_are_served_whole_and_the_next_waits passes on the Mac, whose Route B owes this row]  first_done >= T * 4,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:999  Poll::Pending => panic!("the stall deadline passed and the write still waited"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:67:12 True  [the true side of a condition is a passing path]  if self.sent >= self.total {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:315:9 True  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row] [the true side of a condition is a passing path]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:315:48 True  [host red here (this kernel cuts the paced reader, proved on START): a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole passes on the Mac, whose Route B owes this row] [the true side of a condition is a passing path]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:578:9 True  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row] [the true side of a condition is a passing path]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:578:48 True  [host red here (this kernel cuts the paced reader, proved on START): a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole passes on the Mac, whose Route B owes this row] [the true side of a condition is a passing path]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:639:12 True  [the true side of a condition is a passing path]  if size == 0 {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:639:12 False  if size == 0 {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:22  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  const fn label(self) -> &'static str {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:23  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  match self {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:24  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Self::All => "all",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:25  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Self::Qualified => "qualified",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:27  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:130  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Ok(body) => {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:131  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let reply = response(StatusCode::OK, &body);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:132  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:133  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  refused(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:135  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "cumulative response exceeds byte admission; no prefix returned",
GAP prod LINE crates/api/src/indexstoprankingjson.rs:138  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reply
GAP prod LINE crates/api/src/indexstoprankingjson.rs:196  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  old.root == root
GAP prod LINE crates/api/src/indexstoprankingjson.rs:197  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.identity == asked.identity
GAP prod LINE crates/api/src/indexstoprankingjson.rs:198  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.rung == asked.rung
GAP prod LINE crates/api/src/indexstoprankingjson.rs:199  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.budget == budget
GAP prod LINE crates/api/src/indexstoprankingjson.rs:200  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.reader.checkpoint() == pin
GAP prod LINE crates/api/src/indexstoprankingjson.rs:201  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }) {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:206  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let (ranked, qualified, summary) = rank(&reader, budget.bytes)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:207  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  *held = Some(Cached {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:208  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  root: root.to_owned(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:209  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  identity: asked.identity,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:210  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rung: asked.rung,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:211  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  budget,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:212  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reader,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:213  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  ranked,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:214  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:215  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  summary,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:216  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  });
GAP prod LINE crates/api/src/indexstoprankingjson.rs:217  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:218  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let cached = held.as_ref().ok_or("cumulative cache admission missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:219  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let result = cached.reader.with_current(|| project(cached, asked));
GAP prod LINE crates/api/src/indexstoprankingjson.rs:220  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if result.is_err() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:221  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  *held = None;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:222  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:223  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  result
GAP prod LINE crates/api/src/indexstoprankingjson.rs:233  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn rank(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:234  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reader.with_current(|| rank_current(reader, budget))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:235  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:237  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn rank_current(reader: &Reader, budget: u64) -> Result<(Vec<Rank>, Vec<usize>, Value), String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:238  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let count = reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:239  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .families()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:240  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .iter()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:241  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .try_fold(0_usize, |count, family| {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:242  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  count
GAP prod LINE crates/api/src/indexstoprankingjson.rs:243  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .checked_add(family.reader.rows().len())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:244  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative rank count overflow")
GAP prod LINE crates/api/src/indexstoprankingjson.rs:245  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  })?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:246  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let extra = u64::try_from(count)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:247  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .map_err(display)?
GAP prod LINE crates/api/src/indexstoprankingjson.rs:248  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .checked_mul(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:249  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  u64::try_from(std::mem::size_of::<Rank>() + 2 * std::mem::size_of::<usize>())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:250  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .map_err(display)?,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:252  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative rank byte count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:253  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:254  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .admitted_bytes()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:255  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .checked_add(reader.projection_scratch_bytes())
GAP prod LINE crates/api/src/indexstoprankingjson.rs:256  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .and_then(|bytes| bytes.checked_add(extra))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:257  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .is_none_or(|bytes| bytes > budget)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:259  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  return Err("cumulative evidence and complete rank indexes exceed read admission".into());
GAP prod LINE crates/api/src/indexstoprankingjson.rs:260  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:261  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut rows = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:262  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rows.try_reserve_exact(count).map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:263  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut counts = [0_u64; 5];
GAP prod LINE crates/api/src/indexstoprankingjson.rs:264  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for (family_index, family) in reader.families().iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:265  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let first = rows.len();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:266  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for (index, row) in family.reader.rows().iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:267  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let later = family
GAP prod LINE crates/api/src/indexstoprankingjson.rs:268  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:269  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .later()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:270  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .records()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:271  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:272  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative later coordinate absent")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:273  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let daily = family
GAP prod LINE crates/api/src/indexstoprankingjson.rs:274  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:275  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .consistencies()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:276  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:277  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative daily coordinate absent")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:278  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let state = row.projection.verdict().status();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:279  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let qualified = daily.combined_qualifies(state);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:280  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let slot = match state {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:281  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  cli::boolean_evidence::AdmissionStatusV1::Admitted => 0,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:282  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  cli::boolean_evidence::AdmissionStatusV1::Rejected => 1,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:283  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  cli::boolean_evidence::AdmissionStatusV1::Refused => 2,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:284  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  cli::boolean_evidence::AdmissionStatusV1::Unmeasured => 3,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:286  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let count = counts
GAP prod LINE crates/api/src/indexstoprankingjson.rs:287  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get_mut(slot)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:288  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative verdict slot unavailable")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:289  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  *count = count
GAP prod LINE crates/api/src/indexstoprankingjson.rs:290  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .checked_add(1)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:291  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative verdict count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:292  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  counts[4] = counts[4]
GAP prod LINE crates/api/src/indexstoprankingjson.rs:293  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .checked_add(u64::from(qualified))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:294  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative combined count overflow")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:295  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rows.push(Rank {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:296  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  score: later.metrics().pessimistic_paisa,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:297  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  family: family_index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:298  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  batch: family.batch,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:299  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  setting: index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:300  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  family_position: 0,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:301  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:302  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  });
GAP prod LINE crates/api/src/indexstoprankingjson.rs:306  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let family_rows = rows
GAP prod LINE crates/api/src/indexstoprankingjson.rs:307  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get_mut(first..)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:308  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative family rank extent missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:309  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  sort_ranks(family_rows);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:310  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for (rank, row) in family_rows.iter_mut().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:311  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  row.family_position = rank + 1;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:312  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:314  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  sort_ranks(&mut rows);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:315  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut qualified = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:316  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  qualified
GAP prod LINE crates/api/src/indexstoprankingjson.rs:317  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .try_reserve_exact(usize::try_from(counts[4]).map_err(display)?)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:318  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:319  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for (index, row) in rows.iter().enumerate() {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:320  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if row.qualified {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:321  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  qualified.push(index);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:322  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:324  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Ok((
GAP prod LINE crates/api/src/indexstoprankingjson.rs:325  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rows,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:326  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  qualified,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:327  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!({"institutional_admitted":counts[0].to_string(),"rejected":counts[1].to_string(),"refused":counts[2].to_string(),"unmeasured":counts[3
GAP prod LINE crates/api/src/indexstoprankingjson.rs:328  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  ))
GAP prod LINE crates/api/src/indexstoprankingjson.rs:329  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:330  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn project(cached: &Cached, asked: &Asked) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:331  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let reader = &cached.reader;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:332  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let total = match asked.filter {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:333  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Filter::All => cached.ranked.len(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:334  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Filter::Qualified => cached.qualified.len(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:336  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.offset > total {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:337  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  return Err("cumulative ranking page lies outside its pinned filtered extent".into());
GAP prod LINE crates/api/src/indexstoprankingjson.rs:338  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:339  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let end = asked.offset.saturating_add(asked.limit).min(total);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:340  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut rows = Vec::new();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:341  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rows.try_reserve_exact(end - asked.offset)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:342  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .map_err(display)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:343  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  for index in asked.offset..end {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:344  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let global = match asked.filter {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:345  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Filter::All => index,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:346  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Filter::Qualified => *cached
GAP prod LINE crates/api/src/indexstoprankingjson.rs:347  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .qualified
GAP prod LINE crates/api/src/indexstoprankingjson.rs:348  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(index)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:349  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("qualified rank index missing")?,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:351  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let ranked = cached
GAP prod LINE crates/api/src/indexstoprankingjson.rs:352  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ranked
GAP prod LINE crates/api/src/indexstoprankingjson.rs:353  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(global)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:354  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative rank index missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:355  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let family = reader
GAP prod LINE crates/api/src/indexstoprankingjson.rs:356  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .families()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:357  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(ranked.family)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:358  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative family missing")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:361  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rows.push(row(&family.reader, *ranked, index + 1, global + 1)?);
GAP prod LINE crates/api/src/indexstoprankingjson.rs:363  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let (sequence, pin) = reader.checkpoint();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:364  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let (grammar, bootstrap, splits) = reader.charged_work();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:365  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let declared = reader.declaration();
GAP prod LINE crates/api/src/indexstoprankingjson.rs:366  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let timeframe = cli::EVERY_RUNG
GAP prod LINE crates/api/src/indexstoprankingjson.rs:367  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .get(cached.rung)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:368  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("cumulative timeframe unavailable")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:370  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  json!({"schema_version":1,"status":"saved","model":MODEL,"identity":hex(cached.identity),"timeframe":timeframe,"rung":cached.rung.to_string(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:371  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "checkpoint":{"sequence":sequence.to_string(),"completion":hex(pin)},
GAP prod LINE crates/api/src/indexstoprankingjson.rs:372  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "scope":{"completed_batches":reader.completed_batches().to_string(),"completed_programs":reader.completed_programs().to_string(),"completed_
GAP prod LINE crates/api/src/indexstoprankingjson.rs:373  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "included_families":reader.families().len().to_string(),"first_batch":reader.families().first().map(|family|family.batch.to_string()),"last_
GAP prod LINE crates/api/src/indexstoprankingjson.rs:374  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "pending_next_batch":reader.pending(),"exhausted":reader.exhausted(),"interrupted_checkpoints":reader.interrupted().to_string(),"writer_obse
GAP prod LINE crates/api/src/indexstoprankingjson.rs:375  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "instrument":declared.index(),"feed":declared.feed(),"training_first_day":declared.training_days().0.to_string(),"training_last_day":declare
GAP prod LINE crates/api/src/indexstoprankingjson.rs:376  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "ranking":"later_pessimistic_descending_then_batch_then_setting","filter":asked.filter.label(),"total":total.to_string(),"all_total":cached.
GAP prod LINE crates/api/src/indexstoprankingjson.rs:377  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "read_work":{"retained_serialized_bytes":reader.admitted_bytes().to_string(),"charged_grammar_nodes":grammar.to_string(),"charged_required_b
GAP prod LINE crates/api/src/indexstoprankingjson.rs:378  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "cold":"complete_acknowledged_prefix_then_global_sort","warm":"ancestor_generation_checks_then_bounded_indexed_page"},"rows":rows,"refusal":
GAP prod LINE crates/api/src/indexstoprankingjson.rs:380  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:381  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn row(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:382  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  reader: &cli::index_stop_qualification::Reader,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:383  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  ranked: Rank,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:384  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  rank: usize,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:385  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  global_rank: usize,
GAP prod LINE crates/api/src/indexstoprankingjson.rs:386  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  ) -> Result<Value, String> {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:387  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let index = ranked.setting;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:388  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let mut value = crate::indexstopqualificationjson::setting(reader, index, rank)?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:389  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let fields = value
GAP prod LINE crates/api/src/indexstoprankingjson.rs:390  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .as_object_mut()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:391  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("native setting serializer returned a nonobject")?;
GAP prod LINE crates/api/src/indexstoprankingjson.rs:392  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  let extra = json!({"global_rank":global_rank.to_string(),"batch":ranked.batch.to_string(),"qualification_rank":ranked.family_position.to_str
GAP prod LINE crates/api/src/indexstoprankingjson.rs:393  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "qualification":{"identity":hex(reader.identity()),"completion":hex(reader.completion_digest())},"qualification_total":reader.rows().len().t
GAP prod LINE crates/api/src/indexstoprankingjson.rs:394  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  "original_source":{"identity":hex(reader.training().identity()),"completion":hex(reader.training().completion_digest())},"later_source":{"id
GAP prod LINE crates/api/src/indexstoprankingjson.rs:395  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fields.extend(
GAP prod LINE crates/api/src/indexstoprankingjson.rs:396  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  extra
GAP prod LINE crates/api/src/indexstoprankingjson.rs:397  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .as_object()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:398  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .ok_or("native ranking links returned a nonobject")?
GAP prod LINE crates/api/src/indexstoprankingjson.rs:399  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  .clone(),
GAP prod LINE crates/api/src/indexstoprankingjson.rs:401  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  Ok(value)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:402  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:403  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn hex(value: [u8; 32]) -> String {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:404  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  crate::server::hex32(value)
GAP prod LINE crates/api/src/indexstoprankingjson.rs:405  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod LINE crates/api/src/indexstoprankingjson.rs:406  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  fn display(value: impl std::fmt::Display) -> String {
GAP prod LINE crates/api/src/indexstoprankingjson.rs:407  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  value.to_string()
GAP prod LINE crates/api/src/indexstoprankingjson.rs:408  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  }
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:88:12 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if identity == [0; 32]
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:89:16 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  || completion == Some([0; 32])
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:132:16 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:132:16 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reply.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:195:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if !held.as_ref().is_some_and(|old| {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:196:9 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  old.root == root
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:196:9 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  old.root == root
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:197:16 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:197:16 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.identity == asked.identity
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:198:16 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.rung == asked.rung
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:198:16 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.rung == asked.rung
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:199:16 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.budget == budget
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:199:16 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  && old.budget == budget
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:220:8 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if result.is_err() {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:220:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if result.is_err() {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:253:8 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reader
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:253:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if reader
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:320:12 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if row.qualified {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:320:12 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if row.qualified {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:336:8 True  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.offset > total {
GAP prod BRANCH crates/api/src/indexstoprankingjson.rs:336:8 False  [L-BCA-5: past the parse, a saved index-stop search or Boolean qualification no fixture here writes; the operator's fixture decision]  if asked.offset > total {
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
GAP prod BRANCH crates/api/src/instruments_route.rs:194:51 False  || (master == MasterState::Unavailable && feed.publishes_master())
GAP prod LINE crates/api/src/master.rs:357  return Err(format!(
GAP prod LINE crates/api/src/master.rs:358  "{}: grew past {MAX_MASTER_BYTES} bytes while it was read; this \
GAP prod LINE crates/api/src/master.rs:359  reader holds at most {MAX_MASTER_BYTES}",
GAP prod LINE crates/api/src/master.rs:360  path.display()
GAP prod LINE crates/api/src/master.rs:361  ));
GAP prod BRANCH crates/api/src/master.rs:356:8 True  if text.len() as u64 > MAX_MASTER_BYTES {
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
GAP prod BRANCH crates/api/src/operation_audit.rs:226:20 False  if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
GAP test BRANCH crates/api/src/operation_audit_tests.rs:316:9 False  let Asked::Page {
GAP prod LINE crates/api/src/provenance.rs:412  }
GAP prod LINE crates/api/src/provenance.rs:422  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:423  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:425  asked,
GAP prod LINE crates/api/src/provenance.rs:426  &format!("{} {}: {why}", asked.key(), asked.month),
GAP prod LINE crates/api/src/provenance.rs:427  Value::Null,
GAP prod LINE crates/api/src/provenance.rs:475  Err(why @ StoreError::Missing { .. }) => {
GAP prod LINE crates/api/src/provenance.rs:476  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:478  asked,
GAP prod LINE crates/api/src/provenance.rs:479  &format!("{why}: this month is not held"),
GAP prod LINE crates/api/src/provenance.rs:480  json!({ "held": false, "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:523  Err(why) => {
GAP prod LINE crates/api/src/provenance.rs:524  return Answer::refused(
GAP prod LINE crates/api/src/provenance.rs:526  asked,
GAP prod LINE crates/api/src/provenance.rs:527  &why,
GAP prod LINE crates/api/src/provenance.rs:528  json!({ "path": relative }),
GAP prod LINE crates/api/src/provenance.rs:702  }
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
GAP prod BRANCH crates/api/src/provenance.rs:670:12 False  if let Some(object) = body.as_object_mut() {
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
GAP prod LINE crates/api/src/quarantine_route.rs:219  Err(why) => busy(&why),
GAP test LINE crates/api/src/quarantine_route_tests.rs:134  }
GAP test LINE crates/api/src/quarantine_route_tests.rs:135  tokio::time::sleep(std::time::Duration::from_millis(5)).await;
GAP test BRANCH crates/api/src/quarantine_route_tests.rs:132:12 False  if status != StatusCode::TOO_MANY_REQUESTS {
GAP test LINE crates/api/src/recovery_control.rs:523  "/System"
GAP prod LINE crates/api/src/render.rs:3284  continue;
GAP prod BRANCH crates/api/src/render.rs:3283:13 False  let Ok(kind) = e.file_type() else {
GAP prod LINE crates/api/src/scrub_route.rs:518  }
GAP prod LINE crates/api/src/scrub_route.rs:551  continue;
GAP prod LINE crates/api/src/scrub_route.rs:568  }
GAP prod LINE crates/api/src/scrub_route.rs:576  }
GAP prod LINE crates/api/src/scrub_route.rs:652  continue;
GAP prod LINE crates/api/src/scrub_route.rs:836  break;
GAP prod LINE crates/api/src/scrub_route.rs:1052  return Ok(read(&Index::default()));
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
GAP prod LINE crates/api/src/server.rs:1923  return (
GAP prod LINE crates/api/src/server.rs:1924  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:1925  headers,
GAP prod LINE crates/api/src/server.rs:1926  no_such_feed_json(&asked),
GAP prod LINE crates/api/src/server.rs:1927  );
GAP prod LINE crates/api/src/server.rs:1991  body.push('\n');
GAP prod LINE crates/api/src/server.rs:4102  return Some(format!("{}: {why}", dir.display()));
GAP prod LINE crates/api/src/server.rs:5020  telemetry::now_millis().unsigned_abs()
GAP prod LINE crates/api/src/server.rs:5825  Err(pull::session::SessionError::SessionLengthUnmeasured { .. }) => false,
GAP prod LINE crates/api/src/server.rs:5826  Err(why) => return Err(why.to_string()),
GAP prod LINE crates/api/src/server.rs:6174  None => None,
GAP prod LINE crates/api/src/server.rs:6177  pull::ingest::landing_run::Mode::Repair
GAP prod LINE crates/api/src/server.rs:6435  break;
GAP prod LINE crates/api/src/server.rs:6614  || spot_audit_origin(asked.cash_identity, &run.origin),
GAP prod LINE crates/api/src/server.rs:6633  return Err(why);
GAP prod LINE crates/api/src/server.rs:8997  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  async fn read_credential(
GAP prod LINE crates/api/src/server.rs:8998  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  identity: &pull::ssm::AwsIdentity,
GAP prod LINE crates/api/src/server.rs:8999  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  config: &pull::config::CredentialConfig,
GAP prod LINE crates/api/src/server.rs:9000  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  vendor: brutex_core::vendor::Vendor,
GAP prod LINE crates/api/src/server.rs:9001  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  auth: pull::vendor::Auth,
GAP prod LINE crates/api/src/server.rs:9002  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  ) -> Result<pull::http::Credential, crate::credential_law::Unreadable> {
GAP prod LINE crates/api/src/server.rs:9008  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  let stamp = pull::ssm::now_stamp().map_err(|why| Unreadable::configuration(why.detail))?;
GAP prod LINE crates/api/src/server.rs:9009  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  let read = async |field: &str| -> Result<String, Unreadable> {
GAP prod LINE crates/api/src/server.rs:9010  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  let path = config.path_for(vendor, field).map_err(|why| {
GAP prod LINE crates/api/src/server.rs:9011  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  Unreadable::configuration(format!(
GAP prod LINE crates/api/src/server.rs:9014  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  })?;
GAP prod LINE crates/api/src/server.rs:9015  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  pull::ssm::get_parameter(identity, config.region(), &path.to_string(), &stamp)
GAP prod LINE crates/api/src/server.rs:9016  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  .await
GAP prod LINE crates/api/src/server.rs:9017  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  .map_err(|why| {
GAP prod LINE crates/api/src/server.rs:9018  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  Unreadable::of_secret(
GAP prod LINE crates/api/src/server.rs:9019  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  why.kind,
GAP prod LINE crates/api/src/server.rs:9020  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  format!(
GAP prod LINE crates/api/src/server.rs:9025  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  })
GAP prod LINE crates/api/src/server.rs:9026  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  };
GAP prod LINE crates/api/src/server.rs:9028  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  let token = read("access-token").await?;
GAP prod LINE crates/api/src/server.rs:9029  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  match auth.key_field {
GAP prod LINE crates/api/src/server.rs:9033  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  None => Ok(pull::http::Credential::token(token)),
GAP prod LINE crates/api/src/server.rs:9037  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  Some(field) => Ok(pull::http::Credential::pair(read(field).await?, token)),
GAP prod LINE crates/api/src/server.rs:9039  [L-BCA-6: read_credential ends at a real host (Parameter Store or the index host); the operator's seam decision]  }
GAP prod LINE crates/api/src/server.rs:9093  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  let identity = pull::ssm::AwsIdentity::discover()
GAP prod LINE crates/api/src/server.rs:9094  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  .map_err(|why| Unreadable::configuration(format!("no AWS identity: {why}")))?;
GAP prod LINE crates/api/src/server.rs:9116  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  let vendor = feed.store_vendor().ok_or_else(|| {
GAP prod LINE crates/api/src/server.rs:9117  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  Unreadable::configuration(format!(
GAP prod LINE crates/api/src/server.rs:9122  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  feed.display(),
GAP prod LINE crates/api/src/server.rs:9123  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  feed.display()
GAP prod LINE crates/api/src/server.rs:9125  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  })?;
GAP prod LINE crates/api/src/server.rs:9126  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  let credential = read_credential(&identity, &config, vendor, spec.auth).await?;
GAP prod LINE crates/api/src/server.rs:9134  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  let source = pull::http::HttpSource::new(*spec, credential)
GAP prod LINE crates/api/src/server.rs:9135  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  .map_err(|why| Unreadable::configuration(why.to_string()))?;
GAP prod LINE crates/api/src/server.rs:9137  [L-BCA-6: credentialed_source ends at a real host (Parameter Store or the index host); the operator's seam decision]  Ok((source, vendor))
GAP prod LINE crates/api/src/server.rs:9884  format!(
GAP prod LINE crates/api/src/server.rs:9888  feed.display()
GAP prod LINE crates/api/src/server.rs:9890  })?;
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
GAP prod LINE crates/api/src/server.rs:18323  Poll::Ready(Err(why)) => {
GAP prod LINE crates/api/src/server.rs:18324  let _left = self.report(
GAP prod LINE crates/api/src/server.rs:18325  telemetry::Level::Info,
GAP prod LINE crates/api/src/server.rs:18326  "response closed by the client",
GAP prod LINE crates/api/src/server.rs:18327  &format!("a write failed: {}", why.kind()),
GAP prod LINE crates/api/src/server.rs:18329  Poll::Ready(Err(why))
GAP test LINE crates/api/src/server.rs:18926  Ok(())
GAP test LINE crates/api/src/server.rs:18927  }),
GAP test LINE crates/api/src/server.rs:18952  Err(_elapsed) => return (String::from_utf8_lossy(&got).into_owned(), false),
GAP test LINE crates/api/src/server.rs:19078  Ok(())
GAP test LINE crates/api/src/server.rs:19079  }),
GAP test LINE crates/api/src/server.rs:19129  break;
GAP test LINE crates/api/src/server.rs:19397  break;
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
GAP ignored LINE crates/api/src/server.rs:22342  fn the_pull_order_on_a_real_census() {
GAP ignored LINE crates/api/src/server.rs:22343  let masters = std::env::var("BRUTEX_WP62_MASTERS")
GAP ignored LINE crates/api/src/server.rs:22344  .expect("BRUTEX_WP62_MASTERS names the masters directory");
GAP ignored LINE crates/api/src/server.rs:22345  let store =
GAP ignored LINE crates/api/src/server.rs:22346  std::env::var("BRUTEX_WP62_STORE").expect("BRUTEX_WP62_STORE names a shadow store");
GAP ignored LINE crates/api/src/server.rs:22347  let site = Site::load(Path::new(&masters), Path::new(&store));
GAP ignored LINE crates/api/src/server.rs:22348  println!(
GAP ignored LINE crates/api/src/server.rs:22350  site.restart.word(),
GAP ignored LINE crates/api/src/server.rs:22351  site.restart.words()
GAP ignored LINE crates/api/src/server.rs:22353  let censuses = census::read_all(&site.store_root);
GAP ignored LINE crates/api/src/server.rs:22354  for target in ["indices", "fno"] {
GAP ignored LINE crates/api/src/server.rs:22355  let asked = ingest::parse_spot(
GAP ignored LINE crates/api/src/server.rs:22356  &format!(
GAP ignored LINE crates/api/src/server.rs:22357  "vendor=zerodha&target={target}&cash_identity=zerodha_nse_checked\
GAP ignored LINE crates/api/src/server.rs:22358  &granularity=1min&from=2018-01-01&to=2026-09-30"
GAP ignored LINE crates/api/src/server.rs:22359  ),
GAP ignored LINE crates/api/src/server.rs:22360  day(2026, 10, 12),
GAP ignored LINE crates/api/src/server.rs:22362  .expect("the operator's own window");
GAP ignored LINE crates/api/src/server.rs:22363  let mut targets = spot_targets(&asked, &site);
GAP ignored LINE crates/api/src/server.rs:22364  targets.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22365  let asked_for = targets.len();
GAP ignored LINE crates/api/src/server.rs:22366  assert!(asked_for > 0, "{target}: the masters name no instrument");
GAP ignored LINE crates/api/src/server.rs:22367  let whole = ladder_refusal(&asked, &targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22368  let began = std::time::Instant::now();
GAP ignored LINE crates/api/src/server.rs:22369  let gate = order_gate(&asked, &mut targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22370  let took = began.elapsed();
GAP ignored LINE crates/api/src/server.rs:22371  let whole = whole.unwrap_or_else(|| {
GAP ignored LINE crates/api/src/server.rs:22372  panic!("{target}: the whole-run gate is open, so this store shows nothing")
GAP ignored LINE crates/api/src/server.rs:22374  println!(
GAP ignored LINE crates/api/src/server.rs:22376  whole.chars().take(160).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22378  let withheld = gate
GAP ignored LINE crates/api/src/server.rs:22379  .unwrap_or_else(|why| panic!("{target}: still refused whole in {took:?}: {why}"));
GAP ignored LINE crates/api/src/server.rs:22380  println!(
GAP ignored LINE crates/api/src/server.rs:22382  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22383  targets.len()
GAP ignored LINE crates/api/src/server.rs:22385  assert!(
GAP ignored LINE crates/api/src/server.rs:22386  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored LINE crates/api/src/server.rs:22388  withheld.len()
GAP ignored LINE crates/api/src/server.rs:22390  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22391  targets.len() + withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22395  let mut named: Vec<_> = withheld.iter().map(|(key, _)| *key).collect();
GAP ignored LINE crates/api/src/server.rs:22396  named.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22397  named.dedup();
GAP ignored LINE crates/api/src/server.rs:22398  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22399  named.len(),
GAP ignored LINE crates/api/src/server.rs:22400  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22403  for (key, why) in withheld {
GAP ignored LINE crates/api/src/server.rs:22404  assert!(
GAP ignored LINE crates/api/src/server.rs:22405  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored LINE crates/api/src/server.rs:22409  println!(
GAP ignored LINE crates/api/src/server.rs:22412  why.chars().take(150).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22416  }
GAP test LINE crates/api/src/server.rs:28348  [read: the second operand of an || in the condition, never reached: not the failing arm]  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test LINE crates/api/src/server.rs:28349  [read: part of that never-reached operand]  .unwrap_or_default()
GAP test LINE crates/api/src/server.rs:28350  [read: part of that never-reached operand]  .contains(") Z "),
GAP test LINE crates/api/src/server.rs:29577  let lock_path = root.join("serve.lock");
GAP test LINE crates/api/src/server.rs:29578  let busy = std::fs::OpenOptions::new()
GAP test LINE crates/api/src/server.rs:29579  .read(true)
GAP test LINE crates/api/src/server.rs:29580  .write(true)
GAP test LINE crates/api/src/server.rs:29581  .open(&lock_path)
GAP test LINE crates/api/src/server.rs:29582  .is_ok_and(|file| store::flock::Flock::try_lock(file, lock_path.clone()).is_err());
GAP test LINE crates/api/src/server.rs:29583  if busy {
GAP test LINE crates/api/src/server.rs:29589  assert_eq!(
GAP test LINE crates/api/src/server.rs:29590  run(&argv(&["serve", "127.0.0.1:0"]), fired()).await,
GAP test LINE crates/api/src/server.rs:29594  let refused =
GAP test LINE crates/api/src/server.rs:29595  crate::emitted::landed(from, "api.serve", "refused: this store cannot be served");
GAP test LINE crates/api/src/server.rs:29596  assert!(
GAP test LINE crates/api/src/server.rs:29597  refused.iter().any(|record| {
GAP test LINE crates/api/src/server.rs:29598  record.level == telemetry::Level::Error
GAP test LINE crates/api/src/server.rs:29599  && crate::emitted::says(record, "why", "already serving this store")
GAP test LINE crates/api/src/server.rs:29600  }),
GAP test LINE crates/api/src/server.rs:29603  return;
GAP test LINE crates/api/src/server.rs:29604  }
GAP test LINE crates/api/src/server.rs:29606  let expected =
GAP test LINE crates/api/src/server.rs:29607  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test LINE crates/api/src/server.rs:29613  assert_eq!(
GAP test LINE crates/api/src/server.rs:29614  run(&argv(&["serve", "127.0.0.1:0"]), after_the_load(&root)).await,
GAP test LINE crates/api/src/server.rs:29619  let listening = crate::emitted::landed(from, "api.serve", "listening");
GAP test LINE crates/api/src/server.rs:29620  let mine = listening
GAP test LINE crates/api/src/server.rs:29621  .iter()
GAP test LINE crates/api/src/server.rs:29622  .find(|record| crate::emitted::says(record, "addr", "127.0.0.1:"))
GAP test LINE crates/api/src/server.rs:29623  .expect("the process's own configuration is the first thing it writes");
GAP test LINE crates/api/src/server.rs:29624  assert_eq!(mine.level, telemetry::Level::Info);
GAP test LINE crates/api/src/server.rs:29628  assert!(
GAP test LINE crates/api/src/server.rs:29655  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:30010  [read: the second operand of an || in the condition, never reached: not the failing arm]  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test LINE crates/api/src/server.rs:30191  matches!(
GAP test LINE crates/api/src/server.rs:30778  feed.display()
GAP test LINE crates/api/src/server.rs:31803  (axum::http::StatusCode::OK, String::new())
GAP test LINE crates/api/src/server.rs:31804  })
GAP test LINE crates/api/src/server.rs:32147  assert!(
GAP test LINE crates/api/src/server.rs:32148  spec.window_caps.is_empty(),
GAP test LINE crates/api/src/server.rs:32153  feed.display()
GAP test LINE crates/api/src/server.rs:32155  continue;
GAP test LINE crates/api/src/server.rs:32546  || "null".to_owned(),
GAP test LINE crates/api/src/server.rs:32746  other => panic!("attempt {attempt} of a 429 gave {other:?}"),
GAP test LINE crates/api/src/server.rs:34513  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:34627  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP test LINE crates/api/src/server.rs:34847  key("NIFTYCHEMICALS", m)
GAP test LINE crates/api/src/server.rs:34848  })
GAP test LINE crates/api/src/server.rs:34867  pull::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
GAP prod LINE crates/api/src/server.rs:36805  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let feed = vendor.as_str().to_owned();
GAP prod LINE crates/api/src/server.rs:36821  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let owned = {
GAP prod LINE crates/api/src/server.rs:36822  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let held = site.universe();
GAP prod LINE crates/api/src/server.rs:36823  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  resolved_master_rows(&held.read.merged, vendor)
GAP prod LINE crates/api/src/server.rs:36825  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let master: Vec<pull::universe::VendorInstrument<'_>> = owned
GAP prod LINE crates/api/src/server.rs:36826  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .iter()
GAP prod LINE crates/api/src/server.rs:36827  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .map(|(id, symbol, isin)| pull::universe::VendorInstrument {
GAP prod LINE crates/api/src/server.rs:36828  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  vendor_id: id,
GAP prod LINE crates/api/src/server.rs:36829  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  trading_symbol: symbol,
GAP prod LINE crates/api/src/server.rs:36830  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  isin,
GAP prod LINE crates/api/src/server.rs:36831  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  })
GAP prod LINE crates/api/src/server.rs:36832  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .collect();
GAP prod LINE crates/api/src/server.rs:36834  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let source = match pull::resolve::HttpDocuments::new() {
GAP prod LINE crates/api/src/server.rs:36835  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Ok(client) => client,
GAP prod LINE crates/api/src/server.rs:36836  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Err(why) => {
GAP prod LINE crates/api/src/server.rs:36837  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  return (
GAP prod LINE crates/api/src/server.rs:36838  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:36839  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:36840  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  format!(r#"{{"ok":false,"why":{}}}"#, render::json_string(&why)),
GAP prod LINE crates/api/src/server.rs:36841  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  );
GAP prod LINE crates/api/src/server.rs:36847  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let today = match crate::ingest::today_ist() {
GAP prod LINE crates/api/src/server.rs:36848  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Ok(day) => day,
GAP prod LINE crates/api/src/server.rs:36849  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Err(why) => {
GAP prod LINE crates/api/src/server.rs:36850  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  return (
GAP prod LINE crates/api/src/server.rs:36851  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:36852  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:36853  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  format!(
GAP prod LINE crates/api/src/server.rs:36854  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  r#"{{"ok":false,"why":{}}}"#,
GAP prod LINE crates/api/src/server.rs:36855  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  render::json_string(&why.to_string())
GAP prod LINE crates/api/src/server.rs:36856  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  ),
GAP prod LINE crates/api/src/server.rs:36857  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  );
GAP prod LINE crates/api/src/server.rs:36861  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let snap = pull::resolve::crawl(
GAP prod LINE crates/api/src/server.rs:36862  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &source,
GAP prod LINE crates/api/src/server.rs:36863  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  INDEX_HOST,
GAP prod LINE crates/api/src/server.rs:36864  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  today,
GAP prod LINE crates/api/src/server.rs:36865  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &feed,
GAP prod LINE crates/api/src/server.rs:36866  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // ASKED OF THE VENDOR, NOT ASSUMED -- and this line used to assume.
GAP prod LINE crates/api/src/server.rs:36867  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  //
GAP prod LINE crates/api/src/server.rs:36868  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // It was the literal `JoinKey::Isin`, under a comment reading "EVERY
GAP prod LINE crates/api/src/server.rs:36869  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // FEED IN THIS BUILD PUBLISHES AN ISIN. The symbol key exists for the
GAP prod LINE crates/api/src/server.rs:36870  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // one that does not, and asking the descriptor rather than assuming is
GAP prod LINE crates/api/src/server.rs:36871  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // what keeps that true when it lands." **It landed on 14 Aug 2026.**
GAP prod LINE crates/api/src/server.rs:36872  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // Zerodha's master has twelve columns and none is an ISIN, so this
GAP prod LINE crates/api/src/server.rs:36873  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // literal joined its rows on an EMPTY field -- and `universe::resolve`
GAP prod LINE crates/api/src/server.rs:36874  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // skips an empty key deliberately, so nothing would have errored. Every
GAP prod LINE crates/api/src/server.rs:36875  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // Zerodha instrument would have landed in `Verdict::Lacks`, which reads
GAP prod LINE crates/api/src/server.rs:36876  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // exactly like a vendor that lists nothing.
GAP prod LINE crates/api/src/server.rs:36877  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  //
GAP prod LINE crates/api/src/server.rs:36878  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // `JoinKey::for_vendor` derives it from `master_columns().isin`, which
GAP prod LINE crates/api/src/server.rs:36879  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // already spells Zerodha's as empty, so there is no second table to
GAP prod LINE crates/api/src/server.rs:36880  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // fall behind the first.
GAP prod LINE crates/api/src/server.rs:36881  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  pull::universe::JoinKey::for_vendor(vendor),
GAP prod LINE crates/api/src/server.rs:36882  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &master,
GAP prod LINE crates/api/src/server.rs:36883  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  )
GAP prod LINE crates/api/src/server.rs:36884  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .await;
GAP prod LINE crates/api/src/server.rs:36886  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let verdict = snap.admits_publication(None);
GAP prod LINE crates/api/src/server.rs:36887  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  (
GAP prod LINE crates/api/src/server.rs:36888  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::OK,
GAP prod LINE crates/api/src/server.rs:36889  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:36890  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  universe_snapshot_json(&snap, &verdict),
GAP prod LINE crates/api/src/server.rs:36891  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  )
GAP prod LINE crates/api/src/server.rs:37589  break;
GAP prod LINE crates/api/src/server.rs:37596  break;
GAP prod BRANCH crates/api/src/server.rs:1922:9 False  let Some(census) = now.census() else {
GAP prod BRANCH crates/api/src/server.rs:1990:8 True  if !body.ends_with('\n') {
GAP prod BRANCH crates/api/src/server.rs:2758:31 False  if counted && let Some(n) = targets.get_mut(slot) {
GAP prod BRANCH crates/api/src/server.rs:4100:12 True  && why.kind() != std::io::ErrorKind::AlreadyExists
GAP prod BRANCH crates/api/src/server.rs:4521:16 False  && let Ok(one) = pull::session::Window::new(day, day)
GAP prod BRANCH crates/api/src/server.rs:5019:21 True  let id = if run == 0 {
GAP prod BRANCH crates/api/src/server.rs:6075:12 True  if vendor != Vendor::Zerodha
GAP prod BRANCH crates/api/src/server.rs:6176:27 True  let mode = if asked.repair.is_some() {
GAP prod BRANCH crates/api/src/server.rs:6434:12 True  if out.store_unavailable.is_some() {
GAP prod BRANCH crates/api/src/server.rs:6632:12 True  if let Some(why) = run.ledger.refused.clone() {
GAP prod BRANCH crates/api/src/server.rs:6855:68 False  if asked.window.to() > today || (asked.window.to() == today && !closed) {
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
GAP prod BRANCH crates/api/src/server.rs:18229:43 True  HeadState::Awaiting { .. } if entered > finished => self.deliver(),
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
GAP prod BRANCH crates/api/src/server.rs:20528:8 False  if suppressed.is_some() {
GAP prod BRANCH crates/api/src/server.rs:20998:24 True  if let Some(why) = gate.unstamped {
GAP prod BRANCH crates/api/src/server.rs:21001:20 True  if flying_on_start {
GAP prod BRANCH crates/api/src/server.rs:21085:24 True  if let Err(why) = calendar_route::warm_at_start(&site) {
GAP prod BRANCH crates/api/src/server.rs:21091:24 True  if let Err(why) = census_cache::start(&site, &calendar_route::offered_feeds()) {
GAP test BRANCH crates/api/src/server.rs:21730:30 False  [read: the body of an .any() closure opened on the line above: not the failing arm]  |record| crate::emitted::says(record, "instrument", "BANKNIFTY")
GAP ignored BRANCH crates/api/src/server.rs:22386:17 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22386:17 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22386:41 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22386:41 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22405:21 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22405:21 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22405:59 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22405:59 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP test BRANCH crates/api/src/server.rs:22796:13 False  let pull::vendor::Transport::Http(spec) =
GAP test BRANCH crates/api/src/server.rs:24581:28 False  [read: an operand inside an .any() closure opened above: not the failing arm]  && f.why.contains("(it ends on 2026-12-31): 2027-01-08")
GAP test BRANCH crates/api/src/server.rs:24587:34 False  [inside a closure whose false side does not fail the check]  .any(|f| !f.why.starts_with("request minute coverage")
GAP test BRANCH crates/api/src/server.rs:25370:17 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:26622:28 False  && record.message == "listening"
GAP test BRANCH crates/api/src/server.rs:27289:17 False  [an operand followed by ||: neither side fails the check by itself]  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:27289:61 True  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:27289:61 False  over.contains("no vendor was contacted") || over.contains("no vendor is contacted"),
GAP test BRANCH crates/api/src/server.rs:28347:13 False  [read: the first operand of an || continued on the next line: not the failing arm]  !std::path::Path::new(&format!("/proc/{pid}")).exists()
GAP test BRANCH crates/api/src/server.rs:28348:20 True  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test BRANCH crates/api/src/server.rs:28348:20 False  || !std::fs::read_to_string(format!("/proc/{pid}/stat"))
GAP test BRANCH crates/api/src/server.rs:29557:13 True  let Some(root) = root else {
GAP test BRANCH crates/api/src/server.rs:29567:21 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29583:12 True  if busy {
GAP test BRANCH crates/api/src/server.rs:29583:12 False  if busy {
GAP test BRANCH crates/api/src/server.rs:29598:21 True  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29598:21 False  record.level == telemetry::Level::Error
GAP test BRANCH crates/api/src/server.rs:29607:51 True  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test BRANCH crates/api/src/server.rs:29607:51 False  masters_dir().map_or(FAILED, |dir| if report(&dir).1 { OK } else { DEGRADED });
GAP test BRANCH crates/api/src/server.rs:29629:13 True  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29629:13 False  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29629:50 True  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:29629:50 False  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
GAP test BRANCH crates/api/src/server.rs:30009:13 False  [read: the first operand of an || continued on the next line: not the failing arm]  whole_body.contains(r#""reason":"closed""#)
GAP test BRANCH crates/api/src/server.rs:30010:20 True  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test BRANCH crates/api/src/server.rs:30010:20 False  || whole_body.contains(r#""reason":"unmeasured""#),
GAP test BRANCH crates/api/src/server.rs:31178:13 False  let pull::vendor::Transport::Http(dhan) = pull::vendor::Feed::Dhan.descriptor().transport
GAP test BRANCH crates/api/src/server.rs:31242:13 False  let pull::vendor::Transport::Http(spec) =
GAP test BRANCH crates/api/src/server.rs:32125:17 False  let Some(cap) = spec.window_cap_days(pull::vendor::Granularity::Minute1) else {
GAP test BRANCH crates/api/src/server.rs:34439:24 False  if let Some(bytes) = answer {
GAP test BRANCH crates/api/src/server.rs:34915:17 False  record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/server.rs:34916:24 False  && crate::emitted::says(record, "instrument", "NIFTYWAVES")
GAP test BRANCH crates/api/src/server.rs:35023:13 False  let pull::vendor::Transport::Http(shipped) =
GAP test BRANCH crates/api/src/server.rs:35203:17 False  record.level == telemetry::Level::Warn
GAP test BRANCH crates/api/src/server.rs:35204:24 False  && crate::emitted::says(record, "instrument", "NSE:NIFTY")
GAP test BRANCH crates/api/src/server.rs:35220:17 False  record.level == telemetry::Level::Info
GAP test BRANCH crates/api/src/server.rs:35221:24 False  && crate::emitted::counts(record, "days", 25)
GAP test BRANCH crates/api/src/server.rs:35222:24 False  && crate::emitted::counts(record, "instruments", 208)
GAP test BRANCH crates/api/src/server.rs:35960:17 False  crate::emitted::counts(record, "instruments", 0)
GAP prod BRANCH crates/api/src/server.rs:36788:9 True  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let Some(vendor) = named else {
GAP prod BRANCH crates/api/src/server.rs:37588:13 False  let Ok(position) = u16::try_from(index) else {
GAP prod BRANCH crates/api/src/server.rs:37595:13 False  let Some(def) = vocab::table::definition(position) else {
GAP prod LINE crates/api/src/sources.rs:217  return Self::gone(why);
GAP prod LINE crates/api/src/sources.rs:706  Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:708  open.file.path().display()
GAP prod LINE crates/api/src/sources.rs:710  });
GAP prod LINE crates/api/src/sources.rs:721  Refusal::store(format!(
GAP prod LINE crates/api/src/sources.rs:723  file.path().display()
GAP prod LINE crates/api/src/sources.rs:725  })
GAP prod LINE crates/api/src/sources.rs:846  }
GAP prod LINE crates/api/src/sources.rs:1100  Self::Unexpected => "unexpected",
GAP prod LINE crates/api/src/sources.rs:1428  Sessions::default()
GAP prod LINE crates/api/src/sources.rs:1767  "unexpected"
GAP prod LINE crates/api/src/sources.rs:2044  }
GAP prod LINE crates/api/src/sources.rs:2267  notes.push(format!(
GAP prod LINE crates/api/src/sources.rs:2268  "no offered feed holds {} at {} or a finer rung that folds to it",
GAP prod LINE crates/api/src/sources.rs:2269  asked.inst.key(),
GAP prod LINE crates/api/src/sources.rs:2270  asked.rung.as_str()
GAP prod LINE crates/api/src/sources.rs:2271  ));
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
GAP prod LINE crates/api/src/sources.rs:3856  (true, false) => tally.zerodha_only += 1,
GAP prod LINE crates/api/src/sources.rs:4108  flags.push("open_mismatch");
GAP prod LINE crates/api/src/sources.rs:4117  flags.push("gdfl_extremes_outside_day");
GAP prod BRANCH crates/api/src/sources.rs:216:12 True  if pull::gdfl_lineage::is_contract_refused(&why) {
GAP prod BRANCH crates/api/src/sources.rs:264:12 False  && inst.contract.is_some()
GAP prod BRANCH crates/api/src/sources.rs:841:16 False  if !pending.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:983:51 False  pull::calendar::DayKind::Open(session) if session.count > 0 => session
GAP prod BRANCH crates/api/src/sources.rs:1416:48 False  if rung.rung == Timeframe::SECOND_1 && rung.kind == Kind::Stored {
GAP prod BRANCH crates/api/src/sources.rs:1509:8 True  if !status.is_success() || answer.get("status").and_then(Value::as_str) != Some("ok") {
GAP prod BRANCH crates/api/src/sources.rs:1543:16 False  whole: !truncated && u64::try_from(days.len()).is_ok_and(|named| named == count),
GAP prod BRANCH crates/api/src/sources.rs:1573:25 False  if derived.whole && minutes.whole {
GAP prod BRANCH crates/api/src/sources.rs:1695:16 False  if uncounted.len() < MAX_NAMED {
GAP prod BRANCH crates/api/src/sources.rs:1739:37 True  (!missing.is_empty() || days.is_some_and(|d| d > 0) || sessions_short).then(|| {
GAP prod BRANCH crates/api/src/sources.rs:1765:40 True  held.kind == Kind::Derived && (sessions.not_held.is_none() || minute.not_held.is_none());
GAP prod BRANCH crates/api/src/sources.rs:1766:20 True  let state = if held.kind == Kind::Unexpected {
GAP prod BRANCH crates/api/src/sources.rs:1770:31 True  } else if !census_held || uncounted_count > 0 {
GAP prod BRANCH crates/api/src/sources.rs:2036:16 False  if !held.is_empty() {
GAP prod BRANCH crates/api/src/sources.rs:2129:20 False  if let Some(i) = at.get_mut(n)
GAP prod BRANCH crates/api/src/sources.rs:2266:8 True  if sources.is_empty() {
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
GAP prod BRANCH crates/api/src/sources.rs:3884:19 False  shares && bases == Some(true) && factor_pair.is_some_and(|(g, z)| basis_differs(g, z));
GAP prod BRANCH crates/api/src/sources.rs:3891:40 False  .map(|(before, now)| shares && bases == Some(true) && factor_stepped(before, now));
GAP prod BRANCH crates/api/src/sources.rs:3955:35 True  .is_some_and(|(d, g)| g.bar.high > d.high || g.bar.low < d.low);
GAP prod BRANCH crates/api/src/sources.rs:4071:17 False  scaled > 0 && (zv * zc - scaled).abs() * 1_000 > VOLUME_TOLERANCE_PERMILLE * scaled
GAP prod BRANCH crates/api/src/sources.rs:4087:17 False  gc > 0 && zg > (gg * zc + gc - 1) / gc
GAP prod BRANCH crates/api/src/sources.rs:4107:8 True  if open_mismatch {
GAP prod BRANCH crates/api/src/sources.rs:4116:8 True  if gdfl_outside {
GAP prod BRANCH crates/api/src/sources.rs:4130:22 False  if minute_off && !index {
GAP test BRANCH crates/api/src/sources_tests.rs:2335:25 False  [inside a closure whose false side does not fail the check]  .any(|note| note.starts_with("cannot read ") && note.contains("SYNTHA")),
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
GAP prod BRANCH crates/api/src/speed.rs:640:20 False  if let Some((figures, window_ms)) = figures
GAP prod BRANCH crates/api/src/speed.rs:755:16 True  if let Err(why) = spawned {
GAP prod BRANCH crates/api/src/speed.rs:771:16 False  if let Some(table) = TABLE.get() {
GAP prod BRANCH crates/api/src/speed.rs:894:24 True  if term.recv().await.is_none() {
GAP prod BRANCH crates/api/src/speed.rs:894:24 False  if term.recv().await.is_none() {
GAP test LINE crates/api/src/store_wire.rs:477  [read: a builder closure inside the condition, never called because the cache refused first: not the failing arm]  "[]".into()
GAP test LINE crates/api/src/store_wire.rs:522  || -> Result<u64, String> { panic!("derived twice") },
GAP prod LINE crates/api/src/topjson.rs:137  return refusal(
GAP prod LINE crates/api/src/topjson.rs:139  "top report exceeds the bounded JSON response size; no prefix was exposed",
GAP prod LINE crates/api/src/topjson.rs:180  return Ok(cli::render_top_record(
GAP prod LINE crates/api/src/topjson.rs:181  &row,
GAP prod LINE crates/api/src/topjson.rs:182  &[],
GAP prod LINE crates/api/src/topjson.rs:183  None,
GAP prod LINE crates/api/src/topjson.rs:184  "No committed result-set receipt exists; no orphan frontier rows are displayed.",
GAP prod LINE crates/api/src/topjson.rs:185  ));
GAP prod LINE crates/api/src/topjson.rs:194  return Err(why);
GAP test LINE crates/api/src/topjson.rs:609  |_| Ok(()),
GAP prod BRANCH crates/api/src/topjson.rs:136:16 True  if body.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/topjson.rs:179:9 False  let Some(receipt) = receipt else {
GAP prod BRANCH crates/api/src/topjson.rs:193:12 True  if let Some(why) = partial {
GAP prod LINE crates/api/src/trades.rs:350  Err(unreadable) => {
GAP prod LINE crates/api/src/trades.rs:351  return refuse(
GAP prod LINE crates/api/src/trades.rs:352  json,
GAP prod LINE crates/api/src/trades.rs:353  &format!(
GAP prod LINE crates/api/src/trades.rs:354  "{why}. {} could not be checked for absence: {unreadable}",
GAP prod LINE crates/api/src/trades.rs:355  child.display()
GAP prod LINE crates/api/src/trades.rs:356  ),
GAP prod BRANCH crates/api/src/trades.rs:340:25 False  Err(missing) if missing.kind() == std::io::ErrorKind::NotFound => {}
GAP test BRANCH crates/api/src/trades.rs:1122:16 False  if let Some(byte) = damaged.get_mut(40) {
GAP test BRANCH crates/api/src/trades.rs:1187:16 False  if let Some(byte) = damaged.get_mut(40) {
GAP prod LINE crates/api/src/verify.rs:326  crate::speed::Outcome::Cancelled
GAP prod BRANCH crates/api/src/verify.rs:321:26 True  let outcome = if report
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:460:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:496:28 False  |k: &EntryKey| k.timeframe == Timeframe::DAY_1 && matches!(k.month.month(), 1 | 3);
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:503:13 False  let Gate::RungFirst {
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:612:13 False  let Gate::RungFirst {
GAP test BRANCH crates/api/tests/cloud_mutants_bars.rs:1487:30 False  [inside a closure whose false side does not fail the check]  .any(|fault| fault.contains("2025-05") && fault.contains("another writer holds")),
`````

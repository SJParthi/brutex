# gaps-AS2.txt.md

Source: this shard's Route B whole:api readout of tip `1fbb5ad6` (`d3-route-b-cloud.sh`, the kit's set file, this Linux host), classified as its first lines say. One fenced block below holds it.

`````text
BASE-COV-API-AS2 (repair round 3, shard G2), Route B on tip 1fbb5ad66048cdf9899448c4b9bc3c53833c2ce5, whole:api (the kit's set file deep-BASE-COV-API-6ea46e59c074-set.txt, 205 coverage files), host Linux x86_64, root, 4 cores; test api rc=101 END 2026-11-07 22:51:33 IST (7 min); tests passed 3195 failed 16 ignored 16
NOT PROVED lines of the run: 23 files, every one of them crates/api/benches/** or crates/api/src/lib.rs (no object holds them: L-BCA-2)
TOTAL (whole api, this host) lines 180320/184518 (4198 uncovered), branches 12946/15482 (2536 uncovered sides)
G2 (58 files of SHARDS.md's G2) lines 70425/71407 (982 uncovered), branches 4761/5494 (733 uncovered sides): (F) 174 lines 549 sides; (U) 123 lines 90 sides; GAP 685 lines 94 sides (production 524 lines 75 sides; test code 51 lines 7 sides; inside #[ignore]d tests 110 lines 12 sides)
Classes: a row whose line the predecessor's lists (tip b3d5321c) classed keeps that class, its line mapped through git diff; (U) rows carry their proof (fu-AS2.txt); every other row was classed by recipe v4 step 4 (test code only: a panic/unreachable line, a message argument of an assert whose statement ran, the false side of an && operand inside an assert whose line ran, the false arm of a matches! inside an assert whose statement ran (as the previous lists read the same rows), the else of a let-else that panics whose line ran) and read by hand in session -5. Other groups' files were measured in the same run and are their shards' to list.
per file (lines a/b, branches c/d; GAP lines/sides):
  crates/api/src/server.rs: lines 19298/19596, branches 1243/1466; GAP 190/20
  crates/api/src/indexstoprankingjson.rs: lines 308/502, branches 24/48; GAP 194/21
  crates/api/src/sources.rs: lines 2759/2766, branches 532/542; GAP 0/0
  crates/api/src/sources_day_truth_tests.rs: lines 596/596, branches 7/10; GAP 0/0
  crates/api/src/sources_tests.rs: lines 2802/2804, branches 9/12; GAP 0/0
  crates/api/src/provenance.rs: lines 1160/1171, branches 158/166; GAP 0/0
  crates/api/src/provenance_tests.rs: lines 2204/2210, branches 28/50; GAP 0/0
  crates/api/src/booleanqualification_projection.rs: lines 44/143, branches 6/12; GAP 99/6
  crates/api/src/booleanqualification_projection_tests.rs: lines 71/71, branches 0/0; GAP 0/0
  crates/api/src/bars_sort.rs: lines 3791/3813, branches 447/488; GAP 5/2
  crates/api/src/bars_sort_tests.rs: lines 1263/1266, branches 73/112; GAP 0/0
  crates/api/src/census_cache.rs: lines 1799/1843, branches 169/194; GAP 17/3
  crates/api/src/calendar_of.rs: lines 1999/2038, branches 118/136; GAP 34/6
  crates/api/src/calendar_route.rs: lines 876/879, branches 96/100; GAP 1/0
  crates/api/src/calendar_route_tests.rs: lines 1167/1169, branches 33/60; GAP 0/0
  crates/api/src/booleanlaunch.rs: lines 376/393, branches 85/88; GAP 17/2
  crates/api/src/booleanlaunch_tests.rs: lines 711/711, branches 18/22; GAP 0/0
  crates/api/src/expressionsearchjson.rs: lines 148/181, branches 18/36; GAP 33/18
  crates/api/src/disk_route.rs: lines 1413/1418, branches 143/156; GAP 3/0
  crates/api/src/disk_route_seal_tests.rs: lines 631/631, branches 10/14; GAP 0/0
  crates/api/src/operation_audit.rs: lines 223/223, branches 45/46; GAP 0/0
  crates/api/src/operation_audit_tests.rs: lines 781/782, branches 6/8; GAP 0/0
  crates/api/src/speed.rs: lines 551/557, branches 76/82; GAP 4/2
  crates/api/src/speed_tests.rs: lines 1002/1003, branches 41/70; GAP 0/0
  crates/api/src/dayflags_rules.rs: lines 940/941, branches 186/188; GAP 1/1
  crates/api/src/dayflags_rules_tests.rs: lines 3663/3712, branches 52/78; GAP 18/0
  crates/api/src/trades.rs: lines 1061/1073, branches 40/46; GAP 0/0
  crates/api/src/booleanjson.rs: lines 324/326, branches 41/42; GAP 0/0
  crates/api/src/booleanjson_tests.rs: lines 679/679, branches 12/14; GAP 0/0
  crates/api/src/calendar_record.rs: lines 666/675, branches 86/92; GAP 0/0
  crates/api/src/calendar_record_tests.rs: lines 533/535, branches 17/22; GAP 0/0
  crates/api/src/scrub_route.rs: lines 1669/1673, branches 181/186; GAP 0/0
  crates/api/src/topjson.rs: lines 537/544, branches 24/28; GAP 6/1
  crates/api/src/quarantine_route.rs: lines 126/126, branches 12/12; GAP 0/0
  crates/api/src/quarantine_route_tests.rs: lines 563/564, branches 14/14; GAP 0/0
  crates/api/src/booleancampaignjson.rs: lines 109/111, branches 23/24; GAP 0/0
  crates/api/src/booleancampaignjson_tests.rs: lines 564/564, branches 10/10; GAP 0/0
  crates/api/src/recovery_control.rs: lines 386/386, branches 26/26; GAP 0/0
  crates/api/src/indexstopvixjson.rs: lines 271/278, branches 22/26; GAP 6/3
  crates/api/src/head_deadline_body_tests.rs: lines 635/694, branches 35/60; GAP 51/7
  crates/api/src/render.rs: lines 2654/2661, branches 245/278; GAP 1/1
  crates/api/tests/logs_r3_24.rs: lines 240/241, branches 10/10; GAP 0/0
  crates/api/src/verify.rs: lines 356/360, branches 29/30; GAP 0/0
  crates/api/src/master.rs: lines 654/661, branches 43/58; GAP 5/1
  crates/api/src/store_wire.rs: lines 426/429, branches 18/18; GAP 0/0
  crates/api/src/instruments_route.rs: lines 824/824, branches 48/64; GAP 0/0
  crates/api/src/fault_log.rs: lines 403/403, branches 39/40; GAP 0/0
  crates/api/src/server_cloud_mutant_tests_e.rs: lines 585/587, branches 11/12; GAP 0/0
  crates/api/tests/cloud_mutants_bars.rs: lines 1788/1791, branches 14/26; GAP 0/0
  crates/api/src/constituents.rs: lines 932/932, branches 55/62; GAP 0/0
  crates/api/src/boolean_observation_budget.rs: lines 79/79, branches 17/22; GAP 0/0
  crates/api/src/fno_boundary_tests.rs: lines 1295/1297, branches 28/32; GAP 0/0
  crates/api/src/bars_unknown_series_tests.rs: lines 516/516, branches 2/2; GAP 0/0
  crates/api/src/census_tail_tests.rs: lines 376/378, branches 4/8; GAP 0/0
  crates/api/tests/store_step_forms.rs: lines 50/50, branches 0/0; GAP 0/0
  crates/api/src/boolean_search_budget.rs: lines 47/47, branches 8/8; GAP 0/0
  crates/api/src/livejson.rs: lines 405/407, branches 22/36; GAP 0/0
  crates/api/src/research_doors_tests.rs: lines 96/97, branches 2/2; GAP 0/0

rows:
GAP prod LINE crates/api/src/bars_sort.rs:2075  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2145  continue;
GAP prod LINE crates/api/src/bars_sort.rs:2388  Err(why) => {
GAP prod LINE crates/api/src/bars_sort.rs:2389  faults.push_once(why);
GAP prod LINE crates/api/src/bars_sort.rs:2390  continue;
GAP prod BRANCH crates/api/src/bars_sort.rs:2074:13 False  let Ok(series) = u32::try_from(cut.series) else {
GAP prod BRANCH crates/api/src/bars_sort.rs:2144:13 False  let Ok(series) = u32::try_from(cut.series) else {
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
GAP ignored LINE crates/api/src/calendar_of.rs:2560  fn it_reproduces_the_operators_store() {
GAP ignored LINE crates/api/src/calendar_of.rs:2561  let Some(home) = std::env::var_os("HOME") else {
GAP ignored LINE crates/api/src/calendar_of.rs:2562  panic!("no HOME");
GAP ignored LINE crates/api/src/calendar_of.rs:2564  let root = std::path::PathBuf::from(home).join(".brutex/store");
GAP ignored LINE crates/api/src/calendar_of.rs:2565  assert!(
GAP ignored LINE crates/api/src/calendar_of.rs:2566  root.join("bars").is_dir(),
GAP ignored LINE crates/api/src/calendar_of.rs:2569  root.display()
GAP ignored LINE crates/api/src/calendar_of.rs:2572  let mut months = Vec::new();
GAP ignored LINE crates/api/src/calendar_of.rs:2573  for year in 2019..=2026_u16 {
GAP ignored LINE crates/api/src/calendar_of.rs:2574  for month in 1..=12_u8 {
GAP ignored LINE crates/api/src/calendar_of.rs:2575  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored LINE crates/api/src/calendar_of.rs:2576  months.push(ym);
GAP ignored LINE crates/api/src/calendar_of.rs:2577  }
GAP ignored LINE crates/api/src/calendar_of.rs:2581  let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
GAP ignored LINE crates/api/src/calendar_of.rs:2583  println!(
GAP ignored LINE crates/api/src/calendar_of.rs:2585  cal.span(),
GAP ignored LINE crates/api/src/calendar_of.rs:2586  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2589  report.unreadable.len()
GAP ignored LINE crates/api/src/calendar_of.rs:2592  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2593  cal.sessions(),
GAP ignored LINE crates/api/src/calendar_of.rs:2599  let mut owed = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2600  let mut unmeasured = 0_u32;
GAP ignored LINE crates/api/src/calendar_of.rs:2601  for day in cal.first_day()..=cal.last_day() {
GAP ignored LINE crates/api/src/calendar_of.rs:2602  match cal.expected_bars(day) {
GAP ignored LINE crates/api/src/calendar_of.rs:2603  Some(n) => owed = owed.saturating_add(u32::from(n)),
GAP ignored LINE crates/api/src/calendar_of.rs:2605  if matches!(
GAP ignored LINE crates/api/src/calendar_of.rs:2606  cal.kind_of(day),
GAP ignored LINE crates/api/src/calendar_of.rs:2608  ) {
GAP ignored LINE crates/api/src/calendar_of.rs:2609  unmeasured = unmeasured.saturating_add(1);
GAP ignored LINE crates/api/src/calendar_of.rs:2610  }
GAP ignored LINE crates/api/src/calendar_of.rs:2614  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2619  println!("minute bars owed: {owed}");
GAP ignored LINE crates/api/src/calendar_of.rs:2620  assert_eq!(
GAP ignored LINE crates/api/src/calendar_of.rs:2626  }
GAP prod BRANCH crates/api/src/calendar_of.rs:551:32 False  if let Some(civil) = day_month(day) {
GAP prod BRANCH crates/api/src/calendar_of.rs:827:35 True  Some(last) if last.1 == minute => {}
GAP ignored BRANCH crates/api/src/calendar_of.rs:2561:13 True  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2561:13 False  let Some(home) = std::env::var_os("HOME") else {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2575:24 True  if let Ok(ym) = YearMonth::new(year, month) {
GAP ignored BRANCH crates/api/src/calendar_of.rs:2575:24 False  if let Ok(ym) = YearMonth::new(year, month) {
GAP prod LINE crates/api/src/calendar_route.rs:1312  Err(_) => Err("the pull journal's walk at start unwound".to_owned()),
GAP prod LINE crates/api/src/census_cache.rs:1210  // A WARM THAT PANICKED leaves its rows as they were (`warming`,
GAP prod LINE crates/api/src/census_cache.rs:1211  // or the last state it reached), and readers read for themselves.
GAP prod LINE crates/api/src/census_cache.rs:1212  warn_line!("a census warm thread panicked; its caches are read by their first request");
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
GAP prod BRANCH crates/api/src/census_cache.rs:918:16 False  if let Some(row) = rows
GAP prod BRANCH crates/api/src/census_cache.rs:1209:12 True  if handle.join().is_err() {
GAP prod BRANCH crates/api/src/census_cache.rs:1287:8 False  if behind.is_empty() {
GAP prod LINE crates/api/src/dayflags_rules.rs:320  return Ok(kept);
GAP prod BRANCH crates/api/src/dayflags_rules.rs:319:16 True  if let Some(kept) = flags().get(&key) {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2715  fn the_day_band_is_timed_cold_warm_concurrent_and_growing() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2716  let _ = day_band_timed(&[1, 10, 100], [20, 200, 60], 300);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:2717  }
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3062  fn the_cold_auction_era_share_page_is_timed() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3063  let root = std::path::PathBuf::from(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3064  std::env::var_os("BRUTEX_DAYFLAGS_STORE")
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3065  .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3067  cold_share_pages_timed(
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3068  &root,
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3069  &[
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3070  ("NSE-CASH-DALBHARAT", "2026-08-10", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3071  ("NSE-CASH-RELIANCE", "2026-08-03", "2026-09-30"),
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3072  ],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3074  [20, 10],
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3076  }
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3296  fn the_day_band_pages_for_a_profiler() {
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3297  let _ = profiled_pages("r3-13-profile", 10, &TIMED);
GAP ignored LINE crates/api/src/dayflags_rules_tests.rs:3298  }
GAP ignored LINE crates/api/src/disk_route.rs:2456  fn the_call_at_its_p99_costs_less_than_the_df_child_at_its_p50() {
GAP ignored LINE crates/api/src/disk_route.rs:2457  timed_against_the_child("disk-route-timing", 1_000, 1);
GAP ignored LINE crates/api/src/disk_route.rs:2458  }
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
GAP test LINE crates/api/src/head_deadline_body_tests.rs:304  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:305  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:306  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:307  took >= T * 4,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:310  let said = drain(&mut client, T * 10).await;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:311  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:312  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:518  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:519  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:520  });
GAP test LINE crates/api/src/head_deadline_body_tests.rs:543  assert!(lines(from, "response cut", "test=pauses-under").is_empty());
GAP test LINE crates/api/src/head_deadline_body_tests.rs:544  assert_eq!(lines(from, "response cut", "test=pause-over").len(), 1);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:572  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:573  let said = drain(&mut client, T * 20).await;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:574  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:575  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test LINE crates/api/src/head_deadline_body_tests.rs:626  fn unchunk(mut framed: &[u8]) -> Vec<u8> {
GAP test LINE crates/api/src/head_deadline_body_tests.rs:627  let mut body = Vec::new();
GAP test LINE crates/api/src/head_deadline_body_tests.rs:629  let line = framed
GAP test LINE crates/api/src/head_deadline_body_tests.rs:630  .windows(2)
GAP test LINE crates/api/src/head_deadline_body_tests.rs:631  .position(|w| w == b"\r\n")
GAP test LINE crates/api/src/head_deadline_body_tests.rs:632  .expect("a chunk size line");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:633  let size = usize::from_str_radix(std::str::from_utf8(&framed[..line]).unwrap(), 16)
GAP test LINE crates/api/src/head_deadline_body_tests.rs:634  .expect("a hex chunk size");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:635  framed = &framed[line + 2..];
GAP test LINE crates/api/src/head_deadline_body_tests.rs:636  if size == 0 {
GAP test LINE crates/api/src/head_deadline_body_tests.rs:637  assert_eq!(framed, b"\r\n", "the last chunk ends the body");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:638  return body;
GAP test LINE crates/api/src/head_deadline_body_tests.rs:639  }
GAP test LINE crates/api/src/head_deadline_body_tests.rs:640  body.extend_from_slice(&framed[..size]);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:641  assert_eq!(&framed[size..size + 2], b"\r\n");
GAP test LINE crates/api/src/head_deadline_body_tests.rs:642  framed = &framed[size + 2..];
GAP test LINE crates/api/src/head_deadline_body_tests.rs:644  }
GAP test LINE crates/api/src/head_deadline_body_tests.rs:685  let body = unchunk(&framed);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:686  assert_eq!(body.len(), length);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:687  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:688  assert!(began.elapsed() >= T * 4);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:797  assert_pattern(&body);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:798  let done = began.elapsed();
GAP test LINE crates/api/src/head_deadline_body_tests.rs:799  drop(client);
GAP test LINE crates/api/src/head_deadline_body_tests.rs:800  done
GAP test LINE crates/api/src/head_deadline_body_tests.rs:801  }));
GAP test LINE crates/api/src/head_deadline_body_tests.rs:815  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:816  answered + EARLY >= first_done,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:819  assert!(
GAP test LINE crates/api/src/head_deadline_body_tests.rs:820  first_done >= T * 4,
GAP test LINE crates/api/src/head_deadline_body_tests.rs:996  Poll::Pending => panic!("the stall deadline passed and the write still waited"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:67:12 True  if self.sent >= self.total {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:312:9 True  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:312:48 True  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:575:9 True  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:575:48 True  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:636:12 True  if size == 0 {
GAP test BRANCH crates/api/src/head_deadline_body_tests.rs:636:12 False  if size == 0 {
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
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:107:16 True  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:107:16 False  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod BRANCH crates/api/src/indexstopvixjson.rs:151:16 False  && cached.identity == asked.identity
GAP prod LINE crates/api/src/master.rs:357  return Err(format!(
GAP prod LINE crates/api/src/master.rs:358  "{}: grew past {MAX_MASTER_BYTES} bytes while it was read; this \
GAP prod LINE crates/api/src/master.rs:359  reader holds at most {MAX_MASTER_BYTES}",
GAP prod LINE crates/api/src/master.rs:360  path.display()
GAP prod LINE crates/api/src/master.rs:361  ));
GAP prod BRANCH crates/api/src/master.rs:356:8 True  if text.len() as u64 > MAX_MASTER_BYTES {
GAP prod LINE crates/api/src/render.rs:3284  continue;
GAP prod BRANCH crates/api/src/render.rs:3283:13 False  let Ok(kind) = e.file_type() else {
GAP prod LINE crates/api/src/server.rs:6435  break;
GAP prod LINE crates/api/src/server.rs:6633  return Err(why);
GAP prod LINE crates/api/src/server.rs:6864  return Err(match settled {
GAP prod LINE crates/api/src/server.rs:6865  Err(why) if asked.window.to() == today => format!("{refusal} Today: {why}."),
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
GAP prod LINE crates/api/src/server.rs:10114  return (
GAP prod LINE crates/api/src/server.rs:10115  axum::http::StatusCode::CONFLICT,
GAP prod LINE crates/api/src/server.rs:10116  json_headers(),
GAP prod LINE crates/api/src/server.rs:10117  run_refusal_json(&crate::pullrun::Refusal::AlreadyRunning.why()),
GAP prod LINE crates/api/src/server.rs:10118  );
GAP prod LINE crates/api/src/server.rs:10594  Err(why) => (
GAP prod LINE crates/api/src/server.rs:10595  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/server.rs:10596  refusal_html("Spot pull", &why),
GAP prod LINE crates/api/src/server.rs:10597  ),
GAP prod LINE crates/api/src/server.rs:11993  Ok(chain) => fno_report(&page, facts, &chain, asked, site, &wire).await,
GAP prod LINE crates/api/src/server.rs:14899  Err(why) => (
GAP prod LINE crates/api/src/server.rs:14900  axum::http::StatusCode::INTERNAL_SERVER_ERROR,
GAP prod LINE crates/api/src/server.rs:14901  refusal_html("Expired F&O pull", &why),
GAP prod LINE crates/api/src/server.rs:14902  ),
GAP prod LINE crates/api/src/server.rs:18323  Poll::Ready(Err(why)) => {
GAP prod LINE crates/api/src/server.rs:18324  let _left = self.report(
GAP prod LINE crates/api/src/server.rs:18325  telemetry::Level::Info,
GAP prod LINE crates/api/src/server.rs:18326  "response closed by the client",
GAP prod LINE crates/api/src/server.rs:18327  &format!("a write failed: {}", why.kind()),
GAP prod LINE crates/api/src/server.rs:18329  Poll::Ready(Err(why))
GAP prod LINE crates/api/src/server.rs:20811  Err(why) => {
GAP prod LINE crates/api/src/server.rs:20812  warn_line!(
GAP prod LINE crates/api/src/server.rs:20815  return FAILED;
GAP prod LINE crates/api/src/server.rs:20856  Err(why) => {
GAP prod LINE crates/api/src/server.rs:20857  warn_line!(
GAP prod LINE crates/api/src/server.rs:20861  return FAILED;
GAP prod LINE crates/api/src/server.rs:20935  Loading::Failed(why) => return load_failed(&why),
GAP prod LINE crates/api/src/server.rs:20980  say!("  NO WRITES: {why}");
GAP prod LINE crates/api/src/server.rs:20983  say!(
GAP prod LINE crates/api/src/server.rs:20984  "  autopilot: starting in {}s — http://{addr}/autopilot.json",
GAP prod LINE crates/api/src/server.rs:20985  autopilot::GRACE_SECS
GAP prod LINE crates/api/src/server.rs:20986  );
GAP prod LINE crates/api/src/server.rs:21052  Ok(()) => say!("  opening: {home}"),
GAP prod LINE crates/api/src/server.rs:21067  warn_line!("Calendars NOT warmed at start: {why}");
GAP prod LINE crates/api/src/server.rs:21073  warn_line!("Census caches NOT warmed at start: {why}");
GAP ignored LINE crates/api/src/server.rs:22114  fn pull_runtime_costs() {
GAP ignored LINE crates/api/src/server.rs:22115  costs_of_the_pull_runtime("pull-runtime-costs-store", 200, 100_000, 8_000);
GAP ignored LINE crates/api/src/server.rs:22116  }
GAP ignored LINE crates/api/src/server.rs:22337  fn the_pull_order_on_a_real_census() {
GAP ignored LINE crates/api/src/server.rs:22338  let masters = std::env::var("BRUTEX_WP62_MASTERS")
GAP ignored LINE crates/api/src/server.rs:22339  .expect("BRUTEX_WP62_MASTERS names the masters directory");
GAP ignored LINE crates/api/src/server.rs:22340  let store =
GAP ignored LINE crates/api/src/server.rs:22341  std::env::var("BRUTEX_WP62_STORE").expect("BRUTEX_WP62_STORE names a shadow store");
GAP ignored LINE crates/api/src/server.rs:22342  let site = Site::load(Path::new(&masters), Path::new(&store));
GAP ignored LINE crates/api/src/server.rs:22343  println!(
GAP ignored LINE crates/api/src/server.rs:22345  site.restart.word(),
GAP ignored LINE crates/api/src/server.rs:22346  site.restart.words()
GAP ignored LINE crates/api/src/server.rs:22348  let censuses = census::read_all(&site.store_root);
GAP ignored LINE crates/api/src/server.rs:22349  for target in ["indices", "fno"] {
GAP ignored LINE crates/api/src/server.rs:22350  let asked = ingest::parse_spot(
GAP ignored LINE crates/api/src/server.rs:22351  &format!(
GAP ignored LINE crates/api/src/server.rs:22352  "vendor=zerodha&target={target}&cash_identity=zerodha_nse_checked\
GAP ignored LINE crates/api/src/server.rs:22353  &granularity=1min&from=2018-01-01&to=2026-09-30"
GAP ignored LINE crates/api/src/server.rs:22354  ),
GAP ignored LINE crates/api/src/server.rs:22355  day(2026, 10, 12),
GAP ignored LINE crates/api/src/server.rs:22357  .expect("the operator's own window");
GAP ignored LINE crates/api/src/server.rs:22358  let mut targets = spot_targets(&asked, &site);
GAP ignored LINE crates/api/src/server.rs:22359  targets.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22360  let asked_for = targets.len();
GAP ignored LINE crates/api/src/server.rs:22361  assert!(asked_for > 0, "{target}: the masters name no instrument");
GAP ignored LINE crates/api/src/server.rs:22362  let whole = ladder_refusal(&asked, &targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22363  let began = std::time::Instant::now();
GAP ignored LINE crates/api/src/server.rs:22364  let gate = order_gate(&asked, &mut targets, &censuses);
GAP ignored LINE crates/api/src/server.rs:22365  let took = began.elapsed();
GAP ignored LINE crates/api/src/server.rs:22366  let whole = whole.unwrap_or_else(|| {
GAP ignored LINE crates/api/src/server.rs:22367  panic!("{target}: the whole-run gate is open, so this store shows nothing")
GAP ignored LINE crates/api/src/server.rs:22369  println!(
GAP ignored LINE crates/api/src/server.rs:22371  whole.chars().take(160).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22373  let withheld = gate
GAP ignored LINE crates/api/src/server.rs:22374  .unwrap_or_else(|why| panic!("{target}: still refused whole in {took:?}: {why}"));
GAP ignored LINE crates/api/src/server.rs:22375  println!(
GAP ignored LINE crates/api/src/server.rs:22377  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22378  targets.len()
GAP ignored LINE crates/api/src/server.rs:22380  assert!(
GAP ignored LINE crates/api/src/server.rs:22381  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored LINE crates/api/src/server.rs:22383  withheld.len()
GAP ignored LINE crates/api/src/server.rs:22385  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22386  targets.len() + withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22390  let mut named: Vec<_> = withheld.iter().map(|(key, _)| *key).collect();
GAP ignored LINE crates/api/src/server.rs:22391  named.sort_unstable();
GAP ignored LINE crates/api/src/server.rs:22392  named.dedup();
GAP ignored LINE crates/api/src/server.rs:22393  assert_eq!(
GAP ignored LINE crates/api/src/server.rs:22394  named.len(),
GAP ignored LINE crates/api/src/server.rs:22395  withheld.len(),
GAP ignored LINE crates/api/src/server.rs:22398  for (key, why) in withheld {
GAP ignored LINE crates/api/src/server.rs:22399  assert!(
GAP ignored LINE crates/api/src/server.rs:22400  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored LINE crates/api/src/server.rs:22404  println!(
GAP ignored LINE crates/api/src/server.rs:22407  why.chars().take(150).collect::<String>()
GAP ignored LINE crates/api/src/server.rs:22411  }
GAP prod LINE crates/api/src/server.rs:37571  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let feed = vendor.as_str().to_owned();
GAP prod LINE crates/api/src/server.rs:37587  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let owned = {
GAP prod LINE crates/api/src/server.rs:37588  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let held = site.universe();
GAP prod LINE crates/api/src/server.rs:37589  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  resolved_master_rows(&held.read.merged, vendor)
GAP prod LINE crates/api/src/server.rs:37591  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let master: Vec<pull::universe::VendorInstrument<'_>> = owned
GAP prod LINE crates/api/src/server.rs:37592  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .iter()
GAP prod LINE crates/api/src/server.rs:37593  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .map(|(id, symbol, isin)| pull::universe::VendorInstrument {
GAP prod LINE crates/api/src/server.rs:37594  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  vendor_id: id,
GAP prod LINE crates/api/src/server.rs:37595  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  trading_symbol: symbol,
GAP prod LINE crates/api/src/server.rs:37596  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  isin,
GAP prod LINE crates/api/src/server.rs:37597  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  })
GAP prod LINE crates/api/src/server.rs:37598  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .collect();
GAP prod LINE crates/api/src/server.rs:37600  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let source = match pull::resolve::HttpDocuments::new() {
GAP prod LINE crates/api/src/server.rs:37601  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Ok(client) => client,
GAP prod LINE crates/api/src/server.rs:37602  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Err(why) => {
GAP prod LINE crates/api/src/server.rs:37603  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  return (
GAP prod LINE crates/api/src/server.rs:37604  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:37605  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:37606  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  format!(r#"{{"ok":false,"why":{}}}"#, render::json_string(&why)),
GAP prod LINE crates/api/src/server.rs:37607  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  );
GAP prod LINE crates/api/src/server.rs:37613  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let today = match crate::ingest::today_ist() {
GAP prod LINE crates/api/src/server.rs:37614  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Ok(day) => day,
GAP prod LINE crates/api/src/server.rs:37615  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  Err(why) => {
GAP prod LINE crates/api/src/server.rs:37616  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  return (
GAP prod LINE crates/api/src/server.rs:37617  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/server.rs:37618  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:37619  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  format!(
GAP prod LINE crates/api/src/server.rs:37620  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  r#"{{"ok":false,"why":{}}}"#,
GAP prod LINE crates/api/src/server.rs:37621  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  render::json_string(&why.to_string())
GAP prod LINE crates/api/src/server.rs:37622  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  ),
GAP prod LINE crates/api/src/server.rs:37623  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  );
GAP prod LINE crates/api/src/server.rs:37627  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let snap = pull::resolve::crawl(
GAP prod LINE crates/api/src/server.rs:37628  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &source,
GAP prod LINE crates/api/src/server.rs:37629  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  INDEX_HOST,
GAP prod LINE crates/api/src/server.rs:37630  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  today,
GAP prod LINE crates/api/src/server.rs:37631  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &feed,
GAP prod LINE crates/api/src/server.rs:37632  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // ASKED OF THE VENDOR, NOT ASSUMED -- and this line used to assume.
GAP prod LINE crates/api/src/server.rs:37633  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  //
GAP prod LINE crates/api/src/server.rs:37634  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // It was the literal `JoinKey::Isin`, under a comment reading "EVERY
GAP prod LINE crates/api/src/server.rs:37635  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // FEED IN THIS BUILD PUBLISHES AN ISIN. The symbol key exists for the
GAP prod LINE crates/api/src/server.rs:37636  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // one that does not, and asking the descriptor rather than assuming is
GAP prod LINE crates/api/src/server.rs:37637  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // what keeps that true when it lands." **It landed on 14 Aug 2026.**
GAP prod LINE crates/api/src/server.rs:37638  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // Zerodha's master has twelve columns and none is an ISIN, so this
GAP prod LINE crates/api/src/server.rs:37639  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // literal joined its rows on an EMPTY field -- and `universe::resolve`
GAP prod LINE crates/api/src/server.rs:37640  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // skips an empty key deliberately, so nothing would have errored. Every
GAP prod LINE crates/api/src/server.rs:37641  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // Zerodha instrument would have landed in `Verdict::Lacks`, which reads
GAP prod LINE crates/api/src/server.rs:37642  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // exactly like a vendor that lists nothing.
GAP prod LINE crates/api/src/server.rs:37643  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  //
GAP prod LINE crates/api/src/server.rs:37644  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // `JoinKey::for_vendor` derives it from `master_columns().isin`, which
GAP prod LINE crates/api/src/server.rs:37645  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // already spells Zerodha's as empty, so there is no second table to
GAP prod LINE crates/api/src/server.rs:37646  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  // fall behind the first.
GAP prod LINE crates/api/src/server.rs:37647  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  pull::universe::JoinKey::for_vendor(vendor),
GAP prod LINE crates/api/src/server.rs:37648  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  &master,
GAP prod LINE crates/api/src/server.rs:37649  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  )
GAP prod LINE crates/api/src/server.rs:37650  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  .await;
GAP prod LINE crates/api/src/server.rs:37652  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let verdict = snap.admits_publication(None);
GAP prod LINE crates/api/src/server.rs:37653  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  (
GAP prod LINE crates/api/src/server.rs:37654  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  axum::http::StatusCode::OK,
GAP prod LINE crates/api/src/server.rs:37655  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  json,
GAP prod LINE crates/api/src/server.rs:37656  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  universe_snapshot_json(&snap, &verdict),
GAP prod LINE crates/api/src/server.rs:37657  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  )
GAP prod BRANCH crates/api/src/server.rs:6434:12 True  if out.store_unavailable.is_some() {
GAP prod BRANCH crates/api/src/server.rs:6632:12 True  if let Some(why) = run.ledger.refused.clone() {
GAP prod BRANCH crates/api/src/server.rs:6855:68 True  if asked.window.to() > today || (asked.window.to() == today && !closed) {
GAP prod BRANCH crates/api/src/server.rs:6865:25 True  Err(why) if asked.window.to() == today => format!("{refusal} Today: {why}."),
GAP prod BRANCH crates/api/src/server.rs:6865:25 False  Err(why) if asked.window.to() == today => format!("{refusal} Today: {why}."),
GAP prod BRANCH crates/api/src/server.rs:10113:9 False  let Some(progress) = held.as_mut().filter(|progress| progress.generation == run) else {
GAP prod BRANCH crates/api/src/server.rs:18229:43 True  HeadState::Awaiting { .. } if entered > finished => self.deliver(),
GAP prod BRANCH crates/api/src/server.rs:20979:24 True  if let Some(why) = gate.unstamped {
GAP prod BRANCH crates/api/src/server.rs:20982:20 True  if flying_on_start {
GAP prod BRANCH crates/api/src/server.rs:21066:24 True  if let Err(why) = calendar_route::warm_at_start(&site) {
GAP prod BRANCH crates/api/src/server.rs:21072:24 True  if let Err(why) = census_cache::start(&site, &calendar_route::offered_feeds()) {
GAP ignored BRANCH crates/api/src/server.rs:22381:17 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22381:17 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22381:41 True  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22381:41 False  !withheld.is_empty() && withheld.len() < asked_for,
GAP ignored BRANCH crates/api/src/server.rs:22400:21 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22400:21 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22400:59 True  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP ignored BRANCH crates/api/src/server.rs:22400:59 False  why.starts_with(WITHHELD_BY_ORDER) && !targets.contains(&key),
GAP prod BRANCH crates/api/src/server.rs:37554:9 True  [L-BCA-6: universe_resolve ends at a real host (Parameter Store or the index host); the operator's seam decision]  let Some(vendor) = named else {
GAP prod LINE crates/api/src/speed.rs:586  return Handle(Arc::clone(row));
GAP prod LINE crates/api/src/speed.rs:888  Ok::<(), std::io::Error>(())
GAP prod LINE crates/api/src/speed.rs:896  }
GAP prod LINE crates/api/src/speed.rs:897  })
GAP prod BRANCH crates/api/src/speed.rs:585:16 True  if let Some(row) = rows.by_name.get(name) {
GAP prod BRANCH crates/api/src/speed.rs:894:24 False  if term.recv().await.is_none() {
GAP prod LINE crates/api/src/topjson.rs:180  return Ok(cli::render_top_record(
GAP prod LINE crates/api/src/topjson.rs:181  &row,
GAP prod LINE crates/api/src/topjson.rs:182  &[],
GAP prod LINE crates/api/src/topjson.rs:183  None,
GAP prod LINE crates/api/src/topjson.rs:184  "No committed result-set receipt exists; no orphan frontier rows are displayed.",
GAP prod LINE crates/api/src/topjson.rs:185  ));
GAP prod BRANCH crates/api/src/topjson.rs:179:9 False  let Some(receipt) = receipt else {
`````

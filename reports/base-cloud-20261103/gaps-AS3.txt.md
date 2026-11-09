# gaps-AS3.txt.md

One fenced block: shard G3's Route B figures on the code tip, and every row of a G3 file it leaves uncovered that
is no (F), (U) or (H) item of recipe step 4 (those are in `fu-AS3.txt.md`). A production GAP row owed outside this
shard says to what; a row marked "reachable, not done in this round" is G3's still. This host's figures are the
builder's: the Mac re-measures (Route B whole:api on its pin). Continuation -6 (9 Nov) measured the code tip
`c40fac07` and carried every row of continuation -5's measurement (`98908e77`) over through a line map from
`git diff 98908e77 c40fac07`, its kind, place and source text checked; the rows that did not carry over were
classified by hand (the two new test files' rows, and every remaining row of `booleanevidencejson.rs` and
`selectionv6json.rs`, whose earlier reason, L-BCA-5, no longer holds: a production launch writes their saved
artifacts in this round's tests).

`````text
BASE-COV-API-AS3 (shard G3, repair r3), Route B on tip c40fac07b7ab47af3f243f65158b430ab0cf4550, whole:api, log /var/tmp/as3/routeb8.log (this host: 4 cores, root; ROUTE-B DONE 2026-11-09 20:00:56 IST, exit 3: the NOT PROVED files below only)
set: the kit's deep-BASE-COV-API-6ea46e59c074-set.txt (whole:api) plus one `coverage <file> whole` line for each of the 10 crates/api/src files the branch added (215 coverage lines)
tests: cargo test -p api --locked --no-fail-fast -- --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held; test api rc=101 END 2026-11-09 19:41:45 IST (9 min); tests passed 3193 failed 13 ignored 16
files read 192; NOT PROVED (no object holds them) 23: crates/api/benches/ratio.rs crates/api/benches/ratio/basis.rs crates/api/benches/ratio/calendar.rs crates/api/benches/ratio/census.rs crates/api/benches/ratio/chain.rs crates/api/benches/ratio/dayflags.rs crates/api/benches/ratio/db_page.rs crates/api/benches/ratio/export.rs crates/api/benches/ratio/feed_checks.rs crates/api/benches/ratio/gaps.rs crates/api/benches/ratio/greeks.rs crates/api/benches/ratio/logs_dirs.rs crates/api/benches/ratio/months.rs crates/api/benches/ratio/options.rs crates/api/benches/ratio/provenance.rs crates/api/benches/ratio/reads.rs crates/api/benches/ratio/rollup.rs crates/api/benches/ratio/served.rs crates/api/benches/ratio/settings.rs crates/api/benches/ratio/sources.rs crates/api/benches/ratio/spot.rs crates/api/benches/ratio/window.rs crates/api/src/lib.rs
TOTAL (whole api) lines 180196/183615 (3419 uncovered), branches 13073/15412 (2339 uncovered sides)
G3 (58 files: the 48 SHARDS.md gives G3 and the 10 new test files): lines 64037/64569 (532 uncovered), branches 4756/5150 (394 uncovered sides)
G3 classified strictly: (F) 234 lines 314 sides; (U) 117 lines 59 sides; (H) 44 lines 2 sides; GAP 137 lines 19 sides:
  production, L-BCA-6 (the real public hosts or the credentialed dump): 18 lines 7 sides
  production, a failing medium or a cut append (no file a test lays out produces it): 3 lines 1 sides
  production, a race between two lock holds (no hook orders it): 4 lines 4 sides
  production, macOS only (a test for the Mac's Route B: selectionv6json.rs 175 False, 178, 180): 2 lines 1 sides
  production, reachable, not done in this round (indexstopqualificationjson.rs 237; indexstopcandlesjson.rs 117 True, 118, 120; booleanevidencejson.rs 209 False, 212, 214, 467): 6 lines 2 sides
  production, the host clock (no test may set the machine clock): 18 lines 4 sides
  in #[ignore]d tests (L-BCA-3, real data: the Mac): 86 lines 0 sides
  test code: 0 lines 0 sides
per file (lines a/b, branches c/d; (F), (U), (H) and GAP lines/sides):
  crates/api/src/autopilot.rs: lines 4464/4527, branches 326/386; (F) 34/51 (U) 11/5 (H) 0/0 GAP 18/4
  crates/api/src/bars_export.rs: lines 1625/1628, branches 106/112; (F) 1/6 (U) 1/0 (H) 0/0 GAP 1/0
  crates/api/src/bars_route.rs: lines 1868/1873, branches 157/160; (F) 5/3 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/bars_series.rs: lines 1852/1859, branches 270/276; (F) 4/2 (U) 3/4 (H) 0/0 GAP 0/0
  crates/api/src/bars_window_route_tests.rs: lines 1843/1847, branches 46/52; (F) 4/6 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/booleanadmission_projection.rs: lines 64/65, branches 7/8; (F) 0/0 (U) 1/1 (H) 0/0 GAP 0/0
  crates/api/src/booleanadmission_projection_tests.rs: lines 174/174, branches 8/8; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/booleanevidencejson.rs: lines 357/373, branches 38/42; (F) 0/0 (U) 13/3 (H) 0/0 GAP 3/1
  crates/api/src/booleanevidencejson_saved_tests.rs: lines 290/291, branches 9/12; (F) 0/2 (U) 1/1 (H) 0/0 GAP 0/0
  crates/api/src/booleanevidencejson_tests.rs: lines 457/457, branches 56/60; (F) 0/4 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/booleanlaunch_work.rs: lines 33/34, branches 0/0; (F) 0/0 (U) 1/0 (H) 0/0 GAP 0/0
  crates/api/src/census_request_tests.rs: lines 2280/2291, branches 50/72; (F) 11/22 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/census_rollup.rs: lines 330/331, branches 28/28; (F) 0/0 (U) 1/0 (H) 0/0 GAP 0/0
  crates/api/src/census_rollup_cover_tests.rs: lines 246/246, branches 0/0; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/dayflags.rs: lines 6124/6221, branches 700/720; (F) 7/9 (U) 7/8 (H) 0/0 GAP 83/3
  crates/api/src/dayflags_cover_tests.rs: lines 1183/1187, branches 4/8; (F) 4/4 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/dayflags_filter_tests.rs: lines 602/607, branches 6/6; (F) 5/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/feeds_route.rs: lines 2019/2079, branches 177/208; (F) 8/21 (U) 14/8 (H) 35/2 GAP 3/0
  crates/api/src/frontierjson.rs: lines 1032/1046, branches 40/50; (F) 2/8 (U) 12/2 (H) 0/0 GAP 0/0
  crates/api/src/gaps_route.rs: lines 3854/3868, branches 288/320; (F) 9/30 (U) 5/2 (H) 0/0 GAP 0/0
  crates/api/src/http_admission_tests.rs: lines 506/507, branches 12/12; (F) 1/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/imports.rs: lines 1870/1873, branches 201/206; (F) 1/2 (U) 0/2 (H) 0/0 GAP 2/1
  crates/api/src/indexstopcandlesjson.rs: lines 214/220, branches 36/38; (F) 0/0 (U) 1/0 (H) 0/0 GAP 5/2
  crates/api/src/indexstopjson.rs: lines 248/250, branches 31/32; (F) 0/0 (U) 2/1 (H) 0/0 GAP 0/0
  crates/api/src/indexstopjson_cover_tests.rs: lines 34/34, branches 0/0; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/indexstoplaunch.rs: lines 326/326, branches 136/136; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/indexstoplaunch_metadata.rs: lines 182/182, branches 37/38; (F) 0/0 (U) 0/1 (H) 0/0 GAP 0/0
  crates/api/src/indexstoplaunch_tests.rs: lines 913/913, branches 16/18; (F) 0/2 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/indexstopqualificationjson.rs: lines 509/513, branches 44/46; (F) 0/0 (U) 3/2 (H) 0/0 GAP 1/0
  crates/api/src/indexstopqualificationjson_saved_tests.rs: lines 526/531, branches 30/38; (F) 0/4 (U) 5/4 (H) 0/0 GAP 0/0
  crates/api/src/ingest.rs: lines 2145/2159, branches 141/154; (F) 13/12 (U) 1/1 (H) 0/0 GAP 0/0
  crates/api/src/ingest_cover_tests.rs: lines 185/191, branches 6/12; (F) 6/6 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/ingest_refusals.rs: lines 445/445, branches 30/30; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/ladder.rs: lines 698/703, branches 60/62; (F) 4/0 (U) 1/2 (H) 0/0 GAP 0/0
  crates/api/src/logs.rs: lines 7229/7254, branches 947/1002; (F) 23/50 (U) 2/5 (H) 0/0 GAP 0/0
  crates/api/src/logs_cover_tests.rs: lines 614/616, branches 3/4; (F) 2/1 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/logs_units_tests.rs: lines 173/174, branches 13/22; (F) 1/9 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/mastersrun.rs: lines 1358/1387, branches 74/88; (F) 4/6 (U) 7/1 (H) 0/0 GAP 18/7
  crates/api/src/month_change.rs: lines 267/267, branches 33/36; (F) 0/3 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/nse_identity.rs: lines 326/331, branches 15/20; (F) 5/5 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/ops_safety_tests.rs: lines 434/436, branches 20/26; (F) 2/6 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/optiongreeks.rs: lines 466/479, branches 54/56; (F) 0/0 (U) 12/2 (H) 0/0 GAP 1/0
  crates/api/src/optiongreeks_cover_tests.rs: lines 336/336, branches 6/6; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/owed_tests.rs: lines 332/334, branches 17/22; (F) 2/5 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/pool_table_tests.rs: lines 1151/1169, branches 32/38; (F) 18/6 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/selectionv6json.rs: lines 161/163, branches 15/16; (F) 0/0 (U) 0/0 (H) 0/0 GAP 2/1
  crates/api/src/selectionv6json_saved_tests.rs: lines 198/201, branches 3/4; (F) 0/0 (U) 3/1 (H) 0/0 GAP 0/0
  crates/api/src/selectionv6json_tests.rs: lines 182/182, branches 2/2; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/serve_edge_tests.rs: lines 262/262, branches 16/16; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/server_cloud_mutant_tests_b.rs: lines 1673/1694, branches 4/6; (F) 21/2 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/server_cloud_mutant_tests_c.rs: lines 2413/2427, branches 28/34; (F) 5/6 (U) 0/0 (H) 9/0 GAP 0/0
  crates/api/src/server_cloud_mutant_tests_d.rs: lines 596/599, branches 6/8; (F) 3/2 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/speed_route.rs: lines 47/54, branches 4/4; (F) 0/0 (U) 7/0 (H) 0/0 GAP 0/0
  crates/api/src/strict_sweep_tests.rs: lines 496/500, branches 8/8; (F) 4/0 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/sweepevidence.rs: lines 648/650, branches 51/52; (F) 0/0 (U) 2/1 (H) 0/0 GAP 0/0
  crates/api/src/sweeprun.rs: lines 4197/4222, branches 298/318; (F) 24/18 (U) 1/2 (H) 0/0 GAP 0/0
  crates/api/src/sweeprun_cover_tests.rs: lines 541/542, branches 9/10; (F) 1/1 (U) 0/0 (H) 0/0 GAP 0/0
  crates/api/src/sweeprun_status_cache_tests.rs: lines 439/439, branches 2/2; (F) 0/0 (U) 0/0 (H) 0/0 GAP 0/0

rows:
GAP prod BRANCH crates/api/src/autopilot.rs:2991:16 False  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  if let Some(day) = yesterday_ist(std::time::SystemTime::now()) {
GAP prod LINE crates/api/src/autopilot.rs:2993  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  }
GAP prod BRANCH crates/api/src/autopilot.rs:2994:13 False  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  let Some(saying) = clock_wait(clock_waits) else {
GAP prod BRANCH crates/api/src/autopilot.rs:2994:13 True  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  let Some(saying) = clock_wait(clock_waits) else {
GAP prod LINE crates/api/src/autopilot.rs:2994  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  let Some(saying) = clock_wait(clock_waits) else {
GAP prod LINE crates/api/src/autopilot.rs:2995  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  site.autopilot.publish(|status| {
GAP prod LINE crates/api/src/autopilot.rs:2996  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.phase = Phase::Halted;
GAP prod LINE crates/api/src/autopilot.rs:2997  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.detail = format!(
GAP prod LINE crates/api/src/autopilot.rs:3003  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  u64::from(CLOCK_WAITS) * IDLE_POLL_SECS / 60
GAP prod LINE crates/api/src/autopilot.rs:3005  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.due_unix = 0;
GAP prod LINE crates/api/src/autopilot.rs:3006  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  });
GAP prod LINE crates/api/src/autopilot.rs:3007  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  return;
GAP prod LINE crates/api/src/autopilot.rs:3009  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  site.autopilot.publish(move |status| {
GAP prod LINE crates/api/src/autopilot.rs:3010  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.phase = Phase::Halted;
GAP prod LINE crates/api/src/autopilot.rs:3011  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.detail = saying;
GAP prod LINE crates/api/src/autopilot.rs:3012  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.since_unix = ingest::epoch_secs(std::time::SystemTime::now());
GAP prod LINE crates/api/src/autopilot.rs:3013  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  status.due_unix = 0;
GAP prod LINE crates/api/src/autopilot.rs:3014  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  });
GAP prod LINE crates/api/src/autopilot.rs:3015  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  clock_waits = clock_waits.saturating_add(1);
GAP prod LINE crates/api/src/autopilot.rs:3016  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  tokio::time::sleep(std::time::Duration::from_secs(IDLE_POLL_SECS)).await;
GAP prod BRANCH crates/api/src/autopilot.rs:3638:9 False  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  let Some(yesterday) = yesterday_ist(std::time::SystemTime::now()) else {
GAP prod LINE crates/api/src/autopilot.rs:3639  [owed: the host clock: yesterday_ist(SystemTime::now()) is None only on a clock before the epoch or past 9999, and no test may set the machine clock]  return Pass::owed(IDLE_POLL_SECS);
GAP prod LINE crates/api/src/bars_export.rs:979  [owed: a race: file_days (731-764) errs in two cases. (a) The first page is the whole export but holds no row. That is impossible: first_page_refusal refuses rows == 0 (889), and Page.span is Some whenever rows >= 1 (Page::of 358-385). (b) An end read counts a different total than the first page. The totals come from the same plan, built from the same headers, series, months and filter, so sort, direction, offset and limit do not change them (window_admitted's tally runs before any seek, bars.rs:1724-1725). Case (b) therefore needs a writer to append to or replace a month between the first page read reader(0) (959) and the end_day reads (970-971).]  Err(refusal) => return *refusal,
GAP prod BRANCH crates/api/src/booleanevidencejson.rs:209:16 False  [reachable, not done in this round: a Boolean evidence page past detail::MAX_RESPONSE_BYTES (8 MiB). A page holds at most 256 rows, each of bounded size except a qualification row's fold list, whose length is the policy's fold minimums (min_decided_folds, min_profitable_oos_folds) capped by the later window's civil days (cli/src/boolean_qualification_plan.rs). A qualification page of 256 settings each carrying about 150 folds (a policy with fold minimums near 150 over a later window of at least 150 civil days, the saved launch otherwise) passes the ceiling; the saved launch of booleanevidencejson_saved_tests.rs carries six]  if response.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/booleanevidencejson.rs:212  [reachable, not done in this round: a Boolean evidence page past detail::MAX_RESPONSE_BYTES (8 MiB). A page holds at most 256 rows, each of bounded size except a qualification row's fold list, whose length is the policy's fold minimums (min_decided_folds, min_profitable_oos_folds) capped by the later window's civil days (cli/src/boolean_qualification_plan.rs). A qualification page of 256 settings each carrying about 150 folds (a policy with fold minimums near 150 over a later window of at least 150 civil days, the saved launch otherwise) passes the ceiling; the saved launch of booleanevidencejson_saved_tests.rs carries six]  refused(
GAP prod LINE crates/api/src/booleanevidencejson.rs:214  [reachable, not done in this round: a Boolean evidence page past detail::MAX_RESPONSE_BYTES (8 MiB). A page holds at most 256 rows, each of bounded size except a qualification row's fold list, whose length is the policy's fold minimums (min_decided_folds, min_profitable_oos_folds) capped by the later window's civil days (cli/src/boolean_qualification_plan.rs). A qualification page of 256 settings each carrying about 150 folds (a policy with fold minimums near 150 over a later window of at least 150 civil days, the saved launch otherwise) passes the ceiling; the saved launch of booleanevidencejson_saved_tests.rs carries six]  "Boolean evidence response exceeds its byte ceiling; no prefix returned",
GAP prod LINE crates/api/src/booleanevidencejson.rs:467  [reachable, not done in this round: a saved Romano-Wolf availability of 1 (measured). measure_returns (cli/src/boolean_statistics_numeric_v1.rs:102-131) measures the family only when no candidate's period returns are constant; in the saved launch some candidates never trade (all-zero returns), so its family is "unavailable-constant-return-candidate". A launch whose every candidate trades with returns that differ between periods writes code 1]  1 => Ok("measured"),
GAP prod BRANCH crates/api/src/dayflags.rs:3220:12 False  [owed: a race: a concurrent walk of the same pull journal that changed its identity or extent between this walk's two lock holds; no hook orders it without restructuring the path (D-6017)]  && tail.identity == walk.identity
GAP prod BRANCH crates/api/src/dayflags.rs:3322:16 False  [owed: a race: a concurrent walk of the same pull journal that changed its identity or extent between this walk's two lock holds; no hook orders it without restructuring the path (D-6017)]  && tail.identity == walk.identity
GAP prod BRANCH crates/api/src/dayflags.rs:3323:16 False  [owed: a race: a concurrent walk of the same pull journal that changed its identity or extent between this walk's two lock holds; no hook orders it without restructuring the path (D-6017)]  && tail.upto <= walk.upto
GAP ignored LINE crates/api/src/dayflags.rs:9739  fn the_real_days_are_labelled_and_the_answer_is_timed() {
GAP ignored LINE crates/api/src/dayflags.rs:9740  let root = PathBuf::from(
GAP ignored LINE crates/api/src/dayflags.rs:9741  std::env::var_os("BRUTEX_DAYFLAGS_STORE")
GAP ignored LINE crates/api/src/dayflags.rs:9742  .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
GAP ignored LINE crates/api/src/dayflags.rs:9744  let get = |instrument: &str, on: &str| -> Value {
GAP ignored LINE crates/api/src/dayflags.rs:9745  let got = answer(
GAP ignored LINE crates/api/src/dayflags.rs:9746  &root,
GAP ignored LINE crates/api/src/dayflags.rs:9747  &ask(&format!(
GAP ignored LINE crates/api/src/dayflags.rs:9748  "feed=gdfl&instrument={instrument}&from={on}&to={on}"
GAP ignored LINE crates/api/src/dayflags.rs:9749  )),
GAP ignored LINE crates/api/src/dayflags.rs:9751  let rec = record(&got, on).clone();
GAP ignored LINE crates/api/src/dayflags.rs:9752  println!(
GAP ignored LINE crates/api/src/dayflags.rs:9754  rec["held"]["bars"],
GAP ignored LINE crates/api/src/dayflags.rs:9755  rec["calendar"]["code"],
GAP ignored LINE crates/api/src/dayflags.rs:9756  rec["sampled"],
GAP ignored LINE crates/api/src/dayflags.rs:9757  rec["events"]["halt"]["held"],
GAP ignored LINE crates/api/src/dayflags.rs:9758  rec["degraded"],
GAP ignored LINE crates/api/src/dayflags.rs:9759  rec["says"]
GAP ignored LINE crates/api/src/dayflags.rs:9761  rec
GAP ignored LINE crates/api/src/dayflags.rs:9762  };
GAP ignored LINE crates/api/src/dayflags.rs:9763  let sampled_day = [
GAP ignored LINE crates/api/src/dayflags.rs:9764  "NSE-INDEX-NIFTY",
GAP ignored LINE crates/api/src/dayflags.rs:9765  "NSE-INDEX-BANKNIFTY",
GAP ignored LINE crates/api/src/dayflags.rs:9766  "NSE-FNO-NIFTY-2019-05-30-1190000-CE",
GAP ignored LINE crates/api/src/dayflags.rs:9767  "NSE-FNO-NIFTY-2019-05-30-1300000-PE",
GAP ignored LINE crates/api/src/dayflags.rs:9768  "NSE-CASH-RELIANCE",
GAP ignored LINE crates/api/src/dayflags.rs:9769  ]
GAP ignored LINE crates/api/src/dayflags.rs:9770  .map(|instrument| get(instrument, "2019-05-30"));
GAP ignored LINE crates/api/src/dayflags.rs:9771  let ordinary = get("NSE-INDEX-NIFTY", "2019-05-29");
GAP ignored LINE crates/api/src/dayflags.rs:9772  let halts = [
GAP ignored LINE crates/api/src/dayflags.rs:9773  ("NSE-INDEX-NIFTY", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9774  ("NSE-INDEX-NIFTY", "2020-03-23"),
GAP ignored LINE crates/api/src/dayflags.rs:9775  ("NSE-INDEX-BANKNIFTY", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9776  ("NSE-FNO-NIFTY-2020-03-19-950000-PE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9777  ("NSE-FNO-NIFTY-2020-03-26-1250000-CE", "2020-03-23"),
GAP ignored LINE crates/api/src/dayflags.rs:9778  ("NSE-FNO-BANKNIFTY-2020-03-19-2500000-PE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9779  ("NSE-CASH-RELIANCE", "2020-03-13"),
GAP ignored LINE crates/api/src/dayflags.rs:9780  ]
GAP ignored LINE crates/api/src/dayflags.rs:9781  .map(|(instrument, on)| get(instrument, on));
GAP ignored LINE crates/api/src/dayflags.rs:9782  for query in [
GAP ignored LINE crates/api/src/dayflags.rs:9783  "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9784  "feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9785  "feed=gdfl&instrument=NSE-FNO-NIFTY-2020-03-26-950000-PE&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9786  "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
GAP ignored LINE crates/api/src/dayflags.rs:9787  ] {
GAP ignored LINE crates/api/src/dayflags.rs:9788  let asked = ask(query);
GAP ignored LINE crates/api/src/dayflags.rs:9789  let _warm = answer(&root, &asked);
GAP ignored LINE crates/api/src/dayflags.rs:9790  let mut micros: Vec<u128> = (0..200)
GAP ignored LINE crates/api/src/dayflags.rs:9791  .map(|_| {
GAP ignored LINE crates/api/src/dayflags.rs:9792  let at = std::time::Instant::now();
GAP ignored LINE crates/api/src/dayflags.rs:9793  let got = answer(&root, &asked);
GAP ignored LINE crates/api/src/dayflags.rs:9794  let took = at.elapsed().as_micros();
GAP ignored LINE crates/api/src/dayflags.rs:9795  assert_eq!(got["days"].as_array().map(Vec::len), Some(91), "{query}");
GAP ignored LINE crates/api/src/dayflags.rs:9796  took
GAP ignored LINE crates/api/src/dayflags.rs:9797  })
GAP ignored LINE crates/api/src/dayflags.rs:9798  .collect();
GAP ignored LINE crates/api/src/dayflags.rs:9799  micros.sort_unstable();
GAP ignored LINE crates/api/src/dayflags.rs:9800  println!(
GAP ignored LINE crates/api/src/dayflags.rs:9802  micros[99], micros[197], micros[199]
GAP ignored LINE crates/api/src/dayflags.rs:9807  for rec in &sampled_day[..2] {
GAP ignored LINE crates/api/src/dayflags.rs:9808  assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9809  assert_eq!(rec["sampled"]["of"], "own", "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9810  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9811  says(rec).iter().any(|s| s.contains("minute-sampled")),
GAP ignored LINE crates/api/src/dayflags.rs:9814  assert!(!rec["degraded"].as_array().unwrap().is_empty(), "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9816  for rec in &sampled_day[2..4] {
GAP ignored LINE crates/api/src/dayflags.rs:9817  assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9818  assert_eq!(rec["sampled"]["of"], "underlying", "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9822  assert_eq!(
GAP ignored LINE crates/api/src/dayflags.rs:9823  sampled_day[4]["sampled"]["of"], "market",
GAP ignored LINE crates/api/src/dayflags.rs:9825  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9827  assert_eq!(
GAP ignored LINE crates/api/src/dayflags.rs:9828  sampled_day[4]["sampled"]["is"], true,
GAP ignored LINE crates/api/src/dayflags.rs:9830  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9832  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9833  sampled_day[4]["sampled"]["first_held"].is_string(),
GAP ignored LINE crates/api/src/dayflags.rs:9835  sampled_day[4]
GAP ignored LINE crates/api/src/dayflags.rs:9837  assert_eq!(ordinary["sampled"]["is"], false, "{ordinary:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9840  for rec in &halts {
GAP ignored LINE crates/api/src/dayflags.rs:9841  assert!(rec["events"]["halt"]["held"]["bars"].is_u64(), "{rec:#}");
GAP ignored LINE crates/api/src/dayflags.rs:9842  assert!(
GAP ignored LINE crates/api/src/dayflags.rs:9843  says(rec).iter().any(|s| s.contains("inside the halt")),
GAP ignored LINE crates/api/src/dayflags.rs:9847  }
GAP ignored LINE crates/api/src/feeds_route.rs:2973  fn held_span_growth_bench() {
GAP ignored LINE crates/api/src/feeds_route.rs:2974  held_span_growth("bench", [1_000, 10_000, 100_000], 200, 4);
GAP ignored LINE crates/api/src/feeds_route.rs:2975  }
GAP prod BRANCH crates/api/src/imports.rs:1342:13 False  [owed: outside a test's reach: An Err entry from ReadDir after read_dir succeeded needs the OS directory read to fail, from a failing medium or a FUSE or network filesystem. Permission and absence errors surface at read_dir (1328-1339) instead.]  let Ok(entry) = entry else {
GAP prod LINE crates/api/src/imports.rs:1343  [owed: outside a test's reach: ReadDir yields an entry Err only when the OS directory read fails after read_dir has opened the directory. Permission errors and absence are refused by read_dir itself (1328-1339), and glibc and musl report a deleted directory as end-of-directory, not as an error. What remains is an I/O failure from getdents or getdirentries on a failing disk, or a FUSE or network mount that returns EIO or ESTALE. A test cannot create such a mount, and no file content, name or permission a test lays out produces this error.]  odd.unnamed = odd.unnamed.saturating_add(1);
GAP prod LINE crates/api/src/imports.rs:1344  [owed: outside a test's reach: This is the same let-else as line 1343. It needs a directory read that fails after opening, which only a faulty medium or a FUSE or network filesystem produces.]  continue;
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:117:16 True  [reachable, not done in this round: This is the same input as line 118. A re-sealed source context whose referenced condition name or commit text exceeds 8 MiB makes result.2.len() > MAX_RESPONSE_BYTES on a kind=source page. Those are free saved-file strings with no bound (codec.rs:275-301).]  if result.2.len() > crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:118  [reachable, not done in this round: This field has no bound. The source-context decoder accepts each condition name, and the build commit, as any non-empty UTF-8 text whose length is a u64 read from the saved body (cli/src/index_stop_source_context_codec.rs:275-301; text() 562-565). admit() bounds only the name count (211-214). selection() returns the names of every bit the saved expression references (index_stop_source_context.rs:770-797), and project() serializes them verbatim (indexstopcandlesjson.rs:238-242, 248-249). Candles alone stay under about 1 MB (before+after < 4,096 at 58-64; count <= page_records at source_context.rs:740-745). Input: run a child with BRUTEX_BOOLEAN_OBSERVATION_BYTES set large (for example 1 GiB). Take a catalog and context produced by the launch, as the saved test does. Rewrite the context body so a referenced bit's name (for example bit 30) is about 9 MiB long. Then re-seal the chain with the keyless hashes the reader recomputes: (1) the context identity, blake3 of the domain string plus the body (349-353), and its receipt (boolean_observation_file.rs:49-62); (2) the BRISCL01 relation under catalog_link_identity(catalog, pin) (355-361, 800-823); (3) the catalog re-identified as contextual_catalog(legacy, link) (341-348, 877), with legacy = catalog_identity_of(...) (cli/src/index_stop.rs:601-620) written into the catalog header (index_stop_store.rs:324). A kind=source request then exceeds 8 MiB.]  refused(
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:120  [reachable, not done in this round: This is the same statement as line 118. A re-sealed source context whose referenced condition name (or commit text) is longer than 8 MiB makes the serialized kind=source body exceed MAX_RESPONSE_BYTES.]  "original-source response exceeds byte admission; no prefix returned",
GAP prod BRANCH crates/api/src/indexstopcandlesjson.rs:188:25 True  [owed: a race: The second conjunct means the reader is current at 181 and not current at 188, with only select_setting and with_current in between (185-187). With unchanged files, the shared-lock and generation checks (boolean_observation_file.rs:99-109, 169-186, 219-234) give the same answer both times. The second check fails only after a concurrent file replacement or exclusive publication lock between line 181 and line 188.]  if body.is_err() && reader.require_current().is_err() {
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:189  [owed: a race: This is the comment inside the block at 188-192; see that branch. It runs only if the reader.require_current() at 181 passes and the one at 188 fails. Both are the same generation and shared-lock check over the same files, so the evidence must change between them.]  // The current request still refuses. An explicit retry may re-admit
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:190  [owed: a race: This is the same block as line 189. It needs a concurrent replacement of a catalog, relation or context file, or an exclusive publication lock, taken after the check at line 181 and before the one at line 188.]  // byte-identical restored files under the same original completion.
GAP prod LINE crates/api/src/indexstopcandlesjson.rs:191  [owed: a race: This line needs require_current() to pass at 181 and fail at 188. require_current is Observation::with_current: a non-blocking shared flock on owner.lock plus file-generation checks (cli/src/boolean_observation_file.rs:93-109, 169-186, 219-234). The reader is checked out exclusively (159; detail.rs:1820-1863), and its nesting flags reset on drop. So with unchanged files the two checks agree. Only a file replaced, or an exclusive lock taken, between 181 and 188 makes body Err and the second check Err.]  *held = None;
GAP prod LINE crates/api/src/indexstopqualificationjson.rs:237  [reachable, not done in this round: This needs a saved qualification row whose verdict is Refused: an execution_complete(before) that is false (metrics.rs:86-96: an unreachable trade, an entry, path or closing refusal, a period with unavailable minutes, an unverified close or a refusal) or a training-window calendar assessment with a missing or unmeasured day (numeric.rs:645-651), on a store the strict load admits. Continuation -5 measured three ways that do NOT do it: a minute left out of a training session (the launch is refused before any metric: "calendar receipt V2 for 60 seconds over IST days 20209..=20239 is not complete: status Incomplete, offered 7874, expected 7875, missing 1, unexpected 0"); a training day whose stopped trade's optimistic and pessimistic readings straddle a later win (index_consistency counts it a zero day under day_rule sign_under_both_readings, not unmeasured: the Admitted launch with 23 April so shaped stayed Admitted, zero_days 1, unmeasured_days 0); and an unmeasured calendar day (eligibility_of answers Unmeasured only for DayKind::Unmeasured, a day outside the calendar's measured window, the five OpenLengthUnmeasured Muhurats all being in CHARTER_NON_REGULAR_IST_DAYS and so Excluded). Left: an entry, path or closing refusal (a minute the facts do not accept: misaligned, duplicated or outside the bounds, each of which the store writer or the calendar receipt may refuse first), which needs that chain read end to end before this row can be called either reachable by a file or (U); then rank counts it at 237.]  cli::boolean_evidence::AdmissionStatusV1::Refused => refused += 1,
GAP prod LINE crates/api/src/mastersrun.rs:351  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  Ok(ref wire) => {
GAP prod LINE crates/api/src/mastersrun.rs:352  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let (got, steps) = obtain(wire, clock, dir, source).await;
GAP prod LINE crates/api/src/mastersrun.rs:353  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  *outcome = Ok(got);
GAP prod LINE crates/api/src/mastersrun.rs:354  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  *tried = steps;
GAP prod BRANCH crates/api/src/mastersrun.rs:586:9 True  [owed: L-BCA-6: refresh_work past masters_dir fetches from the real public hosts (NSE, the vendor CDNs), which no test may ask]  let Ok(dir) = crate::server::masters_dir() else {
GAP prod LINE crates/api/src/mastersrun.rs:593  [owed: L-BCA-6: refresh_work past masters_dir fetches from the real public hosts (NSE, the vendor CDNs), which no test may ask]  let from = match pull::masters::PublicFetch::new() {
GAP prod LINE crates/api/src/mastersrun.rs:594  [owed: L-BCA-6: refresh_work past masters_dir fetches from the real public hosts (NSE, the vendor CDNs), which no test may ask]  Ok(from) => from,
GAP prod LINE crates/api/src/mastersrun.rs:595  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  Err(why) => {
GAP prod LINE crates/api/src/mastersrun.rs:596  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  return (
GAP prod LINE crates/api/src/mastersrun.rs:597  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
GAP prod LINE crates/api/src/mastersrun.rs:598  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  json_headers(),
GAP prod LINE crates/api/src/mastersrun.rs:599  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  format!(
GAP prod LINE crates/api/src/mastersrun.rs:600  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  r#"{{"landed":[],"refusal":{}}}"#,
GAP prod LINE crates/api/src/mastersrun.rs:601  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  crate::render::json_string(&why)
GAP prod LINE crates/api/src/mastersrun.rs:602  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  ),
GAP prod LINE crates/api/src/mastersrun.rs:603  [owed: the HTTP client builder refusing (pull::masters::PublicFetch::new): its roots and provider are compiled in, so only the process proxy environment could reach it, and the path it guards ends at real hosts (L-BCA-6); not driven]  );
GAP prod LINE crates/api/src/mastersrun.rs:607  [owed: L-BCA-6: refresh_work past masters_dir fetches from the real public hosts (NSE, the vendor CDNs), which no test may ask]  refresh_over(&site, &dir, &from, &masters::Clock).await
GAP prod BRANCH crates/api/src/mastersrun.rs:650:12 True  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  if written == landed.len() {
GAP prod LINE crates/api/src/mastersrun.rs:651  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  telemetry::Level::Info
GAP prod BRANCH crates/api/src/mastersrun.rs:710:21 True  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:41 False  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:41 True  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:53 False  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod BRANCH crates/api/src/mastersrun.rs:710:53 True  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  let status = if attempted_landed && complete && reloaded.is_ok() {
GAP prod LINE crates/api/src/mastersrun.rs:711  [owed: L-BCA-6: needs the credentialed Zerodha dump, a Parameter Store read no test may make (CLAUDE.md section 8)]  axum::http::StatusCode::OK
GAP prod LINE crates/api/src/optiongreeks.rs:558  [owed: a premium record stamped 0 or out of order inside the committed records: the store writer refuses both, and its block checksum refuses a damaged one (Unreadable); only a cut append's zero-filled extent could hold one]  Spot::OrderBroken => return Cell::refused("option_order_broken", None),
GAP prod BRANCH crates/api/src/selectionv6json.rs:175:16 False  [owed: the Mac (macOS only): the 503 needs a page past detail::MAX_RESPONSE_BYTES (8 MiB). A page reads at most 8 blocks of 8 rungs (asked limit 1..=PAGE_BLOCKS), each block at most 25 winners of fixed-width fields (the decoder refuses more), about 1.8 MB; past that only the store root grows it, echoed once as "root" and once in each of the 8 rungs' path or refusal, at most 6 bytes a byte (a control character JSON-escaped). Linux caps one environment string at MAX_ARG_STRLEN (131,072 bytes), so BRUTEX_STORE holds at most 131,058 bytes and the page at most about 9 x 786,348 = 7.08 MB, under the ceiling: no Linux test reaches it. macOS has no per-string cap (ARG_MAX 1 MiB for all of argv and envp), so a child given a BRUTEX_STORE of about 1 MiB of control characters answers a page of about 56 MB and the 503: a test for the Mac's Route B]  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
GAP prod LINE crates/api/src/selectionv6json.rs:178  [owed: the Mac (macOS only): the 503 needs a page past detail::MAX_RESPONSE_BYTES (8 MiB). A page reads at most 8 blocks of 8 rungs (asked limit 1..=PAGE_BLOCKS), each block at most 25 winners of fixed-width fields (the decoder refuses more), about 1.8 MB; past that only the store root grows it, echoed once as "root" and once in each of the 8 rungs' path or refusal, at most 6 bytes a byte (a control character JSON-escaped). Linux caps one environment string at MAX_ARG_STRLEN (131,072 bytes), so BRUTEX_STORE holds at most 131,058 bytes and the page at most about 9 x 786,348 = 7.08 MB, under the ceiling: no Linux test reaches it. macOS has no per-string cap (ARG_MAX 1 MiB for all of argv and envp), so a child given a BRUTEX_STORE of about 1 MiB of control characters answers a page of about 56 MB and the 503: a test for the Mac's Route B]  refused(
GAP prod LINE crates/api/src/selectionv6json.rs:180  [owed: the Mac (macOS only): the 503 needs a page past detail::MAX_RESPONSE_BYTES (8 MiB). A page reads at most 8 blocks of 8 rungs (asked limit 1..=PAGE_BLOCKS), each block at most 25 winners of fixed-width fields (the decoder refuses more), about 1.8 MB; past that only the store root grows it, echoed once as "root" and once in each of the 8 rungs' path or refusal, at most 6 bytes a byte (a control character JSON-escaped). Linux caps one environment string at MAX_ARG_STRLEN (131,072 bytes), so BRUTEX_STORE holds at most 131,058 bytes and the page at most about 9 x 786,348 = 7.08 MB, under the ceiling: no Linux test reaches it. macOS has no per-string cap (ARG_MAX 1 MiB for all of argv and envp), so a child given a BRUTEX_STORE of about 1 MiB of control characters answers a page of about 56 MB and the 503: a test for the Mac's Route B]  "Selection V6 response exceeds its byte ceiling; no prefix returned",
`````

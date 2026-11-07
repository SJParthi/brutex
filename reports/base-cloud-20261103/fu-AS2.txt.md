# fu-AS2.txt.md

Source: this shard's Route B whole:api readout of tip `2dc1534d` (`d3-route-b-cloud.sh`, the kit's set file, this Linux host), classified as its first lines say. One fenced block below holds it.

`````text
(F)/(U) list of BASE-COV-API-AS2 (repair round 3, shard G2), tip 2dc1534d8cdfaff34a6509339ac425da49037913, whole:api, Route B (see gaps-AS2.txt for the run).
(F) = the failing side of an assert!/assert_eq!/assert_ne!/panic!/unreachable! whose statement ran, in test code (recipe v4 step 4), as repair r2 read it.
(U) = unreachable on every target, aarch64-apple-darwin included: the proof from the code is in brackets on each row.
(F) 174 lines 549 sides; (U) 123 lines 90 sides.

(F) LINE crates/api/src/bars_sort.rs:2934  [failing arm of the statement at 2931, which ran]  shared.bytes()
(F) BRANCH crates/api/src/bars_sort.rs:3822:13 False  [the false side fails the check, and its line ran]  why.contains("; index them with: cli srt-build ") && why.ends_with("2031-02 2031-02"),
(F) BRANCH crates/api/src/bars_sort.rs:3822:66 False  [the false side fails the check, and its line ran]  why.contains("; index them with: cli srt-build ") && why.ends_with("2031-02 2031-02"),
(F) BRANCH crates/api/src/bars_sort.rs:3896:13 False  [the false side fails the check, and its line ran]  why.starts_with("scan_budget_exceeded: the extremes need 120 records read")
(F) BRANCH crates/api/src/bars_sort.rs:3897:20 False  [the false side fails the check, and its line ran]  && why.contains("past the 119 one request may fold"),
(F) BRANCH crates/api/src/bars_sort.rs:3936:13 False  [the false side fails the check, and its line ran]  why.starts_with("scan_budget_exceeded: the extremes need 120 records read")
(F) BRANCH crates/api/src/bars_sort.rs:3937:20 False  [the false side fails the check, and its line ran]  && why.contains("past the 119 one request may fold"),
(F) BRANCH crates/api/src/bars_sort.rs:4194:69 False  [the false side fails the check, and its line ran]  matches!(&refused, Err(super::Attempt::Refused(why)) if why == "a value page needs a value column"),
(F) BRANCH crates/api/src/bars_sort.rs:4564:17 False  [the false side fails the check, and its line ran]  assert!(read[1] > 0 && read[1] < 110, "{read:?}");
(F) BRANCH crates/api/src/bars_sort.rs:4564:32 False  [the false side fails the check, and its line ran]  assert!(read[1] > 0 && read[1] < 110, "{read:?}");
(F) BRANCH crates/api/src/bars_sort.rs:5047:26 False  [the false side fails the check, and its line ran]  } => (*slot, *lo, *hi) == (0, whole.lo, whole.hi) && entries.is_none(),
(F) BRANCH crates/api/src/bars_sort.rs:5507:13 False  [the false side fails the check, and its line ran]  !want.is_empty() && want.len() < 37,
(F) BRANCH crates/api/src/bars_sort.rs:5507:33 False  [the false side fails the check, and its line ran]  !want.is_empty() && want.len() < 37,
(F) BRANCH crates/api/src/bars_sort.rs:5629:13 False  [the false side fails the check, and its line ran]  !rows.is_empty() && !named.is_empty(),
(F) BRANCH crates/api/src/bars_sort.rs:5629:33 False  [the false side fails the check, and its line ran]  !rows.is_empty() && !named.is_empty(),
(F) BRANCH crates/api/src/bars_sort.rs:5661:13 False  [the false side fails the check, and its line ran]  served.is_empty() && faults.is_empty(),
(F) BRANCH crates/api/src/bars_sort.rs:5661:34 False  [the false side fails the check, and its line ran]  served.is_empty() && faults.is_empty(),
(F) LINE crates/api/src/bars_sort_tests.rs:246  [failing arm of the statement at 242, which ran]  column.as_str()
(F) LINE crates/api/src/bars_sort_tests.rs:1258  [failing arm of the statement at 1261, which ran]  column.as_str()
(F) LINE crates/api/src/bars_sort_tests.rs:1502  [failing arm of the statement at 1499, which ran]  answer.bars.first()
(F) BRANCH crates/api/src/bars_sort_tests.rs:403:13 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("would not serve")
(F) BRANCH crates/api/src/bars_sort_tests.rs:404:20 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("checksum"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:590:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:591:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:594:13 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why[1].contains("2031-03") && report.scanned_why[1].contains("0..260"));
(F) BRANCH crates/api/src/bars_sort_tests.rs:594:58 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why[1].contains("2031-03") && report.scanned_why[1].contains("0..260"));
(F) BRANCH crates/api/src/bars_sort_tests.rs:665:13 False  [the false side fails the check, and its line ran]  fault.contains("2031-02") && fault.contains("none of its 360 kept rows"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:665:42 False  [the false side fails the check, and its line ran]  fault.contains("2031-02") && fault.contains("none of its 360 kept rows"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:702:13 False  [the false side fails the check, and its line ran]  assert!(answer.extremes.is_some() && answer.extremes_why.is_none());
(F) BRANCH crates/api/src/bars_sort_tests.rs:702:42 False  [the false side fails the check, and its line ran]  assert!(answer.extremes.is_some() && answer.extremes_why.is_none());
(F) BRANCH crates/api/src/bars_sort_tests.rs:746:13 False  [the false side fails the check, and its line ran]  assert!(report.unread_rows > 0 && report.unread_rows + report.scanned_rows == answer.total);
(F) BRANCH crates/api/src/bars_sort_tests.rs:746:39 False  [the false side fails the check, and its line ran]  assert!(report.unread_rows > 0 && report.unread_rows + report.scanned_rows == answer.total);
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:9 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:36 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:850:63 False  [the false side fails the check, and its line ran]  why.contains("2031-03") && why.contains("is torn") && why.contains("0..260"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:908:21 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:909:28 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("failed when it was read")
(F) BRANCH crates/api/src/bars_sort_tests.rs:910:28 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("checksum"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1193:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-05")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1194:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1198:9 False  [the false side fails the check, and its line ran]  report.scanned_why[1].contains("2031-06")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1199:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[1].contains("the month has no .srt sort index")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1200:16 False  [the false side fails the check, and its line ran]  && !report.scanned_why[1].contains("small-month rule"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1462:13 False  [the false side fails the check, and its line ran]  assert!(stamps_served.contains(&lo) && stamps_served.contains(&(hi - MICROS)));
(F) BRANCH crates/api/src/bars_sort_tests.rs:1462:44 False  [the false side fails the check, and its line ran]  assert!(stamps_served.contains(&lo) && stamps_served.contains(&(hi - MICROS)));
(F) BRANCH crates/api/src/bars_sort_tests.rs:1513:26 False  [the false side fails the check, and its line ran]  .all(|fault| fault.contains("the sort index names")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1568:13 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1569:20 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("is not this bar file's"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1658:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1659:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("would not read"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1672:13 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why.is_empty() && report.build.is_empty());
(F) BRANCH crates/api/src/bars_sort_tests.rs:1672:46 False  [the false side fails the check, and its line ran]  assert!(report.scanned_why.is_empty() && report.build.is_empty());
(F) BRANCH crates/api/src/bars_sort_tests.rs:1728:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1729:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("the host refused its .srt sort index")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1730:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("the next request asks again"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1772:9 False  [the false side fails the check, and its line ran]  report.scanned_why[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1773:16 False  [the false side fails the check, and its line ran]  && report.scanned_why[0].contains("symbolic link; no link is followed"),
(F) BRANCH crates/api/src/bars_sort_tests.rs:1812:9 False  [the false side fails the check, and its line ran]  answer.faults[0].contains("2031-02")
(F) BRANCH crates/api/src/bars_sort_tests.rs:1813:16 False  [the false side fails the check, and its line ran]  && answer.faults[0].contains("none of its 37 rows on this page was read"),
(F) BRANCH crates/api/src/boolean_observation_budget.rs:110:21 False  [the false side fails the check, and its line ran]  assert!(why.contains(ENV) && why.contains("no default substituted"));
(F) BRANCH crates/api/src/boolean_observation_budget.rs:110:42 False  [the false side fails the check, and its line ran]  assert!(why.contains(ENV) && why.contains("no default substituted"));
(F) BRANCH crates/api/src/boolean_observation_budget.rs:126:13 False  [the false side fails the check, and its line ran]  why.contains("receipt seal differs")
(F) BRANCH crates/api/src/boolean_observation_budget.rs:127:20 False  [the false side fails the check, and its line ran]  && why.contains("123 bytes")
(F) BRANCH crates/api/src/boolean_observation_budget.rs:128:20 False  [the false side fails the check, and its line ran]  && why.contains("no body prefix")
(F) BRANCH crates/api/src/booleanjson_tests.rs:232:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanjson_tests.rs:232:40 False  [the false side fails the check, and its line ran]  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:472:13 False  [the false side fails the check, and its line ran]  start.elapsed() <= std::time::Duration::from_secs(45)
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:473:20 False  [the false side fails the check, and its line ran]  && std::fs::metadata(&log).unwrap().len() <= 1024 * 1024,
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:801:9 False  [the false side fails the check, and its line ran]  queried.contains("metadata accepts only one exact max_points query")
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:802:16 False  [the false side fails the check, and its line ran]  && !queried.contains(NO_STORE),
(F) LINE crates/api/src/calendar_of.rs:1344  [panic/unreachable line]  panic!("day {day} is {kind:?}, not an open session");
(F) LINE crates/api/src/calendar_of.rs:1789  [failing arm of the statement at 1784, which ran]  24 * 48
(F) LINE crates/api/src/calendar_of.rs:2517  [panic/unreachable line]  panic!("a landing with an answer lands answered")
(F) LINE crates/api/src/calendar_of.rs:3493  [failing arm of the statement at 3029, which ran]  json(&exchange)
(F) BRANCH crates/api/src/calendar_of.rs:1343:13 False  [the else of a let-else that panics, whose line ran]  let DayKind::Open(session) = kind else {
(F) BRANCH crates/api/src/calendar_of.rs:2278:13 False  [the false side fails the check, and its line ran]  named.contains(&format!(
(F) BRANCH crates/api/src/calendar_of.rs:2281:19 False  [the false side fails the check, and its line ran]  )) && named.ends_with("a split run is a midday break the exchange never took."),
(F) BRANCH crates/api/src/calendar_of.rs:2500:38 False  [the false side fails the check, and its line ran]  .is_ok_and(|flights| flights.len() == 1 && flights.contains_key(&key))
(F) BRANCH crates/api/src/calendar_of.rs:2516:17 False  [the else of a let-else that panics, whose line ran]  let Landed::Answered(derived) = &*landed else {
(F) BRANCH crates/api/src/calendar_of.rs:3158:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3158:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3629:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3629:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) LINE crates/api/src/calendar_record_tests.rs:78  [failing arm of the statement at 75, which ran]  iso(day)
(F) LINE crates/api/src/calendar_record_tests.rs:565  [failing arm of the statement at 544, which ran]  widened["special"]
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:13 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:40 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:13 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:39 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:422:32 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("1day") && why.contains("NIFTY")),
(F) LINE crates/api/src/calendar_route.rs:1353  [panic/unreachable line]  panic!("a space is no URI character");
(F) BRANCH crates/api/src/calendar_route.rs:1352:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, headers, body)) = asked else {
(F) BRANCH crates/api/src/calendar_route.rs:1425:13 False  [the false side fails the check, and its line ran]  answer.body.contains("\"feed\":\"gdfl\"")
(F) BRANCH crates/api/src/calendar_route.rs:1426:20 False  [the false side fails the check, and its line ran]  && answer
(F) LINE crates/api/src/calendar_route_tests.rs:670  [panic/unreachable line]  panic!("{query}: still building after 1,000 asks");
(F) LINE crates/api/src/calendar_route_tests.rs:1286  [panic/unreachable line]  panic!("an empty map is led");
(F) BRANCH crates/api/src/calendar_route_tests.rs:326:17 False  [the false side fails the check, and its line ran]  assert!(reason.contains(rung) && reason.contains("1s"), "{reason}");
(F) BRANCH crates/api/src/calendar_route_tests.rs:326:42 False  [the false side fails the check, and its line ran]  assert!(reason.contains(rung) && reason.contains("1s"), "{reason}");
(F) BRANCH crates/api/src/calendar_route_tests.rs:496:9 False  [the false side fails the check, and its line ran]  spanned.contains("calendar_json_held(site, uri).await")
(F) BRANCH crates/api/src/calendar_route_tests.rs:497:16 False  [the false side fails the check, and its line ran]  && !spanned.contains("census")
(F) BRANCH crates/api/src/calendar_route_tests.rs:498:16 False  [the false side fails the check, and its line ran]  && !spanned.contains("lead("),
(F) BRANCH crates/api/src/calendar_route_tests.rs:503:9 False  [the false side fails the check, and its line ran]  calendar.contains("lead(&site, uri, &asked, &build, now)")
(F) BRANCH crates/api/src/calendar_route_tests.rs:504:16 False  [the false side fails the check, and its line ran]  && !calendar.contains("calendar_json_reading(")
(F) BRANCH crates/api/src/calendar_route_tests.rs:505:16 False  [the false side fails the check, and its line ran]  && !calendar.contains("census_now_stamped"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:510:9 False  [the false side fails the check, and its line ran]  lead.contains("crate::detail::run_calendar(move ||")
(F) BRANCH crates/api/src/calendar_route_tests.rs:511:16 False  [the false side fails the check, and its line ran]  && lead.contains("calendar_json_reading(&site, &uri, census_now_stamped)"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:518:9 False  [the false side fails the check, and its line ran]  gaps.contains("voted(&site, &asked).await")
(F) BRANCH crates/api/src/calendar_route_tests.rs:519:16 False  [the false side fails the check, and its line ran]  && !gaps.contains("peer_calendar(")
(F) BRANCH crates/api/src/calendar_route_tests.rs:520:16 False  [the false side fails the check, and its line ran]  && !gaps.contains("peer_vote_reading("),
(F) BRANCH crates/api/src/calendar_route_tests.rs:540:9 False  [the false side fails the check, and its line ran]  body.contains("Saturated") && body.contains("at most 8"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:540:39 False  [the false side fails the check, and its line ran]  body.contains("Saturated") && body.contains("at most 8"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:614:9 False  [the false side fails the check, and its line ran]  !map.contains("Published::read") && !map.contains("masters_dir()"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:614:45 False  [the false side fails the check, and its line ran]  !map.contains("Published::read") && !map.contains("masters_dir()"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:895:13 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:895:46 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:942:9 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:942:77 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:1285:9 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(build) = builds.join(&key, Some(&now)) else {
(F) BRANCH crates/api/src/calendar_route_tests.rs:1445:9 False  [the false side fails the check, and its line ran]  why.contains("holds no readable 1s bar for BANKNIFTY: 1 month(s) could not be read (")
(F) BRANCH crates/api/src/calendar_route_tests.rs:1446:16 False  [the false side fails the check, and its line ran]  && why.contains("BANKNIFTY: 2022-03: the lookup for day "),
(F) BRANCH crates/api/src/calendar_route_tests.rs:1676:32 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("1min") && why.contains("SENSEX")),
(F) BRANCH crates/api/src/calendar_route_tests.rs:1734:9 False  [the false side fails the check, and its line ran]  why.contains(&MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:1734:50 False  [the false side fails the check, and its line ran]  why.contains(&MAX_BUILDS.to_string()) && why.contains("retry"),
(F) LINE crates/api/src/census_cache.rs:2611  [failing arm of the statement at 2608, which ran]  vendor.as_str()
(F) LINE crates/api/src/census_cache.rs:2723  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:2731  [failing arm of the statement at 2722, which ran]  read.note()
(F) LINE crates/api/src/census_cache.rs:2794  [failing arm of the statement at 2791, which ran]  row.note()
(F) LINE crates/api/src/census_cache.rs:2829  [failing arm of the statement at 2826, which ran]  now.freshness()
(F) LINE crates/api/src/census_cache.rs:3129  [failing arm of the statement at 3125, which ran]  vendor.as_str()
(F) LINE crates/api/src/census_cache.rs:3137  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:3145  [failing arm of the statement at 3136, which ran]  gdfl.note()
(F) LINE crates/api/src/census_cache.rs:3583  [panic/unreachable line]  panic!("a vendor directory is stamped by its partitions");
(F) LINE crates/api/src/census_cache.rs:3611  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:3619  [failing arm of the statement at 3610, which ran]  refused.note()
(F) LINE crates/api/src/census_cache.rs:3648  [panic/unreachable line]  panic!("still partitioned");
(F) LINE crates/api/src/census_cache.rs:3652  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:3711  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:3733  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) LINE crates/api/src/census_cache.rs:3752  [the false arm of a matches! inside an assert whose statement ran]  matches!(
(F) BRANCH crates/api/src/census_cache.rs:2671:13 False  [the false side fails the check, and its line ran]  refusal.contains("census-v9") && refusal.contains("REQUIRES"),
(F) BRANCH crates/api/src/census_cache.rs:2671:46 False  [the false side fails the check, and its line ran]  refusal.contains("census-v9") && refusal.contains("REQUIRES"),
(F) BRANCH crates/api/src/census_cache.rs:2681:17 False  [the false side fails the check, and its line ran]  why.starts_with(
(F) BRANCH crates/api/src/census_cache.rs:2684:22 False  [the false side fails the check, and its line ran]  ) && why.contains(&refusal)
(F) BRANCH crates/api/src/census_cache.rs:2685:24 False  [the false side fails the check, and its line ran]  && why.ends_with("); the first request that needs it builds it"),
(F) BRANCH crates/api/src/census_cache.rs:2705:32 False  [the false side fails the check, and its line ran]  .all(|row| row.name == CENSUS && row.state == WarmState::Failed),
(F) BRANCH crates/api/src/census_cache.rs:3020:13 False  [the false side fails the check, and its line ran]  changed.since >= before_change && changed.since > first.since,
(F) BRANCH crates/api/src/census_cache.rs:3020:47 False  [the false side fails the check, and its line ran]  changed.since >= before_change && changed.since > first.since,
(F) BRANCH crates/api/src/census_cache.rs:3352:13 False  [the false side fails the check, and its line ran]  why.contains("configured store root")
(F) BRANCH crates/api/src/census_cache.rs:3353:20 False  [the false side fails the check, and its line ran]  && why.contains("is unavailable")
(F) BRANCH crates/api/src/census_cache.rs:3354:20 False  [the false side fails the check, and its line ran]  && why.contains(&root.display().to_string()),
(F) BRANCH crates/api/src/census_cache.rs:3464:32 False  [the false side fails the check, and its line ran]  .all(|row| row.state == WarmState::Failed && row.took_ms == Some(4))
(F) BRANCH crates/api/src/census_cache.rs:3582:13 False  [the else of a let-else that panics, whose line ran]  let ManifestStamp::Partitioned(stamp) = vendor_stamp(&root, Vendor::Gdfl) else {
(F) BRANCH crates/api/src/census_cache.rs:3593:22 False  [the false side fails the check, and its line ran]  ] if kind == looped
(F) BRANCH crates/api/src/census_cache.rs:3647:13 False  [the else of a let-else that panics, whose line ran]  let ManifestStamp::Partitioned(moved) = vendor_stamp(&root, Vendor::Gdfl) else {
(F) BRANCH crates/api/src/census_cache.rs:3854:17 False  [the false side fails the check, and its line ran]  reason.starts_with("the background re-read could not be started (")
(F) BRANCH crates/api/src/census_cache.rs:3855:24 False  [the false side fails the check, and its line ran]  && reason.ends_with("); readers read it themselves"),
(F) LINE crates/api/src/census_tail_tests.rs:109  [panic/unreachable line]  panic!("not held: {}", census.note());
(F) LINE crates/api/src/census_tail_tests.rs:497  [panic/unreachable line]  panic!("served past its capacity: {}", over.note());
(F) BRANCH crates/api/src/census_tail_tests.rs:108:9 False  [the else of a let-else that panics, whose line ran]  let Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/census_tail_tests.rs:496:9 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { ref reason, fault } = over.state else {
(F) BRANCH crates/api/src/census_tail_tests.rs:572:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/census_tail_tests.rs:572:39 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/constituents.rs:1734:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1734:45 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1735:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1735:47 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1787:13 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(F) BRANCH crates/api/src/constituents.rs:1787:49 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(F) LINE crates/api/src/dayflags_rules_tests.rs:178  [failing arm of the statement at 173, which ran]  says(shown)
(F) LINE crates/api/src/dayflags_rules_tests.rs:577  [failing arm of the statement at 574, which ran]  got[6]
(F) LINE crates/api/src/dayflags_rules_tests.rs:873  [failing arm of the statement at 791, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:939  [failing arm of the statement at 857, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1047  [failing arm of the statement at 964, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1091  [failing arm of the statement at 1009, which ran]  sidecars.refusals()
(F) LINE crates/api/src/dayflags_rules_tests.rs:1105  [failing arm of the statement at 1022, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1129  [failing arm of the statement at 1046, which ran]  sidecars.refusals()
(F) LINE crates/api/src/dayflags_rules_tests.rs:1174  [failing arm of the statement at 1092, which ran]  says(after)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1447  [failing arm of the statement at 1365, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:1613  [failing arm of the statement at 1525, which ran]  says(record(&got, "2024-10-03"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:1659  [failing arm of the statement at 1574, which ran]  says(unheld)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2437  [failing arm of the statement at 2348, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2833  [failing arm of the statement at 2669, which ran]  TIMED[1]
(F) LINE crates/api/src/dayflags_rules_tests.rs:2977  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3505  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3532  [panic/unreachable line]  panic!("the month opens again");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3717  [failing arm of the statement at 3382, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:3809  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3979  [failing arm of the statement at 3644, which ran]  says(record(&got, "2024-11-09"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:4058  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:4102  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:4207  [failing arm of the statement at 3874, which ran]  says(closed)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4294  [failing arm of the statement at 3959, which ran]  says(after)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4398  [failing arm of the statement at 4065, which ran]  says(quiet)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4628  [failing arm of the statement at 4293, which ran]  super::super::tests::says(unstamped)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4637  [failing arm of the statement at 4302, which ran]  super::super::tests::says(short)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4915  [failing arm of the statement at 4582, which ran]  says(moving)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4930  [failing arm of the statement at 4593, which ran]  says(both)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4979  [failing arm of the statement at 4646, which ran]  says(aside)
(F) LINE crates/api/src/dayflags_rules_tests.rs:5315  [failing arm of the statement at 4982, which ran]  says(empty)
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:98:9 False  [the false side fails the check, and its line ran]  why.starts_with("timeframe=\"7min\" is not a rung this store files: one of ")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:99:16 False  [the false side fails the check, and its line ran]  && why.contains("1day")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:100:16 False  [the false side fails the check, and its line ran]  && why.contains("1s"),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:317:56 False  [the false side fails the check, and its line ran]  matches!(&flag.basis, CasBasis::Unread(why) if why.starts_with("2026-08-04: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:405:58 False  [the false side fails the check, and its line ran]  matches!(&got[2].basis, CasBasis::Unread(why) if why.starts_with("2026-08-10: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:478:25 False  [the false side fails the check, and its line ran]  .all(|flag| flag.eligibility == CasEligibility::Eligible
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:485:13 False  [the false side fails the check, and its line ran]  assert!(kept && held >= 1);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:485:21 False  [the false side fails the check, and its line ran]  assert!(kept && held >= 1);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:505:58 False  [the false side fails the check, and its line ran]  matches!(&absent.basis, CasBasis::Unread(why) if why.starts_with("2026-08-11: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:520:59 False  [the false side fails the check, and its line ran]  matches!(&outside.basis, CasBasis::Unread(why) if why.starts_with("2026-08-11: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:575:58 False  [the false side fails the check, and its line ran]  matches!(&got[6].basis, CasBasis::Unread(why) if why.starts_with("2026-08-25: ")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:1044:28 False  [the false side fails the check, and its line ran]  .all(|say| !say.contains("its print is not kept apart")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:1102:24 False  [the false side fails the check, and its line ran]  .all(|say| !say.contains("holds no auction sidecar")
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:1172:25 False  [the false side fails the check, and its line ran]  .all(|said| !said.starts_with("no GDFL") && !said.contains("holds no file")),
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:2976:13 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = opened else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3504:9 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = opened else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3527:13 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3527:33 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3531:9 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = again else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3577:13 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3577:33 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3808:9 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = opened else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:4057:9 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = opened else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:4101:13 False  [the else of a let-else that panics, whose line ran]  let super::super::Opened::File(file) = read else {
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5419:13 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5419:48 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
(F) LINE crates/api/src/disk_route.rs:1998  [failing arm of the statement at 1660, which ran]  dir.display()
(F) LINE crates/api/src/disk_route.rs:2008  [failing arm of the statement at 2004, which ran]  dir.display()
(F) BRANCH crates/api/src/disk_route.rs:1483:17 False  [the false side fails the check, and its line ran]  why.contains(said) && why.contains("ops/backups.log"),
(F) BRANCH crates/api/src/disk_route.rs:1483:39 False  [the false side fails the check, and its line ran]  why.contains(said) && why.contains("ops/backups.log"),
(F) BRANCH crates/api/src/disk_route.rs:1565:17 False  [the false side fails the check, and its line ran]  why.contains("ops/backups.log") && why.contains("cannot be told"),
(F) BRANCH crates/api/src/disk_route.rs:1565:52 False  [the false side fails the check, and its line ran]  why.contains("ops/backups.log") && why.contains("cannot be told"),
(F) BRANCH crates/api/src/disk_route.rs:2075:17 False  [the false side fails the check, and its line ran]  why.starts_with(
(F) BRANCH crates/api/src/disk_route.rs:2077:22 False  [the false side fails the check, and its line ran]  ) && why.contains(says)
(F) BRANCH crates/api/src/disk_route.rs:2078:24 False  [the false side fails the check, and its line ran]  && why.contains(&format!("{bad:?}")),
(F) BRANCH crates/api/src/disk_route.rs:2209:13 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("does not exist"),
(F) BRANCH crates/api/src/disk_route.rs:2209:37 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("does not exist"),
(F) BRANCH crates/api/src/disk_route.rs:2247:13 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("is not a directory"),
(F) BRANCH crates/api/src/disk_route.rs:2247:37 False  [the false side fails the check, and its line ran]  why.contains(&named) && why.contains("is not a directory"),
(F) BRANCH crates/api/src/disk_route.rs:2256:13 False  [the false side fails the check, and its line ran]  why.contains(&below.display().to_string()) && why.contains("cannot be measured"),
(F) BRANCH crates/api/src/disk_route.rs:2256:59 False  [the false side fails the check, and its line ran]  why.contains(&below.display().to_string()) && why.contains("cannot be measured"),
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:233:13 False  [the false side fails the check, and its line ran]  assert!(receipt(root).is_file() && census_store::requires_path(root).is_file());
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:233:40 False  [the false side fails the check, and its line ran]  assert!(receipt(root).is_file() && census_store::requires_path(root).is_file());
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:586:9 False  [the false side fails the check, and its line ran]  why.contains("a seal was interrupted after its link and before its stub")
(F) BRANCH crates/api/src/disk_route_seal_tests.rs:587:16 False  [the false side fails the check, and its line ran]  && why.contains(&format!("one file of {bytes} bytes")),
(F) LINE crates/api/src/fno_boundary_tests.rs:45  [panic/unreachable line]  panic!("named-chain fixture requires an HTTP descriptor");
(F) LINE crates/api/src/fno_boundary_tests.rs:227  [panic/unreachable line]  panic!("the rolling fixture requires an HTTP descriptor");
(F) BRANCH crates/api/src/fno_boundary_tests.rs:44:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/fno_boundary_tests.rs:226:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = self.asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/fno_boundary_tests.rs:1492:9 False  [the false side fails the check, and its line ran]  body.contains("<th>Contracts discovered</th><td>3</td>")
(F) BRANCH crates/api/src/fno_boundary_tests.rs:1493:16 False  [the false side fails the check, and its line ran]  && body.contains("<th>Contracts asked for</th><td>3</td>"),
(F) LINE crates/api/src/head_deadline_body_tests.rs:167  [failing arm of the statement at 164, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/head_deadline_body_tests.rs:197  [panic/unreachable line]  panic!("no byte for 10 s after {} of {want}", have.len());
(F) LINE crates/api/src/head_deadline_body_tests.rs:218  [panic/unreachable line]  panic!("held past {limit:?}; {} byte(s) came", got.len());
(F) LINE crates/api/src/head_deadline_body_tests.rs:239  [failing arm of the statement at 232, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/head_deadline_body_tests.rs:252  [failing arm of the statement at 248, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:359  [failing arm of the statement at 348, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:441  [failing arm of the statement at 427, which ran]  cut[0]
(F) LINE crates/api/src/head_deadline_body_tests.rs:538  [failing arm of the statement at 527, which ran]  body.len()
(F) LINE crates/api/src/head_deadline_body_tests.rs:769  [failing arm of the statement at 758, which ran]  left.elapsed()
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:196:13 False  [the else of a let-else that panics, whose line ran]  let Ok(read) = read.await else {
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:217:13 False  [the else of a let-else that panics, whose line ran]  let Ok(read) = tokio::time::timeout_at(until, stream.read(&mut chunk)).await else {
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:312:9 False  [the false side fails the check, and its line ran]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:312:48 False  [the false side fails the check, and its line ran]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("ok"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:357:9 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:357:19 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:379:9 False  [the false side fails the check, and its line ran]  went > 0 && went < u64::try_from(BIG).unwrap(),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:379:21 False  [the false side fails the check, and its line ran]  went > 0 && went < u64::try_from(BIG).unwrap(),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:432:17 False  [the false side fails the check, and its line ran]  assert!(closed && body.len() < BIG, "{target}: {} bytes", body.len());
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:432:27 False  [the false side fails the check, and its line ran]  assert!(closed && body.len() < BIG, "{target}: {} bytes", body.len());
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:458:13 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == BIG);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:458:24 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == BIG);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:477:13 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == length);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:477:24 False  [the false side fails the check, and its line ran]  assert!(!closed && body.len() == length);
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:536:13 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:536:23 False  [the false side fails the check, and its line ran]  closed && body.len() < length,
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:575:9 False  [the false side fails the check, and its line ran]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
(F) BRANCH crates/api/src/head_deadline_body_tests.rs:575:48 False  [the false side fails the check, and its line ran]  said.starts_with("HTTP/1.1 200 OK") && said.ends_with("slow"),
(F) BRANCH crates/api/src/indexstoprankingjson.rs:540:17 False  [the false side fails the check, and its line ran]  assert!(discover < cache && pinned < cache);
(F) BRANCH crates/api/src/indexstoprankingjson.rs:540:37 False  [the false side fails the check, and its line ran]  assert!(discover < cache && pinned < cache);
(F) BRANCH crates/api/src/indexstoprankingjson.rs:643:37 False  [the false side fails the check, and its line ran]  Ok(Err(why)) if why.starts_with("Cumulative saved comparison: no saved checkpoint exists")
(F) LINE crates/api/src/indexstopvixjson.rs:340  [panic/unreachable line]  panic!("generated exact reference fixture must contain a candle");
(F) BRANCH crates/api/src/indexstopvixjson.rs:339:13 False  [the else of a let-else that panics, whose line ran]  let Stamp::Exact(candle) = &mut original else {
(F) BRANCH crates/api/src/instruments_route.rs:882:17 False  [the false side fails the check, and its line ran]  answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
(F) BRANCH crates/api/src/instruments_route.rs:882:61 False  [the false side fails the check, and its line ran]  answer.contains("x-brutex-census-note:") && answer.contains("UNREADABLE"),
(F) BRANCH crates/api/src/instruments_route.rs:992:13 False  [the false side fails the check, and its line ran]  !json.contains('\n') && !json.contains('\t'),
(F) BRANCH crates/api/src/instruments_route.rs:992:37 False  [the false side fails the check, and its line ran]  !json.contains('\n') && !json.contains('\t'),
(F) BRANCH crates/api/src/instruments_route.rs:996:13 False  [the false side fails the check, and its line ran]  json.starts_with('[') && json.ends_with(']'),
(F) BRANCH crates/api/src/instruments_route.rs:996:38 False  [the false side fails the check, and its line ran]  json.starts_with('[') && json.ends_with(']'),
(F) BRANCH crates/api/src/instruments_route.rs:1176:13 False  [the false side fails the check, and its line ran]  archive.contains(r#""mastered":false"#)
(F) BRANCH crates/api/src/instruments_route.rs:1177:20 False  [the false side fails the check, and its line ran]  && archive.contains(r#""counted_from":"no master""#),
(F) BRANCH crates/api/src/instruments_route.rs:1196:13 False  [the false side fails the check, and its line ran]  refused.contains(r#""feed":"upstox""#)
(F) BRANCH crates/api/src/instruments_route.rs:1197:20 False  [the false side fails the check, and its line ran]  && refused.contains("this build reads no feed called that")
(F) BRANCH crates/api/src/instruments_route.rs:1198:20 False  [the false side fails the check, and its line ran]  && refused.contains("groww")
(F) BRANCH crates/api/src/instruments_route.rs:1199:20 False  [the false side fails the check, and its line ran]  && refused.contains("dhan"),
(F) BRANCH crates/api/src/instruments_route.rs:1231:32 False  [the false side fails the check, and its line ran]  .is_ok_and(|t| t.starts_with('"') && t.ends_with('"')),
(F) BRANCH crates/api/src/instruments_route.rs:1418:13 False  [the false side fails the check, and its line ran]  note.starts_with("gdfl: UNAVAILABLE") && note.contains("first reason"),
(F) BRANCH crates/api/src/instruments_route.rs:1418:54 False  [the false side fails the check, and its line ran]  note.starts_with("gdfl: UNAVAILABLE") && note.contains("first reason"),
(F) LINE crates/api/src/livejson.rs:682  [failing arm of the statement at 678, which ran]  body.len()
(F) LINE crates/api/src/livejson.rs:687  [failing arm of the statement at 684, which ran]  body.len()
(F) BRANCH crates/api/src/livejson.rs:336:13 False  [the false side fails the check, and its line ran]  !body.contains(r#""stale""#) && !body.contains(r#""strays""#),
(F) BRANCH crates/api/src/livejson.rs:336:45 False  [the false side fails the check, and its line ran]  !body.contains(r#""stale""#) && !body.contains(r#""strays""#),
(F) BRANCH crates/api/src/livejson.rs:528:13 False  [the false side fails the check, and its line ran]  body.contains(r#""t_milli":1802"#) && body.contains(r#""clears_bar":false"#),
(F) BRANCH crates/api/src/livejson.rs:528:51 False  [the false side fails the check, and its line ran]  body.contains(r#""t_milli":1802"#) && body.contains(r#""clears_bar":false"#),
(F) BRANCH crates/api/src/livejson.rs:537:13 False  [the false side fails the check, and its line ran]  !body.contains(r#""meets""#) && !body.contains(r#""win_rate_bp""#),
(F) BRANCH crates/api/src/livejson.rs:537:45 False  [the false side fails the check, and its line ran]  !body.contains(r#""meets""#) && !body.contains(r#""win_rate_bp""#),
(F) BRANCH crates/api/src/livejson.rs:550:13 False  [the false side fails the check, and its line ran]  body.contains(r#""idle_secs":"#) && !body.contains(r#""idle_secs":null"#),
(F) BRANCH crates/api/src/livejson.rs:550:49 False  [the false side fails the check, and its line ran]  body.contains(r#""idle_secs":"#) && !body.contains(r#""idle_secs":null"#),
(F) BRANCH crates/api/src/livejson.rs:555:13 False  [the false side fails the check, and its line ran]  body.contains(r#""stale":false"#) && body.contains(r#""stale_after_secs":86400"#),
(F) BRANCH crates/api/src/livejson.rs:555:50 False  [the false side fails the check, and its line ran]  body.contains(r#""stale":false"#) && body.contains(r#""stale_after_secs":86400"#),
(F) BRANCH crates/api/src/livejson.rs:690:13 False  [the false side fails the check, and its line ran]  body.contains("hard response ceiling") && body.contains(r#""runs":[]"#),
(F) BRANCH crates/api/src/livejson.rs:690:55 False  [the false side fails the check, and its line ran]  body.contains("hard response ceiling") && body.contains(r#""runs":[]"#),
(F) BRANCH crates/api/src/livejson.rs:703:13 False  [the false side fails the check, and its line ran]  encoded > 0 && encoded < cli::live::LIVE_RUN_LIMIT,
(F) BRANCH crates/api/src/livejson.rs:703:28 False  [the false side fails the check, and its line ran]  encoded > 0 && encoded < cli::live::LIVE_RUN_LIMIT,
(F) LINE crates/api/src/master.rs:698  [failing arm of the statement at 693, which ran]  got.kept.len(),
(F) LINE crates/api/src/master.rs:699  [failing arm of the statement at 693, which ran]  got.declined.len(),
(F) BRANCH crates/api/src/master.rs:626:13 False  [the false side fails the check, and its line ran]  kept.contains(&"INFY") && kept.contains(&"RELIANCE"),
(F) BRANCH crates/api/src/master.rs:626:39 False  [the false side fails the check, and its line ran]  kept.contains(&"INFY") && kept.contains(&"RELIANCE"),
(F) BRANCH crates/api/src/master.rs:969:13 False  [the false side fails the check, and its line ran]  reason.contains("row has 5 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:969:54 False  [the false side fails the check, and its line ran]  reason.contains("row has 5 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:995:13 False  [the false side fails the check, and its line ran]  reason.contains("row has 9 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:995:54 False  [the false side fails the check, and its line ran]  reason.contains("row has 9 field(s)") && reason.contains("run to 10"),
(F) BRANCH crates/api/src/master.rs:1182:13 False  [the false side fails the check, and its line ran]  err.contains(&(MAX_MASTER_BYTES + 1).to_string())
(F) BRANCH crates/api/src/master.rs:1183:20 False  [the false side fails the check, and its line ran]  && err.contains(&MAX_MASTER_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1222:13 False  [the false side fails the check, and its line ran]  err.contains("header row is") && err.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1222:46 False  [the false side fails the check, and its line ran]  err.contains("header row is") && err.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1263:13 False  [the false side fails the check, and its line ran]  reason.contains("row is") && reason.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1263:42 False  [the false side fails the check, and its line ran]  reason.contains("row is") && reason.contains(&MAX_ROW_BYTES.to_string()),
(F) BRANCH crates/api/src/master.rs:1325:17 False  [the false side fails the check, and its line ran]  assert!(got.kept.is_empty() && got.skipped.is_empty());
(F) BRANCH crates/api/src/master.rs:1325:40 False  [the false side fails the check, and its line ran]  assert!(got.kept.is_empty() && got.skipped.is_empty());
(F) LINE crates/api/src/operation_audit_tests.rs:200  [failing arm of the statement at 197, which ran]  routes.len()
(F) BRANCH crates/api/src/operation_audit_tests.rs:318:75 False  [the false side fails the check, and its line ran]  matches!(parsed, Asked::Page { before: Some(asked), limit: 2 } if asked == before),
(F) BRANCH crates/api/src/operation_audit_tests.rs:337:29 False  [the false side fails the check, and its line ran]  Asked::Exact(id) if id == ID_BASE + 1
(F) LINE crates/api/src/provenance_tests.rs:1230  [panic/unreachable line]  panic!("a sidecar");
(F) LINE crates/api/src/provenance_tests.rs:1331  [failing arm of the statement at 1328, which ran]  wire.len()
(F) LINE crates/api/src/provenance_tests.rs:2221  [panic/unreachable line]  panic!("the sidecar is present");
(F) LINE crates/api/src/provenance_tests.rs:2638  [panic/unreachable line]  panic!("the sidecar is present");
(F) LINE crates/api/src/provenance_tests.rs:2744  [panic/unreachable line]  panic!("a journal below a file is refused");
(F) LINE crates/api/src/provenance_tests.rs:2811  [panic/unreachable line]  panic!("a journal that ends before its length is refused");
(F) BRANCH crates/api/src/provenance_tests.rs:1229:9 False  [the else of a let-else that panics, whose line ran]  let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
(F) BRANCH crates/api/src/provenance_tests.rs:2014:13 False  [the false side fails the check, and its line ran]  got.body.get("path").is_none() && got.body.get("held").is_none(),
(F) BRANCH crates/api/src/provenance_tests.rs:2014:47 False  [the false side fails the check, and its line ran]  got.body.get("path").is_none() && got.body.get("held").is_none(),
(F) BRANCH crates/api/src/provenance_tests.rs:2153:9 False  [the false side fails the check, and its line ran]  why.starts_with(&format!(
(F) BRANCH crates/api/src/provenance_tests.rs:2156:15 False  [the false side fails the check, and its line ran]  )) && why.contains("Not a directory"),
(F) BRANCH crates/api/src/provenance_tests.rs:2220:9 False  [the else of a let-else that panics, whose line ran]  let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
(F) BRANCH crates/api/src/provenance_tests.rs:2397:13 False  [the false side fails the check, and its line ran]  why.starts_with(on)
(F) BRANCH crates/api/src/provenance_tests.rs:2398:20 False  [the false side fails the check, and its line ran]  && why.contains("block 0 was sealed")
(F) BRANCH crates/api/src/provenance_tests.rs:2399:20 False  [the false side fails the check, and its line ran]  && why.ends_with("those are not the bytes that were committed"),
(F) BRANCH crates/api/src/provenance_tests.rs:2420:9 False  [the false side fails the check, and its line ran]  why.starts_with(&format!("t={fifth}: ")) && why.contains("block 0 was sealed"),
(F) BRANCH crates/api/src/provenance_tests.rs:2420:53 False  [the false side fails the check, and its line ran]  why.starts_with(&format!("t={fifth}: ")) && why.contains("block 0 was sealed"),
(F) BRANCH crates/api/src/provenance_tests.rs:2541:9 False  [the false side fails the check, and its line ran]  own["stem"].is_null() && own.get("zip_day").is_none(),
(F) BRANCH crates/api/src/provenance_tests.rs:2541:34 False  [the false side fails the check, and its line ran]  own["stem"].is_null() && own.get("zip_day").is_none(),
(F) BRANCH crates/api/src/provenance_tests.rs:2637:9 False  [the else of a let-else that panics, whose line ran]  let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
(F) BRANCH crates/api/src/provenance_tests.rs:2719:9 False  [the false side fails the check, and its line ran]  notes[0].starts_with(&lead)
(F) BRANCH crates/api/src/provenance_tests.rs:2720:16 False  [the false side fails the check, and its line ran]  && notes[0].ends_with(" ms; later requests read only what is appended"),
(F) BRANCH crates/api/src/provenance_tests.rs:2735:9 False  [the false side fails the check, and its line ran]  why.starts_with(&format!("{}: ", audit::journal_path(&root).display()))
(F) BRANCH crates/api/src/provenance_tests.rs:2736:16 False  [the false side fails the check, and its line ran]  && why.contains("Not a directory"),
(F) BRANCH crates/api/src/provenance_tests.rs:2743:9 False  [the else of a let-else that panics, whose line ran]  let Err(direct) = read_runs(&root) else {
(F) BRANCH crates/api/src/provenance_tests.rs:2796:9 False  [the false side fails the check, and its line ran]  claimed >= audit::RECORD_LEN_U64 && held < audit::RECORD_LEN,
(F) BRANCH crates/api/src/provenance_tests.rs:2796:45 False  [the false side fails the check, and its line ran]  claimed >= audit::RECORD_LEN_U64 && held < audit::RECORD_LEN,
(F) BRANCH crates/api/src/provenance_tests.rs:2810:13 False  [the else of a let-else that panics, whose line ran]  let Err(why) = read_runs(&root) else {
(F) LINE crates/api/src/quarantine_route_tests.rs:553  [failing arm of the statement at 542, which ran]  matches!(
(F) LINE crates/api/src/render.rs:3932  [failing arm of the statement at 3926, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:3937  [failing arm of the statement at 3931, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:3950  [failing arm of the statement at 3940, which ran]  feed.display()
(F) LINE crates/api/src/render.rs:4091  [failing arm of the statement at 4085, which ran]  list.get(..160).unwrap_or(list)
(F) LINE crates/api/src/render.rs:4244  [failing arm of the statement at 4237, which ran]  reason.label()
(F) LINE crates/api/src/render.rs:5724  [failing arm of the statement at 5687, which ran]  String::from_utf8_lossy(&output.stderr)
(F) BRANCH crates/api/src/render.rs:3542:17 False  [the false side fails the check, and its line ran]  assert!(mid.contains("previous") && mid.contains("next"));
(F) BRANCH crates/api/src/render.rs:3542:45 False  [the false side fails the check, and its line ran]  assert!(mid.contains("previous") && mid.contains("next"));
(F) BRANCH crates/api/src/render.rs:3848:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("785") && html.contains("Tracked"));
(F) BRANCH crates/api/src/render.rs:3848:41 False  [the false side fails the check, and its line ran]  assert!(html.contains("785") && html.contains("Tracked"));
(F) BRANCH crates/api/src/render.rs:3954:13 False  [the false side fails the check, and its line ran]  !html.contains("broker \u{b7}") && !html.contains("archive \u{b7}"),
(F) BRANCH crates/api/src/render.rs:3954:48 False  [the false side fails the check, and its line ran]  !html.contains("broker \u{b7}") && !html.contains("archive \u{b7}"),
(F) BRANCH crates/api/src/render.rs:4004:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"));
(F) BRANCH crates/api/src/render.rs:4004:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"));
(F) BRANCH crates/api/src/render.rs:4202:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("12040") && html.contains("11990"));
(F) BRANCH crates/api/src/render.rs:4202:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("12040") && html.contains("11990"));
(F) BRANCH crates/api/src/render.rs:4204:17 False  [the false side fails the check, and its line ran]  assert!(html.contains(">40<") && html.contains(">10<"));
(F) BRANCH crates/api/src/render.rs:4204:42 False  [the false side fails the check, and its line ran]  assert!(html.contains(">40<") && html.contains(">10<"));
(F) BRANCH crates/api/src/render.rs:4400:13 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4400:42 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4400:71 False  [the false side fails the check, and its line ran]  strip.contains("x=\"0\"") && strip.contains("x=\"2\"") && strip.contains("x=\"3\"")
(F) BRANCH crates/api/src/render.rs:4460:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("#41") && html.contains("#40"), "{html}");
(F) BRANCH crates/api/src/render.rs:4460:41 False  [the false side fails the check, and its line ran]  assert!(html.contains("#41") && html.contains("#40"), "{html}");
(F) BRANCH crates/api/src/render.rs:4572:21 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4572:49 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4572:73 False  [the false side fails the check, and its line ran]  !url.contains("07?page") && !url.contains("=0?") && !url.contains("=1?"),
(F) BRANCH crates/api/src/render.rs:4686:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("2026-07") && html.contains("2026-06"));
(F) BRANCH crates/api/src/render.rs:4686:45 False  [the false side fails the check, and its line ran]  assert!(html.contains("2026-07") && html.contains("2026-06"));
(F) BRANCH crates/api/src/render.rs:4842:17 False  [the false side fails the check, and its line ran]  assert!(cell.contains("INE121A01024") && cell.contains("INE121A08PJ0"));
(F) BRANCH crates/api/src/render.rs:4842:50 False  [the false side fails the check, and its line ran]  assert!(cell.contains("INE121A01024") && cell.contains("INE121A08PJ0"));
(F) BRANCH crates/api/src/render.rs:4857:17 False  [the false side fails the check, and its line ran]  assert!(fut.contains("Future") && fut.contains("2026-09-29"));
(F) BRANCH crates/api/src/render.rs:4857:43 False  [the false side fails the check, and its line ran]  assert!(fut.contains("Future") && fut.contains("2026-09-29"));
(F) BRANCH crates/api/src/render.rs:5318:17 False  [the false side fails the check, and its line ran]  assert!(n.contains("index") && n.contains("F&amp;O"));
(F) BRANCH crates/api/src/render.rs:5318:40 False  [the false side fails the check, and its line ran]  assert!(n.contains("index") && n.contains("F&amp;O"));
(F) BRANCH crates/api/src/render.rs:5328:17 False  [the false side fails the check, and its line ran]  assert!(r.contains("F&amp;O") && r.contains("total mkt"));
(F) BRANCH crates/api/src/render.rs:5328:42 False  [the false side fails the check, and its line ran]  assert!(r.contains("F&amp;O") && r.contains("total mkt"));
(F) BRANCH crates/api/src/render.rs:5722:13 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/api/src/render.rs:5722:40 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) LINE crates/api/src/research_doors_tests.rs:95  [panic/unreachable line]  panic!("{door} {query}: a refused body is JSON ({why}): {body}");
(F) LINE crates/api/src/server.rs:18907  [failing arm of the statement at 18904, which ran]  began.elapsed()
(F) LINE crates/api/src/server.rs:18949  [panic/unreachable line]  panic!("held past {limit:?}; {} byte(s) came", got.len());
(F) LINE crates/api/src/server.rs:18970  [failing arm of the statement at 18966, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/server.rs:21373  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:21661  [failing arm of the statement at 21674, which ran]  withheld[0].1
(F) LINE crates/api/src/server.rs:22793  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24134  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24259  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24309  [failing arm of the statement at 24202, which ran]  dir.display()
(F) LINE crates/api/src/server.rs:24315  [panic/unreachable line]  panic!("{receipts:?}");
(F) LINE crates/api/src/server.rs:24318  [panic/unreachable line]  panic!("{segment:?}");
(F) LINE crates/api/src/server.rs:24613  [panic/unreachable line]  panic!("the walk's feed is an HTTP feed");
(F) LINE crates/api/src/server.rs:24647  [panic/unreachable line]  panic!("a walk opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24652  [panic/unreachable line]  panic!("a spot request opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24696  [panic/unreachable line]  panic!("a walk took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24705  [panic/unreachable line]  panic!("a spot request took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24740  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24837  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24880  [failing arm of the statement at 24578, which ran]  away.map(|calendar| calendar.sessions())
(F) LINE crates/api/src/server.rs:24927  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:25029  [panic/unreachable line]  panic!("HTTP descriptor")
(F) LINE crates/api/src/server.rs:25815  [failing arm of the statement at 25513, which ran]  root.display()
(F) LINE crates/api/src/server.rs:29196  [panic/unreachable line]  panic!("{feed:?} is HTTP");
(F) LINE crates/api/src/server.rs:29917  [failing arm of the statement at 29913, which ran]  root.display()
(F) LINE crates/api/src/server.rs:30052  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:30066  [failing arm of the statement at 29570, which ran]  stock.len(),
(F) LINE crates/api/src/server.rs:30067  [failing arm of the statement at 29570, which ran]  index.len()
(F) LINE crates/api/src/server.rs:31149  [panic/unreachable line]  panic!("{} is an HTTP feed and must have a budget", feed.display())
(F) LINE crates/api/src/server.rs:31186  [failing arm of the statement at 30691, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:31231  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31581  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31645  [panic/unreachable line]  panic!("Zerodha is an HTTP feed");
(F) LINE crates/api/src/server.rs:32138  [panic/unreachable line]  panic!("Zerodha is an HTTP feed")
(F) LINE crates/api/src/server.rs:32825  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:32937  [failing arm of the statement at 32080, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32983  [failing arm of the statement at 32125, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32991  [failing arm of the statement at 32133, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32996  [failing arm of the statement at 32139, which ran]  feed.display(),
(F) LINE crates/api/src/server.rs:32997  [failing arm of the statement at 32139, which ran]  months.len()
(F) LINE crates/api/src/server.rs:33063  [panic/unreachable line]  panic!("{} declares no HTTP transport", feed.display());
(F) LINE crates/api/src/server.rs:33069  [failing arm of the statement at 32211, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33076  [failing arm of the statement at 32217, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33090  [failing arm of the statement at 32232, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33096  [failing arm of the statement at 32238, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33109  [failing arm of the statement at 32252, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33116  [failing arm of the statement at 32258, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:33307  [failing arm of the statement at 32446, which ran]  &body[..body.len().min(120)]
(F) LINE crates/api/src/server.rs:33509  [panic/unreachable line]  panic!("attempt {attempt} of a 429 gave {stepped:?}");
(F) LINE crates/api/src/server.rs:33838  [failing arm of the statement at 32980, which ran]  marked[0]
(F) LINE crates/api/src/server.rs:34028  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:35282  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:35313  [failing arm of the statement at 34450, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:35396  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:35427  [failing arm of the statement at 34564, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:35637  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:35794  [panic/unreachable line]  panic!("this feed authenticates, so its transport is HTTP")
(F) LINE crates/api/src/server.rs:35910  [failing arm of the statement at 35047, which ran]  chunks.len()
(F) LINE crates/api/src/server.rs:36108  [failing arm of the statement at 35245, which ran]  refused.len()
(F) LINE crates/api/src/server.rs:36340  [failing arm of the statement at 35476, which ran]  ledger.fact(1).1
(F) LINE crates/api/src/server.rs:37015  [panic/unreachable line]  panic!("{feed} is an HTTP broker");
(F) LINE crates/api/src/server.rs:37333  [failing arm of the statement at 36473, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:37385  [failing arm of the statement at 36525, which ran]  tf.as_str()
(F) LINE crates/api/src/server.rs:37433  [failing arm of the statement at 36573, which ran]  rung.as_str()
(F) LINE crates/api/src/server.rs:37985  [failing arm of the statement at 37126, which ran]  bucket.word()
(F) BRANCH crates/api/src/server.rs:18948:17 False  [the else of a let-else that panics, whose line ran]  let Ok(read) = tokio::time::timeout_at(until, stream.read(&mut chunk)).await else {
(F) BRANCH crates/api/src/server.rs:21293:17 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21293:41 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21372:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:21744:13 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21744:32 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21748:80 False  [the false side fails the check, and its line ran]  ladder::Gate::RungFirst { missing: 13, runs: 13, ref owed, .. } if owed.len() == ladder::OWED_NAMED
(F) BRANCH crates/api/src/server.rs:21771:13 False  [the false side fails the check, and its line ran]  at_close.contains("ends at 15:30 IST and is asked for from 15:35")
(F) BRANCH crates/api/src/server.rs:21772:20 False  [the false side fails the check, and its line ran]  && at_close.contains("it is 15:30 now"),
(F) BRANCH crates/api/src/server.rs:21792:61 False  [the false side fails the check, and its line ran]  assert!(matches!(muhurat, DayKind::Open(session) if session != Session::full()));
(F) BRANCH crates/api/src/server.rs:21893:13 False  [the false side fails the check, and its line ran]  unmade.contains("store-fault-probe-shut/audit: ")
(F) BRANCH crates/api/src/server.rs:21894:20 False  [the false side fails the check, and its line ran]  && unmade.contains("Permission denied"),
(F) BRANCH crates/api/src/server.rs:21941:13 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21941:54 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21990:13 False  [the false side fails the check, and its line ran]  line.why.starts_with(pull::calendar::PAST_RECORD_FINDING)
(F) BRANCH crates/api/src/server.rs:21991:20 False  [the false side fails the check, and its line ran]  && line.why.contains(
(F) BRANCH crates/api/src/server.rs:21994:20 False  [the false side fails the check, and its line ran]  && line
(F) BRANCH crates/api/src/server.rs:22487:13 False  [the false side fails the check, and its line ran]  said.starts_with("transient — 0 failure(s) another attempt could change")
(F) BRANCH crates/api/src/server.rs:22488:20 False  [the false side fails the check, and its line ran]  && said.contains("2 of the store and not the vendor")
(F) BRANCH crates/api/src/server.rs:22489:20 False  [the false side fails the check, and its line ran]  && said.contains("and 0 it would repeat"),
(F) BRANCH crates/api/src/server.rs:22783:13 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:22783:47 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:22792:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:23662:13 False  [the false side fails the check, and its line ran]  !read.merged.contributed.contains(&Vendor::Groww)
(F) BRANCH crates/api/src/server.rs:23663:20 False  [the false side fails the check, and its line ran]  && !read.merged.contributed.contains(&Vendor::Dhan),
(F) BRANCH crates/api/src/server.rs:24133:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24258:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24314:13 False  [the else of a let-else that panics, whose line ran]  let [(0, segment)] = receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24317:13 False  [the else of a let-else that panics, whose line ran]  let [receipt] = segment.receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24612:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24646:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24650:17 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24650:46 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24651:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24655:17 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24655:48 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24695:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24704:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24739:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24836:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24926:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:25028:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:25392:17 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25392:51 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25448:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25448:51 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25555:13 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25555:45 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25751:13 False  [the false side fails the check, and its line ran]  body.contains(concat!("if let Some(warning) = ", "stamped {"))
(F) BRANCH crates/api/src/server.rs:25752:20 False  [the false side fails the check, and its line ran]  && body.contains(concat!(
(F) BRANCH crates/api/src/server.rs:25763:13 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25763:48 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25919:13 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:25919:45 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:26111:13 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:26111:46 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:26196:17 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26196:45 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26197:17 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26197:47 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26201:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26201:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26217:17 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26217:46 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26224:13 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26224:43 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26240:17 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26240:47 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26256:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26256:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26298:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26298:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26320:17 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26320:46 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26708:13 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:26708:52 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:27405:21 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27405:58 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27455:17 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:27455:48 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:28066:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:28066:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:28085:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:28085:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:28335:13 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28335:38 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28451:31 False  [the false side fails the check, and its line ran]  .all(|(t, l)| t == "pull" && *l == telemetry::Level::Debug),
(F) BRANCH crates/api/src/server.rs:28456:13 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28456:41 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28632:17 False  [the false side fails the check, and its line ran]  missing.starts_with(&format!("{program}: ")) && missing.contains("No such file"),
(F) BRANCH crates/api/src/server.rs:28632:65 False  [the false side fails the check, and its line ran]  missing.starts_with(&format!("{program}: ")) && missing.contains("No such file"),
(F) BRANCH crates/api/src/server.rs:28772:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28772:49 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28776:13 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:28776:42 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:29195:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = descriptor.transport else {
(F) BRANCH crates/api/src/server.rs:29270:13 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:29270:49 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:29274:13 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:29274:52 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:29481:13 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29481:44 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29537:17 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29537:40 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29537:62 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29544:13 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29544:35 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29554:17 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29554:37 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29554:56 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29563:13 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29563:34 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29563:58 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29781:13 False  [the false side fails the check, and its line ran]  words.starts_with("INTERRUPTED: an expired-F&O walk for 2026-07-01..2026-07-30")
(F) BRANCH crates/api/src/server.rs:29782:20 False  [the false side fails the check, and its line ran]  && words.contains("run the same walk again from /pull/fno")
(F) BRANCH crates/api/src/server.rs:29783:20 False  [the false side fails the check, and its line ran]  && !words.contains("/pull/spot")
(F) BRANCH crates/api/src/server.rs:29784:20 False  [the false side fails the check, and its line ran]  && !words.contains("press Pull"),
(F) BRANCH crates/api/src/server.rs:30028:13 False  [the false side fails the check, and its line ran]  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
(F) BRANCH crates/api/src/server.rs:30028:50 False  [the false side fails the check, and its line ran]  mine.field("web_built").is_some() && mine.field("autopilot_flies").is_some(),
(F) BRANCH crates/api/src/server.rs:30051:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:30492:13 False  [the false side fails the check, and its line ran]  whole_body.contains(&format!(r#""expected":{}"#, owed.len()))
(F) BRANCH crates/api/src/server.rs:30493:20 False  [the false side fails the check, and its line ran]  && holed_body.contains(&format!(r#""expected":{}"#, owed.len())),
(F) BRANCH crates/api/src/server.rs:30501:13 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30501:60 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30768:17 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30768:62 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30899:13 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30899:38 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30903:13 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:30903:51 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:31230:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:31580:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(dhan) = transport else {
(F) BRANCH crates/api/src/server.rs:31644:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:31651:17 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:31651:49 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:32103:17 False  [the false side fails the check, and its line ran]  rebuilt.contains("did not match its commit log") && rebuilt.contains("missing"),
(F) BRANCH crates/api/src/server.rs:32103:69 False  [the false side fails the check, and its line ran]  rebuilt.contains("did not match its commit log") && rebuilt.contains("missing"),
(F) BRANCH crates/api/src/server.rs:32137:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:32223:13 False  [the false side fails the check, and its line ran]  body.contains("NOT STARTED") && body.contains("clock names no usable date"),
(F) BRANCH crates/api/src/server.rs:32223:45 False  [the false side fails the check, and its line ran]  body.contains("NOT STARTED") && body.contains("clock names no usable date"),
(F) BRANCH crates/api/src/server.rs:32597:17 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:32597:52 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:32686:13 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:32686:57 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:33062:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:33281:26 False  [the false side fails the check, and its line ran]  .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
(F) BRANCH crates/api/src/server.rs:33508:17 False  [the else of a let-else that panics, whose line ran]  let Step::Again { wait_ms, throttled } = stepped else {
(F) BRANCH crates/api/src/server.rs:33857:13 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33857:45 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33861:13 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33861:44 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33925:13 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:33925:40 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:34488:13 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:34488:43 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:35281:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:35395:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:35516:13 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:35516:50 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:35530:13 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:35530:51 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:35548:13 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:35548:43 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:35636:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:35793:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:35925:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "chunk", 1)
(F) BRANCH crates/api/src/server.rs:35926:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:36118:17 False  [the false side fails the check, and its line ran]  crate::emitted::says(record, "instrument", "NIFTY")
(F) BRANCH crates/api/src/server.rs:36119:24 False  [the false side fails the check, and its line ran]  && crate::emitted::says(record, "feed", "dhan"),
(F) BRANCH crates/api/src/server.rs:36123:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "index", 1)
(F) BRANCH crates/api/src/server.rs:36124:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:36533:13 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:36533:33 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:36732:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "rung", "1min")
(F) BRANCH crates/api/src/server.rs:36733:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "target", asked.target.label()),
(F) BRANCH crates/api/src/server.rs:36748:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(mine, "failed", 0)
(F) BRANCH crates/api/src/server.rs:36749:20 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(mine, "bars_stored", 0),
(F) BRANCH crates/api/src/server.rs:36798:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "month", "2026-08")
(F) BRANCH crates/api/src/server.rs:36799:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "day", "2026-08-04")
(F) BRANCH crates/api/src/server.rs:36800:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "window", "2026-08-03..=2026-08-05")
(F) BRANCH crates/api/src/server.rs:36801:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "why", "the store refused the month"),
(F) BRANCH crates/api/src/server.rs:36853:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(ordinary, "status", 200)
(F) BRANCH crates/api/src/server.rs:36854:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(ordinary, "method", "GET")
(F) BRANCH crates/api/src/server.rs:36855:20 False  [the false side fails the check, and its line ran]  && ordinary.field("micros").is_some(),
(F) BRANCH crates/api/src/server.rs:37014:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:37373:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:37373:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:37422:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:37422:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) LINE crates/api/src/server_cloud_mutant_tests_e.rs:256  [failing arm of the statement at 255, which ran]  matches!(
(F) LINE crates/api/src/server_cloud_mutant_tests_e.rs:267  [panic/unreachable line]  panic!("the flush re-arms the head clock");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:266:9 False  [the else of a let-else that panics, whose line ran]  let HeadState::Awaiting { deadline, .. } = wrapped.state else {
(F) BRANCH crates/api/src/sources_day_truth_tests.rs:784:9 False  [the false side fails the check, and its line ran]  why.contains("UNVERIFIED") && why.contains("no EQ mapping") && why.contains("RELIANCE"),
(F) BRANCH crates/api/src/sources_day_truth_tests.rs:784:39 False  [the false side fails the check, and its line ran]  why.contains("UNVERIFIED") && why.contains("no EQ mapping") && why.contains("RELIANCE"),
(F) BRANCH crates/api/src/sources_day_truth_tests.rs:784:72 False  [the false side fails the check, and its line ran]  why.contains("UNVERIFIED") && why.contains("no EQ mapping") && why.contains("RELIANCE"),
(F) LINE crates/api/src/sources_tests.rs:983  [failing arm of the statement at 974, which ran]  a["marks"]
(F) LINE crates/api/src/sources_tests.rs:1365  [failing arm of the statement at 1358, which ran]  matches!(
(F) BRANCH crates/api/src/sources_tests.rs:1180:25 False  [the false side fails the check, and its line ran]  .all(|flag| flag != "basis_differs" && flag != "basis_step"),
(F) BRANCH crates/api/src/sources_tests.rs:2430:13 False  [the false side fails the check, and its line ran]  assert!(!basis_differs(1_002, 1_000) && basis_differs(1_003, 1_000));
(F) BRANCH crates/api/src/sources_tests.rs:2430:45 False  [the false side fails the check, and its line ran]  assert!(!basis_differs(1_002, 1_000) && basis_differs(1_003, 1_000));
(F) LINE crates/api/src/speed_tests.rs:930  [failing arm of the statement at 927, which ran]  row_of()
(F) BRANCH crates/api/src/speed_tests.rs:489:9 False  [the false side fails the check, and its line ran]  unknown.starts_with("HTTP/1.1 404 ")
(F) BRANCH crates/api/src/speed_tests.rs:490:16 False  [the false side fails the check, and its line ran]  && unknown.contains("no speed row is named '/no-such-route-anywhere'"),
(F) BRANCH crates/api/src/speed_tests.rs:543:9 False  [the false side fails the check, and its line ran]  empty_what.starts_with("HTTP/1.1 400 ")
(F) BRANCH crates/api/src/speed_tests.rs:544:16 False  [the false side fails the check, and its line ran]  && empty_what.contains("\"written\":false")
(F) BRANCH crates/api/src/speed_tests.rs:545:16 False  [the false side fails the check, and its line ran]  && empty_what.contains("what="),
(F) BRANCH crates/api/src/speed_tests.rs:554:9 False  [the false side fails the check, and its line ran]  too_long.starts_with("HTTP/1.1 413 ") && too_long.contains("\"written\":false"),
(F) BRANCH crates/api/src/speed_tests.rs:554:50 False  [the false side fails the check, and its line ran]  too_long.starts_with("HTTP/1.1 413 ") && too_long.contains("\"written\":false"),
(F) BRANCH crates/api/src/speed_tests.rs:561:9 False  [the false side fails the check, and its line ran]  cross.starts_with("HTTP/1.1 403 ") && !cross.contains("\"written\""),
(F) BRANCH crates/api/src/speed_tests.rs:561:47 False  [the false side fails the check, and its line ran]  cross.starts_with("HTTP/1.1 403 ") && !cross.contains("\"written\""),
(F) BRANCH crates/api/src/speed_tests.rs:583:13 False  [the false side fails the check, and its line ran]  assert!(figures.p50_micros <= figures.p99_micros && figures.p99_micros <= figures.max_micros);
(F) BRANCH crates/api/src/speed_tests.rs:583:57 False  [the false side fails the check, and its line ran]  assert!(figures.p50_micros <= figures.p99_micros && figures.p99_micros <= figures.max_micros);
(F) BRANCH crates/api/src/speed_tests.rs:696:53 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:696:62 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:696:71 False  [the false side fails the check, and its line ran]  (Some(i), Some(t), Some(d), Some(r)) if i < d && t < d && d < r
(F) BRANCH crates/api/src/speed_tests.rs:715:53 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:715:62 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:715:71 False  [the false side fails the check, and its line ran]  (Some(g), Some(e), Some(a), Some(b)) if g < e && e < a && a < b
(F) BRANCH crates/api/src/speed_tests.rs:716:14 False  [the false side fails the check, and its line ran]  ) && main.contains(concat!("run_held(&args, ", "stop, &mut held)")),
(F) BRANCH crates/api/src/speed_tests.rs:724:54 False  [the false side fails the check, and its line ran]  matches!((ran, ended), (Some(r), Some(e)) if r < e),
(F) BRANCH crates/api/src/speed_tests.rs:745:61 False  [the false side fails the check, and its line ran]  matches!((graceful, flushed), (Some(g), Some(f)) if g < f),
(F) BRANCH crates/api/src/speed_tests.rs:819:74 False  [the false side fails the check, and its line ran]  matches!((armed, raced, flushed), (Some(a), Some(r), Some(f)) if a < r && r < f),
(F) BRANCH crates/api/src/speed_tests.rs:819:83 False  [the false side fails the check, and its line ran]  matches!((armed, raced, flushed), (Some(a), Some(r), Some(f)) if a < r && r < f),
(F) BRANCH crates/api/src/speed_tests.rs:895:78 False  [the false side fails the check, and its line ran]  matches!((started, landed, finished), (Some(s), Some(l), Some(f)) if s < l && l < f),
(F) BRANCH crates/api/src/speed_tests.rs:895:87 False  [the false side fails the check, and its line ran]  matches!((started, landed, finished), (Some(s), Some(l), Some(f)) if s < l && l < f),
(F) BRANCH crates/api/src/speed_tests.rs:976:75 False  [the false side fails the check, and its line ran]  matches!((shared, scrub, counted), (Some(a), Some(b), Some(c)) if a < b && b < c),
(F) BRANCH crates/api/src/speed_tests.rs:976:84 False  [the false side fails the check, and its line ran]  matches!((shared, scrub, counted), (Some(a), Some(b), Some(c)) if a < b && b < c),
(F) BRANCH crates/api/src/speed_tests.rs:1151:9 False  [the false side fails the check, and its line ran]  said.contains("started_ms: 5") && said.contains("cap: 3"),
(F) BRANCH crates/api/src/speed_tests.rs:1151:43 False  [the false side fails the check, and its line ran]  said.contains("started_ms: 5") && said.contains("cap: 3"),
(F) BRANCH crates/api/src/speed_tests.rs:1373:31 False  [the false side fails the check, and its line ran]  .all(|record| record.level == telemetry::Level::Error
(F) LINE crates/api/src/store_wire.rs:347  [panic/unreachable line]  panic!("warm read rebuilt")
(F) LINE crates/api/src/store_wire.rs:403  [panic/unreachable line]  panic!("GDFL's body was dropped by another vendor's read")
(F) LINE crates/api/src/store_wire.rs:519  [panic/unreachable line]  panic!("derived twice")
(F) BRANCH crates/api/src/topjson.rs:730:17 False  [the false side fails the check, and its line ran]  assert!(whole.contains("?370") && whole.contains("?383"), "(10)");
(F) BRANCH crates/api/src/topjson.rs:730:43 False  [the false side fails the check, and its line ran]  assert!(whole.contains("?370") && whole.contains("?383"), "(10)");
(F) BRANCH crates/api/src/trades.rs:860:13 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/trades.rs:860:32 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/trades.rs:1342:13 False  [the false side fails the check, and its line ran]  body.contains("the child was absent at its open.")
(F) BRANCH crates/api/src/trades.rs:1343:20 False  [the false side fails the check, and its line ran]  && body.contains("could not be checked for absence"),
(F) LINE crates/api/src/verify.rs:397  [failing arm of the statement at 394, which ran]  empty.say()
(F) LINE crates/api/src/verify.rs:427  [failing arm of the statement at 423, which ran]  bad.say()
(F) LINE crates/api/src/verify.rs:516  [panic/unreachable line]  panic!("a recovered counter still loads: {:?}", dhan.state.name());
(F) LINE crates/api/src/verify.rs:690  [failing arm of the statement at 687, which ran]  first.say()
(F) BRANCH crates/api/src/verify.rs:515:13 False  [the else of a let-else that panics, whose line ran]  let Census::Held { ref manifest } = dhan.state else {
(F) LINE crates/api/tests/cloud_mutants_bars.rs:504  [panic/unreachable line]  panic!("February is owed: {gated:?}");
(F) LINE crates/api/tests/cloud_mutants_bars.rs:615  [panic/unreachable line]  panic!("every month is owed: {gated:?}");
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:503:13 False  [the else of a let-else that panics, whose line ran]  let Gate::RungFirst { owed, .. } = &gated else {
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:614:13 False  [the else of a let-else that panics, whose line ran]  let Gate::RungFirst { needs, owed, .. } = &gated else {
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:950:13 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:950:52 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2121:13 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2121:48 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2125:13 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2125:48 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2359:13 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2359:55 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2654:13 False  [the false side fails the check, and its line ran]  why.contains(&format!("{} is not a regular file", srt.display()))
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2655:20 False  [the false side fails the check, and its line ran]  && why.starts_with("NSE-INDEX-NIFTY 2025-05: 3 rows read: "),
(F) LINE crates/api/tests/logs_r3_24.rs:351  [panic/unreachable line]  panic!("{query}: no end in 200 pages, {} line(s) read", seqs.len());
(U) LINE crates/api/src/bars_sort.rs:518  [run_at ranges over 0..plan.runs.len() fixed at the loop's start, and nothing in the body shrinks plan.runs (view reads slots and key and writes only Slot::file/failed, bars_series.rs:1647-1710): get(run_at) is Some]  continue;
(U) LINE crates/api/src/bars_sort.rs:1291  [array ranges over 0..self.runs.len(); the body calls only RunArray::group on the element, which cannot change the Vec's length]  continue;
(U) LINE crates/api/src/bars_sort.rs:1461  [array ranges over 0..arrays.runs.len(); group, at_or_after, partition and Arrays::advance (bars_sort.rs:1258-1281) never push, remove or truncate arrays.runs]  continue;
(U) LINE crates/api/src/bars_sort.rs:1568  [heads gets one entry per cursor (an Err returns early), cursors is a fixed slice, heads is never resized, and best is set only from an index whose head is Some; 1568 is the if-let's closing brace (its implicit else)]  }
(U) LINE crates/api/src/bars_sort.rs:2085  [cuts is built inside page_once by cut(root, plans, ..), and cut.series is the enumerate index over that same plans slice, which cannot change length: plans.get(_mut)(cut.series) is Some and map_or_else's default never runs]  continue;
(U) LINE crates/api/src/bars_sort.rs:2172  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  continue;
(U) LINE crates/api/src/bars_sort.rs:2175  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  continue;
(U) LINE crates/api/src/bars_sort.rs:2178  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  continue;
(U) LINE crates/api/src/bars_sort.rs:2216  [a Cut piece of a non-null series either gets its entries filled or is replaced by Piece::Read in the loop before, and a null series continues before this match; null_series depends only on column and rules, fixed for the call]  Some(None) => continue,
(U) LINE crates/api/src/bars_sort.rs:2220  [cuts is built inside page_once by cut(root, plans, ..), and cut.series is the enumerate index over that same plans slice, which cannot change length: plans.get(_mut)(cut.series) is Some and map_or_else's default never runs]  || (Arc::from(""), Arc::from("")),
(U) LINE crates/api/src/bars_sort.rs:2303  [cuts is built inside page_once by cut(root, plans, ..), and cut.series is the enumerate index over that same plans slice, which cannot change length: plans.get(_mut)(cut.series) is Some and map_or_else's default never runs]  continue;
(U) LINE crates/api/src/bars_sort.rs:2381  [a Found::Kept comes only from arrays.kept, whose key.series is u32::try_from(cut.series) and run is cut.run: in range as above]  continue;
(U) LINE crates/api/src/bars_sort.rs:2384  [a Found::Kept comes only from arrays.kept, whose key.series is u32::try_from(cut.series) and run is cut.run: in range as above]  continue;
(U) LINE crates/api/src/bars_sort.rs:2407  [a Found::Run names RunArray::series, set to u32::try_from(cut.series): < plans.len()]  continue;
(U) LINE crates/api/src/bars_sort.rs:2531  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  continue;
(U) LINE crates/api/src/bars_sort.rs:2566  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  continue;
(U) BRANCH crates/api/src/bars_sort.rs:517:17 False  [run_at ranges over 0..plan.runs.len() fixed at the loop's start, and nothing in the body shrinks plan.runs (view reads slots and key and writes only Slot::file/failed, bars_series.rs:1647-1710): get(run_at) is Some]  let Some(run) = plan.runs.get(run_at).copied() else {
(U) BRANCH crates/api/src/bars_sort.rs:1290:17 False  [array ranges over 0..self.runs.len(); the body calls only RunArray::group on the element, which cannot change the Vec's length]  let Some(run) = self.runs.get_mut(array) else {
(U) BRANCH crates/api/src/bars_sort.rs:1333:20 False  [the method takes &self, so self.runs cannot change, and array < self.runs.len()]  if let Some(run) = self.runs.get(array)
(U) BRANCH crates/api/src/bars_sort.rs:1460:13 False  [array ranges over 0..arrays.runs.len(); group, at_or_after, partition and Arrays::advance (bars_sort.rs:1258-1281) never push, remove or truncate arrays.runs]  let Some(run) = arrays.runs.get_mut(array) else {
(U) BRANCH crates/api/src/bars_sort.rs:1492:16 False  [array ranges over 0..arrays.runs.len(); group, at_or_after, partition and Arrays::advance (bars_sort.rs:1258-1281) never push, remove or truncate arrays.runs]  if let Some(run) = arrays.runs.get_mut(array)
(U) BRANCH crates/api/src/bars_sort.rs:1559:16 False  [heads gets one entry per cursor (an Err returns early), cursors is a fixed slice, heads is never resized, and best is set only from an index whose head is Some; 1568 is the if-let's closing brace (its implicit else)]  if let Some(Some((_, found))) = heads.get(at) {
(U) BRANCH crates/api/src/bars_sort.rs:1562:16 False  [heads gets one entry per cursor (an Err returns early), cursors is a fixed slice, heads is never resized, and best is set only from an index whose head is Some; 1568 is the if-let's closing brace (its implicit else)]  if let Some(cursor) = cursors.get_mut(at) {
(U) BRANCH crates/api/src/bars_sort.rs:1565:20 False  [heads gets one entry per cursor (an Err returns early), cursors is a fixed slice, heads is never resized, and best is set only from an index whose head is Some; 1568 is the if-let's closing brace (its implicit else)]  if let Some(slot) = heads.get_mut(at) {
(U) BRANCH crates/api/src/bars_sort.rs:1898:28 False  [*piece was matched as Piece::Cut by the let-else just above, and nothing between assigns it]  if let Piece::Cut { entries, .. } = piece {
(U) BRANCH crates/api/src/bars_sort.rs:2084:17 False  [cuts is built inside page_once by cut(root, plans, ..), and cut.series is the enumerate index over that same plans slice, which cannot change length: plans.get(_mut)(cut.series) is Some and map_or_else's default never runs]  let Some(plan) = plans.get_mut(cut.series) else {
(U) BRANCH crates/api/src/bars_sort.rs:2151:20 False  [plans.get(cut.series) is Some as above, and cut.run is run_at < plan.runs.len() from cut; nothing page_once calls between changes runs]  && let Some(run) = plans
(U) BRANCH crates/api/src/bars_sort.rs:2171:17 False  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  let Some((found, stamp)) = views.get(&(cut.series, slot)).cloned() else {
(U) BRANCH crates/api/src/bars_sort.rs:2174:17 False  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  let Ok(view) = found.as_ref() else {
(U) BRANCH crates/api/src/bars_sort.rs:2177:17 False  [an Indexed or Cut piece is pushed only in cut's Ok(walked) arm, after its view is in views under (series, slot); views is created in page_once and afterwards only read, and the piece's run indexes that view's runs behind an Arc]  let Some(info) = view.walked.runs.get(run).copied() else {
(U) BRANCH crates/api/src/bars_sort.rs:2302:25 False  [cuts is built inside page_once by cut(root, plans, ..), and cut.series is the enumerate index over that same plans slice, which cannot change length: plans.get(_mut)(cut.series) is Some and map_or_else's default never runs]  let Some(plan) = plans.get_mut(cut.series) else {
(U) BRANCH crates/api/src/bars_sort.rs:2315:28 False  [inside if extremes.is_some(), the only extremes = None is in the owed > ceiling arm, and this loop is in the else arm, which assigns nothing to extremes]  if let Some(kept) = extremes.as_mut() {
(U) BRANCH crates/api/src/bars_sort.rs:2380:21 False  [a Found::Kept comes only from arrays.kept, whose key.series is u32::try_from(cut.series) and run is cut.run: in range as above]  let Some(plan) = plans.get_mut(series as usize) else {
(U) BRANCH crates/api/src/bars_sort.rs:2383:21 False  [a Found::Kept comes only from arrays.kept, whose key.series is u32::try_from(cut.series) and run is cut.run: in range as above]  let Some(slot) = plan.runs.get(run).map(|at| at.slot) else {
(U) BRANCH crates/api/src/bars_sort.rs:2406:21 False  [a Found::Run names RunArray::series, set to u32::try_from(cut.series): < plans.len()]  let Some(plan) = plans.get_mut(series as usize) else {
(U) BRANCH crates/api/src/bars_sort.rs:2515:28 False  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  if let Some(summary) = summary {
(U) BRANCH crates/api/src/bars_sort.rs:2530:17 False  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  let Some(plan) = plans.get(cut.series) else {
(U) BRANCH crates/api/src/bars_sort.rs:2565:13 False  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  let Some(plan) = plans.get_mut(series) else {
(U) BRANCH crates/api/src/bars_sort.rs:2593:20 False  [views and cuts are built inside extremes_of, so every Indexed piece's summary is Some and plans.get(cut.series) is Some; read entries carry cut.series; 2593 is reached only when whole found plan.slots.get(slot) Some and read_stretch does not change slots' length]  && let Some(at) = plan.slots.get(slot)
(U) LINE crates/api/src/booleancampaignjson.rs:76  [a size bound: 8 rungs, each at most RESEARCH_FAMILY_CAPACITY_V1 = 215 catalogs of under 600 bytes and a reason of at most 1,024 bytes (crates/cli/src/boolean_campaign_codec.rs:14-23, 192-204): under 1.1 MB; the qualified answer under 60 KB; both below 8,388,608]  refused(
(U) LINE crates/api/src/booleancampaignjson.rs:78  [a size bound: 8 rungs, each at most RESEARCH_FAMILY_CAPACITY_V1 = 215 catalogs of under 600 bytes and a reason of at most 1,024 bytes (crates/cli/src/boolean_campaign_codec.rs:14-23, 192-204): under 1.1 MB; the qualified answer under 60 KB; both below 8,388,608]  "campaign comparison exceeds response byte ceiling; no partial rows returned",
(U) BRANCH crates/api/src/booleancampaignjson.rs:73:16 False  [a size bound: 8 rungs, each at most RESEARCH_FAMILY_CAPACITY_V1 = 215 catalogs of under 600 bytes and a reason of at most 1,024 bytes (crates/cli/src/boolean_campaign_codec.rs:14-23, 192-204): under 1.1 MB; the qualified answer under 60 KB; both below 8,388,608]  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
(U) LINE crates/api/src/booleanjson.rs:141  [a size bound: a page holds at most limit <= 256 rows, the largest row a coordinate whose program text is at most 17,265 bytes (MAX_INSTRUCTIONS * 3 tokens, crates/vocab/src/expression.rs:15, 400-459) plus under 2 KiB of fixed fields, two grid summaries of about 3.5 KB: under 5 MB, below MAX_RESPONSE_BYTES = 8,388,608 (detail.rs:32) on every input]  refused(
(U) LINE crates/api/src/booleanjson.rs:143  [a size bound: a page holds at most limit <= 256 rows, the largest row a coordinate whose program text is at most 17,265 bytes (MAX_INSTRUCTIONS * 3 tokens, crates/vocab/src/expression.rs:15, 400-459) plus under 2 KiB of fixed fields, two grid summaries of about 3.5 KB: under 5 MB, below MAX_RESPONSE_BYTES = 8,388,608 (detail.rs:32) on every input]  "catalog response exceeds byte admission; no prefix returned",
(U) BRANCH crates/api/src/booleanjson.rs:138:16 False  [a size bound: a page holds at most limit <= 256 rows, the largest row a coordinate whose program text is at most 17,265 bytes (MAX_INSTRUCTIONS * 3 tokens, crates/vocab/src/expression.rs:15, 400-459) plus under 2 KiB of fixed fields, two grid summaries of about 3.5 KB: under 5 MB, below MAX_RESPONSE_BYTES = 8,388,608 (detail.rs:32) on every input]  if result.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
(U) BRANCH crates/api/src/booleanlaunch.rs:450:12 False  [booleanevidencejson::search_policy returns admission_projection::policy, whose tail expression is the object literal json!({"digest":..,"values":rows}) (booleanadmission_projection.rs:76-133), so as_object_mut() is Some]  if let Some(fields) = policy.as_object_mut() {
(U) LINE crates/api/src/calendar_of.rs:3086  [the loop takes the open run at every non-Unmeasured day, and a Calendar's last day is never Unmeasured: from_observed sets the last observed day's slot to Open or OpenLengthUnmeasured and withhold_closed changes only Closed slots (crates/pull/src/calendar.rs:5009-5106, 5162-5187)]  flush(&mut out, done);
(U) BRANCH crates/api/src/calendar_of.rs:585:28 False  [day comes from by_month, which gets a day only right after inserting it into traded (549-553), and traded loses no key before into_iter: get_mut is Some]  if let Some(slot) = traded.get_mut(day) {
(U) BRANCH crates/api/src/calendar_of.rs:684:24 False  [month is a pull::session::Day, validated into 1970..=9999 with months 1..=12 at construction (crates/pull/src/session.rs), exactly the range YearMonth::new accepts (crates/store/src/path.rs:583-591), so YearMonth::new(month.year(), month.month()) is Ok]  && let Ok(named) = YearMonth::new(month.year(), month.month())
(U) BRANCH crates/api/src/calendar_of.rs:3085:12 True  [the loop takes the open run at every non-Unmeasured day, and a Calendar's last day is never Unmeasured: from_observed sets the last observed day's slot to Open or OpenLengthUnmeasured and withhold_closed changes only Closed slots (crates/pull/src/calendar.rs:5009-5106, 5162-5187)]  if let Some(done) = run {
(U) LINE crates/api/src/calendar_record.rs:341  [every store::path::YearMonth is year 1970..=9999 and month 1..=12 (YearMonth::new, crates/store/src/path.rs:583-591, the only way to make one), and pull::session::Day::new accepts the first day of every such month (a Day is validated into 1970..=9999)]  continue;
(U) LINE crates/api/src/calendar_record.rs:356  [the line before resizes counts to at + 1 whenever its length is at or below at, so get_mut(at) is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/calendar_record.rs:502  [auction_start takes no argument: high starts at pull::calendar::SOURCED_LAST_DAY, 2026-12-31 (crates/pull/src/calendar.rs:982-990), and auction_day is cash_auction_eligibility_required, day >= 2026-08-03 (crates/pull/src/vendor.rs:1443-1447), so the record's last day is always an auction day]  return None;
(U) LINE crates/api/src/calendar_record.rs:615  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  None => serde_json::json!({
(U) LINE crates/api/src/calendar_record.rs:616  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "from": iso(from),
(U) LINE crates/api/src/calendar_record.rs:617  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "to": iso(to),
(U) LINE crates/api/src/calendar_record.rs:618  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "regular": serde_json::Value::Null,
(U) LINE crates/api/src/calendar_record.rs:619  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "why": "the derivatives session table carries no verified hours for these days",
(U) LINE crates/api/src/calendar_record.rs:644  [day comes only from the const tables SPECIAL_SESSIONS and HALTS, past owed_minutes(day)? and record_code(day)?, and every listed day is Open under exchange_kind_of (calendar.rs:1521-1528, 1642-1700; calendar_tradable_tests.rs:60-61)]  return None;
(U) BRANCH crates/api/src/calendar_record.rs:180:20 True  [the right operand runs only when special_session(day) is None, and every Open(session) with session != Session::full() falls on an IRREGULAR day, each a SPECIAL_SESSIONS day (crates/pull/src/calendar.rs:513-525, 1365-1477; pinned by the_special_table_is_sorted_and_shares_irregulars_windows)]  || session != Session::full()
(U) BRANCH crates/api/src/calendar_record.rs:340:17 False  [every store::path::YearMonth is year 1970..=9999 and month 1..=12 (YearMonth::new, crates/store/src/path.rs:583-591, the only way to make one), and pull::session::Day::new accepts the first day of every such month (a Day is validated into 1970..=9999)]  let Ok(one) = pull::session::Day::new(month.year(), month.month(), 1) else {
(U) BRANCH crates/api/src/calendar_record.rs:354:24 False  [the line before resizes counts to at + 1 whenever its length is at or below at, so get_mut(at) is Some]  if let Some(slot) = counts.get_mut(at) {
(U) BRANCH crates/api/src/calendar_record.rs:501:8 True  [auction_start takes no argument: high starts at pull::calendar::SOURCED_LAST_DAY, 2026-12-31 (crates/pull/src/calendar.rs:982-990), and auction_day is cash_auction_eligibility_required, day >= 2026-08-03 (crates/pull/src/vendor.rs:1443-1447), so the record's last day is always an auction day]  if !auction_day(high) {
(U) BRANCH crates/api/src/calendar_record.rs:580:27 False  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990), and those two rows' (open, close, source) differ, so the walk meets exactly two eras and rows.len() is at most 2 when it is asked]  while from <= last && rows.len() <= 8 {
(U) BRANCH crates/api/src/calendar_record.rs:643:17 False  [day comes only from the const tables SPECIAL_SESSIONS and HALTS, past owed_minutes(day)? and record_code(day)?, and every listed day is Open under exchange_kind_of (calendar.rs:1521-1528, 1642-1700; calendar_tradable_tests.rs:60-61)]  let DayKind::Open(session) = pull::calendar::exchange_kind_of(day) else {
(U) LINE crates/api/src/calendar_route.rs:1298  [warm queues only keys (feed, None|Some(rung), symbol) with symbol one of FIRST_SYMBOLS, and question spells /calendar.json?feed=<a lower-case ASCII word>[&rung=<a [0-9a-z] word>][&symbol=<ASCII>] under 80 bytes: text.parse::<Uri>() cannot fail for any key warm queues]  Err(refused) => refused,
(U) BRANCH crates/api/src/calendar_route.rs:548:12 False  [body is the object literal json!({"status", "feed", "rung", "symbol", "retry_after_ms"}) two lines above, and building_answer's two callers (building and building_census, calendar_route.rs) each pass &serde_json::json!({...}), an object literal, as fields, so both are Some]  if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
(U) LINE crates/api/src/census_cache.rs:471  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  return (
(U) LINE crates/api/src/census_cache.rs:472  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  Arc::new(Slot {
(U) LINE crates/api/src/census_cache.rs:473  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  read_at: std::time::SystemTime::now(),
(U) LINE crates/api/src/census_cache.rs:474  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  census: fresh_read(&site.store_root, vendor, None),
(U) LINE crates/api/src/census_cache.rs:475  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  entries: OnceLock::new(),
(U) LINE crates/api/src/census_cache.rs:476  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  }),
(U) LINE crates/api/src/census_cache.rs:477  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  Freshness::Current,
(U) LINE crates/api/src/census_cache.rs:478  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  None,
(U) LINE crates/api/src/census_cache.rs:479  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  );
(U) LINE crates/api/src/census_cache.rs:616  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  return false;
(U) LINE crates/api/src/census_cache.rs:628  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  return;
(U) BRANCH crates/api/src/census_cache.rs:468:9 False  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  let Some(cell) = vendors.cells.get(vendor as usize) else {
(U) BRANCH crates/api/src/census_cache.rs:615:9 False  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  let Some(cell) = vendors.cells.get(vendor as usize) else {
(U) BRANCH crates/api/src/census_cache.rs:627:17 False  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  let Some(cell) = mine.cells.get(vendor as usize) else {
(U) BRANCH crates/api/src/census_cache.rs:768:16 False  [stamps is manifest_stamps(&site.store_root), made two lines above in this function, whose manifests is collected from Vendor::ALL (five, census_cache.rs:1735-1739), and vendor as usize is below 5 (crates/core/src/vendor.rs:46-53), so get_mut is Some]  && let Some(at) = stamps.manifests.get_mut(vendor as usize)
(U) BRANCH crates/api/src/census_cache.rs:1226:12 False  [cells is [Cell; Vendor::ALL.len()] (census_cache.rs:127), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5 and cells.get(vendor as usize) is Some]  if let Some(cell) = vendors.cells.get(vendor as usize) {
(U) BRANCH crates/api/src/constituents.rs:534:20 False  [per_vendor is [Vec<VendorId>; Vendor::ALL.len()] (constituents.rs:498), five slots, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so get_mut is Some]  && let Some(slot) = self.per_vendor.get_mut(vendor as usize)
(U) BRANCH crates/api/src/dayflags_rules.rs:1190:20 False  [a judged run starts only at a flat bar on a judged minute (not Aside), and that bar leaves the run-through Some, continued or restarted at its value; a judged run continues only on a flat bar of the same value one minute on, which continues the run-through too; so after every bar run.is_some() implies through.is_some(), and close_run is called only with the previous bar's pair and after the loop]  if let Some(through) = through {
(U) BRANCH crates/api/src/fault_log.rs:186:34 False  [unnamed grows only behind runs_named >= MAX_NAMED_FAULTS (16), runs_named grows only beside a push into named, absorb moves the other log's named sentences (16 or more whenever its unnamed > 0) in first, and nothing in fault_log.rs removes from named; so unnamed > 0 implies named is non-empty, and this operand is asked only when named is empty]  self.named.is_empty() && self.unnamed == 0 && self.open.is_none()
(U) BRANCH crates/api/src/instruments_route.rs:204:20 False  [the tag is format!("\"{}\"", hex32(blake3(body))) (store_route.rs:734-740): a double quote, 64 hex digits and a double quote, all visible ASCII, which HeaderValue::from_str accepts]  if let Ok(value) = axum::http::HeaderValue::from_str(&tagged.etag) {
(U) BRANCH crates/api/src/operation_audit.rs:226:20 False  [id is cli::operation_audit::Attempt::id, a u64 (crates/cli/src/operation_audit.rs:425), whose to_string is ASCII digits only, every one a visible byte HeaderValue::from_str accepts]  if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
(U) LINE crates/api/src/provenance.rs:412  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/provenance.rs:702  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/provenance.rs:1000  [journal.page(n + 1, 0, 1) is reached only for n < records, and read_page then reads exactly one 256-byte record or fails (audit.rs:1962-2045): every Ok holds exactly one entry, so entries.first() is Some]  None => json!({
(U) LINE crates/api/src/provenance.rs:1001  [journal.page(n + 1, 0, 1) is reached only for n < records, and read_page then reads exactly one 256-byte record or fails (audit.rs:1962-2045): every Ok holds exactly one entry, so entries.first() is Some]  "source": "audit/pull.journal",
(U) LINE crates/api/src/provenance.rs:1002  [journal.page(n + 1, 0, 1) is reached only for n < records, and read_page then reads exactly one 256-byte record or fails (audit.rs:1962-2045): every Ok holds exactly one entry, so entries.first() is Some]  "error": format!("record {} was not read", record.journal_ordinal),
(U) LINE crates/api/src/provenance.rs:1038  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  return json!({ "source": source, "error": "the month names no calendar day" });
(U) LINE crates/api/src/provenance.rs:1610  [fold_record's one caller passes the items of buffer.chunks_exact(audit::RECORD_LEN) (provenance.rs, the record walk of runs_read), and chunks_exact yields only slices of exactly that length, so the conversion to &[u8; RECORD_LEN] is Ok]  return;
(U) LINE crates/api/src/provenance.rs:1680  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  return json!({ "source": source, "error": "the month names no calendar day" });
(U) LINE crates/api/src/provenance.rs:1951  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  return Value::Null;
(U) LINE crates/api/src/provenance.rs:1961  [window_json reads Feed::ALL descriptors' window (compile-time DESCRIPTORS): only ZERODHA sets one, with SpanStanding::Operator (crates/pull/src/vendor.rs:6343-6348)]  pull::vendor::SpanStanding::Measured => "measured",
(U) LINE crates/api/src/provenance.rs:1968  [only a Zerodha month gets here, its window runs 2018-01-01..2026-09-30 (vendor.rs:4376-4378), and days() gives a whole month: a month is wholly before, wholly after or wholly inside the window, never touching it only partly]  Some(PARTLY_OUTSIDE_WINDOW)
(U) BRANCH crates/api/src/provenance.rs:407:16 False  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some]  if let Some(object) = body.as_object_mut() {
(U) BRANCH crates/api/src/provenance.rs:670:12 False  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some]  if let Some(object) = body.as_object_mut() {
(U) BRANCH crates/api/src/provenance.rs:1037:9 False  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  let Some((from, to)) = asked.days() else {
(U) BRANCH crates/api/src/provenance.rs:1609:9 False  [fold_record's one caller passes the items of buffer.chunks_exact(audit::RECORD_LEN) (provenance.rs, the record walk of runs_read), and chunks_exact yields only slices of exactly that length, so the conversion to &[u8; RECORD_LEN] is Ok]  let Ok(image) = <&[u8; audit::RECORD_LEN]>::try_from(image) else {
(U) BRANCH crates/api/src/provenance.rs:1621:23 False  [fold_record reaches the match only past kind == Kind::Run.code() on image byte 118, and decode reads its kind from the same byte (audit.rs:222, 1143-1146, 1173-1175): an Ok record here is a Run]  Ok(record) if record.kind == audit::Kind::Run => index.push(PullRun {
(U) BRANCH crates/api/src/provenance.rs:1679:9 False  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  let Some((from, to)) = asked.days() else {
(U) BRANCH crates/api/src/provenance.rs:1946:9 False  [Asked::days (provenance.rs:217-223) is Some for every YearMonth: YearMonth::new admits 1970..=9999 and months 1..=12 (crates/store/src/path.rs:571-591), and Day::new(y, m, 1) accepts exactly that range (crates/pull/src/session.rs:380-398)]  let Some((first, last)) = (Asked {
(U) BRANCH crates/api/src/provenance.rs:1967:19 True  [only a Zerodha month gets here, its window runs 2018-01-01..2026-09-30 (vendor.rs:4376-4378), and days() gives a whole month: a month is wholly before, wholly after or wholly inside the window, never touching it only partly]  } else if touches {
(U) LINE crates/api/src/scrub_route.rs:551  [the keys are collected from by_pass[pass] before the loop; the loop removes only the current key, and file inserts only into by_pass[pass - 1]: each later key is still there when its turn comes]  continue;
(U) LINE crates/api/src/scrub_route.rs:652  [the keys are collected per source from by_pass[source] just before its loop; the loop removes only the current key and file inserts into by_pass[pass] with source != pass]  continue;
(U) LINE crates/api/src/scrub_route.rs:836  [next is ranges.iter_mut().map(pull).collect(), so next.len() == ranges.len(), and at enumerates next; nothing in the loop pushes or pops either]  break;
(U) LINE crates/api/src/scrub_route.rs:1052  [KEPT is [Mutex<Option<Kept>>; Vendor::ALL.len()] (scrub_route.rs:1010-1011), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so KEPT.get(vendor as usize) is Some]  return Ok(read(&Index::default()));
(U) BRANCH crates/api/src/scrub_route.rs:546:17 False  [the keys are collected from by_pass[pass] before the loop; the loop removes only the current key, and file inserts only into by_pass[pass - 1]: each later key is still there when its turn comes]  let Some(mut finding) = self
(U) BRANCH crates/api/src/scrub_route.rs:647:21 False  [the keys are collected per source from by_pass[source] just before its loop; the loop removes only the current key and file inserts into by_pass[pass] with source != pass]  let Some(mut finding) = self
(U) BRANCH crates/api/src/scrub_route.rs:835:13 False  [next is ranges.iter_mut().map(pull).collect(), so next.len() == ranges.len(), and at enumerates next; nothing in the loop pushes or pops either]  let (Some(slot), Some(range)) = (next.get_mut(at), ranges.get_mut(at)) else {
(U) BRANCH crates/api/src/scrub_route.rs:838:16 False  [at is chosen among indices whose head is Some, and nothing touches next between that choice and slot.take()]  if let Some(taken) = slot.take() {
(U) BRANCH crates/api/src/scrub_route.rs:1051:9 False  [KEPT is [Mutex<Option<Kept>>; Vendor::ALL.len()] (scrub_route.rs:1010-1011), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so KEPT.get(vendor as usize) is Some]  let Some(cell) = KEPT.get(vendor as usize) else {
(U) LINE crates/api/src/server.rs:1923  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  return (
(U) LINE crates/api/src/server.rs:1924  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  axum::http::StatusCode::SERVICE_UNAVAILABLE,
(U) LINE crates/api/src/server.rs:1925  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  headers,
(U) LINE crates/api/src/server.rs:1926  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  no_such_feed_json(&asked),
(U) LINE crates/api/src/server.rs:1927  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  );
(U) LINE crates/api/src/server.rs:1991  [report_from (server.rs:768-783) ends the body with writeln! into a String, whose fmt::Write cannot fail: the body always ends with a newline]  body.push('\n');
(U) LINE crates/api/src/server.rs:5020  [note_run_started's one caller (open_spot_landing) passes the run of a landing that is always opened (Feed::store_vendor is Some on every arm, crates/pull/src/vendor.rs:3891-3906), and pullrun::open_landing (pullrun.rs:3637-3643) takes a given non-zero run or allocates max(now, last_id + 1, ..) >= 1 (crates/pull/src/gdfl_run_ledger.rs:435-452): run is never 0]  telemetry::now_millis().unsigned_abs()
(U) LINE crates/api/src/server.rs:5825  [Window::verdict returns SessionLengthUnmeasured only for a day with_recorded_window leaves OpenLengthUnmeasured (crates/pull/src/session.rs:1190, 1252-1254); calendar::kind_of = measured_kind (calendar.rs:654-656) answers that only for the five LENGTH_UNMEASURED days (calendar.rs:579-585, 688-693), each a SPECIAL_SESSIONS day (calendar.rs:1365-1470) that with_recorded_window turns into Open (session.rs:1349-1358)]  Err(pull::session::SessionError::SessionLengthUnmeasured { .. }) => false,
(U) LINE crates/api/src/server.rs:5826  [Window::verdict's other errors: IstMoment::from_epoch_secs, already Ok for this epoch three lines above, and VenueHoursUnknown from Venue::NseCash.hours_on, whose table NSE_CASH_SESSIONS (crates/pull/src/vendor.rs) holds only Hours::Verified rows from EPOCH_DAY on]  Err(why) => return Err(why.to_string()),
(U) LINE crates/api/src/server.rs:6174  [Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match answering Some on every arm]  None => None,
(U) LINE crates/api/src/server.rs:6614  [read only when run.blocked is None; every return of broker_run_by that leaves blocked None is BrokerRun::opened (server.rs 4670-4686), whose ledger is RunLedger::open, which sets source to Some (4230-4236); no code assigns a BrokerRun's ledger after]  || spot_audit_origin(asked.cash_identity, &run.origin),
(U) LINE crates/api/src/server.rs:9464  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  return Err(format!(
(U) LINE crates/api/src/server.rs:9465  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  "{} has no store prefix of its own, so there is nowhere to file its \
(U) LINE crates/api/src/server.rs:9466  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  bars. Nothing is written rather than another vendor's prefix being \
(U) LINE crates/api/src/server.rs:9467  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  borrowed.",
(U) LINE crates/api/src/server.rs:9468  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  vendor.as_str()
(U) LINE crates/api/src/server.rs:9469  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  ));
(U) LINE crates/api/src/server.rs:9884  [Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match answering Some on every arm, so this ok_or_else closure is called by no input]  format!(
(U) LINE crates/api/src/server.rs:9888  [Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match answering Some on every arm, so this ok_or_else closure is called by no input]  feed.display()
(U) LINE crates/api/src/server.rs:9890  [Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match answering Some on every arm, so this ok_or_else closure is called by no input]  })?;
(U) LINE crates/api/src/server.rs:9961  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  return Err(format!(
(U) LINE crates/api/src/server.rs:9962  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  "{} declares no FNO column layout, and this path files under FNO. \
(U) LINE crates/api/src/server.rs:9963  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  Refused rather than decoded against another vendor's shape — a \
(U) LINE crates/api/src/server.rs:9964  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  file read with the wrong columns is wrong in every field, and the \
(U) LINE crates/api/src/server.rs:9965  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  store is append-only.",
(U) LINE crates/api/src/server.rs:9966  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  feed.display()
(U) LINE crates/api/src/server.rs:9967  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  ));
(U) LINE crates/api/src/server.rs:11550  [Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day: years 1970..=9999 and months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591)]  note_price_refusal(&mut out, "the chunk's first day names no month file");
(U) LINE crates/api/src/server.rs:11551  [Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day: years 1970..=9999 and months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591)]  return out;
(U) LINE crates/api/src/server.rs:11929  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  return page.say(
(U) LINE crates/api/src/server.rs:11930  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  facts,
(U) LINE crates/api/src/server.rs:11932  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  audit::Outcome::NotStarted,
(U) LINE crates/api/src/server.rs:11933  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  "this feed declares no expired-derivative history at all, so there \
(U) LINE crates/api/src/server.rs:11934  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  is nothing to ask it for",
(U) LINE crates/api/src/server.rs:12070  [months is crate::ladder::months_of(asked.window) on the line before, whose first iteration pushes window.from().year_month() (ladder.rs:451-468); Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day, since a Day is validated into 1970..=9999 with months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591); so months is never empty and last() is Some]  return months;
(U) LINE crates/api/src/server.rs:12949  [Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day]  Err(why) => return Some(format!("the Greek-file month could not be named: {why}")),
(U) LINE crates/api/src/server.rs:13020  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  out.refused = out.refused.saturating_add(1);
(U) LINE crates/api/src/server.rs:13021  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  note_price_refusal(
(U) LINE crates/api/src/server.rs:13022  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  &mut out,
(U) LINE crates/api/src/server.rs:13023  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  "the bar's stamp is not a moment any calendar places",
(U) LINE crates/api/src/server.rs:13025  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  continue;
(U) LINE crates/api/src/server.rs:15926  [bars_html calls contract_param(query) only after locate_series returned Ok, and locate_series (server.rs 15429-15435) returns Err(Unlocated::Refused) whenever contract_param(query) is Err; contract_param is a pure function of the same query]  Err(why) => return unlocatable(site, &symbol, &Unlocated::Refused(why)),
(U) LINE crates/api/src/server.rs:38355  [index runs over 0..vocab::table::COUNT, and COUNT <= ConditionMask::BITS is a const assertion (crates/vocab/src/table.rs:1448-1452) with BITS = WORDS * 64 = 384 (crates/vocab/src/mask.rs:18, :44), so index is below 65,536 and u16::try_from is Ok]  break;
(U) LINE crates/api/src/server.rs:38362  [the loop is for index in 0..vocab::table::COUNT and COUNT is TABLE.len() (crates/vocab/src/table.rs:1432); definition is TABLE.get(usize::from(index)) (table.rs:1581-1583): Some for every position]  break;
(U) BRANCH crates/api/src/server.rs:1922:9 False  [census_cache::VendorCensusNow::census (census_cache.rs:688-690) is a const fn whose body is Some(&self.census): Some on every call]  let Some(census) = now.census() else {
(U) BRANCH crates/api/src/server.rs:1990:8 True  [report_from (server.rs:768-783) ends the body with writeln! into a String, whose fmt::Write cannot fail: the body always ends with a newline]  if !body.ends_with('\n') {
(U) BRANCH crates/api/src/server.rs:2758:31 False  [targets is [usize; SpotTarget::ALL.len()] and slot comes from SpotTarget::ALL.into_iter().enumerate(), so targets.get_mut(slot) is Some]  if counted && let Some(n) = targets.get_mut(slot) {
(U) BRANCH crates/api/src/server.rs:4521:16 False  [pull::session::Window::new (crates/pull/src/session.rs:1056-1067) refuses only a window whose to is before its from; Window::new(day, day) is Ok]  && let Ok(one) = pull::session::Window::new(day, day)
(U) BRANCH crates/api/src/server.rs:5019:21 True  [note_run_started's one caller (open_spot_landing) passes the run of a landing that is always opened (Feed::store_vendor is Some on every arm, crates/pull/src/vendor.rs:3891-3906), and pullrun::open_landing (pullrun.rs:3637-3643) takes a given non-zero run or allocates max(now, last_id + 1, ..) >= 1 (crates/pull/src/gdfl_run_ledger.rs:435-452): run is never 0]  let id = if run == 0 {
(U) BRANCH crates/api/src/server.rs:6075:12 True  [spot_vendor_identity_for returns at its first statement for policy != Isin && vendor != Zerodha (server.rs 6055-6057), and this check runs only inside if policy != Isin: vendor == Zerodha here]  if vendor != Vendor::Zerodha
(U) BRANCH crates/api/src/server.rs:9463:9 False  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  let Some(store_vendor) = feed.store_vendor() else {
(U) BRANCH crates/api/src/server.rs:9960:9 False  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  let Some(layout) = archive.layout(Segment::Fno) else {
(U) BRANCH crates/api/src/server.rs:11549:9 False  [Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day: years 1970..=9999 and months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591)]  let Ok(month) = month_of.chunk.from().year_month() else {
(U) BRANCH crates/api/src/server.rs:11928:8 True  [the only HttpSpecs are DHAN (FnoAccess::ByStrikeOffset, crates/pull/src/vendor.rs:5048), GROWW (ByName, :5285) and ZERODHA (ByToken, :6107); fno_walk returns at by_offset (Dhan) and by_token (Zerodha) above, and Groww's ByName serves (FnoAccess::serves, vendor.rs:2925-2927)]  if !spec.fno.serves() {
(U) BRANCH crates/api/src/server.rs:12069:13 False  [months is crate::ladder::months_of(asked.window) on the line before, whose first iteration pushes window.from().year_month() (ladder.rs:451-468); Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day, since a Day is validated into 1970..=9999 with months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591); so months is never empty and last() is Some]  let Some(&last) = months.last() else {
(U) BRANCH crates/api/src/server.rs:13019:13 False  [the loop continues past its head only when window.verdict(ts.div_euclid(1_000_000), ..) is Ok(None), and Window::verdict begins with IstMoment::from_epoch_secs(epoch_secs)? on that same value (crates/pull/src/session.rs:1160-1166): Ok here on every row]  let Ok(at) = pull::session::IstMoment::from_epoch_secs(ts.div_euclid(1_000_000)) else {
(U) BRANCH crates/api/src/server.rs:38354:13 False  [index runs over 0..vocab::table::COUNT, and COUNT <= ConditionMask::BITS is a const assertion (crates/vocab/src/table.rs:1448-1452) with BITS = WORDS * 64 = 384 (crates/vocab/src/mask.rs:18, :44), so index is below 65,536 and u16::try_from is Ok]  let Ok(position) = u16::try_from(index) else {
(U) BRANCH crates/api/src/server.rs:38361:13 False  [the loop is for index in 0..vocab::table::COUNT and COUNT is TABLE.len() (crates/vocab/src/table.rs:1432); definition is TABLE.get(usize::from(index)) (table.rs:1581-1583): Some for every position]  let Some(def) = vocab::table::definition(position) else {
(U) LINE crates/api/src/sources.rs:1500  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  return Sessions {
(U) LINE crates/api/src/sources.rs:1501  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  json: json!({ "calendar": link, "why": "the calendar link would not parse" }),
(U) LINE crates/api/src/sources.rs:1502  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  not_held: None,
(U) LINE crates/api/src/sources.rs:1503  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  };
(U) LINE crates/api/src/sources.rs:1633  [pair comes from months.windows(2), and slice::windows(n) yields only slices of exactly n elements, so first() and get(1) are both Some]  continue;
(U) LINE crates/api/src/sources.rs:2942  [called only with DAY_1, MINUTE_1 and SECOND_1, and feed_holding reads only compile-time descriptors: GDFL serves Second1 and Zerodha Minute1 and Day1 (crates/pull/src/vendor.rs:5815, 6317-6319; pinned by the_tags_are_the_four_d3339_names)]  return Ok(None);
(U) LINE crates/api/src/sources.rs:3543  [day_table gets flags only from dayflags::answer, whose one exit builds ("days", Value::Array(records)) (dayflags.rs:566-568, 650-688): flags.get("days").and_then(as_array) is Some]  }
(U) BRANCH crates/api/src/sources.rs:426:24 False  [the nibble is byte >> 4 or byte & 0x0F of a u8, so at most 15, and char::from_digit(n, 16) is Some for every n < 16]  if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
(U) BRANCH crates/api/src/sources.rs:983:51 False  [exchange_kind_of answers Open(session) only with a const session built by one_window (count 1) or two_windows (count 2) or Session::full() (count 1) (crates/pull/src/calendar.rs:446-566, 1216-1224, 1521-1529): count > 0 on every day]  pull::calendar::DayKind::Open(session) if session.count > 0 => session
(U) BRANCH crates/api/src/sources.rs:1499:9 False  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  let Ok(uri) = link.parse::<axum::http::Uri>() else {
(U) BRANCH crates/api/src/sources.rs:1632:13 False  [pair comes from months.windows(2), and slice::windows(n) yields only slices of exactly n elements, so first() and get(1) are both Some]  let (Some(&before), Some(&after)) = (pair.first(), pair.get(1)) else {
(U) BRANCH crates/api/src/sources.rs:2129:20 False  [at is vec![0; reads.len()] and n enumerates reads, so at.get_mut(n) is Some; union_of takes only reads and desc]  if let Some(i) = at.get_mut(n)
(U) BRANCH crates/api/src/sources.rs:2458:16 False  [at enumerates the four-element array literal of o, h, l and c (0..=3) and equal is [false; 4], so get_mut(at) is Some]  if let Some(slot) = equal.get_mut(at) {
(U) BRANCH crates/api/src/sources.rs:2941:17 False  [called only with DAY_1, MINUTE_1 and SECOND_1, and feed_holding reads only compile-time descriptors: GDFL serves Second1 and Zerodha Minute1 and Day1 (crates/pull/src/vendor.rs:5815, 6317-6319; pinned by the_tags_are_the_four_d3339_names)]  let Some((_, vendor)) = feed_holding(rung) else {
(U) BRANCH crates/api/src/sources.rs:3537:12 False  [day_table gets flags only from dayflags::answer, whose one exit builds ("days", Value::Array(records)) (dayflags.rs:566-568, 650-688): flags.get("days").and_then(as_array) is Some]  if let Some(records) = flags.get("days").and_then(Value::as_array) {
(U) BRANCH crates/api/src/sources.rs:3539:20 False  [every record is Context::record's single-exit object carrying ("day", day.to_string()) (dayflags.rs:5138-5253); later writers only insert notes or reorder]  if let Some(day) = record.get("day").and_then(Value::as_str) {
(U) BRANCH crates/api/src/sources.rs:3578:16 False  [row is day_row's answer, and day_row has no return: its tail expression is the object literal json!({"day", "absent", ..., "records_read"}) (sources.rs, fn day_row), so as_object_mut() is Some]  if let Some(object) = row.as_object_mut() {
(U) LINE crates/api/src/speed.rs:349  [std::array::from_fn for [u64; OUTCOMES] passes i in 0..OUTCOMES, and both Frozen snapshots hold outcomes: [u32; OUTCOMES] (speed.rs:225), so both get(i) are Some]  _ => 0,
(U) LINE crates/api/src/speed.rs:895  [tokio::signal::unix::Signal::recv is self.inner.recv().await; Some(()) (tokio 1.53.3 src/signal/unix.rs:452-455, pinned by Cargo.lock): never None]  std::future::pending::<()>().await;
(U) BRANCH crates/api/src/speed.rs:270:16 False  [Live::new, the one constructor of Live (speed.rs:206-212), makes BUCKETS buckets, and bucket_of answers below BUCKETS for every u64: micros < SUB_BUCKETS (8) answers micros; an octave past TOP_OCTAVE (35) answers BUCKETS - 1; otherwise SUB_BITS <= octave <= TOP_OCTAVE and the answer is at most SUB_BUCKETS + (TOP_OCTAVE - SUB_BITS) * SUB_BUCKETS + SUB_BUCKETS - 1 = BUCKETS - 1 (speed.rs:64-75, :95-106)]  if let Some(bucket) = self.live.buckets.get(bucket_of(micros)) {
(U) BRANCH crates/api/src/speed.rs:273:16 False  [Outcome::slot answers 0 to 4 (speed.rs:170-178) and outcomes is [AtomicU64; OUTCOMES] with OUTCOMES = 5 (speed.rs:153, :194)]  if let Some(count) = self.live.outcomes.get(outcome.slot()) {
(U) BRANCH crates/api/src/speed.rs:894:24 True  [tokio::signal::unix::Signal::recv is self.inner.recv().await; Some(()) (tokio 1.53.3 src/signal/unix.rs:452-455, pinned by Cargo.lock): never None]  if term.recv().await.is_none() {
(U) BRANCH crates/api/src/speed.rs:1087:16 False  [Pending is built only by Pending::start, with row: Some (speed.rs:1072-1077), and finish takes self by value, so its take() is the first of that Pending's row; only Drop, after finish, meets None]  if let Some(row) = self.row.take() {
(U) LINE crates/api/src/topjson.rs:194  [every Ok of of_run_against_receipt carries partial == None: read_against_receipt returns Err whenever damaged is Some and its Ok returns are (Vec::new(), None) and (found, damaged) after that check (crates/cli/src/frontier.rs:1517-1668)]  return Err(why);
(U) BRANCH crates/api/src/topjson.rs:193:12 True  [every Ok of of_run_against_receipt carries partial == None: read_against_receipt returns Err whenever damaged is Some and its Ok returns are (Vec::new(), None) and (found, damaged) after that check (crates/cli/src/frontier.rs:1517-1668)]  if let Some(why) = partial {
(U) LINE crates/api/src/trades.rs:181  [the window is detail::window(rows.len(), page), whose seek_window answers Ok only with start <= end <= total: start is the offset, below total when total > 0, and 0 for page 0 of an empty result; end is min(start + limit, total) (detail.rs:1340-1412), so rows.get(start..end) is Some]  return refuse(
(U) LINE crates/api/src/trades.rs:182  [the window is detail::window(rows.len(), page), whose seek_window answers Ok only with start <= end <= total: start is the offset, below total when total > 0, and 0 for page 0 of an empty result; end is min(start + limit, total) (detail.rs:1340-1412), so rows.get(start..end) is Some]  json,
(U) LINE crates/api/src/trades.rs:183  [the window is detail::window(rows.len(), page), whose seek_window answers Ok only with start <= end <= total: start is the offset, below total when total > 0, and 0 for page 0 of an empty result; end is min(start + limit, total) (detail.rs:1340-1412), so rows.get(start..end) is Some]  "the validated trade page window did not fit its verified row vector; no fallback page is exposed",
(U) LINE crates/api/src/trades.rs:253  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  return too_large(
(U) LINE crates/api/src/trades.rs:254  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  json,
(U) LINE crates/api/src/trades.rs:255  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  &identity,
(U) LINE crates/api/src/trades.rs:256  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  u64::try_from(rows.len()).unwrap_or(u64::MAX),
(U) LINE crates/api/src/trades.rs:257  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  &format!(
(U) LINE crates/api/src/trades.rs:258  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  "the bounded trade response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
(U) LINE crates/api/src/trades.rs:259  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  out.len(),
(U) LINE crates/api/src/trades.rs:260  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  crate::detail::MAX_RESPONSE_BYTES
(U) LINE crates/api/src/trades.rs:261  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  ),
(U) BRANCH crates/api/src/trades.rs:180:9 False  [the window is detail::window(rows.len(), page), whose seek_window answers Ok only with start <= end <= total: start is the offset, below total when total > 0, and 0 for page 0 of an empty result; end is min(start + limit, total) (detail.rs:1340-1412), so rows.get(start..end) is Some]  let Some(page_rows) = rows.get(window.start..window.end) else {
(U) BRANCH crates/api/src/trades.rs:252:8 True  [the largest body respond can write is under 6.6 MB, below MAX_RESPONSE_BYTES (8 MiB, detail.rs:32): a result holds at most 4,096 rows (detail::window refuses more, detail.rs:1340-1348) and a page at most 256 (MAX_PAGE_ROWS) rows of 14 integers, each at most 20 characters, about 550 bytes a row; the periods (cli::trades::Period::ALL, eight) are day, week, month, quarter, half and year with at most one bucket per row each, weekday at most 7 and hour at most 24 (cli/src/trades.rs:2429-2440), each bucket 8 integers in at most 261 bytes; so 6 x 4,096 x 261 + 31 x 261 + 256 x 550 bytes plus a head and tail of a 64-hex identity, two policy words, the equity note and one partial sentence stays under 8 MiB]  if out.len() > crate::detail::MAX_RESPONSE_BYTES {
`````

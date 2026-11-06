# fu-AS2.txt.md

Source: this shard's Route B whole:api readout of tip `b3d5321c` (`d3-route-b-cloud.sh`, the kit's set file, this Linux host), classified as its first lines say. One fenced block below holds it.

`````text
(F)/(U) list of BASE-COV-API-AS2 (repair round 3, shard G2), tip b3d5321cf1b4fe9365f09b7976f35312b1080661, whole:api, Route B (see gaps-AS2.txt for the run).
(F) = the failing side of an assert!/assert_eq!/assert_ne!/panic!/unreachable! whose statement ran, in test code (recipe v4 step 4), as repair r2 read it.
(U) = unreachable on every target, aarch64-apple-darwin included: the proof from the code is in brackets on each row.
(F) 144 lines 474 sides; (U) 56 lines 35 sides.

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
(U) BRANCH crates/api/src/booleanlaunch.rs:450:12 False  [booleanevidencejson::search_policy returns admission_projection::policy, whose tail expression is the object literal json!({"digest":..,"values":rows}) (booleanadmission_projection.rs:76-133), so as_object_mut() is Some]  if let Some(fields) = policy.as_object_mut() {
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:472:13 False  [the false side fails the check, and its line ran]  start.elapsed() <= std::time::Duration::from_secs(45)
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:473:20 False  [the false side fails the check, and its line ran]  && std::fs::metadata(&log).unwrap().len() <= 1024 * 1024,
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:801:9 False  [the false side fails the check, and its line ran]  queried.contains("metadata accepts only one exact max_points query")
(F) BRANCH crates/api/src/booleanlaunch_tests.rs:802:16 False  [the false side fails the check, and its line ran]  && !queried.contains(NO_STORE),
(F) LINE crates/api/src/calendar_of.rs:1344  [panic/unreachable line]  panic!("day {day} is {kind:?}, not an open session");
(F) LINE crates/api/src/calendar_of.rs:1789  [failing arm of the statement at 1784, which ran]  24 * 48
(F) LINE crates/api/src/calendar_of.rs:3033  [failing arm of the statement at 3029, which ran]  json(&exchange)
(U) BRANCH crates/api/src/calendar_of.rs:684:24 False  [month is a pull::session::Day, validated into 1970..=9999 with months 1..=12 at construction (crates/pull/src/session.rs), exactly the range YearMonth::new accepts (crates/store/src/path.rs:583-591), so YearMonth::new(month.year(), month.month()) is Ok]  && let Ok(named) = YearMonth::new(month.year(), month.month())
(F) BRANCH crates/api/src/calendar_of.rs:1343:13 False  [the else of a let-else that panics, whose line ran]  let DayKind::Open(session) = kind else {
(F) BRANCH crates/api/src/calendar_of.rs:2757:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:2757:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3169:17 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(F) BRANCH crates/api/src/calendar_of.rs:3169:42 False  [the false side fails the check, and its line ran]  assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
(U) LINE crates/api/src/calendar_record.rs:341  [every store::path::YearMonth is year 1970..=9999 and month 1..=12 (YearMonth::new, crates/store/src/path.rs:583-591, the only way to make one), and pull::session::Day::new accepts the first day of every such month (a Day is validated into 1970..=9999)]  continue;
(U) LINE crates/api/src/calendar_record.rs:356  [the line before resizes counts to at + 1 whenever its length is at or below at, so get_mut(at) is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/calendar_record.rs:502  [auction_start takes no argument: high starts at pull::calendar::SOURCED_LAST_DAY, 2026-12-31 (crates/pull/src/calendar.rs:982-990), and auction_day is cash_auction_eligibility_required, day >= 2026-08-03 (crates/pull/src/vendor.rs:1443-1447), so the record's last day is always an auction day]  return None;
(U) LINE crates/api/src/calendar_record.rs:615  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  None => serde_json::json!({
(U) LINE crates/api/src/calendar_record.rs:616  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "from": iso(from),
(U) LINE crates/api/src/calendar_record.rs:617  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "to": iso(to),
(U) LINE crates/api/src/calendar_record.rs:618  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "regular": serde_json::Value::Null,
(U) LINE crates/api/src/calendar_record.rs:619  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990): hours(day) is never None]  "why": "the derivatives session table carries no verified hours for these days",
(U) BRANCH crates/api/src/calendar_record.rs:340:17 False  [every store::path::YearMonth is year 1970..=9999 and month 1..=12 (YearMonth::new, crates/store/src/path.rs:583-591, the only way to make one), and pull::session::Day::new accepts the first day of every such month (a Day is validated into 1970..=9999)]  let Ok(one) = pull::session::Day::new(month.year(), month.month(), 1) else {
(U) BRANCH crates/api/src/calendar_record.rs:354:24 False  [the line before resizes counts to at + 1 whenever its length is at or below at, so get_mut(at) is Some]  if let Some(slot) = counts.get_mut(at) {
(U) BRANCH crates/api/src/calendar_record.rs:501:8 True  [auction_start takes no argument: high starts at pull::calendar::SOURCED_LAST_DAY, 2026-12-31 (crates/pull/src/calendar.rs:982-990), and auction_day is cash_auction_eligibility_required, day >= 2026-08-03 (crates/pull/src/vendor.rs:1443-1447), so the record's last day is always an auction day]  if !auction_day(high) {
(U) BRANCH crates/api/src/calendar_record.rs:580:27 False  [derivatives_owed asks Venue::NseDerivatives alone, whose table NSE_DERIVATIVES_SESSIONS (crates/pull/src/vendor.rs:1575-1593) is an anchor from 1970-01-01 and one later row from 2026-08-03, both SessionRow::verified (Hours::Verified, vendor.rs:1052-1068), so SessionTable::hours_on (vendor.rs:1230-1259) answers Ok on every day, and civil is Some on every day the bisection reads, SOURCED_FIRST_DAY..=SOURCED_LAST_DAY (2018-01-01 to 2026-12-31, crates/pull/src/calendar.rs:978-990), and those two rows' (open, close, source) differ, so the walk meets exactly two eras and rows.len() is at most 2 when it is asked]  while from <= last && rows.len() <= 8 {
(F) LINE crates/api/src/calendar_record_tests.rs:78  [failing arm of the statement at 75, which ran]  iso(day)
(F) LINE crates/api/src/calendar_record_tests.rs:551  [failing arm of the statement at 544, which ran]  widened["special"]
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:13 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:172:40 False  [the false side fails the check, and its line ran]  assert!(!minutes.contains(&562) && !minutes.contains(&620));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:13 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:173:39 False  [the false side fails the check, and its line ran]  assert!(minutes.contains(&561) && minutes.contains(&621));
(F) BRANCH crates/api/src/calendar_record_tests.rs:412:32 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("1day") && why.contains("NIFTY")),
(U) BRANCH crates/api/src/calendar_route.rs:548:12 False  [body is the object literal json!({"status", "feed", "rung", "symbol", "retry_after_ms"}) two lines above, and building_answer's two callers (building and building_census, calendar_route.rs) each pass &serde_json::json!({...}), an object literal, as fields, so both are Some]  if let (Some(body), Some(fields)) = (body.as_object_mut(), fields.as_object()) {
(F) LINE crates/api/src/calendar_route_tests.rs:663  [panic/unreachable line]  panic!("{query}: still building after 1,000 asks");
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
(F) BRANCH crates/api/src/calendar_route_tests.rs:887:13 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:887:46 False  [the false side fails the check, and its line ran]  assert!(feeds.contains(&Vendor::Gdfl) && feeds.contains(&Vendor::Zerodha));
(F) BRANCH crates/api/src/calendar_route_tests.rs:934:9 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
(F) BRANCH crates/api/src/calendar_route_tests.rs:934:77 False  [the false side fails the check, and its line ran]  why.contains(&crate::calendar_of::build::MAX_BUILDS.to_string()) && why.contains("retry"),
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
(F) LINE crates/api/src/census_tail_tests.rs:109  [panic/unreachable line]  panic!("not held: {}", census.note());
(F) LINE crates/api/src/census_tail_tests.rs:497  [panic/unreachable line]  panic!("served past its capacity: {}", over.note());
(F) BRANCH crates/api/src/census_tail_tests.rs:108:9 False  [the else of a let-else that panics, whose line ran]  let Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/census_tail_tests.rs:496:9 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { ref reason, fault } = over.state else {
(F) BRANCH crates/api/src/census_tail_tests.rs:572:13 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/census_tail_tests.rs:572:39 False  [the false side fails the check, and its line ran]  assert!(why.contains("capped") && why.contains("FNO"), "{why}");
(U) BRANCH crates/api/src/constituents.rs:534:20 False  [per_vendor is [Vec<VendorId>; Vendor::ALL.len()] (constituents.rs:498), five slots, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so get_mut is Some]  && let Some(slot) = self.per_vendor.get_mut(vendor as usize)
(F) BRANCH crates/api/src/constituents.rs:1734:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1734:45 False  [the false side fails the check, and its line ran]  assert!(head.contains("matched") && head.contains("vendor lacks"));
(F) BRANCH crates/api/src/constituents.rs:1735:17 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1735:47 False  [the false side fails the check, and its line ran]  assert!(head.contains("ambiguous") && head.contains("malformed"));
(F) BRANCH crates/api/src/constituents.rs:1787:13 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(F) BRANCH crates/api/src/constituents.rs:1787:49 False  [the false side fails the check, and its line ran]  quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
(U) BRANCH crates/api/src/dayflags_rules.rs:1190:20 False  [a judged run starts only at a flat bar on a judged minute (not Aside), and that bar leaves the run-through Some, continued or restarted at its value; a judged run continues only on a flat bar of the same value one minute on, which continues the run-through too; so after every bar run.is_some() implies through.is_some(), and close_run is called only with the previous bar's pair and after the loop]  if let Some(through) = through {
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
(F) LINE crates/api/src/dayflags_rules_tests.rs:1610  [failing arm of the statement at 1525, which ran]  says(record(&got, "2024-10-03"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:1656  [failing arm of the statement at 1574, which ran]  says(unheld)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2431  [failing arm of the statement at 2348, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:2827  [failing arm of the statement at 2669, which ran]  TIMED[1]
(F) LINE crates/api/src/dayflags_rules_tests.rs:3498  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3526  [panic/unreachable line]  panic!("the month opens again");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3711  [failing arm of the statement at 3382, which ran]  says(rec)
(F) LINE crates/api/src/dayflags_rules_tests.rs:3803  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:3973  [failing arm of the statement at 3644, which ran]  says(record(&got, "2024-11-09"))
(F) LINE crates/api/src/dayflags_rules_tests.rs:4052  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:4096  [panic/unreachable line]  panic!("the month opens");
(F) LINE crates/api/src/dayflags_rules_tests.rs:4201  [failing arm of the statement at 3874, which ran]  says(closed)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4288  [failing arm of the statement at 3959, which ran]  says(after)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4392  [failing arm of the statement at 4065, which ran]  says(quiet)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4622  [failing arm of the statement at 4293, which ran]  super::super::tests::says(unstamped)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4631  [failing arm of the statement at 4302, which ran]  super::super::tests::says(short)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4909  [failing arm of the statement at 4582, which ran]  says(moving)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4924  [failing arm of the statement at 4593, which ran]  says(both)
(F) LINE crates/api/src/dayflags_rules_tests.rs:4973  [failing arm of the statement at 4646, which ran]  says(aside)
(F) LINE crates/api/src/dayflags_rules_tests.rs:5309  [failing arm of the statement at 4982, which ran]  says(empty)
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
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3520:13 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3520:33 False  [the false side fails the check, and its line ran]  assert!(whole.is_empty() && whole_faults.is_empty(), "{whole:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3571:13 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:3571:33 False  [the false side fails the check, and its line ran]  assert!(quiet.is_empty() && quiet_faults.is_empty(), "{quiet:#?}");
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5413:13 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
(F) BRANCH crates/api/src/dayflags_rules_tests.rs:5413:48 False  [the false side fails the check, and its line ran]  assert!(kept_flag(&a, on, "RELIANCE").0 && kept_flag(&b, on, "RELIANCE").0);
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
(U) BRANCH crates/api/src/fault_log.rs:186:34 False  [unnamed grows only behind runs_named >= MAX_NAMED_FAULTS (16), runs_named grows only beside a push into named, absorb moves the other log's named sentences (16 or more whenever its unnamed > 0) in first, and nothing in fault_log.rs removes from named; so unnamed > 0 implies named is non-empty, and this operand is asked only when named is empty]  self.named.is_empty() && self.unnamed == 0 && self.open.is_none()
(F) LINE crates/api/src/fno_boundary_tests.rs:45  [panic/unreachable line]  panic!("named-chain fixture requires an HTTP descriptor");
(F) LINE crates/api/src/fno_boundary_tests.rs:227  [panic/unreachable line]  panic!("the rolling fixture requires an HTTP descriptor");
(F) BRANCH crates/api/src/fno_boundary_tests.rs:44:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/fno_boundary_tests.rs:226:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = self.asked.feed.descriptor().transport else {
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
(U) BRANCH crates/api/src/instruments_route.rs:204:20 False  [the tag is format!("\"{}\"", hex32(blake3(body))) (store_route.rs:734-740): a double quote, 64 hex digits and a double quote, all visible ASCII, which HeaderValue::from_str accepts]  if let Ok(value) = axum::http::HeaderValue::from_str(&tagged.etag) {
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
(U) BRANCH crates/api/src/operation_audit.rs:226:20 False  [id is cli::operation_audit::Attempt::id, a u64 (crates/cli/src/operation_audit.rs:425), whose to_string is ASCII digits only, every one a visible byte HeaderValue::from_str accepts]  if let Ok(value) = axum::http::HeaderValue::from_str(&id.to_string()) {
(F) LINE crates/api/src/operation_audit_tests.rs:200  [failing arm of the statement at 197, which ran]  routes.len()
(F) BRANCH crates/api/src/operation_audit_tests.rs:318:75 False  [the false side fails the check, and its line ran]  matches!(parsed, Asked::Page { before: Some(asked), limit: 2 } if asked == before),
(F) BRANCH crates/api/src/operation_audit_tests.rs:337:29 False  [the false side fails the check, and its line ran]  Asked::Exact(id) if id == ID_BASE + 1
(U) LINE crates/api/src/provenance.rs:412  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/provenance.rs:702  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some (the implicit else sits on the if's closing brace)]  }
(U) LINE crates/api/src/provenance.rs:1605  [fold_record's one caller passes the items of buffer.chunks_exact(audit::RECORD_LEN) (provenance.rs, the record walk of runs_read), and chunks_exact yields only slices of exactly that length, so the conversion to &[u8; RECORD_LEN] is Ok]  return;
(U) BRANCH crates/api/src/provenance.rs:407:16 False  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some]  if let Some(object) = body.as_object_mut() {
(U) BRANCH crates/api/src/provenance.rs:670:12 False  [body is asked.echo() on the line before, and Asked::echo's tail expression is the object literal json!({"feed", "instrument", "timeframe", "month", "t"}) (provenance.rs:226-234), so as_object_mut() is Some]  if let Some(object) = body.as_object_mut() {
(U) BRANCH crates/api/src/provenance.rs:1604:9 False  [fold_record's one caller passes the items of buffer.chunks_exact(audit::RECORD_LEN) (provenance.rs, the record walk of runs_read), and chunks_exact yields only slices of exactly that length, so the conversion to &[u8; RECORD_LEN] is Ok]  let Ok(image) = <&[u8; audit::RECORD_LEN]>::try_from(image) else {
(F) LINE crates/api/src/provenance_tests.rs:1230  [panic/unreachable line]  panic!("a sidecar");
(F) LINE crates/api/src/provenance_tests.rs:1331  [failing arm of the statement at 1328, which ran]  wire.len()
(F) BRANCH crates/api/src/provenance_tests.rs:1229:9 False  [the else of a let-else that panics, whose line ran]  let Found::Present(sidecar) = store::provenance::read(&root, path).unwrap() else {
(F) LINE crates/api/src/quarantine_route_tests.rs:137  [panic/unreachable line]  panic!("the pool stayed full for two seconds");
(F) LINE crates/api/src/quarantine_route_tests.rs:545  [failing arm of the statement at 542, which ran]  matches!(
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
(U) LINE crates/api/src/scrub_route.rs:1052  [KEPT is [Mutex<Option<Kept>>; Vendor::ALL.len()] (scrub_route.rs:1010-1011), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so KEPT.get(vendor as usize) is Some]  return Ok(read(&Index::default()));
(U) BRANCH crates/api/src/scrub_route.rs:1051:9 False  [KEPT is [Mutex<Option<Kept>>; Vendor::ALL.len()] (scrub_route.rs:1010-1011), five cells, and Vendor is a fieldless enum of five variants with implicit discriminants 0 to 4 (crates/core/src/vendor.rs:46-53), so vendor as usize is below 5, so KEPT.get(vendor as usize) is Some]  let Some(cell) = KEPT.get(vendor as usize) else {
(U) LINE crates/api/src/server.rs:9464  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  return Err(format!(
(U) LINE crates/api/src/server.rs:9465  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  "{} has no store prefix of its own, so there is nowhere to file its \
(U) LINE crates/api/src/server.rs:9466  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  bars. Nothing is written rather than another vendor's prefix being \
(U) LINE crates/api/src/server.rs:9467  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  borrowed.",
(U) LINE crates/api/src/server.rs:9468  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  vendor.as_str()
(U) LINE crates/api/src/server.rs:9469  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  ));
(U) LINE crates/api/src/server.rs:9961  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  return Err(format!(
(U) LINE crates/api/src/server.rs:9962  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  "{} declares no FNO column layout, and this path files under FNO. \
(U) LINE crates/api/src/server.rs:9963  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  Refused rather than decoded against another vendor's shape — a \
(U) LINE crates/api/src/server.rs:9964  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  file read with the wrong columns is wrong in every field, and the \
(U) LINE crates/api/src/server.rs:9965  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  store is append-only.",
(U) LINE crates/api/src/server.rs:9966  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  feed.display()
(U) LINE crates/api/src/server.rs:9967  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  ));
(U) LINE crates/api/src/server.rs:12070  [months is crate::ladder::months_of(asked.window) on the line before, whose first iteration pushes window.from().year_month() (ladder.rs:451-468); Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day, since a Day is validated into 1970..=9999 with months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591); so months is never empty and last() is Some]  return months;
(F) LINE crates/api/src/server.rs:18907  [failing arm of the statement at 18904, which ran]  began.elapsed()
(F) LINE crates/api/src/server.rs:18969  [failing arm of the statement at 18966, which ran]  String::from_utf8_lossy(&got)
(F) LINE crates/api/src/server.rs:21392  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:21680  [failing arm of the statement at 21674, which ran]  withheld[0].1
(F) LINE crates/api/src/server.rs:22798  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24119  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24244  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24294  [failing arm of the statement at 24202, which ran]  dir.display()
(F) LINE crates/api/src/server.rs:24300  [panic/unreachable line]  panic!("{receipts:?}");
(F) LINE crates/api/src/server.rs:24303  [panic/unreachable line]  panic!("{segment:?}");
(F) LINE crates/api/src/server.rs:24424  [panic/unreachable line]  panic!("the walk's feed is an HTTP feed");
(F) LINE crates/api/src/server.rs:24458  [panic/unreachable line]  panic!("a walk opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24463  [panic/unreachable line]  panic!("a spot request opened while another process held the vendor");
(F) LINE crates/api/src/server.rs:24488  [panic/unreachable line]  panic!("a walk took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24497  [panic/unreachable line]  panic!("a spot request took an id from a damaged ledger");
(F) LINE crates/api/src/server.rs:24532  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24627  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24670  [failing arm of the statement at 24578, which ran]  away.map(|calendar| calendar.sessions())
(F) LINE crates/api/src/server.rs:24717  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/server.rs:24819  [panic/unreachable line]  panic!("HTTP descriptor")
(F) LINE crates/api/src/server.rs:25605  [failing arm of the statement at 25513, which ran]  root.display()
(F) LINE crates/api/src/server.rs:28878  [panic/unreachable line]  panic!("{feed:?} is HTTP");
(F) LINE crates/api/src/server.rs:29651  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:29665  [failing arm of the statement at 29570, which ran]  stock.len(),
(F) LINE crates/api/src/server.rs:29666  [failing arm of the statement at 29570, which ran]  index.len()
(F) LINE crates/api/src/server.rs:30751  [panic/unreachable line]  panic!("{} is an HTTP feed and must have a budget", feed.display())
(F) LINE crates/api/src/server.rs:30785  [failing arm of the statement at 30691, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:30830  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31180  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) LINE crates/api/src/server.rs:31244  [panic/unreachable line]  panic!("Zerodha is an HTTP feed");
(F) LINE crates/api/src/server.rs:32075  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:32172  [failing arm of the statement at 32080, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32218  [failing arm of the statement at 32125, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32226  [failing arm of the statement at 32133, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32231  [failing arm of the statement at 32139, which ran]  feed.display(),
(F) LINE crates/api/src/server.rs:32232  [failing arm of the statement at 32139, which ran]  months.len()
(F) LINE crates/api/src/server.rs:32298  [panic/unreachable line]  panic!("{} declares no HTTP transport", feed.display());
(F) LINE crates/api/src/server.rs:32304  [failing arm of the statement at 32211, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32311  [failing arm of the statement at 32217, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32325  [failing arm of the statement at 32232, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32331  [failing arm of the statement at 32238, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32344  [failing arm of the statement at 32252, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32351  [failing arm of the statement at 32258, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:32542  [failing arm of the statement at 32446, which ran]  &body[..body.len().min(120)]
(F) LINE crates/api/src/server.rs:32742  [panic/unreachable line]  panic!("attempt {attempt} of a 429 gave {stepped:?}");
(F) LINE crates/api/src/server.rs:33071  [failing arm of the statement at 32980, which ran]  marked[0]
(F) LINE crates/api/src/server.rs:33261  [panic/unreachable line]  panic!(
(F) LINE crates/api/src/server.rs:34511  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:34542  [failing arm of the statement at 34450, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:34625  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:34656  [failing arm of the statement at 34564, which ran]  got.bodies.iter().map(|(w, _)| *w).collect::<Vec<_>>()
(F) LINE crates/api/src/server.rs:34865  [panic/unreachable line]  panic!("this feed is HTTP");
(F) LINE crates/api/src/server.rs:35023  [panic/unreachable line]  panic!("this feed authenticates, so its transport is HTTP")
(F) LINE crates/api/src/server.rs:35139  [failing arm of the statement at 35047, which ran]  chunks.len()
(F) LINE crates/api/src/server.rs:35337  [failing arm of the statement at 35245, which ran]  refused.len()
(F) LINE crates/api/src/server.rs:35569  [failing arm of the statement at 35476, which ran]  ledger.fact(1).1
(F) LINE crates/api/src/server.rs:36246  [panic/unreachable line]  panic!("{feed} is an HTTP broker");
(F) LINE crates/api/src/server.rs:36564  [failing arm of the statement at 36473, which ran]  feed.display()
(F) LINE crates/api/src/server.rs:36616  [failing arm of the statement at 36525, which ran]  tf.as_str()
(F) LINE crates/api/src/server.rs:36664  [failing arm of the statement at 36573, which ran]  rung.as_str()
(F) LINE crates/api/src/server.rs:37216  [failing arm of the statement at 37126, which ran]  bucket.word()
(U) LINE crates/api/src/server.rs:37586  [index runs over 0..vocab::table::COUNT, and COUNT <= ConditionMask::BITS is a const assertion (crates/vocab/src/table.rs:1448-1452) with BITS = WORDS * 64 = 384 (crates/vocab/src/mask.rs:18, :44), so index is below 65,536 and u16::try_from is Ok]  break;
(U) BRANCH crates/api/src/server.rs:9463:9 False  [pull::vendor::Feed::store_vendor (crates/pull/src/vendor.rs:3891-3906) is an exhaustive match that answers Some on every arm (Dhan, Groww, TrueData, Gdfl, Zerodha), so no Feed takes the else]  let Some(store_vendor) = feed.store_vendor() else {
(U) BRANCH crates/api/src/server.rs:9960:9 False  [the only LocalArchive transports are TrueData's and GDFL's descriptors (crates/pull/src/vendor.rs:5670, :5784), whose layouts [TRUEDATA_INDEX, TRUEDATA_FNO] (:5707) and [GDFL_FNO] (:5801) each hold a Segment::Fno row (:5647-5648, :5756-5757), so ArchiveSpec::layout(Segment::Fno) (:3570) is Some for every archive]  let Some(layout) = archive.layout(Segment::Fno) else {
(U) BRANCH crates/api/src/server.rs:12069:13 False  [months is crate::ladder::months_of(asked.window) on the line before, whose first iteration pushes window.from().year_month() (ladder.rs:451-468); Day::year_month is YearMonth::new(year, month) (crates/pull/src/session.rs:433-435), Ok for every Day, since a Day is validated into 1970..=9999 with months 1..=12, the range YearMonth::new accepts (crates/store/src/path.rs:583-591); so months is never empty and last() is Some]  let Some(&last) = months.last() else {
(F) BRANCH crates/api/src/server.rs:19172:17 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK"),
(F) BRANCH crates/api/src/server.rs:19172:27 False  [the false side fails the check, and its line ran]  closed && said.starts_with("HTTP/1.1 200 OK"),
(F) BRANCH crates/api/src/server.rs:19316:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19316:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19348:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19348:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19369:17 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19369:27 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19523:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19523:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 408"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19569:21 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:19569:31 False  [the false side fails the check, and its line ran]  assert!(closed && said.starts_with("HTTP/1.1 200 OK"), "{said:?}");
(F) BRANCH crates/api/src/server.rs:21312:17 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21312:41 False  [the false side fails the check, and its line ran]  why.contains("1day") && why.contains("1 of 1"),
(F) BRANCH crates/api/src/server.rs:21391:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:21765:13 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21765:32 False  [the false side fails the check, and its line ran]  run.months == 1 && run.from == month_of(2026, 7) && run.why == ladder::OwedWhy::Interior
(F) BRANCH crates/api/src/server.rs:21769:80 False  [the false side fails the check, and its line ran]  ladder::Gate::RungFirst { missing: 13, runs: 13, ref owed, .. } if owed.len() == ladder::OWED_NAMED
(F) BRANCH crates/api/src/server.rs:21792:13 False  [the false side fails the check, and its line ran]  at_close.contains("ends at 15:30 IST and is asked for from 15:35")
(F) BRANCH crates/api/src/server.rs:21793:20 False  [the false side fails the check, and its line ran]  && at_close.contains("it is 15:30 now"),
(F) BRANCH crates/api/src/server.rs:21813:61 False  [the false side fails the check, and its line ran]  assert!(matches!(muhurat, DayKind::Open(session) if session != Session::full()));
(F) BRANCH crates/api/src/server.rs:21946:13 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21946:54 False  [the false side fails the check, and its line ran]  said.starts_with("STORE UNAVAILABLE") && said.contains("after 9 of 208"),
(F) BRANCH crates/api/src/server.rs:21995:13 False  [the false side fails the check, and its line ran]  line.why.starts_with(pull::calendar::PAST_RECORD_FINDING)
(F) BRANCH crates/api/src/server.rs:21996:20 False  [the false side fails the check, and its line ran]  && line.why.contains(
(F) BRANCH crates/api/src/server.rs:21999:20 False  [the false side fails the check, and its line ran]  && line
(F) BRANCH crates/api/src/server.rs:22492:13 False  [the false side fails the check, and its line ran]  said.starts_with("transient — 0 failure(s) another attempt could change")
(F) BRANCH crates/api/src/server.rs:22493:20 False  [the false side fails the check, and its line ran]  && said.contains("2 of the store and not the vendor")
(F) BRANCH crates/api/src/server.rs:22494:20 False  [the false side fails the check, and its line ran]  && said.contains("and 0 it would repeat"),
(F) BRANCH crates/api/src/server.rs:22788:13 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:22788:47 False  [the false side fails the check, and its line ran]  fault.contains(".store-probe") && fault.contains("could not be unlinked"),
(F) BRANCH crates/api/src/server.rs:22797:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:23667:13 False  [the false side fails the check, and its line ran]  !read.merged.contributed.contains(&Vendor::Groww)
(F) BRANCH crates/api/src/server.rs:23668:20 False  [the false side fails the check, and its line ran]  && !read.merged.contributed.contains(&Vendor::Dhan),
(F) BRANCH crates/api/src/server.rs:24118:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24243:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24299:13 False  [the else of a let-else that panics, whose line ran]  let [(0, segment)] = receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24302:13 False  [the else of a let-else that panics, whose line ran]  let [receipt] = segment.receipts.as_slice() else {
(F) BRANCH crates/api/src/server.rs:24423:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24457:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24461:17 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24461:46 False  [the false side fails the check, and its line ran]  assert!(body.contains("run_busy") && body.contains(holder), "{body}");
(F) BRANCH crates/api/src/server.rs:24462:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24466:17 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24466:48 False  [the false side fails the check, and its line ran]  assert!(why.starts_with("run_busy") && why.contains(holder), "{why}");
(F) BRANCH crates/api/src/server.rs:24487:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, body)) = fno_landing(&page, Vec::new(), (&site, &wire, &asked), 1) else {
(F) BRANCH crates/api/src/server.rs:24496:13 False  [the else of a let-else that panics, whose line ran]  let Err((why, code)) = open_spot_landing(&spot, &site, 1) else {
(F) BRANCH crates/api/src/server.rs:24531:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24626:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24716:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:24818:13 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:25182:17 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25182:51 False  [the false side fails the check, and its line ran]  assert!(e.contains("unknown argument") && e.contains("usage"), "{e}");
(F) BRANCH crates/api/src/server.rs:25238:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25238:51 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_MASTERS") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:25345:13 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25345:45 False  [the false side fails the check, and its line ran]  refused.contains("pid=4242") && refused.contains("addr=127.0.0.1:8080"),
(F) BRANCH crates/api/src/server.rs:25541:13 False  [the false side fails the check, and its line ran]  body.contains(concat!("if let Some(warning) = ", "stamped {"))
(F) BRANCH crates/api/src/server.rs:25542:20 False  [the false side fails the check, and its line ran]  && body.contains(concat!(
(F) BRANCH crates/api/src/server.rs:25553:13 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25553:48 False  [the false side fails the check, and its line ran]  refused.contains("os error 28") && refused.contains("os error 5"),
(F) BRANCH crates/api/src/server.rs:25709:13 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:25709:45 False  [the false side fails the check, and its line ran]  html.contains("appended to") && !html.contains("NOT in the journal"),
(F) BRANCH crates/api/src/server.rs:25901:13 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:25901:46 False  [the false side fails the check, and its line ran]  text.contains("INE121A01024") && text.contains("INE121A08PJ0"),
(F) BRANCH crates/api/src/server.rs:25986:17 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25986:45 False  [the false side fails the check, and its line ran]  assert!(up.contains("NSE-NIFTY") && up.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25987:17 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25987:47 False  [the false side fails the check, and its line ran]  assert!(down.contains("NSE-NIFTY") && down.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25991:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:25991:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26007:17 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26007:46 False  [the false side fails the check, and its line ran]  assert!(one.contains("NSE-NIFTY") && one.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26014:13 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26014:43 False  [the false side fails the check, and its line ran]  past.contains("NSE-NIFTY") && past.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26030:17 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26030:47 False  [the false side fails the check, and its line ran]  html.contains("NSE-NIFTY") && html.contains("NSE-RELIANCE"),
(F) BRANCH crates/api/src/server.rs:26046:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26046:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26088:17 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26088:48 False  [the false side fails the check, and its line ran]  assert!(stale.contains("NSE-NIFTY") && stale.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26110:17 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26110:46 False  [the false side fails the check, and its line ran]  assert!(all.contains("NSE-NIFTY") && all.contains("NSE-RELIANCE"));
(F) BRANCH crates/api/src/server.rs:26498:13 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:26498:52 False  [the false side fails the check, and its line ran]  reach.contains(r#""feed":"groww""#) && reach.contains(r#""target":"n50""#),
(F) BRANCH crates/api/src/server.rs:27197:21 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27197:58 False  [the false side fails the check, and its line ran]  !fine.contains("400 Bad Request") && !fine.contains("reads no feed called"),
(F) BRANCH crates/api/src/server.rs:27247:17 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:27247:48 False  [the false side fails the check, and its line ran]  back.contains("2022-02-08") && back.contains("2022-01-08"),
(F) BRANCH crates/api/src/server.rs:27856:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:27856:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("09:15") && html.contains("15:30"), "{html}");
(F) BRANCH crates/api/src/server.rs:27875:17 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:27875:43 False  [the false side fails the check, and its line ran]  assert!(html.contains("groww") && html.contains("dhan"), "{html}");
(F) BRANCH crates/api/src/server.rs:28125:13 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28125:38 False  [the false side fails the check, and its line ran]  note.contains("debg") && note.contains("notalevel"),
(F) BRANCH crates/api/src/server.rs:28241:31 False  [the false side fails the check, and its line ran]  .all(|(t, l)| t == "pull" && *l == telemetry::Level::Debug),
(F) BRANCH crates/api/src/server.rs:28246:13 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28246:41 False  [the false side fails the check, and its line ran]  note.contains("DROPPED") && note.contains("api=trace"),
(F) BRANCH crates/api/src/server.rs:28454:13 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28454:49 False  [the false side fails the check, and its line ran]  refused.contains("BRUTEX_STORE") && refused.contains("HOME"),
(F) BRANCH crates/api/src/server.rs:28458:13 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:28458:42 False  [the false side fails the check, and its line ran]  refused.contains("bars/") && refused.contains("manifest/"),
(F) BRANCH crates/api/src/server.rs:28877:17 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = descriptor.transport else {
(F) BRANCH crates/api/src/server.rs:28952:13 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:28952:49 False  [the false side fails the check, and its line ran]  html.contains("NSE-INDEX-NIFTY") && html.contains("NSE-INDEX-BANKNIFTY"),
(F) BRANCH crates/api/src/server.rs:28956:13 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:28956:52 False  [the false side fails the check, and its line ran]  html.contains("groww: UNAVAILABLE") && html.contains("dhan: UNAVAILABLE"),
(F) BRANCH crates/api/src/server.rs:29149:13 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29149:44 False  [the false side fails the check, and its line ran]  !CREDENTIAL_FACT.is_empty() && CREDENTIAL_FACT.contains("no longer valid"),
(F) BRANCH crates/api/src/server.rs:29205:17 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29205:40 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29205:62 False  [the false side fails the check, and its line ran]  assert!(!plain.reached_wire && !plain.vendor_down && !plain.credential_dead);
(F) BRANCH crates/api/src/server.rs:29212:13 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29212:35 False  [the false side fails the check, and its line ran]  !cred.reached_wire && !cred.vendor_down,
(F) BRANCH crates/api/src/server.rs:29222:17 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29222:37 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29222:56 False  [the false side fails the check, and its line ran]  assert!(all.reached_wire && all.vendor_down && all.credential_dead);
(F) BRANCH crates/api/src/server.rs:29231:13 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29231:34 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29231:58 False  [the false side fails the check, and its line ran]  real.reached_wire && real.credential_dead && !real.vendor_down,
(F) BRANCH crates/api/src/server.rs:29449:13 False  [the false side fails the check, and its line ran]  words.starts_with("INTERRUPTED: an expired-F&O walk for 2026-07-01..2026-07-30")
(F) BRANCH crates/api/src/server.rs:29450:20 False  [the false side fails the check, and its line ran]  && words.contains("run the same walk again from /pull/fno")
(F) BRANCH crates/api/src/server.rs:29451:20 False  [the false side fails the check, and its line ran]  && !words.contains("/pull/spot")
(F) BRANCH crates/api/src/server.rs:29452:20 False  [the false side fails the check, and its line ran]  && !words.contains("press Pull"),
(F) BRANCH crates/api/src/server.rs:29650:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:30090:13 False  [the false side fails the check, and its line ran]  whole_body.contains(&format!(r#""expected":{}"#, owed.len()))
(F) BRANCH crates/api/src/server.rs:30091:20 False  [the false side fails the check, and its line ran]  && holed_body.contains(&format!(r#""expected":{}"#, owed.len())),
(F) BRANCH crates/api/src/server.rs:30099:13 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30099:60 False  [the false side fails the check, and its line ran]  whole_body.contains(r#""source":"record""#) && whole_body.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/server.rs:30370:17 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30370:62 False  [the false side fails the check, and its line ran]  damaged.contains("x-brutex-census-note:") && damaged.contains("UNREADABLE"),
(F) BRANCH crates/api/src/server.rs:30501:13 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30501:38 False  [the false side fails the check, and its line ran]  !clean.contains('\r') && !clean.contains('\n'),
(F) BRANCH crates/api/src/server.rs:30505:13 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:30505:51 False  [the false side fails the check, and its line ran]  clean.contains("groww UNREADABLE") && clean.contains("at /tmp/x"),
(F) BRANCH crates/api/src/server.rs:30829:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:31179:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(dhan) = transport else {
(F) BRANCH crates/api/src/server.rs:31243:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:31250:17 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:31250:49 False  [the false side fails the check, and its line ran]  assert!(spec.fno.by_name().is_none() && spec.fno.by_offset().is_none());
(F) BRANCH crates/api/src/server.rs:31847:17 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:31847:52 False  [the false side fails the check, and its line ran]  body.contains("detached_pull(") && body.contains(held),
(F) BRANCH crates/api/src/server.rs:31936:13 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:31936:57 False  [the false side fails the check, and its line ran]  !source.contains("sort_unstable_by_key") && !source.contains(".sort("),
(F) BRANCH crates/api/src/server.rs:32297:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:32516:26 False  [the false side fails the check, and its line ran]  .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
(F) BRANCH crates/api/src/server.rs:32741:17 False  [the else of a let-else that panics, whose line ran]  let Step::Again { wait_ms, throttled } = stepped else {
(F) BRANCH crates/api/src/server.rs:33090:13 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33090:45 False  [the false side fails the check, and its line ran]  code.contains("BAD_REQUEST") && code.contains("BAD_GATEWAY"),
(F) BRANCH crates/api/src/server.rs:33094:13 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33094:44 False  [the false side fails the check, and its line ran]  !code.contains("contains(") && !code.contains("starts_with("),
(F) BRANCH crates/api/src/server.rs:33158:13 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:33158:40 False  [the false side fails the check, and its line ran]  !code.contains("15:30") && !code.contains("930"),
(F) BRANCH crates/api/src/server.rs:33721:13 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:33721:43 False  [the false side fails the check, and its line ran]  page.contains(">376</td>") && page.contains(">375</td>"),
(F) BRANCH crates/api/src/server.rs:34510:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:34624:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:34745:13 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:34745:50 False  [the false side fails the check, and its line ran]  note.contains("BEFORE FIRST BAR") && note.contains("2018-01-01..=2023-06-23"),
(F) BRANCH crates/api/src/server.rs:34759:13 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:34759:51 False  [the false side fails the check, and its line ran]  failure.contains("NO ROWS AT ALL") && failure.contains("2016-03"),
(F) BRANCH crates/api/src/server.rs:34777:13 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:34777:43 False  [the false side fails the check, and its line ran]  failure.contains("1 of 3") && failure.contains(&late.from().to_string()),
(F) BRANCH crates/api/src/server.rs:34864:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) BRANCH crates/api/src/server.rs:35022:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = transport else {
(F) BRANCH crates/api/src/server.rs:35154:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "chunk", 1)
(F) BRANCH crates/api/src/server.rs:35155:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:35347:17 False  [the false side fails the check, and its line ran]  crate::emitted::says(record, "instrument", "NIFTY")
(F) BRANCH crates/api/src/server.rs:35348:24 False  [the false side fails the check, and its line ran]  && crate::emitted::says(record, "feed", "dhan"),
(F) BRANCH crates/api/src/server.rs:35352:17 False  [the false side fails the check, and its line ran]  crate::emitted::counts(record, "index", 1)
(F) BRANCH crates/api/src/server.rs:35353:24 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(record, "of", 1),
(F) BRANCH crates/api/src/server.rs:35762:13 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:35762:33 False  [the false side fails the check, and its line ran]  !ledger.opened() && ledger.opened_refused.is_some(),
(F) BRANCH crates/api/src/server.rs:35963:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "rung", "1min")
(F) BRANCH crates/api/src/server.rs:35964:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "target", asked.target.label()),
(F) BRANCH crates/api/src/server.rs:35979:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(mine, "failed", 0)
(F) BRANCH crates/api/src/server.rs:35980:20 False  [the false side fails the check, and its line ran]  && crate::emitted::counts(mine, "bars_stored", 0),
(F) BRANCH crates/api/src/server.rs:36029:13 False  [the false side fails the check, and its line ran]  crate::emitted::says(mine, "month", "2026-08")
(F) BRANCH crates/api/src/server.rs:36030:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "day", "2026-08-04")
(F) BRANCH crates/api/src/server.rs:36031:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "window", "2026-08-03..=2026-08-05")
(F) BRANCH crates/api/src/server.rs:36032:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(mine, "why", "the store refused the month"),
(F) BRANCH crates/api/src/server.rs:36084:13 False  [the false side fails the check, and its line ran]  crate::emitted::counts(ordinary, "status", 200)
(F) BRANCH crates/api/src/server.rs:36085:20 False  [the false side fails the check, and its line ran]  && crate::emitted::says(ordinary, "method", "GET")
(F) BRANCH crates/api/src/server.rs:36086:20 False  [the false side fails the check, and its line ran]  && ordinary.field("micros").is_some(),
(F) BRANCH crates/api/src/server.rs:36245:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) BRANCH crates/api/src/server.rs:36604:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36604:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36653:13 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(F) BRANCH crates/api/src/server.rs:36653:37 False  [the false side fails the check, and its line ran]  why.contains("1min") && why.contains("1day"),
(U) BRANCH crates/api/src/server.rs:37585:13 False  [index runs over 0..vocab::table::COUNT, and COUNT <= ConditionMask::BITS is a const assertion (crates/vocab/src/table.rs:1448-1452) with BITS = WORDS * 64 = 384 (crates/vocab/src/mask.rs:18, :44), so index is below 65,536 and u16::try_from is Ok]  let Ok(position) = u16::try_from(index) else {
(F) LINE crates/api/src/server_cloud_mutant_tests_e.rs:256  [failing arm of the statement at 255, which ran]  matches!(
(F) LINE crates/api/src/server_cloud_mutant_tests_e.rs:267  [panic/unreachable line]  panic!("the flush re-arms the head clock");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_e.rs:266:9 False  [the else of a let-else that panics, whose line ran]  let HeadState::Awaiting { deadline, .. } = wrapped.state else {
(U) LINE crates/api/src/sources.rs:1500  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  return Sessions {
(U) LINE crates/api/src/sources.rs:1501  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  json: json!({ "calendar": link, "why": "the calendar link would not parse" }),
(U) LINE crates/api/src/sources.rs:1502  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  not_held: None,
(U) LINE crates/api/src/sources.rs:1503  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  };
(U) LINE crates/api/src/sources.rs:1633  [pair comes from months.windows(2), and slice::windows(n) yields only slices of exactly n elements, so first() and get(1) are both Some]  continue;
(U) BRANCH crates/api/src/sources.rs:426:24 False  [the nibble is byte >> 4 or byte & 0x0F of a u8, so at most 15, and char::from_digit(n, 16) is Some for every n < 16]  if let Some(digit) = char::from_digit(u32::from(nibble), 16) {
(U) BRANCH crates/api/src/sources.rs:1499:9 False  [the link is /calendar.json?feed= + Vendor::as_str (lowercase letters, crates/core/src/vendor.rs:169-177) + &symbol= + query_value (unreserved bytes and %XX only) + &rung= + a Timeframe name (1s, 1min to 60min, 1day; crates/store/src/path.rs:360-499); every byte is one an origin-form URI accepts, and Symbol::new bounds the symbol at 1 to 24 bytes, so the link parses]  let Ok(uri) = link.parse::<axum::http::Uri>() else {
(U) BRANCH crates/api/src/sources.rs:1632:13 False  [pair comes from months.windows(2), and slice::windows(n) yields only slices of exactly n elements, so first() and get(1) are both Some]  let (Some(&before), Some(&after)) = (pair.first(), pair.get(1)) else {
(U) BRANCH crates/api/src/sources.rs:2458:16 False  [at enumerates the four-element array literal of o, h, l and c (0..=3) and equal is [false; 4], so get_mut(at) is Some]  if let Some(slot) = equal.get_mut(at) {
(U) BRANCH crates/api/src/sources.rs:3578:16 False  [row is day_row's answer, and day_row has no return: its tail expression is the object literal json!({"day", "absent", ..., "records_read"}) (sources.rs, fn day_row), so as_object_mut() is Some]  if let Some(object) = row.as_object_mut() {
(F) LINE crates/api/src/sources_tests.rs:983  [failing arm of the statement at 974, which ran]  a["marks"]
(F) LINE crates/api/src/sources_tests.rs:1365  [failing arm of the statement at 1358, which ran]  matches!(
(F) BRANCH crates/api/src/sources_tests.rs:1180:25 False  [the false side fails the check, and its line ran]  .all(|flag| flag != "basis_differs" && flag != "basis_step"),
(F) BRANCH crates/api/src/sources_tests.rs:2430:13 False  [the false side fails the check, and its line ran]  assert!(!basis_differs(1_002, 1_000) && basis_differs(1_003, 1_000));
(F) BRANCH crates/api/src/sources_tests.rs:2430:45 False  [the false side fails the check, and its line ran]  assert!(!basis_differs(1_002, 1_000) && basis_differs(1_003, 1_000));
(U) LINE crates/api/src/speed.rs:349  [std::array::from_fn for [u64; OUTCOMES] passes i in 0..OUTCOMES, and both Frozen snapshots hold outcomes: [u32; OUTCOMES] (speed.rs:225), so both get(i) are Some]  _ => 0,
(U) BRANCH crates/api/src/speed.rs:270:16 False  [Live::new, the one constructor of Live (speed.rs:206-212), makes BUCKETS buckets, and bucket_of answers below BUCKETS for every u64: micros < SUB_BUCKETS (8) answers micros; an octave past TOP_OCTAVE (35) answers BUCKETS - 1; otherwise SUB_BITS <= octave <= TOP_OCTAVE and the answer is at most SUB_BUCKETS + (TOP_OCTAVE - SUB_BITS) * SUB_BUCKETS + SUB_BUCKETS - 1 = BUCKETS - 1 (speed.rs:64-75, :95-106)]  if let Some(bucket) = self.live.buckets.get(bucket_of(micros)) {
(U) BRANCH crates/api/src/speed.rs:273:16 False  [Outcome::slot answers 0 to 4 (speed.rs:170-178) and outcomes is [AtomicU64; OUTCOMES] with OUTCOMES = 5 (speed.rs:153, :194)]  if let Some(count) = self.live.outcomes.get(outcome.slot()) {
(U) BRANCH crates/api/src/speed.rs:1087:16 False  [Pending is built only by Pending::start, with row: Some (speed.rs:1072-1077), and finish takes self by value, so its take() is the first of that Pending's row; only Drop, after finish, meets None]  if let Some(row) = self.row.take() {
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
(F) LINE crates/api/src/store_wire.rs:347  [panic/unreachable line]  panic!("warm read rebuilt")
(F) LINE crates/api/src/store_wire.rs:403  [panic/unreachable line]  panic!("GDFL's body was dropped by another vendor's read")
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
(F) BRANCH crates/api/src/trades.rs:860:13 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/trades.rs:860:32 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/trades.rs:1342:13 False  [the false side fails the check, and its line ran]  body.contains("the child was absent at its open.")
(F) BRANCH crates/api/src/trades.rs:1343:20 False  [the false side fails the check, and its line ran]  && body.contains("could not be checked for absence"),
(F) LINE crates/api/src/verify.rs:397  [failing arm of the statement at 394, which ran]  empty.say()
(F) LINE crates/api/src/verify.rs:427  [failing arm of the statement at 423, which ran]  bad.say()
(F) LINE crates/api/src/verify.rs:516  [panic/unreachable line]  panic!("a recovered counter still loads: {:?}", dhan.state.name());
(F) BRANCH crates/api/src/verify.rs:515:13 False  [the else of a let-else that panics, whose line ran]  let Census::Held { ref manifest } = dhan.state else {
(F) LINE crates/api/tests/cloud_mutants_bars.rs:511  [panic/unreachable line]  panic!("February is owed: {gated:?}");
(F) LINE crates/api/tests/cloud_mutants_bars.rs:620  [panic/unreachable line]  panic!("every month is owed: {gated:?}");
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:946:13 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:946:52 False  [the false side fails the check, and its line ran]  none.starts_with("too_many_series") && none.contains("0 were named"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2116:13 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2116:48 False  [the false side fails the check, and its line ran]  unopened[0].contains("2025-04") && unopened[0].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2120:13 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2120:48 False  [the false side fails the check, and its line ran]  unopened[1].contains("2025-05") && unopened[1].contains("none of its 2 kept rows"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2354:13 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2354:55 False  [the false side fails the check, and its line ran]  got.faults[1].starts_with("2025-06: ") && got.faults[1].contains("not a regular file"),
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2649:13 False  [the false side fails the check, and its line ran]  why.contains(&format!("{} is not a regular file", srt.display()))
(F) BRANCH crates/api/tests/cloud_mutants_bars.rs:2650:20 False  [the false side fails the check, and its line ran]  && why.starts_with("NSE-INDEX-NIFTY 2025-05: 3 rows read: "),
(F) LINE crates/api/tests/logs_r3_24.rs:351  [panic/unreachable line]  panic!("{query}: no end in 200 pages, {} line(s) read", seqs.len());
`````

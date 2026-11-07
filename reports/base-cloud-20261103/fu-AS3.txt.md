# fu-AS3.txt.md

One fenced block: the (F), (U) and (H) rows of shard G3's files on the code tip (`gaps-AS3.txt.md` holds the figures
and the GAP rows).

`````text
(F)/(U)/(H) list of BASE-COV-API-AS3 (shard G3), tip 98908e7773e8f4d1d81a3cf9a6a2ac9f9a73884b, whole:api, from /var/tmp/as3/routeb7.log (continuation -5, 7 Nov: every row of its measurement of 4a86f408 carried over through a line map from git diff 4a86f408 98908e77, its text checked; the rows that did not carry over classified by hand).
(F) = the failing side of an assert!/assert_eq!/assert_ne!/panic!/unreachable! in test code whose statement ran (recipe step 4): a panic line, a message argument of a check (evaluated only when it fails), the false side of a check's condition (an && operand, the last || operand, a matches! or its guard, an .all()/.is_some_and()/.is_ok_and() closure inside it, the no-match arm of a matches! that is the whole check), the else of a let-else that panics. Classified by the session's classify.mjs (node, the rule on each row); 52 of them read by hand against the source in two samples spread over the files (the first sample showed a closure inside a condition is no failing arm, and the rule was narrowed before this list).
(U) = a row the code proves never runs, its proof beside it (no row rests on "hard to reach", "needs a fault" or "never happens").
(H) = a row a root host red reaches only when its test runs as non-root: the three G3 root reds were run as uid 65534 with the same instrumented binary (the report names them); covered there, owed to the Mac's Route B.
(F) 235 lines 312 sides; (U) 100 lines 54 sides; (H) 44 lines 2 sides.

(F) BRANCH crates/api/src/autopilot.rs:4813:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) LINE crates/api/src/autopilot.rs:4814  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) BRANCH crates/api/src/autopilot.rs:4816:13 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::HistoryFloor::Rolling { years } = spec.history_floor else {
(F) LINE crates/api/src/autopilot.rs:4817  [panic/unreachable line]  panic!("Dhan's floor is the rolling one");
(F) BRANCH crates/api/src/autopilot.rs:4877:17 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(F) LINE crates/api/src/autopilot.rs:4878  [panic/unreachable line]  panic!("{} is driven, so it is HTTP", feed.display());
(F) LINE crates/api/src/autopilot.rs:4883  [a message argument of the statement at 4880, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4888  [a message argument of the statement at 4885, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4893  [a message argument of the statement at 4890, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4905  [a message argument of the statement at 4902, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/autopilot.rs:4915  [a message argument of the statement at 4912, which ran: evaluated only when it fails]  feed.display()
(F) BRANCH crates/api/src/autopilot.rs:4942:45 False  [the false side fails the check, and its line ran]  !path_names_the_rung || spec.granularity_token(rung).is_some(),
(F) LINE crates/api/src/autopilot.rs:4945  [a message argument of the statement at 4941, which ran: evaluated only when it fails]  feed.display()
(F) BRANCH crates/api/src/autopilot.rs:5057:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&dead) else {
(F) LINE crates/api/src/autopilot.rs:5058  [panic/unreachable line]  panic!("a same-value verdict must halt");
(F) BRANCH crates/api/src/autopilot.rs:5142:13 False  [the else of a let-else that panics, whose line ran]  let Some(Next::Stall { reason }) = verdicts.last() else {
(F) LINE crates/api/src/autopilot.rs:5143  [panic/unreachable line]  panic!("attempt {MAX_MONTH_ATTEMPTS} must stall, got {verdicts:?}");
(F) LINE crates/api/src/autopilot.rs:5161  [a message argument of the statement at 5158, which ran: evaluated only when it fails]  status.json()
(F) BRANCH crates/api/src/autopilot.rs:5312:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halt_kind == Some(Halt::Census) && f.halted.is_some()),
(F) BRANCH crates/api/src/autopilot.rs:5317:17 False  [the false side fails the check, and its line ran]  report.halted.contains(drift) && report.halted.contains("is not known"),
(F) BRANCH crates/api/src/autopilot.rs:5317:50 False  [the false side fails the check, and its line ran]  report.halted.contains(drift) && report.halted.contains("is not known"),
(F) BRANCH crates/api/src/autopilot.rs:5337:50 False  [the false side fails the check, and its line ran]  Census::Unreadable { reason, .. } if reason == "entry 41 fails its own checksum"
(F) BRANCH crates/api/src/autopilot.rs:5394:13 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { reason, .. } = &refused.state else {
(F) LINE crates/api/src/autopilot.rs:5395  [panic/unreachable line]  panic!("a census whose migration receipt will not read is refused, never held");
(F) BRANCH crates/api/src/autopilot.rs:5398:13 False  [the false side fails the check, and its line ran]  reason.contains("is not known")
(F) BRANCH crates/api/src/autopilot.rs:5399:20 False  [the false side fails the check, and its line ran]  && reason.contains("MIGRATED")
(F) BRANCH crates/api/src/autopilot.rs:5400:20 False  [the false side fails the check, and its line ran]  && reason.contains("cannot be read"),
(F) LINE crates/api/src/autopilot.rs:5464  [a message argument of the statement at 5460, which ran: evaluated only when it fails]  action.slug()
(F) BRANCH crates/api/src/autopilot.rs:5531:13 False  [the false side fails the check, and its line ran]  body.contains("the broker credential is dead")
(F) BRANCH crates/api/src/autopilot.rs:5532:20 False  [the false side fails the check, and its line ran]  && body.contains("the store refused the same write twice"),
(F) BRANCH crates/api/src/autopilot.rs:6041:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&full) else {
(F) LINE crates/api/src/autopilot.rs:6042  [panic/unreachable line]  panic!("a repeated disk failure must halt");
(F) LINE crates/api/src/autopilot.rs:6767  [a message argument of the statement at 6764, which ran: evaluated only when it fails]  phase.state()
(F) BRANCH crates/api/src/autopilot.rs:7120:13 False  [the false side fails the check, and its line ran]  state.halted.is_none() && state.halt_kind.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7120:39 False  [the false side fails the check, and its line ran]  state.halted.is_none() && state.halt_kind.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7140:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = token.observe(&dead) else {
(F) LINE crates/api/src/autopilot.rs:7141  [panic/unreachable line]  panic!("a same-value verdict must halt");
(F) BRANCH crates/api/src/autopilot.rs:7190:17 False  [the else of a let-else that panics, whose line ran]  let Next::Stall { reason: said } = state.observe(&out) else {
(F) LINE crates/api/src/autopilot.rs:7191  [panic/unreachable line]  panic!("{credential:?}: the bound stalls the month, it does not halt the feed");
(F) BRANCH crates/api/src/autopilot.rs:7223:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { reason } = state.observe(&blocked) else {
(F) LINE crates/api/src/autopilot.rs:7224  [panic/unreachable line]  panic!("an unusable configuration halts");
(F) BRANCH crates/api/src/autopilot.rs:7292:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halt_kind == Some(Halt::Census) && f.halted.is_some()),
(F) BRANCH crates/api/src/autopilot.rs:7320:26 False  [the false side fails the check, and its line ran]  .all(|f| f.halted.is_none() && f.halt_kind.is_none()),
(F) BRANCH crates/api/src/autopilot.rs:7326:26 False  [the false side fails the check, and its line ran]  .all(|r| r.halted.is_empty() && r.last_reason.contains("manifest loads again")),
(F) LINE crates/api/src/autopilot.rs:7328  [a message argument of the statement at 7323, which ran: evaluated only when it fails]  reports.iter().map(|r| &r.last_reason).collect::<Vec<_>>()
(F) BRANCH crates/api/src/autopilot.rs:7364:13 False  [the else of a let-else that panics, whose line ran]  let Due::Spent { saying } = store_due(None, 0) else {
(F) LINE crates/api/src/autopilot.rs:7365  [panic/unreachable line]  panic!("an unarmed feed must not answer Now");
(F) BRANCH crates/api/src/autopilot.rs:7388:17 False  [the else of a let-else that panics, whose line ran]  let Due::Now { made } = store_due(Some(&probe), now) else {
(F) LINE crates/api/src/autopilot.rs:7389  [panic/unreachable line]  panic!("probe {expected} of {STORE_PROBES} must be due, at {now}");
(F) BRANCH crates/api/src/autopilot.rs:7400:13 False  [the else of a let-else that panics, whose line ran]  let Due::Spent { saying } = store_due(Some(&probe), now.saturating_add(1_000_000)) else {
(F) LINE crates/api/src/autopilot.rs:7401  [panic/unreachable line]  panic!("the {STORE_PROBES}-probe allowance must be spent, not renewed by time");
(F) LINE crates/api/src/autopilot.rs:7443  [a message argument of the statement at 7440, which ran: evaluated only when it fails]  missing.display()
(F) BRANCH crates/api/src/autopilot.rs:7575:13 False  [the false side fails the check, and its line ran]  why.contains("already holds the probe's name") && why.contains(STORE_PROBE_PREFIX),
(F) BRANCH crates/api/src/autopilot.rs:7575:63 False  [the false side fails the check, and its line ran]  why.contains("already holds the probe's name") && why.contains(STORE_PROBE_PREFIX),
(F) BRANCH crates/api/src/autopilot.rs:7632:13 False  [the else of a let-else that panics, whose line ran]  let Next::Halt { .. } = first.observe(&disk) else {
(F) LINE crates/api/src/autopilot.rs:7633  [panic/unreachable line]  panic!("the same store refusal twice must halt");
(F) BRANCH crates/api/src/autopilot.rs:7654:13 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7654:38 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7654:66 False  [the false side fails the check, and its line ran]  back.halted.is_none() && back.halt_kind.is_none() && back.probe.is_none(),
(F) BRANCH crates/api/src/autopilot.rs:7729:17 False  [the false side fails the check, and its line ran]  saying.contains("2021-03") && saying.contains("being reconsidered"),
(F) BRANCH crates/api/src/autopilot.rs:7729:47 False  [the false side fails the check, and its line ran]  saying.contains("2021-03") && saying.contains("being reconsidered"),
(F) BRANCH crates/api/src/autopilot.rs:7849:13 False  [the else of a let-else that panics, whose line ran]  let Settled::Complete { instruments } = Settled::over(&one) else {
(F) LINE crates/api/src/autopilot.rs:7850  [panic/unreachable line]  panic!("one tracked instrument is a universe");
(F) BRANCH crates/api/src/autopilot.rs:7972:13 False  [the false side fails the check, and its line ran]  detail.contains("NOT COMPLETE") && detail.contains("NOTHING IS TRACKED"),
(F) BRANCH crates/api/src/autopilot.rs:7972:48 False  [the false side fails the check, and its line ran]  detail.contains("NOT COMPLETE") && detail.contains("NOTHING IS TRACKED"),
(F) LINE crates/api/src/autopilot.rs:8114  [a message argument of the statement at 8109, which ran: evaluated only when it fails]  kind.word()
(F) BRANCH crates/api/src/autopilot.rs:8117:21 False  [the false side fails the check, and its line ran]  assert!(state.halted.is_none() && state.halt_kind.is_none());
(F) BRANCH crates/api/src/autopilot.rs:8117:47 False  [the false side fails the check, and its line ran]  assert!(state.halted.is_none() && state.halt_kind.is_none());
(F) BRANCH crates/api/src/autopilot.rs:8281:17 False  [the false side fails the check, and its line ran]  assert!(due > before && due <= before + 3, "{due} vs {before}");
(F) BRANCH crates/api/src/autopilot.rs:8281:33 False  [the false side fails the check, and its line ran]  assert!(due > before && due <= before + 3, "{due} vs {before}");
(F) LINE crates/api/src/autopilot.rs:8460  [a message argument of the statement at 8457, which ran: evaluated only when it fails]  site.autopilot.json()
(F) LINE crates/api/src/autopilot.rs:8472  [a message argument of the statement at 8469, which ran: evaluated only when it fails]  site.autopilot.json()
(F) BRANCH crates/api/src/autopilot.rs:8672:17 False  [the else of a let-else that panics, whose line ran]  let Some(&(_, flies, paused, why)) = CASES.iter().find(|(named, ..)| *named == case)
(F) LINE crates/api/src/autopilot.rs:8674  [panic/unreachable line]  panic!("unknown case {case}");
(F) LINE crates/api/src/autopilot.rs:8844  [a message argument of the statement at 8841, which ran: evaluated only when it fails]  reports[0].halted
(F) BRANCH crates/api/src/autopilot.rs:8891:13 False  [the false side fails the check, and its line ran]  shown >= before && shown <= after,
(F) BRANCH crates/api/src/autopilot.rs:8891:32 False  [the false side fails the check, and its line ran]  shown >= before && shown <= after,
(F) LINE crates/api/src/autopilot.rs:8933  [a message argument of the statement at 8930, which ran: evaluated only when it fails]  site.autopilot.json()
(F) LINE crates/api/src/autopilot.rs:9010  [a message argument of the statement at 9007, which ran: evaluated only when it fails]  site.autopilot.json()
(F) BRANCH crates/api/src/autopilot.rs:9025:13 False  [the false side fails the check, and its line ran]  why.contains("did not complete. Attempt ") && why.contains("NO ROWS AT ALL"),
(F) BRANCH crates/api/src/autopilot.rs:9025:59 False  [the false side fails the check, and its line ran]  why.contains("did not complete. Attempt ") && why.contains("NO ROWS AT ALL"),
(F) BRANCH crates/api/src/autopilot.rs:9121:13 False  [the else of a let-else that panics, whose line ran]  let audit::Log::Held { records, .. } = journal.look() else {
(F) LINE crates/api/src/autopilot.rs:9122  [panic/unreachable line]  panic!("the tick wrote its journal: {:?}", journal.look());
(F) BRANCH crates/api/src/autopilot.rs:9143:13 False  [the false side fails the check, and its line ran]  whole.starts_with(&closed.note) && !closed.note.is_empty(),
(F) BRANCH crates/api/src/autopilot.rs:9143:48 False  [the false side fails the check, and its line ran]  whole.starts_with(&closed.note) && !closed.note.is_empty(),
(F) LINE crates/api/src/bars_export.rs:1183  [panic/unreachable line]  panic!("the pools stayed full");
(F) BRANCH crates/api/src/bars_export.rs:1742:13 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("2025-06"),
(F) BRANCH crates/api/src/bars_export.rs:1742:50 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("2025-06"),
(F) BRANCH crates/api/src/bars_export.rs:1862:13 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("first: 2025-07: "),
(F) BRANCH crates/api/src/bars_export.rs:1862:50 False  [the false side fails the check, and its line ran]  body.contains("export_not_whole") && body.contains("first: 2025-07: "),
(F) BRANCH crates/api/src/bars_export.rs:1942:13 False  [the false side fails the check, and its line ran]  body.contains("\"being_written\":true") && body.contains("r3-10-export-writer"),
(F) BRANCH crates/api/src/bars_export.rs:1942:56 False  [the false side fails the check, and its line ran]  body.contains("\"being_written\":true") && body.contains("r3-10-export-writer"),
(F) LINE crates/api/src/bars_route.rs:1581  [a message argument of the statement at 1575, which ran: evaluated only when it fails]  &field[field.len() - 80..]
(F) BRANCH crates/api/src/bars_route.rs:2028:25 False  [the else of a let-else that panics, whose line ran]  let Ok((status, body)) = got.as_ref() else {
(F) LINE crates/api/src/bars_route.rs:2377  [panic/unreachable line]  panic!("the pool stayed full for two seconds");
(F) LINE crates/api/src/bars_route.rs:2418  [panic/unreachable line]  panic!("the pool stayed full");
(F) LINE crates/api/src/bars_route.rs:2446  [panic/unreachable line]  panic!("the pool stayed full");
(F) BRANCH crates/api/src/bars_route.rs:2660:40 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("not held") && why.contains("2024-01")),
(F) BRANCH crates/api/src/bars_route.rs:2891:17 False  [the else of a let-else that panics, whose line ran]  let Err(why) = WindowAsk::parse(&format!("{base}&{bound}={text}")) else {
(F) LINE crates/api/src/bars_route.rs:2892  [panic/unreachable line]  panic!("{bound}={text} was read");
(F) LINE crates/api/src/bars_series.rs:1942  [a message argument of the statement at 1939, which ran: evaluated only when it fails]  memo.len(),
(F) LINE crates/api/src/bars_series.rs:1943  [a message argument of the statement at 1939, which ran: evaluated only when it fails]  memo.turns()
(F) BRANCH crates/api/src/bars_series.rs:2609:13 False  [the false side fails the check, and its line ran]  whole_s.keeps_seconds() && gapped_s.keeps_seconds(),
(F) BRANCH crates/api/src/bars_series.rs:2609:40 False  [the false side fails the check, and its line ran]  whole_s.keeps_seconds() && gapped_s.keeps_seconds(),
(F) LINE crates/api/src/bars_series.rs:2715  [a message argument of the statement at 2712, which ran: evaluated only when it fails]  memo.bytes()
(F) LINE crates/api/src/bars_series.rs:2726  [a message argument of the statement at 2723, which ran: evaluated only when it fails]  memo.bytes()
(F) LINE crates/api/src/bars_window_route_tests.rs:159  [panic/unreachable line]  panic!("the pool stayed full for two seconds");
(F) BRANCH crates/api/src/bars_window_route_tests.rs:574:9 False  [the false side fails the check, and its line ran]  withheld[0].contains("`v`") && withheld[0].contains(&past.to_string()),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:574:40 False  [the false side fails the check, and its line ran]  withheld[0].contains("`v`") && withheld[0].contains(&past.to_string()),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:580:9 False  [the false side fails the check, and its line ran]  none.is_empty() && edge.contains("9007199254740991"),
(F) BRANCH crates/api/src/bars_window_route_tests.rs:580:28 False  [the false side fails the check, and its line ran]  none.is_empty() && edge.contains("9007199254740991"),
(F) LINE crates/api/src/bars_window_route_tests.rs:902  [a message argument of the statement at 899, which ran: evaluated only when it fails]  value(a),
(F) LINE crates/api/src/bars_window_route_tests.rs:903  [a message argument of the statement at 899, which ran: evaluated only when it fails]  value(b)
(F) BRANCH crates/api/src/bars_window_route_tests.rs:1263:24 False  [the false side fails the check, and its line ran]  .all(|row| !row.oi_applies && row.oichg_why == "not_a_derivative")
(F) BRANCH crates/api/src/bars_window_route_tests.rs:2140:9 False  [the else of a let-else that panics, whose line ran]  let Some(note) = copied.iter().find(|note| note.starts_with(&copy)) else {
(F) LINE crates/api/src/bars_window_route_tests.rs:2141  [panic/unreachable line]  panic!("{copied:?}");
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:165:13 False  [the false side fails the check, and its line ran]  why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:165:49 False  [the false side fails the check, and its line ran]  why.contains("1610612736 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES")
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:167:17 False  [the false side fails the check, and its line ran]  assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
(F) BRANCH crates/api/src/booleanevidencejson_tests.rs:167:47 False  [the false side fails the check, and its line ran]  assert!(why.contains(model.name()) && why.contains("no body prefix returned"));
(F) BRANCH crates/api/src/census_request_tests.rs:327:81 False  [the false side fails the check, and its line ran]  || matches!(located, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
(F) BRANCH crates/api/src/census_request_tests.rs:1001:9 False  [the false side fails the check, and its line ran]  page.contains("NSE/CASH") && page.contains("NSE/INDEX"),
(F) BRANCH crates/api/src/census_request_tests.rs:1001:38 False  [the false side fails the check, and its line ran]  page.contains("NSE/CASH") && page.contains("NSE/INDEX"),
(F) BRANCH crates/api/src/census_request_tests.rs:1069:9 False  [the false side fails the check, and its line ran]  exchange.contains(r#""target":{"kind":"exchange"}"#)
(F) BRANCH crates/api/src/census_request_tests.rs:1070:16 False  [the false side fails the check, and its line ran]  && exchange.contains(r#""derivedFrom":[]"#),
(F) LINE crates/api/src/census_request_tests.rs:1225  [a message argument of the statement at 1221, which ran: evaluated only when it fails]  path.display()
(F) BRANCH crates/api/src/census_request_tests.rs:1425:13 False  [the false side fails the check, and its line ran]  page.contains("DHAN · CASH · 2025-05") && page.contains("<b>1</b> bar(s)"),
(F) BRANCH crates/api/src/census_request_tests.rs:1425:57 False  [the false side fails the check, and its line ran]  page.contains("DHAN · CASH · 2025-05") && page.contains("<b>1</b> bar(s)"),
(F) LINE crates/api/src/census_request_tests.rs:1807  [the no-match arm of the matches! that is the whole check (llvm-cov maps its _ => false to this line): the side that fails it]  matches!(
(F) LINE crates/api/src/census_request_tests.rs:1820  [a message argument of the statement at 1817, which ran: evaluated only when it fails]  dhan.note()
(F) BRANCH crates/api/src/census_request_tests.rs:1977:42 False  [the false side fails the check, and its line ran]  matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
(F) LINE crates/api/src/census_request_tests.rs:1986  [a message argument of the statement at 1983, which ran: evaluated only when it fails]  at_path.file_type()
(F) BRANCH crates/api/src/census_request_tests.rs:2037:9 False  [the else of a let-else that panics, whose line ran]  let census::Census::Unreadable { fault, .. } = &state else {
(F) LINE crates/api/src/census_request_tests.rs:2038  [panic/unreachable line]  panic!("`read_vendor` names an unreadable census: {state:?}");
(F) BRANCH crates/api/src/census_request_tests.rs:2040:9 False  [the else of a let-else that panics, whose line ran]  let census::Fault::Io(kind) = *fault else {
(F) LINE crates/api/src/census_request_tests.rs:2041  [panic/unreachable line]  panic!("`read_vendor` names an I/O fault: {state:?}");
(F) BRANCH crates/api/src/census_request_tests.rs:2231:73 False  [the false side fails the check, and its line ran]  matches!(state, census::Census::Unreadable { fault, .. } if fault == from),
(F) BRANCH crates/api/src/census_request_tests.rs:2340:9 False  [the else of a let-else that panics, whose line ran]  let Some(dhan) = one_off.get_mut(1) else {
(F) LINE crates/api/src/census_request_tests.rs:2341  [panic/unreachable line]  panic!("five rows");
(F) BRANCH crates/api/src/census_request_tests.rs:2419:21 False  [the else of a let-else that panics, whose line ran]  let Census::Unreadable { ref reason, fault } = after.state else {
(F) LINE crates/api/src/census_request_tests.rs:2420  [panic/unreachable line]  panic!("{why}: served unreadable, as {:?}", after.state);
(F) LINE crates/api/src/census_request_tests.rs:2680  [a message argument of the statement at 2676, which ran: evaluated only when it fails]  fixture.fresh_states()
(F) BRANCH crates/api/src/census_request_tests.rs:2832:31 False  [the false side fails the check, and its line ran]  .all(|record| record.level == telemetry::Level::Warn
(F) BRANCH crates/api/src/census_request_tests.rs:3066:9 False  [the false side fails the check, and its line ran]  report.contains(concat!("eprintln", "!"))
(F) BRANCH crates/api/src/census_request_tests.rs:3067:16 False  [the false side fails the check, and its line ran]  && report.contains(".compare_exchange(false, true,"),
(F) BRANCH crates/api/src/census_request_tests.rs:3167:13 False  [the false side fails the check, and its line ran]  body.contains("NIFTY") && body.contains("2025-05"),
(F) BRANCH crates/api/src/census_request_tests.rs:3167:39 False  [the false side fails the check, and its line ran]  body.contains("NIFTY") && body.contains("2025-05"),
(F) LINE crates/api/src/census_request_tests.rs:3243  [a message argument of the statement at 3240, which ran: evaluated only when it fails]  partial.calendar.map(|calendar| calendar.sessions())
(F) BRANCH crates/api/src/census_request_tests.rs:3300:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = transport else {
(F) LINE crates/api/src/census_request_tests.rs:3301  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) BRANCH crates/api/src/census_request_tests.rs:3686:17 False  [the false side fails the check, and its line ran]  raced.starts_with("503 ")
(F) BRANCH crates/api/src/census_request_tests.rs:3687:24 False  [the false side fails the check, and its line ran]  && raced.contains(r#""census":"unreadable""#)
(F) BRANCH crates/api/src/census_request_tests.rs:3688:24 False  [the false side fails the check, and its line ran]  && raced.contains(CONTRADICTED_ABSENCE),
(F) BRANCH crates/api/src/dayflags.rs:6972:13 False  [the false side fails the check, and its line ran]  phases.contains(&"halted") && phases.contains(&"reopening_auction"),
(F) BRANCH crates/api/src/dayflags.rs:6972:43 False  [the false side fails the check, and its line ran]  phases.contains(&"halted") && phases.contains(&"reopening_auction"),
(F) LINE crates/api/src/dayflags.rs:7120  [a message argument of the statement at 7115, which ran: evaluated only when it fails]  none["lineage"]
(F) LINE crates/api/src/dayflags.rs:8478  [a message argument of the statement at 8473, which ran: evaluated only when it fails]  says(muhurat)
(F) LINE crates/api/src/dayflags.rs:8520  [a message argument of the statement at 8515, which ran: evaluated only when it fails]  says(nothing)
(F) BRANCH crates/api/src/dayflags.rs:8556:58 False  [the false side fails the check, and its line ran]  matches!(first.via, Via::Walk { ref why } if why.contains("no index")),
(F) BRANCH crates/api/src/dayflags.rs:8715:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("1 of the index's entries cannot be read")
(F) BRANCH crates/api/src/dayflags.rs:8787:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("is not a refusal entry")
(F) BRANCH crates/api/src/dayflags.rs:8788:24 False  [the false side fails the check, and its line ran]  && why.contains("ZZOIL.idx")
(F) BRANCH crates/api/src/dayflags.rs:8847:42 False  [the false side fails the check, and its line ran]  Via::Walk { ref why } if why.contains("covers 4 records and the journal holds 2")
(F) BRANCH crates/api/src/dayflags.rs:8936:13 False  [the else of a let-else that panics, whose line ran]  let Some(PullUnread::Warming { records, .. }) = unread else {
(F) LINE crates/api/src/dayflags.rs:8937  [panic/unreachable line]  panic!("a request walked beside the start, or the journal is gone: {unread:?}");
(F) LINE crates/api/src/dayflags.rs:8947  [a message argument of the statement at 8944, which ran: evaluated only when it fails]  got["sources"]
(F) LINE crates/api/src/dayflags.rs:8959  [a message argument of the statement at 8950, which ran: evaluated only when it fails]  got["notes"]
(F) BRANCH crates/api/src/dayflags.rs:9704:26 False  [the false side fails the check, and its line ran]  .all(|s| !s.starts_with("the import placed")
(F) LINE crates/api/src/dayflags.rs:9862  [a message argument of the statement at 9859, which ran: evaluated only when it fails]  sampled_rule_held()
(F) BRANCH crates/api/src/dayflags_cover_tests.rs:1140:9 False  [the else of a let-else that panics, whose line ran]  let Some(PullUnread::Failed(failed)) = unread else {
(F) LINE crates/api/src/dayflags_cover_tests.rs:1141  [panic/unreachable line]  panic!("a journal under a file is refused: {unread:?}");
(F) BRANCH crates/api/src/dayflags_cover_tests.rs:1197:9 False  [the else of a let-else that panics, whose line ran]  let Some(PullUnread::Failed(request)) = unread else {
(F) LINE crates/api/src/dayflags_cover_tests.rs:1198  [panic/unreachable line]  panic!("a folder is refused on a request: {unread:?}");
(F) BRANCH crates/api/src/dayflags_cover_tests.rs:1305:13 False  [the else of a let-else that panics, whose line ran]  let Opened::Refused(why) = opened else {
(F) LINE crates/api/src/dayflags_cover_tests.rs:1306  [panic/unreachable line]  panic!("{symbol:?}: a path the store cannot name is refused");
(F) BRANCH crates/api/src/dayflags_cover_tests.rs:1335:9 False  [the else of a let-else that panics, whose line ran]  let Opened::Refused(why) = gone else {
(F) LINE crates/api/src/dayflags_cover_tests.rs:1336  [panic/unreachable line]  panic!("a served revision that is gone is refused");
(F) LINE crates/api/src/dayflags_filter_tests.rs:144  [a message argument of the statement at 139, which ran: evaluated only when it fails]  says(&record)
(F) LINE crates/api/src/dayflags_filter_tests.rs:248  [a message argument of the statement at 243, which ran: evaluated only when it fails]  says(&bank)
(F) LINE crates/api/src/dayflags_filter_tests.rs:435  [a message argument of the statement at 430, which ran: evaluated only when it fails]  says(&put)
(F) LINE crates/api/src/dayflags_filter_tests.rs:595  [a message argument of the statement at 590, which ran: evaluated only when it fails]  says(&bank)
(F) LINE crates/api/src/dayflags_filter_tests.rs:871  [a message argument of the statement at 866, which ran: evaluated only when it fails]  says(&abcapital)
(F) BRANCH crates/api/src/feeds_route.rs:1763:13 False  [the false side fails the check, and its line ran]  body.contains(r#""standing":"operator""#)
(F) BRANCH crates/api/src/feeds_route.rs:1764:20 False  [the false side fails the check, and its line ran]  && body.contains(r#""standing":"vendor_doc""#),
(F) BRANCH crates/api/src/feeds_route.rs:1786:13 False  [the false side fails the check, and its line ran]  oldest.starts_with("20") && oldest.contains('-'),
(F) BRANCH crates/api/src/feeds_route.rs:1786:41 False  [the false side fails the check, and its line ran]  oldest.starts_with("20") && oldest.contains('-'),
(F) BRANCH crates/api/src/feeds_route.rs:2011:13 False  [the false side fails the check, and its line ran]  fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
(F) BRANCH crates/api/src/feeds_route.rs:2011:55 False  [the false side fails the check, and its line ran]  fields.contains(r#""kind":"unknown""#) && fields.contains(r#""source":null"#),
(F) BRANCH crates/api/src/feeds_route.rs:2233:13 False  [the false side fails the check, and its line ran]  !why.is_empty() && !why.contains("vendor-data"),
(F) BRANCH crates/api/src/feeds_route.rs:2233:32 False  [the false side fails the check, and its line ran]  !why.is_empty() && !why.contains("vendor-data"),
(F) BRANCH crates/api/src/feeds_route.rs:2434:17 False  [the false side fails the check, and its line ran]  feed["archive"].is_null() && feed["window"].is_null(),
(F) BRANCH crates/api/src/feeds_route.rs:2434:46 False  [the false side fails the check, and its line ran]  feed["archive"].is_null() && feed["window"].is_null(),
(F) BRANCH crates/api/src/feeds_route.rs:2452:13 False  [the false side fails the check, and its line ran]  tf.secs() > 60 && tf.secs() % 60 == 0 && *tf != store::path::Timeframe::DAY_1
(F) BRANCH crates/api/src/feeds_route.rs:2452:31 False  [the false side fails the check, and its line ran]  tf.secs() > 60 && tf.secs() % 60 == 0 && *tf != store::path::Timeframe::DAY_1
(F) BRANCH crates/api/src/feeds_route.rs:2487:17 False  [the false side fails the check, and its line ran]  assert!(!ready && why.contains("has no census row"), "{why}");
(F) BRANCH crates/api/src/feeds_route.rs:2487:27 False  [the false side fails the check, and its line ran]  assert!(!ready && why.contains("has no census row"), "{why}");
(F) BRANCH crates/api/src/feeds_route.rs:2775:13 False  [the false side fails the check, and its line ran]  why.contains("2099-01-01") && why.contains("2026-10-15"),
(F) BRANCH crates/api/src/feeds_route.rs:2775:43 False  [the false side fails the check, and its line ran]  why.contains("2099-01-01") && why.contains("2026-10-15"),
(F) BRANCH crates/api/src/feeds_route.rs:2829:13 False  [the false side fails the check, and its line ran]  source.contains("2099-01-01") && source.contains("passed over"),
(F) BRANCH crates/api/src/feeds_route.rs:2829:46 False  [the false side fails the check, and its line ran]  source.contains("2099-01-01") && source.contains("passed over"),
(F) LINE crates/api/src/feeds_route.rs:3233  [a message argument of the statement at 3229, which ran: evaluated only when it fails]  3 * reps
(F) BRANCH crates/api/src/feeds_route.rs:3565:36 False  [the false side fails the check, and its line ran]  .is_some_and(|why| why.contains("cannot be read") && why.contains(hole)),
(F) LINE crates/api/src/feeds_route.rs:3591  [a message argument of the statement at 3588, which ran: evaluated only when it fails]  feed.display()
(F) BRANCH crates/api/src/feeds_route.rs:3595:42 False  [the false side fails the check, and its line ran]  feed.serves(rung) || feed.descriptor().history_row(rung).is_none(),
(F) LINE crates/api/src/feeds_route.rs:3597  [a message argument of the statement at 3594, which ran: evaluated only when it fails]  feed.display(),
(F) LINE crates/api/src/feeds_route.rs:3598  [a message argument of the statement at 3594, which ran: evaluated only when it fails]  rung.dir()
(F) BRANCH crates/api/src/feeds_route.rs:3607:32 False  [the false side fails the check, and its line ran]  || spec.fno.by_token().is_some(),
(F) LINE crates/api/src/feeds_route.rs:3609  [a message argument of the statement at 3604, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/feeds_route.rs:3618  [a message argument of the statement at 3614, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/feeds_route.rs:3626  [a message argument of the statement at 3623, which ran: evaluated only when it fails]  feed.display()
(F) LINE crates/api/src/feeds_route.rs:3638  [a message argument of the statement at 3635, which ran: evaluated only when it fails]  target.as_str()
(F) BRANCH crates/api/src/frontierjson.rs:1085:13 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/frontierjson.rs:1085:32 False  [the false side fails the check, and its line ran]  receipt < child && child < reconcile,
(F) BRANCH crates/api/src/frontierjson.rs:1275:13 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":2"#) && !body.contains(r#""rank":1"#),
(F) BRANCH crates/api/src/frontierjson.rs:1275:45 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":2"#) && !body.contains(r#""rank":1"#),
(F) BRANCH crates/api/src/frontierjson.rs:1284:17 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":1"#) && body.contains(r#""rank":2"#),
(F) BRANCH crates/api/src/frontierjson.rs:1284:49 False  [the false side fails the check, and its line ran]  body.contains(r#""rank":1"#) && body.contains(r#""rank":2"#),
(F) LINE crates/api/src/frontierjson.rs:1620  [a message argument of the statement at 1616, which ran: evaluated only when it fails]  root.display()
(F) BRANCH crates/api/src/frontierjson.rs:1623:17 False  [the false side fails the check, and its line ran]  body.contains(&format!("{why}. "))
(F) BRANCH crates/api/src/frontierjson.rs:1624:24 False  [the false side fails the check, and its line ran]  && body.contains("could not be checked for absence: "),
(F) LINE crates/api/src/frontierjson.rs:1626  [a message argument of the statement at 1622, which ran: evaluated only when it fails]  root.display()
(F) LINE crates/api/src/gaps_route.rs:2636  [panic/unreachable line]  panic!("a reader gone first: no month is read")
(F) BRANCH crates/api/src/gaps_route.rs:3161:13 False  [the false side fails the check, and its line ran]  three.contains(r#""months":3"#) && three.contains(r#""months_absent":2"#),
(F) BRANCH crates/api/src/gaps_route.rs:3161:48 False  [the false side fails the check, and its line ran]  three.contains(r#""months":3"#) && three.contains(r#""months_absent":2"#),
(F) BRANCH crates/api/src/gaps_route.rs:3179:13 False  [the false side fails the check, and its line ran]  one.contains(r#""months":1"#) && one.contains(r#""months_absent":0"#),
(F) BRANCH crates/api/src/gaps_route.rs:3179:46 False  [the false side fails the check, and its line ran]  one.contains(r#""months":1"#) && one.contains(r#""months_absent":0"#),
(F) BRANCH crates/api/src/gaps_route.rs:3203:13 False  [the false side fails the check, and its line ran]  typo.contains("YYYY-MM") && typo.contains("to"),
(F) BRANCH crates/api/src/gaps_route.rs:3203:41 False  [the false side fails the check, and its line ran]  typo.contains("YYYY-MM") && typo.contains("to"),
(F) BRANCH crates/api/src/gaps_route.rs:3241:13 False  [the false side fails the check, and its line ran]  three.contains(r#""source":"record""#) && three.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/gaps_route.rs:3241:55 False  [the false side fails the check, and its line ran]  three.contains(r#""source":"record""#) && three.contains(r#""voted_by":[]"#),
(F) BRANCH crates/api/src/gaps_route.rs:3475:13 False  [the false side fails the check, and its line ran]  reasons.contains("no-print") && reasons.contains("expired"),
(F) BRANCH crates/api/src/gaps_route.rs:3475:45 False  [the false side fails the check, and its line ran]  reasons.contains("no-print") && reasons.contains("expired"),
(F) BRANCH crates/api/src/gaps_route.rs:3657:37 False  [the false side fails the check, and its line ran]  .is_some_and(|from| from.contains("import journal") && from.contains("indices")),
(F) BRANCH crates/api/src/gaps_route.rs:3666:28 False  [the false side fails the check, and its line ran]  .all(|run| run["reason"] == "not-imported" && run["minutes"] == 375),
(F) BRANCH crates/api/src/gaps_route.rs:3742:37 False  [the false side fails the check, and its line ran]  .is_some_and(|from| from.contains("2099-01-01") && from.contains("passed over")),
(F) BRANCH crates/api/src/gaps_route.rs:3797:21 False  [the false side fails the check, and its line ran]  assert!(reach.not_yet(day(16)) && !reach.not_imported(day(16)));
(F) BRANCH crates/api/src/gaps_route.rs:3797:47 False  [the false side fails the check, and its line ran]  assert!(reach.not_yet(day(16)) && !reach.not_imported(day(16)));
(F) BRANCH crates/api/src/gaps_route.rs:4014:13 False  [the false side fails the check, and its line ran]  why.contains("NSE/INDEX") && why.contains("NSE/CASH"),
(F) BRANCH crates/api/src/gaps_route.rs:4014:42 False  [the false side fails the check, and its line ran]  why.contains("NSE/INDEX") && why.contains("NSE/CASH"),
(F) LINE crates/api/src/gaps_route.rs:4463  [a message argument of the statement at 4459, which ran: evaluated only when it fails]  iso(*halt)
(F) BRANCH crates/api/src/gaps_route.rs:4687:17 False  [the false side fails the check, and its line ran]  !faults.is_empty()
(F) BRANCH crates/api/src/gaps_route.rs:4688:24 False  [the false side fails the check, and its line ran]  && !faults
(F) BRANCH crates/api/src/gaps_route.rs:5026:59 False  [the false side fails the check, and its line ran]  pull::calendar::DayKind::Open(session) if session.expects(555)
(F) BRANCH crates/api/src/gaps_route.rs:5296:13 False  [the false side fails the check, and its line ran]  body.contains(&crate::calendar_of::build::MAX_BUILDS.to_string())
(F) BRANCH crates/api/src/gaps_route.rs:5297:20 False  [the false side fails the check, and its line ran]  && body.contains("calendar=record"),
(F) BRANCH crates/api/src/gaps_route.rs:5470:17 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = answer else {
(F) LINE crates/api/src/gaps_route.rs:5471  [panic/unreachable line]  panic!("a vote with no build behind it answered");
(F) BRANCH crates/api/src/gaps_route.rs:5489:13 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(queued) = builds.votes.join(&key, Some(&now), None) else {
(F) LINE crates/api/src/gaps_route.rs:5490  [panic/unreachable line]  panic!("the first join leads");
(F) BRANCH crates/api/src/gaps_route.rs:5500:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) LINE crates/api/src/gaps_route.rs:5501  [panic/unreachable line]  panic!("a parked refusal is handed out as it is");
(F) BRANCH crates/api/src/gaps_route.rs:5508:17 False  [the else of a let-else that panics, whose line ran]  let Joined::Lead(build) = builds.votes.join(&key, Some(&now), None) else {
(F) LINE crates/api/src/gaps_route.rs:5509  [panic/unreachable line]  panic!("room for build {other}");
(F) BRANCH crates/api/src/gaps_route.rs:5513:13 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) LINE crates/api/src/gaps_route.rs:5514  [panic/unreachable line]  panic!("a full build map admits no vote");
(F) BRANCH crates/api/src/gaps_route.rs:5562:17 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) BRANCH crates/api/src/gaps_route.rs:5562:44 False  [the false side fails the check, and its line ran]  output.status.success() && stdout.contains("1 passed"),
(F) LINE crates/api/src/gaps_route.rs:5564  [a message argument of the statement at 5561, which ran: evaluated only when it fails]  String::from_utf8_lossy(&output.stderr)
(F) BRANCH crates/api/src/gaps_route.rs:5623:17 False  [the else of a let-else that panics, whose line ran]  let Err((status, _, body)) = voted(&site, &asked).await else {
(F) LINE crates/api/src/gaps_route.rs:5624  [panic/unreachable line]  panic!("a vote whose reading was not admitted answered");
(F) LINE crates/api/src/http_admission_tests.rs:686  [a message argument of the statement at 683, which ran: evaluated only when it fails]  String::from_utf8_lossy(&output.stderr)
(F) BRANCH crates/api/src/imports.rs:2088:17 False  [the false side fails the check, and its line ran]  assert!(why.contains("bogus") && why.contains("reasn"), "{why}");
(F) BRANCH crates/api/src/imports.rs:2088:42 False  [the false side fails the check, and its line ran]  assert!(why.contains("bogus") && why.contains("reasn"), "{why}");
(F) LINE crates/api/src/imports.rs:2563  [a message argument of the statement at 2559, which ran: evaluated only when it fails]  ids(&ends_inside)
(F) BRANCH crates/api/src/indexstoplaunch_tests.rs:600:13 False  [the false side fails the check, and its line ran]  start.elapsed() <= std::time::Duration::from_secs(45)
(F) BRANCH crates/api/src/indexstoplaunch_tests.rs:601:20 False  [the false side fails the check, and its line ran]  && std::fs::metadata(&log).unwrap().len() <= 1024 * 1024,
(F) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:465:13 False  [the false side fails the check, and its line ran]  elsewhere.contains("Native qualification") && elsewhere.contains("another-root"),
(F) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:465:59 False  [the false side fails the check, and its line ran]  elsewhere.contains("Native qualification") && elsewhere.contains("another-root"),
(F) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:602:13 False  [the false side fails the check, and its line ran]  elsewhere.starts_with("original_context_refused: ") && elsewhere.contains("(os error 2)"),
(F) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:602:68 False  [the false side fails the check, and its line ran]  elsewhere.starts_with("original_context_refused: ") && elsewhere.contains("(os error 2)"),
(F) LINE crates/api/src/ingest.rs:2518  [the no-match arm of the matches! that is the whole check (llvm-cov maps its _ => false to this line): the side that fails it]  matches!(
(F) BRANCH crates/api/src/ingest.rs:2623:13 False  [the false side fails the check, and its line ran]  text.contains("2026-08-08") && text.contains("2026-08-07"),
(F) BRANCH crates/api/src/ingest.rs:2623:44 False  [the false side fails the check, and its line ran]  text.contains("2026-08-08") && text.contains("2026-08-07"),
(F) BRANCH crates/api/src/ingest.rs:2732:13 False  [the false side fails the check, and its line ran]  text.contains("expiry") && text.contains("2024-2-9"),
(F) BRANCH crates/api/src/ingest.rs:2732:40 False  [the false side fails the check, and its line ran]  text.contains("expiry") && text.contains("2024-2-9"),
(F) BRANCH crates/api/src/ingest.rs:2770:13 False  [the false side fails the check, and its line ran]  text.contains("2022-02-08") && text.contains("2022-01-08"),
(F) BRANCH crates/api/src/ingest.rs:2770:44 False  [the false side fails the check, and its line ran]  text.contains("2022-02-08") && text.contains("2022-01-08"),
(F) BRANCH crates/api/src/ingest.rs:2816:17 False  [the false side fails the check, and its line ran]  assert!(text.contains("3654") && text.contains("3653"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:2816:42 False  [the false side fails the check, and its line ran]  assert!(text.contains("3654") && text.contains("3653"), "{text}");
(F) LINE crates/api/src/ingest.rs:2832  [a message argument of the statement at 2829, which ran: evaluated only when it fails]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2833  [a message argument of the statement at 2829, which ran: evaluated only when it fails]  target.note()
(F) LINE crates/api/src/ingest.rs:2840  [a message argument of the statement at 2837, which ran: evaluated only when it fails]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2841  [a message argument of the statement at 2837, which ran: evaluated only when it fails]  target.note()
(F) LINE crates/api/src/ingest.rs:2847  [a message argument of the statement at 2844, which ran: evaluated only when it fails]  target.slug(),
(F) LINE crates/api/src/ingest.rs:2848  [a message argument of the statement at 2844, which ran: evaluated only when it fails]  target.note()
(F) LINE crates/api/src/ingest.rs:3110  [a message argument of the statement at 3105, which ran: evaluated only when it fails]  target.slug()
(F) LINE crates/api/src/ingest.rs:3145  [a message argument of the statement at 3142, which ran: evaluated only when it fails]  tier.slug()
(F) LINE crates/api/src/ingest.rs:3190  [a message argument of the statement at 3183, which ran: evaluated only when it fails]  target.slug()
(F) LINE crates/api/src/ingest.rs:3403  [a message argument of the statement at 3400, which ran: evaluated only when it fails]  target.slug()
(F) BRANCH crates/api/src/ingest.rs:3910:21 False  [the false side fails the check, and its line ran]  assert!(text.contains(live) && text.contains("2026-08-07"), "{text}");
(F) BRANCH crates/api/src/ingest.rs:3910:44 False  [the false side fails the check, and its line ran]  assert!(text.contains(live) && text.contains("2026-08-07"), "{text}");
(F) LINE crates/api/src/ingest.rs:3928  [the no-match arm of the matches! that is the whole check (llvm-cov maps its _ => false to this line): the side that fails it]  matches!(
(F) LINE crates/api/src/ingest.rs:4584  [a message argument of the statement at 4580, which ran: evaluated only when it fails]  root.display()
(F) BRANCH crates/api/src/ingest.rs:4941:13 False  [the false side fails the check, and its line ran]  idle["in_flight"].is_null() && idle["run_target"].is_null(),
(F) BRANCH crates/api/src/ingest.rs:4941:44 False  [the false side fails the check, and its line ran]  idle["in_flight"].is_null() && idle["run_target"].is_null(),
(F) BRANCH crates/api/src/ingest_cover_tests.rs:118:13 False  [the else of a let-else that panics, whose line ran]  let Refusal::UnreadableRate { ref got } = refused else {
(F) LINE crates/api/src/ingest_cover_tests.rs:119  [panic/unreachable line]  panic!("{typed}: {refused:?}");
(F) BRANCH crates/api/src/ingest_cover_tests.rs:132:9 False  [the else of a let-else that panics, whose line ran]  let Refusal::UnreadableRate { ref got } = refused else {
(F) LINE crates/api/src/ingest_cover_tests.rs:133  [panic/unreachable line]  panic!("{refused:?}");
(F) BRANCH crates/api/src/ingest_cover_tests.rs:165:9 False  [the else of a let-else that panics, whose line ran]  let Refusal::ClockUnusable { .. } = refused else {
(F) LINE crates/api/src/ingest_cover_tests.rs:166  [panic/unreachable line]  panic!("{refused:?}");
(F) BRANCH crates/api/src/ingest_cover_tests.rs:180:9 False  [the else of a let-else that panics, whose line ran]  let Refusal::ClockUnusable { .. } = named else {
(F) LINE crates/api/src/ingest_cover_tests.rs:181  [panic/unreachable line]  panic!("{named:?}");
(F) BRANCH crates/api/src/ingest_cover_tests.rs:188:9 False  [the else of a let-else that panics, whose line ran]  let Refusal::ClockUnusable { .. } = first else {
(F) LINE crates/api/src/ingest_cover_tests.rs:189  [panic/unreachable line]  panic!("{first:?}");
(F) BRANCH crates/api/src/ingest_cover_tests.rs:231:9 False  [the else of a let-else that panics, whose line ran]  let Refusal::DateImpossible { field, ref got, .. } = zeros else {
(F) LINE crates/api/src/ingest_cover_tests.rs:232  [panic/unreachable line]  panic!("{zeros:?}");
(F) LINE crates/api/src/ladder.rs:1077  [the no-match arm of the matches! that is the whole check (llvm-cov maps its _ => false to this line): the side that fails it]  matches!(
(F) LINE crates/api/src/ladder.rs:1092  [the no-match arm of the matches! that is the whole check (llvm-cov maps its _ => false to this line): the side that fails it]  matches!(
(F) LINE crates/api/src/ladder.rs:1297  [a message argument of the statement at 1293, which ran: evaluated only when it fails]  tf.as_str()
(F) LINE crates/api/src/ladder.rs:1310  [a message argument of the statement at 1307, which ran: evaluated only when it fails]  tf.as_str()
(F) BRANCH crates/api/src/logs.rs:6746:17 False  [the false side fails the check, and its line ran]  assert!(with("cross-site") && with("same-site"));
(F) BRANCH crates/api/src/logs.rs:6746:39 False  [the false side fails the check, and its line ran]  assert!(with("cross-site") && with("same-site"));
(F) BRANCH crates/api/src/logs.rs:6747:17 False  [the false side fails the check, and its line ran]  assert!(!with("same-origin") && !with("none"));
(F) BRANCH crates/api/src/logs.rs:6747:41 False  [the false side fails the check, and its line ran]  assert!(!with("same-origin") && !with("none"));
(F) BRANCH crates/api/src/logs.rs:6890:13 False  [the false side fails the check, and its line ran]  json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
(F) BRANCH crates/api/src/logs.rs:6890:57 False  [the false side fails the check, and its line ran]  json.contains(r#""hit_scan_cap":false"#) && json.contains(r#""malformed":0"#),
(F) BRANCH crates/api/src/logs.rs:7203:13 False  [the false side fails the check, and its line ran]  all.contains("landed") && all.contains("did not land"),
(F) BRANCH crates/api/src/logs.rs:7203:39 False  [the false side fails the check, and its line ran]  all.contains("landed") && all.contains("did not land"),
(F) BRANCH crates/api/src/logs.rs:7328:13 False  [the false side fails the check, and its line ran]  plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
(F) BRANCH crates/api/src/logs.rs:7328:57 False  [the false side fails the check, and its line ran]  plain.contains("BRUTEX_LOG_LEVEL=debug") && !plain.contains("the filters in force"),
(F) LINE crates/api/src/logs.rs:7371  [a message argument of the statement at 7367, which ran: evaluated only when it fails]  note_of(&after_restart)
(F) LINE crates/api/src/logs.rs:7376  [a message argument of the statement at 7373, which ran: evaluated only when it fails]  note_of(&after_restart)
(F) LINE crates/api/src/logs.rs:7385  [a message argument of the statement at 7382, which ran: evaluated only when it fails]  note_of(&same_process)
(F) LINE crates/api/src/logs.rs:7395  [a message argument of the statement at 7392, which ran: evaluated only when it fails]  note_of(&rolled)
(F) LINE crates/api/src/logs.rs:7403  [a message argument of the statement at 7400, which ran: evaluated only when it fails]  note_of(&none)
(F) BRANCH crates/api/src/logs.rs:7442:13 False  [the false side fails the check, and its line ran]  page.contains("clock is held") && page.contains("3 event(s)"),
(F) BRANCH crates/api/src/logs.rs:7442:47 False  [the false side fails the check, and its line ran]  page.contains("clock is held") && page.contains("3 event(s)"),
(F) BRANCH crates/api/src/logs.rs:7448:13 False  [the false side fails the check, and its line ran]  json.contains(r#""clock_held":3"#) && json.contains(r#""loud":true"#),
(F) BRANCH crates/api/src/logs.rs:7448:51 False  [the false side fails the check, and its line ran]  json.contains(r#""clock_held":3"#) && json.contains(r#""loud":true"#),
(F) BRANCH crates/api/src/logs.rs:7525:13 False  [the false side fails the check, and its line ran]  page.contains("halt") && page.contains("2 roll(s) failed"),
(F) BRANCH crates/api/src/logs.rs:7525:38 False  [the false side fails the check, and its line ran]  page.contains("halt") && page.contains("2 roll(s) failed"),
(F) BRANCH crates/api/src/logs.rs:7542:13 False  [the false side fails the check, and its line ran]  none.contains("No sink") && none.contains("older run"),
(F) BRANCH crates/api/src/logs.rs:7542:41 False  [the false side fails the check, and its line ran]  none.contains("No sink") && none.contains("older run"),
(F) BRANCH crates/api/src/logs.rs:7864:13 False  [the false side fails the check, and its line ran]  both.contains("<code>/tmp/served</code>")
(F) BRANCH crates/api/src/logs.rs:7865:20 False  [the false side fails the check, and its line ran]  && both.contains("<code>/tmp/served/cli</code>"),
(F) BRANCH crates/api/src/logs.rs:8040:13 False  [the false side fails the check, and its line ran]  body.starts_with(r#"{"records":["#) && body.ends_with('}'),
(F) BRANCH crates/api/src/logs.rs:8040:52 False  [the false side fails the check, and its line ran]  body.starts_with(r#"{"records":["#) && body.ends_with('}'),
(F) BRANCH crates/api/src/logs.rs:8774:17 False  [the false side fails the check, and its line ran]  assert!(is_db_read("/bars.json") && !is_db_read("/pull/spot"));
(F) BRANCH crates/api/src/logs.rs:8774:45 False  [the false side fails the check, and its line ran]  assert!(is_db_read("/bars.json") && !is_db_read("/pull/spot"));
(F) LINE crates/api/src/logs.rs:9076  [a message argument of the statement at 9073, which ran: evaluated only when it fails]  dir.name()
(F) LINE crates/api/src/logs.rs:9092  [a message argument of the statement at 9089, which ran: evaluated only when it fails]  kind.as_str(),
(F) LINE crates/api/src/logs.rs:9100  [a message argument of the statement at 9095, which ran: evaluated only when it fails]  written.name()
(F) LINE crates/api/src/logs.rs:9123  [a message argument of the statement at 9118, which ran: evaluated only when it fails]  dir.display()
(F) BRANCH crates/api/src/logs.rs:9173:17 False  [the else of a let-else that panics, whose line ran]  let DirState::Unreadable(why) = state else {
(F) LINE crates/api/src/logs.rs:9174  [panic/unreachable line]  panic!("{half} reads {state:?}, not unreadable");
(F) BRANCH crates/api/src/logs.rs:9222:21 False  [the false side fails the check, and its line ran]  said.starts_with(&format!("{dir} unreadable: "))
(F) BRANCH crates/api/src/logs.rs:9223:28 False  [the false side fails the check, and its line ran]  && said.contains("Permission denied"),
(F) LINE crates/api/src/logs.rs:9371  [a message argument of the statement at 9368, which ran: evaluated only when it fails]  first.errors
(F) BRANCH crates/api/src/logs.rs:9513:28 False  [the false side fails the check, and its line ran]  .all(|row| row["half"] == "gdfl/indices" && row["source"] == "audit"),
(F) BRANCH crates/api/src/logs.rs:10061:17 False  [the false side fails the check, and its line ran]  got.ignored.contains(&(bad.0, bad.1.to_owned()))
(F) BRANCH crates/api/src/logs.rs:10062:24 False  [the false side fails the check, and its line ran]  && got.ignored.contains(&(good, said.to_owned())),
(F) LINE crates/api/src/logs.rs:10142  [a message argument of the statement at 10138, which ran: evaluated only when it fails]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10149  [a message argument of the statement at 10146, which ran: evaluated only when it fails]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10237  [a message argument of the statement at 10233, which ran: evaluated only when it fails]  got["keyed"]
(F) LINE crates/api/src/logs.rs:10280  [a message argument of the statement at 10273, which ran: evaluated only when it fails]  none["keyed"]
(F) BRANCH crates/api/src/logs.rs:10802:17 False  [the false side fails the check, and its line ran]  assert!(none["audit_stream"].is_null() && none["state_stream"].is_null());
(F) BRANCH crates/api/src/logs.rs:10802:51 False  [the false side fails the check, and its line ran]  assert!(none["audit_stream"].is_null() && none["state_stream"].is_null());
(F) LINE crates/api/src/logs.rs:10879  [a message argument of the statement at 10876, which ran: evaluated only when it fails]  held.bytes - read
(F) LINE crates/api/src/logs.rs:10927  [a message argument of the statement at 10924, which ran: evaluated only when it fails]  now.unread("options")
(F) LINE crates/api/src/logs.rs:11031  [a message argument of the statement at 11028, which ran: evaluated only when it fails]  held.malformed
(F) BRANCH crates/api/src/logs.rs:11262:17 False  [the false side fails the check, and its line ran]  assert!(none.pointers.is_empty() && !more);
(F) BRANCH crates/api/src/logs.rs:11262:45 False  [the false side fails the check, and its line ran]  assert!(none.pointers.is_empty() && !more);
(F) BRANCH crates/api/src/logs.rs:11281:56 False  [the false side fails the check, and its line ran]  matches!(spent, Ok(Walked::Stopped(at)) if at == len),
(F) BRANCH crates/api/src/logs.rs:11293:57 False  [the false side fails the check, and its line ran]  matches!(walked, Ok(Walked::Stopped(at)) if at == first),
(F) BRANCH crates/api/src/logs.rs:11302:59 False  [the false side fails the check, and its line ran]  assert!(matches!(torn, Ok(Walked::Stopped(at)) if at == len - 1));
(F) BRANCH crates/api/src/logs.rs:11304:17 False  [the false side fails the check, and its line ran]  assert!(out.page.pointers.is_empty() && out.damage.is_empty());
(F) BRANCH crates/api/src/logs.rs:11304:49 False  [the false side fails the check, and its line ran]  assert!(out.page.pointers.is_empty() && out.damage.is_empty());
(F) BRANCH crates/api/src/logs.rs:11310:59 False  [the false side fails the check, and its line ran]  assert!(matches!(full, Ok(Walked::Stopped(at)) if at == len));
(F) BRANCH crates/api/src/logs.rs:11465:13 False  [the false side fails the check, and its line ran]  probed.contains(&"day") && !probed.contains(&"span"),
(F) BRANCH crates/api/src/logs.rs:11465:40 False  [the false side fails the check, and its line ran]  probed.contains(&"day") && !probed.contains(&"span"),
(F) LINE crates/api/src/logs.rs:11479  [a message argument of the statement at 11476, which ran: evaluated only when it fails]  again["keyed"]
(F) LINE crates/api/src/logs.rs:11537  [a message argument of the statement at 11533, which ran: evaluated only when it fails]  old["keyed"]
(F) LINE crates/api/src/logs.rs:11676  [a message argument of the statement at 11672, which ran: evaluated only when it fails]  second["keyed"]
(F) LINE crates/api/src/logs.rs:11741  [a message argument of the statement at 11737, which ran: evaluated only when it fails]  second["keyed"]
(F) LINE crates/api/src/logs.rs:11748  [a message argument of the statement at 11745, which ran: evaluated only when it fails]  second["keyed"]["index"]
(F) BRANCH crates/api/src/logs.rs:11894:17 False  [the false side fails the check, and its line ran]  assert!(read > 0 && read <= 1_000, "{read}");
(F) BRANCH crates/api/src/logs.rs:11894:29 False  [the false side fails the check, and its line ran]  assert!(read > 0 && read <= 1_000, "{read}");
(F) BRANCH crates/api/src/logs.rs:11922:20 False  [the false side fails the check, and its line ran]  if why.contains(&rolled.display().to_string())),
(F) BRANCH crates/api/src/logs.rs:12117:13 False  [the false side fails the check, and its line ran]  why.contains("damaged.ndjson") && why.contains("undecodable lines"),
(F) BRANCH crates/api/src/logs.rs:12117:47 False  [the false side fails the check, and its line ran]  why.contains("damaged.ndjson") && why.contains("undecodable lines"),
(F) BRANCH crates/api/src/logs.rs:12201:40 False  [the false side fails the check, and its line ran]  ms > to || start < cut.at,
(F) BRANCH crates/api/src/logs.rs:12227:13 False  [the false side fails the check, and its line ran]  exact > 0 && around > 0,
(F) BRANCH crates/api/src/logs.rs:12227:26 False  [the false side fails the check, and its line ran]  exact > 0 && around > 0,
(F) LINE crates/api/src/logs.rs:12287  [a message argument of the statement at 12284, which ran: evaluated only when it fails]  read_words(&indexed)
(F) BRANCH crates/api/src/logs_cover_tests.rs:747:9 False  [the else of a let-else that panics, whose line ran]  let &[one, two, ..] = halves.as_slice() else {
(F) LINE crates/api/src/logs_cover_tests.rs:748  [panic/unreachable line]  panic!("a run of options can write two directories: {halves:?}")
(F) LINE crates/api/src/logs_cover_tests.rs:902  [a message argument of the statement at 899, which ran: evaluated only when it fails]  String::from_utf8_lossy(&reply)
(F) BRANCH crates/api/src/logs_units_tests.rs:77:17 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:77:44 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:77:67 False  [the false side fails the check, and its line ran]  keys.contract.is_none() && keys.code.is_none() && keys.reason.is_none(),
(F) BRANCH crates/api/src/logs_units_tests.rs:89:13 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:89:37 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:89:60 False  [the false side fails the check, and its line ran]  assert!(keys.month.is_none() && keys.code.is_none() && keys.contract.is_none());
(F) BRANCH crates/api/src/logs_units_tests.rs:107:13 False  [the false side fails the check, and its line ran]  assert!(tail.records.is_empty() && tail.errors.is_empty());
(F) BRANCH crates/api/src/logs_units_tests.rs:107:40 False  [the false side fails the check, and its line ran]  assert!(tail.records.is_empty() && tail.errors.is_empty());
(F) BRANCH crates/api/src/logs_units_tests.rs:251:9 False  [the else of a let-else that panics, whose line ran]  let DirState::Unreadable(why) = state else {
(F) LINE crates/api/src/logs_units_tests.rs:252  [panic/unreachable line]  panic!("a file is no log directory: {word}");
(F) BRANCH crates/api/src/mastersrun.rs:1184:13 False  [the else of a let-else that panics, whose line ran]  let [record] = summary.as_slice() else {
(F) LINE crates/api/src/mastersrun.rs:1185  [panic/unreachable line]  panic!("{mode}: one summary record: {summary:?}");
(F) BRANCH crates/api/src/mastersrun.rs:1385:13 False  [the else of a let-else that panics, whose line ran]  let masters::Got::Refused {
(F) LINE crates/api/src/mastersrun.rs:1389  [panic/unreachable line]  panic!("the primary attempt is a refusal: {tried:?}");
(F) BRANCH crates/api/src/mastersrun.rs:1913:13 False  [the else of a let-else that panics, whose line ran]  let Landed::Refused(ref why) = landed else {
(F) LINE crates/api/src/mastersrun.rs:1914  [panic/unreachable line]  panic!("nothing answered, so nothing can be written: {landed:?}");
(F) BRANCH crates/api/src/mastersrun.rs:2193:13 False  [the false side fails the check, and its line ran]  credentialed.contains("Landed::Refused(why.clone())")
(F) BRANCH crates/api/src/mastersrun.rs:2194:20 False  [the false side fails the check, and its line ran]  && credentialed.contains("Fetched::default()"),
(F) BRANCH crates/api/src/mastersrun.rs:2488:17 False  [the else of a let-else that panics, whose line ran]  let Err(ref why) = refused else {
(F) LINE crates/api/src/mastersrun.rs:2489  [panic/unreachable line]  panic!("{}: {refused:?}", folder.display());
(F) BRANCH crates/api/src/month_change.rs:534:13 False  [the false side fails the check, and its line ran]  !can_be_rebased(Segment::Index, "NIFTY")
(F) BRANCH crates/api/src/month_change.rs:535:20 False  [the false side fails the check, and its line ran]  && can_be_rebased(Segment::Cash, "RELIANCE")
(F) BRANCH crates/api/src/month_change.rs:536:20 False  [the false side fails the check, and its line ran]  && can_be_rebased(Segment::Fno, "RELIANCE")
(F) BRANCH crates/api/src/nse_identity.rs:490:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::FileUnavailable(why)) = absent.confirm(&merged, "RELIANCE") else {
(F) LINE crates/api/src/nse_identity.rs:491  [panic/unreachable line]  panic!("an absent file refuses");
(F) BRANCH crates/api/src/nse_identity.rs:503:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::FileUnavailable(why)) = confirmed else {
(F) LINE crates/api/src/nse_identity.rs:504  [panic/unreachable line]  panic!("an unreadable file refuses");
(F) BRANCH crates/api/src/nse_identity.rs:524:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::ZerodhaUndated(why)) = confirmed else {
(F) LINE crates/api/src/nse_identity.rs:525  [panic/unreachable line]  panic!("an undated Zerodha dump refuses");
(F) BRANCH crates/api/src/nse_identity.rs:532:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::FileUnavailable(why)) = confirmed else {
(F) LINE crates/api/src/nse_identity.rs:533  [panic/unreachable line]  panic!("an oversized file refuses");
(F) BRANCH crates/api/src/nse_identity.rs:541:13 False  [the else of a let-else that panics, whose line ran]  let Err(Refusal::FileUnavailable(why)) = confirmed else {
(F) LINE crates/api/src/nse_identity.rs:542  [panic/unreachable line]  panic!("a file at the bound that is not the constituent file refuses");
(F) LINE crates/api/src/ops_safety_tests.rs:82  [panic/unreachable line]  panic!("the store-read pool stayed full for two seconds");
(F) BRANCH crates/api/src/ops_safety_tests.rs:265:9 False  [the false side fails the check, and its line ran]  drift.contains("1128 bytes") && drift.contains("1000"),
(F) BRANCH crates/api/src/ops_safety_tests.rs:265:41 False  [the false side fails the check, and its line ran]  drift.contains("1128 bytes") && drift.contains("1000"),
(F) BRANCH crates/api/src/ops_safety_tests.rs:518:29 False  [the false side fails the check, and its line ran]  !wrote.contains(READ_ONLY_ENV) && !wrote.contains(UNSTAMPED),
(F) BRANCH crates/api/src/ops_safety_tests.rs:518:63 False  [the false side fails the check, and its line ran]  !wrote.contains(READ_ONLY_ENV) && !wrote.contains(UNSTAMPED),
(F) BRANCH crates/api/src/ops_safety_tests.rs:547:13 False  [the false side fails the check, and its line ran]  assert!(why.contains(UNSTAMPED) && why.contains("POST"), "{why}");
(F) BRANCH crates/api/src/ops_safety_tests.rs:547:40 False  [the false side fails the check, and its line ran]  assert!(why.contains(UNSTAMPED) && why.contains("POST"), "{why}");
(F) LINE crates/api/src/ops_safety_tests.rs:618  [a message argument of the statement at 615, which ran: evaluated only when it fails]  most.len()
(F) BRANCH crates/api/src/owed_tests.rs:249:9 False  [the false side fails the check, and its line ran]  evidence.contains("CML58560") && evidence.contains("CML60618"),
(F) BRANCH crates/api/src/owed_tests.rs:249:42 False  [the false side fails the check, and its line ran]  evidence.contains("CML58560") && evidence.contains("CML60618"),
(F) BRANCH crates/api/src/owed_tests.rs:372:25 False  [the false side fails the check, and its line ran]  .all(|word| word != "no-print" && word != "vendor-hole"),
(F) BRANCH crates/api/src/owed_tests.rs:388:9 False  [the else of a let-else that panics, whose line ran]  let Some(top) = answer.as_object_mut() else {
(F) LINE crates/api/src/owed_tests.rs:389  [panic/unreachable line]  panic!("not an object: {body}")
(F) BRANCH crates/api/src/owed_tests.rs:393:13 False  [the else of a let-else that panics, whose line ran]  let Some(row) = month.as_object_mut() else {
(F) LINE crates/api/src/owed_tests.rs:394  [panic/unreachable line]  panic!("a month row: {body}")
(F) LINE crates/api/src/pool_table_tests.rs:750  [a message argument of the statement at 746, which ran: evaluated only when it fails]  named.pool().name()
(F) LINE crates/api/src/pool_table_tests.rs:1151  [a message argument of the statement at 1148, which ran: evaluated only when it fails]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1181  [a message argument of the statement at 1178, which ran: evaluated only when it fails]  json.len()
(F) LINE crates/api/src/pool_table_tests.rs:1238  [a message argument of the statement at 1235, which ran: evaluated only when it fails]  whole.len()
(F) BRANCH crates/api/src/pool_table_tests.rs:1297:24 False  [the false side fails the check, and its line ran]  .all(|day| day["held"]["why"] == "being_written"
(F) LINE crates/api/src/pool_table_tests.rs:1300  [a message argument of the statement at 1295, which ran: evaluated only when it fails]  body["days"]
(F) LINE crates/api/src/pool_table_tests.rs:1307  [a message argument of the statement at 1303, which ran: evaluated only when it fails]  body["month_faults"]
(F) BRANCH crates/api/src/pool_table_tests.rs:1331:17 False  [the false side fails the check, and its line ran]  !answer.contains("being written") && !answer.contains("\"unreadable\""),
(F) BRANCH crates/api/src/pool_table_tests.rs:1331:54 False  [the false side fails the check, and its line ran]  !answer.contains("being written") && !answer.contains("\"unreadable\""),
(F) LINE crates/api/src/pool_table_tests.rs:1333  [a message argument of the statement at 1330, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1374  [a message argument of the statement at 1369, which ran: evaluated only when it fails]  body["source_faults"]
(F) LINE crates/api/src/pool_table_tests.rs:1380  [a message argument of the statement at 1377, which ran: evaluated only when it fails]  body["days"]
(F) LINE crates/api/src/pool_table_tests.rs:1464  [a message argument of the statement at 1461, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1469  [a message argument of the statement at 1466, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1544  [a message argument of the statement at 1540, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1549  [a message argument of the statement at 1546, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1575  [a message argument of the statement at 1571, which ran: evaluated only when it fails]  head_text(&answer)
(F) LINE crates/api/src/pool_table_tests.rs:1735  [a message argument of the statement at 1731, which ran: evaluated only when it fails]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1740  [a message argument of the statement at 1737, which ran: evaluated only when it fails]  head_text(answer)
(F) LINE crates/api/src/pool_table_tests.rs:1839  [a message argument of the statement at 1836, which ran: evaluated only when it fails]  body.len()
(F) LINE crates/api/src/pool_table_tests.rs:2085  [a message argument of the statement at 2082, which ran: evaluated only when it fails]  head_text(&answer)
(F) BRANCH crates/api/src/pool_table_tests.rs:2104:13 False  [the false side fails the check, and its line ran]  say.contains("A pool's answered is work it ran to its end")
(F) BRANCH crates/api/src/pool_table_tests.rs:2105:20 False  [the false side fails the check, and its line ran]  && say.contains("the two answered columns count different things"),
(F) BRANCH crates/api/src/pool_table_tests.rs:2118:43 False  [the false side fails the check, and its line ran]  route.starts_with('/') || route == crate::detail::FRONT_END,
(F) BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:210:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(shipped) = feed.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:211  [panic/unreachable line]  panic!("{} is an HTTP feed", feed.display());
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:513  [panic/unreachable line]  panic!("no store rung, so nothing is probed")
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1435  [a message argument of the statement at 1432, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1440  [a message argument of the statement at 1437, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1445  [a message argument of the statement at 1442, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1471  [a message argument of the statement at 1468, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1479  [a message argument of the statement at 1473, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1511  [a message argument of the statement at 1508, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1560  [a message argument of the statement at 1557, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1569  [a message argument of the statement at 1566, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1577  [a message argument of the statement at 1574, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1585  [a message argument of the statement at 1582, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1614  [a message argument of the statement at 1611, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1657  [a message argument of the statement at 1654, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1662  [a message argument of the statement at 1659, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1684  [a message argument of the statement at 1678, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1717  [a message argument of the statement at 1711, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:1760  [a message argument of the statement at 1757, which ran: evaluated only when it fails]  visible(&body)
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2401  [a message argument of the statement at 2398, which ran: evaluated only when it fails]  visible(&body)
(F) BRANCH crates/api/src/server_cloud_mutant_tests_b.rs:2437:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = asked.feed.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2438  [panic/unreachable line]  panic!("the premise: Zerodha is an HTTP feed");
(F) LINE crates/api/src/server_cloud_mutant_tests_b.rs:2454  [a message argument of the statement at 2448, which ran: evaluated only when it fails]  visible(&body)
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:520:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Dhan.descriptor().transport
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:522  [panic/unreachable line]  panic!("Dhan is HTTP");
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1118  [a message argument of the statement at 1115, which ran: evaluated only when it fails]  started.elapsed()
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:1739:9 False  [the else of a let-else that panics, whose line ran]  let census::Census::Held { ref manifest } = census.state else {
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1740  [panic/unreachable line]  panic!("a recovered census still loads: {:?}", census.state.name());
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2430:13 False  [the false side fails the check, and its line ran]  !why.contains(VENDOR_DOWN) && !why.contains(WIRE_REACHED),
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:2430:43 False  [the false side fails the check, and its line ran]  !why.contains(VENDOR_DOWN) && !why.contains(WIRE_REACHED),
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:3280:9 False  [the else of a let-else that panics, whose line ran]  let Err(refused) = refused else {
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:3281  [panic/unreachable line]  panic!("nothing landed before the refusal");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_c.rs:3365:13 False  [the else of a let-else that panics, whose line ran]  let Err(refused) = refused else {
(F) LINE crates/api/src/server_cloud_mutant_tests_c.rs:3366  [panic/unreachable line]  panic!("nothing landed before the refusal");
(F) LINE crates/api/src/server_cloud_mutant_tests_d.rs:103  [a message argument of the statement at 100, which ran: evaluated only when it fails]  answered.map(|schedule| schedule.is_some())
(F) BRANCH crates/api/src/server_cloud_mutant_tests_d.rs:158:9 False  [the else of a let-else that panics, whose line ran]  let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Dhan.descriptor().transport
(F) LINE crates/api/src/server_cloud_mutant_tests_d.rs:160  [panic/unreachable line]  panic!("Dhan is an HTTP feed");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_d.rs:677:9 False  [the else of a let-else that panics, whose line ran]  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_d.rs:678  [panic/unreachable line]  panic!("HTTP descriptor");
(F) LINE crates/api/src/strict_sweep_tests.rs:303  [a message argument of the statement at 300, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/strict_sweep_tests.rs:387  [panic/unreachable line]  panic!("the commit gate refuses before the configuration is read")
(F) LINE crates/api/src/strict_sweep_tests.rs:493  [panic/unreachable line]  panic!("{name}={value} is refused before the configuration is read")
(F) LINE crates/api/src/strict_sweep_tests.rs:538  [panic/unreachable line]  panic!("an invalid server setting is refused before the configuration is read")
(F) LINE crates/api/src/sweeprun.rs:4571  [a message argument of the statement at 4568, which ran: evaluated only when it fails]  String::from_utf8_lossy(&result.stderr)
(F) LINE crates/api/src/sweeprun.rs:4687  [a closure whose only work is to panic: the failing arm]  || panic!("the first admission found the slot busy"),
(F) LINE crates/api/src/sweeprun.rs:4705  [panic/unreachable line]  panic!("a second admission prepared while the first was in flight")
(F) BRANCH crates/api/src/sweeprun.rs:4722:17 False  [the false side fails the check, and its line ran]  assert!(installed.in_flight() && installed.attempt == 1);
(F) BRANCH crates/api/src/sweeprun.rs:4722:42 False  [the false side fails the check, and its line ran]  assert!(installed.in_flight() && installed.attempt == 1);
(F) BRANCH crates/api/src/sweeprun.rs:4771:17 False  [the false side fails the check, and its line ran]  !body.contains("sweep.lock()") && !body.contains(".sweep\n"),
(F) BRANCH crates/api/src/sweeprun.rs:4771:51 False  [the false side fails the check, and its line ran]  !body.contains("sweep.lock()") && !body.contains(".sweep\n"),
(F) LINE crates/api/src/sweeprun.rs:4820  [a message argument of the statement at 4817, which ran: evaluated only when it fails]  poll_text.len()
(F) LINE crates/api/src/sweeprun.rs:5030  [a message argument of the statement at 5027, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5102  [a message argument of the statement at 5099, which ran: evaluated only when it fails]  why.why()
(F) BRANCH crates/api/src/sweeprun.rs:5223:38 False  [the false side fails the check, and its line ran]  .all(|(asked, name)| *asked != "screen_budget_ms"
(F) LINE crates/api/src/sweeprun.rs:5236  [a message argument of the statement at 5233, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5241  [a message argument of the statement at 5238, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5445  [a message argument of the statement at 5441, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5524  [a message argument of the statement at 5520, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:5571  [a message argument of the statement at 5567, which ran: evaluated only when it fails]  why.why()
(F) BRANCH crates/api/src/sweeprun.rs:6532:24 False  [the false side fails the check, and its line ran]  || refusal.why().contains("No engine work was started"),
(F) LINE crates/api/src/sweeprun.rs:6534  [a message argument of the statement at 6530, which ran: evaluated only when it fails]  refusal.why()
(F) LINE crates/api/src/sweeprun.rs:6776  [a message argument of the statement at 6772, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6782  [a message argument of the statement at 6778, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6811  [a message argument of the statement at 6807, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6881  [a message argument of the statement at 6878, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:6887  [a message argument of the statement at 6884, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7030  [a message argument of the statement at 7026, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7035  [a message argument of the statement at 7032, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7069  [a message argument of the statement at 7065, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7105  [a message argument of the statement at 7101, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7141  [a message argument of the statement at 7138, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7191  [a message argument of the statement at 7187, which ran: evaluated only when it fails]  why.why()
(F) LINE crates/api/src/sweeprun.rs:7196  [a message argument of the statement at 7193, which ran: evaluated only when it fails]  why.why()
(F) BRANCH crates/api/src/sweeprun.rs:7628:13 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7628:34 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7628:53 False  [the false side fails the check, and its line ran]  done.launch_clear && !done.uncertain && !done.in_flight,
(F) BRANCH crates/api/src/sweeprun.rs:7642:13 False  [the false side fails the check, and its line ran]  running.in_flight && !running.launch_clear,
(F) BRANCH crates/api/src/sweeprun.rs:7642:34 False  [the false side fails the check, and its line ran]  running.in_flight && !running.launch_clear,
(F) BRANCH crates/api/src/sweeprun.rs:7653:17 False  [the false side fails the check, and its line ran]  assert!(lost.uncertain && !lost.launch_clear, "{}", lost.body);
(F) BRANCH crates/api/src/sweeprun.rs:7653:35 False  [the false side fails the check, and its line ran]  assert!(lost.uncertain && !lost.launch_clear, "{}", lost.body);
(F) BRANCH crates/api/src/sweeprun.rs:7722:17 False  [the false side fails the check, and its line ran]  assert!(clear.launch_clear && !clear.uncertain, "{}", clear.body);
(F) BRANCH crates/api/src/sweeprun.rs:7722:39 False  [the false side fails the check, and its line ran]  assert!(clear.launch_clear && !clear.uncertain, "{}", clear.body);
(F) BRANCH crates/api/src/sweeprun.rs:7811:13 False  [the false side fails the check, and its line ran]  why.starts_with(
(F) BRANCH crates/api/src/sweeprun.rs:7813:18 False  [the false side fails the check, and its line ran]  ) && why.contains("so it is running or was stopped without logging one.")
(F) BRANCH crates/api/src/sweeprun.rs:7814:20 False  [the false side fails the check, and its line ran]  && why.contains(
(F) BRANCH crates/api/src/sweeprun_cover_tests.rs:470:13 False  [the else of a let-else that panics, whose line ran]  let Command::AuditAuditedRange { knobs, .. } = asked else {
(F) LINE crates/api/src/sweeprun_cover_tests.rs:471  [panic/unreachable line]  panic!("{written}: the strict command")
(H) BRANCH crates/api/src/feeds_route.rs:1498:13 True  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  && (from.origin != "generation" || to.origin != "generation")
(H) BRANCH crates/api/src/feeds_route.rs:1511:8 True  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  if first > last {
(H) LINE crates/api/src/feeds_route.rs:1512  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  return refuse(format!(
(H) LINE crates/api/src/feeds_route.rs:1513  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  "the window in force begins {first} ({}) after it ends {last} ({}): refused, not \
(H) LINE crates/api/src/feeds_route.rs:1514  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  served inverted",
(H) LINE crates/api/src/feeds_route.rs:1515  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  from.source, to.source
(H) LINE crates/api/src/feeds_route.rs:1516  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  ));
(H) LINE crates/api/src/feeds_route.rs:2884  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let window = &row(&rows, "zerodha")["window"];
(H) LINE crates/api/src/feeds_route.rs:2885  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(window["from"].is_null(), "{window}");
(H) LINE crates/api/src/feeds_route.rs:2886  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(window["to"].is_null(), "{window}");
(H) LINE crates/api/src/feeds_route.rs:2887  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(
(H) LINE crates/api/src/feeds_route.rs:2888  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  window["why"]
(H) LINE crates/api/src/feeds_route.rs:2889  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .as_str()
(H) LINE crates/api/src/feeds_route.rs:2890  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .is_some_and(|why| why.contains("cannot be read")),
(H) LINE crates/api/src/feeds_route.rs:2893  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(window["degraded"].is_string(), "{window}");
(H) LINE crates/api/src/feeds_route.rs:2897  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let root = store_root("feeds-window-inverted");
(H) LINE crates/api/src/feeds_route.rs:2898  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let mut reader = store::settings::Reader::new(&root);
(H) LINE crates/api/src/feeds_route.rs:2899  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  store::settings::save(
(H) LINE crates/api/src/feeds_route.rs:2900  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  &mut reader,
(H) LINE crates/api/src/feeds_route.rs:2901  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  &store::settings::Change {
(H) LINE crates/api/src/feeds_route.rs:2902  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  expected: 0,
(H) LINE crates/api/src/feeds_route.rs:2903  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  set: vec![(store::settings::Key::WindowFrom, "2026-10-01".to_owned())],
(H) LINE crates/api/src/feeds_route.rs:2904  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  unset: Vec::new(),
(H) LINE crates/api/src/feeds_route.rs:2905  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  changed_by: "an older build".to_owned(),
(H) LINE crates/api/src/feeds_route.rs:2906  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  reason: "narrow to October".to_owned(),
(H) LINE crates/api/src/feeds_route.rs:2907  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  at_micros: 1_700_000_000_000_000,
(H) LINE crates/api/src/feeds_route.rs:2908  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  },
(H) LINE crates/api/src/feeds_route.rs:2909  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  |_| Ok(()),
(H) LINE crates/api/src/feeds_route.rs:2911  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .expect("the store's own check passes one end");
(H) LINE crates/api/src/feeds_route.rs:2912  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let rows = feeds_of("feeds-window-inverted", &root).await;
(H) LINE crates/api/src/feeds_route.rs:2913  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let window = &row(&rows, "zerodha")["window"];
(H) LINE crates/api/src/feeds_route.rs:2914  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(window["from"].is_null(), "{window}");
(H) LINE crates/api/src/feeds_route.rs:2915  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(
(H) LINE crates/api/src/feeds_route.rs:2916  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  window["why"]
(H) LINE crates/api/src/feeds_route.rs:2917  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .as_str()
(H) LINE crates/api/src/feeds_route.rs:2918  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .is_some_and(|why| why.contains("not served inverted")),
(H) LINE crates/api/src/feeds_route.rs:2921  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(window["degraded"].is_null(), "{window}");
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1606  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(fault.contains("Permission denied"), "{fault}");
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:1607  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  }
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2104  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert_eq!(run.total.bars_committed, 0);
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2105  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  let fault = run
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2106  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .store_unavailable
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2107  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .as_deref()
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2108  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  .expect("the breaker tripped");
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2109  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert!(fault.contains(".store-probe"), "{fault}");
(H) LINE crates/api/src/server_cloud_mutant_tests_c.rs:2110  [covered when its test runs as non-root: a root host red, proved; owed to the Mac's Route B]  assert_eq!(
(U) LINE crates/api/src/autopilot.rs:319  [`Window::from()`/`to()` are Days, and `Day::year_month` cannot fail (Day's bounds are YearMonth's, pull/src/session.rs:132-136 and 425-432, store/src/path.rs:583-588)]  _ => String::new(),
(U) BRANCH crates/api/src/autopilot.rs:958:21 False  [`day.succ()` refuses only 9999-12-31 (pull/src/session.rs:483-500), and this arm runs only when `day < to`; no Day is later than 9999-12-31 (MAX_YEAR), so succ never fails here]  let Ok(next) = day.succ() else {
(U) LINE crates/api/src/autopilot.rs:959  [as autopilot.rs:958:21 False]  done = done.saturating_add(1);
(U) LINE crates/api/src/autopilot.rs:960  [as autopilot.rs:958:21 False]  continue;
(U) BRANCH crates/api/src/autopilot.rs:2898:9 False  [`Day::year_month` cannot fail for any Day (pull/src/session.rs:132-136 and 425-432, store/src/path.rs:583-588)]  let (Ok(from), Ok(to)) = (floor.year_month(), yesterday.year_month()) else {
(U) LINE crates/api/src/autopilot.rs:2899  [as autopilot.rs:2898:9 False]  return 0;
(U) BRANCH crates/api/src/autopilot.rs:3047:9 False  [`Granularity::Day1.store_timeframe()` is the constant `Some(Timeframe::DAY_1)` (pull/src/vendor.rs:517)]  let Some(timeframe) = granularity.store_timeframe() else {
(U) LINE crates/api/src/autopilot.rs:3048  [as autopilot.rs:3047:9 False]  site.autopilot.publish(|status| {
(U) LINE crates/api/src/autopilot.rs:3049  [as autopilot.rs:3047:9 False]  status.phase = Phase::Halted;
(U) LINE crates/api/src/autopilot.rs:3050  [as autopilot.rs:3047:9 False]  status.detail = String::from("the daily rung has no directory in this store build");
(U) LINE crates/api/src/autopilot.rs:3051  [as autopilot.rs:3047:9 False]  });
(U) LINE crates/api/src/autopilot.rs:3052  [as autopilot.rs:3047:9 False]  return;
(U) BRANCH crates/api/src/autopilot.rs:3102:13 False  [`rung` is `rung_for(..)`, which returns only Granularity::Minute1 or Day1 (autopilot.rs:1121-1127), and both have a store timeframe (pull/src/vendor.rs:461, 517)]  let Some(rung_timeframe) = rung.store_timeframe() else {
(U) LINE crates/api/src/autopilot.rs:3103  [as autopilot.rs:3102:13 False]  continue;
(U) BRANCH crates/api/src/autopilot.rs:3726:9 False  [`slot` is `survey`'s, set only from `feeds.iter_mut().enumerate()` over the same `feeds` (autopilot.rs:3220-3316), whose length nothing changes in between, so `feeds.get_mut(slot)` is Some]  let Some(state) = feeds.get_mut(slot) else {
(U) LINE crates/api/src/autopilot.rs:3727  [as autopilot.rs:3726:9 False]  return Pass::owed(IDLE_POLL_SECS);
(U) LINE crates/api/src/bars_export.rs:984  [export_headers fails only if HeaderValue::from_str meets a byte below 32 other than tab, or 127 (http 1.5.0 per Cargo.lock:503-504; src/header/value.rs:105-107, 212-223, 563-565; bytes of 0x80 and above are accepted). Its inputs are: name = file_name(), which is fixed text plus Vendor::as_str, Timeframe::as_str (a fixed table with private fields, store/src/path.rs:351-354), the symbol filtered to [A-Za-z0-9._-] (bars_export.rs:782-792), the two days, and a u64. Each day is either bars::ist_day (bars.rs:230-237), which gives Day's %04d-%02d-%02d (pull/src/session.rs:650-655) or the em dash (bytes E2 80 94), or end_day's 'none' (713). None of these contains a forbidden byte.]  Err(why) => return refused(StatusCode::INTERNAL_SERVER_ERROR, "file_name", &why),
(U) BRANCH crates/api/src/bars_series.rs:525:13 False  [Same as line 526. usize::try_from(u64) is infallible on 64-bit hosts, including aarch64-apple-darwin, and the production unit keeps local at 1,350 or below (682-696). Only a direct call on a 32-bit target could fail it.]  let Ok(local) = usize::try_from(local) else {
(U) LINE crates/api/src/bars_series.rs:526  [local is a u64, and usize::try_from(u64) cannot fail where usize is 64 bits, which covers aarch64-apple-darwin and every 64-bit Linux host. Separately, the only production caller, rows_before (682-696), passes unit = ceil(into/span), where into = ts - day_start(day_of(ts)) is in [0, 86,400 s) (1896-1910). So unit is at most 86,400, word at most 1,350 and local at most 1,350, which fits any usize. Caveat: on a 32-bit target, a direct call TodDay::below(u) with u/64 - first_word > u32::MAX could take this line.]  return self.bars;
(U) BRANCH crates/api/src/bars_series.rs:631:24 False  [(1) minute_stamps() (store/src/month_summary.rs:757-769) emits, for day offset o in 0..days.len(), stamps of ((first_day+o) x 86,400 - 19,800 + m x 60) x 10^6. (2) m comes from set_bits masked to first/60..=last/60 (246-272), and every DayBits has last < 86,400: decode refuses otherwise (1071-1076), and fold derives it with rem_euclid (520-529, 627-636). So m is at most 1,439. (3) day_of (bars_series.rs:1896-1898, using the same 19,800 as pull/src/session.rs:88) maps the stamp back to first_day+o, and minute = m. (4) minutes has last_day-first_day+1 = days.len() slots (627; month_summary.rs:737-739). So the index is in range and u32::try_from(m) is Ok.]  if let (Some(slot), Ok(minute)) = (
(U) BRANCH crates/api/src/bars_series.rs:1272:17 False  [slot is below the self.slots.len() fixed at loop entry, and slots never shrinks: only push at 1250 writes it, and the body only reads or get_mut's it (1647-1670). So get(slot) is Some.]  let Some(at) = self.slots.get(slot) else {
(U) LINE crates/api/src/bars_series.rs:1273  [The loop bound 0..self.slots.len() is fixed once. slots is only ever pushed (admit, 1250; created at 1156) and is never removed from, truncated or replaced anywhere in bars_series.rs. The loop body (locate, which calls file() at 1647-1670, and runs.push) only reads or get_mut's slots. So get(slot) is Some for every slot below len.]  continue;
(U) BRANCH crates/api/src/bars_series.rs:1328:17 False  [Same as 1272. The bound is fixed at entry, and no code in cut_days or its callees removes from slots, so get(slot) is Some.]  let Some(at) = self.slots.get(slot) else {
(U) LINE crates/api/src/bars_series.rs:1329  [Same as line 1273. In cut_days the bound 0..self.slots.len() is fixed, and nothing in the body (cancel.check, the RUNS memo, tod_index_of, day_runs, locate/file()) shrinks slots. Only push at 1250 ever writes it.]  continue;
(U) BRANCH crates/api/src/booleanadmission_projection.rs:13:8 True  [every AdmissionVerdictV1 reconciles (runner/src/admission.rs:2044-2063 from_parts, the one-partition check_* calls of evaluate, and decode_verdict's refusal at 2654)]  if !verdict.reconciles() {
(U) LINE crates/api/src/booleanadmission_projection.rs:14  [every AdmissionVerdictV1 reconciles: its fields are private to runner::admission; `evaluate` (and _v2/_v3) decides each reason once through a check_* call that sets it in exactly one partition, `from_parts` unions them and derives the status by the precedence `reconciles` checks, and `decode_verdict` refuses one that does not reconcile (runner/src/admission.rs:837-…, 2044-2063, 2654)]  return Err("saved admission verdict partitions do not reconcile".to_owned());
(U) LINE crates/api/src/booleanlaunch_work.rs:19  [`cli::boolean_search_launch::work_model()` takes no argument and reads only compiled tables (`runner::live_positions()`, `vocab::expression::MAX_INSTRUCTIONS`, `ConditionMask::BITS`; cli/src/boolean_search_sizing.rs:30-46); the test `sizing_is_a_lower_bound_and_never_a_measured_population_or_eta` shows it is Ok on this build, so no input reaches the Err arm]  Err(why) => json!({"version":1,"state":"refused","refusal":why}),
(U) LINE crates/api/src/census_rollup.rs:409  [`resolve` answers Ok(exchange) only when `index.holds(exchange, symbol)`, `underlyings.contains_key(&(exchange, symbol))` (facets.rs:318-320); `underlying_cells` reads the same map by the same key through the same shared `&Index` (facets.rs:331-335), so it is Some]  None => return not_held(index, asked, symbol),
(U) BRANCH crates/api/src/dayflags.rs:388:13 False  [split_contract: `at` is from `rest.match_indices('-')`, a one-byte ASCII char, so both `get` calls are on char boundaries and Some for every input]  let (Some(head), Some(tail)) = (rest.get(..at), rest.get(at.saturating_add(1)..)) else {
(U) LINE crates/api/src/dayflags.rs:389  [split_contract: `at` is from `rest.match_indices('-')`, a one-byte ASCII char, so `rest.get(..at)` and `rest.get(at + 1..)` are both on char boundaries and Some for every input; the `else { continue }` never runs]  continue;
(U) BRANCH crates/api/src/dayflags.rs:772:20 False  [`note` is `basis_route::action_json(row)`, which returns the object literal `json!({..})` for every row (basis_route.rs:228-229), so `as_object_mut()` is Some]  if let Some(object) = note.as_object_mut() {
(U) LINE crates/api/src/dayflags.rs:792  [the fall-through of `if let Some(object) = note.as_object_mut()`: `note` is always the object `basis_route::action_json` builds (basis_route.rs:228-229)]  }
(U) BRANCH crates/api/src/dayflags.rs:1268:17 False  [every row of `imports::answer`'s `rows` is `imports::row_json` (imports.rs:1593-1599), which writes `"day": row.day.to_string()` and `"filter": row.filter` (a String) (imports.rs:703-705), so both `as_str()` are Some]  let (Some(day), Some(filter)) = (
(U) LINE crates/api/src/dayflags.rs:1272  [the `else { continue }` of a row with no string `day` or `filter`: every row `imports::answer` returns is `row_json`'s, which writes both as strings (imports.rs:703-705)]  continue;
(U) BRANCH crates/api/src/dayflags.rs:2430:24 False  [named_fields: `found` is `vec![None; want.len()]` and `at` enumerates `want`, so `found.get_mut(at)` is Some for every input]  && let Some(slot) = found.get_mut(at)
(U) BRANCH crates/api/src/dayflags.rs:3219:12 False  [pull_walked: the first lock hold inserts the path's tail (`held.entry(path).or_insert_with`, dayflags.rs:3184-3186) and no production code removes an entry (the one `.remove` is in a test), so `held.get_mut(path)` is Some]  if let Some(tail) = held.get_mut(path)
(U) BRANCH crates/api/src/dayflags.rs:3320:16 False  [warm_pull_journal: the first lock hold inserts the path's tail (`entry().or_insert_with`) and no production code removes an entry (the one `.remove` is in a test), so `held.get_mut(&path)` is Some]  if let Some(tail) = held.get_mut(&path)
(U) BRANCH crates/api/src/dayflags.rs:3546:9 False  [`Day::year_month` cannot fail: a Day holds years MIN_YEAR..=MAX_YEAR = 1970..=9999 (pull/src/session.rs:132-136, enforced by Day::new and kept private) and months 1..=12, exactly what `YearMonth::new` admits (store/src/path.rs:583-588); session.rs:425-432 says so]  let Ok(month) = page_first.year_month() else {
(U) LINE crates/api/src/dayflags.rs:3547  [the `else` of `page_first.year_month()`: `Day::year_month` cannot fail for any Day (Day's bounds are YearMonth's, pull/src/session.rs:132-136, store/src/path.rs:583-588)]  return SeriesStart::NoneHeld;
(U) LINE crates/api/src/dayflags.rs:3905  [`first.year_month()` cannot fail for any Day (Day's bounds are YearMonth's, pull/src/session.rs:132-136 and 425-432, store/src/path.rs:583-588)]  Err(why) => Opened::Refused(why.to_string()),
(U) LINE crates/api/src/dayflags.rs:4320  [`first.year_month()` cannot fail for any Day (Day's bounds are YearMonth's, pull/src/session.rs:132-136 and 425-432, store/src/path.rs:583-588)]  Err(why) => Opened::Refused(why.to_string()),
(U) BRANCH crates/api/src/dayflags.rs:4590:8 True  [`secs` is `finest(feed).secs()`, `rules::feed_rungs(feed).finest`, a constant rung for every feed, none of them zero seconds, so `secs == 0` is never true]  if secs == 0 || 60 / secs <= pull::gdfl_import::quality::SAMPLED_SS_DISTINCT {
(U) LINE crates/api/src/dayflags.rs:4718  [`code::record_code` (calendar_record.rs:171-187) returns None or one of CLOSED, UNSIZED, HALTED, SPECIAL, AUCTION, REGULAR, each matched above, so `Some(other)` never matches]  Some(other) => (other, "unnamed_code"),
(U) BRANCH crates/api/src/feeds_route.rs:339:27 False  [D-8665: no feed's descriptor records a history row for a rung it does not serve (compile-time pull::vendor descriptors), asserted feed by feed and rung by rung by `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`]  if !served && row.is_none() {
(U) BRANCH crates/api/src/feeds_route.rs:397:27 False  [D-8665: every HTTP feed's descriptor names how it addresses F&O (by name, offset or token), asserted feed by feed by `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`]  } else if spec.fno.by_token().is_some() {
(U) LINE crates/api/src/feeds_route.rs:403  [as feeds_route.rs:397:27 False (D-8665)]  "none"
(U) BRANCH crates/api/src/feeds_route.rs:510:15 True  [D-8665: every feed has a store prefix (`store_vendor()` is Some), asserted feed by feed by `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`]  } else if feed.store_vendor().is_none() {
(U) LINE crates/api/src/feeds_route.rs:513  [as feeds_route.rs:510:15 True (D-8665)]  (
(U) LINE crates/api/src/feeds_route.rs:514  [as feeds_route.rs:510:15 True (D-8665)]  false,
(U) LINE crates/api/src/feeds_route.rs:515  [as feeds_route.rs:510:15 True (D-8665)]  format!(
(U) LINE crates/api/src/feeds_route.rs:516  [as feeds_route.rs:510:15 True (D-8665)]  "{} has no store prefix, so nothing it pulled could be filed",
(U) LINE crates/api/src/feeds_route.rs:517  [as feeds_route.rs:510:15 True (D-8665)]  feed.display()
(U) LINE crates/api/src/feeds_route.rs:518  [as feeds_route.rs:510:15 True (D-8665)]  ),
(U) LINE crates/api/src/feeds_route.rs:519  [as feeds_route.rs:510:15 True (D-8665)]  )
(U) BRANCH crates/api/src/feeds_route.rs:732:16 False  [`segments` is `[Option<DaySpan>; 3]` (feeds_route.rs:711, 724) and `segment_slot` maps the three Segment variants to 0, 1, 2 by an exhaustive match (715-721), so `get_mut` is Some]  if let Some(slot) = self.segments.get_mut(segment_slot(key.segment)) {
(U) LINE crates/api/src/feeds_route.rs:734  [as feeds_route.rs:732:16 False]  }
(U) BRANCH crates/api/src/feeds_route.rs:926:20 False  [`closed` is `[Option<Day>; 3]` (feeds_route.rs:850) and `kind_slot` maps the three ImportKind variants to 0, 1, 2 by an exhaustive match (862-869), so `get_mut` is Some]  if let Some(slot) = reach.closed.get_mut(kind_slot(row.kind))
(U) BRANCH crates/api/src/feeds_route.rs:1505:9 False  [window_json returns at 1482-1484 for a feed with no descriptor window; for any other feed `settings_route::effective` answers WindowFrom/WindowTo from a generation (`value: Some`, settings_route.rs:294-305), never the environment (`Key::env()` is None for both, store/src/settings.rs:178-183), or from `default_of`, which is Some whenever any feed's descriptor has a window (settings_route.rs:196-213), and this feed's has]  let (Some(first), Some(last)) = (from.value, to.value) else {
(U) LINE crates/api/src/feeds_route.rs:1506  [as feeds_route.rs:1505:9 False]  return refuse(format!(
(U) LINE crates/api/src/feeds_route.rs:1507  [as feeds_route.rs:1505:9 False]  "the operator's window has no first or last day in force (from: {}; to: {})",
(U) LINE crates/api/src/feeds_route.rs:1508  [as feeds_route.rs:1505:9 False]  from.source, to.source
(U) LINE crates/api/src/feeds_route.rs:1509  [as feeds_route.rs:1505:9 False]  ));
(U) BRANCH crates/api/src/feeds_route.rs:1573:24 False  [D-8665: every known rung above a minute is whole minutes, asserted against the minute and the day by `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`]  && target.secs().is_multiple_of(source)
(U) BRANCH crates/api/src/feeds_route.rs:1665:20 True  [D-8665: no measured day basis is a fold, asserted feed by feed by `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`]  if day.is_fold {
(U) LINE crates/api/src/feeds_route.rs:1666  [as feeds_route.rs:1665:20 True (D-8665)]  "true"
(U) BRANCH crates/api/src/frontierjson.rs:193:9 False  [seek_window always returns start <= end <= total = rows.len() (detail.rs:1379-1412), and rows is the same Vec that was passed in at 189. So get(start..end) is Some.]  let Some(page_rows) = rows.get(window.start..window.end) else {
(U) LINE crates/api/src/frontierjson.rs:194  [window = detail::window(rows.len(), page) (189), which delegates to seek_window (detail.rs:1340-1348, 1379-1412). If total == 0, page must be 0, so start = 0 and end = min(take, 0) = 0. If total > 0, an offset >= total is refused, so start = offset < total and end = min(start+take, total) is between start and total. rows is the same, unchanged Vec, so rows.get(start..end) is always Some.]  return refuse(
(U) LINE crates/api/src/frontierjson.rs:195  [This is the same let-else as line 194. seek_window guarantees start <= end <= rows.len() (detail.rs:1379-1412), so the else never runs.]  json,
(U) LINE crates/api/src/frontierjson.rs:196  [This is the same let-else as line 194, and it is unreachable for the same reason.]  "the validated frontier page window did not fit its verified row vector; no fallback page is exposed",
(U) BRANCH crates/api/src/frontierjson.rs:265:8 True  [This is the same arithmetic as line 266. At most 4,096 rows (256 on HTTP), each at most 1,061 bytes of integer, bool and static fields, plus an envelope under 4 KB, gives about 4.35 MB at most, which is below 8,388,608 (detail.rs:32).]  if out.len() > crate::detail::MAX_RESPONSE_BYTES {
(U) LINE crates/api/src/frontierjson.rs:266  [Every field is an integer, a bool, a 64-hex digest, a static direction word, or the constant equity-note text. Per row: the template has 474 literal bytes (396-438). Add rank (u16, at most 5), direction (at most 16), 6 mask words (at most 120), 20 i64/u64 fields (at most 400; cli/src/frontier.rs:251-320, Derived 3230-3252), 9 bools (at most 45) and a comma, for at most 1,061 bytes. Rows: on HTTP, Page::parse caps limit at 256 (detail.rs:1185). Even a direct respond() call with any Page holds at most 4,096 rows: window refuses more (detail.rs:1342), and receipts above 4,096 rows are refused (frontierjson.rs:143-157). Envelope, under 4 KB: the identity; the note (cli/src/stored.rs:2564-2566, which is constant text in runner/src/audit.rs:130-154); about 20 integers; and a refusal that is either None for the read-only handle or a sentence of integers (frontierjson.rs:244-253; cli/src/frontier.rs:1650-1661). Maximum: 4,096 x 1,061 + 4,096, about 4.35 MB, which is below 8,388,608.]  return too_large(
(U) LINE crates/api/src/frontierjson.rs:267  [as crates/api/src/frontierjson.rs:266]  json,
(U) LINE crates/api/src/frontierjson.rs:268  [as crates/api/src/frontierjson.rs:266]  &identity,
(U) LINE crates/api/src/frontierjson.rs:269  [as crates/api/src/frontierjson.rs:266]  u64::try_from(rows.len()).unwrap_or(u64::MAX),
(U) LINE crates/api/src/frontierjson.rs:270  [as crates/api/src/frontierjson.rs:266]  &format!(
(U) LINE crates/api/src/frontierjson.rs:271  [as crates/api/src/frontierjson.rs:266]  "the bounded frontier response encoded to {} bytes; the hard response ceiling is {}. No oversized JSON was returned",
(U) LINE crates/api/src/frontierjson.rs:272  [as crates/api/src/frontierjson.rs:266]  out.len(),
(U) LINE crates/api/src/frontierjson.rs:273  [as crates/api/src/frontierjson.rs:266]  crate::detail::MAX_RESPONSE_BYTES
(U) LINE crates/api/src/frontierjson.rs:274  [as crates/api/src/frontierjson.rs:266]  ),
(U) BRANCH crates/api/src/gaps_route.rs:505:13 False  [`next_month` fails only for December 9999 (YearMonth::new refuses year 10000; gaps_route.rs:1230-1236, store/src/path.rs:583-588); this line runs only after `cursor >= last` was false, and no month is later than December 9999, so cursor is not it]  let Some(next) = next_month(cursor) else {
(U) LINE crates/api/src/gaps_route.rs:506  [as gaps_route.rs:505:13 False]  return (span, false);
(U) BRANCH crates/api/src/gaps_route.rs:1654:9 False  [`month` is a YearMonth (years 1970..=9999, months 1..=12, store/src/path.rs:583-588) and `Day::new` admits exactly those years (pull/src/session.rs:132-136, 380-386); day 1 exists in every month, so Day::new(year, month, 1) is Ok]  let Ok(first_day) = pull::session::Day::new(month.year(), month.month(), 1) else {
(U) LINE crates/api/src/gaps_route.rs:1655  [as gaps_route.rs:1654:9 False]  return empty(Some(format!(
(U) LINE crates/api/src/gaps_route.rs:1656  [as gaps_route.rs:1654:9 False]  "{month} is a month this build's calendar cannot name a first day \
(U) LINE crates/api/src/gaps_route.rs:1657  [as gaps_route.rs:1654:9 False]  for, so there is no range to audit against"
(U) LINE crates/api/src/gaps_route.rs:1658  [as gaps_route.rs:1654:9 False]  )));
(U) BRANCH crates/api/src/imports.rs:417:12 False  [snapshot comes from import_journal::snapshot (402), which builds a Snapshot in exactly two places. For an absent journal it sets present: false with a parsed Arc freshly allocated in this call (import_journal.rs:711-719). For a present journal it sets present: true (771-777). The kept entry holds a strong Arc (imports.rs:425-431), which is alive while the if-let holds the guard (411-417). Two live Arcs from different allocations are never ptr_eq, so when present is false, ptr_eq at 415 is false and the && stops before line 417. Whenever 417 is evaluated, present is true.]  && snapshot.present
(U) BRANCH crates/api/src/imports.rs:848:16 False  [DayOutcome has exactly five variants (import_journal.rs:64-77), and ALL lists all five (81-87). PartialEq is derived (63), so position() always finds row.outcome at an index in 0..5. counts is a [u64; DayOutcome::ALL.len()] array (imports.rs:819), so get_mut(at) is Some.]  if let Some(slot) = DayOutcome::ALL
(U) LINE crates/api/src/indexstopcandlesjson.rs:209  [View::direction() is the reader's direction field (index_stop_source_context.rs:487-489). That field is set only from selection(), which returns row.direction(), at open (609) and at select_setting (655-666). The row is a cli::index_stop_store::Record. Records have private fields (26-41), and the only code that builds one is decode (322, 391-406). decode maps direction word 1 to Long and 2 to Short and refuses every other value (344-348). The Reader also has private fields, and only open() builds one (538-633). So Undirected never reaches project().]  Direction::Undirected => return Err("saved candle source has no trading direction".into()),
(U) BRANCH crates/api/src/indexstopjson.rs:99:16 False  [This is the same bound as line 102. At most 256 rows, each at most 7,233 bytes of typed or bounded fields, plus an envelope under 10 KB, is at most about 1.86 MB. So reply.2.len() <= 8,388,608 always holds.]  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
(U) LINE crates/api/src/indexstopjson.rs:102  [Rows: the handler always runs Asked::parse (91-94), which caps limit at 256 (57), and window() returns at most limit rows (224-232). Row widths: every field is a bounded typed value. setting() is at most 7,233 bytes: 224 structure plus 17 values, including the Expression render of at most 5,757 bytes (MAX_INSTRUCTIONS 1,151 at vocab/src/expression.rs:15; at most 5 bytes per instruction, 399-461), an EVERY_RUNG timeframe, and 'NSE-' plus a symbol of at most 24 bytes (236-242). trade is about 810 bytes at most (243-245), event about 390 (246-248) and day about 420 (249-251). The envelope, including one selected setting and the constant policy (233-235), is about 9 KB at most. Maximum: 256 x 7,233 + 257 + 9,000, about 1.86 MB, which is below 8,388,608.]  refused(
(U) LINE crates/api/src/indexstopjson.rs:104  [as crates/api/src/indexstopjson.rs:102]  "single-stop response exceeds byte limit; no prefix returned",
(U) BRANCH crates/api/src/indexstoplaunch_metadata.rs:52:20 False  [`policy` is `booleanevidencejson::search_policy(..)` = `admission_projection::policy(..)`, which returns the object literal `json!({"digest":..,"values":rows})` (booleanadmission_projection.rs:76-134), so `as_object_mut()` is Some]  if let Some(fields) = policy.as_object_mut() {
(U) BRANCH crates/api/src/indexstopqualificationjson.rs:114:16 False  [This is the same arithmetic as line 117. Every page has at most 256 rows (72, 387-393), and each row is at most 28,978 bytes of typed or bounded fields. With the envelope, the maximum body is 7,422,805 bytes, which is below 8,388,608 (detail.rs:32). So the comparison is never false.]  if reply.2.len() <= crate::detail::MAX_RESPONSE_BYTES {
(U) LINE crates/api/src/indexstopqualificationjson.rs:117  [The byte limit cannot be passed. (1) Row count: no page has more than 256 rows, because Asked::parse refuses a limit outside 1..=256 (indexstopqualificationjson.rs:72) and window() returns at most `limit` rows (387-393). (2) Field widths: every field is one of these: an integer string of at most 20 chars; a 64-hex digest (server.rs:37302-37308); a static word; or typed text with a hard bound. No string is copied from a saved file. The bounded text fields are: (a) the expression, which is Expression::to_string over at most MAX_INSTRUCTIONS = 1,151 instructions (vocab/src/expression.rs:15,149-152). A leaf is Bit(u16), at most 5 digits (124). NOT adds 3 bytes and AND/OR add 5 (render_program 399-461), so a valid tree renders to at most 5,755 bytes. (b) The timeframe, which is an EVERY_RUNG &'static str of at most 5 chars (cli/src/lib.rs:10213-10215; index_stop_store.rs:349-351). (c) The instrument, which is 'NSE-' plus a validated symbol of at most 24 bytes (runner/src/research_family.rs:153-182). (3) One settings row (setting(), 290-322) is at most 28,978 bytes: 77 structure + 44 (index, rank) + 2 x indexstopjson::setting at most 7,233 each (224 structure + at most 7,009 values including the 5,757-byte expression; indexstopjson.rs:236-242) + admission_projection::row at most 6,596 (103 + 188 + 44 checks at most 2,733 whose 44 reason names total 718 chars, runner/src/admission.rs:1852,1901-1947, + 44 evidence rows at most 3,572 whose field names total 843 chars, booleanadmission_projection.rs:11-75) + saved_record at most 7,795 (4 reason lists of 14 names totalling 417 chars, cli/src/index_consistency.rs:551-567; 4 summaries of 21 integers; index_consistency_projection.rs:48-113). (4) Total: 256 x 28,978 + 257 separators + an envelope of at most 4,180 (19 keys, 3 digests, and a 39-row policy of at most 3,091, booleanadmission_projection.rs:76-134) = at most 7,422,805 < 8,388,608 = MAX_RESPONSE_BYTES (detail.rs:32). Fold, day and week rows are each under 1 KB (384-386; index_consistency_projection.rs:180-199). So every OK body fits.]  refused(
(U) LINE crates/api/src/indexstopqualificationjson.rs:119  [This is the same else-block as line 117. The proof there bounds every OK body at 7,422,805 bytes or less, which is below 8,388,608.]  "native qualification response exceeds byte limit; no prefix returned",
(U) BRANCH crates/api/src/indexstopqualificationjson.rs:154:16 False  [budget is BooleanObservationBudget::load() (146). That is a pure function of the process variable BRUTEX_BOOLEAN_OBSERVATION_BYTES (boolean_observation_budget.rs:7-13; derived Eq on a u64). The only Cached ever stored is built at 176-184 from that same call's budget, and CACHE is a function-local static (145) that nothing else can write. So, with root and identity equal, the stored budget can differ only if the process environment changed between two requests. No code in crates/ calls std::env::set_var or remove_var. set_var is unsafe under edition 2024 (workspace edition = 2024), and the crate is #![forbid(unsafe_code)] (lib.rs:40); isolated.rs:10-12 says a test may not set its own environment. A child process has one fixed environment, so the cache it fills always holds that budget. No query, file or direct call supplies the budget.]  && value.budget == budget
(U) LINE crates/api/src/indexstopqualificationjson.rs:317  [The ? can never return Err. (1) admission_projection::row errs only when !verdict.reconciles() (booleanadmission_projection.rs:12-15) or when evidence() errs (33). evidence() can err only if super::put meets a non-object (booleanevidencejson.rs:346-351), and every renderer it wraps returns a json!({..}) object literal (booleanadmission_projection.rs:36-55). (2) The verdict is row.projection.verdict() (315) of a ResearchAdmissionProjectionV1. Its fields are private (runner/src/research_admission_projection.rs:17-21), and its only constructor is AdmissionPolicyV1::evaluate_research_projection (47-57); the codec also goes through it (research_admission_projection_codec.rs:32-34). (3) That constructor sets verdict = evaluate(), which ends in AdmissionVerdictV1::from_parts (admission.rs:1194, 2045-2063). There, reasons is the union of the partitions and status is derived with exactly the precedence that reconciles() checks. (4) Each of the 44 checks inserts its own distinct bit (mask 1<<reason, 1950-1952) into at most one partition (2853-2985), so the partitions are disjoint. Therefore reconciles() (2103-2118) always holds.]  )?;
(U) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:347:13 False  [`cli::commit_stamp()` is `option_env!("BRUTEX_COMMIT")` (cli/src/lib.rs:3949-3953), fixed when the object is compiled; cli's build.rs stamps a clean tree, and the measured object (Route B's own checkout of a commit) is one, so the stamp is Some for every run of this object: the unstamped arm is reached only by a different object, a build of a dirty tree]  let Some(_stamp) = cli::commit_stamp() else {
(U) LINE crates/api/src/indexstopqualificationjson_saved_tests.rs:348  [as indexstopqualificationjson_saved_tests.rs:347:13 False]  return assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{reply}");
(U) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:639:13 False  [`cli::commit_stamp()` is `option_env!("BRUTEX_COMMIT")` (cli/src/lib.rs:3949-3953), fixed when the object is compiled; cli's build.rs stamps a clean tree, and the measured object (Route B's own checkout of a commit) is one, so the stamp is Some for every run of this object: the unstamped arm is reached only by a different object, a build of a dirty tree]  let Some(_stamp) = cli::commit_stamp() else {
(U) LINE crates/api/src/indexstopqualificationjson_saved_tests.rs:640  [as indexstopqualificationjson_saved_tests.rs:639:13 False]  assert_eq!(code, StatusCode::SERVICE_UNAVAILABLE, "{reply}");
(U) LINE crates/api/src/indexstopqualificationjson_saved_tests.rs:641  [as indexstopqualificationjson_saved_tests.rs:639:13 False]  return None;
(U) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:720:9 False  [`launched_settings` answers None only past its let-else at indexstopqualificationjson_saved_tests.rs:639:13, whose else runs only in an object built without a commit stamp: `cli::commit_stamp()` is `option_env!("BRUTEX_COMMIT")` (cli/src/lib.rs:3949-3953), fixed when the object is compiled; cli's build.rs stamps a clean tree, and the measured object (Route B's own checkout of a commit) is one, so the stamp is Some for every run of this object]  let Some(settings) = launched_settings(Path::new(&root), (4, 5), (6, 6)) else {
(U) LINE crates/api/src/indexstopqualificationjson_saved_tests.rs:721  [as indexstopqualificationjson_saved_tests.rs:720:9 False]  return;
(U) BRANCH crates/api/src/indexstopqualificationjson_saved_tests.rs:788:9 False  [`launched_settings` answers None only past its let-else at indexstopqualificationjson_saved_tests.rs:639:13, whose else runs only in an object built without a commit stamp: `cli::commit_stamp()` is `option_env!("BRUTEX_COMMIT")` (cli/src/lib.rs:3949-3953), fixed when the object is compiled; cli's build.rs stamps a clean tree, and the measured object (Route B's own checkout of a commit) is one, so the stamp is Some for every run of this object]  let Some(settings) = launched_settings(Path::new(&root), (4, 5), (6, 6)) else {
(U) LINE crates/api/src/indexstopqualificationjson_saved_tests.rs:789  [as indexstopqualificationjson_saved_tests.rs:788:9 False]  return;
(U) BRANCH crates/api/src/ingest.rs:1332:9 False  [Same proof as line 1334. Four, two and two ASCII digits always parse as u16 and u8 (1318-1329), so the tuple pattern always matches.]  let (Ok(year), Ok(month), Ok(day)) = (y.parse::<u16>(), m.parse::<u8>(), d.parse::<u8>())
(U) LINE crates/api/src/ingest.rs:1334  [By line 1332, y, m and d are the three '-'-separated pieces (1318-1323), of lengths 4, 2 and 2 (1324), and every byte of the text is '-' or an ASCII digit (1329). So each piece is all digits. y is at most 9999, which fits u16; m and d are at most 99, which fits u8; and from_str accepts leading zeros. All three parses are Ok.]  return Err(bad());
(U) BRANCH crates/api/src/ladder.rs:455:16 False  [`Day::year_month` cannot fail for any Day (Day's bounds are YearMonth's, pull/src/session.rs:132-136 and 425-432, store/src/path.rs:583-588)]  if let Ok(ym) = at.year_month() {
(U) BRANCH crates/api/src/ladder.rs:462:13 False  [`Day::from_days(last + 1)` fails only past 9999-12-31, the last Day (MAX_DAY_NUMBER, pull/src/session.rs:666); this line runs only when `last < window.to()`, and no window ends after 9999-12-31, so `last + 1` is a Day]  let Ok(next) = pull::session::Day::from_days(last.days_from_epoch().saturating_add(1))
(U) LINE crates/api/src/ladder.rs:464  [as ladder.rs:462:13 False]  return out;
(U) BRANCH crates/api/src/logs.rs:2664:16 False  [The block runs only when selection is Some (2651). selection is Some only when is_db_read(&path) is true (2616), that is, when DB_READS.contains(&path) (2826-2828). path is the same immutable String (2613), so find(|r| *r == path), which does the same content comparison over the same array, always finds it.]  if let Some(route) = DB_READS.iter().copied().find(|route| *route == path) {
(U) BRANCH crates/api/src/logs.rs:4005:13 False  [The loop bounds keep 0 <= current - serial <= 15 (3998-4001). audit_path_for returns Some for every such back (sink.rs:3190-3197, AUDIT_KEEP_FILES = 16 at 2465), so the let-else always matches.]  let Some(path) = telemetry::audit_path_for(dir, current, serial) else {
(U) LINE crates/api/src/logs.rs:4006  [oldest = current - (AUDIT_KEEP_FILES - 1), saturating (3998), and serial runs over oldest..=current (4001). So back = current - serial is always in 0..=15. telemetry::audit_path_for returns Some when back == 0 and Some when back fits a u8 and is below AUDIT_KEEP_FILES = 16 (telemetry/src/sink.rs:3189-3198, 2465). It returns None only for serial > current or back >= 16, and the loop produces neither. So the else/continue never runs.]  continue;
(U) BRANCH crates/api/src/logs.rs:4608:20 False  [unreadable is incremented only in fill's Unreadable arm (4353-4359). The first time that arm runs, it sets unread_at = Some(errors.len()) and pushes onto that same vector. Streams start with unreadable 0 and unread_at None (4749-4750). fill is called only at 4794, with &mut out.errors, and index_rows only ever appends to out.errors (4564-4628). So unreadable > 1 implies unread_at = Some(i) with i < out.errors.len(), and get_mut(i) is Some.]  && let Some(why) = stream.unread_at.and_then(|at| out.errors.get_mut(at))
(U) BRANCH crates/api/src/logs.rs:6023:16 False  [n comes from enumerate over lists.iter().zip(heads.iter()) (6014-6019), and heads.len() == lists.len() (6011). Both are unchanged before 6023, so lists.get(n) and heads.get_mut(n) are both Some.]  if let (Some(list), Some(head)) = (lists.get(n), heads.get_mut(n)) {
(U) BRANCH crates/api/src/logs.rs:6024:20 False  [n was chosen only if list.get(*head) was Some (6018), and neither list nor heads[n] changes between 6018 and 6024. So the same get is Some.]  if let Some(item) = list.get(*head) {
(U) LINE crates/api/src/logs.rs:6028  [This brace is the implicit else of 6023, which never runs. next yields n only from lists.iter().zip(heads.iter()).enumerate() where list.get(*head) is Some (6014-6019). heads has lists.len() entries (6011), and neither lists nor heads changes before 6023. So lists.get(n) and heads.get_mut(n) are Some, and list.get(*head) is the same Some.]  }
(U) LINE crates/api/src/mastersrun.rs:469  [obtain's `tried` is `masters::fetch(..)`'s; fetch asks every URL of `source.every_url()` (always at least `source.url`) and pushes a `Got::Refused` for every ask that brings no body (pull/src/masters.rs:1728-1790), so a Fetched without a body always holds a refusal and `last_refusal()` is Some]  || format!("`{}` — no URL was reachable", source.url),
(U) BRANCH crates/api/src/mastersrun.rs:493:9 False  [`Feed::Zerodha.descriptor()` is the constant row `pull::vendor::ZERODHA` (pull/src/vendor.rs:3878-3880, 6060), whose transport is `Transport::Http(..)` (vendor.rs:6076)]  let pull::vendor::Transport::Http(spec) = feed.descriptor().transport else {
(U) LINE crates/api/src/mastersrun.rs:494  [Zerodha's descriptor is the constant HTTP row (pull/src/vendor.rs:6060, 6076): the non-HTTP refusal never runs]  return Err(
(U) LINE crates/api/src/mastersrun.rs:495  [as mastersrun.rs:494]  "Zerodha's descriptor is not an HTTP transport, so its instrument \
(U) LINE crates/api/src/mastersrun.rs:496  [as mastersrun.rs:494]  dump cannot be fetched"
(U) LINE crates/api/src/mastersrun.rs:497  [as mastersrun.rs:494]  .to_owned(),
(U) LINE crates/api/src/mastersrun.rs:498  [as mastersrun.rs:494]  );
(U) LINE crates/api/src/mastersrun.rs:850  [`masters::SOURCES` is the constant array of five (pull/src/masters.rs:269) whose vendors are Dhan, Groww, Zerodha, None and None (masters.rs:272, 301, 310, 329, 351): `Some(other)` past those three never matches]  _ => "another feed",
(U) LINE crates/api/src/optiongreeks.rs:213  [the `None` arm of `stamp`: `meta.modified()` on unix is `SystemTime::new(st_mtime, st_mtime_nsec)` in std, which fails only for a nanosecond field outside 0..1e9, and the kernel never reports one (utimensat refuses it with EINVAL); so every stat'ed file has a stamp and this arm never runs on aarch64-apple-darwin or Linux]  if let Some(held) = kept.as_mut() {
(U) BRANCH crates/api/src/optiongreeks.rs:213:24 False  [as optiongreeks.rs:213]  if let Some(held) = kept.as_mut() {
(U) BRANCH crates/api/src/optiongreeks.rs:213:24 True  [as optiongreeks.rs:213]  if let Some(held) = kept.as_mut() {
(U) LINE crates/api/src/optiongreeks.rs:214  [as optiongreeks.rs:213]  held.loads = loads;
(U) LINE crates/api/src/optiongreeks.rs:215  [as optiongreeks.rs:213]  }
(U) LINE crates/api/src/optiongreeks.rs:492  [the underlying's path is the option's (already checked by StorePath::new at 460-466, whose Invalid returned first) with its segment and symbol from `brutex_core::alias::option_underlying` (static upper-case words) or the option's own symbol, and no contract; StorePath::new checks only those words (store/src/path.rs:897-903), so it cannot be Invalid when the option's was not]  OpenRefusalKind::Invalid => return Err(refusal.message),
(U) LINE crates/api/src/optiongreeks.rs:556  [the premium read is `spot_at(option, self.month, ts, AsOf::Exact)` for an instant `answer_with` already placed in `self.month` (optiongreeks.rs:508-518); spot_at answers OutsideMonth only for an instant outside that month (spot.rs:305-311)]  Spot::OutsideMonth => return Cell::refused("t_outside_month", None),
(U) LINE crates/api/src/optiongreeks.rs:557  [the premium read is under AsOf::Exact, and spot_at answers OffGrid only under AsOf::Prev (spot.rs:312-314)]  Spot::OffGrid => return Cell::refused("t_off_grid", None),
(U) LINE crates/api/src/optiongreeks.rs:559  [spot_at never answers NoFile (spot.rs:305-341: OutsideMonth, OffGrid, At, NoBar, OrderBroken, NoBarInSession) and the `read` closure maps its Err to Unreadable, so the premium read is never NoFile]  Spot::NoFile => return Cell::refused("no_option_file", None),
(U) LINE crates/api/src/optiongreeks.rs:561  [spot_at never answers BeingWritten (spot.rs:305-341) and the `read` closure maps its Err to Unreadable, so the premium read is never BeingWritten]  Spot::BeingWritten => return Cell::refused("option_being_written", None),
(U) LINE crates/api/src/optiongreeks.rs:794  [`self.settled` is `settled_of(symbol, expiry)` = `settle(symbol, day, None)`, whose only refusal with no trade day is a closed stated day walked back to an unmeasured one (pull/src/expiry_alias.rs:775-791); the sourced calendar opens on a trading day, 2018-01-01, so it never refuses for any Expiry (1990..=2100): the test every_expiry_the_instrument_model_admits_is_settled walks every one]  Err(why) => (
(U) LINE crates/api/src/optiongreeks.rs:795  [as optiongreeks.rs:794]  "null".to_owned(),
(U) LINE crates/api/src/optiongreeks.rs:796  [as optiongreeks.rs:794]  render::json_string(&format!("unsettled: {why}")),
(U) LINE crates/api/src/optiongreeks.rs:797  [as optiongreeks.rs:794]  ),
(U) LINE crates/api/src/speed_route.rs:69  [`speed::Report` is derived Serialize over u64, String, &'static str, Vec<RowReport>, Option<u64> and the unit-variant enum Kind (speed.rs:182, 355, 468-501); no map key and no hand-written Serialize, so serde_json::to_string into a String cannot return Err]  Err(why) => (
(U) LINE crates/api/src/speed_route.rs:70  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  StatusCode::INTERNAL_SERVER_ERROR,
(U) LINE crates/api/src/speed_route.rs:71  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  format!(
(U) LINE crates/api/src/speed_route.rs:72  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  "{{\"error\":\"the speed report would not serialise: {}\"}}",
(U) LINE crates/api/src/speed_route.rs:73  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  why.to_string().replace(['"', '\\'], "'")
(U) LINE crates/api/src/speed_route.rs:74  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  ),
(U) LINE crates/api/src/speed_route.rs:75  [as speed_route.rs:69: serde_json::to_string of the derived plain-data Report cannot fail]  ),
(U) BRANCH crates/api/src/sweepevidence.rs:95:21 False  [This is the same bound as line 98. At most 4,096 rows of at most 616 bytes each, plus an envelope under 2 KB, is at most about 2.53 MB. So an Ok body is always 8,388,608 bytes or less.]  Ok(body) if body.len() <= crate::detail::MAX_RESPONSE_BYTES => {
(U) LINE crates/api/src/sweepevidence.rs:98  [Rows: Page::parse caps limit at 256 on HTTP (detail.rs:1185). Even a direct respond() call with a hand-made Asked reads at most 4,096 rows, because sweep_evidence::page refuses more with an Err, not a body (cli/src/sweep_evidence.rs:1579-1581). Field widths: every field is a u64, i64, bool or static word (cli/src/sweep_evidence.rs:251-269, 338-365, 457-476; as_str 135-227). A ranked row is at most 615 bytes (218 literal + 6 mask words at most 137 + 13 integers at most 260; sweepevidence.rs:230-254). A depth row is at most 320 bytes (213-228). Envelope, under 2 KB: the identity, metadata (196-211), the constant equity note (detail.rs:1765-1771) and integers. Maximum: 4,096 x 616 + 2,048, about 2.53 MB, which is below 8,388,608.]  Ok(_) => refusal(
(U) LINE crates/api/src/sweepevidence.rs:100  [as crates/api/src/sweepevidence.rs:98]  "saved evidence exceeds the JSON response bound; no prefix was exposed",
(U) BRANCH crates/api/src/sweeprun.rs:204:20 False  [`search` is a `booleanlaunch::Status`, whose `fields()` returns the object literal `json!({..})` (booleanlaunch.rs:60-61), so `as_object()` is Some]  && let Some(fields) = search.fields().as_object()
(U) BRANCH crates/api/src/sweeprun.rs:212:20 False  [`search` is an `indexstoplaunch::Status`, whose `fields()` returns the object literal `json!({"schema_version":1,..})` (indexstoplaunch.rs:110-135), so `as_object_mut()` is Some]  if let Some(object) = fields.as_object_mut() {
(U) LINE crates/api/src/sweeprun.rs:223  [as sweeprun.rs:212:20 False]  }
`````

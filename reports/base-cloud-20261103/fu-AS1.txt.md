# fu-AS1.txt.md

Session BASE-COV-API-AS1. One fenced block below: every uncovered row of G1's files that is an (F) failing arm of an assert or panic in test code whose statement ran, or a (U) row with its proof in brackets.

`````text
BASE-COV-API repair 3, shard AS1 (G1 of SHARDS.md), Route B on code tip 3f324ee88b6c24fff98aa59779ac164896a13a31, reading whole:api, set whole-api-set-4.txt
G1 files read 76 of 76: lines 55261/56448 (1187 uncovered), branches 4033/4890 (857 uncovered sides)
classified: GAP prod 821 lines 366 sides; GAP test 116 lines 112 sides; GAP ignored 37 lines 0 sides; HOST 64 lines 10 sides; (F) 125 lines 357 sides; (U) 24 lines 12 sides
HOST = covered on the Mac's run of START (the kit's lists hold no row for it) and not here: the test that reaches it is a host red of this 4-core root host or one of the two tests skipped here as 4-core hangs; owed to the Mac's Route B.
(F) LINE crates/api/src/assets.rs:1137  default_web_dir().display()
(F) LINE crates/api/src/assets.rs:1453  assets.named().display()
(F) LINE crates/api/src/assets.rs:1566  panic!(
(F) BRANCH crates/api/src/assets.rs:1530:17 False  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1530:46 False  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1565:13 False  let Build::Stale { ref newer, by_secs } = *built.build() else {
(F) BRANCH crates/api/src/assets.rs:1582:13 False  note.contains("STALE") && note.contains("does not carry that edit"),
(F) BRANCH crates/api/src/assets.rs:1582:39 False  note.contains("STALE") && note.contains("does not carry that edit"),
(F) LINE crates/api/src/audit.rs:3566  panic!("three entries: {page:?}");
(F) LINE crates/api/src/audit.rs:3635  &read[read.len().saturating_sub(40)..]
(F) LINE crates/api/src/audit.rs:4041  kind.label(),
(F) LINE crates/api/src/audit.rs:4042  scope.label(),
(F) LINE crates/api/src/audit.rs:4043  outcome.label()
(F) LINE crates/api/src/audit.rs:5670  [START line 5587] panic!(
(F) LINE crates/api/src/audit.rs:6162  [panic/unreachable line] panic!("one entry: {page:?}");
(F) BRANCH crates/api/src/audit.rs:3410:13 False  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3410:42 False  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3565:13 False  let [run, fits, whole] = page.as_slice() else {
(F) BRANCH crates/api/src/audit.rs:4431:13 False  refusal.contains("injected short write")
(F) BRANCH crates/api/src/audit.rs:4432:20 False  && refusal.contains("truncated back to 256 bytes"),
(F) BRANCH crates/api/src/audit.rs:5669:13 False  [START line 5586] let Some(Err(why)) = &page[0].whole_note else {
(F) BRANCH crates/api/src/audit.rs:5416:25 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] assert!(p50 <= p99 && p99 <= max, "{tag}: {p50} {p99} {max}");
(F) BRANCH crates/api/src/audit.rs:5416:39 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] assert!(p50 <= p99 && p99 <= max, "{tag}: {p50} {p99} {max}");
(F) LINE crates/api/src/audit_json.rs:1538  other.as_str()
(F) LINE crates/api/src/audit_json.rs:1840  panic!("counted again for an unchanged census")
(F) BRANCH crates/api/src/audit_json.rs:1425:17 False  assert!(json.starts_with('{') && json.ends_with('}'), "{json}");
(F) BRANCH crates/api/src/audit_json.rs:1425:42 False  assert!(json.starts_with('{') && json.ends_with('}'), "{json}");
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:86:80 False  matches!(census.state, crate::census::Census::Held { ref manifest } if manifest.is_partitioned()),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:184:9 False  note.contains("CASH") && note.contains("lower bound"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:184:34 False  note.contains("CASH") && note.contains("lower bound"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:256:9 False  note.contains("is a directory") && note.contains("not a commit"),
(F) BRANCH crates/api/src/audit_json_commit_tests.rs:256:44 False  note.contains("is a directory") && note.contains("not a commit"),
(F) LINE crates/api/src/audit_lock_wait_tests.rs:659  panic!("the parent should have killed this child");
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:9 False  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:42 False  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) BRANCH crates/api/src/audit_lock_wait_tests.rs:715:74 False  !source.contains("\nstatic ") && !source.contains("LazyLock") && !source.contains("Mutex<"),
(F) LINE crates/api/src/autopilot_stall_tests.rs:105  feeds[0].stalls
(F) LINE crates/api/src/backtest.rs:2326  broken.to_json()
(F) BRANCH crates/api/src/backtest.rs:2151:13 False  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) BRANCH crates/api/src/backtest.rs:2151:46 False  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) LINE crates/api/src/bars.rs:2869  found.len()
(F) LINE crates/api/src/bars.rs:3028  compared.get()
(F) LINE crates/api/src/bars.rs:3103  ist_clock(at)
(F) LINE crates/api/src/bars.rs:3656  got.bars[0].chg_why
(F) LINE crates/api/src/bars.rs:4620  panic!("one request took more than 256 looks at its flag");
(F) BRANCH crates/api/src/bars.rs:3920:17 False  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:3920:62 False  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:4202:13 False  got.faults[0].contains(" record 73: ")
(F) BRANCH crates/api/src/bars.rs:4203:20 False  && got.faults[0].contains("73 kept rows of 2026-01 in this window would not read"),
(F) BRANCH crates/api/src/bars.rs:4301:17 False  assert!(first.oi_applies && first.v_why.is_none());
(F) BRANCH crates/api/src/bars.rs:4301:37 False  assert!(first.oi_applies && first.v_why.is_none());
(F) BRANCH crates/api/src/bars_change.rs:764:13 False  why.contains("UNVERIFIED") && why.contains("absent"),
(F) BRANCH crates/api/src/bars_change.rs:764:43 False  why.contains("UNVERIFIED") && why.contains("absent"),
(F) BRANCH crates/api/src/bars_change.rs:1036:13 False  fault.starts_with("cannot read ") && fault.ends_with("no lineage was applied"),
(F) BRANCH crates/api/src/bars_change.rs:1036:50 False  fault.starts_with("cannot read ") && fault.ends_with("no lineage was applied"),
(F) LINE crates/api/src/bars_page_tests.rs:465  panic!("{route}: the pool stayed full for two seconds");
(F) BRANCH crates/api/src/bars_page_tests.rs:286:9 False  !page.contains("/FNO/NIFTY/1s/") && !page.contains("/INDEX/"),
(F) BRANCH crates/api/src/bars_page_tests.rs:286:45 False  !page.contains("/FNO/NIFTY/1s/") && !page.contains("/INDEX/"),
(F) LINE crates/api/src/basis_route_tests.rs:181  body["corporate_actions"]
(F) LINE crates/api/src/basis_route_tests.rs:243  body["corporate_actions"]
(F) LINE crates/api/src/basis_route_tests.rs:499  repaired.actions.table().err()
(F) BRANCH crates/api/src/booleanoosjson_tests.rs:184:13 False  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) BRANCH crates/api/src/booleanoosjson_tests.rs:184:40 False  assert!(why.contains("1 bytes") && why.contains("BRUTEX_BOOLEAN_OBSERVATION_BYTES"));
(F) LINE crates/api/src/calendar_build.rs:822  panic!("the first request leads");
(F) LINE crates/api/src/calendar_build.rs:825  panic!("the second follows");
(F) LINE crates/api/src/calendar_build.rs:856  panic!("the first request leads");
(F) LINE crates/api/src/calendar_build.rs:881  panic!("leads");
(F) LINE crates/api/src/calendar_build.rs:904  panic!("leads");
(F) LINE crates/api/src/calendar_build.rs:930  [panic/unreachable line] panic!("a queued key is followed: {joined:?}");
(F) LINE crates/api/src/calendar_build.rs:935  [START line 933] matches!(
(F) LINE crates/api/src/calendar_build.rs:988  [panic/unreachable line] panic!("room for build {n}: {joined:?}");
(F) BRANCH crates/api/src/calendar_build.rs:821:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:824:13 False  let Joined::Follow(follow) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:855:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:880:13 False  let Joined::Lead(second) = builds.join(&key("BANKNIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:903:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:929:17 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Joined::Follow(build) = joined else {
(F) BRANCH crates/api/src/calendar_build.rs:987:17 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Joined::Lead(build) = joined else {
(F) BRANCH crates/api/src/catalog.rs:918:17 False  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:918:52 False  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:919:17 False  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:919:61 False  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:920:17 False  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(F) BRANCH crates/api/src/catalog.rs:920:56 False  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(F) LINE crates/api/src/census.rs:2646  got.note()
(F) LINE crates/api/src/census.rs:3189  grid.len()
(F) BRANCH crates/api/src/census.rs:2764:13 False  note.contains(&(MAX_MANIFEST_BYTES + 1).to_string())
(F) BRANCH crates/api/src/census.rs:2765:20 False  && note.contains(&MAX_MANIFEST_BYTES.to_string()),
(F) BRANCH crates/api/src/census.rs:2824:17 False  why.contains("(char_device)")
(F) BRANCH crates/api/src/census.rs:2825:24 False  && why.contains("a character device, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:17 False  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:46 False  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2841:21 False  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:2841:41 False  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:3193:26 False  rows == 0 || filled > 0,
(F) LINE crates/api/src/census_epoch_link_tests.rs:92  panic!("a link must be refused, never read: {}", census.note());
(F) LINE crates/api/src/census_epoch_link_tests.rs:104  path.display()
(F) LINE crates/api/src/census_epoch_link_tests.rs:136  census.note()
(F) LINE crates/api/src/census_epoch_link_tests.rs:231  census.note()
(F) LINE crates/api/src/census_epoch_link_tests.rs:294  panic!("a link must be refused, never read: {}", census.note());
(F) LINE crates/api/src/census_epoch_link_tests.rs:323  census.note()
(F) BRANCH crates/api/src/census_epoch_link_tests.rs:91:9 False  let Census::Unreadable { reason, fault } = &census.state else {
(F) BRANCH crates/api/src/census_epoch_link_tests.rs:293:9 False  let Census::Unreadable { reason, fault } = &census.state else {
(F) LINE crates/api/src/census_warm_tests.rs:197  panic!("not held: {}", census.note());
(F) LINE crates/api/src/census_warm_tests.rs:428  took[499]
(F) LINE crates/api/src/census_warm_tests.rs:440  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:687  &store.reads()[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:702  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:823  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:983  &reads[mark..]
(F) LINE crates/api/src/census_warm_tests.rs:1499  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1516  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1728  began.elapsed()
(F) LINE crates/api/src/census_warm_tests.rs:1799  began.elapsed()
(F) BRANCH crates/api/src/census_warm_tests.rs:196:9 False  let census::Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/census_warm_tests.rs:785:41 False  .all(|(path, kind, thread)| *kind == "whole"
(F) BRANCH crates/api/src/census_warm_tests.rs:786:20 False  && path.starts_with(Path::new(&store.root))
(F) BRANCH crates/api/src/census_warm_tests.rs:981:38 False  .all(|(_, kind, thread)| *kind != "whole" && is_census_thread(thread)),
(F) BRANCH crates/api/src/census_warm_tests.rs:1032:13 False  assert!(whole["underlying"].is_null() && whole["exchange"].is_null());
(F) BRANCH crates/api/src/census_warm_tests.rs:1032:46 False  assert!(whole["underlying"].is_null() && whole["exchange"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:423:17 False  assert!(chain["atm"].is_null() && chain["pick"]["rung"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:423:43 False  assert!(chain["atm"].is_null() && chain["pick"]["rung"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:440:13 False  assert!(chain["spot"].is_null() && chain["spot_t"].is_null());
(F) BRANCH crates/api/src/chain_route_tests.rs:440:40 False  assert!(chain["spot"].is_null() && chain["spot_t"].is_null());
(F) BRANCH crates/api/src/cost_limits_tests.rs:108:9 False  !inside.contains("ingestion_observations") && inside.contains("land_bodies_observed("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:108:55 False  !inside.contains("ingestion_observations") && inside.contains("land_bodies_observed("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:149:13 False  assert!(append < hook && repair < hook, "{body}");
(F) BRANCH crates/api/src/cost_limits_tests.rs:149:30 False  assert!(append < hook && repair < hook, "{body}");
(F) BRANCH crates/api/src/cost_limits_tests.rs:156:9 False  walk[hook..].contains("(&landing, door)") && walk.contains("done.absorb(landing.ingested)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:156:54 False  walk[hook..].contains("(&landing, door)") && walk.contains("done.absorb(landing.ingested)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:160:9 False  item(INGEST, "pub fn from_window(").contains("from_window_with_refusals(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:161:16 False  && item(INGEST, "pub fn repair_window(").contains("repair_window_with_refusals("),
(F) BRANCH crates/api/src/cost_limits_tests.rs:177:9 False  item(CENSUS_STORE, "pub fn cycle(").contains("cycle_with(target, held, mode, None)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:178:16 False  && item(CENSUS_STORE, "pub fn record(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:180:16 False  && item(CENSUS_STORE, "fn cycle_with(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:189:9 False  load.contains("read_single(&path, self.vendor)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:190:16 False  && load.contains("read_epoch(&path, self.vendor, segment)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:191:16 False  && item(CENSUS_STORE, "fn read_single(").contains("read_census(path, vendor)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:229:9 False  miss.contains("let (mut censuses, kept_under) = read(&site.store_root);")
(F) BRANCH crates/api/src/cost_limits_tests.rs:230:16 False  && item(CENSUS_CACHE, "pub(super) fn census_now_stamping(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:235:9 False  miss.contains("HeldEntries::of(&censuses)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:236:16 False  && !miss.contains("census::held_entries(&censuses)"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:248:9 False  item(CENSUS_CACHE, "fn vendor_slot_fresh(").contains("rebuild(&site.store_root, cell,")
(F) BRANCH crates/api/src/cost_limits_tests.rs:249:16 False  && item(CENSUS_CACHE, "fn rebuild(").contains("let census = fresh_read(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:250:16 False  && item(CENSUS_CACHE, "fn fresh_read(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:291:9 False  answer.contains("site.census_wire.kept(") && answer.contains("universe.generation"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:291:54 False  answer.contains("site.census_wire.kept(") && answer.contains("universe.generation"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:342:9 False  window.contains("sort::page(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:343:16 False  && window.contains("scan(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:346:16 False  && scan.contains("plan.read_stretch(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:347:16 False  && scan.contains("(run.start, run.end),"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:359:9 False  !window.contains("all.sort_by(order)") && !scan.contains("want - 1"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:359:51 False  !window.contains("all.sort_by(order)") && !scan.contains("want - 1"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:393:9 False  walk.contains("manifest.held_keys()") && !walk.contains("held_entries"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:393:50 False  walk.contains("manifest.held_keys()") && !walk.contains("held_entries"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:479:9 False  before.contains("file.first_at_or_after(at)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:480:16 False  && after.contains("file.read_record(")
(F) BRANCH crates/api/src/cost_limits_tests.rs:481:16 False  && after.contains(r#"return Ok((axum::http::StatusCode::OK, "[]".to_owned()));"#),
(F) BRANCH crates/api/src/cost_limits_tests.rs:490:9 False  decision.contains("held.checked_sub(1).filter(|_| landed >= held)")
(F) BRANCH crates/api/src/cost_limits_tests.rs:491:16 False  && decision.contains("stamp != 0 && stamp == header_last_ts && stamp < from"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:560:9 False  SERVER.contains("// D-0136.") && SERVER.contains("THE TARGET GUARD IS GONE"),
(F) BRANCH crates/api/src/cost_limits_tests.rs:560:42 False  SERVER.contains("// D-0136.") && SERVER.contains("THE TARGET GUARD IS GONE"),
(F) BRANCH crates/api/src/coverage.rs:840:13 False  why.contains("900001") && why.contains("no ISIN column"),
(F) BRANCH crates/api/src/coverage.rs:840:39 False  why.contains("900001") && why.contains("no ISIN column"),
(F) BRANCH crates/api/src/coverage.rs:1206:17 False  line.contains("not pull readiness")
(F) BRANCH crates/api/src/coverage.rs:1207:24 False  && line.contains("does not silently skip")
(F) BRANCH crates/api/src/coverage.rs:1208:24 False  && line.contains("non-ISIN"),
(F) LINE crates/api/src/credential_law_tests.rs:193  panic!("a same-value verdict halts the feed");
(F) LINE crates/api/src/credential_law_tests.rs:392  panic!("an unusable configuration halts");
(F) LINE crates/api/src/credential_law_tests.rs:445  panic!("Dhan is HTTP");
(F) LINE crates/api/src/credential_law_tests.rs:508  vendor.seen()
(F) LINE crates/api/src/credential_law_tests.rs:633  panic!("Groww is HTTP");
(F) BRANCH crates/api/src/credential_law_tests.rs:192:9 False  let autopilot::Next::Halt { reason } = state.observe(&outcome) else {
(F) BRANCH crates/api/src/credential_law_tests.rs:227:13 False  assert!(seen[0].contains("stale") && !seen[0].contains("fresh"));
(F) BRANCH crates/api/src/credential_law_tests.rs:227:42 False  assert!(seen[0].contains("stale") && !seen[0].contains("fresh"));
(F) BRANCH crates/api/src/credential_law_tests.rs:229:9 False  seen[1].contains("fresh") && !seen[1].contains("stale"),
(F) BRANCH crates/api/src/credential_law_tests.rs:229:38 False  seen[1].contains("fresh") && !seen[1].contains("stale"),
(F) BRANCH crates/api/src/credential_law_tests.rs:286:28 False  .all(|carried| carried.contains("fresh") && !carried.contains("stale")),
(F) BRANCH crates/api/src/credential_law_tests.rs:444:13 False  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/credential_law_tests.rs:514:9 False  said.contains("same value") && said.contains("Dhan"),
(F) BRANCH crates/api/src/credential_law_tests.rs:514:40 False  said.contains("same value") && said.contains("Dhan"),
(F) BRANCH crates/api/src/credential_law_tests.rs:584:9 False  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:584:38 False  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:632:9 False  let pull::vendor::Transport::Http(shipped) = asked.feed.descriptor().transport else {
(F) BRANCH crates/api/src/credential_law_tests.rs:700:9 False  seen[1].contains("fresh") && seen[2].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:700:38 False  seen[1].contains("fresh") && seen[2].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:720:9 False  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) BRANCH crates/api/src/credential_law_tests.rs:720:38 False  seen[0].contains("stale") && seen[1].contains("fresh"),
(F) LINE crates/api/src/detail.rs:1977  [START line 1978] pool.name()
(F) LINE crates/api/src/detail.rs:2182  [START line 2147] started.elapsed()
(F) LINE crates/api/src/detail.rs:2341  [panic/unreachable line] || -> Result<u32, String> { unreachable!("a fallback never hides the failed check") },
(F) LINE crates/api/src/detail.rs:2373  [panic/unreachable line] |_| -> Result<(), String> { unreachable!("a new root is opened") },
(F) LINE crates/api/src/detail.rs:2379  [panic/unreachable line] |_| -> Result<(), String> { unreachable!("nothing cached here, so nothing refreshes") },
(F) LINE crates/api/src/detail.rs:2389  [panic/unreachable line] || -> Result<u32, String> { unreachable!("the handle is cached, so nothing opens") },
(F) LINE crates/api/src/detail.rs:2438  [panic/unreachable line] |_| -> Result<(), String> { unreachable!("the first call never refreshes") },
(F) LINE crates/api/src/detail.rs:2450  [panic/unreachable line] || -> Result<u32, String> { unreachable!("the second call reuses the handle") },
(F) LINE crates/api/src/detail.rs:2485  [panic/unreachable line] |_| -> Result<(), String> { unreachable!("another root opens, it does not refresh") },
(F) LINE crates/api/src/detail.rs:2501  [panic/unreachable line] |_| -> Result<(), String> { unreachable!("a third root has nothing to refresh") },
(F) LINE crates/api/src/detail.rs:2513  [panic/unreachable line] || -> Result<u32, String> { unreachable!("the prior handle refreshes, never reopens") },
(F) BRANCH crates/api/src/detail.rs:1975:17 False  [START line 1976] pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:1975:36 False  [START line 1976] pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:2321:17 False  [START line 2286] why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2321:40 False  [START line 2286] why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2653:17 False  [START line 2646] assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) BRANCH crates/api/src/detail.rs:2653:37 False  [START line 2646] assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) BRANCH crates/api/src/facets_tests.rs:652:13 False  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:652:38 False  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:1785:13 False  [START line 1777] exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1785:35 False  [START line 1777] exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1785:54 False  [START line 1777] exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:9 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] got.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:26 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] got.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:45 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] got.is_none() && keys.is_empty() && sums.is_empty(),
(F) LINE crates/api/src/feed_checks_tests.rs:197  [panic/unreachable line] panic!("expected a 400 refusal, got {result:?}");
(F) LINE crates/api/src/feed_checks_tests.rs:204  [panic/unreachable line] panic!("expected a 404, got {result:?}");
(F) LINE crates/api/src/feed_checks_tests.rs:211  [panic/unreachable line] panic!("expected a 500 refusal, got {result:?}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:884:13 False  assert!(first > 1970 && last >= first, "{first}..{last}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:884:29 False  assert!(first > 1970 && last >= first, "{first}..{last}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:1085:13 False  why.contains("cut or damaged") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1085:47 False  why.contains("cut or damaged") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1108:13 False  why.contains("is not the CSV its index") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1108:57 False  why.contains("is not the CSV its index") && why.contains("held:"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1151:13 False  why.contains("is not a folder") && why.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1151:48 False  why.contains("is not a folder") && why.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1158:13 False  why.contains("is not a folder") && why.contains("tick-to-second"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1158:48 False  why.contains("is not a folder") && why.contains("tick-to-second"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1185:9 False  gap.contains("2025-02") && gap.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1185:36 False  gap.contains("2025-02") && gap.contains("feed-reconcile"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1418:13 False  why.contains(&format!("line {line}:")) && why.contains("(day, symbol, role) order"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:1418:55 False  why.contains(&format!("line {line}:")) && why.contains("(day, symbol, role) order"),
(F) BRANCH crates/api/src/feed_checks_tests.rs:196:9 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Asked(why)) = result else {
(F) BRANCH crates/api/src/feed_checks_tests.rs:203:9 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Absent(why)) = result else {
(F) BRANCH crates/api/src/feed_checks_tests.rs:210:9 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Refused(why)) = result else {
(F) BRANCH crates/api/src/folder.rs:606:13 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:38 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:62 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) LINE crates/api/src/gap_peer_route_tests.rs:398  panic!("the peer vote was still building after 1,000 asks");
(F) BRANCH crates/api/src/import_journal.rs:1136:28 False  [START line 1065] .all(|end| end.end_line.is_none() && end.fields.len() == 3),
(F) BRANCH crates/api/src/import_journal.rs:1493:17 False  [START line 1374] assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1493:24 False  [START line 1374] assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1493:57 False  [START line 1374] assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/imports_filter_tests.rs:136:24 False  .all(|row| row["filter"] != "*" && row["outcome_from"] == "journal"),
(F) BRANCH crates/api/src/indexmap.rs:912:17 False  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:912:40 False  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:1346:13 False  error.contains("gdfl") && error.contains("no manifest"),
(F) BRANCH crates/api/src/indexmap.rs:1346:39 False  error.contains("gdfl") && error.contains("no manifest"),
(F) BRANCH crates/api/src/merge.rs:1017:13 False  e.universe.contains(Universe::FNO) && e.universe.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/api/src/merge.rs:1017:51 False  e.universe.contains(Universe::FNO) && e.universe.contains(Universe::TOTAL_MARKET),
(F) BRANCH crates/api/src/merge.rs:1042:17 False  assert!(line.contains("groww") && line.contains("dhan"), "{line}");
(F) BRANCH crates/api/src/merge.rs:1042:43 False  assert!(line.contains("groww") && line.contains("dhan"), "{line}");
(F) BRANCH crates/api/src/merge.rs:1055:17 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1055:54 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1339:17 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1339:54 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1379:17 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) BRANCH crates/api/src/merge.rs:1379:54 False  assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
(F) LINE crates/api/src/optiongreeks_route_tests.rs:253  unreachable!("loaded above")
(F) LINE crates/api/src/optiongreeks_route_tests.rs:811  panic!("loaded: {moved:?}")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:252:9 False  let RateTable::Loaded(series) = &rate else {
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:304:17 False  assert!(iv > 0.0 && iv < 1.0, "{iv}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:304:29 False  assert!(iv > 0.0 && iv < 1.0, "{iv}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:451:13 False  detail.contains("rate_carried")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:452:20 False  && detail.contains("2026-08-12")
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:453:20 False  && detail.contains("2026-08-01"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:502:17 False  row.get("iv").is_none() && row.get("delta").is_none(),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:502:44 False  row.get("iv").is_none() && row.get("delta").is_none(),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:505:21 False  assert!(row["premium"].is_i64() && row["spot"].is_i64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:505:48 False  assert!(row["premium"].is_i64() && row["spot"].is_i64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:506:21 False  assert!(row["tenor_s"].is_i64() && row["tenor_y"].is_f64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:506:48 False  assert!(row["tenor_s"].is_i64() && row["tenor_y"].is_f64(), "{row}");
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:621:9 False  detail.contains("ZERO carry") && detail.contains("not a verdict on the print"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:621:42 False  detail.contains("ZERO carry") && detail.contains("not a verdict on the print"),
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:627:9 False  row["tenor_s"].is_i64() && row["above_rate_tenor"] == false,
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:627:36 False  row["tenor_s"].is_i64() && row["above_rate_tenor"] == false,
(F) BRANCH crates/api/src/optiongreeks_route_tests.rs:810:9 False  let RateTable::Loaded(series) = &*moved else {
(F) BRANCH crates/api/src/page_report_route_tests.rs:153:60 False  matches!((asked, read_back), (Some(a), Some(r)) if a < r),
(F) LINE crates/api/src/pullrun.rs:4446  [panic/unreachable line] panic!("a disagreeing leg must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4580  [panic/unreachable line] panic!("a repeated payload field must refuse the run: {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4680  [panic/unreachable line] panic!("a four-part leg must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4729  [panic/unreachable line] panic!("one leg past the bound must be named, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4755  [START line 4757] form.len()
(F) LINE crates/api/src/pullrun.rs:4764  [START line 4766] leg.len()
(F) LINE crates/api/src/pullrun.rs:4791  [panic/unreachable line] panic!("a leg naming no feed must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:5326  [START line 5327] observed(&site).feeds
(F) LINE crates/api/src/pullrun.rs:6987  [START line 6988] restart_in(&unseen)
(F) LINE crates/api/src/pullrun.rs:7310  [START line 7311] run.words()
(F) LINE crates/api/src/pullrun.rs:7396  [START line 7397] panic!("with the lock held nothing is known: {held:?}");
(F) LINE crates/api/src/pullrun.rs:7422  [START line 7423] panic!("an index that cannot be written leaves nothing known: {refused:?}");
(F) LINE crates/api/src/pullrun.rs:7492  [panic/unreachable line] panic!("NOT KNOWN was expected, and the scan said {found:?}");
(F) LINE crates/api/src/pullrun.rs:8597  [panic/unreachable line] &|_site, _leg| -> Answer { unreachable!("a terminal feed is never asked again") },
(F) LINE crates/api/src/pullrun.rs:9782  [panic/unreachable line] panic!("a payload past the bound must refuse the run: {answer:?}");
(F) LINE crates/api/src/pullrun.rs:10879  [START line 10861] panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:10954  [START line 10936] panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:11038  [panic/unreachable line] panic!("expected a guessed answer, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:11091  [START line 11073] panic!("one doubted run: {:?}", answer.doubted);
(F) BRANCH crates/api/src/pullrun.rs:5833:17 False  [START line 5834] assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5833:51 False  [START line 5834] assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5835:17 False  [START line 5836] assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5835:62 False  [START line 5836] assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5865:17 False  [START line 5866] assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5865:49 False  [START line 5866] assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5888:17 False  [START line 5889] assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:5888:61 False  [START line 5889] assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:6049:21 False  [START line 6050] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6049:38 False  [START line 6050] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6124:17 False  [START line 6125] assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6124:37 False  [START line 6125] assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6167:21 False  [START line 6168] assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6167:55 False  [START line 6168] assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6193:17 False  [START line 6194] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6193:34 False  [START line 6194] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6229:17 False  [START line 6230] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6229:34 False  [START line 6230] assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6270:26 False  [START line 6271] .all(|f| f.finished && f.doing.is_empty())
(F) BRANCH crates/api/src/pullrun.rs:6357:13 False  [START line 6358] summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:6358:20 False  [START line 6359] && summary.contains("; 0 bar(s) written by this run's own legs"),
(F) BRANCH crates/api/src/pullrun.rs:6564:13 False  [START line 6565] summary.contains("FINISHED WITH STANDING FAILURES")
(F) BRANCH crates/api/src/pullrun.rs:6565:20 False  [START line 6566] && summary.contains("zerodha (1 leg(s))")
(F) BRANCH crates/api/src/pullrun.rs:6566:20 False  [START line 6567] && summary.contains("is not asked again"),
(F) BRANCH crates/api/src/pullrun.rs:6877:13 False  [START line 6878] words.starts_with(
(F) BRANCH crates/api/src/pullrun.rs:6879:18 False  [START line 6880] ) && words.contains("with 2 member failure(s) on record"),
(F) BRANCH crates/api/src/pullrun.rs:6885:13 False  [START line 6886] !words.contains("The server stopped while it ran")
(F) BRANCH crates/api/src/pullrun.rs:6886:20 False  [START line 6887] && words.contains("unless another process"),
(F) BRANCH crates/api/src/pullrun.rs:7395:17 False  [START line 7396] let Restart::NotKnown(why) = &held else {
(F) BRANCH crates/api/src/pullrun.rs:7399:17 False  [START line 7400] why.contains("no index at")
(F) BRANCH crates/api/src/pullrun.rs:7400:24 False  [START line 7401] && why.contains("another writer held the journal's lock through 2 wait(s)"),
(F) BRANCH crates/api/src/pullrun.rs:7410:17 False  [START line 7411] words.starts_with("NOT KNOWN whether a pull run was left open")
(F) BRANCH crates/api/src/pullrun.rs:7411:24 False  [START line 7412] && !words.contains("INTERRUPTED"),
(F) BRANCH crates/api/src/pullrun.rs:7421:17 False  [START line 7422] let Restart::NotKnown(why) = &refused else {
(F) BRANCH crates/api/src/pullrun.rs:7549:13 False  [START line 7550] why.contains("leave the run opened at record 0 open")
(F) BRANCH crates/api/src/pullrun.rs:7550:20 False  [START line 7551] && why.contains("may be its closing record"),
(F) BRANCH crates/api/src/pullrun.rs:7619:13 False  [START line 7620] why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7619:43 False  [START line 7620] why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7619:73 False  [START line 7620] why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7982:13 False  [START line 7983] words.contains("whose opening record names the same ask")
(F) BRANCH crates/api/src/pullrun.rs:7983:20 False  [START line 7984] && words.contains("a refusal that opened no run closes nothing"),
(F) BRANCH crates/api/src/pullrun.rs:8288:38 False  [START line 8289] .is_some_and(|words| words.starts_with("INTERRUPTED: a pull run")
(F) BRANCH crates/api/src/pullrun.rs:8289:24 False  [START line 8290] && words.contains("run the same pull again from /pull/spot")
(F) BRANCH crates/api/src/pullrun.rs:8456:13 False  [START line 8457] summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:8457:20 False  [START line 8458] && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8514:13 False  [START line 8515] summary.contains("ceiling")
(F) BRANCH crates/api/src/pullrun.rs:8515:20 False  [START line 8516] && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8607:17 False  [START line 8611] assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8607:45 False  [START line 8611] assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8673:17 False  [START line 8677] assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:8673:47 False  [START line 8677] assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:9363:17 False  [START line 9343] assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:9363:44 False  [START line 9343] assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:10878:13 False  [START line 10860] let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:10953:13 False  [START line 10935] let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:11061:17 False  [START line 11043] assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11061:43 False  [START line 11043] assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11090:13 False  [START line 11072] let [doubted] = answer.doubted.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:4445:17 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Disagrees { leg, why }) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4579:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Malformed(why)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4679:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Malformed(named)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4728:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::TooManyLegs(count)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4790:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Malformed(named)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:7491:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Restart::NotKnown(why) = found else {
(F) BRANCH crates/api/src/pullrun.rs:9781:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Err(Refusal::Malformed(why)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:11037:13 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Restart::Guessed(guessed) = answer else {
(F) LINE crates/api/src/pullrun_restart_tests.rs:117  [panic/unreachable line] panic!("expected a guessed NOT KNOWN, got {answer:?}");
(F) LINE crates/api/src/pullrun_restart_tests.rs:376  [START line 374] run.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:634  [panic/unreachable line] panic!("expected a guessed NOT KNOWN, got {answer:?}");
(F) LINE crates/api/src/pullrun_restart_tests.rs:733  [START line 731] answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:927  [START line 925] answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:1024  [START line 1022] answer.words()
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:170:9 False  [START line 168] words.contains("3 runs read have no closing record")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:171:16 False  [START line 169] && words.contains("record 2 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:172:16 False  [START line 170] && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:173:16 False  [START line 171] && words.contains("ag01-three-zerodha"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:534:9 False  [START line 532] why.contains("the closing record at record 4")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:535:16 False  [START line 533] && why.contains("a close written without its run's thread could end either run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:536:16 False  [START line 534] && why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:662:13 False  [START line 660] guessed.why.contains("the closing record at record 2")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:663:20 False  [START line 661] && guessed.why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:681:13 False  [START line 679] words.starts_with("NOT KNOWN which pull runs were left open, though at least 1 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:682:20 False  [START line 680] && words.contains("so 1 of them have no closing record; which is not known"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:778:9 False  [START line 776] words.starts_with("NOT KNOWN which pull runs were left open, though at least 2 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:779:16 False  [START line 777] && words.contains("Open whichever way the guesses are read")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:780:16 False  [START line 778] && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:781:16 False  [START line 779] && words.contains("ag01-p4-kite")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:782:16 False  [START line 780] && words.contains("A guess names the run(s) opened at records 2, 3"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:983:13 False  [START line 981] words.contains("named by no guess, though something read may have closed it")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:984:20 False  [START line 982] && words.contains(&format!(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:987:20 False  [START line 985] && words.contains(CLOSE_UNDECODED)
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:988:20 False  [START line 986] && words.contains("3 older run record(s) that the journal's index names"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1186:9 False  [START line 1184] why.contains(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1188:14 False  [START line 1186] ) && why.contains("closing record at record 900"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1382:13 False  [START line 1380] words.contains("writes no opening record") && words.contains("never reported here"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1382:59 False  [START line 1380] words.contains("writes no opening record") && words.contains("never reported here"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:116:9 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Restart::Guessed(guessed) = answer else {
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:633:9 False  [the else of a let-else that panics, whose line ran] [a line of this change] let Restart::Guessed(guessed) = answer else {
(F) LINE crates/api/src/recovery.rs:2493  panic!("a server that refuses writes resumed, or said nothing");
(F) LINE crates/api/src/recovery.rs:2503  hex(id)
(F) LINE crates/api/src/recovery.rs:2765  attempts
(F) LINE crates/api/src/recovery.rs:2766  .latest
(F) LINE crates/api/src/recovery.rs:2767  .values()
(F) LINE crates/api/src/recovery.rs:2768  .map(|row| row.attempts)
(F) LINE crates/api/src/recovery.rs:2769  .collect::<Vec<_>>()
(F) BRANCH crates/api/src/recovery.rs:2272:17 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:38 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:63 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2506:17 False  line.starts_with(&format!("recovery not resumed: {word}"))
(F) BRANCH crates/api/src/recovery.rs:2507:24 False  && line.contains(&format!("plan {} is IN FLIGHT", hex(id))),
(F) BRANCH crates/api/src/recovery.rs:3323:28 False  .all(|row| row.status == Status::Queued && row.attempts == 0)
(F) BRANCH crates/api/src/recovery_journal.rs:1639:17 False  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1639:46 False  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1683:21 False  error.contains("may be partial")
(F) BRANCH crates/api/src/recovery_journal.rs:1684:28 False  && error.contains("truncation back to 1024 bytes also failed"),
(F) LINE crates/api/src/repair_route.rs:598  panic!("an HTTP descriptor");
(F) BRANCH crates/api/src/repair_route.rs:597:13 False  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:392  panic!("re-armed above");
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:1805  answered.map(|schedule| schedule.is_some())
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:2030  placed.as_ref().ok()
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:391:9 False  let HeadState::Awaiting { deadline, .. } = wrapped.state else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:13 False  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:33 False  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:2028:69 False  matches!(placed, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
(F) LINE crates/api/src/settings_route_tests.rs:561  panic!("the snapshot of a store with a FIFO generation waited");
(F) LINE crates/api/src/settings_route_tests.rs:570  panic!("another store's snapshot waited behind the FIFO");
(F) LINE crates/api/src/settings_route_tests.rs:577  panic!("/settings.json?history=1 waited on the FIFO");
(F) LINE crates/api/src/settings_route_tests.rs:914  path.display()
(F) BRANCH crates/api/src/settings_route_tests.rs:559:9 False  let Some(stuck_snapshot) = within(move || snapshot(&at)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:568:9 False  let Some(clean) = within(move || snapshot(&at)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:575:9 False  let Some(answer) = within(move || super::answer(&at, 1)) else {
(F) BRANCH crates/api/src/settings_route_tests.rs:683:9 False  got.source.contains("is in force") && got.source.contains("would refuse"),
(F) BRANCH crates/api/src/settings_route_tests.rs:683:47 False  got.source.contains("is in force") && got.source.contains("would refuse"),
(F) BRANCH crates/api/src/settings_route_tests.rs:733:34 False  .is_some_and(|holes| holes.starts_with("1 generation number(s) missing")
(F) BRANCH crates/api/src/settings_route_tests.rs:792:9 False  warning.contains("generation 2 was saved") && warning.contains("generation 3"),
(F) BRANCH crates/api/src/settings_route_tests.rs:792:55 False  warning.contains("generation 2 was saved") && warning.contains("generation 3"),
(F) BRANCH crates/api/src/settings_route_tests.rs:812:9 False  warning.contains("did not read back") && warning.contains("built on generation 1"),
(F) BRANCH crates/api/src/settings_route_tests.rs:812:50 False  warning.contains("did not read back") && warning.contains("built on generation 1"),
(F) BRANCH crates/api/src/spot_route_tests.rs:481:9 False  fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
(F) BRANCH crates/api/src/spot_route_tests.rs:481:42 False  fault.contains("2025-05.bin") && fault.contains(&format!("t={t0}:")),
(F) BRANCH crates/api/src/spot_route_tests.rs:531:9 False  bars_route.contains("crate::detail::Named::StoreRead")
(F) BRANCH crates/api/src/spot_route_tests.rs:532:16 False  && bars_route.contains("crate::detail::cancellable_in(named, work).await"),
(F) BRANCH crates/api/src/spot_route_tests.rs:541:9 False  greeks.contains("super::bars_route::cancellable(false, move |cancel| {")
(F) BRANCH crates/api/src/spot_route_tests.rs:542:16 False  && greeks.contains("RateTable::current(std::env::var_os(RATE_ENV))"),
(F) BRANCH crates/api/src/spot_route_tests.rs:550:9 False  body.contains("a spot read") && body.contains("retry"),
(F) BRANCH crates/api/src/spot_route_tests.rs:550:41 False  body.contains("a spot read") && body.contains("retry"),
(F) LINE crates/api/src/stop_tests.rs:567  matches!(
(F) LINE crates/api/src/stop_tests.rs:575  matches!(
(F) LINE crates/api/src/stop_tests.rs:587  panic!("a failed stop future is a stop, not a load");
(F) LINE crates/api/src/stop_tests.rs:603  panic!("a load that panicked is a failure");
(F) BRANCH crates/api/src/stop_tests.rs:203:17 False  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:203:40 False  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:203:65 False  !log_is_free(&root) && !store_is_free(&root) && !port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:332:9 False  store_is_free(&root) && port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:332:33 False  store_is_free(&root) && port_is_free(addr),
(F) BRANCH crates/api/src/stop_tests.rs:354:13 False  assert!(store_is_free(&root) && port_is_free(addr));
(F) BRANCH crates/api/src/stop_tests.rs:354:37 False  assert!(store_is_free(&root) && port_is_free(addr));
(F) BRANCH crates/api/src/stop_tests.rs:586:9 False  let Loading::Stopped(failed) = loaded_or_stopped(|| 7_u8, &mut broken).await else {
(F) BRANCH crates/api/src/stop_tests.rs:641:9 False  race < load && load < awaited,
(F) BRANCH crates/api/src/stop_tests.rs:641:24 False  race < load && load < awaited,
(F) BRANCH crates/api/src/store_route.rs:1951:17 False  [START line 1778] why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:1951:48 False  [START line 1778] why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:2053:13 False  [START line 1880] parts.starts_with("INDEX=")
(F) BRANCH crates/api/src/store_route.rs:2054:20 False  [START line 1881] && parts.contains(",CASH=")
(F) BRANCH crates/api/src/store_route.rs:2055:20 False  [START line 1882] && parts.ends_with(&format!(",FNO={}", fno.header().generation)),
(F) BRANCH crates/api/src/store_route.rs:2075:17 False  [START line 1902] assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:2075:48 False  [START line 1902] assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:2748:13 False  [START line 2575] one.starts_with('"') && one.ends_with('"'),
(F) BRANCH crates/api/src/store_route.rs:2748:37 False  [START line 2575] one.starts_with('"') && one.ends_with('"'),
(F) BRANCH crates/api/src/store_route.rs:1693:13 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] !body.contains(&longest) && !body.contains("tést-作者"),
(F) BRANCH crates/api/src/store_route.rs:1693:41 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] !body.contains(&longest) && !body.contains("tést-作者"),
(F) BRANCH crates/api/src/store_route.rs:2807:13 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] said.starts_with("the import journal could not be read (")
(F) BRANCH crates/api/src/store_route.rs:2808:20 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] && said.ends_with("so whether this census is behind its import is not known"),
(F) BRANCH crates/api/src/store_route.rs:2818:13 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] said.starts_with("behind: ") && said.contains("; the import journal could not be read"),
(F) BRANCH crates/api/src/store_route.rs:2818:45 False  [the failing side of an && operand inside an assert! whose statement ran: either operand false fails the check by itself] [a line of this change] said.starts_with("behind: ") && said.contains("; the import journal could not be read"),
(F) LINE crates/api/tests/pool_load.rs:382  strange.iter().take(5).collect::<Vec<_>>()
(F) LINE crates/api/tests/pool_load.rs:564  other
(F) LINE crates/api/tests/pool_load.rs:565  .iter()
(F) LINE crates/api/tests/pool_load.rs:566  .take(3)
(F) LINE crates/api/tests/pool_load.rs:567  .map(|(s, _, b)| (s, b.chars().take(400).collect::<String>()))
(F) LINE crates/api/tests/pool_load.rs:568  .collect::<Vec<_>>()
(F) LINE crates/api/tests/pool_load.rs:710  &strange[..strange.len().min(3)]
(F) LINE crates/api/tests/pool_load.rs:810  &strange[..strange.len().min(3)]
(F) BRANCH crates/api/tests/pool_load.rs:579:9 False  served > 0 && holds > 0,
(F) BRANCH crates/api/tests/pool_load.rs:579:23 False  served > 0 && holds > 0,
(F) BRANCH crates/api/tests/pool_load.rs:716:17 False  body.contains("log read not admitted") && body.contains("run at once"),
(F) BRANCH crates/api/tests/pool_load.rs:716:59 False  body.contains("log read not admitted") && body.contains("run at once"),
(U) LINE crates/api/src/audit_json.rs:634  [(U) on every target: last_runs asks Journal::page(first + 1, 0, 1): records >= 1, skip 0 and take 1 give wanted == 1 (audit.rs:1982-1987), so it answers one entry or an error, never an empty page] continue;
(U) BRANCH crates/api/src/audit_json.rs:633:17 False  [(U) on every target: last_runs asks Journal::page(first + 1, 0, 1): records >= 1, skip 0 and take 1 give wanted == 1 (audit.rs:1982-1987), so it answers one entry or an error, never an empty page] let Some(entry) = record.into_iter().next() else {
(U) LINE crates/api/src/feed_checks_files.rs:202  [(U) on aarch64-apple-darwin: Metadata::modified on unix (Linux and Darwin) builds the time from stat's st_mtime and st_mtime_nsec, which the kernel keeps in range, so it never refuses on aarch64-apple-darwin; the Err map is a backstop for targets without a modification time] Refusal::Refused(format!(
(U) LINE crates/api/src/feed_checks_files.rs:203  [(U) on aarch64-apple-darwin: Metadata::modified on unix (Linux and Darwin) builds the time from stat's st_mtime and st_mtime_nsec, which the kernel keeps in range, so it never refuses on aarch64-apple-darwin; the Err map is a backstop for targets without a modification time] "`{}` has no modification time: {why}",
(U) LINE crates/api/src/feed_checks_files.rs:204  [(U) on aarch64-apple-darwin: Metadata::modified on unix (Linux and Darwin) builds the time from stat's st_mtime and st_mtime_nsec, which the kernel keeps in range, so it never refuses on aarch64-apple-darwin; the Err map is a backstop for targets without a modification time] dir.display()
(U) LINE crates/api/src/feed_checks_files.rs:205  [(U) on aarch64-apple-darwin: Metadata::modified on unix (Linux and Darwin) builds the time from stat's st_mtime and st_mtime_nsec, which the kernel keeps in range, so it never refuses on aarch64-apple-darwin; the Err map is a backstop for targets without a modification time] ))
(U) LINE crates/api/src/feed_checks_files.rs:206  [(U) on aarch64-apple-darwin: Metadata::modified on unix (Linux and Darwin) builds the time from stat's st_mtime and st_mtime_nsec, which the kernel keeps in range, so it never refuses on aarch64-apple-darwin; the Err map is a backstop for targets without a modification time] })?),
(U) LINE crates/api/src/feed_checks_files.rs:1200  [(U) on every target: file_tick takes a slot from held.slots, which holds only slots made where stocks and members are pushed together, or makes one there and pushes both (feed_checks_files.rs:1180-1196), so both vectors hold it] return Err(format!("month {month}: a share's slot {slot} is not held"));
(U) LINE crates/api/src/feed_checks_files.rs:1233  [(U) on every target: group_by_share walks order = 0..held.stocks.len() over members = held.members, which file_tick pushes in step with stocks, so both answer every slot] return Err(format!(
(U) LINE crates/api/src/feed_checks_files.rs:1234  [(U) on every target: group_by_share walks order = 0..held.stocks.len() over members = held.members, which file_tick pushes in step with stocks, so both answer every slot] "month {}: a share's slot {slot} is not held",
(U) LINE crates/api/src/feed_checks_files.rs:1235  [(U) on every target: group_by_share walks order = 0..held.stocks.len() over members = held.members, which file_tick pushes in step with stocks, so both answer every slot] held.month
(U) LINE crates/api/src/feed_checks_files.rs:1236  [(U) on every target: group_by_share walks order = 0..held.stocks.len() over members = held.members, which file_tick pushes in step with stocks, so both answer every slot] ));
(U) BRANCH crates/api/src/feed_checks_files.rs:119:12 False  [(U) on every target: bump resizes counts to at + 1 on the line before when it is shorter, so get_mut(at) always answers] if let Some(count) = counts.get_mut(at) {
(U) BRANCH crates/api/src/feed_checks_files.rs:685:16 False  [(U) on every target: a row whose share differs from current pushes its stock first (lines 665-683) and the first row's current is None, so stocks.last_mut() always answers] if let Some(stock) = parsed.stocks.last_mut() {
(U) BRANCH crates/api/src/feed_checks_files.rs:1198:9 False  [(U) on every target: file_tick takes a slot from held.slots, which holds only slots made where stocks and members are pushed together, or makes one there and pushes both (feed_checks_files.rs:1180-1196), so both vectors hold it] let (Some(stock), Some(members)) = (held.stocks.get_mut(slot), held.members.get_mut(slot))
(U) BRANCH crates/api/src/feed_checks_files.rs:1232:17 False  [(U) on every target: group_by_share walks order = 0..held.stocks.len() over members = held.members, which file_tick pushes in step with stocks, so both answer every slot] let (Some(stock), Some(rows)) = (held.stocks.get(slot), members.get_mut(slot)) else {
(U) BRANCH crates/api/src/import_journal.rs:417:28 False  [(U) on every target: runs_by_id holds only positions taken as self.runs.len() in the step that then pushes the run (lines 427-431) and runs never shrinks, so every held position answers] && let Some(run) = self.runs.get_mut(start)
(U) BRANCH crates/api/src/import_journal.rs:536:16 False  [(U) on every target: last is [_; 3] (line 294) and ImportKindOrder::of is a position in ImportKind::ALL, [Self; 3], which lists every variant (crates/pull/src/gdfl_import.rs:412-423)] if let Some(last) = self.last.get_mut(slot) {
(U) LINE crates/api/src/merge.rs:711  [(U) on every target: the empty else of the if-let at 694; Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] }
(U) LINE crates/api/src/merge.rs:718  [(U) on every target: the empty else of the if-let at 712; Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] }
(U) BRANCH crates/api/src/merge.rs:694:12 False  [(U) on every target: Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] if let Some(slot) = e.ids.get_mut(vendor as usize) {
(U) BRANCH crates/api/src/merge.rs:712:12 False  [(U) on every target: Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] if let Some(slot) = e.vendor_isins.get_mut(vendor as usize) {
(U) LINE crates/api/src/pullrun.rs:876  [(U) on every target: Feed::ALL holds TrueData and GDFL, whose transport kind is Folder (pull's own test asserts both, crates/pull/src/vendor.rs:6444-6445), so unjournalled_feeds is never empty] return String::new();
(U) BRANCH crates/api/src/pullrun.rs:875:8 True  [(U) on every target: Feed::ALL holds TrueData and GDFL, whose transport kind is Folder (pull's own test asserts both, crates/pull/src/vendor.rs:6444-6445), so unjournalled_feeds is never empty] if names.is_empty() {
(U) LINE crates/api/src/repair_route.rs:167  [(U) on every target: a validated Day lies in 1970..=9999 (pull::session MIN_YEAR, MAX_YEAR), the span YearMonth::new admits (crates/store/src/path.rs:584), so year_month() of a window's day never refuses (its own doc says so, crates/pull/src/session.rs:425-433)] return refused("the window's first month is outside the store's calendar");
(U) BRANCH crates/api/src/repair_route.rs:166:9 False  [(U) on every target: a validated Day lies in 1970..=9999 (pull::session MIN_YEAR, MAX_YEAR), the span YearMonth::new admits (crates/store/src/path.rs:584), so year_month() of a window's day never refuses (its own doc says so, crates/pull/src/session.rs:425-433)] let Ok(first_month) = window.from().year_month() else {
(U) BRANCH crates/api/src/repair_route.rs:255:20 False  [(U) on every target: every planned month is the month of a day of the window (repair_plan::plan, crates/pull/src/repair_plan.rs:450-470), so its first day is a valid Day] if let Ok(first) = Day::new(month.month.year(), month.month.month(), 1) {
(U) LINE crates/api/src/settings_route.rs:331  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] None => Effective {
(U) LINE crates/api/src/settings_route.rs:332  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] value: None,
(U) LINE crates/api/src/settings_route.rs:333  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] source: format!(
(U) LINE crates/api/src/settings_route.rs:334  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] "environment {name} holds {value:?}, which is refused ({refusal}): \
(U) LINE crates/api/src/settings_route.rs:335  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] nothing is in force for {}",
(U) LINE crates/api/src/settings_route.rs:336  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] key.name()
(U) LINE crates/api/src/settings_route.rs:337  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] ),
(U) LINE crates/api/src/settings_route.rs:338  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] origin: "environment",
(U) LINE crates/api/src/settings_route.rs:339  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] },
`````

# fu-AS1.txt.md

Session BASE-COV-API-AS1 (third continuation, token BASE-COV-API-AS1-20261105-4). One fenced block below: every uncovered row of G1's files that is an (F) failing arm of an assert or panic in test code whose statement runs, or a (U) row with its proof in brackets (on every target unless the proof names one).

`````text
BASE-COV-API repair 3, shard AS1 (G1 of SHARDS.md), Route B on code tip 4325492338f45405d8ba4bea05f1e16fd4549a2f, reading whole:api, set whole-api-set-AS1.txt (the kit's 205 coverage files and the 12 test files AS1 added, 217)
G1 files read 88 of 88 (SHARDS.md's 76 and the 12 test files AS1 declared in them): lines 61508/62240 (732 uncovered), branches 4342/5000 (658 uncovered sides)
classified: GAP prod 342 lines 110 sides; GAP test 75 lines 81 sides; GAP ignored 37 lines 0 sides; HOST 51 lines 10 sides; (F) 143 lines 400 sides; (U) 84 lines 57 sides
(F) LINE crates/api/src/assets.rs:1137  default_web_dir().display()
(F) LINE crates/api/src/assets.rs:1453  assets.named().display()
(F) LINE crates/api/src/assets.rs:1566  panic!(
(F) BRANCH crates/api/src/assets.rs:1530:17 False  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1530:46 False  assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
(F) BRANCH crates/api/src/assets.rs:1565:13 False  let Build::Stale { ref newer, by_secs } = *built.build() else {
(F) BRANCH crates/api/src/assets.rs:1582:13 False  note.contains("STALE") && note.contains("does not carry that edit"),
(F) BRANCH crates/api/src/assets.rs:1582:39 False  note.contains("STALE") && note.contains("does not carry that edit"),
(U) LINE crates/api/src/audit.rs:634  [(U) on every target: the wildcard arm of a match on pull::session::DropReason, a foreign #[non_exhaustive] _ => 0,
(F) LINE crates/api/src/audit.rs:3566  panic!("three entries: {page:?}");
(F) LINE crates/api/src/audit.rs:3635  &read[read.len().saturating_sub(40)..]
(F) LINE crates/api/src/audit.rs:4041  kind.label(),
(F) LINE crates/api/src/audit.rs:4042  scope.label(),
(F) LINE crates/api/src/audit.rs:4043  outcome.label()
(F) LINE crates/api/src/audit.rs:5670  panic!(
(F) LINE crates/api/src/audit.rs:6162  panic!("one entry: {page:?}");
(F) BRANCH crates/api/src/audit.rs:3410:13 False  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3410:42 False  empty.failures.is_empty() && empty.balances(),
(F) BRANCH crates/api/src/audit.rs:3565:13 False  let [run, fits, whole] = page.as_slice() else {
(F) BRANCH crates/api/src/audit.rs:4431:13 False  refusal.contains("injected short write")
(F) BRANCH crates/api/src/audit.rs:4432:20 False  && refusal.contains("truncated back to 256 bytes"),
(F) BRANCH crates/api/src/audit.rs:5416:25 False  assert!(p50 <= p99 && p99 <= max, "{tag}: {p50} {p99} {max}");
(F) BRANCH crates/api/src/audit.rs:5416:39 False  assert!(p50 <= p99 && p99 <= max, "{tag}: {p50} {p99} {max}");
(F) BRANCH crates/api/src/audit.rs:5669:13 False  let Some(Err(why)) = &page[0].whole_note else {
(U) LINE crates/api/src/audit_json.rs:634  [(U) on every target: last_runs asks Journal::page(first + 1, 0, 1): records >= 1, skip 0 and take 1 give wanted == 1 (audit.rs:1982-1987), so it answers one entry or an error, never an empty page] continue;
(U) LINE crates/api/src/audit_json.rs:1016  [(U) on every target: census is now.census() (line 1014), and VendorCensusNow::census is const fn returning Some (census_cache.rs 688-690)] || std::sync::Arc::new(FeedRollup::default()),
(U) LINE crates/api/src/audit_json.rs:1041  [(U) on every target: as audit_json.rs:1016 (census is always Some)] || format!("no census row for {} in this build", feed.as_str()),
(U) LINE crates/api/src/audit_json.rs:1054  [(U) on every target: as audit_json.rs:1016 (census is always Some)] || render::json_string(""),
(F) LINE crates/api/src/audit_json.rs:1538  other.as_str()
(F) LINE crates/api/src/audit_json.rs:1840  panic!("counted again for an unchanged census")
(U) BRANCH crates/api/src/audit_json.rs:633:17 False  [(U) on every target: last_runs asks Journal::page(first + 1, 0, 1): records >= 1, skip 0 and take 1 give wanted == 1 (audit.rs:1982-1987), so it answers one entry or an error, never an empty page] let Some(entry) = record.into_iter().next() else {
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
(F) LINE crates/api/src/backtest.rs:2330  broken.to_json()
(F) BRANCH crates/api/src/backtest.rs:2151:13 False  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) BRANCH crates/api/src/backtest.rs:2151:46 False  body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
(F) BRANCH crates/api/src/backtest.rs:2638:17 False  assert!(both.partial_tail && both.refusal.is_some(), "both said");
(F) BRANCH crates/api/src/backtest.rs:2638:38 False  assert!(both.partial_tail && both.refusal.is_some(), "both said");
(U) LINE crates/api/src/bars.rs:749  [(U) on every target: as bars.rs:748:13 False (u64::try_from of a usize is always Ok, so the else never runs)] break;
(U) LINE crates/api/src/bars.rs:2219  [(U) on every target: inside if plans.len() == 1 (2217), plans.first() is Some] return Ok(Vec::new());
(U) LINE crates/api/src/bars.rs:2246  [(U) on every target: every position seek walks is made by SeriesPlan::nth or after (one plan) or by merged's heads, each only through runs.get(run)? (bars_series.rs 1552, 1563, 1571; bars.rs head 2394), and runs is written only by SeriesPlan::build (bars_series.rs 1296 and 1372, in cut_months and cut_days, called only at 1203-1205 from build), so plan.runs.get(at.run) is Some] continue;
(U) LINE crates/api/src/bars.rs:2371  [(U) on every target: best is Some((ts, series)) only for a series whose held head was Some in the loop over heads just above (2359-2365), and nothing writes heads between, so heads.get(series) is Some(Some(..))] break;
(U) LINE crates/api/src/bars.rs:2504  [(U) on every target: run_at ranges over 0..runs where runs is plan.runs.len() taken before the loop (2498-2499); runs is written only by SeriesPlan::build (bars_series.rs 1296, 1372, called at 1203-1205), never by read_stretch or behind_of, so plan.runs.get(run_at) is Some] continue;
(U) LINE crates/api/src/bars.rs:2572  [(U) on every target: a Candidate is pushed only for a series whose rules.get(series) was Some (2492) with series from enumerate over plans (2491), and neither slice changes length after, so (plans.get_mut(series), rules.get(series)) are both Some] continue;
(U) LINE crates/api/src/bars.rs:2575  [(U) on every target: chosen.run is a run_at that indexed plan.runs (2503) when the candidate was made, and runs is written only by SeriesPlan::build, so plan.runs.get(chosen.run) is Some] continue;
(U) LINE crates/api/src/bars.rs:2609  [(U) on every target: slot ranges over 0..plan.slots.len() (2606) and slots is written only by SeriesPlan::build (bars_series.rs 1250 in admit, called at 1182), never by days_covered, so plan.slots.get(slot) is Some] continue;
(F) LINE crates/api/src/bars.rs:2892  found.len()
(F) LINE crates/api/src/bars.rs:3051  compared.get()
(F) LINE crates/api/src/bars.rs:3126  ist_clock(at)
(F) LINE crates/api/src/bars.rs:3679  got.bars[0].chg_why
(F) LINE crates/api/src/bars.rs:4643  panic!("one request took more than 256 looks at its flag");
(U) BRANCH crates/api/src/bars.rs:738:16 False  [(U) on every target: usize is at most 64 bits wide on every Rust target, so u64::try_from of a usize (here skip and skip + take) is always Ok and the pattern always matches] && let (Ok(first), Ok(end)) = (
(U) BRANCH crates/api/src/bars.rs:748:13 False  [(U) on every target: usize is at most 64 bits wide on every Rust target, so u64::try_from(i) of the usize index is always Ok] let Ok(index) = u64::try_from(i) else {
(U) BRANCH crates/api/src/bars.rs:2218:13 False  [(U) on every target: inside if plans.len() == 1 (2217), plans.first() is Some] let Some(plan) = plans.first() else {
(U) BRANCH crates/api/src/bars.rs:2245:13 False  [(U) on every target: every position seek walks is made by SeriesPlan::nth or after (one plan) or by merged's heads, each only through runs.get(run)? (bars_series.rs 1552, 1563, 1571; bars.rs head 2394), and runs is written only by SeriesPlan::build (bars_series.rs 1296 and 1372, in cut_months and cut_days, called only at 1203-1205 from build), so plan.runs.get(at.run) is Some] let Some(run) = plan.runs.get(at.run).copied() else {
(U) BRANCH crates/api/src/bars.rs:2253:20 False  [(U) on every target: index is either unopened.len() - 1 taken right after unopened.push (2249-2251) or one stored by an earlier push; unopened is a local never shortened, so unopened.get_mut(index) is Some] if let Some((_, rows)) = unopened.get_mut(index) {
(U) BRANCH crates/api/src/bars.rs:2256:20 False  [(U) on every target: last is vec![None; plans.len()] (2231) and series < plans.len() was just shown by plans.get_mut(series) being Some at 2242, so last.get_mut(series) is Some] if let Some(slot) = last.get_mut(series) {
(U) BRANCH crates/api/src/bars.rs:2267:16 False  [(U) on every target: last is vec![None; plans.len()] (2231) and series < plans.len() was just shown by plans.get_mut(series) being Some at 2242, so last.get_mut(series) is Some] if let Some(slot) = last.get_mut(series) {
(U) BRANCH crates/api/src/bars.rs:2370:13 False  [(U) on every target: best is Some((ts, series)) only for a series whose held head was Some in the loop over heads just above (2359-2365), and nothing writes heads between, so heads.get(series) is Some(Some(..))] let Some(Some((_, at))) = heads.get(series).copied() else {
(U) BRANCH crates/api/src/bars.rs:2378:16 False  [(U) on every target: heads has one entry per plan, pushed in the loop over plans (2349-2353), and series < heads.len() (P6 above), so both plans.get_mut(series) and heads.get_mut(series) are Some] if let (Some(plan), Some(slot)) = (plans.get_mut(series), heads.get_mut(series)) {
(U) BRANCH crates/api/src/bars.rs:2503:17 False  [(U) on every target: run_at ranges over 0..runs where runs is plan.runs.len() taken before the loop (2498-2499); runs is written only by SeriesPlan::build (bars_series.rs 1296, 1372, called at 1203-1205), never by read_stretch or behind_of, so plan.runs.get(run_at) is Some] let Some(run) = plan.runs.get(run_at as usize).copied() else {
(U) BRANCH crates/api/src/bars.rs:2571:13 False  [(U) on every target: a Candidate is pushed only for a series whose rules.get(series) was Some (2492) with series from enumerate over plans (2491), and neither slice changes length after, so (plans.get_mut(series), rules.get(series)) are both Some] let (Some(plan), Some(series_rules)) = (plans.get_mut(series), rules.get(series)) else {
(U) BRANCH crates/api/src/bars.rs:2574:13 False  [(U) on every target: chosen.run is a run_at that indexed plan.runs (2503) when the candidate was made, and runs is written only by SeriesPlan::build, so plan.runs.get(chosen.run) is Some] let Some(run) = plan.runs.get(chosen.run as usize).copied() else {
(U) BRANCH crates/api/src/bars.rs:2607:17 False  [(U) on every target: slot ranges over 0..plan.slots.len() (2606) and slots is written only by SeriesPlan::build (bars_series.rs 1250 in admit, called at 1182), never by days_covered, so plan.slots.get(slot) is Some] let Some((month, rows)) = plan.slots.get(slot).map(|at| (at.month, at.held.n_valid))
(F) BRANCH crates/api/src/bars.rs:3943:17 False  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:3943:62 False  !body.contains("note_unreadable_records") && !body.contains("telemetry::emit"),
(F) BRANCH crates/api/src/bars.rs:4225:13 False  got.faults[0].contains(" record 73: ")
(F) BRANCH crates/api/src/bars.rs:4226:20 False  && got.faults[0].contains("73 kept rows of 2026-01 in this window would not read"),
(F) BRANCH crates/api/src/bars.rs:4324:17 False  assert!(first.oi_applies && first.v_why.is_none());
(F) BRANCH crates/api/src/bars.rs:4324:37 False  assert!(first.oi_applies && first.v_why.is_none());
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
(F) LINE crates/api/src/calendar_build.rs:930  panic!("a queued key is followed: {joined:?}");
(F) LINE crates/api/src/calendar_build.rs:935  matches!(
(F) LINE crates/api/src/calendar_build.rs:988  panic!("room for build {n}: {joined:?}");
(F) BRANCH crates/api/src/calendar_build.rs:821:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:824:13 False  let Joined::Follow(follow) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:855:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:880:13 False  let Joined::Lead(second) = builds.join(&key("BANKNIFTY"), Some(&stamp)) else {
(F) BRANCH crates/api/src/calendar_build.rs:903:13 False  let Joined::Lead(lead) = builds.join(&key("NIFTY"), None) else {
(F) BRANCH crates/api/src/calendar_build.rs:929:17 False  let Joined::Follow(build) = joined else {
(F) BRANCH crates/api/src/calendar_build.rs:987:17 False  let Joined::Lead(build) = joined else {
(F) BRANCH crates/api/src/catalog.rs:918:17 False  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:918:52 False  assert!(Pill::Fno.admits(Universe::FNO) && !Pill::Fno.admits(Universe::INDEX));
(F) BRANCH crates/api/src/catalog.rs:919:17 False  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:919:61 False  assert!(Pill::Ntm.admits(Universe::TOTAL_MARKET) && !Pill::Ntm.admits(Universe::FNO));
(F) BRANCH crates/api/src/catalog.rs:920:17 False  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(F) BRANCH crates/api/src/catalog.rs:920:56 False  assert!(Pill::Index.admits(Universe::INDEX) && !Pill::Index.admits(Universe::NONE));
(U) LINE crates/api/src/census.rs:582  [(U) on every target: read_back passes line 563 only when the base holds an entry (from < to = base.n_valid), and a part read from a file with entries carries its first entry (read_single and read_partitioned take it from the bytes Manifest::open validated; extend keeps previous.first_entry)] }
(U) LINE crates/api/src/census.rs:667  [(U) on every target: still_holds_last_of runs only when same holds, which needs other.first_entry Some; a part carries its first and last entries together from the same validated bytes (read_single, read_partitioned) or from the appended entries (extend, adding >= 1 whenever its header moved)] return header.n_valid == 0;
(U) LINE crates/api/src/census.rs:670  [(U) on every target: other.header() is a header ManifestHeader::decode accepted (read_region or extend's newest_slot), and decode resolves the version through Layout::for_version (pull/src/manifest.rs:2651), so the same call cannot fail here] return false;
(U) LINE crates/api/src/census.rs:678  [(U) on every target: last_entry Some means other read at least one entry (n_valid >= 1), and n_valid <= its limit (2,097,152 or a u32 capacity) times a stride of at most 128 plus the header region cannot overflow u64] return false;
(U) LINE crates/api/src/census.rs:1733  [(U) on every target: a part read from a file carries its first and last entries together and both are Some whenever it holds an entry (read_single, read_partitioned, extend), so the pair is (Some, Some) or (None, None) with n_valid 0] _ => return None,
(U) LINE crates/api/src/census.rs:1941  [(U) on every target: store::safe_open::ReadRefusal has two variants and found() is None only for ReadRefusal::Io (crates/store/src/safe_open.rs:143-188), which the arm above takes] other => Ok(Err(other.to_string())),
(U) LINE crates/api/src/census.rs:1955  [(U) on Linux x86_64/aarch64 and on macOS: NONBLOCK is Some(0x800) and Some(0x4) there (census.rs:1893-1899); the None arm exists for other targets only] }
(F) LINE crates/api/src/census.rs:2646  got.note()
(F) LINE crates/api/src/census.rs:3189  grid.len()
(U) BRANCH crates/api/src/census.rs:577:16 False  [(U) on every target: read_back passes line 563 only when the base holds an entry (from < to = base.n_valid), and a part read from a file with entries carries its first entry (read_single and read_partitioned take it from the bytes Manifest::open validated; extend keeps previous.first_entry)] if let Some(first) = self.first_entry.as_deref() {
(U) BRANCH crates/api/src/census.rs:666:13 False  [(U) on every target: still_holds_last_of runs only when same holds, which needs other.first_entry Some; a part carries its first and last entries together from the same validated bytes (read_single, read_partitioned) or from the appended entries (extend, adding >= 1 whenever its header moved)] let Some(last) = other.last_entry.as_deref() else {
(U) BRANCH crates/api/src/census.rs:669:13 False  [(U) on every target: other.header() is a header ManifestHeader::decode accepted (read_region or extend's newest_slot), and decode resolves the version through Layout::for_version (pull/src/manifest.rs:2651), so the same call cannot fail here] let Ok(layout) = Layout::for_version(header.format_version) else {
(U) BRANCH crates/api/src/census.rs:677:13 False  [(U) on every target: last_entry Some means other read at least one entry (n_valid >= 1), and n_valid <= its limit (2,097,152 or a u32 capacity) times a stride of at most 128 plus the header region cannot overflow u64] let Some(at) = at else {
(U) BRANCH crates/api/src/census.rs:684:12 True  [(U) on every unix target (Linux and aarch64-apple-darwin): FileId::of is Some(dev, ino) from MetadataExt on unix (census.rs:160-172); its None is the not-unix twin only] if identity.is_none() || identity != self.file {
(U) BRANCH crates/api/src/census.rs:1691:12 True  [(U) on every target: ManifestHeader::decode refuses a stride other than its version's layout (pull/src/manifest.rs decode_with_layout, StrideMismatch), so two decoded headers of one format_version (1690 False) carry one stride] || newest.entry_stride != before.entry_stride
(U) BRANCH crates/api/src/census.rs:1732:25 False  [(U) on every target: a part read from a file carries its first and last entries together and both are Some whenever it holds an entry (read_single, read_partitioned, extend), so the pair is (Some, Some) or (None, None) with n_valid 0] (None, None) if before.n_valid == 0 => {}
(U) BRANCH crates/api/src/census.rs:1952:12 False  [(U) on Linux x86_64/aarch64 and on macOS: NONBLOCK is Some(0x800) and Some(0x4) there (census.rs:1893-1899); the None arm exists for other targets only] if let Some(flags) = NONBLOCK {
(F) BRANCH crates/api/src/census.rs:2764:13 False  note.contains(&(MAX_MANIFEST_BYTES + 1).to_string())
(F) BRANCH crates/api/src/census.rs:2765:20 False  && note.contains(&MAX_MANIFEST_BYTES.to_string()),
(F) BRANCH crates/api/src/census.rs:2824:17 False  why.contains("(char_device)")
(F) BRANCH crates/api/src/census.rs:2825:24 False  && why.contains("a character device, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:17 False  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2836:46 False  why.contains("(symlink)") && why.contains("a symbolic link, not a regular file"),
(F) BRANCH crates/api/src/census.rs:2841:21 False  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:2841:41 False  assert!(census.is_loud() && census.note().contains("not a regular file"));
(F) BRANCH crates/api/src/census.rs:3193:26 False  rows == 0 || filled > 0,
(F) LINE crates/api/src/census_cover_tests.rs:224  panic!("a refusal is unreadable: {other:?}");
(F) LINE crates/api/src/census_cover_tests.rs:493  panic!("not held: {}", census.note());
(F) LINE crates/api/src/census_cover_tests.rs:531  panic!("{segment:?}: not refused: {}", census.note());
(F) LINE crates/api/src/census_cover_tests.rs:710  after.note()
(F) LINE crates/api/src/census_cover_tests.rs:734  after.note()
(F) LINE crates/api/src/census_cover_tests.rs:760  after.note()
(F) LINE crates/api/src/census_cover_tests.rs:916  after.note()
(F) LINE crates/api/src/census_cover_tests.rs:1168  panic!("not refused: {}", census.note());
(F) BRANCH crates/api/src/census_cover_tests.rs:118:13 False  assert!(tail.settled.changes.is_empty() && tail.fresh.changes.is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:118:48 False  assert!(tail.settled.changes.is_empty() && tail.fresh.changes.is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:141:13 False  assert!(tail.settled.held.contains_key(&row) && tail.fresh.held.is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:141:53 False  assert!(tail.settled.held.contains_key(&row) && tail.fresh.held.is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:167:13 False  assert!(none.is_partitioned() && none.parts().is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:167:38 False  assert!(none.is_partitioned() && none.parts().is_empty());
(F) BRANCH crates/api/src/census_cover_tests.rs:223:17 False  let Census::Unreadable { reason, fault } = &other else {
(F) BRANCH crates/api/src/census_cover_tests.rs:236:9 False  refused.starts_with("refused Refused") && refused.contains("is not a directory"),
(F) BRANCH crates/api/src/census_cover_tests.rs:236:51 False  refused.starts_with("refused Refused") && refused.contains("is not a directory"),
(F) BRANCH crates/api/src/census_cover_tests.rs:255:9 False  refused.starts_with("refused Io(FilesystemLoop)")
(F) BRANCH crates/api/src/census_cover_tests.rs:256:16 False  && refused.contains("could not be measured"),
(F) BRANCH crates/api/src/census_cover_tests.rs:301:13 False  assert!(all.is_empty() && all.keeps(&cash, july));
(F) BRANCH crates/api/src/census_cover_tests.rs:301:31 False  assert!(all.is_empty() && all.keeps(&cash, july));
(F) BRANCH crates/api/src/census_cover_tests.rs:492:9 False  let Census::Held { ref manifest } = census.state else {
(F) BRANCH crates/api/src/census_cover_tests.rs:530:13 False  let Census::Unreadable { reason, fault } = &census.state else {
(F) BRANCH crates/api/src/census_cover_tests.rs:1167:13 False  let Census::Unreadable { reason, fault } = &census.state else {
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
(U) LINE crates/api/src/chain.rs:833  [(U) on every target: line 803 pushes (month, ..) whenever no entry of self.opened has month, so the find at 832 always finds one] return Ok(Spot::Unreadable);
(U) LINE crates/api/src/chain.rs:917  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] return bad_request("no census row for this feed");
(U) BRANCH crates/api/src/chain.rs:776:12 False  [(U) on aarch64-apple-darwin and Linux: Metadata::modified is Ok on every unix target (it reads st_mtime), so modified, its .ok(), is never None there] if let Some(modified) = modified {
(U) BRANCH crates/api/src/chain.rs:832:13 False  [(U) on every target: as chain.rs:833 (the month was found or just pushed)] let Some((_, opened)) = self.opened.iter().find(|(held, _)| *held == month) else {
(U) BRANCH crates/api/src/chain.rs:916:9 False  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] let Some(census) = now.census() else {
(F) BRANCH crates/api/src/chain_cover_tests.rs:125:9 False  why.starts_with("unreadable: ") && why.contains("gdfl.lineage"),
(F) BRANCH crates/api/src/chain_cover_tests.rs:125:44 False  why.starts_with("unreadable: ") && why.contains("gdfl.lineage"),
(F) BRANCH crates/api/src/chain_cover_tests.rs:197:13 False  assert!(ladder_now.stored.is_empty() && ladder_now.atm.is_none());
(F) BRANCH crates/api/src/chain_cover_tests.rs:197:45 False  assert!(ladder_now.stored.is_empty() && ladder_now.atm.is_none());
(F) BRANCH crates/api/src/chain_cover_tests.rs:230:9 False  faults.is_empty() && months.opened.is_empty(),
(F) BRANCH crates/api/src/chain_cover_tests.rs:230:30 False  faults.is_empty() && months.opened.is_empty(),
(F) LINE crates/api/src/chain_route_tests.rs:1005  &mut |_, _, _| unreachable!("no listed key"),
(F) LINE crates/api/src/chain_route_tests.rs:1013  unreachable!("no listed key")
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
(F) LINE crates/api/src/credential_law_cover_tests.rs:117  panic!("a value rejected earlier is never sent again")
(F) BRANCH crates/api/src/credential_law_cover_tests.rs:114:9 False  let Reread::Halted(CredentialStop::SameValue, why) =
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
(F) LINE crates/api/src/detail.rs:1977  pool.name()
(F) LINE crates/api/src/detail.rs:2182  started.elapsed()
(F) LINE crates/api/src/detail.rs:2341  || -> Result<u32, String> { unreachable!("a fallback never hides the failed check") },
(F) LINE crates/api/src/detail.rs:2373  |_| -> Result<(), String> { unreachable!("a new root is opened") },
(F) LINE crates/api/src/detail.rs:2379  |_| -> Result<(), String> { unreachable!("nothing cached here, so nothing refreshes") },
(F) LINE crates/api/src/detail.rs:2389  || -> Result<u32, String> { unreachable!("the handle is cached, so nothing opens") },
(F) LINE crates/api/src/detail.rs:2438  |_| -> Result<(), String> { unreachable!("the first call never refreshes") },
(F) LINE crates/api/src/detail.rs:2450  || -> Result<u32, String> { unreachable!("the second call reuses the handle") },
(F) LINE crates/api/src/detail.rs:2485  |_| -> Result<(), String> { unreachable!("another root opens, it does not refresh") },
(F) LINE crates/api/src/detail.rs:2501  |_| -> Result<(), String> { unreachable!("a third root has nothing to refresh") },
(F) LINE crates/api/src/detail.rs:2513  || -> Result<u32, String> { unreachable!("the prior handle refreshes, never reopens") },
(F) BRANCH crates/api/src/detail.rs:1975:17 False  pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:1975:36 False  pool.max() >= 1 && pool.max() <= cores.max(1),
(F) BRANCH crates/api/src/detail.rs:2321:17 False  why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2321:40 False  why.contains(field) && why.contains("canonical"),
(F) BRANCH crates/api/src/detail.rs:2653:17 False  assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) BRANCH crates/api/src/detail.rs:2653:37 False  assert!(!held.is_empty() && held.len() <= super::MAX_OPS_READ_CONCURRENT);
(F) LINE crates/api/src/detail_cover_tests.rs:238  || -> Result<u32, String> { unreachable!("a kept handle is served, never reopened") };
(F) LINE crates/api/src/detail_cover_tests.rs:249  |_| unreachable!("nothing to refresh"),
(F) BRANCH crates/api/src/detail_cover_tests.rs:175:9 False  why.contains("runs.bin at")
(F) BRANCH crates/api/src/detail_cover_tests.rs:176:16 False  && why.contains("could not be measured before its bounded read"),
(U) LINE crates/api/src/facets.rs:932  [(U) on every target: line 921 returns unless serves_as_previous(&held, ..), which is held.reflects.as_ref().is_some_and(..) (line 838); held is the same read guard at 924, so reflects is Some there] None => Err(ms_since(since)),
(U) LINE crates/api/src/facets.rs:1980  [(U) on every target: rows_answer's one caller (answer, line 1632) passes now.census(), and VendorCensusNow::census is const fn returning Some(&self.census) (census_cache.rs 688-690)] None => super::store_wire::compact(&[], &entries, asked.feed),
(U) LINE crates/api/src/facets.rs:1984  [(U) on every target: store_wire::compact is serde_json::to_string(&Compact) (store_wire.rs 210-217); Compact holds a u8, &'static str fields, Vec<String> and a Vec of integer tuples, written to an in-memory Vec writer: serde_json fails only on a map key that is not a string, a non-finite float or a writer error, none of which this type or that writer can produce] Err(why) => {
(U) LINE crates/api/src/facets.rs:1985  [(U) on every target: as facets.rs:1984 (compact cannot fail)] return (
(U) LINE crates/api/src/facets.rs:1986  [(U) on every target: as facets.rs:1984 (compact cannot fail)] Err((
(U) LINE crates/api/src/facets.rs:1987  [(U) on every target: as facets.rs:1984 (compact cannot fail)] axum::http::StatusCode::INTERNAL_SERVER_ERROR,
(U) LINE crates/api/src/facets.rs:1988  [(U) on every target: as facets.rs:1984 (compact cannot fail)] error_json("census_encoding_failed", &why),
(U) LINE crates/api/src/facets.rs:1989  [(U) on every target: as facets.rs:1984 (compact cannot fail)] )),
(U) LINE crates/api/src/facets.rs:1990  [(U) on every target: as facets.rs:1984 (compact cannot fail)] generation,
(U) LINE crates/api/src/facets.rs:1991  [(U) on every target: as facets.rs:1984 (compact cannot fail)] );
(U) BRANCH crates/api/src/facets.rs:757:44 False  [(U) on every target: moved_off_lock answers true whenever part_of(slot, segment) is None (facets.rs 1135-1148: it returns at 1138 or at 1148), so the right operand of the && at 757 is never false when it is evaluated] if part_of(slot, segment).is_none() && moved_off_lock(site, vendor, cell, segment, slot) {
(U) BRANCH crates/api/src/facets.rs:2307:32 False  [(U) on every target: sides is a [Option<&Agg>; 2] if let Some(side) = sides.get_mut(usize::from(!call)) {
(F) BRANCH crates/api/src/facets_tests.rs:652:13 False  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:652:38 False  assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
(F) BRANCH crates/api/src/facets_tests.rs:1785:13 False  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1785:35 False  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1785:54 False  exchange.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:9 False  got.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:26 False  got.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:1866:45 False  got.is_none() && keys.is_empty() && sums.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:2131:13 False  assert!(empty.underlyings.is_empty() && empty.generation == 0);
(F) BRANCH crates/api/src/facets_tests.rs:2131:45 False  assert!(empty.underlyings.is_empty() && empty.generation == 0);
(F) BRANCH crates/api/src/facets_tests.rs:2166:9 False  index
(F) BRANCH crates/api/src/facets_tests.rs:2170:16 False  && index.underlyings.is_empty(),
(F) BRANCH crates/api/src/facets_tests.rs:2254:9 False  body.contains("-9223372036854775808-CE") && body.contains("does not parse back"),
(F) BRANCH crates/api/src/facets_tests.rs:2254:53 False  body.contains("-9223372036854775808-CE") && body.contains("does not parse back"),
(U) LINE crates/api/src/feed_checks.rs:974  [(U) on every target: record_years() maps the compiled calendar record's first day to its civil year, never 0; server::feed_checks::tests::every_day_and_year_the_calendar_holds_is_said_and_a_refused_version_is_a_500 asserts first > 0, and every_year_of_the_record_is_counted_as_the_calendar_counts_it asserts first > 1970] 0
(U) LINE crates/api/src/feed_checks.rs:994  [(U) on every target: calendar_answer reads calendar_year_json only for a year inside record_years(), and every_day_and_year_the_calendar_holds_is_said_and_a_refused_version_is_a_500 asserts calendar::year_exclusions(year).is_ok() for every such year, so no year it answers carries "refused"] return Err(Refusal::Absent(why.to_owned()));
(U) LINE crates/api/src/feed_checks.rs:998  [(U) on every target: line 993 returns unless calendar::year_exclusions(year) is Ok, and that Ok needs the same two Day::new(year, 1, 1) and Day::new(year, 12, 31) calls to succeed (pull/src/calendar.rs:3678-3691), so this let-else cannot fail] return Err(Refusal::Asked(format!(
(U) LINE crates/api/src/feed_checks.rs:999  [(U) on every target: line 993 returns unless calendar::year_exclusions(year) is Ok, and that Ok needs the same two Day::new(year, 1, 1) and Day::new(year, 12, 31) calls to succeed (pull/src/calendar.rs:3678-3691), so this let-else cannot fail] "{year} is not a year this calendar spells"
(U) LINE crates/api/src/feed_checks.rs:1000  [(U) on every target: line 993 returns unless calendar::year_exclusions(year) is Ok, and that Ok needs the same two Day::new(year, 1, 1) and Day::new(year, 12, 31) calls to succeed (pull/src/calendar.rs:3678-3691), so this let-else cannot fail] )));
(U) BRANCH crates/api/src/feed_checks.rs:973:24 True  [(U) on every target: record_years() maps the compiled calendar record's first day to its civil year, never 0; server::feed_checks::tests::every_day_and_year_the_calendar_holds_is_said_and_a_refused_version_is_a_500 asserts first > 0, and every_year_of_the_record_is_counted_as_the_calendar_counts_it asserts first > 1970] let total = if first == 0 {
(U) BRANCH crates/api/src/feed_checks.rs:993:12 True  [(U) on every target: calendar_answer reads calendar_year_json only for a year inside record_years(), and every_day_and_year_the_calendar_holds_is_said_and_a_refused_version_is_a_500 asserts calendar::year_exclusions(year).is_ok() for every such year, so no year it answers carries "refused"] if let Some(why) = counted.get("refused").and_then(Value::as_str) {
(U) BRANCH crates/api/src/feed_checks.rs:997:9 False  [(U) on every target: line 993 returns unless calendar::year_exclusions(year) is Ok, and that Ok needs the same two Day::new(year, 1, 1) and Day::new(year, 12, 31) calls to succeed (pull/src/calendar.rs:3678-3691), so this let-else cannot fail] let (Ok(start), Ok(end)) = (Day::new(year, 1, 1), Day::new(year, 12, 31)) else {
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
(F) LINE crates/api/src/feed_checks_files_cover_tests.rs:255  panic!("a folder under a file is refused");
(F) LINE crates/api/src/feed_checks_files_cover_tests.rs:292  panic!("a file is not a folder");
(F) BRANCH crates/api/src/feed_checks_files_cover_tests.rs:254:9 False  let Err(Refusal::Refused(why)) = feed_listing(&file.join("under")) else {
(F) BRANCH crates/api/src/feed_checks_files_cover_tests.rs:291:9 False  let Err(Refusal::Refused(why)) = feed_listing(&named) else {
(F) LINE crates/api/src/feed_checks_tests.rs:197  panic!("expected a 400 refusal, got {result:?}");
(F) LINE crates/api/src/feed_checks_tests.rs:204  panic!("expected a 404, got {result:?}");
(F) LINE crates/api/src/feed_checks_tests.rs:211  panic!("expected a 500 refusal, got {result:?}");
(F) BRANCH crates/api/src/feed_checks_tests.rs:196:9 False  let Err(Refusal::Asked(why)) = result else {
(F) BRANCH crates/api/src/feed_checks_tests.rs:203:9 False  let Err(Refusal::Absent(why)) = result else {
(F) BRANCH crates/api/src/feed_checks_tests.rs:210:9 False  let Err(Refusal::Refused(why)) = result else {
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
(F) BRANCH crates/api/src/folder.rs:606:13 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:38 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) BRANCH crates/api/src/folder.rs:606:62 False  why.contains("INDEX") && why.contains("CASH") && why.contains("FNO"),
(F) LINE crates/api/src/gap_peer_route_tests.rs:398  panic!("the peer vote was still building after 1,000 asks");
(U) BRANCH crates/api/src/import_journal.rs:417:28 False  [(U) on every target: runs_by_id holds only positions taken as self.runs.len() in the step that then pushes the run (lines 427-431) and runs never shrinks, so every held position answers] && let Some(run) = self.runs.get_mut(start)
(U) BRANCH crates/api/src/import_journal.rs:536:16 False  [(U) on every target: last is [_; 3] if let Some(last) = self.last.get_mut(slot) {
(F) BRANCH crates/api/src/import_journal.rs:1136:28 False  .all(|end| end.end_line.is_none() && end.fields.len() == 3),
(F) BRANCH crates/api/src/import_journal.rs:1493:17 False  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1493:24 False  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/import_journal.rs:1493:57 False  assert!(cut && kept.len() <= MAX_VALUE_BYTES && kept.is_char_boundary(kept.len()));
(F) BRANCH crates/api/src/imports_filter_tests.rs:136:24 False  .all(|row| row["filter"] != "*" && row["outcome_from"] == "journal"),
(U) LINE crates/api/src/indexmap.rs:638  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] return census_refused(&format!(
(U) LINE crates/api/src/indexmap.rs:639  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] "{}: the census read returned no row for this feed",
(U) LINE crates/api/src/indexmap.rs:640  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] feed.as_str()
(U) LINE crates/api/src/indexmap.rs:641  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] ));
(U) LINE crates/api/src/indexmap.rs:671  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] return Err(format!(
(U) LINE crates/api/src/indexmap.rs:672  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] "{}: the census read returned no row for this feed",
(U) LINE crates/api/src/indexmap.rs:673  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] feed.as_str()
(U) LINE crates/api/src/indexmap.rs:674  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] ));
(U) BRANCH crates/api/src/indexmap.rs:592:16 False  [(U) on every target: slots is a Mutex<[Kept; Vendor::ALL.len()]> and slot is census.vendor as usize; Vendor declares five variants with implicit discriminants 0-4 (core/src/vendor.rs:52-80) and ALL lists all five in that order, so get_mut(slot) is Some; indexmap::tests::a_name_of_nothing_.. asserts every vendor's place in ALL is its discriminant] if let Some(held) = self
(U) BRANCH crates/api/src/indexmap.rs:637:9 False  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] let Some(census) = now.census() else {
(U) BRANCH crates/api/src/indexmap.rs:670:9 False  [(U) on every target: VendorCensusNow::census() is a const fn returning Some(&self.census) always (census_cache.rs:688-690, D-3327)] let Some(census) = now.census() else {
(F) BRANCH crates/api/src/indexmap.rs:912:17 False  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:912:40 False  assert!(banknifty.is_some() && nifty.is_some());
(F) BRANCH crates/api/src/indexmap.rs:1346:13 False  error.contains("gdfl") && error.contains("no manifest"),
(F) BRANCH crates/api/src/indexmap.rs:1346:39 False  error.contains("gdfl") && error.contains("no manifest"),
(U) LINE crates/api/src/merge.rs:711  [(U) on every target: the empty else of the if-let at 694; Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] }
(U) LINE crates/api/src/merge.rs:718  [(U) on every target: the empty else of the if-let at 712; Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] }
(U) BRANCH crates/api/src/merge.rs:694:12 False  [(U) on every target: Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] if let Some(slot) = e.ids.get_mut(vendor as usize) {
(U) BRANCH crates/api/src/merge.rs:712:12 False  [(U) on every target: Entry::ids and Entry::vendor_isins are arrays of Vendor::ALL.len() (merge.rs:90, 92) and Vendor is a fieldless enum of exactly the five variants Vendor::ALL lists, default discriminants 0..=4 (crates/core/src/vendor.rs:52-98), so vendor as usize always indexes inside] if let Some(slot) = e.vendor_isins.get_mut(vendor as usize) {
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
(F) BRANCH crates/api/src/merge_cover_tests.rs:74:13 False  assert!(!ambiguous(&out, Vendor::Groww) && out.conflicts.is_empty());
(F) BRANCH crates/api/src/merge_cover_tests.rs:74:48 False  assert!(!ambiguous(&out, Vendor::Groww) && out.conflicts.is_empty());
(F) BRANCH crates/api/src/merge_cover_tests.rs:152:13 False  assert!(ambiguous(&raw, Vendor::Dhan) && id_of(&raw, Vendor::Dhan).is_none());
(F) BRANCH crates/api/src/merge_cover_tests.rs:152:46 False  assert!(ambiguous(&raw, Vendor::Dhan) && id_of(&raw, Vendor::Dhan).is_none());
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
(U) LINE crates/api/src/pullrun.rs:876  [(U) on every target: Feed::ALL holds TrueData and GDFL, whose transport kind is Folder (pull's own test asserts both, crates/pull/src/vendor.rs:6444-6445), so unjournalled_feeds is never empty] return String::new();
(U) LINE crates/api/src/pullrun.rs:1726  [(U) on every target: every Guess is pushed with at: self.position (pullrun.rs 1804-1808, 1980-1984), and walk sets walker.position to the enumerate index over the same read it is handed (1675-1676) before the guesses are read here, so read.get(*at) is Some] continue;
(U) LINE crates/api/src/pullrun.rs:2093  [(U) on every target: the loop runs while skip < INTERRUPTED_SCAN.min(records), and Journal::page(records, skip, MAX_PAGE_RECORDS) reads wanted = min(records - skip, MAX_PAGE_RECORDS) >= 1 records (audit.rs 1982-1985) and returns one Entry per record read (2017-2037) or the read error (2006), never an empty page] break;
(F) LINE crates/api/src/pullrun.rs:4446  panic!("a disagreeing leg must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4580  panic!("a repeated payload field must refuse the run: {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4680  panic!("a four-part leg must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4729  panic!("one leg past the bound must be named, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:4755  form.len()
(F) LINE crates/api/src/pullrun.rs:4764  leg.len()
(F) LINE crates/api/src/pullrun.rs:4791  panic!("a leg naming no feed must be refused, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:5326  observed(&site).feeds
(F) LINE crates/api/src/pullrun.rs:6987  restart_in(&unseen)
(F) LINE crates/api/src/pullrun.rs:7310  run.words()
(F) LINE crates/api/src/pullrun.rs:7396  panic!("with the lock held nothing is known: {held:?}");
(F) LINE crates/api/src/pullrun.rs:7422  panic!("an index that cannot be written leaves nothing known: {refused:?}");
(F) LINE crates/api/src/pullrun.rs:7492  panic!("NOT KNOWN was expected, and the scan said {found:?}");
(F) LINE crates/api/src/pullrun.rs:8753  &|_site, _leg| -> Answer { unreachable!("a terminal feed is never asked again") },
(F) LINE crates/api/src/pullrun.rs:9938  panic!("a payload past the bound must refuse the run: {answer:?}");
(F) LINE crates/api/src/pullrun.rs:11035  panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:11110  panic!("one counted pairing: {:?}", found.left_open);
(F) LINE crates/api/src/pullrun.rs:11194  panic!("expected a guessed answer, got {answer:?}");
(F) LINE crates/api/src/pullrun.rs:11247  panic!("one doubted run: {:?}", answer.doubted);
(U) BRANCH crates/api/src/pullrun.rs:875:8 True  [(U) on every target: Feed::ALL holds TrueData and GDFL, whose transport kind is Folder (pull's own test asserts both, crates/pull/src/vendor.rs:6444-6445), so unjournalled_feeds is never empty] if names.is_empty() {
(U) BRANCH crates/api/src/pullrun.rs:1725:13 False  [(U) on every target: every Guess is pushed with at: self.position (pullrun.rs 1804-1808, 1980-1984), and walk sets walker.position to the enumerate index over the same read it is handed (1675-1676) before the guesses are read here, so read.get(*at) is Some] let Some((_, made)) = read.get(*at) else {
(U) BRANCH crates/api/src/pullrun.rs:2092:12 True  [(U) on every target: the loop runs while skip < INTERRUPTED_SCAN.min(records), and Journal::page(records, skip, MAX_PAGE_RECORDS) reads wanted = min(records - skip, MAX_PAGE_RECORDS) >= 1 records (audit.rs 1982-1985) and returns one Entry per record read (2017-2037) or the read error (2006), never an empty page] if page.is_empty() {
(U) BRANCH crates/api/src/pullrun.rs:4049:16 False  [(U) on every target: nth comes from enumerate over groups.iter().zip(outcomes.iter_mut()).zip(checkpoints) (3979-3983), so nth < outcomes.len() and outcomes.get_mut(nth) is Some] if let Some(held) = outcomes.get_mut(nth) {
(F) BRANCH crates/api/src/pullrun.rs:4445:17 False  let Err(Refusal::Disagrees { leg, why }) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4579:13 False  let Err(Refusal::Malformed(why)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4679:13 False  let Err(Refusal::Malformed(named)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4728:13 False  let Err(Refusal::TooManyLegs(count)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:4790:13 False  let Err(Refusal::Malformed(named)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:5833:17 False  assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5833:51 False  assert!(summary.contains("INCOMPLETE") && summary.contains("zerodha"));
(F) BRANCH crates/api/src/pullrun.rs:5835:17 False  assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5835:62 False  assert!(!summary.contains("asked for everything") && !summary.contains("no more data"));
(F) BRANCH crates/api/src/pullrun.rs:5865:17 False  assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5865:49 False  assert!(summary.contains("HTTP 422") && summary.contains("PERMANENT REFUSAL"));
(F) BRANCH crates/api/src/pullrun.rs:5888:17 False  assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:5888:61 False  assert!(summary.contains("Idle retries stopped") && summary.contains("Empty responses"));
(F) BRANCH crates/api/src/pullrun.rs:6049:21 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6049:38 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6124:17 False  assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6124:37 False  assert!(healthy.finished && healthy.last_error.is_none());
(F) BRANCH crates/api/src/pullrun.rs:6167:21 False  assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6167:55 False  assert!(summary.contains("INCOMPLETE") && summary.contains(&format!("HTTP {code}")));
(F) BRANCH crates/api/src/pullrun.rs:6193:17 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6193:34 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6229:17 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6229:34 False  assert!(feed.finished && feed.doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:6270:26 False  .all(|f| f.finished && f.doing.is_empty())
(F) BRANCH crates/api/src/pullrun.rs:6357:13 False  summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:6358:20 False  && summary.contains("; 0 bar(s) written by this run's own legs"),
(F) BRANCH crates/api/src/pullrun.rs:6564:13 False  summary.contains("FINISHED WITH STANDING FAILURES")
(F) BRANCH crates/api/src/pullrun.rs:6565:20 False  && summary.contains("zerodha (1 leg(s))")
(F) BRANCH crates/api/src/pullrun.rs:6566:20 False  && summary.contains("is not asked again"),
(F) BRANCH crates/api/src/pullrun.rs:6877:13 False  words.starts_with(
(F) BRANCH crates/api/src/pullrun.rs:6879:18 False  ) && words.contains("with 2 member failure(s) on record"),
(F) BRANCH crates/api/src/pullrun.rs:6885:13 False  !words.contains("The server stopped while it ran")
(F) BRANCH crates/api/src/pullrun.rs:6886:20 False  && words.contains("unless another process"),
(F) BRANCH crates/api/src/pullrun.rs:7395:17 False  let Restart::NotKnown(why) = &held else {
(F) BRANCH crates/api/src/pullrun.rs:7399:17 False  why.contains("no index at")
(F) BRANCH crates/api/src/pullrun.rs:7400:24 False  && why.contains("another writer held the journal's lock through 2 wait(s)"),
(F) BRANCH crates/api/src/pullrun.rs:7410:17 False  words.starts_with("NOT KNOWN whether a pull run was left open")
(F) BRANCH crates/api/src/pullrun.rs:7411:24 False  && !words.contains("INTERRUPTED"),
(F) BRANCH crates/api/src/pullrun.rs:7421:17 False  let Restart::NotKnown(why) = &refused else {
(F) BRANCH crates/api/src/pullrun.rs:7491:13 False  let Restart::NotKnown(why) = found else {
(F) BRANCH crates/api/src/pullrun.rs:7549:13 False  why.contains("leave the run opened at record 0 open")
(F) BRANCH crates/api/src/pullrun.rs:7550:20 False  && why.contains("may be its closing record"),
(F) BRANCH crates/api/src/pullrun.rs:7619:13 False  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7619:43 False  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7619:73 False  why.contains("record 2 (") && why.contains("record 4 (") && !why.contains("record 5 ("),
(F) BRANCH crates/api/src/pullrun.rs:7982:13 False  words.contains("whose opening record names the same ask")
(F) BRANCH crates/api/src/pullrun.rs:7983:20 False  && words.contains("a refusal that opened no run closes nothing"),
(F) BRANCH crates/api/src/pullrun.rs:8288:38 False  .is_some_and(|words| words.starts_with("INTERRUPTED: a pull run")
(F) BRANCH crates/api/src/pullrun.rs:8289:24 False  && words.contains("run the same pull again from /pull/spot")
(F) BRANCH crates/api/src/pullrun.rs:8456:13 False  summary.contains("Idle retries stopped")
(F) BRANCH crates/api/src/pullrun.rs:8457:20 False  && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8514:13 False  summary.contains("ceiling")
(F) BRANCH crates/api/src/pullrun.rs:8515:20 False  && summary.contains("Full basket coverage has not been verified")
(F) BRANCH crates/api/src/pullrun.rs:8763:17 False  assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8763:45 False  assert!(why.contains("HTTP 422") && !why.contains("HTTP 503"));
(F) BRANCH crates/api/src/pullrun.rs:8829:17 False  assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:8829:47 False  assert!(progress.feeds[1].finished && progress.feeds[1].doing.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:9519:17 False  assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:9519:44 False  assert!(!feed.store_unavailable && !feed.credential_dead);
(F) BRANCH crates/api/src/pullrun.rs:9937:13 False  let Err(Refusal::Malformed(why)) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:11034:13 False  let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:11109:13 False  let [counted] = found.left_open.as_slice() else {
(F) BRANCH crates/api/src/pullrun.rs:11193:13 False  let Restart::Guessed(guessed) = answer else {
(F) BRANCH crates/api/src/pullrun.rs:11217:17 False  assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11217:43 False  assert!(answer.sure.is_empty() && answer.doubted.is_empty());
(F) BRANCH crates/api/src/pullrun.rs:11246:13 False  let [doubted] = answer.doubted.as_slice() else {
(F) LINE crates/api/src/pullrun_restart_tests.rs:117  panic!("expected a guessed NOT KNOWN, got {answer:?}");
(F) LINE crates/api/src/pullrun_restart_tests.rs:376  run.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:634  panic!("expected a guessed NOT KNOWN, got {answer:?}");
(F) LINE crates/api/src/pullrun_restart_tests.rs:733  answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:927  answer.words()
(F) LINE crates/api/src/pullrun_restart_tests.rs:1024  answer.words()
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:116:9 False  let Restart::Guessed(guessed) = answer else {
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:170:9 False  words.contains("3 runs read have no closing record")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:171:16 False  && words.contains("record 2 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:172:16 False  && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:173:16 False  && words.contains("ag01-three-zerodha"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:534:9 False  why.contains("the closing record at record 4")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:535:16 False  && why.contains("a close written without its run's thread could end either run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:536:16 False  && why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:633:9 False  let Restart::Guessed(guessed) = answer else {
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:662:13 False  guessed.why.contains("the closing record at record 2")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:663:20 False  && guessed.why.contains("opened at records 1, 0"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:681:13 False  words.starts_with("NOT KNOWN which pull runs were left open, though at least 1 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:682:20 False  && words.contains("so 1 of them have no closing record; which is not known"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:778:9 False  words.starts_with("NOT KNOWN which pull runs were left open, though at least 2 run(s)")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:779:16 False  && words.contains("Open whichever way the guesses are read")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:780:16 False  && words.contains("record 0 (a pull run")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:781:16 False  && words.contains("ag01-p4-kite")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:782:16 False  && words.contains("A guess names the run(s) opened at records 2, 3"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:983:13 False  words.contains("named by no guess, though something read may have closed it")
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:984:20 False  && words.contains(&format!(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:987:20 False  && words.contains(CLOSE_UNDECODED)
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:988:20 False  && words.contains("3 older run record(s) that the journal's index names"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1186:9 False  why.contains(
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1188:14 False  ) && why.contains("closing record at record 900"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1382:13 False  words.contains("writes no opening record") && words.contains("never reported here"),
(F) BRANCH crates/api/src/pullrun_restart_tests.rs:1382:59 False  words.contains("writes no opening record") && words.contains("never reported here"),
(U) LINE crates/api/src/recovery.rs:769  [(U) on every target: active_history returns Ok over an existing pointer file only with a non-empty latest (recovery.rs 266-268), and an absent file returned early (764, 809), so max_by_key over latest is Some] return Ok(None);
(U) LINE crates/api/src/recovery.rs:773  [(U) on every target: active_history refused every row whose key is not pointer_identity(&row.body) (recovery.rs 273), so for the row it returned, pointer.key == pointer_identity(&pointer.body)] return Err("active recovery identity mismatch".to_owned());
(U) LINE crates/api/src/recovery.rs:814  [(U) on every target: active_history returns Ok over an existing pointer file only with a non-empty latest (recovery.rs 266-268), and an absent file returned early (764, 809), so max_by_key over latest is Some] return Ok(false);
(U) LINE crates/api/src/recovery.rs:818  [(U) on every target: active_history refused every row whose key is not pointer_identity(&row.body) (recovery.rs 273), so for the row it returned, pointer.key == pointer_identity(&pointer.body)] return Err("active recovery identity mismatch".to_owned());
(F) LINE crates/api/src/recovery.rs:2493  panic!("a server that refuses writes resumed, or said nothing");
(F) LINE crates/api/src/recovery.rs:2503  hex(id)
(F) LINE crates/api/src/recovery.rs:2762  attempts
(F) LINE crates/api/src/recovery.rs:2763  .latest
(F) LINE crates/api/src/recovery.rs:2764  .values()
(F) LINE crates/api/src/recovery.rs:2765  .map(|row| row.attempts)
(F) LINE crates/api/src/recovery.rs:2766  .collect::<Vec<_>>()
(U) BRANCH crates/api/src/recovery.rs:768:9 False  [(U) on every target: active_history returns Ok over an existing pointer file only with a non-empty latest (recovery.rs 266-268), and an absent file returned early (764, 809), so max_by_key over latest is Some] let Some(pointer) = active.latest.values().max_by_key(|record| record.attempts) else {
(U) BRANCH crates/api/src/recovery.rs:772:8 True  [(U) on every target: active_history refused every row whose key is not pointer_identity(&row.body) (recovery.rs 273), so for the row it returned, pointer.key == pointer_identity(&pointer.body)] if pointer.key != id {
(U) BRANCH crates/api/src/recovery.rs:813:9 False  [(U) on every target: active_history returns Ok over an existing pointer file only with a non-empty latest (recovery.rs 266-268), and an absent file returned early (764, 809), so max_by_key over latest is Some] let Some(pointer) = active.latest.values().max_by_key(|record| record.attempts) else {
(U) BRANCH crates/api/src/recovery.rs:817:8 True  [(U) on every target: active_history refused every row whose key is not pointer_identity(&row.body) (recovery.rs 273), so for the row it returned, pointer.key == pointer_identity(&pointer.body)] if pointer.key != id {
(U) BRANCH crates/api/src/recovery.rs:1584:27 True  [(U) on every target: the shape is classified over an empty slice (1573-1582), and the ledger's invalid_timestamps counts only the stamps it is handed (pull/src/gaps.rs 796-799), so it is 0] if shape.truncated || shape.invalid_timestamps > 0 {
(F) BRANCH crates/api/src/recovery.rs:2272:17 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:38 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2272:63 False  assert!(plan_records > 40 && attempt_records == 25 && active_records == 1);
(F) BRANCH crates/api/src/recovery.rs:2506:17 False  line.starts_with(&format!("recovery not resumed: {word}"))
(F) BRANCH crates/api/src/recovery.rs:2507:24 False  && line.contains(&format!("plan {} is IN FLIGHT", hex(id))),
(F) BRANCH crates/api/src/recovery.rs:3320:28 False  .all(|row| row.status == Status::Queued && row.attempts == 0)
(F) LINE crates/api/src/recovery_cover_tests.rs:704  panic!("{day} is an open day of the calendar of record");
(F) LINE crates/api/src/recovery_cover_tests.rs:1876  unreachable!("no driver runs over a refused plan")
(F) BRANCH crates/api/src/recovery_cover_tests.rs:701:9 False  let pull::calendar::DayKind::Open(session) =
(U) LINE crates/api/src/recovery_journal.rs:192  [(U) on every target: put writes 8 bytes at UNVERIFIED_OFFSET 856 into a RECORD_LEN 1,024 image, so the range is always in bounds] )?;
(U) LINE crates/api/src/recovery_journal.rs:386  [(U) on every target: self.bytes is the journal's length, at most a file length (st_size is a signed 64-bit off_t), so adding one RECORD_LEN of 1,024 cannot overflow u64] io::Error::other("recovery journal length cannot fit its next record")
(U) LINE crates/api/src/recovery_journal.rs:387  [(U) on every target: self.bytes is the journal's length, at most a file length (st_size is a signed 64-bit off_t), so adding one RECORD_LEN of 1,024 cannot overflow u64] })?;
(F) BRANCH crates/api/src/recovery_journal.rs:1639:17 False  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1639:46 False  assert!(journal.latest.is_empty() && journal.order.is_empty());
(F) BRANCH crates/api/src/recovery_journal.rs:1683:21 False  error.contains("may be partial")
(F) BRANCH crates/api/src/recovery_journal.rs:1684:28 False  && error.contains("truncation back to 1024 bytes also failed"),
(F) LINE crates/api/src/recovery_journal_cover_tests.rs:57  panic!("a FIFO is no journal");
(F) BRANCH crates/api/src/recovery_journal_cover_tests.rs:56:9 False  let Err(error) = Journal::open(&fifo) else {
(U) LINE crates/api/src/repair_route.rs:167  [(U) on every target: a validated Day lies in 1970..=9999 (pull::session MIN_YEAR, MAX_YEAR), the span YearMonth::new admits (crates/store/src/path.rs:584), so year_month() of a window's day never refuses (its own doc says so, crates/pull/src/session.rs:425-433)] return refused("the window's first month is outside the store's calendar");
(F) LINE crates/api/src/repair_route.rs:598  panic!("an HTTP descriptor");
(U) BRANCH crates/api/src/repair_route.rs:166:9 False  [(U) on every target: a validated Day lies in 1970..=9999 (pull::session MIN_YEAR, MAX_YEAR), the span YearMonth::new admits (crates/store/src/path.rs:584), so year_month() of a window's day never refuses (its own doc says so, crates/pull/src/session.rs:425-433)] let Ok(first_month) = window.from().year_month() else {
(U) BRANCH crates/api/src/repair_route.rs:255:20 False  [(U) on every target: every planned month is the month of a day of the window (repair_plan::plan, crates/pull/src/repair_plan.rs:450-470), so its first day is a valid Day] if let Ok(first) = Day::new(month.month.year(), month.month.month(), 1) {
(F) BRANCH crates/api/src/repair_route.rs:597:13 False  let Transport::Http(spec) = Feed::Zerodha.descriptor().transport else {
(F) BRANCH crates/api/src/repair_route_cover_tests.rs:289:9 False  said.starts_with("cannot read ") && said.ends_with("zerodha.repairs"),
(F) BRANCH crates/api/src/repair_route_cover_tests.rs:289:45 False  said.starts_with("cannot read ") && said.ends_with("zerodha.repairs"),
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:392  panic!("re-armed above");
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:1805  answered.map(|schedule| schedule.is_some())
(F) LINE crates/api/src/server_cloud_mutant_tests_a.rs:2030  placed.as_ref().ok()
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:391:9 False  let HeadState::Awaiting { deadline, .. } = wrapped.state else {
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:13 False  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:491:33 False  assert!(!note.is_empty() && note != "xyzzy", "{note}");
(F) BRANCH crates/api/src/server_cloud_mutant_tests_a.rs:2028:69 False  matches!(placed, Err(Unlocated::NotHeld(ref unreadable)) if unreadable.is_empty()),
(U) LINE crates/api/src/settings_route.rs:213  [(U) by the compiled feed table: some feed's descriptor records a window (Feed::ALL find_map at 197-200), so window is never None; asserted by server::settings_route::tests::a_save_names_the_key_it_moved_and_a_reader_that_has_seen_further_is_kept (default_of(WindowFrom) and default_of(WindowTo) both Some), which turns red if the table loses its window] None => (None, "no window is recorded in this build".to_owned()),
(U) LINE crates/api/src/settings_route.rs:331  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] None => Effective {
(U) LINE crates/api/src/settings_route.rs:332  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] value: None,
(U) LINE crates/api/src/settings_route.rs:333  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] source: format!(
(U) LINE crates/api/src/settings_route.rs:334  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] "environment {name} holds {value:?}, which is refused ({refusal}): \
(U) LINE crates/api/src/settings_route.rs:335  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] nothing is in force for {}",
(U) LINE crates/api/src/settings_route.rs:336  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] key.name()
(U) LINE crates/api/src/settings_route.rs:337  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] ),
(U) LINE crates/api/src/settings_route.rs:338  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] origin: "environment",
(U) LINE crates/api/src/settings_route.rs:339  [(U) on every target: only Key::SourceMasters names an environment variable (crates/store/src/settings.rs:178-183) and fallback_reader_of(Key::SourceMasters) is Some (settings_route.rs:377-382), so the None arm under an environment value is never taken] },
(U) LINE crates/api/src/settings_route.rs:773  [(U) by the compiled feed table: as settings_route.rs:772] {
(U) LINE crates/api/src/settings_route.rs:774  [(U) by the compiled feed table: as settings_route.rs:772] fields.push(field);
(U) BRANCH crates/api/src/settings_route.rs:592:28 False  [(U) on every target: generation_json returns serde_json::json!({..}) (settings_route.rs 450-461), always an Object, so as_object_mut is always Some] if let Some(object) = row.as_object_mut() {
(U) BRANCH crates/api/src/settings_route.rs:726:12 False  [(U) by the compiled feed table: both window ends always carry a value (a saved one, else the recorded default, settings_route.rs:213's proof), so the let-chain's pattern always matches; asserted by the same test] if let (Some(first), Some(last)) = (&from.value, &to.value)
(U) BRANCH crates/api/src/settings_route.rs:733:20 False  [(U) on every target: Key::slot is 0 to 10 and the array has Key::ALL.len() = KEYS = 11 slots (crates/store/src/settings.rs:132-200), so get_mut always answers] if let Some(slot) = unset.get_mut(key.slot()) {
(U) BRANCH crates/api/src/settings_route.rs:772:20 True  [(U) by the compiled feed table: every HTTP feed's key_field is one of CHARTER_FIELDS, so contains is always true; asserted by server::settings_route::tests::every_credential_field_a_feed_names_is_one_the_charter_records, which turns red if a feed names a fifth field] && !fields.contains(&field)
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
(U) LINE crates/api/src/store_route.rs:1297  [(U) on every target: store_wire::compact_delta is serde_json::to_string(&Delta) (store_wire.rs 249-263): Compact (facets.rs:1984's proof), a &str and two u64 counters, written to an in-memory Vec writer, so it cannot fail] Err(why) => (
(U) LINE crates/api/src/store_route.rs:1298  [(U) on every target: as store_route.rs:1297] axum::http::StatusCode::SERVICE_UNAVAILABLE,
(U) LINE crates/api/src/store_route.rs:1299  [(U) on every target: as store_route.rs:1297] serde_json::json!({"error":why}).to_string(),
(U) LINE crates/api/src/store_route.rs:1300  [(U) on every target: as store_route.rs:1297] ),
(U) BRANCH crates/api/src/store_route.rs:928:12 False  [(U) on every target: census_etag is a quote, 64 hex digits and a quote (store_route.rs:734-740), which HeaderValue::from_str always accepts] if let Ok(value) = axum::http::HeaderValue::from_str(etag) {
(F) BRANCH crates/api/src/store_route.rs:1693:13 False  !body.contains(&longest) && !body.contains("tést-作者"),
(F) BRANCH crates/api/src/store_route.rs:1693:41 False  !body.contains(&longest) && !body.contains("tést-作者"),
(F) BRANCH crates/api/src/store_route.rs:1951:17 False  why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:1951:48 False  why.starts_with("behind: ") && why.contains("2019-11-15"),
(F) BRANCH crates/api/src/store_route.rs:2053:13 False  parts.starts_with("INDEX=")
(F) BRANCH crates/api/src/store_route.rs:2054:20 False  && parts.contains(",CASH=")
(F) BRANCH crates/api/src/store_route.rs:2055:20 False  && parts.ends_with(&format!(",FNO={}", fno.header().generation)),
(F) BRANCH crates/api/src/store_route.rs:2075:17 False  assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:2075:48 False  assert!(why.starts_with("capped: ") && why.contains("FNO"), "{why}");
(F) BRANCH crates/api/src/store_route.rs:2748:13 False  one.starts_with('"') && one.ends_with('"'),
(F) BRANCH crates/api/src/store_route.rs:2748:37 False  one.starts_with('"') && one.ends_with('"'),
(F) BRANCH crates/api/src/store_route.rs:2807:13 False  said.starts_with("the import journal could not be read (")
(F) BRANCH crates/api/src/store_route.rs:2808:20 False  && said.ends_with("so whether this census is behind its import is not known"),
(F) BRANCH crates/api/src/store_route.rs:2818:13 False  said.starts_with("behind: ") && said.contains("; the import journal could not be read"),
(F) BRANCH crates/api/src/store_route.rs:2818:45 False  said.starts_with("behind: ") && said.contains("; the import journal could not be read"),
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
`````

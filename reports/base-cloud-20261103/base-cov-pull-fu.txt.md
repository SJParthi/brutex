# base-cov-pull-fu.txt.md

Source: `state/fix/round2/base-cov-pull-fu.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
PROVISIONAL (3 Nov 2026, BASE-COV-PULL repair 2): classified from the Route B figures of c9d653d5f9145c4d82b77a261b7614db6987f0b1 (the second check r2: state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt), the last tip measured; every line number is of that tip. The tests of repair 2 are NOT in these figures (Route B not run on the repair 2 tip), so many GAP rows are closed by them; the next Route B replaces this file. Round 1's provisional list (tip 0b8a21a5b) is kept as base-cov-pull-fu.r1.txt.
BASE-COV-PULL (F)/(U) list (recipe v4 step 4). Named in docs/05-decisions.md D-7334. Every other uncovered row of the gaps file is a GAP.
Method: (F) = an uncovered row in test code (a *_tests.rs or tests/ file, or past a file's first #[cfg(test)] mod) inside a panic!/unreachable! invocation, the message rows of an assert!/assert_eq!/assert_ne! whose first row ran, the one uncovered side of a branch on such an assert's first row, or the else of a let-else whose block panics; a pattern classifier (node, read-only over the tree at c9d653d5f), NOT checked row by row. (U) = only the rows below, each with its proof from the code.
Counts at c9d653d5f: 9643 uncovered rows (6,949 lines + 2,694 sides); (F) 1116 rows; (U) 23 rows; GAP 8504 rows.

(U) items, each with its proof:
crates/pull/src/master_snapshots.rs:955 (U) Err(why) => return refused(why), -- proof: the error arm of compress(): a GzEncoder writing into an in-memory Vec<u8> at Compression::new(6) has no failing writer (std: Write for Vec<u8> never returns Err) and no invalid level, so compress answers Ok for every body
crates/pull/src/master_snapshots.rs:2129:17 False (U) let Some(before) = old_rows.get(at) else { -- proof: at comes from old_keys, built inside diff() by enumerate() over this same old_rows, so at < old_rows.len(); no caller hands diff() a map
crates/pull/src/master_snapshots.rs:2130 (U) continue; -- proof: the else of 2129 above
crates/pull/src/master_snapshots.rs:2132:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: matched is vec![false; old_rows.len()] and at indexes old_rows (2129)
crates/pull/src/master_snapshots.rs:2174:20 False (U) && let Some(before) = old_rows.get(at) -- proof: at comes from old_tokens, built inside diff() by enumerate() over old_rows
crates/pull/src/master_snapshots.rs:2181:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: matched has old_rows.len() slots and at indexes old_rows (2174)
crates/pull/src/master_snapshots.rs:2215:20 False (U) && let Some(before) = old_rows.get(at) -- proof: at comes from gone_by_isin, built inside diff() from enumerate() over old_rows
crates/pull/src/master_snapshots.rs:2218:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: matched has old_rows.len() slots and at indexes old_rows (2215)
crates/pull/src/master_snapshots.rs:2473:20 False (U) if let Some(count) = counts.get_mut(one.change.index()) { -- proof: counts is [u64; 8] and Change::index maps each of the 8 variants of Change (Change::ALL) onto 0..=7
crates/pull/src/gdfl_run.rs:1412:16 False (U) && held.source == source -- proof: held is self.held[source.slot()], and SourceCache::origin is the only writer of held, storing Measured { source, .. } at that same slot; slot() is injective (every_source_has_its_own_cache_slot), so the slot of source holds source or nothing
crates/pull/src/gdfl_run.rs:1423:16 False (U) if let Some(entry) = self.held.get_mut(slot) { -- proof: held is [Option<Measured>; SOURCE_SLOTS = 3] and slot() answers 0, 1 or 2 by a total match over the three SourceIds
crates/pull/src/gdfl_run.rs:1431 (U) } -- proof: the else of 1423 above
crates/pull/src/gdfl_run.rs:1489:21 False (U) let Ok(next) = day.succ() else { -- proof: `if day >= end { break; }` runs just before, so day < end, a representable Day, and day.succ() is Ok
crates/pull/src/gdfl_run.rs:1490 (U) return Some(month(first)); -- proof: the else of 1489 above
crates/pull/src/gdfl_run.rs:1498:13 False (U) let Ok(next) = end.succ() else { -- proof: `if end >= to { return None; }` runs just before, so end < to, a representable Day, and end.succ() is Ok
crates/pull/src/gdfl_run.rs:1499 (U) return None; -- proof: the else of 1498 above
crates/pull/src/gdfl_run.rs:1949 (U) format!( -- proof: the ok_or_else closure of run_dir: telemetry::run_dirs::for_gdfl answers Some for every word GDFL_KINDS holds (indices, stocks, options), and kind is an ImportKind whose as_str is one of those three
crates/pull/src/gdfl_run.rs:1951 (U) kind.as_str() -- proof: as 1949
crates/pull/src/gdfl_run.rs:1953 (U) }) -- proof: as 1949
crates/pull/src/vendor.rs:504:31 True (U) Self::Minute30 if Timeframe::MINUTE_30.aligns_with_the_open() => { -- proof: 555 (09:15 in minutes) is not a multiple of 30, so the constant predicate is false on this build (round 1)
crates/pull/src/vendor.rs:505 (U) Some(Timeframe::MINUTE_30) -- proof: the arm of the constant-false guard above
crates/pull/src/vendor.rs:507:28 True (U) Self::Hour1 if Timeframe::MINUTE_60.aligns_with_the_open() => { -- proof: 555 is not a multiple of 60, so the constant predicate is false on this build (round 1)
crates/pull/src/vendor.rs:508 (U) Some(Timeframe::MINUTE_60) -- proof: the arm of the constant-false guard above

(F) items:
crates/pull/src/alerts_tests.rs:78 (F) Put::Duplicate { ordinal } => panic!("a duplicate of {ordinal}, not a write"),
crates/pull/src/alerts_tests.rs:210 (F) matches!(
crates/pull/src/alerts_tests.rs:243 (F) panic!("{got:?}")
crates/pull/src/alerts_tests.rs:414 (F) panic!("{:?}", kept[3])
crates/pull/src/alerts_tests.rs:1100 (F) matches!(
crates/pull/src/alerts_tests.rs:1134 (F) panic!("{refused:?}")
crates/pull/src/alerts_tests.rs:1588 (F) matches!(
crates/pull/src/alerts_tests.rs:413:9 False (F) let Kept::Good(4, record) = &kept[3] else {
crates/pull/src/alerts_tests.rs:779:52 False (F) assert!(matches!(&seen[0], Kept::Good(1, r) if r.text == "invented k1"));
crates/pull/src/alerts_tests.rs:780:52 False (F) assert!(matches!(&seen[2], Kept::Good(3, r) if r.open_after == 3));
crates/pull/src/alerts_tests.rs:1133:9 False (F) let Err(AlertError::Io { kind, op, .. }) = &refused else {
crates/pull/src/alerts_tests.rs:1538:56 False (F) matches!(&first, Some(Kept::Good(1, r)) if r.kind == KIND_QUEUE_REPLACED),
crates/pull/src/alerts_tests.rs:2458:62 False (F) matches!(&refused, Err(AlertError::Io { op, .. }) if *op == "keep the damaged queue"),
crates/pull/src/archive.rs:1319 (F) walked.len()
crates/pull/src/archive.rs:1335 (F) sentence.len()
crates/pull/src/archive.rs:1370 (F) matches!(
crates/pull/src/archive.rs:1118:77 False (F) matches!(why, ArchiveError::MemberMalformed { ref path, .. } if *path == odd),
crates/pull/src/base_cov_words_tests.rs:127 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/calendar.rs:4212 (F) DayKind::Unmeasured => panic!("day {day} is inside the range and unmeasured"),
crates/pull/src/calendar.rs:4334 (F) panic!("{index} {day}")
crates/pull/src/calendar.rs:4368 (F) panic!("2021-02-24 traded");
crates/pull/src/calendar.rs:4384 (F) panic!("2025-10-21 traded");
crates/pull/src/calendar.rs:4394 (F) panic!("2024-03-02 traded");
crates/pull/src/calendar.rs:4411 (F) panic!("an ordinary weekday traded");
crates/pull/src/calendar.rs:4585 (F) panic!("it traded");
crates/pull/src/calendar.rs:5371 (F) panic!("{label} is open on the calendar of record")
crates/pull/src/calendar.rs:5388 (F) other => panic!("{label}: sourced_kind_of answered {other:?}"),
crates/pull/src/calendar.rs:5444 (F) _ => false,
crates/pull/src/calendar.rs:5448 (F) matches!(
crates/pull/src/calendar.rs:6451 (F) ExpiryDay::Unmeasured => panic!("day {day} is answered"),
crates/pull/src/calendar.rs:3890:65 False (F) matches!(record.kind, DayKind::Open(session) if session.bars() == 375),
crates/pull/src/calendar.rs:3900:62 False (F) .all(|pair| matches!(pair, [older, newer] if older.day < newer.day)),
crates/pull/src/calendar.rs:4333:21 False (F) let DayKind::Open(session) = kind_of(day) else {
crates/pull/src/calendar.rs:4348:71 False (F) .all(|pair| matches!(pair, [first, second] if second - first == 60))
crates/pull/src/calendar.rs:4367:13 False (F) let DayKind::Open(outage) = kind_of(SYSTEMS_OUTAGE_DAY) else {
crates/pull/src/calendar.rs:4383:13 False (F) let DayKind::Open(muhurat) = kind_of(20_382) else {
crates/pull/src/calendar.rs:4393:13 False (F) let DayKind::Open(dr) = kind_of(19_784) else {
crates/pull/src/calendar.rs:4410:13 False (F) let DayKind::Open(full) = kind_of(20_668) else {
crates/pull/src/calendar.rs:4584:13 False (F) let DayKind::Open(session) = cal.kind_of(20_382) else {
crates/pull/src/calendar.rs:5370:17 False (F) let DayKind::Open(session) = exchange_kind_of(day) else {
crates/pull/src/calendar_tradable_tests.rs:249 (F) matches!(
crates/pull/src/calendar_tradable_tests.rs:1015:20 False (F) if (record_first, record_last) == (SOURCED_FIRST_DAY, SOURCED_LAST_DAY)),
crates/pull/src/capture.rs:582:13 False (F) !text.to_ascii_lowercase().contains("authorization")
crates/pull/src/capture.rs:583:20 False (F) && !text.to_ascii_lowercase().contains("access-token"),
crates/pull/src/cash_auction.rs:971 (F) matches!(
crates/pull/src/cash_session_cache.rs:2148:13 False (F) why.contains("readers or another installer held it")
crates/pull/src/cash_session_cache.rs:2149:20 False (F) && why.contains("for the whole 30 ms bound")
crates/pull/src/cash_session_cache.rs:2150:20 False (F) && why.contains("nothing was installed"),
crates/pull/src/cash_session_cache.rs:2220:21 False (F) why.starts_with("UNVERIFIED cannot open local lifecycle lock")
crates/pull/src/cash_session_cache.rs:2221:28 False (F) && why.contains(&lock.display().to_string()),
crates/pull/src/cash_session_cache.rs:2313:21 False (F) why.starts_with(&format!(
crates/pull/src/cash_session_cache.rs:2315:27 False (F) )) && why.ends_with("; crash durability UNVERIFIED"),
crates/pull/src/cash_session_cache_lock_tests.rs:130:53 False (F) assert!(solo.iter().all(|&(_, hindalco, micro)| hindalco && !micro));
crates/pull/src/cash_session_cache_lock_tests.rs:193:9 False (F) why.contains("unavailable")
crates/pull/src/cash_session_cache_lock_tests.rs:194:16 False (F) && why.contains("an installer held it exclusively")
crates/pull/src/cash_session_cache_lock_tests.rs:195:16 False (F) && why.contains("40 ms bound")
crates/pull/src/cash_session_cache_lock_tests.rs:196:16 False (F) && why.contains("not read as absent"),
crates/pull/src/cash_session_cache_lock_tests.rs:234:50 False (F) why.contains("is absent") || why.contains("missing locally"),
crates/pull/src/census_store_kept_tests.rs:755:9 False (F) why.contains(&vendor_dir(root, Vendor::Gdfl).display().to_string())
crates/pull/src/census_store_kept_tests.rs:756:16 False (F) && why.contains("is not a directory"),
crates/pull/src/census_store_link_tests.rs:205 (F) target.display()
crates/pull/src/census_store_tests.rs:305 (F) done.refused[0].1
crates/pull/src/census_store_tests.rs:585 (F) done.left_open[2]
crates/pull/src/census_store_tests.rs:1321 (F) seal_receipt(&at)
crates/pull/src/census_store_tests.rs:1358:9 False (F) why.contains("SEALED")
crates/pull/src/census_store_tests.rs:1359:16 False (F) && why.contains("census-seal-v2")
crates/pull/src/census_store_tests.rs:1360:16 False (F) && why.contains("nothing was written"),
crates/pull/src/census_store_tests.rs:1505:9 False (F) why.contains("manifest/REQUIRES")
crates/pull/src/census_store_tests.rs:1506:16 False (F) && why.contains("line 3")
crates/pull/src/census_store_tests.rs:1507:16 False (F) && why.contains("\"census-v5\"")
crates/pull/src/census_store_tests.rs:1508:16 False (F) && why.contains("this build does not have")
crates/pull/src/census_store_tests.rs:1509:16 False (F) && why.contains("DB-B68"),
crates/pull/src/chain.rs:516 (F) panic!("Dhan publishes no contract name to discover");
crates/pull/src/chain.rs:539 (F) panic!("an archive has no vendor to publish a lookup");
crates/pull/src/chain.rs:620 (F) seen[0]
crates/pull/src/chain.rs:721 (F) chain.unreadable[0]
crates/pull/src/chain.rs:828 (F) from.asked.borrow()
crates/pull/src/chain.rs:856 (F) panic!("nothing answered, so the walk cannot have succeeded");
crates/pull/src/chain.rs:717:13 False (F) chain.unreadable[0].contains("FINNIFTY")
crates/pull/src/chain.rs:718:20 False (F) && chain.unreadable[0].contains("NIFTY was asked for"),
crates/pull/src/corporate_actions_cov_tests.rs:177 (F) refused(&backwards)
crates/pull/src/corporate_actions_tests.rs:265 (F) panic!("the runs load from the store");
crates/pull/src/corporate_actions_tests.rs:293 (F) panic!("the ledger loads from the store");
crates/pull/src/corporate_actions_tests.rs:368 (F) rights[0].ratio
crates/pull/src/corporate_actions_tests.rs:584 (F) panic!("a swapped table is never served");
crates/pull/src/corporate_actions_tests.rs:663 (F) refused(&twice)
crates/pull/src/corporate_actions_tests.rs:697 (F) refused(&overlap)
crates/pull/src/corporate_actions_tests.rs:866 (F) refused(&backwards)
crates/pull/src/corporate_actions_tests.rs:879 (F) refused(&kinds)
crates/pull/src/corporate_actions_tests.rs:892 (F) refused(&symbols)
crates/pull/src/corporate_actions_tests.rs:905 (F) refused(&split_up)
crates/pull/src/corporate_actions_tests.rs:914 (F) refused(&runs_back)
crates/pull/src/corporate_actions_tests.rs:1189 (F) alt_steps.iter().map(|j| j.verdict).collect::<Vec<_>>()
crates/pull/src/corporate_actions_tests.rs:1265 (F) found[0]
crates/pull/src/corporate_actions_tests.rs:1307 (F) rows[0].evidence
crates/pull/src/corporate_actions_tests.rs:264:9 False (F) let Slot::Store { table: loaded, .. } = load_runs(&store) else {
crates/pull/src/corporate_actions_tests.rs:409:9 False (F) said.contains("SPLITCO")
crates/pull/src/corporate_actions_tests.rs:410:16 False (F) && said.contains("(2023-06-30, 2023-07-03]")
crates/pull/src/corporate_actions_tests.rs:411:16 False (F) && said.contains("factor 2/1"),
crates/pull/src/corporate_actions_tests.rs:426:75 False (F) assert!(matches!(&found[0], Finding::FactorDiffers { product, .. } if *product == r("5")));
crates/pull/src/corporate_actions_tests.rs:583:9 False (F) let Slot::Refused(why) = load_actions(&store) else {
crates/pull/src/corporate_actions_tests.rs:1077:57 False (F) assert!(matches!(&found[0], Finding::NoRow(step) if step.symbol == "REALCO"));
crates/pull/src/corporate_actions_tests.rs:1216:9 False (F) said.contains("basis_anomaly SEGCO (2022-01-28, 2022-01-31]")
crates/pull/src/corporate_actions_tests.rs:1217:16 False (F) && said.contains(
crates/pull/src/corporate_actions_tests.rs:1292:63 False (F) Verdict::Anomaly(Anomaly::WithinWobble { wobble }) if wobble > r("1012/1000")
crates/pull/src/corporate_actions_tests.rs:1312:78 False (F) Finding::BasisAnomaly { step, why: Anomaly::WithinWobble { .. } } if step.symbol == "WOBCO"
crates/pull/src/corporate_actions_tests.rs:1317:58 False (F) .any(|f| matches!(f, Finding::NoRow(step) if step.symbol == "LASTCO"))
crates/pull/src/corporate_actions_tests.rs:1589:22 False (F) .all(|j| !j.step.up() && j.step.far() < judged[0].step.near()),
crates/pull/src/corporate_actions_tests.rs:1752:9 False (F) said.contains("product_too_large SPLITCO (2023-06-30, 2023-07-03]: the 2 rows there")
crates/pull/src/corporate_actions_tests.rs:1753:16 False (F) && !said.contains("multiply to 1/1"),
crates/pull/src/csv.rs:1889 (F) columns.count()
crates/pull/src/csv.rs:1513:50 False (F) assert!(!text.is_char_boundary(4) || !text.is_ascii());
crates/pull/src/csv.rs:1936:66 False (F) Err(CsvError::HeaderMismatch { ref got, .. }) if *got == swapped
crates/pull/src/csv.rs:1945:62 False (F) Err(CsvError::HeaderMismatch { ref got, .. }) if got.is_empty()
crates/pull/src/disk_projection_tests.rs:392 (F) let read = |name: &str| std::env::var(name).unwrap_or_else(|_| panic!("{name} is not set"));
crates/pull/src/disk_projection_tests.rs:546:83 False (F) matches!(refused, FloorRefusal::NeedOverflows { projected: p, .. } if p == projected),
crates/pull/src/expiry_alias.rs:1101 (F) matches!(
crates/pull/src/expiry_alias.rs:1250 (F) panic!("aliased")
crates/pull/src/expiry_alias.rs:1274 (F) panic!("aliased")
crates/pull/src/expiry_alias.rs:1607 (F) panic!("{under} {stated}: {:?}", old.settlement);
crates/pull/src/expiry_alias.rs:1659 (F) panic!("{:?}", old.settlement);
crates/pull/src/expiry_alias.rs:1182:55 True (F) !(a.underlying == b.underlying && a.stated == b.stated),
crates/pull/src/expiry_alias.rs:1249:13 False (F) let Settlement::Aliased(row) = chain.settlement else {
crates/pull/src/expiry_alias.rs:1273:13 False (F) let Settlement::Aliased(row) = got.settlement else {
crates/pull/src/expiry_alias.rs:1606:17 False (F) let Settlement::Aliased(row) = old.settlement else {
crates/pull/src/expiry_alias.rs:1658:21 False (F) let Settlement::ExpiredEarly(row) = old.settlement else {
crates/pull/src/fno.rs:406 (F) panic!("Groww is an HTTP broker");
crates/pull/src/fno.rs:533 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/fno.rs:405:13 False (F) let Transport::Http(spec) = Feed::Groww.descriptor().transport else {
crates/pull/src/fno.rs:532:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/fno_membership_tests.rs:265 (F) panic!("one handover: {:?}", held.handovers());
crates/pull/src/fno_membership_tests.rs:1452 (F) matches!(
crates/pull/src/fno_membership_tests.rs:264:9 False (F) let [handover] = held.handovers() else {
crates/pull/src/fno_membership_tests.rs:744:55 False (F) Err(MembershipError::Corrupt { path, .. }) if path.ends_with("notes.txt")
crates/pull/src/fno_membership_tests.rs:843:66 False (F) matches!(&refusal, MembershipError::Stale { why, .. } if why.contains("carry")),
crates/pull/src/fno_membership_tests.rs:1199:24 False (F) .any(|why| why.starts_with("ZZILLIQ -> ZZNEWBIE")
crates/pull/src/fno_membership_tests.rs:1248:9 False (F) strays[0].1.starts_with("ZZOLDCO listed 1 contract key(s)")
crates/pull/src/fno_membership_tests.rs:1249:16 False (F) && strays[0].1.contains("filed under ZZNEWCO"),
crates/pull/src/fno_membership_tests.rs:1446:30 False (F) assert!(a.is_ok() || b.is_ok(), "round {round}: {a:?} | {b:?}");
crates/pull/src/fnowork_cov_tests.rs:197 (F) panic!("nothing should be asked, and {url} was");
crates/pull/src/fnowork_futures_tests.rs:56 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/fnowork_futures_tests.rs:286 (F) panic!("Groww is an HTTP feed");
crates/pull/src/fnowork_futures_tests.rs:651 (F) landed.balance_line()
crates/pull/src/fnowork_futures_tests.rs:1450 (F) alike.series[0]
crates/pull/src/fnowork_futures_tests.rs:1464 (F) unlike.balance_line()
crates/pull/src/fnowork_futures_tests.rs:1469 (F) unlike.series[0]
crates/pull/src/fnowork_futures_tests.rs:659:9 False (F) stopped.contains("0 bars written, 0 offered from 0 rows")
crates/pull/src/fnowork_futures_tests.rs:660:16 False (F) && stopped.contains("; then stopped: vendor_refused"),
crates/pull/src/fnowork_futures_tests.rs:879:27 False (F) .filter(|why| why.starts_with("contract_duplicated")
crates/pull/src/fnowork_futures_tests.rs:1130:9 False (F) said.starts_with("generation g1 of the futures index did not match its commit log")
crates/pull/src/fnowork_futures_tests.rs:1131:16 False (F) && said.contains(why)
crates/pull/src/fnowork_futures_tests.rs:1132:16 False (F) && said.contains("generation g2 was started"),
crates/pull/src/fnowork_futures_tests.rs:1496:9 False (F) why.starts_with("snapshot_from_the_future: ")
crates/pull/src/fnowork_futures_tests.rs:1497:16 False (F) && why.contains("2030-01-05")
crates/pull/src/fnowork_futures_tests.rs:1498:16 False (F) && why.contains("after today 2026-01-07"),
crates/pull/src/fold.rs:1925 (F) diagnostics.len(),
crates/pull/src/fold.rs:1926 (F) diagnostics.iter().take(7).collect::<Vec<_>>()
crates/pull/src/fold.rs:2270 (F) derived.diagnostics[0]
crates/pull/src/fold.rs:2322 (F) lines[0]
crates/pull/src/gaps.rs:1138:32 False (F) .any(|gap| gap.reason == Reason::OutsideWindow
crates/pull/src/gaps.rs:1176:40 False (F) lost.gaps.iter().any(|gap| gap.day == day
crates/pull/src/gaps.rs:1178:20 False (F) && gap.to == 939
crates/pull/src/gaps.rs:1242:16 False (F) && gap.to == 763
crates/pull/src/gaps.rs:1281:47 False (F) .any(|gap| gap.from == 915 && gap.to == 929 && gap.reason == Reason::VendorHole)
crates/pull/src/gaps.rs:1428:26 False (F) .any(|g| g.reason == Reason::OutsideWindow && g.from == 600),
crates/pull/src/gdfl_backfill_tests.rs:255 (F) store::file::TimeLookup::Indexed
crates/pull/src/gdfl_backfill_tests.rs:322 (F) lines[intent]
crates/pull/src/gdfl_backfill_tests.rs:340 (F) lines[repair]
crates/pull/src/gdfl_backfill_tests.rs:802 (F) nifty[0].why
crates/pull/src/gdfl_backfill_tests.rs:1269 (F) refused.failures[0]
crates/pull/src/gdfl_backfill_tests.rs:1282 (F) refused.failures[0]
crates/pull/src/gdfl_backfill_tests.rs:252:12 False (F) if small {
crates/pull/src/gdfl_backfill_tests.rs:402:35 False (F) intents.iter().any(|line| line
crates/pull/src/gdfl_backfill_tests.rs:523:13 False (F) failure.why.contains("the versioned repair refused it too")
crates/pull/src/gdfl_backfill_tests.rs:524:20 False (F) && failure.why.contains("bar-revisions-v1"),
crates/pull/src/gdfl_cm_admission_tests.rs:704 (F) other => panic!("{stem} {d}: {other:?}"),
crates/pull/src/gdfl_cm_admission_tests.rs:1663 (F) path.display()
crates/pull/src/gdfl_cm_admission_tests.rs:952:32 False (F) assert!(vix.iter().all(|b| b.close == 6_141 || b.close == 6_142));
crates/pull/src/gdfl_cm_admission_tests.rs:1692:51 False (F) Err(CmRefusal::AuctionSidecar { why }) if why.contains("new generation")
crates/pull/src/gdfl_cm_admission_tests.rs:1697:51 False (F) Err(CmRefusal::AuctionSidecar { why }) if why.contains("never an insertion")
crates/pull/src/gdfl_cm_auction.rs:972 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_cm_auction.rs:1132 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_cm_auction_v2_tests.rs:80 (F) Ok(sidecar) => panic!(
crates/pull/src/gdfl_cm_auction_v2_tests.rs:82 (F) sidecar.records.len()
crates/pull/src/gdfl_cm_auction_v2_tests.rs:91 (F) other => panic!("not a sidecar refusal: {other:?}"),
crates/pull/src/gdfl_cm_auction_v2_tests.rs:260 (F) made.display()
crates/pull/src/gdfl_cm_auction_v2_tests.rs:437 (F) Err(refusal) => panic!("a reader beside an append was refused: {refusal}"),
crates/pull/src/gdfl_dropbox_tests.rs:957 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1005 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1093 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1141 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1198 (F) panic!("{cut:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1228 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1252 (F) panic!("{shrank:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1403 (F) panic!("{storm:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1431 (F) panic!("{down:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1603 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1687 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1766 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1812 (F) panic!("{report:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:1953 (F) panic!("{full:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:2058 (F) again.refused[0].detail
crates/pull/src/gdfl_dropbox_tests.rs:2093 (F) panic!("{stopped:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:2347 (F) report.refused[0]
crates/pull/src/gdfl_dropbox_tests.rs:2472 (F) panic!("{report:#?}");
crates/pull/src/gdfl_dropbox_tests.rs:2580 (F) panic!("{outcome:?}");
crates/pull/src/gdfl_dropbox_tests.rs:3078 (F) report.refused[0]
crates/pull/src/gdfl_dropbox_tests.rs:3155 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_dropbox_tests.rs:3205 (F) accepted[0]
crates/pull/src/gdfl_dropbox_tests.rs:3459 (F) codes(&two)
crates/pull/src/gdfl_dropbox_tests.rs:3521 (F) panic!("{two:#?}")
crates/pull/src/gdfl_dropbox_tests.rs:868:9 False (F) journal_text(&roots).contains("v=1 wait ")
crates/pull/src/gdfl_dropbox_tests.rs:869:16 False (F) && journal_text(&roots).contains("reason=dropbox_link_missing")
crates/pull/src/gdfl_dropbox_tests.rs:956:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1004:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1092:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1140:13 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1197:9 False (F) let Outcome::Refused(refusal) = &cut.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1227:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1251:9 False (F) let Outcome::Refused(refusal) = &shrank.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1402:9 False (F) let Outcome::Refused(refusal) = &storm.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1430:9 False (F) let Outcome::Refused(refusal) = &down.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1602:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1686:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1765:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1784:60 False (F) assert!(matches!(&lost.outcome, Outcome::Refused(r) if r.code == "store_lost"));
crates/pull/src/gdfl_dropbox_tests.rs:1811:9 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:1952:9 False (F) let Outcome::Refused(refusal) = &full.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:2092:9 False (F) let Outcome::Refused(refusal) = &stopped.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:2101:16 False (F) && l.contains("stage=download")
crates/pull/src/gdfl_dropbox_tests.rs:2127:57 False (F) matches!(&early.outcome, Outcome::Refused(r) if r.code == "stopped"),
crates/pull/src/gdfl_dropbox_tests.rs:2442:20 False (F) && l.contains("complete=false")
crates/pull/src/gdfl_dropbox_tests.rs:2471:13 False (F) let Outcome::Refused(refusal) = &report.outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:2579:13 False (F) let Outcome::Refused(refusal) = &outcome else {
crates/pull/src/gdfl_dropbox_tests.rs:3202:9 False (F) accepted[0].contains(" complete=true count=2 ")
crates/pull/src/gdfl_dropbox_tests.rs:3203:16 False (F) && accepted[0].ends_with(&format!(" accepted_shrink={token} was=3")),
crates/pull/src/gdfl_dropbox_tests.rs:3520:9 False (F) let Outcome::Refused(refusal) = &two.outcome else {
crates/pull/src/gdfl_format_change_tests.rs:373 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_format_change_tests.rs:221:22 False (F) .any(|f| f.instrument.contains("2024-04-09") && f.why.contains(&shape)),
crates/pull/src/gdfl_format_change_tests.rs:283:22 False (F) .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
crates/pull/src/gdfl_ia16_tests.rs:2222 (F) matches!(
crates/pull/src/gdfl_ia16_tests.rs:2283 (F) matches!(
crates/pull/src/gdfl_ia16_tests.rs:394:13 False (F) line.starts_with("incomplete ")
crates/pull/src/gdfl_ia16_tests.rs:395:20 False (F) && line.contains(" stage=file ")
crates/pull/src/gdfl_ia16_tests.rs:396:20 False (F) && line.contains(" reason=needs_month_generation"),
crates/pull/src/gdfl_ia16_tests.rs:647:29 False (F) .any(|line| line.starts_with("repairing ") && line.contains("sealed-month")),
crates/pull/src/gdfl_ia16_tests.rs:1827:71 False (F) matches!(&ended, ImportRefusal::SourceLost { day, .. } if *day == d2()),
crates/pull/src/gdfl_ia16_tests.rs:1930:24 False (F) .any(|one| one.instrument.ends_with("BANKNIFTY") && one.why.contains("absent")),
crates/pull/src/gdfl_ia16_tests.rs:2167:68 False (F) Some(RunRefusal::RepairReasonInvalid { bytes }) if bytes == bad.len()
crates/pull/src/gdfl_ia16_tests.rs:2721:13 False (F) said.contains("chflags nouchg")
crates/pull/src/gdfl_ia16_tests.rs:2722:20 False (F) && said.contains("chmod -N")
crates/pull/src/gdfl_ia16_tests.rs:2723:20 False (F) && said.contains("read-write"),
crates/pull/src/gdfl_ia16_tests.rs:2957:56 True (F) !(one.instrument.contains("NIFTY BANK") && one.why.contains("needs_month_generation")),
crates/pull/src/gdfl_ia3_tests.rs:322:9 False (F) line.starts_with("refused indices 2024-04-01 * ")
crates/pull/src/gdfl_ia3_tests.rs:323:16 False (F) && line.contains("stage=vendor reason=day_file_misfiled"),
crates/pull/src/gdfl_ia4_tests.rs:154 (F) kind.as_str()
crates/pull/src/gdfl_import_cov_tests.rs:89:32 False (F) .is_some_and(|why| why.contains("plain.zip") && why.contains("no end-of-central")),
crates/pull/src/gdfl_import_cov_tests.rs:262:62 False (F) matches!(&refused, ImportRefusal::Journal { why } if why.contains("gdfl.journal")),
crates/pull/src/gdfl_import_ia2_tests.rs:119 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_ia2_tests.rs:271 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_ia2_tests.rs:142:9 False (F) closed.starts_with("incomplete indices 2024-04-02 * ")
crates/pull/src/gdfl_import_ia2_tests.rs:143:16 False (F) && closed.contains(" stage=run reason=stopped at=")
crates/pull/src/gdfl_import_ia2_tests.rs:144:16 False (F) && closed.ends_with(" run=1790000101"),
crates/pull/src/gdfl_import_ia2_tests.rs:151:9 False (F) last.starts_with("run 1790000101 ended=")
crates/pull/src/gdfl_import_ia2_tests.rs:152:16 False (F) && last.contains(" outcome=stopped ")
crates/pull/src/gdfl_import_ia2_tests.rs:153:16 False (F) && last.ends_with(&format!(" definition={BAR_DEFINITION}")),
crates/pull/src/gdfl_import_ia2_tests.rs:204:71 False (F) matches!(&got, ImportRefusal::Stopped { day: Some(d), .. } if *d == day(2)),
crates/pull/src/gdfl_import_tests.rs:382 (F) texts[8]
crates/pull/src/gdfl_import_tests.rs:387 (F) texts[9]
crates/pull/src/gdfl_import_tests.rs:558 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_tests.rs:1074 (F) matches!(
crates/pull/src/gdfl_import_tests.rs:2078 (F) bin.display()
crates/pull/src/gdfl_import_tests.rs:2634 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_tests.rs:4711 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:4902 (F) indices[0].0
crates/pull/src/gdfl_import_tests.rs:4952 (F) bytes.len()
crates/pull/src/gdfl_import_tests.rs:5096 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5244 (F) durability.as_str()
crates/pull/src/gdfl_import_tests.rs:5250 (F) durability.as_str()
crates/pull/src/gdfl_import_tests.rs:5256 (F) durability.as_str(),
crates/pull/src/gdfl_import_tests.rs:5257 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5382 (F) panic!("the run stopped at the day's flush: {refused:?}");
crates/pull/src/gdfl_import_tests.rs:5535 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5545 (F) one.display()
crates/pull/src/gdfl_import_tests.rs:5697 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5904 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:247:40 False (F) assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100));
crates/pull/src/gdfl_import_tests.rs:616:19 False (F) )) && !l.contains(" stage=")
crates/pull/src/gdfl_import_tests.rs:623:9 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_import_tests.rs:625:15 False (F) )) && last.contains(" failures=1 ")
crates/pull/src/gdfl_import_tests.rs:626:16 False (F) && last.contains(" stage=census reason=too_many_entries run="),
crates/pull/src/gdfl_import_tests.rs:646:9 False (F) journal.contains("\ndone options 2024-04-04 * ")
crates/pull/src/gdfl_import_tests.rs:647:16 False (F) && journal.contains("\ndone options 2024-04-05 * "),
crates/pull/src/gdfl_import_tests.rs:685:9 False (F) text.contains("refused stocks 2024-04-01 although the day's batch fits")
crates/pull/src/gdfl_import_tests.rs:686:16 False (F) && text.contains("another ingest holds the census lock"),
crates/pull/src/gdfl_import_tests.rs:1459:9 False (F) damaged.digest.is_none()
crates/pull/src/gdfl_import_tests.rs:1460:16 False (F) && damaged
crates/pull/src/gdfl_import_tests.rs:1493:9 False (F) unformatted.digest.is_none()
crates/pull/src/gdfl_import_tests.rs:1494:16 False (F) && unformatted
crates/pull/src/gdfl_import_tests.rs:1636:9 False (F) second.contains(" source=zip ")
crates/pull/src/gdfl_import_tests.rs:1637:16 False (F) && second.contains(" commit=unstamped ")
crates/pull/src/gdfl_import_tests.rs:1638:16 False (F) && second.contains(" started=1750000000 ")
crates/pull/src/gdfl_import_tests.rs:1639:16 False (F) && second.ends_with(&format!(
crates/pull/src/gdfl_import_tests.rs:1660:9 False (F) first.starts_with("run ")
crates/pull/src/gdfl_import_tests.rs:1661:16 False (F) && first.contains(
crates/pull/src/gdfl_import_tests.rs:1708:9 False (F) line.starts_with("incomplete indices 2024-04-01 ")
crates/pull/src/gdfl_import_tests.rs:1711:16 False (F) && line.contains(" stage=file reason=bars_differ_held run="),
crates/pull/src/gdfl_import_tests.rs:1723:9 False (F) line.starts_with("incomplete options 2024-04-01 ")
crates/pull/src/gdfl_import_tests.rs:1724:16 False (F) && line.contains(" stage=vendor reason="),
crates/pull/src/gdfl_import_tests.rs:2041:22 False (F) .all(|l| l.contains(" member_bytes=0 ") && l.ends_with(" member_src=none")),
crates/pull/src/gdfl_import_tests.rs:2046:22 False (F) .all(|l| !l.contains(" member_bytes=0 ") && l.ends_with(" member_src=zip")),
crates/pull/src/gdfl_import_tests.rs:2087:22 False (F) .all(|b| b.volume == 0 && b.open_interest == OI_NULL)
crates/pull/src/gdfl_import_tests.rs:2096:31 False (F) assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000));
crates/pull/src/gdfl_import_tests.rs:2640:34 False (F) .is_err_and(|unread| unread.contains("gdfl.man") && unread.contains("not a regular")),
crates/pull/src/gdfl_import_tests.rs:2648:22 False (F) .any(|f| f.instrument == "2024-04-01 census" && f.why.contains("nothing of the batch")),
crates/pull/src/gdfl_import_tests.rs:2654:9 False (F) text.contains("indices 2024-04-01")
crates/pull/src/gdfl_import_tests.rs:2655:16 False (F) && text.contains("its room could not be measured")
crates/pull/src/gdfl_import_tests.rs:2656:16 False (F) && text.contains("the run stopped there after 0 day(s) imported"),
crates/pull/src/gdfl_import_tests.rs:2663:9 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_import_tests.rs:2665:15 False (F) )) && last.contains(" stage=census reason=census_refused run="),
crates/pull/src/gdfl_import_tests.rs:2916:74 False (F) matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder"))
crates/pull/src/gdfl_import_tests.rs:4359:20 False (F) || crate::gdfl_lineage::EventRecord::parse(&line).is_some(),
crates/pull/src/gdfl_import_tests.rs:4948:21 False (F) u64::try_from(bytes.len()).unwrap() <= store::format::HEADER_LEN
crates/pull/src/gdfl_import_tests.rs:4949:28 False (F) && rest.iter().all(|&byte| byte == 0)
crates/pull/src/gdfl_import_tests.rs:4950:28 False (F) && store::header::Header::decode(slot).is_err(),
crates/pull/src/gdfl_import_tests.rs:5050:28 False (F) unborn == 0 || open_day,
crates/pull/src/gdfl_import_tests.rs:5435:22 False (F) .any(|f| f.why.contains("durability barrier") && f.why.contains("already failed")),
crates/pull/src/gdfl_import_tests.rs:5680:13 False (F) last.starts_with(&format!("incomplete options {}", d2()))
crates/pull/src/gdfl_import_tests.rs:5681:20 False (F) && last.contains(&format!(" failures={} ", refused + 1)),
crates/pull/src/gdfl_lineage_tests.rs:1093 (F) panic!("an old name on its change day");
crates/pull/src/gdfl_lineage_tests.rs:1092:9 False (F) let Err(Stale::AfterChange(change)) = filed("NIITTECH", d(2020, 8, 20)) else {
crates/pull/src/gdfl_lineage_tests.rs:1135:9 False (F) said.contains("NIITTECH27AUG201900CE")
crates/pull/src/gdfl_lineage_tests.rs:1136:16 False (F) && said.contains("NIITTECH became COFORGE on 2020-08-20"),
crates/pull/src/gdfl_nfo_attack_tests.rs:328 (F) why => panic!("{ticker} on {on}: {why:?}"),
crates/pull/src/gdfl_nfo_attack_tests.rs:352 (F) misread.len(),
crates/pull/src/gdfl_nfo_attack_tests.rs:353 (F) &misread[..8.min(misread.len())]
crates/pull/src/gdfl_nfo_attack_tests.rs:358 (F) missed.len(),
crates/pull/src/gdfl_nfo_attack_tests.rs:359 (F) &missed[..8.min(missed.len())]
crates/pull/src/gdfl_nfo_attack_tests.rs:553 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:650 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:751 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:811 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:953 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:1048 (F) got.contract.as_str(),
crates/pull/src/gdfl_nfo_attack_tests.rs:1049 (F) reencode(&got)
crates/pull/src/gdfl_nfo_attack_tests.rs:1384 (F) NfoRefusal::HeaderUnknown { .. } => panic!("the header was intact"),
crates/pull/src/gdfl_nfo_attack_tests.rs:1399 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_nfo_attack_tests.rs:1742 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:1750 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:1932 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:1951 (F) matches!(
crates/pull/src/gdfl_nfo_attack_tests.rs:1063:21 False (F) expiry >= on
crates/pull/src/gdfl_nfo_attack_tests.rs:1064:28 False (F) && expiry.days_from_epoch() <= on.days_from_epoch() + EXPIRY_HORIZON_DAYS
crates/pull/src/gdfl_nfo_attack_tests.rs:1376:45 False (F) assert!(row.ltq == 0 || row.ltp >= costs::rate::TICK.raw());
crates/pull/src/gdfl_nfo_attack_tests.rs:1522:24 False (F) || entry.ends_with(&format!("{ticker}.NFO.CSV")),
crates/pull/src/gdfl_nfo_tests.rs:295 (F) matches!(
crates/pull/src/gdfl_nfo_tests.rs:358 (F) panic!("{name}: {got:?}")
crates/pull/src/gdfl_nfo_tests.rs:1190 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_nfo_tests.rs:3376 (F) let got = new.unwrap_or_else(|why| panic!("{on} {name}: {why}"));
crates/pull/src/gdfl_r2_attack_tests.rs:1643 (F) matches!(
crates/pull/src/gdfl_r2_attack_tests.rs:1582:34 False (F) .any(|r| &r.instrument == name && r.why.contains("same contract")),
crates/pull/src/gdfl_r2_attack_tests.rs:1863:13 False (F) !text.contains("2023-06-29") && !text.contains("2023-08-15")
crates/pull/src/gdfl_r3_attack_tests.rs:449 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:458 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:483 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:779 (F) block.len(),
crates/pull/src/gdfl_r3_attack_tests.rs:780 (F) got.as_ref().map(Vec::len)
crates/pull/src/gdfl_r3_attack_tests.rs:795 (F) block.len(),
crates/pull/src/gdfl_r3_attack_tests.rs:796 (F) got.as_ref().map(Vec::len)
crates/pull/src/gdfl_r3_attack_tests.rs:821 (F) got.map(|b| b.len())
crates/pull/src/gdfl_r3_attack_tests.rs:904 (F) unreachable!("only swept index names are drawn")
crates/pull/src/gdfl_r3_attack_tests.rs:1421 (F) all.files + all.files_refused
crates/pull/src/gdfl_r3_attack_tests.rs:383:34 False (F) name == want_name || name.starts_with(&format!("{want_name}.")),
crates/pull/src/gdfl_r4_attack_tests.rs:133 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:208 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:681 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:285:13 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_r4_attack_tests.rs:287:19 False (F) )) && last.contains(" late=1 late_unresolved=0 max_back_s=7 ")
crates/pull/src/gdfl_r4_attack_tests.rs:288:20 False (F) && last.contains(" stage=file reason=bars_differ_held"),
crates/pull/src/gdfl_r4_attack_tests.rs:803:25 False (F) (at == 0 && appended.starts_with("run "))
crates/pull/src/gdfl_r4_attack_tests.rs:805:20 False (F) || appended.starts_with("done stocks 2024-04-")
crates/pull/src/gdfl_r4_attack_tests.rs:806:24 False (F) && appended.contains(&format!(" definition={BAR_DEFINITION} ")),
crates/pull/src/gdfl_rekey_tests.rs:629 (F) done.refused[0].why
crates/pull/src/gdfl_rekey_tests.rs:814 (F) refusal.refusal()
crates/pull/src/gdfl_rekey_tests.rs:1284 (F) decided.unresolved[0].why
crates/pull/src/gdfl_rekey_tests.rs:1316 (F) || panic!("{key}: {:?}", decided.unresolved),
crates/pull/src/gdfl_rekey_tests.rs:1760 (F) panic!("{:?}", decided.census);
crates/pull/src/gdfl_rekey_tests.rs:2379 (F) || refused.contains("the ledger changed since this plan")),
crates/pull/src/gdfl_rekey_tests.rs:1409:9 False (F) why.contains("and ledger line 1 retires it as")
crates/pull/src/gdfl_rekey_tests.rs:1410:16 False (F) && why.contains("one retirement has one account"),
crates/pull/src/gdfl_rekey_tests.rs:1759:9 False (F) let CensusStep::Unreadable(why) = &decided.census else {
crates/pull/src/gdfl_rekey_tests.rs:1867:24 False (F) .any(|one| one.key == format!("NSE-FNO-NIFTY-{STATED}")
crates/pull/src/gdfl_rekey_tests.rs:1894:9 False (F) left.why.contains("(apply's own check: ")
crates/pull/src/gdfl_rekey_tests.rs:1895:16 False (F) && left.why.contains("would precede or contradict"),
crates/pull/src/gdfl_rekey_tests.rs:2301:17 False (F) why.contains("the ledger changed since this plan; re-plan")
crates/pull/src/gdfl_rekey_tests.rs:2302:24 False (F) && why.contains("nothing was written"),
crates/pull/src/gdfl_rekey_tests.rs:2377:17 False (F) refused.contains("nothing was written")
crates/pull/src/gdfl_rekey_tests.rs:2378:25 False (F) && (refused.contains("held by another re-key apply")
crates/pull/src/gdfl_rekey_tests.rs:2410:13 False (F) why.contains("the plan counted 3 bar(s)")
crates/pull/src/gdfl_rekey_tests.rs:2411:20 False (F) && why.contains("bars landed under it after the survey; re-plan"),
crates/pull/src/gdfl_rekey_tests.rs:2442:13 False (F) why.contains("retires NSE-FNO-NIFTY-2020-06-25-191250000-CE")
crates/pull/src/gdfl_rekey_tests.rs:2443:20 False (F) && why.contains("bars move only into a key that continues"),
crates/pull/src/gdfl_rekey_tests.rs:2564:9 False (F) left.why.contains("retires it with 1 bar(s) on 1 day(s)")
crates/pull/src/gdfl_rekey_tests.rs:2565:16 False (F) && left.why.contains("holds 2 bar(s) on 1 day(s) now"),
crates/pull/src/gdfl_rekey_tests.rs:2697:9 False (F) refused.why.contains("the plan counted 1 bar(s) to retire")
crates/pull/src/gdfl_rekey_tests.rs:2698:16 False (F) && refused.why.contains("it holds 2 bar(s) in 1 month(s) now")
crates/pull/src/gdfl_rekey_tests.rs:2699:16 False (F) && refused.why.ends_with("re-plan"),
crates/pull/src/gdfl_rekey_tests.rs:2785:13 False (F) why.contains("the ledger changed since this plan; re-plan")
crates/pull/src/gdfl_rekey_tests.rs:2786:20 False (F) && why.contains("nothing was written"),
crates/pull/src/gdfl_run_ia2_tests.rs:422 (F) panic!("{api:?}");
crates/pull/src/gdfl_run_ia2_tests.rs:545 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_run_ia2_tests.rs:582 (F) panic!("{seen:?}");
crates/pull/src/gdfl_run_ia2_tests.rs:1206 (F) panic!("{busy:?}");
crates/pull/src/gdfl_run_ia2_tests.rs:338:9 False (F) refused.to_string().contains("runs.claims")
crates/pull/src/gdfl_run_ia2_tests.rs:339:16 False (F) && refused.to_string().contains("not a regular file"),
crates/pull/src/gdfl_run_ia2_tests.rs:354:9 False (F) refused.to_string().contains("runs.claims")
crates/pull/src/gdfl_run_ia2_tests.rs:355:16 False (F) && refused.to_string().contains("was not handed out"),
crates/pull/src/gdfl_run_ia2_tests.rs:421:9 False (F) let RunRefusal::RunBusy { lock, holder } = &api else {
crates/pull/src/gdfl_run_ia2_tests.rs:550:96 False (F) seen.iter().any(|s| matches!(s, LockState::Held { claim: ClaimState::Claim(c), .. } if c.run == claimed.run)),
crates/pull/src/gdfl_run_ia2_tests.rs:592:93 False (F) matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("crc"))
crates/pull/src/gdfl_run_ia2_tests.rs:596:93 False (F) matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("digest"))
crates/pull/src/gdfl_run_ia2_tests.rs:600:93 False (F) matches!(locks::read_sidecar(&sidecar, &found.request), SidecarState::Stale(why) if why.contains("torn"))
crates/pull/src/gdfl_run_ia2_tests.rs:607:106 False (F) matches!(seen.as_slice(), [LockState::Interrupted { claim: ClaimState::Unreadable(why), .. }] if why.contains("\\x1b")),
crates/pull/src/gdfl_run_ia2_tests.rs:647:67 False (F) matches!(&busy, RunRefusal::RunBusy { holder, .. } if holder.contains(&format!("run={run} ")))
crates/pull/src/gdfl_run_ia2_tests.rs:906:9 False (F) busy.starts_with("run_busy: ")
crates/pull/src/gdfl_run_ia2_tests.rs:907:16 False (F) && busy.contains(&format!("run={} ", run.run))
crates/pull/src/gdfl_run_ia2_tests.rs:908:16 False (F) && busy.ends_with("Nothing was written"),
crates/pull/src/gdfl_run_ia2_tests.rs:1146:62 False (F) matches!(&unnamed, LockFault::Busy { holder, .. } if holder.is_empty()),
crates/pull/src/gdfl_run_ia2_tests.rs:1210:13 False (F) named.contains(&format!("run={} ", holder.run))
crates/pull/src/gdfl_run_ia2_tests.rs:1211:20 False (F) && named.contains(" pid=4242 ")
crates/pull/src/gdfl_run_ia2_tests.rs:1212:20 False (F) && named.contains(" by=cli "),
crates/pull/src/gdfl_run_locks_cov_tests.rs:301:61 False (F) matches!(&state, LockState::Unprobed { why, .. } if why == "is a directory"),
crates/pull/src/gdfl_run_tests.rs:624 (F) panic!("{end:?}");
crates/pull/src/gdfl_run_tests.rs:678 (F) panic!("{ends:?}");
crates/pull/src/gdfl_run_tests.rs:1464 (F) panic!("a prefix of a journal is a journal: cut at {cut_at}");
crates/pull/src/gdfl_run_tests.rs:623:13 False (F) let KindEnd::Report(report) = end else {
crates/pull/src/gdfl_run_tests.rs:677:9 False (F) let KindEnd::Unopened(refused) = &ends[0].1 else {
crates/pull/src/gdfl_run_tests.rs:1378:44 False (F) Refreshed::Reloaded { now, .. } if now == head.len() as u64
crates/pull/src/gdfl_run_tests.rs:1463:13 False (F) let Ok(mut reader) = JournalReader::open(&store) else {
crates/pull/src/gdfl_scope_tests.rs:217 (F) path.display()
crates/pull/src/gdfl_scope_tests.rs:745 (F) after.keys().collect::<Vec<_>>()
crates/pull/src/gdfl_scope_tests.rs:1275 (F) closes[2]
crates/pull/src/gdfl_scope_tests.rs:1694 (F) panic!("{d}: a close with no scope owes the members beyond the swept surface");
crates/pull/src/gdfl_scope_tests.rs:2231 (F) journal_text(&store)
crates/pull/src/gdfl_scope_tests.rs:2287 (F) journal_text(&store)
crates/pull/src/gdfl_scope_tests.rs:2450 (F) journal_text(&store)
crates/pull/src/gdfl_scope_tests.rs:2551 (F) journal_text(&store)
crates/pull/src/gdfl_scope_tests.rs:1693:13 False (F) let Owed::Probe(beyond) = owed else {
crates/pull/src/gdfl_sealed_tests.rs:51:29 False (F) parts.len() == 7 || parts.len() == 8,
crates/pull/src/gdfl_seconds_attack_tests.rs:344 (F) (got, want) => panic!("case {case} {kind:?} {ticks:?}: {got:?} against {want:?}"),
crates/pull/src/gdfl_seconds_attack_tests.rs:832 (F) matches!(
crates/pull/src/gdfl_seconds_attack_tests.rs:147:27 False (F) assert!(k == 0 || placed[k - 1].0 <= *s, "placement runs forward");
crates/pull/src/gdfl_series_code_tests.rs:234:58 False (F) Some(CmRefusal::SeriesNotStored { code, .. }) if *code == *edge
crates/pull/src/gdfl_series_code_tests.rs:238:59 False (F) Some(CmRefusal::SeriesNotStored { share, .. }) if &*share == widest
crates/pull/src/gdfl_tickstore.rs:1785 (F) frame.len()
crates/pull/src/gdfl_tickstore.rs:1701:28 False (F) if what.contains(named)),
crates/pull/src/gdfl_v3_tests.rs:207 (F) panic!("a listing without its directories agreed");
crates/pull/src/gdfl_v3_tests.rs:221 (F) panic!("a configured ledger is unset");
crates/pull/src/gdfl_v3_tests.rs:304 (F) panic!("two differences agreed");
crates/pull/src/gdfl_v3_tests.rs:311 (F) panic!("a renamed path agreed");
crates/pull/src/gdfl_v3_tests.rs:337 (F) panic!("a longer listing agreed");
crates/pull/src/gdfl_v3_tests.rs:465 (F) panic!("a directory at the marker's name was trusted");
crates/pull/src/gdfl_v3_tests.rs:574 (F) panic!("a directory at the manifest's name was trusted");
crates/pull/src/gdfl_v3_tests.rs:763 (F) panic!("an empty manifest agreed with an entry");
crates/pull/src/gdfl_v3_tests.rs:869 (F) panic!("BRUTEX_GDFL_V3_LEDGER is unset");
crates/pull/src/gdfl_v3_tests.rs:206:9 False (F) let V3::Disagrees { count, first } = v3.check(d, Some(&files_only)) else {
crates/pull/src/gdfl_v3_tests.rs:220:9 False (F) let V3Ledger::At { root } = &v3 else {
crates/pull/src/gdfl_v3_tests.rs:303:9 False (F) let V3::Disagrees { count, first } = check(&both, Some(99)) else {
crates/pull/src/gdfl_v3_tests.rs:310:9 False (F) let V3::Disagrees { count, first } = check(&renamed, Some(99)) else {
crates/pull/src/gdfl_v3_tests.rs:336:9 False (F) let V3::Disagrees { count, first } = check(&longer, Some(99)) else {
crates/pull/src/gdfl_v3_tests.rs:464:9 False (F) let V3::Unverified { why } = dir_marker.check(d, Some(&good)) else {
crates/pull/src/gdfl_v3_tests.rs:573:9 False (F) let V3::Unverified { why } = dir_manifest.check(d, Some(&good)) else {
crates/pull/src/gdfl_wp12e_tests.rs:610:57 False (F) Err(ImportRefusal::UnderlyingSplit { held }) if held == [("TATAMOTORS".to_owned(), "TMPV".to_owned())]
crates/pull/src/http.rs:4133 (F) panic!("{feed:?} is an HTTP feed");
crates/pull/src/http.rs:4156 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4180 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4199 (F) panic!("request {} in one second was admitted", per_second + 1)
crates/pull/src/http.rs:4245 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4319 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4361 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4561 (F) panic!("a repeated key must be refused: {refused:?}");
crates/pull/src/http.rs:4895 (F) panic!("{sent} is not zero and must not be stored as one")
crates/pull/src/http.rs:5225 (F) panic!("{name} ({volume}, {interest}): one row goes, not the window: {why}")
crates/pull/src/http.rs:5836 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:5849 (F) panic!("a body naming DH-901 is a refusal whatever the status says")
crates/pull/src/http.rs:5866 (F) panic!("the sentence names a dead token")
crates/pull/src/http.rs:5895 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:5949 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:6042 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:6150 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:6300 (F) panic!("a missing timestamp array must refuse")
crates/pull/src/http.rs:6386 (F) panic!("the descriptor says top level, so `data` is not searched")
crates/pull/src/http.rs:6410 (F) panic!("the declared envelope is absent and must be named")
crates/pull/src/http.rs:6422 (F) panic!("an array is not an envelope")
crates/pull/src/http.rs:6432 (F) panic!("an empty object holds no envelope")
crates/pull/src/http.rs:6461 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/http.rs:6517 (F) || zoneless.bars.first().map(|b| b.ts_micros)
crates/pull/src/http.rs:6518 (F) != landed.bars.first().map(|b| b.ts_micros),
crates/pull/src/http.rs:6521 (F) zoneless.bars.len(),
crates/pull/src/http.rs:6522 (F) landed.bars.len()
crates/pull/src/http.rs:6550 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/http.rs:6571 (F) matches!(
crates/pull/src/http.rs:6587 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/http.rs:6645 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/http.rs:6650 (F) panic!("Groww is an HTTP feed");
crates/pull/src/http.rs:6879 (F) panic!("a redirect is a refusal this build reports, not one it hides")
crates/pull/src/http.rs:6926 (F) panic!("429 is a refusal the governor needs to see")
crates/pull/src/http.rs:6970 (F) panic!("an answer declaring more than the cap is refused, not decoded")
crates/pull/src/http.rs:7014 (F) panic!("an answer past the cap is refused, not decoded")
crates/pull/src/http.rs:7078 (F) Ok(body) => panic!(
crates/pull/src/http.rs:7081 (F) body.len()
crates/pull/src/http.rs:7175 (F) Ok(body) => panic!("an invalid byte is refused, not replaced: {body:?}"),
crates/pull/src/http.rs:7324 (F) credited.join("; ")
crates/pull/src/http.rs:7347 (F) allowance(&held)
crates/pull/src/http.rs:7412 (F) panic!("500 is a refusal, however long its body")
crates/pull/src/http.rs:7427 (F) detail.chars().count()
crates/pull/src/http.rs:7608 (F) panic!("the variant a bad field already uses, not a new one")
crates/pull/src/http.rs:7879 (F) panic!("500 is a refusal")
crates/pull/src/http.rs:7904 (F) panic!("a 307 is still a redirect this build refuses")
crates/pull/src/http.rs:7935 (F) panic!("a 301 is still a redirect this build refuses")
crates/pull/src/http.rs:7959 (F) panic!("an empty object holds no bars and must be refused")
crates/pull/src/http.rs:7980 (F) panic!("the sync seam cannot work and must say so")
crates/pull/src/http.rs:8019 (F) panic!("{bad} is not a whole count and must be refused, not truncated")
crates/pull/src/http.rs:8039 (F) panic!("an unreachable host is a transport failure, not a window")
crates/pull/src/http.rs:8057 (F) panic!("9999-12-31 has no day after it")
crates/pull/src/http.rs:8140 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
crates/pull/src/http.rs:8161 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is a broker"),
crates/pull/src/http.rs:8343 (F) panic!(
crates/pull/src/http.rs:8423 (F) panic!("a bar with no close must be refused, not defaulted")
crates/pull/src/http.rs:8462 (F) panic!("the declared envelope is absent and must be named")
crates/pull/src/http.rs:8532 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/http.rs:8613 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/http.rs:8654 (F) panic!("Groww is an HTTP broker");
crates/pull/src/http.rs:8684 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:8695 (F) panic!("a body naming DH-901 is a refusal whatever the status says")
crates/pull/src/http.rs:8722 (F) crate::vendor::Transport::LocalArchive(_) => panic!("this feed is HTTP"),
crates/pull/src/http.rs:8758 (F) panic!("a refusal under a 200 is still a refusal")
crates/pull/src/http.rs:8770 (F) panic!("a body that is not JSON fails the decode")
crates/pull/src/http.rs:8788 (F) panic!("this feed is HTTP")
crates/pull/src/http.rs:8915 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/http.rs:4132:17 False (F) let crate::vendor::Transport::Http(spec) = feed.descriptor().transport else {
crates/pull/src/http.rs:4560:13 False (F) let Err(FetchError::TransportFailed { detail }) = refused else {
crates/pull/src/http.rs:6516:13 False (F) zoneless.bars.len() != landed.bars.len()
crates/pull/src/http.rs:6634:31 False (F) .any(|(h, v)| *h == "X-Kite-Version" && *v == "3"),
crates/pull/src/http.rs:6878:13 False (F) let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
crates/pull/src/http.rs:6925:13 False (F) let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
crates/pull/src/http.rs:6969:13 False (F) let Err(FetchError::TransportFailed { detail }) = outcome else {
crates/pull/src/http.rs:7087:13 False (F) refusal.detail.contains("ceiling")
crates/pull/src/http.rs:7088:20 False (F) && refusal.detail.contains(&MAX_RESPONSE_BYTES.to_string()),
crates/pull/src/http.rs:7607:13 False (F) let FetchError::TransportFailed { detail } = why else {
crates/pull/src/http.rs:7878:13 False (F) let Err(FetchError::VendorRefused { status, detail, .. }) = outcome else {
crates/pull/src/http.rs:7979:13 False (F) let Err(FetchError::TransportFailed { detail }) = source.window(&one_day()) else {
crates/pull/src/http.rs:8342:13 False (F) let Err(FetchError::RungNotSpellable { rung, field }) = source.block_on(&daily) else {
crates/pull/src/http.rs:9181:86 False (F) matches!(unanswered.error, FetchError::TransportFailed { ref detail } if detail.contains("Refused before the body was read")),
crates/pull/src/ingest.rs:6430 (F) field("why")
crates/pull/src/ingest.rs:6478 (F) panic!("a lock that cannot be opened must refuse, never run unlocked")
crates/pull/src/ingest.rs:6553 (F) panic!("a lock that could not be created must refuse, never run unlocked")
crates/pull/src/ingest.rs:6561 (F) panic!("a regular lock that would not open must refuse, never run unlocked")
crates/pull/src/ingest.rs:6657 (F) panic!("the premise: a held census lock refuses a second run")
crates/pull/src/ingest.rs:6663 (F) panic!("the dropped lock was released despite the duplicate: {why}")
crates/pull/src/ingest.rs:6704 (F) panic!("a FIFO at the lock path must refuse promptly, got {answer:?}")
crates/pull/src/ingest.rs:6748 (F) panic!(
crates/pull/src/ingest.rs:6750 (F) path.display(),
crates/pull/src/ingest.rs:6751 (F) tried.join("; ")
crates/pull/src/ingest.rs:6776 (F) found.file_type()
crates/pull/src/ingest.rs:6783 (F) link.display()
crates/pull/src/ingest.rs:6836 (F) panic!("a socket at the lock path must refuse, got {refused:?}")
crates/pull/src/ingest.rs:6849 (F) panic!("a dangling symlink at the lock path must refuse, got {refused:?}")
crates/pull/src/ingest.rs:6477:13 False (F) let Err(why) = CensusLock::take(&census) else {
crates/pull/src/ingest.rs:6552:13 False (F) let Err(why) = CensusLock::unopened(&lock_path, &full) else {
crates/pull/src/ingest.rs:6560:13 False (F) let Err(why) = CensusLock::unopened(&lock_path, &starved) else {
crates/pull/src/ingest.rs:6656:13 False (F) let Err(busy) = CensusLock::take(&census) else {
crates/pull/src/ingest.rs:6703:13 False (F) let Ok(Err(why)) = answer else {
crates/pull/src/ingest.rs:6768:46 False (F) matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
crates/pull/src/ingest.rs:6835:13 False (F) let Err(why) = refused else {
crates/pull/src/ingest.rs:6848:13 False (F) let Err(why) = refused else {
crates/pull/src/landing_run_tests.rs:62 (F) Transport::LocalArchive(_) => panic!("Zerodha is an HTTP feed"),
crates/pull/src/landing_run_tests.rs:387 (F) mode.word()
crates/pull/src/landing_run_tests.rs:1149 (F) ledger_text(&root, "repairs")
crates/pull/src/landing_run_tests.rs:1324 (F) before.len()
crates/pull/src/landing_run_tests.rs:1676 (F) notes[0]
crates/pull/src/landing_run_tests.rs:1866 (F) hook.series_room()
crates/pull/src/landing_run_tests.rs:2227 (F) panic!("{on} is not a session of record");
crates/pull/src/landing_run_tests.rs:2308 (F) dir.display()
crates/pull/src/landing_run_tests.rs:345:9 False (F) gdfl.windows(2).all(|pair| pair[0] < pair[1])
crates/pull/src/landing_run_tests.rs:346:16 False (F) && zerodha.windows(2).all(|pair| pair[0] < pair[1]),
crates/pull/src/landing_run_tests.rs:394:26 False (F) .all(|entry| entry.mode == "import" && entry.by == "cli"),
crates/pull/src/landing_run_tests.rs:675:21 False (F) grown.starts_with(&claims)
crates/pull/src/landing_run_tests.rs:676:28 False (F) && grown.lines().count() == 3
crates/pull/src/landing_run_tests.rs:677:28 False (F) && grown
crates/pull/src/landing_run_tests.rs:903:13 False (F) said.contains(&format!("run {wider} is past {largest}"))
crates/pull/src/landing_run_tests.rs:904:20 False (F) && said.contains("32-bit run field (offset 60)")
crates/pull/src/landing_run_tests.rs:905:20 False (F) && said.contains("nothing is asked of the vendor"),
crates/pull/src/landing_run_tests.rs:1519:67 False (F) matches!(hooked.kept, Err(NotKept::RunTooWide { run }) if run == u64::from(u32::MAX) + 1)
crates/pull/src/listing_tests.rs:146 (F) panic!("a master span names its master");
crates/pull/src/listing_tests.rs:491 (F) Err(why) => panic!("an odd name: {why}"),
crates/pull/src/listing_tests.rs:145:9 False (F) let Source::NseSecurityMaster { file, sha256 } = span.source else {
crates/pull/src/master_snapshots_first_listed_tests.rs:515 (F) other @ Rebuilt::Done(..) => panic!("{other:?}"),
crates/pull/src/master_snapshots_first_listed_tests.rs:279:13 False (F) listed.refused[0]
crates/pull/src/master_snapshots_first_listed_tests.rs:281:20 False (F) && listed.refused[0].contains("recorded when the futures index took it in"),
crates/pull/src/master_snapshots_first_listed_tests.rs:448:13 False (F) said.starts_with("generation g1 of the futures index did not match its commit log")
crates/pull/src/master_snapshots_first_listed_tests.rs:449:20 False (F) && said.contains(why)
crates/pull/src/master_snapshots_first_listed_tests.rs:450:20 False (F) && said.contains("generation g2 was started"),
crates/pull/src/master_snapshots_tests.rs:154 (F) panic!("the first date resolves");
crates/pull/src/master_snapshots_tests.rs:161 (F) panic!("the second date resolves");
crates/pull/src/master_snapshots_tests.rs:184 (F) other => panic!("a reused token between snapshots is ambiguous, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:192 (F) panic!("an agreeing token resolves");
crates/pull/src/master_snapshots_tests.rs:198 (F) panic!("after the newest snapshot the newest answers");
crates/pull/src/master_snapshots_tests.rs:211 (F) other => panic!("before the first snapshot nothing resolves, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:284 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:305 (F) panic!("the receipted refresh answers");
crates/pull/src/master_snapshots_tests.rs:761 (F) k1.json()
crates/pull/src/master_snapshots_tests.rs:847 (F) other => panic!("a torn snapshot is unreadable, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:924 (F) panic!("one row is found");
crates/pull/src/master_snapshots_tests.rs:944 (F) panic!("found");
crates/pull/src/master_snapshots_tests.rs:996 (F) other => panic!("resolves, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:1013 (F) other => panic!("expired, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:1051 (F) other => panic!("{on}: expired, got {other:?}"),
crates/pull/src/master_snapshots_tests.rs:1057 (F) matches!(
crates/pull/src/master_snapshots_tests.rs:1065 (F) matches!(
crates/pull/src/master_snapshots_tests.rs:1225 (F) outcome.json()
crates/pull/src/master_snapshots_tests.rs:1245 (F) panic!("set BRUTEX_KITE_DUMP to a copy of a landed Kite dump");
crates/pull/src/master_snapshots_tests.rs:1341 (F) panic!("the day's newest snapshot agrees, so the older one is believed");
crates/pull/src/master_snapshots_tests.rs:1359 (F) answer => panic!("the day's newest disagrees, so ambiguous; got {answer:?}"),
crates/pull/src/master_snapshots_tests.rs:1390 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:1437 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:1472 (F) panic!("a diff against the day before: {:?}", outcome.diff);
crates/pull/src/master_snapshots_tests.rs:1516 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:1623 (F) book.incomplete()
crates/pull/src/master_snapshots_tests.rs:1637 (F) book.incomplete()
crates/pull/src/master_snapshots_tests.rs:1672 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:1736 (F) catalog.incomplete()
crates/pull/src/master_snapshots_tests.rs:1959 (F) panic!("read in any column order");
crates/pull/src/master_snapshots_tests.rs:1970 (F) panic!("found");
crates/pull/src/master_snapshots_tests.rs:2002 (F) panic!("the row beside it answers");
crates/pull/src/master_snapshots_tests.rs:191:9 False (F) let TokenOn::Resolved { snapshot, listed } = book.token_on(between, "NFO", 9_000_002) else {
crates/pull/src/master_snapshots_tests.rs:197:9 False (F) let TokenOn::Resolved { listed, .. } = book.token_on(day(2026, 2, 20), "NFO", 9_000_001) else {
crates/pull/src/master_snapshots_tests.rs:304:9 False (F) let TokenOn::Resolved { listed, .. } = book.token_on(d1, "NFO", 9_000_021) else {
crates/pull/src/master_snapshots_tests.rs:362:9 False (F) json.contains(r#""incomplete":{"held":3,"named":[{"file":"instruments_20260304.csv.gz","stamp":"2026-03-04","why":"a body with no receipt (a
crates/pull/src/master_snapshots_tests.rs:363:16 False (F) && json.contains(r#""unnamed":0,"refusal":null}"#),
crates/pull/src/master_snapshots_tests.rs:823:9 False (F) why.contains("after today 2026-02-01")
crates/pull/src/master_snapshots_tests.rs:824:16 False (F) && why.contains(&file_name(Kind::Kite, stamp(d2, 1)))
crates/pull/src/master_snapshots_tests.rs:825:16 False (F) && why.contains("refused rather than believed fresh"),
crates/pull/src/master_snapshots_tests.rs:923:9 False (F) let Lookup::Listed(listed) = one.lookup("NFO", 9_800_001) else {
crates/pull/src/master_snapshots_tests.rs:943:9 False (F) let Lookup::Listed(again) = other.lookup("NFO", 9_800_001) else {
crates/pull/src/master_snapshots_tests.rs:1387:16 False (F) && one.to_string().starts_with(&partial)
crates/pull/src/master_snapshots_tests.rs:1471:9 False (F) let Some(Ok(Some(ref summary))) = outcome.diff else {
crates/pull/src/master_snapshots_tests.rs:1511:47 False (F) catalog.incomplete().iter().any(|one| one.file == cut
crates/pull/src/master_snapshots_tests.rs:1512:16 False (F) && one.kind == Kind::Kite
crates/pull/src/master_snapshots_tests.rs:1513:16 False (F) && one.stamp == b
crates/pull/src/master_snapshots_tests.rs:1635:24 False (F) .any(|one| one.stamp == cut && one.why.contains("no receipt")),
crates/pull/src/master_snapshots_tests.rs:1670:24 False (F) .any(|one| one.file == twin && one.why.contains("another case")),
crates/pull/src/master_snapshots_tests.rs:1734:24 False (F) .any(|one| one.file == upper && one.why == CUT_DIFF),
crates/pull/src/master_snapshots_tests.rs:1958:9 False (F) let Lookup::Listed(listed) = one.lookup("NFO", 9_800_101) else {
crates/pull/src/master_snapshots_tests.rs:1969:9 False (F) let Lookup::Listed(first) = parsed.lookup("NFO", 9_800_102) else {
crates/pull/src/master_snapshots_tests.rs:2001:9 False (F) let Lookup::Listed(other) = parsed.lookup("NFO", 9_900_102) else {
crates/pull/src/masters.rs:1973 (F) why.chars().take(200).collect::<String>()
crates/pull/src/masters.rs:1981 (F) why.chars().take(200).collect::<String>()
crates/pull/src/masters.rs:2059 (F) panic!("{name} must be refused");
crates/pull/src/masters.rs:2122 (F) panic!("a short body must be refused");
crates/pull/src/masters.rs:2141 (F) panic!("a long HTML page is not a CSV");
crates/pull/src/masters.rs:2173 (F) panic!("JSON is not the CSV a master is");
crates/pull/src/masters.rs:2295 (F) panic!("the first write lands");
crates/pull/src/masters.rs:2301 (F) panic!("the second write lands");
crates/pull/src/masters.rs:2309 (F) panic!("the third write lands");
crates/pull/src/masters.rs:2345 (F) panic!("the identical landing is still written");
crates/pull/src/masters.rs:2366 (F) panic!("the repair lands");
crates/pull/src/masters.rs:2376 (F) panic!("the repair lands");
crates/pull/src/masters.rs:2404 (F) panic!("an unreadable data row must refuse");
crates/pull/src/masters.rs:2543 (F) panic!("a malformed row refuses");
crates/pull/src/masters.rs:3159 (F) matches!(
crates/pull/src/masters.rs:3189 (F) other => panic!("expected a refusal, got {other:?}"),
crates/pull/src/masters.rs:3397 (F) unreachable!("an unconvertible body cannot be written")
crates/pull/src/masters.rs:3423 (F) unreachable!("a body with no columns cannot be a master")
crates/pull/src/masters.rs:3565 (F) unreachable!("a master the reader cannot read must not be written")
crates/pull/src/masters.rs:3632 (F) panic!("a target that is not a regular file is refused: {landed:?}");
crates/pull/src/masters.rs:1954:56 False (F) source.file == NSE_INDICES_FILE || source.file == NSE_TOTAL_MARKET_FILE,
crates/pull/src/masters.rs:2058:17 False (F) let Landed::Refused(why) = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2121:13 False (F) let Landed::Refused(why) = outcome else {
crates/pull/src/masters.rs:2140:13 False (F) let Landed::Refused(why) = land(&dir, &SOURCES[0], &html) else {
crates/pull/src/masters.rs:2172:13 False (F) let Landed::Refused(why) = land(&dir, &SOURCES[0], &json) else {
crates/pull/src/masters.rs:2294:13 False (F) let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2300:13 False (F) let Landed::Written { changed, .. } = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2308:13 False (F) let Landed::Written { changed, .. } = land(&dir, source, &moved) else {
crates/pull/src/masters.rs:2344:13 False (F) let Landed::Written { changed, bytes } = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2365:13 False (F) let Landed::Written { changed, .. } = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2375:13 False (F) let Landed::Written { changed, .. } = land(&dir, source, &body) else {
crates/pull/src/masters.rs:2403:17 False (F) let Landed::Refused(why) = land(&dir, source, &candidate) else {
crates/pull/src/masters.rs:2542:13 False (F) let Landed::Refused(why) = land(&dir, source, &bad) else {
crates/pull/src/masters.rs:3396:13 False (F) let Landed::Refused(ref why) = landed else {
crates/pull/src/masters.rs:3422:13 False (F) let Landed::Refused(ref why) = landed else {
crates/pull/src/masters.rs:3564:13 False (F) let Landed::Refused(ref why) = landed else {
crates/pull/src/masters.rs:3631:13 False (F) let Landed::Refused(ref why) = landed else {
crates/pull/src/nse.rs:885 (F) panic!("an interstitial must refuse as NotCsv, got {why:?}");
crates/pull/src/nse.rs:1258 (F) panic!("got {why:?}");
crates/pull/src/nse.rs:884:13 False (F) let NseError::NotCsv { ref head } = why else {
crates/pull/src/nse.rs:1026:51 False (F) NseError::HeaderUnexpected { got } if got.len() == MAX_DOCUMENT_BYTES
crates/pull/src/nse.rs:1217:72 False (F) matches!(refused, NseError::TooManyIndexLinks { links } if links > MAX_INDEX_LINKS),
crates/pull/src/nse.rs:1257:13 False (F) let NseError::NoConstituentLink { ref page } = why else {
crates/pull/src/padded_tests.rs:305 (F) panic!("2024-03-02 is a recorded special session: {session:?}");
crates/pull/src/padded_tests.rs:304:9 False (F) let DayKind::Open(s) = session else {
crates/pull/src/pricing.rs:1421 (F) matches!(
crates/pull/src/pricing.rs:1434 (F) matches!(
crates/pull/src/pricing.rs:1616 (F) matches!(
crates/pull/src/pricing.rs:1873 (F) matches!(
crates/pull/src/pricing.rs:1361:64 False (F) Err(PricingError::RateNotFinite { annual }) if annual.is_nan()
crates/pull/src/pricing.rs:1399:79 False (F) matches!(error, PricingError::NotPositive { field: f, .. } if f == field),
crates/pull/src/pricing.rs:1928:77 False (F) matches!(why, PricingError::NotPositive { field: f, .. } if f == field),
crates/pull/src/reference_data_tests.rs:428 (F) other => panic!("{other:?}"),
crates/pull/src/reference_data_tests.rs:481 (F) other => panic!("{other:?}"),
crates/pull/src/reference_data_tests.rs:592 (F) mine[0]
crates/pull/src/reference_data_tests.rs:774 (F) other => panic!("a span past MAX_ROWS days must be refused: {other:?}"),
crates/pull/src/reference_data_tests.rs:866 (F) Err(other) => panic!("{other}"),
crates/pull/src/reference_data_tests.rs:1028 (F) panic!("BRUTEX_R330_RATES names no file");
crates/pull/src/reference_data_tests.rs:1060 (F) Err(why) => panic!("{why}"),
crates/pull/src/rehearsal_tests.rs:277 (F) why.len()
crates/pull/src/repair_plan.rs:954 (F) other => panic!("{other:?}"),
crates/pull/src/repair_plan.rs:1004:39 False (F) assert!(months.iter().all(|m| m.bars == 0 && m.refused.is_none()));
crates/pull/src/repair_plan_not_owed_tests.rs:93 (F) panic!("{on}: a landed session states its windows");
crates/pull/src/repair_plan_not_owed_tests.rs:172 (F) panic!("{}: {:?}", session.day, session.why);
crates/pull/src/repair_plan_not_owed_tests.rs:171:13 False (F) let NotOwed::InstrumentInactive(interval) = session.why else {
crates/pull/src/repair_plan_not_owed_tests.rs:282:22 False (F) .all(|m| !m.needs_repair() && m.refused.is_none())
crates/pull/src/request_minutes.rs:1320 (F) texts(&none)
crates/pull/src/request_minutes.rs:1360 (F) panic!("{day} is attested")
crates/pull/src/request_minutes.rs:1479 (F) groups[1].text
crates/pull/src/request_minutes.rs:1879 (F) panic!("{:?}", audit.findings);
crates/pull/src/request_minutes.rs:1949 (F) panic!("{groups:?}");
crates/pull/src/request_minutes.rs:2056 (F) panic!("{:?}", audit.days);
crates/pull/src/request_minutes.rs:2134 (F) panic!("{:?}", audit.findings);
crates/pull/src/request_minutes.rs:2178 (F) panic!("{:?}", done.failures);
crates/pull/src/request_minutes.rs:2200 (F) lines[0]
crates/pull/src/request_minutes.rs:2236 (F) named.failures[1].why
crates/pull/src/request_minutes.rs:2253 (F) panic!("{day}: {:?}", audit.findings);
crates/pull/src/request_minutes.rs:2287 (F) panic!("{:?}", audit.days);
crates/pull/src/request_minutes.rs:2349 (F) panic!("{:?}", unflagged.findings);
crates/pull/src/request_minutes.rs:1475:13 False (F) groups[1].text.starts_with(
crates/pull/src/request_minutes.rs:1477:18 False (F) ) && groups[1].text.ends_with("the first: gap b"),
crates/pull/src/request_minutes.rs:1878:13 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:1886:13 False (F) finding
crates/pull/src/request_minutes.rs:1889:20 False (F) && finding
crates/pull/src/request_minutes.rs:1948:13 False (F) let [group] = groups.as_slice() else {
crates/pull/src/request_minutes.rs:2055:13 False (F) let [line] = audit.days.as_slice() else {
crates/pull/src/request_minutes.rs:2133:13 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:2177:13 False (F) let [failure] = done.failures.as_slice() else {
crates/pull/src/request_minutes.rs:2252:17 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:2286:13 False (F) let [line] = audit.days.as_slice() else {
crates/pull/src/request_minutes.rs:2348:13 False (F) let [finding] = unflagged.findings.as_slice() else {
crates/pull/src/resolve.rs:1033 (F) snap.unlinked[0]
crates/pull/src/resolve.rs:1040 (F) snap.unlinked[0]
crates/pull/src/resolve.rs:1045 (F) snap.unlinked[0]
crates/pull/src/response_keep_tests.rs:325 (F) panic!("{refused:?}");
crates/pull/src/response_keep_tests.rs:925 (F) matches!(
crates/pull/src/response_keep_tests.rs:966 (F) panic!("{refused:?}");
crates/pull/src/response_keep_tests.rs:1605 (F) started.elapsed()
crates/pull/src/response_keep_tests.rs:2001 (F) panic!("one further month: {outcome:?}");
crates/pull/src/response_keep_tests.rs:2089 (F) matches!(
crates/pull/src/response_keep_tests.rs:2127 (F) panic!("July holds exactly the missing pointer: {july:?}");
crates/pull/src/response_keep_tests.rs:324:9 False (F) let Err(KeepError::ReceiptsFull { pack: named }) = &refused else {
crates/pull/src/response_keep_tests.rs:2000:9 False (F) let [july] = outcome.further.as_slice() else {
crates/pull/src/response_keep_tests.rs:2126:9 False (F) let [pointer] = july.as_slice() else {
crates/pull/src/riskfree.rs:1593 (F) holed.on(asked)
crates/pull/src/riskfree.rs:1829:56 False (F) matches!(tails.as_slice(), [small, big] if big.1 <= small.1.saturating_mul(10).max(2_000)),
crates/pull/src/rolling.rs:1084 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/rolling.rs:1410 (F) matches!(
crates/pull/src/rolling.rs:1470 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/rolling.rs:1083:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/rolling.rs:1469:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/rolling.rs:1546:52 False (F) Err(RollingError::NoExpiry { why }) if why.contains("not one this vendor serves")
crates/pull/src/rolling.rs:1565:60 False (F) Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
crates/pull/src/rolling.rs:1602:60 False (F) Err(RollingError::NoExpiry { why }) if why.contains("full regular session")
crates/pull/src/rolling.rs:1615:52 False (F) Err(RollingError::NoExpiry { why }) if why.contains("marks closed")
crates/pull/src/rolling.rs:1727:77 False (F) Err(RollingError::Uncountable { field: "oi", text }) if text == cell
crates/pull/src/rolling.rs:1789:95 False (F) matches!(&got, Err(RollingError::Unstampable { field: "timestamp", text }) if *text == stamp),
crates/pull/src/rolling.rs:1810:92 False (F) matches!(&got, Err(RollingError::Uncountable { field: "volume", text }) if text == volume),
crates/pull/src/rolling.rs:1844:89 False (F) matches!(&got, Err(RollingError::NotAPrice { field: "close", text }) if text == close),
crates/pull/src/scrub.rs:759:31 False (F) i == j || one != other,
crates/pull/src/session.rs:1951 (F) c.from(),
crates/pull/src/session.rs:1952 (F) c.to()
crates/pull/src/session.rs:2165 (F) panic!("there is no December 1969 in this calendar")
crates/pull/src/session.rs:1949:76 False (F) c.to() == c.to().end_of_month() || c.to() == to || inside_a_wide_month,
crates/pull/src/session.rs:2164:13 False (F) let Err(why) = first.months_before(1) else {
crates/pull/src/ssm.rs:1370 (F) token.is_some()
crates/pull/src/ssm.rs:1458 (F) panic!("{region:?} is not ap-south-1 and must not be signed for")
crates/pull/src/ssm.rs:1487 (F) panic!("a stamp with no date in it cannot scope a signature")
crates/pull/src/ssm.rs:1529 (F) panic!("{wrong} carries no Parameter.Value and must be refused")
crates/pull/src/ssm.rs:1534 (F) panic!("a non-JSON answer is a different fault and is named as one")
crates/pull/src/ssm.rs:1636 (F) panic!("a profile that is not in the file must be refused")
crates/pull/src/ssm.rs:1647 (F) panic!("an id with no secret cannot sign anything")
crates/pull/src/ssm.rs:1673 (F) panic!("{tag}: a blank half of the pair is no pair")
crates/pull/src/ssm.rs:1700 (F) panic!("an empty exported key id must not become an identity")
crates/pull/src/ssm.rs:1747 (F) panic!("a half-set environment must be refused: {found:?}")
crates/pull/src/ssm.rs:1785 (F) panic!("a file that is not there cannot hold an identity")
crates/pull/src/ssm.rs:1849 (F) panic!("a foreign region must not yield a parameter value")
crates/pull/src/ssm.rs:1869 (F) panic!("{bad} not UTF-8 must be refused")
crates/pull/src/ssm.rs:1937 (F) panic!("a FIFO credentials file must refuse promptly, got {answer:?}")
crates/pull/src/ssm.rs:1972 (F) panic!("one byte past the bound must be refused")
crates/pull/src/ssm.rs:1982 (F) panic!("a file that is not text holds no profile")
crates/pull/src/ssm.rs:2034 (F) panic!("one byte past the cap must be refused")
crates/pull/src/ssm.rs:2041 (F) panic!("a body cut short is not an answer")
crates/pull/src/ssm.rs:1366:45 False (F) FIXED.contains(name) || sent.contains(name),
crates/pull/src/ssm.rs:1457:17 False (F) let Err(SsmError { detail, kind }) = foreign.authorization(&identity()) else {
crates/pull/src/ssm.rs:1486:13 False (F) let Err(SsmError { detail, .. }) = bad.authorization(&identity()) else {
crates/pull/src/ssm.rs:1528:17 False (F) let Err(SsmError { detail, .. }) = value_of(wrong) else {
crates/pull/src/ssm.rs:1533:13 False (F) let Err(SsmError { detail, .. }) = value_of("not json") else {
crates/pull/src/ssm.rs:1635:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "nope") else {
crates/pull/src/ssm.rs:1746:17 False (F) let Err(SsmError { detail, .. }) = found else {
crates/pull/src/ssm.rs:1814:38 False (F) .any(|(name, value)| *name == "content-type"
crates/pull/src/ssm.rs:1868:17 False (F) let Err(SsmError { detail, .. }) = refuse_non_unicode(read) else {
crates/pull/src/ssm.rs:1936:13 False (F) let Ok(Err(SsmError { detail, kind })) = answer else {
crates/pull/src/ssm.rs:2033:13 False (F) let Err(SsmError { detail, kind }) = answered("", vec![b'a'; MAX_ANSWER_BYTES + 1]) else {
crates/pull/src/ssm.rs:2040:13 False (F) let Err(SsmError { detail, .. }) = answered("Content-Length: 4\r\n", b"ab".to_vec()) else {
crates/pull/src/ssm_cov_tests.rs:55:9 True (F) set("AWS_ACCESS_KEY_ID") && set("AWS_SECRET_ACCESS_KEY")
crates/pull/src/tenor.rs:490 (F) at.years(YearBasis::Calendar365)
crates/pull/src/tenor.rs:510 (F) born.years(YearBasis::Calendar365)
crates/pull/src/tickstore_write_tests.rs:232 (F) panic!("written: {:?}", run.result);
crates/pull/src/tickstore_write_tests.rs:359 (F) panic!("{first:?}");
crates/pull/src/tickstore_write_tests.rs:369 (F) panic!("{again:?}");
crates/pull/src/tickstore_write_tests.rs:399 (F) panic!("{first:?}");
crates/pull/src/tickstore_write_tests.rs:407 (F) panic!("{again:?}");
crates/pull/src/tickstore_write_tests.rs:573 (F) data.len()
crates/pull/src/tickstore_write_tests.rs:594 (F) panic!("{run:?}");
crates/pull/src/tickstore_write_tests.rs:699 (F) panic!("byte {at}: {run:?}");
crates/pull/src/tickstore_write_tests.rs:861 (F) matches!(
crates/pull/src/tickstore_write_tests.rs:904 (F) panic!("{held:?}");
crates/pull/src/tickstore_write_tests.rs:911 (F) panic!("{ours:?}");
crates/pull/src/tickstore_write_tests.rs:971 (F) panic!("{run:?}");
crates/pull/src/tickstore_write_tests.rs:1086 (F) String::from_utf8_lossy(field)
crates/pull/src/tickstore_write_tests.rs:1135 (F) encoded.block.len(),
crates/pull/src/tickstore_write_tests.rs:1136 (F) text.len()
crates/pull/src/tickstore_write_tests.rs:1308 (F) Err(why) => panic!("{name}: {why}"),
crates/pull/src/tickstore_write_tests.rs:1576 (F) log_of(&root)
crates/pull/src/tickstore_write_tests.rs:1589 (F) panic!("{run:?}");
crates/pull/src/tickstore_write_tests.rs:1699 (F) panic!("{run:?}");
crates/pull/src/tickstore_write_tests.rs:1715 (F) panic!("{first:?}");
crates/pull/src/tickstore_write_tests.rs:1729 (F) panic!("{again:?}");
crates/pull/src/tickstore_write_tests.rs:1743 (F) panic!("{dated:?}");
crates/pull/src/tickstore_write_tests.rs:231:9 False (F) let Ok(Outcome::Written(report)) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:358:9 False (F) let Ok(Outcome::Written(written)) = first.result else {
crates/pull/src/tickstore_write_tests.rs:368:13 False (F) let Ok(Outcome::Held(held)) = again.result else {
crates/pull/src/tickstore_write_tests.rs:398:9 False (F) let Ok(Outcome::Written(written)) = first.result else {
crates/pull/src/tickstore_write_tests.rs:406:9 False (F) let Err(Refusal::HeldDayDiffers { path, why }) = &again.result else {
crates/pull/src/tickstore_write_tests.rs:537:17 False (F) at + if checksum { 4 } else { 0 },
crates/pull/src/tickstore_write_tests.rs:593:9 False (F) let Err(Refusal::MemberCorrupt { name, why }) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:698:13 False (F) let Err(Refusal::VerifyFailed { temp, .. }) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:903:9 False (F) let Ok(Outcome::Held(report)) = &held.result else {
crates/pull/src/tickstore_write_tests.rs:910:9 False (F) let Ok(Outcome::Written(mine)) = &ours.result else {
crates/pull/src/tickstore_write_tests.rs:970:9 False (F) let Err(Refusal::HeldDayDiffers { why, .. }) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:1588:9 False (F) let Err(Refusal::TempNotCreated { word, .. }) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:1698:9 False (F) let Ok(Outcome::Written(report)) = &run.result else {
crates/pull/src/tickstore_write_tests.rs:1714:9 False (F) let Ok(Outcome::Written(written)) = first.result else {
crates/pull/src/tickstore_write_tests.rs:1728:9 False (F) let Err(Refusal::ZipCorruptOnVerify { path, why }) = &again.result else {
crates/pull/src/tickstore_write_tests.rs:1742:9 False (F) let Err(Refusal::HeldDayDiffers { why, .. }) = &dated.result else {
crates/pull/src/universe.rs:798 (F) master.len(),
crates/pull/src/universe.rs:799 (F) index.len(),
crates/pull/src/universe.rs:800 (F) index.capacity()
crates/pull/src/upkeep_tests.rs:372 (F) lines[0]
crates/pull/src/upkeep_tests.rs:380 (F) lines[4]
crates/pull/src/upkeep_tests.rs:388 (F) lines[6]
crates/pull/src/upkeep_tests.rs:398 (F) lines[6]
crates/pull/src/upkeep_tests.rs:521:9 False (F) why.contains("another srt upkeep run of gdfl holds")
crates/pull/src/upkeep_tests.rs:522:16 False (F) && why.contains("one run of a kind at a time"),
crates/pull/src/upkeep_tests.rs:1042:9 False (F) why.starts_with(&format!("{}: ", gone.join("audit").display()))
crates/pull/src/upkeep_tests.rs:1043:16 False (F) && why.ends_with("; nothing was written"),
crates/pull/src/upkeep_tests.rs:1399:9 False (F) why.starts_with("NSE/FNO/NIFTY/2019-03-28-1160000-CE names nothing under bars/gdfl: ")
crates/pull/src/upkeep_tests.rs:1400:16 False (F) && why.ends_with("2019-03-28-1160000-CE is absent"),
crates/pull/src/upkeep_tests.rs:1941:9 False (F) text.contains(
crates/pull/src/upkeep_tests.rs:1944:14 False (F) ) && text.contains("end 300 how=stopped n=2 ok=2 of=6 t="),
crates/pull/src/vendor.rs:6890 (F) other => panic!("{rung} sits on {other:?}, which the store cannot file"),
crates/pull/src/vendor.rs:7011 (F) _ => false,
crates/pull/src/vendor.rs:7082 (F) feed.display()
crates/pull/src/vendor.rs:7090 (F) feed.display()
crates/pull/src/vendor.rs:7096 (F) feed.display()
crates/pull/src/vendor.rs:7124 (F) feed.display()
crates/pull/src/vendor.rs:7163 (F) feed.display()
crates/pull/src/vendor.rs:7203 (F) feed.display()
crates/pull/src/vendor.rs:7262 (F) feed.display()
crates/pull/src/vendor.rs:7267 (F) feed.display()
crates/pull/src/vendor.rs:7746 (F) panic!("Groww is an HTTP feed");
crates/pull/src/vendor.rs:7764 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/vendor.rs:8155 (F) panic!(
crates/pull/src/vendor.rs:8159 (F) feed.display()
crates/pull/src/vendor.rs:8166 (F) feed.display()
crates/pull/src/vendor.rs:8206 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/vendor.rs:8394 (F) spec.path_template()
crates/pull/src/vendor.rs:9055 (F) other.standing.label()
crates/pull/src/vendor.rs:9330 (F) panic!("Groww is an HTTP feed");
crates/pull/src/vendor.rs:7088:37 True (F) !(named_in_query && named_in_path),
crates/pull/src/vendor.rs:7677:39 False (F) assert!(i == j || one != other, "offset {one} appears twice");
crates/pull/src/vendor.rs:7745:13 False (F) let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
crates/pull/src/vendor.rs:7763:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/vendor.rs:8205:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/vendor.rs:8684:13 False (F) !TRUEDATA_INDEX.columns.contains(&Column::Volume)
crates/pull/src/vendor.rs:8685:20 False (F) && !TRUEDATA_INDEX.columns.contains(&Column::OpenInterest),
crates/pull/src/vendor.rs:8950:48 False (F) row.contested.is_none() || !row.binds_because.is_empty(),
crates/pull/src/vendor.rs:9329:13 False (F) let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
crates/pull/src/verdict_words.rs:256 (F) cause.code()
crates/pull/src/verdict_words.rs:184:21 False (F) !word.is_empty()
crates/pull/src/verdict_words.rs:185:28 False (F) && word
crates/pull/src/verdict_words.rs:187:64 False (F) .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit())
crates/pull/src/zerodha_verify_tests.rs:179 (F) Found::NoRecord(none) => panic!("no record: {none:?}"),
crates/pull/src/zerodha_verify_tests.rs:216 (F) panic!("a regular day owes windows");
crates/pull/src/zerodha_verify_tests.rs:757 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/zerodha_verify_tests.rs:1562 (F) panic!("the Muhurat of record owes windows");
crates/pull/src/zerodha_verify_tests.rs:2591 (F) other => panic!("a log past the cap is refused by name: {other:?}"),
crates/pull/src/zerodha_verify_tests.rs:2603 (F) other => panic!("a log of the cap is read: {other:?}"),
crates/pull/src/zerodha_verify_tests.rs:2955 (F) panic!("Zerodha is an HTTP feed");
crates/pull/src/zerodha_verify_tests.rs:4267 (F) first.live[0].2
crates/pull/src/zerodha_verify_tests.rs:4384 (F) spool::Live::Spooling(holder) => panic!("free run 15 read as live: {holder}"),
crates/pull/src/zerodha_verify_tests.rs:4770 (F) other => panic!("a full spool took the append: {other:?}"),
crates/pull/src/zerodha_verify_tests.rs:5102 (F) other => panic!("not the host's words: {other}"),
crates/pull/src/zerodha_verify_tests.rs:209:35 False (F) DayKind::Open(session) if session == calendar::Session::full()
crates/pull/src/zerodha_verify_tests.rs:215:9 False (F) let Owes::Windows(windows) = repair_plan::owed("INDEX", regular().days_from_epoch()) else {
crates/pull/tests/alerts_seams.rs:59 (F) Put::Duplicate { ordinal } => panic!("a fresh key was a duplicate of #{ordinal}"),
crates/pull/tests/allocation.rs:173 (F) wide.len()
crates/pull/tests/allocation.rs:303 (F) panic!("a frame cut to {len} bytes was {:?}", frame_answer.0);
crates/pull/tests/allocation.rs:323 (F) full.len()
crates/pull/tests/allocation.rs:302:17 False (F) let Err(FrameError::Truncated { part }) = frame_answer.0 else {
crates/pull/tests/anchor.rs:78 (F) first - OPEN_UTC
crates/pull/tests/anchor.rs:176 (F) panic!("known session")
crates/pull/tests/anchor.rs:442 (F) panic!("known session")
crates/pull/tests/anchor.rs:562 (F) diagnostics[0]
crates/pull/tests/anchor.rs:639 (F) matches!(
crates/pull/tests/anchor.rs:725 (F) first - midnight_edge
crates/pull/tests/anchor.rs:175:13 False (F) let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
crates/pull/tests/anchor.rs:441:9 False (F) let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
crates/pull/tests/broker.rs:397 (F) bars.display()
crates/pull/tests/broker.rs:765 (F) at("1day").display()
crates/pull/tests/broker.rs:927 (F) panic!("Groww is an HTTP broker");
crates/pull/tests/broker.rs:959 (F) panic!("Dhan is an HTTP broker");
crates/pull/tests/broker.rs:1088 (F) contract.as_str()
crates/pull/tests/broker.rs:926:9 False (F) let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
crates/pull/tests/broker.rs:958:9 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/tests/census.rs:304 (F) path.display()
crates/pull/tests/census.rs:334 (F) rungs(),
crates/pull/tests/census.rs:494 (F) rungs()
crates/pull/tests/census.rs:620 (F) done.failures[0].why
crates/pull/tests/census.rs:724 (F) done.failures[0].why
crates/pull/tests/census.rs:743 (F) done.failures[0].why
crates/pull/tests/census.rs:750 (F) done.failures[0].why
crates/pull/tests/census.rs:937 (F) done.failures[0].why
crates/pull/tests/census.rs:1104 (F) panic!(
crates/pull/tests/census.rs:1127 (F) store.join("bars").display()
crates/pull/tests/census.rs:1199 (F) n + 2
crates/pull/tests/census.rs:1222 (F) n + 2,
crates/pull/tests/census_day_pace.rs:129 (F) at + 1
crates/pull/tests/census_day_pace.rs:126:13 False (F) line.contains(&format!("\"read\":\"{read}\""))
crates/pull/tests/census_day_pace.rs:127:20 False (F) && line.contains(&format!("\"why\":\"{why}\"")),
crates/pull/tests/census_resume.rs:128 (F) panic!("one CASH row after day one: {:?}", cash_row(root));
crates/pull/tests/census_resume.rs:147 (F) panic!("{stopped:?}");
crates/pull/tests/census_resume.rs:191 (F) panic!("one CASH row after the recount: {:?}", cash_row(root));
crates/pull/tests/census_resume.rs:209 (F) panic!("one CASH row after day three: {:?}", cash_row(root));
crates/pull/tests/census_resume.rs:127:9 False (F) let [(rows_one, _)] = cash_row(root)[..] else {
crates/pull/tests/census_resume.rs:146:9 False (F) let ImportRefusal::CensusRefused { day: at, .. } = &stopped else {
crates/pull/tests/census_resume.rs:190:9 False (F) let [(rows_two, _)] = cash_row(root)[..] else {
crates/pull/tests/census_resume.rs:208:9 False (F) let [(rows_three, _)] = cash_row(root)[..] else {
crates/pull/tests/census_resume.rs:258:36 False (F) .is_some_and(|journal| journal.closed.is_empty() && journal.run.is_none()),
crates/pull/tests/cloud_mutants_import.rs:307 (F) other => panic!("a rotted record read as {other:?}"),
crates/pull/tests/cloud_mutants_import.rs:373 (F) other => panic!("{other:?}"),
crates/pull/tests/cloud_mutants_import.rs:402 (F) other => panic!("{other:?}"),
crates/pull/tests/cloud_mutants_import.rs:854 (F) panic!(
crates/pull/tests/cloud_mutants_import.rs:856 (F) path.display(),
crates/pull/tests/cloud_mutants_import.rs:857 (F) tried.join("; ")
crates/pull/tests/cloud_mutants_import.rs:884 (F) link.display()
crates/pull/tests/cloud_mutants_import.rs:909 (F) other => panic!("{other:?}"),
crates/pull/tests/cloud_mutants_import.rs:1752 (F) lines[0]
crates/pull/tests/cloud_mutants_import.rs:497:9 False (F) said.starts_with("the snapshot stamped -42 cannot be shifted onto the IST day grid")
crates/pull/tests/cloud_mutants_import.rs:498:16 False (F) && said.contains("Refused rather than saturated"),
crates/pull/tests/cloud_mutants_import.rs:872:42 False (F) matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
crates/pull/tests/cloud_mutants_import.rs:976:9 False (F) gap.starts_with("gap NSE-INDEX-NIFTY 2025-07-01 v=1 rung=1min owed=375 held=374 missing=1")
crates/pull/tests/cloud_mutants_import.rs:977:16 False (F) && gap.ends_with("why=vendor_empty run=4242"),
crates/pull/tests/cloud_mutants_import.rs:1269:26 True (F) .any(|m| m.contains(".BE") || m.contains("ZZSHARE"))
crates/pull/tests/cloud_mutants_import.rs:1339:82 False (F) matches!(listing.auction_for(&cash("TCS")), Auction::Unknown(why) if !why.is_empty())
crates/pull/tests/cloud_mutants_import.rs:1343:38 False (F) Auction::Unknown(why) if why.contains("records no ISIN")
crates/pull/tests/cloud_mutants_import.rs:1415:38 False (F) Auction::Unknown(why) if why.contains("core::universe records no ISIN")
crates/pull/tests/derive.rs:164 (F) tf.as_str()
crates/pull/tests/derive.rs:407 (F) conflicts[0].why
crates/pull/tests/derive.rs:414 (F) conflicts[0].why
crates/pull/tests/derive.rs:612 (F) refusal.why.len(),
crates/pull/tests/derive.rs:613 (F) refusal.why.get(..300)
crates/pull/tests/derive.rs:687 (F) reports[0].why
crates/pull/tests/derive.rs:744 (F) audit[0].why
crates/pull/tests/derive.rs:876 (F) gaps[1]
crates/pull/tests/derive.rs:973 (F) done.failures[0].why
crates/pull/tests/derive.rs:1047 (F) done.failures[0].why
crates/pull/tests/derive.rs:1466 (F) tf.as_str()
crates/pull/tests/derive.rs:422:22 False (F) .all(|f| f.why.contains("incomplete or invalid minute coverage")
crates/pull/tests/derive.rs:468:22 False (F) .any(|f| f.why.contains("UNVERIFIED") && f.why.contains("cash eligibility"))
crates/pull/tests/derive.rs:736:9 False (F) audit[0]
crates/pull/tests/derive.rs:739:16 False (F) && audit[0]
crates/pull/tests/derive.rs:742:16 False (F) && audit[0].why.contains("observed trading does not attest"),
crates/pull/tests/derive.rs:747:9 False (F) audit[1].why.contains("2027-01-07")
crates/pull/tests/derive.rs:748:16 False (F) && audit[1]
crates/pull/tests/derive.rs:755:22 False (F) .any(|f| !f.why.starts_with("request minute coverage")
crates/pull/tests/derive.rs:1312:42 False (F) assert!(done.failures.iter().all(|f| !f.why.contains("versioned")
crates/pull/tests/derive.rs:1396:20 False (F) && f.why.contains(expected)
crates/pull/tests/folder.rs:269 (F) panic!("a folder holding two days must report them: {reach:?}")
crates/pull/tests/folder.rs:459 (F) panic!("{reach:?}")
crates/pull/tests/folder.rs:925 (F) panic!("{:?}", census.reach)
crates/pull/tests/folder.rs:188:67 False (F) assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Dhan));
crates/pull/tests/folder.rs:394:67 False (F) assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Groww));
crates/pull/tests/folder.rs:418:64 False (F) assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
crates/pull/tests/folder.rs:751:64 False (F) assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN));
crates/pull/tests/futures_events.rs:90 (F) panic!("Zerodha is an HTTP feed");
crates/pull/tests/gdfl_bad_row_audit.rs:230 (F) lines[0]
crates/pull/tests/gdfl_bad_row_audit.rs:235 (F) lines[0]
crates/pull/tests/gdfl_bad_row_audit.rs:293 (F) lines[0]
crates/pull/tests/gdfl_bad_row_audit.rs:336 (F) lines[0]
crates/pull/tests/gdfl_bad_row_audit.rs:341 (F) lines[0]
crates/pull/tests/gdfl_bad_row_audit.rs:346 (F) || lines[0].contains("pull.gdfl.format")
crates/pull/tests/gdfl_bad_row_audit.rs:345:9 False (F) lines[0].contains("\"target\":\"pull.gdfl.format\"")
crates/pull/tests/gdfl_census_stop.rs:117 (F) panic!("{got:?}");
crates/pull/tests/gdfl_census_stop.rs:177 (F) stop[0]
crates/pull/tests/gdfl_census_stop.rs:182 (F) stop[0]
crates/pull/tests/gdfl_census_stop.rs:214 (F) imported[0]
crates/pull/tests/gdfl_census_stop.rs:221 (F) refused[0]
crates/pull/tests/gdfl_census_stop.rs:116:9 False (F) let ImportRefusal::CensusRefused { day: at, done, .. } = &got else {
crates/pull/tests/gdfl_census_stop.rs:125:30 False (F) .is_some_and(|l| l.starts_with("incomplete stocks 2024-04-01 * ")
crates/pull/tests/integration.rs:326 (F) at.day()
crates/pull/tests/padded_refold.rs:290 (F) refused[0]
crates/pull/tests/padded_refold.rs:350 (F) refused[0]
crates/pull/tests/padded_refold.rs:381 (F) refused[0]
crates/pull/tests/padded_refold.rs:400 (F) journal[0]
crates/pull/tests/padded_refold.rs:405 (F) journal[0]
crates/pull/tests/padded_refold.rs:446 (F) refused[0]
crates/pull/tests/padded_refold.rs:459 (F) refused[0]
crates/pull/tests/padded_refold.rs:464 (F) refused[0]
crates/pull/tests/padded_refold.rs:474 (F) journal[0]
crates/pull/tests/padded_refold.rs:516 (F) refused[0]
crates/pull/tests/padded_refold.rs:522 (F) refused[0]
crates/pull/tests/padded_refold.rs:578 (F) refused[0]
crates/pull/tests/padded_refold.rs:598 (F) refused[0]
crates/pull/tests/padded_refold.rs:620 (F) refused[0]
crates/pull/tests/pipeline.rs:192 (F) cases[4]
crates/pull/tests/pipeline.rs:372 (F) Ok((members, rejected)) => panic!(
crates/pull/tests/pipeline.rs:375 (F) members.len(),
crates/pull/tests/pipeline.rs:376 (F) rejected.len(),
crates/pull/tests/pipeline.rs:377 (F) members.len() + rejected.len()
crates/pull/tests/pipeline.rs:437 (F) wide.why.len(),
crates/pull/tests/pipeline.rs:438 (F) wide.why.get(wide.why.len().saturating_sub(48)..)
crates/pull/tests/pipeline.rs:443 (F) wide.why.capacity()
crates/pull/tests/pipeline.rs:517 (F) cases[4]
crates/pull/tests/pipeline.rs:726 (F) cases[4]
crates/pull/tests/pipeline.rs:1568 (F) done.failures[0].why
crates/pull/tests/pipeline.rs:1594 (F) done.failures[0].why
crates/pull/tests/pipeline.rs:1701 (F) done.failures[0].why
crates/pull/tests/pipeline.rs:1028:9 False (F) text.contains(&120_000_000.to_string())
crates/pull/tests/pipeline.rs:1029:16 False (F) && text.contains(&60_000_000.to_string())
crates/pull/tests/pipeline.rs:1030:16 False (F) && text.contains('1'),
crates/pull/tests/pipeline.rs:1487:16 False (F) if path.ends_with("ZZZZ.csv")),
crates/pull/tests/refusal.rs:551 (F) feed.display()
crates/pull/tests/refusal.rs:585 (F) feed.display()
crates/pull/tests/refusal.rs:642 (F) other => panic!(
crates/pull/tests/refusal.rs:647 (F) other.master_columns().isin
crates/pull/tests/refusal.rs:655 (F) vendor.master_columns().isin
crates/pull/tests/refusal.rs:690 (F) panic!("Zerodha is an HTTP feed");
crates/pull/tests/request_gap_events.rs:145 (F) lines[0]
crates/pull/tests/request_gap_events.rs:138:9 False (F) lines[0].contains("\"count\":2")
crates/pull/tests/request_gap_events.rs:139:16 False (F) && lines[0].contains("\"first_day\":\"2025-07-01\"")
crates/pull/tests/request_gap_events.rs:140:16 False (F) && lines[0].contains("\"last_day\":\"2025-07-01\"")
crates/pull/tests/request_gap_events.rs:141:16 False (F) && lines[0].contains("\"month\":\"2025-07\"")
crates/pull/tests/request_gap_events.rs:142:16 False (F) && lines[0].contains("\"code\":\"vendor_empty\"")
crates/pull/tests/request_gap_events.rs:143:16 False (F) && lines[0].contains("\"feed\":\"truedata\""),
crates/pull/tests/unit.rs:143 (F) other => panic!("expected Unreadable, got {other:?}"),
crates/pull/tests/unit.rs:766 (F) other => panic!("expected an unreadable file, got {other:?}"),
crates/pull/tests/unit.rs:1298 (F) other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
crates/pull/tests/unit.rs:1319 (F) other => panic!("expected a checksum refusal, got {other:?}"),
crates/pull/tests/unit.rs:1624 (F) manifest.entries()
crates/pull/tests/unit.rs:1686 (F) other => panic!("the recovery must name what it stepped over, got {other:?}"),
crates/pull/tests/unit.rs:1781 (F) other => panic!("expected the slot checksum to be reported, got {other:?}"),
crates/pull/tests/unit.rs:1864 (F) manifest.reserved()
crates/pull/tests/unit.rs:1869 (F) manifest.reserved()
crates/pull/tests/unit.rs:2006 (F) manifest.reserved()
crates/pull/tests/unit.rs:2064 (F) other => panic!("expected entry 1 to be named, got {other:?}"),
crates/pull/tests/unit.rs:2204 (F) other => panic!("expected a checksum refusal, got {other:?}"),
crates/pull/tests/unit.rs:2930 (F) Verdict::Admit => panic!("a drained bucket admits nothing at once"),
crates/pull/tests/unit.rs:3047 (F) governor.permitted(WindowSpan::Day)
crates/pull/tests/unit.rs:3379 (F) panic!("the eighth request must be refused");
crates/pull/tests/unit.rs:4120 (F) other => panic!("month {other}"),
crates/pull/tests/unit.rs:4890 (F) Some(other) => panic!("a bar inside its own window was dropped as {other}"),
crates/pull/tests/unit.rs:5358 (F) errors[0]
crates/pull/tests/unit.rs:6793 (F) other => panic!("prefix of {cut} bytes was not refused by name: {other:?}"),
crates/pull/tests/unit.rs:6813 (F) other => panic!("a corrupt header slot must be refused by name, got {other:?}"),
crates/pull/tests/unit.rs:7880 (F) other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
crates/pull/tests/unit.rs:7951 (F) layout.version()
crates/pull/tests/unit.rs:1622:40 False (F) manifest.entries() == 0 || manifest.entries() == 1,
crates/pull/tests/unit.rs:3378:9 False (F) let Verdict::Deny { span, wait_micros } = governor.admit(0) else {
crates/pull/tests/zerodha_gaps.rs:225 (F) gaps[0].why
crates/pull/tests/zerodha_gaps.rs:235 (F) lines[0]
crates/pull/tests/zerodha_gaps.rs:543 (F) other => panic!("2021-02-24 is a session of record: {other:?}"),
crates/pull/tests/zerodha_gaps.rs:651 (F) panic!("a regular cash session owes one window");
crates/pull/tests/zerodha_gaps.rs:715 (F) series.lines[0]
crates/pull/tests/zerodha_gaps.rs:824 (F) other => panic!("{other:?}"),
crates/pull/tests/zerodha_gaps.rs:630:22 False (F) .all(|m| m.short.is_empty() && m.length_unjudged.is_empty())
crates/pull/tests/zerodha_gaps.rs:650:9 False (F) let Owes::Windows((windows, 1)) = repair_plan::owed("CASH", from.days_from_epoch()) else {
crates/pull/tests/zerodha_gaps.rs:712:9 False (F) series.lines[0].starts_with("gap NSE-CASH-INVENTEDSHR 2026-08-06 v=1 rung=1min owed=360 held=359 missing=1 first_missing=11:00 ")
crates/pull/tests/zerodha_gaps.rs:713:16 False (F) && series.lines[0].ends_with(" fill=interior source=store absence=may_be_no_trade"),
crates/pull/tests/zerodha_gaps.rs:1348:22 False (F) .all(|l| l.contains(" why=stored_short ") && l.ends_with(" source=store")),
crates/pull/tests/zerodha_identity.rs:304 (F) rung.as_str()
crates/pull/tests/zerodha_identity.rs:560 (F) Err(why) => panic!("{symbol} {}: {why}", tf.as_str()),
crates/pull/tests/zerodha_identity.rs:717 (F) rung.as_str()
crates/pull/tests/zerodha_identity.rs:836 (F) named[0]
crates/pull/tests/zerodha_identity.rs:1755 (F) lines[0]
crates/pull/tests/zerodha_identity.rs:1764 (F) lines[0]
crates/pull/tests/zerodha_identity.rs:1769 (F) lines[0]
crates/pull/tests/zerodha_identity.rs:1810 (F) identity_lines(sink, "PATANJALI")
crates/pull/tests/zerodha_identity.rs:1957 (F) named[0]
crates/pull/tests/zerodha_identity.rs:1966 (F) said[0]
crates/pull/tests/zerodha_identity.rs:1971 (F) said[0]
crates/pull/tests/zerodha_identity.rs:1976 (F) said[0]
crates/pull/tests/zerodha_identity.rs:302:30 False (F) .all(|b| b.high <= LEVEL + 2_000 && b.low >= LEVEL - 1_000),
crates/pull/tests/zerodha_identity.rs:1862:22 False (F) .any(|l| l.contains("the day rung would not open")
crates/pull/tests/zerodha_identity.rs:1863:20 False (F) && l.contains("\"why\":")
crates/pull/tests/zerodha_repair.rs:287 (F) lines[0]
crates/pull/tests/zerodha_repair.rs:355 (F) front[0]
crates/pull/tests/zerodha_repair.rs:405 (F) lines[11]
crates/pull/tests/zerodha_repair.rs:340:22 False (F) .any(|f| f.why.contains("2020-12:") && f.why.contains("this month only")),
`````

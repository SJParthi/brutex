# base-cov-pull-ps1-fu.txt.md

Written by BASE-COV-PULL-PS1 (recipe v4 step 4) from Route B run 9 on the code tip `3c2e0e39b08d0ef1a465d7630f19beda3e1c22c2` (5 Nov 2026; the 3 Nov list was of run 5 on `feeba59beb6a`): G1's (F)/(U) list, with every other uncovered row of G1 named and why it is left. One fenced block below holds the list.

`````text
BASE-COV-PULL-PS1 (F)/(U) list for G1 (recipe v4 step 4), from Route B run 9 on the code tip 3c2e0e39b08d0ef1a465d7630f19beda3e1c22c2 (kit d3-route-b-cloud.sh, pinned nightly), reading whole:pull, set = the kit f98a899398f6 set plus the 23 files step (2) added; every line number is of that tip.
G1 = SHARDS.md 64 files, repair 2 fno_membership_cov_tests.rs and manifest_cov_tests.rs, and the 23 new files: 89 files.
Method: (F) by pattern = a node classifier (read-only over the tree): an uncovered row of test code inside a panic!/unreachable! invocation, a message row of an assert whose first row ran, the failing side of an assert condition (decided by enumerating every truth assignment under short-circuit evaluation), or the else block of a let-else whose block panics; the sides it left open (in matches! guards and in closures inside an assert condition) were checked by hand. (F) by role = a test body that runs only when the property under test fails, each named with its role. (U) = a production row no input reaches, each with its proof from the code, re-anchored to this tip by its text and by the diff of every later commit. L-BCP-3 = rows inside the bodies of G1 #[ignore] real-data tests. GAP = every other row, each with why it is left. 5 Nov: the 3 Nov list (run 5 on feeba59beb6a) re-anchored to this tip through git diff -U0 feeba59beb6a 3c2e0e39b08d; the 13 rows run 9 no longer leaves are dropped (the race test's 9, run 5's 2 timing rows, and the two-builds judgement's 2, which moved into its helper and are listed again at their new lines); each of the 13 rows run 9 leaves that run 5 did not is the failing side of an assert or a let-else of a test, (F) by pattern, checked by hand.
Counts: 2312 uncovered rows; (U) 138; (F) 849 (828 by pattern, 21 by role); L-BCP-3 1306; GAP 19.

(U) items, each with its proof:
crates/pull/src/cash_auction.rs:713 (U) None => break, -- proof: the body runs only when input.peek() answered Some(c), and Peekable::next returns the item peek stored, with nothing touching input between the two calls, so next() is Some
crates/pull/src/expiry_alias.rs:919 (U) && matches!( -- proof: the row is the _ => false arm of the matches! (its tokens map to the call site; the run shows 918 = 13, 920 = 7, 919 = 0). A contract the first filter keeps has settle_by(.., held) = AsStatedUnmeasured, so alias_of(underlying, stated) is None (an aliased contract is Aliased or Err), and alias_of reads only ALIASES; the second settle_by(.., newer) then settles by newer(stated) alone: Closed gives ClosedDayRule with expiry = previous_open_in(..), a day strictly before stated; Unmeasured gives AsStatedUnmeasured and Open gives AsStated, both with expiry = stated; with trade None nothing else is added (line 854). So an Ok now with now.expiry == stated is AsStated or AsStatedUnmeasured, and the false arm runs for no contract and no calendar pair
crates/pull/src/fetch.rs:518 (U) break; -- proof: the else of 515 above
crates/pull/src/fetch.rs:521 (U) break; -- proof: the else of 520 above
crates/pull/src/fetch.rs:870 (U) fn note_refused(days: &mut Vec<RefusedDay>, day: u32) { -- proof: its one caller is the SessionLengthUnmeasured arm at 1032, which no input reaches (below)
crates/pull/src/fetch.rs:871 (U) match days.iter_mut().find(|held| held.day == day) { -- proof: as 870
crates/pull/src/fetch.rs:872 (U) Some(held) => held.rows = held.rows.saturating_add(1), -- proof: as 870
crates/pull/src/fetch.rs:873 (U) None => days.push(RefusedDay { day, rows: 1 }), -- proof: as 870
crates/pull/src/fetch.rs:875 (U) } -- proof: as 870
crates/pull/src/fetch.rs:928 (U) FetchError::BodyNotUnderstood { -- proof: cash_close_verdict runs only after request.window.verdict(epoch_utc, ..) answered Ok (land_rows 1019-1061), and verdict's first statement is IstMoment::from_epoch_secs(epoch_secs)? (session.rs:1146) on the same value; the conversion is a pure const fn of its argument, so here it cannot fail
crates/pull/src/fetch.rs:929 (U) detail: why.to_string(), -- proof: as 928
crates/pull/src/fetch.rs:930 (U) } -- proof: as 928
crates/pull/src/fetch.rs:931 (U) })?; -- proof: as 928
crates/pull/src/fetch.rs:1032 (U) Err(crate::session::SessionError::SessionLengthUnmeasured { days }) => { -- proof: Window::verdict (session.rs:1140) returns SessionLengthUnmeasured only when with_recorded_window(day, calendar::kind_of(day)) is OpenLengthUnmeasured (session.rs:1252); kind_of answers OpenLengthUnmeasured only for the five LENGTH_UNMEASURED days (calendar.rs:579, 689-693; every other day is Unmeasured, Closed or Open, calendar.rs:662-701), and each of the five has a SPECIAL_SESSIONS row (the seven Muhurats 2018-11-07..2024-11-01, calendar.rs:1498), which with_recorded_window (session.rs:1349-1357) turns into DayKind::Open
crates/pull/src/fetch.rs:1033 (U) note_refused(&mut refused_days, days); -- proof: as 1032
crates/pull/src/fetch.rs:1034 (U) continue; -- proof: as 1032
crates/pull/src/fetch.rs:515:17 False (U) let (Some(&open), Some(&high), Some(&low), Some(&close)) = -- proof: decode refuses (LengthDisagreement, 484-497) unless high, low, close, volume and timestamp all have open's length n, and the loop index i runs 0..n, so every get(i) is Some
crates/pull/src/fetch.rs:520:17 False (U) let (Some(&volume), Some(&timestamp)) = (a.volume.get(i), a.timestamp.get(i)) else { -- proof: as 515: volume and timestamp have length n and i < n
crates/pull/src/fno_membership.rs:1315 (U) )?; -- proof: the ? of checked_add in Counts::body_len: every count is Cursor::count = usize::try_from(u32) <= 4_294_967_295, so each pair-count product * 8 <= 68_719_476_720 and their sum is far below usize::MAX on a 64-bit target (aarch64-apple-darwin included); checked_add is always Some
crates/pull/src/fno_membership.rs:2638 (U) return Vec::new(); -- proof: the else of 2634
crates/pull/src/fno_membership.rs:2642 (U) continue; -- proof: the else of 2641
crates/pull/src/fno_membership.rs:2680 (U) continue; -- proof: the else of 2679
crates/pull/src/fno_membership.rs:2695 (U) continue; -- proof: the else of 2694
crates/pull/src/fno_membership.rs:2920 (U) } -- proof: the close of the false side of 2918
crates/pull/src/fno_membership.rs:2166:20 False (U) if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) { -- proof: <[u8]>::get(a..) is None only when a > len, and a = len.saturating_sub(32) <= len for every slice
crates/pull/src/fno_membership.rs:2193:24 False (U) if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) { -- proof: as 2166
crates/pull/src/fno_membership.rs:2326:20 False (U) if let Some(slot) = usize::try_from(spelling.series_id) -- proof: assemble runs chain on the same months and sizes newest to the series count chain returns; chain admits a spelling only with series_id == series (then counts it) or handed over with series_id < series, and series never decreases, so every registered series_id < newest.len(), and u32 -> usize cannot fail on a 64-bit target; assemble is private and builds the registry itself
crates/pull/src/fno_membership.rs:2384:28 False (U) if let Some(word) = out.bits.get_mut(day_bits + series_at / 64) { -- proof: out.bits is resized to day_bits + words just above it with words = newest.len().div_ceil(64).max(1), and series_at < newest.len() (2326), so the index is in range
crates/pull/src/fno_membership.rs:2634:13 False (U) let (Ok(first), Ok(last)) = ( -- proof: u32::try_from of SOURCED_FIRST_DAY and SOURCED_LAST_DAY, constants pinned at compile time (const asserts 17_532 and 20_818), both in u32
crates/pull/src/fno_membership.rs:2641:17 False (U) let Ok(day) = Day::from_days(epoch) else { -- proof: epoch runs over 17_532..=20_818 only, and Day::from_days refuses only a count past its day range (far above 20_818)
crates/pull/src/fno_membership.rs:2679:17 False (U) let Ok(day) = Day::from_days(self.first_epoch.saturating_add(offset)) else { -- proof: a Membership is only Default (no slot, the loop does not run) or built by assemble, whose first_epoch and slots come from decoded months inside the sourced span 17_532..=20_818, so first_epoch + offset is a day count of that span
crates/pull/src/fno_membership.rs:2694:25 False (U) let Some(share) = self.series(id).map(|spelling| spelling.name) else { -- proof: id is a member's series_id, which assemble copies from a registry spelling, so id < newest.len() and series(id) is Some
crates/pull/src/fno_membership.rs:2761:16 False (U) if let Some((offset, slot)) = previous -- proof: resume_state returns before this when no month is held, and with a month held its slots hold every row, so previous (the last sealed session before the resume day) is Some
crates/pull/src/fno_membership.rs:2914:28 False (U) && let Some(table) = open.take() -- proof: the && evaluates open.take() only after open.as_ref().is_some_and(..) was true on the same open in the same expression, so take answers Some
crates/pull/src/fno_membership.rs:2918:24 False (U) if work.touched { -- proof: month.touched is set by every month.note (one per fold) and cleared only by the mem::take just above, after a note; a month reaching here was noted
crates/pull/src/folder.rs:521 (U) Err(why) => { -- proof: reach_of refuses only a row whose IstMoment::from_epoch_secs fails (folder.rs:436); walk() builds every row through csv::decode, whose stamp is days*86400 + secs - IST_OFFSET_SECS (csv.rs:928) with days a Day's count (>= 0) and secs from ist_seconds, which refuses h > 23, m > 59, s > 59 (csv.rs:439); so from_epoch_secs sees local = days*86400 + secs >= 0 whose day is that Day, and never fails
crates/pull/src/folder.rs:522 (U) note_refused(dir, feed, &why); -- proof: as 521
crates/pull/src/folder.rs:523 (U) Err(why) -- proof: as 521
crates/pull/src/folder.rs:553 (U) Err(why) => { -- proof: as 521, for census_of over walk_reporting's rows (the same csv::decode)
crates/pull/src/folder.rs:554 (U) note_refused(dir, feed, &why); -- proof: as 553
crates/pull/src/folder.rs:555 (U) Err(why) -- proof: as 553
crates/pull/src/gdfl_cm_auction.rs:650 (U) } -- proof: the implicit else of 642
crates/pull/src/gdfl_cm_auction.rs:642:12 False (U) if let Some(parent) = path.parent() { -- proof: path is path(store_root, key, day): the store's own path of the key's month file (StorePath::to_path_buf) with with_extension(EXTENSION), so it ends in a file name, and Path::parent is None only for an empty path or a root; the one caller is append_with, after path(..)? answered
crates/pull/src/gdfl_import.rs:2654 (U) Ok(None) => false, -- proof: cm_present asks listing.filing(stem) only for a stem listing.member(stem) answered (the let-else above), and member_of and filing agree: both strip the same suffix; for Stocks both ask files_share of the same share (series_of under the same series flag, false in CmScope::swept and true in CmScope::stored_with, the only two constructors), after which filing answers Ok(swept) unstored, Swept exactly when files_share holds (resolve and files_share build the same InstrumentKey::cash, exact bytes, sweepable), or, stored, InstrumentKey::cash(Nse, LINEAGE.filed_under(name, day)?).ok(), whose name is the stem's own (files_share built its key) or a compiled symbol-change row's new symbol (all 27 are A-Z, 0-9, & and -, at most 24 bytes); for Indices member_of answers only for a Swept key (filing answers it) or, stored, an index row found by name, whose store key filing builds with InstrumentKey::index, and every such key is one it accepts: the 9 compiled INDEX_ROWS keys are A-Z and 0-9, and the store table's loader refuses any key_gdfl InstrumentKey::index refuses (reference_data.rs:1306-1311). So filing never answers Ok(None) for a member
crates/pull/src/gdfl_import_day.rs:1207 (U) Refused::at( -- proof: the ok_or_else closure of drive_with's read runs only when slot.take() is None; handed is a fresh Vec of Mutex<Option<Handed>>, every slot Some, handed to one file_all call, whose todo holds each job at most once and whose file_jobs calls read once per index drawn from one shared AtomicUsize::fetch_add (on_workers), the single-thread path running only when no worker started; back_fill never calls read; so every take() meets Some, and the closure is a literal no direct call reaches
crates/pull/src/gdfl_import_day.rs:1208 (U) Cause::open(Stage::File, "file_taken_twice"), -- proof: the ok_or_else closure of drive_with's read runs only when slot.take() is None; handed is a fresh Vec of Mutex<Option<Handed>>, every slot Some, handed to one file_all call, whose todo holds each job at most once and whose file_jobs calls read once per index drawn from one shared AtomicUsize::fetch_add (on_workers), the single-thread path running only when no worker started; back_fill never calls read; so every take() meets Some, and the closure is a literal no direct call reaches
crates/pull/src/gdfl_import_day.rs:1209 (U) failure("file", "a handed file was taken twice".to_owned()), -- proof: the ok_or_else closure of drive_with's read runs only when slot.take() is None; handed is a fresh Vec of Mutex<Option<Handed>>, every slot Some, handed to one file_all call, whose todo holds each job at most once and whose file_jobs calls read once per index drawn from one shared AtomicUsize::fetch_add (on_workers), the single-thread path running only when no worker started; back_fill never calls read; so every take() meets Some, and the closure is a literal no direct call reaches
crates/pull/src/gdfl_import_day.rs:1211 (U) }) -- proof: the ok_or_else closure of drive_with's read runs only when slot.take() is None; handed is a fresh Vec of Mutex<Option<Handed>>, every slot Some, handed to one file_all call, whose todo holds each job at most once and whose file_jobs calls read once per index drawn from one shared AtomicUsize::fetch_add (on_workers), the single-thread path running only when no worker started; back_fill never calls read; so every take() meets Some, and the closure is a literal no direct call reaches
crates/pull/src/gdfl_import_day.rs:1747 (U) from: first, -- proof: the RangeBackwards closure in reach_back runs only when Window::new(first, last) errs, which it does only when last is before first (session.rs:1056-1065); first and last are the ends of before_range(run.from), an ascending integer range mapped through Day::from_days (strictly increasing) with refusals dropped, so first <= last
crates/pull/src/gdfl_import_day.rs:1748 (U) to: last, -- proof: the RangeBackwards closure in reach_back runs only when Window::new(first, last) errs, which it does only when last is before first (session.rs:1056-1065); first and last are the ends of before_range(run.from), an ascending integer range mapped through Day::from_days (strictly increasing) with refusals dropped, so first <= last
crates/pull/src/gdfl_import_day.rs:1749 (U) })?, -- proof: the RangeBackwards closure in reach_back runs only when Window::new(first, last) errs, which it does only when last is before first (session.rs:1056-1065); first and last are the ends of before_range(run.from), an ascending integer range mapped through Day::from_days (strictly increasing) with refusals dropped, so first <= last
crates/pull/src/gdfl_run_locks.rs:424 (U) return Err(LockFault::Refused { -- proof: the else of 423
crates/pull/src/gdfl_run_locks.rs:425 (U) lock: name.to_owned(), -- proof: as 424
crates/pull/src/gdfl_run_locks.rs:426 (U) why: "the lock file was lost between two tries".to_owned(), -- proof: as 424
crates/pull/src/gdfl_run_locks.rs:427 (U) }); -- proof: as 424
crates/pull/src/gdfl_run_locks.rs:508 (U) return Err(LockFault::Refused { -- proof: the else of 507
crates/pull/src/gdfl_run_locks.rs:509 (U) lock: name.to_owned(), -- proof: as 508
crates/pull/src/gdfl_run_locks.rs:510 (U) why: "the lock file was lost between two tries".to_owned(), -- proof: as 508
crates/pull/src/gdfl_run_locks.rs:511 (U) }); -- proof: as 508
crates/pull/src/gdfl_run_locks.rs:1168 (U) } -- proof: the implicit else of 1164
crates/pull/src/gdfl_run_locks.rs:423:13 False (U) let Some(one) = file.take() else { -- proof: file is Some before take_cloning's loop (420); every arm of the body returns except WouldBlock under the retry count, which ends with file = Some(clone(&reader)...?) whose ? returns on failure, so file is Some at every loop head
crates/pull/src/gdfl_run_locks.rs:507:13 False (U) let Some(one) = file.take() else { -- proof: as 423, for hold_within: file is Some (505); the body returns from Ok, Error, the ? of try_clone and WouldBlock past the wait, and the one path back to the head ends file = Some(kept) (542)
crates/pull/src/gdfl_run_locks.rs:1164:12 False (U) if let Some(dir) = path.parent() { -- proof: Path::parent is None only for "" or a root ("/"); for "" staging is "" and store::staging::create fails (ENOENT) and write_sidecar returns at 1158; for "/" create's open fails AlreadyExists and its look finds a directory (NotRegular), returning at 1158 too; every other path has a parent
crates/pull/src/identity_screen.rs:780 (U) (FirstDay::On(_), None) => false, -- proof: earliest is None exactly when no row has a stamp_day, and the months read are those same rows (months.insert sits inside that Some), so then months is empty, days is empty, and DayBook::new(&[], FirstDay::NoDayRung) keeps NoDayRung (series_identity.rs:359-364), taking the first arm; (On(_), None) needs days non-empty with no row stamped, which the code above excludes, and a direct call cannot separate them: day_book_read derives both from its one rows argument
crates/pull/src/identity_screen.rs:747:16 False (U) if let Ok(month) = day.year_month() { -- proof: day is a session::Day, whose every constructor keeps the year in 1970..=9999 and the month in 1..=12, and Day::year_month is YearMonth::new(year, month), which refuses only outside those bounds
crates/pull/src/manifest.rs:2175:20 False (U) if let Some(slot) = closes.get_mut(C_CONTRACT..C_CONTRACT + text.len()) { -- proof: text is Contract::as_str's bytes, a sub-slice of its [u8; CONTRACT_CAPACITY = 24] array or "" (core/src/instrument.rs:442-445, 562-570), so C_CONTRACT + text.len() <= 16 + 24 = 40 <= IMAGE_LEN = 64 (the const assert C_CONTRACT_N < OFF_CRC states it) and closes is a [u8; IMAGE_LEN]
crates/pull/src/master_snapshots.rs:968 (U) Err(why) => return refused(why), -- proof: compress fails only if GzEncoder<Vec<u8>>::write_all or finish does; Vec<u8>'s Write never errs and never writes 0 of a non-empty buffer, and in the locked flate2 1.1.9 over miniz_oxide 0.8.9 the codec answers an error only on BadParam (a call after a non-Okay status or a non-Finish flush after Finish, which flate2 never makes) or PutBufFailed (its own output buffer, drained by dump); the kit's provisional list held the same item
crates/pull/src/master_snapshots.rs:2143 (U) continue; -- proof: the else of 2142
crates/pull/src/master_snapshots.rs:2142:17 False (U) let Some(before) = old_rows.get(at) else { -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2145:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2187:20 False (U) && let Some(before) = old_rows.get(at) -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2194:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2228:20 False (U) && let Some(before) = old_rows.get(at) -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2231:20 False (U) if let Some(slot) = matched.get_mut(at) { -- proof: at comes from a map built inside diff by enumerate over this same old_rows (old_keys, old_tokens, gone_by_isin), so at < old_rows.len(), and matched is vec![false; old_rows.len()]; no caller hands diff such a map
crates/pull/src/master_snapshots.rs:2486:20 False (U) if let Some(count) = counts.get_mut(one.change.index()) { -- proof: counts is [u64; 8] and Change::index maps each of Change's 8 variants (Change::ALL) onto 0..=7
crates/pull/src/master_snapshots_first_listed.rs:713 (U) } -- proof: the close of the None side of 711
crates/pull/src/master_snapshots_first_listed.rs:768 (U) Rebuilt::StillDamaged(why) => Err(format!("futures_index_damaged: {why}")), -- proof: absorb_held returns at 757-762 for any route carried refuses, so with_rebuild runs only for a carriable route; StillDamaged needs absorb_in on next to answer Fault::Damaged, and next is new and empty (start_next's create_dir fails on any existing entry) under the writers' lock, so read_log reads nothing torn and every line absorb_in itself writes (future_line, absorbed_line with a carried exchange, type and name, dashed days that parse_dashed reads back, u64 tokens, encoded names with no ',' or '=') checks and parses, and committed starts empty, so held.count < committed cannot hold; absorb_held takes only (root, route), so no direct call changes this. The full argument is in the helper's rows.md (scratchpad), case (b) included for a case-insensitive volume
crates/pull/src/master_snapshots_first_listed.rs:711:16 False (U) if let Some(path) = path_of(dir, route.exchange, name) { -- proof: a key enters new only at 705, inside the walk of rows whose let-else at 681-688 has just bound Some(path) = path_of(dir, route.exchange, name) for that same name; dir and route are never reassigned and path_of is a pure function of its three arguments (encoded maps bytes to text, Path::join and format! are pure), so the call answers Some for every key of new
crates/pull/src/nse.rs:400 (U) return out; -- proof: the else of 399
crates/pull/src/nse.rs:443 (U) break; -- proof: the else of 442
crates/pull/src/nse.rs:585 (U) break; -- proof: the else of 584
crates/pull/src/nse.rs:396:16 False (U) if let Some(head) = rest.get(..at) { -- proof: at = rest.find("<!--"), a char boundary of rest, so get(..at) is Some
crates/pull/src/nse.rs:399:13 False (U) let Some(after) = rest.get(at.saturating_add(4)..) else { -- proof: "<!--" is four ASCII bytes found at at, so at + 4 <= rest.len() is a char boundary
crates/pull/src/nse.rs:442:13 False (U) let Some(after) = rest.get(at.saturating_add(5)..) else { -- proof: "href=" is five ASCII bytes found at at
crates/pull/src/nse.rs:457:13 False (U) let Some(value) = value else { break }; -- proof: value is after.get(1..) where after's first byte is an ASCII quote (446-447), so it is Some
crates/pull/src/nse.rs:584:13 False (U) let Some(after) = rest.get(at.saturating_add(5)..) else { -- proof: as 442
crates/pull/src/nse.rs:595:13 False (U) let Some(value) = value else { break }; -- proof: as 457 (588-589)
crates/pull/src/nse.rs:824:24 False (U) if let Some(field) = line.get(start..at) { -- proof: start is 0 or one past an ASCII ',' and at indexes an ASCII ',' (split_row steps only over bytes, and a quote step of two crosses two ASCII quotes), so both are char boundaries with start <= at
crates/pull/src/nse.rs:833:12 False (U) if let Some(field) = line.get(start..) { -- proof: start is 0 or one past an ASCII ',' at most line.len(), a char boundary
crates/pull/src/pricing.rs:598 (U) return Err(PricingError::BasisMismatch { -- proof: as 597
crates/pull/src/pricing.rs:599 (U) rate: rate.basis(), -- proof: as 597
crates/pull/src/pricing.rs:600 (U) asked: basis, -- proof: as 597
crates/pull/src/pricing.rs:601 (U) }); -- proof: as 597
crates/pull/src/pricing.rs:597:8 True (U) if rate.basis() != basis { -- proof: YearBasis has exactly one variant, Calendar365 (tenor.rs:115-123), so two values of it are always equal
crates/pull/src/reference_data.rs:2519 (U) table, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2520 (U) path: dir.clone(), -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2521 (U) generation: None, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2522 (U) kind: RefusalKind::Row, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2523 (U) detail: format!("the compiled table cannot be rendered: {why}"), -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2524 (U) })?; -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2526 (U) table, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2527 (U) path: dir.clone(), -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2528 (U) generation: None, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2529 (U) kind: RefusalKind::Row, -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2530 (U) detail: format!("the compiled table cannot be rendered: {why}"), -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:2531 (U) })?; -- proof: the closure is the error arm of seed_draft/render inside slot's compiled closure (entered only through None => compiled(), 2546, and only for a compiled table); seed_draft reads no store, file, folder or variable, only this build's constants (core::alias, core::universe::FNO_UNDERLYINGS, expiry_alias::ALIASES, costs::lot::compiled_rows, calendar_seed over calendar's constants), so its answer is fixed at build time, and reference_data_tests::stray_names_are_not_generations_and_an_empty_store_serves_the_compiled_tables_labelled asserts every compiled slot answers origin compiled (the Ok side) and seeded() asserts each draft renders; render's refusals do not depend on the generation handed
crates/pull/src/reference_data.rs:3212 (U) return Err(format!("day {day} inside the record is unmeasured")); -- proof: calendar_seed's loop walks exactly SOURCED_FIRST_DAY..=SOURCED_LAST_DAY asking exchange_kind_of(day), which is sourced_kind_of(day) except that OpenLengthUnmeasured may become Open (calendar.rs:1521-1529); sourced_kind_of answers kind_of inside FIRST_DAY..=LAST_DAY (whose only Unmeasured returns are its own span check and a try_from of day - FIRST_DAY, neither possible there), sourced_kind (no Unmeasured return) inside the sourced span, and Unmeasured only outside both, which the loop never visits
crates/pull/src/rehearsal.rs:917 (U) Err(why) => Err(Refusal::ListUnreadable { -- proof (this build): the only producer of that Err is Gate::embedded(), GATE.get_or_init(|| Self::from_list(LIST)) over LIST = include_str!(config/store-steps.toml), a constant of the build that from_list reads alone (no file, folder, variable or clock); in this build it reads (rehearsal_tests.rs:325 the_tracked_list_reads_and_names_no_kind_yet expects it), and may_start_by_itself's arguments do not reach embedded()
crates/pull/src/rehearsal.rs:918 (U) kind: kind.to_owned(), -- proof (this build): the only producer of that Err is Gate::embedded(), GATE.get_or_init(|| Self::from_list(LIST)) over LIST = include_str!(config/store-steps.toml), a constant of the build that from_list reads alone (no file, folder, variable or clock); in this build it reads (rehearsal_tests.rs:325 the_tracked_list_reads_and_names_no_kind_yet expects it), and may_start_by_itself's arguments do not reach embedded()
crates/pull/src/rehearsal.rs:919 (U) why: why.to_owned(), -- proof (this build): the only producer of that Err is Gate::embedded(), GATE.get_or_init(|| Self::from_list(LIST)) over LIST = include_str!(config/store-steps.toml), a constant of the build that from_list reads alone (no file, folder, variable or clock); in this build it reads (rehearsal_tests.rs:325 the_tracked_list_reads_and_names_no_kind_yet expects it), and may_start_by_itself's arguments do not reach embedded()
crates/pull/src/rehearsal.rs:920 (U) }), -- proof (this build): the only producer of that Err is Gate::embedded(), GATE.get_or_init(|| Self::from_list(LIST)) over LIST = include_str!(config/store-steps.toml), a constant of the build that from_list reads alone (no file, folder, variable or clock); in this build it reads (rehearsal_tests.rs:325 the_tracked_list_reads_and_names_no_kind_yet expects it), and may_start_by_itself's arguments do not reach embedded()
crates/pull/src/rehearsal.rs:922:12 False (U) if let Err(refusal) = &answer { -- proof (this build): answer is Ok only from gate.admit(kind, facts) on the embedded gate, whose one Ok follows self.kinds.get(kind); config/store-steps.toml has no kind = line, so the embedded gate holds no kind (rehearsal_tests.rs:328 asserts gate.kinds() == 0) and admit refuses every kind
crates/pull/src/request_minutes.rs:378 (U) other => { -- proof: as 377
crates/pull/src/request_minutes.rs:379 (U) return DayPlan::Unverified("venue hours", format!("venue hours {other:?}")); -- proof: as 377
crates/pull/src/request_minutes.rs:456 (U) audit.findings.push(Finding { -- proof: the else of 455
crates/pull/src/request_minutes.rs:457 (U) why: Why::Unverified, -- proof: as 456
crates/pull/src/request_minutes.rs:458 (U) day: None, -- proof: as 456
crates/pull/src/request_minutes.rs:459 (U) detail: "invalid day", -- proof: as 456
crates/pull/src/request_minutes.rs:460 (U) minutes: 0, -- proof: as 456
crates/pull/src/request_minutes.rs:461 (U) text: format!("request minute coverage UNVERIFIED: invalid day {number}"), -- proof: as 456
crates/pull/src/request_minutes.rs:462 (U) }); -- proof: as 456
crates/pull/src/request_minutes.rs:463 (U) continue; -- proof: as 456
crates/pull/src/request_minutes.rs:377:22 False (U) Ok(hours) if hours.kind() == SessionKind::Continuous => hours, -- proof: venue.hours_on (vendor.rs:891, 1230-1258) answers Err only for a row of Hours::Unverified and otherwise the row's kind; every production row of the three venue tables is built by SessionRow::verified with SessionKind::Continuous (SessionRow::unverified is dead_code outside tests by its own expect, vendor.rs:1085-1092, and no table row names SessionKind::ClosingAuction)
crates/pull/src/request_minutes.rs:455:13 False (U) let Ok(day) = Day::from_days(number) else { -- proof: number runs from window.from().days_from_epoch() to window.to().days_from_epoch(), the counts of two valid Days, and every count between two valid Day counts is a valid Day count
crates/pull/src/scrub.rs:219 (U) Err(why) => { -- proof: StorePath::new refuses only through check_segment (store/src/path.rs:897-907, 1087-1114): empty, over MAX_SEGMENT_LEN (24), "." or "..", a byte outside [A-Za-z0-9-_&], or lower case; an Entry's exchange and segment are Exchange/Segment::as_str ("NSE", "BSE", "INDEX", "CASH", "FNO"), its symbol a Symbol (1..=24 bytes of A-Z 0-9 - _ & after Symbol::new's upper-case fold, core/src/symbol.rs:62-83), and its contract a Contract (digits, '-', FUT/CE/PE, at most CONTRACT_CAPACITY = 24 bytes, core/src/instrument.rs:416, 475-492)
crates/pull/src/scrub.rs:220 (U) return Finding::Unreadable { -- proof: as 219
crates/pull/src/scrub.rs:221 (U) path: format!("{:?}", entry.key.symbol), -- proof: as 219
crates/pull/src/scrub.rs:222 (U) why: format!("the entry does not name a legal store path: {why}"), -- proof: as 219
crates/pull/src/scrub.rs:223 (U) }; -- proof: as 219
crates/pull/src/series_identity.rs:438 (U) return Verdict::Unchecked(Unchecked::NoDayBar); -- proof: the else of 437
crates/pull/src/series_identity.rs:437:13 False (U) let Some(range) = self.days.get(at) else { -- proof: binary_search_by_key answers Ok(i) only with the index of a matching element, so i < self.days.len(), and judge takes &self with nothing between the two lines touching days
crates/pull/src/ssm.rs:925 (U) SsmError::unreachable(format!("the HTTPS client could not be built: {why}")) -- proof: pooled_client caches one build of Client::builder().timeout(10 s).redirect(Policy::none()).build(); with the locked reqwest 0.12.28 and this workspace's features (rustls-tls-webpki-roots-no-provider, json, gzip) build() errs only on a user agent (not set), an added root (none), native roots (off), an empty version list (all versions) or a provider that fails the version check, and ensure_tls_provider installs rustls_graviola's default provider first, which passes it
crates/pull/src/ssm.rs:926 (U) })?; -- proof: the close of the map_err closure of 925
crates/pull/src/vendor.rs:505 (U) Some(Timeframe::MINUTE_30) -- proof: the arm of the constant-false guard at 504
crates/pull/src/vendor.rs:508 (U) Some(Timeframe::MINUTE_60) -- proof: the arm of the constant-false guard at 507
crates/pull/src/vendor.rs:504:31 True (U) Self::Minute30 if Timeframe::MINUTE_30.aligns_with_the_open() => { -- proof: aligns_with_the_open (store/src/path.rs:536-539) is secs % 60 == 0 && OPEN_MINUTES_PAST_IST_MIDNIGHT % (secs / 60) == 0, a const of its constant receiver; 09:15 is 555 minutes and 555 % 30 = 15, so the guard is false on every build
crates/pull/src/vendor.rs:507:28 True (U) Self::Hour1 if Timeframe::MINUTE_60.aligns_with_the_open() => { -- proof: as 504: 555 % 60 = 15
crates/pull/src/vendor.rs:1175:20 True (U) if interval_secs == 0 || !session_secs.is_multiple_of(interval_secs) { -- proof: the side is interval_secs == 0, and Granularity::grid (vendor.rs:337-349) answers Intraday only with 1, 5, 60, 180, 300, 900, 1800 or 3600, never 0

(F) items by role (a body that runs only when the property under test fails; listed apart so the Mac can judge them):
crates/pull/src/cash_session_cache.rs:1277 (F) |url| -> std::future::Ready<Result<Vec<u8>, String>> { -- role: the header of a fetch closure whose whole body is unreachable!: the test proves the fetch is never made
crates/pull/src/cash_session_cache.rs:1526 (F) |url| -> std::future::Ready<Result<Vec<u8>, String>> { -- role: the header of a fetch closure whose whole body is unreachable!: the test proves the fetch is never made
crates/pull/src/cash_session_cache.rs:1607 (F) |url| -> std::future::Ready<Result<Vec<u8>, String>> { -- role: the header of a fetch closure whose whole body is unreachable!: the test proves the fetch is never made
crates/pull/src/fno_membership_ps1_tests.rs:166 (F) fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> { -- role: the header of a trait method whose whole body is unreachable!: membership reads no tick file
crates/pull/src/fno_membership_tests.rs:78 (F) fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> { -- role: the header of a trait method whose whole body is unreachable!: membership reads no tick file
crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:25 False (F) Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {} -- role: a match guard whose False side falls to the next arm, `Err(refusal) => panic!(..)`
crates/pull/src/gdfl_cm_auction_v2_tests.rs:418:39 False (F) Err(refusal) if *seen == 0 && refusal.why.contains("holds no header") => {} -- role: a match guard whose False side falls to the next arm, `Err(refusal) => panic!(..)`
crates/pull/src/gdfl_import_tests.rs:5976 (F) Err(std::io::Error::other(format!( -- role: the body of a flush hook that refuses the run: the back-fill day's batch is handed nothing to flush, and the run's unwrap fails the test if it ever is
crates/pull/src/gdfl_import_tests.rs:5977 (F) "{word}: the back-fill day's batch flushed {}", -- role: the body of a flush hook that refuses the run: the back-fill day's batch is handed nothing to flush, and the run's unwrap fails the test if it ever is
crates/pull/src/gdfl_import_tests.rs:5978 (F) path.display() -- role: the body of a flush hook that refuses the run: the back-fill day's batch is handed nothing to flush, and the run's unwrap fails the test if it ever is
crates/pull/src/gdfl_import_tests.rs:5979 (F) ))) -- role: the body of a flush hook that refuses the run: the back-fill day's batch is handed nothing to flush, and the run's unwrap fails the test if it ever is
crates/pull/src/gdfl_import_tests.rs:5980 (F) }), -- role: the body of a flush hook that refuses the run: the back-fill day's batch is handed nothing to flush, and the run's unwrap fails the test if it ever is
crates/pull/src/gdfl_nfo_tests.rs:314 (F) fn day(&self, _: Day) -> Result<Option<NfoDay<u8>>, CmRefusal> { -- role: the header of NoFetch::day, whose whole body is unreachable!: read_file is handed the listing and never lists the day
crates/pull/src/gdfl_nfo_tests.rs:2589 (F) _ => None, -- role: a false answer of D-4816's relation (definition_5_differs_only_where_4_refused), which every caller asserts true
crates/pull/src/gdfl_nfo_tests.rs:2607 (F) (Err(_), Ok(_)) => false, -- role: a false answer of D-4816's relation (definition_5_differs_only_where_4_refused), which every caller asserts true
crates/pull/src/gdfl_nfo_tests.rs:2593:13 False (F) new.rows == old.rows -- role: a false answer of D-4816's relation (definition_5_differs_only_where_4_refused), which every caller asserts true
crates/pull/src/gdfl_nfo_tests.rs:2594:20 False (F) && new.quoted_rows == old.quoted_rows -- role: a false answer of D-4816's relation (definition_5_differs_only_where_4_refused), which every caller asserts true
crates/pull/src/gdfl_nfo_tests.rs:2595:20 False (F) && new.refused.total() == 0 -- role: a false answer of D-4816's relation (definition_5_differs_only_where_4_refused), which every caller asserts true
crates/pull/src/ssm.rs:1949:16 True (F) if answer.is_err() { -- role: inside the else block of a let-else whose block panics (the kit's own (F) rule; the classifier marks only the pattern's side)
crates/pull/src/ssm.rs:1949:16 False (F) if answer.is_err() { -- role: inside the else block of a let-else whose block panics (the kit's own (F) rule; the classifier marks only the pattern's side)
crates/pull/tests/support/mod.rs:62:15 False (F) let ran = output.status.success() && stdout.contains("1 passed"); -- role: the child failed: `ran` is false and the assert two lines on fails

(F) items by pattern:
crates/pull/src/calendar_tradable_tests.rs:252 (F) matches!(
crates/pull/src/calendar_tradable_tests.rs:247:13 False (F) notice.source.contains(" https://") && notice.source.contains(" of "),
crates/pull/src/calendar_tradable_tests.rs:247:52 False (F) notice.source.contains(" https://") && notice.source.contains(" of "),
crates/pull/src/calendar_tradable_tests.rs:341:13 False (F) assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
crates/pull/src/calendar_tradable_tests.rs:341:51 False (F) assert!(!NotTradable::Holiday.is_session() && !NotTradable::Weekend.is_session());
crates/pull/src/calendar_tradable_tests.rs:1028:20 False (F) if (record_first, record_last) == (SOURCED_FIRST_DAY, SOURCED_LAST_DAY)), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/calendar_tradable_tests.rs:1190:13 False (F) assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1190:37 False (F) assert!(samples.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1204:13 False (F) assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1204:37 False (F) assert!(buckets.len() >= 200 && p99 < 50_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1219:13 False (F) assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1219:35 False (F) assert!(years.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1234:13 False (F) assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
crates/pull/src/calendar_tradable_tests.rs:1234:37 False (F) assert!(records.len() >= 200 && p99 < 100_000, "p99 {p99} ns");
crates/pull/src/capture.rs:598:13 False (F) !text.to_ascii_lowercase().contains("authorization")
crates/pull/src/capture.rs:599:20 False (F) && !text.to_ascii_lowercase().contains("access-token"),
crates/pull/src/cash_auction.rs:971 (F) matches!(
crates/pull/src/cash_session_cache.rs:1278 (F) unreachable!("{url}: a fully cached window must not use the network")
crates/pull/src/cash_session_cache.rs:1527 (F) unreachable!("{url}: cached dates must never fetch again")
crates/pull/src/cash_session_cache.rs:1608 (F) unreachable!("{url}: a root with no receipt is refused before any fetch")
crates/pull/src/cash_session_cache.rs:933:17 False (F) assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
crates/pull/src/cash_session_cache.rs:933:47 False (F) assert!(why.contains("UNVERIFIED") && why.contains("no files created"));
crates/pull/src/cash_session_cache.rs:938:17 False (F) assert!(why.contains("absent") && why.contains("no download attempted"));
crates/pull/src/cash_session_cache.rs:938:43 False (F) assert!(why.contains("absent") && why.contains("no download attempted"));
crates/pull/src/cash_session_cache.rs:940:17 False (F) assert!(!payload.exists() && !metadata.exists());
crates/pull/src/cash_session_cache.rs:940:38 False (F) assert!(!payload.exists() && !metadata.exists());
crates/pull/src/cash_session_cache.rs:983:17 False (F) why.contains("UNVERIFIED") && why.contains("receipt"),
crates/pull/src/cash_session_cache.rs:983:47 False (F) why.contains("UNVERIFIED") && why.contains("receipt"),
crates/pull/src/cash_session_cache.rs:1005:13 False (F) why.contains("unavailable") && why.contains("30 ms bound"),
crates/pull/src/cash_session_cache.rs:1005:44 False (F) why.contains("unavailable") && why.contains("30 ms bound"),
crates/pull/src/cash_session_cache.rs:1626:17 False (F) assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
crates/pull/src/cash_session_cache.rs:1626:49 False (F) assert!(error.contains("2026-08-23") && error.contains("HTTP 404"));
crates/pull/src/cash_session_cache.rs:1650:17 False (F) assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
crates/pull/src/cash_session_cache.rs:1650:49 False (F) assert!(error.contains("2026-08-04") && error.contains("HTTP 404"));
crates/pull/src/cash_session_cache.rs:2212:13 False (F) why.contains("readers or another installer held it")
crates/pull/src/cash_session_cache.rs:2213:20 False (F) && why.contains("for the whole 30 ms bound")
crates/pull/src/cash_session_cache.rs:2214:20 False (F) && why.contains("nothing was installed"),
crates/pull/src/cash_session_cache.rs:2284:21 False (F) why.starts_with("UNVERIFIED cannot open local lifecycle lock")
crates/pull/src/cash_session_cache.rs:2285:28 False (F) && why.contains(&lock.display().to_string()),
crates/pull/src/cash_session_cache.rs:2377:21 False (F) why.starts_with(&format!(
crates/pull/src/cash_session_cache.rs:2379:27 False (F) )) && why.ends_with("; crash durability UNVERIFIED"),
crates/pull/src/cash_session_cache_ps1_tests.rs:63 (F) panic!("the host took a shared flock on an O_PATH descriptor");
crates/pull/src/cash_session_cache_ps1_tests.rs:66 (F) panic!("the host took an exclusive flock on an O_PATH descriptor");
crates/pull/src/cash_session_cache_ps1_tests.rs:62:9 False (F) let Err(TryLockError::Error(shared)) = unlockable(&lock).try_lock_shared() else {
crates/pull/src/cash_session_cache_ps1_tests.rs:65:9 False (F) let Err(TryLockError::Error(exclusive)) = unlockable(&lock).try_lock() else {
crates/pull/src/expiry_alias.rs:1101 (F) matches!(
crates/pull/src/expiry_alias.rs:1250 (F) panic!("aliased")
crates/pull/src/expiry_alias.rs:1274 (F) panic!("aliased")
crates/pull/src/expiry_alias.rs:1607 (F) panic!("{under} {stated}: {:?}", old.settlement);
crates/pull/src/expiry_alias.rs:1659 (F) panic!("{:?}", old.settlement);
crates/pull/src/expiry_alias.rs:1108:21 False (F) assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
crates/pull/src/expiry_alias.rs:1108:33 False (F) assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
crates/pull/src/expiry_alias.rs:1109:21 False (F) assert!(first <= settled && first < stated, "{row:?}: first day");
crates/pull/src/expiry_alias.rs:1109:41 False (F) assert!(first <= settled && first < stated, "{row:?}: first day");
crates/pull/src/expiry_alias.rs:1182:55 True (F) !(a.underlying == b.underlying && a.stated == b.stated),
crates/pull/src/expiry_alias.rs:1249:13 False (F) let Settlement::Aliased(row) = chain.settlement else {
crates/pull/src/expiry_alias.rs:1273:13 False (F) let Settlement::Aliased(row) = got.settlement else {
crates/pull/src/expiry_alias.rs:1429:17 False (F) assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
crates/pull/src/expiry_alias.rs:1429:38 False (F) assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
crates/pull/src/expiry_alias.rs:1429:57 False (F) assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
crates/pull/src/expiry_alias.rs:1606:17 False (F) let Settlement::Aliased(row) = old.settlement else {
crates/pull/src/expiry_alias.rs:1658:21 False (F) let Settlement::ExpiredEarly(row) = old.settlement else {
crates/pull/src/expiry_alias.rs:1697:21 False (F) assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
crates/pull/src/expiry_alias.rs:1697:49 False (F) assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
crates/pull/src/fetch_ps1_tests.rs:123 (F) panic!("a short array is a length disagreement");
crates/pull/src/fetch_ps1_tests.rs:113:13 False (F) let FetchError::LengthDisagreement {
crates/pull/src/fno_membership_cov_tests.rs:109 (F) panic!("{refused:?}");
crates/pull/src/fno_membership_cov_tests.rs:108:9 False (F) let MembershipError::Corrupt { why, .. } = &refused else {
crates/pull/src/fno_membership_ps1_tests.rs:167 (F) unreachable!("membership is read from listed names: no tick file is fetched")
crates/pull/src/fno_membership_ps1_tests.rs:218 (F) panic!("{refusal:?}");
crates/pull/src/fno_membership_ps1_tests.rs:234 (F) panic!("{refusal:?}");
crates/pull/src/fno_membership_ps1_tests.rs:1035 (F) panic!("an invented non-ticker decoded");
crates/pull/src/fno_membership_ps1_tests.rs:1060 (F) panic!("{:?}", record.handovers);
crates/pull/src/fno_membership_ps1_tests.rs:1115 (F) panic!("both faults are one Io");
crates/pull/src/fno_membership_ps1_tests.rs:217:9 False (F) let MembershipError::Corrupt { path, why } = &refusal else {
crates/pull/src/fno_membership_ps1_tests.rs:233:9 False (F) let MembershipError::Stale { path, why } = &refusal else {
crates/pull/src/fno_membership_ps1_tests.rs:1034:9 False (F) let Err(why) = decode_ticker("ZQNOTATICKER", monday) else {
crates/pull/src/fno_membership_ps1_tests.rs:1059:9 False (F) let [handover] = record.handovers[..] else {
crates/pull/src/fno_membership_ps1_tests.rs:1112:9 False (F) let MembershipError::Io { path, why } =
crates/pull/src/fno_membership_ps1_tests.rs:1131:62 False (F) matches!(&alone, MembershipError::Io { path, .. } if *path == nowhere), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:79 (F) unreachable!("membership is read from listed names: no tick file is fetched")
crates/pull/src/fno_membership_tests.rs:265 (F) panic!("one handover: {:?}", held.handovers());
crates/pull/src/fno_membership_tests.rs:1204 (F) panic!("{reported:?}")
crates/pull/src/fno_membership_tests.rs:1473 (F) matches!(
crates/pull/src/fno_membership_tests.rs:264:9 False (F) let [handover] = held.handovers() else {
crates/pull/src/fno_membership_tests.rs:728:16 False (F) if *path == second && why.contains("BLAKE3")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:728:35 False (F) if *path == second && why.contains("BLAKE3")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:744:55 False (F) Err(MembershipError::Corrupt { path, .. }) if path.ends_with("notes.txt") -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:843:66 False (F) matches!(&refusal, MembershipError::Stale { why, .. } if why.contains("carry")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:1203:9 False (F) let [illiq, one] = reported.as_slice() else {
crates/pull/src/fno_membership_tests.rs:1207:9 False (F) illiq.starts_with("ZZILLIQ -> ZZNEWBIE")
crates/pull/src/fno_membership_tests.rs:1208:16 False (F) && illiq.ends_with("a minority of the new spelling's keys"),
crates/pull/src/fno_membership_tests.rs:1212:9 False (F) one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
crates/pull/src/fno_membership_tests.rs:1212:49 False (F) one.starts_with("ZZONE -> ZZNEWONE") && one.ends_with("fewer than 2 shared keys"),
crates/pull/src/fno_membership_tests.rs:1253:9 False (F) strays[0].1.starts_with("ZZOLDCO listed 1 contract key(s)")
crates/pull/src/fno_membership_tests.rs:1254:16 False (F) && strays[0].1.contains("filed under ZZNEWCO"),
crates/pull/src/fno_membership_tests.rs:1286:16 False (F) if *day == d(2024, 3, 6) && !why.is_empty()), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:1286:41 False (F) if *day == d(2024, 3, 6) && !why.is_empty()), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:1342:16 False (F) if *path == missing && why.contains("no such membership directory")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:1342:36 False (F) if *path == missing && why.contains("no such membership directory")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/fno_membership_tests.rs:1467:26 False (F) assert!(a.is_ok() || b.is_ok(), "round {round}: {a:?} | {b:?}");
crates/pull/src/folder.rs:638:13 False (F) !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
crates/pull/src/folder.rs:638:47 False (F) !derived.starts_with("/Users") && !derived.starts_with("/home/y"),
crates/pull/src/folder.rs:669:13 False (F) said.contains(ROOT_ENV) && said.contains("HOME"),
crates/pull/src/folder.rs:669:40 False (F) said.contains(ROOT_ENV) && said.contains("HOME"),
crates/pull/src/gdfl_cm_auction.rs:998 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_cm_auction.rs:1158 (F) other => panic!("{other:?}"),
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:42 (F) other => panic!("not a sidecar refusal: {other:?}"),
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:82 (F) panic!("the host took a shared flock on an O_PATH descriptor");
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:85 (F) panic!("the host took an exclusive flock on an O_PATH descriptor");
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:174 (F) cut: &|_, _| unreachable!("a fresh sidecar has no torn tail to cut"),
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:81:9 False (F) let Err(std::fs::TryLockError::Error(shared)) = unlockable(&lock_at).try_lock_shared() else {
crates/pull/src/gdfl_cm_auction_ps1_tests.rs:84:9 False (F) let Err(std::fs::TryLockError::Error(exclusive)) = unlockable(&lock_at).try_lock() else {
crates/pull/src/gdfl_cm_auction_v2_tests.rs:80 (F) Ok(sidecar) => panic!(
crates/pull/src/gdfl_cm_auction_v2_tests.rs:82 (F) sidecar.records.len()
crates/pull/src/gdfl_cm_auction_v2_tests.rs:91 (F) other => panic!("not a sidecar refusal: {other:?}"),
crates/pull/src/gdfl_cm_auction_v2_tests.rs:260 (F) made.display()
crates/pull/src/gdfl_cm_auction_v2_tests.rs:419 (F) Err(refusal) => panic!("a reader beside an append was refused: {refusal}"),
crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:17 False (F) refusal.why.contains("record 1") && refusal.why.contains("checksum"),
crates/pull/src/gdfl_cm_auction_v2_tests.rs:121:53 False (F) refusal.why.contains("record 1") && refusal.why.contains("checksum"),
crates/pull/src/gdfl_import_cov_tests.rs:48:9 False (F) why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
crates/pull/src/gdfl_import_cov_tests.rs:48:56 False (F) why.contains("is over the cap of 67108864") && why.contains("67108865 bytes"),
crates/pull/src/gdfl_import_cov_tests.rs:57:9 False (F) why.contains("lies past the end") && !why.contains("over the cap"),
crates/pull/src/gdfl_import_cov_tests.rs:57:46 False (F) why.contains("lies past the end") && !why.contains("over the cap"),
crates/pull/src/gdfl_import_cov_tests.rs:89:32 False (F) .is_some_and(|why| why.contains("plain.zip") && why.contains("no end-of-central")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_cov_tests.rs:150:9 False (F) fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
crates/pull/src/gdfl_import_cov_tests.rs:150:74 False (F) fault.contains("more than 2 entries; the walk stopped there") && fault.contains("ts"),
crates/pull/src/gdfl_import_cov_tests.rs:179:9 False (F) fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
crates/pull/src/gdfl_import_cov_tests.rs:179:40 False (F) fault.contains("FORMAT.md") && fault.contains("1048577 bytes, over the cap of 1048576"),
crates/pull/src/gdfl_import_cov_tests.rs:262:62 False (F) matches!(&refused, ImportRefusal::Journal { why } if why.contains("gdfl.journal")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_day_ps1_tests.rs:429 (F) panic!("a day before the range is never read whole");
crates/pull/src/gdfl_import_day_ps1_tests.rs:434 (F) panic!("without the flag the close still owes ZQB");
crates/pull/src/gdfl_import_day_ps1_tests.rs:478 (F) panic!("ZQZ's own key is still open");
crates/pull/src/gdfl_import_day_ps1_tests.rs:500 (F) panic!("a close under an unrecorded scope owes the members beyond the swept surface");
crates/pull/src/gdfl_import_day_ps1_tests.rs:503 (F) panic!("so does the second day's");
crates/pull/src/gdfl_import_day_ps1_tests.rs:535 (F) panic!("{:?}", report.failures);
crates/pull/src/gdfl_import_day_ps1_tests.rs:580 (F) panic!("a restated day is not read when the run asks not to restate")
crates/pull/src/gdfl_import_day_ps1_tests.rs:622 (F) unreachable!("a day every share of which stands asks no scope")
crates/pull/src/gdfl_import_day_ps1_tests.rs:732 (F) Ask::All => panic!("the day was closed done: read again for its new members only"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:776 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:816 (F) Ask::All => panic!("both days were closed done: read again for ZQB only"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:826 (F) panic!("{flags}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:940 (F) panic!("{key_held:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:948 (F) panic!("{run_finished:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:957 (F) panic!("{not_written:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:982 (F) panic!("{unrecorded_said:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1048 (F) |_, _| -> DayRead { panic!("no day is begun") },
crates/pull/src/gdfl_import_day_ps1_tests.rs:1059 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1089 (F) panic!("a refused run reads no day")
crates/pull/src/gdfl_import_day_ps1_tests.rs:1093 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1124 (F) |_, _| -> DayRead { panic!("a refused run reads no day") },
crates/pull/src/gdfl_import_day_ps1_tests.rs:1134 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1177 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1221 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1276 (F) Ask::All => panic!("no day of the range is begun"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1287 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1298 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1386 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1398 (F) panic!("{reach:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1473 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1597 (F) Ask::All => panic!("the day was closed done: read again for ZQB only"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1621 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1666 (F) panic!("{:?}", refused.failures);
crates/pull/src/gdfl_import_day_ps1_tests.rs:1701 (F) scopes_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1761 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1780 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1813 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:1827 (F) journal_text(&store)
crates/pull/src/gdfl_import_day_ps1_tests.rs:1858 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_day_ps1_tests.rs:428:9 False (F) let Owed::Probe(early) = ctx.owed(&own(&key, april(1), true)) else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:433:9 False (F) let Owed::Probe(beyond) = plain.owed(&own(&key, april(1), false)) else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:477:9 False (F) let Owed::Pending(members) = ctx.owed(&own(&key, april(1), false)) else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:499:9 False (F) let Owed::Probe(one) = ctx.owed(&own(&first, april(1), false)) else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:502:9 False (F) let Owed::Probe(two) = ctx.owed(&own(&second, april(2), false)) else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:534:9 False (F) let [refused] = report.failures.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:539:9 False (F) refused.why.starts_with("the import journal: ")
crates/pull/src/gdfl_import_day_ps1_tests.rs:540:16 False (F) && refused.why.contains(&ledger.display().to_string()),
crates/pull/src/gdfl_import_day_ps1_tests.rs:591:13 False (F) assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
crates/pull/src/gdfl_import_day_ps1_tests.rs:591:46 False (F) assert!(first_run.restated.is_empty() && first_run.resumed.is_empty());
crates/pull/src/gdfl_import_day_ps1_tests.rs:772:66 False (F) last_line_of(&store, &member_key).is_some_and(|line| line -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_day_ps1_tests.rs:825:9 False (F) let [line] = lines.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:939:9 False (F) let [held] = key_held.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:947:9 False (F) let [finished] = run_finished.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:956:9 False (F) let [unwritten] = not_written.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:981:9 False (F) let [said] = unrecorded_said.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1016:9 False (F) said.starts_with("run 1790100201 ")
crates/pull/src/gdfl_import_day_ps1_tests.rs:1017:16 False (F) && said.contains(&format!("\ndone indices {} * ", april(1))),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1051:9 False (F) let ImportRefusal::Stopped {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1092:9 False (F) let ImportRefusal::Journal { why } = &got else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1127:9 False (F) let ImportRefusal::StoreLost {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1170:9 False (F) let ImportRefusal::StoreLost {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1181:9 False (F) why.starts_with(&format!(
crates/pull/src/gdfl_import_day_ps1_tests.rs:1184:15 False (F) )) && why.contains("File exists"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1214:9 False (F) let ImportRefusal::StoreLost {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1280:9 False (F) let ImportRefusal::StoreNotWritable {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1397:9 False (F) let [said] = reach.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1665:9 False (F) let [named] = refused.failures.as_slice() else {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1754:9 False (F) let ImportRefusal::SourceLost {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1777:33 False (F) .is_some_and(|line| line.starts_with(&format!("incomplete {key} ")) -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_day_ps1_tests.rs:1806:9 False (F) let ImportRefusal::StoreNotWritable {
crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:9 False (F) why.contains("planted by the test") && why.contains("chflags nouchg"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1817:48 False (F) why.contains("planted by the test") && why.contains("chflags nouchg"),
crates/pull/src/gdfl_import_day_ps1_tests.rs:1824:33 False (F) .is_some_and(|line| line.starts_with(&format!("incomplete {key} ")) -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_day_ps1_tests.rs:1857:9 False (F) let ImportRefusal::StoreLost { why, .. } = &got else {
crates/pull/src/gdfl_import_ps1_tests.rs:171 (F) panic!("one failure, not {:?}", extras.failures);
crates/pull/src/gdfl_import_ps1_tests.rs:170:9 False (F) let [staged] = extras.failures.as_slice() else {
crates/pull/src/gdfl_import_tests.rs:382 (F) texts[8]
crates/pull/src/gdfl_import_tests.rs:387 (F) texts[9]
crates/pull/src/gdfl_import_tests.rs:558 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_tests.rs:1070 (F) matches!(
crates/pull/src/gdfl_import_tests.rs:2074 (F) bin.display()
crates/pull/src/gdfl_import_tests.rs:2630 (F) panic!("{got:?}");
crates/pull/src/gdfl_import_tests.rs:4440 (F) panic!("{rel}: not a bar month of the store");
crates/pull/src/gdfl_import_tests.rs:4707 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:4898 (F) indices[0].0
crates/pull/src/gdfl_import_tests.rs:4948 (F) bytes.len()
crates/pull/src/gdfl_import_tests.rs:5186 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5382 (F) durability.as_str()
crates/pull/src/gdfl_import_tests.rs:5388 (F) durability.as_str()
crates/pull/src/gdfl_import_tests.rs:5394 (F) durability.as_str(),
crates/pull/src/gdfl_import_tests.rs:5395 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5521 (F) panic!("the run stopped at the day's flush: {refused:?}");
crates/pull/src/gdfl_import_tests.rs:5675 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:5685 (F) one.display()
crates/pull/src/gdfl_import_tests.rs:5837 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:6036 (F) rel.display()
crates/pull/src/gdfl_import_tests.rs:6389 (F) panic!("the run is refused at the census: {got:?}")
crates/pull/src/gdfl_import_tests.rs:247:40 False (F) assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:380:9 False (F) texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
crates/pull/src/gdfl_import_tests.rs:380:63 False (F) texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
crates/pull/src/gdfl_import_tests.rs:400:13 False (F) assert!(all[0].report().is_none() && all[11].report().is_none());
crates/pull/src/gdfl_import_tests.rs:400:42 False (F) assert!(all[0].report().is_none() && all[11].report().is_none());
crates/pull/src/gdfl_import_tests.rs:550:9 False (F) let ImportRefusal::CensusFull {
crates/pull/src/gdfl_import_tests.rs:562:13 False (F) assert!(!measured.fits() && measured.free() == 1);
crates/pull/src/gdfl_import_tests.rs:562:33 False (F) assert!(!measured.fits() && measured.free() == 1);
crates/pull/src/gdfl_import_tests.rs:616:19 False (F) )) && !l.contains(" stage=") -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:623:9 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_import_tests.rs:625:15 False (F) )) && last.contains(" failures=1 ")
crates/pull/src/gdfl_import_tests.rs:626:16 False (F) && last.contains(" stage=census reason=too_many_entries run="),
crates/pull/src/gdfl_import_tests.rs:646:9 False (F) journal.contains("\ndone options 2024-04-04 * ")
crates/pull/src/gdfl_import_tests.rs:647:16 False (F) && journal.contains("\ndone options 2024-04-05 * "),
crates/pull/src/gdfl_import_tests.rs:685:9 False (F) text.contains("refused stocks 2024-04-01 although the day's batch fits")
crates/pull/src/gdfl_import_tests.rs:686:16 False (F) && text.contains("another ingest holds the census lock"),
crates/pull/src/gdfl_import_tests.rs:730:13 False (F) assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
crates/pull/src/gdfl_import_tests.rs:730:27 False (F) assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
crates/pull/src/gdfl_import_tests.rs:1028:13 False (F) assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
crates/pull/src/gdfl_import_tests.rs:1028:47 False (F) assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
crates/pull/src/gdfl_import_tests.rs:1029:13 False (F) assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
crates/pull/src/gdfl_import_tests.rs:1029:47 False (F) assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
crates/pull/src/gdfl_import_tests.rs:1031:9 False (F) journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
crates/pull/src/gdfl_import_tests.rs:1031:43 False (F) journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
crates/pull/src/gdfl_import_tests.rs:1034:13 False (F) assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
crates/pull/src/gdfl_import_tests.rs:1034:53 False (F) assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
crates/pull/src/gdfl_import_tests.rs:1042:9 False (F) journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
crates/pull/src/gdfl_import_tests.rs:1042:43 False (F) journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
crates/pull/src/gdfl_import_tests.rs:1045:13 False (F) assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
crates/pull/src/gdfl_import_tests.rs:1045:51 False (F) assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
crates/pull/src/gdfl_import_tests.rs:1364:13 False (F) assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
crates/pull/src/gdfl_import_tests.rs:1364:38 False (F) assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
crates/pull/src/gdfl_import_tests.rs:1455:9 False (F) damaged.digest.is_none()
crates/pull/src/gdfl_import_tests.rs:1456:16 False (F) && damaged
crates/pull/src/gdfl_import_tests.rs:1489:9 False (F) unformatted.digest.is_none()
crates/pull/src/gdfl_import_tests.rs:1490:16 False (F) && unformatted
crates/pull/src/gdfl_import_tests.rs:1632:9 False (F) second.contains(" source=zip ")
crates/pull/src/gdfl_import_tests.rs:1633:16 False (F) && second.contains(" commit=unstamped ")
crates/pull/src/gdfl_import_tests.rs:1634:16 False (F) && second.contains(" started=1750000000 ")
crates/pull/src/gdfl_import_tests.rs:1635:16 False (F) && second.ends_with(&format!(
crates/pull/src/gdfl_import_tests.rs:1656:9 False (F) first.starts_with("run ")
crates/pull/src/gdfl_import_tests.rs:1657:16 False (F) && first.contains(
crates/pull/src/gdfl_import_tests.rs:1704:9 False (F) line.starts_with("incomplete indices 2024-04-01 ")
crates/pull/src/gdfl_import_tests.rs:1707:16 False (F) && line.contains(" stage=file reason=bars_differ_held run="),
crates/pull/src/gdfl_import_tests.rs:1719:9 False (F) line.starts_with("incomplete options 2024-04-01 ")
crates/pull/src/gdfl_import_tests.rs:1720:16 False (F) && line.contains(" stage=vendor reason="),
crates/pull/src/gdfl_import_tests.rs:2010:13 False (F) ra.seconds > 0 && ra.seconds_committed == ra.seconds,
crates/pull/src/gdfl_import_tests.rs:2010:31 False (F) ra.seconds > 0 && ra.seconds_committed == ra.seconds,
crates/pull/src/gdfl_import_tests.rs:2022:13 False (F) assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
crates/pull/src/gdfl_import_tests.rs:2022:41 False (F) assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
crates/pull/src/gdfl_import_tests.rs:2037:22 False (F) .all(|l| l.contains(" member_bytes=0 ") && l.ends_with(" member_src=none")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2042:22 False (F) .all(|l| !l.contains(" member_bytes=0 ") && l.ends_with(" member_src=zip")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2083:22 False (F) .all(|b| b.volume == 0 && b.open_interest == OI_NULL) -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2092:31 False (F) assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2622:9 False (F) let ImportRefusal::CensusRefused {
crates/pull/src/gdfl_import_tests.rs:2636:34 False (F) .is_err_and(|unread| unread.contains("gdfl.man") && unread.contains("not a regular")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2644:22 False (F) .any(|f| f.instrument == "2024-04-01 census" && f.why.contains("nothing of the batch")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:2650:9 False (F) text.contains("indices 2024-04-01")
crates/pull/src/gdfl_import_tests.rs:2651:16 False (F) && text.contains("its room could not be measured")
crates/pull/src/gdfl_import_tests.rs:2652:16 False (F) && text.contains("the run stopped there after 0 day(s) imported"),
crates/pull/src/gdfl_import_tests.rs:2659:9 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_import_tests.rs:2661:15 False (F) )) && last.contains(" stage=census reason=census_refused run="),
crates/pull/src/gdfl_import_tests.rs:2913:74 False (F) matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder")) -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_import_tests.rs:4356:20 False (F) || crate::gdfl_lineage::EventRecord::parse(&line).is_some(),
crates/pull/src/gdfl_import_tests.rs:4439:17 False (F) let [_, _, _, _, under, contract, ..] = parts.as_slice() else {
crates/pull/src/gdfl_import_tests.rs:4683:17 False (F) assert!(one.provenance_records > 0 && one.seconds_committed > 0);
crates/pull/src/gdfl_import_tests.rs:4683:47 False (F) assert!(one.provenance_records > 0 && one.seconds_committed > 0);
crates/pull/src/gdfl_import_tests.rs:4896:9 False (F) indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
crates/pull/src/gdfl_import_tests.rs:4896:36 False (F) indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
crates/pull/src/gdfl_import_tests.rs:4944:21 False (F) u64::try_from(bytes.len()).unwrap() <= store::format::HEADER_LEN
crates/pull/src/gdfl_import_tests.rs:4945:28 False (F) && rest.iter().all(|&byte| byte == 0)
crates/pull/src/gdfl_import_tests.rs:4946:28 False (F) && store::header::Header::decode(slot).is_err(),
crates/pull/src/gdfl_import_tests.rs:5128:17 False (F) !status.success() && journal.is_empty() && months == 0 && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5128:38 False (F) !status.success() && journal.is_empty() && months == 0 && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5128:60 False (F) !status.success() && journal.is_empty() && months == 0 && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5128:75 False (F) !status.success() && journal.is_empty() && months == 0 && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5133:17 False (F) status.success() && !open_day && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5133:37 False (F) status.success() && !open_day && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5133:50 False (F) status.success() && !open_day && unborn == 0,
crates/pull/src/gdfl_import_tests.rs:5140:28 False (F) unborn == 0 || open_day,
crates/pull/src/gdfl_import_tests.rs:5374:13 False (F) assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
crates/pull/src/gdfl_import_tests.rs:5374:50 False (F) assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
crates/pull/src/gdfl_import_tests.rs:5514:9 False (F) let Err(ImportRefusal::Durability {
crates/pull/src/gdfl_import_tests.rs:5525:9 False (F) why.contains("syncing") && why.contains(&store.display().to_string()),
crates/pull/src/gdfl_import_tests.rs:5525:36 False (F) why.contains("syncing") && why.contains(&store.display().to_string()),
crates/pull/src/gdfl_import_tests.rs:5820:13 False (F) last.starts_with(&format!("incomplete options {}", d2()))
crates/pull/src/gdfl_import_tests.rs:5821:20 False (F) && last.contains(&format!(" failures={} ", refused + 1)),
crates/pull/src/gdfl_import_tests.rs:5908:13 False (F) assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
crates/pull/src/gdfl_import_tests.rs:5908:41 False (F) assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
crates/pull/src/gdfl_import_tests.rs:6181:46 False (F) why.contains("holds") || why.contains("lock"),
crates/pull/src/gdfl_import_tests.rs:6388:9 False (F) let Err(ImportRefusal::CensusRefused { day: at, why, .. }) = &got else {
crates/pull/src/gdfl_nfo_tests.rs:295 (F) matches!(
crates/pull/src/gdfl_nfo_tests.rs:315 (F) unreachable!("read_file is handed its listing: it never asks the source for a day")
crates/pull/src/gdfl_nfo_tests.rs:358 (F) panic!("{name}: {got:?}")
crates/pull/src/gdfl_nfo_tests.rs:352:13 False (F) let NfoRefusal::StrikeImplausible {
crates/pull/src/gdfl_nfo_tests.rs:1909:9 False (F) words.contains("2026.zip:") && words.contains("2025.zip:"),
crates/pull/src/gdfl_nfo_tests.rs:1909:40 False (F) words.contains("2026.zip:") && words.contains("2025.zip:"),
crates/pull/src/gdfl_r3_attack_tests.rs:442 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:451 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:476 (F) String::from_utf8_lossy(tail)
crates/pull/src/gdfl_r3_attack_tests.rs:772 (F) block.len(),
crates/pull/src/gdfl_r3_attack_tests.rs:773 (F) got.as_ref().map(Vec::len)
crates/pull/src/gdfl_r3_attack_tests.rs:788 (F) block.len(),
crates/pull/src/gdfl_r3_attack_tests.rs:789 (F) got.as_ref().map(Vec::len)
crates/pull/src/gdfl_r3_attack_tests.rs:820 (F) got.map(|b| b.len())
crates/pull/src/gdfl_r3_attack_tests.rs:903 (F) unreachable!("only swept index names are drawn")
crates/pull/src/gdfl_r3_attack_tests.rs:1420 (F) all.files + all.files_refused
crates/pull/src/gdfl_r3_attack_tests.rs:377:34 False (F) name == want_name || name.starts_with(&format!("{want_name}.")),
crates/pull/src/gdfl_r3_attack_tests.rs:589:9 False (F) loads > 15_000 && line_tears > 2_000 && close_tears > 500,
crates/pull/src/gdfl_r3_attack_tests.rs:589:27 False (F) loads > 15_000 && line_tears > 2_000 && close_tears > 500,
crates/pull/src/gdfl_r3_attack_tests.rs:589:49 False (F) loads > 15_000 && line_tears > 2_000 && close_tears > 500,
crates/pull/src/gdfl_r3_attack_tests.rs:818:29 False (F) got.is_err() || rebuilt_whole,
crates/pull/src/gdfl_r3_attack_tests.rs:1219:9 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r3_attack_tests.rs:1219:25 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r3_attack_tests.rs:1219:43 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r3_attack_tests.rs:1228:9 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r3_attack_tests.rs:1228:25 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r3_attack_tests.rs:1228:43 False (F) bars > 1_000 && refusals > 300 && filtered > 150,
crates/pull/src/gdfl_r4_attack_tests.rs:130 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:205 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:678 (F) journal_text(&root)
crates/pull/src/gdfl_r4_attack_tests.rs:282:13 False (F) last.starts_with(&format!(
crates/pull/src/gdfl_r4_attack_tests.rs:284:19 False (F) )) && last.contains(" late=1 late_unresolved=0 max_back_s=7 ")
crates/pull/src/gdfl_r4_attack_tests.rs:285:20 False (F) && last.contains(" stage=file reason=bars_differ_held"),
crates/pull/src/gdfl_r4_attack_tests.rs:491:13 False (F) assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
crates/pull/src/gdfl_r4_attack_tests.rs:491:33 False (F) assert!(decoded > 20_000 && built == 300_000, "{decoded} decoded");
crates/pull/src/gdfl_r4_attack_tests.rs:799:20 False (F) || appended.starts_with("done stocks 2024-04-")
crates/pull/src/gdfl_r4_attack_tests.rs:800:24 False (F) && appended.contains(&format!(" definition={BAR_DEFINITION} ")),
crates/pull/src/gdfl_r4_attack_tests.rs:818:13 False (F) assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
crates/pull/src/gdfl_r4_attack_tests.rs:818:54 False (F) assert!(loaded.restated.iter().eq([&options]) && loaded.open.is_empty());
crates/pull/src/gdfl_r4_attack_tests.rs:819:13 False (F) assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
crates/pull/src/gdfl_r4_attack_tests.rs:819:62 False (F) assert!(loaded.done.contains(&key("indices", 1, "*")) && loaded.incomplete.is_empty());
crates/pull/src/gdfl_run_locks_cov_tests.rs:70:9 False (F) said.contains("was v3 when") && said.contains("is v4 now"),
crates/pull/src/gdfl_run_locks_cov_tests.rs:70:41 False (F) said.contains("was v3 when") && said.contains("is v4 now"),
crates/pull/src/gdfl_run_locks_cov_tests.rs:161:9 False (F) long.len() <= HOST_MAX && long.len().is_multiple_of(3),
crates/pull/src/gdfl_run_locks_cov_tests.rs:161:35 False (F) long.len() <= HOST_MAX && long.len().is_multiple_of(3),
crates/pull/src/gdfl_run_locks_cov_tests.rs:301:61 False (F) matches!(&state, LockState::Unprobed { why, .. } if why == "is a directory"), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_run_locks_ps1_tests.rs:67 (F) panic!("a file at the census's name was located");
crates/pull/src/gdfl_run_locks_ps1_tests.rs:212 (F) panic!("the root opened as a lock file");
crates/pull/src/gdfl_run_locks_ps1_tests.rs:229 (F) panic!("a directory opened as a lock file");
crates/pull/src/gdfl_run_locks_ps1_tests.rs:283 (F) panic!("the host took a flock it refuses");
crates/pull/src/gdfl_run_locks_ps1_tests.rs:353 (F) matches!(
crates/pull/src/gdfl_run_locks_ps1_tests.rs:66:9 False (F) let Err(why) = locate(&store, Vendor::Gdfl) else {
crates/pull/src/gdfl_run_locks_ps1_tests.rs:211:9 False (F) let Err(LockFault::Refused { lock, why }) = open_lock_file(&store, "/") else {
crates/pull/src/gdfl_run_locks_ps1_tests.rs:228:9 False (F) let Err(LockFault::Refused { lock, why }) = take_path_within(&path, Duration::ZERO) else {
crates/pull/src/gdfl_run_locks_ps1_tests.rs:282:9 False (F) let Err(TryLockError::Error(host)) = unlockable(&path).try_lock() else {
crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:9 False (F) output.status.success() && said.contains("1 passed"),
crates/pull/src/gdfl_run_locks_ps1_tests.rs:406:36 False (F) output.status.success() && said.contains("1 passed"),
crates/pull/src/gdfl_run_locks_ps1_tests.rs:503:35 False (F) faults.iter().all(|fault| fault -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/gdfl_run_stop_cov_tests.rs:94 (F) panic!("{refused:?}");
crates/pull/src/gdfl_run_stop_cov_tests.rs:93:9 False (F) let StopRefusal::ClaimBeingWritten { lock } = &refused else {
crates/pull/src/listing_tests.rs:146 (F) panic!("a master span names its master");
crates/pull/src/listing_tests.rs:491 (F) Err(why) => panic!("an odd name: {why}"),
crates/pull/src/listing_tests.rs:145:9 False (F) let Source::NseSecurityMaster { file, sha256 } = span.source else {
crates/pull/src/manifest_ps1_tests.rs:108 (F) String::from_utf8_lossy(&magic)
crates/pull/src/master_snapshots_cov_tests.rs:695 (F) wrote[0]
crates/pull/src/master_snapshots_cov_tests.rs:380:9 False (F) words.contains(&"expiry") && words.contains(&"lot_size"),
crates/pull/src/master_snapshots_cov_tests.rs:380:38 False (F) words.contains(&"expiry") && words.contains(&"lot_size"),
crates/pull/src/master_snapshots_first_listed_tests.rs:515 (F) other @ Rebuilt::Done(..) => panic!("{other:?}"),
crates/pull/src/master_snapshots_first_listed_tests.rs:107:13 False (F) assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
crates/pull/src/master_snapshots_first_listed_tests.rs:107:32 False (F) assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
crates/pull/src/master_snapshots_first_listed_tests.rs:107:52 False (F) assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
crates/pull/src/master_snapshots_first_listed_tests.rs:107:72 False (F) assert!(!carried("A|B") && !carried("A\nB") && !carried("A\rB") && !carried(""));
crates/pull/src/master_snapshots_first_listed_tests.rs:199:9 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
crates/pull/src/master_snapshots_first_listed_tests.rs:199:38 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
crates/pull/src/master_snapshots_first_listed_tests.rs:234:13 False (F) assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
crates/pull/src/master_snapshots_first_listed_tests.rs:234:38 False (F) assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
crates/pull/src/master_snapshots_first_listed_tests.rs:234:65 False (F) assert!(none.first.is_empty() && none.refused.is_empty() && none.rebuilt.is_none());
crates/pull/src/master_snapshots_first_listed_tests.rs:279:13 False (F) listed.refused[0]
crates/pull/src/master_snapshots_first_listed_tests.rs:281:20 False (F) && listed.refused[0].contains("recorded when the futures index took it in"),
crates/pull/src/master_snapshots_first_listed_tests.rs:380:9 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
crates/pull/src/master_snapshots_first_listed_tests.rs:380:38 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
crates/pull/src/master_snapshots_first_listed_tests.rs:448:13 False (F) said.starts_with("generation g1 of the futures index did not match its commit log")
crates/pull/src/master_snapshots_first_listed_tests.rs:449:20 False (F) && said.contains(why)
crates/pull/src/master_snapshots_first_listed_tests.rs:450:20 False (F) && said.contains("generation g2 was started"),
crates/pull/src/master_snapshots_first_listed_tests.rs:467:13 False (F) record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
crates/pull/src/master_snapshots_first_listed_tests.rs:467:64 False (F) record.starts_with("fl2|generation|2|NFO|FUT|") && record.contains(why),
crates/pull/src/master_snapshots_first_listed_tests.rs:518:9 False (F) why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
crates/pull/src/master_snapshots_first_listed_tests.rs:518:40 False (F) why.contains("(g1 rotted)") && why.ends_with("either: g2 rotted"),
crates/pull/src/master_snapshots_first_listed_tests.rs:581:9 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
crates/pull/src/master_snapshots_first_listed_tests.rs:581:38 False (F) listed.refused.is_empty() && listed.rebuilt.is_none(),
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
crates/pull/src/master_snapshots_tests.rs:149:9 False (F) let TokenOn::Resolved {
crates/pull/src/master_snapshots_tests.rs:156:9 False (F) let TokenOn::Resolved {
crates/pull/src/master_snapshots_tests.rs:191:9 False (F) let TokenOn::Resolved { snapshot, listed } = book.token_on(between, "NFO", 9_000_002) else {
crates/pull/src/master_snapshots_tests.rs:197:9 False (F) let TokenOn::Resolved { listed, .. } = book.token_on(day(2026, 2, 20), "NFO", 9_000_001) else {
crates/pull/src/master_snapshots_tests.rs:304:9 False (F) let TokenOn::Resolved { listed, .. } = book.token_on(d1, "NFO", 9_000_021) else {
crates/pull/src/master_snapshots_tests.rs:362:9 False (F) json.contains(r#""incomplete":{"held":3,"named":[{"file":"instruments_20260304.csv.gz","stamp":"2026-03-04","why":"a body with no receipt (a
crates/pull/src/master_snapshots_tests.rs:363:16 False (F) && json.contains(r#""unnamed":0,"refusal":null}"#),
crates/pull/src/master_snapshots_tests.rs:369:9 False (F) why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
crates/pull/src/master_snapshots_tests.rs:369:52 False (F) why.starts_with("ordinal 2: 1 entries") && why.contains(&cut_today),
crates/pull/src/master_snapshots_tests.rs:466:9 False (F) json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
crates/pull/src/master_snapshots_tests.rs:466:57 False (F) json.contains(&format!(r#""held":{held},"#)) && json.contains(r#""unnamed":2,"#),
crates/pull/src/master_snapshots_tests.rs:823:9 False (F) why.contains("after today 2026-02-01")
crates/pull/src/master_snapshots_tests.rs:824:16 False (F) && why.contains(&file_name(Kind::Kite, stamp(d2, 1)))
crates/pull/src/master_snapshots_tests.rs:825:16 False (F) && why.contains("refused rather than believed fresh"),
crates/pull/src/master_snapshots_tests.rs:886:9 False (F) garbage.contains("line 2") && garbage.contains("instrument_token"),
crates/pull/src/master_snapshots_tests.rs:886:39 False (F) garbage.contains("line 2") && garbage.contains("instrument_token"),
crates/pull/src/master_snapshots_tests.rs:923:9 False (F) let Lookup::Listed(listed) = one.lookup("NFO", 9_800_001) else {
crates/pull/src/master_snapshots_tests.rs:943:9 False (F) let Lookup::Listed(again) = other.lookup("NFO", 9_800_001) else {
crates/pull/src/master_snapshots_tests.rs:1219:9 False (F) why.contains("no snapshot was written") && why.contains("1970"),
crates/pull/src/master_snapshots_tests.rs:1219:52 False (F) why.contains("no snapshot was written") && why.contains("1970"),
crates/pull/src/master_snapshots_tests.rs:1336:9 False (F) let TokenOn::Resolved {
crates/pull/src/master_snapshots_tests.rs:1387:16 False (F) && one.to_string().starts_with(&partial) -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1471:9 False (F) let Some(Ok(Some(ref summary))) = outcome.diff else {
crates/pull/src/master_snapshots_tests.rs:1511:47 False (F) catalog.incomplete().iter().any(|one| one.file == cut -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1512:16 False (F) && one.kind == Kind::Kite -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1513:16 False (F) && one.stamp == b -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1635:24 False (F) .any(|one| one.stamp == cut && one.why.contains("no receipt")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1670:24 False (F) .any(|one| one.file == twin && one.why.contains("another case")), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1734:24 False (F) .any(|one| one.file == upper && one.why == CUT_DIFF), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/master_snapshots_tests.rs:1958:9 False (F) let Lookup::Listed(listed) = one.lookup("NFO", 9_800_101) else {
crates/pull/src/master_snapshots_tests.rs:1969:9 False (F) let Lookup::Listed(first) = parsed.lookup("NFO", 9_800_102) else {
crates/pull/src/master_snapshots_tests.rs:2001:9 False (F) let Lookup::Listed(other) = parsed.lookup("NFO", 9_900_102) else {
crates/pull/src/nse.rs:885 (F) panic!("an interstitial must refuse as NotCsv, got {why:?}");
crates/pull/src/nse.rs:1258 (F) panic!("got {why:?}");
crates/pull/src/nse.rs:884:13 False (F) let NseError::NotCsv { ref head } = why else {
crates/pull/src/nse.rs:1018:24 False (F) if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/nse.rs:1018:59 False (F) if bytes == MAX_DOCUMENT_BYTES + 1 && cap == MAX_DOCUMENT_BYTES -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/nse.rs:1026:51 False (F) NseError::HeaderUnexpected { got } if got.len() == MAX_DOCUMENT_BYTES -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/nse.rs:1217:72 False (F) matches!(refused, NseError::TooManyIndexLinks { links } if links > MAX_INDEX_LINKS), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/nse.rs:1257:13 False (F) let NseError::NoConstituentLink { ref page } = why else {
crates/pull/src/nse_ps1_tests.rs:34 (F) panic!("an open value is no link");
crates/pull/src/nse_ps1_tests.rs:65 (F) panic!("the bound is TooManyRows");
crates/pull/src/nse_ps1_tests.rs:31:9 False (F) let NseError::NoConstituentLink { page } =
crates/pull/src/nse_ps1_tests.rs:62:9 False (F) let NseError::TooManyRows { rows, cap } =
crates/pull/src/pricing.rs:1421 (F) matches!(
crates/pull/src/pricing.rs:1434 (F) matches!(
crates/pull/src/pricing.rs:1616 (F) matches!(
crates/pull/src/pricing.rs:1873 (F) matches!(
crates/pull/src/pricing.rs:1178:13 False (F) solved.volatility > 0.0 && solved.volatility < 5.0,
crates/pull/src/pricing.rs:1178:40 False (F) solved.volatility > 0.0 && solved.volatility < 5.0,
crates/pull/src/pricing.rs:1310:13 False (F) at_zero > at_twelve && at_five_five > at_eight,
crates/pull/src/pricing.rs:1310:36 False (F) at_zero > at_twelve && at_five_five > at_eight,
crates/pull/src/pricing.rs:1361:64 False (F) Err(PricingError::RateNotFinite { annual }) if annual.is_nan() -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/pricing.rs:1399:79 False (F) matches!(error, PricingError::NotPositive { field: f, .. } if f == field), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/pricing.rs:1501:17 False (F) assert!(row.volatility > 0.0 && row.volatility < 5.0);
crates/pull/src/pricing.rs:1501:41 False (F) assert!(row.volatility > 0.0 && row.volatility < 5.0);
crates/pull/src/pricing.rs:1833:45 False (F) OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
crates/pull/src/pricing.rs:1833:64 False (F) OptionSide::Call => assert!(got.delta > 0.0 && got.rho_rupees_per_rate_point > 0.0),
crates/pull/src/pricing.rs:1834:44 False (F) OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
crates/pull/src/pricing.rs:1834:63 False (F) OptionSide::Put => assert!(got.delta < 0.0 && got.rho_rupees_per_rate_point < 0.0),
crates/pull/src/pricing.rs:1836:21 False (F) assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
crates/pull/src/pricing.rs:1836:50 False (F) assert!(got.gamma_per_rupee > 0.0 && got.vega_rupees_per_vol_point > 0.0);
crates/pull/src/pricing.rs:1889:13 False (F) price.to_bits() == 100_f64.to_bits() && floor > price,
crates/pull/src/pricing.rs:1889:53 False (F) price.to_bits() == 100_f64.to_bits() && floor > price,
crates/pull/src/pricing.rs:1928:77 False (F) matches!(why, PricingError::NotPositive { field: f, .. } if f == field), -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/pricing_ps1_tests.rs:59 (F) panic!("a day the ladder does not cover is LadderUnknown");
crates/pull/src/pricing_ps1_tests.rs:72 (F) panic!("a spot on rung zero is LadderUnknown");
crates/pull/src/pricing_ps1_tests.rs:56:9 False (F) let PricingError::LadderUnknown { why } =
crates/pull/src/pricing_ps1_tests.rs:69:9 False (F) let PricingError::LadderUnknown { why: rung } =
crates/pull/src/reference_data_cov_tests.rs:391:9 False (F) current.contains(&"ZQA") && current.contains(&"ZQNOW"),
crates/pull/src/reference_data_cov_tests.rs:391:37 False (F) current.contains(&"ZQA") && current.contains(&"ZQNOW"),
crates/pull/src/reference_data_ps1_tests.rs:626 (F) panic!("{outcome:?}");
crates/pull/src/reference_data_ps1_tests.rs:652 (F) panic!("a draft with no decision was written");
crates/pull/src/reference_data_ps1_tests.rs:707 (F) panic!("{outcome:?}");
crates/pull/src/reference_data_ps1_tests.rs:761 (F) panic!("another fault is Io");
crates/pull/src/reference_data_ps1_tests.rs:625:9 False (F) let Err(WriteError::Refused(refused)) = &outcome else {
crates/pull/src/reference_data_ps1_tests.rs:651:9 False (F) let Err(WriteError::Draft(why)) = write(&store, &unrendered) else {
crates/pull/src/reference_data_ps1_tests.rs:706:9 False (F) let Err(WriteError::Io(said)) = &outcome else {
crates/pull/src/reference_data_ps1_tests.rs:760:9 False (F) let WriteError::Io(why) = write_linking(&store, &fno, &gone).unwrap_err() else {
crates/pull/src/reference_data_tests.rs:428 (F) other => panic!("{other:?}"),
crates/pull/src/reference_data_tests.rs:481 (F) other => panic!("{other:?}"),
crates/pull/src/reference_data_tests.rs:592 (F) mine[0]
crates/pull/src/reference_data_tests.rs:774 (F) other => panic!("a span past MAX_ROWS days must be refused: {other:?}"),
crates/pull/src/reference_data_tests.rs:881 (F) Err(other) => panic!("{other}"),
crates/pull/src/reference_data_tests.rs:144:9 False (F) said.contains("index_identity") && said.contains("checksum"),
crates/pull/src/reference_data_tests.rs:144:44 False (F) said.contains("index_identity") && said.contains("checksum"),
crates/pull/src/reference_data_tests.rs:197:21 False (F) assert!(receipt.is_some() && refused.is_none());
crates/pull/src/reference_data_tests.rs:197:42 False (F) assert!(receipt.is_some() && refused.is_none());
crates/pull/src/reference_data_tests.rs:523:13 False (F) assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
crates/pull/src/reference_data_tests.rs:523:47 False (F) assert!(calendar.horizon(last).alert() && !calendar.horizon(last).ended);
crates/pull/src/reference_data_tests.rs:1345:9 False (F) said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
crates/pull/src/reference_data_tests.rs:1345:70 False (F) said.starts_with("1 documented gap row(s) carry nothing") && said.contains("2020-01-20"),
crates/pull/src/rehearsal_tests.rs:277 (F) why.len()
crates/pull/src/rehearsal_tests.rs:275:9 False (F) why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
crates/pull/src/rehearsal_tests.rs:275:39 False (F) why.len() <= MAX_WHY_BYTES && why.len() >= MAX_WHY_BYTES - 1,
crates/pull/src/rehearsal_tests.rs:498:13 False (F) why.contains("S-90: operator_yes") && why.contains(says),
crates/pull/src/rehearsal_tests.rs:498:51 False (F) why.contains("S-90: operator_yes") && why.contains(says),
crates/pull/src/request_minutes.rs:1320 (F) texts(&none)
crates/pull/src/request_minutes.rs:1360 (F) panic!("{day} is attested")
crates/pull/src/request_minutes.rs:1479 (F) groups[1].text
crates/pull/src/request_minutes.rs:1879 (F) panic!("{:?}", audit.findings);
crates/pull/src/request_minutes.rs:1914 (F) panic!(
crates/pull/src/request_minutes.rs:1951 (F) panic!("{groups:?}");
crates/pull/src/request_minutes.rs:2058 (F) panic!("{:?}", audit.days);
crates/pull/src/request_minutes.rs:2136 (F) panic!("{:?}", audit.findings);
crates/pull/src/request_minutes.rs:2180 (F) panic!("{:?}", done.failures);
crates/pull/src/request_minutes.rs:2202 (F) lines[0]
crates/pull/src/request_minutes.rs:2238 (F) named.failures[1].why
crates/pull/src/request_minutes.rs:2255 (F) panic!("{day}: {:?}", audit.findings);
crates/pull/src/request_minutes.rs:2289 (F) panic!("{:?}", audit.days);
crates/pull/src/request_minutes.rs:2351 (F) panic!("{:?}", unflagged.findings);
crates/pull/src/request_minutes.rs:1283:13 False (F) only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
crates/pull/src/request_minutes.rs:1283:47 False (F) only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
crates/pull/src/request_minutes.rs:1300:13 False (F) whole.findings.is_empty() && whole.days.is_empty(),
crates/pull/src/request_minutes.rs:1300:42 False (F) whole.findings.is_empty() && whole.days.is_empty(),
crates/pull/src/request_minutes.rs:1475:13 False (F) groups[1].text.starts_with(
crates/pull/src/request_minutes.rs:1477:18 False (F) ) && groups[1].text.ends_with("the first: gap b"),
crates/pull/src/request_minutes.rs:1683:21 False (F) first.starts_with(head) && first.contains(" P0 "),
crates/pull/src/request_minutes.rs:1683:48 False (F) first.starts_with(head) && first.contains(" P0 "),
crates/pull/src/request_minutes.rs:1878:13 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:1886:13 False (F) finding
crates/pull/src/request_minutes.rs:1889:20 False (F) && finding
crates/pull/src/request_minutes.rs:1913:13 False (F) let Some(number) = found else {
crates/pull/src/request_minutes.rs:1950:13 False (F) let [group] = groups.as_slice() else {
crates/pull/src/request_minutes.rs:2057:13 False (F) let [line] = audit.days.as_slice() else {
crates/pull/src/request_minutes.rs:2135:13 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:2179:13 False (F) let [failure] = done.failures.as_slice() else {
crates/pull/src/request_minutes.rs:2254:17 False (F) let [finding] = audit.findings.as_slice() else {
crates/pull/src/request_minutes.rs:2288:13 False (F) let [line] = audit.days.as_slice() else {
crates/pull/src/request_minutes.rs:2317:13 False (F) audit.findings.is_empty() && audit.days.is_empty(),
crates/pull/src/request_minutes.rs:2317:42 False (F) audit.findings.is_empty() && audit.days.is_empty(),
crates/pull/src/request_minutes.rs:2332:13 False (F) audit.findings.is_empty() && audit.days.is_empty(),
crates/pull/src/request_minutes.rs:2332:42 False (F) audit.findings.is_empty() && audit.days.is_empty(),
crates/pull/src/request_minutes.rs:2350:13 False (F) let [finding] = unflagged.findings.as_slice() else {
crates/pull/src/scrub.rs:718:50 False (F) Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty() -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/scrub.rs:718:86 False (F) Finding::Unreadable { path, why } if path == at.display().to_string() && !why.is_empty() -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/src/scrub.rs:759:31 False (F) i == j || one != other,
crates/pull/src/series_identity.rs:977:17 False (F) assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
crates/pull/src/series_identity.rs:977:39 False (F) assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
crates/pull/src/series_identity.rs:977:61 False (F) assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
crates/pull/src/ssm.rs:1391 (F) token.is_some()
crates/pull/src/ssm.rs:1479 (F) panic!("{region:?} is not ap-south-1 and must not be signed for")
crates/pull/src/ssm.rs:1508 (F) panic!("a stamp with no date in it cannot scope a signature")
crates/pull/src/ssm.rs:1550 (F) panic!("{wrong} carries no Parameter.Value and must be refused")
crates/pull/src/ssm.rs:1555 (F) panic!("a non-JSON answer is a different fault and is named as one")
crates/pull/src/ssm.rs:1657 (F) panic!("a profile that is not in the file must be refused")
crates/pull/src/ssm.rs:1668 (F) panic!("an id with no secret cannot sign anything")
crates/pull/src/ssm.rs:1694 (F) panic!("{tag}: a blank half of the pair is no pair")
crates/pull/src/ssm.rs:1721 (F) panic!("an empty exported key id must not become an identity")
crates/pull/src/ssm.rs:1768 (F) panic!("a half-set environment must be refused: {found:?}")
crates/pull/src/ssm.rs:1806 (F) panic!("a file that is not there cannot hold an identity")
crates/pull/src/ssm.rs:1865 (F) panic!("a foreign region must not yield a parameter value")
crates/pull/src/ssm.rs:1885 (F) panic!("{bad} not UTF-8 must be refused")
crates/pull/src/ssm.rs:1949 (F) if answer.is_err() {
crates/pull/src/ssm.rs:1950 (F) // Release the reader blocked in open(2) before failing.
crates/pull/src/ssm.rs:1951 (F) let _writer = std::fs::OpenOptions::new().write(true).open(&path);
crates/pull/src/ssm.rs:1952 (F) }
crates/pull/src/ssm.rs:1953 (F) panic!("a FIFO credentials file must refuse promptly, got {answer:?}")
crates/pull/src/ssm.rs:1988 (F) panic!("one byte past the bound must be refused")
crates/pull/src/ssm.rs:1998 (F) panic!("a file that is not text holds no profile")
crates/pull/src/ssm.rs:2050 (F) panic!("one byte past the cap must be refused")
crates/pull/src/ssm.rs:2057 (F) panic!("a body cut short is not an answer")
crates/pull/src/ssm.rs:1387:45 False (F) FIXED.contains(name) || sent.contains(name),
crates/pull/src/ssm.rs:1478:17 False (F) let Err(SsmError { detail, kind }) = foreign.authorization(&identity()) else {
crates/pull/src/ssm.rs:1507:13 False (F) let Err(SsmError { detail, .. }) = bad.authorization(&identity()) else {
crates/pull/src/ssm.rs:1549:17 False (F) let Err(SsmError { detail, .. }) = value_of(wrong) else {
crates/pull/src/ssm.rs:1554:13 False (F) let Err(SsmError { detail, .. }) = value_of("not json") else {
crates/pull/src/ssm.rs:1656:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "nope") else {
crates/pull/src/ssm.rs:1666:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
crates/pull/src/ssm.rs:1692:17 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&path, "default")
crates/pull/src/ssm.rs:1717:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_lookup(env(&[
crates/pull/src/ssm.rs:1767:17 False (F) let Err(SsmError { detail, .. }) = found else {
crates/pull/src/ssm.rs:1771:17 False (F) detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
crates/pull/src/ssm.rs:1771:57 False (F) detail.contains("AWS_ACCESS_KEY_ID") && detail.contains("AWS_SECRET_ACCESS_KEY"),
crates/pull/src/ssm.rs:1804:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&missing, "default")
crates/pull/src/ssm.rs:1835:17 False (F) !value.contains("Put") && !value.contains("Delete"),
crates/pull/src/ssm.rs:1835:43 False (F) !value.contains("Put") && !value.contains("Delete"),
crates/pull/src/ssm.rs:1857:13 False (F) let Err(SsmError { detail, kind }) = get_parameter(
crates/pull/src/ssm.rs:1884:17 False (F) let Err(SsmError { detail, .. }) = refuse_non_unicode(read) else {
crates/pull/src/ssm.rs:1907:13 False (F) !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
crates/pull/src/ssm.rs:1907:50 False (F) !shown.contains("PARAMETER-NAME") && !shown.contains("TOKEN-VALUE"),
crates/pull/src/ssm.rs:1911:13 False (F) shown.contains("ap-south-1") && shown.contains("<redacted>"),
crates/pull/src/ssm.rs:1911:45 False (F) shown.contains("ap-south-1") && shown.contains("<redacted>"),
crates/pull/src/ssm.rs:1948:13 False (F) let Ok(Err(SsmError { detail, kind })) = answer else {
crates/pull/src/ssm.rs:1986:13 False (F) let Err(SsmError { detail, kind }) = AwsIdentity::from_credentials_file(&over, "default")
crates/pull/src/ssm.rs:1996:13 False (F) let Err(SsmError { detail, .. }) = AwsIdentity::from_credentials_file(&binary, "default")
crates/pull/src/ssm.rs:2049:13 False (F) let Err(SsmError { detail, kind }) = answered("", vec![b'a'; MAX_ANSWER_BYTES + 1]) else {
crates/pull/src/ssm.rs:2056:13 False (F) let Err(SsmError { detail, .. }) = answered("Content-Length: 4\r\n", b"ab".to_vec()) else {
crates/pull/src/universe.rs:798 (F) master.len(),
crates/pull/src/universe.rs:799 (F) index.len(),
crates/pull/src/universe.rs:800 (F) index.capacity()
crates/pull/src/upkeep_tests.rs:372 (F) lines[0]
crates/pull/src/upkeep_tests.rs:380 (F) lines[4]
crates/pull/src/upkeep_tests.rs:388 (F) lines[6]
crates/pull/src/upkeep_tests.rs:398 (F) lines[6]
crates/pull/src/upkeep_tests.rs:472:9 False (F) why.contains("space") && why.contains("nothing was written"),
crates/pull/src/upkeep_tests.rs:472:34 False (F) why.contains("space") && why.contains("nothing was written"),
crates/pull/src/upkeep_tests.rs:521:9 False (F) why.contains("another srt upkeep run of gdfl holds")
crates/pull/src/upkeep_tests.rs:522:16 False (F) && why.contains("one run of a kind at a time"),
crates/pull/src/upkeep_tests.rs:1001:47 False (F) why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"),
crates/pull/src/upkeep_tests.rs:1042:9 False (F) why.starts_with(&format!("{}: ", gone.join("audit").display()))
crates/pull/src/upkeep_tests.rs:1043:16 False (F) && why.ends_with("; nothing was written"),
crates/pull/src/upkeep_tests.rs:1399:9 False (F) why.starts_with("NSE/FNO/NIFTY/2019-03-28-1160000-CE names nothing under bars/gdfl: ")
crates/pull/src/upkeep_tests.rs:1400:16 False (F) && why.ends_with("2019-03-28-1160000-CE is absent"),
crates/pull/src/upkeep_tests.rs:1517:9 False (F) why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
crates/pull/src/upkeep_tests.rs:1517:44 False (F) why.starts_with("cannot read ") && why.contains("bars/gdfl/MCX/INDEX: "),
crates/pull/src/upkeep_tests.rs:1553:9 False (F) lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
crates/pull/src/upkeep_tests.rs:1553:42 False (F) lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
crates/pull/src/upkeep_tests.rs:1553:72 False (F) lacks.has(Kind::MonthSummary) && !lacks.has(Kind::DayIndex) && !lacks.has(Kind::SortIndex)
crates/pull/src/upkeep_tests.rs:1555:13 False (F) assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
crates/pull/src/upkeep_tests.rs:1555:57 False (F) assert!(!Lacks::of(Kind::ALL).has(Kind::Recount) && !Lacks::of(Kind::ALL).has(Kind::Rekey));
crates/pull/src/upkeep_tests.rs:1941:9 False (F) text.contains(
crates/pull/src/upkeep_tests.rs:1944:14 False (F) ) && text.contains("end 300 how=stopped n=2 ok=2 of=6 t="),
crates/pull/src/upkeep_tests.rs:2085:9 False (F) why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
crates/pull/src/upkeep_tests.rs:2085:45 False (F) why.starts_with("no run 77 in ") && why.ends_with("nothing was written"),
crates/pull/src/vendor.rs:6890 (F) other => panic!("{rung} sits on {other:?}, which the store cannot file"),
crates/pull/src/vendor.rs:7011 (F) _ => false,
crates/pull/src/vendor.rs:7082 (F) feed.display()
crates/pull/src/vendor.rs:7090 (F) feed.display()
crates/pull/src/vendor.rs:7096 (F) feed.display()
crates/pull/src/vendor.rs:7121 (F) feed.display()
crates/pull/src/vendor.rs:7159 (F) feed.display()
crates/pull/src/vendor.rs:7199 (F) feed.display()
crates/pull/src/vendor.rs:7258 (F) feed.display()
crates/pull/src/vendor.rs:7263 (F) feed.display()
crates/pull/src/vendor.rs:7742 (F) panic!("Groww is an HTTP feed");
crates/pull/src/vendor.rs:7760 (F) panic!("Dhan is an HTTP feed");
crates/pull/src/vendor.rs:8163 (F) panic!(
crates/pull/src/vendor.rs:8167 (F) feed.display()
crates/pull/src/vendor.rs:8174 (F) feed.display()
crates/pull/src/vendor.rs:8214 (F) panic!("Dhan is an HTTP broker");
crates/pull/src/vendor.rs:8425 (F) spec.path_template()
crates/pull/src/vendor.rs:9133 (F) other.standing.label()
crates/pull/src/vendor.rs:9399 (F) panic!("Groww is an HTTP feed");
crates/pull/src/vendor.rs:7088:37 True (F) !(named_in_query && named_in_path),
crates/pull/src/vendor.rs:7293:17 False (F) assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
crates/pull/src/vendor.rs:7293:53 False (F) assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
crates/pull/src/vendor.rs:7317:17 False (F) assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
crates/pull/src/vendor.rs:7317:56 False (F) assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
crates/pull/src/vendor.rs:7415:13 False (F) index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
crates/pull/src/vendor.rs:7415:60 False (F) index.close_minute() == cash.close_minute() && cash.close_minute() < fno.close_minute(),
crates/pull/src/vendor.rs:7673:39 False (F) assert!(i == j || one != other, "offset {one} appears twice");
crates/pull/src/vendor.rs:7741:13 False (F) let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
crates/pull/src/vendor.rs:7759:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/vendor.rs:8132:17 False (F) !row.granularities.is_empty() && !row.segments.is_empty(),
crates/pull/src/vendor.rs:8132:50 False (F) !row.granularities.is_empty() && !row.segments.is_empty(),
crates/pull/src/vendor.rs:8213:13 False (F) let Transport::Http(dhan) = Feed::Dhan.descriptor().transport else {
crates/pull/src/vendor.rs:8596:13 False (F) fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
crates/pull/src/vendor.rs:8596:54 False (F) fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
crates/pull/src/vendor.rs:8715:13 False (F) !TRUEDATA_INDEX.columns.contains(&Column::Volume)
crates/pull/src/vendor.rs:8716:20 False (F) && !TRUEDATA_INDEX.columns.contains(&Column::OpenInterest),
crates/pull/src/vendor.rs:8981:48 False (F) row.contested.is_none() || !row.binds_because.is_empty(),
crates/pull/src/vendor.rs:9398:13 False (F) let Transport::Http(groww) = Feed::Groww.descriptor().transport else {
crates/pull/src/vendor.rs:9681:13 False (F) OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
crates/pull/src/vendor.rs:9681:54 False (F) OPERATOR_EXCLUSION.contains("D-3338") && OPERATOR_EXCLUSION.contains("6 Oct 2026"),
crates/pull/src/vendor.rs:9709:17 False (F) assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
crates/pull/src/vendor.rs:9709:54 False (F) assert!(archive.source.contains("D-2802") && archive.source.contains("D-3338"));
crates/pull/src/vendor.rs:9734:17 False (F) assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
crates/pull/src/vendor.rs:9734:53 False (F) assert!(window.source.contains("D-3338") && window.source.contains("82,079"));
crates/pull/src/vendor.rs:9760:17 False (F) assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
crates/pull/src/vendor.rs:9760:45 False (F) assert!(archive.is_well_shaped() && !backwards.is_well_shaped());
crates/pull/src/vendor.rs:9788:17 False (F) assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
crates/pull/src/vendor.rs:9788:51 False (F) assert!(cash.source.contains("D-3323") && cash.source.contains("101 of 208"));
crates/pull/src/vendor.rs:9889:17 False (F) assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
crates/pull/src/vendor.rs:9889:45 False (F) assert!(zerodha.is_well_shaped() && gdfl.is_well_shaped());
crates/pull/src/vendor.rs:9992:17 False (F) said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
crates/pull/src/vendor.rs:9992:54 False (F) said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
crates/pull/src/vendor.rs:9992:80 False (F) said.contains(&after.to_string()) && said.contains("15:15") && said.contains("§9"),
crates/pull/src/verdict_words.rs:256 (F) cause.code()
crates/pull/src/verdict_words.rs:184:21 False (F) !word.is_empty()
crates/pull/src/verdict_words.rs:185:28 False (F) && word
crates/pull/tests/alerts_seams.rs:59 (F) Put::Duplicate { ordinal } => panic!("a fresh key was a duplicate of #{ordinal}"),
crates/pull/tests/allocation.rs:173 (F) wide.len()
crates/pull/tests/allocation.rs:303 (F) panic!("a frame cut to {len} bytes was {:?}", frame_answer.0);
crates/pull/tests/allocation.rs:323 (F) full.len()
crates/pull/tests/allocation.rs:302:17 False (F) let Err(FrameError::Truncated { part }) = frame_answer.0 else {
crates/pull/tests/allocation.rs:321:13 False (F) in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
crates/pull/tests/allocation.rs:321:36 False (F) in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
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
crates/pull/tests/census.rs:332:9 False (F) total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
crates/pull/tests/census.rs:332:45 False (F) total >= done.bars_stored as u64 && total <= done.bars_stored as u64 * rungs() as u64,
crates/pull/tests/census.rs:492:9 False (F) total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
crates/pull/tests/census.rs:492:33 False (F) total >= BARS as u64 && total <= BARS as u64 * rungs() as u64,
crates/pull/tests/census.rs:655:13 False (F) why.contains("BSE") && why.contains("D-0017"),
crates/pull/tests/census.rs:655:36 False (F) why.contains("BSE") && why.contains("D-0017"),
crates/pull/tests/census.rs:775:9 False (F) why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
crates/pull/tests/census.rs:775:53 False (F) why.contains(&(ceiling + 1).to_string()) && why.contains(&ceiling.to_string()),
crates/pull/tests/census.rs:1028:9 False (F) total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
crates/pull/tests/census.rs:1028:38 False (F) total >= 10 + BARS as u64 && total <= 10 + BARS as u64 * rungs() as u64,
crates/pull/tests/census.rs:1130:9 False (F) done.counted == 0 && done.rows_read > 0,
crates/pull/tests/census.rs:1130:30 False (F) done.counted == 0 && done.rows_read > 0,
crates/pull/tests/census.rs:1371:9 False (F) total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
crates/pull/tests/census.rs:1371:41 False (F) total >= 4_321 + BARS as u64 && total <= 4_321 + BARS as u64 * rungs() as u64,
crates/pull/tests/census_day_pace.rs:129 (F) at + 1
crates/pull/tests/census_day_pace.rs:126:13 False (F) line.contains(&format!("\"read\":\"{read}\""))
crates/pull/tests/census_day_pace.rs:127:20 False (F) && line.contains(&format!("\"why\":\"{why}\"")),
crates/pull/tests/census_day_pace.rs:132:13 False (F) line.contains("\"load_us\":") && line.contains("\"total_us\":"),
crates/pull/tests/census_day_pace.rs:132:46 False (F) line.contains("\"load_us\":") && line.contains("\"total_us\":"),
crates/pull/tests/folder.rs:269 (F) panic!("a folder holding two days must report them: {reach:?}")
crates/pull/tests/folder.rs:459 (F) panic!("{reach:?}")
crates/pull/tests/folder.rs:925 (F) panic!("{:?}", census.reach)
crates/pull/tests/folder.rs:188:67 False (F) assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Dhan)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/tests/folder.rs:262:9 False (F) let Reach::Days {
crates/pull/tests/folder.rs:394:67 False (F) assert!(matches!(why, FolderError::NotAFolderFeed { feed } if feed == Feed::Groww)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/tests/folder.rs:418:64 False (F) assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/tests/folder.rs:455:9 False (F) let Reach::Days {
crates/pull/tests/folder.rs:751:64 False (F) assert!(matches!(why, FolderError::Untimed { secs, .. } if secs == i64::MIN)); -- checked by hand: a side of a matches! guard, of an .all() or of an .is_some_and()/.is_err_and() on one value, or of an .any() over a list of one entry, inside the assert's condition: that side makes the condition false and the assert fail
crates/pull/tests/folder.rs:921:9 False (F) let Reach::Days {
crates/pull/tests/refusal.rs:554 (F) feed.display()
crates/pull/tests/refusal.rs:588 (F) feed.display()
crates/pull/tests/refusal.rs:645 (F) other => panic!(
crates/pull/tests/refusal.rs:650 (F) other.master_columns().isin
crates/pull/tests/refusal.rs:658 (F) vendor.master_columns().isin
crates/pull/tests/refusal.rs:693 (F) panic!("Zerodha is an HTTP feed");
crates/pull/tests/refusal.rs:691:9 False (F) let pull::vendor::Transport::Http(spec) = pull::vendor::Feed::Zerodha.descriptor().transport
crates/pull/tests/request_gap_events.rs:145 (F) lines[0]
crates/pull/tests/request_gap_events.rs:138:9 False (F) lines[0].contains("\"count\":2")
crates/pull/tests/request_gap_events.rs:139:16 False (F) && lines[0].contains("\"first_day\":\"2025-07-01\"")
crates/pull/tests/request_gap_events.rs:140:16 False (F) && lines[0].contains("\"last_day\":\"2025-07-01\"")
crates/pull/tests/request_gap_events.rs:141:16 False (F) && lines[0].contains("\"month\":\"2025-07\"")
crates/pull/tests/request_gap_events.rs:142:16 False (F) && lines[0].contains("\"code\":\"vendor_empty\"")
crates/pull/tests/request_gap_events.rs:143:16 False (F) && lines[0].contains("\"feed\":\"truedata\""),
crates/pull/tests/sealed_month.rs:437:9 False (F) words.contains("repair revision 3") && words.contains("restates"),
crates/pull/tests/sealed_month.rs:437:48 False (F) words.contains("repair revision 3") && words.contains("restates"),
crates/pull/tests/sealed_month.rs:497:9 False (F) words.contains("sealed") && words.contains("restates"),
crates/pull/tests/sealed_month.rs:497:37 False (F) words.contains("sealed") && words.contains("restates"),
crates/pull/tests/token_identity.rs:374:15 True (F) !(TYPES.iter().any(|name| {
crates/pull/tests/token_identity.rs:378:19 True (F) }) || FUNCTIONS.iter().any(|(_, name)| {
crates/pull/tests/unit.rs:143 (F) other => panic!("expected Unreadable, got {other:?}"),
crates/pull/tests/unit.rs:766 (F) other => panic!("expected an unreadable file, got {other:?}"),
crates/pull/tests/unit.rs:1303 (F) other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
crates/pull/tests/unit.rs:1324 (F) other => panic!("expected a checksum refusal, got {other:?}"),
crates/pull/tests/unit.rs:1629 (F) manifest.entries()
crates/pull/tests/unit.rs:1691 (F) other => panic!("the recovery must name what it stepped over, got {other:?}"),
crates/pull/tests/unit.rs:1786 (F) other => panic!("expected the slot checksum to be reported, got {other:?}"),
crates/pull/tests/unit.rs:1869 (F) manifest.reserved()
crates/pull/tests/unit.rs:1874 (F) manifest.reserved()
crates/pull/tests/unit.rs:2011 (F) manifest.reserved()
crates/pull/tests/unit.rs:2069 (F) other => panic!("expected entry 1 to be named, got {other:?}"),
crates/pull/tests/unit.rs:2209 (F) other => panic!("expected a checksum refusal, got {other:?}"),
crates/pull/tests/unit.rs:2935 (F) Verdict::Admit => panic!("a drained bucket admits nothing at once"),
crates/pull/tests/unit.rs:3052 (F) governor.permitted(WindowSpan::Day)
crates/pull/tests/unit.rs:3384 (F) panic!("the eighth request must be refused");
crates/pull/tests/unit.rs:4125 (F) other => panic!("month {other}"),
crates/pull/tests/unit.rs:4891 (F) Some(other) => panic!("a bar inside its own window was dropped as {other}"),
crates/pull/tests/unit.rs:5359 (F) errors[0]
crates/pull/tests/unit.rs:6794 (F) other => panic!("prefix of {cut} bytes was not refused by name: {other:?}"),
crates/pull/tests/unit.rs:6814 (F) other => panic!("a corrupt header slot must be refused by name, got {other:?}"),
crates/pull/tests/unit.rs:7881 (F) other => panic!("byte {byte} bit {bit} was not caught: {other:?}"),
crates/pull/tests/unit.rs:7952 (F) layout.version()
crates/pull/tests/unit.rs:126:9 False (F) first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
crates/pull/tests/unit.rs:126:51 False (F) first.to_string_lossy().contains(&pid) && second.to_string_lossy().contains(&pid),
crates/pull/tests/unit.rs:801:13 False (F) assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
crates/pull/tests/unit.rs:801:49 False (F) assert!(rendered.contains("brutex-pull") && rendered.contains("not a regular file"));
crates/pull/tests/unit.rs:1016:13 False (F) assert!(rendered.contains("groww") && rendered.contains("fieldone"));
crates/pull/tests/unit.rs:1016:43 False (F) assert!(rendered.contains("groww") && rendered.contains("fieldone"));
crates/pull/tests/unit.rs:1017:13 False (F) assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
crates/pull/tests/unit.rs:1017:45 False (F) assert!(!rendered.contains("orgone") && !rendered.contains("vendorone"));
crates/pull/tests/unit.rs:1627:40 False (F) manifest.entries() == 0 || manifest.entries() == 1,
crates/pull/tests/unit.rs:3383:9 False (F) let Verdict::Deny { span, wait_micros } = governor.admit(0) else {
crates/pull/tests/unit.rs:5203:13 False (F) assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
crates/pull/tests/unit.rs:5203:46 False (F) assert!(month.contains(d(2024, 2, 1)) && month.contains(d(2024, 2, 29)));
crates/pull/tests/unit.rs:5204:13 False (F) assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
crates/pull/tests/unit.rs:5204:48 False (F) assert!(!month.contains(d(2024, 1, 31)) && !month.contains(d(2024, 3, 1)));
crates/pull/tests/unit.rs:5366:13 False (F) assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
crates/pull/tests/unit.rs:5366:53 False (F) assert!(errors[5].to_string().contains("29") && errors[5].to_string().contains("28"));
crates/pull/tests/unit.rs:5410:13 False (F) assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
crates/pull/tests/unit.rs:5410:48 False (F) assert!(YearMonth::new(1970, 1).is_ok() && Day::new(1970, 1, 1).is_ok());
crates/pull/tests/unit.rs:5411:13 False (F) assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
crates/pull/tests/unit.rs:5411:49 False (F) assert!(YearMonth::new(9999, 12).is_ok() && Day::new(9999, 12, 31).is_ok());
crates/pull/tests/unit.rs:5425:13 False (F) assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
crates/pull/tests/unit.rs:5425:46 False (F) assert!(Day::new(2024, 0, 1).is_err() && YearMonth::new(2024, 0).is_err());
crates/pull/tests/unit.rs:5426:13 False (F) assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
crates/pull/tests/unit.rs:5426:47 False (F) assert!(Day::new(2024, 13, 1).is_err() && YearMonth::new(2024, 13).is_err());
crates/pull/tests/unit.rs:5556:9 False (F) text.contains("volume 2") && text.contains("open 3"),
crates/pull/tests/unit.rs:5556:38 False (F) text.contains("volume 2") && text.contains("open 3"),
crates/pull/tests/unit.rs:5736:9 False (F) group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),
crates/pull/tests/unit.rs:5736:46 False (F) group.why.contains("row 0: open") && group.why.contains(&i64::MAX.to_string()),

L-BCP-3 rows (inside the bodies of #[ignore] real-data tests, owed to the operator's real-data run), counted by file:
crates/pull/src/cash_auction.rs 57
crates/pull/src/cash_session_cache.rs 168
crates/pull/src/gdfl_nfo_tests.rs 910
crates/pull/src/master_snapshots_tests.rs 127
crates/pull/src/reference_data_tests.rs 44

GAP rows, each with why it is left:
crates/pull/src/gdfl_import_tests.rs:5140:13 False GAP unborn == 0 || open_day, -- why: timing: the kill test meets an unborn month with its day open only when one of its timed SIGKILLs lands inside a month creation (covered in runs 6 and 8, not in runs 5, 7 and 9); its other sides are stopped on purpose since batch 22; a deterministic test needs a kill point inside a month creation (an operator call: the report, Exact changes outside G1, item 1)
crates/pull/src/gdfl_import_tests.rs:5140:28 True GAP unborn == 0 || open_day, -- why: timing: the kill test meets an unborn month with its day open only when one of its timed SIGKILLs lands inside a month creation (covered in runs 6 and 8, not in runs 5, 7 and 9); its other sides are stopped on purpose since batch 22; a deterministic test needs a kill point inside a month creation (an operator call: the report, Exact changes outside G1, item 1)
crates/pull/src/listing_tests.rs:490 GAP Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {} -- why: macOS only (cfg!(target_os = "macos")): owed to the Mac, never claimed
crates/pull/src/ssm.rs:966 GAP let status = answer.status(); -- why: after a send that reached AWS: no test can answer a signed read without AWS, a trusted TLS root or a second endpoint for the read, and adding one to a credential read is an operator call
crates/pull/src/ssm.rs:970 GAP let text = answer_within(&mut answer).await?; -- why: after a send that reached AWS: no test can answer a signed read without AWS, a trusted TLS root or a second endpoint for the read, and adding one to a credential read is an operator call
crates/pull/src/ssm.rs:971 GAP answer_of(region, status.as_u16(), &text) -- why: after a send that reached AWS: no test can answer a signed read without AWS, a trusted TLS root or a second endpoint for the read, and adding one to a credential read is an operator call
crates/pull/src/upkeep.rs:1581 GAP continue; -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep.rs:1200:16 False GAP && why.kind() == ErrorKind::NotFound -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep.rs:1580:20 True GAP if !scope.admits(2, &symbol) { -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep.rs:1676:20 False GAP if scope.prefix.len() <= 3 && into(walked, what, &path, true) { -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:460 GAP built.fetch_add(1, Ordering::Relaxed); -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:461 GAP clean(key, &at) -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:462 GAP }; -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1741 GAP built.fetch_add(1, Ordering::Relaxed); -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1742 GAP clean(key, &at) -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1743 GAP }; -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1001:9 False GAP why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"), -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1001:47 True GAP why.contains("not a regular file") || why.contains("upkeep.gdfl.srt.log"), -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
crates/pull/src/upkeep_tests.rs:1657:41 False GAP if text.starts_with("item ") && !self.refused.load(Ordering::Acquire) { -- why: MS1's file (SHARDS.md, MSUM-BACKFILL), not edited here
`````

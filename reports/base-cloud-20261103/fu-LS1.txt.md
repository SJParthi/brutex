# fu-LS1.txt.md

BASE-COV-LIBS-LS1 (crate `store`): the (F) and (U) items of recipe step 4 and every row that is neither, from the final Route B run of the continuation (5 Nov 2026). One fenced block.

`````text
FINAL: Route B (the kit's d3-route-b-cloud.sh, sha256 f032b525..., unchanged; d3-covset.mjs ecef3bdc...) on tip 4b3ee558a38fbc65f6c0dfcf59893cff750568c0 (the code tip of the 5 Nov continuation), set 'reading whole:store' (117 .rs files: the 113 of 3 Nov and 4 new test files), pinned rustc 1.99.0-nightly (1a98b1e13 2026-08-07) installed as nightly-2026-08-08, LLVM 23.1.0-rust-1.99.0-nightly, Linux x86_64 root host (4 cores), run under umask 000 so the profile folder takes the nobody re-runs' profiles (D-0995) and the fresh-process children's. Run 2026-11-05 08:47:41-09:05:56 IST (instrumented build 08:47:41-08:47:57 IST, 45 artifacts as its log reports them; tests 08:47:57-09:04:52 IST, 17 min: 1,064 passed, 0 failed, 8 ignored; 82 profiles merged). BEFORE (3 Nov final, tip fc9b489f0e3c): lines 51,094/52,629 (1,535 uncovered), branches 3,351/3,916 (565 uncovered). PROGRESS (this session, tip 38ee3db3d2fc): lines 51,474/52,972 (1,498 uncovered), branches 3,364/3,924 (560 uncovered).

RULES (recipe v4 step 4): (F) only the failing side of assert!/assert_eq!/assert_ne!/panic!/unreachable! inside a #[test] body or a cfg(test) helper only tests call (and a check module's own failing branch); (U) a region no input, file, folder, variable or direct call can reach on aarch64-apple-darwin, with its proof from the code. A (U) here is the session's proof for the Mac's check; every other uncovered row is a GAP. A row a direct call reaches is never (U): this continuation wrote a direct-call test for each one it found.

(U) ITEMS (81), each with its proof
  BRANCH crates/store/src/checksum_audit.rs:288:12 True: || layout.header_len() != HEADER_BYTES as u64
      (U) a header region of another size: header_len is slot_count * SLOT_STRIDE (layout.rs:363-365) and every Layout is built by Layout::declared or Layout::declare, which refuse a slot_count outside 2..=MAX_SLOT_COUNT with MAX_SLOT_COUNT = 2 (layout.rs:674-675, format.rs:95); so header_len is 2 * 16,384 = 32,768 = HEADER_BYTES for every Layout (checksum_audit_cover_tests.rs asserts it beside the flag and block cases).
  LINE   crates/store/src/file.rs:2891: )?;
      (U) block::seal refuses only by covered_len, which is layout.covered_byte_range(block, n_valid) itself (block.rs:332-334), already Ok for this block and counter at file.rs:2882-2885, or by a length mismatch, and bytes is slice_of(room, end - start), exactly the covered length (file.rs:2886, 6232-6249).
  LINE   crates/store/src/file.rs:3172: return Err(StoreError::Format {
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3173: path: self.bars_path.clone(),
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3174: source,
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3175: });
      (U) raw_answer's None arm: diagnose_overlap answers None only when every row at or before the header's last stamp matched records located from locate(first) and then either no row is past it (offered == len) or the walk ended at n_valid (file.rs:4446-4477). In the first case already_stored (file.rs:3145) compared the same rows at the same located index and answered Some; in the second the matched rows are the last `offered` records, which is exactly suffix_that_follows' tail anchor (file.rs:4533-4542), which then answered Some(suffix). survey refuses an empty batch first.
  LINE   crates/store/src/file.rs:3856: return Ok(None);
      (U) as file.rs:3855:21 False.
  LINE   crates/store/src/file.rs:3870: return Ok(None);
      (U) as file.rs:3869:13 False.
  LINE   crates/store/src/file.rs:3891: Err(other) => Err(index_refused(&tix.path, &other)),
      (U) extend's refusals are Outside, OffGrid, SharedSlot (matched by the arms above) and Unreadable, which only an Err stamp makes (time_index.rs:650); index_batch hands it Ok(row.stamp()) for every row (file.rs:3876).
  LINE   crates/store/src/file.rs:4013: Err(other) => return Err(index_refused(&tix.path, &other)),
      (U) as file.rs:3891: index_at_threshold hands extend Ok(row.stamp()) for every batch row (file.rs:3991).
  LINE   crates/store/src/file.rs:4316: path: self.bars_path.clone(),
      (U) a covered range longer than the handle's buffer: covered_byte_range's length is records_in_block * stride <= records_per_block * stride = block_len (layout.rs:543-551), and every geometry a BarFile resolves has block_len <= MAX_BLOCK_LEN (the compile-time assertion beside MAX_BLOCK_LEN that the doc comment at file.rs:4309-4313 names), so cache.bytes.get_mut(..span) is always Some.
  LINE   crates/store/src/file.rs:4317: source: FormatError::OffsetOverflow,
      (U) a covered range longer than the handle's buffer: covered_byte_range's length is records_in_block * stride <= records_per_block * stride = block_len (layout.rs:543-551), and every geometry a BarFile resolves has block_len <= MAX_BLOCK_LEN (the compile-time assertion beside MAX_BLOCK_LEN that the doc comment at file.rs:4309-4313 names), so cache.bytes.get_mut(..span) is always Some.
  LINE   crates/store/src/file.rs:4318: })?;
      (U) a covered range longer than the handle's buffer: covered_byte_range's length is records_in_block * stride <= records_per_block * stride = block_len (layout.rs:543-551), and every geometry a BarFile resolves has block_len <= MAX_BLOCK_LEN (the compile-time assertion beside MAX_BLOCK_LEN that the doc comment at file.rs:4309-4313 names), so cache.bytes.get_mut(..span) is always Some.
  BRANCH crates/store/src/file.rs:3855:21 False: let Some(tix) = self.tix.as_ref() else {
      (U) index_batch re-reads self.tix after reindex: it is inside the Some(tix) arm of file.rs:3836, self.tix is assigned only when the handle is built (file.rs:2758-2762; file_sealed.rs:161) and nothing sets it to None (set_tix_state, file.rs:3761-3764, writes the state inside the Some), so it is Some here.
  BRANCH crates/store/src/file.rs:3869:13 False: let Some(tix) = self.tix.as_ref() else {
      (U) as file.rs:3855:21 False: self.tix was Some at file.rs:3836 and nothing between sets it to None.
  BRANCH crates/store/src/file.rs:4189:36 True: if cache.block != block || cache.n_valid != n_valid {
      (U) read_verified keyed by the counter alone: self.header moves only at file.rs:3442 in append, and every append of a month with a sidecar (the only kind read_verified serves, file.rs:4163-4165) clears the cache first in seal_committed (file.rs:2836-2839, called at file.rs:3420); the post-commit sort and summary steps run inside untouched (sort_index_writer.rs:192-206), which puts the cache back. So cache.block == block implies cache.n_valid == n_valid.
  BRANCH crates/store/src/file.rs:5666:42 True: BatchFlush::OncePerDevice if O_SYNC.is_none() => return Ok(false),
      (U) O_SYNC is the const Some(open_flags::O_SYNC) on macOS and on Linux/Android x86_64/aarch64 (file.rs:5568-5576); the None definition compiles only on other targets, so O_SYNC.is_none() is false on aarch64-apple-darwin.
  LINE   crates/store/src/file_sealed.rs:136: return Err(StoreError::SealedRefused {
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:137: path: at,
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:138: reason: "block_rows",
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:139: detail: format!(
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:140: "the head declares {} rows a frame and a handle caches at most {}",
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:141: head.block_rows,
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:142: CACHED_BLOCK_LEN / RECORD_LEN
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:143: ),
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:144: });
      (U) as file_sealed.rs:135.
  LINE   crates/store/src/file_sealed.rs:231: )?;
      (U) offset_of refused inside fill_sealed: fill_sealed reaches line 229 only after sealed.read_block accepted block < n_blocks (sealed.rs:2203); n_blocks is a u32 (sealed.rs:1172) pinned to ceil(n_valid / block_rows) (sealed.rs:1240-1244) with block_rows <= 256, so block * rows < 2^40 and Layout::offset_of overflows only past (2^64 - header) / 56 (layout.rs:410-418).
  LINE   crates/store/src/file_sealed.rs:236: path: self.bars_path.clone(),
      (U) a decoded frame longer than the cache: read_block(block) decodes one frame of rows_in(block) <= block_rows <= 256 bars (sealed.rs:993-997, 2202-2213), and the cache has CACHED_BLOCK_LEN / RECORD_LEN = 14,336 / 56 = 256 slots (file.rs:302), so slots.next() never ends first.
  LINE   crates/store/src/file_sealed.rs:237: source: FormatError::OffsetOverflow,
      (U) as file_sealed.rs:236.
  LINE   crates/store/src/file_sealed.rs:238: })?;
      (U) as file_sealed.rs:236.
  LINE   crates/store/src/file_sealed.rs:359: }
      (U) the close of file_sealed.rs:354-355's arms; reached only through their else sides, which are (U).
  LINE   crates/store/src/file_sealed.rs:372: continue;
      (U) as file_sealed.rs:371.
  LINE   crates/store/src/file_sealed.rs:418: return Ok(None);
      (U) as file_sealed.rs:417.
  BRANCH crates/store/src/file_sealed.rs:135:12 True: if usize::from(head.block_rows).saturating_mul(RECORD_LEN) > CACHED_BLOCK_LEN {
      (U) block_rows past the one-frame cache: every head reaching line 135 was accepted by SealedMonth::open (file_sealed.rs:129) -> Head::decode, which refuses block_rows == 0 || block_rows > BLOCK_ROWS (sealed.rs:1117-1119), and file.rs:304 asserts at compile time CACHED_BLOCK_LEN == BLOCK_ROWS * RECORD_LEN; so block_rows * RECORD_LEN <= CACHED_BLOCK_LEN always.
  BRANCH crates/store/src/file_sealed.rs:202:36 True: if cache.block != block || cache.n_valid != n_valid {
      (U) a sealed handle's cache keyed by the counter: the handle's header is set once by open_sealed_at (file_sealed.rs:146) and self.header is assigned elsewhere only at file.rs:3442 inside append, which a sealed handle leaves at file.rs:3226-3227 (sealed_offer) before any write; cache.n_valid is set only to header.n_valid after a fill (file_sealed.rs:207). So cache.block == block implies cache.n_valid == n_valid.
  BRANCH crates/store/src/file_sealed.rs:341:20 False: if let Some(count) = first.get_mut(at + 1) {
      (U) read_rows_at placement: each group value is an index into blocks (file_sealed.rs:328-333: an existing entry, or blocks.len() just before the push), so at + 1 <= blocks.len() < first.len() = blocks.len() + 1 (file_sealed.rs:338).
  BRANCH crates/store/src/file_sealed.rs:347:20 False: if let Some(start) = first.get_mut(at) {
      (U) read_rows_at placement: at runs over 1..first.len() (file_sealed.rs:345), so first.get_mut(at) is always Some.
  BRANCH crates/store/src/file_sealed.rs:354:20 False: if let Some(slot) = next_slot.get_mut(at) {
      (U) read_rows_at placement: next_slot is first.clone() (file_sealed.rs:351) and at < blocks.len() < next_slot.len().
  BRANCH crates/store/src/file_sealed.rs:355:24 False: if let Some(place) = placed.get_mut(*slot) {
      (U) read_rows_at placement: first is the prefix sum of the group sizes over rows.len() rows, so group g's slots run first[g] .. first[g + 1] - 1 <= rows.len() - 1 < placed.len() (file_sealed.rs:352).
  BRANCH crates/store/src/file_sealed.rs:371:21 False: let Some(&row) = rows.get(asked) else {
      (U) read_rows_at: placed holds the asked positions 0 .. rows.len() (file_sealed.rs:353-356), so rows.get(asked) is always Some.
  BRANCH crates/store/src/file_sealed.rs:379:24 False: if let Some(slot) = out.get_mut(asked) {
      (U) read_rows_at: out has rows.len() entries (file_sealed.rs:362) and asked < rows.len().
  BRANCH crates/store/src/file_sealed.rs:417:9 False: let Some(sealed) = file.sealed.as_ref() else {
      (U) audit_sealed: open_sealed_at's only Ok(Some) builds the handle with sealed: Some(month) (file_sealed.rs:146-164), so file.sealed is never None here.
  LINE   crates/store/src/flock.rs:335: || "an unnamed program".to_owned(),
      (U) the program name of a process started with an EMPTY argument vector: std::env::args_os().next() is None only then; std::process::Command always passes an argv[0] (arg0 can make it empty, never absent) and the workspace denies unsafe code, so no test can start such a process, and PROCESS is set once per process.
  BRANCH crates/store/src/flock.rs:560:24 False: if let Ok(modified) = meta.modified() {
      (U) Metadata::modified() on Unix is Ok(SystemTime::new(st_mtime, st_mtime_nsec)), an Err only for a nanosecond count outside 0..1e9, which no kernel returns; so the else side is never taken on macOS or Linux.
  BRANCH crates/store/src/flock.rs:563:24 False: if let Ok(accessed) = meta.accessed() {
      (U) as flock.rs:560:24 False, for accessed() and st_atime.
  LINE   crates/store/src/month_summary.rs:1087: }
      (U) the close of the if-let at :1085, counted on its else side; as month_summary.rs:1085:24 False.
  BRANCH crates/store/src/month_summary.rs:939:24 False: if let Some(words) = entry.seconds.as_deref() {
      (U) a held day without its seconds bitmap while seconds are kept: keeps_seconds() needs self.seconds (month_summary.rs:684); every Summary is built by new(), whose days are DayBits::empty(seconds) (:550), or by decode, which sets self.seconds (:1053) and then rebuilds every day as DayBits::empty(tracked) (:1069); DayBits::empty(true) holds Some (:217), and no code sets a day's seconds to None or flips self.seconds afterwards.
  BRANCH crates/store/src/month_summary.rs:1055:12 True: if at.short || at.at != payload.len() {
      (U) at.short at the end of decode: every short read of a day entry yields last = 0, which makes the entry invalid (refused at line 1076) unless first == 0 and bars > 0, and then that day is held and its minute bitmap's at.bits refuses itself ("a bitmap runs past the payload", line 1253) because the cursor is already past the end; at.bits refuses every short bitmap itself. So decode never reaches line 1055 with at.short set.
  BRANCH crates/store/src/month_summary.rs:1085:24 False: if let Some(words) = entry.seconds.as_deref_mut() {
      (U) as month_summary.rs:939:24 False: decode_days rebuilt every entry as DayBits::empty(tracked) at :1069, so with tracked every entry's seconds is Some at :1085.
  BRANCH crates/store/src/provenance.rs:1658:12 False: if let Some(dst) = out.get_mut(at..at.saturating_add(bytes.len())) {
      (U) put: every call passes a constant offset and a constant-width value inside the fixed-size image it writes (the record image, provenance.rs:373-402; the header image), so the range is always inside.
  BRANCH crates/store/src/provenance.rs:1666:12 False: if let Some(run) = bytes.get(at..at.saturating_add(N)) {
      (U) take: every call reads a constant offset and width inside a fixed-size record or header image (provenance.rs:416-430, 596-603), so the range is always inside.
  LINE   crates/store/src/repair.rs:851: rows: header.n_valid,
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  LINE   crates/store/src/repair.rs:852: ceiling,
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  LINE   crates/store/src/repair.rs:853: };
      (U) the RowLimit closure runs only when usize::try_from(header.n_valid) fails (repair.rs:854); usize is 64 bits on aarch64-apple-darwin, so a u64 always converts.
  LINE   crates/store/src/safe_open.rs:300: return Err(io(
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:301: "open",
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:302: io::Error::new(
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:303: ErrorKind::Unsupported,
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:304: "safe reads require verified macOS or Linux x86_64/aarch64 open flags",
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:305: ),
      (U) as safe_open.rs:299.
  LINE   crates/store/src/safe_open.rs:306: ));
      (U) as safe_open.rs:299.
  BRANCH crates/store/src/safe_open.rs:299:9 False: let Some(flags) = FLAGS else {
      (U) on every verified target: FLAGS is the const Some(O_NOFOLLOW_NONBLOCK) under cfg(any(macos, linux|android on x86_64|aarch64)) (safe_open.rs:231-238); the None arm compiles only on other targets.
  BRANCH crates/store/src/sealed.rs:1499:23 False: } else if let Some(range) = self.ranges.get_mut(5) {
      (U) self.ranges is a [_; 6] array (one range per column): get_mut(5) is always Some.
  BRANCH crates/store/src/sealed.rs:1525:16 False: if let Some(oi) = columns.get_mut(5) {
      (U) columns is [ColumnRange::default(); 6] (sealed.rs:1519): get_mut(5) is always Some.
  BRANCH crates/store/src/sealed.rs:2352:35 False: Some(previous) if block > 0 => entries.get(previous).map_or(0, |entry| entry.end),
      (U) last_before's short window: skip = lo - lo.saturating_sub(1) is 0 when lo = 0 and 1 otherwise (sealed.rs:2341-2343), index = skip + at and block = lo + at (:2349-2350); index >= 1 (Some(previous)) needs lo >= 1 or at >= 1, and either makes block >= 1, so the guard block > 0 holds whenever the arm is reached, for any lo, hi and ts.
  BRANCH crates/store/src/settings.rs:220:16 False: if let Some(slot) = self.values.get_mut(key.slot()) {
      (U) Settings::put: values is [Option<String>; KEYS] with KEYS = 11 (settings.rs:132, 209) and Key::slot maps the eleven keys to 0..=10 (settings.rs:187-200), so get_mut(key.slot()) is always Some.
  BRANCH crates/store/src/settings.rs:688:30 False: let percent_too = if unit.is_empty() && matches!(amount, 1..=95) {
      (U) a unit under the floor: every unit scales by at least 10^9 (settings.rs:661-665), MIN_FLOOR_BYTES is 10^9 (:635) and a zero amount is refused at :682-685, so with a unit bytes >= MIN_FLOOR_BYTES and line 687's branch is entered only with unit "" (settings_door_cover_tests.rs asserts 1GB is the floor itself).
  LINE   crates/store/src/verdict.rs:717: return None;
      (U) as verdict.rs:716:8 True.
  LINE   crates/store/src/verdict.rs:3924: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:3923:13 False.
  LINE   crates/store/src/verdict.rs:4076: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:4075:13 False.
  LINE   crates/store/src/verdict.rs:4137: return Err(VerdictError::ImageMissing { path });
      (U) as verdict.rs:4136:13 False.
  BRANCH crates/store/src/verdict.rs:716:8 True: if Vendor::ALL.get(at) != Some(&feed) {
      (U) core::vendor::Vendor declares Groww, Dhan, TrueData, Gdfl, Zerodha with default discriminants 0..=4 and ALL lists them in that order (core/src/vendor.rs:52-90, :94-104), so ALL.get(feed as usize) == Some(&feed) for every feed.
  BRANCH crates/store/src/verdict.rs:2278:16 False: if let Some(slot) = out.first.get_mut(ordinal) {
      (U) out.first is vec![0; planned.len()] (verdict.rs:2259) and ordinal enumerates planned (:2269), so get_mut(ordinal) is always Some. (Filed as test code by the 3 Nov classifier; it is production.)
  BRANCH crates/store/src/verdict.rs:2312:16 False: if let Some(count) = out.counts.get_mut(usize::from(code.code())) {
      (U) counts has CODE_SLOTS = 24 slots (verdict.rs:168, :2301) and Code's discriminants are 1..=18 (verdict.rs:249-289; code() is `self as u8`, :318), so get_mut(code) is always Some. (Filed as test code by the 3 Nov classifier.)
  BRANCH crates/store/src/verdict.rs:2478:16 False: if let Some(c) = counts.get_mut(usize::from(record.facts.code.code())) {
      (U) as verdict.rs:2312:16 False: the record's code was decoded by Code::from_code, which answers only known codes (verdict.rs:1776-1781). (Filed as test code by the 3 Nov classifier.)
  BRANCH crates/store/src/verdict.rs:3143:15 False: while let Some(parent) = at.parent() {
      (U) make_dirs' walk: dir is series.dir() joined with the year, and series.dir() is self.root joined with four fixed relative words (verdict.rs:805-812), so walking parent() from dir meets series.root, where the loop breaks (:3145-3146), before parent() can answer None.
  BRANCH crates/store/src/verdict.rs:3417:12 False: if let Some(entry) = days.get_mut(doy) {
      (U) day_of_year is a civil day's offset in its own year, 0..=365 (verdict.rs:870-874), and every day table holds YEAR_DAYS = 366 entries (verdict.rs:194; built at 2789, 3096, 3382), so get_mut(doy) is always Some.
  BRANCH crates/store/src/verdict.rs:3853:24 False: && let Some(oldest) = self
      (U) handle cache: the let runs only when self.handles.len() >= HANDLE_CACHE >= 1, so min_by_key over a non-empty map is always Some.
  BRANCH crates/store/src/verdict.rs:3923:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) lookup calls self.prefix(.., &path, ..) first (verdict.rs:3921), which answers ImageMissing itself when handle() is None (:4017-4021) and otherwise leaves path in self.handles (handle inserts it, :3861); the second handle(&path) finds contains_key true (:3843) and answers Some without opening, and nothing between removes the entry (eviction happens only inside handle when a new path is inserted).
  BRANCH crates/store/src/verdict.rs:4075:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) as verdict.rs:3923:13 False: page calls self.prefix with the same path first.
  BRANCH crates/store/src/verdict.rs:4136:13 False: let Some((file, _)) = self.handle(&path, &mut cost)? else {
      (U) as verdict.rs:3923:13 False: the third reader calls self.prefix with the same path first.

PRODUCTION ROWS ONLY A HOST REFUSAL OR A RACE REACHES (61): no input, file, folder, variable or direct call reaches them on this host or the Mac, and no test seam of the crate reaches the call (the file.rs seam covers its own fault() and positional reads and writes; flock.rs's covers try_lock and unlock only). Each needs a decision: a cfg(test) seam at that call (the file.rs pattern), or a recorded limit.
  BRANCH crates/store/src/file_sealed.rs:425:21 False: Err(why) if why.kind() == ErrorKind::NotFound => Err(format!(
      RACE: the raw form's lstat refused otherwise than NotFound right after the sealed form in the same directory opened: the directory must stop being searchable between the two calls.
  LINE   crates/store/src/flock.rs:557: |_| std::fs::FileTimes::new(),
      HOST: fstat of the open lock file refused (try_lock_noted); no seam reaches File::metadata here.
  LINE   crates/store/src/flock.rs:627: Impatient::Host(why) => TryLockError::Error(why),
      HOST: the host refuses try_lock_shared otherwise than EWOULDBLOCK (try_lock_shared_patiently); flock.rs's lock seam answers Flock::try_lock only.
  LINE   crates/store/src/flock.rs:682: note_unnoted(self.path.as_ref(), &why);
      HOST: as flock.rs:681:20 True.
  LINE   crates/store/src/flock.rs:974: host @ Impatient::Host(_) => host,
      HOST: the host refuses the patient exclusive lock otherwise than EWOULDBLOCK (try_lock_patiently's attempt calls File::try_lock directly, past the seam).
  BRANCH crates/store/src/flock.rs:681:20 True: if let Err(why) = file.set_len(0).and_then(|()| file.set_times(times)) {
      HOST: clearing a holder's note refused (set_len(0) or set_times on the held lock file); no portable file refuses either on a descriptor this process wrote the note through (a device file does, but flock on one is not portable to macOS).
  LINE   crates/store/src/generation.rs:519: )?;
      HOST: the pointer record's write refused (ENOSPC, EIO) on the file this promoter just opened and locked.
  LINE   crates/store/src/provenance.rs:1282: )?;
      HOST: the new sidecar's header write refused on the file this landing just opened read-write and locked.
  LINE   crates/store/src/quarantine.rs:831: )?;
      HOST: the bucket read refused on an index open_index measured as exactly INDEX_LEN bytes (quarantine.rs:850-855): a short read needs the file cut between the fstat and the read (a race), any other refusal is the host's.
  LINE   crates/store/src/quarantine.rs:861: )?;
      HOST: as quarantine.rs:831, for the 64-byte header read.
  LINE   crates/store/src/quarantine.rs:1032: Err(std::fs::TryLockError::Error(_)) => false,
      HOST: the host refuses writer_active's shared flock otherwise than EWOULDBLOCK; Flock::try_lock_shared has no seam.
  LINE   crates/store/src/quarantine.rs:1186: )?;
      HOST: read_to_end refused on an index read_image measured as exactly INDEX_LEN bytes.
  LINE   crates/store/src/quarantine.rs:1188: return Err(QuarantineError::Torn {
      RACE: as quarantine.rs:1187:8 True.
  LINE   crates/store/src/quarantine.rs:1189: path: index.to_path_buf(),
      RACE: as quarantine.rs:1187:8 True.
  LINE   crates/store/src/quarantine.rs:1190: len: image.len() as u64,
      RACE: as quarantine.rs:1187:8 True.
  LINE   crates/store/src/quarantine.rs:1191: });
      RACE: as quarantine.rs:1187:8 True.
  LINE   crates/store/src/quarantine.rs:1467: Err(other) => Err(QuarantineError::Io {
      HOST: the host refuses the quarantine's patient writer lock otherwise than by a holder (Impatient::Host from try_lock_patiently, which has no seam).
  LINE   crates/store/src/quarantine.rs:1468: path,
      HOST: the host refuses the quarantine's patient writer lock otherwise than by a holder (Impatient::Host from try_lock_patiently, which has no seam).
  LINE   crates/store/src/quarantine.rs:1469: operation: "lock the quarantine",
      HOST: the host refuses the quarantine's patient writer lock otherwise than by a holder (Impatient::Host from try_lock_patiently, which has no seam).
  LINE   crates/store/src/quarantine.rs:1470: kind: impatient_kind(&other),
      HOST: the host refuses the quarantine's patient writer lock otherwise than by a holder (Impatient::Host from try_lock_patiently, which has no seam).
  LINE   crates/store/src/quarantine.rs:1471: }),
      HOST: the host refuses the quarantine's patient writer lock otherwise than by a holder (Impatient::Host from try_lock_patiently, which has no seam).
  LINE   crates/store/src/quarantine.rs:1759: )?;
      HOST: read_to_end of the ledger refused after safe_open::open_read opened it as a regular file.
  LINE   crates/store/src/quarantine.rs:2218: )?;
      HOST: the index image's write to its new staging file refused (ENOSPC, EIO).
  LINE   crates/store/src/quarantine.rs:2224: )?;
      HOST: the staging file's rename onto the index refused: a directory or other non-file at the index name is refused first by read_image (quarantine_tests.rs covers that), and the staging file lives in the same directory, so only a host refusal (EIO, EXDEV under a bind mount) remains.
  BRANCH crates/store/src/quarantine.rs:1187:8 True: if image.len() as u64 != INDEX_LEN {
      RACE: the index's length changed between read_image's fstat (checked == INDEX_LEN, quarantine.rs:1174-1179) and its read_to_end.
  BRANCH crates/store/src/quarantine.rs:1439:25 False: Err(why) if why.kind() != io::ErrorKind::AlreadyExists => {
      RACE: the audit directory was made by someone else between audit_dir's lstat (NotFound) and its create_dir (AlreadyExists).
  LINE   crates/store/src/repair.rs:1184: return Err(RepairError::ReadBack {
      RACE: the revision read back differs from what publish answered Ok for: publish returns Ok only after writing and verifying every bar (repair.rs:1372-1375) or reusing a revision verified bar by bar (:481-492), so the files must change under the reservation between publish and RevisionReader::open.
  LINE   crates/store/src/repair.rs:1185: index: index as u64,
      RACE: the revision read back differs from what publish answered Ok for: publish returns Ok only after writing and verifying every bar (repair.rs:1372-1375) or reusing a revision verified bar by bar (:481-492), so the files must change under the reservation between publish and RevisionReader::open.
  LINE   crates/store/src/repair.rs:1186: });
      RACE: the revision read back differs from what publish answered Ok for: publish returns Ok only after writing and verifying every bar (repair.rs:1372-1375) or reusing a revision verified bar by bar (:481-492), so the files must change under the reservation between publish and RevisionReader::open.
  LINE   crates/store/src/repair.rs:1342: )?;
      HOST: fsync of the new reservation file refused.
  LINE   crates/store/src/repair.rs:1374: return Err(RepairError::Conflict);
      RACE: as repair.rs:1373:12 True.
  LINE   crates/store/src/repair.rs:1392: )?;
      HOST: the receipt's write refused on the file create_new just made.
  LINE   crates/store/src/repair.rs:1398: )?;
      HOST: the receipt's fsync refused.
  LINE   crates/store/src/repair.rs:1458: Err(TryLockError::Error(error)) => io_at(Err(error), &path, "lock shared", false),
      HOST: the host refuses shared_lock's try_lock_shared otherwise than EWOULDBLOCK (no seam).
  BRANCH crates/store/src/repair.rs:1183:24 True: if reader.read_record(index as u64)? != *bar {
      RACE: as repair.rs:1184-1186.
  BRANCH crates/store/src/repair.rs:1373:12 True: if writer.read_record(index as u64)? != *offered {
      RACE: a record the revision writer just appended reads back different while it holds the revision's month lock exclusively: another actor must write the file past the lock.
  LINE   crates/store/src/safe_open.rs:107: Some(Self::Unknown)
      HOST: as safe_open.rs:104:19 False.
  BRANCH crates/store/src/safe_open.rs:104:19 False: } else if kind.is_block_device() {
      HOST: a file type that is none of regular, link, directory, FIFO, socket, character or block device: on macOS only a whiteout entry (S_IFWHT) of a union mount, which no test can create; std's FileType cannot be built directly.
  LINE   crates/store/src/settings.rs:886: return Err(fault(format!(
      RACE: as settings.rs:885:8 True.
  LINE   crates/store/src/settings.rs:887: "it grew past the {MAX_FILE_BYTES} bytes a settings file may hold while it was read"
      RACE: as settings.rs:885:8 True.
  LINE   crates/store/src/settings.rs:888: )));
      RACE: as settings.rs:885:8 True.
  LINE   crates/store/src/settings.rs:1227: _ => ErrorKind::AlreadyExists,
      RACE: staging::create refused the save's temp name otherwise than by I/O (a link, a non-file, a contended leftover): the name carries the pid, a sequence and the clock's nanoseconds, so only another actor creating that exact name in between reaches it.
  LINE   crates/store/src/settings.rs:1235: let _ = std::fs::remove_file(&temp);
      HOST: as settings.rs:1234:12 True.
  LINE   crates/store/src/settings.rs:1236: return Err(io_refusal(&temp, &why));
      HOST: as settings.rs:1234:12 True.
  LINE   crates/store/src/settings.rs:1321: path: fault.path,
      RACE: the re-list's refresh refused right after the save's own listing of the same directory succeeded (refresh errs only when list_numbers does, settings.rs:1041-1042): the directory must stop being readable between the two calls.
  LINE   crates/store/src/settings.rs:1322: kind: ErrorKind::Other,
      RACE: the re-list's refresh refused right after the save's own listing of the same directory succeeded (refresh errs only when list_numbers does, settings.rs:1041-1042): the directory must stop being readable between the two calls.
  LINE   crates/store/src/settings.rs:1323: why: format!("the settings directory cannot be read: {}", fault.why),
      RACE: the re-list's refresh refused right after the save's own listing of the same directory succeeded (refresh errs only when list_numbers does, settings.rs:1041-1042): the directory must stop being readable between the two calls.
  LINE   crates/store/src/settings.rs:1324: })?;
      RACE: the re-list's refresh refused right after the save's own listing of the same directory succeeded (refresh errs only when list_numbers does, settings.rs:1041-1042): the directory must stop being readable between the two calls.
  LINE   crates/store/src/settings.rs:1367: match linked {
      RACE: the hard link of two racing savers: covered only when the base suite's racing-writers tests interleave that way (covered at the 3 Nov baseline, not in its final run); a deterministic test needs a seam between the listing and the link.
  LINE   crates/store/src/settings.rs:1369: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1370: let _ = reader.refresh();
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1371: return Err(Refusal::Stale {
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1372: expected: change.expected,
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1373: newest: reader.highest().max(number),
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1374: });
      RACE: as settings.rs:1367.
  LINE   crates/store/src/settings.rs:1376: Err(why) => return Err(io_refusal(&target, &why)),
      RACE: as settings.rs:1367, for a link refused otherwise than AlreadyExists after the listing found the number free.
  LINE   crates/store/src/settings.rs:1445: Ok(_) => "it reads when read again, and the refresh did not step to it".to_owned(),
      RACE: a save landed and the reader's refresh did not step to it while the generation reads when read again: the file must appear readable between the refresh and read_one.
  BRANCH crates/store/src/settings.rs:885:8 True: if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
      RACE: a settings file grew past MAX_FILE_BYTES between the fstat that admitted it and the read.
  BRANCH crates/store/src/settings.rs:1234:12 True: if let Err(why) = written {
      HOST: write_all or sync_all of the staged temp file refused (ENOSPC, EIO).
  BRANCH crates/store/src/settings.rs:1369:21 True: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
      RACE: as settings.rs:1367.
  BRANCH crates/store/src/settings.rs:1369:21 False: Err(why) if why.kind() == ErrorKind::AlreadyExists => {
      RACE: as settings.rs:1376.

PRODUCTION GAPS (0): a production row with no test, no (U) proof and no host/race reason

IGNORED TESTS (1108 rows): bodies cargo test never runs, and helpers only they call; owed to the operator (L-BCL-3) or to a run made on purpose
  24 rows: crates/store/src/file.rs (the #[ignore]d full-volume test (BRUTEX_FULL_VOLUME, docs/06 L-PREC1-01))
  26 rows: crates/store/src/file_sealed_tests.rs (the #[ignore]d timing test read_rows_at_timing (a release-mode measurement))
  410 rows: crates/store/tests/sealed_codec_samples.rs (every test of the file is #[ignore]d: real months or a release-mode timing, L-BCL-3)
  95 rows: crates/store/tests/sealed_door.rs (the #[ignore]d private-root sealing run and the helpers only it calls (BRUTEX_SC3_ROOT, real months, L-BCL-3))
  186 rows: crates/store/tests/sealed_samples.rs (every test of the file is #[ignore]d: real months or a release-mode timing, L-BCL-3)
  189 rows: crates/store/tests/sealed_write_door.rs (every test of the file is #[ignore]d: real months or a release-mode timing, L-BCL-3)
  178 rows: crates/store/tests/tix_latency.rs (every test of the file is #[ignore]d: release-mode timings, L-BCL-3)

TEST-CODE GAPS (144): uncovered test code that is not a failing assert/panic arm by pattern: skip paths for a host without git or a FIFO peer, timeouts, fault-mock guards, a check module's failure-collection lines (a check module's own failing branch is (F) by step 4's second clause; each such row is for the Mac to confirm), macOS-only arms, helpers' rarely taken arms
  LINE   crates/store/src/checksum_audit_tests.rs:153: child.kill().expect("bounded timeout");
  LINE   crates/store/src/checksum_audit_tests.rs:154: child.wait().expect("reap child");
  LINE   crates/store/src/checksum_audit_tests.rs:155: return Err("strict checksum source open waited for a FIFO peer".into());
  BRANCH crates/store/src/checksum_audit_tests.rs:152:16 True: if started.elapsed() > std::time::Duration::from_secs(2) {
  BRANCH crates/store/src/emits.rs:311:12 False: if let Some(byte) = region.get_mut(16_384 + 56) {
  LINE   crates/store/src/file.rs:8676: continue;
  BRANCH crates/store/src/file.rs:8675:16 True: if &slot[..7] != b"BRUTEXB" {
  BRANCH crates/store/src/file.rs:9091:16 False: if let Some(head) = cache.bytes.get_mut(..8) {
  BRANCH crates/store/src/file.rs:10727:24 False: if *ts_micros == bar(10).ts_micros
  LINE   crates/store/src/file_write_door_tests.rs:68: continue;
  BRANCH crates/store/src/file_write_door_tests.rs:67:13 False: let Ok(entries) = fs::read_dir(&at) else {
  LINE   crates/store/src/flock_cover_tests.rs:87: }
  LINE   crates/store/src/flock_cover_tests.rs:88: self.took.push_str(s);
  LINE   crates/store/src/flock_cover_tests.rs:89: Ok(())
  BRANCH crates/store/src/flock_cover_tests.rs:85:12 False: if call == self.refuse {
  LINE   crates/store/src/flock_patient_tests.rs:323: )?;
  LINE   crates/store/src/flock_patient_tests.rs:364: Err(_) => refused += 1,
  LINE   crates/store/src/flock_patient_tests.rs:611: Err(_) => refused += 1,
  LINE   crates/store/src/flock_patient_tests.rs:640: unnamed += 1;
  BRANCH crates/store/src/flock_patient_tests.rs:639:16 True: if !named {
  LINE   crates/store/src/flock_tests.rs:699: matches!(
  LINE   crates/store/src/month_summary_tests.rs:294: let _release = fs::OpenOptions::new().write(true).open(pipe);
  LINE   crates/store/src/repair_tests.rs:700: }
  BRANCH crates/store/src/repair_tests.rs:691:12 False: if let Ok(entries) = fs::read_dir(dir) {
  LINE   crates/store/src/safe_open_tests.rs:67: tried.push(format!("{}: {why}", link.display()));
  LINE   crates/store/src/safe_open_tests.rs:68: continue;
  LINE   crates/store/src/safe_open_tests.rs:74: Err(why) => tried.push(format!("{}: {why}", link.display())),
  LINE   crates/store/src/safe_open_tests.rs:123: let _release = fs::OpenOptions::new().write(true).open(pipe);
  LINE   crates/store/src/safe_open_tests.rs:269: let refusal = open_read(&path).unwrap_err();
  BRANCH crates/store/src/safe_open_tests.rs:66:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  BRANCH crates/store/src/safe_open_tests.rs:203:12 False: if let Some(device) = a_block_device() {
  LINE   crates/store/src/sealed_tests.rs:499: Ok(())
  LINE   crates/store/src/sealed_tests.rs:939: fn flush(&mut self) -> io::Result<()> {
  LINE   crates/store/src/sealed_tests.rs:940: Ok(())
  LINE   crates/store/src/sealed_tests.rs:941: }
  BRANCH crates/store/src/sealed_tests.rs:237:12 False: if offset <= self.fail_at && self.fail_at < offset + buf.len() as u64 {
  LINE   crates/store/src/settings_tests.rs:261: Ok(())
  LINE   crates/store/src/settings_tests.rs:709: if let Some(pipe) = pipe {
  LINE   crates/store/src/settings_tests.rs:710: let _release = std::fs::OpenOptions::new().write(true).open(pipe);
  LINE   crates/store/src/settings_tests.rs:711: }
  LINE   crates/store/src/settings_tests.rs:1154: return None;
  BRANCH crates/store/src/settings_tests.rs:255:12 False: if s.get(Key::OpenFeed) == Some("dhan") {
  BRANCH crates/store/src/settings_tests.rs:708:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  BRANCH crates/store/src/settings_tests.rs:709:16 True: if let Some(pipe) = pipe {
  BRANCH crates/store/src/settings_tests.rs:709:16 False: if let Some(pipe) = pipe {
  BRANCH crates/store/src/settings_tests.rs:1153:20 True: if started.elapsed() > std::time::Duration::from_secs(10) {
  LINE   crates/store/src/verdict_tests.rs:461: let _release = fs::OpenOptions::new().write(true).open(pipe);
  LINE   crates/store/tests/catalog.rs:308: match made {
  LINE   crates/store/tests/catalog.rs:310: Err(why) if cfg!(target_os = "macos") && why.raw_os_error() == Some(92) => {
  LINE   crates/store/tests/catalog.rs:311: refused_by_the_volume += 1;
  LINE   crates/store/tests/catalog.rs:312: }
  LINE   crates/store/tests/catalog.rs:321: eprintln!(
  LINE   crates/store/tests/catalog.rs:322: "this volume refused all {refused_by_the_volume} non-UTF-8 names (os error 92); \
  LINE   crates/store/tests/catalog.rs:323: the walk is checked on what it holds and the five shapes by \
  LINE   crates/store/tests/catalog.rs:324: every_non_utf8_shape_is_counted_by_the_walks_own_step_on_every_host"
  LINE   crates/store/tests/catalog.rs:325: );
  LINE   crates/store/tests/catalog.rs:815: tried.push(format!("{}: {why}", link.display()));
  LINE   crates/store/tests/catalog.rs:816: continue;
  LINE   crates/store/tests/catalog.rs:822: Err(why) => tried.push(format!("{}: {why}", link.display())),
  BRANCH crates/store/tests/catalog.rs:68:12 False: if let Some(parent) = full.parent() {
  BRANCH crates/store/tests/catalog.rs:320:8 True: if refused_by_the_volume > 0 {
  BRANCH crates/store/tests/catalog.rs:814:16 True: if let Err(why) = std::os::unix::fs::symlink(dir, &link) {
  LINE   crates/store/tests/cited_commits.rs:289: println!("SKIPPING: `git` cannot be run here ({why}), so no commit can be resolved.");
  LINE   crates/store/tests/cited_commits.rs:290: return false;
  LINE   crates/store/tests/cited_commits.rs:568: faults.push(format!(
  LINE   crates/store/tests/cited_commits.rs:571: continue;
  LINE   crates/store/tests/cited_commits.rs:575: faults.push(format!(
  LINE   crates/store/tests/cited_commits.rs:576: "{commit} is cited in {} by a sentence that does not say \"{says}\": {}",
  LINE   crates/store/tests/cited_commits.rs:577: at.name, at.sentence
  LINE   crates/store/tests/cited_commits.rs:578: ));
  LINE   crates/store/tests/cited_commits.rs:579: }
  LINE   crates/store/tests/cited_commits.rs:602: let names: BTreeSet<&str> = citing.iter().map(|at| at.name.as_str()).collect();
  LINE   crates/store/tests/cited_commits.rs:603: faults.push(format!(
  LINE   crates/store/tests/cited_commits.rs:611: faults.push(format!("{commit} is listed as {off:?}, and it is on main"));
  LINE   crates/store/tests/cited_commits.rs:614: faults.push(format!(
  LINE   crates/store/tests/cited_commits.rs:615: "{commit} is listed as {off:?}, and it is an ancestor of HEAD"
  LINE   crates/store/tests/cited_commits.rs:616: ));
  LINE   crates/store/tests/cited_commits.rs:732: println!("SKIPPING: git cannot run the synthetic history fixture");
  LINE   crates/store/tests/cited_commits.rs:733: return;
  LINE   crates/store/tests/cited_commits.rs:820: println!("SKIPPING: git cannot run the synthetic history fixture");
  LINE   crates/store/tests/cited_commits.rs:821: return;
  LINE   crates/store/tests/cited_commits.rs:979: return;
  LINE   crates/store/tests/cited_commits.rs:997: wrong.push(format!("{module}.rs:{number}: {}", line.trim()));
  BRANCH crates/store/tests/cited_commits.rs:288:12 True: if let Err(why) = Command::new("git").arg("--version").output() {
  BRANCH crates/store/tests/cited_commits.rs:294:21 False: Err(why) if why.kind() == std::io::ErrorKind::NotFound
  BRANCH crates/store/tests/cited_commits.rs:306:12 False: if let Some(above) = root.parent() {
  BRANCH crates/store/tests/cited_commits.rs:567:13 False: let Some(citing) = cited.get(commit) else {
  BRANCH crates/store/tests/cited_commits.rs:601:16 True: if !is_ancestor_at(root, commit, main_ref) {
  BRANCH crates/store/tests/cited_commits.rs:610:12 True: if is_ancestor_at(root, commit, main_ref) {
  BRANCH crates/store/tests/cited_commits.rs:613:35 True: if off == Off::Neither && is_ancestor_at(root, commit, "HEAD") {
  BRANCH crates/store/tests/cited_commits.rs:676:8 False: if history_is_here() {
  BRANCH crates/store/tests/cited_commits.rs:731:8 True: if Command::new("git").arg("--version").output().is_err() {
  BRANCH crates/store/tests/cited_commits.rs:819:8 True: if Command::new("git").arg("--version").output().is_err() {
  BRANCH crates/store/tests/cited_commits.rs:898:39 False: } else if token.len() == 2 && token.starts_with('v') {
  BRANCH crates/store/tests/cited_commits.rs:903:19 False: } else if !token.is_empty() && token.bytes().all(|b| b.is_ascii_digit() || b == b'/') {
  BRANCH crates/store/tests/cited_commits.rs:978:8 True: if !history_is_here() {
  BRANCH crates/store/tests/cited_commits.rs:996:12 True: if !takes_a_lock(line) {
  LINE   crates/store/tests/cloud_mutants_store.rs:2385: return Err(std::io::ErrorKind::UnexpectedEof.into());
  LINE   crates/store/tests/cloud_mutants_store.rs:3172: return Err(std::io::Error::other("flushed alone"));
  LINE   crates/store/tests/cloud_mutants_store.rs:3253: _ => None,
  BRANCH crates/store/tests/cloud_mutants_store.rs:144:48 False: while (vanished < 20 || contended < 20) && Instant::now() < deadline {
  BRANCH crates/store/tests/cloud_mutants_store.rs:1686:20 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1686:37 False: if path == &year && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1720:24 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1720:38 False: if path == at && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:2349:20 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  BRANCH crates/store/tests/cloud_mutants_store.rs:2349:41 False: if *path == bar_path && *offset == 32_768 + 56 * 4
  BRANCH crates/store/tests/cloud_mutants_store.rs:2384:13 False: let Some(bytes) = self.bytes.get(at..at + buf.len()) else {
  BRANCH crates/store/tests/cloud_mutants_store.rs:3171:20 True: if since.elapsed() > std::time::Duration::from_secs(5) {
  LINE   crates/store/tests/docs.rs:226: }
  LINE   crates/store/tests/docs.rs:227: let entry = DECISIONS_DOC
  LINE   crates/store/tests/docs.rs:228: .split("\n### D-0790 ")
  LINE   crates/store/tests/docs.rs:229: .nth(1)
  LINE   crates/store/tests/docs.rs:230: .and_then(|rest| rest.split("\n### ").next())
  LINE   crates/store/tests/docs.rs:231: .map(flat)
  LINE   crates/store/tests/docs.rs:232: .expect("docs/05-decisions.md has a D-0790 entry");
  BRANCH crates/store/tests/docs.rs:224:8 False: if !flat(INVARIANTS_DOC).contains("no bench in this repository times a syscall") {
  LINE   crates/store/tests/fifo.rs:124: }
  LINE   crates/store/tests/fifo.rs:126: drop(fs::OpenOptions::new().write(true).open(release));
  LINE   crates/store/tests/fifo.rs:127: drop(worker.join());
  LINE   crates/store/tests/fifo.rs:213: return;
  BRANCH crates/store/tests/fifo.rs:121:12 False: if let Ok(outcome) = recv.recv_timeout(BOUND) {
  BRANCH crates/store/tests/fifo.rs:212:8 True: if !null.exists() {
  LINE   crates/store/tests/libm_key.rs:205: matches!(
  BRANCH crates/store/tests/misfiled.rs:125:16 True: let y = if month <= 2 { year - 1 } else { year };
  BRANCH crates/store/tests/misfiled.rs:129:26 False: let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i32::from(day) - 1;
  LINE   crates/store/tests/month_repair.rs:637: calls += 1;
  LINE   crates/store/tests/month_repair.rs:638: Ok(())
  LINE   crates/store/tests/month_repair.rs:639: };
  LINE   crates/store/tests/month_summary_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  LINE   crates/store/tests/month_summary_log.rs:75: other => format!("{other:?}"),
  LINE   crates/store/tests/open_flags.rs:176: offenders.push(format!("{}:{}", path.display(), number + 1));
  BRANCH crates/store/tests/open_flags.rs:175:16 True: if names_the_x86_bit(line) {
  LINE   crates/store/tests/slot_audit_sealed.rs:155: }
  LINE   crates/store/tests/slot_damaged_log.rs:93: Some(OwnedValue::Uint(value)) => *value,
  LINE   crates/store/tests/sort_index_log.rs:74: OwnedValue::Uint(n) => n.to_string(),
  LINE   crates/store/tests/sort_index_log.rs:75: other => format!("{other:?}"),
  BRANCH crates/store/tests/tix.rs:127:12 False: if let Some(last) = bars.last() {
  BRANCH crates/store/tests/tix_rebuilder.rs:154:12 False: if let Some(last) = bars.last() {
  BRANCH crates/store/tests/tix_small_month.rs:143:12 False: if let Some(last) = bars.last() {
  LINE   crates/store/tests/tix_warn_once.rs:77: other => format!("{other:?}"),
  LINE   crates/store/tests/tix_warn_once.rs:125: Some((_, OwnedValue::Uint(n))) => *n,
  BRANCH crates/store/tests/unit.rs:908:16 False: if one.magic()[..7] == other.magic()[..7]
  LINE   crates/store/tests/verdict.rs:588: }

(F) ITEMS (641), by pattern: the failing side of an assert macro or a panic!/unreachable! arm inside test code (a let-else or if whose block panics included); classified by pattern and spot-checked, not read one by one
  LINE   crates/store/src/catalog_tests.rs:268: panic!("a dangling `bars` link is refused");
  LINE   crates/store/src/catalog_tests.rs:282: panic!("an unsearchable root is refused, not empty: {locked:?}");
  LINE   crates/store/src/catalog_tests.rs:460: path.display()
  BRANCH crates/store/src/catalog_tests.rs:267:9 False: let Err(CatalogError::BarsUnreadable { because }) = bars_present(&bars) else {
  BRANCH crates/store/src/catalog_tests.rs:281:9 False: let Err(CatalogError::BarsUnreadable { because }) = locked else {
  BRANCH crates/store/src/catalog_tests.rs:314:13 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:314:33 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:320:13 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:320:32 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:320:50 False: assert!(!kind.is_file() && !kind.is_dir() && !kind.is_symlink());
  BRANCH crates/store/src/catalog_tests.rs:336:13 False: assert!(stack.is_empty() && out.held.is_empty());
  BRANCH crates/store/src/catalog_tests.rs:336:33 False: assert!(stack.is_empty() && out.held.is_empty());
  LINE   crates/store/src/emits.rs:835: found
  LINE   crates/store/src/emits.rs:836: .records
  LINE   crates/store/src/emits.rs:837: .iter()
  LINE   crates/store/src/emits.rs:838: .map(|line| (line.target.clone(), line.message.clone()))
  LINE   crates/store/src/emits.rs:839: .collect::<Vec<_>>()
  LINE   crates/store/src/emits.rs:848: panic!(
  LINE   crates/store/src/file.rs:6732: parent.display()
  LINE   crates/store/src/file.rs:6936: matches!(
  LINE   crates/store/src/file.rs:6956: level.display()
  LINE   crates/store/src/file.rs:7215: matches!(
  LINE   crates/store/src/file.rs:7269: other => panic!("the flush was refused by path: {other:?}"),
  LINE   crates/store/src/file.rs:7589: matches!(
  LINE   crates/store/src/file.rs:8353: reads.get()
  LINE   crates/store/src/file.rs:8692: other => panic!("a version-3 month without its flag must be refused, got {other:?}"),
  LINE   crates/store/src/file.rs:8910: matches!(
  LINE   crates/store/src/file.rs:8919: matches!(
  LINE   crates/store/src/file.rs:8929: matches!(
  LINE   crates/store/src/file.rs:9031: matches!(
  LINE   crates/store/src/file.rs:9074: matches!(
  LINE   crates/store/src/file.rs:9280: matches!(
  LINE   crates/store/src/file.rs:10225: matches!(
  LINE   crates/store/src/file.rs:10351: matches!(
  BRANCH crates/store/src/file.rs:6879:82 False: matches!(&refused, Err(StoreError::Denied { action: at, .. }) if *at == action),
  BRANCH crates/store/src/file.rs:9103:13 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  BRANCH crates/store/src/file.rs:9103:45 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  BRANCH crates/store/src/file.rs:9103:73 False: shown.contains("start: 100") && shown.contains("len: 8") && !shown.contains("bytes"),
  BRANCH crates/store/src/file_cover_tests.rs:91:9 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  BRANCH crates/store/src/file_cover_tests.rs:91:65 False: follows.contains("at 1927000001000000 follows them") && !follows.contains("restates"),
  BRANCH crates/store/src/file_cover_tests.rs:118:9 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  BRANCH crates/store/src/file_cover_tests.rs:118:42 False: whole.starts_with(&last.took) && last.took.len() < whole.len(),
  LINE   crates/store/src/file_fault_tests.rs:320: other => panic!("read {skip}: {other:?}"),
  LINE   crates/store/src/file_fault_tests.rs:657: Err(other) => panic!("read {skip}: {other:?}"),
  LINE   crates/store/src/file_fault_tests.rs:696: other => panic!("{}: {other:?}", looked.display()),
  LINE   crates/store/src/file_fault_tests.rs:869: other => panic!("{kind:?} measure {skip}: {other:?}"),
  BRANCH crates/store/src/file_fault_tests.rs:315:57 False: Err(StoreError::Denied { path, action }) if fired => {
  BRANCH crates/store/src/file_fault_tests.rs:334:9 False: routed >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:334:24 False: routed >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:650:21 False: reason.starts_with(&format!("{}: ", srt.display()))
  BRANCH crates/store/src/file_fault_tests.rs:651:28 False: && reason.contains("refused by the file test seam"),
  BRANCH crates/store/src/file_fault_tests.rs:661:9 False: built >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:661:23 False: built >= 1 && refused >= 1,
  BRANCH crates/store/src/file_fault_tests.rs:755:71 False: matches!(&answer.outcome, super::Resorted::Unsortable(why) if !why.is_empty()),
  BRANCH crates/store/src/file_fault_tests.rs:845:61 False: Err(StoreError::Denied { path, action }) if fired => {
  BRANCH crates/store/src/file_fault_tests.rs:879:26 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  BRANCH crates/store/src/file_fault_tests.rs:879:42 False: _ => assert!(refused >= 1 && barred == 0, "{kind:?}: {refused}, {barred}"),
  BRANCH crates/store/src/file_fault_tests.rs:912:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:912:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:949:13 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  BRANCH crates/store/src/file_fault_tests.rs:949:54 False: assert!(root.at(FileKind::TimeIndex).exists() && root.at(FileKind::SortIndex).exists());
  LINE   crates/store/src/file_index_cover_tests.rs:158: matches!(
  LINE   crates/store/src/file_index_cover_tests.rs:639: other => panic!("the leaf is refused by the last step: {other:?}"),
  BRANCH crates/store/src/file_index_cover_tests.rs:574:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/file_index_cover_tests.rs:574:36 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:67:9 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:67:51 False: fewer.contains("rows 899 against 900") && fewer.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:74:9 False: twice.contains("rows 900 against 900")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:75:16 False: && twice.contains("generation 2 against 1")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:76:16 False: && twice.contains("records BLAKE3 equal: true"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:90:9 False: shifted.contains("rows 900 against 900")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:91:16 False: && shifted.contains("generation 1 against 1")
  BRANCH crates/store/src/file_sealed_cover_tests.rs:92:16 False: && shifted.contains("records BLAKE3 equal: false"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:100:9 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:100:50 False: late.contains("rows 900 against 900") && late.contains("generation 1 against 1"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:166:13 False: refused.contains("other.sbar") && refused.contains("disagree"),
  BRANCH crates/store/src/file_sealed_cover_tests.rs:166:47 False: refused.contains("other.sbar") && refused.contains("disagree"),
  LINE   crates/store/src/file_sealed_tests.rs:321: other => panic!("expected Missing naming the .bin, got {other:?}"),
  LINE   crates/store/src/file_sealed_tests.rs:419: other => panic!("a damaged frame must be refused by name, got {other:?}"),
  LINE   crates/store/src/file_sealed_tests.rs:439: other => panic!("a damaged head must be refused, got {other:?}"),
  LINE   crates/store/src/file_sealed_tests.rs:798: matches!(
  LINE   crates/store/src/file_sealed_tests.rs:803: crate::provenance::read(&root.0, path)
  LINE   crates/store/src/file_sealed_tests.rs:885: page.len()
  LINE   crates/store/src/file_sealed_tests.rs:898: page.len()
  BRANCH crates/store/src/file_sealed_tests.rs:214:69 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  BRANCH crates/store/src/file_sealed_tests.rs:214:83 False: Err(StoreError::NotCommitted { index, n_valid }) if index == n && n_valid == n
  BRANCH crates/store/src/file_sealed_tests.rs:315:13 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  BRANCH crates/store/src/file_sealed_tests.rs:315:30 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  BRANCH crates/store/src/file_sealed_tests.rs:315:50 False: assert!(!bin.exists() && retired.exists() && sbar.exists());
  BRANCH crates/store/src/file_sealed_tests.rs:407:17 False: !reason.is_empty() && !detail.is_empty(),
  BRANCH crates/store/src/file_sealed_tests.rs:407:39 False: !reason.is_empty() && !detail.is_empty(),
  BRANCH crates/store/src/file_sealed_tests.rs:824:9 False: refused.starts_with("Err(") && refused.contains("sbar"),
  BRANCH crates/store/src/file_sealed_tests.rs:824:40 False: refused.starts_with("Err(") && refused.contains("sbar"),
  BRANCH crates/store/src/file_sealed_tests.rs:874:73 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  BRANCH crates/store/src/file_sealed_tests.rs:874:92 False: Err(StoreError::NotCommitted { index, n_valid }) if index == *named && n_valid == N
  LINE   crates/store/src/file_write_door_tests.rs:165: panic!("a new bar at a sealed month is `Sealed`, got {refused}");
  LINE   crates/store/src/file_write_door_tests.rs:322: _ => panic!("{case}: raw {raw:?}, sealed {sealed:?}"),
  LINE   crates/store/src/file_write_door_tests.rs:439: _ => panic!("{case} sealed={sealed}: raw {raw:?}, offered {got:?}"),
  LINE   crates/store/src/file_write_door_tests.rs:539: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:595: other => panic!("{n}: {other}"),
  LINE   crates/store/src/file_write_door_tests.rs:605: other => panic!("{n}: {other}"),
  LINE   crates/store/src/file_write_door_tests.rs:721: file.display()
  LINE   crates/store/src/file_write_door_tests.rs:749: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:765: other => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:809: Err(other) => panic!("{other}"),
  LINE   crates/store/src/file_write_door_tests.rs:844: matches!(
  LINE   crates/store/src/file_write_door_tests.rs:859: panic!("{case}: {repaired:?}");
  LINE   crates/store/src/file_write_door_tests.rs:902: matches!(
  LINE   crates/store/src/file_write_door_tests.rs:943: panic!("{repaired:?}");
  LINE   crates/store/src/file_write_door_tests.rs:983: matches!(
  LINE   crates/store/src/file_write_door_tests.rs:1027: panic!("{case}: {repaired:?}");
  LINE   crates/store/src/file_write_door_tests.rs:1067: panic!("{case}: {repaired:?}");
  BRANCH crates/store/src/file_write_door_tests.rs:156:13 False: let StoreError::Sealed {
  BRANCH crates/store/src/file_write_door_tests.rs:852:13 False: let MonthRepair::Repaired {
  BRANCH crates/store/src/file_write_door_tests.rs:942:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  BRANCH crates/store/src/file_write_door_tests.rs:1026:9 False: let MonthRepair::Repaired { to, skipped, .. } = repaired else {
  BRANCH crates/store/src/file_write_door_tests.rs:1060:9 False: let MonthRepair::Repaired {
  LINE   crates/store/src/flock_cover_tests.rs:116: matches!(
  BRANCH crates/store/src/flock_cover_tests.rs:68:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/flock_cover_tests.rs:68:36 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/flock_cover_tests.rs:133:17 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  BRANCH crates/store/src/flock_cover_tests.rs:133:43 False: assert!(entry.writer.is_none() && entry.queue.is_none());
  BRANCH crates/store/src/flock_cover_tests.rs:199:9 False: whole_sentence.starts_with("still held after waiting 21 ms of a 20 ms bound")
  BRANCH crates/store/src/flock_cover_tests.rs:200:16 False: && whole_sentence.ends_with("; pid 1 holds it"),
  LINE   crates/store/src/flock_patient_tests.rs:109: panic!("contention, not a host refusal: {refused}");
  LINE   crates/store/src/flock_patient_tests.rs:173: panic!("a host refusal: {got:?}");
  LINE   crates/store/src/flock_patient_tests.rs:195: panic!("{got:?}");
  LINE   crates/store/src/flock_patient_tests.rs:265: panic!("{got:?}");
  LINE   crates/store/src/flock_patient_tests.rs:439: panic!("it gave up: {refused:?}");
  LINE   crates/store/src/flock_patient_tests.rs:630: Err(TryLockError::Error(why)) => panic!("the probe was refused: {why}"),
  LINE   crates/store/src/flock_patient_tests.rs:687: matches!(
  BRANCH crates/store/src/flock_patient_tests.rs:101:9 False: let Impatient::Held {
  BRANCH crates/store/src/flock_patient_tests.rs:123:9 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  BRANCH crates/store/src/flock_patient_tests.rs:123:26 False: took >= bound && took < bound + MAX_PAUSE + SLACK,
  BRANCH crates/store/src/flock_patient_tests.rs:172:9 False: let Err(Impatient::Host(why)) = got else {
  BRANCH crates/store/src/flock_patient_tests.rs:191:9 False: let Err(Impatient::Held {
  BRANCH crates/store/src/flock_patient_tests.rs:219:62 False: matches!(got, Err(Impatient::Held { waited, .. }) if waited == Duration::from_millis(100))
  BRANCH crates/store/src/flock_patient_tests.rs:261:9 False: let Err(Impatient::Held {
  BRANCH crates/store/src/flock_patient_tests.rs:435:9 False: let Err(Impatient::Held {
  BRANCH crates/store/src/flock_patient_tests.rs:565:9 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  BRANCH crates/store/src/flock_patient_tests.rs:565:51 False: note.contains("thread ag06-successor") && note.contains("doing the second"),
  BRANCH crates/store/src/flock_patient_tests.rs:683:9 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  BRANCH crates/store/src/flock_patient_tests.rs:683:50 False: text.contains("held by this process") && text.contains("doing the journal index build"),
  LINE   crates/store/src/flock_tests.rs:213: panic!(
  LINE   crates/store/src/flock_tests.rs:244: panic!("{take:?} (release: {release}) over &File kept the lock — {why}")
  LINE   crates/store/src/flock_tests.rs:264: panic!("{take:?} (release: {release}) over &mut File kept the lock — {why}")
  LINE   crates/store/src/flock_tests.rs:283: panic!(
  LINE   crates/store/src/flock_tests.rs:320: panic!("{take:?} over {over}: the last guard's release left the lock held — {why}")
  LINE   crates/store/src/flock_tests.rs:924: matches!(
  BRANCH crates/store/src/flock_tests.rs:732:13 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  BRANCH crates/store/src/flock_tests.rs:732:38 False: assert!(nobody.contains(&pid) && nobody.contains(&lock), "{nobody}");
  BRANCH crates/store/src/flock_tests.rs:759:9 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  BRANCH crates/store/src/flock_tests.rs:759:42 False: said.contains("r3-08-holder") && !said.contains("nreadable"),
  BRANCH crates/store/src/flock_tests.rs:841:9 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  BRANCH crates/store/src/flock_tests.rs:841:51 False: said.starts_with(super::BEING_WRITTEN) && said.contains("pid 4242424"),
  BRANCH crates/store/src/flock_tests.rs:952:9 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  BRANCH crates/store/src/flock_tests.rs:952:66 False: note.contains(&format!("pid {}", std::process::id())) && !note.contains("9191919"),
  BRANCH crates/store/src/generation_cover_tests.rs:125:55 False: Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
  BRANCH crates/store/src/generation_cover_tests.rs:132:55 False: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  BRANCH crates/store/src/generation_cover_tests.rs:138:55 True: Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
  LINE   crates/store/src/generation_door_cover_tests.rs:99: panic!("revision 1 is served");
  LINE   crates/store/src/generation_door_cover_tests.rs:155: other => panic!("the pointer read is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:180: other => panic!("the linked pointer is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:212: other => panic!("the directory's creation is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:223: other => panic!("the pointer's open is refused: {other:?}"),
  LINE   crates/store/src/generation_door_cover_tests.rs:234: other => panic!("the pointer's lock is refused: {other:?}"),
  BRANCH crates/store/src/generation_door_cover_tests.rs:98:9 False: let Served::Revision(record0) = served(&root.0, month()).unwrap() else {
  BRANCH crates/store/src/generation_door_cover_tests.rs:136:59 False: Ok(Promoted::Promoted(Pointer { record, .. })) if record == MAX_RECORDS - 1
  LINE   crates/store/src/month_summary_decode_cover_tests.rs:171: other => panic!("the open is refused in the host's words: {other:?}"),
  LINE   crates/store/src/month_summary_tests.rs:295: panic!("a read of {} waited for a writer", pipe.display());
  LINE   crates/store/src/month_summary_tests.rs:791: other => panic!("{other:?}"),
  BRANCH crates/store/src/month_summary_tests.rs:278:52 False: matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
  BRANCH crates/store/src/month_summary_tests.rs:293:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  BRANCH crates/store/src/month_summary_tests.rs:320:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
  BRANCH crates/store/src/month_summary_tests.rs:340:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  BRANCH crates/store/src/month_summary_tests.rs:355:53 False: matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
  BRANCH crates/store/src/month_summary_tests.rs:377:13 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  BRANCH crates/store/src/month_summary_tests.rs:377:40 False: assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
  BRANCH crates/store/src/month_summary_tests.rs:461:32 False: Refused::Shape(why) if why.contains("flag")
  BRANCH crates/store/src/month_summary_tests.rs:467:60 False: matches!(shape(&unflagged), Refused::Shape(why) if why.contains("rung")),
  BRANCH crates/store/src/month_summary_tests.rs:480:59 False: assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
  BRANCH crates/store/src/month_summary_tests.rs:483:63 False: assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
  BRANCH crates/store/src/month_summary_tests.rs:488:61 False: assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
  BRANCH crates/store/src/month_summary_tests.rs:491:59 False: assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
  BRANCH crates/store/src/month_summary_tests.rs:655:13 False: assert!(round.tracks_carries() && !round.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:655:39 False: assert!(round.tracks_carries() && !round.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:658:13 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:658:42 False: assert!(!seconds.tracks_carries() && !seconds.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:739:37 False: Err(Refused::Shape(why)) if why.contains("not a held minute")
  BRANCH crates/store/src/month_summary_tests.rs:757:13 False: assert!(!read.tracks_carries() && read.wants_carries());
  BRANCH crates/store/src/month_summary_tests.rs:757:39 False: assert!(!read.tracks_carries() && read.wants_carries());
  BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:13 False: assert!(staging.is_dir() && !root.sum().exists());
  BRANCH crates/store/src/month_summary_writer_cover_tests.rs:250:33 False: assert!(staging.is_dir() && !root.sum().exists());
  LINE   crates/store/src/provenance_cover_tests.rs:166: other => panic!("the pointer refuses the read: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:186: other => panic!("the sealed form refuses the read: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:204: panic!("the raw sidecar holds a record");
  LINE   crates/store/src/provenance_cover_tests.rs:218: other => panic!("the sealed image's record is served: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:252: other => panic!("the open is refused by name: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:276: panic!("the batch's record is on disk");
  LINE   crates/store/src/provenance_cover_tests.rs:325: other => panic!("the undecodable head refuses the read: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:349: other => panic!("the lock refuses the landing: {other:?}"),
  LINE   crates/store/src/provenance_cover_tests.rs:394: other => panic!("the refused look refuses the flush: {other:?}"),
  BRANCH crates/store/src/provenance_cover_tests.rs:203:9 False: let Found::Present(raw) = read(&root.0, june).unwrap() else {
  BRANCH crates/store/src/provenance_cover_tests.rs:275:9 False: let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
  LINE   crates/store/src/quarantine_cover_tests.rs:635: Ok(Level::Current(_)) => panic!("a torn image is never current"),
  LINE   crates/store/src/quarantine_cover_tests.rs:636: Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
  LINE   crates/store/src/quarantine_cover_tests.rs:643: Ok(Level::Current(_)) => panic!("a damaged header is never current"),
  LINE   crates/store/src/quarantine_cover_tests.rs:644: Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
  BRANCH crates/store/src/quarantine_cover_tests.rs:449:65 False: matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
  LINE   crates/store/src/quarantine_tests.rs:145: panic!("expected a dropped generation, got {applied:?}");
  LINE   crates/store/src/quarantine_tests.rs:175: panic!("the month is labelled");
  LINE   crates/store/src/quarantine_tests.rs:264: panic!("the month is answered by name");
  LINE   crates/store/src/quarantine_tests.rs:551: other => panic!("expected the hold named, got {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:800: panic!("the month is still answered by name");
  LINE   crates/store/src/quarantine_tests.rs:892: other => panic!("{why}: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:1092: other => panic!("expected the quarantine to refuse by name, got {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:1268: other => panic!("the rerun indexes the drop: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:1995: other => panic!("{other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2348: other => panic!("the lock's open refuses the act: {other:?}"),
  LINE   crates/store/src/quarantine_tests.rs:2365: other => panic!("the index's open refuses the act: {other:?}"),
  BRANCH crates/store/src/quarantine_tests.rs:144:9 False: let Applied::Dropped { drop, .. } = applied else {
  BRANCH crates/store/src/quarantine_tests.rs:174:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("the probe answers") else {
  BRANCH crates/store/src/quarantine_tests.rs:210:63 False: assert!(matches!(generation, Served::Revision(pointer) if pointer.revision.ordinal() == 1));
  BRANCH crates/store/src/quarantine_tests.rs:263:9 False: let Probe::Quarantined(entry) = probe(&store.root, path).expect("probe") else {
  BRANCH crates/store/src/quarantine_tests.rs:342:66 False: matches!(torn, Err(QuarantineError::Torn { len, .. }) if len == INDEX_LEN - 17),
  BRANCH crates/store/src/quarantine_tests.rs:348:77 False: assert!(matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 1));
  BRANCH crates/store/src/quarantine_tests.rs:497:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  BRANCH crates/store/src/quarantine_tests.rs:497:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 0
  BRANCH crates/store/src/quarantine_tests.rs:508:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  BRANCH crates/store/src/quarantine_tests.rs:508:92 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.revision == 1 && e.bars == 2)
  BRANCH crates/store/src/quarantine_tests.rs:799:9 False: let Ok(Probe::Quarantined(entry)) = probe(&store.root, path) else {
  BRANCH crates/store/src/quarantine_tests.rs:855:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:855:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1071:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  BRANCH crates/store/src/quarantine_tests.rs:1072:16 False: && e.bars == 6
  BRANCH crates/store/src/quarantine_tests.rs:1073:16 False: && e.causes.said() == "before_listing and foreign_level"
  BRANCH crates/store/src/quarantine_tests.rs:1327:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1327:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1386:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  BRANCH crates/store/src/quarantine_tests.rs:1386:70 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth && e.bars == 2
  BRANCH crates/store/src/quarantine_tests.rs:1419:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth
  BRANCH crates/store/src/quarantine_tests.rs:1466:38 False: Ok(Probe::Quarantined(e)) if e.bars == 3
  BRANCH crates/store/src/quarantine_tests.rs:1533:9 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1533:29 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1584:9 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1584:29 False: refused.is_err() && words.contains("quarantined"),
  BRANCH crates/store/src/quarantine_tests.rs:1639:38 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1639:67 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1639:82 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 2 && e.revision == 1
  BRANCH crates/store/src/quarantine_tests.rs:1747:58 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  BRANCH crates/store/src/quarantine_tests.rs:1747:94 False: matches!(after, Ok(Probe::Quarantined(entry)) if entry.scope == Scope::WholeMonth && entry.bars == 2),
  BRANCH crates/store/src/quarantine_tests.rs:1865:77 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.bars == 2),
  BRANCH crates/store/src/quarantine_tests.rs:2261:42 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2261:71 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2261:86 False: Ok(Probe::Quarantined(e)) if e.scope == Scope::Dropped && e.bars == 3 && e.revision == 2
  BRANCH crates/store/src/quarantine_tests.rs:2316:73 False: matches!(probe(&store.root, path), Ok(Probe::Quarantined(e)) if e.scope == Scope::WholeMonth),
  BRANCH crates/store/src/repair_cover_tests.rs:62:41 False: text.contains("refused") || text.contains("drop names"),
  LINE   crates/store/src/repair_door_cover_tests.rs:221: other @ MonthRestate::NothingNew { .. } => {
  LINE   crates/store/src/repair_door_cover_tests.rs:222: panic!("the added bars are published: {other:?}")
  LINE   crates/store/src/repair_door_cover_tests.rs:246: other @ MonthRepair::NothingNew { .. } => {
  LINE   crates/store/src/repair_door_cover_tests.rs:247: panic!("the empty month takes the bars: {other:?}")
  LINE   crates/store/src/repair_door_cover_tests.rs:323: matches!(
  LINE   crates/store/src/repair_door_cover_tests.rs:393: other => panic!("the added bar is restated: {other:?}"),
  BRANCH crates/store/src/repair_door_cover_tests.rs:424:9 False: output.status.success() && stdout.contains("1 passed"),
  BRANCH crates/store/src/repair_door_cover_tests.rs:424:36 False: output.status.success() && stdout.contains("1 passed"),
  LINE   crates/store/src/repair_tests.rs:116: Err(TryLockError::Error(why)) => panic!("the probe could not ask: {why}"),
  LINE   crates/store/src/repair_tests.rs:630: timeframe.as_str()
  LINE   crates/store/src/repair_tests.rs:891: panic!("expected a restated generation, got {done:?}");
  BRANCH crates/store/src/repair_tests.rs:824:94 False: matches!(&held, Err(RepairError::Store(crate::file::StoreError::Locked { path })) if *path == lock),
  BRANCH crates/store/src/repair_tests.rs:882:9 False: let super::MonthRestate::Restated {
  LINE   crates/store/src/safe_open_tests.rs:77: panic!(
  LINE   crates/store/src/safe_open_tests.rs:79: path.display(),
  LINE   crates/store/src/safe_open_tests.rs:80: tried.join("; ")
  LINE   crates/store/src/safe_open_tests.rs:103: found.file_type()
  LINE   crates/store/src/safe_open_tests.rs:110: link.display()
  LINE   crates/store/src/safe_open_tests.rs:124: panic!("an open of {} waited for a writer", pipe.display());
  LINE   crates/store/src/safe_open_tests.rs:270: assert_eq!((refusal.word(), refusal.found()), ("io", None), "{refusal}");
  LINE   crates/store/src/safe_open_tests.rs:271: assert!(!refusal.is_missing());
  LINE   crates/store/src/safe_open_tests.rs:401: Err(other) => panic!("{symbol}: refused, but not by the link: {other}"),
  LINE   crates/store/src/safe_open_tests.rs:402: Ok(_) => panic!("{symbol}: a month was opened for writing through the link"),
  BRANCH crates/store/src/safe_open_tests.rs:95:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  BRANCH crates/store/src/safe_open_tests.rs:122:9 False: let Ok(answered) = answer.recv_timeout(Duration::from_secs(10)) else {
  BRANCH crates/store/src/safe_open_tests.rs:268:8 True: if fs::File::open(&path).is_err() {
  BRANCH crates/store/src/safe_open_tests.rs:298:16 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  BRANCH crates/store/src/safe_open_tests.rs:298:35 False: if *root == shadow && *bars == shadow.join(BARS_DIR)
  BRANCH crates/store/src/safe_open_tests.rs:299:20 False: && target.as_deref() == Some(real.join(BARS_DIR).as_path())),
  BRANCH crates/store/src/safe_open_tests.rs:459:77 False: matches!(&absent, ReadRefusal::Io { action: "open", source, .. } if source.kind() == std::io::ErrorKind::NotFound),
  LINE   crates/store/src/sealed_codec_tests.rs:81: frame.len()
  LINE   crates/store/src/sealed_codec_tests.rs:282: frame.len()
  LINE   crates/store/src/sealed_codec_tests.rs:439: largest[rows],
  LINE   crates/store/src/sealed_codec_tests.rs:440: max_frame_len(m)
  BRANCH crates/store/src/sealed_codec_tests.rs:1511:21 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  BRANCH crates/store/src/sealed_codec_tests.rs:1511:38 False: assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
  BRANCH crates/store/src/sealed_codec_tests.rs:1556:13 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  BRANCH crates/store/src/sealed_codec_tests.rs:1556:33 False: last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
  LINE   crates/store/src/sealed_tests.rs:159: Ok(value) => panic!("expected a refusal, got {value:?}"),
  LINE   crates/store/src/sealed_tests.rs:502: Ok(()) => panic!("byte {at} bit {bit} flipped and nothing refused"),
  LINE   crates/store/src/sealed_tests.rs:653: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:837: other => panic!("{field}: {other}"),
  LINE   crates/store/src/sealed_tests.rs:1170: packed.len(),
  LINE   crates/store/src/sealed_tests.rs:1171: image.len()
  LINE   crates/store/src/sealed_tests.rs:1192: other => panic!("{other:?}"),
  LINE   crates/store/src/sealed_tests.rs:1241: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1319: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1656: other => panic!("{field}: {other}"),
  LINE   crates/store/src/sealed_tests.rs:1673: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1689: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1774: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1802: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1891: other => panic!("{other}"),
  LINE   crates/store/src/sealed_tests.rs:1982: matches!(
  LINE   crates/store/src/sealed_tests.rs:2104: other => panic!("{other}"),
  BRANCH crates/store/src/sealed_tests.rs:696:27 False: found == 0 || bars[found as usize - 1].ts_micros < ts,
  BRANCH crates/store/src/sealed_tests.rs:1816:100 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::Table { index } if index == s)
  BRANCH crates/store/src/sealed_tests.rs:1822:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  BRANCH crates/store/src/sealed_tests.rs:1828:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s + 1)
  BRANCH crates/store/src/sealed_tests.rs:1836:105 False: matches!(refusal(open(path, bytes).first_at_or_after(ts)), SealedError::TableOrder { index } if index == s)
  BRANCH crates/store/src/settings_cover_tests.rs:198:57 False: Err(Refusal::Invalid { key: None, why }) if why.contains("1 to 240 bytes")
  LINE   crates/store/src/settings_door_cover_tests.rs:123: other => panic!("the listing refuses the save: {other:?}"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:92:9 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:92:47 False: with_mb.contains("is not a floor") && !with_mb.contains("percent"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:157:9 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  BRANCH crates/store/src/settings_door_cover_tests.rs:157:51 False: fault.why.contains("ermission denied") && !fault.why.contains("not a regular file"),
  LINE   crates/store/src/settings_tests.rs:152: other => panic!("{bad:?} for {key:?}: {other:?}"),
  LINE   crates/store/src/settings_tests.rs:228: matches!(
  LINE   crates/store/src/settings_tests.rs:414: reader.faults()
  LINE   crates/store/src/settings_tests.rs:580: reader.faults()
  LINE   crates/store/src/settings_tests.rs:651: reader.stopped()
  LINE   crates/store/src/settings_tests.rs:712: panic!("{what}: the settings read waited instead of naming a fault");
  LINE   crates/store/src/settings_tests.rs:990: matches!(
  BRANCH crates/store/src/settings_tests.rs:786:48 False: matches!(&listed[0], Err(f) if f.why.contains("not a regular file")),
  BRANCH crates/store/src/settings_tests.rs:910:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("cannot be read")),
  BRANCH crates/store/src/settings_tests.rs:956:60 False: matches!(&refused, Err(Refusal::Io { why, .. }) if why.contains("exhausted")),
  BRANCH crates/store/src/settings_tests.rs:1097:9 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  BRANCH crates/store/src/settings_tests.rs:1097:52 False: read_back.contains("did not read back") && read_back.contains("generation in force is 1"),
  BRANCH crates/store/src/settings_tests.rs:1187:13 True: a.superseded_by == Some(3) || a.in_force == Some(2),
  BRANCH crates/store/src/settings_tests.rs:1187:43 False: a.superseded_by == Some(3) || a.in_force == Some(2),
  BRANCH crates/store/src/small_month_tests.rs:152:52 False: Ok(Appended::Committed { n_valid, .. }) if n_valid == INDEX_MIN_ROWS + 1
  BRANCH crates/store/src/sort_index_cover_tests.rs:294:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  BRANCH crates/store/src/sort_index_cover_tests.rs:301:38 False: Err(Why::Unreadable(why)) if why.contains("is not the file that was walked")
  BRANCH crates/store/src/sort_index_cover_tests.rs:305:38 False: Err(Why::Unreadable(why)) if why.contains("is not a regular file")
  BRANCH crates/store/src/sort_index_cover_tests.rs:312:38 False: Err(Why::Unreadable(why)) if why.contains("is a symbolic link")
  BRANCH crates/store/src/sort_index_cover_tests.rs:318:38 False: Err(Why::Unreadable(why)) if why.starts_with(&under.display().to_string())
  BRANCH crates/store/src/sort_index_tests.rs:324:41 False: assert!(why.contains("kind") || why.contains("reserved"), "{why}");
  LINE   crates/store/src/staging_cover_tests.rs:56: Err(other) => panic!("the removal is refused by name: {other:?}"),
  LINE   crates/store/src/staging_cover_tests.rs:57: Ok(_) => panic!("the removal is refused by name, not staged"),
  LINE   crates/store/src/staging_tests.rs:421: within.display()
  BRANCH crates/store/src/staging_tests.rs:143:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:143:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:206:67 False: matches!(&refusal, StagingError::Contended { path: p } if *p == path),
  BRANCH crates/store/src/staging_tests.rs:238:16 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:238:64 False: if source.kind() == ErrorKind::PermissionDenied && *p == path),
  BRANCH crates/store/src/staging_tests.rs:297:78 False: matches!(&refusal, StagingError::Linked { links: 2, path: p } if *p == path),
  BRANCH crates/store/src/staging_tests.rs:334:88 False: matches!(&refusal, StagingError::NotRegular { found: Found::Symlink, path } if *path == link),
  BRANCH crates/store/src/staging_tests.rs:369:77 False: matches!(&refusal, StagingError::NotRegular { found: f, .. } if *f == found),
  BRANCH crates/store/src/time_index.rs:921:21 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  BRANCH crates/store/src/time_index.rs:921:41 False: assert!(entry_reads <= 1 && bar_reads <= 1);
  BRANCH crates/store/src/time_index.rs:1057:17 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1057:38 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1057:59 False: assert!(entry.occupied(0) && entry.occupied(2) && entry.occupied(63));
  BRANCH crates/store/src/time_index.rs:1058:17 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  BRANCH crates/store/src/time_index.rs:1058:39 False: assert!(!entry.occupied(1) && !entry.occupied(62));
  BRANCH crates/store/src/time_index.rs:1105:17 False: assert!(!daily.exact() && g.exact());
  BRANCH crates/store/src/time_index.rs:1105:35 False: assert!(!daily.exact() && g.exact());
  LINE   crates/store/src/verdict.rs:4219: other => panic!("not image_corrupt: {other:?}"),
  LINE   crates/store/src/verdict.rs:4227: other => panic!("not written_by_newer_build: {other:?}"),
  BRANCH crates/store/src/verdict.rs:4637:25 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  BRANCH crates/store/src/verdict.rs:4637:37 True: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  BRANCH crates/store/src/verdict.rs:4637:37 False: .find(|&i| (i < home || i >= home + super::MAX_PROBE) && mark_of(&good, i) == 0)
  BRANCH crates/store/src/verdict.rs:4974:24 False: .find(|&i| mark(&bytes, i) != 0 && tag(&bytes, i) == other_tag)
  LINE   crates/store/src/verdict_tests.rs:366: other => panic!("expected the host's refusal, got {other}"),
  LINE   crates/store/src/verdict_tests.rs:462: panic!("a verdict call waited on the pipe at {}", pipe.display());
  LINE   crates/store/src/verdict_tests.rs:568: other => panic!("expected the host's refusal, got {other}"),
  LINE   crates/store/src/verdict_tests.rs:813: other => panic!("not image_corrupt: {other:?}"),
  LINE   crates/store/src/verdict_tests.rs:879: other.dir().display()
  LINE   crates/store/src/verdict_tests.rs:951: panic!("the key's record: {:?}", found.found);
  BRANCH crates/store/src/verdict_tests.rs:460:9 False: let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
  BRANCH crates/store/src/verdict_tests.rs:950:9 False: let super::Found::Record { ordinal, record } = found.found else {
  LINE   crates/store/tests/catalog.rs:313: Err(why) => panic!("{} could not be made: {why}", full.display()),
  LINE   crates/store/tests/catalog.rs:825: panic!(
  LINE   crates/store/tests/catalog.rs:827: path.display(),
  LINE   crates/store/tests/catalog.rs:828: tried.join("; ")
  LINE   crates/store/tests/catalog.rs:852: found.file_type()
  LINE   crates/store/tests/catalog.rs:859: link.display()
  LINE   crates/store/tests/catalog.rs:875: path.display()
  BRANCH crates/store/tests/catalog.rs:317:9 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  BRANCH crates/store/tests/catalog.rs:317:39 True: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  BRANCH crates/store/tests/catalog.rs:317:39 False: refused_by_the_volume == 0 || refused_by_the_volume == paths.len() as u64,
  BRANCH crates/store/tests/catalog.rs:844:42 False: matches!(direct, Err(ref why) if why.kind() == std::io::ErrorKind::InvalidInput),
  LINE   crates/store/tests/cited_commits.rs:673: cited.len()
  LINE   crates/store/tests/cited_commits.rs:709: String::from_utf8_lossy(&out.stderr)
  LINE   crates/store/tests/cited_commits.rs:976: sites.len()
  BRANCH crates/store/tests/cited_commits.rs:371:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  BRANCH crates/store/tests/cited_commits.rs:372:16 False: && said.contains("not a git repository"),
  BRANCH crates/store/tests/cited_commits.rs:384:9 False: said.contains("holds a `.git`, and `git rev-parse --git-dir` failed there")
  BRANCH crates/store/tests/cited_commits.rs:385:16 False: && said.contains("not a git repository"),
  LINE   crates/store/tests/cloud_mutants_store.rs:367: other => panic!("{door}: {other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:371: panic!("a moved revision is audited");
  LINE   crates/store/tests/cloud_mutants_store.rs:446: other => panic!("{other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1073: other => panic!("not image_corrupt: {other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1098: panic!("ZZB is in the day");
  LINE   crates/store/tests/cloud_mutants_store.rs:1171: matches!(
  LINE   crates/store/tests/cloud_mutants_store.rs:1361: other => panic!("{other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1568: other => panic!("{other:?}"),
  LINE   crates/store/tests/cloud_mutants_store.rs:1925: panic!("{name} is in the day");
  LINE   crates/store/tests/cloud_mutants_store.rs:2876: writer.time_lookup()
  BRANCH crates/store/tests/cloud_mutants_store.rs:162:9 False: vanished >= 20 && contended >= 20,
  BRANCH crates/store/tests/cloud_mutants_store.rs:162:27 False: vanished >= 20 && contended >= 20,
  BRANCH crates/store/tests/cloud_mutants_store.rs:201:18 False: } if source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:370:9 False: let Err(audited) = BarFile::open_existing_audited(&root, month, SYMBOL, u64::MAX) else {
  BRANCH crates/store/tests/cloud_mutants_store.rs:374:9 False: audited.starts_with("repair revision 1 of ")
  BRANCH crates/store/tests/cloud_mutants_store.rs:375:16 False: && audited.ends_with(" is not the one its promotion recorded; not audited"),
  BRANCH crates/store/tests/cloud_mutants_store.rs:1097:9 False: let Some(Found::Record { ordinal, record }) = lookup(&mut Reader::new()).unwrap() else {
  BRANCH crates/store/tests/cloud_mutants_store.rs:1315:9 False: generation.contains("/nonexistent/cloud/2031-02.tix")
  BRANCH crates/store/tests/cloud_mutants_store.rs:1316:16 False: && generation.contains("revision 1 is not the one its promotion recorded"),
  BRANCH crates/store/tests/cloud_mutants_store.rs:1642:20 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1642:37 False: if path == &root && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:1823:20 False: if path == &lock_path && *tries == steps + 1
  BRANCH crates/store/tests/cloud_mutants_store.rs:1823:42 False: if path == &lock_path && *tries == steps + 1
  BRANCH crates/store/tests/cloud_mutants_store.rs:1923:13 False: let Some(Found::Record { ordinal, record }) = lookup.answer.map(|answer| answer.found)
  BRANCH crates/store/tests/cloud_mutants_store.rs:1986:20 False: if path.parent() == Some(year.as_path())
  BRANCH crates/store/tests/cloud_mutants_store.rs:1987:24 False: && path.extension().is_some_and(|ext| ext == "tmp")
  BRANCH crates/store/tests/cloud_mutants_store.rs:1988:24 False: && source.kind() == std::io::ErrorKind::PermissionDenied
  BRANCH crates/store/tests/cloud_mutants_store.rs:2256:94 False: TimeLookup::Bisection(Why::Unreadable(StoreError::IsADirectory { path, .. })) if *path == tix
  BRANCH crates/store/tests/cloud_mutants_store.rs:2636:20 False: if *path == bar_path
  BRANCH crates/store/tests/cloud_mutants_store.rs:2873:80 False: TimeLookup::Bisection(Why::SharedSlot { index: 10, ts_micros }) if ts_micros == second.ts_micros
  BRANCH crates/store/tests/cloud_mutants_store.rs:2962:20 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  BRANCH crates/store/tests/cloud_mutants_store.rs:2962:40 False: if *path == sidecar && format!("{kind:?}") == "FilesystemLoop"
  BRANCH crates/store/tests/cloud_mutants_store.rs:3073:76 False: StoreError::NotADirectory { path, action: Action::Measure } if *path == bars.to_path_buf(&other)
  BRANCH crates/store/tests/cloud_mutants_store.rs:3124:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  BRANCH crates/store/tests/cloud_mutants_store.rs:3357:59 False: matches!(&refusal, StoreError::Io { path, .. } if *path == month_dir),
  LINE   crates/store/tests/docs.rs:233: assert!(
  LINE   crates/store/tests/docs.rs:234: entry.contains(
  LINE   crates/store/tests/docs.rs:235: "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
  LINE   crates/store/tests/docs.rs:236: ) && entry.contains("this entry supersedes that sentence"),
  LINE   crates/store/tests/docs.rs:239: assert!(
  LINE   crates/store/tests/docs.rs:240: INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
  BRANCH crates/store/tests/docs.rs:217:9 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  BRANCH crates/store/tests/docs.rs:217:47 False: BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
  BRANCH crates/store/tests/docs.rs:234:9 True: entry.contains(
  BRANCH crates/store/tests/docs.rs:234:9 False: entry.contains(
  BRANCH crates/store/tests/docs.rs:236:14 True: ) && entry.contains("this entry supersedes that sentence"),
  BRANCH crates/store/tests/docs.rs:236:14 False: ) && entry.contains("this entry supersedes that sentence"),
  BRANCH crates/store/tests/docs.rs:253:9 False: FILE_RS.contains("if claimed > header.n_valid {")
  BRANCH crates/store/tests/docs.rs:254:16 False: && FILE_RS.contains("source: FormatError::CounterExceedsFile,"),
  LINE   crates/store/tests/durability.rs:381: panic!(
  LINE   crates/store/tests/fault.rs:376: panic!("bit {bit} of byte {byte} took the whole file down: {e}")
  BRANCH crates/store/tests/fault.rs:143:36 False: read.n_valid == 150 || read.n_valid == 175,
  BRANCH crates/store/tests/fault.rs:220:59 False: horizon == previous_commit.durable_through || horizon == commit.durable_through,
  LINE   crates/store/tests/fifo.rs:108: at.display()
  LINE   crates/store/tests/fifo.rs:128: panic!(
  LINE   crates/store/tests/fifo.rs:130: release.display()
  BRANCH crates/store/tests/fifo.rs:157:13 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  BRANCH crates/store/tests/fifo.rs:157:61 False: rendered.contains(&at.display().to_string()) && rendered.contains("not a regular file"),
  BRANCH crates/store/tests/fifo.rs:162:53 False: fs::symlink_metadata(&at).is_ok_and(|m| !m.is_file() && !m.is_dir()),
  LINE   crates/store/tests/generation.rs:165: panic!("{done:?}");
  LINE   crates/store/tests/generation.rs:217: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:238: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:331: matches!(
  LINE   crates/store/tests/generation.rs:367: panic!("repaired");
  LINE   crates/store/tests/generation.rs:388: panic!("{second:?}");
  LINE   crates/store/tests/generation.rs:475: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:517: other => panic!("{other:?}"),
  LINE   crates/store/tests/generation.rs:758: panic!("{repaired:?}");
  BRANCH crates/store/tests/generation.rs:153:9 False: let MonthRepair::Repaired {
  BRANCH crates/store/tests/generation.rs:179:57 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  BRANCH crates/store/tests/generation.rs:179:76 False: assert!(matches!(promoted, Promoted::Promoted(p) if p.previous == 0 && p.n_valid == 25));
  BRANCH crates/store/tests/generation.rs:364:9 False: let MonthRepair::Repaired { to, skipped, .. } =
  BRANCH crates/store/tests/generation.rs:384:9 False: let MonthRepair::Repaired {
  BRANCH crates/store/tests/generation.rs:670:13 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:670:42 True: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:670:42 False: assert!(!store.pointer().exists() || fs::read(store.pointer()).unwrap().is_empty());
  BRANCH crates/store/tests/generation.rs:694:74 False: matches!(&refused, Err(StoreError::Generation { reason, .. }) if reason.contains("cannot open it")),
  BRANCH crates/store/tests/generation.rs:749:9 False: let MonthRepair::Repaired {
  BRANCH crates/store/tests/geometry.rs:99:13 False: start >= block_start && end <= block_end,
  BRANCH crates/store/tests/geometry.rs:99:37 False: start >= block_start && end <= block_end,
  BRANCH crates/store/tests/geometry.rs:134:13 True: start < block_start || end > block_end
  LINE   crates/store/tests/libm_key.rs:1330: if mentions { "is not" } else { "is" }
  BRANCH crates/store/tests/libm_key.rs:428:9 False: decision < caveat && caveat < rejected,
  BRANCH crates/store/tests/libm_key.rs:428:30 False: decision < caveat && caveat < rejected,
  BRANCH crates/store/tests/libm_key.rs:1330:16 True: if mentions { "is not" } else { "is" }
  BRANCH crates/store/tests/libm_key.rs:1330:16 False: if mentions { "is not" } else { "is" }
  LINE   crates/store/tests/misfiled.rs:184: other => panic!("expected Misfiled, found {other:?}"),
  LINE   crates/store/tests/misfiled.rs:321: unborn.len()
  BRANCH crates/store/tests/misfiled.rs:217:9 False: said.contains("2019-11-28-1250000-CE")
  BRANCH crates/store/tests/misfiled.rs:218:16 False: && said.contains("its store path is not this month's")
  BRANCH crates/store/tests/misfiled.rs:219:16 False: && said.contains("DB-B65"),
  LINE   crates/store/tests/month_repair.rs:156: other => panic!("not a repair: {other:?}"),
  LINE   crates/store/tests/month_repair.rs:324: timeframe.as_str()
  LINE   crates/store/tests/month_repair.rs:856: panic!("{outcome:?}");
  LINE   crates/store/tests/month_repair.rs:918: left.keys()
  LINE   crates/store/tests/month_repair.rs:944: other => panic!("not a repair: {other:?}"),
  BRANCH crates/store/tests/month_repair.rs:606:13 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  BRANCH crates/store/tests/month_repair.rs:606:47 False: assert!(left.remove(pointer).is_some() && right.remove(pointer).is_some());
  BRANCH crates/store/tests/month_repair.rs:739:83 False: matches!(&behind, Err(RepairError::Store(StoreError::Locked { path })) if *path == pointer),
  BRANCH crates/store/tests/month_repair.rs:852:9 False: let Ok(MonthRepair::Repaired {
  BRANCH crates/store/tests/month_repair.rs:879:38 False: Served::Revision(pointer) if pointer.revision.ordinal() == 3
  LINE   crates/store/tests/provenance.rs:66: other => panic!("expected a sidecar, found {other:?}"),
  LINE   crates/store/tests/provenance.rs:161: Recorded::NothingAppended => panic!("expected a record"),
  LINE   crates/store/tests/provenance.rs:451: other => panic!("expected NotTheNewestCommit, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:464: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:476: other => panic!("expected WrongBarFile, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:531: other => panic!("expected OutOfOrder, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:639: other => panic!("expected Damaged, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:698: result => panic!("expected a {field} mismatch, got {result:?}"),
  LINE   crates/store/tests/provenance.rs:746: other => panic!("expected PastTheBars, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1200: other => panic!("expected {fault:?}, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1308: other => panic!("expected TooLarge, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1409: other => panic!("expected Record, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1611: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1617: other => panic!("expected NotABarPath, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1671: other => panic!("expected the directory sync refused, got {other:?}"),
  LINE   crates/store/tests/provenance.rs:1721: other => panic!("expected the open refused, got {other:?}"),
  BRANCH crates/store/tests/repair.rs:506:13 True: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  BRANCH crates/store/tests/repair.rs:506:41 False: assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
  LINE   crates/store/tests/sealed_codec_alloc.rs:115: panic!("a frame cut to {len} bytes left {frame_answer:?}");
  LINE   crates/store/tests/sealed_codec_alloc.rs:125: full.len()
  BRANCH crates/store/tests/sealed_codec_alloc.rs:114:13 False: let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
  BRANCH crates/store/tests/sealed_codec_alloc.rs:123:9 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:123:32 False: in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:332:13 False: !text.contains("by its checked rows and no more")
  BRANCH crates/store/tests/sealed_codec_alloc.rs:333:20 False: && !text.contains("checked rows times 56 bytes, once"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:341:13 False: text.contains("the largest of twice") && text.contains("and 4"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:341:54 False: text.contains("the largest of twice") && text.contains("and 4"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:347:13 False: text.contains("capacity 0") && text.contains("cleared"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:347:44 False: text.contains("capacity 0") && text.contains("cleared"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:357:13 False: !text.contains("random fields cost at most 57 bytes a bar")
  BRANCH crates/store/tests/sealed_codec_alloc.rs:358:20 False: && !text.contains("at most 57 bytes a bar, 1 more than raw"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:387:9 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  BRANCH crates/store/tests/sealed_codec_alloc.rs:387:52 False: SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
  BRANCH crates/store/tests/sealed_door.rs:153:17 True: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  BRANCH crates/store/tests/sealed_door.rs:153:31 False: assert!(at == rows || bars[at as usize].ts_micros >= probe);
  BRANCH crates/store/tests/sealed_door.rs:231:9 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  BRANCH crates/store/tests/sealed_door.rs:231:45 False: large.0 <= 3.0 * small.0 + 200.0 && large.1 <= 3.0 * small.1 + 200.0,
  LINE   crates/store/tests/slot_audit.rs:116: matches!(
  LINE   crates/store/tests/slot_audit.rs:137: panic!("v{version}: a damaged newest slot is a revert: {audit:?}");
  LINE   crates/store/tests/slot_audit.rs:277: matches!(
  LINE   crates/store/tests/slot_audit.rs:347: matches!(
  LINE   crates/store/tests/slot_audit.rs:410: matches!(
  LINE   crates/store/tests/slot_audit.rs:485: panic!("damaged");
  BRANCH crates/store/tests/slot_audit.rs:130:13 False: let MonthVerdict::Reverted {
  BRANCH crates/store/tests/slot_audit.rs:484:9 False: let SlotState::Damaged { reason, .. } = audit.slots[1] else {
  LINE   crates/store/tests/slot_audit_sealed.rs:95: raw.slot_audit()
  LINE   crates/store/tests/slot_audit_sealed.rs:149: Ok(file) => {
  LINE   crates/store/tests/slot_audit_sealed.rs:150: assert_eq!(file.form(), Form::Sealed, "byte {at}");
  LINE   crates/store/tests/slot_audit_sealed.rs:151: assert_eq!(file.slot_audit(), None, "byte {at}");
  LINE   crates/store/tests/slot_audit_sealed.rs:152: assert_eq!(file.header().n_valid, 40, "byte {at}: never read short");
  LINE   crates/store/tests/slot_audit_sealed.rs:153: if let Ok(last) = file.read_record(39) {
  LINE   crates/store/tests/slot_audit_sealed.rs:154: assert_eq!(last.close, 7_039, "byte {at}");
  BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 True: if let Ok(last) = file.read_record(39) {
  BRANCH crates/store/tests/slot_audit_sealed.rs:153:24 False: if let Ok(last) = file.read_record(39) {
  LINE   crates/store/tests/slot_damaged_log.rs:94: other => panic!("{key}: {other:?}"),
  LINE   crates/store/tests/slot_damaged_log.rs:101: other => panic!("{key}: {other:?}"),
  LINE   crates/store/tests/slot_damaged_log.rs:192: found
  LINE   crates/store/tests/slot_damaged_log.rs:193: .records
  LINE   crates/store/tests/slot_damaged_log.rs:194: .iter()
  LINE   crates/store/tests/slot_damaged_log.rs:195: .map(|r| (r.target.clone(), r.message.clone()))
  LINE   crates/store/tests/slot_damaged_log.rs:196: .collect::<Vec<_>>()
  LINE   crates/store/tests/sort_index.rs:488: index.torn()
  LINE   crates/store/tests/sort_index.rs:761: column.as_str()
  LINE   crates/store/tests/sort_index.rs:843: column.as_str()
  BRANCH crates/store/tests/sort_index.rs:387:62 False: matches!(&again.outcome, Resorted::Built(why) if why.contains("fails the checksum")),
  BRANCH crates/store/tests/sort_index.rs:430:56 False: matches!(&two.outcome, Resorted::Built(why) if why.contains("2 runs")),
  BRANCH crates/store/tests/sort_index.rs:446:63 False: assert!(matches!(&absent.outcome, Resorted::Built(why) if why.contains("no .srt")));
  BRANCH crates/store/tests/sort_index.rs:660:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  BRANCH crates/store/tests/sort_index.rs:736:58 False: matches!(&built.outcome, Resorted::Built(why) if why.contains("no .srt")),
  BRANCH crates/store/tests/sort_index.rs:874:16 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  BRANCH crates/store/tests/sort_index.rs:874:54 False: if reason.contains("2031-02.new.srt") && reason.contains("not a regular file")),
  BRANCH crates/store/tests/sort_index.rs:889:16 False: if reason.contains("2031-02.new.srt")),
  BRANCH crates/store/tests/support/mod.rs:62:15 False: let ran = output.status.success() && stdout.contains("1 passed");
  BRANCH crates/store/tests/tail_proof.rs:144:9 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  BRANCH crates/store/tests/tail_proof.rs:144:36 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  BRANCH crates/store/tests/tail_proof.rs:144:60 False: found.errors.is_empty() && found.malformed == 0 && !found.hit_scan_cap,
  LINE   crates/store/tests/tix.rs:173: timeframe.as_str()
  BRANCH crates/store/tests/tix.rs:605:76 True: && matches!(value, telemetry::OwnedValue::Str(file) if file.starts_with(&ours))
  BRANCH crates/store/tests/tix_rebuilder.rs:521:15 False: }) if asked == SYMBOL + 1
  LINE   crates/store/tests/tix_small_month.rs:160: other => panic!("not committed: {other:?}"),
  LINE   crates/store/tests/tix_small_month.rs:463: reader.time_lookup()
  BRANCH crates/store/tests/tix_small_month.rs:461:74 False: matches!(reader.time_lookup(), TimeLookup::Bisection(ref why) if *why != Why::BelowThreshold),
  LINE   crates/store/tests/tix_warn_once.rs:127: other => panic!("{key} is not a count: {other:?}"),
  LINE   crates/store/tests/tix_warn_once.rs:160: other => panic!("unexpected store.tix info line {other:?}: {line:?}"),
  LINE   crates/store/tests/tix_warn_once.rs:162: other => panic!("unexpected store.tix line at {other:?}: {line:?}"),
  LINE   crates/store/tests/unit.rs:110: input.len()
  LINE   crates/store/tests/unit.rs:527: _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
  LINE   crates/store/tests/unit.rs:1820: matches!(
  LINE   crates/store/tests/unit.rs:1859: matches!(
  LINE   crates/store/tests/unit.rs:1874: matches!(
  LINE   crates/store/tests/unit.rs:2096: vendor.as_str(),
  LINE   crates/store/tests/unit.rs:2826: a.as_str(),
  LINE   crates/store/tests/unit.rs:2827: b.as_str()
  LINE   crates/store/tests/unit.rs:2833: a.as_str()
  LINE   crates/store/tests/unit.rs:2893: tf.as_str()
  BRANCH crates/store/tests/unit.rs:399:13 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  BRANCH crates/store/tests/unit.rs:399:36 False: assert!(real.ohlc_is_sane() && real.counts_are_sane());
  BRANCH crates/store/tests/unit.rs:868:13 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  BRANCH crates/store/tests/unit.rs:868:54 False: assert!(!Layout::OVERLAY.requires_checksums() && !Layout::GREEKS.requires_checksums());
  BRANCH crates/store/tests/unit.rs:1475:32 False: read == old || read == new,
  BRANCH crates/store/tests/unit.rs:2047:26 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  BRANCH crates/store/tests/unit.rs:2047:43 False: .all(|c| !c.is_empty() && *c != ".." && *c != "."),
  BRANCH crates/store/tests/unit.rs:2173:13 False: err.to_string().contains(field) && err.to_string().contains("case"),
  BRANCH crates/store/tests/unit.rs:2173:48 False: err.to_string().contains(field) && err.to_string().contains("case"),
  LINE   crates/store/tests/verdict.rs:129: Found::NoRecord(none) => panic!("expected a record, got the no-record answer {none:?}"),
  LINE   crates/store/tests/verdict.rs:168: other => panic!("100x must be refused table_too_large, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:395: other => panic!("a 65-byte instrument key must be refused, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:450: panic!("{whole} must be found");
  LINE   crates/store/tests/verdict.rs:728: other => panic!("expected image_missing, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:793: other => panic!("expected generation_collision, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:1318: other => panic!("bit {bit}: expected flag_unknown, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:1821: other => panic!(
  LINE   crates/store/tests/verdict.rs:1823: clean.len()
  LINE   crates/store/tests/verdict.rs:1856: other => panic!("expected record_corrupt, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:2283: feed.as_str(),
  LINE   crates/store/tests/verdict.rs:2284: timeframe.as_str()
  LINE   crates/store/tests/verdict.rs:2435: other => panic!("expected lock_stalled, got {other:?}"),
  LINE   crates/store/tests/verdict.rs:2455: matches!(
  LINE   crates/store/tests/verdict.rs:2549: image.display()
  LINE   crates/store/tests/verdict.rs:2554: other => panic!("expected index_missing, got {other}"),
  LINE   crates/store/tests/verdict.rs:2730: other => panic!("{other}"),
  BRANCH crates/store/tests/verdict.rs:228:13 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  BRANCH crates/store/tests/verdict.rs:228:36 False: got.cost.reads <= 5 && got.cost.opens <= 2,
  BRANCH crates/store/tests/verdict.rs:449:13 False: let Found::Record { ordinal, record } = &answer.found else {
  BRANCH crates/store/tests/verdict.rs:584:12 False: if count > 2 {
  BRANCH crates/store/tests/verdict.rs:1890:17 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  BRANCH crates/store/tests/verdict.rs:1890:40 False: assert!(got.cost.opens <= 2 && got.cost.reads == 5);
  BRANCH crates/store/tests/verdict.rs:2123:17 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  BRANCH crates/store/tests/verdict.rs:2123:40 False: assert!(got.cost.reads <= 5 && got.cost.opens <= 2, "{:?}", got.cost);
  BRANCH crates/store/tests/verdict.rs:2358:58 False: matches!(&refused, VerdictError::Staging(why) if why.path() == image_tmp),
  BRANCH crates/store/tests/verdict.rs:2545:21 False: image.extension().is_some_and(|ext| ext == "vdt")
  BRANCH crates/store/tests/verdict.rs:2546:28 False: && image.parent().is_some_and(|dir| dir.ends_with("2031"))
  BRANCH crates/store/tests/verdict.rs:2547:28 False: && image.exists(),
  BRANCH crates/store/tests/verdict.rs:2604:13 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  BRANCH crates/store/tests/verdict.rs:2604:22 False: assert!(named && (1..=2).contains(&seen), "seen {seen}");
  BRANCH crates/store/tests/verdict.rs:2782:9 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  BRANCH crates/store/tests/verdict.rs:2782:61 False: said.contains(&format!("{} bytes", ceiling + 1)) && said.contains("not read"),
  LINE   crates/store/tests/write.rs:1402: sidecar.display()
  LINE   crates/store/tests/write.rs:1419: sidecar.display()
  LINE   crates/store/tests/write.rs:1619: panic!("{torn_at}: a torn genesis slot holds nothing and must be repaired: {why}")
  LINE   crates/store/tests/write.rs:1663: panic!(
  BRANCH crates/store/tests/write.rs:1564:13 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  BRANCH crates/store/tests/write.rs:1564:62 False: text.contains(&on_disk.display().to_string()) && text.contains("committed"),
  BRANCH crates/store/tests/write.rs:2103:85 False: Err(StoreError::Format { path, source: FormatError::NoValidHeader }) if *path == bars
`````

# Cloud fix lane 1

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue (state/c4/wave2-after-merge.json on the local drive), 2 Oct 2026. Base every fix on origin/main.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md. One branch and one PR per item, named `fix/cloud-<id>`.

## W3-store1-3 · high bug · store

**Where:** `crates/store/src/file.rs:1573`

**Finding:** BarFile::seal_committed (via BarFile::append forward path, 1813-1846) (crates/store/src/file.rs:1573): corrupt file + rerun: a committed record in the tail block rots (or is a lost write) between appends, and the next strictly-following append re-seals the block over the corrupted bytes, so the corruption becomes permanently 'verified' Code path: the forward append (advance Ok -> commit -> write_fully -> sync -> seal_committed) never calls read_row or verify_block_of. seal_committed reads the covered bytes back and seals whatever is on disk (file.rs:1596-1601 `let mut bytes = vec![0u8; span]; read_fully(&self.bars, ...)?;

**Evidence:**
  - The path then goes straight to commit (1813), `write_fully` + `sync_all` (1825-1826) and `self.seal_committed(first_index, commit.header.n_valid)` (1841). It reads those bytes from disk (`let mut bytes = vec![0u8; span]; read_fully(&self.bars, &self.bars_path, start, &mut bytes)?;`, 1596-1597), runs `crate::block::seal(self.layout, n_valid, block, &bytes)` (1598-1599) and writes the result to the sidecar (1605-1610).
  - In `BarFile::append` (crates/store/src/file.rs:1711), a batch that strictly follows the commit takes the `Ok(next)` arm of `self.header.advance(...)`. `seal_committed` (file.rs:1550) starts at `first = self.layout.block_of(first_index)`, which is the old tail block.

**Expected fix and test:** None

## W3-store1-0 · medium cost · store

**Where:** `crates/store/src/file.rs:2397`

**Finding:** first_at_or_after (fn) / BarFile::first_at_or_after (1931) / RevisionReader::first_at_or_after (repair.rs:217) (crates/store/src/file.rs:2397): per one timestamp-to-index lookup; api /bars.json calls it twice per request (crates/api/src/server.rs:2291 and :2310), the cost is O(log n_valid) record preads, and each probe outside the single cached block also pays a cold block verify: a pread of up to 4,088 bytes, a 4-byte sidecar pread, a CRC-32C over the block, and an fstat when it is the tail block; it grows with n_valid of the month (log2), plus how many distinct checksum blocks the probes touch. Auditor verdict: undocumented-scan. Documented: Source only: file.rs:1918-1925 says 'addressable in log2(n_valid) reads... UNVERIFIED as a measured bound'. docs/06-limits.md has no entry: a grep for first_at_or_after and bisect finds only greeks-solver lines.

**Evidence:**
  - crates/store/src/file.rs:2401-2415: `while low < high { let mid = low.saturating_add(high.saturating_sub(low) / 2); if read(mid)?.stamp() < ts {`. BarFile::first_at_or_after (1931-1932) passes `|index| self.read_record(index)`, and read_record calls read_row (1981), which calls `self.verify_block_of(index)?` (2003) after every record read.
  - file.rs:2403-2416 is `while low < high { let mid = low.saturating_add(high.saturating_sub(low) / 2); if read(mid)?.stamp() < ts {`, which makes log2(n_valid)+1 probes. BarFile::first_at_or_after (file.rs:1931-1932) passes `|index| self.read_record(index)`. file.rs:2081 is `if self.verified.load(Ordering::Relaxed) == block { return Ok(()); }` and file.rs:2140 stores one block.

**Expected fix and test:** None

## AC-whp-cx-0 · medium bug · store

**Where:** `crates/store/src/file.rs:1137`

**Finding:** `BarFile::open_existing` is the read door behind every stored sweep: cli::stored::load, load_span, load_daily_context and load_exact_minute_context all reach it through load_classified_with_ceiling (crates/cli/src/stored.rs:2216). It is also used by api/src/bars.rs:330, api/src/server.rs:12131, pull ingest/scrub and store::repair. It opens the bar file with `File::open(&bars_path)`, the lock with `File::open(&lock_path)`, and the checksum sidecar with `open_read(at)` = `fs::OpenOptions::new().read(true).open(path)`. None of these passes O_NONBLOCK, and nothing checks `is_file()` before the open blocks. When a FIFO (or a symlink to one) sits at `<month>.bin`, `<month>.lock` or `<month>.crc`, open(2) blocks until a writer appears. So the sweep, the HTTP worker or the pull hangs with no refusal, no timeout and no log line. `catalog::walk` (catalog.rs:207) sends every non-directory entry to `classify`, so `pool`/`sweep-all` list a FIFO named like a month as a held month and then hang opening it. The repository already knows the defect: `checksum_audit::open_regular` (checksum_audit.rs:28-48) opens with O_NONBLOCK|O_NOFOLLOW and refuses a non-regular file, and its module doc says the or

**Evidence:**
  - - crates/store/src/file.rs:1137: `let bars = fault(File::open(&bars_path), &bars_path, Action::Open)?;` - file.rs:1144: `let lock = match File::open(&lock_path) {` - file.rs:1486: `Access::Read => match open_read(at) {` - file.rs:4696-4697: `fn open_read(path: &Path) -> io::Result<File> { fs::OpenOptions::new().read(true).open(path) }` - None of these opens passes O_NONBLOCK, and no file-type check runs first. checksum_audit::open_regular(at)` (file.rs:1491-1495). `classify` checks only depth, ...
  - At fix/c2-final:crates/store/src/file.rs:1137 it runs `let bars = fault(File::open(&bars_path), &bars_path, Action::Open)?;`. For example, cli stored.rs:2216 calls `BarFile::open_existing(root, path, symbol_id)` directly, and api bars.rs:330, server.rs:12131, pull ingest.rs:2030/2214/2541, scrub.rs:204 and store repair.rs:175/275 all reach the same door.

**Expected fix and test:** None

## ET-rust-only-purity-5 · medium bug · vocab

**Where:** `crates/vocab/tests/workspace_is_rust.rs:348`

**Finding:** `no_declared_native_dependency_is_compiled_any_more` checks a hand-written boolean, not the build, so it cannot fail (§4: a test that asserts nothing). The name-set fingerprint also misses a feature flip that re-enables an optional C dependency already in the lock. Only cargo-deny (Gate 3, build job) caught ring returning.

**Evidence:**
  - crates/vocab/tests/workspace_is_rust.rs:352 filters on `d.compiled_on_this_target && packages.contains(d.name)`. `compiled_on_this_target` is a literal `false` in all four DECLARED rows (:62, :80, :87, :94). `git grep compiled_on_this_target` finds only the field declaration (:54), the four literals and the read at :352.
  - In crates/vocab/tests/workspace_is_rust.rs, the filter at :350-353 is `d.compiled_on_this_target && packages.contains(d.name)`. All four DECLARED rows set that field to `false` (:62, :80, :87, :94). `git grep compiled_on_this_target` finds no other code that reads it, so `compiled` is always empty and the assert at :367 cannot fail unless someone edits the table.

**Expected fix and test:** None

## ET-bars-candles-store-9 · low cost · store

**Where:** `crates/store/benches/ratio.rs:217`

**Finding:** Claimed: docs/04-invariants.md C-28/C-29: bar lookup is measured flat and within 800 floors Actual: Both rows re-read one fixed index, so after the first call only the warm cached-block path is timed. The cold block verify (3 preads, a 4 KB CRC and an allocation) that every random access and every bisection probe pays is never measured.

**Evidence:**
  - Gate 8 timing: cost_ps keeps the minimum over TRIALS=60 (crates/store/benches/ratio.rs:62, 68-81). C-29 reads the fixed index 9_999, 2,000 reps per trial (ratio.rs:246). C-28 reads the fixed index 0 or n-1, 200 reps per trial (ratio.rs:279-284). The cache: verify_block_of returns at file.rs:2081 when the verified block equals block_of(index), and it stores the block at file.rs:2140.
  - C-29 times `file.read_record(black_box(9_999))` at crates/store/benches/ratio.rs:246. That is one fixed index, 2,000 reps, and the minimum over TRIALS=60 (:63, :69-81). C-28 uses the same fixed-index closures: `first` reads 0 (:279) and `last` reads n-1 (:280-284). `verify_block_of` returns at file.rs:2081 when `self.verified` equals the block. the record pread (:1995)

**Expected fix and test:** None

## ET-o1-proof-coverage-4 · low bug · store

**Where:** `crates/store/benches/ratio.rs:217`

**Finding:** Bar-lookup rows re-read one index, so only the cached-block path is timed. A read that crosses checksum blocks costs about 10x more (2,034-2,238 floors) and exceeds C-29's 800-floor budget, which has never measured it. The uncached path also heap-allocates vec![0u8; span] on every read.

**Evidence:**
  - C-29 reads one fixed index 2,000 times: crates/store/benches/ratio.rs:239, `cost_ps(2_000, || file.read_record(black_box(9_999)))`. C-28's closures also read one fixed index per loop (ratio.rs:280-285). cost_ps (ratio.rs:69-82) keeps the minimum over TRIALS=60 (ratio.rs:63). read_row calls verify_block_of for every read (file.rs:2003).
  - Every read goes through `read_row`, which calls `verify_block_of` (crates/store/src/file.rs:2003). `verify_block_of` returns early only when `self.verified` already holds the same block (file.rs:2081). allocates `vec![0u8; span]` on the heap (file.rs:2105), does a pread of up to BLOCK_LEN bytes (file.rs:2106), For the tail block that adds an fstat and a second `vec![0u8; span]` (file.rs:2185).

**Expected fix and test:** None

## ET-bars-candles-store-0 · low bug · store

**Where:** `crates/store/src/file.rs:1995`

**Finding:** After the first read in a checksum block, later reads in that block on the same handle return fresh disk bytes with no verification. The bytes served are never the bytes that were checksummed. A corruption that lands after the first read is served as a plausible wrong price. The field doc (file.rs ~862-866) claims a re-verification `reaches the same verdict` and calls this a cost, not a correctness hole, which A15 disproves. Existing test a_block_is_verified_once_per_handle_and_not_once_per_record damages only the sidecar, so the bars it serves are still correct and it cannot see this.

**Evidence:**
  - read_row reads the record into a stack buffer with a separate pread at crates/store/src/file.rs:1995. It then calls verify_block_of(index) at :2003. verify_block_of returns Ok at :2081 when `self.verified.load(Relaxed) == block`, so the record bytes read at :1995 are never checked on that path. On a block's first touch, the block is re-read with its own pread at :2106. The cache is set at :2140.
  - read_row pulls the record into its own stack buffer at :1995 (`read_fully(&self.bars, …, at, image)`). Only after that does it call verify_block_of at :2003. verify_block_of returns Ok at :2081-2083 whenever `self.verified == block`, without doing any read. A full check reads the block into a separate Vec (:2105-2106), checks it, and then stores the block index (:2140).

**Expected fix and test:** None

## ET-bars-candles-store-2 · low bug · store

**Where:** `crates/store/src/file.rs:1711`

**Finding:** The store's write boundary does not check that a bar's timestamp falls inside the month its path names, and accepts i64::MIN and i64::MAX timestamps. One such bar permanently blocks every later legitimate append to that month (append-only, strictly increasing). Month membership is enforced only upstream by pull::ingest::months_in (ingest.rs:1916) and re-checked by one reader, committed_cash_days (ingest.rs ~2036-2041); cli::stored::decode_loaded does not re-check it.

**Evidence:**
  - A 1893456000000000 (2030-01-01) bar then gave Ok(Committed{first_index:3,n_valid:4}). survey (crates/store/src/file.rs:2263-2302) checks only is_sane, bad_counts and strict order within the batch. Header::advance (crates/store/src/header.rs:301-341) checks only the counters and that the batch follows last_ts_micros. BarFile (file.rs:796ff) keeps no month, and store/src contains no `.month()` call.
  - CODE: BarFile::append (crates/store/src/file.rs:1711-1720) checks the stride and then calls survey(batch) and header.advance(). survey (file.rs:2263-2302) checks only is_sane, bad_counts and strict ordering within the batch. Header::advance (crates/store/src/header.rs:301-332) checks only the counters, last>=first and first>self.last_ts_micros. BarFile (file.rs:796ff) keeps no month field.

**Expected fix and test:** None

## ET-bars-candles-store-3 · low bug · store

**Where:** `crates/store/src/file.rs:2263`

**Finding:** A 1min file accepts a timestamp that is not on the 60-second grid (09:15:30) and one outside every NSE session (03:00 IST). The declared timeframe is metadata only. Session and grid filtering happen only upstream in fetch::land (Window::verdict, fetch.rs:803) and complete_minutes.

**Evidence:**
  - Setup: a 1min file for NSE/INDEX/NIFTY 2024-06 created with `BarFile::open_or_create`; its header reports `timeframe_secs = 60`. T0 = 1_717_386_300_000_000 is 2024-06-03 09:15 IST, the same constant `tests/write.rs` uses. 09:15:30 (T0 + 30s): `Ok(Committed { first_index: 0, n_valid: 1 })` 2024-06-04 03:00 IST (T0 + 17h45m): `Ok(Committed { first_index: 1, n_valid: 2 })`
  - It opened a 1min NSE/INDEX/NIFTY 2024-06 bar file, whose header.timeframe_secs was 60. `append 09:15:30 IST -> Ok(Committed { first_index: 0, n_valid: 1 })` `append 03:00 IST next day -> Ok(Committed { first_index: 1, n_valid: 2 })` `append 09:15:00.000001 IST next day -> Ok(Committed { first_index: 2, n_valid: 3 })` `append 2024-07-01 09:15 IST into a 2024-06 file -> Ok(Committed { first_index: 3, n_valid: 4 })`

**Expected fix and test:** None

## ET-bars-candles-store-12 · low cost · store

**Where:** `crates/store/src/file.rs:1884`

**Finding:** Claimed: `no bench in this repository times a syscall` Actual: Stale: C-28/C-29 time read_record, which includes one pread per call

**Evidence:**
  - `BarFile::read_record` (crates/store/src/file.rs:1902) calls `read_row::<Bar>`. That calls `read_fully(&self.bars, …)` (file.rs:1995). That calls `Positional::get`, which for `File` is `FileExt::read_at` (file.rs:2632). C-29 at crates/store/benches/ratio.rs:246 (`file.read_record(black_box(9_999))`). C-28 at ratio.rs:279-283. `main` runs both (ratio.rs:419-420). crates/store/src/file.rs:1886-1888
  - crates/store/src/file.rs:1885-1888 says: "no bench in this repository times a syscall — `crates/store/benches/ratio.rs` measures the arithmetic and the checksum. The syscall cost is UNVERIFIED." docs/06-limits.md:2831-2834 (§40.2) repeats "no bench in this repository times a syscall". read_record (file.rs:1895-1903) calls read_row (1981-2005).

**Expected fix and test:** None

## W3-store1-1 · low cost · store

**Where:** `crates/store/src/file.rs:2344`

**Finding:** already_stored (called from BarFile::append at file.rs:1772) (crates/store/src/file.rs:2344): per one re-offered batch at the store's ingest boundary (the store-level duplicate check that answers AlreadyPresent), the cost is O(log n_valid + batch) record reads, each able to trigger a cold block verify; it grows with log2(n_valid) for the locate, plus the batch length. Auditor verdict: undocumented-scan. Documented: Source only: file.rs:2319-2335 says 'O(log n_valid) reads to locate the run... This is NOT an O(1) path and does not claim to be'. docs/06-limits.md has no cost statement.

**Evidence:**
  - file.rs:1772 calls `already_stored(batch, first_ts, self.header.n_valid, |index| self.read_row::<R>(index))` when `Header::advance` refuses with `TimestampsOutOfOrder`. already_stored (file.rs:2344) runs `let at = first_at_or_after(n_valid, first_ts, &read)?;`, which bisects on `read(mid)?.stamp() < ts` at file.rs:2397-2418. != *bar` at file.rs:2370-2371.
  - Call site: file.rs:1772 `let located = already_stored(batch, first_ts, self.header.n_valid, |index| { self.read_row::<R>(index) })?;`. Locate step: file.rs:2353 `let at = first_at_or_after(n_valid, first_ts, &read)?;`. That is a bisection over the stored records (file.rs:2408-2410, `let mid = low.saturating_add(high.saturating_sub(low) / 2); if read(mid)?.stamp() < ts`).

**Expected fix and test:** None

## ET-o1-proof-coverage-3 · low bug · vocab

**Where:** `crates/vocab/benches/ratio.rs:266`

**Finding:** C-V-02 is documented as the measurement that catches an early-exit hits loop. At the asserted 3.0x ceiling it passes one (2.056x); engine C-E-03 passes too (0.558x). Only the source-shape unit test catches it.

**Evidence:**
  - *The mutation.** I replaced the branchless body of `hits` (crates/vocab/src/mask.rs:145-151) with an early-exit loop over the six words: `while i < WORDS { if (self.0[i] & candidate.0[i]) != candidate.0[i] { return false; } i += 1; } true`. C-V-02 word 0 to the last word measured 2.158x, 2.055x and 2.211x, against the 3000 permille ceiling at crates/vocab/benches/ratio.rs:59.
  - All timings below are scratch timings under load (load average 21-41), not Gate 8. docs/04-invariants.md:1804 says C-V-02 "is the measurement that catches an early-exit loop". crates/vocab/benches/ratio.rs:260 says "This is the measurement that would catch an early-exit loop". ratio.rs:20-22 and docs/05-decisions.md:9531-9533 repeat the claim. The ceiling is `CEILING_PERMILLE: u128 = 3_000` (ratio.rs:59).

**Expected fix and test:** None

## ET-o1-proof-coverage-13 · low cost · vocab

**Where:** `crates/vocab/benches/ratio.rs:266`

**Finding:** Claimed: 'this is the measurement that catches an early-exit loop' Actual: An early-exit word loop in hits passes C-V-02 at 2.056x (monotone 1.093/1.275/1.560/1.895/2.056 across words 1-5) under the 3.0x ceiling. The engine bench also passes (C-E-03 0.558x). Only the source-shape unit test catches it

**Evidence:**
  - Mutant A replaces the body of hits (crates/vocab/src/mask.rs:144-152) with `while w < WORDS { if (self.0[w] & candidate.0[w]) != candidate.0[w] { return false; } w += 1; } true`. The engine ratio is already two-sided (crates/engine/benches/ratio.rs:110-113), but 1/0.561 = 1.78x, which is under the 3.0x ceiling. FAILED`, panicking at mask.rs:496 with "`hits` contains `while `".
  - The shipped `hits` has no branch: six AND/XOR lines and one OR-fold (crates/vocab/src/mask.rs:144-152). The vocab bench's `ratio` only checks one direction, `permille <= CEILING_PERMILLE` (crates/vocab/benches/ratio.rs:88-104), with CEILING_PERMILLE = 3_000 (ratio.rs:59). The engine bench checks both directions, `up.max(down) <= CEILING_PERMILLE` (crates/engine/benches/ratio.rs:106-113).

**Expected fix and test:** None

## UC-7 · low bug · vocab

**Where:** `crates/vocab/tests/workspace_is_rust.rs:348`

**Finding:** Claimed: test `no_declared_native_dependency_is_compiled_any_more` guards against native code returning ('do not let a green suite imply purity that is gone') Actual: It filters on a hand-written constant `compiled_on_this_target: false` (lines 62, 80, 87, 94), so it asserts nothing about the build. With ring re-enabled and 24 ring object files compiled, it passed. The fingerprint test also passed, because the lock's package-name set did not change.

**Evidence:**
  - In crates/vocab/tests/workspace_is_rust.rs:350-353, `no_declared_native_dependency_is_compiled_any_more` filters on `d.compiled_on_this_target && packages.contains(d.name)`. So `compiled` is always empty and the assertion at :367 cannot fail unless someone edits the source literal.
  - CODE: crates/vocab/tests/workspace_is_rust.rs:352 filters on `d.compiled_on_this_target && packages.contains(d.name)`. So `compiled` is always empty, and the assert at lines 367-375 cannot fail whatever the build does. deny.toml:127 and :130 ban `ring` and `cc`, added by D-0211, and CI Gate 3 runs `cargo deny check` unconditionally.

**Expected fix and test:** None

## UC-13 · low bug · vocab

**Where:** `crates/vocab/tests/workspace_is_rust.rs:348`

**Finding:** crates/vocab/tests/workspace_is_rust.rs is partial: The unlisted-runtime lock fails correctly: `the lockfile holds: ["rustpython-vm", "cpython", "napi"]` plus 'the dependency count changed'. Clean lock: 3 passed. But the third test filters on the hand-written constant `compiled_on_this_target: false` (lines 62, 80, 87, 94; filter at :352), so it cannot fail. With ring re-enabled and compiled (24 .o files) all 3 still passed, because the lock's name set was unchanged (only 1 line in a dependency list changed).

**Evidence:**
  - All four DECLARED rows set `compiled_on_this_target: false` (crates/vocab/tests/workspace_is_rust.rs:62, :80, :87, :94). The third test filters on `d.compiled_on_this_target && packages.contains(d.name)` (:352). With those constants its result is always empty, so the assertion at :367 cannot be moved by any lockfile or build change.
  - CODE: In crates/vocab/tests/workspace_is_rust.rs, `no_declared_native_dependency_is_compiled_any_more` filters with `d.compiled_on_this_target && packages.contains(d.name)` (:352). All four DECLARED rows hard-code `compiled_on_this_target: false` (:62 ring, :80 cc, :87 wit-bindgen, :94 iana-time-zone-haiku).

**Expected fix and test:** None

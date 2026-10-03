# conc-pass1 / cli2: concurrency and state audit of the cli result-store slice at 331b05c

**Verdict: 5 findings (0 high, 2 medium, 3 low).** No deadlock, wrong atomic ordering or production `.lock().unwrap()` in this slice. Every defect is a crash/ENOSPC recovery gap, an unlocked length measurement against a concurrent appender, or a key-based lookup that returns another run's row.

Slice: `crates/cli/src/{sweep_evidence,candidate_trades(+candidate_trades/codec),stored,result_set,results,trades,frontier,pool}.rs`, plus the `lib.rs` orchestration these files are called from (`record_all_attempt`, `record_unadmitted`, `ensure_*`, `latest_for`, `one_rung`). Method: source reading only. No cargo was run. Prior reports `hunt-conc.md` and `errpaths.md` were read, and nothing they already list is repeated here: hunt-conc-1/2 (durable writes inside rayon), hunt-conc-8, errpaths-2 (`exists()` in `committed_receipt_with_limit`), and the `candidate_trades.rs:426` poisoned-lock `if let Ok` (info).

---

## cli2-1: medium. One ENOSPC/EDQUOT short write leaves the global sweep-evidence journal torn and refuses every later attempt start, until someone repairs it by hand

- **Where:** `sweep_evidence.rs:1364-1397` (`append_events`, used by `allocate` at 1401 and `journal` at 829). The same no-rollback shape also appears in `append_row` (1282-1297, per-attempt `levels`) and in `Attempt::ranked` (601-640, per-row `write_all` at 629).
- **Code:**
  ```rust
  file.seek(SeekFrom::End(0))
      .and_then(|_| file.write_all(&bytes))
      .and_then(|()| barrier(&file, path))
      .map_err(io_error)?;
  ```
  Then, on every later append, `shape()` (1195-1200):
  ```rust
  if len < HEADER || !(len - HEADER).is_multiple_of(stride) {
      return Err(format!("{} has a torn or short fixed-stride evidence file; no byte was changed", ...
  ```
- **Why it is wrong:** A `write_all` that fails after a short write leaves a partial row. The cause is known, the process is alive, and the lock is held, yet the partial row is not rolled back. The sibling stores roll back exactly this case and explain why: `results.rs:1339` (`set_len(at)` with "the difference between a disk that filled up and a ledger that has to be repaired by hand"), `frontier.rs` `append_locked_with` (`set_len(end)`), `trades.rs:659` and `result_set.rs:479`. `attempts.bin` is the single journal that allocates tokens for every operation (Sweep, Audit, boolean candidates, expression search, index-stop, and others), so one torn tail stops all of them. The invariant test `judge` (`sweep_evidence_tests.rs:375-414`) only models a crash: it accepts a torn journal as a refusal. That is a different situation from a live, known write failure.
- **Repro:**
  1. Fill the store's filesystem until the block that holds `attempts.bin`'s tail has less than 96·N bytes free and no block can be allocated.
  2. Start a grouped begin (`begin_many`, for example index-stop catalog groups of 16 = 1,536 bytes), or call `finish_many`.
  3. `write(2)` writes the bytes up to the block boundary and returns short. The next `write` in `write_all` gets ENOSPC.
  4. `append_events` returns `sweep evidence I/O refused: No space left on device` and leaves `(len-16) % 96 != 0`.
  5. Free disk space and run any `sweep-stored`, `audit-*` or boolean verb. `begin` → `allocate` → `append_events` → `shape` refuses `attempts.bin has a torn or short fixed-stride evidence file` every time, for every identity, forever.
  6. The `Drop` Refused-terminal path for live attempts refuses the same way, so their lifecycles also stay `Running`.

  If the same thing happens in `append_row` or `ranked`, that attempt's `levels`/`ranked` file is torn. `seal` → `measured_details` → `shape` then refuses, so no terminal (not even Refused) can ever be written, and `read(identity)` refuses until a newer attempt for the same identity starts.
- **Minimal fix:** Under the held exclusive lock, take `let end = file.seek(SeekFrom::End(0))?` before the write, and on `write_all` error call `file.set_len(end)` (report both errors if the truncate also fails). This is the pattern `results.rs:1339` already uses. Do it in `append_events`, `append_row` and `ranked` (`ranked` also needs to compute `end` before its per-row loop). Do not truncate when only the barrier failed, for the reason `results.rs:1286-1300` gives.

## cli2-2: medium. `one_rung` (and therefore `pool`) reports the newest row with a matching key, not this run's row, so a rerun or a concurrent writer substitutes another run's numbers and identity

- **Where:** `lib.rs:13705` → `latest_for`, `lib.rs:15586-15619`. Consumed by `pool.rs:294-300` (pass 1 `Screened.outcome`) and by `pool.rs:745-800` (`union_of` reads `frontier.of_run(&record.identity)`).
- **Code:**
  ```rust
  for back in 1..=count {
      let record = store.read(count.saturating_sub(back))?;
      if record.feed == feed && record.underlying == name && record.timeframe == tf
          && record.from_year == from.0 && ... && record.min_hits == min_hits
      { return Ok(record); }
  ```
- **Why it is wrong:** The key (feed, underlying, rung, span, `min_hits`) leaves out terms that are in the identity and that change the recorded answer. One is `Params::policy`, which folds `Rules::operator()` from `BRUTEX_MIN_RR_BP`, `BRUTEX_MAX_MAE_PPM`, `BRUTEX_MIN_TRADES` and the other rule knobs, plus the lens, grid rungs and screen cap (`policy_of`, lib.rs:10402). The others are `Params::ceiling` and `commit`. The `one_rung` comment (13689-13703) closes only the case where the append failed and calls identity matching "the stronger fix". It misses the two paths where the append succeeds or is reused but a different row is newer.
- **Repro, sequential, which breaks §3 rule 5:**
  1. `BRUTEX_MIN_RR_BP=200 cli range-rung dhan NIFTY 15min 2020 1 2026 9 <ppm>` appends row A (identity Ia).
  2. `BRUTEX_MIN_RR_BP=0 cli range-rung ...` with the same arguments appends row B (Ib ≠ Ia, different `trades` and `pessimistic`).
  3. Repeat step 1 exactly. `ensure_run_record` finds Ia → `Committed::Reused`, so the audit text has no NOT_RECORDED. `latest_for` scans backwards and returns row B.

  The table now prints B's trades, totals and exit rungs under a MIN_RR=200 run, while step 1 printed A's. In `pool`, `union_of` then reads `of_run(Ib)` and pools the MIN_RR=0 run's frontier.
- **Repro, concurrent:** The `cli` process (rules X) and the api server (request knobs Y) sweep the same rung, span and support at the same time. Both append (with different identities). Whichever row lands second is returned to both `latest_for` calls, so one of them renders the other's run.
- **Minimal fix:** Carry the computed `RunId` (or the committed ledger index from `ensure_run_record`) back from `audit_range_for_attempt`/`record_*` into `one_rung`. Read the row with `Results::of_identity(&id)` (O(1) on the shared writer) and refuse when it is absent, instead of keying on span fields.

## cli2-3: low. Frontier and trade opens measure the length and index rows without a lock, so a concurrent append produces a false "append was interrupted" refusal that loses a recording

- **Where:**
  - `frontier.rs:880-935` (`Frontier::open`): `let len = file.metadata()…len()` at 893, then `check_header` at 910 and `index_of`, with no flock.
  - `frontier.rs:811-845` (`open_read_bounded`): 824/834, same.
  - `trades.rs:427-470` (`open_read_bounded`, 453) and `trades.rs:480-530` (`open`, 510), same.
  - `results.rs:707-752` and `result_set.rs:249-262` take a validation flock for exactly this reason ("Cooperative appenders must not change the length in between those steps").
- **Code:** `check_header` (frontier.rs:1858-1868):
  ```rust
  if !body.is_multiple_of(STRIDE) { return Err(format!("{} has {spare} bytes past its last whole row — an append was interrupted. ..."
  ```
- **Why it is wrong:** `append_all` writes one buffer of n×208 bytes under an exclusive flock. Linux raises `i_size` page by page during that `write(2)`, so an unlocked `stat` in another thread or process can see a length that ends mid-row. `record_all_attempt` serializes frontier writers with `LEDGER` + `ResultSetLock`, but `record_unadmitted` (lib.rs:19064-19077) takes neither and calls `record_frontier` → `Frontier::open`. Both `range-all` (8 rungs, `sweep_rungs` par_iter, lib.rs:15218) and `pool` pass 1 (`pool.rs:294`, up to 210 instruments in par_iter) run admitted and unadmitted rungs at the same time in one process.
- **Repro:**
  1. Thread A (an admitted rung) is in `record_all_attempt` → `Frontier::append_all` writing 25 rows (5,200 bytes, which crosses a page).
  2. After the first page copy, `i_size` = the page boundary, which is generally not 16+k·208.
  3. Thread B (an unadmitted rung) runs `record_unadmitted` → `Frontier::open` → `metadata().len()` returns that length → `check_header` refuses "… bytes past its last whole row — an append was interrupted".
  4. B prints `NOT RECORDED: …` and `one_rung` turns B's outcome into `Err("the result was not recorded: … append was interrupted")`. On a healthy file, that instrument drops out of the pool's pass 1.
  5. The same race hits api's cached-handle (re)open (`detail::FRONTIER`/`TRADES` `open_read_bounded`) during a CLI append. The request is refused with a corruption claim.
- **Minimal fix:** Take the open-time validation lock the way `results::open_result_file` does: `Flock::lock_shared` on a `try_clone` for readers, and exclusive for `open`, including the `len == 0` → `write_fresh_header` branch. Measure, `check_header` and `index_of` under it, then release by name.

## cli2-4: low. `record_unadmitted` publishes the ledger marker without the directory barrier that its own protocol requires first

- **Where:** `lib.rs:19064-19077` (`record_unadmitted`), compared with `lib.rs:20314-20322` (`record_all_attempt`). The rule is in the doc on `confirm_result_directory` (lib.rs:17296-17301).
- **Code:**
  ```rust
  let (frontier, rows) = record_frontier(into.root, id, what.by_evidence, what.rules, what.priced)?;
  let receipt = ensure_detail_receipt(into.root, id.bytes(), rows, 0, Direction::Long)?;
  let (summary, _) = record_swept_run(into, id, what.sweep, what.bars, what.min_hits)?;
  ```
  `record_swept_run` → `ensure_run_record` calls `confirm_result_directory` only after `store.append(record)` (lib.rs:17344-17346).
- **Why it is wrong:** The documented rule is "The children are confirmed before the ledger marker" (17299). `record_all_attempt` honours it with `// FILE CONTENTS ARE NOT THEIR NAMES … this barrier makes newly-created directory entries durable before the ledger can advertise them`. `record_unadmitted` skips the barrier. On a fresh store, the first unadmitted run creates `frontier.bin` and `detail-sets.bin`. Each is fsynced, but their directory entries are not durable when `runs.bin`'s row is synced.
- **Repro:**
  1. On a fresh `results/`, an unadmitted audit appends and fsyncs the `runs.bin` row.
  2. Power is lost before the trailing `confirm_result_directory`.
  3. On a filesystem that orders directory entries independently of an unrelated file's fsync (POSIX permits this; ext4's journal usually masks it), the ledger row survives and the `detail-sets.bin` entry does not.
  4. Every reader then refuses the committed run: `committed_receipt_with_limit` → "has a results-ledger parent, but its detail receipt could not be read". The run stays unreadable until the exact inputs are rerun. This is a loud refusal, not silent loss, so it is low.
- **Minimal fix:** Call `confirm_result_directory(into.root)?` between `ensure_detail_receipt` and `record_swept_run` in `record_unadmitted`. Better, have `record_unadmitted` go through the same `LEDGER` + `ResultSetLock` + barrier sequence as `record_all_attempt`.

## cli2-5: low. `write_exact` makes a candidate file visible empty before it locks it, so a concurrent reader is told a finishing capture is corrupt

- **Where:** `candidate_trades.rs:1307-1347` (`write_exact`) and `read_model` at 742-748. `sweep_evidence.rs:706-726` (`reserve_start`) has the same shape, but no production reader reaches a reservation before it is written (`latest()` has no production caller), so this finding is only about candidate captures.
- **Code:**
  ```rust
  match OpenOptions::new().read(true).write(true).create_new(true).open(path) {
      Ok(file) => {
          let mut file = Flock::lock(file, path).map_err(io_error)?;   // lock taken AFTER the name exists
          file.write_all(&header) ...
  ```
  Reader (742-748, and `read_sealed_generation` 1354-1370):
  ```rust
  if !path.try_exists().map_err(io_error)? { return Ok(None); }
  ... Flock::try_lock_shared(...)      // succeeds if the writer has not locked yet
  if !metadata.is_file() || len < (HEADER + SEAL) as u64 || len > max_bytes {
      return Err("candidate detail file is nonregular, truncated or above its byte admission".to_owned());
  ```
- **Why it is wrong:** `catalog.bin` is created last, and its absence means "incomplete, never completed-empty" (doc at 715-719). Between `create_new` and `Flock::lock`, the name exists with 0 bytes and no lock. A reader such as api candidate pages for the attempt being finished gets `try_lock_shared` and len 0, and refuses with a truncation/corruption claim. In that window the honest answer is "absent" (`Ok(None)`) or "busy; retry", which is what it gets one instruction later.
- **Repro:**
  1. Thread W in `Capture::finish` → `finish_inner` → `write_exact(catalog.bin)` has returned from `open(O_CREAT|O_EXCL)` and has not reached `flock(LOCK_EX)`.
  2. An api request calls `candidate_trades::read(root, id, attempt, …)` → `try_exists` is true → `try_lock_shared` succeeds → `len = 0` → `Err("candidate detail file is nonregular, truncated …")`.
  3. W's `Flock::lock` then blocks until the reader releases, and completes normally.
  4. The same applies to `start.bin`, tier and candidate files read during a run.
- **Minimal fix:** Create the file under a temporary name (unique per attempt directory, created with `create_new`), write, `sync_all`, then `hard_link`/`rename_noreplace` it to the final name and fsync the directory. Alternatively, in `read_sealed_generation` treat `len == 0` under a successfully taken shared lock as "busy/incomplete" (`Ok(None)` for `catalog.bin`) rather than corruption.

---

## Checked and found clean (at 331b05c)

- **`sweep_evidence` locks and atomics.** The depth digest mutex is held across `append_row` and its acknowledgement, so digest order equals file order. `failure` → digest are never taken in reverse order. `ranked_published.swap(AcqRel)` blocks double publication. `acknowledged_*` are stored with Release and read with Acquire in `seal`, which owns `self`. `FLUSHED`/`FORGOTTEN`: `if flushed().contains(..)` drops its guard before the block, and `io_error` → `forget_flushed` is never called while FLUSHED is held, so there is no self-deadlock. The epoch check correctly refuses to remember a chain flushed during a concurrent refusal.
- **`sweep_evidence` token allocation.** Tokens come from the `attempts.bin` row count under an exclusive flock, so tokens are unique across processes. A token is never reserved before its journal row is fsynced. A `create_new` reservation refuses reuse. `append_identity_start` refuses an older attempt that lands after a newer one, and Drop writes a Refused terminal.
- **Readers.** `read`/`read_attempt` see only starts that are already in `starts.bin`, which is written after the reservation and lifecycle are durable. `page` refuses a cardinality change instead of mixing snapshots.
- **`results.rs`.**
  - `append` holds an exclusive flock across absorb, duplicate check, seek-to-end, write, rollback and sync.
  - The rollback never fires on a sync-only failure.
  - `read`/`refresh` take a shared lock.
  - `open_with` validates under a dup-fd flock released by name (D-0693).
  - `with_shared_writer` holds one process mutex across the operation. The lock order is always LEDGER → `write.lock` flock → WRITER mutex → `runs.bin` flock, and no path takes them in reverse.
- **`result_set.rs`.** `CommittedParents::refresh` and `committed_receipt_with_limit` read the ledger before the receipts. The writer appends the receipt before the ledger, so a reader never sees a parent without its receipt because of ordering alone. `append_exact` absorbs under an exclusive flock and reuses an identical receipt. A duplicate receipt cannot be produced by two cooperating writers.
- **`frontier.rs` and `trades.rs` append.** Duplicate detection is repeated under the exclusive flock after `absorb_new_rows`, the offset is taken from the length under the lock, a partial write is rolled back, and the sync runs after the rollback arm. Read-only refresh takes a shared lock. `confirm_durable` syncs under a shared lock.
- **`ensure_frontier_rows`/`ensure_trade_rows`/`ensure_run_record`** recover correctly when another writer wins the race between `holds` and the append lock: they reopen and byte-compare.
- **`pool.rs`.** Both passes use indexed `par_iter().collect()`, the union keeps first-seen order through a `Vec` beside its `HashSet`, and `fold` is serial. No `HashMap` order reaches the output. (The `SharedBy` gap in pass 1 is already reported as hunt-conc-8.)
- **`stored.rs`.** It only reads. `BarFile::open_existing` holds a shared `try_lock` for the life of the handle, so a pull in flight is refused by name. `calendar_policy_digest_v2` is a `OnceLock` over compiled-in inputs. The `COST_SCOPE_KEY` thread-local is `cfg(test)` only.
- **`candidate_trades.rs` writer.** The state mutex serializes publication. `facts` is a `OnceLock`. `refuse` ignoring poison is harmless because `check`/`finish`/`confirm` refuse on poison. Paths are per-(identity, attempt token), so two processes cannot collide on one `create_new`. `pinned` refuses a generation change between pages.

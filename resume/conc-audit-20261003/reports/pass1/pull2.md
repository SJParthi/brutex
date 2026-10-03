# pull2 (conc-pass1): crates/pull/src, everything except http.rs, capture.rs and resolve.rs, at 331b05c

**Verdict:** 5 findings. 1 medium, 4 low. The medium one is a census lock that can still silently turn into no lock. Crash recovery of the bar-file/census pair otherwise holds.

All findings were read from source. No cargo was run, as instructed.

## Findings

### pull2-1 (medium): CensusLock::take still runs unlocked on transient and space errors, so concurrent installs can lose census rows

- **Where:** `crates/pull/src/ingest.rs:3118-3130`. The publish temp name is at `ingest.rs:3345`.
- **Code:**
```rust
Err(why) => match fs::symlink_metadata(&lock_path) {
    Ok(found) if !found.is_file() => { return Err(...) }
    _ => return Ok(Self { _held: None }),
},
```
- **Why it is wrong:** The function's own doc says "Every reason that leaves the census writable is an `Err`". The deferral rests on the claim that an existing regular file which still will not open means "the install fails on the same host". That claim does not hold for two kinds of error.
  - **(a) Transient per-process errors:** `EMFILE`/`ENFILE` (too many open files), `ENOMEM`, `EINTR`. `symlink_metadata` succeeds on the existing regular lock file, so the code falls to `_ => Ok(None)`. Later, after other threads close descriptors, `read_census`, the bar appends and `write_appends`/`publish` all succeed.
  - **(b) A lock file that does not exist yet while the census does:** for example, the store was copied or rsynced without dotfiles, or the lock was deleted. If the create fails with `ENOSPC` because inodes are exhausted, `symlink_metadata` returns NotFound, which also gives `Ok(None)`. In-place `write_appends` only needs data blocks, so on an inode-exhausted disk the appends succeed.

  In both cases the run does an unserialised read-modify-write of the census. That is the exact hazard the comment block above describes ("IT BECAME LIVE TODAY ... feeds CONCURRENTLY").
- **Repro (EMFILE, one api process):**
  1. Thread A ingests vendor V. `CensusLock::take` hits EMFILE on the `open`, so it returns `Ok(None)`.
  2. Thread B ingests vendor V and takes the real flock. Nobody else holds it, so the lock succeeds.
  3. Both read census generation g with `n_valid = N`.
  4. Both compute `Append { offset: HEADER_LEN + N*stride, commit: slot (g+1)%2 }`.
  5. Each writes its entry at the same offset, then the same slot. The last writer wins.
  6. The other run's entries are overwritten, yet its receipt says `counted` and it reports no failure.

  The whole-image path (`repairing`, `virgin`, or `upgrading`) is worse. Both runs `File::create` the same `<vendor>.man.writing`, which truncates the other's half-written temp file. A's `rename` can then publish B's partially written bytes, and B's own `rename` fails with ENOENT. The next start loads a degraded or refused census.
- **Fix:** Defer to the install only when `symlink_metadata` fails with `NotFound` or `NotADirectory` **and** the census itself is absent (a first run). For every other open failure, return `Err`. A run that cannot hold the lock must refuse: the rule is "refuse unless provably first-ever", not "defer unless provably misfiled".

### pull2-2 (low): record_all drops every census row in a batch when one row is refused

- **Where:** `crates/pull/src/ingest.rs:1215-1220`.
- **Code:**
```rust
for one in held {
    match count(&mut census, *one) {
        Ok(Some(append)) => appends.push(append),
        Ok(None) => {}
        Err(why) => return Some(why),
    }
}
```
- **Why it is wrong:** The `Held` rows reaching `record_held` come from `from_rows`. That function wrote bars and read the header **outside** the census lock. So a row can be stale against the census by the time it is recorded, for example when a concurrent `from_members_inner` or `from_window` on the same file recorded a larger `rows` value first. A stale row is refused with `RowCountWentBackwards`. The early `return` then discards every other contract's append in the batch, even though those bars are already on disk.

  `from_members_inner` handles the same error per entry and keeps going (`ingest.rs:886-905`). The two doors behave differently for one error.
- **Repro:**
  1. A rolling group files contracts C1..C30 through `from_rows`.
  2. During the group, another ingest of C7's month file appends and records rows=900.
  3. C7's pending `Held` says rows=375.
  4. `record_held` refuses C7 and returns before `install_census`. C1..C6 and C8..C30 stay uncounted.
  5. `fnowork::owed` (a census probe) then reports all 30 as owed from their first day, and the next run refetches 29 months that are already on disk.
- **Fix:** Push the failure onto a list and `continue`. Install the appends that succeeded, then return the joined reasons, as `from_members_inner` does.

### pull2-3 (low): a crash between the cash-master payload and its receipt wedges that day with no path to recover

- **Where:** `crates/pull/src/cash_session_cache.rs:502-507` (install). The refusal is at `cash_session_cache.rs:433-442`.
- **Code:**
```rust
write_new(&payload, bytes)?;
// The receipt is last. ...
write_new(&metadata, receipt(day, bytes).as_bytes())?;
File::open(root).and_then(|directory| directory.sync_all())
```
and in `read_entry`:
```rust
_ => return Err("UNVERIFIED incomplete cash-session cache for {day}: payload and receipt must both exist")
```
- **Why it is wrong:** The payload is created under its final name with `create_new`, and nothing syncs the directory before the receipt is created. Two crash points leave a broken pair:
  - The process dies after `write_new(&payload)` (or during it, leaving a zero-length or partial payload).
  - The power is lost after the receipt's `sync_all` but before the directory `sync_all`, and only the payload's dirent survives.

  Either way the next start finds a payload with no receipt. Every later `prepare`, `prepare_observed` or `prepare_local_observed` that covers that day then refuses. That aborts the whole call, so it blocks every cash-equity minute ingest whose month includes that day. The refusal does not say what to do about it. `install` cannot repair the state either, because it calls `read_entry` first and gets the same error. The code comment calls this deliberate ("cannot become an implicit refetch/overwrite"), but no operator path exists to clear it.
- **Repro:** Kill the api right after `write_new(&payload, bytes)` returns for day D. Every later equity ingest touching D's month then fails with "incomplete cash-session cache for D" until someone deletes the file by hand.
- **Fix (keeps no-overwrite):**
  1. Write the payload to `.<name>.partial`, sync it, then rename it into place.
  2. Write the receipt the same way, then sync the directory.
  3. In `install_and_read`, when a payload exists without a receipt, accept the newly fetched bytes only if they are **byte-equal** to the orphan payload, and write the receipt for it. Otherwise refuse as now.

  At minimum, name the orphan file in the refusal and tell the operator to remove it.

### pull2-4 (low): read-only cache validation takes an exclusive try-lock, so concurrent equity ingests refuse each other

- **Where:** `crates/pull/src/cash_session_cache.rs:270` (`prepare_with`), `:314` (`prepare_observed_with`) and `lock_day` at `:378-396`.
- **Code:**
```rust
let lock = lock_day(root, day)?;      // Flock::try_lock (exclusive, non-blocking)
match read_entry(root, day)? { ... decode(&bytes)? ... }
```
- **Why it is wrong:** The validation pass only reads, but it takes an **exclusive** non-blocking lock and holds it across `decode`, which gunzips and parses up to 32 MiB. A second ingest validating the same day during that window gets `"cash-session cache lock ... unavailable"`, and its whole window fails.

  The api caller (`server.rs:7454-7461`) runs this for every NSE cash minute window. It covers every historical day of the month (`committed_cash_days`) plus the observed days, so two equities pulled at once, in the same months, by two concurrent feeds collide on the same day locks. Nothing is corrupted: the failure is a spurious refusal of a valid run.

  `read_local_lifecycle` already shows the right shape: it uses `try_lock_shared`.
- **Repro:** Feed A (Groww, RELIANCE 2026-09) and feed B (Dhan, TCS 2026-09) both reach `prepare_local_observed` for 2026-09-01. A holds the exclusive lock while it decodes. B's `try_lock` returns WouldBlock, and B's window fails.
- **Fix:** In the two validation loops, take `Flock::try_lock_shared`, or a blocking shared lock inside `spawn_blocking`. Keep the exclusive lock in `install_and_read` only.

### pull2-5 (low): in-place census appends let a lock-free reader see a torn slot and report "degraded"

- **Where:** `crates/pull/src/ingest.rs:3313-3331` (`write_appends`). The readers that take no `.man.lock` are in `api/src/census.rs:303-330`, outside this slice.
- **Code:**
```rust
file.seek(SeekFrom::Start(append.commit.offset))?;
file.write_all(&append.commit.bytes)?;
file.sync_all()?;
```
- **Why it is wrong:** Before the incremental path, every census change was a whole-image `rename`, which is atomic for a lock-free reader. Now the newest header slot is rewritten in place. A reader whose `fs::read` overlaps that `write` can get a slot that is half old and half new. That slot fails its CRC-32C, so `walk_generations` falls back to the other slot and sets `degraded = Some(...)`. `/store.json` and `/audit.json` then render a healthy census as damaged ("needs attention"), and `note_census_load` logs the step-over. The counts the reader gets are one generation stale but correct.

  This needs a read and a write to overlap at byte level. Linux gives no atomicity guarantee for a buffered `read` concurrent with a `write` to the same page, but the window is small. It is a false alarm, not data loss.
- **Repro:** api polls `/store.json` once a second while an ingest appends. The read during the slot write sees a torn slot, and that one response reports the census as degraded.
- **Fix:** Have readers take `.man.lock` shared (`try_lock_shared`, and on WouldBlock report the census as "busy" rather than damaged). Alternatively, treat a CRC failure on the newest slot as "retry the read once" before reporting it as degraded.

## Checked and clean

- **Census read-modify-write in `from_members_inner` and `record_all`:** the lock is taken before `read_census` and dropped after the install (except pull2-1).
- **Crash between bar append and census install:** this case self-heals. On rerun `BarFile::append` returns `AlreadyPresent`, or does a suffix append for a partial overlap, and `write_and_count` re-reads `rows` and the timestamps from the file header. `count` therefore records the true state.
  - For F&O, `fnowork::owed` resumes from the census `last_ts`, and the overlap is absorbed the same way.
  - Derived rungs reconcile against stored history (`reconcile_derived`).
- **`write_appends` ordering:** entry write, `sync_data`, slot write, `sync_all`, so a crash at any line leaves either the old slot or a CRC-failing new slot. `walk_generations` recovers generation g-1 and the next run repairs by whole-image install.
- **`publish`:** write, `sync_all`, rename, then directory `sync_all`. The temp name is serialised by the census lock (except pull2-1).
- **Lock release:** `Flock` unlocks explicitly on Drop (D-0693), so the early `?` returns in `cash_session_cache` do not leak a lock into a forked child.
- **`masters.rs`:** the lock precedes both the `changed` read and the write. The `.partial` temp file is shared but serialised by a blocking flock. Write, `sync_all`, rename and directory sync all run in order, and the uncertain state is surfaced as `Landed::Uncertain`. The cookie jar mutex recovers from poisoning, and `BTreeMap` keeps the header order deterministic.
- **`rate.rs`:** the `ABSORBED_MICROS` Relaxed counter is display-only, and `monotonic_micros` uses a `OnceLock<Instant>`.
- **`from_window`:** the dedup `HashMap` is used only for probes. Rows are pushed in input order, so no iteration order leaks into the output.
- **`work::gaps` and `fnowork::owed`:** state is derived from the census, there is no progress file, and the iteration order is that of the input slice.
- **`cash_session_cache::prepare_*`:** the in-memory map changes only on complete success. A conflicting re-download is refused, not overwritten.

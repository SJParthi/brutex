# conc-pass1 / store1: store bar file writes, append, locking, readers vs writers

**Verdict: the month lock, append ordering and crash recovery hold at 331b05c. I found 2 low-severity findings and no high or medium ones.**

Slice: `crates/store/src/file.rs`, `crates/store/src/flock.rs`, `crates/store/src/emits.rs`. I read the source only and did not run cargo.

Already reported and not repeated here: hunt-store-2 (torn genesis slot), hunt-store-3 (`create_dir_all` recreates the root), hunt-store-4 (only the leaf month directory is fsynced) and hunt-store-10 (`O_NONBLOCK` literals). I re-checked all four at 331b05c and they are still present as described. I have nothing material to add to them.

---

## store1-1 (low): `open_existing` reads with no lock when `.lock` is absent, and a writer that arrives later is not excluded

**Where:** `crates/store/src/file.rs:1400-1407`

```rust
let lock = match open_read(&lock_path) {
    Ok(handle) => Some(
        Flock::try_lock_shared(handle, lock_path.clone())
            .map_err(|refusal| lock_fault(&lock_path, refusal))?,
    ),
    Err(why) if why.is_absent() => None,
    Err(why) => return Err(why.refusal(&lock_path)),
};
```

The justification is at 1395-1399 and in the module header at 53-57: "a bar file with no lock beside it has had no writer since it was written, so there is nothing to wait for". That covers past writers only. It says nothing about a future writer. `open_or_create` (1248) creates `.lock` with `open_rw` (`create(true)`), and its `try_lock` succeeds because the lockless reader holds nothing. The writer then appends while the reader's handle is live. The module header says "a writer holding it exclusively still refuses them all", and in this state neither side refuses the other. `store/src/repair.rs:171` already treats this as unsafe for revisions ("Unlike the legacy opener, revisions must never admit a missing lock").

**Precondition:** a month with `.bin` (and `.crc`) present and no `.lock`. Production never deletes a `.lock` and `open_or_create` creates it before the `.bin`. So this happens only to a store restored or copied without lock files, or to files from a build that predated the lock.

**Repro (two processes):**
1. Process A (api `GET /bars`, or `cli sweep-stored`) calls `open_existing`. The lock is `None`, the header has `n_valid = N`, and N is not a multiple of 73, so the tail block is partial.
2. Process B (pull) calls `open_or_create`. It creates `.lock`, `try_lock` returns Ok, and `append(k bars)` runs. The records are fsynced, `seal_committed` re-seals the tail-block entry over N+k, and the slot is committed.
3. A reads its tail block: `verify_block_of` compares the stored sum (now over N+k) against CRC(N committed). They mismatch. `past_the_commit` `fstat`s and finds B's records "past the commit", and `verify_through` admits them through the D-0688 proof. `block.rs:286` `note_interrupted_append` then writes a `store.block` WARN that names an interrupted append. Nothing was interrupted: the append was live and succeeded. The operator gets a crash diagnosis for a healthy concurrent write, repeated on every alternation (D-1448).

No wrong bar is served, because A's snapshot is a committed prefix. The damage is that the documented reader/writer exclusion is false for this file and the log is misleading.

**Minimal fix:** remove the absent-lock exception in `open_existing` and refuse with a named error, as `repair.rs` does. Or, if read-only stores without locks must keep working, keep reading but say in the doc that a later writer is not excluded, and have `open_or_create` refuse to create a `.lock` beside an existing non-empty `.bin` without an explicit operator step.

---

## store1-2 (low): after a failed `sync_all` in `append`, the page cache can serve uncommitted state, and a later run answers `AlreadyPresent` or commits on top of it

**Where:** `crates/store/src/file.rs:2149-2171`

```rust
write_fully(&self.bars, &self.bars_path, at, &image)?;
fault(self.bars.sync_all(), &self.bars_path, Action::Sync)?;
self.seal_committed(first_index, commit.header.n_valid)?;
write_fully(&self.bars, &self.bars_path, commit.offset, &commit.bytes)?;
fault(self.bars.sync_all(), &self.bars_path, Action::Sync)?;
self.header = commit.header;
```

Also the `AlreadyPresent` return at 2080-2085, which does no write and no sync.

On Linux, a writeback `EIO` reported by `fsync` marks the failed pages clean but leaves their contents in the page cache. A second `fsync` then returns Ok and does not rewrite them ("fsyncgate"). `append` returns the error and leaves the handle's `self.header` unchanged. The pages it wrote stay readable through every later `open`.

**Repro (one line, then a later process):**
1. The process dies, or the caller gives up, after the second `sync_all` (2170) returns `Err(EIO)`. Slot g+1 naming the batch is in the page cache but not durable. If the first `sync_all` (2150) failed instead, the records are in the same state.
2. The next run calls `open_or_create`. `read_header` reads slot g+1 from cache, so `n_valid` includes the batch. Re-offering the same bars hits `advance` → `TimestampsOutOfOrder` → `already_stored` → `Appended::AlreadyPresent`, with no write and no sync. The caller records the window as stored.
3. Either of two outcomes follows:
   - **Power loss.** Slot g+1 is gone and slot g is intact. `claimed == n_valid`, so `validated` sees nothing. The month is silently one batch short after a run reported it present.
   - **The run appends batch g+2.** It commits into slot `g % 2` with a successful fsync. That slot is durable and names the g+1 records, whose pages were marked clean and never reached disk. After a reboot, the block read of those records fails CRC (`seal_committed` read them from cache), so the month is refused as `BlockChecksum` for good, and append-only (§3 rule 8) forbids rewriting it.

**Minimal fix:** after any `sync_all` failure inside `append`, poison the handle so it refuses every later `append` and read. Treat that failure as fatal to the process (PostgreSQL's answer is a PANIC). Document in the module durability table that `AlreadyPresent` is only as durable as the commit that put the bars there, and that it is unproven after a reported sync failure. Making `AlreadyPresent` call `sync_all` on `.bin` and `.crc` before returning is cheap and closes the case where a different process left the dirty pages, but it cannot fix pages the kernel already marked clean.

UNVERIFIED: the page-cache behaviour after writeback `EIO` depends on the filesystem (ext4 and xfs behave as described above). I did not measure it here.

---

## Checked and sound

- **Lock order and scope.** `open_or_create` takes `try_lock` on `.lock` before it opens or measures `.bin` (1248-1255). `open_existing` measures `len` and reads the header only after the shared lock (1410). The audited door takes the lock first (1444). No path takes a second lock while holding the month lock. `flock` locks conflict across open file descriptions, including within one process, so a reader handle and a writer handle in the same process exclude each other.
- **Release.** `Flock` unlocks explicitly in `release` and `Drop` (flock.rs:139-145, 172-180). `held` is cleared before a release, so nothing is reported twice. `_lock` is the last field of `BarFile`, so `.bin` and `.crc` close before the unlock. Every `?` after lock acquisition releases through the guard. No lock is left stale after a crash, because the kernel frees `flock` on process exit.
- **Crash points in a new month.** `initialise` writes zeros, then the slot, then `sync_all`, then `fsync_dir`. A crash at any point leaves a file that is all zeros and `<= REGION_LEN`, which is re-initialised under the lock (except the torn slot in hunt-store-2). The sidecar is created only when `n_valid == 0`, and its directory entry is fsynced (1716-1740).
- **Append order.** Records, fsync, seal, fsync of `.crc`, slot, fsync. A crash at each step is covered by the generation fallback, the `claimed > n_valid` guard and the D-0688 tail proof. The old tail is verified before it is re-sealed (D-0910). The terminal counter refusals do not recurse.
- **Verified-block `Mutex`.** The block is set to `NO_BLOCK` before every fill and set only after a check passes, so recovering from poison is safe. `verify_old_tail_before_reseal` holds the guard and calls `verify_block_of`, which does not lock again, so there is no self-deadlock. The `pread` under the mutex serialises one shared handle only, as documented.
- **`append` takes `&mut self`,** so no read on the same handle runs during a commit.
- **Two writer processes on a new month.** Both `create_dir_all` and both `open_rw(.lock)` (O_CREAT on the same inode). One wins `try_lock` and the other gets `Locked`.
- **A reader during a writer's first create.** The `.bin` cannot exist before the `.lock`, so the reader gets `Missing` or `Locked`.
- **emits.rs** is `cfg(test)` only. Every store unit test that reaches a production emit holds `hold_the_sink`. I listed every `#[test]` in `file.rs` that does not hold it: all of them are pure arithmetic or fake-host tests. `header.rs` and `block.rs` have no unit tests. The refused-release tests in `flock_tests.rs` go through `release()`, which does not emit, and the one `Drop`-refusal test holds the sink.
- **No `rename` or `remove_file` of `.bin`, `.crc` or `.lock`** in production code anywhere in the workspace (grep), so the open-by-path then lock sequence has no swap window.

Out of slice, noted only: `pull/src/ingest.rs:2224` reads the whole minute month under a shared lock that it drops when the closure returns, then folds and appends derived rungs. Another writer can append minute bars between those two steps, so the derived fold can be built from a stale snapshot. I did not analyse it further.

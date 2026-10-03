# conc-pass2 / barflow: end-to-end bar write flow (vendor fetch -> pull::ingest -> rung fold -> store::file append), at 331b05c

**Verdict: 1 finding (0 high, 1 medium, 0 low). The census lock serialises same-vendor ingests, so the ingest.rs:2224 snapshot window raised in pass 1 is not reachable in production. The live defect points the other way: a READER's shared month lock refuses the ingest WRITER, and a derived rung refused that way is never re-derived for a past month.**

Slice: `crates/pull/src/ingest.rs` (`from_window` -> `from_members_inner` -> `one` -> `write_and_count` / `derive_all` / `derive` / `reconcile_derived`), `crates/store/src/file.rs` (`open_or_create`, `open_existing`, `append`, `already_stored`), and the concurrent readers `cli::stored::load` (sweep-stored) and `api::bars` (`/bars.json`, `/bars/window.json`), plus `api::calendar_of`. I read the source only and did not run cargo.

## Flow as built (for reference)

1. `from_members_inner` (ingest.rs:769) takes `CensusLock::take` (exclusive `Flock::try_lock` on `<vendor>.man.lock`) BEFORE any bar is written and holds it until `install_census` after the member loop (ingest.rs:805, 936). Every bar-writing door that derives (`from_window`, `from_dir`, `from_members`) goes through it. So two same-vendor ingests cannot overlap: the second is refused "another ingest holds the census lock".
2. `one` -> `write_and_count` (ingest.rs:2624): `BarFile::open_or_create` (exclusive non-blocking `try_lock` on the month `.lock`, file.rs:1248), `append`, read header + closes, drop (unlock).
3. `derive_all` (ingest.rs:2166): re-opens the minute month with `open_existing` (shared lock, ingest.rs:2224), reads all `n_valid` records, drops the handle when the closure returns.
4. Per derived rung, `derive`: `fold::complete_minutes_with_calendar`, then `reconcile_derived` (`try_exists` + `open_existing` shared on the derived month, compare, drop), then `write_and_count` (exclusive again).
5. `count` + one census install at the end of the run.

`only api calls pull::ingest` (grep: no cli call site), and `serve.lock` excludes a second api process, so every bar writer is in-process and same-vendor writers are serialised by the census lock (see the pass-1 verification of the store1 note). The concurrent parties that remain are READERS: other api threads (`/bars.json`, `/bars/window.json`, `/calendar.json` derivation, browser-launched sweeps in `spawn_blocking`) and other processes (`cli sweep-stored`, `range-all`, `pool`, `fold_audit`, `checksum_receipts`).

## barflow-1 (medium): a reader's shared month lock makes the ingest's non-blocking exclusive `try_lock` fail. The failure is worded as "another writer holds", and a derived rung refused this way is never re-derived for a past month

**Where:**
- crates/store/src/file.rs:1248-1252 (`open_or_create`):
  ```rust
  let lock = Flock::try_lock(
      fault(open_rw(&lock_path), &lock_path, Action::Open)?,
      lock_path.clone(),
  )
  .map_err(|refusal| lock_fault(&lock_path, refusal))?;
  ```
  `lock_fault` maps `TryLockError::WouldBlock` to `StoreError::Locked` (file.rs:3272), which displays as `"another writer holds {}"` (file.rs:867).
- crates/pull/src/ingest.rs:2632-2633 (`write_and_count`, the one door every pulled AND derived bar goes through):
  ```rust
  let mut file =
      BarFile::open_or_create(store_root, path, symbol_id).map_err(|why| why.to_string())?;
  ```
  It makes one attempt, with no wait and no retry.
- Readers hold `open_existing`'s shared lock (file.rs:1400-1405, `Flock::try_lock_shared`) for the life of the handle. The longest hold is crates/api/src/bars.rs:954-1015 (`window`): `/* OPENED ONCE, HELD FOR THE REQUEST. */` It opens up to `MAX_WINDOW_MONTHS = 240` month files at once. With `sort.scans()` or `extremes=1` (server.rs:3327), it then reads every record of every one (`slots(file, 0, held)` per file) while all 240 shared locks stay held. Other readers: `cli::stored::load` (stored.rs:2308, the whole month decoded under the lock), `api::bars::open` per page, and `calendar_of::open_rung` (calendar_of.rs:537), which re-derives over every held index month whenever the manifest stamp moves. Every ingest install moves that stamp.

**Why it is wrong:** `flock` shared and exclusive locks conflict across open file descriptions, in-process too (store1 confirmed this in pass 1). So any reader that has a month open makes the writer's `try_lock` fail. The store's design is "a writer refuses readers" (file.rs module header). The reverse is not designed for, and it is not reported honestly. The error says "another writer holds <month>.lock" when the holder is a reader. That sends the operator looking for a second pull that does not exist. This is the mirror of the misreport that cli/stored.rs:2292-2306 already fixed for the reader side.

The consequence depends on which write is refused:
- **The pulled (minute/day) month:** `one` returns `Err`, the member is "not landed", and the vendor request that was already paid for is thrown away. The autopilot frontier re-reads the manifest's `last_ts_micros` (autopilot.rs:2851-2858) and asks again on a later round, so the bars recover, at vendor-quota cost.
- **A derived month (2min..60min, 1day-from-minute):** `derive` returns `Err`, `derive_all` turns it into a `Failure` (ingest.rs:2304-2310), and `derived_shortfall` reports it. But the MINUTE entry is still counted and installed in the census (ingest.rs:876-907). The autopilot frontier is computed only from the pulled rungs' manifest entries (`RUNGS = [Day1, Minute1]`, autopilot.rs:898. Coarser rungs "are never requested", ladder.rs:133-135). So for any month the minute rung has finished, nothing ever calls `derive_all` for that month again. The derived month stays short until an operator re-pulls that minute month by hand. For the current month, the next day's pull re-derives from the full minute history (ingest.rs:2209-2227), so it heals there. For a backfilled past month it does not.

**Repro (one process, two threads):**
1. The autopilot is backfilling dhan NIFTY 1min for month M (spot index, so derived rungs apply).
2. The operator has the DB page open and requests `/bars/window.json?feed=dhan&exchange=NSE&segment=INDEX&symbol=NIFTY&tf=5min&from=<M-239>&to=<M>&extremes=1`. `window` opens all 240 5min files with shared locks and starts `slots(...)` over every record.
3. During that read, the autopilot's `one` commits M's minute bars (the minute file is not held by the reader), and `derive_all` reads the history. `derive(..., rung=5min)` -> `reconcile_derived` succeeds, because shared and shared are compatible. Then `write_and_count` -> `open_or_create` -> `try_lock` returns `WouldBlock`, which becomes `StoreError::Locked`, and the failure reads `"NIFTY: 5min: another writer holds .../NIFTY/5min/<M>.lock"`.
4. The census install records M's 1min entry as complete. Later passes of the autopilot's `frontier` skip M. `bars/dhan/NSE/INDEX/NIFTY/5min/<M>.bin` keeps its old tail (or never exists) for good. A later `cli sweep-stored dhan NIFTY 5min` over M sweeps the short month, which `data_digest` faithfully names, so nothing flags it.

The cross-process variant is the same: `cli range-all` or `pool` decoding months of the same vendor while the api autopilot writes. Each `load` holds a shared lock across a whole-month decode (stored.rs:2308-2337).

**Minimal fix:**
1. In `write_and_count`, wait for the month lock instead of failing at once. Either use a blocking `Flock::lock` (readers hold it only for a bounded read) or a bounded `try_lock` retry, such as up to 2 s with short sleeps, before refusing. Keep `try_lock` failing fast only when the holder is exclusive. A shared-then-exclusive probe can tell the two apart: if `try_lock_shared` succeeds, the holder is a reader.
2. Word the refusal by holder: "a reader holds" vs "a writer holds".
3. Do not let the census mark a minute month complete for the frontier while one of its derived rungs fell short. Alternatively, add a derived-shortfall re-derive pass that calls `derive_all` from the stored minute month with no vendor fetch. The code already supports this, because `derive_all` resumes from the committed source month.

## Pass-1 verification (findings touching this slice)

- **store1, out-of-slice note ("ingest.rs:2224 reads the minute month under a shared lock it drops ... another writer can append minute bars between those two steps, so the derived fold can be built from a stale snapshot"): REFUTED as a reachable defect, and its consequence is overstated.** (a) Every writer of spot/future minute and derived months is `from_members_inner`, which holds the exclusive `CensusLock` on `<vendor>.man.lock` from before the first bar write until after the install (ingest.rs:805-808, 936). Only the api calls `pull::ingest`, and `serve.lock` permits one api process, so no second writer can append to that minute month inside the window. The other bar writers, `from_rows`, `write_overlay` and `write_greeks`, write option-contract paths, and `derive_all` returns early for options (ingest.rs:2206-2208). (b) Even when the census lock is bypassed (pull2-1 below), a stale snapshot cannot produce wrong derived bytes. Minute months are append-only, so every snapshot is a prefix. `complete_minutes_with_calendar` emits only complete buckets, which are identical across prefixes. `reconcile_derived` compares every stored derived bar byte for byte (ingest.rs:2568-2577), and `BarFile::append` verifies any overlap through `already_stored` / `suffix_that_follows` (file.rs:2083-2112). The worst outcome is a false refusal. A writer whose snapshot is OLDER than a derived tail the other writer already filed hits `matching == false` and returns "historical derived evidence incomplete ... restore complete minute source" (ingest.rs:2568-2571), which misdiagnoses a race as missing source. That outcome needs pull2-1's unlocked run, so it is recorded here as a consequence of pull2-1 rather than counted again.
- **pull2-1 (CensusLock::take runs unlocked on transient and space errors): CONFIRMED.** ingest.rs:3118-3130: when the open fails with any kind other than `PermissionDenied`/`IsADirectory` and `symlink_metadata` succeeds on a regular file (EMFILE, ENFILE, ENOMEM, EINTR, EIO), or fails (ENOSPC/EDQUOT when the lock file does not yet exist), the arm is `_ => return Ok(Self { _held: None })`. The run then writes bars unserialised. This slice adds a consequence: under that bypass, two same-vendor runs reach the reconcile/write window described above and produce the false "historical derived evidence incomplete" refusal.
- **store1-1 (`open_existing` reads with no lock when `.lock` is absent): CONFIRMED.** file.rs:1400-1406 still reads `Err(why) if why.is_absent() => None`, and `open_or_create` creates `.lock` and takes `try_lock` without contention. In this flow, the lockless case arises only for months written before `.lock` existed. Unchanged.
- **store1-2 (a failed `sync_all` in `append` leaves cached state that a later run answers `AlreadyPresent` against): CONFIRMED.** file.rs:2150 and 2170 are `fault(self.bars.sync_all(), ...)?` with no handle poisoning, and `self.header = commit.header` runs only after the second sync. In this flow, `write_and_count` opens a fresh handle per call (ingest.rs:2632), so the next member or run reads the cached slot exactly as store1-2 describes.

## Checked and clean

- **Lock order.** No path holds two month locks at once. The minute handle in `write_and_count` is dropped before `derive_all` opens it shared. The derived handle in `reconcile_derived` is dropped before `write_and_count` opens it exclusive. The census lock is always taken first and the month locks are nested inside it, and no path takes them in the opposite order, so no deadlock cycle exists. Every acquisition is non-blocking anyway.
- **Overlap and idempotence.** A rerun offers bars the month already holds, which come back `AlreadyPresent` with 0 written. A partial overlap recurses on the verified suffix. A conflicting restatement is refused. `reconcile_derived` re-offers the stored tail on a no-op (`bars.push(last)`), which also comes back `AlreadyPresent`.
- **Crash between the minute commit and derived writes or census install.** The minute bars are durable and the census does not count them. The next autopilot pass sees the manifest short, re-asks, gets `AlreadyPresent`, re-derives from the full stored month (ingest.rs:2209-2227), and counts it. This self-heals as long as the vendor still serves the window.
- **Mixed-rung reads mid-flow (cli sweep-stored).** The signal rung is loaded before `load_exact_minute_context` (stored.rs:3288-3297). Derived months are always written after their minute month and minute months only grow, so the minute span a sweep reads is always a superset of what its signal span was derived from. A reader cannot see a derived bucket without its source minutes.
- **What readers see mid-flow.** A reader that opens a month while it is being written gets `Locked`. cli words this correctly ("a pull is in flight", stored.rs:2309-2316). A reader that opens between batches sees a committed prefix: the header slot is written after the records are fsynced.
- **Census vs file during a run.** The census is installed once at the end, so mid-run a reader of `/store` sees older row counts than the files. No reader cross-checks census rows against `n_valid` and refuses, so this produces no false refusal. Census correctness under concurrent installs belongs to pull2-1, pull2-5 and xcut-2.
- **`from_window` dedup.** It uses a `HashMap` only for lookups. Output rows keep vendor order, so iteration order cannot reach the bytes.

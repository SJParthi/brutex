# conc-pass2 / equity: equity ingest (cash_session_cache, masters, fnowork/work resume, mastersrun + reload, F&O universe walks)

Verdict: 2 findings (0 high, 1 medium, 1 low). The medium one is a resume hole: a cash-schedule refusal on one chunk, followed by a landing on the next chunk, leaves a permanent gap that the resume logic treats as complete and the store will not let anyone backfill. Commit 331b05c. Everything below comes from reading the source. Nothing was built or run.

## equity-1 (medium): a cash-schedule refusal on an earlier chunk, then a landing on a later chunk in the same month, leaves a hole that nothing will ever refill

- **Where:**
  - `crates/api/src/server.rs:7586-7610` (`land_spot`)
  - `crates/pull/src/session.rs:1218-1258` (`split_window`; chunks are cap-sized, not month-aligned)
  - `crates/api/src/autopilot.rs:762-777` (`next_window` resumes from the last held timestamp)
  - `crates/store/src/file.rs:2112-2121` (`append` refuses unstored bars that are earlier than the tail)
- **Code:**
```rust
for bodies in landed.bodies.chunks(1) {
    match prepare_cash_schedule(landed, bodies, instrument, site, dated).await {
        Ok(schedule) => done.absorb(land_bodies_observed(landed, site, schedule.as_ref(), bodies, ...)),
        Err(why) => { /* telemetry error + one Failure, then the loop CONTINUES */ }
    }
}
```
```rust
// next_window
let resume = match held(&one.at(month)).and_then(day_of) {
    None => from,
    Some(day) if day < to => day.succ() ...,
    Some(_) => { done += 1; continue; }   // "held through the last day"
};
```
- **Why it is wrong:**
  - `fetch_chunks` deliberately lands only a prefix (`prefix_or_refusal`), so a *fetch* failure never leaves a hole. The *landing* stage does not follow that rule. Each body's `prepare_cash_schedule` can refuse independently, and `land_spot` carries on to the next body.
  - Since D-1370, `split_window` chunks by the vendor cap and not by month. Groww's 1-minute cap is 30 days (`vendor.rs:4766`), so an autopilot month unit for a 31-day month is two bodies, `[1..30]` and `[31]`. A hand `/pull/spot` window is split at arbitrary days.
  - Suppose body 1 is refused and body 2 lands. The month file then holds only its tail days. The census `last_ts` points past the hole, and everything that resumes reads that stamp:
    - `next_window` (and `fnowork::owed` for the same shape on F&O) resumes after it, and counts the month `done` when the tail day is the month's last.
    - A manual re-pull of the missing days is refused. `BarFile::append` only accepts bars at or before `last_ts_micros` when they are already stored ("earlier than the last held is NOT the same claim as already held"), so the batch hits the `StoreError::Format` refusal.
  - The only repair is deleting the month file by hand.
  - Body 1's refusal is easy to trigger from this slice:
    - (a) the exclusive per-day `try_lock` collision of pull2-4 / equity-2;
    - (b) a transient NSE archive failure in `prepare_observed`'s download (30 s timeout, no retry);
    - (c) the pull2-3 orphan payload for a day covered by body 1 only.
  - Body 2 does not need body 1's days. `observed_cash_days` covers only the body's own rows, and `committed_cash_days` of a month with nothing committed is empty, so body 2 passes.
- **Repro:**
  1. Groww autopilot tick for `RELIANCE` 2026-08, with an empty month file. The unit window is 2026-08-01..=2026-08-31 and `split_window(.., Some(30))` gives `[08-01..08-30]` and `[08-31]`.
  2. While body 1 runs `prepare_observed`, the NSE archive times out on `NSE_CM_security_14082026.csv.gz`. Alternatively, a concurrent `/gaps.json` on `NSE-TCS` holds `.NSE_CM_security_14082026.csv.gz.lock` (equity-2). Body 1 is recorded as "cash schedule refused".
  3. Body 2 (Monday 2026-08-31) prepares its single day and lands 375 bars. The census entry for 2026-08 now has `last_ts` = 2026-08-31 15:29.
  4. The next tick: `next_window` sees `day == to`, so the month is `done` and the autopilot moves on to September. August 3-28 is missing for good.
  5. The operator presses Pull for 2026-08-01..=2026-08-30. `append` sees the batch is entirely `<= last_ts` and not stored, and refuses it as a format conflict.
- **Minimal fix:** Make landing prefix-only, the same as fetching: in `land_spot`, `break` out of the body loop on the first `prepare_cash_schedule` `Err` and record the remaining bodies as not landed. If later bodies should still land, record an explicit "owed from" floor and have `next_window`/`owed` honour it. A hole must never sit behind a later tail.

## equity-2 (low; pull2-4 missed the read-only triggers): a `/gaps.json` GET or the recovery auditor takes the same exclusive non-blocking per-day lock as ingest, so browsing refuses a live equity ingest (and vice versa)

- **Where:**
  - `crates/pull/src/cash_session_cache.rs:378-396` (`lock_day` → `Flock::try_lock`), reached through `prepare_observed_with:314`
  - `crates/api/src/server.rs:2673` (`audit_span` → `audit_cash_schedule`) and `server.rs:3072` (`audit_cash_schedule_window` → `prepare_local_observed`)
  - `server.rs:3093` (`recovery_cash_schedule`, the same path)
  - `crates/api/src/recovery.rs:860` (`read_local_lifecycle`, `try_lock_shared`)
- **Code:**
```rust
// audit_cash_schedule_window: a READ-ONLY page
pull::cash_session_cache::prepare_local_observed(&site.store_root.join("session-masters"), &days, &mut dated).await?;
// -> prepare_observed_with
let lock = lock_day(root, day)?;          // exclusive try_lock, held across decode (gunzip <= 32 MiB + CSV parse)
```
- **Why it is wrong:** pull2-4 named only ingest-against-ingest. The wider trigger is the gap page. `GET /gaps.json` for any NSE cash instrument walks every full session day of each month in the span and takes each day's lock **exclusively and without blocking**, holding it while it decompresses and parses the master. The recovery auditor does the same. Any equity ingest of any symbol that validates or installs the same day during that window gets `cash-session cache lock … unavailable`, and its body is refused. Combined with equity-1, that refusal becomes a permanent hole.
  - The collision runs both ways:
    - the GET's `evidence_error` reports a spurious failure while an ingest decodes;
    - `read_local_lifecycle`'s `try_lock_shared` (recovery) fails against either exclusive holder;
    - `install_and_read`'s exclusive `try_lock` also refuses two ingests that both downloaded the same missing day, instead of the second one waiting and finding byte-equal bytes.
  - Nothing is corrupted. The defect is spurious refusals, and a refusal on the write path triggers equity-1.
- **Repro:**
  1. The autopilot is landing Groww `INFY` 2026-09 minute bars.
  2. The operator opens the gap page for `NSE-TCS` over 2026-09. `audit_span` (calendar pool) holds `.NSE_CM_security_01092026.csv.gz.lock` while it decodes.
  3. `INFY` body 1's `prepare_observed` reaches 2026-09-01 and `try_lock` returns `EWOULDBLOCK`. The body is refused as "cash schedule refused".
- **Minimal fix:** In `prepare_observed_with` and `prepare_with`'s validation loops, take `Flock::try_lock_shared`, or better a blocking shared lock inside `spawn_blocking`, since the critical section is bounded local I/O. In `install_and_read`, take a *blocking* exclusive lock, so a second installer waits and then finds byte-equal bytes, which `install_and_read` already accepts.

## Pass-1 verification (findings touching this slice)

| Pass-1 ID | Status | Reason (from the code at 331b05c) |
|---|---|---|
| pull2-2 (record_all drops a batch on one refused row) | CONFIRMED | `ingest.rs:1214-1219` still `return Some(why)` inside the loop before `install_census`. The consequence for `fnowork::owed` is only a refetch: the bars are on disk, `append` answers `AlreadyPresent`, and the next count heals. It is not a hole. |
| pull2-3 (payload without receipt wedges a day) | CONFIRMED, with a variant | `install_and_read:502-507` creates the payload with `create_new` under its final name, then the receipt, and syncs the directory only after both. `read_entry:433-442` refuses `(true,false)` permanently. Variant pull2-3 did not name: a crash or ENOSPC *during* the receipt's `write_all` leaves a torn receipt. `read_entry` then refuses it at 447-452 ("receipt … mismatch; retained"), which is the same wedge by a different arm. It also feeds equity-1 as trigger (c). |
| pull2-4 (exclusive try_lock in read-only validation) | CONFIRMED | `prepare_with:270` and `prepare_observed_with:314` call `lock_day` → `Flock::try_lock` (exclusive) and hold it across `decode`. The read-only page and auditor triggers pull2-4 missed are reported above as equity-2. |
| pull2 "checked clean: masters.rs" | CONFIRMED | `land_validated:1140-1160` takes the blocking `Flock` before both the `changed` read and `replace_locked`. `replace_locked:871-929` writes, `sync_all`s, renames, then syncs the directory, and maps a post-rename failure to `Landed::Uncertain`. The `.partial` is removed on every failure arm and is truncated on reuse under the lock. |
| pull2 "checked clean: work::gaps / fnowork::owed" | CONFIRMED for the functions themselves | They are pure functions of the census probe and input order, with no progress file. Their *inputs* can be made wrong by equity-1 (a hole behind a later tail), which is a defect in the landing loop, not in these functions. |
| runs "mastersrun clean" / errpaths REFRESH note | CONFIRMED | `REFRESH` (`tokio::sync::Mutex`, mastersrun.rs:40,514) covers fetch, land and reload FIFO. `Site::reparse` (server.rs:5439-5501) takes `reload_lock`, scopes the `universe()` read guard out before `parsed.write()`, and recovers write poisoning. No second `universe()` read is taken while a guard is held on the paths at server.rs 1195, 1439, 13682, 32221 and 32840. The 1195 site is documented as fixed and I re-checked it: `census_now` does not re-enter `universe()`. |

## Checked and clean

- **`cash_session_cache` in-memory map:** `prepare_observed_with` changes `cache` only on complete success. The `HashMap`s are used only for keyed lookup, so no iteration order reaches output.
- **`cash_session_cache` lock release:** `Flock` unlocks explicitly on Drop (D-0693), so early `?` returns do not leak a lock.
- **`cash_session_cache` validate-then-fetch TOCTOU:** `install_and_read` re-locks and re-runs `read_entry`, accepting a byte-equal existing entry and refusing a different one. It is never an overwrite.
- **masters across two `api serve` processes on different stores** (the masters dir is shared via `BRUTEX_MASTERS`/HOME): the persistent `.FILE.lock` flock serialises them, and the rename makes each file atomic for lock-free readers.
- **Mid-walk universe reload:** each instrument resolves its vendor ID against the current generation (server.rs:10360) and refuses per instrument if unmapped. The refusal is loud, not silent.
- **F&O universe walks:**
  - `spot_targets` sorts its targets by the full `InstrumentKey`;
  - `ladder`/`cells` follow input order;
  - `pull::universe` iterates the master slice, not its hash indices.
  
  No `HashMap` order reaches output or identity.
- **`fno_land`:** it reads the census once per run. That snapshot can only be older than the store, and an older snapshot only causes overlap, which `append` verifies byte for byte.

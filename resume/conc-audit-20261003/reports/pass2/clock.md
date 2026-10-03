# conc-pass2 / clock: wall clock vs monotonic, clock steps, IST day and session boundaries

**Verdict: 5 findings (0 high, 2 medium, 3 low).** Monotonic timing (rate governor, tokio timeouts, sleeps, elapsed measurements) is correct throughout. The defects are in places where the wall clock decides something that persists or gates writes: the autopilot's feed set is frozen at the first clock reading, the "session has closed" gate trusts the wall clock with no margin, backoff deadlines are wall-clock deadlines, the telemetry clamp freezes stamps after a forward step, and the masters "newer than parse" test orders two wall-clock readings.

Commit 331b05c. Slice: workspace non-test code, every `SystemTime`, `Instant`, `modified()`, `ist_day`/`ist_moment`/`today_ist` site. I read the source only and did not run cargo.

**Realistic trigger used below (T-IST).** An Indian host whose RTC holds local time while Linux reads it as UTC (the usual dual-boot setup) boots with the wall clock **5h30m ahead**. NTP then steps it **back** 5h30m. That is one forward-skewed interval followed by one backward step, and it needs no unusual hardware.

---

## clock-1 (medium): the autopilot's feed set is derived once from the boot-time clock, so a wrong clock at boot drops fixed-floor feeds for the life of the process

**Where:** crates/api/src/autopilot.rs:2549-2551 (pre-loop clock read), 2613, 2491-2502, 2468-2482, 3189.

```rust
let yesterday = loop {
    if let Some(day) = yesterday_ist(std::time::SystemTime::now()) { break day; }
...
let mut feeds = drivable(yesterday);          // 2613, once, never re-derived
```
```rust
pub fn drivable(yesterday: Day) -> Vec<FeedState> {
    ... .filter_map(|feed| {
            let vendor = feed.store_vendor()?;
            let floor = floor_day(feed, yesterday)?;   // None => feed silently left out
```
`floor_day` calls `clamp_to_floor(Window(epoch, yesterday), spec.history_floor, today)`. For a `Fixed` floor it refuses when `window.to() < oldest` (server.rs:8774). Groww is `Fixed 2020-01-01` (pull/src/vendor.rs:4734) and Zerodha is `Fixed 2015-01-01` (vendor.rs:5419).

**Why it is wrong.** `round` re-reads `yesterday` on every pass (autopilot.rs:3175), but the `feeds` vector it walks was built once from the first usable reading. The clock-wait loop only rejects a clock that cannot name a day (before 1970-01-02 IST or after 9999). A clock that is valid but wrong is accepted, and every feed whose fixed floor is later than that wrong date is dropped from `feeds` with nothing said. Rolling floors (Dhan) survive because they move with the wrong `today`.

**Repro.**
1. The host boots with its RTC at a default such as 2000-01-01, or any date before 2015. `yesterday_ist` returns 1999-12-31, so the clock-wait loop is skipped.
2. `drivable` evaluates Groww: `Window(1970-01-01, 1999-12-31)` against floor 2020-01-01 returns Err, then `.ok()?` drops Groww. Zerodha is dropped the same way. Dhan is kept, with its frontier at about 1995-01.
3. NTP corrects the clock a few seconds later. Every later `round` surveys only Dhan. Groww and Zerodha are never backfilled, never appear in `status.feeds`, and nothing on the page names them, until the server is restarted.
4. If Dhan is also unusable (for example, no credential so it halts), `feeds.iter().all(|f| f.halted.is_some())` is true. With an empty `feeds` it is vacuously true, so the page says "every feed is halted. The reasons are below" with nothing below. `admit_resume` (autopilot.rs:3680-3691) then refuses Resume with "it has returned", while the task is alive and looping.

**Minimal fix.** Re-derive the feed set when `yesterday` changes. Keep `FeedState`s keyed by feed, and in `round` add any feed that `drivable(yesterday)` now admits and the vector lacks. Alternatively, keep every HTTP feed in `feeds` and carry an explicit "floor not reachable from today's clock" halt that `survey` clears once `floor_day` answers. Either way, a dropped feed must be named on the page rather than missing.

---

## clock-2 (medium): "today's session has closed" is decided by the wall clock alone with zero margin, so a forward-skewed clock admits a partial day into the append-only store

**Where:** crates/api/src/server.rs:8203-8254 (`finished_day_only`, called per instrument from `broker_window` at 10246).

```rust
let now = ingest::ist_moment(std::time::SystemTime::now())...;
let today = now.day();
let closed = pull::vendor::Venue::NseCash
    .hours_on(today)
    .map_or(true, |session| now.minute_of_day() >= session.close_minute());
if asked.window.to() > today || (asked.window.to() == today && !closed) { return Err(...) }
```

**Why it is wrong.** The function's own comment states the rule: "A partial day must never be stored: the store is append-only, so a half session written now can never be completed". The only evidence that the session is over is the host wall clock. There is no margin past `close_minute()` and no cross-check against the data itself, such as whether the last returned bar reaches the close minute. Any forward error in the clock, of any size up to a day, is converted directly into a write of a running session.

**Repro (T-IST).** The host boots 5h30m ahead and NTP has not yet stepped it back, or the clock was never synced. At real 10:05 IST the clock reads 15:35 IST. An operator POSTs `/pull/spot` with `to = today`. `closed` is true, so the guard passes. The vendor returns 09:15..10:04 bars, and they are appended to `bars/<vendor>/.../<month>.bin`. The manifest's `last_ts_micros` now sits at 10:04 of a day the store believes is settled. A later pull resumes after 10:04 and can only append a suffix. If the vendor's 10:04 candle was still forming when it was fetched, the byte-for-byte overlap check refuses every later attempt to complete that day. (Whether a given vendor returns a forming candle is UNVERIFIED and is not claimed here. The partial write itself is certain.) The same zero margin applies with a correct clock: at exactly `close_minute`, the last minute's bar has not had any time to finalize.

**Minimal fix.** Require a stated settle margin past the close (for example, the next IST day, or close + N minutes with N recorded in `docs/05-decisions.md`). Refuse when the newest bar returned for `today` is earlier than the session's last minute. The second check is data-side, so it does not trust the clock. The autopilot path is unaffected because it never asks for today (`yesterday_ist`).

---

## clock-3 (low): autopilot backoff deadlines are wall-clock deadlines, so a clock step compresses or stretches them, against their documented bounds

**Where:**
- autopilot.rs:1464 `let due = now_unix.saturating_sub(stall.at_unix) >= STALL_RECHECK_SECS;` with `at_unix` stamped from `ingest::epoch_secs(SystemTime::now())` (3204, 1452, 1479).
- autopilot.rs:2917 `let now = ingest::epoch_secs(std::time::SystemTime::now());`, 2945 `due_unix: now.saturating_add(...)`, 548 `if now_unix < probe.due_unix { return Due::Later {..} }`.

**Why it is wrong.** Both bounds are stated as elapsed-time guarantees. One is "the wait is always at least `STALL_RECHECK_SECS` and never less" (1440, 158-163). The other is "re-checked 8 times over two hours and three minutes" (540) before the allowance is spent for good. Both are measured as differences of `SystemTime` readings, which move with clock steps.

**Repro A (compression, T-IST in reverse).** A host that boots BEHIND (RTC lost time) and is stepped forward by NTP. Before the step, a store halt arms a probe with `due_unix = now + 60`. After a forward step of hours, every later pass sees `now >= due_unix`. The 8 probes then run on 8 consecutive idle passes about 60 s apart (IDLE_POLL_SECS), instead of being spread across 2h03m. `store_due` answers `Spent` after about 8 minutes. The feed stays halted "until ... the server is restarted" (543) even though the disk recovered an hour later. Stall reconsiderations are compressed the same way.

**Repro B (stretch, T-IST).** A stall or probe is stamped during the 5h30m-ahead interval. NTP steps the clock back. `now_unix - at_unix` is negative for 5h30m, so the recheck is delayed by the size of the step. A step of days (a VM restored from a snapshot) delays it by days.

**Minimal fix.** Keep `due`/`at` as `std::time::Instant` (or `tokio::time::Instant`) in `Probe` and `Stall`, and derive the epoch-seconds fields only for display. `Instant` cannot step.

---

## clock-4 (low): the telemetry `ms` clamp turns a forward clock step into frozen timestamps, and carries the freeze across restarts

**Where:** crates/telemetry/src/sink.rs:579-582, 746 (`resume_point` floor, D-1325).

```rust
fn stamp(&mut self, now: i64) -> i64 {
    self.last_at = now.max(self.last_at);
    self.last_at
}
```

**Why it is wrong.** The clamp was added for a backward step, and it handles that case correctly. Its mirror case is a forward step that is later corrected. After that, every event carries the forward reading until real time catches up. `ts` is derived from `ms` (clock.rs header), so the human-readable time is equally wrong. Because the floor is resumed from the last line on disk, restarting after NTP has fixed the clock does not clear it. No code path notes that the clamp is engaged, so a page of identical stamps looks like a burst of events in one millisecond.

**Repro (T-IST).** The api server starts at boot with the clock 5h30m ahead and writes events stamped T+5h30m. NTP steps back. For the next 5h30m of real time, every `/logs` row carries the same `ms`, equal to the last pre-step stamp, and the same `ts`. A restart in that window resumes `last_at` from disk and keeps the freeze. Requests like "events in the last hour" and per-event timing on `/logs` are meaningless for 5.5 hours. (The test `a_restart_behind_the_last_stamp_keeps_ms_in_file_order` pins exactly this behaviour as intended.)

**Minimal fix.** Keep the order guarantee without freezing time. Stamp `max(now, last_at)` only when `last_at - now` is under a small bound (for example, 2 s, which covers scheduling inversions). When the gap is larger, write the real `now`, and either emit one `Warn` "clock stepped back by X ms" record or set a flag, and make `tail`'s `since` early exit tolerate that record (treat it as a barrier and keep walking). At minimum, add a `clamped_by_ms` field when the clamp engages, so the frozen stamps are visible as such.

---

## clock-5 (low): `/masters/status.json` decides "restart required" by ordering a file mtime against a wall-clock parse time

**Where:** crates/api/src/mastersrun.rs:687-691, with `parsed_at` from server.rs:5446 / 5559.

```rust
let newer = held.as_ref().and_then(|m| m.modified().ok())
    .is_some_and(|at| at > parsed_at);
```

**Why it is wrong.** `parsed_at` is `SystemTime::now()` at parse time. `modified` is the filesystem's wall clock at write time. Comparing the two by order is only valid if the clock never stepped backward in between. `restart_required` is the flag the page relies on to tell the operator that the process is answering from old masters. Its doc (mastersrun.rs:693-697) warns that a stale master "resolves every renamed symbol to the old row".

**Repro (T-IST).** The server starts while the clock is 5h30m ahead, so `parsed_at` is T+5h30m. NTP steps back. Within the next 5h30m, new masters land through `POST /masters/refresh` with a reparse that is refused (refusal paths at server.rs:5459-5484 keep the old `at`), or through any out-of-band download into the masters directory. The files' mtime is real time, which is earlier than `parsed_at`, so `newer_than_parse` is false and `restart_required` is false. The page reports that the served universe is current while it is not.

**Minimal fix.** Record each master's `(dev, ino, len, mtime, ctime)` at parse time and report `changed_since_parse` as inequality, not ordering. That is the same generation-equality rule `cli::live::LiveStamp` and the census cache already use.

---

## Pass-1 verification (findings touching this slice)

| Pass-1 ID | Verdict | Reason from the code at 331b05c |
|---|---|---|
| autopilot-3 (low): Resume refused as "the task has returned" during the clock wait | **CONFIRMED** | autopilot.rs:2560-2572 publishes `Phase::Halted` with `status.feeds` still empty (feeds are first published by `round`). `admit_resume` 3680-3691 maps `feeds.is_empty() && phase == Halted` to a refusal saying the task "has returned". The loop is alive and sleeping `IDLE_POLL_SECS`. clock-1 step 4 reaches the same false text by a second route (an empty `feeds` after the loop). |
| telemetry-1 repro 3 (medium, `since` truncation through two cli writers) | **CONFIRMED** (clock-ordering part) | sink.rs has no `flock`/`try_lock` on the directory (the only `try_lock` at 1792 is a test of the in-process mutex). `stamp` orders only within one process's `Inner`, and tail.rs:625 ends the walk at the first older record. |
| hunt-conc row "screen budget calibration" (non-finding) | **CONFIRMED** | lib.rs:10098-10160 times with `Instant`. Every recording path refuses `BRUTEX_SCREEN_BUDGET_MS` (sweeprun.rs:901, 1078, 2779, 3310; `SCREEN_BUDGET_NOT_RECORDABLE` at lib.rs:10239). |
| hunt-conc row "`finished_micros` in the run record" (non-finding) | **CONFIRMED** | lib.rs:17438 is a display stamp outside `RunId`. |

## Checked and found sound

- `pull::rate` governor: `monotonic_micros` uses a `OnceLock<Instant>` (rate.rs:1302). All tokio timeouts and sleeps (server.rs:16905-17002 head-read deadline, `nap`, `grace`, `dwell_paused`) are monotonic. `cli::readonly_file` polls with `Instant`, and `operation_audit` uses `Instant` for elapsed time.
- `pull::ssm::now_stamp`: SigV4 needs the wall clock, and a pre-epoch clock is refused by name. `pull::capture::process_stamp` is a hint only, because `create_new` is the authority.
- IST day arithmetic: `IstMoment::from_epoch_secs` refuses pre-epoch, `indicators::ist_day` uses `div_euclid`, and `api::bars::ist_day` renders out-of-range values as "—". The midnight straddles in `feeds_json` (one read), `pull_spot` (one `now` for gate and record) and `fno_walk` (one `today` for the walk) are handled. The per-instrument `finished_day_only` re-read can only become stricter as time moves forward.
- `yesterday` re-derived at the end of a tick (autopilot.rs:3489): a midnight crossing only makes one more day owed, which the next pass fetches. It does not count as a failed attempt.
- `recovery.rs` re-parses journaled bodies against a fresh `today_ist()`. After a backward step a future-dated window is refused loudly (`WindowInFuture`), not silently re-shaped.
- `cli::live`: freshness answers `Unknown` for an mtime after `now`, and the census cache keys on `(len, mtime, created, dev, ino, ctime, ctime_nsec)`.
- Calendar and census caches (server.rs `manifest_stamps`, calendar_of.rs `kept`): equality keys, so a backward step produces a different key and a re-read. A rewrite that keeps the stamp is served stale, which is a stated limit pinned by AF-23 (D-0686).
- `api::assets::Build::Stale`: advisory banner only.
- `execution_lease`: flock-based, with no time component. No time-based lease or staleness steal exists anywhere in the workspace.
- `sweep_evidence::now`, `sweeprun::now_micros`, audit `at`: display and journal stamps. No reader orders or selects records by them.
- `telemetry::lib::sweep_stale_scratch_in` is test-only.

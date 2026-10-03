Verdict: 2 findings in the interplay between recovery and the autopilot at 331b05c (1 medium, 1 low). The main one widens pass-1's recovery-1 from a narrow race into the default case. All 10 pass-1 findings that touch this slice are CONFIRMED. None is refuted.

Slice: how `crates/api/src/recovery.rs`, `recovery_control.rs` and `recovery_journal.rs` interact with `crates/api/src/autopilot.rs`, through what they share in `server.rs`: `site.run`, `site.recovery_active`, the per-feed seat mask, the autopilot stop epoch read inside `broker_run`, `audit/pull.journal`, and the boot order. Method: read the source only. No cargo was run.

## recauto-1 (medium): any recovery started while the autopilot is mid-tick is BLOCKED on its first gap day and loses an attempt from that day's budget; three tries exhaust the day with no vendor request ever sent

This is the same mechanism as pass-1's recovery-1. Pass-1 named the autopilot only as a narrow race: it checks `site.run`, then takes every seat, and a recovery can claim in between. That missed the main trigger. The autopilot never claims `site.run`, so recovery's `claim` does not exclude a tick that has already started. A tick holds every feed's seat for minutes. During an active backfill, ticks run back to back. So the collision is the ordinary outcome of pressing Start on recovery while the backfill runs, the backfill being on by default (`Control::serving`). No unlucky timing is needed.

**Where**
- crates/api/src/autopilot.rs:3162-3174 (`round`). The autopilot only reads `site.run`; it never claims it. Then it takes all seats and holds `_seats` through `tick` and `settle`:
  ```rust
  let pressing = site.run.lock()...is_some_and(crate::pullrun::Progress::running);
  if pressing { return stand_off(site, "a hand-made pull is running"); }
  let Some(_seats) = site.autopilot.take_every_seat() else { ... };
  ```
  The lock is `std::sync::Mutex`, taken for the read alone. `take_every_seat` is `compare_exchange(0, ALL_SEATS)` (autopilot.rs:2129-2134), so a tick on any feed (Dhan, Groww, ...) also holds the Zerodha bit that recovery needs.
- crates/api/src/recovery.rs:560-576 (`claim`). It checks only `site.run`, never the seat mask:
  ```rust
  if held.as_ref().is_some_and(Progress::running) { return Err(...); }
  ...
  *held = Some(Progress::claimed());
  ```
- crates/api/src/recovery.rs:1547-1552 (`retry_day`). The reservation is made durable first, then the seat is tried with no wait:
  ```rust
  if !reserve(&mut item) { break; }            // attempts += 1, InFlight
  append_attempt(journal, attempts, item.clone())?;   // synced to attempts.bin (shared) + plan
  let asked = checked(&item.body, ...)?;
  let run = server::recovery_spot(site, &asked).await?;
  ```
- crates/api/src/server.rs:8022-8025 (`recovery_spot`):
  ```rust
  let _seat = site.autopilot.take_seat(asked.feed)
      .ok_or_else(|| "the selected feed already has an active pull".to_owned())?;
  ```
- What the next run does with the charged attempt (recovery.rs:1103-1131, `reassessed`): `next.status = ... else if next.attempts >= ATTEMPT_LIMIT { Exhausted } else { Queued }`, with `attempts` left as it was. `ATTEMPT_LIMIT = 3` (recovery.rs:26). `refresh_owed` only re-queues `Verified`. So once a day is `Exhausted`, it stays `Exhausted`.

**Why it is wrong.** The two drivers coordinate in one direction only. The autopilot backs off from recovery, at its next round. Recovery does not back off from the autopilot. `pullrun::conduct` handles a busy seat by sleeping `RETRY_WAIT` and trying again; pullrun.rs:647 says "409 stays retryable for a busy feed seat". Recovery has no such retry. It makes the attempt durable before the seat is taken. It then treats the refused seat as a fatal `Err`, and `drive` seals the plan `Blocked` (recovery.rs:821-827). A Blocked plan is not resumed at boot (`resume` returns `Ok(false)` unless control is `InFlight`, recovery.rs:713-719). The operator has to start it again by hand.

Each manual restart charges one more unit. While recovery is BLOCKED, `site.run` is no longer running. The autopilot's `settle` returns 0 for `Advance` and `Retry`, so it starts its next minutes-long tick at once. The operator's next Start, seconds later, meets a held seat again.

**Repro (exact interleaving)**
1. `api serve` with the default autopilot. Wait for `/autopilot.json` to show `phase: running` (a tick on any feed, `now.index` climbing).
2. POST `/pull/recovery` with a Zerodha leg whose window contains an owed day D. `claim` succeeds because `site.run` is idle: the autopilot never claimed it. `seeded` and `drive` run, then `execute` → `load_lifecycle` → `reconcile_pending` → `assess` (all within seconds). `assess` returns `retry_days = [D]`.
3. `retry_day`: `reserve` sets D to attempts=1, InFlight. `append_attempt` syncs that to `audit/recovery-v1/attempts.bin` and the plan. `recovery_spot` → `take_seat(Zerodha)` returns `None`, because the autopilot's `AllSeats` is live. `Err("the selected feed already has an active pull")` propagates. `drive` appends control `Blocked`, and the page shows "Recovery BLOCKED: the selected feed already has an active pull …". No vendor request was made.
4. The autopilot finishes its tick and starts the next one at once, since `site.run` is not running.
5. The operator presses Start again. `reconcile_pending` reassesses D from InFlight to Queued with attempts=1, then step 3 repeats: attempts=2. Next time attempts=3. On the fourth Start, `reserve` sees `attempts >= ATTEMPT_LIMIT` and marks D `Exhausted` (recovery.rs:105-108). The shared budget is never refunded, so D can never be recovered under any plan, even though the vendor was never asked once.

**Minimal fix.** Make recovery acquire the seat the way a press does, and do it before any reservation:
- (a) In `claim` (or right after it), bump the autopilot's stop generation without setting `paused` (a new `Control::yield_to_press()` doing `epoch.fetch_add(1, Release)`). An in-flight tick then stops at its next instrument (server.rs:7840). `observe` sees `stopped` and returns `Retry` (autopilot.rs:1353). The next `round` sees `pressing` and stands off.
- (b) In `retry_day`, take `site.autopilot.take_seat(Feed::Zerodha)` before `reserve`/`append_attempt`. While it returns `None`, sleep one second, re-checking `stopping(site)`, up to a bound. Then pass the held `Seat` into `recovery_spot` instead of letting it take its own. If the seat never frees, return without reserving. Fix (b) alone also closes recovery-1's other triggers: a hand `/pull/spot`, and the expired-F&O walk.

## recauto-2 (low): recovery's member-failure receipt is the one audit-journal appender that holds no seat; it can collide with the autopilot's once-per-tick record, and either side loses

Pass-1's autopilot report listed this as sound: "Every in-process path that appends spot or F&O records is seat-gated, and the autopilot holds all seats across its append, so I found no concurrent appender to race it." That is false for recovery. recovery-4 and server1-1 named recovery's seated `recovery_spot` receipt and multi-feed presses. They did not name this unseated appender, or the autopilot tick as its counterpart.

**Where**
- crates/api/src/recovery.rs:947-962 (`execute`, the `Err` arm of `assess`). No seat is held and it runs under `site.run` only:
  ```rust
  site.journal()
      .append(&crate::audit::Record::member_failure(...))
      .map_err(failure)?;
  ```
- crates/api/src/autopilot.rs:3482 (`tick`), under `AllSeats` but not under `site.run`: `let journal_error = site.journal().append(&record).err();`
- crates/api/src/audit.rs:1190-1205: `Flock::try_lock(OpenOptions::new()...open(&self.path)?, ...)`. Each call opens a new file description, so two in-process appends conflict, and the second gets `WouldBlock` and is refused.

**Why it is wrong.** As recauto-1 shows, a recovery routinely runs alongside a tick that started before the recovery claimed. If one of the recovery's windows fails `assess` (a mapping refusal, an off-minute or unordered source stamp, a source over the read bound, ...) while the autopilot is appending its tick record, one of the two `try_lock`s fails:
- If recovery loses, the `?` aborts `execute`. The whole plan is sealed `Blocked` (no boot resume), the run ends "Recovery BLOCKED: … cannot take the journal append lock", and the window's `Blocked` item is never written to the plan journal, because `journal.append(item)` at recovery.rs:965 is skipped.
- If the autopilot loses, the tick's single `/audit` record for a month whose bars already landed is refused (`journal_error`). That is the receipt `/audit`'s own footnote says must exist.

**Repro.**
1. The autopilot is mid-tick on month M. A recovery is started; its first window W has an `assess` error, e.g. its source file holds an off-minute stamp.
2. Autopilot thread: `broker_run` returns. tick → `site.journal().append` takes the flock and is inside `sync_all`.
3. Recovery thread, at the same moment: `execute` → `assess(W)` is `Err` → `site.journal().append(member_failure)` → `try_lock` gets `WouldBlock` → `Err` → the run is BLOCKED.

The window is one write plus fsync per tick, so this is rare, but each hit is a deterministic loss.

**Minimal fix.** The same fix as recovery-4: serialise in-process appends with a process-wide `Mutex` inside `audit::Journal::appended` (or take the flock blocking), keeping `try_lock` as the cross-process guard only. Independently, `execute` should record the Blocked window in its plan journal before attempting the audit append, and should not let a refused audit receipt abort the plan.

## Pass-1 verification

| Pass-1 ID | Verdict | Grounds at 331b05c |
|---|---|---|
| autopilot-1 | CONFIRMED | fly reads `is_paused()` at autopilot.rs:2643. `broker_run` captures `epoch` only at server.rs:7819, after the census, survey, `spot_targets` and `ladder_refusal`. `pause()` bumps the epoch (autopilot.rs:2033-2035) and nothing in between re-reads `paused`, so a pause in that gap is absorbed into the captured epoch. All loads and the bump are `Relaxed` (2034, 2081, 2088). |
| autopilot-2 | CONFIRMED | The breaker sets `out.stopped` (server.rs:7932-7938). `outcome_of` maps it to `stopped: run.stopped.is_some() && run.credential_stop.is_none()` (autopilot.rs:3545). `observe` returns `Next::Retry` at 1353 before the attempt or backoff arms run. `settle` returns 0 for Retry. |
| autopilot-3 | CONFIRMED | The clock-wait loop publishes `Phase::Halted` with `feeds` empty (autopilot.rs:2565-2572). `admit_resume` refuses that exact shape with the "it has returned" text (3680-3692) while the task is still alive in `sleep`. |
| autopilot-4 | CONFIRMED | server.rs:18555 calls `flying.abort()`. The tick's only journal append, autopilot.rs:3482, runs after `broker_run(...).await` returns, so an abort at any inner await drops it. |
| recovery-1 | CONFIRMED, and widened by recauto-1 | recovery.rs:1550-1552 reserves before `recovery_spot` takes the seat (server.rs:8022-8025). Its trigger list missed the dominant case: the autopilot never claims `site.run`, so any tick already in progress refuses recovery's seat. |
| recovery-2 | CONFIRMED | `Journal::open` uses `create(true)` (recovery_journal.rs:287-310) and creates active.bin before `active.append(pointer)`. `active_history` refuses an empty file (recovery.rs:266-268). Every entry point goes through it. |
| recovery-3 | CONFIRMED | `attempts.latest` is a `HashMap` (recovery_journal.rs:268). `reconcile_pending` iterates `.values()` (recovery.rs:1046-1056), not `attempts.order`. |
| recovery-4 | CONFIRMED, and widened by recauto-2 | audit.rs:1190-1205 is a non-blocking `Flock::try_lock` on a freshly opened description. recauto-2 adds an unseated recovery appender, and the autopilot tick as its counterpart. |
| recovery-5 (= runs-3) | CONFIRMED | `claim` holds `held` (the `site.run` guard) across `clear_stop` → `persist` (recovery.rs:560-576). `pull_run_stop` holds it across `recovery_control::stop` → `persist` (server.rs:10984-10995). `persist` does `Journal::open`, `append` and two directory `sync_all` calls (recovery_control.rs:148-168). The autopilot's `round` takes the same std mutex at autopilot.rs:3162, so its worker blocks too. |
| recovery-6 | CONFIRMED | recovery.rs:821-827: `journal.append(control).map_err(failure)?; answer`. A poisoned plan journal (recovery_journal.rs:366-388) replaces `answer`'s root cause. |

## Checked and found sound (not findings)
- **Lock order across the two subsystems.** The autopilot takes `site.run` alone, for a read (autopilot.rs:3162), and never while holding `recovery_active` or the status mutex. Recovery always takes `run` then `recovery_active`. `autopilot.publish` (status mutex) is never taken while `run` is held: `update` closures touch only `Progress`. No cycle.
- **Boot after a crash mid-recovery.** server.rs:18536-18540 spawns `fly` and then calls `recovery::resume` synchronously. `resume` claims `site.run` before it returns, and `fly` cannot reach `round` before `grace` (`GRACE_SECS = 20`, autopilot.rs:86, asserted ≥5). So the resumed recovery always holds the slot first, and the autopilot's first round stands off. recauto-1 does not apply at boot. A boot-resumed run whose `seeded` fails leaves control `InFlight` (drive's worker returns before the terminal append), so the next boot retries, as designed.
- **Autopilot Pause/Stop against recovery's `broker_run`.** `recovery_spot` calls the shared `broker_run`, which reads the autopilot epoch. But a recovery request names one member (`checked` requires an explicit `FNO_INDEX` member; `canonical` writes one), so the loop has one target. The only `stopped(epoch)` check that can fire comes right after the capture, with no await between. A pause cannot reasonably stop a recovery request, so no recovery budget is lost to an autopilot pause. The vendor-down breaker needs 3 consecutive instruments and is unreachable with one target.
- **Recovery STOP against the autopilot.** `pull_run_stop` sets `Progress::stopping` only and does not bump the autopilot epoch or pause flag, so a recovery stop never stops or pauses the backfill. After recovery ends, `drive` calls `idle` before the `finished` update. The autopilot's `pressing` read therefore stays true until the slot really is free, and it then resumes on its own (the next `round` re-reads `site.run`).
- **Shared store writes.** Recovery's vendor writes go through `broker_run` under the Zerodha seat. The autopilot's go through `broker_run` under all seats. The seat mask is `AcqRel` `fetch_or` / `compare_exchange` with release on drop, so the two never write bars at the same time. Recovery's own journals (`audit/recovery-v1/*.bin`) are never touched by the autopilot. The autopilot's write probe (`store_writable`) uses its own per-vendor dot-file.
- **Recovery journal flocks.** `drive` holds exclusive flocks on the plan and on attempts.bin for the whole run. Concurrent openers (a second start's `preflight_submission`, `prepare_successor`, the page's `snapshot`) refuse loudly with `WouldBlock`; they never interleave. The 503 for a concurrent start is a less specific message than the 409 that `claim` would give, but it is not a correctness issue.
- **Ctrl-C with a recovery running.** Only `flying` is aborted. The recovery `drive` task is dropped when the runtime shuts down. Sync journal appends complete within a poll. An InFlight reservation dropped after bars land is the "interrupted after storage but before their receipt" case that `reconcile_pending` reassesses to `Unverified` on the next boot, as designed. The lost `/audit` receipt is the known autopilot-4 / hunt-api-1 class.

# conc-pass6: in-process state of the api server and long-running commands, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero):** 4 new findings (0 high, 0 medium, 4 low): conc6-1 to conc6-4. No path lets two pull runs, two sweeps or two recoveries run at once, and no slot leaks for good (every slot has a Drop guard or a terminal write). The new defects are all **reporting or control-coupling** defects: the page or a control answer says something false about a live machine. All 9 known findings re-checked on this theme are still NOT FIXED (table at the end). Another pass is needed.

Method: I read the source only, in the read-only checkout /home/claude/wt/zero3 at 1f4de71. Files: `api/src/autopilot.rs`, `pullrun.rs`, `recovery.rs`, `recovery_control.rs`, `sweeprun.rs`, `booleanlaunch*.rs`, `mastersrun.rs`, `livejson.rs`, `detail.rs`, `main.rs`, and in `server.rs` `pull_run*`, `broker_run`, `recovery_spot`, `Site::reparse`, `serve_limited`, the serve arm and `end_runtime`. I also read `cli/src/live.rs` `CensusCache`. No cargo was run, because no finding is high. Before filing, I checked every finding against concurrency.md (all the autopilot, runs, recovery, recauto, lifecycle, server, log, resources, locks and expr ids, plus its "checked and sound" notes), conc-pass4, conc-pass5 and crash-edge passes 5-7.

---

## State-transition tables

### A. Autopilot (`Control.paused`, `Control.epoch`, `Status.phase`, the `fly` task)

Phases: Starting, Running, Backoff ("waiting"), Paused, Halted, Idle ("complete"), plus the hidden fact "the task has returned".

| from | event | to | verdict |
|---|---|---|---|
| (boot) | `Control::serving()` without `BRUTEX_AUTOPILOT=run` | paused flag set, epoch 1 | clean (safe default) |
| any | `fly` pre-loop exit (no live broker, no day rung, clock allowance spent) | Halted, no feeds, task returned | clean |
| (boot) | clock not ready | Halted, no feeds, task **alive** (clock wait) | autopilot-3 / CE-46, NOT FIXED |
| Halted (clock wait) | Stop | stays Halted (`stop` leaves Halted alone, CE-24) | CE-46, NOT FIXED: a later Resume is refused as "returned" |
| Starting (grace) | Stop | Paused; `grace` returns at its next second | clean |
| loop top, not paused | Stop lands before `broker_run` captures the epoch | the whole month is fetched anyway | autopilot-1, NOT FIXED (server.rs:7917) |
| Running (tick) | Stop | Paused; `broker_run` breaks at the next instrument | clean |
| Running | `settle` returns `Next::Halt` for one feed | Halted (phase and detail only), then a 60 s `nap` | **conc6-1**: `status.feeds` is not refreshed |
| Halted (`settle`) | Stop within the nap | `nap` returns, then `dwell_paused` publishes Paused and overwrites the halt reason; feeds stay stale while paused | **conc6-1** |
| Paused / Halted (`settle`) | Resume | `admit_resume` reads stale feeds, gets Clear, and `start` publishes Running "resumed" | **conc6-1** |
| any | autopilot Stop while a hand `/pull/spot` or a `/pull/run` press leg is walking | that hand walk is cut short as "stopped by the operator"; the press goes on with its next leg | **conc6-2** |
| Paused | `round` stands off because `site.run` is held | Paused, "a hand-made pull is running" | **conc6-4** when the holder is a recovery |
| Idle / Backoff | Resume (not paused) | `start` publishes Running; `state()` maps a Running phase with no in-flight cell to "waiting" | clean (cosmetic) |
| any | `admit_resume` then `start` race against a terminal `publish(Halted)` | for a few microseconds `start` can paint Running over a terminal halt | not filed: the window is a few instructions wide, and the next round republishes |
| Running | shutdown (Ctrl-C) | `flying.abort()` at the next await, mid-tick | autopilot-4 / lifecycle-1, NOT FIXED |

### B. Pull run slot `site.run` (shared by the `/pull/run` press and recovery)

States: None · running (`started && finished.is_none()`) · running+stopping · finished.

| from | event | to | verdict |
|---|---|---|---|
| None / finished | POST `/pull/run` | running (claimed under the lock, then spawned) | clean: double start returns 409 |
| running | POST `/pull/run` or recovery `claim` | refused (409) | clean |
| running | POST `/pull/run/stop` | stopping; recovery STOP is persisted **while `site.run` is held** | runs-3 / recovery-5, NOT FIXED (server.rs:11133-11145) |
| running | `conduct` ends normally | finished(summary), then `Finisher` drops | runs-1, NOT FIXED: `Finisher` (pullrun.rs:695-708) still tests only `finished.is_none()`, and the ticker's in-poll `rows_now` can land after `abort()` (:1046-1053, :1105) |
| running | chain task dies | `note_dead_chain` names it on that feed | clean (release builds abort on panic anyway) |
| running | runtime shutdown | the task is dropped and `Finisher` writes "ended abnormally"; nothing durable | lifecycle-1, NOT FIXED |
| running (recovery) | `drive` ends | `idle` then `update(finished)` | clean (the order keeps the autopilot standing off until the slot is really free) |

### C. Recovery (`recovery_active`, durable STOP `Queued`/`Blocked`, plan control `InFlight`/`Verified`/`Blocked`)

| from | event | to | verdict |
|---|---|---|---|
| idle | `start` (explicit) | preflight (blocking), `claim` (clears STOP **under `site.run`**), `activate`, spawn `activate_durable` | recovery-5 class, NOT FIXED (recovery.rs:561-575) |
| seeding | Stop | stopping and Blocked persisted; `activate_durable` sees `stopping`, idles and finishes BLOCKED | clean |
| seeding | seed I/O fails | a Blocked STOP is persisted, idle, finished BLOCKED; boot never resumes it | clean (by design: only an explicit start clears it) |
| driving | client disconnects | the work is detached and continues | clean |
| driving | Ctrl-C | dropped; the InFlight reservation is reassessed at the next boot | known (concurrency.md "sound" note, lifecycle-1) |
| boot | `resume` | resumes only when InFlight and not STOP; claims the slot before `fly` leaves its 20 s grace | clean |
| two starts | concurrent | the second `claim` returns 409, or a journal flock refuses with 503 | clean (the message is less specific, noted before) |

### D. Sweep / descent / command slot `site.sweep` (with `ADMISSION`, execution lease, `TaskFinisher`, operation audit)

| from | event | to | verdict |
|---|---|---|---|
| free | POST run/descend/command | `admit` under `ADMISSION`: busy check, `prepare` (lease, audit `begin`, marker), install in_flight | clean for exclusion. runs-4 (permits parked on the std `ADMISSION` mutex) NOT FIXED (sweeprun.rs:1916-1939) |
| in_flight | second POST | 409 Busy | clean |
| in_flight | engine finishes | audit `finish`, slot installed, guard disarmed, lease dropped | clean. In the instants between the slot install and the lease drop, a new press is refused Busy by the lease (transient, not filed) |
| in_flight | panic or shutdown before or while queued | `TaskFinisher::drop` writes ABNORMAL_END and a Cancelled/Failed audit | clean |
| in_flight | shutdown | `cli::cancel::request()`, wait for `SHUTDOWN_GRACE`, then abandon | known (hunt-api-2, D-1582) |
| Boolean / index-stop in_flight | per-batch status callback | updates the slot only when `attempt` matches and the run is in_flight | clean |
| restart | GET `/backtest/run.json?attempt=` | persisted invocation, `in_flight:false` | clean |

### E. Masters refresh and universe reparse

| from | event | to | verdict |
|---|---|---|---|
| idle | POST `/masters/refresh` | detached task waits on `REFRESH` (FIFO), then fetch, land, `reparse` | the P3-01-04 fix is correct for one press |
| refreshing | page aborts at 90 s and the operator presses again (or several tabs press) | each press queues another **full** credentialed refresh, with no limit | **conc6-3** |
| any | `reparse` | `reload_lock`, then the read guard is scoped, then a write swap | clean. No `universe()` guard is held across `.await` (it would make the future `!Send`) |

### F. Seats (`Control.seats`)

| event | verdict |
|---|---|
| per-feed `fetch_or`; autopilot `compare_exchange(0, ALL)`; release on Drop | clean. A hand pull and the autopilot never write one feed at once. recauto-1 (a recovery blocked by a mid-tick autopilot) is known |

### G. Live progress `/live.json` (`CENSUS` LazyLock Mutex)

| event | verdict |
|---|---|
| cli renames a live file in place | a new inode means a new `LiveStamp`, so it misses and re-reads; a file that changes mid-read is refused for one coherent snapshot. Clean |
| two concurrent polls | the second waits on the std mutex inside a `detail::run` permit (expr-3 class, not refiled) |

### H. Shutdown

| signal | verdict |
|---|---|
| SIGINT | graceful HTTP drain (blocked by an in-flight hand pull, server1-2), `flying.abort()`, wait for engine tasks only | server1-2, autopilot-4, lifecycle-1: NOT FIXED |
| SIGTERM / SIGHUP | no handler: default disposition, no drain, no exit event | lifecycle-2, NOT FIXED (main.rs passes `tokio::signal::ctrl_c()` only) |

---

## New findings

### conc6-1 (low): a feed halt published by `settle` leaves `status.feeds` stale. A Stop then erases the halt from the page for as long as the autopilot stays paused, and Resume is admitted as "started" against a terminal feed

- **Where:** `crates/api/src/autopilot.rs:3425-3431` (`settle`, the `Next::Halt` arm), `:2752-2765` (`dwell_paused`), `:3757-3810` (`admit_resume`), `:4057-4075` (`start`).
- **Code:**
  ```rust
  Next::Halt { reason } => {
      site.autopilot.publish(move |status| {
          status.phase = Phase::Halted;
          status.detail = reason;
          status.due_unix = 0;
      });
      IDLE_POLL_SECS
  }
  ```
  `admit_resume` decides from `status.feeds[..].halted` only. `status.feeds` is written only by `round`, before the tick (`:3348-3363`) or in the nothing-chosen arm. `dwell_paused` overwrites `phase` and `detail` and leaves `feeds` alone.
- **Why it is wrong:** `observe` sets `state.halted` on fly's local `FeedState` (credential, configuration or store halt, `:1369/1382/1413`). The published report for that feed is still the pre-tick one, with `halted: ""`, until the next `round`. The next round is 60 s away (`nap(IDLE_POLL_SECS)`), and it never comes while the autopilot is paused. So:
  1. Stop pressed within that minute: `nap` returns at once, the loop takes `dwell_paused`, and the page shows Paused, "paused by the operator", with **no feed marked halted**. The credential halt's reason is gone from `/autopilot.json` for the whole pause.
  2. Resume, paused or within the minute: `admit_resume` sees no halted feed and returns `Clear`. `start` publishes Running with "resumed. The next unit is whatever the store is missing", and the route answers 200 `accepted:true`. If this was the last live feed, every feed is in fact terminal and `RESUME_CANNOT_CLEAR` says this exact answer is a lie (the 409 "Every feed is terminal" arm is what should have fired). The next round restores "every feed is halted".
  This is the defect `admit_resume` exists to prevent ("writing 'resumed' over it is the defect this whole admission check exists to remove", `:3891-3893`), reached through stale input rather than a missing check.
- **Repro (not run):** run with one drivable feed whose token is dead. The tick halts it with `Halt::Credential` and the page shows "halted" plus the reason. Within 60 s POST `/autopilot/control action=stop`, then GET `/autopilot.json`: state "paused", every `feeds[].halted` is "", and the reason appears nowhere. POST `action=resume`: you get 200 with `accepted:true` and "started", where 409 was expected.
- **Minimal fix:** in `settle`'s `Halt` arm (and in `probe_store_halts`' revive path), also publish the halted feed's report: write `status.feeds[slot].halted = reason.clone()` (`reports` is parallel to `feeds` by index). Or have `admit_resume` read a halt bitmask that `FeedState::halt` and `revive` keep in an atomic on `Control`, so it never reads published prose.

### conc6-2 (low): the autopilot's Stop cancels a hand `/pull/spot` or `/pull/run` press leg in flight, while its answer says "nothing further is asked of any vendor" and the press goes on

- **Where:** `crates/api/src/server.rs:7917` and `:7938-7946` (`broker_run`: `let epoch = site.autopilot.epoch();` ... `if site.autopilot.stopped(epoch) { out.stopped = Some(... CANCELLED ...); break; }`). `broker_run` is the shared walk for the autopilot tick, the hand spot pull (`spot_pull_held` → `broker_answer`), every press leg (`pullrun::request_leg` → `pull_spot`) and `recovery_spot`. Also `autopilot.rs:2071-2080` (`pause` bumps the epoch), `:3950-3965` (`act` Stop text), and `:213-224` / `:2752-2765` (Paused means "will not start new automatic pulls").
- **Why it is wrong:** the epoch is the autopilot's stop generation, but every manual walk captures and obeys it too. Pressing Stop on the autopilot page (or POST `/autopilot/pause`) therefore:
  - truncates an operator's hand pull at its next instrument. Its receipt says FAILED with "stopped by the operator — stopped after k of n" (`record_stop` pushes a failure), and that is journalled at `/audit`, although that pull's operator stopped nothing;
  - truncates the current leg of every feed in a running press. That leg grades `Retry`, the press sleeps `RETRY_WAIT` and asks again, and the next legs capture the new epoch and run normally. So the vendor *is* asked further, while the Stop answer just said "The sweep stops at its next instrument and nothing further is asked of any vendor".
  Either meaning of Stop is defensible. The code implements neither consistently: the autopilot's control half-stops manual work and reports a full stop. (concurrency.md's server1-2 and lifecycle-1 fixes *propose* bumping this epoch at shutdown, which is a different and intended use. The cross-control coupling itself was never filed. Recovery is safe: one target, and the check right after capture cannot fire, as the "sound" note in concurrency.md says.)
- **Repro (not run):** POST `/pull/run` with Groww and Dhan spot legs over a month. While `/pull/run.json` shows `doing` mid-leg, POST `/autopilot/control action=stop`. Both current legs end with `last_error` "...receipt did not read clean...", and `/audit` gains two FAILED spot records saying "stopped by the operator". About 20 s later the press re-asks those legs and continues, so `/pull/run.json` keeps running:true.
- **Minimal fix:** give manual walks their own stop. Pass the stop source into `broker_run` (`enum StopBy { Autopilot(u64), Press, Never }`): the tick passes the epoch, press legs check `site.run.stopping`, and a hand pull passes `Never`. Alternatively, make Stop truly global: also set `site.run.stopping` and say so in the answer.

### conc6-3 (low): `/masters/refresh` queues an unbounded number of full credentialed refreshes. Each abandoned press (the page aborts at 90 s) still runs later, back to back

- **Where:** `crates/api/src/mastersrun.rs:38` (`static REFRESH: tokio::sync::Mutex<()>`), `:511-515` (`refresh` = `detached(refresh_work(site)).await`), `:552` (`let _refresh = REFRESH.lock().await;`), `:589` (`credentialed_leg`). Page: `web/src/routes/mapping/+page.svelte:144-153` (`ask('/masters/refresh', { ms: 90_000 })`, and the button re-arms when the call fails).
- **Why it is wrong:** before D-1974 (P3-01-04), an aborted request dropped its future, including its place in the `REFRESH` queue. Now the work is detached, and the wait for the lock is detached with it. On the sick host D-1974 was written for (ladder longer than 90 s), the page aborts and re-arms Refresh. Every further press, from any tab or from `web/masters.js:78`, adds one more task that waits FIFO and then runs a complete public-and-credentialed refresh, including the Parameter Store read and the Zerodha dump on the shared token. Nothing bounds the queue, and each of those pages has also timed out, so the operator is told nothing landed while N refreshes are still to run. This is an unbounded queue, which this pass's theme (2) asks about.
- **Repro (not run):** make one master host slow (about 120 s ladder), then press Refresh 5 times at 90 s intervals. `api.masters.source` events show 5 full sequential refreshes over about 10 min, every press after the first answered by an aborted page.
- **Minimal fix:** `REFRESH.try_lock_owned()` at the door. On contention, answer 409 `{"refusal":"a refresh is already running; its outcome will appear on /masters/status.json"}`, and move the owned guard into the detached task. That gives at most one in flight and none queued.

### conc6-4 (low): the autopilot reports "a hand-made pull is running" while a recovery holds the run slot, including one the server resumed by itself at boot

- **Where:** `crates/api/src/autopilot.rs:3238-3246` (`let pressing = site.run.lock()...is_some_and(Progress::running); if pressing { return stand_off(site, "a hand-made pull is running"); }`). `recovery.rs:561-576` (`claim` installs `Progress::claimed()` in the same `site.run`) and `:690-726` (boot `resume` claims it with no operator action).
- **Why it is wrong:** `site.run` is shared by the `/pull/run` press and recovery, but the stand-off text names only the first. After a restart with an InFlight recovery plan, the autopilot page says Paused, "a hand-made pull is running, so the backfill is standing off", and the Pull page shows a run the operator never pressed in this process. The operator is sent to look for a press that does not exist. It is display-only and the exclusion itself is correct.
- **Repro (not run):** leave a recovery plan InFlight (kill during `drive`), start `api serve` with `BRUTEX_AUTOPILOT=run`, and read `/autopilot.json` after the 20 s grace: the detail names "a hand-made pull".
- **Minimal fix:** record the slot owner in `Progress` (`owner: Press | Recovery`, set by `pull_run` and `claim`). `round` then passes `"a recovery plan is running"` or `"a hand-made pull is running"` accordingly.

---

## Checked and clean (this pass)

- **No lock held across `.await`:** `with_progress`, `update`, `stopping`, `halted_feeds`, `Control::publish/inspect`, `Cached::with*` and `admit` are all closure- or statement-scoped. A `Site::universe()` read guard held across `.await` would not compile in a spawned handler (`!Send`).
- **Lock order:** run before `recovery_active` (claim, pull_run_stop). `recovery_active` alone (idle, is_stopped, clear_stop outside claim). `ADMISSION` before `sweep`, and never the reverse. No cycle.
- **Double-claim races:** `pull_run` and `claim` check and install under one take. `admit` checks and installs under `ADMISSION`. Seats use `fetch_or` / `compare_exchange(0, ALL)`. No path admits two runs.
- **Slot leaks:** `pullrun::Finisher`, `sweeprun::TaskFinisher` (armed before `spawn_blocking`, so a dropped queued closure still releases the slot), and recovery `drive`'s unconditional `update(finished)` are all present. None leaks for good. runs-1's mis-attribution is the known one.
- **Cancellation:** hand pulls, masters refresh and recovery start/prepare are detached (`detached_pull`, `detached`, `spawn_blocking`). Between `claim` and `tokio::spawn(activate_durable)` in recovery `start` there is no await, so a disconnect cannot leave a claimed slot with no worker.
- **Sweep restart:** a browser attempt from a dead process is served from the persisted invocation as `in_flight:false`, so the page cannot spin forever on it.

## Verification of known findings on this theme (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| autopilot-1 | NOT FIXED | `fly` checks `is_paused` at loop top (autopilot.rs:2703). `broker_run` first captures the epoch at server.rs:7917, after the census, survey and ladder |
| autopilot-3 / CE-46 | NOT FIXED | the clock wait publishes `Phase::Halted` with no feeds (autopilot.rs:2627-2632). `admit_resume` refuses that shape as "returned" (:3762-3776). `stop` leaves Halted alone (:4040-4043) |
| autopilot-4 | NOT FIXED | server.rs:19267 `flying.abort()` mid-tick, and nothing pauses first |
| runs-1 | NOT FIXED | pullrun.rs:695-708 `Finisher` still tests only `finished.is_none()`. The ticker abort is at :1105 |
| runs-3 / recovery-5 | NOT FIXED | server.rs:11133-11145 holds `site.run` across `recovery_control::stop` (persist plus 2 dir fsyncs). recovery.rs:561-575 holds it across `clear_stop` |
| runs-4 | NOT FIXED | sweeprun.rs:1916-1939: `ADMISSION` (std) is held across `prepare` (lease, audit `begin`, log walk) inside `detail::run` |
| server1-2 | NOT FIXED | server.rs:17552-17557: graceful drain only. The shutdown path bumps no epoch, and `detached_pull` is still awaited by its connection |
| lifecycle-1 | NOT FIXED | server.rs:19248-19267: only `flying` is aborted. The press and recovery handles are dropped |
| lifecycle-2 | NOT FIXED | main.rs passes `Box::pin(tokio::signal::ctrl_c())` only, so SIGTERM and SIGHUP are unhandled |

Tally: 9 checked, 0 FIXED, 9 NOT FIXED.

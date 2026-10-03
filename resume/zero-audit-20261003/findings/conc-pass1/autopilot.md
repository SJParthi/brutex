# conc-pass1 / autopilot — 4 findings (2 medium, 2 low)

Verdict: two real defects in how the autopilot's scheduler state reacts to concurrent control and to the run's own stop, plus two low state/restart defects. Audit only, from source at 331b05c. No cargo was run.

Slice: `crates/api/src/autopilot.rs` (fly / round / tick / settle / observe / Control / admit_resume / act), and what it calls to persist or decide: `server::broker_run`, `audit::Journal::append`, `store_writable`, the serve-lock spawn and abort in `server.rs`.

| ID | sev | where | one line |
|---|---|---|---|
| autopilot-1 | medium | autopilot.rs:2643 → server.rs:7819 | A Pause that lands between the loop's `is_paused()` check and `broker_run`'s epoch capture is ignored, and a whole month (~765 instruments) is fetched anyway. |
| autopilot-2 | medium | autopilot.rs:3545, 1353; server.rs:7932 | The vendor-down breaker's stop is read as "the operator stopped it": `Next::Retry` with zero wait and no attempt counted. The backoff and the 9-attempt bound are bypassed against a down vendor, with no limit. |
| autopilot-3 | low | autopilot.rs:2549-2573, 3681 | During the pre-loop clock wait (up to 20 min) the task is alive, but `admit_resume` refuses Resume with 409 and says the task "has returned". |
| autopilot-4 | low | server.rs:18555; autopilot.rs:3443/3482 | `flying.abort()` on Ctrl-C drops a tick after bars landed and before its journal record. The month's bars are on disk with no `/audit` row. |

---

## autopilot-1 (medium): a pause in the round's pre-fetch window does not stop the fetch

**Code.**
- fly, autopilot.rs:2642-2646:
  ```rust
  loop {
      if site.autopilot.is_paused() {
          dwell_paused(&site.autopilot).await;
          continue;
      }
  ```
- `round` then takes the seats and runs `census::read_all` (autopilot.rs:3180), `probe_store_halts` and `survey`. `tick` runs a second `census::read_all` (autopilot.rs:3443) and calls `broker_run`. Inside `broker_run`, after `spot_targets`, the sort, `ladder_refusal` and `note_run_started`, server.rs:7819 runs:
  ```rust
  let epoch = site.autopilot.epoch();
  ```
- The only stop check in the instrument loop, server.rs:7840:
  ```rust
  if site.autopilot.stopped(epoch) {
  ```
- `Control::pause`, autopilot.rs:2033-2035, sets `paused = true` and bumps `epoch`. Nothing between autopilot.rs:2643 and server.rs:7819 reads `is_paused()` again. Grep confirms: the only non-test `is_paused` reads in autopilot.rs are 2643, 2711 (grace) and 2734 (nap). `broker_run` never reads it.

**Why it is wrong.** The epoch design is correct for a pause that arrives after the capture. A pause that arrives before the capture has already bumped the epoch, so the run captures the new value and `stopped(epoch)` is false for the whole run. The window covers two full `census::read_all` passes. Each re-reads and CRC-checks every vendor manifest (o1surface2-2 measures these at up to 268 MB each, read inline on a Tokio worker). It also covers a store probe, the survey and the target build. That is plausibly hundreds of ms to seconds on every tick. The page says Pause "bites within a second", and `stop()` publishes "nothing further is asked of any vendor".

**Repro (two tasks).**
1. Autopilot task: fly at :2643 reads `paused == false`, enters `round`, and starts `census::read_all` at :3180.
2. HTTP task: `POST /autopilot/control action=stop` → `act` → `stop` → `pause()`. Now `paused = true` and `epoch` goes from N to N+1. The page shows "pause requested".
3. Autopilot task: finishes the census, survey and `tick`, enters `broker_run`, and captures `epoch = N+1` at server.rs:7819.
4. Every instrument check at :7840 compares N+1 with N+1, so it never stops. The full month for all ~765 tracked instruments is fetched against the vendor quota (5 to 37 minutes per the comment at :7836) and written to the append-only store. The loop only pauses after `round` returns.

**Minimal fix.** Capture the stop generation before the pause check and hand it down. For example, `let at = site.autopilot.epoch(); if site.autopilot.is_paused() {…}`, then pass `at` through `round → tick → broker_run` (a new parameter; hand callers pass `site.autopilot.epoch()` as today). Make the bump `epoch.fetch_add(1, Release)` and the capture `epoch.load(Acquire)` (or SeqCst both) so that "saw the old epoch" implies the later `stopped(at)` sees the bump. A cheaper alternative: re-check `site.autopilot.is_paused()` immediately after the capture at server.rs:7819, only for autopilot callers, and return `stopped` if it is set.

---

## autopilot-2 (medium): a vendor-down breaker stop bypasses the backoff and the attempt bound

**Code.**
- server.rs:7932-7938, inside `broker_run`:
  ```rust
  if breaker_trips(vendor_down_streak) {
      out.stopped = Some(vendor_down_sentence(...));
      break;
  }
  ```
- autopilot.rs:3545, `outcome_of`:
  ```rust
  stopped: run.stopped.is_some() && run.credential_stop.is_none(),
  ```
- autopilot.rs:1353-1355, `FeedState::observe`:
  ```rust
  if out.stopped {
      return Next::Retry;
  }
  ```
- `settle` maps `Next::Retry` to `0` (autopilot.rs:3326), so `round` returns 0 and `fly` skips `nap`.

**Why it is wrong.** `TickOutcome::stopped` is documented as "Set when the operator stopped the sweep part-way. Not a failure." The test `being_stopped_by_the_operator_costs_no_attempt` pins exactly that meaning. However, `BrokerRun::stopped` has two producers: the operator's epoch at server.rs:7840 and the vendor-down breaker at server.rs:7932. The breaker means "the vendor's side answered 5xx on 3 consecutive instruments". That is a transport failure, and the backoff (30 s → 900 s) plus `MAX_MONTH_ATTEMPTS` → `Stall` → `STALL_RETRIES` exist for it ("at most 9 attempts per stalled month per process", autopilot.rs:1431). Because the breaker stop is folded into `stopped`, observe returns Retry before the reason or attempt arms run. No attempt is counted, no backoff is armed, the month never stalls, and nothing bounds the retries.

**Repro.** The vendor returns 5xx for every request for an hour. The day rung of month M is chosen; the minute rung for M is already complete, or later on the ladder.
1. Tick: instruments 1 to 3 each pay the per-instrument 5xx ladder, the breaker trips and `out.stopped = Some(..)`. The journal gets a record, `outcome.stopped = true`, observe returns `Retry`, and `round` returns 0.
2. `fly` does not nap. The next iteration takes the minute rung. Either `survey` finds nothing and returns `IDLE_POLL_SECS`, or the minute run hits the same breaker and also returns 0.
3. The day rung for M is picked again on the next pass. Repeat for as long as the vendor is down. Each pass costs ≥3 instruments of vendor requests, plus their in-pull retries, and one 256-byte journal record. `attempts` stays at 0, so the 9-attempt bound and the 6 h reconsider spacing never apply. Pass spacing is at most ~60 s (the idle poll), not the 30-900 s backoff. The breaker's own sentence says "Press Pull again when it is back". The autopilot instead presses again at once.

**Minimal fix.** Tell the two stops apart at the source. Add `BrokerRun::cancelled: bool`, set only in the `stopped(epoch)` arm (server.rs:7840), and use `stopped: run.cancelled && run.credential_stop.is_none()` in `outcome_of`. The breaker's stop then reaches observe with `reason = Some(..)`, is classified `Transport`, and gets the bounded backoff and stall like any other vendor failure. Add a test that drives `outcome_of` with a breaker-stopped `BrokerRun` and asserts `Next::Wait`.

---

## autopilot-3 (low): Resume is refused as "the task has returned" while the task is alive in the clock wait

**Code.**
- fly, autopilot.rs:2547-2573. While `yesterday_ist` fails, the task loops up to `CLOCK_WAITS` (20) times, 60 s apart. Each time it publishes:
  ```rust
  status.phase = Phase::Halted;
  status.detail = saying;
  ```
  `status.feeds` is still empty, because the feeds are published only by the first `round`.
- `admit_resume`, autopilot.rs:3680-3690:
  ```rust
  if status.feeds.is_empty() {
      if status.phase == Phase::Halted {
          return Admission::Refused { why: format!("{RESUME_CANNOT_CLEAR} No feed has reported at all and the autopilot is halted, which is the state it takes when the backfill task stopped before its first round — it has returned, so nothing is left to read the flag ...") };
  ```

**Why it is wrong.** The decision reads published state that one live phase (the clock wait) shares with the three terminal pre-loop exits. The doc on `admit_resume` lists the three `return` sites but not this loop, which was added later ("This used to `return`…"). In the default boot (paused, `Control::serving`), an operator who presses Resume during the up to 20-minute wait gets a 409. The 409 says the task has returned and only a restart helps. That is false. If the clock heals, the task proceeds to `grace`, then `dwell_paused` publishes `Paused`, and the same Resume is then admitted. The operator may restart a healthy process on the strength of the false refusal.

**Repro.** Boot with the system clock not yet usable (`ingest::ist_day` errs). Within the first 20 minutes, `POST /autopilot/control action=resume` returns 409 with the "it has returned" text. Fix the clock. About 60 s later the task leaves the wait loop and a second resume returns 200.

**Minimal fix.** Do not reuse `Phase::Halted` for the waiting loop. Use `Phase::Starting` (or a distinct waiting phase) with `due_unix` set to the next retry, or add a `terminal: bool` on `Status` that only the three `return` paths set. `admit_resume` then refuses only on `terminal`.

---

## autopilot-4 (low; class KNOWN as hunt-api-1, new trigger): Ctrl-C mid-tick leaves stored bars with no audit record

**Code.**
- server.rs:18555 (serve arm, after `serve(...)` returns on Ctrl-C):
  ```rust
  flying.abort();
  ```
- autopilot.rs:3443: `crate::server::broker_run(&asked, site, ...).await;`. `broker_run` lands each instrument via `land_broker_member(...).await` and then awaits the next `broker_window(...).await` (server.rs:7861, 7920-7921).
- autopilot.rs:3482: `let journal_error = site.journal().append(&record).err();` runs only after `broker_run` returns.

**Why it is wrong.** `abort()` cancels the task at its next await. After at least one instrument has landed, the next await is the following instrument's `broker_window`. The future is dropped there, so the tick's single journal record is never written. `note_run_finished` and the `status.now = None` clear are skipped too, but the process is exiting, so those do not matter. The bars are durable (the comment at server.rs:18550 is right about the bar file). `/audit`, whose own footnote says a run missing from it "was never written down, and that is a defect in the journal", shows no run for them. hunt-api-1 reported the same lost receipt for hand pulls dropped by a client disconnect. This is the autopilot's own path to it, and it happens on every Ctrl-C during a tick.

**Repro.** `BRUTEX_AUTOPILOT=run api serve`. Wait until `/autopilot.json` shows `phase: running` with `now.index >= 2`, then press Ctrl-C. The store's manifest for that month has gained rows, and `audit/` has no `autopilot <feed> <month>` record for the tick.

**Minimal fix.** Do not abort a running tick. Have the shutdown path `pause()` the control, which bumps the epoch so `broker_run` breaks at its next instrument and the tick journals its partial run. Then `await` the join handle with a bound (for example one instrument's worth, or `tokio::time::timeout`), and `abort()` only if that expires.

---

## Checked and not reported

- **Seat bitmask** (`take_seat` `fetch_or` AcqRel, `take_every_seat` CAS 0→ALL, `AllSeats::drop` `store(0)`). While ALL is held no `Seat` can exist, so `store(0)` cannot wipe a live seat. Sound.
- **`site.run` read then `take_every_seat` (TOCTOU with `/pull/run`).** A press that claims `site.run` in the gap loses its first legs with 409 for at most one tick. `conduct` retries every 20 s for up to 400 passes. The comments already acknowledge this and it is bounded. Not a defect.
- **Lock order.** `site.run` is taken and released before the seats. `status` is never held across I/O or another lock (`publish` closures are pure). Poisoned `publish` was already listed in errpaths line 58.
- **Journal append from the tick.** `Flock::try_lock` refuses rather than waits. Every in-process path that appends spot or F&O records is seat-gated, and the autopilot holds all seats across its append, so I found no concurrent appender to race it. A refusal is published as `journal_error`, not swallowed.
- **Write probe** (`store_writable`). One per-vendor fixed path, used only by the single autopilot task. A crash between create and remove leaves a 19-byte dot-file that the next `File::create` truncates. It refuses a missing root rather than recreating it.
- **Restart.** `FeedState` (frontier, attempts, stalls, halts) is process-local by design and re-derived from the store census. The stated bounds are "per process", so a restart resetting attempts and halts is documented behaviour. Only one `fly` is spawned per process (server.rs:18536), and `serve.lock` stops two servers on one store root. The ghost-after-Ctrl-C case is hunt-api-2.
- **`SeriesCache`.** It reads `generation` and the universe in two separate snapshots. A reparse between them pairs a newer list with an older generation, which only causes one extra rebuild, never a stale list.
- **`broker_run` target order.** `sort_unstable` on the full `InstrumentKey` `Ord` is deterministic. HashMap order does not leak.

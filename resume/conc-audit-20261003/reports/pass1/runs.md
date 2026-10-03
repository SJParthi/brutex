# conc-pass1 / runs — launch, cancel and finish lifecycle (sweeprun, pullrun, mastersrun, indexstoplaunch)

Verdict: 4 findings at 331b05c (0 high, 1 medium, 3 low). The browser engine slot (ADMISSION, then the in-flight check, then the lease, then TaskFinisher) is sound against two launches at once. The pull-run slot is not: its abnormal-end guard and its ticker write into whatever run occupies `site.run`, not into their own run.

Scope read in full: `crates/api/src/sweeprun.rs` (admission, TaskFinisher, run/descend/command, run_json, external observation), `pullrun.rs` (conductor, chains, Finisher, ticker), `mastersrun.rs` (refresh, reload), `indexstoplaunch.rs`. Also read where these call out: `server.rs` pull_run/pull_run_stop, `recovery_control.rs`, `cli/src/execution_lease.rs`, `cli::run_durable`, `detail.rs`. Skipped as already reported: hunt-conc-6 (knobs poison) and hunt-conc-8 (SharedBy), and the "no std guard across .await" note in errpaths.md.

---

## runs-1 — medium — pull-run Finisher and ticker write into the NEXT run's progress; a live run can be marked finished and a second concurrent pull admitted

**Where**: `crates/api/src/pullrun.rs:929` (`let _finisher = Finisher {...}`), `:590-605` (`impl Drop for Finisher`), `:985-992` (ticker), `:1005-1013` (abort, then the final write). `server.rs:10920-10933` is the claim.

```rust
// conduct_with
let _finisher = Finisher { site: Loaded::clone(&site) };   // declared first => dropped LAST
...
ticker.abort();
let current_rows = rows_now(&site);
with_progress(&site, |progress| {
    progress.rows_now = current_rows;
    progress.finished = Some(run_summary(...));            // slot now reads running()==false
});
}   // locals drop: checkpoints, outcomes, ticker handle, groups (every Leg's Strings), THEN _finisher

impl Drop for Finisher {
    fn drop(&mut self) {
        with_progress(&self.site, |progress| {
            if progress.finished.is_none() {                // no check that this is OUR run
                progress.finished = Some("The run ended without recording a summary ...");
```

**Why it is wrong**: `running()` is `started && finished.is_none()`, and `pull_run` (and `recovery::claim`) install a fresh `Progress::claimed()` the moment `running()` is false. The normal path publishes `finished` and only later drops `_finisher`. Between those two points another worker can claim the slot. `Finisher::drop` then checks only `finished.is_none()`, so it sees the new run's `None` and stamps "ended abnormally" on it. The ticker is the same class. `JoinHandle::abort` does not stop a poll that is already running. A ticker that is inside the synchronous `rows_now` (a full census read, `rows_now`'s own doc says O(manifest bytes + E log E)) when `abort()` runs still finishes that poll, and its `with_progress(|p| p.rows_now = seen)` lands after the conductor's final write, possibly on the next run.

**Repro (two runtime workers)**:
1. W1 (conductor A) runs the final `with_progress` (`finished = Some(summary)`) and releases `site.run`. It is now dropping `groups`, which for a large basket is thousands of `Leg`s, each owning several `String`s.
2. W2 handles `POST /pull/run` B, for example from a script that retries on 409 until accepted. B sees `!running()`, installs `Progress::claimed()`, spawns `conduct(B)`, and answers 202.
3. W1 reaches `_finisher` drop. `with_progress` finds B's progress with `finished == None` and writes the "stopped abnormally" sentence. `running()` is now false while B's conductor and chains are live.
4. `POST /pull/run` C is accepted (`server.rs:10926` sees `!running()`). B and C now pull concurrently over one store. That is the interleaving that `pull_run`'s own doc forbids ("Two runs over one store would interleave two vendors' writes into a single month file"). B's chains also `with_progress(|p| p.feeds.get_mut(nth))` into C's freshly rebuilt `feeds` rows, so both runs' counters corrupt each other's status. When B ends, it overwrites C's `finished` with B's summary.

**Fix**: give `pullrun::Progress` a generation (a `u64` from a process counter, set in `claimed()` and returned to the claimer). Pass it to `conduct`, and make `with_progress`, `Finisher::drop` and the ticker write only when `progress.generation == mine`. Alternatively use the `TaskFinisher` pattern: make the final summary write a method on the Finisher (`finish(summary)`) that writes and disarms under one lock take. Then `Drop` is inert on the normal path. The ticker still needs the generation check, or it must be joined (`ticker.abort(); let _ = ticker.await;`) before the final write.

---

## runs-2 — low — a GET of `/backtest/run.json` takes the store's exclusive execution lock, so a concurrent CLI sweep or browser launch is refused as "another sweep owns this store"

**Where**: `crates/api/src/sweeprun.rs:2235` (`observed_status_with_admission`), which calls `crates/cli/src/execution_lease.rs:125-134`:

```rust
pub fn probe(root: &Path) -> Result<(), Refusal> {
    ...
    let held = lock(file, &path)?;          // Flock::try_lock => exclusive, non-blocking
    verify(&held, &path)?;                  // fstat + lstat
    held.release().map_err(unavailable)
}
```

`Lease::acquire` (used by `cli::run_durable`, lib.rs:2101, and by the browser's `claim_execution`, sweeprun.rs:1761) uses the same non-blocking exclusive `try_lock` and maps `WouldBlock` to `Refusal::Busy`.

**Why it is wrong**: a read-only status poll becomes, for the length of an open + flock + two stats + unlock, an owner of the write-admission lock. Every idle poll of `/backtest/run.json` without `?attempt` (no local run in flight) takes this path. A real launch that lands inside that window gets a false negative: no one is sweeping, yet the answer says "another sweep owns this store's execution lease; no new work was queued". For the CLI that is `FAILED` with "required execution admission could not start", which loses an unattended or scripted run. A status read that changes the answer to a write is the fallback-shaped coupling §4 warns about. The refusal is loud, but it is untrue.

**Repro**: process A, the API, serves the backtest page, which polls `/backtest/run.json`. Process B runs `cli range-all ...`. Interleaving: A's `probe` has `flock(LOCK_EX|LOCK_NB)` succeed → B's `Lease::acquire` `flock(LOCK_EX|LOCK_NB)` returns `EWOULDBLOCK` → B prints "refused: required execution admission could not start: another sweep owns this store's execution lease" and exits non-zero → A's `release()`. The same race exists between a second browser tab polling and the first tab's `POST /backtest/run`, which answers 409 `Busy`.

A smaller sibling of the same class is in `TaskFinisher::finish` (sweeprun.rs:1233-1263). The slot is published as `in_flight == false` before `self` is dropped, and dropping `self` is what releases the `lease`. A POST admitted in that gap passes the slot check and then gets `Busy` from `Lease::acquire`. The gap is microseconds, so the probe race above is the reachable one.

**Fix**: do not take the exclusive lock to answer a GET. Report `available: "unknown"`, or read the lock state without owning it (`fcntl(F_OFD_GETLK)` on an OFD lock, or `/proc/locks`). If the probe must stay, make `Lease::acquire` retry `WouldBlock` a bounded few times (for example 3 × 2 ms), because a probe's hold is bounded and a real owner's is not. Separately, in `TaskFinisher::finish`, `drop(self.lease.take())` before writing the slot.

---

## runs-3 — low — `POST /pull/run/stop` holds the `site.run` std mutex across directory creation, a journal append and three fsyncs, on an async worker

**Where**: `crates/api/src/server.rs:10981-11008`:

```rust
let mut held = site.run.lock().unwrap_or_else(PoisonError::into_inner);
let stopping = match held.as_mut() { Some(p) if p.running() => { p.stopping = true; true } _ => false };
if stopping && let Err(why) = crate::recovery_control::stop(&site) {   // `held` still alive
```

`recovery_control::stop` → `persist` (`recovery_control.rs:148-168`) does `create_dir` ×2, `Journal::open` (which syncs its directory), `journal.append`, then `File::open(..).sync_all()` on `audit/` and on the store root.

**Why it is wrong**: `held` is not dropped before `stop`, so the pull slot's std mutex is held for the length of up to three device syncs, on a Tokio worker (the handler is `async` and calls this inline, not through `detail::run`). Everything that touches `site.run` blocks its own worker thread until the syncs return: every recovery and pull chain's `with_progress` and `update`, `stopping()`, `halted_feeds()`, the ticker, and every `/pull/run.json` poll (`server.rs:10962`). On a slow or saturated disk, one stop press freezes a whole worker pool's worth of pull tasks and status polls. The lock order (`run` then `recovery_active`) is deliberate, but the order only has to hold for the in-memory `stopping` flag and the active-id read, not for the fsyncs.

**Repro**: a recovery run is active with 4 runtime workers. The operator presses Stop while the disk is slow (an fsync taking 2 s). W1 holds `site.run` inside `sync_all`. Chains on W2-W4 reach `with_progress` after their next leg and block on the std mutex. All 4 workers are now parked, so no HTTP request is served, `/pull/run.json` included, until the sync returns.

**Fix**: inside the lock, set `stopping` and copy the active recovery id. Drop `held`, then run `persist` (through `detail::run_owed` or `spawn_blocking`). The `run` → `recovery_active` order is kept for the read, and the I/O runs outside both locks.

---

## runs-4 — low — queued browser launches park shared `detail::run` permits on the `ADMISSION` std mutex, so one slow admission returns 429/503 on unrelated routes

**Where**: `crates/api/src/sweeprun.rs:1727` / `1963` / `2983` (`crate::detail::run(move || run_with(...))`) and `:1680` (`ADMISSION.lock()`). `detail.rs:14` sets `MAX_CONCURRENT = 4`, and `Permit::try_take` refuses `Saturated` when the permits are gone.

**Why it is wrong**: each POST takes one of the 4 shared blocking permits before it reaches `admit`, then blocks on `ADMISSION` while another admission does its unbounded I/O: canonicalizations, lease, an external-log walk of up to 8 MiB (`observe_elsewhere`), launch `prepare`, audit `begin` with syncs, and a telemetry marker. The busy check runs only after the mutex is taken, so a POST that will be refused as `Busy` still holds a permit for the full length of the first admission. `detail::run` is the door for most read routes, `/backtest/run.json`'s external-status path among them (sweeprun.rs:2148). That doc comment (sweeprun.rs:1648-1663) cites W1-api6-2 for removing exactly this kind of stall from the slot mutex. The stall has moved to the permit pool.

**Repro**: tab 1 presses Run. Its admission is walking an 8 MiB telemetry tail plus syncs on a slow disk. Tabs 2-4 (or a double-click plus a retry) press Run, Descend or a command. Each takes a permit and parks on `ADMISSION`. The pool is now 4/4. Every `detail::run` route, including the status poll the pages use to learn the first run was accepted, answers `RunError::Saturated` (429/503, "external sweep status is unavailable") until admission 1 finishes.

**Fix**: run the in-flight check before taking a permit (a cheap `site.sweep` read in the async handler; it is advisory, and `admit` rechecks it). Or use `ADMISSION.try_lock()` and answer `Busy` on `WouldBlock` instead of queueing, which matches the documented "refused rather than queued" behavior.

---

## Checked and clean

- **Two browser launches at once.** `admit` serialises in-process. The slot is read and written under ADMISSION, so no install can come between the busy check and the write. Across processes, `Lease::acquire` is held from admission until the `TaskFinisher` drops. No interleaving gives two in-flight slots.
- **TaskFinisher cancel versus finish.** The guard is armed before `spawn_blocking`, so a queued closure dropped at shutdown is covered. `finish` writes and then disarms; `Drop` is inert afterwards. `Drop` uses an `in_flight()`-only filter, which is safe because admission excludes a second in-flight run while the guard is armed. The terminal audit is written before the slot is published, and the knob guard (`Applied`) clears on unwind.
- **indexstoplaunch::conduct callback.** It takes `site.sweep` only inside the observer and checks `attempt` plus `in_flight` plus the exact request. No path holds `site.sweep` while calling into cli, so there is no lock-order cycle with the cli journal. The `finished_micros = Some(started)` placeholder is overwritten by the caller.
- **pullrun checkpoints.** Relaxed `AtomicBool`s carry no other data, and every chain is `.await`ed (joined) before the next pass reads them. A chain panic becomes `note_dead_chain` plus `Retry`.
- **Lock order.** Always `site.run` then `recovery_active`. ADMISSION is never taken while the slot is held.
- **mastersrun.** `REFRESH` (tokio mutex) covers fetch, land and reload FIFO. `land` and `reparse` run synchronously on the async worker; that blocks a worker but there is no correctness race, and `reparse` takes `reload_lock` and then `parsed.write()` with the read guard scoped out first. Masters are landed only through this route, with no CLI writer.

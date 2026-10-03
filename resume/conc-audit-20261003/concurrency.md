# Concurrency and state audit (commit 331b05c, origin/final/all-fixes)

Pass 1: 20 auditors, 63 findings (about 59 unique): 0 high, 24 medium, 39 low. Pass 2: 20 auditors, 51 new findings: 0 high, 16 medium, 35 low. Source reading only, no cargo. Pass 3: 2 agents, 6 new (1 medium, 5 low).

Overlaps: cli1-5 = telemetry-1 (xcut-1 widens it to api vs cli); cli1-3 = runs-2 (execution lease probe); server1-1 widens recovery-4 (audit journal try_lock); runs-3 = recovery-5 (site.run held across fsyncs).

## Pass 1 reports

---

<!-- conc-pass1/engine.md -->
Verdict: the compute path is deterministic across thread counts and holds no shared mutable state. There is 1 finding (0 high, 1 medium, 0 low): a transient OS resource refusal gets frozen into a durable checkpoint.

Slice: crates/engine, runner, indicators, vocab, costs, greeks, core. Commit 331b05c. I read the source only. I did not run cargo.

## engine-1 (medium): a transient `Breach::Workers` / `Breach::Memory` halt is checkpointed as terminal, and every rerun of the same identity replays it

**Where**
- crates/engine/src/lib.rs:2373-2383 (`drain`):
  ```rust
  std::thread::Builder::new()
      .spawn_scoped(scope, move || { ... })
      .map_err(|_| Breach::Workers)?;
  ```
- crates/engine/src/lib.rs:2210-2212 (`cannot_grow` → `Breach::Memory`): `out.try_reserve(by).is_err()`
- crates/engine/src/lib.rs, `continue_walk` (around lines 1660-1675):
  ```rust
  progress.halted = halt;
  sink.report(&current, progress.admitted, progress.pairs);
  sink.checkpoint(&current, &progress)?;
  if halt.is_some() { break; }
  ```
  The checkpoint is taken for every breach kind. It does not distinguish the budget breaches (Candidates, Pairs), which follow from the inputs, from the environmental ones (Workers, Memory).
- crates/engine/src/resume.rs:268-280 (`validate_halt`) accepts `Breach::Workers` and `Breach::Memory`. resume.rs:631-635 decodes tags 3 and 4.
- crates/engine/src/resume.rs:509-548 (`resume_checkpointed`) says *"Extinct and resource-halted checkpoints return their original outcome; a partial frontier never seeds another level."* `continue_walk` is entered with `progress.halted = Some(..)`. Its `while ... && progress.halted.is_none()` loop therefore never runs, so the saved halt is returned as the answer.
- Consumer: crates/cli/src/and_checkpoint.rs:145-215. `Journal::open(root, NAMESPACE, attempt.identity())` is keyed by the run identity, not by the attempt token. `recover()` takes the newest acknowledged boundary, and `ladder.resume_checkpointed(...)` replays it.

**Why it is wrong**

`Breach::Workers` is documented (lib.rs:677-679) as "The operating system refused to create a support-counting worker". That is a property of the moment, not of the inputs. Typical causes are EAGAIN from RLIMIT_NPROC or a cgroup `pids.max` while the api server's and rayon's threads are live, because `drain` spawns `lane_count` new OS threads on every batch. `Breach::Memory` is by its own doc "what the allocator genuinely refuses" at that instant, which includes pressure from another process.

Neither cause is a term of the run identity (CLAUDE.md §3 rule 3). Yet the halted level is published as a sealed, acknowledged boundary under that identity, and resume treats it as terminal. Rerunning the same inputs on the same machine, with the pressure gone, returns the same `WORKERS` / `MEMORY` halt and spawns no thread at all. This breaks "Reruns are safe" (§3 rule 5). A one-off resource failure becomes the permanent recorded outcome for that identity. The only escape is deleting the journal by hand, and nothing tells the operator to do that.

**Repro (exact sequence)**
1. Run `cli sweep-stored` (AND checkpoint path) on instrument X. The column is wide enough that k=3 needs more than one batch (`batch_cap = lanes × BATCH_PER_LANE`).
2. During k=3, a cgroup `pids.max` is reached because the api server is busy. `spawn_scoped` returns EAGAIN. `drain` returns `Err(Breach::Workers)`. `next_level` returns `halt = Some(Halt{k:3, breach: Workers, ..})`.
3. `continue_walk` sets `progress.halted` and calls `sink.checkpoint(..)`. `walk_within`'s callback publishes the k=3 boundary to the journal with `journal.publish`, and it is acknowledged. The run ends halted with WORKERS.
4. The load goes away. The operator reruns the identical command. The identity is the same, so the journal is the same. `recover()` returns the k=3 boundary, and `validate_for` passes because lanes are deliberately not compared. `resume_checkpointed` → `continue_walk`: the loop condition `progress.halted.is_none()` is false, so the walk returns the WORKERS halt again with no spawn attempted. Every later rerun does the same.

**Minimal fix**

Do not make an environmental halt durable. Pick one of these:
- (a) In `continue_walk`, skip `sink.checkpoint` when `halt` is `Some` with `breach ∈ {Memory, Workers}`. The last durable boundary is then the previous complete level, and a rerun resumes from k-1 and retries.
- (b) Have `Checkpoint::validate` / `resume_checkpointed` refuse, or rewind past, a checkpoint whose halt is Memory or Workers. Then `walk_within` retries from the last complete boundary.

Option (a) is the smaller change. Either way, keep the halt loud in the live run's report.

## What I checked and found sound (not findings)
- **engine `drain`** (lib.rs:2361-2400). Scoped threads write disjoint `chunks_mut` slices, and results are committed serially in candidate order only after every spawn succeeded. On a spawn failure the scope joins the threads that did start and nothing is committed; `generated` and `emitted` are rolled back by `batch.len()` (lib.rs:2044-2051). A worker panic propagates through the scope rather than being lost. The ceiling, pair-budget and prune checks run serially before batching, so the lane count cannot move a halt. `Memory`'s `grow_by = batch.len()+1` depends on the lane count, but only at the allocator's own non-deterministic boundary.
- **Lane count in identity and checkpoint.** `support_lanes` is written into the checkpoint header (resume.rs:387) but deliberately not compared on resume (resume.rs:162-186, D-1439), so a resume on a different core count works. It is not a run-identity term.
- **runner rayon sites** (rank.rs:705 `par_chunks` with an indexed collect and a serial `admit`; validate.rs:2661, 4710, 4987; bootstrap_family_pass.rs:396). All gather results by index and fold serially. The rank comparator ends in `total_cmp` and then the mask. This matches the earlier hunt-conc table; nothing has changed at this commit.
- **Shared statics.** The only non-test runtime static is `runner::research_family::membership_snapshot_digest_v1`'s `OnceLock<[u8;32]>` (research_family.rs:205). It is a pure function of const tables, so a race to initialise it is harmless. Every other `static` in the slice is const-built and immutable: core vendor.rs and universe.rs `MemberIndex`, vocab table.rs `NAME_INDEX`. Every `thread_local!` (runner bootstrap, trade, identity, validate, exit_grid_policy; greeks bsm; indicators anchored) is `#[cfg(test)]` and counts calls only. There are no `static mut`, atomics, Mutex or RwLock in production code in the slice.
- **HashMap and HashSet use** (engine resume.rs and lib.rs, runner closed.rs and rank.rs). These are used only for membership checks. Output is always sorted canonically (`sort_canonically` by mask words; closed.rs sorts before output). I found no case where hash iteration order reaches output bytes or a digest.
- **indicators, costs, greeks, core, vocab.** None of these crates has threading or shared mutable state.
- **runner auto_probe** (lib.rs:1142). It uses a `RefCell` failure slot inside a `&dyn Fn` reporter. The engine calls the reporter only from the serial level boundary, never from a lane thread.

---

<!-- conc-pass1/server1.md -->
### conc-pass1 / server1: verdict: 1 medium (extends sibling recovery-4 with the main trigger it missed) and 1 low; no high. 2 findings.

Slice: `crates/api/src/server.rs` lines 1-17000 at 331b05c. Covered: every Mutex, RwLock and atomic, the spawns, connection handling (`Slots`, `LimitedListener`, `HeadDeadline`), the shared `Site` state, the run and sweep slots, and the shutdown wiring the slice hands to `serve`. I read source only and did not run cargo.

Already reported in this pass and still present, so not re-counted: recovery-5 and runs-3 (`pull_run_stop` holds the `site.run` std mutex across `recovery_control::stop` fsyncs, server.rs:10984-10995); pull2 (the cash-session cache takes an exclusive `try_lock` even to read; its in-slice caller is server.rs:7454-7461); hunt-api-1 (a hand pull is cancelled by a client disconnect, with no Drop guard in `broker_run`, server.rs:7806-7944, still present); autopilot-4 and hunt-api-2 (abort and runtime drop at shutdown).

## server1-1 (medium): two parallel feed legs of one Pull press refuse each other's audit records

- **Where:** crates/api/src/server.rs:10616-10628 (`recorded_fact`) and 10663-10690 (`recorded_with_failures`). Reached from `broker_answer` (6520+) for every leg of a press, because `pullrun::conduct` runs one chain per feed in parallel (pullrun.rs:871 `tokio::spawn(run_chain(..))`, and the doc at pullrun.rs:915 says "Feeds run in parallel"). Each leg is `server::pull_spot` (pullrun.rs:687). The lock is in crates/api/src/audit.rs:1190-1205.
- **Code:**
  ```
  let mut file = Flock::try_lock(OpenOptions::new().read(true).append(true).create(true).open(&self.path)..., ...)
      .map_err(|e| format!("...cannot take the journal append lock; another writer may be appending, so this record was refused rather than interleaved — {e}"))?;
  ...
  file.write_all(&record.image())?; file.sync_all()?;
  ```
  The comment at server.rs:6937-6950 says it directly: "this function runs once per LEG and the legs of one press run CONCURRENTLY".
- **Why it is wrong:** recovery-4 found the refusal, but as a recovery-plus-hand-pull collision it rated low. The common trigger is the operator's ordinary multi-feed press, with no recovery involved. Every leg of every feed appends to the same `<store>/audit/pull.journal` (`Site::journal`, server.rs:5613). `flock` conflicts between two open file descriptions in one process, and `try_lock` does not wait. So two legs whose appends overlap within one `write_all` plus `sync_all` window refuse each other.

  The window is wide. `recorded_with_failures` appends one record and fsync per failed member, in a loop, so a Dhan leg with 100 failed instruments holds the lock on and off through 100 fsyncs. A Groww leg finishing at any point in that loop loses its run record. Its later member records are also refused one by one, while the Dhan loop counts "the rest are NOT on disk".

  A background press has no answer page that anyone reads. The only trace is the `note_append_failed` event, and the run's bars sit in the store with no `/audit` row. The interleaving the refusal guards against cannot happen in-process anyway, because each record is a single 256-byte `write_all` on an `O_APPEND` handle. So in-process, the refusal buys nothing.
- **Repro:** Press Pull with two feeds, dhan and groww, for one month over the tracked universe. Chain A (dhan) finishes a leg in which N members failed and enters the `recorded_with_failures` loop: `append` → `try_lock` OK → `sync_all`, N times. Chain B (groww) finishes its leg during that loop. `recorded_fact` → `append` → `try_lock` returns `EWOULDBLOCK`, and B's facts read "Recorded: NO — this run is NOT in the journal". The groww month's bars are durable with no run record. A also loses every failure record whose `try_lock` lands during B's own appends. The same thing happens between a press leg and a `POST /pull/fno` on a third feed, and between any of these and a `refused` record from a bad request (server.rs:6147).
- **Fix:** Serialise appends inside the process before the flock: a `static APPEND: Mutex<()>` (or one on `Site`) held around `appended`, so in-process writers queue for one fsync. Keep `Flock::try_lock` as the cross-process guard only. Or move the append onto `spawn_blocking` and take the flock blocking (`lock()`), so the tokio worker is not the one that waits.

## server1-2 (low): Ctrl-C during a hand `/pull/spot` or `/pull/fno` walk does not stop the process, and a second Ctrl-C does nothing

- **Where:** crates/api/src/server.rs:11014-11122 (`pull_spot` runs the whole `broker_run` inside the request) and 14833-14935 (`pull_fno`, which its own comment says "can run for half an hour"). Shutdown is wired at server.rs:17115-17127 (`axum::serve(..).with_graceful_shutdown(..)`) and main.rs:36 (`Box::pin(tokio::signal::ctrl_c())`).
- **Code:** `axum::serve(listener, app).with_graceful_shutdown(async move { let _ = shutdown.await; }).await`. The per-instrument stop check in `broker_run` (server.rs:7840) reads `site.autopilot.stopped(epoch)`, and nothing on the shutdown path bumps that epoch.
- **Why it is wrong:** Graceful shutdown in axum 0.8 (Cargo.lock: 0.8.9) stops accepting, then waits for every connection task. An HTTP/1 connection with a request in flight completes that response first. A hand pull answers only when its whole walk ends: up to 800 instruments, "five to thirty-seven minutes" a month (server.rs:7836-7838), or a 30-minute F&O walk. So after Ctrl-C the process keeps fetching from the vendor and writing to the store for the rest of the walk. It prints nothing until `serve` returns.

  `tokio::signal::ctrl_c` installs a process-wide SIGINT handler that is never removed, so the operator's second Ctrl-C is swallowed too. The only way out is SIGKILL. SIGKILL then lands mid-walk, so the run record is never journalled: the hunt-api-1 or autopilot-4 outcome, reached by the operator's own escalation.

  The cheap stop the code already has, the pause epoch that `broker_run` checks before every instrument, is never signalled on shutdown.
- **Repro:** `api serve`, then POST `/pull/spot` with `target=` for the whole tracked universe and a live feed, then press Ctrl-C after instrument 5. The listener closes and requests continue through instrument N. Press Ctrl-C again: nothing happens. `kill -9`: no `pull.journal` record for the instruments already landed.
  - UNVERIFIED: the axum serve source was not available on this box (no registry checkout), so the claim that it waits for in-flight connections rests on axum 0.8's documented graceful-shutdown behaviour, not on reading its source. No probe was run (no cargo, by instruction).
- **Fix:** Wrap the shutdown future so that it first calls `site.autopilot.pause()` (bumping the epoch). Every `broker_run`, hand or autopilot, then breaks at its next instrument, and the leg journals its partial run ("stopped after k of n"). Also bound the drain with `tokio::time::timeout` after the signal. Optionally, a second `ctrl_c()` inside the wrapper exits immediately.

## Checked and clean

- `Slots` / `LimitedListener` / `Slot` (server.rs:16793-16860): the CAS uses AcqRel/Acquire, the slot is bound before the accept await so cancellation gives it back, and `notify_one` stores a permit, so a free racing the wait is not lost. The cap of 0 is clamped to 1.
- `HeadDeadline` (16880-17073): the alarm and the parked-waker re-arm are correct. The body is not covered by the deadline and pipelining can be cut, but both are documented in docs/06-limits.md under D-1200.
- `Site::reparse` (5434-5503): `reload_lock` serialises reparses. Validation runs under a scoped read guard that is dropped before `write()`, so there is no read-then-write self-deadlock. The single-guard fix at 1195 is present.
- No `RwLockReadGuard` or `MutexGuard` crosses an `.await` in a handler. The compiler would refuse, because axum handler futures must be `Send`.
- `census_now_stamping` (3605-3735): stamps are taken before the read, and the double-checked install keeps the canonical Arc. A slow older reader can overwrite a newer entry, but the next request's stamps mismatch and it re-reads. That costs a rebuild and never serves a stale answer.
- `store_wire::Cache` and `audit_json::RollupCache` are keyed by `Weak::ptr_eq`. A live `Weak` keeps the allocation, so a freed census Arc's address cannot be reused (no ABA).
- `await_budget` (8572-8622): lock order is budgets, then governor, everywhere. Both are released before the sleep. The transport takes only the inner lock.
- `pull_run` (10906-10945): check and claim happen under one `site.run` take, before `tokio::spawn`.
- Seats: `pull_spot`, `pull_fno` and `recovery_spot` each take their feed's seat before any socket and hold it across the walk. The autopilot takes all seats. I found no unseated store writer in the slice.
- The slice makes no direct filesystem writes (grep for `fs::write`, `OpenOptions`, `create_dir`, `rename` outside tests found none). All durable writes go through `audit`, `pull::ingest` and `recovery_control`, which belong to other slices.

---

<!-- conc-pass1/server2.md -->
### conc-pass1 / server2: verdict: no high or medium defects in the slice; 2 low findings (one of them unreachable in production)

Slice: `crates/api/src/server.rs` lines 17000 to the end, plus `crates/api/src/ingest.rs` and `crates/api/src/assets.rs`. Commit 331b05c. I read the source only and did not run cargo.

The production code in this slice is small. Most of server.rs 17000+ is `#[cfg(test)]`: modules at 17134-17450, 18583-31114, 31116-31270, 31413-32110 and 32354-32750, plus the test modules declared at 33528+. What is left is the connection wrapper (17000-17133), the serve lock and the serve arm (17451-18582), universe/indexmap/vocab/calendar handlers (31271-33527), ingest status/queue (ingest.rs:1-2246), and static assets (assets.rs:1-1031). ingest.rs has no production disk writes; `status_json` and `queue` only read the autopilot's in-memory state.

## Findings

### server2-1 (low): the front-end root and build state are cached at startup, so `/` stays 503 after the operator builds the bundle the 503 page tells them to build

- File: `crates/api/src/assets.rs:681-701` (`Assets::new`) and `:798-800` (`respond`); `:988-1021` (`not_built`). The value is constructed once at `crates/api/src/server.rs:18432`.
- Code:
  - `let root = std::fs::canonicalize(&named).ok().filter(|resolved| resolved.is_dir());` (read once, in `new`)
  - `let Some(root) = self.root.as_deref() else { return self.not_built(); };` (every request)
- Why it is wrong: `root: Option<PathBuf>` is fixed for the life of the process. If `web/build` is absent at startup, every page request answers 503 "The front end is not on disk ... What produces it: `npm --prefix web ci && npm --prefix web run build`" for the rest of the process, even after that command has succeeded and the files are on disk. The page and the banner never say that a restart is also needed. The operator follows the printed instruction and still gets the same 503. That is a cache serving a stale answer after the file changed, and the refusal names an incomplete remedy (CLAUDE.md §4: name the reason). The reverse case is also cached: `root` is the canonical target, so if `web/build` is a symlink that is retargeted (an atomic `build -> build.v2` swap) and the old target is removed, every request canonicalizes under the dead old root and answers 404 or the 503 shell page until restart.
- Repro: (1) `mv web/build web/build.off`. (2) Start `api serve`. The banner says `NOT BUILT`. (3) `mv web/build.off web/build`, or run the printed npm command. (4) `GET /` still answers 503 "The front end is not on disk", because `self.root` is still `None`.
- Minimal fix: when `root` is `None`, re-resolve it per request (`canonicalize(&self.named)` on that branch only, so the served path costs nothing extra). Or keep the cache and add one sentence to `not_built` (and to `Build::note`'s NOT BUILT/NO SHELL text) saying the server must be restarted after the build. Re-resolving is the more honest of the two.

### server2-2 (low; unreachable in production, reachable in-process): dropping a pass-through `ServeLock` frees the in-process key that a still-live holder owns

- File: `crates/api/src/server.rs:17582-17589` (`impl Drop for ServeLock`) together with `:17653-17673`.
- Code:
  - take: `if !held.insert(key.clone()) { return Ok(ServeLock { held: None, root: key }); }`
  - drop: `drop(self.held.take()); if let Ok(mut held) = serving_roots().lock() { held.remove(&self.root); }`
- Why it is wrong: the second in-process serve of a root gets a pass-through (`held: None`), but its `Drop` still removes the key unconditionally. The set has no owner or refcount, so whichever value drops first frees the key, and the file lock goes with the value that is `Some`.
- Repro, interleaving in one process (the case the doc at 17591-17600 says is allowed, "tests that run in parallel against the developer's real store root"):
  1. A: `take_serve_lock(R)` gives `held: Some` (file locked, key inserted).
  2. B: `take_serve_lock(R)` gives `held: None`.
  3. A drops: unlock, key removed.
  4. C: `take_serve_lock(R)` inserts the key, opens a new OFD, and `try_lock` succeeds (`Some`).
  5. B drops: removes the key that C owns.
  6. D: `take_serve_lock(R)` inserts the key, opens a third OFD, and `try_lock` fails against C's OFD. It refuses "another brutex api is already serving this store" and quotes its own pid.

  A second variant: if A drops while B is still serving, the flock is released and the key removed while B is still serving R, so a second process can take `serve.lock` and serve the same store alongside B.
- Reachability: `api::main` calls `server::run` exactly once, so a production binary never holds two `ServeLock`s. The tests I found that exercise the lock use unique scratch roots (`crate::scratch::path("serve-lock-…")`), so I found no current test that overlaps. This is a latent defect in a path the doc declares supported, not a live production bug.
- Minimal fix: replace the `BTreeSet<PathBuf>` with a `BTreeMap<PathBuf, usize>` refcount. Increment on take, including the pass-through, and decrement in `Drop`, removing at zero. Only the `Some` holder unlocks the file, and it must not unlock while the count is above 1 (otherwise the second variant above remains). Alternatively, make the pass-through return the refusal instead of `Ok`.

## Previously reported, re-checked and still present (not counted)

- hunt-api-2 (medium, KNOWN): Ctrl-C releases `serve.lock` while `spawn_blocking` sweeps and backfills keep writing. This is still present at 331b05c. `server.rs:18555-18569`: `flying.abort(); code` and then `one_server` drops at the end of the arm. `main.rs:31` is still `#[tokio::main]` with no `shutdown_timeout`. The D-0693 explicit unlock in `ServeLock::drop` releases the store before the runtime's blocking pool has drained, so a second `api serve` can start next to the ghost writers. The comment at 18562-18566, "A sweep aborted mid-append is safe by construction", is about torn appends. It does not address two writers, and `abort()` does not stop a blocking task. Nothing material to add beyond the earlier report.
- errpaths line 58 (`server.rs:17533/17691` `if let Ok(mut held)` on a poisoned lock). The lines have moved to 17585 and 17822 (`release_root`). This is still latent only: the guarded code is `BTreeSet::insert/remove`, which cannot panic.

## Checked and clean

- `take_serve_lock` order: it checks that the root exists, canonicalizes it, takes the in-process key, opens `<canonical>/serve.lock` (no truncate), takes `Flock::try_lock`, and only then stamps and truncates to the stamp's length. Truncation never happens before the lock is held. A crash between `write_all` and `set_len` leaves the new stamp on line 1, and readers quote only line 1. A crash at any point releases the OFD lock in the kernel, so no stale lock file wedges the next start. The failure paths release the in-process key, and on a stamp failure the flock too (`drop(file); release_root(&key)`).
- The serve lock opened through the canonical path, not the configured symlink: a retarget after admission cannot move it. Two processes reaching one directory through different paths, such as bind mounts, still contend on the same inode.
- The `open_in_browser` child: std opens files `O_CLOEXEC`, and D-0693's explicit unlock covers the duplicate-descriptor case anyway.
- `Assets::respond`: it reads instead of `is_file()` then `read`, so there is no exists/read TOCTOU. The `missing` counter is Relaxed and gates only the log-line cadence; it publishes no data.
- `Build::read` and `newest_under` run once at startup and feed only the banner and the first event, never a per-request decision.
- `ingest::status_json`: `paused`, `seat` and the status snapshot are three separate reads, so they can be momentarily inconsistent. The output is display-only and is refused by name when poisoned. Not a finding.
- `universe_resolve`: the universe read guard is released before the ~150-request crawl.
- `indexmap_reading` and `calendar_json_reading`: they run on the admitted blocking pool. HashMap keys are sorted before emission (33397-33398). The calendar cache is keyed by the stamp taken before the census was read; the stamp's definition (`census_now_stamped`, server.rs:3564) is outside this slice.
- `HeadDeadline` / `serve_limited`: no locks and no disk. The slot counter is constructed here, and its accounting (`Slots`) is defined before line 17000, outside this slice.

---

<!-- conc-pass1/runs.md -->
### conc-pass1 / runs — launch, cancel and finish lifecycle (sweeprun, pullrun, mastersrun, indexstoplaunch)

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

---

<!-- conc-pass1/autopilot.md -->
### conc-pass1 / autopilot — 4 findings (2 medium, 2 low)

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

---

<!-- conc-pass1/recovery.md -->
Verdict: 6 findings in the recovery and audit slice at 331b05c: 2 medium, 4 low. None of them is a data race on bytes. Two are crash or contention states that leave the recovery feature wedged or bill a retry budget for a request that was never sent.

Slice: crates/api/src/recovery.rs, recovery_journal.rs, recovery_control.rs, credential_law.rs, audit.rs, operation_audit.rs (non-test). Method: read the source only. No cargo was run.

## recovery-1 (medium): a seat refusal before any network call is charged as a used vendor attempt

- **Where:** crates/api/src/recovery.rs:1550-1552, together with crates/api/src/server.rs:8022-8025.
- **Code:**
  ```
  append_attempt(journal, attempts, item.clone())?;          // InFlight, attempts += 1, synced to attempts.bin
  let asked = checked(&item.body, ...)?;
  let run = server::recovery_spot(site, &asked).await?;       // first line: take_seat(...).ok_or_else(|| "the selected feed already has an active pull")
  ```
- **Why it is wrong:** The reservation is made durable before the seat is taken. `recovery_spot` can refuse with `Err` before any vendor request: the zerodha feed's seat is held by someone else. The `?` then leaves the reserved InFlight attempt on disk.

  On the next run, `reconcile_pending` (recovery.rs:1046) picks that attempt up. `reassessed` turns it back into `Queued` with `attempts` unchanged, so the budget unit stays spent. Three collisions take the gap day to `Exhausted`. Shared budgets are never refunded, so that day can never be retried, even though no request was ever sent.
- **Who can hold the seat while recovery owns `site.run`:**
  - A `/pull/spot` with vendor=zerodha. It takes the seat at server.rs:11087 and never checks `site.run`.
  - An expired F&O walk. It holds the seat for the whole walk (server.rs:14899).
  - The autopilot. Its check of `site.run` followed by `take_every_seat` is a TOCTOU (autopilot.rs:3162-3172). It can read "not pressing" at t0 and take every seat at t0+ε, after recovery has claimed at t0+δ.
- **Repro:**
  1. Recovery is in `retry_day` for day D, and the operator posts a zerodha `/pull/spot` that holds the seat.
  2. Recovery appends InFlight with attempts=1 to attempts.bin and syncs it.
  3. `recovery_spot` returns `Err`. `execute` returns `Err` and the run ends BLOCKED.
  4. An explicit restart reassesses: Queued, attempts=1.
  5. Repeat twice more and D is Exhausted with zero vendor requests made.
- **Fix:** Take the feed seat in `retry_day` before `reserve`/`append_attempt`, and pass the held `Seat` into `recovery_spot`. Alternatively, treat a pre-network seat refusal as a refusal that never reserves. Either way, reserve only once nothing can fail before the network call except the call itself.

## recovery-2 (medium): a crash, or a failed write or sync, during the first activation leaves an empty active.bin that refuses every later start, prepare and resume

- **Where:** crates/api/src/recovery.rs:777-794 (`seeded`), together with recovery.rs:262-268 (`active_history`).
- **Code:**
  ```
  let mut active = Journal::open(&active_path(site)).map_err(failure)?;   // create(true): creates 0-byte active.bin, syncs it and the parent dir
  ...
  active.append(pointer).map_err(failure)?;
  ```
  and:
  ```
  if history.latest.is_empty() {
      return Err("active recovery pointer is empty; prior activation is unverified".to_owned());
  }
  ```
- **Why it is wrong:** On the first activation `Journal::open` durably creates an empty active.bin before the pointer is written. If the process dies between those two lines, or `append` fails with ENOSPC or EIO (the handle poisons, nothing is written), the empty file persists.

  Every entry point then refuses:
  - `resume` at boot, via `active_history`.
  - `start`, via `preflight_submission` → `preflight_plan` → `active_history`.
  - `seeded`, via `preflight_plan` at recovery.rs:537 (file numbering ~ line 735).
  - `prepare_successor`, via `active_history`.

  No code path ever appends to an empty active.bin. The empty file cannot stand for an activation that did vendor work: `drive` is only spawned after `seeded` returns `Ok`, after the pointer append. Even so, the feature is wedged until someone deletes the file by hand. Nothing tells the operator that deleting it is safe, and the message claims "prior activation is unverified".
- **Repro:**
  1. Fresh store. POST `/pull/recovery` (start).
  2. Kill -9 the process after recovery.rs:777 returns and before :794. Or fill the disk so that the 1024-byte append fails.
  3. Restart. The log shows "Recovery NOT resumed: active recovery pointer is empty…".
  4. Every later POST answers 503 with the same reason, for every plan, forever.
- **Fix:** Do not create active.bin empty. Either:
  - write the first pointer record to a temp name, fsync it, `rename` it to active.bin and fsync the directory; or
  - in `active_history`, treat a zero-length active.bin as "never activated" when no plan seal or attempts history contradicts it. That case is provable here because `seeded` writes nothing after `Journal::open(active)` except the pointer.

## recovery-3 (low): `reconcile_pending` iterates a HashMap, so vendor request order and journal bytes differ between runs

- **Where:** crates/api/src/recovery.rs:1046-1057.
- **Code:**
  ```
  let pending: Vec<_> = attempts.latest.values().filter(...).cloned().collect();
  for mut item in pending { ... assess ... append_attempt(journal, attempts, item)?; }
  ```
- **Why it is wrong:** `latest` is a `HashMap<[u8;32], Record>` with a random per-process seed. Over the same store and the same attempts.bin, two runs reconcile pending gap days in different orders. That changes three things:
  - the order of `assess` calls;
  - the order in which records are appended to attempts.bin and the plan journal, so the files differ byte for byte;
  - which items are left unreconciled when a STOP lands mid-loop (`stopping` is checked per item).

  This breaks §3 rule 5 (same inputs, same bytes). The journal already keeps a deterministic first-seen order in `attempts.order`, and it goes unused here.
- **Repro:** Seed attempts.bin with two Queued gap days. Run a reconciliation twice from the same snapshot, in two processes. The record order of the appended attempts.bin tail differs between them with probability about 1/2.
- **Fix:** Iterate `attempts.order` and look each key up in `latest`:
  ```
  attempts.order.iter().filter_map(|k| attempts.latest.get(k))
  ```

## recovery-4 (low): the audit journal refuses, instead of waiting, when two in-process writers append at once, and recovery then loses the receipt of a vendor run that already committed bars

- **Where:** crates/api/src/audit.rs:1190-1205 (`Flock::try_lock` in `appended`), together with server.rs:8049 (`journal.append(&record)?` inside `recovery_spot`).
- **Code:**
  ```
  let mut file = Flock::try_lock(OpenOptions::new()...open(&self.path)?, ...)
      .map_err(|e| format!("...cannot take the journal append lock; another writer may be appending, so this record was refused rather than interleaved — {e}"))?;
  ```
- **Why it is wrong:** The api process is the only writer of audit/pull.journal (serve.lock excludes a second server). But inside that one process, writers on other feeds append concurrently: recovery on zerodha alongside a `/pull/spot` on dhan, or the member-failure loop. A non-blocking flock turns ordinary in-process contention, lasting one write plus fsync, into a hard refusal.

  In `recovery_spot` the refusal comes after `broker_run` has committed bars. The receipt is lost, `retry_day` returns `Err`, and the recovery ends BLOCKED. The concurrent dhan pull can just as well be the one told "NO — this run is NOT in the journal". It is loud (an Error event), but the cause is a scheduling collision, not a fault.
- **Repro:**
  1. Thread A, recovery: `recovery_spot` → `journal.append` takes the flock and is inside `sync_all`.
  2. Thread B, `/pull/spot` with vendor=dhan, finishes and calls `site.journal().append`. `try_lock` returns `WouldBlock`.
  3. B's receipt is refused. Swap A and B, and recovery's receipt is refused after its bars landed.
- **Fix:** Serialise in-process appends with a process-wide `Mutex` around `appended`, or take the flock blocking (`lock()`). Keep `try_lock` only as the cross-process guard.

## recovery-5 (low): the `site.run` std Mutex is held across fsyncs on a Tokio worker

- **Where:**
  - crates/api/src/recovery.rs:560-576 (`claim` → `recovery_control::clear_stop` → `persist`).
  - crates/api/src/server.rs:10984-10995 (`pull_run_stop` → `recovery_control::stop` → `persist`).
- **Code:** `let mut held = site.run.lock()...; ... crate::recovery_control::clear_stop(site, id)?;`. `persist` opens and replays the stop journal, then runs `sync_all` on the file, then on `audit/recovery-v1`, `audit/` and the store root. That is up to five fsyncs, all done while `held` is alive, inside an async handler.
- **Why it is wrong:** Every other `site.run` taker blocks its Tokio worker thread for the whole fsync chain: the `/pull/run.json` poll, the recovery page, the autopilot tick, `update()` from a running recovery worker. On a slow or failing disk this stalls unrelated requests and can starve the runtime. The lock order (run, then active) is correct, as hunt-conc noted, but I/O under the lock is not covered there.
- **Repro:** On a device with fsync latency L, POST `/pull/run/stop` during a recovery run. Concurrent GETs of `/pull/run.json` stall for about 4L, and each pins a runtime worker.
- **Fix:** Persist STOP or clear outside the `run` guard on `spawn_blocking`. Keep the in-memory flag change under the guard and re-check the active id afterwards. Or make `site.run` a `tokio::sync::Mutex` and move the persist to a blocking task.

## recovery-6 (low): a failed terminal control append replaces the real error that ended the run

- **Where:** crates/api/src/recovery.rs:821-827 (`drive` worker).
- **Code:**
  ```
  control.status = if answer.is_ok() { Status::Verified } else { Status::Blocked };
  journal.append(control).map_err(failure)?;
  answer
  ```
- **Why it is wrong:** The usual reason `execute` fails mid-run is that a plan-journal append failed, through `append_attempt` or `journal.append(item)`. That failure poisons the plan `Journal`. The terminal `append(control)` then fails with "recovery journal is poisoned after uncertain I/O; drop and reopen it", and the `?` returns that string instead of `answer`.

  The root cause (ENOSPC, EIO, a foreign length change, a replaced inode) appears nowhere: not in `progress.finished`, not in the `pull.recovery` telemetry event. The durable state itself is correct (control stays InFlight, so the next boot resumes). Only the diagnosis is lost.
- **Repro:** Make the plan journal's write fail once, for example with a full disk during `append_attempt`'s second append. The page and telemetry then say only "poisoned …", never "No space left on device".
- **Fix:**
  ```
  let terminal = journal.append(control).map_err(failure);
  match (answer, terminal) {
      (Err(a), Err(t)) => Err(format!("{a}; terminal state not recorded: {t}")),
      (a, t) => t.and(a),
  }
  ```

## Checked and found sound
- **recovery_journal open/append:** The exclusive flock covers the handle's whole lifetime. The length is re-checked after replay. File and parent are synced before the index is exposed. Each append checks inode and length, writes, syncs and only then publishes. Uncertain I/O poisons the handle. A torn tail refuses rather than truncating (deliberate and documented). The CRC covers 0..1020. The lock guard sits on a dup of the same open file description, with an explicit unlock on drop. `append_new` publishes only after a single sync.
- **`tail`:** It takes no lock but bounds the snapshot by checking the length before and after. 1024-byte records sit at 1024 alignment, so a ragged length cannot be observed from one in-flight `write`.
- **`snapshot`:** It takes a shared flock and refuses while a writer is active, with WouldBlock rather than reporting an empty inventory.
- **Crash at each step of `seeded`:**
  - after the plan create or seed batch;
  - after the InFlight control is written;
  - after attempts.bin is created;
  - after the pointer is appended (on a re-activation).

  Each leaves a state that the next explicit start or boot `resume` handles correctly. The one exception is the empty first active.bin (recovery-2). InFlight reservations survive and stay charged, by design.
- **STOP:**
  - `claim` (clear_stop + activate under `site.run`) is atomic with respect to `pull_run_stop`.
  - A STOP that lands during seeding is caught by the `stopping` re-check in `activate_durable`.
  - `idle` runs before the progress update that marks the run finished, so no new claim can interleave.
  - An empty stop journal is repaired by an explicit start.
- **Activation sequence:** It is computed under active.bin's exclusive lock, so `max_by_key` is deterministic. This was already noted in hunt-conc.
- **Two processes:** serve.lock (server2 report) excludes a second api server on one store, and no cli path writes `audit/recovery-v1` or `audit/pull.journal`.
- **audit.rs `look`/`page`:** Unlocked reads of whole fixed-stride records with a CRC check per record.
- **api operation_audit wrapper:** Begin runs before dispatch, the terminal uses `run_owed`, and busy maps to 429. The journal itself lives in the cli crate, outside this slice.
- **credential_law:** The only shared state is in the `#[cfg(test)]` Script. `Watch` is per run.

---

<!-- conc-pass1/apicache.md -->
### conc-pass1 / apicache — api read caches vs files changing underneath

**Verdict: 2 findings (0 high, 1 medium, 1 low).** Both are in-process cache-eviction defects. Neither serves wrong bytes. Each one makes a route refuse requests it should answer after a file underneath changes. Audited at commit 331b05c, by reading the source only (no cargo).

Slice: `crates/api/src` non-test files, excluding server, ingest, assets, sweeprun, pullrun, mastersrun, autopilot, recovery*, credential_law, audit and render.

---

## apicache-1 (medium): `/candidate-trades.json` keeps a stale `TradeReader` and refuses that candidate until restart

**Where:** `crates/api/src/candidatejson.rs:302-328` (`trade_page`). The refusal is raised at `crates/cli/src/candidate_trades.rs:1106-1108` (`TradeReader::page_locked`).

```rust
static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
...
if cached.as_ref().is_none_or(|held| {
    held.model != summary.model || held.root != root || held.identity != summary.identity
        || held.attempt != summary.attempt || held.digest != summary.digest || held.key != key
}) { /* cold TradeReader::open, replace slot */ }
let held = cached.as_mut().ok_or("candidate reader cache missing")?;
let rows = held.reader.page(offset, limit)?;      // Err leaves `held` in the slot
```

and in cli:

```rust
fn page_locked(&mut self, ...) {
    let generation = crate::result_set::file_generation(&self.file, &self.path)?;
    crate::result_set::require_generation_unchanged(self.generation, generation, &self.path)?;
```

**Why it is wrong.**
- The slot is keyed only on content: model, root, identity, attempt, catalog digest and candidate key. Every request reads `summary` fresh through `read_model`.
- The cached reader also pins the trade file's filesystem generation: device, inode, mtime and ctime (`result_set.rs:200-214`).
- Suppose the trade file's generation moves while its content and the catalog digest stay the same. `page()` then refuses with "...validated filesystem generation changed; ... Reopen to validate the complete manifest".
- The cache is not evicted on that error. The next request computes the same key, hits the same stale reader, and refuses again. This repeats for every page and for the unpinned first page too. Nothing reopens, even though the error text says to reopen.
- It clears only when some other candidate key is requested, or when the process restarts.
- Every sibling cache evicts on this exact condition:
  - `detail::Cached::with_verified` sets `*held = None`.
  - `indexstopcandlesjson.rs:186-192` and `indexstopvixjson.rs:162-166` say in their comments that a retry "may re-admit byte-identical restored files under the same original completion".
  - `indexstoprankingjson.rs:221-223` and `indexstopqualificationjson.rs:192-194` also evict.
- `candidatejson` is the one cache in the slice without eviction.

**Repro (one process, no race needed).**
1. Start the api. Open `/candidate-trades.json?identity=I&attempt=A&tier=0&rank=0&direction=long`. A cold `TradeReader` is cached for key K.
2. Change the trade file's ctime without changing its bytes. Any one of these works:
   - `ln <store>/.../trades-0-0-long.bin /backup/x` (link() bumps ctime; `cp -al` and rsnapshot-style backups do this to every file);
   - `chmod`/`chown`/`touch` on the file;
   - an operator deletes a capture directory that a crash left torn, and reruns the capture. `write_exact` is deterministic and idempotent, so the catalog digest is the same but the inodes are new.
3. Request the same URL again, with or without `digest=`. The response is a 503 "kept length N but its validated filesystem generation changed". Every later request for K gets the same 503 until restart. A cold `TradeReader::open` on the same files would succeed.

**Minimal fix.** Evict when a page fails, the same way `with_verified` does:

```rust
let page = held.reader.page(offset, limit);
if page.is_err() { *cached = None; }
let rows = page?;
```

Alternatively, refuse only when `require_current` fails, as `indexstopvixjson::project_cached` does. Either way the next request cold-opens and re-verifies every row and the seal.

---

## apicache-2 (low): an unpinned `/expression-search.json` request discards another viewer's pagination session

**Where:** `crates/api/src/expressionsearchjson.rs:129-144` (`render`).

```rust
let Some(reader) = Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? else { ... };
sessions.retain(|held| {
    held.root != root
        || held.identity != asked.identity
        || held.reader.progress().checkpoint != reader.progress().checkpoint
});
...
sessions.push_back(Session { root, identity, reader });
```

**Why it is wrong.**
- A `Session`'s reader holds `admitted`: the continuation anchors learned while walking pages (`cli/src/expression_search_reader.rs:209-218`).
- A pinned page with a cursor is accepted only if that cursor is in `admitted` (`page()`: "search cursor is not linked to this admitted snapshot").
- Pinned requests look up the session by checkpoint alone (`position(... checkpoint == Some(snapshot))`).
- An unpinned (first-page) request for the same identity opens a fresh reader. If the search has not written a new checkpoint, the fresh reader has the same checkpoint, and the old session (with all its learned cursors) is deleted and replaced by one whose `admitted` holds only the head.
- This happens exactly when the search is paused, stopped or exhausted, which is when people browse its history.
- One viewer's first-page load therefore breaks every other viewer's deep pagination on the same search.

**Repro (two clients, any order).**
1. Search S is paused at checkpoint C.
2. Tab A requests `identity=S` and gets `snapshot=C, next=N1`. It then requests `snapshot=C&cursor=N1` and gets `next=N2`. A's session now has `admitted={C,N1,N2}`.
3. Tab B (or A's own reload in another window) requests `identity=S` with no snapshot. `retain` drops A's session (same checkpoint C) and pushes a new one with `admitted={C}`.
4. Tab A requests `snapshot=C&cursor=N2`. `position` finds B's session (checkpoint C), and `page()` returns `Err("search cursor is not linked to this admitted snapshot")`. A must restart from page 1.

**Minimal fix.** On an unpinned request whose freshly opened checkpoint equals a held session's checkpoint, keep the held session and move it to the back of the LRU. Do not replace it. Optionally drop the fresh reader. The fresh reader is still needed to *learn* the current checkpoint, but the old one's `admitted` set is a superset of what the fresh one would hold.

---

## Checked and found sound (not findings)

- **`detail::Cached` (TRADES, FRONTIER, PARENTS, LEDGER; topjson SELECTION):**
  - `with_verified` evicts on a refresh refusal, and the next request reopens. The lock is read through poison.
  - No path nests two `Cached` locks. trades and frontier take PARENTS, then TRADES or FRONTIER, one after the other.
  - The `Results::refresh` and `read` shared `flock` waits out a cli append, so there is no torn-record false alarm.
- **topjson `Selection::extend`:** a mid-batch `read` error leaves `best`/`pairs` partly updated. `with_verified` then discards the whole slot (`*held = None`), so the partial state is never served.
- **`detail::recorded_underlying`:** the zero-length check races only toward "no run recorded". That is true for a header-less ledger.
- **booleanjson, booleanevidencejson, booleanoosjson, indexstopjson:**
  - A pinned request over a held reader reuses it and refuses a changed generation by design. The slot is not evicted.
  - The unpinned first page that the client must restart from always re-admits (`must_admit` / `completion.is_none()`), so they recover.
  - Their `lock().map_err("poisoned")` would wedge after a panic. I found no reachable panic under the lock (`start+n` sites are bounded by rows actually returned), so this is not reported.
- **indexstopcandlesjson, indexstopvixjson, indexstopqualificationjson, indexstoprankingjson:** `try_lock` (busy refusal, no queueing) plus eviction on currency failure.
- **`calendar_of::Cache`:**
  - The single-flight `Landing` drop wakes followers on a panic. Followers loop to a new leader.
  - The stamp is the caller's pre-read census stamp (D-0695).
  - A slow older-stamp leader can overwrite a newer entry in `held`. Only requests carrying that older stamp can hit it, so the cost is one extra derivation and never a stale answer.
- **`audit_json::RollupCache` and `store_wire::Cache`:**
  - Both are keyed by `Weak::ptr_eq`. The held `Weak` keeps the `ArcInner` allocation alive, so the address cannot be reused (no ABA).
  - A late publisher of an older snapshot only causes a recount.
- **`census::sized`:** one `O_NONBLOCK` open, size from that descriptor, and a capped read. The manifest is installed by temp+rename (`pull/src/manifest.rs:3006`), so a reader never sees a half-written manifest.
- **`livejson` CENSUS:** the lock is read through poison, and the refresh logic lives in cli (`cli::live::CensusCache`).
- **`logs::both_halves`:** a stable sort with a `(millis, seq)` tiebreak, served half first, so the order is deterministic. Torn tails are reported by `telemetry::tail`.
- **`detail::Permit`:** `fetch_update(AcqRel)` admission, and Drop releases the slot. `owed()` counts past the cap intentionally (D-1445).
- **booleanlaunch / indexstoplaunch `conduct`:** the status lock is held only for one assignment. `finished_micros = Some(started)` is overwritten by sweeprun.rs:3159.
- **No writes:** no non-test file in the slice creates, renames or writes store files. `folder.rs`'s `NEXT` and `isolated`/`scratch`/`emitted` are test-only.

---

<!-- conc-pass1/telemetry.md -->
Verdict: 1 finding (0 high, 1 medium, 0 low). In-process concurrency in crates/telemetry is sound. The one real defect is that two PROCESSES can share one sink directory and nothing stops them.

Commit audited: 331b05c. Slice: crates/telemetry/src (sink.rs, tail.rs, lib.rs; record/encode/clock read where the slice calls them). Method: source reading only. No cargo was run, so no measurement is claimed.

## telemetry-1 (medium): two `cli` processes append to and rotate the same `<store>/logs/cli` set with no lock. Sequence numbers fork, `reserve_run_id` hands out ids already used, `Tail::missing` hides real loss, and `ms` order stops being file order

**Where**
- crates/cli/src/lib.rs:2996: `.or_else(|| store.map(|s| s.join("logs").join("cli")))`. Every cli process resolves the same directory. `cli/src/main.rs:69` calls `cli::install_log()` for EVERY verb, before dispatch.
- crates/cli/src/lib.rs:2972-2976 (doc) claims *"each writer owning one means each owns its own `events.ndjson` ... No lock, no coordination, nothing to get wrong under concurrency"*. That holds for api versus cli. It does not hold for cli versus cli.
- crates/cli/src/lib.rs:2101: only `is_sweep_command` verbs take `execution_lease::Lease::acquire`. Non-sweep verbs (report, verify, pool, ...) run alongside a sweep and alongside each other, each with its own sink on the same files.
- crates/telemetry/src/lib.rs (install doc): *"two sinks on one path would each keep their own byte count, so each would roll the other's file out from under it"*. The guard is a per-process `OnceLock` only. `Sink::open` (sink.rs:732) takes no file lock and does not detect another live writer.
- sink.rs:746 `let (seq, last_at) = resume_point(&config.dir, config.keep_files);`, sink.rs:788 `reserved_run: AtomicU64::new(seq)`, sink.rs:1128-1129 (`stamp` and `seq += 1` against per-process state only).
- tail.rs:361 `let span = newest.checked_sub(oldest)?.checked_add(1)?;` and tail.rs:625 `.is_some_and(|floor| record.at_unix_millis < floor)` (`since` ends the walk on the first older record).

**Why it is wrong.** Each process keeps its own `seq`, `last_at` (ms clamp) and `bytes`, and all of them append through O_APPEND to one `events.ndjson`. The crate's invariants are "seq is consecutive in file order", "ms is non-decreasing in file order" and "a reserved run id is above every id in the log". All three are enforced only inside one process's mutex.

**Repro 1: run-id reuse across commands that never overlap (concrete, needs no tight timing).**
1. P_A = `cli sweep-all ...` (long). It opens the sink when the last line is seq S, so A's counter is at S.
2. While A is quiet, P_B1 = `cli report ...` opens. It resumes at S and reserves run id S+1 (sink.rs:1032; cli/lib.rs:2137). It writes "command started"/"command finished" with seq S+1, S+2 and run=S+1, then exits.
3. P_B2 = `cli verify ...` opens, resumes at S+2, reserves run id **S+3**, writes seq S+3, S+4 with run=S+3, and exits.
4. A emits its next event with its own counter: seq **S+1**. The newest line in the file now carries S+1. A emits again: seq S+2.
5. P_E = any non-sweep verb opens. `resume_point` reads the last decodable line (S+2), so `reserved_run` = S+2 and E reserves run id **S+3**, the id B2 already used.

Result: `/logs?run=S+3` (and `Query::from_run`) returns B2's and E's lifecycle events as one story. This breaks the guarantee written at sink.rs:620-629 (*"makes the next process start strictly above every id still present in the log"*).

**Repro 2: `Tail::missing` masks a real drop.** With A and B interleaved as above, the file holds seq S+1(B), S+2(B), S+1(A), S+2(A), S+3(A), and so on. An unfiltered `tail` gets more records than `newest - oldest + 1`, so `span.saturating_sub(len)` returns 0. A genuine hole, for example a `Dropped` event that burned a seq in either writer, is then reported as `Some(0)`. When the newest line comes from the lagging writer (A's S+2 after B's S+4), `checked_sub` underflows and `missing` is `None`, which callers cannot tell apart from "a filter was applied". The field the crate calls "the one number that reports a loss by the WRITER" is therefore wrong whenever two cli processes have overlapped.

**Repro 3: `since` truncation.** A runs `stamp(now_millis())` and gets t under A's mutex. B runs `stamp` and gets t+1 under B's mutex, and B's `write` lands first. File order is now B(t+1) then A(t). `Query::since(t+1)` walks backwards, meets A(t) first, `take_line` returns `true` at tail.rs:625, and the walk ends without B's record. The comment at tail.rs:604-612 says this early exit is sound only because one writer's clamp orders the file. Two writers break that.

**Repro 4: rotation (needs 8 MiB of cli events, rarer).** B's `roll` renames `events.ndjson` to `.1` (sink.rs:1284) and reopens. A's descriptor still points at the renamed inode, so A keeps appending NEWER events into `.1`. The tail reader assumes `.1` is entirely older than the current file, so the `since` early exit and newest-first order both break across the boundary. A later roll by either process shifts the other's live file further down the set. After `keep_files - 1` rolls by the busy writer, `remove_if_present(oldest)` unlinks the inode the quiet writer is still appending to. The quiet writer's `emit` then returns `Written` and counts `written += 1` for bytes that no path will ever show.

A side effect at open: if B opens while A's 41 KB line is half-visible, `ends_mid_line` (sink.rs ~1423) reports a torn tail that is not torn. B then spends its one stderr notice and sets `last_error` to "a record torn by a kill or a power cut" for a healthy file.

**Minimal fix.** Make the sink directory single-writer, in the place the crate already says the rule belongs:
- In `Sink::open`, open `dir/events.lock` and call `File::try_lock()` (std, stable since 1.89; the toolchain is 1.97.1). Hold the handle in `Sink` for its whole life. On `WouldBlock`, return `Err("<dir> is already being written by another process")`. The kernel releases the lock on exit or crash, so no stale lock file can block forever.
- In `cli::install_log`, when that refusal comes back, fall back LOUDLY to a per-process child `logs/cli/<pid>` (or refuse and say so), and have `api::logs` walk the children.

Either way `seq`, `last_at`, `bytes` and `reserved_run` each go back to having exactly one owner per set.

## Checked and found sound (no finding)

- **Level floors.** One packed `AtomicU16` (sink.rs `floors`). A relaxed single-word store and load cannot be observed crossed, and nothing else is published through it.
- **`claim_run`/`release_run`.** AcqRel CAS: two claimants cannot both win, and a loser cannot clear someone else's key. `set_run` has no production caller (already noted in hunt-conc.md).
- **`reserve_run_id`.** `fetch_update` is correct within one process. The cross-process failure is telemetry-1.
- **Poisoned `inner`/`last_error` mutexes.** Recovered with `into_inner`. A panic inside `line()` leaves only a burned seq (visible as a hole) and a buffer that is cleared on the next emit.
- **No lock held across stderr.** `report` runs after `drop(guard)` on every path, including the roll-failure path. `Debug` takes no lock. `health` takes the two locks one after the other, never nested, so there is no lock-order cycle.
- **`ms` monotonicity within one process.** The clock is read inside the lock and clamped. The clamp is resumed from disk at open (D-1325).
- **Rotation crash points.** A kill after unlinking the oldest file, after any middle rename, or after `current -> .1` but before `reopen` all restart cleanly. Open recreates `events.ndjson`, and `resume_point` takes the first non-empty file newest-first. No fsync of file or directory is intended (stated in the module doc), so a power cut losing a rename loses only stream history.
- **The failed-roll storm.** Stopped by `rotation_broken`, which is set and read under the inner lock.
- **A partial `write_all` on ENOSPC.** Terminated with `\n` before the lock is released, so the next event cannot fuse onto it.
- **Install race inside one process.** `OnceLock::get_or_init` plus the path comparison. The losing `Sink` is dropped. At worst its open wrote one extra torn-tail `\n`, which the reader skips as a blank line.
- **Tail reader against a roll mid-walk.** One `open` plus one `fstat` per file (no path-then-open TOCTOU). Dedup by `(dev, ino)`. A roll between files makes `.1` the already-read inode, which is skipped, and `.2` is then correct. A reader descriptor survives unlink. A writer caught mid-line is handled by `partial_tail`. Inode reuse and out-of-order records would need two 8 MiB rolls during one millisecond-scale walk, which is not a concrete failure path.
- **`last_record`/`ends_mid_line` at open.** Read-only, run once, and bounded at 64 KiB. They become wrong only with another live writer (telemetry-1).

---

<!-- conc-pass1/store1.md -->
### conc-pass1 / store1: store bar file writes, append, locking, readers vs writers

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

---

<!-- conc-pass1/store2.md -->
### conc-pass1 / store2: concurrency and state audit of store (repair, checksum audit, sidecar/block, layout, header, catalog, path, format, crc) and all of lake, at 331b05c

**Verdict: one low finding, and it is latent because `repair` has no production caller. Every other path in this slice is clean at 331b05c.**

Scope: `crates/store/src/{repair,checksum_audit,block,catalog,crc,format,header,layout,path,open_flags,lib}.rs` and every file in `crates/lake/src`. `file.rs`, `flock.rs` and `emits.rs` belong to the other slice. I read `file.rs` only where my slice calls into it: `open_or_create`, `open_existing`, `open_existing_audited` and `checksum_inputs`. I did not run cargo; every claim below comes from reading the source.

## Findings

### store2-1 (low, latent): a concurrent identical `repair::publish` loses its race with a misleading refusal

- **Where:** `crates/store/src/repair.rs:261` (the pre-check), `:353-361` (the reservation) and `:162`/`:434` (the reader's receipt refusal).
- **Code:**
  - `if exists(&reservation)? { let reader = RevisionReader::open(root, path, symbol_id, revision)?; ...`
  - `OpenOptions::new().write(true).create_new(true).open(&reservation), &reservation, "reserve revision", false,`
- **Why it is wrong:** The choice between a Created publish and a Reused retry depends on `exists(&reservation)`. No lock is held for that check; the source lock is taken later, and it is shared, so two publishers can hold it at once. Two callers can therefore make the same request at the same time, and the loser gets one of two answers. Neither is `Reused` or `Conflict`:
  1. **Both callers pass the `exists` check.** The loser's `create_new` returns `Io { operation: "reserve revision", kind: AlreadyExists, publication_may_be_visible: false }`. Per the field's doc (repair.rs:87), `false` means the caller may report that publication did not happen. But the exact revision the loser asked for is published, or about to be, by the winner.
  2. **The loser's check runs after the winner reserved but before its receipt.** The loser takes the Reused branch, and `read_receipt` returns `Incomplete(receipt)`, which displays as `"incomplete repair at …; preserved"`. That is the message for an abandoned publication, and here a live one is still in progress. REPAIR.md says an unfinished ordinal must be reviewed and replaced by another one ("After review a caller can choose another ordinal").
- **Repro (two processes, same root, path, symbol, revision, `expected_source` and `merged`):** P1 `exists(reservation)` returns false. P2 `exists(reservation)` returns false. P1 and P2 both take `shared_lock(source)` (shared locks are compatible). P1 `create_new(reservation)` succeeds. P2 `create_new(reservation)` fails with EEXIST and returns `Io{.., publication_may_be_visible: false}`. P1 finishes and returns `Created`. Variant: P2 starts after P1's line 362 and before P1's line 387; P2 then returns `Incomplete(<month>.repair-v1)`.
- **Consequence:** A caller that follows the documented contract treats the ordinal as failed or abandoned and publishes the same rows under the next ordinal. That gives two revisions of identical data, and ordinals are capped at 1,024 per month (`MAX_REVISIONS`) and never reused. Nothing is corrupted: the reservation serializes the writers and the pair stays unmixed.
- **Why latent:** grep finds no caller of `repair::publish` or `RevisionReader::open` outside `crates/store/tests/repair.rs` and the module's own tests.
- **Minimal fix:** Map `AlreadyExists` from the reservation `create_new` to a dedicated `RepairError::Busy` (or `Locked`), not `Io{visible:false}`. In the Reused branch, try the revision lock before reading the receipt: if the writer's exclusive revision lock is held, return `Busy`, and return `Incomplete` only when the lock is free and the receipt is still missing. The writer holds that lock from `open_or_create` through the receipt sync. Either way the caller can retry the same ordinal and get `Reused`.

## Checked and clean (no finding)

| area | what was checked | result |
|---|---|---|
| repair: crash points | Death after each of these steps: `create_dir_all`, reservation `create_new`/`sync_all`, `open_or_create` (lock, then bars, then `initialise`, then leaf `fsync_dir`), `append` (records fsync, then sidecar fsync, then slot fsync), readback, `sync_ancestors(…, false)`, receipt `create_new`, `write_all`, `sync_all`, final `sync_ancestors`. | Every intermediate state either refuses by name or serves the complete pair. Missing receipt gives `Incomplete`; a short receipt gives `InvalidReceipt` (exact 144-byte length check); a zeroed receipt fails the magic check. Nothing is ever resumed or overwritten. REPAIR.md documents that the ordinal is stranded. |
| repair: directory durability | `sync_ancestors` walks from the leaf to `root`, including `bar-revisions-v1/<n>`, and runs before the receipt and again after it. Lexical `parent()` always reaches `root`, because the physical path is `root.join(…)` and `Path ==` compares normalized components. | Clean. Unlike hunt-store-4 for the ordinary writer, every new directory entry is synced. |
| repair: lock order | Source shared lock (try), then revision exclusive lock (`open_or_create`, try). The reader takes only revision locks. Every acquisition is nonblocking. | No wait cycle is possible. |
| repair: source TOCTOU | The source header is compared to `expected_source` and every timestamp is rescanned while the shared source lock is held, and a writer needs the exclusive lock. | Clean. |
| repair: reader pinning | `RevisionReader::open` releases its own shared lock only after `open_existing` has taken a second shared lock through a separate descriptor. The receipt is compared to the opened header. | Clean. |
| checksum_audit: generations | dev, ino, len, mtime/ctime (ns) and nlink are compared between `fstat` on the held fd and `lstat` on the path, for data, sidecar and lock. They are compared before and after the full scan, and on every `read_record` both before and after the read. The shared month lock is held for the `AuditedBarFile`'s lifetime, so a cooperating writer's `try_lock` fails before it opens the bar file. | Clean. A same-length in-place rewrite inside the mtime granularity is excluded, because writers cannot take the lock and the only same-length write is the header slot, which follows a growing record write. |
| checksum_audit: interrupted append | A file left longer than `offset_of(n_valid)` by a crash between the record write and the slot write is refused as "non-exact extents". | Documented: docs/06-limits.md:9147 "The strict audit door (D-0525) refuses any bytes past the commit and is unchanged". |
| block: crash between sidecar seal and slot | `verify_through` / `sealed_past_the_commit` admits the longer sealed extent only on a CRC-extension proof bounded to the tail block's room. | Clean; covered by AF-40/42/48 and D-0688/D-0910. |
| header: torn or concurrent slot reads | A slot is selected by CRC, the highest generation, and a position equal to `generation % slot_count`. A misplaced slot is terminal. Fallback is bounded by `MAX_SLOTS`. | Pure logic. A torn 64-byte slot fails its CRC and the reader falls back to g−1. |
| catalog: walk under concurrent writers | The walk takes a snapshot. A directory that vanishes is counted `unreadable`, not dropped. Output is `sort_unstable` + `dedup` on a total `Ord`. The census is order-independent. | Deterministic. A month created mid-walk can be missed, which is inherent to a listing and not a defect. |
| layout / path / format / crc / open_flags | Searched for statics, atomics, locks, HashMap, threads and I/O. | None: only consts and pure functions. |
| lake | `LakeFile::open` does one `fs::read`, and everything after it is in memory. The `stranded` `AtomicUsize` (page.rs:550, reader.rs:645) is stored and loaded on the same thread: the page reader is boxed into the column reader and drained by `read_exactly` before the load, and the one-record probe forces the walk's final `Ok(None)` that stores it. Relaxed is correct for that. There is no HashMap, no cache, no rayon and no writer. | Clean. The uncapped `fs::read` is already o1store2-4. |

Not filed (outside the concurrency angle, or already known): `repair` uses blocking `File::open` for the lock, the receipt and the sync opens, so a planted FIFO would stall it. REPAIR.md scopes this out ("cooperating writers… do not defend against hostile…"). Also already known: hunt-store-2/4, attackdata-1 and the ragged-tail behaviour, all of which live in `file.rs`.

---

<!-- conc-pass1/pull1.md -->
Verdict: 3 findings in the pull HTTP/capture/credential slice at 331b05c (0 high, 1 medium, 2 low). No deadlock, no lock held across `.await`, and no data race found.

Slice: crates/pull/src/http.rs, capture.rs, resolve.rs, and the credential/token re-read path with its shared state. That covers pull/src/rate.rs `Governor`, pull/src/secret.rs `reread_after_rejection`, and the live re-read in api/src/credential_law.rs `Watch`, plus its callers in api/src/server.rs, because that is where pull's `CredentialPrint` is consumed. Method: source reading only. No cargo, no edits.

## Findings

### pull1-1 (medium): a throttle named in a 2xx body backs the shared governor off twice
- Sites:
  - crates/pull/src/http.rs:3077-3086 (`settle_answer` calls `weigh_parsed`)
  - crates/pull/src/http.rs:3180-3187 (`weigh_body_parsed`)
  - crates/api/src/server.rs:9759-9768 (`with_retry`, `Step::Again`)
- Code, in pull (`window_async` → `settle_answer` → `weigh_parsed` → `weigh_body_parsed`):
  ```rust
  if named == crate::refusal::Disposition::Throttled
      && let Some(lock) = self.governor.as_ref()
  { let mut g = lock.lock()...; g.record_throttled(); }
  return Some(named);
  ```
  This becomes `Err(FetchError::VendorRefused { status /* 200 */, named: Some(Throttled), .. })`.
- Code, in api `with_retry`, on the same error:
  ```rust
  if throttled
      && status != Some(429)
      && let Ok(budgets) = site.budgets.lock()
      && let Some(Some(shared)) = budgets.get(feed as usize)
  { shared.lock()...record_throttled(); }
  ```
- Why it is wrong:
  - `step(Some(200), _, Some(Throttled), ..)` returns `throttle_ladder` → `Step::Again { throttled: true }`. The status is 200, not 429, so api records the throttle again.
  - The source's governor is the same `Arc` as `site.budgets[feed]`, because the source is built with `.sharing(shared_governor(site, feed))` (server.rs:10325).
  - So one vendor answer applies `relax()` twice: two additive steps down, and credit is zeroed.
  - D-0322 (docs/05-decisions.md:24353-24356) says the wrapper records "only when throttled is set and the status is not 429, which is exactly the set the transport misses. Neither path now records twice". That held when the transport judged on status alone. P-63/D-0950 then taught `window_async` to read a refusal from a 2xx body and record `Throttled` itself. After that, "status != 429" no longer equals "the transport missed it". The 2xx-with-named-throttle case is now counted by both.
  - This is the double-count class D-0322 measured, where the allowance collapsed and a 213-instrument pull could not finish. Here it is reached through Dhan's documented habit of putting refusals under a 200 (P-63 cites Dhan's SDK testing `status == "failure"`).
- Repro:
  - A Dhan bars request is answered `200 {"status":"failure","errorCode":"DH-904",...}`.
  - `window_async` → `weigh_body_parsed` → `record_throttled()`. permitted drops by step_of(ceiling).
  - The function returns `VendorRefused{status:200, named:Throttled}`.
  - `with_retry` → `step` → `Again{throttled:true}`. `status=Some(200) != Some(429)` → `record_throttled()` a second time on the same governor.
  - The allowance drops two steps for one refusal. Every retry attempt (up to `THROTTLE_ATTEMPTS`) doubles again.
- Fix: in `with_retry`, narrow the condition to statuses the transport does not weigh. That is non-2xx and non-429: `throttled && status.is_some_and(|s| s != 429 && !(200..=299).contains(&s))`. Alternatively, have `window_async` record a named throttle in its `!is_success()` branch and delete the api block, so "the transport owns the feedback" (D-0322's rule) holds without exceptions. Add an api test: a 200 carrying DH-904 must move the allowance exactly one step.

### pull1-2 (low): `HttpSource::sharing(None)` drops the source's own governor, and api reaches that through a poison-swallowing `.ok()?`
- Sites:
  - crates/pull/src/http.rs:602-616 (`sharing`)
  - crates/pull/src/http.rs:644-651 (`wait_for_permit`)
  - crates/api/src/server.rs:5183-5190 (`shared_governor`)
- Code:
  ```rust
  // pull
  if self.governor.is_some() {
      self.charged_by_caller = governor.is_some();
      self.governor = governor;          // None replaces the descriptor's own governor
  }
  // wait_for_permit
  let Some(lock) = self.governor.as_ref() else { return Ok(()); };
  // api
  fn shared_governor(site, feed) -> Option<SharedGovernor> {
      site.budgets.lock().ok()?.get(feed as usize)?.as_ref().map(Arc::clone)
  }
  ```
- Why it is wrong:
  - The `charged_by_caller` doc says "a caller that does not share keeps the private governor `new` built and is gated here exactly as before". `sharing(None)` is "not sharing", yet it discards the private governor. The source then has no governor at all: `wait_for_permit` returns Ok immediately, and `record_throttled`/`record_success` are skipped.
  - api's `shared_governor` turns a poisoned `site.budgets` mutex into `None` with no word. Meanwhile `await_budget` (server.rs:8585) refuses by name on the same poison. So the same fault is a loud refusal on one path and a silent ungoverned source on another: the §4 shape.
- Repro (latent: it needs a panic while `site.budgets` is held, at server.rs:8585 or 9761):
  - After that panic, every `credentials.source(feed)` → `.sharing(shared_governor(..))` (server.rs:8005, 10325, 12197) yields a source with `governor: None`.
  - Any request path that does not first call `await_budget` sends ungoverned. The discovery and rolling `get`/`post_json` calls rely on `wait_for_permit` or on the caller charging.
- Fix:
  - In pull, make `sharing(None)` a no-op: `if let Some(g) = governor { if self.governor.is_some() { self.charged_by_caller = true; self.governor = Some(g); } }`.
  - In api, have `shared_governor` return `Result` and refuse on poison, as `await_budget` does.

### pull1-3 (low): a capture is created at its final name and written in place, so a crash leaves a torn "verbatim" fixture
- Site: crates/pull/src/capture.rs:236-262 (`write_new_capture_with_stamp`).
- Code:
  ```rust
  let opened = OpenOptions::new().write(true).create_new(true).open(&path);
  ...
  if let Err(why) = file.write_all(bytes).and_then(|()| file.sync_all()) { ... remove_file ... }
  return Ok(path);
  ```
  There is no temp name, no rename, and no fsync of `captures/`.
- Why it is wrong:
  - The cleanup arm only runs when the write returns an error. If the process dies (SIGKILL, OOM, power loss) after `create_new` and before `write_all`/`sync_all` completes, the final-named file stays on disk empty or partial.
  - Nothing marks it incomplete. A 0-byte file has no header. A partial one has a `bytes: N` line followed by fewer than N bytes.
  - The module's purpose is "a request the operator spends once becomes a fixture for ever" (capture.rs:31-32), and it is the verbatim evidence for vendor-defect triage (`record_unreadable`). A torn file looks like a short vendor answer, which is exactly the misdiagnosis the module exists to prevent.
  - The directory entry is also not fsynced, so after power loss a capture reported as written may be absent.
  - No production reader exists, so the damage is limited to fixtures an operator promotes by hand.
- Repro: an api pull hits its first Dhan rolling POST. `record` creates `captures/dhan-POST-0-p<pid>-t<stamp>-c0.txt`. The process is killed during `write_all` of a multi-MB body. On restart nothing removes or flags the file, and the new process writes its captures under a new stamp beside it.
- Fix:
  - Write to `<name>.partial` with `create_new` and `sync_all` it.
  - Then `std::fs::hard_link(partial, final)`. That is no-clobber, and it fails with AlreadyExists so the next attempt is used.
  - Then `remove_file(partial)` and `File::open(dir)?.sync_all()`.
  - Alternatively, append a fixed end-trailer line and document that a file without it is torn.

## Checked and clean
- capture.rs slot budgets: `fetch_update` decides capture under the race, so two threads cannot both take the last slot. The `kept()` pre-check in `keep_first`/`keep_unreadable` is only an early exit. File names carry seq, pid and a process stamp, and `create_new` is the authority, so two threads or two processes cannot overwrite each other (covered by tests at capture.rs:209, 237). A slot spent on a failed write is counted in `REFUSED` and emitted, not hidden.
- capture.rs `process_stamp`: `OnceLock` init is race-free.
- http.rs `pooled_client` / ssm.rs `POOL`: `OnceLock<Result<..>>` caches a build error for the life of the process. That is deterministic and the error is surfaced on every call, so it is not hidden.
- http.rs governor locking: every `lock()` is a std mutex taken and dropped before any `.await` (`wait_for_permit` computes `reserve` inside a block, then sleeps). Poison is recovered with `into_inner`, deliberately and with documentation. Lock order is api `site.budgets` → governor (server.rs:8585, 9761), and pull never takes `budgets`, so there is no inversion.
- rate.rs `Governor`: `advance_to` is monotonic (cursor never moves backwards when a thread with an older `now` locks second). In `reserve` the unchecked `charge` after `advance_to(at)` is safe: `wait >= shortfall_w` for every window, and `capacity >= len_micros` because `permitted >= 1`. `monotonic_micros` uses a `OnceLock<Instant>`. `ABSORBED_MICROS` is a Relaxed counter that publishes no data.
- `charged_by_caller` stops a double withdrawal on shared governors. Feedback (`record_*`) is taken exactly once per answer in pull, apart from pull1-1.
- Credential re-read (api credential_law.rs `Watch`, pull secret.rs): `Watch` is a per-run local (server.rs:7834, 11532, 14114) driven sequentially, so no state is shared across tasks. The comparison is by `CredentialPrint` (SHA-256 of length-prefixed secrets). A re-read returning a previously rejected print halts. A rotated source is re-wrapped with the shared governor in `reread_wire`. pull's `CredentialReader::reread_after_rejection` has no production caller, as its doc says. There is no token cache anywhere: `credentialed_source` re-reads config, identity and SSM on every call, so there is no stale-token cache to serve.
- resolve.rs: `crawl` is sequential, holds no shared state, writes nothing, and returns the snapshot. `to_wire`/`digest` iterate Vecs in crawl order. The `HashSet` in `MasterIndex::resolve` feeds only a sum, so no hash order reaches the bytes or the digest. Out of angle and not counted: `HttpDocuments::body_of` (resolve.rs:~727) decodes with `String::from_utf8_lossy`, silently repairing invalid UTF-8. This contradicts the `DocumentSource` doc ("a transport that cannot produce valid UTF-8 ... has already found something worth refusing").
- Threads: every `std::thread::spawn` in http.rs, capture.rs, resolve.rs and ingest.rs is inside `#[cfg(test)]`. There are no production spawns in the slice.

---

<!-- conc-pass1/pull2.md -->
### pull2 (conc-pass1): crates/pull/src, everything except http.rs, capture.rs and resolve.rs, at 331b05c

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

---

<!-- conc-pass1/cli1.md -->
### cli1: concurrency and state audit of the cli entry slice (pass 1, commit 331b05c)

**Verdict: 5 findings (1 medium, 4 low). The lease, the ledger lock and the batch fold hold up. The defects are in admission contention, invocation-journal recovery, thread-local audit binding under rayon, and log sharing between CLI processes.**

Slice: `crates/cli/src/{lib.rs, batch.rs, knobs.rs, operation_audit.rs, execution_lease.rs, main.rs}`. All findings come from reading the source. Nothing was built or run. I skipped the earlier reports (hunt-conc-1..8 and errpaths-1..9), including GAP13-13 (durable writes inside rayon workers in `batch::one`/`sweep_rungs`) and the knobs poison fallback (hunt-conc-6).

---

## cli1-1 (medium): a zero-length per-invocation journal makes that ID unreadable forever and breaks audit paging across it. The same state also shows up during a normal concurrent begin and is reported as corruption.

`crates/cli/src/operation_audit.rs:557-564` (begin)
```rust
    index.release().map_err(|u| error(u.why))?;
    let mut file = options()
        .read(true)
        .append(true)
        .create_new(true)
        .open(own(&base, id))
        .map_err(error)?;
    write_synced(&mut file, &image)?;
```
`operation_audit.rs:611-620` (read)
```rust
    let mut file = match options().read(true).open(own(&base, id)) {
        Ok(file) => file,
        Err(why) if why.kind() == io::ErrorKind::NotFound => return Ok(Some(started)),
        Err(why) => return Err(error(why)),
    };
    ...
    let bytes = length(&file)?;
    if bytes == 0 || at(&mut file, 0)? != started {
        return Err(error("invocation journal lost its indexed start"));
    }
```
`operation_audit.rs:660-664` (page): `(0..count).map(|offset| read(root, ID_BASE + last - offset)?...).collect()`

**Why it is wrong.** `begin` makes the index record durable and releases the index lock. Only after that does it create the per-ID file, and it writes that file's first record as a separate step. The module documents a missing per-ID file as "its unconfirmed indexed start" (`read` returns `Some(started)`). An empty file is the same state, one syscall later, but `read` reports it as the hard error "invocation journal lost its indexed start". Nothing ever repairs the file. `page` collects with `?`, so a single unreadable ID refuses every page whose window covers it. The cursor contract ("continue with the last returned ID") means the client never gets a page that steps past it.

**Repro A (crash or ENOSPC, permanent).** The disk fills. `cli sweep-stored ...` runs `begin`. The index append of 256 bytes succeeds, because index records are 256-aligned and 15 of every 16 appends land in a block that is already allocated. `create_new` succeeds, because it needs an inode and no data block. `write_synced(&mut file, &image)` fails with ENOSPC. `begin` returns Err and leaves `<store>/audit/invocations-v1/<id>.bin` at 0 bytes. The same state follows from a SIGKILL or Ctrl-C landing between line 562 and line 564. From then on, `operation_audit::read(root, id)` returns Err on every call, so the API's `persisted_status` for that attempt fails. `page(root, None, 32)` (the audit page) refuses until 32 newer invocations exist. Any `before` cursor whose 32-ID window includes `id` refuses permanently, and history older than it cannot be reached by paging.

**Repro B (no crash, transient).** Thread or process A is inside `begin` between line 557 (index released) and line 564. Thread B serves a GET audit page: `page` → `read(id)` takes the index's shared lock without trouble (A released it), opens the empty file, and returns "invocation journal lost its indexed start". That error does not carry the `BUSY` prefix, so the caller reports corrupt data, not contention.

**Minimal fix.** In `read`, treat `bytes == 0` the same as `NotFound` and return `Ok(Some(started))`, the documented unconfirmed state. Optionally, in `begin`, remove the just-created file on a failed first write (best effort). Better still, write the first record to a temporary name and `link`/`rename` it into place, so the per-ID file never exists without its start record.

---

## cli1-2 (low): CLI sweep admission fails as "busy" when it collides with ordinary browser polling, because the invocation index is locked non-blocking and the CLI never retries

`crates/cli/src/operation_audit.rs:522` (begin): `let mut index = Flock::try_lock(` ... `.map_err(lock_error)?;`. The lock is held through `write_synced` (write + `sync_all`) at :556.
`operation_audit.rs:599` (read): `Flock::try_lock_shared(index, ...)`. The lock is held across `index.sync_all()` at :608.
`crates/cli/src/lib.rs:2100-2112` (run_durable)
```rust
        let lease = execution_lease::Lease::acquire(&root).map_err(|why| why.to_string())?;
        let audit = operation_audit::begin(&root, operation_audit::Origin::Cli, command)?;
    ...
            "refused: required execution admission could not start: {why}. No command was dispatched."
            return FAILED;
```

**Why it is wrong.** The API's middleware (`api/src/operation_audit.rs:134`) runs `journal::begin` for every request to the 21 `AUDITED` routes. That list includes GET `/backtest/run.json`, `/live.json`, `/backtest.json`, `/trades.json` and `/engine/top.json`, all of which the open browser page polls. Each such begin holds the exclusive index lock across an fsync. Each `read`/`page` holds the shared lock across another fsync. The API maps a busy refusal to 429, where a retry is safe. `run_durable` instead turns it into a FAILED exit for the operator's sweep. No sweep is running, so the refusal is spurious. Whether a scripted `cli sweep-stored`/`range-all` gets refused depends on fsync latency and how often a browser tab polls.

**Repro.** Keep a browser tab open, polling `/backtest/run.json`. At t0 the HTTP begin takes the index lock and runs `write_synced` (fsync, about 5-30 ms on a disk). At t0+1 ms, `cli range-all ...` acquires the lease and calls `operation_audit::begin`. `try_lock` returns WouldBlock, and the command prints "required execution admission could not start: invocation audit busy: another operation holds the journal lock" and exits FAILED.

**Minimal fix.** In `begin` and in the CLI path, wait on the index lock with a bounded retry (for example up to 1 s with short sleeps) before refusing busy. The lock is only ever held for one append plus one fsync. An alternative is a blocking `Flock::lock` with a timeout. The readers could also skip `sync_all` while they hold the shared lock.

---

## cli1-3 (low): `execution_lease::probe` takes the exclusive lock, so a status poll can make a real launch fail with a false "another sweep owns this store's execution lease"

`crates/cli/src/execution_lease.rs:125-134`
```rust
pub fn probe(root: &Path) -> Result<(), Refusal> {
    ...
    let held = lock(file, &path)?;      // Flock::try_lock: exclusive
    verify(&held, &path)?;
    held.release().map_err(unavailable)
}
```
`execution_lease.rs:104-113` (acquire): `let held = lock(file, &path)?;` maps `WouldBlock` to `Refusal::Busy`, whose text is "another sweep owns this store's execution lease; no new work was queued".

**Why it is wrong.** `probe` is meant as a read-only snapshot. The doc says "Checks current admission without creating a file or changing a result". But it takes the same exclusive slot as a real lease for open + flock + fstat + lstat + unlock. The API calls it on every `/backtest/run.json` status read when no local run is in flight (`api/src/sweeprun.rs:2148` → `observed_status_with_admission` → `cli::execution_lease::probe`). A `Lease::acquire` from the CLI, or from a browser POST in the API, that lands inside that window gets `Busy`. The message names a sweep that does not exist.

**Repro.** The browser polls `/backtest/run.json`. A detail worker is inside `probe` between `lock` (:132) and `release` (:134). At that moment `cli sweep-stored ...` runs `run_durable` → `Lease::acquire` → `flock(LOCK_EX|LOCK_NB)` → EWOULDBLOCK → "refused: required execution admission could not start: another sweep owns this store's execution lease". The exit is FAILED and no sweep was running.

**Minimal fix.** Have `Lease::acquire` retry `WouldBlock` a few times with a short sleep (a real owner holds the lease for minutes; a probe holds it for microseconds). Alternatively, have `probe` take `try_lock_shared`, and have `acquire` treat a short WouldBlock as retryable. Either way, `Busy` should only be returned after the retries.

---

## cli1-4 (low): rung boundaries and the CLI attempt binding are silently lost under `sweep_rungs`' rayon map, because the invocation is held in a thread-local that is only set on the calling thread

`crates/cli/src/operation_audit.rs:438-441, 476-504`
```rust
    pub fn enter<T>(&self, work: impl FnOnce() -> T) -> T {
        let prior = CURRENT.with(|slot| slot.replace(Some(Arc::clone(&self.state))));
...
pub fn completed_boundary() {
    CURRENT.with(|slot| {
        if let Some(state) = slot.borrow().as_ref() {   // None on any other thread: nothing recorded, nothing said
```
`crates/cli/src/lib.rs:15218-15231` (sweep_rungs)
```rust
    let _sharing = SharedBy::these(rungs.len());
    rungs
        .par_iter()
        .map(|&rung| { one_rung(vendor_word, underlying, rung, from, to, support_ppm, attempt) })
```
`one_rung` → `note_rung_finished` (lib.rs:13249-13250) → `operation_audit::completed_boundary();`. Its rung events take `progress.attempt.or_else(binding_attempt)` (lib.rs:13242), and `binding_attempt` (lib.rs:3108-3109) is `operation_audit::current_id().or_else(|| telemetry::global().map(telemetry::Sink::run))`.

**Why it is wrong.** `run_durable` (lib.rs:2115) calls `audit.enter(|| run(args, out))` on the main thread. The API's guards do the same on a detail thread (`api/src/sweeprun.rs:1918`). `range_over` → `sweep_rungs` then calls `par_iter` from that non-pool thread. Rayon injects the job and the caller blocks, so every `one_rung` runs on a pool worker whose `CURRENT` is `None`. The results:
- `completed_boundary()` does nothing for every rung of `range-all` (CLI and browser). The terminal invocation record reads `completed_boundaries: 0` for an eight-rung run that finished, and nothing reports the loss. A descent (lib.rs:11181) runs on the calling thread and does record its boundaries, so the two verbs disagree.
- For CLI `range-all` (`attempt == None`), the rung events carry `attempt` = `Sink::run()`. That is 0 in the CLI, because nothing in `cli` calls `claim_run`. The command's own start and finish events carry the invocation ID (lib.rs:2136-2137). So the rung records are not bound to the command that produced them, which contradicts `binding_attempt`'s doc ("the page's token, the record's `run` and the record's `attempt` are one value chosen once").

**Repro.** Run `cli range-all zerodha NIFTY 2024 1 2024 3 5`. When it finishes, read the run's record with `operation_audit::read(root, id)`: it shows `phase=Completed, completed_boundaries=0`. The eight `cli.audit "rung finished"` lines in `<store>/logs/cli/events.ndjson` carry `"attempt":0` and run 0, while the `cli.lifecycle "command started"` line carries run = the ≥2^63 invocation ID.

**Minimal fix.** Expose a cloneable handle, for example `operation_audit::ambient() -> Option<Arc<Mutex<State>>>` and `operation_audit::within(handle, work)`, which sets and restores `CURRENT`. In `sweep_rungs` (and any other `par_iter` that reaches `note_rung_finished`), capture the handle before `par_iter` and re-enter it inside each closure. `State` is already behind a `Mutex`, so concurrent boundary appends from several workers are safe.

---

## cli1-5 (low): every CLI process installs a sink on the same `<store>/logs/cli` with no cross-process coordination, so two concurrent CLI commands duplicate `seq`/run IDs and rotate each other's file. The doc claims the opposite.

`crates/cli/src/lib.rs:2973-2976` (doc)
> "each writer owning one means each owns its own `events.ndjson`, its own rotation and its own byte budget. No lock, no coordination, nothing to get wrong under concurrency"

`lib.rs:2996`: `.or_else(|| store.map(|s| s.join("logs").join("cli")))`. `main.rs:69`: `let where_events_went = cli::install_log();` runs unconditionally before dispatch and before the execution lease (lib.rs:2101).

**Why it is wrong.** The subdirectory separates the CLI from the API. It does nothing to separate one CLI process from another. Several CLI processes running at once is normal: a long sweep plus `cli top`/`results`/`verify`/`research-plan`/`checksum-audit-stored` (not lease-gated), or even a second sweep, which the lease refuses only after `install_log` has opened the sink. Each process's `Sink::open` resumes `seq` and `reserved_run` from the same last line (telemetry `resume_point`). Each keeps a private byte counter, and `roll` renames `events.ndjson` → `.1` with no inter-process lock (telemetry/src/sink.rs:1266-1300). The ms clamp is per process, so lines interleaved from two processes can step backwards in time inside one file. `tail`'s `since` walk trusts time order and stops at the first older record.

**Repro.** Process A runs `cli range-all ...` (hours), with its last written `seq` = S. Process B runs `cli results`. `resume_point` returns S, so B writes `command started` with `seq` S+1 and run ID `reserve_run_id()` = S+1. A's next event is also `seq` S+1. When A reaches `max_file_bytes` it renames `events.ndjson` (which now holds B's lines too) to `.1` and reopens. B, if still running, keeps appending through its old descriptor into `.1`, behind newer rotated content. Two concurrent non-sweep commands (`cli top` twice) also reserve the same run ID for their lifecycle events.

**Minimal fix.** Give each CLI process its own file in the directory, for example `events-<pid>.ndjson` or a directory per process, with readers merging them. Alternatively, take an exclusive `flock` on a `logs/cli/.writer.lock` in `install_log`, and when it is held, refuse the sink loudly ("events NOT recorded: another cli process owns logs/cli"). At minimum, correct the doc sentence at lib.rs:2973-2976.

---

## Checked and clean (this slice)

- `execution_lease::Lease`: O_NOFOLLOW open, lock taken before `verify` (dev/ino, nlink == 1, len == 0, so a swapped or hardlinked path is refused). It is released by explicit unlock, so a duplicated descriptor in a spawned child cannot pin it (D-0693). The kernel drops the flock on crash, so no stale lock file can block. Canonicalised root, so aliases share one slot.
- `run_durable` ordering: lease, then audit begin, then dispatch, then terminal. On a panic, `Attempt::drop` writes `Failed` while unwinding and the lease guard unlocks. A SIGKILL leaves `Started` (documented as unconfirmed) and the kernel releases the lease.
- `operation_audit` index ID allocation: the exclusive lock is held across length read, tail check and append+fsync. The tail check refuses an index whose last record is not `id - 1`. Per-ID appends hold the per-file lock, check the expected length and fsync. A failure is latched in `State.failure`, so no later success can be acknowledged.
- `batch::sweep_under`: indexed `par_iter().collect()`, sequential `Tally::fold`, per-month `BATCH_CEILING` folded into `Params`. Support-lane count does not affect results. The output text is schedule-independent (token and row order on disk is GAP13-13, known).
- `LEDGER` mutex plus `ResultSetLock` (`results/write.lock`, blocking flock) serialise `record_all_attempt` in-process and cross-process. `record_swept_run` → `ensure_run_record` → `results::with_shared_writer` (in-process mutex, then `refresh`, `holds`, and a flock'd append that re-checks duplicates). An append error refreshes and verifies instead of double-appending.
- `knobs`: read/write `RwLock`; `refused` is a `BTreeMap`, so its rendering is ordered; `describe` sorts. A CLI process never sets knobs. The poison case is known (hunt-conc-6).
- `GridProgress::tick`: Relaxed `fetch_add` only decides whether to emit, and each `done` value is unique.
- `ScreenCache` is scoped to one descent (lib.rs:14245) and keyed by root/vendor/underlying/rung/span. The cached digest is the digest of the bars actually used, so identity matches data even if a pull replaces a month mid-descent.
- `main.rs`: `args_os`, `deliver` uses `write_all` (no `println!` panic), and `preflight_store_root` runs before any write.

---

<!-- conc-pass1/cli2.md -->
### conc-pass1 / cli2: concurrency and state audit of the cli result-store slice at 331b05c

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

---

<!-- conc-pass1/cli3.md -->
Verdict: 3 findings at 331b05c in the cli3 slice (1 medium, 2 low). None of them overlaps the earlier hunt-conc, errpaths or hunt-cli-a/b reports, or the other pass-1 files.

### conc-pass1 / cli3: crates/cli/src (remaining non-test files) plus crates/cli/build_provenance.rs

Method: I read the source only. I ran no cargo. I grepped all 55 slice files plus build_provenance.rs for rename, sync_all, OpenOptions, Mutex, RwLock, spawn, flock, Atomic, create_new, set_len and remove_file. I then read every hit in the writer paths. Before reporting a module I checked that production code can reach it. gaps-1 says `admission_store`, `global_replay` v1/v2/v3, `institutional_statistics`, `stored_data_completeness` and `admission_join` have no production caller, and I re-checked that at this commit: `AdmissionAuthorityLedger::open` is called only from tests, and `global_replay_v3` is named by no other module.

---

## cli3-1 (medium): a VIX companion interrupted after its directory is created can never be published again, and the index-stop catalog stays refused for good

**Where:** crates/cli/src/index_stop_vix.rs:385-411 (`publish`). It is reached from index_stop.rs:415 (`publish_vix`) inside `produce_catalog_inner`, and that runs for every rung of every index-stop search batch (index_stop_search.rs:449 / 459 → `produce_catalog_with_context`).

```rust
let directory = root.join(NAMESPACE).join(crate::identity_hex(&lookup));
match std::fs::symlink_metadata(&directory) {
    Ok(_) => {
        let saved = Reader::open(root, identity, pin, bounds)?;
        ...
        return Ok(saved);
    }
    Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
    Err(why) => return Err(display(why)),
}
catalog.with_current(|| {
    ...
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
    pending.verify_body(digest, body.len() as u64)?;
    pending.finish(lookup, digest, body.len() as u64)?;
```

**Why it is wrong:** The shortcut treats "the directory exists" as "the companion is complete". But `prepare_in_namespace` (boolean_candidate_persistence.rs:94-119) creates that directory first. It then creates `owner.lock` and writes and fsyncs `body.bin`. Only after that does `finish` write `complete.bin` (:74). So a process that dies anywhere between `directory(&base, &directory_path)` and the end of `finish` leaves a directory with no `complete.bin`. On every later call, `publish` takes the `Ok(_)` arm and calls `Reader::open`. That reaches `Observation::open` (boolean_observation_file.rs:47), whose `read_held(&directory.join("complete.bin"), 112)` fails with NotFound. The result is `Err("saved VIX reference companion unavailable or invalid; no current-data substitution: ...")`. Nothing ever re-enters `prepare_in_namespace`, even though that function would complete the publication (or match it byte for byte). The shortcut is the only place that sends the retry down the read path.

**Repro (crash and restart):**
1. `index-stop-search` runs batch N. In one rung, `produce_catalog_inner` publishes the catalog (index_stop.rs:401-405) and calls `publish_vix`.
2. `index_stop_vix::publish` gets NotFound, captures and encodes the image, and enters `prepare_in_namespace`. The directory `index-stop-vix-reference-v1/<lookup>` is created and synced. The process is then SIGKILLed or loses power while writing `body.bin`, or anywhere before `finish` writes `complete.bin`.
3. The operator reruns the identical command. `checkpoint::recover` restores the pending frame for batch N, and the same rung recomputes the same catalog identity and completion. `lookup_identity(identity, pin)` is therefore the same. The catalog's own `prepare_in_namespace` is idempotent and passes. Then `publish_vix` finds the directory, calls `Reader::open`, and refuses on the missing `complete.bin`. `produce_catalog_inner` fails, the catalog attempt gets the Refused terminal, `completed_links` reports "Single-stop batch remains pending", and the search cannot get past batch N. Every later rerun repeats this, because the inputs are deterministic. Nothing in the tree removes the directory, and §3 rule 8 says nothing should.

A second, transient case shows up from the same line. If two processes (the api's index-stop worker and a cli run) reach the same catalog, the second one sees the first one's half-built directory and refuses instead of waiting or matching.

**Not the same as conc-pass1/search.md (`write_or_equal`):** that finding is about a torn `body.bin` or `complete.bin` being refused on retry. Here the retry never reaches `write_or_equal` at all. A crash right after the `mkdir`, with no files yet, wedges this namespace too. Fixing `write_or_equal` alone leaves this wedge in place.

**Minimal fix:** Key the shortcut on the completion receipt, not on the directory. Return `Reader::open` only when `directory/complete.bin` exists. In every other case (no directory, or a directory without `complete.bin`), go to `prepare_in_namespace`/`finish`. They already reuse identical bytes, and the owner `try_lock` in `prepare_in_namespace` keeps a live publisher exclusive. Add a test that removes `complete.bin` (and separately leaves only the empty directory), then calls `publish` and expects success.

---

## cli3-2 (low): the build stamp is checked once, then other source files are compiled later; an edit in that window gives a clean-HEAD stamp on a binary built from different bytes

**Where:** crates/cli/build.rs:79-90 and build_provenance.rs:155-188 (`verify` → `worktree_matches` → `Verification { commit: Some(head), .. }`).

```rust
let verified = build_provenance::verify(Path::new(&manifest), explicit.as_deref());
...
if let Some(commit) = verified.commit {
    println!("cargo:rustc-env=BRUTEX_COMMIT={commit}");
```

**Why it is wrong:** `verify` hashes every relevant working-tree file at the moment `cli`'s build script runs (build_provenance.rs:758-766, `fs::read(full)... object_oid("blob", &bytes) == expected.oid`). rustc reads those same files later:
- `crates/cli/src/**` is compiled after the build script finishes.
- `engine`, `runner`, `vocab`, `indicators`, `store`, `pull`, `costs` and `core` are normal dependencies of `cli`. Cargo is free to compile them at the same time as `cli`'s build script or after it, because the script waits only on its build-dependencies.

Nothing re-checks after compilation. `rerun-if-changed` makes the NEXT build re-verify, but the binary already built keeps `BRUTEX_COMMIT=<HEAD>`. That stamp feeds the `commit` term of the run identity (§3 rule 3). So runs get recorded under a commit whose source did not produce them, which is the outcome build.rs:51-55 says it exists to prevent ("would stamp a result with a commit whose source never produced it").

**Repro:** On a clean HEAD, start `cargo build --release -p cli` (or press Run in the IDE, which docs/07-plan.md §0 names as the operator path). After the `cli` build-script step has run, while `runner`/`engine` or `cli`'s own lib are still compiling, save an edit to `crates/runner/src/rank.rs`. An IDE with autosave does this without being asked. rustc reads the edited file. The finished binary prints a verified commit, and `sweep-stored` records ledger rows with commit = HEAD for code that is not HEAD. The next `cargo build` would rebuild and unstamp, but the bad binary has already run.

**Minimal fix:** A build script cannot see reads that happen after it exits, so the fix belongs at a point after compilation:
- Emit a digest of the verified blob set as a second `rustc-env` (for example `BRUTEX_TREE`).
- Have CI and the launcher re-run `build_provenance::verify` after the binary is linked, and refuse to treat the stamp as authoritative if the two disagree.

At minimum, record the limit in docs/06-limits.md and in the build.rs header. The stamp proves the tree as it was when the build script ran, not as rustc read it. §3 rule 6 requires that limit to be stated.

---

## cli3-3 (low, latent: no production caller today): a kill or power loss mid-`write_all` leaves a ragged tail, and every later open of these ledgers refuses it with no repair path; one comment says this cannot happen

**Where:**
- crates/cli/src/admission_store.rs:1019-1061 (`write_decisions`), with refusal at :1638-1643 (`check_header`).
- The same append shape with the same refuse-on-ragged rule at institutional_statistics.rs:1404-1435 (`append_sync_with`), with refusal at :1330.
- stored_data_completeness.rs:870-893 (`commit`).

```rust
/// Writes `decisions` at `at` as ONE buffer, so a kill can no longer stop
/// between records of one call, and rolls a failed write back to `at`.
...
if let Err(why) = self.decision_file.write_all(&buffer) {
    return Err(rollback_message(&self.decision_file, at, ...));
```
```rust
if !len.saturating_sub(HEADER).is_multiple_of(stride) {
    return Err(format!("{} has length {len}, not a {HEADER}-byte header plus whole {stride}-byte records; a torn/ragged tail is never ignored", ...
```

**Why it is wrong:** The rollback (`set_len(at)`) runs only when `write_all` returns an error. A process killed inside the write never reaches it. The doc's premise, that one buffer means "a kill can no longer stop between records", is false on Linux for a multi-page buffer. `generic_perform_write` copies page by page and stops with the bytes already copied when a fatal signal is pending. The file is then left extended to a page boundary that is generally not a multiple of the record stride. Power loss gives the same result through partially persisted extents. The next `open`/`open_read` hits `check_header`'s ragged-tail refusal, and nothing in the module can truncate or complete it. The D-1630 orphan recovery handles only a whole-record prefix. So one interrupted decision block wedges the whole admission ledger for every population, not just the one being written.

**Repro:** `AdmissionAuthorityLedger::open(root)` → `commit(pop, digest, policy, decisions)` with enough decisions that the buffer spans several pages. SIGKILL the process while it is inside `write_all` (:1047). On restart, `AdmissionAuthorityLedger::open(root)` returns `Err("... not a 16-byte header plus whole N-byte records; a torn/ragged tail is never ignored")`, and does so on every later open.

**Severity:** Low because `AdmissionAuthorityLedger::open`, `InstitutionalStatisticsLedgerV1::open` and the stored-data-completeness writer have no production caller at this commit (gaps-1; only tests open them). It becomes medium the day a verb wires any of them in.

**Minimal fix:** On writable open, treat a ragged tail past the last whole record as the trace of an interrupted append. If the whole-record prefix validates, truncate to it (`set_len`), `sync_all`, and record the event loudly (a telemetry event and the refusal text), consistent with the D-1630 orphan rule. Keep read-only opens refusing. Correct the `write_decisions` doc comment either way.

---

## Checked and clean (no finding)

- **search_checkpoint.rs:** Journal and Snapshot locks are released by explicit unlock. Payload, marker and directory fsyncs are in the right order. The torn `complete` marker and the missing `DIRECTORY_LIMIT` check are already KNOWN (hunt-cli-b re-1/re-2), so I did not repeat them.
- **expression.rs:** `EvidenceWriter` uses a create_new pending file under an exclusive flock, verifies through the locked handle, hard-links to publish (it never replaces), removes the pending file, then fsyncs the directory. Each token gets its own attempt directory, and the leaf `create_dir` refuses an existing token.
- **live.rs:** The temp file is `<hex>.<pid>.tmp`, then `sync_data`, then `rename`. The census ignores strays and sorts by identity, so output is deterministic. The write-once staleness behaviour and the strays left by killed writers are documented in the module.
- **checksum_receipts.rs:** `publish` takes an exclusive try_lock, checks for an exact-prefix completion, writes the payload, fsyncs, writes the seal, fsyncs, then fsyncs the parent directory. Concurrent publishers refuse loudly with WouldBlock. `Receipt` keeps a shared lease and rechecks the generation.
- **global_replay_v4_store.rs:** The exclusive lock covers prefix compare plus append. The seal is written last behind its own fsync, then the directory is fsynced. Readers take a blocking shared lock and check exact length and digest.
- **global_replay_v4_lifecycle.rs:** Every begun attempt gets a terminal (Refused on error, overlapping stages refused).
- **and_checkpoint.rs:** `recover` correctly handles an orphan chunk whose boundary was never acknowledged, and the final boundary is re-verified against its acknowledged seal.
- **index_stop_search.rs:** Results are reassembled by rung index, and a failure keeps the batch pending.
- **index_stop.rs, index_consistency_store.rs, index_stop_qualification.rs:** All call `prepare_in_namespace` unconditionally, with no existence shortcut. Their remaining torn-file exposure is the `write_or_equal` finding in search.md.
- **ledger_v6.rs, ledger_all.rs, research_policy.rs, research.rs, fold_audit.rs, stability.rs, strict_range_*, audited_*, *_codec.rs, main.rs, readonly_file.rs:** No shared mutable state, no lock and no durable write outside test code, except `create_dir_all` of output roots.
- **HashMap/HashSet in the slice:** These are used only for membership or duplicate checks on reachable paths. The admission_store refusal-order nondeterminism is already KNOWN as hunt-conc-3.

---

<!-- conc-pass1/pop1.md -->
### pop1: concurrency and state audit of the population ledgers (pass 1)

Verdict: 4 findings (0 high, 2 medium, 2 low). All are crash or short-write recovery defects. The flock discipline in this slice is sound: every writer re-validates under an exclusive lock, and readers re-check generations.

Slice: crates/cli/src/population.rs, population_v5.rs, population_v6.rs, all_rung_population_v5.rs, population_observations_v1.rs, population_base_evidence_ledger_v2.rs, at commit 331b05c. Audited from source only; cargo was not run.

---

## pop1-1 (medium): a V1 population interrupted between row chunks can never be committed

**Location.** crates/cli/src/population.rs:3367-3404 (`append_rows`) and :3318-3347 (`require_exact_existing`).

```rust
for chunk in rows.chunks(ROW_WRITE_CHUNK_ROWS) {      // 32 rows per write
    ...
    self.row_file
        .write_all(encoded)
        .map_err(|why| rollback_message(&self.row_file, at, "population rows", &why))?;
}
self.row_file.sync_all()...
```

```rust
let mut expected_at_actual_location = expected;
expected_at_actual_location.block.first = actual.block.first;
if actual != expected_at_actual_location {
    return Err(format!(
        "population {} already has different row facts; no byte was replaced", ...
```

**Why it is wrong.**
- A block is written as many separate `write_all` calls, one per 32 rows.
- `rollback_message` only runs when a write returns an error. It does not run when the process dies between two chunks (Ctrl-C/SIGINT, SIGKILL, OOM-kill).
- After such a death, the row file ends in a whole-row prefix: sequences 0..k-1 of population P, with k < n. That prefix is not ragged, and `index_rows_range` accepts it as a valid block of `count = k`.
- On the exact rerun, `append_complete_locked` takes the `raw_blocks.contains_key` branch (:3153). `require_exact_existing` then compares facts with `count = k` against `count = n` and refuses.
- No code path completes a trailing prefix. Rows are append-only, and a second block for P would be refused as "interleaves"/"non-contiguous duplicate". So P is permanently uncommittable.
- The module doc (:29-31) covers only "a crash after the row sync". The newer ledgers in the same slice do handle this case:
  - V5: `complete_trailing`, population_v5.rs:2220
  - V6: the trailing prefix check, population_v6.rs:2116-2134
  - Base: `compare_prepared_prefix`

**Repro.**
1. Run a population with n > 32 rows.
2. SIGINT the process after the first `write_all(encoded)` at :3401 returns and before the second.
3. The row file now holds rows 0..31 of P.
4. Rerun the identical command. `PopulationLedger::open` succeeds and `append_complete_v4` returns "population <P> already has different row facts; no byte was replaced". This happens on every retry, forever.

**Minimal fix.** In the `raw_blocks.contains_key` branch, if the existing block is the last block in the file, has no receipt, and is a row-exact prefix of `rows` (byte compare via `require_exact_block` on its `count`), then:
- append `rows[count..]`;
- `sync_all`;
- update `raw_blocks[P]` to the full facts;
- continue to the receipt.

This is the same rule V5 and V6 already implement. Alternatively, encode the whole block into one buffer and issue a single `write_all`. That narrows the window but does not close it for short writes.

---

## pop1-2 (medium): a short write leaves a ragged tail that makes the whole V5/V6/Observation ledger unopenable

**Location.**
- population_v5.rs:3149-3153 (`append_raw`), used by `append_row_suffix` (:2263) and `append_completion` (:2279)
- population_v6.rs:2444-2448 (`append_raw`), used by `append_locked` (:2168, :2173)
- population_observations_v1.rs:2278-2282 (V1 Data), :2301-2305 (V1 Completion), :3432-3436 and :3442-3446 (V2)

```rust
fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationV5Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Population V5 fixed record: {why}"))
}
```

```rust
self.file.seek(SeekFrom::End(0))
    .and_then(|_| self.file.write_all(&record))
    .and_then(|()| self.file.sync_data())
    .map_err(|why| format!("cannot sync observation authority Data: {why}"))?;
```

**Why it is wrong.**
- `write_all` can fail after a partial write. A typical case is ENOSPC or EDQUOT partway through a record: the first `write` returns a short count, and the next returns an error. Some bytes of the record then stay at the tail.
- None of these sites truncates back to the pre-append length.
- Every open of these ledgers refuses a non-stride length, so one failed append makes the ledger unopenable for every reader and writer, including all populations that were already committed:
  - V5: `checked_record_count`, population_v5.rs:3067
  - V6: "Population V6 data file is ragged", :2019
  - Observation V1: :2552
  - Observation V2: :3577
- No repair path exists. The same slice already treats this as a defect elsewhere:
  - population.rs rolls back with `set_len(at)` (`rollback_message`, :5146).
  - population_base_evidence_ledger_v2.rs rolls back with `set_len(original)` at :1497 and :1524.

**Repro.**
1. Fill the filesystem so only `RECORD_BYTES - 10` bytes are free.
2. Run a V6 append. `append_raw` writes a partial record, then gets ENOSPC, and returns `Err`.
3. Free space and rerun. `PopulationV6Ledger::open` refuses with "Population V6 data file is ragged". Every Selection/ledger_v6 reader of this root refuses too.

**Minimal fix.**
- Record `len` before each append.
- On any write error, `set_len(len)` and `sync_all`.
- Report a separate "poisoned" error if the truncation itself fails.

This mirrors `append_prepared_records` in population_base_evidence_ledger_v2.rs:1463-1503. Apply it to V5 `append_raw`, V6 `append_raw`, and the four Observation V1/V2 write sites.

---

## pop1-3 (low): the Observation authority ledgers never fsync their directory after creating files

**Location.** population_observations_v1.rs:2141-2182 (V1 `open_inner`) and :3332-3363 (V2 `open_inner`).

```rust
let mut file = OpenOptions::new().read(true).write(writable).create(writable).open(&file_path)?;
if writable && file.metadata()?.len() == 0 {
    file.write_all(&authority_header()).and_then(|()| file.sync_data())...
}
```

After this, `append_data` syncs only the file (`sync_data`). Nothing in the module calls `sync_all` on the directory: grep finds no directory sync in the file.

**Why it is wrong.**
- POSIX durability of a newly created name requires an fsync of the parent directory.
- Every sibling ledger in this slice does that after creating its files:
  - population.rs `sync_directory(&dir)` (:2731)
  - V5 `sync_directory` (:2025, :2289)
  - V6 `root_file.sync_all()` (:1966, :1976, :2182)
  - Base `sync_directory` (:1190, :1250)
- `append_and_reopen` returns `Written` once the Completion is `sync_data`-ed, and callers treat the authority as durable. A power loss before the directory entry is written back can drop the whole `AUTHORITY_FILE` (and the lock file) even though its records were synced.

**Repro.** On a filesystem without ordered-metadata guarantees (or ext4 with data=writeback), on a fresh root:
1. Call `append_and_reopen`; it returns `Written`.
2. Cut power before the periodic metadata writeback.
3. After reboot, the authority file name is absent, and `open_read` refuses the "absent file" that the caller was told was committed.

**Minimal fix.** In both `open_inner` functions, when the lock or authority file was created, or the header was written, call `File::open(admitted_root)?.sync_all()` before scanning. This is the same pattern as V5 `sync_directory`.

---

## pop1-4 (low): a V1 retry after a failed row fsync trusts the second fsync and can commit a receipt over rows that never reached disk

**Location.** population.rs:3405-3407 (`append_rows`: on `sync_all` failure it returns `Err` with the bytes left in place) and :3153-3157 (the retry path).

```rust
self.row_file
    .sync_all()
    .map_err(|why| format!("the new population rows could not be synced: {why}"))?;
```

```rust
if self.raw_blocks.contains_key(&receipt.population_id) {
    self.require_exact_existing(rows, receipt, expected)?;
    self.row_file.sync_all().map_err(...)?;
}
```

**Why it is wrong.**
- On Linux, a writeback error is reported to one `fsync` per open file description (errseq). The pages can be marked clean while the data is not on disk.
- The retry absorbs the unsynced tail as an orphan block. `require_exact_block` then reads it back from the page cache, so it matches.
- The second `sync_all` on the same handle returns `Ok`, and the V2/V3/V4 receipts are then written and synced.
- The commit marker now vouches for row bytes that may not exist on the device.
- After a crash, the open-time reconciliation finds receipt facts with no matching row block and refuses the entire ledger. So the failure is loud, but it lands on the whole store rather than on the one retry.

**Repro.**
1. Inject an EIO on the first fsync of population-v1.bin (dm-flakey or fault injection).
2. `append_complete_v4` returns "the new population rows could not be synced".
3. The caller retries on the same handle (or a fresh handle in the same boot). The retry returns `Written`.
4. Power-cycle. `PopulationLedger::open_read` refuses with "has a receipt but no row block", or with a seal or digest mismatch.

**Minimal fix.** On a row `sync_all` failure:
- `set_len(at)` to roll the unsynced block back;
- mark the handle unusable, for example `self.writable = false`, with an error telling the operator to reopen.

The retry must then rewrite the rows rather than reuse page-cache bytes. The same reasoning applies to V5 and V6 trailing-prefix reuse after a `sync_data`/`sync_all` failure; this report lists V1 only, because only there is the retry path on the same live handle.

---

## Checked and found sound (not findings)

- **population.rs.** The writer re-absorbs all four files under the exclusive `population-write.lock` before every decision. A reader (`open_read`) holds the shared lock only during open. `page_v4` re-checks every generation under the shared writer lock. Legacy `page()` flocks the row file, which the writer never locks, but it reads only committed, immutable ranges.
- **Lock clones.** `try_clone` on `writer_lock` shares one open file description, so unlock through either handle is consistent. Separate handles in one process use separate descriptions, so they exclude each other correctly.
- **Crash between rows, V2, V3 and V4 receipts (population.rs).** The exact rerun reuses each lower receipt and appends the missing ones. Verified in `append_complete_v4_locked`.
- **V5/V6.** The lock is taken before any child is created or initialised (except the lock file itself). A trailing whole-record prefix is completed on exact retry. A foreign trailing prefix is refused loudly, by design. The directory is synced after creation and after the completion.
- **Base Evidence ledger V2.** It rolls back on write error, completes an orphan prefix on exact retry, writes completion last, and syncs the directory.
- **Observation V1/V2.** The flock is held for the whole ledger lifetime through `Flock`, and the writer is dropped before the read-only reopen in both `append_and_reopen` paths, so there is no self-deadlock.
- **all_rung_population_v5.rs.** Directory admission and identity checks only. No writes, locks or threads in production code.
- **Nondeterminism.** No HashMap or HashSet iteration feeds output bytes or digests in this slice. `FactsBuilder::finish` iterates `masks` only to compute order-independent sums, minimums and maximums. The `reconcile_receipts*` refusal order was already reported as hunt-conc-3 and is not repeated here.
- **Threads and atomics.** None in production code. The atomics are test-only temp-name counters.

---

<!-- conc-pass1/pop2.md -->
Verdict: 7 findings (0 high, 3 medium, 4 low) in the Population Admission/Finalization/Statistics V2-V4 ledgers at 331b05c. All of them are about crash recovery and durability. No data race, lock-order cycle or HashMap-order leak was found.

### pop2: concurrency and state audit, pass 1

Slice: crates/cli/src/population_admission_v2.rs, _v3.rs, _v4.rs; population_finalization_v2.rs, _v3.rs, _v4.rs; population_statistics_v2.rs, _v3.rs.

**Liveness.** I traced callers before rating anything:
- **Admission V3, Finalization V3 and Statistics V2:** live through `cli ledger-all` (`all_rung_population_v5.rs:651-676`).
- **Statistics V3, Admission V4 and Finalization V4:** live through `cli ledger-v6` (`step3_orchestrator.rs:2295`, `:2336`, `:2358`).
- **Admission V2 and Finalization V2:** the write half is `dead_code` outside tests, so they get no finding of their own.

Every per-rung root is shared by every run against the same ROOT argument (`ledger_v6.rs:200-205`, `RungRoots::create`).

## Findings

### pop2-1 (medium): Admission V3 cannot recover a crash in the middle of its decision block, and the rung ledger then refuses every later append
- **Where:** `population_admission_v3.rs:3987-3989` (writes decisions one record at a time), `:4071-4077` (retry check).
- **Code:**
  ```rust
  for decision in &prepared.decisions {
      append_raw(&mut self.decision_file, &encode_decision(decision)?)?;
  }
  ...
  if trailing.block_id != prepared.source.block_id || trailing.decisions != prepared.decisions
  { return Err(format!("Admission V3 trailing block {} is not exact retry {}", ...)); }
  ```
- **Why it is wrong:** the writer appends each 2,048-byte decision with its own `write_all` and syncs once at the end. A crash after decision i of n (0 < i < n) leaves a whole-record proper prefix. `scan` accepts that prefix as `trailing` (`validate_trailing_decisions` checks prefix order only, `:3699-3716`). On the next run, `append_locked` routes to `complete_trailing` because `trailing.is_some()` (`:3982-3984`). That function demands the full decision vector, so even the byte-exact retry refuses, with both hex ids equal. Every other block that run, or any later run on that rung root, tries to append also enters `complete_trailing` and refuses. The ledger is wedged for good, and no tool repairs it.
- **Inconsistent with its siblings:** Finalization V3 handles the identical shape with `prepared.rows.starts_with(&trailing.rows)` (`population_finalization_v3.rs:2418`), with a comment saying exactly this crash can happen. D-1630 (commit 66f140c, W2-cli1-4) fixed the same class in `admission_store` and left V3 alone. The test `partial_orphan_corruption_reserve_and_lock_bytes_refuse` (`:6365-6386`) pins the wedge as expected behaviour.
- **Repro:** run `cli ledger-all … ROOT` and SIGKILL it while rung `1min` Admission V3 is between `append_raw` calls (for example, after the first decision). Rerun the identical command. It fails with `all-rung 1min Admission V3 refused: Step 3 Population Admission V3 commit refused: Admission V3 trailing block X is not exact retry X`. It fails the same way on every later rerun, and for every other month range on that ROOT.
- **Fix:** accept `prepared.decisions.starts_with(&trailing.decisions)`. Append the missing suffix, `sync_data`, then write the Completion and sync the directory, as Finalization V3's `complete_trailing` does. Better still, also write the block in one `write_all`, as D-1630 did. Then change the test to assert that the exact retry recovers.

### pop2-2 (medium): Finalization V3/V4 and Statistics V2/V3 never got the failed-append rollback that Admission V3/V4 received
- **Where:**
  - `population_finalization_v3.rs:2857-2861`
  - `population_finalization_v4.rs:2718-2725`
  - `population_statistics_v3.rs:2903-2910`
  - `population_statistics_v2.rs:5479-5486`
- **Code (identical shape at all four sites):**
  ```rust
  file.seek(SeekFrom::End(0))
      .and_then(|_| file.write_all(raw))
      .map_err(|why| format!("cannot append Finalization V4 record: {why}"))
  ```
- **Why it is wrong:** Admission V3/V4 wrap the same call in `append_with_rollback` (D-0916, W2-cli10-3, invariant C4-CLI-05-01). Their own doc comment states the defect: "Without that a partial `write_all` left a ragged tail, and every later open, read-only included, refused the file's already committed authorities as ragged." The four writers above still have that defect. Their scans refuse any non-stride length:
  - Finalization V4: `"Finalization V4 data file is ragged"`, `:2230`
  - Finalization V3: `checked_record_count`, `:2777`
  - Statistics V3: `record_count`, `:2868`
  - Statistics V2: `record_count`, `:5441`
- **Repro:** the volume holding ROOT fills up during `commit_population_finalization_v4`. `write_all` writes 1,000 of the record's 4,096 bytes, and the next `write` returns ENOSPC. The file is now `64 + 4096·n + 1000` bytes. After space is freed, every `open_read` and `open_write` on that root fails as "ragged", so every committed Finalization V4 block on that rung, from every earlier month, becomes unreadable. Nothing in the tree repairs it.
- **Fix:** move `append_with_rollback` (`set_len(end)` on error) into a shared helper and call it from all four `append_raw` functions. Add the C4-CLI-05-01 test to each module.

### pop2-3 (medium): a crash in the middle of a record leaves a sub-record tail that permanently fails every open, read-only included
- **Where:** `population_admission_v4.rs:2618-2625` (`scan`), and the equivalent ragged checks in Finalization V4 (`:2230`), Admission V3 (`checked_record_count`, `:4428`), Finalization V3 (`:2777`), Statistics V3 (`:2868`) and Statistics V2 (`:5441`).
- **Code:**
  ```rust
  if len < HEADER_BYTES as u64 || !(len - HEADER_BYTES as u64).is_multiple_of(RECORD_BYTES as u64)
  { return Err("Admission V4 data file is ragged".to_owned()); }
  ```
- **Why it is wrong:** the rollback only covers a write that *returns* an error. A process killed inside `write_all` leaves the same ragged tail.
  - Admission V4 and Finalization V4 records are 4,096 bytes at offset `64 + 4096·k`, so every record straddles a page boundary.
  - In Statistics V2/V3, every fourth 1,024-byte record straddles one.
  - Linux `generic_perform_write` checks `fatal_signal_pending` between page chunks and returns a short count, so SIGKILL or the OOM killer can stop a write after the first 4,032 bytes.
  - Power loss can also persist a partial extension.

  The bytes past the last whole record were never covered by a synced Completion, so they are provably unacknowledged. Even so, the writer, which holds the exclusive lock, refuses them exactly as a reader does. For the torn *header*, the writer repairs the identical situation (`holds_torn_header`, `:1704`). For the torn *tail*, nothing does.
- **Repro:** SIGKILL `cli ledger-v6` during the Completion `append_raw` in `PopulationAdmissionV4Ledger::append_locked` (`:2789`). The file ends 4,032 bytes into a record. The rerun fails at `open_write` with `"Admission V4 data file is ragged"`. So does every later run on that rung root, and so does `open_read` of blocks committed months earlier.
- **Fix:** in `open(writable = true)`, under the exclusive lock, truncate `len` down to `HEADER + k·RECORD` when the remainder is shorter than one record. Then `sync_all` and resume through the existing trailing-prefix logic. Readers keep refusing. A whole-record tail with a bad seal must still refuse.

### pop2-4 (low): a receipt-less trailing prefix accepts only an exact retry whose identity includes the build commit, so a rebuild after a crash wedges the rung ledger
- **Where:** `population_admission_v4.rs:2727-2731` (`trailing.source != prepared.source`), with the same rule in Finalization V4 (`:2361`), Statistics V3 (`:2394-2395`), Statistics V2 (`:4611-4616`) and Admission V3 (`:4071`).
- **Code:** `if trailing.source != prepared.source || … { return Err("Admission V4 trailing prefix is not the exact retry".to_owned()); }`
- **Why it is wrong:** `BlockSourceV4.source_commit_digest` is `hash(verified_commit)` (`step3_orchestrator.rs:3593`, and `population_admission_v4.rs:746`). After a crash leaves a trailing prefix, the only block that can ever be appended to that root again is one prepared by the same binary commit from the same inputs. The usual response to a crash is to rebuild with a fix, and after that the exact retry can no longer be constructed. The per-rung root is shared by every month range, so ledger-v6 on that rung is wedged with no recovery tool (a grep for quarantine, discard-orphan or truncate-orphan finds nothing for these ledgers). Other ledgers in the repo instead tolerate and skip a valid foreign orphan: Execution V1 "retains valid orphan evidence" (`docs/04-invariants.md` EC-01).
- **Repro:**
  1. SIGKILL ledger-v6 after the Admission V4 evidence `sync_all` (`:2788`) and before the Completion.
  2. Rebuild at a new commit.
  3. Rerun. It fails with `Admission V4 trailing prefix is not the exact retry`, and every later run on that rung fails the same way.
- **Fix:** let the writer skip a fully valid receipt-less foreign prefix, as Execution V1 does: the next block starts after it, and the prefix stays as unreferenced evidence. Alternatively, ship an operator verb that records and quarantines it. At minimum, put the orphan's block id and source commit in the refusal text.

### pop2-5 (low): Statistics V2/V3 never fsync the root directory after creating their files or after committing
- **Where:** `population_statistics_v3.rs:2142` and `:2158` (`open_file(..., create)`), and `append_locked` `:2478-2487`; the same in `population_statistics_v2.rs:2187`, `:2211` and `:4582-4645`.
- **Code:** the data and lock files are created with `.create(create)`. Only `self.data_file.sync_all()` follows, and there is no `root_file.sync_all()` anywhere in either module.
- **Why it is wrong:** every sibling in the slice syncs the directory after creating a file and after a commit (Admission V4 `:2583-2586` and `:2797`; Finalization V3 and Admission V3 `sync_directory`). `fsync` on the file does not make a new directory entry durable under POSIX.
- **Repro:**
  1. ledger-v6 commits Statistics V3 into a fresh `statistics/1min`.
  2. Admission V4 then appends a block that binds that Statistics authority and syncs its own directory.
  3. Power is lost.
  4. After reboot, the Admission V4 block is present, but `statistics/1min/<DATA_FILE>` may be absent. Any audit that re-verifies Admission against its Statistics source now fails to open it.

  A rerun recreates identical bytes, which is why this is rated low.
- **Fix:** hold the root `File` and `sync_all` it after a create, and again after the Completion sync, as the V4 ledgers do.

### pop2-6 (low): Statistics V2/V3 cannot recover a torn header; Admission V4 and Finalization V4 can
- **Where:** `population_statistics_v3.rs:2822-2834` and `population_statistics_v2.rs:5395-5408`.
- **Code:** `if len == 0 { write header; sync_all; return Ok(()) } verify_header(file, path)`. `verify_header` then refuses `len < 64` with "shorter than Statistics V3 header".
- **Why it is wrong:** a crash or power loss during the first `write_all(&header)` and `sync_all` can leave between 1 and 63 bytes. From then on, every open refuses, the writer included. Admission V4 and Finalization V4 added `holds_torn_header` (`:1704`) for exactly this residue.
- **Repro:** power loss while the first Statistics V3 append on a new rung is initializing the file leaves a file of, say, 32 bytes. Every ledger-v6 run on that rung then fails with `… is 32 bytes, shorter than Statistics V3 header`.
- **Fix:** port `holds_torn_header`. When the writer, under the exclusive lock, finds a strict prefix of the constant header, it rewrites the header.

### pop2-7 (low): an unrelated append by a concurrent run invalidates retained authorities, and nothing serializes runs on one ROOT
- **Where:** `population_admission_v4.rs:2893-2908` (`require_unchanged`, which compares a blake3 hash of the whole file), reached from `finalization_projection` → `read_complete` (`:2821`). The commit that sets it up is `step3_orchestrator.rs:2336-2363`.
- **Code:** `file_generation(&self.data_file, &self.data_path, …)? != self.data_generation` → `"Admission V4 retained file/root generation changed"`.
- **Why it is wrong:** the ledger is append-only, and its flock is held only during open and append. A second process that appends a different, legitimate block to the same per-rung file changes the whole-file digest. That invalidates the first process's retained authority between the Admission commit and the Finalization commit. The ledger-v6 and ledger-all verbs take no run-level lock on ROOT (`lib.rs:1899-1910`, `ledger_v6.rs`). The first run therefore fails partway through the pipeline, after several ledgers have already durably committed its blocks. Finalization V4 and Population V6 reauthenticate the retained Admission source again later, so the failure window covers most of the route.
- **Repro:**
  1. Process A runs `cli ledger-v6 V 2024 1 2024 3 … ROOT` and process B runs `cli ledger-v6 V 2024 4 2024 6 … ROOT`.
  2. A returns from `commit_population_admission_v4` for `1min`.
  3. B appends its own `1min` Admission V4 block.
  4. A calls `commit_population_finalization_v4` and fails with `Finalization V4 Admission source refused: Admission V4 retained file/root generation changed`.

  A's rerun recovers through exact reuse, so the harm is a spurious refusal and wasted work.
- **Fix:** take an exclusive per-ROOT run lock in `ledger_v6`/`ledger_all` and refuse a second run up front. Alternatively, have the retained authority check only its own block's byte range plus the inode, not the whole file's digest.

## Checked and clean
- **Lock pairing:** every `lock()`/`lock_shared()` in the Admission V2/V3/V4 and Finalization V3/V4 ledgers is followed by `unlock()` on both the Ok and Err paths (the closure-then-combine pattern). Statistics V2/V3 use a `store::flock::Flock` guard whose `Drop` unlocks.
- **Lock order:** no ledger holds its flock while taking another ledger's flock. `commit_population_finalization_v4` prepares (taking the Admission shared lock and releasing it) before it opens the Finalization writer, and Admission V4 commit does the same with Statistics. No ordering cycle exists.
- **Append order:** evidence is synced before the Completion is appended, and the Completion is synced before the directory, in all V3/V4 writers. The reuse path re-issues both barriers.
- **Data-file creation:** in V4 it happens under the exclusive lock. Lock-file creation races (`exists()` then `create`) are harmless: both sides only sync the directory.
- **Determinism:** no HashMap iteration reaches output, digests or refusal text in this slice. `receipts` and `audits` are only `get`/`insert`/`contains_key`. There are no threads, rayon, atomics or wall-clock values outside tests.
- **No overlap with earlier reports:** none of the above duplicates hunt-conc-1 to hunt-conc-8 or errpaths. errpaths' `exists()` note for Admission V4 and Finalization V4 is correct and not repeated here.

---

<!-- conc-pass1/sel.md -->
### conc-pass1 / sel — Selection, Execution and Step-3 slice at 331b05c

**Verdict: 1 finding (1 medium). The live append paths are serialized correctly under flock, and their crash-prefix recovery works. The one live gap is that Execution V4 (and the dead Execution V3) still use `seek(End)+write_all` with no rollback. That is the D-1622 class, which was fixed only for Selection V5 and institutional statistics.**

Slice: `crates/cli/src/selection.rs`, `selection_v3.rs`, `selection_v4.rs`, `selection_v4_authority.rs`, `selection_v5.rs`, `selection_v6.rs`, `selection_v6_source.rs`, `execution_capability.rs`, `execution_disposition_v2.rs`, `execution_v3.rs`, `execution_v4.rs`, `step3_comparison.rs`, `step3_orchestrator.rs` (production code only; `execution_lease.rs` excluded). I worked from source only. No cargo was run.

Reachability, which sets severity: production reaches only `execution_v4` → `selection_v6`, via `ledger-v6` → `step3_orchestrator.rs:2378` and `ledger_v6.rs:587`. Selection V1–V4, `selection_v4_authority`, `execution_capability`, `execution_disposition_v2` and `step3_comparison` have no production caller (gaps-1). `execution_v3`, `selection_v5`, `all_rung_*_v5` are `expect(dead_code)` (lib.rs:110-127).

## Findings

### sel-1 — medium — Execution V4 append has no rollback: one short write (ENOSPC/EIO) permanently wedges the rung's execution root

- **Where:** `crates/cli/src/execution_v4.rs:4931-4935`. Callers are at :2905, :2918, :2931 and :2949 (`append_*_suffix`, `append_completion`). The same code is at `execution_v3.rs:4047-4051`, but that file is dead code in production.
- **Code:**
  ```rust
  fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), ExecutionV4Refusal> {
      file.seek(SeekFrom::End(0))
          .and_then(|_| file.write_all(raw))
          .map_err(|why| format!("cannot append Execution V4 fixed record: {why}"))
  }
  ```
- **Why it is wrong:** Records are 1,280 bytes for parameters and Completion, 128 for percentiles, and 1,024 for dispositions (execution_v4.rs:72-78). Suppose `write(2)` accepts part of a record because the filesystem fills mid-record, and `write_all` then gets `ENOSPC`. The function returns `Err`, but it leaves a file whose length is not a multiple of the stride. Every later `ExecutionV4Ledger::open` runs `scan` → `HeldFile::record_count` → `checked_record_count`, which refuses: `"Execution V4 {name} file has ragged length {bytes}, not a multiple of {stride}"` (execution_v4.rs:4893-4896). This happens before `append_locked` can consider an exact-retry orphan, so freeing disk space and re-running the identical command cannot recover. Every committed execution in that root also becomes unreadable through `open_read`. D-1622 (docs/05-decisions.md:54083) names this exact defect: *"A short write (ENOSPC, EIO) left a ragged or torn tail that every later open refused ... so the ledger was wedged for good"*. It fixed `selection_v5::append_raw` (now `append_with_rollback`, selection_v5.rs:3028-3055) and `institutional_statistics`, but not the live Execution V4 writer that feeds Selection V6. Whole-record crash prefixes are handled (`TrailingExecutionV4` + `require_exact_prefix`); partial records are not.
- **Repro (exact):** run `ledger-v6` for one rung on a volume with fewer than 1,280 bytes free when `append_parameter_suffix` (execution_v4.rs:2727) runs. Its first `write` stores, for example, 700 bytes of a parameter record, and the next returns `ENOSPC`. The command fails with "cannot append Execution V4 fixed record: No space left on device". Free space, then re-run the same command. `commit_stored_execution_v4` → `persist_prepared` → `ExecutionV4Ledger::open_write` → `scan` → refuses "parameter file has ragged length N·1280+700". Every later `ledger-v6` / `ledger-v6-replay` for that rung refuses at Step 3 Execution V4, and nothing in the code can repair it. The same happens if the short write lands in the percentile, disposition or Completion file.
- **Minimal fix:** route `append_raw` through the same `append_with_rollback` that selection_v5.rs:3036-3055 uses. Record `end = seek(End(0))`, and on a write error call `file.set_len(end)`, naming both errors if the truncation fails. Do the same in `execution_v3.rs:4047`. Add a test in the style of `selection_v5::tests::a_failed_append_truncates_back_and_the_ledger_stays_open` with a write closure that writes half a record and returns `Err`.

## Checked and clean (or already reported)

| area | file:line | concern | verdict |
|---|---|---|---|
| Execution V4 lock discipline | execution_v4.rs:2458-2554, 2685-2705, 3010-3160 | read-modify-write outside the lock | The exclusive flock is held across `require_unchanged` → `scan` / suffix writes → Completion → `scan`. Lookups take a shared flock and recheck the full generation (len, inode, mtime, content digest). `try_clone` shares the OFD, so the unlock through the original fd and the retained clone agree. |
| Execution V4 crash prefixes | execution_v4.rs:2726-2766, 2819-2852 | crash between data and Completion | Data files are `sync_data`'d before Completion. Completion is synced, then the directory. A whole-record orphan is resumed only by the exact retry, and the order rule (params before percentiles before dispositions) is enforced. |
| Execution V4 creation durability | execution_v4.rs:2468-2481 | new file entry not durable | The directory is fsynced when any child is created, and children are created under the writer flock. |
| Selection V6 persist | selection_v6.rs:296-360 | payload/seal ordering, directory sync, lock | Exclusive flock across scan+write; payload synced before seal; seal synced; directory synced. Readers hold a shared flock. An exact partial prefix resumes. The wedge after an interrupted persist with a changed source, and every committed read refusing while a partial tail exists, are already **hunt-cli-a-5**. Selection V6 also lacks write-error rollback, but there a short write is an exact prefix that the exact retry completes, so it adds nothing material to hunt-cli-a-5. |
| Selection V6 authority reads | selection_v6.rs:117-133, 362-381 | stale cache | Every read re-derives the source, rescans under a shared lock and re-derives again. Nothing is cached. |
| Foreign orphan wedge in Execution V4 | execution_v4.rs:2824-2833 | crash, then source change, refuses all new commits | Intentional, documented design ("foreign/orphan Execution bytes ... refuse", 04-invariants EX-03 and siblings). Same class as hunt-cli-a-5; not re-reported. |
| Selection V5 | selection_v5.rs:1906-2440, 3019-3055 | create before lock, rollback | Children are created before the lock with `create(true).truncate(false)`. That is harmless, and the directory is fsynced when created. Rollback is present (D-1622). The module is dead in production. |
| Selection V1–V4 ledgers | selection.rs:1769-1995, 2085-2310; selection_v3.rs:1115-1330; selection_v4.rs:1700-1910 | no rollback; V1–V3 `create_dir_all` with no directory fsync | These defects are real in the code, but no production caller writes these ledgers (only tests; V4 is read by the unreachable `step3_comparison`). Not raised. |
| selection_v4_authority nested flocks | selection_v4_authority.rs:89-123, 177-188 | outer shared lock held while inner ledgers take the same lock file | All acquisitions are shared, and Linux flock grants a compatible shared request even while an exclusive waiter is queued, so there is no self-deadlock. Unreachable anyway. |
| HashMap → output | selection_v6_source.rs:91-130; execution_*; selection_* | iteration order leak | Maps are used for lookups only. Output order comes from file order / `Vec`. |
| Threads, atomics, statics | whole slice | races | No production thread, rayon, atomic, static or Mutex. Atomics appear only in test root counters. |
| Step-3 root admission | step3_orchestrator.rs:354-445 | rename-and-replace TOCTOU | The residual pathname race is acknowledged in the doc comment (:361-367), and the post-check refuses. Not a new finding. |
| step3_comparison `exists()` absence | step3_comparison.rs:510-947 | stat error reported as absence | Already **errpaths-6**. |

---

<!-- conc-pass1/search.md -->
### conc-pass1 / search: 2 findings (0 high, 2 medium, 0 low) at 331b05c

Slice: crates/cli/src/anchored_search_lineage_{v2,v3,v4}.rs, candidate_universe.rs, pre_admission_data.rs,
boolean_observation_file.rs, boolean_*.rs (non-test), index_stop_search_{checkpoint,progress,reader}.rs,
index_stop_source_context{,_codec}.rs. Audit only; no cargo run, nothing edited.

## search-1 (medium): a crash inside a content-addressed Boolean/index-stop evidence write wedges that identity forever, and with it every resumable search whose pending batch needs it

File: crates/cli/src/boolean_candidate_persistence.rs:189-211 (`write_or_equal`), reached from :119 (`body.bin`) and :74 (`complete.bin`).

```rust
fn write_or_equal(path: &Path, body: &[u8]) -> Result<(), String> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(body).map_err(display)?;
            file.sync_all().map_err(display)?;
            ...
        }
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_exact(path, body.len() as u64)? != body {
                return Err("Boolean evidence already exists with different or incomplete bytes; history preserved".to_owned());
            }
            Ok(())
        }
```

Why it is wrong. The file is created with `create_new` *before* any byte is written or synced, and nothing distinguishes "a crash left a short prefix of my own bytes" from "different evidence". Every namespace that goes through `prepare_in_namespace` gets its directory name from a deterministic identity. That covers boolean-candidates/statistics/admission/oos/qualification-v1, index-stop-candidates/qualification/source-context/catalog-context/vix-reference-v1 and index-consistency-v1. Some identities are a hash of the request, source and commit (`boolean_candidate_v1.rs:300` `catalog_identity`). Others are a hash of the body itself (`index_stop_source_context.rs:318` `context_identity(body)`). So the only retry that can ever happen writes the same identity again. That retry reaches the `AlreadyExists` arm and refuses for good. Nothing deletes or repairs a directory here, and that is correct under the append-only rule. This breaks CLAUDE.md §3 rule 5 ("Reruns are safe"). It also breaks the promise printed by `index_stop_search.rs:302` ("saved successful children are retained and exact retry reuses them"). The single-stop checkpoint makes the retry mandatory: `Frame::done` and `transition` (index_stop_search_checkpoint.rs:165-207, 529-543) only accept a completion of the exact reserved batch. So the search cannot route around the dead child.

Repro (process death):
1. `index-stop` single-stop search publishes the pending frame (index_stop_search.rs:230-231) and enters `live::parallel`.
2. One lane reaches `index_stop.rs:401` `prepare_in_namespace(root, "index-stop-candidates-v1", identity, &body)`. It enters `write_or_equal(body.bin)`, `create_new` succeeds, and the process is SIGKILLed during `write_all` of a multi-page body. Linux `generic_perform_write` stops between pages on a fatal signal and leaves a short file. Power loss has the same effect: it can strike after `create_new` and before `sync_all`, which on ext4 delalloc leaves a 0-byte or partial `body.bin`.
3. The flock dies with the process. The next identical invocation recovers the pending frame (`checkpoint::recover`) and re-runs the same batch. That rung recomputes the same identity, takes the owner lock (the owner file is empty, so the check passes), and hits `AlreadyExists`. `read_exact` returns the short prefix, `!= body`, and the call refuses with "already exists with different or incomplete bytes".
4. `completed_links` turns that into "Single-stop batch remains pending ...". Every later invocation repeats steps 3 and 4, so the search can never advance. The same happens for `complete.bin` if the crash lands between its `create_new` and `sync_all`: a 0-byte receipt. `Observation::open` and `persistence::verify` also refuse that identity for every reader.

Minimal fix. Write to a temporary name in the identity directory while holding the owner lock (unique per process, e.g. `body.bin.<pid>.<nonce>`), then `sync_all`, then `rename` (or `link`) onto the final name, then fsync the directory. The final name then appears only complete. Alternatively, keep `create_new`, but in the `AlreadyExists` arm, while holding the exclusive owner lock, treat a file that is a strict byte prefix of `body` (including empty) and has no `complete.bin` beside it as an interrupted write of this same evidence. Complete it in place (append the suffix, `sync_all`). Refuse only on a true mismatch. Add a test that truncates `body.bin` and `complete.bin` to 0 and to N/2 and expects the exact retry to succeed.

Not reported before. hunt-conc, errpaths and the other workspace hunts do not mention `write_or_equal`, `body.bin` or `complete.bin` torn writes, and docs/06-limits.md has no entry for them.

## search-2 (medium): Pre-Admission Data V1/V2 append has no write-error rollback, so one ENOSPC/EIO mid-record makes the ledger unreadable for every later open, read-only included

File: crates/cli/src/pre_admission_data.rs:3595-3599 and :3817-3824. Call sites :1335, :1376, :1385 (V1) and :2557, :2598, :2607 (V2).

```rust
fn append_record(file: &mut File, raw: &[u8; RECORD_BYTES]) -> Result<(), PreAdmissionDataRefusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append pre-admission record: {why}"))
}
```
and the open-time check at :3561-3571 (V2 has the same check at :3785-3795):
```rust
    if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V1) {
        return Err(format!("pre-admission file body has {body} bytes, ragged against ..."));
```

Why it is wrong. This is exactly the defect D-0916 (W2-cli10-3) fixed for Admission V3/V4. D-1620 fixed it for Search Lineage V4. `candidate_universe.rs` `append_rows` and `append_receipt` (:6255-6324) already roll back with `set_len(original)`. Pre-Admission Data was missed, and neither docs/05-decisions.md nor docs/06-limits.md names it. `write_all` that fails partway leaves the bytes it did write. Pre-admission records are 740 bytes (V1) and 812 bytes (V2), so they regularly straddle a 4 KiB block. The ledger is live: `step3_orchestrator.rs:3899` and `:3969` write it, and `stored_family_v6.rs:169` and `:200` read it with `open_read`.

Repro:
1. Run the step-3 transaction on a filesystem with less free space than one block. `append_complete_locked` writes the Data record (:1376). Its `write_all` copies the bytes that fit before the block boundary and then gets ENOSPC. The error is returned, but the partial bytes stay in the file.
2. Free some space and rerun. `PreAdmissionDataLedgerV1::open` reaches `record_count(file_len)` (:1113). The body is no longer a multiple of 740, so the open refuses with "ragged against 740-byte records".
3. The same refusal hits `open_read` from `stored_family_v6.rs:169`. Every previously committed pre-admission authority in that ledger is now unreadable, although none of its bytes changed. Nothing in the code repairs the tail. The same sequence applies to the Completion record (:1385) and to V2.

Minimal fix. Port `append_with_rollback` (anchored_search_lineage_v4.rs:1645-1665). Record `end = seek(End)`; on a `write_all` error, `set_len(end)` and name both errors if the truncation fails too. Use it in `append_record` and `append_record_v2`. Add the D-0916-style test `a_partial_append_error_truncates_back_and_committed_authority_stays_readable` for both versions, and list the change under D-0916/D-1620 in docs.

---

## Checked and not reported

- **index_stop_search_progress.rs `parallel` / `schedule`.** The `Relaxed` `fetch_add` work counter only hands out indices. Results are reassembled by index (`assemble` refuses duplicates and gaps). `cancelled` uses Release/Acquire. `for … in receiver` drops the receiver on `break`, so a blocked `SyncSender::send` returns Err and cannot deadlock. A panic in a lane propagates through `broadcast` to the scoped thread, and `join` maps it to "pending checkpoint retained". All of this was also noted by hunt-conc.
- **index_stop_search_checkpoint.rs.** A pending frame is published before the work and a done frame after it. `transition` forbids skipping a reservation, re-assigning one, or changing a reserved batch. Recovery walks the pinned predecessor chain and requires `history.len() == journal.acknowledged()`. I found no skip or double count. Crash holes in the journal are handled by search_checkpoint (out of slice; the `complete` marker issue is the known GAP11-0).
- **boolean_qualified_journal.rs.** The Writer is poisoned on any attempted publication until a validating reopen. It relies on the Journal's exclusive owner lock, which is held for the Writer's whole lifetime.
- **boolean_campaign.rs and boolean_grammar_campaign.rs.** Their checkpoint chains are safe after a publish error because `Journal::publish` self-poisons. The refusal publish after a failed publish therefore errors loudly instead of forking the chain.
- **boolean_observation_file.rs.** Leases are taken on a duplicated owner descriptor, so they share one OFD. The `projecting` CAS refuses nested or concurrent use of that one OFD. Field drop order unlocks before the flag clears. `with_current_many` sorts and dedups by directory before taking leases, so acquisition order is consistent. All locks are shared.
- **anchored_search_lineage_v4.rs.** The open, lookup and append paths each take and release the lock-file flock around their work. A generation recheck catches a writer from another process. Members are written in one call with write-error rollback (D-1620). A *process-killed* ragged tail is still refused for good, but this class is documented in docs/06-limits.md ("Interrupted Population ledger writes ... Still refused", and the V2/V3 section), so it is not re-reported. A side note: the D-1620 comment "the only whole-record prefix a crash can leave is the NIFTY member" is true, but a crash can just as easily leave a non-record-aligned prefix, which still refuses. V2 and V3 are dead code (documented).
- **candidate_universe.rs.** Rows are written before the receipt, there is a whole-row orphan rule, and write errors roll back. Opens are generation-checked. The process-kill ragged tail is the same documented class as above.
- **HashMap use in candidate_universe, pre_admission_data and lineage_v4.** These maps are used only for keyed lookup and duplicate detection. No iteration order reaches output bytes or a digest.
- **Worker-count text** (boolean_catalog_prepared.rs:206, boolean_oos_command.rs:92, index_stop_search.rs:206) was already reported as hunt-conc-5.
- **Durable writes inside rayon workers at the Boolean sites** were already reported as hunt-conc-2.
- **No directory fsync after creating the candidate_universe and pre_admission_data files.** Not reported: on ext4, XFS and btrfs, `fdatasync` of a newly created file commits its directory entry, so I could not build a concrete loss path.

---

<!-- conc-pass1/xcut.md -->
### conc-pass1 / xcut: 3 findings at 331b05c (0 high, 1 medium, 2 low)

Verdict: the atomic-publish helpers are mostly sound. Every rename-based publish syncs the file before the rename. Every durable one also syncs the parent directory after the rename. Every fixed temp name is written only while a flock is held. The cross-process defect that remains is the telemetry sink. The api server and the cli write the same `events.ndjson` whenever `BRUTEX_LOG_DIR` is set, and D-0301's own text tells the operator to set it "for both". Two smaller defects: the census reports "not published" after its rename already happened, and candidate-trades directories are created without syncing their new ancestors.

Slice: CROSS-CUTTING across the whole workspace. Every production `fs::rename`, `hard_link`, `create_new`, `File::create`/`truncate(true)`, temp-name scheme (`.tmp`, `.partial`, `.writing`, `.pending`, pid-stamped) and `sync_all`-around-publish. Test modules (`mod tests`, `*_tests.rs`, `tests/`) were excluded after checking each hit's position. Source reading only. No cargo was run, nothing was edited.

## Helper inventory (deduplicated, one row per helper)

| # | helper | file:line | temp name | file sync before publish | parent-dir fsync after | unique across procs/threads | orphan on restart | create_new vs truncate | api+cli same path? | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| H1 | census install (`install_locked`→`publish`) | pull/src/ingest.rs:3340-3366 | `<vendor>.man.writing` (fixed) | yes, `sync_all` | yes (3365) | serialised by the `CensusLock` flock on `<vendor>.man.lock` (ingest.rs:2990-3110); both api ingest and cli ingest take it | left behind, truncated by `File::create` on the next install, never listed (no `read_dir` over `manifest/`) | truncate under lock: safe | both, serialised by the lock | **xcut-2** (misreport after the rename). Lock-bypass arms are pull2-1 |
| H2 | census in-place append (`write_appends`) | pull/src/ingest.rs:3313-3330 | none (positional) | `sync_data` before the slot, `sync_all` after | n/a | same lock | n/a | n/a | same lock | sound for writers. Lock-free reader tear is pull2-5. **xcut-2** covers its error text too |
| H3 | masters `replace_locked` | pull/src/masters.rs:871-918 | `.<file>.partial` (fixed per source) | yes | yes, and a failure there is reported as `Landed::Uncertain`, not as a refusal | blocking `Flock` on `.<file>.lock` (masters.rs:841-852) | removed on every failure arm. A crash orphan is truncated by the next `truncate(true)` under the lock | safe under lock | both, serialised | clean |
| H4 | `cli::live::Live::publish` | cli/src/live.rs:368-392 | `<hex>.<pid>.tmp` | `sync_data` | **no**. Deliberate: "this file is not history" (live.rs:375-378) | pid-unique across processes. Same-thread reuse only, because one `Live` owns one identity path | crash orphans stay forever. The reader counts them as strays, not runs (live.rs:1553-1616) | `File::create` truncates its own pid name | api sweeps and cli sweeps share `results/live/` but are serialised by `execution_lease` (`.sweep-execution-v1.lock`, cli lib.rs:2101, api sweeprun.rs:1761), so two processes cannot publish one identity at once | clean (documented limits) |
| H5 | expression evidence (`EvidenceWriter`) | cli/src/expression.rs:218-345 | `expression-v1.pending` inside a per-attempt-token directory | yes | yes. Every ancestor is synced on create (expression.rs:146-170), and the dir is synced again after link and unlink | per-token directory from the sweep-evidence journal | a crash leaves a `.pending` in a dead token's directory. A rerun gets a new token | `create_new` plus a `hard_link` publish that never replaces anything | n/a | clean |
| H6 | sweep-evidence token reservation (`reserve_start`) | cli/src/sweep_evidence.rs:1045-1065, 507-560 | `<token>-start.bin` | `barrier` = `sync_all` | `flush_directory(base)` before and after | tokens come from the flock-appended `attempts.bin` | a torn start file affects only its own dead token | `create_new` | shared journal, serialised | clean |
| H7 | candidate detail `write_exact` | cli/src/candidate_trades.rs:1307-1349 | none (final name, `create_new`, then `Flock::lock`) | yes | leaf only | per `(identity, token)` directory | per-token, so a rerun is unaffected | `create_new`. A reader can try-lock-shared in the gap before the writer's lock and get a transient "truncated" or "busy" refusal | n/a | **xcut-3** (ancestors not synced) |
| H8 | search checkpoint `publish_inner` | cli/src/search_checkpoint.rs:215-285 | `<seq>/payload` plus `<seq>/complete` | yes | yes | owner flock plus the sequence | interrupted seq directories are counted as `interrupted` | `create_new` | n/a | torn `complete` marker is KNOWN (GAP11-0 / hunt-cli-b re-1). Still present at 331b05c (lines 262-266, 457-463) |
| H9 | Boolean persistence `write_or_equal` / owner.lock | cli/src/boolean_candidate_persistence.rs:96-125, 189-213 | none | yes | yes | owner flock | crash wedge = search-1 | `create_new` / compare-equal | n/a | reported by search-1 |
| H10 | operation audit per-invocation file | cli/src/operation_audit.rs:556-567 | `own(base,id)` | `write_synced` | yes | id from the flocked `index.bin` | — | `create_new` | api and cli share `index.bin` under a flock | clean (recovery slice covers the rest) |
| H11 | NSE cash-session cache | pull/src/cash_session_cache.rs:483-529 | none (payload, then receipt) | yes | yes | `try_lock` per day | a payload without a receipt is a permanent loud refusal = pull2-3 | `create_new` | api and cli refuse each other promptly (pull2-4) | reported |
| H12 | vendor capture | pull/src/capture.rs:224-276 | `<prefix>-p<pid>-t<stamp>-c<n>.txt` | yes | no (diagnostic) | pid, per-slot `fetch_update` seq, and 16 `create_new` attempts | torn-capture-at-final-name = pull1-3 | `create_new` | n/a | reported |
| H13 | store repair revision | store/src/repair.rs:327-397 | `.reserved-v1` reservation and `.repair-v1` receipt | yes | `sync_ancestors` before and after the receipt | `create_new` reservation | the reservation is never removed, by design. An interrupted revision number is burnt | `create_new` | no production caller | store2-1 |
| H14 | population / admission / finalization / execution `open_child` | e.g. cli/src/population_v5.rs:3203-3238, execution_v4.rs:4680-4712 | none (create-or-open journals) | per-journal | pop1-3 for Observation ledgers | root flock | pop1-1/2 | `create_new` then reopen without truncate | n/a | covered by pop1 |
| H15 | autopilot write probe | api/src/autopilot.rs:590-627 | `<STORE_PROBE_PREFIX><vendor>` (fixed) | yes | no (the file is removed again) | one autopilot task | a 19-byte orphan, truncated next time | `File::create` | api only | clean (autopilot slice) |
| H16 | telemetry roll (`Sink::roll`) and the append target | telemetry/src/sink.rs:1267-1303, 246-271, 732-758 | none (renames `events.ndjson` to `.1` ... `.N`) | append target `sync_all` on demand | **no** | **in-process only** (`OnceLock` in `telemetry::install`). No flock anywhere in the crate | — | `append(true).create(true)` | **YES whenever `BRUTEX_LOG_DIR` is set** | **xcut-1** |

## xcut-1 (medium): with `BRUTEX_LOG_DIR` set, the api server and every cli process append to and rotate one telemetry set with no cross-process lock

This extends telemetry-1 (this pass) and hunt-costs-3 (audit-20261003), which are both still present at 331b05c. Those reports scope the race to cli against cli. telemetry-1 says the cli doc's "each owns its own events.ndjson" claim "holds for api versus cli". It does not hold. Both resolvers return `BRUTEX_LOG_DIR` verbatim, and the project's own decision text tells the operator to set it for both.

Code:
- api: `crates/api/src/server.rs:18266-18278`
  ```rust
  fn log_dir_from(named: Option<std::ffi::OsString>, cwd: Option<&Path>, store_root: &Path) -> PathBuf {
      if let Some(named) = named {
          return PathBuf::from(named);
      }
  ```
- cli: `crates/cli/src/lib.rs:2990-2997`
  ```rust
  explicit
      .map(std::path::PathBuf::from)
      .or_else(|| store.map(|s| s.join("logs").join("cli")))
  ```
  Its own doc says "`BRUTEX_LOG_DIR` still wins outright and is used exactly as given" (lib.rs:2982-2983). Directly above (lib.rs:2959-2962) the same doc records that two live processes appending to one `events.ndjson` was "a corrupted record of the one thing that exists to say what happened".
- `docs/05-decisions.md:18914-18916`: success prints that "`/logs` reads a different one **unless `BRUTEX_LOG_DIR` is set for both**". That is the operator instruction that produces the shared directory.
- Sink: `crates/telemetry/src/lib.rs:169-183` refuses only a second sink in one process (`GLOBAL: OnceLock`). `Sink::open` (sink.rs:732-758) resumes `seq` and `reserved_run` from the newest record on disk, and `roll` (sink.rs:1267-1303) renames `events.ndjson` to `.1` with no lock. `crates/telemetry/Cargo.toml` has an empty `[dependencies]`, and no `lock`/`flock` call exists in `crates/telemetry/src`.

Why it is wrong: the api server is a long-lived process that rotates on its own byte count, and a cli sweep (`range-all`, hours) runs alongside it. That is the exact pairing lib.rs:2959 names as the reason the `cli/` subdirectory exists. Under `BRUTEX_LOG_DIR` the subdirectory is bypassed for both processes.

Repro (exact interleaving, two processes, `BRUTEX_LOG_DIR=/x` exported in the shell that starts both):
1. `api` starts. `Sink::open(/x)` resumes `seq = S` and `reserved_run = S` from `/x/events.ndjson`.
2. The operator runs `cli range-all ...`. `install_log` opens `/x` too and also resumes `seq = S`. Each process now numbers its own lines S+1, S+2, ... into the same file, so every `seq` value appears twice. `Tail::missing` (tail.rs:351-363) computes `span - records.len()` and saturates to 0, so a real dropped-event gap is hidden.
3. api's `inner.bytes` crosses `max_file_bytes`. Its `roll` renames `/x/events.ndjson` to `/x/events.1.ndjson` and reopens a fresh current file. The cli still holds the old descriptor, so its lines now land in `events.1.ndjson`, newer than lines already in the new current file. `tail`'s newest-first `since` walk stops at the first older record (D-1325's invariant that `ms` order is file order). `/logs` then omits the cli's later events for any `since` query. The `.1` file also grows past its byte bound.
4. The cli then rolls on its own counter. It deletes `.N`, shifts `.1` (its own live file) to `.2`, and renames api's current file to `.1`, while api keeps writing into what is now `.1`. Retention roughly halves because both processes roll the one set.
5. Restart api after the cli wrote last. `resume_point` takes the cli's final `seq`, which can be below api's last `seq` because the two numbered independently. api's `reserve_run_id` (sink.rs:1032-1039) then hands browser attempts (`server.rs:6953`, `claim_run(id)`) run ids that api's own earlier events in the rotated files still carry. That is the D-1326 regression the resume point exists to prevent.

Minimal fix: take a non-blocking exclusive `std::fs::File::try_lock` on a `<dir>/.sink.lock` inside `Sink::open` and hold it for the sink's lifetime. On `WouldBlock`, refuse loudly: api refuses at startup, and the cli prints its existing "events are NOT being recorded: ..." line. Alternatively, have the cli always append `cli/` (or `cli/<pid>`) beneath an explicit `BRUTEX_LOG_DIR`, and have `api::logs::cli_half` follow the same rule. Either fix also closes telemetry-1 and hunt-costs-3.

## xcut-2 (low): census install reports "not published" after its rename has already published, unlike the masters helper beside it

Code: `crates/pull/src/ingest.rs:3357-3366`
```rust
fn publish(dir: &Path, tmp: &Path, path: &Path, image: &[u8]) -> std::io::Result<()> {
    ...
    fs::rename(tmp, path)?;
    fs::File::open(dir)?.sync_all()
}
```
`install_locked` (3340-3351) turns any error here into "`{path}` could not be published through `{tmp}`". The caller (ingest.rs:936-945) records the failure "`{n}` slice(s) are on disk and the census that counts them was not published", and `note_census_unpublished` emits `pull.census` / `not published` at Error. The append path has the same shape: `write_appends`'s final `file.sync_all()?` (3328) fails after the slot bytes are already written and visible. Compare `pull/src/masters.rs:907-917`. That helper names exactly this state ("the new bytes are visible, but crash durability is UNVERIFIED") and returns `Landed::Uncertain` instead of a refusal.

Why it is wrong: after the rename, every reader (api census, autopilot `manifest_loads`, the next ingest) sees the new census. So the receipt and the Error event make a false statement about the store. The true state is "published, durability unconfirmed". This is §3 rule 6 (honest limits): the operator is told to recover from a census that is in fact present and counting the slices.

Repro: the process gets EIO (or EROFS after a remount) on `File::open(manifest_dir).sync_all()` at ingest.rs:3365, right after `fs::rename` at 3364 succeeds. The receipt says the census "was not published". A `GET` of the census page in the same second shows it Held with the new counts. The next ingest of the same folder finds every slice already counted, and nothing is appended.

Minimal fix: split `publish` so that an error after the rename returns a distinct `Uncertain(why)` variant, as `masters::replace_locked` does. Word the failure as "published; directory sync failed, crash durability UNVERIFIED". Apply the same split to `write_appends` after its slot write.

## xcut-3 (low): candidate-trades directories are created with `create_dir_all` and only the leaf is ever synced

Code: `crates/cli/src/candidate_trades.rs:321-322`
```rust
let directory = directory_for(root, &attempt.identity(), attempt.token(), model);
fs::create_dir_all(&directory).map_err(io_error)?;
```
This creates up to four new levels: `results/{candidate-trades-v1|expression-candidate-trades-v1}/<identity>/<token>`, per `directory_for` (1249-1257). The only directory barrier in the module is `File::open(path.parent())...sync_all()` in `write_exact` (1342-1344), which syncs the leaf `<token>` directory. The entries `<token>` in `<identity>`, `<identity>` in the namespace, and the namespace in `results/` are never synced. Same module's neighbour `expression::attempt_directory` (expression.rs:146-170) states the rule this breaks: "Syncing only the leaf would not persist newly created ancestors." `sweep_evidence::durable_directory` (1115-1130) does the same for its own tree.

Why it is wrong: the sweep-evidence lifecycle row that records the attempt `Completed` is synced (`barrier`) after these files. After a power loss on a filesystem that does not order a later fsync of another inode behind an earlier `mkdir`, the completion row can be durable while `<identity>/<token>` is gone. Every later candidate read of that completed attempt then refuses as missing or corrupt detail rather than showing it. This is the same class as pop1-3 at a new site.

Repro: run `audit-stored` (or `expression-backtest-stored`) for a new identity on a fresh store. Cut power after `attempt.finish(Completed)` returns, before the kernel's periodic writeback commits the parent directories. On restart the lifecycle says Completed, and `results/candidate-trades-v1/<identity>/` may be absent. Whether the entry survives depends on the filesystem's metadata ordering, not on anything this code guarantees.

Minimal fix: replace the `create_dir_all` with the `durable_directory`-style walk. Create each missing level with `create_dir`, then sync its parent, as `expression::attempt_directory` already does.

## Checked and clean / already reported (not counted)

- No production helper uses a temp name that two processes or two threads can both claim without a lock. Fixed names (H1, H3, H15) are always written under a flock or by a single task. Pid-stamped names (H4, H12) are pid-unique, and the in-process callers own distinct paths.
- Every rename-publish syncs file data before the rename (H1, H3, H4). H1 and H3 sync the parent directory after. H4 deliberately does not, and documents why.
- api against cli on result outputs: every sweep command on either side takes `execution_lease` on the canonical store (cli/src/lib.rs:2096-2103, api/src/sweeprun.rs:1756-1770), so `results/live`, the run ledger and the evidence trees are not written by both at once. Census, masters, cash cache, operation audit and sweep-evidence journals each take their own flock.
- Orphans are never "cleaned up on restart" anywhere. Each one is either truncated on the next use of its fixed name (H1, H3, H15), confined to a dead token or pid (H4, H5, H6, H7, H12), or deliberately left as a loud refusal (H11 pull2-3, H13 store2-1, H9 search-1).
- Already reported and still present at 331b05c: GAP11-0 (checkpoint `complete` marker), search-1, pull1-3, pull2-1, pull2-3, pull2-4, pull2-5, store2-1, pop1-3, telemetry-1, and hunt-costs-3.

---

## Pass 2 (20 auditors, sliced by flow, same commit 331b05c)

Pass 2 found 51 new findings: 0 high, 16 medium and 35 low. That is fewer than pass 1's 63, but not zero.

**New medium findings:**
- barflow-1: readers' shared locks make the ingest writer's try_lock fail, and derived rungs are never re-derived.
- recauto-1: the autopilot never claims site.run, so recovery attempts burn on a busy seat.
- equity-1: a later chunk lands after an earlier one fails, leaving a gap that can't be refilled.
- replay-1: a VIX lock refusal is saved permanently as Unavailable.
- replay-2: a VIX companion retry recaptures different bytes and wedges the search.
- census-1: the census cache keeps a stale or torn slot under the new mtime.
- clock-1: the autopilot builds its feed list once, from the boot clock.
- clock-2: finished_day_only trusts the wall clock with zero margin, so a partial day can be appended.
- ledgerv6-1: retained strict inputs exhaust file descriptors on spans of 5 months or more.
- sweep-1: a Ctrl-C'd cli sweep blocks browser runs after 15 minutes.
- sweep-2: a kill inside a multi-page append tears frontier/trades/detail-sets/runs.bin for good.
- logs-1: a non-blocking invocation-lock 429 leaves Run disabled until reload.
- ledgerall-1: shared store-root ledgers wedge after Ctrl-C, because the retry identity includes data digest and commit.
- ledgerall-2: Selection V5 commits fewer than 25 winners, then preflight refuses forever.
- resources-1: an fsync failure is retried into Reused on the run-commit path.
- resources-2: /bars/window.json opens 3 fds per month, up to 720, against macOS's 256 limit.

**Pass-1 claims refuted or narrowed:**
- store1's ingest.rs:2224 stale-fold note is refuted (barflow).
- pop2-6 is refuted: the header is a single 64-byte write.
- pop2-2/pop2-3 hold only for Statistics V2. Admission V3, Finalization V3 and Population V5 (pop1-2) records are page-aligned (ledgerall).
- xcut-1 named the wrong variable: the api reads BRUTEX_LOGS, not BRUTEX_LOG_DIR (lifecycle, logs).
- cli3-1's api-vs-cli race case is refuted (indexstop).
- sel-1's "execution_v3/selection_v5 are dead" note is refuted: they are live via ledger-all.
- The "panic becomes Failed/Retry" claims are false in release, where panic = abort (poison).
- runs-1, the Finisher race, is confirmed but with a narrow window.

Every other pass-1 finding that was re-checked is confirmed.

### Pass 2 reports

---

<!-- conc-pass2/barflow.md -->
### conc-pass2 / barflow: end-to-end bar write flow (vendor fetch -> pull::ingest -> rung fold -> store::file append), at 331b05c

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

---

<!-- conc-pass2/press.md -->
Verdict: no high or medium defect found in the Pull press slice. 2 findings, both low. Every pass-1 finding that touches the slice is still present (11 CONFIRMED, 0 REFUTED).

Slice: the api Pull press end to end at 331b05c. Covered: the `/pull/spot`, `/pull/fno`, `/pull/run`, `/pull/run.json` and `/pull/run/stop` handlers (crates/api/src/server.rs:10884-11125, 14833-14895); `pullrun.rs` (claim, `conduct_with`, `run_pass`, `run_chain`, `Finisher`, ticker, `legs_from`, `by_feed`, `rows_now`); `broker_answer`, `broker_run`, `land_broker_member`, `land_spot`, `prepare_cash_schedule`, `land_bodies_observed`, `await_budget`, `credential_halts`, `recovery_spot`, `fno_walk`, `read_month_bars`; the seats (`take_seat`, `take_every_seat`, `Seat`/`AllSeats` Drop, autopilot.rs:2097-2268); `audit::Journal::append`/`appended`/`page` (audit.rs:1143-1335); `recorded_fact` and `recorded_with_failures` (server.rs:10616-10690); `recovery_control::stop`/`persist`; and how a `cli` process on the same store meets these paths (`cli::stored` load, `fold_audit`, `pull::ingest::derive_all`, `store::file::BarFile::open_existing`/`open_or_create` locks). I read the source only and did not run cargo.

Already filed elsewhere, so not counted here: lifecycle-1 in pass 2 (a press is not awaited at shutdown, so the receipt of its in-flight leg is lost). I had drafted it independently and dropped it. tests-docs-security (`legs_from` does not check the envelope vendor/dir against the payload). The concurrency consequence of that one is noted under "Checked" below.

---

## press-1 (low): writes by other pullers count as press growth, so a hand pull running beside a press holds off its idle stop, forces back-to-back full re-pulls and can spend the 400-pass ceiling

- **Where:** crates/api/src/pullrun.rs:509-516 (`rows_now` sums every vendor's census) and :966-983 (`conduct_with`), together with :853-857 (`run_pass` clears every checkpoint when no feed is retrying).
- **Code:**
  ```rust
  pub(crate) fn rows_now(site: &Site) -> u64 {
      let (censuses, _) = crate::server::census_now(site);
      censuses.iter().filter_map(census::VendorCensus::counters)
          .map(|(_months, rows, _entries)| rows).sum()
  }
  ...
  let before = rows_now(&site);
  run_pass(&site, &groups, &mut outcomes, &checkpoints, &request).await;
  ...
  let after = rows_now(&site);
  ...
  if after > before {
      clean_empty = 0;
      continue;            // no RETRY_WAIT, straight into the next pass
  }
  ```
  and in `run_pass`: `if !retry_cycle { for leg in clean.iter() { leg.store(false, ..) } }`.
- **Why it is wrong:** The press uses growth of the whole store across all five vendors as its proof that "this pass found new bars". Other writers can add rows while a press runs: `pull_spot` and `pull_fno` check only their own feed's seat and never `site.run`. Their rows land in the same sum. So a pass in which the press itself added nothing reads `after > before`. The loop then:
  - resets `clean_empty`;
  - skips the `RETRY_WAIT` sleep;
  - starts a fresh pass with every checkpoint cleared.

  Every leg of every feed in the basket is asked of the vendor again, back to back, for as long as the foreign writer keeps landing rows. Three things follow:
  - **Vendor quota.** A full basket is re-requested on each such pass, over one shared token.
  - **The pass ceiling.** It is spent on passes that are not the press's own work. Archive-feed legs (TrueData/GDFL) re-read a local folder and finish in seconds, so a long hand walk can burn most of `MAX_PASSES = 400`. The run then ends with "Reached the 400-pass ceiling … This is a runaway stop" (`summary_of`), which is a false verdict about the press.
  - **Misattributed counts.** The press summary's "N bar(s) added to the store census" (`current_rows - started_rows`) includes bars the hand pull wrote.

  conc-pass2/census.md covers the opposite error: a stale census cache making `after == before` when the press did add rows. This one inflates instead.
- **Repro:** Press Pull with one feed, dhan 1day over the tracked universe, for a month already fully on disk, so each pass lands 0 rows. While that runs, POST `/pull/fno` for groww, a walk that "can run for half an hour" and lands rows steadily, or a `/pull/spot` on groww for a missing month. In each press pass, groww's manifest grows, so `after > before` holds. `clean_empty` stays 0, no sleep follows, and pass N+1 clears the checkpoints and re-requests all ~800 dhan instruments. The press cannot reach its 3-clean-pass idle stop until the groww walk finishes. With an archive-feed press instead of dhan, passes are short enough that `passes` climbs toward 400. `/pull/run.json` shows `passes` rising and `rowsNow - rowsAtStart` growing by groww's rows.
- **Minimal fix:** Measure only the press's own feeds. Have `rows_now` take the vendor set of `groups` (`Feed::store_vendor` of each group key) and sum only those censuses. Better, count the press's own receipts: sum `bars_stored` out of each leg's `Ingested`, which `request_leg` would have to return. The rows of a feed the press does not touch then cannot reset its idle counter. A same-feed hand pull between two legs is still counted; refusing `/pull/spot`/`/pull/fno` while `site.run` is running for that feed would close that too.

## press-2 (low): the pull journal's first record is reported "Recorded: yes" before the new `audit/` directory and `pull.journal` directory entries are durable

- **Where:** crates/api/src/audit.rs:1172-1220 (`Journal::appended`).
- **Code:**
  ```rust
  if let Some(dir) = self.path.parent() {
      match std::fs::create_dir(dir) { Ok(()) => {} ... }
  }
  let mut file = Flock::try_lock(std::fs::OpenOptions::new()
      .read(true).append(true).create(true).open(&self.path)..., ...)?;
  ...
  file.write_all(&record.image())...;
  file.sync_all()...;      // the file's inode and data only
  drop(file);
  Ok(())
  ```
  Nothing in audit.rs (non-test, lines 1-1576) opens or syncs a directory: `grep sync audit.rs` finds only the file's `sync_all`.
- **Why it is wrong:** The module promises "written where a restart cannot lose it … `fsync`-ed before the answer is rendered" (audit.rs:1-12), and `recorded_fact` renders "Recorded: yes — appended to …" on that basis. POSIX makes a newly created file reachable after a crash only once its parent directory is fsynced. Here two new entries are created and neither parent is synced: `audit/` in the store root, and `pull.journal` in `audit/`.

  On the first record ever written to a store, a power loss before the filesystem's next metadata commit can leave the synced data blocks unreachable. The next boot sees `Log::Absent` ("never pulled"), and the bars of that run are on disk without their receipt. That is the state the module calls "a defect in the journal".

  The sibling writer that shares the directory gets this right: `recovery_control::persist` syncs `audit/` and the store root after creating children (recovery_control.rs:148-168). So does `recovery_journal::open`. The window is one-time per store: once the entries have been committed, later appends only extend an existing inode, and `sync_all` covers them.
- **Repro:** Fresh store root with no `audit/`. Press Pull (or a hand `/pull/spot`) on one instrument. `appended` runs `create_dir(audit)`, then `open(create)` of `pull.journal`, `write_all` and `sync_all`, and the page says "Recorded: yes". Cut power within the filesystem's commit interval (ext4 default 5 s), before any other metadata sync. After reboot, `audit/pull.journal` (or `audit/` itself) can be missing. `/audit` reports no journal while the manifest holds the run's rows. Not executed: this needs a power cut, and the claim rests on POSIX directory-entry durability, not a measurement.
- **Minimal fix:** After a successful `create_dir(dir)`, fsync the store root. When the `open` actually created the file (check `bytes == 0` under the lock, or use `create_new` first), fsync `dir` before returning `Ok`. Both run once per store lifetime, so the steady-state append cost is unchanged.

---

## Pass-1 verification

- **server1-1 (two parallel feed legs refuse each other's audit records): CONFIRMED.**
  - audit.rs:1190-1205 still takes `Flock::try_lock` and maps `WouldBlock` to "cannot take the journal append lock … refused".
  - `run_pass` still spawns one `run_chain` per feed (pullrun.rs:871). Each leg reaches `broker_answer` → `landed_answer` → `recorded_with_failures` (server.rs:10655-10690), which appends once per failed member, each with its own fsync.
  - No in-process mutex exists around `appended`.
- **server1-2 (Ctrl-C during a hand walk does not stop it; a second Ctrl-C does nothing): CONFIRMED in code.**
  - main.rs still passes `Box::pin(tokio::signal::ctrl_c())`.
  - The serve arm (server.rs:18541-18556) only awaits `serve(..)`, then `flying.abort()`.
  - `pull_spot` and `pull_fno` still run the whole walk inside the request.
  - Nothing on the shutdown path bumps `autopilot.epoch`, which `broker_run` checks (server.rs:7840).
  - Same caveat as the original: the axum drain behaviour is from its documentation, not its source.
- **runs-1 (Finisher and ticker write into the next run's progress): CONFIRMED.**
  - `let _finisher = Finisher {..}` is still the first local in `conduct_with` (pullrun.rs:929), so it drops after `groups`, `checkpoints`, `outcomes` and the ticker handle.
  - `Finisher::drop` (590-605) still checks only `finished.is_none()`.
  - `Progress` still has no generation.
  - `ticker.abort()` (1005) is still followed by the final write, with no join.
  - The "next run" can equally be a recovery. `recovery::claim` (recovery.rs:560-576) installs `Progress::claimed()` on the same `!running()` test. Its `drive` then writes `finished` without an identity check either (recovery.rs:832).
- **runs-3 / recovery-5 (`pull_run_stop` holds `site.run` across fsyncs): CONFIRMED, with one narrowing.**
  - server.rs:10981-10995 still calls `recovery_control::stop(&site)` while `held` is alive.
  - For a plain press, `recovery_active` is `None` and `stop` returns `Ok(())` with no I/O (recovery_control.rs:84-92). The fsync stall needs an active recovery, as both reports' repros already assume.
- **recovery-1 (a seat refusal is charged as a vendor attempt): CONFIRMED.** recovery.rs:1550 `append_attempt(..)?` still precedes `server::recovery_spot(site, &asked).await?` (1552), and `recovery_spot` still takes the seat on its first line (server.rs:8022-8025).
- **recovery-4 (the audit journal refuses instead of waiting between in-process writers): CONFIRMED.** Same code as server1-1. `recovery_spot` still appends with `journal.append(&record)?` after `broker_run` has landed bars.
- **autopilot-4 (Ctrl-C mid-tick leaves bars with no record): CONFIRMED.** `flying.abort()` is still at server.rs:18555. The receipt append still follows `broker_run` (autopilot.rs:3443, 3482).
- **hunt-api-1 (a hand pull cancelled by client disconnect loses its record): CONFIRMED.** `broker_run` (server.rs:7737-7948) still has no Drop guard between `note_run_started` and `note_run_finished`/`publish(now = None)`. The journal append is still in the caller after the await. A press leg is not exposed: it runs inside a spawned chain, not a connection future.
- **pull2-4 (exclusive try-lock on cash-session cache reads): CONFIRMED on the press path.** `prepare_cash_schedule` (server.rs:7422-7470) calls `prepare_local_observed` and then `prepare_observed` for every NSE cash minute body. Two equity feeds of one press run in parallel chains, so they reach the same day locks at the same time. That is the ordinary trigger, not an edge case.
- **server1 "Checked: seats" and autopilot "Checked: seat bitmask": CONFIRMED sound.**
  - `fetch_or` (AcqRel) with a lone-bit check. `compare_exchange(0, ALL)`.
  - `AllSeats::drop` uses `store(0)`. That is safe because no `Seat` can be live while the mask is ALL (`fetch_or` on a set bit returns `None`).
  - `Seat::drop` uses `fetch_and(!bit)`.
  - Every store writer in the api is under a seat: `pull_spot` (local and broker), `pull_fno`, `recovery_spot` and the autopilot round. `land_*`, `fno_land` and `run_local` are reachable only from those.
  - `Feed::store_vendor` is 1:1, so one seat stands for exactly one vendor census.
- **server1 "Checked: `pull_run` check-and-claim": CONFIRMED.** server.rs:10901-10914: the `running()` test and `*held = Some(Progress::claimed())` sit under one guard, before `tokio::spawn`.

## Checked and clean (no finding)

- **Lock order across the press.** `site.run` (std) is taken only inside closures (`with_progress`, `stopping`, `halted_feeds`) or short handler scopes, and never across an `.await`. The compiler would refuse it, because the futures are spawned `Send`. `site.run` → `recovery_active` is the only nesting. `site.budgets` → `governor` is released before the sleep in `await_budget`. The `Seat` is an atomic held across awaits and not a lock, so it has no ordering hazard.
- **Chain atomics.** `Checkpoints` and `attempted` are Relaxed and carry no other data. Every chain is joined (`chain.await`) before the conductor reads them, and the join gives the happens-before. `note_dead_chain` reads `attempted` after a failed join, which also synchronizes.
- **Panicking leg.** `Seat` and the `Flock` guards release on unwind. `run_pass` turns a `JoinError` into `note_dead_chain` plus `Retry`. The press slot is released by `Finisher`, with the runs-1 caveat.
- **Same-feed writers inside one api process.** Legs of one feed run sequentially in one chain. The seat refuses a concurrent hand pull on that feed with 409, which the press classifies as `Retry` (`leg_outcome`). The autopilot stands off on `site.run` and then CASes all seats. Its TOCTOU is bounded, as pass 1 noted. `pull::ingest::derive_all`'s read-minutes-then-append-derived gap, which store1 noted out of slice, is therefore not reachable inside the api: every writer of a vendor's months holds that vendor's seat.
- **The cli on the same store.** No cli path writes bars, manifests or `audit/pull.journal`. `grep` of `crates/cli/src` finds only `pull::{calendar,session,fold,vendor}` and `BarFile::open_existing`. Two kinds of contention remain, and neither is a finding:
  - A cli read and an api landing of the same month refuse each other promptly through the month lock: a shared try-lock against an exclusive try-lock. `cli::stored` names the lock, and the press retries the leg. A refused `derive_all` rung is re-derived from the whole minute month on the next pass's ingest of that month, so nothing is permanently short.
  - `cli fold-audit` run during a landing can transiently report a coarse rung behind its minutes, because the minute commit and the derived commits are separate locks. The module says a mismatch can mean a stale coarse file, the command only prints, and a rerun clears it.
- **`fno_walk` / `read_month_bars`.** These read the underlying's spot bars from the same vendor (`wire.store_vendor`), which is under the walk's own seat. No cross-feed read.
- **Feed parsing.** `pull_spot`/`pull_fno` take the seat from `param(body, "vendor")`, and `parse_spot`/`parse_fno` read the same first-match `param`. The seat and the written vendor therefore cannot diverge for a repeated `vendor=`.
- **`legs_from` envelope vs payload (filed as tests-docs-security).** The concurrency consequence: two groups whose payloads name the same feed become two parallel chains on one seat. They 409 each other and retry within `MAX_PASSES`. The seat still serializes their writes, so the cost is retries and mislabelled progress, not interleaved bytes.
- **`audit::Journal` torn tail.** `appended` refuses a length that is not a multiple of 256 under the lock, and never repairs it. This is documented and deliberate (module doc, 06-limits J-04). Records are 256-aligned and never cross a filesystem block, so an ENOSPC short write cannot tear one in practice.
- **Display-only cross-talk, not reported.** Concurrent `broker_run`s on different feeds share `autopilot.status.now` and `failures`, so the first to finish publishes `now = None` while another still walks. `Status::json` documents that `pull_active` "does not … claim to count every concurrent pull", and no decision reads `now`.

---

<!-- conc-pass2/recauto.md -->
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

---

<!-- conc-pass2/sweep.md -->
Verdict: 3 new findings at 331b05c (0 high, 2 medium, 1 low). Two are crash-window defects in the sweep chain, and the third is a lock-free reader. 13 pass-1 findings touching the slice were re-verified: all CONFIRMED, none refuted.

### conc-pass2 / sweep: one sweep end to end, browser POST to backtest JSON readers

Slice: `api/src/sweeprun.rs` (admit, claim_execution, TaskFinisher, run/descend/command, run_json, observe_elsewhere), `cli::run_durable`, `cli/src/execution_lease.rs`, `cli::operation_audit` (begin/enter/read), `range_over_for_attempt` → `sweep_rungs` → `one_rung` → `record_all_attempt` / `record_unadmitted` / `ensure_*`, `cli/src/{results,frontier,trades,result_set,sweep_evidence}.rs`, and the api readers `backtest.rs`, `frontierjson.rs`, `trades.rs`, `topjson.rs`, `detail.rs` (Cached, PARENTS, LEDGER). Method: source reading only. I did not run cargo.

Kernel fact used by sweep-2: Linux `generic_perform_write` (mm/filemap.c, the buffered-write path that ext4 and xfs use) checks `fatal_signal_pending()` before copying each page-sized chunk. On a pending signal it returns the bytes already copied. A SIGINT with the default action, a SIGKILL or the OOM killer, arriving during a `write(2)` whose buffer crosses a page boundary, therefore leaves the file extended to that boundary. The process dies before any user-space rollback runs. Neither `cli` nor `api` installs a SIGINT handler for sweeps: grep for `signal`/`SIGINT`/`ctrlc` over cli, store and telemetry finds none, and `cli/src/main.rs` has no handler. Ctrl-C on a `cli range-all` therefore terminates the process at an arbitrary instruction.

---

## sweep-1 (medium): a CLI sweep that is interrupted (Ctrl-C, kill, panic, OOM) leaves its `command started` marker as the newest one, and every later browser launch is refused, even though the execution lease is free

**Where**
- `crates/api/src/sweeprun.rs:1761-1775` (`claim_execution`):
  ```rust
  let lease = cli::execution_lease::Lease::acquire(&site.store_root).map_err(...)?;
  if let Some(dir) = crate::logs::cli_log_dir() {
      let observed = observe_elsewhere(&dir, now_micros() / 1_000);
      if !observed.launch_clear {
          return Err(Refusal::Unobservable(
              "the external command evidence still reports activity or is damaged/unconfirmed; inspect the execution-status note before starting another run".to_owned(),
  ```
- `sweeprun.rs:2433-2448` and `:2476`:
  ```rust
  ("command started", "running") if age <= STALE_AFTER_MILLIS && marker.run > 0 && named_sweep => ("running", ""),
  ("command finished", "completed") ... => ("completed", ""),
  ("command finished", "refused") ... => ("refused", ...),
  _ => ("unknown", "the external command has no usable terminal receipt or recent activity; silence is not completion"),
  ...
  launch_clear: matches!(status, "completed" | "refused"),
  ```
- The writer side is `crates/cli/src/lib.rs:2134-2156` (`run_with_sink`). It emits `cli.lifecycle "command started"` with phase `running`, then `dispatch`, then `command finished`. Nothing emits a terminal marker when the process dies inside `dispatch`.

**Why it is wrong.** The lease (`.sweep-execution-v1.lock`, flock) is the store's execution authority. The kernel drops it when its holder dies, so a free lease plus a `command started` marker whose `run` is a durable invocation id means the cooperating sweep is no longer alive. `claim_execution` has just acquired that lease, so it holds the proof. It still refuses because the telemetry marker has no terminal. After 15 minutes the marker's status becomes `unknown`, and `unknown` is never `launch_clear`. The lifecycle lock is effectively stale, and it is never detected or cleared. The only ways out are not in the refusal text:
- (a) run another CLI sweep verb to completion or refusal, which writes a newer `command finished`;
- (b) run about 128 non-sweep CLI commands, which push the marker out of the 256-record `cli.lifecycle` window;
- (c) wait for log rotation to drop the file.

The refusal tells the operator to "inspect the execution-status note", which repeats the same `unknown`. The CLI side does not consult this evidence (`run_durable` only takes the lease), so the block is one-sided: `cli range-all` still runs while the browser's Run, Descend and every `/engine/command` are refused. The background idle poll (`observed_status_with_admission`) also reports `available:false` with "External activity or damaged execution evidence remains unresolved".

This is the "lock file left behind after a crash that blocks forever" shape, except that the stale lock is a telemetry record rather than a lock file. Interrupting a long CLI sweep with Ctrl-C is the operator's ordinary way to stop it.

**Repro**
1. `cli range-all zerodha NIFTY 2024 1 2024 3` (with or without a support argument). `run_durable` takes the lease and `operation_audit::begin`. `run_with_sink` writes `{"target":"cli.lifecycle","message":"command started","command":"range-all","phase":"running","run":<id ≥ 2^63>}`.
2. Press Ctrl-C. No handler is installed, so SIGINT terminates the process. No `command finished` is written. The kernel releases the flock.
3. More than 15 minutes later (or at once, with `status=running`), press Run on `/backtest`. `admit` → `claim_execution` → `Lease::acquire` succeeds → `observe_elsewhere` finds the `command started` marker with age > `STALE_AFTER_MILLIS` → `unknown` → `launch_clear == false` → `503 {"accepted":false,"refusal":"the external command evidence still reports activity or is damaged/unconfirmed; ..."}`. The lease is dropped.
4. Every later Run, Descend or command gets the same refusal until one of (a)-(c) happens. A panic inside `dispatch` (unwinding past `run_with_sink`, with `Attempt::drop` writing `Failed` to the operation audit) leaves the same telemetry state.

**Minimal fix.** Inside `claim_execution`, the lease is held, so a `command started` marker whose `run` is in the durable `operation_audit` id range cannot belong to a live cooperating sweep. Look that id up with `operation_audit::read(root, run)`. If it has no terminal phase, record it durably as abandoned (for example `Phase::Cancelled`, with a note that the lease was found free) and treat the marker as `launch_clear`, with a loud telemetry event naming the abandoned id. Keep refusing for markers whose `run` is not a durable id: older binaries and bypassing callers, which the module doc says must stay visible. The same lookup should drive the idle-poll `admission.available` answer.

---

## sweep-2 (medium): a kill or power loss inside a result-chain append leaves a ragged tail that wedges every later recording in the store. For the three child files those bytes are provably uncommitted, yet nothing can remove them

**Where.** Each append below rolls back only when `write_all` returns `Err`:
- `cli/src/frontier.rs:1092-1109` (`append_locked_with`): one `write_all` of `rows.len() × 280` bytes into `results/frontier.bin`;
- `cli/src/trades.rs:647-672` (`append_locked`): one `write_all` of `rows.len() × 136` bytes into `results/chosen-trades.bin`;
- `cli/src/result_set.rs:472-482` (`append_locked`): 64 bytes into `results/detail-sets.bin`;
- `cli/src/results.rs:1278-1350` (`append_locked`): 261 bytes into `results/runs.bin`;
- `cli/src/sweep_evidence.rs:1383-1386` (`append_events`): `attempts.bin`, per-identity `starts.bin` and `<token>-lifecycle.bin`, with no rollback at all (cli2-1).

Each of these files refuses any ragged tail on every later open, read-only included, with no repair path:
- `frontier.rs:1858-1868` (`check_header`), reached by `Frontier::open` (:910) and `open_read_bounded` (:834): `"... bytes past its last whole row — an append was interrupted ... the remainder is left alone. Nothing was written."`
- `trades.rs:1185-1192` (`check_header`), reached by `Trades::open` (:510) and `open_read_bounded` (:453);
- `result_set.rs:515-517` / `:944` (`Receipts::open`, `open_read*`);
- `results.rs:951-966` (`open_with`): `"Nothing here is repaired automatically ... truncating them is a decision about history that belongs to you."`
- `sweep_evidence.rs:1195-1200` (`shape`).

**Why it is wrong.** The rollback handles ENOSPC and EIO, where `write_all` returns. It cannot handle the process dying inside `write(2)`. With page cache semantics, a fatal signal between page chunks of one buffered write leaves the file extended to the page boundary already copied (see the kernel note at the top). Power loss between `write` and `sync_all` can do the same through partially written-back extents. These strides do not divide 4096 (16-byte header, rows of 280, 136, 64, 261 and 96 bytes), so the boundary is generally mid-row. Frontier and trade blocks are often thousands of bytes, so the write spans several pages. The window is the length of the `write` syscall plus the following `sync_all`, at the very end of a multi-hour sweep, which is exactly when an operator who thinks the run is stuck presses Ctrl-C.

The consequence is not limited to one run:
- `record_all_attempt` → `record_frontier` → `ensure_frontier_rows` → `Frontier::open` refuses. Every later admitted or unadmitted recording, for every identity, in every process, prints `NOT_RECORDED`. `one_rung` turns that into an `Err`, so `range-all`, `pool` pass 1 and browser sweeps all record nothing.
- `/frontier.json`, `/trades.json`, `/engine/top.json` and `/backtest/frontier` reopen through `open_read_bounded` and refuse for all runs, the healthy committed ones included.

For the three child files the torn bytes are never history. `record_all_attempt` (lib.rs:20242-20357) holds `LEDGER` plus the `write.lock` flock, appends and syncs each child before the ledger row, and readers expose a child block only under a committed ledger row and receipt (`of_run` → `committed_receipt`). A partial child row past the last whole row therefore cannot belong to any committed result set. The repo's own rollback comment says so: "a record that was never completed was never a record" (results.rs:1328-1333). Refusing forever over such bytes turns a recoverable interruption into the manual repair the rollback exists to avoid ("the difference between a disk that filled up and a ledger that has to be repaired by hand"). No verb or tool performs that repair. For `attempts.bin`, cli2-1's proposed fix (roll back on `Err`) does not cover this kill path, so the global token journal wedges as well.

**Repro (one machine, no race)**
1. A store with committed runs. Start `cli range-rung zerodha NIFTY 5min 2020 1 2026 9` (or any sweep that ends in `record_all` with ≥ 31 chosen trades, more than one page of `chosen-trades.bin` rows).
2. Send SIGINT while it is inside `Trades::append_locked`'s `write_all`. Under `strace -e write -p <pid>`, the pause on the large write is visible. With strace attached, `kill -INT` on its entry reproduces this deterministically. The process exits. `chosen-trades.bin` now ends at a page boundary that is not `16 + k·136`. No ledger row was written, so nothing committed is affected.
3. Run any later sweep for any identity. `ensure_trade_rows` → `Trades::open` → `check_header` → `"… bytes past its last whole row — an append was interrupted"` → `NOT_RECORDED`. Every later run gets the same result.
4. `GET /trades.json?identity=<any committed run>` → `detail::TRADES` cold open → the same refusal. The healthy history is now unreadable through the API too.

**Minimal fix.** In the writer-side `open` of `frontier.bin`, `chosen-trades.bin` and `detail-sets.bin`, called only under `ResultSetLock` (and `record_unadmitted` must take it too, see cli2-4), treat a ragged tail past the last whole row as the trace of an interrupted, uncommitted preparation:
1. take the file's exclusive flock;
2. `set_len(HEADER + whole·STRIDE)`;
3. `sync_all`;
4. emit an error-level telemetry event naming the file, the cut byte range and the reason.

Read-only opens keep refusing until a writer heals the file. For `runs.bin` and the sweep-evidence journals, either do the same under their exclusive lock (a partial sealed record cannot be a record) or ship an explicit `cli repair-tail <file>` verb that the refusal names. Record the choice in `docs/05-decisions.md`, because it narrows the "refused rather than healed" rule on these files.

---

## sweep-3 (low): `/backtest.json` reads `runs.bin` with no shared lock, so a concurrent append is reported as an interrupted write

**Where**: `crates/api/src/backtest.rs:1014-1017` and `:1067-1143`:
```rust
match File::open(&path) { Ok(mut file) => read_from(path, &mut file, limit), ... }
...
let len = match src.seek(SeekFrom::End(0)) { ... };
...
// A RAGGED TAIL IS REPORTED, NOT REPAIRED AND NOT FATAL. `cli` appends one
// whole stride and flushes, so a partial tail means the writer was
// interrupted — a full disk, a kill.
let partial_tail = body % stride != 0;
```

**Why it is wrong.** `Results::read` takes `lock_shared` specifically so that a reader "waits for an in-progress append rather than see half of it", calling that kind of false alarm "its own defect" (results.rs:1464-1479). `Results::refresh` and the `detail::LEDGER` cache do the same. `/backtest.json`, the ledger page the operator leaves open, measures the length and reads with no lock. A 261-byte ledger record that straddles a page boundary (about 6% of record offsets, since 4096 mod 261 ≠ 0) is copied page chunk by page chunk, and `i_size` rises after the first chunk. A poll landing between the two chunks sees a length ending mid-record and serves `"partial_tail":true`, which the module documents to the page as "the writer was interrupted — a full disk, a kill", on a ledger that is whole a microsecond later. A poll during a writer's ENOSPC rollback (`set_len(at)`) can also compute `total` from the extended length and then hit `read_exact` EOF on the record being cut, which returns a refusal "record N could not be read".

**Repro**: keep `/backtest` open and polling while a browser or CLI sweep commits rungs. On the commit whose ledger offset `16 + 261·k` lies in `(4096·m − 261, 4096·m)`, a poll inside the `write(2)` returns `partial_tail:true`. The next poll returns `false`.

**Minimal fix**: take `file.lock_shared()` around the `seek(End)` and the record reads in `read`, then release by explicit `unlock`, as `Results::read` does. The handler also runs this synchronous I/O directly on the async worker. Moving it onto `detail::run` costs nothing extra once the lock can wait.

---

## Pass-1 verification (findings touching this slice)

| id | verdict | reason (from code at 331b05c) |
|---|---|---|
| runs-2 | CONFIRMED | `observed_status_with_admission` (sweeprun.rs:2235) calls `execution_lease::probe`, which calls `lock` = `Flock::try_lock` (exclusive, non-blocking; execution_lease.rs:66-71, 125-134). `Lease::acquire` maps `WouldBlock` to `Busy`. The `TaskFinisher::finish` sibling also holds: the slot is written (1255-1260) before `self.lease` drops at the end of `finish`. |
| runs-4 | CONFIRMED | Each POST runs inside `crate::detail::run` (1727, 1963, 2983), which holds a permit. `admit` then blocks on `ADMISSION.lock()` (1680) before the busy check (1683-1691). `claim_execution` → `observe_elsewhere` runs under it. |
| cli1-1 | CONFIRMED | operation_audit.rs:556-563 releases the index, then `create_new` and `write_synced` run as separate steps. `read` (:611-619) maps a 0-byte own file to "lost its indexed start". In this slice it also makes `/backtest/run.json?attempt=X` (`persisted_status` → `journal::read`) answer 503 for good if the API dies in that gap. |
| cli1-2 | CONFIRMED | `operation_audit::begin` takes `Flock::try_lock` on the index (operation_audit.rs:522). `run_durable` (lib.rs:2100-2113) has no retry and returns `FAILED`. |
| cli1-3 | CONFIRMED | Same code as runs-2 (a duplicate). |
| cli1-4 | CONFIRMED | `CURRENT` is a `thread_local!` (operation_audit.rs:464). `enter` sets it only on the calling thread (:438-441). `sweep_rungs` (lib.rs:15209-15233) runs `one_rung` on rayon workers. CLI `range_all` passes `attempt: None` (lib.rs:15168), so `progress.attempt.or_else(binding_attempt)` (lib.rs:13242) is `None` on a worker. A further consequence in this slice: `observe_elsewhere`'s activity tail is filtered by `marker.run` (sweeprun.rs:2411), so a healthy multi-hour CLI `range-all` is reported `unknown` 15 minutes after its start marker, and browser launches are refused meanwhile. The lease would refuse them anyway, but with the wrong reason. |
| cli2-1 | CONFIRMED (and incomplete) | `append_events` (sweep_evidence.rs:1383-1386) chains `write_all` and `barrier` with no `set_len` rollback. The proposed fix covers only a returned error; the kill path stays open (see sweep-2). |
| cli2-2 | CONFIRMED | `one_rung` falls through to `latest_for(vendor_word, underlying, rung, from, to, min_hits)` (lib.rs:13705). That is keyed on span fields, not identity, and the browser `run` path reaches it. |
| cli2-3 | CONFIRMED | `Frontier::open` (frontier.rs:893-914), `Frontier::open_read_bounded` (:824-838), `Trades::open` (trades.rs:492-514) and `Trades::open_read_bounded` (:443-457) measure and index with no flock. `Results::open_with` (results.rs:728-750) and `Receipts::open*` (`validation_lock`, result_set.rs:249-262) do take one. |
| cli2-4 | CONFIRMED | `record_unadmitted` (lib.rs:19064-19077) runs `record_frontier` → `ensure_detail_receipt` → `record_swept_run` with no `LEDGER`, no `ResultSetLock` and no `confirm_result_directory` before the ledger append. `ensure_run_record` syncs the directory only after `append` (17343-17346). |
| cli2-5 | not re-verified | candidate_trades is outside this slice. |
| engine-1 | CONFIRMED | engine/src/lib.rs:2381 maps a spawn failure to `Breach::Workers`. `continue_walk` sets `progress.halted = halt` and calls `sink.checkpoint` (:1665-1667). The halt is a property of the checkpoint, not of a durable identity term. |
| hunt-conc-8 | CONFIRMED (unchanged) | `SharedBy::these(rungs.len())` in `sweep_rungs` (lib.rs:15218). In the API it is serialized by `ADMISSION` plus the slot plus the lease. |
| apicache "detail::Cached clean" | CONFIRMED | `with_verified` evicts on a refresh error (detail.rs:618-622). PARENTS is refreshed before FRONTIER and TRADES (frontierjson.rs:128 then 166; trades.rs:111 then 152). The writer order is child, then receipt, then ledger, so a visible ledger row implies a visible child. |

## Checked and clean (not findings)

- **Browser admission vs two launches, and vs the CLI.** `ADMISSION` serializes in-process. The slot's busy check and its install happen under it. The lease is acquired inside `prepare`, travels in `TaskFinisher`, and is released only when the guard drops, after `audit.finish` and the slot write. A queued `spawn_blocking` closure dropped at shutdown still drops the armed guard. A panic in `conduct` unwinds through `Applied` (knobs cleared) and `TaskFinisher::drop` (audit `Failed`, slot refusal), and the lease is unlocked explicitly (D-0693). `main` returns through the runtime drop, which waits for running blocking tasks, so an API Ctrl-C does not cut a sweep's write mid-syscall.
- **Commit protocol order.** `record_all_attempt` does frontier → trades → receipt → `confirm_result_directory` → ledger, under `LEDGER` and `write.lock`. After a crash at any step, an exact rerun byte-compares the prepared blocks (`ensure_frontier_rows` / `ensure_trade_rows` / `append_exact`) and re-syncs (`confirm_durable`) before appending the marker. `same_run_answer` excludes only `finished_micros`. The cross-process lock order is `write.lock` → each child flock → the `WRITER` mutex → the `runs.bin` flock, and readers take each shared lock singly, so there is no cycle.
- **`results.rs`.** `append` holds the exclusive flock across absorb, duplicate check, write, rollback and sync. A sync failure does not truncate. `with_shared_writer` drops the cached handle on any error. Open validation takes the dup-fd flock. A same-length generation change costs one refused request or commit, after which the handle is reopened.
- **`sweep_evidence` begin and finish.** Tokens come from `attempts.bin` under the flock. Reservations use `create_new`. The order is journal → base barrier → reservation → identity dir → barrier → lifecycle → `starts.bin`. A crash at any step leaves either an unused token or a start that readers do not index. The superseded-attempt check prevents an older token from overwriting a newer start.
- **Read paths.** `/backtest/run.json?attempt=` never substitutes another attempt. A persisted browser record is reported as exact evidence with `in_flight:false`. The PARENTS → child snapshot is consistent with the writer order.

---

<!-- conc-pass2/rangeall.md -->
Verdict: 2 new findings (0 high, 0 medium, 2 low). Both are read-twice TOCTOUs against a concurrent pull. Nothing in the rayon/lock/ledger mechanics is new beyond pass 1, and all 6 pass-1 findings that touch this slice are CONFIRMED at 331b05c.

### conc-pass2 / rangeall: `cli range-all` / `sweep-all` / `pool` (commit 331b05c)

Slice: `crates/cli/src/lib.rs` `sweep_rungs` (15209), `one_rung` (13381), `affordable_min_hits` (13311), `latest_for` (15586), `record_all`/`record_all_attempt` (20204/20242), `record_unadmitted` (19064), `ensure_run_record` (17317), `SharedBy`/`SWEEPS_SHARING_THIS_MACHINE` (16495-16540), `ceiling_from_env`/`shared_out` (16726/16790); `crates/cli/src/pool.rs` (`run_under`, `union_of`, `price_all`, `fold`); `crates/cli/src/batch.rs` (`sweep_under`, `one`). I read the callers in `api/src/sweeprun.rs` (`conduct`, the spawn at 1910-1922, `apply_knobs`) and `run_durable` (lib.rs:2095). Method: source reading only. No cargo was run.

Skipped as already reported: hunt-conc-1 (durable writes inside rayon workers), hunt-conc-8 (`SharedBy` / pool pass 1), cli1-4, cli2-2, cli2-3, cli2-4 (each re-verified below), and the knobs poison fallback (hunt-conc-6).

---

## rangeall-1 (low): `one_rung` derives `min_hits` from one read of the span, then sweeps and records a second, independent read. A pull that lands between the two reads records a run whose threshold came from bars it did not sweep, and the report contradicts itself.

**Where**
- `crates/cli/src/lib.rs:13433` (one_rung, read #1):
  ```rust
  let mut span = match stored::load_span(&root, vendor, underlying, rung, from, to) {
  ...
  let bars = span.bars.len();
  let missing = span.missing.clone();
  ```
  `min_hits` is derived from `bars` (statistical floor, `min_hits_for(bars, ..)`, 13497-13502). When no support is named, it also comes from `affordable_min_hits(&column, &root, &span, digest)` (13582), which runs on a column built from read #1 and writes AutoSearch evidence under read #1's digest (13368).
- `lib.rs:13648`: `let text = audit_range_for_attempt(vendor_word, underlying, rung, from, to, min_hits, attempt);` passes only `min_hits`. No digest, bar count or month census goes with it.
- `lib.rs:6729` (audit_range_inner, read #2): `let mut span = stored::load_span(&root, vendor, underlying, rung, from, to)?;`. The sweep, the identity's `data_digest`, and the ledger row's `bars` / `months_found` all come from this read.
- `lib.rs:15423-15433` (range_over_inner) prints `MONTHS MISSING FROM THIS SPAN` from `row.missing`, which is read #1. The table row's `bars`/`min_hits`/`months` columns come from the ledger record via `latest_for`, which is read #2.

**Why it is wrong**

Nothing stops the store's bar files changing between the two reads:
- Pulls do not take the execution lease. Only sweep entry points do (`run_durable` lib.rs:2101; `execution_lease.rs` header: "cooperating CLI and HTTP sweep entry points").
- `BarFile::append` grows a month file in place, and the current month grows every trading day.
- `stored`'s shared `try_lock` is held only for the life of each read handle, so it cannot span both reads.

So the api autopilot, a hand `/pull/spot`, or a recovery fill can append to, or create, a month in the span after read #1 and before read #2. The run then records `min_hits = f(N1 bars)` beside `bars = N2` and `data_digest = D2`. That breaks three things:
- The range opening says "each rung's `min_hits` is derived from that rung's OWN bar count". That is false for this row.
- The run cannot be reproduced. Rerunning the identical command on the now-stable store derives `f(N2) != f(N1)`, which is a different identity and a second ledger row. The first row can never be regenerated by its own command (§3 rule 5).
- The affordability-probe evidence is filed under a digest D1 that no recorded run used.

**Repro (exact interleaving)**
1. P_cli: `cli range-all zerodha NIFTY 2025 1 2026 10 auto` on 2026-10-03, with 2026-10 not yet on disk for `1min`. The `1min` rung's `one_rung` read #1 records `missing = [2026-10]` and N1 bars, and derives `min_hits = M1` (named ppm: `M1 = N1·ppm/1e6`; auto: the probe over N1).
2. P_api: the autopilot's minute pull lands 2026-10 for NIFTY (`BarFile` create/append and sync, no lease).
3. P_cli: `audit_range_inner` read #2 finds 2026-10 and N2 > N1 bars. It sweeps at `M1` and appends a ledger row with `months_found = asked`, `bars = N2`, `min_hits = M1`, digest D2.
4. The table shows the row with all months found and N2 bars, and below it prints `MONTHS MISSING FROM THIS SPAN (1): 2026-10`. The `rung sweeping` event (13636-13646) carries `bars = N1`, while `rung finished` and the ledger carry N2.
5. Re-run step 1 with nothing pulling: `M2 = f(N2) != M1` gives a new identity and a new row, and the step-3 row is orphaned from any command that produces it.

**Minimal fix**

Read once. Hand the already-loaded span (or at least its `data_digest` and `missing`) from `one_rung` into the audit body: split `audit_range_inner` into a load half and a sweep half, and have `one_rung` call the sweep half with its own span. Failing that, carry read #1's digest into `audit_range_inner` and refuse with "the span changed between derivation and sweep; rerun" when read #2's digest differs.

---

## rangeall-2 (low): `pool` pass 2 re-reads every instrument's span and prices the union on it without checking that the bars are the ones pass 1 screened, so a concurrent pull silently mixes two snapshots in one pooled table.

**Where**
- `crates/cli/src/pool.rs:293-300`: pass 1 calls `crate::one_rung(...)`, which loads, sweeps and records each instrument under identity `I(D1)` (digest of read #1).
- `pool.rs:745-800` (`union_of`): reads `frontier.of_run(&record.identity)`, which holds the D1 candidates.
- `pool.rs:337-339` and `pool.rs:806-822` (`price_all`): `let mut span = stored::load_span(root, vendor, underlying, rung, from, to)?;` with fresh loads of the signal, `1min`, daily and exact-minute series, then `grid::evaluate_over` over that new read. No digest is compared with `record.identity`/`data_digest`.
- The pool's claim in `price_all`'s doc: "a cell here is the cell `range-rung` would show for that mask on that instrument". The module doc (pool.rs:92-96) says "Pass 1's per-instrument runs are the same runs `range-rung` makes".

**Why it is wrong**

Pass 1 over up to 210 instruments takes minutes to hours. Pass 2 starts only after the whole of pass 1 is collected. Pulls run during all of that time with no lease (see rangeall-1). For any instrument whose current-month file grew, or whose missing month arrived, between its pass-1 load and its pass-2 load:
- The per-symbol table (pass 1, D1) and the pooled cells (pass 2, D2) describe different bars.
- `fired`, `trades`, `net` and `dd_bound` in the pooled row include trades on bars that no recorded run covered.
- Nothing on the page says so, and the pooled table has no identity to check later (pool.rs:68-77 says deliberately so).

That is a silent mixed snapshot. A rerun of `pool` on the stable store prints different pooled bytes from the same recorded pass-1 identities.

**Repro**
1. `cli pool zerodha 15min 2025 1 2026 10` on a trading day, with the api autopilot running.
2. Pass 1 screens RELIANCE at 10:05 and records `I_R(D1)` with N1 bars.
3. At 10:20 the autopilot appends RELIANCE's 15min and 1min bars for the morning.
4. Pass 2 (`price_all` for RELIANCE) loads N2 > N1 bars and prices every union mask over them. The pooled row folds those cells. The per-symbol RELIANCE line above still shows pass 1's D1 numbers.

**Minimal fix**

Have pass 1 return the prepared span, or at least its digest, with each `Screened`. In `price_all`, recompute `stored_anchored_digest` after loading and refuse that instrument with "bars changed since pass 1" on a mismatch, listed like the existing `priced` refusals. Alternatively, price from the span pass 1 already held, which also saves a second full load per instrument.

---

## Pass-1 verification (findings that touch this slice)

- **hunt-conc-1 (durable writes inside rayon workers). CONFIRMED.** `batch.rs:356-360` still runs `wanted.par_iter().map(|held| one(root, held, min_hits, commit))`. Inside `one`, `crate::sweep_evidence::begin(` is at batch.rs:631 and `crate::record_swept_run(` at batch.rs:724. The comment "Nothing is shared and nothing is written" still stands at batch.rs:332. `sweep_rungs` (lib.rs:15218-15232) runs `one_rung`, which writes AutoSearch evidence (`affordable_min_hits`, lib.rs:13368) and the full audit commit in workers. `runs.bin` row order and attempt tokens follow completion order.
- **hunt-conc-8 (`SharedBy` divisor; pool pass 1 takes none). CONFIRMED.** lib.rs:16527-16540: a plain `store(count)` and `Drop` does `store(1)`. `SharedBy::these(` appears only at batch.rs:356 and lib.rs:15218. pool.rs:293-300 runs pass 1 with no guard, so every concurrent `one_rung` gets the undivided `whole_machine_ceiling()` and all support lanes (`shared_support_lanes` reads 1). The API overlap is still unreachable: `range_over_for_attempt` and `batch::sweep_all` are reached only from `conduct`/`command`, under the single in-flight admission, and `pool` has no HTTP route (grep of `api/src` finds no `cli::pool`).
- **cli1-4 (rung boundaries lost under `sweep_rungs`' rayon map). CONFIRMED.** `operation_audit.rs:464` `CURRENT` is a `thread_local!`, set only by `Attempt::enter` (438-442). `completed_boundary` (489-504) does nothing when the slot is `None`. Callers enter on a non-pool thread: CLI `run_durable` at lib.rs:2115, and the API at `sweeprun.rs:1918`, `spawn_blocking(move || guard.enter(|| conduct(..)))`. `par_iter` (lib.rs:15220) then runs each `one_rung` on a rayon worker, so `note_rung_finished` → `completed_boundary` (lib.rs:13249-13250) is a no-op, and `binding_attempt` (3108) falls back to `Sink::run`. It also applies to `pool` pass 1 (pool.rs:294, same `one_rung`, also under `run_durable`). `completed_boundaries` is display-only (`api/src/operation_audit.rs:205`), so the impact is audit/telemetry binding only, as reported.
- **cli2-2 (`latest_for` returns newest key match, not this run's row). CONFIRMED.** lib.rs:15602-15614 still keys only on feed, underlying, timeframe, span and `min_hits`. `one_rung` falls through to it at lib.rs:13708 on every non-refused, recorded outcome, and `union_of` (pool.rs:767) reads `of_run(&record.identity)` from that row. Within one `range-all`/`pool` invocation the key always differs (rung, or symbol), so the collision needs a rerun with different policy knobs or a second process, exactly as reported.
- **cli2-3 (unlocked length measurement in `Frontier::open` / `open_read_bounded`). CONFIRMED.** frontier.rs:893-896 and 824-827 run `metadata().len()` with no flock, then `check_header`. `record_unadmitted` (lib.rs:19064-19077) takes neither `LEDGER` nor `ResultSetLock`. In this slice it is worse than reported: `union_of` (pool.rs:752-761) opens the frontier ONCE. If that single open hits the race (an api `range-all` appending frontier rows in another process), every screened instrument is pushed to `unread` and the page prints `POOLED: nothing to pool` for the whole surface.
- **cli2-4 (`record_unadmitted` skips the directory barrier before the ledger marker). CONFIRMED.** lib.rs:19070-19072: `record_frontier` → `ensure_detail_receipt` → `record_swept_run`, with no `confirm_result_directory` between them. `ensure_run_record` syncs the directory only after `store.append` (lib.rs:17346-17348). `record_all_attempt` has the barrier at lib.rs:20314-20322.
- **pull2-2 (`record_all` drops census rows)**: not this `record_all`. That one is in the `pull` crate's census code, outside this slice. Not re-verified.

## Checked and found sound (not findings)

- **`SWEEPS_SHARING_THIS_MACHINE` ordering.** It is `Relaxed`, but it is stored on the caller thread before `par_iter`. Rayon's injector and latch synchronisation gives happens-before to every worker, and every reader (`ceiling_from_env`/`shared_out`, `shared_support_lanes`) runs inside the job. On a panic, rayon `join` waits for the sibling half before resuming the unwind, so the guard's `store(1)` cannot run while a rung is still building its ladder.
- **Identity under `SharedBy`.** In `sweep_rungs` the divisor is `rungs.len()`, a function of the request, so the divided `Params::ceiling` is deterministic per request. This is documented (D-0685, `range_rung_arm`). `batch::one` uses the constant `BATCH_CEILING`, so `min(len, threads)` affects support lanes only, and those are not an identity term (`Params::of`, runner/identity.rs:209-221).
- **Duplicate rungs.** `range_over_inner` does not dedupe, but its callers do: `range_rung_arm` passes one rung, `range_all` passes `EVERY_RUNG`, and the API parser collapses duplicates (`sweeprun.rs` test at 3749-3760). Two workers never sweep one identity in one run. Batch holdings map one-to-one to keys (`stored::misfiled` requires an exact spelling), and the pool surface is a sorted unique list.
- **Lock order.** `record_all_attempt` takes `LEDGER` → `results/write.lock` (blocking flock) → `with_shared_writer` mutex → `runs.bin` flock. `record_unadmitted` and `batch::one` take only the tail of that chain, so no path inverts it. `LEDGER` poisoning refuses loudly (lib.rs:20247-20251) rather than proceeding.
- **Output determinism.** `sweep_rungs`, both pool passes and batch use indexed `collect` with serial folds and renders. `union_of`'s `HashSet` is membership-only beside a first-seen `Vec`. `fold` sorts by a total key. No `HashMap` iteration reaches report bytes. The `priced` `HashMap` in `Unadmitted` is looked up by key only.
- **Thread-locals.** Apart from `operation_audit::CURRENT` (cli1-4), every `thread_local!` on these paths is `#[cfg(test)]` (`NONE_CLOSED`, `SCREEN_SPAN_LOADS`, the candidate_trades counters).
- **Process-global knobs.** These are read inside workers via `knobs::var` (an `RwLock`). The API sets them once before `range_over` under the single-run admission and clears them on drop (`Applied`, sweeprun.rs:1422-1462), so no worker sees a value change mid-run.
- **`Frontier::open`'s unlocked `write_fresh_header`** (frontier.rs:897-907, 1745-1773) can be run by two threads on a fresh file. Both write the same 16 bytes at offset 0, and rows are appended later under the flock at end-of-file, so the overlap is benign.

---

<!-- conc-pass2/ledgerv6.md -->
Verdict: 3 findings at 331b05c (0 high, 1 medium, 2 low). The per-rung commit chain is ordered correctly: every successor binds a synced predecessor, and a crash *between* stages is always recovered by an exact rerun. The new defects are at the orchestration level: retained input handles across all eight rungs, a long-held store-root Base reader, and parent directories that are never synced. Crashes *inside* a stage are the pass-1 wedges, re-verified below.

### conc-pass2 / ledgerv6: `step3_orchestrator.rs` plus the `ledger-v6` and `ledger-v6-replay` verbs

Method: source only. No cargo was run and nothing in the repository was edited. Files read: `crates/cli/src/step3_orchestrator.rs` (route at :2268-2381, family commit at :3007-3170 and :3891-4221), `ledger_v6.rs`, `strict_v6_inputs.rs`, `stored_family_v6.rs`, `audited_range.rs`, `checksum_receipts.rs`, `selection_v6.rs`, `global_replay_v4_store.rs`, `population_base_evidence_ledger_v2.rs`, `population_v6.rs`, and the retained-generation and append code of each stage ledger.

## Commit order (per run)

`run_route` (`ledger_v6.rs:289-397`) walks `LEDGER_RUNGS` one after another, with no threads. For each rung:

0. `strict::size_sweeper` loads the NIFTY span under strict guards. It writes only checksum receipts and bindings.
1. For each family, NIFTY then BANKNIFTY, under the **store root**, which `ledger-all` and every other ROOT share:
   - Candidate Universe
   - Base Evidence V2
   - Pre-Admission V1, then Pre-Admission V2. An extinct family writes V2 only.

   Each is receipt-last and reopened (`step3_orchestrator.rs:4023-4121`).
2. `commit_stored_population_v6_route` (`step3_orchestrator.rs:2268-2381`) writes the stages below, under `ROOT/<stage>/<rung>`, in this order. Each is reauthenticated before the next reads it.
   1. **Statistics V3** (:2295)
   2. Search Lineage V4 (:2308)
   3. **Admission V4** (:2336)
   4. **Finalization V4** (:2358)
   5. **Population V6** (:2372)
   6. **Execution V4** (:2376)
3. **Selection V6**, in `render_selection` → `commit_stored_selection_v6` (`ledger_v6.rs:587`) → `ROOT/selection/<rung>/global-selection-v6.bin`.

`ledger-v6-replay` repeats the whole route (`replay_route` → `run_route`, `ledger_v6.rs:690`). After all eight rungs it then writes Global Replay V4 under `ROOT/global-replay-v4/<publication>.bin`.

The task named the order Population V6 → Statistics V3 → …. That is not the code's order. Statistics V3 commits first and Population V6 fifth.

## What a crash between two stages leaves

| Process dies after… | On disk | Exact rerun (same binary, same inputs) | Rerun after a rebuild |
|---|---|---|---|
| family ledgers, before Statistics V3 | complete Candidate, Base and Pre-Admission blocks under the store root | every family ledger reuses its block, and Statistics V3 is written | the commit digest changes the identities, so new blocks are appended. The old blocks stay as complete, unreferenced history. |
| Statistics V3, before Lineage V4 | complete Statistics block | reused, and the route continues | new blocks |
| Lineage V4, before Admission V4 | complete Lineage block | reused | new blocks |
| Admission V4, before Finalization V4 | complete Admission block | reused | new blocks |
| Finalization V4, before Population V6 | complete Finalization block | reused | new blocks |
| Population V6, before Execution V4 | complete Population block | reused | new blocks |
| Execution V4, before Selection V6 | complete Execution block | reused, and Selection is written ("written") | new blocks |
| Selection V6 of rung *k*, before rung *k+1* | rungs 1..k complete | rungs 1..k print "reused", and the run continues at k+1 | new blocks |
| all 8 Selections, before Global Replay (replay verb) | 8 complete Selections | all reused, and replay is written | new blocks |

In every row the successor binds the predecessor's synced identity. No successor block can exist without its predecessor's Completion. The identities are deterministic (seeded bootstrap; no map iteration or clock reaches the bytes), so an exact rerun converges.

The unsafe cases are crashes **inside** one stage's append. These are pass-1 pop2-1/2/3/4/6, sel-1 and hunt-cli-a-5, re-verified below. A torn tail, or a whole-record orphan followed by a rebuild, wedges that rung's stage root for every later span.

## Two runs on one ROOT (and on two ROOTs)

- **Same span, same binary, concurrent.** Each stage's writer takes an exclusive flock. The second process finds the exact block and reuses it, without writing. No retained generation is disturbed, because reuse changes neither length nor mtime. This case is safe.
- **Different span, or a different binary, on the same ROOT.** This is pass-1 **pop2-7**. Confirmed, and its window is wider than reported (see the verification section below).
- **Different ROOTs, or `ledger-all` running beside `ledger-v6`.** These still share the store-root family ledgers. **ledgerv6-2** is a refusal that a per-ROOT lock (pop2-7's fix) does not prevent.

---

## ledgerv6-1 (medium): every rung's strict source handles and flocks are kept open until the run ends, so a multi-month `ledger-v6` runs out of file descriptors part-way and fails the same way on every rerun

**Where:**
- `ledger_v6.rs:307` and `:393`: `committed.push(selection)` keeps each rung's `CommittedStoredSelectionV6` until `run_route` returns. In replay, they are kept until Global Replay ends.
- `step3_orchestrator.rs:4242` (`bind_stored_population_v6`): `source.retain_strict_inputs(inputs)?;`
- `population_v6.rs:2606-2611`: the struct that holds those inputs.

The retention chain, with the code at each link:
- `selection_v6.rs:76`: `source: CommittedStoredExecutionV4`
- `execution_v4.rs:3513-3515`: `source: CommittedStoredPopulationV6`
- `population_v6.rs:3211-3213`: `upstream: PopulationV6ProductionSourceV1`
- `population_v6.rs:2609`: `strict_inputs: crate::step3_orchestrator::strict::Guards`, which holds `Arc<Inputs>`
- `strict_v6_inputs.rs:14-17`: `guard: RangeGuard`
- `audited_range.rs:36-41`: `sources: Vec<AdmittedMonth>, bindings: Vec<Receipt>`
- `checksum_receipts.rs:46-51`: `source: AuditedBarFile, receipt: Receipt`
- `store/src/file.rs:1444-1450`: the audited open keeps a shared-flocked `.lock` handle, the `bars` handle and the checksum handle. `Receipt` keeps one more shared-flocked handle (`checksum_receipts.rs:274`).

**Why it is wrong:** for one family on one rung with an M-month span, `Loader::load` (`audited_range.rs:190-233`) audits:
- the prior minute month and the prior daily month;
- per month, the minute, daily and (for rungs other than 1min) signal files.

Each audited month holds about 4 descriptors: lock, bars, crc and receipt. Each month also holds one span-binding `Receipt`. Two families are retained for each of the 8 rungs, so at the last rung the process holds roughly 2·[(8+9M) + 7·(8+13M)] = 128 + 200M descriptors. On top of that come the retained stage-ledger handles, which are several per stage per rung, plus the current rung's sizing guard. **This is an extrapolation from the code; I measured nothing.**

Rust std does not raise `RLIMIT_NOFILE`. The repo has no `setrlimit` (grep finds no `RLIMIT`). The common Linux soft limit is 1024, which this estimate passes at about M ≥ 5.

From that point the next `open` fails with `EMFILE`. That happens inside a late rung's family load or stage commit, after the earlier rungs have committed durably. The refusal reads "Too many open files" and gives no cause.

A rerun reuses the earlier rungs, but it reopens and keeps exactly the same handles. It therefore fails at the same rung every time, and `ledger-v6-replay`, which requires all eight Selections at once (`ledger_v6.rs:688-691`), can never run for that span. `docs/06-limits.md` does not mention the limit.

A second effect, cross-process, comes from the same retention. Every source month in the span stays shared-flocked for the whole run, which can take hours across 8 rungs. The store writer's non-blocking exclusive `try_lock` refuses for the whole run any pull, repair or backfill that touches those months.

**Repro (extrapolated):** `ulimit -n 1024; cli ledger-v6 V 2024 1 2024 6 SUPPORT PTS ROOT`. The estimate for M = 6 is 1,328 source descriptors at rung 8. Expected result: rungs 1..~6 commit, then `refused: v6 <rung> … Too many open files (os error 24)`. The identical command fails at the same rung again.

**Minimal fix:** once a rung's Selection V6 has been published and reauthenticated, stop carrying the strict guards forward. For example, `CommittedStoredSelectionV6` could keep only its identity, with `ledger-v6` dropping the rest, while replay reopens the guards it needs. Alternatively, raise the soft `RLIMIT_NOFILE` to the hard limit at CLI start and refuse up front with a computed descriptor budget when the span exceeds it. At minimum, record the per-month descriptor cost in `docs/06-limits.md`.

---

## ledgerv6-2 (low): the paired Base Evidence reader is opened at the start of the route and kept through Statistics V3 production; any Base append to the shared store-root ledger in that window refuses Admission V4

**Where:**
- `step3_orchestrator.rs:2280-2281`: `let (_base_evidence, mut base_reader) = family_v6::reopen_pair(&nifty_source, &banknifty_source)?;`
- The reader is first used at `:2319-2327` (`prepare_population_admission_v4(…, &mut base_reader, …)`), through `population_admission_v4.rs:679-680` (`base.candidate(candidate.sequence())`).
- The check sits in `population_base_evidence_ledger_v2.rs:1111-1122` (`with_shared_lock` → `require_unchanged`) and `:1258-1266`. These compare the root, lock, record and completion generations (length, inode, mtime, ctime) captured at open.

**Why it is wrong:**
- Between the open and the first read, the route runs three steps. Each can take a long time, and none of them uses the reader:
  1. the whole Statistics V3 production (1,000 bootstrap draws, `ledger_all.rs:1294`) and its commit;
  2. the Search Lineage V4 commit;
  3. the start of Admission preparation.
- The Base Evidence files `base-evidence-{records,completions}-v2.bin` live in the **store root**. Every `ledger-v6` (any ROOT), every `ledger-all` (`all_rung_population_v5.rs:584/608` → `commit_family_base`) and every family commit append to that root.
- `root_generation` also covers the store-root directory's own mtime and ctime. So even creating a new entry directly in the store root, such as a first-time lock or ledger file, trips the check.
- Any such append or create inside the window makes A's reader stale. A then refuses `Step 3 Population Admission V4 preparation refused: … base-evidence-records-v2.bin changed since open; cached Base authority is stale`, after A's Statistics V3 and Lineage V4 blocks have committed.
- pop2-7's proposed per-ROOT run lock does not prevent this, because the two runs need not share a ROOT.
- The binding `_base_evidence` is unused, so nothing requires opening the reader that early.

**Repro:**
1. Process A runs `cli ledger-v6 V 2024 1 2024 3 … ROOT_A`. It reaches rung 1min's `v6_statistics_adapter::produce` (`step3_orchestrator.rs:2292`).
2. Process B runs `cli ledger-all V 2024 4 2024 6 …` (or `cli ledger-v6 … ROOT_B` with another span). B commits a new family, so `commit_family_base` appends a Base block to the store root.
3. A finishes Statistics V3 and Lineage V4, then refuses at Admission V4 preparation with the stale-Base message.

A rerun of A recovers through exact reuse, so the harm is a spurious, misleading refusal. It reads as if the source data changed, and the run's work is wasted.

**Minimal fix:** call `family_v6::reopen_pair` immediately before `prepare_population_admission_v4`, after the Lineage commit, and drop the unused `_base_evidence`. Longer term, have the Base reader recheck only its own completed blocks' byte range plus the inode, not whole-file and directory mtimes.

---

## ledgerv6-3 (low): the stage, rung and global-replay directories are made with `create_dir_all` and never synced into their parents; each ledger syncs only its leaf

**Where:**
- `ledger_v6.rs:200-206` (`RungRoots::create`): `std::fs::create_dir_all(&path)` for `ROOT/<stage>/<rung>`, eight times per rung.
- `ledger_v6.rs:692-693`: `std::fs::create_dir_all(&root)` for `ROOT/global-replay-v4`.
- The ledgers sync only the directory they were given. Examples:
  - Admission V4 syncs its held root (pass-1 cites `:2583-2586`, `:2797`);
  - Selection V6 has `sync_directory(root)` (`selection_v6.rs:362-366`);
  - Global Replay V4 has `File::open(&root)…sync_all()` (`global_replay_v4_store.rs:67-69`).
- Nothing syncs `ROOT`, `ROOT/<stage>`, or the store root's ancestors.

**Why it is wrong:**
- POSIX makes a new directory entry durable only after its parent is fsynced.
- After a power loss, `ROOT/statistics` (or `ROOT/statistics/1min`) can be missing even though `ROOT/admission/1min` holds a synced Admission V4 block, and that block binds the Statistics V3 authority id. Whether this happens depends on the filesystem's metadata ordering; it is not something the code guarantees.
- Pass-1 noted this `create_dir_all` (concurrency.md:1543) but did not raise it. The same class is reported elsewhere as hunt-store-4 and xcut-3.

**Repro:**
1. Fresh ROOT.
2. `cli ledger-v6 …` commits rung 1min through Admission V4, which syncs `ROOT/admission/1min`.
3. Cut power before writeback commits `ROOT`'s entries.
4. On reboot, `ROOT/statistics` may be absent while `ROOT/admission/1min` holds a block that names a Statistics authority that no longer exists. Any audit of that rung refuses until the operator reruns. The rerun recreates the directory and re-appends identical bytes, which is why this is rated low.

**Minimal fix:** in `RungRoots::create` and `replay_route`, walk from ROOT down. Create each missing level with `create_dir`, then fsync its parent, in the style of `expression::attempt_directory` and the fix proposed for xcut-3.

---

## Pass-1 verification

- **pop2-7 (concurrent run invalidates retained Admission V4): CONFIRMED, and the window is wider than reported.**
  - `population_admission_v4.rs:2893-2908` compares the whole data-file generation, including `len`.
  - `ledger_v6.rs` takes no ROOT-level lock.
  - The same whole-file check exists for the retained Statistics V3 ledger (`population_statistics_v3.rs:2332-2333`, `:2375-2376`), Finalization V4 (`:2532-2544`), Population V6 (`:2277-2289`) and Execution V4 (`:2373-2380`, `:3162-3170`).
  - Execution V4's check is re-run on every Selection V6 read (`selection_v6.rs:141-150` → `execution_v4.rs:3609-3633`). In `ledger-v6-replay` that includes reads after all eight rungs, so a foreign append to any rung's stage files at any point in the multi-hour run makes the replay refuse. The whole run is the window, not only the Admission → Finalization gap.
  - The fix is unchanged. ledgerv6-2 is the cross-ROOT case this fix does not cover.
- **pop2-3 (sub-record tail wedges Admission/Finalization V4 for good): CONFIRMED.** `population_admission_v4.rs:2618-2625` refuses any non-stride length before any trailing logic runs. Population V6 has the same check (`population_v6.rs:2017-2019`), as pass-1 pop1 also reported.
- **pop2-4 (orphan prefix accepts only an exact retry that includes the build commit): CONFIRMED.**
  - `population_admission_v4.rs:2727` has `trailing.source != prepared.source`.
  - `BlockSourceV4.source_commit_digest` comes from `statistics.source_commit_digest()` (`:746`).
  - That digest comes from `hash(verified_commit.0.as_bytes())` (`step3_orchestrator.rs:3593`).
- **pop2-5 / pop2-6 (Statistics V3 has no root-directory fsync and no torn-header repair): CONFIRMED.**
  - `population_statistics_v3.rs` has no `root_file` or directory sync anywhere.
  - `:2827` initializes only when `len == 0`.
  - `:2845` refuses a short header.
- **sel-1 (Execution V4 `append_raw` has no rollback): CONFIRMED.** `execution_v4.rs:4931-4935` is exactly `file.seek(SeekFrom::End(0)).and_then(|_| file.write_all(raw))` with no `set_len`. It is reached from `step3_orchestrator.rs:2376-2378`.
- **sel "Step-3 root admission TOCTOU (acknowledged)": CONFIRMED as not a finding.** `AdmittedRootV1::require_same` is called before and after every family-ledger append (`step3_orchestrator.rs:4045`, `:4049`, and others).

## Checked and clean
- **Threads, atomics and locks in this slice:** none outside tests. The rung loop and the family loop are sequential.
- **Lock order across stages:** prepare (shared on the upstream ledger, then released) always happens before the downstream exclusive lock. No stage takes an upstream lock while it holds its own exclusive lock. Selection V6 derives its block before `persist` takes its lock (`selection_v6.rs:194-197`). No cross-process cycle exists.
- **Determinism:** the summary and report fields come from reauthenticated projections. No map iteration or wall clock reaches block bytes or identities in the route.
- **Global Replay V4 persist:** content-addressed by publication. A torn prefix of the same publication is completed. A different publication goes to a different file. It is fsynced before the seal and after it, and the leaf directory is synced. Its VIX lock collision is replay-5 in this pass's `replay.md` and is not repeated here.
- **Selection V6 concurrent identical runs:** writers are serialized by an exclusive flock and the exact block is reused. The partial-tail wedge is hunt-cli-a-5, already reported.

---

<!-- conc-pass2/ledgerall.md -->
Verdict: 3 findings (0 high, 2 medium, 1 low) in the `cli ledger-all` chain at 331b05c. The chain commits in a sound order inside one process, but it has three problems. An interrupted run can wedge the store-wide Candidate ledgers for every Step-3 verb. A successor check that can never pass runs after every ledger is already durably committed. And the support threshold is sized from a bar read that is taken earlier than, and separately from, the read that is actually swept.

### conc-pass2 / ledgerall: the `cli ledger-all` verb chain

Slice: `crates/cli/src/ledger_all.rs` (`run_chain`, `LedgerTree`, `build_sweepers`, `render_winners`), `all_rung_population_v5.rs`, `all_rung_selection_v5.rs`, and the ledgers they drive: Candidate, Base Evidence V2 and Pre-Admission V1/V2 under the store root; Observation V1/V2, Statistics V2, Search Lineage V4, Admission V3, Finalization V3 and Population V5 under `ROOT/authority/<rung>`; Execution V3 under `ROOT/execution/<rung>`; Selection V5 under `ROOT/selection/<rung>`. Source reading only; cargo was not run.

Reachability: the chain is live. `lib.rs:2275` `["ledger-all", ...] => ledger_all_arm(...)` → `ledger_all::ledger_all` → `run_chain` → `commit_all_rung_stored_population_v5` → `commit_all_rung_stored_execution_v3` → `commit_all_rung_stored_selection_v5`. `mod execution_v3;` (lib.rs:126) carries no `expect(dead_code)`.

---

## ledgerall-1 (medium): one interrupted Candidate, Base or Pre-Admission append in the shared STORE root wedges every later Step-3 run on that store (`ledger-all` and `ledger-v6`, any span) once any input changes

**Where**
- `crates/cli/src/all_rung_population_v5.rs:584-631`: phase one commits all 16 Candidate/Base/Pre-Admission families into `roots.source`, which is `crate::store_root()` (`ledger_all.rs:874`). It does not write them under the operator's ROOT.
- `crates/cli/src/candidate_universe.rs:3701-3709` (`append_complete_locked`):
  ```rust
  let (first_row, prefix) = match self.orphan {
      Some(orphan) => {
          if orphan.universe_id != receipt.universe_id() {
              return Err(format!(
                  "candidate row tail belongs to {}, not requested {}; no fallback may hide it", ...
  ```
- The same rule applies to the other store-root ledgers in the same transaction: `population_base_evidence_ledger_v2.rs:1212` ("physical Base tail belongs to another source"), and `pre_admission_data.rs:1328` and `:2550` ("trailing orphan belongs to …, not exact retry").
- `ledger_v6.rs:328` reaches the same files through `commit_strict_candidate_pre_admission_authority_v1` → `commit_family_with_inputs_v6`, the same function `ledger-all` uses (`step3_orchestrator.rs:3060`).

**Why it is wrong**

`append_rows` (candidate_universe.rs:6258+) writes the rows in chunks of `ROW_WRITE_CHUNK_ROWS` with one `write_all` per chunk. It then calls `sync_data` and only after that appends the receipt. A process that dies between two chunks, or between the row sync and the receipt, leaves a receipt-less whole-row orphan. `scan_orphan` accepts it, and from then on the writer accepts only a block whose `universe_id` equals the orphan's. That id is derived from the data digest, the feed, the **source commit**, the rung, the family and the span (`CandidateUniverseIdentitiesV1`, candidate_universe.rs:174-191; `derive_universe_id` :5504).

This repeats the class of pop2-4 and hunt-cli-a-5, but those reports cover per-rung roots. Three things make this case worse:
1. **Blast radius.** The ledger is one file per store, shared by every Step-3 run against that store. That covers every span and both verbs, and with them every ROOT an operator might use to keep runs apart. The usage text says ROOT holds these ledgers (lib.rs:543-550, "WRITES the ledgers -- candidate, pre-admission, … -- under ROOT"). They are written in the store.
2. **The trigger needs no rebuild.** If the TO month is the current month, the autopilot's appends change `data_digest`. The interrupted universe then cannot be prepared again by anyone, ever.
3. **There is no signal handling.** The cli installs no handler (no `ctrlc`/`signal_hook`/`sigaction` in crates/cli). An operator's Ctrl-C during a multi-hour `ledger-all` therefore lands wherever the process happens to be.

Universes that are already complete still reuse, because the `audits` lookup comes before the orphan check. Only new universes refuse. A Step-3 run whose span, build or data differs from the interrupted one can never commit again until someone truncates the store file by hand.

**Repro**
1. `cli ledger-all V 2025 1 2026 10 S P /x/ledgers` on 2026-10-03, during market hours. Press Ctrl-C while `append_rows` for 3min BANKNIFTY is between chunks.
2. The autopilot appends 2026-10 minutes.
3. Rerun the identical command. 1min NIFTY prepares a universe U' whose `data_digest` differs from the stored one. U' has no receipt, so it reaches the orphan branch and refuses: `all-rung 1min NIFTY refused: … candidate row tail belongs to <U>, not requested <U'>`.
4. `cli ledger-v6 V 2024 1 2024 12 S P /y` (another span, another ROOT) refuses the same way at its first universe not yet complete. So does every later run on that store.

**Minimal fix**

Under the exclusive writer lock, a receipt-less tail is provably unacknowledged. Choose one of these, and make the same change in the Base Evidence V2 and Pre-Admission V1/V2 writers:
- (a) Let the writer leave a fully valid foreign orphan in place as unreferenced evidence and start the new block after it (the Execution V1 EC-01 rule). `scan` then needs to index orphan ranges.
- (b) Let the writer truncate a foreign whole-row orphan to `committed`, `sync_all`, and emit a loud telemetry event that names the discarded universe.

At a minimum, the refusal must name the file and tell the operator the recovery step. The usage text at lib.rs:543 should also say these ledgers live in the store, not under ROOT.

---

## ledgerall-2 (medium): Selection V5 commits Top-*min(eligible, 25)* per rung, but the chain's own successor check requires exactly 25. The check runs after all three stages are durably committed, so every run where any rung admits fewer than 25 is reported as refused, and every rerun repeats it

**Where**
- `crates/cli/src/selection_v5.rs:634`, where the commit-time invariant is `self.selected_count != self.eligible_count.min(REQUESTED_TOP_U64)`, so fewer than 25 is a legal commit. `all_rung_selection_v5.rs:696-707` (`require_rung_receipt_proof`) also accepts `selected_count <= 25`.
- `crates/cli/src/all_rung_selection_v5.rs:362-372` (`preflight_successor_rung`):
  ```rust
  let winners = selection.successor_winners()?;
  if winners.len() != MAX_WINNERS_USIZE {
      return Err(format!(
          "all-rung {rung_name} successor requires exactly 25 winners, observed {}", ...
  ```
- `crates/cli/src/ledger_all.rs:917-933`. Here `stage_finished_event(.., SELECTION_STAGE, ..)` is emitted and "BLOCKS WRITTEN …" is printed. Then `render_winners(out, selection)?` → `selection.into_successor_set()?.visit_canonical(..)` (`:965`) runs the preflight.

**Why it is wrong**

The ordering is commit first, check second, and the second check is stricter than the first. `successor_winners()` returns `top_twenty_five()`, whose length is `selected_count = min(eligible, 25)`. On any span where one rung has fewer than 25 eligible (admitted) candidates, the following happens:
- Population V5, Execution V3 and Selection V5 for all eight rungs are already synced and their directories fsynced.
- The telemetry says the Selection stage finished.
- Then `ledger_all` prints `refused: all-rung Xmin successor requires exactly 25 winners, observed N`, emits `run_refused`, and the arm exits `MISUSED`.

None of the eight rungs' winners are shown, including the rungs that had 25. A rerun reuses every block byte for byte and refuses identically, forever, for that span. A durable commit reported as a failure is the success/failure confusion that CLAUDE.md §4 bans, here in the reverse direction.

With 39 admission gates, fewer than 25 admitted candidates on a rung is an ordinary outcome. The tightest and loosest rungs (1min/60min) are the likely cases.

**Repro**

Run `cli ledger-all …` on a span where 60min admits 7 candidates. All stages commit: "Population V5 8 Execution V3 8 Selection V5 8" is printed. The next line is `refused: all-rung 60min successor requires exactly 25 winners, observed 7`, and the exit code is non-zero. The rerun prints "Selection V5 0" written and then the same refusal.

**Minimal fix**

Make the successor's arity agree with the committed semantics. Either have `preflight_successor_rung` accept `winners.len() == receipt.selected_count() as usize` (≤ 25), or, if the Global Replay successor really needs 25, refuse inside `commit_selection_rung` *before* `commit_stored_selection_v5` writes. Either way, have `render_winners` render what was committed rather than gate the report on the replay successor's arity.

---

## ledgerall-3 (low): the support threshold is sized from one read of the store, and the bars swept come from a second, later read. A concurrent pull between the two makes the recorded run unreproducible from its recorded inputs

**Where**
- `crates/cli/src/ledger_all.rs:1400-1420` (`build_sweepers`): `let span = crate::stored::load_span(root, vendor, "NIFTY", rung, request.from, request.to)` → `let min_hits = crate::min_hits_for(span.bars.len(), request.support_ppm)` → `Sweeper::new(ladder)`, for all eight rungs, before any commit (`run_chain`, `:877`).
- `crates/cli/src/step3_orchestrator.rs:3068`: each family later reloads its bars with `load_bounded_stored_context_v1(&request, &root, config)`. This happens hours later for the 60min rung, because phase one sweeps 16 families in sequence.
- `step3_orchestrator.rs:2813`: `runner::identity::Params::of(self.sweeper_ladder)` folds the stale `min_hits` into identity. The candidate's `data_digest` (candidate_universe.rs:176) is taken from the *second* read.

**Why it is wrong**

Bar files for the current month are appended in place by the pull and autopilot path, and `ledger-all` holds no lock on the store. If minutes for the TO month land between T0 (sizing) and T1 (the family's own load), the sweep runs with `min_hits = floor(N0·ppm/1e6)` over N1 > N0 bars. The durable Candidate block then records {data_digest(N1), ladder(min_hits from N0)}, and no single read of the store produces that pair. Rerunning the same command on the same, unchanged store computes min_hits from N1, so it gets a different ladder and a different `universe_id`. It appends a second universe instead of reusing the first. This breaks §3 rule 5 (same inputs, same bytes) and §3 rule 3 (identity names what was computed). It also triggers ledgerall-1's wedge if that second attempt is interrupted.

`min_hits` changes whenever N crosses a multiple of `1e6/ppm`. At 1000 ppm on the 1min rung that is every 1,000 bars, under three trading days, so a run during market hours can cross it.

**Repro**

Start `cli ledger-all V 2026 1 2026 10 1000 P R` at 10:00 IST. `build_sweepers` sizes 1min at N0 = 69,999 bars, so `min_hits = 69`. The autopilot appends 1 minute. The 1min NIFTY family loads N1 = 70,000 bars, sweeps at 69, and commits. Rerun after 15:30 on a frozen store: `min_hits = 70`, a different universe, so the blocks are written again rather than reused, and the two runs disagree on the frontier.

**Minimal fix**

Derive `min_hits` from the bars the family actually loads, inside the commit and after `load_bounded_stored_context_v1`. Alternatively, pass the sized `span` (or its data digest) into the commit and refuse if the reloaded data digest differs. `ledger_v6` sizes each rung just before its commit (`size_sweeper`), which narrows the window but has the same two-read shape.

---

## Pass-1 verification (findings that touch this chain)

| pass-1 id | verdict | reason (from code at 331b05c) |
|---|---|---|
| pop2-1 Admission V3 mid-block crash wedge | CONFIRMED | `population_admission_v3.rs:3987-3989` writes one `append_raw` per 2,048-byte decision. `:3699-3720` indexes a whole-record proper prefix as `trailing`. `complete_trailing` (`:4071-4077`) demands `trailing.decisions == prepared.decisions`, so even the exact retry refuses. |
| pop2-2 Finalization V3 / Statistics V2 no write-error rollback | CONFIRMED in code. Finalization V3 is practically unreachable. | Both still use plain `seek(End)+write_all` (`population_finalization_v3.rs:2857-2861`, `population_statistics_v2.rs:5479-5486`). Finalization V3 files have no header, and its strides are 2,048/4,096 at aligned offsets, so each record lies inside one 4 KiB page/block. ENOSPC/EDQUOT on a 4 KiB-block filesystem then allocates the block whole or fails with 0 bytes, which leaves no ragged tail. That case is reachable only on block sizes below 2 KiB. Statistics V2 is fully confirmed: its 64-byte header (`:61-69`) shifts the 1,024-byte records, so every fourth one straddles a block. |
| pop2-3 SIGKILL mid-record leaves a ragged tail | REFUTED for Admission V3 and Finalization V3. CONFIRMED for Statistics V2. | One `write(2)` of ≤ 4,096 bytes that does not cross a page boundary is copied in a single `generic_perform_write` iteration. The fatal-signal check sits between page iterations, so it cannot split the write. Admission V3 (decisions 2,048, completions 4,096) and Finalization V3 (rows 2,048, completions 4,096) are headerless and stride-aligned. Statistics V2 has the 64-byte header (`record_count`, `:5435-5445`), so a torn straddling record is real. |
| pop2-4 receipt-less trailing prefix only accepts the exact retry, and the identity includes the build commit | CONFIRMED | Admission V3 `:4071`. Statistics V2 `resume_orphan` (`:4610-4616`, "trailing orphan belongs to …, not exact retry"). In `ledger-all` the same rule also holds in Execution V3 (`require_exact_prefix`, `execution_v3.rs:2325-2357`) and Selection V5 (`selection_v5.rs:2168`, "orphan tail is not the exact canonical retry prefix"). ledgerall-1 shows that a change in the data alone is a sufficient trigger and that the store-root ledgers carry the same rule. |
| pop2-5 Statistics V2 never fsyncs its directory | CONFIRMED | The only syncs in production code are file `sync_all`s (`:4582`, `:4591`, `:4636`, `:4645`, `:5404`). There is no root `File` and no directory sync. |
| pop2-6 Statistics V2 cannot recover a torn header (1-63 bytes) | REFUTED as stated | `ensure_header` (`:5395-5408`) writes the 64-byte header as one `write_all` at offset 0 of an empty file. That single-page write cannot be split by a signal, and `i_size` goes from 0 to 64 in one step, so a file of 1-63 bytes is not produced by kill. A power loss on a filesystem without data ordering could leave 64 bytes with stale contents. `verify_header` would refuse that as a bad seal, but it is a different mechanism and pass-1 did not show it. |
| pop2-7 nothing serialises runs on one ROOT; a concurrent append invalidates retained authorities | CONFIRMED for ledger-all | `ledger_all.rs` takes no lock or lease in production code. Execution V3's retained generation is a double full-content hash plus mtime and inode (`execution_v3.rs:3883-3960`), and it is re-checked by `require_live_topology` before Selection. |
| pop1-2 Population V5 short write leaves a ragged tail | REFUTED for V5 (the V6 and Observation parts are outside this slice) | `population_v5.rs` is headerless, with rows of 4,096 and completions of 1,024 (`:65-67`) at aligned offsets. The same alignment argument as pop2-2 and pop2-3 applies, so the tear cannot happen by ENOSPC on a 4 KiB block or by SIGKILL. |
| pop1-3 Observation authority ledgers never fsync their directory | CONFIRMED | `population_observations_v1.rs` production code has only file `sync_data` (`:2180`, `:2281`, `:2304`, `:3361`, `:3435`, `:3445`) and no directory sync. In `ledger-all` these files live in `ROOT/authority/<rung>`. |
| search-2 Pre-Admission V1/V2 append has no rollback | CONFIRMED (store root, written by ledger-all phase one) | `pre_admission_data.rs:3595-3599` uses `seek(End)+write_all` with no `set_len` anywhere in the file. The records are 740 and 812 bytes, so they straddle blocks. |
| sel-1, its reachability statement ("`execution_v3`, `selection_v5`, `all_rung_*_v5` are `expect(dead_code)`") | REFUTED | `mod execution_v3;` (lib.rs:126) has no attribute. All three are reached in production through `lib.rs:2275` → `ledger_all_arm` → `run_chain`. The `expect(dead_code)` on `all_rung_*_v5` is satisfied by other items in those modules, not by the commit doors. The Execution V3 copy of sel-1's hazard is still not reachable, for alignment reasons: headerless, strides 1,024/128 (`execution_v3.rs:66-72`). So no new finding follows from the refutation. Selection V5 has `append_with_rollback` (`:3028-3055`), as sel noted. |

## Checked and clean (no finding)

- **Commit order inside one run.** All eight rungs' store-root families are frozen before any successor is retained (phase one). Each rung then commits Observation → Statistics → Search V4 → Admission V3 → Finalization V3 → Population V5 into its own disjoint directory (`all_rung_population_v5.rs:640-688`). After that come Execution V3 for each rung (`:772-843`) and Selection V5 for each rung. Each writer syncs data before its Completion and syncs the Completion before the directory (for example `execution_v3.rs:2224-2268`, `:2438-2460`).
- **Rerun after a crash between ledgers**, with unchanged inputs: every committed block is reused through the `receipts`/`audits` lookup, which runs before any trailing logic. A whole-record trailing prefix of the same block completes in Execution V3, Finalization V3 and Candidate. Admission V3 is the exception (pop2-1).
- **Cross-ledger binding.** Execution V3 joins on `population_id` and the Population receipt (`all_rung_population_v5.rs:859-894`, `:1013-1053`). Selection V5 joins on the Execution completion id (`all_rung_selection_v5.rs:657-665`). Each retained authority re-derives its source after it persists (`execution_v3.rs:2997-3018`). I found no path where a downstream block can bind a divergent upstream digest without a refusal.
- **Retained store-root authorities.** `CommittedCandidatePreAdmissionV1` keeps only audits (step3_orchestrator.rs:219-225). Successor reads reopen under a shared flock and check the generation before and after, so a concurrent ledger-v6 append to the store-root ledgers does not invalidate a running ledger-all's retained Candidate authority.
- **Locks held across the run.** Observation and Statistics V2 readers release their flock when open returns (`population_observations_v1.rs:2383-2395`, `population_statistics_v2.rs:2192-2230`). Execution V3 and Candidate lock only for the duration of open and append. No lock is held across another ledger's lock, so no deadlock is possible between two ledger-all processes, or between ledger-all and ledger-v6.
- **ledger-all and ledger-v6 on the same ROOT.** Both use `ROOT/execution/<rung>` and `ROOT/selection/<rung>`, but the file names are disjoint (`*-v3.bin`/`execution-v3.lock` against `*-v4.bin`; `global-selection-*-v5` against V6), so neither corrupts the other.
- **Nondeterminism.** There is no rayon, thread or wall-clock value in the production code of the chain modules. HashMap/HashSet are used only for keyed lookup and duplicate checks (`execution_v3.rs:1928`, `population_v5.rs:1983`, `all_rung_selection_v5.rs:810-847`). Bootstrap draws use the fixed `BOOTSTRAP_SEED`.
- **`LedgerTree::create`** uses `create_dir_all` without a parent fsync. Each ledger fsyncs its own leaf when it creates files. A lost parent entry after power loss costs recomputation only, never a wedge, so I did not report it (the class is xcut-3).

Outside this angle (not counted): `AdmittedDirectoryV1::admit` requires the exact canonical spelling (all_rung_population_v5.rs:1255-1266), and `ledger_all` passes `request.root` unchanged. A relative or symlinked ROOT therefore refuses after `LedgerTree::create` has already made the 24 directories.

---

<!-- conc-pass2/indexstop.md -->
Verdict: 2 new findings at 331b05c in the index-stop slice (0 high, 0 medium, 2 low). Pass-1 findings that touch the slice are re-verified below: 3 confirmed (two of them have triggers or fix gaps pass 1 missed) and 1 sub-claim refuted.

### conc-pass2 / indexstop: the index-stop search, from launch to the readers that read while it writes

Slice:
- api: `indexstoplaunch.rs` and `indexstoplaunch_metadata.rs`, plus the `sweeprun.rs` admission, lease and `spawn_blocking` wiring (2990-3160).
- cli: `index_stop.rs`, `index_stop_launch.rs`, `index_stop_search.rs`, `index_stop_search_checkpoint.rs`, `index_stop_search_progress.rs`, `index_stop_search_reader.rs`, `index_stop_source_context.rs`, `index_stop_vix.rs`, `index_stop_qualification.rs`, and the `index_consistency_store::produce` call.
- Shared helpers I read where the slice calls them: `search_checkpoint.rs` (Journal and Snapshot), `boolean_candidate_persistence.rs` (`prepare_in_namespace` and `write_or_equal`), `boolean_observation_file.rs` (Observation and ReadLease), `sweep_evidence::begin_group` and `append_events`, `vix_reference.rs`, and `store::file::BarFile::open_existing`.
- api readers: `indexstopjson.rs`, `indexstopcandlesjson.rs`, `indexstopqualificationjson.rs`, `indexstoprankingjson.rs` and `indexstopvixjson.rs`.

Method: source reading only. I ran no cargo and edited nothing.

---

## indexstop-1 (low): VIX companion bytes depend on whether the VIX store happened to be locked, or not yet pulled, at capture time, but the companion is keyed only by the catalog and kept forever

**Where:**
- crates/cli/src/index_stop_vix.rs:530-559 (`load_month`), which turns every open error into a saved "unavailable" month:
  ```rust
  match VixReferenceMonth::open(store, feed, month) {
      Ok(loaded) => Ok((Month { ..., records: Some(loaded.records()), ... }, Some(loaded))),
      Err(reason) => {
          ...
          Ok((Month { ..., records: None, snapshot_digest: None, unavailable_reason: Some(reason) }, None))
  ```
- crates/cli/src/vix_reference.rs:122 (`BarFile::open_existing`). Its doc at :78-80 says it "Refuses a missing, locked, torn, malformed, or otherwise unreadable store file".
- The lock in question is a non-blocking shared lock: `store/src/file.rs:1401-1404` `Flock::try_lock_shared(handle, ...)` returns `StoreError::Locked` when any writer holds the month (`file.rs:3272`).
- The companion's key, index_stop_vix.rs:707-713:
  ```rust
  pub(crate) fn lookup_identity(catalog: [u8; 32], pin: [u8; 32]) -> [u8; 32] { ... catalog ... pin ... }
  ```
- First writer wins, index_stop_vix.rs:386-395. Once the directory exists, every later `publish` returns the saved companion.

**Why it is wrong:**
- The companion body is a function of the catalog and of the VIX store's state at that moment. Its address is a function of the catalog alone.
- A transient condition is captured and published as permanent evidence. The clearest case is a pull writer holding month M of `NSE-INDIAVIX/1min`, which yields `StoreError::Locked`.
- Pulls do not take the execution lease. `claim_execution` exists only in `sweeprun.rs`, so an api Pull press or autopilot run can append VIX month M while the index-stop worker captures it.
- No later run can correct the companion. The catalog identity is deterministic, so `publish` takes the `Ok(_)` shortcut and returns the frozen "reference_month_refused" for every trade in M. This holds for that catalog and pin for good.
- This is the engine-1 pattern (an environmental refusal made durable under an identity that does not name the environment), on a reference-only artifact. It also breaks "same inputs, same outputs, byte for byte" (§3 rule 5) for these bytes: two otherwise identical runs differ only in scheduling.
- A month that has simply not been pulled yet is frozen the same way. The code comment (index_stop.rs:409-411) presents that case as a visible snapshot, so I note it but do not count it.
- Second consequence: because the body is not a function of the address, a retry after an interrupted publication can capture different bytes once a pull has changed the VIX store. That retry fails `write_or_equal`'s equality check. So the minimal fixes proposed for cli3-1 and search-1 do not, alone, make this namespace recoverable (see the verification section).

**Repro (two api threads):**
1. Launch an index-stop search whose training window covers month M. Batch N starts.
2. During batch N, press Pull for a range including M that writes `NSE-INDIAVIX` month M. The ingest `BarFile` writer holds the exclusive month lock while it appends.
3. Inside that window, one lane reaches `publish_vix` → `capture` → `load_month(M)` → `BarFile::open_existing`. `try_lock_shared` gets WouldBlock, `Err("... could not be opened as reference evidence: ...locked...")` is returned, and it is saved as `unavailable_reason`.
4. `prepare_in_namespace`/`finish` publish it, and the batch completes normally.
5. Every later `/index-stop-vix.json` for every trade in M shows `reference_month_refused`, although the month is complete on disk. Rerunning the identical search reuses the same catalog identity and pin and returns the frozen companion.

**Minimal fix:** In `load_month`, treat a lock refusal as a failure, not as data: return `Err` so that publication refuses and the batch stays pending, and the retry recaptures. Keep only a genuinely absent or invalid month as a saved "unavailable" reason. Better still, fold a digest of the captured months' state (records count plus snapshot digest, or "absent") into the address, or verify on reuse, so that a changed VIX store is a new companion rather than a byte conflict. Add a test that holds the VIX month lock during `publish` and expects a refusal, not a saved "unavailable".

---

## indexstop-2 (low): browser read leases make the search writer's non-blocking exclusive `try_lock` refuse, and for VIX a refusal at the right moment becomes the permanent cli3-1 wedge with no crash needed

**Where:**
- Writer side: crates/cli/src/boolean_candidate_persistence.rs:101-118 (`prepare_in_namespace`):
  ```rust
  let owner = Flock::try_lock(
      crate::readonly_file::open(&owner_path).map_err(display)?,
      owner_path.clone(),
  )
  .map_err(|why| format!("Boolean candidate namespace already owned or lock refused: {why}"))?;
  ```
  This runs on every launch for every source context (index_stop_search.rs:322-335 → index_stop_source_context.rs:321). It also runs for every catalog, relation, VIX, qualification and consistency child.
- Reader side: a shared lease on the same `owner.lock`:
  - `boolean_observation_file.rs:228` `Flock::try_lock_shared(&observation.owner, ...)` is held across a whole projection.
  - `Observation::open` (:46) takes `read_held(&owner_path, 0)` shared before it even looks for `complete.bin`.
  - The candles reader holds catalog, relation and source-context leases at once (index_stop_source_context.rs:701-712). It runs from `/index-stop-candles.json` on api detail threads.
- Search journal:
  - The reader probe is search_checkpoint.rs:108 `owner.try_lock_shared()`, reached from `Snapshot::open` through `Reader::latest_checkpoint` on every unpinned `/index-stop-ranking.json`.
  - The writer is `Journal::open` :153-157 `Flock::try_lock(...)` → "this exact search is already owned or cannot be locked". It runs only after `load_sources` and `prepare_sources` have finished (index_stop_search.rs:176-191).

**Why it is wrong:**
- flock shared and exclusive locks conflict across open file descriptions, including within one process. The writer uses `try_lock` and never waits.
- A browser read of saved evidence that overlaps a writer step on the same identity therefore turns the write into a refusal. The refusal message calls the namespace "already owned", which names no owner.
- Case (a), source context:
  - Continuing a search re-publishes the same content-addressed source contexts the candles page is reading. The identity is `context_identity(body)` and the sources are unchanged.
  - The refusal lands in `prepare_sources`, so the whole invocation fails after all selected training and later sources were loaded and decoded.
  - The rung is reported `Refused` with the misleading owner message.
- Case (b), journal: a ranking request's probe that straddles `Journal::open` refuses the launch at the same late point. The window is two syscalls, so this is rare.
- Case (c), VIX, which is the one that persists:
  - `prepare_in_namespace` creates the directory and `owner.lock`, and `sync_all`s it, before it calls `try_lock`.
  - If a `/index-stop-vix.json` request for that catalog and pin lands in that window, its `Observation::open` holds `owner.lock` shared until it fails on the missing `complete.bin`. The writer's `try_lock` then refuses.
  - The catalog is complete before VIX publication, and `/index-stop.json?identity=` returns the pin unpinned, so the request is possible.
  - The refusal leaves `index-stop-vix-reference-v1/<lookup>/` holding only `owner.lock`. Every retry then takes `publish`'s `Ok(_)` directory shortcut (index_stop_vix.rs:387) into `Reader::open` and refuses for good.
  - This is the cli3-1 wedge reached by a live, ordinary race rather than a crash. The window is the owner fsync plus a few syscalls, so it is narrow, but the result is permanent.

**Repro (case a, the likely one):**
1. Search S has saved batches. The operator opens a saved trade's candles. `indexstopcandlesjson::render` → `reader.with_current(...)` holds a shared lease on `index-stop-source-context-v1/<C>/owner.lock`.
2. In parallel, the operator presses Run to continue S. The worker loads every source (minutes), then `prepare_sources` → `source_context::publish` → `prepare_in_namespace(..., C, ...)` → `Flock::try_lock` → WouldBlock.
3. The invocation returns "Boolean candidate namespace already owned or lock refused: ...". The status marks the rung Refused, and the loaded sources are discarded. A retry after the read finishes succeeds.

**Repro (case c, permanent):**
1. Batch N's lane publishes catalog K (pin P) and enters `publish_vix`. `symlink_metadata` gives NotFound. `prepare_in_namespace` makes the directory, creates `owner.lock` and is inside `file.sync_all()`.
2. An api detail thread serving `/index-stop-vix.json?identity=K&pin=P` runs `Reader::open` → `Catalog::open` (succeeds, K is complete) → `Observation::open(..., lookup, ...)` → `read_held(owner.lock)` → `try_lock_shared` succeeds.
3. The writer's `Flock::try_lock` gets WouldBlock and `publish_vix` errors. The reader then fails on `complete.bin` and releases.
4. The batch stays pending. Every retry: `symlink_metadata(directory)` is Ok → `Reader::open` → "saved VIX reference companion unavailable or invalid". The search can never pass batch N.

**Minimal fix:**
- In `prepare_in_namespace`, take the owner lock with a blocking `lock()` (bounded by a deadline if needed) rather than `try_lock`. Readers hold it only for one projection.
- Report a contended lock as "busy, retry", not "already owned".
- Fix the cli3-1 shortcut (key it on `complete.bin`), which removes the permanent half of case (c).
- Move `Journal::open` ahead of `load_sources`, so a contended or owned journal refuses before any source is loaded.

---

## Pass-1 verification (findings that touch this slice)

- **cli3-1 (VIX directory-exists shortcut wedges the search): CONFIRMED, and wider than reported.**
  - index_stop_vix.rs:386-395 returns `Reader::open` whenever the directory exists. `prepare_in_namespace` creates that directory first (boolean_candidate_persistence.rs:96-98), and `complete.bin` is written last by `finish` (:74).
  - The trigger is not limited to a crash. Any live error after the directory exists has the same effect:
    - ENOSPC or EIO in `write_or_equal(body.bin)` or `(complete.bin)`;
    - a `verify_body` mismatch;
    - an owner `try_lock` refusal (indexstop-2 case c).
    The error is returned, the directory stays, and every retry takes the shortcut.
  - Fix gap: the proposed fix (shortcut only on `complete.bin`, otherwise `prepare_in_namespace`) is not enough for this namespace. The VIX body is not a function of its lookup identity (indexstop-1). After a pull changes the VIX store, the recapture differs from a fully written `body.bin` left by the failed attempt, and `write_or_equal` refuses for good.
- **cli3-1 second, transient case ("the api's index-stop worker and a cli run" reaching the same catalog): REFUTED.**
  - No cli verb runs an index-stop search or produces a catalog. `produce_catalog` and `index_stop_search::execute*` have no caller outside `api::indexstoplaunch` and tests; main.rs names neither.
  - In the api, launches are serialized by the `admit` slot and by `cli::execution_lease` (`claim_execution`, sweeprun.rs:3079).
  - Within one batch, lanes are distinct rungs, so their catalog and VIX lookup identities differ. Two writers cannot reach one VIX directory.
- **search-1 (torn content-addressed body/receipt wedges the identity and the search): CONFIRMED.**
  - `write_or_equal` (boolean_candidate_persistence.rs:189-211): `create_new`, then `write_all`, then `sync_all`. The `AlreadyExists` arm refuses any prefix. All five index-stop namespaces route through it.
  - `Frame::done` and `transition` (index_stop_search_checkpoint.rs:165-207, 529-543) force the exact batch retry.
  - Two additions:
    1. A live ENOSPC or EIO in `write_all` also leaves a short file. Nothing removes it and the error is returned, so no crash is needed.
    2. For `index-stop-source-context-v1` the wedge is wider than one batch. That context is re-published at the start of every launch over that source (index_stop_search.rs:322-335), so every search over that source, new or continued, refuses in `prepare_sources` forever.
- **cli2-1 (torn `attempts.bin` after a short write, no rollback): CONFIRMED for the index-stop trigger it names.** `append_events` (sweep_evidence.rs:1383-1386) has no `set_len` on a `write_all` error. Index-stop calls `begin_many` in groups of 16 (index_stop.rs:474-477) and `finish_many` (:427), plus single `begin` calls for the catalog, preparation, qualification and consistency attempts.
- **hunt-conc-5 (worker count in report bytes): CONFIRMED, info.** index_stop_search.rs:206-221 prints `lanes`, which comes from `available_parallelism`. It is not part of the declaration or identity (`legacy_declaration` omits `workers`).

## Checked and sound (not findings)

- **Single writer per search.**
  - `Journal::open` holds an exclusive owner flock for the whole invocation.
  - api launches are serialized by `admit` and by the store-scoped execution lease, which the `TaskFinisher` holds until the worker returns.
  - No cli verb writes `index-stop-search-v1`.
- **Checkpoint crash points.**
  - The pending frame is published before any child, and the done frame after all of them.
  - A crash between the two re-runs the same reserved batch. `recover` requires `history.len() == acknowledged` and a pinned predecessor chain, and `transition` forbids skipping or changing a reservation.
  - Reserved directories without `complete` are counted as interrupted and skipped by `next`.
  - I found no skip or double count. The 0-byte `complete` marker is GAP11-0, already known.
- **Determinism of retried children.** Catalog, qualification, consistency and source-context bodies contain no wall clock. Qualification numerics use no rayon, and the only parallelism (lanes) is reassembled by index. A retry after the done-publish fails is therefore byte-identical, except for VIX (indexstop-1).
- **`live::parallel` and `schedule`.**
  - A bounded channel. The receiver is dropped on `break`, so a blocked `send` returns Err.
  - `cancelled` is Release/Acquire. Scoped join with panic mapping. `broadcast` jobs are pinned to their thread, so the `workers` cap holds.
  - Telemetry `note` and the api observer run on the coordinator (`spawn_blocking`) thread, which carries the operation-audit thread-local.
- **`indexstoplaunch::conduct`.**
  - The observer takes `site.sweep` only for one assignment and never across a cli call.
  - Poison maps to a refusal, which cancels at the next boundary.
  - `Status::observe` monotonicity matches `progress()` across pending, stage and done transitions, including a recovered pending frame (identical repeat observation accepted).
- **Readers during writes.**
  - `Snapshot::open_prefix` pins a sequence ceiling, so a newer append cannot widen a pinned read.
  - An unpinned read during the publish window gets "busy" (payload exclusive lock) or a marker-width refusal. These are transient and leave the api caches intact, because the error comes before any slot mutation.
  - Every index-stop api cache uses `try_lock` (busy, no queueing) and evicts on currency failure. `indexstopjson` re-admits on an unpinned request.
- **Lock ordering.** Every observation lease is a non-blocking `try_lock_shared`. The only blocking flocks on the path are `attempts.bin` (taken while holding shared leases, never the reverse) and the fresh `create_new` checkpoint payload. A wait cycle is impossible.
- **Environment.** There is no runtime `set_var`. The configuration env vars read at admission and at execution are fingerprint-compared.

---

<!-- conc-pass2/expr.md -->
### conc-pass2 / expr: 3 findings (0 high, 0 medium, 3 low) at 331b05c

Verdict: the checkpoint chain itself is resume-safe: no skip and no double count after a kill at any write. Every defect I found is a read-side or lock-interaction problem: a reader's momentary shared flock makes a non-blocking writer acquisition fail, and the two API caches hold one global mutex across disk I/O. The known torn-`complete` wedge (GAP11-0) is still present and is the only crash-time defect in this slice.

Slice: `crates/cli/src/search_checkpoint.rs` (the shared journal under every search here), `expression_search.rs`, `expression_search_reader.rs`, `boolean_grammar_batch.rs`, `boolean_grammar_campaign.rs`, `boolean_campaign.rs`, `boolean_campaign_codec.rs` (transition), `boolean_campaign_reader.rs`, `crates/vocab/src/expression_search.rs` (`Cursor` encode/decode), `crates/api/src/expressionsearchjson.rs`, `booleanjson.rs` and `booleancampaignjson.rs`. I followed calls out into `boolean_candidate_persistence.rs`, `boolean_observation_file.rs`, `boolean_catalog_prepared.rs::run` and `sweep_evidence.rs` (begin/Drop/last_event) as far as the slice reaches them. Audit only: source reading, no cargo run, nothing edited.

---

## expr-1 (low): a read-only `Snapshot` probe takes a shared flock on the writer's `owner.lock`, so a CLI that starts or resumes the same search at that instant is refused as "already owned"

**Where:** `crates/cli/src/search_checkpoint.rs:106-115` (reader probe) against `:153-156` (writer acquisition).

```rust
// Snapshot::open_through (every API / observer read)
let owner = crate::readonly_file::open(&owner_path).map_err(error)?;
let before = crate::result_set::file_generation(&owner, &owner_path)?;
let writer_observed = match owner.try_lock_shared() {
    Ok(()) => { owner.unlock().map_err(error)?; false }
    Err(std::fs::TryLockError::WouldBlock) => true,
    ...
// Journal::open (the only writer door)
let owner =
    Flock::try_lock(open_owner(&owner_path)?, owner_path.clone()).map_err(|why| {
        format!("this exact search is already owned or cannot be locked: {why}")
    })?;
```

**Why it is wrong:** `Journal::open` uses a non-blocking exclusive `flock` to mean "another writer owns this search". A reader that only wants to *observe* liveness takes a shared lock on the same file, and an exclusive `try_lock` fails with `EWOULDBLOCK` against a shared holder too. So a harmless observer turns into a false "already owned" refusal for the real writer. Every `Snapshot::open` takes this probe. That covers `/expression-search.json` (`expression_search_reader.rs:97`), `/boolean-campaign.json` (`boolean_campaign_reader.rs:26`, taken twice per request: once in `Reader::open` and again in `require_current`), and the CLI's own `boolean_campaign::verify_complete` (`boolean_campaign.rs:693`). The refusal costs the writer more than a retry:

- `expression_search::run` has already appended an `ExpressionSearch` attempt start (`expression_search.rs:254-258`) before `Journal::open` (`:259`). That attempt is now sealed `Refused` by `Attempt::drop`.
- `boolean_grammar_campaign::execute_with` builds the full eight-rung `probe` preparation (`:147`) before `Journal::open` (`:162`). That preparation loads and audits every source month, and all of it is thrown away.
- `boolean_campaign::run_borrowed` (`:419`), reached from a grammar batch, fails the batch. The grammar `PLAN` stays pending until someone reruns the command by hand.

**Repro (two processes):**
1. Search S is paused. A browser tab loads `/expression-search.json?identity=S`. The API thread reaches `search_checkpoint.rs:108` and holds `LOCK_SH` on `expression-search-v1/<S>/owner.lock`.
2. Before that thread reaches `:110`, the operator runs `cli expression-search-stored ...` for S. Its `Journal::open` reaches `:154`, and `flock(LOCK_EX|LOCK_NB)` returns `EWOULDBLOCK`.
3. The CLI prints "this exact search is already owned or cannot be locked: …", and the attempt journal records a Refused `ExpressionSearch` attempt. No process owned S.

The window is short (open, fstat, lock, unlock), so this is low. It is still deterministic once the interleaving happens, and it does the opposite of what the probe was added for.

**Minimal fix:** make the writer tolerate observers. In `Journal::open`, retry `try_lock` on `WouldBlock` for a short bounded period (for example 50 attempts × 10 ms) before refusing as owned. A real owner holds the lock for the whole run, so it still refuses. Alternatively, have readers detect a writer without taking a lock that conflicts with the writer's acquisition, for example a separate `writer.lock` that only writers take exclusively and readers probe on a duplicate descriptor. The first fix is smaller.

---

## expr-2 (low): resuming a campaign rung re-prepares every child already completed, under a non-blocking exclusive owner lock, so one dashboard read of that child makes the rung refuse and burns two campaign checkpoints

**Where:** `crates/cli/src/boolean_candidate_persistence.rs:111-115`, reached on every rung (re)run from `boolean_catalog_prepared.rs:224-233` (all families recomputed and `persistence::prepare` called, with no completed-child shortcut), `boolean_statistics_v1.rs:191` and `boolean_admission_v1.rs:117`.

```rust
let owner = Flock::try_lock(
    crate::readonly_file::open(&owner_path).map_err(display)?,
    owner_path.clone(),
)
.map_err(|why| format!("Boolean candidate namespace already owned or lock refused: {why}"))?;
```

The readers that hold a shared lock on that same `owner.lock`:
- `boolean_campaign_reader.rs:170`: `Flock::try_lock_shared(&owner, …)` for every completed child listed in the snapshot, on each `/boolean-campaign.json` request, twice (open and `require_current`).
- `boolean_observation_file.rs:47` (`read_held(&owner_path, 0)`, shared), at every cold open, plus `ReadLease::acquire` (`:228`, shared) for every projection: `/boolean.json`, `/boolean-evidence.json` and the OOS pages.

**Why it is wrong:** the identities are content-addressed, so a resumed rung recomputes and re-`prepare`s exactly the children the snapshot already records as complete (`record_stage` accepts the identical pin). Those are exactly the children the dashboard reads. An exclusive `try_lock` meant to detect a concurrent *publisher* also fails against a *reader's* shared lease. The family result becomes `Err`, `collect_families` (`boolean_catalog_command.rs:189-209`) refuses the whole rung, and `run_borrowed` publishes `Refused` (`boolean_campaign.rs:478-495`). Each such retry costs two of the 1,024 campaign checkpoints (`MAX_CHECKPOINTS`, `:24`): one `Running` and one `Refused`. A grammar campaign whose pending batch needs the rung fails that invocation.

**Repro (two processes):**
1. Campaign C, rung `5m`. Families F1..Fn completed, then the statistics stage was refused, or the process was killed. The snapshot records F1..Fn completions.
2. The operator opens `/boolean.json?identity=<F3>` (or `/boolean-campaign.json?identity=C`). The API thread is inside `Observation::open` or `with_current`, holding `LOCK_SH` on `boolean-candidates-v1/<F3>/owner.lock`.
3. Meanwhile `cli boolean-campaign-stored …` (or `boolean-grammar-campaign-stored`) resumes C. The rayon worker for F3 reaches `persistence::prepare` → `:111`, gets `EWOULDBLOCK`, and returns "Boolean candidate namespace already owned or lock refused".
4. `collect_families` refuses. The rung is checkpointed `Refused`, and the command exits non-zero, although no other writer existed.

**Minimal fix:** in `prepare_in_namespace`, take the owner lock with the blocking `Flock::lock` (two publishers of one content-addressed identity produce identical bytes, so waiting is safe), or with a bounded retry loop. The cleaner fix is to short-circuit before `prepare`: when `complete.bin` already exists and `persistence::verify` accepts the recomputed `(identity, payload, bytes)`, return the committed value without taking the exclusive lock at all.

---

## expr-3 (low): `/expression-search.json` and `/boolean.json` hold a process-wide std `Mutex` across cold disk verification, so concurrent requests park shared `detail::run` permits and unrelated routes answer 429

**Where:**
- `crates/api/src/expressionsearchjson.rs:115-154`: `SESSIONS.lock()` is held across `Reader::open` (directory discovery up to 1,000,000 entries, then a checkpoint read) **and** `held.reader.page(...)` (up to 256 links within `MAX_SCAN_BYTES` = 64 MiB: each child's signal file is read and verified, `sweep_evidence::read_attempt` and `candidate_trades::read_model` run, and `last_event` takes a **blocking** `lock_shared` at `sweep_evidence.rs:1422`, which waits out a CLI append's write and fsync).
- `crates/api/src/booleanjson.rs:192-221`: `CACHE.lock()` is held across a cold `Reader::open`, which hashes and decodes the whole catalog body (bounded by `BooleanObservationBudget`), and across `project`.

```rust
let mut sessions = SESSIONS.get_or_init(...).lock().map_err(|_| "search snapshot cache poisoned")?;
... Reader::open(root, asked.identity, crate::detail::MAX_SCAN_BYTES)? ...
let page = held.reader.page(asked.cursor, asked.limit, crate::detail::MAX_SCAN_BYTES)?;
```

**Why it is wrong:** both handlers run inside `crate::detail::run`, which first takes one of `MAX_CONCURRENT = 4` shared permits (`detail.rs:14`, refused as `Saturated` with nothing queued). A second request to the same route, **for any identity**, then blocks on the mutex while still holding its permit. With three such requests parked behind one slow cold read, every other `detail::run` route in the server returns 429 "capacity full". That is the same mechanism as pass-1 **runs-4** (the `ADMISSION` mutex), at two new sites. It also serializes reads of unrelated identities, which the per-identity design does not need.

**Repro:** tab 1 opens the first page of a long expression search (`limit=256`) whose 256 child signal files total about 60 MiB. That page reads all of them three times under `SESSIONS`. Tabs 2 and 3 request any other expression-search page, and tab 4 requests a third page. Each takes a permit and blocks on `SESSIONS`. Every other `detail::run` route (`/boolean-campaign.json`, `/candidate-trades.json`, `/backtest/run.json` status, …) now gets `RunError::Saturated` until tab 1's verification ends. The same happens with `/boolean.json` while a cold catalog hash holds `CACHE`.

**Minimal fix:** take the mutex only to look up, insert or evict entries. Hold each session or reader behind its own `Arc<Mutex<…>>` (or take it out of the deque and put it back afterwards). Run `Reader::open` and `page`/`project` outside the global lock. Alternatively, `try_lock` the global mutex and answer 503 "busy" instead of parking a permit, as the `indexstop*json` handlers already do.

---

## Pass-1 verification (findings touching this slice)

| Pass-1 ID | Verdict | Reason (at 331b05c) |
|---|---|---|
| **apicache-2** (unpinned `/expression-search.json` discards another viewer's session) | **CONFIRMED** | `expressionsearchjson.rs:133-137` still `retain`s away every session with the same `(root, identity, checkpoint)` and pushes a fresh `Reader` whose `admitted` is `{checkpoint}` only (`expression_search_reader.rs:127`). The pinned lookup at `:119-127` matches on checkpoint alone, so the old viewer's learned cursor fails `page()` at `expression_search_reader.rs:151-152`. **Addendum (same class, not counted):** eviction at `:138-140` is `pop_front` by insertion order, and pinned use never moves a session to the back. So eight first-page loads of *other* identities also evict an actively paginating viewer ("snapshot is not admitted or expired"). The proposed fix should also move a pinned hit to the back. |
| **search-1** (crash inside `write_or_equal` wedges a content-addressed identity forever) | **CONFIRMED**, and it reaches this slice | `boolean_candidate_persistence.rs:189-211` is unchanged: `create_new` comes before the write, and on retry `AlreadyExists` → `read_exact(path, body.len()) != body` → permanent refusal. In this slice the effect is: every resume of the campaign rung re-`prepare`s that identity (`boolean_catalog_prepared.rs:224-233`) and refuses, publishing `Running` then `Refused` (two of `MAX_CHECKPOINTS` = 1024) each time, until `publish` refuses "checkpoint history ceiling reached". A grammar campaign cannot route around the batch, because `restore` refuses "grammar advanced past an unfinished batch" (`boolean_grammar_campaign.rs:383-385`), so the whole grammar search is wedged too. That was already implied by search-1's "every resumable search whose pending batch needs it". |
| search (pass-1) "boolean_campaign.rs and boolean_grammar_campaign.rs: checkpoint chains are safe after a publish error because `Journal::publish` self-poisons" (clean note) | **CONFIRMED** | `search_checkpoint.rs:204-211` sets `poisoned` on any `publish_inner` error, so the refusal publish in `run_borrowed:478-495` errors loudly and does not fork the chain. |
| xcut H8 note: torn `complete` marker (GAP11-0) and no `DIRECTORY_LIMIT` check (W2-cli13-5), both KNOWN | **CONFIRMED still present** | `search_checkpoint.rs:263-267` is still `File::create_new(...complete)` followed by `write_all(&seal)`. `discover_through:457-463` counts a 0-byte marker as acknowledged and makes it `latest`. `read_saved:486-488` refuses "marker width mismatch" from then on for both `Journal::latest` (the writer can never resume) and every `Snapshot` reader. **Not counted, but a trigger the earlier reports did not name:** the same 0-byte marker is visible *without a crash*. A `Snapshot::open` that runs between `:263` and `:265` of a live publish returns "checkpoint marker width mismatch" to `/expression-search.json` or `/boolean-campaign.json`, which reads as corruption rather than "busy". The GAP11-0 fix (write the seal under a temporary name, fsync, then `link`/`rename` to `complete`) removes both. `publish_inner:224-230` still has no `DIRECTORY_LIMIT` check. `expression_search` relies only on discovery refusing at 1,000,000 entries, after the fact. |

## Checked and clean (not findings)

- **Expression search resume after a kill at each write** (`expression_search.rs:509-551`, `run:246-310`):
  - Killed after a child's `attempt.finish(Completed)` but before `journal.publish`: the child attempt is an unreferenced orphan, and the rerun re-advances the same cursor from the last acknowledged checkpoint, evaluates the same expression under a **new** token, and publishes it. `candidates`/`rows`/`qualifying` live only in the checkpoint, so nothing is double-counted.
  - Killed inside `publish_inner` before the marker: a reserved hole, counted as `interrupted`, with `previous` still pointing at the last acknowledged seal.
  - Killed mid-`evaluate_candidate`: the child attempt never gets a terminal record (no Drop on SIGKILL). It is token-isolated, and `verify_history` only follows `last` pointers in acknowledged checkpoints.
  - `verify_history`/`verify_transition` re-prove every transition, with work delta ≤ 4096, matching `execute`'s `remaining.min(4096)`.
- **`Cursor` resume determinism** (`vocab/src/expression_search.rs`): equality and the encoded form exclude scratch (D-0750/D-0754), and `decode` rebuilds `code`/`starts`/`depths` below `at` through `place` (`:370-373`). A resumed cursor therefore advances exactly as an uninterrupted one. `Batch::follows` compares encodings only.
- **Grammar campaign** (`boolean_grammar_campaign.rs`):
  - A kill after `PLAN` and before `DONE` leaves a pending batch, and resume re-runs the same campaign identity.
  - A kill after the inner campaign completes but before `DONE`: `run_borrowed` sees `Completed` and returns the same pin without publishing (`:422-429`). `codec::transition` forbids any record after `Completed`, so the pin `DONE` stores can never be superseded.
  - `restore` refuses skips, repeats and post-exhaustion records. `admit_checkpoints` charges holes.
- **Boolean campaign** (`boolean_campaign.rs`): `Running`, `Paused` and `Refused` rows resume to `Running`. Completed and Excluded rows are immutable under `transition`. Completion is published in the same record as the last rung's `Completed`. The reader refuses `sequence >= MAX_CHECKPOINTS`, and the writer's guard keeps sequences ≤ 1023.
- **Two writers of one search identity:** both `Journal::open` calls use an exclusive non-blocking flock that the owner keeps until drop, so the second refuses. (A side note, not counted: `expression_search::run` appends its `ExpressionSearch` attempt start before `Journal::open`. A refused second process therefore becomes the identity's newest attempt, marked Refused, while the first is still running. No reader consumes that operation's latest attempt; grep shows only `expression_search.rs:256` and tests.)
- **Child identity sharing across searches:** expression candidate identities omit the search alphabet, so two searches can evaluate the same child concurrently. `sweep_evidence::begin` allocates distinct tokens under the flocked journal, and every reader reads `(identity, attempt)` exactly, so they do not collide.
- **Reader staleness:** `Snapshot` holds no lock between calls. A held `expression_search_reader::Reader` stays pinned to an immutable checkpoint and re-verifies it on every page (`:155-157`, `:204-206`). The `booleanjson` cached `Observation` keeps body and receipt shared-locked, but the writer only ever takes shared locks on those two files (`read_held`), so a cached reader never blocks a publish.
- **Ordering and atomics:** no atomics, threads or `HashMap` iteration reach output in this slice. The `HashSet<Anchor>` in the expression reader is membership-only. Rayon family results in `boolean_catalog_prepared::run` come back in input order (`par_iter().map().collect()`). The grammar identity and the campaign descriptor hash fixed-order inputs.
- **Poisoning:** `SESSIONS`, `CACHE` and `ranked_digest` map poison to a refusal. I found no reachable panic under `SESSIONS` or `CACHE` (all arithmetic in `page`/`transition` is checked or guarded), so the wedge-after-panic path is not reachable.

---

<!-- conc-pass2/replay.md -->
Verdict: 5 findings at 331b05c (0 high, 2 medium, 3 low). The production replay writer (V4) is sound. The defects are in the VIX companion's month capture, the checksum-receipt reuse path, and a late VIX lock collision.

### conc-pass2 / replay: global replay (v1-v4), VIX month identity, api readers of replay output, checksum_receipts, stored_data_completeness

Method: I read the source only and ran no cargo. Reachability at this commit:
- The only production replay path is `ledger_v6::replay_route` → `global_replay_v4::commit_stored_global_replay_v4`.
- `global_replay` (v1), `global_replay_v2` and `global_replay_v3` have no non-test caller. v2 is named only by `step3_comparison`, and nothing calls that.
- No api route reads Global Replay output. A grep of crates/api for global/replay finds only the Boolean replay-node knobs.
- The api reads the index-stop VIX companion through `indexstopvixjson.rs`, and the api's index-stop worker writes it.
- `checksum_receipts` is reached from cli `checksum-audit-stored` and from api `sweeprun` → `audited_range_command` → `audited_range` / `audited_stored`.
- `stored_data_completeness`'s ledger is opened only by tests and by the uncalled `step3_comparison`.

---

## replay-1 (medium): a transient VIX month-lock collision is published permanently as "Unavailable" in the index-stop VIX companion

**Where:** crates/cli/src/index_stop_vix.rs:530-559 (`load_month`), called from `capture` (:478-503), which `publish` (:373-411) calls. The writer is the api pull of `SpotTarget::Indices` (crates/api/src/ingest.rs:136, which "includes NSE-INDIAVIX") through `BarFile::open_or_create`.

```rust
match VixReferenceMonth::open(store, feed, month) {
    Ok(loaded) => Ok((Month { ... records: Some(loaded.records()), snapshot_digest: Some(loaded.snapshot_digest()), ..}, Some(loaded))),
    Err(reason) => {
        if reason.is_empty() || reason.len() > MAX_REASON_BYTES { return Err(..) }
        Ok((Month { ..., records: None, snapshot_digest: None, unavailable_reason: Some(reason) }, None))
    }
}
```

**Why it is wrong:**
- `VixReferenceMonth::open` → `BarFile::open_existing` takes a non-blocking shared try-lock on the month `.lock`. It refuses while any writer holds the month. vix_reference.rs:612-630 pins this as "a live writer must refuse the reference reader … another writer holds".
- `load_month` turns *every* `Err` into a typed `Unavailable` month. That includes this transient lock refusal, not only a missing file.
- The image is written content-addressed under `lookup_identity(catalog.identity(), catalog.completion_digest())` (:707-713). That identity names no VIX bytes.
- Every later `publish` for the same catalog sees the directory and returns `Reader::open` of the saved companion (:384-391). It never recaptures.
- So one moment of lock contention becomes a permanent statement that the VIX month was unavailable. Every trade in that month then carries `Stamp::Unavailable` in the companion and in `/index-stop-vix.json`, although the data exists.
- This is the "fallback that hides a failure" shape of §4, made durable. It is loud once, in a saved diagnostic, but it is wrong forever, and §3 rule 8 forbids rewriting it.

**Repro (two writers in one api process, or api plus cli):**
1. The api autopilot or a hand `/pull` ingests `NSE-INDIAVIX` 1min for month M. `BarFile::open_or_create` holds the exclusive month lock while it appends.
2. In the same window, an index-stop search (the api index-stop worker or `cli index-stop-search`) reaches `produce_catalog_inner` → `publish_vix` for a catalog whose trades fall in M.
3. `capture` → `load_month(M)` → `open_existing` gets EWOULDBLOCK. The month is saved with `unavailable_reason = "... could not be opened as reference evidence: ... another writer holds ..."`. `prepare_in_namespace` and `finish` publish it.
4. The pull finishes a second later. Every rerun of the search, and every `/index-stop-vix.json` read, now serves the saved Unavailable month for M, permanently.

**Minimal fix:** Classify the open error. Persist `Unavailable` only for durable facts about the store (a missing month file, or a corrupt or malformed month). Propagate a lock or busy refusal as an `Err` from `capture`, so nothing is published and the retry recaptures. Alternatively, wait on a blocking shared lock with a bound. Add a test that holds a writer on the VIX month and asserts that `publish` errs and leaves no namespace directory.

---

## replay-2 (medium): the VIX companion body is not a function of its lookup identity, so a retry after a crash or a race writes different bytes and `write_or_equal` refuses for good. cli3-1's proposed fix does not close the wedge.

**Where:** crates/cli/src/index_stop_vix.rs:384-410 (`publish`) and :530-559 (`load_month`). crates/cli/src/boolean_candidate_persistence.rs:189-208 (`write_or_equal`).

```rust
let lookup = lookup_identity(identity, pin);           // catalog only
...
let image = capture(store, feed, catalog, bounds)?;     // reads live VIX months
let body = codec::encode(&image, bounds.bytes)?;        // embeds records, snapshot_digest, reason text
let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
```
```rust
Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
    if read_exact(path, body.len() as u64)? != body {
        return Err("Boolean evidence already exists with different or incomplete bytes; history preserved".to_owned());
```

**Why it is wrong:**
- The encoded body includes, for each VIX month, `records`, `snapshot_digest` and the free-text `unavailable_reason` (index_stop_vix_codec.rs:93-99).
- Each of those depends on the live store at capture time. The current VIX month grows minute by minute, and a month can be busy (replay-1) or absent before a backfill.
- The directory name, `lookup`, depends only on the catalog.
- cli3-1's minimal fix is to key the shortcut on `complete.bin` and otherwise re-enter `prepare_in_namespace`. That re-enters with a *freshly captured* body.
- If the crash came after `body.bin` was fully written (`write_or_equal` create_new + `write_all` + `sync_all`) but before `finish` wrote `complete.bin`, the retry compares the new body to the old one. They differ whenever any VIX month changed in between, and the retry refuses "different or incomplete bytes" on every later attempt. The single-stop checkpoint makes that retry mandatory, so the search stays stuck at batch N exactly as cli3-1 describes.

**Repro (crash):**
1. Batch N's catalog has trades in the current month M. `publish` captures M with 4,000 VIX records, writes and fsyncs `body.bin`, and is SIGKILLed before `finish` (boolean_candidate_persistence.rs:66-79).
2. The pull appends 30 more VIX minutes to M.
3. With cli3-1's fix applied, the rerun sees no `complete.bin` and recaptures: M now has 4,030 records and a new `snapshot_digest`. `write_or_equal(body.bin)` hits AlreadyExists, the bytes are unequal, and it refuses. Every rerun repeats this, because the store only grows.
4. Without cli3-1's fix the same state refuses on the missing `complete.bin`. Either way the identity is wedged.

**Repro (race, transient):**
1. Process A and process B both reach `symlink_metadata(&directory)`, and both get NotFound.
2. A captures, publishes and releases its owner lock.
3. B captures after a VIX append. `prepare_in_namespace` takes the now-free owner lock, and `write_or_equal` refuses "different bytes".
4. B's catalog attempt is marked Refused, although a valid companion exists.

**Minimal fix:** When `body.bin` exists and `complete.bin` does not, *adopt* the saved body instead of recapturing:
- read it under the owner lock;
- decode it, and check that its catalog identity, completion pin and feed equal this call's;
- verify its digest and `finish` with it.

On a race, treat "AlreadyExists with different bytes" from a concurrent completed publisher as success when `complete.bin` now exists, by re-checking it and returning `Reader::open`. Apply this together with cli3-1's fix.

---

## replay-3 (low): `checksum_receipts::publish` reuses a full-length receipt without ever fsyncing it or its directory, so a crash before the first publisher's barrier leaves a receipt that is admitted but was never made durable

**Where:** crates/cli/src/checksum_receipts.rs:306-312 (fast path). The barriers it skips are at :345-357. The same path serves `publish_binding` (:421) and `publish_span_binding` (:470).

```rust
fn publish(path: &Path, expected: &[u8; BYTES]) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == BYTES as u64)
    {
        Receipt::open(path, expected)?;   // read-only, shared lock, compares bytes
        return Ok(());
    }
    ...
        file.sync_all().map_err(error)?;          // :354
        File::open(path.parent()...).and_then(|dir| dir.sync_all())  // :355-357
```

**Why it is wrong:**
- The slow path makes the file and its directory entry durable before it returns.
- The fast path decides "already published" from the length alone, and verifies the bytes through the page cache. It never syncs anything.
- So a publisher killed after the seal `write_all` (:347-352) and before `sync_all` (:354), or before the directory sync (:355-357), leaves a full-length, byte-correct, *unsynced* receipt.
- Every later `audit_month`, `publish_binding` or `publish_span_binding` takes the fast path, admits it, and continues. The sweep's strict results and span bindings then name this receipt identity.
- The doc on `perform` promises a refusal on "any failed ... publication or completion durability barrier". Here the barrier never ran, and nothing reports it.
- V4's `persist` (global_replay_v4_store.rs:27-76) shows the correct pattern: it always takes the locked path, and on an exact full-length file it writes nothing but still runs both fsyncs and the directory fsync.

**Repro:**
1. Run `cli checksum-audit-stored ...`. The new receipt is written. The process is SIGKILLed between :352 and :354.
2. The operator reruns, or the api strict sweep audits the same month. `symlink_metadata` sees 512 bytes, `Receipt::open` matches, and the run continues and records output that names the receipt.
3. Power is lost before writeback.
4. On reboot the receipt is absent or zero-length. `admit_month(expected)` refuses it as missing. The strict-read door "never creates or repairs", so every strict re-admission of that recorded identity fails until someone reruns `audit_month`. On filesystems that expose unwritten extents as zeros, the 512-byte zero file is refused as "not an exact prefix" for good.

**Minimal fix:** In the fast path, after `Receipt::open` succeeds:
- open the file writable;
- take the exclusive lock, or do it under the shared lease, since fsync needs no write access;
- call `sync_all` on the file and fsync the parent directory.

The simpler alternative is to drop the fast path and always take the locked path, which writes nothing on an exact file, as V4 does.

---

## replay-4 (low): `namespace_directory` fsyncs the receipt root only when it creates the namespace, so a crash between `create_dir` and that fsync is never repaired

**Where:** crates/cli/src/checksum_receipts.rs:375-383.

```rust
match fs::create_dir(&base) {
    Ok(()) => File::open(&root).and_then(|dir| dir.sync_all()).map_err(error)?,
    Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
```

**Why it is wrong:**
- A run killed after `create_dir` and before `sync_all` leaves the namespace entry in `root` not yet durable.
- Every later run takes the `AlreadyExists` arm and never fsyncs `root`.
- Receipts are then fsynced into `checksum-receipts-v1/`, `audited-inputs-v1/` and `audited-spans-v1/` (`publish` syncs only `path.parent()`), and reported durable. The entry that makes those directories reachable has never had a barrier.

**Repro:**
1. Run 1 is SIGKILLed between :376 and :377.
2. Run 2 publishes and syncs a receipt, and the strict sweep records results.
3. Power is lost. Under POSIX semantics the `checksum-receipts-v1` entry may be lost, taking every receipt in it.
4. ext4 and xfs usually commit the mkdir along with the later fsync, so this is filesystem-dependent. That is why it is low.

**Minimal fix:** fsync `root` in the `AlreadyExists` arm as well. It is one cheap fsync per call, and it makes the barrier idempotent.

---

## replay-5 (low): Global Replay V4 opens VIX months with a non-blocking try-lock only after all OOS work, so a concurrent VIX pull refuses the whole replay

**Where:** crates/cli/src/global_replay_v4.rs:473-489 (`VixCatalog::stamp`), reached from `schedule` (:325) → `account_decision` (:607-608). That is after `selection.replay(...)` (:229) has folded and replayed every stream.

```rust
self.months.insert(
    (feed, month),
    VixReferenceMonth::open(self.root, feed, month)?,
);
```

**Why it is wrong:**
- `VixReferenceMonth::open` → `BarFile::open_existing` refuses immediately when a writer holds the month lock.
- The months are opened lazily, one per (feed, IST civil month) of each admitted priceable trade. That happens only at the end of `prepare`.
- So any api VIX pull overlapping the end of a `ledger-v6-replay` aborts the whole run: all eight Selection V6 replays and every OOS fold, possibly hours of work. Lifecycle records it as Refused, and the run must be redone.
- It is loud and a retry succeeds, so it is a liveness defect, not a correctness one.
- The month key is computed correctly: IST civil month via `pull::session::IstMoment`, the same authority as `vix_reference::slot_of`, and entry and exit months are opened separately.

**Repro:**
1. Start `cli ledger-v6-replay ...` with an OOS span whose admitted trades fall in month M.
2. While it computes, the api autopilot pulls `SpotTarget::Indices` and holds the `NSE-INDIAVIX` 1min M writer.
3. When `schedule` reaches the first admitted trade in M, `stamp` gets "another writer holds". `prepare` fails, and `recorded` finishes the attempt as Refused.

**Minimal fix:** Resolve the set of VIX months the trades need, and open them *before* the replay work. Alternatively, retry the shared try-lock with a bounded backoff that names the wait. Do not convert the refusal into absence (see replay-1).

---

## Pass-1 verification

- **cli3-1 (index_stop_vix publish shortcut on directory existence):** CONFIRMED. index_stop_vix.rs:384-395 still returns `Reader::open` whenever the directory exists. `prepare_in_namespace` (boolean_candidate_persistence.rs:96-119) still creates the directory before `owner.lock`, `body.bin` and `complete.bin`. Its minimal fix is insufficient on its own; see replay-2.
- **cli3-3 (stored_data_completeness ragged tail, latent):** CONFIRMED.
  - `commit` (stored_data_completeness.rs:885-893) rolls back only when `write_all` returns an error. The ragged-tail refusal in `scan_file` remains.
  - It is still latent. The only non-test openers are in `step3_comparison` (:772 `open_read`; :1712 is inside `mod tests`, which starts at :1508), and `compare_step3_on_disk_v1` has no caller anywhere in the workspace.
  - Addition of the same latent class: a failed `sync_all` (:890-894) returns without `set_len(start)` and without refreshing `generation`. The written record stays in the page cache but not in `self.receipts`. The same handle's next `commit` then refuses "changed since validation", and a reopen adopts the unsynced record.
- **cli3 "checked clean": checksum_receipts.rs:** PARTLY REFUTED. The locked slow path is as described. The unlocked full-length fast path skips every durability barrier (replay-3), and the namespace-root fsync is not idempotent (replay-4).
- **cli3 "checked clean": global_replay_v4_store.rs:** CONFIRMED.
  - The blocking exclusive `lock()` covers the prefix compare and the append.
  - A rerun on any prefix, including the full length, always reaches both `sync_all`s and the directory fsync (:63-74).
  - `verify` takes a blocking shared lock and checks exact length, per-record checks, ordering, digest and generation.
  - A crash at any line between create and directory fsync is completed or matched on the next run.
- **cli3 "checked clean": global_replay_v4_lifecycle.rs:** CONFIRMED. `recorded` gives every begun stage and the parent a terminal. Overlapping or mismatched stages are refused, and both causes are kept by `combine`.

## Checked and clean (no finding)

- **V4 determinism:**
  - `schedule` uses `sort_unstable_by_key((entry_micros, priority, strategy_digest))`. Full-key ties cannot reach output, because `GlobalSinglePositionV1::schedule_minute` refuses `DuplicateOrderingKey` (runner/src/portfolio.rs:464).
  - `VixCatalog.months` is a HashMap used only for lookup. No iteration reaches the bytes.
  - `replay_id` excludes VIX rows (kind 5). `publication_id` includes them by design.
- **V4 publication races:** two processes publishing the same `publication_id` serialize on the blocking flock. The second one compares the full prefix, writes nothing and still fsyncs. Different VIX snapshots give different publication ids, so neither file can be corrupted.
- **checksum_receipts concurrency:**
  - The writer's exclusive try-lock against a reader's shared try-lock refuses loudly (WouldBlock), never silently.
  - `Receipt` holds its shared flock for its lifetime, with an explicit unlock on drop (store::flock), and rechecks generation, nlink and bytes on every `require_current`/`read_record`.
  - std `File::try_lock` and `store::flock::Flock` are both flock(2), so they exclude each other.
- **stored_data_completeness locking:** the writable open holds an exclusive flock across the header create and scan. `commit` holds it across `require_unchanged`, append and sync. `authority` re-checks length and digest under a shared lock.
- **vix_reference.rs:** no shared state. The month is indexed from one `BarFile` snapshot of `n_valid`, and the `BarFile` is dropped before return. `slot_of` and `IstMoment` month identity are consistent across both callers.
- **global_replay v1/v2/v3:** not reachable in production at this commit, so not audited for crash behaviour.

---

<!-- conc-pass2/cand.md -->
Verdict: 2 new findings at 331b05c in the candidate slice (0 high, 0 medium, 2 low). All 7 pass-1/workspace findings that touch the slice re-verify as CONFIRMED. None is refuted.

Slice: crates/cli/src/candidate_universe.rs, candidate_trades.rs + candidate_trades/codec.rs, boolean_candidate_{grid,persistence,reader,v1}.rs (plus boolean_observation_file.rs, which persistence includes with `#[path]`), crates/api/src/candidatejson.rs. Method: source reading only. No cargo was run.

---

## cand-1 (low): a foreign-identity row orphan wedges the shared Candidate Universe ledger, and after a rebuild nothing can complete it

New site of the pop2-4 class. pop2-4 lists only the Population Admission, Finalization and Statistics ledgers. Pass-1 search marked candidate_universe "checked clean" for rollback and ragged tails, but not for this.

- **Where:** `crates/cli/src/candidate_universe.rs:3700-3712` (`append_complete_locked`). Identity is set at `:5504-5519` (`derive_universe_id` hashes `identities`, including `source_commit_digest`, `:5548-5563`). Source commit is set at `:4272` (`source_commit_digest: hash(source.execution_series.commit().as_bytes())`).
- **Code:**
  ```rust
  let (first_row, prefix) = match self.orphan {
      Some(orphan) => {
          if orphan.universe_id != receipt.universe_id() {
              return Err(format!(
                  "candidate row tail belongs to {}, not requested {}; no fallback may hide it", ...
  ```
- **Why it is wrong:**
  - The rows are synced first (`:3747-3750`) and the receipt is written after them (`:3758-3761`).
  - A process death between those two leaves a whole-row orphan block for universe U. `scan_orphan` reconstructs it on every open.
  - After that, the only append the ledger accepts is the exact retry of U. U's id includes the build commit, the data digest, the requested span and every policy digest.
  - The Step 3 root is shared: `step3_orchestrator.rs:3135` passes `root.path()` for every family and span, and default bounds allow 8 universes (`:4734`). So the orphan blocks every other universe on that root too: the other family, other spans, and later runs.
  - Usually the fix for a crash is to rebuild. Once the build commit changes, the exact retry can never be built again, and the root refuses every later Candidate append with no recovery tool. A grep for quarantine or discard-orphan in the module finds nothing.
  - CU-02 (`docs/04-invariants.md:3634`) documents that a foreign orphan refuses. It does not say that the refusal is permanent across builds. docs/06-limits.md is silent on it.
- **Repro:**
  1. Run a Step 3 Candidate commit for family NIFTY on root R. SIGKILL the process after `self.row_file.sync_data()` at `:3747` returns and before `append_receipt` at `:3758`. The rows are durable and no receipt exists.
  2. Rebuild at a new commit, for example with the fix for whatever crashed. `commit_stamp` requires a clean build, so the commit differs.
  3. Rerun Step 3 on R. `produce_candidate_universe_v1` derives U' != U, because `source_commit_digest` changed.
  4. `append_complete_locked` refuses with "candidate row tail belongs to U, not requested U'".
  5. Every later run on R refuses the same way, for any family or span.
- **Minimal fix:** One option is to let the writer skip a fully valid, receipt-less foreign orphan, as Execution V1 does (EC-01). It starts the new block after the orphan and leaves the orphan as unreferenced evidence. `scan` would need to accept more than one receipt-less block, or use a skip marker. The other option is an operator verb that records the orphan and quarantines it. At minimum, put the orphan's source commit into the refusal text, so the operator knows which build can heal it.

---

## cand-2 (low): `/candidate-trades.json` waits on a blocking process-wide mutex while it holds a shared detail permit, so contention on trade pages returns 429 on unrelated detail routes

- **Where:** `crates/api/src/candidatejson.rs:302-328` (`trade_page`), reached inside `crate::detail::run` (`:146`). `detail::run` takes one of `MAX_CONCURRENT = 4` permits (`detail.rs:14`, `:105`). Those permits are shared by the routes in 23 api files.
- **Code:**
  ```rust
  static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
  let mut cached = CACHE.get_or_init(|| Mutex::new(None)).lock()   // blocking
      .map_err(|_| "candidate trade reader cache poisoned")?;
  if cached.as_ref().is_none_or(|held| ... || held.key != key) {
      let reader = TradeReader::open(root, summary, key, crate::detail::MAX_SCAN_BYTES)?;  // under the lock
  ```
- **Why it is wrong:**
  - `TradeReader::open` (`candidate_trades.rs:1047-1092`) runs while the global mutex is held. It reads, BLAKE3-hashes and seal-checks every trade row, up to 64 MiB. It also runs `pinned` twice, and the first `pinned` can be a cold catalog read.
  - There is one slot, so any change of key causes a cold open (documented as W1-api2-3 / D-1444).
  - Every other trade-page request blocks in `lock()` on its `spawn_blocking` thread and keeps its detail permit while it waits.
  - With one cold open running and three requests waiting, all 4 permits are held. Every other detail route then returns 429 "capacity is full", even though only one thread is doing work. That includes `/trades.json`, `/frontier.json`, booleanjson and indexstop*.
  - The sibling caches in the same pool avoid this by design: indexstopcandles, indexstopvix, indexstopqualification and indexstopranking use `try_lock` with a busy refusal and no queueing.
  - D-1444 states the per-request cost. It does not state the head-of-line blocking or the permit starvation.
- **Repro:**
  1. Two viewers page the trades of two different candidates K1 and K2 of one capture, each with a large trade file (for example 400k trades, about 54 MB). Each request evicts the other's slot, so every request is a cold open under the mutex.
  2. With 4 such requests in flight, a concurrent `GET /trades.json` gets 429 for the whole time the queue drains.
- **Minimal fix:** Use `try_lock` and refuse "candidate trade reader busy; retry", as the indexstop routes do. Alternatively, run `TradeReader::open` outside the lock and install it afterwards, holding the lock only for the swap and the `page` call. Either way, also evict on a `page` error (apicache-1).

---

## Pass-1 verification

| finding | verdict | reason (code at 331b05c) |
|---|---|---|
| **apicache-1** (stale `TradeReader` never evicted) | CONFIRMED | `candidatejson.rs:326-327`: `let held = cached.as_mut()...; let rows = held.reader.page(offset, limit)?;`. An Err returns with the slot still occupied. The key check at `:308-315` compares content only (model, root, identity, attempt, digest, key). `page_locked` (`candidate_trades.rs:1106-1108`) refuses any generation change, ctime included (`result_set.rs:577-591`). |
| **cli2-5** (`write_exact` exposes an empty file before it locks) | CONFIRMED | `candidate_trades.rs:1316-1322`: `create_new` comes first, then `Flock::lock(file, path)`. `read_model` at `:742-744` checks only `try_exists`, then `read_sealed_generation` at `:1361-1368` takes `try_lock_shared` and refuses `len < HEADER+SEAL` as "nonregular, truncated". The same path is also permanent for an attempt whose process died between `create_new` and `sync_all` of `catalog.bin`. The api then reports corruption for that attempt, not "missing/incomplete". The attempt token is never reused, so a rerun is not wedged. |
| **xcut-3** (candidate-trades ancestors never fsynced) | CONFIRMED | `candidate_trades.rs:322`: `fs::create_dir_all(&directory)` creates up to 4 levels (`directory_for`, `:1249-1257`). The only directory sync is `File::open(path.parent())...sync_all()` at `:1342-1344`, which is the leaf. |
| **search-1** (`write_or_equal` crash wedges a content-addressed identity) | CONFIRMED | `boolean_candidate_persistence.rs:189-211` is unchanged. `create_new` comes before `write_all`/`sync_all`, and the `AlreadyExists` arm refuses any prefix as "different or incomplete bytes". Identities are deterministic (`boolean_candidate_v1.rs:300-301`). |
| **cli3-1** (index-stop VIX companion `symlink_metadata` shortcut) | CONFIRMED | `index_stop_vix.rs:386-392`: `Ok(_) => Reader::open(...)` whenever the directory exists. `prepare_in_namespace` creates and syncs the directory first (`boolean_candidate_persistence.rs:92-95`) and writes `complete.bin` last (`:74`). `Observation::open` then fails on the missing `complete.bin` (`boolean_observation_file.rs:47`). |
| **hunt-conc-2** (Boolean attempt tokens follow the rayon schedule) | CONFIRMED | `boolean_catalog_prepared.rs:224-231`: `families().par_iter().map(... self.produce(...))`. That reaches `boolean_candidate_v1.rs:303-304` (`sweep_evidence::begin`) and, per program, `:752-753`. |
| **errpaths** `candidate_trades.rs:389` poison-swallowing `if let Ok` (info) | CONFIRMED (now `:425-429`, `refuse`) | It is still latent. `check`, `tier`, `record_inner`, `finish` and `confirm` all map poison to Err, so a dropped `failed` latch cannot let a poisoned capture publish. |
| pop2-7 class (an unrelated append invalidates a held handle; nothing serializes runs on one root) | applies here, not re-counted | `CandidateUniverseLedgerV1` releases its flock between `open_read`/`reopen_audit` and `complete_population_rows` (`step3_orchestrator.rs:696-729`). A concurrent append by another process changes `row_generation`, and `require_unchanged` refuses. That fails closed. |

---

## Checked and clean (not findings)

- **candidate_trades publication order.**
  - `start.bin` and the tier files are written before pricing. Each candidate's trades are written before its manifest, and both before `catalog.bin`.
  - `finish` re-reads every acknowledged child under shared locks before it seals the catalog. `confirm` re-reads the catalog before `attempt.finish(Completed)` (`lib.rs:19037-19049`), so no Completed audit lacks a sealed catalog.
  - Paths are per (identity, globally unique attempt token), so two processes or a rerun never share a `create_new` name.
  - The `facts` `OnceLock` initialiser (`SliceFacts::of`) is serial, with no rayon inside. That rules out reentrant initialisation from a work-stealing rayon worker.
  - Tier indices come from serial `tier()` calls. Records in `par_iter` are keyed by deterministic rank and direction, so file names and catalog bytes do not depend on the schedule.
  - The state mutex is held across each child's fsyncs. That serializes publication by design ("the short publication section is serialized") and is not a correctness issue.
- **candidate_trades readers.**
  - `pinned` warm-path refusals release through the guard's Drop.
  - `TradeReader::page` unlocks on both arms.
  - `candidates_page` budget `metadata` TOCTOU is harmless, because the content is digest-checked against the pinned catalog.
- **candidatejson render.** Catalog digest pin, then a re-read of the catalog after the pages, then `read_attempt`. A mixed page is refused. `audit_completion` is read independently and labelled as such.
- **candidate_universe.**
  - The open takes an exclusive (writer) or shared (reader) flock across the header check and the scan.
  - Append holds the exclusive flock across `require_unchanged`, rows, `sync_data`, receipt and `sync_data`.
  - Write errors roll back with `set_len(original)` (`:6292-6299`, `:6316-6323`).
  - The dup'd lock fd shares the OFD, so dropping the original does not drop the lock.
  - `ensure_header` writes only into a zero-length file.
  - `audits` is a `HashMap` used only for get/insert, and no iteration order reaches bytes.
  - Lock order is Candidate ledger, dropped, then Base Evidence. These are never nested.
- **Boolean persistence and observation.**
  - Publication holds the exclusive `owner.lock` from `prepare` through `finish`. Readers take a shared owner lock for `Observation::open` and for each projection lease, so a reader never sees a `complete.bin` being written. The "busy" answer is documented (docs/27 table).
  - `ReadLease` field order unlocks before the nesting flag clears, and `compare_exchange(AcqRel/Acquire)` is correct.
  - `with_current_many` takes leases in sorted directory order, so there is no lock-order inversion between two compound projections.
  - Body and receipt shared locks are never upgraded or contended by an exclusive lock.
- **Boolean `finish_stored_month` order.** The attempt is marked Completed before `complete.bin` is written (`boolean_candidate_v1.rs:331-339`, `lib.rs:3765-3774`). This is the documented contract: docs/24 §"finish_stored_month" says "a completed attempt describes that completed computation snapshot, not … proof of a later summary append". A retry heals it unless search-1's torn file applies. Not counted.
- **boolean_candidate_grid.rs and boolean_candidate_reader.rs.** No file I/O, locks, statics or threads beyond the `Observation` they wrap.

---

<!-- conc-pass2/logs.md -->
### conc-pass2 / logs: readers of the live logs, the invocation journal and the pull audit journal (commit 331b05c)

**Verdict: 3 new findings (0 high, 1 medium, 2 low). The tail reader and the pull-journal reader are sound against their single writers. The new defects are in how the invocation journal's non-blocking locks meet the browser, and in blocking work that runs on Tokio workers. Pass-1 check: 7 findings CONFIRMED, 1 REFUTED as stated (xcut-1: its env-var mechanism is wrong; a narrower operator-configured residual remains).**

Method: source reading only. No cargo was run, nothing was edited, and no timing is claimed.

Slice:
- `crates/api/src/logs.rs`: `/logs`, `/logs.json`, `both_halves`.
- `crates/telemetry/src/tail.rs`, in full.
- `crates/api/src/operation_audit.rs`: the request middleware, `/backtest/audit.json` and `persisted_status`.
- `crates/cli/src/operation_audit.rs`, in full.
- `crates/api/src/audit.rs`: `look`, `page`, `append`/`appended`.
- `server.rs`: `audit_get`/`audit_html`/`audit_rows`, `served_log_dir`/`log_dir_from`, `LOG_DIR_ENV`.

Where a consequence lands in the page, I also read the front-end callers: `web/src/lib/sweep-admission.js`, `web/src/routes/backtest/+page.svelte`, `web/src/lib/InvocationAudit.svelte` and `web/src/lib/ask.js`.

---

## log-1 (medium): in-process contention on the invocation index makes `POST /backtest/run` return a busy 429 for a handler that never ran, and the backtest page then locks its Run control for the session, saying the launch "may have started"

**Where**
- `crates/cli/src/operation_audit.rs:522-557` (`begin`): `Flock::try_lock(...)`. The exclusive index lock is non-blocking and is held through `write_synced(&mut *index, &image)?`, which is `write_all` plus `sync_all`.
- `crates/cli/src/operation_audit.rs:599-609` (`read`): `Flock::try_lock_shared(index, ...)` is held across `index.sync_all()`. `page` (`:660-665`) calls `read` up to 32 times, and each call takes and releases that shared lock.
- `crates/api/src/operation_audit.rs:134-152` (`request_audited`): this runs before every one of the 21 `AUDITED` routes, `/backtest/run` among them.
  ```rust
  Ok(Err(why)) => {
      let mut refusal = failure(&why, false);
      if journal::is_busy(&why) {
          refusal.0 = StatusCode::TOO_MANY_REQUESTS;
      }
      return refusal.into_response();
  }
  Err(crate::detail::RunError::Saturated) => { ... refusal.0 = StatusCode::TOO_MANY_REQUESTS; ...
  ```
  The body is `{"schema_version":1,"refusal":..,"code":"invocation_audit_unavailable","handler_completed":false,"why":"The handler was not dispatched ..."}`. It has no `accepted` key.
- `web/src/lib/sweep-admission.js:30-44` (`sweepSubmission`): only `body.accepted === false` counts as a decisive refusal. Anything else returns `{ phase: 'unknown', ..., why: 'The launch response did not confirm acceptance or refusal. It may have started; do not submit it again.', confirmed: false }`.
- `web/src/routes/backtest/+page.svelte:2546` sets `unconfirmedSubmission = true` before the POST. `:2598` keeps it true when `!outcome.confirmed`. `pollSweep` (`:2446-2449`) then loops on "The previous launch response is unconfirmed ... no duplicate request will be sent". `sweepLaunchStop` (`sweep-admission.js:20`) disables launch while the flag is true. The flag is assigned only at `:2546`, `:2598` and `:2840` (Descend). No later poll clears it.

**Why it is wrong.** Two kinds of operation inside the one api process contend for the index lock:
- every audited request's `begin`, which takes the exclusive lock across an fsync;
- every `/backtest/audit.json` page read, which takes the shared lock 32 times, each across an fsync of the index.

`try_lock` does not wait, so an overlap is a hard refusal. cli1-2 reported this lock as it hits the CLI, and it judged the API's 429 harmless ("where a retry is safe"). For the launch POST that judgement is wrong. The page never retries `/backtest/run`, because `ask_` has no retry and the POST deliberately avoids `fetchWithBusyRetry`. It also reads the 429 as an *unconfirmed* launch, even though the server's own body says `handler_completed: false`, "The handler was not dispatched". The UI therefore makes two false claims:
- the run "may have started";
- the operator must not resubmit.

It then disables Run until the page is reloaded. The same thing happens when the 4-permit `detail` pool is full (`RunError::Saturated` → 429, same body). runs-4 shows parked admissions filling that pool.

The handler's own second `begin` (`sweeprun.rs:1582-1596`, `reserve_invocation`, Origin::Browser) hits the same lock. That path is answered correctly: `refused()` sends `{"accepted":false,...}` (sweeprun.rs:1740-1748). So only the middleware's refusal shape is misread.

**Repro (one browser, one api process).**
1. Open `/backtest` and expand "Saved run and request history". `InvocationAudit.svelte` now polls `/backtest/audit.json?limit=32` every 5000 ms. Each poll runs `page` → 32 × `read`, and each `read` holds the index's shared lock across `index.sync_all()`.
2. Press Run while one of those reads holds the shared lock. Alternatively, press it while another tab's 2-second `/live.json` or `/backtest/run.json` poll is inside its own `begin`, or while a `cli` sweep's `begin` holds the exclusive lock.
3. The middleware's `journal::begin` gets `WouldBlock` and answers 429 `invocation_audit_unavailable`, with `handler_completed:false`. No sweep was admitted.
4. `applySweepSubmission(429, body)` → `sweepSubmission` → phase `unknown`, `confirmed:false`. The page shows "It may have started; do not submit it again". Every 2 s `pollSweep` repeats "The previous launch response is unconfirmed". Run stays disabled until a reload.

How often this happens depends on fsync latency and on how many audited requests are in flight. The interleaving is deterministic once the windows overlap. I did not measure it.

**Minimal fix (either half closes it; both is better).**
- Server: serialise in-process callers of `begin` with a process-wide `Mutex<()>` taken before the flock, so in-process contention waits for one append plus fsync instead of refusing. Keep `try_lock` for the cross-process case only. In `read`, drop `index.sync_all()`, or move it outside the shared lock. A reader has no reason to fsync while it blocks writers.
- Client: in `sweepSubmission`, treat `code === 'invocation_audit_unavailable' && handler_completed === false` as a decisive pre-dispatch refusal (`phase:'failed', confirmed:true`). `invocation-audit.js:44` already parses exactly that shape.

---

## log-2 (low): a client disconnect drops the armed HTTP `Attempt` on the Tokio worker, which then takes a flock, writes and fsyncs synchronously, and the usual trigger is the browser's 15 s timeout on a slow server

**Where**
- `crates/api/src/operation_audit.rs:161-162`: `let id = attempt.id(); let mut response = handler.await;`. `attempt` lives in the middleware future across the handler's await.
- `crates/cli/src/operation_audit.rs:445-461` (`Attempt::drop`) → `self.finish(Phase::Cancelled, 0)` → `State::update` → `append` (`:324-335`): `file.try_lock()`, `write_all`, `sync_all`, `unlock`. All of it runs inline, with no `spawn_blocking`.
- The module's own comment at `api/src/operation_audit.rs:171-175` says this shape is a defect: *"refusing dropped the armed attempt, whose `Drop` then wrote `Cancelled`/0 synchronously on this Tokio worker"*. D-1445 fixed it only for the refused-terminal path, by moving the terminal into `run_owed`.

**Why it is wrong.** When a client goes away while its handler is pending, hyper drops the connection's service future. That drops `request_audited` at `handler.await`, and `Attempt::drop` then does blocking file I/O and an fsync on the async worker that was polling it. `ask()` in `web/src/lib/ask.js` aborts every request after 15 s. So the abort comes exactly when the server is already slow, for example a busy disk under a sweep. The cancellation then adds an fsync on that same slow disk to a Tokio worker. `#[tokio::main]` (api/src/main.rs:31) uses one worker per core, which is 4 on this box. A handful of timed-out audited reads can therefore stall every async worker at once. While they are stalled, `/health`, the pull ticker and every other route stop being polled.

**Repro.** Run a `range-all` sweep so the store disk is saturated. Open a page that reads a large `/trades.json` or `/candidate-trades.json`, both audited, and let it take more than 15 s. The browser aborts at 15 s. Hyper drops the future, and `Attempt::drop` runs `try_lock`, a 256-byte `write_all` and `sync_all` on the worker thread. Repeat from 4 tabs and the runtime has no free worker until those fsyncs return.

**Minimal fix.** Give the middleware a drop guard that owns the `Attempt` and moves it in its `Drop` into `tokio::task::spawn_blocking(move || drop(attempt))`. `Handle::try_current()` is available there. Alternatively, have `Attempt::drop` hand the terminal to a blocking thread whenever it detects a Tokio context. The `Cancelled` record is still written, just off the async worker.

---

## log-3 (low): `/logs.json` and `/logs` scan up to 2 × 4 MiB synchronously on a Tokio worker with no `detail` admission, and the backtest page polls one every 2 s per running sweep with a `run` filter that the other half can never match

**Where**
- `crates/api/src/logs.rs:164-207` (`logs_json`) and `:495-547` (`logs_page`) are `async fn`s. Each calls `json_over`/`page_over` → `both_halves` (`:325-384`) inline, which runs two `telemetry::tail` walks.
- `logs.rs:51` sets `SCAN_BYTES = 4 MiB` and `:144` sets `query.max_scan_bytes = SCAN_BYTES`, per half. `tail::walk_back` (`tail.rs:440-540`) reads blocks and JSON-decodes every line until it has `limit` matches or the budget is spent (`take_line` returns `false` for a non-matching record, so the walk continues).
- `web/src/routes/backtest/+page.svelte:1970-1972`: `ask_('/logs.json?limit=200&run=' + attempt)` runs on every 2-second `pollSweep` tick while a sweep runs (`:2416`, `:2473`).

**Why it is wrong.** A browser sweep's events go to the server half, and a CLI sweep's go to `<store>/logs/cli`. A `run=` query therefore always has one half with zero matches. That half's walk never fills its limit. It reads and decodes up to the full 4 MiB cap, or the whole set if smaller, on every poll. Every other bounded reader in this crate passes its file work through `crate::detail::run` (`detail.rs:100`, a 4-permit `spawn_blocking` door). These two routes bypass it, so neither the permit cap nor `Saturated` limits them. Several tabs, or one `/logs` page with a filter that matches nothing, put repeated multi-MiB synchronous decodes onto the async workers. While those run, the workers are not polling the HTTP accept loop, the pull ticker or the audited routes' futures.

**Repro.** Fill `<store>/logs/cli/events.ndjson` past 4 MiB, which any long `range-all` does. Press Run on `/backtest`. Every 2 s, `/logs.json?limit=200&run=<browser attempt>` walks the cli half to `hit_scan_cap = true`, a 4 MiB read and decode, inside `logs_json` on a Tokio worker. Open three more tabs on the same run and four workers each spend that decode every 2 s. Cost per poll is argued from the code (`READ_BLOCK` reads up to `max_scan_bytes`) and is not measured.

**Minimal fix.** Wrap the two `both_halves` calls in `crate::detail::run(move || ...)`, answering `Saturated` as 429 the way the other read routes do. Optionally, skip the half that cannot hold the run, since the attempt's origin says which sink wrote it.

---

## Pass-1 verification

| Pass-1 finding | Verdict | Reason (from the code at 331b05c) |
|---|---|---|
| telemetry-1 (medium): cli processes share `<store>/logs/cli` with no lock | **CONFIRMED** | `cli/src/main.rs:69` calls `cli::install_log()` for every verb. `cli/src/lib.rs:3061` resolves `log_dir_from(BRUTEX_LOG_DIR, store)` → `<store>/logs/cli` for every process. The only file locks in `telemetry/src/sink.rs` are `Mutex` (`:1115`, `:1225`, `:1241`); there is no flock. `tail.rs` `missing_between` (`span = newest.checked_sub(oldest)?...`) and the `since` early exit in `take_line` are unchanged and still assume one writer per set. |
| cli1-5 (low): same defect, plus the false doc | **CONFIRMED** | It duplicates telemetry-1. The doc sentence "No lock, no coordination, nothing to get wrong under concurrency" is still at `cli/src/lib.rs:2973-2976`. |
| xcut-1 (medium): with `BRUTEX_LOG_DIR` set, api and cli share one telemetry set | **REFUTED as stated** | The api does not read `BRUTEX_LOG_DIR`. `server.rs:17950` has `pub const LOG_DIR_ENV: &str = "BRUTEX_LOGS";`, and `served_log_dir` (`:18227-18233`) passes `std::env::var_os(LOG_DIR_ENV)` to `log_dir_from`. Only the cli reads `BRUTEX_LOG_DIR` (`cli/src/lib.rs:3061`). The report's premise, "both resolvers return `BRUTEX_LOG_DIR` verbatim", is false, and the cli's current success line (`lib.rs:3072-3076`) no longer tells the operator to set it "for both". **Residual (low):** an operator who sets `BRUTEX_LOGS` and `BRUTEX_LOG_DIR` to one directory, or who points `BRUTEX_LOG_DIR` at the directory the api resolved by default (`<workspace>/logs` under `.claude/launch.json`), does get xcut-1's mechanism. That requires deliberate configuration, not one exported variable. |
| cli1-1 (medium): a zero-length per-ID journal is unreadable forever and breaks paging | **CONFIRMED** | `begin` releases the index (`:557`) before `create_new` (`:558-563`) and `write_synced` (`:564`). `read` returns `Ok(Some(started))` only on `NotFound` (`:612`), and `bytes == 0` falls into `"invocation journal lost its indexed start"` (`:618-619`), which has no `BUSY` prefix. `page` collects with `?` (`:660-665`). The `/backtest/audit.json` panel in log-1 hits Repro B (a `begin` in flight) as a 503 with no retry. |
| cli1-2 (low): CLI sweep admission fails busy against browser polling | **CONFIRMED** | `try_lock` at `:522`, held across `write_synced` (`:556`). `try_lock_shared` at `:599`, held across `index.sync_all()` (`:608`). log-1 widens this to the in-process browser launch. |
| cli1-4 (low): rung boundaries lost under rayon because `CURRENT` is thread-local | **CONFIRMED** | `CURRENT` is a `thread_local!` (`:464`). `completed_boundary` does nothing when the slot is `None` (`:489-504`). `sweep_rungs` still runs `.par_iter()` → `one_rung` (`cli/src/lib.rs:15220-15222`) → `note_rung_finished` → `operation_audit::completed_boundary()` (`:13250`). |
| recovery-4 / server1-1 (low / medium): `pull.journal` append refuses instead of waiting | **CONFIRMED** | `audit.rs:1190` `Flock::try_lock(...)` → refusal text "another writer may be appending". `pullrun.rs:871` `tokio::spawn(run_chain(..))` runs one chain per feed, and each leg is `server::pull_spot` (`pullrun.rs:687`) → `recorded_fact` → `journal.append` (`server.rs:10616-10628`). Within one process, two descriptions of one file conflict under flock. |
| apicache "clean": `logs::both_halves` order is deterministic | **CONFIRMED clean** | `sort_by` is stable, with the tie-break `(at_unix_millis desc, seq desc)` (`logs.rs:340-345`). The served half goes first and the limit applies after the merge. |
| recovery "clean": `audit.rs` `look`/`page` unlocked reads | **CONFIRMED clean** | `look` takes one `metadata`. `page` reads `[start*256, end*256)` of whole records, bounded by that count. An appender's single 256-byte `O_APPEND` write sits at a 256-aligned offset inside one page, so `i_size` never exposes a half-copied record. A torn tail is reported (`Log::Held::torn`), and each record is CRC-checked into its own row. |

## Checked and clean (no finding)

- **`tail.rs` against a roll mid-walk.** One `open` plus one `fstat` per path, and dedup by `(dev, ino)`. Every interleaving is safe:
  - The current file read, then rolled: `.1` is the same inode and is skipped, and `.2` is the true next-older file.
  - The path opened between the rename and `reopen`: NotFound → `continue`, `first_file` stays true, and `.1` is treated as the newest file, so its tail is checked for a partial line.
  - A fresh empty current file: `len == 0` → `continue`, with the same effect.
- **`tail.rs` against the in-process writer mid-line.** Only the first file actually read gets `partial_tail`/`drop_fragment`. The sink only rolls between events, under its lock, so a non-newest file never ends mid-line from a live writer. The `since` early exit is sound for a single writer, because `ms` is clamped inside the lock. The api does not use `since` (`logs.rs::asked` sets no `since`).
- **`both_halves` honesty flags.** `hit_scan_cap` is OR-ed and `reached_oldest` is AND-ed. `missing` sums only the halves that can answer, and a half with no directory defaults to `reached_oldest = true`. None of this depends on scheduling.
- **`operation_audit::read` against a concurrent per-ID append.** The writer appends whole 256-byte records under the per-file flock. The reader works on a length snapshot: record 0, then the last record, then `sync_all`, then it re-measures and answers `BUSY` if the length moved. It never reads past its snapshot and never takes the writer's lock.
- **`operation_audit` ID allocation.** The exclusive lock covers the length, the tail check (`previous.id == id - 1 && Started`) and append plus fsync. A failed sync leaves a Started record, the next `begin` steps past it, and `read` reports it as unconfirmed. Nothing can reuse an ID.
- **`page` cursor under concurrent `begin`s.** `total` is measured once without the lock, and later IDs fall outside the window. `next_before` is the last returned ID, so paging is stable. The one exception is a zero-length per-ID file (cli1-1).
- **`State` mutex and poisoning.** `finish` on a poisoned mutex returns early with `armed` still true. `Drop` then retries once, fails the same way and reports it on stderr and in telemetry. No record is fabricated. `current_id` reads through poison, but only for the ID, which never changes.
- **`Restore` guard.** It restores the prior `CURRENT` on unwind, so a reused blocking-pool thread cannot keep a stale attempt.
- **`/audit` (`audit_html`).** It does one `look`, then `page` on the same count and path. A concurrent append can only add records past the count, and a failed read is shown as `UNREADABLE` rather than an empty table. The read is bounded at `PAGE_ROWS × 256` bytes, so its inline blocking read is negligible.

---

<!-- conc-pass2/census.md -->
### conc-pass2 / census: the census writer, the api census reader and cache, store catalog, checksum audit and repair publish, at 331b05c

**Verdict: 1 new finding (0 high, 1 medium, 0 low). Every pass-1 finding in this slice is confirmed. One pass-1 "checked and clean" claim is refuted by census-1.**

Slice: `crates/pull/src/ingest.rs` census section (`from_members_inner` 768-950, `record_all` 1202-1226, `read_census` 2909-2950, `CensusLock` 2990-3200, `install_census`/`write_appends`/`install_locked`/`publish` 3272-3366), `crates/pull/src/manifest.rs` load path (`open`, `open_image`, `walk_generations`, `walk`, `validate`), `crates/api/src/census.rs` (`read_vendor`, `sized`, `read_all`), `crates/api/src/server.rs` census cache (`census_now*`, `manifest_stamps`, `read_as_stamped`, `stamp_could_read`, `refuse_contradicted_absences`, 3528-4110), the cache's consumers in `api/src/pullrun.rs` (`rows_now` and the ticker), `crates/store/src/{catalog,checksum_audit,repair}.rs`, and `store/src/file.rs` where those call into it (`open_or_create`, `open_existing`, `open_existing_audited`, `checksum_inputs`). Source reading only. No cargo was run.

---

## census-1 (medium): an in-place census append moves the manifest's mtime BEFORE its bytes land, so `census_now` can cache the pre-append census (or a torn, "degraded" one) under the post-append stamp and serve it until the next manifest write

**Where**
- Writer: `crates/pull/src/ingest.rs:3313-3331` (`write_appends`), the slot write at 3327:
  ```rust
  file.seek(SeekFrom::Start(append.commit.offset))?;
  file.write_all(&append.commit.bytes)?;
  file.sync_all()?;
  ```
- Cache: `crates/api/src/server.rs:3630` (`let stamps = stamp(&site.store_root);`), 3662 (`let mut censuses = read(&site.store_root);`), 3687-3707 (keep under `stamps`). The key is `manifest_stamps` (3774-3800): `ManifestStamp::At { modified, changed }` from one `stat`, with no size and no inode.
- The invariant the cache relies on, stated at 3656-3660: *"THE STAMPS ABOVE ARE OLDER THAN THIS READ, AND MUST BE. A manifest installed between the two keys a newer census under an older stamp, which the next request's stamp no longer matches, so it reads again: stale for at most that one request."*
- Reader: `crates/api/src/census.rs:419-455` (`sized`): one lock-free `read_to_end` of the whole manifest. It takes no `.man.lock`.

**Why it is wrong**

D-0695's ordering argument holds for the whole-image install: `publish` writes and syncs a temp, then `rename`s it (ingest.rs:3356-3366), so any stamp that shows the new inode's times was taken after that inode's bytes were complete. It does not hold for the incremental path, which has been the normal path since the 424 GB fix (`install_census` takes `append_locked` unless repairing, virgin or upgrading). `write_appends` rewrites a 64-byte header slot in place. On Linux the buffered-write path updates the inode's timestamps **before** it copies the data into the page cache: ext4 `ext4_buffered_write_iter` → `ext4_write_checks` → `file_modified()`, then `generic_perform_write()`, both under the inode lock. The generic `__generic_file_write_iter` has the same order. A buffered reader (`filemap_read`) on ext4, btrfs or tmpfs takes neither the inode lock nor the folio lock to copy an up-to-date page. So for the duration of the copy, `stat` already reports the final mtime and ctime while `read` still returns the old slot bytes, or a half-copied slot.

Nothing moves the times after that. `sync_all` does not, and the slot write is the last write of the install. So the cache keeps a census that is older than the stamp it is filed under, and `*at == stamps` matches on every later request. This is not the documented "two writes inside one timestamp tick" gap (docs/06-limits.md, D-0686/D-0695). It does not depend on timestamp resolution at all, and multigrain timestamps do not close it.

Two outcomes, both cached:
- The read copies the header page before the slot copy. The census is clean at generation g and is missing the month or rows the append just counted.
- The read overlaps the copy. The newest slot fails its CRC-32C, `walk_generations` steps over it, and the census is `Held` with `degraded = Some(..)`. `stamp_could_read(Held, At)` is `true` (server.rs:3880-3884), so the "degraded / needs attention" census is **kept**. pull2-5 said this alarm lasted "that one response". Through the cache it lasts until the manifest is next written.

The persistence matters because of who reads the cache:
- `/store.json`, `/instruments.json`, `/verify.json`, `/audit.json` and the store page.
- `calendar_of::cached`, keyed by `CensusStamps::modified` from the same stamps. It derives and keeps a calendar from the stale months, which feeds `/calendar.json` and the `/gaps.json` peer vote.
- `pullrun::rows_now` (pullrun.rs:509). The press's 5-second ticker (pullrun.rs:947-955) is the reader most likely to land in the window, because it polls **while the legs are appending**. The same cached value is then read for `after` (968) and for the run's final `current_rows` (1006-1012). A pass that added rows can read `after == before` and count as a clean empty pass (`CLEAN_EMPTY_PASSES = 3`), and the finished summary's `current_rows - started_rows` undercounts.

`from_window` installs once per window fetched (one append, one slot write, per vendor request), so a backfill makes thousands of these windows per run.

XFS takes the I/O lock shared for buffered reads and exclusive for buffered writes, so there the read waits for the copy and gets the new bytes. The defect is filesystem-dependent: present on ext4, btrfs and tmpfs, absent on XFS. The operator's filesystem is UNVERIFIED.

**Repro (one api process; ext4 store; one Dhan leg of a press)**
1. Ingest thread W holds `dhan.man.lock` and is in `write_appends` for its last append. Its `pwrite` of the 64-byte slot has run `file_modified()`, so the inode's mtime and ctime are now T, and it has not yet copied the bytes.
2. The pullrun ticker (or any `/store.json` poll) R calls `census_now_stamping`. `manifest_stamps` stats `dhan.man` and gets `At{T, ..}`, the cache key misses, and `read_all` → `sized` → `read_to_end` copies the header page before W's copy. The census is generation g, or degraded if the copy overlapped.
3. W finishes the copy and `sync_all`, then the run ends. No further write touches `dhan.man`.
4. R: `read_as_stamped` returns true, and the cache stores `(stamps{T}, census@g)`.
5. Every later request stamps `At{T}`, hits, and serves generation g (or "degraded"). This includes the press's final `rows_now`. It stays wrong until any vendor's manifest is written again, which can be the next day's pull.

**Minimal fix**

Make a time move **after** the bytes are in the page cache. In `write_appends`, after the final slot `sync_all`, call `file.set_modified(std::time::SystemTime::now())` (or `File::set_times`). This `setattr` moves both mtime and ctime after the copy, so any stamp taken inside the window differs from the final one, and the next request re-reads. The alternative is to have `census::sized` take `.man.lock` shared with `try_lock_shared`, and on `WouldBlock` serve the read uncached. That also closes pull2-5's torn read. But the ingest holds the lock for a whole run, so every request during a run would go uncached.

---

## Pass-1 verification (findings that touch this slice)

| Pass-1 item | Verdict | Reason, from the code at 331b05c |
|---|---|---|
| **pull2-1** (medium): `CensusLock::take` runs unlocked on transient or space errors | CONFIRMED | ingest.rs:3118-3130 is unchanged. Only `PermissionDenied` and `IsADirectory` refuse. Every other open error on a lock path whose `symlink_metadata` is a regular file (EMFILE, ENFILE, ENOMEM), or is `NotFound` (ENOSPC on inode creation while the census exists), reaches `_ => return Ok(Self { _held: None })` at 3129. Both `read_census` and `install_census` then run unserialised, and `install_locked`'s fixed temp `<vendor>.man.writing` (3345) is shared by `File::create` in two runs. |
| **pull2-2** (low): `record_all` drops a whole batch on one refused row | CONFIRMED | ingest.rs:1215-1219: `Err(why) => return Some(why)` returns before `install_census`, which discards every append already collected. The premise holds: `from_rows` writes bars outside the census lock, and the api's rolling path collects every group's `pending` and calls `record_held` once (server.rs:12819). |
| **pull2-5** (low): an in-place slot write lets a lock-free reader see a torn slot and report "degraded" | CONFIRMED, and understated | The write is at ingest.rs:3326-3328 and the reader at census.rs:419-455 takes no lock. The finding said the false alarm lasts one response. census-1 shows the cache keeps that degraded census under the post-write stamp, so it persists until the next manifest write. |
| **xcut-2** (low): census install says "not published" after the rename published | CONFIRMED, plus one unreported case | ingest.rs:3364-3365: an error after `fs::rename` still becomes "could not be published", and the caller (941-946) words it as "the census that counts them was not published". **Also:** `write_appends` loops over a batch and commits each append (slot write plus `sync_all`) before the next. If append *k* fails (for example ENOSPC growing the file for its entry), appends 1..k-1 are durable and published, yet `append_locked` reports "could not be appended to after {n} entry write(s)" and `from_members_inner` reports all `done.counted` slices as unpublished. The next run self-heals: `count` sees the published rows and re-appends only the rest. So the cost is a false receipt and an Error event, not lost data. |
| **store2-1** (low, latent): a concurrent identical `repair::publish` loses with a misleading refusal | CONFIRMED | repair.rs:261 `if exists(&reservation)?` is a check-then-act against the `create_new` at 353-361, which maps `AlreadyExists` to `Io{.., publication_may_be_visible: false}`. The Reused branch's `RevisionReader::open` reads the receipt (line 162) before it takes any lock, so a live publication reads as `Incomplete`. Still latent: grep finds no caller of `repair::publish` or `RevisionReader` outside `store/src/repair_tests.rs`, `store/tests/` and `flock_tests.rs`. |
| **store1-1** (low): `open_existing` reads with no lock when `.lock` is absent | CONFIRMED (not in this slice; checked only where the slice calls it) | file.rs:1398-1406 still answers `Err(why) if why.is_absent() => None`. The checksum-audit door is **not** exposed: `open_existing_audited` (file.rs:1433-1449) opens the lock with `open_regular` and has no absent-lock arm, and `checksum_inputs` refuses a file with no held lock. |
| apicache "checked clean": `census_now_stamping` "never serves a stale answer" | REFUTED | See census-1. The argument assumes a write moves the stamp after its bytes are readable. That is true for the rename install and false for `write_appends`. |
| apicache "checked clean": `census::sized` never sees a half-written manifest because the manifest is installed by temp+rename | STALE (already covered by pull2-5) | Since the incremental install, the common path is an in-place slot write, not a rename. |

---

## Checked and found sound (not findings)

- **Census RMW serialisation.** In both `from_members_inner` (805) and `record_all` (1204), the lock is taken before `read_census` and declared before the census so it drops last. `install_locked` and `append_locked` take `&CensusLock`, so neither can be reached without the guard. Intra-process exclusion also holds: every `take` is a separate `open`, and therefore a separate open file description, so two threads conflict on `flock`.
- **Lock ordering.** The spot path takes the census lock, then each month's bar lock. The rolling F&O path takes the bar lock, releases it, then takes the census lock. Every acquisition is `try_lock`, so there is no wait cycle. A collision is a named refusal (`lock_refusal`).
- **`write_appends` crash points.** Each append runs entry write → `sync_data` → slot write → `sync_all`. Dying before the slot leaves bytes past `n_valid`, which `walk` ignores (`zip(0..n_valid)` over `chunks_exact`) and the next append overwrites. Dying mid-slot leaves the other slot valid, so the next load comes back degraded and the next ingest repairs it by a whole-image install. An entry `sync_data` EIO leaves the counter unpublished, and the next run rewrites the same offset.
- **Lock-free reader versus append ordering.** `read_to_end` reads from offset 0, so the header is copied before the entries. Each entry is written and synced before the slot that counts it, so a reader that sees slot g+1 also sees entry N+1. The only tear is inside the slot itself (pull2-5 / census-1).
- **Whole-image `publish`.** It runs write → `sync_all` → `rename` → directory `sync_all`, with the fixed temp name serialised by the lock (except pull2-1). A temp left by a crash is truncated by the next `File::create` under the lock, and nothing lists `manifest/`.
- **`read_census`.** Only `NotFound`/`NotADirectory` give genesis. Other errors refuse. A file shorter than the header is refused, and only an empty file is genesis. No writer creates an empty `<vendor>.man` (only `rename` of a synced temp), so a crash cannot produce one.
- **Census cache install race.** A slow older reader overwriting a newer entry is re-read on the next request (stamps differ). The double-checked install keeps the canonical `Arc`. The mutex is read through poison, and nothing is held across the read.
- **`store::catalog::walk`.** It lists directories only and opens no file. Output is `sort_unstable` + `dedup`. A month created mid-walk is caught by `open_or_create`'s order: the lock is created and `flock`ed **before** the `.bin` is opened, so a reader of a freshly listed month gets `Locked`, not a zero-length header. `bar-revisions-v1/` sits outside `bars/`, so repair artefacts are never catalogued.
- **`checksum_audit`.** The shared month lock is held for the `AuditedBarFile`'s lifetime. Generations (dev, ino, len, mtime/ctime ns, nlink=1) for data, sidecar and lock are compared fd against path before and after the full scan and around every `read_record`. A cooperating writer's `open_or_create` opens the lock with `create(true)` and no truncate, which moves no time, and then fails `try_lock`.
- **`repair::publish` versus ingest.** The source is shared-locked through the whole revision write, so `open_or_create`'s exclusive `try_lock` refuses, and the reverse order refuses `Locked`. The revision directories are synced to the root before and after the receipt.

---

<!-- conc-pass2/equity.md -->
### conc-pass2 / equity: equity ingest (cash_session_cache, masters, fnowork/work resume, mastersrun + reload, F&O universe walks)

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

---

<!-- conc-pass2/lifecycle.md -->
Verdict: shutdown handles exactly one signal and stops exactly one of the three kinds of background vendor work. 3 findings (0 high, 0 medium, 3 low). All three are new triggers of classes already filed (lost pull receipt, unlocked shared telemetry sink); none are duplicates of a prior report. Pass 1 re-verified: 9 confirmed, 1 partially refuted (xcut-1).

Slice: process lifecycle at 331b05c. Covered: `crates/api/src/main.rs`, the startup order in `server.rs` `run_in_over` (18347-18573: bind, store root, `sole_server`/`serve.lock`, telemetry install, `Assets::new`, `Site::serving`, browser open, `autopilot::fly` spawn, `recovery::resume`, `serve`, abort), `serve`/`serve_limited` (17090-17130), `ServeLock` (17478-17760), every production `tokio::spawn`/`spawn_blocking` in `crates/api/src` (server.rs:10938, 18536; pullrun.rs:871, 949; recovery.rs:534, 614, 643, 663, 684, 724, 808, 1289; sweeprun.rs:1917, 2052, 3136; detail.rs:172, 201), and `crates/cli/src/main.rs` plus `run_durable` (cli/lib.rs:2060-2128). I read source only and did not run cargo. Versions are from Cargo.lock: tokio 1.53.1, axum 0.8.9.

---

## lifecycle-1 (low): shutdown aborts only the autopilot. A `/pull/run` press and a recovery drive are neither signalled nor awaited, so runtime teardown drops them mid-leg after `serve.lock` is released, and each landed leg's pull-journal record is lost

**Where**
- crates/api/src/server.rs:18536-18556 (serve arm):
  ```rust
  let flying = tokio::spawn(autopilot::fly(Loaded::clone(&site)));
  if let Err(why) = crate::recovery::resume(Loaded::clone(&site)) { ... }
  let code = stopped_over(serve(listener, audited_router_serving(site, front, bound_addr), shutdown).await, clean);
  ...
  flying.abort();
  code
  ```
  `one_server` (the `ServeLock`) drops when this arm ends.
- crates/api/src/server.rs:10938 (`pull_run`): `let _flying = tokio::spawn(crate::pullrun::conduct(Loaded::clone(&site), legs));`. The handle is dropped, and the task is detached from the HTTP connection.
- crates/api/src/recovery.rs:684 and 724: `let _task = tokio::spawn(drive(site, id, ..));`. The handle is dropped.
- crates/api/src/pullrun.rs:683-695: each press leg is `crate::server::pull_spot(..).await` (or `pull_fno`). That calls `broker_answer` (server.rs:6520), which runs `broker_run(..).await` at 6534 and only then appends the journal record (`recorded_fact(journal, &record)`, 6548/6577).
- crates/api/src/main.rs:31: `#[tokio::main]`. The runtime drops after `run` returns and `note_exit` has logged.

**Why it is wrong.** Graceful shutdown waits only for HTTP connections. A press returns 202 at once, so after Ctrl-C `serve` returns without waiting for its `conduct`. Nothing on the shutdown path touches the press or a recovery drive. `site.run.stopping` is not set, the autopilot epoch is not bumped, and neither task's handle is kept, so neither can be awaited or aborted. Both keep running on the worker threads while the arm releases `serve.lock` and `main` logs "exited cleanly". The runtime drop then cancels them at their next `.await`, usually the next instrument's vendor request inside `broker_run`. The bars from instruments already landed are durable, but `broker_answer` never reaches its append. The pull journal then has no record of a pull that wrote data.

This is the hunt-api-1 / autopilot-4 outcome reached by a third route. It is not covered by them:
- autopilot-4's proposed fix (pause, then await `flying` with a bound) would still drop these tasks, because nothing holds their handles.
- server1-2 covers the opposite case. A hand `/pull/spot` in flight *blocks* shutdown. The same walk started from the Pull page's press is *abandoned* without a word.

`pullrun::Finisher` (pullrun.rs:590) only writes the in-memory slot, which dies with the process.

**Repro.**
1. `api serve`. POST `/pull/run` with legs for one feed and a month of the tracked universe. It answers 202.
2. Wait until `/pull/run.json` shows the leg's `doing` past its third instrument, then press Ctrl-C once.
3. `serve` returns immediately (no connection is open), the banner ends, and the process exits 0.
4. The store's manifest for that month has gained rows for instruments 1..k. `/audit` (the pull journal) has no record for that leg. The same happens with an active recovery plan (`recovery::start`, or one resumed at boot by `recovery::resume`). Its in-flight attempt is left InFlight and charged to its budget, which recovery.rs:805-806 calls intentional, but the landed-leg receipt is missing for the same reason.

**Minimal fix.** Keep the handles. Store the press's `JoinHandle` (and the recovery drive's) on `Site` beside the run slot. In the serve arm, after `serve` returns and before the arm releases `serve.lock`:
- set `site.run.stopping = true` and call `site.autopilot.pause()`. `broker_run` checks the epoch per instrument (server.rs:7840), and `run_chain` checks `stopping` per leg (pullrun.rs:757), so each walk ends at an instrument boundary and journals its partial run;
- then `tokio::time::timeout(bound, handle)` each of the three tasks, and `abort()` only on expiry.
- Drop `one_server` explicitly after that, not at the end of the arm.

---

## lifecycle-2 (low): only SIGINT is handled. SIGTERM and SIGHUP end the server with no shutdown path at all, and the exit event that main.rs documents as written is absent

**Where**
- crates/api/src/main.rs:36: `api::server::run(&args, Box::pin(tokio::signal::ctrl_c())).await`. This is the only signal source. `grep -rn "signal::unix\|SIGTERM\|SIGHUP" crates/api` finds nothing.
- crates/api/src/main.rs:102-105 (doc of `note_exit`): "**Written** — exit `0` or `1` after a successful bind: a served process stopping, cleanly or on an accept failure. ... These are the two the 64 MiB window exists to keep."

**Why it is wrong.** SIGTERM is the default of `kill`, `pkill`, `systemctl stop`, `docker stop` and most IDE and preview stop buttons. SIGHUP arrives when the terminal running `api serve` is closed. Both keep their default disposition, which terminates the process at once:
- `serve`'s graceful drain never runs;
- `flying.abort()` never runs;
- `stopped_over`'s DEGRADED event never runs;
- `note_exit` never runs.

So a server stopped the ordinary way leaves a log that looks exactly like one that was SIGKILLed or crashed. The exit record the doc promises for "a served process stopping" is missing, and so is the DEGRADED exit (`stopped_over`, server.rs:17851-17865) that D-0026 made non-zero so a monitor could see it. A tick, hand pull or press in flight loses its journal record the same way as in autopilot-4, lifecycle-1 and hunt-api-1, with no chance of the pause-then-await fix those reports propose, because no code runs. The kernel releases `serve.lock` and the execution lease, so nothing wedges. The defect is the missing record, not a stuck store.

**Repro.**
1. `BRUTEX_AUTOPILOT=run api serve`. Wait for `/autopilot.json` `phase: running`.
2. Run `kill <pid>` (SIGTERM), or close the terminal window.
3. Read `/logs` (or the `events.ndjson` file) after a restart: there is no `api.main` "exited ..." line and no `api.server` stop line. The landed month has bars and no `/audit` record.
4. Repeat with Ctrl-C. The `api.main` line is present.

**Minimal fix.** Make the shutdown future the first of `ctrl_c()`, `signal(SignalKind::terminate())` and `signal(SignalKind::hangup())`. tokio's `signal` feature is already enabled in the workspace manifest. Route all three through the same graceful path, with the lifecycle-1 / server1-2 bounded drain, so `note_exit` runs.

---

## lifecycle-3 (low): two `api serve` processes on two different stores, both started from the checkout, write one unlocked telemetry set. This is the configuration the serve-lock refusal tells the operator to use

**Where**
- crates/api/src/server.rs:18266-18278 (`log_dir_from`):
  ```rust
  if let Some(named) = named { return PathBuf::from(named); }
  match cwd {
      Some(cwd) if is_workspace_root(cwd) => cwd.join("logs"),
      _ => telemetry::dir_beneath_store(store_root),
  }
  ```
  The directory depends on the working directory, not on the store that `serve.lock` admitted.
- crates/api/src/server.rs:17706-17712 (`take_serve_lock` refusal): "... A different port is not a second store. Stop the other instance, **or point this one at another BRUTEX_STORE**."
- `.claude/launch.json`: `BRUTEX_COMMIT=$(git rev-parse HEAD) exec cargo run --release -p api -- serve`, run from the workspace root. So cwd is the root and the log goes to `<checkout>/logs`.
- crates/api/src/main.rs:126-129 already concedes the sharing, but only as a sequence: "the sink directory is shared — every `api` run on the machine appends to the same file". It adds `pid` to the exit event for that reason.
- The sink has no cross-process guard. `crates/telemetry/src/sink.rs` takes no file lock (its only `try_lock` hits are on its in-process mutex, 1712/1792), and `telemetry::install` refuses only a second sink in one process. crates/api/src/logs.rs:64-66 serves `/logs` from `telemetry::global()`'s directory.

**Why it is wrong.** `serve.lock` (one server per store) is the only startup exclusion, and its refusal sends the operator to a second store. The second store gets its own lock, but both processes resolve the same `<checkout>/logs` and each runs its own `Sink`, with its own `seq`, `last_at`, byte count and `reserved_run`, against one `events.ndjson`. That is the multi-writer corruption telemetry-1 describes for cli against cli:
- `seq` forks, and `Tail::missing` stops reporting real loss;
- `reserve_run_id` hands both servers the same run ids (server.rs:6953 `claim_run`), so `/logs?run=N` merges two servers' pulls into one story;
- each process's `roll` renames the other's live file, so retention halves and the newer events of one land in `.1`.

On top of that, `/logs` on the server for store A lists store B's pulls, refusals and credential halts as its own, because the page reads the directory, not the store. This trigger needs no `BRUTEX_LOG_DIR`/`BRUTEX_LOGS` variable at all, unlike xcut-1 (see below).

**Repro.**
1. In the checkout: `BRUTEX_STORE=/s/a cargo run --release -p api -- serve 127.0.0.1:8080`.
2. Start a second instance with the store unchanged. It is refused with "point this one at another BRUTEX_STORE".
3. Following that text: `BRUTEX_STORE=/s/b cargo run --release -p api -- serve 127.0.0.1:8081`. Both print `log: <checkout>/logs/events.ndjson`.
4. Make both emit, for example by opening each `/autopilot.json` and pressing Pause/Resume on each. `events.ndjson` now holds two interleaved `seq` sequences.
5. `GET http://127.0.0.1:8080/logs` shows 8081's `autopilot paused` events.

**Minimal fix.** Either key the default log directory by the admitted store (`telemetry::dir_beneath_store(&one_server.root)` whenever `BRUTEX_LOGS` is unset, dropping the cwd probe), or implement the sink lock telemetry-1 and xcut-1 propose (`try_lock` on `<dir>/.sink.lock` held for the sink's life). With the lock, the second server reports `log: NOT WRITABLE — <dir> is already being written by another process` on its banner instead of corrupting the first server's log.

---

## Pass-1 verification (findings touching this slice)

| id | verdict | reason (code at 331b05c) |
|---|---|---|
| hunt-api-2 | CONFIRMED | main.rs:31 is still `#[tokio::main]` with no `shutdown_timeout`. `one_server` drops at the end of the serve arm (server.rs:18556-18557), before the runtime drop that waits for `spawn_blocking` sweeps (sweeprun.rs:1917/2052/3136). |
| autopilot-4 | CONFIRMED | server.rs:18555 `flying.abort();`. Nothing pauses or awaits it first. |
| server1-2 | CONFIRMED | server.rs:17121-17127: `with_graceful_shutdown(async move { let _ = shutdown.await; })`. Nothing on the shutdown path bumps `site.autopilot` epoch. tokio's SIGINT registration (on first poll of `ctrl_c()`) is never undone, so a second Ctrl-C has no effect. |
| server2-1 | CONFIRMED | assets.rs:681-700: `root` and `build` are computed once in `Assets::new`, which `run_in_over` calls once (server.rs:18422). |
| server2-2 | CONFIRMED | server.rs:17657-17673 return `ServeLock { held: None, root: key }` for an in-process second serve. Its `Drop` (17579-17586) removes the key the live holder owns. Reachable only in-process, as filed. |
| hunt-api-6 | CONFIRMED | server.rs:18189-18198: `.spawn().map(\|_\| ())`. The `Child` is dropped unwaited and nothing reaps it. |
| hunt-api-7 / runs-1 (ticker part) | CONFIRMED | pullrun.rs: `ticker.abort()` runs only after the pass loop. An unwinding `conduct_with` leaves the ticker running, and `abort()` cannot stop a poll already inside `rows_now`. |
| runs-1 (Finisher part) | CONFIRMED | `progress.finished` is set inside `with_progress` and the lock is released before `_finisher` drops at function exit. A `pull_run` on another worker can claim in that gap (server.rs:10920-10933), and `Finisher::drop` then sees the new run's `finished == None`. The window is narrow, with no await, but it exists. |
| telemetry-1 / cli1-5 | CONFIRMED | cli/main.rs:69: `cli::install_log()` runs for every verb before dispatch, and `run_durable` (cli/lib.rs:2097-2099) takes the lease only for sweep verbs. |
| xcut-1 | PARTIALLY REFUTED | The repro is "`BRUTEX_LOG_DIR=/x` exported for both". The api never reads `BRUTEX_LOG_DIR`: its variable is `BRUTEX_LOGS` (server.rs:17950 `pub const LOG_DIR_ENV: &str = "BRUTEX_LOGS";`, used at 18229). With only `BRUTEX_LOG_DIR` set, the api writes `<cwd>/logs` or `<store>/telemetry` (18274-18277) and the cli writes `/x`, which are different sets. The unlocked-sink hazard is real (sink.rs has no file lock), but between api and cli it needs `BRUTEX_LOG_DIR` and `BRUTEX_LOGS` to name the same directory, or `BRUTEX_LOG_DIR` to equal the api's default. The D-text at docs/05-decisions.md:18916 ("unless `BRUTEX_LOG_DIR` is set for both") is itself wrong about the api's variable, which is a doc defect. lifecycle-3 above is the trigger that needs no variable. |

## Checked and not reported

- **Lost panics in detached tasks.** `fly` (server.rs:18536), the press `conduct` (10938) and recovery `drive` (recovery.rs:684/724) all drop or never poll their `JoinHandle`. In the release profile (`panic = "abort"`, Cargo.toml:185) a panic aborts the process, so nothing is lost silently. In `dev`, which the Cargo.toml:127-140 comment says is what the IDE Run button builds, a panic would unwind into the unread handle. `fly` would then leave `/autopilot.json` frozen at its last phase, and `admit_resume` (autopilot.rs:3671) would answer Clear with no task left to read the flag. I found no reachable panic site in `fly`/`round`/`tick`, `pullrun` or `recovery`: clippy denies unwrap, expect, panic and indexing; the shifts are guarded (autopilot.rs:506-516, 1262-1272); month arithmetic is bounded (2381). So this is not reported. `drive` catches its worker's `JoinError` (recovery.rs:829). `conduct`'s chains are awaited and `JoinError` is noted (pullrun.rs:888-899). Sweep closures carry `TaskFinisher`, armed before `spawn_blocking`.
- **Startup order.** The bind comes first and is dropped unserved on every refusal. `sole_server` runs before telemetry, masters, `Site`, the browser and every writer, and refuses a missing root rather than creating one (17628-17640). The lock is taken through the canonical root, and every later child uses `one_server.root` (18403). The lock is stamped after it is held, and a failed stamp is a refusal (17738-17742). `fly` waits `GRACE_SECS` before any vendor work, so spawning it before `recovery::resume` does not race the resume's `claim`.
- **Ctrl-C registration window.** `tokio::signal::ctrl_c` is an `async fn` and registers on first poll, which happens inside `axum::serve`. Until then SIGINT has its default disposition, so a Ctrl-C during `Site::serving` or `recovery::resume`→`seeded` kills the process outright. That is ordinary crash semantics, and the journals' crash behaviour is filed separately (recovery-2).
- **Browser child and lock inheritance.** `open_in_browser` spawns after `serve.lock` is held. Rust opens files `O_CLOEXEC`, so the child does not inherit the lock descriptor, and D-0693's explicit unlock covers the fork-before-exec window.
- **cli main.** It installs no signal handler, so SIGINT/SIGTERM kill it outright. The execution lease is a kernel flock and is released. The operation-audit start without a terminal reads "unconfirmed" by design (operation_audit.rs:79, 98-99), not "running". The audit terminal is written before `deliver`. Output is buffered and written once with `write_all`, and a closed stdout is handled (v53-2). The cli has no production `std::thread::spawn` (every hit is under `#[cfg(test)]`). `thread::scope`/rayon sites join before `run` returns, so `main` never returns over a live writer thread. No `process::exit` or `abort` exists in either binary's production path.
- **`ServeLock` poison paths.** `take_serve_lock` recovers a poisoned `serving_roots` with `into_inner`, while `Drop`/`release_root` skip removal on poison. Poisoning needs a panic inside a `BTreeSet` insert/remove, which is not reachable.

---

<!-- conc-pass2/poison.md -->
### conc-pass2 / poison: lock poisoning, panics under locks and mid-write, Drop impls that do I/O

Verdict: 1 finding (0 high, 0 medium, 1 low). No reachable panic exists inside any production critical section I read. The key fact for this slice is that the shipped binary is built with `panic = "abort"`. Every poison branch in production code is therefore dead in that binary. The unwind-reliant recovery code (Drop guards, `JoinError` handling) runs only in dev and test builds, and several pass-1 "sound" verdicts silently assume unwinding.

Commit 331b05c. I read the source only and did not run cargo. Slice: workspace-wide, non-test code. It covers every `Mutex`/`RwLock` `.lock()`/`.read()`/`.write()` and its poison policy, panics while a lock is held or a file is half-written, and `Drop` impls that do I/O.

## The profile fact everything below depends on

- `Cargo.toml:174-185` `[profile.release]`: `overflow-checks = true` and `panic = "abort"`. `[profile.dev]` (`:127`) sets no `panic`, so dev unwinds, and `cargo test` always unwinds.
- The operator's run configuration is `.claude/launch.json`: `exec cargo run --release -p api -- serve`. `README.md:99` says the same, `cargo run -p api --release`.
- The code base knows this. `sweeprun.rs:936-950` says *"`overflow-checks = true` is set for `release` as well as `debug`, and `panic = "abort"` is set beside it, so [a crafted year] did not return a refusal -- it called `abort()` and took every other in-flight request on the process with it."*

Consequences in the shipped binary:
1. A `PoisonError` can never be observed. `unwrap_or_else(PoisonError::into_inner)`, `.map_err(|_| "... poisoned")`, `if let Ok(..)` and `.ok()?` all behave the same: the poisoned arm is unreachable.
2. A panic anywhere is a process crash at that instruction. No `Drop` runs, no guard unlocks (the kernel releases OFD/flock locks at exit), and nothing is flushed. "Panic while a file is half-written" is exactly the crash-at-line case pass 1 already analysed per writer. It adds no new on-disk state.
3. Poison handling therefore matters only in a dev build (`cargo run -p api` without `--release`) and in tests.

## poison-1 (low): unwind-only panic containment is documented as if it held in the shipped server; in the release binary one panic in any task aborts every concurrent writer

**Where**
- `crates/api/src/pullrun.rs:574-582` (`Finisher` doc): *"A task that panicked ... would leave that `None` in place forever ... `Drop` runs on the panic path, so the slot is released on every exit"*.
- `crates/api/src/pullrun.rs:1053-1066` (`note_dead_chain` doc): *"A panicking task never reaches the code that clears its own fields ... measured 2026-08-20 ... The panic itself goes to standard error through the panic hook and never reaches `telemetry`, so this is the only surface that can say it happened."* The caller is `pullrun.rs:885` `Err(dead) => { note_dead_chain(..) }` after `tokio::spawn(run_chain(..))` at `:871`.
- `crates/api/src/sweeprun.rs:1440-1462` (`Applied` doc): *"MEASURED by attacking the route: a panic there left every knob set for the LIFE OF THE PROCESS ... `Drop` runs during unwinding, so this holds on every exit"*.
- `crates/api/src/sweeprun.rs:1265-1284` (`TaskFinisher::drop`) and `crates/cli/src/operation_audit.rs:445-462` (`Attempt::drop`): `let phase = if std::thread::panicking() { Phase::Failed } else { Phase::Cancelled };`
- `crates/api/src/calendar_of.rs:128-131` and `:205-229`: *"A leader that panics abandons its flight, and each waiter then tries again"*.

**Why it is wrong**

Each of these is correct in a dev or test build, which is where the measurements cited in them were taken. None of them can happen in the binary the operator runs. With `panic = "abort"`:
- A `tokio::spawn`ed chain that panics does not yield `Err(JoinError::Panic)`. The process aborts, so `note_dead_chain` is unreachable for panics and only fires on cancellation.
- `std::thread::panicking()` is never true inside a `Drop`, because `Drop` does not run on a panic. The `Phase::Failed` arm of both audit guards is dead in release.
- The "knobs left set for the life of the process" outcome that `Applied` guards against cannot occur, because the process does not survive the panic.

The comments and the pass-1 verdicts that rely on them describe per-task containment. What actually ships is whole-process termination. The blast radius is every other in-flight request and writer: a sweep mid-`append`, a pull leg mid-ingest, an index-stop journal, the autopilot tick. The serve lock goes with them. §3 rule 6 (honest limits) asks for this to be stated. As written, a reader concludes that a pull-chain panic costs one feed's chain. It costs the server.

**Repro**

Use the `launch.json` configuration (`cargo run --release -p api -- serve`). Start a sweep (`POST /backtest/run`) and a pull (`POST /pull/run`). Any panic inside one pull chain then aborts the process. Overflow-checks panics are the reachable class: `note_dead_chain`'s own doc records that a chain panic was measured on 2026-08-20, and sweeprun.rs:936-950 records an earlier network-reachable overflow. After the abort:
- `/pull/run.json` never shows the "stopped abnormally" sentence. The process is gone, and pull checkpoints are memory-only (`pullrun.rs:600-602`).
- The sweep's operation-audit record stays at `Started`/`Progress`, never `Failed`. Its `sweep_evidence` lifecycle stays `Running`. Both are the documented SIGKILL shape, not the "Failed while unwinding" shape that pass-1 cli1 recorded as verified.

In a dev build the same panic is contained to its chain, exactly as the comments say. The difference exists only in the profile no test runs, which is the trap `sweeprun.rs:944-948` already describes for one route.

**Minimal fix**

Keep `panic = "abort"`; it is a deliberate choice and the right one for a store writer. Correct the five comments to say that the guards cover cancellation in every build and panics only in dev and test builds, and that in release a panic is a process crash recovered by the crash paths. Alternatively, if per-chain containment is genuinely wanted in production, that is a profile decision for `docs/05-decisions.md` (`panic = "unwind"` in release), not something the code can provide on its own.

## Inventory: every production lock and its poison policy

I found no lock held across `.await`. Every std guard in async code is block-scoped, and a `!Send` guard across an await would not compile in the spawned futures. I found no lock-order cycle. The orders I traced are listed below. Policies: **R** reads through (`into_inner`), **F** refuses by name, **S** silently skips (`if let Ok` / `.ok()?`).

| Lock | Sites | Policy | Held across |
|---|---|---|---|
| telemetry `Sink.inner`, `last_error` | sink.rs:1115, 1225, 1241, 1246, 1308 | R | file append (`target.append`), roll (rename); stderr `report` is after `drop(guard)` |
| store `BarFile.verified` | file.rs:1832, 1920, 2367 | R, forgets first | `pread` + CRC of one block |
| pull governor `Arc<Mutex<Governor>>` | http.rs:682, 3184-3196, 3709-3735, 3816-3915, 5086-5254; server.rs:8606, 9765 | R | `reserve` arithmetic is checked (rate.rs:940-962); `record_throttled` emits telemetry (a file append) under it |
| api `site.budgets` (outer) | server.rs:5185 (S, `.ok()?`), 8585 (F), 9761 (S) | mixed | the inner governor lock (order budgets → governor, never reversed) |
| api `site.run` | pullrun.rs:526, 548, 567; recovery.rs:553, 563, 580, 1683; autopilot.rs:3164; server.rs:10924, 10964, 10986 | R | `recovery_control::stop` fsyncs at 10986 (known runs-3/recovery-5) |
| api `site.recovery_active` | recovery_control.rs:28, 37 (R); 46, 59, 82 (F) | mixed | `persist` / journal open and fsyncs (F sites) |
| api `site.sweep` | sweeprun.rs:1258, 1281, 1685, 1695, 2093 (R); booleanlaunch.rs:280, indexstoplaunch.rs:323 (F) | mixed | O(1) clone/assign only |
| api `ADMISSION: Mutex<()>` | sweeprun.rs:1681 | R | the whole `prepare`: lease flock, audit begin with fsync (known W1-api6-2 design) |
| api `site.parsed` RwLock | server.rs:5407 (R), 5491 (R) | R | a three-field move |
| api `site.reload_lock: Mutex<()>` | server.rs:5442 | F | the whole masters parse `universe(masters)` |
| api `site.census` | server.rs:3638, 3693 | R | lookup or Arc swap only; the read is outside the lock |
| api `store_wire::Cache` | store_wire.rs:55 | F (test `poisoned_cache_refuses_instead_of_serving_stale_bytes`) | `build()` (body encoding) |
| api `detail::Cached<T>` (TRADES, FRONTIER, PARENTS, LEDGER) | detail.rs:583, 615 | R | `refresh` (flock shared, incremental index) and `f` |
| api static page caches (boolean*, candidate, expressionsearch, indexstop*) | booleanjson.rs:191, booleanoosjson.rs:145, booleanevidencejson.rs:270, candidatejson.rs:305, expressionsearchjson.rs:117 (F); indexstop*json `try_lock` (F/busy) | F | reader open and projection |
| api `calendar_of` `held`, `flights`, `landed` | calendar_of.rs:216, 223, 2059, 2091, 2120, 2170 | R | map ops; Condvar wait loop re-checks `Deriving` (spurious-wake safe) |
| api `livejson::CENSUS` | livejson.rs:128 | R | `CensusCache::refresh` (commit-at-end, live.rs:887-967) |
| api `audit_json::RollupCache` | audit_json.rs:489, 501 | R | build |
| api `autopilot` `status` | autopilot.rs:2140 (S), 2160 (F→None), 2220 (F→Halted json) | mixed | pure closures |
| api `serving_roots` | server.rs:17653 (R), 17585 and 17822 (S) | mixed | BTreeSet insert/remove |
| cli `LEDGER: Mutex<()>` | lib.rs:20247 | F | the whole result-set commit (flock, frontier, trades, results, fsyncs) |
| cli `results::WRITER` | results.rs:780 | F | `Results::open`, `refresh`, `append`, `confirm_durable` (fsync) |
| cli `operation_audit` `State` | operation_audit.rs:404, 480 (R); 415, 429, 493 (F) | mixed | `append` = try_lock + write_all + `sync_all` |
| cli `sweep_evidence` `failure`, `depth_digest`, `ranked_digest` | sweep_evidence.rs:582, 640, 662, 671, 738, 750 | F | `level()` holds `depth_digest` across `append_row` (write + barrier) |
| cli `sweep_evidence::FLUSHED` | sweep_evidence.rs:1137 | R | set ops; epoch check (FORGOTTEN) under the same guard |
| cli `candidate_trades` `state` | candidate_trades.rs:351, 419, 466, 507, 573, 591 (F); 426 (S) | mixed | `write_exact` (tier/trade/candidate files) |
| cli `knobs` store/refusals RwLock | knobs.rs:125, 236, 292, 320, 360, 363, 383 | S | map ops (known hunt-conc-6) |
| pull `masters` cookie jar | masters.rs:604, 635 | R | header map ops |

Every `File::lock`/`lock_shared`/`try_lock` hit in `crates/cli/src/{selection*,population*,execution*,anchored_search_lineage*,global_replay*,institutional_statistics,stored_data_completeness,checksum_receipts,frontier,trades,result_set,admission_store}.rs` is an OS file lock (flock), not a mutex, and has no poison semantics. All of them use the explicit `lock → *_locked → unlock` shape with no guard (D-0693). In a dev build, a panic inside `*_locked` therefore skips the unlock. The lock is then held until the `File` drops. When the owner object is dropped during the unwind, that drop releases it. See latent item L4 for where it can outlive the panic.

## Checked and latent (not counted: no reachable panic in the critical section)

Each item below needs a panic while the lock is held. I found none reachable in these critical sections: they hold map ops, checked arithmetic, `write_all`/`sync_all`, and checked decoding. In release they cannot happen at all.
- **L1. `Mutex<()>` locks that refuse forever on poison.** `Site::reload_lock` (server.rs:5440-5443, "master reload lock poisoned; previous universe retained") and cli `LEDGER` (lib.rs:20247-20251). A `()` guard carries no invariant, and the real cross-process guard (`ResultSetLock` flock, or the swap under `parsed.write()`) is unaffected. Yet one dev-build panic during a masters refresh or a result commit disables that function for the life of the process. This contradicts the policy stated three screens up, at server.rs:5397-5404: *"Refusing every request afterwards would turn one panicking request into a dead server"*. Fix if it ever matters: `unwrap_or_else(PoisonError::into_inner)` on both, as `ADMISSION` already does.
- **L2. `serving_roots` asymmetry (adds to errpaths line 58 / pass-1 server2).** `take_serve_lock` reads through poison (server.rs:17653-17672), but `ServeLock::drop` (17585) and `release_root` (17822) use `if let Ok` and skip removal. On a poisoned set, the first serve's key is never removed. The next `take_serve_lock` of that root in the same process then gets the pass-through `ServeLock { held: None }` and serves **without the file lock**, so a second process can take `serve.lock` beside it. errpaths-58 described the consequence as a dropped write; the actual consequence is a lock bypass. It is still unreachable, because the only work under that guard is `BTreeSet::insert/remove` of an already-built `PathBuf`.
- **L3. `site.budgets` refusal text is inverted.** server.rs:8585-8592 refuses because *"the allowance already spent is unknown"*. But the outer lock guards only the list of `Arc`s. The allowance lives in the inner governor mutex, which this same block reads through poison at 8606-8609, as do all of http.rs's sites. Same mutex, three policies (5185 S, 8585 F, 9761 S). The S sites are pass-1 pull1-2.
- **L4. Explicit-unlock writers whose owner survives the panic.** `results::with_shared_writer` keeps its `Results` in the `WRITER` static (results.rs:777-795). A dev-build panic inside `Results::append_locked` (results.rs:1268) skips `self.file.unlock()` (results.rs:1189-1193) and poisons `WRITER`. `WRITER` is then never reused or dropped, so the **exclusive** flock on `results/runs.bin` stays held for the life of the process. Every `cli` writer on that store and every shared-lock reader, including the api's own `detail::LEDGER` handle (a separate open file description), would block in `lock()`/`lock_shared()` indefinitely. I found no panic source in `append_locked`: `to_bytes` uses fixed-layout slices and the arithmetic is saturating. The fix if wanted: hold the lock in a `store::flock::Flock` guard, whose `Drop` unlocks, or reset `WRITER` through poison.
- **L5. Mixed policies on one lock.** `recovery_active`: set and clear read through poison, while `stop`, `is_stopped` and `clear_stop` refuse. `site.sweep`: sweeprun reads through, while the Boolean and index-stop progress callbacks refuse; the run's result is then still installed by `TaskFinisher::finish` through poison, so the end state is a consistent refusal. Neither has a panic source under the guard: `persist` errors are `Result`s.
- **L6. `record_throttled` emits telemetry under the governor and, at server.rs:9761-9767, under `site.budgets` too, on a Tokio worker.** The emit is a file append. The only blocking stderr write is `Sink::report`, which runs at most once per sink and after the sink guard is dropped, but still under these two locks. Not a finding without a full stderr pipe.

## Drop impls that do I/O (production)

| Drop | I/O | Notes |
|---|---|---|
| `store::flock::Flock` (flock.rs:169-180) | `unlock` syscall; on refusal `note_unreleased` (telemetry) | Lock order store → sink only. No cycle. |
| `api::ServeLock` (server.rs:17579-17589) | drops the `Flock`, then the set removal | L2 |
| `cli::execution_lease::Lease` (via `Flock`) | unlock | kernel releases on crash |
| `cli::sweep_evidence::Attempt` (sweep_evidence.rs:881-897) | `write_terminal(Refused)`: O(rows) re-read and hash, lifecycle append with barrier, journal append with barrier | Only on blocking threads or the cli main thread; I found no `Attempt` held in async api code. If the panic it is unwinding from happened inside `level()` holding `depth_digest`, `seal` refuses "digest lock poisoned", no terminal is written, and the lifecycle stays `Running` with a telemetry error. That is loud, not wrong. |
| `cli::operation_audit::Attempt` (operation_audit.rs:445-462) | `finish` = try_lock + write + `sync_all` | See poison-1 for the `panicking()` arm. The guard is taken in a `let` temporary and released before `telemetry::emit`, so the telemetry call cannot re-enter it. |
| `api::sweeprun::TaskFinisher` (sweeprun.rs:1265-1284) | operation audit finish (fsync), then `site.sweep` | Lock order: audit `State` → `site.sweep`. No site takes them in reverse; the Boolean and index-stop callbacks take only `site.sweep`. |
| `api::sweeprun::Applied` (1459-1462) | none (knob map clear) | poison-1 |
| `api::pullrun::Finisher` (590-605) | none (memory) | pass-1 runs finding about generation still stands |
| `api::calendar_of::Landing` (211-229) | none | flights then landed, each taken and released alone |
| `api::server::Slot`, `detail::Permit`, `autopilot::{Seat, AllSeats}`, `boolean_observation_file::Projecting`, `index_stop_*::Projection` | atomics only | |

## Pass-1 verification (items touching this slice)

- **hunt-conc-6 / errpaths line 58 (knobs, autopilot `publish`, ServeLock `if let Ok`)**: CONFIRMED as latent. The code is unchanged: knobs.rs:125-128, 291-293, 319-327, 359-366; autopilot.rs:2139-2143; server.rs:17585, 17822. It is unreachable in release (abort). L2 corrects the ServeLock consequence.
- **pull1-2 (`shared_governor` `.ok()?` drops the governor on poison)**: CONFIRMED present at server.rs:5183-5190. It is latent: there is no panic source under `site.budgets` (rate.rs `reserve` uses `checked_add`/`saturating_sub`). L3 adds the inverted rationale.
- **telemetry (pass 1): "Poisoned `inner`/`last_error` recovered with `into_inner`; a panic inside `line()` leaves only a burned seq"**: CONFIRMED. sink.rs:1115-1131 increments `seq` and clears `buf` before `line()`, and `bytes` is updated only after `append` returns `Ok`.
- **store1/store2: "Verified-block Mutex ... set to NO_BLOCK before every fill ... recovering from poison is safe"**: CONFIRMED (file.rs:1832-1841, 1920-1922, 2367-2371).
- **apicache: "`lock().map_err("poisoned")` would wedge after a panic; no reachable panic under the lock"**: CONFIRMED for the static page caches. booleanjson.rs:189-213 sets `*cache = None` before reopening, so a failed open cannot leave a stale reader.
- **apicache: "single-flight `Landing` drop wakes followers on a panic"**: CONFIRMED for dev and test builds. In release a leader panic aborts the process, so it is moot (poison-1).
- **runs (pass 1): "TaskFinisher ... the knob guard (`Applied`) clears on unwind"**: CONFIRMED for dev and test builds only. In the release binary there is no unwind (poison-1).
- **runs (pass 1): "A chain panic becomes `note_dead_chain` plus `Retry`"**: REFUTED for the shipped binary. With `panic = "abort"` (Cargo.toml:185) a chain panic aborts the process, so `pullrun.rs:885`'s `Err(dead)` arm is reached only by cancellation. It is true only for `cargo run` without `--release` and for tests.
- **cli1 (pass 1): "On a panic, `Attempt::drop` writes `Failed` while unwinding and the lease guard unlocks"**: REFUTED for the shipped binary, for the same reason. In release a panic leaves the operation-audit record at `Started`/`Progress`, exactly like the SIGKILL case that pass 1 lists next to it (the kernel still releases the lease). It holds for dev builds.
- **runs-3 / recovery-5 (`site.run` held across `recovery_control::stop` fsyncs)**: CONFIRMED still present (server.rs:10984-10995 holds `held`, then calls `recovery_control::stop`, which takes `recovery_active` and calls `persist`).
- **hunt-conc (pass 0): "no production `.lock().unwrap()`"**: CONFIRMED. Every `.lock().unwrap()`/`.expect(` hit is under `#[cfg(test)]` (for example emitted.rs is a test-only module, and recovery_control.rs:342+ is in its test module). Production `Mutex::try_lock` (indexstop*json caches) maps both `WouldBlock` and `Poisoned` to a named busy refusal.

---

<!-- conc-pass2/atomics.md -->
Verdict: the workspace's production atomics are mostly sound. I found 1 new finding (0 high, 0 medium, 1 low), and the one pass-1 finding about atomic ordering (autopilot-1) is still present.

Slice: workspace-wide, non-test code only. That covers every `Atomic*` use and its `Ordering`, flag-then-data publication, counters used as generations or epochs, `compare_exchange` loops, `fetch_add` overflow, and Relaxed loads that gate correctness. Commit 331b05c. I read the source only and ran no cargo.

Method: I grepped every `Atomic*`, `fetch_*`, `compare_exchange`, `swap(` and `Ordering::{Relaxed,Acquire,Release,AcqRel,SeqCst}` in `crates/**/*.rs` and dropped `*_tests.rs`, `tests/` and `#[cfg(test)] mod` bodies. That left 414 raw hit lines, and I sorted each one into production or test. Most cli and store hits are temp-root counters inside test modules, and those are not reported. `api::isolated` is `#[cfg(test)]` (lib.rs:137), and `cli::stored::CALENDAR_POLICY_DIGEST_INITIALIZATIONS_V2` is `#[cfg(test)]`.

## atomics-1 (low): the pull-run telemetry key is claimed and released per leg, not per press, so the first leg to finish clears it while sibling feeds are still running

**Where**
- crates/api/src/server.rs:7811 (inside `broker_run`, which runs once per leg): `let claimed_run = note_run_started(asked, targets.len());`
- crates/api/src/server.rs:6951-6954 (`note_run_started`):
  ```rust
  let claimed = telemetry::global().and_then(|sink| {
      let id = telemetry::now_millis().unsigned_abs();
      sink.claim_run(id).then_some(id)
  });
  ```
- crates/api/src/server.rs:7058-7060 (`note_run_finished`, reached from server.rs:7947 at the end of the same leg):
  ```rust
  if let (Some(sink), Some(run)) = (telemetry::global(), claimed) {
      sink.release_run(run);
  }
  ```
- crates/telemetry/src/sink.rs:993-996 (`claim_run`: `compare_exchange(0, run, AcqRel, Acquire)`), 1015-1018 (`release_run`: `compare_exchange(run, 0, ..)`), and 1080 (`emit` stamps `self.run.load(Ordering::Relaxed)`).
- crates/api/src/pullrun.rs:851-875: `run_pass` does `tokio::spawn(run_chain(..))` once per feed, so the feeds run concurrently. Each chain runs its legs one after another through `request_leg` → `pull_spot` → `broker_answer` → `broker_run` (pullrun.rs:682-687, server.rs:6534).

**Why it is wrong**

The CAS itself is correct. The scope that holds the key is the problem. The design comments say the key belongs to the press:
- sink.rs:972: "The OUTERMOST scope that knows a run has begun takes the key, every concurrent leg beneath it inherits that one id."
- server.rs:7052-7055: "a leg that finished early can no longer clear the key out from under the ones still running -- which is what left the survivors emitting events with no run at all."

But the claim is made in `broker_run`, one level below the press. Whichever leg claims first also releases at the end of its own leg, while the other feeds' chains are still running. The CAS stops a loser from clearing the key, but nothing stops the winner from clearing it early. Once it is cleared, the next leg on any chain claims a new millisecond id, and the still-running sibling inherits that one. The defect the comments call fixed, a story that stops mid-sentence and a second story assembled from two feeds, can still happen whenever a press has two or more feeds.

**Repro (exact interleaving)**

Take a press of two feeds: Dhan with legs D-day and D-minute, and Groww with leg G-day.

1. t0: the chains spawn. D-day enters `broker_run` and claims X (`run = X`). G-day enters `broker_run`, its `claim_run` fails, and `claimed = None`. G-day's "pull.run started" event and its per-instrument events carry X.
2. t1: D-day finishes. `note_run_finished` → `release_run(X)` succeeds, so `run = 0`.
3. t2: G-day's next per-instrument events go through `telemetry::emit` → `emit_for_run(self.run.load())` = 0. The `run` key is omitted.
4. t3: the Dhan chain starts D-minute. `claim_run(Y)` succeeds because the key is free. G-day's remaining events, including its "pull.run finished" event, now carry Y.

Result: `/logs?run=X` shows G-day starting and never finishing. `/logs?run=Y` shows a G-day finish with no start, mixed in with D-minute. Some of G-day's events carry no run at all. This is telemetry correlation only. No stored bar or journal row is affected.

**Minimal fix**

Claim the key in the press scope. In `pullrun::conduct_with`, take `id = sink.reserve_run_id()`, call `claim_run(id)` before the first `run_pass`, and `release_run(id)` after the last pass has joined (or in `Finisher::drop`). In `broker_run`, keep the claim only as a fallback for a lone hand or autopilot run: if the press holds the key, `claim_run` already fails and the leg does not release. Alternatively, thread the press id into the legs and emit with `emit_for_run`, as `sweeprun` already does (sweeprun.rs:1371, 1613).

## Pass-1 verification (findings that touch this slice)

| Pass-1 item | Verdict | Reason from the code |
|---|---|---|
| autopilot-1 (pause in the pre-fetch window is ignored) | CONFIRMED | autopilot.rs:2643 checks `is_paused()` (Relaxed). server.rs:7819 later captures `epoch()` (Relaxed). `pause()` (autopilot.rs:2034-2035) stores `paused` and then does `fetch_add` on the epoch. A pause between those two reads bumps the epoch before it is captured, so `stopped(epoch)` (server.rs:7840) stays false for the whole run. On ordering: as written, both are Relaxed on two different atomics. So even with the suggested reorder (capture, then check paused), a reader could see the new epoch and still read `paused == false`. Nothing orders the two locations. The pass-1 fix note is correct that the bump must be Release and the capture Acquire (or both SeqCst). |
| telemetry-1 / cli1-5 / xcut-1 (`reserve_run_id` duplicates across processes) | CONFIRMED (the atomic part) | `reserved_run` is a per-process `AtomicU64` seeded from `resume_point` (sink.rs:746, 788). `fetch_update` (sink.rs:1032-1039) is exact only inside one process. Two processes that share a log dir start from the same seed and hand out the same ids. |
| hunt-conc-8 (`SWEEPS_SHARING_THIS_MACHINE` is process-global, and the last guard drop resets it to 1) | CONFIRMED (info, as reported) | cli/lib.rs:16529-16540: `these` is `store(count)` and `Drop` is `store(1)`, both Relaxed, with no counting. Overlapping guards would clobber each other. Only batch.rs:356 and lib.rs:15218 create guards, and the server's admission serialises them. Relaxed is otherwise sufficient: rayon's job injection orders the store before worker reads. |
| pass-1 telemetry "claim_run/release_run sound" (concurrency.md:753) | PARTLY REFUTED | The CAS pair is correct as a primitive. The production caller holds it at the wrong scope; see atomics-1. |
| pass-1 server/runs "pullrun checkpoints Relaxed OK" (concurrency.md:286) | CONFIRMED | `clean` flags (pullrun.rs:760, 785, 855) and `attempted` (776, 889, 895) are read only after `chain.await`. A JoinHandle await synchronizes-with the task's completion. The reset at 855 runs before the spawn. |
| sweep_evidence atomics (concurrency.md:1412) | CONFIRMED | `acknowledged_*` use Release/Acquire and are read in `seal`. `ranked_published.swap(AcqRel)` is a one-shot. The `FORGOTTEN` epoch is bumped while holding the `FLUSHED` lock (sweep_evidence.rs:1141-1144), and re-read under that lock before insert (1125-1128), so a refusal that lands during a flush keeps the chain unremembered. The epoch is a u64 counter that cannot wrap in practice. |
| index-stop scheduler (concurrency.md:1974, hunt-conc) | CONFIRMED | index_stop_search_progress.rs:180-188: the Relaxed `fetch_add` only hands out indices, and `assemble` refuses gaps and duplicates. `cancelled` is Release/Acquire and only advisory, because dropping the receiver also fails `send`. Overshoot past `len` is at most one per lane. |
| capture slots (hunt-conc) | CONFIRMED | capture.rs:315, 423: `fetch_update` with a `< PER_SLOT` bound is the whole decision. It publishes no data, and file names are created with `create_new`. |
| lake `stranded` (concurrency.md:893) | CONFIRMED | page.rs:550 stores and reader.rs:645 loads on the same thread, after `read_records` drains the boxed page reader. |
| GridProgress (concurrency.md:1282) | CONFIRMED | cli/lib.rs:18381-18388: the counter only decides whether to print. |
| `Assets::missing` (concurrency.md:171), `rate::ABSORBED_MICROS` (998/1133), telemetry floors (752) | CONFIRMED | All are display or gating counters with no data published through them. `floors` is one packed u16, so the global and fast floors cannot tear. |

## Checked and clean (not findings)

- **autopilot seats** (autopilot.rs:2103, 2131, 2246, 2260): `fetch_or` for one seat, CAS 0→ALL for every seat, `fetch_and(!bit, Release)` and `store(0, Release)` to drop. A loser changes nothing. `AllSeats::drop`'s blanket `store(0)` is safe because no single seat can be taken while all are held. `ALL_SEATS` covers every `seat_bit`, with a const assert that `FEED_COUNT < 8`.
- **autopilot epoch otherwise**: one u64 with coherent modification order. A pause after capture is always eventually observed by `stopped`. `resume` correctly does not bump it.
- **server `Slots`** (server.rs:16801-16824) and **detail permits** (detail.rs:59-82): `fetch_update` with a cap check, and a decrement on Drop. `Permit::owed` deliberately exceeds the cap and is counted. There is no lost wake-up, because `Notify::notify_one` stores a permit.
- **calendar_of `waiting`** (calendar_of.rs:2086-2104): a gauge that no production code reads.
- **telemetry**: `rotation_broken` is read and written only under the `inner` mutex. `reported` is a one-shot CAS. `reserve_run_id` uses checked_add with a JS-safe cap and refuses rather than wrapping. `written`, `dropped` and `rotations` are counters.
- **re-entrancy guards**: `boolean_observation_file::ReadLease` (CAS AcqRel; field order drops the flock before clearing the flag), `index_stop_source_context::Projection` (CAS Acquire / store Release), and `index_stop_search_reader::Projection` plus the `require_current` Acquire check. All are try-guards that refuse rather than wait, and no data is published through them.
- **engine, runner, indicators, vocab, core, store, costs, greeks**: no production atomics. The only hits are in test modules.

---

<!-- conc-pass2/determinism.md -->
Verdict: 3 new findings (0 high, 0 medium, 3 low). Each is a path where read_dir order or HashMap order reaches output bytes or refusal text. None of them reaches a digest, the run identity, or journal byte order, and none is listed in hunt-conc.md or pass 1.

### conc-pass2 / determinism (commit 331b05c)

Slice: workspace-wide, non-test code only. I looked for every HashMap/HashSet iteration, `read_dir` ordering, thread schedule and wall-clock value that can reach output bytes, digests, run identity, refusal text, journal byte order, or which work runs first before a stop. I read source only and ran no cargo.

Method:
- A script listed every non-test HashMap/HashSet binding and struct field, then every `.iter()`/`.values()`/`.keys()`/`.into_iter()`/`.drain()` and every `for (a, b) in map` over them, on one line or split across lines. Each hit was checked by hand.
- All 93 `read_dir` sites were checked; about 18 are in production code.
- I checked every `SystemTime::now` and `available_parallelism` site, and every rayon / `thread::scope` / `tokio::spawn` site outside tests.

## determinism-1 (low): the `/pull` folder picker's "capped" note can never print, and the 60 folders it offers are chosen by `read_dir` order

- **Where:** crates/api/src/render.rs:3142-3175 (`collect_csv_dirs`), 3097-3111 (`discover_folders`), 3036-3037 and 3053-3063 (`folder_input`).
- **Code:**
  ```rust
  fn collect_csv_dirs(dir: &Path, depth: usize, out: &mut Vec<String>) {
      if depth == 0 || out.len() >= MAX_FOLDER_SUGGESTIONS || !dir.is_dir() { return; }
      let Ok(entries) = std::fs::read_dir(dir) else { return; };
      ... for e in entries.flatten() { ... children.push(p) ... }
      if has_csv { out.push(dir.to_string_lossy().into_owned()); }
      for c in children { collect_csv_dirs(&c, depth - 1, out); }
  }
  ...
  found.sort(); found.dedup();
  ...
  let capped = suggestions.len() > MAX_FOLDER_SUGGESTIONS;
  ```
- **Why it is wrong:**
  - The guard runs before the push, so `out` stops at exactly 60. `dedup` cannot add entries, so `suggestions.len() > 60` is never true, and the ", capped at 60" clause in `folder_input` can never print.
  - Once the walk has more than 60 CSV-bearing folders, the 60 it keeps are the first 60 in depth-first `read_dir` order. That is filesystem order, which differs between machines, filesystems, and even the same directory before and after a copy. Sorting happens only after the walk has already truncated.
  - The page then says "60 folder(s) holding CSVs found" with no truncation note. The code's own doc at 3010-3012 and 3069-3071 calls that "a lie about completeness": *"A cap that silently truncates is a lie about completeness"*.
  - Same inputs give different page bytes (CLAUDE.md §3 rule 5). The truncation is hidden, which is the §4 "fallback that hides a failure".
- **Repro:** Put 61 or more subfolders, each holding one `.csv`, under `~/Downloads`, then start `api serve` and GET `/pull`. The datalist has 60 options and the footnote says "60 folder(s) … found" with no "capped" text. Copy the same tree to another filesystem (for example, tmpfs vs ext4 hash order) and restart: a different set of 60 folders is offered.
- **Fix:** Let the walk collect one more than the cap, so the check can fire: guard on `out.len() > MAX_FOLDER_SUGGESTIONS`, or keep a separate `truncated: bool` set when a push is refused. Then either sort each directory's `children` before recursing, which makes the truncated subset deterministic, or state on the page that the subset is filesystem-ordered.

## determinism-2 (low): folder-census `rejected` findings go out in `read_dir` order, and the strict walk's refusal names whichever bad member the filesystem lists first

- **Where:**
  - Production: crates/pull/src/archive.rs:703-794 (`descend` pushes `rejected` in `fs::read_dir` order), 613-665 (`walk` sorts `out` only: `sort_members(out)`), and 560-587 (`read_dir_reporting` returns `rejected` unsorted).
  - Census: crates/pull/src/folder.rs:380-394 (`census_of` sorts `instruments` and passes `rejected` through unchanged).
  - Output: crates/api/src/folder.rs:332-345 (writes `"rejected":[…]` in that order).
  - Strict path: archive.rs:787-789 (`Malformed::Refuse => return Err(ArchiveError::MemberMalformed { path, why })`), plus `read_bounded` (`MemberTooLarge { path }`) and `MemberNotText { path }` at 775-777. All return on the first offending entry met in directory order.
- **Code:**
  ```rust
  // archive.rs walk
  descend(dir, columns, out, passed, on_malformed, rejected, 0)?;
  sort_members(out);            // `rejected` is never ordered
  // folder.rs census_of
  instruments.sort_unstable(); ... Ok(Census { ..., instruments, rejected })
  ```
- **Why it is wrong:**
  - The module states twice (archive.rs:819-822 and 867-873; folder.rs:337-341) that `read_dir` order "is not stable between machines or between runs" and that §3 rule 5 requires sorting. The rule is applied to `out` and `instruments` but not to the sibling list that is also shipped on the wire.
  - A folder census with two or more undecodable members returns the same findings in a different order on another machine, or after the folder is copied.
  - On the ingest path (`read_dir`, `Malformed::Refuse`), a folder with two malformed or oversized members is refused with text naming member A on one machine and member B on another. That is "same bad input, different refusal text", the class hunt-conc-3 recorded for HashMap order, here reached through directory order.
- **Repro:** Create `vendor-data/gdfl/X/` with `a.csv` and `z.csv`, both with nine fields (one fewer than the declared layout), plus one good member. GET the folder census route for that path. The order of `"rejected"` is whatever `getdents` yields, so on ext4 with `dir_index` it follows the filename hash, not `a` before `z`. Copy the folder with `cp -r` to tmpfs and ask again: the order can flip. POST the same folder as an ingest and the refusal names `a.csv` or `z.csv` to match.
- **Fix:**
  - In `walk`, after `sort_members(out)`, add `rejected.sort_by(|a, b| a.path.cmp(&b.path))`.
  - For the strict refusal, collect directory entries into a `Vec<PathBuf>`, sort them, and then iterate, as `cli/build_provenance.rs:988-1001` already does. The first refusal is then the lexically first bad member everywhere. This also makes the `MAX_MEMBERS` cutoff point deterministic.

## determinism-3 (low): `absorb_rows` folds a `HashMap` into the index, so the duplicate-block refusal names a random population, and a retry on the same handle names one that is not duplicated at all

- **Where:** crates/cli/src/population.rs:3538-3552 (`absorb_rows`), with `index_rows_range` (4491-4560) returning `HashMap<[u8; 32], BlockFacts>`. Reached from `append_complete_locked` / `_v3_locked` / `_v4_locked` (3116, 3186, …), which run on a reusable `&mut self` ledger handle.
- **Code:**
  ```rust
  let new_blocks = index_rows_range(&mut self.row_file, &self.row_path, self.row_scanned, len)?;
  reserve_map(&mut self.raw_blocks, new_blocks.len(), "population-block index")?;
  for (identity, facts) in new_blocks {
      if self.raw_blocks.insert(identity, facts).is_some() {
          return Err(format!("{} gained a duplicate/non-contiguous block for population {}", ..., hex(&identity)));
      }
  }
  self.row_scanned = len;
  ```
- **Why it is wrong:**
  1. This is the hunt-conc-3 class at a site that report does not list (it names 4644/4670/4706 only). When the newly appended range repeats more than one already-indexed population, the identity printed is whichever the per-process `RandomState` yields first. Same bytes on disk give different refusal text from one process to the next.
  2. The loop inserts into `self.raw_blocks` before it refuses and never rolls back. `row_scanned` is not advanced, so the next call on the same handle re-indexes the same range. The entries the failed call already inserted now collide with themselves, and the refusal names a population whose block is *not* duplicated in the file. Which population it names depends on HashMap order again. The handle's in-memory index also now holds a seed-dependent subset of blocks from beyond `row_scanned`, which `contains_key(&receipt.population_id)` (3153) and `raw_blocks.get` (3165) read.
- **Repro:**
  1. Ledger L holds blocks P and Q, already absorbed by handle H.
  2. A foreign writer appends a second, non-contiguous block for P, then one for Q, then a fresh block R.
  3. `H.append_complete(...)` → `absorb_rows` → `new_blocks = {P, Q, R}`. Iteration order varies by process. The refusal names P in some processes and Q in others. In a process where the order is R, P, …, R is inserted before the refusal.
  4. Call `H.append_complete(...)` again. The first entry iterated may now be R, so the error reads "gained a duplicate/non-contiguous block for population R", and R is not duplicated in the file.
- **Fix:** Validate first and commit after: check every `identity` with `!self.raw_blocks.contains_key(&identity)` in a sorted pass (collect `new_blocks` into a `Vec` and sort by identity, or have `index_rows_range` return a `Vec` in file order, which it already walks). Insert only once all checks pass. That fixes both the random name and the poisoned retry.

## Pass-1 verification (findings that touch this slice)

- **recovery-3 (HashMap order in `reconcile_pending`): CONFIRMED.**
  - api/src/recovery.rs:1046-1056 still builds `pending` from `attempts.latest.values()`, where `latest` is a `HashMap<[u8;32], Record>` (recovery_journal.rs:630).
  - It then calls `assess`/`append_attempt` and checks `stopping(site)` per item in that order (1057-1060). The deterministic `attempts.order` (recovery_journal.rs:631) is unused here.
- **hunt-conc-3 (refusal names the first bad identity in HashMap order): CONFIRMED.**
  - admission_store.rs:1385-1406 `reconcile_all` still iterates `&HashMap` with early `return Err(... hex(population_id))`.
  - population.rs:4644 (and 4670/4706) still iterate `receipts: &HashMap` with `?` refusals.
- **hunt-conc-1 (durable writes inside rayon workers): CONFIRMED.**
  - batch.rs:357-360 is still `wanted.par_iter().map(|held| one(root, held, min_hits, commit))`.
  - `one` still calls `sweep_evidence::begin` (631) and `record_swept_run` (724) on the worker.
- **cli1-4 (thread-local invocation lost under `sweep_rungs`' `par_iter`): CONFIRMED.**
  - operation_audit.rs:438-441 `enter` sets the thread-local `CURRENT` on the calling thread only.
  - `completed_boundary` (493-506) does nothing when `CURRENT` is `None`.
  - lib.rs:15220 runs `one_rung` under `par_iter` with no re-entry. Called from a non-pool thread, rayon runs every closure on pool workers.
- **The pass-1 "clean" claims I re-checked hold:**
  - HashMap walks that reach output are sorted: server.rs:1196-1222 (total key on an NSE-only merge), 2875-2878, 6608-6613, 7363-7393, 7539-7540, 7783 (`targets.sort_unstable()` on the full `InstrumentKey`), 32126-32141, 33372; merge.rs:278-291, 562-564; autopilot.rs:2300-2320; indexmap.rs:175-176; census.rs:820-896; trades.rs:2411-2412.
  - Order-independent folds: merge.rs:237, 328; server.rs:5749; universe.rs:456/521; manifest.rs:2516; nseindex.rs:217-224 (ambiguity count), 296; recovery.rs:269 (constant refusal text), 464, 695 (unique attempt sequence), 787 (max).
  - Recovery seal ordering: `scope_order` sorts by the full body (recovery.rs:199-217), which is unique per key.

## Checked and not a finding

- **Other production `read_dir` sites:**
  - store/catalog.rs:291: `held` is sorted at 317, and the census holds only counts.
  - cli/live.rs:609: sorted at 678. live.rs:888 goes into a `BTreeMap`.
  - cli/search_checkpoint.rs:429: refusal texts are constants; `latest`/`next` are max folds.
  - cli/build_provenance.rs:408 and 988: both sort before use.
  - api/server.rs:8714: only tests emptiness.
  - api/assets.rs:387 and 626: counts, and the newest-mtime file; only a tie on mtime or more than 100,000 web/src entries makes the order matter, which is a dev-only banner.
  - telemetry/lib.rs:416: scratch cleanup.
- **Wall clock:**
  - cli/research.rs:59 resolves "today" once and prints it as the stated cutoff of an inventory report; nothing is hashed.
  - sweep_evidence.rs:1104 and lib.rs:17438 are ledger timestamps, not identity terms (as hunt-conc already noted).
  - The screen budget calibration at lib.rs:10106-10140 is refused on recording runs (D-0685).
- **Thread scheduling:**
  - The capture writes in the lib.rs:12064 screen `par_iter` go to per-(tier, rank, direction) files. The catalog is sealed in slot order (candidate_trades.rs:610-640), and `capture.check()?` after the collect (12205-12207) refuses the screen rather than publishing a schedule-dependent subset.
  - The only schedule dependence left is which concurrent failure's text is latched first by `refuse`'s `get_or_insert` (candidate_trades.rs:425-429). That needs two different simultaneous failures, so I did not raise it.
- **`serde_json`:** no `preserve_order`; its Map is a BTreeMap, as hunt-conc noted.

---

<!-- conc-pass2/clock.md -->
### conc-pass2 / clock: wall clock vs monotonic, clock steps, IST day and session boundaries

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

---

<!-- conc-pass2/resources.md -->
### conc-pass2 / resources: 4 findings at 331b05c (0 high, 2 medium, 2 low)

**Verdict.** Request-fed queues, channels and caches are bounded, and thread counts do not grow with input. Four defects remain:
- The run-commit protocol turns a *reported* fsync failure into a committed success, within the same call, by retrying the fsync.
- `/bars/window.json` holds three descriptors per month for up to 240 months. On the operator's platform one request can exhaust the descriptor table.
- A client disconnect writes the audit terminal, with an fsync, on a Tokio worker.
- Two uncapped read routes (`/bars/window.json`, `/backtest.json`) run inline on Tokio workers and are missing from the D-1443 list of what still blocks there.

Slice: workspace-wide non-test code. I looked for descriptor leaks and caps, unbounded channels, queues and Vecs fed by requests, thread or task counts that grow with input, blocking calls on async workers, and ENOSPC/EIO paths other than the missing-`set_len` rollback class. Method: source reading only. I ran no cargo and edited nothing in the repository. Previously reported material was skipped: pass 1 (`concurrency.md`), `hunt-conc.md`, `errpaths.md`, and `o1surface2.md`, which the slice overlaps on worker blocking. Anything `docs/06-limits.md` already states was also skipped.

---

## resources-1 (medium): retrying `sync_all` after it reported an error "confirms" durability, so one EIO/ENOSPC on a detail block or the ledger row becomes `Reused`/committed in the same invocation

**Where**
- `crates/cli/src/lib.rs:16986-16998` (`ensure_frontier_rows`, `Err(first)` arm). `ensure_trade_rows` has the same arm at :17029-17037.
- `crates/cli/src/lib.rs:17348-17359` (`ensure_run_record`, `Err(first)` arm).
- The barrier that is retried: `crates/cli/src/results.rs:1245-1261`, `frontier.rs:1320-1336` and `trades.rs:697-712` (`confirm_durable`).

```rust
// lib.rs:17348
// A concurrent identical append or a completed write followed by a
// failed sync may already be present. Refresh the validated handle
// and compare all deterministic fields before confirming durability.
Err(first) => {
    store.refresh().map_err(...)?;
    if store.holds(&record.identity) {
        verify(store)            // -> store.confirm_durable()?; Ok(Committed::Reused(index))
    } else { Err(first) }
}
```
```rust
// lib.rs:16986 (frontier; trades identical)
match store.append_all(rows) {
    Ok(_) => Ok(Prepared::Written(rows.len())),
    Err(first) => {
        let mut reopened = frontier::Frontier::open(root)...?;
        if reopened.holds(identity) { verify(&mut reopened) }   // -> confirm_durable()?; Ok(Prepared::Reused(..))
        else { Err(first) }
    }
}
```
`confirm_durable` is just `lock` + `sync_all` + `unlock`. The workspace pins this intent in a test (`results.rs:2633-2638`): "one in `confirm_durable` for recovery of a complete prior write whose original barrier did not return success".

**Why it is wrong**
- `append_locked` (results.rs:1365), `append_all` (frontier.rs:1112) and the trades append (trades.rs:676) all return `Err` when `sync_all` fails *after* a complete `write_all`. They leave the bytes in place on purpose ("Nothing was rolled back", results.rs:1366-1371).
- On Linux, a writeback error is reported once per open file description (errseq) and the failed pages are left clean in the page cache. results.rs:1360-1361 itself says "on Linux a failed `fsync` also clears the dirty-page error state".
- The `Err(first)` arms exist for a peer that won the identity race, but they also catch this call's own sync failure:
  - The rows or the record are visible from the page cache, so `holds()` is true and the byte compare matches.
  - The second `sync_all` returns `Ok`. For results it is the same descriptor, whose error was already reported. For frontier and trades it is a fresh descriptor opened after the error had been seen.
- So the I/O error that `append_*` just returned is discarded. The call reports `Prepared::Reused` or `Committed::Reused`, which the CLI prints as `reused`, and the sweep goes on to append the public ledger marker.
- D-0404's ordering (children durable, then the marker) is the whole commit guarantee, and this path makes it vacuous. After a crash, or simply after the clean pages are evicted, the run has:
  - a durable ledger marker over frontier or trade rows that were never written, which is a damaged committed run that §3 rule 8 forbids repairing; or
  - a ledger row that the operator was told was committed and that is gone.
- This is §4's "fallback that hides a failure". The class was reported in pass 1 only for the store (`store1-2`) and Population V1 (`pop1-4`), and only for a *later* retry. Here the failure is swallowed inside one call on the main sweep path, with no rerun needed.

**Repro**
1. On Linux, run `cli sweep-stored ...` (or any `record_all` verb) on a filesystem where writeback of `results/frontier-*.bin` fails once. Use dm-flakey, or NFS/thin-provisioned ENOSPC at fsync.
2. `Frontier::append_all` writes the block, and `sync_all` returns `EIO`, so `append_all` returns "the frontier rows could not be flushed".
3. `ensure_frontier_rows` takes `Err(first)`. `Frontier::open` reindexes and `holds(identity)` is true, because the page cache still holds the rows. `of_run` returns `found == rows`, and `confirm_durable` calls `sync_all`, which returns `Ok` because the error was already consumed. The result is `Ok(Prepared::Reused(n))`.
4. Trades and the ledger row commit normally. The command reports success.
5. Drop caches (`echo 1 > /proc/sys/vm/drop_caches`) or power-cycle. The frontier block reads back as on-disk bytes, so `of_run` reports the run's frontier as damaged under a committed ledger row.

The same steps work with the ledger row itself (`ensure_run_record`), giving `Committed::Reused` for a row whose barrier failed. UNVERIFIED: on macOS, `sync_all` is `F_FULLFSYNC` and its behaviour after a failed flush is undocumented. I did not measure it on either OS.

**Minimal fix**
- Track, per handle, whether this process saw a failed barrier on it. When it did, the `Err(first)` arms must return `first`, with "durability unknown; rerun after the device is healthy", and never call `verify`.
- More generally, `confirm_durable` may only vouch for bytes that this boot has never seen a failed `sync_all` for. After any `sync_all` error on a ledger handle, poison the handle (the store1-2 fix) and refuse in-process promotion.
- Delete the "completed write followed by a failed sync" sentence and fix the test text at results.rs:2633-2638, which pins the wrong recovery as intended.

---

## resources-2 (medium): `/bars/window.json` opens and holds 3 descriptors per held month for up to 240 months, with no admission. One request can exhaust the operator's descriptor table, and the EMFILE then lands on unrelated writers

**Where**
- `crates/api/src/bars.rs:950-975` (`window`: "OPENED ONCE, HELD FOR THE REQUEST") and `bars.rs:1119-1160` (`open_window_months` pushes every opened `BarFile` into `files: Vec<BarFile>`).
- `crates/store/src/file.rs:1368-1415` (`open_existing`): `bars` (one fd), `lock` (`open_read(&lock_path)`, a held shared `Flock`), and the `.crc` sidecar held in `checksums: Option<File>` (file.rs:1041). That is 3 descriptors per month.
- `bars.rs:535` `MAX_WINDOW_MONTHS = 240`.
- The handler `server.rs:3332-3390` calls `bars::window` inline. It goes through none of the `detail::run*` pools.
- No crate raises `RLIMIT_NOFILE`: `rg -i 'rlimit|nofile'` over `crates/*/src` finds nothing.

**Why it is wrong**
- Months with no file cost nothing, but every held month costs 3 descriptors until the response is built. Both the default `ts` seek path (which needs one or two files) and the scan path open all of them up front.
- A 240-month window is 720 descriptors in one request.
- `docs/06-limits.md:5991` says macOS "is the operator's platform". A process started from Terminal or launchd there gets the default soft `maxfiles` of 256. That is UNVERIFIED on the operator's own machine, and an IDE may raise it. Linux's usual soft default is 1024.
- 86 held months (258 fds) exceed 256 by themselves. The 79 months the docs record for the spot store (06-limits ~2722) give 237, plus the process's baseline descriptors.
- `MAX_CONNECTIONS = 256` (server.rs:16772) cannot protect the table either. Its doc says it exists because "the descriptor table was the only ceiling", yet on a 256 soft limit the cap equals the table.
- The handler runs inline, so its concurrency is bounded only by the Tokio worker count. Two to four concurrent windows reach 1024 on Linux.
- While the request holds the table, every other `open` in the process fails with EMFILE:
  - A running sweep's ledger or bar open refuses, and the run fails.
  - `journal::begin` fails, so audited routes answer 503.
  - `accept` fails, and axum sleeps.
  - A concurrent pull reaches `CensusLock::take`, whose EMFILE arm returns `Ok(Self { _held: None })` (pull/src/ingest.rs:3118-3130). That is the pass-1 `pull2-1` unserialised census read-modify-write, and it loses census rows silently.

  pass 1 called that EMFILE trigger hypothetical. This route is a concrete, request-driven source of it.

**Repro**
1. macOS, default `ulimit -n` 256. A store holds NIFTY `1min` for 90 months.
2. A pull of another symbol is running.
3. Open the bars grid, or `curl 'http://127.0.0.1:<port>/bars/window.json?feed=zerodha&exchange=NSE&segment=INDEX&symbol=NIFTY%2050&timeframe=1min&from=2019-01&to=2026-06&sort=close'`.
4. `open_window_months` holds about 250 or more descriptors.
5. Either the window's own later months fail (answered 206 with faults), or the pull's next `CensusLock::take` hits EMFILE and runs unlocked.

On Linux (1024), four concurrent requests of this kind on a 4-worker runtime do the same.

**Minimal fix**
- Do not hold the window's files.
  - For the `ts` seek path: read each month's `n_valid` from its header, close it, then open only the one or two months the page touches.
  - For the scan path: open, read and drop one month at a time. The change fold already runs per file.
- Optionally, admit the route through `detail::run_store_read` (8 slots), which also covers resources-4.
- At startup, raise the soft `RLIMIT_NOFILE` to the hard limit, or refuse to serve below a stated floor and say so. Then bring `MAX_CONNECTIONS` under the soft limit minus a reserve.

---

## resources-3 (low): a client disconnect drops `request_audited`'s armed `Attempt` on the Tokio worker, whose `Drop` appends and `sync_all`s the journal there

**Where**
- `crates/api/src/operation_audit.rs:134-162`. `attempt` is a local of the async fn, held across `let mut response = handler.await;` (:162). It is moved into `run_owed` only after the handler returns.
- `crates/cli/src/operation_audit.rs:445-461`: `impl Drop for Attempt { ... self.finish(Phase::Cancelled, 0) ... }` → `State::update` → `append` (:324-335) → `write_all` + `sync_all` (:345-349), synchronously.

**Why it is wrong**
- hyper 1's HTTP/1 dispatcher watches the read side while a request is in flight. On EOF it reports "unexpected EOF on busy connection" and errors the connection, dropping the in-flight service future. `HeadDeadline::poll_read` passes the 0-byte read through.
- The dropped future drops `attempt`, and its `Drop` writes the `Cancelled` terminal, with a device flush, on whichever Tokio worker was polling the connection.
- D-1445 (06-limits:11733-11743, operation_audit.rs:166-170) records exactly this ("the dropped attempt wrote `Cancelled` ... synchronously on a Tokio worker") as removed. It was removed only for the full-pool path; the disconnect path still does it.
- The 21 `AUDITED` routes include the console's polls (`/live.json`, `/backtest/run.json`, `/backtest.json`). A tab closed or navigated mid-poll, or a `fetch` aborted by the page, costs one fsync on a worker each time. On a slow or failing disk that stalls every task sharing the worker.

**Repro**
1. While `/live.json` is being answered (the handler is inside `detail::run`), the client closes the socket: close the tab, or `curl --max-time 0.01`.
2. hyper drops the future, and `Attempt::drop` runs on the worker: flock `try_lock`, `fstat`, 256-byte `write_all` and `sync_all` on `audit/invocations-v1/<id>.bin`.
3. With `strace -f -e fsync` on the api process, the `fsync` appears on a `tokio-runtime-worker` thread, not a blocking-pool thread.

**Minimal fix**
- Do not let the armed `Attempt` live in the request future. Keep it in a guard whose `Drop` moves it into `tokio::task::spawn_blocking` (or `run_owed`) so the `Cancelled` write runs off the workers.
- Or begin with the attempt owned by a detached blocking task that receives the response status over a oneshot, and treat a dropped sender as `Cancelled`.

---

## resources-4 (low): `/bars/window.json` and `/backtest.json` do uncapped-in-practice synchronous reads and rendering on Tokio workers, and neither is in the D-1443 list of routes that still block there

**Where**
- `crates/api/src/server.rs:3332-3390` (`bars_window_json`) → `bars::window` (bars.rs:918-1060). The scan path reads every record of every held month and builds a `Vec<WindowBar>`; its own comment says "A 240-month window is roughly 1.9 million bars".
- `crates/api/src/backtest.rs:1229-1231` (`backtest_json` → `respond` → `read(&root, limit_asked(query))`). `limit` is clamped to `MAX_RUNS = 20_000` (backtest.rs:390). That is up to 20,000 `seek` + `read_exact` pairs in `read_from` (backtest.rs:1143-1175) plus a JSON body that 06-limits:9395-9399 computes at about 14.6 MB of equity notes alone, all inside an `async fn` with no `detail::run`.

**Why it is wrong**
- `#[tokio::main]` (api/src/main.rs:31) gives one worker per core, and these handlers occupy a worker for the whole read and render.
- D-1443/D-1508 (06-limits:11800-11815) moved `/calendar.json`, `/gaps.json`, `/folder.json` and `/indexmap.json` off the workers, and listed `/verify.json` (06-limits:14011-14016) as the one route still doing a scrub inline. These two are heavier and unnamed. Their costs are documented (W1-api5-4, AF-19), but where they run is not.
- While N such requests run on an N-core host, every task sharing the runtime waits: `/health`, `/pull/run.json` polls, the pull conductor and its ticker, the autopilot `fly` task, and `serve`'s accept loop.

**Repro**
1. On a 4-core host, issue 4 concurrent `GET /bars/window.json?...&sort=close&from=2006-01&to=2025-12` over a long 1-min series, or `GET /backtest.json?limit=20000` against a large ledger.
2. A concurrent `GET /health` is not answered until one of them returns.

**Minimal fix**
- Run both bodies through `detail::run_store_read` (or `detail::run`), answering 429 at the bound as the other routes do.
- Add both routes to the D-1443 paragraph.

---

## Pass-1 verification (findings that touch this slice)

| id | verdict | reason (code at 331b05c) |
|---|---|---|
| engine-1 | CONFIRMED | engine/src/lib.rs:2373-2383 still maps a failed `spawn_scoped` to `Breach::Workers`, and :2210-2212 `cannot_grow`/:2366-2367 `try_reserve` map to `Breach::Memory`. `continue_walk` sets `progress.halted = halt` and calls `sink.checkpoint` (:1665-1667) for every breach kind. resume.rs:268-280 `validate_halt` accepts Workers/Memory, and `resume_checkpointed` (:509-548) returns the saved halt. |
| runs-3 | CONFIRMED | server.rs:10984-10995: `held` (the `site.run` guard) is alive when `recovery_control::stop` → `persist` runs `create_child` ×2, `Journal::open`, `append` and two `sync_all`s (recovery_control.rs:148-168), inline in an `async fn`. |
| recovery-5 | CONFIRMED (same root as runs-3, plus `claim`) | recovery.rs:560-576: `claim` takes `site.run` and calls `clear_stop` → `persist` under it. `start` calls `claim` inline on the worker (recovery.rs:630). |
| runs-4 | CONFIRMED | sweeprun.rs:1727/1963/2983 take a `detail::run` permit, then `admit` blocks on `ADMISSION.lock()` (:1680-1682) before the in-flight check (:1683-1691). `MAX_CONCURRENT = 4` (detail.rs:14). |
| pull2-1 | CONFIRMED | pull/src/ingest.rs:3118-3130: any open error other than PermissionDenied/IsADirectory, with a regular file at the name, returns `Ok(Self { _held: None })`, EMFILE included. resources-2 is a concrete in-process trigger. |
| store1-2 | CONFIRMED | store/src/file.rs:2148-2170: two `sync_all` failures return without poisoning the handle. |
| pop1-4 | CONFIRMED | cli/src/population.rs:3405-3407 leaves the bytes after a failed `sync_all`, and :3153-3157 re-syncs and reuses them. resources-1 is the same class on the main run-commit path, inside one call. |
| recovery-2 | CONFIRMED | recovery.rs:777 `Journal::open(&active_path(site))` creates `active.bin` before :794 `active.append(pointer)`. `active_history` (:262-268) refuses an empty file. |
| recovery-6 | CONFIRMED | recovery.rs:821-827: `journal.append(control).map_err(failure)?;` returns before `answer`. |
| cli1-1 | CONFIRMED | cli/src/operation_audit.rs:556-564: the index record is synced before `create_new`, then `write_synced`. `read` (:617-619) refuses `bytes == 0` as "invocation journal lost its indexed start". |
| xcut-2 | CONFIRMED | pull/src/ingest.rs:3364-3365: `fs::rename(tmp, path)?; fs::File::open(dir)?.sync_all()`. An error after the rename is reported as "not published". |

---

## Checked and clean (not counted)

- **Admission pools.** `detail::run`/`run_calendar`/`run_store_read` (4/8/8, CAS on an `AtomicUsize`; a permit is released by `Drop` inside the blocking closure, so a disconnect cannot leak one). `run_owed` exceeding the cap is documented (D-1445).
- **Connections.** `Slots`/`LimitedListener`/`HeadDeadline`: the slot is bound before the accept await. The body and response have no deadline, documented under D-1200. The one undocumented gap, the cap versus a 256 soft `RLIMIT_NOFILE`, is folded into resources-2.
- **Request-fed collections.**
  - `pullrun::legs_from` refuses unknown vendors, so `by_feed`'s chain count is bounded by the feeds (D-0906).
  - Recovery caps units at `MAX_UNITS = 100_000`.
  - `/ingest/queue` has no queue (`NO_QUEUE`).
  - Autopilot `stalls` (D-0949) and `failures` (`MAX_FAILURES`) are bounded.
  - `expressionsearchjson` `SESSIONS` is capped at 8.
  - Every other `static CACHE` is one slot.
  - `calendar_of::Cache` is keyed by census series and stamp. Flights are removed by `Landing`'s drop.
  - `sweep_evidence::FLUSHED` holds one path per store root.
  - The recovery journal tail is capped at 256.
- **Channels.** The only production channel is `index_stop_search_progress::parallel`'s `sync_channel(prepared.len())`. It is bounded, and dropping the receiver on `break` unblocks senders.
- **Threads.**
  - No production `std::thread::spawn` in api, pull or telemetry; all hits are under `#[cfg(test)]`.
  - Engine `drain` lanes are bounded by `available_parallelism` divided by `SharedBy`. Per-batch spawning (not concurrent growth) is engine-1's ground.
  - The rayon pools in `index_stop_search` (cores), `boolean_catalog_prepared` and `boolean_oos_command` (lanes ≤ cores) are built per command and dropped.
- **Vendor and HTTP bodies.**
  - pull `http.rs` streams with `MAX_RESPONSE_BYTES` and has a `REQUEST_TIMEOUT_SECS` client timeout.
  - `cash_session_cache::read_limited` checks `fstat` and then reads through `take(limit + 1)`.
  - Store and readonly opens are non-blocking against FIFOs (D-1432, D-0955).
  - Info only: `pull/src/ssm.rs:832` reads AWS's answer with an uncapped `.text()`, bounded by the credential timeout and a trusted endpoint.
- **Already reported or documented, not repeated.**
  - The autopilot tick's manifest reads (o1surface2-2) and the pull landing on workers (o1surface2-3). The recovery `drive`/`execute` journal fsyncs inside `tokio::spawn` (recovery.rs:808-827) are the same class as o1surface2-3.
  - `/verify.json` inline (W1-api6-0).
  - The census miss cost (W1-api5-2).
  - The `note_request` emit (§46).
  - Inode growth of one file per audited request (D-1445).
  - `run_owed` (D-1445).
- **Child processes.** `open_in_browser` (server.rs:18189-18199) drops the `Child` without `wait`, so one zombie remains per api start until exit. Info.
- **Other.** `operation_audit::read` issues `sync_all` on the index and on the invocation file for every status read (cli/src/operation_audit.rs:608, 632). It runs on the blocking pool behind a permit, so this is a cost, not a defect.


---

## Pass 3 (2 auditors, family sweeps, at final/all-fixes-zero 5140aca3)

Pass 3 found 6 new findings: 0 high, 1 medium, 5 low. It also built coverage tables of 37 durable writers and 33 non-blocking lock sites; those tables are in conc-pass3/.

- **ledgers-3 (medium):** a kill mid-record, with no write error, wedges Pre-Admission V1/V2, Base Evidence V2, Candidate Universe, Execution V4, Observation V1/V2 and Lineage V4. Rollback cannot help a killed process. docs/06-limits.md does not cover these.
- **ledgers-1 (low):** the new arm at index_stop_vix.rs:411 (D-1760) treats a 112-byte but unsynced complete.bin as committed after this call's own finish failed.
- **ledgers-2 (low):** a failed fsync is retried into success at Base Evidence V2, Candidate Universe, Pre-Admission, Lineage V4, Statistics V3, Admission/Finalization V4, Observation, Population V6, Execution V4, Selection V6, Global Replay V4, Boolean write_or_equal and attempts.bin.
- **locks-1 (low):** take_serve_lock reports ENOLCK/ENOTSUP as "another api is serving" and quotes a dead pid.
- **locks-2 (low):** the five index-stop JSON caches keep their try_lock slot through an aborted fetch, so the replacement request gets a 503 with no retry.
- **locks-3 (low):** search_checkpoint publishes the complete marker before releasing the payload lock, so readers get "busy".

Fix verification: search-1, cli3-1 and replay-2 are FIXED by D-1760. indexstop-2 (c) is FIXED; indexstop-2 (a)/(b) and expr-2 are NOT FIXED and were not claimed. Nothing else in these families changed between 331b05c and HEAD.

### Pass 3 reports

---

<!-- conc-pass3/ledgers.md -->
Verdict: 3 new findings (0 high, 1 medium, 2 low) in the append-only durable-writer family at 5140aca3. D-1760 fixes search-1, cli3-1 and replay-2, but its new `Err(_) if committed` arm in `index_stop_vix::publish` reports success after the call's own failed fsync.

### conc-pass3 / ledgers: workspace-wide sweep of every append-only durable writer in non-test code

Method: source only. No cargo was run and nothing in the repository was edited. I grepped every crate for `write_all`, `SeekFrom::End`, `.append(true)`, `create_new(true)`, `set_len(` and `sync_all`/`sync_data`, dropped `#[cfg(test)]` code and test drivers (`store/src/emits.rs`, `api/src/emitted.rs`, `pull/src/emit_sites.rs`), and read the append path and the writable-open path of each writer that remained. Liveness follows gaps-1 and pass-1 sel/cli3/replay. A "dead" module is compiled outside tests but has no production caller.

Kernel facts relied on, as in the earlier passes:
- **K1.** A buffered `write(2)` copies page by page. `generic_perform_write` stops at the next page when a fatal signal is pending and returns the bytes it already copied. SIGKILL, the OOM killer and an unhandled SIGINT (Ctrl-C) are all fatal; the cli installs no handler (ledgerall-1). So a record that straddles a page boundary can be cut mid-record.
- **K2.** After a writeback EIO, ext4 and XFS report the error once per open file description (errseq), mark the pages clean and leave them readable in the page cache. A later `fsync` on a new descriptor returns `Ok` and does not rewrite those pages. This depends on the filesystem; it is UNVERIFIED here and not measured, as in resources-1 and store1-2.

---

## ledgers-1 (low): `index_stop_vix::publish` turns this call's own failed receipt or directory fsync into a successful publication

**Where.** `crates/cli/src/index_stop_vix.rs:405-413`, added by D-1760. The receipt is written at `boolean_candidate_persistence.rs:66-78` (`Pending::finish`), through `write_or_equal` (:230-244) and `retained` (:246-257).

```rust
let published = catalog.with_current(|| {
    ...
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
    pending.verify_body(digest, body.len() as u64)?;
    pending.finish(lookup, digest, body.len() as u64)?;
    Ok(())
});
match published {
    Ok(()) => Reader::open(root, identity, pin, bounds),
    // A concurrent publisher of the same catalog finished first, and its
    // capture saw a different VIX store: its receipt is the answer.
    Err(_) if persistence::committed(&directory)? => saved(root, identity, pin, feed, bounds),
    Err(why) => Err(why),
}
```

```rust
pub(crate) fn committed(directory: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(directory.join("complete.bin")) {
        Ok(meta) => Ok(meta.len() == RECEIPT_LEN),
```

**Why it is wrong.**
- `finish` creates `complete.bin` with `create_new`, writes all 112 bytes, and only then calls `sync_all` (`retained`), followed by the directory `sync_all`. If either sync fails, `finish` returns `Err`, and the 112 bytes are already visible in the page cache.
- The arm is meant for a concurrent publisher that won the race. It does not ask whose receipt it is looking at. `committed()` is a length check, so it is `true` for this call's own unsynced receipt. `saved` → `Reader::open` then re-reads body and receipt from the page cache, they match, and `publish` returns `Ok(Reader)`.
- The error is dropped with `Err(_)` and never logged. The index-stop catalog then binds this VIX companion as published authority. This is §4's "fallback that hides a failure", and resources-1's in-call shape at a new site that D-1760 created.
- The same arm also hides any other error once a whole receipt exists (for example a `capture` refusal). The race comment justifies only one of those cases.
- The pre-check at :390 has the same weakness without any failure in this process. It runs with no owner lock, so it can return a companion whose receipt the publisher (a second process) has written but not yet synced, and whose sync then fails.

**Repro.**
1. Put the store root on a dm-flakey (or dm-error) target that fails writes for the window after `complete.bin` is created. Alternatively, fill a thin-provisioned volume so that writeback fails at fsync.
2. Run an index-stop verb that publishes a VIX companion for catalog C.
3. `Pending::finish` → `write_or_equal(complete.bin)`: `create_new` succeeds, `write_all(112)` succeeds, and `retained` → `sync_all` returns `EIO`. `finish` returns `Err("Input/output error")`.
4. `published` is `Err`. `committed(&directory)` returns `true` (length 112 from the page cache). `saved(...)` returns `Ok(Reader)`, and the command reports the companion as published.
5. After the cache is dropped or the machine reboots, `complete.bin` holds whatever reached the device. If its length survives but its bytes do not, every later `publish` takes the :390 shortcut, `Reader::open` refuses "receipt changed", and `prepare_in_namespace` compares the body only because the receipt is "whole". The D-1760 recovery never runs, and the identity is wedged again. (Which outcome you get depends on the filesystem, per K2.)

**Minimal fix.**
- Bind the race arm to the race. Take it only when the error is the owner-lock refusal from `prepare_in_namespace` (`"... already owned or lock refused"`), or when `prepare` failed before this call wrote anything. Never take it after this call's own `finish` returned `Err`; return `why` there.
- Do not discard the error. If the arm stays, wrap it: `Err(why) if committed => saved(...).map_err(|s| format!("{why}; {s}"))`, and emit an error event either way.

---

## ledgers-2 (low): failed-fsync-retried-into-success at the exact-retry and reuse paths that pop1-4, resources-1, store1-2 and replay-3 did not name

The class is already reported, but only for named sites: Population V1 (pop1-4), runs/frontier/trades `ensure_*` (resources-1), the bar store (store1-2) and checksum receipts (replay-3). Each proposed fix is per site ("poison the handle", "never call `verify` after `first`"). The live writers below have the same hole, and none of those fixes touches them.

| writer (live) | where the failed sync leaves bytes | where a later run re-syncs or reuses them and reports success |
|---|---|---|
| Base Evidence V2 (store root) | `population_base_evidence_ledger_v2.rs:1239-1241` (`sync Base records` `?` with the records left in place) | the orphan exact retry at :1205-1231 → :1237-1241 (`sync_data` Ok on a new fd); the reuse path at :1179-1191 ("re-sync reused Base records/completion") |
| Candidate Universe (store root) | rows `sync_data` before the receipt (the cand-1 window) | the orphan exact retry in `append_complete_locked` |
| Pre-Admission V1/V2 (store root) | Data `sync_data` at `pre_admission_data.rs:1378`/`:2600` | the trailing-orphan exact retry at :1328/:2550 |
| Search Lineage V4 | members `sync_data` at `anchored_search_lineage_v4.rs:976` | the trailing retry re-sync at :937-940 ("sync retry search-lineage V4 members"), and reuse at :918-925 |
| Statistics V3, Admission V4, Finalization V4 | data `sync_all` before the Completion | the receipt-less trailing-prefix exact retry (the pop2-4 path) re-syncs and appends the Completion |
| Observation V1/V2 | `population_observations_v1.rs:2278-2282`: `?` returns before `self.orphan` is set | next process: `orphan == data` (:2249-2255), then the Completion is appended and synced |
| Population V6 | `population_v6.rs:2170-2172` | `trailing` exact retry (:2116-2134), then the Completion `sync_all` |
| Execution V4 | data `sync_data` at `execution_v4.rs:2952` and siblings | `TrailingExecutionV4` exact-prefix resume |
| Selection V6 | `selection_v6.rs:343` / `:350` | `found` → `sync_all` → `Ok(false)` (:304-310); a partial exact prefix is resumed |
| Global Replay V4 | `global_replay_v4_store.rs:62`/`:69` | full-length rerun: compare, then `sync_all` (:30-69) |
| Boolean/index-stop evidence (11 namespaces) | `retained` `sync_all` (`boolean_candidate_persistence.rs:247`) | a committed reuse: `write_or_equal`'s `AlreadyExists` arm (:237-242) compares and **never fsyncs the file at all**; only the directory is synced. This is replay-3's exact shape, in a module replay-3 did not cover. |
| sweep-evidence `attempts.bin` | `append_events` barrier (`sweep_evidence.rs:1383-1386`) | the next `allocate` counts the unsynced rows and allocates after them; later barriers succeed |

**Concrete repro (Base Evidence, store-wide blast radius).**
1. `cli ledger-all ...` reaches `append_locked` for a universe U. The records `write_all` succeeds and `self.record_file.sync_data()` (:1239) returns `EIO`. The run fails with "sync Base records".
2. The operator reruns the same command on the same binary, with the store unchanged.
3. `open` → `scan` → `scan_orphan` finds U's records, read from the page cache.
4. `append_locked` takes the orphan branch: `compare_prepared_prefix` matches, so it appends 0 records. `sync_data` returns `Ok`: the first process saw the error, and this descriptor was opened after it (K2).
5. The Completion is appended and synced. The result is `Written`.
6. After eviction or a reboot, the records under U's durable Completion read back stale. `scan` → `validate_block` (:1029) refuses on every open, writable or read-only. Every Step-3 run on the store then refuses, because this file is in the store root (ledgerall-1).

**Minimal fix.** Apply the store1-2/resources-1 rule once, in a shared helper used by every writer above. After any `sync_all`/`sync_data` error on a ledger file:
- write a durable "uncertain" marker next to the file, or refuse in-process and also refuse a later exact retry that would complete a prefix written before that error;
- never let a later `fsync` on a fresh descriptor vouch for bytes it did not write.

The cheapest sound form: in every exact-retry and reuse path, rewrite the reused prefix bytes (`pwrite` the same bytes) before the `fsync`. The pages are then dirty again and actually reach the disk. For `write_or_equal`'s `AlreadyExists` arm, at least `sync_all` the file, as replay-3 asks for checksum receipts.

---

## ledgers-3 (medium): a kill or power loss mid-record wedges the store-root Pre-Admission, Base Evidence and Candidate Universe ledgers, and the Execution V4, Observation and Lineage V4 roots. Each has no recovery path, and each site has only been reported for the write-error case

**Where.** In every case the open path refuses any non-stride length before any orphan or retry logic runs, and the refusal applies to read-only opens too:
- Pre-Admission V1/V2. Writes: `pre_admission_data.rs:3595-3599` and `:3817-3824`. Refusal: `:3567-3571` and `:3789-3793`. Records are 740 and 812 bytes.
- Base Evidence V2. Writes: `population_base_evidence_ledger_v2.rs:1463-1503` (one `write_all` per 1,024-byte record) and `:1507-1530`. Refusal: `:1590-1593` ("ragged … body").
- Candidate Universe. Writes: `candidate_universe.rs:6258-6301` (32 rows per `write_all`, multi-page). Refusal: `:6158-6162` ("payload … is ragged").
- Execution V4. Write: `execution_v4.rs:4931-4935`. Refusal: `checked_record_count`. Records are 1,280, 128 and 1,024 bytes.
- Observation V1/V2. Writes: `population_observations_v1.rs:2278-2305` and `:3433-3446`. Refusal: `:2552` ("fixed-stride file is ragged"). Records are 512 and 1,024 bytes after a 64-byte header.
- Search Lineage V4. Write: `anchored_search_lineage_v4.rs:1645-1665`, both 768-byte members in one write. Refusal: `:1625`.

```rust
if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V1) {
    return Err(format!("pre-admission file body has {body} bytes, ragged against ..."));
```

**Why it is wrong, and why it is not already reported.**
- Earlier reports named only the write-error case at these sites: search-2 (Pre-Admission), sel-1 (Execution V4) and pop1-2 (Observation). Their fixes are rollback on `Err`, and that cannot run when the process dies inside `write` (K1).
- The kill-mid-record case was reported only for Admission, Finalization and Statistics (pop2-3), the result chain (sweep-2), the dead ledgers (cli3-3) and Population V6 (ledgerv6's pop2-3 verification).
- Pass-1 *search* waved Candidate Universe and Lineage V4 through as "documented in docs/06-limits.md". They are not documented there. The only limits text on kill-torn tails is "Interrupted Population ledger writes" (`06-limits.md:10438-10460`), which names Admission V3/V4 and Finalization V3, and the D-1631 section, which covers Lineage V2/V3, not V4. Base Evidence and Pre-Admission appear nowhere.
- The bytes past the last whole record are provably unacknowledged: every one of these ledgers is receipt-last, and the tail has no Completion. Even so, the writer holding the exclusive lock refuses them exactly as a reader does.
- Three of these ledgers are in the **store root** (`ledger_all.rs:874`, `roots.source = store_root()`). One Ctrl-C therefore wedges every `ledger-all` and `ledger-v6` run on the store, for every span and every ROOT. It also wedges `stored_family_v6`'s `open_read` (`:169`, `:200`) of every universe already committed.
- This is a different failure from ledgerall-1. ledgerall-1 is a whole-row orphan that the exact retry could still complete. A sub-record tail is refused before the orphan logic runs, so not even the identical rerun on the identical binary recovers.

**Repro (no rebuild, no data change).**
1. On 2026-10-03, run `cli ledger-all V 2025 1 2025 12 S P /x/ledgers`.
2. Press Ctrl-C while `append_rows` (`candidate_universe.rs:6283`) is writing a 32-row chunk. The chunk spans several pages, so the window is the whole syscall. With `strace -e write` attached, `kill -INT` on that write reproduces it every time. The cli has no SIGINT handler, so the signal is fatal (K1), and the row file ends at a page boundary that is not `64 + k·stride`.
3. Rerun the identical command. `CandidateUniverseLedgerV1::open` → `checked count` → "candidate row file payload … is ragged for fixed stride …". All 16 families refuse, and so does every later `ledger-all` or `ledger-v6` on that store.
4. `GET` or `cli` readers of committed universes (`open_read`) refuse the same way.

The same steps work with Base Evidence (every 4th record straddles a page), with Pre-Admission (740- and 812-byte records straddle pages regularly), and, per rung root, with Execution V4, Observation and Lineage V4.

**Minimal fix.** As in pop2-3 and sweep-2, implemented once and shared by all of these writers. In the writable `open`, under the exclusive writer lock:
1. If `body % stride != 0` and the bytes before the remainder validate as whole records, `set_len(header + whole·stride)`.
2. `sync_all` the file.
3. Emit an error-level telemetry event that names the file and the cut byte range.

Read-only opens keep refusing until a writer has healed the file. A whole-record tail with a bad seal must still refuse. Record the rule in `docs/05-decisions.md` and list these ledgers in `docs/06-limits.md`.

---

## Full writer table (coverage)

Legend: **RB** = rolls back (`set_len`) on a failed write. **Kill-tail** = what a writable open does with a sub-record tail left by a kill or power loss. **Retry identity** = whether an interrupted block's exact retry can be replayed after a rebuild or new data. **Sync** = whether a failed `sync_all` can later be confirmed as success. "Named" gives the earlier report that covers the cell.

| # | writer | file:line (append / refusal) | live | RB | Kill-tail on writable open | Retry identity (rebuild / new data) | Failed sync retried into success | Named / new |
|---|---|---|---|---|---|---|---|---|
| 1 | api pull audit journal | api/src/audit.rs:1172-1220 / :1205-1212 | yes | no, but not needed: 256-byte records in an O_APPEND file never straddle a 4 KiB page, and block allocation is all-or-nothing | refuses forever (D-0402, limits §97, documented) | N/A (no retry identity) | no: error returned, record later counted by `look` | recovery-4, press-2, autopilot-4; clean for this family |
| 2 | api recovery journal | api/src/recovery_journal.rs:369-460 | yes | 1024-aligned; handle poisoned on uncertain I/O | refuses (documented) | N/A | open re-syncs the file before exposing it (minor, K2 class) | recovery-2; checked sound |
| 3 | api recovery STOP control | api/src/recovery_control.rs:147-168 | yes | via #2 | via #2 | N/A | if state already equals status, no append; only the parents are synced (relies on #2's open sync) | runs-3, recovery-5 |
| 4 | cli operation audit index + per-invocation | cli/src/operation_audit.rs:324-347, 512-575 / :306-312 | yes | no, but not needed (256 B, aligned) | refuses | N/A (fresh id) | per-invocation handle poisoned (`failure`); `read` re-syncs the index (status only) | cli1-1/2/3/5 |
| 5 | sweep-evidence attempts/starts/lifecycle/levels/ranked | cli/src/sweep_evidence.rs:1364-1397, 1282-1297, 601-640 / :1195-1200 | yes | **no** | refuses | N/A (token-scoped) | yes: unsynced rows counted by the next `allocate` (ledgers-2) | cli2-1, sweep-2; sync: **ledgers-2** |
| 6 | sweep-evidence `reserve_start` | sweep_evidence.rs:1045-1065 | yes | n/a (create_new per token) | that token only | N/A | no | clean |
| 7 | results `runs.bin` | cli/src/results.rs:1268-1370 / :951-966 | yes | yes | refuses | identity has commit and digest, so a rebuild gets a new identity (no wedge) | yes | sweep-2, resources-1 |
| 8 | `frontier.bin` | cli/src/frontier.rs:1050-1120 / :1858-1868 | yes | yes | refuses | same as 7 | yes | sweep-2, resources-1, cli2-3 |
| 9 | `chosen-trades.bin` | cli/src/trades.rs:622-712 / :1185-1192 | yes | yes | refuses | same as 7 | yes | sweep-2, resources-1 |
| 10 | `detail-sets.bin` | cli/src/result_set.rs:449-490 / :515 | yes | yes | refuses | same as 7 | class | sweep-2 |
| 11 | candidate-trades detail files | cli/src/candidate_trades.rs:1307-1348 | yes | n/a (create_new per token) | dead token only | N/A | no (a token is never retried) | cli2-5, xcut-3 |
| 12 | expression evidence | cli/src/expression.rs:218-345 | yes | pending + link | pending file left in a dead token | N/A | no | clean (H5) |
| 13 | search checkpoint journal | cli/src/search_checkpoint.rs:208-290 | yes | per-sequence directory | interrupted hole skipped | N/A | writer poisoned | GAP11-0; clean |
| 14 | Boolean/index-stop evidence (11 namespaces) | cli/src/boolean_candidate_persistence.rs:88-260 | yes | rewrites unreceipted scratch (D-1760) | **now recovered** (D-1760) | recovered (D-1760) | yes: the `AlreadyExists` reuse never fsyncs | search-1 FIXED; **ledgers-2** |
| 15 | VIX companion publish | cli/src/index_stop_vix.rs:374-413 | yes | via 14 | via 14 | via 14 | **yes, inside one call** | **ledgers-1** |
| 16 | checksum receipts | cli/src/checksum_receipts.rs:306-360 | yes | n/a | n/a | content-addressed | yes | replay-3, replay-4 |
| 17 | Candidate Universe (store root) | cli/src/candidate_universe.rs:3666-3770, 6258-6324 / :6158-6162 | yes | yes | **refuses forever** | **wedges (commit and data digest)** | yes | cand-1, ledgerall-1; kill: **ledgers-3**; sync: **ledgers-2** |
| 18 | Base Evidence V2 (store root) | population_base_evidence_ledger_v2.rs:1158-1250, 1463-1530 / :1590 | yes | yes | **refuses forever** | wedges | yes (explicit "re-sync reused") | ledgerall-1; **ledgers-3**, **ledgers-2** |
| 19 | Pre-Admission V1/V2 (store root) | pre_admission_data.rs:1307-1395, 2529-2615, 3595, 3817 / :3567, :3789 | yes | **no** | **refuses forever** | wedges | yes | search-2, ledgerall-1; **ledgers-3**, **ledgers-2** |
| 20 | Statistics V3 | population_statistics_v3.rs:2371-2490, 2903 / :2868 | yes | **no** | refuses | wedges | yes | pop2-2/3/4/5/6; sync: **ledgers-2** |
| 21 | Observation V1/V2 | population_observations_v1.rs:2217-2310, 3398-3450 / :2552 | yes (via Statistics V3) | **no** | **refuses forever** | foreign orphan refuses (:2249-2255) | yes | pop1-2, pop1-3; **ledgers-3**, **ledgers-2** |
| 22 | Search Lineage V4 | anchored_search_lineage_v4.rs:911-1010, 1645-1665 / :1625 | yes | yes (D-1620) | **refuses forever** | wedges (pair id comes from Pre-Admission ids, which carry the commit); unenumerated site of pop2-4 | yes (explicit retry re-sync) | **ledgers-3**, **ledgers-2** |
| 23 | Admission V4 | population_admission_v4.rs:2698-2810, 3052-3080 / :2618-2625 | yes | yes | refuses | wedges | yes | pop2-3/4/7; sync: **ledgers-2** |
| 24 | Finalization V4 | population_finalization_v4.rs:2328-2440, 2718 / :2230 | yes | **no** | refuses | wedges | yes | pop2-2/3/4; sync: **ledgers-2** |
| 25 | Population V6 | population_v6.rs:2098-2190, 2444 / :2017-2019 | yes | **no** | refuses | wedges ("foreign retry cannot replace trailing prefix", :2116); unenumerated site of pop2-4 | yes | pop1-2, ledgerv6 (pop2-3 verification); sync: **ledgers-2** |
| 26 | Execution V4 | execution_v4.rs:2707-2960, 4931 / :4893-4896 | yes | **no** | **refuses forever** | foreign orphan refuses (documented EX-03) | yes | sel-1; **ledgers-3**, **ledgers-2** |
| 27 | Selection V6 | selection_v6.rs:299-360 | yes | no, but an exact byte prefix resumes | partial block refuses every read until the exact retry | wedges on a source change | yes (`found` → sync) | hunt-cli-a-5; sync: **ledgers-2** |
| 28 | Global Replay V4 | global_replay_v4_store.rs:10-80 | yes | byte-prefix resume | resumed | content-addressed (clean) | yes (full-length re-sync) | **ledgers-2** (minor) |
| 29 | store bar month | store/src/file.rs:2021-2175 | yes | header-slot commit | tail past `n_valid` tolerated | N/A | yes | store1-1, store1-2 |
| 30 | store repair revision | store/src/repair.rs:330-405 | yes | reservation never removed | revision Incomplete | per revision | n/a | store2-1 |
| 31 | census manifest | pull/src/ingest.rs:3296-3330 | yes | slot-counter commit | tail past the counter overwritten | N/A | slot `sync_all` failure has the store1-2 shape (unenumerated, derived index) | pull2-1/5, census-1, xcut-2 |
| 32 | NSE cash-session cache | pull/src/cash_session_cache.rs:483-528 | yes | n/a (create_new) | payload without receipt wedges the day | N/A | no | pull2-3 |
| 33 | vendor capture | pull/src/capture.rs:247-260 | yes | n/a | torn fixture | N/A | no | pull1-3 |
| 34 | telemetry sink | telemetry/src/sink.rs:262-272, 1161-1195, 1499 | yes | newline-terminates after a failed append | newline-terminated at open | N/A | no | telemetry-1, xcut-1, lifecycle-3; clean here |
| 35 | Population V1, V5; Admission V2/V3; Finalization V2/V3; Statistics V2 | population*.rs (see pop1/pop2) | per pop1/pop2 | mixed | refuses | wedges | yes | pop1-1..4, pop2-1..6 |
| 36 | dead: admission_store, institutional_statistics, stored_data_completeness | as cli3-3 | no | yes | refuses | — | institutional documents "failed sync → orphan an exact retry continues" | cli3-3 |
| 37 | dead: global_replay v1/v2/v3, selection v1-v5, execution_v3, execution_capability, execution_disposition_v2, lineage v2/v3 | various | no | v5/capability/disposition yes; others no | refuses | — | — | sel "checked", limits D-1631 |

---

## Fix verification (pass-1/pass-2 findings in this family that 331b05c..HEAD claims to fix)

| id | verdict | reason (code at 5140aca3) |
|---|---|---|
| search-1 | **FIXED** | `prepare_in_namespace` (`boolean_candidate_persistence.rs:119-132`) runs under the exclusive owner lock. When `committed()` is false (no 112-byte `complete.bin`), it unlinks any torn receipt and any earlier body, then writes `body.bin` fresh. A crash or short write before the receipt is no longer compared against, so the exact retry recovers. A committed directory still compares (history kept). Residual sync weakness: ledgers-2, row 14. |
| cli3-1 | **FIXED** | `index_stop_vix::publish` now shortcuts only on `persistence::committed(&directory)` (:390), not on the directory existing. An interrupted publish falls through to `prepare_in_namespace` and resumes. |
| replay-2 | **FIXED (permanent wedge); transient refusal remains** | An unreceipted body is scratch and is rewritten, so a retry whose capture differs no longer hits `write_or_equal`'s mismatch. A concurrent publisher that loses `try_lock` while the winner has not yet finished still gets an error (a transient refusal; a retry succeeds). The new error arm introduces ledgers-1. |

No other finding in this family (pop1-*, pop2-*, sel-1, search-2, cli2-1, cli3-3, sweep-2, cand-1, ledgerall-1, resources-1, store1-2, replay-3/4, pull1-3, pull2-3) is touched by 331b05c..HEAD. `git diff --stat` shows no change to their files, apart from doc-only edits in `frontier.rs` and the retired-stride refusal in `execution_capability.rs` and `execution_disposition_v2.rs`.

## Checked and clean (this family)
- 256- and 1,024-byte-aligned journals (api audit, operation audit, recovery journal) cannot be cut mid-record by K1, because no record crosses a page.
- Global Replay V4 and the Boolean evidence are content-addressed or byte-prefix resumed, so a rebuild cannot wedge them.
- The run-identity result chain (runs, frontier, trades, detail-sets) has the commit and data digest in each identity, so a rebuild produces a new identity rather than a foreign-orphan wedge.
- Every `set_len` rollback measures `end` under the same exclusive lock it truncates under.

---

<!-- conc-pass3/locks.md -->
### conc-pass3 / locks: every non-blocking lock acquisition in non-test code, at 5140aca3

**Verdict: 3 new findings (0 high, 0 medium, 3 low).** Every other non-blocking site has a defect already named in concurrency.md passes 1-2, or I found none. D-1760 fixes the permanent half of indexstop-2. Its concurrent-publisher clause is PARTIAL.

Method: I grepped `try_lock|try_lock_shared|LOCK_NB|flock(` across `crates/` and dropped `*_tests.rs`, `tests/` and `#[cfg(test)]` modules: telemetry sink.rs:1792, sweeprun.rs:3436, mastersrun.rs:926, server.rs:20989/21110/21134/24988, audit.rs:2371, selection_v4_authority.rs:1641-1646, population.rs:7745-7752 and admission_join.rs:934 are all test code. No production `libc::flock`, `LOCK_NB`, `RwLock::try_read` or `RwLock::try_write` exists. `server.rs:17091 try_write` is a socket write. `store/src/flock.rs:109/121` is the wrapper itself. I read the source only and did not run cargo.

## Site table (one row per production site)

"Prior" is the concurrency.md finding that already names the defect at that site. "—" means none, and no defect beyond the row's note.

| # | Site | Lock / mode | Who else can hold it at the same time | Loser does | Refusal text truthful? | User-visible consequence | Prior |
|---|---|---|---|---|---|---|---|
| 1 | store/src/file.rs:1248 `open_or_create` | month `.lock`, EX, try | another writer (in-process ingest; cross-process none, because only api ingests); **any reader's SH** (`open_existing`, audited reads, `/bars/window.json` x240, cli `stored::load`, strict ranges for the whole run) | refuse (`StoreError::Locked`) | **No**: says "another writer holds" when the holder is a reader. Host error is classified separately. | pulled month lost (re-fetched later at quota cost); derived rung never re-derived for a past month | barflow-1; ledgerv6-1 (whole-run hold) |
| 2 | store/src/file.rs:1403 `open_existing` | month `.lock`, SH, try | the writer's EX | refuse `Locked` | yes (the only conflicting holder is a writer) | read page or sweep refuses during an ingest of that month | replay-5 (late VIX open), store1-1 (absent `.lock` → unlocked) |
| 3 | store/src/file.rs:1444 `open_existing_audited` | month `.lock`, SH, try, held for the AdmittedMonth's life | writer EX | refuse (`why.to_string()`) | vague: std "operation would block", names no holder | strict sweep refused during a pull; while held, it refuses writers for the whole run | ledgerv6-1, barflow-1 |
| 4 | store/src/repair.rs:448 `shared_lock` | source/revision `.lock`, SH | writer EX | refuse `Locked` ("another writer holds") | yes | none in production (no caller outside tests) | store2-1 (latent) |
| 5 | pull/src/ingest.rs:3156 `CensusLock::take` | `<vendor>.man.lock`, EX, try | another same-vendor ingest in-process (seats prevent most of these); no cross-process writer | refuse; **silently proceeds unlocked** on EMFILE/ENOSPC open errors | yes (WouldBlock and host error are split, D-0955) | rows lost on the unlocked arm | pull2-1 |
| 6 | pull/src/cash_session_cache.rs:390 `lock_day` (callers :270, :314, :491) | per-day cache `.lock`, EX, try, held across a gunzip and parse of up to 32 MiB | another ingest validating or installing the same day; `/gaps.json` and the recovery auditor (same EX); `read_local_lifecycle` SH | refuse ("unavailable") | neutral | equity window refused; combined with equity-1, a permanent hole | pull2-4, equity-2 |
| 7 | pull/src/cash_session_cache.rs:121 `read_local_lifecycle` | per-day `.lock`, SH, try | row 6's EX holders | refuse "UNVERIFIED … unavailable" | yes | recovery lifecycle read refused | equity-2 |
| 8 | api/src/server.rs:17800 `take_serve_lock` | `<store>/serve.lock`, EX, try | a second `api serve` process | refuse, exit | **No on a host error**: both arms say "another brutex api is already serving" and quote whatever pid is in the file | **locks-1** | — |
| 9 | api/src/audit.rs:1190 `Journal::appended` | `audit/pull.journal`, EX, try, across write + fsync | parallel press legs; recovery receipt; autopilot tick; refused-request records (all in-process) | refuse | hedged ("may be appending") | run/receipt record lost from `/audit`; recovery BLOCKED | server1-1, recovery-4, recauto-2 |
| 10 | api/src/recovery_journal.rs:315 `open_at` | plan/attempts/active/stop `.bin`, EX, try, held for the Journal's life | `snapshot` SH (preflight of a second start/prepare); a running recovery's worker EX | refuse WouldBlock "already exclusively locked; no work was admitted" | slightly off: the holder can be a shared snapshot. In-process only, and STOP/is_stopped serialise on `recovery_active` | a racing double start or prepare is refused | — (checked; no material defect) |
| 11 | api/src/recovery_journal.rs:639 `snapshot` | same files, SH, try | a live writer EX (running recovery) | refuse "has a writer" | yes | a second start or prepare is refused while one runs (intended) | — |
| 12-16 | api/src/indexstopvixjson.rs:130, indexstopcandlesjson.rs:159, indexstopqualificationjson.rs:150, indexstopjson.rs:131, indexstoprankingjson.rs:185 | process-wide `Mutex<Option<Cached>>`, `try_lock` | **the same viewer's own abandoned request** (still running in `spawn_blocking`); a second tab | refuse 503 "…busy; nothing/no request queued". Poisoned is mapped to busy too (unreachable in release, panic=abort) | yes ("busy") | **locks-2** | — (pass 2 listed it as the good design) |
| 17 | cli/src/execution_lease.rs:66 (`acquire` and `probe`) | `execution.lock`, EX, try | a real sweep (cli or browser); **a `/backtest/run.json` probe holding EX** | refuse `Busy` | **No** when the holder is a probe ("another sweep owns…") | FAILED cli sweep / 409 launch with no sweep running | cli1-3, runs-2 |
| 18 | cli/src/operation_audit.rs:522 `begin` | `invocation index.bin`, EX, try, across write + fsync | every audited HTTP request's begin (in-process api); cli begins; readers' SH (:599) across `sync_all` | refuse BUSY | yes-ish ("another operation holds") | cli FAILED; browser 429 then Run locked | cli1-2, log-1 |
| 19 | cli/src/operation_audit.rs:599 `read` | index, SH, try, across `sync_all` | `begin` EX | refuse BUSY | yes | `/backtest/audit.json` 429 | log-1 |
| 20 | cli/src/operation_audit.rs:326 `append` | per-invocation file, EX, try (raw `File::try_lock` + `unlock`) | nobody: readers deliberately take no lock on it, and the State sits behind a Mutex | n/a (the WouldBlock arm is unreachable) | — | none | — (checked) |
| 21 | cli/src/search_checkpoint.rs:154 `Journal::open` | `owner.lock`, EX, try | a second writer of the same search; **any `Snapshot::open` probe SH** (:108) | refuse "already owned or cannot be locked" | **No** when the holder is a probe | false "already owned" refusal; attempt sealed Refused | expr-1, indexstop-2(b) |
| 22 | cli/src/search_checkpoint.rs:108 `Snapshot::open_through` probe | `owner.lock`, SH, try, released immediately | the writer's EX → sets `writer_observed` (correct) | proceeds (it is a probe) | n/a | causes row 21 | expr-1 |
| 23 | cli/src/search_checkpoint.rs:494 `read_saved` | `<seq>/payload`, SH, try | **the publishing writer's EX, still held after the `complete` marker is visible** | refuse "checkpoint payload is busy or cannot be read" | yes ("busy") | **locks-3** | — (3259 named only the 0-byte-marker sub-window) |
| 24 | cli/src/boolean_candidate_persistence.rs:110 `prepare_in_namespace` | namespace `owner.lock`, EX, try, held through `finish` | a concurrent publisher; **reader leases SH** (rows 25-27, Observation::open) | refuse "already owned or lock refused" | **No** when the holder is a reader | rung or batch refused, campaign checkpoints burned | expr-2, indexstop-2(a) |
| 25 | cli/src/boolean_candidate_persistence.rs:270 `read_held` | the file being read, SH, try | `write_or_equal` takes no lock, so nobody | refuse "busy" | yes | none found | — |
| 26 | cli/src/boolean_observation_file.rs:228 `ReadLease::acquire` | `owner.lock`, SH, try (CAS guards one lease per descriptor) | publisher EX | refuse "busy or cannot be locked" | yes | read refused while publishing; it in turn refuses the publisher (row 24) | expr-2, indexstop-2 |
| 27 | cli/src/boolean_campaign_reader.rs:170 | child `owner.lock`, SH, try | re-preparing child EX | refuse "publication is busy" | yes | `/boolean-campaign.json` error; refuses row 24 | expr-2 |
| 28 | cli/src/expression_search_reader.rs:253 | signal `.rows`, SH, try | `EvidenceWriter` EX until the end of `finish` | refuse "busy" | yes | unreachable: the reader requires a Completed attempt receipt first, which is written after `finish` | — (checked) |
| 29 | cli/src/expression.rs:465 `read` | `.rows`, SH, try | the same writer | refuse "busy" | yes | as row 28 | — (checked) |
| 30 | cli/src/checksum_receipts.rs:314 `publish` | receipt, EX, try (raw) | a concurrent publisher of the same receipt (same month audited by two processes) | refuse with std's bare WouldBlock text | vague, names nothing | audit refused; a retry reuses the receipt | replay-3 (durability of the fast path); pass 2 judged the refusal clean |
| 31 | cli/src/checksum_receipts.rs:274 `Receipt::open` | receipt, SH, try, held for the authority's life | a publisher's EX between its seal write and its unlock | refuse bare | vague | transient | replay-3 |
| 32 | cli/src/candidate_trades.rs:852, 1049, 1362 | catalog, trade and detail files, SH, try | the writer's EX in `write_exact` | refuse "candidate detail is busy; retry" | yes | transient 503 | cli2-5 (empty before lock) |
| 33 | cli/src/candidate_trades.rs:1101 `TradeReader::page` | cached trade file, SH, try (raw) | a writer EX (the file is immutable once published) | refuse busy | yes | none found | apicache-1 (staleness, not the lock) |

## Findings

### locks-1 (low): the serve lock reports every flock refusal as "another brutex api is already serving this store" and quotes a stale pid, even when the host refused the lock itself

- **Where:** crates/api/src/server.rs:17800-17827 (`take_serve_lock`).
- **Code:**
  ```rust
  let file = match store::flock::Flock::try_lock(file, path.clone()) {
      Ok(held) => held,
      Err(refusal) => {
          let held_by = std::fs::read_to_string(&path).unwrap_or_default();
          let held_by = held_by.lines().next().unwrap_or_default().trim();
          release_root(&key);
          return Err(format!(
              "REFUSED: another brutex api is already serving this store.\n  \
               store: {}\n  lock:  {} ({refusal})\n  held by: {}\n ...
               Stop the other instance, or point this one at another BRUTEX_STORE.", ...
  ```
- **Why it is wrong:** `refusal` is a `TryLockError`. It is `WouldBlock` only when another description holds the lock. `TryLockError::Error` is the host refusing `flock` (ENOLCK on an NFS or SMB mount without a lock manager, ENOTSUP on a filesystem with no advisory locks). In that case no other instance exists, but the message asserts one does. It then fills "held by:" from line 1 of the file. `serve.lock` is opened without truncation and only re-stamped after a successful lock, so that line is the **last successful holder's** `addr=… pid=…`, which is a dead process. The repo already fixed exactly this conflation for the census lock (`lock_refusal`, ingest.rs:3201-3218, R9-csr-cx-1/D-0955: "Only WouldBlock means another run holds it… Both arms used to say there was one"). It also fixed the stale-pid misattribution twice (D-1446, D-1481), but not on this arm. `execution_lease::lock` and `store::file::lock_fault` split the two arms; this site does not.
- **Repro:** Run `api serve` once on store S, which stamps `addr=127.0.0.1:8080 pid=4242`, then stop it. Move S to (or mount it from) a filesystem whose `flock` returns ENOLCK or ENOTSUP, for example an NFSv3 export with no lockd, or an SMB share mounted `nobrl`/`nolock`. Run `api serve` again. `try_lock` returns `Err(TryLockError::Error(ENOLCK))`, and the process exits with "REFUSED: another brutex api is already serving this store … held by: addr=127.0.0.1:8080 pid=4242". pid 4242 does not exist, and stopping "the other instance" or changing the port can never help. The refusal itself is correct, since serving without the lock would be unsafe. Only the stated cause is false.
- **Minimal fix:** Match on the error. `TryLockError::WouldBlock` keeps today's text. `TryLockError::Error(host)` returns "REFUSED: the host refused to lock {path}: {host}. No other instance is implied; put the store on a filesystem that supports advisory locks", and does not read or quote the stamp.

### locks-2 (low): the five index-stop JSON caches refuse a viewer's own next request as "busy" (503), because the request the page just abandoned still holds the process-wide slot

- **Where:** crates/api/src/indexstopvixjson.rs:128-130, indexstopcandlesjson.rs:157-159, indexstopqualificationjson.rs:148-151, indexstopjson.rs:129-132, indexstoprankingjson.rs:183-186. Each handler runs `render` inside `crate::detail::run` (detail.rs:100-107, `spawn_blocking`). Client: web/src/lib/index-stop-vix.js:89-93 and index-stop-source.js:77 (`reads.cancel()` then a new read), and detail-refusal.js:3-15.
- **Code:**
  ```rust
  static CACHE: OnceLock<Mutex<Option<Cached>>> = OnceLock::new();
  let mut held = CACHE.get_or_init(|| Mutex::new(None)).try_lock()
      .map_err(|_| "saved-VIX reader busy; nothing queued")?;
  if !held.as_ref().is_some_and(|cached| cached.root == root && cached.identity == asked.identity && ...) {
      *held = None;
      let reader = Reader::open(root, asked.identity, asked.pin, bounds)?;   // cold: verifies the whole saved evidence
  ```
  The `Err(why)` arm maps to `refused(StatusCode::SERVICE_UNAVAILABLE, &why)`.
- **Why it is wrong:** The slot is one per route for the whole process, and it is held for the whole closure, including a cold `Reader::open`, which re-verifies the saved catalog or observation (O(file)). The browser cancels and replaces reads on every selection change ("One read in flight; selection changes revoke stale publication even if abort is ignored", index-stop-vix.js:87). An `AbortController` abort drops the axum future, but `spawn_blocking` cannot be cancelled, so the abandoned closure keeps the guard until it finishes. The replacement request that the same page sends a moment later then gets `try_lock` → `WouldBlock` → 503 "…busy; nothing queued". The page renders it as `phase:'failed'` with no retry: "The saved VIX companion is unavailable… saved-VIX reader busy; nothing queued". The same happens between two tabs on one route. Pass 2 recorded `try_lock` here as the better design, because it does not park a `detail` permit (expr-3, cand-2). That is true, but the cost is that the most ordinary interaction, clicking one trade and then another, refuses the second click. Nobody else is contending: the holder is work whose result nobody will read. No wrong data is served.
- **Repro:** On the index-stop page, open trade A's VIX companion for catalog K1 (cold `Reader::open`, many MB). While it loads, click trade B of a different setting or catalog K2. The client aborts A's fetch and GETs `/index-stop-vix.json?identity=K2…`. The server thread for A is still inside `Reader::open` holding `CACHE`. B's closure → `try_lock` → `WouldBlock` → 503. The panel shows the busy refusal. A's result is discarded on arrival. The user must click B again after A's closure ends. The same applies to `/index-stop-candles.json` (trade candles) and to paging `/index-stop-ranking.json` twice quickly.
- **Minimal fix:** Hold the global mutex only to look up, take or install the `Cached` entry. Move the reader out (`Option::take`), run `Reader::open` and the projection outside the lock, and put it back afterwards. Concurrent requests for other identities then simply open their own reader, and the slot is never held across verification. Alternatively, keep `try_lock` but have the client retry a `busy` refusal once or twice after a short delay (as `fetchWithBusyRetry` does for `/backtest.json`), and answer 429 rather than 503 so the busy refusal matches the server's other busy refusals.

### locks-3 (low): a live checkpoint is visible to readers before its writer releases the payload lock, so polling a running search or campaign gets "checkpoint payload is busy" at every publication

- **Where:** writer crates/cli/src/search_checkpoint.rs:263-281 (`publish_inner`); discovery :457-463 (`discover_through`); reader :494-498 (`read_saved`, via `Snapshot::read` :128).
- **Code:**
  ```rust
  // writer, holding Flock::lock(payload) EXCLUSIVE since :236
  let mut marker = File::create_new(directory.join("complete")).map_err(error)?;   // :263  <- now discoverable
  marker.write_all(&seal).and_then(|()| marker.sync_all())...;                      // fsync
  File::open(&directory)...sync_all()...;                                            // dir fsync
  verify_acknowledged(&mut file, &path, &header, payload, seal)?;                    // :272  re-reads the WHOLE payload
  if regular_bytes(&directory.join("complete"), 32)? != seal { ... }
  file.release()...;                                                                 // :281  payload lock released
  // reader
  Ok(metadata) if metadata.file_type().is_file() => { ...; latest = Some(...max(sequence)); }   // :458-462
  let mut file = Flock::try_lock_shared(readonly_file::open(&path)?, path.as_path())
      .map_err(|why| format!("checkpoint payload is busy or cannot be read: {why}"))?;            // :494-498
  ```
- **Why it is wrong:** `discover_through` treats a `complete` marker as an acknowledged checkpoint and makes it `latest`. The writer creates that marker while it still holds the payload's exclusive flock. It keeps holding it through two fsyncs and a full re-read of the payload, and releases it only at :281. Any `Snapshot` reader in that window picks the new sequence, and its shared `try_lock` on the payload fails with `WouldBlock`. Pass 2's xcut H8 note (concurrency.md:3259) named only the sub-window :263-:265, where the marker is still 0 bytes and the reader says "marker width mismatch". This window is the longer one that follows: it lasts until :281 and includes two fsyncs plus an O(payload) re-read. It also survives the GAP11-0 fix proposed there (temp name, then rename to `complete`), because the renamed marker would still appear before :281. The checkpoint is fully durable and verified at :265, so the reader is refused for bytes that are already published.
- **Who hits it:** every observer of a live search or campaign: the campaign monitor (web/src/lib/campaign-monitor.js polls `/boolean-campaign.json` every 5 s while `running`, through `boolean_campaign_reader.rs:26-33`), unpinned `/index-stop-ranking.json` (`index_stop_search_reader::latest_checkpoint`), `/expression-search.json`, and `boolean_qualified_observer.rs:33/84/176`. All of these are cross-process when the search runs in `cli`, and cross-thread when the browser launched it in `api`.
- **Repro:** Launch a boolean campaign and open its monitor. When the CLI publishes checkpoint N, thread W is between :263 and :281: marker written, fsyncing or re-reading payload. API thread R runs `Snapshot::open` → `discover_through` → `latest = N` → `read(N)` → `read_saved` → `try_lock_shared(N/payload)` → `WouldBlock` → `Err("checkpoint payload is busy or cannot be read: …")`. `/boolean-campaign.json` returns an error. The monitor shows `why` and increments `failures`. Three such hits in a row stop the watch (campaign-monitor.js:28, `failures<3`). For the ranking page the result is a failed load of the newest batch at the moment it lands.
- **Minimal fix:** In `publish_inner`, release the payload lock before the marker is created: after the first `verify_acknowledged` (:258) and the directory fsync (:259-262), call `file.release()`. Keep a plain unlocked handle for the second verification, or drop that verification, since the `complete` marker and the reader's own seal check already cover it. Alternatively, have `read_saved` wait on the shared lock (`Flock::lock_shared`), because the writer's hold after the marker is bounded and the writer never re-locks an acknowledged payload.

## Fix verification (lock-family findings that the new commits claim to touch)

`git log 331b05c..HEAD` contains D-1760..D-1765. Only D-1760 touches a lock-family site (`boolean_candidate_persistence::prepare_in_namespace` and `index_stop_vix::publish`). `store/src/file.rs`, `pull/src/cash_session_cache.rs`, `pull/src/ingest.rs`, `cli/src/operation_audit.rs`, `cli/src/execution_lease.rs` and `cli/src/search_checkpoint.rs` are unchanged. The `api/src/audit.rs` diff is doc-only. So barflow-1, pull2-1, pull2-4, equity-2, server1-1, recovery-4, recauto-2, cli1-2, cli1-3, runs-2, log-1, expr-1, expr-2 and replay-5 are unchanged and still present. None of them is claimed fixed.

- **indexstop-2 case (c), the permanent VIX wedge reached by a reader lease: FIXED.** `index_stop_vix::publish` now shortcuts only on `persistence::committed(&directory)` (a 112-byte `complete.bin`), not on the directory existing. After a writer `try_lock` is refused by an `Observation::open` lease, the directory holds only `owner.lock` and `committed` is false. The rerun therefore falls through to `prepare_in_namespace`, whose `directory()` accepts the existing directory. `try_lock` now succeeds, and because the receipt is not whole, it discards any stale `complete.bin`/`body.bin` and writes the body fresh. The permanent half is gone.
- **indexstop-2 cases (a)/(b) and expr-2 (writer `try_lock` refused by reader leases, worded "already owned"): NOT FIXED** (not claimed). boolean_candidate_persistence.rs:110-114 is unchanged.
- **D-1760's own clause "when its own attempt fails after a concurrent publisher committed, it answers with that committed companion": PARTIAL.** The loser reaches `Err(_) if persistence::committed(&directory)?` (index_stop_vix.rs ~:411) only if the winner's `complete.bin` is whole. But the winner keeps the exclusive owner lock (`Pending.owner`) until its `with_current` closure returns, which is after `finish`'s directory fsync. If the check lands in that window, `saved()` → `Reader::open` → `Observation::open` → `read_held(owner.lock)` shared `try_lock` → `WouldBlock` → "Boolean evidence is busy or cannot be locked", and the loser is refused although the companion is committed. More commonly, the loser's own `try_lock` refusal happens while the winner is still writing its body, so `committed` is false and the loser is refused outright. Either way the next run succeeds, so this is transient and not counted as a finding.

## Checked and clean (no new defect)

- `Flock` (store/src/flock.rs): every guard path unlocks explicitly. Raw `File::try_lock`/`try_lock_shared` sites (search_checkpoint.rs:108, checksum_receipts.rs:314, operation_audit.rs:326, candidate_trades.rs:1101) each pair success with an explicit `unlock()` on every path that follows.
- Lock ordering: every acquisition in the family is non-blocking, so no wait cycle is possible.
- No site in the family silently proceeds unlocked except the already-known ones: pull2-1 (census lock on EMFILE/ENOSPC), store1-1 (absent month `.lock`), and server2-2 (in-process `ServeLock { held: None }`).
- The `Mutex::try_lock` sites map `Poisoned` to "busy". This is unreachable in the shipped binary (`panic = "abort"`, poison-1).


---

## Queue and remaining plan (updated 2026-10-03 after pass 3)

- New findings per pass: 63, then 51, then 6. Pass 3 swept only two families with 2 agents, so the drop is partly a result of its smaller scope.
- Pass 4 waits for the zero-findings thread to land its concurrency fixes. It then verifies those fixes and re-sweeps the same families at the new head, using at most 2 agents while weekly usage is above 80%. Stop when a pass finds nothing new.

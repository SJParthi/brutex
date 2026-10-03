# conc-pass1 / server1: verdict: 1 medium (extends sibling recovery-4 with the main trigger it missed) and 1 low; no high. 2 findings.

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

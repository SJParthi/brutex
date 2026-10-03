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

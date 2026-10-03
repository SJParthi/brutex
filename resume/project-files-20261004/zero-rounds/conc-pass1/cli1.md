# cli1: concurrency and state audit of the cli entry slice (pass 1, commit 331b05c)

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

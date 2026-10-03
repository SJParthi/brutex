# conc-pass2 / resources: 4 findings at 331b05c (0 high, 2 medium, 2 low)

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

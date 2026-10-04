# conc-pass16: api server start and stop lifecycle, at 1f4de71

Head: `/home/claude/wt/zero3`, detached at origin/final/all-fixes-zero 1f4de71. Audit only. I read the source and ran no cargo, because neither new finding is high.

**Counts:** 2 new findings (0 high, 0 medium, 2 low): conc16-1 and conc16-2. I re-verified 10 known lifecycle rows: 1 FIXED, 1 PARTIAL, 8 NOT FIXED.

I checked for duplicates against concurrency.md (server1-*, server2-*, autopilot-4, lifecycle-1..3, poison-1, hunt-api-*), conc-pass2/lifecycle.md, conc-pass6, conc-pass12, conc-pass14, crash-edge-pass16 (CE-87) and tests-docs-security-pass4 (P1-12-02). conc12-1, CE-87 and conc14-2 are not re-reported.

## Lifecycle map (state at 1f4de71)

Startup, in the order `main.rs` and `server.rs` `run_in_over` (19046-19292) run it:
1. `main` builds a multi-thread runtime by hand (main.rs:44-58). `run_from` resolves the masters dir, and `run_in` resolves the store root (env only, nothing opened).
2. **Bind** (19054). If the port is in use, the arm at 19271-19285 emits `cannot bind the listening address` and returns FAILED. That event is lost (conc16-1).
3. **serve.lock** via `sole_server` (19094). If the store is missing, not a directory, or held by another instance, `sole_server` emits a refusal and returns FAILED. That event is lost too (conc16-1). The lock is a kernel flock, so a crash leaves no stale lock. The stale pid in the file is quoted only on `WouldBlock`.
4. Log dir (`served_log_dir`), then **`telemetry::install`** (19115). This is the first point from which any event can reach a file.
5. `web_dir` → `Site::serving` (masters parse, `census::read_all`). Nothing here writes, and credentials are read lazily from SSM at the first pull.
6. Banner and `api.serve listening`, then the browser launch (now reaped, hunt-api-6 FIXED).
7. `autopilot::fly` is spawned. It waits out a 20 s grace before any vendor work.
8. `recovery::resume`. On failure it is named and logged.
9. `serve` (graceful drain, no deadline).

Stop:
- SIGINT resolves `ctrl_c()` and the graceful drain starts. The drain is unbounded (server1-2).
- After the drain, `flying.abort()` runs (19267, autopilot-4) and `run_in_over` returns, which **drops `one_server` (serve.lock)**.
- Then `end_runtime` (main.rs:59) runs: `cli::cancel::request()`, a wait of at most 10 s while `ENGINE_TASKS > 0` (an "abandoned" Error event if it runs out), and `shutdown_timeout`. After that comes `note_exit(code)`.
- The press `conduct` (server.rs:11086) and recovery `drive` (recovery.rs:684/724) are not signalled. They keep running during the wait and are dropped at `shutdown_timeout` (lifecycle-1).
- SIGTERM and SIGHUP have no handler (lifecycle-2). There is no `SignalKind` anywhere in `crates/`.

| scenario | durable state | loud report (event + exit code) | next start |
|---|---|---|---|
| port in use | none touched | stderr only. **The event is dropped (conc16-1).** Exit 1, but the exit event is dropped too (main.rs:124-126 documents this) | clean |
| second server, same store | none touched | stderr only. **The event is dropped (conc16-1).** Exit 1. Its exit event is also dropped, although main.rs:120-123 says an exit-1 after a successful bind is written | clean |
| stale serve.lock after a crash | flock released by the kernel | n/a | clean, and the first line of the stamp is rewritten |
| SIGINT, active hand pull / masters refresh / archive ingest | the request completes first | drain is unbounded, and a second Ctrl-C is swallowed | server1-2, conc14-2 (known) |
| SIGINT, active press / recovery | legs dropped at `shutdown_timeout`; landed bars with no journal record; recovery InFlight left charged | none | lifecycle-1, recovery-2 (known) |
| SIGINT, autopilot tick | aborted mid-tick, no journal record | none | autopilot-4 (known) |
| SIGINT, engine sweep | cancelled at a boundary with a `Cancelled` audit, or abandoned after 10 s with the audit non-terminal | abandon event at Error, **but exit 0 "exited cleanly" (conc16-2)** | an abandoned audit reads "unconfirmed" |
| SIGTERM / SIGHUP | as SIGKILL | no events, no exit record | lifecycle-2 (known) |
| panic (release, abort) | process dies at the instruction, and every in-flight writer with it | no event | crash paths. poison-1 still open (pullrun.rs:679, sweeprun.rs:1482) |
| cli running while the server starts | startup only reads (masters, census). The execution lease is a flock, so whichever of server sweep and cli sweep comes second is refused | | clean apart from shared logs (lifecycle-3 / xcut-1, known) |

## New findings

### conc16-1 (low): every refusal before `telemetry::install` (port in use, a second server on the same store, a missing store root, an unstamped serve lock) is emitted to a sink that does not exist yet. The tests that "prove it is in the file" pass only because the test harness installs a sink first

- **Where:**
  - `crates/api/src/server.rs:19094` (`sole_server` is called before install at `:19115`);
  - `:18068-18075` (the `refused: this store cannot be served` emit);
  - `:18381-18388` (`note_unstamped_lock`);
  - `:19271-19285` (the bind arm's `cannot bind the listening address`);
  - `crates/telemetry/src/lib.rs:284-286`:
    ```rust
    pub fn emit(event: &Event<'_>) -> Emitted {
        global().map_or(Emitted::NotInstalled, |sink| sink.emit(event))
    }
    ```
  - Tests: `server.rs:21470-21500` (`a_refused_bind_is_logged_and_not_only_printed`, whose comment says *"The shared sink is installed first, so `run_in`'s own install is refused and its emit lands where `emitted::landed` reads"*), `:21737-21752` and `:25900-25937` (*"the refusal is in the file as well as on the terminal"*), and `:26052-26070`.
- **Why it is wrong:**
  - In the shipped binary, nothing installs a sink before `:19115`. So each of these `emit` calls returns `NotInstalled`, and the result is discarded (`let _noted`).
  - `sole_server`'s own doc says *"a refusal that is only printed is a refusal that exists for whoever was watching the terminal"*. The bind arm says *"It belongs in the file, not only on the terminal"*. In production, both exist only on stderr.
  - `main.rs:120-123` promises that an exit `1` *after a successful bind* is written. A second server on the same store binds successfully, is refused at serve.lock, and exits 1. Neither the refusal nor its `api.main` exit event reaches the log.
  - The P1-12-02 "fix" (read the event back) turned these tests green against a harness-only state: `emitted::sink()` pre-installs the shared sink, so the tests prove a path production never takes. Gate 23 / `emitted.rs` counts these sites as "reached".
  - The two scenarios this theme names (port in use, and a second server on one store) are therefore the ones that leave no durable trace.
- **Repro (not run):** `BRUTEX_LOGS=/tmp/l api serve 127.0.0.1:8080` in one shell. In a second shell, `BRUTEX_LOGS=/tmp/l2 api serve 127.0.0.1:8081` against the same `BRUTEX_STORE`. The second exits 1 with "REFUSED: another brutex api is already serving this store" on stderr, and `/tmp/l2` is never created (no `events.ndjson`, no `api.serve refused` line, no `api.main exited non-zero`). The same holds for `api serve 127.0.0.1:8080` while 8080 is taken.
- **Minimal fix:** when the log dir does not depend on the store (`BRUTEX_LOGS` set, or a workspace cwd), install the sink before the bind. Otherwise say plainly in `sole_server`, the bind arm and `main.rs:120-126` that these refusals are stderr-only. Writing into the holder's store log would be a second unlocked writer (lifecycle-3). Also make the tests assert the production outcome: run `run_in_over` in a child process with no pre-installed sink and assert `Emitted::NotInstalled`, or assert the file, rather than the harness sink.

### conc16-2 (low): a stop that abandons running engine work still exits 0 and logs "exited cleanly — everything went as asked", right after the Error event saying their results are not recorded

- **Where:** `crates/api/src/main.rs:59-61`:
  ```rust
  let _abandoned = api::server::end_runtime(runtime, api::server::SHUTDOWN_GRACE);
  note_exit(code, count);
  std::process::ExitCode::from(code)
  ```
  `code` was decided by `stopped_over` (server.rs:18459) before the wait. `wait_then_end` (server.rs:18553-18580) returns the abandoned count, and its warning says *"Their results are not recorded; their invocation audits stay non-terminal. Re-run them."*
- **Why it is wrong:**
  - D-1582's point is that abandoned work is "said, never silent". The count is computed and then discarded, so the exit code and the `api.main` record contradict the event written one line earlier.
  - A supervisor or monitor reading the exit status (the thing D-0026 made non-zero for a DEGRADED session, which `stopped_over`'s doc calls *"the one fact every operator and every monitor acts on"*) sees 0 for a stop that killed a sweep mid-run. The log's final word on the process is Info "exited cleanly".
  - The same applies to the `DEGRADED`-vs-`OK` decision: an abandoned sweep is "did the work and the answer must not be trusted" at least.
- **Repro (not run):** `api serve`, POST a long `/backtest/command` (any engine command that runs more than 10 s between two `cli::cancel::check` boundaries), then Ctrl-C. stderr shows "STOPPING WITH 1 ENGINE TASK(S) STILL RUNNING", the log has `api.main engine tasks abandoned at shutdown` (Error) followed by `api.main exited cleanly code=0`, and `echo $?` prints 0.
- **Minimal fix:** `let code = if abandoned > 0 && code == OK { FAILED } else { code };` before `note_exit`. Use `FAILED` because the process could not finish work it had accepted (or `DEGRADED`, if the ledger prefers that word). Add a test over `wait_then_end` plus the mapping.

## Verification of known lifecycle rows (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| hunt-api-2 | PARTIAL | D-1582/D-1551: `main.rs:44-59` builds the runtime and calls `end_runtime`. `cli::cancel::request()` plus a 10 s bounded wait, with an abandoned-count event (server.rs:18541-18580). But `one_server` is a local of the serve arm (server.rs:19094) and is dropped when `run_in_over` returns, *before* `end_runtime` runs. serve.lock is therefore still free during the whole grace, while cancelled sweeps, the press and the recovery drive keep running on the live runtime. Fix: return the `ServeLock` out of `run`, or call the wait inside the arm, and drop it after `end_runtime`. |
| autopilot-4 | NOT FIXED | server.rs:19267 `flying.abort();`, with no `pause()` and no bounded await first |
| server1-2 | NOT FIXED | server.rs:17552-17558 `with_graceful_shutdown(async move { let _ = shutdown.await; })`: no drain deadline, no epoch bump, and a second Ctrl-C is swallowed |
| lifecycle-1 | NOT FIXED | server.rs:11086 `let _flying = tokio::spawn(conduct ..)` and recovery.rs:684/724 `let _task = tokio::spawn(drive ..)`: handles are dropped and nothing signals them at shutdown |
| lifecycle-2 | NOT FIXED | main.rs:52 `Box::pin(tokio::signal::ctrl_c())`. Grep finds no `SignalKind`/`SIGTERM` handler in `crates/` |
| lifecycle-3 | NOT FIXED | server.rs:18962-18975 `log_dir_from`: a workspace cwd still wins over the store-scoped dir |
| server2-2 | NOT FIXED (in-process only) | server.rs:18263-18276 still returns `ServeLock { held: None }`, whose `Drop` (18148-18154) removes the live holder's key |
| poison-1 | NOT FIXED | pullrun.rs:679 still says "`Drop` runs on the panic path". sweeprun.rs:1482 and cli operation_audit.rs:448 keep `thread::panicking()` arms that are dead under `panic = "abort"` |
| recovery-2 | NOT FIXED | recovery.rs:777 `Journal::open(&active_path(site))` still creates `active.bin` before the pointer append |
| hunt-api-6 | FIXED | server.rs:18875-18896: `reap_detached` waits for the launcher on its own thread (D-1590) |

Tally: FIXED 1, PARTIAL 1, NOT FIXED 8.

## Checked and not filed

- **Stale serve.lock.** It is a kernel flock, so it is released on crash or SIGKILL. The stamp is rewritten after the lock is held and cut to its own length, and the refusal quotes line 1 only on `WouldBlock`. Nothing wedges.
- **Startup order.** Bind, then store root, then serve.lock, then everything else. Nothing is created or written before serve.lock, and a missing root is refused, not created. `Site::serving` and `census::read_all` only read. `fly`'s 20 s grace keeps `recovery::resume`'s claim ahead of the first tick.
- **cli running during server start.** Startup writes nothing the cli writes. Sweeps contend only on the execution-lease flock (the second is refused). The cli's `cancel` flag is per-process, so a stopping server never cancels a cli run.
- **`ctrl_c()` Err.** `serve_limited` treats a failed registration as "stop" and exits 0 with no event. I did not file it: I found no reachable failure on a runtime built with `enable_all`.
- **Two binds on one port with different specificity** (the `take_serve_lock` doc says macOS lets `127.0.0.1:8080` and `0.0.0.0:8080` coexist). I did not file it: serve.lock still refuses a same-store second server, and the different-store case could not be measured on this Linux host.
- **Engine task count.** `EngineTaskCount` is a field of `TaskFinisher`, so it decrements after the finisher's audit write (field drop follows `Drop::drop`). The wait therefore cannot end between the cancel and its `Cancelled` audit.

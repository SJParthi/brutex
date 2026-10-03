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

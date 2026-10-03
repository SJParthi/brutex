Verdict: 3 new findings at 331b05c (0 high, 2 medium, 1 low). Two are crash-window defects in the sweep chain, and the third is a lock-free reader. 13 pass-1 findings touching the slice were re-verified: all CONFIRMED, none refuted.

# conc-pass2 / sweep: one sweep end to end, browser POST to backtest JSON readers

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

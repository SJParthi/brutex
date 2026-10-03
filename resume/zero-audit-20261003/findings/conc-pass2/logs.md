# conc-pass2 / logs: readers of the live logs, the invocation journal and the pull audit journal (commit 331b05c)

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

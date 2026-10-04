# Concurrency and state audit, pass 19: are the audit journals and trails usable as evidence?

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 3 new findings, conc19-1 to conc19-3, all low. No cargo was run, and every repro is "not run". Each claim rests on quoted source and on grep for absent writers or readers.

**Not re-reported (known):**
- conc15-4, CE-78 and conc17-2
- conc13-1 to conc13-8
- concurrency.md cli1-1 (a 0-byte per-ID file blocks paging), cli1-2, cli1-5, xcut-1, telemetry-1, lifecycle-1, press-2, recauto-2 and poison-1 (dead `panicking()` arms under `panic = "abort"`)
- conc-pass5's "an index row whose sync failed burns that id"

## Journal-by-journal answers to (1)-(6)

| Journal | (1) loss / duplicate / reorder | (2) tamper evidence | (3) full readback | (4) who / what / outcome / why | (5) write failure | (6) growth |
|---|---|---|---|---|---|---|
| **cli/api invocation journal** `audit/invocations-v1` (cli operation_audit.rs) | The start is synced before dispatch (`begin` :510-576). The terminal is written after the work. A crash leaves "unconfirmed", which is documented. A 0-byte file is cli1-1. A duplicate terminal is refused (`State::update` :360-368). The id comes from a flocked index length, so ids cannot be reordered. | Per-record CRC32C only, with no chain. The exact read checks the index start plus the first and last records (:584-653, documented in `integrity_scope`). Truncating a per-ID file to a stride boundary reads as "unconfirmed", not as an error. Nothing claims tamper evidence. | Yes. `/backtest/audit.json` pages 32 newest-first, and `InvocationAudit.svelte` renders it. One bad ID blocks a page window (cli1-1). | Origin, label "METHOD /route", phase, HTTP status and time. **No refusal reason and no arguments, by design** (module doc :8-9; 05-decisions.md:34529-34531). The reason exists only in telemetry (conc13-1). | Start: **fail-closed**, so the route answers 503/429 and the cli sweep refuses (lib.rs:2110-2122). Terminal: the result is replaced by "terminal audit unconfirmed" (lib.rs:2132-2138, api operation_audit.rs:229-247). Both are documented (D-1445). | Unbounded: one file per audited request (06-limits:11750, conc17-2) |
| **pull journal** `audit/pull.journal`, used by press, F&O walk, autopilot tick and recovery (api audit.rs) | Written **after** the run, with no start record. A kill mid-run loses the record (lifecycle-1). The seat, unknown-vendor and clock refusals are never written: **conc19-1**. Non-blocking flock collisions are concurrency.md:519 and recauto-2. | CRC32C per record. A torn tail is reported (`look` :1088). A deleted tail at a stride boundary is undetected. | Yes. `/audit.json?feed=&page=` pages up to 200 per page through the whole file. | Scope, source text, window, counts and the first reason (68 B note). Per-member reasons are **written only by the hand spot path**: **conc19-2**. | Warn only. The receipt says "NO — not in the journal" and an Error event is emitted (`note_append_failed` :1340). The autopilot exposes `journal_error` (conc15-4, CE-78). Documented. | 256 B per record, never rotated (module doc :49-55) |
| **autopilot journal**: the same file, one record per tick (autopilot.rs:3532-3567) | As for the pull journal. A refused append is overwritten by the next tick (conc15-4). | As for the pull journal. | `/audit` and `/audit.json` | A tick record carries at most one reason, and member failures 2..N are absent: **conc19-2**. Halt, stall and backoff decisions are absent (conc13-4). | Warn only (`journal_error`) | One record per tick that ran a unit |
| **recovery journal** `audit/recovery-v1/{plan}.bin`, `attempts.bin`, `active.bin` (recovery_journal.rs) | InFlight is synced **before** the vendor request. Uncertain I/O poisons the handle. Reopen syncs the surviving bytes. The inode is checked around each append. This is sound. | CRC32C per record, with no chain. The plan seal (`validate_seal` recovery.rs:289) binds the scan inventory to the plan id, but not the order or count of transitions. Rewinding or deleting a tail record at a stride boundary is undetected, as D-0569 states. | **No.** Only the newest 100 events of the active plan are readable. The plan-level outcome and its reason are not persisted: **conc19-3**. | Exact request body, attempts, counts and HTTP status. **No reason field.** The window's reason goes to the pull journal's 68-byte note and to telemetry. The plan's reason goes only to an Info event (conc13-1). | **Fail-closed**: every append is `?` and blocks the recovery. Documented in the module doc. | One file per plan, plus attempts and active, never removed |
| **api request log** (`/logs`, the telemetry sink) | Not synced, by design (sink.rs:51-63). A drop is counted in `Health`. Cross-process forks are xcut-1 and telemetry-1. | None (NDJSON) | `/logs` tail, bounded at 64 MiB | 4xx/5xx path and status only (conc13-3) | Never stops the caller. Documented (sink.rs:66-90). | Rotated, 8 MiB × 8 |
| **checksum receipts and evidence envelopes** (cli checksum_receipts.rs, `BRBLCM01`) | `create_new` plus sync plus directory sync (:306-357) | A BLAKE3 seal over each receipt. These are per-artifact, not a sequential trail. | Read by their own verbs | The source identity and evidence | Fail-closed | One per audited month |

**Hash-chained ledger:** none exists. `grep prev_digest|chain_digest|hash chain` finds only a local `prior_digest` comparison at cli stored.rs:5564. Every trail in the repository is per-record CRC32C, and the evidence artifacts carry per-artifact BLAKE3 seals. None of them claims to be tamper-evident (`grep -i tamper docs crates`), so the missing chain is not a contradiction and is not filed.

## New findings

### conc19-1 (low): a `/pull/spot` or `/pull/fno` refused for a busy seat (409), an unknown feed (400) or an unreadable clock (500) writes no audit-journal record, yet `/pull/run` tells the operator "The reason is in the audit journal"

- **Sites:**
  - crates/api/src/server.rs:11297-11320 (spot) and :15123-15145 (F&O):
    ```rust
    let Some(_seat) = site.autopilot.take_seat(wants) else {
        return (axum::http::StatusCode::CONFLICT, receipt(), axum::response::Html(accepted_html("Spot pull", ...
    ```
    The unknown-feed arms at :11276-11295 and :15104-15121, and the `ist_day` `Err` arm at :11329-11332, also return without touching `site.journal()`.
  - crates/api/src/pullrun.rs:816-821:
    ```rust
    _ => { "its receipt did not read clean. The reason is in the audit journal; \
            this leg remains owed and is eligible for another pass." }
    ```
    `leg_outcome` (:746) maps 409 to `Retry` ("409 stays retryable for a busy feed seat"). `request_leg` (:786) calls `pull_spot` directly, not through the router, so `logs.rs`'s 4xx line does not fire either.
- **Why it is wrong:** `spot_answer`'s own rule is "A REFUSAL IS RECORDED TOO. 'What was asked' includes the requests that were not honoured" (server.rs:6227). The arms before `spot_answer` break that rule. A press leg that loses its seat to an autopilot tick already holding `AllSeats`, or to a recovery holding Zerodha's seat, is retried, and `/pull/run.json` `last_error` sends the operator to `/audit`. `/audit` has no row, `/logs` has no line, and the page offers nothing else. §4: the claim names a record that does not exist.
- **Repro (not run):**
  1. Let the autopilot start a tick, which takes `take_every_seat`.
  2. While it holds the seats, POST `/pull/run` with one Dhan spot leg. The leg returns 409, and `last_error` reads "... answered HTTP 409: its receipt did not read clean. The reason is in the audit journal".
  3. `/audit.json?feed=dhan` shows no record for the leg's time.
- **Minimal fix:** append `audit::Record::refused(Scope::Spot|Fno, Outcome::NotStarted, now, target, why)` on the seat, unknown-feed and clock arms, and render its `recorded_fact` as `spot_answer` does. Alternatively, make `note_leg_failure` for 409 say that the seat was busy and that no journal record exists.

### conc19-2 (low): only the hand spot path writes per-member failure records. The autopilot tick and the expired F&O walk record a count and at most one reason, so the failures D-0073 says are "on disk" are not

- **Sites:**
  - crates/api/src/autopilot.rs:3532-3567: one `Record::refused` or `Record::of_run` per tick, then `site.journal().append(&record)`. There is no `member_failure` loop: `grep member_failure crates/api/src/autopilot.rs` returns nothing.
  - crates/api/src/server.rs:11459-11471 (`FnoPage::say_counted`): `Record::refused(...).with_counts(contracts, rows_read, bars_stored, failures)`. The per-contract reasons go to the page only ("First reasons", :14782 and :14996).
  - The only `Record::member_failure` loops are `recorded_with_failures` (server.rs:10803, a single caller at :10655 on the spot receipt path) and `recovery_spot` (:8149).
- **Why it is wrong:** audit.rs:191-209 (`Kind`) and server.rs:10775-10790 say the failure records exist because "a run that refused ten members therefore left nine reasons nowhere on disk", and conclude that "every failure is on disk". The autopilot comment says its record goes "through the same constructors the HTTP receipt uses — so `/audit` cannot tell an autopilot run from a hand-made one except by its source" (autopilot.rs:3532-3534). The autopilot is the unattended backfill, so it is where unread failures pile up. Its per-member reasons live only in the in-memory `/autopilot.json` list (capped at 40, gone on restart) and in rotating telemetry. The journal shows `failures: N` and one 68-byte reason.
- **Repro (not run):** run an autopilot tick in which 3 of N instruments fail (for example a vendor 4xx per instrument). `/audit.json` shows one `run` row with `failures: 3`, one note, and no `member` rows. A hand `/pull/spot` over the same month writes 1 + 3 rows.
- **Minimal fix:** after the autopilot's run record, append `Record::member_failure(Scope::Spot, now, &f.instrument, unit.window, &f.why)` for each `run.total.failures`, reusing `recorded_with_failures`' loop and keeping the first refusal in `journal_error`. Collect the F&O walk's per-contract `(instrument, why)` and do the same.

### conc19-3 (low): the recovery journal cannot be read in full. The plan-level outcome and its reason are never persisted, and after a restart nothing says how a plan ended

- **Sites:**
  - crates/api/src/recovery.rs:1653: `crate::recovery_journal::tail(&plan_path(site, id), 100)` is the only reader behind `/pull/recovery` and `/pull/recovery.json`. There is no paging parameter. `tail` is capped at `MAX_TAIL_RECORDS = 256` (recovery_journal.rs:84). `attempts.bin`, `active.bin` history and superseded plans have no route.
  - recovery.rs:1779: `.filter(|row| !server::param(&row.body, "member").is_empty())` hides the CONTROL (plan-seal) row on the HTML page.
  - recovery.rs:814-843 (`drive`): the plan's `Verified`/`Blocked` is appended with no reason (the record has no reason field). The summary ("Reconciliation finished: V verified ... B missing/blocked", or "Recovery BLOCKED: why") goes only to the in-memory `progress.finished` and to an `Info` telemetry event.
  - recovery.rs:1760: `"No recovery is running in this process. Recorded progress below remains inspectable."`; recovery.rs:985: `"Details: /pull/recovery.json and /audit."`
- **Why it is wrong:**
  - A plan holds up to `MAX_UNITS = 100_000` windows. The seed alone writes one record per window, and each window then appends `InFlight` and terminal records, so the newest 100 events cover only the last few dozen windows.
  - After a restart, or once the in-memory summary is gone, no route can say how many windows ended Verified, Exhausted or Blocked, or why the plan was BLOCKED.
  - Neither the page nor the JSON lets the operator see the earlier windows the summary told them to inspect.
  - The journal has every fact on disk (`snapshot` replays them), but no reader exposes them. That fails (3) and, for the plan's reason, (4).
- **Repro (not run):**
  1. Start a recovery plan of 200 scan windows and let it finish.
  2. Restart `api serve`.
  3. `/pull/recovery` shows "No recovery is running ..." and 100 event rows, all from the last windows. The plan's Blocked or Verified row is hidden, and the first ~150 windows' outcomes cannot be found through any route. `/pull/recovery.json` returns the same 100 rows.
- **Minimal fix:**
  - Add a `before=<ordinal>` cursor to `/pull/recovery.json`, through a bounded `read_range` beside `tail`.
  - Add a summary computed from `snapshot(plan).latest`, with counts per status, served when no writer holds the lock.
  - Persist the BLOCKED reason, for example as a final `Unverified` evidence record whose body carries the reason, as `load_lifecycle` already does for lifecycle evidence.

## Verification of known items touched by this theme

| ID | Status | Evidence |
|---|---|---|
| conc15-4 | NOT FIXED | autopilot.rs:3373-3376 still assigns `status.journal_error = out.journal_error.clone().unwrap_or_default()` every tick |
| CE-78 | NOT FIXED | `grep -rn journal_error web/src` finds no match. The field is emitted at autopilot.rs:1759-1765 and never read. |
| conc17-2 | NOT FIXED | web/src/routes/backtest/+page.svelte:2303 `createPageRequests()`; `statusRequests.schedule(pollSweep, 2000)` at :2395-2449 has no visibility gate |
| conc13-1 | NOT FIXED | cli lib.rs:2161 `Event::info("cli.lifecycle", "command finished")` with no `why`; recovery.rs:840 `Event::info("pull.recovery", "recovery ended")` |
| conc13-4 | NOT FIXED | autopilot halt, stall and backoff decisions are still not appended to the journal (autopilot.rs:3532-3567 is the only append) |
| cli1-1 (concurrency.md) | NOT FIXED | cli operation_audit.rs:618 still errors on `bytes == 0`; `page` (:660-665) still collects with `?` |
| poison-1 | NOT FIXED | cli operation_audit.rs:448 and api sweeprun.rs:1482 still use `thread::panicking()` arms, which are dead under `panic = "abort"` (Cargo.toml:188) |

Tally: 7 rows, NOT FIXED 7.

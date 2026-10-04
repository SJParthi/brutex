# Concurrency and state audit, pass 13: does logging and monitoring cover every failure?

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 8 new findings, conc13-1 to conc13-8: 2 medium, 6 low. Another pass is needed.

The pull data path is well covered. Every member, chunk, census, credential, folder and archive refusal has an `Error` or `Warn` event that names the instrument or file and the reason. Per-run failure totals are in `pull.run finished` and in the journal.

The gaps are in four places:
- **Decisions:** autopilot and pull-run halt, stall and backoff decisions are not logged.
- **Severity:** cli and recovery terminal refusals are logged at `Info`, and halted or partly refused cli sweeps exit 0 and are recorded as "completed".
- **Attribution:** the store's only-trace events name a 32-bit symbol hash, not a file.
- **Counting:** retries that recover are not counted, and no surface counts failures across a window.

**Method.** I read the source. No cargo was run, and every repro below is "not run". Every claim rests on a quoted line and on grep for absent emits. Known ids are not re-reported: conc9-*, conc11-2, conc11-3, P8-01 and CE-45.

## Coverage table (64 sampled error, refusal, retry and halt paths)

Columns:
- (1) context: a durable record names it with enough context to diagnose it.
- (2) surface: it reaches a surface the operator sees.
- (3) severity: its level is right.
- (4) counted: a repeat is visible as a rate.

Marks: Y = yes, P = partial, N = no.

| # | site (file:line) | path | (1) context | (2) surface | (3) severity | (4) counted | note |
|---|---|---|---|---|---|---|---|
| 1 | pull/ingest.rs:655 | member did not land | Y instrument, why | /logs | Error Y | P per-run `failed` | |
| 2 | pull/ingest.rs:687 | bars not filed | Y | /logs | Error Y | P | |
| 3 | pull/ingest.rs:708 | derived rung did not land | Y | /logs | Error Y | P | |
| 4 | pull/ingest.rs:760 | rung not derived | Y | /logs | Warn Y | N | |
| 5 | pull/ingest.rs:971 | pull run refused | Y about-path | /logs | Error Y | N | |
| 6 | pull/ingest.rs:994 | census loaded degraded | Y | /logs | Warn Y | N | |
| 7 | pull/ingest.rs:1009 | census not published | Y | /logs | Error Y | N | |
| 8 | pull/ingest.rs:1022 | bars not counted | Y | /logs | Error Y | N | |
| 9 | pull/ingest.rs:1115 | request minutes incomplete | Y | /logs | Warn Y | N | |
| 10 | pull/ingest.rs:1128 | duplicate | Y | /logs | Warn Y | N | |
| 11 | pull/manifest.rs:3207 | census load (level varies) | Y vendor | /logs | Y | N | |
| 12 | pull/secret.rs:552 | credential refused | Y vendor | /logs | Error Y | N | |
| 13 | pull/config.rs:275 | config refused | Y file | /logs + halt | Y | N | |
| 14 | pull/totp.rs:209 | TOTP secret will not decode | P (no vendor) | /logs | Error Y | N | |
| 15 | pull/folder.rs:484 | folder refused | Y dir | /logs | Error Y | N | |
| 16 | pull/archive.rs:404 | archive folder refused | Y dir | /logs | Error Y | N | |
| 17 | pull/csv.rs:606 | CSV file refused | P (no file; the caller's #16 names the dir) | /logs | Warn (documented) | N | |
| 18 | pull/resolve.rs:462/475 | index has no constituents / refused | Y at, why | /logs | Warn/Error Y | N | |
| 19 | pull/capture.rs:164 | vendor capture not written | Y feed | /logs | Error Y | N | |
| 20 | pull/work.rs:211 | selection narrowed | Y | /logs | Warn Y | N | |
| 21 | pull/rate.rs:1071 | every span backed off | Y | /logs | Warn Y | N | |
| 22 | pull/http.rs:884-925 | vendor answered non-2xx (each retry) | Y url, status, instrument, window, sent | /logs | Warn Y | P one line per answer | |
| 23 | pull/http.rs:3052-3058 | transport failure (no answer) | **N** | none until exhausted | – | **N** | **conc13-2** |
| 24 | pull/http.rs:1193..1802, 3534 | decode warnings | Y | /logs | Warn Y | N | |
| 25 | api/server.rs:9832-9880 | `with_retry` `Step::Again` (sleep, re-ask) | **N** | none | – | **N** | **conc13-2** |
| 26 | api/server.rs:9084 | chunk refused after retries | Y instrument, feed, chunk, window, vendor_said | /logs | Error Y | P | |
| 27 | api/server.rs:6866 | broker request refused | P instrument, why (no feed or month) | /logs + receipt | Error Y | P | |
| 28 | api/server.rs:7187 | member did not land (spot) | Y instrument, month, feed, rung | /logs + /autopilot.json | Error Y | P (list capped at 40) | conc13-8 |
| 29 | api/server.rs:7695 | cash schedule refused | P (why only; #28 carries the context) | /logs | Error Y | N | |
| 30 | api/server.rs:7727 | recovery not resumed | Y | /logs | Error Y | N | |
| 31 | api/server.rs:7060 | pull run finished with failures | P counts, no feed or window (see `started`) | /logs | Warn Y | **Y `failed`** | |
| 32 | api/server.rs:10174 | instrument refused | Y | /logs | Error Y | N | |
| 33 | api/server.rs:12512 | F&O discovery refused | Y | /logs | Error Y | N | |
| 34 | api/server.rs:8699/8721 | rate governor refuses a permit | P via the caller's member failure | /logs | Y | N | |
| 35 | api/logs.rs:1096-1156 | any 4xx/5xx response | **P** path + status only, no reason, no query | /logs | Warn/Error Y | Y rationed with a summary | **conc13-3** |
| 36 | api/topjson.rs:144 (and 19 other `*json.rs` readers) | 503 with reason | **N** reason only in the body | the response | – | N | **conc13-3** |
| 37 | api/sweeprun.rs:2045-2056 | sweep body refused | Y why | /logs | Warn Y | N | |
| 38 | api/sweeprun.rs:2058-2065 | no commit stamp | Y | /logs | Warn Y | N | |
| 39 | api/sweeprun.rs:2086 | environment budget refusal | **N** | the response only | – | N | **conc13-3** |
| 40 | api/sweeprun.rs:2107 | `claim_execution` refusal (lease or CLI sweep running) | **N** | the response only | – | N | **conc13-3** |
| 41 | api/sweeprun.rs:2108 | `reserve_invocation` refusal | **N** | the response only | – | N | **conc13-3** |
| 42 | api/sweeprun.rs:2075-2083 | busy | Y | /logs | Warn Y | N | |
| 43 | api/sweeprun.rs:2227-2256 | descent refusals (stamp, budget, lease, audit) | **N** except the first arm | the response only | – | N | **conc13-3** |
| 44 | api/sweeprun.rs:1605-1622 | engine task finished (outcome) | Y | /logs + /backtest/run.json | Y via `completion_audit` | N | |
| 45 | api/bars.rs:174 | read refused | Y vendor, exch, seg, symbol, tf, month | /logs | Warn Y | N | |
| 46 | api/ingest.rs:1434 | form refused | Y form, field | /logs | Warn Y | N | |
| 47 | api/autopilot.rs:3402-3410 | Backoff (retry the month in N s) | **N** | /autopilot.json only, overwritten | – | P attempts | **conc13-4** |
| 48 | api/autopilot.rs:3412-3423 | Stall (month passed over) | **N** | /autopilot.json only | – | P `stalls` list | **conc13-4** |
| 49 | api/autopilot.rs:3425-3431 | Halt (feed terminal) | **N** | /autopilot.json only | – | N | **conc13-4** |
| 50 | api/autopilot.rs:2589, 2614-2640, 2666 | pre-loop halts (clock, broker, no rung directory) | **N** | /autopilot.json only | – | N | **conc13-4** |
| 51 | api/autopilot.rs:2078 | paused | Y | /logs | Warn Y | N | |
| 52 | api/pullrun.rs:797-833 | leg failure, feed halted for this run | **N** | /pull/run.json only | – | P `retries` | **conc13-4** |
| 53 | api/recovery.rs:839-843 | "Recovery BLOCKED: ..." | Y plan, summary | /logs | **Info N** | N | **conc13-1** |
| 54 | api/recovery.rs:991 | window unresolved | Y | /logs | Warn Y | N | |
| 55 | api/main.rs:171-206 | api exit non-zero | Y | /logs | Error Y | – | the house pattern conc13-1 breaks |
| 56 | store/header.rs:706 | no committed header | **N** no file, vendor or month | /logs + caller Err | Error Y | N | **conc13-5** |
| 57 | store/header.rs:738 | fell back to an older generation (only trace) | **N** symbol hash only | /logs only | Warn Y | N | **conc13-5** |
| 58 | store/header.rs:773 | commit refused | **N** symbol hash only | /logs + Err | Error Y | N | **conc13-5** |
| 59 | store/block.rs:109/148/213 | no checksums, mismatch, interrupted append (only trace) | **N** symbol hash and block only | /logs (+ Err for mismatch) | Y | N | **conc13-5** |
| 60 | store/file.rs:3364 | bytes past the commit | Y file | /logs | Warn Y | N | |
| 61 | store/flock.rs:261 | lock not released | Y | /logs | Warn Y | N | |
| 62 | cli/lib.rs:2145-2167 | any cli command refused | **P** no reason | /logs + exit code | **Info N** | N | **conc13-1** |
| 63 | cli/lib.rs:3621-3631 | sweep-stored `?` refusals (evidence begin, integrity, column) | P lifecycle only | stdout + exit 2 | Info N | N | conc13-1 |
| 64 | cli/lib.rs:3710-3731, 3741-3790 | ladder halted on a budget | Y `halted=true`, ledger and evidence flag | stdout says REFUSED, **exit 0** | **Info N** | N | **conc13-6** |
| 65 | cli/lib.rs:2197 | `sweep` halted on a budget | – | stdout, **exit 0** | – | N | **conc13-6** |
| 66 | cli/batch.rs:521-630, 838-846 | sweep-all months refused | **N** no event | stdout only, **exit 0** unless every month refused | – | **N** in the log | **conc13-7** |
| 67 | cli/batch.rs:788 | month swept, `completed=false` | Y | /logs | Info (partial) | N | conc13-7 |
| 68 | cli/lib.rs:3165-3196 | stdout not writable | Y | /logs + stderr | Warn Y | – | |
| 69 | cli/main.rs:50-54 | store root preflight refused | – (no sink yet; correct) | stdout + exit 1 | Y | – | |
| 70 | cli/lib.rs:20627 | result set refused | Y identity, why | /logs | check level | N | |
| 71 | cli/checksum_receipts.rs:205 | checksum attempt refused | Y | /logs | Y | N | |
| 72 | runner grid.rs:951/2462, audit.rs:648 | unpriceable paths (gate 17 inner loop) | Y counted in `Grid`, rendered on the audit page | stdout | Y | Y per grid | gate 17 holds; see note |
| 73 | engine Halt, report.rs:876-905 | budget, memory or worker breach (inner loop) | Y as a value | stdout verdict | Y in the page; exit is conc13-6 | – | |

**Gate 17.** vocab, engine, indicators and runner still have no telemetry dependency. Inner-loop failures do not vanish:
- The engine returns `Halt { breach, k, candidates, pairs }`.
- The runner counts `refused_paths` and `refused_levels` in `Grid`, and `audit::grid` prints a non-zero `refused_paths`.
- The cli boundary events carry the totals (`priced`, `candidates`, `trades`, `halted`).

The defect at that boundary is what happens to the counted value afterwards: the exit code and the event level (conc13-6). One remaining hole is the screen in `cli/lib.rs:12393-12417`. It drops a candidate whose two grids yield no cell (`best?`) without a reason. The only trace is `priced < candidates` in `exit grid finished`. That is visible, but the reason is not recorded. Grid `refused_paths` is not summed across the screen (`grep refused_paths crates/cli/src/lib.rs` returns nothing). I list this as a note, not a finding.

## New findings

### conc13-1 (low): terminal refusals are logged at `Info`, and the cli lifecycle marker carries no reason
- **Sites:**
  - crates/cli/src/lib.rs:2158-2166:
    ```rust
    &telemetry::Event::info("cli.lifecycle", "command finished")
        .with("command", command)
        .with("sweep_command", is_sweep_command(command))
        .with("phase", if code == OK { "completed" } else { "refused" })
        .with("exit_code", u64::from(code)),
    ```
  - crates/api/src/recovery.rs:830 and :839-843:
    ```rust
    let text = result.unwrap_or_else(|why| format!("Recovery BLOCKED: {why}. ..."));
    ...
    &telemetry::Event::info("pull.recovery", "recovery ended")
    ```
- **Why it is wrong:**
  - The api binary maps every non-zero exit to `Error` (api/main.rs:171-206), on the stated ground that "a level below `error` would put the same defect back one layer down".
  - The cli writes its refusal at `Info`, and so does the recovery when it blocks.
  - `/logs?level=warn` uses `Query::at_least`, so it hides both.
  - The cli marker has no `why` field. The refusal text exists only on the cli's stdout.
  - `/backtest/run.json` then tells the operator "the external command reported a refusal; inspect its lifecycle and result logs" (sweeprun.rs:2712-2714). Those logs do not contain the reason.
  - So a browser-observed refused cli sweep leaves no diagnosable durable record. The exception is the few verbs that emit their own `result set refused`.
- **Repro (not run):**
  1. Run `cli sweep-stored dhan NIFTY 1min 2026 13 500` (month 13), or any stored month that is not on disk.
  2. Then read `/logs?level=warn`: nothing. `/logs` shows `command finished phase=refused exit_code=2` at info with no reason.
- **Minimal fix:**
  - Use `Level::Warn` (or `Error`) when `code != OK`, and add `.with("why", refusal_reason(out).unwrap_or("..."))`, clipped to the field ceiling. `run_with_sink` has `out` in scope after `dispatch`.
  - Emit `recovery ended` at `Error` when `result.is_err()`.

### conc13-2 (low): a transport failure that is retried and then recovers leaves no line and no count
- **Sites:**
  - pull/http.rs:3052-3058: `send().await.map_err(|why| FetchError::TransportFailed { .. })?`. This path emits nothing, because `note_answer` runs only after a status is read.
  - api/server.rs:9832-9880: the `Step::Again` arm only sleeps:
    ```rust
    tokio::time::sleep(std::time::Duration::from_millis(wait_ms)).await;
    ```
- **Why it is wrong:**
  - A timeout, reset or DNS failure goes through up to `THROTTLE_ATTEMPTS` (6) attempts.
  - If any later attempt succeeds, nothing records that the earlier ones failed. There is no event, no counter on the run, no field in `pull.run finished`, and nothing in /pull/run.json or /autopilot.json.
  - The doc above `with_retry` calls these blips "certainties, not edge cases" over ~62,600 requests. A link that fails 30% of first attempts, each paying a 250 ms/1 s backoff and a permit, is fully invisible until it gets bad enough to exhaust six attempts.
  - Answered 429 and 5xx retries are visible (one `pull.http vendor answered` Warn per answer). Unanswered ones are not.
- **Repro (not run):** point a feed at a loopback socket that drops the first connection and answers the second. `/logs` then holds only the trace-level success.
- **Minimal fix:**
  - Count `Step::Again` occurrences in `BrokerRun` (for example `retried_transport`) and put the count on `pull.run finished`, which is already Warn-capable.
  - Emit one `Debug`/`Warn` `pull.http "transport failed, retrying"` with instrument, attempt and the error. It is per failure, not per request, so it is affordable.

### conc13-3 (low): many HTTP refusals reach /logs as a bare path and status, and three sweep-launch refusals as nothing
- **Sites:**
  - api/logs.rs:1148-1153, which records `method`, `path`, `status` and `micros` only.
  - api/topjson.rs:144: `Err(why) => refusal(axum::http::StatusCode::SERVICE_UNAVAILABLE, &why)`. The same shape is in 19 other `*json.rs` readers with zero emits: candidatejson, booleanjson, indexstop*json, livejson, frontierjson, trades, detail, sweepevidence, folder, and others.
  - api/sweeprun.rs:2086-2088 (`environment_budget_refusal`), :2107 (`claim_execution`) and :2108 (`reserve_invocation`), each `return refused(&why)` with no emit. The same applies to the descent at :2242 and :2256-2257.
- **Why it is wrong:**
  - The middleware is the only record for these handlers. It logs the path, deliberately not the query (which carries feed and underlying), and not the reason.
  - A ledger that will not decode, or a store that refuses, shows on /logs as `served /top.json 503`. That does not say which instrument or why.
  - In `run_with`, three of six refusal arms emit a Warn with `why` and three do not. The silent three include the lease refusal: a sweep refused because a cli sweep holds the store, or because the external log is unreadable (conc9-1b's case). So the operator's own Run press that was refused leaves no reason anywhere but the browser toast.
- **Repro (not run):**
  1. Hold the execution lease with a running `cli sweep-stored`.
  2. POST /backtest/run.
  3. The response is a refusal. `/logs` holds only `api.request served status=409` (or similar), with no `api.sweep` line.
- **Minimal fix:**
  - Emit the same `api.sweep` Warn with `why` on the three arms. Factoring a `refuse_logged(&why)` helper used by all six would do it.
  - Optionally carry a bounded `refusal` string from handlers to the middleware through a response extension, so `api.request` at Warn/Error carries the reason.

### conc13-4 (medium): autopilot and pull-run halt, stall and backoff decisions are never logged and do not survive a restart
- **Sites:**
  - api/autopilot.rs:3388-3433 (`settle`):
    ```rust
    Next::Halt { reason } => {
        site.autopilot.publish(move |status| {
            status.phase = Phase::Halted;
            status.detail = reason;
    ```
    The same holds for `Next::Wait` (:3397-3410) and `Next::Stall` (:3412-3423).
  - The pre-loop halts at :2589, :2614-2640 and :2666 (`fly`).
  - api/pullrun.rs:797-833 (`note_leg_failure`, "This feed is halted for the rest of this run") and :1165 (`note_dead_chain`).

  `grep -n telemetry:: crates/api/src/autopilot.rs` finds only `paused` (Warn) and `resumed` (Info). pullrun.rs has no emit at all.
- **Why it is wrong:**
  - Every retry, stall and halt decision of the two long-running pull drivers lives only in an in-memory `Status`/`Progress`.
  - `detail` is overwritten by the next publish. A Stall's "kept on the page for the life of the process" ends at restart.
  - /logs (the only durable, searchable surface) has no line saying "Dhan halted: credential dead" or "2021-03 passed over after 3 attempts".
  - The per-tick journal records the run, not the decision taken from it.
  - The operator cannot learn, after a restart or after a 12-hour unattended backfill, when and why a feed stopped. The pause, which is the operator's own action, IS logged at Warn. The halt, which is the machine's decision, is not.
- **Repro (not run):** let a feed's credential die mid-backfill. /autopilot.json shows `Halted` with the reason. `/logs?target=autopilot` shows nothing. Restart the api, and the reason is gone.
- **Minimal fix:**
  - In `settle`, emit `autopilot` events:
    - `Halt` at Error, with feed, month and reason;
    - `Stall` at Warn, with feed, month, attempts and reason;
    - `Wait` at Warn (or Info), with month, attempt and secs.
  - Do the same for the `fly` pre-loop halts.
  - In `note_leg_failure`, emit `api.pull "leg failed"` with feed, leg label, status and outcome, at Error for Credential/Permanent.

### conc13-5 (medium): the store's only-trace events cannot be traced to a file. They name a 32-bit symbol hash, not the path, vendor, month or rung
- **Sites:**
  - store/header.rs:735-748, `note_header_fell_back`, with fields `generation, rejected, n_valid, symbol_id, file_len, why`;
  - header.rs:706 (`no committed header`) and :773 (`commit refused`);
  - block.rs:107-117 (`no checksums`), :146-159 (`checksum mismatch`) and :211-224 (`interrupted append`).

  Here `symbol_id = brutex_core::universe::fnv1a(symbol) as u32` (pull/ingest.rs:2103, 2825).
- **Why it is wrong:**
  - store/emits.rs:21-30 says the fall-back and the interrupted-append lines "hand back working data, so the line is the only trace there is".
  - That trace says `fell back to an older generation symbol_id=2851307223`. The operator has to invert a hash over the universe, and even then does not know the vendor (five feeds store the same symbol), the rung or the month.
  - The month whose newest commit was lost, so that it now silently serves older bars, cannot be found from the log.
  - By contrast `store.open` (file.rs:3364) and `store.append` (file.rs:2396) carry `file`.
- **Repro (not run):**
  1. Corrupt the newest header slot of any `bars/<vendor>/.../<month>.bin`.
  2. Open it via `cli sweep-stored`.
  3. `logs/cli/events.ndjson` holds the fall-back line with no path.
- **Minimal fix:** give `Header::read_region`, `commit` and `block::verify` an optional `&Path` (or a `&str` label) from `BarFile`, which holds `bars_path`. Then add `.with("file", ...)` to the six events. `emit_if!` keeps the cost at zero when filtered.

### conc13-6 (low): a ladder halted on its budget exits 0 from `sweep` and `sweep-stored`, and is marked "completed" in the lifecycle and `/backtest/run.json`
- **Sites:**
  - runner/report.rs:876: `Some(halt) => { row(out, "outcome", "REFUSED", "the walk stopped short");`, rendered as `  outcome          REFUSED  the walk stopped short`.
  - cli/lib.rs:14661-14670: `refusal_reason` matches only `refused` at column 0, `REFUSED. ` or `REFUSED -- `.
  - cli/lib.rs:2197: `sweep` checks `carries_refusal || nothing_measured`.
  - cli/lib.rs:1865-1868: `stored_month_arm` checks `carries_refusal` only.
  - cli/lib.rs:3785, where sweep_stored returns `Ok(out)` for a halted outcome.
- **Why it is wrong:**
  - The run's own records disagree:
    - the verdict says "trustworthy as a whole answer NO";
    - the sweep evidence is `Completion::Halted` (lib.rs:18129-18136);
    - the ledger carries the halted byte.
  - Yet the exit code is 0, `cli.lifecycle command finished phase=completed` is written at Info, `cli.sweep ladder walked halted=true` is at Info, and `/backtest/run.json` reports `"status":"completed"`.
  - `sweep-stored` also exits 0 on `NOTHING MEASURED`, which `sweep` already refuses.
  - `audit*` verbs are not affected: `ranked_opening` adds `REFUSED -- `.
  - P8-01 covered `auto` and `auto-stored`. The `sweep` and `sweep-stored` halt case, and the lifecycle consequence, are new.
- **Repro (not run):** `cli sweep-stored <feed> NIFTY 1min <y> <m> 1` on a full month (min_hits 1 exceeds `DEFAULT_CEILING`), then `echo $?` prints 0. Or `BRUTEX_*` ceiling knobs set small.
- **Minimal fix:**
  - Add a `walk_halted(text)` predicate matching the verdict row `outcome ... REFUSED`. Alternatively have the verbs return a code from `outcome.is_complete()` rather than parse text.
  - Use it with `nothing_measured` in both arms.
  - Emit `ladder walked` at Warn when `halted`.

### conc13-7 (low): `sweep-all` exits 0 and logs only successes when some months are refused
- **Sites:**
  - cli/batch.rs:521-630: every refused month is returned as `Row { refused: Some(why), .. }` with no event.
  - batch.rs:788: the only event is the success `stored month swept`.
  - batch.rs:440: `if rows.is_empty() || rows.iter().any(|row| row.refused.is_none()) { return Ok(()); }`.
  - batch.rs:951: `"  REFUSED  {}  — {why}"`, a spelling `refusal_reason` does not match.
  - cli/lib.rs:1664-1667: `if refused { MISUSED } else { OK }`.
- **Why it is wrong:**
  - A sweep over 54,000 months with 20,000 unreadable files exits 0 and writes `command finished phase=completed`.
  - It writes 34,000 Info success lines and zero refusal lines, so /logs and /backtest/run.json show a clean run.
  - The refused count and reasons exist only in the stdout tally (`batch.rs:892`).
  - Months refused before `sweep_evidence::begin` have no evidence record either.
  - A degraded run is shown as healthy (§4), and a repeating store failure is not countable from any durable surface.
- **Repro (not run):**
  1. Truncate one month file under a feed and rung.
  2. Run `cli sweep-all <feed> 1min 500`, then `echo $?`: it prints 0.
  3. `/logs?level=warn` shows nothing.
- **Minimal fix:**
  - Emit `cli.sweep "stored month refused"` at Warn with label and why for each refused row. That is one per instrument-month, which is gate-17-compliant.
  - Emit a summary event with `offered`, `swept` and `refused`, at Warn when `refused > 0`.
  - Return `FAILED` (or a distinct partial code) when `tally.refused > 0`.

### conc13-8 (low): no surface counts failures over a window, so a repeating failure is not visible as a rate
- **Sites:**
  - telemetry sink.rs:497-527: `Health` counts written, dropped and rotations, and has no per-level or per-target count.
  - api/logs.rs:44: `PAGE_LIMIT = 200`, and the page reports "{} event(s)" = rows returned, not rows matching.
  - api/autopilot.rs:1632 and :2253: `MAX_FAILURES = 40` with `truncate`, and `Status` has no failure total.
- **Why it is wrong:**
  - A `pull.spot member did not land` repeating 5,000 times in a backfill shows on /logs as "200 event(s)" and on /autopilot.json as 40 rows. Each is indistinguishable from a burst of 40 or 200.
  - Per-run totals exist (`pull.run finished failed=`, the journal's "Failure diagnostics"), but nothing aggregates across runs or shows a rate on a live surface.
  - The operator's requirement (4) is unmet.
- **Repro:** not run. Read the source.
- **Minimal fix:**
  - Add `failures_total: u64` to the autopilot `Status`, incremented in `Control::fail`, and render it beside the 40-row list.
  - Add `warn` and `error` counters to `Health`: two relaxed atomics on the emit path, O(1). Show them in `sink_json` and the /logs banner.

## Verification of earlier rows touching this theme
| id | state | evidence |
|---|---|---|
| P8-01 | NOT FIXED | cli/lib.rs:2306-2310 `["auto", sessions] => ... OK`; :1180-1182 `auto_stored_arm` still `carries_refusal` only |
| conc9-1 | NOT FIXED | cli/lib.rs:18824-18831 `tick` unchanged (`fetch_add` and then emit outside any order) |
| conc9-2 | NOT FIXED | cli/main.rs:69-84 still prints `where_events_went` only after `run_durable_os` returns |
| conc9-3 | NOT FIXED | telemetry sink.rs:532-534 `is_loud` ignores `clock_held`; `grep clock_held crates/api/src/logs.rs` finds only the test literal at :2014 |

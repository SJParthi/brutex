# Crash and edge-input audit, pass 6

**Verdict at `final/all-fixes-zero` 1f4de71:** 5 new findings (3 medium, 2 low). All three medium findings are in the class "producer/consumer contract drift". In each one, a Rust JSON arm that Rust itself documents and tests as valid is rejected wholesale by the strict web/ validator, and a whole page goes blank. The "silent skip" hunt found 2 low instances. From pass 5, CE-42 is FIXED (cli/src/frontier.rs:3377 now sets `protective_exits_unchecked: true` on the unpriced arm). CE-43 is still NOT FIXED (api/src/server.rs:14466-14475 is still `expiry_of(..).is_ok()`).

Method: the source was read at /home/claude/wt/zero2. Every medium finding was reproduced by running the real web/src validator under Node 22 on the exact JSON shape the Rust arm writes. No cargo run was needed, and nothing was edited.

---

## New findings

### CE-47 (medium): a ragged tail on `results/runs.bin` blanks the whole backtest ledger. Rust and JS disagree on `appendable`.
- **Producer:** crates/api/src/backtest.rs:919-924, `Ledger::to_json`:
  ```
  r#","appendable":{}"#,
  // A RAGGED TAIL IS NOT APPENDABLE ... (Z1-slice11-F1, D-1762).
  self.version == VERSION && self.refusal.is_none() && !self.partial_tail
  ```
  backtest.rs:1144-1149 says *"A RAGGED TAIL IS REPORTED, NOT REPAIRED AND NOT FATAL ... The whole records before it are perfectly good and are served"*. The Rust test `a_ragged_tail_is_named_and_the_whole_records_are_still_served` (:1886-1897) pins this shape: `refusal: None`, both runs served, `"appendable":false`, `"partial_tail":true`.
- **Consumer:** web/src/lib/comparison.js:469-470:
  ```
  if (body.appendable !== (body.version === body.writes_version && body.refusal === null)) {
    return invalidLedger('`appendable` contradicts the read/write versions or refusal state.');
  ```
  The JS rule has no `partial_tail` term. The D-1762 fix changed the Rust side only.
- **Input:** any interrupted `cli` ledger append. A kill or a full disk leaves `(len-16) % 261 != 0` on a version-3 file. This is the exact state the Rust comment calls ordinary.
- **What the user sees:** backtest/+page.svelte:2230-2237 sets `load.phase = 'failed'` with "/backtest.json returned a malformed envelope: `appendable` contradicts the read/write versions or refusal state. Nothing from it was ranked or opened." Every good run disappears from the page. The answer, the comparison, the ledger and the "best complete" headline all go blank.
- **Repro (ran):** a Node script fed `validateLedgerPayload` a healthy v3 envelope, which returned `ok`. It then sent the same envelope with `partial_tail:true, appendable:false`, as Rust writes it, which returned the refusal quoted above. The web fixtures (web/tests/comparison.test.js:73, backtest-ledger-retry.test.js:19) only ever use `partial_tail:false`.
- **Fix:** in comparison.js, use `body.appendable !== (body.version === body.writes_version && body.refusal === null && body.partial_tail === false)`. Add a web fixture with `partial_tail:true`.

### CE-48 (medium): one damaged (unsealed) ledger record empties the backtest page, which the Rust contract says must never happen
- **Producer:** backtest.rs:487-499, the `Run::sealed` doc: *"Reported per record rather than fatally ... one damaged record must not empty the page of the good ones beside it. The page shows the row and marks it."* `Run::from_bytes` (:532-606) decodes every field of a damaged record verbatim (*"every byte pattern is a legal value of its type"*). `to_json` (:629-705) serves those values as they are, next to `"sealed":false`. The Rust test `a_damaged_record_is_shown_and_marked_rather_than_dropped` (:1763-1775) pins "both rows are served".
- **Consumer:** comparison.js:478-480 calls `validateRun(run, false)` on every row, and the first invalid row returns `invalidLedger(...)` for the whole envelope. `validateRun` checks the full schema even for an unsealed row: safe-integer u64s, `1 <= month <= 12`, `months_asked === span`, `bars >= 1`, `pessimistic <= optimistic`, `exit_rungs` in -1..32767, non-empty `feed`/`underlying`/`timeframe`, and `trades == 0` implies all money fields are 0. Its own doc (:172-174) says *"A refused row remains displayable as escaped string metadata"*, but the envelope loop does not do that.
- **Input:** a bit flip, torn sector or partial overwrite inside any checked field of one record. The seal exists to catch this kind of corruption. Examples: the high byte of `bars` (u64 above 2^53), the `from_month` byte, or a `pessimistic` that grows past `optimistic`. Damage confined to the seal bytes or a mask word passes.
- **What the user sees:** the same blank ledger as CE-47. Example message: "runs[0] is invalid: bars is absent or is not an exactly representable integer." The `unsealedRuns` banner at +page.svelte:7572 is never reached.
- **Repro (ran):** each envelope held one sealed valid run and one `sealed:false` run, with the damage below. Each returned `ok:false` with the whole envelope refused:
  - `bars: 6855 + 2**56` → "bars is absent or is not an exactly representable integer"
  - `from_month: 0` → "contradictory span, bar, or support counts"
  - `pessimistic: 400 > optimistic: 300` → "The adverse-fill total exceeds the optimistic-fill total"
- **Fix:** in `validateLedgerPayload`, when `run.sealed === false` and the full check fails, keep the row as escaped metadata. It is already excluded from ranking: `best` requires `run.sealed`, and `trustworthy()` excludes it. Refuse the envelope only for a sealed row that fails. Alternatively, have Rust serve unsealed rows with only identity, index and labels. Add a web fixture with one garbage unsealed row next to a good one.

### CE-49 (medium): live sweep progress is refused as soon as the exit grid reports progress (and, for a terminal sweep, as soon as the grid starts)
- **Producer:** crates/cli/src/lib.rs:18873-18885, `note_grid_progress`:
  ```
  telemetry::Event::info("cli.audit", "exit grid progress")
      .with("rung", rung).with("priced", ..).with("candidates", ..);
  if let Some(held) = recording { event = event.with("feed", ..).with("underlying", ..); }
  ```
  This event never carries an `attempt` field, and never carries `from_year..to_month`. api/src/logs.rs:454-460 derives `attempt_key` only from an `attempt` field, so the record reaches the browser with neither token.
- **Consumer:** web/src/lib/live-progress.ts:311-316. Every `cli.audit` record whose message is in `LIVE_MESSAGES` is checked, and `GRID_PROGRESS` is in that set (:82, :97). The check refuses the whole fold when `exactToken(fields,'attempt','attempt_key')` is null. This happens before the "foreign attempt" skip, so a grid-progress line from any run in the window, including an older one, also refuses the panel. Even with a token, `sameContext` (:321-327) would refuse the missing `from_year`.
- **Second instance (read from source, not run):** a terminal sweep. `grid_entered_event` (:18750-18771), `grid_finished_event` (:18901-18921) and `validation_stage_event` (:18980-19002) add `attempt` only `if let Some(attempt) = held.attempt`. The rung events instead use `progress.attempt.or_else(binding_attempt)` (:13432, :13472), and their doc (:13424-13431) says that fallback exists so terminal sweeps fold. On a terminal sweep, `Recording.attempt` comes from `ask.attempt` and is None (lib.rs:16504). The fix that gave rung events a token for terminal sweeps therefore missed the next three boundaries. The browser refuses at "exit grid entered".
- **Why the tests miss it:** web/tests/live-progress.test.js:35-47 `event()` spreads the full `CONTEXT`, attempt included, into every fixture event. That includes `progress()` at :129, which is not the shape Rust emits. On the Rust side, `every_live_boundary_carries_the_exact_question_inside_the_field_ceiling` (lib.rs:21022-21082) checks six boundaries and omits `note_grid_progress`.
- **What the user sees:** during the pricing phase, the live rung panel switches to "A exit grid progress event has no consistent exact field attempt token." That phase is 87.6% of a measured run, per live-progress.ts:76-81. This is the defect `GRID_PROGRESS` was admitted to fix, now made worse: the phase goes from "priced stays 0" to "the whole panel refuses".
- **Repro (ran):** `reduceLiveProgress` was fed marker, rung sweeping and exit grid entered with the exact Rust field sets, plus logs.rs's derived `attempt_key`. The result was `ready`, with rung 15min `pricing`. After one `exit grid progress` record with the Rust fields `{rung, priced, candidates, feed, underlying}` was appended, the result was `failed: A exit grid progress event has no consistent exact field attempt token.`
- **Fix:**
  - Give `note_grid_progress` the same context and `attempt` fields as `grid_entered_event`. It has room within `MAX_FIELDS`.
  - In all four Recording-based constructors, use `held.attempt.or_else(binding_attempt)`.
  - Add `grid_progress` to the Rust shape test.
  - Make the web fixtures emit the Rust field sets rather than spreading `CONTEXT`.

### CE-50 (low): a pre-1970 clock is silently recorded as `finished_micros: 0`, though the comment says it is kept negative so the operator sees it
- **Site:** crates/cli/src/lib.rs:17842-17848, `record_run`:
  ```
  // ... recorded as the negative it is rather than clamped: a row stamped
  // before 1970 is a clock problem an operator should see.
  finished_micros: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .map_or(0, |d| ...),
  ```
- **Why it is wrong:** `duration_since` returns Err before the epoch, and `map_or(0, ..)` clamps to 0. That is the opposite of the comment, and a §4 fallback: the row is stamped 1970-01-01 00:00 with no event. Do not "fix" it to a negative value. web comparison.js:191-195 refuses a negative `finished_micros` and would blank the ledger (see CE-48). Wiring it up as the comment describes would turn a cosmetic defect into CE-47's symptom.
- **Repro:** not run (needs a pre-epoch clock); read from source.
- **Fix:** refuse the ledger append, or emit a warn event naming the clock, and correct the comment. Keep the stored value non-negative.

### CE-51 (low): a reopened telemetry sink silently restarts `seq`/run ids at zero, and an unreadable newest file is skipped for an older one
- **Site:** crates/telemetry/src/sink.rs:1457-1466, `resume_point`:
  ```
  let len = std::fs::metadata(path).map_or(0, |meta| meta.len());
  (len > 0).then_some((path, len))
  ...
  .and_then(|(path, len)| last_records(path, len))
  .unwrap_or_default()
  ```
  and :1485 `crate::tail::read_at(..).ok()?`.
- **Why it is wrong:**
  1. A read error on the newest non-empty file, or a last block with no decodable line, returns `Resumed::default()`. The caller (:773-799) calls `sink.report` for a torn tail and for a clock floor, but says nothing for this case. The doc (:1435-1440) calls it "a stated limit and not a silent one". It is stated only in docs; at runtime it is silent. `reserve_run_id` then reissues run ids that events in the rolled files still carry, the hazard D-1326/D-1536 were written to close.
  2. A metadata error (EACCES, EIO) on the newest file maps to length 0, so `find_map` moves on to an older file. The doc says this must not happen (*"a corrupt one is not skipped in favour of an older file, whose number could be lower"*).
- **Input:** the newest log file holds a single torn line (killed right after a roll), or it is unreadable while `.1` is readable.
- **Repro:** not run; read from source.
- **Fix:**
  - Return `Result`/an enum from `resume_point` that separates "nothing on disk" from "could not read".
  - `report` the latter once at open, naming the file.
  - Treat a metadata error other than NotFound as "could not read", and do not fall through to an older file.

---

## Sites checked, and why each holds

### Class 1: a Result's Err turned into a skip or default (`.is_ok()`, `.ok()`, `filter_map(..ok)`, `let Ok else continue`, `Err(_) =>` arms; about 330 + 430 + 60 production sites triaged)
- **api booleanjson.rs:197 / booleanevidencejson.rs:300** `require_current().is_ok()`: a stale handle is reopened, and a reopen failure is returned as the route's error.
- **api detail.rs:613** `refresh(handle).is_ok()`: a refused refresh reopens fresh, and the reopen's error propagates. `with_verified` (:634) is the strict variant the detail routes use.
- **api server.rs:2450/2468** `first_at_or_after(at).ok()`: the bisection is an optional narrowing. On failure the whole month is read and `bars_array` filters it, and faults are reported (:2476).
- **api server.rs:2715** `schedule.as_ref().ok()`: the error is carried next to it as `audited.evidence_error = schedule.err()`.
- **api server.rs:5211** `Governor::new(..).ok()`: static descriptors, pinned by the test at :27562.
- **api server.rs:5258** `budgets.lock().ok()?`: poisoning is unreachable under `panic = "abort"`. Note for the fixer: in a debug/test build a poisoned lock makes `HttpSource::sharing(None)` drop the source's governor entirely (pull/src/http.rs:606-613).
- **api server.rs:5705** `newest_record` `.ok()`: documented as display-only; `Journal::look` reports the failure.
- **api recovery.rs:1473** `master.inspect(..).ok()`: the `None` branch (:1501-1504) counts an evidence issue whenever anything is held or missing.
- **api recovery.rs:879** `Ok(loaded.ok())`: the failure is journalled first as "lifecycle UNVERIFIED".
- **api census.rs:1081, render.rs:973, server.rs:3091:** filters over constants or display-only maxima.
- **api mastersrun.rs:553/697** `masters_dir() else`: answers 503 with a refusal.
- **api audit_json.rs:211:** an unparseable page warns once with the asked value.
- **cli boolean_catalog_command.rs:178 / boolean_catalog_prepared.rs:225** `filter_map(.ok())`: only the receipt table is filtered. `collect_families` (:189-217) refuses the cohort and names every Err.
- **cli pool.rs:597:** a non-swept symbol is skipped by design.
- **cli pool.rs:951** (refused pricing): the refusal is named by the renderer (:995-1067), never folded in as zeros.
- **cli frontier.rs:1613/1635:** `chunks_exact` cannot fail. The first schema error is recorded as `damaged`, and only later duplicates are dropped.
- **cli live.rs:725-793** `.ok()?`/`break`: this is the transient live directory, documented as skip-not-refuse. The capped reader (`Some(LIVE_ROW_LIMIT)`) refuses a short row set.
- **cli results.rs:681** `.is_ok()`: it feeds a loud refusal.
- **runner grid.rs:1783/4960** `exit_fill(..).unwrap_or(fills.charged)`: refused bars are excluded upstream (`blocks_without_pricing`, `money_envelope_fits`), and pricing indices are in range.
- **runner trade.rs:1344-1353:** `facts.accepts` guards first, and `None` is accounted as "signal took no trade".
- **store file.rs:2103/2702/3529:** `try_from(..).ok()` flows into `ok_or(StoreError)`.
- **store file.rs:3311** `highest_claim`: `read_region` refuses first.
- **pull manifest.rs:3186:** telemetry for a refusal that is already logged at Error.
- **pull fold.rs:941:** `minute: None` makes `valid` false and the day is reported as unmeasured.
- **pull ingest.rs:3287** `Err(_) => true` (virgin): it reinstalls a full image under the lock, which is safe.
- **pull masters.rs:2078:** test code.
- **telemetry sink.rs:1489:** see CE-51.

### Class 2: Rust JSON compared against web/src strict validators
- **/trades.json** (api trades.rs:88-327, 366-481) against trade-analytics.js:210-520:
  - `count === trades.length` on every arm.
  - On the committed zero-row arm, the policy/direction pair is non-null with `refusal:null`.
  - On the absent arm, both are null with a non-empty refusal.
  - Periods skip `entry_micros == 0` on both sides (cli trades.rs:2474, JS :443). The 0 is unreachable from grid.rs:4198.
  - `largest_win/largest_loss` are seeded from the worst fill on both sides.
  - `worst <= best`, and `worst_trade <= 0` (grid.rs:3063).
  - Paging: the page joins pages, and `MAX_RESULT_ROWS` 4096 / 256 = 16 pages, below `PAGE_GUARD` 64. A count of exactly 4096 is served; 4097 gets `too_large` before any read.
- **/frontier.json** (frontierjson.rs) against frontier-pages.js and frontier-analytics.js:
  - The pagination notice text is byte-identical (frontierjson.rs:246, JS :93).
  - The committed zero-row arm gives `rules:null, refusal:null`, and the absent arm gives `rules:null` plus a refusal. Both are admitted.
  - CE-42 is fixed at frontier.rs:3377.
  - Caps: 4096/256 on both sides.
- **/backtest.json** run rows from sealed writers (cli lib.rs:17837-17880):
  - `bars` is `loaded.bars.len()` on a non-empty load.
  - `worst_trade` and the money fields are all 0 when `selected` is None.
  - `exit_rungs` is -1 or a small ladder index.
  - `best_complete` uses the same filter and tie-break as JS (backtest.rs:846-847, comparison.js:507-514).
  - The configuration-refusal shape (:1265-1268) matches the JS versionless branch.
  - A mid-read refusal gives `appendable=false`, `has_mask=false`, `scanned==runs.length`.
  - Only CE-47 and CE-48 disagree.
- **/backtest/run POST** (sweeprun.rs:1980-1988, 3464-3467, 3505-3540) against sweep-admission.js `sweepSubmission`:
  - Acceptance is 202 with a string `attempt` and `refusal:null`.
  - Every refusal is 400, 409 or 503 with no `attempt`/`attempt_key`, and `started:false` where present.
- **/logs.json** record envelope against live-progress.ts `recordsOf`: `run`, `run_key`, `cut`, `dropped_fields`, the derived `attempt_key` and newest-first order agree. The only defect is in the field set of the four event constructors (CE-49).
- **gap-verdict.js `isWholeAudit`:** every strict check fails safe ("not whole") on a cap or omission. No drift can produce a false "whole".

### Edge inputs
- **Empty arrays / 0 rows:** trades, frontier and backtest all give an explicit zero-row arm accepted by JS (checked above). `limit=0` on /backtest.json gives `scanned 0, hit_scan_cap = total>0`, which JS accepts.
- **i64::MIN / i64::MAX:** frontier sends `i64::MAX` ratios as null (pass 5). A bucket's `best_paisa`/`worst_paisa` is i128 and exact, and JS refuses anything over 2^53 (loud). The OI `i64::MIN` sentinel does not reach these routes.
- **4096 exactly / 4097:** see the trades and frontier rows above. `seek_window` refuses past `(MAX_PAGE+1)*limit` instead of serving a `next_page` the parser would refuse.

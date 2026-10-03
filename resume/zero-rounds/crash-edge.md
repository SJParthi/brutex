# Crash/edge-input helper: resume state (saved 2026-10-03 ~18:35 UTC, usage stop)

Live copy: /mnt/project-files/zero-rounds/crash-edge.md (this file is a snapshot of it).
Audited heads: final/all-fixes 331b05c (passes 1-2), final/all-fixes-zero 5140aca (passes 3-4). Read only; this helper never edits the repo.

State:
- Pass 1 (20 agents, file slices): CE-1. Pass 2 (21 agents, themes): CE-2..CE-22. Pass 3 (5 themes): CE-23..CE-35. Pass 4: HTML sinks 0 new, durable appends 0 new, empty config CE-36..CE-39; wedge states CE-40..CE-41.
- CE-1..CE-41 handed to the zero-findings session (session_019avWwsbRWYrj7eHCev5fVr), which assigned fixers.
- Next on resume (2 agents max): pass 5 theme "same rule implemented twice that drifted" (api vs cli vs web parsers; was stopped at launch); then re-audit every CE fix when the zero-findings session reports the new final/all-fixes-zero head. Stop when a pass finds nothing new.

---

# Crash and edge-input audit (helper thread for "Audit rounds until zero findings")

Target: `final/all-fixes` at 331b05c, read only. Angle: panics, overflow, empty/huge/malformed inputs, boundary dates and sizes.
Per-agent reports (every risky site checked, with the guard that makes it safe) are in the helper session's scratchpad; this file carries the findings.

Context every agent confirmed: workspace lints deny unwrap/expect/panic/indexing_slicing/truncating casts in production code, and the release profile keeps `overflow-checks = true` with `panic = "abort"` (Cargo.toml:181). So only raw arithmetic, division, shifts, std calls that can panic and header-sized allocations could crash; every one found is guarded.

## Pass 1 (20 agents, whole workspace by slice) — 2026-10-03

Slices: api x3, cli x10, runner x2, pull x2, core+costs+greeks, indicators+engine+vocab, store+lake+telemetry. Result: **0 crash/overflow findings**. One low edge-input defect, verified by the helper:

### CE-1: master reader's short-row gate leaves out the `vendor_id` column
- Severity: low (malformed vendor master only; the row is still refused, under the wrong reason)
- Location: crates/api/src/master.rs:268-290 (`Columns::widest`), used at :365 and :404
- Evidence: `widest()` folds `exchange, segment, trading_symbol, instrument_type, expiry, strike` plus the optional columns, but never `self.vendor_id` (set at :251). Its doc says it is "The highest column index this decoder reads", and :418 reads `get(cols.vendor_id)` with an `unwrap_or("")` fallback. When the vendor-id column is the right-most needed column, a row truncated just before it passes the `f.len() <= widest` gate at :404 and decodes with `vendor_id = ""`, then is refused later by `VendorId::new` (crates/core/src/vendor.rs:1726) as an empty id instead of "row has N field(s)". That is the defaulting-to-`""` hazard the comment at :394-403 says this gate exists to stop.
- Repro: a master whose header puts the vendor-id column last among needed columns, with one data row that drops that last field. Expected: "row has N field(s)…"; actual: an empty-vendor-id refusal. Not run.
- Fix: add `self.vendor_id` to the array in `widest()`, plus a test with the vendor-id column last.

Side checks the helper ran and cleared (not findings): `batch.rs:675` discards `attempt.level(..)`'s Result, but `finish` compares `depth_rows` with `acknowledged_depths` (sweep_evidence.rs:722) so a lost row still refuses; `strict_range_knobs::value` accepts only "0"/"1" for BRUTEX_VALIDATE, but api `strict_knobs` (sweeprun.rs:3328) normalises aliases first.

CE-1 update (helper RAN it, throwaway test since deleted): a Groww master with `groww_symbol` last and one row cut before it loads with `errors=[]`, `kept=0`, `skipped={"no vendor id on the row": 1}`. So the malformed row is not even counted as unreadable; it is filed as a routine skip.

## Pass 2 (21 agents, end to end by input type) — findings so far, each re-checked by the helper against the code

### CE-2: one GET to /bars/window.json can abort the release API server on a corrupt stored bar
- Severity: medium (found independently by two agents)
- Location: crates/api/src/server.rs:4227 (`basis_points`), called from crates/api/src/bars.rs:694 (close) and :712 (open interest)
- Evidence: `let delta = last_paisa - first_paisa;` is unchecked; only `first_paisa <= 0` is refused. The doc at :4209-4213 says both args come from `pull::manifest::Closes::paisa` and are non-negative, but `bars::with_change` passes raw stored `bar.close` / `bar.open_interest` (OI only screened for `OI_NULL`). Store reads do not validate prices, and an unsealed (no `FLAG_CHECKSUMS`) month is read with no checksum (store/src/file.rs:2343-2347). Release profile has `overflow-checks = true`, `panic = "abort"`.
- Repro: unsealed month with adjacent closes 100 then i64::MIN (or OI 2 then i64::MIN+1) → `GET /bars/window.json` → "attempt to subtract with overflow", process aborts. Not run.
- Fix: `checked_sub` mapped to `Unknown::Overflow`; correct the precondition comment.

### CE-3: a torn checkpoint `complete` marker blocks every later resume
- Severity: medium
- Location: crates/cli/src/search_checkpoint.rs:263-266 (writer), :457-463 (scan), :485-488 (read)
- Evidence: the marker is created with `File::create_new(directory.join("complete"))` under its final name and then written; a crash/ENOSPC between leaves a 0-31 byte file. `discover_through` counts any regular `complete` file as acknowledged and makes it `latest`; `read_saved` then fails "checkpoint marker width mismatch" on every call, and older intact checkpoints are never tried. Affects AND-checkpoint recovery, boolean search, boolean qualified journal, expression search.
- Repro: run a checkpointed search, truncate the newest `<seq>/complete` to 0 bytes, resume → refused forever until the directory is deleted by hand. Not run.
- Fix: write `complete.writing`, fsync, rename to `complete`, fsync dir; and/or treat a marker that is not exactly 32 bytes as interrupted so `latest` falls back.

### CE-4: pool pass-1 table puts refused instruments FIRST, against its own comment
- Severity: low
- Location: crates/cli/src/pool.rs:682-692 (`render_per_symbol`)
- Evidence: keys are `(false, -dd, worst, net)` for Ok and `(true, MIN, MIN, MIN)` for refusals, sorted ascending then `rows.reverse()`. The reverse puts every `true` (refused) row on top, though the comment says "Refusals sort last". Ties among Ok rows also come out in reverse symbol order.
- Fix: sort with an explicit comparator (Ok before Err, dd ascending) and pin refusal placement in a test.

### CE-5: an empty BRUTEX_LOG_DIR writes the event log into the current directory
- Severity: low
- Location: crates/cli/src/lib.rs:2990-2996 (`log_dir_from`)
- Evidence: `explicit.map(PathBuf::from)` turns `""` into an empty path; the store fallback is skipped and `events.ndjson` opens relative to the cwd; the banner prints `events -> ` and /logs never reads it. Agent RAN `BRUTEX_LOG_DIR= cli top`: created `<cwd>/events.ndjson`.
- Fix: treat empty/whitespace as unset, or refuse it by name.

### CE-6: BRUTEX_VALIDATE=false/off/no turns validation ON in the cli with no notice
- Severity: low (fails safe, but reverses the operator's stated choice silently)
- Location: crates/cli/src/lib.rs:10358-10362 (`validates`) vs crates/cli/src/strict_range_knobs.rs:49-55 (`request_value`)
- Evidence: `validates` turns validation off only for a literal `0`; anything else is ON with no "KNOB REFUSED" line. `request_value` accepts `false/off/no` as valid, and api `knobs_in` (sweeprun.rs:734-737) maps them to `0`, but the cli env path (`validate_from_env`, used by `screen`) has no such mapping.
- Fix: accept the same words as `request_value` in `validates` and refuse any other value by name.

### CE-7: live view marks a row as clearing the Bonferroni bar when |t| is up to ~1.5 milli below it
- Severity: low (wrong verdict at the boundary, shown as "proved" on the page)
- Location: crates/cli/src/lib.rs:17894 (`bar_milli = (bar * 1_000.0) as i64`, truncates down), crates/runner/src/outcome.rs:1574 (`t_milli` uses `.round()`), crates/api/src/livejson.rs:229 (`t_milli.saturating_abs() >= bar_milli`)
- Evidence: bar 5.6739 → bar_milli 5673; t 5.6725 → t_milli 5673 → `clears_bar = true` though t < bar. The allow-reason at :17885 argues a last-place unit cannot move a verdict; it can at the boundary.
- Fix: ceil the bar (with the NaN/range guards `milli` has) so the conversion goes against the finding.

### CE-8: Global Replay V4 keeps every opened VIX month in memory with no bound
- Severity: low
- Location: crates/cli/src/global_replay_v4.rs:470-491 (`VixCatalog.months`, a HashMap that is only inserted into), allocation at crates/cli/src/vix_reference.rs:125
- Evidence: one `VixReferenceMonth` (~2.86 MB per the agent) per distinct (feed, month) a priced trade touches, never evicted; `GlobalReplayV4Bounds` counts only output records. A 10-year OOS span holds ~343 MB, and the `vec!` allocation aborts rather than refuses on OOM.
- Fix: trades are walked in entry order, so drop months older than the current entry month (as the index-stop VIX path already does) and count the month buffer against a stated memory bound.

### CE-9 (plausible, design call for the fixer): a host-dependent Memory/Workers halt is saved as the run's final checkpoint
- Severity: low
- Location: crates/engine/src/lib.rs:2370-2381 (`drain` maps allocation refusal / thread spawn failure to `Breach::Memory` / `Breach::Workers`), :1757-1765, :1665-1667 (`sink.checkpoint` saves the halted level)
- Evidence: docs/20-sweep-resume.md:15 says checkpoints preserve the halt, which is right for Candidates/Pairs budgets (part of the identity) but Memory/Workers depend on the host, not the identity. A rerun on a bigger machine resumes the saved halt and never retries the level.
- Fix: do not persist Memory/Workers halts as terminal, or resume them from the previous level.

### CE-10: the first signal day can be anchored to a stale daily record (wrong pivots / prev-day / prev5 / gap bits, no refusal)
- Severity: medium (realistic data: previous month's `1day` file present but ending early)
- Location: crates/cli/src/stored.rs:2978-2985 (`daily_context_from_span`), lands in crates/indicators/src/anchored.rs:484 (`advance_before`)
- Evidence: for the first signal day the only check is "ANY eligible reference strictly before it" (:2978). The per-day loop below that requires the previous session's daily record only runs when `previous_signal_day` is `Some`, so it never covers the first day. `Span::complete()` (:2490) only refuses an absent month file, not a short one. The exact-minute side does it right via `prior_accepted_session(first_signal_day)` (:3195, :3226-3255), so in one run GapFib anchors to D-1 while pivots/prev-day anchor to an older day.
- Repro: like `an_observed_regular_session_without_its_daily_record_refuses` (stored.rs:4657) but daily = [Mon 2026-08-03], signal = [Wed 2026-08-05] only → Ok, Wednesday anchored to Monday though Tuesday traded. Not run.
- Fix: require the last eligible reference before `first_signal_day` to equal `prior_accepted_session(first_signal_day)` (and optionally the five sessions prev5 needs).

### CE-11: one torn CLI log line older than the newest sweep marker blocks every browser launch
- Severity: medium (a CLI sweep killed by kill -9/OOM/power loss; wrong refusal, not a panic)
- Location: crates/api/src/sweeprun.rs:2342-2351 (`tail_fault_unless_answered`), via `observe_elsewhere` (:2370, :2382) and `claim_execution` (:1767-1773)
- Evidence: the lifecycle walk reads back 256 records (:2370) and `tail.malformed > 0 || tail.partial_tail` is a fault even when `answered` (:2346-2348); only the scan cap is excused. A malformed line OLDER than the newest decoded marker cannot hide a newer record (file order is seq order), which is the comment's stated reason (:2336-2338). `terminate_torn_tail` (telemetry/src/sink.rs:1503) turns a killed write into one malformed line. Browser runs log elsewhere, so only ~128 further CLI commands push it out of the window.
- Repro: agent RAN it with the public telemetry API (throwaway test, deleted): torn fragment, reopen, then a clean started/finished pair → `malformed = 1`, `tail_fault_unless_answered(&t, true)` is `Some`, launch refused.
- Fix: count only faults met before the walk reaches the newest marker (stop the walk at the first marker, or have tail report faults seen before the first returned record).

## Queue (user asked to throttle at 13:13 UTC; resume after 18:00 UTC at 3 agents at a time)
- Pass 2 agents still running at throttle time (allowed to finish, results to be appended here): p2-vendor, p2-lake, p2-trades, p2-costs, p2-sentinel, p2-stats, p2-limits.
- Then send all pass-2 findings (CE-2..CE-11 plus any from those 7) to the zero-findings session.
- Pass 3 (queued, not started), 3 agents at a time, themes not yet covered end to end: (1) web/ front end decoding of API output (masks via /vocab.json, OI sentinel string, huge numbers > 2^53 in JSON), (2) api autopilot + ingest queue state machines under restart/duplicate requests, (3) cli step3 orchestrator / selection V6 with refusals mid-pipeline, (4) pull rate limiter + retry/backoff at limits and clock jumps, (5) store catalog/census on a store root with stray, symlinked or non-UTF-8 entries, (6) re-audit of every CE fix once the zero-findings thread lands them. Stop when a pass finds nothing new.

### CE-12: a 21-byte parquet file aborts the process in `LakeFile::from_bytes`
- Severity: high in the crate, latent today (no workspace crate depends on `lake` yet)
- Location: crates/lake/src/reader.rs:140 (parquet 59.2 `parse_and_finish`); parquet reserves the footer's schema list length before reading entries (parquet_thrift.rs:724)
- Repro: footer `15 02 19 FC FF FF FF FF 07` wrapped as `PAR1` + footer + 4-byte length + `PAR1`. Agent RAN it: requested 206,158,430,112 bytes, allocation failed, process aborted; the crate's `FooterUnreadable` never returns.
- Fix: pre-validate the footer with a decoder that bounds allocation by bytes present, and refuse any schema that is not the lake's flat 8- or 18-element shape before calling parquet.

### CE-13: a deeply nested parquet schema overflows the stack
- Severity: high in the crate, latent today
- Location: same call; parquet rebuilds the schema tree recursively with no depth limit (schema/types.rs:1322-1395). The crate's `detect` shape check runs only after parsing.
- Repro: 1,000,000 nested one-child groups (8 MB file), 8 MiB stack. Agent RAN it: `fatal runtime error: stack overflow, aborting`.
- Fix: same pre-validation as CE-12 (bounded, non-recursive footer check first).

### CE-14: rolling-option contracts in holiday weeks get a closed day as their expiry and a tenor ~4.8x too long
- Severity: medium (silently wrong IV/greeks and a contract filed under two expiry keys; no refusal)
- Location: crates/pull/src/rolling.rs:150-176 (`expiry_of` → `costs::expiry::next_weekly_expiry` / `next_monthly_expiry`), reaching crates/api/src/server.rs:13853 (`rolling_key`) and `pull::tenor::Tenor::between`
- Evidence: `costs::expiry` returns the plain calendar weekday and documents that holidays are the caller's job; `expiry_of` never asks `pull::calendar::kind_of`, which does mark those days `Closed`; `Venue::NseDerivatives.hours_on` accepts the closed day with a 15:30 close. `docs/06-limits.md` §26 and costs/src/expiry.rs still say "there is no holiday calendar in this repository", which is stale.
- Repro: agent RAN a throwaway test (deleted): NIFTY weekly on 2024-08-14 → 2024-08-15 (calendar: Closed); 2024-04-10 → 2024-04-11 (Closed); monthly on 2023-03-29 → 2023-03-30 (Closed). Tenor 108,900 s at 09:15 instead of 22,500 s.
- Fix: in `expiry_of`, refuse (or step back) when the computed expiry is `Closed`, refuse on `Unmeasured`; fix the stale doc sentences. NOTE for the fixer (CLAUDE.md §3 rule 1): the "expiry moves to the previous trading day" rule is not recorded in docs/00-charter.md (only the holiday authority at :768-773 is). Until it is charter-sourced, the safe fix is to REFUSE a closed-day expiry by name, not to step back.

### CE-15: a vendor-supplied expiry string goes into an outbound URL unescaped and is validated only afterwards
- Severity: low (worst case one malformed or parameter-injected request to the vendor; its contracts are never filed)
- Location: crates/pull/src/fno.rs:222-231 (query built with `push_str(&value)`, no percent-encoding), validation later at crates/pull/src/chain.rs:336
- Evidence: an expiry value containing `&`, `=`, `#` or spaces from the vendor's own expiry list is appended raw to the next request's query string.
- Fix: validate the expiry shape (the chain.rs check) before building the URL, and percent-encode every resolved value.

(CE-2 was found a third time, independently, by the sentinel agent.)

### CE-16: an unreadable vendor IV cell is stored as "vendor sent none" (silent fallback, CLAUDE.md §4)
- Severity: low
- Location: crates/pull/src/rolling.rs:633 and :841-847 (`micros_of`)
- Evidence: any non-string/non-number cell, or text `shift_six` cannot read (overflow, exponent form like `1e-7`, junk), becomes `OI_NULL` with no error or event, while OI and spot a few lines above refuse a bad cell. Pricing then solves its own IV and labels it SOLVED; the append-only overlay records forever that the vendor sent no IV.
- Fix: `micros_of` returns a Result; JSON null → absent, any other unreadable present cell → refuse (or at minimum count and emit a warning event).

### CE-17: a profitable variant with return/drawdown below 0.01x prints "-" ("no profit") in the range-all table
- Severity: low (display)
- Location: crates/runner/src/grid.rs:739-750 (`return_over_drawdown`, integer `pessimistic*100 / max_drawdown` truncates to 0), crates/cli/src/lib.rs:15084-15089 (`0 => "-"`)
- Evidence: pessimistic 1,000 paisa, max_drawdown 200,000 paisa → 0 → "-", though the variant made money; the function itself returns 0 for `pessimistic <= 0` too, so the two meanings collide.
- Fix: decide "-" from `pessimistic <= 0` in the renderer, and print a truncated zero as `<0.01`.

### CE-18..CE-22: writer/reader limit mismatches (all low, not run; from the limits agent, spot-checked by the helper for CE-20 and CE-21)
- **CE-18** expression search can publish past the point it can be reopened: crates/cli/src/expression_search.rs:530-565 publishes per step bounded only by CANDIDATES/NODES; reopening refuses at `DIRECTORY_LIMIT` with `owner.lock` counted (search_checkpoint.rs:430) and `HISTORY_LIMIT` (expression_search.rs:641) disagrees by one at exactly 1,000,000. Siblings check before publishing (boolean_search_command.rs:528, boolean_grammar_campaign.rs:270, boolean_qualified_journal.rs:280). Fix: same pre-publish admission check.
- **CE-19** frontier rows written beyond what the api serves: TOP/`BRUTEX_TOP` unbounded, `record_frontier` (cli/src/lib.rs:17107, :17196) writes up to 65,535 u16 ranks; `/frontier.json` and `/top` refuse > 4,096 (api/src/frontierjson.rs:144, topjson.rs:166); `/trades` same for chosen-trade rows (trades.rs:127), which have no writer cap. Fix: cap TOP at the reader bound and refuse above it before the run.
- **CE-20** killed runs lock up the live dashboard: a killed run's live file is never removed (cli/src/live.rs:293, `finish` only on success); the census refuses the whole refresh at `next.len() >= LIVE_RUN_LIMIT` (live.rs:924, 128). ~17 killed 8-rung range runs are enough. Fix: reclaim stale live files whose run lock is gone, or bound the census to the newest N and name the rest.
- **CE-21** archive MAX_DEPTH doc says "one level of headroom" (pull/src/archive.rs:603-610) but the check `depth >= MAX_DEPTH` (:692) puts the deepest real GDFL member at the limit, so one wrapper folder silently skips every Futures -I/-II/-III contract (only a `skipped` count). Test covers depth 5, not 4. Fix: make code and doc agree (allow 4, or change the doc) and test exactly-at-limit.
- **CE-22** `/operation-audit.json` returns 503 for a client error: api parser (api/src/operation_audit.rs:252-257) accepts `limit=0` and any `before`; the cli reader (cli/src/operation_audit.rs:649) refuses them and the api maps that to 503 "audit read unavailable". Fix: refuse in the api parser with 400 like the other page parsers.

## Pass 2 summary
21 agents. 0 panics reachable in normal use except CE-2 (corrupt store bar aborts the server) and CE-12/13 (parquet, latent). 21 new findings total (CE-2..CE-22): 6 medium (CE-2, CE-3, CE-10, CE-11, CE-14, plus CE-12/13 high-but-latent), the rest low. Pass 2 found new issues, so pass 3 is required (queued above).

## Pass 3 (2 agents at a time, on final/all-fixes-zero 5140aca) — 2026-10-03 from 18:06 UTC

### CE-23: the autopilot stops fetching the current month once it has caught up, and says the store is complete
- Severity: high (days silently never pulled while the server runs)
- Location: crates/api/src/autopilot.rs:2451-2477 (`frontier`), stored by `survey` (:2886) and `settle`'s Advance (:3359)
- Evidence: when every month through yesterday's month is held, the loop exits with `month = month_after(last)` and returns it; that becomes the feed's place, which only moves forward (`reconsider` is for stalled months only). The next day is in the same month, below the place, so it is never scanned; `Settled::Complete` publishes "store is complete … a new day is picked up on its own". Also triggered after `DRY_ROUNDS` empty rounds (weekend/holiday yesterday). Both rungs.
- Repro: agent RAN a throwaway test (deleted): store held through 2026-10-02 → frontier returns 2026-11 with no work; feeding 2026-11 back with yesterday 2026-10-03 still offers nothing; a fresh process does offer 2026-10-03.
- Fix: clamp the returned month to yesterday's month and never let `settle` advance past it.

### CE-24: Stop on an already-exited autopilot task erases the halt, and a following Resume reports "running" forever
- Severity: medium
- Location: crates/api/src/autopilot.rs:3988-4021 (`stop`, `start`), :3714-3735 (`admit_resume`)
- Evidence: `fly` exits before its loop when the clock is unusable for its 20-minute allowance or the day rung has no store directory, publishing "halted" with no feeds. `admit_resume` detects an exited task only from that phase; `stop()` overwrites it with "paused" unconditionally, so a later resume is admitted and `start()` publishes "running" (200, accepted:true) with no task behind it. `pause`'s doc says a stop keeps the halt phase; it does not.
- Repro: agent RAN at decision level: halted/no feeds → refused; after the exact `stop()` overwrite → `admit_resume` returns Clear. Full HTTP sequence not run.
- Fix: a "task returned" flag on `Control` that `stop()` cannot overwrite, read by `admit_resume`; `stop()` leaves a halted status alone.

### CE-25: /masters page injects host-supplied text with innerHTML (script injection)
- Severity: medium (needs a hostile or compromised master host / proxy; runs script in the operator's local UI)
- Location: crates/pull/src/masters.rs:1021-1024 and :1038-1041 (rejection message quotes the first 80-120 chars of the host's body), crates/api/src/mastersrun.rs:226-235, :262-265, :418-438 (carried as `refusal` / `attempts[].detail`), web/masters.js:33-37, :59, :61 (`innerHTML`)
- Evidence (helper re-read both sides): a first line like `x,<img src=x onerror=...>` passes the comma check, lands in `refusal`, and is assigned via `c.innerHTML = ...${m.refusal}...` and the attempts `<li>` template. Server CSP sets only `frame-ancestors` (crates/api/src/server.rs:16200-16212).
- Fix: build those cells with `textContent`/`createElement` (as web/typeahead.js does); optionally add `script-src 'self'` to the CSP.

### CE-26: /masters page shows success when the refresh failed to reload
- Severity: low
- Location: web/masters.js:48-70 vs crates/api/src/mastersrun.rs:631-650
- Evidence: on a refused re-parse the route answers 502 with `reloaded:false` and the reason in `universe`; the page never checks status, `reloaded`, `attempted_landed` or `universe`, and its restart hint reads `restart_required`, which this route always sends false. The Svelte /mapping page already handles this.
- Fix: check `r.ok` and `reloaded`, show `universe`; drop the dead `restart_required` hint.

### CE-27: /db bar grid calls two real server reason codes "a defect"
- Severity: low
- Location: web/src/routes/db/+page.svelte:4876-4890 (reason table), :4975, :4978; server codes at crates/api/src/bars.rs:706, :709, :722, :727
- Evidence: `previous_unreadable` and `overflow` have no entry, so cells get the "unknown … a defect here" tooltip; the comment points at `WHY.overflow` in a different table.
- Fix: add both sentences and a test that the table covers every code bars.rs can send.

### CE-28: 32 vendor 429s floor Dhan's daily allowance at 1 request, and each later request sleeps ~24 h
- Severity: medium (feed stalls until restart; only a per-429 warning shows)
- Location: crates/pull/src/rate.rs:689-701 (`relax`, `step_of` = ceiling/32), :940-968 (`reserve`); sleeps at crates/api/src/server.rs:8642-8686 (`await_budget`) and crates/pull/src/http.rs:661-705 (`wait_for_permit`)
- Evidence: each 429 lowers every window by ceiling/32 and drains its bucket, so the 100,000/day window reaches 1 after exactly 32 refusals; neither sleep site caps or refuses a long wait; recovery is ~32 days per step. The module doc says the day quota "stays usable".
- Repro: agent RAN a throwaway test (deleted): permitted = 1, wait 86,390 s, a second queued wait 172,790 s; 3,126/day after 32 successes.
- Fix: apply the decrease only to the window the vendor named (or the shortest window), and refuse by name any wait above a stated bound instead of sleeping it.

### CE-29: rolling POST and discovery GET never read a non-2xx body, so a dead Dhan token is not detected (CLAUDE.md §8)
- Severity: medium
- Location: crates/pull/src/http.rs:2576-2597 (`post_json`, helper re-read: returns `Refusal::answered(status, "the vendor answered {status}")` without the body), :2759-2766 (`Discovery::get`); `Invalid_Authentication` check at crates/api/src/server.rs:8518 can never match here
- Evidence: Dhan's dead-token answer is HTTP 400 with `DH-906` "Invalid Token" (D-0325); without the body it is a plain answered refusal, `roll_every` never re-reads the token and sends it to every remaining cell instead of halting. Conversely a 403 "not entitled" is classified as a dead token and falsely halts with a Parameter Store re-read. The bars path reads the body and handles both. Not run.
- Fix: read (bounded) the refused body on both paths and reuse the bars path's classifier.

### CE-30: a throttle named in a 2xx body is counted twice against the shared limiter
- Severity: low (halves the throttles needed to reach CE-28's floor)
- Location: crates/pull/src/http.rs:3168-3186 (`weigh_body_parsed`, records it since D-0950) and crates/api/src/server.rs:9816-9826 (`with_retry`, records it again; its comment says it covers only what the transport cannot see)
- Fix: drop the second record in `with_retry` and test one named throttle → one decrement. Not run.

### CE-31: five live ledger-v6 stage ledgers keep a half-written record after a failed append, blocking that stage for the rung
- Severity: medium (disk full / file-size limit mid-record; every later open, read-only too, refuses the file as ragged)
- Location: crates/cli/src/population_statistics_v3.rs:2903, population_finalization_v4.rs:2718, population_v6.rs:2444 (helper re-read: `append_raw` = `seek(End) + write_all`, no rollback), execution_v4.rs:4931, population_observations_v1.rs:3432-3445
- Evidence: the same pattern was fixed with `append_with_rollback` in D-0916, D-1620, D-1622, and Lineage V4 / Admission V4 in the same `commit_stored_population_v6_route` use it; these five do not. Effect: exact rerun refuses at that stage, any other training span on the same `ROOT/<stage>/<rung>` refuses, and Selection V6 winner reads refuse via Population V6 / Execution V4. docs/06-limits.md records the gap only for Finalization V3 and unused Lineage V2/V3. Not run.
- Fix: route all five through `append_with_rollback`, each with a failing-write injection test; extend the limits entry about torn 1-63 byte headers to Statistics V3 and Observation V2.

### CE-32: the /pull folder picker's "capped at 60" notice can never print
- Severity: low
- Location: crates/api/src/render.rs:3069 (`capped = suggestions.len() > MAX_FOLDER_SUGGESTIONS`) vs :3176 (walk stops at `out.len() >= MAX_FOLDER_SUGGESTIONS`)
- Evidence (helper re-read): the walk never collects more than 60, so `capped` is always false and a cut list reads as complete; which 60 survive depends on directory order. The constant's doc calls a silent cap "a lie about completeness". No test.
- Fix: collect up to 61 (or carry a `truncated` flag out of the walk) and test the boundary.

### CE-33: an empty BRUTEX_STORE / BRUTEX_MASTERS / BRUTEX_ARCHIVES puts the api's folders in the working directory
- Severity: low (same class as CE-5)
- Location: crates/api/src/server.rs:293-294 (`store_dir_from`: `Some(value) => Ok(PathBuf::from(value))`, helper re-read), :241-242, crates/pull/src/folder.rs:196-197
- Evidence: set-but-empty gives `Ok("")`, so ingest creates `bars/`, `manifest/`, `logs/` in the server's cwd: the "." fallback the refusal text says was removed. The cli canonicalizes and refuses `""`.
- Fix: refuse an empty or whitespace value by name in all three readers.

### CE-34: one `.DS_Store` (or any stray entry) in a search checkpoint folder blocks that search, without naming the file
- Severity: low (macOS store; Finder creates it on open)
- Location: crates/cli/src/search_checkpoint.rs:436-446
- Evidence: discovery accepts only `owner.lock` and 16-hex directories; anything else refuses every start, resume and dashboard read with "invalid checkpoint reservation name".
- Fix: at minimum name the offending entry in the refusal; deciding to skip known OS litter (`.DS_Store`, `._*`) is a design call for the fixer.

### CE-35: the startup CSV-folder search follows symlinks, so a link loop multiplies the walk
- Severity: low (bounded by depth 6, but ~b^6 listings before the server starts)
- Location: crates/api/src/render.rs:3175-3195 (`collect_csv_dirs`, `dir.is_dir()` follows links; the doc says no cycle check is needed because depth is bounded)
- Evidence: `~/Downloads/loop -> ~/Downloads` re-walks the same tree at every level; `assets` and `store::catalog` walks deliberately do not follow links.
- Fix: use `symlink_metadata` / `DirEntry::file_type` and skip links, like the other walks.

Pass 3 complete: 5 themes, 13 new findings (CE-23..CE-35). Pass 4 is running.

## Pass 4 (2 agents at a time)
- HTML-sink sweep (web/ + every api HTML builder): 0 new; only CE-25.
- Durable-append sweep (every write_all/rename/create_new): 0 new. Note for the fixer: CE-31 overlaps the concurrency thread's sel-1 / pop1-2 / pop2-2 rows (conc-pass3/ledgers.md); fix once. Correction to those rows: Execution V3 and Selection V5 are LIVE via `cli ledger-all` (ledger_all.rs:824 → run_chain → all_rung_population_v5.rs:733/878 → commit_stored_execution_v3). V3's `append_raw` (execution_v3.rs:4047-4051) has no rollback but its 1024/128-byte headerless records cannot tear inside one page; add `append_with_rollback` there too when fixing V4.

### CE-36..CE-39: empty or loosely-read config values (all low; helper re-read CE-36, 37, 39)
- **CE-36** an empty `BRUTEX_LOGS` makes the api write `events.ndjson` into the cwd: crates/api/src/server.rs:18380-18381 (`if let Some(named) = named { return PathBuf::from(named) }`). The api's own copy of CE-5 under a different name, so CE-5's fix will not reach it; started from the repo root it leaves an untracked `.ndjson` that gate 1 forbids. Fix: refuse empty/whitespace by name (share one helper with CE-5 and CE-33).
- **CE-37** an empty `BRUTEX_WEB` serves `./build` of the cwd as the front end: crates/api/src/assets.rs:109-110 (`value.map_or_else(default_web_dir, PathBuf::from)`); otherwise the 503 page names an empty directory. Fix: as CE-36.
- **CE-38** `HOME` set but empty turns every `$HOME/...` default into a cwd-relative path: masters dir (server.rs:354), BRUTEX_ARCHIVES default (pull/src/folder.rs:199-202), the §8 credential-config path (server.rs:10200-10205), `~/.aws/credentials` (pull/src/ssm.rs:329-337), folder suggestions (render.rs:3135). Each refuses only an unset HOME. Not run. Fix: one `home_dir()` helper that refuses empty or relative HOME by name.
- **CE-39** `BRUTEX_ARCHIVE_SUGGESTIONS` is off only for a literal `0` (render.rs:3126-3127); `false`/`off`/`no`/` 0` silently leave the `~/Downloads` walk running. Same shape as CE-6. Fix: one shared boolean-knob parser that refuses unknown words by name.

### CE-40: an extra hard link to the store's execution lock refuses every sweep forever, without naming the file
- Severity: low (one-off `cp -al` snapshot or `ln`)
- Location: crates/cli/src/execution_lease.rs:73-86 (`verify`, `nlink() != 1`, helper re-read; message has no path), reached from cli `run_durable` (lib.rs:2101), browser launch (api sweeprun.rs:1761), status probe (sweeprun.rs:2235). Same unnamed refusal in selection_v6.rs:243, global_replay_v4_store.rs:192, checksum_receipts.rs:480 (Execution V3/V4, Population V5, Selection V5, checksum_audit do name the path).
- Fix: name the lock path and the remedy in every such refusal.

### CE-41: one failed log roll disables rotation for the process lifetime, and /logs says the opposite
- Severity: low
- Location: crates/telemetry/src/sink.rs:1147-1158 (`rotation_broken`, set once, never cleared; helper confirmed no reset), :1284-1298 (a failed `reopen` after rename leaves events going to `events.1.ndjson` with no `events.ndjson`); banner at crates/api/src/logs.rs:881-887 says the oldest events "may already have been overwritten"
- Evidence: a transient roll failure (e.g. fd exhaustion) makes the log grow without bound until restart; the banner claims overwrite, which `rotation_broken` guarantees did not happen, and never says a restart re-enables rotation.
- Fix: retry the roll on a later write (or after a bounded number of events) and make the banner state "rotation stopped after a failed roll; restart to resume".

Pass 4 complete: CE-36..CE-41. The wedge agent also lists concurrency.md wedges (cli2-1/sweep-2 widest: one torn `sweep-evidence/attempts.bin` append refuses every sweep store-wide) so the fixer can treat the class together; see its report section "already filed".

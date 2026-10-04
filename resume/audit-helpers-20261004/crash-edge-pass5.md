# Crash and edge-input audit, pass 5 plus fix re-audit

**Verdict (head 1f4de71, final/all-fixes-zero):** 36 of 41 FIXED (CE-12/13 now merged), 2 PARTIAL (CE-19, CE-41), 2 NOT FIXED (CE-9, CE-20), 1 WRONG FIX (CE-14, see CE-43). Pass 5 adds 5 new findings: CE-42 to CE-46 (1 high, 1 medium, 3 low). There are new findings, so another pass is needed.

First audited at 6104a4b. Every row and finding below was re-checked at 1f4de71 (checkout /home/claude/wt/zero2), which merges zero/api-routes, zero/cli-edges-2 and zero/numeric.

## Verification table (state at 1f4de71)

| CE | state | evidence |
|---|---|---|
| CE-1 | FIXED | 3f22aed8: api/src/master.rs `widest()` now folds `self.vendor_id`. Every `get(cols.*)` column at :419-429 is covered. |
| CE-2 | FIXED | api/src/server.rs `basis_points`: `checked_sub`, then `checked_mul`, both mapped to `Unknown::Overflow`. Rounding is safe for base >= 1. No other unchecked bar-field arithmetic remains in api outside tests (bars.rs:2140 is test code). |
| CE-3 | FIXED | 78a216d (D-1909): the marker is written as `complete.writing`, then renamed. Discovery counts a `complete` shorter than 32 bytes as interrupted. |
| CE-4 | FIXED | 9c6284c: cli/src/pool.rs uses one stable ascending key `(refused, dd, Reverse(worst), Reverse(net))` with no reverse. Covered by a test. |
| CE-5 | FIXED | cli/src/lib.rs `log_dir_from` goes through `brutex_core::knob::folder`. |
| CE-6 | FIXED | cli `validates` = `knob::switch`. The fix leaves a drifted twin in the strict path: see CE-44. |
| CE-7 | FIXED | cli/src/lib.rs ceilings `bar_milli` (non-finite becomes i64::MAX). livejson compares strictly `>` (zero/numeric adds an n >= 30 condition, :245). |
| CE-8 | FIXED | global_replay_v4.rs `make_room` caps VIX months at 4. `schedule` sorts attempts by entry time, so the eviction premise holds. |
| CE-9 | NOT FIXED | engine/src/lib.rs:2371-2382 still maps allocation and spawn failure to `Breach::Memory`/`Breach::Workers`, and the halt is checkpointed. The tracker says "found". No unmerged branch addresses it. |
| CE-10 | FIXED | cli/src/stored.rs `anchored_to_prior_session` requires the newest eligible daily record to equal `prior_accepted_session(first_signal_day)`. Both functions use the same eligibility rules. |
| CE-11 | FIXED | 80b389c (D-1914): `observe_elsewhere` re-walks up to the marker. A walk that does not end on the same record keeps the conservative verdict. |
| CE-12/13 | FIXED | Merged df31af3: lake/src/footer.rs pre-walks the footer without recursion (depth 32, schema at most 19 elements, every container count at most the bytes left). Called at lake/src/reader.rs:141. |
| CE-14 | WRONG FIX | pull/src/rolling.rs:185-195 refuses a closed-day expiry, but the api's cadence filter reads that refusal as "no contracts on this cadence" and skips silently: CE-43. |
| CE-15 | FIXED | pull/src/fno.rs `push_encoded` (unreserved set). The expiry is decoded before the request (chain.rs). |
| CE-16 | FIXED | rolling.rs `micros_of` returns `RollingError::Undecimal`. Only JSON null or a missing cell counts as absent. |
| CE-17 | FIXED | cli/src/lib.rs `return_over_drawdown_cell`: `0 if pessimistic <= 0` prints "-", otherwise "<0.01". |
| CE-18 | FIXED (untracked) | cli/src/search_checkpoint.rs:226-236, the shared `publish_inner` guard (79090b06, D-1563), refuses reservation >= DIRECTORY_LIMIT before creating it. Sequences start at 1, so the limit and discovery's `index == DIRECTORY_LIMIT` agree. The tracker still says "found". |
| CE-19 | PARTIAL | Merged ed5f3b3 (D-1981): `frontier::admit_top` caps TOP at 4096 at the writer and before the run. Chosen-trade rows still have no writer cap (api/src/trades.rs:126-140 still refuses above 4096 on read). |
| CE-20 | NOT FIXED | cli/src/live.rs:822/924: `LIVE_RUN_LIMIT` is 128, and no stale live file is reclaimed. |
| CE-21 | FIXED | pull/src/archive.rs:617 `MAX_DEPTH = 5`. Doc and code agree, and a wrapper-folder test exists. |
| CE-22 | FIXED | api/src/operation_audit.rs:216-218: `integer` refuses a leading `0`, so `limit=0` gets 400. `before <= ID_BASE` also gets 400 (:248). |
| CE-23 | FIXED | 092a693: `frontier` clamps to `last` (yesterday's month), with a test. |
| CE-24 | FIXED, with a side effect | 092a693: `stop` leaves `Halted` alone. The side effect is CE-46. |
| CE-25 | FIXED | web/masters.js: `esc()` wraps every server or vendor string that reaches innerHTML (:25, :40-45, :86-91). |
| CE-26 | FIXED | web/masters.js:51-70 reads `reloaded` and `universe`. |
| CE-27 | FIXED | web/src/routes/db/+page.svelte:4886-4888 has `previous_unreadable` and `overflow`. |
| CE-28 | FIXED | pull/src/rate.rs `floor_of`: one permit per second per span (a day floors at 86,400, not 1). Every span is at least 1 s, so the floor is >= 1. |
| CE-29 | FIXED | pull/src/http.rs:2762-2787 `refused_status` reads the body through `refusal_words`, maps SessionDead to a credential refusal, and carries `named` into api `step`. |
| CE-30 | FIXED | api/src/server.rs:9871 `transport_missed_throttle` (non-429, non-2xx only). `with_retry` is bars-only, and `refused_status` records a non-429 named throttle once on the rolling and discovery path, so no path counts it twice. |
| CE-31 | FIXED | Statistics V3, Finalization V4, Population V6, Execution V4, Observations V1 and Execution V3 all append through `fixed_tail::write_at_end` (rolls back to the block start on error). |
| CE-32 | FIXED | api/src/render.rs:3213 collects 61 and offers 60. Boundary tested at :3295. |
| CE-33 | FIXED | api `store_dir_from` and `masters_dir_from`, and pull `folder.rs:202`, use `knob::folder`. |
| CE-34 | FIXED | search_checkpoint.rs:445-484 skips `.DS_Store` and `._*` files and names any other stray entry and its directory. |
| CE-35 | FIXED | render.rs `collect_csv_dirs` uses `DirEntry::file_type`, so a symlinked child is not walked. |
| CE-36 | FIXED | api server.rs `log_dir_from` uses `knob::folder(LOG_DIR_ENV)`. |
| CE-37 | FIXED | api/src/assets.rs:113 uses `knob::folder(WEB_ENV)`. |
| CE-38 | FIXED | Every production HOME read goes through `knob::home` (server.rs:313/363/10289, pull ssm.rs:377, folder.rs:209, render.rs:3160, cli lib.rs). The only raw read left is in an `#[ignore]` test (calendar_of.rs:1918). |
| CE-39 | FIXED | render.rs:3143 uses `knob::switch`. |
| CE-40 | FIXED | cli/src/execution_lease.rs:82-92 names the lock and the remedy. selection_v6.rs:243, global_replay_v4_store.rs:192 and checksum_receipts.rs:480 now print the path. |
| CE-41 | PARTIAL | The banner is now true (api/src/logs.rs:880-893: rotation stops, restart resumes it). The cause is not fixed: telemetry/src/sink.rs:1207-1213 still sets `rotation_broken` once and never retries, so one transient roll failure lets the log grow without bound until restart. |

## New findings (pass 5, theme: the same rule implemented twice, drifted)

### CE-42 (medium): the browser refuses the whole frontier when any row is unpriced. Rust and JS disagree on `protective_exits_unchecked`.
- **Sites:**
  - crates/cli/src/frontier.rs:3346-3353, `Row::verdict`:
    ```
    if !d.priced { return Verdict { stop_unchecked: true, ..Verdict::default() }; }
    ```
    so an unpriced row carries `protective_exits_unchecked: false`. The field's own doc (:3274) says "Always `true`".
  - web/src/lib/frontier-analytics.js:333 requires `row.meets.protective_exits_unchecked !== true` to be false for every row, priced or not, and :297-321 expects all-false `meets` for an unpriced row.
- **Input on which they disagree:** any `/frontier.json` row with `trades == 0`. That is routine: `record_frontier` writes priced rows first and then every ranked row `screen_cap` never priced (cli/src/lib.rs:17161-17207, `Row::of(.., None, None, ..)` at :17939).
- **What the user sees:** the backtest page's combinations table and the rung comparison show "Frontier analytics refused this answer: /frontier.json rows[i].meets does not match its raw cell and run rules." This happens for any run whose TOP exceeds the priced count.
- **Repro (ran):** a Node script feeding `validateFrontierPayload` one unpriced row exactly as Rust emits it (`trades:0`, all-zero money, `reward_to_risk_bp:null`, `meets` all false, `stop_unchecked:true`, `protective_exits_unchecked:false`) returns `ok=false` with the message above. With `protective_exits_unchecked:true` it returns `ok=true`. The Rust side was read from source; the field defaults to false.
- **Fix:** set `protective_exits_unchecked: true` in the unpriced arm (the doc's "Always `true`"). Add a Rust test that `/frontier.json` serves an unpriced row with both unchecked flags true, and a web fixture with an unpriced row.

### CE-43 (high): the CE-14 fix makes the api silently skip whole cadences and chunks of an F&O rolling pull in holiday weeks
- **Sites:**
  - crates/pull/src/rolling.rs:185-195 now returns `Err(NoExpiry{"..marks closed.."})` when the computed expiry is a closed day.
  - crates/api/src/server.rs:14466-14475, `cadence_has_contracts_on`, is `expiry_of(..).is_ok()`. It is the "this underlying had contracts on this cadence" test and is used:
    - at :14296: the WINDOW-level filter, which drops the whole cadence;
    - at :14301: the planned count;
    - at :14373: `continue` per chunk.
- **Why it is wrong:** "contract refused because its expiry is a holiday" and "no contracts exist on this cadence" are now the same `false`. Both are dropped without counting them in `failed` or `declined`, emitting no event, and with `planned` reduced to match, so the walk reports complete. That is a §4 fallback that hides a failure, and worse than the pre-fix behaviour, which at least fetched the data (under the wrong key).
- **Input:** a NIFTY options pull window starting 2024-08-14 (or any window whose first day computes to a closed weekly expiry). `cadence_has_contracts(.., "WEEK")` is false, so no WEEK request is made for the entire window. Per chunk, every chunk whose `from` lands in a holiday week is skipped. For MONTH, a window from 2023-03-29 drops every monthly contract.
- **Repro:** not run as an api test. The premise is proved by the repo's own test `a_computed_expiry_on_a_closed_day_is_refused` (rolling.rs:1397-1425), which uses the shipped Dhan spec (`spec()` at :959 is `Feed::Dhan.descriptor()`, and `expiry_codes.first()` is "1") and asserts `Err` for NIFTY WEEK on 2024-08-14. The api function is that call with `.is_ok()`.
- **Fix:**
  - Give `expiry_of` distinct outcomes for "no contract on this cadence" (withdrawn regime) and "contract refused".
  - Let `cadence_has_contracts_on` skip only the former.
  - Count the latter as a declined or failed cell with its reason, and never filter a whole cadence on the window's first day.

### CE-44 (low): BRUTEX_VALIDATE has two env readers in cli, and one command uses both
- **Sites:**
  - `brutex_core::knob::switch` (core/src/knob.rs:70-82), used by `cli::validates` (lib.rs:10413), accepts 1/true/on/yes/0/false/off/no.
  - crates/cli/src/strict_range_knobs.rs:67 `value()`: `"BRUTEX_VALIDATE" => matches!(raw.trim(), "0" | "1")`. It is used by `settings::current()` and `resolved()`, by api `validate_runtime` (sweeprun.rs:3115) for the server env, and by `ledger_v6.rs:404`. Its test (:235) pins "false" as refused.
- **Input:** `BRUTEX_VALIDATE=false` (or off, no, true, on, yes).
- **What the user sees:** `cli screen` honours it. `cli audit-audited-range` with the same env refuses at audited_range_command.rs:175, `settings::current()`, with "strict range runtime settings refused before computation; invalid fields: BRUTEX_VALIDATE", even though line 207 of that same command reads the knob through the wider `validate_from_env`. A server started with `BRUTEX_VALIDATE=off` refuses every strict browser run unless the body overrides it. This fails safe and loudly, but it contradicts knob.rs's own rationale ("fixed once, here, so the next reader cannot drift from it").
- **Repro:** not run; read from source.
- **Fix:** make `value("BRUTEX_VALIDATE", raw)` be `knob::switch(..).is_ok()` and update the test at :235.

### CE-45 (low): with BRUTEX_LOG_DIR set, the cli tells the operator its events appear on /logs, and they never do
- **Sites:**
  - crates/cli/src/lib.rs:3009-3016 `log_dir_from` honours `BRUTEX_LOG_DIR` outright.
  - lib.rs:3081-3103 `install_log` then always prints "events -> <dir> The /logs page walks BOTH halves -- the server's directory and this `cli/` beside it -- ... so these events appear there."
  - The api reads the cli half only at `<store>/logs/cli` (api/src/logs.rs:285-287, `cli_half`, used by logs_page and logs_json). Its own override variable is a different name, `BRUTEX_LOGS` (server.rs:18606), which only moves the server half.
- **Input:** `BRUTEX_LOG_DIR=/tmp/x cli sweep-stored ...`.
- **What the user sees:** the banner promises the events are on /logs; /logs shows none of them. The operator cannot tell "none were written" from "looked elsewhere", which logs.rs:272-283 itself names as a §4 defect.
- **Repro:** not run.
- **Fix:** print the BOTH-halves sentence only when the dir equals `<store>/logs/cli`, otherwise state that /logs will not show these events. Alternatively, have the api read the same override.

### CE-46 (low): after the CE-24 fix, Stop during the autopilot's clock wait makes Resume refuse with a false "task has returned"
- **Sites:**
  - crates/api/src/autopilot.rs `fly` (:2586): the clock-retry loop (around :2627-2634) publishes `Phase::Halted` with no feeds while the task is ALIVE and sleeping `IDLE_POLL_SECS`.
  - `stop` (:4037-4049) now keeps a Halted phase.
  - `admit_resume` (:3766-3777) treats "no feeds + Halted" as "the backfill task stopped ... it has returned, so nothing is left to read the flag".
- **Input:** an unusable clock at startup, then Stop, then Resume, before the clock recovers.
- **What the user sees:** Resume is refused with a sentence saying the task exited, while it is in fact waiting. The pause flag stays set, and Resume is admitted only after the task later publishes a non-Halted phase. Before the CE-24 fix this sequence was admitted.
- **Repro:** not run.
- **Fix:** the CE-24 finding's suggested shape: a `task_returned` flag set only on `fly`'s terminal returns and read by `admit_resume`, instead of inferring exit from the phase. Alternatively, publish the clock wait as a distinct phase.

## Checked and cleared (pass 5 drift hunt)
- api `detail::Page` (limit 1..256, rows 4096) agrees with web `frontier-pages.js` (256 per page, 4096 rows), and with cli `frontier::MAX_ROWS` through a const assert.
- The JS return/drawdown, reward/risk, avg and Wilson formulas in frontier-analytics.js match `runner::grid::Cell`, including `i64::MAX` sent as null.
- Bars URLs (http.rs `path_safe` and reqwest `.query`) against discovery URLs (fno.rs `push_encoded`): the discovery path does not refuse "." or "..", but no shipped discovery descriptor has a path value. Latent only.
- `/frontier.json` meets against api `livejson` `clears_bar`: separate rules, no shared input.
- api `ingest::parse_vendor` (case-insensitive, empty means Dhan) against cli `parse_vendor` (exact): the sweep route requires `feed` and the cli refuses an unknown spelling by name, so the mismatch is loud.

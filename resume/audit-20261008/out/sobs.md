sobs (observability, tree 9b0614be): 19 NEW gaps (5 medium, 14 low) · 11 FIXED-ON-BRANCH (OBSV-01..11, on attack/observability 838f5e6c, not in this tree) · 1 earlier-lens candidate re-confirmed NOT-FIXED (W5) + 8 UNVERIFIED · 12 BY-DESIGN (DOCUMENTED) · 18 paths/probes COVERED · 9 attack probes run on real tmpfs/ENOSPC

# Observability audit — "save, log, track, debug, search, monitor, dashboard" on every path

Tree `/home/claude/wt-read` at **9b0614be**, read-only. `git merge-base --is-ancestor 838f5e6c HEAD` prints NOT-ANCESTOR, and `git log HEAD..838f5e6c` lists 6 commits (0016a466..838f5e6c). So every OBSV fix is **on branch attack/observability and not in PR 74**.
cli was **read, not run**. Probes ran in `crates/telemetry` only, with `CARGO_TARGET_DIR=/home/claude/t-audit CARGO_BUILD_JOBS=1`. I made real ENOSPC and EROFS conditions with 64 KiB tmpfs mounts and removed them afterwards. Both probe files were deleted, and `git status --porcelain` shows no `sobs` file.

Legend: **COVERED**: crash-safe where it matters, success/failure in the event log, and visible on a page, a JSON endpoint or in cli output. **GAP**: a NEW finding `sobs-N`. **FIXED-ON-BRANCH**: fixed on attack/observability. **BY-DESIGN**: the law or a stated limit says so (quoted).

---

## 1. Durable write paths

| # | Path (what is saved) | Crash-safe? | Event on success / failure | Where a person sees it | Class | Evidence |
|---|---|---|---|---|---|---|
| W1 | Bar month files (`store::file::BarFile::append`) | Yes. Data write, then `barrier`, then seal, then header slot, then `barrier`. A new file gets `fsync_dir` | success `store.append committed` at Debug (filtered at the default Info). Failure returns `StoreError`, which the caller logs as `pull.member "did not land"` Error | /store, /bars, /db, /gaps | COVERED | store/src/file.rs:2360-2556 (`write_fully`…`barrier`…`emit_if!`), file.rs:4033 `fsync_dir`; pull/src/ingest.rs:653 `note_not_landed` |
| W2 | Census manifest (`pull::ingest` append and install) | Yes. Append: `sync_data` barrier, then commit slot, then `sync_all`, then `set_modified`. Install: tmp, `sync_all`, rename, dir `sync_all` | `pull.census` / `pull.file` on failure (`note_census_unpublished`, `note_not_filed`) | /store.json, /db, /ingest/status.json, /audit.json store block | COVERED (the error-count and batch-drop defects are FIXED-ON-BRANCH as OBSV-04 and OBSV-11) | pull/src/ingest.rs:3586-3613, 3640-3650, 1007, 685 |
| W3 | Instrument masters (`pull::masters::replace_locked`) | Yes. `.partial`, `sync_all`, rename, dir sync. A failed dir sync is the explicit `Uncertain` state | `api.masters.attempt` / `api.masters.source` | /masters, /mapping, /masters/status.json | COVERED (an unreadable master shown as absent is FIXED-ON-BRANCH as OBSV-02) | pull/src/masters.rs:947-998; api/src/mastersrun.rs:721 |
| W4 | Cash-session cache (`pull::cash_session_cache`) | `create_new` + `sync_all` + root dir sync. A reinstall after a failed dir sync is not re-synced (OBSV-03) | No event in the module. Failure propagates and is logged by `api.pull`; success is `pull.cash_session "dated eligibility verified"` | Run receipt only | FIXED-ON-BRANCH (OBSV-03) | pull/src/cash_session_cache.rs:495, 518-536; api/src/server.rs:7957 |
| W5 | Vendor captures (`pull::capture`) | File `sync_all`. **No directory fsync** | Failure: `pull.capture` Error. **Success: nothing names the file. The caller drops the path** | **No page lists `captures/`** | GAP sobs-11 | pull/src/capture.rs:247-271; pull/src/http.rs:2860, 3233 `drop(crate::capture::record…)` |
| W6 | Pull run journal `audit/pull.journal` (`api::audit`) | CRC, `write_rolled_back`, file `sync_all`. **No directory fsync when the file or `audit/` is first created** | `api.audit "record not appended"` Error | /audit (web), /audit.json, /audit/page | COVERED for events and visibility, GAP sobs-12 for first-create durability | api/src/audit.rs:1290-1360 (only fsync is :1352), :1501 |
| W7 | Invocation journal (`cli::operation_audit`, used by cli and HTTP) | Yes. Index + record `sync_all`, `directory()` syncs the dir and its parent | `cli.audit "terminal/boundary audit unconfirmed"` **only from `Drop`**. An explicit `finish` failure in `run_durable` is not logged | /backtest/audit.json (InvocationAudit component); `x-brutex-request-audit` header | GAP sobs-4 | cli/src/operation_audit.rs:276-295, 456, 478-523; cli/src/lib.rs:2313-2346 |
| W8 | Recovery journal `audit/recovery-v1` (`api::recovery_journal`) | Yes. `sync_all` before the index publish, rollback with `set_len`, parent sync at open | `pull.recovery` "window unresolved" / "recovery ended". **A BLOCKED activation is not logged** | /pull/recovery, /pull/recovery.json | COVERED, except the BLOCKED reason: GAP sobs-8 | api/src/recovery_journal.rs:30-40, 345-353, 625-631; api/src/recovery.rs:878, 1030, 679 |
| W9 | Recovery STOP control (`api::recovery_control::persist`) | Yes. Journal, then `audit/` sync, then root sync | Failure becomes a 5xx, logged as `api.request` with status only | /pull/recovery | COVERED | api/src/recovery_control.rs:148-168 |
| W10 | Result ledgers `results/{runs,frontier,trades,chosen-trades,detail-sets}.bin` | Records: `fixed_tail` write-at-end, sync-or-roll-back, torn-tail heal. **Directory is never fsynced when a file is first created** | `cli.audit "result set committed"` / `"result set persistence refused"`; `cli.ledger` warns on torn-tail heal | /backtest.json, /frontier.json, /trades.json, /backtest | COVERED for events, GAP sobs-12 for first-create durability | cli/src/lib.rs:23187-23224, 23253, 23263; cli/src/fixed_tail.rs:297, 340, 400; results.rs:934, frontier.rs:908, trades.rs:482, result_set.rs:314 (`create_dir_all`, no dir sync) |
| W11 | Sweep evidence, candidate trades, Boolean candidate persistence, search checkpoints, Selection V6 | Yes. Each module has its own directory sync (heuristic scan: dir-sync calls found) | Run-level `cli.lifecycle` plus result-set events; per-ledger failure text in the cli report | /sweep-evidence.json, /candidate-trades.json, /boolean-*.json, /expression-search.json, /selection-v6.json | COVERED | cli/src/sweep_evidence.rs, boolean_candidate_persistence.rs:217-278, search_checkpoint.rs, selection_v6.rs (dir-sync counts 2/1/5/5) |
| W12 | Research ledgers: Population V1–V6, Execution V3/V4, capability, disposition, admission store, Global Replay V1–V4, institutional statistics, pool OOS, Selection V3–V5, candidate universe, stored-data completeness, anchored lineage, checksum receipts | Per module. Most sync their directory. selection.rs, selection_v4.rs, global_replay*.rs, institutional_statistics.rs and stored_data_completeness.rs showed no directory sync near a create (heuristic, not proven) | Only the run-level `cli.lifecycle` exit code (the reason arrives with OBSV-08), plus `ledger_all` / `ledger_v6` notes | **cli report only. No api route reads any of these modules** | GAP sobs-18 (visibility); durability UNVERIFIED | `grep -ho "cli::[a-z_0-9]*" crates/api/src/*.rs` names none of them |
| W13 | Live view `results/live` | tmp, `sync_data`, rename, no dir sync | Failure returned | /live.json | BY-DESIGN: "this file is not history, and a torn live view costs a poll rather than a run" | cli/src/live.rs:396-415 |
| W14 | Event log `logs/{api,cli}/events.ndjson` (telemetry) | No per-event fsync. Torn tails are terminated, rotation is bounded | Drops counted in `Health`; the first failure goes to stderr once | /logs, /logs.json (the api's own sink health only) | BY-DESIGN durability; GAPs sobs-2, sobs-3, sobs-13 | telemetry/src/sink.rs:47-63, 1151-1301 |
| W15 | Store revisions (`store::repair::publish`) | Yes (`sync_ancestors`) | none | none | BY-DESIGN: no production caller ("integration deliberately left to the calendar and ingest owners") | store/src/repair.rs:1-7, 362, 407, 490-505; no `store::repair` use in api, pull or cli |
| W16 | Lake (Parquet) | read-only crate | `lake.file`, `lake.page`, `lake.schema` | none | BY-DESIGN: no binary depends on `lake` | `grep -rln "lake::" crates/{api,cli,pull,runner}/src` finds nothing |
| W17 | Serve lock stamp `store/serve.lock` | n/a (advisory lock) | An unreadable stamp is reported as "not yet stamped" | startup refusal text | GAP sobs-15 | api/src/server.rs:19234 `read_to_string(path).unwrap_or_default()` |

## 2. Refusal and error paths

| Area | What is logged | Class | Evidence |
|---|---|---|---|
| Every HTTP answer ≥ 400 | `api.request served` at Warn/Error with method, path, status and micros, rationed 50 cross-site and 200 same-origin per window, with a counted "suppressed" summary. The reason is NOT logged unless the handler emits its own event | COVERED for status, BY-DESIGN for reason (docs/06-limits.md:10004-10008: "`/logs` shows the 503, and not why") | api/src/logs.rs:1179-1237; server.rs:16674-16693 (layer order: the log sees the 403) |
| HTTP 2xx | `api.request` at Debug, filtered at the default Info | BY-DESIGN | logs.rs:1191-1197 |
| Handler panic (dev profile) or any panic (release `panic="abort"`) | **No event**. The default hook writes stderr only | GAP sobs-1 | no `panic::set_hook` in crates/ (rg 0 hits); Cargo.toml:191 `panic = "abort"`; pullrun.rs:1322 "the server's standard error, which is the only place a panic is written" |
| /pull/run coordinator | **Zero events.** Leg failures, halted feeds, dead chains and the summary exist only in in-memory progress | GAP sobs-5 | pullrun.rs has 0 telemetry sites; :917-958, :1300-1331; server.rs:11488 `let _flying = tokio::spawn(...)` |
| Broker pull (`broker_run`) | `pull.run started/finished`; member and window refusals in `pull.*` / `api.pull`. Early refusals (unreachable broker, blocked, out of order) come before `note_run_started` | journaled (NotStarted records), telemetry UNVERIFIED | server.rs:8079-8153 |
| F&O contract refusals | Counted and the first 5 reasons kept. **No event, and reasons after the 5th are lost** | GAP sobs-7 | server.rs:11940-11947 vs spot peer :7102-7106 |
| Recovery activation BLOCKED | in-memory and the 503 body only | GAP sobs-8 | recovery.rs:666-682 |
| Sweep launch stamp refusal | `api.sweep` Warn on /backtest/run only. /backtest/descend and /engine/command are silent | GAP sobs-9 | sweeprun.rs:2088-2095 vs :2251-2253, :3436-3438 |
| Sweep environment-budget refusal | none | BY-DESIGN (docs/06-limits.md:9899-9902 "writes no telemetry event") | sweeprun.rs:2100, 2256, 3442 |
| Autopilot exits, halts and stalls; task panic | Status only; the JoinHandle is aborted, never awaited | GAP sobs-6 | autopilot.rs (3 emit sites: paused/resumed); server.rs:20195, 20214 |
| Store scrub /verify.json | **No event for findings.** 200 with findings is logged at Debug only | GAP sobs-10 | api/src/verify.rs (0 telemetry); server.rs:4966-4985 |
| Vendor HTTP, bar windows | `pull.http "vendor answered"` (Trace when ok, Warn with sent params on refusal) | COVERED | pull/src/http.rs:862-931, :3065 |
| Vendor HTTP, discovery GET and POST JSON | no per-answer event | GAP sobs-19 | http.rs:2692 `post_json`, :2900 `Discovery::get` (no `note_answer`) |
| Credentials, SSM, TOTP, rate, config | `pull.secret`, `pull.ssm`, `pull.totp`, `pull.rate`, `pull.config` | COVERED | pull/src/secret.rs (6), ssm.rs (2), totp.rs (2), rate.rs (2), config.rs (2) |
| Store open/read damage | `store.open`, `store.header`, `store.block`, `store.tix`, `store.flock`, driven from production calls | COVERED | store/src/emits.rs header; block.rs:108, 147, 212; file.rs:3776, 3786, 3959 |
| Store plain I/O refusals | returned as `StoreError`. pull logs them (`pull.member`); api shows status only; cli in the report (the reason reaches the log only with OBSV-07/08) | PARTIAL / FIXED-ON-BRANCH | lib.rs:3802 (`sweep_stored`), :4594 (`auto_stored`), :8870 (`top_list`) refuse with no note |
| cli command start/finish | `cli.lifecycle` command started/finished with phase and exit code, **no reason** | FIXED-ON-BRANCH (OBSV-08) | cli/src/lib.rs:2352-2376 |
| cli required admission and terminal audit (`run_durable`) | **none**, and the log says `completed exit_code=0` when the terminal audit then fails | GAP sobs-4 | lib.rs:2318-2345, 2368; operation_audit.rs:456 |
| cli stdout unwritable | `cli.output` Warn | COVERED | lib.rs:3647-3675 |
| cli log could not install | stderr and report line "events are NOT being recorded" | COVERED (named) | lib.rs:3516-3536, 3567-3575 |
| api log could not install | server keeps serving; stdout banner; /logs shows "No sink" | BY-DESIGN (server.rs comment "A LOG THAT CANNOT BE OPENED DOES NOT STOP THE SERVER, AND SAYS SO") | server.rs:20052-20085; logs.rs:924-931 |
| Discarded results on write/sync/log calls | Production hits: only `cli::live` temp cleanup (`let _ = remove_file`) after the primary error is already returned. `let _dropped_when_filtered = telemetry::emit(..)` is by design (drops counted). All other `let _ =` / `.ok()` hits on fs calls are inside `#[cfg(test)]`. Production `.ok()` / `unwrap_or_default` on fs reads: mastersrun.rs:721 (OBSV-02) and server.rs:19234 (sobs-15) | see rows | rg scan, section 2 of method |
| Gate 17 crates (`vocab`, `engine`, `indicators`, `runner`) | no events | BY-DESIGN — CLAUDE.md §5: "Gate 17 silences `vocab engine indicators runner` … its rule is … 'the innermost loop calls nothing at all'" | CLAUDE.md §5 |

## 3. Search, monitor, dashboard

| Subsystem | Events searchable by | Page / endpoint | Gap |
|---|---|---|---|
| All events | `level`, `target` (exact or dotted prefix), `run`, `limit` ≤ 1000. `since` exists only on the branch (OBSV-05). **Not by message text, field value (instrument, feed, path, why) or an upper time bound.** The walk is capped at 4 MiB per half (P7: a rare target 6.6 MB back returns 0 with `hit_scan_cap=true`) | /logs, /logs.json (merges the api and cli halves) | sobs-16; the scan cap is BY-DESIGN and flagged |
| Run correlation | `run` = pull press id (claimed) or cli invocation id (= `/backtest/audit.json` invocation). **Unrelated events emitted during a claimed pull run carry its id** (P9) | /logs?run= | sobs-14 |
| pull | pull.* targets; pull.journal by `feed` and `page` | /pull, /ingest, /autopilot, /audit, /pull/recovery, /pull/run.json (memory only) | sobs-5, sobs-6 (coordinator and autopilot not in the log) |
| sweep | cli.* targets; invocation journal by `invocation`, `before`, `limit` | /backtest, /selection, /backtest/audit.json, /live.json, the *.json evidence routes | sobs-18 (research ledgers have no page) |
| store | store.* targets | /store, /db, /gaps, /bars; /verify.json is **JSON only, no page links it** | sobs-10 |
| api | api.request by target and level | /logs; /health = master read verdict only | — |
| telemetry itself | api sink health on the /logs banner | **the cli sink's health is shown nowhere** | sobs-2 |
| vendor evidence (captures) | none | none | sobs-11 |

## 4. O(1) of the logging and search paths

| Operation | Bound | Where stated | Class |
|---|---|---|---|
| `emit` (admitted) | O(1) per event: one load, one clock read, one mutex, render bounded by the event ceilings (widest line 38,977 B measured in P3), one `write`. A roll is ≤ 2×`keep_files` syscalls. Worst case is OS-bound: p99 3.1 µs at 1 thread, 503 µs at 8 threads, max 5.5 ms with rolls (macOS). Linux p99 2.8–4.6 µs | sink.rs:4-47; docs/06-limits.md §46 (3433-3548); C-T-01/C-T-01b gate p99 flatness | BY-DESIGN (bounded, measured) |
| `emit` (filtered) | one relaxed atomic load | §46; C-T-02 5.8 ns | O(1) |
| `api.request` ration | one mutex and counters | logs.rs:1322-1395 | O(1) |
| `tail` / `/logs` | **Not O(1).** O(limit × line width), capped at `max_scan_bytes` (/logs: 4 MiB per half, 2 halves), then a sort of ≤ 2×limit; `since` exits early. Linear in bytes scanned | tail.rs:1-41; logs.rs:27-33; docs/06-limits.md §47 and :14214 (sort allowance), :11999 (SCAN_BYTES) | BY-DESIGN (named bound); §47's line-width figure is stale: sobs-17 |
| Sink open (resume) | one 64 KiB read, once | sink.rs:1440-1520 | O(1) |
| /audit.json, /backtest/audit.json | page ≤ 32 rows; reads only the shown records | docs/06-limits.md:12316-12345, §35 | bounded |
| /verify.json | O(log length) + O(E_v) file opens | server.rs:4918-4924 (D-1446) | named, not O(1) |

## 5. Attack probes (real ENOSPC and EROFS on 64 KiB tmpfs; output recorded, files deleted)

| Probe | Combination | What was recorded | What was lost | Verdict |
|---|---|---|---|---|
| P1 | disk fills mid-run, space returns, process restarts | `written_during=13 dropped_during=27`; file ends mid-line; next event leads with `\n`; unfiltered tail `malformed=1 missing=Some(27)`; after restart still `missing=Some(27)`, 0 duplicate seqs | 27 events (counted and visible as a gap) | COVERED |
| P2 | log dir path is a file; read-only filesystem; refused install | `cannot create the telemetry directory — File exists`; `install` refused, `global()=None`, `emit → NotInstalled`; EROFS: `cannot open the sink's lock — Read-only file system` | every event, named once by the caller | COVERED |
| P3 | widest event: 15 fields of 612-byte control-char values, 105-byte target, newline in message; then a foreign non-UTF-8 line and a 100 KiB line injected | line = 38,977 bytes, `cut=true dropped_fields=3`, target cut to 48, newline escaped (5 physical lines); foreign lines `malformed=2`, both own records intact, `missing=Some(0)`; restart resumes `next_seq=3` | nothing of ours | COVERED; §47 figure stale (sobs-17) |
| P4 | 8 threads × 500 events; a second sink on the same dir | 4000 written, 4000 decoded, seq 1..4000 strictly increasing, ms monotone; second sink refused by name (flock) | nothing | COVERED |
| P5 | last line truncated, restart | tear named in `last_error` and on stderr, `malformed=1`, next event readable, `missing=Some(0)` (torn seq reused) | the torn event's bytes | BY-DESIGN (sink.rs "What it does NOT recover") |
| P6 | torn last line **and** disk full at open, then space returns | `last_error` predicts the fuse; next `emit → Written`, `written=1 dropped=0`, but the event is **not readable** (`malformed=1`) | the first event of the new process, reported as written | GAP sobs-3 |
| P7 | rare target older than the 4 MiB /logs scan | 6.6 MB log: at 4 MiB `records=0 hit_scan_cap=true`; at 64 MiB `records=1` | not lost, but unreachable from /logs | BY-DESIGN flag; sobs-16 |
| P8 | disk full until the process exits (185 drops), restart | new sink `health.dropped=0`, `next_seq_at_open=21` (re-issues 21..205), `missing=Some(0)`, `malformed=1`, restart notice blames "a kill or a power cut" | 185 events, **with no trace after restart** | GAP sobs-2 |
| P9 | pull run 777 claimed; unrelated `api.request` and `autopilot` events emitted | `from_run(777)` returns `["autopilot","api.request","pull.run"]` | correlation accuracy | GAP sobs-14 |

## 6. NEW findings

| id | severity | crate | file:line | what is wrong in plain words | evidence | how to fix |
|---|---|---|---|---|---|---|
| sobs-1 | medium | api, cli | crates/api/src/main.rs; crates/cli/src/main.rs; Cargo.toml:191 | A crash (panic) is written only to the terminal and never to the event log. Release builds abort and `overflow-checks=true` makes an arithmetic panic the expected failure. The pull coordinator's own text admits stderr "is the only place a panic is written" | `rg 'std::panic::(set_hook\|take_hook)' crates` → 0; pullrun.rs:1322 | Install `std::panic::set_hook` in both mains after the sink: emit an Error `api.main`/`cli.lifecycle` "panicked" event (message, location, thread), call `Sink::sync`, then chain the previous hook. Test: a child process that panics, then read the log |
| sobs-2 | medium | telemetry, cli | crates/cli/src/main.rs; crates/api/src/logs.rs:903-905; crates/telemetry/src/sink.rs:1477-1486 | When the cli loses events (disk full), nobody is told after its single stderr line. /logs shows only the server's sink health. A restart re-issues the lost sequence numbers, so even the gap disappears | P8: 185 dropped, then after restart `dropped=0`, `missing=Some(0)`; no production caller of `Sink::health` in cli (rg: test-only `ledger_all.rs:1550`) | At cli exit read `telemetry::global().map(Sink::health)`; when `is_loud()`, append "N events were not recorded: <last_error>" to the report and exit DEGRADED. Persist the drop count in a small sidecar on the first drop and have the next `Sink::open` report it |
| sobs-3 | low | telemetry | crates/telemetry/src/sink.rs:796, 872, 1667-1678 | If a log's last line is torn and the disk is full when it reopens, the next event is fused onto the fragment and lost while reported as written | P6: `emit=Written written=1 dropped=0 new_event_readable=false malformed=1`. `open` drops the "still torn" state; `around` sets `torn: false` | Make `terminate_torn_tail` return "still torn" and set `Inner::torn = true`, exactly as `emit` does at :1278 (D-1539). Test with the refusing `Target` double |
| sobs-4 | medium | cli | crates/cli/src/lib.rs:2318-2345, 2368; operation_audit.rs:456 | A sweep refused because its required audit could not start leaves no log line. When the terminal audit fails, the log says "completed, exit 0" while the process exits FAILED | code read (cli not run): both arms only `writeln!(out, …)`; `finish` sets `armed=false`, so the `Drop` event never fires | In both arms `note(Event::error("cli.lifecycle","command refused").with("phase","admission"/"terminal_audit").with("reason",why).with("exit_code",FAILED))`. Test with an audit dir that is a file |
| sobs-5 | medium | api | crates/api/src/pullrun.rs (0 emit sites), :917-958, :1300-1331; server.rs:11488 | The multi-feed pull coordinator writes nothing to the log: leg failures, halted feeds, crashed chains and the final summary live only in memory and vanish on restart | `grep -c telemetry pullrun.rs` = 0; `let _flying = tokio::spawn(...)` | Emit `api.pullrun` events in `note_leg_failure`, `note_dead_chain` and at summary, under `emit_for_run(run)`. Await the JoinHandle and log a JoinError |
| sobs-6 | low | api | crates/api/src/server.rs:20195, 20214; autopilot.rs:2650 `fly` | If the autopilot task stops or panics, the log says nothing and its status can stay "running". Its exits and halts are status-only | autopilot.rs has 3 emit sites (paused/resumed); `flying.abort()` with no await | Supervise `fly`: await it and emit `autopilot` Error "stopped" with the reason; emit on each exit arm |
| sobs-7 | low | api | crates/api/src/server.rs:11940-11947 | An expired-contract (F&O) refusal is counted but not logged, and reasons after the fifth are discarded | spot peer logs at :7102-7106; F&O does not | Emit `api.pull "contract refused"` per refusal (rationed) |
| sobs-8 | low | api | crates/api/src/recovery.rs:666-682 | Why a recovery was BLOCKED is kept only in memory and the HTTP reply, never in the log | no emit in `activate_durable`'s failure arm | Emit `pull.recovery` Error "recovery blocked" with `why` |
| sobs-9 | low | api | crates/api/src/sweeprun.rs:2251-2253, 3436-3438 | Two of the three sweep launch routes refuse an unstamped build without a log line | /backtest/run emits at :2088-2095; descend and command do not | Use the same `emit_if!` `api.sweep` Warn in `descend_with` and `command_with_configuration` |
| sobs-10 | medium | api, web | crates/api/src/server.rs:4966-4985; api/src/verify.rs | The only check of the store index against the files on disk reports findings to nobody: it is not logged and no page shows it | verify.rs has 0 telemetry; 200 with findings logs at Debug; `grep -rn verify.json web/src` → none | Emit `api.verify` Warn when not verified (tallies plus first findings); link and render it on /store and /db |
| sobs-11 | low | pull | crates/pull/src/capture.rs:247-271; http.rs:2860, 3233 | A kept copy of an unreadable vendor reply cannot be found from the refusal that kept it, and its file name may not survive a power cut | path dropped; no success event; no directory fsync after `create_new`; no page lists `captures/` | Emit `pull.capture` Info "vendor body kept" with path, feed and kind; fsync the captures dir after create |
| sobs-12 | low | api, cli | api/src/audit.rs:1301-1352; cli/src/results.rs:934, frontier.rs:908, trades.rs:482, result_set.rs:314 | The pull journal and the result ledgers never fsync their folder when first created, so a power cut can lose a file whose first record was "synced" | only the file is `sync_all`ed; contrast operation_audit.rs:276-295 and store `fsync_dir` | fsync the parent (and a newly made directory's parent) once at creation |
| sobs-13 | low | telemetry, api, cli | crates/telemetry/src/sink.rs:1309 | The log is never flushed to disk, not even at a clean shutdown, so a power cut can lose the "exited" line | rg: no production caller of `Sink::sync` | Call `Sink::sync` after `note_exit` in api main, at cli exit, and in the sobs-1 hook |
| sobs-14 | low | telemetry, api | sink.rs:1151-1152; api/src/logs.rs:1230; server.rs:7262-7265 | While a pull run is active, every unrelated event (page loads, autopilot, sweeps) is filed under that run, so "show me this run" mixes in other stories | P9: `from_run(777)` → `["autopilot","api.request","pull.run"]` | Emit request, autopilot and sweep events with an explicit run (`emit_for_run(0 or own id)`), or have pull legs pass their id explicitly instead of the ambient claim |
| sobs-15 | low | api | crates/api/src/server.rs:19234 | When the "already serving" lock file cannot be read, the refusal claims the other server "had not yet stamped" it | `read_to_string(path).unwrap_or_default()` | Name the read error in the refusal |
| sobs-16 | low | telemetry, api | telemetry/src/tail.rs:117-221; api/src/logs.rs:136-196 | The log can be searched only by level, subsystem and run, not by instrument, feed, file or text. Older rare events are out of reach | `Query` has no field or text filter; P7 | Add `field=key:value` and `q=` substring filters to `Query` (same scan cap, flagged) and expose them on /logs |
| sobs-17 | low | docs | docs/06-limits.md:3580-3583 | The limits document says the widest log line is about 14 KiB; it is about 39–41 KB | P3 measured 38,977 B; tail.rs:96-100 says ~41 KB (D-1323) | Correct §47 to the D-1323 figures |
| sobs-18 | low | api, web, cli | crates/api/src/*.rs (no `cli::population*`, `execution_v*`, `global_replay*`, `selection_v3..v5`, `institutional_statistics`, `pool_oos`, `admission_store`, …) | Most research ledgers (population, execution, replay and earlier selection versions) have no page or endpoint; you can only see them in a cli printout | grep of `cli::` references in api | One read-only /ledgers.json plus a page: per ledger, path, record count, last commit and last refusal |
| sobs-19 | low | pull | crates/pull/src/http.rs:2692, 2900 vs :3065 | Vendor answers to instrument-list and POST calls are not logged per answer the way bar downloads are | `note_answer` called only from `window_async` | Call `note_answer` (or a discovery variant) in `Discovery::get` and `post_json` |

## 7. FIXED-ON-BRANCH (attack/observability 838f5e6c, not yet in PR 74)

Each one was checked as still unfixed in 9b0614be at the cited site.

| id | what | still-unfixed site here |
|---|---|---|
| OBSV-01 | merged /logs page claimed `reached_oldest` after the limit cut | api/src/logs.rs:400 `merged` |
| OBSV-02 | /masters/status.json showed an unreadable master as absent | api/src/mastersrun.rs:721 `metadata(&path).ok()` |
| OBSV-03 | cash-session cache reinstall after a failed dir sync not re-synced | pull/src/cash_session_cache.rs:495 `install_and_read` |
| OBSV-04 | failed census append named the requested count, not the landed count | pull/src/ingest.rs:3548 `append_locked` |
| OBSV-05 | live progress refused on old log damage; no `since=` | api/src/logs.rs:136-196 (no `since` param) |
| OBSV-06 | sweep-all refused months left no event | cli/src/batch.rs:531 `sweep_chunk` |
| OBSV-07 | sweep-stored refusal reason not logged | cli/src/lib.rs:3802 |
| OBSV-08 | `command finished` carried no reason (also covers auto_stored :4594 and top_list :8870) | cli/src/lib.rs:2368-2374 |
| OBSV-09 | /ingest called a taken-but-unpersisted stop undelivered | web/src/routes/ingest/+page.svelte |
| OBSV-10 | /audit counted an unreadable older page as held | web/src/routes/audit/+page.svelte, web/src/lib/audit-pages.js |
| OBSV-11 | one refused census row dropped the whole rolling batch | pull/src/ingest.rs:1205 `record_all` |

The branch's Gate 18 api pre-run was incomplete when it was parked (11 of 22 mutants tested) — PARK-L1.md.

## 8. Earlier-lens round-3 candidates (PARK-L1.md), not fixed anywhere

- **W1–W7** (web pages dropping the server's stated reason). The refuter upheld all seven. **W5 re-checked here: NOT-FIXED.** web/src/lib/sweep-evidence.js:43 throws "Saved evidence request failed (HTTP n)" and drops the body's refusal. W1–W4, W6 and W7 were not re-checked: UNVERIFIED.
- Candidate 1 is now sobs-5, 3 is sobs-6, 4 is sobs-7 and sobs-19, 6 is sobs-8, and 7 is sobs-9 (its budget half is DOCUMENTED).
- Candidate 2 (broker_run early refusals before `note_run_started`) and candidate 5 (transport retries unlogged) were not re-verified: UNVERIFIED.
- The earlier lens's low notes "/verify.json has no page" and "/logs has no field filter" are filed here as sobs-10 and sobs-16.

## 9. BY-DESIGN (DOCUMENTED), with the law quoted

| id | what | law |
|---|---|---|
| BD-1 | no fsync per event; a power cut loses the page cache | sink.rs:56-63 "It does **not** survive a power cut … That is deliberate" |
| BD-2 | log bounded at 64 MiB; oldest deleted | sink.rs:88-114 "the journal is the history and is never rotated; this is the stream and is deliberately bounded" |
| BD-3 | dropped events not queued; first failure to stderr once; counted in Health; so `let _dropped_when_filtered = emit(..)` discards nothing that is not counted | sink.rs:66-87 "an unbounded backlog of a log that cannot be written is precisely how a logging subsystem kills its host" |
| BD-4 | gate 17 crates emit nothing | CLAUDE.md §5 "the innermost loop calls nothing at all" |
| BD-5 | failed-request lines rationed (counted summary) | logs.rs:1259-1285 (D-1583, D-1552) |
| BD-6 | HTTP 503 reason not in the log; environment-budget refusal emits nothing | docs/06-limits.md:9899-9902, 10004-10008 |
| BD-7 | /logs scan cap 4 MiB, flagged | logs.rs:21-25 "It does not search the whole history … rendered rather than hidden" |
| BD-8 | a torn record's seq reused after restart | sink.rs:1615-1627 "What it does NOT recover" |
| BD-9 | live view not directory-synced | cli/src/live.rs:399 "this file is not history" |
| BD-10 | lake and store::repair have no production caller | Cargo manifests; store/src/repair.rs:1-7 |
| BD-11 | api serves with no log if the sink cannot open, and says so | server.rs:20052-20057; logs.rs:924-931 "No sink" |
| BD-12 | successful vendor answers logged at Trace only | pull/src/http.rs:884-889, 917-919 "one line per request is ~62,600 on a one-minute backfill" |

## Method (commands actually run)

- Write-path inventory: rg for `sync_all|sync_data|fs::write|File::create|OpenOptions::new|fs::rename|create_new` outside test modules (75 files), plus a heuristic create-vs-dir-sync scan, then reading every non-cli site and the named cli ledgers.
- Discard hunt: rg for `let _… = …(sync_all|flush|write_all|rename|remove_file|…)` and `.ok()` / `unwrap_or*` on fs calls, each hit classified test or production by its enclosing `#[cfg(test)]`.
- Emit inventory: multiline rg over `Event::<level>(` / `emit_if!` targets.
- Probes: `cargo test -p telemetry --test zz_audit_sobs_{1,2} -- --test-threads=1 --nocapture` (7 + 2 passed) on 64 KiB tmpfs mounts (one remounted read-only). Files were deleted and the mounts unmounted.

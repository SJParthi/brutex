# conc-pass17: memory and resource growth in a long-running api server, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero):** 2 new findings (0 high, 0 medium, 2 low): conc17-1 and conc17-2. The api's **in-process** state is bounded. Every process-lifetime collection has a fixed cap, a single slot, or a size that follows the store and is replaced on each change. I found no Arc cycle, no leaked task, no unbounded channel and no `Box::leak` outside tests. The growth that does occur is **on disk**, and it is driven by per-run or per-poll writes that nothing trims:
- conc17-1: a refused in-process sweep rung leaves its live file. 128 of them disable `/live.json`.
- conc17-2: a backtest tab polls two audited routes every 2 s with no visibility gate, so the invocation journal grows by about 86k files per day per tab while a run lasts.

5 known findings on this theme were re-checked. All are NOT FIXED (table at the end).

Method: source reading only, in the read-only checkout /home/claude/wt/zero3 at 1f4de71. No cargo was run, because no finding is medium or high. I read conc-pass6 first and did not refile its IDs. I also checked concurrency.md (resources-*, server*, lifecycle-*), crash-edge.md (CE-20), conc-pass8/10/11/14 and numeric-pass12 so that known items are not refiled.

---

## Inventory: every process-lifetime collection or resource in api (plus the pull, telemetry and store state it holds)

| # | what | file:line | adds | removes | bound | verdict |
|---|---|---|---|---|---|---|
| 1 | `Site.budgets` `Mutex<Vec<Option<SharedGovernor>>>` | server.rs:5295 | boot | never | one per feed; `Governor` is fixed windows (rate.rs:805) | bounded |
| 2 | `Site.calendars.held` `HashMap<Key,(stamp,Arc<Calendar>)>` | calendar_of.rs:136, :2261 | `derived_and_kept` on a census-stamped derivation | replaced per key, never evicted | keys come only from census-held spot series (server.rs:2908-2939, :6411-6421), so the bound is the series count in the store | bounded by the store. A series removed from the store keeps its entry (store-proportional, not a finding) |
| 3 | `Site.calendars.flights` | calendar_of.rs:140, :2154 | leader registers | `Landing::drop` removes, including on unwind | in-flight derivations | bounded |
| 4 | `Site.census` `CensusCache` (stamps + 2 Arcs) | server.rs:3575 | rebuilt when a manifest stamp moves | the old Arc is dropped with its last holder | 1 snapshot | bounded |
| 5 | `census_wire` `Snapshot{Weak, HashMap<(Vendor,Format),Encoded>}` | store_wire.rs:40-78 | per (vendor, format) | whole map replaced when the census Arc changes | 2 × vendors | bounded. The Weak pins only the allocation, so there is no ABA on `ptr_eq` |
| 6 | `audit_rollup` `RollupCache` | audit_json.rs:464 | per feed | replaced on a census change | vendors | bounded |
| 7 | `Site.run` / `Site.sweep` / `recovery_active` | server.rs:5357-5372 | one run | overwritten by the next run | 1 slot each. The sweep report is an `Arc<str>` with a span bound of 240 months | bounded |
| 8 | `Site.parsed` (masters universe) | server.rs:5394 | `reparse` swap | the old snapshot is dropped | 1 | bounded. The masters files land in place (pull masters.rs:942 tmp+rename), so there is no on-disk history either |
| 9 | `Site.censuses/series/folders/entries` boot snapshot | server.rs:5398-5412 | boot | never | store at boot | bounded (retained, not grown) |
| 10 | `autopilot::Control.status.failures` | autopilot.rs:2244-2253 | `fail` | `truncate(MAX_FAILURES)` | MAX_FAILURES | bounded |
| 11 | `FeedState.stalls` / `FeedReport.stalls` | autopilot.rs:1214-1245 | `record_stall` (updates in place, W1-api1-8) | `unstall` | months owed × 2 rungs (D-0949). That grows by one per calendar month, so it is store-proportional | bounded |
| 12 | `pullrun` `flying` handles and ticker | pullrun.rs:966-1000, :1048, :1105 | per pass, per feed | joined each pass; the ticker is aborted | feeds | bounded |
| 13 | recovery `drive` worker, `journal.latest` | recovery.rs:808 | per plan | joined; the journal is dropped with the task | 1 plan (the `site.run` slot) | bounded |
| 14 | detached write handlers (`request_audited_detached`, `detached_pull`) | operation_audit.rs:139-158, server.rs:11186 | per POST | task end | each admission refuses at once (409, seat, `Saturated`). The one queued exception is `/masters/refresh` (conc6-3, known) | bounded except conc6-3 |
| 15 | `detail::{TRADES,FRONTIER,PARENTS,LEDGER}` `Cached<T>` (open File + block index) | detail.rs:664-691 | first read | replaced when the root differs | 1 handle each. The index is O(runs), under the 20,000-run ceiling (results.rs:619-620) | bounded, store-proportional |
| 16 | 9 single-slot reader caches: booleanjson:188, booleanoosjson:142, candidatejson:302, booleanevidencejson:292, indexstopvixjson:119, indexstopcandlesjson:147, indexstopqualificationjson:145, indexstopjson:127, indexstoprankingjson:173; and topjson `SELECTION` :74 | as listed | first view of an identity | replaced only when another identity is asked | 1 decoded body each, admitted up to `BRUTEX_BOOLEAN_OBSERVATION_BYTES` (default 64 MiB, boolean_observation_budget.rs:16-18) | **bounded but large, with no idle release.** After one view of each kind, up to about 9 × budget of decoded evidence stays resident for the life of the process (about 576 MiB of admitted bytes at the default; the decoded size is not measured). D-0546 says outright that the allowance "controls admitted serialized evidence, not measured process RSS". The aggregate check `bytes × MAX_CONCURRENT ≤ isize::MAX` (boolean_observation_budget.rs:33-35) is about addressing, not residency. Not filed: it does not grow, and the docs disclaim it as an RSS bound |
| 17 | expression-search `SESSIONS` `VecDeque<Session>` | expressionsearchjson.rs:114-150 | new snapshot | `pop_front` at 8, `retain` on the same key | 8 Readers. Each Reader's `admitted` set is capped at `MAX_CONTINUATIONS` (expression_search_reader.rs:208) | bounded |
| 18 | livejson `CENSUS` (`cli::live::CensusCache.runs` BTreeMap) | livejson.rs:71, cli live.rs:879-985 | refresh | rebuilt wholesale each refresh | `LIVE_RUN_LIMIT` 128 (live.rs:829). Over the limit it **refuses**, which is what conc17-1 reaches | bounded in memory. Disk growth: conc17-1 |
| 19 | logs `FAILED_LINES` rations | logs.rs:1219 | per failed request | window reset | 2 × 3 counters | bounded |
| 20 | `serving_roots` BTreeSet | server.rs:18176 | per `serve` | (one per process in production) | 1 | bounded |
| 21 | `mastersrun::REFRESH` FIFO | mastersrun.rs:38, :552 | per press | lock release | **unbounded queue** | conc6-3, known, NOT FIXED |
| 22 | telemetry `Sink.inner.buf` | telemetry sink.rs:565 | each emit | cleared, never shrunk | high-water mark of the largest event | bounded |
| 23 | telemetry files | sink.rs (roll) | each line | rotation, `keep_files` | `max_file_bytes × keep_files`. Unbounded only after `rotation_broken`, which is documented and visible in `Health` | bounded |
| 24 | pull capture files | pull capture.rs:79-94, :360 | first answers / unreadable bodies | never | `FEED_COUNT*2*PER_SLOT + FEED_COUNT*PER_SLOT` = 60 files per process | bounded per process. Grows only per restart, by design ("a fixture for ever") |
| 25 | reqwest pools (`http.rs:128`, `ssm.rs:175`) | | per host | idle timeout | a few hosts | bounded |
| 26 | audit pull journal `audit.bin` | audit.rs:1143, page :1254-1285 | per walk | never (append-only by law) | O(pulls). Reads are paged by offset (`MAX_PAGE_RECORDS`) | grows with work, not with polls, and reads are O(page). Not filed |
| 27 | invocation journal `audit/invocations-v1/` | cli operation_audit.rs:510-576 | **per audited request** | never | none (D-1445) | **conc17-2** (rate driven by an ungated poll) |
| 28 | `results/live/<hex>.bin` | cli live.rs:427-437 | per recording rung | `Live::finish` on the success path only | none, and the reader refuses above 128 | CE-20 (killed runs), plus **conc17-1** (in-process refusals) |
| 29 | statics compiled only under `cfg(test)` (census `READ`, `REPLAYS`, `COLD_ADMISSIONS`, `DAY_MONTH_CALLS`, archive `SORTS`, http `PARSES`, all `Box::leak` sites) | | | | | not in production |

**Arc cycles:** none. The two back-references are `Weak` (store_wire.rs:33, audit_json.rs:471). `Site` is held by handlers and tasks only, and holds no task handle or closure that captures itself.

### Web front end (long-lived pages)

| page / module | timers and listeners | arrays and maps across polls | verdict |
|---|---|---|---|
| `lib/store.svelte.js` shared poll | one module listener (`armWake`, :675-683); one awaited `tick` loop; holders released by filter | none grow | clean |
| `+layout.svelte` pull tracker | `inflight` push on start, filter on end in both try and catch (:759-813); the 1 s interval exists only while non-empty and is cleared (:826-831) | `inflight` ≤ concurrent pulls | clean |
| `routes/autopilot` | `trail` `.slice(0, 60)` (:661); interval cleared (:822-823); visibility listener removed (:812-813) | bounded | clean |
| `routes/ingest` | `samples` `.slice(-12)` (:5665); `receipts` reset per run (:6706/:6905/:7192); interval and listener cleared (:7394-7412) | `receipts` append is O(n) per batch but bounded by the run's batches | clean |
| `routes/terminal` | resize, keydown and pointerdown removed (:1280-1283, :1591-1595) | `quotes` cleared on window change (:812-818). Module caches `inflight` and `months` (lib/terminal.svelte.js:215, :506) are cleared on every census generation (`forgetQuotes`, page :81-84), and their keys are bounded by the store × the operator's own selections | clean |
| `routes/audit`, `lib/InvocationAudit.svelte` | `watchVisible`, listener removed | bounded | clean |
| `routes/backtest` | `statusRequests = createPageRequests()` (:2303) **with no visibility gate**. Each running tick fires `fetchLive` and `fetchLiveTop` un-awaited, with no signal (:2474-2478); overlap is capped only by `ASK_MS` 15 s (lib/ask.js:40), so at most about 7 per kind | `pagedRows` per opened run | **conc17-2** (polling continues while hidden, and each poll writes journal files) |

---

## New findings

### conc17-1 (low): an in-process sweep rung that is refused after ranking leaves its live file behind for good. Each such identity adds one file that nothing removes, and at 128 `/live.json` refuses for every run

- **Where:** `crates/cli/src/lib.rs:19862` (`let (live, live_note) = live_view(...)`, after which the file exists on disk). The early exits that drop `live` without `Live::finish` are at :19891 (candidate capture did not start), :19916 (`trade_and_screen` Err), :19992 (unadmitted seal refused), :20008 (`record_unadmitted` Err), :20057 (`evaluator_stored` Err, `return format!("refused: ...")`) and :20182 (admitted seal refused). `record_and_finish` (:18195-18211) finishes only `if committed`. `live_view` (:18337-18341) drops a `Live` whose first `publish` failed, after `Live::open` has already written the header-only file. `Live` has no `Drop` (cli/src/live.rs:410-412: "It is the ONLY remover, and it runs only on the success path"). The reader cap is `LIVE_RUN_LIMIT = 128` (live.rs:829), enforced as a whole-refresh refusal at :931-935.
- **Code:**
  ```rust
  let (report, _committed) = record_and_finish(recording, id, &recorded, live);
  ...
  if committed {
      out.push_str(&live.map_or_else(String::new, crate::live::Live::finish));
  }
  ```
- **Why it is wrong:** the api runs sweeps **in process** (`sweeprun` → `cli` on `spawn_blocking`), so these are not crashes. The server lives on, and every refusal on these paths adds one permanent file under `results/live/`. Two problems follow:
  1. `/live.json` serves each leftover file as a run. It is `Touched` for a day, then `Stale`, and `Freshness` cannot say "dead" (live.rs:440-470). The backtest page's "best rows so far" panel picks the freshest heap. Inside the 24 h `STALE_AFTER_SECS` window, that can be a refused run's rows (backtest +page.svelte:331-360).
  2. Once 128 distinct identities are left, `CensusCache::refresh_at` returns `Err("live directory exceeds 128 runs ...")` on every refresh. `/live.json` then refuses for the life of the store, including for healthy running sweeps, until someone deletes files by hand.

  CE-20 (crash-edge.md:136) covers the same cap reached by **killed** runs, and proposes reclaiming files "whose run lock is gone". This finding is the in-process trigger: no process dies, so an operator who never kills anything still reaches the cap. Either fix closes both if it is written for both.
- **Growth (extrapolation, not measured):** one file of about 0.3–6.8 KB per distinct refused rung identity. The 6,840 B figure is measured in live.rs:413-414 for 25 rows. A rerun of the same identity reuses the name, so growth follows distinct identities and not attempts. An 8-rung range in which every rung meets a recording fault (for example a full disk for `record_all`, which also makes `committed` false) leaves 8 files. 16 such ranges reach the 128 cap.
- **Repro (not run):** make `results/` unwritable for the ledger but not for `results/live/` (or fill the disk after ranking). POST `/backtest/run` over a span with 8 rungs. Each rung's `record_all` refuses, `committed` is false, and 8 files remain in `results/live/`. GET `/live.json` lists 8 runs. Repeat with 16 different spans and `/live.json` answers the "exceeds 128 runs" refusal.
- **Minimal fix:** give `Live` a `Drop` that removes its file unless `finish` already ran (keep `finish` for the reported error text). A `Live` value that goes out of scope then can no longer survive its run, on any exit. Keep CE-20's reclaim for SIGKILL. Alternatively, have the reader skip and count files whose run lease is not held, instead of refusing the whole snapshot.

### conc17-2 (low): an open backtest tab polls two audited routes every 2 s even while hidden. Each poll creates one invocation-journal file and about 8 fsyncs, so `audit/invocations-v1/` grows about 86k files per day per tab for as long as a run lasts

- **Where:** `web/src/routes/backtest/+page.svelte:2303` (`const statusRequests = createPageRequests();`, with no visibility option). Polls are scheduled at `:2395-2522` (`statusRequests.schedule(pollSweep, 2000)`). On every running tick, `:2474` `fetchLive(next.run)` and `:2478` `fetchLiveTop()` also fire. `fetchLiveTop` GETs `/live.json` (:305). The audited list is in `crates/api/src/operation_audit.rs:60-81` and includes `/backtest/run.json` and `/live.json`. Each one calls `journal::begin` (cli/src/operation_audit.rs:510-576): an index append, `create_new` of `<id>.bin` and about 7 fsyncs, then the terminal append and fsync. By contrast, `lib/page-requests.js:36-48` (`watchVisible`), the shared store poll (store.svelte.js:668-693) and `/audit` all skip reads while hidden.
- **Why it is wrong:** D-1445 (06-limits:11750-11793) states that the journal has "no retention, rotation or sharding" and that it "grows while the operator only watches". It gives no rate, and it does not say that the backtest page's status poll is the one unbounded writer that ignores visibility. A sweep, descent or Boolean campaign can run for hours to days, and a tab left in the background keeps writing. Each tab multiplies the rate. This is not O(1) bounded state. The directory insert and lookup cost then depends on an entry count that only grows ("Not timed", per D-1445).
- **Growth (extrapolation from the poll cadence; not measured):** 2 audited GETs per tick, and a tick every 2 s plus the `/backtest/run.json` latency. That gives at most about 86,400 files per day per tab, plus 22 MB per day of index (86,400 × 256 B). On a filesystem with 4 KiB blocks each 512 B file takes one block, so about 350 MB per day. That is about 2.4 M directory entries over a 4-week campaign with one tab open. fsyncs: about 8 × 86,400 ≈ 690k per day.
- **Repro (not run):** start a long sweep (POST `/backtest/run`), open `/backtest`, and switch to another tab for 10 min. `ls audit/invocations-v1 | wc -l` grows by about 600 (2 per 2 s), and no other page writes while it is hidden.
- **Minimal fix:** (a) build `statusRequests` with the visibility gate that `watchVisible` already provides, and resume with one immediate poll on `visibilitychange`. (b) Stop journaling `GET /backtest/run.json` and `GET /live.json` as invocations: they are status reads, not operations. Alternatively, journal reads in a bounded ring and keep `create_new` per-file records for launches only. (b) is the change that bounds the growth. (a) only cuts the rate.

---

## Checked and clean (this pass)

- **No in-process collection grows per request, poll, run or month without a cap.** See inventory rows 1-20 and 22-25. The only unbounded in-process queue is conc6-3 (known).
- **Spawned tasks:** every `tokio::spawn` in production api code is awaited in the same function (`detached_pull`, `request_audited_detached`, mastersrun :526, pullrun chains), aborted (pullrun ticker, `flying` at shutdown), or owned by one slot (recovery `drive`, the press `conduct`, the autopilot `fly`). A disconnect cannot multiply them past their admission, except masters (conc6-3).
- **Channels:** none in production api code. The `std::sync::mpsc` uses are in tests only (census.rs:1362, server.rs:31143/31169, sweeprun.rs:3700/3762).
- **File handles:** the long-lived handles are 4 `detail::Cached` handles, up to 8 expression-search snapshots, the telemetry current file and the serve lock. Per-request handles are closed at scope end. resources-2 (window route) is the known per-request exception.
- **Web:** no interval left running past unmount, and no listener added per render. Every `addEventListener` in `web/src` has a matching removal in its effect's cleanup, or is the single module-level `armWake`. Bounded history arrays: `trail` 60, `samples` 12.

## Verification of known findings on this theme (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| conc6-3 | NOT FIXED | mastersrun.rs:552 `let _refresh = REFRESH.lock().await;` inside the detached work. There is no `try_lock` at the door |
| CE-20 | NOT FIXED | cli/src/live.rs:410-412: `finish` is still the only remover and `Live` has no `Drop`. live.rs:829 `LIVE_RUN_LIMIT = 128` is still a whole-refresh refusal (:931-935). conc17-1 adds the in-process trigger |
| resources-2 | NOT FIXED | server.rs:3406-3440: `bars_window_json` calls `bars::window` directly on the worker, with no `detail::run_store_read`. bars.rs:1251-1264 still pushes every month's file into `opened.files` (MAX_WINDOW_MONTHS 240, :542) |
| conc8-4 | NOT FIXED | pull/src/ssm.rs:879-882: `answer.text().await` with no length cap |
| server1-2 / lifecycle-1 | NOT FIXED | (re-checked only for task lifetime) shutdown still aborts only `flying`. The press, recovery and detached-handler tasks are not drained or bounded at shutdown, per conc-pass6 table H |

Tally: 5 checked, 0 FIXED, 5 NOT FIXED.

# conc-pass2 / poison: lock poisoning, panics under locks and mid-write, Drop impls that do I/O

Verdict: 1 finding (0 high, 0 medium, 1 low). No reachable panic exists inside any production critical section I read. The key fact for this slice is that the shipped binary is built with `panic = "abort"`. Every poison branch in production code is therefore dead in that binary. The unwind-reliant recovery code (Drop guards, `JoinError` handling) runs only in dev and test builds, and several pass-1 "sound" verdicts silently assume unwinding.

Commit 331b05c. I read the source only and did not run cargo. Slice: workspace-wide, non-test code. It covers every `Mutex`/`RwLock` `.lock()`/`.read()`/`.write()` and its poison policy, panics while a lock is held or a file is half-written, and `Drop` impls that do I/O.

## The profile fact everything below depends on

- `Cargo.toml:174-185` `[profile.release]`: `overflow-checks = true` and `panic = "abort"`. `[profile.dev]` (`:127`) sets no `panic`, so dev unwinds, and `cargo test` always unwinds.
- The operator's run configuration is `.claude/launch.json`: `exec cargo run --release -p api -- serve`. `README.md:99` says the same, `cargo run -p api --release`.
- The code base knows this. `sweeprun.rs:936-950` says *"`overflow-checks = true` is set for `release` as well as `debug`, and `panic = "abort"` is set beside it, so [a crafted year] did not return a refusal -- it called `abort()` and took every other in-flight request on the process with it."*

Consequences in the shipped binary:
1. A `PoisonError` can never be observed. `unwrap_or_else(PoisonError::into_inner)`, `.map_err(|_| "... poisoned")`, `if let Ok(..)` and `.ok()?` all behave the same: the poisoned arm is unreachable.
2. A panic anywhere is a process crash at that instruction. No `Drop` runs, no guard unlocks (the kernel releases OFD/flock locks at exit), and nothing is flushed. "Panic while a file is half-written" is exactly the crash-at-line case pass 1 already analysed per writer. It adds no new on-disk state.
3. Poison handling therefore matters only in a dev build (`cargo run -p api` without `--release`) and in tests.

## poison-1 (low): unwind-only panic containment is documented as if it held in the shipped server; in the release binary one panic in any task aborts every concurrent writer

**Where**
- `crates/api/src/pullrun.rs:574-582` (`Finisher` doc): *"A task that panicked ... would leave that `None` in place forever ... `Drop` runs on the panic path, so the slot is released on every exit"*.
- `crates/api/src/pullrun.rs:1053-1066` (`note_dead_chain` doc): *"A panicking task never reaches the code that clears its own fields ... measured 2026-08-20 ... The panic itself goes to standard error through the panic hook and never reaches `telemetry`, so this is the only surface that can say it happened."* The caller is `pullrun.rs:885` `Err(dead) => { note_dead_chain(..) }` after `tokio::spawn(run_chain(..))` at `:871`.
- `crates/api/src/sweeprun.rs:1440-1462` (`Applied` doc): *"MEASURED by attacking the route: a panic there left every knob set for the LIFE OF THE PROCESS ... `Drop` runs during unwinding, so this holds on every exit"*.
- `crates/api/src/sweeprun.rs:1265-1284` (`TaskFinisher::drop`) and `crates/cli/src/operation_audit.rs:445-462` (`Attempt::drop`): `let phase = if std::thread::panicking() { Phase::Failed } else { Phase::Cancelled };`
- `crates/api/src/calendar_of.rs:128-131` and `:205-229`: *"A leader that panics abandons its flight, and each waiter then tries again"*.

**Why it is wrong**

Each of these is correct in a dev or test build, which is where the measurements cited in them were taken. None of them can happen in the binary the operator runs. With `panic = "abort"`:
- A `tokio::spawn`ed chain that panics does not yield `Err(JoinError::Panic)`. The process aborts, so `note_dead_chain` is unreachable for panics and only fires on cancellation.
- `std::thread::panicking()` is never true inside a `Drop`, because `Drop` does not run on a panic. The `Phase::Failed` arm of both audit guards is dead in release.
- The "knobs left set for the life of the process" outcome that `Applied` guards against cannot occur, because the process does not survive the panic.

The comments and the pass-1 verdicts that rely on them describe per-task containment. What actually ships is whole-process termination. The blast radius is every other in-flight request and writer: a sweep mid-`append`, a pull leg mid-ingest, an index-stop journal, the autopilot tick. The serve lock goes with them. §3 rule 6 (honest limits) asks for this to be stated. As written, a reader concludes that a pull-chain panic costs one feed's chain. It costs the server.

**Repro**

Use the `launch.json` configuration (`cargo run --release -p api -- serve`). Start a sweep (`POST /backtest/run`) and a pull (`POST /pull/run`). Any panic inside one pull chain then aborts the process. Overflow-checks panics are the reachable class: `note_dead_chain`'s own doc records that a chain panic was measured on 2026-08-20, and sweeprun.rs:936-950 records an earlier network-reachable overflow. After the abort:
- `/pull/run.json` never shows the "stopped abnormally" sentence. The process is gone, and pull checkpoints are memory-only (`pullrun.rs:600-602`).
- The sweep's operation-audit record stays at `Started`/`Progress`, never `Failed`. Its `sweep_evidence` lifecycle stays `Running`. Both are the documented SIGKILL shape, not the "Failed while unwinding" shape that pass-1 cli1 recorded as verified.

In a dev build the same panic is contained to its chain, exactly as the comments say. The difference exists only in the profile no test runs, which is the trap `sweeprun.rs:944-948` already describes for one route.

**Minimal fix**

Keep `panic = "abort"`; it is a deliberate choice and the right one for a store writer. Correct the five comments to say that the guards cover cancellation in every build and panics only in dev and test builds, and that in release a panic is a process crash recovered by the crash paths. Alternatively, if per-chain containment is genuinely wanted in production, that is a profile decision for `docs/05-decisions.md` (`panic = "unwind"` in release), not something the code can provide on its own.

## Inventory: every production lock and its poison policy

I found no lock held across `.await`. Every std guard in async code is block-scoped, and a `!Send` guard across an await would not compile in the spawned futures. I found no lock-order cycle. The orders I traced are listed below. Policies: **R** reads through (`into_inner`), **F** refuses by name, **S** silently skips (`if let Ok` / `.ok()?`).

| Lock | Sites | Policy | Held across |
|---|---|---|---|
| telemetry `Sink.inner`, `last_error` | sink.rs:1115, 1225, 1241, 1246, 1308 | R | file append (`target.append`), roll (rename); stderr `report` is after `drop(guard)` |
| store `BarFile.verified` | file.rs:1832, 1920, 2367 | R, forgets first | `pread` + CRC of one block |
| pull governor `Arc<Mutex<Governor>>` | http.rs:682, 3184-3196, 3709-3735, 3816-3915, 5086-5254; server.rs:8606, 9765 | R | `reserve` arithmetic is checked (rate.rs:940-962); `record_throttled` emits telemetry (a file append) under it |
| api `site.budgets` (outer) | server.rs:5185 (S, `.ok()?`), 8585 (F), 9761 (S) | mixed | the inner governor lock (order budgets → governor, never reversed) |
| api `site.run` | pullrun.rs:526, 548, 567; recovery.rs:553, 563, 580, 1683; autopilot.rs:3164; server.rs:10924, 10964, 10986 | R | `recovery_control::stop` fsyncs at 10986 (known runs-3/recovery-5) |
| api `site.recovery_active` | recovery_control.rs:28, 37 (R); 46, 59, 82 (F) | mixed | `persist` / journal open and fsyncs (F sites) |
| api `site.sweep` | sweeprun.rs:1258, 1281, 1685, 1695, 2093 (R); booleanlaunch.rs:280, indexstoplaunch.rs:323 (F) | mixed | O(1) clone/assign only |
| api `ADMISSION: Mutex<()>` | sweeprun.rs:1681 | R | the whole `prepare`: lease flock, audit begin with fsync (known W1-api6-2 design) |
| api `site.parsed` RwLock | server.rs:5407 (R), 5491 (R) | R | a three-field move |
| api `site.reload_lock: Mutex<()>` | server.rs:5442 | F | the whole masters parse `universe(masters)` |
| api `site.census` | server.rs:3638, 3693 | R | lookup or Arc swap only; the read is outside the lock |
| api `store_wire::Cache` | store_wire.rs:55 | F (test `poisoned_cache_refuses_instead_of_serving_stale_bytes`) | `build()` (body encoding) |
| api `detail::Cached<T>` (TRADES, FRONTIER, PARENTS, LEDGER) | detail.rs:583, 615 | R | `refresh` (flock shared, incremental index) and `f` |
| api static page caches (boolean*, candidate, expressionsearch, indexstop*) | booleanjson.rs:191, booleanoosjson.rs:145, booleanevidencejson.rs:270, candidatejson.rs:305, expressionsearchjson.rs:117 (F); indexstop*json `try_lock` (F/busy) | F | reader open and projection |
| api `calendar_of` `held`, `flights`, `landed` | calendar_of.rs:216, 223, 2059, 2091, 2120, 2170 | R | map ops; Condvar wait loop re-checks `Deriving` (spurious-wake safe) |
| api `livejson::CENSUS` | livejson.rs:128 | R | `CensusCache::refresh` (commit-at-end, live.rs:887-967) |
| api `audit_json::RollupCache` | audit_json.rs:489, 501 | R | build |
| api `autopilot` `status` | autopilot.rs:2140 (S), 2160 (F→None), 2220 (F→Halted json) | mixed | pure closures |
| api `serving_roots` | server.rs:17653 (R), 17585 and 17822 (S) | mixed | BTreeSet insert/remove |
| cli `LEDGER: Mutex<()>` | lib.rs:20247 | F | the whole result-set commit (flock, frontier, trades, results, fsyncs) |
| cli `results::WRITER` | results.rs:780 | F | `Results::open`, `refresh`, `append`, `confirm_durable` (fsync) |
| cli `operation_audit` `State` | operation_audit.rs:404, 480 (R); 415, 429, 493 (F) | mixed | `append` = try_lock + write_all + `sync_all` |
| cli `sweep_evidence` `failure`, `depth_digest`, `ranked_digest` | sweep_evidence.rs:582, 640, 662, 671, 738, 750 | F | `level()` holds `depth_digest` across `append_row` (write + barrier) |
| cli `sweep_evidence::FLUSHED` | sweep_evidence.rs:1137 | R | set ops; epoch check (FORGOTTEN) under the same guard |
| cli `candidate_trades` `state` | candidate_trades.rs:351, 419, 466, 507, 573, 591 (F); 426 (S) | mixed | `write_exact` (tier/trade/candidate files) |
| cli `knobs` store/refusals RwLock | knobs.rs:125, 236, 292, 320, 360, 363, 383 | S | map ops (known hunt-conc-6) |
| pull `masters` cookie jar | masters.rs:604, 635 | R | header map ops |

Every `File::lock`/`lock_shared`/`try_lock` hit in `crates/cli/src/{selection*,population*,execution*,anchored_search_lineage*,global_replay*,institutional_statistics,stored_data_completeness,checksum_receipts,frontier,trades,result_set,admission_store}.rs` is an OS file lock (flock), not a mutex, and has no poison semantics. All of them use the explicit `lock → *_locked → unlock` shape with no guard (D-0693). In a dev build, a panic inside `*_locked` therefore skips the unlock. The lock is then held until the `File` drops. When the owner object is dropped during the unwind, that drop releases it. See latent item L4 for where it can outlive the panic.

## Checked and latent (not counted: no reachable panic in the critical section)

Each item below needs a panic while the lock is held. I found none reachable in these critical sections: they hold map ops, checked arithmetic, `write_all`/`sync_all`, and checked decoding. In release they cannot happen at all.
- **L1. `Mutex<()>` locks that refuse forever on poison.** `Site::reload_lock` (server.rs:5440-5443, "master reload lock poisoned; previous universe retained") and cli `LEDGER` (lib.rs:20247-20251). A `()` guard carries no invariant, and the real cross-process guard (`ResultSetLock` flock, or the swap under `parsed.write()`) is unaffected. Yet one dev-build panic during a masters refresh or a result commit disables that function for the life of the process. This contradicts the policy stated three screens up, at server.rs:5397-5404: *"Refusing every request afterwards would turn one panicking request into a dead server"*. Fix if it ever matters: `unwrap_or_else(PoisonError::into_inner)` on both, as `ADMISSION` already does.
- **L2. `serving_roots` asymmetry (adds to errpaths line 58 / pass-1 server2).** `take_serve_lock` reads through poison (server.rs:17653-17672), but `ServeLock::drop` (17585) and `release_root` (17822) use `if let Ok` and skip removal. On a poisoned set, the first serve's key is never removed. The next `take_serve_lock` of that root in the same process then gets the pass-through `ServeLock { held: None }` and serves **without the file lock**, so a second process can take `serve.lock` beside it. errpaths-58 described the consequence as a dropped write; the actual consequence is a lock bypass. It is still unreachable, because the only work under that guard is `BTreeSet::insert/remove` of an already-built `PathBuf`.
- **L3. `site.budgets` refusal text is inverted.** server.rs:8585-8592 refuses because *"the allowance already spent is unknown"*. But the outer lock guards only the list of `Arc`s. The allowance lives in the inner governor mutex, which this same block reads through poison at 8606-8609, as do all of http.rs's sites. Same mutex, three policies (5185 S, 8585 F, 9761 S). The S sites are pass-1 pull1-2.
- **L4. Explicit-unlock writers whose owner survives the panic.** `results::with_shared_writer` keeps its `Results` in the `WRITER` static (results.rs:777-795). A dev-build panic inside `Results::append_locked` (results.rs:1268) skips `self.file.unlock()` (results.rs:1189-1193) and poisons `WRITER`. `WRITER` is then never reused or dropped, so the **exclusive** flock on `results/runs.bin` stays held for the life of the process. Every `cli` writer on that store and every shared-lock reader, including the api's own `detail::LEDGER` handle (a separate open file description), would block in `lock()`/`lock_shared()` indefinitely. I found no panic source in `append_locked`: `to_bytes` uses fixed-layout slices and the arithmetic is saturating. The fix if wanted: hold the lock in a `store::flock::Flock` guard, whose `Drop` unlocks, or reset `WRITER` through poison.
- **L5. Mixed policies on one lock.** `recovery_active`: set and clear read through poison, while `stop`, `is_stopped` and `clear_stop` refuse. `site.sweep`: sweeprun reads through, while the Boolean and index-stop progress callbacks refuse; the run's result is then still installed by `TaskFinisher::finish` through poison, so the end state is a consistent refusal. Neither has a panic source under the guard: `persist` errors are `Result`s.
- **L6. `record_throttled` emits telemetry under the governor and, at server.rs:9761-9767, under `site.budgets` too, on a Tokio worker.** The emit is a file append. The only blocking stderr write is `Sink::report`, which runs at most once per sink and after the sink guard is dropped, but still under these two locks. Not a finding without a full stderr pipe.

## Drop impls that do I/O (production)

| Drop | I/O | Notes |
|---|---|---|
| `store::flock::Flock` (flock.rs:169-180) | `unlock` syscall; on refusal `note_unreleased` (telemetry) | Lock order store → sink only. No cycle. |
| `api::ServeLock` (server.rs:17579-17589) | drops the `Flock`, then the set removal | L2 |
| `cli::execution_lease::Lease` (via `Flock`) | unlock | kernel releases on crash |
| `cli::sweep_evidence::Attempt` (sweep_evidence.rs:881-897) | `write_terminal(Refused)`: O(rows) re-read and hash, lifecycle append with barrier, journal append with barrier | Only on blocking threads or the cli main thread; I found no `Attempt` held in async api code. If the panic it is unwinding from happened inside `level()` holding `depth_digest`, `seal` refuses "digest lock poisoned", no terminal is written, and the lifecycle stays `Running` with a telemetry error. That is loud, not wrong. |
| `cli::operation_audit::Attempt` (operation_audit.rs:445-462) | `finish` = try_lock + write + `sync_all` | See poison-1 for the `panicking()` arm. The guard is taken in a `let` temporary and released before `telemetry::emit`, so the telemetry call cannot re-enter it. |
| `api::sweeprun::TaskFinisher` (sweeprun.rs:1265-1284) | operation audit finish (fsync), then `site.sweep` | Lock order: audit `State` → `site.sweep`. No site takes them in reverse; the Boolean and index-stop callbacks take only `site.sweep`. |
| `api::sweeprun::Applied` (1459-1462) | none (knob map clear) | poison-1 |
| `api::pullrun::Finisher` (590-605) | none (memory) | pass-1 runs finding about generation still stands |
| `api::calendar_of::Landing` (211-229) | none | flights then landed, each taken and released alone |
| `api::server::Slot`, `detail::Permit`, `autopilot::{Seat, AllSeats}`, `boolean_observation_file::Projecting`, `index_stop_*::Projection` | atomics only | |

## Pass-1 verification (items touching this slice)

- **hunt-conc-6 / errpaths line 58 (knobs, autopilot `publish`, ServeLock `if let Ok`)**: CONFIRMED as latent. The code is unchanged: knobs.rs:125-128, 291-293, 319-327, 359-366; autopilot.rs:2139-2143; server.rs:17585, 17822. It is unreachable in release (abort). L2 corrects the ServeLock consequence.
- **pull1-2 (`shared_governor` `.ok()?` drops the governor on poison)**: CONFIRMED present at server.rs:5183-5190. It is latent: there is no panic source under `site.budgets` (rate.rs `reserve` uses `checked_add`/`saturating_sub`). L3 adds the inverted rationale.
- **telemetry (pass 1): "Poisoned `inner`/`last_error` recovered with `into_inner`; a panic inside `line()` leaves only a burned seq"**: CONFIRMED. sink.rs:1115-1131 increments `seq` and clears `buf` before `line()`, and `bytes` is updated only after `append` returns `Ok`.
- **store1/store2: "Verified-block Mutex ... set to NO_BLOCK before every fill ... recovering from poison is safe"**: CONFIRMED (file.rs:1832-1841, 1920-1922, 2367-2371).
- **apicache: "`lock().map_err("poisoned")` would wedge after a panic; no reachable panic under the lock"**: CONFIRMED for the static page caches. booleanjson.rs:189-213 sets `*cache = None` before reopening, so a failed open cannot leave a stale reader.
- **apicache: "single-flight `Landing` drop wakes followers on a panic"**: CONFIRMED for dev and test builds. In release a leader panic aborts the process, so it is moot (poison-1).
- **runs (pass 1): "TaskFinisher ... the knob guard (`Applied`) clears on unwind"**: CONFIRMED for dev and test builds only. In the release binary there is no unwind (poison-1).
- **runs (pass 1): "A chain panic becomes `note_dead_chain` plus `Retry`"**: REFUTED for the shipped binary. With `panic = "abort"` (Cargo.toml:185) a chain panic aborts the process, so `pullrun.rs:885`'s `Err(dead)` arm is reached only by cancellation. It is true only for `cargo run` without `--release` and for tests.
- **cli1 (pass 1): "On a panic, `Attempt::drop` writes `Failed` while unwinding and the lease guard unlocks"**: REFUTED for the shipped binary, for the same reason. In release a panic leaves the operation-audit record at `Started`/`Progress`, exactly like the SIGKILL case that pass 1 lists next to it (the kernel still releases the lease). It holds for dev builds.
- **runs-3 / recovery-5 (`site.run` held across `recovery_control::stop` fsyncs)**: CONFIRMED still present (server.rs:10984-10995 holds `held`, then calls `recovery_control::stop`, which takes `recovery_active` and calls `persist`).
- **hunt-conc (pass 0): "no production `.lock().unwrap()`"**: CONFIRMED. Every `.lock().unwrap()`/`.expect(` hit is under `#[cfg(test)]` (for example emitted.rs is a test-only module, and recovery_control.rs:342+ is in its test module). Production `Mutex::try_lock` (indexstop*json caches) maps both `WouldBlock` and `Poisoned` to a named busy refusal.

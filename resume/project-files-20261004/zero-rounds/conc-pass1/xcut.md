# conc-pass1 / xcut: 3 findings at 331b05c (0 high, 1 medium, 2 low)

Verdict: the atomic-publish helpers are mostly sound. Every rename-based publish syncs the file before the rename. Every durable one also syncs the parent directory after the rename. Every fixed temp name is written only while a flock is held. The cross-process defect that remains is the telemetry sink. The api server and the cli write the same `events.ndjson` whenever `BRUTEX_LOG_DIR` is set, and D-0301's own text tells the operator to set it "for both". Two smaller defects: the census reports "not published" after its rename already happened, and candidate-trades directories are created without syncing their new ancestors.

Slice: CROSS-CUTTING across the whole workspace. Every production `fs::rename`, `hard_link`, `create_new`, `File::create`/`truncate(true)`, temp-name scheme (`.tmp`, `.partial`, `.writing`, `.pending`, pid-stamped) and `sync_all`-around-publish. Test modules (`mod tests`, `*_tests.rs`, `tests/`) were excluded after checking each hit's position. Source reading only. No cargo was run, nothing was edited.

## Helper inventory (deduplicated, one row per helper)

| # | helper | file:line | temp name | file sync before publish | parent-dir fsync after | unique across procs/threads | orphan on restart | create_new vs truncate | api+cli same path? | verdict |
|---|---|---|---|---|---|---|---|---|---|---|
| H1 | census install (`install_locked`→`publish`) | pull/src/ingest.rs:3340-3366 | `<vendor>.man.writing` (fixed) | yes, `sync_all` | yes (3365) | serialised by the `CensusLock` flock on `<vendor>.man.lock` (ingest.rs:2990-3110); both api ingest and cli ingest take it | left behind, truncated by `File::create` on the next install, never listed (no `read_dir` over `manifest/`) | truncate under lock: safe | both, serialised by the lock | **xcut-2** (misreport after the rename). Lock-bypass arms are pull2-1 |
| H2 | census in-place append (`write_appends`) | pull/src/ingest.rs:3313-3330 | none (positional) | `sync_data` before the slot, `sync_all` after | n/a | same lock | n/a | n/a | same lock | sound for writers. Lock-free reader tear is pull2-5. **xcut-2** covers its error text too |
| H3 | masters `replace_locked` | pull/src/masters.rs:871-918 | `.<file>.partial` (fixed per source) | yes | yes, and a failure there is reported as `Landed::Uncertain`, not as a refusal | blocking `Flock` on `.<file>.lock` (masters.rs:841-852) | removed on every failure arm. A crash orphan is truncated by the next `truncate(true)` under the lock | safe under lock | both, serialised | clean |
| H4 | `cli::live::Live::publish` | cli/src/live.rs:368-392 | `<hex>.<pid>.tmp` | `sync_data` | **no**. Deliberate: "this file is not history" (live.rs:375-378) | pid-unique across processes. Same-thread reuse only, because one `Live` owns one identity path | crash orphans stay forever. The reader counts them as strays, not runs (live.rs:1553-1616) | `File::create` truncates its own pid name | api sweeps and cli sweeps share `results/live/` but are serialised by `execution_lease` (`.sweep-execution-v1.lock`, cli lib.rs:2101, api sweeprun.rs:1761), so two processes cannot publish one identity at once | clean (documented limits) |
| H5 | expression evidence (`EvidenceWriter`) | cli/src/expression.rs:218-345 | `expression-v1.pending` inside a per-attempt-token directory | yes | yes. Every ancestor is synced on create (expression.rs:146-170), and the dir is synced again after link and unlink | per-token directory from the sweep-evidence journal | a crash leaves a `.pending` in a dead token's directory. A rerun gets a new token | `create_new` plus a `hard_link` publish that never replaces anything | n/a | clean |
| H6 | sweep-evidence token reservation (`reserve_start`) | cli/src/sweep_evidence.rs:1045-1065, 507-560 | `<token>-start.bin` | `barrier` = `sync_all` | `flush_directory(base)` before and after | tokens come from the flock-appended `attempts.bin` | a torn start file affects only its own dead token | `create_new` | shared journal, serialised | clean |
| H7 | candidate detail `write_exact` | cli/src/candidate_trades.rs:1307-1349 | none (final name, `create_new`, then `Flock::lock`) | yes | leaf only | per `(identity, token)` directory | per-token, so a rerun is unaffected | `create_new`. A reader can try-lock-shared in the gap before the writer's lock and get a transient "truncated" or "busy" refusal | n/a | **xcut-3** (ancestors not synced) |
| H8 | search checkpoint `publish_inner` | cli/src/search_checkpoint.rs:215-285 | `<seq>/payload` plus `<seq>/complete` | yes | yes | owner flock plus the sequence | interrupted seq directories are counted as `interrupted` | `create_new` | n/a | torn `complete` marker is KNOWN (GAP11-0 / hunt-cli-b re-1). Still present at 331b05c (lines 262-266, 457-463) |
| H9 | Boolean persistence `write_or_equal` / owner.lock | cli/src/boolean_candidate_persistence.rs:96-125, 189-213 | none | yes | yes | owner flock | crash wedge = search-1 | `create_new` / compare-equal | n/a | reported by search-1 |
| H10 | operation audit per-invocation file | cli/src/operation_audit.rs:556-567 | `own(base,id)` | `write_synced` | yes | id from the flocked `index.bin` | — | `create_new` | api and cli share `index.bin` under a flock | clean (recovery slice covers the rest) |
| H11 | NSE cash-session cache | pull/src/cash_session_cache.rs:483-529 | none (payload, then receipt) | yes | yes | `try_lock` per day | a payload without a receipt is a permanent loud refusal = pull2-3 | `create_new` | api and cli refuse each other promptly (pull2-4) | reported |
| H12 | vendor capture | pull/src/capture.rs:224-276 | `<prefix>-p<pid>-t<stamp>-c<n>.txt` | yes | no (diagnostic) | pid, per-slot `fetch_update` seq, and 16 `create_new` attempts | torn-capture-at-final-name = pull1-3 | `create_new` | n/a | reported |
| H13 | store repair revision | store/src/repair.rs:327-397 | `.reserved-v1` reservation and `.repair-v1` receipt | yes | `sync_ancestors` before and after the receipt | `create_new` reservation | the reservation is never removed, by design. An interrupted revision number is burnt | `create_new` | no production caller | store2-1 |
| H14 | population / admission / finalization / execution `open_child` | e.g. cli/src/population_v5.rs:3203-3238, execution_v4.rs:4680-4712 | none (create-or-open journals) | per-journal | pop1-3 for Observation ledgers | root flock | pop1-1/2 | `create_new` then reopen without truncate | n/a | covered by pop1 |
| H15 | autopilot write probe | api/src/autopilot.rs:590-627 | `<STORE_PROBE_PREFIX><vendor>` (fixed) | yes | no (the file is removed again) | one autopilot task | a 19-byte orphan, truncated next time | `File::create` | api only | clean (autopilot slice) |
| H16 | telemetry roll (`Sink::roll`) and the append target | telemetry/src/sink.rs:1267-1303, 246-271, 732-758 | none (renames `events.ndjson` to `.1` ... `.N`) | append target `sync_all` on demand | **no** | **in-process only** (`OnceLock` in `telemetry::install`). No flock anywhere in the crate | — | `append(true).create(true)` | **YES whenever `BRUTEX_LOG_DIR` is set** | **xcut-1** |

## xcut-1 (medium): with `BRUTEX_LOG_DIR` set, the api server and every cli process append to and rotate one telemetry set with no cross-process lock

This extends telemetry-1 (this pass) and hunt-costs-3 (audit-20261003), which are both still present at 331b05c. Those reports scope the race to cli against cli. telemetry-1 says the cli doc's "each owns its own events.ndjson" claim "holds for api versus cli". It does not hold. Both resolvers return `BRUTEX_LOG_DIR` verbatim, and the project's own decision text tells the operator to set it for both.

Code:
- api: `crates/api/src/server.rs:18266-18278`
  ```rust
  fn log_dir_from(named: Option<std::ffi::OsString>, cwd: Option<&Path>, store_root: &Path) -> PathBuf {
      if let Some(named) = named {
          return PathBuf::from(named);
      }
  ```
- cli: `crates/cli/src/lib.rs:2990-2997`
  ```rust
  explicit
      .map(std::path::PathBuf::from)
      .or_else(|| store.map(|s| s.join("logs").join("cli")))
  ```
  Its own doc says "`BRUTEX_LOG_DIR` still wins outright and is used exactly as given" (lib.rs:2982-2983). Directly above (lib.rs:2959-2962) the same doc records that two live processes appending to one `events.ndjson` was "a corrupted record of the one thing that exists to say what happened".
- `docs/05-decisions.md:18914-18916`: success prints that "`/logs` reads a different one **unless `BRUTEX_LOG_DIR` is set for both**". That is the operator instruction that produces the shared directory.
- Sink: `crates/telemetry/src/lib.rs:169-183` refuses only a second sink in one process (`GLOBAL: OnceLock`). `Sink::open` (sink.rs:732-758) resumes `seq` and `reserved_run` from the newest record on disk, and `roll` (sink.rs:1267-1303) renames `events.ndjson` to `.1` with no lock. `crates/telemetry/Cargo.toml` has an empty `[dependencies]`, and no `lock`/`flock` call exists in `crates/telemetry/src`.

Why it is wrong: the api server is a long-lived process that rotates on its own byte count, and a cli sweep (`range-all`, hours) runs alongside it. That is the exact pairing lib.rs:2959 names as the reason the `cli/` subdirectory exists. Under `BRUTEX_LOG_DIR` the subdirectory is bypassed for both processes.

Repro (exact interleaving, two processes, `BRUTEX_LOG_DIR=/x` exported in the shell that starts both):
1. `api` starts. `Sink::open(/x)` resumes `seq = S` and `reserved_run = S` from `/x/events.ndjson`.
2. The operator runs `cli range-all ...`. `install_log` opens `/x` too and also resumes `seq = S`. Each process now numbers its own lines S+1, S+2, ... into the same file, so every `seq` value appears twice. `Tail::missing` (tail.rs:351-363) computes `span - records.len()` and saturates to 0, so a real dropped-event gap is hidden.
3. api's `inner.bytes` crosses `max_file_bytes`. Its `roll` renames `/x/events.ndjson` to `/x/events.1.ndjson` and reopens a fresh current file. The cli still holds the old descriptor, so its lines now land in `events.1.ndjson`, newer than lines already in the new current file. `tail`'s newest-first `since` walk stops at the first older record (D-1325's invariant that `ms` order is file order). `/logs` then omits the cli's later events for any `since` query. The `.1` file also grows past its byte bound.
4. The cli then rolls on its own counter. It deletes `.N`, shifts `.1` (its own live file) to `.2`, and renames api's current file to `.1`, while api keeps writing into what is now `.1`. Retention roughly halves because both processes roll the one set.
5. Restart api after the cli wrote last. `resume_point` takes the cli's final `seq`, which can be below api's last `seq` because the two numbered independently. api's `reserve_run_id` (sink.rs:1032-1039) then hands browser attempts (`server.rs:6953`, `claim_run(id)`) run ids that api's own earlier events in the rotated files still carry. That is the D-1326 regression the resume point exists to prevent.

Minimal fix: take a non-blocking exclusive `std::fs::File::try_lock` on a `<dir>/.sink.lock` inside `Sink::open` and hold it for the sink's lifetime. On `WouldBlock`, refuse loudly: api refuses at startup, and the cli prints its existing "events are NOT being recorded: ..." line. Alternatively, have the cli always append `cli/` (or `cli/<pid>`) beneath an explicit `BRUTEX_LOG_DIR`, and have `api::logs::cli_half` follow the same rule. Either fix also closes telemetry-1 and hunt-costs-3.

## xcut-2 (low): census install reports "not published" after its rename has already published, unlike the masters helper beside it

Code: `crates/pull/src/ingest.rs:3357-3366`
```rust
fn publish(dir: &Path, tmp: &Path, path: &Path, image: &[u8]) -> std::io::Result<()> {
    ...
    fs::rename(tmp, path)?;
    fs::File::open(dir)?.sync_all()
}
```
`install_locked` (3340-3351) turns any error here into "`{path}` could not be published through `{tmp}`". The caller (ingest.rs:936-945) records the failure "`{n}` slice(s) are on disk and the census that counts them was not published", and `note_census_unpublished` emits `pull.census` / `not published` at Error. The append path has the same shape: `write_appends`'s final `file.sync_all()?` (3328) fails after the slot bytes are already written and visible. Compare `pull/src/masters.rs:907-917`. That helper names exactly this state ("the new bytes are visible, but crash durability is UNVERIFIED") and returns `Landed::Uncertain` instead of a refusal.

Why it is wrong: after the rename, every reader (api census, autopilot `manifest_loads`, the next ingest) sees the new census. So the receipt and the Error event make a false statement about the store. The true state is "published, durability unconfirmed". This is §3 rule 6 (honest limits): the operator is told to recover from a census that is in fact present and counting the slices.

Repro: the process gets EIO (or EROFS after a remount) on `File::open(manifest_dir).sync_all()` at ingest.rs:3365, right after `fs::rename` at 3364 succeeds. The receipt says the census "was not published". A `GET` of the census page in the same second shows it Held with the new counts. The next ingest of the same folder finds every slice already counted, and nothing is appended.

Minimal fix: split `publish` so that an error after the rename returns a distinct `Uncertain(why)` variant, as `masters::replace_locked` does. Word the failure as "published; directory sync failed, crash durability UNVERIFIED". Apply the same split to `write_appends` after its slot write.

## xcut-3 (low): candidate-trades directories are created with `create_dir_all` and only the leaf is ever synced

Code: `crates/cli/src/candidate_trades.rs:321-322`
```rust
let directory = directory_for(root, &attempt.identity(), attempt.token(), model);
fs::create_dir_all(&directory).map_err(io_error)?;
```
This creates up to four new levels: `results/{candidate-trades-v1|expression-candidate-trades-v1}/<identity>/<token>`, per `directory_for` (1249-1257). The only directory barrier in the module is `File::open(path.parent())...sync_all()` in `write_exact` (1342-1344), which syncs the leaf `<token>` directory. The entries `<token>` in `<identity>`, `<identity>` in the namespace, and the namespace in `results/` are never synced. Same module's neighbour `expression::attempt_directory` (expression.rs:146-170) states the rule this breaks: "Syncing only the leaf would not persist newly created ancestors." `sweep_evidence::durable_directory` (1115-1130) does the same for its own tree.

Why it is wrong: the sweep-evidence lifecycle row that records the attempt `Completed` is synced (`barrier`) after these files. After a power loss on a filesystem that does not order a later fsync of another inode behind an earlier `mkdir`, the completion row can be durable while `<identity>/<token>` is gone. Every later candidate read of that completed attempt then refuses as missing or corrupt detail rather than showing it. This is the same class as pop1-3 at a new site.

Repro: run `audit-stored` (or `expression-backtest-stored`) for a new identity on a fresh store. Cut power after `attempt.finish(Completed)` returns, before the kernel's periodic writeback commits the parent directories. On restart the lifecycle says Completed, and `results/candidate-trades-v1/<identity>/` may be absent. Whether the entry survives depends on the filesystem's metadata ordering, not on anything this code guarantees.

Minimal fix: replace the `create_dir_all` with the `durable_directory`-style walk. Create each missing level with `create_dir`, then sync its parent, as `expression::attempt_directory` already does.

## Checked and clean / already reported (not counted)

- No production helper uses a temp name that two processes or two threads can both claim without a lock. Fixed names (H1, H3, H15) are always written under a flock or by a single task. Pid-stamped names (H4, H12) are pid-unique, and the in-process callers own distinct paths.
- Every rename-publish syncs file data before the rename (H1, H3, H4). H1 and H3 sync the parent directory after. H4 deliberately does not, and documents why.
- api against cli on result outputs: every sweep command on either side takes `execution_lease` on the canonical store (cli/src/lib.rs:2096-2103, api/src/sweeprun.rs:1756-1770), so `results/live`, the run ledger and the evidence trees are not written by both at once. Census, masters, cash cache, operation audit and sweep-evidence journals each take their own flock.
- Orphans are never "cleaned up on restart" anywhere. Each one is either truncated on the next use of its fixed name (H1, H3, H15), confined to a dead token or pid (H4, H5, H6, H7, H12), or deliberately left as a loud refusal (H11 pull2-3, H13 store2-1, H9 search-1).
- Already reported and still present at 331b05c: GAP11-0 (checkpoint `complete` marker), search-1, pull1-3, pull2-1, pull2-3, pull2-4, pull2-5, store2-1, pop1-3, telemetry-1, and hunt-costs-3.

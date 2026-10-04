# conc-pass4 / ledgers + locks: durable writers and non-blocking lock sites, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero):** 2 new findings (0 high, 1 medium, 1 low). All 6 pass-3 findings (ledgers-1/2/3, locks-1/2/3) were addressed by D-1908..D-1915. 5 are FIXED. ledgers-2 is PARTIAL, as D-1915 itself says. Of the 25 pass-1/pass-2/other ids the commits cite in this family, 20 are FIXED and 5 are PARTIAL: pop1-4, pop2-4, store1-2, ledgers-2 and hunt-cli-a-5. hunt-cli-a-5's fix introduced conc4-1. No claimed fix rolls back without the writer's exclusive lock, and none truncates another process's append.

Method: source only, read-only checkouts /home/claude/wt/zero (6104a4b) and /home/claude/wt/zero2 (1f4de71). No cargo was run. I read `git log 5140aca..1f4de71` and D-1900..D-1915 in `docs/05-decisions.md`, then the whole of `crates/cli/src/fixed_tail.rs`. I read every production call of `heal_torn_tail`, `discard_orphan` and `sync_or_roll_back`, and checked each for the lock it runs under. I read `append_rollback.rs`, the changes to `boolean_candidate_persistence.rs`, `index_stop_vix.rs`, `search_checkpoint.rs` and `selection_v6.rs`, `store/src/file.rs`'s barrier memory, `api` `detail::Checkout` / `serve_lock_refusal` / `observe_elsewhere`, `cli/src/ordered.rs` and the telemetry sink's new directory lock. I also checked `git diff 6104a4b 1f4de71` for writers and locks: the only new writer is Selection V6's quarantine (conc4-1), and the API now detaches the audited handlers of non-GET requests (sound). Kernel facts K1 and K2 are as in pass 3. K1 still holds: `cli::cancel` (40b1c2a, merged) has no caller of `cancel::request`, so no SIGINT handler exists and `STOP` is never set.

---

## New findings

### conc4-1 (medium): Selection V6's new abandoned-tail quarantine wedges the rung for good after a torn or ENOSPC quarantine write, or after a second abandonment at the same offset

**Where:** `crates/cli/src/selection_v6.rs:381-420` (`set_aside_abandoned_tail`), called from `persist` (:311-313 `found` branch and :334-342 foreign-prefix branch). Added by D-1569 (hunt-cli-a-5), merged in 1f4de71.

```rust
let aside = root.join(format!("{FILE_NAME}.abandoned-{committed}"));
match OpenOptions::new().write(true).create_new(true).open(&aside) {
    Ok(mut out) => {
        out.write_all(&tail)
            .and_then(|()| out.sync_all())
            .map_err(|why| format!("Selection V6 abandoned tail quarantine: {why}"))?;   // file left behind
    }
    Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
        let held = std::fs::read(&aside)...;
        if held != tail {
            return Err(format!("Selection V6 abandoned tail quarantine {} already holds different bytes; nothing repaired", ...));
```

**Why it is wrong.** The quarantine name depends only on the offset `committed`. Its content must equal the current tail exactly, or `persist` refuses before writing anything. Three ordinary paths leave a quarantine file that can never match:

- (a) `write_all` fails part way (ENOSPC/EDQUOT/EIO) or `sync_all` fails. The `?` returns, and the `create_new` file is left holding a prefix of the tail, or a whole tail whose barrier failed.
- (b) A kill or Ctrl-C inside `write_all` (K1). The tail can be up to `SELECTION_V6_BLOCK_BYTES-1` bytes and can cross a page.
- (c) Source A is killed mid-block at offset X. Source B sets A's tail aside to `abandoned-X`, cuts the file and starts its own block at X. B is then killed mid-block too. Source C now finds B's tail at X, `abandoned-X` holds A's bytes, and C refuses.

In every case each later `persist` on that rung refuses, whatever its source: the foreign-prefix branch refuses, and so does the `found` branch, which means even the exact rerun of a block already committed refuses. This is the permanent rung wedge hunt-cli-a-5 reported, moved from the ledger to its quarantine. The refusal names the file but does not say that removing it is the remedy. §4: a repair that can fail into a permanent refusal is not a recovery.

**Repro (not run).**
1. Fill the volume so that a write of the tail's length fails, for example a small loop-mounted ext4 holding the Selection V6 root.
2. Leave a foreign partial block: start a V6 persist for source A and `kill -9` it during `write_all`.
3. Run a V6 persist for source B. `create_new(abandoned-X)` succeeds, `write_all` returns ENOSPC, and `persist` returns `Err(".. quarantine: No space left on device")`. The partial `abandoned-X` stays.
4. Free space and rerun B. You get `AlreadyExists` → `held` (short) != `tail` → "already holds different bytes; nothing repaired". Every later run refuses the same way.

For (c), use two `kill -9`s at the same offset with no disk fault.

**Minimal fix.**
- Write the quarantine under a scratch name (`.abandoned-X.writing`), `sync_all` it, then `rename` it to a name keyed by the tail's content as well as its offset (`abandoned-X-<blake3(tail)>`), then sync the directory. That is the D-1909 pattern.
- On any write or sync error, `remove_file` the scratch.
- On `AlreadyExists` for the content-keyed name, the bytes are equal by construction, so the move proceeds.

### conc4-2 (low): Population V5, live in `ledger-all`, still confirms a failed row barrier with a second one. D-1900/D-1915 skip it, and D-1915's "still open" list leaves it out

**Where:** `crates/cli/src/population_v5.rs:2190-2217` (`append_locked`) and `:2220-2260` (`complete_trailing`). Its appends go through `append_rollback::append` (:3152-3153), whose module doc reads: "A failed durability barrier after a whole write is not this helper's case: the whole record is a valid orphan that each ledger's exact-retry rule already continues, so it is left in place" (`append_rollback.rs:12-14`). That is the doctrine D-1900 rejects.

```rust
self.append_row_suffix(&prepared.rows, 0)?;
self.row_file.sync_data().map_err(|why| format!("cannot sync Population V5 rows: {why}"))?;   // rows left on failure
...
// next process, exact retry:
self.append_row_suffix(&prepared.rows, prefix_len)?;      // appends 0 rows when the prefix is whole
self.row_file.sync_data()...                              // fresh descriptor: Ok under K2, writes nothing
self.append_completion(prepared, trailing.first_row_record)?;   // durable Completion over unsynced rows
```

**Why it is wrong.** pop1-4 already noted that "the same reasoning applies to V5 and V6 trailing-prefix reuse" (concurrency.md:1719). D-1900 routed Population V1 and V6 through `fixed_tail`, but not V5. D-1915's list of open sites names Lineage V4, Selection V6, Global Replay V4 and Admission V4, and does not name V5. V5 is written by every `ledger-all` run (`ledger_all.rs:1080-1100` → `all_rung_population_v5`). Admission V2 and Finalization V2, the other `append_rollback` users with that doctrine, have no production caller (grep found none), so they are not reported.

**Repro (not run).**
1. `ledger-all` reaches the V5 append for rung R, and the rows `sync_data` returns EIO. The run fails.
2. Rerun it unchanged. `scan` finds a whole trailing block, exact for this identity. `complete_trailing` appends nothing; `sync_data` returns `Ok` on the new descriptor (K2); the Completion is written and synced.
3. After eviction or a reboot, the rows under that Completion read back stale, and `validate_complete_block` refuses on every open.

**Minimal fix.** Route V5's rows and Completion through `fixed_tail::append_block` / `sync_or_roll_back`, as V6 does. At the same time, change `append_rollback`'s module doc so that it no longer endorses leaving a failed-barrier orphan, or retire the helper.

---

## Verification table

Head 1f4de71. "Fix" is the commit or decision that claims the id.

| id | claimed by | verdict | evidence |
|---|---|---|---|
| ledgers-1 | D-1908, 38280a9 | **FIXED** | `index_stop_vix.rs:410`: only `persistence::lost_owner_race(&why)` (prefix `OWNER_REFUSED`, `boolean_candidate_persistence.rs:155-161`) maps to `Ok(Some)`. A failed `finish` propagates. Residual: the race answer can still be refused transiently while the winner holds its owner lock (pass-3 note, unchanged). |
| ledgers-2 | D-1915, 281cbb4 | **PARTIAL** | Fixed: Base Evidence V2 (`population_base_evidence_ledger_v2.rs` `sync_or_roll_back` on records and completion; orphan cut and rewritten; reuse refuses a failed path) and Candidate Universe (`cut_orphan_for_rewrite`, `sync_or_roll_back`). Boolean `write_or_equal` withdraws a failed created file (:243-262), and its reuse arm now syncs (:267-272; sound only because creators now withdraw on failure). Open by D-1915's own admission: Lineage V4, Selection V6 (`selection_v6.rs:311-317` `found` → `sync_all` → `Ok(false)`; a partial ≥ SEAL_AT prefix is resumed after `sync_all`), Global Replay V4, Admission V4. Also open and not admitted: Population V5 (conc4-2). Base Evidence/Candidate reuse refuse only in-process. Across processes, the bytes a failed barrier left are cut by the rollback, so this holds unless `set_len` also failed (a double fault, named in the refusal). |
| ledgers-3 | D-1910, bd22a9a | **FIXED** | `heal_torn_tail` is called in the writable open of all seven listed ledgers, each under its exclusive lock: Pre-Admission V1/V2 (`ensure_header`, under `Flock::lock` in `open`), Candidate Universe and Base Evidence (`ensure_header` under `Flock::lock`), Execution V4 (`execution_v4.rs` writable branch after `lock_file.lock()`), Observation V1/V2 (`population_observations_v1.rs:2190`, `:3413`), Lineage V4 (`anchored_search_lineage_v4.rs:747`, member stride 768). Each heal runs after the header is verified. Readers still refuse. |
| locks-1 | D-1911, 9ba9110 | **FIXED** | `server.rs:18311` `serve_lock_refusal` matches `WouldBlock` (stamp quoted) versus `Error(host)` (host refusal; no instance implied). |
| locks-2 | D-1912, 66c7632 | **FIXED** | `detail.rs:793-835` `Checkout` takes the slot under a lock held only for `Option::take`/put-back. All five handlers use it (`indexstopvixjson.rs:130`, `indexstopjson.rs:131`, `indexstopcandlesjson.rs:159`, `indexstopqualificationjson.rs:150`, `indexstoprankingjson.rs:185`). A late put-back of an older entry costs only a cold reopen, because the cache key (pin/checkpoint) is compared on every use. |
| locks-3 | D-1913, 46f2c76 | **FIXED** | `search_checkpoint.rs:272-281`: `file.release()` comes after the payload sync, the first verification and the directory fsync, and before `complete.writing` is created. |
| CE-3 | D-1909, 78a216d | **FIXED** | `complete.writing` → `sync_all` → `rename` → dir fsync (`search_checkpoint.rs:286-300`). Discovery treats a short legacy `complete` as interrupted. |
| CE-11 | D-1914, 80b389c | **FIXED** | `sweeprun.rs:2650` `observe_elsewhere` re-walks with limit `index+1` and keeps that walk only if `records[index].seq` is unchanged between the walks, which closes the race. |
| cli2-1 | D-1900 | **FIXED** | `sweep_evidence.rs` `append_row`, `append_events` and `ranked` hold the flock across `start` / `write_at_end` / `sync_or_roll_back`. A first-append failure cuts to 0, and readers count an empty file as no rows. |
| pop1-2 | D-1900 | **FIXED** | Population V6 (`population_v6.rs:2176/2186` `append_block`) and Observation V1/V2 (`:2294/2319/3490/3502`). |
| sel-1 | D-1900 | **FIXED** | Execution V4 `append_block` (`execution_v4.rs:2930-2992`), under the open's exclusive lock. |
| search-2 | D-1900 | **FIXED** | Pre-Admission V1/V2 `append_block` (`pre_admission_data.rs:3608`), under `append_complete`'s `lock_file.lock()`. |
| slice24-F1 | D-1900 | **FIXED** | Finalization V3/V4 and Statistics V2: `start`/`sync_or_roll_back` pairs at the listed lines, inside `append_locked`. |
| resources-1 | D-1900 | **FIXED** | runs/frontier/trades/detail-sets roll back on a failed barrier. `results.rs:1292-1312` `confirm_durable` calls `refuse_after_failed_barrier` before its sync. Residual: only a double fault (the rollback's own `set_len` failing) leaves bytes a later process could confirm. |
| pop1-4 | D-1900 | **PARTIAL** | Population V1 and V6 are fixed. Population V5 `complete_trailing` (`population_v5.rs:2220-2253`) still confirms by a second barrier → conc4-2. |
| sweep-2 | D-1901 | **FIXED** | Writer-side `heal_torn_tail` under the exclusive lock: `results.rs:890`, `frontier.rs:945/1103`, `trades.rs:523/659`, `result_set.rs:329`, `sweep_evidence.rs` `heal_torn`. Frontier and trades now take the lock in `open`. |
| cli3-3 | D-1901 | **FIXED** | `admission_store.rs:638`, `institutional_statistics.rs:1000` and `stored_data_completeness.rs:779` each heal under the writer lock (all three modules are dead in production). |
| pop2-5, slice24-F2 | D-1903 | **FIXED** (call-site evidence; not exercised) | `sync_parent` / `sync_observation_root` after a new header (`population_observations_v1.rs` "THE NAMES ARE DURABLE TOO"). |
| slice24-F3 | D-1904 | **FIXED** (call-site evidence) | Scan refuses a trailing orphan that repeats a completed identity. |
| pop2-4 | D-1905 | **PARTIAL** | `discard_orphan` is used, under each `append_locked`, in Statistics V2/V3, Finalization V4, Admission V3/V4 and Population V1. Not applied to the other `ledger-all` writers pass 2 named for the same rule: Execution V3 (`execution_v3.rs:2332` "orphan tail is not an exact retry prefix"), Selection V5 (`selection_v5.rs:2168`) and Population V5 (`population_v5.rs:2236`). Nor to Candidate Universe and Base Evidence (`cut_orphan_for_rewrite` / `append_locked` still refuse "belongs to another source" → ledgerall-1 NOT FIXED). |
| pop2-1 / slice23-F1, pop1-1 | D-1906 | **FIXED** (call-site evidence) | Admission V3 `discard_trailing` / whole-row-prefix completion (`population_admission_v3.rs:3991-4180`). Population V1 `discard_unreceipted_trailing_block` (`population.rs:3154/3232/3365`). |
| store1-2 | D-1907 | **PARTIAL** | `store/src/file.rs:3621-3655`: `FAILED_BARRIERS` refuses any later `append` on the path in this process (:2197). The header slot whose barrier failed is not rolled back, so (a) readers in the same process and in other processes (`open_existing`) take the page-cache header as a commit, and (b) after an api restart the set is empty, so the next append re-verifies the tail from the page cache and commits behind a second barrier (K2). Rolling back to the previous slot, or invalidating the failed slot and syncing, would close it. |
| hunt-conc-1, hunt-conc-2 | D-1556/D-1564 | **FIXED** | `batch.rs:372-392`: begin and file run serially in walk order, and only the per-attempt sweep runs in parallel. `ordered.rs` lanes (OS threads, window 8): `turn()` is taken before `WRITER`/`LEDGER`/the cross-process lock (`results.rs:797`, `lib.rs:20693`, `sweep_evidence.rs:844/1455`). Pool pass 2 `price_all` writes nothing. A deadlock would need a blocking lock held between turns, and I found none (owner locks are `try_lock`). |
| hunt-cli-a-5 | D-1569 | **PARTIAL** (fix introduces conc4-1) | The foreign unsealed tail is moved aside under the exclusive lock, and `require_committed` no longer refuses committed blocks behind it. The quarantine can wedge (conc4-1). |

**Lock-family ids not claimed and still open at 1f4de71:**

| id(s) | status at 1f4de71 |
|---|---|
| barflow-1 | `store/src/file.rs:988` still says "another writer holds" when the holder is a reader |
| store1-1, store2-1 | not claimed |
| pull2-1, pull2-4, equity-2 | `ingest.rs` and `cash_session_cache.rs` untouched |
| server1-1, recovery-4, recauto-2 | `api/src/audit.rs` untouched |
| cli1-2, log-1 | `operation_audit.rs:522/599` unchanged apart from a flag constant |
| cli1-3, runs-2 | `execution_lease.rs:66` unchanged; only the hard-link refusal text changed |
| expr-1 | `search_checkpoint.rs:109/155` |
| expr-2, indexstop-2(a)(b) | `boolean_candidate_persistence.rs:110` |
| replay-3, replay-5, cli2-5 | untouched |

---

## Coverage table A: durable writers (pass-3 rows updated, plus rows added since 5140aca)

RB = rolls back a failed write. Sync = a failed barrier can later be confirmed as success.

| # | writer | RB | kill-tail on writable open | failed sync later confirmed? | status at 1f4de71 |
|---|---|---|---|---|---|
| 1-4 | api audit journal, recovery journal, STOP control, operation audit | aligned / poison | refuse (documented) | no | unchanged, clean |
| 5 | sweep-evidence attempts/starts/lifecycle/levels/ranked | **yes** (fixed_tail, under flock) | **healed** (header and tail) | **no** (cut) | cli2-1, sweep-2 FIXED |
| 6, 11, 12, 13 | reserve_start, candidate-trades, expression evidence, checkpoint journal | n/a | per token / scratch name | no | checkpoint marker now renamed (CE-3 FIXED) |
| 7-10 | runs.bin, frontier.bin, chosen-trades.bin, detail-sets.bin | **yes** | **healed** | **no** (rollback plus in-process refuse) | resources-1, sweep-2 FIXED |
| 14 | Boolean/index-stop evidence | create_new; withdrawn on failure | recovered (D-1760) | reuse syncs (ok now that creators withdraw) | ledgers-2 FIXED for this row |
| 15 | VIX companion publish | via 14 | via 14 | **no** | ledgers-1 FIXED |
| 16 | checksum receipts | n/a | n/a | yes | replay-3 open |
| 17 | Candidate Universe | yes | **healed** | **no** | ledgers-2/3 FIXED; foreign orphan still wedges (ledgerall-1 open) |
| 18 | Base Evidence V2 | yes | **healed** | **no** | as 17 |
| 19 | Pre-Admission V1/V2 | **yes** | **healed** | no (rolled back) | search-2, ledgers-3 FIXED; trailing foreign orphan still refuses |
| 20 | Statistics V3 | **yes** | refuse | no | foreign orphan discarded (D-1905) |
| 21 | Observation V1/V2 | **yes** | **healed** | no | pop1-2, ledgers-3 FIXED |
| 22 | Search Lineage V4 | yes | **healed** | **yes** (admitted open) | ledgers-2 open |
| 23 | Admission V4 | yes | refuse | **yes** on reuse/retry (admitted open) | ledgers-2 open |
| 24 | Finalization V4 | **yes** | refuse | no | — |
| 25 | Population V6 | **yes** | refuse | no | pop1-2 FIXED |
| 26 | Execution V4 | **yes** | **healed** | no | sel-1, ledgers-3 FIXED |
| 27 | Selection V6 | no (prefix resume) | **foreign tail moved aside** | **yes** (admitted open) | **conc4-1** (quarantine wedge) |
| 28 | Global Replay V4 | byte-prefix | resumed | yes (admitted open) | ledgers-2 open |
| 29 | store bar month | header-slot commit | tail past n_valid | **in-process no; cross-process/readers yes** | store1-2 PARTIAL |
| 30-34 | repair revision, census, cash-session cache, capture, telemetry sink | as pass 3 | as pass 3 | as pass 3 | unchanged; the sink now also takes `events.lock` (table B row 35) |
| 35 | Population V5 (live, ledger-all) | append_rollback (write only) | 4096/1024-byte aligned, so K1 cannot tear a record | **yes** | **conc4-2**; foreign trailing still refuses (pop2-4 PARTIAL) |
| 35b | Population V1; Admission V3; Finalization V3; Statistics V2 | **yes** | refuse | no | D-1900/D-1905/D-1906 |
| 35c | Execution V3 (live) | **yes** (append_block) | refuse | no | foreign orphan refuses (pop2-4 PARTIAL) |
| 36-37 | dead modules (admission_store, institutional, completeness; selection v1-v5, Admission V2, Finalization V2, ...) | mixed | heal (36) | — | no production caller |
| new | Selection V6 quarantine `selection-v6.bin.abandoned-<off>` | **no** | n/a | n/a | **conc4-1** |

## Coverage table B: non-blocking lock sites (pass-3 rows 1-33, plus rows added since 5140aca)

Rows 1-7, 9-11 and 17-33 are unchanged from pass 3. Each prior finding still applies; see the unclaimed-ids table above.

| # | site | change since pass 3 | verdict |
|---|---|---|---|
| 8 | api `take_serve_lock` (`server.rs:18193`) | `serve_lock_refusal` splits WouldBlock from host error | locks-1 FIXED |
| 12-16 | five index-stop JSON caches | `Mutex::try_lock` replaced by `detail::Checkout` (blocking lock held only to move the `Option`) | locks-2 FIXED; no try_lock remains |
| 21-23 | search_checkpoint owner / probe / payload | payload lock released before the marker; marker renamed into place | locks-3 FIXED; expr-1 (rows 21/22) open |
| 24 | boolean `prepare_in_namespace` owner try_lock (`:110`) | refusal now prefixed `OWNER_REFUSED` for `lost_owner_race` | expr-2 / indexstop-2(a,b) open |
| 34 (new) | `cli::ordered::turn` (Mutex + Condvar) | new (D-1556); deadlock-free by (k,i) order; taken before every lock it guards | clean |
| 35 (new) | telemetry `hold_directory` (`sink.rs:1508`), `events.lock` EX try_lock for the sink's life | new (D-1537) | Splits WouldBlock from host error. The api (`logs/`) and the cli (`logs/cli/`) use separate directories, so they do not collide. A second concurrent `cli` command runs unrecorded and says so. Documented by D-1537, not counted as a finding. |
| 36 (new) | `store::file` `FAILED_BARRIERS` and `cli::fixed_tail` `FAILED_BARRIERS` (process Mutex<BTreeSet>) | new | Poison is recovered with `into_inner`; no I/O happens under the lock. Keyed by the path as spelled, not canonicalised (low impact: one store root per process). |

## Checked and clean
- `fixed_tail::roll_back`, `heal_torn_tail`, `heal_torn_header` and `discard_orphan`: every production caller runs them under that ledger's exclusive flock, taken in `open` or in `append_locked`, and every appender to those files takes the same flock. So no rollback or cut can truncate another process's live append.
- Rollback ordering: `set_len(end)` then `sync_all`. The committed prefix was synced by an earlier successful barrier, so a failed writeback of the boundary page cannot lose committed bytes.
- `write_or_equal`'s withdrawal runs under the namespace owner lock, and readers reach `complete.bin` only through a shared lease on `owner.lock`. A concurrent reader therefore cannot see a 112-byte receipt between its write and its withdrawal.
- `api` now runs audited non-GET handlers in `tokio::spawn`, so a client disconnect no longer cancels the handler between its begin record and its end record.

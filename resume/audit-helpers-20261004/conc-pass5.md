# conc-pass5: side files and failed write barriers, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero):** 2 new findings (0 high, 0 medium, 2 low). No side file is ever trusted half-written. Every scratch name is either per token, pid or sequence, or it is truncated under a lock. The open defects are the ones already on record (conc4-1, pull1-3, pull2-3, store2-1, replay-3, xcut-2, xcut-3, cli2-5), plus two new ones:
- **conc5-1:** a fresh ledger header whose barrier fails, or that a power cut zero-fills, is not rolled back or remembered. Every later open refuses that empty ledger for good.
- **conc5-2:** Selection V5, which `ledger-all` writes on every run, confirms a failed row or Completion barrier with a second barrier. It is the sibling of conc4-2, and neither D-1915 nor conc4-2 names it.

Method: I read the source in the read-only checkout /home/claude/wt/zero3 at 1f4de71. No cargo was run, because neither finding is high. I listed every production `create_new`, `rename`, `File::create`, `truncate(true)`, `remove_file`, `hard_link`, `sync_all` and `sync_data` in `crates/*/src`, excluding `#[cfg(test)]` modules and `*_tests.rs` by position. I checked the result against the xcut helper inventory (H1-H16, concurrency.md:1993-2016), conc-pass4/ledgers-locks.md and crash-edge passes 1-6. I then read `git diff 331b05c 1f4de71` for new side files. The only new ones are CE-3's `complete.writing`, Selection V6's quarantine (conc4-1), `write_or_equal`'s withdrawal, `fixed_tail`, and the telemetry `events.lock`. K1 (no SIGINT handler, so a kill is fatal mid-`write_all`) and K2 (a barrier on a descriptor opened after a writeback error returns Ok) are as in pass 3 and pass 4.

---

## Theme 1 coverage: side files

| site | name rule | crash leftover handling | verdict |
|---|---|---|---|
| census `<vendor>.man.writing` (`pull/src/ingest.rs:3345-3366`) | fixed, under `CensusLock` | `File::create` truncates it on the next install; nothing lists it | clean. xcut-2 (Err reported after the rename has published) still NOT FIXED at :3364-3365 |
| masters `.<file>.partial` (`pull/src/masters.rs:871-918`) | fixed per source, under a blocking flock | removed on every failure arm; a crash orphan is truncated under the lock | clean; a failed dir fsync is reported as `Landed::Uncertain` |
| vendor capture (`pull/src/capture.rs:236-270`) | pid + stamp + `create_new`, 16 tries | a write/sync failure removes the file. A kill mid-write leaves a torn file under the final name | pull1-3, NOT FIXED |
| cash-session payload + receipt (`pull/src/cash_session_cache.rs:483-529`) | final names, `create_new`, per-day `try_lock` | a payload with no receipt refuses for good. A receipt whose sync failed is kept and later read as valid | pull2-3, NOT FIXED (the receipt half is the same class) |
| store bar month genesis (`store/src/file.rs:1424-1490`, `initialise` :3205-3251) | final name, under the month lock | an all-zero or torn genesis is re-initialised (D-1520/1521); a sealed sidecar refuses | clean. A failed genesis sync is safe because an evicted page rereads as all-zero and is repaired |
| store `.crc` sidecar (`file.rs:1906-1931`) | created only when `n_valid == 0`; dir fsynced | a missing sidecar on a sealed month refuses (D-0688) | clean |
| store repair `.reserved-v1` / `.repair-v1` (`store/src/repair.rs:351-397`) | `create_new` reservation | never removed; the revision is burnt | store2-1, no production caller |
| `cli::live` `<hex>.<pid>.tmp` (`live.rs:375-399`) | pid-unique, serialised by the execution lease | removed on failure; crash orphans are counted as strays | clean (documented, "not history") |
| expression evidence `expression-v1.pending` (`expression.rs:260-345`) | per attempt token directory | dead-token scratch, never reused | clean |
| sweep-evidence `<token>-start.bin` (`sweep_evidence.rs:1060-1078`) | token from the flocked, durable `attempts.bin` | a failed barrier leaves a dead token's file; no later attempt reuses the token | clean |
| candidate detail files and `catalog.bin` (`candidate_trades.rs:1307-1349`) | final name, `create_new`, per `(identity, token)` | a failure poisons the capture (`refuse`); a rerun gets a new token | clean for crash and barrier. cli2-5 (reader gap before the lock) and xcut-3 (`create_dir_all` at :322, ancestors unsynced) NOT FIXED |
| search checkpoint `<seq>/payload`, `complete.writing` → `complete` (`search_checkpoint.rs:240-310`) | sequence under the owner flock; `create_new` in a fresh dir | a `.writing` or marker-less sequence counts as interrupted; the writer poisons on any Err | CE-3 FIXED. If the dir fsync after the rename fails, publish returns Err but the marker is visible. Next process: content-verified, so not a defect |
| Boolean / index-stop `owner.lock`, `body.bin`, `complete.bin` (`boolean_candidate_persistence.rs:96-275`) | per identity, owner `try_lock` | uncommitted (no 112-byte receipt) scratch is discarded under the owner lock (D-1760). A failed write or barrier withdraws the file (D-1915) | clean; the reuse arm now syncs |
| checksum receipts and bindings (`checksum_receipts.rs:306-361`) | per identity | an exact prefix is resumed under `try_lock` | replay-3 NOT FIXED: the unlocked full-length fast path :307-311 still syncs nothing |
| Selection V6 `selection-v6.bin.abandoned-<off>` (`selection_v6.rs:381-420`) | offset only, `create_new` | left on a write or sync failure; a mismatch refuses forever | conc4-1, NOT FIXED |
| operation audit `index.bin` + per-invocation file (`operation_audit.rs:509-574`) | id from the flocked index; `create_new` | a torn index refuses (`length` :304-311). An index row whose sync failed burns that id | clean for this theme. The 256-byte rows are page-aligned, so a kill cannot tear them |
| recovery plan / `attempts.bin` / active journals (`api/src/recovery.rs:455-790`, `recovery_journal.rs:299-340`) | `create_new` or `open_existing` by presence | an empty `attempts.bin` from a crash reopens as an existing journal | clean (pass-3 rows 1-4) |
| autopilot write probe (`api/src/autopilot.rs:620-630`) | fixed name, one task | truncated next time, then removed | clean |
| telemetry roll `events.N.ndjson` and `events.lock` (`telemetry/src/sink.rs:1333-1410`, `:1508-1528`) | per directory, held by an EX `try_lock` | a torn last line is terminated on open (`terminate_torn_tail`). The lock file is permanent and harmless | xcut-1 FIXED by D-1537. A roll that fails part-way can drop one extra rotated file on the next roll (telemetry only, not reported) |
| ledger lock files (`.lock`, `owner.lock`, `<vendor>.man.lock`, cash `.<day>.lock`) | fixed, `create(true)`, never truncated | permanent by design | clean |
| **fresh ledger headers** (results, frontier, trades, result_set, sweep-evidence, Candidate, Base V2, Pre-Admission V1/V2, Population V1/V6, Admission V4, Finalization V4, Observation V1/V2, Statistics V2/V3) | final name, written into a zero-length file under the ledger lock | a strict-prefix tear is healed only by sweep-evidence (`heal_torn_header`) and Admission/Finalization V4 (`holds_torn_header`). A full-length header that never passed its barrier is refused everywhere | **conc5-1** |

## Theme 2 coverage: failed write barriers

RB = the failed bytes are rolled back. Mem = the failure is remembered in-process (`FAILED_BARRIERS`). X-proc = a later process trusts the unsynced bytes.

| site | RB | Mem | X-proc trusts? | verdict |
|---|---|---|---|---|
| `fixed_tail::sync_or_roll_back` / `append_block` users (sweep-evidence, runs/frontier/trades/detail-sets, Candidate, Base V2, Pre-Admission, Population V1/V6, Observation, Admission V3/V4, Finalization V3/V4, Statistics V2/V3, Execution V3/V4) | yes (`set_len` + `sync_all`) | yes | no. Only a double fault (rollback also fails, named in the refusal) leaves bytes | clean |
| store bar month append `barrier` (`store/src/file.rs:3626-3655`) | no (header slot kept) | yes | yes | store1-2 PARTIAL, unchanged |
| store genesis `initialise` sync (`file.rs:3250`) | no | no | the header is deterministic, and a page evicted before the first commit rereads all-zero and is re-initialised | clean |
| Population V5 rows / Completion | no | no | yes | conc4-2, NOT FIXED |
| **Selection V5 rows / Completion** (`selection_v5.rs:2107-2110`, `:2150-2155`, `:2226-2229`) | no (write errors only, `append_with_rollback` :3035-3054) | no | **yes** | **conc5-2** |
| Selection V6, Lineage V4, Global Replay V4, Admission V4 reuse | partial | partial | yes | ledgers-2 PARTIAL (admitted open by D-1915) |
| Boolean `write_or_equal` | withdrawn | n/a | no | clean (D-1915) |
| checksum receipts | no | no | yes | replay-3 |
| cash-session receipt `write_new` (`cash_session_cache.rs:528-546`) | no ("retained") | no | yes, `read_entry` takes the pair | the receipt half of pull2-3 (same fix) |
| census append `write_appends` final `sync_all` | slot left | no | yes, until the next commit overwrites it | xcut-2 / pull2-5 class, known |
| search checkpoint payload / marker | the reservation is abandoned | writer poisoned | no: marker-less sequences are interrupted | clean |
| candidate detail `write_exact` | no (the file is left) | capture poisoned | no: per-token, never reused | clean |
| operation audit `write_synced` | no | the Attempt latches `failure` | the reader `read()` syncs and reports (cli1-2/log-1 area) | not new |
| masters / census rename-publish | the temp is removed | n/a | no: the target is not replaced | clean |
| **fresh ledger header write + barrier** (sites in conc5-1) | **no** | **no** | header bytes are deterministic, so a page-cache read is correct. After eviction or power loss the block reads zero and every open refuses | **conc5-1** |

---

## New findings

### conc5-1 (low): a fresh cli ledger header that never passed its barrier is neither rolled back nor remembered, and a zero-filled one wedges the empty ledger for good

**Where.** Every fresh-header writer in `crates/cli/src` writes the header into a zero-length file and syncs it. On failure it returns the error and leaves the header in place. Representative sites:
- `population_v6.rs:1966-1969`
- `pre_admission_data.rs:3525-3531` (V1) and `:3760-3766` (V2)
- `candidate_universe.rs:6132-6137`
- `population_base_evidence_ledger_v2.rs:1672-1677`
- `population_observations_v1.rs:2178-2181`
- `population_admission_v4.rs:2577-2582`
- `population_finalization_v4.rs:2187-2192`
- `population.rs:4943-4950`
- `population_statistics_v2.rs:5456`, `population_statistics_v3.rs:2877`
- `results.rs:656-711`, `frontier.rs:1809-1812`, `result_set.rs:1049-1056`
- `sweep_evidence.rs:1209-1211`

```rust
// population_v6.rs:1961
if writable && empty {
    ...
    data_file
        .write_all(&header())
        .and_then(|()| data_file.sync_all())
        .map_err(|why| format!("cannot initialize Population V6 header: {why}"))?;   // header left, not remembered
```
```rust
// pre_admission_data.rs:3525 (Candidate Universe and Base V2 are the same shape)
if len == 0 {
    ... file.write_all(&bytes).and_then(|()| file.sync_data()) ...?;
    return Ok(());
}
verify_header(file, path)?;    // any non-zero length: magic/version or refuse, readers and the writer alike
```

**Why it is wrong.**
- The header is the one write in each of these ledgers that no `fixed_tail` rollback covers, and no call reaches `remember_failed_barrier`. D-1900 and D-1907 apply that rule to rows and slots only.
- After a header sync EIO the page is clean but never reached the device. If it is evicted before the first record write re-dirties page 0, or the machine loses power, the 64 or 16 bytes read back as zeros at full header length. The same state follows a power cut between the header write and its sync on a filesystem that extends the size before the data (XFS, ext4 `data=writeback`), with no I/O error at all.
- Each ledger refuses that state on every open, writer included. "len == 0" is the only re-initialise condition. `heal_torn_header` (sweep-evidence only) and `holds_torn_header` (Admission V4 and Finalization V4) accept only a strict prefix of the header, so a full-length zero header fails both.
- The file provably holds nothing: it is exactly header length and no record follows. Re-initialising it therefore destroys nothing. The store already applies this rule to its own genesis (`store/src/file.rs:1424-1490`, `is_interrupted_genesis`, D-1520/D-1521). The cli ledgers do not.
- §4: a state the writer's own failure can produce becomes a permanent refusal, with no stated remedy. For the store-root ledgers (Candidate Universe, Base Evidence V2, Pre-Admission), that refusal blocks every `ledger-all` / `ledger-v6` run on the store (ledgers-3's blast radius).

**Repro (not run).**
1. On a fresh root, use dm-flakey (or `dmsetup message ... error_writes`) to fail writes during the first `cli ledger-all ...` open of the Pre-Admission ledger. `sync_data` returns EIO, and the run fails with "cannot initialize ...".
2. Drop caches (`echo 3 > /proc/sys/vm/drop_caches`) or reboot. Restore the device.
3. Rerun. The open finds a header-length file of zeros, and `verify_header` refuses ("wrong magic" / incompatible header). Every later run refuses the same way until the operator deletes the file by hand.

**Minimal fix.** Add one shared helper in `fixed_tail` and use it at every site:
- On a header write or sync error, `set_len(0)` + `sync_all`, then `remember_failed_barrier(path)`.
- In each writer-side open, treat "length is exactly the header length, and every byte is zero" as an interrupted header: cut it to 0 and re-initialise, under the ledger's exclusive lock, as the store's `is_interrupted_genesis` does. Readers keep refusing.

### conc5-2 (low): Selection V5, live in `ledger-all`, keeps rows or a Completion whose barrier failed and confirms them with a second barrier

**Where.** `crates/cli/src/selection_v5.rs`. Reached on every run through `ledger_all.rs:1218` → `all_rung_selection_v5` (live per concurrency.md:2120 / crash-edge.md:229).
- `append_locked`, :2104-2110: rows, then `sync_data`, with the rows left on failure.
- `append_completion`, :2221-2229: the Completion, then `sync_data`, with the Completion left on failure.
- `reuse_existing`, :2149-2155: a second `sync_data` on reuse.
- `append_with_rollback`, :3035-3054, rolls back write errors only.
- The module has no `fixed_tail` call and no `refuse_after_failed_barrier`.

```rust
self.append_row_suffix(prepared, trailing.rows.len())?;
self.rows.file.sync_data()
    .map_err(|why| format!("cannot sync Selection V5 winner rows: {why}"))?;      // rows left
...
// reuse_existing
self.rows.file.sync_data()
    .and_then(|()| self.completions.file.sync_data())
    .map_err(|why| format!("cannot sync reused Selection V5 authority: {why}"))?;  // Ok on a fresh fd (K2)
Ok(SelectionV5StructuralCommit::Reused(existing))
```

**Why it is wrong.** This is conc4-2's defect in the next stage of the same chain. D-1900 rejects the doctrine "a whole record whose barrier failed is a valid orphan the exact retry continues". Two paths break it:
- (a) The rows' `sync_data` fails. The next process finds them as an exact trailing prefix, appends 0 rows, gets `sync_data` Ok on its new descriptor, and writes a durable Completion over rows that never reached the disk.
- (b) The Completion's `sync_data` fails. The next process indexes that Completion as a receipt, `reuse_existing` re-syncs on a fresh descriptor, and reports `Reused`.

Either way `ledger-all` prints the rung as committed. After eviction or a reboot, the rows or Completion read back stale. The scan then refuses that root on every open, and Global Replay's successor check fails behind it. D-1915's open list (Lineage V4, Selection V6, Global Replay V4, Admission V4) leaves it out, and so does conc4-2, which names only Population V5. Execution V3, between them in the chain, is fixed: it uses `append_block` → `sync_or_roll_back`.

**Repro (not run).**
1. `cli ledger-all V 2025 1 2025 12 S P /x/ledgers` reaches the Selection V5 commit for rung R. The rows' `sync_data` returns EIO (dm-flakey), and the run fails "cannot sync Selection V5 winner rows".
2. Rerun unchanged. `scan` finds a whole trailing prefix that exactly matches this selection. 0 rows are appended, `sync_data` returns Ok (K2), the Completion is written and synced, and the result is `Written`.
3. Drop caches or reboot. Stale rows sit under a durable Completion, and every later open of R's Selection V5 root refuses.

**Minimal fix.** Route the rows and the Completion through `fixed_tail::append_block` (or `start` / `write_at_end` / `sync_or_roll_back`), as Execution V3 does. Call `refuse_after_failed_barrier` before the zero-suffix and reuse re-syncs. Fix it together with conc4-2 (Population V5), since both are stages of one `ledger-all` chain.

---

## Verification (head unchanged from pass 4)

| id | verdict | evidence |
|---|---|---|
| conc4-1 | NOT FIXED | `selection_v6.rs:392-410`: offset-only name, `?` leaves the partial file |
| conc4-2 | NOT FIXED | `population_v5.rs` `append_locked` / `complete_trailing` unchanged |
| CE-3 | FIXED | `search_checkpoint.rs:288-300` `complete.writing` → sync → rename → dir fsync |
| ledgers-2 | PARTIAL | as pass 4. Selection V5 (conc5-2) is a further open site |
| store1-2 | PARTIAL | `store/src/file.rs:3626-3655`, in-process memory only |
| xcut-1 | FIXED | `telemetry/src/sink.rs:1508-1528` `events.lock` EX `try_lock` (D-1537) |
| xcut-2 | NOT FIXED | `pull/src/ingest.rs:3364-3365` |
| xcut-3 | NOT FIXED | `candidate_trades.rs:322` `create_dir_all`, no ancestor fsync |
| cli2-5 | NOT FIXED | `candidate_trades.rs:1312-1321` `create_new`, then `Flock::lock` |
| pull1-3 | NOT FIXED | `pull/src/capture.rs:247-262` |
| pull2-3 | NOT FIXED | `pull/src/cash_session_cache.rs:500-507`, `write_new` :528-546 |
| replay-3 | NOT FIXED | `checksum_receipts.rs:307-311` |
| store2-1 | NOT FIXED (no production caller) | `store/src/repair.rs:351-397` |

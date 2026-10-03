Verdict: 3 new findings (0 high, 1 medium, 2 low) in the append-only durable-writer family at 5140aca3. D-1760 fixes search-1, cli3-1 and replay-2, but its new `Err(_) if committed` arm in `index_stop_vix::publish` reports success after the call's own failed fsync.

### conc-pass3 / ledgers: workspace-wide sweep of every append-only durable writer in non-test code

Method: source only. No cargo was run and nothing in the repository was edited. I grepped every crate for `write_all`, `SeekFrom::End`, `.append(true)`, `create_new(true)`, `set_len(` and `sync_all`/`sync_data`, dropped `#[cfg(test)]` code and test drivers (`store/src/emits.rs`, `api/src/emitted.rs`, `pull/src/emit_sites.rs`), and read the append path and the writable-open path of each writer that remained. Liveness follows gaps-1 and pass-1 sel/cli3/replay. A "dead" module is compiled outside tests but has no production caller.

Kernel facts relied on, as in the earlier passes:
- **K1.** A buffered `write(2)` copies page by page. `generic_perform_write` stops at the next page when a fatal signal is pending and returns the bytes it already copied. SIGKILL, the OOM killer and an unhandled SIGINT (Ctrl-C) are all fatal; the cli installs no handler (ledgerall-1). So a record that straddles a page boundary can be cut mid-record.
- **K2.** After a writeback EIO, ext4 and XFS report the error once per open file description (errseq), mark the pages clean and leave them readable in the page cache. A later `fsync` on a new descriptor returns `Ok` and does not rewrite those pages. This depends on the filesystem; it is UNVERIFIED here and not measured, as in resources-1 and store1-2.

---

## ledgers-1 (low): `index_stop_vix::publish` turns this call's own failed receipt or directory fsync into a successful publication

**Where.** `crates/cli/src/index_stop_vix.rs:405-413`, added by D-1760. The receipt is written at `boolean_candidate_persistence.rs:66-78` (`Pending::finish`), through `write_or_equal` (:230-244) and `retained` (:246-257).

```rust
let published = catalog.with_current(|| {
    ...
    let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
    pending.verify_body(digest, body.len() as u64)?;
    pending.finish(lookup, digest, body.len() as u64)?;
    Ok(())
});
match published {
    Ok(()) => Reader::open(root, identity, pin, bounds),
    // A concurrent publisher of the same catalog finished first, and its
    // capture saw a different VIX store: its receipt is the answer.
    Err(_) if persistence::committed(&directory)? => saved(root, identity, pin, feed, bounds),
    Err(why) => Err(why),
}
```

```rust
pub(crate) fn committed(directory: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(directory.join("complete.bin")) {
        Ok(meta) => Ok(meta.len() == RECEIPT_LEN),
```

**Why it is wrong.**
- `finish` creates `complete.bin` with `create_new`, writes all 112 bytes, and only then calls `sync_all` (`retained`), followed by the directory `sync_all`. If either sync fails, `finish` returns `Err`, and the 112 bytes are already visible in the page cache.
- The arm is meant for a concurrent publisher that won the race. It does not ask whose receipt it is looking at. `committed()` is a length check, so it is `true` for this call's own unsynced receipt. `saved` → `Reader::open` then re-reads body and receipt from the page cache, they match, and `publish` returns `Ok(Reader)`.
- The error is dropped with `Err(_)` and never logged. The index-stop catalog then binds this VIX companion as published authority. This is §4's "fallback that hides a failure", and resources-1's in-call shape at a new site that D-1760 created.
- The same arm also hides any other error once a whole receipt exists (for example a `capture` refusal). The race comment justifies only one of those cases.
- The pre-check at :390 has the same weakness without any failure in this process. It runs with no owner lock, so it can return a companion whose receipt the publisher (a second process) has written but not yet synced, and whose sync then fails.

**Repro.**
1. Put the store root on a dm-flakey (or dm-error) target that fails writes for the window after `complete.bin` is created. Alternatively, fill a thin-provisioned volume so that writeback fails at fsync.
2. Run an index-stop verb that publishes a VIX companion for catalog C.
3. `Pending::finish` → `write_or_equal(complete.bin)`: `create_new` succeeds, `write_all(112)` succeeds, and `retained` → `sync_all` returns `EIO`. `finish` returns `Err("Input/output error")`.
4. `published` is `Err`. `committed(&directory)` returns `true` (length 112 from the page cache). `saved(...)` returns `Ok(Reader)`, and the command reports the companion as published.
5. After the cache is dropped or the machine reboots, `complete.bin` holds whatever reached the device. If its length survives but its bytes do not, every later `publish` takes the :390 shortcut, `Reader::open` refuses "receipt changed", and `prepare_in_namespace` compares the body only because the receipt is "whole". The D-1760 recovery never runs, and the identity is wedged again. (Which outcome you get depends on the filesystem, per K2.)

**Minimal fix.**
- Bind the race arm to the race. Take it only when the error is the owner-lock refusal from `prepare_in_namespace` (`"... already owned or lock refused"`), or when `prepare` failed before this call wrote anything. Never take it after this call's own `finish` returned `Err`; return `why` there.
- Do not discard the error. If the arm stays, wrap it: `Err(why) if committed => saved(...).map_err(|s| format!("{why}; {s}"))`, and emit an error event either way.

---

## ledgers-2 (low): failed-fsync-retried-into-success at the exact-retry and reuse paths that pop1-4, resources-1, store1-2 and replay-3 did not name

The class is already reported, but only for named sites: Population V1 (pop1-4), runs/frontier/trades `ensure_*` (resources-1), the bar store (store1-2) and checksum receipts (replay-3). Each proposed fix is per site ("poison the handle", "never call `verify` after `first`"). The live writers below have the same hole, and none of those fixes touches them.

| writer (live) | where the failed sync leaves bytes | where a later run re-syncs or reuses them and reports success |
|---|---|---|
| Base Evidence V2 (store root) | `population_base_evidence_ledger_v2.rs:1239-1241` (`sync Base records` `?` with the records left in place) | the orphan exact retry at :1205-1231 → :1237-1241 (`sync_data` Ok on a new fd); the reuse path at :1179-1191 ("re-sync reused Base records/completion") |
| Candidate Universe (store root) | rows `sync_data` before the receipt (the cand-1 window) | the orphan exact retry in `append_complete_locked` |
| Pre-Admission V1/V2 (store root) | Data `sync_data` at `pre_admission_data.rs:1378`/`:2600` | the trailing-orphan exact retry at :1328/:2550 |
| Search Lineage V4 | members `sync_data` at `anchored_search_lineage_v4.rs:976` | the trailing retry re-sync at :937-940 ("sync retry search-lineage V4 members"), and reuse at :918-925 |
| Statistics V3, Admission V4, Finalization V4 | data `sync_all` before the Completion | the receipt-less trailing-prefix exact retry (the pop2-4 path) re-syncs and appends the Completion |
| Observation V1/V2 | `population_observations_v1.rs:2278-2282`: `?` returns before `self.orphan` is set | next process: `orphan == data` (:2249-2255), then the Completion is appended and synced |
| Population V6 | `population_v6.rs:2170-2172` | `trailing` exact retry (:2116-2134), then the Completion `sync_all` |
| Execution V4 | data `sync_data` at `execution_v4.rs:2952` and siblings | `TrailingExecutionV4` exact-prefix resume |
| Selection V6 | `selection_v6.rs:343` / `:350` | `found` → `sync_all` → `Ok(false)` (:304-310); a partial exact prefix is resumed |
| Global Replay V4 | `global_replay_v4_store.rs:62`/`:69` | full-length rerun: compare, then `sync_all` (:30-69) |
| Boolean/index-stop evidence (11 namespaces) | `retained` `sync_all` (`boolean_candidate_persistence.rs:247`) | a committed reuse: `write_or_equal`'s `AlreadyExists` arm (:237-242) compares and **never fsyncs the file at all**; only the directory is synced. This is replay-3's exact shape, in a module replay-3 did not cover. |
| sweep-evidence `attempts.bin` | `append_events` barrier (`sweep_evidence.rs:1383-1386`) | the next `allocate` counts the unsynced rows and allocates after them; later barriers succeed |

**Concrete repro (Base Evidence, store-wide blast radius).**
1. `cli ledger-all ...` reaches `append_locked` for a universe U. The records `write_all` succeeds and `self.record_file.sync_data()` (:1239) returns `EIO`. The run fails with "sync Base records".
2. The operator reruns the same command on the same binary, with the store unchanged.
3. `open` → `scan` → `scan_orphan` finds U's records, read from the page cache.
4. `append_locked` takes the orphan branch: `compare_prepared_prefix` matches, so it appends 0 records. `sync_data` returns `Ok`: the first process saw the error, and this descriptor was opened after it (K2).
5. The Completion is appended and synced. The result is `Written`.
6. After eviction or a reboot, the records under U's durable Completion read back stale. `scan` → `validate_block` (:1029) refuses on every open, writable or read-only. Every Step-3 run on the store then refuses, because this file is in the store root (ledgerall-1).

**Minimal fix.** Apply the store1-2/resources-1 rule once, in a shared helper used by every writer above. After any `sync_all`/`sync_data` error on a ledger file:
- write a durable "uncertain" marker next to the file, or refuse in-process and also refuse a later exact retry that would complete a prefix written before that error;
- never let a later `fsync` on a fresh descriptor vouch for bytes it did not write.

The cheapest sound form: in every exact-retry and reuse path, rewrite the reused prefix bytes (`pwrite` the same bytes) before the `fsync`. The pages are then dirty again and actually reach the disk. For `write_or_equal`'s `AlreadyExists` arm, at least `sync_all` the file, as replay-3 asks for checksum receipts.

---

## ledgers-3 (medium): a kill or power loss mid-record wedges the store-root Pre-Admission, Base Evidence and Candidate Universe ledgers, and the Execution V4, Observation and Lineage V4 roots. Each has no recovery path, and each site has only been reported for the write-error case

**Where.** In every case the open path refuses any non-stride length before any orphan or retry logic runs, and the refusal applies to read-only opens too:
- Pre-Admission V1/V2. Writes: `pre_admission_data.rs:3595-3599` and `:3817-3824`. Refusal: `:3567-3571` and `:3789-3793`. Records are 740 and 812 bytes.
- Base Evidence V2. Writes: `population_base_evidence_ledger_v2.rs:1463-1503` (one `write_all` per 1,024-byte record) and `:1507-1530`. Refusal: `:1590-1593` ("ragged … body").
- Candidate Universe. Writes: `candidate_universe.rs:6258-6301` (32 rows per `write_all`, multi-page). Refusal: `:6158-6162` ("payload … is ragged").
- Execution V4. Write: `execution_v4.rs:4931-4935`. Refusal: `checked_record_count`. Records are 1,280, 128 and 1,024 bytes.
- Observation V1/V2. Writes: `population_observations_v1.rs:2278-2305` and `:3433-3446`. Refusal: `:2552` ("fixed-stride file is ragged"). Records are 512 and 1,024 bytes after a 64-byte header.
- Search Lineage V4. Write: `anchored_search_lineage_v4.rs:1645-1665`, both 768-byte members in one write. Refusal: `:1625`.

```rust
if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V1) {
    return Err(format!("pre-admission file body has {body} bytes, ragged against ..."));
```

**Why it is wrong, and why it is not already reported.**
- Earlier reports named only the write-error case at these sites: search-2 (Pre-Admission), sel-1 (Execution V4) and pop1-2 (Observation). Their fixes are rollback on `Err`, and that cannot run when the process dies inside `write` (K1).
- The kill-mid-record case was reported only for Admission, Finalization and Statistics (pop2-3), the result chain (sweep-2), the dead ledgers (cli3-3) and Population V6 (ledgerv6's pop2-3 verification).
- Pass-1 *search* waved Candidate Universe and Lineage V4 through as "documented in docs/06-limits.md". They are not documented there. The only limits text on kill-torn tails is "Interrupted Population ledger writes" (`06-limits.md:10438-10460`), which names Admission V3/V4 and Finalization V3, and the D-1631 section, which covers Lineage V2/V3, not V4. Base Evidence and Pre-Admission appear nowhere.
- The bytes past the last whole record are provably unacknowledged: every one of these ledgers is receipt-last, and the tail has no Completion. Even so, the writer holding the exclusive lock refuses them exactly as a reader does.
- Three of these ledgers are in the **store root** (`ledger_all.rs:874`, `roots.source = store_root()`). One Ctrl-C therefore wedges every `ledger-all` and `ledger-v6` run on the store, for every span and every ROOT. It also wedges `stored_family_v6`'s `open_read` (`:169`, `:200`) of every universe already committed.
- This is a different failure from ledgerall-1. ledgerall-1 is a whole-row orphan that the exact retry could still complete. A sub-record tail is refused before the orphan logic runs, so not even the identical rerun on the identical binary recovers.

**Repro (no rebuild, no data change).**
1. On 2026-10-03, run `cli ledger-all V 2025 1 2025 12 S P /x/ledgers`.
2. Press Ctrl-C while `append_rows` (`candidate_universe.rs:6283`) is writing a 32-row chunk. The chunk spans several pages, so the window is the whole syscall. With `strace -e write` attached, `kill -INT` on that write reproduces it every time. The cli has no SIGINT handler, so the signal is fatal (K1), and the row file ends at a page boundary that is not `64 + k·stride`.
3. Rerun the identical command. `CandidateUniverseLedgerV1::open` → `checked count` → "candidate row file payload … is ragged for fixed stride …". All 16 families refuse, and so does every later `ledger-all` or `ledger-v6` on that store.
4. `GET` or `cli` readers of committed universes (`open_read`) refuse the same way.

The same steps work with Base Evidence (every 4th record straddles a page), with Pre-Admission (740- and 812-byte records straddle pages regularly), and, per rung root, with Execution V4, Observation and Lineage V4.

**Minimal fix.** As in pop2-3 and sweep-2, implemented once and shared by all of these writers. In the writable `open`, under the exclusive writer lock:
1. If `body % stride != 0` and the bytes before the remainder validate as whole records, `set_len(header + whole·stride)`.
2. `sync_all` the file.
3. Emit an error-level telemetry event that names the file and the cut byte range.

Read-only opens keep refusing until a writer has healed the file. A whole-record tail with a bad seal must still refuse. Record the rule in `docs/05-decisions.md` and list these ledgers in `docs/06-limits.md`.

---

## Full writer table (coverage)

Legend: **RB** = rolls back (`set_len`) on a failed write. **Kill-tail** = what a writable open does with a sub-record tail left by a kill or power loss. **Retry identity** = whether an interrupted block's exact retry can be replayed after a rebuild or new data. **Sync** = whether a failed `sync_all` can later be confirmed as success. "Named" gives the earlier report that covers the cell.

| # | writer | file:line (append / refusal) | live | RB | Kill-tail on writable open | Retry identity (rebuild / new data) | Failed sync retried into success | Named / new |
|---|---|---|---|---|---|---|---|---|
| 1 | api pull audit journal | api/src/audit.rs:1172-1220 / :1205-1212 | yes | no, but not needed: 256-byte records in an O_APPEND file never straddle a 4 KiB page, and block allocation is all-or-nothing | refuses forever (D-0402, limits §97, documented) | N/A (no retry identity) | no: error returned, record later counted by `look` | recovery-4, press-2, autopilot-4; clean for this family |
| 2 | api recovery journal | api/src/recovery_journal.rs:369-460 | yes | 1024-aligned; handle poisoned on uncertain I/O | refuses (documented) | N/A | open re-syncs the file before exposing it (minor, K2 class) | recovery-2; checked sound |
| 3 | api recovery STOP control | api/src/recovery_control.rs:147-168 | yes | via #2 | via #2 | N/A | if state already equals status, no append; only the parents are synced (relies on #2's open sync) | runs-3, recovery-5 |
| 4 | cli operation audit index + per-invocation | cli/src/operation_audit.rs:324-347, 512-575 / :306-312 | yes | no, but not needed (256 B, aligned) | refuses | N/A (fresh id) | per-invocation handle poisoned (`failure`); `read` re-syncs the index (status only) | cli1-1/2/3/5 |
| 5 | sweep-evidence attempts/starts/lifecycle/levels/ranked | cli/src/sweep_evidence.rs:1364-1397, 1282-1297, 601-640 / :1195-1200 | yes | **no** | refuses | N/A (token-scoped) | yes: unsynced rows counted by the next `allocate` (ledgers-2) | cli2-1, sweep-2; sync: **ledgers-2** |
| 6 | sweep-evidence `reserve_start` | sweep_evidence.rs:1045-1065 | yes | n/a (create_new per token) | that token only | N/A | no | clean |
| 7 | results `runs.bin` | cli/src/results.rs:1268-1370 / :951-966 | yes | yes | refuses | identity has commit and digest, so a rebuild gets a new identity (no wedge) | yes | sweep-2, resources-1 |
| 8 | `frontier.bin` | cli/src/frontier.rs:1050-1120 / :1858-1868 | yes | yes | refuses | same as 7 | yes | sweep-2, resources-1, cli2-3 |
| 9 | `chosen-trades.bin` | cli/src/trades.rs:622-712 / :1185-1192 | yes | yes | refuses | same as 7 | yes | sweep-2, resources-1 |
| 10 | `detail-sets.bin` | cli/src/result_set.rs:449-490 / :515 | yes | yes | refuses | same as 7 | class | sweep-2 |
| 11 | candidate-trades detail files | cli/src/candidate_trades.rs:1307-1348 | yes | n/a (create_new per token) | dead token only | N/A | no (a token is never retried) | cli2-5, xcut-3 |
| 12 | expression evidence | cli/src/expression.rs:218-345 | yes | pending + link | pending file left in a dead token | N/A | no | clean (H5) |
| 13 | search checkpoint journal | cli/src/search_checkpoint.rs:208-290 | yes | per-sequence directory | interrupted hole skipped | N/A | writer poisoned | GAP11-0; clean |
| 14 | Boolean/index-stop evidence (11 namespaces) | cli/src/boolean_candidate_persistence.rs:88-260 | yes | rewrites unreceipted scratch (D-1760) | **now recovered** (D-1760) | recovered (D-1760) | yes: the `AlreadyExists` reuse never fsyncs | search-1 FIXED; **ledgers-2** |
| 15 | VIX companion publish | cli/src/index_stop_vix.rs:374-413 | yes | via 14 | via 14 | via 14 | **yes, inside one call** | **ledgers-1** |
| 16 | checksum receipts | cli/src/checksum_receipts.rs:306-360 | yes | n/a | n/a | content-addressed | yes | replay-3, replay-4 |
| 17 | Candidate Universe (store root) | cli/src/candidate_universe.rs:3666-3770, 6258-6324 / :6158-6162 | yes | yes | **refuses forever** | **wedges (commit and data digest)** | yes | cand-1, ledgerall-1; kill: **ledgers-3**; sync: **ledgers-2** |
| 18 | Base Evidence V2 (store root) | population_base_evidence_ledger_v2.rs:1158-1250, 1463-1530 / :1590 | yes | yes | **refuses forever** | wedges | yes (explicit "re-sync reused") | ledgerall-1; **ledgers-3**, **ledgers-2** |
| 19 | Pre-Admission V1/V2 (store root) | pre_admission_data.rs:1307-1395, 2529-2615, 3595, 3817 / :3567, :3789 | yes | **no** | **refuses forever** | wedges | yes | search-2, ledgerall-1; **ledgers-3**, **ledgers-2** |
| 20 | Statistics V3 | population_statistics_v3.rs:2371-2490, 2903 / :2868 | yes | **no** | refuses | wedges | yes | pop2-2/3/4/5/6; sync: **ledgers-2** |
| 21 | Observation V1/V2 | population_observations_v1.rs:2217-2310, 3398-3450 / :2552 | yes (via Statistics V3) | **no** | **refuses forever** | foreign orphan refuses (:2249-2255) | yes | pop1-2, pop1-3; **ledgers-3**, **ledgers-2** |
| 22 | Search Lineage V4 | anchored_search_lineage_v4.rs:911-1010, 1645-1665 / :1625 | yes | yes (D-1620) | **refuses forever** | wedges (pair id comes from Pre-Admission ids, which carry the commit); unenumerated site of pop2-4 | yes (explicit retry re-sync) | **ledgers-3**, **ledgers-2** |
| 23 | Admission V4 | population_admission_v4.rs:2698-2810, 3052-3080 / :2618-2625 | yes | yes | refuses | wedges | yes | pop2-3/4/7; sync: **ledgers-2** |
| 24 | Finalization V4 | population_finalization_v4.rs:2328-2440, 2718 / :2230 | yes | **no** | refuses | wedges | yes | pop2-2/3/4; sync: **ledgers-2** |
| 25 | Population V6 | population_v6.rs:2098-2190, 2444 / :2017-2019 | yes | **no** | refuses | wedges ("foreign retry cannot replace trailing prefix", :2116); unenumerated site of pop2-4 | yes | pop1-2, ledgerv6 (pop2-3 verification); sync: **ledgers-2** |
| 26 | Execution V4 | execution_v4.rs:2707-2960, 4931 / :4893-4896 | yes | **no** | **refuses forever** | foreign orphan refuses (documented EX-03) | yes | sel-1; **ledgers-3**, **ledgers-2** |
| 27 | Selection V6 | selection_v6.rs:299-360 | yes | no, but an exact byte prefix resumes | partial block refuses every read until the exact retry | wedges on a source change | yes (`found` → sync) | hunt-cli-a-5; sync: **ledgers-2** |
| 28 | Global Replay V4 | global_replay_v4_store.rs:10-80 | yes | byte-prefix resume | resumed | content-addressed (clean) | yes (full-length re-sync) | **ledgers-2** (minor) |
| 29 | store bar month | store/src/file.rs:2021-2175 | yes | header-slot commit | tail past `n_valid` tolerated | N/A | yes | store1-1, store1-2 |
| 30 | store repair revision | store/src/repair.rs:330-405 | yes | reservation never removed | revision Incomplete | per revision | n/a | store2-1 |
| 31 | census manifest | pull/src/ingest.rs:3296-3330 | yes | slot-counter commit | tail past the counter overwritten | N/A | slot `sync_all` failure has the store1-2 shape (unenumerated, derived index) | pull2-1/5, census-1, xcut-2 |
| 32 | NSE cash-session cache | pull/src/cash_session_cache.rs:483-528 | yes | n/a (create_new) | payload without receipt wedges the day | N/A | no | pull2-3 |
| 33 | vendor capture | pull/src/capture.rs:247-260 | yes | n/a | torn fixture | N/A | no | pull1-3 |
| 34 | telemetry sink | telemetry/src/sink.rs:262-272, 1161-1195, 1499 | yes | newline-terminates after a failed append | newline-terminated at open | N/A | no | telemetry-1, xcut-1, lifecycle-3; clean here |
| 35 | Population V1, V5; Admission V2/V3; Finalization V2/V3; Statistics V2 | population*.rs (see pop1/pop2) | per pop1/pop2 | mixed | refuses | wedges | yes | pop1-1..4, pop2-1..6 |
| 36 | dead: admission_store, institutional_statistics, stored_data_completeness | as cli3-3 | no | yes | refuses | — | institutional documents "failed sync → orphan an exact retry continues" | cli3-3 |
| 37 | dead: global_replay v1/v2/v3, selection v1-v5, execution_v3, execution_capability, execution_disposition_v2, lineage v2/v3 | various | no | v5/capability/disposition yes; others no | refuses | — | — | sel "checked", limits D-1631 |

---

## Fix verification (pass-1/pass-2 findings in this family that 331b05c..HEAD claims to fix)

| id | verdict | reason (code at 5140aca3) |
|---|---|---|
| search-1 | **FIXED** | `prepare_in_namespace` (`boolean_candidate_persistence.rs:119-132`) runs under the exclusive owner lock. When `committed()` is false (no 112-byte `complete.bin`), it unlinks any torn receipt and any earlier body, then writes `body.bin` fresh. A crash or short write before the receipt is no longer compared against, so the exact retry recovers. A committed directory still compares (history kept). Residual sync weakness: ledgers-2, row 14. |
| cli3-1 | **FIXED** | `index_stop_vix::publish` now shortcuts only on `persistence::committed(&directory)` (:390), not on the directory existing. An interrupted publish falls through to `prepare_in_namespace` and resumes. |
| replay-2 | **FIXED (permanent wedge); transient refusal remains** | An unreceipted body is scratch and is rewritten, so a retry whose capture differs no longer hits `write_or_equal`'s mismatch. A concurrent publisher that loses `try_lock` while the winner has not yet finished still gets an error (a transient refusal; a retry succeeds). The new error arm introduces ledgers-1. |

No other finding in this family (pop1-*, pop2-*, sel-1, search-2, cli2-1, cli3-3, sweep-2, cand-1, ledgerall-1, resources-1, store1-2, replay-3/4, pull1-3, pull2-3) is touched by 331b05c..HEAD. `git diff --stat` shows no change to their files, apart from doc-only edits in `frontier.rs` and the retired-stride refusal in `execution_capability.rs` and `execution_disposition_v2.rs`.

## Checked and clean (this family)
- 256- and 1,024-byte-aligned journals (api audit, operation audit, recovery journal) cannot be cut mid-record by K1, because no record crosses a page.
- Global Replay V4 and the Boolean evidence are content-addressed or byte-prefix resumed, so a rebuild cannot wedge them.
- The run-identity result chain (runs, frontier, trades, detail-sets) has the commit and data digest in each identity, so a rebuild produces a new identity rather than a foreign-orphan wedge.
- Every `set_len` rollback measures `end` under the same exclusive lock it truncates under.

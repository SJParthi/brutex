Verdict: 5 findings at 331b05c (0 high, 2 medium, 3 low). The production replay writer (V4) is sound. The defects are in the VIX companion's month capture, the checksum-receipt reuse path, and a late VIX lock collision.

### conc-pass2 / replay: global replay (v1-v4), VIX month identity, api readers of replay output, checksum_receipts, stored_data_completeness

Method: I read the source only and ran no cargo. Reachability at this commit:
- The only production replay path is `ledger_v6::replay_route` → `global_replay_v4::commit_stored_global_replay_v4`.
- `global_replay` (v1), `global_replay_v2` and `global_replay_v3` have no non-test caller. v2 is named only by `step3_comparison`, and nothing calls that.
- No api route reads Global Replay output. A grep of crates/api for global/replay finds only the Boolean replay-node knobs.
- The api reads the index-stop VIX companion through `indexstopvixjson.rs`, and the api's index-stop worker writes it.
- `checksum_receipts` is reached from cli `checksum-audit-stored` and from api `sweeprun` → `audited_range_command` → `audited_range` / `audited_stored`.
- `stored_data_completeness`'s ledger is opened only by tests and by the uncalled `step3_comparison`.

---

## replay-1 (medium): a transient VIX month-lock collision is published permanently as "Unavailable" in the index-stop VIX companion

**Where:** crates/cli/src/index_stop_vix.rs:530-559 (`load_month`), called from `capture` (:478-503), which `publish` (:373-411) calls. The writer is the api pull of `SpotTarget::Indices` (crates/api/src/ingest.rs:136, which "includes NSE-INDIAVIX") through `BarFile::open_or_create`.

```rust
match VixReferenceMonth::open(store, feed, month) {
    Ok(loaded) => Ok((Month { ... records: Some(loaded.records()), snapshot_digest: Some(loaded.snapshot_digest()), ..}, Some(loaded))),
    Err(reason) => {
        if reason.is_empty() || reason.len() > MAX_REASON_BYTES { return Err(..) }
        Ok((Month { ..., records: None, snapshot_digest: None, unavailable_reason: Some(reason) }, None))
    }
}
```

**Why it is wrong:**
- `VixReferenceMonth::open` → `BarFile::open_existing` takes a non-blocking shared try-lock on the month `.lock`. It refuses while any writer holds the month. vix_reference.rs:612-630 pins this as "a live writer must refuse the reference reader … another writer holds".
- `load_month` turns *every* `Err` into a typed `Unavailable` month. That includes this transient lock refusal, not only a missing file.
- The image is written content-addressed under `lookup_identity(catalog.identity(), catalog.completion_digest())` (:707-713). That identity names no VIX bytes.
- Every later `publish` for the same catalog sees the directory and returns `Reader::open` of the saved companion (:384-391). It never recaptures.
- So one moment of lock contention becomes a permanent statement that the VIX month was unavailable. Every trade in that month then carries `Stamp::Unavailable` in the companion and in `/index-stop-vix.json`, although the data exists.
- This is the "fallback that hides a failure" shape of §4, made durable. It is loud once, in a saved diagnostic, but it is wrong forever, and §3 rule 8 forbids rewriting it.

**Repro (two writers in one api process, or api plus cli):**
1. The api autopilot or a hand `/pull` ingests `NSE-INDIAVIX` 1min for month M. `BarFile::open_or_create` holds the exclusive month lock while it appends.
2. In the same window, an index-stop search (the api index-stop worker or `cli index-stop-search`) reaches `produce_catalog_inner` → `publish_vix` for a catalog whose trades fall in M.
3. `capture` → `load_month(M)` → `open_existing` gets EWOULDBLOCK. The month is saved with `unavailable_reason = "... could not be opened as reference evidence: ... another writer holds ..."`. `prepare_in_namespace` and `finish` publish it.
4. The pull finishes a second later. Every rerun of the search, and every `/index-stop-vix.json` read, now serves the saved Unavailable month for M, permanently.

**Minimal fix:** Classify the open error. Persist `Unavailable` only for durable facts about the store (a missing month file, or a corrupt or malformed month). Propagate a lock or busy refusal as an `Err` from `capture`, so nothing is published and the retry recaptures. Alternatively, wait on a blocking shared lock with a bound. Add a test that holds a writer on the VIX month and asserts that `publish` errs and leaves no namespace directory.

---

## replay-2 (medium): the VIX companion body is not a function of its lookup identity, so a retry after a crash or a race writes different bytes and `write_or_equal` refuses for good. cli3-1's proposed fix does not close the wedge.

**Where:** crates/cli/src/index_stop_vix.rs:384-410 (`publish`) and :530-559 (`load_month`). crates/cli/src/boolean_candidate_persistence.rs:189-208 (`write_or_equal`).

```rust
let lookup = lookup_identity(identity, pin);           // catalog only
...
let image = capture(store, feed, catalog, bounds)?;     // reads live VIX months
let body = codec::encode(&image, bounds.bytes)?;        // embeds records, snapshot_digest, reason text
let pending = persistence::prepare_in_namespace(root, NAMESPACE, lookup, &body)?;
```
```rust
Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
    if read_exact(path, body.len() as u64)? != body {
        return Err("Boolean evidence already exists with different or incomplete bytes; history preserved".to_owned());
```

**Why it is wrong:**
- The encoded body includes, for each VIX month, `records`, `snapshot_digest` and the free-text `unavailable_reason` (index_stop_vix_codec.rs:93-99).
- Each of those depends on the live store at capture time. The current VIX month grows minute by minute, and a month can be busy (replay-1) or absent before a backfill.
- The directory name, `lookup`, depends only on the catalog.
- cli3-1's minimal fix is to key the shortcut on `complete.bin` and otherwise re-enter `prepare_in_namespace`. That re-enters with a *freshly captured* body.
- If the crash came after `body.bin` was fully written (`write_or_equal` create_new + `write_all` + `sync_all`) but before `finish` wrote `complete.bin`, the retry compares the new body to the old one. They differ whenever any VIX month changed in between, and the retry refuses "different or incomplete bytes" on every later attempt. The single-stop checkpoint makes that retry mandatory, so the search stays stuck at batch N exactly as cli3-1 describes.

**Repro (crash):**
1. Batch N's catalog has trades in the current month M. `publish` captures M with 4,000 VIX records, writes and fsyncs `body.bin`, and is SIGKILLed before `finish` (boolean_candidate_persistence.rs:66-79).
2. The pull appends 30 more VIX minutes to M.
3. With cli3-1's fix applied, the rerun sees no `complete.bin` and recaptures: M now has 4,030 records and a new `snapshot_digest`. `write_or_equal(body.bin)` hits AlreadyExists, the bytes are unequal, and it refuses. Every rerun repeats this, because the store only grows.
4. Without cli3-1's fix the same state refuses on the missing `complete.bin`. Either way the identity is wedged.

**Repro (race, transient):**
1. Process A and process B both reach `symlink_metadata(&directory)`, and both get NotFound.
2. A captures, publishes and releases its owner lock.
3. B captures after a VIX append. `prepare_in_namespace` takes the now-free owner lock, and `write_or_equal` refuses "different bytes".
4. B's catalog attempt is marked Refused, although a valid companion exists.

**Minimal fix:** When `body.bin` exists and `complete.bin` does not, *adopt* the saved body instead of recapturing:
- read it under the owner lock;
- decode it, and check that its catalog identity, completion pin and feed equal this call's;
- verify its digest and `finish` with it.

On a race, treat "AlreadyExists with different bytes" from a concurrent completed publisher as success when `complete.bin` now exists, by re-checking it and returning `Reader::open`. Apply this together with cli3-1's fix.

---

## replay-3 (low): `checksum_receipts::publish` reuses a full-length receipt without ever fsyncing it or its directory, so a crash before the first publisher's barrier leaves a receipt that is admitted but was never made durable

**Where:** crates/cli/src/checksum_receipts.rs:306-312 (fast path). The barriers it skips are at :345-357. The same path serves `publish_binding` (:421) and `publish_span_binding` (:470).

```rust
fn publish(path: &Path, expected: &[u8; BYTES]) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == BYTES as u64)
    {
        Receipt::open(path, expected)?;   // read-only, shared lock, compares bytes
        return Ok(());
    }
    ...
        file.sync_all().map_err(error)?;          // :354
        File::open(path.parent()...).and_then(|dir| dir.sync_all())  // :355-357
```

**Why it is wrong:**
- The slow path makes the file and its directory entry durable before it returns.
- The fast path decides "already published" from the length alone, and verifies the bytes through the page cache. It never syncs anything.
- So a publisher killed after the seal `write_all` (:347-352) and before `sync_all` (:354), or before the directory sync (:355-357), leaves a full-length, byte-correct, *unsynced* receipt.
- Every later `audit_month`, `publish_binding` or `publish_span_binding` takes the fast path, admits it, and continues. The sweep's strict results and span bindings then name this receipt identity.
- The doc on `perform` promises a refusal on "any failed ... publication or completion durability barrier". Here the barrier never ran, and nothing reports it.
- V4's `persist` (global_replay_v4_store.rs:27-76) shows the correct pattern: it always takes the locked path, and on an exact full-length file it writes nothing but still runs both fsyncs and the directory fsync.

**Repro:**
1. Run `cli checksum-audit-stored ...`. The new receipt is written. The process is SIGKILLed between :352 and :354.
2. The operator reruns, or the api strict sweep audits the same month. `symlink_metadata` sees 512 bytes, `Receipt::open` matches, and the run continues and records output that names the receipt.
3. Power is lost before writeback.
4. On reboot the receipt is absent or zero-length. `admit_month(expected)` refuses it as missing. The strict-read door "never creates or repairs", so every strict re-admission of that recorded identity fails until someone reruns `audit_month`. On filesystems that expose unwritten extents as zeros, the 512-byte zero file is refused as "not an exact prefix" for good.

**Minimal fix:** In the fast path, after `Receipt::open` succeeds:
- open the file writable;
- take the exclusive lock, or do it under the shared lease, since fsync needs no write access;
- call `sync_all` on the file and fsync the parent directory.

The simpler alternative is to drop the fast path and always take the locked path, which writes nothing on an exact file, as V4 does.

---

## replay-4 (low): `namespace_directory` fsyncs the receipt root only when it creates the namespace, so a crash between `create_dir` and that fsync is never repaired

**Where:** crates/cli/src/checksum_receipts.rs:375-383.

```rust
match fs::create_dir(&base) {
    Ok(()) => File::open(&root).and_then(|dir| dir.sync_all()).map_err(error)?,
    Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
```

**Why it is wrong:**
- A run killed after `create_dir` and before `sync_all` leaves the namespace entry in `root` not yet durable.
- Every later run takes the `AlreadyExists` arm and never fsyncs `root`.
- Receipts are then fsynced into `checksum-receipts-v1/`, `audited-inputs-v1/` and `audited-spans-v1/` (`publish` syncs only `path.parent()`), and reported durable. The entry that makes those directories reachable has never had a barrier.

**Repro:**
1. Run 1 is SIGKILLed between :376 and :377.
2. Run 2 publishes and syncs a receipt, and the strict sweep records results.
3. Power is lost. Under POSIX semantics the `checksum-receipts-v1` entry may be lost, taking every receipt in it.
4. ext4 and xfs usually commit the mkdir along with the later fsync, so this is filesystem-dependent. That is why it is low.

**Minimal fix:** fsync `root` in the `AlreadyExists` arm as well. It is one cheap fsync per call, and it makes the barrier idempotent.

---

## replay-5 (low): Global Replay V4 opens VIX months with a non-blocking try-lock only after all OOS work, so a concurrent VIX pull refuses the whole replay

**Where:** crates/cli/src/global_replay_v4.rs:473-489 (`VixCatalog::stamp`), reached from `schedule` (:325) → `account_decision` (:607-608). That is after `selection.replay(...)` (:229) has folded and replayed every stream.

```rust
self.months.insert(
    (feed, month),
    VixReferenceMonth::open(self.root, feed, month)?,
);
```

**Why it is wrong:**
- `VixReferenceMonth::open` → `BarFile::open_existing` refuses immediately when a writer holds the month lock.
- The months are opened lazily, one per (feed, IST civil month) of each admitted priceable trade. That happens only at the end of `prepare`.
- So any api VIX pull overlapping the end of a `ledger-v6-replay` aborts the whole run: all eight Selection V6 replays and every OOS fold, possibly hours of work. Lifecycle records it as Refused, and the run must be redone.
- It is loud and a retry succeeds, so it is a liveness defect, not a correctness one.
- The month key is computed correctly: IST civil month via `pull::session::IstMoment`, the same authority as `vix_reference::slot_of`, and entry and exit months are opened separately.

**Repro:**
1. Start `cli ledger-v6-replay ...` with an OOS span whose admitted trades fall in month M.
2. While it computes, the api autopilot pulls `SpotTarget::Indices` and holds the `NSE-INDIAVIX` 1min M writer.
3. When `schedule` reaches the first admitted trade in M, `stamp` gets "another writer holds". `prepare` fails, and `recorded` finishes the attempt as Refused.

**Minimal fix:** Resolve the set of VIX months the trades need, and open them *before* the replay work. Alternatively, retry the shared try-lock with a bounded backoff that names the wait. Do not convert the refusal into absence (see replay-1).

---

## Pass-1 verification

- **cli3-1 (index_stop_vix publish shortcut on directory existence):** CONFIRMED. index_stop_vix.rs:384-395 still returns `Reader::open` whenever the directory exists. `prepare_in_namespace` (boolean_candidate_persistence.rs:96-119) still creates the directory before `owner.lock`, `body.bin` and `complete.bin`. Its minimal fix is insufficient on its own; see replay-2.
- **cli3-3 (stored_data_completeness ragged tail, latent):** CONFIRMED.
  - `commit` (stored_data_completeness.rs:885-893) rolls back only when `write_all` returns an error. The ragged-tail refusal in `scan_file` remains.
  - It is still latent. The only non-test openers are in `step3_comparison` (:772 `open_read`; :1712 is inside `mod tests`, which starts at :1508), and `compare_step3_on_disk_v1` has no caller anywhere in the workspace.
  - Addition of the same latent class: a failed `sync_all` (:890-894) returns without `set_len(start)` and without refreshing `generation`. The written record stays in the page cache but not in `self.receipts`. The same handle's next `commit` then refuses "changed since validation", and a reopen adopts the unsynced record.
- **cli3 "checked clean": checksum_receipts.rs:** PARTLY REFUTED. The locked slow path is as described. The unlocked full-length fast path skips every durability barrier (replay-3), and the namespace-root fsync is not idempotent (replay-4).
- **cli3 "checked clean": global_replay_v4_store.rs:** CONFIRMED.
  - The blocking exclusive `lock()` covers the prefix compare and the append.
  - A rerun on any prefix, including the full length, always reaches both `sync_all`s and the directory fsync (:63-74).
  - `verify` takes a blocking shared lock and checks exact length, per-record checks, ordering, digest and generation.
  - A crash at any line between create and directory fsync is completed or matched on the next run.
- **cli3 "checked clean": global_replay_v4_lifecycle.rs:** CONFIRMED. `recorded` gives every begun stage and the parent a terminal. Overlapping or mismatched stages are refused, and both causes are kept by `combine`.

## Checked and clean (no finding)

- **V4 determinism:**
  - `schedule` uses `sort_unstable_by_key((entry_micros, priority, strategy_digest))`. Full-key ties cannot reach output, because `GlobalSinglePositionV1::schedule_minute` refuses `DuplicateOrderingKey` (runner/src/portfolio.rs:464).
  - `VixCatalog.months` is a HashMap used only for lookup. No iteration reaches the bytes.
  - `replay_id` excludes VIX rows (kind 5). `publication_id` includes them by design.
- **V4 publication races:** two processes publishing the same `publication_id` serialize on the blocking flock. The second one compares the full prefix, writes nothing and still fsyncs. Different VIX snapshots give different publication ids, so neither file can be corrupted.
- **checksum_receipts concurrency:**
  - The writer's exclusive try-lock against a reader's shared try-lock refuses loudly (WouldBlock), never silently.
  - `Receipt` holds its shared flock for its lifetime, with an explicit unlock on drop (store::flock), and rechecks generation, nlink and bytes on every `require_current`/`read_record`.
  - std `File::try_lock` and `store::flock::Flock` are both flock(2), so they exclude each other.
- **stored_data_completeness locking:** the writable open holds an exclusive flock across the header create and scan. `commit` holds it across `require_unchanged`, append and sync. `authority` re-checks length and digest under a shared lock.
- **vix_reference.rs:** no shared state. The month is indexed from one `BarFile` snapshot of `n_valid`, and the `BarFile` is dropped before return. `slot_of` and `IstMoment` month identity are consistent across both callers.
- **global_replay v1/v2/v3:** not reachable in production at this commit, so not audited for crash behaviour.

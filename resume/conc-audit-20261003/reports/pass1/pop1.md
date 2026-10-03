# pop1: concurrency and state audit of the population ledgers (pass 1)

Verdict: 4 findings (0 high, 2 medium, 2 low). All are crash or short-write recovery defects. The flock discipline in this slice is sound: every writer re-validates under an exclusive lock, and readers re-check generations.

Slice: crates/cli/src/population.rs, population_v5.rs, population_v6.rs, all_rung_population_v5.rs, population_observations_v1.rs, population_base_evidence_ledger_v2.rs, at commit 331b05c. Audited from source only; cargo was not run.

---

## pop1-1 (medium): a V1 population interrupted between row chunks can never be committed

**Location.** crates/cli/src/population.rs:3367-3404 (`append_rows`) and :3318-3347 (`require_exact_existing`).

```rust
for chunk in rows.chunks(ROW_WRITE_CHUNK_ROWS) {      // 32 rows per write
    ...
    self.row_file
        .write_all(encoded)
        .map_err(|why| rollback_message(&self.row_file, at, "population rows", &why))?;
}
self.row_file.sync_all()...
```

```rust
let mut expected_at_actual_location = expected;
expected_at_actual_location.block.first = actual.block.first;
if actual != expected_at_actual_location {
    return Err(format!(
        "population {} already has different row facts; no byte was replaced", ...
```

**Why it is wrong.**
- A block is written as many separate `write_all` calls, one per 32 rows.
- `rollback_message` only runs when a write returns an error. It does not run when the process dies between two chunks (Ctrl-C/SIGINT, SIGKILL, OOM-kill).
- After such a death, the row file ends in a whole-row prefix: sequences 0..k-1 of population P, with k < n. That prefix is not ragged, and `index_rows_range` accepts it as a valid block of `count = k`.
- On the exact rerun, `append_complete_locked` takes the `raw_blocks.contains_key` branch (:3153). `require_exact_existing` then compares facts with `count = k` against `count = n` and refuses.
- No code path completes a trailing prefix. Rows are append-only, and a second block for P would be refused as "interleaves"/"non-contiguous duplicate". So P is permanently uncommittable.
- The module doc (:29-31) covers only "a crash after the row sync". The newer ledgers in the same slice do handle this case:
  - V5: `complete_trailing`, population_v5.rs:2220
  - V6: the trailing prefix check, population_v6.rs:2116-2134
  - Base: `compare_prepared_prefix`

**Repro.**
1. Run a population with n > 32 rows.
2. SIGINT the process after the first `write_all(encoded)` at :3401 returns and before the second.
3. The row file now holds rows 0..31 of P.
4. Rerun the identical command. `PopulationLedger::open` succeeds and `append_complete_v4` returns "population <P> already has different row facts; no byte was replaced". This happens on every retry, forever.

**Minimal fix.** In the `raw_blocks.contains_key` branch, if the existing block is the last block in the file, has no receipt, and is a row-exact prefix of `rows` (byte compare via `require_exact_block` on its `count`), then:
- append `rows[count..]`;
- `sync_all`;
- update `raw_blocks[P]` to the full facts;
- continue to the receipt.

This is the same rule V5 and V6 already implement. Alternatively, encode the whole block into one buffer and issue a single `write_all`. That narrows the window but does not close it for short writes.

---

## pop1-2 (medium): a short write leaves a ragged tail that makes the whole V5/V6/Observation ledger unopenable

**Location.**
- population_v5.rs:3149-3153 (`append_raw`), used by `append_row_suffix` (:2263) and `append_completion` (:2279)
- population_v6.rs:2444-2448 (`append_raw`), used by `append_locked` (:2168, :2173)
- population_observations_v1.rs:2278-2282 (V1 Data), :2301-2305 (V1 Completion), :3432-3436 and :3442-3446 (V2)

```rust
fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), PopulationV5Refusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append Population V5 fixed record: {why}"))
}
```

```rust
self.file.seek(SeekFrom::End(0))
    .and_then(|_| self.file.write_all(&record))
    .and_then(|()| self.file.sync_data())
    .map_err(|why| format!("cannot sync observation authority Data: {why}"))?;
```

**Why it is wrong.**
- `write_all` can fail after a partial write. A typical case is ENOSPC or EDQUOT partway through a record: the first `write` returns a short count, and the next returns an error. Some bytes of the record then stay at the tail.
- None of these sites truncates back to the pre-append length.
- Every open of these ledgers refuses a non-stride length, so one failed append makes the ledger unopenable for every reader and writer, including all populations that were already committed:
  - V5: `checked_record_count`, population_v5.rs:3067
  - V6: "Population V6 data file is ragged", :2019
  - Observation V1: :2552
  - Observation V2: :3577
- No repair path exists. The same slice already treats this as a defect elsewhere:
  - population.rs rolls back with `set_len(at)` (`rollback_message`, :5146).
  - population_base_evidence_ledger_v2.rs rolls back with `set_len(original)` at :1497 and :1524.

**Repro.**
1. Fill the filesystem so only `RECORD_BYTES - 10` bytes are free.
2. Run a V6 append. `append_raw` writes a partial record, then gets ENOSPC, and returns `Err`.
3. Free space and rerun. `PopulationV6Ledger::open` refuses with "Population V6 data file is ragged". Every Selection/ledger_v6 reader of this root refuses too.

**Minimal fix.**
- Record `len` before each append.
- On any write error, `set_len(len)` and `sync_all`.
- Report a separate "poisoned" error if the truncation itself fails.

This mirrors `append_prepared_records` in population_base_evidence_ledger_v2.rs:1463-1503. Apply it to V5 `append_raw`, V6 `append_raw`, and the four Observation V1/V2 write sites.

---

## pop1-3 (low): the Observation authority ledgers never fsync their directory after creating files

**Location.** population_observations_v1.rs:2141-2182 (V1 `open_inner`) and :3332-3363 (V2 `open_inner`).

```rust
let mut file = OpenOptions::new().read(true).write(writable).create(writable).open(&file_path)?;
if writable && file.metadata()?.len() == 0 {
    file.write_all(&authority_header()).and_then(|()| file.sync_data())...
}
```

After this, `append_data` syncs only the file (`sync_data`). Nothing in the module calls `sync_all` on the directory: grep finds no directory sync in the file.

**Why it is wrong.**
- POSIX durability of a newly created name requires an fsync of the parent directory.
- Every sibling ledger in this slice does that after creating its files:
  - population.rs `sync_directory(&dir)` (:2731)
  - V5 `sync_directory` (:2025, :2289)
  - V6 `root_file.sync_all()` (:1966, :1976, :2182)
  - Base `sync_directory` (:1190, :1250)
- `append_and_reopen` returns `Written` once the Completion is `sync_data`-ed, and callers treat the authority as durable. A power loss before the directory entry is written back can drop the whole `AUTHORITY_FILE` (and the lock file) even though its records were synced.

**Repro.** On a filesystem without ordered-metadata guarantees (or ext4 with data=writeback), on a fresh root:
1. Call `append_and_reopen`; it returns `Written`.
2. Cut power before the periodic metadata writeback.
3. After reboot, the authority file name is absent, and `open_read` refuses the "absent file" that the caller was told was committed.

**Minimal fix.** In both `open_inner` functions, when the lock or authority file was created, or the header was written, call `File::open(admitted_root)?.sync_all()` before scanning. This is the same pattern as V5 `sync_directory`.

---

## pop1-4 (low): a V1 retry after a failed row fsync trusts the second fsync and can commit a receipt over rows that never reached disk

**Location.** population.rs:3405-3407 (`append_rows`: on `sync_all` failure it returns `Err` with the bytes left in place) and :3153-3157 (the retry path).

```rust
self.row_file
    .sync_all()
    .map_err(|why| format!("the new population rows could not be synced: {why}"))?;
```

```rust
if self.raw_blocks.contains_key(&receipt.population_id) {
    self.require_exact_existing(rows, receipt, expected)?;
    self.row_file.sync_all().map_err(...)?;
}
```

**Why it is wrong.**
- On Linux, a writeback error is reported to one `fsync` per open file description (errseq). The pages can be marked clean while the data is not on disk.
- The retry absorbs the unsynced tail as an orphan block. `require_exact_block` then reads it back from the page cache, so it matches.
- The second `sync_all` on the same handle returns `Ok`, and the V2/V3/V4 receipts are then written and synced.
- The commit marker now vouches for row bytes that may not exist on the device.
- After a crash, the open-time reconciliation finds receipt facts with no matching row block and refuses the entire ledger. So the failure is loud, but it lands on the whole store rather than on the one retry.

**Repro.**
1. Inject an EIO on the first fsync of population-v1.bin (dm-flakey or fault injection).
2. `append_complete_v4` returns "the new population rows could not be synced".
3. The caller retries on the same handle (or a fresh handle in the same boot). The retry returns `Written`.
4. Power-cycle. `PopulationLedger::open_read` refuses with "has a receipt but no row block", or with a seal or digest mismatch.

**Minimal fix.** On a row `sync_all` failure:
- `set_len(at)` to roll the unsynced block back;
- mark the handle unusable, for example `self.writable = false`, with an error telling the operator to reopen.

The retry must then rewrite the rows rather than reuse page-cache bytes. The same reasoning applies to V5 and V6 trailing-prefix reuse after a `sync_data`/`sync_all` failure; this report lists V1 only, because only there is the retry path on the same live handle.

---

## Checked and found sound (not findings)

- **population.rs.** The writer re-absorbs all four files under the exclusive `population-write.lock` before every decision. A reader (`open_read`) holds the shared lock only during open. `page_v4` re-checks every generation under the shared writer lock. Legacy `page()` flocks the row file, which the writer never locks, but it reads only committed, immutable ranges.
- **Lock clones.** `try_clone` on `writer_lock` shares one open file description, so unlock through either handle is consistent. Separate handles in one process use separate descriptions, so they exclude each other correctly.
- **Crash between rows, V2, V3 and V4 receipts (population.rs).** The exact rerun reuses each lower receipt and appends the missing ones. Verified in `append_complete_v4_locked`.
- **V5/V6.** The lock is taken before any child is created or initialised (except the lock file itself). A trailing whole-record prefix is completed on exact retry. A foreign trailing prefix is refused loudly, by design. The directory is synced after creation and after the completion.
- **Base Evidence ledger V2.** It rolls back on write error, completes an orphan prefix on exact retry, writes completion last, and syncs the directory.
- **Observation V1/V2.** The flock is held for the whole ledger lifetime through `Flock`, and the writer is dropped before the read-only reopen in both `append_and_reopen` paths, so there is no self-deadlock.
- **all_rung_population_v5.rs.** Directory admission and identity checks only. No writes, locks or threads in production code.
- **Nondeterminism.** No HashMap or HashSet iteration feeds output bytes or digests in this slice. `FactsBuilder::finish` iterates `masks` only to compute order-independent sums, minimums and maximums. The `reconcile_receipts*` refusal order was already reported as hunt-conc-3 and is not repeated here.
- **Threads and atomics.** None in production code. The atomics are test-only temp-name counters.

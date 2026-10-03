Verdict: 7 findings (0 high, 3 medium, 4 low) in the Population Admission/Finalization/Statistics V2-V4 ledgers at 331b05c. All of them are about crash recovery and durability. No data race, lock-order cycle or HashMap-order leak was found.

# pop2: concurrency and state audit, pass 1

Slice: crates/cli/src/population_admission_v2.rs, _v3.rs, _v4.rs; population_finalization_v2.rs, _v3.rs, _v4.rs; population_statistics_v2.rs, _v3.rs.

**Liveness.** I traced callers before rating anything:
- **Admission V3, Finalization V3 and Statistics V2:** live through `cli ledger-all` (`all_rung_population_v5.rs:651-676`).
- **Statistics V3, Admission V4 and Finalization V4:** live through `cli ledger-v6` (`step3_orchestrator.rs:2295`, `:2336`, `:2358`).
- **Admission V2 and Finalization V2:** the write half is `dead_code` outside tests, so they get no finding of their own.

Every per-rung root is shared by every run against the same ROOT argument (`ledger_v6.rs:200-205`, `RungRoots::create`).

## Findings

### pop2-1 (medium): Admission V3 cannot recover a crash in the middle of its decision block, and the rung ledger then refuses every later append
- **Where:** `population_admission_v3.rs:3987-3989` (writes decisions one record at a time), `:4071-4077` (retry check).
- **Code:**
  ```rust
  for decision in &prepared.decisions {
      append_raw(&mut self.decision_file, &encode_decision(decision)?)?;
  }
  ...
  if trailing.block_id != prepared.source.block_id || trailing.decisions != prepared.decisions
  { return Err(format!("Admission V3 trailing block {} is not exact retry {}", ...)); }
  ```
- **Why it is wrong:** the writer appends each 2,048-byte decision with its own `write_all` and syncs once at the end. A crash after decision i of n (0 < i < n) leaves a whole-record proper prefix. `scan` accepts that prefix as `trailing` (`validate_trailing_decisions` checks prefix order only, `:3699-3716`). On the next run, `append_locked` routes to `complete_trailing` because `trailing.is_some()` (`:3982-3984`). That function demands the full decision vector, so even the byte-exact retry refuses, with both hex ids equal. Every other block that run, or any later run on that rung root, tries to append also enters `complete_trailing` and refuses. The ledger is wedged for good, and no tool repairs it.
- **Inconsistent with its siblings:** Finalization V3 handles the identical shape with `prepared.rows.starts_with(&trailing.rows)` (`population_finalization_v3.rs:2418`), with a comment saying exactly this crash can happen. D-1630 (commit 66f140c, W2-cli1-4) fixed the same class in `admission_store` and left V3 alone. The test `partial_orphan_corruption_reserve_and_lock_bytes_refuse` (`:6365-6386`) pins the wedge as expected behaviour.
- **Repro:** run `cli ledger-all … ROOT` and SIGKILL it while rung `1min` Admission V3 is between `append_raw` calls (for example, after the first decision). Rerun the identical command. It fails with `all-rung 1min Admission V3 refused: Step 3 Population Admission V3 commit refused: Admission V3 trailing block X is not exact retry X`. It fails the same way on every later rerun, and for every other month range on that ROOT.
- **Fix:** accept `prepared.decisions.starts_with(&trailing.decisions)`. Append the missing suffix, `sync_data`, then write the Completion and sync the directory, as Finalization V3's `complete_trailing` does. Better still, also write the block in one `write_all`, as D-1630 did. Then change the test to assert that the exact retry recovers.

### pop2-2 (medium): Finalization V3/V4 and Statistics V2/V3 never got the failed-append rollback that Admission V3/V4 received
- **Where:**
  - `population_finalization_v3.rs:2857-2861`
  - `population_finalization_v4.rs:2718-2725`
  - `population_statistics_v3.rs:2903-2910`
  - `population_statistics_v2.rs:5479-5486`
- **Code (identical shape at all four sites):**
  ```rust
  file.seek(SeekFrom::End(0))
      .and_then(|_| file.write_all(raw))
      .map_err(|why| format!("cannot append Finalization V4 record: {why}"))
  ```
- **Why it is wrong:** Admission V3/V4 wrap the same call in `append_with_rollback` (D-0916, W2-cli10-3, invariant C4-CLI-05-01). Their own doc comment states the defect: "Without that a partial `write_all` left a ragged tail, and every later open, read-only included, refused the file's already committed authorities as ragged." The four writers above still have that defect. Their scans refuse any non-stride length:
  - Finalization V4: `"Finalization V4 data file is ragged"`, `:2230`
  - Finalization V3: `checked_record_count`, `:2777`
  - Statistics V3: `record_count`, `:2868`
  - Statistics V2: `record_count`, `:5441`
- **Repro:** the volume holding ROOT fills up during `commit_population_finalization_v4`. `write_all` writes 1,000 of the record's 4,096 bytes, and the next `write` returns ENOSPC. The file is now `64 + 4096·n + 1000` bytes. After space is freed, every `open_read` and `open_write` on that root fails as "ragged", so every committed Finalization V4 block on that rung, from every earlier month, becomes unreadable. Nothing in the tree repairs it.
- **Fix:** move `append_with_rollback` (`set_len(end)` on error) into a shared helper and call it from all four `append_raw` functions. Add the C4-CLI-05-01 test to each module.

### pop2-3 (medium): a crash in the middle of a record leaves a sub-record tail that permanently fails every open, read-only included
- **Where:** `population_admission_v4.rs:2618-2625` (`scan`), and the equivalent ragged checks in Finalization V4 (`:2230`), Admission V3 (`checked_record_count`, `:4428`), Finalization V3 (`:2777`), Statistics V3 (`:2868`) and Statistics V2 (`:5441`).
- **Code:**
  ```rust
  if len < HEADER_BYTES as u64 || !(len - HEADER_BYTES as u64).is_multiple_of(RECORD_BYTES as u64)
  { return Err("Admission V4 data file is ragged".to_owned()); }
  ```
- **Why it is wrong:** the rollback only covers a write that *returns* an error. A process killed inside `write_all` leaves the same ragged tail.
  - Admission V4 and Finalization V4 records are 4,096 bytes at offset `64 + 4096·k`, so every record straddles a page boundary.
  - In Statistics V2/V3, every fourth 1,024-byte record straddles one.
  - Linux `generic_perform_write` checks `fatal_signal_pending` between page chunks and returns a short count, so SIGKILL or the OOM killer can stop a write after the first 4,032 bytes.
  - Power loss can also persist a partial extension.

  The bytes past the last whole record were never covered by a synced Completion, so they are provably unacknowledged. Even so, the writer, which holds the exclusive lock, refuses them exactly as a reader does. For the torn *header*, the writer repairs the identical situation (`holds_torn_header`, `:1704`). For the torn *tail*, nothing does.
- **Repro:** SIGKILL `cli ledger-v6` during the Completion `append_raw` in `PopulationAdmissionV4Ledger::append_locked` (`:2789`). The file ends 4,032 bytes into a record. The rerun fails at `open_write` with `"Admission V4 data file is ragged"`. So does every later run on that rung root, and so does `open_read` of blocks committed months earlier.
- **Fix:** in `open(writable = true)`, under the exclusive lock, truncate `len` down to `HEADER + k·RECORD` when the remainder is shorter than one record. Then `sync_all` and resume through the existing trailing-prefix logic. Readers keep refusing. A whole-record tail with a bad seal must still refuse.

### pop2-4 (low): a receipt-less trailing prefix accepts only an exact retry whose identity includes the build commit, so a rebuild after a crash wedges the rung ledger
- **Where:** `population_admission_v4.rs:2727-2731` (`trailing.source != prepared.source`), with the same rule in Finalization V4 (`:2361`), Statistics V3 (`:2394-2395`), Statistics V2 (`:4611-4616`) and Admission V3 (`:4071`).
- **Code:** `if trailing.source != prepared.source || … { return Err("Admission V4 trailing prefix is not the exact retry".to_owned()); }`
- **Why it is wrong:** `BlockSourceV4.source_commit_digest` is `hash(verified_commit)` (`step3_orchestrator.rs:3593`, and `population_admission_v4.rs:746`). After a crash leaves a trailing prefix, the only block that can ever be appended to that root again is one prepared by the same binary commit from the same inputs. The usual response to a crash is to rebuild with a fix, and after that the exact retry can no longer be constructed. The per-rung root is shared by every month range, so ledger-v6 on that rung is wedged with no recovery tool (a grep for quarantine, discard-orphan or truncate-orphan finds nothing for these ledgers). Other ledgers in the repo instead tolerate and skip a valid foreign orphan: Execution V1 "retains valid orphan evidence" (`docs/04-invariants.md` EC-01).
- **Repro:**
  1. SIGKILL ledger-v6 after the Admission V4 evidence `sync_all` (`:2788`) and before the Completion.
  2. Rebuild at a new commit.
  3. Rerun. It fails with `Admission V4 trailing prefix is not the exact retry`, and every later run on that rung fails the same way.
- **Fix:** let the writer skip a fully valid receipt-less foreign prefix, as Execution V1 does: the next block starts after it, and the prefix stays as unreferenced evidence. Alternatively, ship an operator verb that records and quarantines it. At minimum, put the orphan's block id and source commit in the refusal text.

### pop2-5 (low): Statistics V2/V3 never fsync the root directory after creating their files or after committing
- **Where:** `population_statistics_v3.rs:2142` and `:2158` (`open_file(..., create)`), and `append_locked` `:2478-2487`; the same in `population_statistics_v2.rs:2187`, `:2211` and `:4582-4645`.
- **Code:** the data and lock files are created with `.create(create)`. Only `self.data_file.sync_all()` follows, and there is no `root_file.sync_all()` anywhere in either module.
- **Why it is wrong:** every sibling in the slice syncs the directory after creating a file and after a commit (Admission V4 `:2583-2586` and `:2797`; Finalization V3 and Admission V3 `sync_directory`). `fsync` on the file does not make a new directory entry durable under POSIX.
- **Repro:**
  1. ledger-v6 commits Statistics V3 into a fresh `statistics/1min`.
  2. Admission V4 then appends a block that binds that Statistics authority and syncs its own directory.
  3. Power is lost.
  4. After reboot, the Admission V4 block is present, but `statistics/1min/<DATA_FILE>` may be absent. Any audit that re-verifies Admission against its Statistics source now fails to open it.

  A rerun recreates identical bytes, which is why this is rated low.
- **Fix:** hold the root `File` and `sync_all` it after a create, and again after the Completion sync, as the V4 ledgers do.

### pop2-6 (low): Statistics V2/V3 cannot recover a torn header; Admission V4 and Finalization V4 can
- **Where:** `population_statistics_v3.rs:2822-2834` and `population_statistics_v2.rs:5395-5408`.
- **Code:** `if len == 0 { write header; sync_all; return Ok(()) } verify_header(file, path)`. `verify_header` then refuses `len < 64` with "shorter than Statistics V3 header".
- **Why it is wrong:** a crash or power loss during the first `write_all(&header)` and `sync_all` can leave between 1 and 63 bytes. From then on, every open refuses, the writer included. Admission V4 and Finalization V4 added `holds_torn_header` (`:1704`) for exactly this residue.
- **Repro:** power loss while the first Statistics V3 append on a new rung is initializing the file leaves a file of, say, 32 bytes. Every ledger-v6 run on that rung then fails with `… is 32 bytes, shorter than Statistics V3 header`.
- **Fix:** port `holds_torn_header`. When the writer, under the exclusive lock, finds a strict prefix of the constant header, it rewrites the header.

### pop2-7 (low): an unrelated append by a concurrent run invalidates retained authorities, and nothing serializes runs on one ROOT
- **Where:** `population_admission_v4.rs:2893-2908` (`require_unchanged`, which compares a blake3 hash of the whole file), reached from `finalization_projection` → `read_complete` (`:2821`). The commit that sets it up is `step3_orchestrator.rs:2336-2363`.
- **Code:** `file_generation(&self.data_file, &self.data_path, …)? != self.data_generation` → `"Admission V4 retained file/root generation changed"`.
- **Why it is wrong:** the ledger is append-only, and its flock is held only during open and append. A second process that appends a different, legitimate block to the same per-rung file changes the whole-file digest. That invalidates the first process's retained authority between the Admission commit and the Finalization commit. The ledger-v6 and ledger-all verbs take no run-level lock on ROOT (`lib.rs:1899-1910`, `ledger_v6.rs`). The first run therefore fails partway through the pipeline, after several ledgers have already durably committed its blocks. Finalization V4 and Population V6 reauthenticate the retained Admission source again later, so the failure window covers most of the route.
- **Repro:**
  1. Process A runs `cli ledger-v6 V 2024 1 2024 3 … ROOT` and process B runs `cli ledger-v6 V 2024 4 2024 6 … ROOT`.
  2. A returns from `commit_population_admission_v4` for `1min`.
  3. B appends its own `1min` Admission V4 block.
  4. A calls `commit_population_finalization_v4` and fails with `Finalization V4 Admission source refused: Admission V4 retained file/root generation changed`.

  A's rerun recovers through exact reuse, so the harm is a spurious refusal and wasted work.
- **Fix:** take an exclusive per-ROOT run lock in `ledger_v6`/`ledger_all` and refuse a second run up front. Alternatively, have the retained authority check only its own block's byte range plus the inode, not the whole file's digest.

## Checked and clean
- **Lock pairing:** every `lock()`/`lock_shared()` in the Admission V2/V3/V4 and Finalization V3/V4 ledgers is followed by `unlock()` on both the Ok and Err paths (the closure-then-combine pattern). Statistics V2/V3 use a `store::flock::Flock` guard whose `Drop` unlocks.
- **Lock order:** no ledger holds its flock while taking another ledger's flock. `commit_population_finalization_v4` prepares (taking the Admission shared lock and releasing it) before it opens the Finalization writer, and Admission V4 commit does the same with Statistics. No ordering cycle exists.
- **Append order:** evidence is synced before the Completion is appended, and the Completion is synced before the directory, in all V3/V4 writers. The reuse path re-issues both barriers.
- **Data-file creation:** in V4 it happens under the exclusive lock. Lock-file creation races (`exists()` then `create`) are harmless: both sides only sync the directory.
- **Determinism:** no HashMap iteration reaches output, digests or refusal text in this slice. `receipts` and `audits` are only `get`/`insert`/`contains_key`. There are no threads, rayon, atomics or wall-clock values outside tests.
- **No overlap with earlier reports:** none of the above duplicates hunt-conc-1 to hunt-conc-8 or errpaths. errpaths' `exists()` note for Admission V4 and Finalization V4 is correct and not repeated here.

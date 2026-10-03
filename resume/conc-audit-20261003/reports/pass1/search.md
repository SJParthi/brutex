# conc-pass1 / search: 2 findings (0 high, 2 medium, 0 low) at 331b05c

Slice: crates/cli/src/anchored_search_lineage_{v2,v3,v4}.rs, candidate_universe.rs, pre_admission_data.rs,
boolean_observation_file.rs, boolean_*.rs (non-test), index_stop_search_{checkpoint,progress,reader}.rs,
index_stop_source_context{,_codec}.rs. Audit only; no cargo run, nothing edited.

## search-1 (medium): a crash inside a content-addressed Boolean/index-stop evidence write wedges that identity forever, and with it every resumable search whose pending batch needs it

File: crates/cli/src/boolean_candidate_persistence.rs:189-211 (`write_or_equal`), reached from :119 (`body.bin`) and :74 (`complete.bin`).

```rust
fn write_or_equal(path: &Path, body: &[u8]) -> Result<(), String> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            file.write_all(body).map_err(display)?;
            file.sync_all().map_err(display)?;
            ...
        }
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_exact(path, body.len() as u64)? != body {
                return Err("Boolean evidence already exists with different or incomplete bytes; history preserved".to_owned());
            }
            Ok(())
        }
```

Why it is wrong. The file is created with `create_new` *before* any byte is written or synced, and nothing distinguishes "a crash left a short prefix of my own bytes" from "different evidence". Every namespace that goes through `prepare_in_namespace` gets its directory name from a deterministic identity. That covers boolean-candidates/statistics/admission/oos/qualification-v1, index-stop-candidates/qualification/source-context/catalog-context/vix-reference-v1 and index-consistency-v1. Some identities are a hash of the request, source and commit (`boolean_candidate_v1.rs:300` `catalog_identity`). Others are a hash of the body itself (`index_stop_source_context.rs:318` `context_identity(body)`). So the only retry that can ever happen writes the same identity again. That retry reaches the `AlreadyExists` arm and refuses for good. Nothing deletes or repairs a directory here, and that is correct under the append-only rule. This breaks CLAUDE.md §3 rule 5 ("Reruns are safe"). It also breaks the promise printed by `index_stop_search.rs:302` ("saved successful children are retained and exact retry reuses them"). The single-stop checkpoint makes the retry mandatory: `Frame::done` and `transition` (index_stop_search_checkpoint.rs:165-207, 529-543) only accept a completion of the exact reserved batch. So the search cannot route around the dead child.

Repro (process death):
1. `index-stop` single-stop search publishes the pending frame (index_stop_search.rs:230-231) and enters `live::parallel`.
2. One lane reaches `index_stop.rs:401` `prepare_in_namespace(root, "index-stop-candidates-v1", identity, &body)`. It enters `write_or_equal(body.bin)`, `create_new` succeeds, and the process is SIGKILLed during `write_all` of a multi-page body. Linux `generic_perform_write` stops between pages on a fatal signal and leaves a short file. Power loss has the same effect: it can strike after `create_new` and before `sync_all`, which on ext4 delalloc leaves a 0-byte or partial `body.bin`.
3. The flock dies with the process. The next identical invocation recovers the pending frame (`checkpoint::recover`) and re-runs the same batch. That rung recomputes the same identity, takes the owner lock (the owner file is empty, so the check passes), and hits `AlreadyExists`. `read_exact` returns the short prefix, `!= body`, and the call refuses with "already exists with different or incomplete bytes".
4. `completed_links` turns that into "Single-stop batch remains pending ...". Every later invocation repeats steps 3 and 4, so the search can never advance. The same happens for `complete.bin` if the crash lands between its `create_new` and `sync_all`: a 0-byte receipt. `Observation::open` and `persistence::verify` also refuse that identity for every reader.

Minimal fix. Write to a temporary name in the identity directory while holding the owner lock (unique per process, e.g. `body.bin.<pid>.<nonce>`), then `sync_all`, then `rename` (or `link`) onto the final name, then fsync the directory. The final name then appears only complete. Alternatively, keep `create_new`, but in the `AlreadyExists` arm, while holding the exclusive owner lock, treat a file that is a strict byte prefix of `body` (including empty) and has no `complete.bin` beside it as an interrupted write of this same evidence. Complete it in place (append the suffix, `sync_all`). Refuse only on a true mismatch. Add a test that truncates `body.bin` and `complete.bin` to 0 and to N/2 and expects the exact retry to succeed.

Not reported before. hunt-conc, errpaths and the other workspace hunts do not mention `write_or_equal`, `body.bin` or `complete.bin` torn writes, and docs/06-limits.md has no entry for them.

## search-2 (medium): Pre-Admission Data V1/V2 append has no write-error rollback, so one ENOSPC/EIO mid-record makes the ledger unreadable for every later open, read-only included

File: crates/cli/src/pre_admission_data.rs:3595-3599 and :3817-3824. Call sites :1335, :1376, :1385 (V1) and :2557, :2598, :2607 (V2).

```rust
fn append_record(file: &mut File, raw: &[u8; RECORD_BYTES]) -> Result<(), PreAdmissionDataRefusal> {
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(raw))
        .map_err(|why| format!("cannot append pre-admission record: {why}"))
}
```
and the open-time check at :3561-3571 (V2 has the same check at :3785-3795):
```rust
    if !body.is_multiple_of(PRE_ADMISSION_RECORD_STRIDE_V1) {
        return Err(format!("pre-admission file body has {body} bytes, ragged against ..."));
```

Why it is wrong. This is exactly the defect D-0916 (W2-cli10-3) fixed for Admission V3/V4. D-1620 fixed it for Search Lineage V4. `candidate_universe.rs` `append_rows` and `append_receipt` (:6255-6324) already roll back with `set_len(original)`. Pre-Admission Data was missed, and neither docs/05-decisions.md nor docs/06-limits.md names it. `write_all` that fails partway leaves the bytes it did write. Pre-admission records are 740 bytes (V1) and 812 bytes (V2), so they regularly straddle a 4 KiB block. The ledger is live: `step3_orchestrator.rs:3899` and `:3969` write it, and `stored_family_v6.rs:169` and `:200` read it with `open_read`.

Repro:
1. Run the step-3 transaction on a filesystem with less free space than one block. `append_complete_locked` writes the Data record (:1376). Its `write_all` copies the bytes that fit before the block boundary and then gets ENOSPC. The error is returned, but the partial bytes stay in the file.
2. Free some space and rerun. `PreAdmissionDataLedgerV1::open` reaches `record_count(file_len)` (:1113). The body is no longer a multiple of 740, so the open refuses with "ragged against 740-byte records".
3. The same refusal hits `open_read` from `stored_family_v6.rs:169`. Every previously committed pre-admission authority in that ledger is now unreadable, although none of its bytes changed. Nothing in the code repairs the tail. The same sequence applies to the Completion record (:1385) and to V2.

Minimal fix. Port `append_with_rollback` (anchored_search_lineage_v4.rs:1645-1665). Record `end = seek(End)`; on a `write_all` error, `set_len(end)` and name both errors if the truncation fails too. Use it in `append_record` and `append_record_v2`. Add the D-0916-style test `a_partial_append_error_truncates_back_and_committed_authority_stays_readable` for both versions, and list the change under D-0916/D-1620 in docs.

---

## Checked and not reported

- **index_stop_search_progress.rs `parallel` / `schedule`.** The `Relaxed` `fetch_add` work counter only hands out indices. Results are reassembled by index (`assemble` refuses duplicates and gaps). `cancelled` uses Release/Acquire. `for … in receiver` drops the receiver on `break`, so a blocked `SyncSender::send` returns Err and cannot deadlock. A panic in a lane propagates through `broadcast` to the scoped thread, and `join` maps it to "pending checkpoint retained". All of this was also noted by hunt-conc.
- **index_stop_search_checkpoint.rs.** A pending frame is published before the work and a done frame after it. `transition` forbids skipping a reservation, re-assigning one, or changing a reserved batch. Recovery walks the pinned predecessor chain and requires `history.len() == journal.acknowledged()`. I found no skip or double count. Crash holes in the journal are handled by search_checkpoint (out of slice; the `complete` marker issue is the known GAP11-0).
- **boolean_qualified_journal.rs.** The Writer is poisoned on any attempted publication until a validating reopen. It relies on the Journal's exclusive owner lock, which is held for the Writer's whole lifetime.
- **boolean_campaign.rs and boolean_grammar_campaign.rs.** Their checkpoint chains are safe after a publish error because `Journal::publish` self-poisons. The refusal publish after a failed publish therefore errors loudly instead of forking the chain.
- **boolean_observation_file.rs.** Leases are taken on a duplicated owner descriptor, so they share one OFD. The `projecting` CAS refuses nested or concurrent use of that one OFD. Field drop order unlocks before the flag clears. `with_current_many` sorts and dedups by directory before taking leases, so acquisition order is consistent. All locks are shared.
- **anchored_search_lineage_v4.rs.** The open, lookup and append paths each take and release the lock-file flock around their work. A generation recheck catches a writer from another process. Members are written in one call with write-error rollback (D-1620). A *process-killed* ragged tail is still refused for good, but this class is documented in docs/06-limits.md ("Interrupted Population ledger writes ... Still refused", and the V2/V3 section), so it is not re-reported. A side note: the D-1620 comment "the only whole-record prefix a crash can leave is the NIFTY member" is true, but a crash can just as easily leave a non-record-aligned prefix, which still refuses. V2 and V3 are dead code (documented).
- **candidate_universe.rs.** Rows are written before the receipt, there is a whole-row orphan rule, and write errors roll back. Opens are generation-checked. The process-kill ragged tail is the same documented class as above.
- **HashMap use in candidate_universe, pre_admission_data and lineage_v4.** These maps are used only for keyed lookup and duplicate detection. No iteration order reaches output bytes or a digest.
- **Worker-count text** (boolean_catalog_prepared.rs:206, boolean_oos_command.rs:92, index_stop_search.rs:206) was already reported as hunt-conc-5.
- **Durable writes inside rayon workers at the Boolean sites** were already reported as hunt-conc-2.
- **No directory fsync after creating the candidate_universe and pre_admission_data files.** Not reported: on ext4, XFS and btrfs, `fdatasync` of a newly created file commits its directory entry, so I could not build a concrete loss path.

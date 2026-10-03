# conc-pass1 / sel — Selection, Execution and Step-3 slice at 331b05c

**Verdict: 1 finding (1 medium). The live append paths are serialized correctly under flock, and their crash-prefix recovery works. The one live gap is that Execution V4 (and the dead Execution V3) still use `seek(End)+write_all` with no rollback. That is the D-1622 class, which was fixed only for Selection V5 and institutional statistics.**

Slice: `crates/cli/src/selection.rs`, `selection_v3.rs`, `selection_v4.rs`, `selection_v4_authority.rs`, `selection_v5.rs`, `selection_v6.rs`, `selection_v6_source.rs`, `execution_capability.rs`, `execution_disposition_v2.rs`, `execution_v3.rs`, `execution_v4.rs`, `step3_comparison.rs`, `step3_orchestrator.rs` (production code only; `execution_lease.rs` excluded). I worked from source only. No cargo was run.

Reachability, which sets severity: production reaches only `execution_v4` → `selection_v6`, via `ledger-v6` → `step3_orchestrator.rs:2378` and `ledger_v6.rs:587`. Selection V1–V4, `selection_v4_authority`, `execution_capability`, `execution_disposition_v2` and `step3_comparison` have no production caller (gaps-1). `execution_v3`, `selection_v5`, `all_rung_*_v5` are `expect(dead_code)` (lib.rs:110-127).

## Findings

### sel-1 — medium — Execution V4 append has no rollback: one short write (ENOSPC/EIO) permanently wedges the rung's execution root

- **Where:** `crates/cli/src/execution_v4.rs:4931-4935`. Callers are at :2905, :2918, :2931 and :2949 (`append_*_suffix`, `append_completion`). The same code is at `execution_v3.rs:4047-4051`, but that file is dead code in production.
- **Code:**
  ```rust
  fn append_raw(file: &mut File, raw: &[u8]) -> Result<(), ExecutionV4Refusal> {
      file.seek(SeekFrom::End(0))
          .and_then(|_| file.write_all(raw))
          .map_err(|why| format!("cannot append Execution V4 fixed record: {why}"))
  }
  ```
- **Why it is wrong:** Records are 1,280 bytes for parameters and Completion, 128 for percentiles, and 1,024 for dispositions (execution_v4.rs:72-78). Suppose `write(2)` accepts part of a record because the filesystem fills mid-record, and `write_all` then gets `ENOSPC`. The function returns `Err`, but it leaves a file whose length is not a multiple of the stride. Every later `ExecutionV4Ledger::open` runs `scan` → `HeldFile::record_count` → `checked_record_count`, which refuses: `"Execution V4 {name} file has ragged length {bytes}, not a multiple of {stride}"` (execution_v4.rs:4893-4896). This happens before `append_locked` can consider an exact-retry orphan, so freeing disk space and re-running the identical command cannot recover. Every committed execution in that root also becomes unreadable through `open_read`. D-1622 (docs/05-decisions.md:54083) names this exact defect: *"A short write (ENOSPC, EIO) left a ragged or torn tail that every later open refused ... so the ledger was wedged for good"*. It fixed `selection_v5::append_raw` (now `append_with_rollback`, selection_v5.rs:3028-3055) and `institutional_statistics`, but not the live Execution V4 writer that feeds Selection V6. Whole-record crash prefixes are handled (`TrailingExecutionV4` + `require_exact_prefix`); partial records are not.
- **Repro (exact):** run `ledger-v6` for one rung on a volume with fewer than 1,280 bytes free when `append_parameter_suffix` (execution_v4.rs:2727) runs. Its first `write` stores, for example, 700 bytes of a parameter record, and the next returns `ENOSPC`. The command fails with "cannot append Execution V4 fixed record: No space left on device". Free space, then re-run the same command. `commit_stored_execution_v4` → `persist_prepared` → `ExecutionV4Ledger::open_write` → `scan` → refuses "parameter file has ragged length N·1280+700". Every later `ledger-v6` / `ledger-v6-replay` for that rung refuses at Step 3 Execution V4, and nothing in the code can repair it. The same happens if the short write lands in the percentile, disposition or Completion file.
- **Minimal fix:** route `append_raw` through the same `append_with_rollback` that selection_v5.rs:3036-3055 uses. Record `end = seek(End(0))`, and on a write error call `file.set_len(end)`, naming both errors if the truncation fails. Do the same in `execution_v3.rs:4047`. Add a test in the style of `selection_v5::tests::a_failed_append_truncates_back_and_the_ledger_stays_open` with a write closure that writes half a record and returns `Err`.

## Checked and clean (or already reported)

| area | file:line | concern | verdict |
|---|---|---|---|
| Execution V4 lock discipline | execution_v4.rs:2458-2554, 2685-2705, 3010-3160 | read-modify-write outside the lock | The exclusive flock is held across `require_unchanged` → `scan` / suffix writes → Completion → `scan`. Lookups take a shared flock and recheck the full generation (len, inode, mtime, content digest). `try_clone` shares the OFD, so the unlock through the original fd and the retained clone agree. |
| Execution V4 crash prefixes | execution_v4.rs:2726-2766, 2819-2852 | crash between data and Completion | Data files are `sync_data`'d before Completion. Completion is synced, then the directory. A whole-record orphan is resumed only by the exact retry, and the order rule (params before percentiles before dispositions) is enforced. |
| Execution V4 creation durability | execution_v4.rs:2468-2481 | new file entry not durable | The directory is fsynced when any child is created, and children are created under the writer flock. |
| Selection V6 persist | selection_v6.rs:296-360 | payload/seal ordering, directory sync, lock | Exclusive flock across scan+write; payload synced before seal; seal synced; directory synced. Readers hold a shared flock. An exact partial prefix resumes. The wedge after an interrupted persist with a changed source, and every committed read refusing while a partial tail exists, are already **hunt-cli-a-5**. Selection V6 also lacks write-error rollback, but there a short write is an exact prefix that the exact retry completes, so it adds nothing material to hunt-cli-a-5. |
| Selection V6 authority reads | selection_v6.rs:117-133, 362-381 | stale cache | Every read re-derives the source, rescans under a shared lock and re-derives again. Nothing is cached. |
| Foreign orphan wedge in Execution V4 | execution_v4.rs:2824-2833 | crash, then source change, refuses all new commits | Intentional, documented design ("foreign/orphan Execution bytes ... refuse", 04-invariants EX-03 and siblings). Same class as hunt-cli-a-5; not re-reported. |
| Selection V5 | selection_v5.rs:1906-2440, 3019-3055 | create before lock, rollback | Children are created before the lock with `create(true).truncate(false)`. That is harmless, and the directory is fsynced when created. Rollback is present (D-1622). The module is dead in production. |
| Selection V1–V4 ledgers | selection.rs:1769-1995, 2085-2310; selection_v3.rs:1115-1330; selection_v4.rs:1700-1910 | no rollback; V1–V3 `create_dir_all` with no directory fsync | These defects are real in the code, but no production caller writes these ledgers (only tests; V4 is read by the unreachable `step3_comparison`). Not raised. |
| selection_v4_authority nested flocks | selection_v4_authority.rs:89-123, 177-188 | outer shared lock held while inner ledgers take the same lock file | All acquisitions are shared, and Linux flock grants a compatible shared request even while an exclusive waiter is queued, so there is no self-deadlock. Unreachable anyway. |
| HashMap → output | selection_v6_source.rs:91-130; execution_*; selection_* | iteration order leak | Maps are used for lookups only. Output order comes from file order / `Vec`. |
| Threads, atomics, statics | whole slice | races | No production thread, rayon, atomic, static or Mutex. Atomics appear only in test root counters. |
| Step-3 root admission | step3_orchestrator.rs:354-445 | rename-and-replace TOCTOU | The residual pathname race is acknowledged in the doc comment (:361-367), and the post-check refuses. Not a new finding. |
| step3_comparison `exists()` absence | step3_comparison.rs:510-947 | stat error reported as absence | Already **errpaths-6**. |

# slice24 — 3 findings

Files: crates/cli/src/population_finalization_v3.rs, population_finalization_v4.rs,
population_observations_v1.rs, population_statistics_v2.rs. All four are live
from crates/cli/src/step3_orchestrator.rs (:1694, :2295, :2358, :2421, :2445,
:4440, :4497).

## F1 [medium] A failed append write is never rolled back in Finalization V3/V4, Statistics V2 or Observation V1/V2. The ragged tail then blocks every later open, read-only included.
- where:
  - population_finalization_v3.rs:2857-2861 `append_raw` = `seek(End(0)).and_then(write_all)` with no `set_len`. Callers: :2335, :2347, :2443, :2456.
  - population_finalization_v4.rs:2718-2725, the same. Callers: :2417, :2422.
  - population_statistics_v2.rs:5479-5486 `append_raw_record`, the same. Callers: :4579, :4584 (new block) and :4633, :4638 (orphan resume).
  - population_observations_v1.rs:2278-2282 and :2301-2305 (V1 Data and Completion), :3432-3436 and :3442-3446 (V2). Each is inline `seek(End).and_then(write_all).and_then(sync_data)` with no rollback.
- what: `write_all` can fail partway through a record (ENOSPC, EIO, EFBIG). When it does, the bytes already written stay in the file and the error is returned through `?`. Nothing truncates back to the start offset. On every later open, the whole-file stride check refuses:
  - V3: `checked_record_count` :2775-2779, "ragged length".
  - V4: `scan` :2227-2231, "data file is ragged".
  - Statistics V2: :5441, "ragged bytes against stride".
  - Observation V1: `scan_authority_file` :2552-2553. Observation V2 scans the same way.

  The refusal also hits `open_read`. Every completed authority already in the file becomes unreadable, and no retry can repair the file, because the exact-retry and orphan paths only recover whole-record prefixes. This is the defect class W2-cli10-3 / D-0916 (docs/05-decisions.md:44757-44800). That entry fixed it for Admission V3/V4 with `append_with_rollback`, and its "What this does not do" section limits it to those two modules. These five ledgers still have the unfixed pattern. None of the four files is listed in known/*, FIXED.md or docs/11-findings.md for this class. c4b-1 and c4b-5 cover other modules: lineage, selection_v5, institutional_statistics.
- evidence: a trace, plus existing tests that pin the refusal half:
  1. The writer is in `append_locked` (stats :4579). `append_raw_record` → `write_all(raw)` writes k < 4096 bytes and then gets ENOSPC.
  2. The `?` returns Err. `self.data_file` now has a length that is not a multiple of `POPULATION_STATISTICS_V2_RECORD_STRIDE`.
  3. On the next `PopulationStatisticsV2Ledger::open_read` → `scan` → `record_count(file_len)` → Err "...ragged bytes...". `ragged_corrupt_and_semantically_resealed_sources_refuse` (:6868-6879) shows that one stray byte makes open_read refuse. `v2_ragged_corrupt_resealed_and_stale_authorities_fail_closed` (observations :4269-4286) shows the same for Observation V2, and the V4 test `every_exact_trailing_prefix_recovers_but_foreign_and_ragged_refuse` (:3304-3316) for V4.

  D-0916 reproduced this exact code shape on Admission V3/V4 with a half-record append.
- fix: use the D-0916 pattern at every site. Record `end = seek(End(0))` before the first record of the call. On any `write_all` or `sync` error, `set_len(end)` and return an error naming both the write error and whether the truncation held. For Finalization V3, roll back the row file and the Completion file separately.

## F2 [low] Statistics V2 and Observation V1/V2 create their lock and data files but never sync the containing directory. Their "durable" invariants (PS-02, CO-03) are therefore not met on the first append.
- where:
  - population_statistics_v2.rs:2185 `open_file(&lock_path, writable, writable)` and :2214 `open_file(&data_path, ...)` (create), then `ensure_header` :5395-5408, which runs `file.sync_all()` only. The file has no directory handle and no directory sync anywhere: grep finds no `sync_directory` and no root `File` in the file.
  - population_observations_v1.rs:2134-2139 and :2160-2165 (V1 `create(writable)`), :2178-2181 (header `sync_data` only). For V2: :3330-3337, :3343-3350, :3359-3362. No directory sync in either.
- what: docs/04-invariants.md PS-02 (:3677) says Statistics V2 "becomes visible only after its sealed Data/raw-row block and matching receipt-last Completion are durable". CO-03 (:3708) says the observation authority "becomes visible only after fixed-stride Data and Completion are synced". On the first append into an empty root, the data file's directory entry is created and never fsynced. POSIX does not make the new name durable just because the file's own data was fsynced. A power loss after `append_population_statistics_v2` returns `Written` can therefore lose the whole ledger file even though the caller holds a durable receipt.

  Sibling ledgers in this slice do sync the directory after creating:
  - Finalization V3: `sync_directory(&root_file, &root_path)` at :1968-1970 and :2351.
  - Finalization V4: root `sync_all` at :2189-2193 and :2432-2434.

  AV-06 (:3747) and ML-02 (:3278) make the directory barrier an explicit part of "durable".
- evidence: a trace. The first `append_population_statistics_v2(root, ...)` → `open_writer` → `open_file(create=true)` creates `DATA_FILE`. `ensure_header` syncs the file only. `append_locked` syncs the file only (:4582, :4591). It returns `Written`, and nothing ever fsyncs `root`. Observation V1 follows the same path: `open_inner` → `append_data` → `sync_data` only.
- fix: open the root directory like V3's `open_root_directory`. When either file was created, sync it after the header write and again after the first Completion, as Finalization V3 does at :1968 and :2351.

## F3 [low] Statistics V2 and Observation V1/V2 scans accept a trailing orphan whose identity duplicates a completed authority. After that, any other append is refused for good.
- where: population_statistics_v2.rs:2266-2276 (orphan branch of `scan`), population_observations_v1.rs:2584-2615 (V1 `scan_authority_file`, `pending` left as the orphan) and the V2 scan at :3600-3633.
- what: `derive_audit_id` zeroes `sequence` before hashing (stats :5218-5219), so a block's audit_id does not depend on its position. `scan` stores a trailing Data/prefix as `self.orphan` without checking whether `self.audits` already contains `manifest.audit_id`. Finalization V3 does make that check (:2093-2098, "trailing block ... duplicates a completed block"), and so does Finalization V4 (:2257-2261, "trailing prefix repeats a complete block").

  With such a file:
  - `append_locked` (:4536-4540) takes the `Reused` branch for that id.
  - Every other prepared block goes to `resume_orphan`, which refuses "trailing orphan belongs to ..., not exact retry" (:4610-4616).

  So the ledger opens cleanly and reports a trailing prefix, but can never accept another audit. The open_read rustdoc (:2150-2153) lists "duplicate audit" among the states it refuses. Observation V1 behaves the same way: `append_data` hits `Reused` first (:2228-2241), and a foreign pair refuses on the orphan (:2249-2254). The state cannot come from a crash, because the writer checks Reused before writing. It needs a corrupted or edited file, which these ledgers otherwise go to lengths to refuse at open.
- evidence: a trace through the cited lines. In `scan`, a second Data record for an existing id with sequence = `completed_audits` passes `validate_orphan_prefix` (:3133-3226). That function never consults `self.audits`. The tail is then installed as `OrphanV2`.
- fix: in the stats `scan` orphan branch, and at the end of both observation scans, refuse when the orphan's id is already a key of `audits`, mirroring V3 :2093 and V4 :2257.

## Checked, no finding
- CSCV: `next_train_mask` (Gosper), `first_train_mask`, `segment_mask`, `choose_u64`, `canonical_split_count` (C(S-1,S/2), e.g. 35 for S=8), `derive_layout` (largest even divisor ≤16) and `cscv_placement`. The doubled-midrank comparison is m > (N-1)/2, the same as `runner::pbo::bottom_half`.
- `pbo_from_counts`, `wilson_lower` (count-only floats, documented), Romano-Wolf rank permutation, and recomputation of the bootstrap evidence.
- Observation building: one period per accepted IST session, cross-day refusal, exit-day attribution, total reconciliation against the evaluated cell, and the duplicate-semantic refusal.
- Finalization V3/V4: trailing-prefix recovery, exact reuse and bound checks; Statistics V2: orphan resume and bound checks.

Known items not re-reported: W2-cli12-0/1/2/5, W2-cli11-3, and the O_NOFOLLOW constant.

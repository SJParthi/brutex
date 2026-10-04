# Concurrency and state audit, pass 10: the multi-stage ledger chain as a whole

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 2 new findings: conc10-1 (medium, reproduced with cargo) and conc10-2 (low). Another pass is needed.

What holds up across stages:
- Every successor binds its predecessor by identity, taken from a freshly reopened, receipt-last Completion.
- A crash *between* two stages is recovered exactly: the rerun reuses stage k and resumes at k+1.
- No stage reads "latest" by scanning. Every lookup is keyed.
- V5 and V6 files never collide on one ROOT.

Both new defects sit in the three **store-root** ledgers that every Step-3 run on a store shares (Candidate Universe V1, Base Evidence V2 and Pre-Admission V1/V2). Those are the only ledgers in the chain that outlive a ROOT.

Method: I read the source. I ran cargo once, for conc10-1 only. That was one throwaway test in `/home/claude/wt/scratch-conc10` with CARGO_BUILD_JOBS=2, and the worktree and its target directory have been removed. Known ids are not re-reported: ledgerall-1/2/3, ledgerv6-1/2/3, ledgers-1/2/3, pop1-*, pop2-1..7, sel-1, conc4-*, conc5-*, the conc-pass3 locks table (row 3: strict month shared locks held for the run), and barflow-1.

## Stage-by-stage table

Where the files live: S = store root (`crate::store_root()`, shared by every ROOT and both verbs), R = the operator's ROOT.

### `ledger-all` (`ledger_all.rs:860-930`: Population V5, then Execution V3, then Selection V5; each stage walks all 8 rungs internally)

| stage (where) | input binding | crash between this stage and the next | rerun behaviour | verdict |
|---|---|---|---|---|
| Candidate Universe V1 (S, 16 families) | `universe_id` derives from data digest, feed, source commit, rung, family and span (candidate_universe.rs:174-191, :5504). It is keyed in `audits` at open (:3402) | complete blocks plus receipts | exact: `Reused` (:3735-3741). A receipt-less orphan of the exact retry is cut and rewritten (:3668-3709). Changed input: new block appended | a foreign orphan wedges the store (ledgerall-1, known). **The cumulative row ceiling wedges the store for good (conc10-1).** **The open→append window refuses a concurrent run (conc10-2)** |
| Base Evidence V2 (S) | the freshly reopened Candidate receipt (candidate_universe.rs:2677-2690; Base `validate_candidate_receipt`) | Candidate complete, Base absent | Candidate `Reused`, then Base written | **same cumulative ceiling (conc10-1)**, because Base bounds = Candidate bounds (step3_orchestrator.rs:4194-4197). **conc10-2** applies too: `require_unchanged` runs before the reuse check (population_base_evidence_ledger_v2.rs:1200-1209) |
| Pre-Admission V1/V2 (S) | Candidate and Base audits | complete | reused | one row per universe, so the row ceiling is not reached. Orphans: ledgerall-1 |
| Observation, Statistics V2, Lineage V4, Admission V3, Finalization V3, Population V5 (R/authority/<rung>) | each binds its predecessor's reopened Completion identity | complete predecessor blocks | exact: reused, and the run continues at the next stage. Rebuild: new blocks, and the old ones stay as history | sound between stages. Inside one stage: pop2-1..6 (known) |
| Execution V3 (R/execution/<rung>) | the in-memory committed Population V5 successor, reauthenticated before and after | Population V5 complete for all 8 rungs | Population reused for all 8, then Execution written | sound |
| Selection V5 (R/selection/<rung>) | the committed Execution V3 successor | n/a (last stage) | reused | ledgerall-2 and conc5-2 (known) |

### `ledger-v6` / `ledger-v6-replay` (`ledger_v6.rs:289-397`: one rung at a time; families go to S, then the route goes to R/<stage>/<rung>)

| stage | input binding | crash between this stage and the next | rerun behaviour | verdict |
|---|---|---|---|---|
| strict sizing | checksum receipts and span bindings (audited_range.rs:94-123). The shared month locks stay held | nothing committed but receipts | redone | locks: known (conc-pass3 locks row 3, ledgerv6-1) |
| Candidate, Base and Pre-Admission (S) | as above, through the same `commit_family_with_inputs_v6` (step3_orchestrator.rs:3060) | complete | reused | **conc10-1, conc10-2** |
| Statistics V3, Lineage V4, Admission V4, Finalization V4, Population V6, Execution V4 (R/<stage>/<rung>) | each binds its predecessor's reopened identity. Population V6 reauthenticates the Candidate source by a fresh `open_read` (step3_orchestrator.rs:693-715; stored_family_v6.rs:97-122), not by a retained generation | complete | exact: reused and continued. Rebuild: new blocks | sound between stages. Inside a stage: pop2-*, sel-1. Concurrent ROOT: pop2-7 |
| Selection V6 (R/selection/<rung>) | the committed Execution V4 successor | rungs 1..k complete | rungs 1..k "reused", and the run resumes at k+1 | quarantine wedge: conc4-1 (known) |
| Global Replay V4 (R/global-replay-v4/<publication>.bin) | the in-memory Selection V6 handles for all 8 rungs (ledger_v6.rs:688-691), plus the OOS store bytes. The file is content-addressed | partial publication file | byte-prefix compare, then append of the remainder (global_replay_v4_store.rs:30-71). A crash leaves a resumable prefix | sound. A zero-filled tail refuses that one publication (conc5-1 class) |

### Theme questions

1. **A later stage referencing a rolled-back, healed or quarantined record.** No path found. Every rollback cuts only bytes that have no receipt, and it does so under the exclusive lock:
   - `append_rollback.rs:43-56`
   - `fixed_tail::append_block` / `sync_or_roll_back`
   - `cut_orphan_for_rewrite`

   Successors bind only receipted Completions (CU-02), so nothing can name the cut bytes. Store months cannot be healed underneath a run, because the strict span keeps shared month locks held (known). The Selection V6 quarantine never reaches Global Replay, which takes in-memory handles.
2. **Partial chain.** Resume is exact at k+1 for both verbs; see the tables. A crash *inside* stage k is the known wedge family.
3. **Two chains.**
   - Same ROOT: pop2-7 (known).
   - Different ROOTs or verbs on one store: they share S. ledgerv6-2 (known) and **conc10-2** (new: the writers' own open→append window).
   - Two rungs at once: not possible within one process, because rungs are sequential.
4. **V5 next to V6.** No collision:
   - R/execution/<rung> holds `execution-*-v3.bin` and `execution-*-v4.bin`, with separate locks (execution_v3.rs:121-125; execution_v4.rs:125-129).
   - R/selection/<rung> holds `global-selection-*-v5` and `global-selection-v6.bin` (selection_v5.rs:97-99; selection_v6.rs:30).
   - No module in the chain runs `read_dir` on a stage directory.

   S is version-shared on purpose. **Its ceiling, however, is consumed by every version and build that ever runs on the store (conc10-1).**
5. **"Latest" by scanning.** None. Lookups are by `universe_id` or `authority_id` in maps built at open, and by content-addressed filename for Global Replay. Opens are O(F) full scans, which is documented (06-limits §150, §170 and the D-0918 section). For the S ledgers, F is the history of every run ever made on the store, which is the growth conc10-1 turns into a hard stop.

## New findings

### conc10-1 (medium): the shared store-root Candidate and Base ledgers have one *cumulative* 2^24-row ceiling across every run, rung, family, span, build and verb, so a store stops accepting any new Step-3 run for good once enough runs have accumulated
- **Site:** ledger_all.rs:1272-1276 (`CEILING_RECORDS`) and :1314:
  ```
  /// 2^24 records is sixteen million per ledger per rung, which is far more than
  /// a span of this store can produce, ...
  pub(crate) const CEILING_RECORDS: u64 = 1 << 24;
  ...
  candidate: CandidateUniverseBoundsV1::new(CEILING_RECORDS, CEILING_RECORDS)
  ```
  The ceiling is enforced against the file total at candidate_universe.rs:3753-3762:
  ```
  let desired_total = first_row.checked_add(receipt.row_count) ...;
  if desired_total > self.bounds.max_rows {
      return Err(format!("candidate append would produce {desired_total} rows, above maximum {}", ...
  ```
  It is enforced again at open (`record_count(.., self.bounds.max_rows, ..)`, :3367-3372). Base inherits the same bound (step3_orchestrator.rs:4194-4197) and enforces the total the same way (population_base_evidence_ledger_v2.rs:1258-1265, `require_bound("records", new_total, self.bounds.records)`).
- **Why it is wrong:**
  - The comment sizes the ceiling "per ledger per rung". The Candidate and Base ledgers are one file pair per **store**: `roots.source = crate::store_root()`, `candidate_ledger_paths(root)`.
  - Every universe ever committed adds rows: `closed_itemsets × (long + short cells per mask)` (candidate_universe.rs:5197-5203, :5267-5283). That holds for 8 rungs × 2 families per run, for every new span, every rebuild (the source commit is in the identity) and every data change (an open current month changes the data digest).
  - The file is append-only and never rotates. Once the total reaches 2^24, every universe that is not an exact reuse refuses, and so does every `ledger-all` and `ledger-v6` on that store, for any span and any ROOT.
  - A new ROOT does not help, because S is fixed by `BRUTEX_STORE`. Moving the file aside breaks the reopen of every earlier ROOT, because successors re-read the Candidate source (step3_orchestrator.rs:693-715).
  - The refusal is loud, so this is not a silent failure. It is a permanent capacity wedge reached through normal use.
  - **Extrapolation, not measured:** at the default five-step grid (up to 125 cells per side) and about 400 closed itemsets per family, one run writes about 16 × 400 × 250 = 1.6 M rows, so the ceiling falls after about 10 distinct runs. A larger `BRUTEX_GRID_RUNGS` (the ceiling is 16,384 cells per side, ledger_all.rs:221) reaches it within one or two runs.
- **Repro (ran):** a throwaway test in `candidate_universe::tests` (`prepared(30)` and `prepared(31)`, 4 rows each, with `CandidateUniverseBoundsV1::new(6, 1<<24)`). The first universe is written. The second, a *different* universe of 4 rows, refuses with `candidate append would produce 8 rows, above maximum 6`. The ceiling is therefore a lifetime file total, not a per-universe limit.
- **Minimal fix:** bound the **per-universe** row count with `max_rows`, and bound the file by bytes (64 GiB, or a size derived from the free disk). Alternatively, partition S's Candidate and Base files per rung and family (or per span), so that the existing per-ledger ceiling means what its comment says. Correct the comment in either case, and add a 06-limits line stating that S grows with every run and build.

### conc10-2 (low): the store-root writers release their open lock before taking the append lock. A second run whose open overlaps is refused as "stale" after its whole family preparation
- **Site:** candidate_universe.rs:3844-3851:
  ```
  let mut ledger = CandidateUniverseLedgerV1::open(root, bounds)?;   // EX lock: scan, then release (:3258-3345)
  let committed = ledger.append_complete(&produced.prepared)?;      // EX lock again: require_unchanged() (:3715)
  ```
  The Base equivalent is population_base_evidence_ledger_v2.rs:1318-1319 with `append_locked` → `require_unchanged()` at :1200. The Base check runs **before** the exact-reuse lookup at :1208.
- **Why it is wrong:**
  - `open` scans the whole shared file (O(F), with F the history of all runs) under the exclusive lock, then releases it.
  - A second process blocked in its own `open` gets the lock in that gap and scans in turn.
  - Whichever process appends second then finds the generation changed (len/mtime, :6469-6480) and refuses: `candidate file … changed since open; cached audit is stale`.
  - The ledger is append-only, so a grown file with the same inode is exactly the case a rescan under the lock could absorb.
  - The losing run has already paid for its whole universe preparation (sweep plus grids). The rerun recovers, but repeats that preparation.
  - In Base, even a pure reuse refuses.
  - This trigger is not covered by pop2-7 or ledgerv6-2: those name retained *readers* and per-ROOT Admission V4. This is the S writers' own open→append gap.
- **Repro (not run):** two `ledger-v6` runs on one store with different ROOTs and spans, timed to finish a family's preparation within one open-scan duration of each other. A's `open` scans; B blocks; A releases; B scans; A appends; B appends and refuses as stale.
- **Minimal fix:** keep the exclusive lock from `open` through `append_complete` (and through Base `append`) on the writable path. Alternatively, inside `append_*_locked`, when the generation differs but (dev, ino) is unchanged and len grew, re-run `scan()` under the held lock instead of refusing.

## Verification tally (known ids re-seen this pass, state at 1f4de71)
- ledgerall-1: NOT FIXED. The `cut_orphan_for_rewrite` foreign-id refusal is still at candidate_universe.rs:3676-3681.
- pop2-7: NOT FIXED. No ROOT-level run lock in ledger_all.rs or ledger_v6.rs.
- conc-pass3 locks row 3 / ledgerv6-1: NOT FIXED. `RangeGuard` still keeps every `AdmittedMonth` (shared month lock) until terminal sealing (audited_range.rs:36-55, store file.rs:1635).
- Everything else touched (pop2-*, sel-1, conc4-1, conc5-*, ledgerall-2/3) was cited, not re-verified.

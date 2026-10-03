Verdict: 3 findings at 331b05c (0 high, 1 medium, 2 low). The per-rung commit chain is ordered correctly: every successor binds a synced predecessor, and a crash *between* stages is always recovered by an exact rerun. The new defects are at the orchestration level: retained input handles across all eight rungs, a long-held store-root Base reader, and parent directories that are never synced. Crashes *inside* a stage are the pass-1 wedges, re-verified below.

### conc-pass2 / ledgerv6: `step3_orchestrator.rs` plus the `ledger-v6` and `ledger-v6-replay` verbs

Method: source only. No cargo was run and nothing in the repository was edited. Files read: `crates/cli/src/step3_orchestrator.rs` (route at :2268-2381, family commit at :3007-3170 and :3891-4221), `ledger_v6.rs`, `strict_v6_inputs.rs`, `stored_family_v6.rs`, `audited_range.rs`, `checksum_receipts.rs`, `selection_v6.rs`, `global_replay_v4_store.rs`, `population_base_evidence_ledger_v2.rs`, `population_v6.rs`, and the retained-generation and append code of each stage ledger.

## Commit order (per run)

`run_route` (`ledger_v6.rs:289-397`) walks `LEDGER_RUNGS` one after another, with no threads. For each rung:

0. `strict::size_sweeper` loads the NIFTY span under strict guards. It writes only checksum receipts and bindings.
1. For each family, NIFTY then BANKNIFTY, under the **store root**, which `ledger-all` and every other ROOT share:
   - Candidate Universe
   - Base Evidence V2
   - Pre-Admission V1, then Pre-Admission V2. An extinct family writes V2 only.

   Each is receipt-last and reopened (`step3_orchestrator.rs:4023-4121`).
2. `commit_stored_population_v6_route` (`step3_orchestrator.rs:2268-2381`) writes the stages below, under `ROOT/<stage>/<rung>`, in this order. Each is reauthenticated before the next reads it.
   1. **Statistics V3** (:2295)
   2. Search Lineage V4 (:2308)
   3. **Admission V4** (:2336)
   4. **Finalization V4** (:2358)
   5. **Population V6** (:2372)
   6. **Execution V4** (:2376)
3. **Selection V6**, in `render_selection` → `commit_stored_selection_v6` (`ledger_v6.rs:587`) → `ROOT/selection/<rung>/global-selection-v6.bin`.

`ledger-v6-replay` repeats the whole route (`replay_route` → `run_route`, `ledger_v6.rs:690`). After all eight rungs it then writes Global Replay V4 under `ROOT/global-replay-v4/<publication>.bin`.

The task named the order Population V6 → Statistics V3 → …. That is not the code's order. Statistics V3 commits first and Population V6 fifth.

## What a crash between two stages leaves

| Process dies after… | On disk | Exact rerun (same binary, same inputs) | Rerun after a rebuild |
|---|---|---|---|
| family ledgers, before Statistics V3 | complete Candidate, Base and Pre-Admission blocks under the store root | every family ledger reuses its block, and Statistics V3 is written | the commit digest changes the identities, so new blocks are appended. The old blocks stay as complete, unreferenced history. |
| Statistics V3, before Lineage V4 | complete Statistics block | reused, and the route continues | new blocks |
| Lineage V4, before Admission V4 | complete Lineage block | reused | new blocks |
| Admission V4, before Finalization V4 | complete Admission block | reused | new blocks |
| Finalization V4, before Population V6 | complete Finalization block | reused | new blocks |
| Population V6, before Execution V4 | complete Population block | reused | new blocks |
| Execution V4, before Selection V6 | complete Execution block | reused, and Selection is written ("written") | new blocks |
| Selection V6 of rung *k*, before rung *k+1* | rungs 1..k complete | rungs 1..k print "reused", and the run continues at k+1 | new blocks |
| all 8 Selections, before Global Replay (replay verb) | 8 complete Selections | all reused, and replay is written | new blocks |

In every row the successor binds the predecessor's synced identity. No successor block can exist without its predecessor's Completion. The identities are deterministic (seeded bootstrap; no map iteration or clock reaches the bytes), so an exact rerun converges.

The unsafe cases are crashes **inside** one stage's append. These are pass-1 pop2-1/2/3/4/6, sel-1 and hunt-cli-a-5, re-verified below. A torn tail, or a whole-record orphan followed by a rebuild, wedges that rung's stage root for every later span.

## Two runs on one ROOT (and on two ROOTs)

- **Same span, same binary, concurrent.** Each stage's writer takes an exclusive flock. The second process finds the exact block and reuses it, without writing. No retained generation is disturbed, because reuse changes neither length nor mtime. This case is safe.
- **Different span, or a different binary, on the same ROOT.** This is pass-1 **pop2-7**. Confirmed, and its window is wider than reported (see the verification section below).
- **Different ROOTs, or `ledger-all` running beside `ledger-v6`.** These still share the store-root family ledgers. **ledgerv6-2** is a refusal that a per-ROOT lock (pop2-7's fix) does not prevent.

---

## ledgerv6-1 (medium): every rung's strict source handles and flocks are kept open until the run ends, so a multi-month `ledger-v6` runs out of file descriptors part-way and fails the same way on every rerun

**Where:**
- `ledger_v6.rs:307` and `:393`: `committed.push(selection)` keeps each rung's `CommittedStoredSelectionV6` until `run_route` returns. In replay, they are kept until Global Replay ends.
- `step3_orchestrator.rs:4242` (`bind_stored_population_v6`): `source.retain_strict_inputs(inputs)?;`
- `population_v6.rs:2606-2611`: the struct that holds those inputs.

The retention chain, with the code at each link:
- `selection_v6.rs:76`: `source: CommittedStoredExecutionV4`
- `execution_v4.rs:3513-3515`: `source: CommittedStoredPopulationV6`
- `population_v6.rs:3211-3213`: `upstream: PopulationV6ProductionSourceV1`
- `population_v6.rs:2609`: `strict_inputs: crate::step3_orchestrator::strict::Guards`, which holds `Arc<Inputs>`
- `strict_v6_inputs.rs:14-17`: `guard: RangeGuard`
- `audited_range.rs:36-41`: `sources: Vec<AdmittedMonth>, bindings: Vec<Receipt>`
- `checksum_receipts.rs:46-51`: `source: AuditedBarFile, receipt: Receipt`
- `store/src/file.rs:1444-1450`: the audited open keeps a shared-flocked `.lock` handle, the `bars` handle and the checksum handle. `Receipt` keeps one more shared-flocked handle (`checksum_receipts.rs:274`).

**Why it is wrong:** for one family on one rung with an M-month span, `Loader::load` (`audited_range.rs:190-233`) audits:
- the prior minute month and the prior daily month;
- per month, the minute, daily and (for rungs other than 1min) signal files.

Each audited month holds about 4 descriptors: lock, bars, crc and receipt. Each month also holds one span-binding `Receipt`. Two families are retained for each of the 8 rungs, so at the last rung the process holds roughly 2·[(8+9M) + 7·(8+13M)] = 128 + 200M descriptors. On top of that come the retained stage-ledger handles, which are several per stage per rung, plus the current rung's sizing guard. **This is an extrapolation from the code; I measured nothing.**

Rust std does not raise `RLIMIT_NOFILE`. The repo has no `setrlimit` (grep finds no `RLIMIT`). The common Linux soft limit is 1024, which this estimate passes at about M ≥ 5.

From that point the next `open` fails with `EMFILE`. That happens inside a late rung's family load or stage commit, after the earlier rungs have committed durably. The refusal reads "Too many open files" and gives no cause.

A rerun reuses the earlier rungs, but it reopens and keeps exactly the same handles. It therefore fails at the same rung every time, and `ledger-v6-replay`, which requires all eight Selections at once (`ledger_v6.rs:688-691`), can never run for that span. `docs/06-limits.md` does not mention the limit.

A second effect, cross-process, comes from the same retention. Every source month in the span stays shared-flocked for the whole run, which can take hours across 8 rungs. The store writer's non-blocking exclusive `try_lock` refuses for the whole run any pull, repair or backfill that touches those months.

**Repro (extrapolated):** `ulimit -n 1024; cli ledger-v6 V 2024 1 2024 6 SUPPORT PTS ROOT`. The estimate for M = 6 is 1,328 source descriptors at rung 8. Expected result: rungs 1..~6 commit, then `refused: v6 <rung> … Too many open files (os error 24)`. The identical command fails at the same rung again.

**Minimal fix:** once a rung's Selection V6 has been published and reauthenticated, stop carrying the strict guards forward. For example, `CommittedStoredSelectionV6` could keep only its identity, with `ledger-v6` dropping the rest, while replay reopens the guards it needs. Alternatively, raise the soft `RLIMIT_NOFILE` to the hard limit at CLI start and refuse up front with a computed descriptor budget when the span exceeds it. At minimum, record the per-month descriptor cost in `docs/06-limits.md`.

---

## ledgerv6-2 (low): the paired Base Evidence reader is opened at the start of the route and kept through Statistics V3 production; any Base append to the shared store-root ledger in that window refuses Admission V4

**Where:**
- `step3_orchestrator.rs:2280-2281`: `let (_base_evidence, mut base_reader) = family_v6::reopen_pair(&nifty_source, &banknifty_source)?;`
- The reader is first used at `:2319-2327` (`prepare_population_admission_v4(…, &mut base_reader, …)`), through `population_admission_v4.rs:679-680` (`base.candidate(candidate.sequence())`).
- The check sits in `population_base_evidence_ledger_v2.rs:1111-1122` (`with_shared_lock` → `require_unchanged`) and `:1258-1266`. These compare the root, lock, record and completion generations (length, inode, mtime, ctime) captured at open.

**Why it is wrong:**
- Between the open and the first read, the route runs three steps. Each can take a long time, and none of them uses the reader:
  1. the whole Statistics V3 production (1,000 bootstrap draws, `ledger_all.rs:1294`) and its commit;
  2. the Search Lineage V4 commit;
  3. the start of Admission preparation.
- The Base Evidence files `base-evidence-{records,completions}-v2.bin` live in the **store root**. Every `ledger-v6` (any ROOT), every `ledger-all` (`all_rung_population_v5.rs:584/608` → `commit_family_base`) and every family commit append to that root.
- `root_generation` also covers the store-root directory's own mtime and ctime. So even creating a new entry directly in the store root, such as a first-time lock or ledger file, trips the check.
- Any such append or create inside the window makes A's reader stale. A then refuses `Step 3 Population Admission V4 preparation refused: … base-evidence-records-v2.bin changed since open; cached Base authority is stale`, after A's Statistics V3 and Lineage V4 blocks have committed.
- pop2-7's proposed per-ROOT run lock does not prevent this, because the two runs need not share a ROOT.
- The binding `_base_evidence` is unused, so nothing requires opening the reader that early.

**Repro:**
1. Process A runs `cli ledger-v6 V 2024 1 2024 3 … ROOT_A`. It reaches rung 1min's `v6_statistics_adapter::produce` (`step3_orchestrator.rs:2292`).
2. Process B runs `cli ledger-all V 2024 4 2024 6 …` (or `cli ledger-v6 … ROOT_B` with another span). B commits a new family, so `commit_family_base` appends a Base block to the store root.
3. A finishes Statistics V3 and Lineage V4, then refuses at Admission V4 preparation with the stale-Base message.

A rerun of A recovers through exact reuse, so the harm is a spurious, misleading refusal. It reads as if the source data changed, and the run's work is wasted.

**Minimal fix:** call `family_v6::reopen_pair` immediately before `prepare_population_admission_v4`, after the Lineage commit, and drop the unused `_base_evidence`. Longer term, have the Base reader recheck only its own completed blocks' byte range plus the inode, not whole-file and directory mtimes.

---

## ledgerv6-3 (low): the stage, rung and global-replay directories are made with `create_dir_all` and never synced into their parents; each ledger syncs only its leaf

**Where:**
- `ledger_v6.rs:200-206` (`RungRoots::create`): `std::fs::create_dir_all(&path)` for `ROOT/<stage>/<rung>`, eight times per rung.
- `ledger_v6.rs:692-693`: `std::fs::create_dir_all(&root)` for `ROOT/global-replay-v4`.
- The ledgers sync only the directory they were given. Examples:
  - Admission V4 syncs its held root (pass-1 cites `:2583-2586`, `:2797`);
  - Selection V6 has `sync_directory(root)` (`selection_v6.rs:362-366`);
  - Global Replay V4 has `File::open(&root)…sync_all()` (`global_replay_v4_store.rs:67-69`).
- Nothing syncs `ROOT`, `ROOT/<stage>`, or the store root's ancestors.

**Why it is wrong:**
- POSIX makes a new directory entry durable only after its parent is fsynced.
- After a power loss, `ROOT/statistics` (or `ROOT/statistics/1min`) can be missing even though `ROOT/admission/1min` holds a synced Admission V4 block, and that block binds the Statistics V3 authority id. Whether this happens depends on the filesystem's metadata ordering; it is not something the code guarantees.
- Pass-1 noted this `create_dir_all` (concurrency.md:1543) but did not raise it. The same class is reported elsewhere as hunt-store-4 and xcut-3.

**Repro:**
1. Fresh ROOT.
2. `cli ledger-v6 …` commits rung 1min through Admission V4, which syncs `ROOT/admission/1min`.
3. Cut power before writeback commits `ROOT`'s entries.
4. On reboot, `ROOT/statistics` may be absent while `ROOT/admission/1min` holds a block that names a Statistics authority that no longer exists. Any audit of that rung refuses until the operator reruns. The rerun recreates the directory and re-appends identical bytes, which is why this is rated low.

**Minimal fix:** in `RungRoots::create` and `replay_route`, walk from ROOT down. Create each missing level with `create_dir`, then fsync its parent, in the style of `expression::attempt_directory` and the fix proposed for xcut-3.

---

## Pass-1 verification

- **pop2-7 (concurrent run invalidates retained Admission V4): CONFIRMED, and the window is wider than reported.**
  - `population_admission_v4.rs:2893-2908` compares the whole data-file generation, including `len`.
  - `ledger_v6.rs` takes no ROOT-level lock.
  - The same whole-file check exists for the retained Statistics V3 ledger (`population_statistics_v3.rs:2332-2333`, `:2375-2376`), Finalization V4 (`:2532-2544`), Population V6 (`:2277-2289`) and Execution V4 (`:2373-2380`, `:3162-3170`).
  - Execution V4's check is re-run on every Selection V6 read (`selection_v6.rs:141-150` → `execution_v4.rs:3609-3633`). In `ledger-v6-replay` that includes reads after all eight rungs, so a foreign append to any rung's stage files at any point in the multi-hour run makes the replay refuse. The whole run is the window, not only the Admission → Finalization gap.
  - The fix is unchanged. ledgerv6-2 is the cross-ROOT case this fix does not cover.
- **pop2-3 (sub-record tail wedges Admission/Finalization V4 for good): CONFIRMED.** `population_admission_v4.rs:2618-2625` refuses any non-stride length before any trailing logic runs. Population V6 has the same check (`population_v6.rs:2017-2019`), as pass-1 pop1 also reported.
- **pop2-4 (orphan prefix accepts only an exact retry that includes the build commit): CONFIRMED.**
  - `population_admission_v4.rs:2727` has `trailing.source != prepared.source`.
  - `BlockSourceV4.source_commit_digest` comes from `statistics.source_commit_digest()` (`:746`).
  - That digest comes from `hash(verified_commit.0.as_bytes())` (`step3_orchestrator.rs:3593`).
- **pop2-5 / pop2-6 (Statistics V3 has no root-directory fsync and no torn-header repair): CONFIRMED.**
  - `population_statistics_v3.rs` has no `root_file` or directory sync anywhere.
  - `:2827` initializes only when `len == 0`.
  - `:2845` refuses a short header.
- **sel-1 (Execution V4 `append_raw` has no rollback): CONFIRMED.** `execution_v4.rs:4931-4935` is exactly `file.seek(SeekFrom::End(0)).and_then(|_| file.write_all(raw))` with no `set_len`. It is reached from `step3_orchestrator.rs:2376-2378`.
- **sel "Step-3 root admission TOCTOU (acknowledged)": CONFIRMED as not a finding.** `AdmittedRootV1::require_same` is called before and after every family-ledger append (`step3_orchestrator.rs:4045`, `:4049`, and others).

## Checked and clean
- **Threads, atomics and locks in this slice:** none outside tests. The rung loop and the family loop are sequential.
- **Lock order across stages:** prepare (shared on the upstream ledger, then released) always happens before the downstream exclusive lock. No stage takes an upstream lock while it holds its own exclusive lock. Selection V6 derives its block before `persist` takes its lock (`selection_v6.rs:194-197`). No cross-process cycle exists.
- **Determinism:** the summary and report fields come from reauthenticated projections. No map iteration or wall clock reaches block bytes or identities in the route.
- **Global Replay V4 persist:** content-addressed by publication. A torn prefix of the same publication is completed. A different publication goes to a different file. It is fsynced before the seal and after it, and the leaf directory is synced. Its VIX lock collision is replay-5 in this pass's `replay.md` and is not repeated here.
- **Selection V6 concurrent identical runs:** writers are serialized by an exclusive flock and the exact block is reused. The partial-tail wedge is hunt-cli-a-5, already reported.

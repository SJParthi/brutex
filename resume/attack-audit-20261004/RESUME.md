# Attack audit (thread "Attack audit", new account) — resume state, 2026-10-04 13:45 UTC

GitHub beats this file. Thread decision ranges: F5 D-1643..1649/D-1800..1809, F6 D-1810..1819 (+D-2200..2219),
F7 D-1509..1519/D-1820..1829, F8 D-1830..1849 (+D-2240..2269), F9 D-1850..1859, F10 D-1860..1869 (+D-2220..2239),
hunter fixes D-2270..2279 (AHC-). Coordinator gave this thread D-2200..D-2299.

## Done
- F5 (GAP15-19, GAP15-17, AC-whp-tb-2, ET-7 cli halves; D-1643..1646) and F7 (D-1509 gate8 self-test, D-1510 body
  deadline, D-1512 pull-run leg repeats, D-1514 grid cost model V2, D-1515, D-1516; h-api-1/3 = already fixed on #74 by
  D-1580/D-1583/D-1552, entries D-1511/D-1513 say so) PUSHED to final/all-fixes as 8c9313c (fast-forward of 73441e5).
  Combined: 6,777 passed / 0 failed; fmt, clippy, static gates green. Mutation on 73441e5..8c9313c: runner/store/api
  0 missed; cli 9/63 done (0 missed, 1 slow timeout explained to PR #74 thread), rest left to CI Gate 18 (run 1269 on bb6b3b4).
- The 3 "unlanded" branches fix/cloud-GAP13-15, -GAP4-46, -W2-cli8-9 are on lane 1-b staging (final/all-fixes-00bbns): lane 1-b owns them.
- Hunter (pull/store/lake) done: h-pull-1 lake converted TIMESTAMP_MILLIS read as micros (medium), h-pull-2 TOTP base32 leftover bits (low). Report copied below.

## In flight (local worktrees, snapshots pushed)
- F10 + hunter fixes: wip/audit-fixes-10 (audit/f10, base c97ff00): 72d86f7 h-eng-1 D-1860, 0e63ac0 h-eng-2 D-1861,
  b7dfe6b h-cli-4 D-1854 (all validated, 0 mutants missed), then h-pull-1/h-pull-2 fixes (D-2270, D-2271) — fixer was
  finishing checks/mutation at save. NOTE: F10 fixed rollback in lineage V2/V3 while F8 DELETED those modules (D-1832):
  at integration keep the deletion and drop F10's V2/V3 hunks.
- F6: wip/audit-fixes-6 (audit/f6, base c97ff00): deb8100 W2-cli5-4 D-1810 (one admission rule served to the browser),
  e90c9d8 W3-runner5-0 D-1811, bce683e W3-runner2-8 column digest V2 D-1812; W3-runner2-7 likely closed by D-1514
  (agent to confirm). Was running mutation (api frontierjson) at save.
- F8: wip/audit-fixes-8b (audit/f8, base c97ff00): 46bbc6a W3-runner2-3/-5 D-1831, 12d6a25 W3-runner2-1 D-1833,
  05d0539 delete Search Lineage V2/V3 (W2-cli1-2/-3) D-1832, 1aad6af W3-runner4-1 D-1837 ... continuing on the Fix
  Board's list: cli-14 (+W2-cli11-0/-1, W2-cli12-3/-4, D-0934), W2-cli6-0 (D-1633), W2-cli16-2/-3 (D-1634),
  W2-cli15-2 (D-1636), W2-cli7-0 (D-1638), W2-cli10-2 (D-1639), W2-cli14-1/2/3 (D-1642), W2-cli5-3, then every
  documented row of resume/audit-20261003/out/f8-triage.md. Nothing documented-only.

## Next
1. Collect F6/F8/F10 reports; in one worktree off newest origin/final/all-fixes merge audit/f10, audit/f6, audit/f8
   (tails of docs/04,05,06: keep both sides with resume/audit-20261003/prompts/union.awk.md; check duplicate D-numbers).
2. fmt, clippy -D warnings, full workspace tests, static gates (script: extract each language-purity step with gate.sh,
   skip 1e; note the extractor runs past Gate 14's end into the next job: "Gates: command not found" is harmless),
   mutation per crate (cargo-mutants 26.2.0: --in-place cannot be combined with --jobs; cli mutants cost ~10 min
   each on this box, so leave cli to CI Gate 18 and tell the PR #74 thread).
3. Tell the PR #74 CI thread, fetch + merge its head, push once. Update /mnt/project-files/fix-board/status/attack-audit.tsv
   (id, state, commit, note) with commit hashes.
4. Disk on the cloud box is ~25 GB: delete finished target dirs (t5 and t10m were deleted to recover).

## PAUSE 2026-10-04 13:56 UTC (5-hour limit at 91%; resume ~17:03 UTC)
All agents stopped, no cargo running, worktrees clean. Heads pushed: wip/audit-fixes-6 = bce683e (F6: D-1810..1812 done;
was mid cli mutation, W2-cli5-4/W3-runner2-7 report not delivered), wip/audit-fixes-8b = 1aad6af (F8: 4 commits; rest of
the documented-only list still to do), wip/audit-fixes-10 = 1da2630 (F10 3 fixes + h-pull-1 D-2270 + h-pull-2 D-2271;
hunter fixes' mutation unfinished: one in-place mutant was reverted by hand). Restart: brief each fixer again from these
heads with the same prompts (in the thread's transcript; rules in resume/audit-20261003/out/FIXRULES.md).

## SAVE 2026-10-04 18:50 UTC (pause for 5-hour limit; resume ~22:03 UTC)
- wip/audit-batch2 = 6cf0582: origin/final/all-fixes fc6dbb9 + audit/f10 (h-eng-1 D-1860, h-eng-2 D-1861, h-cli-4
  D-1854, h-pull-1 D-2270 lake converted-type refusal, h-pull-2 D-2271 TOTP leftover bits). On merging fc6dbb9, lane
  1-b's D-1741 sweep-evidence rollback was kept and D-1854 keeps only the reservation half (test
  a_failed_reservation_leaves_no_file). Earlier batch-2 run on 1071b51: 6,832 passed / 0 failed, clippy + gates green.
  Final full test on 6cf0582 was running at save. NEXT: if green, hand wip/audit-batch2 to the PR #74 CI thread
  (coordinator: do NOT push final/all-fixes ourselves).
- F6 wip/audit-fixes-6 = 308e2d7 (D-1810..1812; local tree had 1 uncommitted file, likely an in-place mutant: check
  `git -C wt6 diff` and revert if it is a cargo-mutants marker). Report not delivered yet.
- F8 wip/audit-fixes-8b = 1f7da97 (D-1831..1837: W3-runner2-1/-3/-5, W3-runner4-1, W2-cli1-2/-3 deleted, W2-cli10-2,
  W2-cli7-0, W2-cli15-2, W2-cli16-3); 7 uncommitted files of WIP. Owns cli + runner rows.
- F8b (new) wip/audit-fixes-8c = fc6dbb9 base, 14 uncommitted files of WIP at save (not pushed). Owns api, pull, store,
  engine rows (D-2280..2299, AHD-).
- Tracker: /mnt/project-files/fix-board/status/attack-audit.tsv (64 rows fixing, owner in note).

## RESTART 2026-10-04 (container restart after batch 2 landed in PR #74 as merge 9e11023, head 25bc8aa)
- Batch 2 tracker rows set to pushed/9e11023.
- Worktrees survived. Snapshots: wip/audit-fixes-6b = ed6a7bf (F6, clean), wip/audit-fixes-8d = e9ac205 (F8 head
  17bf742 + D-1839 WIP as a stash commit; apply with `git stash apply e9ac205`), wip/audit-fixes-8e = 5a42e58 (F8b,
  D-2280..2290, clean). Fixers relaunched; they snapshot to wip/audit-fixes-6c / -8f / -8g as they commit.
- NEXT: batch 3 = merge F6 + F8 + F8b onto newest final/all-fixes, validate, hand to PR #74 CI thread.

## STOP 2026-10-05 01:56 UTC (weekly usage 95%; user moves to a new account)
All agents and cargo jobs stopped. Every branch below is on GitHub; nothing lives only on the cloud box.
| Fixer | Branch (resume from) | Head | Done | Left |
|---|---|---|---|---|
| F6 | wip/audit-fixes-6c | ed6a7bf | D-1810 W2-cli5-4, D-1811 W3-runner5-0, D-1812 W3-runner2-8 | W3-runner2-7: confirm closed by D-1514 (else fix as D-1813); copy-mode mutation of changed fns |
| F8 | wip/audit-fixes-8f | 7293587 + stash c249ffc (o1cli-6 WIP: `git stash apply c249ffc` or cherry-pick -n) | D-1831..1841 (latest: D-1839 o1cli-5, D-1840 o1cli-2/3/4, D-1841 W2-cli16-2) | o1cli-6 (WIP), AC-whp-o1-1, W2-cli8-6, cli-14 (W2-cli11-0/-1, W2-cli12-3/-4), W2-cli6-0, D-1638, W2-cli2-5, W2-cli14-1/2/3, ET-bars-candles-store-1/-8, rederive. Range D-1842..1849 |
| F8b | wip/audit-fixes-8g | 3e0ce95 | D-2280..2291 (all api/pull/store/engine rows re-examined, D-2290; /indexmap.json memo D-2291) | verify each row's evidence, full api/pull/store/engine/lake tests, allgates, copy-mode mutation. Range D-2292..2299 |
Batch 3 NOT yet handed to PR #74: merge all three onto newest final/all-fixes, fmt + clippy -D warnings + full tests + allgates, then hand the merged wip branch to the PR #74 CI thread (only that thread pushes final/all-fixes). Tracker: /mnt/project-files/fix-board/status/attack-audit.tsv (64 rows "fixing").

### Prompt for the new account (paste into a fresh Claude Code session on github.com/SJParthi/brutex)
Resume the brutex attack-audit fix queue. Read CLAUDE.md fully, then resume/attack-audit-20261004/RESUME.md on branch fix-queue, section "STOP 2026-10-05". GitHub state wins over the file. Finish the three fixers' remaining rows from their wip branches (F6, F8, F8b) with the same rules: Rust only, every cost fixed or measured (p50/p99/max) or argued inherent, nothing documented-only, a test for every change, decision + invariant entries in their stated D-ranges. Then merge all three onto the newest final/all-fixes into one branch wip/audit-batch3, run cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace --locked and the CI language-purity gates, fix any failure, and push only wip/audit-batch3 for merging into PR #74 (final/all-fixes). Do not open any other PR, never force-push shared branches, never push GDFL data. At 93% usage save state back to this file and stop.

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

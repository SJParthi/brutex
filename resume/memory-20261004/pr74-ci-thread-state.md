---
name: pr74-ci-thread-state
description: PR #74 CI-to-green thread at 2026-10-03 15:02 UTC - pushed head 0cab319, decision numbers used, local worktrees, what is pending
metadata:
  type: project
---
- 18:34 UTC: pushed 8a59ff4 (D-1461: store bench root, api ptr::eq test) after run 1261 on 0cab319 went green except Gate 8 and coverage. Resume file: fix-queue resume/pr74-ci-thread.md.
- 15:02 UTC 2026-10-03: pushed final/all-fixes 0cab319 (fast-forward from the sweep's a21d031). Local branch mut-all at /home/claude/mut-all mirrors it.
- Carried: Gate 18 survivor kills round 1 (D-1452) and round 2 (D-1454 api, D-1455 runner/vocab, D-1456 cli/pull); D-1453 resizes Gate 18 (ESTIMATED_CASE_SECONDS 540, max-parallel 20, ~78 jobs); D-1457 pins every workflow action to a commit SHA; D-1458/D-1459/D-1460 fix 8 static gates that were red on a21d031 (1d, 10, 11, 12, 14, 15, 21, 23).
- Gate 14 trap: since D-1601, `source_scan step-runs` refuses a needle line inside if/loop; gate 8's empty-bench proof is now a top-level `git ls-files --error-unmatch` line.
- CLI test failure in the audit run: not reproducible (passes in CI and 5 local runs); cause not recoverable.
- subscribe_pr_activity on #74 fails; use send_later check-ins.
- Next free D-number for this thread: D-1462 (and D-2000+ for new work).

**Why:** a fresh session must not reuse these numbers or redo the gate fixes.
**How to apply:** check the CI run on 0cab319 first; fix new Gate 18 survivors on mut-all, merge origin before one push. See [[brutex-ci-and-merge-gotchas]].

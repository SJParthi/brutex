---
name: brutex-resume-facts
description: Where Brutex resume state lives, decision-number ranges per workstream, and key branches (as of 2026-10-04)
metadata:
  type: project
  modified: 2026-10-04T07:09:34.698Z
---

- Master resume: `resume/RESUME-20261004.md` on branch `fix-queue` (never merged). Newest state is its §12 (FINAL SAVE 06:00 UTC 2026-10-04), then §10/§11; §5 has one ready prompt per workstream (5.1 PR #74 CI ... 5.8 Fix Board). GitHub state beats the file.
- Old project memory copy: `resume/memory-20261004/` on fix-queue. Per-workstream notes: `resume/pr74-ci-thread.md`, `resume/lane1b-redo/`, `resume/audit-20261003/NEXT.md`, `resume/workspace-audit-20261003/`, `resume/zero-audit-20261003/RESUME.md`, `resume/gdfl-build.md`, Fix Board source `resume/fix-board/brutex-fix-board.html` (builder lost; spec in `resume/memory-20261004/brutex-fix-board-artifact.md`).
- PR #74 base `main` 4bbd37d; head at resume c97ff00. CI ~6-8 h per push, so batch: one validated push per round; PR #74 CI thread is the merge gate for other workstreams.
- Staging branches: lane 1-b `final/all-fixes-00bbns`; zero-findings `final/all-fixes-zero`; sweep `audit-fix/w9`; attack audit `wip/audit-fixes-7b`, `wip/audit-fixes-5b`.
- **Decision numbers (Gate 27b refuses duplicates).** Old ranges: PR #74 CI D-1452..1463 used (D-1463 = Gate 6d arbitrary_precision fix, 73441e5); attack audit D-1480..1519, 1620..1659, 1800..1899; sweep D-1520..1619; lane 1-b D-1660..1759; zero loop D-1760..1799, D-1900..1999. **New-account blocks (coordinator, 2026-10-04 07:15 UTC):** PR #74 CI D-2000..2099; lane 1 D-2100..; attack audit D-2200..; Rust/O(1) sweep D-2300..; audit helpers D-2400..; zero-findings D-2500..2699 (+ invariant prefixes ZC/ZK/ZQ/ZX); Fix Board D-2700..; Mac GDFL D-2800..; WD Black space D-2900..
- Held on the user (list once, never guess): gaps-6 split threshold, gaps-7 F&O membership source, hunt-costs-5/hunt-runner-5 charter sources, gaps-8, hunt-ci-1 branch protection, testgaps-7 operator data, numeric pst-3/clib-1/clib-2, docs/07-plan §3 operator items, GAP12-6 cash-auction schedule.

Related: [[user-rules]], [[brutex-ci-gotchas]].

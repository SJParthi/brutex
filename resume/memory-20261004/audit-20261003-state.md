---
name: audit-20261003-state
description: Attack audit of final/all-fixes (1087e544), 2026-10-03 - verification results, fix branches, decision ranges, what is left
metadata:
  type: project
  modified: 2026-10-03T09:45:32.380Z
---
Reports: fix-queue branch resume/audit-20261003/out/ (v1 v2 v3a v3b v53 v4 fold c4a c4a-verdicts c4b). RESUME.md there has the live state.

Verified (2026-10-03): lane1 79 (55 NOT FIXED, owned by the lane1-b redo thread); lane2 82 (73F 4D 5P); lane3 89 (58F 27D 3P 1NF, GAP17-33 is a process item); audit-53 (43F 9D 1P); batch-1 64 groups (42F 2D 4P; 16 groups never fixed, their 60 original findings recovered from the Mac into fix-queue resume/lanes/c4-missing-findings.md). Fold of PR #74 did no damage.

Fixes (local branches mirrored as wip/audit-fixes, -2, -3, -4; combined as audit-combined): D-1480..1488 (new findings), D-1490..1502 (C4 non-cli, invariant prefix AFX), D-1620..1642 (C4 cli, FX3), D-1503..1519 (lane2/3 leftovers). Pushed to final/all-fixes ONCE after local cargo-mutants 26.2.0 on changed lines (PR #74 CI thread asked: every push restarts Gate 18).

Needs the user's call (left documented, not changed): GAP15-19 (restoring a ceiling changes Global Replay results), GAP15-17 (rounding change needs a new Base Evidence version), W2-cli5-4 (cli+api+web change), W3-runner2-8 (column digest change alters stored digests).

**Why:** a fresh session must not redo verified work or reuse these decision numbers.
**How to apply:** resume from wip/audit-* and RESUME.md; see [[lane1b-redo]] for other threads' ranges.

---
name: resume-20261003-pr74-state
description: State of the one final PR #74 (final/all-fixes) and every lane at the 2026-10-03 ~04:00 UTC stop, from fix-queue resume/RESUME-20261003.md
metadata:
  type: project
---
Source: branch fix-queue, resume/RESUME-20261003.md (read 2026-10-03). GitHub PR state wins over these notes.

- **PR #74**, branch `final/all-fixes`, head `1087e544` at stop. Marked Ready with squash auto-merge ON; merges itself when required check `ci-ok` is green. Repo allows squash only; branch protection refused a direct merge. Claude cannot merge or enable auto-merge itself; `.github/workflows/auto-merge.yml` merges a Ready PR once every check is green.
- PRs #21, #23-#73 are all CLOSED "Folded into #74". Every lane's work (lane 2 `lane2-wip/for-final` b36350a, lane 4 #35 head e6518e38, all lane-3 `fix/cloud-*` heads) is an ancestor of #74. D-1450 = renumber map + rival-fix choices.
- CI at stop: run 1258 (id 37092404876) on 1087e544: Gate W and Gate 1+2 green; Gates 3-6 running (fmt/lock/clippy passed). Full CI takes ~6 h (Gate 1e ~80 min, coverage 3-5 h). If #74 sits queued, cancel CI runs on branches of closed PRs.
- Lane 1 check-in trigger `trig_016etFn8hbMCwnKXj7bpizZp` was DISABLED for the stop; re-enable or recreate on resume.
- **Lost in the 2026-10-03 00:00 container restart:** lane 1's lane1-b work. Restart from `fix-queue/lane1-b.md`; new fixes go on branches merged into final/all-fixes (or one new PR if #74 merged). See [[audit-20261003-state]].
- Lanes 2, 3, 4: no open work; their deliberate deferrals are listed in the resume file.
- Decision-number ranges: lane 1 D-0910..0919, D-0960..0999 (and D-14xx used for fold, last D-1451); lane 2 D-0920..0939, D-1100..1199; lane 3 D-0940..0959, D-1200..1299 (next free above D-1204); lane 4 D-1300..1399; Mac D-08xx, D-1000..1099. Gate 27b refuses duplicate D-numbers.
- Status board artifact (old account): https://claude.ai/artifact/52q7dK65WWtSXsYcc2yF4F; #74 table: https://claude.ai/artifact/NPAVGrepQksZvVJeBNi7jC

**Why:** previous account stopped at 99% weekly usage; fix-queue branch is the only handoff.
**How to apply:** check #74 on GitHub first; see [[brutex-ci-and-merge-gotchas]] before touching final/all-fixes.

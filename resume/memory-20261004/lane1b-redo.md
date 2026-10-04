---
name: lane1b-redo
description: How the 2026-10-03 redo of the 55 missing lane 1-b fixes is organised (branches, D-number ranges, merge target)
metadata:
  type: project
  modified: 2026-10-03T04:05:51.201Z
---

Redo of the 55 lane 1-b findings that the 2026-10-03 audit found NOT fixed on final/all-fixes (PR #74, head 1087e54).

- Source lists: fix-queue branch `fix-queue/lane1-b-plan.md` (units U1..U41) and `resume/audit-20261003/out/v1.md` (current evidence). U1, U2, U14, U21 already landed.
- Staging branch: `final/all-fixes-00bbns` (from final/all-fixes). Five worker groups A-E commit on local branches lane1b/a..e, merged into the staging branch, then merged (never force-pushed) into `final/all-fixes` if #74 is still open, else one new combined PR from the staging branch.
- Decision numbers reserved for this redo: D-1660..D-1759 (D 1660-79, E 1680-99, A 1700-19, B 1720-39, C 1740-54); the codebase-sweep thread owns D-1520..D-1619 and invariant prefixes AFA..AFE, the attack-audit thread D-1480..1519 and D-1620..1659. Other threads should not use them.

**Why:** the user allows only one combined PR; another thread drives #74's CI and also pushes to final/all-fixes, so always fetch+merge before pushing.
**How to apply:** if resuming, check those branches and the D-16xx entries in docs/05-decisions.md before redoing anything. Related: [[brutex-project]]

Fix Board: per-finding state lives in /mnt/project-files/fix-board/status/lane1.tsv (id, state, commit; state = found|fixing|branch|pushed|green). Update it when a group merges, when the one push lands on final/all-fixes (pushed), and when CI is green (green).

State 2026-10-03 18:45 UTC: staging final/all-fixes-00bbns (90de0cc) has A, C, D, E merged and verified (fmt, clippy, tests, all 28 static gates); B not merged (follow-ups); mutation testing not done; nothing pushed to final/all-fixes yet. Full resume: fix-queue resume/RESUME-20261003.md section lane1b-redo.

FINAL 2026-10-03 20:15 UTC: all 5 groups merged on staging bacd5a2 (pushed); fmt+clippy clean; full test cut by timeout (1,374 pass, 0 fail). Nothing on final/all-fixes yet. Next steps in fix-queue resume/RESUME-20261003.md section lane1b-redo.

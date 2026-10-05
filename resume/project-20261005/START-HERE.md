# START HERE (project snapshot 2026-10-05)

Contents: `memory/` (project memory), `fix-board-status/` (Fix Board status TSVs), `SPACE-PLAN.md`.
Not copied: zero-rounds and audit-20261003-workspace (threads saved their own state).

## Per-workstream resume files on fix-queue (priority order)
1. **PR 74 CI**: `resume/pr74-ci-thread.md`. The only workstream that pushes to `final/all-fixes`.
2. **Data-path attack**: `resume/data-path-attack-20261004/RESUME.md`. Main branch `claude/attack-data-pipeline-hgxmw9` @0368dbb still needs a full test run; GDFL `claude/attack-gdfl` validated only up to bda0464.
3. **Fix Board**: `resume/fix-board/RESUME-fixboard.md`. `fixboard/pr74-ce2` ccd37ec8 is pushed; merge with PR74 head, re-test, then hand off to PR 74 CI.
4. **Attack audit**: `resume/attack-audit-20261004/RESUME.md`, section "STOP 2026-10-05".
5. **Zero-findings**: `resume/zero-rounds/NEXT-SESSION.md`.
6. **GDFL (Mac only)**: `resume/gdfl-build.md` plus `RESUME-GDFL-20261005.md` in the Mac project folder.

## Standing rules (short; full text in memory/user-rules.md)
- One PR only: #74 (`final/all-fixes`). Merge commits only; never force-push or rebase shared branches.
- Rust only, except `web/`.
- O(1) wherever possible; name exceptions; measure p50/p99/max.
- Real evidence only, no guessing; attack edge cases adversarially; work in parallel.
- Fix every finding fully; repeat audit rounds until a round finds zero.
- Never ask the user to tap, paste or approve; keep working.
- Usage: save resume state to `fix-queue` `resume/` at 93%; stop at 98%.
- Deliver results as comparison-table Artifacts.
- GDFL: never push real GDFL data; build only on the user's Mac; never touch brutexOld or delete raw data.

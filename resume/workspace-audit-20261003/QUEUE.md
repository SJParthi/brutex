# Workspace-audit fix queue (paused 2026-10-03 13:20 UTC for the 5-hour usage limit)

Rule until 18:00 UTC: one agent at a time, nothing new launched.

Pushed: a21d031 on final/all-fixes (67 fixes + D-1592). Full suite 6680 passed / 2 failed before the two clash fixes; not re-run since.
Running: mut1 (branch audit-fix/mut1, survivor-killing tests). Only agent.

Queued, WIP pushed to origin, unfinished and untested. Resume each in its worktree with scratchpad FIXRULES2.md rules (copy below):
1. audit-fix/w6 (api): hunt-api-2 sweep cancel flag, hunt-api-3 same-origin log cap, errpaths-4 private Widths fields. D-1551..1555, AFF-01..19.
2. audit-fix/w7 (cli): hunt-conc-1 range-all/pool order, hunt-conc-2 ordered phases, hunt-cli-a-5, o1surface2-1. D-1556..1559, D-1569, AFF-20..39.
3. audit-fix/w8 (data/engine): attackdata-4, attackdata-8, o1eng2-1, gaps-1, gaps-3. D-1570..1575, AFF-40..59.
4. audit-fix/w9 (features, not started): gaps-5, gaps-11, gaps-10. D-1576..1579, D-1593..1594, AFF-60..79.
Free D: D-1595..1599, D-1615..1619.

Done elsewhere: hunt-ci-12 by the CI thread (D-1457, eec9c64).
Blocked on facts or the owner: gaps-6 threshold, gaps-7, gaps-8, hunt-costs-5, hunt-runner-5 (charter sources missing), hunt-ci-1 (branch protection setting), testgaps-7 (operator data), rustonly2-10 (no defect).
Before pushing a batch: send the commit to the CI thread cse_01TPJRnnkg5yuNeRBHDyzaaP first. Gate 15: never spell another language's name.

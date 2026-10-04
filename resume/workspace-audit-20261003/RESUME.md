# FINAL SAVE (2026-10-04 06:00 UTC, account switch). Nothing is running.

Exact heads at the save:

| Branch | Head | Meaning |
|---|---|---|
| final/all-fixes (PR #74) | c97ff00 | Carries all 78 fixed findings. Verified locally before the push: 6743 passed, 0 failed, 12 ignored; fmt, clippy, cargo deny 0.19.0 and 29 gates green. CI re-runs there, and the "PR 74 CI and CLI test" thread watches it. |
| audit-fix/w9 | 9e829f4 | gaps-5 (pool-oos out-of-sample judging), gaps-11 (catalog handoff) and gaps-10 (Selection V6 route and page), D-1576..D-1578. Built on b23976f. The worker was stopped while its full test suite was still running, so it is **NOT verified**. |
| audit-fix/integrate-20261004 | c97ff00 | Same commit as PR #74; a backup. |

Next steps for the new account:
1. Check PR #74 CI on c97ff00 (or newer) and fix anything red that this batch caused. Gate 18 sees new code in api, cli, store, pull and runner.
2. Finish gaps-5, gaps-10 and gaps-11, in a worktree off origin/final/all-fixes:
   - Merge origin/audit-fix/w9; keep both sides at doc tails (keepboth.md).
   - Read the D-1576..D-1578 entries and the code, and complete anything half-wired.
   - Run fmt, clippy -D warnings, `cargo test --workspace --locked`, web tests under web/tests, and Gate 15 (no other language names).
   - Send the commit to whoever drives PR #74, then push HEAD:final/all-fixes. Never force-push and open no PR.
3. The 10 blocked findings need the owner (tables below). Everything else is done.
4. Update STATUS.md here and republish the audit page from brutex-workspace-audit.html.md (rename it to .html; a new account publishes a new URL).

# LATEST (2026-10-04 05:35 UTC): resumed on request, 2 agents running

| Item | State |
|---|---|
| PR #74 head | b23976f (pushed by other threads overnight) |
| Fixed | 78 of 91, ALL on PR #74 (final/all-fixes c97ff00, pushed 2026-10-04 ~05:50 UTC; verified 6743 passed, 0 failed, 12 ignored; fmt, clippy, deny and 29 gates green locally) |
| Features (gaps-5, gaps-10, gaps-11) | Being built on **audit-fix/w9 @ 3f78c29** (WIP, untested), based on b23976f |
| Blocked (10) | Unchanged; see the table below |

Next: PR #74 CI on c97ff00 (the CI thread watches it). Finish w9 features, verify, send the commit to the CI thread, push.

# Workspace audit + fixes: FINAL SAVE (2026-10-03 20:12 UTC, stopped at 93% weekly usage)

**Start here.** Read in this order:
1. PROMPTS-AND-BRIEFS.md: the user's words, the coordinator's rules and every agent brief.
2. STATUS.md: all 91 findings.
3. FIXRULES2.md: worker rules.

Artifact source: brutex-workspace-audit.html.md. Rename it to .html and publish it as a new Artifact; the old URL https://claude.ai/artifact/YYYZhcv7YjfW5txZL12Ki1 belongs to the old account.

## Where things stand

| Item | State |
|---|---|
| PR #74 (final/all-fixes) | 0cab319: this thread's a21d031 (67 fixes) plus the CI thread's mutant kills and action pins (hunt-ci-12) |
| Fixed | 78 of 91 |
| Pushed | 68 |
| Done on branches, NOT yet on PR #74 (10) | see the branch table below |
| Not started (3) | gaps-5, gaps-10, gaps-11 on audit-fix/w9 (the brief is in PROMPTS-AND-BRIEFS.md) |
| Blocked (10) | see the blocked table below |

Branches done but not on PR #74:

| Branch | Findings | Checks |
|---|---|---|
| audit-fix/w6 @ 700e644 | hunt-api-2, hunt-api-3, errpaths-4 | fmt, clippy and tests green on indicators, runner, cli, api, core, store |
| audit-fix/w7 @ 858c8bb | hunt-conc-1, hunt-conc-2, hunt-cli-a-5, o1surface2-1 | cli 1755 pass, clippy clean |
| audit-fix/w8 @ c6d03c6 | attackdata-4 (D-1570), attackdata-8 (store format v3, D-1571), o1eng2-1 (D-1572) | The worker was stopped before its final report, so fmt, clippy and tests on store, pull, runner and their dependents must be run and confirmed first |

Blocked findings:

| Finding | What is missing |
|---|---|
| gaps-1, gaps-3 | An owner decision on wiring the superseded Step-3 V1-V4 and BH/walk-forward modules (D-1568, D-1544) |
| gaps-6, gaps-7, gaps-8, hunt-costs-5, hunt-runner-5 | A charter source for the fact |
| hunt-ci-1 | The owner turning on "Require review from Code Owners" |
| testgaps-7 | Operator market data |
| rustonly2-10 | Nothing: it is not a defect |

## Next steps
1. In a worktree off origin/final/all-fixes, merge audit-fix/w6, w7 and w8 with merge commits. For doc-tail conflicts, keep both sides (keepboth.md).
2. Verify w8 first. Run fmt, clippy -D warnings and `cargo test --workspace --locked`, and run root-only store tests as uid 65534. Check Gate 15: no other language names in the diff.
3. Coordinate with the PR #74 CI thread, or whoever now drives PR #74. Then push HEAD:final/all-fixes. Never force-push and open no PR.
4. Run the w9 brief (features).
5. Update STATUS.md and republish the Artifact.

---
# History (earlier state, kept)

# Workspace audit + fixes (thread "workspace audit", started 2026-10-03)

Audited: final/all-fixes @ 1087e54 (PR #74). Artifact: https://claude.ai/artifact/YYYZhcv7YjfW5txZL12Ki1
Reports: this folder (*.md, one per worker). 91 NEW findings: 1 high, 28 medium, 50 low, 12 info.
Gates at 1087e54: fmt clean, clippy -D warnings clean, tests 6565 pass / 4 root-only fail (pass as uid 65534) / 12 ignored.

## Fix phase (started 05:30 UTC)
Five workers, local worktrees /home/claude/fix-w1..w5 on branches audit-fix/w1..w5 off 1087e54.
Backups pushed to origin as audit-fix/w1..w5 (no PRs; user rule: one combined PR only).
Rules every worker follows: FIXRULES.md in this folder.
- w1 store/lake/pull/costs/telemetry: D-1520..1539, invariants AFA-*
- w2 runner/indicators + high gaps-6 (splits): D-1540..1559, AFB-*
- w3 cli: D-1560..1579, AFC-*
- w4 api/web: D-1580..1599, AFD-*
- w5 CI/gates/test gaps: D-1600..1619, AFE-*
Other lanes' ranges: attack audit D-1480..1519, D-1620..1659; lane 1 D-1660..1759.

## To resume
1. git fetch origin 'audit-fix/*' ; check which findings each branch fixed (commit messages cite audit ids).
2. Restart any unfinished worker from FIXRULES.md + its list in this file's history.
3. Merge w1..w5 into a branch off origin/final/all-fixes (merge commits, keep both sides at docs tails),
   run fmt, clippy -D warnings, cargo test --workspace --locked (root-only store tests: re-run as uid 65534),
   then git push origin HEAD:final/all-fixes. Never force-push. No new PR.
4. Republish the artifact with FIXED/SKIPPED per row.

# Workspace audit + fixes: CURRENT STATE (updated 2026-10-03 18:55 UTC)

**Start here.** Read in this order:
1. PROMPTS-AND-BRIEFS.md: the user's words, the coordinator's rules and every agent brief.
2. STATUS.md: all 91 findings, with status and commit.
3. QUEUE.md: what to run next.
4. FIXRULES2.md: worker rules.

Artifact (audit page, has the tail-latency tab): https://claude.ai/artifact/YYYZhcv7YjfW5txZL12Ki1. The source is brutex-workspace-audit.html.md here; rename it to .html before publishing. A new account can't update that URL, so it must publish a new one.

## Where things stand

| Item | State |
|---|---|
| PR #74 (final/all-fixes) | 0cab319 (CI thread push), which carries this thread's a21d031 (67 fixes, D-1520..D-1592) and the mutant-kill tests |
| Before a21d031 | full workspace tests: 6680 passed, 2 failed; both were clashes, fixed in a21d031 and passing alone |
| Fixed | 75 of 91 |
| Pushed | 67 + hunt-ci-12 (CI thread, D-1457) |
| On a branch, not pushed | 3: hunt-api-2, hunt-api-3, errpaths-4 on origin audit-fix/w6 @ 700e644. Verified: fmt, clippy, tests green on indicators, runner, cli, api, core and store |
| Done on branch, not pushed (2) | audit-fix/w7 @ 858c8bb: hunt-conc-1, hunt-conc-2, hunt-cli-a-5, o1surface2-1 (cli tests 1755 pass, clippy clean) |
| In progress | audit-fix/w8 (WIP d2ed52d, being finished) |
| Not started | audit-fix/w9 (gaps-5, gaps-10, gaps-11) |
| Blocked, cannot be fixed by code | gaps-6 (split-refusal threshold), gaps-7 (point-in-time F&O membership), gaps-8 and hunt-costs-5 (charge rates): the charter has no source, and CLAUDE.md §3 rule 1 forbids invention. hunt-runner-5: no sourced HAC bandwidth. hunt-ci-1: needs the owner to turn on "Require review from Code Owners" in branch protection. testgaps-7: the ignored tests need operator market data. rustonly2-10: macOS-only dependency, not a defect |

## Next steps (one agent at a time while usage is tight)
1. Finish w8, then w9, each in its own worktree from its origin branch, following FIXRULES2.md.
2. Merge w6, w7, w8 and w9 into a branch off origin/final/all-fixes. For doc-tail conflicts, keep both sides (keepboth.md).
3. Run fmt, clippy -D warnings and `cargo test --workspace --locked`. Run root-only store tests as uid 65534.
4. Tell the PR #74 CI thread the commit, then push HEAD:final/all-fixes. Never force-push and open no PR.
5. Update STATUS.md and /mnt/project-files/fix-board/status/sweep.tsv, then republish the Artifact.

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

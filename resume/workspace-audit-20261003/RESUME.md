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

# PAUSE (2nd, 08:07 UTC) — lens L3 permutations (attack/permutations) — 2026-10-06

Head: `attack/permutations` @ `09a9793` (pushed at pause).
Supersedes PAUSE-permutations.md (1st pause).

## Done
- Round 1: F-25A074 FIXED (pattern midpoints, D-3402, XPERM-02, f9ab789). Refuted: VWAP floor, SuperTrend seed past warm (D-3401).
- Round 2: F-3B4D5A FIXED (SuperTrend stop held exactly over 2*SCALE*1000, D-3403, XPERM-03, 30a8cde). Refuted: daily pivot floor (daily.rs:30-40). Already tracked: gap mid 66/67 and EMA sides (ind1-2, owner wip/zero/numeric-edges de48df5), rolling IV (STO-2), forced square-off (p9num-2), fold grid (D-3130).
- D-3400 (EMA sides) WITHDRAWN as a duplicate of ind1-2; runner and cli pins back at base; F-DBC24E marked WITHDRAWN.
- Validation at aa735db (+ docs sha commit): fmt, clippy -D warnings, tests indicators/runner/cli/api (non-root, 0 failures), static gates (1e skipped) all green.

## In progress
- Gate 18 pre-run on the final indicators diff, worktree /tmp/claude-0/wt-mut, log /tmp/claude-0/mut-r2.log, results in /tmp/claude-0/wt-mut/mutants.out/{caught,missed,timeout}.txt (81 mutants). Left running (no usage).
- Round 3 had started with 4 read-only agents (indicator reflection/time-shift symmetry; engine/runner order-invariance, identity, resume; core/costs/greeks cross-module extremes; fold/calendar special sessions); all STOPPED at the pause, no results.

## Next steps after RESUME
1. Read mutants.out: kill every MISSED/TIMEOUT with a real test (never skip), re-run those mutants by --re, push.
2. Re-run round 3 (same four angles; brief at /tmp/claude-0/agent-brief.md incl. stricter dedupe: git grep origin/fix-queue -- resume, and owner wip/* branch diffs).
3. Rounds until one finds zero; then tracker resume/attack-lenses-20261006/permutations.tsv and RESULT-permutations.md on fix-queue.

## Restart
`cd /home/user/brutex && git fetch origin attack/permutations final/all-fixes && git checkout attack/permutations` ; mutants: `cd /tmp/claude-0/wt-mut && cargo mutants --in-diff /tmp/claude-0/mine.diff --baseline skip --in-place --timeout 900 --cap-lints true -p indicators` (no --jobs with --in-place in 26.2.0); tests as non-root via /tmp/claude-0/runtests.sh <crate>.

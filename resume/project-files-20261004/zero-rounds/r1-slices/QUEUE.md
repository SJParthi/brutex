Round 1 plan: ~100 agents in waves of 20 (session cap 20 concurrent; Workflow cap = CPUs-2 = 2 here).
Launched: slice00-19 (general lens).
Queue: slice20-30 general; ci; web; slices 00-30 lens B (edge inputs/panics/overflow + tests that can't fail); per-doc checks docs 00..11 + CLAUDE.md; per-crate invariants-table checks.
Decision numbers: D-1760..D-1799 and D-1900..D-1999. Avoid invariant prefixes AGA..AGD, AHA..AHD, AFA..AFE, AFX, FX3, AUF.
Coordinator started 4 helper threads (crashes/edge, concurrency/state, numbers/complexity, tests/docs/security) writing /mnt/project-files/zero-rounds/*.md. Drop my lens-B wave; count their findings.
THROTTLE 13:14: cap 5 running. Stopped & requeued: slice21 22 25 27 29 30 ci (+ web, docs passes not started). Prioritise fixes.

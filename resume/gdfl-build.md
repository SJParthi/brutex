# GDFL one-second engine build: resume state (Mac)

Updated 2026-10-03 14:10 UTC by the GDFL build thread (Mac, Remote Control).

The full resume kit lives on the operator's Mac at
`/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/RESUME.md`. Start there.
This file is the GitHub copy of where things stand, so nothing is lost if the Mac session stops.

## The four wave-1 parts (local branches on the Mac; never pushed)

| Part | Branch | Head | Stage at 14:10 UTC | Blocking issues left |
|---|---|---|---|---|
| census (`gdfl-census` report-only verb) | feat/gdfl-census | 56962ae2 | review round 2: last lens running | 0 so far; tests-bite r2 refuted with 1 should-fix |
| core-second (`core::second`) | feat/gdfl-core-second | 27d2dcd3 | repair round 2 queued | 0 |
| store-grid (one-second grid file) | feat/gdfl-store-grid | 2707349a (+11 uncommitted files, backed up in wip-20261003) | repair round 2 queued | 0 (round 1 tail re-seal blocker repaired) |
| gdfl-cm (`pull::gdfl_cm` reader) | feat/gdfl-cm-reader | bfee2e39 | repair round 2 queued | 0 |

## Hard rules carried forward

- **Real GDFL rows**: feat/gdfl-census and feat/gdfl-cm-reader were each squashed to one commit on 3 Oct (trees identical to the reviewed tips). A grep for the GDFL row shape over `git log -p` of each branch now finds 0 real rows. census has 1 hit, a synthetic probe row with price 1 and zeros. The old tips survive only as local refs `refs/backup/*-prerewrite-20261003`, which must never be pushed. None of these branches is pushed.
- Integration into `feat/gdfl-1s` must SQUASH each part and prove with `git log -p` that no GDFL row reaches it.
- Agent cap: the operator asked to slow the agents. The build script caps live agents at `MAX_LIVE` (1 until the 5-hour reset, then 3).

## How to continue

- Same account, Mac: `Workflow({scriptPath: ".../state/resume-kit/scripts/build-wave1-continue.js", resumeFromRunId: "wf_3e6c4c50-45c"})`.
- Another account: regenerate the start state from that run's journal (the method is in RESUME.md, checkpoint 13:25 UTC), then launch build-wave1-continue.js fresh.
- D-0802 is folded into PR #74 (PR #21 is closed). Nothing more to do for it.

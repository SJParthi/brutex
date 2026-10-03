# GDFL one-second engine build: resume state (Mac)

Saved by the GDFL build thread (Mac, Remote Control) before the 98% weekly stop. The exact prompts, scripts and every open review issue are in [gdfl-build-prompts.md](gdfl-build-prompts.md). The full kit is on the Mac at `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/` (RESUME.md, read from the end).

## Where each part stands

| Part | Branch (local on the Mac, never pushed) | Head | Next step | Blocking issues left |
|---|---|---|---|---|
| census: `gdfl-census` report-only verb | feat/gdfl-census | 56962ae2 | repair round 2 (2 should-fix, 1 nit) | 0 |
| core-second: `core::second` | feat/gdfl-core-second | 27d2dcd3 | review round 3 (round 2 repaired) | 0 |
| store-grid: one-second grid file | feat/gdfl-store-grid | 2707349a, plus 11 uncommitted files from a stopped repairer (backed up in state/resume-kit/wip-20261003/) | repair round 2 (in progress at save) | 0 (round 1 tail re-seal blocker repaired) |
| gdfl-cm: `pull::gdfl_cm` reader | feat/gdfl-cm-reader | bfee2e39 | repair round 2 | 0 |
| Integration | feat/gdfl-1s (not created yet) | none | after the parts are clean: squash each part in, run the full Definition of Done | none |

Each part loops review (3 lenses: tests-bite, behaviour-extremes-o1, law-and-design) then repair, up to round 5, until no lens refutes and nothing above nit is open.

## Steps for the next session or account (on the Mac, project folder /Volumes/WD_BLACK/brutex/fresh-20260919/project)

1. Add `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925` as a session folder.
2. Check usage with get_usage. Kill leftover build processes from earlier sessions, one pid at a time, unsandboxed, in **bash** (zsh does not word-split `$pids`). Spare tools-work/fullscan, NSE_Options_Tick, Python and caffeinate. Run `nohup caffeinate -dimsu -t 172800 &`.
3. Regenerate the per-part state from the newest build journal(s), newest first: `node state/resume-kit/make-continue-state.js <newest journal.jsonl> [older ...]`. The journals are under `~/.claude/projects/-Volumes-WD-BLACK-brutex-fresh-20260919-project/<session>/subagents/workflows/<run>/journal.jsonl`. Last run: session 0093cf60-67a5-4c37-a8f5-11bed51e6a92, runs wf_3e6c4c50-45c (newest) and wf_124b6c56-8e9. It rewrites the START line of scripts/build-wave1-continue.js.
4. Set `MAX_LIVE` in that script to match the usage you can afford (the operator asked to slow the agents; it was 1 at the stop).
5. Launch it fresh: `Workflow({scriptPath: "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/resume-kit/scripts/build-wave1-continue.js"})`. Do NOT use resumeFromRunId on a run with many failed calls: on 3 Oct that re-ran finished builds.
6. Watch it: `state/resume-kit/watch-build.sh 570` (set BUILD_JOURNAL to the new run's journal). Save again with `state/resume-kit/save-to-github.sh <status.md>`, unsandboxed.

## Hard rules

- **No real GDFL rows on GitHub, ever.** feat/gdfl-census and feat/gdfl-cm-reader were each squashed to one commit on 3 Oct, with trees identical to the reviewed tips. A grep for the GDFL row shape over each branch's `git log -p` finds 0 real rows; census's single hit is a synthetic probe row (price 1, zeros). The old tips survive only as local refs `refs/backup/gdfl-*-prerewrite-20261003`; never push them. The integration step squashes each part and must prove the same with `git log -p`.
- Never push the part branches or feat/gdfl-1s until the operator says to land them.
- Rust only outside web/. O(1) where claimed, proven or listed in docs/06-limits.md. CLAUDE.md is law.
- D-0802 is folded into PR #74 (PR #21 closed). The 16 unfixed batch-1 C4 groups' findings are in resume/lanes/c4-missing-findings.md.

## Machine

M4 Pro, 14 cores, 48 GB. WD_BLACK had ~529 GiB free on 3 Oct. Build targets for this wave are ~40 GB under work-20260925/tgt. The cargo queue in work-20260925/bin/cargo has 8 slots × 4 jobs.

# Prompts used by the lane 1-b redo thread (2026-10-03)

## The user's original request (Parthiban Subramanian, 04:01 UTC)
Continue Brutex from the previous account: read fix-queue resume notes, then in parallel: redo the 55 missing lane 1 fixes and push them onto final/all-fixes (PR #74), or one new PR if #74 merged. Rules: Rust only except web frontend; O(1) wherever possible, naming anything that can't be; work in parallel; attack extreme edge cases adversarially; verify with real evidence, never guess; no new PRs except one final combined PR; never ask the user to tap/paste/approve; keep running continuously; stop at 98% weekly usage after saving resume state to GitHub; deliver results as easy comparison-table Artifacts.

## Coordinator brief for this thread
Redo the 55 missing lane 1 fixes and push them onto final/all-fixes (PR #74); if #74 merged, exactly one new combined PR. List in resume/RESUME-20261003.md and resume/audit-20261003/RESUME.md. Another thread drives #74's CI and may push to final/all-fixes: always fetch and merge before pushing, never force-push or rewrite history. Open no other PRs.

## Worker prompt template (one per group; all read BRIEF.md)
"Read <scratchpad>/BRIEF.md first and follow it exactly. Do not call any mcp__hearthbot__ tool. You are group X. Worktree: /home/claude/wt/x (branch lane1b/x). CARGO_TARGET_DIR=/home/claude/tgt/x. Decision numbers you may use: D-nnnn..D-nnnn, in order. Units (plan.md sections), in this order: ... Findings: ... Mainly files ...; other workers edit other parts of crates/cli/src/lib.rs, keep edits confined to the functions your units name. Commit per unit. Finish with the report the brief specifies."
Groups: A U9,U3,U5,U37,U28,U31,U41 (D-1700..1719); B U8,U24,U25,U23,U26,U27,U34,U17,U32,U35,U36 (D-1720..1739); C U6,U7,U12,U13,U22,U19 (D-1740..1754); D U4,U10,U11,U20,U40,U29,U30,U39 (D-1660..1679); E U15,U38,U18,U16,U33 (D-1680..1699).

## Follow-up prompts that mattered
- B: remove or prove the mildest-tier-first probe; make api max_points==0 mean no ceiling (done, D-1731..1733).
- A: close the D-1702 pool pass-2 withholding gap (done, D-1707); record failing-first evidence (done in report); mutation testing (not done).
- Mutation recipe: `git diff 1087e54...HEAD -- 'crates/*.rs' > x.diff; cargo mutants --in-place --jobs 1 --in-diff x.diff -p cli --cap-lints true --timeout-multiplier 2 -- <test filters>` (faster: opt-level 0, incremental, `--baseline skip --timeout 900`).

## Lessons
- 4 cores/15 GB: at most 2 cargo jobs at once; 5 workers overloaded the box (load 25-36, OOM kills, 2-hour tool limits).
- `cp -al` target dirs share .cargo-lock files: recreate them per copy.
- Disk: delete finished worktrees' target dirs; keep >10 GB free.
- After a merge, grep the WHOLE tree for conflict markers before committing (one slipped into docs/04 once).

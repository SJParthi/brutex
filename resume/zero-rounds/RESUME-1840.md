# Zero-findings thread (5.5) — save at 2026-10-04 18:40 UTC

Supersedes RESUME-1341.md. The tracker of record is `/mnt/project-files/fix-board/status/zero-findings.tsv`, and `tracker-1840.md` beside this file is a copy of it.

**Scope (user and coordinator, 18:34 UTC):** usage is high. Fix only the findings already logged. Run no new audit rounds; the helpers session was told to stop. Use few parallel agents, and pause at 93% of either limit.

## Branches on origin (no PRs; PR #74 is the only PR)

- `zero-work` (staging). It merges edges-3 part 1, ci-web, zero-web12, zero-num56, zero-p10, zero-p8, zero/calendar, zero/data-edges, zero/web-contract and zero/docs04, plus D-1930 (the gate 12 "flat" scrub).
  - Gates FAIL there:
    - 1d: undeclared literals in crates/pull. http.rs 4421-4423 (`5e-1` and similar, from P10-07a), fetch.rs 1027/1033 (`on_closed_day`, `kept_unclassified_day`, from P-03), and `length`.
    - 11: rule 5d now has 7 runtime asserts, so a new one needs an allowlist count and a reason.
    - 23: clause C finds a temp path that does not name its process.
  - Fix those, then merge `zero/edges-3`.
- `zero/edges-3` e004faa1: D-1771..D-1780. Full cli lib suite: 1802 pass, and the one failure is fixed. runner, api, pull, indicators and engine targeted tests pass.
- `wip/zero/{network,numeric-edges,conc-api,conc-data}`: the helpers were stopped at the pause and have uncommitted work in their local worktrees. Resume them one at a time, or re-dispatch from the findings.

## Helpers running at this save

- docs-batch: zero/docs-batch, D-1950..1969. It covers P17-02..21, P18-*, P14-*, p19num-*, p16num-2/3 and CE-95..97.
- withheld-splice: zero/withheld-splice, p11num-1, D-1781..1785.

## Decision numbers

Main thread: D-1930..D-1949 (D-1930 used). D-1771..1799 are all used. The helpers hold D-1950..1969, D-1781..1789 and D-1790..1799.

## Next steps

1. Fix gates 1d, 11 and 23 on zero-work. Merge zero/edges-3. Run `npm --prefix web run build` and commit web/build (P13-01). Then W1, W2 and W3, and all gates.
2. Push `zero-work:final/all-fixes-zero` as a merge, then merge into `final/all-fixes` (PR #74; its head was ddf6d693 at 18:11).
3. Merge the helper branches as they report.
4. Fix the remaining `found` rows with at most two agents at a time. Fix Board owns conc18-1/2, P17-01, p14num-1 and the pr74-conc-api batch. The O(1) sweep thread owns P13-04/05, P15-07/08/09/17, P1-19-03, log-3 and P1-04-02.

## Update 18:50 UTC

- zero-work is now at 726971d7, with gate 1d fixed (D-1931). Gates 11 and 23 still fail.
  - Every count in their allowlists matches (gate 11 rule 5d: 7 of 7; gate 23 clause C: 7 of 7).
  - So the refusal is in a part that `run.sh`'s `tail -15` cut off. Extract the step to a file and run it whole to see it.
- **New task from the coordinator (18:38):** this thread owns the merge of zero-work onto PR #74's head, 1f588aa (about 58 conflict hunks across ci.yml, api, cli, pull, runner and docs).
  - Merge `origin/final/all-fixes` into zero-work, after zero/edges-3 is merged in.
  - Resolve the conflicts. Then run fmt, clippy, the 29 static gates and the affected suites.
  - Hand the PR 74 CI thread ONE validated commit with a note of its contents.
  - Do NOT push to final/all-fixes ourselves.
- Pause at 93% of the 5-hour limit; resume is scheduled for 22:03 UTC.

## Update 19:15 UTC (pausing at the usage limit)

- zero-work is at **0aa8daa4**, pushed. It now also holds zero/withheld-splice (p11num-1, D-1781..1783), zero/edges-3 (D-1771..1780 plus the e004faa1 test fixes), D-1930..D-1933 (gates 12, 1d, 11 and 23), and a rebuilt web/build (P13-01).
  - Every static gate passes locally; 1e was not run.
  - W2: 842 pass and 3 cancelled. The cancellations are ask.test.js on Node 22, which is P19-07; CI uses Node 24.
  - W3: 0 errors. `cargo fmt --check` is clean.
  - Workspace clippy and the full suites have NOT been run on this exact head.
- **Merge onto PR #74 (8f58d915): not done yet.** A trial merge showed 59 conflicts in 61 files. The 31 outside web/build are:
  - `.github`: invariant_paths.rs, source_scan.rs, workflows/ci.yml.
  - api: pullrun.rs, server.rs, strict_sweep_tests.rs.
  - cli: audited_stored_tests.rs, batch.rs, execution_capability.rs, global_replay_v4_tests.rs, lib.rs, minute_gaps.rs, pool.rs, population_base_evidence_v2.rs, population_statistics_v2.rs, search_checkpoint.rs, search_checkpoint_tests.rs, selection_v6_tests.rs, strict_range_knobs.rs, sweep_evidence.rs, sweep_wiring_tests.rs.
  - indicators: tests/invariants.rs.
  - pull: http.rs, ingest.rs, masters.rs, ssm.rs.
  - runner: admission.rs, outcome.rs.
  - docs: 04, 05, 06.
  - web: routes/backtest/+page.svelte.
- **How to do it next session:**
  1. Merge on a scratch branch.
  2. For web/build, take either side, then rebuild after resolving web/src.
  3. Union the append-only docs.
  4. Resolve the code conflicts one by one.
  5. Run `cargo clippy --workspace --all-targets -D warnings`, all the gates, and the affected suites as uid 65534 (one crate at a time, because of memory).
  6. Hand the one commit to the PR 74 CI thread. Do not push final/all-fixes.
- The docs-batch helper (D-1950..1969) is still running on zero/docs-batch.
- Disk: about 5 GB free. Delete /home/claude/target-net and the agent worktree targets if more is needed.

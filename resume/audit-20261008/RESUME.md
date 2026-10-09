# Attack audit completion (third account, thread "Attack audit completion") — resume state

GitHub beats this file. Decision range for this thread: **D-4400..D-4499** (max on any branch was D-3700 on 2026-10-08).
Rule from the coordinator (2026-10-08 23:06 UTC): do NOT push final/all-fixes directly; push `wip/audit-batch3-integ`
and send branch + sha to the PR 74 CI thread, which batches pushes.

## State 2026-10-08 ~23:40 UTC
- `wip/audit-batch3-integ` = final/all-fixes fbdabaec + merge of wip/audit-batch3 181e9e42 (9b0614be, D-4400:
  3 cli conflicts; G18-cli-a-17 checks removed with the second read, elite_ceiling_ppm kept with the seeded cache,
  G18-cli-a-24 test rewritten) + 0f75440f (D-4401: the 5 chunk-A survivors were unobservable; least_first guard and
  final_selection_split's gate filter removed with calendar_holds; equivalence test runs 4 calendar floors).
- Running: full validation (fmt ok, clippy, all test binaries as uid 65534, doctests); cli nextest baseline as root
  in worktree mutwt (target /home/claude/t-mut), then `cargo mutants --in-place --in-diff <(git diff fbdabaec 0f75440f -- crates) -p cli`
  (166 mutants) plus runner/api/engine/vocab in-diff sets.
- Workers (read-only on 9b0614be): r3 (lane 3, 89), r53 (audit-53), r64 (batch-1 64 groups + 60 recovered),
  rnew (audit's own new items + tracker rows), srust (Rust-only scan), so1 (O(1)/p99 inventory), sobs (persistence/
  observability coverage), satk (adversarial probes). Outputs land in the container only; summaries will be copied here.
- If stopped: re-run any missing worker from its brief (same areas), re-run validation on the branch head, run the
  mutation, then hand the branch to the PR 74 thread and publish the comparison Artifact from this account.

## State 2026-10-09 ~00:40 UTC
- Verification done on 9b0614be; reports and machine tables are in `out/` here (r3 lane 3, r53 last audit, r64 batch-1
  64 groups + recovered + new, rnew own/new items, so1 O(1)/p99 inventory, srust Rust-only scan, sobs logging/saving
  coverage, satk adversarial probes). Rules given to verifiers and fixers: VERIFIER-RULES.md, FIXER-RULES.md.
- Full validation of audit/integ at 0f75440f: fmt and clippy clean; 126 test binaries, 123 green as uid 65534, 2 green
  only as root (core findings, store cited_commits: git ownership), 1 (limits_o1cli_6) fixed by 1c9ec4ca.
- Mutation pre-run on `git diff fbdabaec 1c9ec4ca -- crates` in worktree mutwt: engine 14 caught / 0 missed; runner, api,
  cli running (vocab and pull still to run).
- Lens L1 observability merged: audit/obsv = 1c9ec4ca + attack/observability (21443a2a) + web/build rebuild (de932e5b).
  Lens L4 one-authority being merged by fixer fxl4 on audit/l4. WS2, WS3, WS5 belong to another thread (D-4600..D-4699).
- Fixers (local branches, each from de932e5b unless noted; merged into audit/integ by merge commits when done):
  fxa audit/fx-a (from 1c9ec4ca; telemetry/pull/store/lake/docs, D-4410..4429), fxb1 audit/fx-b1 (api costs,
  D-4430..4444), fxb2 audit/fx-b2 (api logging, D-4445..4459), fxc audit/fx-c (cli, D-4460..4479), fxd audit/fx-d
  (from 21443a2a; engine/vocab, D-4480..4489), fxe audit/fx-e (from 21443a2a; CI gates, D-4490..4499), fxr audit/fx-r
  (runner look-ahead, pull, core, D-4500..4519), fxw audit/fx-w (web refusal reasons, D-3211..3225), fxl4 audit/l4
  (D-3516..3530). Fixer reports land in /tmp/claude-0/audit/out/<name>-report.md (container only).
- If stopped: fixer branches are local only until merged and pushed as wip/audit-batch3-integ. Re-run any fixer whose
  branch is missing from GitHub from its item list (out/*.tsv rows not FIXED/DOCUMENTED).

## State 2026-10-09 ~04:50 UTC (usage pacing in force)
- Pacing (coordinator, at Parthi's request): let the in-flight fixers finish and merge; no new discovery/attack fan-out;
  afterwards at most 2 agents at a time, only on work that lands in PR #74 (merge, revalidate, mutation survivors);
  the comparison-table Artifact last, one agent. Any rate/usage-limit error = save here and resume after the reset.
- `audit/integ` = de932e5b + merge of audit/fx-e (b395d99a), pushed as `wip/audit-fx/integ`. fx-e (srust-1..5,
  D-4490..D-4494) is DONE; its report: srust-5 needs the owner (are GitHub's Node-based actions a forbidden runtime under
  CLAUDE.md §2? D-4494 lists them; a gate now refuses any unpinned action and JS outside the web job).
- Fixer branches pushed for safety under `wip/audit-fx/<name>` (fx-a, fx-d, fx-w, l4 so far); fx-b1, fx-b2, fx-c, fx-r
  had no commits yet at 04:50 (CPU-bound box, cli/api builds slow).
- Mutation pre-run (mutwt @ 1c9ec4ca, diff fbdabaec..1c9ec4ca): engine 14 caught/0 missed; runner 41 caught/23 unviable/0
  missed; api 22 of 86 so far with 2 MISSED: `api/src/answer_memo.rs` Debug for Memo -> Ok(Default) (add a Debug output
  test) and `api/src/candidatejson.rs` summary_for `&&`->`||` at the identity clause (test: same identity+attempt but
  different model or root must be cold). cli (166), vocab, pull not yet run.
- Resume order if stopped: (1) for each fixer branch, finish or re-run from its report/items; (2) merge every fixer
  branch into audit/integ with merge commits (docs tails: keep both sides); (3) rebuild web/build (Gate W1) if web/src
  changed; (4) full validation (/tmp scripts are lost with the container: fmt, clippy -D warnings, all test binaries
  as uid 65534, doctests, gates 10/10b/27/27b/W1); (5) mutation pre-run on `git diff fbdabaec <head> -- crates` per crate,
  kill survivors; (6) push wip/audit-batch3-integ and send branch+sha to the PR 74 CI thread; (7) Artifact
  https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx from out/*.tsv plus fixer outcomes.

## PAUSED 2026-10-09 ~05:30 UTC for the usage window (resume check-in armed 09:05 UTC)
Every fixer committed and stopped; branches pushed as `wip/audit-fx/<name>`; notes in `fixers/` here.
| fixer | branch head | done | WIP / not started |
|---|---|---|---|
| fxe CI gates | merged in integ | srust-1..5 (D-4490..4494) | owner: srust-5 Node actions |
| fxw web | merged in integ | W1-W7, F1-F7 (D-3211..3224) | owner: 2 policy items in fxw-report |
| fxa telemetry/pull/store/lake | 8ec02b03 | sobs-2,3,11,13; r53-1; satk-2,5,6,9 (D-4410..4418) | WIP so1-2, so1-6; not started rnew-2, rnew-3, W1-pull1-0, sobs-16, sobs-17, sobs-19, r53-2; ET-bars-candles-store-3 owner; o1api-33 |
| fxb1 api costs | 32896202 | 14 items (D-4430..4443) | WIP W1-api1-4; re-run api suite+clippy (4 fixes unverified) |
| fxb2 api logging | fae2225e | 11 items (D-4445..4455) | clippy api/telemetry not re-run; sobs-18 dropped |
| fxc cli | 5cd98c62 | 9 items + W2-cli16-1 (D-4460..4469) | WIP W2-cli10-0 (re-run clippy); not started items 7-11 |
| fxd engine/vocab | 2cfa70ae | o1engine-20 (D-4480) | WIP so1-1, so1-3, W3-engine1-1, o1engine-22/23, GAP16-26 (D-4481..4486); runner suite, gates, mutants |
| fxr runner/pull/core | ce2090fd | lookahead+r64-5 (D-4500), satk-1, r64-4 | WIP satk-7/8, closure items, c4a-6, comments; not started W3-runner2-3/2-5, satk-3, r64-2, W1-pull3-4. cli must switch to printed_ohlcv_cost_model_id_v3 (D-4500) or its policies are refused |
| fxl4 one-authority | 70e2c987 | merge 9b47820c validated (non api/cli) | WIP round 4 (D-3516..3522); re-run validate for 7 crates; D-3510 owner |
After 09:05: at most 2 agents. Finish WIP per notes, merge all into audit/integ, cli switch to cost model v3, full
validation, mutation survivors (api so far: answer_memo Debug, candidatejson summary_for), push, hand to PR 74 thread,
then the Artifact.

## RESUMED 2026-10-09 09:30 UTC at full speed (Parthi lifted the 2-agent cap; weekly guard 93%/98% still applies)
- `audit/integ` = a7a27dc3 (pushed as `wip/audit-fx/integ`): every fixer lane merged (fx-e, fx-w, fx-b2, fx-a, fx-b1,
  fx-c, fx-d, fx-r, l4), web/build rebuilt (W1 passes at 3617160f), lake bench clippy fixed. `wip/audit-batch3-integ`
  is still de932e5b; it moves only when the combined branch is validated.
- Running from a7a27dc3 (each in its own worktree and target dir):
  - int-u (`audit/int-u`, wt-u): api + cli compile/clippy breaks from the merge (~31 api clippy errors), lane WIP there.
  - int-l (`audit/int-l`, wt-l): lower crates (runner test, engine/vocab/pull/store/lake/telemetry WIP).
  - def-c (`audit/def-c`, wt-dc): cli replay cost items, D-4520..D-4534.
  - def-p (`audit/def-p`, wt-dp): pull/store cost items, D-4550..D-4564.
  - Artifact agent: fills https://claude.ai/artifact/7cvC2yoWTHviHri8zz4TPx (Dashboard type) via ArtifactData from
    out/*.tsv + after.tsv; leaves a refresh recipe.
- Reserved but not started: D-4535..D-4549 (sobs-16 searchable log, sobs-18 research ledger read endpoints).
- Next: merge int-u, int-l, def-c, def-p into audit/integ (merge commits), rebuild web/build if web/src changed, full
  validation, mutation pre-run on `git diff fbdabaec <head> -- crates`, kill survivors, push wip/audit-batch3-integ,
  send the sha to the PR 74 CI thread and to the Merge leftover workstreams thread (it merges this branch into its own
  and hands PR 74 one sha).

## PAUSED 2026-10-09 ~10:25 UTC: GDFL is the top priority (coordinator, at Parthi's word); resume only when told
Owner choices posted in the thread with defaults taken (Parthi may switch with one word):
- JS helper actions in non-web CI jobs (srust-5 = workflow-js-actions): default **Allow** (keep the 7 pinned actions;
  record the ruling in CLAUDE.md §2 + a D-entry). Alternative Replace.
- Store admits off-session bars (ET-bars-candles-store-3): default **Inject** (core declares a session-check trait,
  pull's calendar implements it, store refuses an off-hours bar at write, by name). Alternatives Move, Leave.
Not started yet: both defaults still need doing.

Owner rule restated by the coordinator: "documented only" is not done. The 87 DOCUMENTED ids (out/doc87.tsv) must each
be fixed, or kept only if truly inherent WITH a committed measurement (p50/p99/max, n, load). Triage stopped at 36/87
(6 FIX, 6 KEEP-MEASURE, 6 ALREADY-FIXED, 18 OVERLAP with defp/intl, 51 NOT-YET-TRIAGED); see out/doc87.md and
TRIAGE-DOC87-BRIEF.md. Note: the D-2290 figures come from an uncommitted scratch harness (docs/06-limits.md:16700).

Branches (all pushed as `wip/audit-fx/<name>`; pause notes in fixers/<name>-pause.md):
| branch | head | done | left |
|---|---|---|---|
| audit/int-u | 4e1e6a08 | api clippy clean (D-4470); cli on cost model V3, V2 refused by name (D-4471) | telemetry::sync at exit (D-4472), tell -> stderr_line + Gate 23 (D-4473), ⟨a14_*⟩ placeholders (docs/06 12624-5), W2-cli10-0 + review 0e3eee9a, cli clippy, validation |
| audit/int-l | af71bb1e | runner clippy; fxr WIP (D-4501..4506); so1-2/so1-6 (D-4420); fxd WIP + CLAUDE.md support_each (D-4487/8); rnew-2, r53-2/srust-6, sobs-17 (D-4489) | r64-2, sobs-19, W3-runner2-5; engine bench comments C-E-07/C-E-09 still say Column::support; full runner suite; validation |
| audit/def-c | aa55d682 | AC-whp-o1-1 (D-4520) | W3-runner2-3 WIP (D-4521 not written); cli-14 & W2-cli12-3/4, W2-cli6-0, W2-cli2-5, W2-cli14-1/2/3 (D-4522..4525 planned; no O(1) fix found yet, plan: measure) |
| audit/def-p | 386c4f07 | satk-3 (D-4550) | cli test resample_matches_fold edit not compiled; W1-pull3-4, W1-pull1-0, rnew-3, o1api-33 |
`audit/integ` now merges int-u and int-l on top of a7a27dc3 (see the line below for its sha and check results).
def-c and def-p are NOT merged into integ.

Cross-branch notes from the merge-leftover thread (it is merging a7a27dc3 into claude/project-thread-lx6ptl as
bff477f0 and will merge our final sha on top): api operation_audit_tests.rs keeps its test-only `poll` helper (D-4622);
pull/benches/ratio.rs `session_member` needs `skipped: DecodeSkips::default()` after their D-3125 (their D-4620).

Artifact: filled (Dashboard type) with interim numbers; refresh recipe ARTIFACT-REFRESH.md; after.tsv in out/.
Helper scripts (build-data.py, build-after.py, build-proof.py, check-after.py, merge-fx.sh, union-tail.py,
validate.sh, gates-doc.sh) are container-local and deliberately untracked; rewrite from these notes if lost.
- 10:27 UTC: `audit/integ` = 8102ca76 (a7a27dc3 + merge int-u 4a6272e4 + merge int-l), pushed as `wip/audit-fx/integ`.
  On 8102ca76: fmt clean; gates 27, 27b, 10b, 10 pass. Workspace clippy was stopped unfinished (usage); not run: cli
  clippy, test suites, mutants. Sent 8102ca76 to the merge-leftover thread (session_01UQpSgcb4nDRnDnd3ajjWJc) as the
  current, not final, sha. Stopped at 5-hour 80% / weekly 76%; idle until the coordinator says resume.

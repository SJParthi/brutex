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

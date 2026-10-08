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

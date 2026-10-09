# Merge leftover workstreams — resume state, 2026-10-09

Thread "Merge leftover workstreams" (Brutex project). Saved because usage is
the binding constraint; this thread goes quiet after the handover.

## What exists

- Branch `claude/project-thread-lx6ptl` on origin, head `5e7c048f` (09:20 UTC;
  was `061c7004`). No PR, by
  rule. Handed to the PR 74 CI thread, which batches pushes to
  `final/all-fixes` and merges this branch once it is told the local gates are
  green.
- `5e7c048f` is again a merge with first parent `fbdabaec` and second parent
  `57236428` (= `061c7004` + the D-4613 test), so it fast-forwards from
  `061c7004` and Gate 18 still plans against fbdabaec.
- `061c7004` is a merge with FIRST parent `fbdabaec` (final/all-fixes at the
  time) and tree = `f97eb1f7`, so a `workflow_dispatch` on the branch plans
  Gate 18 against exactly the three workstreams (HEAD~1 = fbdabaec).
- Merged, merge commits only:
  - WS2 data-path attack `claude/attack-data-pipeline-hgxmw9` @ 66eb9ebc
  - WS3 Fix Board `fixboard/pr74-batch4` @ e0709bd3 (contains batch3) and
    `fixboard/pr74-batch5` @ 7dd8f8d1
  - WS5 zero rounds `zero/next` @ 8db2fa83
- Decisions D-4600..D-4613 (range D-4600..D-4699 is this thread's; next is
  D-4614). Invariants MRG-01..MRG-05 in docs/04, section "Integration of the
  data-path, Fix Board and zero-round branches".

## Validation (local, 4-core box; tests as uid 65534, git-reading ones as root)

- fmt clean; `cargo clippy --workspace --all-targets --locked -- -D warnings`
  clean; `cargo deny check` ok (advisories, bans, licenses, sources).
- language-purity static gates: 29 pass, 0 fail, Gate 1e not run locally
  (heavy; CI run 1287 runs it).
- web gates W1-W6: all pass (W2 911/911).
- Full suite run 1 (before the fixes): 134 binaries pass, 9 fail. 2 were
  git-as-uid-65534; 2 ran pricing binaries built before the D-4609 test
  edits; 5 held real cross-branch conflicts (6 tests: tix, attack_store, core
  contract order, attack_r6 scratch race, api pause-gap and emit-site pin),
  fixed in D-4610..D-4612.
- Full suite run 2 on f97eb1f7: 141 pass as uid 65534; `store/cited_commits`
  (6/6) and `core/findings` (14/14) pass as root. cli lib 2068 pass, api lib
  1626 pass.
- Build-job gates at save time (05:03 UTC): 6a, 4a, 13a, 13b, 4 pass; 13c, 5,
  6c, 6d were still running. The "Is there a crate" probe fails locally only
  because GITHUB_OUTPUT is unset; its own 28 tests pass.

## CI

- Run 1287 (id 37884363547), workflow_dispatch on 061c7004:
  https://github.com/SJParthi/brutex/actions/runs/37884363547 — coverage,
  complexity, Gate 1e and the Gate 18 pre-run (~1,732 changed-line mutants).
  The PR 74 CI thread asked that any MISSED/TIMEOUT from it be fixed on this
  branch and the new sha sent to it (its own survivor fixes use D-4100..D-4199).
- Survivors: MISSED via `curl https://api.github.com/repos/SJParthi/brutex/check-runs/<job>/annotations`;
  TIMEOUT only in job logs.

- Run 1287 result: everything passed (Gate 1e, coverage floors, Gate 8,
  Gates 1+2, W, 3-6) except Gate 20 (telemetry sink.rs 28 uncovered vs 20
  declared). The mutants job `needs: coverage`, so every Gate 18 shard was
  SKIPPED: no survivor data from 1287. Fixed by D-4613 (a test for
  `resume_point`'s unmeasurable-file arm); crate-scoped coverage reproduces
  CI exactly (20 on fbdabaec, 28 on 061c7004, 20 after the fix).
- Run 1288 (id 37910274155), workflow_dispatch on 5e7c048f:
  https://github.com/SJParthi/brutex/actions/runs/37910274155 — carries the
  Gate 18 pre-run (1735 cases, 56 jobs). The PR 74 thread was told to merge
  5e7c048f, not 061c7004.
- Coverage artifacts can't be downloaded here (blob host denied by the proxy).
  Measure telemetry instead: build `cargo test -p telemetry` with
  `RUSTFLAGS=-C instrument-coverage` in a dir under /home/claude (not
  /tmp/claude-0, which is 0700 and breaks the uid-65534 child tests), run the
  binaries, `llvm-profdata merge`, `llvm-cov show <bin> -object <bin2>` on
  sink.rs and count `|      0|` lines.

## Cross-branch traps the first full test run found (no textual conflict)

- `.tix`: D-3134's one-entry repair met a CUT index (D-2077's test) and left it
  incomplete; fixed with one fstat (D-4611, MRG-04).
- `Contract::parse` became strict (D-3151) under a zero/next test that parsed a
  padded strike; `member=NIFTY` needed after D-2759's swept mapping gate;
  `ReaderHolds` (D-2552) vs `Locked`; a shared scratch tag raced in
  `attack_r6_rolling` (D-4612).
- Pins both sides re-took: gap digest now 17_175_828_523_226_610_466; api lib
  emit sites 77, server-test sites 25 (D-4610).

## Local validation recipe

- Worktree `/home/claude/wt-int` (branch int/ws3) with target `/home/claude/tgt-int`
  is container-local and will be gone; recreate from the pushed branch.
- Static gates: run the `language-purity` job steps with `RUNNER_TEMP` and
  `SOURCE_SCAN=$RUNNER_TEMP/source-scan` exported (Gate 0 writes it to
  GITHUB_ENV in CI). Gate 1d refuses any new lowercase-hyphen literal under
  `crates/pull`, test scratch tags included: reuse a tag already declared in
  `.github/gates_tree.rs` `ATTACK_LITERAL`.
- `store/cited_commits` and `core/findings` fail as uid 65534 (git ownership)
  and must be run as root. Four store catalog lock tests fail only as root.

## State at 10:20 UTC 2026-10-09

- Run 1288 went red at Gate 1e (an operation-audit test unwrapped a BUSY
  journal read). Fixed in `bff477f0` (D-4622, test-only `poll` helper), which
  also merged the attack-audit head `de932e5b` (D-4614..D-4621). Pushed to
  `claude/project-thread-lx6ptl`. The full local test run on it passed except
  the two git-ownership binaries, and those pass as root.
- In progress, NOT pushed: merging `wip/audit-fx/integ` `a7a27dc3` (81
  commits, 282 files) onto `bff477f0` in `/home/claude/wt-int2` (branch
  int/ws3-fx). Decisions D-4647..D-4658 (lead) and D-4623..D-4630 (api
  agent). Real semantic overlaps found: double directory sync in
  `pull::capture` (D-4649); the trade-reader cache, where each side's fix undid
  the other (D-4655); and double autopilot verdict logging (D-4657, one writer:
  `note_decision`). Check pullrun, sweeprun and server for the same
  double-logging pattern. `api::emitted` site counts must be re-measured.
- Coordinator (09:58 UTC): GDFL is Parthi's top priority. Use at most one
  agent and start no fan-outs. The audit thread is pausing and will send its
  current sha; merge that, then hand PR 74 one sha.

## Next

1. Finish the a7a27dc3 merge: emitted.rs counts, fmt, clippy, the tests, the
   static gates, and Gate 20 for telemetry `sink.rs` (re-measure).
2. Merge the audit thread's paused sha when it arrives and revalidate.
3. Build the final head as a merge whose first parent is fbdabaec and whose
   tree is the combined integration. Push it and send its sha to the PR 74 CI
   thread (cse_0192cvXYTyfh6ihYTAF7UgaR). Do not dispatch CI unless that is
   agreed.

## State at 14:20 UTC 2026-10-09 (handed, idle)

- `claude/project-thread-lx6ptl` @ `614fdc4b` (pushed): `bff477f0` + the
  merge of `wip/audit-fx/integ` `a7a27dc3` (98ae0a42, D-4623..D-4628,
  D-4647..D-4658) + the merge of the audit's paused head `8102ca76`
  (1d98ec9c, D-4659..D-4661) + fixes for the checks that merge failed
  (614fdc4b, D-4662, MRG-17). Sent to the PR 74 thread, which merges it on
  top of final/all-fixes `ff390a4f` (ledger tails: keep both).
- `8102ca76` is NOT the audit's final `wip/audit-batch3-integ`; that comes
  when the audit resumes and must be merged the same way.
- Validated: fmt; clippy -D warnings (workspace); full test run on 1d98ec9c
  147 pass, 5 fail (2 git-ownership env-only, 3 fixed in 614fdc4b and
  re-run green); static gates on 614fdc4b 25 pass, 4 fail.
- The 4 failing gates fail on `8102ca76` itself (checked by running the job
  there with only its broken Gate 0 sed self-test removed). They are the
  audit's open items: 1d (`microseconds`, `x7` in crates/pull tests), 12
  (operation_audit_tests latency doc, topjson.rs:1), 21 (telemetry loss.rs
  OpenOptions::new, sink.rs File::open), 23 (10 api latency-test prints).
- Decision numbers used: D-4600..D-4662. Next free in this range: D-4663.
- Traps this merge found (no textual conflict): browser word `fetch(` vs
  the boundary-read check; D-3511's `echo "use sed here"` vs D-4493's word
  rule; a dropped `cli::live::Live` removes its file (D-2641), so fixtures
  must hold views; `target=swept` needs `member=NIFTY` (D-2759); census
  surface is 210 rows (D-3507); IST offset must be `pull::session`'s.
- Check-in routine for run 1288 fired at 11:46 UTC and was read at 14:18:
  run 1288 completed 09:36 UTC, failed at Gate 1e (fixed in bff477f0,
  D-4622); Coverage, Gates 3-6, Gate 8, Gate 18 and every shard were
  SKIPPED, so it has no survivor data. Not re-armed: the branch is handed to
  PR 74, whose CI on the combined push supersedes 1288.

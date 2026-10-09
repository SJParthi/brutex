# Merge leftover workstreams — resume state, 2026-10-09

Thread "Merge leftover workstreams" (Brutex project). Saved because usage is
the binding constraint; this thread goes quiet after the handover.

## What exists

- Branch `claude/project-thread-lx6ptl` on origin, head `061c7004`. No PR, by
  rule. Handed to the PR 74 CI thread, which batches pushes to
  `final/all-fixes` and merges this branch once it is told the local gates are
  green.
- `061c7004` is a merge with FIRST parent `fbdabaec` (final/all-fixes at the
  time) and tree = `f97eb1f7`, so a `workflow_dispatch` on the branch plans
  Gate 18 against exactly the three workstreams (HEAD~1 = fbdabaec).
- Merged, merge commits only:
  - WS2 data-path attack `claude/attack-data-pipeline-hgxmw9` @ 66eb9ebc
  - WS3 Fix Board `fixboard/pr74-batch4` @ e0709bd3 (contains batch3) and
    `fixboard/pr74-batch5` @ 7dd8f8d1
  - WS5 zero rounds `zero/next` @ 8db2fa83
- Decisions D-4600..D-4612 (range D-4600..D-4699 is this thread's).
  Invariants MRG-01..MRG-04 in docs/04, section "Integration of the
  data-path, Fix Board and zero-round branches".

## Validation (local, 4-core box; tests as uid 65534, git-reading ones as root)

- fmt clean; `cargo clippy --workspace --all-targets --locked -- -D warnings`
  clean; `cargo deny check` ok (advisories, bans, licenses, sources).
- language-purity static gates: 29 pass, 0 fail, Gate 1e not run locally
  (heavy; CI run 1287 runs it).
- web gates W1-W6: all pass (W2 911/911).
- Full suite run 1 (before the fixes): 134 binaries pass, 9 fail; 6 were real
  cross-branch conflicts, fixed in D-4609..D-4612; 3 were git-as-uid-65534.
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

## Next

1. Fix any MISSED/TIMEOUT run 1287 reports, on this branch; send the new sha
   to the PR 74 CI thread.
2. Nothing else is owed by this thread.

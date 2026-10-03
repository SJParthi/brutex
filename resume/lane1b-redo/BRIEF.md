# Lane 1-b redo: worker brief (shared by every group)

You are one of four parallel workers redoing the lane 1-b fixes for the Rust
repo `SJParthi/brutex`. These findings were planned and partly fixed on
2026-10-02, then the work was lost in a container restart. An audit of
`final/all-fixes` (head 1087e544) confirmed they are NOT fixed. You redo them.

## Where you work
- Your git worktree and branch are given in your prompt. Work ONLY there.
  Never touch /home/claude/brutex (the main checkout) or another worker's worktree.
- Set for every cargo command: `CARGO_TARGET_DIR=<your target dir>` and
  `CARGO_BUILD_JOBS=2`. The machine has 4 cores and 15 GB RAM shared by four
  workers: never run two cargo commands at once yourself, and prefer
  `cargo test -p <crate> <filter>` over workspace-wide runs until the end.
- Commit on your branch, one commit (or a few) per unit. NEVER push, never
  open a PR, never call any `mcp__hearthbot__*` tool, never call GitHub tools.
  The lead session merges and pushes your branch.
- Commit message: what changed in plain words, then the finding ids and the
  decision number, then these two trailer lines exactly:
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_014G2sJnsvpkQsce9x4YKvd8`
  Use `git -c user.name=Claude -c user.email=noreply@anthropic.com commit`.

## What to read first (all in the scratchpad dir
`/tmp/claude-0/-home-claude-brutex/2a78e5db-4588-5749-a293-e6303c197302/scratchpad/`)
1. `CLAUDE.md` at your worktree root: the repo law. Read all of it. It wins.
2. `plan.md`: the 2026-10-02 plan, one `## Uxx` section per unit: findings,
   files, goal, failing-first tests, edge cases. Its line numbers are from an
   OLDER commit (bc53131); the code has moved.
3. `v1.md`: the 2026-10-03 audit verdict per finding with CURRENT file:line
   evidence at 1087e544. Trust its line numbers over the plan's.
4. `lane1-b.md`: the original finding texts (search by finding id).

## The rules (the owner's, and the repo's)
- Rust only (outside `web/`). No new file extensions, no build.rs, no scripts
  committed. Your scratch scripts stay outside the worktree.
- O(1) wherever possible. Anything that cannot be made O(1) or
  output-proportional gets an honest bound in `docs/06-limits.md`, naming it.
- Attack extreme edge cases adversarially (empty, one element, exact limit,
  limit+1, overflow, i64::MIN/MAX, races, reversed order, foreign input).
- Verify with real evidence. Never claim a measurement or a test run you did
  not take. Label extrapolations. `UNVERIFIED` where unsure.
- Failing-first: for each bug, write the test, watch it FAIL on the old code
  (record the failure), then fix and watch it pass.
- No test that asserts nothing; every new branch must be exercised by a test
  (the repo requires 100% line+branch coverage and no surviving mutant on
  touched modules — write tests that would kill obvious mutants: flipped
  comparisons, off-by-one, removed calls).
- Never weaken, skip, `#[ignore]` or delete an existing test to get green.
  If an existing test pinned the buggy behaviour, change it and say why in
  the decision entry.
- No fallback that hides a failure. Refuse loudly with a named reason.

## Ledger bookkeeping per unit (gates check these)
- `docs/05-decisions.md`: append ONE entry per unit at the END of the file,
  heading in the same format as the existing last entries (look at the tail),
  using ONLY the decision numbers in your prompt, in order. The entry says the
  finding ids, what was wrong, the decision, and what was rejected. Gate 27b:
  every number heads exactly one entry.
- `docs/04-invariants.md`: every new invariant gets a row naming the exact
  test that proves it (Gate 10 checks the test exists; Gate 27 checks each
  row id is unique — pick fresh ids, grep first).
- `docs/06-limits.md`: cost bounds that are not O(1); update or remove claims
  your fix makes false. If you add or change a Gate 11 allowlist count in
  `.github/workflows/ci.yml`, its reason goes in the docs/06-limits.md section
  "Gate 11 allowlist reasons", NOT as a comment in ci.yml. ci.yml must stay
  well under 524,288 bytes.
- Append at the END of docs/04/05/06 (or in the relevant existing section),
  so the lead can union-merge four workers' tails.
- A doc-false finding is fixed by making the text true; if a test in the repo
  refuses stale sentences (e.g. `crates/*/tests/docs.rs`,
  `crates/vocab/tests/stale_claims.rs`), add the old false sentence there.

## Gates you can run locally
`python3 /tmp/claude-0/-home-claude-brutex/2a78e5db-4588-5749-a293-e6303c197302/scratchpad/gate.py <worktree> <label>`
extracts and runs the CI step "Gate <label>". Labels: `python3 gate.py . LIST`.
Run from a clean `git add -A` state. Run at least 10, 10b, 11, 12, 14, 17, 22,
23, 27, 27b before you finish (gate 11 is slow, ~25 min; run it once at the end).

## Checks before you finish
1. `cargo fmt --all --check` clean.
2. `cargo clippy --workspace --all-targets --locked -- -D warnings` clean.
3. `cargo test -p <each crate you touched> --locked` green (as root, the 4
   store catalog-lock tests and some api/pull/telemetry permission tests fail
   only because of root; that is known and not yours). For `cli`, the full
   lib run is ~10+ min; run it once at the end.
4. The gates above.

## Your final report (your last message; the lead reads only this)
A table with one row per finding id in your units: id | verdict
(FIXED / DOCUMENTED / NOT-DONE) | commit sha | decision | test name(s) |
one-line evidence (file:line). Then: checks you ran with their real result
lines, anything you could not do and why, any NEW defect you found. Be honest:
a NOT-DONE with a reason beats a fake FIXED.

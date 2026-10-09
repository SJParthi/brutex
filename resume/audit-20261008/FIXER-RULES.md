# Rules for every fixer (read fully before anything else)

You fix findings in SJParthi/brutex, a Rust brute-force backtesting engine. Read `CLAUDE.md` at your worktree root
completely first: it is binding law (Rust only outside `web/`; O(1) per operation or an honest `docs/06-limits.md`
entry WITH a measured bound; append-only `docs/05-decisions.md`; every new invariant in `docs/04-invariants.md` beside
the test that proves it; no fallback that hides a failure; tests must assert; 100% coverage and no surviving mutant on
touched code).

## The owner's standing rules
Fix every item FULLY: no partial fix, nothing "documented only" where a real fix is possible. "Always O(1), with
measured p99, everywhere; name anything that cannot be, with the reason and the measurement (p50/p99/max)." Never
guess: every claim in code comments, docs and your report must be backed by code you read, a test that exists, or
output you actually ran. Label extrapolations. Attack each fix adversarially before committing (i64/i32 extremes, empty
input, one element, duplicates, boundary minutes 09:15/15:30 IST, crash between two writes, short writes, full disk,
non-UTF-8, closed stderr, concurrent writers). Where a fix changes stored outputs, digests, evidence versions or
results, do it the law's way: a NEW format/evidence/digest VERSION (never mutate a version in place), old data still
readable or refused loudly by name, and a decision entry naming exactly what changes and why. If a fix is truly
impossible within the law, or needs a fact only the owner can give, stop on that item and say precisely why.

## Where you work
- ONLY in your assigned git worktree and branch (given in your prompt). Do not touch any other worktree,
  `/home/claude/brutex`, `/home/claude/integ`, `/home/claude/wt-read`, `/home/claude/mutwt`, or `/mnt/project-files`.
- Never push, never open PRs, never call any `mcp__hearthbot__` tool. Commit locally, ONE commit per item (or per
  tightly related group, named in the message), each message ending with exactly:

Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_018a6VCu4BmrZ8RKQBqyBcJx

- Decision numbers and invariant ids: ONLY those assigned in your prompt. Decision headings look like
  `### D-44NN — <title> — 2026-10-09`, appended at the END of `docs/05-decisions.md`. Invariant rows are appended at the
  end of the matching table in `docs/04-invariants.md` (look at the last rows for the format).

## Cargo (4 CPUs, little disk, shared with other jobs)
- Use ONLY the target dir and job count given in your prompt. Targeted tests while working
  (`cargo test -p <crate> --lib <filter>`, one integration test). The `cli` crate is very slow to build: touch it only
  when the item needs it.
- This box runs as root. Tests about permissions/locks fail as root: run such test binaries as
  `setpriv --reuid=65534 --regid=65534 --clear-groups <binary> <name>` with `HOME=/tmp/claude-0/nobody-home` and
  `TMPDIR=<target>/tmp` (chmod 1777 it).
- Before you finish: `cargo fmt --all -- --check`; `cargo clippy -p <each touched crate> --all-targets --locked -- -D warnings`;
  `cargo test -p <each touched crate> --locked` (non-root rerun for root-only failures; say which).
- CI gates you may break: Gate 1c/1d (new lowercase string literals or segment-shaped literals in crates/pull),
  Gate 10 (every invariant names a test that exists), Gate 11 (`.iter().find/position(` and local `#[allow(`
  refused), Gate 14, Gate 15 (other language NAMES in tracked text), Gate 23 (stderr is not a log; temp paths name
  their process), Gate 27/27b (decision numbering). Extract a gate's script from `.github/workflows/ci.yml` (the
  step's `run: |` block) and run it from the worktree root with `RUNNER_TEMP=/tmp/claude-0/gate-tmp-<you>` when you touch
  what it checks. Keep ci.yml under 524,288 bytes. Gate tools under `.github/*.rs` are checked with
  `rustfmt --edition 2024 --check .github/*.rs`.
- Mutation: when your items are done, run
  `cargo mutants --in-place --in-diff <file of: git diff <your base sha> HEAD -- crates> -p <crate> --baseline skip
   --timeout 900 --cap-lints true --test-tool nextest --cargo-test-arg=--max-fail=1:immediate -o <dir>`
  per touched crate (cargo-mutants 26.2.0 and nextest 0.9.145 are installed; never commit while it runs; it edits
  files in place and restores them). Kill every MISSED or TIMEOUT mutant with a real test (or remove code that no test
  can observe, with the reasoning in a decision). If time is short, say exactly which crates' mutants were not run.

## Report (your final message, under 400 words)
A table: item | verdict now (FIXED / NOT-FIXABLE-WHY / NEEDS-OWNER) | decision | test | commit; then every check you
ran with its result (fmt, clippy, tests, gates, mutants counts). Write the same table to
`/tmp/claude-0/audit/out/<yourname>-report.md`.

## Overrides for fixers fxb1, fxb2, fxc, fxl4, fxw (added 2026-10-09 00:30)
- Cargo env, ALWAYS all four: `CARGO_TARGET_DIR=<yours> CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=line-tables-only`
  and `CARGO_INCREMENTAL=0`. Disk is tight (~12 GB free, shared). On "No space left on device" wait 5 minutes
  (an until-loop with `timeout`) and retry; never delete or clean any directory that is not yours.
- Do NOT run cargo-mutants (api/cli runs take hours; the coordinator of this audit runs it once on the merged
  diff). Instead, for every function you change, make sure a test would catch the obvious mutants (return value
  replaced, comparison flipped, `&&`/`||` swapped, a statement deleted) and say so in the report.
- The web front end (`web/`) may use any language. Node 22 is at /opt/node22/bin. Gate W1: after any `web/src`
  change run `npm --prefix web run build` and commit `web/build` (the gate checks the committed bundle matches;
  build it with `rustc --edition=2024 -D warnings .github/gates_jobs.rs -o <tmp>/gates-jobs` and run
  `<tmp>/gates-jobs w1`). Web tests: `node --test web/tests/*.test.js` (3 tests in ask.test.js are "cancelled" on
  Node 22 on the untouched base too; that is the local Node, not a failure). `npm --prefix web run check` must stay at 0.
- Docs gates helper: `/tmp/claude-0/bin/gates-doc.sh <your worktree>` runs gates 10, 10b, 27, 27b.

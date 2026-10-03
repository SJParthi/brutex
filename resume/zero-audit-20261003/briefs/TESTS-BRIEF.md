# Tests that cannot fail: fix brief

You are fixing one cluster of confirmed audit findings in the brutex Rust workspace: tests that pass whatever the code does, or that prove less than their invariant row says. CLAUDE.md in the repo is the law; §4 bans "a test that asserts nothing", and §9 requires no surviving mutant. Rust only.

## Where to work
- Your own git worktree (you were started with isolation). Check out a local branch `zero/test-teeth` from the commit you start on. Do NOT push. Do NOT open a PR. The main session merges your branch.
- Build and test with `CARGO_BUILD_JOBS=2`; another build may be running on this 4-CPU box.
- The container runs as root; permission tests need non-root (`setpriv --reuid=65534 --regid=65534 --clear-groups ...`). If you cannot run one, say so.
- /mnt/project-files is shared with other sessions: read it, never write there. Install nothing.

## Findings to fix
All in `/mnt/project-files/zero-rounds/tests-docs-security.md`. Read each `### P1-...` section for evidence and the suggested fix:
- P1-10-01, P1-10-02, P1-10-03, P1-10-04 (cli tests, first half)
- P1-11-01, P1-11-02, P1-11-03 (cli tests, second half; P1-11-01 must make the test read the real dispatch, or the docs and SC-08 must stop claiming it does — prefer the real check)
- P1-12-01 .. P1-12-05 (api tests)
- P1-13-01, P1-13-02, P1-13-03 (engine/indicators/vocab tests)
- P1-14-01 .. P1-14-07 (core, pull, costs, lake tests; P1-14-01 must compare against the published SigV4 vector)
- P1-17-01 .. P1-17-04 (invariant rows whose cited test does not check the claim; fix the test, or correct the row if the claim is wrong)

## How to fix
- A child-process test asserts the child ran exactly one test (`1 passed` in its stdout), and builds the test path from `module_path!()` where it can.
- A source-shape test bounds every function body it reads (`"\n}\n"` after the named `fn`), ignores comments when it says it reads code, and cannot match its own needle.
- Replace an assertion that is satisfied by a banner, a count or an always-printed string with one on the real result.
- For each fix, show the test now fails on the defect the finding describes (break the code locally, run, restore) before you commit. Say in your report which ones you checked this way.

## Ledger
- Decisions: D-1920..D-1939 only, appended at the end of `docs/05-decisions.md` as `### D-NNNN — title — 2026-10-03`.
- Invariant rows only where an invariant changes: prefix `ZT-` in `docs/04-invariants.md`, `| ZT-01 | must hold | full::module::path::test_name | ✓ |`, full module path.
- Gate rules: no `.iter().find(` or `.position(` (gate 11); no new local `#[allow(` on production lines; a `#[path]` test file starts with `#![cfg(test)]`; no new fully-lowercase no-space string literal under `crates/pull` unless gate 1d declares it.
- Before you finish: `cargo fmt --check`, `cargo clippy -p <crate> --all-targets -- -D warnings` per touched crate, and the touched tests pass. If `cargo mutants --version` works, run it with `--in-diff` on your changes; otherwise say it was not run.

## Return
A list: finding ID, fixed or not, commit, test name, whether you saw it fail on the defect. Under 60 lines.

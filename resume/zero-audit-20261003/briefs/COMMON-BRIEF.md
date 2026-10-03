# Common rules for every zero-findings fix agent

CLAUDE.md in the repo is the law; it is loaded for you. Rust only outside `web/`.

## Where to work
- Your own git worktree (isolation). First run `git checkout -b <your branch> 77448192` (head of final/all-fixes-zero). Commit on that local branch only. Never push, never open a PR, never create a tag. The main session merges.
- `CARGO_BUILD_JOBS=2`: up to six builds share this 4-CPU box. Run only the tests of the modules you touch, not the whole workspace.
- The container runs as root. Permission tests need non-root (`setpriv --reuid=65534 --regid=65534 --clear-groups ...`); if you cannot run one, say so.
- /mnt/project-files is shared with other sessions: read it, never write there. Install nothing. Never touch the network beyond what a test already does.

## How to fix
- Fix the finding fully; "documented only" is not a fix. If a fix needs a decision only the operator can make, do not guess: write it as NOT FIXED with the exact question.
- Every fix gets a test that fails on the defect and passes after; show it fails (break the code locally, run, restore) and say which you checked that way. No test that asserts nothing (CLAUDE.md §4).
- No silent fallback (§4): refuse by name or degrade loudly.
- Keep each change minimal and local to the finding.

## Ledger
- Decisions only in your assigned D-range, appended at the very end of `docs/05-decisions.md` as `### D-NNNN — title — 2026-10-03`. One entry may cover several findings.
- Invariant rows for new tests in `docs/04-invariants.md`, with your assigned prefix, as `| ZX-01 | must hold | full::module::path::test_name | ✓ |` (full module path; gate 10 resolves it). Append after the last existing `ZR-`/`Z*-` row block or at the table end.

## Gates that bite
- No `.iter().find(` or `.position(` (gate 11); no new local `#[allow(` on production lines; a `#[path]` test file starts with `#![cfg(test)]`; no new fully-lowercase no-space string literal under `crates/pull` unless gate 1d declares it; no new file extension outside the CLAUDE.md §2 list.
- Clippy is pedantic (`-D warnings`, indexing_slicing, expect_used, too_many_lines at 100).
- Before you finish: `cargo fmt --check`, `cargo clippy -p <crate> --all-targets -- -D warnings` per touched crate, the touched tests pass. For web changes run the relevant `node --test web/tests/<file>` (node_modules exists under web/). If `cargo mutants --version` works, run `cargo mutants --in-diff` on your diff; otherwise say it was not run.

## Return
One line per finding: ID, FIXED / NOT FIXED (why), commit, test name, whether you saw the test fail on the defect. Under 60 lines.

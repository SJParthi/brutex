---
name: brutex-ci-and-merge-gotchas
description: Non-obvious CI, test and merge traps in brutex learned by earlier lanes (ci.yml size limit, root-only tests, docs ledger conflicts, gate 11 timing)
metadata:
  type: reference
---
- GitHub silently refuses any CI run whose `.github/workflows/ci.yml` exceeds ~520 KB (run ends "failure", 0 jobs). Keep it well under 524,288 bytes; new Gate 11 allowlist reasons go in docs/06-limits.md "Gate 11 allowlist reasons" (D-1451).
- Cloud boxes run as root: 4 store permission tests (16-20 across api/pull/store/telemetry on older bases) fail only as root. Run tests as non-root, e.g. `setpriv --reuid=65534`.
- `crates/cli/src/step3_all_rung_tests.rs` was a 58-min test, cut to ~8 min by lane 1's PR #40.
- Merging branches conflicts at the tails of append-only docs/04, 05, 06, 11: keep both sides (base first), then `cargo check --workspace --all-targets` / `cargo test -p core`. Merge commits only, no rebase on shared branches.
- Gate 11 runs only after Gate 1e (~70-80 min), so refusals show late; extract its step from ci.yml and run locally after `git add -A` (~25 min). Common refusals: `.iter().find/position(` (rule 6), local `#[allow(` on production lines, `#[path]` test file without `#![cfg(test)]` on line 1. Gate 1c: no new lowercase string literal in crates/pull.
- Build with CARGO_BUILD_JOBS=2, one target dir per worktree. Full cloud build ~7 min.
- Never touch the Mac's `~/IdeaProjects/brutexOld` (only copy of Apr-Aug history); local Mac work only under WD_BLACK.

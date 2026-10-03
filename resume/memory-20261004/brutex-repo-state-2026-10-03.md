---
name: brutex-repo-state-2026-10-03
description: Snapshot of brutex repo shape and in-flight work as of 2026-10-03 (PR #74 fold-up branch)
metadata:
  type: project
  modified: 2026-10-03T04:02:52.522Z
---

As of 2026-10-03: brutex is a Rust-only (web/ exempt, Svelte) brute-force backtester over NSE 1-minute bars (NIFTY, BANKNIFTY, 208 F&O stock equities). 13 crates, ~680k lines Rust, ~70k lines of docs, decision ledger past D-1450.

In flight: PR #74 `final/all-fixes` folds 52 earlier fix PRs (#21, #23-#73, now closed) plus lane branch-only fixes into one merge. Known exceptions in its body: 4 store catalog lock tests fail only as root; an api fno_boundary test hangs under D-1203 Governor::reserve (lane 3 fixing). ~150 stale remote branches (fix/c4-*, fix/cloud-*, lane2-wip/*, bundle/*) remain. No open issues.

Operator-blocked items (docs/07-plan.md §3): moving 194 misfiled GDFL months out of bars/dhan/, Dhan token mint (tickvault conflict), TrueData descriptor needs a real archive.

**Why:** saves the next session re-surveying a very large repo.
**How to apply:** verify PR #74 state with list_project_prs before repeating; see [[brutex-project]].

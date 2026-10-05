---
name: brutex-ci-gotchas
description: Brutex CI and merge pitfalls on final/all-fixes (ci.yml size cap, root tests, static gates locally, Gate 6d, mutants)
metadata:
  type: reference
  modified: 2026-10-04T07:09:43.206Z
---

- `.github/workflows/ci.yml` over ~520 KB makes GitHub silently refuse the run (0 jobs). Keep under 524,288 bytes; Gate 11 allowlist reasons go in docs/06-limits.md.
- Cloud box runs as root: store lock/permission tests fail as root. Run tests with `setpriv --reuid=65534 --regid=65534 --clear-groups`.
- Merges conflict at tails of append-only docs/04, 05, 06, 11: keep both sides (base first), then `cargo check --workspace --all-targets`, `cargo test -p core`, grep for conflict markers.
- Static gates locally: extract `jobs.language-purity.steps` from ci.yml, run with `RUNNER_TEMP=/tmp/claude-0 SOURCE_SCAN=/tmp/claude-0/source-scan` from repo root; skip 1e (full build). Common refusals: `.iter().find/position(`, local `#[allow(`, `#[path]` test without `#![cfg(test)]` line 1; Gate 1c/1d new lowercase literals in crates/pull; Gate 14 `step-runs` refuses needle inside if/loop; Gate 15 bans other language NAMES in any tracked text.
- Gate 6d (D-1607) builds web/*.rs native tests with rustc against the workspace's rlibs, so workspace feature changes (e.g. serde_json `arbitrary_precision`, D-1570) reach web/ code that cargo test never runs. Reproduce with the ci.yml step locally.
- Box: 4 cores/15 GB; CARGO_BUILD_JOBS=2-3; full `cargo bench --workspace` OOMs (fat LTO); Gate 8 in CI is the authority.
- Mutants: cargo-mutants 26.2.0; `--baseline skip --timeout 900 --in-place --jobs 1 --in-diff x.diff -p <crate>`; read shard survivors via get_job_logs (artifact download proxy-blocked). Never skip a mutant (D-0192).
- `cargo test -p core` (lib `brutex_core`). Never `pkill -f` a pattern contained in its own command. In tests, `+` in a query is `%2B`.
- Gate order: W, 1+2 (1e ~30 min), 3-6 (~50 min), then Gate 8, coverage, Gate 18 shards (only when the rest is green), then ci-ok. Auto-merge workflow merges #74 when ci-ok is green.
- Disk allowance is ~40 GB writable per session: two parallel mutant worktrees (4-8 GB targets each) plus main target filled it on 2026-10-04 and killed every run with ENOSPC/SIGBUS. Run mutant jobs one at a time and delete each worktree target after.
- Gate 8 (crates/pull segment-literal scan) refuses test literals like `1e999999999`; build such inputs with format!/repeat.
- Gate 6c runs `rustfmt --check` on `.github/*.rs` gate tools; `cargo fmt` does not touch them. Run `rustfmt --edition 2024 --check .github/*.rs` before pushing a tool change.
- A push to final/all-fixes cancels the running CI (Gate 18 shards included). When the PR 74 CI thread owns the branch, send batches to it rather than pushing during a run.
- Gate 18 is 127 shards, max-parallel 20, 240-min timeout; on run 1271 none finished in 70 min, so the shard phase takes many hours. Pushes are cheap before the shards start (about 2 h into a run) and very costly after. Watch CI with curl on api.github.com through the proxy (15000/hr).

Related: [[brutex-resume-facts]], [[user-rules]].

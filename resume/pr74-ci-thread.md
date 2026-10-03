# PR #74 CI-to-green thread: resume state (2026-10-03 18:35 UTC)

## Where it stands
- final/all-fixes head **8a59ff4** (pushed 18:34 UTC). CI run on it was just starting at the stop.
- Previous run 1261 on 0cab319: Gates 0-2, 1e, W, 3-6 (fmt, clippy, deny, all tests) and Gate 18's planner were **green**. Two jobs failed, and both are fixed in 8a59ff4 (D-1461):
  - Gate 8: `crates/store/benches/ratio.rs` relied on `open_or_create` creating the store root, which D-1522 stopped. The bench now creates its own root. Local: "all ratios within the ceiling".
  - Coverage: `api operation_audit::tests::every_registered_route_is_audited_or_exempt_by_name` used `std::ptr::eq` on two copies of one const literal, which the coverage build did not merge. It now checks the label is not the request's heap bytes. Local: 15/15.
- The Gate 18 mutation shards have NOT run on this content yet: they were skipped because Gate 8 and coverage were red. They are next. Sizing per D-1453 is about 78 jobs, 20 at once.
- The full `cargo bench --workspace` was not finished locally (OOM on a 15 GB box with fat-LTO cli; rerun with CARGO_BUILD_JOBS=2). If Gate 8 is red again, it is another crate's bench.

## Next steps on resume
1. List jobs of the newest CI run on final/all-fixes. Fix any red gate on a local branch made from origin/final/all-fixes.
2. When mutation shards finish, read MISSED/TIMEOUT lines from failed shards' logs (the artifact download is proxy-blocked; use get_job_logs). Fix survivors by crate in worktrees, merge, validate (fmt, clippy, static gates extracted from ci.yml job `language-purity`, affected tests as non-root), then push ONCE (merge-based, never force).
3. Coordinate with the audit-fix thread: it sends its batch commit before pushing; fold it into the one push.

## Decisions used by this thread
D-1452..D-1461. Use D-1462+ next, or D-2000+ for new work.
- D-1452 runner survivors; D-1453 Gate 18 sizing (ESTIMATED_CASE_SECONDS 540, max-parallel 20); D-1454 api, D-1455 runner/vocab, D-1456 cli/pull survivors; D-1457 every workflow action pinned to a commit SHA; D-1458 gates 23/21/11/K-44; D-1459 gate 12; D-1460 gate 8's empty-bench refusal is a top-level `git ls-files --error-unmatch` (since D-1601, `step-runs` refuses lines inside `if`); D-1461 above.

## Gotchas learned
- Extract static gates with python yaml from `jobs.language-purity.steps` and run with `RUNNER_TEMP=/tmp/claude-0 SOURCE_SCAN=/tmp/claude-0/source-scan` from the repo root. Gate 1e (03) is a full build; skip it locally.
- Gate 15 bans a language name in all tracked files, prose included. Gate 1d needs every new segment-shaped literal in crates/pull declared in ci.yml.
- CLI test failure from the audit run: not reproducible (green in CI and 5 local runs); cause not recoverable.

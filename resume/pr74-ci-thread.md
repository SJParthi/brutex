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

## Exact prompt for the new account (paste as-is into a Claude Code session on SJParthi/brutex)

```
Resume the PR #74 CI-to-green work on SJParthi/brutex. First read, in full,
these files on branch fix-queue: resume/pr74-ci-thread.md (this thread),
resume/RESUME-20261003.md and resume/audit-20261003/RESUME.md, plus CLAUDE.md on
final/all-fixes. Then:
1. Find the newest CI run on branch final/all-fixes (PR #74) and list its jobs.
   For any red job, read its log, reproduce locally, fix it, and validate:
   cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings,
   the static gates (extract jobs.language-purity.steps from
   .github/workflows/ci.yml), and the affected crates' tests as a non-root
   user (setpriv --reuid=65534).
2. When the Gate 18 mutation shards finish, read each failed shard's MISSED
   and TIMEOUT lines from its job log. Kill each survivor with a real test, or
   restructure the code (D-0192: never skip a mutant). Work crate by crate, in
   parallel worktrees.
3. Fetch origin/final/all-fixes and merge it before pushing (merge commits
   only, never force-push). Push ONCE per round, because every push restarts
   the ~6 h CI. Open no new PR: #74 is the one final combined PR.
4. Record each locked choice in docs/05-decisions.md, starting at D-1462. Each
   new invariant goes in docs/04-invariants.md with the test that proves it.
5. Repeat until ci-ok is green on #74.
My rules: Rust only except web/. O(1) wherever possible, naming anything that
can't be. Work in parallel. Attack extreme edge cases adversarially. Verify
with real evidence and never guess. Never ask me to tap, paste or approve
anything. Keep everything running continuously. Stop at 98% weekly usage after
saving resume state to fix-queue. Deliver results as easy comparison-table
Artifacts.
```

## Status at the 18:50 UTC stop
- Local `cargo bench --workspace` was inconclusive: it was killed for lack of memory twice (on the cli and api lib-test builds, fat LTO, 15 GB box). Only the store bench was run on its own, and it passed. Gate 8 in CI is the authority.
- No agents and no open local work. Everything is pushed (8a59ff4 on final/all-fixes).
- 20:07 UTC update, from the Fix Board thread: on run 37144534483 (head 8a59ff4), Gates 1+2, W, 3-6 and 18 (the planner) are green. Gate 8 died at 20:01 UTC with "The runner has received a shutdown signal" (exit 143) while `api` was compiling, before any bench ran. That is runner loss, so it qualifies for ONE re-run of failed jobs once the run completes. Coverage was still running. FIRST ACTION on resume: when run 37144534483 completes, re-run its failed jobs once (actions_run_trigger rerun_failed_jobs). A second Gate 8 failure is real.
- 21:12 UTC update (Fix Board thread): another session pushed head **b23976f** at about 20:17. It merged wip/audit-fixes-9 and added D-1853, which supersedes 8a59ff4's run and its owed re-run. Run 37151036264 failed Gate 1+2 at 20:44 on the telemetry test `sink::tests::a_file_that_ends_mid_line_is_terminated_at_open_and_not_appended_onto` (sink.rs:4116). The reopen is refused with "another sink ... holds this telemetry directory", which looks like D-1537's events.lock flock still being held by the first sink when the test reopens. FIRST ACTION on resume: fetch b23976f, reproduce that test, and fix it (drop the first sink before the reopen, or the lock release). Then push once.
- 21:15 UTC: the pusher of b23976f says its diff from 8a59ff4 touches only cli, docs and a runner test, and that the sink test passed in its full local run (6,726 passed). The one-sink lock came in with 347c77b5, already on 8a59ff4, so this is most likely a lock-release race in that test, red only intermittently. **This thread (PR #74 CI) owns it on resume.** Root-cause it, never just re-run: find what still holds `<dir>/events.lock` when the test reopens. Also in resume/audit-20261003/NEXT.md.
- 2026-10-04 ~04:10 UTC: resumed. Root-caused the telemetry sink failure: a forked child (`where_permission_binds`, `.uid()` means fork then exec) inherits the `events.lock` flock. Reproduced 4/60 locally, 0/60 after the fix. Pushed **9eeff08** (D-1462). All 29 static gates pass and telemetry clippy is clean. Next: watch run on 9eeff08 (Gate 8, coverage, Gate 18 shards). Next D-number: D-1463.
- 2026-10-04 05:31 UTC status: run **37175179591** on 9eeff08 is in progress. Gate W, Gate 1+2, Gates 3-6 (fmt, clippy, deny, every test) and the Gate 18 planner are **green**. Gate 8 and Coverage started 05:18 and are running. The mutation shards start after both pass. No job has failed. The final/all-fixes head is still 9eeff08, nothing is unpushed, and no agents are running. Next D-number: D-1463.

## FINAL SAVE, 2026-10-04 05:57 UTC (user switching accounts)
- **final/all-fixes head: c97ff00**, pushed by the audit-fix thread at about 05:42. It fast-forwards from 9eeff08 and adds 10 audit fixes: api D-1551..1553, cli D-1556/1557/1569, and data D-1570..1572 with store format v3.
- The last complete signal is from run 37175179591 on 9eeff08. Gates W, 1+2, 3-6 (all tests), **Gate 8** and the Gate 18 planner were all GREEN. Coverage was still running when c97ff00 superseded it.
- The CI run on c97ff00 was started by that push and has not been checked. FIRST ACTION: list its jobs and fix anything red, root cause and one validated push. Then work the Gate 18 mutation shards, which have never run on this content. c97ff00 adds new mutation targets in api, cli, store, pull and runner.
- The audit-fix thread said one more "features" batch (gaps-5, gaps-10, gaps-11) is still to come. Coordinate so it lands in one push.
- This thread has no unpushed work, no agents and no timers. Next D-number: D-1463.

## 2026-10-04 07:45 UTC (new account, project thread "PR #74 CI")
- Run 1265 (37180670656) on c97ff00 failed only Gate 6d: two web/ native tests in `deployment-preflight-tests.rs`. Cause: D-1570's workspace `arbitrary_precision` reaches web/*.rs through Gate 6d's shared serde_json rlib; `files::json` decoded `0.125` as an object and accepted `1e999`.
- Fixed in **73441e5** (D-1463): `visit_map` decodes serde_json's number-token map into its digits, refuses nonfinite/forged numbers; new test `report_json_decimals_keep_their_digits_and_refuse_nonfinite_or_forged_numbers` + invariant row. Reproduced 3 failures on the old reader, 0 after; all four Gate 6d binaries and 29 static gates green locally.
- Next: watch the run on 73441e5 (Gate 8, coverage, Gate 18 shards). Next D-number: D-1464.

## 2026-10-04 13:35 UTC — bb6b3b4 pushed (new account, thread 5.1)
- final/all-fixes = bb6b3b4 = sweep 8c16ba3 (ci.yml gates ported to .github/gates_*.rs) + D-1464.
- D-1464: Gate 18 survivors killed ahead of CI (runner BlockExtremes::of x3 + outcome tests, pull number_text 12/12 caught, telemetry hold_directory unreachable arm removed); Gate 20 non-root count = clock 1, level 1, lib 6, record 1, sink 20, tail 3 (re-measured on bb6b3b4 incl. sweep's OD-4 sink lines).
- Run 1267 (8c9313c): only Gate 20 red; 6d, tests, Gate 8 green; Gate 18 shards skipped. CI run 1269 on bb6b3b4 is the first that reaches Gate 18 shards.
- Attack-audit runner survivors exit_grid_policy.rs:86 and research_exit_grid.rs:204 are caught by full suite (filter artifact). Local cli mutant runs (wt-a, wt-c) died on disk-full; CI Gate 18 shards are authority now.
- Local gates: extract jobs.language-purity steps with python yaml to /tmp/claude-0/gates2, run with RUNNER_TEMP and GITHUB_ENV set, source GITHUB_ENV between steps.

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
- 13:43 UTC pause-ready. Open items: watch run 1269 (id 37205839728) on bb6b3b4; read Gate 18 shard survivors via get_job_logs and route attack-audit ones to session_016tvv63ByrRGE6qoEFR5ctD. If cli lib.rs traded_preamble "xyzzy" mutant TIMEOUTs in CI, the attack-audit thread adds a fast unit test. send_later trig_016wFqHSSpwEsDi4JQjSUMwt fires 14:16 UTC. Sweep thread has OS-7 (D-2319) coming; lane 1 merges bb6b3b4 before pushing.
- 17:10 UTC: pending merge of sweep 913b9dc (branch claude/project-thread-v8j0jv, D-2320, .github+docs only) into next push after run 1271 Gate 18. Local mutant pre-run of sweep diff (8c9313c..1071b51) in wt-c, results /tmp/claude-0/mp/sw/summary.txt.

## 2026-10-04 18:50 UTC pause point (usage 93% guard)
- final/all-fixes = 8b1e5bd = 1f588aa + sweep c863e92 (D-2321..2331, new Gate 6d root discovery). Contents since bb6b3b4: sweep b97b2ca/913b9dc/c863e92, W3 fix 1071b51, lane1-b 31f1619 (D-2100..2105), gate-tool rustfmt 956424c, pull holds_exactly mutant fixes ddf6d69+66edfe2, Zerodha pull order 2d9e7f2 (D-3000..3002), Fix Board web18 891d8f8.
- All 29 static gates, fmt, clippy pass locally on 8b1e5bd. Gate 6d new step was running locally (/tmp/claude-0/g6d.log); sweep asked to watch 6d in CI first.
- Local Gate 18 pre-run on sweep diff 8c9313c..1071b51: telemetry/engine/store/lake/pull clean after holds_exactly fixes; runner and api in progress (/tmp/claude-0/mp/sw/).
- Run 1271's shards: none finished in 70 min (timeout 240, max-parallel 20, 127 shards): Gate 18 takes many hours. Push only before shards start.
- Policy agreed: lane1-b and sweep send batches to this thread, no direct pushes. zero-work: zero-findings thread will merge onto head and hand one validated commit (coordinator).
- Watch CI via public API through proxy: curl https://api.github.com/repos/SJParthi/brutex/actions/runs?branch=final/all-fixes (15000/hr).
- 19:20 UTC: head is 8f58d91 (8b1e5bd + Fix Board web18 605c529). CI run 37226157475 on it; Gate W green. New Gate 6d passed locally. Local runner mutants 22/78, 0 missed so far; api next. Paused for usage at 93%; resume 22:03.
- Pending at pause: Fix Board hand-off 3 origin/fixboard/pr74-conc-api dee61cfd (api+pull Rust, D-2760..2766, already merges 8f58d91). Merge on resume; shards will likely have started, so decide merge-now vs after the Gate 18 phase.
- Local survivor (fix on resume): crates/runner/src/bootstrap.rs:1847:17 replace > with == in ExactPrefix::sum (see /tmp/claude-0/mp/sw/runner/mutants.out).
- Pending at pause: sweep 446c8cd on claude/project-thread-v8j0jv (zf/store-o1 D-2329/2330, .tix time index, store benches C-TIX-01/02; already merges 8f58d91).
- Pending at pause: attack audit batch 2 wip/audit-batch2 6cf0582 (D-1854/1860/1861/2270/2271; on fc6dbb9, merges cleanly with 8f58d91). Reply SHA to session_016tvv63ByrRGE6qoEFR5ctD after merge.

## 2026-10-05 02:00 UTC pause point (weekly usage 95%)
- final/all-fixes = 969493e1 (also on claude/project-thread-jkytmf). Contents since 8f58d91: Fix Board dee61cfd (D-2760..2766), sweep 446c8cd (D-2329/2330), attack audit 6cf0582, runner ExactPrefix::sum mutant test, Gate 1d PULL_FIXTURE literals, Gate 8 CARGO_BUILD_JOBS 2 + timeout 40 (D-1465), lane 1-b ac0802f (D-2106), zero-findings hand-off 6db4dfb8 (all of zero-work, D-1934..1939, D-1969, D-2660..2690), and D-2001 (cli elite_descend checks top_refusal 1000 before frontier 4096; fixed the one Gate 1e test failure on 6db4dfb).
- Every hand-off received has been merged. Nothing pending from other threads.
- CI on 969493e: just started at pause. Not yet proven on this head: Gate 1e full build, gates 3-6, Gate 8 with D-1465 (it OOMed twice before), coverage + Gate 20, Gate 18 shards, ci-ok. Gates W, 1+2 static, and 3-6 tests were green on 25bc8aa/a3f6e8a; static gates pass locally on 969493e.
- Local api mutant pre-run output was lost (disk cleanup); never rerun. Gate 18 in CI covers it.
- Next steps: (1) find the run for 969493e via curl https://api.github.com/repos/SJParthi/brutex/actions/runs?branch=final/all-fixes&per_page=1 ; (2) on any red job read logs with mcp__github__get_job_logs (job id from /actions/runs/<id>/jobs), fix, validate (fmt, rustfmt --edition 2024 --check .github/*.rs, clippy -D warnings, the affected tests as uid 65534, static gates from ci.yml language-purity steps skipping 1e), push to final/all-fixes and claude/project-thread-jkytmf ONLY before Gate 18 shards start; (3) after shards start, do not push until the run ends; collect all survivors from shard logs and fix them in one push; (4) ci-ok green = auto-merge of PR 74.
- Next free decision numbers for this thread: D-2002 onward (block 2000).

### Prompt for a new session (paste as is)
Read CLAUDE.md, then the fix-queue branch file resume/pr74-ci-thread.md (last section) in sjparthi/brutex. You are the PR 74 CI thread: drive PR #74 (head final/all-fixes) to ci-ok green. Find the latest CI run on final/all-fixes, fix every red job and every Gate 18 mutation survivor, validate locally before each push, and push only before Gate 18 shards start. Append progress to resume/pr74-ci-thread.md on fix-queue. Do not merge other branches unless a thread hands one over as validated.
- Update: run 37251141390 on 969493e: W, 1+2 (incl. 1e), 3-6, Gate 8 (D-1465 works), Gate 18 plan all green. Remaining: coverage + Gate 20, Gate 18 shards, ci-ok.

## 2026-10-06 03:15 UTC: coordinator session session_01UGhT9yCM8wk4VFp9A2cjt1 (resumed from START-HERE)
- final/all-fixes head still 969493e1. Run 1283 (37251141390): W, 1+2, 1e, 3-6, Gate 8, Coverage, Gate 18 plan GREEN. Gate 18 shards: 150 failed, 13 cancelled (11 never got a runner, 2 hit 4 h), 22 green, 23 still running/queued at 03:10.
- 296 distinct survivors collected from check-run annotations (curl api.github.com/repos/.../check-runs/<job>/annotations works through the proxy; blob log download is 403). Lists: resume/pr74-g18-run1283/survivors-*.md.
- Parallel sessions started (tag brutex-20261006), prompts in resume/pr74-g18-run1283/prompts/:
  | session | work | branch | D range |
  |---|---|---|---|
  | session_01DJvVUSWVanqmvaJm7Hu57Z | G18 cli-a (73) | pr74/g18-cli-a | D-2002..2019 |
  | session_015LgncCWJsrUwHGbV78iP4s | G18 cli-b (81) | pr74/g18-cli-b | D-2020..2039 |
  | session_01DgSCthYoWpqqwsSinpVTQF | G18 api (50) | pr74/g18-api | D-2040..2054 |
  | session_01RrcitsqaTTNzPaQusAXZmM | G18 runner (45) | pr74/g18-runner | D-2055..2069 |
  | session_019Rqb9Yi2ndZ4bzvwo7AYFx | G18 rest (47) | pr74/g18-rest | D-2070..2089 |
  | session_01UunmFW6rqJzr4tvqZ6kcbq | WS2 data path | claude/attack-data-pipeline-hgxmw9 | D-3127..3139 |
  | session_01CbHGWDcRRiPcQWEMPH5JHg | WS3 Fix Board | fixboard/pr74-batch3 | D-2779, 2795..2799 |
  | session_01JdvYEitZmXjZfqekdkTn15 | WS4 attack audit | wip/audit-batch3 | D-1842..1849, 2292..2299 |
  | session_01QLkK4a9pEwwpAEMZhwMK3q | WS5 zero-findings | zero/next | D-2691.., 2500.. |
- Plan: no push to final/all-fixes until run 1283 ends. Then merge every handed-over branch into final/all-fixes, validate, ONE push.
- 03:40 UTC: owner sent new standing rules (zero bugs with evidence, attack every extreme permutation, O(1) with measured p99, everything persisted/logged/searchable/visible at O(1), Rust only everywhere except web/, one common incremental runtime, everything in parallel, comparison-table artifact). Saved as resume/pr74-g18-run1283/prompts/owner-rules-20261006.md and sent to all 9 sessions. WS2 now continues attack rounds until a round finds zero; WS3 continues with its own unclaimed found rows on fixboard/pr74-batch4.
- 03:38 UTC: four attack-lens sessions started (prompts in resume/attack-lenses-20261006/prompts/, trackers + RESULT files go in resume/attack-lenses-20261006/):
  | session | lens | branch | D range | invariant prefix |
  |---|---|---|---|---|
  | session_011y3kRe4mCpbx4d7HK6WEPo | L1 observability/persistence/audit | attack/observability | D-3200..3299 | OBSV- |
  | session_01XZSFAKEtQLAHWiX5iWiHrM | L2 O(1) with measured p99 | attack/o1-p99 | D-3300..3399 | O1P- |
  | session_01R51JmkeR6PuVuzyEFgGFXq | L3 permutations + differential | attack/permutations | D-3400..3499 | XPERM- |
  | session_01HjzyWzsojN9XoNtyh6n1Dd | L4 Rust-only/one authority/docs drift | attack/one-authority | D-3500..3599 | ONEAUTH- |
  WS2 overflow range D-3700..3799.
- 03:47 UTC: Brutex Status Board republished in place (https://claude.ai/artifact/6e7CWygfPUuz3P9Vz6E15f, version 3). It is now a live page backed by the artifact's db (rules: read view, write admin). Collections: board/summary (tiles), workstreams, gates, survivors, speed, rustonly, owner; every row has `order`. Update rows with ArtifactData set/update/batch (pin `if_version`), never by republishing. Seed source: the scratchpad mkseed.py of this session; numbers cited in each row (docs/06-limits.md, run 1283, trackers).
- Rust-only scan on 969493e1: 1041 tracked files, 0 violations; no native code compiled (ring closed by D-0211); stale ring comment in crates/pull/Cargo.toml handed to lens L4. Evidence: resume/pr74-g18-run1283/rust-only-scan-20261006.md.
- 03:55 UTC: reproduced run 1283's Gate 18 plan locally (gates_jobs mutant-diff with BASE_REF 4bbd37df, respell, cargo mutants --list): 6236 cases, 202 jobs, 31 per job, matches CI. The 21 shards CI never tested (8 97 111 116 119 120 125 126 128 131 132 136 140 141 142 144 145 146 150 167 168) hold 651 mutants: cli 211, api 126, runner 116, pull 63, store 52, indicators 22, lake 21, vocab 20, engine 18, greeks 2 (list: resume/pr74-g18-run1283/untested-mutants-run1283.md). Running them on the coordinator box with CI's flags, crate by crate, detached: /tmp/claude-0/mut-untested/run.sh, summary in /tmp/claude-0/mut-untested/summary.txt, worktree /tmp/claude-0/wt-mut on 969493e1. Survivors go to the per-crate fixer sessions.
- Usage guard: resume/pr74-g18-run1283/USAGE-GUARD.md. Polls at 04:22 UTC (trig_01YDHbcwnkgZJab6fnT89iZC), resume at 07:12 UTC (trig_01UoWW9ugvVA2FCgxT1nT1gM).
- 04:08 UTC INTEGRATION PLAN (coordinator box tools, all scratch, never committed):
  1. local branch `integ` = origin/final/all-fixes (969493e1). Merge (merge commits only) as each branch is handed over with a RESULT file: pr74/g18-rest, -runner, -api, -cli-a, -cli-b; then fixboard/pr74-batch3, wip/audit-batch3, claude/attack-data-pipeline-hgxmw9, zero/next, attack/* (validated rounds only).
  2. Validate the merged tree: cargo fmt --check; rustfmt --edition 2024 --check .github/*.rs; cargo clippy --workspace --all-targets -- -D warnings; static gates `python3 /tmp/claude-0/run_gates.py <repo>` (29 language-purity steps, 1e skipped); tests as uid 65534 `/tmp/claude-0/run_tests_nobody.sh <repo> <logdir>` (PKGS="-p x" to narrow); coverage + Gate 20 with cargo-llvm-cov 0.8.4 (`cargo llvm-cov --workspace --locked --fail-under-lines 90 --fail-under-regions 89 --summary-only`, then the Gate 20 step from ci.yml); web gates W1-W6 if web/ changed; complexity (Gate 8 ratio benches) for changed crates with CARGO_BUILD_JOBS=2.
  3. Independent kill checks: re-run each fixer's claimed mutants on its head with CI's flags (done for g18-rest: /tmp/claude-0/verify-rest).
  4. ONE push of integ to final/all-fixes (git push origin integ:final/all-fixes), never force. Then watch the run; collect survivors from annotations (curl api.github.com/.../check-runs/<job>/annotations).
- g18-rest review (99d8217f): three production rewrites checked and behaviour-preserving (vocab place(): depth-1 stack starts at 0 so right.checked_sub(1) is the guard; table.rs MAX_NAME_BYTES saturating_sub == max; costs gst max over BpsX100(i64) newtype).

## 2026-10-06 04:45 UTC PAUSE (usage guard, 5-hour window ~70%; resets 07:10 UTC)
- All 13 sessions told to PAUSE (commit+push WIP on their own branch, write PAUSE-*.md, end turn). Resume trigger: 07:12 UTC (trig_01UoWW9ugvVA2FCgxT1nT1gM) sends RESUME to each.
- Coordinator box keeps running machine-only jobs (no tokens): /tmp/claude-0/mut-untested (runner now; then api, cli; then vocab2, shard 138 via after.sh/after2.sh; store+pull IN PLACE via after3.sh in /tmp/nobody-wt), /tmp/claude-0/verify-rest3 (g18-rest bb736104: telemetry, pull, store in place in /tmp/nobody-wt-rest). Root-run caveat: store/pull tests re-exec as uid 65534 and fail in cargo-mutants' private temp copy; run them --in-place in a world-readable worktree.
- Found this window: (1) 23 Gate 18 TIMEOUTS missing from the annotation lists (cargo-mutants annotates only MISSED); lists resume/pr74-g18-run1283/timeouts-*.md, sent to fixers. Total to fix 332. (2) vocab compile-time loop hang (shards 146, 150 at 240m): fixed by g18-rest 3565e479 (D-2083), coordinator re-run 41/41 caught or unviable, 0 timeout; CI bound committed on local integ 46439dee (D-2090, --build-timeout-multiplier 2; proven TIMEOUT in 60s build). (3) g18-rest's walk_back TIMEOUT (u64 pos >= 0): fixed bb736104 (D-2084), verification running. (4) g18-rest 99d8217 had committed a live mutation (repaired 0d8c386); all sessions now grep for the cargo-mutants marker before committing; L4 asked to add a gate for it.
- Verified so far: g18-rest kills costs 1/1, lake 6/6, indicators 12/12; g18-rest and g18-runner production rewrites reviewed as behaviour-preserving. Untested-shard mutants: greeks 2/2, engine 18/18, indicators 22/22, lake 14+7 unviable, vocab 8+2 unviable; 0 survivors so far.
- On resume: re-arm the monitor (marker scan), read PAUSE-*.md files, check local job summaries, update the board, continue integration (local integ = 969493e1 + 46439dee).

Number ranges assigned 07:51 UTC 2026-10-06. Before assigning, every live branch head was
checked: none uses D-36xx, and none uses FB-110..149.
- D-3600..D-3649 and FB-110..FB-139: WS3 Fix Board batch4 (fixboard/pr74-batch4).
- D-3650..D-3659: Gate 18 rest fixer, overflow for the untested-case store survivors.
- Still free: D-3660..D-3699.

## Integration branch state, 08:20 UTC 2026-10-06 (local branch `integ` in the coordinator's checkout, not pushed)
- 969493e1 (final/all-fixes) + 46439dee (D-2090 build bound)
- + c268344f: merge of attack/o1-p99 @3539d87 (L2 done; round 15 was its zero round)
- + e62e3fe5: merge of pr74/g18-runner @c216c97 (runner survivors all killed; M3 56/56)
- Both merges hit append-only conflicts in docs/05-decisions.md; each was resolved as
  ours-then-theirs with /tmp/claude-0/union_merge.py. After both merges, fmt is clean
  and all 29 static gates PASS. Clippy and the nobody-uid tests run once the next
  hand-overs are in.
- Pre-existing defect: docs/05-decisions.md on final/all-fixes has `### D-0370` twice and
  `### D-0372` twice. Routed to L4 (docs drift).
- The runner fixer was reassigned at 08:20 to the 121 runner cases CI never tested
  (D-2065..2069, then D-3660..3669).

## Integration validation at cab2f2ae (09:40 UTC)
- fmt clean; 29/29 static gates PASS; workspace clippy -D warnings PASS.
- Tests (each binary as uid 65534): 122 binaries pass, 3 fail.
  - pull `unit` (2 tests at crates/pull/tests/unit.rs:112) and store `cited_commits` (2 tests):
    both pass as root, so the failures are environmental. In pull, the shared /tmp/brutex-pull
    dir was created earlier by root. In store, git refuses a repo owned by another uid.
  - core `findings` FAILS as root too, so it is REAL. L2 (attack/o1-p99) appended 10 rows to
    docs/11-findings.md without updating the disposition tally (stated 111, has 121), the
    "N stood" prose, or the row digest. It also marked rows FIXED with branch commits:
    F-8D5719 names f80e4d11, which is not on main (the test requires IN PROGRESS until the
    squash merge), and F-A86CB4 says FIXED with no sha. L2 never ran core's tests (its RESULT
    lists touched-crate tests only).
  - Owner of the fix: L2, at the 12:12 RESUME. Nothing is pushed until it passes.

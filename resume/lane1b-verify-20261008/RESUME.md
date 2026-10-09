# Lane 1-b verify and finish — resume state (thread started 2026-10-08)

Owner ask (2026-10-08): "Redo the 55 missing lane 1 fixes and push them onto final/all-fixes (PR #74)."

## Fact found first
The 55 were NOT missing: the redo on `final/all-fixes-00bbns` (ae479d2) and follow-ups D-2100..D-2106
(last ac0802f) are ancestors of PR #74 head fbdabaec. RESUME-20261003.md predates that merge.
So this thread re-verified every id on fbdabaec and is fixing what is not fully fixed.

## Verification on fbdabaec (static, 5 checkers; reports G1..G5.md here)
56 ids (55 NOT-FIXED + W2-cli8-6 PARTIAL from v1.md): 33 FIXED, 6 FIXED-BOUNDED, 13 PARTIAL,
4 DOCUMENTED-ONLY, 0 NOT-FIXED, 0 REGRESSED; ~25 new small defects (G1-1.., G2-1.., G3-1.., G4-1.., G5-1..).

## Fix lanes (local worktrees, branches lane1b/fx-a..e from fbdabaec; decision range D-4700..D-4799 (moved from D-4500.. on 2026-10-09))
| Fixer | Items | D-numbers | Invariant ids |
|---|---|---|---|
| A | G1-1 pool-oos pass 1 order + sharing, G1-2, G2-4, W2-cli8-9/GAP13-13/GAP13-15 test gaps, G1-3, stretch R9-cli-o1-1 | D-4700..4715 | L1FA- |
| B | G2-5 recorded audit capture cost, W2-cli8-10, W2-cli8-11, W2-cli8-6+G2-3 census, G2-1, G2-2, W2-cli8-4/5/7 test gaps | D-4716..4731 | L1FB- |
| C | G5-2/W2-cli13-4 FIFO opens, G5-1, G5-3, G5-4, G1-4/R9-cli-law-1, G3-3, G3-5, G3-8, web zero ceiling | D-4732..4747 | L1FC- |
| D | G3-1/GAP12-6 Boolean CAS close, G3-4 identity reason codes, AC-whp-law-2, GAP15-21/G3-2 banners, G3-6, G3-7, G3-9, F-CEC7A0 | D-4748..4763 | L1FD- |
| E | W2-cli12-1, G4-1, W2-cli11-3, W2-cli11-2, W2-cli12-2 (population ledgers O(1)) | D-4764..4779 | L1FE- |
| F (queued) | W2-cli3-4 one writer per run, G4-4, W2-cli3-3, G4-2, G4-3, W2-cli7-2 decision | D-4780..4795 | L1FF- |

W2-cli7-2 default chosen by this thread: replay stays a full re-proof (a replay that trusts its own sealed
output proves nothing new); cost stated as the price of the proof. G4.md §A has the cheaper sealed-manifest
design if the owner prefers it.

## Landing
Do not push to final/all-fixes directly (every push cancels the long Gate 18 run). Merge the fixer branches
into `final/all-fixes-xp04wq`, validate (fmt, clippy, full tests as non-root, static gates, cargo-mutants on
the diff), push that branch, and send branch + sha to the PR 74 CI thread to land in its one combined push.
If #74 has merged, open exactly one new PR instead.

## Snapshot 2026-10-09 ~04:50 UTC (usage pacing: save so a stop loses nothing)
Fixer work saved as patches in `patches/` (base fbdabaec): `fx-<x>.commits.patch.md` = `git format-patch --stdout fbdabaec..HEAD`
(apply with `git am`), `fx-<x>.uncommitted.patch.md` = uncommitted WIP (`git apply`). Heads at save: A 93489016 (5 commits),
B 48af6b95 (2, WIP tests+fixes), C 63b2beba (9), D 20577d69 (8), E fa3100b9 (5), F none committed (WIP diff only).
Baseline on fbdabaec: full workspace tests as root 7,364 passed, 0 failed, 15 ignored.
Usage rule (owner 2026-10-09): after the in-flight fixers finish, at most 2 agents at a time; any usage-limit error =
save here, pause, check in just after 09:00 UTC. Resume: re-create worktrees from fbdabaec, `git am` each patch, finish
each fixer's item list (table above), then merge into `final/all-fixes-xp04wq`, validate, push it, send branch+sha to
the PR 74 CI thread.

- 05:05 UTC re-save: a=93489016/5 b=48af6b95/2 c=63b2beba/9 d=2e2aadd1/8 e=fa3100b9/5 f=fbdabaec/0 (head/commits). Fixer D DONE (8/8 fixed, D-4748..D-4756; gates incl. 11 pass; touched cli modules 526 pass). Check-in armed for 09:05 UTC (trig_016mComA6rA1qmTfyX3xop6W).

## PAUSE 2026-10-09 05:36 UTC (usage: 5-hour window ~90%; resume at the 09:05 UTC check-in, trig_016mComA6rA1qmTfyX3xop6W)
Heads (head/commits/dirty files): a=93489016/5/dirty7 b=fc5291b6/8/dirty0 c=1010370e/9/dirty0 d=2e2aadd1/8/dirty0 e=fa3100b9/5/dirty0 f=3d1d7552/4/dirty0
- B DONE: 8 commits, D-4716..D-4724. G2-5 capture bound 6+16C fsyncs (was (1+T)(2+8C)+2); W2-cli8-10/-11 zero ceiling + one support validator in api+cli; census at the auto-support door; test gaps closed. All gates incl. 11 pass. Left open: cli argv `screen` still refuses a zero point ceiling at its own entry (api accepts); elite descent floor >= 1,000,000 ppm not checked by the shared validator; web label "Visited policy passes".
- D DONE: 8 commits, D-4748..D-4756. Identity changes: Boolean source identity for NSE cash shares (receipt policy 4); stored runs over a share with an unreadable CAS-day close. F-CEC7A0 read-side fix; docs/11 row stays OPEN with an IN PROGRESS bullet.
- E DONE: 5 commits, D-4764..D-4768. Bounded limit: a same-length rewrite with equal metadata is caught by the next open, not by a read of an unreturned record.
- A STOPPED mid-work: 5 commits (G1-1, G1-2, G2-4, A-B-A/pass-1 tests, G1-3 docs); uncommitted WIP (7 files, likely the R9-cli-o1-1 stretch) in fx-a.uncommitted.patch.md. Not yet reported: re-run its checks before merging.
- C STOPPED during final checks: 9 commits (G5-2, G5-1, G5-3, G5-4, G1-4, G3-3, G3-5, G3-8, web zero ceiling). Re-run its checks.
- F STOPPED during final checks: 4 commits (W2-cli3-3 D-4782; W2-cli3-4+G4-3+G4-2 D-4780/4783/4784; W2-cli7-2 full re-proof kept, duplicated validation removed D-4785; + 1). G4-4 status unknown: check. Re-run its checks.

## RESUMED 2026-10-09 09:30 UTC (owner: "Full speed"; weekly guard: save at 93%, stop at 98%)
- Merged into `final/all-fixes-xp04wq` (local, not yet pushed), first-parent chain from fbdabaec:
  a3bf5c9e B, 415a5838 D, d0376a9c E, dac4e954 C, a9fee04b F (hand-resolved candidate_universe.rs: D's `cash`
  parameter threaded through `build_full_candidate_signal_column`; F's prebuilt NIFTY column passes `None`).
  Ledgers union-merged; no conflict markers; D-47xx headings unique (D-0370/D-0372 duplicates are pre-existing on fbdabaec).
- Running: workspace clippy + full tests on a9fee04b (worktree /home/claude/wt-fix, logs int-clippy-2/int-test-2).
- Fixer A still finishing (5 commits + WIP R9-cli-o1-1 stretch).
- New fixers from a9fee04b: G (branch lane1b/fx-g, D-4769..D-4779, L1FG-): cli `screen` zero ceiling, elite descent
  floor domain, web "Visited policy passes" label, W2-cli9-5 torn journal, W2-cli13-5 litter count, W2-cli10-0 Base
  Evidence V2 two-open append. H (lane1b/fx-h, D-4786..D-4795, L1FH-): G4's UNVERIFIED audit of whole-file content
  hashes on cached reads in execution_v3/v4, admission_v2/v4, finalization_v4, statistics_v3, population_v5/v6,
  pre_admission_data; fix to the D-4765 pattern or correct the docs.
- Adversarial review workflow over each fixer diff + the merge resolutions (worktree /home/claude/wt-review @ a9fee04b).
- Resume if lost: re-create the merge chain above from the fixer branches' patches (patches/), then continue A, G, H.

## PAUSE 2026-10-09 10:05 UTC (owner: GDFL is the top priority; weekly usage 68%; coordinator: pause until told to resume)
- PUSHED: `origin/final/all-fixes-xp04wq` @ 3fa8adf7 = fbdabaec + merges B (a3bf5c9e), D (415a5838), E (d0376a9c),
  C (dac4e954), F (a9fee04b), A (3fa8adf7). NO PR (owner rule: one combined PR, #74). Not yet handed to the PR 74 CI thread.
- Validated on a9fee04b (B..F): cargo fmt --check clean; .github/*.rs rustfmt clean; workspace clippy --all-targets -D warnings
  clean; all 29 static gates pass (0,1,1b,1g,1f,1c,1d,2,9,9b,10b,7 skip,13,15,16,17,23,21,22,24,25,27,27b,26,19,10,12,14,11).
  Full workspace test run on a9fee04b was still compiling at pause (log scratchpad/logs/int-test-2.log; result appended below if it finishes).
- NOT yet validated: the A merge (3fa8adf7) on the combined tree (A's own tree passed its checks: see fixer A report). pool.rs
  conflict resolved by keeping both new tests (B/D's ledger-banner test and A's pass-one-order tests).
- STOPPED mid-work (WIP saved, nothing committed on their branches):
  - Fixer G (lane1b/fx-g from a9fee04b, D-4769..D-4779, L1FG-): patches/fx-g.uncommitted.patch.md (8 files: api sweeprun,
    operation_audit/search_checkpoint/operator_boundary/audited_stored tests, web CandidateTrades label + test, docs/19).
    It was writing item 4's test (W2-cli9-5 torn journal). Items 5 and 6 likely not started.
  - Fixer H (lane1b/fx-h from a9fee04b, D-4786..D-4795, L1FH-): patches/fx-h.uncommitted.patch.md (19 files). It was implementing
    the Execution V4 metadata-only generation. Its audit table was not reported.
  - Adversarial review: REVIEW-PARTIAL.md (reviewers B and C done, 7 findings, unverified; note reviewer C labelled its ids B-1..B-5).
    review-workflow.js.md is the script; re-run it in a new session against the then-current tree.
- Resume order: apply G and H patches onto lane1b/fx-g / fx-h (base a9fee04b), finish their items; re-run the review workflow
  (D, E, F, merges still to do) and fix confirmed findings (start with REVIEW-PARTIAL.md); merge G and H into
  final/all-fixes-xp04wq; full workspace tests as uid 65534 for root-only failures; static gates; cargo-mutants sample; push;
  send branch+sha to the PR 74 CI thread; publish the comparison-table artifact.
- 2026-10-09 test result on a9fee04b (B..F merged), as root: `cargo test --workspace --locked --no-fail-fast` exit 0, 7,424 passed, 0 failed, 15 ignored (baseline fbdabaec: 7,364 / 0 / 15). Log: scratchpad/logs/int-test-2.log.

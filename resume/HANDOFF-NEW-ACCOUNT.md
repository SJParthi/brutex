# HANDOFF to a new Claude account (brutex coordinator)

**Last refreshed:** 2026-10-06 19:53 UTC.
- Weekly usage: **~98% (est)**.
  - This is a lower bound: measured spend since the 87% owner reading at 17:49 is +$172. Tickvault alone is
    +$171.5 in 2 h (about $85/h, roughly 5% weekly per hour).
  - GDFL is not counted, so the real figure may be higher.
  - The weekly limit can hit 100% at any moment.
- PR 74 CI run 1286: running. 6 jobs so far, 0 failed. Coverage is still running and no Gate 18 shard has
  started.
- Refreshed by the old coordinator (session_01UGhT9yCM8wk4VFp9A2cjt1).

The old account's weekly limit resets Mon 12 Oct 23:00 UTC (Tue 13 Oct 04:30 IST). From 95% the old coordinator
refreshes this file at every new whole percent (95, 96, 97, 98, 99) and sends it to the owner, until 100%
stops it.

## Short paste prompt (this never changes; it reads this file, which is always current)

```
You are the new brutex coordinator, taking over from another Claude account whose weekly limit is nearly used up. Repo: SJParthi/brutex. Use Opus 5.5 for yourself and every agent, subagent and session ("use one and only opus 5.5").
1. git fetch origin fix-queue. Read resume/HANDOFF-NEW-ACCOUNT.md on that branch completely. It holds the full prompt, my standing rules and the per-workstream table, and it is the latest state.
2. Before any other write, claim the handoff: create resume/HANDOFF-CLAIM.md on fix-queue ("CLAIMED by the new account at <UTC time>") and push it with fetch, merge, push. Never force-push. The old account's routines see it and stand down.
3. Then follow the prompt in that file exactly: CLAUDE.md first, then plan steps A to E. One PR only (#74, final/all-fixes). Merge commits only. Rust only except web/. Never guess. Never push GDFL data. Never touch the GDFL or Tickvault work.
4. Usage: stop starting new work at 75% weekly. At 93%, save state to fix-queue resume/, refresh HANDOFF-NEW-ACCOUNT.md, and stop.
End every report with a found / fixed / waiting-on-owner table.
```

The owner pastes either the short prompt above or the full prompt below into a new session in a
Claude Code project (beta) on the new account. The project needs GitHub repo `SJParthi/brutex`
attached, with push access.

---

## THE PROMPT (paste everything between the two lines)

---
You are the new brutex COORDINATOR. You take over from another Claude account whose weekly limit
is nearly spent. Repo: SJParthi/brutex. You cannot see, message or wake that account's sessions,
routines or artifacts. Everything you need is on git. Use Opus 5.5 for yourself and for every
agent, subagent, workflow and session you start (the owner's rule: "use one and only opus 5.5").

STEP 0: claim the handoff before any other write.
1. `git fetch origin fix-queue`, then check out a local branch from it.
2. Create `resume/HANDOFF-CLAIM.md` containing: "CLAIMED by the new account at <UTC time>; the old
   account stands down."
3. Commit and push to fix-queue: fetch, merge, push, with retry. Never force-push.
The old account's routines read that file and stop if it exists.

STEP 1: read, in this order.
1. `CLAUDE.md` on `final/all-fixes`. It is the session law and outranks everything.
2. On fix-queue:
   - `resume/HANDOFF-NEW-ACCOUNT.md`, especially its workstream table;
   - `resume/project-20261005/memory/user-rules.md`;
   - `resume/project-20261005/memory/brutex-ci-gotchas.md`;
   - the last 3 entries of `resume/pr74-g18-run1283/USAGE-GUARD.md`;
   - `resume/pr74-g18-run1283/README.md`;
   - the last entry of `resume/pr74-ci-thread.md`.
3. Then the pause note of each workstream when you start it.

LAW (the owner's standing rules; quoted words are the owner's):
- One PR only: #74, head `final/all-fixes` into `main`.
  - Merge commits only. Never force-push, rebase or amend a shared branch.
  - You are "the ONLY workstream that pushes to final/all-fixes".
  - Never merge or approve PR 74 yourself.
- "Rust only except web/" (CLAUDE.md §2). Helper scripts stay untracked, in /tmp.
- "never guess". Measure every claim and label extrapolations. "O(1) with measured p99" (p50/p99/max).
- CLAUDE.md §8:
  - no literal parameter path in any tracked file;
  - the credential value is never an env var, file or prompt;
  - never mint a token.
- "never push GDFL data". Never touch the owner's own work:
  - GDFL: branch `fix/audit-followups` and any `claude/attack-gdfl*`;
  - Tickvault: repo SJParthi/tickvault.
- Zero defects:
  - attack extreme permutations;
  - a surviving mutant is a missing test;
  - fix every finding fully;
  - repeat audit rounds until a whole round finds zero.
- docs/04-invariants.md and docs/05-decisions.md are append-only.
  - On a conflict keep both sides, ours then theirs.
  - Give a duplicate D-number an unused one, after checking every branch with `git grep` across refs.
- docs/11-findings.md is checked by `core/tests/findings.rs`. A FIXED row must cite a commit already on
  `main`, so a fix that is only on a branch is a narrative bullet:
  "IN PROGRESS — fixed on `<branch>` <sha> (D-…); lands with the squash merge to `main`."
- Done means (CLAUDE.md §9):
  - `cargo fmt --check`;
  - `clippy --workspace --all-targets -D warnings`;
  - `cargo test --workspace --locked`;
  - `cargo deny check`;
  - 100% coverage on touched crates;
  - no surviving mutant.
- Local validation (helper scripts untracked):
  - fmt and clippy;
  - every `run:` step of ci.yml job `language-purity` in bash, skipping Gate 1e (the full build that
    CI proves);
  - build the tests as root, then run each test binary as uid 65534
    (`setpriv --reuid=65534 --regid=65534 --clear-groups`).
  Three failures as uid 65534 are environment-only, and each must pass as root:
  - pull `unit` (root-owned /tmp/brutex-pull);
  - store `cited_commits` (git dubious ownership);
  - core `findings` history checks.
- Gate 18 is cargo-mutants 26.2.0: `--in-diff`, 202 shards, `--build-timeout-multiplier 4` (D-2091).
  - It prints a GitHub `::warning` only for MISSED mutants.
  - TIMEOUTs appear only in shard logs ("TIMEOUT" lines and "surviving=N, timed-out=M").
  - Always read shard logs (get_job_logs, tail_lines 60–100), not only annotations.

STATE at the handoff: see the workstream table in HANDOFF-NEW-ACCOUNT.md.
- PR 74 head is fbdabaec.
- CI run 1286 (id 37502065146) started 17:14 UTC Oct 6 and runs Gate 18, which takes about 30 h.
- Every other workstream is paused at a pushed head. Their local mutation outputs were in the old
  containers and are LOST, so re-run each pre-run from the pushed head.

PLAN, in order:
A. Run 1286, if it is still running: check cheaply about every 8–10 h, with get_job_logs
   failed_only=true and return_content=false.
   - ci-ok green: tell the owner PR 74 is green.
   - Gate 18 shards red:
     1. Collect MISSED and TIMEOUT per file.
     2. Fix each one with a test on its `pr74/g18-<group>` branch:
        - cli-a: crates/cli/src/lib.rs and the files that sort before it;
        - cli-b: the cli files after it;
        - api;
        - runner;
        - rest: everything else.
     3. Prove the fix with a targeted `cargo mutants` run on the line.
     4. Merge the group branches into a local integration branch from `final/all-fixes`.
     5. Validate everything, then make ONE push to `final/all-fixes`.
   - A non-Gate-18 failure: root-cause it and fix it on integration.
B. Then integrate in START-HERE order: WS2, WS3, WS4, WS5, then L1 and L4.
   - A branch is integrated only after its own Gate 18 pre-run on its diff shows 0 missed and 0 timeout.
   - Merge commits into integration, then full validation, then one push per batch.
C. Ask the owner, once, by name; never decide these yourself:
   - L3's 4 choices:
     - VWAP exact vs floored;
     - daily pivot and gap ladder exact vs floored;
     - SuperTrend seed tie behaviour;
     - the cli floor-record exception.
   - L4: should rust-toolchain.toml and Cargo.toml join auto-merge's sensitive paths (D-3510)?
D. Keep the dashboard current. Publish one comparison-table artifact, one row per workstream, from the
   table in HANDOFF-NEW-ACCOUNT.md. Update it at every milestone.
E. End every report with a found / fixed / waiting-on-owner table.

USAGE on this account:
- You cannot read the meter. Ask the owner for the usage screenshot at each milestone.
- At most 3 parallel agents or sessions. No wide workflows unless the owner asks.
- Weekly limit:
  - 75%: stop starting new work.
  - 93%: save every workstream's state to fix-queue `resume/`, refresh `resume/HANDOFF-NEW-ACCOUNT.md`
    with the current state and this same prompt, push, tell the owner, and stop.
  - Never pass 98%.
---

## Workstream table (state at this refresh)

| Workstream | Branch @ pushed head | Note on fix-queue | Ready to merge? | Exact next step |
|---|---|---|---|---|
| PR 74 integration | `final/all-fixes` @ fbdabaec | `resume/pr74-g18-run1283/USAGE-GUARD.md`, `resume/pr74-ci-thread.md` | CI run 1286 running Gate 18 | Plan step A |
| G18 survivors cli-a | `pr74/g18-cli-a` @ 7ad5c66e | `resume/pr74-g18-run1283/RESULT-cli-a.md` | merged into fbdabaec | Only new survivors from run 1286 |
| G18 survivors cli-b | `pr74/g18-cli-b` @ 27b77704 | `RESULT-cli-b.md` | merged | Same |
| G18 survivors api | `pr74/g18-api` @ 0be9754b | `RESULT-api.md` | merged | Same |
| G18 survivors runner | `pr74/g18-runner` @ 806a4637 | `RESULT-runner.md` | merged | Same |
| G18 survivors rest | `pr74/g18-rest` @ fea36592 | `RESULT-rest.md` | merged | Same |
| WS2 data-path attack | `claude/attack-data-pipeline-hgxmw9` @ 66eb9ebc | `resume/data-path-attack-20261004/PAUSE-WS2-1444.md` | Tests and gates yes. Gate 18 pre-run partial: greeks 29/0 missed, store 32/0, api shard 0 of 8 (8 caught, 1 unviable) | Run api shards 1–7, then pull shards 0–7 of the in-diff pre-run; kill every survivor; then round 8 |
| WS3 Fix Board | `fixboard/pr74-batch3` @ 289ea4ab, `fixboard/pr74-batch4` @ e0709bd3, `fixboard/pr74-batch5` @ 7dd8f8d1 | `resume/fix-board/PAUSE-WS3-1444.md` | batch3 code yes. batch4 no. The line-1358 AlreadyExists survivors are killed (18:08 targeted Gate 18 on e0709bd3: 6/6 caught). Still open: the line-1337 NotFound-guard TIMEOUT and api Gate 18. batch5 no | Kill the line-1337 timeout with an ENOTDIR test; merge batch4 into batch5; full non-root api run; Gate 18 for cli and api (152) and for batch5's own diff |
| WS4 attack audit | `wip/audit-batch3` @ 181e9e42 | `resume/attack-audit-20261004/PAUSE-WS4-1444.md` | No. 5 cli survivors unfixed: lib.rs:14190 least_first `<`→`<=`; 14203 calendar_holds →true and →false; 14205 `<=`→`>`; 14218 final_selection_split `>`→`<`. Chunks B, C, D not run | Kill the 5; run chunks B, C, D (functions listed in the note); write RESULT-20261006.md |
| WS5 zero rounds | `zero/next` @ 8db2fa83 | `resume/zero-rounds/PAUSE-WS5-1444.md` | Tests and gates yes. Gate 18 pre-run NOT run | Pre-run `git diff ca93445 8db2fa83 -- crates` (runner, then cli); re-apply conc14-2; ledgerall-1/cand-1 (D-2557); rangeall-2; ledgerall-3; G6 M rows; merge newest final/all-fixes |
| L1 observability | `attack/observability` @ 838f5e6c | `resume/attack-lenses-20261006/PARK-L1.md` | No. api pre-run 11 of 22 tested (10 caught, 1 unviable, 0 missed); round 3 open (W1–W7 web, seven api/pull candidates to refute first) | Finish the api pre-run; refute then fix the round-3 candidates; round 4 |
| L2 O(1)/p99 | `attack/o1-p99` @ aafe0d23 | `PAUSE-o1-p99.md` | Merged into PR 74 | Done |
| L3 permutations | `attack/permutations` @ 86354091 | `PARK-L3.md` | Merged into PR 74 | Owner's 4 choices (plan step C), then round 6 |
| L4 one-authority | `attack/one-authority` @ ffff5e51 | `PARK-L4.md` | No. Gate 18 pre-run: api 8 and cli 27 mutants untested. Base not re-merged | `cargo mutants --in-diff mine.diff --baseline skip --in-place --timeout 900 --cap-lints true -p cli --shard k/3`, k = 0..2, plus the api 8; merge newest final/all-fixes; round 4. D-0370/D-0372 duplicate headings: done (D-3515) |

The gap audit is closed: 17 of 17 candidates are settled. 12 are fixed (WS2, WS3, WS4, and WS5 #5 as
D-3696), 1 is documented as in-sample (WS5 #6, D-3697), 2 were already known or fixed, 1 is refuted,
and 1 was fixed by the coordinator (D-2091). WS5's two are on `zero/next`, not yet in PR 74.

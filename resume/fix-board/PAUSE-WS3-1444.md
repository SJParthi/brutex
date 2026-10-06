# WS3 Fix Board: PAUSE 2026-10-06 14:44 UTC (weekly 75% brake)

## Branches (all pushed)
- `fixboard/pr74-batch3` **289ea4ab**: PR #74 head + api2 + ce2 merges.
- `fixboard/pr74-batch4` **e0709bd3**: contains batch3; 11 conc rows. e0709bd3 extracts `linked_or_lost_race` for a Gate 18 survivor and adds a test for it.
- `fixboard/pr74-batch5` **7dd8f8d1**: on batch4 a08d2ec4; gap-audit #2 #3 #4 #13 #14 #17 (D-3684..3689, FB-113..117). It does NOT yet contain e0709bd3: merge batch4 into batch5 on resume.

## Validated
- batch4 at a08d2ec4: fmt, workspace clippy, static gates. Lib tests as uid 65534: store 114, pull 617, cli 1952, api 1506, all green.
- batch5 7dd8f8d1: fmt, workspace clippy, static gates, web gates W1 (fingerprint 55591a988a38ceca), W2 (870/870), W3/W4/W5/W6. api head-deadline, assets, bars, backtest and logs tests green. The 4 standalone new tests FAIL at a08d2ec4 (throwaway worktree) and pass at 7dd8f8d1. The full non-root api run at 7dd8f8d1 is NOT done.
- e0709bd3: cli candidate_trades tests 21/21, cli clippy clean.

## Gate 18 pre-run (diff origin/final/all-fixes...a08d2ec4, 244 mutants)
- pull: 21 caught, 13 unviable, **0 missed**.
- store: 19 caught, 1 unviable, **0 missed**.
- cli (partial, still running): 11 caught, 3 unviable, **2 TIMEOUT**, both in `candidate_trades::write_exact_via` (cli2-5, D-3603):
  - line 1337: `NotFound` match guard → true
  - line 1358: `AlreadyExists` guard → true. e0709bd3 moves this one into `linked_or_lost_race` with a direct test; a targeted re-run (/tmp/claude-0/mfix1) was in flight.
- api (152): not started.
- batch5's own diff: not started.

## Ready to merge
- batch3: yes, code-wise. Gate 18 for its api lines is not finished.
- batch4: NO until both cli2-5 timeouts are killed and api Gate 18 is done.
- batch5: NO until batch4 is merged in, the full non-root api run passes, and Gate 18 runs on its diff.

## Exact next step on RESUME
1. Read /tmp/claude-0/mfix1 (line 1358 kill proof). Kill the line 1337 NotFound-guard timeout with a test where `symlink_metadata` errors with something other than NotFound (e.g. a path through a non-directory, ENOTDIR) and the write must refuse. Push to batch4.
2. Merge batch4 into batch5. Run the full non-root api lib run.
3. Let /tmp/claude-0/mutall.sh finish cli and api (it resumes with --iterate). Then run Gate 18 on `git diff fixboard/pr74-batch4...fixboard/pr74-batch5 -- crates`.
4. Update RESULT-20261006.md.
- 14:45 UTC: the Gate 18 cli run (old tree a08d2ec4) also reports `candidate_trades.rs:1358 AlreadyExists guard -> false` as not caught (missed or timeout); e0709bd3's `linked_or_lost_race` test covers both the true and false variants.
- 15:04 UTC: the old-tree cli run also reports `candidate_trades.rs:1358 == -> !=` (same AlreadyExists guard) as not caught; also covered by e0709bd3's direct test (each io::ErrorKind asserted).

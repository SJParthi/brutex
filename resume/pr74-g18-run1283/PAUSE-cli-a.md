# PAUSE — cli-a (Gate 18 run 1283), 2026-10-06 ~04:55 UTC

Branch `pr74/g18-cli-a`, head **805abecc0babd352998fb91f35b88359181bde6a** (pushed, clean;
`git grep -n "changed by cargo-mutants"` prints nothing). No PR opened.

## Done (committed)
- All 73 listed survivors + shard-178 `results_at` + the 4 timeouts have a fix:
  a test asserting the exact behaviour, or a restructure removing an equivalent
  mutant. Decisions D-2002..D-2018, invariants G18-cli-a-01..36 (docs/04, docs/05).
- Boundary/extreme permutations per the owner addendum (commit 211c34b), incl. a
  differential test of `div_round_half_away` vs exact i128 rounding.
- Targeted tests pass as non-root (`setpriv --reuid=65534`): 20/20 of the new ones
  at 211c34b. Full cli suite NOT yet run on the final head. UNVERIFIED.

## Mutation proof so far (cargo-mutants 26.2.0, --baseline skip, --cap-lints true, filtered --lib tests)
- Run A (in-place, interrupted on purpose): 7 caught, 0 missed —
  checksum_receipts 480:44 and 480:24; admission_store 995:33;
  boolean_campaign command ->0/->1; boolean_oos_command command ->0/->1.
- Run C (stopped for the pause after 4 of 107): 4 caught, 0 missed —
  lib.rs column_withholding_unsourceable_days ->Ok(([0;32]))/Ok(([1;32])),
  pool_oos_arm `==`->`!=` (1818:22), pool_oos_arm guard ->true (1852:16).
- NOT yet proven: the other 103 of run C and the 4 timeout fixes (commits ab3382b, fcbecf6).

## Next steps on RESUME
1. Re-run the remaining mutation proofs from a worktree at the head (not in place):
   `git -C /home/user/brutex-wt merge --ff-only origin/pr74/g18-cli-a` (or recreate:
   `git worktree add /home/user/brutex-wt origin/pr74/g18-cli-a`), then
   `setsid nohup bash /tmp/claude-0/runC.sh > /tmp/claude-0/runC.log 2>&1 &`
   (script: cargo mutants --baseline skip --jobs 2 --timeout 900 --cap-lints true -p cli,
   --file lib.rs + 7 small files, name regexes for every survivor function,
   `-- --lib -- <the killing tests>`; output /tmp/claude-0/mutC). Add the 4 timeout
   mutants (`first_accepted_in_order`, `validates`, `validate_from_env`) with
   `--build-timeout 300` and the five premise tests to the filter.
2. Fix any MISSED/TIMEOUT, commit (check the marker first), push.
3. p50/p99/max of restructured per-op paths: harness at /tmp/claude-0/g18_measure.rs
   (temporary, never committed). Not yet run. UNVERIFIED.
4. Finish: merge newest origin/final/all-fixes (merge commit), fmt --check, clippy
   -D warnings, cli tests as non-root, static gates (language-purity steps), push,
   then RESULT-cli-a.md on fix-queue.

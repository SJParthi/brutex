# WS5 pause — 2026-10-06 14:44 UTC

- **Branch:** `zero/next`
- **Pushed head:** `8db2fa83`. Its parent is `ca93445`, the G1-G8 batch, which was validated.

## What is validated in 8db2fa83

All of the following were run before the commit:

- Every runner test binary passed, run non-root (lib: 751 passed).
- All 16 cli test binaries passed, run non-root (exit 0).
- `cargo fmt --check` is clean.
- `clippy --workspace --all-targets -D warnings` is clean.
- The static gates are green, with gate 1e skipped. They were re-run after `web/build` was staged.
- web: 908/908 tests pass, svelte-check reports 0 errors, and `web/build` was rebuilt.

## Audit #5 (look-ahead): confirmed and fixed (D-3696)

- **The defect, measured.** Widening only the last fold's test window moved the whole-span affordable probe. On sessions(8) it moved from 205 to 495 hits, so fold 0's threshold moved from 69 to 165. On sessions(16) it moved from 727 to 1,073.
- **The fix.** `runner::validate::FoldSupport::FirstTraining` probes the first fold whose training column has rows, then rescales that answer to every later fold. The range path passes it; every other caller passes `Scaled`. Two identity terms are appended to the span policy.
- **Fails without the fix.** `a_test_window_bar_cannot_move_an_earlier_folds_support` failed with "Anchored fold 1: a test-window bar moved a training answer".
- **Left to do:** the Gate 18 `cargo mutants --in-diff` pre-run for this commit (runner, then cli) has NOT been run.

## Audit #6 (stated stop): confirmed in-sample only, documented (D-3697)

- **Measured.** Doubling only the last bar's high moved the stop from 1,999 to 1,332 ppm.
- **Why it is not a fix.** The stop never reaches a walk-forward fold, which a test pins. The screen is in-sample by construction (D-1660). The false "fifty means fifty" doc was corrected.
- **Owner decision.** A per-trade N-point stop needs a per-entry stop in `runner::grid`.

## Not committed

- **conc14-2.** The archive ingest goes through `off_the_workers`. Its patch is kept locally and was never compiled.
- **ledgerall-1 / cand-1.** The discard of a foreign orphan was not started; the code edit failed before it wrote anything.

## Next step on resume

1. Run the Gate 18 mutants for `git diff ca93445 8db2fa83 -- crates`, runner and then cli, in a separate worktree.
2. Send the coordinator the #5/#6 reply.
3. Re-apply and validate the conc14-2 patch.
4. Fix ledgerall-1/cand-1 under D-2557, then rangeall-2, ledgerall-3 and the G6 M rows.
5. Merge the newest final/all-fixes, then write RESULT-20261006.md.

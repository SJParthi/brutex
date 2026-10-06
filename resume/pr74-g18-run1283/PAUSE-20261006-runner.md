# PAUSE 2026-10-06 08:07 UTC — g18 runner (pr74/g18-runner)

Head of `pr74/g18-runner`: **c216c97**, pushed, working tree clean, `git grep "changed by cargo-mutants"` empty.
Nothing unvalidated, so there is no WIP commit.

## Done
The whole survivor and timeout list is closed and proven; see RESULT-runner.md (same folder). Final checks on c216c97:
fmt, workspace clippy -D warnings, all 13 runner test binaries as uid 65534, static gates (1e skipped) all green.
origin/final/all-fixes is still 969493e1, so the merge is a no-op.

## The 5 not caught in the first run (M2: 113 mutants on fe5c87a, 108 caught) and how the second run settled them
1. TIMEOUT `crates/runner/src/significance.rs:509:13: replace < with > in ln_gamma`. That was the old unbounded `while z < 10.0` loop. c216c97 bounded it to ten fixed steps, bit-identical
   (D-2064). M3 (56 mutants on c216c97, `--build-timeout 300`) caught every ln_gamma mutant: 0 missed, 0 timeout.
2-5. UNVIABLE: the replacement does not compile because the return type has no `Default`. Gate 18 accepts unviable;
   these are not survivors and need no kill:
  - crates/runner/src/audit.rs:228:5: replace largest_overnight_move -> Option<OvernightMove> with Some(Default::default())
  - crates/runner/src/bootstrap.rs:1241:5: replace romano_wolf -> Vec<Rejected> with vec![Default::default()]
  - crates/runner/src/bootstrap.rs:1266:5: replace romano_wolf_receipt -> Option<RomanoWolfReceipt> with Some(Default::default())
  - crates/runner/src/bootstrap.rs:1332:5: replace romano_wolf_adjusted_p_values_v1 -> Option<RomanoWolfAdjustedReceiptV1> with Some(Default::default())

## In progress
Nothing. No build, test or mutation run is active.

## Next steps on RESUME
None for this group. The coordinator can merge `pr74/g18-runner` (c216c97) into PR #74. If final/all-fixes has moved,
run in the repo: `git fetch origin final/all-fixes && git merge origin/final/all-fixes`, then cargo fmt --check, clippy,
and runner tests as non-root, then push.

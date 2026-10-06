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

## PAUSE 08:28 UTC: second task (run 1283 untested runner mutants)
Branch head unchanged: **c216c97** (clean, pushed). No code change has been made for this task yet.

**Mapping.** The 116 `crates/runner` lines of untested-mutants-run1283.md use 969493e1 line numbers. They were mapped to
c216c97 through `git diff -U0 969493e1 HEAD` hunks.
- 113 map exactly to mutants that exist on c216c97.
- 3 no longer exist (`audit.rs` `grid`: the match guard `key(b) < key(a)` -> false, and `<` -> `==` / `>`). That guarded
  swap was replaced by `cmp::min_by_key`/`max_by_key` in aaa9fcb/f097f06 (D-2056).

**In progress.** Run M4 started about 08:25 UTC and may keep running during the pause: 113 mutants on c216c97, a copied
tree (not in-place), the full runner test suite. Outputs are this container's scratchpad `mut4/` and `mut4.log`.
`--in-diff` was not used: it would drop the mutants on lines my diff does not touch, which is nearly all of them, so the
exact-name `--re` is used instead. With `--baseline skip`, `--minimum-test-timeout` alone left a 300 s default, so an
explicit `--timeout 900` was added. The command:
`cargo mutants --baseline skip --jobs 1 --timeout 900 --minimum-test-timeout 900 --build-timeout 900 --cap-lints true -p runner --re "<exact names>" --cargo-arg=--locked --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --no-shuffle --colors never -o <dir>`

**Next on RESUME.**
1. Read M4: `wc -l mut4/mutants.out/{caught,missed,timeout,unviable}.txt`.
2. Kill each MISSED or TIMEOUT with a test or a restructure, using D-2065..D-2069 then D-3660..D-3669 and invariants
   G18-runner-19 onward.
3. Re-run cargo-mutants on just those mutants and require 0 missed and 0 timeout.
4. Run fmt, clippy, runner tests as non-root and the static gates, then push and update RESULT-runner.md.
If M4 is lost with the container, rebuild the list: map the 116 lines as above and run the command above.

## PAUSE 09:23 UTC: untested-mutant task, partly done
Branch head: **a8d09ad**, pushed and clean.
- Done: D-2065 (a8d09ad). `audit.rs` `grid` now pivots `select_nth_unstable_by_key` on index `shown` under
  `shown < len` alone. This removes the M4 survivor `audit.rs:1148:50 - -> / in grid`. fmt, runner clippy and the audit
  tests pass as non-root. NOT yet proven by cargo-mutants on a8d09ad.
- In progress: M4 (113 mapped untested mutants, on c216c97, full suite) is still running and may continue.
  At pause it had caught 82, unviable 6, missed:
  - crates/runner/src/audit.rs:1148:50: replace - with / in grid
  timeout:

- Count note: the coordinator says 121, but untested-mutants-run1283.md has 116 runner lines (113 still exist and 3 were
  removed by D-2056). I found no separate list of the "5 from shard 138".

## Next on RESUME
1. Read the final M4 counts. Kill every remaining MISSED or TIMEOUT, using D-2066..D-2069, then D-3660 onward, and
   invariants G18-runner-19 onward.
2. Re-run cargo-mutants on a8d09ad plus later fixes, for the new `audit.rs` pivot line and every fixed mutant,
   with the same flags as M4. Require 0 missed and 0 timeout.
3. Run fmt, workspace clippy, runner tests as non-root and the static gates, then push and update RESULT-runner.md.

## M4 finished during the pause
113 mutants on c216c97: caught 104, missed 1, timeout 1, unviable 7. Missed or timeout:
  - crates/runner/src/audit.rs:1148:50: replace - with / in grid
  - crates/runner/src/expression_validation.rs:261:18: replace += with *= in index_folds
(The audit.rs:1148 `-` -> `/` survivor is already restructured away in a8d09ad, not yet proven.)

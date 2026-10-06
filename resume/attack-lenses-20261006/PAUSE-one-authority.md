# PAUSE (09:23 UTC) — lens L4 one-authority

Branch `attack/one-authority`, head `2f4be13` (pushed). Nothing real is uncommitted: the working tree shows only
cargo-mutants' in-place mutation of the file under test — DO NOT commit while `cargo mutants --in-place` runs.
Base origin/final/all-fixes 969493e (merge newest before the validated push).

## Done (pushed)
- Rounds 1-3: D-3500..D-3515, ONEAUTH-01..15 (see RESULT-one-authority.md on fix-queue).
- 2f4be13 validated: fmt, clippy -D warnings, all language-purity gates (not 1e); web W1-W5 at 7cf031b.
- D-0370/D-0372 question answered (D-0684 history; gate 27b already refuses duplicates) — in D-3515.

## In progress (running in this container)
- Non-root tests of core pull indicators runner cli api on 2f4be13 → /tmp/claude-0/a/nr-r3.log
- cargo mutants --in-diff (per crate: core indicators pull runner api cli) → /tmp/claude-0/mut-all.log,
  outcomes in /tmp/claude-0/mut-<crate>/mutants.out/{missed,timeout,caught}.txt
  core: 2 tested, 1 caught, 1 unviable, 0 missed.
- Known likely survivors to check: indicators orb.rs:41/42 (OPEN_MINUTE / MICROS_PER_MINUTE consts), pull ssm
  now_stamp (clock-reading; may need `stamp_of` only), api coverage.rs/autopilot.rs (comment-only lines in diff).

## Next steps after RESUME
1. When mutants finish: `git status` must be clean; `git grep -n "changed by cargo-mutants"` must print nothing.
2. Kill every MISSED/TIMEOUT with a real test (re-run with `--file <f> --re '<name>'` to prove each kill).
3. Confirm nr-r3.log green (findings history tests fail only as nobody; ok as root).
4. Merge origin/final/all-fixes (merge commit), commit "validated", push.
5. Round 4: fresh-eyes pass over (a)-(d) + review of round-3 changes; exit when a whole round finds zero.
6. Update RESULT-one-authority.md and one-authority.tsv.

## Restart
git fetch origin attack/one-authority final/all-fixes fix-queue && git checkout attack/one-authority
cargo-mutants 26.2.0 + cargo-nextest installed here; `--jobs` is refused together with `--in-place` (omit it).

## Background result: non-root tests on 2f4be13
```
FAIL-nobody core findings-d1a517611526752b test result: FAILED. 10 passed; 4 failed; 0 ignored; 0 measured; 0 fil
test every_named_commit_exists ... FAILED
test every_named_commit_is_in_this_branchs_history ... FAILED
test only_a_tree_with_no_git_dir_skips_the_history ... FAILED
test every_named_commit_is_on_main ... FAILED
test result: FAILED. 10 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
   as root: test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 0 filtere
FAIL=0
DONE
ok binaries: 77
```

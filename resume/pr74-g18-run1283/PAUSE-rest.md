# PAUSE — group rest (branch pr74/g18-rest)

Paused 2026-10-06 on the coordinator's usage guard. Head of `pr74/g18-rest`: **d170a21** (pushed; `git grep "changed by cargo-mutants"` prints nothing). No WIP commit was needed: the tree was clean. No mutation run is active.

## Done (committed, pushed)

| Commit | Content |
|---|---|
| a54f945 | costs, indicators, vocab survivors (D-2070, D-2071, D-2081) |
| 92bcd1a | lake, telemetry survivors (D-2072, D-2080). **Carried a committed mutant in tail.rs:503, fixed by 0d8c386** |
| e3e8370 | pull survivors (D-2073 to D-2076) |
| 3de8a19 | store survivors (D-2073, D-2077 to D-2079) |
| 99d8217 | docs D-2070..D-2081, G18-rest-01..26 |
| 0d8c386 | restores tail.rs `while pos > 0` |
| 3b18a82 | Overlay::same_bytes kill (D-2082, G18-rest-27) |
| 37e4c56 | test literals / fixture temp path for gates 1d, 23 |
| 3565e47 | vocab NAME_INDEX recursion, no build hang (D-2083, G18-rest-28/29) |
| bb73610 | telemetry walk_back bounded by range (D-2084, G18-rest-30) |
| d170a21 | lake footer walk pass bound + masters FIFO test unblock (D-2085, G18-rest-31/32) |

## Mutation evidence so far (cargo-mutants 26.2.0)

- Caught, in-place in the main checkout (root, world-readable path): costs regime 181; indicators pattern 647, trend 329; lake footer 106/207/240x3/248; telemetry tail 565 (`>=`, `==`); pull archive 134 (4).
- Caught in the world-readable worktree `/home/user/wt-g18`, each killed by its own test: pull fetch 845; http 695 and 997; ingest daycheck_headline guard (true and false); masters 397 and 449 (2); rolling 613; session irregular_verdict (3); pull support 45 and 64; store support 45 and 64.
- store file.rs, partly done: caught 2660, 2699, 2747, 2748, 2912, 3312 (`&&`, `< with ==`). **Not yet run:** 3312 (the other 4), 3535, 3752 (3), 4089, 4090, 4279. store format.rs Overlay::same_bytes not yet run.
- With CI's flags plus `--build-timeout 300`: vocab table.rs 39 mutants, 13 caught and 26 unviable (0 missed, 0 timeout); telemetry walk_back 13 mutants, 13 caught (0 timeout).
- Removed by restructure (absent from `--list`): trip.rs gst `>`; gap.rs fold `>` and `<`; calendar last_day guard; vocab MAX_NAME_BYTES `>` and place guard. open_flags whole-fn mutant is unviable.
- **Interrupted, re-run needed:** lake footer.rs all mutants (CI flags plus build-timeout) for the d170a21 pass bound and Cursor::byte Ok(1); pull masters holds_exactly (shard 10 timeout).

## Not done yet

1. Finish store-b and store-c, then lake-ci and masters-ci (commands below).
2. The in-diff run of my own production changes: `git diff 969493e1 HEAD -- crates > mine.diff` and `cargo mutants --in-diff`.
3. Static gates: all passed at 37e4c56 except 1e (skipped). Re-run them after d170a21.
4. Non-root tests: all touched crates passed as uid 65534 at 3b18a82, except store cited_commits (environment: git and the target tmpdir; passes as root). Re-run for vocab, telemetry, lake and pull.
5. Timing p50/p99/max: harness built in /tmp/claude-0/tbench-{base,head}, not yet run (UNVERIFIED).
6. Merge newest origin/final/all-fixes (it had not moved past 969493e1 at last check). Then write RESULT-rest.md.

## Restart commands

```
cd /home/user/brutex && git checkout pr74/g18-rest && git pull --ff-only origin pr74/g18-rest
git -C /home/user/wt-g18 checkout -q --detach origin/pr74/g18-rest   # world-readable worktree; in-place runs go here only
F="--jobs 2 --jobserver-tasks 4 --minimum-test-timeout 900 --timeout-multiplier 2 --build-timeout 300 --cap-lints true --cargo-arg=--locked --test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --colors never"
cd /home/user/wt-g18
cargo mutants -p store --file crates/store/src/file.rs --re 'file.rs:3312:|file.rs:3535:.*replace < with <=|file.rs:3752:.*(with true|replace ==)|replace refuse_link -> |is_symlink\(\) with false in refuse_link|replace write_symlinked' --in-place $F
cargo mutants -p store --file crates/store/src/format.rs --re 'Row for Overlay>::same_bytes' --in-place $F
cargo mutants -p lake --file crates/lake/src/footer.rs --in-place $F
cargo mutants -p pull --file crates/pull/src/masters.rs --re holds_exactly --in-place $F
```
(`--in-place` takes no `--jobs`; drop `--jobs 2 --jobserver-tasks 4` when using it. Run one at a time.)
